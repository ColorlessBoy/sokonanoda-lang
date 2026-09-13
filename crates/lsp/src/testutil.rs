//! Shared protocol-level test helpers: drive requests/notifications through
//! an in-memory `LspService` (no stdio). Feature-specific request helpers
//! live next to the tests that use them.

use crate::Backend;
use futures::StreamExt;
use serde_json::{json, Value};
use std::time::Duration;
use tower::Service;
use tower::ServiceExt;
use tower_lsp::jsonrpc::Request as RpcRequest;
use tower_lsp::lsp_types::*;
use tower_lsp::{ClientSocket, LspService};

/// 带全部自定义方法注册的服务（soko/goals、soko/nextHole、soko/hints、
/// soko/stateAt）。
pub(crate) fn test_service() -> (LspService<Backend>, ClientSocket) {
    LspService::build(Backend::new)
        .custom_method("soko/goals", Backend::goals)
        .custom_method("soko/nextHole", Backend::next_hole)
        .custom_method("soko/hints", Backend::hints)
        .custom_method("soko/stateAt", Backend::state_at)
        .finish()
}

/// Guard only: any server→client message must arrive within this budget.
pub(crate) const TIMEOUT: Duration = Duration::from_secs(2);
pub(crate) const URI: &str = "file:///test.sokonanoda";

/// 0-based LSP position for a char offset in an (ASCII) source text.
pub(crate) fn lsp_pos(src: &str, offset: usize) -> Position {
    let before = &src[..offset];
    let line = before.matches('\n').count() as u32;
    let character = (offset - before.rfind('\n').map(|i| i + 1).unwrap_or(0)) as u32;
    Position { line, character }
}

pub(crate) fn offset_of(src: &str, needle: &str) -> usize {
    src.find(needle)
        .unwrap_or_else(|| panic!("`{needle}` not found in `{src}`"))
}

pub(crate) fn position_json(pos: Position) -> Value {
    json!({"line": pos.line, "character": pos.character})
}

/// Drive one request/notification through the service (no stdio involved).
pub(crate) async fn call(service: &mut LspService<Backend>, req: RpcRequest) -> Option<Value> {
    let resp = service
        .ready()
        .await
        .expect("service ready")
        .call(req)
        .await
        .expect("service call succeeded");
    resp.map(|resp| match resp.into_parts() {
        (_, Ok(result)) => result,
        (_, Err(err)) => panic!("json-rpc error response: {err:?}"),
    })
}

pub(crate) async fn notify(service: &mut LspService<Backend>, method: &'static str, params: Value) {
    let req = RpcRequest::build(method).params(params).finish();
    service
        .ready()
        .await
        .expect("service ready")
        .call(req)
        .await
        .expect("notification processed");
}

/// initialize/shutdown handshake: checks the advertised capabilities once.
pub(crate) async fn handshake(service: &mut LspService<Backend>) {
    let init = RpcRequest::build("initialize")
        .params(json!({"capabilities": {}}))
        .id(1)
        .finish();
    let result = call(service, init).await.expect("initialize must answer");
    let result: InitializeResult = serde_json::from_value(result).expect("valid InitializeResult");
    let caps = result.capabilities;
    assert_eq!(
        caps.text_document_sync,
        Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        "full text sync expected"
    );
    assert_eq!(
        caps.hover_provider,
        Some(HoverProviderCapability::Simple(true))
    );
    assert_eq!(caps.document_symbol_provider, Some(OneOf::Left(true)));
    assert_eq!(
        caps.code_lens_provider,
        Some(CodeLensOptions {
            resolve_provider: Some(false)
        })
    );
    assert_eq!(
        caps.code_action_provider,
        Some(CodeActionProviderCapability::Simple(true))
    );
    assert_eq!(
        caps.semantic_tokens_provider,
        Some(SemanticTokensServerCapabilities::SemanticTokensOptions(
            crate::semantic_token_options()
        )),
        "full semantic tokens with the shared legend expected"
    );
    assert_eq!(
        caps.definition_provider,
        Some(OneOf::Left(true)),
        "go-to-definition must be advertised"
    );
    assert_eq!(
        caps.document_highlight_provider,
        Some(OneOf::Left(true)),
        "document highlight must be advertised"
    );
}

pub(crate) async fn shutdown(service: &mut LspService<Backend>) {
    let req = RpcRequest::build("shutdown").id(i64::MAX).finish();
    let result = call(service, req).await;
    assert!(result.is_some(), "shutdown must answer");
}

pub(crate) async fn did_open(service: &mut LspService<Backend>, text: &str) {
    notify(
        service,
        "textDocument/didOpen",
        json!({"textDocument": {
            "uri": URI, "languageId": "sokonanoda", "version": 1, "text": text
        }}),
    )
    .await;
}

/// 一次 didChange（FULL sync）。`version` 必须严格递增——`did_open` 用的是 1。
pub(crate) async fn did_change(service: &mut LspService<Backend>, version: i32, text: &str) {
    notify(
        service,
        "textDocument/didChange",
        json!({
            "textDocument": {"uri": URI, "version": version},
            "contentChanges": [{"text": text}],
        }),
    )
    .await;
}

/// 输入脚本的一步：把**当前文本**的 `[offset, offset + delete)` 换成 `insert`。
///
/// - `delete == 0` = 纯输入（光标处打字）；
/// - `insert == ""` = 纯删除；
/// - 两者都有 = 「选中重打」（学习者最常做的动作，例如把 `sorry` 删掉改敲 `intro`）。
///
/// 用元组而不是结构体，是为了让脚本在测试里能一行写完、一眼看出编辑形态。
pub(crate) type TypedStep<'a> = (usize, usize, &'a str);

/// **每一步**一次 didChange + 等到诊断落地，原地推进 `cur` / `version`。
///
/// 为什么不提供「跑完整条脚本再返回所有中间文本」的封装：服务器只有**最新**
/// 状态，历史文本快照拿回来也断言不了任何东西（曾据此写出一个假测试，
/// 见 `docs/design/real-input-tests.md` §2）。所以这里只给「一步」这个原语，
/// 由测试在**每步之间**做断言。
///
/// 确定性：`refresh` 每次都会发诊断（`lsp/lib.rs:183-241`），因此每步都用
/// `wait_diagnostics` 当就绪信号，**不需要 sleep**。
pub(crate) async fn type_step(
    service: &mut LspService<Backend>,
    socket: &mut ClientSocket,
    cur: &mut String,
    version: &mut i32,
    (offset, delete, insert): TypedStep<'_>,
) {
    let end = offset + delete;
    assert!(
        end <= cur.len(),
        "typing step ({offset}, {delete}) is past the end of the current text (len {})",
        cur.len()
    );
    assert!(
        cur.is_char_boundary(offset) && cur.is_char_boundary(end),
        "typing step ({offset}, {delete}) is not on char boundaries"
    );
    *cur = format!("{}{}{}", &cur[..offset], insert, &cur[end..]);
    *version += 1;
    did_change(service, *version, cur).await;
    let _ = wait_diagnostics(socket, "diagnostics after one typed step").await;
}

/// 把 `text` 展开成「在 `offset` 起逐字符输入」的步骤序列（纯函数，不驱动服务）。
///
/// 用于「前缀不触发补全、整词才触发」这类门控行为（`intro`/`apply` 的值位
/// 补全都是整词门控）：调用方逐步 `type_step`，并在**每个中间态**断言。
pub(crate) fn char_steps<'a>(text: &'a str, offset: usize) -> Vec<TypedStep<'a>> {
    let mut steps = Vec::new();
    let mut consumed = 0usize;
    for ch in text.chars() {
        let next = consumed + ch.len_utf8();
        steps.push((offset + consumed, 0, &text[consumed..next]));
        consumed = next;
    }
    steps
}

/// Read the next server→client message, failing with context if it never comes.
pub(crate) async fn next_socket(socket: &mut ClientSocket, waiting_for: &str) -> RpcRequest {
    tokio::time::timeout(TIMEOUT, socket.next())
        .await
        .unwrap_or_else(|_| panic!("timed out after {TIMEOUT:?} waiting for {waiting_for}"))
        .unwrap_or_else(|| panic!("server socket closed while waiting for {waiting_for}"))
}

/// Drain server→client messages until a publishDiagnostics for our URI arrives.
pub(crate) async fn wait_diagnostics(
    socket: &mut ClientSocket,
    waiting_for: &str,
) -> PublishDiagnosticsParams {
    loop {
        let msg = next_socket(socket, waiting_for).await;
        if msg.method() != "textDocument/publishDiagnostics" {
            continue;
        }
        let params: PublishDiagnosticsParams =
            serde_json::from_value(msg.params().cloned().unwrap_or(json!(null)))
                .expect("valid PublishDiagnosticsParams");
        if params.uri.as_str() == URI {
            return params;
        }
    }
}

pub(crate) fn code_of(diag: &Diagnostic) -> &str {
    match &diag.code {
        Some(NumberOrString::String(code)) => code,
        other => panic!("expected a string diagnostic code, got {other:?}"),
    }
}

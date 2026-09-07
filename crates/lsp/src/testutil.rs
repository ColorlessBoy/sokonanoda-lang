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

/// 带全部自定义方法注册的服务（soko/goals、soko/nextHole、soko/hints）。
pub(crate) fn test_service() -> (LspService<Backend>, ClientSocket) {
    LspService::build(Backend::new)
        .custom_method("soko/goals", Backend::goals)
        .custom_method("soko/nextHole", Backend::next_hole)
        .custom_method("soko/hints", Backend::hints)
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

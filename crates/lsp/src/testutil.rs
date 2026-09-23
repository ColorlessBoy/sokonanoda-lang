//! Shared protocol-level test helpers: drive requests/notifications through
//! an in-memory `LspService` (no stdio). Feature-specific request helpers
//! live next to the tests that use them.

use crate::Backend;
use futures::{FutureExt, StreamExt};
use serde_json::{json, Value};
use std::time::Duration;
use tower::Service;
use tower::ServiceExt;
use tower_lsp::jsonrpc::Request as RpcRequest;
use tower_lsp::lsp_types::*;
use tower_lsp::{ClientSocket, LspService};

/// 带全部自定义方法注册的服务（soko/goals、soko/nextHole、soko/hints、
/// soko/stateAt、soko/project、soko/version）。
pub(crate) fn test_service() -> (LspService<Backend>, ClientSocket) {
    LspService::build(Backend::new)
        .custom_method("soko/goals", Backend::goals)
        .custom_method("soko/nextHole", Backend::next_hole)
        .custom_method("soko/hints", Backend::hints)
        .custom_method("soko/stateAt", Backend::state_at)
        .custom_method("soko/project", Backend::project)
        .custom_method("soko/version", Backend::version)
        .finish()
}

/// Guard only: any server→client message must arrive within this budget.
///
/// 2s 曾在 CI 的 ubuntu runner 上偶发超时（负载尖峰 + LSP 测试全并行，
/// 2026-09-13 两次 ci 红、本地与相邻提交均绿）。加宽到 30s——它只在
/// 「消息永远不来」的真回归时才会拖慢失败，平时零成本。
pub(crate) const TIMEOUT: Duration = Duration::from_secs(30);

/// **重活互斥锁**（2026-09-23）：课程规模的编译用例（`perf_course` / `perf` 的
/// 项目档）会**整门课编一遍**，几个并行就能把 CPU 抢干；而**时序敏感的跨文件刷新
/// 用例**（`editing_a_dependency_refreshes_the_open_entry`）需要"改依赖 → 下游
/// 重发"这条链在合理时间内跑完。
///
/// 实测：`cargo test -p sokonanoda-lsp --lib` 全量跑时那条**静默失败**（没有任何
/// panic 文本），而 `--skip perf_course` 立刻全绿（151 通过、3.4s）、单跑 5/5 过、
/// 只跑 `project` + `perf_course` 两组也过 ⇒ **是争抢，不是逻辑**。
/// 这把锁让"重课程编译"与"时序敏感的跨文件刷新"**互斥**，不动任何断言。
pub(crate) static HEAVY_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
pub(crate) const URI: &str = "file:///test.sokonanoda";

/// 0-based LSP position for a **byte** offset in the source text.
///
/// `character` 是**字符数**（= 行内 char 下标），不是字节差：LSP 的 position 是
/// 字符单位，而 `position_to_offset`（`lib.rs`）按 `char_indices().nth(n)` 解释它。
/// 从前这里算的是字节差，于是**任何含多字节符号的行都会偏**——`⊗`（3 字节）后面
/// 的位置差 2，光标落到隔壁 token 上（实测：想 hover `⊗` 却 hover 到了 `b`）。
/// ASCII 行上两种算法恒等，所以这个 bug 只在非 ASCII 夹具里显形。
pub(crate) fn lsp_pos(src: &str, offset: usize) -> Position {
    let before = &src[..offset];
    let line = before.matches('\n').count() as u32;
    let line_start = before.rfind('\n').map(|i| i + 1).unwrap_or(0);
    let character = src[line_start..offset].chars().count() as u32;
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

/// `initialize` 带会话根（I16 P5：项目模式要它来定位模块根）。
pub(crate) async fn handshake_with_root(service: &mut LspService<Backend>, root: &Url) {
    let init = RpcRequest::build("initialize")
        .params(json!({"capabilities": {}, "rootUri": root}))
        .id(1)
        .finish();
    call(service, init).await.expect("initialize must answer");
}

/// 通知 + **并发排空**：服务器可能在一次通知里连发多条诊断（多文档项目里改
/// 依赖会同时刷新下游），而客户端 socket 缓冲有限——测试若先 `await` 通知处理完
/// 再去读，服务端的 send 就会等测试、测试又在等通知，直接死锁（I16 P5 实测：
/// 两条文档时必挂）。这里边处理边收，处理完再把队列里剩下的取走（不等待新消息）。
pub(crate) async fn notify_with_drain(
    service: &mut LspService<Backend>,
    socket: &mut ClientSocket,
    method: &'static str,
    params: Value,
    expect: &[Url],
) -> Vec<PublishDiagnosticsParams> {
    let mut collected: Vec<PublishDiagnosticsParams> = Vec::new();
    {
        let notify = notify(service, method, params);
        tokio::pin!(notify);
        loop {
            tokio::select! {
                _ = &mut notify => break,
                msg = socket.next() => match msg {
                    Some(msg) => push_diagnostics(&mut collected, msg),
                    None => break,
                },
            }
        }
    }
    // 服务端处理完 ⇒ 它发的消息都已经在队列里，直接取（非阻塞）。
    while let Some(Some(msg)) = socket.next().now_or_never() {
        push_diagnostics(&mut collected, msg);
    }
    // **T-A30 起编译在别的任务里跑**：通知返回时诊断还没到（这正是"不阻塞消息
    // 循环"的代价）。所以这里**等到期望的每份文档都发过一轮**，再收队列里剩下
    // 的——否则测试量到的是"还没编"的中间态，而它断言的是编译之后的样子。
    // 已经在 drain 阶段收到的不再等（否则会等一条永远不来的第二条）。
    for uri in expect {
        if collected.iter().any(|params| &params.uri == uri) {
            continue;
        }
        let params = wait_diagnostics_for(socket, uri, "drained notify").await;
        collected.push(params);
    }
    while let Some(Some(msg)) = socket.next().now_or_never() {
        push_diagnostics(&mut collected, msg);
    }
    collected
}

fn push_diagnostics(collected: &mut Vec<PublishDiagnosticsParams>, msg: RpcRequest) {
    if msg.method() != "textDocument/publishDiagnostics" {
        return;
    }
    let params: PublishDiagnosticsParams =
        serde_json::from_value(msg.params().cloned().unwrap_or(json!(null)))
            .expect("valid PublishDiagnosticsParams");
    collected.push(params);
}

/// 指定 URI 的 didOpen（多文件项目测试用；`did_open` 固定单 URI 夹具）。
pub(crate) async fn did_open_at(service: &mut LspService<Backend>, uri: &Url, text: &str) {
    notify(
        service,
        "textDocument/didOpen",
        json!({"textDocument": {
            "uri": uri, "languageId": "sokonanoda", "version": 1, "text": text
        }}),
    )
    .await;
}

/// 指定 URI 的 didChange（多文件项目测试用；`did_change` 固定单 URI 夹具）。
///
/// 当前没有测试消费它：跨文件失效测试属于 P5 余项（见
/// `crates/lsp/src/tests/project.rs` 末尾说明），等那条测试落地时它会立刻有用。
#[allow(dead_code)]
pub(crate) async fn did_change_at(
    service: &mut LspService<Backend>,
    uri: &Url,
    version: i32,
    text: &str,
) {
    notify(
        service,
        "textDocument/didChange",
        json!({
            "textDocument": {"uri": uri, "version": version},
            "contentChanges": [{"text": text}],
        }),
    )
    .await;
}

/// 排空版 didOpen：返回这一次通知里服务器发出的全部诊断（可能不止一份文档）。
pub(crate) async fn did_open_at_drained(
    service: &mut LspService<Backend>,
    socket: &mut ClientSocket,
    uri: &Url,
    text: &str,
) -> Vec<PublishDiagnosticsParams> {
    notify_with_drain(
        service,
        socket,
        "textDocument/didOpen",
        json!({"textDocument": {
            "uri": uri, "languageId": "sokonanoda", "version": 1, "text": text
        }}),
        std::slice::from_ref(uri),
    )
    .await
}

/// 排空版 didChange（多文档项目：改一份会让下游重新编译并再发诊断）。
pub(crate) async fn did_change_at_drained(
    service: &mut LspService<Backend>,
    socket: &mut ClientSocket,
    uri: &Url,
    version: i32,
    text: &str,
) -> Vec<PublishDiagnosticsParams> {
    notify_with_drain(
        service,
        socket,
        "textDocument/didChange",
        json!({
            "textDocument": {"uri": uri, "version": version},
            "contentChanges": [{"text": text}],
        }),
        std::slice::from_ref(uri),
    )
    .await
}

/// 排空版 didChange，**等指定的每一份文档都发过一轮**（跨文件刷新用）。
///
/// 为什么需要它：`did_change_at_drained` 只等**被改的那份**文档的诊断，而
/// 改依赖会让**下游**重新编译并稍后发诊断（T-A30 起编译在别的任务里跑）——
/// 在快机器上"稍后"落在排水窗口里（**碰巧过**），在 CI 的慢 runner 上落到窗口
/// 外（**假红**）。实测：2026-09-23 CI 上 `editing_a_dependency_refreshes_the_open_entry`
/// 与 `perf_project_dependency_edit_refreshes_dependents` 就是这么红的，本机全绿。
pub(crate) async fn did_change_at_drained_expecting(
    service: &mut LspService<Backend>,
    socket: &mut ClientSocket,
    uri: &Url,
    version: i32,
    text: &str,
    expect: &[Url],
) -> Vec<PublishDiagnosticsParams> {
    notify_with_drain(
        service,
        socket,
        "textDocument/didChange",
        json!({
            "textDocument": {"uri": uri, "version": version},
            "contentChanges": [{"text": text}],
        }),
        expect,
    )
    .await
}

/// 排空版 didChange，**等到 `expect` 的每一份文档都发过一轮、且 `done` 成立**。
///
/// 为什么还要 `done`：跨文件刷新时下游可能**先发一轮旧的**（上一趟编译的结果）、
/// 再发一轮新的。只等"发过一轮"会抓到旧的那份 ⇒ 断言假红（本机与 CI 都实测到过：
/// `editing_a_dependency_refreshes_the_open_entry`）。判据应当是**内容**而不是
/// "有没有发过"。
pub(crate) async fn did_change_until(
    service: &mut LspService<Backend>,
    socket: &mut ClientSocket,
    uri: &Url,
    version: i32,
    text: &str,
    expect: &[Url],
    done: impl Fn(&[PublishDiagnosticsParams]) -> bool,
) -> Vec<PublishDiagnosticsParams> {
    let deadline = tokio::time::Instant::now() + TIMEOUT;
    let mut collected: Vec<PublishDiagnosticsParams> = Vec::new();
    loop {
        let batch =
            did_change_at_drained_expecting(service, socket, uri, version, text, expect).await;
        collected.extend(batch);
        if done(&collected) {
            return collected;
        }
        if tokio::time::Instant::now() >= deadline {
            return collected;
        }
        // 再推一次同样的 didChange：**同文本通知**不重编（T-A21 的短路），
        // 但会把已经算好的结果再发一遍——用它把"晚到的第二轮"取出来。
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

/// 指定 URI 的 didClose（当前没有测试消费它：多文档刷新是 P5 余项，
/// 见 `crates/lsp/src/tests/project.rs` 末尾的说明）。
#[allow(dead_code)]
pub(crate) async fn did_close_at(service: &mut LspService<Backend>, uri: &Url) {
    notify(
        service,
        "textDocument/didClose",
        json!({"textDocument": {"uri": uri}}),
    )
    .await;
}

/// Drain until a publishDiagnostics for `uri` arrives (any order across docs).
pub(crate) async fn wait_diagnostics_for(
    socket: &mut ClientSocket,
    uri: &Url,
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
        if &params.uri == uri {
            return params;
        }
    }
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

//! **S5 的两条核心判据**（设计 `docs/design/dialect-config.md` §2 第 3 档 / §3.4 / §4）：
//!
//! ① **两次 `initialize`**：`initializationOptions.dialect` 是 `strict` ⇒ `b2` 被拒、
//!    `loose`（或缺省）⇒ 接受；
//! ② **一次 `workspace/didChangeConfiguration`**：先 loose（接受）⇒ 推一条 strict
//!    通知 ⇒ **同一个会话、不重启**再判同一份文件 ⇒ 必须被拒。
//!
//! 夹具取 `crates/cli/tests/dialect_flag.rs` 的 `REPRO` 形状：`b2` 是**唯一**能区分
//! 两档的声明（loose ⇒ checked，strict ⇒ `elab-implicit-argument-unsolved`）——
//! 所以每条断言都能先证明"两档真的不同"（判据有咬合力，不是空转）。

use super::*;

const REPRO: &str = "\
def h {α : Type} {β : Type} (f : α → β) (a : α) : β := f a

def b1 : Nat := h (fun (n : Nat) => n) 5
def b2 : Nat := h Nat Nat (fun (n : Nat) => n) 5
def b3 : Nat := @h Nat Nat (fun (n : Nat) => n) 5
";

/// 严格档拒 `b2` 的那条诊断码（与 CLI 侧 S3/S4 的判据同一个码）。
const STRICT_CODE: &str = "elab-implicit-argument-unsolved";

fn codes(diagnostics: &[Diagnostic]) -> Vec<&str> {
    diagnostics
        .iter()
        .filter(|d| d.severity == Some(DiagnosticSeverity::ERROR))
        .map(code_of)
        .collect()
}

/// `initialize`（可带 `initializationOptions`）+ `didOpen` ⇒ 这一版的诊断。
async fn open_with(initialization_options: Option<serde_json::Value>) -> Vec<Diagnostic> {
    let (mut service, mut socket) = test_service();
    let mut params = json!({"capabilities": {}});
    if let Some(options) = initialization_options {
        params["initializationOptions"] = options;
    }
    let init = RpcRequest::build("initialize")
        .params(params)
        .id(1)
        .finish();
    call(&mut service, init)
        .await
        .expect("initialize must answer");
    did_open(&mut service, REPRO).await;
    wait_diagnostics(&mut socket, "initialize + didOpen")
        .await
        .diagnostics
}

/// 判据 ①：**两次独立的 `initialize`** 选出不同档位 ⇒ 同一份 `b2` 判定不同。
///
/// 三个服务器是三个独立的 `Backend`（同一个测试进程）：方言状态**不是**进程级
/// 静态量（`dialect.rs` 的 `DialectConfig` 挂在每个 `Compiler` 上）⇒ 并行跑的
/// 其它 LSP 用例不会被这里推进的 strict 污染。
#[tokio::test]
async fn two_initializations_pick_different_dialects() {
    let strict = open_with(Some(json!({"dialect": "strict"}))).await;
    assert_eq!(
        codes(&strict),
        vec![STRICT_CODE],
        "`initializationOptions.dialect = strict` 必须拒 `b2`（实际诊断：{strict:?}）"
    );

    let loose = open_with(Some(json!({"dialect": "loose"}))).await;
    assert!(
        codes(&loose).is_empty(),
        "`dialect = loose` 必须接受 `b2`（实际诊断：{loose:?}）"
    );

    // 缺省 = **不表态** ⇒ 落到环境变量、再落到默认 `loose`（与今天逐字节相同）。
    let absent = open_with(None).await;
    assert!(
        codes(&absent).is_empty(),
        "没有 `initializationOptions` 时必须保持旧行为（loose，实际诊断：{absent:?}）"
    );

    // 大小写不敏感（与 CLI 旗标 / 文件指令共用 `Dialect::parse_keyword`）。
    let upper = open_with(Some(json!({"dialect": "STRICT"}))).await;
    assert_eq!(
        codes(&upper),
        vec![STRICT_CODE],
        "关键字大小写不敏感（实际诊断：{upper:?}）"
    );
}

/// 判据 ②：**一次配置通知** ⇒ 同一个会话里判定翻转（不重启、不重新 initialize、
/// 不重发 didOpen/didChange）。
///
/// **"没重启"的证据**（三层，任一层单独都不够）：
/// 1. 整个用例只发过**一次** `initialize`（下面没有任何第二次）；
/// 2. 服务器进程的 `soko/version.pid` 前后相同；
/// 3. 同一个文档表还在：`textDocument/documentSymbol` 仍能答出这份文档的声明 ——
///    重启的服务器没有打开文档，这里会答空。
#[tokio::test]
async fn a_configuration_change_flips_the_verdict_without_a_restart() {
    let (mut service, mut socket) = test_service();
    let init = RpcRequest::build("initialize")
        .params(json!({
            "capabilities": {},
            "initializationOptions": {"dialect": "loose"},
        }))
        .id(1)
        .finish();
    call(&mut service, init)
        .await
        .expect("initialize must answer");

    did_open(&mut service, REPRO).await;
    let before = wait_diagnostics(&mut socket, "loose didOpen").await;
    assert!(
        codes(&before.diagnostics).is_empty(),
        "热身条件不成立：loose 档下 `b2` 必须通过（实际：{:?}）",
        before.diagnostics
    );
    let pid_before = server_pid(&mut service).await;

    // **只推设置**：不 didChange、不 didClose/didOpen、不 initialize。
    notify(
        &mut service,
        "workspace/didChangeConfiguration",
        json!({"settings": {"sokonanoda": {"dialect": {"strictImplicitArgs": true}}}}),
    )
    .await;
    let after = wait_diagnostics(&mut socket, "hot swap to strict").await;
    assert_eq!(
        codes(&after.diagnostics),
        vec![STRICT_CODE],
        "热切换后同一份 `b2` 必须被拒（实际：{:?}）",
        after.diagnostics
    );

    // 再切回 loose：翻转是双向的（不是"一次性的粘滞状态"）。
    notify(
        &mut service,
        "workspace/didChangeConfiguration",
        json!({"settings": {"sokonanoda": {"dialect": "loose"}}}),
    )
    .await;
    let back = wait_diagnostics(&mut socket, "hot swap back to loose").await;
    assert!(
        codes(&back.diagnostics).is_empty(),
        "切回 loose 后必须重新接受 `b2`（实际：{:?}）",
        back.diagnostics
    );

    // 证据 2：同一个服务器进程（`soko/version` 的 pid 不变）。
    let pid_after = server_pid(&mut service).await;
    assert_eq!(
        pid_before, pid_after,
        "pid 变了 ⇒ 服务器被重启了，热切换的判据不成立"
    );
    // 证据 3：文档表还在（重启会丢）。`documentSymbol` 为 null/空 ⇒ 会话没了。
    let symbols = call(
        &mut service,
        RpcRequest::build("textDocument/documentSymbol")
            .params(json!({"textDocument": {"uri": URI}}))
            .id(4)
            .finish(),
    )
    .await
    .expect("documentSymbol must answer");
    assert!(
        symbols
            .as_array()
            .is_some_and(|items| items.iter().any(|item| item["name"] == "b2")),
        "文档表丢了 ⇒ 会话不是同一个（实际 documentSymbol：{symbols}）"
    );
}

/// `soko/version` 的读数（`pid` 是"服务器有没有被重启"的直接证据）。
async fn server_pid(service: &mut LspService<Backend>) -> u64 {
    let answer = call(
        service,
        RpcRequest::build("soko/version")
            .params(json!({}))
            .id(3)
            .finish(),
    )
    .await
    .expect("soko/version must answer");
    answer["pid"]
        .as_u64()
        .unwrap_or_else(|| panic!("soko/version 必须带 pid：{answer}"))
}

//! **S5 端到端**（真 `sokonanoda-lsp` 子进程 + stdio JSON-RPC）：设计
//! `docs/design/dialect-config.md` §2 第 3 档 / §3.4。
//!
//! 三条判据，全部断言**用户可见的结果**（`publishDiagnostics` 里有没有
//! `elab-implicit-argument-unsolved`）：
//! 1. `initializationOptions.dialect` 的 `strict` / `loose` / **缺省** 三态；
//! 2. `workspace/didChangeConfiguration` **热切换**：同一个子进程（pid 不变）、
//!    只发过一次 `initialize`、不重发 `didOpen`/`didChange`，判定双向翻转；
//! 3. **优先级**（设计 §2 的表）：`SOKO_STRICT_AT` **高于**编辑器那一档。
//!
//! 为什么是集成测试：单元测试跑在同一进程里，`pid` 相等是平凡的；这里 server 是
//! 独立子进程，"没重启"才有可观察的证据。夹具与 `crates/cli/tests/dialect_flag.rs`
//! 的 `REPRO` 逐字相同（`b2` 是唯一能区分两档的声明）。

mod common;

use common::Client;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

const REPRO: &str = "\
def h {α : Type} {β : Type} (f : α → β) (a : α) : β := f a

def b1 : Nat := h (fun (n : Nat) => n) 5
def b2 : Nat := h Nat Nat (fun (n : Nat) => n) 5
def b3 : Nat := @h Nat Nat (fun (n : Nat) => n) 5
";

const STRICT_CODE: &str = "elab-implicit-argument-unsolved";

/// 每个用例一份独立目录与缓存（cargo 测试并行跑，共用路径会互相覆盖）。
fn fixture(tag: &str) -> (PathBuf, PathBuf) {
    let dir = std::env::temp_dir().join(format!("soko-s5-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture dir");
    let cache = dir.join("cache");
    std::fs::create_dir_all(&cache).expect("cache dir");
    // 夹具落盘（URI 走 `canonicalize`；不存在的路径在 macOS 上不会规范化到
    // `/private/var`，两边字符串可能对不上 —— 既有测试踩过这个形状）。
    std::fs::write(dir.join("Probe.sokonanoda"), REPRO).expect("write fixture");
    (dir, cache)
}

/// 起一个**真的** server 子进程；`SOKO_STRICT_AT` 显式清空（否则开发机的环境值
/// 会让"缺省/loose"那几臂跟着环境漂移 —— 判据不许随环境变）。
fn start(cache: &Path, env: &[(&str, &str)]) -> Client {
    let mut all: Vec<(&str, &str)> = vec![("SOKO_STRICT_AT", "")];
    all.extend_from_slice(env);
    Client::start_traced_with_env(cache, &all)
}

/// `initialize`（可带 `initializationOptions`）+ `initialized`。
fn initialize(client: &mut Client, root: &Path, options: Option<Value>) {
    let mut params = json!({
        "processId": null,
        "rootUri": Client::file_uri(root),
        "capabilities": {},
    });
    if let Some(options) = options {
        params["initializationOptions"] = options;
    }
    client.send(json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": params,
    }));
    client.wait_for(|message| message.get("id") == Some(&json!(1)));
    client.send(json!({"jsonrpc": "2.0", "method": "initialized", "params": {}}));
}

/// `publishDiagnostics.diagnostics` 里的**错误码**（只看 ERROR，`sorry` 之类的
/// 警告不参与判定）。
fn error_codes(diagnostics_json: &str) -> Vec<String> {
    let diagnostics: Value = serde_json::from_str(diagnostics_json).expect("diagnostics JSON");
    diagnostics
        .as_array()
        .expect("diagnostics 必须是数组")
        .iter()
        .filter(|diagnostic| diagnostic["severity"] == json!(1))
        .filter_map(|diagnostic| diagnostic["code"].as_str().map(str::to_string))
        .collect()
}

fn pid(client: &mut Client) -> u64 {
    let answer = client.request(9, "soko/version", json!({}));
    answer["result"]["pid"]
        .as_u64()
        .unwrap_or_else(|| panic!("soko/version 必须带 pid：{answer}"))
}

/// 判据 1：`initializationOptions` 的三种形态 ⇒ 三种判定。
#[test]
fn initialization_options_choose_the_dialect() {
    // strict：b2 被拒。
    let (dir, cache) = fixture("init-strict");
    let mut client = start(&cache, &[]);
    initialize(&mut client, &dir, Some(json!({"dialect": "strict"})));
    let uri = Client::file_uri(&dir.join("Probe.sokonanoda"));
    client.did_open(&uri, REPRO);
    let diagnostics = client.diagnostics_for(&uri);
    assert_eq!(
        error_codes(&diagnostics),
        vec![STRICT_CODE.to_string()],
        "strict 初始化后 `b2` 必须出诊断（实际：{diagnostics}）"
    );

    // loose：接受。
    let (dir, cache) = fixture("init-loose");
    let mut client = start(&cache, &[]);
    initialize(&mut client, &dir, Some(json!({"dialect": "loose"})));
    let uri = Client::file_uri(&dir.join("Probe.sokonanoda"));
    client.did_open(&uri, REPRO);
    let diagnostics = client.diagnostics_for(&uri);
    assert_eq!(
        error_codes(&diagnostics),
        Vec::<String>::new(),
        "loose 初始化后 `b2` 必须通过（实际：{diagnostics}）"
    );

    // 缺省：**不表态** ⇒ 与今天一样（loose）。
    let (dir, cache) = fixture("init-absent");
    let mut client = start(&cache, &[]);
    initialize(&mut client, &dir, None);
    let uri = Client::file_uri(&dir.join("Probe.sokonanoda"));
    client.did_open(&uri, REPRO);
    let diagnostics = client.diagnostics_for(&uri);
    assert_eq!(
        error_codes(&diagnostics),
        Vec::<String>::new(),
        "没有 initializationOptions 时必须保持旧行为（实际：{diagnostics}）"
    );
}

/// 判据 2：配置通知热切换 —— **同一个进程**里判定双向翻转。
///
/// 「没重启」的证据：① 整个用例只发过一次 `initialize`（且没有再发）；
/// ② `soko/version.pid` 前后相同；③ 不重发 `didOpen`/`didChange`。
#[test]
fn a_configuration_change_flips_the_verdict_without_a_restart() {
    let (dir, cache) = fixture("hot-swap");
    let mut client = start(&cache, &[]);
    initialize(&mut client, &dir, Some(json!({"dialect": "loose"})));
    let uri = Client::file_uri(&dir.join("Probe.sokonanoda"));
    client.did_open(&uri, REPRO);
    let before = client.diagnostics_for(&uri);
    assert_eq!(
        error_codes(&before),
        Vec::<String>::new(),
        "热身条件不成立：loose 档下 `b2` 必须通过（实际：{before}）"
    );
    let pid_before = pid(&mut client);

    // **只推设置**（不重启、不重发 initialize/didOpen/didChange）。
    client.send(json!({
        "jsonrpc": "2.0",
        "method": "workspace/didChangeConfiguration",
        "params": {"settings": {"sokonanoda": {"dialect": "strict"}}},
    }));
    let after = client.diagnostics_for(&uri);
    assert_eq!(
        error_codes(&after),
        vec![STRICT_CODE.to_string()],
        "热切换后同一份 `b2` 必须被拒（实际：{after}）"
    );

    // 再切回 loose：翻转是双向的，不是粘滞状态。
    client.send(json!({
        "jsonrpc": "2.0",
        "method": "workspace/didChangeConfiguration",
        "params": {"settings": {"sokonanoda": {"dialect": "loose"}}},
    }));
    let back = client.diagnostics_for(&uri);
    assert_eq!(
        error_codes(&back),
        Vec::<String>::new(),
        "切回 loose 后必须重新接受 `b2`（实际：{back}）"
    );

    assert_eq!(
        pid_before,
        pid(&mut client),
        "pid 变了 ⇒ 服务器被重启了，热切换的判据不成立"
    );
}

/// 判据 3：**优先级**（设计 §2 的表）：环境变量 `SOKO_STRICT_AT` **高于**编辑器。
///
/// 两臂各起一个子进程（环境变量是进程级的，不能在同进程里来回改）：
/// * `SOKO_STRICT_AT=0` + `initializationOptions: strict` ⇒ **loose 赢**（接受 `b2`）；
/// * `SOKO_STRICT_AT=1` + `initializationOptions: loose` ⇒ **strict 赢**（拒 `b2`）。
#[test]
fn the_environment_outranks_the_editor() {
    let (dir, cache) = fixture("env-off-editor-strict");
    let mut client = start(&cache, &[("SOKO_STRICT_AT", "0")]);
    initialize(&mut client, &dir, Some(json!({"dialect": "strict"})));
    let uri = Client::file_uri(&dir.join("Probe.sokonanoda"));
    client.did_open(&uri, REPRO);
    let diagnostics = client.diagnostics_for(&uri);
    assert_eq!(
        error_codes(&diagnostics),
        Vec::<String>::new(),
        "`SOKO_STRICT_AT=0` 必须盖住编辑器的 strict（实际：{diagnostics}）"
    );

    let (dir, cache) = fixture("env-on-editor-loose");
    let mut client = start(&cache, &[("SOKO_STRICT_AT", "1")]);
    initialize(&mut client, &dir, Some(json!({"dialect": "loose"})));
    let uri = Client::file_uri(&dir.join("Probe.sokonanoda"));
    client.did_open(&uri, REPRO);
    let diagnostics = client.diagnostics_for(&uri);
    assert_eq!(
        error_codes(&diagnostics),
        vec![STRICT_CODE.to_string()],
        "`SOKO_STRICT_AT=1` 必须盖住编辑器的 loose（实际：{diagnostics}）"
    );
}

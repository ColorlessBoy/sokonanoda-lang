//! **§11.22 的判据**：静默期**只保护"写紧跟写"** —— 连打被合并，开档后第一刀不等 ✓。
//!
//! ## 为什么需要它（本轮改动的安全性论证）
//!
//! 改前：只要这份文档**上一次编译**超过 `SLOW_REBUILD`（150ms）⇒ 每一条编辑都等满静默期
//! （默认 120ms）⇒ "**打开 → 读一眼 → 敲**"的第一刀白等 120ms ✗
//! （实测 `perf_course_first_keystroke_after_open`：205.6ms 里 ≈120ms 是纯等；
//! `SOKO_DEBOUNCE_MS=0` 那一臂 95.0ms ✓）。
//!
//! 改后（§11.22）：`schedule` 里判"**这次编辑到来时这份文档已经有待编/在飞的任务**"
//! （clangd 的"写紧跟写"字面义）⇒ 只有**连打**才等 ✓。
//!
//! ⇒ 本文件钉**两件**都不许丢：① **连打被合并**（编译次数 < 编辑次数）✓；
//! ② **间隔够大（≈"读一眼"）时立刻编**（每条都编 ⇒ 不受静默期拖累）✓。
//! 少一条就证明不了"改对了" —— 只证明"更坏了"或"没生效" ✗。

mod common;

use common::Client;
use serde_json::json;

fn course_root() -> Option<std::path::PathBuf> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("courses")
        .join("set-theory");
    root.join("sokonanoda.toml").is_file().then_some(root)
}

/// 第 `k` 刀：在 `Set.mem_image α β f A<k 个空格>y` 里多插一个空格 ⇒ **每刀都是新文本** ✓
/// （同文本会被 T-A21 短路 ⇒ 量不到编译 ✗）。
fn edit_k(base: &str, k: usize) -> String {
    let marker = "Set.mem_image α β f A";
    let at = base.find(marker).expect("夹具前提：锚点必须在");
    let rest = &base[at + marker.len()..];
    let spaces = rest.chars().take_while(|c| *c == ' ').count();
    let mut out = String::with_capacity(base.len() + 1);
    out.push_str(&base[..at + marker.len()]);
    out.push_str(&" ".repeat(spaces + 1));
    out.push_str(&rest[spaces..]);
    let _ = k;
    out
}

/// 连打 5 刀 ⇒ 编译次数必须**少于** 5（静默期把"写紧跟写"合并了 ✓）。
#[test]
fn a_burst_of_edits_is_still_coalesced() {
    let Some(root) = course_root() else {
        eprintln!("跳过：找不到 courses/set-theory/sokonanoda.toml");
        return;
    };
    let rel = "units/I.3/unit08-images-preimages.sokonanoda";
    let path = root.join(rel);
    let Ok(text) = std::fs::read_to_string(&path) else {
        eprintln!("跳过：读不到 {}", path.display());
        return;
    };
    let cache = std::env::temp_dir().join(format!("soko-debounce-burst-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&cache);
    // **必须真编一次**（`NO_PROJECT_ARTIFACTS=1` ⇒ 开档走冷编）⇒ 记下"重建慢"（≥150ms）
    // ⇒ 否则静默期根本不生效，本判据空转 ✗。
    let mut client =
        Client::start_traced_with_env(&cache, &[("SOKONANODA_NO_PROJECT_ARTIFACTS", "1")]);
    let uri = Client::file_uri(&path);
    let _ = client.open(&root, &uri, &text);
    let _ = client.wait_for_trace_after(0);
    let _ = client.settled_compile_count();
    let before = client.compile_count();

    // **连打**：5 刀、每刀间隔 30ms（≪ 静默期 120ms），且**不等诊断**（fire-and-forget ✓）。
    let mut current = text.clone();
    for k in 0..5usize {
        current = edit_k(&current, k);
        client.send(json!({
            "jsonrpc": "2.0", "method": "textDocument/didChange",
            "params": {
                "textDocument": {"uri": uri, "version": 2 + k as i64},
                "contentChanges": [{"text": current}],
            },
        }));
        std::thread::sleep(std::time::Duration::from_millis(30));
    }
    let _ = client.settled_compile_count();
    let compiles = client.compile_count() - before;
    assert!(
        compiles >= 1,
        "连打后至少要编一次（否则是没跑到 ✗）—— 判据会空转"
    );
    assert!(
        compiles < 5,
        "**连打保护丢了** ✗：5 刀只允许被合并成更少的编译（实测 {compiles} 次）\
         —— 静默期必须对「写紧跟写」生效"
    );
    println!("PERF debounce-burst: edits=5 compiles={compiles}");
    let _ = client.request(99, "shutdown", json!(null));
    let _ = std::fs::remove_dir_all(&cache);
}

/// **对照臂**：每刀之间等诊断回来（≈"读一眼再敲"）⇒ **每条都该编**（不该被静默期拖）✓。
#[test]
fn spaced_out_edits_are_not_debounced() {
    let Some(root) = course_root() else {
        eprintln!("跳过：找不到 courses/set-theory/sokonanoda.toml");
        return;
    };
    let rel = "units/I.3/unit08-images-preimages.sokonanoda";
    let path = root.join(rel);
    let Ok(text) = std::fs::read_to_string(&path) else {
        return;
    };
    let cache = std::env::temp_dir().join(format!("soko-debounce-spaced-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&cache);
    let mut client =
        Client::start_traced_with_env(&cache, &[("SOKONANODA_NO_PROJECT_ARTIFACTS", "1")]);
    let uri = Client::file_uri(&path);
    let _ = client.open(&root, &uri, &text);
    let _ = client.wait_for_trace_after(0);
    let _ = client.settled_compile_count();
    let before = client.compile_count();
    let mut current = text.clone();
    for k in 0..3usize {
        current = edit_k(&current, k);
        let _ = client.did_change(&uri, 2 + k as i64, &current); // 等诊断 ⇒ 不是连打 ✓
    }
    let compiles = client.compile_count() - before;
    assert_eq!(
        compiles, 3,
        "**间隔够大的编辑不该被静默期吞掉** ✗：3 刀应当各编一次（实测 {compiles}）"
    );
    let _ = client.request(99, "shutdown", json!(null));
    let _ = std::fs::remove_dir_all(&cache);
}

//! **`soko/goals` 的 LSP 层成本读数（第 90 轮 · 平行线）** —— 补齐第 88 轮分账里那 **3.7ms** 的
//! 第一刀：① **响应体积**（响应里带整份文档文本 ⇒ 若体积大，主因可能在**载荷** ✓）；
//! ② **首次 vs 后续**（若首次明显慢 ⇒ 有一次性的成本，与"每次都要付"的固定开销不同 ✓）。
//!
//! ⚠ **本文件是新增的、从零写的** ✓ —— 前 5 次尝试都栽在"改既有文件的块"上 ✗
//! （`perf_keystroke_wallclock.rs` 那块点不动 ✓）⇒ 按纪律**换招**：只加新文件 ✓。
//! 读数**不是判据**（墙钟不许当门禁 ✓）；夹具是真课程单元（找不到就如实跳过 ✓，同按键探针纪律 ✓）。

mod common;

use common::Client;

#[test]
fn goals_response_size_and_first_vs_rest() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let root = root.join("courses/set-theory");
    let path = root.join("units/I.3/unit08-images-preimages.sokonanoda");
    let Ok(text) = std::fs::read_to_string(&path) else {
        eprintln!("跳过：读不到 {}", path.display());
        return;
    };
    let cache = std::env::temp_dir().join(format!("soko-goals-cost-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&cache);
    let mut client = Client::start(&cache);
    let uri = Client::file_uri(&path);
    // 等诊断落地（`open` 返回诊断文本 ✓ ⇒ 报告已经在 ✓）。
    let _ = client.open(&root, &uri, &text);

    let mut per = Vec::new();
    let mut size = 0usize;
    for k in 0..6 {
        let id = 7000 + k;
        let t = std::time::Instant::now();
        client.send(serde_json::json!({
            "jsonrpc": "2.0",
            "method": "soko/goals",
            "id": id,
            "params": {"textDocument": {"uri": uri}, "position": null},
        }));
        let answered = client.wait_for(|m| m.get("id") == Some(&serde_json::json!(id)));
        per.push(t.elapsed().as_secs_f64() * 1000.0);
        size = serde_json::to_string(&answered)
            .map(|s| s.len())
            .unwrap_or(0);
    }
    let first = per[0];
    let mut rest = per[1..].to_vec();
    rest.sort_by(|a, b| a.partial_cmp(b).unwrap());
    println!(
        "PERF goals-lsp: 首次 {:.1}ms · 后续 median {:.1}ms（n=5）· 响应≈{}B · 文档≈{}B · cache={}",
        first,
        rest[2],
        size,
        text.len(),
        cache.display()
    );
    let _ = std::fs::remove_dir_all(&cache);
}

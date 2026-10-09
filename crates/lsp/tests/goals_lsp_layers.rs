//! **LSP 那 ~2.8ms 的分层读数（第 97 轮 · 平行线）** —— 上一轮实测：`runs:false` 把载荷砍了 71%
//! （91 361B → 26 723B）却只把往返从 6.1ms 降到 5.0ms ⇒ **LSP 的 3.7ms 不全是载荷** ✗。
//!
//! 本文件用**三条工作量递增的请求**把它切开（同一进程、同一文档、稳态 ✓）：
//! ① `soko/version` = **空转底噪**（只答 {version,pid} ✓）；② `soko/nextHole` = 读报告但**不建
//! 声明列表** ✓；③ `soko/goals`（`runs:false`）= 建 27 条声明 + 映射 + 序列化 ✓。
//! 差值 ⇒ 定位在"**每声明**的映射/序列化"还是在"**固定**开销" ✓。读数不是门槛 ✓。

mod common;

use common::Client;

#[test]
fn where_the_remaining_lsp_milliseconds_go() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let root = root.join("courses/set-theory");
    let path = root.join("units/I.3/unit08-images-preimages.sokonanoda");
    let Ok(text) = std::fs::read_to_string(&path) else {
        eprintln!("跳过：读不到 {}", path.display());
        return;
    };
    let cache = std::env::temp_dir().join(format!("soko-goals-layers-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&cache);
    let mut client = Client::start(&cache);
    let uri = Client::file_uri(&path);
    let _ = client.open(&root, &uri, &text);

    let mut time = |method: &str, params: serde_json::Value, id: i64| -> (f64, usize) {
        let t = std::time::Instant::now();
        client.send(serde_json::json!({
            "jsonrpc": "2.0", "method": method, "id": id, "params": params,
        }));
        let answered = client.wait_for(|m| m.get("id") == Some(&serde_json::json!(id)));
        let ms = t.elapsed().as_secs_f64() * 1000.0;
        (
            ms,
            serde_json::to_string(&answered)
                .map(|s| s.len())
                .unwrap_or(0),
        )
    };
    let doc = serde_json::json!({"textDocument": {"uri": uri}, "position": null});
    // 预热（报告已落 ✓）
    let _ = time("soko/goals", doc.clone(), 9000);
    let mut collect = |method: &str, params: serde_json::Value, base: i64| -> (f64, usize) {
        let mut v = Vec::new();
        for k in 0..3 {
            v.push(time(method, params.clone(), base + k));
        }
        v.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        v[1]
    };
    let (v_ms, v_b) = collect("soko/version", serde_json::json!({}), 9100);
    let (h_ms, h_b) = collect("soko/nextHole", doc.clone(), 9200);
    let mut light = doc.clone();
    light["runs"] = serde_json::json!(false);
    let (g_ms, g_b) = collect("soko/goals", light.clone(), 9300);
    light["runs"] = serde_json::json!(true);
    let (f_ms, f_b) = collect("soko/goals", light, 9400);
    // **光标处那一条**：`soko/stateAt`（扩展真正的按键路径）与 `soko/goalAt`
    // （声明卡片那一半）在同一位置上的读数 —— 用来判断"3.5ms 到底是不是
    // 用户按键时付的那一笔"（AGENTS.md：探针必须打在用户实际动作上 ✓）。
    let at = common::Client::marker_position(&text);
    let cursor = serde_json::json!({
        "textDocument": {"uri": uri},
        "position": {"line": at.0, "character": at.1},
    });
    let (s_ms, s_b) = collect("soko/stateAt", cursor.clone(), 9500);
    let (a_ms, a_b) = collect("soko/goalAt", cursor, 9600);
    println!(
        "PERF goals-lsp-layers: version {:.1}ms/{}B · nextHole {:.1}ms/{}B · \
         goals(runs=false) {:.1}ms/{}B · goals(runs=true) {:.1}ms/{}B · \
         stateAt@cursor {:.1}ms/{}B · goalAt@cursor {:.1}ms/{}B ⇒ \
         每声明那一段 ≈{:.1}ms（goals−nextHole）· 固定段 ≈{:.1}ms（nextHole−version）· cache={}",
        v_ms,
        v_b,
        h_ms,
        h_b,
        g_ms,
        g_b,
        f_ms,
        f_b,
        s_ms,
        s_b,
        a_ms,
        a_b,
        g_ms - h_ms,
        h_ms - v_ms,
        cache.display()
    );
    let _ = std::fs::remove_dir_all(&cache);
}

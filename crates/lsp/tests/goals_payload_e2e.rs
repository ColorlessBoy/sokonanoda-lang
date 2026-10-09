//! **`soko/goals` 候选 B 的端到端读数（第 96 轮 · 平行线）** —— 真子进程、真 stdio、真课程：
//! 同一份文档上比较 `runs: true`（缺省 = 今天的行为 ✓）与 `runs: false`（候选 B ✓）的
//! **响应体积**与**往返延迟** ✓。
//!
//! 判据（**契约**）：两种模式的 `decls` **条数相同**、且 `runs:false` 那版每条声明的
//! `ty_runs`/`value_runs`/`goal_runs` **全空** ✓（其余字段由 front 层的
//! `goals_runs_optional.rs` 判据钉住 ✓）。体积/延迟是**读数**（不进门槛 ✓）。

mod common;

use common::Client;

#[test]
fn the_runs_false_mode_shrinks_the_response() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let root = root.join("courses/set-theory");
    let path = root.join("units/I.3/unit08-images-preimages.sokonanoda");
    let Ok(text) = std::fs::read_to_string(&path) else {
        eprintln!("跳过：读不到 {}", path.display());
        return;
    };
    let cache = std::env::temp_dir().join(format!("soko-goals-e2e-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&cache);
    let mut client = Client::start(&cache);
    let uri = Client::file_uri(&path);
    let _ = client.open(&root, &uri, &text);

    let mut probe = |runs: bool, id: i64| -> (usize, f64, serde_json::Value) {
        let t = std::time::Instant::now();
        client.send(serde_json::json!({
            "jsonrpc": "2.0",
            "method": "soko/goals",
            "id": id,
            "params": {"textDocument": {"uri": uri}, "position": null, "runs": runs},
        }));
        let answered = client.wait_for(|m| m.get("id") == Some(&serde_json::json!(id)));
        let ms = t.elapsed().as_secs_f64() * 1000.0;
        let size = serde_json::to_string(&answered)
            .map(|s| s.len())
            .unwrap_or(0);
        (size, ms, answered)
    };
    let _ = probe(true, 8001); // 预热一次（报告已落，量稳态 ✓）
    let mut full = Vec::new();
    let mut light = Vec::new();
    for k in 0..4 {
        full.push(probe(true, 8100 + k));
        light.push(probe(false, 8200 + k));
    }
    let med = |v: &[(usize, f64, serde_json::Value)]| {
        let mut ms: Vec<f64> = v.iter().map(|x| x.1).collect();
        ms.sort_by(|a, b| a.partial_cmp(b).unwrap());
        ms[1]
    };
    let size_full = full[3].0;
    let size_light = light[3].0;
    // **契约判据**：条数相同 + `runs:false` 那版三组 runs 全空 ✓。
    let decls_of = |v: &serde_json::Value| -> Vec<serde_json::Value> {
        v.pointer("/result/decls")
            .and_then(|d| d.as_array())
            .cloned()
            .unwrap_or_default()
    };
    let df = decls_of(&full[3].2);
    let dl = decls_of(&light[3].2);
    assert_eq!(df.len(), dl.len(), "两种模式的 decls 条数必须相同");
    assert!(!dl.is_empty(), "夹具前提：必须有声明（否则判据空转 ✗）");
    for d in &dl {
        for key in ["ty_runs", "value_runs", "goal_runs"] {
            let n = d
                .get(key)
                .and_then(|x| x.as_array())
                .map(|a| a.len())
                .unwrap_or(0);
            assert_eq!(n, 0, "`runs:false` 时 `{key}` 必须为空");
        }
    }
    println!(
        "PERF goals-payload-e2e: 响应≈{}B（runs=true）→ ≈{}B（runs=false，**−{:.0}%**）· \
         median {:.1}ms → {:.1}ms · decls {} 条 · cache={}",
        size_full,
        size_light,
        100.0 * (size_full - size_light) as f64 / size_full as f64,
        med(&full),
        med(&light),
        dl.len(),
        cache.display()
    );
    let _ = std::fs::remove_dir_all(&cache);
}

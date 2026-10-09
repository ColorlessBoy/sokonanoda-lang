//! **`soko/goals` 载荷形状的读数（第 92 轮 · 平行线）** —— 设计档
//! `docs/design/goals-payload-slimming.md` §3 的**第一步：先量候选 A 的压缩比** ✓。
//!
//! 背景（全部实测 ✓）：真子进程 `soko/goals` 往返 **5.9ms**、响应 **91 361B**（文档才 24 019B），
//! 其中 LSP 层 ≈3.7ms ⇒ 主因是**载荷**。候选 A = **稀疏 runs**：同一段连续的同 kind token
//! 只发**一个** `{start, kind, len}` ✓ ⇒ 客户端线形展开 ✓。
//!
//! 本文件**只读地**算两件事：① 现状 payload 的 JSON 体积；② 稀疏编码后的估计体积 ✓。
//! ⚠ **不是判据**（体积是读数 ✓）；判据在实现那一轮（等价性「先建先红」✓）。

use sokonanoda_front::query::QueryDoc;

fn real_unit08() -> Option<(QueryDoc, String)> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let path = root.join("courses/set-theory/units/I.3/unit08-images-preimages.sokonanoda");
    let text = std::fs::read_to_string(&path).ok()?;
    let mut doc = QueryDoc::new();
    doc.path = Some(path.clone()); // 项目模式（unit08 有 `import Set` ✓）
    doc.set_text(&text, 1, None);
    Some((doc, text))
}

#[test]
fn sparse_runs_compression_ratio_on_the_real_unit() {
    let Some((doc, text)) = real_unit08() else {
        eprintln!("跳过：读不到课程单元");
        return;
    };
    let decls = doc.goals(false).expect("goals（项目模式）");
    // ① 现状：整个 decls 的 JSON（≈ wire 的 `decls` 字段 ✓；响应还多一个 `text` ✓ 单列）。
    let now = serde_json::to_string(&decls).expect("serialize").len();
    // ② 候选 A：把每条 runs 折成"kind 变更点"（`{s, k, n}` = 起点/kind/重复数 ✓）。
    let mut sparse = 0usize;
    let mut runs_total = 0usize;
    let mut runs_sparse = 0usize;
    let fold = |runs: &[serde_json::Value]| -> (usize, usize) {
        if runs.is_empty() {
            return (0, 0);
        }
        let mut groups: Vec<(i64, String, usize)> = Vec::new();
        for r in runs {
            let kind = r
                .get("kind")
                .and_then(|k| k.as_str())
                .unwrap_or("")
                .to_string();
            let start = r.get("start").and_then(|s| s.as_i64()).unwrap_or(0);
            match groups.last_mut() {
                Some(g) if g.1 == kind => g.2 += 1,
                _ => groups.push((start, kind, 1)),
            }
        }
        let full = serde_json::to_string(runs).map(|s| s.len()).unwrap_or(0);
        let thin = groups
            .iter()
            .map(|(s, k, n)| s.to_string().len() + k.len() + n.to_string().len() + 12)
            .sum::<usize>();
        (full, thin)
    };
    for v in serde_json::to_value(&decls)
        .expect("to_value")
        .as_array()
        .cloned()
        .unwrap_or_default()
    {
        for key in ["ty_runs", "value_runs"] {
            if let Some(rs) = v.get(key).and_then(|r| r.as_array()) {
                let (a, b) = fold(rs);
                runs_total += a;
                runs_sparse += b;
                sparse += b;
            }
        }
        if let Some(gs) = v.get("goal_runs").and_then(|g| g.as_array()) {
            for g in gs {
                if let Some(rs) = g.as_array() {
                    let (a, b) = fold(rs);
                    runs_total += a;
                    runs_sparse += b;
                    sparse += b;
                }
            }
        }
    }
    let _ = sparse;
    println!(
        "PERF goals-payload: 现状 decls≈{}B（文档 text≈{}B）· runs JSON≈{}B ⇒ 稀疏后≈{}B \
         （压缩比 **{:.1}×**，省 {}B）· 预计响应 {}B → {}B",
        now,
        text.len(),
        runs_total,
        runs_sparse,
        runs_total as f64 / runs_sparse.max(1) as f64,
        runs_total.saturating_sub(runs_sparse),
        now + text.len(),
        now + text.len() - runs_total.saturating_sub(runs_sparse),
    );
}

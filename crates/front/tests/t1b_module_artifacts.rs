//! **T1-B 批 2 的跨进程判据**：库层产物让"**另一个进程**"不再 elaborate 库层。
//!
//! 为什么放**集成测试**：`by_calls_total()` 是**进程级**计数 ⇒ 放进 lib 测试会被
//! 并行的别的测试干扰（`session_reuse.rs` 的同一理由）；集成测试各自独立进程 ✓。
//!
//! ## 两臂怎么造"另一个进程"
//!
//! 同一进程里第二次跑会先命中**线程局部检查点**（T1-A）⇒ 掩盖产物那条路 ✗。
//! 所以命中臂前先 `lib_checkpoint_reset()` —— 线程局部清空之后，唯一还能省掉库层
//! 的**只有磁盘产物** ✓（这正是"另一个进程"的等价物：新进程的线程局部必然是空的 ✓）。
//!
//! ## 判据两条（缺一不可）
//!
//! ① **输出逐字节相同**（真相红线）：命中产物那条路的回调结果与冷跑**一字不差**；
//! ② **`by_calls` 真的降了**（结构计数 · 噪声免疫）：库层那几条 `by` 证明**没有重跑**
//!    ⇒ ② 同时证明 ① 不是空转（若产物没被用上，两条路会一样忙 ⇒ ② 判红 ✓）。

use sokonanoda_front::compile::{by_calls_total, CompileOptions, SourceUnit};
use sokonanoda_front::parse;
use sokonanoda_front::project::session::{
    lib_checkpoint_reset, with_project_session, with_project_session_artifacts,
};

/// 每个用例一份**独立**的模块根（pid + 纳秒 ⇒ 并发跑也不撞 ✓）。
fn temp_root(tag: &str) -> std::path::PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let root = std::env::temp_dir().join(format!("soko-t1b-{tag}-{}-{nanos}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    root
}

/// 回调收集成**可逐字节比较**的形状（`CompileOutput` 有 `PartialEq`；
/// `DocumentReport` 没有 ⇒ 走 serde 文本 ✓）。
type Collected = Vec<(usize, String, Vec<String>, Vec<String>)>;

fn collect<F>(run: F) -> Collected
where
    F: FnOnce(&mut dyn FnMut(usize, String, Vec<String>, Vec<String>)),
{
    let mut seen: Collected = Vec::new();
    run(&mut |i, out, entry_reports, lib_reports| {
        seen.push((i, out, entry_reports, lib_reports));
    });
    seen
}

#[test]
fn a_disk_artifact_skips_the_library_walk_and_keeps_the_output_identical() {
    // 库层：三条**带 `by` 的证明**（⇒ 冷跑时必然产生 `by_calls` ✓）。
    let dep = parse(
        "theorem t1 (P : Prop) (h : P) : P := by\n  exact h\n\
         theorem t2 (P Q : Prop) (hp : P) (hq : Q) : P ∧ Q := by\n  apply And.intro\n  exact hp\n  exact hq\n\
         theorem t3 (P : Prop) (h : P) : P ∧ P := by\n  apply And.intro\n  exact h\n  exact h\n",
    )
    .unwrap();
    // 入口：**不用 `by`** ⇒ 命中断言"库层没跑"时 `by_calls` 会明显更低 ✓。
    let entry = parse("import Dep\ntheorem e (P : Prop) : P → P := fun h => h\n").unwrap();
    let options = CompileOptions::default();
    let root = temp_root("skips-lib");

    let lib_units = [SourceUnit::single("Dep", &dep)];
    let entry_units: Vec<Vec<SourceUnit<'_>>> = vec![vec![SourceUnit::single("E", &entry)]];

    // ── 臂 A：冷跑（不带产物）────────────────────────────────────────────
    lib_checkpoint_reset();
    let before = by_calls_total();
    let cold = collect(|cb| {
        with_project_session(
            &lib_units,
            &entry_units,
            &options,
            |i, out, er, lr, _, _| {
                cb(
                    i,
                    serde_json::to_string(&out).unwrap(),
                    er.iter()
                        .map(|r| serde_json::to_string(r).unwrap())
                        .collect(),
                    lr.iter()
                        .map(|r| serde_json::to_string(r).unwrap())
                        .collect(),
                );
            },
        );
    });
    let cold_by = by_calls_total() - before;
    assert_eq!(cold.len(), 1, "一个入口 ⇒ 回调一次");
    assert!(
        cold_by > 0,
        "夹具必须真的跑过 `by`（否则判据没牙）：by_calls={cold_by}"
    );

    // ── 臂 B：带产物跑**第一遍**（这一遍把产物写出来）──────────────────
    lib_checkpoint_reset();
    let with_write = collect(|cb| {
        with_project_session_artifacts(
            &lib_units,
            &entry_units,
            &options,
            &root,
            |i, out, er, lr, _, _| {
                cb(
                    i,
                    serde_json::to_string(&out).unwrap(),
                    er.iter()
                        .map(|r| serde_json::to_string(r).unwrap())
                        .collect(),
                    lr.iter()
                        .map(|r| serde_json::to_string(r).unwrap())
                        .collect(),
                );
            },
        );
    });
    assert_eq!(
        with_write, cold,
        "第一遍（冷 + 写产物）的输出必须与冷跑一字不差"
    );
    let dir = sokonanoda_front::project::artifacts::dir(&root);
    let n_artifacts = std::fs::read_dir(&dir)
        .map(|it| it.filter(|e| e.is_ok()).count())
        .unwrap_or(0);
    assert!(
        n_artifacts >= 2,
        "库层趟跑完必须写出 `.bin` + `.meta.json`（实得 {n_artifacts} 个，目录 {dir:?}）"
    );

    // ── 臂 C：**清掉线程局部检查点** = 另一个进程 ⇒ 只剩磁盘产物 ────────
    lib_checkpoint_reset();
    let before = by_calls_total();
    let hit = collect(|cb| {
        with_project_session_artifacts(
            &lib_units,
            &entry_units,
            &options,
            &root,
            |i, out, er, lr, _, _| {
                cb(
                    i,
                    serde_json::to_string(&out).unwrap(),
                    er.iter()
                        .map(|r| serde_json::to_string(r).unwrap())
                        .collect(),
                    lr.iter()
                        .map(|r| serde_json::to_string(r).unwrap())
                        .collect(),
                );
            },
        );
    });
    let hit_by = by_calls_total() - before;

    // ① 真相红线：**逐字节相同**（含库层报告 C 块 —— 它是产物里那一段回放出来的 ✓）。
    assert_eq!(
        hit, cold,
        "命中产物的输出必须与冷跑**逐字节相同**（库层报告也要一字不差 ⇒ C 块对 ✓）"
    );
    // ② 结构计数：库层那三条 `by` 证明**没有重跑**。
    assert!(
        hit_by < cold_by,
        "命中产物必须真的省掉库层 elaborate（冷 {cold_by} vs 命中 {hit_by}）—— \
         若两者相等 ⇒ 产物根本没被用上 ⇒ ① 是空转 ✗"
    );

    let _ = std::fs::remove_dir_all(&root);
}

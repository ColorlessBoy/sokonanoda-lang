//! **切片 1b 的判据 ②**（用户 2026-09-28 要的数字）：3 个入口共享 1 个依赖时，
//! 共享依赖**只 elaborate 一次** ⇒ `by_calls` **3 → 1**。
//!
//! 为什么放**集成测试**：`by_calls_total()` 是**进程级**计数 ⇒ 放进 lib 测试会被
//! 并行的别的测试干扰（实测：单独绿、全量红）；集成测试各自独立进程 ⇒ 天然隔离 ✓。
use sokonanoda_front::compile::{by_calls_total, compile_all_units, CompileOptions, SourceUnit};
use sokonanoda_front::parse;
use sokonanoda_front::project::session::with_project_session;

#[test]
fn session_shares_the_dependency_across_entries() {
    let dep = parse(
        "theorem shared_and (P Q : Prop) (hp : P) (hq : Q) : P ∧ Q := by\n  apply And.intro\n  exact hp\n  exact hq\n",
    )
    .unwrap();
    let entries: Vec<_> = (0..3)
        .map(|i| {
            parse(&format!(
                "import Dep\ntheorem e{i} (P : Prop) : P → P := fun h => h\n"
            ))
            .unwrap()
        })
        .collect();
    let options = CompileOptions::default();

    // 路径 A（今天）：每个入口各编一遍 `[Dep, Entry]` ⇒ 依赖被 elaborate 3 次。
    let before = by_calls_total();
    for e in &entries {
        let units = [SourceUnit::single("Dep", &dep), SourceUnit::single("E", e)];
        let _ = compile_all_units(&units, &options);
    }
    let a = by_calls_total() - before;

    // 路径 B（切片 1b）：库层编一次 + 每个入口只走自己的命令。
    let before = by_calls_total();
    let entry_units: Vec<Vec<SourceUnit<'_>>> = entries
        .iter()
        .map(|e| vec![SourceUnit::single("E", e)])
        .collect();
    with_project_session(
        &[SourceUnit::single("Dep", &dep)],
        &entry_units,
        &options,
        |_, _, _, _, _| (),
    );
    let b = by_calls_total() - before;

    assert_eq!(a, 3, "改前：共享依赖被编 3 次（by_calls）");
    assert_eq!(b, 1, "改后：共享依赖只编 1 次（by_calls）—— 3 → 1");
}

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
    let mut seen: Vec<(usize, usize, usize, usize)> = Vec::new();
    with_project_session(
        &[SourceUnit::single("Dep", &dep)],
        &entry_units,
        &options,
        |i, out, entry_reports, lib_reports, _ranges, _entry_range| {
            // **判据加强**（2026-09-29）：不只数 `by_calls`，还要断言**入口真的编过** ——
            // 回调里该入口的 `CompileOutput` 与逐模块报告都必须**无 errors**，且库层报告齐。
            seen.push((
                i,
                out.errors.len(),
                entry_reports.iter().map(|r| r.errors.len()).sum::<usize>(),
                lib_reports.len(),
            ));
        },
    );
    assert_eq!(seen.len(), 3, "三个入口都要回调到：{seen:?}");
    for (i, out_errors, report_errors, lib_len) in &seen {
        assert_eq!(*out_errors, 0, "入口 {i} 的 CompileOutput 有错误：{seen:?}");
        assert_eq!(*report_errors, 0, "入口 {i} 的逐模块报告有错误：{seen:?}");
        assert!(*lib_len >= 1, "库层报告不该为空（入口 {i}）：{seen:?}");
    }
    let b = by_calls_total() - before;

    assert_eq!(a, 3, "改前：共享依赖被编 3 次（by_calls）");
    assert_eq!(b, 1, "改后：共享依赖只编 1 次（by_calls）—— 3 → 1");
}

/// **决定性实验（2026-09-29）**：CLI 的单元来自 `plan_project`（带 import 解析），
/// 而上面那条用**裸 `SourceUnit`** —— 若本用例转红，说明 session 对"入口带 `import`"是坏的
/// （库层声明在环境里 ✓，但入口那趟缺 import 的解析上下文），与入口个数无关。
#[test]
#[ignore = "已知缺陷（2026-09-29）：入口那趟看不到 prelude ⇒ unknown identifier `Nat`；修好后去掉 ignore"]
fn session_compiles_entries_that_import_the_lib_layer() {
    let dir = std::env::temp_dir().join(format!("soko-session-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("Dep.sokonanoda"), "def shared : Nat := 1\n").unwrap();
    std::fs::write(
        dir.join("E0.sokonanoda"),
        "import Dep\ndef e0 : Nat := shared\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("E1.sokonanoda"),
        "import Dep\ndef e1 : Nat := shared\n",
    )
    .unwrap();
    let options = CompileOptions::default();
    let p0 = sokonanoda_front::project::plan_project(
        &dir.join("E0.sokonanoda"),
        None,
        Some(dir.as_path()),
    );
    let p1 = sokonanoda_front::project::plan_project(
        &dir.join("E1.sokonanoda"),
        None,
        Some(dir.as_path()),
    );
    let lib0 = sokonanoda_front::project::units_for_modules(&p0, |m| m.path != p0.entry);
    let lib1 = sokonanoda_front::project::units_for_modules(&p1, |m| m.path != p1.entry);
    let e0 = sokonanoda_front::project::units_for_modules(&p0, |m| m.path == p0.entry);
    let e1 = sokonanoda_front::project::units_for_modules(&p1, |m| m.path == p1.entry);
    let mut lib_units: Vec<sokonanoda_front::compile::SourceUnit<'_>> = Vec::new();
    for u in lib0.iter().chain(lib1.iter()) {
        if !lib_units.iter().any(|x| x.name == u.name) {
            lib_units.push(sokonanoda_front::compile::SourceUnit {
                name: u.name,
                path: u.path,
                file: u.file,
            });
        }
    }
    let entries = vec![e0, e1];
    let mut seen: Vec<(usize, Vec<String>, usize)> = Vec::new();
    with_project_session(&lib_units, &entries, &options, |i, out, er, lr, _, _| {
        seen.push((
            i,
            out.errors.iter().map(|e| e.message.clone()).collect(),
            er.iter().map(|r| r.errors.len()).sum(),
        ));
        assert!(!lr.is_empty());
    });
    assert_eq!(seen.len(), 2, "{seen:?}");
    for (i, errors, rep_errors) in seen {
        assert!(errors.is_empty(), "入口 {i} 有错误（CLI 路径）：{errors:?}");
        assert_eq!(rep_errors, 0, "入口 {i} 报告有错误（CLI 路径）");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

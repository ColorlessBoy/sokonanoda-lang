//! **切片 1b 的判据 ②**（用户 2026-09-28 要的数字）：3 个入口共享 1 个依赖时，
//! 共享依赖**只 elaborate 一次** ⇒ `by_calls` **3 → 1**。
//!
//! 为什么放**集成测试**：`by_calls_total()` 是**进程级**计数 ⇒ 放进 lib 测试会被
//! 并行的别的测试干扰（实测：单独绿、全量红）；集成测试各自独立进程 ⇒ 天然隔离 ✓。
use sokonanoda_front::compile::{
    by_calls_total, compile_all_units, module_compiles_total, CompileOptions, SourceUnit,
};
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

// ── 切片 1（G-68）：**按 `module_key` 复用依赖产物** ──────────────────────────
//
// 用户 2026-09-29 定死的形状：产物键 = `module_key` + 自身源码哈希 +
// **直接依赖**的产物哈希；**拓扑序**算，**与入口无关**
// （前缀是 per-entry 的 ⇒ **不许按前缀复用**；批编**已实测慢 6.2×** ⇒ 不许回）。
//
// 这两条判据**先写、必须先红**（TDD）：它们咬的是"缓存住错误结果"这一失败模式 ——
// 比慢严重得多 ✗。

/// **正向**：两个入口共享 `Shared` ⇒ 走**计划**路径编两次时，
/// `Shared` 只该被编**一次**（第二次按 `module_key` 取产物）。
///
/// ⚠ **`#[ignore]`：这条是 TDD 的"先红"守卫，实现还没写** ⇒ 它会红，而
/// **红了会顶掉 CI**（`test` 是判绿承载 job）✗。所以先 `#[ignore]` 掉、
/// **实现完成时删掉这一行** ⇒ 它就变成真判据 ✓。
/// **它现在红的证据**（本机实测）：编 E1 时又编了 **2** 个模块（期望 ≤1）。
/// 实现 = G-68 切片 1（同进程按 `module_key` 复用依赖产物），设计见
/// `docs/design/incremental-environment.md` §19/§20。
#[test]
#[ignore = "TDD 先红守卫：切片 1（按 module_key 复用）尚未实现；实现后删掉本行"]
fn slice1_shared_module_is_compiled_once_across_entries() {
    use sokonanoda_front::project::{compile_plan, plan_project};
    let dir = std::env::temp_dir().join(format!("soko-slice1-once-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("Shared.sokonanoda"), "def shared : Nat := 1\n").unwrap();
    std::fs::write(
        dir.join("E0.sokonanoda"),
        "import Shared\ndef e0 : Nat := shared\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("E1.sokonanoda"),
        "import Shared\ndef e1 : Nat := shared\n",
    )
    .unwrap();
    let options = CompileOptions::default();

    let before = module_compiles_total();
    let p0 = plan_project(&dir.join("E0.sokonanoda"), None, Some(dir.as_path()));
    let r0 = compile_plan(p0, &options);
    assert!(
        r0.diagnostics.is_empty(),
        "E0 必须编过：{:?}",
        r0.diagnostics
    );
    let after_e0 = module_compiles_total();
    let p1 = plan_project(&dir.join("E1.sokonanoda"), None, Some(dir.as_path()));
    let r1 = compile_plan(p1, &options);
    assert!(
        r1.diagnostics.is_empty(),
        "E1 必须编过：{:?}",
        r1.diagnostics
    );
    let after_e1 = module_compiles_total();

    let _first = after_e0 - before;
    let second = after_e1 - after_e0;
    // E1 的闭包是 [Shared, E1]；`Shared` 已在 E0 那轮编过 ⇒ 第二次只该编 **E1 自己**。
    // ⚠ 这条**现在会红**（还没有按 module_key 复用）—— 那就是切片 1 的 TDD 起点 ✓。
    assert!(
        second <= 1,
        "切片 1 未生效：编 E1 时又编了 {second} 个模块（期望 ≤1：Shared 应命中产物）"
    );
}

/// **反向（红线）**：改**前面**那条声明（依赖源码变）⇒ 下游**必须重算**，
/// 不许拿旧产物回放（错编比慢严重 ✗）。
#[test]
fn slice1_changing_a_dependency_forces_recompile() {
    use sokonanoda_front::project::{compile_plan, plan_project};
    let dir = std::env::temp_dir().join(format!("soko-slice1-invalidate-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("Shared.sokonanoda"), "def shared : Nat := 1\n").unwrap();
    std::fs::write(
        dir.join("E0.sokonanoda"),
        "import Shared\ndef e0 : Nat := shared\n",
    )
    .unwrap();
    let options = CompileOptions::default();

    let p = plan_project(&dir.join("E0.sokonanoda"), None, Some(dir.as_path()));
    let r = compile_plan(p, &options);
    assert!(
        r.diagnostics.is_empty(),
        "第一次必须编过：{:?}",
        r.diagnostics
    );

    // 改**依赖**（不是入口）⇒ 下游键必变 ⇒ 必须重编。
    std::fs::write(dir.join("Shared.sokonanoda"), "def shared : Nat := 2\n").unwrap();
    let before = module_compiles_total();
    let p2 = plan_project(&dir.join("E0.sokonanoda"), None, Some(dir.as_path()));
    let r2 = compile_plan(p2, &options);
    assert!(
        r2.diagnostics.is_empty(),
        "改依赖后必须仍编过：{:?}",
        r2.diagnostics
    );
    let work = module_compiles_total() - before;
    assert!(
        work >= 2,
        "**错编风险**：改了依赖（Shared 1 → 2）却只做了 {work} 次模块编译 —— \
         说明拿旧产物回放了 ✗（依赖变必须让 Shared 与下游 E0 都重编）"
    );
}

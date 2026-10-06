//! **G-68 切片（2026-10-06）的判据**：一批入口**共享库层** ⇒ 共享依赖只 elaborate 一次。
//!
//! 两条断言，缺一不可：
//! 1. **等价**（红线）：走会话的那一份报告必须与**逐入口编**的那一份**逐字节相同**
//!    （`ProjectReport` 的 JSON —— 它就是进缓存、被 `query` 读的那份 ✓）；
//! 2. **省功**（缺口）：`by_calls` 从 **3 → 1**（台账 G-68 的 `expected_lean` 原文 ✓），
//!    且 `module_compiles_total` 明显下降 ✓。
//!
//! 为什么放**独立文件**：`by_calls_total()` / `module_compiles_total()` 都是**进程级**
//! 计数 ⇒ 同一个测试二进制里并行跑别的用例会互相污染（`session_reuse.rs` 的注释 ✓）。
//! 独立文件 = 独立进程 ✓。
//!
//! ## 反向验证（这条判据咬得住吗）
//!
//! 把 `compile_entries_shared` 的分组键改成"一个大并集"（不按签名分组）⇒ 断言 1
//! 立刻红 ✓ —— 那正是台账 G-68 第 11/12 棒实测到的 264 行 `compiled → failed` ✗。
//! 见本文件末尾的 `union_library_layer_is_not_equivalent`（**故意用错形状**，断言它**不**等价 ✓）。

use sokonanoda_front::compile::{by_calls_total, module_compiles_total, CompileOptions};
use sokonanoda_front::project::{
    compile_entries_shared, compile_plan_prechecked, plan_project, precheck_plan, ProjectPlan,
};

/// **计数器是进程级的** ⇒ 本文件里的用例必须**串行**（`session_reuse.rs` 的注释 ✓：
/// 并行跑别的用例会污染 `by_calls_total()` 的差值 ✗）。三处都先拿这把锁 ✓。
static COUNTER_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn exclusive() -> std::sync::MutexGuard<'static, ()> {
    COUNTER_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// 铺一个"共享库链 + N 个入口"的工程，返回 `(目录, 入口路径列表)`。
fn fixture(tag: &str, entries: usize) -> (std::path::PathBuf, Vec<std::path::PathBuf>) {
    let dir = std::env::temp_dir().join(format!("soko-g68-shared-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("lib")).unwrap();
    std::fs::write(dir.join("sokonanoda.toml"), "[project]\nname = \"g68\"\n").unwrap();
    std::fs::write(
        dir.join("lib/A.sokonanoda"),
        "def A.id (α : Type) (a : α) : α := a\n",
    )
    .unwrap();
    // **共享依赖里恰好一个 `by`** —— 判据的分度就是它（`scripts/check-recompile-factor.py` 同款）。
    std::fs::write(
        dir.join("lib/B.sokonanoda"),
        "import lib.A\n\ntheorem B.and_self (P : Prop) (h : P) : P ∧ P := by\n  apply And.intro\n  exact h\n  exact h\n",
    )
    .unwrap();
    let paths: Vec<std::path::PathBuf> = (0..entries)
        .map(|i| {
            let path = dir.join(format!("e{i}.sokonanoda"));
            std::fs::write(
                &path,
                format!(
                    "import lib.A\nimport lib.B\n\ndef e{i}_v (α : Type) (a : α) : α := A.id α a\n"
                ),
            )
            .unwrap();
            path
        })
        .collect();
    (dir, paths)
}

fn plans(paths: &[std::path::PathBuf], root: &std::path::Path) -> Vec<ProjectPlan> {
    paths
        .iter()
        .map(|path| plan_project(path, None, Some(root)))
        .collect()
}

/// 报告的可比形状：JSON（`ProjectReport` 进缓存、被 `query` 读 —— 逐字节相同才叫等价 ✓）。
fn json(report: &sokonanoda_front::project::ProjectReport) -> String {
    serde_json::to_string(report).expect("ProjectReport 必须可序列化")
}

/// **判据 1 + 2**：会话 vs 逐入口 —— 报告逐字节相同，而共享依赖只编一次。
#[test]
fn shared_entries_are_equivalent_and_compile_the_library_once() {
    let _guard = exclusive();
    let (dir, paths) = fixture("equiv", 3);
    let options = CompileOptions::default();

    // 路径 A（今天）：逐入口各编一遍整条闭包。
    let mut per_entry: Vec<String> = Vec::new();
    let before = by_calls_total();
    let modules_before = module_compiles_total();
    for plan in plans(&paths, &dir) {
        let mut plan = plan;
        precheck_plan(&mut plan, &options);
        per_entry.push(json(&compile_plan_prechecked(plan, &options, None)));
    }
    let a_by = by_calls_total() - before;
    let a_modules = module_compiles_total() - modules_before;

    // 路径 B（本切片）：一组会话，库层只编一次。
    let mut shared_plans = plans(&paths, &dir);
    let before = by_calls_total();
    let modules_before = module_compiles_total();
    let shared =
        compile_entries_shared(&mut shared_plans, &[options, options, options], 1, &|_| {});
    let b_by = by_calls_total() - before;
    let b_modules = module_compiles_total() - modules_before;

    assert_eq!(a_by, 3, "改前：3 个入口各编一次共享依赖（by_calls）");
    assert_eq!(
        b_by, 1,
        "改后：共享依赖只编一次（by_calls 3 → 1，G-68 的判据 ✓）"
    );
    // ⚠ `module_compiles_total` **只数老路**（`check::run` 那一层）⇒ 会话那条路它恒 0 ✗
    // ⇒ 它**不能**当会话的判据（这里只如实打出来，不当断言 ✓）。会话的判据 = `by_calls` ✓
    // 与 `scripts/check-recompile-factor.py` 同一把尺 ✓。
    println!("PERF g68 session: by_calls {a_by}→{b_by} · module_compiles(legacy) {a_modules}→{b_modules}");

    for (i, got) in shared.iter().enumerate() {
        let got = got
            .as_ref()
            .unwrap_or_else(|| panic!("入口 {i} 必须走会话"));
        assert_eq!(
            json(got),
            per_entry[i],
            "入口 {i} 的报告与逐入口路径**不逐字节相同** ✗ —— 会话接线改变了判定/报告"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// **组间并行**不许改变结果：`jobs=4` 与 `jobs=1` 的报告必须**逐字节相同** ✓。
///
/// 夹具刻意做成**两个库闭包**（`e0..e2` 依赖 `A+B`；`e3..e5` 依赖 `A+C`）⇒
/// 两个组 ⇒ `jobs=4` 真的会把它们分到**两个线程** ✓（单组时并行分支根本不进 ✗）。
#[test]
fn parallel_groups_give_the_same_reports_as_serial() {
    let _guard = exclusive();
    let dir = std::env::temp_dir().join(format!("soko-g68-par-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("lib")).unwrap();
    std::fs::write(dir.join("sokonanoda.toml"), "[project]\nname = \"g68\"\n").unwrap();
    std::fs::write(
        dir.join("lib/A.sokonanoda"),
        "def A.id (α : Type) (a : α) : α := a\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("lib/B.sokonanoda"),
        "import lib.A\n\ntheorem B.and_self (P : Prop) (h : P) : P ∧ P := by\n  apply And.intro\n  exact h\n  exact h\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("lib/C.sokonanoda"),
        "import lib.A\n\ntheorem C.or_self (P : Prop) (h : P) : P ∨ P := by\n  exact Or.inl h\n",
    )
    .unwrap();
    let mut paths: Vec<std::path::PathBuf> = Vec::new();
    for i in 0..6 {
        let dep = if i < 3 { "lib.B" } else { "lib.C" };
        let path = dir.join(format!("e{i}.sokonanoda"));
        std::fs::write(
            &path,
            format!(
                "import lib.A\nimport {dep}\n\ndef e{i}_v (α : Type) (a : α) : α := A.id α a\n"
            ),
        )
        .unwrap();
        paths.push(path);
    }
    let options = CompileOptions::default();
    let opts: Vec<CompileOptions> = (0..6).map(|_| options).collect();

    let serial = compile_entries_shared(&mut plans(&paths, &dir), &opts, 1, &|_| {});
    let parallel = compile_entries_shared(&mut plans(&paths, &dir), &opts, 4, &|_| {});
    for i in 0..6 {
        let s = serial[i]
            .as_ref()
            .unwrap_or_else(|| panic!("入口 {i} 必须走会话"));
        let p = parallel[i]
            .as_ref()
            .unwrap_or_else(|| panic!("入口 {i} 必须走会话"));
        assert_eq!(
            json(s),
            json(p),
            "入口 {i}：`jobs=4` 与 `jobs=1` 的报告不同 ✗ —— 组间并行改变了判定"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}
///
/// 为什么需要它：判据 1 若在两种形状下都绿，它就是**咬不住**的守卫 ✗。
/// 夹具照抄**真课程**的失败形状（台账 G-68 第 11 棒 ✓）：入口 A 自己的闭包**不含**
/// `lib/Only`，而 `lib/Only` 里声明的名字与入口 A 声明的**同名** ⇒ 并集环境里
/// 入口 A 变成"重复声明" ✗（逐入口路径里它是好的 ✓）。
#[test]
fn union_library_layer_is_not_equivalent() {
    let _guard = exclusive();
    let dir = std::env::temp_dir().join(format!("soko-g68-union-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("lib")).unwrap();
    std::fs::write(dir.join("sokonanoda.toml"), "[project]\nname = \"g68\"\n").unwrap();
    // `Only` 声明 `e0` —— 与入口 e0 自己的声明**同名**（真课程 `Reflexive` 那一类 ✓）。
    std::fs::write(dir.join("lib/Only.sokonanoda"), "def e0 : Nat := 9\n").unwrap();
    std::fs::write(
        dir.join("lib/Shared.sokonanoda"),
        "def Shared.y : Nat := 2\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("e0.sokonanoda"),
        "import lib.Shared\ndef e0 : Nat := Shared.y\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("e1.sokonanoda"),
        "import lib.Only\nimport lib.Shared\ndef e1 : Nat := Only.e0\n",
    )
    .unwrap();
    let options = CompileOptions::default();

    // ① **正确**报告：e0 走逐入口路径（闭包 = [Shared, e0]）。
    let mut p0 = plan_project(&dir.join("e0.sokonanoda"), None, Some(&dir));
    precheck_plan(&mut p0, &options);
    let correct = json(&compile_plan_prechecked(p0, &options, None));

    // ② **错形状**：并集库层 = [Only, Shared]（= e1 的库层）。
    let p0 = plan_project(&dir.join("e0.sokonanoda"), None, Some(&dir));
    let p1 = plan_project(&dir.join("e1.sokonanoda"), None, Some(&dir));
    let union_lib = sokonanoda_front::project::units_for_modules(&p1, |m| m.path != p1.entry);
    let entry0 = sokonanoda_front::project::units_for_modules(&p0, |m| m.path == p0.entry);
    let e0_lib = {
        let p = plan_project(&dir.join("e0.sokonanoda"), None, Some(&dir));
        sokonanoda_front::project::units_for_modules(&p, |m| m.path != p.entry).len()
    };
    assert!(
        union_lib.len() > e0_lib,
        "反向验证失效：并集库层（{} 个模块）必须**大于** e0 自己的库层（{e0_lib} 个）\
         —— 否则这个夹具证明不了「并集 ≠ 各自闭包」",
        union_lib.len()
    );
    let collected = sokonanoda_front::project::session::with_project_session(
        &union_lib,
        &[entry0],
        &options,
        |_i, merged, entry_reports, lib_reports, _r, _e| {
            (merged, entry_reports, lib_reports.to_vec())
        },
    );
    assert_eq!(collected.len(), 1, "会话必须回调一次");
    let (merged, entry_reports, lib_reports) = collected.into_iter().next().unwrap();
    let closure_units = sokonanoda_front::project::units_for_modules(&p0, |_| true);
    let reports = sokonanoda_front::project::merge_session_reports(
        &closure_units,
        &union_lib,
        &lib_reports,
        entry_reports,
    );
    let wrong = json(&sokonanoda_front::project::assemble_from_session(
        &p0, merged, reports,
    ));
    assert_ne!(
        wrong, correct,
        "并集库层给出了**相同**的报告 —— 那么「按签名分组」就没有存在的理由，\
         这条反向验证咬不住 ✗（判据 1 也失去了意义）"
    );
    assert!(
        wrong.contains("duplicate declaration") || wrong.contains("\"status\":\"Failed\""),
        "并集库层应当让 e0 **报错**（重复声明 `e0`）：{wrong}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// **会话里的两个入口可以声明同名**（课程里 canvas 与它的 solution 就是这种形状 ✓）。
///
/// ⚠ **本用例不隔离"登记表还原"那一条** ✗（**实测过**：把 `session.rs` 的
/// `tables = snapshot.clone()` 注释掉，它**照样绿** ✗ —— 重名判定走的是 `declars`
/// （内核环境 ✓，由 `restore_declars` 管着 ✓），不是登记表 ✗）。它守的是**会话本身**
/// （多入口共用一个 builder 时互不串味 ✓）。**咬得住那一条的是下面
/// [`a_leaked_registry_table_shows_up_as_a_raw_kernel_rejection`]** ✓。
#[test]
fn two_entries_in_one_session_may_declare_the_same_name() {
    let _guard = exclusive();
    let dir = std::env::temp_dir().join(format!("soko-g68-dup-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("lib")).unwrap();
    std::fs::write(dir.join("sokonanoda.toml"), "[project]\nname = \"g68\"\n").unwrap();
    std::fs::write(
        dir.join("lib/Shared.sokonanoda"),
        "def Shared.y : Nat := 2\n",
    )
    .unwrap();
    // **两个入口声明同一个名字**（课程里 canvas 与它的 solution 就是这种形状 ✓）。
    for i in 0..2 {
        std::fs::write(
            dir.join(format!("e{i}.sokonanoda")),
            "import lib.Shared\ndef dup : Nat := Shared.y\n",
        )
        .unwrap();
    }
    let options = CompileOptions::default();
    let paths: Vec<std::path::PathBuf> = (0..2)
        .map(|i| dir.join(format!("e{i}.sokonanoda")))
        .collect();
    let reports = compile_entries_shared(&mut plans(&paths, &dir), &[options, options], 1, &|_| {});
    for (i, report) in reports.iter().enumerate() {
        let report = report
            .as_ref()
            .unwrap_or_else(|| panic!("入口 {i} 必须走会话"));
        assert!(
            !report.has_errors(),
            "入口 {i} 报了错（登记表没还原到库层快照？）：{:?}",
            report
                .modules
                .iter()
                .flat_map(|m| m.report.errors.iter().map(|e| e.message.clone()))
                .collect::<Vec<_>>()
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// **登记表（`PassTables`）必须逐入口还原到"只有库层"的快照** ✓ —— 这条**咬得住** ✓。
///
/// **形状**（2026-10-06 实测 ✓）：入口 A 声明 `def helper : Nat := 1`；入口 B
/// （闭包只有 `lib.Shared`，**看不见 A**）写 `def uses : Nat := helper`。
///
/// * **还原了**（今天）⇒ B 报 `ElabUnknownIdentifier`：`unknown identifier \`helper\`` ✓
///   —— 那正是逐入口路径给的用户可见诊断 ✓；
/// * **不还原**（反向验证：把 `session.rs` 的 `tables = snapshot.clone()` 注释掉）⇒
///   B 报 **`KernelRejected`：`rejected: const_head_type: unknown const NamePtr(0x…)`** ✗✗
///   —— A 的 `known` 表泄进 B ⇒ B 把一个**不在它环境里**的名字 elaborate 成了常量 ⇒
///   内核兜底拒绝，**还漏出内部指针** ✗。
///
/// ⇒ 判据 = 「B 的诊断里**不许**出现内核级拒绝」✓（比"有错就行"精确得多 ✓）。
#[test]
fn a_leaked_registry_table_shows_up_as_a_raw_kernel_rejection() {
    let _guard = exclusive();
    let dir = std::env::temp_dir().join(format!("soko-g68-leak-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("lib")).unwrap();
    std::fs::write(dir.join("sokonanoda.toml"), "[project]\nname = \"g68\"\n").unwrap();
    std::fs::write(
        dir.join("lib/Shared.sokonanoda"),
        "def Shared.y : Nat := 2\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("a.sokonanoda"),
        "import lib.Shared\ndef helper : Nat := 1\n",
    )
    .unwrap();
    // B **引用 A 的私有名字**（B 的闭包只有 `lib.Shared`）。
    std::fs::write(
        dir.join("b.sokonanoda"),
        "import lib.Shared\ndef uses : Nat := helper\n",
    )
    .unwrap();
    let options = CompileOptions::default();
    let paths = vec![dir.join("a.sokonanoda"), dir.join("b.sokonanoda")];
    let reports = compile_entries_shared(&mut plans(&paths, &dir), &[options, options], 1, &|_| {});
    let b = reports[1].as_ref().expect("入口 b 必须走会话");
    let messages: Vec<String> = b
        .modules
        .iter()
        .flat_map(|m| m.report.errors.iter().map(|e| e.message.clone()))
        .collect();
    assert!(
        messages.iter().any(|m| m.contains("unknown identifier")),
        "入口 b 应当报「unknown identifier `helper`」（它看不见 a 的私有声明）：{messages:?}"
    );
    assert!(
        !messages.iter().any(|m| m.contains("const_head_type") || m.contains("KernelRejected")),
        "**登记表泄漏** ✗：入口 b 的诊断里出现了内核级拒绝（= a 的 `known` 表泄进了 b）：{messages:?}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

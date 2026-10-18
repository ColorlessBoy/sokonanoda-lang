//! 项目层单测：清单、闭包、闭包级规则与逐模块报告（都在临时目录里跑）。

use std::path::{Path, PathBuf};

use super::*;
use crate::compile::{CompileOptions, DeclStatus, PreludeMode};

fn tmp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "soko-front-project-test-{tag}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn write(dir: &Path, relative: &str, text: &str) -> PathBuf {
    let path = dir.join(relative);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create parent");
    }
    std::fs::write(&path, text).expect("write file");
    path
}

fn compile(dir: &Path, entry: &str) -> ProjectReport {
    let path = dir.join(entry);
    compile_project(&path, None, &CompileOptions::default(), None)
}

fn codes(report: &ProjectReport) -> Vec<&str> {
    report.diagnostics.iter().map(|diag| diag.code()).collect()
}

#[test]
fn imports_work_without_any_manifest() {
    let dir = tmp_dir("zero-config");
    write(&dir, "Bar.sokonanoda", "def bar : Nat := 2\n");
    write(
        &dir,
        "Main.sokonanoda",
        "import Bar\n\ndef two : Nat := bar\n",
    );
    let report = compile(&dir, "Main.sokonanoda");
    assert!(
        !report.has_errors(),
        "zero-config two-file import must compile: {:?}",
        codes(&report)
    );
    assert_eq!(report.manifest, None);
    assert_eq!(
        report
            .modules
            .iter()
            .map(|module| module.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Bar", "Main"],
        "topological order: dependencies first, entry last"
    );
    let entry = report.entry_report().expect("entry report");
    assert_eq!(entry.decls.len(), 1);
    assert!(
        entry.decls.iter().all(|decl| decl.cmd < 3),
        "entry command indices are rebased to the module: {:?}",
        entry.decls.iter().map(|decl| decl.cmd).collect::<Vec<_>>()
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn manifest_moves_the_module_root_and_nested_names_resolve() {
    let dir = tmp_dir("src-root");
    write(&dir, "sokonanoda.toml", "name = \"proj\"\nsrc = \"src\"\n");
    write(
        &dir,
        "src/Lesson/Logic.sokonanoda",
        "axiom And : Prop -> Prop -> Prop\naxiom And.intro : forall (a b : Prop), a -> b -> And a b\n",
    );
    write(
        &dir,
        "src/Canvas.sokonanoda",
        "import Lesson.Logic\n\ntheorem t (a b : Prop) (ha : a) (hb : b) : And a b := And.intro a b ha hb\n",
    );
    let report = compile(&dir, "src/Canvas.sokonanoda");
    assert!(!report.has_errors(), "diagnostics: {:?}", codes(&report));
    assert_eq!(report.root, dir.join("src"));
    assert_eq!(report.manifest, Some(dir.join("sokonanoda.toml")));
    assert_eq!(
        report.modules.first().map(|m| m.name.as_str()),
        Some("Lesson.Logic")
    );
    let _ = std::fs::remove_dir_all(&dir);
}

// ── G-12：入口与模块根必须**一起**绝对化（cwd 只参与这一步）──────────────

#[test]
fn absolute_lexical_is_a_pure_text_operation() {
    let cwd = std::env::current_dir().expect("cwd");
    let relative = absolute_lexical(Path::new("units/u.sokonanoda"));
    assert!(relative.is_absolute(), "{relative:?}");
    assert_eq!(relative, cwd.join("units/u.sokonanoda"), "尾部原样保留");
    // 已经绝对 ⇒ 原样返回（`..` 与符号链接的解析**不**在词法层的职责里）。
    let already = std::env::temp_dir().join("../u.sokonanoda");
    assert_eq!(absolute_lexical(&already), already);
    // 空路径归一为 CWD（`--root ''`、裸文件名的空 parent）：结果永远绝对且非空。
    assert_eq!(absolute_lexical(Path::new("")), cwd);
    assert!(!absolute_lexical(Path::new("")).as_os_str().is_empty());
}

#[test]
fn a_plan_keeps_entry_and_root_on_the_same_absolute_footing() {
    // "修一半"陷阱：只绝对化 entry、不绝对化 `--root` / `--no-project` 传下来的
    // `root_override` ⇒ `module_name_of_path` 的 `strip_prefix(root)` 失配 ⇒
    // 模块名退化成裸 `file_stem` ⇒ `ProjectPlan::digest`（缓存键）跟着漂。
    let plan = plan_project_with_overlay(
        Path::new("src/units/u.sokonanoda"),
        Some("def one : Nat := 1\n"),
        Some(Path::new("src")),
        &[],
    );
    assert!(plan.entry.is_absolute(), "entry: {:?}", plan.entry);
    assert!(plan.root.is_absolute(), "root: {:?}", plan.root);
    assert!(!plan.root.as_os_str().is_empty(), "模块根永不为空");
    assert_eq!(
        plan.entry.strip_prefix(&plan.root),
        Ok(Path::new("units/u.sokonanoda")),
        "entry 必须落在 root 下，模块名才有目录前缀"
    );
    assert_eq!(
        plan.entry(),
        "units.u",
        "模块名跟着 root 走，不许退化成裸 file_stem"
    );

    // 零配置退路（没有 root_override、没有清单）：root = 入口目录，同样绝对。
    let zero_config = plan_project_with_overlay(
        Path::new("src/units/u.sokonanoda"),
        Some("def one : Nat := 1\n"),
        None,
        &[],
    );
    assert!(zero_config.entry.is_absolute(), "{:?}", zero_config.entry);
    assert!(zero_config.root.is_absolute(), "{:?}", zero_config.root);
    assert!(!zero_config.root.as_os_str().is_empty());
}

#[test]
fn missing_module_reports_the_path_and_dash_hint() {
    let dir = tmp_dir("missing");
    write(&dir, "unit1-propositions.sokonanoda", "def x : Nat := 1\n");
    write(&dir, "Main.sokonanoda", "import unit1\n");
    let report = compile(&dir, "Main.sokonanoda");
    let diagnostic = report
        .diagnostics
        .iter()
        .find(|diag| diag.code() == "import-not-found")
        .expect("import-not-found");
    assert!(
        diagnostic.message.contains("主意的") || diagnostic.message.contains("你是不是想 import"),
        "dash candidates show up: {}",
        diagnostic.message
    );
    // 入口报告里也能看到这条错误（挂在 import 行上）。
    let entry = report.entry_report().expect("entry report");
    assert!(
        entry
            .errors
            .iter()
            .any(|error| error.code() == "import-not-found"),
        "the entry report carries the import error"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn case_mismatch_is_an_error_even_on_case_insensitive_filesystems() {
    let dir = tmp_dir("case");
    write(&dir, "Logic.sokonanoda", "def x : Nat := 1\n");
    write(&dir, "Main.sokonanoda", "import logic\n");
    let report = compile(&dir, "Main.sokonanoda");
    let diagnostic = report
        .diagnostics
        .iter()
        .find(|diag| diag.code() == "import-not-found")
        .expect("import-not-found");
    assert!(
        diagnostic.message.contains("大小写"),
        "case mismatch gets its own wording: {}",
        diagnostic.message
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cycles_are_reported_with_the_cycle_path() {
    let dir = tmp_dir("cycle");
    write(&dir, "A.sokonanoda", "import B\n\ndef a : Nat := 1\n");
    write(&dir, "B.sokonanoda", "import A\n\ndef b : Nat := 2\n");
    let report = compile(&dir, "A.sokonanoda");
    let diagnostic = report
        .diagnostics
        .iter()
        .find(|diag| diag.code() == "import-cycle")
        .expect("import-cycle");
    assert!(
        diagnostic.message.contains("→"),
        "cycle path is printed: {}",
        diagnostic.message
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_failed_dependency_blocks_its_importers_with_one_diagnostic() {
    let dir = tmp_dir("dep-failed");
    // Logic 有类型错误：它的下游只报一条 dependency-failed，不级联。
    write(&dir, "Logic.sokonanoda", "def bad : Nat := Prop\n");
    write(
        &dir,
        "Main.sokonanoda",
        "import Logic\n\ndef use : Nat := bad\n",
    );
    let report = compile(&dir, "Main.sokonanoda");
    let entry = report.entry_report().expect("entry report");
    assert_eq!(
        entry
            .errors
            .iter()
            .filter(|error| error.code() == "import-dependency-failed")
            .count(),
        1,
        "exactly one blocking diagnostic on the import line: {:?} (per module: {:?})",
        entry.errors.iter().map(|e| e.code()).collect::<Vec<_>>(),
        report
            .modules
            .iter()
            .map(|m| (
                m.name.as_str(),
                m.report.errors.iter().map(|e| e.code()).collect::<Vec<_>>()
            ))
            .collect::<Vec<_>>()
    );
    assert!(
        !entry
            .errors
            .iter()
            .any(|e| e.code() == "elab-unknown-identifier"),
        "no cascading unknown-identifier noise: {:?}",
        entry.errors.iter().map(|e| e.code()).collect::<Vec<_>>()
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_shared_prelude_module_suppresses_the_builtin_nat_for_the_closure() {
    let dir = tmp_dir("shared-nat");
    write(
        &dir,
        "Lesson/Nat.sokonanoda",
        "inductive Nat : Type\n\
ctor zero : Nat\n\
ctor succ (n : Nat) : Nat\n\
rec Nat.rec {u} :\n\
  (motive : (n : Nat) -> Sort u) ->\n\
  (mz : motive zero) ->\n\
  (ms : (n : Nat) -> motive n -> motive (succ n)) ->\n\
  (n : Nat) -> motive n\n\
iota zero :=\n\
  fun (motive : (n : Nat) -> Sort u) =>\n\
  fun (mz : motive zero) =>\n\
  fun (ms : (n : Nat) -> motive n -> motive (succ n)) => mz\n\
iota succ :=\n\
  fun (motive : (n : Nat) -> Sort u) =>\n\
  fun (mz : motive zero) =>\n\
  fun (ms : (n : Nat) -> motive n -> motive (succ n)) =>\n\
  fun (n : Nat) => ms n (Nat.rec.{u} motive mz ms n)\n\
end\n",
    );
    write(
        &dir,
        "Canvas.sokonanoda",
        "import Lesson.Nat\n\ndef two : Nat := succ (succ zero)\n",
    );
    let report = compile(&dir, "Canvas.sokonanoda");
    assert!(
        !report.has_errors(),
        "the shared Nat module must be usable: {:?}",
        codes(&report)
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_dependency_with_a_conflicting_prelude_directive_is_reported() {
    let dir = tmp_dir("prelude-conflict");
    write(
        &dir,
        "Bare.sokonanoda",
        "-- sokonanoda:prelude none\naxiom A : Prop\n",
    );
    write(&dir, "Main.sokonanoda", "import Bare\n\ndef x : Nat := 1\n");
    let report = compile(&dir, "Main.sokonanoda");
    assert!(
        codes(&report).contains(&"import-prelude-conflict"),
        "directive mismatch is a prelude conflict: {:?}",
        codes(&report)
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_duplicate_name_across_modules_is_a_collision_not_a_kernel_panic() {
    let dir = tmp_dir("collision");
    write(&dir, "A.sokonanoda", "def shared : Nat := 1\n");
    write(&dir, "B.sokonanoda", "def shared : Nat := 2\n");
    write(
        &dir,
        "Main.sokonanoda",
        "import A\nimport B\n\ndef use : Nat := shared\n",
    );
    let report = compile(&dir, "Main.sokonanoda");
    let diagnostic = report
        .diagnostics
        .iter()
        .find(|diag| diag.code() == "import-name-collision")
        .expect("import-name-collision");
    assert!(
        diagnostic.message.contains("A") && diagnostic.message.contains("B"),
        "both origins are named: {}",
        diagnostic.message
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn open_exercises_in_a_dependency_warn_on_the_import_line() {
    let dir = tmp_dir("open-exercise");
    write(
        &dir,
        "Lesson.sokonanoda",
        "def done : Nat := 1\ntheorem later : forall (a : Prop), a -> a := sorry\n",
    );
    write(
        &dir,
        "Main.sokonanoda",
        "import Lesson\n\ndef use : Nat := done\n",
    );
    let report = compile(&dir, "Main.sokonanoda");
    assert!(!report.has_errors(), "diagnostics: {:?}", codes(&report));
    let entry = report.entry_report().expect("entry report");
    assert!(
        entry
            .warnings
            .iter()
            .any(|warning| warning.code() == "import-has-open-exercises"),
        "the entry report warns about the dependency's open exercise: {:?}",
        entry.warnings.iter().map(|w| w.code()).collect::<Vec<_>>()
    );
    assert!(
        !entry
            .errors
            .iter()
            .any(|error| error.code() == "elab-unknown-identifier"),
        "an open exercise does not enter the environment, but `done` still resolves"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn bare_entry_makes_the_whole_closure_bare() {
    let dir = tmp_dir("bare-closure");
    write(&dir, "Logic.sokonanoda", "axiom A : Prop\n");
    write(
        &dir,
        "Main.sokonanoda",
        "-- sokonanoda:prelude none\nimport Logic\n\naxiom B : Prop\n",
    );
    let path = dir.join("Main.sokonanoda");
    let options = CompileOptions {
        prelude: PreludeMode::Bare,
    };
    let report = compile_project(&path, None, &options, None);
    assert!(!report.has_errors(), "diagnostics: {:?}", codes(&report));
    // `Nat` 在 Bare 闭包里不存在（依赖也没有偷偷装 prelude）。
    write(
        &dir,
        "Main.sokonanoda",
        "-- sokonanoda:prelude none\nimport Logic\n\ndef n : Nat := 1\n",
    );
    let report = compile_project(&path, None, &options, None);
    let entry = report.entry_report().expect("entry report");
    assert!(
        entry
            .errors
            .iter()
            .any(|error| error.code() == "elab-unknown-identifier"),
        "Bare closure has no Nat: {:?}",
        entry.errors.iter().map(|e| e.code()).collect::<Vec<_>>()
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn open_exercises_do_not_leak_into_the_importer() {
    let dir = tmp_dir("open-not-visible");
    write(
        &dir,
        "Lesson.sokonanoda",
        "theorem later : forall (a : Prop), a -> a := sorry\n",
    );
    write(
        &dir,
        "Main.sokonanoda",
        "import Lesson\n\ndef use : Nat := later\n",
    );
    let report = compile(&dir, "Main.sokonanoda");
    let entry = report.entry_report().expect("entry report");
    assert!(
        entry
            .errors
            .iter()
            .any(|error| error.code() == "elab-unknown-identifier"),
        "the unfinished declaration is invisible downstream: {:?}",
        entry.errors.iter().map(|e| e.code()).collect::<Vec<_>>()
    );
    assert!(
        report
            .module("Lesson")
            .is_some_and(|module| module.open_exercises() == 1),
        "the dependency itself still reports its open exercise"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn entry_only_files_still_compile_through_the_project_path() {
    let dir = tmp_dir("single");
    write(&dir, "Main.sokonanoda", "def one : Nat := 1\n");
    let report = compile(&dir, "Main.sokonanoda");
    assert!(!report.has_errors());
    assert_eq!(report.modules.len(), 1);
    let entry = report.entry_report().expect("entry report");
    assert_eq!(entry.decls.len(), 1);
    assert_eq!(entry.decls[0].status, DeclStatus::Checked);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_broken_manifest_is_a_project_error_not_a_panic() {
    let dir = tmp_dir("bad-manifest");
    write(&dir, "sokonanoda.toml", "src = \"../escape\"\n");
    write(&dir, "Main.sokonanoda", "def one : Nat := 1\n");
    let report = compile(&dir, "Main.sokonanoda");
    assert!(
        codes(&report).contains(&"manifest-invalid"),
        "bad manifest is reported: {:?}",
        codes(&report)
    );
    assert!(
        report.entry_report().is_some(),
        "the entry still compiles in the fallback root"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

// ── P4：闭包摘要（缓存键）─────────────────────────────────────────────────

#[test]
fn closure_digest_is_stable_and_dependency_sensitive() {
    let dir = tmp_dir("digest");
    write(&dir, "Bar.sokonanoda", "def bar : Nat := 2\n");
    write(
        &dir,
        "Main.sokonanoda",
        "import Bar\n\ndef two : Nat := bar\n",
    );
    let path = dir.join("Main.sokonanoda");
    let options = CompileOptions::default();

    let digest = |options: &CompileOptions| plan_project(&path, None, None).digest(options);
    let first = digest(&options);
    assert_eq!(first, digest(&options), "same inputs ⇒ same digest");

    // 依赖变了 ⇒ 入口的键必须变（否则会拿旧报告当"检查过"）。
    write(&dir, "Bar.sokonanoda", "def bar : Nat := 3\n");
    assert_ne!(first, digest(&options), "a dependency edit invalidates it");

    // 入口自己变了 ⇒ 也变。
    write(&dir, "Bar.sokonanoda", "def bar : Nat := 2\n");
    write(
        &dir,
        "Main.sokonanoda",
        "import Bar\n\ndef two : Nat := bar + 0\n",
    );
    assert_ne!(first, digest(&options), "an entry edit invalidates it");

    // prelude 模式变了 ⇒ 也变（Bare 下 `Nat` 根本不存在）。
    let bare = CompileOptions {
        prelude: PreludeMode::Bare,
    };
    assert_ne!(
        digest(&options),
        digest(&bare),
        "the prelude shape is part of the key"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn module_keys_are_dependency_scoped_not_entry_scoped() {
    // **per-module 产物键的性质判据**（设计 `docs/design/module-artifacts.md` §3）：
    // 这是"结果复用"能成立、且**不会错编**的那两条性质。
    //
    // 为什么必须有它：per-entry 的 `digest` 把整条闭包折成一条键 ⇒ **改任何一个依赖，
    // 42 个入口全部失效**（G-68 的 4.14× 重复功）；而 per-module 键要让
    // **没变的模块键逐字节不变**（⇒ 产物可复用），同时**变了的下游必须全变**（⇒ 不错编）。
    let dir = tmp_dir("module-keys");
    write(&dir, "Bar.sokonanoda", "def bar : Nat := 2\n");
    write(&dir, "Baz.sokonanoda", "def baz : Nat := 4\n");
    write(
        &dir,
        "Main.sokonanoda",
        "import Bar\nimport Baz\n\ndef two : Nat := bar\n",
    );
    let path = dir.join("Main.sokonanoda");
    let options = CompileOptions::default();
    let keys = || plan_project(&path, None, None).module_keys(&options);
    let key_of = |list: &[(String, String)], name: &str| {
        list.iter()
            .find(|(module, _)| module == name)
            .unwrap_or_else(|| panic!("{name} 必须在闭包里：{list:?}"))
            .1
            .clone()
    };

    let first = keys();
    // ① 确定性：同一份输入两次算出同一批键。
    assert_eq!(first, keys(), "模块键必须确定性");
    // ② 拓扑序：依赖在前、入口在最后（`ProjectPlan::digest` 同一条纪律）。
    assert_eq!(first.last().map(|(name, _)| name.as_str()), Some("Main"));
    // ③ 每个模块一条键，互不相同（不是把整条闭包又折成一条）。
    let unique: std::collections::HashSet<&String> = first.iter().map(|(_, key)| key).collect();
    assert_eq!(
        unique.len(),
        first.len(),
        "每个模块必须有自己的一条键：{first:?}"
    );

    let bar_before = key_of(&first, "Bar");
    let main_before = key_of(&first, "Main");

    // ④ **无关模块变 ⇒ 别人的键逐字节不变**（这就是 per-module 相对 per-entry 的收益：
    //    `Baz` 与 `Bar` 互不依赖 ⇒ 改 `Baz` 不该让 `Bar` 的产物失效）。
    write(&dir, "Baz.sokonanoda", "def baz : Nat := 9\n");
    let after_unrelated = keys();
    assert_eq!(
        key_of(&after_unrelated, "Bar"),
        bar_before,
        "无关模块（Baz）改了，Bar 的键必须逐字节不变（否则共享依赖的产物永远复用不上）"
    );
    assert_ne!(
        key_of(&after_unrelated, "Main"),
        main_before,
        "入口直接 import 了 Baz ⇒ 它的键必须变"
    );

    // ⑤ **依赖变 ⇒ 它自己与所有下游都变**（红线：错编比慢严重得多）。
    let main_after_unrelated = key_of(&after_unrelated, "Main");
    write(&dir, "Bar.sokonanoda", "def bar : Nat := 3\n");
    let after_dep = keys();
    assert_ne!(
        key_of(&after_dep, "Bar"),
        bar_before,
        "依赖自己变了，键必须变"
    );
    assert_ne!(
        key_of(&after_dep, "Main"),
        main_after_unrelated,
        "**依赖变了，入口的键必须变** —— 不变就是拿旧产物回放，会错编"
    );
}

/// **阶段 1a 的目标判据**（`docs/design/incremental-environment.md` §9.1 的 1a 行：
/// "本阶段只补『同一模块跨入口同一条 key』的守卫，已落 `f7069589`"）：
/// **同一个模块，在不同入口的闭包里必须算出同一条 `module_key`** ——
/// 这是"按 key 复用产物"能成立的**充要前提**：
/// 若同一个模块在两个入口下 key 不同，产物就永远复用不上（G-68 的 4.14× 回来了 ✗）；
/// 若 key 相同却内容不同，就是**错编**（比慢严重得多 ✗）。
///
/// ⚠ **本用例现在就该绿**（`module_keys` 是 per-module 的，不是 per-entry 的）；
/// 它守的是"**接线时别把它退回 per-entry**"——接线（把 key 用进 `build`）在阶段 1b，
/// 而那条路一旦写成"按入口算 key"，本用例立刻判红 ✓。
#[test]
fn the_same_module_has_one_key_across_different_entries() {
    let dir = tmp_dir("module-key-shared");
    write(&dir, "Shared.sokonanoda", "def shared : Nat := 1\n");
    write(
        &dir,
        "E0.sokonanoda",
        "import Shared\ndef e0 : Nat := shared\n",
    );
    write(
        &dir,
        "E1.sokonanoda",
        "import Shared\ndef e1 : Nat := shared\n",
    );
    let options = CompileOptions::default();
    let keys_for = |entry: &str| {
        plan_project(&dir.join(entry), None, Some(dir.as_path())).module_keys(&options)
    };
    let key_of = |list: &[(String, String)], name: &str| {
        list.iter()
            .find(|(module, _)| module == name)
            .unwrap_or_else(|| panic!("{name} 必须在闭包里：{list:?}"))
            .1
            .clone()
    };
    let a = keys_for("E0.sokonanoda");
    let b = keys_for("E1.sokonanoda");
    assert_eq!(
        key_of(&a, "Shared"),
        key_of(&b, "Shared"),
        "**同一个模块在两个入口下必须是同一条 key** —— 不同就永远复用不上产物（G-68 回来）✗"
    );
    assert_ne!(
        key_of(&a, "E0"),
        key_of(&b, "E1"),
        "两个入口自己是不同模块 ⇒ key 必须不同"
    );
}

#[test]
fn closure_digest_marks_the_module_set() {
    let dir = tmp_dir("digest-set");
    write(&dir, "Bar.sokonanoda", "def bar : Nat := 2\n");
    write(
        &dir,
        "Main.sokonanoda",
        "import Bar\n\ndef two : Nat := bar\n",
    );
    let path = dir.join("Main.sokonanoda");
    let options = CompileOptions::default();
    let one_dep = plan_project(&path, None, None).digest(&options);

    write(&dir, "Baz.sokonanoda", "def baz : Nat := 4\n");
    write(
        &dir,
        "Main.sokonanoda",
        "import Bar\nimport Baz\n\ndef two : Nat := bar + baz\n",
    );
    assert_ne!(
        one_dep,
        plan_project(&path, None, None).digest(&options),
        "adding an import changes the closure"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn closure_digest_is_scoped_to_the_entry_location() {
    // **T-B4 的判据**（`docs/design/project-artifacts.md` §1.1/§2 的承重主张）：
    // 项目条目的键里**含入口的绝对路径**（T-A06）⇒ 两份**逐字相同**、只是放在不同
    // 目录的项目，digest 必须**不同**。
    //
    // 为什么这条是承重的：`.sokonanoda/` 的分工决定（项目条目落模块根、单文件条目留
    // 全局缓存）建立在"项目条目**天生按位置隔离**"之上。若它其实与位置无关，把产物
    // 放进模块根就会**丢掉跨项目复用** ✗ —— 那分工得推翻重来。反过来，单文件条目的
    // 键只含内容（`compile::cache::key(src, options)` 连路径参数都没有）⇒ 它留在
    // 全局缓存才对。
    //
    // 反例守卫：把 `ProjectPlan::digest` 里 `digest_path(&self.entry)` 那一行去掉，
    // 本测试立刻红 ✗（T-A06 的原始 bug 就是把它漏了）。
    let a = tmp_dir("digest-loc-a");
    let b = tmp_dir("digest-loc-b");
    for dir in [&a, &b] {
        write(dir, "sokonanoda.toml", "name = \"x\"\n");
        write(dir, "Bar.sokonanoda", "def bar : Nat := 2\n");
        write(
            dir,
            "Main.sokonanoda",
            "import Bar\n\ndef two : Nat := bar\n",
        );
    }
    let options = CompileOptions::default();
    let digest_in =
        |dir: &Path| plan_project(&dir.join("Main.sokonanoda"), None, None).digest(&options);

    assert_eq!(digest_in(&a), digest_in(&a), "same location ⇒ stable");
    assert_ne!(
        digest_in(&a),
        digest_in(&b),
        "identical content at a different path must not share a digest \
         (the project entry is location-scoped by construction)"
    );
    let _ = std::fs::remove_dir_all(&a);
    let _ = std::fs::remove_dir_all(&b);
}

#[test]
fn an_overlay_makes_unsaved_dependency_edits_visible() {
    let dir = tmp_dir("overlay");
    write(
        &dir,
        "Logic.sokonanoda",
        "axiom And : Prop -> Prop -> Prop\n\
axiom And.intro : forall (a b : Prop), a -> b -> And a b\n",
    );
    let canvas = write(
        &dir,
        "Canvas.sokonanoda",
        "import Logic\n\n\
theorem t (a b : Prop) (h : a) (k : b) : And a b := And.intro a b h k\n",
    );
    let options = CompileOptions::default();
    let clean = compile_project(&canvas, None, &options, None);
    assert!(clean.diagnostics.is_empty(), "{:?}", codes(&clean));

    // 依赖的**未落盘**改名（编辑器里的中间态）通过覆盖进入闭包：
    // 入口里的 `And.intro` 立刻变成未知标识符。
    let overlay = vec![(
        dir.join("Logic.sokonanoda"),
        "axiom And : Prop -> Prop -> Prop\n\
axiom And.mk : forall (a b : Prop), a -> b -> And a b\n"
            .to_string(),
    )];
    let broken = compile_project_with_overlay(&canvas, None, &options, None, &overlay);
    // 入口的 elab 错误挂在**入口模块的报告**里（`ProjectReport::diagnostics`
    // 只装闭包级规则：找不到/环/重名/依赖阻断）。
    let entry_errors: Vec<&str> = broken
        .entry_module()
        .map(|module| module.report.errors.iter().map(|e| e.code()).collect())
        .unwrap_or_default();
    assert!(
        entry_errors.contains(&"elab-unknown-identifier"),
        "{entry_errors:?}"
    );
    // 覆盖里的模块文本本身也进报告（跨文件引用/改名按它算 span）。
    let logic = broken
        .modules
        .iter()
        .find(|module| module.name == "Logic")
        .expect("Logic in the closure");
    assert!(logic.source.contains("And.mk"), "{}", logic.source);
    assert!(!logic.source.contains("And.intro"), "{}", logic.source);
    // 覆盖也进闭包摘要：编辑器里的中间态不会命中磁盘上那份旧缓存。
    let plan = plan_project(&canvas, None, None);
    let overlaid = plan_project_with_overlay(&canvas, None, None, &overlay);
    assert_ne!(plan.digest(&options), overlaid.digest(&options));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn match_on_an_imported_inductive_infers_its_universe() {
    // 回归：入口里对**被导入模块**声明的归纳类型做 `match`。宇宙层级查询走
    // `judge_infer(prefix_src, …)` 合成一个前缀文件再问内核；前缀只拼本文件时
    // 内核看不见被导入的 `Or`，于是报 `elab-match-no-expected-type`
    // （2026-09-18 实测：`import Logic` + `match h with | inl … | inr …`）。
    // 修法：闭包模式下前缀 = 前面各单元的声明文本（去掉 `import` 行）+ 本文件前缀。
    let dir = tmp_dir("match-imported");
    write(
        &dir,
        "Logic.sokonanoda",
        "inductive Or (A B : Prop) : Prop\nctor inl (a : A) : Or A B\nctor inr (b : B) : Or A B\nend\n",
    );
    let entry = write(
        &dir,
        "Main.sokonanoda",
        "import Logic\n\n\
theorem or_comm (A : Prop) (B : Prop) (h : Or A B) : Or B A :=\n\
  match h with\n\
  | inl a => @inr B A a\n\
  | inr b => @inl B A b\n",
    );
    let report = compile_project(&entry, None, &CompileOptions::default(), None);
    assert!(report.diagnostics.is_empty(), "{:?}", codes(&report));
    let entry_errors: Vec<&str> = report
        .entry_module()
        .map(|module| {
            module
                .report
                .errors
                .iter()
                .map(|error| error.code())
                .collect()
        })
        .unwrap_or_default();
    assert!(entry_errors.is_empty(), "{entry_errors:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn by_tactics_read_imported_types_through_the_judge_prefix() {
    // 同一条前缀也喂给 `apply`（它要读被应用函数的类型）与 binder 推断。
    // 依赖里声明、入口里 `apply`：闭包前缀必须让 judge 看见 `And.intro`。
    let dir = tmp_dir("judge-imported");
    write(
        &dir,
        "Logic.sokonanoda",
        "axiom And : Prop -> Prop -> Prop\n\
axiom And.intro : (a : Prop) -> (b : Prop) -> a -> b -> And a b\n",
    );
    let entry = write(
        &dir,
        "Main.sokonanoda",
        "import Logic\n\n\
theorem and_intro_demo (a b : Prop) (h : a) (k : b) : And a b := by\n\
  apply And.intro\n\
  exact h\n\
  exact k\n",
    );
    let report = compile_project(&entry, None, &CompileOptions::default(), None);
    let entry_errors: Vec<&str> = report
        .entry_module()
        .map(|module| {
            module
                .report
                .errors
                .iter()
                .map(|error| error.code())
                .collect()
        })
        .unwrap_or_default();
    assert!(entry_errors.is_empty(), "{entry_errors:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// G-02 的第二道闸（WO-005 验收「项目级」第 6 条）：A/B 两个模块各声明
/// `inductive A/B` + `ctor mk`，入口 import 两者 ⇒ **不得**出现
/// `import-name-collision`（规范名 `A.mk`/`B.mk` 在闭包里天然不同）。
#[test]
fn ctors_in_two_modules_do_not_collide_after_namespacing() {
    let dir = tmp_dir("ctor-namespace");
    write(
        &dir,
        "A.sokonanoda",
        "inductive A : Type\nctor mk : A\nend\n",
    );
    write(
        &dir,
        "B.sokonanoda",
        "inductive B : Type\nctor mk : B\nend\n",
    );
    write(
        &dir,
        "Main.sokonanoda",
        "import A\nimport B\n\ndef useA : A := A.mk\ndef useB : B := B.mk\n",
    );
    let report = compile(&dir, "Main.sokonanoda");
    assert!(
        !codes(&report).contains(&"import-name-collision"),
        "namespaced ctors must not collide: {:?}",
        codes(&report)
    );
    assert!(
        !report.has_errors(),
        "the closure must compile: {:?}",
        codes(&report)
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// 对照（R2 的闭包级口径）：两个模块各有一个**同名裸** ctor ⇒ 裸名在闭包里
/// 歧义、不可解析；写全规范名照常。这正是 `elab-ambiguous-ctor-alias` 的
/// 闭包级场景（模块作用域属 G-05，本轮别名天然是闭包级的）。
#[test]
fn a_duplicated_bare_ctor_across_modules_is_ambiguous() {
    let dir = tmp_dir("ctor-alias-ambiguous");
    write(
        &dir,
        "A.sokonanoda",
        "inductive A : Type\nctor mk : A\nend\n",
    );
    write(
        &dir,
        "B.sokonanoda",
        "inductive B : Type\nctor mk : B\nend\n",
    );
    write(
        &dir,
        "Main.sokonanoda",
        "import A\nimport B\n\ndef bad : A := mk\n",
    );
    let report = compile(&dir, "Main.sokonanoda");
    // elab 错误挂在**入口模块的报告**里（`diagnostics` 只装闭包级规则）。
    let entry_errors: Vec<&str> = report
        .entry_module()
        .map(|module| module.report.errors.iter().map(|e| e.code()).collect())
        .unwrap_or_default();
    assert!(
        entry_errors.contains(&"elab-ambiguous-ctor-alias"),
        "the bare name is ambiguous in the closure: {entry_errors:?}"
    );
    assert!(
        !codes(&report).contains(&"import-name-collision"),
        "the canonical names differ, so the modules do not collide: {:?}",
        codes(&report)
    );
    let _ = std::fs::remove_dir_all(&dir);
}

// ---- 跨 `import` 的记法传播（G-04 第二刀，设计 `docs/design/notation-subset.md` §10.3）

/// 卷 I 形状的最小库：声明两条记法（`prefix` 与 `postfix`）。
const NOTATION_LIB: &str = "\
def Set (α : Type) : Type := α -> Prop\n\
axiom Set.union : (α : Type) -> Set α -> Set α -> Set α\n\
axiom Set.compl : (α : Type) -> Set α -> Set α\n\
infixl:65 \" ∪ \" => Set.union\n\
postfix:100 \" ᶜ \" => Set.compl\n";

#[test]
fn a_dependency_notation_is_usable_in_the_entry() {
    let dir = tmp_dir("notation-import");
    write(&dir, "lib/Set.sokonanoda", NOTATION_LIB);
    // 入口**不重声明**记法：它直接用库里的 `ᶜ` / `∪`。
    write(
        &dir,
        "Main.sokonanoda",
        "import lib.Set\n\ndef use1 (α : Type) (A B : Set α) : Set α := Aᶜ ∪ B\n\
         def use2 (α : Type) (A B : Set α) : Set α := Set.union α (Set.compl α A) B\n",
    );
    let report = compile(&dir, "Main.sokonanoda");
    assert!(
        report.diagnostics.is_empty(),
        "the closure must load cleanly: {:?}",
        report.diagnostics
    );
    let entry = report.entry_module().expect("entry module");
    assert_eq!(
        entry.report.errors,
        vec![],
        "the entry must elaborate the library notation"
    );
    assert_eq!(
        entry
            .report
            .decls
            .iter()
            .filter(|decl| decl.status == DeclStatus::Checked)
            .count(),
        2,
        "both spellings must check: {:?}",
        entry.report.decls
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_notation_from_a_module_that_was_not_imported_is_not_visible() {
    // 传播是**按 import 边**的，不是"闭包里全局"：没 import 的模块声明的记法
    // 不泄漏（`lib/Other` 甚至不在闭包里）。
    let dir = tmp_dir("notation-no-leak");
    write(&dir, "lib/Set.sokonanoda", NOTATION_LIB);
    write(
        &dir,
        "lib/Other.sokonanoda",
        "prefix:100 \" 𝒫 \" => Other.powerset\naxiom Other.powerset : (α : Type) -> α -> α\n",
    );
    write(
        &dir,
        "Main.sokonanoda",
        "import lib.Set\n\ndef use (α : Type) (A : Set α) : Set α := Aᶜ\n\
         def leak (α : Type) (A : α) : α := 𝒫 A\n",
    );
    let report = compile(&dir, "Main.sokonanoda");
    assert_eq!(
        report
            .modules
            .iter()
            .map(|m| m.name.as_str())
            .collect::<Vec<_>>(),
        vec!["lib.Set", "Main"],
        "an unimported module must not even be loaded"
    );
    let entry_errors: Vec<&str> = report
        .entry_module()
        .map(|module| module.report.errors.iter().map(|e| e.code()).collect())
        .unwrap_or_default();
    assert!(
        entry_errors.contains(&"elab-unknown-identifier"),
        "`𝒫` is not visible without the import: {entry_errors:?}"
    );
    let entry = report.entry_module().expect("entry module");
    assert!(
        entry
            .report
            .decls
            .iter()
            .any(|decl| decl.name.as_deref() == Some("use") && decl.status == DeclStatus::Checked),
        "`ᶜ` comes through the import and must still work: {:?}",
        entry.report.decls
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn redeclaring_an_inherited_notation_is_a_dedicated_error() {
    // 第二刀：同一符号只能声明一次——**包括** import 带进来的那些。判卷通道
    // 把闭包首尾相接成一份合成源码，两个模块各声明一次 `∪` 在那里必然撞车，
    // 所以源头就不许（设计 §10.3）。
    let dir = tmp_dir("notation-redeclare");
    write(&dir, "lib/Set.sokonanoda", NOTATION_LIB);
    write(
        &dir,
        "Main.sokonanoda",
        "import lib.Set\n\
         axiom Other.union : (α : Type) -> Set α -> Set α -> Set α\n\
         infixl:65 \" ∪ \" => Other.union\n",
    );
    let report = compile(&dir, "Main.sokonanoda");
    let codes: Vec<&str> = report.diagnostics.iter().map(|d| d.code()).collect();
    // **归因必须是入口自己**（长尾 ②/r06，2026-10-12）：入口重声明继承来的记法
    // 是**本文件**的解析失败，闭包加载器一律包成 `import-module-invalid` 的话，
    // 学习者会被指去翻 `lib/Set.sokonanoda`（那是好的）✗。`lib.Set` 自己仍然
    // 用 `entry-parse-failed` 以外的码表，见 `graph.rs` 的入口/依赖分支。
    assert!(
        codes.contains(&"entry-parse-failed"),
        "the entry's own parse failure must be attributed to the entry: {codes:?}"
    );
    assert!(
        !codes.contains(&"import-module-invalid"),
        "the entry is not an imported module: {codes:?}"
    );
    let message = report
        .diagnostics
        .iter()
        .find(|d| d.code() == "entry-parse-failed")
        .map(|d| d.message.clone())
        .unwrap_or_default();
    assert!(
        message.contains("已经声明过记法"),
        "the message must name the rule: {message}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// **长尾 ②/r06（2026-10-12）**：入口**自己**的语法错必须归因到入口。
///
/// 受控对照（唯一变量 = 入口有没有用依赖声明的记法 `ᶜ`）：
/// * 不用记法 ⇒ 单独 parse 就失败，报 `unexpected-token`（诚实的旧行为）；
/// * 用了记法 ⇒ 单独 parse 也是失败，但**闭包先收 import 边、拿到记法表**之后
///   才解析自己 ⇒ 真实第一因是**入口里那个 `·`**。以前闭包把这条也包成
///   `import-module-invalid`（"**被导入的文件**没解析成功"）+ 甩锅 hint，
///   学习者会去翻 `lib/Set.sokonanoda`（它完全没问题）✗。
///
/// 判据是**真相层**的"哪个模块 parse 失败"（入口 vs 依赖），不是文件名启发式。
#[test]
fn an_entry_parse_error_is_attributed_to_the_entry_not_to_its_imports() {
    let dir = tmp_dir("entry-parse-attribution");
    // 依赖是干净的（`∪` / `ᶜ` 都声明在这里）。
    write(&dir, "lib/Set.sokonanoda", NOTATION_LIB);
    write(
        &dir,
        "Main.sokonanoda",
        "import lib.Set\n\n\
         axiom Other.bullet : (α : Type) -> Set α -> Set α\n\
         infixl:65 \" · \" => Other.bullet\n\
         \u{00b7}\n",
    );
    let report = compile(&dir, "Main.sokonanoda");
    let entry_codes = codes(&report);
    assert!(
        entry_codes.contains(&"entry-parse-failed"),
        "入口自己的 parse 失败要归因到入口：{entry_codes:?}"
    );
    assert!(
        !entry_codes.contains(&"import-module-invalid"),
        "依赖是干净的，不许甩锅给它：{entry_codes:?}"
    );
    // 位置 = 入口里真正出错的那一行（第 5 行那个裸 `·`），不是 import 行。
    let diag = report
        .diagnostics
        .iter()
        .find(|diag| diag.code() == "entry-parse-failed")
        .expect("entry-parse-failed");
    let span = diag.span.expect("the parse diagnostic carries a span");
    assert_eq!(span.start.line, 5, "真错在第 5 行：{diag:?}");
    assert_eq!(diag.module, "Main", "归属模块是入口：{diag:?}");
    // hint 不许甩锅到别的文件（那条话属于 `import-module-invalid`）。
    let hint = diag.hint();
    assert!(hint.contains("本文件"), "hint 必须把读者留在本文件：{hint}");
    assert!(
        !hint.contains("打开它"),
        "hint 不许让读者去打开依赖：{hint}"
    );
    // **r16（2026-10-13）**：入口解析失败时，消息要**点名"哪些声明已经解析成功、
    // 为什么没有判定"** —— 计数仍然诚实地全 0（一条都没验过 ✓），但学习者不再
    // 两眼一抹黑。⚠ 坏掉的那一条**不许**列进去（恢复式解析会把它的前半截救回来 ✗）。
    assert!(
        diag.message.contains("没有判定"),
        "r16：解析失败要明说「没有判定」：{diag:?}"
    );

    // **对照**：依赖自己坏掉时仍走 `import-module-invalid`（那才是它的语义），
    // 且消息带**依赖的**模块名 —— 归因改动不许把这一半也改掉。
    let dir = tmp_dir("entry-parse-attribution-dep");
    write(&dir, "Bad.sokonanoda", "def broken : Nat := )\n");
    write(
        &dir,
        "Main.sokonanoda",
        "import Bad\n\ndef use : Nat := 1\n",
    );
    let report = compile(&dir, "Main.sokonanoda");
    let dep_codes = codes(&report);
    assert!(
        dep_codes.contains(&"import-module-invalid"),
        "依赖的解析失败仍是 import-module-invalid：{dep_codes:?}"
    );
    assert!(
        !dep_codes.contains(&"entry-parse-failed"),
        "入口自己没写错，不许报入口：{dep_codes:?}"
    );
    let dep = report
        .diagnostics
        .iter()
        .find(|diag| diag.code() == "import-module-invalid")
        .expect("import-module-invalid");
    assert_eq!(dep.module, "Bad", "{dep:?}");
    assert!(
        dep.message.contains("Bad"),
        "依赖的诊断要带它的模块名：{dep:?}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// ⭐ **`SUB-parse-error-kills-whole-file`（用户 2026-10-15 亲报）· 错误隔离**：
/// 入口里**一处**真语法错**不许**把整份文件废掉 —— 能解析的声明照常判定，
/// 出错的那一条仍如实报 `entry-parse-failed`（带真位置）。
///
/// **改前（实测读数）**：`counts` 全 0、`failed` 只有一条 `entry-parse-failed`
/// ⇒ Infoview 的「目标」「声明」全没 ✗（用户原话：「整个文件都报错了」）。
/// **改后**：坏块**之前**与**之后**的声明都照常 `decl_checked` ✓。
///
/// **Lean 4 对照**：`Lean/Parser/Module.lean:137` 的 `parseCommand` 逐条命令解析 +
/// 恢复（失败时 `recovering := true`、"advance at least one token to prevent
/// infinite loops"，然后**继续**下一条，`:162-175`）⇒ Lean 里一处语法错不会让别的
/// 声明从 infoview 消失。我们的 `Parser::parse_file_recovering` 早就是同一条语义，
/// 只是闭包这条路没用它 ✗。
///
/// 撤掉 `graph.rs` 里那个 `is_entry` 恢复分支（回到 `FolFile { commands: vec![] }`）
/// ⇒ 本条判红 ✓（反向验证）。
#[test]
fn an_entry_syntax_error_does_not_scrap_the_rest_of_the_file() {
    let dir = tmp_dir("entry-error-isolation");
    // 库层声明 `∈`（与 `crates/cli/tests/query.rs` 的两通道夹具同款）。
    write(
        &dir,
        "lib/Set.sokonanoda",
        "def Set (α : Type) : Type := α -> Prop\n\
         def Set.mem (α : Type) (a : α) (A : Set α) : Prop := A a\n\
         infix:50 \" ∈ \" => Set.mem\n",
    );
    // 记法（`∈`）出现在**错误之前**：`is_project_source` 才会把入口判成项目源
    // （裸 parse 先死在 `∈` 上 ⇒ `notation-unknown-symbol`）—— 这正是用户
    // 那个课程单元的形状（签名里就有 `∈` / `⊆`）。
    write(
        &dir,
        "Main.sokonanoda",
        "import lib.Set\n\n\
         theorem before (α : Type) (a : α) (A : Set α) (h : a ∈ A) : a ∈ A := by\n\
         \x20 exact h\n\n\
         def broken : Nat := )\n\n\
         def after : Nat := 1\n",
    );
    let report = compile(&dir, "Main.sokonanoda");

    // ① 错**没被吞**：仍是 `entry-parse-failed`、仍带真位置（第 6 行那个 `)`）。
    let diag = report
        .diagnostics
        .iter()
        .find(|diag| diag.code() == "entry-parse-failed")
        .expect("入口自己的语法错仍要如实归因");
    assert_eq!(
        diag.span.expect("带真位置").start.line,
        6,
        "真错在第 6 行：{diag:?}"
    );
    // ② 坏块**之前**与**之后**的声明都照常判定（改前这里是空报告 ✗）。
    let entry = report
        .modules
        .iter()
        .find(|module| module.name == "Main")
        .expect("entry module");
    let checked: Vec<&str> = entry
        .report
        .decls
        .iter()
        .filter(|decl| decl.status == DeclStatus::Checked)
        .filter_map(|decl| decl.name.as_deref())
        .collect();
    assert!(
        checked.contains(&"before"),
        "坏块**之前**的声明要照常判定：{checked:?}"
    );
    assert!(
        checked.contains(&"after"),
        "坏块**之后**的声明也要照常判定：{checked:?}"
    );
    assert!(
        !checked.contains(&"broken"),
        "出错的那一条不许被判定：{checked:?}"
    );
    // ③ 出口码不变：入口的语法错仍然是**拒绝**。
    assert!(report.has_errors(), "语法错必须仍然判红");
    let _ = std::fs::remove_dir_all(&dir);
}

/// **S2 步 2 的判据**（设计 `docs/design/declaration-incremental.md` §4）：入口那一趟
/// 带上**信任前缀**（`EntryTrust`）之后 ——
///
/// 1. **前缀不再重查**（内核检查数下降，结构计数、噪声免疫）；
/// 2. **后缀的判定逐字节不变**（被信任那段不进报告，后缀必须与整份重查时一模一样）。
///
/// 为什么必须两条一起断言：只断言"变少了"会把**丢声明**（报告缺前缀）当成成功 ✗。
/// 这条钉的正是 §4.2 那个"算错不是崩、是静默错编"的风险面。
#[test]
fn entry_trust_skips_the_prefix_and_keeps_the_suffix_identical() {
    use crate::compile::TrustPlan;
    use crate::project::session::{with_project_session, with_project_session_trusted, EntryTrust};
    use std::collections::HashMap;

    let dir = tmp_dir("entry-trust");
    write(
        &dir,
        "lib/Lib.sokonanoda",
        "def Set (α : Type) : Type := α -> Prop\n\n\
         def Set.mem (α : Type) (a : α) (A : Set α) : Prop := A a\n\n\
         infix:50 \" ∈ \" => Set.mem\n",
    );
    // 入口：6 条声明，前缀 3 条 / 后缀 3 条。都用库层记法 ⇒ 前缀真被 elaborate 过。
    let mut entry = String::from("import lib.Lib\n\n");
    for i in 0..6 {
        entry.push_str(&format!(
            "theorem t{i} (α : Type) (a : α) (A : Set α) (h : a ∈ A) : a ∈ A := h\n"
        ));
    }
    write(&dir, "units/entry.sokonanoda", &entry);

    let options = CompileOptions::default();
    let plan = plan_project(
        &dir.join("units/entry.sokonanoda"),
        None,
        Some(dir.as_path()),
    );
    let modules = plan.modules();
    let last = modules.len() - 1;
    // 闭包表达不了这个生命周期（返回借用自参数）⇒ 老实展开。
    let mut lib_units: Vec<SourceUnit<'_>> = Vec::new();
    for module in &modules[..last] {
        lib_units.push(SourceUnit {
            name: &module.name,
            path: Some(module.path.as_path()),
            file: &module.file,
        });
    }
    assert!(
        !lib_units.is_empty(),
        "夹具必须有库层（否则这条量不到东西）"
    );
    let entry_module = &modules[last];
    let entry_units: Vec<SourceUnit<'_>> = vec![SourceUnit {
        name: &entry_module.name,
        path: Some(entry_module.path.as_path()),
        file: &entry_module.file,
    }];

    // ① 整份重查（今天的行为）。
    let mut untrusted_checks = 0usize;
    let mut untrusted_entry: Vec<crate::compile::DocumentReport> = Vec::new();
    with_project_session(
        &lib_units,
        std::slice::from_ref(&entry_units),
        &options,
        |_, out, entry_reports, _, _, _| {
            untrusted_checks = out.stats.kernel_checks;
            untrusted_entry = entry_reports;
        },
    );

    // ② 带信任前缀：前 4 条命令"文本没变过、上一轮查过"。
    // ⚠ 命令 0 是 `import lib.Lib`（它也是一条命令）⇒ `before: 4` = import + t0/t1/t2。
    let trust = vec![Some(EntryTrust {
        plan: TrustPlan {
            before: 4,
            trusted_extra: Vec::new(),
            prev_signatures: Vec::new(),
            text_unchanged: Vec::new(),
            allow_cutoff: false,
            trusted_entered: None,
        },
        failures: HashMap::new(),
    })];
    let mut trusted_checks = 0usize;
    let mut trusted_entry: Vec<crate::compile::DocumentReport> = Vec::new();
    with_project_session_trusted(
        &lib_units,
        std::slice::from_ref(&entry_units),
        &options,
        &trust,
        |_, out, entry_reports, _, _, _| {
            trusted_checks = out.stats.kernel_checks;
            trusted_entry = entry_reports;
        },
    );

    let names = |reports: &[crate::compile::DocumentReport]| -> Vec<String> {
        reports
            .iter()
            .flat_map(|r| r.decls.iter())
            .filter_map(|d| d.name.clone())
            .collect()
    };
    let untrusted_names = names(&untrusted_entry);
    let trusted_names = names(&trusted_entry);
    assert_eq!(
        untrusted_names,
        (0..6).map(|i| format!("t{i}")).collect::<Vec<_>>(),
        "整份重查必须给出全部 6 条声明（夹具前提）"
    );
    assert_eq!(
        trusted_names,
        vec!["t3".to_string(), "t4".to_string(), "t5".to_string()],
        "带信任前缀时**只有后缀**是新查的 —— 前缀由调用方的会话缓存补（设计 §4.2）"
    );

    // **判据 ②（内容级）**：后缀那三条的判定必须与整份重查时**逐字节相同**。
    let by_name = |reports: &[crate::compile::DocumentReport], want: &str| -> String {
        reports
            .iter()
            .flat_map(|r| r.decls.iter())
            .find(|d| d.name.as_deref() == Some(want))
            .map(|d| format!("{d:?}"))
            .unwrap_or_else(|| panic!("缺少声明 {want}"))
    };
    for want in ["t3", "t4", "t5"] {
        assert_eq!(
            by_name(&trusted_entry, want),
            by_name(&untrusted_entry, want),
            "后缀声明 `{want}` 的判定必须与整份重查逐字节相同（信任前缀不许改判定）"
        );
    }

    // **判据 ①（结构计数）**：前缀不再重查 ⇒ 内核检查数严格下降。
    // `PERF` 行（仓库惯例，见 `docs/PERF.md`）：判读用，断言才是判据。
    println!(
        "PERF entry_trust kernel_checks full={untrusted_checks} trusted={trusted_checks} \
entry_decls full={} trusted={}",
        untrusted_names.len(),
        trusted_names.len()
    );
    assert!(
        trusted_checks < untrusted_checks,
        "信任前缀必须让内核检查数下降：整份={untrusted_checks} · 带信任={trusted_checks}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// **§5.1 的验收读数必须能从 `ProjectReport` 上读到**（用户 2026-10-01 拍板的
/// 「重查命令数 = 1（无人依赖）/ 1+m（被 m 条依赖）」）。
///
/// 它的载体是 `CompileOutput.stats.kernel_checks`（定义原文：「`try_check_declar`
/// 的实际调用次数，**受信任前缀不计入**」）—— 但组装段（`assemble_report`）把逐模块
/// 事件**重建**了 ⇒ `flat_out.stats` 被丢掉 ⇒ LSP / `QueryDoc` 那条路**恒读 0** ✗
/// （台账 G-29 的 `notes` 记了这条实测）。
///
/// 判据是**结构计数**（墙钟会翻面，计数不会）：整份重查 > 0；带信任前缀**更少**。
#[test]
fn project_report_carries_the_recheck_count() {
    let dir = tmp_dir("recheck");
    write(&dir, "sokonanoda.toml", "name = \"recheck\"\n");
    write(&dir, "lib/Lib.sokonanoda", "def Point : Type := Nat\n");
    let mut entry = String::from("import lib.Lib\n");
    for i in 0..6 {
        entry.push_str(&format!("theorem t{i} (a b : Point) : Point := a\n"));
    }
    write(&dir, "units/entry.sokonanoda", &entry);
    let options = CompileOptions::default();

    // ① 整份重查（今天的行为）：报告上必须**读得到**这个数（不是 0）。
    let full = compile_plan(
        plan_project(
            &dir.join("units/entry.sokonanoda"),
            None,
            Some(dir.as_path()),
        ),
        &options,
    );
    assert!(
        full.kernel_checks > 0,
        "整份重查的 `kernel_checks` 必须 > 0（实测恒 0 就是组装段把它丢了）"
    );

    // ② 带信任前缀（前 4 条命令已核过）⇒ 重查数必须**严格更少**。
    let plan = plan_project(
        &dir.join("units/entry.sokonanoda"),
        None,
        Some(dir.as_path()),
    );
    let trusted = compile_plan_incremental(
        plan,
        &options,
        Some(crate::project::session::EntryTrust {
            plan: crate::compile::TrustPlan {
                before: 4,
                trusted_extra: Vec::new(),
                prev_signatures: Vec::new(),
                text_unchanged: Vec::new(),
                allow_cutoff: false,
                trusted_entered: None,
            },
            failures: HashMap::new(),
        }),
        None,
        // 测试走**不建检查点**那条路（跨调用复用是 LSP/`QueryDoc` 的面，见设计 §33）。
        false,
    );
    assert!(
        trusted.kernel_checks < full.kernel_checks,
        "信任 4 条前缀之后重查数必须更少：整份 {} vs 信任后 {}",
        full.kernel_checks,
        trusted.kernel_checks
    );
    println!(
        "PERF recheck full={} trusted={}",
        full.kernel_checks, trusted.kernel_checks
    );
}

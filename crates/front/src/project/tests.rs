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

fn codes(report: &ProjectReport) -> Vec<&'static str> {
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

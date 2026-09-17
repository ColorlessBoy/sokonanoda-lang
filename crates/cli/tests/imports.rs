//! CLI 端到端：多文件 `import` 与项目管理（ROADMAP I16 / P3）。
//!
//! 契约（`docs/design/imports-and-projects.md` §4.9、§6）：
//!
//! * 有 `import` 的文件走项目闭包；**没有 import 的文件逐字节不变**（A1）；
//! * 依赖模块的诊断带**文件路径**，JSON 视图额外带 `file`/`module`；
//! * 依赖没编译成功时，下游只在 `import` 行报一条 `import-dependency-failed`；
//! * `--root` 指定模块根、`--no-project` 不读清单；`--text`/stdin 有 import
//!   却没有 `--root` 时明确报错。

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn tmp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sokonanoda-cli-imports-{tag}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn write(dir: &Path, relative: &str, text: &str) {
    let path = dir.join(relative);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create parent");
    }
    std::fs::write(path, text).expect("write file");
}

/// 隔离缓存目录，避免测试之间互相看到对方的编译结果。
fn run(dir: &Path, args: &[&str], stdin: Option<&str>) -> std::process::Output {
    let cache = dir.join(".cache");
    let mut child = Command::new(env!("CARGO_BIN_EXE_sokonanoda"))
        .args(args)
        .current_dir(dir)
        .env("SOKONANODA_CACHE_DIR", &cache)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn sokonanoda");
    if let Some(input) = stdin {
        use std::io::Write;
        child
            .stdin
            .as_mut()
            .expect("stdin")
            .write_all(input.as_bytes())
            .expect("write stdin");
    }
    child.wait_with_output().expect("wait")
}

fn stdout(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn stderr(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

#[test]
fn cli_compiles_a_two_file_project_without_any_manifest() {
    let dir = tmp_dir("two-file");
    write(&dir, "Bar.sokonanoda", "def bar : Nat := 2\n");
    write(
        &dir,
        "Main.sokonanoda",
        "import Bar\n\ndef two : Nat := bar\n",
    );
    let out = run(&dir, &["Main.sokonanoda"], None);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    let text = stdout(&out);
    assert!(
        text.contains("checked declaration two"),
        "the entry's checks are printed: {text}"
    );
    assert!(
        !text.contains("checked declaration bar"),
        "a dependency's successful declarations stay quiet (diagnostics still print): {text}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn import_free_files_are_byte_identical_to_the_single_file_path() {
    // A1 的最强形式：同一份没有 `import` 的文本，走文件路径（项目入口）与走
    // stdin（单文件路径）必须**逐字节相同**。用对拍而不是手写 golden，测试
    // 自己不会随事件形状漂移。
    let dir = tmp_dir("byte-identical");
    let src = "def one : Nat := 1\n#check one\n#reduce one + 1\n";
    write(&dir, "Plain.sokonanoda", src);
    let via_file = run(&dir, &["--json", "Plain.sokonanoda"], None);
    let via_stdin = run(&dir, &["--json", "-"], Some(src));
    assert!(via_file.status.success(), "stderr: {}", stderr(&via_file));
    assert_eq!(
        stdout(&via_file),
        stdout(&via_stdin),
        "an import-free file must behave exactly like the single-file path"
    );
    assert_eq!(stderr(&via_file), stderr(&via_stdin));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn dependency_diagnostics_carry_the_file_path_and_module_in_json() {
    let dir = tmp_dir("dep-diagnostic");
    write(&dir, "Logic.sokonanoda", "def bad : Nat := Prop\n");
    write(&dir, "Main.sokonanoda", "import Logic\n");
    let out = run(&dir, &["--json", "Main.sokonanoda"], None);
    let text = stdout(&out);
    assert!(
        text.contains("\"file\":\"Logic.sokonanoda\"") && text.contains("\"module\":\"Logic\""),
        "dependency diagnostics are attributed to the file: {text}"
    );
    assert!(
        !out.status.success(),
        "a failed dependency means a failed run"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_failed_dependency_blocks_the_entry_with_one_diagnostic() {
    let dir = tmp_dir("blocked");
    write(&dir, "Logic.sokonanoda", "def bad : Nat := Prop\n");
    write(
        &dir,
        "Main.sokonanoda",
        "import Logic\n\ndef use : Nat := bad\n",
    );
    let out = run(&dir, &["--json", "Main.sokonanoda"], None);
    let text = stdout(&out);
    let blocked = text
        .lines()
        .filter(|line| line.contains("import-dependency-failed"))
        .count();
    assert_eq!(blocked, 1, "exactly one blocking diagnostic: {text}");
    assert!(
        !text.contains("elab-unknown-identifier"),
        "no cascade of unknown identifiers: {text}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn missing_module_reports_the_expected_path_on_the_import_line() {
    let dir = tmp_dir("missing");
    write(&dir, "Main.sokonanoda", "import Lesson.Logic\n");
    let out = run(&dir, &["Main.sokonanoda"], None);
    let text = stderr(&out);
    assert!(
        text.contains("import-not-found") && text.contains("Lesson/Logic.sokonanoda"),
        "the message shows the path that was tried: {text}"
    );
    assert!(!out.status.success());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn root_flag_points_at_the_module_root_from_elsewhere() {
    let dir = tmp_dir("root-flag");
    write(&dir, "src/Lesson/Logic.sokonanoda", "axiom A : Prop\n");
    write(
        &dir,
        "src/Canvas.sokonanoda",
        "import Lesson.Logic\n\ndef use : Prop := A\n",
    );
    // 从别处启动、用 --root 指定模块根（stdin 也适用）。
    let out = run(
        &dir,
        &["--root", "src", "-"],
        Some("import Lesson.Logic\n\ndef use : Prop := A\n"),
    );
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn stdin_with_imports_without_root_is_a_clear_error() {
    let dir = tmp_dir("stdin-no-root");
    let out = run(&dir, &["-"], Some("import Foo\n"));
    let text = stderr(&out);
    assert!(
        text.contains("import-not-found") && text.contains("--root"),
        "stdin imports need --root: {text}"
    );
    assert!(!out.status.success());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn no_project_ignores_the_manifest_and_uses_the_entry_directory() {
    let dir = tmp_dir("no-project");
    write(&dir, "sokonanoda.toml", "src = \"lib\"\n");
    write(&dir, "lib/Logic.sokonanoda", "axiom A : Prop\n");
    write(
        &dir,
        "lib/Main.sokonanoda",
        "import Logic\n\ndef use : Prop := A\n",
    );
    // 正常：清单把模块根指到 lib/，从仓库根跑也能解析。
    let ok = run(&dir, &["lib/Main.sokonanoda"], None);
    assert!(ok.status.success(), "stderr: {}", stderr(&ok));
    // `--no-project`：不读清单，模块根 = 入口目录（同样能解析这里的 import）。
    let also_ok = run(&dir, &["--no-project", "lib/Main.sokonanoda"], None);
    assert!(also_ok.status.success(), "stderr: {}", stderr(&also_ok));
    // 反过来：入口在根目录、依赖在 lib/ —— 清单把模块根指到 lib/，所以正常；
    // `--no-project` 不读清单，模块根 = 入口目录，于是找不到 `Logic`。
    write(
        &dir,
        "Main.sokonanoda",
        "import Logic\n\ndef use : Prop := A\n",
    );
    let with_manifest = run(&dir, &["Main.sokonanoda"], None);
    assert!(
        with_manifest.status.success(),
        "the manifest points the module root at lib/: {}",
        stderr(&with_manifest)
    );
    let without_manifest = run(&dir, &["--no-project", "Main.sokonanoda"], None);
    assert!(
        !without_manifest.status.success()
            && stderr(&without_manifest).contains("import-not-found"),
        "without the manifest the module root is the entry directory: {}",
        stderr(&without_manifest)
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn manifest_requires_mismatch_is_a_warning_not_an_error() {
    let dir = tmp_dir("requires");
    write(&dir, "sokonanoda.toml", "requires = \"0.1\"\n");
    write(&dir, "Bar.sokonanoda", "def bar : Nat := 2\n");
    write(
        &dir,
        "Main.sokonanoda",
        "import Bar\n\ndef two : Nat := bar\n",
    );
    let out = run(&dir, &["Main.sokonanoda"], None);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert!(
        stderr(&out).contains("manifest-version"),
        "version mismatch is reported on stderr: {}",
        stderr(&out)
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn build_walks_a_project_and_reports_per_file_status() {
    let dir = tmp_dir("build");
    write(&dir, "Bar.sokonanoda", "def bar : Nat := 2\n");
    write(
        &dir,
        "Main.sokonanoda",
        "import Bar\n\ndef two : Nat := bar\n",
    );
    let out = run(&dir, &["build", "--json", "."], None);
    let text = stdout(&out);
    assert!(
        text.contains("\"type\":\"build.summary\"") && text.contains("\"failed\":0"),
        "build summary: {text}"
    );
    assert!(
        text.contains("Main.sokonanoda") && text.contains("Bar.sokonanoda"),
        "both files are visited: {text}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn query_resolves_imported_names_in_the_closure() {
    let dir = tmp_dir("query");
    write(
        &dir,
        "Logic.sokonanoda",
        "axiom And : Prop -> Prop -> Prop\naxiom And.intro : forall (a b : Prop), a -> b -> And a b\n",
    );
    write(
        &dir,
        "Main.sokonanoda",
        "import Lesson_Logic\nimport Logic\n\ntheorem t (a b : Prop) (ha : a) (hb : b) : And a b := And.intro a b ha hb\n",
    );
    // `import Lesson_Logic` 不存在 ⇒ 明确报错；把文件改成只 import Logic 再查。
    write(
        &dir,
        "Main.sokonanoda",
        "import Logic\n\ntheorem t (a b : Prop) (ha : a) (hb : b) : And a b := And.intro a b ha hb\n",
    );
    let out = run(&dir, &["query", "check", "--file", "Main.sokonanoda"], None);
    let text = stdout(&out);
    assert!(
        text.contains("\"decl_checked\": 1") && text.contains("\"ok\": true"),
        "the imported declaration is visible to the theorem: {text} stderr: {}",
        stderr(&out)
    );
    let _ = std::fs::remove_dir_all(&dir);
}

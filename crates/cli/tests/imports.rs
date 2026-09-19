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
    run_with_cache(dir, &dir.join(".cache"), args, stdin)
}

/// 固定缓存目录：观察"热跑命中"与"依赖变更后失效"必须复用同一个缓存。
fn run_with_cache(
    dir: &Path,
    cache: &Path,
    args: &[&str],
    stdin: Option<&str>,
) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_sokonanoda"))
        .args(args)
        .current_dir(dir)
        .env("SOKONANODA_CACHE_DIR", cache)
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
    // G-12 后入口与依赖都用**绝对**路径解析，所以依赖诊断的 `file` 也是绝对路径：
    // 这里按后缀断言（写死相对路径会随模块根发现一起漂）。
    let diagnostic = text
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .find(|event| event["module"] == "Logic")
        .unwrap_or_else(|| panic!("dependency diagnostics are attributed to the file: {text}"));
    assert!(
        diagnostic["file"]
            .as_str()
            .is_some_and(|file| file.ends_with("Logic.sokonanoda")),
        "dependency diagnostics are attributed to the file: {diagnostic}"
    );
    assert_eq!(diagnostic["module"], "Logic");
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

// ── G-12：入口写相对还是绝对，解析结果必须一样（cwd 不参与语义）──────────

/// 事件流逐行解析（去掉空行）。
fn events(out: &std::process::Output) -> Vec<serde_json::Value> {
    stdout(out)
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            serde_json::from_str::<serde_json::Value>(line)
                .unwrap_or_else(|err| panic!("每一行都是 JSON（{err}）：{line}"))
        })
        .collect()
}

fn count_type(events: &[serde_json::Value], kind: &str) -> usize {
    events.iter().filter(|event| event["type"] == kind).count()
}

fn abs_str(path: &Path) -> String {
    path.display().to_string()
}

#[test]
fn a_relative_entry_and_its_absolute_twin_agree_event_for_event() {
    // G-12 症状：同一个文件写相对路径报 `import-not-found`，写成绝对路径就绿。
    // 修后：模块根发现与 cwd 无关——两种写法**逐行一致**（含缓存摘要相同：
    // 第二次跑命中的是同一把键）。
    let dir = tmp_dir("rel-vs-abs");
    write(&dir, "sokonanoda.toml", "name = \"proj\"\n");
    write(&dir, "lib/Lib.sokonanoda", "axiom P : Prop\n");
    write(
        &dir,
        "units/u.sokonanoda",
        "import lib.Lib\n\ntheorem t (h : P) : P := h\n",
    );

    let relative = run(&dir, &["--json", "units/u.sokonanoda"], None);
    let absolute = run(
        &dir,
        &["--json", &abs_str(&dir.join("units/u.sokonanoda"))],
        None,
    );
    assert!(relative.status.success(), "stderr: {}", stderr(&relative));
    assert_eq!(relative.status.code(), Some(0));
    assert_eq!(
        stdout(&relative),
        stdout(&absolute),
        "同一文件的相对/绝对写法必须给出同一份事件流\nrel: {}\nabs: {}",
        stdout(&relative),
        stdout(&absolute)
    );
    let list = events(&relative);
    assert_eq!(count_type(&list, "diagnostic"), 0, "{list:?}");
    assert_eq!(count_type(&list, "decl.checked"), 1, "{list:?}");
    assert_eq!(
        list.last().map(|event| event["type"].clone()),
        Some(serde_json::json!("decl.checked")),
        "{list:?}"
    );
}

#[test]
fn a_bare_file_name_under_a_subdirectory_finds_the_ancestor_manifest() {
    // 形状②：入口用**裸文件名**、cwd 在清单的子目录里。上溯若在空分量
    // （`Path::new("units").parent()` = `""`）处断掉，零配置退路就抢跑 —
    // 模块根退化成入口目录，祖先清单里的 `lib/` 再也看不见。
    let dir = tmp_dir("bare-name");
    write(&dir, "sokonanoda.toml", "name = \"proj\"\n");
    write(&dir, "lib/Logic.sokonanoda", "axiom P : Prop\n");
    write(
        &dir,
        "units/u.sokonanoda",
        "import lib.Logic\n\ntheorem t (h : P) : P := h\n",
    );

    let from_subdir = run(&dir.join("units"), &["--json", "u.sokonanoda"], None);
    assert!(
        from_subdir.status.success(),
        "stderr: {}",
        stderr(&from_subdir)
    );
    let list = events(&from_subdir);
    assert_eq!(count_type(&list, "diagnostic"), 0, "{list:?}");
    assert_eq!(count_type(&list, "decl.checked"), 1, "{list:?}");

    // 与"从仓库根用相对路径"以及"绝对路径"两种写法逐行一致。
    let from_root = run(&dir, &["--json", "units/u.sokonanoda"], None);
    let absolute = run(
        &dir,
        &["--json", &abs_str(&dir.join("units/u.sokonanoda"))],
        None,
    );
    assert_eq!(stdout(&from_subdir), stdout(&from_root));
    assert_eq!(stdout(&from_subdir), stdout(&absolute));
}

#[test]
fn query_project_reports_the_same_absolute_root_for_both_entry_spellings() {
    // `docs/protocol.md` 的 `soko/project` 契约："Paths are absolute"、root 永不空。
    //
    // 两次调用**各用一份冷缓存**：本用例量的是 G-12 的模块根发现，不是缓存行为。
    // （既有缺陷另记：项目缓存**热命中**时 `set_cached_entry` 把 `project` 置空，
    // `query project` 会答 `project:null / reason:"no-path"`；它在 HEAD 上就能复现
    // ——`grade <绝对路径>` 之后 `query project --file <同一路径>`——与本 WO 无关，
    // 不在本单范围。）
    let dir = tmp_dir("query-root");
    write(&dir, "sokonanoda.toml", "name = \"proj\"\n");
    write(&dir, "lib/Lib.sokonanoda", "axiom P : Prop\n");
    write(
        &dir,
        "units/u.sokonanoda",
        "import lib.Lib\n\ntheorem t (h : P) : P := h\n",
    );
    let entry = dir.join("units/u.sokonanoda");

    let relative = run_with_cache(
        &dir,
        &dir.join(".cache-relative"),
        &[
            "query",
            "project",
            "--file",
            "units/u.sokonanoda",
            "--compact",
        ],
        None,
    );
    let absolute = run_with_cache(
        &dir,
        &dir.join(".cache-absolute"),
        &["query", "project", "--file", &abs_str(&entry), "--compact"],
        None,
    );
    assert!(relative.status.success(), "stderr: {}", stderr(&relative));
    assert!(absolute.status.success(), "stderr: {}", stderr(&absolute));
    let relative: serde_json::Value = serde_json::from_slice(&relative.stdout).expect("one JSON");
    let absolute: serde_json::Value = serde_json::from_slice(&absolute.stdout).expect("one JSON");

    let root = relative["data"]["project"]["root"]
        .as_str()
        .expect("root is a string");
    assert_eq!(
        root,
        absolute["data"]["project"]["root"].as_str().unwrap_or(""),
        "两种写法给出逐字相同的模块根：rel={relative} abs={absolute}"
    );
    assert_eq!(
        root,
        std::fs::canonicalize(&dir)
            .expect("canonicalize")
            .to_str()
            .unwrap(),
        "root 是绝对路径、等于清单所在目录（永不空）"
    );
    assert_eq!(relative["data"]["project"]["entry"], "units.u");
    assert_eq!(absolute["data"]["project"]["entry"], "units.u");
    assert_eq!(
        relative["data"]["project"]["manifest"].as_str(),
        std::fs::canonicalize(dir.join("sokonanoda.toml"))
            .expect("canonicalize manifest")
            .to_str()
    );
}

#[test]
fn without_the_ancestor_manifest_an_import_still_fails() {
    // 负向护栏："修好"不能靠"到处都能找到清单"。把祖先清单删掉（这里从未建过），
    // 祖先目录的 `lib/` 就与入口无关，`import lib.Lib` 必须照旧失败。
    let dir = tmp_dir("no-ancestor-manifest");
    write(&dir, "lib/Lib.sokonanoda", "axiom P : Prop\n");
    write(
        &dir,
        "units/u.sokonanoda",
        "import lib.Lib\n\ntheorem t (h : P) : P := h\n",
    );
    let out = run(&dir.join("units"), &["--json", "u.sokonanoda"], None);
    assert_eq!(out.status.code(), Some(1), "stderr: {}", stderr(&out));
    assert!(
        stdout(&out).contains("import-not-found"),
        "没有清单 ⇒ 零配置退路（模块根 = 入口目录）：{}",
        stdout(&out)
    );
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

#[test]
fn a_project_cache_hits_and_a_dependency_change_invalidates_it() {
    let dir = tmp_dir("cache");
    let cache = dir.join(".cache");
    write(&dir, "Bar.sokonanoda", "def bar : Nat := 2\n");
    write(&dir, "Main.sokonanoda", "import Bar\n\n#reduce bar\n");

    // 冷跑：编译；热跑：命中，且输出逐字节一致。
    let cold = run_with_cache(&dir, &cache, &["--json", "Main.sokonanoda"], None);
    let warm = run_with_cache(&dir, &cache, &["--json", "Main.sokonanoda"], None);
    assert_eq!(stdout(&cold), stdout(&warm), "warm output must match cold");
    assert!(
        stdout(&cold).contains("\"value\":\"2\""),
        "{}",
        stdout(&cold)
    );

    // 改**依赖**（入口一字未动）：闭包哈希变 ⇒ 必须重编译，不能拿旧报告。
    write(&dir, "Bar.sokonanoda", "def bar : Nat := 3\n");
    let after = run_with_cache(&dir, &cache, &["--json", "Main.sokonanoda"], None);
    assert!(
        stdout(&after).contains("\"value\":\"3\""),
        "a dependency change must invalidate the entry's cached report: {}",
        stdout(&after)
    );

    // `build` 也能看到 hit/compiled 的区别：入口（项目键）已经在上面热过，
    // 所以第一次 build 只需编译没有 import 的 `Bar`，第二次两个都是 hit。
    let build_first = run_with_cache(&dir, &cache, &["build", "--json", "."], None);
    assert!(
        stdout(&build_first).contains("\"compiled\":1")
            && stdout(&build_first).contains("\"hit\":1"),
        "the entry is a project hit, the plain file compiles: {}",
        stdout(&build_first)
    );
    let build_second = run_with_cache(&dir, &cache, &["build", "--json", "."], None);
    assert!(
        stdout(&build_second).contains("\"hit\":2"),
        "everything is warm now: {}",
        stdout(&build_second)
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn query_uses_the_same_project_cache_as_check_and_build() {
    // `query` 曾自己重编译整个闭包（热跑 37ms，`check` 热跑 3ms）。现在三条命令
    // 共用 `crate::project_cache` 的同一份摘要键：query 冷跑写缓存，`build` 立刻
    // 就能看到入口是 hit；query 冷/热计数一致、且与 `--json` 事件计数相同。
    let dir = tmp_dir("query-cache");
    let cache = dir.join(".cache");
    write(&dir, "Lib.sokonanoda", "axiom P : Prop\naxiom proofP : P\n");
    write(
        &dir,
        "Main.sokonanoda",
        "import Lib\n\ntheorem main_s : P := proofP\ntheorem open_x : P := sorry\n",
    );

    let cold = run_with_cache(
        &dir,
        &cache,
        &["query", "check", "--file", "Main.sokonanoda"],
        None,
    );
    assert!(cold.status.success(), "{}", stderr(&cold));
    let counts: serde_json::Value = serde_json::from_slice(&cold.stdout).expect("one JSON object");
    let counts = counts["data"]["counts"].clone();

    // `build` 与 query 用同一份键：入口（项目）应当是 hit，纯文件才需要编译。
    let build = run_with_cache(&dir, &cache, &["build", "--json", "."], None);
    let summary = stdout(&build)
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .find(|event| event["type"] == "build.summary")
        .expect("build prints a summary");
    assert!(
        summary["hit"].as_u64().unwrap_or(0) >= 1,
        "the project entry must be a cache hit after `query` warmed it: {summary}"
    );

    // 热跑与冷跑逐字节一致；且计数与 `--json` 事件流相同。
    let warm = run_with_cache(
        &dir,
        &cache,
        &["query", "check", "--file", "Main.sokonanoda"],
        None,
    );
    assert_eq!(stdout(&cold), stdout(&warm), "warm must match cold");
    let events = run_with_cache(&dir, &cache, &["--json", "Main.sokonanoda"], None);
    let mut expected = serde_json::json!({
        "decl_checked": 0, "example_checked": 0, "exercise_open": 0,
        "expr_typed": 0, "expr_reduced": 0, "decl_printed": 0,
    });
    for line in stdout(&events).lines() {
        let event: serde_json::Value = serde_json::from_str(line).expect("json line");
        let key = match event["type"].as_str().unwrap_or("") {
            "decl.checked" => "decl_checked",
            "example.checked" => "example_checked",
            "exercise.open" => "exercise_open",
            "expr.typed" => "expr_typed",
            "expr.reduced" => "expr_reduced",
            "decl.printed" => "decl_printed",
            _ => continue,
        };
        expected[key] = serde_json::json!(expected[key].as_u64().unwrap_or(0) + 1);
    }
    assert_eq!(counts, expected, "query counts must equal the event stream");

    // 改依赖 ⇒ 摘要变 ⇒ 不能拿旧结果。
    write(&dir, "Lib.sokonanoda", "axiom Q : Prop\naxiom proofQ : Q\n");
    let after = run_with_cache(
        &dir,
        &cache,
        &["query", "check", "--file", "Main.sokonanoda"],
        None,
    );
    assert!(
        !after.status.success() || !stderr(&after).is_empty() || stdout(&after) != stdout(&cold),
        "a dependency change must invalidate the entry's cached report"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

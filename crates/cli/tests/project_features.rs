//! CLI 端到端：项目行为的**第二层**补充（ROADMAP I16）。
//!
//! `imports.rs` 已钉死 12 条基线（零清单两文件、`--root`/`--no-project`、缺模块报
//! 期望路径、依赖失败只阻断一次、`build` 汇总、单依赖缓存失效、依赖诊断带
//! `file`/`module`）。本文件只测基线没覆盖的：嵌套模块路径、菱形依赖、多入口共享
//! 依赖、依赖解析失败、同名文件/目录、`import` 置顶与写法错误、`build <目录>` 的
//! 逐文件状态、项目版 `query check` 计数契约、两种退出码。每个用例都跑**真二进制**
//! （`CARGO_BIN_EXE_sokonanoda`），临时目录带 tag + pid。

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn tmp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sokonanoda-cli-project-features-{tag}-{}",
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

/// `--json` 的每一行都必须是一个事件对象；不是就报出那一行。
fn events(out: &std::process::Output) -> Vec<serde_json::Value> {
    stdout(out)
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            serde_json::from_str(line).unwrap_or_else(|e| panic!("不是事件对象（{e}）：{line}"))
        })
        .collect()
}

/// 事件流里 `code` 命中给定值的条数（诊断与警告共用 `code` 字段）。
fn count_code(events: &[serde_json::Value], code: &str) -> usize {
    events.iter().filter(|event| event["code"] == code).count()
}

/// 恰好一条给定 `type` 的事件（多了少了都是测试自己写错）。
fn event_of<'a>(events: &'a [serde_json::Value], kind: &str) -> &'a serde_json::Value {
    let matched: Vec<&serde_json::Value> = events.iter().filter(|e| e["type"] == kind).collect();
    assert_eq!(matched.len(), 1, "期望恰好一条 {kind}：{events:?}");
    matched[0]
}

#[test]
fn nested_module_paths_resolve_at_two_levels() {
    let dir = tmp_dir("nested");
    // 模块名 = 模块根下的相对路径：`Lib.And` ↔ `Lib/And.sokonanoda`、
    // `A.B.C` ↔ `A/B/C.sokonanoda`（两级嵌套，零清单）。
    write(
        &dir,
        "Lib/And.sokonanoda",
        "axiom And : Prop -> Prop -> Prop\n",
    );
    write(&dir, "A/B/C.sokonanoda", "def c : Nat := 3\n");
    write(
        &dir,
        "Main.sokonanoda",
        "import Lib.And\nimport A.B.C\n\n\
def use : Prop -> Prop -> Prop := And\n\
def three : Nat := c\n",
    );
    let out = run(&dir, &["--json", "Main.sokonanoda"], None);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    let list = events(&out);
    assert_eq!(count_code(&list, "elab-unknown-identifier"), 0, "{list:?}");
    assert_eq!(count_code(&list, "import-not-found"), 0, "{list:?}");
    let names: Vec<&str> = list
        .iter()
        .filter(|e| e["type"] == "decl.checked")
        .map(|e| e["name"].as_str().expect("name"))
        .collect();
    assert_eq!(names, vec!["use", "three"], "{list:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_diamond_dependency_compiles_once_and_its_edit_invalidates_the_entry() {
    let dir = tmp_dir("diamond");
    let cache = dir.join(".cache");
    write(&dir, "D.sokonanoda", "def shared : Nat := 7\n");
    write(
        &dir,
        "B.sokonanoda",
        "import D\n\ndef b : Nat := shared + 1\n",
    );
    write(
        &dir,
        "C.sokonanoda",
        "import D\n\ndef c : Nat := shared + 2\n",
    );
    write(
        &dir,
        "Main.sokonanoda",
        "import B\nimport C\n\n#reduce b + c\n",
    );

    // D 被 B 与 C 各 import、Main 再 import B/C。D 只进闭包一次，所以既不该有跨模块
    // 重名，也不该有"同一个声明装了两遍"的内核拒绝。
    let cold = run_with_cache(&dir, &cache, &["--json", "Main.sokonanoda"], None);
    assert!(cold.status.success(), "stderr: {}", stderr(&cold));
    let cold_events = events(&cold);
    let reduced = event_of(&cold_events, "expr.reduced");
    assert_eq!(reduced["value"], "17", "shared = 7：{cold_events:?}");
    assert!(
        cold_events.iter().all(|e| e["type"] != "diagnostic"),
        "干净的菱形不该有诊断：{cold_events:?}"
    );
    assert_eq!(count_code(&cold_events, "elab-duplicate-declaration"), 0);
    assert_eq!(count_code(&cold_events, "import-name-collision"), 0);

    // 热跑逐字节一致 ⇒ 闭包摘要稳定、缓存命中（含 D 的参与）。
    let warm = run_with_cache(&dir, &cache, &["--json", "Main.sokonanoda"], None);
    assert_eq!(stdout(&cold), stdout(&warm), "热跑必须与冷跑一致");

    // 改**传递依赖** D：入口 Main 一字未动，报告也必须重算（Merkle 链）。
    write(&dir, "D.sokonanoda", "def shared : Nat := 8\n");
    let after = run_with_cache(&dir, &cache, &["--json", "Main.sokonanoda"], None);
    let after_events = events(&after);
    assert_eq!(event_of(&after_events, "expr.reduced")["value"], "19");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn two_entries_share_one_dependency_with_independent_caches() {
    let dir = tmp_dir("shared-dep");
    let cache = dir.join(".cache");
    write(&dir, "Shared.sokonanoda", "def shared : Nat := 5\n");
    write(
        &dir,
        "One.sokonanoda",
        "import Shared\n\n#reduce shared + 1\n",
    );
    write(
        &dir,
        "Two.sokonanoda",
        "import Shared\n\n#reduce shared + 2\n",
    );

    let one = run_with_cache(&dir, &cache, &["--json", "One.sokonanoda"], None);
    assert!(one.status.success(), "stderr: {}", stderr(&one));
    assert_eq!(event_of(&events(&one), "expr.reduced")["value"], "6");
    let one_warm = run_with_cache(&dir, &cache, &["--json", "One.sokonanoda"], None);
    assert_eq!(stdout(&one), stdout(&one_warm), "One 的热跑一致");

    // Two 是另一个入口：它有自己的闭包报告，绝不能回放 One 的那份。
    let two = run_with_cache(&dir, &cache, &["--json", "Two.sokonanoda"], None);
    assert!(two.status.success(), "stderr: {}", stderr(&two));
    assert_eq!(event_of(&events(&two), "expr.reduced")["value"], "7");
    let two_warm = run_with_cache(&dir, &cache, &["--json", "Two.sokonanoda"], None);
    assert_eq!(stdout(&two), stdout(&two_warm), "Two 的热跑一致");
    assert_ne!(stdout(&one_warm), stdout(&two_warm), "两个入口 = 两份报告");

    // 两个入口闭包都热了：`build` 只需把 Shared **自己**编译一次——共享依赖在一次
    // build 里是一个文件，不是"每个入口各编一遍"。
    let build = run_with_cache(&dir, &cache, &["build", "--json", "."], None);
    let build_events = events(&build);
    let summary = event_of(&build_events, "build.summary");
    assert_eq!(summary["hit"], 2, "两个入口闭包都是命中：{summary}");
    assert_eq!(summary["compiled"], 1, "Shared 只编译一次：{summary}");

    // 共享依赖改动 ⇒ 两个入口各自失效，各自算出新值。
    write(&dir, "Shared.sokonanoda", "def shared : Nat := 10\n");
    let one = run_with_cache(&dir, &cache, &["--json", "One.sokonanoda"], None);
    let two = run_with_cache(&dir, &cache, &["--json", "Two.sokonanoda"], None);
    assert_eq!(event_of(&events(&one), "expr.reduced")["value"], "11");
    assert_eq!(event_of(&events(&two), "expr.reduced")["value"], "12");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_parse_error_in_a_dependency_is_attributed_to_its_file() {
    let dir = tmp_dir("dep-parse-error");
    write(&dir, "Bad.sokonanoda", "def broken : Nat := )\n");
    write(
        &dir,
        "Main.sokonanoda",
        "import Bad\n\ndef use : Nat := broken\n",
    );
    let out = run(&dir, &["--json", "Main.sokonanoda"], None);
    assert_eq!(out.status.code(), Some(1), "stderr: {}", stderr(&out));
    let list = events(&out);

    // 入口只在 import 行被阻断一次，且不级联未知标识符。
    assert_eq!(count_code(&list, "import-dependency-failed"), 1, "{list:?}");
    let blocking = list
        .iter()
        .find(|e| e["code"] == "import-dependency-failed")
        .expect("import-dependency-failed");
    assert_eq!(blocking["span"]["start"]["line"], 1, "{blocking}");
    assert_eq!(count_code(&list, "elab-unknown-identifier"), 0, "{list:?}");

    // 依赖**自己**的解析诊断带它的路径与模块名（协议码 `import-module-invalid`：它是
    // 闭包加载期发现的，不是入口的 parse 诊断）。
    assert_eq!(count_code(&list, "import-module-invalid"), 1, "{list:?}");
    let parse_error = list
        .iter()
        .find(|e| e["code"] == "import-module-invalid")
        .expect("import-module-invalid");
    // G-12 后依赖模块的路径也是绝对的（入口先绝对化再解析）：按后缀断言，
    // 与同文件 `build.file` 的 `ends_with` 写法一致。
    assert!(
        parse_error["file"]
            .as_str()
            .is_some_and(|file| file.ends_with("Bad.sokonanoda")),
        "依赖诊断带它的绝对路径：{parse_error}"
    );
    assert_eq!(parse_error["module"], "Bad");
    assert_eq!(parse_error["stage"], "import");
    assert_eq!(parse_error["span"]["start"]["line"], 1, "{parse_error}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_file_and_a_directory_module_with_the_same_name_coexist() {
    // 记录实测行为（不先验假设）：模块名的**末分量**只匹配 `<名>.sokonanoda` 文件、
    // 中间分量只匹配目录（`front::project::resolve::exact_lookup`）。所以
    // `Foo.sokonanoda` 与 `Foo/Bar.sokonanoda` 能并存：`import Foo` 命中文件、
    // `import Foo.Bar` 命中目录，两条边互不干扰，也没有"歧义"诊断。
    let dir = tmp_dir("file-vs-dir");
    write(&dir, "Foo.sokonanoda", "def from_file : Nat := 1\n");
    write(&dir, "Foo/Bar.sokonanoda", "def from_nested : Nat := 2\n");
    write(
        &dir,
        "Main.sokonanoda",
        "import Foo\nimport Foo.Bar\n\ndef use : Nat := from_file + from_nested\n",
    );
    let out = run(&dir, &["--json", "Main.sokonanoda"], None);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    let list = events(&out);
    assert_eq!(count_code(&list, "import-not-found"), 0, "{list:?}");
    assert_eq!(event_of(&list, "decl.checked")["name"], "use", "{list:?}");

    // 只有目录、没有同名文件时，`import Foo` 是明确的 not-found（期望 `Foo.sokonanoda`：
    // 目录不是模块，不会退回去把目录当成模块），而 `Foo.Bar` 仍然可用。
    let only_dir = tmp_dir("dir-only");
    write(
        &only_dir,
        "Foo/Bar.sokonanoda",
        "def from_nested : Nat := 2\n",
    );
    write(&only_dir, "Main.sokonanoda", "import Foo\n");
    let text = stdout(&run(&only_dir, &["--json", "Main.sokonanoda"], None));
    assert!(
        text.contains("import-not-found") && text.contains("Foo.sokonanoda"),
        "--json 要说清期望路径：{text}"
    );
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&only_dir);
}

#[test]
fn an_import_after_a_declaration_reports_on_the_offending_line() {
    let dir = tmp_dir("import-precedence");
    write(&dir, "Bar.sokonanoda", "def bar : Nat := 2\n");
    write(&dir, "Main.sokonanoda", "def one : Nat := 1\nimport Bar\n");
    let out = run(&dir, &["--json", "Main.sokonanoda"], None);
    assert_eq!(out.status.code(), Some(1), "stderr: {}", stderr(&out));
    let list = events(&out);
    assert_eq!(count_code(&list, "import-must-precede-declarations"), 1);
    let error = list
        .iter()
        .find(|e| e["code"] == "import-must-precede-declarations")
        .expect("import-must-precede-declarations");
    assert_eq!(error["type"], "diagnostic");
    assert_eq!(error["stage"], "parse");
    assert_eq!(error["span"]["start"]["line"], 2, "{error}");

    // 人类视图同样指到第 2 行第 1 列。
    let human = run(&dir, &["Main.sokonanoda"], None);
    assert_eq!(human.status.code(), Some(1));
    assert!(
        stderr(&human).contains("Main.sokonanoda:2:1: error[parse]"),
        "stderr: {}",
        stderr(&human)
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn malformed_import_lines_report_import_malformed() {
    let dir = tmp_dir("import-malformed");
    // `import` 后面没有名字。
    write(&dir, "NoName.sokonanoda", "import\n");
    let out = run(&dir, &["--json", "NoName.sokonanoda"], None);
    assert_eq!(out.status.code(), Some(1), "stderr: {}", stderr(&out));
    let list = events(&out);
    let error = list
        .iter()
        .find(|e| e["code"] == "import-malformed")
        .expect("import-malformed");
    assert_eq!(error["stage"], "parse");
    assert_eq!(count_code(&list, "import-malformed"), 1, "{list:?}");

    // 一行写了两个名字。
    write(&dir, "TwoNames.sokonanoda", "import a b\n");
    let out = run(&dir, &["--json", "TwoNames.sokonanoda"], None);
    assert_eq!(out.status.code(), Some(1), "stderr: {}", stderr(&out));
    let list = events(&out);
    let error = list
        .iter()
        .find(|e| e["code"] == "import-malformed")
        .expect("import-malformed");
    assert_eq!(error["stage"], "parse");
    assert_eq!(error["span"]["start"]["line"], 1, "{error}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_dashed_module_name_is_rejected_with_the_dash_hint() {
    let dir = tmp_dir("dashed-import");
    // 文件名里的横线不是模块名字符：词法层就该给出这条专门诊断（而不是泛泛的
    // `expected '->' or '--'`），并附上"改文件名"的 hint。
    write(&dir, "unit1-propositions.sokonanoda", "def x : Nat := 1\n");
    write(&dir, "Main.sokonanoda", "import unit1-propositions\n");
    let out = run(&dir, &["--json", "Main.sokonanoda"], None);
    assert_eq!(out.status.code(), Some(1), "stderr: {}", stderr(&out));
    let list = events(&out);
    assert_eq!(count_code(&list, "import-not-a-valid-module-name"), 1);
    let error = list
        .iter()
        .find(|e| e["code"] == "import-not-a-valid-module-name")
        .expect("import-not-a-valid-module-name");
    assert_eq!(error["stage"], "parse");
    assert_eq!(error["span"]["start"]["line"], 1);
    let hint = error["hint"].as_str().expect("hint 是字符串");
    assert!(hint.contains("`-`") && hint.contains("横线"), "{hint}");
    let message = error["message"].as_str().expect("message 是字符串");
    assert!(message.contains("不能有 `-`"), "{message}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn build_walks_a_project_directory_and_reports_each_status() {
    let dir = tmp_dir("build-walk");
    write(
        &dir,
        "proj/sokonanoda.toml",
        "name = \"proj\"\nsrc = \".\"\n",
    );
    write(
        &dir,
        "proj/Lib/And.sokonanoda",
        "axiom And : Prop -> Prop -> Prop\n",
    );
    write(
        &dir,
        "proj/Main.sokonanoda",
        "import Lib.And\n\ndef use : Prop -> Prop -> Prop := And\n",
    );
    write(&dir, "proj/Broken.sokonanoda", "def broken : Nat := )\n");

    // 从项目**外面**给一个目录：build 递归收集 + 排序，再逐文件报告状态。
    let out = run(&dir, &["build", "--json", "proj"], None);
    assert!(out.status.success(), "解析失败只计数：{}", stderr(&out));
    let list = events(&out);
    let status_of = |name: &str| -> String {
        let matched: Vec<&serde_json::Value> = list
            .iter()
            .filter(|e| e["type"] == "build.file")
            .filter(|e| e["file"].as_str().is_some_and(|f| f.ends_with(name)))
            .collect();
        assert_eq!(matched.len(), 1, "每文件一条 build.file：{list:?}");
        matched[0]["status"].as_str().expect("status").to_string()
    };
    assert_eq!(status_of("And.sokonanoda"), "compiled");
    assert_eq!(status_of("Main.sokonanoda"), "compiled");
    assert_eq!(status_of("Broken.sokonanoda"), "failed");
    let summary = event_of(&list, "build.summary");
    assert_eq!(summary["files"], 3);
    assert_eq!(summary["hit"], 0);
    assert_eq!(summary["compiled"], 2);
    assert_eq!(summary["failed"], 1);

    // 第二次：干净的两个文件命中缓存，坏文件依旧 failed（它不进缓存）。
    let again = run(&dir, &["build", "--json", "proj"], None);
    let again_events = events(&again);
    let summary = event_of(&again_events, "build.summary");
    assert_eq!(summary["hit"], 2, "{summary}");
    assert_eq!(summary["compiled"], 0, "{summary}");
    assert_eq!(summary["failed"], 1, "{summary}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn query_check_counts_match_the_json_stream_for_a_nested_project() {
    let dir = tmp_dir("query-project");
    write(
        &dir,
        "Lib/And.sokonanoda",
        "axiom And : Prop -> Prop -> Prop\n",
    );
    write(&dir, "A/B/C.sokonanoda", "def c : Nat := 3\n");
    write(
        &dir,
        "Main.sokonanoda",
        "import Lib.And\nimport A.B.C\n\n\
def myAnd : Prop -> Prop -> Prop := And\n\
example : forall (a : Prop), a -> a := sorry\n\
#check c\n#reduce c + 1\n",
    );

    // 有 import 的文件：`query check` 看的是**闭包**（`Lib.And` 的名字可用）。
    let query = run(&dir, &["query", "check", "--file", "Main.sokonanoda"], None);
    assert_eq!(query.status.code(), Some(0), "stderr: {}", stderr(&query));
    let text = stdout(&query);
    let value: serde_json::Value = serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("query check 必须输出单个 JSON 对象（{e}）：{text}"));
    assert_eq!(value["schema"], "soko.query/1");
    assert_eq!(value["ok"], true);
    assert_eq!(value["data"]["failed"].as_array().map(Vec::len), Some(0));
    let counts = &value["data"]["counts"];

    // 同一份判卷的另一个视图：全量事件流（与 query 共用缓存目录 ⇒ 顺带钉住热回放）。
    let stream = run(&dir, &["--json", "Main.sokonanoda"], None);
    assert!(stream.status.success(), "stderr: {}", stderr(&stream));
    let list = events(&stream);
    let count_of = |kind: &str| list.iter().filter(|e| e["type"] == kind).count();
    for (key, kind) in [
        ("decl_checked", "decl.checked"),
        ("example_checked", "example.checked"),
        ("exercise_open", "exercise.open"),
        ("expr_typed", "expr.typed"),
        ("expr_reduced", "expr.reduced"),
        ("decl_printed", "decl.printed"),
    ] {
        assert_eq!(
            counts[key],
            serde_json::json!(count_of(kind)),
            "counts.{key} 必须等于 {kind} 事件数：{value}"
        );
    }
    assert_eq!(counts["decl_checked"], 1, "入口自己的 def：{value}");
    assert_eq!(counts["exercise_open"], 1, "入口自己的开放练习：{value}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn entry_rejection_exits_one_while_a_dependency_sorry_exits_zero() {
    // 入口被内核拒绝 ⇒ 1。
    let rejected = tmp_dir("entry-rejected");
    write(&rejected, "Dep.sokonanoda", "def good : Nat := 1\n");
    write(
        &rejected,
        "Main.sokonanoda",
        "import Dep\n\ndef bad : Nat := Prop\n",
    );
    let out = run(&rejected, &["--json", "Main.sokonanoda"], None);
    assert_eq!(out.status.code(), Some(1), "stderr: {}", stderr(&out));
    let list = events(&out);
    assert!(
        list.iter().any(|e| e["type"] == "diagnostic"),
        "入口的内核拒绝要报出来：{list:?}"
    );
    assert_eq!(count_code(&list, "import-dependency-failed"), 0, "{list:?}");

    // 依赖里有开放 `sorry` ⇒ 0 + import 行上的警告。
    let open = tmp_dir("dep-sorry");
    write(
        &open,
        "Lesson.sokonanoda",
        "def done : Nat := 1\ntheorem later : forall (a : Prop), a -> a := sorry\n",
    );
    write(
        &open,
        "Main.sokonanoda",
        "import Lesson\n\ndef use : Nat := done\n",
    );
    let out = run(&open, &["--json", "Main.sokonanoda"], None);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    let list = events(&out);
    assert!(list.iter().all(|e| e["type"] != "diagnostic"), "{list:?}");
    assert_eq!(
        count_code(&list, "import-has-open-exercises"),
        1,
        "{list:?}"
    );
    let warning = list
        .iter()
        .find(|e| e["code"] == "import-has-open-exercises")
        .expect("import-has-open-exercises");
    assert_eq!(warning["type"], "warning");
    assert_eq!(warning["span"]["start"]["line"], 1, "{warning}");
    assert!(
        stdout(&out).contains("\"name\":\"use\""),
        "{}",
        stdout(&out)
    );

    // 人类视图：警告走 stderr（带 `warning[code]`），退出码不变。
    let human = run(&open, &["Main.sokonanoda"], None);
    assert_eq!(human.status.code(), Some(0), "stderr: {}", stderr(&human));
    let stderr_text = stderr(&human);
    assert!(
        stderr_text.contains("warning[import-has-open-exercises]"),
        "{stderr_text}"
    );
    let _ = std::fs::remove_dir_all(&rejected);
    let _ = std::fs::remove_dir_all(&open);
}

// ── `query project`：项目状态视图（0.58.0 批次 4）────────────────────────────
//
// 与 LSP `soko/project` 共用 `front::query::QueryDoc::project_view()`：这里是
// CLI 侧 e2e（真二进制），字段名与退出码是对外契约（`docs/protocol.md`）。

/// 单个 JSON 对象 + 退出码；`query project` 的包装。
fn query_project(dir: &Path) -> (serde_json::Value, i32) {
    let out = run(
        dir,
        &["query", "project", "--file", "Main.sokonanoda"],
        None,
    );
    let value: serde_json::Value = serde_json::from_str(&stdout(&out)).unwrap_or_else(|e| {
        panic!(
            "query project 必须输出单个 JSON 对象（{e}）：{}",
            stdout(&out)
        )
    });
    (value, out.status.code().unwrap_or(-1))
}

#[test]
fn query_project_describes_root_manifest_and_module_statuses() {
    let dir = tmp_dir("query-project-view");
    write(&dir, "sokonanoda.toml", "[project]\nname = \"demo\"\n");
    write(&dir, "Lib.sokonanoda", "def lib_value : Nat := 2\n");
    write(
        &dir,
        "Main.sokonanoda",
        "import Lib\n\ndef two : Nat := lib_value\n\nexample : Nat := sorry\n",
    );
    let (value, code) = query_project(&dir);
    assert_eq!(code, 0, "单文件/项目都要退出 0；收到 {value}");
    assert_eq!(value["schema"], "soko.query/1");
    assert_eq!(value["op"], "project");
    assert_eq!(value["ok"], true);
    assert_eq!(value["data"]["reason"], serde_json::Value::Null);
    let project = &value["data"]["project"];
    assert_eq!(project["entry"], "Main");
    assert!(
        project["root"]
            .as_str()
            .unwrap_or_default()
            .ends_with("query-project-view-")
            || !project["root"].as_str().unwrap_or_default().is_empty()
    );
    assert_eq!(
        project["counts"]["modules"], 2,
        "闭包两个模块（依赖在前、入口在后）：{project}"
    );
    assert_eq!(project["counts"]["compiled"], 2);
    assert_eq!(project["counts"]["failed"], 0);
    assert_eq!(project["counts"]["open_exercises"], 1);
    let modules = project["modules"].as_array().expect("modules array");
    assert_eq!(modules[0]["name"], "Lib");
    assert_eq!(modules[0]["status"], "compiled");
    assert_eq!(modules[0]["entry"], false);
    assert_eq!(modules[1]["name"], "Main");
    assert_eq!(modules[1]["entry"], true);
    assert_eq!(modules[1]["imports"], serde_json::json!(["Lib"]));
    assert!(
        modules[0]["path"]
            .as_str()
            .unwrap_or_default()
            .ends_with("Lib.sokonanoda"),
        "模块路径要能直接用来开文件：{modules:?}"
    );
    assert!(
        project["manifest"]
            .as_str()
            .unwrap_or_default()
            .ends_with("sokonanoda.toml"),
        "有清单时 manifest 指向它：{project}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn query_project_separates_the_broken_module_from_the_blocked_ones() {
    let dir = tmp_dir("query-project-blocked");
    write(&dir, "A.sokonanoda", "def a : Nat := 1\n");
    // B 自己坏（依赖的文件不存在）⇒ 根因；C 只是被拖住 ⇒ 受害者。
    write(&dir, "B.sokonanoda", "import Missing\n\ndef b : Nat := 1\n");
    write(&dir, "C.sokonanoda", "import B\n\ndef c : Nat := 1\n");
    write(
        &dir,
        "Main.sokonanoda",
        "import A\nimport B\nimport C\n\ndef main_value : Nat := a\n",
    );
    let (value, code) = query_project(&dir);
    assert_eq!(code, 0, "有失败模块也是给答案（退出码只区分用途/环境）");
    let project = &value["data"]["project"];
    let module = |name: &str| {
        project["modules"]
            .as_array()
            .expect("modules")
            .iter()
            .find(|module| module["name"] == name)
            .unwrap_or_else(|| panic!("module {name} missing: {project}"))
            .clone()
    };
    assert_eq!(module("A")["status"], "compiled");
    assert_eq!(module("B")["status"], "load-failed");
    assert!(
        module("B")["message"]
            .as_str()
            .unwrap_or_default()
            .contains("Missing"),
        "根因要说出缺哪个模块：{}",
        module("B")
    );
    assert_eq!(module("C")["status"], "blocked");
    assert!(project["counts"]["failed"].as_u64().unwrap_or(0) >= 1);
    assert!(project["counts"]["blocked"].as_u64().unwrap_or(0) >= 1);
    assert!(
        project["diagnostics"]
            .as_array()
            .expect("diagnostics")
            .iter()
            .any(|diag| diag["code"] == "import-not-found"),
        "缺模块进项目级诊断：{project}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn query_project_answers_null_with_a_reason_for_single_files() {
    let dir = tmp_dir("query-project-single");
    write(&dir, "Main.sokonanoda", "def two : Nat := 2\n");
    let (value, code) = query_project(&dir);
    assert_eq!(code, 0, "单文件是合法答案，不是错误：{value}");
    assert_eq!(value["ok"], true);
    assert_eq!(value["data"]["project"], serde_json::Value::Null);
    assert_eq!(value["data"]["reason"], "no-imports");

    // 有 `import` 但入口定位不到（stdin）：另一种原因，同样退出 0。
    let out = run(
        &dir,
        &["query", "project"],
        Some("import Lib\n\ndef two : Nat := 2\n"),
    );
    let value: serde_json::Value = serde_json::from_str(&stdout(&out)).expect("json");
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(value["data"]["project"], serde_json::Value::Null);
    assert_eq!(value["data"]["reason"], "no-path");
    let _ = std::fs::remove_dir_all(&dir);
}

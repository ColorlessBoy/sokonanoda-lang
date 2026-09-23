//! `sokonanoda query <op>` end-to-end: the agent-facing single-JSON view of the
//! kernel truth (`docs/protocol.md` §"`query` subcommand",
//! `docs/design/agent-query-channel.md` H6-A).
//!
//! The load-bearing test here is [`query_check_counts_match_the_json_event_stream`]:
//! `query check` and `--json` must agree, because they are two views of the same
//! compile — a divergence would mean a second source of truth (the exact failure
//! mode this design exists to prevent).

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn cache_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sokonanoda-query-cache-{tag}-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

/// `sokonanoda query …` with the given args, feeding `input` on stdin.
fn query(args: &[&str], input: Option<&str>) -> (serde_json::Value, i32) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_sokonanoda"))
        .arg("query")
        .args(args)
        .env("SOKONANODA_CACHE_DIR", cache_dir("q"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn sokonanoda");
    if let Some(input) = input {
        child
            .stdin
            .as_mut()
            .expect("stdin")
            .write_all(input.as_bytes())
            .expect("write stdin");
    }
    let out = child.wait_with_output().expect("wait");
    let code = out.status.code().unwrap_or(-1);
    let text = String::from_utf8_lossy(&out.stdout);
    let value: serde_json::Value = serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("query must print one JSON object ({e}): {text}"));
    (value, code)
}

/// A small canvas: one open `by` declaration with two sub-goals, one plain open
/// exercise, one checked axiom, and a hint ladder.
const CANVAS: &str = "\
axiom And : Prop -> Prop -> Prop
axiom And.intro : (a : Prop) -> (b : Prop) -> a -> b -> And a b

-- soko:hint 先拆开 And a b
-- soko:hint 再用 And.intro 装回去
theorem and_swap : (a : Prop) -> (b : Prop) -> And a b -> And b a := by
  intro a
  intro b
  intro h
  apply And.intro
  sorry
  sorry

theorem open_one (a : Prop) : And a a := sorry
";

fn canvas_file(tag: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "sokonanoda-query-{tag}-{}-{}.sokonanoda",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::write(&path, CANVAS).expect("write canvas");
    path
}

#[test]
fn query_check_counts_match_the_json_event_stream() {
    let path = canvas_file("counts");
    let (value, code) = query(&["check", "--file", path.to_str().unwrap()], None);
    assert_eq!(code, 0, "an open `sorry` is a legal state: {value}");
    assert_eq!(value["schema"], "soko.query/1");
    assert_eq!(value["op"], "check");
    assert_eq!(value["ok"], true);
    let counts = &value["data"]["counts"];

    // The `--json` event stream over the same file.
    let out = Command::new(env!("CARGO_BIN_EXE_sokonanoda"))
        .args(["--json", path.to_str().unwrap()])
        .env("SOKONANODA_CACHE_DIR", cache_dir("json"))
        .output()
        .expect("run --json");
    let stream = String::from_utf8_lossy(&out.stdout);
    let count_of = |kind: &str| {
        stream
            .lines()
            .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
            .filter(|event| event["type"] == kind)
            .count()
    };
    assert_eq!(
        counts["decl_checked"].as_u64().unwrap() as usize,
        count_of("decl.checked"),
        "query check and --json must agree on decl.checked (same compile)"
    );
    assert_eq!(
        counts["exercise_open"].as_u64().unwrap() as usize,
        count_of("exercise.open")
    );
    assert_eq!(
        counts["example_checked"].as_u64().unwrap() as usize,
        count_of("example.checked")
    );
    assert!(
        counts["exercise_open"].as_u64().unwrap() >= 2,
        "both open exercises are counted: {counts}"
    );
    assert_eq!(
        value["data"]["failed"].as_array().map(Vec::len),
        Some(0),
        "no kernel rejections in this canvas"
    );
    let _ = std::fs::remove_file(path);
}

#[test]
fn query_state_at_a_tactic_shows_the_entering_goal_state() {
    let path = canvas_file("state");
    let src = CANVAS;
    let line_of = |needle: &str| {
        src.lines()
            .position(|l| l.contains(needle))
            .map(|i| i + 1)
            .expect("needle")
    };
    // Cursor on the `apply And.intro` line → the state entering that tactic:
    // a, b, h in scope and the goal still `And b a`.
    let (value, code) = query(
        &[
            "state",
            "--file",
            path.to_str().unwrap(),
            "--line",
            &line_of("apply And.intro").to_string(),
            "--col",
            "3",
        ],
        None,
    );
    assert_eq!(code, 0);
    assert_eq!(value["ok"], true);
    assert_eq!(value["data"]["decl"]["name"], "and_swap");
    // **线 C（T-C22）之后**：`by` 步进的**展示副本**过记法折叠 ⇒ `And b a`
    // 变成 `b ∧ a`（判定输入没动，见 `query::tests` 的那条双面守护）。
    assert_eq!(value["data"]["goal"], "b ∧ a");
    let binders: Vec<&str> = value["data"]["goals"][0]["binders"]
        .as_array()
        .expect("binders")
        .iter()
        .map(|b| b["name"].as_str().unwrap())
        .collect();
    assert_eq!(binders, vec!["a", "b", "h"]);
    assert!(
        !value["data"]["goal_runs"].as_array().unwrap().is_empty(),
        "semantic runs travel with the goal text"
    );
    let _ = std::fs::remove_file(path);
}

#[test]
fn query_state_outside_a_declaration_is_a_structured_error() {
    // A comment-only file: the position is valid but answers nothing.
    let (value, code) = query(
        &[
            "state",
            "--text",
            "-- only a comment\n",
            "--line",
            "1",
            "--col",
            "1",
        ],
        None,
    );
    assert_eq!(
        code, 0,
        "a structured error answer exits 0: the JSON is the answer, `ok:false` is its verdict"
    );
    assert_eq!(value["ok"], false);
    assert_eq!(value["error"]["code"], "outside-declarations");
    assert!(
        value["error"]["message"].as_str().unwrap().contains("声明"),
        "the message explains the situation: {value}"
    );
    assert!(value.get("data").is_none(), "no data on a failure envelope");
}

#[test]
fn query_holes_lists_stable_ids_and_navigates() {
    let path = canvas_file("holes");
    let (value, code) = query(&["holes", "--file", path.to_str().unwrap()], None);
    assert_eq!(code, 0);
    let holes = value["data"]["holes"].as_array().expect("holes");
    assert_eq!(
        holes.len(),
        3,
        "two sub-goal holes + one open exercise: {holes:?}"
    );
    let ids: Vec<&str> = holes.iter().map(|h| h["id"].as_str().unwrap()).collect();
    assert_eq!(ids, vec!["and_swap:0", "and_swap:1", "open_one:0"]);
    let first = holes[0]["start"].as_u64().unwrap();
    // Stepping from the first hole lands on the *next position*, i.e. the group
    // is stepped over as one (the documented `soko/nextHole` limitation).
    let (nav, code) = query(
        &[
            "holes",
            "--file",
            path.to_str().unwrap(),
            "--offset",
            &first.to_string(),
            "--direction",
            "next",
        ],
        None,
    );
    assert_eq!(code, 0);
    assert_eq!(nav["data"]["navigated"]["id"], "open_one:0");
    let _ = std::fs::remove_file(path);
}

#[test]
fn query_hints_returns_the_authored_ladder() {
    let path = canvas_file("hints");
    let line = CANVAS
        .lines()
        .position(|l| l.contains("theorem and_swap"))
        .map(|i| i + 1)
        .expect("decl line");
    let (value, code) = query(
        &[
            "hints",
            "--file",
            path.to_str().unwrap(),
            "--line",
            &line.to_string(),
            "--col",
            "1",
        ],
        None,
    );
    assert_eq!(code, 0);
    let hints = value["data"]["hints"].as_array().expect("hints");
    assert_eq!(hints.len(), 2, "{value}");
    assert!(hints[0].as_str().unwrap().contains("拆开"));
    let _ = std::fs::remove_file(path);
}

#[test]
fn query_reduce_returns_the_kernel_normal_form() {
    let (value, code) = query(
        &[
            "reduce",
            "--text",
            "def two : Nat := 2\n",
            "--expr",
            "1 + 1",
        ],
        None,
    );
    assert_eq!(code, 0);
    assert_eq!(value["data"]["value"].as_str().unwrap().trim(), "2");
}

#[test]
fn query_goals_lists_every_declaration() {
    let path = canvas_file("goals");
    let (value, code) = query(&["goals", "--file", path.to_str().unwrap()], None);
    assert_eq!(code, 0);
    let decls = value["data"].as_array().expect("decls");
    assert_eq!(decls.len(), 4, "2 axioms + 2 theorems: {decls:?}");
    let swap = decls
        .iter()
        .find(|d| d["name"] == "and_swap")
        .expect("and_swap listed");
    assert_eq!(swap["status"], "open");
    assert_eq!(swap["kind"], "theorem");
    assert!(swap["ty"].is_string(), "the kernel-rendered signature");
    assert_eq!(
        swap["holes"].as_array().map(Vec::len),
        Some(2),
        "both spine holes are addressable"
    );
    let _ = std::fs::remove_file(path);
}

#[test]
fn query_reports_kernel_rejection_with_exit_code_one() {
    let (value, code) = query(&["check", "--text", "example : Prop := 1\n"], None);
    assert_eq!(code, 1, "kernel-rejected content exits 1: {value}");
    assert_eq!(value["ok"], true, "the query itself succeeded");
    assert!(
        !value["data"]["failed"].as_array().unwrap().is_empty(),
        "the failure is reported in the payload: {value}"
    );
}

/// G-01 / WO-004：签名不过的开放练习在**两个视图**里都是失败——
/// `query check` 的 `failed` 非空、`exercise_open` 计数为 0，且 `grade --json`
/// 的事件流与之严格一致（同一个编译出来的两条视图不许分歧）。
#[test]
fn query_check_reports_a_bad_open_exercise_signature() {
    let text = "theorem t : 3 := sorry\n";
    let (value, code) = query(&["check", "--text", text], None);
    assert_eq!(code, 1, "a bad signature exits 1: {value}");
    assert_eq!(value["ok"], true, "the query itself succeeded");
    assert_eq!(
        value["data"]["counts"]["exercise_open"], 0,
        "a rejected signature is not an open exercise: {value}"
    );
    let failed = value["data"]["failed"].as_array().unwrap();
    assert_eq!(failed.len(), 1, "{value}");
    assert_eq!(failed[0]["code"], "kernel-expected-sort", "{value}");

    // 同一份文本走 `grade --json`：事件流里没有 `exercise.open`，有一条 diagnostic。
    let path = temp_source("g01-bad-signature", text);
    let out = Command::new(env!("CARGO_BIN_EXE_sokonanoda"))
        .args(["--json", path.to_str().unwrap()])
        .env("SOKONANODA_CACHE_DIR", cache_dir("g01-json"))
        .output()
        .expect("run --json");
    let stream = String::from_utf8_lossy(&out.stdout);
    let count_of = |kind: &str| {
        stream
            .lines()
            .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
            .filter(|event| event["type"] == kind)
            .count()
    };
    assert_eq!(count_of("exercise.open"), 0, "stream: {stream}");
    assert_eq!(count_of("diagnostic"), failed.len(), "stream: {stream}");
    assert!(!out.status.success(), "grade exits non-zero: {stream}");
    let _ = std::fs::remove_file(path);
}

// ── G-10 / G-17：解析失败不许假绿（0.59.0）─────────────────────────────────
//
// 同一个病根的两个出口：`front::query::QueryDoc.parse_error` 曾经**只写不读**。
// `check` 因此把"解析不了"答成"全零 + 无失败"（G-10，退出码 0），`goals`/`holes`
// 答成空数组 + `ok:true`（G-17）——agent 的主判卷通道全部假绿，而同一份文本走
// `grade` 是对的。判据是"两条通道同口径"，不是文案。

/// 坏文本（`docs/gaps/repro/G10-query-check-parse-error.sh` 的最小复现）：
/// `grade --json` 在 offset 19 报 `unexpected-token`。
///
/// 夹具原本是 `infix:50 " e " => mem`——0.59.0 起那是一條**有意义**的
/// `notation-shape` 诊断（记法符号不能是标识符词，G-04 / WO-011），不再是通用
/// parse 错误，所以换成括号不配对（这条测试要的是"解析不了"这件事本身）。
const UNPARSABLE: &str = "def p : Prop := (a\n";

/// 把 `text` 写进一个唯一命名的临时文件（`--file` 通道）。
fn temp_source(tag: &str, text: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "sokonanoda-query-{tag}-{}-{}.sokonanoda",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::write(&path, text).expect("write source");
    path
}

/// 删掉源码里**第一个出现在代码行**（非 `--` 注释行）的 `:=`。
///
/// 比"删一个右括号"更不容易误伤注释：单元⑤ 的文件头注释里就写着 `(a,b) := …`。
fn remove_first_code_assign(src: &str) -> String {
    let mut offset = 0;
    for line in src.split_inclusive('\n') {
        if !line.trim_start().starts_with("--") {
            if let Some(col) = line.find(":=") {
                return format!("{}{}", &src[..offset + col], &src[offset + col + 2..]);
            }
        }
        offset += line.len();
    }
    panic!("no `:=` on a code line");
}

/// G-10 主验收：解析失败的源文本必须作为 parse 诊断出现在 `check` 负载里，退出码
/// **1**（与同一份文本的 `grade` 同口径）。`--text` 与 `--file` 两条输入通道各来
/// 一次：它们合流到同一个 `set_text`，假绿时也是一起假绿。
#[test]
fn query_check_reports_parse_errors_with_exit_one() {
    let path = temp_source("parse", UNPARSABLE);
    let channels: [(&str, Vec<&str>); 2] = [
        ("--text", vec!["check", "--text", UNPARSABLE]),
        ("--file", vec!["check", "--file", path.to_str().unwrap()]),
    ];
    for (channel, args) in channels {
        let (value, code) = query(&args, None);
        assert_eq!(
            code, 1,
            "{channel}: a parse failure is a rejection, not an empty file: {value}"
        );
        assert_eq!(
            value["ok"], true,
            "{channel}: the query itself was answered (`ok` = 问出来了)"
        );
        let failed = value["data"]["failed"].as_array().expect("failed[]");
        assert_eq!(
            failed.len(),
            1,
            "{channel}: exactly the parse diagnostic: {value}"
        );
        assert_eq!(failed[0]["code"], "unexpected-token", "{channel}: {value}");
        assert_eq!(
            failed[0]["name"],
            serde_json::Value::Null,
            "{channel}: no declaration name is trustworthy when the text does not parse"
        );
        assert_eq!(
            failed[0]["start"], 19,
            "{channel}: the same byte offset as `grade --json`"
        );
        assert_eq!(failed[0]["end"], 19, "{channel}");
        let counts = value["data"]["counts"].as_object().expect("counts");
        assert!(
            counts.values().all(|v| v.as_u64() == Some(0)),
            "{channel}: nothing was checked — the counts stay honestly zero: {counts:?}"
        );
    }
    // 同一份文本的另一个视图：`grade` 的 parse 诊断带同样的 code 与位置，且同样
    // 以非零退出码结束（这就是 G-10 之前两条通道不一致的反面）。
    let out = Command::new(env!("CARGO_BIN_EXE_sokonanoda"))
        .args(["grade", path.to_str().unwrap()])
        .env("SOKONANODA_CACHE_DIR", cache_dir("grade-parse"))
        .output()
        .expect("run grade");
    assert_eq!(out.status.code(), Some(1), "grade rejects it too");
    let last = String::from_utf8_lossy(&out.stdout)
        .lines()
        .last()
        .map(str::to_string)
        .unwrap_or_default();
    let diag: serde_json::Value =
        serde_json::from_str(&last).expect("the last event is the parse diagnostic");
    assert_eq!(diag["code"], "unexpected-token");
    assert_eq!(diag["span"]["start"]["offset"], 19);
    assert_eq!(diag["span"]["end"]["offset"], 19);
    let _ = std::fs::remove_file(path);
}

/// G-17：同族的另外两个出口。`goals`/`holes` 在解析失败时曾经答空数组 +
/// `ok:true`（读起来就是"这份画布没有声明 / 没有洞"）——现在必须与 `check` 一样
/// 承认"问不出来"：`ok:false` + `error.code = not-parsable` + 退出码 1。
#[test]
fn query_goals_and_holes_report_parse_errors_instead_of_empty_answers() {
    let (goals, code) = query(&["goals", "--text", UNPARSABLE], None);
    assert_eq!(code, 1, "{goals}");
    assert_eq!(goals["ok"], false, "{goals}");
    assert_eq!(goals["error"]["code"], "not-parsable", "{goals}");
    assert!(
        goals.get("data").is_none(),
        "no fake empty declaration list: {goals}"
    );
    // 导航形态（`--offset --direction`）走同一个出口。
    let (holes, code) = query(
        &[
            "holes",
            "--text",
            UNPARSABLE,
            "--offset",
            "0",
            "--direction",
            "next",
        ],
        None,
    );
    assert_eq!(code, 1, "{holes}");
    assert_eq!(holes["ok"], false, "{holes}");
    assert_eq!(holes["error"]["code"], "not-parsable", "{holes}");
    assert!(
        holes.get("data").is_none(),
        "no fake `holes: []` + `navigated: null`: {holes}"
    );
    // 对照组：能解析的空画布仍是 `ok:true` + 空数组——"正常的没有"不是错误
    // （协议把两者严格分开，`docs/design/agent-query-channel.md` §4.1）。
    let (empty, code) = query(&["goals", "--text", "-- 只有注释\n"], None);
    assert_eq!(code, 0, "{empty}");
    assert_eq!(empty["ok"], true, "{empty}");
    assert_eq!(empty["data"].as_array().map(Vec::len), Some(0), "{empty}");
    let (empty, code) = query(&["holes", "--text", "-- 只有注释\n"], None);
    assert_eq!(code, 0, "{empty}");
    assert_eq!(
        empty["data"]["holes"].as_array().map(Vec::len),
        Some(0),
        "{empty}"
    );
    assert!(empty["data"]["navigated"].is_null(), "{empty}");
}

/// G-10 的项目路径回归：坏的是**被 import 的依赖**（入口自己解析得了）时，
/// `query check --file <入口>` 仍报 `import-dependency-failed` + 退出码 1。
/// 这条路径在修 G-10 之前就是对的（归因机制见 `docs/architecture.md`），改动不许
/// 把它弄坏；连跑两次顺手钉住"带诊断的项目永远不进缓存"（`store_if_clean`）。
///
/// G-15 / WO-010 补充：`failed[]` 里**只有**入口自己的那条诊断，坐标空间 = 入口
/// 文件（依赖的病由 `grade --json` 的 `file`/`module` 字段或 `query project` 承担）。
#[test]
fn query_check_still_reports_a_broken_dependency_in_a_project() {
    let dir = std::env::temp_dir().join(format!(
        "sokonanoda-query-bad-dep-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp project");
    std::fs::write(dir.join("Bad.sokonanoda"), "def broken : Nat := )\n").expect("write dep");
    let entry = dir.join("Main.sokonanoda");
    std::fs::write(&entry, "import Bad\n\ndef use : Nat := broken\n").expect("write entry");
    for round in 0..2 {
        let (value, code) = query(&["check", "--file", entry.to_str().unwrap()], None);
        assert_eq!(
            code, 1,
            "round {round}: a broken dependency is a rejection: {value}"
        );
        assert_eq!(value["ok"], true, "round {round}: {value}");
        let failed = value["data"]["failed"].as_array().expect("failed[]");
        assert_eq!(
            failed.len(),
            1,
            "round {round}: only the entry's own diagnostic — the dependency's parse error \
             stays in the dependency's coordinate space: {value}"
        );
        assert_eq!(
            failed[0]["code"], "import-dependency-failed",
            "round {round}: the entry blames the module that failed: {value}"
        );
        let name = entry.to_str().unwrap();
        assert_eq!(
            failed[0]["start_line"], 1,
            "round {round}: {name} line 1 is the import: {value}"
        );
        assert_eq!(failed[0]["start_col"], 1, "round {round}: {value}");
        assert_eq!(failed[0]["end_line"], 1, "round {round}: {value}");
        assert_eq!(failed[0]["end_col"], 11, "round {round}: {value}");
        assert_eq!(failed[0]["start"], 0, "round {round}: byte offset: {value}");
        assert_eq!(failed[0]["end"], 10, "round {round}: byte offset: {value}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// 课程用例（卷 I 单元⑤）：正例保持基线（5 checked · 7 open · 0 failed · exit 0，
/// `sorry` 是合法状态）；反例——同一份源码删掉代码里的第一个 `:=`——必须与
/// `grade` 同口径：同一个 parse code、同一个 span、同样退出 1。
#[test]
fn query_check_matches_grade_on_a_real_course_unit() {
    let unit = PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../courses/set-theory/units/unit05-pairs-products.sokonanoda"
    ));
    let (value, code) = query(&["check", "--file", unit.to_str().unwrap()], None);
    assert_eq!(
        code, 0,
        "an open `sorry` exercise is a legal state: {value}"
    );
    assert_eq!(value["data"]["counts"]["decl_checked"], 5, "{value}");
    assert_eq!(value["data"]["counts"]["exercise_open"], 7, "{value}");
    assert_eq!(
        value["data"]["failed"].as_array().map(Vec::len),
        Some(0),
        "{value}"
    );

    let src = std::fs::read_to_string(&unit).expect("read unit05");
    let broken = remove_first_code_assign(&src);
    assert_ne!(broken, src, "the mutation must actually fire");
    let broken_file = temp_source("unit05-broken", &broken);

    let (query_value, query_code) =
        query(&["check", "--file", broken_file.to_str().unwrap()], None);
    assert_eq!(query_code, 1, "the broken copy is rejected: {query_value}");
    let q = &query_value["data"]["failed"][0];

    let out = Command::new(env!("CARGO_BIN_EXE_sokonanoda"))
        .args(["grade", broken_file.to_str().unwrap()])
        .env("SOKONANODA_CACHE_DIR", cache_dir("grade-unit05"))
        .output()
        .expect("run grade");
    assert_eq!(out.status.code(), Some(1), "grade agrees on the exit code");
    let last = String::from_utf8_lossy(&out.stdout)
        .lines()
        .last()
        .map(str::to_string)
        .unwrap_or_default();
    let diag: serde_json::Value = serde_json::from_str(&last).expect("the parse diagnostic");
    assert_eq!(
        q["code"], diag["code"],
        "the two channels must agree on the code\nquery: {query_value}\ngrade: {diag}"
    );
    assert_eq!(q["code"], "unexpected-token", "{query_value}");
    assert_eq!(
        q["start"], diag["span"]["start"]["offset"],
        "and on the span start\nquery: {query_value}\ngrade: {diag}"
    );
    assert_eq!(
        q["end"], diag["span"]["end"]["offset"],
        "and on the span end\nquery: {query_value}\ngrade: {diag}"
    );
    let _ = std::fs::remove_file(broken_file);
}

/// G-15 / WO-010：`failed[]` / `warnings[]` 必须**自带单位**——`start`/`end` 仍是
/// **字节** offset（坐标空间 = 入口文件），另加 1 基 `start_line`/`start_col`/
/// `end_line`/`end_col`，且与 `grade --json` 同一诊断的 `span` 逐字段一致。
///
/// 夹具故意在前面放一行**中文注释**（多字节）：字节 offset ≠ 字符下标，正是
/// G-15 假缺口的来源（旧量具拿 `src[:offset]` 当字符切片）。坏声明在**中间**，
/// 所以漂到 `good`/`after` 上就会红。
#[test]
fn query_check_failure_positions_are_typed_and_match_grade() {
    const DEP: &str = "\
axiom P : Prop

example : P := sorry
";
    const ENTRY: &str = "\
-- 中文注释：多字节字符让字节 offset 与字符下标不再相等
import Lib

def good : Prop -> Prop := fun (p : Prop) => p

def bad : Prop -> Type := fun (x : Prop) => x

def after : Prop -> Prop := fun (p : Prop) => p
";
    const BAD: &str = "def bad : Prop -> Type := fun (x : Prop) => x";
    let dir = std::env::temp_dir().join(format!(
        "sokonanoda-query-g15-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp project");
    std::fs::write(dir.join("Lib.sokonanoda"), DEP).expect("write dep");
    let entry = dir.join("Main.sokonanoda");
    std::fs::write(&entry, ENTRY).expect("write entry");

    let (value, code) = query(&["check", "--file", entry.to_str().unwrap()], None);
    assert_eq!(code, 1, "the bad declaration is a rejection: {value}");
    assert_eq!(value["ok"], true, "{value}");

    // 量具（测试自己）：按**字节**找坏声明并数行——不许用字符下标。
    let start = ENTRY.find(BAD).expect("fixture contains the bad command");
    let end = start + BAD.len();
    let start_line = ENTRY.as_bytes()[..start]
        .iter()
        .filter(|b| **b == b'\n')
        .count()
        + 1;
    assert_eq!(start_line, 6, "the bad command is on line 6 of the entry");
    assert!(
        ENTRY[..start].chars().count() < start,
        "the fixture must keep bytes ≠ chars before the failure (that is the G-15 trap)"
    );

    let failed = &value["data"]["failed"][0];
    assert_eq!(failed["code"], "kernel-rejected", "{value}");
    assert_eq!(failed["start"], start, "start is a BYTE offset: {value}");
    assert_eq!(failed["end"], end, "end is a BYTE offset: {value}");
    assert_eq!(failed["start_line"], 6, "{value}");
    assert_eq!(failed["start_col"], 1, "{value}");
    assert_eq!(failed["end_line"], 6, "no overflow onto `after`: {value}");
    assert_eq!(failed["end_col"], 46, "{value}");

    // 同一份文本的另一个视图：`grade --json` 的诊断事件。行列必须一致。
    let out = Command::new(env!("CARGO_BIN_EXE_sokonanoda"))
        .args(["--json", entry.to_str().unwrap()])
        .env("SOKONANODA_CACHE_DIR", cache_dir("g15-json"))
        .output()
        .expect("run --json");
    assert_eq!(out.status.code(), Some(1), "grade rejects it too");
    let stream = String::from_utf8_lossy(&out.stdout);
    let events: Vec<serde_json::Value> = stream
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();
    let diag = events
        .iter()
        .find(|e| e["type"] == "diagnostic" && e["code"] == "kernel-rejected")
        .unwrap_or_else(|| panic!("kernel diagnostic in: {stream}"));
    assert_eq!(diag["span"]["start"]["offset"], start, "{stream}");
    assert_eq!(diag["span"]["end"]["offset"], end, "{stream}");
    assert_eq!(
        failed["start_line"], diag["span"]["start"]["line"],
        "{value}"
    );
    assert_eq!(
        failed["start_col"], diag["span"]["start"]["column"],
        "{value}"
    );
    assert_eq!(failed["end_line"], diag["span"]["end"]["line"], "{value}");
    assert_eq!(failed["end_col"], diag["span"]["end"]["column"], "{value}");

    // 字节切出来的就是出错的那条命令（`start`/`end` 的语义没被动过）。
    assert_eq!(&ENTRY[start..end], BAD);

    // `warnings[]` 同样带行列（依赖里的开放练习在**入口**的 import 行上报）。
    let warning = &value["data"]["warnings"][0];
    assert_eq!(warning["code"], "import-has-open-exercises", "{value}");
    assert_eq!(warning["start_line"], 2, "the import line: {value}");
    assert_eq!(warning["start_col"], 1, "{value}");
    assert_eq!(warning["end_line"], 2, "{value}");
    assert_eq!(warning["end_col"], 11, "{value}");
    let wdiag = events
        .iter()
        .find(|e| e["type"] == "warning")
        .unwrap_or_else(|| panic!("warning event in: {stream}"));
    assert_eq!(
        warning["start"], wdiag["span"]["start"]["offset"],
        "{value}"
    );
    assert_eq!(warning["end"], wdiag["span"]["end"]["offset"], "{value}");
    assert_eq!(
        warning["start_line"], wdiag["span"]["start"]["line"],
        "{value}"
    );
    assert_eq!(
        warning["end_col"], wdiag["span"]["end"]["column"],
        "{value}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn query_usage_errors_exit_two_without_a_payload() {
    let out = Command::new(env!("CARGO_BIN_EXE_sokonanoda"))
        .args(["query", "bogus-op"])
        .env("SOKONANODA_CACHE_DIR", cache_dir("bad"))
        .output()
        .expect("run");
    assert_eq!(out.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("未知的 query op"),
        "the usage error explains itself"
    );
    // `--line` without `--col` is a usage-shaped answer (structured, exit 0).
    let (value, code) = query(
        &[
            "state",
            "--text",
            "theorem t : Prop := sorry\n",
            "--line",
            "1",
        ],
        None,
    );
    assert_eq!(code, 0);
    assert_eq!(value["ok"], false);
    assert_eq!(value["error"]["code"], "position-out-of-range");
}

#[test]
fn query_reads_source_from_stdin_and_accepts_compact() {
    let (value, code) = query(&["check", "--compact"], Some("axiom P : Prop\n"));
    assert_eq!(code, 0);
    assert_eq!(value["data"]["counts"]["decl_checked"], 1);
    // `--compact` prints exactly one line.
    let mut child = Command::new(env!("CARGO_BIN_EXE_sokonanoda"))
        .args(["query", "check", "--compact"])
        .env("SOKONANODA_CACHE_DIR", cache_dir("compact"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn");
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"axiom P : Prop\n")
        .unwrap();
    let out = child.wait_with_output().unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    assert_eq!(text.trim().lines().count(), 1, "compact = one JSON line");
}

// ── CLI ≡ LSP 一致性契约（设计文档 A4：防"两套真相"）────────────────────────
//
// `query state` 与 `soko/stateAt` 必须给出**同一份真相**：同一个位置、同样的
// step/goal/binders。它们由 `front::query::select_state_at` 唯一实现，但这个测试
// 存在的意义是——**上一次它们真的是两套**（真相层把"光标恰在某 tactic 末尾"
// 判成"之内"，且根状态带了走查后的剩余目标），而当时没有任何测试会红。
//
// 走真 LSP over stdio（不是进程内 rpc），因为契约的另一半是 wire 形状。

/// `sokonanoda-lsp` 与当前测试二进制同目录（`target/<profile>/`）。
fn lsp_binary() -> PathBuf {
    let mut dir = std::env::current_exe().expect("current exe");
    dir.pop(); // deps/
    dir.pop(); // <profile>/
    let name = if cfg!(windows) {
        "sokonanoda-lsp.exe"
    } else {
        "sokonanoda-lsp"
    };
    dir.join(name)
}

/// 一个极小的 LSP 客户端：发 `initialize` + `didOpen`，然后问一个自定义请求。
fn lsp_request(
    uri: &str,
    text: &str,
    method: &str,
    params: serde_json::Value,
) -> serde_json::Value {
    use std::io::{BufRead, BufReader, Write};
    let binary = lsp_binary();
    if !binary.exists() {
        // 只跑 `cargo test -p sokonanoda-cli` 时 LSP 二进制可能没编；不静默跳过，
        // 但也别让无关的测试套件红——打印一条明确的提示。
        eprintln!(
            "skipping CLI≡LSP consistency check: {} not built (run `cargo build --workspace`)",
            binary.display()
        );
        return serde_json::Value::Null;
    }
    let mut child = Command::new(&binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn sokonanoda-lsp");
    let mut stdin = child.stdin.take().expect("lsp stdin");
    let stdout = child.stdout.take().expect("lsp stdout");
    let mut reader = BufReader::new(stdout);

    let send = |stdin: &mut std::process::ChildStdin, message: serde_json::Value| {
        let body = serde_json::to_string(&message).expect("serialize");
        write!(stdin, "Content-Length: {}\r\n\r\n{body}", body.len()).expect("write frame");
        stdin.flush().expect("flush");
    };
    let read = |reader: &mut BufReader<std::process::ChildStdout>| -> serde_json::Value {
        let mut length = 0usize;
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line).expect("read header") == 0 {
                panic!("LSP closed the stream before answering");
            }
            if let Some(value) = line.strip_prefix("Content-Length:") {
                length = value.trim().parse().expect("content length");
            }
            if line == "\r\n" || line == "\n" {
                break;
            }
        }
        let mut body = vec![0u8; length];
        std::io::Read::read_exact(reader, &mut body).expect("read body");
        serde_json::from_slice(&body).expect("parse json")
    };

    send(
        &mut stdin,
        serde_json::json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{
            "processId": null,
            "rootUri": null,
            "capabilities": {}
        }}),
    );
    let _ = read(&mut reader); // initialize result
    send(
        &mut stdin,
        serde_json::json!({"jsonrpc":"2.0","method":"initialized","params":{}}),
    );
    send(
        &mut stdin,
        serde_json::json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{
            "textDocument": {"uri": uri, "languageId": "sokonanoda", "version": 1, "text": text}
        }}),
    );
    // **等这次 didOpen 的诊断**（T-A30）：编译现在在服务端的另一个任务里跑，
    // `didOpen` 返回时文档还没编好——不等就会问到一个空文档，而这里比对的是
    // "CLI 与 LSP 答同一件事"。
    loop {
        let message = read(&mut reader);
        if message.get("method").and_then(|v| v.as_str()) == Some("textDocument/publishDiagnostics")
        {
            break;
        }
    }
    send(
        &mut stdin,
        serde_json::json!({"jsonrpc":"2.0","id":2,"method":method,"params":params}),
    );
    // 跳过诊断通知等，直到拿到 id == 2 的应答。
    let answer = loop {
        let message = read(&mut reader);
        if message.get("id").and_then(|v| v.as_i64()) == Some(2) {
            break message;
        }
        if message.get("id").is_some() {
            panic!("unexpected LSP response: {message}");
        }
    };
    drop(stdin);
    let _ = child.wait();
    answer
}

/// 与 `query state` 的同一位置比对：CLI 用 1-based（`--line/--col`，UTF-16 列），
/// LSP 用 0-based（`line`/`character`）。
fn assert_state_matches_lsp(canvas: &str, line_1based: usize, col_1based: usize) {
    let (cli, code) = query(
        &[
            "state",
            "--text",
            canvas,
            "--line",
            &line_1based.to_string(),
            "--col",
            &col_1based.to_string(),
        ],
        None,
    );
    assert_eq!(code, 0);
    assert_eq!(cli["ok"], true, "{cli}");

    let lsp = lsp_request(
        "file:///consistency.sokonanoda",
        canvas,
        "soko/stateAt",
        serde_json::json!({
            "textDocument": {"uri": "file:///consistency.sokonanoda"},
            "position": {"line": line_1based - 1, "character": col_1based - 1},
        }),
    );
    if lsp.is_null() {
        return; // 二进制未编，已打印提示
    }
    let result = &lsp["result"];
    let cli_data = &cli["data"];

    assert_eq!(
        cli_data["step"], result["step"],
        "step must agree between `query state` and `soko/stateAt`\nCLI: {cli_data}\nLSP: {result}"
    );
    assert_eq!(
        cli_data["total"], result["total"],
        "total must agree\nCLI: {cli_data}\nLSP: {result}"
    );
    assert_eq!(
        cli_data["goal"], result["goal"],
        "the goal text must agree (same kernel rendering)\nCLI: {cli_data}\nLSP: {result}"
    );
    let cli_binders: Vec<&str> = cli_data["binders"]
        .as_array()
        .expect("cli binders")
        .iter()
        .map(|b| b["name"].as_str().unwrap())
        .collect();
    let lsp_binders: Vec<&str> = result["binders"]
        .as_array()
        .expect("lsp binders")
        .iter()
        .map(|b| b["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        cli_binders, lsp_binders,
        "the hypothesis list must agree\nCLI: {cli_data}\nLSP: {result}"
    );
    // 多目标列表：长度与每项的目标文本都要对上（`apply` 之后两个子目标）。
    let cli_goals: Vec<&str> = cli_data["goals"]
        .as_array()
        .expect("cli goals")
        .iter()
        .map(|g| g["goal"].as_str().unwrap())
        .collect();
    let lsp_goals: Vec<&str> = result["goals"]
        .as_array()
        .expect("lsp goals")
        .iter()
        .map(|g| g["goal"].as_str().unwrap())
        .collect();
    assert_eq!(
        cli_goals, lsp_goals,
        "the full goal list must agree\nCLI: {cli_data}\nLSP: {result}"
    );
}

/// 「多余的 `sorry`」的洞级标记（`docs/design/redundant-sorry.md` §5）：
/// `query goals`/`query holes` 与 LSP 的 `soko/goals` 必须给出同一个
/// `redundant: true`——agent 侧据此说"删掉这一行"，而不是"你还没证出来"。
/// 真缺口是同一条 op 上的对照组（标记为 `false`）。
#[test]
fn query_and_the_lsp_agree_on_the_redundant_mark() {
    for (canvas, expected) in [(REDUNDANT_CANVAS, true), (GENUINE_CANVAS, false)] {
        let (value, code) = query(&["goals", "--text", canvas], None);
        assert_eq!(code, 0, "{value}");
        let cli_decl = value["data"]
            .as_array()
            .expect("cli decls")
            .iter()
            .find(|d| d["name"] == "t")
            .expect("decl t");
        let cli_holes: Vec<(String, bool)> = cli_decl["holes"]
            .as_array()
            .expect("cli holes")
            .iter()
            .map(|h| {
                (
                    h["id"].as_str().unwrap().to_string(),
                    h["redundant"].as_bool().unwrap_or(false),
                )
            })
            .collect();
        assert_eq!(
            cli_holes,
            vec![("t:0".to_string(), expected)],
            "query goals must mark the hole: {value}"
        );

        let lsp = lsp_request(
            "file:///redundant.sokonanoda",
            canvas,
            "soko/goals",
            serde_json::json!({
                "textDocument": {"uri": "file:///redundant.sokonanoda"},
            }),
        );
        if lsp.is_null() {
            return; // 二进制未编，已打印提示
        }
        let lsp_decl = lsp["result"]["decls"]
            .as_array()
            .expect("lsp decls")
            .iter()
            .find(|d| d["name"] == "t")
            .expect("decl t");
        let lsp_holes: Vec<(String, bool)> = lsp_decl["holes"]
            .as_array()
            .expect("lsp holes")
            .iter()
            .map(|h| {
                (
                    h["id"].as_str().unwrap().to_string(),
                    h["redundant"].as_bool().unwrap_or(false),
                )
            })
            .collect();
        assert_eq!(
            cli_holes, lsp_holes,
            "`query goals` and `soko/goals` must agree on the mark\nCLI: {value}\nLSP: {lsp}"
        );

        // `query holes` 是同一个洞的另一种问法（共用真相层）。
        let (holes, code) = query(&["holes", "--text", canvas], None);
        assert_eq!(code, 0, "{holes}");
        assert_eq!(
            holes["data"]["holes"][0]["redundant"], expected,
            "query holes must agree too: {holes}"
        );
    }
}

/// 「多余的 `sorry`」（答案写全、只多留一行）与真缺口（`f` 缺一个实参）——
/// 洞级标记的正反两个样本。
const REDUNDANT_CANVAS: &str = "\
axiom A : Prop
axiom B : Prop
axiom f : A -> B
theorem t (h : A) : B := f h
 sorry
";

const GENUINE_CANVAS: &str = "\
axiom A : Prop
axiom B : Prop
axiom f : A -> B
theorem t : B := f
 sorry
";

#[test]
fn query_state_agrees_with_the_lsp_state_at_request() {
    // 三个位置覆盖协议的三条分支：根状态、某 tactic 之内（进入它之前）、
    // 以及"最后一条 tactic 之后"。
    let decl_line = CANVAS
        .lines()
        .position(|l| l.contains("theorem and_swap"))
        .map(|i| i + 1)
        .expect("decl line");
    assert_state_matches_lsp(CANVAS, decl_line, 1); // 根状态（step: -1）
    assert_state_matches_lsp(CANVAS, decl_line + 4, 3); // `apply And.intro` 之内
    assert_state_matches_lsp(CANVAS, decl_line + 6, 3); // 最后一个 sorry（apply 之后）
}

/// **没有 `by` 块**的两个分支。协议（`docs/protocol.md` §`soko/stateAt`）规定
/// 这两条都 `step: -1`、`total: 0`：半成品退回声明自己的剩余目标/上下文；已闭合
/// 的声明 `goals: []`（wire `goal: null`）。
///
/// 曾经真出过事：真相层把这两个分支当成"根状态"（目标 = 声明类型、binders 为空），
/// 而 LSP 的既有实现是对的——一次"看起来等价"的重构把 Infoview 的半成品上下文
/// 抹掉了、并给已证的声明安上一个假目标。**LSP 套件里没有任何用例覆盖它们**，
/// 所以只有这条端到端一致性测试能挡住（H6-A 的 A4 契约）。
const NO_BY_CANVAS: &str = "\
axiom And : Prop -> Prop -> Prop
axiom And.intro : (a : Prop) -> (b : Prop) -> a -> b -> And a b

example : (a : Prop) -> a -> a := fun (a : Prop) => fun (h : a) => sorry

theorem closed_identity (a : Prop) : a -> a := fun (x : a) => x
";

#[test]
fn query_state_and_lsp_agree_without_a_by_block() {
    // 半成品（lambda 前缀 + `sorry`）：上下文必须还在。
    let (value, code) = query(
        &["state", "--text", NO_BY_CANVAS, "--line", "4", "--col", "1"],
        None,
    );
    assert_eq!(code, 0, "{value}");
    assert_eq!(value["data"]["step"], -1);
    assert_eq!(
        value["data"]["total"], 0,
        "no tactics → no per-tactic states"
    );
    assert_eq!(value["data"]["goal"], "a", "the remaining goal: {value}");
    let binders: Vec<&str> = value["data"]["binders"]
        .as_array()
        .expect("binders")
        .iter()
        .map(|b| b["name"].as_str().unwrap())
        .collect();
    assert_eq!(binders, vec!["a", "h"], "the half-written proof's context");

    // 已闭合、无 `by` 块：没有目标（不是"目标 = 声明类型"）。
    let (value, code) = query(
        &["state", "--text", NO_BY_CANVAS, "--line", "6", "--col", "1"],
        None,
    );
    assert_eq!(code, 0, "{value}");
    assert_eq!(value["data"]["step"], -1);
    assert_eq!(value["data"]["total"], 0);
    assert_eq!(
        value["data"]["goals"].as_array().map(Vec::len),
        Some(0),
        "a closed proof has no goal: {value}"
    );
    assert!(value["data"]["goal"].is_null(), "{value}");

    // 两侧逐字段一致（同一份真相的两个视图）。
    assert_state_matches_lsp(NO_BY_CANVAS, 4, 1);
    assert_state_matches_lsp(NO_BY_CANVAS, 6, 1);
}

/// **G-22 的 CLI 面**：项目入口的 `query goals` 必须非空——**冷缓存**下也一样。
///
/// 为什么单列一条：CLI 曾经**看不见**这个缺口——`query` 走 `set_cached_entry`
/// （`crates/front/src/query/mod.rs:214` 把 `parse_error` 清掉），于是"缓存热时
/// 偶然正常"；而 `build`/`check` 走 `store_if_clean` 是另一套规则。同一个项目的
/// 答案不该取决于**先跑了哪条命令**（计划 T-B07）。
///
/// 所以这里用**全新的缓存目录**、并且只跑 `query goals` 一次——任何"靠先跑别的
/// 命令把状态捂热"的路径都盖不住它。
/// **T-C31 的判据**：目标文本里的**导入名**要有正确 `kind`，不能是
/// `unknown_ident`。
///
/// `decl_kinds()` 以前只看**入口文件** ⇒ 项目文件里 `Set`/`Set.mem` 全被标成
/// `unknown_ident`（线 C 之后 goal 里全是这些名字，一眼就看得出来）。
/// 现在闭包级声明表在**编译期**算一次、并进 runs 计算。
///
/// 同一条判据也覆盖 **T-C30**：`∈` 是记法符号，要有 `keyword`（不是裸 run）。
#[test]
fn query_goals_classifies_imported_names_and_notation_symbols() {
    let dir = std::env::temp_dir().join(format!(
        "sokonanoda-query-tc31-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp project");
    std::fs::write(
        dir.join("SetLib.sokonanoda"),
        "def Set (α : Type) : Type := α -> Prop\n\
def Set.mem (α : Type) (a : α) (A : Set α) : Prop := A a\n\
infix:50 \" ∈ \" => Set.mem\n",
    )
    .expect("write lib");
    let entry = dir.join("Canvas.sokonanoda");
    std::fs::write(
        &entry,
        "import SetLib\n\n\
theorem mem_self (α : Type) (a : α) (A : Set α) (h : a ∈ A) : a ∈ A := h\n",
    )
    .expect("write entry");

    let cache = cache_dir("tc31");
    let output = Command::new(env!("CARGO_BIN_EXE_sokonanoda"))
        .args([
            "query",
            "goals",
            "--file",
            entry.to_str().unwrap(),
            "--compact",
        ])
        .env("SOKONANODA_CACHE_DIR", &cache)
        .output()
        .expect("spawn sokonanoda");
    assert_eq!(output.status.code().unwrap_or(-1), 0);
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).unwrap_or(serde_json::Value::Null);
    let decl = value["data"]
        .as_array()
        .expect("data[]")
        .iter()
        .find(|d| d["name"] == "mem_self")
        .expect("mem_self listed");
    let runs = decl["ty_runs"].as_array().expect("ty_runs");

    // 导入名（`Set` 来自 `SetLib`）必须有 kind。
    let set_kind = runs
        .iter()
        .find(|r| r["text"] == "Set")
        .and_then(|r| r["kind"].as_str())
        .unwrap_or("<无>");
    assert_eq!(set_kind, "def_use", "导入名要有正确 kind：{runs:?}");

    // 记法符号（T-C30）：`∈` 是 `keyword`，不是裸 run。
    let mem_kind = runs
        .iter()
        .find(|r| r["text"] == "∈")
        .and_then(|r| r["kind"].as_str())
        .unwrap_or("<无>");
    assert_eq!(mem_kind, "keyword", "记法符号要有 kind：{runs:?}");

    // 反向：**不该再有** `unknown_ident` 出现在这个签名里（`α`/`a`/`A`/`h`
    // 是签名自己的 binder，见下面的"已知剩余"）。
    let unknown: Vec<&str> = runs
        .iter()
        .filter(|r| r["kind"] == "unknown_ident")
        .filter_map(|r| r["text"].as_str())
        .collect();
    assert!(
        !unknown.contains(&"Set") && !unknown.contains(&"Set.mem"),
        "导入名不该是 unknown_ident：{unknown:?}"
    );
}

#[test]
fn query_goals_lists_a_project_entry_with_a_cold_cache() {
    let dir = std::env::temp_dir().join(format!(
        "sokonanoda-query-goals-cold-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp project");
    // 与课程单元同形状：记法来自 import ⇒ 入口单独 parse 必然失败、闭包是好的。
    std::fs::write(
        dir.join("SetLib.sokonanoda"),
        "def Set (α : Type) : Type := α -> Prop\n\
def Set.mem (α : Type) (a : α) (A : Set α) : Prop := A a\n\
infix:50 \" ∈ \" => Set.mem\n",
    )
    .expect("write lib");
    let entry = dir.join("Canvas.sokonanoda");
    std::fs::write(
        &entry,
        "import SetLib\n\n\
theorem mem_self (α : Type) (a : α) (A : Set α) (h : a ∈ A) : a ∈ A := h\n",
    )
    .expect("write entry");

    // **自己起进程**：`query()` 这个 helper 的第二个参数是 stdin，而且缓存目录
    // 写死成共享的 `cache_dir("q")`（同一轮测试里别的用例会把它捂热）——
    // 那样就测不到"冷缓存"这条前提。这里用全新的、本次独有的缓存目录。
    let cache = cache_dir("g22-cold");
    let output = Command::new(env!("CARGO_BIN_EXE_sokonanoda"))
        .args([
            "query",
            "goals",
            "--file",
            entry.to_str().unwrap(),
            "--compact",
        ])
        .env("SOKONANODA_CACHE_DIR", &cache)
        .output()
        .expect("spawn sokonanoda");
    let code = output.status.code().unwrap_or(-1);
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).unwrap_or(serde_json::Value::Null);
    assert_eq!(code, 0, "闭包好 ⇒ query goals 必须答得上：{value}");
    assert_eq!(value["ok"], true, "{value}");
    let data = value["data"].as_array().expect("data[]");
    assert!(
        data.iter().any(|d| d["name"] == "mem_self"),
        "项目入口的声明列表必须含 mem_self（G-22），实际 = {value}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

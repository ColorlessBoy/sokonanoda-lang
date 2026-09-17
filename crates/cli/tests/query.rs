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
    assert_eq!(value["data"]["goal"], "And b a");
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

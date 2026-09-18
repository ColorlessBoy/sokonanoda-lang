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

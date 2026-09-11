//! Protocol-conformance tests for `--json` (docs/protocol.md): the JSON Lines
//! event stream is a machine contract, so every batch run must use the closed
//! event vocabulary with stable payload fields.

use std::io::Write;
use std::process::{Command, Output, Stdio};

use serde_json::Value;

use common::EVENT_VOCABULARY as VOCABULARY;

mod common;

fn spawn_json(args: &[&str]) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_sokonanoda"));
    cmd.arg("--json").args(args);
    cmd
}

fn run_json_stdin(input: &str) -> (Output, Vec<Value>) {
    let mut child = spawn_json(&[])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn sokonanoda --json");
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(input.as_bytes())
        .expect("write stdin");
    let out = child.wait_with_output().expect("wait");
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    (out, parse_event_stream(&stdout))
}

fn run_json_file(path: &str) -> (Output, Vec<Value>) {
    let out = spawn_json(&[path])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run sokonanoda --json on file");
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    (out, parse_event_stream(&stdout))
}

fn parse_event_stream(stdout: &str) -> Vec<Value> {
    stdout
        .lines()
        .enumerate()
        .map(|(i, line)| {
            serde_json::from_str(line).unwrap_or_else(|e| {
                panic!(
                    "--json stdout line {} is not valid JSON ({e}):\n  raw: {line:?}\n  full stdout:\n{stdout}",
                    i + 1
                )
            })
        })
        .collect()
}

fn event_type(event: &Value) -> &str {
    event.get("type").and_then(|v| v.as_str()).unwrap_or("")
}

fn non_empty_str<'a>(event: &'a Value, field: &str, context: &str) -> &'a str {
    event
        .get(field)
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| panic!("{context}: event missing non-empty {field:?} field: {event}"))
}

fn diagnostics(events: &[Value]) -> Vec<&Value> {
    events
        .iter()
        .filter(|e| event_type(e) == "diagnostic")
        .collect()
}

fn assert_diagnostic_shape(event: &Value, context: &str) {
    let stage = non_empty_str(event, "stage", context);
    assert!(
        matches!(stage, "parse" | "elab" | "kernel"),
        "{context}: diagnostic stage {stage:?} is outside parse/elab/kernel: {event}"
    );
    non_empty_str(event, "code", context);
    non_empty_str(event, "message", context);
    let span = event
        .get("span")
        .unwrap_or_else(|| panic!("{context}: diagnostic missing span: {event}"));
    for side in ["start", "end"] {
        let point = span
            .get(side)
            .unwrap_or_else(|| panic!("{context}: diagnostic span missing {side:?}: {event}"));
        for field in ["line", "column", "offset"] {
            assert!(
                point.get(field).and_then(|v| v.as_u64()).is_some(),
                "{context}: diagnostic span.{side}.{field} missing or not an integer: {event}"
            );
        }
    }
}

#[test]
fn every_example_stream_is_closed_vocabulary() {
    let examples = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples");
    let mut files: Vec<_> = std::fs::read_dir(examples)
        .expect("read examples dir")
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map(|ext| ext == "sokonanoda")
                .unwrap_or(false)
        })
        .collect();
    files.sort_by_key(|e| e.file_name());
    assert!(!files.is_empty(), "no .sokonanoda files under {examples}");

    for file in files {
        let path = file.path();
        let name = file.file_name().to_string_lossy().into_owned();
        let (out, events) = run_json_file(path.to_str().expect("utf-8 path"));
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            out.status.success(),
            "lesson {name} failed under --json:\nstderr:\n{stderr}"
        );

        for (i, event) in events.iter().enumerate() {
            let context = format!("lesson {name} event {}", i + 1);
            let t = event_type(event);
            assert!(
                VOCABULARY.contains(&t),
                "{context}: type {t:?} outside the closed vocabulary: {event}"
            );
            non_empty_str(event, "human", &context);
            if t == "diagnostic" {
                assert_diagnostic_shape(event, &context);
            }
        }
    }
}

#[test]
fn lesson01_golden_event_sequence() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/lesson-01.sokonanoda"
    );
    let (out, events) = run_json_file(path);
    assert!(
        out.status.success(),
        "lesson-01 must pass the kernel, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let types: Vec<&str> = events.iter().map(event_type).collect();
    assert_eq!(
        types,
        ["decl.checked", "expr.typed", "exercise.open"],
        "lesson-01 emits exactly this ordered event sequence"
    );

    assert_eq!(events[0]["name"], "id", "decl.checked carries the name");
    assert_eq!(
        events[1]["inferred_type"], "Prop -> Prop",
        "expr.typed carries the inferred type"
    );
    assert_eq!(
        events[1]["text"], "id",
        "expr.typed text is the exact source slice"
    );
}

#[test]
fn lesson02_golden_subsequence() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/lesson-02.sokonanoda"
    );
    let (out, events) = run_json_file(path);
    assert!(
        out.status.success(),
        "lesson-02 must pass the kernel, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let types: Vec<&str> = events.iter().map(event_type).collect();
    for expected in [
        "decl.checked",
        "expr.typed",
        "expr.reduced",
        "exercise.open",
    ] {
        assert!(
            types.contains(&expected),
            "lesson-02 stream must contain {expected:?}; got {types:?}"
        );
    }

    assert!(
        events
            .iter()
            .any(|e| event_type(e) == "decl.checked" && e["name"] == "two"),
        "decl.checked {{name: two}} missing: {events:?}"
    );
    assert!(
        events
            .iter()
            .any(|e| event_type(e) == "expr.typed" && e["inferred_type"] == "Nat"),
        "expr.typed {{inferred_type: Nat}} missing: {events:?}"
    );
    assert!(
        events
            .iter()
            .any(|e| event_type(e) == "expr.reduced" && e["value"] == "5"),
        "expr.reduced {{value: 5}} missing (2 + 3 must reduce to 5): {events:?}"
    );
}

#[test]
fn kernel_rejection_diagnostic_shape() {
    let (out, events) = run_json_stdin("def bad : Prop -> Type := fun (x : Prop) => x\n");
    assert!(!out.status.success(), "kernel rejection must exit non-zero");

    let diags = diagnostics(&events);
    assert_eq!(
        diags.len(),
        1,
        "exactly one diagnostic for a kernel rejection: {events:?}"
    );
    let d = diags[0];
    assert_eq!(d["stage"], "kernel", "rejection stages as kernel: {d}");
    assert_eq!(d["code"], "kernel-rejected");
    assert_eq!(
        d["span"]["start"]["line"], 1,
        "rejection span starts on the offending line: {d}"
    );
    assert!(
        d.get("hint")
            .and_then(|h| h.as_str())
            .is_none_or(|s| !s.is_empty()),
        "kernel diagnostics carry a teaching hint per docs/protocol.md: {d}"
    );
    assert_diagnostic_shape(d, "kernel rejection");
}

#[test]
fn parse_error_diagnostic_shape() {
    let (out, events) = run_json_stdin("def broken : Prop :=\n");
    assert!(!out.status.success(), "parse error must exit non-zero");

    let diags = diagnostics(&events);
    assert_eq!(
        diags.len(),
        1,
        "exactly one diagnostic for a parse error: {events:?}"
    );
    let d = diags[0];
    assert_eq!(d["stage"], "parse");
    assert!(
        matches!(
            d["code"].as_str(),
            Some("unexpected-token") | Some("unexpected-eof")
        ),
        "parse code must be a documented parse-stage code: {d}"
    );
    assert_eq!(
        d["span"]["start"]["line"], 2,
        "the parse error points at the line where input ran out: {d}"
    );
    assert_diagnostic_shape(d, "parse error");
    assert!(
        d.get("hint")
            .and_then(|v| v.as_str())
            .map(|h| !h.is_empty())
            .unwrap_or(false),
        "parse diagnostics carry a hint per docs/protocol.md: {d}"
    );
}

#[test]
fn open_exercise_is_success_state() {
    let (out, events) = run_json_stdin("example : Prop -> Prop := sorry\n");
    assert_eq!(
        out.status.code(),
        Some(0),
        "an open exercise is a successful state, not an error; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        events.iter().any(|e| event_type(e) == "exercise.open"),
        "exercise.open must be emitted for an unfilled sorry: {events:?}"
    );
    assert!(
        diagnostics(&events).is_empty(),
        "an open exercise must not produce diagnostics: {events:?}"
    );
}

#[test]
fn reserved_declaration_name_warns_but_stays_successful() {
    let (out, events) = run_json_stdin("axiom Prop : Sort 1\n");
    assert!(
        out.status.success(),
        "a warning must not fail the run; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let warnings: Vec<&Value> = events
        .iter()
        .filter(|e| event_type(e) == "warning")
        .collect();
    assert_eq!(warnings.len(), 1, "exactly one warning: {events:?}");
    let w = warnings[0];
    assert_eq!(w["code"], "reserved-declaration-name");
    non_empty_str(w, "message", "warning");
    non_empty_str(w, "hint", "warning");
    assert_eq!(
        w["span"]["start"]["line"], 1,
        "warning points at the declaration line: {w}"
    );
    assert!(
        events
            .iter()
            .any(|e| event_type(e) == "decl.checked" && e["name"] == "Prop"),
        "the declaration itself still passes the kernel: {events:?}"
    );
}

#[test]
fn protocol_document_lists_every_emitted_type() {
    let doc = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/protocol.md"
    ))
    .expect("read docs/protocol.md");
    for t in VOCABULARY {
        assert!(
            doc.contains(t),
            "docs/protocol.md no longer documents event type {t:?}"
        );
    }
}

#[test]
fn elab_unknown_identifier_diagnostic() {
    let (out, events) = run_json_stdin("#check mystery\n");
    assert!(
        !out.status.success(),
        "unknown identifier must exit non-zero"
    );

    let diags = diagnostics(&events);
    assert_eq!(
        diags.len(),
        1,
        "exactly one diagnostic for an unknown identifier: {events:?}"
    );
    let d = diags[0];
    assert_eq!(d["stage"], "elab", "unknown identifier stages as elab: {d}");
    assert_eq!(d["code"], "elab-unknown-identifier");
    assert!(
        d.get("hint")
            .and_then(|h| h.as_str())
            .is_none_or(|s| !s.is_empty()),
        "elab diagnostics carry a teaching hint per docs/protocol.md: {d}"
    );
    assert_diagnostic_shape(d, "elab unknown identifier");
}

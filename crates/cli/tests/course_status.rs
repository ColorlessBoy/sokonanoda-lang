//! `sokonanoda course <course.json>` (docs/protocol.md "Course map"): the
//! progress-map contract — machine view (JSON Lines over `--json`), human
//! view, error tolerance (progress is not an error: a broken unit still
//! exits 0 with an `error` field), and the only failure mode (unreadable
//! manifest). Per-unit counts mirror the golden map in course.rs.

use std::process::{Command, Stdio};

use serde_json::Value;

const COURSE_MANIFEST: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../course/course.json");

fn run_course(args: &[&str]) -> std::process::Output {
    let child = Command::new(env!("CARGO_BIN_EXE_sokonanoda"))
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn sokonanoda");
    child.wait_with_output().expect("wait")
}

fn parse_lines(stdout: &str) -> Vec<Value> {
    stdout
        .lines()
        .map(|line| {
            serde_json::from_str(line)
                .unwrap_or_else(|e| panic!("stdout line is not valid JSON ({e}): {line:?}"))
        })
        .collect()
}

fn typed<'a>(events: &'a [Value], ty: &str) -> Vec<&'a Value> {
    events
        .iter()
        .filter(|e| e.get("type").and_then(|v| v.as_str()) == Some(ty))
        .collect()
}

fn u64_field(event: &Value, field: &str) -> u64 {
    event
        .get(field)
        .and_then(|v| v.as_u64())
        .unwrap_or_else(|| panic!("course event missing integer {field:?}: {event}"))
}

/// Per-unit golden counts in course.json order:
/// (decl.checked, exercise.open, failed, expr.reduced) — identical to the
/// course.rs golden (checked, open, reduced) with failed = 0 throughout.
const GOLDEN: [(u64, u64, u64, u64); 6] = [
    (12, 5, 0, 1),
    (2, 5, 0, 2),
    (1, 4, 0, 1),
    (0, 3, 0, 0),
    (4, 3, 0, 1),
    (13, 6, 0, 0),
];

#[test]
fn course_subcommand_aggregates_the_manifest() {
    let out = run_course(&["course", COURSE_MANIFEST, "--json"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "course must exit 0 (progress is not an error), stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let events = parse_lines(&stdout);
    assert_eq!(
        events.len(),
        7,
        "exactly 6 course.unit + 1 course.summary, got: {events:?}"
    );
    let units = typed(&events, "course.unit");
    assert_eq!(units.len(), 6, "one course.unit per manifest entry");
    let summaries = typed(&events, "course.summary");
    assert_eq!(summaries.len(), 1, "exactly one course.summary");

    for (i, (unit, expected)) in units.iter().zip(GOLDEN).enumerate() {
        let context = format!("course.unit {}", i + 1);
        assert_eq!(u64_field(unit, "unit"), (i + 1) as u64, "{context}: order");
        assert_eq!(u64_field(unit, "checked"), expected.0, "{context}: checked");
        assert_eq!(u64_field(unit, "open"), expected.1, "{context}: open");
        assert_eq!(u64_field(unit, "failed"), expected.2, "{context}: failed");
        assert_eq!(u64_field(unit, "reduced"), expected.3, "{context}: reduced");
        assert!(
            unit.get("error").is_none(),
            "{context}: healthy unit carries no error field: {unit}"
        );
    }

    let summary = summaries[0];
    assert_eq!(u64_field(summary, "units"), 6, "summary units");
    assert_eq!(u64_field(summary, "checked"), 32, "summary checked");
    assert_eq!(u64_field(summary, "open"), 26, "summary open");
    assert_eq!(u64_field(summary, "failed"), 0, "summary failed");
}

#[test]
fn course_subcommand_human_view_lists_units() {
    let out = run_course(&["course", COURSE_MANIFEST]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "course must exit 0");

    assert!(
        stdout.contains("unit 1"),
        "first unit line missing:\n{stdout}"
    );
    assert!(
        stdout.contains("checked"),
        "progress counts missing:\n{stdout}"
    );
    let last = stdout.lines().last().unwrap_or_default();
    assert!(
        last.contains("26") && last.contains("checked") && last.contains("failed"),
        "the final line must be the totals, got: {last:?}"
    );
}

#[test]
fn course_subcommand_survives_a_broken_unit() {
    let dir = std::env::temp_dir().join(format!("sokonanoda-course-broken-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let manifest = dir.join("course.json");
    std::fs::write(
        &manifest,
        r#"[{"file":"ghost.sokonanoda","title":"幽灵","unit":9}]"#,
    )
    .expect("write manifest");

    let out = run_course(&["course", manifest.to_str().expect("utf-8 path"), "--json"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        out.status.code(),
        Some(0),
        "a broken unit is progress, not an error: stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let events = parse_lines(&stdout);
    let units = typed(&events, "course.unit");
    assert_eq!(units.len(), 1, "the broken unit still reports: {events:?}");
    let unit = units[0];
    assert_eq!(
        unit.get("file").and_then(|v| v.as_str()),
        Some("ghost.sokonanoda"),
        "the unit entry names the missing file: {unit}"
    );
    assert_eq!(u64_field(unit, "unit"), 9);
    let error = unit
        .get("error")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| panic!("broken unit carries an error field: {unit}"));
    assert!(
        error.contains("cannot read"),
        "error names the failure mode (unreadable file): {error:?}"
    );

    let summaries = typed(&events, "course.summary");
    assert_eq!(summaries.len(), 1);
    let summary = summaries[0];
    assert_eq!(u64_field(summary, "units"), 1, "summary counts the unit");
    assert_eq!(u64_field(summary, "checked"), 0, "summary checked");
    assert_eq!(u64_field(summary, "open"), 0, "summary open");
    assert_eq!(u64_field(summary, "failed"), 0, "summary failed");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn course_subcommand_fails_on_missing_manifest() {
    let missing = std::env::temp_dir().join(format!(
        "sokonanoda-course-missing-{}.json",
        std::process::id()
    ));
    let out = run_course(&["course", missing.to_str().expect("utf-8 path")]);
    assert!(
        out.status.code().is_some_and(|code| code != 0),
        "an unreadable manifest must exit non-zero, got {:?}",
        out.status.code()
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("cannot read course manifest"),
        "stderr names the unreadable manifest: {stderr}"
    );
    assert!(
        String::from_utf8_lossy(&out.stdout).is_empty(),
        "no events are emitted when the manifest cannot be read"
    );
}

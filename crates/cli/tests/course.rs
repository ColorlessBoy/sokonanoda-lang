//! The course layer (`course/`) is CI-guarded: every learner canvas must
//! compile with its open exercises, the per-unit golden event counts must stay
//! stable, every agent solution twin must be hole-free and diagnostic-free,
//! `course.json` must list the five units in order, and the bilingual `en/`
//! mirrors must produce byte-identical event counts to their Chinese twins.
//! See `course/README.md`.

use std::process::{Command, Stdio};

use serde_json::Value;

const COURSE_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../course");

fn run_binary(args: &[&str]) -> std::process::Output {
    let child = Command::new(env!("CARGO_BIN_EXE_sokonanoda"))
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn sokonanoda");
    child.wait_with_output().expect("wait")
}

fn json_events(stdout: &str) -> Vec<Value> {
    stdout
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

fn count_type(events: &[Value], ty: &str) -> usize {
    events
        .iter()
        .filter(|e| e.get("type").and_then(|v| v.as_str()) == Some(ty))
        .count()
}

/// Per-unit golden event counts, measured with `sokonanoda --json` when the
/// course layer landed (unit file -> (decl.checked, exercise.open,
/// expr.reduced)). Learner canvases carry open exercises by design; the exact
/// counts are the contract, so adding/removing an exercise is a deliberate
/// golden update.
const GOLDEN: &[(&str, (usize, usize, usize))] = &[
    ("unit1-propositions-proofs.sokonanoda", (12, 5, 1)),
    ("unit2-equality-rfl.sokonanoda", (2, 5, 2)),
    ("unit3-functions-arrows.sokonanoda", (1, 4, 1)),
    ("unit4-universes-sort.sokonanoda", (0, 3, 0)),
    ("unit5-induction-nat-rec.sokonanoda", (4, 3, 1)),
    ("unit6-by-tactics.sokonanoda", (13, 5, 0)),
];

#[test]
fn every_course_canvas_compiles_with_golden_event_counts() {
    // Only the top-level canvases (NOT course/solutions/).
    let mut files: Vec<_> = std::fs::read_dir(COURSE_DIR)
        .expect("read course dir")
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map(|ext| ext == "sokonanoda")
                .unwrap_or(false)
        })
        .collect();
    files.sort_by_key(|e| e.file_name());
    assert!(!files.is_empty(), "no .sokonanoda files under {COURSE_DIR}");
    assert_eq!(
        files.len(),
        GOLDEN.len(),
        "course/*.sokonanoda and the golden map must stay in sync"
    );

    for file in files {
        let path = file.path();
        let name = file.file_name().to_string_lossy().into_owned();
        let expected = GOLDEN
            .iter()
            .find(|(n, _)| *n == name.as_str())
            .unwrap_or_else(|| panic!("golden map is missing course canvas {name}"))
            .1;

        let plain = run_binary(&[path.to_str().expect("utf-8 path")]);
        let stderr = String::from_utf8_lossy(&plain.stderr);
        assert!(
            plain.status.success(),
            "course canvas {name} must compile (exit 0), got {:?}\nstderr:\n{stderr}",
            plain.status.code()
        );
        assert!(
            !stderr.contains("error["),
            "course canvas {name} must not print error lines:\n{stderr}"
        );

        let out = run_binary(&["--json", path.to_str().expect("utf-8 path")]);
        let stdout = String::from_utf8_lossy(&out.stdout);
        let events = json_events(&stdout);
        let actual = (
            count_type(&events, "decl.checked"),
            count_type(&events, "exercise.open"),
            count_type(&events, "expr.reduced"),
        );
        assert_eq!(
            actual, expected,
            "golden event counts drifted for {name} (decl.checked, exercise.open, expr.reduced)"
        );
    }
}

#[test]
fn every_solution_twin_is_fully_solved() {
    let mut files: Vec<_> = std::fs::read_dir(format!("{COURSE_DIR}/solutions"))
        .expect("read course/solutions")
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map(|ext| ext == "sokonanoda")
                .unwrap_or(false)
        })
        .collect();
    files.sort_by_key(|e| e.file_name());
    assert_eq!(
        files.len(),
        GOLDEN.len(),
        "expected one solution twin per unit under course/solutions"
    );

    for file in files {
        let path = file.path();
        let name = file.file_name().to_string_lossy().into_owned();
        let out = run_binary(&["--json", path.to_str().expect("utf-8 path")]);
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(out.status.success(), "solution {name} failed:\n{stdout}");
        let events = json_events(&stdout);
        assert_eq!(
            count_type(&events, "diagnostic"),
            0,
            "solution {name} must compile with 0 diagnostics:\n{stdout}"
        );
        assert_eq!(
            count_type(&events, "exercise.open"),
            0,
            "solution {name} must fill every sorry hole:\n{stdout}"
        );
    }
}

#[test]
fn course_json_lists_the_six_units_in_order() {
    let raw =
        std::fs::read_to_string(format!("{COURSE_DIR}/course.json")).expect("read course.json");
    let entries: Vec<Value> = serde_json::from_str(&raw).expect("parse course.json");

    let expected: [(&str, u64); 6] = [
        ("unit1-propositions-proofs.sokonanoda", 1),
        ("unit2-equality-rfl.sokonanoda", 2),
        ("unit3-functions-arrows.sokonanoda", 3),
        ("unit4-universes-sort.sokonanoda", 4),
        ("unit5-induction-nat-rec.sokonanoda", 5),
        ("unit6-by-tactics.sokonanoda", 6),
    ];

    assert_eq!(
        entries.len(),
        expected.len(),
        "course.json must list exactly the 6 course files"
    );
    for (entry, (file, unit)) in entries.iter().zip(expected) {
        assert_eq!(
            entry.get("file").and_then(|v| v.as_str()),
            Some(file),
            "course.json entry order/file mismatch: expected {file}"
        );
        assert_eq!(
            entry.get("unit").and_then(|v| v.as_u64()),
            Some(unit),
            "course.json unit number mismatch for {file}"
        );
    }
}

/// Bilingual mirrors: each `course/en/` canvas must produce the same event
/// counts as its Chinese twin in `course/`, and each `course/en/solutions/`
/// key must be fully solved (0 diagnostics, 0 open exercises) like its Chinese
/// counterpart. Judgment goes through the kernel (event counts), never text
/// comparison — the comments are meant to differ.
#[test]
fn en_mirrors_match_chinese_event_counts() {
    let cn_dir = std::fs::read_dir(COURSE_DIR).expect("read course dir");
    let en_dir = std::fs::read_dir(format!("{COURSE_DIR}/en")).expect("read course/en");

    let top_level = |dir: std::fs::ReadDir| -> Vec<String> {
        dir.filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .map(|ext| ext == "sokonanoda")
                    .unwrap_or(false)
            })
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect()
    };
    let mut cn_files = top_level(cn_dir);
    let mut en_files = top_level(en_dir);
    cn_files.sort();
    en_files.sort();
    assert_eq!(
        cn_files, en_files,
        "course/ and course/en/ must hold the same .sokonanoda file names"
    );

    for name in &cn_files {
        let cn_path = format!("{COURSE_DIR}/{name}");
        let en_path = format!("{COURSE_DIR}/en/{name}");
        let cn = run_binary(&["--json", &cn_path]);
        let en = run_binary(&["--json", &en_path]);
        assert!(cn.status.success(), "CN canvas {name} failed");
        assert!(en.status.success(), "EN canvas {name} failed");
        let count = |out: &std::process::Output| {
            let events = json_events(&String::from_utf8_lossy(&out.stdout));
            (
                count_type(&events, "decl.checked"),
                count_type(&events, "exercise.open"),
                count_type(&events, "expr.reduced"),
                count_type(&events, "diagnostic"),
            )
        };
        assert_eq!(
            count(&cn),
            count(&en),
            "EN mirror drifted from CN twin for {name}"
        );
    }

    let mut cn_sols: Vec<_> = std::fs::read_dir(format!("{COURSE_DIR}/solutions"))
        .expect("read course/solutions")
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map(|ext| ext == "sokonanoda")
                .unwrap_or(false)
        })
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    let mut en_sols: Vec<_> = std::fs::read_dir(format!("{COURSE_DIR}/en/solutions"))
        .expect("read course/en/solutions")
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map(|ext| ext == "sokonanoda")
                .unwrap_or(false)
        })
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    cn_sols.sort();
    en_sols.sort();
    assert_eq!(
        cn_sols, en_sols,
        "course/solutions/ and course/en/solutions/ must hold the same file names"
    );

    for name in &cn_sols {
        let en_path = format!("{COURSE_DIR}/en/solutions/{name}");
        let out = run_binary(&["--json", &en_path]);
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(out.status.success(), "EN solution {name} failed:\n{stdout}");
        let events = json_events(&stdout);
        assert_eq!(
            count_type(&events, "diagnostic"),
            0,
            "EN solution {name} must compile with 0 diagnostics:\n{stdout}"
        );
        assert_eq!(
            count_type(&events, "exercise.open"),
            0,
            "EN solution {name} must fill every sorry hole:\n{stdout}"
        );
    }
}

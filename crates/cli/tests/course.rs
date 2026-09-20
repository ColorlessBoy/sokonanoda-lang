//! The course layer (`course/`) is CI-guarded: every learner canvas must
//! compile with its open exercises, the per-unit golden event counts must stay
//! stable, every agent solution twin must be hole-free and diagnostic-free,
//! `course.json` must list the eleven units in order, and the bilingual `en/`
//! mirrors must produce byte-identical event counts to their Chinese twins.
//! See `course/README.md`.

use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::Value;

const COURSE_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../course");

static CACHE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// A unique, empty compile-cache dir so this suite never reads another run's
/// entries (`SOKONANODA_CACHE_DIR`, docs/protocol.md).
fn cache_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sokonanoda-course-cache-{tag}-{}-{}",
        std::process::id(),
        CACHE_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn run_binary(args: &[&str]) -> std::process::Output {
    let child = Command::new(env!("CARGO_BIN_EXE_sokonanoda"))
        .args(args)
        .env("SOKONANODA_CACHE_DIR", cache_dir("course"))
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

/// The `name` payload of every event of the given type, in order. Events
/// without a `name` (notably an anonymous `example`'s `exercise.open`) are
/// skipped — the vocabulary simply does not name them.
fn event_names(events: &[Value], ty: &str) -> Vec<String> {
    events
        .iter()
        .filter(|e| e.get("type").and_then(|v| v.as_str()) == Some(ty))
        .filter_map(|e| e.get("name").and_then(|v| v.as_str()).map(str::to_owned))
        .collect()
}

/// The `(decl.checked, exercise.open, expr.reduced, expr.typed, diagnostic)`
/// tuple for a spawned check. `expr.typed` is included so a stray or missing
/// `#check` cannot slip past the bilingual/solution parity checks.
fn event_counts(out: &std::process::Output) -> (usize, usize, usize, usize, usize) {
    let events = json_events(&String::from_utf8_lossy(&out.stdout));
    (
        count_type(&events, "decl.checked"),
        count_type(&events, "exercise.open"),
        count_type(&events, "expr.reduced"),
        count_type(&events, "expr.typed"),
        count_type(&events, "diagnostic"),
    )
}

/// Per-unit golden event counts, measured with `sokonanoda --json` when the
/// course layer landed (unit file -> (decl.checked, exercise.open,
/// expr.reduced)). Learner canvases carry open exercises by design; the exact
/// counts are the contract, so adding/removing an exercise is a deliberate
/// golden update.
const GOLDEN: &[(&str, (usize, usize, usize))] = &[
    ("unit1-propositions-proofs.sokonanoda", (6, 6, 1)),
    ("unit2-equality-rfl.sokonanoda", (3, 5, 2)),
    ("unit3-functions-arrows.sokonanoda", (2, 6, 2)),
    ("unit4-by-tactics.sokonanoda", (9, 6, 0)),
    ("unit5-universes-sort.sokonanoda", (0, 6, 1)),
    ("unit6-induction-recursion-1.sokonanoda", (7, 6, 3)),
    ("unit7-induction-recursion-2.sokonanoda", (7, 4, 4)),
    ("unit8-quantifiers.sokonanoda", (10, 7, 1)),
    ("unit9-relations-connectives.sokonanoda", (8, 8, 0)),
    ("unit10-reading-proofs.sokonanoda", (2, 6, 0)),
    ("unit11-modules-projects.sokonanoda", (2, 6, 0)),
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

/// Every exercise on a learner canvas must exist as a declaration in that
/// unit's solution twin. We compare the kernel event streams, never source
/// text: the canvas reports each open exercise through `exercise.open.name`,
/// and the solution reports the very same name through `decl.checked.name`.
///
/// Anonymous rule: an `example` that still carries a `sorry` emits an
/// `exercise.open` with no `name` field (docs/protocol.md), so it cannot be
/// matched by name. Such anonymous exercises are therefore skipped here; they
/// are still covered by `every_solution_twin_is_fully_solved` and by the
/// bilingual parity test.
#[test]
fn solution_covers_every_canvas_exercise() {
    for &(name, _) in GOLDEN {
        let canvas = format!("{COURSE_DIR}/{name}");
        let solution = format!(
            "{COURSE_DIR}/solutions/{}",
            name.replace(".sokonanoda", "-solution.sokonanoda")
        );
        let canvas_out = run_binary(&["--json", &canvas]);
        let solution_out = run_binary(&["--json", &solution]);
        assert!(
            canvas_out.status.success(),
            "canvas {name} must compile to check its exercises"
        );
        assert!(
            solution_out.status.success(),
            "solution twin of {name} must compile"
        );
        let canvas_events = json_events(&String::from_utf8_lossy(&canvas_out.stdout));
        let solution_events = json_events(&String::from_utf8_lossy(&solution_out.stdout));
        let exercises = event_names(&canvas_events, "exercise.open");
        let declarations = event_names(&solution_events, "decl.checked");
        assert!(
            !exercises.is_empty(),
            "canvas {name} has no named exercises — nothing to cross-check"
        );
        for exercise in &exercises {
            assert!(
                declarations.iter().any(|d| d == exercise),
                "canvas {name}: exercise {exercise:?} has no declaration in its solution twin \
                 (solution declares {declarations:?})"
            );
        }
    }
}

#[test]
fn course_json_lists_the_eleven_units_in_order() {
    let raw =
        std::fs::read_to_string(format!("{COURSE_DIR}/course.json")).expect("read course.json");
    let entries: Vec<Value> = serde_json::from_str(&raw).expect("parse course.json");

    let expected: [(&str, u64); 11] = [
        ("unit1-propositions-proofs.sokonanoda", 1),
        ("unit2-equality-rfl.sokonanoda", 2),
        ("unit3-functions-arrows.sokonanoda", 3),
        ("unit4-by-tactics.sokonanoda", 4),
        ("unit5-universes-sort.sokonanoda", 5),
        ("unit6-induction-recursion-1.sokonanoda", 6),
        ("unit7-induction-recursion-2.sokonanoda", 7),
        ("unit8-quantifiers.sokonanoda", 8),
        ("unit9-relations-connectives.sokonanoda", 9),
        ("unit10-reading-proofs.sokonanoda", 10),
        ("unit11-modules-projects.sokonanoda", 11),
    ];

    assert_eq!(
        entries.len(),
        expected.len(),
        "course.json must list exactly the 11 course files"
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
        assert_eq!(
            event_counts(&cn),
            event_counts(&en),
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

/// The bilingual contract extends to the answer keys: every CN solution under
/// `course/solutions/` and its EN twin under `course/en/solutions/` must
/// produce identical kernel event counts — `(decl.checked, exercise.open,
/// expr.reduced, expr.typed, diagnostic)`. `expr.typed` is part of the tuple
/// because it is exactly the class of drift that once dropped `#check (Type 0)`
/// from the EN Unit 4 key while the four-tuple still matched.
#[test]
fn en_solutions_match_chinese_event_counts() {
    fn soko_names(dir: &str) -> Vec<String> {
        std::fs::read_dir(dir)
            .unwrap_or_else(|e| panic!("read {dir}: {e}"))
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .map(|ext| ext == "sokonanoda")
                    .unwrap_or(false)
            })
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect()
    }

    let mut cn_sols = soko_names(&format!("{COURSE_DIR}/solutions"));
    let mut en_sols = soko_names(&format!("{COURSE_DIR}/en/solutions"));
    cn_sols.sort();
    en_sols.sort();
    assert_eq!(
        cn_sols, en_sols,
        "course/solutions/ and course/en/solutions/ must hold the same file names"
    );
    assert!(!cn_sols.is_empty(), "no solution twins found");

    for name in &cn_sols {
        let cn = run_binary(&["--json", &format!("{COURSE_DIR}/solutions/{name}")]);
        let en = run_binary(&["--json", &format!("{COURSE_DIR}/en/solutions/{name}")]);
        assert!(cn.status.success(), "CN solution {name} failed");
        assert!(en.status.success(), "EN solution {name} failed");
        assert_eq!(
            event_counts(&cn),
            event_counts(&en),
            "EN solution {name} drifted from CN twin"
        );
    }
}

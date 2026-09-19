//! 课程清单 **v2**（`soko.course/2`）与 **v1**（扁平数组）的兼容契约
//! （台账 G-07；设计 `docs/design/course-manifest-v2.md`）。
//!
//! 判据分两层：
//!
//! 1. **向后兼容（硬要求）**：v1 扁平数组的事件里**不出现** `volume`/`chapter`/
//!    `tags` 三个键——不是空数组、不是 `null`，而是**没有这个键**。既有消费者
//!    （VS Code stub 宿主、任何按 `Object.keys` 遍历的脚本）行为逐字节不变；
//! 2. **v2 增量**：`course.unit` 多出 `volume`/`chapter`/`tags`，
//!    `course.summary` 多出 `volumes`/`chapters`；v1 的既有字段一个不少。

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::Value;

const REPO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
const SET_THEORY_MANIFEST: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../courses/set-theory/course.json"
);
const INTRO_MANIFEST: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../course/course.json");

static CACHE_COUNTER: AtomicU64 = AtomicU64::new(0);

fn cache_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sokonanoda-course-manifest-cache-{tag}-{}-{}",
        std::process::id(),
        CACHE_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn run_course(manifest: &str) -> std::process::Output {
    run_course_args(&["course", manifest, "--json"])
}

/// 任意实参的 `course` 调用（多清单聚合要一次给多个路径 / `--all`）。
fn run_course_args(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_sokonanoda"))
        .args(args)
        .env("SOKONANODA_CACHE_DIR", cache_dir("manifest"))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn sokonanoda")
        .wait_with_output()
        .expect("wait")
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

/// 造一个最小课程仓夹具：`sokonanoda.toml` + `lib/` + `units/`（v2 清单）。
/// 单元真的 `import lib.Logic`，所以它走**项目闭包**——与卷 I 的布局同构。
fn fixture(tag: &str, manifest: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sokonanoda-course-manifest-{tag}-{}-{}",
        std::process::id(),
        CACHE_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    write_fixture(&dir, manifest);
    dir
}

/// 在指定目录下造一份课程夹具——多清单聚合（`course --all`）的测试要共用一棵树。
fn fixture_in(root: &Path, name: &str, manifest: &str) -> PathBuf {
    let dir = root.join(name);
    write_fixture(&dir, manifest);
    dir
}

fn write_fixture(dir: &Path, manifest: &str) {
    let _ = std::fs::remove_dir_all(dir);
    std::fs::create_dir_all(dir.join("lib")).expect("lib dir");
    std::fs::create_dir_all(dir.join("units")).expect("units dir");
    std::fs::write(dir.join("sokonanoda.toml"), "name = \"fixture\"\n").expect("manifest");
    std::fs::write(
        dir.join("lib/Logic.sokonanoda"),
        "def sokoFixtureId (P : Prop) : Prop := P\n",
    )
    .expect("lib");
    let unit = "import lib.Logic\n\ndef u (P : Prop) : Prop := sokoFixtureId P\n";
    std::fs::write(dir.join("units/a.sokonanoda"), unit).expect("unit a");
    std::fs::write(dir.join("units/b.sokonanoda"), unit).expect("unit b");
    std::fs::write(dir.join("course.json"), manifest).expect("course.json");
}

fn manifest_path(dir: &Path) -> String {
    dir.join("course.json").to_string_lossy().into_owned()
}

#[test]
fn v1_flat_manifest_keeps_the_event_shape_byte_for_byte() {
    let manifest = r#"[
      {"file":"units/a.sokonanoda","title":"A","title_en":"Unit A","unit":1},
      {"file":"units/b.sokonanoda","title":"B","unit":2}
    ]"#;
    let dir = fixture("v1", manifest);
    let out = run_course(&manifest_path(&dir));
    assert!(
        out.status.success(),
        "a v1 flat manifest must still be accepted, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let events = parse_lines(&String::from_utf8_lossy(&out.stdout));
    let units = typed(&events, "course.unit");
    assert_eq!(units.len(), 2, "one course.unit per v1 entry: {events:?}");
    for unit in &units {
        assert_eq!(u64_field(unit, "checked"), 1, "the unit compiles: {unit}");
        assert_eq!(u64_field(unit, "failed"), 0, "and is not rejected: {unit}");
        // **兼容的硬要求**：三个新键一个都不许出现（不是 null，是没有）。
        for absent in ["volume", "chapter", "tags"] {
            assert!(
                unit.get(absent).is_none(),
                "v1 events must not carry {absent:?} (additive-only protocol): {unit}"
            );
        }
        assert!(unit.get("error").is_none(), "healthy v1 unit: {unit}");
    }
    let summaries = typed(&events, "course.summary");
    assert_eq!(summaries.len(), 1, "exactly one summary: {events:?}");
    let summary = summaries[0];
    assert_eq!(u64_field(summary, "units"), 2, "v1 summary units");
    assert_eq!(
        u64_field(summary, "volumes"),
        0,
        "a flat v1 manifest has no volumes: {summary}"
    );
    assert_eq!(
        u64_field(summary, "chapters"),
        0,
        "a flat v1 manifest has no chapters: {summary}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn v2_manifest_carries_volume_chapter_and_tags_additively() {
    let manifest = r#"{
      "schema": "soko.course/2",
      "name": "fixture",
      "title": "Fixture Volume",
      "volumes": [
        {"id": "I", "title": "Volume One", "chapters": [
          {"id": "I.1", "title": "First Chapter", "prereqs": [],
           "tags": ["alpha", "beta"], "quota": {"exercises": 6},
           "units": [
             {"file": "units/a.sokonanoda", "title": "A", "title_en": "Unit A", "unit": 1},
             {"file": "units/b.sokonanoda", "title": "B", "unit": 2}
           ]},
          {"id": "I.2", "title": "Second Chapter", "prereqs": ["I.1"], "tags": ["gamma"],
           "units": []}
        ]}
      ]
    }"#;
    let dir = fixture("v2", manifest);
    let out = run_course(&manifest_path(&dir));
    assert!(
        out.status.success(),
        "a v2 manifest must be accepted, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let events = parse_lines(&String::from_utf8_lossy(&out.stdout));
    let units = typed(&events, "course.unit");
    assert_eq!(units.len(), 2, "one course.unit per v2 unit: {events:?}");

    for unit in &units {
        // v1 的字段一个都不许丢。
        assert!(
            unit.get("file").and_then(|v| v.as_str()).is_some(),
            "v1 `file` must survive v2: {unit}"
        );
        assert_eq!(u64_field(unit, "checked"), 1, "{unit}");
        assert_eq!(u64_field(unit, "failed"), 0, "{unit}");
        // 新增字段。
        let volume = unit
            .get("volume")
            .and_then(|v| v.as_object())
            .unwrap_or_else(|| panic!("v2 unit must carry `volume`: {unit}"));
        assert_eq!(
            volume.get("id").and_then(|v| v.as_str()),
            Some("I"),
            "volume id: {unit}"
        );
        assert_eq!(
            volume.get("title").and_then(|v| v.as_str()),
            Some("Volume One"),
            "volume title: {unit}"
        );
        let chapter = unit
            .get("chapter")
            .and_then(|v| v.as_object())
            .unwrap_or_else(|| panic!("v2 unit must carry `chapter`: {unit}"));
        assert_eq!(
            chapter.get("id").and_then(|v| v.as_str()),
            Some("I.1"),
            "chapter id: {unit}"
        );
        assert_eq!(
            chapter.get("title").and_then(|v| v.as_str()),
            Some("First Chapter"),
            "chapter title: {unit}"
        );
        assert_eq!(
            unit.get("tags")
                .and_then(|v| v.as_array())
                .map(|tags| tags.iter().filter_map(|t| t.as_str()).collect::<Vec<_>>()),
            Some(vec!["alpha", "beta"]),
            "the flat `tags` copy mirrors chapter.tags: {unit}"
        );
    }

    let summary = typed(&events, "course.summary")[0];
    assert_eq!(u64_field(summary, "units"), 2, "summary units");
    assert_eq!(
        u64_field(summary, "volumes"),
        1,
        "one volume in the fixture: {summary}"
    );
    assert_eq!(
        u64_field(summary, "chapters"),
        2,
        "two chapters in the fixture (even the empty one counts): {summary}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn unknown_schema_is_refused_instead_of_guessed() {
    let dir = fixture("bad-schema", r#"{"schema":"soko.course/99","volumes":[]}"#);
    let out = run_course(&manifest_path(&dir));
    assert!(
        !out.status.success(),
        "an unknown schema must not be guessed: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("soko.course/99") || stderr.contains("schema"),
        "stderr names the unknown schema: {stderr}"
    );
    assert!(
        String::from_utf8_lossy(&out.stdout).is_empty(),
        "no events when the manifest is refused"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// 卷 I 的清单**已经是 v2**（本单的交付物之一），而且每个单元都恰好属于一个章。
#[test]
fn set_theory_manifest_is_v2_and_fully_grouped() {
    let raw = std::fs::read_to_string(SET_THEORY_MANIFEST).expect("read set-theory course.json");
    let value: Value = serde_json::from_str(&raw).expect("parse set-theory course.json");
    assert_eq!(
        value.get("schema").and_then(|v| v.as_str()),
        Some("soko.course/2"),
        "courses/set-theory/course.json must be v2"
    );
    assert_eq!(
        value.get("name").and_then(|v| v.as_str()),
        Some("set-theory"),
        "the manifest names the course"
    );
    let volumes = value
        .get("volumes")
        .and_then(|v| v.as_array())
        .expect("v2 manifest has volumes");
    assert!(!volumes.is_empty(), "at least one volume");
    let mut units = 0usize;
    let mut chapters = 0usize;
    for volume in volumes {
        let chapters_of = volume
            .get("chapters")
            .and_then(|v| v.as_array())
            .expect("each volume has chapters");
        assert!(
            !chapters_of.is_empty(),
            "each volume has at least one chapter"
        );
        for chapter in chapters_of {
            chapters += 1;
            assert!(
                chapter.get("id").and_then(|v| v.as_str()).is_some(),
                "chapter id: {chapter}"
            );
            assert!(
                chapter.get("prereqs").is_some(),
                "prereqs must be present (may be empty): {chapter}"
            );
            let quota = chapter
                .get("quota")
                .and_then(|v| v.get("exercises"))
                .and_then(|v| v.as_u64());
            assert!(
                quota.is_some_and(|n| n > 0),
                "quota.exercises is planned metadata: {chapter}"
            );
            for unit in chapter
                .get("units")
                .and_then(|v| v.as_array())
                .unwrap_or(&Vec::new())
            {
                units += 1;
                assert!(
                    unit.get("file").and_then(|v| v.as_str()).is_some()
                        && unit.get("unit").and_then(|v| v.as_u64()).is_some(),
                    "v2 unit entries keep the v1 shape: {unit}"
                );
            }
        }
    }
    assert_eq!(units, 12, "卷 I 的 12 个单元一个都不能丢");
    assert!(chapters >= 4, "卷 I 至少分成 4 章，实得 {chapters}");
    let _ = REPO;
}

/// 入门课保持 v1：它同时是"旧格式继续合法"的活体回归（本单不改它）。
#[test]
fn intro_course_manifest_stays_v1() {
    let raw = std::fs::read_to_string(INTRO_MANIFEST).expect("read intro course.json");
    let value: Value = serde_json::from_str(&raw).expect("parse intro course.json");
    assert!(
        value.is_array(),
        "course/course.json stays a flat v1 array (backward compatibility is a hard rule)"
    );
    let entries = value.as_array().expect("array");
    assert_eq!(entries.len(), 11, "the intro course keeps its 11 units");
    for entry in entries {
        assert!(
            entry.get("volume").is_none() && entry.get("chapter").is_none(),
            "v1 entries carry no v2 metadata: {entry}"
        );
    }
}

// ── 多清单聚合（`course <path>… [--all]`；设计 course-manifest-v2.md §4.5）──

/// 两份清单一次报：每个单元带 `manifest`，summary 带 `manifests`；
/// **单清单**调用仍然一个 `manifest` 键都不多（向后兼容的硬要求）。
#[test]
fn several_manifests_aggregate_in_order_and_tag_each_unit() {
    let manifest = r#"[
      {"file":"units/a.sokonanoda","title":"A","unit":1},
      {"file":"units/b.sokonanoda","title":"B","unit":2}
    ]"#;
    let one = fixture("agg-one", manifest);
    let two = fixture("agg-two", manifest);
    let one_path = manifest_path(&one);
    let two_path = manifest_path(&two);

    // 单清单：既有事件形状逐字节不变（summary 只多出 `manifests: 1` 这个计数）。
    let single = run_course(&one_path);
    assert!(single.status.success(), "single manifest must exit 0");
    let events = parse_lines(&String::from_utf8_lossy(&single.stdout));
    for unit in typed(&events, "course.unit") {
        assert!(
            unit.get("manifest").is_none(),
            "a single-manifest run must not add `manifest`: {unit}"
        );
    }
    let summary = typed(&events, "course.summary")[0];
    assert_eq!(u64_field(summary, "manifests"), 1, "{summary}");
    assert_eq!(u64_field(summary, "units"), 2, "{summary}");

    // 两份清单：4 个单元，前两个来自第一份、后两个来自第二份（调用序）。
    let out = run_course_args(&["course", &one_path, &two_path, "--json"]);
    assert!(
        out.status.success(),
        "aggregating two manifests must exit 0, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let events = parse_lines(&String::from_utf8_lossy(&out.stdout));
    let units = typed(&events, "course.unit");
    assert_eq!(
        units.len(),
        4,
        "one course.unit per unit of both: {events:?}"
    );
    for unit in &units[..2] {
        assert_eq!(
            unit.get("manifest").and_then(|v| v.as_str()),
            Some(one_path.as_str()),
            "the first manifest's units are tagged with its path: {unit}"
        );
    }
    for unit in &units[2..] {
        assert_eq!(
            unit.get("manifest").and_then(|v| v.as_str()),
            Some(two_path.as_str()),
            "the second manifest's units are tagged with its path: {unit}"
        );
    }
    let summaries = typed(&events, "course.summary");
    assert_eq!(summaries.len(), 1, "exactly one aggregate summary");
    let summary = summaries[0];
    assert_eq!(u64_field(summary, "manifests"), 2, "{summary}");
    assert_eq!(u64_field(summary, "units"), 4, "{summary}");
    assert_eq!(u64_field(summary, "checked"), 4, "4 units × 1 checked");
    assert_eq!(u64_field(summary, "failed"), 0, "{summary}");

    let _ = std::fs::remove_dir_all(&one);
    let _ = std::fs::remove_dir_all(&two);
}

/// `--all <dir>` 递归发现 `course.json`（跳过隐藏目录/`target`/`node_modules`），
/// 目录路径（不带 `--all`）读 `<dir>/course.json`，重复的清单只报一次。
#[test]
fn all_discovers_every_manifest_under_a_root() {
    let manifest = r#"[{"file":"units/a.sokonanoda","title":"A","unit":1}]"#;
    let root = std::env::temp_dir().join(format!(
        "sokonanoda-course-all-{}-{}",
        std::process::id(),
        CACHE_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&root);
    fixture_in(&root, "beta", manifest);
    fixture_in(&root, "alpha", manifest);
    // 干扰项：隐藏目录、`target/`、`node_modules/` 里的清单都不该被捡到。
    for noise in [".hidden", "target", "node_modules"] {
        fixture_in(&root, &format!("{noise}/nested"), manifest);
    }
    let root_arg = root.to_string_lossy().into_owned();

    // 目录路径 = 那个目录里的 course.json（不必写文件名）。
    let dir_only = run_course_args(&["course", &root.join("alpha").to_string_lossy(), "--json"]);
    assert!(
        dir_only.status.success(),
        "a directory path reads <dir>/course.json, stderr: {}",
        String::from_utf8_lossy(&dir_only.stderr)
    );
    let events = parse_lines(&String::from_utf8_lossy(&dir_only.stdout));
    assert_eq!(typed(&events, "course.unit").len(), 1, "{events:?}");

    // `--all`：两份真清单（alpha/beta），按路径字典序；干扰项不进。
    let out = run_course_args(&["course", "--all", &root_arg, "--json"]);
    assert!(
        out.status.success(),
        "--all must aggregate every manifest, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let events = parse_lines(&String::from_utf8_lossy(&out.stdout));
    let units = typed(&events, "course.unit");
    assert_eq!(units.len(), 2, "alpha + beta only: {events:?}");
    let tagged: Vec<&str> = units
        .iter()
        .map(|unit| unit.get("manifest").and_then(|v| v.as_str()).unwrap_or(""))
        .collect();
    assert!(
        tagged[0].ends_with("alpha/course.json") && tagged[1].ends_with("beta/course.json"),
        "discovery is sorted and skips .hidden/target/node_modules: {tagged:?}"
    );
    let summary = typed(&events, "course.summary")[0];
    assert_eq!(u64_field(summary, "manifests"), 2, "{summary}");

    // 同一份清单给两次（文件 + 目录）只算一次。
    let dup = run_course_args(&[
        "course",
        &root.join("alpha/course.json").to_string_lossy(),
        &root.join("alpha").to_string_lossy(),
        "--json",
    ]);
    let events = parse_lines(&String::from_utf8_lossy(&dup.stdout));
    assert_eq!(
        typed(&events, "course.unit").len(),
        1,
        "a duplicated manifest is reported once: {events:?}"
    );
    assert_eq!(
        u64_field(typed(&events, "course.summary")[0], "manifests"),
        1
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// 一批里有一份读不了 ⇒ 整体失败且**不发任何事件**（原子：不报半张表）；
/// `--all` 找不到任何清单同样是错误（不是"空报告成功"）。
#[test]
fn a_bad_manifest_in_the_batch_fails_the_whole_run() {
    let good = fixture("agg-good", r#"[{"file":"units/a.sokonanoda","unit":1}]"#);
    let bad = fixture("agg-bad", r#"{"schema":"soko.course/99","volumes":[]}"#);
    let good_path = manifest_path(&good);
    let bad_path = manifest_path(&bad);

    let out = run_course_args(&["course", &good_path, &bad_path, "--json"]);
    assert!(
        !out.status.success(),
        "an unreadable manifest in the batch must fail the run"
    );
    assert!(
        String::from_utf8_lossy(&out.stdout).is_empty(),
        "nothing is emitted when any manifest is refused: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains(&bad_path),
        "stderr names the refused manifest: {stderr}"
    );

    let empty = std::env::temp_dir().join(format!(
        "sokonanoda-course-all-empty-{}-{}",
        std::process::id(),
        CACHE_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&empty).expect("empty dir");
    let out = run_course_args(&["course", "--all", &empty.to_string_lossy()]);
    assert!(
        !out.status.success(),
        "--all with no manifest is an error, not an empty success"
    );
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("course.json"),
        "stderr says what was not found: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let _ = std::fs::remove_dir_all(&good);
    let _ = std::fs::remove_dir_all(&bad);
    let _ = std::fs::remove_dir_all(&empty);
}

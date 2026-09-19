//! `sokonanoda course` 认 `import`：有 `import` 的单元走**项目闭包**，与
//! `grade` / `query check` / `build` 用同一份闭包、同一个模块根、同一个摘要键
//! （WO-007 / 台账 G-06）。判据：
//!
//! 1. 夹具转绿：`course.unit` 为 `checked:1 / open:0 / failed:0`，summary `failed:0`；
//! 2. 差分不变式（**有清单**的单元）：`course.unit.failed == 0 ⇔ grade <该单元>
//!    exit 0`，且 `checked/open/reduced` 与该单元 `query check --compact` 的
//!    `decl_checked/exercise_open/expr_reduced` 一致（计数只取**入口**模块）；
//! 3. 负例不假绿：依赖（a）删掉、（b）有内核错误 ⇒ `failed > 0`；
//! 4. 模块根：入口最近的 `sokonanoda.toml`（**嵌套清单优先**），否则 `course.json`
//!    所在目录；相对清单路径与 cwd 无关（回退根只属于 `course`，那一条只对拍
//!    `query check --root`）；
//! 5. 缓存共用：course 冷跑写进闭包摘要 ⇒ 同一 cache dir 下 `build` 报 `hit`。

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::Value;

/// 仓库根（这个 crate 在 `<root>/crates/cli`）。
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root exists")
}

/// 入库夹具：`docs/gaps/repro/G06-course-import/`（`proj/` 是一个有清单的真项目）。
fn fixture_dir() -> PathBuf {
    repo_root().join("docs/gaps/repro/G06-course-import")
}

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// 每个测试一份独立空目录（缓存 dir / 临时课程）——互不串味。
fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sokonanoda-course-project-{tag}-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

/// 递归复制一个目录（std only：测试不许引第三方 crate）。
fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("create dir");
    for entry in std::fs::read_dir(from).expect("read dir") {
        let entry = entry.expect("dir entry");
        let target = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target).expect("copy file");
        }
    }
}

struct Run {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

/// 用真二进制跑一次命令（每个测试自己给 cache dir / cwd）。
fn run(cwd: &Path, cache: &Path, args: &[&str]) -> Run {
    let out = Command::new(env!("CARGO_BIN_EXE_sokonanoda"))
        .args(args)
        .current_dir(cwd)
        .env("SOKONANODA_CACHE_DIR", cache)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn sokonanoda");
    Run {
        code: out.status.code(),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

fn json_lines(stdout: &str) -> Vec<Value> {
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
        .unwrap_or_else(|| panic!("event missing integer {field:?}: {event}"))
}

/// 一次 `course <manifest> --json` 的 (unit, summary) 事件。
fn course_events(manifest: &Path, cwd: &Path, cache: &Path) -> (Value, Value) {
    let out = run(
        cwd,
        cache,
        &["course", manifest.to_str().expect("utf-8 path"), "--json"],
    );
    assert_eq!(
        out.code,
        Some(0),
        "course exits 0 even with failed units (progress is not an error)\nstderr: {}",
        out.stderr
    );
    let events = json_lines(&out.stdout);
    let units = typed(&events, "course.unit");
    assert_eq!(units.len(), 1, "one course.unit expected: {events:?}");
    let summaries = typed(&events, "course.summary");
    assert_eq!(
        summaries.len(),
        1,
        "one course.summary expected: {events:?}"
    );
    (units[0].clone(), summaries[0].clone())
}

/// `query check --compact` 的 `(decl_checked, exercise_open, expr_reduced)`。
fn query_counts(unit: &Path, cache: &Path, root: Option<&Path>) -> (u64, u64, u64) {
    let mut args = vec![
        "query",
        "check",
        "--file",
        unit.to_str().expect("utf-8 path"),
    ];
    if let Some(root) = root.map(|r| r.to_str().expect("utf-8 path")) {
        args.push("--root");
        args.push(root);
    }
    args.push("--compact");
    let out = run(&repo_root(), cache, &args);
    let value: Value = serde_json::from_str(out.stdout.trim()).unwrap_or_else(|e| {
        panic!(
            "query check must print one JSON object ({e}): {:?} / stderr {:?}",
            out.stdout, out.stderr
        )
    });
    let counts = &value["data"]["counts"];
    let field = |name: &str| {
        counts
            .get(name)
            .and_then(|v| v.as_u64())
            .unwrap_or_else(|| panic!("query check counts missing {name:?}: {value}"))
    };
    (
        field("decl_checked"),
        field("exercise_open"),
        field("expr_reduced"),
    )
}

/// 计数不变式：`course.unit` 的 checked/open/reduced 与 `query check --compact`
/// 的 decl_checked/exercise_open/expr_reduced 逐项一致。`root` = 显式模块根
/// （只在模块根**由 `course.json` 回退决定**时给出：`query` 的上溯发现只认
/// `sokonanoda.toml`，看不到课程清单）。
fn assert_counts_match_query(unit: &Path, counts: &Value, tag: &str, root: Option<&Path>) {
    let cache = temp_dir(&format!("{tag}-query"));
    let (checked, open, reduced) = query_counts(unit, &cache, root);
    assert_eq!(
        (
            u64_field(counts, "checked"),
            u64_field(counts, "open"),
            u64_field(counts, "reduced")
        ),
        (checked, open, reduced),
        "{tag}: course counts must mirror `query check` (decl_checked/exercise_open/expr_reduced)"
    );
}

/// 核心不变式（有清单的单元）：`course.unit.failed == 0` ⇔ `grade <该单元>` exit 0，
/// 且计数与 `query check --compact` 同源。有清单时三条通道的模块根发现完全一致，
/// 不需要任何旗标。
fn assert_matches_grade_and_query(unit: &Path, counts: &Value, tag: &str) {
    let cache = temp_dir(&format!("{tag}-differential"));
    let grade = run(
        &repo_root(),
        &cache,
        &["grade", unit.to_str().expect("utf-8 path")],
    )
    .code;

    let failed = u64_field(counts, "failed");
    assert_eq!(
        failed == 0,
        grade == Some(0),
        "{tag}: course.unit.failed ({failed}) must agree with `grade` exit code ({grade:?})"
    );

    assert_counts_match_query(unit, counts, tag, None);
}

#[test]
fn course_aggregates_an_importing_unit_through_the_project_closure() {
    let cache = temp_dir("fixed-cache");
    let (unit, summary) = course_events(&fixture_dir().join("course.json"), &repo_root(), &cache);

    assert_eq!(
        u64_field(&unit, "checked"),
        1,
        "the unit's own declaration is counted: {unit}"
    );
    assert_eq!(u64_field(&unit, "open"), 0, "no exercise open: {unit}");
    assert_eq!(u64_field(&unit, "failed"), 0, "import resolves: {unit}");
    assert_eq!(u64_field(&unit, "reduced"), 0, "no reductions: {unit}");
    assert!(
        unit.get("error").is_none(),
        "a healthy unit carries no error field: {unit}"
    );

    assert_eq!(u64_field(&summary, "units"), 1, "summary units");
    assert_eq!(u64_field(&summary, "checked"), 1, "summary checked");
    assert_eq!(u64_field(&summary, "open"), 0, "summary open");
    assert_eq!(u64_field(&summary, "failed"), 0, "summary failed");
}

#[test]
fn course_failed_agrees_with_grade_and_query_check() {
    let cache = temp_dir("fixture-cache");
    let unit_path = fixture_dir().join("proj/U4.sokonanoda");
    let (unit, _) = course_events(&fixture_dir().join("course.json"), &repo_root(), &cache);
    assert_matches_grade_and_query(&unit_path, &unit, "fixture");
}

#[test]
fn a_missing_dependency_is_not_a_silent_zero() {
    let dir = temp_dir("missing-dep");
    copy_dir(&fixture_dir(), &dir);
    std::fs::remove_file(dir.join("proj/Lib2.sokonanoda")).expect("remove the shared library");

    let cache = temp_dir("missing-dep-cache");
    let (unit, summary) = course_events(&dir.join("course.json"), &repo_root(), &cache);
    let failed = u64_field(&unit, "failed");
    assert!(
        failed > 0,
        "a unit whose import vanished must report failed > 0, got {unit}"
    );
    assert!(
        u64_field(&summary, "failed") > 0,
        "the summary must not be all zeros: {summary}"
    );
    assert_matches_grade_and_query(&dir.join("proj/U4.sokonanoda"), &unit, "missing-dep");
}

#[test]
fn a_broken_dependency_is_not_a_silent_zero() {
    let dir = temp_dir("broken-dep");
    copy_dir(&fixture_dir(), &dir);
    std::fs::write(
        dir.join("proj/Lib2.sokonanoda"),
        "axiom And2 : Prop -> Prop -> Prop\n\ntheorem bad : Prop := Nat.zero\n",
    )
    .expect("write a library with a kernel error");

    let cache = temp_dir("broken-dep-cache");
    let (unit, summary) = course_events(&dir.join("course.json"), &repo_root(), &cache);
    let failed = u64_field(&unit, "failed");
    assert!(
        failed > 0,
        "a unit whose dependency is rejected must report failed > 0, got {unit}"
    );
    assert!(
        u64_field(&summary, "failed") > 0,
        "the summary must not be all zeros: {summary}"
    );
    assert_matches_grade_and_query(&dir.join("proj/U4.sokonanoda"), &unit, "broken-dep");
}

/// 没有清单时模块根 = `course.json` 所在目录（课程布局 `<根>/{course.json,lib/,units/}`）。
/// 这是 `course` 独有的回退：`grade`/`query` 的上溯发现看不到 `course.json`，
/// 所以差分对拍用显式 `--root <课程根>`（WO-007 的已知边界）。
#[test]
fn module_root_falls_back_to_the_manifest_directory() {
    let dir = temp_dir("fallback-root");
    std::fs::create_dir_all(dir.join("lib")).expect("create lib");
    std::fs::create_dir_all(dir.join("units")).expect("create units");
    std::fs::write(
        dir.join("lib/Lib3.sokonanoda"),
        "theorem shared3 (A : Prop) (h : A) : A := h\n",
    )
    .expect("write lib");
    std::fs::write(
        dir.join("units/U1.sokonanoda"),
        "import lib.Lib3\n\ntheorem use3 (A : Prop) (h : A) : A := shared3 A h\n",
    )
    .expect("write unit");
    std::fs::write(
        dir.join("course.json"),
        r#"[{"file":"units/U1.sokonanoda","title":"fallback root","unit":1}]"#,
    )
    .expect("write manifest");

    let cache = temp_dir("fallback-root-cache");
    let (unit, summary) = course_events(&dir.join("course.json"), &repo_root(), &cache);
    assert_eq!(
        u64_field(&unit, "failed"),
        0,
        "`import lib.Lib3` must resolve against the course.json directory: {unit}"
    );
    assert_eq!(u64_field(&unit, "checked"), 1, "{unit}");
    assert_eq!(u64_field(&summary, "failed"), 0, "{summary}");
    // 回退根只属于 `course`：`grade`/`query` 的上溯发现看不到 `course.json`，
    // 所以这里只对拍 `query check --root <课程根>` 的计数（`grade` 的
    // `failed == 0` 同判在**有清单**的夹具与负例上钉，见上面三个用例）。
    assert_counts_match_query(
        &dir.join("units/U1.sokonanoda"),
        &unit,
        "fallback-root",
        Some(&dir),
    );
}

/// 单元自己的嵌套清单优先：位于子项目里的单元不会被 `course.json` 所在目录覆盖。
#[test]
fn a_nested_manifest_beats_the_course_root() {
    let dir = temp_dir("nested-manifest");
    std::fs::create_dir_all(dir.join("proj")).expect("create proj");
    // 课程根上的**同名诱饵**：若模块根误取 course.json 目录，`import Lib9` 会命中它。
    std::fs::write(
        dir.join("Lib9.sokonanoda"),
        "theorem decoy (A : Prop) (h : A) : A := h\n",
    )
    .expect("write decoy");
    std::fs::write(dir.join("proj/sokonanoda.toml"), "name = \"nested\"\n")
        .expect("write manifest");
    std::fs::write(
        dir.join("proj/Lib9.sokonanoda"),
        "theorem shared9 (A : Prop) (h : A) : A := h\n",
    )
    .expect("write nested lib");
    std::fs::write(
        dir.join("proj/Sub.sokonanoda"),
        "import Lib9\n\ntheorem use9 (A : Prop) (h : A) : A := shared9 A h\n",
    )
    .expect("write nested unit");
    std::fs::write(
        dir.join("course.json"),
        r#"[{"file":"proj/Sub.sokonanoda","title":"nested","unit":1}]"#,
    )
    .expect("write manifest");

    let cache = temp_dir("nested-manifest-cache");
    let (unit, _) = course_events(&dir.join("course.json"), &repo_root(), &cache);
    assert_eq!(
        u64_field(&unit, "failed"),
        0,
        "the nested sokonanoda.toml must win over the course.json directory: {unit}"
    );
    assert_eq!(u64_field(&unit, "checked"), 1, "{unit}");
}

/// 相对清单路径：仓库根与夹具目录（cwd）都必须绿。
#[test]
fn course_is_relative_path_and_cwd_independent() {
    let cache = temp_dir("relative-cache");
    let fixture = fixture_dir();

    let from_root = run(
        &repo_root(),
        &cache,
        &[
            "course",
            "docs/gaps/repro/G06-course-import/course.json",
            "--json",
        ],
    );
    assert_eq!(from_root.code, Some(0), "stderr: {}", from_root.stderr);
    let root_events = json_lines(&from_root.stdout);
    let units = typed(&root_events, "course.unit");
    assert_eq!(
        u64_field(units[0], "failed"),
        0,
        "relative path from the repo root must be green: {root_events:?}"
    );

    let from_fixture = run(&fixture, &cache, &["course", "course.json", "--json"]);
    assert_eq!(
        from_fixture.code,
        Some(0),
        "stderr: {}",
        from_fixture.stderr
    );
    let fixture_events = json_lines(&from_fixture.stdout);
    let units = typed(&fixture_events, "course.unit");
    assert_eq!(
        u64_field(units[0], "failed"),
        0,
        "relative path from the fixture dir must be green: {fixture_events:?}"
    );
}

/// 闭包摘要键与 `build` 共用：course 冷跑一次后，`build` 必须是 `hit`。
#[test]
fn course_shares_the_closure_cache_with_build() {
    let cache = temp_dir("shared-cache");
    let fixture = fixture_dir();
    let unit = fixture.join("proj/U4.sokonanoda");

    let (course_unit, _) = course_events(&fixture.join("course.json"), &repo_root(), &cache);
    assert_eq!(u64_field(&course_unit, "failed"), 0, "{course_unit}");

    let out = run(
        &repo_root(),
        &cache,
        &["build", "--json", unit.to_str().expect("utf-8 path")],
    );
    assert_eq!(out.code, Some(0), "stderr: {}", out.stderr);
    let events = json_lines(&out.stdout);
    let files = typed(&events, "build.file");
    assert_eq!(files.len(), 1, "one build.file expected: {events:?}");
    assert_eq!(
        files[0].get("status").and_then(|v| v.as_str()),
        Some("hit"),
        "course must have written the same closure digest `build` reads: {events:?}"
    );
}

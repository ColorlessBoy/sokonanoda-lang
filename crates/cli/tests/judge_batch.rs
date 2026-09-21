//! 乐观判定批处理（0.62.0 性能）的**端到端对拍**：`SOKO_NO_JUDGE_BATCH=1`
//! 关掉批处理之后，同一批文件的 `--json` 事件流必须**逐字节相同**。
//!
//! 为什么这条测试是这次性能改动的判据：批处理让 `by` 块里的判定"先记下来、
//! 跑完一次判完"，代价是引入了乐观路径（判定暂时返回 `Match`）。只要有一条
//! 判定不是 `Match`，`by::run_by` 就丢掉乐观结果、改用逐条判定的**严格重跑**。
//! 所以"开与关结果一模一样"正是这次改动**允许**的全部差别——事件流逐字节
//! 相同（含诊断的码/消息/span 与 warning），说明诊断没有因为乐观路径漂移。
//!
//! 覆盖三类输入：
//!   * 正常解答（判定全绿 —— 走乐观快路）；
//!   * 判定**失败**的解答（`exact` 类型不匹配 —— 走严格重跑，诊断必须与从前一致）；
//!   * 带开放练习的画布（`exercise.open` 事件，判定与洞混在一起）。

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn cache_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sokonanoda-judge-batch-{tag}-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

/// 判一个文件，返回 `(退出码, stdout)`。`batching` 决定是否带上关闭批处理的环境变量。
fn grade(path: &Path, batching: bool) -> (i32, String) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_sokonanoda"));
    command
        .args(["grade", "--json"])
        .arg(path)
        .env(
            "SOKONANODA_CACHE_DIR",
            cache_dir(if batching { "on" } else { "off" }),
        )
        .env("SOKONANODA_NO_CACHE", "1");
    if !batching {
        command.env("SOKO_NO_JUDGE_BATCH", "1");
    }
    let output = command.output().expect("spawn sokonanoda");
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).into_owned(),
    )
}

fn assert_same_both_ways(path: &Path, what: &str) {
    let (code_on, out_on) = grade(path, true);
    let (code_off, out_off) = grade(path, false);
    assert_eq!(
        code_on, code_off,
        "{what}：退出码在批处理开关下不同（{code_on} vs {code_off}）"
    );
    assert_eq!(
        out_on, out_off,
        "{what}：事件流在批处理开关下不同——\n--- 开 ---\n{out_on}\n--- 关 ---\n{out_off}"
    );
}

fn write_fixture(dir: &Path, name: &str, src: &str) -> PathBuf {
    std::fs::create_dir_all(dir).expect("mkdir");
    let path = dir.join(name);
    std::fs::write(&path, src).expect("write fixture");
    path
}

#[test]
fn a_correct_tactic_proof_grades_the_same_with_and_without_batching() {
    let dir = std::env::temp_dir().join(format!("soko-batch-ok-{}", std::process::id()));
    let path = write_fixture(
        &dir,
        "ok.sokonanoda",
        "theorem and_intro (A B : Prop) (h1 : A) (h2 : B) : A \u{2227} B := by\n  \
         have a : A := h1\n  \
         have b : B := h2\n  \
         exact \u{27e8}a, b\u{27e9}\n",
    );
    assert_same_both_ways(&path, "正常解答");
    let (code, out) = grade(&path, true);
    assert_eq!(code, 0, "应当判绿：{out}");
    assert!(out.contains("decl.checked"), "应当有 decl.checked：{out}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_failing_judgement_reports_the_same_diagnostic_with_and_without_batching() {
    // `exact h1` 的类型是 `A`，目标是 `B` ⇒ 判定不匹配。乐观那趟会记下这一问、
    // 由 flush 发现不是 Match ⇒ 严格重跑。两条路的**诊断**必须逐字相同
    // （码、消息、span 都在事件流里）。
    let dir = std::env::temp_dir().join(format!("soko-batch-bad-{}", std::process::id()));
    let path = write_fixture(
        &dir,
        "bad.sokonanoda",
        "theorem t (A B : Prop) (h1 : A) : B := by\n  exact h1\n",
    );
    assert_same_both_ways(&path, "判定失败");
    let (code, out) = grade(&path, true);
    assert_ne!(code, 0, "应当判红：{out}");
    assert!(out.contains("diagnostic"), "应当有诊断：{out}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_canvas_with_open_exercises_grades_the_same_with_and_without_batching() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../courses/set-theory/units");
    for name in [
        "unit01-sets-membership.sokonanoda",
        "notation-cheatsheet.sokonanoda",
    ] {
        let path = root.join(name);
        assert!(path.exists(), "找不到课程画布 {path:?}");
        assert_same_both_ways(&path, name);
    }
}

#[test]
fn a_solution_key_grades_the_same_with_and_without_batching() {
    let root =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../courses/set-theory/units/solutions");
    for name in ["unit01-solution.sokonanoda", "unit02-solution.sokonanoda"] {
        let path = root.join(name);
        assert!(path.exists(), "找不到解答钥匙 {path:?}");
        assert_same_both_ways(&path, name);
    }
}

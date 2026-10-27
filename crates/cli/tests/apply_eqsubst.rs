//! **HO1（metavar #16 的 HO 线）**：`apply Eq.subst (Eq.symm h)` 的门控目标测试。
//!
//! 现场 = `courses/set-theory/units/I.1/unit02-subsets-empty.sokonanoda` 练习 6
//! （`eq_empty_iff_forall_notMem`）的**第一问**（`→` 方向），设计见
//! `docs/design/metavar-pattern.md` §2/§4/§5：
//!
//! ```text
//! theorem eq_empty_iff_forall_notMem (α : Type) (A : Set α) :
//!     (A = ∅) → (∀ (x : α), ¬ (x ∈ A)) := by
//!   intro h x
//!   apply Eq.subst (Eq.symm h)      -- 期望：目标变 ¬ (x ∈ ∅)
//!   exact Set.notMem_empty x        -- 收口（收口项本身就是"目标形状"的证人）
//! ```
//!
//! **今天判红**（`elab-tactic-failed`："`apply` 的目标不匹配：… 的结果是 `False`，
//! 无法对齐当前目标 `¬ (x ∈ A)`"）：`apply` 剥穿 `Not` 之后拿 `False` 去对齐未展开
//! 的目标；结果是 2 条 diagnostic、0 个 `decl.checked`、退出码 1。
//!
//! **HO2 之后**必须整条 `decl.checked`、0 diagnostic：HO2 让 `apply` 在剥层时**早停**
//! 在未展开的结论 `¬ (x ∈ A)` 上 ⇒ 只剩子目标 `¬ (x ∈ ∅)`，`Set.notMem_empty x`
//! 正好是它的证明。反过来，若 `apply` 留下的不是 `¬ (x ∈ ∅)`，`exact` 必然报
//! "期望 `¬ (x ∈ A)`，实际是 `¬ (x ∈ ∅)`"（今天的第二条诊断就是它）⇒ **这一条
//! 就是"留下 negation goal"的判据**，不需要再问一次 `query state`。
//!
//! HO2 落地后**默认跑**（旧行为臂 `SOKO_APPLY_PREPEEL=0` 下仍红）。

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

/// 仓库根（这个 crate 在 `<root>/crates/cli`）。
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root exists")
}

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// **临时模块根** = `sokonanoda.toml`（让入口的根 = 本目录）+ `lib/`（课程库逐字
/// 拷贝 ⇒ `import lib.Set` 能解析）+ 入口。
///
/// 不写 `courses/**`（判卷不该碰课程目录），也不发明新 harness：`lib/` 的拷贝方式
/// 与 `crates/cli/tests/notation.rs` 的 `course_lib_dir` 同款，`_ = remove_dir_all`
/// 的清理在 `Drop` 里。
struct Fixture {
    dir: PathBuf,
    entry: PathBuf,
}

impl Fixture {
    fn new(tag: &str, body: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "sokonanoda-ho1-{tag}-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("lib")).expect("create temp module root");
        std::fs::write(dir.join("sokonanoda.toml"), "name = \"ho1-eqsubst\"\n")
            .expect("write manifest");
        let lib_src = repo_root().join("courses/set-theory/lib");
        for item in std::fs::read_dir(&lib_src).expect("read course lib") {
            let item = item.expect("dir entry");
            if item.path().extension().and_then(|e| e.to_str()) == Some("sokonanoda") {
                std::fs::copy(item.path(), dir.join("lib").join(item.file_name()))
                    .expect("copy lib file");
            }
        }
        let entry = dir.join("Canvas.sokonanoda");
        std::fs::write(&entry, body).expect("write entry");
        Self { dir, entry }
    }

    /// `sokonanoda grade --json <entry>`：先解析模块根（本目录的 `sokonanoda.toml`）
    /// 再走 `import` 闭包 ⇒ 返回 `(退出码, 事件流)`。
    ///
    /// 用 `grade --json`（而不是 `query check`）：判据要的就是 `decl.checked` 与
    /// "没有拒绝事件"这两件事，`grade --json` 的事件流直接给（`query check` 自带
    /// JSON，加 `--json` 反而是用法错误）。
    fn grade_json(&self) -> (i32, Vec<serde_json::Value>) {
        let out = Command::new(env!("CARGO_BIN_EXE_sokonanoda"))
            .args(["grade", "--json", self.entry.to_str().unwrap()])
            .env("SOKONANODA_CACHE_DIR", self.dir.join(".cache"))
            .output()
            .expect("run grade --json");
        let stream = String::from_utf8_lossy(&out.stdout);
        let events = stream
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| serde_json::from_str(line).expect("every stdout line is JSON"))
            .collect();
        (out.status.code().unwrap_or(-1), events)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// unit02 练习 6 的**第一问**（`→` 方向；`↔` 的第二问不在本片）。缩进与
/// 练习 6 的 `by` 块逐字一致（2 格），行尾换行保留（否则 `exact` 会被吃进上一行）。
const FORWARD: &str = concat!(
    "import lib.Set\n",
    "\n",
    "theorem eq_empty_iff_forall_notMem (α : Type) (A : Set α) :\n",
    "    (A = ∅) → (∀ (x : α), ¬ (x ∈ A)) := by\n",
    "  intro h x\n",
    "  apply Eq.subst (Eq.symm h)\n",
    "  exact Set.notMem_empty x\n",
);

/// **HO1 门控目标测试**：HO2 落地后默认跑（旧行为臂 `SOKO_APPLY_PREPEEL=0` 下仍红）。
#[test]
fn apply_eqsubst_leaves_negation_goal() {
    let fx = Fixture::new("forward", FORWARD);
    let (code, events) = fx.grade_json();

    let diagnostics: Vec<&serde_json::Value> = events
        .iter()
        .filter(|e| e["type"] == "diagnostic")
        .collect();
    assert!(
        diagnostics.is_empty(),
        "HO2 之后 `apply Eq.subst (Eq.symm h)` 必须留下 `¬ (x ∈ ∅)` 并由 \
         `exact Set.notMem_empty x` 收口（今天 2 条 elab-tactic-failed）；实际 diagnostic = {diagnostics:?}"
    );

    let checked: Vec<&serde_json::Value> = events
        .iter()
        .filter(|e| e["type"] == "decl.checked")
        .collect();
    assert_eq!(
        checked.len(),
        1,
        "第一问必须**判通过**（1 个 decl.checked），不是 exercise.open；事件流 = {events:?}"
    );
    assert_eq!(
        checked[0]["name"], "eq_empty_iff_forall_notMem",
        "判通过的是这条声明：{events:?}"
    );
    assert_eq!(
        code, 0,
        "0 diagnostic 的 `grade --json` 必须退出 0：{events:?}"
    );
}

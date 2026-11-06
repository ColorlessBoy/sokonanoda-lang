//! **issue #10**：`apply` 的部分应用（目标本身是 Π 时，Lean 4 `MVarId.apply`
//! 的 `i < n` 档）在**真课程库**上的门控目标测试。
//!
//! 现场 = `courses/set-theory/units/I.1/unit02-subsets-empty.sokonanoda` 练习 3：
//!
//! ```text
//! theorem subset_antisymm (α : Type) (A B : Set α)
//!     (h1 : A ⊆ B) (h2 : B ⊆ A) : A = B := by
//!   apply Set.ext
//!   intro x
//!   constructor
//!   apply h1        -- 目标 `x ∈ A → x ∈ B`：Lean 用 `h1 x` 直接关掉
//!   apply h2
//! ```
//!
//! 修前：两条 `elab-tactic-failed`（「`h1` 的结果是 `x ∈ D`，无法对齐当前目标
//! `A x → B x`」）⇒ exit 1。修后：整条 `decl.checked`、0 diagnostic、exit 0。
//!
//! 同一份夹具还钉住三件事：
//! 1. 方向错（`h2 : B ⊆ A` 在目标 `x ∈ A → x ∈ B`）**仍判红**（部分应用不许吞）；
//! 2. 无约束的层仍是**子目标**（`f : ∀ n, P → Q` 在目标 `P → Q` 上留 `⊢ Nat`），
//!    与 Lean 的 `forallMetaTelescope` 一致；
//! 3. 目标面板的记法：`apply Set.ext` 之后是 `x ∈ A ↔ x ∈ B`（不是 `A x ↔ B x`）
//!    —— `Set.ext` 的陈述用 `Set.mem` 写，显示层才折得回来。

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

/// 临时模块根 = `sokonanoda.toml` + 课程库逐字拷贝 + 入口（同
/// `apply_eqsubst.rs` 的夹具；不写 `courses/**`）。
struct Fixture {
    dir: PathBuf,
    entry: PathBuf,
}

impl Fixture {
    fn new(tag: &str, body: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "sokonanoda-apply-pi-{tag}-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("lib")).expect("create temp module root");
        std::fs::write(dir.join("sokonanoda.toml"), "name = \"apply-pi-goal\"\n")
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

    fn run(&self, args: &[&str]) -> std::process::Output {
        Command::new(env!("CARGO_BIN_EXE_sokonanoda"))
            .args(args)
            .env("SOKONANODA_CACHE_DIR", self.dir.join(".cache"))
            .output()
            .expect("run sokonanoda")
    }

    fn grade_json(&self) -> (i32, Vec<serde_json::Value>) {
        self.grade_json_with(&[])
    }

    /// 同 [`Self::grade_json`]，但给子进程加环境变量（逃生门的反向臂用）。
    fn grade_json_with(&self, envs: &[(&str, &str)]) -> (i32, Vec<serde_json::Value>) {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_sokonanoda"));
        cmd.args(["grade", "--json", self.entry.to_str().unwrap()])
            .env("SOKONANODA_CACHE_DIR", self.dir.join(".cache"));
        for (key, value) in envs {
            cmd.env(key, value);
        }
        let out = cmd.output().expect("run sokonanoda grade");
        (out.status.code().unwrap_or(-1), events(&out.stdout))
    }

    /// `query state --file <entry> --line L --col C` 的 `data`（「用户此刻看到的目标面板」）。
    fn state_at(&self, line: u32, col: u32) -> serde_json::Value {
        let out = self.run(&[
            "query",
            "state",
            "--file",
            self.entry.to_str().unwrap(),
            "--line",
            &line.to_string(),
            "--col",
            &col.to_string(),
        ]);
        assert!(
            out.status.success(),
            "query state 必须答得上（line {line} col {col}）：{}",
            String::from_utf8_lossy(&out.stderr)
        );
        serde_json::from_slice::<serde_json::Value>(&out.stdout).expect("query state is JSON")
            ["data"]
            .clone()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn events(stdout: &[u8]) -> Vec<serde_json::Value> {
    String::from_utf8_lossy(stdout)
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("every stdout line is JSON"))
        .collect()
}

fn diagnostics(events: &[serde_json::Value]) -> Vec<&serde_json::Value> {
    events
        .iter()
        .filter(|e| e["type"] == "diagnostic")
        .collect()
}

fn checked_names(events: &[serde_json::Value]) -> Vec<String> {
    events
        .iter()
        .filter(|e| e["type"] == "decl.checked")
        .map(|e| e["name"].as_str().unwrap_or_default().to_string())
        .collect()
}

/// 练习 3 的证明骨架（逐字照 hint 里给学习者的那条）+ 一行
/// `query state` 的落点标注（`intro x` 那一行 = `apply Set.ext` 之后的状态）。
const ANTISYMM: &str = concat!(
    "import lib.Set\n",
    "\n",
    "theorem subset_antisymm (α : Type) (A B : Set α)\n",
    "    (h1 : A ⊆ B) (h2 : B ⊆ A) : A = B := by\n",
    "  apply Set.ext\n",
    "  intro x\n",
    "  constructor\n",
    "  apply h1\n",
    "  apply h2\n",
);

#[test]
fn issue_10_skeleton_checks_against_the_real_course_library() {
    let fx = Fixture::new("antisymm", ANTISYMM);
    let (code, events) = fx.grade_json();
    assert!(
        diagnostics(&events).is_empty(),
        "issue #10 的骨架必须 0 diagnostic；实际 = {:?}",
        diagnostics(&events)
    );
    assert_eq!(
        checked_names(&events),
        vec!["subset_antisymm".to_string()],
        "整条 `decl.checked`（不是 exercise.open）：{events:?}"
    );
    assert_eq!(code, 0, "0 diagnostic 的 grade 必须 exit 0：{events:?}");
}

/// 目标面板：`intro x` 之后（光标在 `constructor` 那行 = `apply Set.ext; intro x`
/// 之后的状态）是 `x ∈ A ↔ x ∈ B`，且 `x`/`A`/`B`/`∈` 都是 `binder`/`keyword`
/// 着色（修前 `Set.ext` 陈述写 `A x ↔ B x` ⇒ 面板是 `A x ↔ B x`，`A`/`x` 被标成
/// `unknown_ident` —— 这就是「`ext` 的定义没有用到 `mem`」的可视后果）。
#[test]
fn set_ext_goal_keeps_membership_notation() {
    let fx = Fixture::new("goal", ANTISYMM);
    let state = fx.state_at(7, 3);
    assert_eq!(
        state["goal"], "x ∈ A ↔ x ∈ B",
        "`apply Set.ext; intro x` 之后的目标必须是点态成员关系：{state:?}"
    );
    let unknown: Vec<&serde_json::Value> = state["goal_runs"]
        .as_array()
        .expect("goal_runs")
        .iter()
        .filter(|run| run["kind"] == "unknown_ident")
        .collect();
    assert!(
        unknown.is_empty(),
        "成员关系的每个名字都该有着色（unknown_ident = 显示层认不出）：{unknown:?}"
    );
}

/// **反向验证（逃生门）**：`SOKO_APPLY_PARTIAL=0` ⇒ 逐字节回到改动前 —— 同一份
/// issue #10 骨架重新报「`apply` 的目标不匹配」（两条 `elab-tactic-failed`）。
#[test]
fn the_escape_hatch_restores_the_old_apply_error() {
    let fx = Fixture::new("escape", ANTISYMM);
    let (code, events) = fx.grade_json_with(&[("SOKO_APPLY_PARTIAL", "0")]);
    assert_eq!(code, 1, "关掉部分应用后骨架必须重新判红：{events:?}");
    let messages: Vec<&str> = diagnostics(&events)
        .iter()
        .filter_map(|d| d["message"].as_str())
        .collect();
    assert!(
        messages.iter().any(|m| m.contains("`apply` 的目标不匹配")),
        "旧诊断必须回来（apply 只做整条望远镜那一档）：{messages:?}"
    );
}

/// 部分应用的**方向**：`h2 : B ⊆ A` 在目标 `x ∈ A → x ∈ B` 上不成立 ⇒ 仍报
/// `apply` 的目标不匹配（部分应用不是「随便找一条 Π 假设试试」）。
#[test]
fn a_wrong_direction_is_still_an_apply_error() {
    let src = concat!(
        "import lib.Set\n",
        "\n",
        "theorem wrong (α : Type) (A B : Set α) (h2 : B ⊆ A) (x : α) : x ∈ A → x ∈ B := by\n",
        "  apply h2\n",
    );
    let fx = Fixture::new("wrong", src);
    let (code, events) = fx.grade_json();
    assert_eq!(code, 1, "方向错必须判红：{events:?}");
    let diags = diagnostics(&events);
    assert_eq!(diags.len(), 1, "只该有一条 apply 诊断：{diags:?}");
    let message = diags[0]["message"].as_str().unwrap_or_default();
    assert!(
        message.contains("`apply` 的目标不匹配"),
        "诊断要点名 apply：{message}"
    );
}

/// 部分应用的**子目标**：`f : ∀ n, P → Q` 在目标 `P → Q` 上，`n` 无约束 ⇒
/// Lean 留 `⊢ Nat`。这里用 `exact 0` 收掉它，整条判绿；把 `exact 0` 去掉则留下
/// 一个开目标（`exercise.open`）——两者都说明子目标真的存在。
#[test]
fn a_free_parameter_becomes_a_subgoal() {
    let closed = concat!(
        "theorem free_param (P Q : Prop) (f : ∀ (n : Nat), P → Q) : P → Q := by\n",
        "  apply f\n",
        "  exact 0\n",
    );
    let fx = Fixture::new("free-closed", closed);
    let (code, events) = fx.grade_json();
    assert_eq!(code, 0, "`exact 0` 收掉 `⊢ Nat` 后必须判绿：{events:?}");
    assert_eq!(checked_names(&events), vec!["free_param".to_string()]);

    let opened = concat!(
        "theorem free_param (P Q : Prop) (f : ∀ (n : Nat), P → Q) : P → Q := by\n",
        "  apply f\n",
    );
    let fx = Fixture::new("free-open", opened);
    let (_code, events) = fx.grade_json();
    assert!(
        events.iter().any(|e| e["type"] == "exercise.open"),
        "`n` 无约束 ⇒ 必须留下子目标（不是静默通过）：{events:?}"
    );
}

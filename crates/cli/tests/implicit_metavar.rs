//! **E19 刀2 的判据**：`solve_prefix` **一般路径**（应用 / 裸常量 / `by` 块里的
//! `apply`）上的**待定参数** —— 与刀1 **共用同一个开关** `SOKO_NOTATION_METAVAR`
//! （**2026-10-01 起默认开** —— 逃生门 `=0` / `off`）。
//!
//! 病根与 G-48 同形，只是入口不同：刀1 是**记法**的操作数位，刀2 是**应用**的
//! 实参位。`Set.Equiv {α β : Type} (A : Set α) (B : Set β)` 被写成
//! `Set.Equiv ∅ {b}` 时，`α` 只能从 `∅` 的类型解 —— 而 `∅` 自己又要靠**期望类型**
//! 才定得下论域 ⇒ 鸡生蛋 ⇒ `elab-implicit-argument-unsolved`（刀0 实测）。
//!
//! 四条判据（**缺一不算成立**）：
//! 1. **默认态**（不设开关）⇒ 该形状判绿（声明 elaborate 出来、无 diagnostic）✓；
//! 2. **反向验证**：逃生门 `=0` ⇒ **同一份源码**照旧判红（既有专用码还在）✓；
//! 3. **不猜**：两侧都是零元糖（`Set.Equiv ∅ ∅`）⇒ **没有同形的已解兄弟可依**
//!    ⇒ 默认态也判红 ✓；
//! 4. **逐条重审既有判据**：`crates/cli/tests/notation.rs` 的
//!    `an_unsolvable_implicit_argument_reports_its_own_code`（`{α}` 在实参类型与
//!    期望类型里**都不出现**）在**默认态**下**仍须**报同一个码 ✓ —— 刀2 只该
//!    "多解出能解的"，不该把"解不出"也放过去 ✗。

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::Value;

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn cache_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sokonanoda-implicit-metavar-cache-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn temp_file(tag: &str, text: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "sokonanoda-implicit-metavar-{tag}-{}-{}.sokonanoda",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::write(&path, text).expect("write fixture");
    path
}

/// `sokonanoda --json <file>`，可带 `SOKO_NOTATION_METAVAR`（`None` = **默认态** = 开）。
fn grade_json(path: &Path, metavar: Option<&str>) -> (i32, Vec<Value>) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_sokonanoda"));
    cmd.args(["--json", path.to_str().unwrap()])
        .env("SOKONANODA_CACHE_DIR", cache_dir());
    match metavar {
        Some(v) => cmd.env("SOKO_NOTATION_METAVAR", v),
        None => cmd.env_remove("SOKO_NOTATION_METAVAR"),
    };
    let out = cmd.output().expect("run --json");
    let stream = String::from_utf8_lossy(&out.stdout);
    let events = stream
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .collect();
    (out.status.code().unwrap_or(-1), events)
}

fn diagnostics(events: &[Value]) -> Vec<&Value> {
    events
        .iter()
        .filter(|e| e["type"] == "diagnostic")
        .collect()
}

fn has_code(events: &[Value], code: &str) -> bool {
    diagnostics(events).iter().any(|d| d["code"] == code)
}

/// **一般路径**的最小形状：签名是**隐式**的 `{α β}`，第一个实参是零元糖 `∅`。
const GENERAL_PATH: &str = "\
def Set (α : Type) : Type := α -> Prop
def Set.empty (α : Type) : Set α := fun (_ : α) => False
def Set.singleton (α : Type) (a : α) : Set α := fun (x : α) => x = a
def Set.Equiv {α β : Type} (A : Set α) (B : Set β) : Prop :=
  (forall (x : α), A x -> A x) ∧ (forall (y : β), B y -> B y)
notation \"∅\" => Set.empty
theorem app_shape (α β : Type) (b : β) : ¬ (Set.Equiv ∅ {b}) := by
  sorry
";

/// 两侧都是零元糖 ⇒ **一个兄弟都借不到** ⇒ 开关开也**不许**变绿。
const BOTH_HUNGRY: &str = "\
def Set (α : Type) : Type := α -> Prop
def Set.empty (α : Type) : Set α := fun (_ : α) => False
def Set.Equiv {α β : Type} (A : Set α) (B : Set β) : Prop :=
  (forall (x : α), A x -> A x) ∧ (forall (y : β), B y -> B y)
notation \"∅\" => Set.empty
theorem both_hungry (α β : Type) : ¬ (Set.Equiv ∅ ∅) := by
  sorry
";

/// **既有判据的同一份夹具**（`crates/cli/tests/notation.rs` 的
/// `an_unsolvable_implicit_argument_reports_its_own_code` 逐字相同）：
/// `{α}` 在任何显式实参的类型里、也不在期望类型里出现 ⇒ 永远解不出。
const UNSOLVABLE: &str = "\
def ignores {α : Type} (n : Nat) : Nat := n

def uses : Nat := ignores 3
";

#[test]
fn implicit_metavar_solves_the_application_shape_and_never_guesses() {
    // ① **默认态**（不设开关）⇒ 绿。
    let file = temp_file("general", GENERAL_PATH);
    let (code_on, events_on) = grade_json(&file, None);
    assert_eq!(
        code_on,
        0,
        "默认态（开关默认开）时 `Set.Equiv ∅ {{b}}` 必须判绿：{:?}",
        diagnostics(&events_on)
    );
    assert!(
        diagnostics(&events_on).is_empty(),
        "默认态不该还有诊断：{:?}",
        diagnostics(&events_on)
    );
    assert!(
        events_on
            .iter()
            .any(|e| e["type"] == "exercise.open" && e["name"] == "app_shape"),
        "声明本身必须 elaborate 出来（exercise.open）：{events_on:?}"
    );

    // ② **反向验证**：逃生门 `=0` ⇒ 同一份源码照旧判红（既有专用码）。
    let (code_off, events_off) = grade_json(&file, Some("0"));
    assert_eq!(
        code_off, 1,
        "`SOKO_NOTATION_METAVAR=0` 时必须与刀0 的基线一致（判红）：{events_off:?}"
    );
    assert!(
        has_code(&events_off, "elab-implicit-argument-unsolved"),
        "逃生门关掉时必须还是那条既有诊断：{:?}",
        diagnostics(&events_off)
    );

    // ③ **不猜**：两侧都是零元糖 ⇒ 没有同形兄弟 ⇒ 默认态也判红。
    let both = temp_file("both-hungry", BOTH_HUNGRY);
    let (code_both, events_both) = grade_json(&both, None);
    assert_eq!(
        code_both, 1,
        "`Set.Equiv ∅ ∅` 一个兄弟都借不到 ⇒ 不许变绿（不猜）：{events_both:?}"
    );
    assert!(
        has_code(&events_both, "elab-implicit-argument-unsolved"),
        "判红必须还是那条专用码：{:?}",
        diagnostics(&events_both)
    );

    // ④ **既有判据在默认态仍成立**（逐条重审；夹具与 `notation.rs` 逐字相同）。
    let unsolvable = temp_file("unsolvable", UNSOLVABLE);
    let (code_u, events_u) = grade_json(&unsolvable, None);
    assert_ne!(
        code_u, 0,
        "解不出的隐式参数在默认态**仍须**判红：{events_u:?}"
    );
    assert!(
        has_code(&events_u, "elab-implicit-argument-unsolved"),
        "默认态也必须报同一个专用码：{:?}",
        diagnostics(&events_u)
    );
}

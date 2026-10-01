//! **E19 刀1 的判据**：记法**操作数位**的**待定参数**（`SOKO_NOTATION_METAVAR`，
//! **2026-09-30 起默认开** —— 逃生门 `=0` / `off`）。
//!
//! 缺口 **G-48**：`∅ ≈ {b}` 的两个操作数**都是零元糖 / 集合字面量**——各自都要靠
//! **期望类型**才能定论域，而期望类型又要靠它们自己定 ⇒ 鸡生蛋 ⇒
//! `elab-notation-argument-unsolved`（复现件
//! `docs/gaps/repro/G48-notation-nullary-sugar-operands.sokonanoda`，**默认态必须绿** ✓）。
//!
//! 三条判据（**缺一不算成立**）：
//! 1. **默认态**（不设开关）⇒ 同形状的源码判卷变绿（`exit 0`、无 diagnostic）✓；
//! 2. **反向验证**：`SOKO_NOTATION_METAVAR=0`（逃生门）⇒ **同一份源码**照旧判红
//!    （既有诊断还在）✓ —— 咬不住"开关其实是假的"（默认态已经变了 ✗）；
//! 3. **不猜**：`∅ ≈ ∅`（两侧都定不出论域、**没有同形的已解兄弟可依**）在默认态
//!    **仍须判红** ✓ —— 待定参数只在"有一个已解的同形兄弟"时合一。
//!
//! 为什么放集成测试：开关是**进程级**环境变量、真二进制才读得到（与
//! `crates/front/tests/judge_inplace.rs` 同一个理由 ✓）。

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::Value;

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn cache_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sokonanoda-notation-metavar-cache-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn temp_file(tag: &str, text: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "sokonanoda-notation-metavar-{tag}-{}-{}.sokonanoda",
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

/// 缺口 G-48 的最小形状（与复现件**同签名**：`Set.Equiv` 的两个论域互相独立）。
const G48: &str = "\
def Set (α : Type) : Type := α -> Prop
def Set.empty (α : Type) : Set α := fun (_ : α) => False
def Set.singleton (α : Type) (a : α) : Set α := fun (x : α) => x = a
def Set.Equiv (α β : Type) (A : Set α) (B : Set β) : Prop :=
  (forall (x : α), A x -> A x) ∧ (forall (y : β), B y -> B y)
notation \"∅\" => Set.empty
infix:50 \" ≈ \" => Set.Equiv
theorem empty_not_equiv_singleton (α β : Type) (b : β) : ¬ (∅ ≈ {b}) := by
  sorry
";

/// 对照组：两侧都是零元糖 ⇒ **一个兄弟都借不到** ⇒ 开关开也**不许**变绿。
const BOTH_HUNGRY: &str = "\
def Set (α : Type) : Type := α -> Prop
def Set.empty (α : Type) : Set α := fun (_ : α) => False
def Set.Equiv (α β : Type) (A : Set α) (B : Set β) : Prop :=
  (forall (x : α), A x -> A x) ∧ (forall (y : β), B y -> B y)
notation \"∅\" => Set.empty
infix:50 \" ≈ \" => Set.Equiv
theorem empty_equiv_empty (α β : Type) : ¬ (∅ ≈ ∅) := by
  sorry
";

fn diagnostics(events: &[Value]) -> Vec<&Value> {
    events
        .iter()
        .filter(|e| e["type"] == "diagnostic")
        .collect()
}

#[test]
fn notation_metavar_flips_g48_green_and_never_guesses() {
    // ① **默认态**（不设开关）⇒ 绿（缺口修好的判据）。
    let file = temp_file("g48", G48);
    let (code_on, events_on) = grade_json(&file, None);
    assert_eq!(
        code_on,
        0,
        "默认态（开关默认开）时 `∅ ≈ {{b}}` 必须判绿：{:?}",
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
            .any(|e| e["type"] == "exercise.open" && e["name"] == "empty_not_equiv_singleton"),
        "声明本身必须 elaborate 出来（exercise.open）：{events_on:?}"
    );

    // ② **反向验证**：逃生门 `=0` ⇒ 同一份源码照旧判红（既有诊断在）。
    let (code_off, events_off) = grade_json(&file, Some("0"));
    assert_eq!(
        code_off, 1,
        "`SOKO_NOTATION_METAVAR=0` 时必须与刀0 的基线一致（判红）：{events_off:?}"
    );
    assert!(
        diagnostics(&events_off)
            .iter()
            .any(|d| d["code"] == "elab-notation-argument-unsolved"),
        "逃生门关掉时必须还是那条既有诊断：{:?}",
        diagnostics(&events_off)
    );

    // ③ **不猜**：两侧都是零元糖 ⇒ 没有同形兄弟可依 ⇒ 默认态也判红。
    let both = temp_file("both-hungry", BOTH_HUNGRY);
    let (code_both, events_both) = grade_json(&both, None);
    assert_eq!(
        code_both, 1,
        "`∅ ≈ ∅` 一个兄弟都借不到 ⇒ 不许变绿（不猜）：{events_both:?}"
    );
    assert!(
        !diagnostics(&events_both).is_empty(),
        "判红必须带诊断：{events_both:?}"
    );
}

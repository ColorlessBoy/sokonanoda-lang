//! **G3 的 release 档守卫**（2026-11-01）：**任何**用户可见诊断的正文都不许带内核内部名，
//! 且**带内部名的那条诊断绝不许落在 `0:0`**。
//!
//! 为什么单独一条扫描式判据：G3 的根因是 `pp_shape.rs:145` 在严格档把 `metas` 摘成 `None`
//! （`reports/2026-11-01-g3-meta-leak-rootcause.md`）⇒ 载体 `Ident("\0soko_m2")` 落到
//! `resolve_known` ⇒ 正文 `unknown identifier `\u{0}soko_m2``、**span `0:0`**（载体的
//! `Span::default()`）。修法那一行在 `debug_assert` 档另有牙（`pp_shape.rs`），但
//! **release 档只有这条扫描判据** ⇒ 它必须**咬得住修前的形状**：
//!
//! * 把 `pp_shape.rs` 的 `metas: ctx.metas` 改回 `metas: None` ⇒ 本文件的
//!   [`g3_meta_carrier_never_leaks_internal_names`] **必须当场红**（正文含 `\0` + span 0:0）；
//! * 修后 ⇒ 形状可能仍然判红（`∅ ≈ {b}` 在严格档是**推断边界**，与泄漏无关），但那条诊断
//!   必须**点名得了**：真码 + **真 span**（第 9 行的 `≈`），不是 `0:0`。
//!
//! ## 判据（两条，都是**用户可见**的东西，不是"字段存在"）
//!
//! 1. **正文卫生**：`message` / `hint` 不含 `\0`（NUL）—— 内部 fresh 名的**唯一**形状
//!    （`META_PREFIX = "\u{0}soko_m"`，`meta.rs:27`）⇒ 印出来就是泄漏；
//! 2. **span 卫生**：出现内部名（`\0` / `soko_m` / `soko_p`）的诊断，span **绝不许**是
//!    `0:0` —— `0:0` 在协议里是"没有位置"的哨兵（`Span::default()`）⇒ 用户点不过去、
//!    也看不出是哪一行；这两条一起把"内部名 + 无处可去"这个组合判死。
//!
//! ## 为什么夹具是一组而不是一条
//!
//! 泄漏发生在**多条**路（`elab_engine_value` 的显示形状重 elaborate、记法前导隐式、
//! `#check`/`#reduce` 的 `metas: None` base ctx —— 都是同一族）⇒ 只钉一条夹具的话，
//! 别的路再漏一次也不红。这里放一组**同一族**形状 × **两档方言**，逐条扫。
//!
//! 环境隔离照 `strict_diagnostics.rs`：每次 spawn 先 `env_remove` 掉所有跟判定有关的开关
//! （父环境不许改读数），再给一个**新的**缓存目录（命中的磁盘缓存会整份回放 ⇒ 判据绿得
//! 没道理 ✗），并关掉两处产物 ⇒ 不在仓库里落东西。

use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

static SEQ: AtomicUsize = AtomicUsize::new(0);

/// **跟判定有关的环境变量一律先摘掉**：外部设过任何一个，量到的就不是本用例那一档。
const JUDGE_ENV: [&str; 12] = [
    "SOKO_STRICT_AT",
    "SOKO_KERNEL_ARG_PP",
    "SOKO_META_PATTERN",
    "SOKO_META_APPROX",
    "SOKO_META_INVENTORY",
    "SOKO_METAVAR",
    "SOKO_NOTATION_METAVAR",
    "SOKO_DECL_MVARS",
    "SOKO_PP_SHAPE",
    "SOKO_EXPECTED_SPINE",
    "SOKO_KERNEL_APP",
    "SOKO_STRICT_FALLBACK",
];

/// 自足前奏（**零 import** ⇒ 模块根就是临时目录，没有闭包、没有产物）。
const PRELUDE: &str = "\
def Set (α : Type) : Type := α -> Prop
def Set.empty (α : Type) : Set α := fun (_ : α) => False
def Set.singleton (α : Type) (a : α) : Set α := fun (x : α) => x = a
def Set.Equiv {α β : Type} (A : Set α) (B : Set β) : Prop :=
  (forall (x : α), A x -> A x) ∧ (forall (y : β), B y -> B y)
def Set.mem {α : Type} (a : α) (A : Set α) : Prop := A a
def Set.subset {α : Type} (A : Set α) (B : Set α) : Prop := forall (x : α), A x -> B x
def Set.image {α β : Type} (f : α → β) (A : Set α) : Set β := fun (_ : β) => True
notation \"∅\" => Set.empty
infix:50 \" ≈ \" => Set.Equiv
infix:50 \" ∈ \" => Set.mem
infix:50 \" ⊆ \" => Set.subset
infix:80 \" '' \" => Set.image
";

/// **G3 的复现件**（报告 §Q1 的形状，逐字；第 9 行是 `theorem n3 …`）。
///
/// 修前：正文 `unknown identifier `\0soko_m2`` + span `0:0`。
const G3: &str = "\
def Set (α : Type) : Type := α -> Prop
def Set.empty (α : Type) : Set α := fun (x : α) => False
def Set.singleton (α : Type) (a : α) : Set α := fun (x : α) => x = a
def Set.Equiv {α β : Type} (A : Set α) (B : Set β) : Prop :=
  (forall (x : α), A x -> A x) ∧ (forall (y : β), B y -> B y)
notation \"∅\" => Set.empty
infix:50 \" ≈ \" => Set.Equiv

theorem n3 (β : Type) (b : β) : ¬ (∅ ≈ {b}) := by sorry
";

/// 一组**会产生诊断**的形状：内部名泄漏最容易藏在这些"要补隐式实参"的地方。
/// `(tag, 为什么在表里, 源码)`。
const FIXTURES: &[(&str, &str, &str)] = &[
    (
        "g3",
        "G3 本体：零 import、两侧都是零元糖的记法应用（载体在显示形状里重 elaborate）",
        G3,
    ),
    (
        "g3-swapped",
        "同族：`∅` 换到右边（求解顺序变了，载体还是那些）",
        "theorem a4 (β : Type) (b : β) : ¬ ({b} ≈ ∅) := by sorry\n",
    ),
    (
        "nullary-both-hungry",
        "G-48 的**不猜**对照组：两侧都定不出论域 ⇒ 两档都必须判红（正文不许漏名）",
        "theorem a3 (β : Type) : ¬ (∅ ≈ ∅) := by sorry\n",
    ),
    (
        "mem-prefix",
        "记法前导隐式（`∈` 的显式位不够 ⇒ 求解器补 `α`）",
        "theorem a1 (α : Type) (a : α) (A : Set α) : a ∈ A := by sorry\n",
    ),
    (
        "local-name-typo",
        "真未定义名（**不是**载体）⇒ 仍要报 unknown-identifier、span 必须是真的",
        "theorem a2 : Natx := by sorry\n",
    ),
];

/// 一次判卷（`grade --dialect <d> --json`）。
struct Run {
    ok: bool,
    events: Vec<Value>,
}

impl Run {
    fn diagnostics(&self) -> Vec<&Value> {
        self.events
            .iter()
            .filter(|e| e.get("type").and_then(Value::as_str) == Some("diagnostic"))
            .collect()
    }
}

fn scratch(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64)
        .unwrap_or(0);
    let dir =
        std::env::temp_dir().join(format!("g3-leak-{}-{tag}-{nanos}-{n}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("建临时目录");
    dir
}

fn grade(tag: &str, src: &str, dialect: &str) -> Run {
    let dir = scratch(tag);
    let path = dir.join("Probe.sokonanoda");
    std::fs::write(&path, src).expect("write fixture");
    let cache = dir.join("cache");
    std::fs::create_dir_all(&cache).expect("cache dir");

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_sokonanoda"));
    for key in JUDGE_ENV {
        cmd.env_remove(key);
    }
    cmd.env("SOKONANODA_CACHE_DIR", &cache);
    cmd.env("SOKONANODA_NO_PROJECT_ARTIFACTS", "1");
    cmd.env("SOKONANODA_NO_MODULE_ARTIFACTS", "1");
    cmd.args(["grade", "--dialect", dialect]);
    cmd.arg(&path).arg("--json");
    let out: Output = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run sokonanoda grade");
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let events: Vec<Value> = text
        .lines()
        .filter(|line| line.trim_start().starts_with('{'))
        .map(|line| serde_json::from_str(line).expect("event JSON"))
        .collect();
    let _ = std::fs::remove_dir_all(&dir);
    Run {
        ok: out.status.success(),
        events,
    }
}

fn text_of<'a>(diag: &'a Value, key: &str) -> &'a str {
    diag.get(key).and_then(Value::as_str).unwrap_or("")
}

/// 内部名的三个形状：NUL（fresh 名 `\0soko_m*`）、`soko_m` / `soko_p`（前缀被剥掉时剩下的那半）。
fn internal_name(text: &str) -> Option<&'static str> {
    if text.contains('\0') {
        return Some("\\0（NUL）");
    }
    if text.contains("soko_m") {
        return Some("soko_m");
    }
    if text.contains("soko_p") {
        return Some("soko_p");
    }
    None
}

/// **判据 2 的原料**：`0:0` = "没有位置"的哨兵（`Span::default()`），不是真位置。
fn is_zero_span(diag: &Value) -> bool {
    let pos = |section: &str| {
        (
            diag["span"][section]["line"].as_u64().unwrap_or(1),
            diag["span"][section]["column"].as_u64().unwrap_or(1),
        )
    };
    pos("start") == (0, 0) && pos("end") == (0, 0)
}

/// **判据 1 + 2 的合成扫描**：一次运行的所有诊断逐条过两道闸。
fn scan(tag: &str, dialect: &str, run: &Run) {
    for diag in run.diagnostics() {
        let code = text_of(&diag, "code");
        let message = text_of(&diag, "message");
        let hint = text_of(&diag, "hint");
        for (what, text) in [("message", message), ("hint", hint)] {
            if let Some(shape) = internal_name(text) {
                // ① 正文卫生：内部名**一律**不许进用户文案（REQUIREMENTS §9.39 的机械判据）。
                panic!(
                    "[{tag}/{dialect}] `{code}` 的 {what} 里印了内核内部名（{shape}）：{text:?}\n\
                     诊断全文：{diag}"
                );
            }
        }
        // ② span 卫生：真诊断必须指得到源里的一行（`line == 0` 就是 `Span::default()` 的哨兵）。
        //    修前的 G3 正是 "内部名 + span 0:0" 这个组合 —— 这里把它单独判死。
        let mentions_internal = ["\0", "soko_m", "soko_p"]
            .iter()
            .any(|n| message.contains(n) || hint.contains(n));
        if mentions_internal && is_zero_span(diag) {
            panic!(
                "[{tag}/{dialect}] `{code}` 既漏内部名、span 又是 0:0（载体 `Span::default()` 的形状）：{diag}"
            );
        }
        if code != "-" && diag["span"]["start"]["line"].as_u64() == Some(0) {
            panic!(
                "[{tag}/{dialect}] `{code}` 的 span 落在 0:0（= 没有位置）⇒ 用户点不过去、\
                 也看不出是哪一行：{diag}"
            );
        }
    }
}

/// **G3 的主判据**：复现件的两档都不许把内部名漏到用户文案，且**判红时 span 必须是真的**。
#[test]
fn g3_meta_carrier_never_leaks_internal_names() {
    for dialect in ["loose", "strict"] {
        let run = grade(&format!("g3-{dialect}"), G3, dialect);
        scan("g3", dialect, &run);
        // 判红可以（严格档在 `∅ ≈ {b}` 上是**推断边界**，与泄漏无关）——但那条诊断必须
        // **点名得了**：正文里要有记法/隐式实参这类**用户写得出来**的字眼，不是 `\0soko_m2`。
        if !run.ok {
            let diags = run.diagnostics();
            assert!(
                !diags.is_empty(),
                "[g3/{dialect}] 判红了却没有 diagnostic ⇒ 判据无从归属"
            );
            for diag in &diags {
                let code = text_of(diag, "code");
                assert_ne!(
                    code, "elab-unknown-identifier",
                    "[g3/{dialect}] 复现件报 `unknown identifier`（修前那一格的码）⇒ 载体又漏到 \
                     `resolve_known` 了：{diag}"
                );
                // 第 9 行是 `theorem n3 …`（1-based）⇒ span 必须落在那一行，不许是 0。
                assert_eq!(
                    diag["span"]["start"]["line"].as_u64(),
                    Some(9),
                    "[g3/{dialect}] `{code}` 的 span 不在第 9 行（`theorem n3 …`）⇒ 不是使用处的位置：{diag}"
                );
            }
        }
    }
}

/// **扫描面**：一组同族形状 × 两档，逐条过 [`scan`] 的两道闸。
///
/// 只钉"正文与 span 的卫生"，**不钉**接受/拒绝 —— 判红/判绿都是**判定**的结果，
/// 而"别把内部名印出来"是**文案**的契约，两者不是一个判据（钉死前者会让无关的推断改动变红）。
#[test]
fn every_diagnostic_is_free_of_internal_names_and_has_a_real_span() {
    for (tag, _why, body) in FIXTURES {
        for dialect in ["loose", "strict"] {
            let src = if body.starts_with("theorem a2 : Natx") {
                // 真未定义名那条**不需要**前奏（有前奏反而多四条 checked 事件，没意义）。
                (*body).to_string()
            } else {
                format!("{PRELUDE}{body}")
            };
            let run = grade(&format!("{tag}-{dialect}"), &src, dialect);
            scan(tag, dialect, &run);
        }
    }
}

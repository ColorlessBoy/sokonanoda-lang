//! **M0 的能力清单**（IA-4 元参数引擎，设计 `docs/design/metavar-engine.md` §4 M0；
//! 读数与结论 ⇒ `docs/design/metavar-m0.md`）。
//!
//! **这是什么**：13 个**形状** × **两个开关态**（默认态 = E19 待定档开 / 逃生门
//! `SOKO_NOTATION_METAVAR=0` = 严格档）的**实测分类**，每个形状归入三态之一：
//!
//! | 类别 | 判据（两个开关态） | 含义 |
//! |---|---|---|
//! | **可解** | 两态都绿 | 严格档就解得出来（元变量**不参与**）|
//! | **依赖默认** | 严格档红 / 默认态绿 | 靠 E19 的「同形已解兄弟 ⇒ 取它的值」**选择规则**才绿 |
//! | **解不出** | 两态都红 | 没有类型来源（或**约束冲突**）⇒ 不猜 |
//!
//! **为什么它同时是 M0 的交付与 M1 的红线**：M1（引擎内核）落地时，**`sibling` 态必须
//! 逐字节等于今天** ⇒ 本表**每一条**（退出码 + 诊断码）都是回归判据；哪一条变了就要
//! 逐条重审（设计 §2.6 的十二条同款纪律）。
//!
//! ⚠ **夹具是内联常量、不是 `.sokonanoda` 文件** ✗：`scripts/kernel-diff.sh` 的**非课程
//! 语料**是 `find course examples docs/gaps/repro -name '*.sokonanoda'`（实测 `kernel-diff.sh:109`）
//! ⇒ 往 `docs/gaps/repro/` 放新夹具会**改语料计数与三个 sha256 指纹** ✗（M0 的红线正是
//! 「指纹逐字节不变」）⇒ 夹具只能内联（与 `crates/cli/tests/implicit_metavar.rs` 同一形制）✓。

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::Value;

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn cache_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sokonanoda-metavar-inventory-cache-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn temp_file(tag: &str, text: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "sokonanoda-metavar-inventory-{tag}-{}-{}.sokonanoda",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::write(&path, text).expect("write fixture");
    path
}

/// `sokonanoda --json <file>`；`metavar = None` ⇒ **默认态**（不设开关），
/// `Some("0")` ⇒ **逃生门**（严格档）。
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

/// 唯一的诊断码（`None` = 没有诊断）；多个不同码 ⇒ 用 `+` 连起来（本表里没出现）。
fn code_of(events: &[Value]) -> Option<String> {
    let mut codes: Vec<String> = events
        .iter()
        .filter(|e| e["type"] == "diagnostic")
        .filter_map(|e| e["code"].as_str().map(str::to_string))
        .collect();
    codes.sort();
    codes.dedup();
    if codes.is_empty() {
        None
    } else {
        Some(codes.join("+"))
    }
}

/// 公共前奏：**自足**（不 import，跑得快、互不干扰）。
const PRELUDE: &str = "\
def Set (α : Type) : Type := α -> Prop
def Set.empty (α : Type) : Set α := fun (_ : α) => False
def Set.singleton (α : Type) (a : α) : Set α := fun (x : α) => x = a
def Set.Equiv {α β : Type} (A : Set α) (B : Set β) : Prop :=
  (forall (x : α), A x -> A x) ∧ (forall (y : β), B y -> B y)
def Set.mem {α : Type} (a : α) (A : Set α) : Prop := A a
def Set.image {α β : Type} (f : α → β) (A : Set α) : Set β := fun (_ : β) => True
def Set.univ {α : Type} : Set α := fun (_ : α) => True
def picks {α : Type} (n : Nat) (b : α) : Prop := True
notation \"∅\" => Set.empty
infix:50 \" ≈ \" => Set.Equiv
infix:50 \" ∈ \" => Set.mem
infix:80 \" '' \" => Set.image
";

/// 一个形状：名字 / 机制（走哪条求解路线）/ 期望分类 / 两态的**实测**（退出码 + 诊断码）。
struct Shape {
    name: &'static str,
    /// 这条形状压的是哪条路线（读 `implicit.rs` / `elab.rs` 得来）。
    mechanism: &'static str,
    class: Class,
    body: &'static str,
    default_state: (i32, Option<&'static str>),
    strict_state: (i32, Option<&'static str>),
}

#[derive(PartialEq, Eq, Debug)]
enum Class {
    /// 两态都绿：严格档解得出来（元变量不参与）。
    Solvable,
    /// 严格档红、默认态绿：靠「同形已解兄弟 ⇒ 取它的值」这条**选择规则**。
    NeedsDefault,
    /// 两态都红：没有类型来源，或**约束冲突**。
    Unsolvable,
}

/// 13 个形状（顺序 = 可解 → 依赖默认 → 解不出；**不是**按文件顺序跑）。
const SHAPES: &[Shape] = &[
    Shape {
        name: "S01_notation_mem_first_explicit",
        mechanism: "记法路线①：`∈` 的前导 α 由第一个显式操作数 `a` 的类型解出",
        class: Class::Solvable,
        body: "theorem s01 (α : Type) (a : α) (A : Set α) : a ∈ A := by sorry",
        default_state: (0, None),
        strict_state: (0, None),
    },
    Shape {
        name: "S02_app_scans_later_layer",
        mechanism: "应用路线①：`α` 只在**第二个**显式层（`b : α`）出现 ⇒ 扫全部显式层",
        class: Class::Solvable,
        body: "theorem s02 (α : Type) (b : α) : picks 3 b := by sorry",
        default_state: (0, None),
        strict_state: (0, None),
    },
    Shape {
        name: "S03_app_expected_type",
        mechanism: "应用路线②：`Or.inl` 的 `B` 只出现在结果类型里 ⇒ 由期望类型解出",
        class: Class::Solvable,
        body: "theorem s03 (h : 1 = 1) : Or (1 = 1) (2 = 2) := Or.inl h",
        default_state: (0, None),
        strict_state: (0, None),
    },
    Shape {
        name: "S04_bare_constant_expected",
        mechanism: "裸常量路线②（G-40）：`∅` 是光秃秃的 `Set.empty`（不是 App）⇒ 由期望类型解出",
        class: Class::Solvable,
        body: "def s04 : Set Nat := ∅",
        default_state: (0, None),
        strict_state: (0, None),
    },
    Shape {
        name: "S05_notation_arrow_domain",
        mechanism: "记法路线① 的 Pi 档：`α`/`β` 只在 `f : α → β` 的域/陪域里",
        class: Class::Solvable,
        body: "theorem s05 (α β : Type) (f : α → β) (A : Set α) : f '' A = f '' A := by sorry",
        default_state: (0, None),
        strict_state: (0, None),
    },
    Shape {
        name: "S13_two_constraints_defeq_agree",
        mechanism: "**两条约束 defeq 一致**：`Set Nat` 与 `?α → Prop`（`Set` 是 def）—— \
                    今天只查第一条（route ① 首个命中即返回）⇒ 绿；**M1 必须复用 delta 兜底**，\
                    否则引擎会把它误判成刚性冲突 ✗",
        class: Class::Solvable,
        body: "def F {α : Type} (A : Set α) (g : α → Prop) : Prop := True\n\
               def s13 (A : Set Nat) : Prop := F A A",
        default_state: (0, None),
        strict_state: (0, None),
    },
    Shape {
        name: "S14_route3_surplus_arg",
        mechanism: "路线③（G-41）：只有隐式 binder 的常量被应用 ⇒ 富余实参落到**结果**上",
        class: Class::Solvable,
        body: "theorem s14 (α : Type) (x : α) : Set.univ x := by sorry",
        default_state: (0, None),
        strict_state: (0, None),
    },
    Shape {
        name: "S06_g48_notation_both_nullary",
        mechanism: "记法 + 待定档（G-48）：`{b}` 定出 `β`，`α` **无任何约束** ⇒ 同形兄弟 `β`",
        class: Class::NeedsDefault,
        body: "theorem s06 (β : Type) (b : β) : ¬ (∅ ≈ {b}) := by sorry",
        default_state: (0, None),
        strict_state: (1, Some("elab-notation-argument-unsolved")),
    },
    Shape {
        name: "S07_g48_app_both_nullary",
        mechanism: "应用 + 待定档（E19 刀2）：与 S06 同形，入口是**应用**的实参位",
        class: Class::NeedsDefault,
        body: "theorem s07 (β : Type) (b : β) : ¬ (Set.Equiv ∅ {b}) := by sorry",
        default_state: (0, None),
        strict_state: (1, Some("elab-implicit-argument-unsolved")),
    },
    Shape {
        name: "S09_operand_order_reversed",
        mechanism: "顺序无关（今天）：已解的一侧在后 ⇒ 待定位仍借得到它 ⇒ 与 S07 同判",
        class: Class::NeedsDefault,
        body: "theorem s09 (α : Type) (a : α) : ¬ (Set.Equiv {a} ∅) := by sorry",
        default_state: (0, None),
        strict_state: (1, Some("elab-implicit-argument-unsolved")),
    },
    Shape {
        name: "S10_both_hungry",
        mechanism: "不猜：两侧都是零元糖 ⇒ 一个**已解**兄弟都没有 ⇒ 两态都红",
        class: Class::Unsolvable,
        body: "theorem s10 (α β : Type) : ¬ (Set.Equiv ∅ ∅) := by sorry",
        default_state: (1, Some("elab-implicit-argument-unsolved")),
        strict_state: (1, Some("elab-implicit-argument-unsolved")),
    },
    Shape {
        name: "S11_param_never_mentioned",
        mechanism: "不猜：`α` 在任何实参类型与期望类型里都不出现（既有判据的同一夹具）",
        class: Class::Unsolvable,
        body: "def ignores {α : Type} (n : Nat) : Nat := n\n\
               def s11 : Nat := ignores 3",
        default_state: (1, Some("elab-implicit-argument-unsolved")),
        strict_state: (1, Some("elab-implicit-argument-unsolved")),
    },
    Shape {
        name: "S12_conflicting_constraints",
        mechanism:
            "**冲突**：`A : Set Nat` 与 `B : Set Bool` 要求**同一个** `α` —— 今天取首个命中、\
                    第二条约束**不检查** ⇒ 拖到**内核**才拒（`kernel-rejected`）；\
                    引擎应在 elab 期检出 clash（判定不变、位置与码更准）",
        class: Class::Unsolvable,
        body: "def Two {α : Type} (A : Set α) (B : Set α) : Prop := True\n\
               def s12 (A : Set Nat) (B : Set Bool) : Prop := Two A B",
        default_state: (1, Some("kernel-rejected")),
        strict_state: (1, Some("kernel-rejected")),
    },
];

/// **M0 的能力清单**（一个 `#[test]`：13 形状 × 2 态 = 26 次真进程）。
///
/// 断言三件事（缺一不算成立）：
/// 1. **分类**：`可解` / `依赖默认` / `解不出` 与本表逐条一致；
/// 2. **报错契约**：红的那一态的**诊断码逐条相同**（M1 不许换码，设计 §2.6）；
/// 3. **两态关系**：`依赖默认` 必须严格档红 + 默认态绿（证明那条选择规则**真的在起作用**，
///    而不是"开关是假的"——与 `{notation,implicit}_metavar.rs` 的反向验证同一口径）。
#[test]
fn capability_inventory_is_measured_and_pinned() {
    let mut solved = 0usize;
    let mut needs_default = 0usize;
    let mut unsolvable = 0usize;
    for shape in SHAPES {
        let file = temp_file(shape.name, &format!("{PRELUDE}{}\n", shape.body));
        let (default_exit, default_events) = grade_json(&file, None);
        let (strict_exit, strict_events) = grade_json(&file, Some("0"));
        let default_code = code_of(&default_events);
        let strict_code = code_of(&strict_events);
        let got = match (default_exit, strict_exit) {
            (0, 0) => Class::Solvable,
            (0, _) => Class::NeedsDefault,
            _ => Class::Unsolvable,
        };
        assert_eq!(
            got, shape.class,
            "{}（{}）：分类不符 —— 默认态 exit={default_exit} code={default_code:?} · \
             严格档 exit={strict_exit} code={strict_code:?}",
            shape.name, shape.mechanism
        );
        assert_eq!(
            (default_exit, default_code.as_deref()),
            (shape.default_state.0, shape.default_state.1),
            "{}：**默认态**（E19 待定档开）的退出码/诊断码变了 ✗ —— 这是 M1 的红线",
            shape.name
        );
        assert_eq!(
            (strict_exit, strict_code.as_deref()),
            (shape.strict_state.0, shape.strict_state.1),
            "{}：**严格档**（`SOKO_NOTATION_METAVAR=0`）的退出码/诊断码变了 ✗ —— 这是 M1 的红线",
            shape.name
        );
        match got {
            Class::Solvable => solved += 1,
            Class::NeedsDefault => needs_default += 1,
            Class::Unsolvable => unsolvable += 1,
        }
    }
    assert_eq!(SHAPES.len(), 13, "能力清单的形状数（设计 §4 M0：~12）");
    assert_eq!(
        (solved, needs_default, unsolvable),
        (7, 3, 3),
        "三态分布变了 ⇒ `docs/design/metavar-m0.md` 的结论要重跑"
    );
}

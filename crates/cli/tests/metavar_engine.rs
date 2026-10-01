//! **IA-4 M1 的判据**：元参数引擎（`crate::compile::meta`）+ 记法路径接线 + 开关三态。
//!
//! 设计 ⇒ `docs/design/metavar-engine.md` §2/§4；M0 的基线与 13 形状清单 ⇒
//! `docs/design/metavar-m0.md`。本文件只测 **M1 的接线**（记法路径），一般路径（应用/裸常量/
//! 路线③）归 **M2**。
//!
//! **四条判据**（缺一不算成立）：
//! 1. **默认档 = `sibling`**：不设开关 ≡ `SOKO_METAVAR=sibling`（逐条同判）；
//! 2. **旧逃生门仍等价**：`SOKO_METAVAR=0` ≡ `SOKO_NOTATION_METAVAR=0`（都是严格档）；
//! 3. **引擎不许丢解**（M1 的硬红线）：`sibling` 绿的形状，`engine` **必须也绿**；
//! 4. **引擎的判定逐条钉住**：五个记法形状 × 三档的（退出码 + 诊断码）写死在表里 ——
//!    M0 的预测是「接受面增量 0」⇒ 引擎档与窄版档**逐条同判**；哪天不一样了，本判据当场红 ✓。
//!
//! ⚠ **每个 (形状, 档位) 用独立缓存目录**：编译缓存的键**不含**开关时，同一个目录里先跑的那一档
//! 会污染后面所有档（release `v0.79.0` 实测复现 ✗）—— M1 已把档位字节加进键
//! （`cache::metavar_state`），这里仍按最保守的口径各用各的目录 ✓。

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::Value;

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn cache_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sokonanoda-metavar-engine-{tag}-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn temp_file(tag: &str, text: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "sokonanoda-metavar-engine-{tag}-{}-{}.sokonanoda",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::write(&path, text).expect("write fixture");
    path
}

/// 一次判卷：`envs` 是**完整**的环境覆盖（`SOKO_METAVAR` 等），返回（退出码, 诊断码集合）。
fn grade(path: &Path, tag: &str, envs: &[(&str, &str)]) -> (i32, String) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_sokonanoda"));
    cmd.args(["--json", path.to_str().unwrap()])
        .env("SOKONANODA_CACHE_DIR", cache_dir(tag))
        .env_remove("SOKO_METAVAR")
        .env_remove("SOKO_NOTATION_METAVAR");
    for (k, v) in envs {
        cmd.env(k, v);
    }
    let out = cmd.output().expect("run --json");
    let stream = String::from_utf8_lossy(&out.stdout);
    let events: Vec<Value> = stream
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .collect();
    let mut codes: Vec<String> = events
        .iter()
        .filter(|e| e["type"] == "diagnostic")
        .filter_map(|e| e["code"].as_str().map(str::to_string))
        .collect();
    codes.sort();
    codes.dedup();
    (
        out.status.code().unwrap_or(-1),
        if codes.is_empty() {
            "-".to_string()
        } else {
            codes.join("+")
        },
    )
}

/// 公共前奏（自足，不 import）。
const PRELUDE: &str = "\
def Set (α : Type) : Type := α -> Prop
def Set.empty (α : Type) : Set α := fun (_ : α) => False
def Set.singleton (α : Type) (a : α) : Set α := fun (x : α) => x = a
def Set.Equiv {α β : Type} (A : Set α) (B : Set β) : Prop :=
  (forall (x : α), A x -> A x) ∧ (forall (y : β), B y -> B y)
def Set.mem {α : Type} (a : α) (A : Set α) : Prop := A a
def Set.image {α β : Type} (f : α → β) (A : Set α) : Set β := fun (_ : β) => True
def Set.subset {α : Type} (A : Set α) (B : Set α) : Prop := forall (x : α), A x -> B x
notation \"∅\" => Set.empty
infix:50 \" ≈ \" => Set.Equiv
infix:50 \" ∈ \" => Set.mem
infix:50 \" ⊆ \" => Set.subset
infix:80 \" '' \" => Set.image
";

/// 一个**记法路径**形状：名字 / 为什么它在表里 / 四档的实测（退出码, 诊断码）。
struct Shape {
    name: &'static str,
    why: &'static str,
    body: &'static str,
    default_state: (i32, &'static str),
    sibling: (i32, &'static str),
    engine: (i32, &'static str),
    strict: (i32, &'static str),
}

/// 五个形状（覆盖记法路径的两条路线 + 待定档 + 冲突 + 顺序无关）。
const SHAPES: &[Shape] = &[
    Shape {
        name: "N1_mem_first_explicit",
        why: "记法路线①（首个显式操作数的类型）：三档都解得出来 ⇒ 引擎不参与",
        body: "theorem n1 (α : Type) (a : α) (A : Set α) : a ∈ A := by sorry",
        default_state: (0, "-"),
        sibling: (0, "-"),
        engine: (0, "-"),
        strict: (0, "-"),
    },
    Shape {
        name: "N2_image_arrow",
        why: "记法路线① 的 Pi 档（`α`/`β` 在 `f : α → β` 的域/陪域里）",
        body: "theorem n2 (α β : Type) (f : α → β) (A : Set α) : f '' A = f '' A := by sorry",
        default_state: (0, "-"),
        sibling: (0, "-"),
        engine: (0, "-"),
        strict: (0, "-"),
    },
    Shape {
        name: "N3_g48_both_nullary",
        why: "**G-48**：两侧都是零元糖 ⇒ 严格档红；窄版靠「同形已解兄弟」，引擎靠 defaulting \
              ⇒ **两档都必须绿**（M1 的硬红线：引擎不许丢解）",
        body: "theorem n3 (β : Type) (b : β) : ¬ (∅ ≈ {b}) := by sorry",
        default_state: (0, "-"),
        sibling: (0, "-"),
        engine: (0, "-"),
        strict: (1, "elab-notation-argument-unsolved"),
    },
    Shape {
        name: "N4_two_singletons",
        why: "两个单元素集（元素类型不同）⇒ 两个前导参数各有来源 ⇒ 三档都绿",
        body: "theorem n4 (a : Nat) (b : Bool) : ¬ ({a} ≈ {b}) := by sorry",
        default_state: (0, "-"),
        sibling: (0, "-"),
        engine: (0, "-"),
        strict: (0, "-"),
    },
    Shape {
        name: "N5_subset_conflicting",
        why: "**约束冲突**（同一个 `α` 被 `{a} : Set Nat` 与 `{b} : Set Bool` 两头拉）⇒ 严格档**先成功**\
              （首个命中即返回、不查第二条）⇒ 拖到**内核**才拒 ⇒ 三档同判 `kernel-rejected`（引擎档也一样：\
              严格档先跑且成功 ⇒ 引擎根本没轮到 —— 冲突检出要等 M3 的报错契约收口）",
        body: "theorem n5 (a : Nat) (b : Bool) : ¬ ({a} ⊆ {b}) := by sorry",
        default_state: (1, "kernel-rejected"),
        sibling: (1, "kernel-rejected"),
        engine: (1, "kernel-rejected"),
        strict: (1, "kernel-rejected"),
    },
];

/// **M1 的判据**（一个 `#[test]`：5 形状 × 4 档 = 20 次真进程）。
#[test]
fn engine_reproduces_the_sibling_verdicts_and_never_loses_a_solution() {
    for shape in SHAPES {
        let file = temp_file(shape.name, &format!("{PRELUDE}{}\n", shape.body));
        let d = grade(&file, &format!("{}-d", shape.name), &[]);
        let s = grade(
            &file,
            &format!("{}-s", shape.name),
            &[("SOKO_METAVAR", "sibling")],
        );
        let e = grade(
            &file,
            &format!("{}-e", shape.name),
            &[("SOKO_METAVAR", "engine")],
        );
        let st = grade(
            &file,
            &format!("{}-0", shape.name),
            &[("SOKO_METAVAR", "0")],
        );
        let got: [(&str, (i32, String)); 4] = [
            ("default", d),
            ("sibling", s),
            ("engine", e),
            ("strict", st),
        ];
        let want: [(&str, (i32, &str)); 4] = [
            ("default", shape.default_state),
            ("sibling", shape.sibling),
            ("engine", shape.engine),
            ("strict", shape.strict),
        ];
        for (label, actual) in got.iter() {
            let expected = want.iter().find(|(l, _)| l == label).expect("label").1;
            assert_eq!(
                (actual.0, actual.1.as_str()),
                (expected.0, expected.1),
                "{}（{}）：**{label}** 档的（退出码, 诊断码）与钉住的值不符",
                shape.name,
                shape.why
            );
        }
        // 判据 1：默认档 ≡ sibling（开关的默认值不许漂）
        assert_eq!(
            (got[0].1).0,
            (got[1].1).0,
            "{}：默认档必须与 `sibling` 同判",
            shape.name
        );
        assert_eq!((got[0].1).1, (got[1].1).1, "{}：默认档的诊断码", shape.name);
        // 判据 3（硬红线）：sibling 绿 ⇒ engine 必须绿（**引擎不许丢解**）
        if (got[1].1).0 == 0 {
            assert_eq!(
                (got[2].1).0,
                0,
                "{}：`sibling` 判绿而 `engine` 判红 ⇒ **引擎丢解** ✗（M1 硬红线）",
                shape.name
            );
        }
    }
}

/// **开关语义**：旧逃生门与新开关等价（`0` 都是严格档）；`engine`/`unify` 同义。
#[test]
fn switch_states_are_equivalent_across_spellings() {
    let file = temp_file(
        "switch",
        &format!(
            "{PRELUDE}{}\n",
            "theorem s (β : Type) (b : β) : ¬ (∅ ≈ {b}) := by sorry"
        ),
    );
    let new_off = grade(&file, "sw-new-off", &[("SOKO_METAVAR", "0")]);
    let old_off = grade(&file, "sw-old-off", &[("SOKO_NOTATION_METAVAR", "0")]);
    assert_eq!(new_off, old_off, "`SOKO_METAVAR=0` 必须等于旧逃生门");
    assert_eq!(new_off.0, 1, "逃生门 = 严格档 ⇒ G-48 形状判红");

    let engine = grade(&file, "sw-engine", &[("SOKO_METAVAR", "engine")]);
    let unify = grade(&file, "sw-unify", &[("SOKO_METAVAR", "unify")]);
    assert_eq!(engine, unify, "`engine` 与 `unify` 是同一个档");
    assert_eq!(engine.0, 0, "引擎档要解得出来（与窄版同判）");
}

/// **M2** 的前奏（一般路径：应用 / 裸常量 / 路线③）。
const PRELUDE_APP: &str = "\
def Set (α : Type) : Type := α -> Prop
def Set.empty (α : Type) : Set α := fun (_ : α) => False
def Set.singleton (α : Type) (a : α) : Set α := fun (x : α) => x = a
def Set.Equiv {α β : Type} (A : Set α) (B : Set β) : Prop :=
  (forall (x : α), A x -> A x) ∧ (forall (y : β), B y -> B y)
def Two {α : Type} (A : Set α) (B : Set α) : Prop := True
def ignores {α : Type} (n : Nat) : Nat := n
notation \"∅\" => Set.empty
infix:50 \" ≈ \" => Set.Equiv
";

/// 一个**一般路径**形状（M2）：名字 / 为什么 / 四档实测。
struct AppShape {
    name: &'static str,
    why: &'static str,
    body: &'static str,
    default_state: (i32, &'static str),
    sibling: (i32, &'static str),
    engine: (i32, &'static str),
    strict: (i32, &'static str),
}

const APP_SHAPES: &[AppShape] = &[
    AppShape {
        name: "G1_equiv_empty_singleton",
        why: "应用路径的 G-48 同形（E19 刀2 的夹具）：严格档红 / 窄版与引擎都靠 defaulting 判绿",
        body: "theorem g1 (β : Type) (b : β) : ¬ (Set.Equiv ∅ {b}) := by sorry",
        default_state: (0, "-"),
        sibling: (0, "-"),
        engine: (0, "-"),
        strict: (1, "elab-implicit-argument-unsolved"),
    },
    AppShape {
        name: "G2_equiv_singleton_empty",
        why: "操作数顺序相反（已解的一侧在后）：defaulting 仍借得到 ⇒ 两档都绿",
        body: "theorem g2 (α : Type) (a : α) : ¬ (Set.Equiv {a} ∅) := by sorry",
        default_state: (0, "-"),
        sibling: (0, "-"),
        engine: (0, "-"),
        strict: (1, "elab-implicit-argument-unsolved"),
    },
    AppShape {
        name: "G3_both_hungry",
        why: "两侧都零元糖 ⇒ 一个**已解**兄弟都没有 ⇒ 四档全红（**不猜**）",
        body: "theorem g3 (α β : Type) : ¬ (Set.Equiv ∅ ∅) := by sorry",
        default_state: (1, "elab-implicit-argument-unsolved"),
        sibling: (1, "elab-implicit-argument-unsolved"),
        engine: (1, "elab-implicit-argument-unsolved"),
        strict: (1, "elab-implicit-argument-unsolved"),
    },
    AppShape {
        name: "G4_param_never_mentioned",
        why: "`α` 在任何实参类型与期望类型里都不出现 ⇒ 四档全红（既有判据的同一夹具）",
        body: "def g4 : Nat := ignores 3",
        default_state: (1, "elab-implicit-argument-unsolved"),
        sibling: (1, "elab-implicit-argument-unsolved"),
        engine: (1, "elab-implicit-argument-unsolved"),
        strict: (1, "elab-implicit-argument-unsolved"),
    },
    AppShape {
        name: "G5_conflicting_constraints",
        why: "同一个 `α` 被 `Set Nat` 与 `Set Bool` 两头拉：**严格档先成功**（首个命中即返回）⇒ 引擎              没轮到 ⇒ 四档同判内核拒绝（冲突检出要等 M3 的报错契约收口）",
        body: "def g5 (A : Set Nat) (B : Set Bool) : Prop := Two A B",
        default_state: (1, "kernel-rejected"),
        sibling: (1, "kernel-rejected"),
        engine: (1, "kernel-rejected"),
        strict: (1, "kernel-rejected"),
    },
];

/// **M2 的判据**（5 个一般路径形状 × 4 档 = 20 次真进程）：引擎接进 `implicit::solve_prefix`
/// 之后，`sibling` 档仍是今天，`engine` 档**不许丢解**且逐条与窄版同判。
#[test]
fn general_path_engine_matches_sibling_and_never_loses() {
    for shape in APP_SHAPES {
        let file = temp_file(shape.name, &format!("{PRELUDE_APP}{}\n", shape.body));
        let d = grade(&file, &format!("{}-d", shape.name), &[]);
        let s = grade(
            &file,
            &format!("{}-s", shape.name),
            &[("SOKO_METAVAR", "sibling")],
        );
        let e = grade(
            &file,
            &format!("{}-e", shape.name),
            &[("SOKO_METAVAR", "engine")],
        );
        let st = grade(
            &file,
            &format!("{}-0", shape.name),
            &[("SOKO_METAVAR", "0")],
        );
        let got: [(&str, (i32, String)); 4] = [
            ("default", d),
            ("sibling", s),
            ("engine", e),
            ("strict", st),
        ];
        let want: [(&str, (i32, &str)); 4] = [
            ("default", shape.default_state),
            ("sibling", shape.sibling),
            ("engine", shape.engine),
            ("strict", shape.strict),
        ];
        for (label, actual) in got.iter() {
            let expected = want.iter().find(|(l, _)| l == label).expect("label").1;
            assert_eq!(
                (actual.0, actual.1.as_str()),
                (expected.0, expected.1),
                "{}（{}）：**{label}** 档的（退出码, 诊断码）与钉住的值不符",
                shape.name,
                shape.why
            );
        }
        assert_eq!(
            ((got[0].1).0, (got[0].1).1.clone()),
            ((got[1].1).0, (got[1].1).1.clone()),
            "{}：默认档必须与 `sibling` 同判",
            shape.name
        );
        if (got[1].1).0 == 0 {
            assert_eq!(
                (got[2].1).0,
                0,
                "{}：`sibling` 判绿而 `engine` 判红 ⇒ **引擎丢解** ✗（M2 硬红线）",
                shape.name
            );
        }
    }
}

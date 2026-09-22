//! 可信内置声明（prelude）的安装：Nat 骨架、Eq 三件套与可选性配置。
//!
//! prelude 是「受信任的预置」：安装后不再被内核重查（不进 PendingOp）。
//! 教学文件可以在两种模式下编译（用户要求）：
//! * `PreludeMode::Full` —— 安装全部内置基元（Nat/Eq，若未被文件自带声明占用）；
//! * `PreludeMode::Bare` —— 完全不安装任何东西，课程从零构造一切
//!   （例如自带 `inductive Nat` 块或纯逻辑公理文件）。

use super::elab::{
    build_axiom, build_def, install_inductive_block, params_of_ty, strip_lambdas_n, DefInfo,
    DefTable, ElabCtx, InductiveTable, KnownName, KnownTable,
};
use super::scope::NamespaceScope;
use crate::{Binder, BinderKind, Command, CtorDecl, Expr, SortKind, Span};
use sokonanoda::builder::EnvBuilder;
use sokonanoda::env::{Declar, DeclarInfo, ReducibilityHint};
use sokonanoda::expr::BinderStyle;
use sokonanoda::util::ExprPtr;
use std::collections::HashSet;

/// Which trusted base declarations a compilation installs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PreludeMode {
    /// Install the built-in teaching prelude (Nat + Eq, unless the file
    /// declares its own versions).
    #[default]
    Full,
    /// Install nothing: the file must be self-contained. `1 + 1` and `Nat`
    /// only work if the file provides them.
    Bare,
}

/// Options for one compilation session.
#[derive(Debug, Clone, Copy, Default)]
pub struct CompileOptions {
    pub prelude: PreludeMode,
}

/// Read the file-level prelude directive from `--` comment lines:
/// `-- sokonanoda:prelude none` (or `bare`) selects `PreludeMode::Bare`,
/// `-- sokonanoda:prelude full` selects `Full`. The flag stays declarative:
/// it is a comment, so the file remains a plain text canvas.
pub fn prelude_mode_from_source(src: &str) -> PreludeMode {
    explicit_prelude_mode(src).unwrap_or(PreludeMode::Full)
}

/// 文件**显式**写了 `-- sokonanoda:prelude …` 指令时返回它，否则 `None`。
/// 项目闭包里"没写指令"= 继承入口的模式（设计 §4.6），只有**显式冲突**
/// 才是 `import-prelude-conflict`。
pub fn explicit_prelude_mode(src: &str) -> Option<PreludeMode> {
    for line in src.lines() {
        let trimmed = line.trim_start();
        let Some(comment) = trimmed.strip_prefix("--") else {
            continue;
        };
        let comment = comment.trim();
        let Some(rest) = comment.strip_prefix("sokonanoda:prelude") else {
            continue;
        };
        let value = rest.trim();
        return Some(match value {
            "none" | "bare" => PreludeMode::Bare,
            _ => PreludeMode::Full,
        });
    }
    None
}

/// Trusted equality primitives, written in the teaching syntax itself and
/// installed without re-checking (like the Nat prelude). The signatures match
/// official Lean's `Eq`/`Eq.refl`/`Eq.subst`, so a filled exercise file that
/// uses them still checks in real Lean.
/// The trusted prelude's top-level names (Full mode). Completions material:
/// prelude declarations are trusted installs without `DeclState`s, so the
/// goal view / completion layer needs this list to offer them.
///
/// L1 (`docs/design/prelude-l1-proposal.md` §3.3) added 30 names: the 28
/// declarations of [`PRELUDE_L1_SRC`] plus the two derived recursors
/// (`And.rec`/`Or.rec`, which `install_inductive_block` generates).
///
/// B8（L-03，2026-09-19；0.61.0 扩族）再加 5 条：`Eq.rec`、由它定义的
/// `Eq.ndrec`/`Eq.mp`/`Eq.mpr`/`cast`（Type 层重写；设计
/// `docs/design/eq-type-level-rewriting.md`）。`Eq.mp`/`Eq.mpr`/`cast` 是
/// **宇宙多态**的（层级算术 `u+1` 落地后，签名与 Lean core 逐字对齐）。
pub const PRELUDE_NAMES: &[&str] = &[
    "Nat",
    "Nat.zero",
    "Nat.succ",
    "Nat.rec",
    "Nat.add",
    "Bool",
    "Bool.true",
    "Bool.false",
    "Bool.rec",
    "Eq",
    "Eq.refl",
    "Eq.subst",
    // ---- L1: 真伪 (B1/B2) ----
    "True",
    "True.intro",
    "False",
    "False.rec",
    "False.elim",
    // ---- L1: 且 (B3) ----
    "And",
    "And.intro",
    "And.left",
    "And.right",
    "And.elim",
    "And.rec",
    // ---- L1: 或 (B4) ----
    "Or",
    "Or.inl",
    "Or.inr",
    "Or.elim",
    "Or.rec",
    // ---- L1: 非 (B5) ----
    "Not",
    "Not.intro",
    "Not.elim",
    "absurd",
    // ---- L1: 不相等 (L2.3，`≠` 的目标) ----
    "Ne",
    "Ne.intro",
    // ---- L1: 当且仅当 (B6) ----
    "Iff",
    "Iff.intro",
    "Iff.mp",
    "Iff.mpr",
    "Iff.refl",
    "Iff.symm",
    "Iff.trans",
    // ---- L1: Eq 引理 (B7) ----
    "Eq.symm",
    "Eq.trans",
    "congrArg",
    // ---- L1: Eq 大消去 / Type 层重写 (B8) ----
    "Eq.rec",
    "Eq.ndrec",
    "Eq.mp",
    "Eq.mpr",
    "cast",
];

/// Full 模式下**永不**让位的 prelude 名字（`Nat`/`Bool` 家族）。
///
/// 与 [`PRELUDE_NAMES`] 的分工（设计 §2.3-2）：`PRELUDE_NAMES` 是补全/材料
/// 列表（含 L1），`PRELUDE_NEVER_YIELDS` 只用于 `check_name_collisions` 的
/// 豁免面。L1 名字按**族**合法让位（文件自己声明 ⇒ 整族来自文件），所以它们
/// **不能**进这个列表——否则两个模块各自声明 `True` 就不再报友好的
/// `import-name-collision`，退化成内核裸错。
pub const PRELUDE_NEVER_YIELDS: &[&str] = &[
    "Nat",
    "Nat.zero",
    "Nat.succ",
    "Nat.rec",
    "Nat.add",
    "Bool",
    "Bool.true",
    "Bool.false",
    "Bool.rec",
];

pub const PRELUDE_EQ_SRC: &str = "\
axiom Eq {u} : {α : Sort u} -> α -> α -> Prop
axiom Eq.refl {u} : {α : Sort u} -> (a : α) -> Eq.{u} α a a
axiom Eq.subst {u} : {α : Sort u} -> {p : α -> Prop} -> {a : α} -> {b : α} -> Eq.{u} α a b -> p a -> p b
";

/// L1 的规范源文本（设计 `docs/design/prelude-l1-proposal.md` 附录 A）：
/// Lean core 的逻辑与等式骨架，用教学语法逐字写出来，作为**受信任安装**
/// （与 `Nat`/`Bool`/`Eq` 同一条路径：走 `build_def`/`install_inductive_block`
/// 与用户声明同一个 elaborator，但不再被内核重查）。
///
/// 与课程库 `courses/set-theory/lib/Logic.sokonanoda` 的差异只有两处（附录 A）：
/// **`And` 由 axiom 族改成真归纳块**、**`Or` 的构造子由裸名 `inl`/`inr`
/// 改成 `Or.inl`/`Or.inr`**。文件顺序必须满足依赖：
/// B1/B2 → B3 → B4 → B5 → B6 → B7 → B8（`And.elim` 在 `And.left/right` 之后、
/// `Iff` 在 `And` 之后、`Eq.mp`/`Eq.mpr` 在 `Eq.rec` 之后）。
///
/// 三条硬约束下的定形（设计 §1.2）：`{u}` 是唯一宇宙 binder（G-14）；
/// `axiom` 一律柯里化（G-13）；构造子写点号名（G-02 的可行解）；
/// 隐式实参不自动插入，所以签名显式给全参数。
///
/// **B8（`Eq.rec`/`Eq.ndrec`/`Eq.mp`/`Eq.mpr`/`cast`，L-03，2026-09-19）**：
/// Eq prelude 的 `Eq` 是**公理**（不是归纳块），所以内核不会为它派生消去子
/// （`Eq.rec` 实测 `elab-unknown-constant`）；而本语言的 `inductive` 头部今天
/// 不吃宇宙 binder，没法把 `Eq` 立成宇宙多态的归纳块。因此 B8 走**公理**：
/// `axiom Eq.rec {u, v}` 的签名与 Lean core 的 `Eq.rec.{u, v}` 逐字同形
/// （`docs/design/eq-type-level-rewriting.md` §3 有实测与取舍）。
///
/// 其余四条都是 `def`（不新增信任面）：`Eq.ndrec` 是 Lean core 的
/// **非依赖消去子**（motive 不吃证明，Lean 里它是 `abbrev`），
/// `Eq.mp`/`Eq.mpr`/`cast` 是 `Eq.rec` 在 `Sort u` 上的实例——Lean core 里
/// `cast h a` 就是 `Eq.mp h a`（`h.rec a`），三条签名逐字对齐：
/// `{α β : Sort u} (h : @Eq.{u+1} (Sort u) α β)`。**层级算术 `u+1` 落地后**
/// （`docs/design/type-level-syntax.md` §5）它们才是宇宙多态的；此前是
/// Type 0 实例（0.60.0 的残留边界，设计 §4-1 已销账）。
pub const PRELUDE_L1_SRC: &str = "\
axiom True : Prop
axiom True.intro : True
axiom False : Prop
axiom False.rec {C : Prop} : False -> C
def False.elim {C : Prop} (h : False) : C := False.rec C h
inductive And (a b : Prop) : Prop
ctor And.intro (ha : a) (hb : b) : And a b
end
def And.left {a b : Prop} (h : And a b) : a := And.rec a b (fun (_ : And a b) => a) (fun (ha : a) (hb : b) => ha) h
def And.right {a b : Prop} (h : And a b) : b := And.rec a b (fun (_ : And a b) => b) (fun (ha : a) (hb : b) => hb) h
def And.elim {a b c : Prop} (f : a -> b -> c) (h : And a b) : c := f (And.left a b h) (And.right a b h)
inductive Or (A B : Prop) : Prop
ctor Or.inl (a : A) : Or A B
ctor Or.inr (b : B) : Or A B
end
def Or.elim {a b c : Prop} (f : a -> c) (g : b -> c) (h : Or a b) : c := Or.rec a b (fun (_ : Or a b) => c) f g h
def Not (A : Prop) : Prop := A -> False
def Not.intro {A : Prop} (f : A -> False) : Not A := f
def Not.elim {A C : Prop} (h : Not A) (a : A) : C := False.elim C (h a)
def absurd {a b : Prop} (ha : a) (hna : Not a) : b := False.elim b (hna ha)
-- `Ne`（L2.3）：`≠` 的**目标常量**，与 Lean core 的 `Ne` 同形（`a ≠ b` 就是
-- `a = b -> False`）。带**一个宇宙参数** `u`（`α : Sort u`）——所以 `≠` 的记法
-- 路径要解层级，与 `=` 同一份机械（`elab.rs` 的 `level_text_of_sort`）。
def Ne {u} (α : Sort u) (a b : α) : Prop := Eq.{u} α a b -> False
def Ne.intro {u} {α : Sort u} {a b : α} (h : Eq.{u} α a b -> False) : Ne.{u} α a b := h
def Iff (A B : Prop) : Prop := And (A -> B) (B -> A)
def Iff.intro {A B : Prop} (mp : A -> B) (mpr : B -> A) : Iff A B := And.intro (A -> B) (B -> A) mp mpr
def Iff.mp {A B : Prop} (h : Iff A B) : A -> B := And.left (A -> B) (B -> A) h
def Iff.mpr {A B : Prop} (h : Iff A B) : B -> A := And.right (A -> B) (B -> A) h
def Iff.refl {A : Prop} : Iff A A := Iff.intro A A (fun (h : A) => h) (fun (h : A) => h)
def Iff.symm {A B : Prop} (h : Iff A B) : Iff B A := Iff.intro B A (Iff.mpr A B h) (Iff.mp A B h)
def Iff.trans {A B C : Prop} (h1 : Iff A B) (h2 : Iff B C) : Iff A C := Iff.intro A C (fun (a : A) => Iff.mp B C h2 (Iff.mp A B h1 a)) (fun (c : C) => Iff.mpr A B h1 (Iff.mpr B C h2 c))
def Eq.symm {u} {α : Sort u} {a b : α} (h : Eq.{u} α a b) : Eq.{u} α b a := Eq.subst.{u} α (fun (x : α) => Eq.{u} α x a) a b h (Eq.refl.{u} α a)
def Eq.trans {u} {α : Sort u} {a b c : α} (h1 : Eq.{u} α a b) (h2 : Eq.{u} α b c) : Eq.{u} α a c := Eq.subst.{u} α (fun (x : α) => Eq.{u} α a x) b c h2 h1
def congrArg {u} {α : Sort u} {β : Sort u} {a b : α} (f : α -> β) (h : Eq.{u} α a b) : Eq.{u} β (f a) (f b) := Eq.subst.{u} α (fun (x : α) => Eq.{u} β (f a) (f x)) a b h (Eq.refl.{u} β (f a))
axiom Eq.rec {u, v} : {α : Sort u} -> (a : α) -> (motive : (anon : α) -> Sort v) -> (ha : motive a) -> (b : α) -> (h : @Eq.{u} α a b) -> motive b
def Eq.ndrec {u, v} (α : Sort u) (a : α) (motive : α -> Sort v) (m : motive a) (b : α) (h : @Eq.{u} α a b) : motive b := @Eq.rec.{u, v} α a motive m b h
def Eq.mp {u} {α β : Sort u} (h : @Eq.{u+1} (Sort u) α β) : α -> β := @Eq.rec.{u+1, u} (Sort u) α (fun (x : Sort u) => α -> x) (fun (a : α) => a) β h
def Eq.mpr {u} {α β : Sort u} (h : @Eq.{u+1} (Sort u) α β) : β -> α := @Eq.rec.{u+1, u} (Sort u) α (fun (x : Sort u) => x -> α) (fun (a : α) => a) β h
def cast {u} {α β : Sort u} (h : @Eq.{u+1} (Sort u) α β) (a : α) : β := Eq.mp.{u} α β h a
";

/// 让位的粒度 = **族**，族之间按依赖做闭包让位（设计 §2.2）。
///
/// 一个族要么整族来自 prelude，要么整族来自文件——避免"prelude 的 `And`
/// + 文件的 `And.left`"这种静默不一致。
///
/// `deps` 里的族一旦被文件占用，本族也必须让位：它们的定义体直接引用被依赖
/// 的名字（`Not.elim` 用 `False.elim`、`Iff.mp` 用 `And.left`、`Eq.symm`
/// 用 `Eq.subst`），否则 prelude 源文本自己就 elaborate 不过。
pub(crate) struct PreludeFamily {
    /// 族名（诊断/测试用；`B7` 额外依赖 Eq prelude，见 [`L1_FAMILIES`]）。
    pub name: &'static str,
    /// 本族的全部顶层名字（含派生递归子，用于触发判定）。
    pub names: &'static [&'static str],
    /// 被让位时本族也必须让位的族名。
    pub deps: &'static [&'static str],
}

/// L1 的族表（设计 §2.2 的 B1–B7 + L-03 的 B8 + L2.3 的 B9）。`B7`/`B8`/`B9` 依赖 **Eq prelude**：
/// `Eq` 被占用时 `install_eq_prelude` 整体不装，`Eq.symm`/`Eq.trans`/`congrArg`
/// 与 `Eq.rec`/`Eq.mp`/`Eq.mpr` 的定义体引用的 `Eq.subst`/`Eq` 就不存在，
/// 所以它们必须一起让位。
pub(crate) const L1_FAMILIES: &[PreludeFamily] = &[
    PreludeFamily {
        name: "B1",
        names: &["True", "True.intro"],
        deps: &[],
    },
    PreludeFamily {
        name: "B2",
        names: &["False", "False.rec", "False.elim"],
        deps: &[],
    },
    PreludeFamily {
        name: "B3",
        names: &[
            "And",
            "And.intro",
            "And.left",
            "And.right",
            "And.elim",
            "And.rec",
        ],
        deps: &[],
    },
    PreludeFamily {
        name: "B4",
        names: &["Or", "Or.inl", "Or.inr", "Or.elim", "Or.rec"],
        deps: &[],
    },
    PreludeFamily {
        name: "B5",
        names: &["Not", "Not.intro", "Not.elim", "absurd"],
        deps: &["B2"],
    },
    PreludeFamily {
        name: "B6",
        names: &[
            "Iff",
            "Iff.intro",
            "Iff.mp",
            "Iff.mpr",
            "Iff.refl",
            "Iff.symm",
            "Iff.trans",
        ],
        deps: &["B3"],
    },
    PreludeFamily {
        name: "B7",
        names: &["Eq.symm", "Eq.trans", "congrArg"],
        deps: &["EQ"],
    },
    PreludeFamily {
        name: "B8",
        names: &["Eq.rec", "Eq.ndrec", "Eq.mp", "Eq.mpr", "cast"],
        deps: &["EQ"],
    },
    // B9（L2.3，2026-09-19）：`≠` 的目标常量。定义体同时用 `Eq.{u}` 与 `False`
    // ⇒ 依赖 **EQ 与 B2**（与 B5 的 `Not` 依赖 B2 同一个理由：定义体引用的族
    // 一旦让位，本族也必须让位，否则报「unknown identifier `False`」）。
    PreludeFamily {
        name: "B9",
        names: &["Ne", "Ne.intro"],
        deps: &["B2", "EQ"],
    },
];

/// 一个族名（`B1`…`B9`）是否必须让位：它自己或它的依赖被 `taken` 命中。
fn family_yields(family: &PreludeFamily, taken: &HashSet<String>) -> bool {
    family.names.iter().any(|name| taken.contains(*name))
        || family.deps.iter().any(|dep| {
            dep_is_taken(dep, taken)
                || L1_FAMILIES
                    .iter()
                    .find(|f| f.name == *dep)
                    .is_some_and(|f| family_yields(f, taken))
        })
}

/// `EQ` 不是 L1 族，它是 [`PRELUDE_EQ_SRC`] 的族名（`install_eq_prelude`
/// 的 all-or-nothing 口径）。`taken` 命中三名之一 ⇒ B7 一起让位。
fn dep_is_taken(dep: &str, taken: &HashSet<String>) -> bool {
    if dep == "EQ" {
        return ["Eq", "Eq.refl", "Eq.subst"]
            .iter()
            .any(|name| taken.contains(*name));
    }
    false
}

thread_local! {
    /// 重入闸（as-built，设计 §1.2 之外的**新发现**）：装 L1 的 `And` 归纳块时，
    /// `install_inductive_block` 要用 `large_elim_test_mirror` 问内核「字段类型
    /// 是不是 Prop」（`field_sort_via_kernel` → `judge_infer` → **内层
    /// `compile_fol_with`**）。内层编译又会装一遍 L1 的 `And` ⇒ 无限递归
    /// （实测：`stack overflow, SIGABRT`）。
    ///
    /// 计数 > 0 时 `install_l1_prelude` 直接返回：内层编译的环境里没有 L1。
    /// 这是安全的，因为 L1 安装期间的 kernel 探针只问「`Prop` 参数是不是
    /// `Prop`」（`field_sort_via_kernel` 的 binder 全部来自归纳块自己的参数，
    /// 类型是内建的 `Prop`），不需要任何 L1 名字。用户代码触发的内层编译
    /// （depth 0）照常拿完整 L1。
    static L1_INSTALL_DEPTH: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// 一个名字属于哪个 L1 族（`None` = 不是 L1 名字）。建议材料层用它与
/// [`family_yields_by_name`] 复现同一条让位规则（`goals.rs`）。
pub(crate) fn l1_family_of(name: &str) -> Option<&'static PreludeFamily> {
    L1_FAMILIES
        .iter()
        .find(|family| family.names.contains(&name))
}

/// 名字 `name` 所属的族是否因 `taken` 而让位。`name` 不是 L1 名字时返回
/// `false`（调用方自行判定归属）。
pub(crate) fn family_yields_by_name(name: &str, taken: &HashSet<String>) -> bool {
    l1_family_of(name).is_some_and(|family| family_yields(family, taken))
}

/// 重入闸：见 [`L1_INSTALL_DEPTH`]。
///
/// 装 L1：按族顺序逐条 install，命中让位的族整体跳过。
///
/// 安装走**与用户声明同一条 elaborator**（`build_axiom`/`build_def`/
/// `install_inductive_block`），不是只走 `build_axiom`——设计 §4.1 的
/// 主要风险点。
pub(crate) fn install_l1_prelude<'a>(
    builder: &mut EnvBuilder<'a>,
    known: &mut KnownTable,
    inductives: &mut InductiveTable<'a>,
    defs: &mut DefTable,
    taken: &HashSet<String>,
) {
    // prelude 安装期间关闭隐式实参插入（见 `elab::PreludeInstallGuard` 的注释）。
    let _implicit_guard = crate::compile::elab::PreludeInstallGuard::enter();
    if L1_INSTALL_DEPTH.with(|d| d.get()) > 0 {
        return; // 见 `L1_INSTALL_DEPTH`：内层编译不再装 L1
    }
    let Ok(file) = crate::parse(PRELUDE_L1_SRC) else {
        panic!("L1 prelude source parses");
    };
    let options = CompileOptions::default();
    L1_INSTALL_DEPTH.with(|d| d.set(d.get() + 1));
    for family in L1_FAMILIES {
        if family_yields(family, taken) {
            continue;
        }
        for command in &file.commands {
            if !command_belongs_to(command, family) {
                continue;
            }
            install_l1_command(builder, known, inductives, defs, &options, command);
        }
    }
    L1_INSTALL_DEPTH.with(|d| d.set(d.get() - 1));
}

/// 这条命令是不是本族的（按顶层名字判定；`ctor`/`rec` 归它们的归纳块）。
fn command_belongs_to(command: &Command, family: &PreludeFamily) -> bool {
    match command {
        Command::Axiom { name, .. } | Command::Def { name, .. } => {
            family.names.contains(&name.as_str())
        }
        Command::InductiveBlock { name, .. } => family.names.contains(&name.as_str()),
        _ => false,
    }
}

/// 装一条 L1 命令。axiom 走 `build_axiom`，def 走 `build_def`，归纳块走
/// `install_inductive_block`（构造子与派生递归子由它一起登记）。
fn install_l1_command<'a>(
    builder: &mut EnvBuilder<'a>,
    known: &mut KnownTable,
    inductives: &mut InductiveTable<'a>,
    defs: &mut DefTable,
    options: &CompileOptions,
    command: &Command,
) {
    // `prefix_src` 为空：L1 的签名不依赖文件前缀（它们是闭包无关的骨架），
    // 且安装期间的 kernel 探针只需内建的 `Prop`（`L1_INSTALL_DEPTH` 挡掉
    // 内层重入，所以内层环境里没有 L1 名字也不影响）。G-05：prelude 永远在
    // 根命名空间、没有 `open`，作用域是空的那一份。
    let ns = NamespaceScope::new();
    let empty_defs: DefTable = DefTable::new();
    let ctx = ElabCtx {
        prefix_src: "",
        options,
        inductives,
        ns: &ns,
        defs: &empty_defs,
    };
    let mut hovers = Vec::new();
    match command {
        Command::Axiom {
            name, universe, ty, ..
        } => {
            let decl = build_axiom(builder, name, universe, ty, known, &mut hovers, &ctx)
                .expect("L1 prelude axiom elaborates");
            builder
                .add_declar(decl)
                .expect("duplicate L1 prelude axiom");
            known.insert(
                name.clone(),
                KnownName::Decl {
                    universes: universe.clone(),
                    implicit_prefix: crate::compile::elab::leading_implicit_prefix(ty),
                    explicit_arity: crate::compile::elab::explicit_arity(ty),
                    signature: Some(crate::proof::render_expr(ty)),
                },
            );
        }
        Command::Def {
            name,
            universe,
            ty,
            val,
            ..
        } => {
            let decl = build_def(builder, name, universe, ty, val, known, &mut hovers, &ctx)
                .expect("L1 prelude definition elaborates");
            builder
                .add_declar(decl)
                .expect("duplicate L1 prelude definition");
            known.insert(
                name.clone(),
                KnownName::Decl {
                    universes: universe.clone(),
                    implicit_prefix: crate::compile::elab::leading_implicit_prefix(ty),
                    explicit_arity: crate::compile::elab::explicit_arity(ty),
                    signature: Some(crate::proof::render_expr(ty)),
                },
            );
            // 源级 delta 表：`by` 引擎靠它看穿 `Not`/`Iff` 这类 **def** 头
            // （`intro x` 在 `¬ A` 目标上、`apply h` 在 `h : A ⊆ B` 上都要它）。
            defs.insert(
                name.clone(),
                DefInfo {
                    params: params_of_ty(ty),
                    universes: universe.clone(),
                    body: strip_lambdas_n(val, params_of_ty(ty).len()),
                },
            );
        }
        Command::InductiveBlock {
            name,
            params,
            ty,
            constructors,
            recursor,
            iota_rules,
            ..
        } => {
            let mut built = Vec::new();
            install_inductive_block(
                builder,
                known,
                inductives,
                "",
                options,
                &ns,
                name,
                params,
                ty,
                constructors,
                recursor.as_ref(),
                iota_rules,
                &mut hovers,
                &mut built,
            )
            .expect("L1 prelude inductive block installs");
        }
        _ => panic!("L1 prelude must only contain axiom/def/inductive commands"),
    }
}

/// Install the Eq prelude, skipping the whole block when the file declares
/// any of the names itself (all-or-nothing, mirroring the explicit `Nat`
/// block behavior: the file then owns equality entirely).
/// `known` gains the universe-parameter arity of each installed axiom.
pub(crate) fn install_eq_prelude(
    builder: &mut EnvBuilder<'_>,
    known: &mut KnownTable,
    taken: &std::collections::HashSet<String>,
) {
    // prelude 安装期间关闭隐式实参插入（见 `elab::PreludeInstallGuard` 的注释）。
    let _implicit_guard = crate::compile::elab::PreludeInstallGuard::enter();
    const EQ_NAMES: [&str; 3] = ["Eq", "Eq.refl", "Eq.subst"];
    if EQ_NAMES.iter().any(|name| taken.contains(*name)) {
        return;
    }
    let file = crate::parse(PRELUDE_EQ_SRC).expect("Eq prelude source parses");
    let empty: InductiveTable<'_> = InductiveTable::new();
    let options = CompileOptions::default();
    let ns = NamespaceScope::new();
    let empty_defs: DefTable = DefTable::new();
    let ctx = ElabCtx {
        prefix_src: "",
        options: &options,
        inductives: &empty,
        ns: &ns,
        defs: &empty_defs,
    };
    for command in &file.commands {
        let Command::Axiom {
            name, universe, ty, ..
        } = command
        else {
            panic!("Eq prelude must only contain axioms");
        };
        if taken.contains(name) {
            continue;
        }
        let mut hovers = Vec::new();
        let decl = build_axiom(builder, name, universe, ty, known, &mut hovers, &ctx)
            .expect("Eq prelude axiom elaborates");
        builder.add_declar(decl).expect("duplicate prelude axiom");
        // **Eq 族保持 `implicit_prefix: 0`**：`Eq.{u} α a b` 是用户写全的旧式
        // 调用，而它的**部分应用**（`Eq.{1} Nat 2`）与"隐式调用只给显式实参"
        // 在形状上无法区分——登记成隐式会把 `Nat` 当成 `α` 的值（实测
        // `Eq.{1} Nat 2 2` 报 `Sort(1) vs Sort(2)`）。`=`/`≠` 的记法路径自己补
        // 前导参数，不需要应用路径插手。短写法的宇宙层级推断是独立的一刀。
        known.insert(
            name.clone(),
            KnownName::Decl {
                universes: universe.clone(),
                implicit_prefix: 0,
                explicit_arity: crate::compile::elab::explicit_arity(ty),
                signature: Some(crate::proof::render_expr(ty)),
            },
        );
    }
}

/// Trusted built-in base declarations. These are never re-checked by the
/// kernel. `Nat` is installed as a trusted source-style inductive block
/// (`Nat.zero`/`Nat.succ` constructors + a derived `Nat.rec`) so that `match`
/// on the prelude `Nat` lowers to a real `Nat.rec` and the recursor has a
/// proper iota rule set. `Nat.add` stays the native self-referential
/// definition: the kernel's native Nat reduction is enabled by the matching
/// declaration names.
pub(crate) fn install_prelude<'a>(
    builder: &mut EnvBuilder<'a>,
    known: &mut KnownTable,
    inductives: &mut InductiveTable<'a>,
) {
    // prelude 安装期间关闭隐式实参插入（见 `elab::PreludeInstallGuard` 的注释）。
    let _implicit_guard = crate::compile::elab::PreludeInstallGuard::enter();
    let span = Span::default();
    let nat_sort = Expr::Sort {
        sort: SortKind::Type,
        span,
    };
    let nat_ident = Expr::Ident {
        name: "Nat".to_string(),
        span,
    };
    let constructors = vec![
        CtorDecl {
            name: "Nat.zero".to_string(),
            binders: Vec::new(),
            result: nat_ident.clone(),
            span,
        },
        CtorDecl {
            name: "Nat.succ".to_string(),
            binders: vec![Binder {
                name: "n".to_string(),
                ty: Some(Box::new(nat_ident.clone())),
                style: BinderKind::Explicit,
                span,
            }],
            result: nat_ident,
            span,
        },
    ];
    let mut hovers = Vec::new();
    let mut built = Vec::new();
    // G-05：prelude 永远在根命名空间、没有 `open`。
    let ns = NamespaceScope::new();
    install_inductive_block(
        builder,
        known,
        inductives,
        "",
        &CompileOptions::default(),
        &ns,
        "Nat",
        &[],
        &nat_sort,
        &constructors,
        None,
        &[],
        &mut hovers,
        &mut built,
    )
    .expect("built-in Nat block installs");

    let anon = builder.anonymous();
    let empty = builder.alloc_levels_slice(&[]);
    let nat = builder.name_from_str("Nat");
    let nat_type = builder.mk_const(nat, empty);

    let inner_arrow = builder.mk_pi(anon, BinderStyle::Default, nat_type, nat_type);
    let add_arrow = builder.mk_pi(anon, BinderStyle::Default, nat_type, inner_arrow);
    let add_name = builder.name_from_str("Nat.add");
    let add_levels = builder.alloc_levels_slice(&[]);
    let add_self = builder.mk_const(add_name, add_levels);
    add_definition(builder, "Nat.add", add_arrow, add_self);
    known.insert(
        "Nat.add".to_string(),
        KnownName::Decl {
            universes: Vec::new(),
            implicit_prefix: 0,
            explicit_arity: 2,
            signature: None,
        },
    );
}

/// Trusted built-in `Bool`, installed exactly like the `Nat` block: a
/// source-style inductive with constructors `Bool.true`/`Bool.false` and a
/// derived `Bool.rec`, registered in `known` and the `match` `InductiveTable`.
/// Non-recursive, so the recursor is the plain two-branch eliminator and the
/// kernel needs no change. The names must stay `Bool`/`Bool.true`/`Bool.false`
/// to match the kernel's name cache (frozen; `docs/architecture.md` §5.4).
pub(crate) fn install_bool_prelude<'a>(
    builder: &mut EnvBuilder<'a>,
    known: &mut KnownTable,
    inductives: &mut InductiveTable<'a>,
) {
    // prelude 安装期间关闭隐式实参插入（见 `elab::PreludeInstallGuard` 的注释）。
    let _implicit_guard = crate::compile::elab::PreludeInstallGuard::enter();
    let span = Span::default();
    let bool_sort = Expr::Sort {
        sort: SortKind::Type,
        span,
    };
    let bool_ident = Expr::Ident {
        name: "Bool".to_string(),
        span,
    };
    let constructors = vec![
        CtorDecl {
            name: "Bool.true".to_string(),
            binders: Vec::new(),
            result: bool_ident.clone(),
            span,
        },
        CtorDecl {
            name: "Bool.false".to_string(),
            binders: Vec::new(),
            result: bool_ident,
            span,
        },
    ];
    let mut hovers = Vec::new();
    let mut built = Vec::new();
    // G-05：prelude 永远在根命名空间、没有 `open`。
    let ns = NamespaceScope::new();
    install_inductive_block(
        builder,
        known,
        inductives,
        "",
        &CompileOptions::default(),
        &ns,
        "Bool",
        &[],
        &bool_sort,
        &constructors,
        None,
        &[],
        &mut hovers,
        &mut built,
    )
    .expect("built-in Bool block installs");
}

fn add_definition<'a>(builder: &mut EnvBuilder<'a>, name: &str, ty: ExprPtr<'a>, val: ExprPtr<'a>) {
    let name = builder.name_from_str(name);
    let info = DeclarInfo {
        name,
        uparams: builder.alloc_levels_slice(&[]),
        ty,
    };
    builder
        .add_declar(Declar::Definition {
            info,
            val,
            hint: ReducibilityHint::Regular(0),
        })
        .expect("duplicate builtin definition");
}

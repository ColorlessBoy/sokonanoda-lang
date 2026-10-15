//! **`Eq.subst` 的谓词位**（用户 2026-10-15 亲报：`SUB-eqsubst-clue-order-clash`）。
//!
//! 用户原案（课程 `courses/set-theory/units/I.1` 练习 4）：
//!
//! ```text
//! theorem subset_of_mem_singleton (α : Type) (A : Set α) (a : α) (h : a ∈ A) :
//!     {a} ⊆ A := by
//!   intro x hx
//!   unfold Set.singleton Set.mem at hx
//!   apply Eq.subst (Eq.symm hx) h
//! ```
//!
//! 报 `` `Eq.subst` 的签名 … 里有 4 个**隐式**参数，但补不出来 **两条线索互相矛盾** ``。
//!
//! **根因**（两处，缺一不可；`crates/front/src/compile/meta.rs`）：
//!
//! 1. `unify_body` 的结构分解（App/App）判否时**提前收摊** ⇒ 下面那条
//!    **delta 展开兜底**（S13 硬约束）一步都进不去 ⇒ `A x`（模板）与
//!    `Set.mem x A`（期望类型，来自 `Set.subset` 的**定义体**、是点名应用链）
//!    明明只差一层 delta（`Set.mem a A` 的定义体就是 `A a`）却判 No ✗；
//! 2. 那次走不通的结构尝试会在 `first_err` 上留下**粘性**的 `Clash` ⇒ 兜底把这一对
//!    判成 `Yes` 之后，出口 `discharge` 仍按 `Clash` 归因 ✗。
//!
//! 两处都修好之后，**期望类型是点名应用链**与**期望类型是记法节点**（用户签名里写的
//! `x ∈ A`，走不到结构分解那条路）**判得一样** ✓。
//!
//! **Lean 4 对照**（`~/Documents/lean/lean4/src/Lean/Meta/ExprDefEq.lean`）：
//! * `:1990` `isDefEqApp` —— 结构分解（`isDefEqArgs`）判否**不提前收摊**，而是接着走
//!   `isDefEqOnFailure`（whnf / delta 那一路）✓；
//! * `:1997`/`:2001` 那次结构尝试包在 `checkpointDefEq` 里 —— 失败 ⇒ `s.restore`
//!   **把状态整个回滚**（`Meta/Basic.lean:2397`）⇒ 探索期的错误**不粘** ✓。
//!
//! ⚠ **已知边界（本文件第 3 条用例钉住它）**：`?p ?x ≟ <点名应用链>` 且 `?x` 那一刻
//! **还没解出**时，结构分解会把 `?p` 配到**半个应用**上（`mem a`）⇒ 仍然报同一条诊断。
//! 那条要走 **mvar 应用 / Miller 模式合一**，而 `docs/design/metavar-engine.md` §1.2
//! 第 16 行**明确不做**（"报好错，不搜索"）⇒ 本条**不修**，只钉住当前行为防漂移。

use super::*;
use crate::parse;

/// 用户原案的单文件等价物（`import lib.Set` 换成同名 def；`∈` 记法保留 —— 它正是
/// 「`h` 的类型是**记法节点**」那一半，见第 2 条根因）。
const REPRO: &str = "\
def Set (α : Type) : Type := α → Prop
def mem {α : Type} (a : α) (A : Set α) : Prop := A a
def subset {α : Type} (A B : Set α) : Prop := ∀ (x : α), mem x A → mem x B
def singleton {α : Type} (a : α) : Set α := fun (x : α) => x = a
infix:50 \" ∈ \" => mem

theorem t (α : Type) (A : Set α) (a : α) (h : a ∈ A) : subset (singleton a) A := by
  intro x hx
  unfold singleton mem at hx
  apply Eq.subst (Eq.symm hx) h
";

fn compile(src: &str) -> CompileOutput {
    let file = parse(src).unwrap_or_else(|e| panic!("parse failed: {e:?}"));
    compile_fol(&file)
}

fn checked_names(out: &CompileOutput) -> Vec<String> {
    out.events
        .iter()
        .filter_map(|e| match e {
            CheckEvent::DeclarationChecked { name } => Some(name.clone()),
            _ => None,
        })
        .collect()
}

/// **判据 ①（用户原案）**：删掉 `sorry` 的证明必须**判通过**（`decl.checked`、0 诊断）。
///
/// 改前（基线二进制，逐字）：`errors = [elab-implicit-argument-unsolved @ 第 10 行
/// 「…两条线索互相矛盾…」]`、`t` **不**在 `checked_names` 里 ✗。
#[test]
fn eqsubst_solves_the_predicate_when_the_goal_is_a_point_name_spine() {
    let out = compile(REPRO);
    assert_eq!(out.errors, vec![], "用户原案必须 0 诊断：{:?}", out.errors);
    assert!(
        checked_names(&out).contains(&"t".to_string()),
        "`t` 必须判通过：{:?}",
        checked_names(&out)
    );
}

/// **判据 ②（正对照：真的类型错仍必须报）**：`h` 的等式搬的是 `b`，而目标要的是 `x`
/// ⇒ 证明项对不上 ⇒ **必须判红**（修法不许把这一位变成"随便给个谓词都行" ✗）。
#[test]
fn a_genuinely_wrong_predicate_is_still_rejected() {
    let bad = REPRO.replace("(a : α) (h : a ∈ A)", "(a b : α) (h : b ∈ A)");
    let out = compile(&bad);
    assert!(
        !out.errors.is_empty(),
        "`h : b ∈ A` 搬不到 `x` 上 ⇒ 必须判红（改前改后都红）"
    );
    assert!(
        !checked_names(&out).contains(&"t".to_string()),
        "判红 ⇒ 不许发 `decl.checked`：{:?}",
        checked_names(&out)
    );
}

/// **已知边界（不是修好了，是钉住现状）**：`h` 的类型是**点名应用链**（没有记法）
/// 时，`?p ?x ≟ mem a A` 会在 `?x` 还没解出的那一刻被结构分解成
/// `?p := mem a`（半个应用）+ `?x := A` ⇒ 仍报同一条诊断。
///
/// 它要的是 **mvar 应用 / Miller 模式合一**（`?p x ≟ mem x A` ⇒ `?p := λz. mem z A`），
/// 而 `docs/design/metavar-engine.md` §1.2 第 16 行把这一格**明确列为不做**。
/// ⚠ 本用例**断言的是当前行为**：将来谁把那一格做了，这里**应当判红**（是升级，不是回归）✓。
#[test]
fn a_point_name_hypothesis_still_reports_the_same_diagnostic() {
    let no_notation = REPRO
        .replace("infix:50 \" ∈ \" => mem\n", "")
        .replace("(h : a ∈ A)", "(h : mem a A)");
    let out = compile(&no_notation);
    assert_eq!(out.errors.len(), 1, "errors: {:?}", out.errors);
    assert!(
        out.errors[0].message.contains("两条线索互相矛盾"),
        "边界形状的文案：{}",
        out.errors[0].message
    );
    assert!(
        !checked_names(&out).contains(&"t".to_string()),
        "边界形状仍然判红：{:?}",
        checked_names(&out)
    );
}

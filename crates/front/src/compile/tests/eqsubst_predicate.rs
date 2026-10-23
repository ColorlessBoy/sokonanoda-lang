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

/// **2026-10-22 unit02 现场**：`apply Eq.subst _ hx` 的洞在 **Set 谓词**上。
///
/// 用户原题：
/// ```text
/// intro h x hx
/// apply Eq.subst _ hx
/// exact h
/// ```
/// 目标 `x ∈ ∅`、`hx : x ∈ A`、`h : A = ∅`。motive 是 `fun (s : Set α) => x ∈ s`；
/// a14 的“洞 → 子目标”只在元素等式上够用，Set/函数等式要合成 **old-style 前缀**。
/// 判据：0 诊断 + `t` 在 `checked_names` 里（洞真的变成 `A = ∅` 子目标，
/// `exact h` 才闭得上）。
#[test]
fn apply_eqsubst_argument_hole_on_a_set_predicate() {
    let src = "\
def Set (α : Type) : Type := α → Prop
def mem {α : Type} (a : α) (A : Set α) : Prop := A a
def empty {α : Type} : Set α := fun (_ : α) => False
infix:50 \" ∈ \" => mem

theorem t (α : Type) (A : Set α) (x : α) (hx : x ∈ A) (h : A = empty) : x ∈ empty := by
  apply Eq.subst _ hx
  exact h
";
    let out = compile(src);
    assert_eq!(
        out.errors,
        vec![],
        "Set 谓词的实参位洞必须判绿：{:?}",
        out.errors
    );
    assert!(
        checked_names(&out).contains(&"t".to_string()),
        "`t` 必须 checked：{:?}",
        checked_names(&out)
    );
}

/// **函数谓词**同一条（`A B : α → Prop`，目标 `B x`、`hx : A x`）——
/// 证明修法不是只为 `Set` 写死。
#[test]
fn apply_eqsubst_argument_hole_on_a_function_predicate() {
    let src = "\
theorem t (α : Type) (A B : α → Prop) (x : α) (hx : A x) (h : A = B) : B x := by
  apply Eq.subst _ hx
  exact h
";
    let out = compile(src);
    assert_eq!(
        out.errors,
        vec![],
        "函数谓词的实参位洞必须判绿：{:?}",
        out.errors
    );
    assert!(checked_names(&out).contains(&"t".to_string()));
}

/// **反向**：洞的谓词被合成后，`exact` 给的等式必须真的能闭合 `a = b`；
/// 方向/对象不对仍然判红（修法不许变成“随便给个等式都行”）。
#[test]
fn apply_eqsubst_argument_hole_with_a_wrong_witness_is_still_rejected() {
    let src = "\
def Set (α : Type) : Type := α → Prop
def mem {α : Type} (a : α) (A : Set α) : Prop := A a
def empty {α : Type} : Set α := fun (_ : α) => False
infix:50 \" ∈ \" => mem

theorem t (α : Type) (A : Set α) (x b : α) (hx : b ∈ A) (h : A = empty) : x ∈ empty := by
  apply Eq.subst _ hx
  exact h
";
    let out = compile(src);
    assert!(
        !out.errors.is_empty(),
        "`hx : b ∈ A` 搬不到 `x` 上 ⇒ 必须判红"
    );
    assert!(!checked_names(&out).contains(&"t".to_string()));
}

// ─────────────────────────────────────────────────────────────────────────────
// **2026-10-23 用户亲报（unit02 的姐妹形状）**：`apply Eq.subst h hx`
// —— 等号位**不是洞**，两个实参都给。初学者最先想到的就是这一句。
// ─────────────────────────────────────────────────────────────────────────────

/// 现场：
/// ```text
/// intro h x hx
/// apply Eq.subst h hx      -- ✗ 「4 个隐式参数补不出来 / 两条线索互相矛盾」
/// ```
/// 根因与 2026-10-22 那条同源：谓词 `p` 只能从 `hx` 的类型与目标**反解**，而
/// `hx : x ∈ A` 是**记法节点** ⇒ 结构抽取器看不穿（`tests/eqsubst_predicate.rs`
/// 第 3 条钉的边界）⇒ `p` 解不出。
///
/// 判据：0 诊断 + `t` 在 `checked_names` 里（合成 old-style 前缀
/// `Eq.subst (Set α) (fun s => x ∈ s) A empty h hx`，整条由内核终审）。
#[test]
fn apply_eqsubst_with_the_equality_given_on_a_set_predicate() {
    let src = "\
def Set (α : Type) : Type := α → Prop
def mem {α : Type} (a : α) (A : Set α) : Prop := A a
def empty {α : Type} : Set α := fun (_ : α) => False
infix:50 \" ∈ \" => mem

theorem t (α : Type) (A : Set α) (x : α) (hx : x ∈ A) (h : A = empty) : x ∈ empty := by
  apply Eq.subst h hx
";
    let out = compile(src);
    assert_eq!(
        out.errors,
        vec![],
        "`apply Eq.subst h hx`（记法谓词）必须判绿：{:?}",
        out.errors
    );
    assert!(
        checked_names(&out).contains(&"t".to_string()),
        "`t` 必须 checked：{:?}",
        checked_names(&out)
    );
}

/// 同一条走 **`exact`**（课程里 term 风格与 tactic 风格都教）—— 也必须判绿。
#[test]
fn exact_eqsubst_with_the_equality_given_on_a_set_predicate() {
    let src = "\
def Set (α : Type) : Type := α → Prop
def mem {α : Type} (a : α) (A : Set α) : Prop := A a
def empty {α : Type} : Set α := fun (_ : α) => False
infix:50 \" ∈ \" => mem

theorem t (α : Type) (A : Set α) (x : α) (hx : x ∈ A) (h : A = empty) : x ∈ empty := by
  exact Eq.subst h hx
";
    let out = compile(src);
    assert_eq!(
        out.errors,
        vec![],
        "`exact Eq.subst h hx` 必须判绿：{:?}",
        out.errors
    );
    assert!(checked_names(&out).contains(&"t".to_string()));
}

/// **反向 ①（方向）**：`h : empty = A` 搬不过去（`Eq.subst` 只做 `a → b`）⇒ 必须判红。
#[test]
fn apply_eqsubst_with_the_equality_given_rejects_the_wrong_direction() {
    let src = "\
def Set (α : Type) : Type := α → Prop
def mem {α : Type} (a : α) (A : Set α) : Prop := A a
def empty {α : Type} : Set α := fun (_ : α) => False
infix:50 \" ∈ \" => mem

theorem t (α : Type) (A : Set α) (x : α) (hx : x ∈ A) (h : empty = A) : x ∈ empty := by
  apply Eq.subst h hx
";
    let out = compile(src);
    assert!(
        !out.errors.is_empty(),
        "`h : empty = A` 方向反了 ⇒ 必须判红"
    );
    assert!(!checked_names(&out).contains(&"t".to_string()));
}

/// **反向 ②（对象）**：前提搬的是别的元素（`hx : b ∈ A`）⇒ 必须判红。
#[test]
fn apply_eqsubst_with_the_equality_given_rejects_a_wrong_premise() {
    let src = "\
def Set (α : Type) : Type := α → Prop
def mem {α : Type} (a : α) (A : Set α) : Prop := A a
def empty {α : Type} : Set α := fun (_ : α) => False
infix:50 \" ∈ \" => mem

theorem t (α : Type) (A : Set α) (x b : α) (hx : b ∈ A) (h : A = empty) : x ∈ empty := by
  apply Eq.subst h hx
";
    let out = compile(src);
    assert!(
        !out.errors.is_empty(),
        "`hx : b ∈ A` 搬不到 `x` 上 ⇒ 必须判红"
    );
    assert!(!checked_names(&out).contains(&"t".to_string()));
}

/// **正对照（点名形状本来就过）**：新形状**只在记法谓词上触发**，点名形状
/// （`mem x A` / `mem x B`）本来就能靠通用反解过 —— 不许把它改坏 / 改慢。
#[test]
fn apply_eqsubst_with_the_equality_given_keeps_the_point_name_shape_green() {
    let src = "\
def Set (α : Type) : Type := α → Prop
def mem {α : Type} (a : α) (A : Set α) : Prop := A a

theorem t (α : Type) (A B : Set α) (x : α) (hx : mem x A) (h : A = B) : mem x B := by
  apply Eq.subst h hx
";
    let out = compile(src);
    assert_eq!(out.errors, vec![], "点名形状必须照旧判绿：{:?}", out.errors);
    assert!(checked_names(&out).contains(&"t".to_string()));
}

/// **同一形状、另一种记法**：谓词是 `⊆`（`Set.subset` 是 def，目标本身是 Pi）。
///
/// `hx : A ⊆ C`、`h : C = D` ⇒ `A ⊆ D`。合成
/// `Eq.subst (Set α) (fun s => A ⊆ s) C D h hx`；整条由内核终审。
#[test]
fn apply_eqsubst_with_the_equality_given_on_a_subset_predicate() {
    let src = "\
def Set (α : Type) : Type := α → Prop
def subset {α : Type} (A B : Set α) : Prop := ∀ (x : α), A x → B x
infix:50 \" ⊆ \" => subset

theorem t (α : Type) (A C D : Set α) (hx : A ⊆ C) (h : C = D) : A ⊆ D := by
  apply Eq.subst h hx
";
    let out = compile(src);
    assert_eq!(out.errors, vec![], "`⊆` 谓词也必须判绿：{:?}", out.errors);
    assert!(checked_names(&out).contains(&"t".to_string()));
}

/// **已知边界（钉住现状，不是修好了）**：`apply` 会**一直剥 Π** —— 当被应用项的
/// 结果类型是 `def` 头、展开后是 Π/箭头（`A ⊆ D` → `∀ x, A x → D x`）时，
/// 它把**目标本身**也当函数剥掉两层 ⇒「结果是 `D x`，无法对齐当前目标 `A ⊆ D`」。
///
/// 现场（2026-10-23 顺带发现，与 `Eq.subst` 的隐式反解**无关**）：
/// ```text
/// apply Eq.subst (Set α) (fun (X : Set α) => A ⊆ X) C D h
/// exact hx
/// ```
/// 同一条项写成 `exact @Eq.subst.{1} … h hx` 判绿（`exact` 不剥 Π）。
///
/// ⚠ **本用例断言的是当前行为**：将来谁让 `apply` 按「已写实参 vs 声明元数」
/// 决定剥几层，这里**应当判红**（是升级，不是回归）✓。
#[test]
fn apply_with_a_def_headed_motive_still_over_peels_the_goal() {
    let src = "\
def Set (α : Type) : Type := α → Prop
def subset {α : Type} (A B : Set α) : Prop := ∀ (x : α), A x → B x
infix:50 \" ⊆ \" => subset

theorem t (α : Type) (A C D : Set α) (h : C = D) (hx : A ⊆ C) : A ⊆ D := by
  apply Eq.subst (Set α) (fun (X : Set α) => A ⊆ X) C D h
  exact hx
";
    let out = compile(src);
    assert!(
        !out.errors.is_empty(),
        "已知边界：`apply` 把目标本身剥掉了 ⇒ 今天仍判红"
    );
    assert!(
        out.errors[0].message.contains("无法对齐当前目标"),
        "边界形状的文案（剥 Π 过头）：{}",
        out.errors[0].message
    );
    assert!(!checked_names(&out).contains(&"t".to_string()));
}

/// **应用端差异**（`A` vs `sing a`，没有记法）：真实课程里 `∅` 经内核降级成
/// 应用形态后就是这一类（`x ∈ A` vs `x ∈ ∅` 展开成 `mem x (Set.empty …)`）。
/// ⚠ 这一条**通用反解本来也能过**（自包含夹具里解得出）—— 它钉的是
/// **改写不许把它改坏**（改写会触发：差异端是应用）。
#[test]
fn apply_eqsubst_with_the_equality_given_on_a_definition_application() {
    let src = "\
def Set (α : Type) : Type := α → Prop
def mem {α : Type} (a : α) (A : Set α) : Prop := A a
def sing {α : Type} (a : α) : Set α := fun (x : α) => x = a

theorem t (α : Type) (A : Set α) (x a : α) (hx : mem x A) (h : A = sing a)
    : mem x (sing a) := by
  apply Eq.subst h hx
";
    let out = compile(src);
    assert_eq!(out.errors, vec![], "应用端差异必须判绿：{:?}", out.errors);
    assert!(checked_names(&out).contains(&"t".to_string()));
}

/// **真实课程的形状**：目标经 `subset`（def）展开、`hx` 由 `intro` 得到
/// （类型是内核展开后的应用链）—— unit02 的原案就是这个形状。
/// ⚠ 自包含夹具里通用反解也能过（真实课程失败要靠 lib 的 pp 带占位——见
/// `STATUS.md` 2026-10-23 那条）；它同样钉「改写不许改坏」。
#[test]
fn apply_eqsubst_with_the_equality_given_after_intro_through_a_definition() {
    let src = "\
def Set (α : Type) : Type := α → Prop
def mem {α : Type} (a : α) (A : Set α) : Prop := A a
def empty {α : Type} : Set α := fun (_ : α) => False
def subset {α : Type} (A B : Set α) : Prop := ∀ (x : α), mem x A → mem x B

theorem t (α : Type) (A : Set α) (h : A = empty) : subset A empty := by
  intro x hx
  apply Eq.subst h hx
";
    let out = compile(src);
    assert_eq!(
        out.errors,
        vec![],
        "intro 展开后的形状必须判绿：{:?}",
        out.errors
    );
    assert!(checked_names(&out).contains(&"t".to_string()));
}

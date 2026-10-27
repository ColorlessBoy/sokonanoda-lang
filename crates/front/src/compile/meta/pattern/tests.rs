//! **HO3a** 的真值层单测（`docs/design/metavar-pattern.md` §4 的 14 条矩阵里 HO3a 的那半）。
//!
//! ⚠ **不读环境**：全部直接调 [`MetaCtx::pattern_assign`]（不经 `SOKO_META_PATTERN` 闸）——
//! 单测里读/设 `SOKO_*` 会与同进程其它用例竞态（`scripts/check-test-env-isolation.py` 的纪律）。
//! 闸门本身的判据是**进程级**的：7 文件 `--json` + M0 13 形状（见交付报告）。

use std::rc::Rc;

use super::*;
use crate::ast::BinderKind;
use crate::compile::meta::{MetaErr, MetaStore};

// ───────────────────────────── 夹具 ─────────────────────────────

fn ident(name: &str) -> Expr {
    Expr::Ident {
        name: name.to_string(),
        span: Span::default(),
    }
}

fn app(fun: Expr, arg: Expr) -> Expr {
    Expr::App {
        fun: Box::new(fun),
        arg: Box::new(arg),
        explicit_spine: false,
        span: Span::default(),
    }
}

/// 期望形态：`fun (name : ty) => body`（与 `abstract_over` 同构：逐层单 binder）。
fn lambda(name: &str, ty: Expr, body: Expr) -> Expr {
    Expr::Lambda {
        binders: vec![Binder {
            name: name.to_string(),
            ty: Some(Box::new(ty)),
            style: BinderKind::Explicit,
            span: Span::default(),
        }],
        body: Box::new(body),
        span: Span::default(),
    }
}

fn no_unfold(e: &Expr) -> Expr {
    e.clone()
}

/// 检查回调：恒真（本片只验**规则**；生产实现是 HO3b 的 `judge_terms`）。
fn always(_lambda: &Expr, _ty: &Expr) -> bool {
    true
}

/// 检查回调：恒假（判据：**绝不写未检查的 λ**）。
fn never(_lambda: &Expr, _ty: &Expr) -> bool {
    false
}

/// 显式构造 `MetaCtx`：`check = None` ⇒ 非声明通道同形（fail-closed）。
///
/// ⚠ HO3b 起生产 checker 是 [`check`] 的 `judge`（真 `judge_terms`）；这里的
/// `always`/`never` 是**规则臂**的桩（只验规则本身，见 `check.rs` 的真内核判据）。
fn ctx<'a>(unfold: &'a dyn Fn(&Expr) -> Expr, check: Option<CheckFn>) -> MetaCtx<'a> {
    let store: &'a mut MetaStore = Box::leak(Box::new(MetaStore::default()));
    let mut m = MetaCtx::new(unfold, store);
    m.check = check;
    m
}

/// 一份局部上下文：`(名字, 是否 let)`；类型统一给 `T`（只有 binder 注解会被断言）。
fn scope(binders: &[(&str, bool)]) -> Lctx {
    Rc::new(
        binders
            .iter()
            .map(|(name, is_let)| LocalBinder {
                name: (*name).to_string(),
                ty: Some(ident("T")),
                is_let: *is_let,
            })
            .collect(),
    )
}

fn values(m: &MetaCtx<'_>) -> Vec<Option<Expr>> {
    m.store.mvars.iter().map(|v| v.value.clone()).collect()
}

// ───────────────────────────── 分解 / 顺序 ─────────────────────────────

/// ① 分解（strict 臂）：`?m a ≟ f h`（`a` **不在** `lctx(m)` 里 ⇒ 严格 pattern）
/// ⇒ `?m := fun a => f h`，且 `zonk(?m a) = f h`。
#[test]
fn strict_pattern_abstracts_and_zonks() {
    let mut m = ctx(&no_unfold, Some(Rc::new(always)));
    m.lctx = scope(&[("h", false)]);
    let meta = m.fresh(ident("T"), MetaKind::Natural, vec![]);
    // **注入**额外局部 `a`（strict 的条件 2：实参不在 lctx(m)，但在当前上下文里）
    m.lctx = scope(&[("h", false), ("a", false)]);
    let lhs = app(m.meta_expr(meta), ident("a"));
    let rhs = app(ident("f"), ident("h"));
    assert_eq!(m.pattern_assign(&lhs, &rhs), Tri::Yes);
    assert_eq!(m.value(meta), Some(lambda("a", ident("T"), rhs.clone())));
    assert_eq!(m.zonk(&lhs), rhs);
}

/// ① 分解（quasi 最小形）：`?m a ≟ f a` ⇒ `?m := fun a => f a`，且 `zonk(?m a) = f a`
/// （`substitute` 代入即消 redex ⇒ 不需要额外的 β pass）。
#[test]
fn decomposition_abstracts_and_zonks() {
    let mut m = ctx(&no_unfold, Some(Rc::new(always)));
    // **strict**：先建 m（空 lctx）再设当前 lctx，实参不在 lctx(m) 里。
    let meta = m.fresh(ident("T"), MetaKind::Natural, vec![]);
    m.lctx = scope(&[("a", false)]);
    let lhs = app(m.meta_expr(meta), ident("a"));
    let rhs = app(ident("f"), ident("a"));
    assert_eq!(m.pattern_assign(&lhs, &rhs), Tri::Yes);
    assert_eq!(m.value(meta), Some(lambda("a", ident("T"), rhs.clone())));
    assert_eq!(m.zonk(&lhs), rhs);
}

/// ①' **quasi 捕获闸**：`?m A ≟ ¬ (x ∈ A)`，`A` 在 `lctx(m)` 里且 `A` 在 t 里自由
/// ⇒ 抽象 `fun A => t` 会把 t 里外层 `A` 一起绑掉（丢失依赖）⇒ 弃权。
#[test]
fn quasi_with_arg_free_in_term_is_declined() {
    let mut m = ctx(&no_unfold, Some(Rc::new(always)));
    m.lctx = scope(&[("A", false)]);
    let meta = m.fresh(ident("Prop"), MetaKind::Natural, vec![]);
    // `?m A` / `¬ (Set.mem x A)`（源级点名形态；记法节点与它等价，见 `desugar_notation`）
    let lhs = app(m.meta_expr(meta), ident("A"));
    let rhs = app(
        ident("Not"),
        app(app(ident("Set.mem"), ident("x")), ident("A")),
    );
    assert_eq!(m.pattern_assign(&lhs, &rhs), Tri::Undef);
    assert_eq!(m.value(meta), None);
}

/// ② 多参顺序：`?m a b ≟ g b a` ⇒ `fun a => fun b => g b a`（实参顺序**照写**）。
#[test]
fn multi_argument_abstraction_keeps_order() {
    let mut m = ctx(&no_unfold, Some(Rc::new(always)));
    let meta = m.fresh(ident("T"), MetaKind::Natural, vec![]);
    m.lctx = scope(&[("a", false), ("b", false)]);
    let lhs = app(app(m.meta_expr(meta), ident("a")), ident("b"));
    let rhs = app(app(ident("g"), ident("b")), ident("a"));
    assert_eq!(m.pattern_assign(&lhs, &rhs), Tri::Yes);
    let expected = lambda(
        "a",
        ident("T"),
        lambda(
            "b",
            ident("T"),
            app(app(ident("g"), ident("b")), ident("a")),
        ),
    );
    assert_eq!(m.value(meta), Some(expected));
    assert_eq!(m.zonk(&lhs), rhs);
}

/// 右部是 pattern 也认（左右同序：先左后右）。
#[test]
fn pattern_on_the_right_is_recognized() {
    let mut m = ctx(&no_unfold, Some(Rc::new(always)));
    let meta = m.fresh(ident("T"), MetaKind::Natural, vec![]);
    m.lctx = scope(&[("a", false)]);
    let lhs = app(ident("f"), ident("a"));
    let rhs = app(m.meta_expr(meta), ident("a"));
    assert_eq!(m.pattern_assign(&lhs, &rhs), Tri::Yes);
    assert_eq!(m.value(meta), Some(lambda("a", ident("T"), lhs.clone())));
}

// ───────────────────────────── 否定臂（全部 `Undef`、状态零改动）─────────────────────────────

/// ③ occurs：`?m a ≟ f ?m` ⇒ `Undef`（`assign` 的 occurs 闸原样复用）。
#[test]
fn occurs_is_undef() {
    let mut m = ctx(&no_unfold, Some(Rc::new(always)));
    m.lctx = scope(&[("a", false)]);
    let meta = m.fresh(ident("T"), MetaKind::Natural, vec![]);
    let lhs = app(m.meta_expr(meta), ident("a"));
    let rhs = app(ident("f"), m.meta_expr(meta));
    assert_eq!(m.pattern_assign(&lhs, &rhs), Tri::Undef);
    assert_eq!(m.value(meta), None);
}

/// ④ type-occurs：解里的元变量其**类型**提到 `?m` ⇒ `Undef`。
#[test]
fn type_occurs_is_undef() {
    let mut m = ctx(&no_unfold, Some(Rc::new(always)));
    m.lctx = scope(&[("a", false)]);
    let meta = m.fresh(ident("T"), MetaKind::Natural, vec![]);
    // `?other : Set ?m`
    let other = m.fresh(
        app(ident("Set"), m.meta_expr(meta)),
        MetaKind::Natural,
        vec![],
    );
    let lhs = app(m.meta_expr(meta), ident("a"));
    let rhs = app(ident("f"), m.meta_expr(other));
    assert_eq!(m.pattern_assign(&lhs, &rhs), Tri::Undef);
    assert_eq!(m.value(meta), None);
    assert_eq!(m.value(other), None);
}

/// ⑤ 作用域：解里提到 m 的 `out_of_scope` 名字 ⇒ `Undef`（`Scope` 与 occurs 分开）。
#[test]
fn out_of_scope_is_undef() {
    let mut m = ctx(&no_unfold, Some(Rc::new(always)));
    m.lctx = scope(&[("a", false)]);
    let meta = m.fresh(ident("T"), MetaKind::Natural, vec!["b".to_string()]);
    let lhs = app(m.meta_expr(meta), ident("a"));
    let rhs = app(ident("f"), ident("b"));
    assert_eq!(m.pattern_assign(&lhs, &rhs), Tri::Undef);
    assert_eq!(m.value(meta), None);
}

/// ⑥ 重复实参：`?m a a` **不是** pattern（Lean 条件 1）⇒ `Undef`。
#[test]
fn duplicate_arguments_are_undef() {
    let mut m = ctx(&no_unfold, Some(Rc::new(always)));
    m.lctx = scope(&[("a", false)]);
    let meta = m.fresh(ident("T"), MetaKind::Natural, vec![]);
    let lhs = app(app(m.meta_expr(meta), ident("a")), ident("a"));
    let rhs = app(ident("f"), ident("a"));
    assert_eq!(m.pattern_assign(&lhs, &rhs), Tri::Undef);
    assert_eq!(m.value(meta), None);
}

/// ⑦ 非 fvar 实参：`?m (f a)` ⇒ `Undef`。
#[test]
fn non_fvar_argument_is_undef() {
    let mut m = ctx(&no_unfold, Some(Rc::new(always)));
    m.lctx = scope(&[("a", false)]);
    let meta = m.fresh(ident("T"), MetaKind::Natural, vec![]);
    let lhs = app(m.meta_expr(meta), app(ident("f"), ident("a")));
    let rhs = app(ident("f"), ident("a"));
    assert_eq!(m.pattern_assign(&lhs, &rhs), Tri::Undef);
    assert_eq!(m.value(meta), None);
}

/// ⑦b 实参不是局部名（不在任何 lctx 里，如全局常量）⇒ `Undef`。
#[test]
fn argument_outside_lctx_is_undef() {
    let mut m = ctx(&no_unfold, Some(Rc::new(always)));
    m.lctx = scope(&[("h", false)]);
    let meta = m.fresh(ident("T"), MetaKind::Natural, vec![]);
    let lhs = app(m.meta_expr(meta), ident("c"));
    let rhs = app(ident("f"), ident("c"));
    assert_eq!(m.pattern_assign(&lhs, &rhs), Tri::Undef);
    assert_eq!(m.value(meta), None);
}

/// ⑦c `let` 绑名**不是 fvar**：`?m a`（`a` 是 let）⇒ `Undef`。
#[test]
fn let_bound_argument_is_undef() {
    let mut m = ctx(&no_unfold, Some(Rc::new(always)));
    m.lctx = scope(&[("a", true)]);
    let meta = m.fresh(ident("T"), MetaKind::Natural, vec![]);
    let lhs = app(m.meta_expr(meta), ident("a"));
    let rhs = app(ident("f"), ident("a"));
    assert_eq!(m.pattern_assign(&lhs, &rhs), Tri::Undef);
    assert_eq!(m.value(meta), None);
}

/// ⑧ 没有 checker ⇒ **弃权**（fail-closed；今天所有生产构造点都是这一档）。
#[test]
fn no_checker_declines() {
    let mut m = ctx(&no_unfold, None);
    m.lctx = scope(&[("a", false)]);
    let meta = m.fresh(ident("T"), MetaKind::Natural, vec![]);
    let lhs = app(m.meta_expr(meta), ident("a"));
    let rhs = app(ident("f"), ident("a"));
    assert_eq!(m.pattern_assign(&lhs, &rhs), Tri::Undef);
    assert_eq!(m.value(meta), None);
}

/// ⑨ checker 说 false ⇒ `Undef` 且**零赋值**（"绝不写未检查的 λ"）。
#[test]
fn rejecting_checker_writes_nothing() {
    let mut m = ctx(&no_unfold, Some(Rc::new(never)));
    m.lctx = scope(&[("a", false)]);
    let meta = m.fresh(ident("T"), MetaKind::Natural, vec![]);
    let before = values(&m);
    let lhs = app(m.meta_expr(meta), ident("a"));
    let rhs = app(ident("f"), ident("a"));
    assert_eq!(m.pattern_assign(&lhs, &rhs), Tri::Undef);
    assert_eq!(m.value(meta), None);
    assert_eq!(values(&m), before);
}

/// ⑩ 预算耗尽 ⇒ `Undef`（**不是** `No` —— G-88：判不了只许弃权）。
#[test]
fn exhausted_budget_is_undef_not_no() {
    let mut m = ctx(&no_unfold, Some(Rc::new(always)));
    m.lctx = scope(&[("a", false)]);
    let meta = m.fresh(ident("T"), MetaKind::Natural, vec![]);
    let lhs = app(m.meta_expr(meta), ident("a"));
    let rhs = app(ident("f"), ident("a"));
    m.fuel = 0;
    assert_eq!(m.pattern_assign(&lhs, &rhs), Tri::Undef);
    assert_eq!(m.value(meta), None);
}

/// ⑪ 待定约束闸：`postponed` 里有把 `?m` 往别的方向拉的约束 ⇒ quasi 弃权。
#[test]
fn postponed_constraint_declines_quasi() {
    let mut m = ctx(&no_unfold, Some(Rc::new(always)));
    m.lctx = scope(&[("a", false)]);
    let meta = m.fresh(ident("T"), MetaKind::Natural, vec![]);
    let pending = m.meta_expr(meta);
    m.store.postponed.push((pending, ident("Nat")));
    let lhs = app(m.meta_expr(meta), ident("a"));
    let rhs = app(ident("f"), ident("a"));
    assert_eq!(m.pattern_assign(&lhs, &rhs), Tri::Undef);
    assert_eq!(m.value(meta), None);
    assert_eq!(m.store.postponed.len(), 1);
}

/// ⑫ 回滚：弃权时 `values` / `postponed` / `first_err` **逐项不变**。
#[test]
fn decline_leaves_state_untouched() {
    let mut m = ctx(&no_unfold, Some(Rc::new(always)));
    m.lctx = scope(&[("a", false)]);
    let meta = m.fresh(ident("T"), MetaKind::Natural, vec![]);
    // 让它在 quasi 的待定闸上弃权（同时确保 postponed 非空 ⇒ 真能看出"没动"）。
    let pending = m.meta_expr(meta);
    m.store.postponed.push((pending, ident("Nat")));
    let before_values = values(&m);
    let before_postponed = m.store.postponed.clone();
    let before_err = m.first_err.clone();
    let lhs = app(m.meta_expr(meta), ident("a"));
    let rhs = app(ident("f"), ident("a"));
    assert_eq!(m.pattern_assign(&lhs, &rhs), Tri::Undef);
    assert_eq!(values(&m), before_values);
    assert_eq!(m.store.postponed, before_postponed);
    assert_eq!(m.first_err, before_err);
}

/// ⑬ 常量解陷阱：`?m a ≟ f b`、`b` 是**当前上下文的局部名**但
/// 不在 `lctx(m) ∪ args` 里 ⇒ `Undef`（**绝不**产 `fun _ => f b`）。
#[test]
fn constant_solution_is_rejected() {
    let mut m = ctx(&no_unfold, Some(Rc::new(always)));
    m.lctx = scope(&[("a", false)]);
    let meta = m.fresh(ident("T"), MetaKind::Natural, vec![]);
    // 当前上下文比 lctx(m) 多一个 `b`
    m.lctx = scope(&[("a", false), ("b", false)]);
    let lhs = app(m.meta_expr(meta), ident("a"));
    let rhs = app(ident("f"), ident("b"));
    assert_eq!(m.pattern_assign(&lhs, &rhs), Tri::Undef);
    assert_eq!(m.value(meta), None);
}

/// 遮蔽闸：`t` 内部有 binder 撞上实参 ⇒ 抽象会捕获 ⇒ `Undef`。
#[test]
fn shadowing_binder_declines() {
    let mut m = ctx(&no_unfold, Some(Rc::new(always)));
    m.lctx = scope(&[("a", false)]);
    let meta = m.fresh(ident("T"), MetaKind::Natural, vec![]);
    let lhs = app(m.meta_expr(meta), ident("a"));
    // `fun (a : T) => a`：内层 binder 撞实参 `a`
    let rhs = lambda("a", ident("T"), ident("a"));
    assert_eq!(m.pattern_assign(&lhs, &rhs), Tri::Undef);
    assert_eq!(m.value(meta), None);
}

/// 洞（`SyntheticOpaque`）永不被 pattern 赋。
#[test]
fn synthetic_opaque_is_never_assigned() {
    let mut m = ctx(&no_unfold, Some(Rc::new(always)));
    m.lctx = scope(&[("a", false)]);
    let meta = m.fresh(ident("T"), MetaKind::SyntheticOpaque, vec![]);
    let lhs = app(m.meta_expr(meta), ident("a"));
    let rhs = app(ident("f"), ident("a"));
    assert_eq!(m.pattern_assign(&lhs, &rhs), Tri::Undef);
    assert_eq!(m.value(meta), None);
}

/// 已赋值的 `?m` 不再进来（`zonk` 之后它不是 pattern；`assign` 也不许覆盖）。
#[test]
fn assigned_meta_is_not_reassigned() {
    let mut m = ctx(&no_unfold, Some(Rc::new(always)));
    m.lctx = scope(&[("a", false)]);
    let meta = m.fresh(ident("T"), MetaKind::Natural, vec![]);
    m.assign(meta, ident("F")).expect("先赋成常量函数值");
    let lhs = app(m.meta_expr(meta), ident("a"));
    let rhs = app(ident("f"), ident("a"));
    assert_eq!(m.pattern_assign(&lhs, &rhs), Tri::Undef);
    assert_eq!(m.value(meta), Some(ident("F")));
}

// ───────────────────── **HO3b · FO 近似**（`SOKO_META_APPROX` 的那一档）─────────────────────
//
// ⚠ 这些用例**直接调 `fo_assign`**（`pattern_try` 里的环境闸不在这里测：读 `SOKO_*` 会与
// 同进程其它用例竞态，见文件头；闸门档位的判据是进程级的 M0 三臂）。
//
// FO 的语义：`?m a₁…aₙ ≟ f b₁…bₖ`（`k < n`、`i = n − k`）⇒ 从右往左 `a_{i+j} ≟ b_j`，
// 再 `?m a₁…aᵢ := f`（Lean `processAssignmentFOApprox`）。**只抽象前缀** ⇒ 解更**不一般**。

/// ① 多参 + 前缀：`?m a b ≟ f b` ⇒ `?m := fun a => f`（`?m a b` 只留 `f b`）。
#[test]
fn fo_abstracts_only_the_prefix() {
    let mut m = ctx(&no_unfold, Some(Rc::new(always)));
    m.lctx = scope(&[("a", false), ("b", false)]);
    let meta = m.fresh(ident("T"), MetaKind::Natural, vec![]);
    let lhs = app(app(m.meta_expr(meta), ident("a")), ident("b"));
    let rhs = app(ident("f"), ident("b"));
    assert_eq!(m.fo_assign(&lhs, &rhs), Tri::Yes);
    assert_eq!(
        m.value(meta),
        Some(lambda("a", ident("T"), ident("f"))),
        "FO 只抽象掉被吃掉的实参 ⇒ 值必须是 `fun a => f`"
    );
    assert_eq!(m.zonk(&lhs), rhs, "`?m a b` 必须回到 `f b`");
}

/// ② 两个实参的**部分**前缀：`?m a b c ≟ g c` ⇒ `?m := fun a => fun b => g`。
#[test]
fn fo_abstracts_a_two_argument_prefix() {
    let mut m = ctx(&no_unfold, Some(Rc::new(always)));
    m.lctx = scope(&[("a", false), ("b", false), ("c", false)]);
    let meta = m.fresh(ident("T"), MetaKind::Natural, vec![]);
    let lhs = app(
        app(app(m.meta_expr(meta), ident("a")), ident("b")),
        ident("c"),
    );
    let rhs = app(ident("g"), ident("c"));
    assert_eq!(m.fo_assign(&lhs, &rhs), Tri::Yes);
    assert_eq!(
        m.value(meta),
        Some(lambda("a", ident("T"), lambda("b", ident("T"), ident("g"))))
    );
    assert_eq!(m.zonk(&lhs), rhs);
}

/// ③ **形状闸**：实参个数不少于 FO 需要的（`k = n` ⇒ `i = 0`）⇒ 弃权（那是裸 `?m ≟ f`）。
#[test]
fn fo_requires_a_strictly_shorter_spine() {
    let mut m = ctx(&no_unfold, Some(Rc::new(always)));
    m.lctx = scope(&[("a", false)]);
    let meta = m.fresh(ident("T"), MetaKind::Natural, vec![]);
    let lhs = app(m.meta_expr(meta), ident("a"));
    let rhs = app(ident("f"), ident("a"));
    assert_eq!(m.fo_assign(&lhs, &rhs), Tri::Undef);
    assert_eq!(m.value(meta), None);
}

/// ④ **回滚（半途）**：逐位 `unify` 先把 `?n := b` 写下去，前缀赋值被 checker 拒 ⇒
/// `values` / `postponed` / `first_err` **逐项**回到原样（设计 §3.2.5「失败 restore」）。
///
/// 反向验证：把 [`MetaCtx::restore`] 的两行删掉 ⇒ `?n` 的值留了下来 ⇒ 本条当场红 ✓。
#[test]
fn fo_rolls_back_a_half_applied_attempt() {
    let mut m = ctx(&no_unfold, Some(Rc::new(never)));
    m.lctx = scope(&[("a", false), ("b", false)]);
    let meta = m.fresh(ident("T"), MetaKind::Natural, vec![]);
    let other = m.fresh(ident("U"), MetaKind::Natural, vec![]);
    m.first_err = Some(MetaErr::Clash); // 非 `None` 才看得出"没被清掉"
    m.store.postponed.push((ident("P"), ident("Q")));
    let before_values = values(&m);
    let before_postponed = m.store.postponed.clone();
    let before_err = m.first_err.clone();
    let lhs = app(app(m.meta_expr(meta), ident("a")), ident("b"));
    // `t = f ?n`：逐位那一步真的会写 `?n := b`。
    let rhs = app(ident("f"), m.meta_expr(other));
    assert_eq!(m.fo_assign(&lhs, &rhs), Tri::Undef);
    assert_eq!(values(&m), before_values, "半途写下的值必须回滚");
    assert_eq!(m.value(other), None, "`?n` 不许留下 `:= b`");
    assert_eq!(m.store.postponed, before_postponed);
    assert_eq!(m.first_err, before_err);
}

/// ⑤ **回滚（待定队列）**：逐位那一步把一条约束压进 `postponed`（`b ≟ g ?n` 判不了）⇒
/// 弃权后队列必须**原样**（不多一条）。
#[test]
fn fo_rolls_back_a_pushed_postponed_constraint() {
    let mut m = ctx(&no_unfold, Some(Rc::new(always)));
    m.lctx = scope(&[("a", false), ("b", false)]);
    let meta = m.fresh(ident("T"), MetaKind::Natural, vec![]);
    let other = m.fresh(ident("U"), MetaKind::Natural, vec![]);
    let before_postponed = m.store.postponed.clone();
    let lhs = app(app(m.meta_expr(meta), ident("a")), ident("b"));
    let rhs = app(ident("f"), app(ident("g"), m.meta_expr(other)));
    assert_eq!(m.fo_assign(&lhs, &rhs), Tri::Undef);
    assert_eq!(m.value(meta), None);
    assert_eq!(
        m.store.postponed, before_postponed,
        "FO 弃权时待定队列必须回滚（否则重扫时会用到一次失败的探针 ✗）"
    );
}

/// ⑥ **ctx 近似不做**：解里出现 `lctx(m) ∪ args` **之外**的局部名 ⇒ 弃权（不 `elimMVarDeps`）。
#[test]
fn fo_ctx_approximation_is_rejected() {
    let mut m = ctx(&no_unfold, Some(Rc::new(always)));
    // `m` 在 `[a, d]` 里创建（`lctx(m)`），随后当前上下文多了一个局部 `localc`。
    m.lctx = scope(&[("a", false), ("d", false)]);
    let meta = m.fresh(ident("T"), MetaKind::Natural, vec![]);
    m.lctx = scope(&[("a", false), ("d", false), ("localc", false)]);
    let lhs = app(app(m.meta_expr(meta), ident("a")), ident("d"));
    // `localc` 是**局部名**但不在 `lctx(m) ∪ args` 里 ⇒ 抽象会越界 ⇒ 弃权。
    let rhs = app(ident("localc"), ident("d"));
    assert_eq!(m.fo_assign(&lhs, &rhs), Tri::Undef);
    assert_eq!(m.value(meta), None);
}

/// ⑦ **预算**：`fuel = 0` ⇒ 一次 `unify` 都不做就弃权（不是判否）。
#[test]
fn fo_respects_the_budget() {
    let mut m = ctx(&no_unfold, Some(Rc::new(always)));
    m.lctx = scope(&[("a", false), ("b", false)]);
    let meta = m.fresh(ident("T"), MetaKind::Natural, vec![]);
    m.fuel = 0;
    let before = values(&m);
    let lhs = app(app(m.meta_expr(meta), ident("a")), ident("b"));
    let rhs = app(ident("f"), ident("b"));
    assert_eq!(m.fo_assign(&lhs, &rhs), Tri::Undef);
    assert_eq!(values(&m), before);
}

/// ⑧ **已赋值的 `?m`** 不进 FO（与 strict/quasi 同一条闸）。
#[test]
fn fo_never_touches_an_assigned_meta() {
    let mut m = ctx(&no_unfold, Some(Rc::new(always)));
    m.lctx = scope(&[("a", false), ("b", false)]);
    let meta = m.fresh(ident("T"), MetaKind::Natural, vec![]);
    m.assign(meta, ident("F")).expect("先赋值");
    let lhs = app(app(m.meta_expr(meta), ident("a")), ident("b"));
    let rhs = app(ident("f"), ident("b"));
    assert_eq!(m.fo_assign(&lhs, &rhs), Tri::Undef);
    assert_eq!(m.value(meta), Some(ident("F")));
}

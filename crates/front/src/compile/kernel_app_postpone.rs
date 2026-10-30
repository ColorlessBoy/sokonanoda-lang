//! **P2-1 (b)**：内核应用路径的**推迟实参**（Lean `Elab/App.lean:433-443` `elabAndAddNewArg`：每个显式实参
//! 对着**实例化后的域** elaborate；`TermElabM.lean` 的 postpone 是同一思路的一般形式）。
//!
//! 流程（[`super::kernel_app`] 的 `run`）：第一趟无期望地 elaborate 实参，失败的（以及顶层 λ 带无注解
//! binder 的——它们无期望时会走 `b2_binder` 造声明级元变量，有副作用 ⇒ **根本不试**）记为推迟（`None`）；
//! 内核 `elab_app` 回 `NeedArgs { domains }`（域无元变量）⇒ 按域 elaborate 推迟位，再带齐调一次。
//!
//! 记法 / 应用形态的实参只读**源级**期望（`expected_src`），读不到内核期望 ⇒ 用 [`ExpectedHint`]：
//! 按 `(span, scope 深度)` 交给**下一次** `kernel_app::try_app`（取走即消费；elaborate 结束无条件清除）。

use super::elab::ElabScope;
use crate::{Expr, Span};
use sokonanoda::transfer::OwnedExpr;
use std::cell::RefCell;

#[cfg(test)]
thread_local! {
    /// 测试用：`true` ⇒ 关掉推迟（反向验证：P2-1 之前的行为）。
    pub(crate) static TEST_NO_POSTPONE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// 推迟是否开着（生产恒开；只在 strict + `SOKO_STRICT_KERNEL_APP=1` 的内核路径里被问到）。
pub(crate) fn enabled() -> bool {
    #[cfg(test)]
    if TEST_NO_POSTPONE.with(|c| c.get()) {
        return false;
    }
    true
}

#[cfg(test)]
thread_local! {
    /// 测试用：`Some(n)` ⇒ 推迟轮数上限改成 `n`（反向验证：`1` = P2-1 的单轮）。
    pub(crate) static TEST_MAX_ROUNDS: std::cell::Cell<Option<usize>> = const { std::cell::Cell::new(None) };
}

/// **P2-2**：推迟轮数上限 = 显式实参数（`NeedArgs` 每轮至少补上一位 ⇒ 正常不会跑满）。
pub(crate) fn max_rounds(explicit: usize) -> usize {
    #[cfg(test)]
    if let Some(n) = TEST_MAX_ROUNDS.with(|c| c.get()) {
        return n;
    }
    explicit
}

thread_local! {
    static HINT: RefCell<Option<(Span, usize, OwnedExpr)>> = const { RefCell::new(None) };
}

/// 推迟位 elaborate 期间的内核期望提示（drop ⇒ 清除，不论有没有被取走）。
#[must_use]
pub(crate) struct ExpectedHint(());

impl ExpectedHint {
    pub(crate) fn set(span: Span, depth: usize, domain: OwnedExpr) -> Self {
        HINT.with(|h| *h.borrow_mut() = Some((span, depth, domain)));
        ExpectedHint(())
    }
}

impl Drop for ExpectedHint {
    fn drop(&mut self) {
        HINT.with(|h| *h.borrow_mut() = None);
    }
}

/// `try_app` 入口：提示正好是这个应用（同 span、同 scope 深度）⇒ 取走它（之后的嵌套应用看不见）。
pub(crate) fn take_hint(span: Span, scope: &ElabScope<'_>) -> Option<OwnedExpr> {
    HINT.with(|h| {
        let mut slot = h.borrow_mut();
        match slot.as_ref() {
            Some((s, d, _)) if *s == span && *d == scope.len() => slot.take().map(|(_, _, o)| o),
            _ => None,
        }
    })
}

/// 只看不取：有一条正对这个节点（同 span、同 scope 深度）的提示吗？（零实参的常量头只在有提示时才进
/// 内核路径，见 `kernel_app::try_hinted_const`。）
pub(crate) fn has_hint(span: Span, scope: &ElabScope<'_>) -> bool {
    HINT.with(|h| {
        h.borrow()
            .as_ref()
            .is_some_and(|(s, d, _)| *s == span && *d == scope.len())
    })
}

/// 顶层 λ 链里有无注解 binder、其余部分 `plain` ⇒ `Some(要剥的 Pi 层数)`（到最后一个无注解 binder 为止）。
/// 这种实参**只能**对着域 elaborate（见模块文档），且域必须有这么多层语法 Pi（[`domain_fits`]）。
pub(crate) fn untyped_lambda_layers(e: &Expr, plain: fn(&Expr) -> bool) -> Option<usize> {
    let (mut cur, mut seen, mut need) = (e, 0usize, 0usize);
    while let Expr::Lambda { binders, body, .. } = cur {
        for b in binders {
            seen += 1;
            match b.ty.as_deref() {
                Some(t) if !plain(t) => return None,
                Some(_) => {}
                None => need = seen,
            }
        }
        cur = body;
    }
    (need > 0 && plain(cur)).then_some(need)
}

/// 域（内核项）至少有 `layers` 层语法 Pi（`elab_expr` 的 λ 臂只按语法 `peel_expected` 剥）。
pub(crate) fn domain_fits(dom: sokonanoda::util::ExprPtr<'_>, layers: usize) -> bool {
    let mut cur = dom;
    for _ in 0..layers {
        match *cur {
            sokonanoda::expr::Expr::Pi { body, .. } => cur = body,
            _ => return false,
        }
    }
    true
}

/// λ 链的 binder 总数与最内层体（推迟位是 λ ⇒ 体的内核期望 = 域剥掉这么多层 Pi，在 `depth + n` 之下）。
pub(crate) fn lambda_body(e: &Expr) -> (usize, &Expr) {
    let (mut cur, mut n) = (e, 0usize);
    while let Expr::Lambda { binders, body, .. } = cur {
        n += binders.len();
        cur = body;
    }
    (n, cur)
}

/// 域剥掉 `n` 层语法 Pi 后的体（不够层 ⇒ `None`）。
pub(crate) fn peel_pis(
    dom: sokonanoda::util::ExprPtr<'_>,
    n: usize,
) -> Option<sokonanoda::util::ExprPtr<'_>> {
    let mut cur = dom;
    for _ in 0..n {
        match *cur {
            sokonanoda::expr::Expr::Pi { body, .. } => cur = body,
            _ => return None,
        }
    }
    Some(cur)
}

/// **P2-3**：推迟位是 λ ⇒ 先 elaborate 它**前导带注解** binder 的类型（到第一个无注解 binder 为止），交给内核
/// `AppRequest::lambda_binders` 与域的 Pi 层合一（Lean `elabFunBinders` 先给出 binder、`Binders.lean:417`
/// `propagateExpectedType` 拿 binder 类型对齐期望——体 elaborate 之前 binder 已经定了）。
/// 第 j 个类型在 scope + 前 j 个 binder 之下；任何一个 elaborate 不了 ⇒ 到此为止（用已有的）。
/// 副作用全撤：scope 深度与 hover 原样交还（之后对着域 elaborate 整个 λ 时会再记一遍）。
#[allow(clippy::too_many_arguments)]
pub(crate) fn typed_binder_tys<'a>(
    builder: &mut sokonanoda::builder::EnvBuilder<'a>,
    e: &Expr,
    scope: &mut ElabScope<'a>,
    univ: &super::elab::UnivMap<'a>,
    known: &super::elab::KnownTable,
    hovers: &mut Vec<super::elab::HoverNode<'a>>,
    ctx: &super::elab::ElabCtx<'a, '_>,
) -> Vec<sokonanoda::util::ExprPtr<'a>> {
    if !binder_unify_enabled() {
        return Vec::new();
    }
    let (depth, mark) = (scope.len(), hovers.len());
    let mut out = Vec::new();
    let mut cur = e;
    'outer: while let Expr::Lambda { binders, body, .. } = cur {
        for b in binders {
            let Some(ty) = b.ty.as_deref() else {
                break 'outer;
            };
            let elaborated =
                super::elab::elab_expr(builder, ty, scope, univ, known, hovers, None, None, ctx);
            scope.truncate(depth + out.len());
            let Ok(t) = elaborated else {
                break 'outer;
            };
            out.push(t);
            scope.push(b.name.clone(), t, Some(ty.clone()), b.span);
        }
        cur = body;
    }
    scope.truncate(depth);
    hovers.truncate(mark);
    out
}

#[cfg(test)]
thread_local! {
    /// 测试用：`true` ⇒ 不交 λ binder 类型（反向验证：P2-3 之前的行为）。
    pub(crate) static TEST_NO_BINDER_UNIFY: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// P2-3 的 binder 合一是否开着（生产恒开）。
pub(crate) fn binder_unify_enabled() -> bool {
    #[cfg(test)]
    if TEST_NO_BINDER_UNIFY.with(|c| c.get()) {
        return false;
    }
    true
}

/// 项里最大的 loose `Var` 下标 + 1（= 它至少要在几个 binder 之下；前端拿不到内核的 `num_loose_bvars`）。
/// 按节点地址记忆 ⇒ DAG 共享子项只走一次。
pub(crate) fn loose_bvars(
    e: sokonanoda::util::ExprPtr<'_>,
    memo: &mut std::collections::HashMap<usize, u32>,
) -> u32 {
    use sokonanoda::expr::Expr as K;
    let key = &*e as *const _ as usize;
    if let Some(n) = memo.get(&key) {
        return *n;
    }
    let under = |n: u32| n.saturating_sub(1);
    let n = match *e {
        K::Var { dbj_idx, .. } => u32::from(dbj_idx) + 1,
        K::App { fun, arg, .. } => loose_bvars(fun, memo).max(loose_bvars(arg, memo)),
        K::Pi {
            binder_type, body, ..
        }
        | K::Lambda {
            binder_type, body, ..
        } => loose_bvars(binder_type, memo).max(under(loose_bvars(body, memo))),
        K::Let { data, .. } => loose_bvars(data.binder_type, memo)
            .max(loose_bvars(data.val, memo))
            .max(under(loose_bvars(data.body, memo))),
        K::Proj { structure, .. } => loose_bvars(structure, memo),
        K::Meta { ty, .. } => loose_bvars(ty, memo),
        K::StringLit { .. } | K::NatLit { .. } | K::Sort { .. } | K::Const { .. } => 0,
    };
    memo.insert(key, n);
    n
}

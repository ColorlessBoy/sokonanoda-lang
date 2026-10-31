//! **P2-10b**：**带注解 binder** 的 λ 实参、体是会走内核路径的应用 / 记法 ⇒ 第一趟**推迟**它，让体对着
//! 域的 codomain elaborate（`kernel_app_postpone::elab_ready` 已经会给 λ 的最内层体设 `ExpectedHint`）。
//!
//! Lean 对照（v4.35.0）：`Elab/App.lean:433-443` `elabAndAddNewArg` 对着 binder 类型 elaborate **每个**实参；
//! `Elab/Binders.lean:410-442` `propagateExpectedType` 每个带注解 binder 之后把期望 Pi 剥一层（含元变量也照传），
//! 体拿到 codomain。之前第一趟用 `None` elaborate 带注解 λ（无副作用、总能成功）⇒ 体里的应用永远看不见
//! codomain（P2-10 探针：3,122 次 `fallback_unsolved` 全是 `expected_src = None` 且无提示）。
//!
//! 单调性（最坏 = 今天的行为）：
//! * 域一直卡在元变量上（内核回 `Unsolved`、这一位还空着）⇒ [`fill_late`] 照第一趟的老办法（无期望）补上、
//!   再解一次 = 今天第一轮的请求；
//! * 对着域 elaborate 失败 ⇒ `elab_ready` 撤掉残留、退回 [`elab_bare`]（无期望）。

use super::elab::{
    elab_expr, resolve_known, ElabCtx, ElabScope, HoverNode, KnownName, KnownTable, UnivMap,
};
use super::kernel_app_probe as probe;
use crate::Expr;
use sokonanoda::builder::EnvBuilder;
use sokonanoda::util::ExprPtr;

#[cfg(test)]
thread_local! {
    /// 测试用：`true` ⇒ 带注解 λ 不推迟（反向验证：P2-10b 之前的行为）。
    pub(crate) static TEST_NO_TYPED_LAMBDA: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// 测试用：本线程 [`elab_bare`] 次数（= 推迟的带注解 λ 退回老办法的次数；判据用它证明重做路径真跑过）。
    pub(crate) static TEST_BARE: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// 见模块文档：`a` 是 binder **全带注解**的 λ 链，最内层体是头为常量（有前导隐式、无宇宙参数）的应用，
/// 或带操作数、目标无宇宙参数的记法 ⇒ 推迟。头被局部名（scope 或 λ 自己的 binder）遮蔽 ⇒ 不是常量头。
pub(crate) fn defer(
    a: &Expr,
    scope: &ElabScope<'_>,
    known: &KnownTable,
    ns: &super::scope::NamespaceScope,
) -> bool {
    #[cfg(test)]
    if TEST_NO_TYPED_LAMBDA.with(|c| c.get()) {
        return false;
    }
    if !matches!(a, Expr::Lambda { .. }) {
        return false;
    }
    let mut locals: Vec<String> = scope.local_names().to_vec();
    let mut cur = a;
    while let Expr::Lambda { binders, body, .. } = cur {
        for b in binders {
            if b.ty.is_none() {
                return false;
            }
            locals.push(b.name.clone());
        }
        cur = body;
    }
    let (target, needs_prefix) = match cur {
        Expr::App { .. } => {
            let (head, _) = crate::spine::spine_of(cur);
            let Expr::Ident { name, span } = head else {
                return false;
            };
            if locals.iter().any(|n| n == name) {
                return false;
            }
            match resolve_known(known, ns, name, *span, &locals) {
                Ok(c) => (c, true),
                Err(_) => return false,
            }
        }
        Expr::Notation {
            target, lhs, rhs, ..
        } if lhs.is_some() || rhs.is_some() => (target.clone(), false),
        _ => return false,
    };
    matches!(
        known.get(&target),
        Some(KnownName::Decl { implicit_prefix, universes, .. })
            if universes.is_empty() && (*implicit_prefix > 0 || !needs_prefix)
    )
}

/// 第一趟的老办法：无期望地 elaborate（探测同第一趟）。失败 ⇒ 残留（hover、λ 臂漏的 binder）撤掉，`None`。
#[allow(clippy::too_many_arguments)]
pub(crate) fn elab_bare<'a>(
    builder: &mut EnvBuilder<'a>,
    a: &Expr,
    scope: &mut ElabScope<'a>,
    univ: &UnivMap<'a>,
    known: &KnownTable,
    hovers: &mut Vec<HoverNode<'a>>,
    ctx: &ElabCtx<'a, '_>,
) -> Option<ExprPtr<'a>> {
    #[cfg(test)]
    TEST_BARE.with(|c| c.set(c.get() + 1));
    let (depth, mark) = (scope.len(), hovers.len());
    let guard = probe::ProbeGuard::enter(builder, a.span(), scope);
    let out = elab_expr(builder, a, scope, univ, known, hovers, None, None, ctx);
    drop(guard);
    if out.is_err() {
        hovers.truncate(mark);
        scope.truncate(depth);
    }
    out.ok()
}

/// 内核回 `Unsolved` 时还空着的推迟 λ 位 ⇒ 按 [`elab_bare`] 补上；补上了至少一位 ⇒ `true`（值得再解一次）。
#[allow(clippy::too_many_arguments)]
pub(crate) fn fill_late<'a>(
    builder: &mut EnvBuilder<'a>,
    args: &[&Expr],
    deferred: &[usize],
    explicit: &mut [Option<ExprPtr<'a>>],
    scope: &mut ElabScope<'a>,
    univ: &UnivMap<'a>,
    known: &KnownTable,
    hovers: &mut Vec<HoverNode<'a>>,
    ctx: &ElabCtx<'a, '_>,
) -> bool {
    let mut progress = false;
    for &i in deferred {
        if explicit[i].is_none() {
            explicit[i] = elab_bare(builder, args[i], scope, univ, known, hovers, ctx);
            progress |= explicit[i].is_some();
        }
    }
    progress
}

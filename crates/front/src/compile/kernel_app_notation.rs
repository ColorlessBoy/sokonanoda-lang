//! **P2-4**：记法节点（前导参数**显式**、要由记法补的那一档：`a ≠ b` ⇒ `Ne α a b`）走内核 `elab_app`。
//!
//! Lean 对照（v4.35.0）：记法是宏，展开成普通应用 `Ne a b`，再由 `elabAppArgs` elaborate
//! （`Elab/App.lean:433-443` 显式位按实例化后的域 elaborate；`563-608` 期望类型先传播）。这里的
//! 偏离：本语言的 `Ne` 把 `α` 写成显式，记法替用户补 ⇒ 前 `missing` 个 binder 按隐式处理
//! （`AppRequest::implicit_prefix`，同构造子参数）；其余与普通应用同一条内核路径（推迟 + 多轮）。
//!
//! 为什么需要：老路（`elab_notation` 的 `notation_prefix_args`）先**无期望**地问第一个操作数的类型
//! （`judge_infer` ⇒ 前缀重编），第一个操作数自己要靠第二个操作数定类型时（unit12
//! `flawed_equalities_refuted`：`(fun _ => ∅) '' … ≠ @Set.univ (Set Nat)`）就解错 ⇒ 内核拒绝。
//! 内核路径里第二个操作数第一趟就解出 `α`，第一个推迟、按域 elaborate —— 正是 Lean 的顺序。
//!
//! 闸门：strict + `SOKO_STRICT_KERNEL_APP=1`；只收前导隐式 0、`missing > 0` 的目标（`implicit_prefix > 0`
//! 的记法已经借道应用臂进了 [`super::kernel_app::try_app`]；`missing == 0` 的老路不猜任何东西）。
//! 解不出 ⇒ 回落老路，回落纪律与应用路径共用 [`super::kernel_app::finish`]。

use super::elab::{ElabCtx, ElabScope, HoverNode, KnownName, KnownTable, UnivMap};
use super::kernel_app::{count, env_on, finish, plain, run, Count};
use super::kernel_app_miss::MissGuard;
use super::kernel_app_postpone as postpone;
use crate::Expr;
use sokonanoda::builder::EnvBuilder;
use sokonanoda::util::ExprPtr;

#[cfg(test)]
thread_local! {
    /// 测试用：`true` ⇒ 不走本路径（反向验证：P2-4 之前的行为）。
    pub(crate) static TEST_NO_NOTATION: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

fn routing_enabled() -> bool {
    #[cfg(test)]
    if TEST_NO_NOTATION.with(|c| c.get()) {
        return false;
    }
    true
}

/// 记法要补的前导参数个数：签名层数 − 操作数个数，且这些层**都有名字**（同老路的不变量：结果类型里的
/// 箭头没有名字 ⇒ 不算前导参数）。从大往下取第一个满足的（同老路的候选顺序）。
fn missing_prefix(signature: &str, operands: usize) -> Option<usize> {
    let (layers, _) = super::elab::notation_telescope(signature)?;
    let max = layers.len().checked_sub(operands)?;
    (1..=max)
        .rev()
        .find(|&m| layers[..m].iter().all(|(n, _)| !n.is_empty()))
}

/// 见模块文档。`(None, _)` ⇒ 调用方照走老路；回落过（已计数）⇒ 第二项是归因用的 [`MissGuard`]。
#[allow(clippy::too_many_arguments)]
pub(crate) fn try_notation<'a>(
    builder: &mut EnvBuilder<'a>,
    canonical: &str,
    operands: &[&Expr],
    span: crate::Span,
    scope: &mut ElabScope<'a>,
    univ: &UnivMap<'a>,
    known: &KnownTable,
    hovers: &mut Vec<HoverNode<'a>>,
    expected_src: Option<&Expr>,
    ctx: &ElabCtx<'a, '_>,
) -> (Option<ExprPtr<'a>>, Option<MissGuard>) {
    if !ctx.options.dialect.strict()
        || !env_on()
        || !routing_enabled()
        || operands.is_empty()
        || super::elab::prelude_install_active()
    {
        return (None, None);
    }
    let Some(KnownName::Decl {
        implicit_prefix: 0,
        signature: Some(sig),
        ..
    }) = known.get(canonical)
    else {
        return (None, None);
    };
    let Some(missing) = missing_prefix(sig, operands.len()) else {
        return (None, None);
    };
    let postponable =
        |a: &Expr| postpone::enabled() && postpone::untyped_lambda_layers(a, plain).is_some();
    if !operands.iter().all(|a| plain(a) || postponable(a)) {
        return (None, Some(MissGuard::push(count(Count::Shape))));
    }
    // 头不经 `elab_expr`：记法头在源里不可见（符号的 hover 由调用方记），且要**不带层**的 `Const`
    // ⇒ 宇宙多态的目标（`Ne.{u}`）由内核按 K1d 给层元变量、随合一解出（老路靠操作数文本猜层）。
    let name = builder.name_from_str(canonical);
    let levels = builder.alloc_levels_slice(&[]);
    let head_t = builder.mk_const(name, levels);
    let (mark, depth) = (hovers.len(), scope.len());
    let outcome = run(
        builder,
        span,
        head_t,
        operands,
        missing,
        scope,
        univ,
        known,
        hovers,
        expected_src,
        ctx,
    );
    match finish(builder, outcome, scope, depth, hovers, mark) {
        Ok(out) => (Some(out), None),
        Err(Some(why)) => (None, Some(MissGuard::push(why))),
        Err(None) => (None, None),
    }
}

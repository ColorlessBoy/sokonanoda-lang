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
//! 闸门：**严格方言**（P2-12 S6 翻默认：strict 档恒走内核路径，环境变量已删）；只收前导隐式 0、`missing > 0` 的目标（`implicit_prefix > 0`
//! 的记法已经借道应用臂进了 [`super::kernel_app::try_app`]；`missing == 0` 的老路不猜任何东西）。
//! 解不出 ⇒ 回落老路，回落纪律与应用路径共用 [`super::kernel_app::finish`]。
//!
//! **P2-12 S4b**（`reports/2026-11-01-p212-s4b-spec.md`）：`A ≼ ∅` 这一族里，记法补的前导形参与
//! `∅` 自己的元素类型**互相等待**（`?β ≟ ?δ`，两个都没人定）⇒ 内核只能报 `Unsolved{Implicit{<missing}}`。
//! 老路接住它靠的是**同形兄弟借用**（`implicit::fill_pending_by_shape`）——一条 Lean 没有的启发式。
//! 本刀**不新增猜法**、**不动内核**：只把那条既有启发式在**调内核之前**用一次（[`super::elab::notation_prefix_args`]
//! 的 E19 待定档），解出前导值 ⇒ **前插成显式位**、`implicit_prefix = 0` ⇒ **重试一次** ⇒ 内核 `Solved`。
//! 触发条件三条全满足：① 内核回 `Unsolved`；② sites **全**是 `Implicit{index < missing}`（缺的正是记法补的位）；
//! ③ 反向验证开关开着。前导值里任何一位以 `None` 期望 elaborate 不了 ⇒ **放弃重试、原样回落**（宁可回落，不交病项）。
//! `Solved` 的那 1.6 万次一条都不多跑（零热路径成本）。

use super::elab::{
    elab_expr, ElabCtx, ElabScope, HoverNode, InplaceEnv, KnownName, KnownTable, UnivMap,
};
use super::error::CompileError;
use super::kernel_app::{count, effective, finish, plain, run, Count, Fell};
use super::kernel_app_miss::MissGuard;
use super::kernel_app_postpone as postpone;
use super::kernel_app_probe as probe;
use crate::Expr;
use sokonanoda::builder::EnvBuilder;
use sokonanoda::elab_app::{AppOutcome, UnsolvedSite};
use sokonanoda::util::ExprPtr;

#[cfg(test)]
thread_local! {
    /// 测试用：`true` ⇒ 不走本路径（反向验证：P2-4 之前的行为）。
    pub(crate) static TEST_NO_NOTATION: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// 测试用：`true` ⇒ **不重试**（反向验证：P2-12 S4b 之前的行为）。
    pub(crate) static TEST_NO_RETRY: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// 测试用：本线程的 `(重试次数, 重试后 Solved 次数)`（全局计数会被并行测试串扰）。
    pub(crate) static TEST_RETRY: std::cell::Cell<(u64, u64)> = const { std::cell::Cell::new((0, 0)) };
}

fn routing_enabled() -> bool {
    #[cfg(test)]
    {
        !TEST_NO_NOTATION.with(|c| c.get())
    }
    #[cfg(not(test))]
    {
        true
    }
}

/// P2-12 S4b 的重试开关（生产恒开；只在测试里被关掉做反向验证）。
fn retry_enabled() -> bool {
    #[cfg(test)]
    {
        !TEST_NO_RETRY.with(|c| c.get())
    }
    #[cfg(not(test))]
    {
        true
    }
}

#[cfg(test)]
thread_local! {
    /// **P2-12 S4c** 的反向验证开关：`true` ⇒ 按 S4c **之前**的规则数前导位（只看**前导层**有没有名字），
    /// 用来证明新增的"操作数层也要有名字"那一条**真的在咬**（关掉 ⇒ 记法 `∘` 的夹具回落 `argmismatch`）。
    pub(crate) static TEST_NAIVE_MISSING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// 记法要补的前导参数个数：签名层数 − 操作数个数，且**前导层与操作数层都得有名字**。
///
/// 为什么**操作数层**也要判（P2-12 S4c，`reports/2026-11-01-p212-s4c-readings.md`）：`notation_telescope`
/// 用 `peel_pi` 数层，它把**结果类型里的箭头**也数成一层、名字是空串（`elab::notation_prefix_args`
/// 的注释里已有同一条不变量的前一半）。原先这里只看前 `m` 层 ⇒ 结果带箭头的签名（`Function.comp`
/// `(α) (β) (γ) (g : β → γ) (f : α → β) : α → γ` 数出 **6** 层）在 2 个操作数时取 `m = 4`：
/// 前 4 层（α β γ g）都有名字 ⇒ 过闸，可**操作数会落到第 4 层 `f` 与第 5 层那个无名箭头上**
/// ⇒ 错位一读，内核把 `g` 对着 `f : α → β` elaborate ⇒ `ArgMismatch{arg:0}` ⇒ 老路接住
/// （`fallback_old_ok_notation_argmismatch`）。老路 `notation_prefix_args` 是**从大往小试到解得出**
/// 才算数，同一层条件下它会落到 `m = 3`（α β γ）✓ ⇒ 这里把"操作数位必须有名字"显式写进候选判据，
/// **在调内核之前**就取对切分，不需要新的猜法、也不碰内核（老路那份求解器的**结构前提**）。
fn missing_prefix(signature: &str, operands: usize) -> Option<usize> {
    let (layers, _) = super::elab::notation_telescope(signature)?;
    let max = layers.len().checked_sub(operands)?;
    #[cfg(test)]
    let naive = TEST_NAIVE_MISSING.with(|c| c.get());
    #[cfg(not(test))]
    let naive = false;
    (1..=max).rev().find(|&m| {
        // S4c 之前：只看前缀层（`naive`）。S4c 起：前缀层 + 操作数层都要有名字。
        let upto = if naive { m } else { m + operands };
        layers[..upto].iter().all(|(n, _)| !n.is_empty())
    })
}

/// 见模块文档。`(None, _)` ⇒ 调用方照走老路；回落过（已计数）⇒ 第二项是归因用的 [`MissGuard`]；
/// `Err` ⇒ P2-5 探测中的便宜失败（[`super::kernel_app_probe::fallback`]）。
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
) -> Result<(Option<ExprPtr<'a>>, Option<MissGuard>), CompileError> {
    if !effective(ctx.options.dialect)
        || !routing_enabled()
        || operands.is_empty()
        || super::elab::prelude_install_active()
    {
        return Ok((None, None));
    }
    let Some(KnownName::Decl {
        implicit_prefix: 0,
        signature: Some(sig),
        ..
    }) = known.get(canonical)
    else {
        return Ok((None, None));
    };
    let Some(missing) = missing_prefix(sig, operands.len()) else {
        return Ok((None, None));
    };
    let postponable =
        |a: &Expr| postpone::enabled() && postpone::untyped_lambda_layers(a, plain).is_some();
    if !operands.iter().all(|a| plain(a) || postponable(a)) {
        // P2-12 S0b：记法路由的回落也要结算老路结局（`shape` 同样压一条）。
        let g = MissGuard::push(count(Count::Shape));
        super::kernel_app_census::pending_notation("shape", canonical, span);
        return Ok((None, Some(g)));
    }
    // 头不经 `elab_expr`：记法头在源里不可见（符号的 hover 由调用方记），且要**不带层**的 `Const`
    // ⇒ 宇宙多态的目标（`Ne.{u}`）由内核按 K1d 给层元变量、随合一解出（老路靠操作数文本猜层）。
    let name = builder.name_from_str(canonical);
    let levels = builder.alloc_levels_slice(&[]);
    let head_t = builder.mk_const(name, levels);
    let (mark, depth) = (hovers.len(), scope.len());
    // 提示只取一次（取走即消费）⇒ P2-5 的探测重做与 S4b 的重试都拿到**同一份**。
    let hint = postpone::take_hint(span, scope);
    let mut outcome = run(
        builder,
        hint.clone(),
        head_t,
        operands,
        missing,
        scope,
        univ,
        known,
        hovers,
        expected_src,
        None,
        ctx,
    );
    // **P2-12 S4b**：缺的正是记法补的位 ⇒ 在**调内核之前**把前导值物化、前插成显式位、`implicit_prefix = 0`
    // 再解一次（`implicit_prefix = 0` ⇒ 内核把它们当普通显式位逐位 elaborate，正是 `min6` 已跑通的路径）。
    // ⚠ 成本：见 [`retry_with_prefix`] 的文档（探测切掉的站点上这份前缀**没人复用** ⇒ `strict ≤ loose`
    // 在这些文件上会被抬高；读数与三条试过的收窄方案见 `reports/2026-11-01-p212-s4b-readings.md` §13）。
    // **P2-12 S4c**：重试是不是被"前导值含占位符"那条挡掉的。只有它是真的时候，出口才把这次回落
    // 记成**写明的例外** `prefix_meta`（见 `kernel_app_fallback::notation_fell`）；census 的键不变。
    let mut prefix_meta = false;
    if let Ok(AppOutcome::Unsolved { sites }) = &outcome {
        // `Unsolved` 不可能是空 sites（内核只在 `!unsolved.is_empty()` 时产出它）。
        //
        if retry_enabled()
            && !probe::would_cut(builder, span, depth)
            && sites
                .iter()
                .all(|s| matches!(s, UnsolvedSite::Implicit { index } if *index < missing))
        {
            let (retried, meta) = retry_with_prefix(
                builder,
                sig,
                operands,
                missing,
                head_t,
                hint,
                scope,
                univ,
                known,
                hovers,
                expected_src,
                ctx,
            );
            prefix_meta = meta;
            if let Some(out) = retried {
                outcome = Ok(out);
            }
        }
    }
    // P2-12 S3：开关开着时，出口可能直接报错（`Fell::Dead`）⇒ 交出去，老路一行不跑。
    let judge = |why: &'static str, out: Option<&AppOutcome>| {
        super::kernel_app_fallback::notation_fell(
            why,
            out,
            canonical,
            operands,
            span,
            missing,
            prefix_meta,
            known,
            ctx,
        )
    };
    Ok(
        match finish(builder, outcome, scope, depth, hovers, mark, &judge) {
            Ok(out) => (Some(out), None),
            // P2-12 S0b：压一条（被探测切掉时 `?` 早退 ⇒ 老路不跑、不结算）。
            Err(Fell::Fallback(why)) => {
                let g = probe::fallback(builder, why, span, scope)?;
                super::kernel_app_census::pending_notation(why, canonical, span);
                (None, Some(g))
            }
            Err(Fell::Dead(e)) => return Err(e),
            Err(Fell::Untried) => (None, None),
        },
    )
}

/// **P2-12 S4b 的重试**：把记法前导参数在**调内核之前**物化成源级值（走老路**同一条**既有机制
/// [`super::elab::notation_prefix_args`]：①操作数类型 / ②期望类型 + E19 待定档 + 同形兄弟借用），
/// 加密到操作数前面、`implicit_prefix = 0` 再跑一次内核。`None` ⇒ 放弃重试、原样回落。
///
/// 三条**放弃**条件（宁可回落，不交病项）：
/// 1. `notation_prefix_args` 回 `Err` ⇒ 放弃（老路自己会撞上同一个错，交回原结局 = 与今天逐字节相同）；
/// 2. 解出的个数 ≠ `missing` ⇒ 放弃 —— 它内部会**从大往小**试候选，若只有更小的候选能解出，前插的个数就与
///    内核收到的隐式位数对不上 ⇒ 会把操作数当成形参（错位读法）。`missing` 是权威值，只认它；
/// 3. 任何一位以 `None` 期望 elaborate 不了 ⇒ 放弃（借来的值可能是签名望远镜里的名字，`min7` 的邻域）。
///
/// 只重试**一次**，不循环。提示（`hint`）原样传回去 ⇒ 重试不丢外层的期望。
#[allow(clippy::too_many_arguments)]
fn retry_with_prefix<'a>(
    builder: &mut EnvBuilder<'a>,
    sig: &str,
    operands: &[&Expr],
    missing: usize,
    head_t: ExprPtr<'a>,
    hint: Option<sokonanoda::transfer::OwnedExpr>,
    scope: &mut ElabScope<'a>,
    univ: &UnivMap<'a>,
    known: &KnownTable,
    hovers: &mut Vec<HoverNode<'a>>,
    expected_src: Option<&Expr>,
    ctx: &ElabCtx<'a, '_>,
) -> (Option<AppOutcome>, bool) {
    let prefix = match super::elab::notation_prefix_args(
        sig,
        operands,
        expected_src,
        ctx,
        scope,
        Some(&mut InplaceEnv {
            builder: &mut *builder,
            known,
        }),
    ) {
        Ok(Some(prefix)) if prefix.len() == missing => {
            // **本刀的成本不变量**（`REQUIREMENTS §9.39` strict ≤ loose）：这一份前缀**交回去给老路复用**
            // （[`prefix_args_cached`]）——同一个站点只算一次，与今天完全一样；否则失败的重试会让
            // 老路再算一遍（实测 `unit72-solution` 的 `JUDGE_INFER.calls` 90 → 165）。
            stash_prefix(sig, operands, scope.len(), &prefix);
            prefix
        }
        _ => return (None, false),
    };
    // 前导值解出来还不够：内核项里若已带占位符，这一趟**注定白跑**（见 [`has_meta`]）⇒ 不重试、直接回落。
    let Some(prefix_terms) = elab_all(builder, &prefix, scope, univ, known, ctx) else {
        return (None, false);
    };
    if prefix_terms.iter().any(|t| has_meta(*t)) {
        // **P2-12 S4c**：第二位 = "挡掉它的是**占位符判据**"（不是别的早退）⇒ 出口按**写明的例外**
        // `prefix_meta` 回落并计数（`kernel_app_fallback::notation_fell`）。census 键不变（仍是 `unsolved`）。
        return (None, true);
    }
    #[cfg(test)]
    TEST_RETRY.with(|c| c.set((c.get().0 + 1, c.get().1)));
    let mut args: Vec<&Expr> = Vec::with_capacity(prefix.len() + operands.len());
    args.extend(prefix.iter());
    args.extend(operands.iter().copied());
    // 操作数位的**源级**期望类型：把已物化的前导值代进签名望远镜（老路用的是**同一份**代换
    // `elab::notation_operand_expected`，不是新猜法）。没有它，`∅` 这种零元记法在**只有内核期望**时
    // elaborate 不了（`elab_ready` 只能把内核项交下去 ⇒ `ArgElab`）——实测在 `min3`
    // （`Set.empty (α : Type)` 显式档）上就断在这里；语料里的 `Set.empty {α : Type}` 隐式档不依赖它。
    let want = super::elab::notation_operand_expected(sig, &prefix, operands.len());
    let srcs: Vec<Option<&Expr>> = (0..prefix.len())
        .map(|_| None)
        .chain(want.iter().map(|w| w.as_ref()))
        .collect();
    let out = run(
        builder,
        hint,
        head_t,
        &args,
        0,
        scope,
        univ,
        known,
        hovers,
        expected_src,
        Some(&srcs),
        ctx,
    );
    let out = match out {
        Ok(out) => out,
        Err(_) => return (None, false),
    };
    #[cfg(test)]
    if matches!(out, AppOutcome::Solved(_)) {
        TEST_RETRY.with(|c| c.set((c.get().0, c.get().1 + 1)));
    }
    (Some(out), false)
}

/// **老路的 `notation_prefix_args` 入口**（`elab.rs` 的记法路径改成调它，**字节数不变、行数不增**）：
/// 若 [`retry_with_prefix`] 刚为**同一个站点**算过一份前缀，就直接交回去（只算一次的保证）；
/// 否则原样转给 [`super::elab::notation_prefix_args`]。
///
/// 为什么需要它：重试与老路**都要**同一份前导值。让两边各算一遍 = 判定的活干两遍 ⇒ 失败的重试
/// （`unit72-solution` 的 `⋃₀ (∅)` 那 4 个站点）会把 `JUDGE_INFER.calls` 从 90 抬到 165，破坏
/// `strict ≤ loose`。这里用**一格**线程局部槽（键 = 签名 + 操作数个数 + 第一个操作数的位置 + 作用域深度）
/// 把重试算出的那份交过去 ⇒ 每个站点一次，与今天相同。
pub(crate) fn prefix_args_cached<'a>(
    signature: &str,
    operands: &[&Expr],
    expected_src: Option<&Expr>,
    ctx: &ElabCtx<'a, '_>,
    scope: &ElabScope<'a>,
    env: Option<&mut InplaceEnv<'_, 'a>>,
) -> Result<Option<Vec<Expr>>, CompileError> {
    if let Some(args) = take_prefix(signature, operands, scope.len()) {
        return Ok(Some(args));
    }
    super::elab::notation_prefix_args(signature, operands, expected_src, ctx, scope, env)
}

thread_local! {
    /// 「重试刚算出的那一份前导值」：一格就够（老路的消费调用紧随重试之后，同一栈帧内）。
    static PREFIX_STASH: std::cell::RefCell<Option<(u64, Vec<Expr>)>> =
        const { std::cell::RefCell::new(None) };
}

/// 站点键：签名哈希 + 操作数个数 + 第一个操作数的起点 + 作用域深度（只用便宜、稳定的量）。
fn site_key(signature: &str, operands: &[&Expr], depth: usize) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    signature.hash(&mut h);
    operands.len().hash(&mut h);
    operands
        .first()
        .map(|a| a.span().start.offset)
        .unwrap_or(0)
        .hash(&mut h);
    depth.hash(&mut h);
    h.finish()
}

fn stash_prefix(signature: &str, operands: &[&Expr], depth: usize, args: &[Expr]) {
    let key = site_key(signature, operands, depth);
    PREFIX_STASH.with(|s| *s.borrow_mut() = Some((key, args.to_vec())));
}

/// 取走**只取一次**（键不符 = 不是同一个站点 ⇒ 丢掉旧的、老实重算）。
fn take_prefix(signature: &str, operands: &[&Expr], depth: usize) -> Option<Vec<Expr>> {
    let key = site_key(signature, operands, depth);
    PREFIX_STASH.with(|s| match s.borrow_mut().take() {
        Some((k, args)) if k == key => Some(args),
        _ => None,
    })
}

/// 每个前导值都必须在**当前作用域**里 elaborate 得了（只试、不留痕：hover 记在丢弃的临时表里，
/// 作用域深度试完还原；真 hover 由重试那一趟按**普通显式实参**记）。交回**内核项**
/// （调用方要看里面有没有占位符，见 [`has_meta`]）。
fn elab_all<'a>(
    builder: &mut EnvBuilder<'a>,
    prefix: &[Expr],
    scope: &mut ElabScope<'a>,
    univ: &UnivMap<'a>,
    known: &KnownTable,
    ctx: &ElabCtx<'a, '_>,
) -> Option<Vec<ExprPtr<'a>>> {
    let mut scratch: Vec<HoverNode<'a>> = Vec::new();
    let depth = scope.len();
    let mut out = Vec::with_capacity(prefix.len());
    for p in prefix {
        match elab_expr(
            builder,
            p,
            scope,
            univ,
            known,
            &mut scratch,
            None,
            None,
            ctx,
        ) {
            Ok(t) => out.push(t),
            Err(_) => {
                scope.truncate(depth);
                return None;
            }
        }
    }
    scope.truncate(depth);
    Some(out)
}

/// **P2-12 S4b 修正**：内核项里有没有**占位符**（`Expr::Meta`）或**层元变量**（`Level::MVar`）。
///
/// 为什么要有这一条：内核在**任何**工作之前先扫一遍输入（`binder_tys` / 头 / 已 elaborate 的实参），
/// 见到未赋值元变量立刻回 `Unsupported("input contains a meta")`
/// （`crates/kernel/src/elab_app.rs:184-186`）⇒ 这种重试**注定解不出**，而它已经先付了一次
/// `notation_prefix_args` 的就地判定。实测（`unit72-solution`）：白跑的重试把 strict 的
/// `JUDGE_INFER.calls` 从 90（= loose）抬到 165 ⇒ 破坏 `REQUIREMENTS §9.39` 的 `strict ≤ loose`。
/// 所以**先看前导值的内核项**：只要有占位符就不重试，照今天的方式回落。
///
/// 前端只**读**内核的公开 AST（`sokonanoda::expr` / `sokonanoda::level`），内核一行没动。
fn has_meta(e: ExprPtr<'_>) -> bool {
    use sokonanoda::expr::Expr as K;
    match &*e {
        K::Meta { .. } => true,
        K::App { fun, arg, .. } => has_meta(*fun) || has_meta(*arg),
        K::Pi {
            binder_type, body, ..
        }
        | K::Lambda {
            binder_type, body, ..
        } => has_meta(*binder_type) || has_meta(*body),
        K::Proj { structure, .. } => has_meta(*structure),
        K::Let { data, .. } => {
            has_meta(data.binder_type) || has_meta(data.val) || has_meta(data.body)
        }
        K::Const { levels, .. } => levels.iter().any(|l| has_level_mvar(*l)),
        K::Sort { level, .. } => has_level_mvar(*level),
        _ => false,
    }
}

/// 层里有没有层元变量（`Level::MVar`；`Succ`/`Max`/`IMax` 递归看）。
fn has_level_mvar(l: sokonanoda::util::LevelPtr<'_>) -> bool {
    use sokonanoda::level::Level as L;
    match &*l {
        L::MVar(..) => true,
        L::Succ(inner, _) => has_level_mvar(*inner),
        L::Max(a, b, _) | L::IMax(a, b, _) => has_level_mvar(*a) || has_level_mvar(*b),
        _ => false,
    }
}

//! **P2-2 (c)**：零元隐式记法（`∅`）作内核推迟位时按域解（见 [`try_hinted_const`]）。
//! **P2-11c**：宇宙多态头的两件小工具（[`universe_heads_enabled`]、[`bare_levels`]、[`observe_untried`]）。
//! 拆自 [`super::kernel_app`]（docs-lint ⑨：源码文件 ≤ 500 行）。

use super::elab::{elab_expr, ElabCtx, ElabScope, HoverNode, KnownName, KnownTable, UnivMap};
use super::kernel_app::{count, effective, install_printer, Count};
use super::kernel_app_postpone as postpone;
use crate::Expr;
use sokonanoda::builder::EnvBuilder;
use sokonanoda::elab_app::{AppOutcome, AppRequest};
use sokonanoda::env::EnvLimit;
use sokonanoda::util::ExprPtr;
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(test)]
thread_local! {
    /// 测试用：`true` ⇒ 宇宙多态头照旧挡在门外（反向验证：P2-11c 之前的行为）。
    pub(crate) static TEST_NO_UNIVERSE_HEADS: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// 测试用：本线程 [`observe_untried`] 计到的次数（全局计数会被并行测试串扰）。
    pub(crate) static TEST_UNTRIED_UNIVERSE_APP: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    /// 测试用：`true` ⇒ `Name.{…}` 头照旧不试（反向验证：P2-11e 之前的行为）。
    pub(crate) static TEST_NO_UNIVERSE_APP: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// **P2-11c**：带宇宙参数的 `Ident` 头进内核路径（K1d：不带层的 `Const` ⇒ 内核每个 uparam 发一个会话层元变量）。
fn universe_heads_enabled() -> bool {
    #[cfg(test)]
    if TEST_NO_UNIVERSE_HEADS.with(|c| c.get()) {
        return false;
    }
    true
}

/// 仍挡在门外的宇宙多态头：**高阶 motive**（`Eq.subst {motive : α → Prop}`，结果 `motive b`）。内核没有 motive
/// 推断（Lean 靠 `elab_as_elim`），课程里试了只会 `Unsolved`/`ExpectedMismatch` 回落、再付一遍老路（P2-11c 实测
/// unit02 +3 tried / +2 unsolved / +1 expected）；老路另有 `eqsubst.rs` 专门合成 motive（§9.36）。
const HO_MOTIVE_HEADS: &[&str] = &["Eq.subst"];

/// **P2-11c**：头 `canonical`（宇宙参数 `universes`）该不该挡；挡 ⇒ 回落原因（进 `fallback_unsupported_*` 计数）。
/// `attempt` / [`try_hinted_const`] / `kernel_app_lambda::defer` 三处闸门只读这一个定义。
pub(super) fn universe_gate(canonical: &str, universes: &[String]) -> Option<&'static str> {
    if universes.is_empty() {
        None
    } else if !universe_heads_enabled() {
        Some("universe-polymorphic head")
    } else if HO_MOTIVE_HEADS.contains(&canonical) {
        Some("universe-poly ho motive")
    } else {
        None
    }
}

/// **P2-11c**：`elab_expr` 给裸常量头的每个宇宙位造了**前端**层元变量（R2b-2 (a)，Lean `mkFreshLevelMVars`）；
/// 内核输入不许带元变量（`Unsupported("input contains a meta")`）⇒ 层全是元变量时换成**不带层**的 `Const`，
/// 内核按 K1d 自己发层元变量、`Solved` 前 `instantiate_mvars` 代入并验无残留 ⇒ 交回的项层已具体。
/// 层是具体的（`SOKO_NO_CONST_LEVELS` 下全 0、或无宇宙参数）⇒ 原样（内核按 K1d 原样用齐层）。
/// 解出的项头层不是元变量 ⇒ 调用方随后的 `infer_const_universes_with` 对它是 no-op（只认头层 `MVar`）。
pub(super) fn bare_levels<'a>(builder: &mut EnvBuilder<'a>, head: ExprPtr<'a>) -> ExprPtr<'a> {
    match &*head {
        sokonanoda::expr::Expr::Const { name, levels, .. }
            if !levels.is_empty()
                && levels
                    .iter()
                    .all(|l| matches!(&**l, sokonanoda::level::Level::MVar(..))) =>
        {
            let none = builder.alloc_levels_slice(&[]);
            builder.mk_const(*name, none)
        }
        _ => head,
    }
}

/// **P2-11e**：`Name.{…}` 头（`=` 记法降成的 `Eq.{u}`、用户写的 `Eq.{1}`）进内核路径。
fn universe_app_enabled() -> bool {
    #[cfg(test)]
    if TEST_NO_UNIVERSE_APP.with(|c| c.get()) {
        return false;
    }
    true
}

/// **P2-11e**：`attempt` 的头 ⇒ `(名字, span)`。`Ident` 原样；`Name.{…}` 放行（方案 A：**显式层原样保留**——
/// `elab_expr` 的 `UniverseApp` 臂按层文本建 `Const`，层是具体的 `Param`/数字（`level_ptr` 只解文本，从不发
/// 层元变量）⇒ [`bare_levels`] 原样、内核 K1d「带齐层 ⇒ 原样用」）。层数对不上留给 [`universe_arity_ok`]。
/// 其余头 / 测试开关关 ⇒ `None`（`Name.{…}` 仍计 `untried_universe_app_head`）。
pub(super) fn head_name(head: &Expr) -> Option<(&str, crate::Span)> {
    match head {
        Expr::Ident { name, span } => Some((name, *span)),
        Expr::UniverseApp { name, span, .. } if universe_app_enabled() => Some((name, *span)),
        _ => {
            observe_untried(head);
            None
        }
    }
}

/// **P2-11e**：`Name.{…}` 的层数与常量的宇宙参数数相同（不同 ⇒ 老路报 `ElabUniverseArity`；这里不试、计
/// `untried_universe_app_head`，不让一条注定失败的尝试计进 `tried`）。`Ident` 头恒真。
pub(super) fn universe_arity_ok(head: &Expr, universes: &[String]) -> bool {
    match head {
        Expr::UniverseApp { levels, .. } if levels.len() != universes.len() => {
            observe_untried(head);
            false
        }
        _ => true,
    }
}

/// **P2-11e（收窄）**：实参形状不收（洞 / `by` / 命名实参…）时该不该计 `fallback_shape` 并回落。`Name.{…}` 头 ⇒
/// **不**（计 `untried_universe_app_head`，调用方 `Err(None)` = 同 P2-11e 之前：不进 `probe::fallback`、不计 tried）。
/// 实测放开时 13 个文件 `fallback_shape` 0→103：全是试了立刻回落、再走老路的 `=`（P2-10 同病：纯多出的工作）。
pub(super) fn shape_counts(head: &Expr) -> bool {
    if matches!(head, Expr::UniverseApp { .. }) {
        observe_untried(head);
        return false;
    }
    true
}

static UNTRIED_UNIVERSE_APP: AtomicU64 = AtomicU64::new(0);

/// **P2-11c (1)**：纯观测——头是 `Name.{…}` 且没进内核路径（P2-11e 起只剩层数不符 / 测试开关）⇒ 计一次，
/// **不改控制流**（调用方照旧 `Err(None)`：不进 `probe::fallback`、不计 `tried`）。
fn observe_untried(head: &Expr) {
    if matches!(head, Expr::UniverseApp { .. }) {
        install_printer();
        UNTRIED_UNIVERSE_APP.fetch_add(1, Ordering::Relaxed);
        #[cfg(test)]
        TEST_UNTRIED_UNIVERSE_APP.with(|c| c.set(c.get() + 1));
    }
}

pub(super) fn untried_universe_app() -> u64 {
    UNTRIED_UNIVERSE_APP.load(Ordering::Relaxed)
}

/// **P2-2 (c)**：零元隐式记法（`∅` ⇒ `Set.empty`）作外层的**推迟位**——源级期望缺席、只有内核给的域
/// （[`postpone::ExpectedHint`]）⇒ 头 `canonical` 零显式实参走内核 `elab_app`，期望 = 域（Lean
/// `elabAndAddNewArg` 本来就按域 elaborate）。没有正对本节点的提示 / 闸门关 / 解不出 ⇒ `None`（调用方照报
/// 老诊断，外层随之回落，同今天）。
#[allow(clippy::too_many_arguments)]
pub(crate) fn try_hinted_const<'a>(
    builder: &mut EnvBuilder<'a>,
    canonical: &str,
    span: crate::Span,
    scope: &mut ElabScope<'a>,
    univ: &UnivMap<'a>,
    known: &KnownTable,
    hovers: &mut Vec<HoverNode<'a>>,
    ctx: &ElabCtx<'a, '_>,
) -> Option<ExprPtr<'a>> {
    if !effective(ctx.options.dialect) || !postpone::has_hint(span, scope) {
        return None;
    }
    let Some(KnownName::Decl { universes, .. }) = known.get(canonical) else {
        return None;
    };
    if universe_gate(canonical, universes).is_some() {
        return None;
    }
    let hint = postpone::take_hint(span, scope)?;
    let (mark, depth) = (hovers.len(), scope.len());
    let head = Expr::Ident {
        name: canonical.to_string(),
        span,
    };
    let solved = elab_expr(builder, &head, scope, univ, known, hovers, None, None, ctx)
        .ok()
        .filter(|h| matches!(**h, sokonanoda::expr::Expr::Const { .. }))
        .and_then(|head_t| {
            let head_t = bare_levels(builder, head_t);
            let expected = builder.import_owned(&hint)?;
            let binder_tys = scope.kernel_tys().to_vec();
            let limit = EnvLimit::ByIndex(builder.declaration_count());
            let req = AppRequest {
                binder_tys: &binder_tys,
                head: head_t,
                implicit_prefix: 0,
                explicit_args: &[],
                expected: Some(expected),
                lambda_binders: &[],
            };
            match builder.with_env(|ef| ef.elab_app(limit, &req)) {
                AppOutcome::Solved(t) => builder.import_owned(&t),
                _ => None,
            }
        })
        .filter(|out| postpone::loose_bvars(*out, &mut Default::default()) as usize <= scope.len());
    scope.truncate(depth);
    match solved {
        Some(out) => {
            count(Count::Solved);
            Some(out)
        }
        None => {
            count(Count::ArgElab);
            hovers.truncate(mark);
            None
        }
    }
}

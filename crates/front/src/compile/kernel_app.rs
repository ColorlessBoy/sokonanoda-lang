//! **P1-F2b2**：严格档的应用 elaborate 接到内核 [`sokonanoda::elab_app`]（`reports/2026-10-29-strict-replan.md`
//! P1 F2 行；Lean `Elab/App.lean` `elabAppArgs`）。
//!
//! 闸门 = **严格方言** 且 **`SOKO_STRICT_KERNEL_APP=1`**（进程内读一次）。闸门关 ⇒ [`try_app`] 第一行就返回
//! `None`，后面一行都不跑 ⇒ 与今天逐字节相同。
//!
//! 位置：`try_implicit_application_with` 里 `@` 脊 / prelude 安装两条早退**之后**（`@` 脊不插隐式 ⇒ 内核
//! 路径无事可做；prelude 安装期间禁止插入是防递归），期望类型的 SPINE-N 展开**之后**（用展开后的期望）。
//!
//! 尝试条件（不满足 ⇒ `None`，计 `fallback_shape`，除非连"尝试"都谈不上）：
//! * 头是 `Ident`，`resolve_known` 解析到 `KnownName::Decl`，且 `implicit_prefix > 0`（与老路的免费闸门同口径：
//!   没有前导隐式的头老路也不插，P1 不改它们）；宇宙多态的头不进内核（内核会回 `Unsupported`，这里先判，
//!   免得白 elaborate 实参；计 `fallback_unsupported_universe_polymorphic_head`）；
//! * 实参全是位置实参，且不含洞 / `by` / 无注解 binder（这些在 elaborate 时有线程局部副作用，回落后老路
//!   会再做一遍 ⇒ 不碰）。
//!
//! 结果：`Solved` ⇒ 导入 builder 返回；其余一律回落老路（P1 不改诊断），按种类计数。回落时本路径记下的
//! hover 全部撤掉（老路自己会记）。
//!
//! `solved_out` 不填：调用方拿它喂 `infer_const_universes_with`，而那个函数只处理**恰好 1 个宇宙参数**的头；
//! 这里只放行无宇宙参数的头 ⇒ 它对本路径的结果恒为 no-op。头的 hover（带 `ResolvedTarget::Declaration`）
//! 由 `elab_expr(head)` 照常记；整段应用的 hover 由调用方记（与老路同一处）。

use super::elab::{
    elab_expr, resolve_known, ElabCtx, ElabScope, HoverNode, KnownName, KnownTable, UnivMap,
};
use super::error::CompileError;
use super::kernel_app_miss::MissGuard;
use super::kernel_app_postpone as postpone;
use super::kernel_app_probe as probe;
use crate::Expr;
use sokonanoda::builder::EnvBuilder;
use sokonanoda::elab_app::{AppOutcome, AppRequest};
use sokonanoda::env::EnvLimit;
use sokonanoda::util::ExprPtr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

#[cfg(test)]
thread_local! {
    /// 测试用：`Some(b)` ⇒ 不读进程环境，直接用 `b`（方言仍要是 strict）。
    pub(crate) static TEST_OVERRIDE: std::cell::Cell<Option<bool>> = const { std::cell::Cell::new(None) };
    /// 测试用：本线程 `Solved` 次数（全局计数会被并行测试串扰）。
    pub(crate) static TEST_SOLVED: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    /// 测试用：本线程回落次数。
    pub(crate) static TEST_FALLBACK: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

pub(super) fn env_on() -> bool {
    #[cfg(test)]
    if let Some(b) = TEST_OVERRIDE.with(|c| c.get()) {
        return b;
    }
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var("SOKO_STRICT_KERNEL_APP").is_ok_and(|v| v == "1"))
}

/// **生效档位**（P2-8）：本路径在 `dialect` 下**会不会跑**。三处闸门（[`try_app`]、`kernel_app_hint`、
/// `kernel_app_notation`）与**所有缓存键**都只读这一个定义 ⇒ 闸门换来源（设置项 / 默认开）时键跟着走。
/// 进键的形状见 [`key_tag`]：**关 ⇒ 空串 / 0 位** ⇒ 闸门关的键与 P2-8 之前逐字节相同。
pub(crate) fn effective(dialect: super::Dialect) -> bool {
    dialect.strict() && env_on()
}

/// 判定缓存键（`judge::options_key`）的后缀：生效 ⇒ `"/kapp"`，否则 `""`（旧键不变）。
pub(crate) fn key_tag(dialect: super::Dialect) -> &'static str {
    if effective(dialect) {
        "/kapp"
    } else {
        ""
    }
}

/// 磁盘键（`cache::keys::metavar_state` 那个字节）的位：生效 ⇒ `0b1_0000`，否则 `0`（旧键不变）。
/// 低四位已被元变量三档（0..=2）+ `SOKO_META_PATTERN`（`0b0100`）/ `SOKO_META_APPROX`（`0b1000`）占用。
pub(crate) fn key_bit(dialect: super::Dialect) -> u8 {
    u8::from(effective(dialect)) << 4
}

static TRIED: AtomicU64 = AtomicU64::new(0);
static SOLVED: AtomicU64 = AtomicU64::new(0);
static UNSOLVED: AtomicU64 = AtomicU64::new(0);
static ARG_MISMATCH: AtomicU64 = AtomicU64::new(0);
static EXPECTED: AtomicU64 = AtomicU64::new(0);
static ARG_ELAB: AtomicU64 = AtomicU64::new(0);
static SHAPE: AtomicU64 = AtomicU64::new(0);
static BAD_SCOPE: AtomicU64 = AtomicU64::new(0);
static UNSUPPORTED: Mutex<Vec<(&'static str, u64)>> = Mutex::new(Vec::new());

pub(super) enum Count {
    Solved,
    Unsolved,
    ArgMismatch,
    Expected,
    ArgElab,
    Shape,
    BadScope,
    Unsupported(&'static str),
}

pub(super) fn count(c: Count) -> &'static str {
    install_printer();
    TRIED.fetch_add(1, Ordering::Relaxed);
    #[cfg(test)]
    match c {
        Count::Solved => TEST_SOLVED.with(|x| x.set(x.get() + 1)),
        _ => TEST_FALLBACK.with(|x| x.set(x.get() + 1)),
    }
    let (slot, key) = match c {
        Count::Solved => (&SOLVED, "solved"),
        Count::Unsolved => (&UNSOLVED, "unsolved"),
        Count::ArgMismatch => (&ARG_MISMATCH, "argmismatch"),
        Count::Expected => (&EXPECTED, "expected"),
        Count::ArgElab => (&ARG_ELAB, "argelab"),
        Count::Shape => (&SHAPE, "shape"),
        Count::BadScope => (&BAD_SCOPE, "bad_scope"),
        Count::Unsupported(why) => {
            if let Ok(mut v) = UNSUPPORTED.lock() {
                match v.iter_mut().find(|(k, _)| *k == why) {
                    Some((_, n)) => *n += 1,
                    None => v.push((why, 1)),
                }
            }
            return why;
        }
    };
    slot.fetch_add(1, Ordering::Relaxed);
    key
}

/// `SOKO_JUDGE_STATS` 设了 ⇒ 进程退出时打一行 `KERNEL_APP …`（同 `judge::stats` 的 `atexit` 时机）。
fn install_printer() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        if std::env::var_os("SOKO_JUDGE_STATS").is_none() {
            return;
        }
        extern "C" fn report() {
            eprintln!("{}", report_line());
        }
        unsafe extern "C" {
            fn atexit(cb: extern "C" fn()) -> i32;
        }
        unsafe {
            atexit(report);
        }
    });
}

/// 计数行（`scripts/strict-score.py` 收任意 `TAG k=v` 行）。
pub(crate) fn report_line() -> String {
    let g = |a: &AtomicU64| a.load(Ordering::Relaxed);
    let mut line = format!(
        "KERNEL_APP tried={} solved={} fallback_unsolved={} fallback_argmismatch={} \
         fallback_expected={} fallback_argelab={} fallback_shape={} fallback_bad_scope={} probe_redo={}",
        g(&TRIED),
        g(&SOLVED),
        g(&UNSOLVED),
        g(&ARG_MISMATCH),
        g(&EXPECTED),
        g(&ARG_ELAB),
        g(&SHAPE),
        g(&BAD_SCOPE),
        probe::redos(),
    );
    if let Ok(v) = UNSUPPORTED.lock() {
        for (why, n) in v.iter() {
            let key: String = why
                .chars()
                .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
                .collect();
            line.push_str(&format!(" fallback_unsupported_{key}={n}"));
        }
    }
    line
}

/// 实参里没有带线程局部副作用的子项（洞 / `by` / 无注解 binder / 命名实参）。
pub(super) fn plain(e: &Expr) -> bool {
    match e {
        Expr::Hole { .. } | Expr::By { .. } | Expr::NamedArg { .. } => false,
        Expr::Sort { .. } | Expr::Ident { .. } | Expr::UniverseApp { .. } | Expr::Num { .. } => {
            true
        }
        Expr::App { fun, arg, .. } => plain(fun) && plain(arg),
        Expr::Lambda { binders, body, .. } | Expr::Forall { binders, body, .. } => {
            binders.iter().all(|b| b.ty.as_deref().is_some_and(plain)) && plain(body)
        }
        Expr::Arrow {
            domain, codomain, ..
        } => plain(domain) && plain(codomain),
        Expr::Plus { lhs, rhs, .. } => plain(lhs) && plain(rhs),
        Expr::Notation { lhs, rhs, .. } => {
            lhs.as_deref().is_none_or(plain) && rhs.as_deref().is_none_or(plain)
        }
        Expr::SetLiteral { elements, .. } | Expr::AnonCtor { elements, .. } => {
            elements.iter().all(plain)
        }
        Expr::Let { .. } | Expr::Match { .. } => false,
    }
}

/// 见模块文档。`(None, _)` ⇒ 调用方照走老路；回落（计过数的）⇒ 第二项是回落原因的 [`MissGuard`]，
/// 调用方持有到函数返回 ⇒ 老路里的 `judge_infer` 未命中按它归因（P2-1 (a)）。
/// `Err` ⇒ **P2-5** 探测中的便宜失败（外层第一趟推迟这一位，不走老路，见 [`super::kernel_app_probe`]）。
#[allow(clippy::too_many_arguments)]
pub(crate) fn try_app<'a>(
    builder: &mut EnvBuilder<'a>,
    expr: &Expr,
    scope: &mut ElabScope<'a>,
    univ: &UnivMap<'a>,
    known: &KnownTable,
    hovers: &mut Vec<HoverNode<'a>>,
    expected_src: Option<&Expr>,
    ctx: &ElabCtx<'a, '_>,
) -> Result<(Option<ExprPtr<'a>>, Option<MissGuard>), CompileError> {
    Ok(
        match attempt(builder, expr, scope, univ, known, hovers, expected_src, ctx) {
            Ok(out) => (Some(out), None),
            Err(Some(why)) => (
                None,
                Some(probe::fallback(builder, why, expr.span(), scope)?),
            ),
            Err(None) => (None, None),
        },
    )
}

/// `Err(None)` = 没尝试（不计数）；`Err(Some(原因))` = 尝试后回落（已计数）。
#[allow(clippy::too_many_arguments)]
fn attempt<'a>(
    builder: &mut EnvBuilder<'a>,
    expr: &Expr,
    scope: &mut ElabScope<'a>,
    univ: &UnivMap<'a>,
    known: &KnownTable,
    hovers: &mut Vec<HoverNode<'a>>,
    expected_src: Option<&Expr>,
    ctx: &ElabCtx<'a, '_>,
) -> Result<ExprPtr<'a>, Option<&'static str>> {
    if !effective(ctx.options.dialect) {
        return Err(None);
    }
    let (head, args) = crate::spine::spine_of(expr);
    if args.is_empty() {
        return Err(None);
    }
    let Expr::Ident { name, span } = head else {
        return Err(None);
    };
    let canonical =
        resolve_known(known, ctx.ns, name, *span, scope.local_names()).map_err(|_| None)?;
    // 局部名遮蔽了同名常量 ⇒ 不是常量头。
    if scope.find_last(name).is_some() {
        return Err(None);
    }
    let Some(KnownName::Decl {
        implicit_prefix,
        universes,
        ..
    }) = known.get(&canonical)
    else {
        return Err(None);
    };
    if *implicit_prefix == 0 {
        return Err(None);
    }
    if !universes.is_empty() {
        return Err(Some(count(Count::Unsupported("universe-polymorphic head"))));
    }
    // P2-1：顶层 λ 带无注解 binder 的实参也收（它只会被推迟、对着域 elaborate）。
    let postponable =
        |a: &Expr| postpone::enabled() && postpone::untyped_lambda_layers(a, plain).is_some();
    if !args.iter().all(|a| plain(a) || postponable(a)) {
        return Err(Some(count(Count::Shape)));
    }
    let mark = hovers.len();
    // ⚠ `elab_expr` 的 binder 臂在 `?` 早退时**不** `truncate` 作用域（老路里 Err 直接让声明失败，
    // 留下的 binder 无人看见）；这里吞掉 Err 回落 ⇒ 必须自己恢复深度，否则老路在多出的 binder 下
    // elaborate ⇒ de Bruijn 下标错位（实测 `rejected: loose bvar in infer`，Rel.inv_comp_apply）。
    let depth = scope.len();
    let outcome = elab_expr(builder, head, scope, univ, known, hovers, None, None, ctx)
        .map_err(|_| Count::ArgElab)
        .and_then(|head_t| {
            let (prefix, span) = (*implicit_prefix, expr.span());
            let (sc, ks, hv) = (&mut *scope, known, &mut *hovers);
            run(
                builder,
                span,
                head_t,
                &args,
                prefix,
                sc,
                univ,
                ks,
                hv,
                expected_src,
                ctx,
            )
        });
    finish(builder, outcome, scope, depth, hovers, mark)
}

/// 内核结果 ⇒ 导入的项，或回落（已计数、hover 撤回、作用域深度恢复）。记法路径（[`super::kernel_app_notation`]）
/// 共用这一份出口 ⇒ 回落纪律只有一处。
pub(super) fn finish<'a>(
    builder: &mut EnvBuilder<'a>,
    outcome: Result<AppOutcome, Count>,
    scope: &mut ElabScope<'a>,
    depth: usize,
    hovers: &mut Vec<HoverNode<'a>>,
    mark: usize,
) -> Result<ExprPtr<'a>, Option<&'static str>> {
    scope.truncate(depth);
    let why = match outcome {
        Ok(AppOutcome::Solved(t)) => match builder.import_owned(&t) {
            // 纵深防御：解出的项必须活在当前 scope 里（不该发生；发生就回落并计数）。
            Some(out)
                if postpone::loose_bvars(out, &mut Default::default()) as usize > scope.len() =>
            {
                count(Count::BadScope)
            }
            Some(out) => {
                count(Count::Solved);
                debug_assert_eq!(scope.len(), depth, "内核路径必须原样交还作用域深度");
                return Ok(out);
            }
            None => count(Count::Unsupported("import")),
        },
        Ok(AppOutcome::Unsolved { .. }) => count(Count::Unsolved),
        Ok(AppOutcome::ArgMismatch { .. }) => count(Count::ArgMismatch),
        Ok(AppOutcome::ExpectedMismatch) => count(Count::Expected),
        Ok(AppOutcome::Unsupported(why)) => count(Count::Unsupported(why)),
        // 第二趟带齐实参后不该再要；要了 = 推迟位按域也 elaborate 不了 ⇒ 当 argelab 回落。
        Ok(AppOutcome::NeedArgs { .. }) => count(Count::ArgElab),
        Err(c) => count(c),
    };
    hovers.truncate(mark);
    debug_assert_eq!(scope.len(), depth, "回落前作用域深度必须恢复");
    Err(Some(why))
}

/// `head_t` = 已 elaborate 的头（必须是 `Const`；不带层 ⇒ 内核按 K1d 给层元变量）。
/// **P2-5**：第一趟有探测切掉过老路、结果又不是 `Solved` ⇒ 关掉探测整段重做一次（最坏 = 今天的行为）。
#[allow(clippy::too_many_arguments)]
pub(super) fn run<'a>(
    builder: &mut EnvBuilder<'a>,
    expr_span: crate::Span,
    head_t: ExprPtr<'a>,
    args: &[&Expr],
    implicit_prefix: usize,
    scope: &mut ElabScope<'a>,
    univ: &UnivMap<'a>,
    known: &KnownTable,
    hovers: &mut Vec<HoverNode<'a>>,
    expected_src: Option<&Expr>,
    ctx: &ElabCtx<'a, '_>,
) -> Result<AppOutcome, Count> {
    // 提示只取一次（取走即消费）⇒ 重做时用同一份。
    let hint = postpone::take_hint(expr_span, scope);
    let (depth, mark) = (scope.len(), hovers.len());
    probe::with_redo(
        || {
            scope.truncate(depth);
            hovers.truncate(mark);
            let (h, a) = (hint.clone(), args);
            let r = (head_t, implicit_prefix, expected_src);
            run_once(
                builder, h, r.0, a, r.1, scope, univ, known, hovers, r.2, ctx,
            )
        },
        |out| matches!(out, Ok(AppOutcome::Solved(_))),
    )
}

#[allow(clippy::too_many_arguments)]
fn run_once<'a>(
    builder: &mut EnvBuilder<'a>,
    hint: Option<sokonanoda::transfer::OwnedExpr>,
    head_t: ExprPtr<'a>,
    args: &[&Expr],
    implicit_prefix: usize,
    scope: &mut ElabScope<'a>,
    univ: &UnivMap<'a>,
    known: &KnownTable,
    hovers: &mut Vec<HoverNode<'a>>,
    expected_src: Option<&Expr>,
    ctx: &ElabCtx<'a, '_>,
) -> Result<AppOutcome, Count> {
    if !matches!(*head_t, sokonanoda::expr::Expr::Const { .. }) {
        return Err(Count::Shape);
    }
    let depth = scope.len();
    // `hint` = 本应用自己的内核期望提示（它是外层的推迟位、外层已按域给出）⇒ 源级期望缺席/elaborate 不了时用它。
    // 第一趟：无期望地 elaborate；失败的 / 无注解 λ ⇒ 推迟（`None`），失败的残留（hover、λ 臂漏的
    // binder）一并撤掉。推迟关着 ⇒ 失败即回落（P2-1 之前的行为）。
    let mut explicit: Vec<Option<ExprPtr<'a>>> = Vec::with_capacity(args.len());
    for a in args {
        let bare = || postpone::bare_implicit_const(a, scope, known, ctx.ns).is_some();
        if postpone::enabled() && (postpone::untyped_lambda_layers(a, plain).is_some() || bare()) {
            explicit.push(None);
            continue;
        }
        let mark = hovers.len();
        // P2-5：这一位在探测里（它自己是解不出的内核应用 ⇒ 便宜失败、推迟，不走老路）。
        let probe = probe::ProbeGuard::enter(builder, a.span(), scope);
        let first = elab_expr(builder, a, scope, univ, known, hovers, None, None, ctx);
        drop(probe);
        match first {
            Ok(t) => explicit.push(Some(t)),
            Err(_) if postpone::enabled() => {
                hovers.truncate(mark);
                scope.truncate(depth);
                explicit.push(None);
            }
            Err(_) => return Err(Count::ArgElab),
        }
    }
    // 期望类型只作输入，不留 hover（老路不为它记）。
    let before_expected = hovers.len();
    let expected = expected_src
        .and_then(|e| elab_expr(builder, e, scope, univ, known, hovers, None, None, ctx).ok())
        .or_else(|| hint.as_ref().and_then(|h| builder.import_owned(h)));
    hovers.truncate(before_expected);
    // P2-3：推迟的 λ 位交出前导带注解 binder 的类型（内核拿它们对齐域，见 `AppRequest::lambda_binders`）。
    let lam_tys: Vec<(usize, Vec<ExprPtr<'a>>)> = (0..args.len())
        .filter(|&i| explicit[i].is_none() && matches!(args[i], Expr::Lambda { .. }))
        .map(|i| {
            let t = postpone::typed_binder_tys(builder, args[i], scope, univ, known, hovers, ctx);
            (i, t)
        })
        .filter(|(_, t)| !t.is_empty())
        .collect();
    let lambda_binders: Vec<(usize, &[ExprPtr<'a>])> =
        lam_tys.iter().map(|(i, t)| (*i, t.as_slice())).collect();
    let binder_tys = scope.kernel_tys().to_vec();
    let limit = EnvLimit::ByIndex(builder.declaration_count());
    let req = AppRequest {
        binder_tys: &binder_tys,
        head: head_t,
        implicit_prefix,
        explicit_args: &explicit,
        expected,
        lambda_binders: &lambda_binders,
    };
    let mut out = builder.with_env(|ef| ef.elab_app(limit, &req));
    // **P2-2**（Lean `synthesizeSyntheticMVars`：有进展就再来一轮）：每轮把**域已就绪**的推迟位对着域 elaborate
    // （Lean `elabAndAddNewArg`），再整体重解；`NeedArgs` 每轮至少交出一位 ⇒ 轮数 ≤ 显式实参数（上限是兜底）。
    // 跑满仍是 `NeedArgs` ⇒ 调用方当 argelab 回落（同 P2-1）。
    for _ in 0..postpone::max_rounds(args.len()) {
        let AppOutcome::NeedArgs { domains, .. } = out else {
            return Ok(out);
        };
        postpone::elab_ready(
            builder,
            args,
            domains,
            depth,
            &mut explicit,
            scope,
            univ,
            known,
            hovers,
            ctx,
        )?;
        let req = AppRequest {
            binder_tys: &binder_tys,
            head: head_t,
            implicit_prefix,
            explicit_args: &explicit,
            expected,
            lambda_binders: &lambda_binders,
        };
        out = builder.with_env(|ef| ef.elab_app(limit, &req));
    }
    Ok(out)
}

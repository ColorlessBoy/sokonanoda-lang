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
use super::kernel_app_miss::MissGuard;
use super::kernel_app_postpone as postpone;
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

fn env_on() -> bool {
    #[cfg(test)]
    if let Some(b) = TEST_OVERRIDE.with(|c| c.get()) {
        return b;
    }
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var("SOKO_STRICT_KERNEL_APP").is_ok_and(|v| v == "1"))
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

enum Count {
    Solved,
    Unsolved,
    ArgMismatch,
    Expected,
    ArgElab,
    Shape,
    BadScope,
    Unsupported(&'static str),
}

fn count(c: Count) -> &'static str {
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
         fallback_expected={} fallback_argelab={} fallback_shape={} fallback_bad_scope={}",
        g(&TRIED),
        g(&SOLVED),
        g(&UNSOLVED),
        g(&ARG_MISMATCH),
        g(&EXPECTED),
        g(&ARG_ELAB),
        g(&SHAPE),
        g(&BAD_SCOPE),
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
fn plain(e: &Expr) -> bool {
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
) -> (Option<ExprPtr<'a>>, Option<MissGuard>) {
    match attempt(builder, expr, scope, univ, known, hovers, expected_src, ctx) {
        Ok(out) => (Some(out), None),
        Err(Some(why)) => (None, Some(MissGuard::push(why))),
        Err(None) => (None, None),
    }
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
    if !ctx.options.dialect.strict() || !env_on() {
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
    let outcome = run(
        builder,
        expr.span(),
        head,
        &args,
        *implicit_prefix,
        scope,
        univ,
        known,
        hovers,
        expected_src,
        ctx,
    );
    scope.truncate(depth);
    let why = match outcome {
        Ok(AppOutcome::Solved(t)) => match builder.import_owned(&t) {
            // 纵深防御：解出的项必须活在当前 scope 里（不该发生；发生就回落并计数）。
            Some(out) if loose_bvars(out, &mut Default::default()) as usize > scope.len() => {
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

#[allow(clippy::too_many_arguments)]
fn run<'a>(
    builder: &mut EnvBuilder<'a>,
    expr_span: crate::Span,
    head: &Expr,
    args: &[&Expr],
    implicit_prefix: usize,
    scope: &mut ElabScope<'a>,
    univ: &UnivMap<'a>,
    known: &KnownTable,
    hovers: &mut Vec<HoverNode<'a>>,
    expected_src: Option<&Expr>,
    ctx: &ElabCtx<'a, '_>,
) -> Result<AppOutcome, Count> {
    let head_t = elab_expr(builder, head, scope, univ, known, hovers, None, None, ctx)
        .map_err(|_| Count::ArgElab)?;
    if !matches!(*head_t, sokonanoda::expr::Expr::Const { .. }) {
        return Err(Count::Shape);
    }
    let depth = scope.len();
    // 本应用自己的内核期望提示（它是外层的推迟位、外层已按域给出）⇒ 源级期望缺席/elaborate 不了时用它。
    let hint = postpone::take_hint(expr_span, scope);
    // 第一趟：无期望地 elaborate；失败的 / 无注解 λ ⇒ 推迟（`None`），失败的残留（hover、λ 臂漏的
    // binder）一并撤掉。推迟关着 ⇒ 失败即回落（P2-1 之前的行为）。
    let mut explicit: Vec<Option<ExprPtr<'a>>> = Vec::with_capacity(args.len());
    for a in args {
        if postpone::enabled() && postpone::untyped_lambda_layers(a, plain).is_some() {
            explicit.push(None);
            continue;
        }
        let mark = hovers.len();
        match elab_expr(builder, a, scope, univ, known, hovers, None, None, ctx) {
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
    let binder_tys = scope.kernel_tys().to_vec();
    let limit = EnvLimit::ByIndex(builder.declaration_count());
    let req = AppRequest {
        binder_tys: &binder_tys,
        head: head_t,
        implicit_prefix,
        explicit_args: &explicit,
        expected,
    };
    let first = builder.with_env(|ef| ef.elab_app(limit, &req));
    let AppOutcome::NeedArgs { domains, .. } = first else {
        return Ok(first);
    };
    // 第二趟（Lean `elabAndAddNewArg`）：推迟位对着内核给的域 elaborate，然后带齐实参再解一次。
    for (ai, dom) in domains {
        let a = args[ai];
        let dom_t = builder
            .import_owned(&dom)
            .ok_or(Count::Unsupported("import"))?;
        if let Some(layers) = postpone::untyped_lambda_layers(a, plain) {
            if !postpone::domain_fits(dom_t, layers) {
                return Err(Count::ArgElab);
            }
        }
        // 提示给"这个实参自己"（应用/记法形态）；λ ⇒ 给最内层体（λ 臂只把内核期望传给体，体里的应用
        // 读不到源级期望）。
        let (n, body) = postpone::lambda_body(a);
        let _hint = match postpone::peel_pis(dom_t, n) {
            Some(b) if n > 0 => postpone::ExpectedHint::set(
                body.span(),
                depth + n,
                sokonanoda::transfer::export_expr(b),
            ),
            _ => postpone::ExpectedHint::set(a.span(), depth, dom),
        };
        let t = elab_expr(
            builder,
            a,
            scope,
            univ,
            known,
            hovers,
            Some(dom_t),
            None,
            ctx,
        )
        .map_err(|_| Count::ArgElab)?;
        explicit[ai] = Some(t);
    }
    let req = AppRequest {
        binder_tys: &binder_tys,
        head: head_t,
        implicit_prefix,
        explicit_args: &explicit,
        expected,
    };
    Ok(builder.with_env(|ef| ef.elab_app(limit, &req)))
}

/// 项里最大的 loose `Var` 下标 + 1（= 它至少要在几个 binder 之下；前端拿不到内核的 `num_loose_bvars`）。
/// 按节点地址记忆 ⇒ DAG 共享子项只走一次。
fn loose_bvars(e: ExprPtr<'_>, memo: &mut std::collections::HashMap<usize, u32>) -> u32 {
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

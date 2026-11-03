//! **P1-F2b2**：严格档的应用 elaborate 接到内核 [`sokonanoda::elab_app`]（`reports/2026-10-29-strict-replan.md`
//! P1 F2 行；Lean `Elab/App.lean` `elabAppArgs`）。
//!
//! 闸门 = **严格方言**（**P2-12 S6 翻默认**：环境变量已删，strict 档恒走内核路径）。宽松档 ⇒ [`try_app`]
//! 第一行就返回 `None`，后面一行都不跑 ⇒ 与今天逐字节相同。
//!
//! **P2-12 S3/S6**：strict 档同时**不回落**（见 [`super::kernel_app_fallback`]）——非探测回落改由
//! `kernel_app_report` 出诊断，不再走老路；`report()` 的**例外表**仍回落（写明的例外）。出口（`finish`）
//! 在 `kernel_app_fallback`，这里 re-export 保持调用点稳定。
//!
//! 位置：`try_implicit_application_with` 里 `@` 脊 / prelude 安装两条早退**之后**（`@` 脊不插隐式 ⇒ 内核
//! 路径无事可做；prelude 安装期间禁止插入是防递归），期望类型的 SPINE-N 展开**之后**（用展开后的期望）。
//!
//! 尝试条件（不满足 ⇒ `None`，计 `fallback_shape`，除非连"尝试"都谈不上）：
//! * 头是 `Ident`，`resolve_known` 解析到 `KnownName::Decl`，且 `implicit_prefix > 0`（与老路的免费闸门同口径：
//!   没有前导隐式的头老路也不插，P1 不改它们）；**P2-11c** 起宇宙多态的头也进（头换成不带层的 `Const`，
//!   内核 K1d 发层元变量，见 `kernel_app_hint::bare_levels`）；**P2-11e** 起 `Name.{…}` 头也进（显式层原样，见 `head_name`）；
//! * 实参全是位置实参，且不含洞 / `by` / 无注解 binder（有线程局部副作用，回落后老路会再做一遍 ⇒ 不碰）。
//!
//! 结果：`Solved` ⇒ 导入返回；其余回落老路（P1 不改诊断）或按第二个开关报错，按种类计数，本路径记下的
//! hover 全部撤掉。
//! `solved_out` 不填：调用方拿它喂 `infer_const_universes_with`，它只改**头层是层元变量**的项；`Solved` 的层已被
//! 内核代入成具体层（无残留元变量）⇒ 恒为 no-op。头的 hover 由 `elab_expr(head)` 照常记；整段的由调用方记。

use super::elab::{
    elab_expr, resolve_known, ElabCtx, ElabScope, HoverNode, KnownName, KnownTable, UnivMap,
};
use super::error::CompileError;
use super::kernel_app_lambda as lambda;
use super::kernel_app_miss::MissGuard;
use super::kernel_app_postpone as postpone;
use super::kernel_app_probe as probe;
use crate::Expr;
use sokonanoda::builder::EnvBuilder;
use sokonanoda::elab_app::{AppOutcome, AppRequest};
use sokonanoda::env::EnvLimit;
use sokonanoda::util::ExprPtr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

#[cfg(test)]
thread_local! {
    /// 测试用：`Some(b)` ⇒ 不读进程环境，直接用 `b`（方言仍要是 strict）。
    pub(crate) static TEST_OVERRIDE: std::cell::Cell<Option<bool>> = const { std::cell::Cell::new(None) };
    /// 测试用：本线程 `Solved` 次数（全局计数会被并行测试串扰）。
    pub(crate) static TEST_SOLVED: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    /// 测试用：本线程回落次数。
    pub(crate) static TEST_FALLBACK: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// **生效档位**（P2-8 / **P2-12 S6 翻默认**）：本路径在 `dialect` 下**会不会跑** —— **strict 档恒开**
/// （不再有环境变量闸门，也不再读进程环境）。三处闸门（[`try_app`]、`kernel_app_hint`、
/// `kernel_app_notation`）与**所有缓存键**都只读这一个定义 ⇒ 闸门换来源（设置项 / 默认开）时键跟着走。
/// 进键的形状见 [`key_tag`]：**宽松档 ⇒ 空串 / 0 位** ⇒ 宽松档的键与 P2-8 之前逐字节相同。
///
/// `TEST_OVERRIDE`（`#[cfg(test)]`）仍能把它关掉：单测要在**同一进程**里对比"老路"与"内核路径"。
pub(crate) fn effective(dialect: super::Dialect) -> bool {
    let strict = dialect.strict();
    #[cfg(test)]
    if let Some(b) = TEST_OVERRIDE.with(|c| c.get()) {
        return strict && b;
    }
    strict
}

/// 判定缓存键（`judge::options_key`）的后缀（**P2-12 S6**）：**strict 档恒 `"/kapp/nofb"`**
/// （内核路径 + 不回落都是方言的函数）、宽松档恒 `""`。定义在
/// [`super::kernel_app_fallback::key_parts`]（测试钩子那一支也要进键，见那里的长注释）。
pub(crate) fn key_tag(dialect: super::Dialect) -> &'static str {
    super::kernel_app_fallback::key_parts(dialect).0
}

/// 磁盘键（`cache::keys::metavar_state` 那个字节）的位（**P2-12 S6**）：strict ⇒ `0b11_0000`
/// （内核路径 `0b1_0000` + 不回落 `0b10_0000`），宽松档 ⇒ `0`；低四位已被元变量三档（0..=2）+
/// `SOKO_META_PATTERN`（`0b0100`）/ `SOKO_META_APPROX`（`0b1000`）占用。宽松档的键与 P2-8 之前**逐字节相同**。
pub(crate) fn key_bit(dialect: super::Dialect) -> u8 {
    super::kernel_app_fallback::key_parts(dialect).1
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

/// 一次尝试的结局分类（计数键的来源；`Copy` ⇒ `finish` 能借结果再交出去，S3）。
#[derive(Clone, Copy)]
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
pub(super) fn install_printer() {
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
         fallback_expected={} fallback_argelab={} fallback_shape={} fallback_bad_scope={} probe_redo={} \
         untried_universe_app_head={}",
        g(&TRIED),
        g(&SOLVED),
        g(&UNSOLVED),
        g(&ARG_MISMATCH),
        g(&EXPECTED),
        g(&ARG_ELAB),
        g(&SHAPE),
        g(&BAD_SCOPE),
        probe::redos(),
        super::kernel_app_hint::untried_universe_app(),
    );
    super::kernel_app_census::report(&mut line);
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
        // **2026-11-03 治本**：`let` **不再**被形状闸拒掉。
        //
        // 起因（实测）：`And.intro (fun …) (fun …)` 这类应用的实参里带**前端合成的 `let`**
        // （`by` 块/展开的产物）⇒ `plain` 返回 false ⇒ 内核应用路径**从未尝试** ⇒ 回落 IA-1
        // ⇒ 它要"实参的类型" ⇒ 渲染文本问判卷器 ⇒ 丢隐式实参 ⇒ 必然失败且每次重编整份前缀
        // （`unit06` 2 次 / `G30` 2 次）。`let` 是内核能 elaborate 的正常形式，没必要整条拒掉。
        Expr::Let { val, body, .. } => plain(val) && plain(body),
        Expr::Match { .. } => false,
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
            Err(Fell::Fallback(why)) => {
                let g = probe::fallback(builder, why, expr.span(), scope)?;
                super::kernel_app_census::pending(why, expr);
                (None, Some(g))
            }
            // **P2-12 S3**：开关开 ⇒ 内核路径的结论就是最终诊断（不回落、老路一行不跑）。
            Err(Fell::Dead(e)) => return Err(e),
            Err(Fell::Untried) => (None, None),
        },
    )
}

/// `Fell::Untried` = 没尝试（不计数）；`Fell::Fallback(原因)` = 尝试后回落（已计数）。
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
) -> Result<ExprPtr<'a>, Fell> {
    if !effective(ctx.options.dialect) {
        return Err(Fell::Untried);
    }
    let (head, args) = crate::spine::spine_of(expr);
    if args.is_empty() {
        return Err(Fell::Untried);
    }
    let Some((name, span)) = super::kernel_app_hint::head_name(head) else {
        return Err(Fell::Untried);
    };
    let canonical =
        resolve_known(known, ctx.ns, name, span, scope.local_names()).map_err(|_| Fell::Untried)?;
    // 局部名遮蔽了同名常量 ⇒ 不是常量头。
    if scope.find_last(name).is_some() {
        return Err(Fell::Untried);
    }
    let Some(KnownName::Decl {
        implicit_prefix,
        universes,
        ..
    }) = known.get(&canonical)
    else {
        return Err(Fell::Untried);
    };
    if *implicit_prefix == 0 || !super::kernel_app_hint::universe_arity_ok(head, universes) {
        return Err(Fell::Untried);
    }
    if let Some(why) = super::kernel_app_hint::universe_gate(&canonical, universes) {
        return Err(Fell::Fallback(count(Count::Unsupported(why))));
    }
    // P2-1：顶层 λ 带无注解 binder 的实参也收（它只会被推迟、对着域 elaborate）。
    let postponable =
        |a: &Expr| postpone::enabled() && postpone::untyped_lambda_layers(a, plain).is_some();
    if !args.iter().all(|a| plain(a) || postponable(a)) {
        return Err(Fell::miss(
            super::kernel_app_hint::shape_counts(head).then(|| count(Count::Shape)),
        ));
    }
    let mark = hovers.len();
    // ⚠ `elab_expr` 的 binder 臂在 `?` 早退时**不** `truncate` 作用域（老路里 Err 直接让声明失败，
    // 留下的 binder 无人看见）；这里吞掉 Err 回落 ⇒ 必须自己恢复深度，否则老路在多出的 binder 下
    // elaborate ⇒ de Bruijn 下标错位（实测 `rejected: loose bvar in infer`，Rel.inv_comp_apply）。
    let depth = scope.len();
    let outcome = elab_expr(builder, head, scope, univ, known, hovers, None, None, ctx)
        .map_err(|_| Count::ArgElab)
        .and_then(|head_t| {
            let head_t = super::kernel_app_hint::bare_levels(builder, head_t);
            let (prefix, span) = (*implicit_prefix, expr.span());
            let (sc, ks, hv) = (&mut *scope, known, &mut *hovers);
            let hint = postpone::take_hint(span, sc);
            run(
                builder,
                hint,
                head_t,
                &args,
                prefix,
                sc,
                univ,
                ks,
                hv,
                expected_src,
                None,
                ctx,
            )
        });
    finish(builder, outcome, scope, depth, hovers, mark, &|why, out| {
        super::kernel_app_fallback::app_fell(why, out, expr, head, &args, &canonical, known, ctx)
    })
}

/// **P2-12 S3**：出口与"要不要报错"的判定搬进 [`super::kernel_app_fallback`]（`kernel_app.rs` 逼近软线
/// 500 行；`AGENTS.md` 硬规则 5 允许用 re-export 保持调用点稳定）。记法路径仍从这里拿 [`finish`]。
pub(super) use super::kernel_app_fallback::{finish, Fell};

/// `head_t` = 已 elaborate 的头（必须是 `Const`；不带层 ⇒ 内核按 K1d 给层元变量）。
/// **P2-5**：第一趟有探测切掉过老路、结果又不是 `Solved` ⇒ 关掉探测整段重做一次（最坏 = 今天的行为）。
///
/// `hint` = 本应用的**内核期望提示**（`postpone::take_hint` 的产物：取走即消费）。由调用方在**同一个**
/// `(span, scope 深度)` 上取好再传进来（P2-12 S4b：记法路由要能用**同一份**提示重试一次，见
/// [`super::kernel_app_notation`]）⇒ 取提示的时机比 P2-5 之前晚不了：仍在 `elab_expr(head)` 之后、
/// 本应用自己的 `run_once` 之前。
///
/// `expected_srcs` = **逐位**的源级期望类型（`None` = 今天：第一趟一律无期望；记法重试才给，
/// 见 [`super::kernel_app_notation`] 的 `retry_with_prefix`）—— 长度不足的位按 `None` 处理。
#[allow(clippy::too_many_arguments)]
pub(super) fn run<'a>(
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
    expected_srcs: Option<&[Option<&Expr>]>,
    ctx: &ElabCtx<'a, '_>,
) -> Result<AppOutcome, Count> {
    let (depth, mark) = (scope.len(), hovers.len());
    probe::with_redo(
        || {
            scope.truncate(depth);
            hovers.truncate(mark);
            let (h, a) = (hint.clone(), args);
            let r = (head_t, implicit_prefix, expected_src);
            let es = expected_srcs;
            run_once(
                builder, h, r.0, a, r.1, scope, univ, known, hovers, r.2, es, ctx,
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
    expected_srcs: Option<&[Option<&Expr>]>,
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
    // P2-10b：推迟的带注解 λ 位（体对着域的 codomain elaborate；卡住 ⇒ `fill_late` 照老办法补）。
    let mut typed_lams: Vec<usize> = Vec::new();
    for (i, a) in args.iter().enumerate() {
        // **P2-12 S4b**：这位有没有**源级**期望类型（记法重试按已物化的前导值代换出望远镜的域）。
        let want = expected_srcs.and_then(|s| s.get(i).copied().flatten());
        let bare = || postpone::bare_implicit_const(a, scope, known, ctx.ns).is_some();
        // P2-10b：三类推迟互斥（λ 有无注解 binder / 裸常量）。
        let typed = postpone::enabled() && lambda::defer(a, scope, known, ctx.ns);
        typed_lams.extend(typed.then_some(explicit.len()));
        if typed
            || postpone::enabled()
                && (postpone::untyped_lambda_layers(a, plain).is_some() || bare())
        {
            explicit.push(None);
            continue;
        }
        let mark = hovers.len();
        // P2-5：这一位在探测里（它自己是解不出的内核应用 ⇒ 便宜失败、推迟，不走老路）。
        let probe = probe::ProbeGuard::enter(builder, a.span(), scope);
        let first = elab_expr(builder, a, scope, univ, known, hovers, None, want, ctx);
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
    // P2-10b：推迟的带注解 λ 到头仍空着（域一直带元变量）⇒ 照老办法补上、再解一次（多给一轮）。
    let mut late = false;
    let rounds = postpone::max_rounds(args.len()) + usize::from(!typed_lams.is_empty());
    for _ in 0..rounds {
        let domains = match out {
            AppOutcome::NeedArgs { domains, .. } => domains,
            AppOutcome::Solved(_) => return Ok(out),
            other => {
                let (b, e, s, h) = (&mut *builder, &mut explicit[..], &mut *scope, &mut *hovers);
                if late || !lambda::fill_late(b, args, &typed_lams, e, s, univ, known, h, ctx) {
                    return Ok(other);
                }
                late = true;
                Vec::new()
            }
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

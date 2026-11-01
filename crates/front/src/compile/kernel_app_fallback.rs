//! **P2-12 S3**：第二个过渡开关 `SOKO_STRICT_NO_FALLBACK=1` + 内核应用路径的**出口**。
//!
//! 权威实施单：`reports/2026-11-01-p212-s3-spec.md` §2；文案规则见 [`super::kernel_app_report`]（S2，已定案）。
//!
//! 开关只在 [`super::kernel_app::effective`] 已生效（strict + `SOKO_STRICT_KERNEL_APP=1`）时才有意义：
//! **开** ⇒ 内核路径解不出时**不再回落老路**，改把 [`super::kernel_app_report::report`] 的结论交出去
//! （人能读的错、真 span）；**关** ⇒ 与 P2-12 之前**逐字节相同**（本模块的渲染/解析一次都不跑）。
//!
//! 三件事，各占一段：
//! 1. [`no_fallback`] —— 开关的**唯一**读数（环境读一次 + 测试覆盖）；缓存键（`key_tag` / `key_bit`）也读它；
//! 2. [`Fell`] + [`finish`] —— `finish` 的出口类型（没尝试 / 回落老路 / 报错），从 `kernel_app` 搬来，经
//!    `kernel_app::finish` **re-export** 保持调用点不变（`kernel_app.rs` 逼近软线 500 行）；
//! 3. [`facts`] / [`facts_notation`] —— **懒**填 [`ReportInput`]：只在"已经要回落、且开关开着"时才渲染
//!    文本/解析签名。任何一项取不到（没有源级 `signature`、望远镜解不动）⇒ **不进 `report`、按例外回落**
//!    （保守：报错数据不全就别报）。
//!
//! **写明的例外**（[`Report::Fallback`] 的四种原因 + [`Report::Defer`]，开关开着也仍走老路）：
//! `unsupported` · `need_args` · `expected_retry`（`ExpectedMismatch` 交 S4 重解）· `argmismatch_text`
//! （正文要内核那台打印器）· `defer`（只含外部元变量 ⇒ 外层应用报）。它们进 `no_fallback_kept_<原因>` 计数。

use super::elab::{render_msg, ElabCtx, ElabScope, HoverNode, KnownTable};
use super::error::CompileError;
use super::kernel_app::{count, Count};
use super::kernel_app_postpone as postpone;
use super::kernel_app_report::{report, Report, ReportInput};
use super::Dialect;
use crate::{Expr, Span};
use sokonanoda::builder::EnvBuilder;
use sokonanoda::elab_app::AppOutcome;
use sokonanoda::util::ExprPtr;
#[cfg(test)]
use std::cell::Cell;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

#[cfg(test)]
thread_local! {
    /// 测试用：`Some(b)` ⇒ 不读进程环境（[`super::kernel_app::effective`] 仍要过）。
    pub(crate) static TEST_NO_FALLBACK: Cell<Option<bool>> = const { Cell::new(None) };
    /// **P2-12 S4c** 的反向验证开关：`true` ⇒ 取消 `prefix_meta` 那条**写明的例外**
    /// （`unit19-*` 那 6 个真实站点在开关开时当场变成诊断 ⇒ 正的那条判据判红）。
    pub(crate) static TEST_NO_PREFIX_META_KEPT: Cell<bool> = const { Cell::new(false) };
}

/// 见 [`TEST_NO_PREFIX_META_KEPT`]：生产恒 `false`（`#[cfg(test)]` 之外没有这个开关）。
fn kept_prefix_meta_off() -> bool {
    #[cfg(test)]
    {
        return TEST_NO_PREFIX_META_KEPT.with(|c| c.get());
    }
    #[cfg(not(test))]
    false
}

fn env_on() -> bool {
    #[cfg(test)]
    if let Some(b) = TEST_NO_FALLBACK.with(|c| c.get()) {
        return b;
    }
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var("SOKO_STRICT_NO_FALLBACK").is_ok_and(|v| v == "1"))
}

/// **生效档位**（P2-12 S3）：内核路径生效**且**第二个开关开。出口与**所有缓存键**都只读这一个定义
/// （同 P2-8 的纪律：闸门换来源时键跟着走）。
pub(crate) fn no_fallback(dialect: Dialect) -> bool {
    super::kernel_app::effective(dialect) && env_on()
}

/// `finish` 的出口：没尝试 / 回落老路（`why` 已计数）/ **开关开时报错**（S2 判出来的诊断）。
pub(super) enum Fell {
    /// 连尝试都没尝试（调用方照今天：不计数、不回落）。
    Untried,
    /// 尝试过、交回老路（已计数、hover 撤回、作用域深度恢复）。`why` 进探测切分与 census。
    Fallback(&'static str),
    /// **不回落**：把这条诊断交出去（`Report::Error` 的产物）。
    Dead(CompileError),
}

impl Fell {
    /// `Some(why)` ⇒ 回落；`None` ⇒ 没尝试（= 今天的 `Err(None)`）。
    pub(super) fn miss(why: Option<&'static str>) -> Self {
        match why {
            Some(why) => Fell::Fallback(why),
            None => Fell::Untried,
        }
    }
}

/// 内核结果 ⇒ 导入的项，或 [`Fell`]（回落已计数、hover 撤回、作用域深度恢复）。记法路径
/// （[`super::kernel_app_notation`]）共用这一份出口 ⇒ 回落纪律只有一处。
///
/// **P2-12 S3**：`judge` 是调用方给的"要不要报错"的判定（[`app_fell`] / [`notation_fell`]），只在
/// **已经要回落**的那条路上问一次 —— 开关关时它立刻回 `Fell::Fallback`，热路径一行都不多跑。
pub(super) fn finish<'a>(
    builder: &mut EnvBuilder<'a>,
    outcome: Result<AppOutcome, Count>,
    scope: &mut ElabScope<'a>,
    depth: usize,
    hovers: &mut Vec<HoverNode<'a>>,
    mark: usize,
    judge: &dyn Fn(&'static str, Option<&AppOutcome>) -> Fell,
) -> Result<ExprPtr<'a>, Fell> {
    scope.truncate(depth);
    let why = match &outcome {
        Ok(AppOutcome::Solved(t)) => match builder.import_owned(t) {
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
        Ok(AppOutcome::ExpectedMismatch { .. }) => count(Count::Expected),
        Ok(AppOutcome::Unsupported(why)) => count(Count::Unsupported(why)),
        // 第二趟带齐实参后不该再要；要了 = 推迟位按域也 elaborate 不了 ⇒ 当 argelab 回落。
        Ok(AppOutcome::NeedArgs { .. }) => count(Count::ArgElab),
        Err(c) => count(*c),
    };
    // **P2-12 S5**（设计稿 §Q4.2）：**报错出口不撤 hover** —— 这里的行都是第一趟已经解出来的
    // 具体项（没有元变量），留着只会比今天多；回落出口仍撤回（老路要重新 elaborate 一遍，
    // 留着会让同一段源码有两份行）。判据：`crates/lsp/tests/strict_hover.rs` 在 c1 的
    // `g` 与 `0` 上 hover（用户动作级）。
    let verdict = judge(why, outcome.as_ref().ok());
    if !matches!(verdict, Fell::Dead(_)) {
        hovers.truncate(mark);
    }
    debug_assert_eq!(scope.len(), depth, "回落前作用域深度必须恢复");
    Err(verdict)
}

/// `finish` 的出口判定：**开关关 ⇒ 老路**（`facts` 一次都不调用 ⇒ 零成本）；开 ⇒ 按 S2 的 [`report`] 出诊断。
/// 报错数据不全（`facts` 回 `None`）或 `report` 判"仍走老路"（写明的例外 / `Defer`）⇒ 照样回落。
pub(super) fn fell(
    dialect: Dialect,
    why: &'static str,
    outcome: Option<&AppOutcome>,
    facts: impl FnOnce() -> Option<Facts>,
) -> Fell {
    let Some(outcome) = outcome else {
        return Fell::Fallback(why);
    };
    if !no_fallback(dialect) {
        return Fell::Fallback(why);
    }
    let Some(facts) = facts() else {
        return Fell::Fallback(why);
    };
    match report(outcome, &facts.input()) {
        Report::Error(e) => {
            bump_dead(&facts.head, why, e.span);
            Fell::Dead(e)
        }
        Report::Fallback(reason) => {
            bump_kept(reason, &facts.head, why, facts.span);
            Fell::Fallback(why)
        }
        Report::Defer => {
            bump_kept("defer", &facts.head, why, facts.span);
            Fell::Fallback(why)
        }
    }
}

/// 应用路径的判定（`kernel_app::attempt` 交进来）。`ctx` 只用来读方言（开关只在 strict 生效）。
#[allow(clippy::too_many_arguments)]
pub(super) fn app_fell(
    why: &'static str,
    outcome: Option<&AppOutcome>,
    expr: &Expr,
    head: &Expr,
    args: &[&Expr],
    canonical: &str,
    known: &KnownTable,
    ctx: &ElabCtx<'_, '_>,
) -> Fell {
    fell(ctx.options.dialect, why, outcome, || {
        facts(expr, head, args, canonical, known, ctx)
    })
}

/// 记法路由的判定（`kernel_app_notation::try_notation` 交进来）。`missing` = 记法替用户补的前导层数
/// （= 这条路由的 `k`；`operands + missing <= 层数` ⇒ [`report`] 的"实参写多了"那条永不触发）。
///
/// **P2-12 S4c**（用户 2026-11-01 15:2x 拍板）：`prefix_meta = true` ⇒ 这次 `Unsolved` 是**老路自己也只给得出
/// 占位符**那一族（`\0soko_m*` ⇒ `Expr::Meta`；S4b 的重试被占位符判据挡住）。它是本实现与 Lean 的**有意偏离**：
/// Lean 到声明末尾会报 "don't know how to synthesize implicit argument"，而我们的老路**接受**带占位符的记法
/// 前导位（`fill_pending_by_shape` 借同形兄弟）⇒ strict **不能**比 loose 更严（否则 `unit19-solution`
/// 这类现在能过的课程文件会被拒）。所以它按**写明的例外**回落并计数 `nofallback_kept_prefix_meta`，
/// **不进诊断** —— 这正是 `Report::Fallback` 那张表的用途（与 `argmismatch_text` / `deferred` 同级）。
/// ⚠ census 的键**不变**（仍是 `fallback_old_ok_notation_unsolved`）：本刀只改"开关开时会不会报错"，
/// **不改**任何指标口径（不许把 `Unsolved` 改名成"不计"）。
#[allow(clippy::too_many_arguments)]
pub(super) fn notation_fell(
    why: &'static str,
    outcome: Option<&AppOutcome>,
    canonical: &str,
    operands: &[&Expr],
    span: Span,
    missing: usize,
    prefix_meta: bool,
    known: &KnownTable,
    ctx: &ElabCtx<'_, '_>,
) -> Fell {
    // **P2-12 S4c**：占位符那一族 ⇒ **写明的例外**（照旧回落），只在开关开着时才有分别
    // （开关关着时 census 与今天逐字节相同 ⇒ 这一支只是在同一结局上多记一个桶）。
    if prefix_meta && !kept_prefix_meta_off() && no_fallback(ctx.options.dialect) {
        bump_kept("prefix_meta", canonical, why, span);
        return Fell::Fallback(why);
    }
    fell(ctx.options.dialect, why, outcome, || {
        facts_notation(canonical, operands, span, missing, known)
    })
}

/// [`ReportInput`] 的原料（都已是**渲染好的文本** + 源级 span；S2 的纯函数只吃这个）。
pub(super) struct Facts {
    /// 头名的渲染（`render_msg(head)`；记法路由 = 规范名）。
    head_text: String,
    /// 整条应用的渲染（`render_msg(expr)`；记法路由 = 规范名）。
    whole_text: String,
    /// 声明签名的**源级文本**（源级形参名从这里取）。
    ty_text: String,
    /// 前导隐式层数（应用 = 注册表的 `implicit_prefix`；记法 = 补的前导层数）。
    k: usize,
    /// 签名总层数（含隐式与显式）。
    total_layers: usize,
    /// 显式层数（`total_layers - k`）。
    explicit_layers: usize,
    /// 位置实参个数。
    n_args: usize,
    /// 整条应用的 span。
    span: Span,
    /// 各实参的 span。
    arg_spans: Vec<Span>,
    /// 规范名（只给读数日志用；不参与文案）。
    head: String,
}

impl Facts {
    fn input(&self) -> ReportInput<'_> {
        ReportInput {
            head_text: &self.head_text,
            whole_text: &self.whole_text,
            ty_text: &self.ty_text,
            k: self.k,
            total_layers: self.total_layers,
            explicit_layers: self.explicit_layers,
            n_args: self.n_args,
            span: self.span,
            arg_spans: &self.arg_spans,
        }
    }
}

/// 应用路径的原料。`None` ⇒ **数据不全**（注册表没有源级签名 / 望远镜解不动）⇒ 不报、按例外回落。
fn facts(
    expr: &Expr,
    head: &Expr,
    args: &[&Expr],
    canonical: &str,
    known: &KnownTable,
    ctx: &ElabCtx<'_, '_>,
) -> Option<Facts> {
    let decl = known.get(canonical)?;
    let ty_text = decl.signature()?;
    let (layers, _) = super::implicit::telescope(ty_text)?;
    let k = decl.implicit_prefix();
    Some(Facts {
        head_text: render_msg(ctx, head),
        whole_text: render_msg(ctx, expr),
        ty_text: ty_text.to_string(),
        k,
        total_layers: layers.len(),
        explicit_layers: layers.len().saturating_sub(k),
        n_args: args.len(),
        span: expr.span(),
        arg_spans: args.iter().map(|a| a.span()).collect(),
        head: canonical.to_string(),
    })
}

/// 记法路由的原料：头在源里不可见 ⇒ `head_text = whole_text = 规范名`（实施单 §2.3；出路会退化成
/// `@<头> …`，正是老路的兜底行为）。`missing` = [`super::kernel_app_notation`] 补的前导层数。
fn facts_notation(
    canonical: &str,
    operands: &[&Expr],
    span: Span,
    missing: usize,
    known: &KnownTable,
) -> Option<Facts> {
    let decl = known.get(canonical)?;
    let ty_text = decl.signature()?;
    let (layers, _) = super::implicit::telescope(ty_text)?;
    Some(Facts {
        head_text: canonical.to_string(),
        whole_text: canonical.to_string(),
        ty_text: ty_text.to_string(),
        k: missing,
        total_layers: layers.len(),
        explicit_layers: layers.len().saturating_sub(missing),
        n_args: operands.len(),
        span,
        arg_spans: operands.iter().map(|a| a.span()).collect(),
        head: canonical.to_string(),
    })
}

static DEAD: AtomicU64 = AtomicU64::new(0);
static KEPT: Mutex<Vec<(&'static str, u64)>> = Mutex::new(Vec::new());

#[cfg(test)]
thread_local! {
    /// 测试用：本线程报错的次数（全局原子计数会被并行测试串扰）。
    pub(crate) static TEST_DEAD: Cell<u64> = const { Cell::new(0) };
    /// 测试用：本线程"开关开着仍回落"的原因序列。
    pub(crate) static TEST_KEPT: std::cell::RefCell<Vec<&'static str>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// 报错一次：计数 + `SOKO_KERNEL_APP_LOG` 下记位置（`dead <why> <head> <line>:<col>`，与
/// [`super::kernel_app_census`] 的 `<tag> <why> <head> <line>:<col>` 同格式 ⇒ 一份日志就能对账）。
fn bump_dead(head: &str, why: &'static str, span: Span) {
    #[cfg(test)]
    TEST_DEAD.with(|c| c.set(c.get() + 1));
    DEAD.fetch_add(1, Ordering::Relaxed);
    log_position("dead", why, head, span);
}

/// 开关开着仍回落一次：**写明的例外**（按 `report` 给出的原因分桶）+ 位置日志。
///
/// ⚠ 原因里有空格（`Unsupported("input contains a meta")`）⇒ 计数键与日志记号一律过 [`key`]：
/// `TAG k=v` 行与四列日志都**不许**出现空白（否则 `scripts/strict-score.py` 与日志解析会分叉）。
fn bump_kept(reason: &'static str, head: &str, why: &'static str, span: Span) {
    #[cfg(test)]
    TEST_KEPT.with(|v| v.borrow_mut().push(reason));
    let bucket = key(reason);
    if let Ok(mut v) = KEPT.lock() {
        match v.iter_mut().find(|(k, _)| *k == bucket) {
            Some((_, n)) => *n += 1,
            None => v.push((bucket, 1)),
        }
    }
    log_position("kept", why, head, span);
}

/// 原因 ⇒ 计数键 / 日志记号：非字母数字一律换成 `_`（同 [`super::kernel_app_census`] 的 `key()`）——
/// `TAG k=v` 行与四列日志都**不许**出现空白，`Unsupported("input contains a meta")` 这种带空格的原因
/// 不清化就会把 `nofallback_kept_input contains a meta=9` 拆成 `meta=9`（实测踩过）。
///
/// 原因集合**有限且已知**（`Report::Fallback` 的四种 + `Unsupported` 的 P1 原因）⇒ 查表返回常量，
/// 零分配；将来新加的原因落到 `"other"`，计数仍看得见、日志仍可解析。
fn key(why: &str) -> &'static str {
    match why {
        "expected_retry" => "expected_retry",
        "argmismatch_text" => "argmismatch_text",
        "need_args" => "need_args",
        "defer" => "defer",
        "unsolved_other" => "unsolved_other",
        // **P2-12 S4c**：占位符那一族（写明的例外，见 `notation_fell`）。
        "prefix_meta" => "prefix_meta",
        "input contains a meta" => "input_contains_a_meta",
        "universe arity" => "universe_arity",
        "scope too deep" => "scope_too_deep",
        "meta id space" => "meta_id_space",
        "residual metas" => "residual_metas",
        "recheck" => "recheck",
        "panic" => "panic",
        "import" => "import",
        "surplus" => "surplus",
        _ => "other",
    }
}

/// 日志第二列：与 [`super::kernel_app_census`] 的 `key()` **同一套词**（固定键原样、`Unsupported` 的原因
/// 加 `unsupported_` 前缀并清化空白）⇒ `dead`/`kept` 行能与 `ok|none|err` 行按同一套键对账。
fn why_key(why: &'static str) -> &'static str {
    match why {
        "unsolved" | "argmismatch" | "expected" | "argelab" | "shape" | "bad_scope" => why,
        "input contains a meta" => "unsupported_input_contains_a_meta",
        "universe arity" => "unsupported_universe_arity",
        "scope too deep" => "unsupported_scope_too_deep",
        "meta id space" => "unsupported_meta_id_space",
        "residual metas" => "unsupported_residual_metas",
        "recheck" => "unsupported_recheck",
        "panic" => "unsupported_panic",
        "import" => "unsupported_import",
        "surplus" => "unsupported_surplus",
        _ => "unsupported_other",
    }
}

fn log_position(tag: &str, why: &'static str, head: &str, span: Span) {
    let Some(path) = std::env::var_os("SOKO_KERNEL_APP_LOG") else {
        return;
    };
    use std::io::Write;
    let line = format!(
        "{tag} {} {head} {}:{}\n",
        why_key(why),
        span.start.line,
        span.start.column
    );
    if let Ok(mut fh) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = fh.write_all(line.as_bytes());
    }
}

/// 追加到 `KERNEL_APP …` 行的尾巴。**开关关 ⇒ 一个字节都不加**（统计行与今天逐字节相同）。
pub(super) fn report_counts(line: &mut String) {
    let dead = DEAD.load(Ordering::Relaxed);
    if dead > 0 {
        line.push_str(&format!(" nofallback_dead={dead}"));
    }
    if let Ok(v) = KEPT.lock() {
        for (reason, n) in v.iter() {
            line.push_str(&format!(" nofallback_kept_{reason}={n}"));
        }
    }
}

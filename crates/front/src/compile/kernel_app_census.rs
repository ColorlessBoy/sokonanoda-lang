//! **P2-12 S0/S0b**：内核应用路径**回落之后**老路的结局（纯观测，`reports/2026-10-31-p2-12-design.md` §Q6 S0）。
//!
//! 覆盖四个回落口（S0 只做了第一个，S0b 补齐后三个 ⇒ `fallback_unsolved` 与 `fallback_old_*_unsolved` +
//! `probe_cut` 对得上账）：
//! * **应用路径**：[`super::kernel_app::try_app`] 非探测回落（交回 `MissGuard`）⇒ [`pending`] 压一条原因；
//!   `try_implicit_application_with` 的唯一调用方（`elab_expr` 的 `App` 臂）用 [`settle`] 包住那次调用。
//! * **记法路由**：[`super::kernel_app_notation::try_notation`] 的回落（含 `shape`）⇒ [`pending_notation`]；
//!   老路（`elab_notation` 的其余部分）由 [`arm`] 的守卫按出口结算 —— 走到末尾 `Ok` ⇒ [`Settle::ok`]，
//!   任何早退 / `?` / 显式 `Err` ⇒ `Drop` 记 `err`。
//!
//! 另记两处**不是**应用级半径的回落（写明的例外，S0b 只是让它们可见）：
//! * 零元 [`super::kernel_app_hint::try_hinted_const`] 解不出（[`hinted_fallback`]，按来源分桶）；
//! * 推迟的 λ 位用老办法补上（[`filled_late`]，`kernel_app_lambda::fill_late`）。
//!
//! 结局口径：`Ok(Some)` ⇒ `fallback_old_ok_<route><why>`，`Ok(None)`（老路不插隐式、交回普通应用臂）
//! ⇒ `..._none_...`，`Err` ⇒ `..._err_...`。嵌套应用各自成对压/取 ⇒ 栈平衡。
//! `SOKO_KERNEL_APP_LOG=<path>` ⇒ 每次另追加 `<tag> <why> <head> <line>:<col>`。只计数，不改判定。

use crate::{Expr, Span};
use std::cell::RefCell;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

/// 一次回落的现场：路由（`""` = 应用路径 / `"notation_"` = 记法路由）、原因、头名、位置。
#[derive(Clone)]
struct Fallback {
    route: &'static str,
    why: &'static str,
    head: String,
    span: Span,
}

thread_local! {
    static STACK: RefCell<Vec<Fallback>> = const { RefCell::new(Vec::new()) };
}

#[cfg(test)]
thread_local! {
    /// 测试用：本线程结算过的 `(结局, 原因)`（全局计数会被并行测试串扰）。
    pub(crate) static TEST_SETTLED: RefCell<Vec<(&'static str, &'static str)>> = const { RefCell::new(Vec::new()) };
    /// 测试用：本线程按路由结算过的 `(结局, 路由 + 原因)`（S0b 的记法路由判据）。
    pub(crate) static TEST_SETTLED_ROUTED: RefCell<Vec<(&'static str, String)>> = const { RefCell::new(Vec::new()) };
    /// 测试用：`true` ⇒ 一律按 `err` 结算（反向验证）。
    pub(crate) static TEST_FORCE_ERR: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// 测试用：本线程零元 `try_hinted_const` 的回落次数（按来源）。
    pub(crate) static TEST_HINTED: RefCell<Vec<(&'static str, u64)>> = const { RefCell::new(Vec::new()) };
    /// 测试用：本线程 `fill_late` 补过位的次数。
    pub(crate) static TEST_FILLED_LATE: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

static OLD: Mutex<Vec<(&'static str, &'static str, &'static str, u64)>> = Mutex::new(Vec::new());
static HINTED: Mutex<Vec<(&'static str, u64)>> = Mutex::new(Vec::new());
static FILLED_LATE: AtomicU64 = AtomicU64::new(0);
static PROBE_CUT: AtomicU64 = AtomicU64::new(0);

/// 探测切掉的回落（不走老路 ⇒ 不结算，只计 `probe_cut`）。
pub(crate) fn probe_cut() {
    PROBE_CUT.fetch_add(1, Ordering::Relaxed);
}

/// `try_app` 非探测回落：记下原因（头名与 span 进日志）。
pub(crate) fn pending(why: &'static str, expr: &Expr) {
    let head = match crate::spine::spine_of(expr).0 {
        Expr::Ident { name, .. } | Expr::UniverseApp { name, .. } => name.clone(),
        _ => "<non-ident>".to_string(),
    };
    push(Fallback {
        route: "",
        why,
        head,
        span: expr.span(),
    });
}

/// 记法路由（`kernel_app_notation`）的回落：没有源级 `Expr`（记法头在源里不可见），只有规范名与 span。
/// 原因带 `notation_` 前缀 ⇒ 与应用路径同一个 `why` 也能分开对账（S0b）。
pub(crate) fn pending_notation(why: &'static str, canonical: &str, span: Span) {
    push(Fallback {
        route: "notation_",
        why,
        head: canonical.to_string(),
        span,
    });
}

fn push(f: Fallback) {
    STACK.with(|s| s.borrow_mut().push(f));
}

/// 包住 `try_implicit_application_with` 的一次调用：本次压过原因 ⇒ 按返回值结算。原样返回 `f()`。
pub(crate) fn settle<T, E>(f: impl FnOnce() -> Result<Option<T>, E>) -> Result<Option<T>, E> {
    let before = depth();
    let out = f();
    // 本次压的那条在下标 `before`；更深的残留（panic 展开没走到内层 `settle`）一并丢掉 ⇒ 栈恒平衡。
    let mine = take(before);
    if let Some(f) = mine {
        let tag = match &out {
            Ok(Some(_)) => "ok",
            Ok(None) => "none",
            Err(_) => "err",
        };
        record(f, tag);
    }
    out
}

/// 记法路由的结算守卫：`Drop` 时没有显式结局 ⇒ `err`（老路早退 / `?` / 显式 `Err`）。
///
/// `armed == false`（`try_notation` 没压过原因：没尝试 / 被探测切掉 / 内核直接解出）⇒ 什么都不记。
#[must_use]
pub(crate) struct Settle {
    /// 本帧压进去的那条原因在栈上的下标（`armed == false` ⇒ 不用）。
    at: usize,
    armed: bool,
    tag: &'static str,
}

impl Settle {
    /// 老路走到了末尾并交出自己的项 ⇒ 结算成 `ok`（消费守卫，`Drop` 兑现）。
    pub(crate) fn ok(mut self) {
        self.tag = "ok";
    }
}

impl Drop for Settle {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        if let Some(f) = take(self.at) {
            record(f, self.tag);
        }
    }
}

/// 见 [`Settle`]：`armed` = `try_notation` 交回的 `MissGuard` 是不是 `Some`（= 压过原因）。
pub(crate) fn arm(armed: bool) -> Settle {
    Settle {
        at: depth().saturating_sub(1),
        armed,
        tag: "err",
    }
}

/// 零元 [`super::kernel_app_hint::try_hinted_const`] 解不出：按来源记一次（写明的例外，不是应用级回落）。
pub(crate) fn hinted_fallback(source: &'static str) {
    #[cfg(test)]
    TEST_HINTED.with(|v| {
        let mut v = v.borrow_mut();
        match v.iter_mut().find(|(k, _)| *k == source) {
            Some((_, n)) => *n += 1,
            None => v.push((source, 1)),
        }
    });
    if let Ok(mut v) = HINTED.lock() {
        match v.iter_mut().find(|(k, _)| *k == source) {
            Some((_, n)) => *n += 1,
            None => v.push((source, 1)),
        }
    }
}

/// 推迟的 λ 位用老办法补上（`kernel_app_lambda::fill_late`，每个补上的位记一次；写明的例外）。
pub(crate) fn filled_late() {
    #[cfg(test)]
    TEST_FILLED_LATE.with(|c| c.set(c.get() + 1));
    FILLED_LATE.fetch_add(1, Ordering::Relaxed);
}

fn depth() -> usize {
    STACK.with(|s| s.borrow().len())
}

fn take(before: usize) -> Option<Fallback> {
    STACK.with(|s| {
        let mut s = s.borrow_mut();
        let m = (s.len() > before).then(|| s[before].clone());
        s.truncate(before);
        m
    })
}

fn record(f: Fallback, tag: &'static str) {
    #[cfg(test)]
    let tag = if TEST_FORCE_ERR.with(|c| c.get()) {
        "err"
    } else {
        tag
    };
    if let Some(path) = std::env::var_os("SOKO_KERNEL_APP_LOG") {
        use std::io::Write;
        let line = format!(
            "{tag} {}{} {} {}:{}\n",
            f.route,
            key(f.why),
            f.head,
            f.span.start.line,
            f.span.start.column
        );
        if let Ok(mut fh) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            let _ = fh.write_all(line.as_bytes());
        }
    }
    #[cfg(test)]
    {
        TEST_SETTLED.with(|v| v.borrow_mut().push((tag, f.why)));
        let route_why = format!("{}{}", f.route, key(f.why));
        TEST_SETTLED_ROUTED.with(|v| v.borrow_mut().push((tag, route_why)));
    }
    if let Ok(mut v) = OLD.lock() {
        match v
            .iter_mut()
            .find(|(r, t, w, _)| *r == f.route && *t == tag && *w == f.why)
        {
            Some((_, _, _, n)) => *n += 1,
            None => v.push((f.route, tag, f.why, 1)),
        }
    }
}

/// 回落原因 ⇒ 计数键（`count` 的固定键原样；其余是 `Unsupported` 的原因 ⇒ `unsupported_<净化>`）。
fn key(why: &str) -> String {
    const FIXED: &[&str] = &[
        "unsolved",
        "argmismatch",
        "expected",
        "argelab",
        "shape",
        "bad_scope",
    ];
    if FIXED.contains(&why) {
        return why.to_string();
    }
    let k: String = why
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    format!("unsupported_{k}")
}

/// 追加到 `KERNEL_APP` 行：`fallback_old_<ok|none|err>_<route><why>=n`，另有两处例外计数。
pub(crate) fn report(line: &mut String) {
    line.push_str(&format!(" probe_cut={}", PROBE_CUT.load(Ordering::Relaxed)));
    let v = OLD.lock().map(|v| v.clone()).unwrap_or_default();
    for (route, tag, why, n) in v {
        line.push_str(&format!(" fallback_old_{tag}_{route}{}={n}", key(why)));
    }
    line.push_str(&format!(
        " fill_late={}",
        FILLED_LATE.load(Ordering::Relaxed)
    ));
    if let Ok(v) = HINTED.lock() {
        for (src, n) in v.iter() {
            line.push_str(&format!(" hinted_const_fallback_{src}={n}"));
        }
    }
}

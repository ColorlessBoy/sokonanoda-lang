//! **P2-12 S0**：内核应用路径**回落之后**老路的结局（纯观测，`reports/2026-10-31-p2-12-design.md` §Q6 S0）。
//!
//! 只覆盖主路：[`super::kernel_app::try_app`] 非探测回落（交回 `MissGuard`）⇒ [`pending`] 压一条原因；
//! `try_implicit_application_with` 的唯一调用方（`elab_expr` 的 `App` 臂）用 [`settle`] 包住那次调用，
//! 返回时取走本次压的那条：`Ok(Some)` ⇒ `fallback_old_ok_<why>`，`Ok(None)`（老路不插隐式、交回普通应用臂）
//! ⇒ `fallback_old_none_<why>`，`Err` ⇒ `fallback_old_err_<why>`。嵌套应用各自成对压/取 ⇒ 栈平衡。
//! `SOKO_KERNEL_APP_LOG=<path>` ⇒ 每次另追加 `<ok|none|err> <why> <head> <line>:<col>`。只计数，不改判定。
//! 未覆盖：记法路由、零元 `try_hinted_const`、`fill_late`。

use crate::{Expr, Span};
use std::cell::RefCell;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

thread_local! {
    static STACK: RefCell<Vec<(&'static str, String, Span)>> = const { RefCell::new(Vec::new()) };
}

#[cfg(test)]
thread_local! {
    /// 测试用：本线程结算过的 `(结局, 原因)`（全局计数会被并行测试串扰）。
    pub(crate) static TEST_SETTLED: RefCell<Vec<(&'static str, &'static str)>> = const { RefCell::new(Vec::new()) };
    /// 测试用：`true` ⇒ 一律按 `err` 结算（反向验证）。
    pub(crate) static TEST_FORCE_ERR: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

static OLD: Mutex<Vec<(&'static str, &'static str, u64)>> = Mutex::new(Vec::new());
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
    STACK.with(|s| s.borrow_mut().push((why, head, expr.span())));
}

/// 包住 `try_implicit_application_with` 的一次调用：本次压过原因 ⇒ 按返回值结算。原样返回 `f()`。
pub(crate) fn settle<T, E>(f: impl FnOnce() -> Result<Option<T>, E>) -> Result<Option<T>, E> {
    let before = STACK.with(|s| s.borrow().len());
    let out = f();
    // 本次压的那条在下标 `before`；更深的残留（panic 展开没走到内层 `settle`）一并丢掉 ⇒ 栈恒平衡。
    let mine = STACK.with(|s| {
        let mut s = s.borrow_mut();
        let m = (s.len() > before).then(|| s[before].clone());
        s.truncate(before);
        m
    });
    if let Some((why, head, span)) = mine {
        let tag = match &out {
            Ok(Some(_)) => "ok",
            Ok(None) => "none",
            Err(_) => "err",
        };
        #[cfg(test)]
        let tag = if TEST_FORCE_ERR.with(|c| c.get()) {
            "err"
        } else {
            tag
        };
        record(tag, why, &head, span);
    }
    out
}

fn record(tag: &'static str, why: &'static str, head: &str, span: Span) {
    if let Some(path) = std::env::var_os("SOKO_KERNEL_APP_LOG") {
        use std::io::Write;
        let line = format!(
            "{tag} {} {head} {}:{}\n",
            key(why),
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
    #[cfg(test)]
    TEST_SETTLED.with(|v| v.borrow_mut().push((tag, why)));
    if let Ok(mut v) = OLD.lock() {
        match v.iter_mut().find(|(t, w, _)| *t == tag && *w == why) {
            Some((_, _, n)) => *n += 1,
            None => v.push((tag, why, 1)),
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

/// 追加到 `KERNEL_APP` 行：`fallback_old_<ok|none|err>_<why>=n …`。
pub(crate) fn report(line: &mut String) {
    line.push_str(&format!(" probe_cut={}", PROBE_CUT.load(Ordering::Relaxed)));
    let v = OLD.lock().map(|v| v.clone()).unwrap_or_default();
    for (tag, why, n) in v {
        line.push_str(&format!(" fallback_old_{tag}_{}={n}", key(why)));
    }
}

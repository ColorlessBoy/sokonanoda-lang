//! **P2-1 (a)**：`judge_infer` 未命中的**回落归因**（`SOKO_JUDGE_STATS=1` ⇒ `KERNEL_APP_MISS <reason>=N …`）。
//!
//! 每次内核 `elab_app` 回落，[`super::kernel_app::try_app`] 交回一个 [`MissGuard`]：它把回落原因压进
//! 线程局部栈，调用方（`try_implicit_application_with`）持有到函数返回 ⇒ 老路仍在跑的那段时间里栈顶 =
//! **最内层**仍在走老路的应用。`judge_infer` 未命中时读栈顶记账（空栈 = `none`）。只计数，不改判定。

use std::cell::RefCell;
use std::sync::atomic::Ordering;
use std::sync::Mutex;

thread_local! {
    static STACK: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
}

static MISSES: Mutex<Vec<(&'static str, u64)>> = Mutex::new(Vec::new());

/// 回落原因在栈上的存续期（drop ⇒ 弹出）。
#[must_use]
pub(crate) struct MissGuard(());

impl MissGuard {
    pub(crate) fn push(reason: &'static str) -> Self {
        STACK.with(|s| s.borrow_mut().push(reason));
        MissGuard(())
    }
}

impl Drop for MissGuard {
    fn drop(&mut self) {
        STACK.with(|s| {
            s.borrow_mut().pop();
        });
    }
}

/// 栈顶原因（测试用）。
#[cfg(test)]
pub(crate) fn top() -> Option<&'static str> {
    STACK.with(|s| s.borrow().last().copied())
}

/// `judge_infer` 未命中：`INFER_MISSES += 1`（返回新值，同原来的 `fetch_add + 1`）并按栈顶归因。
pub(crate) fn note_infer_miss() -> u64 {
    let miss = crate::judge::stats::INFER_MISSES.fetch_add(1, Ordering::Relaxed) + 1;
    if std::env::var_os("SOKO_JUDGE_STATS").is_some() {
        let key = STACK.with(|s| s.borrow().last().copied()).unwrap_or("none");
        if let Ok(mut v) = MISSES.lock() {
            match v.iter_mut().find(|(k, _)| *k == key) {
                Some((_, n)) => *n += 1,
                None => v.push((key, 1)),
            }
        }
        install_printer();
    }
    miss
}

fn install_printer() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
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

/// `KERNEL_APP_MISS total=N <reason>=n …`（按次数降序）。
pub(crate) fn report_line() -> String {
    let mut v = MISSES.lock().map(|v| v.clone()).unwrap_or_default();
    v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
    let total: u64 = v.iter().map(|(_, n)| n).sum();
    let mut line = format!("KERNEL_APP_MISS total={total}");
    for (k, n) in v {
        line.push_str(&format!(" {}={n}", k.replace(' ', "_")));
    }
    line
}

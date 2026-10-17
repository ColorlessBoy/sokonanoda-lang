//! **按线程**的 panic 静音区（`KERNEL-GLOBAL-QUIET-HOOK-SILENCES-PANICS`）。
//!
//! 内核以 panic 报"拒绝"（`docs/architecture.md` §8 gotcha 2），而
//! [`crate::util::ExportFile::try_check_declar_at`] 把它 `catch_unwind` 成
//! `CheckError::Rejected` ⇒ 那些 panic 文本**不该**打给用户。这里提供那个静音区。
//!
//! **为什么必须按线程**（2026-10-17，洞 `KERNEL-GLOBAL-QUIET-HOOK-SILENCES-PANICS`）：
//! 旧实现是 `take_hook()` → `set_hook(|_| {})` → 还原 —— panic hook 是**进程全局**的，
//! 而内核从 CLI 的入口级并行编译起会在**多个线程**上被调用
//! （`crates/cli/src/build.rs` 的并行循环）⇒ 静音窗口里**别的线程**的 panic 一个字
//! 都不打 ✗。后果是 CI 判红时日志里没有断言文本（`failures:` 紧接第二个
//! `failures:`，没有一句 `---- … stdout ----`；`docs/CI-FAILURES.md` 2026-10-17 那条）；
//! 而且它会把 front 已经修好的**按线程** hook
//! （`crates/front/src/compile/check/mod.rs` 的 `install_quiet_hook`）整个换掉 ✗。
//! 判据改成"问本线程的计数"后，别处的 panic 照常走到上一个 hook —— 与 front 那条同构
//! （同一条教训：`docs/CI-FAILURES.md` 2026-09-23）。
//!
//! 顺带也是性能：`take_hook`/`set_hook` 每次调用都要拿全局锁并装箱，而
//! `try_check_declar*` 在**编译热路**上（`compile/check/kernel_phase.rs`、`walk.rs`）。

thread_local! {
    /// 本线程正处在「静音」区里（[`quiet`]）的嵌套层数。
    static QUIET_DEPTH: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

/// 装一次「**按线程**静音」的 panic hook（进程只装一次，装完永不卸）。
///
/// hook 只在**当前线程**处于静音区时返回，别处的 panic 照常走到上一个 hook。
fn install_quiet_hook() {
    static ONCE: std::sync::OnceLock<()> = std::sync::OnceLock::new();
    ONCE.get_or_init(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            if quiet_depth() > 0 {
                return;
            }
            previous(info);
        }));
    });
}

/// 进入静音区；离开时（含 unwind）自动恢复。可嵌套。
pub(crate) fn quiet() -> QuietGuard {
    install_quiet_hook();
    QUIET_DEPTH.with(|depth| depth.set(depth.get() + 1));
    QuietGuard
}

/// 本线程当前的静音嵌套层数——[`install_quiet_hook`] 的唯一判据
/// （「静音」只认**当前线程**的计数）。
fn quiet_depth() -> u32 {
    QUIET_DEPTH.with(|depth| depth.get())
}

pub(crate) struct QuietGuard;

impl Drop for QuietGuard {
    fn drop(&mut self) {
        QUIET_DEPTH.with(|depth| depth.set(depth.get().saturating_sub(1)));
    }
}

//! **严格档 v2 的 provenance 骨架**（倒 T 型安全腿 P2）✓。
//!
//! W1/W2 才填实现：`Expr::Engine` 包装、包装计数、漏点计数。本文件先存在，
//! 是为了把 W 线的代码挡在 `elab.rs` / `judge.rs` / `by.rs` 之外 —— 那三份在
//! `scripts/docs-lint.py` 判据 ⑨ 的冻结清单上（余量 4 / 1 / 1 行）。
//!
//! 安全腿阶段这个模块**不参与判定**：所有函数都只在测试里被调用，
//! `#![allow(dead_code)]` 在 W2 接线时删掉。
#![allow(dead_code)]

use std::sync::atomic::{AtomicU64, Ordering};

/// 引擎 AST 被包装的次数（W2 起 > 0；安全腿阶段恒 0）。
static ENGINE_WRAPS: AtomicU64 = AtomicU64::new(0);

/// W2 的包装点调用它（安全腿阶段只由测试调用）。
pub(crate) fn note_engine_wrap() {
    ENGINE_WRAPS.fetch_add(1, Ordering::Relaxed);
}

/// 读数：本进程包装了多少个引擎 AST。
pub(crate) fn engine_wraps() -> u64 {
    ENGINE_WRAPS.load(Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 骨架自身的判据：计数器**真的**能加（防止 W2 接线时发现它是个空壳）。
    #[test]
    fn counter_starts_and_increments() {
        let before = engine_wraps();
        note_engine_wrap();
        assert_eq!(engine_wraps(), before + 1);
    }
}

//! **S5（2026-10-23）**：`by` 引擎的**延迟义务**（基础设施；接线见
//! `plans/apply-unification-plan.md` §18 第 2–4 步）。
//!
//! 为什么是 by 引擎的义务而不是 `Judgement` 的变体：`flush_batch` 把任何非 `Match`
//! 都当失败判定并缓存（`judge.rs:2378`/`:113`）⇒ 加 `Pending` 会污染乐观批。
use crate::span::Span;

/// **S5（2026-10-23）第一步**：延迟的 tactic 义务（**默认关**，`SOKO_DEFER=1` 才启用）。
///
/// 为什么是 by 引擎的义务而不是 `Judgement` 的变体：`flush_batch` 把任何非 `Match`
/// 都当失败判定并缓存（`judge.rs:2378`/`:113`）⇒ 加 `Pending` 会污染乐观批。
/// 计划见 `plans/apply-unification-plan.md` §18。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeferredTactic {
    /// 目标节点（收尾时按"是否仍是洞"过滤已解除的义务）。
    pub node: usize,
    /// 该 tactic 的源码 span（诊断位置）。
    pub span: Span,
    /// 原始错误码（`elab-implicit-argument-unsolved` 一族）。
    pub code: String,
    pub message: String,
}

/// 可延迟的错误码（第一步只认三条隐式/记法未解；Clash 通道**不**在此列，见 §18）。
#[allow(dead_code)] // S5 接线（§18 第 2–4 步）之前只有基础设施与读数
pub const DEFERRABLE_CODES: [&str; 3] = [
    "elab-implicit-argument-unsolved",
    "elab-notation-argument-unsolved",
    "elab-binder-notation-unsolved",
];

/// 开关（只读一次环境）：`SOKO_DEFER=1` 才启用延迟；**不设 = 关**（与 §18 的字节判据一致）。
#[allow(dead_code)] // S5 接线（§18 第 2–4 步）之前只有基础设施与读数
pub fn defer_enabled() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var("SOKO_DEFER").is_ok_and(|v| v == "1"))
}

/// 这个错误码是否属于可延迟一族。
#[allow(dead_code)] // S5 接线（§18 第 2–4 步）之前只有基础设施与读数
pub fn is_deferrable(code: &str) -> bool {
    DEFERRABLE_CODES.contains(&code)
}

thread_local! {
    /// 最近一次 `by` 块记下的延迟义务（判据读数；每块开始时清空）。
    static DEFERRED_LAST: std::cell::RefCell<Vec<DeferredTactic>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

/// 判据读数：最近一次 `by` 块的延迟义务（`#[doc(hidden)]`，只给测试/诊断用）。
#[doc(hidden)]
pub fn deferred_report() -> Vec<DeferredTactic> {
    DEFERRED_LAST.with(|d| d.borrow().clone())
}

/// 判据用：记一条（公开给集成测试；生产路径用 [`record_deferred`]）。
#[doc(hidden)]
pub fn record_deferred_for_test(entry: DeferredTactic) {
    record_deferred(entry);
}

/// 判据用：可延迟码判定（公开给集成测试）。
#[doc(hidden)]
pub fn is_deferrable_for_test(code: &str) -> bool {
    is_deferrable(code)
}

/// 判据用（测试之间隔离）：清空读数。
#[doc(hidden)]
pub fn deferred_reset() {
    DEFERRED_LAST.with(|d| d.borrow_mut().clear());
}

/// 记一条延迟义务（只给 by 引擎内部用）。
#[allow(dead_code)] // S5 接线（§18 第 2–4 步）之前只有基础设施与读数
pub fn record_deferred(entry: DeferredTactic) {
    DEFERRED_LAST.with(|d| d.borrow_mut().push(entry));
}

/// 清空本块的延迟义务（`run_by` 入口调用）。
#[allow(dead_code)] // S5 接线（§18 第 2–4 步）之前只有基础设施与读数
pub fn clear_deferred() {
    DEFERRED_LAST.with(|d| d.borrow_mut().clear());
}


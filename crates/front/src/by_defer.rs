//! **S5（2026-10-23）**：`by` 引擎的**延迟义务**（`SOKO_DEFER=1`；默认关）。
//!
//! 为什么是 by 引擎的义务而不是 `Judgement` 的变体：`flush_batch` 把任何非 `Match`
//! 都当失败判定并缓存（`judge.rs:2378`/`:113`）⇒ 加 `Pending` 会污染乐观批。
//!
//! 语义（`plans/apply-unification-plan.md` §18 第 2–4 步）：顶层 `apply`/`exact`
//! 的判定回 `elab-*-unsolved` 时，开关开着就**不判死** —— 记一条 [`DeferredTactic`]、
//! 目标节点原样开着、tactic 返回 `Ok` ⇒ `by` 块**继续**。收尾只留**仍然开着**的
//! 那些（后来的 tactic 把节点补上了 ⇒ 义务已兑现）。
use crate::compile::{CompileError, ErrorKind};
use crate::judge::Judgement;
use crate::span::Span;

/// **S5 第 2–4 步**：tactic 的**延迟出口**（`None` ⇒ 这一处**不许**推迟）。
///
/// 只有顶层 `apply` / `exact`（以及它们所在的 `by`/`have`/`cases` 臂）传 `Some`；
/// `apply` 的判等兜底、`ctor_tactic`、`use` 传 `None` —— 那里"判定成功"本身有语义
/// （§18 陷阱 2：兜底会把"延迟的 `Ok`"误读成"目标已关闭"）。
pub type DeferSink<'a> = Option<&'a mut Vec<DeferredTactic>>;

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
pub const DEFERRABLE_CODES: [&str; 3] = [
    "elab-implicit-argument-unsolved",
    "elab-notation-argument-unsolved",
    "elab-binder-notation-unsolved",
];

/// 开关（只读一次环境）：`SOKO_DEFER=1` 才启用延迟；**不设 = 关**（与 §18 的字节判据一致）。
pub fn defer_enabled() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var("SOKO_DEFER").is_ok_and(|v| v == "1"))
}

/// 这个错误码是否属于可延迟一族。
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
pub fn record_deferred(entry: DeferredTactic) {
    DEFERRED_LAST.with(|d| d.borrow_mut().push(entry));
}

/// 清空本块的延迟义务（`run_by` 入口调用）。
pub fn clear_deferred() {
    DEFERRED_LAST.with(|d| d.borrow_mut().clear());
}

/// 记下**整块**的结果（`run_by` 出口调用；替换旧值 ⇒ 读数只反映最近一次）。
pub fn set_deferred(entries: Vec<DeferredTactic>) {
    DEFERRED_LAST.with(|d| *d.borrow_mut() = entries);
}

/// 只留**仍然开着**的目标：后面的 tactic 把该节点补上了 ⇒ 这次推迟已兑现，
/// 不该在最终报告里出现（判据 `NodeKind::Hole` 与 `node_body` 同源，由调用方给）。
pub fn retain_open(
    entries: Vec<DeferredTactic>,
    still_open: impl Fn(usize) -> bool,
) -> Vec<DeferredTactic> {
    entries.into_iter().filter(|d| still_open(d.node)).collect()
}

/// 这条判定失败能不能**改成推迟**：`true` ⇒ 已记入 sink（目标节点原样开着），
/// 调用方 `return Ok(())` 让块继续。**默认档**（`SOKO_DEFER` 未设）恒 `false`
/// ⇒ 一个分支都不多走，逐字节回到今天 ✓。
///
/// 放在这里（而不是 `by.rs`）是 `docs-lint.py` ⑨ 的要求：`by.rs` 已顶到行数基线，
/// 而"要不要推迟"正是本模块的语义。
pub fn take_deferrable(
    sink: DeferSink<'_>,
    node: usize,
    span: Span,
    code: &str,
    message: &str,
) -> bool {
    let Some(sink) = sink else { return false };
    if !defer_enabled() || !is_deferrable(code) {
        return false;
    }
    sink.push(DeferredTactic {
        node,
        span,
        code: code.to_string(),
        message: message.to_string(),
    });
    true
}

/// `exact` 的判定失败：可推迟 ⇒ 记下来、`Ok`（目标**留着**）；否则**今天那句**
/// 诊断（码/hint 仍走 `from_judgement`，逐字节不变 ✓）。
pub fn exact_error(
    sink: DeferSink<'_>,
    node: usize,
    span: Span,
    code: String,
    message: String,
    kind: Option<ErrorKind>,
) -> Result<(), CompileError> {
    if take_deferrable(sink, node, span, &code, &message) {
        return Ok(());
    }
    Err(CompileError::from_judgement(
        kind,
        format!("`exact` 判定失败：{message}"),
        span,
    ))
}

/// `apply` 推断被应用函数类型的结果：`Ok(Some(ty))` = 照今天继续；`Ok(None)` =
/// **已推迟**（调用方 `return Ok(())`）；`Err` = 今天那句诊断。
pub fn apply_infer_error(
    judgement: Judgement,
    sink: DeferSink<'_>,
    node: usize,
    span: Span,
) -> Result<Option<String>, CompileError> {
    match judgement {
        Judgement::Error {
            code,
            message,
            kind,
        } => {
            if take_deferrable(sink, node, span, &code, &message) {
                return Ok(None);
            }
            Err(CompileError::from_judgement(kind, message, span))
        }
        Judgement::Mismatch { .. } => Err(CompileError::elab(
            ErrorKind::ElabTacticFailed,
            "无法推断被应用函数的类型",
            span,
        )),
        Judgement::Match => unreachable!(),
    }
}

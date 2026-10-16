//! **T2-B（2026-10-09）的入口尾快照那一块**（`AGENTS.md` 硬规则 5：从 `session.rs` 拆出 ✓）。
//!
//! ⚠ **拆出来的唯一理由是行数**：`session.rs` 在 `scripts/docs-budget.json` 的基线里被
//! **冻结**（只许减不许增 ✗），而 `ARENA-BUDGET-SHARED-BY-TWO-DRIVERS`（2026-10-16）的修
//! 往里加了 68 行 ⇒ `docs-lint.py` 判据 ⑨ 判红。**不许抬上限** ✗ ⇒ 按硬规则 5 拆 ✓。
//! **纯搬移**：`EntryTail` + 尾快照的线程局部 + 两个逃生门 + 键函数，全部只被 `session.rs`
//! 自己引用（搬出去之后加 `pub(super)` ✓）；语义一个字节没动 ✓。
//!
//! ## 这一块是什么
//!
//! * [`EntryTail`] —— 入口趟的**命令级检查点**（跨按键）：装的是"**最后一条命令之前**那个
//!   边界"的 walk 状态 ⇒ 同入口**只有最后一条变了**时从它续编 ⇒ **只 elaborate 1 条命令** ✓；
//! * [`ENTRY_TAILS`] —— 它的线程局部槽（寿命必须 `'static`：库层检查点的 arena 由
//!   `Box::leak` 来 ✓）；按 `(库层键, 入口)` 去重 + 有界（[`MAX_ENTRY_TAILS`]）✓；
//! * [`no_entry_snapshot`] —— **T2-B 的逃生门**（`SOKO_NO_ENTRY_SNAPSHOT=1` ⇒ 不续编）；
//! * [`no_lru_keep`] —— ⭐ **`LEAKED-LIB-ARENAS-CLIFF` 的逃生门**（2026-10-16）：
//!   `SOKO_NO_LRU_KEEP=1` ⇒ 回退路**把修前的策略原样装回来**（① `slots.clear()` 抹掉
//!   LRU 里那些**已经泄漏、键仍然有效**的检查点 ✗；② 轮换闸**不看**泄漏预算 ⇒ 上界 ①
//!   用尽之后依旧按 `MAX_REUSES_PER_CHECKPOINT` 停用那一份检查点 ✗）。留着它的两个理由
//!   （与 `SOKO_NO_ENTRY_SNAPSHOT` 同一类）：① **同一份构建上的 A/B**（纯性能改动 ⇒ 两臂
//!   必须在同一份二进制、同一份夹具、同一操作序列上量 ✓）；② **咬住证明**
//!   （`docs/guard-bite.jsonl`：把"已存在的坏输入"变成一条**可执行**的输入 ✓）。
//!   ⚠ **默认关 ⇒ 生产零影响** ✓；读一次就缓存（在每次编辑的热路径上 ✓）。
//!
//! ## ⚠ 与 `MAX_LEAKED_LIB_ARENAS` 的关系（2026-10-16 实测 · 别当成一处预算 ✗）
//!
//! `MAX_ENTRY_TAILS = 8` 与上界 ① = 8 **数值恰好重合**，而 `compile_plan_incremental`
//! 的 `entries` 恒为 **1 条** ⇒ 尾快照数 ≤ 键数 ≤ 上界 ① = 8 ⇒ **本上界当前永远不会先绑**
//! ✓（横向排查的读数，见 `docs/perf/ledger.jsonl` 的 `arena_budget_two_drivers` 行 ✓）。

use std::cell::RefCell;

use crate::compile::SourceUnit;

/// **T2-B（2026-10-09）**：入口趟的**命令级检查点**（跨按键）。
///
/// 装的是"**最后一条命令之前**那个边界"的 walk 状态（`cp.idx = 命令数 − 2`）⇒
/// 同入口**只有最后一条变了**时从它续编 ⇒ **只 elaborate 1 条命令** ✓。
///
/// ⚠ **报告侧累加器不进快照、也不装回**（`out`/`ops`/`cmd_hovers`/`decl_states`）：
/// 前缀那一段的报告由**既有的 `EntryCache` 拼接**提供（计划 §3.3 的"复用判据沿用
/// `EntryCache`"就是这个意思 ✓）—— 两边都算会让报告里**每条声明出现两次** ✗
/// （实测过：整趟 13 条 vs 续编 22 条 ✓ 被守卫逮到）。快照只负责**环境侧** ✓。
///
/// ⚠ **有界的第一步**：只留**一个**边界（W = 1）；"改第 k 条 ⇒ N−k+1"要每个边界
/// 一份（有界窗口 W）⇒ 设计档 §6.2(b) 的下一刀 ✓。
pub(super) struct EntryTail<'a> {
    // ⚠ **字段也要 `pub(super)`**：构造与复用判据都在父模块 `session.rs` 里 —— 拆出来
    // 之前它们同模块 ⇒ 字段可见 ✓；拆出去之后不加就编译不过 ✗（实测
    // `error[E0616]: field … of struct EntryTail is private`）。
    pub(super) key: String,
    pub(super) entry: usize,
    pub(super) keys: Vec<String>,
    pub(super) cp: crate::compile::WalkCheckpoint<'a>,
}

/// **T2-B**：入口检查点的线程局部槽（同 `LIB_CHECKPOINTS` 的纪律 ✓）—— 寿命必须是
/// `'static`（库层检查点的 arena 由 `Box::leak` 来 ✓）；按 `(库层键, 入口)` 去重 + 有界 ✓。
/// ⭐ **T2-B 的逃生门**（2026-10-09）：`SOKO_NO_ENTRY_SNAPSHOT=1` ⇒ 入口趟**不续编**
/// （回到 T2-B 之前的"整条重走"✓）。用途 = **同一份构建上的 A/B**（`STATUS.md` 的已知缺口 ✓）。
/// 读一次就缓存（它在每次编辑的热路径上 ✓）。
pub(super) fn no_entry_snapshot() -> bool {
    static OFF: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *OFF.get_or_init(|| std::env::var_os("SOKO_NO_ENTRY_SNAPSHOT").is_some())
}

/// ⭐ **`LEAKED-LIB-ARENAS-CLIFF` 的逃生门**（2026-10-16）：`SOKO_NO_LRU_KEEP=1` ⇒
/// 回退路**把修前的策略原样装回来** —— ① `slots.clear()`（把 LRU 里那些**已经泄漏、
/// 键仍然有效**的检查点一起抹掉 ✗）；② 轮换闸**不看**泄漏预算（上界 ① 用尽之后
/// 依旧按 `MAX_REUSES_PER_CHECKPOINT` 停用那一份检查点 ✗）。
///
/// ## 为什么留它（与 `SOKO_NO_ENTRY_SNAPSHOT` 同一类）
///
/// ① **同一份构建上的 A/B**：这是一处**纯性能**改动（判定路径一个字节没动）⇒
///    两臂必须在**同一份二进制、同一份夹具、同一操作序列**上量（`AGENTS.md` 判据
///    纪律②：跨机墙钟不可转移 ✓）；② **咬住证明**（`docs/guard-bite.jsonl`）：
///    它把"已存在的坏输入"（修前那条路）变成一条**可执行**的输入 ✓。
///
/// ⚠ **默认关 ⇒ 生产零影响** ✓；读一次就缓存（在每次编辑的热路径上 ✓）。
pub(super) fn no_lru_keep() -> bool {
    static OFF: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *OFF.get_or_init(|| std::env::var_os("SOKO_NO_LRU_KEEP").is_some())
}

pub(super) const MAX_ENTRY_TAILS: usize = 8;
thread_local! {
    pub(super) static ENTRY_TAILS: RefCell<Vec<EntryTail<'static>>> = const { RefCell::new(Vec::new()) };
}

/// 入口的**命令原文**（判据形状：文本 + 顺序；与 `EntryCache.keys` 同一口径 ✓）。
/// ⚠ **不用哈希**（计划 §3.3 的纪律）：结构相等才作数 —— 错键 = 静默错编 ✗。
pub(super) fn entry_command_keys(units: &[SourceUnit<'_>]) -> Vec<String> {
    let mut out = Vec::new();
    for unit in units {
        for command in unit.file.commands.iter() {
            let span = command.span();
            out.push(unit.file.src[span.start.offset..span.end.offset].to_string());
        }
    }
    out
}

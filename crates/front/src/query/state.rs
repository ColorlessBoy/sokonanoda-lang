//! 光标处状态的选择（Lean `goalsAt?` 语义的**唯一实现**）。
//!
//! 从 `mod.rs` 拆出（模块化硬规则：单文件 ~500 行上限）。语义与协议原文见
//! [`select_state_at`] 的文档；适配器（LSP/CLI/MCP）只做坐标转换，不得重算。

use crate::compile::{ByGoalState, DeclState};
use crate::span::Span;

/// 光标处的状态选择（Lean `goalsAt?` 语义的唯一实现）。
pub struct StateSelection {
    pub goals: Vec<ByGoalState>,
    pub span: Option<Span>,
    pub step: i64,
    pub total: usize,
}

/// 光标处的状态选择（**Lean `goalsAt?` 语义的唯一实现**，协议原文见
/// `docs/protocol.md` §`soko/stateAt`）：
///
/// - 光标落在某条 tactic 的 span 内（**半开区间** `start <= cursor < end`：光标
///   恰在 tactic 末尾算"之后"，不算"之内"）→ 该 tactic **执行前**的状态，
///   即第 `i-1` 条执行后的状态（`i == 0` 时为根状态）；
/// - 否则取"最后一条在光标前（含恰好结束）结束的 tactic"之后的状态；
/// - 根状态（`step: -1`）：**声明类型的内核渲染文本**（`ty_text`，未知时退回走查
///   的剩余目标）+ **空 binders**，`span` = 声明范围；
/// - **没有 `by` 块的声明**（`axiom`、lambda 前缀 + `sorry` 的半成品、已证完的
///   声明）：`step: -1`、`total: 0`，退回声明自己的剩余目标/上下文
///   （`goal` + `binders`；已闭合时为 `[]` ⇒ wire `goal: null`）。这与"根状态"
///   是两回事——根状态只属于有 tactic 的声明。
///
/// 这些条款都是协议规定、且 LSP 客户端（VS Code Infoview / 练习树）依赖的行为；
/// 真相层必须与之逐字一致——先前这里的闭区间与"根状态带 binders/剩余目标"是
/// 错的（`docs/design/agent-query-channel.md` 的 H6-A 一致性契约正是为此）。
pub fn select_state_at(d: &DeclState, cursor: usize) -> StateSelection {
    // 无 `by` ⇒ 没有 per-tactic 状态可选，协议规定退回声明级的目标/上下文。
    if d.by_steps.is_empty() {
        return StateSelection {
            goals: d
                .goal
                .clone()
                .map(|ty| ByGoalState {
                    ty,
                    binders: d.binders.clone(),
                })
                .into_iter()
                .collect(),
            span: Some(d.span),
            step: -1,
            total: 0,
        };
    }
    let root = || StateSelection {
        goals: d
            .ty_text
            .clone()
            .or_else(|| d.goal.clone())
            .map(|ty| ByGoalState {
                ty,
                binders: Vec::new(),
            })
            .into_iter()
            .collect(),
        span: Some(d.span),
        step: -1,
        total: d.by_steps.len(),
    };
    let selected = match d
        .by_steps
        .iter()
        .position(|s| s.span.start.offset <= cursor && cursor < s.span.end.offset)
    {
        Some(i) => i as i64 - 1,
        None => d
            .by_steps
            .iter()
            .rposition(|s| s.span.end.offset <= cursor)
            .map(|i| i as i64)
            .unwrap_or(-1),
    };
    let Some(step) = usize::try_from(selected)
        .ok()
        .filter(|i| *i < d.by_steps.len())
    else {
        return root();
    };
    let s = &d.by_steps[step];
    StateSelection {
        goals: s.goals.clone(),
        span: Some(s.span),
        step: selected,
        total: d.by_steps.len(),
    }
}

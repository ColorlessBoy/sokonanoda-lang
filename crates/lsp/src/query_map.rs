//! 真相层 → `soko/*` wire 的**唯一**形状映射点（`docs/protocol.md`）。
//!
//! `sokonanoda_front::query`（编辑器无关的真相层）用**字节 offset** 作答、
//! 版本是 `u64`；LSP 的 wire 要 `Range`（0-based 行 + UTF-16 `character`）与
//! `i32` 版本。本模块只做这两件事：字段改名、坐标/版本换算。
//!
//! 纪律（`docs/design/agent-query-channel.md` H6-A，`REQUIREMENTS.md` §2 第 4 条）：
//! 这里**不做任何判定**——哪个洞、哪个目标、哪条状态、文本长什么样全部来自真相
//! 层；本模块不读报告、不扫文本、不重新分类，否则就是第二份真相。

use sokonanoda_front::query as truth;
use tower_lsp::lsp_types::{Position, Range};

use super::protocol::{
    GoalBinderInfo, GoalDeclInfo, HoleInfo, RunInfo, StateAtResponse, StateDeclInfo, StateGoalInfo,
    SubGoalInfo,
};

/// 字节 offset → 0-based LSP `Position`。
///
/// 行列口径取真相层的 [`truth::line_col_of`]（1-based 行、UTF-16 列，与 LSP 的
/// `character` 一致），这里只做基数换算。
fn position_of(text: &str, offset: usize) -> Position {
    let (line, col) = truth::line_col_of(text, offset);
    Position {
        line: line.saturating_sub(1) as u32,
        character: col.saturating_sub(1) as u32,
    }
}

/// 字节区间 → LSP `Range`。wire 上所有区间（声明/洞/目标 span）都从这里出。
pub(crate) fn range_of_offsets(text: &str, start: usize, end: usize) -> Range {
    Range {
        start: position_of(text, start),
        end: position_of(text, end),
    }
}

pub(crate) fn run_info(run: truth::RunInfo) -> RunInfo {
    RunInfo {
        text: run.text,
        kind: run.kind,
    }
}

pub(crate) fn binder_info(binder: truth::BinderInfo) -> GoalBinderInfo {
    GoalBinderInfo {
        name: binder.name,
        ty: binder.ty,
        ty_runs: binder.ty_runs.into_iter().map(run_info).collect(),
    }
}

pub(crate) fn hole_info(hole: truth::HoleInfo, text: &str) -> HoleInfo {
    HoleInfo {
        range: range_of_offsets(text, hole.start, hole.end),
        id: hole.id,
    }
}

pub(crate) fn sub_goal_info(sub_goal: truth::SubGoalInfo, text: &str) -> SubGoalInfo {
    SubGoalInfo {
        range: range_of_offsets(text, sub_goal.start, sub_goal.end),
        ty: sub_goal.ty,
    }
}

/// `soko/goals` 的一个声明条目。
///
/// 真相层的 `code_actions` 在 LSP 侧没有对应字段：判卷建议走
/// `textDocument/codeAction`（`crates/lsp/src/actions.rs`，需要 judge 上下文），
/// 既有 wire 与既有行为都保持不变。
pub(crate) fn decl_info(decl: truth::DeclInfo, text: &str) -> GoalDeclInfo {
    GoalDeclInfo {
        name: decl.name,
        kind: decl.kind,
        status: decl.status,
        range: range_of_offsets(text, decl.start, decl.end),
        ty: decl.ty,
        ty_runs: decl.ty_runs.into_iter().map(run_info).collect(),
        goal: decl.goal,
        goals: decl.goals,
        binders: decl.binders.into_iter().map(binder_info).collect(),
        hole: decl
            .hole
            .map(|(start, end)| range_of_offsets(text, start, end)),
        holes: decl
            .holes
            .into_iter()
            .map(|hole| hole_info(hole, text))
            .collect(),
        sub_goals: decl
            .sub_goals
            .into_iter()
            .map(|sub_goal| sub_goal_info(sub_goal, text))
            .collect(),
    }
}

/// `soko/stateAt` 的 `goals[]` 条目。
pub(crate) fn goal_info(goal: truth::GoalInfo) -> StateGoalInfo {
    StateGoalInfo {
        goal: goal.goal,
        goal_runs: goal.goal_runs.into_iter().map(run_info).collect(),
        binders: goal.binders.into_iter().map(binder_info).collect(),
    }
}

/// `soko/stateAt` 响应（`Backend::state_at` 的唯一出口）。
pub(crate) fn state_answer(uri: &str, text: &str, answer: truth::StateAnswer) -> StateAtResponse {
    StateAtResponse {
        uri: uri.to_string(),
        // LSP wire 的版本是 i32（见 `Doc::version`）；真相层是 u64。
        version: answer.version as i32,
        decl: answer.decl.map(|decl| StateDeclInfo {
            name: decl.name,
            kind: decl.kind,
            status: decl.status,
            range: range_of_offsets(text, decl.start, decl.end),
        }),
        goal: answer.goal,
        goal_runs: answer.goal_runs.into_iter().map(run_info).collect(),
        binders: answer.binders.into_iter().map(binder_info).collect(),
        goals: answer.goals.into_iter().map(goal_info).collect(),
        span: answer
            .span
            .map(|(start, end)| range_of_offsets(text, start, end)),
        step: answer.step,
        total: answer.total,
    }
}

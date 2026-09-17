//! `soko/*` 自定义请求的 wire 类型（`docs/protocol.md`）。
//!
//! 这里是 wire 形状的**唯一定义处**：字段名与语义就是协议本身，改字段名等于
//! 破坏协议。`query_map` 只是把真相层的答案映射到这些类型上。

use serde::{Deserialize, Serialize};
use tower_lsp::lsp_types::{Position, Range, TextDocumentIdentifier};

// ---- soko/* 自定义请求的 wire 类型（docs/protocol.md）----

#[derive(Debug, Deserialize)]
pub(crate) struct GoalsParams {
    #[serde(rename = "textDocument")]
    #[allow(dead_code)]
    pub(crate) text_document: TextDocumentIdentifier,
    #[allow(dead_code)]
    #[serde(default)]
    pub(crate) position: Option<Position>,
}

#[derive(Debug, Serialize)]
pub(crate) struct RunInfo {
    pub(crate) text: String,
    /// [`sokonanoda_front::semantic::SemanticKind::as_str`]; absent = plain
    /// connector (whitespace/punctuation), drawn unstyled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) kind: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct GoalBinderInfo {
    pub(crate) name: String,
    pub(crate) ty: String,
    /// Semantic runs of `ty` (`docs/design/goal-rendering.md` §2.1): the same
    /// classification the editor's semantic tokens use, so hover and the
    /// Infoview can never drift.
    pub(crate) ty_runs: Vec<RunInfo>,
}

#[derive(Debug, Serialize)]
pub(crate) struct HoleInfo {
    pub(crate) range: Range,
    /// Stable per (declaration, hole order) within a document version
    /// (`<declName>:<index>`, docs/protocol.md); anonymous examples use the
    /// `example@<line>` name form.
    pub(crate) id: String,
    /// This hole is a **redundant** `sorry` (deleting that line makes the whole
    /// declaration check), not an unsolved goal
    /// (`docs/design/redundant-sorry.md`).
    pub(crate) redundant: bool,
}

#[derive(Debug, Serialize)]
pub(crate) struct GoalDeclInfo {
    pub(crate) name: String,
    pub(crate) kind: String,
    pub(crate) status: String,
    pub(crate) range: Range,
    /// The declaration's kernel-rendered type (signature), when known — used by
    /// the Infoview declaration list as a small hint (`docs/protocol.md`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) ty: Option<String>,
    /// Semantic runs of `ty` (same single source as `goal_runs`, §2.1).
    pub(crate) ty_runs: Vec<RunInfo>,
    pub(crate) goal: Option<String>,
    /// Every open goal after the last recorded tactic (current goal first),
    /// or the single walked remaining goal for non-`by` open exercises.
    /// Empty for non-open declarations. Powers the multi-goal exercise panel.
    pub(crate) goals: Vec<String>,
    pub(crate) binders: Vec<GoalBinderInfo>,
    pub(crate) hole: Option<Range>,
    /// Every `sorry` in the answer (main hole + constructor-spine sub-holes),
    /// as `{range, id}` objects (`id` = `<declName>:<index>`).
    pub(crate) holes: Vec<HoleInfo>,
    /// Expected types for the sub-holes, positionally aligned with `holes`
    /// subset that came from a constructor spine (server-side walk).
    pub(crate) sub_goals: Vec<SubGoalInfo>,
}

#[derive(Debug, Serialize)]
pub(crate) struct SubGoalInfo {
    pub(crate) range: Range,
    pub(crate) ty: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct GoalsResponse {
    pub(crate) decls: Vec<GoalDeclInfo>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct NextHoleParams {
    #[serde(rename = "textDocument")]
    #[allow(dead_code)]
    pub(crate) text_document: TextDocumentIdentifier,
    pub(crate) position: Position,
    #[serde(default)]
    pub(crate) forward: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct StateAtParams {
    #[serde(rename = "textDocument")]
    #[allow(dead_code)]
    pub(crate) text_document: TextDocumentIdentifier,
    pub(crate) position: Position,
}

#[derive(Debug, Serialize)]
pub(crate) struct StateDeclInfo {
    pub(crate) name: String,
    pub(crate) kind: String,
    pub(crate) status: String,
    pub(crate) range: Range,
}

/// One open goal inside `soko/stateAt`'s `goals` array.
#[derive(Debug, Serialize)]
pub(crate) struct StateGoalInfo {
    pub(crate) goal: String,
    /// Semantic runs of `goal` (`docs/design/goal-rendering.md` §2.1).
    pub(crate) goal_runs: Vec<RunInfo>,
    pub(crate) binders: Vec<GoalBinderInfo>,
}

/// `soko/stateAt` response (docs/protocol.md): the goal state at the cursor,
/// plus enough declaration info for the client to label and reveal it.
#[derive(Debug, Serialize)]
pub(crate) struct StateAtResponse {
    pub(crate) version: i32,
    pub(crate) decl: Option<StateDeclInfo>,
    /// `None` = no remaining goals (the proof is closed at this position).
    /// Single-value, equal to `goals[0].goal`; kept for older clients.
    pub(crate) goal: Option<String>,
    /// Semantic runs of `goal` (= `goals[0].goal_runs`); kept for older clients.
    pub(crate) goal_runs: Vec<RunInfo>,
    /// Hypotheses of `goal` (single-value, equal to `goals[0].binders`).
    pub(crate) binders: Vec<GoalBinderInfo>,
    /// Every remaining goal, current goal first (`[]` = closed). Clients that
    /// render goal lists should prefer this over the single `goal`.
    pub(crate) goals: Vec<StateGoalInfo>,
    /// The tactic's range (root state: the declaration's range).
    pub(crate) span: Option<Range>,
    /// Index of the selected per-tactic state; `-1` = root.
    pub(crate) step: i64,
    pub(crate) total: usize,
}

impl StateAtResponse {
    pub(crate) fn empty(version: i32) -> Self {
        Self {
            version,
            decl: None,
            goal: None,
            goal_runs: Vec::new(),
            binders: Vec::new(),
            goals: Vec::new(),
            span: None,
            step: -1,
            total: 0,
        }
    }
}

//! 文档级报告：每个声明的练习状态与 hover 类型。

use super::error::CompileError;
use crate::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeclKind {
    Definition,
    Theorem,
    Axiom,
    Inductive,
    Example,
}

impl DeclKind {
    pub fn as_str(self) -> &'static str {
        match self {
            DeclKind::Definition => "def",
            DeclKind::Theorem => "theorem",
            DeclKind::Axiom => "axiom",
            DeclKind::Inductive => "inductive",
            DeclKind::Example => "example",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeclStatus {
    /// The declaration is an open exercise (`???` in the value position).
    Open,
    /// The declaration passed the complete kernel.
    Checked,
    /// Elaboration or the kernel rejected it.
    Failed,
}

/// One hypothesis already introduced in a partial answer: its written name
/// and the type it carries, rendered as source text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoalBinder {
    pub name: String,
    pub ty: String,
}

/// One `???` inside a constructor spine (multi-hole answer): its span and the
/// expected type the walk recovered for it (best-effort, `None` when the
/// constructor's field type could not be instantiated).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubGoal {
    pub span: Span,
    pub ty: Option<String>,
}

/// One declaration of a `.sokonanoda` document, with its exercise status.
#[derive(Debug, Clone)]
pub struct DeclState {
    pub kind: DeclKind,
    pub name: Option<String>,
    pub span: Span,
    pub status: DeclStatus,
    /// Present when `status == Failed`.
    pub error: Option<CompileError>,
    /// For an open exercise: the remaining goal type, rendered as source text.
    pub goal: Option<String>,
    /// For an open exercise: the hypotheses already introduced by the lambda
    /// binders written so far (the goal view's "context"). Empty when the
    /// answer hole has no lambda prefix yet.
    pub binders: Vec<GoalBinder>,
    /// Index of the command that produced this state (`file.commands[cmd]`),
    /// so incremental sessions and LSP tooling can map states back to source
    /// commands without span guessing.
    pub cmd: usize,
    /// The declaration's universe parameters (`{u}` …). Open exercises carry
    /// them into the goal view / tactic judging so `Sort u` goals can be
    /// judged (the judge synthesizes a declaration with the same universes).
    pub universe: Vec<String>,
    /// Open exercises: every `???` span in the answer (the main hole and any
    /// constructor-spine sub-holes), ordered by offset. Powers `soko/nextHole`
    /// and the goal panel; the server derives them from the walk — clients
    /// never scan text.
    pub holes: Vec<Span>,
    /// Open exercises: expected types for constructor-spine sub-holes,
    /// positionally aligned with `sub_goals` below.
    pub sub_goals: Vec<SubGoal>,
    /// Open exercises: when the remaining goal's head is a constructor with a
    /// known template, a full-application skeleton with auto-filled parameters
    /// and `???` for the proof fields (e.g. `And.intro a b ??? ???`).
    pub refine_template: Option<String>,
}

/// A hover answer for one source span (`span -> inferred type text`).
#[derive(Debug, Clone)]
pub struct HoverType {
    pub span: Span,
    pub text: String,
}

#[derive(Debug, Clone, Default)]
pub struct DocumentReport {
    pub decls: Vec<DeclState>,
    pub hovers: Vec<HoverType>,
    /// Parallel to `hovers`: the command index each hover belongs to, so an
    /// incremental session can cache/re-map hovers per command.
    pub hover_cmds: Vec<usize>,
    /// Parse-level diagnostics live on the `parse` result; this holds
    /// elab/kernel failures that are not attached to a declaration.
    pub errors: Vec<CompileError>,
}

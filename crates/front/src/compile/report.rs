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
    /// The declaration is an open exercise (`sorry` in the value position).
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

/// One `sorry` inside a constructor spine (multi-hole answer): its span and the
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
    /// Open exercises: every `sorry` span in the answer (the main hole and any
    /// constructor-spine sub-holes), ordered by offset. Powers `soko/nextHole`
    /// and the goal panel; the server derives them from the walk — clients
    /// never scan text.
    pub holes: Vec<Span>,
    /// Open exercises: expected types for constructor-spine sub-holes,
    /// positionally aligned with `sub_goals` below.
    pub sub_goals: Vec<SubGoal>,
    /// Kernel-rendered declared type（声明签名文本，如
    /// `axiom And.intro : forall (a : Prop), …`）。声明名 hover 与练习
    /// 面板显示用；Open 练习来自 elaborated type。
    pub ty_text: Option<String>,
    /// Open exercises: when the remaining goal's head is a constructor with a
    /// known template, a full-application skeleton with auto-filled parameters
    /// and `sorry` for the proof fields (e.g. `And.intro a b sorry sorry`).
    pub refine_template: Option<String>,
    /// The declaration's hint ladder, authored in the canvas as
    /// `-- soko:hint <text>` comment directives attached to this declaration
    /// (`compile::hints::attach_hints`). Empty when the source has no hints
    /// for it or when the report was produced without a source text
    /// (e.g. `compile_fol` on a pre-parsed AST).
    pub hints: Vec<String>,
}

/// Where a name use resolves to, together with the definition's source span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedTarget {
    /// A local binder; the span is the binder's own source span.
    Binder(Span),
    /// A top-level declaration of the same file; the span is the defining
    /// command's span.
    Declaration { name: String, span: Span },
}

impl ResolvedTarget {
    /// The definition's source span (go-to-definition / highlight target).
    pub fn span(&self) -> Span {
        match self {
            ResolvedTarget::Binder(span) => *span,
            ResolvedTarget::Declaration { span, .. } => *span,
        }
    }
}

/// A hover answer for one source span (`span -> inferred type text`).
#[derive(Debug, Clone)]
pub struct HoverType {
    pub span: Span,
    pub text: String,
    /// In-scope binder names at this sub-expression, outermost first, for
    /// scoped completion. Anonymous binders (`A -> B`) carry an empty name.
    pub scope_names: Vec<String>,
    /// When this span is a name use point: where the name is defined
    /// (`None` for prelude names and unresolved idents).
    pub resolution: Option<ResolvedTarget>,
    /// This row is a lambda/forall **binder declaration** (`name : ty`): the
    /// editor renders the declaration itself, not `expr : type`.
    pub binder: bool,
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

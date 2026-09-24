//! 文档级报告：每个声明的练习状态与 hover 类型。

use super::error::CompileError;
use super::warning::CompileWarning;
use crate::Span;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoalBinder {
    pub name: String,
    pub ty: String,
}

/// One `sorry` inside a constructor spine (multi-hole answer): its span and the
/// expected type the walk recovered for it (best-effort, `None` when the
/// constructor's field type could not be instantiated).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubGoal {
    pub span: Span,
    pub ty: Option<String>,
}

/// One open goal after a tactic step: its type and the hypotheses in scope
/// for it (a different sub-goal may carry a different context).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ByGoalState {
    pub ty: String,
    pub binders: Vec<GoalBinder>,
}

/// One tactic step of a `by` block, recorded by the engine as the state
/// **after** that tactic executed (`docs/design/by-tactics.md` §6): the
/// tactic's source span and **every** remaining goal (the current goal first;
/// empty when every goal is closed). In the I8 session snapshot this is the
/// editor's "goals at cursor" data (`soko/stateAt`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ByStepState {
    pub span: Span,
    pub goals: Vec<ByGoalState>,
}

/// One declaration of a `.sokonanoda` document, with its exercise status.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// **声明的值**（`:=` 之后那个东西，T-D52 / 用户第 8 条反馈）。
    ///
    /// 只有 `def`/`opaque` 有；`theorem`/`axiom`/`inductive` 是 `None`
    /// （用户要的是 `def` 的"本质"——证明是另一件事）。
    /// 与 `ty_text` 同一形状：内核 pp 出来再过一遍**线 C 的记法折叠**。
    pub val_text: Option<String>,
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
    /// Per-tactic states of a `:= by …` value, in tactic order (each state is
    /// recorded **after** its tactic ran). Empty for declarations without a
    /// `by` block. Powers `soko/stateAt` (cursor goal view).
    pub by_steps: Vec<ByStepState>,
}

/// Where a name use resolves to, together with the definition's source span.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResolvedTarget {
    /// A local binder; the span is the binder's own source span.
    Binder(Span),
    /// A top-level declaration of the same file; the span is the defining
    /// command's span.
    Declaration { name: String, span: Span },
    /// **记法符号**（T-D15）：`span` 是**使用处那个符号自己**的 span
    /// （T-D14 的 `Expr::Notation.symbol_span`），不是整段节点。
    ///
    /// 为什么需要它：以前记法行的 `resolution` 是 `None`，于是"光标在符号上"
    /// 落到两个回退里，会误命中**外层 binder**（T-D30 的守卫是权宜之计）。
    /// 有了这个变体，符号就是**一等目标**。
    ///
    /// `module` 是**声明记法的模块名**（跨模块跳转用）；本文件内声明时为 `None`
    /// ——elaborator 拿不到记法表（它在 parser 手里），所以这里先留 `None`，
    /// **声明点**由 LSP 侧的闭包记法表解析（`QueryDoc::notation_at`，T-D10/T-D16）。
    Notation {
        symbol: String,
        span: Span,
        module: Option<String>,
    },
}

impl ResolvedTarget {
    /// The definition's source span (go-to-definition / highlight target).
    pub fn span(&self) -> Span {
        match self {
            ResolvedTarget::Binder(span) => *span,
            ResolvedTarget::Declaration { span, .. } => *span,
            ResolvedTarget::Notation { span, .. } => *span,
        }
    }
}

/// A hover answer for one source span (`span -> inferred type text`).
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// A `#check` command's result: the checked expression's span and the
/// kernel-pretty-printed type. Carried on the report so the editor can show
/// it persistently (inlay hint), like Lean's Infoview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckInfo {
    pub span: Span,
    pub text: String,
    /// 产生它的命令下标（`file.commands[cmd]`）。跨文件编译时按它归因；
    /// 不进序列化（缓存条目只服务单文件，协议形状保持不变）。
    #[serde(skip)]
    pub cmd: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DocumentReport {
    pub decls: Vec<DeclState>,
    pub hovers: Vec<HoverType>,
    /// Parallel to `hovers`: the command index each hover belongs to, so an
    /// incremental session can cache/re-map hovers per command.
    pub hover_cmds: Vec<usize>,
    /// Parse-level diagnostics live on the `parse` result; this holds
    /// elab/kernel failures that are not attached to a declaration.
    pub errors: Vec<CompileError>,
    /// `#check` results, in source order.
    pub checks: Vec<CheckInfo>,
    /// Syntax-level warnings (e.g. a declaration colliding with a
    /// kernel-defined name).
    /// Independent of declaration status; the LSP renders these as
    /// `DiagnosticSeverity::WARNING`.
    pub warnings: Vec<CompileWarning>,
}

//! Compile a parsed `.sokonanoda` file into kernel declarations and run the
//! complete sokonanoda kernel over them.

use crate::{BinderKind, Command, CtorDecl, Expr, FolFile, RecDecl, SortKind, Span};
use sokonanoda::builder::EnvBuilder;
use sokonanoda::env::{
    ConstructorData, Declar, DeclarInfo, EnvLimit, RecRule, RecursorData, ReducibilityHint,
};
use sokonanoda::expr::BinderStyle;
use sokonanoda::util::{Config, ExprPtr, LevelPtr, NamePtr};
use std::collections::HashMap;
use std::sync::Arc;

type UnivMap<'a> = HashMap<String, LevelPtr<'a>>;

// ---------------------------------------------------------------------------
// Errors: stable stage + kind, with teaching hints
// ---------------------------------------------------------------------------

/// Which pipeline stage produced an error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompileStage {
    Elab,
    Kernel,
}

impl CompileStage {
    pub fn code(self) -> &'static str {
        match self {
            CompileStage::Elab => "elab",
            CompileStage::Kernel => "kernel",
        }
    }
}

/// Stable, fine-grained error kind below the stage level. Every kind maps to a
/// machine code and a first teaching hint (docs/design-infrastructure.md F1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    ElabUnknownIdentifier,
    ElabUnknownConstant,
    ElabUnknownUniverseLevel,
    ElabUniverseArity,
    ElabUntypedBinder,
    ElabHoleMisplaced,
    ElabDuplicateDeclaration,
    ElabTooManyBinders,
    ElabNatLiteralDisabled,
    ElabInvalidNatLiteral,
    ElabTooManyCtorFields,
    ElabUnknownCtorForIota,
    KernelRejected,
    KernelInternal,
}

impl ErrorKind {
    pub fn stage(self) -> CompileStage {
        use ErrorKind::*;
        match self {
            ElabUnknownIdentifier
            | ElabUnknownConstant
            | ElabUnknownUniverseLevel
            | ElabUniverseArity
            | ElabUntypedBinder
            | ElabHoleMisplaced
            | ElabDuplicateDeclaration
            | ElabTooManyBinders
            | ElabNatLiteralDisabled
            | ElabInvalidNatLiteral
            | ElabTooManyCtorFields
            | ElabUnknownCtorForIota => CompileStage::Elab,
            KernelRejected | KernelInternal => CompileStage::Kernel,
        }
    }

    pub fn code(self) -> &'static str {
        use ErrorKind::*;
        match self {
            ElabUnknownIdentifier => "elab-unknown-identifier",
            ElabUnknownConstant => "elab-unknown-constant",
            ElabUnknownUniverseLevel => "elab-unknown-universe-level",
            ElabUniverseArity => "elab-universe-arity",
            ElabUntypedBinder => "elab-untyped-binder",
            ElabHoleMisplaced => "elab-hole-misplaced",
            ElabDuplicateDeclaration => "elab-duplicate-declaration",
            ElabTooManyBinders => "elab-too-many-binders",
            ElabNatLiteralDisabled => "elab-nat-literal-disabled",
            ElabInvalidNatLiteral => "elab-invalid-nat-literal",
            ElabTooManyCtorFields => "elab-too-many-ctor-fields",
            ElabUnknownCtorForIota => "elab-unknown-ctor-for-iota",
            KernelRejected => "kernel-rejected",
            KernelInternal => "kernel-internal",
        }
    }

    /// First-version teaching hint. Learners read these next to the squiggle;
    /// keep them short, concrete and actionable.
    pub fn hint(self) -> &'static str {
        use ErrorKind::*;
        match self {
            ElabUnknownIdentifier => {
                "这个名字还没有被定义。检查拼写，或确认它出现在你前面的某个声明里（练习要在解决之后才能被后面的代码引用）。"
            }
            ElabUnknownConstant => {
                "这里引用了一个不存在的常量。如果它带宇宙参数，请先定义它。"
            }
            ElabUnknownUniverseLevel => {
                "这个宇宙层级变量没有在当前声明里声明。用 {u} 声明它，例如 def id {u} : ...。"
            }
            ElabUniverseArity => {
                "宇宙参数个数不对。这个常量声明了几个宇宙参数，就要给几个，例如 id.{u, v}。"
            }
            ElabUntypedBinder => {
                "这个 binder 缺少类型标注。教学版本要求写全类型，例如 fun (x : Nat) => x。"
            }
            ElabHoleMisplaced => {
                "??? 只能出现在声明的值（答案区）位置，例如 example : T := ???。"
            }
            ElabDuplicateDeclaration => {
                "这个名字已经定义过了。Lean 里每个名字只能声明一次；换一个名字，或删掉前面的声明。"
            }
            ElabTooManyBinders => {
                "嵌套的 binder 太多，超出了内核能表示的深度。把大表达式拆成几个小定义。"
            }
            ElabNatLiteralDisabled => {
                "数字字面量没有被启用。这个版本默认打开 Nat 扩展，如遇到此错误请联系工具作者。"
            }
            ElabInvalidNatLiteral => {
                "这不是一个合法的自然数字面量。"
            }
            ElabTooManyCtorFields => {
                "构造子的字段太多了。"
            }
            ElabUnknownCtorForIota => {
                "iota 规则引用了一个不存在的构造子。检查构造子名字是否与 ctor 声明一致。"
            }
            KernelRejected => {
                "内核判定不成立：类型不匹配或证明项不完整。先对比期望类型与你的值的形状；最常见的错误是两边结构不同（例如期望 a -> a，却写成了返回 Nat 的项）。"
            }
            KernelInternal => {
                "内核内部错误（这不是你的代码问题）。请把这段代码反馈给工具作者。"
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompileError {
    pub message: String,
    pub span: Span,
    pub kind: ErrorKind,
}

impl CompileError {
    fn new(kind: ErrorKind, message: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            span,
            kind,
        }
    }

    fn elab(kind: ErrorKind, message: impl Into<String>, span: Span) -> Self {
        Self::new(kind, message, span)
    }

    fn kernel(kind: ErrorKind, message: impl Into<String>, span: Span) -> Self {
        Self::new(kind, message, span)
    }

    pub fn stage(&self) -> CompileStage {
        self.kind.stage()
    }

    /// Stable machine code (used by `--json` and the protocol).
    pub fn code(&self) -> &'static str {
        self.kind.code()
    }

    /// First teaching hint for this error.
    pub fn hint(&self) -> &'static str {
        self.kind.hint()
    }
}

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum CheckEvent {
    DeclarationChecked { name: String },
    ExampleChecked,
    TypeChecked { text: String, span: Span },
    Reduced { text: String, span: Span },
    Printed { name: String, text: String },
    ExerciseOpen { name: Option<String> },
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct CompileOutput {
    pub events: Vec<CheckEvent>,
    pub errors: Vec<CompileError>,
}

impl CompileOutput {
    pub fn ok(&self) -> bool {
        self.errors.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Per-declaration states and hover types (the "document report")
// ---------------------------------------------------------------------------

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

/// One declaration of a `.sokonanoda` document, with its exercise status.
#[derive(Debug, Clone)]
pub struct DeclState {
    pub kind: DeclKind,
    pub name: Option<String>,
    pub span: Span,
    pub status: DeclStatus,
    /// Present when `status == Failed`.
    pub error: Option<CompileError>,
    /// For an open exercise: the declared goal type, rendered as source text.
    pub goal: Option<String>,
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
    /// Parse-level diagnostics live on the `parse` result; this holds
    /// elab/kernel failures that are not attached to a declaration.
    pub errors: Vec<CompileError>,
}

// ---------------------------------------------------------------------------
// Elaboration scope + hover recording
// ---------------------------------------------------------------------------

struct ElabScope<'a> {
    names: Vec<String>,
    tys: Vec<ExprPtr<'a>>,
}

impl<'a> ElabScope<'a> {
    fn new() -> Self {
        Self {
            names: Vec::new(),
            tys: Vec::new(),
        }
    }
    fn len(&self) -> usize {
        self.names.len()
    }
    fn truncate(&mut self, len: usize) {
        self.names.truncate(len);
        self.tys.truncate(len);
    }
    fn push(&mut self, name: String, ty: ExprPtr<'a>) {
        self.names.push(name);
        self.tys.push(ty);
    }
}

/// One recorded sub-expression during elaboration, with the binder scope it
/// lives under (outermost first). Used to answer editor hovers.
struct HoverNode<'a> {
    span: Span,
    expr: ExprPtr<'a>,
    #[allow(dead_code)] // reserved for named display of open types
    scope_names: Vec<String>,
    scope_tys: Vec<ExprPtr<'a>>,
}

fn record_hover<'a>(
    hovers: &mut Vec<HoverNode<'a>>,
    scope: &ElabScope<'a>,
    span: Span,
    expr: ExprPtr<'a>,
) {
    hovers.push(HoverNode {
        span,
        expr,
        scope_names: scope.names.clone(),
        scope_tys: scope.tys.clone(),
    });
}

// ---------------------------------------------------------------------------
// Pending operations, run against the finished environment
// ---------------------------------------------------------------------------

enum PendingOp<'a> {
    Decl {
        name: Option<String>,
        kind: DeclKind,
        declar: Declar<'a>,
        span: Span,
    },
    OpenExercise {
        name: Option<String>,
        kind: DeclKind,
        goal: Option<String>,
        span: Span,
    },
    Check {
        expr: ExprPtr<'a>,
        env_at: usize,
        span: Span,
    },
    Reduce {
        expr: ExprPtr<'a>,
        env_at: usize,
        span: Span,
    },
    Print {
        name: String,
        ptr: NamePtr<'a>,
        span: Span,
    },
}

struct CmdHover<'a> {
    env_at: usize,
    nodes: Vec<HoverNode<'a>>,
}

/// Compile and kernel-check a whole file in one arena session, returning the
/// batch view (events + errors) that the CLI and tests consume.
pub fn compile_fol(file: &FolFile) -> CompileOutput {
    run(file, false).0
}

/// Compile a file and return the detailed document report (per-declaration
/// states, diagnostics, hover types) that the LSP and agents consume.
pub fn check_document(file: &FolFile) -> DocumentReport {
    run(file, true).1
}

fn run(file: &FolFile, collect: bool) -> (CompileOutput, DocumentReport) {
    let arena = stumpalo::Arena::new();
    let mut builder = EnvBuilder::new(arena.as_arena_ref(), Config::default());
    let mut known_universes: HashMap<String, Vec<String>> = HashMap::new();
    let explicit_nat = file
        .commands
        .iter()
        .any(|command| matches!(command, Command::InductiveBlock { name, .. } if name == "Nat"));
    if !explicit_nat {
        install_prelude(&mut builder);
        for builtin in ["Nat", "Nat.zero", "Nat.succ", "Nat.add"] {
            known_universes.insert(builtin.to_string(), Vec::new());
        }
    }
    let no_universe: UnivMap = UnivMap::new();

    let mut out = CompileOutput::default();
    let mut report = DocumentReport::default();
    let mut ops: Vec<PendingOp<'_>> = Vec::new();
    let mut cmd_hovers: Vec<CmdHover<'_>> = Vec::new();
    let mut decl_states: Vec<DeclState> = Vec::new();
    let mut example_idx = 0usize;

    for command in &file.commands {
        let env_before = builder.declaration_count();
        match command {
            Command::Def {
                name,
                universe,
                ty,
                val,
                span,
            } => {
                if matches!(val, Expr::Hole { .. }) {
                    ops.push(PendingOp::OpenExercise {
                        name: Some(name.clone()),
                        kind: DeclKind::Definition,
                        goal: Some(render_expr(ty)),
                        span: *span,
                    });
                    continue;
                }
                let mut hovers = Vec::new();
                match build_def(
                    &mut builder,
                    name,
                    universe,
                    ty,
                    val,
                    &known_universes,
                    &mut hovers,
                ) {
                    Ok(decl) => {
                        let name_owned = name.clone();
                        if let Err(e) = builder.add_declar(decl.clone()) {
                            let err =
                                CompileError::elab(ErrorKind::ElabDuplicateDeclaration, e, *span);
                            out.errors.push(err.clone());
                            decl_states.push(failed_state(
                                DeclKind::Definition,
                                Some(name_owned.clone()),
                                *span,
                                err,
                            ));
                            continue;
                        }
                        known_universes.insert(name_owned.clone(), universe.clone());
                        let env_after = builder.declaration_count();
                        ops.push(PendingOp::Decl {
                            name: Some(name_owned),
                            kind: DeclKind::Definition,
                            declar: decl,
                            span: *span,
                        });
                        cmd_hovers.push(CmdHover {
                            env_at: env_after,
                            nodes: hovers,
                        });
                    }
                    Err(e) => {
                        out.errors.push(e.clone());
                        decl_states.push(failed_state(
                            DeclKind::Definition,
                            Some(name.clone()),
                            *span,
                            e,
                        ));
                    }
                }
            }
            Command::Theorem {
                name,
                universe,
                ty,
                val,
                span,
            } => {
                if matches!(val, Expr::Hole { .. }) {
                    ops.push(PendingOp::OpenExercise {
                        name: Some(name.clone()),
                        kind: DeclKind::Theorem,
                        goal: Some(render_expr(ty)),
                        span: *span,
                    });
                    continue;
                }
                let mut hovers = Vec::new();
                match build_theorem(
                    &mut builder,
                    name,
                    universe,
                    ty,
                    val,
                    &known_universes,
                    &mut hovers,
                ) {
                    Ok(decl) => {
                        let name_owned = name.clone();
                        if let Err(e) = builder.add_declar(decl.clone()) {
                            let err =
                                CompileError::elab(ErrorKind::ElabDuplicateDeclaration, e, *span);
                            out.errors.push(err.clone());
                            decl_states.push(failed_state(
                                DeclKind::Theorem,
                                Some(name_owned.clone()),
                                *span,
                                err,
                            ));
                            continue;
                        }
                        known_universes.insert(name_owned.clone(), universe.clone());
                        let env_after = builder.declaration_count();
                        ops.push(PendingOp::Decl {
                            name: Some(name_owned),
                            kind: DeclKind::Theorem,
                            declar: decl,
                            span: *span,
                        });
                        cmd_hovers.push(CmdHover {
                            env_at: env_after,
                            nodes: hovers,
                        });
                    }
                    Err(e) => {
                        out.errors.push(e.clone());
                        decl_states.push(failed_state(
                            DeclKind::Theorem,
                            Some(name.clone()),
                            *span,
                            e,
                        ));
                    }
                }
            }
            Command::Axiom {
                name,
                universe,
                ty,
                span,
            } => {
                let mut hovers = Vec::new();
                match build_axiom(
                    &mut builder,
                    name,
                    universe,
                    ty,
                    &known_universes,
                    &mut hovers,
                ) {
                    Ok(decl) => {
                        let name_owned = name.clone();
                        if let Err(e) = builder.add_declar(decl.clone()) {
                            let err =
                                CompileError::elab(ErrorKind::ElabDuplicateDeclaration, e, *span);
                            out.errors.push(err.clone());
                            decl_states.push(failed_state(
                                DeclKind::Axiom,
                                Some(name_owned.clone()),
                                *span,
                                err,
                            ));
                            continue;
                        }
                        known_universes.insert(name_owned.clone(), universe.clone());
                        let env_after = builder.declaration_count();
                        ops.push(PendingOp::Decl {
                            name: Some(name_owned),
                            kind: DeclKind::Axiom,
                            declar: decl,
                            span: *span,
                        });
                        cmd_hovers.push(CmdHover {
                            env_at: env_after,
                            nodes: hovers,
                        });
                    }
                    Err(e) => {
                        out.errors.push(e.clone());
                        decl_states.push(failed_state(
                            DeclKind::Axiom,
                            Some(name.clone()),
                            *span,
                            e,
                        ));
                    }
                }
            }
            Command::Example { ty, val, span } => {
                if matches!(val, Expr::Hole { .. }) {
                    ops.push(PendingOp::OpenExercise {
                        name: None,
                        kind: DeclKind::Example,
                        goal: Some(render_expr(ty)),
                        span: *span,
                    });
                    continue;
                }
                example_idx += 1;
                let internal_name = format!("_example_{example_idx}");
                let mut hovers = Vec::new();
                match build_example(
                    &mut builder,
                    &internal_name,
                    ty,
                    val,
                    &known_universes,
                    &mut hovers,
                ) {
                    Ok(decl) => {
                        if let Err(e) = builder.add_declar(decl.clone()) {
                            let err =
                                CompileError::elab(ErrorKind::ElabDuplicateDeclaration, e, *span);
                            out.errors.push(err.clone());
                            decl_states.push(failed_state(DeclKind::Example, None, *span, err));
                            continue;
                        }
                        let env_after = builder.declaration_count();
                        ops.push(PendingOp::Decl {
                            name: None,
                            kind: DeclKind::Example,
                            declar: decl,
                            span: *span,
                        });
                        cmd_hovers.push(CmdHover {
                            env_at: env_after,
                            nodes: hovers,
                        });
                    }
                    Err(e) => {
                        out.errors.push(e.clone());
                        decl_states.push(failed_state(DeclKind::Example, None, *span, e));
                    }
                }
            }
            Command::InductiveBlock {
                name,
                ty,
                constructors,
                recursor,
                iota_rules,
                span,
            } => {
                let mut hovers = Vec::new();
                match install_inductive_block(
                    &mut builder,
                    &mut known_universes,
                    name,
                    ty,
                    constructors,
                    recursor.as_ref(),
                    iota_rules,
                    &mut hovers,
                ) {
                    Ok(()) => {
                        decl_states.push(DeclState {
                            kind: DeclKind::Inductive,
                            name: Some(name.clone()),
                            span: *span,
                            status: DeclStatus::Checked,
                            error: None,
                            goal: None,
                        });
                        cmd_hovers.push(CmdHover {
                            env_at: builder.declaration_count(),
                            nodes: hovers,
                        });
                    }
                    Err(e) => {
                        out.errors.push(e.clone());
                        decl_states.push(failed_state(
                            DeclKind::Inductive,
                            Some(name.clone()),
                            *span,
                            e,
                        ));
                    }
                }
            }
            Command::Check { expr, span: _ } => {
                let mut hovers = Vec::new();
                match elab_expr(
                    &mut builder,
                    expr,
                    &mut ElabScope::new(),
                    &no_universe,
                    &known_universes,
                    &mut hovers,
                ) {
                    Ok(e) => {
                        ops.push(PendingOp::Check {
                            expr: e,
                            env_at: env_before,
                            span: expr.span(),
                        });
                        cmd_hovers.push(CmdHover {
                            env_at: env_before,
                            nodes: hovers,
                        });
                    }
                    Err(e) => out.errors.push(e),
                }
            }
            Command::Reduce { expr, span: _ } => {
                let mut hovers = Vec::new();
                match elab_expr(
                    &mut builder,
                    expr,
                    &mut ElabScope::new(),
                    &no_universe,
                    &known_universes,
                    &mut hovers,
                ) {
                    Ok(e) => {
                        ops.push(PendingOp::Reduce {
                            expr: e,
                            env_at: env_before,
                            span: expr.span(),
                        });
                        cmd_hovers.push(CmdHover {
                            env_at: env_before,
                            nodes: hovers,
                        });
                    }
                    Err(e) => out.errors.push(e),
                }
            }
            Command::Print { name, span } => {
                let ptr = builder.name_from_str(name);
                ops.push(PendingOp::Print {
                    name: name.clone(),
                    ptr,
                    span: *span,
                });
            }
        }
    }

    let mut env = builder.finish();
    // Print proof terms as terms instead of suppressing them to `_`; the
    // suppression path would try to infer types of open binder bodies.
    env.config.pp_options.proofs = true;

    for op in ops {
        match op {
            PendingOp::OpenExercise {
                name,
                kind,
                goal,
                span,
            } => {
                out.events
                    .push(CheckEvent::ExerciseOpen { name: name.clone() });
                decl_states.push(DeclState {
                    kind,
                    name,
                    span,
                    status: DeclStatus::Open,
                    error: None,
                    goal,
                });
            }
            PendingOp::Decl {
                name,
                kind,
                declar,
                span,
            } => match env.try_check_declar(&declar) {
                Ok(()) => {
                    match kind {
                        DeclKind::Example => out.events.push(CheckEvent::ExampleChecked),
                        _ => {
                            if let Some(n) = &name {
                                out.events
                                    .push(CheckEvent::DeclarationChecked { name: n.clone() });
                            } else {
                                out.events.push(CheckEvent::ExampleChecked);
                            }
                        }
                    }
                    decl_states.push(DeclState {
                        kind,
                        name,
                        span,
                        status: DeclStatus::Checked,
                        error: None,
                        goal: None,
                    });
                }
                Err(e) => {
                    let msg = format!("{e}");
                    let err = if msg.contains("kernel error") || msg.contains("kernel error:") {
                        CompileError::kernel(ErrorKind::KernelInternal, msg, span)
                    } else {
                        CompileError::kernel(ErrorKind::KernelRejected, msg, span)
                    };
                    out.errors.push(err.clone());
                    decl_states.push(failed_state(kind, name, span, err));
                }
            },
            PendingOp::Check { expr, env_at, span } => {
                env.with_tc(EnvLimit::ByIndex(env_at), |tc| {
                    let ty = tc.infer_closed_type(expr);
                    let text = tc.with_pp(|pp| pp.pp_expr(ty));
                    out.events.push(CheckEvent::TypeChecked { text, span });
                });
            }
            PendingOp::Reduce { expr, env_at, span } => {
                env.with_tc(EnvLimit::ByIndex(env_at), |tc| {
                    let reduced = tc.reduce_closed(expr);
                    let text = tc.with_pp(|pp| pp.pp_expr(reduced));
                    out.events.push(CheckEvent::Reduced { text, span });
                });
            }
            PendingOp::Print { name, ptr, span } => {
                let printed = env.with_pp(|pp| pp.pp_declar(ptr));
                match printed {
                    Some(text) => out.events.push(CheckEvent::Printed { name, text }),
                    None => out.errors.push(CompileError::elab(
                        ErrorKind::ElabUnknownIdentifier,
                        format!("unknown declaration `{name}`"),
                        span,
                    )),
                }
            }
        }
    }

    if collect {
        // Open/failed states are recorded during the command walk while
        // checked states come from the kernel phase; keep source order.
        let mut states = decl_states;
        states.sort_by_key(|d| d.span.start.offset);
        report.decls = states;
        report.errors = out.errors.clone();
        resolve_hovers(&env, cmd_hovers, &mut report.hovers);
    }
    (out, report)
}

fn failed_state(
    kind: DeclKind,
    name: Option<String>,
    span: Span,
    error: CompileError,
) -> DeclState {
    DeclState {
        kind,
        name,
        span,
        status: DeclStatus::Failed,
        error: Some(error),
        goal: None,
    }
}

/// Infer a type per recorded sub-expression (in its binder scope) and render
/// it as text. Panics (kernel rejection on intermediate sub-terms) are caught
/// per node so one bad sub-term cannot kill the hover map.
fn resolve_hovers(
    env: &sokonanoda::util::ExportFile<'_>,
    cmd_hovers: Vec<CmdHover<'_>>,
    out: &mut Vec<HoverType>,
) {
    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    for cmd in cmd_hovers {
        for node in cmd.nodes {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                env.with_tc(EnvLimit::ByIndex(cmd.env_at), |tc| {
                    let ty = tc.infer_under_binders(&node.scope_tys, node.expr);
                    tc.with_pp(|pp| pp.pp_expr(ty))
                })
            }));
            let Ok(text) = result else { continue };
            if !text.is_empty() {
                out.push(HoverType {
                    span: node.span,
                    text,
                });
            }
        }
    }
    std::panic::set_hook(previous_hook);
}

/// Trusted built-in base declarations. These are never re-checked by the
/// kernel: they are the axioms/inductive spine that the teaching grammar is
/// built on. The kernel's native Nat reduction is enabled purely by the
/// matching declaration names.
fn install_prelude(builder: &mut EnvBuilder<'_>) {
    let anon = builder.anonymous();
    let empty = builder.alloc_levels_slice(&[]);
    let type_level = builder.succ(builder.zero());
    let type_sort = builder.mk_sort(type_level);

    let nat = builder.name_from_str("Nat");
    let nat_type = builder.mk_const(nat, empty);
    builder
        .add_inductive(
            DeclarInfo {
                name: nat,
                uparams: empty,
                ty: type_sort,
            },
            false,
            0,
            0,
            Arc::from([nat]),
            Arc::from([]),
        )
        .expect("builtin Nat already present");

    add_axiom(builder, "Nat.zero", nat_type);

    let succ_arrow = builder.mk_pi(anon, BinderStyle::Default, nat_type, nat_type);
    let succ_name = builder.name_from_str("Nat.succ");
    let succ_levels = builder.alloc_levels_slice(&[]);
    let succ_self = builder.mk_const(succ_name, succ_levels);
    add_definition(builder, "Nat.succ", succ_arrow, succ_self);

    let inner_arrow = builder.mk_pi(anon, BinderStyle::Default, nat_type, nat_type);
    let add_arrow = builder.mk_pi(anon, BinderStyle::Default, nat_type, inner_arrow);
    let add_name = builder.name_from_str("Nat.add");
    let add_levels = builder.alloc_levels_slice(&[]);
    let add_self = builder.mk_const(add_name, add_levels);
    add_definition(builder, "Nat.add", add_arrow, add_self);
}

fn add_axiom<'a>(builder: &mut EnvBuilder<'a>, name: &str, ty: ExprPtr<'a>) {
    let name = builder.name_from_str(name);
    let info = DeclarInfo {
        name,
        uparams: builder.alloc_levels_slice(&[]),
        ty,
    };
    builder
        .add_declar(Declar::Axiom { info })
        .expect("duplicate builtin axiom");
}

fn add_definition<'a>(builder: &mut EnvBuilder<'a>, name: &str, ty: ExprPtr<'a>, val: ExprPtr<'a>) {
    let name = builder.name_from_str(name);
    let info = DeclarInfo {
        name,
        uparams: builder.alloc_levels_slice(&[]),
        ty,
    };
    builder
        .add_declar(Declar::Definition {
            info,
            val,
            hint: ReducibilityHint::Regular(0),
        })
        .expect("duplicate builtin definition");
}

#[allow(clippy::too_many_arguments)]
fn install_inductive_block<'a>(
    builder: &mut EnvBuilder<'a>,
    known: &mut HashMap<String, Vec<String>>,
    name: &str,
    ty: &Expr,
    constructors: &[CtorDecl],
    recursor: Option<&RecDecl>,
    iota_rules: &[crate::IotaRule],
    hovers: &mut Vec<HoverNode<'a>>,
) -> Result<(), CompileError> {
    let empty: UnivMap = UnivMap::new();
    let ty = elab_expr(builder, ty, &mut ElabScope::new(), &empty, known, hovers)?;
    let ind_name = builder.name_from_str(name);
    let ctor_names: Vec<NamePtr<'a>> = constructors
        .iter()
        .map(|c| builder.name_from_str(&c.name))
        .collect();
    let no_uparams = builder.alloc_levels_slice(&[]);
    builder
        .add_inductive(
            DeclarInfo {
                name: ind_name,
                uparams: no_uparams,
                ty,
            },
            true,
            0,
            0,
            Arc::from([ind_name]),
            Arc::from(ctor_names.clone()),
        )
        .map_err(|e| CompileError::elab(ErrorKind::ElabDuplicateDeclaration, e, Span::default()))?;
    known.insert(name.to_string(), Vec::new());

    for (idx, ctor) in constructors.iter().enumerate() {
        let ctor_ty = Expr::Forall {
            binders: ctor.binders.clone(),
            body: Box::new(ctor.result.clone()),
            span: ctor.span,
        };
        let ctor_ty = elab_expr(
            builder,
            &ctor_ty,
            &mut ElabScope::new(),
            &empty,
            known,
            hovers,
        )?;
        let ctor_name = ctor_names[idx];
        let no_uparams = builder.alloc_levels_slice(&[]);
        let num_fields = u16::try_from(ctor.binders.len()).map_err(|_| {
            CompileError::elab(
                ErrorKind::ElabTooManyCtorFields,
                "too many constructor fields",
                ctor.span,
            )
        })?;
        builder
            .add_declar(Declar::Constructor(ConstructorData {
                info: DeclarInfo {
                    name: ctor_name,
                    uparams: no_uparams,
                    ty: ctor_ty,
                },
                inductive_name: ind_name,
                ctor_idx: idx as u16,
                num_params: 0,
                num_fields,
            }))
            .map_err(|e| CompileError::elab(ErrorKind::ElabDuplicateDeclaration, e, ctor.span))?;
        known.insert(ctor.name.clone(), Vec::new());
    }

    if let Some(rec) = recursor {
        let univ = make_univ_map(builder, &rec.universe);
        let rec_ty = elab_expr(
            builder,
            &rec.ty,
            &mut ElabScope::new(),
            &univ,
            known,
            hovers,
        )?;
        let rec_name = builder.name_from_str(&rec.name);
        let known_rec_universes = rec.universe.clone();
        known.insert(rec.name.clone(), known_rec_universes.clone());

        let mut rules = Vec::with_capacity(iota_rules.len());
        for rule in iota_rules {
            let ctor_idx = constructors
                .iter()
                .position(|c| c.name == rule.ctor_name)
                .ok_or_else(|| {
                    CompileError::elab(
                        ErrorKind::ElabUnknownCtorForIota,
                        format!(
                            "iota rule refers to unknown constructor `{}`",
                            rule.ctor_name
                        ),
                        rule.span,
                    )
                })?;
            let ctor_name = ctor_names[ctor_idx];
            let val = elab_expr(
                builder,
                &rule.val,
                &mut ElabScope::new(),
                &univ,
                known,
                hovers,
            )?;
            rules.push(RecRule {
                ctor_name,
                ctor_telescope_size_wo_params: constructors[ctor_idx].binders.len() as u16,
                val,
            });
        }
        let info = DeclarInfo {
            name: rec_name,
            uparams: collect_uparams(builder, &univ, &known_rec_universes),
            ty: rec_ty,
        };
        builder
            .add_declar(Declar::Recursor(RecursorData {
                info,
                all_inductives: Arc::from([ind_name]),
                num_params: 0,
                num_indices: 0,
                num_motives: 1,
                num_minors: constructors.len() as u16,
                rec_rules: Arc::from(rules),
                is_k: false,
            }))
            .map_err(|e| CompileError::elab(ErrorKind::ElabDuplicateDeclaration, e, rec.span))?;
    }
    Ok(())
}

fn build_def<'a>(
    builder: &mut EnvBuilder<'a>,
    name: &str,
    universe: &[String],
    ty: &Expr,
    val: &Expr,
    known: &HashMap<String, Vec<String>>,
    hovers: &mut Vec<HoverNode<'a>>,
) -> Result<Declar<'a>, CompileError> {
    let mut scope = ElabScope::new();
    let univ = make_univ_map(builder, universe);
    let ty = elab_expr(builder, ty, &mut scope, &univ, known, hovers)?;
    let val = elab_expr(builder, val, &mut scope, &univ, known, hovers)?;
    let name = builder.name_from_str(name);
    let uparams = collect_uparams(builder, &univ, universe);
    Ok(Declar::Definition {
        info: DeclarInfo { name, uparams, ty },
        val,
        hint: ReducibilityHint::Regular(0),
    })
}

fn build_theorem<'a>(
    builder: &mut EnvBuilder<'a>,
    name: &str,
    universe: &[String],
    ty: &Expr,
    val: &Expr,
    known: &HashMap<String, Vec<String>>,
    hovers: &mut Vec<HoverNode<'a>>,
) -> Result<Declar<'a>, CompileError> {
    let mut scope = ElabScope::new();
    let univ = make_univ_map(builder, universe);
    let ty = elab_expr(builder, ty, &mut scope, &univ, known, hovers)?;
    let val = elab_expr(builder, val, &mut scope, &univ, known, hovers)?;
    let name = builder.name_from_str(name);
    let uparams = collect_uparams(builder, &univ, universe);
    Ok(Declar::Theorem {
        info: DeclarInfo { name, uparams, ty },
        val,
    })
}

fn build_example<'a>(
    builder: &mut EnvBuilder<'a>,
    name: &str,
    ty: &Expr,
    val: &Expr,
    known: &HashMap<String, Vec<String>>,
    hovers: &mut Vec<HoverNode<'a>>,
) -> Result<Declar<'a>, CompileError> {
    let mut scope = ElabScope::new();
    let univ = make_univ_map(builder, &[]);
    let ty = elab_expr(builder, ty, &mut scope, &univ, known, hovers)?;
    let val = elab_expr(builder, val, &mut scope, &univ, known, hovers)?;
    let name = builder.name_from_str(name);
    let uparams = builder.alloc_levels_slice(&[]);
    Ok(Declar::Definition {
        info: DeclarInfo { name, uparams, ty },
        val,
        hint: ReducibilityHint::Regular(0),
    })
}

fn build_axiom<'a>(
    builder: &mut EnvBuilder<'a>,
    name: &str,
    universe: &[String],
    ty: &Expr,
    known: &HashMap<String, Vec<String>>,
    hovers: &mut Vec<HoverNode<'a>>,
) -> Result<Declar<'a>, CompileError> {
    let mut scope = ElabScope::new();
    let univ = make_univ_map(builder, universe);
    let ty = elab_expr(builder, ty, &mut scope, &univ, known, hovers)?;
    let name = builder.name_from_str(name);
    let uparams = collect_uparams(builder, &univ, universe);
    Ok(Declar::Axiom {
        info: DeclarInfo { name, uparams, ty },
    })
}

fn make_univ_map<'a>(builder: &mut EnvBuilder<'a>, universe: &[String]) -> UnivMap<'a> {
    universe
        .iter()
        .map(|name| {
            let ptr = builder.name_from_str(name);
            let level = builder.level_param(ptr);
            (name.clone(), level)
        })
        .collect()
}

fn collect_uparams<'a>(
    builder: &mut EnvBuilder<'a>,
    univ: &UnivMap<'a>,
    universe: &[String],
) -> sokonanoda::util::LevelsPtr<'a> {
    let levels: Vec<LevelPtr<'a>> = universe.iter().map(|name| univ[name]).collect();
    builder.alloc_levels_slice(&levels)
}

fn level_ptr<'a>(
    builder: &mut EnvBuilder<'a>,
    level: &str,
    univ: &UnivMap<'a>,
    span: Span,
) -> Result<LevelPtr<'a>, CompileError> {
    if let Ok(n) = level.parse::<u64>() {
        let mut out = builder.zero();
        for _ in 0..n {
            out = builder.succ(out);
        }
        Ok(out)
    } else if let Some(level) = univ.get(level).copied() {
        Ok(level)
    } else {
        Err(CompileError::elab(
            ErrorKind::ElabUnknownUniverseLevel,
            format!("unknown universe level `{level}`"),
            span,
        ))
    }
}

fn kernel_binder_style(kind: &BinderKind) -> BinderStyle {
    match kind {
        BinderKind::Explicit => BinderStyle::Default,
        BinderKind::Implicit => BinderStyle::Implicit,
    }
}

fn elab_expr<'a>(
    builder: &mut EnvBuilder<'a>,
    expr: &Expr,
    scope: &mut ElabScope<'a>,
    univ: &UnivMap<'a>,
    known: &HashMap<String, Vec<String>>,
    hovers: &mut Vec<HoverNode<'a>>,
) -> Result<ExprPtr<'a>, CompileError> {
    match expr {
        Expr::Sort {
            sort: SortKind::Prop,
            span,
        } => {
            let z = builder.zero();
            let out = builder.mk_sort(z);
            record_hover(hovers, scope, *span, out);
            Ok(out)
        }
        Expr::Sort {
            sort: SortKind::Type,
            span,
        } => {
            let z = builder.zero();
            let ty = builder.succ(z);
            let out = builder.mk_sort(ty);
            record_hover(hovers, scope, *span, out);
            Ok(out)
        }
        Expr::Sort {
            sort: SortKind::Sort(n),
            span,
        } => {
            let mut level = builder.zero();
            for _ in 0..*n {
                level = builder.succ(level);
            }
            let out = builder.mk_sort(level);
            record_hover(hovers, scope, *span, out);
            Ok(out)
        }
        Expr::Sort {
            sort: SortKind::Level(name),
            span,
        } => {
            let level = univ.get(name).copied().ok_or_else(|| {
                CompileError::elab(
                    ErrorKind::ElabUnknownUniverseLevel,
                    format!("universe variable `{name}` is not declared in this declaration"),
                    *span,
                )
            })?;
            let out = builder.mk_sort(level);
            record_hover(hovers, scope, *span, out);
            Ok(out)
        }
        Expr::Ident { name, span } => {
            let out = if let Some(pos) = scope.names.iter().rposition(|candidate| candidate == name)
            {
                let idx = u16::try_from(scope.names.len() - 1 - pos).map_err(|_| {
                    CompileError::elab(
                        ErrorKind::ElabTooManyBinders,
                        "too many nested binders for kernel index",
                        *span,
                    )
                })?;
                builder.mk_var(idx)
            } else {
                let params = known.get(name).ok_or_else(|| {
                    CompileError::elab(
                        ErrorKind::ElabUnknownIdentifier,
                        format!("unknown identifier `{name}`"),
                        *span,
                    )
                })?;
                let levels: Vec<LevelPtr<'a>> = params.iter().map(|_| builder.zero()).collect();
                let levels = builder.alloc_levels_slice(&levels);
                let name = builder.name_from_str(name);
                builder.mk_const(name, levels)
            };
            record_hover(hovers, scope, *span, out);
            Ok(out)
        }
        Expr::UniverseApp { name, levels, span } => {
            let params = known.get(name).ok_or_else(|| {
                CompileError::elab(
                    ErrorKind::ElabUnknownConstant,
                    format!("unknown constant `{name}`"),
                    *span,
                )
            })?;
            if params.len() != levels.len() {
                return Err(CompileError::elab(
                    ErrorKind::ElabUniverseArity,
                    format!(
                        "constant `{name}` expects {} universe argument(s), got {}",
                        params.len(),
                        levels.len()
                    ),
                    *span,
                ));
            }
            let mut resolved = Vec::with_capacity(levels.len());
            for level in levels {
                resolved.push(level_ptr(builder, level, univ, *span)?);
            }
            let levels = builder.alloc_levels_slice(&resolved);
            let name = builder.name_from_str(name);
            let out = builder.mk_const(name, levels);
            record_hover(hovers, scope, *span, out);
            Ok(out)
        }
        Expr::Num { value, span } => {
            let n: num_bigint::BigUint = value.parse().map_err(|_| {
                CompileError::elab(
                    ErrorKind::ElabInvalidNatLiteral,
                    format!("invalid natural literal `{value}`"),
                    *span,
                )
            })?;
            let ptr = builder.alloc_bignum(n).ok_or_else(|| {
                CompileError::elab(
                    ErrorKind::ElabNatLiteralDisabled,
                    "Nat literals are disabled",
                    *span,
                )
            })?;
            let out = builder.mk_nat_lit(ptr).ok_or_else(|| {
                CompileError::elab(
                    ErrorKind::ElabNatLiteralDisabled,
                    "Nat literals are disabled",
                    *span,
                )
            })?;
            record_hover(hovers, scope, *span, out);
            Ok(out)
        }
        Expr::Hole { span } => Err(CompileError::elab(
            ErrorKind::ElabHoleMisplaced,
            "`???` is only allowed as the value of an open exercise",
            *span,
        )),
        Expr::App { fun, arg, span } => {
            let fun = elab_expr(builder, fun, scope, univ, known, hovers)?;
            let arg = elab_expr(builder, arg, scope, univ, known, hovers)?;
            let out = builder.mk_app(fun, arg);
            record_hover(hovers, scope, *span, out);
            Ok(out)
        }
        Expr::Lambda {
            binders,
            body,
            span,
        } => {
            let base = scope.len();
            let mut names = Vec::with_capacity(binders.len());
            let mut tys = Vec::with_capacity(binders.len());
            let mut styles = Vec::with_capacity(binders.len());
            for binder in binders {
                let ty = match &binder.ty {
                    Some(ty) => elab_expr(builder, ty, scope, univ, known, hovers)?,
                    None => {
                        return Err(CompileError::elab(
                            ErrorKind::ElabUntypedBinder,
                            "untyped binders need elaboration inference (not in v0)",
                            binder.span,
                        ));
                    }
                };
                let name = builder.name_from_str(&binder.name);
                tys.push(ty);
                names.push(name);
                styles.push(kernel_binder_style(&binder.style));
                scope.push(binder.name.clone(), ty);
            }
            let mut body_expr = elab_expr(builder, body, scope, univ, known, hovers)?;
            scope.truncate(base);
            for ((name, ty), style) in names.into_iter().zip(tys).zip(styles).rev() {
                body_expr = builder.mk_lambda(name, style, ty, body_expr);
            }
            record_hover(hovers, scope, *span, body_expr);
            Ok(body_expr)
        }
        Expr::Forall {
            binders,
            body,
            span,
        } => {
            let base = scope.len();
            let mut names = Vec::with_capacity(binders.len());
            let mut tys = Vec::with_capacity(binders.len());
            let mut styles = Vec::with_capacity(binders.len());
            for binder in binders {
                let ty = match &binder.ty {
                    Some(ty) => elab_expr(builder, ty, scope, univ, known, hovers)?,
                    None => {
                        return Err(CompileError::elab(
                            ErrorKind::ElabUntypedBinder,
                            "untyped binders need elaboration inference (not in v0)",
                            binder.span,
                        ));
                    }
                };
                let name = builder.name_from_str(&binder.name);
                tys.push(ty);
                names.push(name);
                styles.push(kernel_binder_style(&binder.style));
                scope.push(binder.name.clone(), ty);
            }
            let mut body_expr = elab_expr(builder, body, scope, univ, known, hovers)?;
            scope.truncate(base);
            for ((name, ty), style) in names.into_iter().zip(tys).zip(styles).rev() {
                body_expr = builder.mk_pi(name, style, ty, body_expr);
            }
            record_hover(hovers, scope, *span, body_expr);
            Ok(body_expr)
        }
        Expr::Arrow {
            domain,
            codomain,
            span,
        } => {
            let domain = elab_expr(builder, domain, scope, univ, known, hovers)?;
            // `A -> B` desugars to a Pi with an anonymous binder, so free
            // variables in the codomain live one binder deeper.
            scope.push(String::new(), domain);
            let codomain = elab_expr(builder, codomain, scope, univ, known, hovers)?;
            scope.truncate(scope.len() - 1);
            let anon = builder.anonymous();
            let out = builder.mk_pi(anon, BinderStyle::Default, domain, codomain);
            record_hover(hovers, scope, *span, out);
            Ok(out)
        }
        Expr::Plus { lhs, rhs, span } => {
            let add = builder.name_from_str("Nat.add");
            let levels = builder.alloc_levels_slice(&[]);
            let add_const = builder.mk_const(add, levels);
            let lhs = elab_expr(builder, lhs, scope, univ, known, hovers)?;
            let rhs = elab_expr(builder, rhs, scope, univ, known, hovers)?;
            let applied = builder.mk_app(add_const, lhs);
            let out = builder.mk_app(applied, rhs);
            record_hover(hovers, scope, *span, out);
            Ok(out)
        }
    }
}

/// Render an AST expression back to source text (used for open-exercise goals).
pub fn render_expr(expr: &Expr) -> String {
    crate::proof::render_expr(expr)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse;

    fn py_core() -> String {
        std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/py-fol-core.sokonanoda"
        ))
        .expect("read py-fol-core.sokonanoda")
    }

    fn py_core_checks(extra: &str) {
        let full = format!("{}\n{}", py_core(), extra);
        let file = parse(&full).unwrap_or_else(|e| panic!("parse failed: {e:?}"));
        let out = compile_fol(&file);
        assert_eq!(out.errors, vec![], "extra: {extra}");
    }

    fn py_core_rejects(extra: &str) {
        let full = format!("{}\n{}", py_core(), extra);
        let file = parse(&full).unwrap_or_else(|e| panic!("parse failed: {e:?}"));
        let out = compile_fol(&file);
        assert!(!out.errors.is_empty(), "expected rejection: {extra}");
    }

    #[test]
    fn checks_a_valid_file_end_to_end() {
        let file = parse("def id : Prop -> Prop := fun (x : Prop) => x\n#check id\n").unwrap();
        let out = compile_fol(&file);
        assert_eq!(out.errors, vec![]);
        assert_eq!(out.events.len(), 2);
        assert!(matches!(
            &out.events[0],
            CheckEvent::DeclarationChecked { name } if name == "id"
        ));
        assert!(matches!(
            &out.events[1],
            CheckEvent::TypeChecked { text, .. } if text == "Prop -> Prop"
        ));
    }

    #[test]
    fn accepts_open_exercise() {
        let file = parse("example : Prop -> Prop := ???\n").unwrap();
        let out = compile_fol(&file);
        assert_eq!(out.errors, vec![]);
        assert_eq!(out.events, vec![CheckEvent::ExerciseOpen { name: None }]);
    }

    #[test]
    fn checks_dependent_forall_with_lambda() {
        let file = parse(
            "theorem t : ∀ (P : Prop), P -> P :=\n\
             fun (P : Prop) (hp : P) => hp\n",
        )
        .unwrap();
        let out = compile_fol(&file);
        assert_eq!(out.errors, vec![]);
        assert_eq!(
            out.events,
            vec![CheckEvent::DeclarationChecked { name: "t".into() }]
        );
    }

    #[test]
    fn checks_axioms_and_theorems_over_axioms() {
        let file = parse(
            "axiom p : Prop\n\
             theorem t : p -> p := fun (h : p) => h\n",
        )
        .unwrap();
        let out = compile_fol(&file);
        assert_eq!(out.errors, vec![]);
        assert_eq!(
            out.events,
            vec![
                CheckEvent::DeclarationChecked { name: "p".into() },
                CheckEvent::DeclarationChecked { name: "t".into() },
            ]
        );
    }

    #[test]
    fn reports_kernel_rejection_with_span() {
        let file = parse("def bad : Prop -> Type := fun (x : Prop) => x\n").unwrap();
        let out = compile_fol(&file);
        assert!(!out.errors.is_empty(), "expected a kernel rejection");
        assert!(out.errors[0].span.start.line >= 1);
    }

    #[test]
    fn checks_nat_literals_and_reduces_addition() {
        let file = parse(
            "def two : Nat := 1 + 1\n\
             #check two\n\
             #reduce 1 + 1\n",
        )
        .unwrap();
        let out = compile_fol(&file);
        assert_eq!(out.errors, vec![]);
        assert_eq!(out.events.len(), 3);
        assert!(matches!(
            &out.events[1],
            CheckEvent::TypeChecked { text, .. } if text == "Nat"
        ));
        assert!(matches!(
            &out.events[2],
            CheckEvent::Reduced { text, .. } if text == "2"
        ));
    }

    #[test]
    fn checks_from_scratch_fol_proofs() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/fol-basics.sokonanoda"
        );
        let src = std::fs::read_to_string(path).expect("read fol-basics.sokonanoda");
        let file = parse(&src).expect("parse fol-basics.sokonanoda");
        let out = compile_fol(&file);
        assert_eq!(
            out.errors,
            vec![],
            "from-scratch FOL proofs should all check, got {:?}",
            out.errors
        );
        assert!(!out.events.is_empty());
    }

    #[test]
    fn checks_universe_levels_and_function_type_types() {
        let file = parse(
            "#check Sort 2\n\
             #check (fun (α : Sort 2) => α)\n",
        )
        .unwrap();
        let out = compile_fol(&file);
        assert_eq!(out.errors, vec![]);
        assert_eq!(out.events.len(), 2);
        assert!(matches!(
            &out.events[0],
            CheckEvent::TypeChecked { text, .. } if text == "Type 2"
        ));
        assert!(matches!(
            &out.events[1],
            CheckEvent::TypeChecked { text, .. } if text == "Type 1 -> Type 1"
        ));
    }

    #[test]
    fn prints_definitions_without_panicking_on_open_bodies() {
        let file = parse(
            "def id : Prop -> Prop := fun (x : Prop) => x\n\
             #print id\n",
        )
        .unwrap();
        let out = compile_fol(&file);
        assert_eq!(out.errors, vec![]);
        let printed = out.events.iter().find_map(|event| match event {
            CheckEvent::Printed { name, text } => Some((name.as_str(), text.as_str())),
            _ => None,
        });
        assert_eq!(
            printed,
            Some(("id", "def id : Prop -> Prop := fun (x : Prop) => x"))
        );
    }

    #[test]
    fn checks_universe_polymorphic_id_declaration() {
        let file = parse(
            "def id {u} : forall (α : Sort u), α -> α :=\n\
             fun (α : Sort u) => fun (a : α) => a\n",
        )
        .expect("parse universe-polymorphic declaration");
        let out = compile_fol(&file);
        assert_eq!(out.errors, vec![]);
    }

    #[test]
    fn checks_explicit_universe_application() {
        let file = parse(
            "def id {u} : forall (α : Sort u), α -> α :=\n\
             fun (α : Sort u) => fun (a : α) => a\n\
             def id2 {u} : forall (α : Sort u), α -> α := id.{u}\n",
        )
        .expect("parse explicit universe application");
        let out = compile_fol(&file);
        assert_eq!(out.errors, vec![]);
    }

    #[test]
    fn checks_plain_use_defaults_universe_to_zero() {
        let file = parse(
            "def id {u} : forall (α : Sort u), α -> α :=\n\
             fun (α : Sort u) => fun (a : α) => a\n\
             #check id\n",
        )
        .expect("parse plain use");
        let out = compile_fol(&file);
        assert_eq!(out.errors, vec![]);
        assert!(out
            .events
            .iter()
            .any(|event| matches!(event, CheckEvent::TypeChecked { .. })));
    }

    #[test]
    fn checks_explicit_literal_universe_application() {
        let file = parse(
            "def id {u} : forall (α : Sort u), α -> α :=\n\
             fun (α : Sort u) => fun (a : α) => a\n\
             def id0 : (α : Prop) -> α -> α :=\n\
             fun (α : Prop) => id.{0} α\n",
        )
        .expect("parse literal universe application");
        let out = compile_fol(&file);
        assert_eq!(out.errors, vec![]);
    }

    #[test]
    fn checks_axiom_with_two_universe_params() {
        let file = parse(
            "axiom cast {u, v} :\n\
             forall (α : Sort u), forall (β : Sort v), α -> β\n",
        )
        .expect("parse two universe params");
        let out = compile_fol(&file);
        assert_eq!(out.errors, vec![]);
    }

    #[test]
    fn rejects_undeclared_universe_variable() {
        let file = parse(
            "def bad : forall (α : Sort u), α -> α :=\n\
             fun (α : Sort u) => fun (a : α) => a\n",
        )
        .expect("parse undeclared universe");
        let out = compile_fol(&file);
        assert!(!out.errors.is_empty());
    }

    #[test]
    fn rejects_wrong_number_of_universe_arguments() {
        let file = parse(
            "def id {u} : forall (α : Sort u), α -> α :=\n\
             fun (α : Sort u) => fun (a : α) => a\n\
             def bad {u} : forall (α : Sort u), α -> α := id.{u, 0}\n",
        )
        .expect("parse wrong universe count");
        let out = compile_fol(&file);
        assert!(!out.errors.is_empty());
    }

    #[test]
    fn checks_at_marker_and_implicit_binders_in_py_fol_style() {
        let file = parse(
            "def id {u} : forall {α : Sort u}, forall (a : α), α :=\n\
             fun {α : Sort u} => fun (a : α) => a\n\
             def id0 : forall (α : Prop), forall (a : α), α :=\n\
             fun (α : Prop) => @id.{0} α\n",
        )
        .expect("parse @id.{0} and implicit binders");
        let out = compile_fol(&file);
        assert_eq!(out.errors, vec![]);
    }

    #[test]
    fn parses_implicit_binder_styles() {
        let file = parse("#check fun {x : Prop} => fun (y : Prop) => x\n").unwrap();
        assert_eq!(file.commands.len(), 1);
    }

    #[test]
    fn checks_ported_py_fol_core() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/py-fol-core.sokonanoda"
        );
        let src = std::fs::read_to_string(path).expect("read py-fol-core.sokonanoda");
        let file = parse(&src).expect("parse py-fol-core.sokonanoda");
        let out = compile_fol(&file);
        assert_eq!(out.errors, vec![]);
    }

    #[test]
    fn py_mt_and_not_and_of_not_left() {
        py_core_checks(
            "theorem mt_ : {a : Prop} -> {b : Prop} -> (f : a -> b) -> (hb : Not b) -> Not a :=\n\
             fun {a : Prop} => fun {b : Prop} => fun (f : a -> b) => fun (hb : Not b) => fun (ha : a) => hb (f ha)\n\
             theorem not_and_left_ : {a : Prop} -> (b : Prop) -> (ha : Not a) -> Not (And a b) :=\n\
             fun {a : Prop} => fun (b : Prop) => fun (ha : Not a) => fun (x : And a b) => ha (@And.left a b x)\n",
        );
    }

    #[test]
    fn py_or_elim_and_not_or_intro() {
        py_core_checks(
            "theorem or_elim_ : {a : Prop} -> {b : Prop} -> {c : Prop} -> (t : Or a b) -> (left : a -> c) -> (right : b -> c) -> c :=\n\
             fun {a : Prop} => fun {b : Prop} => fun {c : Prop} => fun (t : Or a b) => fun (left : a -> c) => fun (right : b -> c) => @Or.rec a b (fun (x : Or a b) => c) left right t\n\
             theorem not_or_intro_ : {a : Prop} -> {b : Prop} -> (ha : Not a) -> (hb : Not b) -> Not (Or a b) :=\n\
             fun {a : Prop} => fun {b : Prop} => fun (ha : Not a) => fun (hb : Not b) => fun (x : Or a b) => @Or.rec a b (fun (y : Or a b) => False) (fun (l : a) => ha l) (fun (r : b) => hb r) x\n",
        );
    }

    #[test]
    fn py_and_imp_is_iff() {
        py_core_checks(
            "theorem and_imp_iff_ : {a : Prop} -> {b : Prop} -> {c : Prop} -> Iff (And a b -> c) (a -> b -> c) :=\n\
             fun {a : Prop} => fun {b : Prop} => fun {c : Prop} =>\n\
               @Iff.intro (And a b -> c) (a -> b -> c)\n\
                 (fun (h : And a b -> c) => fun (ha : a) => fun (hb : b) => h (@And.intro a b ha hb))\n\
                 (fun (h : a -> b -> c) => fun (x : And a b) => @And.rec a b (fun (y : And a b) => c) (fun (ha : a) => fun (hb : b) => h ha hb) x)\n",
        );
    }

    #[test]
    fn py_iff_refl_and_imp_swap() {
        py_core_checks(
            "theorem iff_refl_ : (a : Prop) -> Iff a a :=\n\
             fun (a : Prop) => @Iff.intro a a (fun (h : a) => h) (fun (h : a) => h)\n\
             theorem imp_swap_ : {a : Prop} -> {b : Prop} -> {c : Prop} -> Iff (a -> b -> c) (b -> a -> c) :=\n\
             fun {a : Prop} => fun {b : Prop} => fun {c : Prop} => @Iff.intro (a -> b -> c) (b -> a -> c) (@flip.{0, 0, 0} a b c) (@flip.{0, 0, 0} b a c)\n",
        );
    }

    #[test]
    fn py_accepts_shadowed_binders() {
        py_core_checks(
            "theorem shadow_ : (a : Prop) -> (a : Prop) -> a -> a :=\n\
             fun (a : Prop) => fun (a : Prop) => fun (h : a) => h\n",
        );
    }

    #[test]
    fn py_rejects_constructor_type_mismatch() {
        py_core_rejects("example : Or True True := True.intro\n");
    }

    #[test]
    fn py_rejects_function_domain_mismatch() {
        py_core_rejects("example : False -> True := fun (h : True) => True.intro\n");
    }

    #[test]
    fn py_rejects_or_constructor_on_wrong_side() {
        py_core_rejects("example : Or True False := @Or.inl False True True.intro\n");
    }

    #[test]
    fn py_rejects_self_application() {
        py_core_rejects("example : (x : Prop) -> Prop := fun (x : Prop) => x x\n");
    }

    #[test]
    fn py_accepts_eta_application_chain() {
        py_core_checks(
            "example : (a : Prop) -> (a -> a) -> a -> a :=\n\
             fun (a : Prop) => fun (f : a -> a) => fun (x : a) => f x\n",
        );
    }

    #[test]
    fn py_accepts_or_rec_with_unrelated_motive() {
        py_core_checks(
            "theorem deep_or_rec_ : {a : Prop} -> {b : Prop} -> {c : Prop} -> (h : Or a b) -> (hc : c) -> c :=\n\
             fun {a : Prop} => fun {b : Prop} => fun {c : Prop} => fun (h : Or a b) => fun (hc : c) =>\n\
               @Or.rec a b (fun (x : Or a b) => c) (fun (ha : a) => hc) (fun (hb : b) => hc) h\n",
        );
    }

    #[test]
    fn py_eq_symm_trans_are_in_ported_core() {
        py_core_checks(
            "theorem eq_symm_test_ {u} : {α : Sort u} -> (a : α) -> (b : α) -> (h : @Eq.{u} α a b) -> @Eq.{u} α b a :=\n\
             fun {α : Sort u} => fun (a : α) => fun (b : α) => fun (h : @Eq.{u} α a b) => @Eq_symm.{u} α a b h\n\
             theorem eq_trans_test_ {u} : {α : Sort u} -> (a : α) -> (b : α) -> (c : α) -> (h1 : @Eq.{u} α a b) -> (h2 : @Eq.{u} α b c) -> @Eq.{u} α a c :=\n\
             fun {α : Sort u} => fun (a : α) => fun (b : α) => fun (c : α) => fun (h1 : @Eq.{u} α a b) => fun (h2 : @Eq.{u} α b c) => @Eq_trans.{u} α a b c h1 h2\n",
        );
    }

    #[test]
    fn py_propext_and_self_eq_checks() {
        py_core_checks(
            "theorem and_self_eq_ : (p : Prop) -> @Eq.{1} Prop (And p p) p :=\n\
             fun (p : Prop) => @propext (And p p) p (@Iff.intro (And p p) p (@And.left p p) (fun (h : p) => @And.intro p p h h))\n",
        );
    }

    #[test]
    fn py_congr_arg_prop_level_checks() {
        py_core_checks(
            "theorem congrArg_prop_test_ {u} : {α : Sort u} -> (β : Prop) -> (f : α -> β) -> (a1 : α) -> (a2 : α) -> (h : @Eq.{u} α a1 a2) -> @Eq.{0} β (f a1) (f a2) :=\n\
             fun {α : Sort u} => fun (β : Prop) => fun (f : α -> β) => fun (a1 : α) => fun (a2 : α) => fun (h : @Eq.{u} α a1 a2) => @congrArg.{u} α β f a1 a2 h\n",
        );
    }

    #[test]
    fn py_rejects_congr_arg_outside_prop_universe() {
        py_core_rejects(
            "theorem congrArg_bad_ {u, v} : {α : Sort u} -> {β : Sort v} -> (f : α -> β) -> (a1 : α) -> (a2 : α) -> (h : @Eq.{u} α a1 a2) -> @Eq.{v} β (f a1) (f a2) :=\n\
             fun {α : Sort u} => fun {β : Sort v} => fun (f : α -> β) => fun (a1 : α) => fun (a2 : α) => fun (h : @Eq.{u} α a1 a2) => @Eq.rec.{u, v} α a1 (fun (x : α) => @Eq.{v} β (f a1) (f x)) (@Eq.refl.{v} β (f a1)) a2 h\n",
        );
    }

    #[test]
    fn py_not_imp_of_and_not_is_in_core() {
        py_core_checks(
            "theorem not_imp_use_ : {a : Prop} -> {b : Prop} -> (x : And a (Not b)) -> Not (a -> b) :=\n\
             fun {a : Prop} => fun {b : Prop} => fun (x : And a (Not b)) => @not_imp_of_and_not_ a b x\n",
        );
    }

    #[test]
    fn py_or_iff_left_of_imp_is_in_core() {
        py_core_checks(
            "theorem or_left_use_ : {b : Prop} -> {a : Prop} -> (hb : b -> a) -> Iff (Or a b) a :=\n\
             fun {b : Prop} => fun {a : Prop} => fun (hb : b -> a) => @or_iff_left_of_imp_ b a hb\n",
        );
    }

    #[test]
    fn explicit_inductive_block_compiles() {
        let src = r#"
inductive MyNat : Type
ctor z : MyNat
ctor s (n : MyNat) : MyNat
rec MyNat.rec {u} : (motive : (n : MyNat) -> Sort u) -> (mz : motive z) -> (ms : (n : MyNat) -> motive n -> motive (s n)) -> (n : MyNat) -> motive n
iota z := fun (motive : (n : MyNat) -> Sort u) => fun (mz : motive z) => fun (ms : (n : MyNat) -> motive n -> motive (s n)) => mz
iota s := fun (motive : (n : MyNat) -> Sort u) => fun (mz : motive z) => fun (ms : (n : MyNat) -> motive n -> motive (s n)) => fun (n : MyNat) => s (MyNat.rec motive mz ms n)
end
def oneMyNat : MyNat := s z
def myAdd : MyNat -> MyNat -> MyNat :=
  fun (m : MyNat) => fun (n : MyNat) =>
    MyNat.rec.{1} (fun (x : MyNat) => MyNat) n (fun (k : MyNat) => fun (ih : MyNat) => s ih) m
#reduce MyNat.rec.{1} (fun (n : MyNat) => MyNat) z (fun (n : MyNat) => fun (ih : MyNat) => s n) z
#reduce MyNat.rec.{1} (fun (n : MyNat) => MyNat) z (fun (n : MyNat) => fun (ih : MyNat) => s n) (s z)
#reduce myAdd (s (s z)) (s z)
"#;
        let file = parse(src).expect("parse explicit inductive block");
        let out = compile_fol(&file);
        assert_eq!(out.errors, vec![]);
        assert!(
            out.events.iter().any(|e| matches!(
                e,
                CheckEvent::Reduced { text, .. } if text == "z"
            )),
            "events: {:?}",
            out.events
        );
        assert!(
            out.events.iter().any(|e| matches!(
                e,
                CheckEvent::Reduced { text, .. } if text == "s z"
            )),
            "events: {:?}",
            out.events
        );
        assert!(
            out.events.iter().any(|e| matches!(
                e,
                CheckEvent::Reduced { text, .. } if text == "s (s (s z))"
            )),
            "events: {:?}",
            out.events
        );
    }

    #[test]
    fn explicit_nat_block_overrides_builtin_prelude() {
        let src = r#"
inductive Nat : Type
ctor zero : Nat
ctor succ (n : Nat) : Nat
rec Nat.rec {u} : (motive : (n : Nat) -> Sort u) -> (mz : motive zero) -> (ms : (n : Nat) -> motive n -> motive (succ n)) -> (n : Nat) -> motive n
iota zero := fun (motive : (n : Nat) -> Sort u) => fun (mz : motive zero) => fun (ms : (n : Nat) -> motive n -> motive (succ n)) => mz
iota succ := fun (motive : (n : Nat) -> Sort u) => fun (mz : motive zero) => fun (ms : (n : Nat) -> motive n -> motive (succ n)) => fun (n : Nat) => succ (Nat.rec motive mz ms n)
end
def oneNat : Nat := succ zero
#reduce Nat.rec.{1} (fun (n : Nat) => Nat) zero (fun (n : Nat) => fun (ih : Nat) => succ n) (succ zero)
"#;
        let file = parse(src).expect("parse explicit Nat block");
        let out = compile_fol(&file);
        assert_eq!(out.errors, vec![]);
        assert!(
            out.events.iter().any(|e| matches!(
                e,
                CheckEvent::Reduced { text, .. } if text == "succ zero"
            )),
            "events: {:?}",
            out.events
        );
    }

    #[test]
    fn ported_nat_fol_add_two_two_reduces() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/py-nat.sokonanoda"
        );
        let src = std::fs::read_to_string(path).expect("read py-nat.sokonanoda");
        let file = parse(&src).expect("parse py-nat.sokonanoda");
        let out = compile_fol(&file);
        assert_eq!(out.errors, vec![]);
        assert!(
            out.events.iter().any(|e| matches!(
                e,
                CheckEvent::Reduced { text, .. } if text == "succ (succ (succ (succ zero)))"
            )),
            "events: {:?}",
            out.events
        );
    }

    #[test]
    fn named_arrow_is_forall_with_explicit_binder() {
        let file = parse(
            "def id {u} : (α : Sort u) -> α -> α :=\n\
             fun {α : Sort u} => fun (a : α) => a\n",
        )
        .expect("parse named arrow");
        let out = compile_fol(&file);
        assert_eq!(out.errors, vec![]);
    }

    #[test]
    fn named_arrow_supports_implicit_binders() {
        let file = parse(
            "def id0 : {a : Prop} -> a -> a :=\n\
             fun {a : Prop} => fun (h : a) => h\n",
        )
        .expect("parse implicit named arrow");
        let out = compile_fol(&file);
        assert_eq!(out.errors, vec![]);
    }

    #[test]
    fn document_report_tracks_open_checked_failed_decls() {
        let file = parse(
            "def ok : Prop -> Prop := fun (x : Prop) => x\n\
             example : Nat := ???\n\
             def bad : Prop -> Type := fun (x : Prop) => x\n",
        )
        .expect("parse");
        let report = check_document(&file);
        assert_eq!(report.decls.len(), 3, "decls: {:?}", report.decls);
        assert_eq!(report.decls[0].status, DeclStatus::Checked);
        assert_eq!(report.decls[0].name.as_deref(), Some("ok"));
        assert_eq!(report.decls[1].status, DeclStatus::Open);
        assert_eq!(report.decls[1].goal.as_deref(), Some("Nat"));
        assert_eq!(report.decls[2].status, DeclStatus::Failed);
        assert_eq!(
            report.decls[2].error.as_ref().map(|e| e.code()),
            Some("kernel-rejected")
        );
        // an open exercise must not make later valid declarations fail
        let file2 = parse(
            "def f : Prop -> Prop := fun (x : Prop) => x\n\
             theorem ex : (a : Prop) -> a -> a := ???\n\
             example : Prop -> Prop := fun (x : Prop) => x\n",
        )
        .expect("parse2");
        let out = compile_fol(&file2);
        assert_eq!(out.errors, vec![], "errors: {:?}", out.errors);
        assert!(out
            .events
            .iter()
            .any(|e| matches!(e, CheckEvent::ExerciseOpen { name: Some(n) } if n == "ex")));
        assert!(out
            .events
            .iter()
            .any(|e| matches!(e, CheckEvent::ExampleChecked)));
    }

    #[test]
    fn document_report_produces_hover_types_for_subexpressions() {
        let file = parse(
            "def two : Nat := 1 + 1\n\
             def id : Prop -> Prop := fun (x : Prop) => x\n",
        )
        .expect("parse");
        let report = check_document(&file);
        assert!(
            report.hovers.iter().any(|h| h.text == "Nat"),
            "expected a Nat hover, got {:?}",
            report
                .hovers
                .iter()
                .map(|h| h.text.clone())
                .collect::<Vec<_>>()
        );
        assert!(
            report.hovers.iter().any(|h| h.text == "Prop -> Prop"),
            "expected a function-type hover, got {:?}",
            report
                .hovers
                .iter()
                .map(|h| h.text.clone())
                .collect::<Vec<_>>()
        );
        assert!(
            report.hovers.iter().any(|h| h.text == "Prop"),
            "expected a Prop hover for the bound variable x, got {:?}",
            report
                .hovers
                .iter()
                .map(|h| h.text.clone())
                .collect::<Vec<_>>()
        );
    }
}

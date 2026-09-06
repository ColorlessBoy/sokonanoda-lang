//! 编译入口：`compile_fol`/`check_document`、待执行操作与 hover 解析。

use super::elab::{
    build_axiom, build_def, build_example, build_theorem, elab_expr, install_inductive_block,
    ElabScope, HoverNode, UnivMap,
};
use super::error::{CompileError, ErrorKind};
use super::event::{CheckEvent, CompileOutput};
use super::prelude::install_prelude;
use super::report::{DeclKind, DeclState, DeclStatus, DocumentReport, HoverType};
use crate::{Command, Expr, FolFile, Span};
use sokonanoda::builder::EnvBuilder;
use sokonanoda::env::{Declar, EnvLimit};
use sokonanoda::util::{Config, ExprPtr, NamePtr};
use std::collections::HashMap;

pub(crate) enum PendingOp<'a> {
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

pub(crate) struct CmdHover<'a> {
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

pub(crate) fn failed_state(
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
pub(crate) fn resolve_hovers(
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

/// Render an AST expression back to source text (used for open-exercise goals).
pub fn render_expr(expr: &Expr) -> String {
    crate::proof::render_expr(expr)
}

//! 编译入口：`compile_fol`/`check_document`、待执行操作与 hover 解析。

use super::elab::{
    build_axiom, build_def, build_example, build_theorem, elab_expr, install_inductive_block,
    ElabScope, HoverNode, UnivMap,
};
use super::error::{parse_def_eq_mismatch, CompileError, ErrorKind};
use super::event::{CheckEvent, CompileOutput};
use super::prelude::{install_eq_prelude, install_prelude, CompileOptions, PreludeMode};
use super::report::{DeclKind, DeclState, DeclStatus, DocumentReport, GoalBinder, HoverType};
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
        cmd: usize,
    },
    /// One whole `inductive ... end` block: the kernel validates each of its
    /// declarations (inductive spine, constructors, recursor rules).
    InductiveBlock {
        name: String,
        declars: Vec<Declar<'a>>,
        span: Span,
        cmd: usize,
    },
    OpenExercise {
        name: Option<String>,
        kind: DeclKind,
        goal: Option<String>,
        binders: Vec<GoalBinder>,
        span: Span,
        cmd: usize,
    },
    Check {
        expr: ExprPtr<'a>,
        env_at: usize,
        span: Span,
        cmd: usize,
    },
    Reduce {
        expr: ExprPtr<'a>,
        env_at: usize,
        span: Span,
        cmd: usize,
    },
    Print {
        name: String,
        ptr: NamePtr<'a>,
        span: Span,
        cmd: usize,
    },
}

pub(crate) struct CmdHover<'a> {
    env_at: usize,
    nodes: Vec<HoverNode<'a>>,
    cmd: usize,
}

/// Incremental trust plan (I8, docs/design-i8-i9.md): commands `[0, before)`
/// were already kernel-checked in a previous session with the identical text,
/// so this run elaborates them into the environment but does NOT re-check
/// them — their states/hovers/events are reused from the session cache.
pub(crate) struct TrustPlan {
    pub before: usize,
}

/// Compile and kernel-check a whole file in one arena session, returning the
/// batch view (events + errors) that the CLI and tests consume.
pub fn compile_fol(file: &FolFile) -> CompileOutput {
    run(file, &CompileOptions::default(), false).0
}

/// Compile with explicit options (e.g. `PreludeMode::Bare` for a fully bare
/// teaching file that builds every concept from scratch).
pub fn compile_fol_with(file: &FolFile, options: &CompileOptions) -> CompileOutput {
    run(file, options, false).0
}

/// Compile a file and return the detailed document report (per-declaration
/// states, diagnostics, hover types) that the LSP and agents consume.
pub fn check_document(file: &FolFile) -> DocumentReport {
    run(file, &CompileOptions::default(), true).1
}

/// `check_document` with explicit compile options.
pub fn check_document_with(file: &FolFile, options: &CompileOptions) -> DocumentReport {
    run(file, options, true).1
}

/// Is the answer an open exercise: does it contain a `???`, and can the
/// remaining goal be recovered by walking the declared type alongside the
/// lambda binders already written? `None` means "no hole" or "hole in a
/// place the goal cannot be recovered from" (the latter falls through to
/// normal elaboration, which reports `elab-hole-misplaced` at the hole).
/// On success it returns the remaining goal text plus the hypotheses the
/// written lambda binders already introduce (the goal view's context).
fn open_goal(ty: &Expr, val: &Expr) -> Option<(String, Vec<GoalBinder>)> {
    if !expr_has_hole(val) {
        return None;
    }
    goal_under_binders(ty, val)
}

fn expr_has_hole(e: &Expr) -> bool {
    match e {
        Expr::Hole { .. } => true,
        Expr::App { fun, arg, .. }
        | Expr::Plus {
            lhs: fun, rhs: arg, ..
        } => expr_has_hole(fun) || expr_has_hole(arg),
        Expr::Lambda { binders, body, .. } | Expr::Forall { binders, body, .. } => {
            binders
                .iter()
                .any(|b| b.ty.as_deref().is_some_and(expr_has_hole))
                || expr_has_hole(body)
        }
        Expr::Arrow {
            domain, codomain, ..
        } => expr_has_hole(domain) || expr_has_hole(codomain),
        _ => false,
    }
}

/// Walk the declared type and the (partial) answer in parallel: every lambda
/// in the answer consumes one Pi layer of the type; when the walk reaches the
/// hole, the remaining type is the exercise's current goal and the consumed
/// binders are its context.
fn goal_under_binders(ty: &Expr, val: &Expr) -> Option<(String, Vec<GoalBinder>)> {
    match val {
        Expr::Hole { .. } => Some((render_expr(ty), Vec::new())),
        Expr::Lambda { binders, body, .. } => {
            let (binder, binders_rest) = binders.split_first()?;
            // Consume one Pi layer; remember the hypothesis it introduces.
            let (rest_ty, layer_ty_text) = match ty {
                Expr::Forall {
                    binders: tbinders,
                    body: tbody,
                    ..
                } => {
                    let (tbinder, trest) = tbinders.split_first()?;
                    let rest = if trest.is_empty() {
                        tbody.as_ref().clone()
                    } else {
                        Expr::Forall {
                            binders: trest.to_vec(),
                            body: tbody.clone(),
                            span: Span::default(),
                        }
                    };
                    let layer_text = tbinder.ty.as_deref().map(render_expr).unwrap_or_default();
                    (rest, layer_text)
                }
                Expr::Arrow {
                    domain, codomain, ..
                } => {
                    let text = render_expr(domain);
                    (codomain.as_ref().clone(), text)
                }
                _ => return None,
            };
            // The learner's own binder wins for the name and (if written) the
            // type; an untyped binder borrows the declared layer's type.
            let binder_text = binder
                .ty
                .as_deref()
                .map(render_expr)
                .unwrap_or(layer_ty_text);
            let introduced = GoalBinder {
                name: binder.name.clone(),
                ty: binder_text,
            };
            let (goal, rest_binders) = if binders_rest.is_empty() {
                goal_under_binders(&rest_ty, body)?
            } else {
                let rest_val = Expr::Lambda {
                    binders: binders_rest.to_vec(),
                    body: body.clone(),
                    span: Span::default(),
                };
                goal_under_binders(&rest_ty, &rest_val)?
            };
            let mut binders = Vec::with_capacity(rest_binders.len() + 1);
            binders.push(introduced);
            binders.extend(rest_binders);
            Some((goal, binders))
        }
        _ => None,
    }
}

/// Top-level names the file itself declares; used to keep the prelude from
/// shadowing a user declaration (e.g. a file that defines its own `Eq`).
fn user_top_level_names(file: &FolFile) -> std::collections::HashSet<String> {
    file.commands
        .iter()
        .filter_map(|command| match command {
            Command::Def { name, .. }
            | Command::Theorem { name, .. }
            | Command::Axiom { name, .. }
            | Command::InductiveBlock { name, .. } => Some(name.clone()),
            Command::Example { .. }
            | Command::Check { .. }
            | Command::Reduce { .. }
            | Command::Print { .. } => None,
        })
        .collect()
}

fn run(file: &FolFile, options: &CompileOptions, collect: bool) -> (CompileOutput, DocumentReport) {
    // Pass 1 checks everything. Kernel-rejected declarations still occupy
    // their names in pass 1, which lets later declarations reference them —
    // unsound for teaching. Pass 2 recomputes in a fresh session with the
    // kernel-failed declarations removed (check-then-add semantics): their
    // names are free again and dependents fail with a proper diagnosis.
    let (mut out, report, failed, checks) = run_pass(file, options, collect, None, None);
    out.stats.kernel_checks = checks;
    if std::env::var("SOKO_DEBUG_PASS1").is_ok() {
        for (idx, err) in &failed {
            eprintln!("pass1 failed cmd {idx}: {} ({:?})", err.message, err.kind);
        }
    }
    if failed.is_empty() {
        return (out, report);
    }
    let (mut out2, report2, _failed2, checks2) =
        run_pass(file, options, collect, Some(&failed), None);
    out2.stats.kernel_checks = checks + checks2;
    (out2, report2)
}

/// Incremental entry (I8): `trust` marks the reusable prefix `[0, before)`;
/// `prefix_failures` maps trusted command indices to their cached failures —
/// those names stay free (check-then-add) and their states are owned by the
/// session cache, so this pass neither re-checks nor re-reports them.
pub(crate) fn run_incremental(
    file: &FolFile,
    options: &CompileOptions,
    trust: &TrustPlan,
    prefix_failures: &KernelFailed,
) -> (CompileOutput, DocumentReport, usize) {
    let (out1, report1, failed1, checks1) =
        run_pass(file, options, true, Some(prefix_failures), Some(trust));
    if failed1.is_empty() {
        return (out1, report1, checks1);
    }
    if std::env::var("SOKO_DEBUG_PASS1").is_ok() {
        for (idx, err) in &failed1 {
            eprintln!("pass1 failed cmd {idx}: {} ({:?})", err.message, err.kind);
        }
    }
    let mut skip2 = prefix_failures.clone();
    for (idx, err) in &failed1 {
        skip2.insert(*idx, err.clone());
    }
    let (mut out2, report2, _failed2, checks2) =
        run_pass(file, options, true, Some(&skip2), Some(trust));
    out2.stats.kernel_checks = checks1 + checks2;
    (out2, report2, checks1 + checks2)
}

type KernelFailed = HashMap<usize, CompileError>;

fn run_pass(
    file: &FolFile,
    options: &CompileOptions,
    collect: bool,
    skip: Option<&KernelFailed>,
    trust: Option<&TrustPlan>,
) -> (CompileOutput, DocumentReport, KernelFailed, usize) {
    let arena = stumpalo::Arena::new();
    let mut builder = EnvBuilder::new(arena.as_arena_ref(), Config::default());
    let mut known_universes: HashMap<String, Vec<String>> = HashMap::new();
    match options.prelude {
        PreludeMode::Bare => {}
        PreludeMode::Full => {
            let explicit_nat = file.commands.iter().any(
                |command| matches!(command, Command::InductiveBlock { name, .. } if name == "Nat"),
            );
            if !explicit_nat {
                install_prelude(&mut builder);
                for builtin in ["Nat", "Nat.zero", "Nat.succ", "Nat.add"] {
                    known_universes.insert(builtin.to_string(), Vec::new());
                }
            }
            let taken = user_top_level_names(file);
            install_eq_prelude(&mut builder, &mut known_universes, &taken);
        }
    }
    let no_universe: UnivMap = UnivMap::new();

    let mut out = CompileOutput::default();
    let mut report = DocumentReport::default();
    let mut ops: Vec<PendingOp<'_>> = Vec::new();
    let mut cmd_hovers: Vec<CmdHover<'_>> = Vec::new();
    let mut decl_states: Vec<DeclState> = Vec::new();
    let mut example_idx = 0usize;

    let mut failed_cmds: KernelFailed = HashMap::new();
    let mut built_inductives: Vec<Declar<'_>> = Vec::new();
    let mut kernel_checks = 0usize;
    for (idx, command) in file.commands.iter().enumerate() {
        let trusted = trust.is_some_and(|t| idx < t.before);
        let env_before = builder.declaration_count();
        match command {
            Command::Def {
                name,
                universe,
                ty,
                val,
                span,
            } => {
                if trusted {
                    // Trusted prefix: keep the environment, skip the kernel.
                    // Cached failures keep the name free (check-then-add);
                    // open exercises never enter the environment anyway.
                    if skip.is_some_and(|s| s.contains_key(&idx)) || open_goal(ty, val).is_some() {
                        continue;
                    }
                    let mut hovers = Vec::new();
                    if let Ok(decl) = build_def(
                        &mut builder,
                        name,
                        universe,
                        ty,
                        val,
                        &known_universes,
                        &mut hovers,
                    ) {
                        let _ = builder.add_declar(decl);
                        known_universes.insert(name.clone(), universe.clone());
                    }
                    continue;
                }
                if let Some(err) = skipped(
                    skip,
                    &mut out.errors,
                    idx,
                    DeclKind::Definition,
                    Some(name.clone()),
                    *span,
                ) {
                    decl_states.push(err);
                    continue;
                }
                if let Some((goal, binders)) = open_goal(ty, val) {
                    ops.push(PendingOp::OpenExercise {
                        name: Some(name.clone()),
                        kind: DeclKind::Definition,
                        goal: Some(goal),
                        binders,
                        span: *span,
                        cmd: idx,
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
                                idx,
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
                            cmd: idx,
                        });
                        cmd_hovers.push(CmdHover {
                            env_at: env_after,
                            nodes: hovers,
                            cmd: idx,
                        });
                    }
                    Err(e) => {
                        out.errors.push(e.clone());
                        decl_states.push(failed_state(
                            DeclKind::Definition,
                            Some(name.clone()),
                            *span,
                            e,
                            idx,
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
                if trusted {
                    if skip.is_some_and(|s| s.contains_key(&idx)) || open_goal(ty, val).is_some() {
                        continue;
                    }
                    let mut hovers = Vec::new();
                    if let Ok(decl) = build_theorem(
                        &mut builder,
                        name,
                        universe,
                        ty,
                        val,
                        &known_universes,
                        &mut hovers,
                    ) {
                        let _ = builder.add_declar(decl);
                        known_universes.insert(name.clone(), universe.clone());
                    }
                    continue;
                }
                if let Some(err) = skipped(
                    skip,
                    &mut out.errors,
                    idx,
                    DeclKind::Theorem,
                    Some(name.clone()),
                    *span,
                ) {
                    decl_states.push(err);
                    continue;
                }
                if let Some((goal, binders)) = open_goal(ty, val) {
                    ops.push(PendingOp::OpenExercise {
                        name: Some(name.clone()),
                        kind: DeclKind::Theorem,
                        goal: Some(goal),
                        binders,
                        span: *span,
                        cmd: idx,
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
                                idx,
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
                            cmd: idx,
                        });
                        cmd_hovers.push(CmdHover {
                            env_at: env_after,
                            nodes: hovers,
                            cmd: idx,
                        });
                    }
                    Err(e) => {
                        out.errors.push(e.clone());
                        decl_states.push(failed_state(
                            DeclKind::Theorem,
                            Some(name.clone()),
                            *span,
                            e,
                            idx,
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
                if trusted {
                    if skip.is_some_and(|s| s.contains_key(&idx)) {
                        continue;
                    }
                    let mut hovers = Vec::new();
                    if let Ok(decl) = build_axiom(
                        &mut builder,
                        name,
                        universe,
                        ty,
                        &known_universes,
                        &mut hovers,
                    ) {
                        let _ = builder.add_declar(decl);
                        known_universes.insert(name.clone(), universe.clone());
                    }
                    continue;
                }
                if let Some(err) = skipped(
                    skip,
                    &mut out.errors,
                    idx,
                    DeclKind::Axiom,
                    Some(name.clone()),
                    *span,
                ) {
                    decl_states.push(err);
                    continue;
                }
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
                                idx,
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
                            cmd: idx,
                        });
                        cmd_hovers.push(CmdHover {
                            env_at: env_after,
                            nodes: hovers,
                            cmd: idx,
                        });
                    }
                    Err(e) => {
                        out.errors.push(e.clone());
                        decl_states.push(failed_state(
                            DeclKind::Axiom,
                            Some(name.clone()),
                            *span,
                            e,
                            idx,
                        ));
                    }
                }
            }
            Command::Example { ty, val, span } => {
                if trusted {
                    if skip.is_some_and(|s| s.contains_key(&idx)) || open_goal(ty, val).is_some() {
                        continue;
                    }
                    example_idx += 1;
                    let internal_name = format!("_example_{example_idx}");
                    let mut hovers = Vec::new();
                    if let Ok(decl) = build_example(
                        &mut builder,
                        &internal_name,
                        ty,
                        val,
                        &known_universes,
                        &mut hovers,
                    ) {
                        let _ = builder.add_declar(decl);
                    }
                    continue;
                }
                if let Some(err) =
                    skipped(skip, &mut out.errors, idx, DeclKind::Example, None, *span)
                {
                    decl_states.push(err);
                    continue;
                }
                if let Some((goal, binders)) = open_goal(ty, val) {
                    ops.push(PendingOp::OpenExercise {
                        name: None,
                        kind: DeclKind::Example,
                        goal: Some(goal),
                        binders,
                        span: *span,
                        cmd: idx,
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
                            decl_states.push(failed_state(
                                DeclKind::Example,
                                None,
                                *span,
                                err,
                                idx,
                            ));
                            continue;
                        }
                        let env_after = builder.declaration_count();
                        ops.push(PendingOp::Decl {
                            name: None,
                            kind: DeclKind::Example,
                            declar: decl,
                            span: *span,
                            cmd: idx,
                        });
                        cmd_hovers.push(CmdHover {
                            env_at: env_after,
                            nodes: hovers,
                            cmd: idx,
                        });
                    }
                    Err(e) => {
                        out.errors.push(e.clone());
                        decl_states.push(failed_state(DeclKind::Example, None, *span, e, idx));
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
                if trusted {
                    // The whole block is the minimal incremental unit: it was
                    // kernel-validated together when first checked.
                    if skip.is_some_and(|s| s.contains_key(&idx)) {
                        continue;
                    }
                    let mut hovers = Vec::new();
                    let mut built: Vec<Declar<'_>> = Vec::new();
                    if install_inductive_block(
                        &mut builder,
                        &mut known_universes,
                        name,
                        ty,
                        constructors,
                        recursor.as_ref(),
                        iota_rules,
                        &mut hovers,
                        &mut built,
                    )
                    .is_ok()
                    {
                        built_inductives.extend(built);
                    }
                    continue;
                }
                if let Some(err) = skipped(
                    skip,
                    &mut out.errors,
                    idx,
                    DeclKind::Inductive,
                    Some(name.clone()),
                    *span,
                ) {
                    decl_states.push(err);
                    continue;
                }
                let mut hovers = Vec::new();
                let mut built: Vec<Declar<'_>> = Vec::new();
                match install_inductive_block(
                    &mut builder,
                    &mut known_universes,
                    name,
                    ty,
                    constructors,
                    recursor.as_ref(),
                    iota_rules,
                    &mut hovers,
                    &mut built,
                ) {
                    Ok(()) => {
                        built_inductives.extend(built.iter().cloned());
                        ops.push(PendingOp::InductiveBlock {
                            name: name.clone(),
                            declars: built,
                            span: *span,
                            cmd: idx,
                        });
                        cmd_hovers.push(CmdHover {
                            env_at: builder.declaration_count(),
                            nodes: hovers,
                            cmd: idx,
                        });
                    }
                    Err(e) => {
                        out.errors.push(e.clone());
                        decl_states.push(failed_state(
                            DeclKind::Inductive,
                            Some(name.clone()),
                            *span,
                            e,
                            idx,
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
                    None,
                ) {
                    Ok(e) => {
                        ops.push(PendingOp::Check {
                            expr: e,
                            env_at: env_before,
                            span: expr.span(),
                            cmd: idx,
                        });
                        cmd_hovers.push(CmdHover {
                            env_at: env_before,
                            nodes: hovers,
                            cmd: idx,
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
                    None,
                ) {
                    Ok(e) => {
                        ops.push(PendingOp::Reduce {
                            expr: e,
                            env_at: env_before,
                            span: expr.span(),
                            cmd: idx,
                        });
                        cmd_hovers.push(CmdHover {
                            env_at: env_before,
                            nodes: hovers,
                            cmd: idx,
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
                    cmd: idx,
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
                binders,
                span,
                cmd,
            } => {
                out.push_event(cmd, CheckEvent::ExerciseOpen { name: name.clone() });
                decl_states.push(DeclState {
                    kind,
                    name,
                    span,
                    status: DeclStatus::Open,
                    error: None,
                    goal,
                    binders,
                    cmd,
                });
            }
            PendingOp::Decl {
                name,
                kind,
                declar,
                span,
                cmd,
            } => {
                kernel_checks += 1;
                match env.try_check_declar(&declar) {
                    Ok(()) => {
                        match kind {
                            DeclKind::Example => out.push_event(cmd, CheckEvent::ExampleChecked),
                            _ => {
                                if let Some(n) = &name {
                                    out.push_event(
                                        cmd,
                                        CheckEvent::DeclarationChecked { name: n.clone() },
                                    );
                                } else {
                                    out.push_event(cmd, CheckEvent::ExampleChecked);
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
                            binders: Vec::new(),
                            cmd,
                        });
                    }
                    Err(e) => {
                        let msg = format!("{e}");
                        let err = if msg.contains("kernel error") || msg.contains("kernel error:") {
                            CompileError::kernel(ErrorKind::KernelInternal, msg, span)
                        } else {
                            let mut err =
                                CompileError::kernel(ErrorKind::KernelRejected, msg, span);
                            if let Some((expected, actual)) = parse_def_eq_mismatch(&err.message) {
                                err.message =
                                    format!("类型不匹配：期望 `{expected}`，实际是 `{actual}`");
                                err.expected = Some(expected);
                                err.actual = Some(actual);
                            }
                            err
                        };
                        failed_cmds.insert(cmd, err.clone());
                        out.errors.push(err.clone());
                        decl_states.push(failed_state(kind, name, span, err, cmd));
                    }
                }
            }
            PendingOp::InductiveBlock {
                name,
                declars,
                span,
                cmd,
            } => {
                let mut failure = None;
                for declar in &declars {
                    kernel_checks += 1;
                    if let Err(e) = env.try_check_declar(declar) {
                        let msg = format!("{e}");
                        let mut err = if msg.contains("kernel error") {
                            CompileError::kernel(ErrorKind::KernelInternal, msg, span)
                        } else {
                            CompileError::kernel(ErrorKind::KernelRejected, msg, span)
                        };
                        if let Some((expected, actual)) = parse_def_eq_mismatch(&err.message) {
                            err.message =
                                format!("类型不匹配：期望 `{expected}`，实际是 `{actual}`");
                            err.expected = Some(expected);
                            err.actual = Some(actual);
                        }
                        failure = Some(err);
                        break;
                    }
                }
                match failure {
                    None => {
                        out.push_event(cmd, CheckEvent::DeclarationChecked { name: name.clone() });
                        decl_states.push(DeclState {
                            kind: DeclKind::Inductive,
                            name: Some(name),
                            span,
                            status: DeclStatus::Checked,
                            error: None,
                            goal: None,
                            binders: Vec::new(),
                            cmd,
                        });
                    }
                    Some(err) => {
                        failed_cmds.insert(cmd, err.clone());
                        out.errors.push(err.clone());
                        decl_states.push(failed_state(
                            DeclKind::Inductive,
                            Some(name),
                            span,
                            err,
                            cmd,
                        ));
                    }
                }
            }
            PendingOp::Check {
                expr,
                env_at,
                span,
                cmd,
            } => {
                env.with_tc(EnvLimit::ByIndex(env_at), |tc| {
                    let ty = tc.infer_closed_type(expr);
                    let text = tc.with_pp(|pp| pp.pp_expr(ty));
                    out.push_event(cmd, CheckEvent::TypeChecked { text, span });
                });
            }
            PendingOp::Reduce {
                expr,
                env_at,
                span,
                cmd,
            } => {
                env.with_tc(EnvLimit::ByIndex(env_at), |tc| {
                    let reduced = tc.reduce_closed(expr);
                    let text = tc.with_pp(|pp| pp.pp_expr(reduced));
                    out.push_event(cmd, CheckEvent::Reduced { text, span });
                });
            }
            PendingOp::Print {
                name,
                ptr,
                span,
                cmd,
            } => {
                let printed = env.with_pp(|pp| pp.pp_declar(ptr));
                match printed {
                    Some(text) => out.push_event(cmd, CheckEvent::Printed { name, text }),
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
        let mut hover_cmds = Vec::new();
        resolve_hovers(&env, cmd_hovers, &mut report.hovers, &mut hover_cmds);
        report.hover_cmds = hover_cmds;
    }
    let _ = built_inductives;
    (out, report, failed_cmds, kernel_checks)
}

/// Build the failed-state placeholder for a command skipped in pass 2
/// (it was kernel-rejected in pass 1; keep that error verbatim).
fn skipped(
    skip: Option<&KernelFailed>,
    errors: &mut Vec<CompileError>,
    idx: usize,
    kind: DeclKind,
    name: Option<String>,
    span: Span,
) -> Option<DeclState> {
    let error = skip?.get(&idx)?;
    errors.push(error.clone());
    Some(failed_state(kind, name, span, error.clone(), idx))
}

pub(crate) fn failed_state(
    kind: DeclKind,
    name: Option<String>,
    span: Span,
    error: CompileError,
    cmd: usize,
) -> DeclState {
    DeclState {
        kind,
        name,
        span,
        status: DeclStatus::Failed,
        error: Some(error),
        goal: None,
        binders: Vec::new(),
        cmd,
    }
}

/// Infer a type per recorded sub-expression (in its binder scope) and render
/// it as text. Panics (kernel rejection on intermediate sub-terms) are caught
/// per node so one bad sub-term cannot kill the hover map.
pub(crate) fn resolve_hovers(
    env: &sokonanoda::util::ExportFile<'_>,
    cmd_hovers: Vec<CmdHover<'_>>,
    out: &mut Vec<HoverType>,
    out_cmds: &mut Vec<usize>,
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
                out_cmds.push(cmd.cmd);
            }
        }
    }
    std::panic::set_hook(previous_hook);
}

/// Render an AST expression back to source text (used for open-exercise goals).
pub fn render_expr(expr: &Expr) -> String {
    crate::proof::render_expr(expr)
}

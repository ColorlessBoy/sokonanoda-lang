//! 编译入口：`compile_fol`/`check_document`、待执行操作与 hover 解析。

use super::elab::{
    build_axiom, build_def, build_example, build_theorem, elab_expr, install_inductive_block,
    ElabCtx, ElabScope, HoverNode, InductiveTable, UnivMap,
};
use super::error::{parse_def_eq_mismatch, refine_kernel_kind, CompileError, ErrorKind};
use super::event::{CheckEvent, CompileOutput};
use super::goals::{expr_has_hole, open_goal, GoalTemplates};
use super::prelude::{install_eq_prelude, install_prelude, CompileOptions, PreludeMode};
use super::report::{
    ByGoalState, ByStepState, DeclKind, DeclState, DeclStatus, DocumentReport, GoalBinder,
    HoverType, ResolvedTarget, SubGoal,
};
use crate::{Command, Expr, FolFile, Span};
use sokonanoda::builder::EnvBuilder;
use sokonanoda::env::{Declar, EnvLimit};
use sokonanoda::util::{Config, ExportFile, ExprPtr, NamePtr};
use std::collections::HashMap;

pub(crate) enum PendingOp<'a> {
    Decl {
        name: Option<String>,
        kind: DeclKind,
        declar: Declar<'a>,
        span: Span,
        cmd: usize,
        /// Per-tactic states for a `by` value (empty otherwise), carried to
        /// the `DeclState` for `soko/stateAt`.
        by_steps: Vec<ByStepState>,
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
        /// The declaration's universe parameters (`{u}` …); the goal view and
        /// tactic judging need them to synthesize a judge declaration for
        /// `Sort u` goals.
        universe: Vec<String>,
        /// Elaborated declared type (kernel expr) — rendered into
        /// `DeclState.ty_text` during the check phase.
        declared_ty: Option<ExprPtr<'a>>,
        goal: Option<String>,
        binders: Vec<GoalBinder>,
        holes: Vec<Span>,
        sub_goals: Vec<SubGoal>,
        refine_template: Option<String>,
        span: Span,
        cmd: usize,
        /// Per-tactic states for a `by` value (empty otherwise).
        by_steps: Vec<ByStepState>,
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

/// Incremental trust plan (I8, docs/design/i8-i9.md): commands `[0, before)`
/// were already kernel-checked in a previous session with the identical text,
/// so this run elaborates them into the environment but does NOT re-check
/// them — their states/hovers/events are reused from the session cache.
///
/// `prev_signatures`/`text_unchanged`/`allow_cutoff` power **early cutoff**:
/// after re-elaborating the changed command, if the accumulated
/// environment contribution `[before, j)` matches the previous session's and
/// every later command's text is unchanged, commands `[j, n)` are reused
/// verbatim and never kernel-checked (I8 依赖精确化).
pub(crate) struct TrustPlan {
    pub before: usize,
    /// Previous session's per-command signature (length = previous command
    /// count). `None` for open/failed/non-declaration commands.
    pub prev_signatures: Vec<Option<String>>,
    /// `true` for a command index whose source text is byte-identical to the
    /// previous session's command at the same index.
    pub text_unchanged: Vec<bool>,
    /// Early cutoff is only attempted when the caller established that all
    /// commands after `before` keep their text (single-edit alignment).
    pub allow_cutoff: bool,
}

/// One `run_pass` result, including the early-cutoff bookkeeping.
struct PassResult {
    out: CompileOutput,
    report: DocumentReport,
    failed: KernelFailed,
    checks: usize,
    /// Per-command environment signatures (length = current command count).
    /// Only populated for incremental runs; `None` means "no env contribution
    /// or the command was past the cutoff".
    sigs: Vec<Option<String>>,
    /// First command index whose previous snapshots may be reused; `n` when
    /// the whole suffix was (re)processed. Fresh results cover `[0, cutoff)`.
    cutoff: usize,
}

/// The command index a pending op belongs to.
fn op_cmd(op: &PendingOp<'_>) -> usize {
    match op {
        PendingOp::Decl { cmd, .. }
        | PendingOp::InductiveBlock { cmd, .. }
        | PendingOp::OpenExercise { cmd, .. }
        | PendingOp::Check { cmd, .. }
        | PendingOp::Reduce { cmd, .. }
        | PendingOp::Print { cmd, .. } => *cmd,
    }
}

/// A declaration's discriminative keyword (part of its signature so that an
/// `axiom` and a `def` of the same name/type never compare equal).
fn declar_keyword(declar: &Declar<'_>) -> &'static str {
    match declar {
        Declar::Axiom { .. } => "axiom",
        Declar::Quot { .. } => "quot",
        Declar::Theorem { .. } => "theorem",
        Declar::Definition { .. } => "def",
        Declar::Opaque { .. } => "opaque",
        Declar::Inductive(_) => "inductive",
        Declar::Constructor(_) => "ctor",
        Declar::Recursor(_) => "recursor",
    }
}

/// Render a declaration into a canonical, environment-observable signature:
/// kind, name, declared universes, type, body (for reducible/opaque
/// declarations) and iota rules (for recursors). Two commands with equal
/// signatures are interchangeable for every later command — the soundness
/// contract for I8 early cutoff. The body is required because delta unfolding
/// is observable (a changed def body with an unchanged type MUST invalidate
/// dependents); theorems/opaques carry their bodies too (conservative:
/// `#print` / `#reduce` can observe them).
///
/// The encoding uses the kernel's structural `debug_print` rather than the
/// pretty printer: every application node, universe instantiation, declared
/// universe parameter (even an unused one) and iota rule appears verbatim, so
/// no notation/elision can make two different declarations compare equal.
/// Binder *names* are included, so alpha-renaming or binder-style differences
/// can only ever produce *false* mismatches (extra rechecks), never false
/// matches.
fn declar_signature(env: &mut ExportFile<'_>, declar: &Declar<'_>) -> String {
    let info = *declar.info();
    let mut out = String::new();
    out.push_str(declar_keyword(declar));
    out.push(' ');
    out.push_str(&env.with_ctx(|ctx, _cache, _bump| format!("{:?}", ctx.debug_print(&info))));
    match declar {
        Declar::Definition { val, hint, .. } => {
            out.push_str(" := ");
            out.push_str(
                &env.with_ctx(|ctx, _cache, _bump| format!("{:?}", ctx.debug_print(*val))),
            );
            out.push_str(&format!(" [{hint:?}]"));
        }
        Declar::Theorem { val, .. } | Declar::Opaque { val, .. } => {
            out.push_str(" := ");
            out.push_str(
                &env.with_ctx(|ctx, _cache, _bump| format!("{:?}", ctx.debug_print(*val))),
            );
        }
        Declar::Constructor(data) => {
            out.push_str(&env.with_ctx(|ctx, _cache, _bump| {
                format!(
                    " |ctor-of {:?} idx={} p={} f={}",
                    ctx.debug_print(data.inductive_name),
                    data.ctor_idx,
                    data.num_params,
                    data.num_fields
                )
            }));
        }
        Declar::Recursor(data) => {
            out.push_str(&format!(
                " |meta p={} i={} m={} n={} k={}",
                data.num_params, data.num_indices, data.num_motives, data.num_minors, data.is_k
            ));
            out.push_str(&env.with_ctx(|ctx, _cache, _bump| {
                format!(" |inds {:?}", ctx.debug_print(&data.all_inductives[..]))
            }));
            out.push_str(&env.with_ctx(|ctx, _cache, _bump| {
                format!(" |rules {:?}", ctx.debug_print(&data.rec_rules[..]))
            }));
        }
        _ => {}
    }
    out
}

/// Signature of an `inductive … end` block: every declaration it installs
/// (spine, constructors, recursor + iota rules), in order.
fn inductive_signature(env: &mut ExportFile<'_>, declars: &[Declar<'_>]) -> String {
    let mut out = String::new();
    for declar in declars {
        out.push('\n');
        out.push_str(&declar_signature(env, declar));
    }
    out
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

/// 值位若是 `by` 块，先用引擎降级成 lambda AST（可能带尾部 `sorry`）；
/// 否则原样 clone。返回降级后的值位 + 引擎记录的 per-tactic 状态
/// （非 by 块为空），交给既有 `open_goal`/`build_*` 分流。
/// `src` 为文件原文、`span_start` 为声明起点——只有真是 `by` 块才切片
/// （judge 合成的文件 src 为空，普通声明不触发切片）。
fn lower_by_val(
    ty: &Expr,
    val: &Expr,
    src: &str,
    span_start: usize,
    options: &CompileOptions,
) -> Result<(Expr, Vec<crate::by::ByStep>), CompileError> {
    if let Some((binders, by)) = crate::by::split_by_value(val) {
        let prefix = src.get(..span_start).unwrap_or("");
        crate::by::run_by(ty, by, &binders, prefix, options).map(|o| (o.expr, o.steps))
    } else {
        Ok((val.clone(), Vec::new()))
    }
}

/// 值位降低产物：`(值, per-tactic 状态)`。`by` 块走 tactic 引擎；其它值原样
/// 透传（per-tactic 状态为空）。
pub(crate) type LoweredValue = (Expr, Vec<crate::by::ByStep>);

/// 值位是 `by` 块时走 tactic 引擎；其它值原样透传。
fn lower_value(
    ty: &Expr,
    val: &Expr,
    src: &str,
    span_start: usize,
    options: &CompileOptions,
) -> Result<LoweredValue, CompileError> {
    lower_by_val(ty, val, src, span_start, options)
}

/// 引擎的 per-step 状态 → 报告层 wire 形状（binder 类型渲染成文本）。
fn by_step_states(steps: &[crate::by::ByStep]) -> Vec<ByStepState> {
    steps
        .iter()
        .map(|s| ByStepState {
            span: s.span,
            goals: s
                .goals
                .iter()
                .map(|g| ByGoalState {
                    ty: g.ty.clone(),
                    binders: g
                        .binders
                        .iter()
                        .map(|b| GoalBinder {
                            name: b.name.clone(),
                            ty: b.ty.as_deref().map(render_expr).unwrap_or_default(),
                        })
                        .collect(),
                })
                .collect(),
        })
        .collect()
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
    let pass = run_pass(file, options, collect, None, None);
    let mut out = pass.out;
    out.stats.kernel_checks = pass.checks;
    if std::env::var("SOKO_DEBUG_PASS1").is_ok() {
        for (idx, err) in &pass.failed {
            eprintln!("pass1 failed cmd {idx}: {} ({:?})", err.message, err.kind);
        }
    }
    if pass.failed.is_empty() {
        return (out, pass.report);
    }
    let pass2 = run_pass(file, options, collect, Some(&pass.failed), None);
    let mut out2 = pass2.out;
    out2.stats.kernel_checks = pass.checks + pass2.checks;
    (out2, pass2.report)
}

/// Incremental entry (I8): `trust` marks the reusable prefix `[0, before)`;
/// `prefix_failures` maps trusted command indices to their cached failures —
/// those names stay free (check-then-add) and their states are owned by the
/// session cache, so this pass neither re-checks nor re-reports them.
///
/// Returns `(output, report, kernel_checks, signatures, cutoff)`. Fresh
/// results cover `[0, cutoff)`; commands `[cutoff, n)` can be reused from the
/// session cache (early cutoff). Early cutoff is abandoned for good as soon
/// as a currently-processed declaration is kernel-rejected (check-then-add
/// makes pass 1's environment provisional), in which case pass 2 reprocesses
/// the whole suffix with cutoff disabled.
pub(crate) fn run_incremental(
    file: &FolFile,
    options: &CompileOptions,
    trust: &TrustPlan,
    prefix_failures: &KernelFailed,
) -> (
    CompileOutput,
    DocumentReport,
    usize,
    Vec<Option<String>>,
    usize,
) {
    let pass1 = run_pass(file, options, true, Some(prefix_failures), Some(trust));
    if pass1.failed.is_empty() {
        let mut out = pass1.out;
        out.stats.kernel_checks = pass1.checks;
        return (out, pass1.report, pass1.checks, pass1.sigs, pass1.cutoff);
    }
    if std::env::var("SOKO_DEBUG_PASS1").is_ok() {
        for (idx, err) in &pass1.failed {
            eprintln!("pass1 failed cmd {idx}: {} ({:?})", err.message, err.kind);
        }
    }
    let mut skip2 = prefix_failures.clone();
    for (idx, err) in &pass1.failed {
        skip2.insert(*idx, err.clone());
    }
    // Pass 2 establishes the true check-then-add environment; cutoff is
    // disabled because pass 1's provisional environment invalidated any
    // signature comparison it might have made.
    let trust2 = TrustPlan {
        before: trust.before,
        prev_signatures: Vec::new(),
        text_unchanged: Vec::new(),
        allow_cutoff: false,
    };
    let pass2 = run_pass(file, options, true, Some(&skip2), Some(&trust2));
    let checks = pass1.checks + pass2.checks;
    let mut out = pass2.out;
    out.stats.kernel_checks = checks;
    (out, pass2.report, checks, pass2.sigs, pass2.cutoff)
}

type KernelFailed = HashMap<usize, CompileError>;

fn run_pass(
    file: &FolFile,
    options: &CompileOptions,
    collect: bool,
    skip: Option<&KernelFailed>,
    trust: Option<&TrustPlan>,
) -> PassResult {
    let arena = stumpalo::Arena::new();
    let mut builder = EnvBuilder::new(arena.as_arena_ref(), Config::default());
    let mut known_universes: HashMap<String, Vec<String>> = HashMap::new();
    let mut inductives = InductiveTable::new();
    match options.prelude {
        PreludeMode::Bare => {}
        PreludeMode::Full => {
            let explicit_nat = file.commands.iter().any(
                |command| matches!(command, Command::InductiveBlock { name, .. } if name == "Nat"),
            );
            if !explicit_nat {
                // Nat 作为受信任的归纳块安装，同时把 Nat/Nat.zero/Nat.succ/
                // Nat.rec 登记进 `known` 与 `match` 的 InductiveTable。
                install_prelude(&mut builder, &mut known_universes, &mut inductives);
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
    let templates = GoalTemplates::new_for(file, options);

    let mut failed_cmds: KernelFailed = HashMap::new();
    let mut built_inductives: Vec<Declar<'_>> = Vec::new();
    let mut kernel_checks = 0usize;
    for (idx, command) in file.commands.iter().enumerate() {
        let trusted = trust.is_some_and(|t| idx < t.before);
        let env_before = builder.declaration_count();
        // `match` 的宇宙查询用前缀源码（与 `by` 同一条合成 `#check` 路线）。
        let prefix_src = file.src.get(..command.span().start.offset).unwrap_or("");
        match command {
            Command::Def {
                name,
                universe,
                ty,
                val,
                span,
            } => {
                let elab_ctx = ElabCtx {
                    prefix_src,
                    options,
                    inductives: &inductives,
                };
                let lowered = match lower_value(ty, val, &file.src, span.start.offset, options) {
                    Ok(v) => v,
                    Err(e) => {
                        out.errors.push(e.clone());
                        decl_states.push(failed_state(
                            DeclKind::Definition,
                            Some(name.clone()),
                            *span,
                            e,
                            idx,
                        ));
                        continue;
                    }
                };
                let val = &lowered.0;
                let by_steps = by_step_states(&lowered.1);
                if trusted {
                    // Trusted prefix: keep the environment, skip the kernel.
                    // Cached failures keep the name free (check-then-add);
                    // open exercises never enter the environment anyway.
                    if skip.is_some_and(|s| s.contains_key(&idx))
                        || open_goal(ty, val, &templates).is_some()
                    {
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
                        &elab_ctx,
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
                let open_info = open_goal(ty, val, &templates);
                if let Some(info) = open_info {
                    let declared_ty = elab_expr(
                        &mut builder,
                        ty,
                        &mut ElabScope::new(),
                        &no_universe,
                        &known_universes,
                        &mut Vec::new(),
                        None,
                        None,
                        &elab_ctx,
                    )
                    .inspect(|_| {
                        // hover 行也要：类型子表达式进 hover 表
                        cmd_hovers.push(CmdHover {
                            env_at: builder.declaration_count(),
                            nodes: Vec::new(),
                            cmd: idx,
                        });
                    })
                    .ok();
                    let _ = &declared_ty;
                    ops.push(PendingOp::OpenExercise {
                        name: Some(name.clone()),
                        kind: DeclKind::Definition,
                        universe: universe.clone(),
                        declared_ty,
                        goal: Some(info.goal),
                        binders: info.binders,
                        holes: info.holes,
                        sub_goals: info.sub_goals,
                        refine_template: info.refine_template,
                        by_steps: by_steps.clone(),
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
                    &elab_ctx,
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
                            by_steps: by_steps.clone(),
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
                let elab_ctx = ElabCtx {
                    prefix_src,
                    options,
                    inductives: &inductives,
                };
                let lowered = match lower_value(ty, val, &file.src, span.start.offset, options) {
                    Ok(v) => v,
                    Err(e) => {
                        out.errors.push(e.clone());
                        decl_states.push(failed_state(
                            DeclKind::Theorem,
                            Some(name.clone()),
                            *span,
                            e,
                            idx,
                        ));
                        continue;
                    }
                };
                let val = &lowered.0;
                let by_steps = by_step_states(&lowered.1);
                if trusted {
                    if skip.is_some_and(|s| s.contains_key(&idx))
                        || open_goal(ty, val, &templates).is_some()
                    {
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
                        &elab_ctx,
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
                // 尾部复用 + fallback（I13-S5b）：open_goal 的 spine 走查
                // 无法分解时（如超量应用、def 展开间接调用），如果值里有
                // 洞 → 生成 **generic open exercise**（整值 = 一个洞，目标 =
                // 声明类型）。学习者看到的是一个可填充的练习而不是报错。
                let open_info = open_goal(ty, val, &templates);
                let open_info = match open_info {
                    Some(info) => Some(info),
                    None if expr_has_hole(val) => {
                        // spine 走查无法分解，但值有洞 → generic open exercise
                        Some(super::goals::OpenGoalInfo {
                            goal: render_expr(ty),
                            binders: Vec::new(),
                            holes: vec![val.span()],
                            sub_goals: Vec::new(),
                            refine_template: None,
                        })
                    }
                    _ => None,
                };
                if let Some(info) = open_info {
                    let declared_ty = elab_expr(
                        &mut builder,
                        ty,
                        &mut ElabScope::new(),
                        &no_universe,
                        &known_universes,
                        &mut Vec::new(),
                        None,
                        None,
                        &elab_ctx,
                    )
                    .ok();
                    ops.push(PendingOp::OpenExercise {
                        name: Some(name.clone()),
                        kind: DeclKind::Theorem,
                        universe: universe.clone(),
                        declared_ty,
                        goal: Some(info.goal),
                        binders: info.binders,
                        holes: info.holes,
                        sub_goals: info.sub_goals,
                        refine_template: info.refine_template,
                        by_steps: by_steps.clone(),
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
                    &elab_ctx,
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
                            by_steps: by_steps.clone(),
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
                let elab_ctx = ElabCtx {
                    prefix_src,
                    options,
                    inductives: &inductives,
                };
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
                        &elab_ctx,
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
                    &elab_ctx,
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
                            by_steps: Vec::new(),
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
                let elab_ctx = ElabCtx {
                    prefix_src,
                    options,
                    inductives: &inductives,
                };
                let lowered = match lower_value(ty, val, &file.src, span.start.offset, options) {
                    Ok(v) => v,
                    Err(e) => {
                        out.errors.push(e.clone());
                        decl_states.push(failed_state(DeclKind::Example, None, *span, e, idx));
                        continue;
                    }
                };
                let val = &lowered.0;
                let by_steps = by_step_states(&lowered.1);
                if trusted {
                    if skip.is_some_and(|s| s.contains_key(&idx))
                        || open_goal(ty, val, &templates).is_some()
                    {
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
                        &elab_ctx,
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
                // 尾部复用 + fallback（I13-S5b）：open_goal 的 spine 走查
                // 无法分解时（如超量应用、def 展开间接调用），如果值里有
                // 洞 → 生成 **generic open exercise**（整值 = 一个洞，目标 =
                // 声明类型）。学习者看到的是一个可填充的练习而不是报错。
                let open_info = open_goal(ty, val, &templates);
                let open_info = match open_info {
                    Some(info) => Some(info),
                    None if expr_has_hole(val) => {
                        // spine 走查无法分解，但值有洞 → generic open exercise
                        Some(super::goals::OpenGoalInfo {
                            goal: render_expr(ty),
                            binders: Vec::new(),
                            holes: vec![val.span()],
                            sub_goals: Vec::new(),
                            refine_template: None,
                        })
                    }
                    _ => None,
                };
                if let Some(info) = open_info {
                    let declared_ty = elab_expr(
                        &mut builder,
                        ty,
                        &mut ElabScope::new(),
                        &no_universe,
                        &known_universes,
                        &mut Vec::new(),
                        None,
                        None,
                        &elab_ctx,
                    )
                    .ok();
                    ops.push(PendingOp::OpenExercise {
                        name: None,
                        kind: DeclKind::Example,
                        universe: Vec::new(),
                        declared_ty,
                        goal: Some(info.goal),
                        binders: info.binders,
                        holes: info.holes,
                        sub_goals: info.sub_goals,
                        refine_template: info.refine_template,
                        by_steps: by_steps.clone(),
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
                    &elab_ctx,
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
                            by_steps: by_steps.clone(),
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
                        &mut inductives,
                        prefix_src,
                        options,
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
                    &mut inductives,
                    prefix_src,
                    options,
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
                    None,
                    &ElabCtx {
                        prefix_src,
                        options,
                        inductives: &inductives,
                    },
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
                    None,
                    &ElabCtx {
                        prefix_src,
                        options,
                        inductives: &inductives,
                    },
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

    let n = file.commands.len();
    let want_sigs = trust.is_some();
    let before = trust.map_or(0, |t| t.before);
    let mut allow_cutoff = trust.is_some_and(|t| t.allow_cutoff);
    let old_sigs: &[Option<String>] = trust.map_or(&[][..], |t| t.prev_signatures.as_slice());
    let text_unchanged: &[bool] = trust.map_or(&[][..], |t| t.text_unchanged.as_slice());
    let mut sigs: Vec<Option<String>> = vec![None; n];
    let mut acc_new: Vec<Option<String>> = Vec::new();
    let mut acc_old: Vec<Option<String>> = Vec::new();
    let mut cutoff = n;

    let mut ops = ops.into_iter().peekable();
    for (j, sig_slot) in sigs.iter_mut().enumerate() {
        // Early cutoff (I8 依赖精确化): the environment contribution of
        // `[before, j)` matches the previous session AND command `j`'s text is
        // unchanged, so `[j, n)` may be reused from the session cache without
        // re-checking. `j > before` because the changed command itself must
        // always be recompiled (its own spans/hovers may have moved).
        if allow_cutoff
            && j > before
            && text_unchanged.get(j).copied().unwrap_or(false)
            && acc_new == acc_old
        {
            cutoff = j;
            break;
        }
        let op = match ops.peek() {
            Some(o) if op_cmd(o) == j => ops.next(),
            _ => None,
        };
        let mut contribution = if want_sigs {
            op.as_ref().and_then(|op| match op {
                PendingOp::Decl { declar, .. } => Some(declar_signature(&mut env, declar)),
                PendingOp::InductiveBlock { declars, .. } => {
                    Some(inductive_signature(&mut env, declars))
                }
                _ => None,
            })
        } else {
            None
        };
        // A kernel-rejected declaration never enters the environment
        // (check-then-add), so its environment contribution is empty.
        let mut op_failed = false;
        if let Some(op) = op {
            match op {
                PendingOp::OpenExercise {
                    name,
                    kind,
                    universe,
                    declared_ty,
                    goal,
                    binders,
                    holes,
                    sub_goals,
                    refine_template,
                    by_steps,
                    span,
                    cmd,
                } => {
                    out.push_event(cmd, CheckEvent::ExerciseOpen { name: name.clone() });
                    let ty_text = declared_ty.and_then(|ty| {
                        quiet_catch(|| {
                            env.with_tc(EnvLimit::Empty, |tc| tc.with_pp(|pp| pp.pp_expr(ty)))
                        })
                        .ok()
                    });
                    decl_states.push(DeclState {
                        kind,
                        name,
                        span,
                        status: DeclStatus::Open,
                        error: None,
                        goal,
                        binders,
                        cmd,
                        universe,
                        holes,
                        sub_goals,
                        refine_template,
                        by_steps,
                        hints: Vec::new(),
                        ty_text,
                    });
                }
                PendingOp::Decl {
                    name,
                    kind,
                    declar,
                    by_steps,
                    span,
                    cmd,
                } => {
                    kernel_checks += 1;
                    let ty_text = quiet_catch(|| {
                        env.with_tc(EnvLimit::Empty, |tc| {
                            let ty = declar.info().ty;
                            tc.with_pp(|pp| pp.pp_expr(ty))
                        })
                    });
                    let ty_text = ty_text.ok();
                    match env.try_check_declar(&declar) {
                        Ok(()) => {
                            match kind {
                                DeclKind::Example => {
                                    out.push_event(cmd, CheckEvent::ExampleChecked)
                                }
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
                                universe: Vec::new(),
                                holes: Vec::new(),
                                sub_goals: Vec::new(),
                                refine_template: None,
                                by_steps,
                                hints: Vec::new(),
                                ty_text,
                            });
                        }
                        Err(e) => {
                            let msg = format!("{e}");
                            let mut err = CompileError::kernel(refine_kernel_kind(&msg), msg, span);
                            if let Some((expected, actual)) = parse_def_eq_mismatch(&err.message) {
                                err.message =
                                    format!("类型不匹配：期望 `{expected}`，实际是 `{actual}`");
                                err.expected = Some(expected);
                                err.actual = Some(actual);
                            }
                            // Check-then-add: a rejected declaration makes pass 1's
                            // environment provisional — stop trusting any cutoff.
                            allow_cutoff = false;
                            op_failed = true;
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
                            let mut err = CompileError::kernel(refine_kernel_kind(&msg), msg, span);
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
                            out.push_event(
                                cmd,
                                CheckEvent::DeclarationChecked { name: name.clone() },
                            );
                            decl_states.push(DeclState {
                                kind: DeclKind::Inductive,
                                name: Some(name),
                                span,
                                status: DeclStatus::Checked,
                                error: None,
                                goal: None,
                                binders: Vec::new(),
                                cmd,
                                universe: Vec::new(),
                                holes: Vec::new(),
                                sub_goals: Vec::new(),
                                refine_template: None,
                                by_steps: Vec::new(),
                                hints: Vec::new(),
                                ty_text: None,
                            });
                        }
                        Some(err) => {
                            allow_cutoff = false;
                            op_failed = true;
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
                    // #check/#reduce 直通内核求值路径：panic（如对非函数应用）
                    // 必须降级为诊断，绝不能崩掉编译/LSP 进程。
                    match quiet_catch(|| {
                        env.with_tc(EnvLimit::ByIndex(env_at), |tc| {
                            let ty = tc.infer_closed_type(expr);
                            tc.with_pp(|pp| pp.pp_expr(ty))
                        })
                    }) {
                        Ok(text) => out.push_event(cmd, CheckEvent::TypeChecked { text, span }),
                        Err(msg) => out.errors.push(CompileError::kernel(
                            refine_kernel_kind(&msg),
                            format!("类型检查失败：{msg}"),
                            span,
                        )),
                    }
                }
                PendingOp::Reduce {
                    expr,
                    env_at,
                    span,
                    cmd,
                } => {
                    match quiet_catch(|| {
                        env.with_tc(EnvLimit::ByIndex(env_at), |tc| {
                            let reduced = tc.reduce_closed(expr);
                            tc.with_pp(|pp| pp.pp_expr(reduced))
                        })
                    }) {
                        Ok(text) => out.push_event(cmd, CheckEvent::Reduced { text, span }),
                        Err(msg) => out.errors.push(CompileError::kernel(
                            refine_kernel_kind(&msg),
                            format!("化简失败：{msg}"),
                            span,
                        )),
                    }
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
        if op_failed {
            contribution = None;
        }
        *sig_slot = contribution.clone();
        if j >= before {
            acc_new.push(contribution);
            acc_old.push(old_sigs.get(j).cloned().flatten());
        }
    }

    if collect {
        // Open/failed states are recorded during the command walk while
        // checked states come from the kernel phase; keep source order.
        let mut states = decl_states;
        states.sort_by_key(|d| d.span.start.offset);
        report.decls = states;
        report.errors = out.errors.clone();
        // Name use → definition: top-level targets were recorded with a
        // placeholder span during elaboration; backfill them from the file's
        // name → def-span map (prelude names resolve to nothing).
        let defs = top_level_def_spans(file);
        for cmd in &mut cmd_hovers {
            for node in &mut cmd.nodes {
                if let Some(ResolvedTarget::Declaration { name, .. }) = &node.resolution {
                    let name = name.clone();
                    node.resolution = defs
                        .get(&name)
                        .map(|&span| ResolvedTarget::Declaration { name, span });
                }
            }
        }
        let mut hover_cmds = Vec::new();
        resolve_hovers(&env, cmd_hovers, &mut report.hovers, &mut hover_cmds);
        report.hover_cmds = hover_cmds;
        report.checks = out
            .events
            .iter()
            .zip(out.event_cmds.iter())
            .filter_map(|(event, _)| match event {
                CheckEvent::TypeChecked { text, span } => Some(super::report::CheckInfo {
                    span: *span,
                    text: text.clone(),
                }),
                _ => None,
            })
            .collect();
    }
    // Syntax-level warnings are independent of the kernel pass: compute them
    // once for the whole file so every return path (batch output + report)
    // carries the same list.
    report.warnings = super::warning::collect_warnings(file);
    out.warnings = report.warnings.clone();
    let _ = built_inductives;
    PassResult {
        out,
        report,
        failed: failed_cmds,
        checks: kernel_checks,
        sigs,
        cutoff,
    }
}

/// Every top-level name this file declares, mapped to the span of the command
/// that first defines it (defs/axioms/inductives, plus constructors and
/// recursors of inductive blocks). Prelude names are absent by construction.
fn top_level_def_spans(file: &FolFile) -> HashMap<String, Span> {
    let mut defs: HashMap<String, Span> = HashMap::new();
    for command in &file.commands {
        match command {
            Command::Def { name, span, .. }
            | Command::Theorem { name, span, .. }
            | Command::Axiom { name, span, .. } => {
                defs.entry(name.clone()).or_insert(*span);
            }
            Command::InductiveBlock {
                name,
                constructors,
                recursor,
                span,
                ..
            } => {
                defs.entry(name.clone()).or_insert(*span);
                for ctor in constructors {
                    defs.entry(ctor.name.clone()).or_insert(ctor.span);
                }
                if let Some(rec) = recursor {
                    defs.entry(rec.name.clone()).or_insert(rec.span);
                }
            }
            Command::Example { .. }
            | Command::Check { .. }
            | Command::Reduce { .. }
            | Command::Print { .. } => {}
        }
    }
    defs
}

/// Run a kernel interaction with panic suppression: panics (assertion /
/// internal errors) become `Err(message)` instead of unwinding through the
/// pipeline, so the caller can classify them like any other rejection.
/// Same contract as `resolve_hovers`.
fn quiet_catch<R>(f: impl FnOnce() -> R) -> Result<R, String> {
    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
    std::panic::set_hook(previous_hook);
    result.map_err(|payload| {
        if let Some(s) = payload.downcast_ref::<&str>() {
            (*s).to_string()
        } else if let Some(s) = payload.downcast_ref::<String>() {
            s.clone()
        } else {
            "unknown kernel panic".to_string()
        }
    })
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
        universe: Vec::new(),
        holes: Vec::new(),
        sub_goals: Vec::new(),
        refine_template: None,
        by_steps: Vec::new(),
        hints: Vec::new(),
        ty_text: None,
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
            // Binder-declaration rows render from their own source slice, so
            // skip the (possibly panic-prone) type inference entirely.
            let text = if node.binder {
                String::new()
            } else {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    env.with_tc(EnvLimit::ByIndex(cmd.env_at), |tc| {
                        let ty = tc.infer_under_binders(&node.scope_tys, node.expr);
                        // 用 scope binder 名字播种 pp：松散变量还原为真名
                        //（`$N` 不再出现），telescope 域的 lift 恰好被抵消。
                        tc.with_pp_scoped(&node.scope_names, |pp| pp.pp_expr(ty))
                    })
                }));
                match result {
                    Ok(t) => name_loose_bvars(&t, &node.scope_names),
                    // infer_under_binders panic（delta 展开限制）：保留 span、
                    // text 置空——LSP 层的括号回退仍能定位到正确的子表达式，
                    // hover 显示源码切片（不带类型后缀）。
                    Err(_) => String::new(),
                }
            };
            // 保留所有行（含 $N 行——LSP 层只显示源码切片，不显示乱码类型）。
            // 此前按 $ 过滤导致子表达式行丢失，外层行"遮蔽"了子表达式 hover。
            out.push(HoverType {
                span: node.span,
                text,
                scope_names: node.scope_names,
                resolution: node.resolution,
                binder: node.binder,
            });
            out_cmds.push(cmd.cmd);
        }
    }
    std::panic::set_hook(previous_hook);
}

/// 内核 pp 把"binder 在被打印项之外"的松散变量渲染为 `$N`（N = de Bruijn
/// 序号，0 = 最内层）。把 `$N` 替换回该处的 binder 名字：`scope_names`
/// 外层在前，第 i 个松散变量（0 起）对应倒数第 i+1 个名字。找不到名字时
/// 保留 `$N`（宁可出现 `$1` 也不错杀数字字面量）。
fn name_loose_bvars(text: &str, scope_names: &[String]) -> String {
    if !text.contains('$') {
        return text.to_string();
    }
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len() + 8);
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'$' && i + 1 < bytes.len() && bytes[i + 1].is_ascii_digit() {
            let start = i + 1;
            let mut j = start;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            let idx: usize = text[start..j].parse().unwrap_or(usize::MAX);
            match scope_names.len().checked_sub(1 + idx) {
                Some(pos) => out.push_str(&scope_names[pos]),
                None => out.push_str(&text[i..j]),
            }
            i = j;
        } else {
            out.push_str(&text[i..i + 1]);
            i += 1;
        }
    }
    out
}

/// Render an AST expression back to source text (used for open-exercise goals).
pub fn render_expr(expr: &Expr) -> String {
    crate::proof::render_expr(expr)
}

//! 编译入口：`compile_fol`/`check_document`、待执行操作与 hover 解析。

use super::elab::{
    build_axiom, build_def, build_example, build_theorem, elab_expr, install_inductive_block,
    ElabScope, HoverNode, UnivMap,
};
use super::error::{parse_def_eq_mismatch, refine_kernel_kind, CompileError, ErrorKind};
use super::event::{CheckEvent, CompileOutput};
use super::prelude::{install_eq_prelude, install_prelude, CompileOptions, PreludeMode};
use super::report::{
    DeclKind, DeclState, DeclStatus, DocumentReport, GoalBinder, HoverType, ResolvedTarget, SubGoal,
};
use crate::{Binder, Command, Expr, FolFile, Span};
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

/// Is the answer an open exercise: does it contain a `sorry`, and can the
/// remaining goal be recovered by walking the declared type alongside the
/// lambda binders already written (and, since multi-hole, constructor-spine
/// arguments)? `None` means "no hole" or "hole in a place the goal cannot be
/// recovered from" (the latter falls through to normal elaboration, which
/// reports `elab-hole-misplaced` at the hole).
fn open_goal(ty: &Expr, val: &Expr, templates: &ConstructorTemplates) -> Option<OpenGoalInfo> {
    if !expr_has_hole(val) {
        return None;
    }
    goal_under_binders(ty, val, templates)
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

/// The walk's answer for an open exercise: the remaining goal (rendered), the
/// lambda binders already written, every hole span, expected types for
/// constructor-spine sub-holes, and a refine skeleton when the goal's head is
/// a known constructor.
pub(crate) struct OpenGoalInfo {
    pub goal: String,
    pub binders: Vec<GoalBinder>,
    pub holes: Vec<Span>,
    pub sub_goals: Vec<SubGoal>,
    pub refine_template: Option<String>,
}

/// Flatten `C x1 … xn` (or `C.{u} x1 … xn`) into `(C, [x1, …, xn])`.
/// Anything whose head is not an identifier (lambda, pi, sort, hole, …) is
/// not a spine.
fn spine_head_args(e: &Expr) -> Option<(String, Vec<&Expr>)> {
    match e {
        Expr::Ident { name, .. } | Expr::UniverseApp { name, .. } => {
            Some((name.clone(), Vec::new()))
        }
        Expr::App { fun, arg, .. } => {
            let (head, mut args) = spine_head_args(fun)?;
            args.push(arg);
            Some((head, args))
        }
        _ => None,
    }
}

/// A constructor application template recovered from the document itself:
/// the constructor's binder names/types (fields) and the identifiers its
/// result applies (which tie parameters to the goal's arguments).
#[derive(Debug, Clone)]
struct CtorTemplate {
    /// The constructor's own name (`And.intro`), used in the refine skeleton.
    name: String,
    binder_names: Vec<String>,
    binder_tys: Vec<Option<Expr>>,
    result_arg_names: Vec<Option<String>>,
}

/// Constructor templates keyed by the inductive/constructor-family head name
/// (`And`, `Or`, `Nat`, …). Sources, in order of authority:
/// 1. `inductive` blocks (their `ctor` declarations carry binders + result);
/// 2. `axiom`s whose result head matches a known name (the teaching
///    skeleton's `axiom And.intro : … -> And a b` shape).
///
/// Suggestion material only — the kernel remains the sole judge.
type ConstructorTemplates = HashMap<String, CtorTemplate>;

/// Flatten a type into `(binder name, binder type)` pairs plus the result.
fn peel_type(ty: &Expr, out: &mut Vec<(String, Option<Expr>)>) -> Expr {
    match ty {
        Expr::Forall { binders, body, .. } => {
            for binder in binders {
                out.push((binder.name.clone(), binder.ty.as_deref().cloned()));
            }
            peel_type(body, out)
        }
        Expr::Arrow {
            domain, codomain, ..
        } => {
            // Anonymous binder (no name): a proof field that cannot be
            // auto-filled or name-mapped.
            out.push((String::new(), Some(domain.as_ref().clone())));
            peel_type(codomain, out)
        }
        other => other.clone(),
    }
}

fn constructor_templates(file: &FolFile) -> ConstructorTemplates {
    let mut templates = ConstructorTemplates::new();
    for command in &file.commands {
        match command {
            Command::InductiveBlock {
                name, constructors, ..
            } => {
                for ctor in constructors {
                    let binder_names = ctor.binders.iter().map(|b| b.name.clone()).collect();
                    let binder_tys = ctor
                        .binders
                        .iter()
                        .map(|b| b.ty.as_deref().cloned())
                        .collect();
                    templates.entry(name.clone()).or_insert(CtorTemplate {
                        name: ctor.name.clone(),
                        binder_names,
                        binder_tys,
                        result_arg_names: Vec::new(),
                    });
                }
            }
            Command::Axiom { name, ty, .. } => {
                let mut binders = Vec::new();
                let result = peel_type(ty, &mut binders);
                let Some((head, result_args)) = spine_head_args(&result) else {
                    continue;
                };
                if result_args.is_empty() {
                    continue; // not a family application (`axiom True : Prop`)
                }
                let binder_names = binders.iter().map(|(n, _)| n.clone()).collect();
                let binder_tys = binders.into_iter().map(|(_, t)| t).collect();
                let result_arg_names = result_args
                    .iter()
                    .map(|arg| match arg {
                        Expr::Ident { name, .. } => Some(name.clone()),
                        _ => None,
                    })
                    .collect();
                templates.entry(head).or_insert(CtorTemplate {
                    name: name.clone(),
                    binder_names,
                    binder_tys,
                    result_arg_names,
                });
            }
            _ => {}
        }
    }
    templates
}

/// Render a goal-type argument, parenthesizing compound expressions so it can
/// be spliced into a template argument list.
fn render_arg(expr: &Expr) -> String {
    match expr {
        Expr::Ident { .. } | Expr::Num { .. } => render_expr(expr),
        other => format!("({})", render_expr(other)),
    }
}

/// Field binder i's filled value: when its name is one of the constructor
/// result's arguments, the goal's argument at that position is the value.
fn template_arg(template: &CtorTemplate, i: usize, ty_args: &[&Expr]) -> Option<String> {
    let name = template.binder_names.get(i)?;
    let j = template
        .result_arg_names
        .iter()
        .position(|n| n.as_deref() == Some(name.as_str()))?;
    let arg = ty_args.get(j)?;
    Some(render_arg(arg))
}

/// Deep-substitute template binder names with the goal's argument ASTs
/// (shadow-guarded: a Forall/Lambda binder named like a key stops
/// substitution beneath it — innermost wins, like elab).
fn substitute_names(expr: &Expr, map: &HashMap<String, Expr>) -> Expr {
    match expr {
        Expr::Ident { name, span } => match map.get(name) {
            // The replacement keeps the hit node's span so diagnostics point
            // at the substituted site.
            Some(replacement) => with_root_span(replacement.clone(), *span),
            None => expr.clone(),
        },
        Expr::App { fun, arg, .. } => Expr::App {
            fun: Box::new(substitute_names(fun, map)),
            arg: Box::new(substitute_names(arg, map)),
            span: expr.span(),
        },
        Expr::Arrow {
            domain, codomain, ..
        } => Expr::Arrow {
            domain: Box::new(substitute_names(domain, map)),
            codomain: Box::new(substitute_names(codomain, map)),
            span: expr.span(),
        },
        Expr::Plus { lhs, rhs, .. } => Expr::Plus {
            lhs: Box::new(substitute_names(lhs, map)),
            rhs: Box::new(substitute_names(rhs, map)),
            span: expr.span(),
        },
        Expr::Lambda {
            binders,
            body,
            span,
        } => {
            let (binders, sub) = substitute_binders(binders, map);
            Expr::Lambda {
                binders,
                body: Box::new(substitute_names(body, &sub)),
                span: *span,
            }
        }
        Expr::Forall {
            binders,
            body,
            span,
        } => {
            let (binders, sub) = substitute_binders(binders, map);
            Expr::Forall {
                binders,
                body: Box::new(substitute_names(body, &sub)),
                span: *span,
            }
        }
        Expr::Sort { .. } | Expr::UniverseApp { .. } | Expr::Num { .. } | Expr::Hole { .. } => {
            expr.clone()
        }
    }
}

/// Rewrite a Forall/Lambda's binder telescope and compute the map that
/// governs its body: a binder's name stops its own key's substitution
/// beneath (innermost wins), while its declared type sits outside its own
/// scope and still sees the earlier siblings of the same group.
fn substitute_binders(
    binders: &[Binder],
    map: &HashMap<String, Expr>,
) -> (Vec<Binder>, HashMap<String, Expr>) {
    let mut sub = map.clone();
    let binders = binders
        .iter()
        .map(|binder| {
            let mut binder = binder.clone();
            let ty = binder.ty.take();
            binder.ty = ty.map(|ty| Box::new(substitute_names(&ty, &sub)));
            sub.remove(&binder.name);
            binder
        })
        .collect();
    (binders, sub)
}

/// Clone of `expr` with its root span replaced: every variant carries a
/// span field, so each variant is rebuilt with the given span.
fn with_root_span(expr: Expr, span: Span) -> Expr {
    match expr {
        Expr::Sort { sort, .. } => Expr::Sort { sort, span },
        Expr::Ident { name, .. } => Expr::Ident { name, span },
        Expr::UniverseApp { name, levels, .. } => Expr::UniverseApp { name, levels, span },
        Expr::Num { value, .. } => Expr::Num { value, span },
        Expr::Hole { .. } => Expr::Hole { span },
        Expr::App { fun, arg, .. } => Expr::App { fun, arg, span },
        Expr::Lambda { binders, body, .. } => Expr::Lambda {
            binders,
            body,
            span,
        },
        Expr::Forall { binders, body, .. } => Expr::Forall {
            binders,
            body,
            span,
        },
        Expr::Arrow {
            domain, codomain, ..
        } => Expr::Arrow {
            domain,
            codomain,
            span,
        },
        Expr::Plus { lhs, rhs, .. } => Expr::Plus { lhs, rhs, span },
    }
}

/// Expected type text for the field at position `i`, instantiated through the
/// result-argument mapping (`binder name → rendered goal argument`). Bare
/// Ident fields keep the plain argument text; compound field types
/// (`Eq a b`, `And a b`, `p a`, …) are deep-substituted with the goal's own
/// argument ASTs (shadow-guarded) before rendering.
fn field_type_text(template: &CtorTemplate, i: usize, ty_args: &[&Expr]) -> Option<String> {
    let ty = template.binder_tys.get(i)?.as_ref()?;
    if let Expr::Ident { name, .. } = ty {
        if let Some(j) = template
            .result_arg_names
            .iter()
            .position(|n| n.as_deref() == Some(name.as_str()))
        {
            let arg = ty_args.get(j)?;
            return Some(render_arg(arg));
        }
        return Some(render_expr(ty));
    }
    // Every binder the goal determines goes into the map (via the
    // result-argument position relation), not just the first hit.
    let mut map: HashMap<String, Expr> = HashMap::new();
    for name in &template.binder_names {
        let Some(j) = template
            .result_arg_names
            .iter()
            .position(|n| n.as_deref() == Some(name.as_str()))
        else {
            continue;
        };
        let Some(arg) = ty_args.get(j) else {
            continue;
        };
        map.insert(name.clone(), (*arg).clone());
    }
    Some(render_expr(&substitute_names(ty, &map)))
}

/// The constructor-spine case of the walk: the answer is a (partial) ctor
/// application `ctor v1 … vn` against the goal `C t1 … tm` — every `sorry`
/// argument is a sub-hole. Parameter positions expect the goal's own
/// argument (the value is determined by the goal); proof-field positions
/// expect the instantiated field type.
fn ctor_spine_case(
    ty: &Expr,
    val: &Expr,
    binders: Vec<GoalBinder>,
    templates: &ConstructorTemplates,
) -> Option<OpenGoalInfo> {
    let (val_head, val_args) = spine_head_args(val)?;
    let (ty_head, ty_args) = spine_head_args(ty)?;
    let template = templates.get(&ty_head)?;
    // 值的头必须是该族的构造子（如目标头 `And` ↔ 构造子 `And.intro`）。
    if template.name != val_head || val_args.len() > template.binder_names.len() {
        return None;
    }
    let mut holes = Vec::new();
    let mut sub_goals = Vec::new();
    for (i, arg) in val_args.iter().enumerate() {
        if let Expr::Hole { span } = arg {
            holes.push(*span);
            let ty_text = template_arg(template, i, &ty_args)
                .or_else(|| field_type_text(template, i, &ty_args));
            sub_goals.push(SubGoal {
                span: *span,
                ty: ty_text,
            });
        }
    }
    if holes.is_empty() {
        return None; // fully applied, no holes: not an open exercise
    }
    Some(OpenGoalInfo {
        goal: render_expr(ty),
        binders,
        holes,
        sub_goals,
        refine_template: None,
    })
}

/// The refine skeleton for a single-hole answer whose goal head is a known
/// constructor: parameters that the goal determines are auto-filled, proof
/// fields become `sorry`.
fn refine_template_for(ty: &Expr, templates: &ConstructorTemplates) -> Option<String> {
    // 目标本身可能是 Pi 链（`… -> And a b`）：剥到结果再取 spine。
    let mut binders = Vec::new();
    let result = peel_type(ty, &mut binders);
    let (head, ty_args) = spine_head_args(&result)?;
    let template = templates.get(&head)?;
    if template.binder_tys.len() != template.binder_names.len()
        || template.result_arg_names.is_empty()
    {
        return None;
    }
    let mut args = Vec::with_capacity(template.binder_names.len());
    let mut any_hole = false;
    for i in 0..template.binder_names.len() {
        match template_arg(template, i, &ty_args) {
            Some(value) => args.push(value),
            None => {
                args.push("sorry".to_string());
                any_hole = true;
            }
        }
    }
    if !any_hole {
        return None; // the goal is fully determined; nothing to refine
    }
    Some(format!("{} {}", template.name, args.join(" ")))
}

/// Walk the declared type and the (partial) answer in parallel: every lambda
/// in the answer consumes one Pi layer of the type; when the walk reaches a
/// hole (or a constructor spine with holes), the remaining type is the
/// exercise's current goal and the consumed binders are its context.
fn goal_under_binders(
    ty: &Expr,
    val: &Expr,
    templates: &ConstructorTemplates,
) -> Option<OpenGoalInfo> {
    match val {
        Expr::Hole { span } => {
            let holes = vec![*span];
            Some(OpenGoalInfo {
                goal: render_expr(ty),
                binders: Vec::new(),
                holes,
                sub_goals: Vec::new(),
                refine_template: refine_template_for(ty, templates),
            })
        }
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
            let mut info = if binders_rest.is_empty() {
                goal_under_binders(&rest_ty, body, templates)?
            } else {
                let rest_val = Expr::Lambda {
                    binders: binders_rest.to_vec(),
                    body: body.clone(),
                    span: Span::default(),
                };
                goal_under_binders(&rest_ty, &rest_val, templates)?
            };
            info.binders.insert(0, introduced);
            Some(info)
        }
        _ => ctor_spine_case(ty, val, Vec::new(), templates),
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
    let templates = constructor_templates(file);

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
                if let Some(info) = open_goal(ty, val, &templates) {
                    let declared_ty = elab_expr(
                        &mut builder,
                        ty,
                        &mut ElabScope::new(),
                        &no_universe,
                        &known_universes,
                        &mut Vec::new(),
                        None,
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
                if let Some(info) = open_goal(ty, val, &templates) {
                    let declared_ty = elab_expr(
                        &mut builder,
                        ty,
                        &mut ElabScope::new(),
                        &no_universe,
                        &known_universes,
                        &mut Vec::new(),
                        None,
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
                if let Some(info) = open_goal(ty, val, &templates) {
                    let declared_ty = elab_expr(
                        &mut builder,
                        ty,
                        &mut ElabScope::new(),
                        &no_universe,
                        &known_universes,
                        &mut Vec::new(),
                        None,
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
                universe,
                declared_ty,
                goal,
                binders,
                holes,
                sub_goals,
                refine_template,
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
                    hints: Vec::new(),
                    ty_text,
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
                            universe: Vec::new(),
                            holes: Vec::new(),
                            sub_goals: Vec::new(),
                            refine_template: None,
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
                            universe: Vec::new(),
                            holes: Vec::new(),
                            sub_goals: Vec::new(),
                            refine_template: None,
                            hints: Vec::new(),
                            ty_text: None,
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
    }
    let _ = built_inductives;
    (out, report, failed_cmds, kernel_checks)
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
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                env.with_tc(EnvLimit::ByIndex(cmd.env_at), |tc| {
                    let ty = tc.infer_under_binders(&node.scope_tys, node.expr);
                    tc.with_pp(|pp| pp.pp_expr(ty))
                })
            }));
            let text = match result {
                Ok(t) => name_loose_bvars(&t, &node.scope_names),
                // infer_under_binders panic（delta 展开限制）：保留 span、
                // text 置空——LSP 层的括号回退仍能定位到正确的子表达式，
                // hover 显示源码切片（不带类型后缀）。
                Err(_) => String::new(),
            };
            // 只过滤 $N 行（de Bruijn 深度错配的乱码）；空 text 行保留
            // （span 精确，LSP 层显示源码切片）。
            if !text.contains('$') {
                out.push(HoverType {
                    span: node.span,
                    text,
                    scope_names: node.scope_names,
                    resolution: node.resolution,
                });
                out_cmds.push(cmd.cmd);
            }
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

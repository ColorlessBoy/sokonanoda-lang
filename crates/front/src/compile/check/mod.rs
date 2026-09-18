//! 编译入口：`compile_fol`/`check_document`、待执行操作与 hover 解析。

mod kernel_phase;
mod walk;

use super::elab::{HoverNode, InductiveTable};
use super::error::CompileError;
use super::event::CompileOutput;
use super::goals::GoalTemplates;
use super::prelude::{
    install_bool_prelude, install_eq_prelude, install_prelude, CompileOptions, PreludeMode,
};
use super::report::{
    ByGoalState, ByStepState, DeclKind, DeclState, DeclStatus, DocumentReport, GoalBinder,
    HoverType, SubGoal,
};
use crate::compile::units::{split_report, SourceUnit};
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
    run(
        &[SourceUnit::single("", file)],
        &CompileOptions::default(),
        false,
    )
    .0
}

/// Compile with explicit options (e.g. `PreludeMode::Bare` for a fully bare
/// teaching file that builds every concept from scratch).
pub fn compile_fol_with(file: &FolFile, options: &CompileOptions) -> CompileOutput {
    run(&[SourceUnit::single("", file)], options, false).0
}

/// Compile a file and return the detailed document report (per-declaration
/// states, diagnostics, hover types) that the LSP and agents consume.
pub fn check_document(file: &FolFile) -> DocumentReport {
    run(
        &[SourceUnit::single("", file)],
        &CompileOptions::default(),
        true,
    )
    .1
    .into_iter()
    .next()
    .unwrap_or_default()
}

/// `check_document` with explicit compile options.
pub fn check_document_with(file: &FolFile, options: &CompileOptions) -> DocumentReport {
    run(&[SourceUnit::single("", file)], options, true)
        .1
        .into_iter()
        .next()
        .unwrap_or_default()
}

/// One pass that yields both the CLI event output and the document report
/// (currently `run(file, options, true)`).
pub fn compile_all_with(
    file: &FolFile,
    options: &CompileOptions,
) -> (CompileOutput, DocumentReport) {
    let (out, mut reports) = run(&[SourceUnit::single("", file)], options, true);
    (out, reports.pop().unwrap_or_default())
}

/// 值位若是 `by` 块，先用引擎降级成 lambda AST（可能带尾部 `sorry`）；
/// 否则原样 clone。返回降级后的值位 + 引擎记录的 per-tactic 状态
/// （非 by 块为空），交给既有 `open_goal`/`build_*` 分流。
/// `src` 为文件原文、`span_start` 为声明起点——只有真是 `by` 块才切片
/// （judge 合成的文件 src 为空，普通声明不触发切片）。
fn lower_by_val(
    ty: &Expr,
    val: &Expr,
    prefix_src: &str,
    options: &CompileOptions,
) -> Result<(Expr, Vec<crate::by::ByStep>), CompileError> {
    if let Some((binders, by)) = crate::by::split_by_value(val) {
        crate::by::run_by(ty, by, &binders, prefix_src, options).map(|o| (o.expr, o.steps))
    } else {
        Ok((val.clone(), Vec::new()))
    }
}

/// 值位降低产物：`(值, per-tactic 状态)`。`by` 块走 tactic 引擎；其它值原样
/// 透传（per-tactic 状态为空）。
pub(crate) type LoweredValue = (Expr, Vec<crate::by::ByStep>);

/// 值位是 `by` 块时走 tactic 引擎；其它值原样透传。
///
/// `prefix_src` 是**已经算好的**前缀源码（闭包模式下含依赖声明，见 `run_pass`
/// 里的 `closure_prefixes`）。tactic 引擎靠它合成 `#check` 文件来解析
/// `apply` 的函数类型、`exact` 的项——只给本文件前缀时，入口里
/// `apply And.intro` 会报 `elab-tactic-failed: unknown identifier`（实测）。
fn lower_value(
    ty: &Expr,
    val: &Expr,
    prefix_src: &str,
    options: &CompileOptions,
) -> Result<LoweredValue, CompileError> {
    lower_by_val(ty, val, prefix_src, options)
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
            | Command::Print { .. }
            | Command::Import { .. } => None,
        })
        .collect()
}

pub(crate) fn run(
    units: &[SourceUnit<'_>],
    options: &CompileOptions,
    collect: bool,
) -> (CompileOutput, Vec<DocumentReport>) {
    // Pass 1 checks everything. Kernel-rejected declarations still occupy
    // their names in pass 1, which lets later declarations reference them —
    // unsound for teaching. Pass 2 recomputes in a fresh session with the
    // kernel-failed declarations removed (check-then-add semantics): their
    // names are free again and dependents fail with a proper diagnosis.
    let pass = run_pass(units, options, collect, None, None);
    let mut out = pass.out;
    out.stats.kernel_checks = pass.checks;
    if std::env::var("SOKO_DEBUG_PASS1").is_ok() {
        for (idx, err) in &pass.failed {
            eprintln!("pass1 failed cmd {idx}: {} ({:?})", err.message, err.kind);
        }
    }
    let (out, flat) = if pass.failed.is_empty() {
        (out, pass.report)
    } else {
        let pass2 = run_pass(units, options, collect, Some(&pass.failed), None);
        let mut out2 = pass2.out;
        out2.stats.kernel_checks = pass.checks + pass2.checks;
        (out2, pass2.report)
    };
    let reports = split_report(flat, &out.error_cmds, units);
    (out, reports)
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
    // 增量路径保持**单文件**语义（I8）：闭包编译不使用 TrustPlan（v1），
    // 所以这里始终是一个单元。
    let units = [SourceUnit::single("", file)];
    let pass1 = run_pass(&units, options, true, Some(prefix_failures), Some(trust));
    if pass1.failed.is_empty() {
        let mut out = pass1.out;
        out.stats.kernel_checks = pass1.checks;
        let report = split_report(pass1.report, &out.error_cmds, &units)
            .pop()
            .unwrap_or_default();
        return (out, report, pass1.checks, pass1.sigs, pass1.cutoff);
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
    let pass2 = run_pass(&units, options, true, Some(&skip2), Some(&trust2));
    let checks = pass1.checks + pass2.checks;
    let mut out = pass2.out;
    out.stats.kernel_checks = checks;
    let report = split_report(pass2.report, &out.error_cmds, &units)
        .pop()
        .unwrap_or_default();
    (out, report, checks, pass2.sigs, pass2.cutoff)
}

type KernelFailed = HashMap<usize, CompileError>;

fn run_pass(
    units: &[SourceUnit<'_>],
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
            // 闭包级预扫描（设计 §4.6）：**任一**单元自带顶层 `inductive Nat`
            // 就让位；单文件编译时这就是今天的行为（一个单元 = 一个文件）。
            let explicit_nat = units.iter().any(|unit| {
                unit.file.commands.iter().any(
                    |command| matches!(command, Command::InductiveBlock { name, .. } if name == "Nat"),
                )
            });
            if !explicit_nat {
                // Nat 作为受信任的归纳块安装，同时把 Nat/Nat.zero/Nat.succ/
                // Nat.rec 登记进 `known` 与 `match` 的 InductiveTable。
                install_prelude(&mut builder, &mut known_universes, &mut inductives);
            }
            let explicit_bool = units.iter().any(|unit| {
                unit.file.commands.iter().any(
                    |command| matches!(command, Command::InductiveBlock { name, .. } if name == "Bool"),
                )
            });
            if !explicit_bool {
                // Bool 同法（非递归）：文件自带 `inductive Bool` 时让位。
                install_bool_prelude(&mut builder, &mut known_universes, &mut inductives);
            }
            // `Eq` 的"被占用名字"取整个闭包的并集（设计 §4.6）。
            let mut taken: std::collections::HashSet<String> = std::collections::HashSet::new();
            for unit in units {
                taken.extend(user_top_level_names(unit.file));
            }
            install_eq_prelude(&mut builder, &mut known_universes, &taken);
        }
    }
    let out = CompileOutput::default();
    let report = DocumentReport::default();
    let ops: Vec<PendingOp<'_>> = Vec::new();
    let cmd_hovers: Vec<CmdHover<'_>> = Vec::new();
    let decl_states: Vec<DeclState> = Vec::new();
    let example_idx = 0usize;
    // 建议材料（refine/intro 的构造子与函数索引）按**闭包前缀**构建：单文件时就是
    // 本文件；项目模式下每个单元看到的是"拓扑序在它之前的单元 + 它自己"。
    // 否则入口看不见被导入的构造子（`And.intro`），项目入口的 refine 建议会凭空
    // 消失——而同一个文件放进单文件就有（2026-09-18 真 LSP 探针实测）。
    // 注：`GoalTemplates::new_for` 只读命令表，`src` 仅作占位。
    let all_templates: Vec<GoalTemplates> = if units.len() == 1 {
        vec![GoalTemplates::new_for(units[0].file, options)]
    } else {
        let mut commands: Vec<Command> = Vec::new();
        let mut templates = Vec::with_capacity(units.len());
        for unit in units {
            commands.extend(unit.file.commands.iter().cloned());
            let combined = FolFile {
                commands: commands.clone(),
                src: String::new(),
            };
            templates.push(GoalTemplates::new_for(&combined, options));
        }
        templates
    };

    // 扁平命令序：先依赖、后入口（单文件就是一个单元）。
    let flat: Vec<(usize, &Command)> = units
        .iter()
        .enumerate()
        .flat_map(|(unit, source)| source.file.commands.iter().map(move |c| (unit, c)))
        .collect();
    let unit_of_cmd: Vec<usize> = flat.iter().map(|(unit, _)| *unit).collect();

    // `match` 的宇宙层级、`apply` 的函数类型等都靠"合成一个前缀文件再问内核"
    // （`judge_infer`）。项目模式下被导入模块的声明**不在本文件源码里**，只拼
    // 本文件前缀会让内核看不到它们——入口里对导入归纳类型做 `match` 就会报
    // `elab-match-no-expected-type`（实测：`import Logic` + `match h with … Or …`）。
    // 所以这里按拓扑序把**前面每个单元的声明文本**接成闭包前缀；单文件模式
    // （`units.len() == 1`）不构造，行为与今天逐字节相同（A1）。
    let closure_prefixes: Vec<String> = if units.len() > 1 {
        let mut prefixes = Vec::with_capacity(units.len());
        let mut accumulated = String::new();
        for unit in units {
            prefixes.push(accumulated.clone());
            accumulated.push_str(&crate::project::importless_source(&unit.file.src));
            if !accumulated.ends_with('\n') {
                accumulated.push('\n');
            }
        }
        prefixes
    } else {
        Vec::new()
    };

    let failed_cmds: KernelFailed = HashMap::new();
    let built_inductives: Vec<Declar<'_>> = Vec::new();
    let kernel_checks = 0usize;
    // 命令走查（elaborate → `PendingOp`）：批次 3 第三刀切到 `walk.rs`；
    // 这里的累加器按值交给 `Walk`，内核阶段再从 `walk` 取回（见文件尾）。
    let mut walk = walk::Walk {
        builder,
        known_universes,
        inductives,
        out,
        ops,
        cmd_hovers,
        decl_states,
        example_idx,
        built_inductives,
    };
    walk.run(
        units,
        &flat,
        options,
        skip,
        trust,
        &all_templates,
        &closure_prefixes,
    );
    kernel_phase::finish_pass(kernel_phase::Walked {
        units,
        unit_of_cmd: &unit_of_cmd,
        n_commands: flat.len(),
        collect,
        trust,
        builder: walk.builder,
        out: walk.out,
        report,
        ops: walk.ops,
        cmd_hovers: walk.cmd_hovers,
        decl_states: walk.decl_states,
        failed_cmds,
        kernel_checks,
        built_inductives: walk.built_inductives,
    })
}

/// Every top-level name this file declares, mapped to the span of the command
/// that first defines it (defs/axioms/inductives, plus constructors and
/// 闭包范围内"顶层名字 → 定义处 span"：名字在闭包里全局唯一（重名是
/// `import-name-collision`），所以先到者胜。单文件编译时与
/// `top_level_def_spans` 等价。`project` 层也用它做闭包级检查（单一实现）。
pub(crate) fn top_level_def_spans_over(units: &[SourceUnit<'_>]) -> HashMap<String, Span> {
    let mut defs: HashMap<String, Span> = HashMap::new();
    for unit in units {
        for (name, span) in top_level_def_spans(unit.file) {
            defs.entry(name).or_insert(span);
        }
    }
    defs
}

/// recursors of inductive blocks). Prelude names are absent by construction.
pub(crate) fn top_level_def_spans(file: &FolFile) -> HashMap<String, Span> {
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
            | Command::Print { .. }
            | Command::Import { .. } => {}
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
    out: &mut CompileOutput,
    idx: usize,
    kind: DeclKind,
    name: Option<String>,
    span: Span,
) -> Option<DeclState> {
    let error = skip?.get(&idx)?;
    // 经 `push_error`：`error_cmds` 与 `errors` 必须严格平行，否则跨文件
    // 归因会失去依据（见 `split_report` 的注释）。
    out.push_error(idx, error.clone());
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

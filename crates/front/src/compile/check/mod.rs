//! 编译入口：`compile_fol`/`check_document`、待执行操作与 hover 解析。

mod kernel_phase;
mod walk;

use super::elab::{canonical_ctor_name, DefTable, HoverNode, InductiveTable, KnownTable};
use super::error::CompileError;
use super::event::CompileOutput;
use super::goals::GoalTemplates;
use super::prelude::{
    install_bool_prelude, install_eq_prelude, install_l1_prelude, install_prelude, CompileOptions,
    PreludeMode,
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
        /// 签名终审探针（G-01 / WO-004）：一条**不入环境**的同签名 axiom，
        /// 内核阶段用 `try_check_declar_at` 问「这个签名是不是一个类型」，
        /// `theorem` 再问「是不是 Prop」。签名不过 ⇒ 声明 Failed，
        /// **不**发 `ExerciseOpen`（与值位 elaborate 失败同罪）。
        ///
        /// `Box`：`Declar` 比本枚举的其它变体大一圈（`large_enum_variant`），
        /// 而探针只在签名检查里用一次——间接一层没有代价。
        sig_probe: Box<Declar<'a>>,
        /// 签名诊断的 span（**签名自身**的源范围——G-01 要求把错报到签名上，
        /// 而不是照抄内核消息的 span；G-15 已修后内核 span 本身是精确的）。
        sig_span: Span,
        goal: Option<String>,
        binders: Vec<GoalBinder>,
        holes: Vec<Span>,
        sub_goals: Vec<SubGoal>,
        refine_template: Option<String>,
        span: Span,
        cmd: usize,
        /// Per-tactic states for a `by` value (empty otherwise).
        by_steps: Vec<ByStepState>,
        /// 探针终审用的**环境可见前缀**：本练习若真被补完，它的名字会在
        /// `add_declar` 时占这个下标（= `builder.declaration_count()`），
        /// 所以 `EnvLimit::ByName(真名)` 与 `ByIndex(env_before)` 等价。
        /// 探针不入环境、名字没有 `decl_idx`，只能显式给这个限界
        /// （`docs/design/redundant-sorry.md` §8）。
        env_before: usize,
        /// 「多余的 `sorry`」的 kernel 探针：pass 1 造、pass 2 查、**不入环境**。
        /// 过了才报 `redundant-sorry`（`docs/design/redundant-sorry.md`）。
        redundant_probes: Vec<(Declar<'a>, Span)>,
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
    stage_stats::VIA_CHECK_DOCUMENT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let _t = StageTimer(
        &stage_stats::VIA_CHECK_DOCUMENT_NANOS,
        std::time::Instant::now(),
    );
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
// 参数已 8 个（0.62.0 起多一个 `universe`：判定合成声明要带声明的宇宙
// 参数，见 `docs/design/by-tactics.md` §12）。为压 clippy 把参数打包成
// 结构体只会给热路径加一层间接，得不偿失。
#[allow(clippy::too_many_arguments)]
fn lower_by_val(
    ty: &Expr,
    val: &Expr,
    universe: &[String],
    prefix_src: &str,
    options: &CompileOptions,
    canonical_goal: bool,
    inductives: &crate::compile::elab::InductiveTable<'_>,
    defs: &crate::compile::elab::DefTable,
) -> Result<(Expr, Vec<crate::by::ByStep>), CompileError> {
    if let Some((binders, by)) = crate::by::split_by_value(val) {
        stage_stats::BYS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let _by_timer = StageTimer(&stage_stats::BY_NANOS, std::time::Instant::now());
        crate::by::run_by(
            ty,
            by,
            &binders,
            universe,
            prefix_src,
            options,
            canonical_goal,
            inductives,
            defs,
        )
        .map(|o| (o.expr, o.steps))
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
///
/// `canonical_goal`（G-05）：本文件用了 `namespace`/`open` 时为 `true`，
/// tactic 引擎把根目标先过一遍内核 pp（设计 `docs/design/namespace-open.md`
/// §4.6）；没碰命名空间的文件零额外开销。
// 参数已 8 个（0.62.0 起多一个 `universe`：判定合成声明要带声明的宇宙
// 参数，见 `docs/design/by-tactics.md` §12）。为压 clippy 把参数打包成
// 结构体只会给热路径加一层间接，得不偿失。
#[allow(clippy::too_many_arguments)]
fn lower_value(
    ty: &Expr,
    val: &Expr,
    universe: &[String],
    prefix_src: &str,
    options: &CompileOptions,
    canonical_goal: bool,
    inductives: &crate::compile::elab::InductiveTable<'_>,
    defs: &crate::compile::elab::DefTable,
) -> Result<LoweredValue, CompileError> {
    lower_by_val(
        ty,
        val,
        universe,
        prefix_src,
        options,
        canonical_goal,
        inductives,
        defs,
    )
}

/// 引擎的 per-step 状态 → 报告层 wire 形状（binder 类型渲染成文本）。
fn by_step_states(
    steps: &[crate::by::ByStep],
    display: &crate::display::DisplayNotations,
) -> Vec<ByStepState> {
    // **展示副本**（线 C / T-C22）：`ByStepState` 只给目标栏看（T-C02 的审计），
    // 所以这里折记法。**引擎内部那份 AST 一个字节没动**——`apply`/`cases` 的子目标
    // 是**判定输入**（要回读），折了就会把判定搅坏。
    let fold = |text: &str| {
        crate::display::print_back(text, display)
            .as_display_str()
            .to_string()
    };
    steps
        .iter()
        .map(|s| ByStepState {
            span: s.span,
            goals: s
                .goals
                .iter()
                .map(|g| ByGoalState {
                    ty: fold(&g.ty),
                    binders: g
                        .binders
                        .iter()
                        .map(|b| GoalBinder {
                            name: b.name.clone(),
                            ty: fold(&b.ty.as_deref().map(render_expr).unwrap_or_default()),
                        })
                        .collect(),
                })
                .collect(),
        })
        .collect()
}

/// **显示期的记法表**（线 C）：从闭包各单元的**已解析命令**收记法 + 内建记法，
/// 元数从源级签名 + prelude。整趟建一次，给 `ty_text` 与 `by` 步进的展示副本共用。
pub(crate) fn display_notations(units: &[SourceUnit<'_>]) -> crate::display::DisplayNotations {
    // **关掉折叠的开关**（诊断/判别性测试用）：`SOKO_NO_NOTATION_FOLD=1` ⇒ 空表 ⇒
    // `print_back` 原样返回。它存在的意义是证明"那几条 surface 测试真的抓得住"
    // ——关掉之后它们**必须全红**（T-C24 的判别性判据）。仿 `SOKO_NO_JUDGE_BATCH`。
    if std::env::var_os("SOKO_NO_NOTATION_FOLD").is_some() {
        return crate::display::DisplayNotations::default();
    }
    let commands: Vec<crate::ast::Command> = units
        .iter()
        .flat_map(|unit| unit.file.commands.iter().cloned())
        .collect();
    let mut table = crate::notation::notation_table(&commands);
    // **内建记法要自己补**（`↔`/`∧`/`∨`/`¬`/`=`/`≠` 不在任何源文本里，
    // parser 有硬编码表）——否则 `Iff` 永远折不成 `↔`。
    //
    // ⚠ **不能"文件里没有 `infix` 就早退"**：内建记法**永远生效**，一个只写
    // `And a b` 的文件也要折成 `a ∧ b`（踩过：早退放在这行之前 ⇒ `And` 不折、
    // `query::tests::state_at_root_before_any_tactic` 立刻红）。
    table.splice(0..0, crate::notation::builtin_notation_decls());
    let arities =
        crate::display::arities_with_prelude_from(crate::display::arities_in_commands(&commands));
    // **③ 的第二层诊断**：那条声明**本身**长什么样（③ 已经夹到"实例里的声明" ✗）。
    if std::env::var_os("SOKO_TRACE_NOTATIONS").is_some() {
        let n = table.iter().filter(|d| d.target == "Exists").count();
        let syms: Vec<&str> = table
            .iter()
            .filter(|d| d.target == "Exists")
            .map(|d| d.symbol.as_str())
            .collect();
        let keys: Vec<&String> = arities.keys().filter(|k| k.ends_with("Exists")).collect();
        eprintln!(
            "[trace-notations] decls(target=Exists)={n} symbols={syms:?} arity_keys={keys:?}"
        );
    }
    // **诊断开关**（`SOKO_TRACE_NOTATIONS=1`，默认零输出）：把线 C 的两张表打出来。
    // 存在的理由：③ 那条报告（Infoview 里 `∃` 折不了）逐层排查时，需要一眼看到
    // "这次编译到底看到了哪些命令、表里有没有目标" —— 别再靠读代码猜 ✗。
    if std::env::var_os("SOKO_TRACE_NOTATIONS").is_some() {
        let targets: Vec<&str> = table.iter().map(|d| d.target.as_str()).collect();
        eprintln!(
            "[trace-notations] units={} commands={} table={} binding_Exists={} \
             symbols={:?} arity_Exists={:?} arity_len={}",
            units.len(),
            commands.len(),
            table.len(),
            targets.iter().any(|t| *t == "Exists"),
            table
                .iter()
                .filter(|d| d.target == "Exists")
                .map(|d| d.symbol.as_str())
                .collect::<Vec<_>>(),
            arities.get("Exists"),
            arities.len()
        );
    }
    crate::display::DisplayNotations::new(table, arities)
}

pub(crate) fn run(
    units: &[SourceUnit<'_>],
    options: &CompileOptions,
    collect: bool,
) -> (CompileOutput, Vec<DocumentReport>) {
    // 常驻诊断（`SOKO_PASS_TRACE=<n>`）：在第 n 次 `run` 上打一份调用栈，
    // 用来回答"这几百趟 pass 到底是谁在调"——G-31/G-34 就是这么定位的
    // （380 趟来自记法消解里的 `judge_infer`）。读一次就缓存，它在热路径上。
    if let Some(want) = pass_trace_spec() {
        let n = stage_stats::RUNS.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
        if want.parse::<u64>() == Ok(n) {
            eprintln!(
                "PASS_TRACE #{}:\n{}",
                n,
                std::backtrace::Backtrace::force_capture()
            );
        }
    }
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
    let reports = split_report(flat, &out.error_cmds, &out.warning_cmds, units);
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
    // **T-K11（K1-a）**：告诉 judge 的 miss 路径"前缀 `[0, before)` 已被担保"
    // ——`run_pass` 期间发生的判定（`by` 块/judge_infer/judge_type_of）因此可以
    // 走 `run_incremental` 而不重查前缀。栈式，进出成对。
    let pass1 = crate::judge::with_trusted_prefix(trust.before, prefix_failures, || {
        run_pass(&units, options, true, Some(prefix_failures), Some(trust))
    });
    if pass1.failed.is_empty() {
        let mut out = pass1.out;
        out.stats.kernel_checks = pass1.checks;
        let report = split_report(pass1.report, &out.error_cmds, &out.warning_cmds, &units)
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
    let pass2 = crate::judge::with_trusted_prefix(trust2.before, &skip2, || {
        run_pass(&units, options, true, Some(&skip2), Some(&trust2))
    });
    let checks = pass1.checks + pass2.checks;
    let mut out = pass2.out;
    out.stats.kernel_checks = checks;
    let report = split_report(pass2.report, &out.error_cmds, &out.warning_cmds, &units)
        .pop()
        .unwrap_or_default();
    (out, report, checks, pass2.sigs, pass2.cutoff)
}

type KernelFailed = HashMap<usize, CompileError>;

/// `SOKO_PASS_TRACE` 的取值（读一次就缓存——`run` 在热路径上）。
fn pass_trace_spec() -> Option<&'static str> {
    static SPEC: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
    SPEC.get_or_init(|| std::env::var("SOKO_PASS_TRACE").ok())
        .as_deref()
}

/// 编译阶段的分段计时（T-K20′ 的判据）。
///
/// `SOKO_STAGE_STATS=1` 时在进程退出前打到 stderr。要回答的问题：
/// **一次判定调用（`judge_pairs_uncached` → `check_document_with`）里，
/// 「前端 elaborate」「内核检查」「`by` 引擎自己」各占多少**——
/// 不量清楚就选不出刀（`docs/design/by-judge-reuse.md` §5）。
pub(crate) mod stage_stats {
    use std::sync::atomic::{AtomicU64, Ordering};

    pub(crate) static PASS_NANOS: AtomicU64 = AtomicU64::new(0);
    pub(crate) static PASSES: AtomicU64 = AtomicU64::new(0);
    pub(crate) static BY_NANOS: AtomicU64 = AtomicU64::new(0);
    /// 经由 `check_document_with` 进来的 pass 次数（T-K20′ 诊断：394 趟里谁占大头）。
    pub(crate) static RUNS: AtomicU64 = AtomicU64::new(0);
    pub(crate) static VIA_CHECK_DOCUMENT: AtomicU64 = AtomicU64::new(0);
    /// 经由 `check_document_with` 进来的 pass 累计耗时。
    pub(crate) static VIA_CHECK_DOCUMENT_NANOS: AtomicU64 = AtomicU64::new(0);
    pub(crate) static BYS: AtomicU64 = AtomicU64::new(0);
    static PRINTED: std::sync::Once = std::sync::Once::new();

    pub(crate) fn install() {
        if std::env::var_os("SOKO_STAGE_STATS").is_none() {
            return;
        }
        PRINTED.call_once(|| {
            extern "C" fn report() {
                let passes = PASSES.load(Ordering::Relaxed);
                let bys = BYS.load(Ordering::Relaxed);
                let ms = |n: u64| n / 1_000_000;
                eprintln!(
                    "STAGE_STATS passes={passes} pass_total_ms={} by_calls={bys} by_total_ms={} judge_ms={} hits={} misses={} doc_passes={} doc_ms={}",
                    ms(PASS_NANOS.load(Ordering::Relaxed)),
                    ms(BY_NANOS.load(Ordering::Relaxed)),
                    ms(crate::judge::stats::nanos()),
                    crate::judge::stats::hits(),
                    crate::judge::stats::misses(),
                    VIA_CHECK_DOCUMENT.load(Ordering::Relaxed),
                    ms(VIA_CHECK_DOCUMENT_NANOS.load(Ordering::Relaxed)),
                );
            }
            unsafe extern "C" {
                fn atexit(cb: extern "C" fn()) -> i32;
            }
            unsafe {
                atexit(report);
            }
        });
    }
}

/// 给一个作用域计时（RAII）。
pub(crate) struct StageTimer<'a>(
    pub(crate) &'a std::sync::atomic::AtomicU64,
    pub(crate) std::time::Instant,
);

impl Drop for StageTimer<'_> {
    fn drop(&mut self) {
        self.0.fetch_add(
            self.1.elapsed().as_nanos() as u64,
            std::sync::atomic::Ordering::Relaxed,
        );
    }
}

/// **把 prelude 装进 `builder` 的唯一实现**（T-K12）。
///
/// 为什么必须有这条"唯一实现"：K1-b 要给 judge 准备**第二份**环境
/// （影子环境，`docs/design/vscode-editor-feedback-plan.md` 的 T-K12），
/// 而两份环境的 prelude 必须**逐条同款** —— prelude 装得不一样，
/// 两边的判定就会分叉（那是 REQUIREMENTS §2 第 1 条的红线）。
/// 所以条件（`prelude_shape` 的预扫描结果）与顺序（先 Eq 后 L1）都**只写一遍**。
fn install_all_preludes<'a>(
    builder: &mut EnvBuilder<'a>,
    known: &mut KnownTable,
    inductives: &mut InductiveTable<'a>,
    defs: &mut DefTable,
    units: &[SourceUnit<'a>],
    options: &CompileOptions,
) {
    match options.prelude {
        PreludeMode::Bare => {}
        PreludeMode::Full => {
            // 闭包级预扫描（设计 §4.6）：**任一**单元自带顶层 `inductive Nat`
            // 就让位；单文件编译时这就是今天的行为（一个单元 = 一个文件）。
            //
            // 判据走 `prelude_shape`（**同一个函数**）——它是 K2 复用的守卫
            // （设计 `closure-incremental.md` §2.1 的 O7）：复用一份共享环境之前
            // 要比对形状，而"比对用的形状"与"安装用的判据"必须是同一份计算，
            // 否则守卫会与实际装了什么漂移。
            let shape = prelude_shape(units);
            if !shape.explicit_nat {
                // Nat 作为受信任的归纳块安装，同时把 Nat/Nat.zero/Nat.succ/
                // Nat.rec 登记进 `known` 与 `match` 的 InductiveTable。
                install_prelude(builder, known, inductives);
            }
            if !shape.explicit_bool {
                // Bool 同法（非递归）：文件自带 `inductive Bool` 时让位。
                install_bool_prelude(builder, known, inductives);
            }
            // `Eq` 与 L1 的"被占用名字"取整个闭包的并集（设计 §4.6）。
            //
            // 口径（设计 §2.3-1）：用 `top_level_def_spans_over` 的**键集**，
            // 而不是 `user_top_level_names`——后者只看 `Command::*{name}`，
            // 漏掉 `ctor`/`rec`。L1 之后这会漏掉"文件在别的归纳块里写了
            // `ctor Or.inl` ⇒ 与 prelude 的 `Or.inl` 撞车"。
            let taken: std::collections::HashSet<String> =
                top_level_def_spans_over(units).into_keys().collect();
            // 顺序（as-built，与提案 §6 的措辞略有出入）：**先 Eq，后 L1**。
            // B7 的 `Eq.symm`/`Eq.trans`/`congrArg` 的定义体直接引用
            // `Eq.subst`/`Eq.refl`，所以它们必须在 Eq 已进环境之后才装；
            // 两者的让位读同一个 `taken`（B7 的 `EQ` 依赖），所以先后顺序
            // 不影响让位结果。
            install_eq_prelude(builder, known, &taken);
            install_l1_prelude(builder, known, inductives, defs, &taken);
        }
    }
}

fn run_pass(
    units: &[SourceUnit<'_>],
    options: &CompileOptions,
    collect: bool,
    skip: Option<&KernelFailed>,
    trust: Option<&TrustPlan>,
) -> PassResult {
    stage_stats::install();
    stage_stats::PASSES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let _pass_timer = StageTimer(&stage_stats::PASS_NANOS, std::time::Instant::now());
    let arena = stumpalo::Arena::new();
    let mut builder = EnvBuilder::new(arena.as_arena_ref(), Config::default());
    let mut known: KnownTable = KnownTable::new();
    let mut inductives = InductiveTable::new();
    // 源级 delta 表（课程 Lean 化）：`by` 引擎靠它看穿 def 头（`A ⊆ B`/`¬ A`）。
    let mut defs = DefTable::new();
    install_all_preludes(
        &mut builder,
        &mut known,
        &mut inductives,
        &mut defs,
        units,
        options,
    );
    // **影子环境**（T-K12b）：一份**只给 judge 用**的环境。prelude 走**同一个助手**
    // ⇒ 条件与顺序不可能与主环境分叉 ✓（分叉 = 判定义分叉 = 红线）。
    // 它从 walk 的 `ops` **惰性重放**（不用就零成本 ✓）。
    // ⚠ **默认不建**（`SOKO_SHADOW_CHECK` 才建）：影子环境是 T-K12b 的**实验品**
    // ——对照判据已判定它与内核阶段**不等价** ✗（差在增量记账：`skip`/`trust`/
    // pass1-pass2 ⇒ 影子偏严），所以它**不能**进判定路径。但每次编译都白装一份
    // prelude 是**热路径成本** ✗ ⇒ 关进开关：默认零成本 ✓、要复现对照实验时打开 ✓。
    // 结论与后续见 `docs/design/vscode-editor-feedback-plan.md` 的 T-K12b。
    let shadow_experiment = std::env::var("SOKO_SHADOW_CHECK").is_ok();
    let shadow_arena = stumpalo::Arena::new();
    let mut shadow = EnvBuilder::new(shadow_arena.as_arena_ref(), Config::default());
    if shadow_experiment {
        install_all_preludes(
            &mut shadow,
            &mut KnownTable::new(),
            &mut InductiveTable::new(),
            &mut DefTable::new(),
            units,
            options,
        );
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
    let kernel_checks = 0usize;
    // 命令走查（elaborate → `PendingOp`）：批次 3 第三刀切到 `walk.rs`；
    // 这里的累加器按值交给 `Walk`，内核阶段再从 `walk` 取回（见文件尾）。
    let mut walk = walk::Walk {
        shadow,
        shadow_upto: 0,
        shadow_failed: Vec::new(),
        shadow_failed_msg: Vec::new(),
        shadow_skip: None,
        display: display_notations(units),
        builder,
        known,
        inductives,
        defs,
        out,
        ops,
        cmd_hovers,
        decl_states,
        example_idx,
        // G-05：命名空间栈 + open 集合按源码顺序驱动；单元切换处 reset
        // （`open` 是文件作用域，不跨 `import`——设计 N5）。第二刀：导出表
        // 是唯一的跨单元例外（`export`，设计 §N7）。
        ns: crate::compile::NamespaceScope::new(),
        exports: Vec::new(),
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
    // **T-K12b 的一致性观测**：把影子环境推进到"全部已 elaborate 的前缀"
    // （`finish_pass` 会把 `walk` 的字段移走 ⇒ 必须在这之前取数 ✓）。
    // `SOKO_SHADOW_CHECK=1` 时打印影子的规模与失败数，供与内核阶段对照
    // ——"影子可不可信"就是靠这条观测来判的（下一步升级成断言 ✓）。
    let shadow_decls = if shadow_experiment {
        walk.shadow_env().declaration_count()
    } else {
        0
    };
    let walk_ops_len = walk.ops.len();
    let mut shadow_failed = walk.shadow_failed.clone();
    // **去重**：影子按"声明"记（一个 `inductive` 块里多条失败 = 多条 ✓），
    // 内核的失败表是 `HashMap<cmd, _>`（**按命令**一条 ✗）⇒ 不去重会把
    // "同一命令多条成员失败"误报成差异 ✗。
    shadow_failed.sort_unstable();
    shadow_failed.dedup();
    // 影子**覆盖**的命令（`Decl`/`InductiveBlock`）——对照只在这批命令上做：
    // `OpenExercise` 的签名探针是 `kernel_phase` 侧合成的（`ByIndex(env_before)`），
    // 影子不镜像它 ⇒ 拿它比会假报差异 ✗。
    // 命令 → 名字（只用于观测：影子失败时要知道**是哪些声明** ✗，否则只能猜 ✓）。
    let shadow_names: std::collections::HashMap<usize, String> = walk
        .ops
        .iter()
        .filter_map(|op| match op {
            PendingOp::Decl {
                cmd, name: Some(n), ..
            } => Some((*cmd, n.clone())),
            PendingOp::InductiveBlock { cmd, name, .. } => Some((*cmd, name.clone())),
            _ => None,
        })
        .collect();
    let shadow_covered: std::collections::HashSet<usize> = shadow_names.keys().copied().collect();
    let pass = kernel_phase::finish_pass(kernel_phase::Walked {
        display: walk.display,
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
    });
    // **T-K12b 的对照判据**：影子的失败表 vs 内核阶段的失败表，逐条比（同键 ✓）。
    // 只在影子覆盖的命令上比（见 `shadow_covered` 的注释）。
    let mut kernel_failed: Vec<usize> = pass
        .failed
        .keys()
        .copied()
        .filter(|c| shadow_covered.contains(c))
        .collect();
    kernel_failed.sort_unstable();
    if shadow_experiment {
        eprintln!(
            "SHADOW: pass={} ops={} decls={shadow_decls} 一致={} shadow_failed={shadow_failed:?} \
             kernel_failed={kernel_failed:?} 影子失败的名字={:?}",
            crate::compile::check::stage_stats::PASSES.load(std::sync::atomic::Ordering::Relaxed),
            walk_ops_len,
            shadow_failed == kernel_failed,
            walk.shadow_failed_msg
                .iter()
                .map(|(c, m)| format!(
                    "{}@{}: {}",
                    shadow_names.get(c).cloned().unwrap_or_else(|| "?".into()),
                    c,
                    m.chars().take(160).collect::<String>()
                ))
                .collect::<Vec<_>>()
        );
    }
    pass
}

/// Every top-level name this file declares, mapped to the span of the command
/// that first defines it (defs/axioms/inductives, plus constructors and
/// 闭包范围内"顶层名字 → 定义处 span"：名字在闭包里全局唯一（重名是
/// `import-name-collision`），所以先到者胜。单文件编译时与
/// 一个闭包的 **prelude 形状**（设计 `closure-incremental.md` §2.1 的 **O7 守卫**）。
///
/// 内核预装（`Nat` / `Bool` / `Eq` / L1）**是按整个闭包决定的**：
/// 任一单元自带顶层 `inductive Nat`/`Bool` 就让位；`Eq`/L1 的让位看整个闭包
/// 顶层名字的并集。⇒ **复用一份共享环境之前必须证明两件事的形状相同**，
/// 否则会**静默改变判卷**（共享环境里可能装着某个入口的闭包里不存在的
/// `Nat`/`Bool`/`Eq`）。
///
/// 这就是"形状"的全部内容：三个判据 + 与 prelude 名字**撞车**的那些顶层名字
/// （排序后比较）。撞车集是 `taken` 里真正影响让位的那部分——用整个 `taken`
/// 会把"两个闭包的用户名字不同"误判成形状不同。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreludeShape {
    pub explicit_nat: bool,
    pub explicit_bool: bool,
    /// 与 prelude 名字撞车的顶层名字（**排序**，便于比较与打印）。
    pub shadowed: Vec<String>,
}

/// 算一个闭包的 prelude 形状。**与 `run_pass` 里的安装判据同源**
/// （`prelude_shape` 就在那段逻辑的正上方，改一处必须改两处——有测试盯着）。
pub fn prelude_shape(units: &[SourceUnit<'_>]) -> PreludeShape {
    let has_inductive = |want: &str| {
        units.iter().any(|unit| {
            crate::ast::effective_commands(unit.file).iter().any(
                |command| matches!(command, Command::InductiveBlock { name, .. } if name == want),
            )
        })
    };
    let taken: std::collections::HashSet<String> =
        top_level_def_spans_over(units).into_keys().collect();
    let mut shadowed: Vec<String> = crate::compile::PRELUDE_NAMES
        .iter()
        .filter(|name| taken.contains(**name))
        .map(|name| (*name).to_string())
        .collect();
    shadowed.sort();
    PreludeShape {
        explicit_nat: has_inductive("Nat"),
        explicit_bool: has_inductive("Bool"),
        shadowed,
    }
}

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
    // 第二刀 §N7：`open … in <声明>` 包住的声明照样要进这张表（hover/goto
    // 回填与闭包级重名检查都读它）。
    for command in crate::ast::effective_commands(file) {
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
                    // R1：闭包唯一性、hover/goto 回填、`import-name-collision`
                    // 的第二道闸都按**规范名**走（源名只活在解析层）。
                    defs.entry(canonical_ctor_name(name, &ctor.name))
                        .or_insert(ctor.span);
                }
                if let Some(rec) = recursor {
                    defs.entry(rec.name.clone()).or_insert(rec.span);
                }
            }
            Command::Example { .. }
            | Command::Check { .. }
            | Command::Reduce { .. }
            | Command::Print { .. }
            | Command::Import { .. }
            // 记法命令不是声明（设计 N6）：不进 `top_level_def_spans`，
            // 所以它既不占名字、也不参与闭包级重名检查。G-05 的
            // namespace/end/open 同理（声明名加前缀已经在 parser 里落定，
            // 所以这里的 `name` 已经是**全局名**）。第二刀：`open … in` 与
            // `export` 同样是**作用域命令**——`open … in` 包住的那条声明
            // 由它自己的变体（`inner`）承载，所以这里不递归进去（`inner`
            // 的声明名已经在 parser 里加了前缀，span 也是它自己的）。
            | Command::Notation { .. }
            | Command::Namespace { .. }
            | Command::End { .. }
            | Command::Open { .. }
            | Command::OpenIn { .. }
            | Command::Export { .. } => {}
        }
    }
    defs
}

thread_local! {
    /// 本线程正处在「静音」区里（`quiet_catch` / `resolve_hovers`）的嵌套层数。
    static QUIET_DEPTH: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

/// 装一次「**按线程**静音」的 panic hook。
///
/// **为什么不是每次调用 `take_hook` + `set_hook`**：panic hook 是**进程全局**的，
/// 而编译自 T-A30 起跑在后台任务里、多份文档的编译**可以并发**。旧写法在 A 线程
/// 静音的窗口里，B 线程的 panic 也一个字都不打——2026-09-23 CI 上
/// `perf_project_dependency_edit_refreshes_dependents` 与
/// `editing_a_dependency_refreshes_the_open_entry` 就是这样「FAILED 但日志里
/// 连一句 `panicked` 都没有」的（`docs/CI-FAILURES.md` 2026-09-23 条）。判据
/// 从"全局静音"改成"问本线程的计数"，别处的 panic 照常可见。
///
/// 顺带也是性能：`take_hook`/`set_hook` 每次都要拿全局锁并装箱，而
/// `quiet_catch` 是**每个内核交互**都走的路。
fn install_quiet_hook() {
    static ONCE: std::sync::OnceLock<()> = std::sync::OnceLock::new();
    ONCE.get_or_init(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            if panic_is_quiet() {
                return;
            }
            previous(info);
        }));
    });
}

/// 进入静音区；离开时（含 unwind）自动恢复。可嵌套。
pub(super) fn quiet() -> QuietGuard {
    install_quiet_hook();
    QUIET_DEPTH.with(|depth| depth.set(depth.get() + 1));
    QuietGuard
}

/// 本线程当前的静音嵌套层数（测试用：判据是「只静音**本**线程」）。
pub(super) fn quiet_depth() -> u32 {
    QUIET_DEPTH.with(|depth| depth.get())
}

/// **这个 panic 该不该被吞掉**——hook 的唯一判据，单独提出来是为了能被直接测：
/// 「静音」只认**当前线程**的计数（见 [`install_quiet_hook`] 的教训）。
pub(super) fn panic_is_quiet() -> bool {
    quiet_depth() > 0
}

pub(super) struct QuietGuard;

impl Drop for QuietGuard {
    fn drop(&mut self) {
        QUIET_DEPTH.with(|depth| depth.set(depth.get().saturating_sub(1)));
    }
}

/// Run a kernel interaction with panic suppression: panics (assertion /
/// internal errors) become `Err(message)` instead of unwinding through the
/// pipeline, so the caller can classify them like any other rejection.
/// Same contract as `resolve_hovers`.
pub(super) fn quiet_catch<R>(f: impl FnOnce() -> R) -> Result<R, String> {
    let _quiet = quiet();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
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
        val_text: None,
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
    let _quiet = quiet();
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

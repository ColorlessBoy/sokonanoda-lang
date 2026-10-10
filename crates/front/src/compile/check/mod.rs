//! 编译入口：`compile_fol`/`check_document`、待执行操作与 hover 解析。

mod kernel_phase;
pub(crate) mod level_exit;
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

/// **T2-B 第 1 步**：`Walk` 的命令级快照要克隆累加器 ⇒ 它必须 `Clone`
/// （纯加法：derive 不改任何行为 ✓）。
#[derive(Clone)]
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
        /// **根状态**（第一条 tactic 之前，见 `walk::by_root_state`）。
        by_root: Option<crate::compile::report::ByGoalState>,
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
        /// **根状态**（第一条 tactic 之前，见 `walk::by_root_state`）。
        by_root: Option<crate::compile::report::ByGoalState>,
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
        /// **U2（2026-10-05 ✓）**：`#check <裸常量>` 时带上**签名源文本** ✓ ——
        /// 渲染**它**而不是实例化后的类型 ✓，因为裸写法的层被默认成 `0` ✗
        /// ⇒ `#check Quot.lift` 会渲成 `{A r B : Prop}` ✗（**给用户的建议错** ✓ = G-63 ✓）。
        /// **Lean 4 对照** ✓：`#check Quot.lift` 显示的是 `.{u, v}` **层参数形式** ✓。
        /// ⚠ **只影响渲染** ✓ —— 判定路径一个字都不动 ✓（零行为变化 ✓）。
        sig: Option<String>,
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

/// **T2-B 第 1 步**（同 [`PendingOp`]）：命令级快照要克隆它。
#[derive(Clone)]
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
    /// **额外的信任集合**（S6 脏集模型，2026-10-01）：`true` 的命令**不重查**
    /// （只 elaborate 进环境），即使它在 `before` **之后**。
    ///
    /// 为什么需要它：`before` 只能表达**连续前缀**（"改动点之前都不必重查"），
    /// 而用户拍板的 §5.1 模型是**依赖图脏传播** —— 改第 k 条时，要重查的是
    /// **它 + 依赖它的下游**，其余后缀（与改动点无依赖关系的那部分）应当从缓存
    /// 恢复。那是一个**不连续**的集合 ⇒ 前缀语义表达不了 ✗。
    ///
    /// 空 = 今天的前缀语义（`idx < before`）✓（所有既有调用方都不受影响）。
    pub trusted_extra: Vec<bool>,
    /// Previous session's per-command signature (length = previous command
    /// count). `None` for open/failed/non-declaration commands.
    pub prev_signatures: Vec<Option<String>>,
    /// `true` for a command index whose source text is byte-identical to the
    /// previous session's command at the same index.
    pub text_unchanged: Vec<bool>,
    /// Early cutoff is only attempted when the caller established that all
    /// commands after `before` keep their text (single-edit alignment).
    pub allow_cutoff: bool,
    /// **G-31/G-92 第二刀**（2026-10-07 ✓）：调用方那一趟**成功进环境**的声明名
    /// （`judge::EnteredNames`）。`Some` ⇒ 合成编译的内层 walk 对**名单里的 `theorem`**
    /// 只 elaborate **类型**、按**不透明常量**（`Declar::Axiom`）加进环境
    /// ⇒ **证明体不再重跑**（`by_calls` 的 Σ(1..N) 消失 ✓）。
    ///
    /// ⚠ **`None` 是绝大多数调用方** ✓（`session` / `query` / 测试 / 主编译 pass）
    /// ⇒ 快路的**爆炸半径 = judge 的合成文档** ✓，其余**逐字节回到今天** ✓。
    /// ⚠ 用**声明名**而不是**命令号**：两套坐标系**不对齐** ✗（实测真实课程
    /// `before = prefix_commands + 1`）—— 名字是声明身份 ⇒ 与坐标系无关 ✓。
    pub trusted_entered: Option<crate::judge::EnteredNames>,
}

/// One `run_pass` result, including the early-cutoff bookkeeping.
/// **切片 1b（2026-09-29）**：prelude 的登记表（`Nat`/`Bool`/`Eq`/L1）—— 它们**不在 `declars` 里**，
/// 所以 `hide_declars`/`restore_declars` 救不了它们 ✗。让它们**跨趟复用**（库层装一次、
/// 各入口趟接着用）⇒ 入口才看得见 `Nat`，且不会重复登记。
/// ⚠ **G-68（2026-10-06）**：它**可以克隆**，而且多入口会话**必须**克隆 ——
/// 这三张表是**跨趟累加**的（`walk` 把本趟登记进去、再把表交回调用方 ✗）⇒
/// 一个 session 里连着编两个入口时，**前一个入口的声明会泄进后一个** ✗
/// （`declars` 有 `hide_declars`/`restore_declars` 挡着，这三张表没有 ✗）。
/// 会话因此按"库层快照"逐入口还原 —— 见 `project/session.rs` 的 `tables_snapshot`。
#[derive(Clone)]
pub(crate) struct PassTables<'a> {
    pub(crate) known: KnownTable,
    pub(crate) inductives: InductiveTable<'a>,
    pub(crate) defs: DefTable,
}

impl<'a> PassTables<'a> {
    pub(crate) fn new() -> Self {
        Self {
            known: KnownTable::new(),
            inductives: InductiveTable::new(),
            defs: DefTable::new(),
        }
    }
}

pub(crate) struct PassResult {
    pub(crate) out: CompileOutput,
    pub(crate) report: DocumentReport,
    /// 本趟的**命令总数**（切片 1b：把多趟的扁平输出拼成闭包级输出时，后一趟的
    /// `event_cmds`/`error_cmds`/`warning_cmds` 要整体偏移这么多）。
    pub(crate) n_commands: usize,
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

impl PassResult {
    /// 这一趟**真的**调了几次内核检查（`try_check_declar`；**受信任前缀不计入**）。
    ///
    /// 为什么要有访问器：`run` 会把 `pass.checks` 搬进 `out.stats.kernel_checks`，
    /// 而 `with_project_session` 那条路**漏了这一步** ⇒ 合并输出里的
    /// `kernel_checks` 恒为 **0**（S2 步 2 的判据正好要读它，实测撞到）。
    pub(crate) fn kernel_checks(&self) -> usize {
        self.checks
    }
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
        None,
    )
    .0
}

/// Compile with explicit options (e.g. `PreludeMode::Bare` for a fully bare
/// teaching file that builds every concept from scratch).
pub fn compile_fol_with(file: &FolFile, options: &CompileOptions) -> CompileOutput {
    run(&[SourceUnit::single("", file)], options, false, None).0
}

/// Compile a file and return the detailed document report (per-declaration
/// states, diagnostics, hover types) that the LSP and agents consume.
pub fn check_document(file: &FolFile) -> DocumentReport {
    run(
        &[SourceUnit::single("", file)],
        &CompileOptions::default(),
        true,
        None,
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
    run(&[SourceUnit::single("", file)], options, true, None)
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
    let (out, mut reports) = run(&[SourceUnit::single("", file)], options, true, None);
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
fn lower_by_val<'a>(
    ty: &Expr,
    val: &Expr,
    universe: &[String],
    prefix_src: &str,
    options: &CompileOptions,
    canonical_goal: bool,
    inductives: &crate::compile::elab::InductiveTable<'_>,
    defs: &crate::compile::elab::DefTable,
    ctx: &crate::compile::elab::ElabCtx<'a, '_>,
    env: Option<&mut crate::compile::elab::InplaceEnv<'_, 'a>>,
) -> Result<(Expr, Vec<crate::by::ByStep>), crate::by::ByFailure> {
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
            ctx,
            env,
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
fn lower_value<'a>(
    ty: &Expr,
    val: &Expr,
    universe: &[String],
    prefix_src: &str,
    options: &CompileOptions,
    canonical_goal: bool,
    inductives: &crate::compile::elab::InductiveTable<'_>,
    defs: &crate::compile::elab::DefTable,
    ctx: &crate::compile::elab::ElabCtx<'a, '_>,
    env: Option<&mut crate::compile::elab::InplaceEnv<'_, 'a>>,
) -> Result<LoweredValue, crate::by::ByFailure> {
    lower_by_val(
        ty,
        val,
        universe,
        prefix_src,
        options,
        canonical_goal,
        inductives,
        defs,
        ctx,
        env,
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
    // **走唯一接口**（T-U11，2026-09-25 ✓）：`fold` 就是 `print_back(text, self)` ✓
    // ⇒ **零行为变化** ✓（判据：front 全量 + T-U12 面级判据 ✓）。
    let fold = |text: &str| display.fold(text);
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
/// **给 front 的消费者建表用**（同上 ✓）：由 front 算 arity ✓，调用方**只拿结果** ✓。
pub fn display_notations(units: &[SourceUnit<'_>]) -> crate::display::DisplayNotations {
    // **T2-B0 的判据读数**（2026-10-09）：这张表是**闭包文本的纯函数**，今天**每刀重算** ✗。
    NOTATION_TABLE_BUILDS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    NOTATION_TABLE_UNITS.fetch_add(units.len() as u64, std::sync::atomic::Ordering::Relaxed);
    let commands: Vec<crate::ast::Command> = units
        .iter()
        .flat_map(|unit| unit.file.commands.iter().cloned())
        .collect();
    display_notations_from_commands(&commands)
}

/// **建表的唯一实现**（T-U11 ✓ 2026-09-25 从 `display_notations` 抽出来 ✓）：
/// 源侧的两条路（项目里已有 `SourceUnit` ✓ / front 之外只有**源文本** ✓）
/// 都汇到这里 ✓ —— **front 是唯一建表方** ✓，调用方**不许**自己造 arity ✗
/// （= 第五套实现 ✓，`audit-notation-paths.py` 会抓 ✓）。
pub fn display_notations_from_commands(
    commands: &[crate::ast::Command],
) -> crate::display::DisplayNotations {
    // **关掉折叠的开关**（诊断/判别性测试用）：`SOKO_NO_NOTATION_FOLD=1` ⇒ 空表 ⇒
    // `print_back` 原样返回。它存在的意义是证明"那几条 surface 测试真的抓得住"
    // ——关掉之后它们**必须全红**（T-C24 的判别性判据）。仿 `SOKO_NO_JUDGE_BATCH`。
    if std::env::var_os("SOKO_NO_NOTATION_FOLD").is_some() {
        return crate::display::DisplayNotations::default();
    }
    let mut table = crate::notation::notation_table(commands);
    // **内建记法要自己补**（`↔`/`∧`/`∨`/`¬`/`=`/`≠` 不在任何源文本里，
    // parser 有硬编码表）——否则 `Iff` 永远折不成 `↔`。
    //
    // ⚠ **不能"文件里没有 `infix` 就早退"**：内建记法**永远生效**，一个只写
    // `And a b` 的文件也要折成 `a ∧ b`（踩过：早退放在这行之前 ⇒ `And` 不折、
    // `query::tests::state_at_root_before_any_tactic` 立刻红）。
    table.splice(0..0, crate::notation::builtin_notation_decls());
    let arities =
        crate::display::arities_with_prelude_from(crate::display::arities_in_commands(commands));
    // **前导隐式个数**（切片：折坏部分应用，2026-10-10）：pp 对**闭项**省略前导
    // 隐式实参、对**开项**不省略 ⇒ 完全应用有两种实参个数（`Set.mem a A` 与
    // `Set.mem α a A`）。两张表必须**一起**建、一起传（`arity` 单独一张会在开项上
    // 判成"部分应用"，在内层节点上判成"完全应用" ⇒ `(α = a) a`）。
    let implicit_prefixes = crate::display::implicit_prefixes_with_prelude_from(
        crate::display::implicit_prefixes_in_commands(commands),
    );
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
            commands.len(),
            commands.len(),
            table.len(),
            targets.contains(&"Exists"),
            table
                .iter()
                .filter(|d| d.target == "Exists")
                .map(|d| d.symbol.as_str())
                .collect::<Vec<_>>(),
            arities.get("Exists"),
            arities.len()
        );
    }
    crate::display::DisplayNotations::new(table, arities).with_implicit_prefix(implicit_prefixes)
}

/// **给 front 之外的消费者（LSP hover 等）折一段文本**（T-U11 A 组 ✓ 2026-09-25）。
/// 从**源文本**建表 ✓（front 是唯一建表方 ✓），再折 ✓。
/// ⚠ 只给**显示路径**用 ✓ —— 判定/解析路径**不许**折 ✗（折了会改判定 = 内核红线 ✓）。
/// 源解析不了 ⇒ **原样返回** ✓（宁可少折一点，也不让 hover 崩 ✓）。
pub fn fold_for_display(source: &str, text: &str) -> String {
    match crate::parser::parse(source) {
        Ok(file) => display_notations_from_commands(&file.commands).fold(text),
        Err(_) => text.to_string(),
    }
}

pub(crate) fn run(
    units: &[SourceUnit<'_>],
    options: &CompileOptions,
    collect: bool,
    progress: Option<&mut dyn crate::compile::ProgressSink>,
) -> (CompileOutput, Vec<DocumentReport>) {
    // **模块编译计数**（切片 1 / G-68 的判据读数）：一次 `run` = 一趟 pass，
    // 而"这一趟编了几个模块"就是 `units.len()` —— 复用生效时它会降下来 ✓。
    // 计数点选在**入口**而不是深处：`run` 是"编一批 unit"的唯一入口
    // （`compile_all_units*` 都走它），所以它数的正是"模块被编了几次" ✓。
    stage_stats::MODULE_COMPILES
        .fetch_add(units.len() as u64, std::sync::atomic::Ordering::Relaxed);
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
    // **进度只挂在 pass 1**（P2）：pass 2 只在 pass 1 出现内核失败时才跑
    // （`check-then-add` 语义），它会把同一批命令**再走一遍** ⇒ 挂上去只会让
    // 同一个声明报两次 ✗；而"大文件卡住"的场景几乎都是干净文件（pass 1 一次过）✓。
    let pass = run_pass(units, options, collect, None, None, progress);
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
        let pass2 = run_pass(units, options, collect, Some(&pass.failed), None, None);
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
///
/// **`units` 是调用方给的**（S2 步 1，2026-10-01）：闭包编译要让**库层单元排在
/// 入口之前**（它们天然全在信任前缀 `[0, before)` 里 —— 一个字节都没变），
/// 这样入口自己的前缀不再重查。此前这里**写死** `[SourceUnit::single("", file)]`
/// （注释原文："闭包编译不使用 TrustPlan（v1）"）⇒ 闭包那条路根本接不上 I8。
///
/// ⚠ **返回的 `sigs`/`cutoff` 与 `trust`/`prefix_failures` 在同一个坐标系**
/// （= `units` 展平后的命令流）。调用方按**入口自己的**命令号建快照时，必须
/// 整体减去库层命令数 `lib_n`（设计 `docs/design/declaration-incremental.md` §4.2）。
/// **算错不是崩，是静默错编** ✗。
pub(crate) fn run_incremental(
    units: &[SourceUnit<'_>],
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
    debug_assert!(
        !units.is_empty(),
        "增量路径至少要有一个单元（单文件 = `[SourceUnit::single(\"\", file)]`）"
    );
    // **T-K11（K1-a）**：告诉 judge 的 miss 路径"前缀 `[0, before)` 已被担保"
    // ——`run_pass` 期间发生的判定（`by` 块/judge_infer/judge_type_of）因此可以
    // 走 `run_incremental` 而不重查前缀。栈式，进出成对。
    let pass1 = crate::judge::with_trusted_prefix(
        trust.before,
        prefix_failures,
        // **G-31/G-92 第二刀** ✓：把「成功进环境」名表透传给 judge 的合成编译。
        // ⚠ 主编译 pass 这里恒 `None` ✓（`TrustPlan::trusted_entered` 只有
        // `run_synthesized_incremental` 会设）⇒ 逐字节回到今天 ✓；真正让内层看到
        // 名表的是 `walk.rs` 的**逐命令压栈**（它带的是**本趟 walk 自己的**那张 ✓）。
        trust.trusted_entered.clone(),
        || {
            run_pass(
                units,
                options,
                true,
                Some(prefix_failures),
                Some(trust),
                None,
            )
        },
    );
    if pass1.failed.is_empty() {
        let mut out = pass1.out;
        out.stats.kernel_checks = pass1.checks;
        let report = split_report(pass1.report, &out.error_cmds, &out.warning_cmds, units)
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
        trusted_extra: Vec::new(),
        prev_signatures: Vec::new(),
        text_unchanged: Vec::new(),
        allow_cutoff: false,
        // pass 2 与 pass 1 **同一份**名表（同一个调用方坐标系 ✓）。
        trusted_entered: trust.trusted_entered.clone(),
    };
    let pass2 = crate::judge::with_trusted_prefix(
        trust2.before,
        &skip2,
        trust2.trusted_entered.clone(),
        || run_pass(units, options, true, Some(&skip2), Some(&trust2), None),
    );
    let checks = pass1.checks + pass2.checks;
    let mut out = pass2.out;
    out.stats.kernel_checks = checks;
    let report = split_report(pass2.report, &out.error_cmds, &out.warning_cmds, units)
        .pop()
        .unwrap_or_default();
    (out, report, checks, pass2.sigs, pass2.cutoff)
}

pub(crate) type KernelFailed = HashMap<usize, CompileError>;

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
/// 进程内 `by` 引擎调用次数（**只给判据用**：集成测试各自独立进程 ⇒ 天然隔离，
/// lib 内并行测试会互相干扰 —— 实测过）。
#[doc(hidden)]
#[allow(unused_imports)]
pub(crate) use walk::WalkCheckpoint;

pub fn by_calls_total() -> u64 {
    stage_stats::BYS.load(std::sync::atomic::Ordering::Relaxed)
}

/// **模块编译次数**（切片 1 的判据读数，G-68）—— 进程级，与 [`by_calls_total`] 同纪律
/// （集成测试各自独立进程 ⇒ 天然隔离）。
///
/// **为什么需要它**：`by_calls` 数的是 **`by` 引擎调用**，而"依赖被编了几次"是
/// **模块编译次数** —— 用 `by_calls` 量复用会**量错东西**（实测：一个只含 `def`
/// 的夹具改依赖后 `by_calls` 增量为 **0**，因为根本没有 `by`）✗。
/// 这个计数是"**174 → 42**"那条口径的**直接读数** ✓。
#[doc(hidden)]
pub fn module_compiles_total() -> u64 {
    stage_stats::MODULE_COMPILES.load(std::sync::atomic::Ordering::Relaxed)
}

/// 记一次模块编译（由 `compile` 路径调用；**只给判据用**）。
#[doc(hidden)]
pub fn note_module_compile() {
    stage_stats::MODULE_COMPILES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}

/// **闭包模块编译次数**（G-29 的精确读数，2026-10-07）—— 与 [`module_compiles_total`]
/// 的分工见 `run_pass_with` 里的长注释：那个把 **judge 合成文档**也算进去（且**漏掉**
/// 会话路），这个**只数真模块**、且**老路/会话路同口径**。
///
/// 「改一行 ⇒ 整条闭包重编」的正身就是它：冷开 = 闭包模块数（5）· 改一行 = 库层 + 入口
/// （4 + 1 = 5，今天）· 修好之后应当是 **1**（只重编入口）。
#[doc(hidden)]
pub fn closure_module_compiles_total() -> u64 {
    stage_stats::CLOSURE_MODULE_COMPILES.load(std::sync::atomic::Ordering::Relaxed)
}

/// **T2-B 的判据读数**：入口趟**真的 elaborate 了几条命令**。
///
/// **判据**（`PLAN-align-lean4` §3.3 T2-B）：改**最后一条**命令 ⇒ 这个增量 = **1**
/// （今天 = 入口文件的命令数 N）；改第 k 条 ⇒ N−k+1。**反向验证**：改依赖文件
/// ⇒ 回到 N。与 [`closure_module_compiles_total`] 的分工：那个数**模块**、
/// 这个数**命令**（入口趟内部的粒度 ✓）。
///
/// ⚠ 它是**进程级累计**（`atexit` 打，同 [`by_calls_total`] 纪律）⇒ 判据一律
/// **前后取差** ✓。今天**还没有**命令级快照 ⇒ 每刀恒 = N（**先建先红**：这条读数
/// 先写死"今天的行为"，T2-B 落地后**改判成 1，不许放宽** ✗）。
#[doc(hidden)]
pub fn elaborated_commands_total() -> u64 {
    stage_stats::TL_ELABORATED.with(|c| c.get())
}

/// **记一条"真的 elaborate 过的命令"**（只由 `walk` 的入口趟调用 ✓）。
///
/// ⚠ **线程局部**（不是进程级原子）：同一个**测试二进制**里的用例**并行**跑，
/// 进程级计数会被别的夹具污染 ⇒ `Counters::now()` 取差读到别人的增量 ✗
/// （实测：单独跑 `t2b` 读到 1 ✓，整包跑读到 33 ✗ —— 那不是机制坏了，是量具串味）。
/// 取差与断言都发生在**同一个线程**（`QueryDoc::set_text` 是同步的 ✓）⇒ 线程局部即可 ✓。
/// 同时**也**累加进程级那一份，供 `SOKO_STAGE_STATS` 的退出打印（诊断用 ✓）。
#[doc(hidden)]
pub fn note_elaborated_command() {
    stage_stats::TL_ELABORATED.with(|c| c.set(c.get() + 1));
    stage_stats::ELABORATED_COMMANDS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}

pub(crate) mod stage_stats {
    use std::sync::atomic::{AtomicU64, Ordering};

    pub(crate) static PASS_NANOS: AtomicU64 = AtomicU64::new(0);
    pub(crate) static PASSES: AtomicU64 = AtomicU64::new(0);
    /// **真的 elaborate 过的命令条数**（进程级；只给 `SOKO_STAGE_STATS` 的退出打印 ✓）。
    pub(crate) static ELABORATED_COMMANDS: AtomicU64 = AtomicU64::new(0);
    thread_local! {
        /// **判据读数（线程局部）** —— 见 [`super::note_elaborated_command`] 的 ⚠。
        pub(crate) static TL_ELABORATED: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    }
    /// 模块编译次数（切片 1 / G-68 的判据读数）。
    pub(crate) static MODULE_COMPILES: AtomicU64 = AtomicU64::new(0);
    /// **闭包模块编译次数**（G-29 的精确读数）：只数 `path: Some(..)` 的真模块。
    pub(crate) static CLOSURE_MODULE_COMPILES: AtomicU64 = AtomicU64::new(0);
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
                // **P1-a 量具**（`SOKO_JUDGE_CLASSIFY=1`）：回答"judge 那 147s 里
                // 有多少能被「裸常量就地查表」消掉"。**只加计数、不改判定** ✓。
                // **P1-a 判据读数**（总是打，`SOKO_STAGE_STATS` 开了就有）：
                // 重跑前缀的**字节数**与**趟数** —— 结构计数，噪声免疫 ✓。
                {
                    // ⚠ **口径警告**：`JUDGE_INFER_SPLIT` 的三个数是**互斥分段**，
                    // 但实测 `hits+misses+key_ms` **远小于** `JUDGE_INFER.total_ms`
                    //（24.8s vs 247.4s）⇒ **未命中那段的计时没被完整捕获**
                    //（见 `docs/design/p1a-measurements.md` 附五）⇒
                    // **要用"未命中总耗时"就以 `JUDGE_INFER.total_ms − hits − key_ms` 反推**，
                    // 别直接引用 `miss_ms` ✗。
                    let (runs, bytes) = crate::judge::stats::prefix_runs();
                    eprintln!(
                        "JUDGE_PREFIX runs={runs} bytes={bytes} bytes_per_run={}",
                        bytes.checked_div(runs).unwrap_or(0)
                    );
                }
                if std::env::var_os("SOKO_JUDGE_CLASSIFY").is_some() {
                    let (calls, bare, bare_miss, resolvable, bare_miss_ns, all_miss, all_ns) =
                        crate::judge::stats::classify();
                    let share = |ns: u64| {
                        if crate::judge::stats::nanos() == 0 {
                            0.0
                        } else {
                            ns as f64 / crate::judge::stats::nanos() as f64
                        }
                    };
                    eprintln!(
                        "JUDGE_CLASSIFY calls={calls} bare={bare} resolvable={resolvable} \
all_miss={all_miss} all_miss_ms={} all_miss_share={:.3} bare_miss={bare_miss} \
bare_miss_ms={} bare_miss_share={:.3}",
                        all_ns / 1_000_000,
                        share(all_ns),
                        bare_miss_ns / 1_000_000,
                        share(bare_miss_ns)
                    );
                    let head = ["<16", "<48", "<160", ">=160"];
                    for (i, (n, ns)) in crate::judge::stats::classify_buckets().iter().enumerate() {
                        if *n == 0 && *ns == 0 {
                            continue;
                        }
                        eprintln!(
                            "JUDGE_MISS_BUCKET bare={} len={} n={n} ms={} share_of_judge={:.3}",
                            i / 4,
                            head[i % 4],
                            ns / 1_000_000,
                            share(*ns)
                        );
                    }
                }
                let passes = PASSES.load(Ordering::Relaxed);
                let bys = BYS.load(Ordering::Relaxed);
                let ms = |n: u64| n / 1_000_000;
                // 判据 ③（值守 2026-10-04 ✓）：`fallbacks` = **身份退回原文的趟数** ✗ ——
                // 整本课程必须 **0** ✓（>0 ⇒ 那个模块根本没吃到「只改证明」这个特性 ✗）。
                let fallbacks = crate::judge::stats::prefix_fallbacks().0;
                // **判据 ② 的结构读数**（O(n²) 的源头 ✓）：`identity_parses` = 判定键
                // 那边**真的重解析了一整份前缀**的趟数 ✗（预置生效 ⇒ 应当 ≈ 0 ✓）。
                let identity_parses = crate::judge::identity_parses();
                let identity_evictions = crate::judge::identity_evictions();
                // **闸类计数出口**（G-91 ✓）：凡「超过某个数字就换一条路」的分支，
                // 触发了多少次**必须看得见** ✗ —— 没有出口就分不清「没触发」和
                // 「触发了但没人知道」✓（`PARSE_LIMIT` 被抓到纯属侥幸 ✗）。
                let gates: String = sokonanoda::gates::report()
                    .iter()
                    .map(|(name, n)| format!(" {name}={n}"))
                    .collect();
                // **`TcCache` 构造次数**（2026-10-08 端到端 profiling 的读数）：
                // 每一次 `with_tc` 都新建一份预分配 ≈ 4 MiB + 20 张表的 `TcCache`
                // ⇒ 实测 61.8 µs/次、占一次按键编译样本的 63%（见 `util::TC_CACHE_BUILDS`）。
                let tc_cache_builds = sokonanoda::util::tc_cache_builds_total();
                eprintln!(
                    "STAGE_STATS passes={passes} pass_total_ms={} elaborated_commands={} by_calls={bys} by_total_ms={} judge_ms={} hits={} misses={} doc_passes={} doc_ms={} fallbacks={fallbacks} identity_parses={identity_parses} identity_evictions={identity_evictions} tc_cache_builds={tc_cache_builds}{gates}",
                    ms(PASS_NANOS.load(Ordering::Relaxed)),
                    ELABORATED_COMMANDS.load(Ordering::Relaxed),
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

/// **T1-A（2026-10-09）**：一个**模块边界**上的"**续编状态**" —— 把 `walk` 的
/// **跨单元累加器**搬到边界上，续编那趟**原样接着跑**。
///
/// ## 为什么需要它（逐字段给理由，缺一样就是静默错编面）
///
/// * `closure_id`：judge 缓存键要的**环境身份**（`walk` 的 `closure_ids[unit]`
///   = 该单元**之前**所有单元的身份之和）。不带它 ⇒ 续编那趟的键**丢掉前缀**
///   ⇒ 变宽 ⇒ **可能错命中**（不同前缀共用一份旧答案 ✗）；
/// * `exports`：`export` 是**唯一**跨 `import` 的可见性通道（设计 §N7）；`walk`
///   在每个单元切换处 `ns.reset()` 之后**重放**它 ⇒ 不带它 = 续编那趟
///   **丢掉前缀的 export** ✗；
/// * `example_idx`：`_example_N` 是**整趟全局**计数器（`walk` 里两处自增）⇒
///   不带它，两个模块都会造出 `_example_1` ⇒ **重名** ✗。
///
/// ⚠ **边界事实**：这三样都**不能**从"源文本"重算 —— 它们是 elaborate 过程中
/// 累加出来的（`exports` 尤其：它来自 `open`/`export` 命令的**解释结果**）。
/// ⇒ 只能在 walk 里**当场取**（`run_pass_with` 的 `snapshot_state`）。
#[derive(Debug, Clone, Default)]
pub(crate) struct ResumeState {
    pub closure_id: String,
    pub exports: Vec<crate::compile::OpenEntry>,
    pub example_idx: usize,
}

/// **把 prelude 装进 `builder` 的唯一实现**（T-K12）。
///
/// **T1-A**：`pub(crate)` 是为了让 `project/session.rs` 的**逐模块库层趟**能在
/// 循环**外面**装一次（per-module 趟传 `install_preludes = false`）。装的仍是
/// **同一个**函数 ⇒ 「唯一实现」的纪律不变 ✓（判据 `prelude_shape` 同源）。
///
/// 为什么必须有这条"唯一实现"：K1-b 要给 judge 准备**第二份**环境
/// （影子环境，`docs/design/vscode-editor-feedback-plan.md` 的 T-K12），
/// 而两份环境的 prelude 必须**逐条同款** —— prelude 装得不一样，
/// 两边的判定就会分叉（那是 REQUIREMENTS §2 第 1 条的红线）。
/// 所以条件（`prelude_shape` 的预扫描结果）与顺序（先 Eq 后 L1）都**只写一遍**。
pub(crate) fn install_all_preludes<'a>(
    builder: &mut EnvBuilder<'a>,
    known: &mut KnownTable,
    inductives: &mut InductiveTable<'a>,
    defs: &mut DefTable,
    // **T2-B（探针）**：与 arena 寿命解绑（prelude 只**读**源码、往 builder 里放
    // 的是 arena 内新 allocation ⇒ 不需要 units 活得像 arena 一样久 ✓）。
    units: &[SourceUnit<'_>],
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

/// Thin wrapper over [`run_pass_in`] that owns the arena for the duration of
/// one pass — the arena still dies when this call returns, so behavior is
/// byte-for-byte today's. A caller that needs the compilation environment to
/// **outlive** a single pass (per-module artifacts, G-68) calls `run_pass_in`
/// directly with an arena it owns.
fn run_pass(
    units: &[SourceUnit<'_>],
    options: &CompileOptions,
    collect: bool,
    skip: Option<&KernelFailed>,
    trust: Option<&TrustPlan>,
    progress: Option<&mut dyn crate::compile::ProgressSink>,
) -> PassResult {
    let arena = stumpalo::Arena::new();
    run_pass_in(
        arena.as_arena_ref(),
        units,
        options,
        collect,
        skip,
        trust,
        progress,
    )
}

/// The body of a pass, with the arena supplied by the caller.
///
/// Identical line-for-line to what `run_pass` used to do; the only change is
/// that the main arena is no longer created here (the shadow arena below is
/// still local — it only serves the `SOKO_SHADOW_*` experiment). Hoisting the
/// arena out is the prerequisite for "compile each module once, reuse the
/// artifact for later entries" (`docs/design/module-artifacts.md`).
fn run_pass_in<'a>(
    arena: &'a stumpalo::ArenaRef<'a>,
    units: &[SourceUnit<'_>],
    options: &CompileOptions,
    collect: bool,
    skip: Option<&KernelFailed>,
    trust: Option<&TrustPlan>,
    progress: Option<&mut dyn crate::compile::ProgressSink>,
) -> PassResult {
    let builder = EnvBuilder::new(arena, Config::default());
    // 影子只服务 `SOKO_SHADOW_*` 实验：局部 arena，寿命短于主环境 ✓（`'a: 's`）。
    let shadow_arena = stumpalo::Arena::new();
    let shadow = EnvBuilder::new(shadow_arena.as_arena_ref(), Config::default());
    let tables = PassTables::new();
    run_pass_with(
        builder,
        Some(shadow),
        true,
        tables,
        units,
        options,
        collect,
        skip,
        trust,
        progress,
        None,
        None,
        None,
        // 建议材料：按 `units` 自己算（老路逐字节不变 ✓）。
        None,
        // 老路/单文件/库层：judge 的前缀与本趟 `idx` **同坐标系** ✓ ⇒ 不平移。
        0,
        // **T1-A**：老路不要续编状态、也不从断点起跑（逐字节回到今天 ✓）。
        false,
        None,
        // **T2-B**：老路既不给快照、也不要快照（逐字节回到今天 ✓）；
        // 也不是"入口趟" ⇒ 不数它的命令 ✓。
        None,
        false,
        false,
    )
    .0
}

/// **闭包前缀**：按拓扑序把**前面每个单元**的声明文本接起来（`import` 行去掉）。
///
/// **唯一实现**（2026-09-29 抽出）：`run_pass_in` 与 `session` 都用它 ⇒
/// **不会出现"两处各算一遍、算法分叉"** ✗。
/// **为什么需要它**：`judge_infer` 只吃**源码字符串**（`judge.rs:949`，没有环境参数）
/// ⇒ 它必须**从源码重跑前缀** ⇒ 入口那趟若只拿到自己的源码，
/// **入口里"问库层声明的类型/宇宙"的 `judge_infer` 就看不到库层** ✗
/// （实测：这正是切片 1 三次接线失败的**同一个根因**，见
/// `docs/design/incremental-environment.md` §29.1）。
///
/// **单单元**（`units.len() == 1`）⇒ 返回**空 `Vec`** ⇒ 调用方走"本文件前缀"那条路
/// ⇒ 与今天逐字节相同（A1 纪律）✓。
pub fn closure_prefixes_for(units: &[SourceUnit<'_>]) -> Vec<String> {
    if units.len() <= 1 {
        return Vec::new();
    }
    accumulate_prefixes(units).0
}

/// **A4a（2026-10-08）**：`units` **全部拼接之后**的累加串 ——
/// `closure_prefixes_for` 的"最后一格**之后**"那一份 ✓（同一套拼接规则：去 `import` 行 +
/// 补行尾换行）。
///
/// **为什么需要它**：入口趟要的前缀恰好就是**库层全部**（`entry_closure` 的最后一格 =
/// 库层那一段 ✓）⇒ 而它是 `lib_key` 的纯函数 ⇒ 可以**随库层检查点存一次**、
/// 每一刀直接克隆 ✓（以前每一刀都重跑一遍 O(闭包) 的累加 ✗）。
pub fn closure_accumulated_over(units: &[SourceUnit<'_>]) -> String {
    accumulate_prefixes(units).1
}

/// **T1-A（2026-10-09）**：`(逐格前缀, 全部之后的累加串)` 的 `pub(crate)` 出口。
///
/// 与 [`closure_prefixes_for`] 的差别**只有** `units.len() <= 1` 那条特例：
/// 那条返回**空 `Vec`**（"单文件没有闭包前缀"的语义 ✓），而逐模块库层趟要的是
/// `prefixes[0] = ""`（第 0 个模块的前缀就是空串 ✓）⇒ 这里**原样**给出
/// [`accumulate_prefixes`] 的结果 ✓。
///
/// ⚠ **一次算全**（不许每个模块各算一遍 ✗）：`accumulate_prefixes` 就是判据读数
/// [`closure_prefix_builds_total`] 的计数点 ⇒ 遂模块各算会把读数从 O(1) 抬成 O(n) ✗。
pub(crate) fn closure_prefixes_and_total(units: &[SourceUnit<'_>]) -> (Vec<String>, String) {
    accumulate_prefixes(units)
}

/// **累加规则的唯一实现**（A4a）：返回 `(逐格前缀, 全部之后的累加串)`。
///
/// ⚠ 判据读数 [`closure_prefix_builds_total`] 就记在这里 —— "派生表重建数"：
/// 冷开 = 库层趟 1 次 + 入口趟 1 次；**检查点复用之后第 2 刀起 = 0** ✓。
fn accumulate_prefixes(units: &[SourceUnit<'_>]) -> (Vec<String>, String) {
    CLOSURE_PREFIX_BUILDS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let mut prefixes = Vec::with_capacity(units.len());
    let mut accumulated = String::new();
    for unit in units {
        prefixes.push(accumulated.clone());
        accumulated.push_str(&crate::project::importless_source(&unit.file.src));
        if !accumulated.ends_with('\n') {
            accumulated.push('\n');
        }
    }
    (prefixes, accumulated)
}

/// **A4a 的判据读数**（`#[doc(hidden)]`，只给判据用）：闭包前缀**累加**跑了几次。
static CLOSURE_PREFIX_BUILDS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// 见 [`closure_accumulated_over`]：冷开 = 2（库层趟 + 入口趟）、**第 2 刀起 = 0** ✓。
#[doc(hidden)]
pub fn closure_prefix_builds_total() -> u64 {
    CLOSURE_PREFIX_BUILDS.load(std::sync::atomic::Ordering::Relaxed)
}

/// **T2-B0 的判据读数**（`#[doc(hidden)]`，只给判据用）：**记法表**建了几次
/// （`display_notations` 的唯一实现点 ⇒ 老路与会话路**同口径** ✓）。
static NOTATION_TABLE_BUILDS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// 见 [`display_notations`]：它是**闭包文本的纯函数** ⇒ **冷开 = k（库层趟 + 入口趟）、
/// 复用之后第 2 刀起应当 = 0**（T2-B0 的目标 ✓）。
#[doc(hidden)]
pub fn notation_table_builds_total() -> u64 {
    NOTATION_TABLE_BUILDS.load(std::sync::atomic::Ordering::Relaxed)
}

/// **T2-B0 的判据读数**：**定义 span 表**建了几次（`top_level_def_spans_over` 的唯一实现点 ✓）。
static DEF_SPANS_BUILDS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// 见 [`top_level_def_spans_over`]：与 [`notation_table_builds_total`] 同一条纪律 ✓。
#[doc(hidden)]
pub fn def_spans_builds_total() -> u64 {
    DEF_SPANS_BUILDS.load(std::sync::atomic::Ordering::Relaxed)
}

/// **T2-B0 的第二条读数**：这两张表**一共处理了多少个单元**。
///
/// **为什么必须有它**：T2-B0 的目标不是"把调用次数打到 0"（入口那一段**本来就该每刀建** ✓），
/// 而是"**只随入口规模**" ✓ —— 调用次数**不变**、处理的**单元数**从 `O(闭包)` 掉到 `1` ✓。
/// 只数调用次数会把"已经切开了"误读成"没做" ✗。
static NOTATION_TABLE_UNITS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static DEF_SPANS_UNITS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// 见 [`notation_table_builds_total`]：**单元数**（切分前 = 每刀 O(闭包)、切分后 = 1 ✓）。
#[doc(hidden)]
pub fn notation_table_units_total() -> u64 {
    NOTATION_TABLE_UNITS.load(std::sync::atomic::Ordering::Relaxed)
}

/// 见 [`def_spans_builds_total`]：**单元数**（同上 ✓）。
#[doc(hidden)]
pub fn def_spans_units_total() -> u64 {
    DEF_SPANS_UNITS.load(std::sync::atomic::Ordering::Relaxed)
}

/// **G-85 的键侧入口**：把一段前缀**源码**规范化成**环境身份** ✓（规则见
/// [`closure_prefix_ids_for`] ✓）。
///
/// **为什么要有它**（而不是把身份从编译侧一路传下来 ✗）：`judge` 侧只有**文本** ✓
/// （`ctx.prefix_src` ✓），而键要的是身份 ✓ ⇒ 在这里**解析一次**、按规则取身份 ✓；
/// 调用方（`judge`）**记忆化**它 ✓ ⇒ 每个不同的前缀只解析一次 ✓。
///
/// ⚠ **解析失败 ⇒ 退回原文** ✓（保守 ✓ 安全 ✓ —— 宁可少复用 ✗，绝不多复用 ✗）。
/// **带「成不成」的身份** ✓（2026-10-04 值守判据 ④ 的配套 ✓）：与 [`canonical_prefix_id`]
/// 逐位相同 ✓，但**解析失败时返回 `None`** ✗ 而不是退回原文 ✓。
///
/// **为什么需要它** ✗：`walk.rs` 的**增量身份**是「上一次的身份 + **新增那一段**的身份」✓，
/// 而「新增那一段」是**片段** ✗（一条命令 ✓）—— 片段**可能解析不过** ✗ ⇒ 那时
/// `canonical_prefix_id` 会退回**原文** ✗ ⇒ 拼出来的身份是**错的** ✗（实测：证明体判据
/// 从 `prefix=0` 掉到 **13** ✗）。有了这个函数，调用方就能「**片段不成 ⇒ 退回整体**」✓。
///
/// ⚠ **它不记 `note_prefix_fallback`** ✓：片段探针**不是**真实前缀的退回 ✓ ——
/// 记进去会让判据 ③ 的课程级读数虚高 ✗。
pub fn canonical_prefix_id_checked(src: &str) -> Option<String> {
    // ⚠ **必须用 `parse_fragment`** ✗→✓（2026-10-04 **实测定位** ✓，见 [`canonical_prefix_id`]）：
    // 前缀是**文件的一个片段** ✓ —— 它**故意**可能停在未闭合的 `namespace` 里 ✓
    // （G-05 §4.1，`judge_infer_uncached` 早已按这条口径用 `parse_fragment` ✓）。
    // 用严格 `parse` ⇒ 那种片段**必然**报 `parse-namespace-unclosed` ✗ ⇒ 返回 `None` ✗
    // ⇒ 调用方退回"整体重解析" ✗（慢 ✗）且整体**同样**解析不过 ✗ ⇒ 一路退到**原文** ✗✗。
    let file = crate::parse_fragment(src).ok()?;
    let mut out = String::new();
    for command in &file.commands {
        let id = command_env_id(src, command);
        if id.is_empty() {
            continue;
        }
        out.push_str(&id);
        out.push('\n');
    }
    Some(out)
}

#[track_caller]
pub fn canonical_prefix_id(src: &str) -> String {
    // ⚠ **`parse` → `parse_fragment`** ✗→✓（2026-10-04 **实测定位** ✓ —— 这就是「5 趟」的根因 ✓）：
    // 判定的前缀是**文件的一个片段** ✓ —— 声明落在 `namespace Foo` 里时，前缀**必然**
    // 带着一个还没闭合的 `namespace` ✓（G-05 §4.1；`judge_infer_uncached` 早就为此
    // 用 `parse_fragment` ✓，**只有这里**还在用严格 `parse` ✗）。
    // 实测后果（课程 `unit08` · 自检 1540 条里 **1505** 条 ✗）：严格 `parse` 报
    // `parse-namespace-unclosed` ⇒ 走下面那条**退回原文** ⇒ 键退化成**前缀原文**的哈希
    // ⇒ ① 「只改证明体 ⇒ 后面不重编」这个特性**整段失效** ✗（证明体一改原文就变 ⇒ 全 miss ✗）；
    // ② walker 累加出来的**真身份**与它对不上 ✗ ⇒ **开预置反而多出 5 趟** ✗（那 5 趟的真正机制 ✓）。
    // 其余解析错误（真的坏文本）仍然退回原文 ✓ —— 保守那条**一个字不改** ✓。
    let Ok(file) = crate::parse_fragment(src) else {
        // 判据 ③（值守 2026-10-04 ✓）：**退回原文要计数** ✓ —— 这是"放弃特性"的
        // 保守退路 ✓，**整本课程必须一次都不触发** ✗（触发了就是特性对那个模块失效 ✗）。
        crate::judge::stats::note_prefix_fallback(src, "parse-failed");
        return src.to_string();
    };
    let mut out = String::new();
    for command in &file.commands {
        let id = command_env_id(src, command);
        // **空身份不进** ✓（`import` 与 `example` 都是空 ✓）：否则会多出一个换行 ✗，
        // 让"改 `example` 不惊动任何东西"这条判据假红 ✗。
        if id.is_empty() {
            continue;
        }
        out.push_str(&id);
        out.push('\n');
    }
    out
}

/// 一条命令贡献给环境的**身份**（见 [`closure_prefix_ids_for`] 的规则 ✓）。
///
/// ⚠ 这里只做**文本切片**，不做语义判断 ✓ —— "证明体从哪开始"用的是 AST 里 `val` 的
/// **span** ✓（不是找 `:=` ✗：注释里也可能有 `:=` ✓）。
fn command_env_id(src: &str, command: &Command) -> String {
    // **零长 span 的命令不进身份** ✓（2026-10-04 **实测定位** ✓）：它**没有源码文本** ✓ ——
    // 判官合成的那批 `_soko_judge_*` 声明就是这种（span 是**默认值**：`line: 0, column: 0` ✓）。
    // 身份是**从源码文本**推出来的 ✓（[`canonical_prefix_id`] 只看得见文本里的命令 ✓）
    // ⇒ 不跳过它，两条路就会分叉 ✗：文本路**没有**这一条、walker 的 AST 累加**有** ✗
    // （实测：课程 `unit08` 自检 **18 处** ✗，全是 `def#` 这个空条目 ✓）。
    // 判据 ④ 要求两条路**逐位相等** ✓ ⇒ 这一条必须两边一致 ✓。
    if command.span().start.offset >= command.span().end.offset {
        return String::new();
    }
    // `import` 不是声明 ✓：库层的内容已经在**更早的单元**里进了身份 ✓。
    if matches!(command, Command::Import { .. }) {
        return String::new();
    }
    let full = || slice_span(src, command.span());
    // `example` **完全不进** ✓（值守口径 ✓）：它**匿名** ⇒ 下游**引用不到**它 ✓
    // ⇒ 改它（连类型一起改）**不该惊动任何东西** ✓。
    if matches!(command, Command::Example { .. }) {
        return String::new();
    }
    let statement = match command {
        Command::Theorem { ty, .. } => {
            // **名字 + 类型** = 从命令起点切到**类型表达式的终点** ✓（`:=` 与证明体都不进 ✓）。
            // ⚠ 别用 `val.span().start` ✗ —— 实测它的起点**不在证明体之前** ✗
            // （`theorem t (n : Nat) : f n = n := …` 切出来只有 `theorem t ` ✗ ⇒ 陈述没进身份 ✗）。
            let head = slice_span(src, Span::new(command.span().start, ty.span().end));
            // **保守闸**：陈述里有独立 `_` ⇒ 证明体可能反过来定类型 ⇒ 整条都算 ✓。
            if has_bare_hole(&head) {
                full()
            } else {
                head
            }
        }
        // `def` 等：体是环境内容 ✓ ⇒ 整条 ✓。
        _ => full(),
    };
    format!("{}#{}", command_kind_tag(command), statement)
}

/// 命令类别标签 ✓（同一段文本在 `theorem` 与 `def` 下语义不同 ✗ ⇒ 标签必须进身份 ✓）。
fn command_kind_tag(command: &Command) -> &'static str {
    match command {
        Command::Def { .. } => "def",
        Command::Theorem { .. } => "theorem",
        Command::Example { .. } => "example",
        Command::Axiom { .. } => "axiom",
        Command::InductiveBlock { .. } => "inductive",
        Command::Notation { .. } => "notation",
        Command::Namespace { .. } => "namespace",
        Command::End { .. } => "end",
        Command::Open { .. } => "open",
        Command::OpenIn { .. } => "open-in",
        Command::Export { .. } => "export",
        Command::Check { .. } => "check",
        Command::Reduce { .. } => "reduce",
        Command::Print { .. } => "print",
        _ => "other",
    }
}

/// 按 span 切一段源码（越界退化成空串 ✓ —— 调用方拿不准时会退回**整条原文** ✓，
/// 所以这里越界**不会**造成"少算" ✗）。
fn slice_span(src: &str, span: Span) -> String {
    src.get(span.start.offset..span.end.offset)
        .unwrap_or_default()
        .to_string()
}

/// 有没有**独立的 `_`**（前后都不是标识符字符 ✓）—— 保守闸用 ✓（见 [`closure_prefix_ids_for`] ✓）。
fn has_bare_hole(text: &str) -> bool {
    let bytes = text.as_bytes();
    let is_word = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
    bytes.iter().enumerate().any(|(i, &b)| {
        b == b'_'
            && (i == 0 || !is_word(bytes[i - 1]))
            && (i + 1 == bytes.len() || !is_word(bytes[i + 1]))
    })
}

/// **G-29 第 5 棒**：一个单元看到的**建议材料**（`GoalTemplates`）= 它闭包前缀里
/// 所有单元的命令 + 它自己的（与 [`run_pass_with`] 多单元分支的**最后一格**同构 ✓）。
///
/// 会话的入口趟 `units` 只有入口 ⇒ 必须由调用方把**闭包**传进来（见
/// `template_closure` 参数），否则被导入模块里的构造子不进 refine/intro 建议 ✗。
fn templates_for_closure(closure: &[SourceUnit<'_>], options: &CompileOptions) -> GoalTemplates {
    let mut commands: Vec<Command> = Vec::new();
    for unit in closure {
        commands.extend(unit.file.commands.iter().cloned());
    }
    let combined = FolFile {
        commands,
        src: String::new(),
    };
    GoalTemplates::new_for(&combined, options)
}

/// **切片 1b**：`builder`（与可选影子）**由调用方提供、编译完交回** ⇒ session 能把
/// **同一套 DAG** 交给每个入口（库层只编一次；`restore_declars`/`hide_declars`
/// 检查点由 session 做）。`'a: 's` 是必要的：影子要重放主 arena 产出的
/// `Declar<'arena>`（`ops`），只有主环境寿命 ⊇ 影子才合法 ✓。
#[allow(clippy::too_many_arguments)] // 与 run_pass_in 同参数表 + builder/shadow（切片 1b）
pub(crate) fn run_pass_with<'a, 's>(
    mut builder: EnvBuilder<'a>,
    mut shadow: Option<EnvBuilder<'s>>,
    // **只装一次**：session 第二趟起必须 `false`（实测重复装 ⇒ `duplicate declaration Nat` ✗）。
    install_preludes: bool,
    // **切片 1b**：prelude 登记表（跨趟复用；库层趟装好，入口趟接着用）。
    tables: PassTables<'a>,
    // **T2-B（探针）**：units 与 arena 寿命**解绑**（`&[SourceUnit<'_>]`）——
    // 只有解绑之后，"入口趟跑在 `'static` arena 上"才可能与"units 是本次调用的"
    // 同时成立 ⇒ 命令级快照才存得进线程局部 ✓。
    units: &[SourceUnit<'_>],
    options: &CompileOptions,
    collect: bool,
    skip: Option<&KernelFailed>,
    trust: Option<&TrustPlan>,
    progress: Option<&mut dyn crate::compile::ProgressSink>,
    // **切片 1（G-68）路乙**：调用方可**显式覆盖**闭包前缀与记法表 ——
    // session 的入口趟要用"**该入口闭包**"的那一份（否则 `judge_infer` 看不到库层，
    // 见 §29）。`None` ⇒ 按 `units` 自己算（**今天的行为，逐字节不变** ✓）。
    closure_prefixes_override: Option<&[String]>,
    display_override: Option<&crate::display::DisplayNotations>,
    // **切片 1b 的入口趟**：跨模块 hover 回填用的 `名字 → 定义 span` 表。
    // `None` ⇒ 按 `units` 自己算（**今天的行为，逐字节不变** ✓）。
    defs_override: Option<&std::collections::HashMap<String, crate::Span>>,
    // **G-29 第 5 棒**：闭包级**建议材料**（`GoalTemplates`）的覆盖 —— 传"该入口
    // **闭包**"的单元（拓扑序、入口在最后）⇒ 取**最后一格**。
    //
    // 为什么需要：会话的**入口趟** `units` 只有入口（长度 1）⇒ 下面那条
    // `units.len() == 1` 分支会**只按入口文件**建模板 ⇒ 被导入模块里的构造子/函数
    // **不进** refine/intro 建议 ✗（实测：`crates/lsp/src/tests/project.rs::
    // code_actions_work_in_a_project_entry` 的 `refine And.intro` 当场消失 ✗ ——
    // 这是 §29「三次接线失败」的**第四件**同类漏接线：凡"按 units 算的闭包上下文"
    // 都要显式覆盖 ✓）。`None` ⇒ 按 `units` 自己算（**今天的行为，逐字节不变** ✓）。
    template_closure: Option<&[SourceUnit<'_>]>,
    // **G-29 第 3 棒**：本趟 `idx` 相对 **judge 合成文档前缀坐标系**的平移量
    // （= 闭包里**排在本趟 units 之前**的命令数 ✓）。`0` ⇒ 同坐标系
    // （老路 / 单文件 / 库层趟 ⇒ **逐字节回到今天** ✓）；session 的**入口趟**
    // 传"库层那一段的命令数"（见 `project/session.rs` ✓）。
    judge_prefix_offset: usize,
    // **T1-A（2026-10-09）**：本趟结束时把 [`ResumeState`]（`closure_id`/`exports`/
    // `example_idx`）交回来。`false` ⇒ 返回 `None`（**与今天逐字节相同** ✓）；
    // 只有 `project/session.rs` 的**逐模块库层趟**传 `true`。
    snapshot_state: bool,
    // **T1-A**：本趟是"**接着某个模块边界继续编**" ⇒ 用它的 walk 状态起跑
    // （见 [`ResumeState`]）。`None` ⇒ **与今天逐字节相同** ✓。
    resume: Option<ResumeState>,
    // **T2-B（2026-10-09）**：从一份**命令边界快照**续编（`flat[..=cp.idx]` 整段跳过）。
    // `None` ⇒ **与今天逐字节相同** ✓。
    //
    // 类型是 `walk::WalkCheckpoint<'a, 's>`：它只借 arena（`'a`）**不借源码**
    //（`Walk` 结构体不带 `'src`）⇒ 只有"入口趟跑在 `'static` arena 上"那一条路
    // 才存得进线程局部 ✓（会话的 ③ 重建路就是这样；栈上 arena 那条路传 `None` ✓）。
    resume_walk: Option<walk::WalkCheckpoint<'a>>,
    // **T2-B**：跑完取一份**边界快照**交回调用方（`false` ⇒ 与今天逐字节相同 ✓）。
    snapshot_walk: bool,
    // **T2-B 的读数开关**：这一趟是**入口趟** ⇒ 它的命令数进
    // [`elaborated_commands_total`]（判据的"elaborate 命令数"只数入口趟 ✓）。
    count_entry_commands: bool,
) -> (
    PassResult,
    EnvBuilder<'a>,
    PassTables<'a>,
    Option<ResumeState>,
    Option<walk::WalkCheckpoint<'a>>,
)
where
    'a: 's,
{
    stage_stats::install();
    stage_stats::PASSES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    // **闭包模块编译次数**（G-29 的精确判据读数，2026-10-07）：只数**真模块**
    // （`path: Some(..)` = 来自 `plan.closure.compilable()` 的源文件），**不数**
    // judge 合成的判定文档（`SourceUnit::single("", file)` ⇒ `path: None`）。
    //
    // 为什么必须分开数（实测归因，见设计 §33）：`MODULE_COMPILES` 只在 `check::run`
    // 里按 `units.len()` 累加，而**会话那条路**（`project/session.rs` 的库层趟 + 入口趟）
    // 走的是本函数 ⇒ 一次都不计 ✗；于是 `LSP_TRACE` 的 `modules=` 在**改一行**那一刀上
    // 读到的是 **7 次 judge 合成编译**（`compile_fol_with` ← `judge_infer_uncached`），
    // **不是**闭包模块编译（真实值 = 库层 4 + 入口 1 = 5，一次都没被计）✗。
    // ⇒ 「改一行 `modules=7` = 整条闭包重编」是**误归因**；本计数才是那句话的读数。
    //
    // ⚠ 计数点选在本函数（**所有 pass 的唯一收口**：`run`/`run_incremental` 都经
    // `run_pass_in` 到这里，session 直接调它 ✓）⇒ 老路与会话路**同口径** ✓。
    // `check-then-add` 的 pass 2 会**再计一次**（那确实又编了一遍 ✓）。
    let closure_modules = units.iter().filter(|u| u.path.is_some()).count();
    if closure_modules > 0 {
        stage_stats::CLOSURE_MODULE_COMPILES
            .fetch_add(closure_modules as u64, std::sync::atomic::Ordering::Relaxed);
    }
    let _pass_timer = StageTimer(&stage_stats::PASS_NANOS, std::time::Instant::now());
    // **切片 1b**：三张表由调用方提供（session 跨趟复用）⇒ 这里不再 `new()`。
    let mut tables = tables;
    if install_preludes {
        install_all_preludes(
            &mut builder,
            &mut tables.known,
            &mut tables.inductives,
            &mut tables.defs,
            units,
            options,
        );
    }
    // **影子环境**（T-K12b）：一份**只给 judge 用**的环境。prelude 走**同一个助手**
    // ⇒ 条件与顺序不可能与主环境分叉 ✓（分叉 = 判定义分叉 = 红线）。
    // 它从 walk 的 `ops` **惰性重放**（不用就零成本 ✓）。
    // ⚠ **默认不建**（`SOKO_SHADOW_CHECK` 才建）：影子环境是 T-K12b 的**实验品**
    // ——对照判据已判定它与内核阶段**不等价** ✗（差在增量记账：`skip`/`trust`/
    // pass1-pass2 ⇒ 影子偏严），所以它**不能**进判定路径。但每次编译都白装一份
    // prelude 是**热路径成本** ✗ ⇒ 关进开关：默认零成本 ✓、要复现对照实验时打开 ✓。
    // 结论与后续见 `docs/design/vscode-editor-feedback-plan.md` 的 T-K12b。
    // **STRICT 也必须建影子**（2026-09-25 round 219 实测）：否则 `SOKO_SHADOW_STRICT=1`
    // 单独用时影子根本没建 => 断言所在的分支不执行 => 那个开关等于空转 ✗
    // （实测：STRICT=1 => MISMATCH=0 且退出码 0 ✗）。
    let shadow_experiment =
        std::env::var("SOKO_SHADOW_CHECK").is_ok() || std::env::var("SOKO_SHADOW_STRICT").is_ok();
    if shadow_experiment {
        if let Some(sh) = shadow.as_mut() {
            install_all_preludes(
                sh,
                &mut KnownTable::new(),
                &mut InductiveTable::new(),
                &mut DefTable::new(),
                units,
                options,
            );
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
    let all_templates: Vec<GoalTemplates> = match template_closure {
        // 会话的**入口趟**：闭包前缀 + 入口（= 与多单元分支的**最后一格**同构 ✓）。
        Some(closure) => vec![templates_for_closure(closure, options)],
        None if units.len() == 1 => vec![GoalTemplates::new_for(units[0].file, options)],
        None => {
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
        }
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
    // **路乙**：调用方给了就用它的（session 入口趟传"该入口闭包"的那份）；
    // 没给就按 `units` 自己算（**今天的行为** ✓）。
    let closure_prefixes_owned: Vec<String>;
    let closure_prefixes: &[String] = match closure_prefixes_override {
        Some(p) => p,
        None => {
            closure_prefixes_owned = closure_prefixes_for(units);
            &closure_prefixes_owned
        }
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
        display: match display_override {
            Some(d) => d.clone(),
            None => display_notations(units),
        },
        builder,
        known: tables.known,
        inductives: tables.inductives,
        defs: tables.defs,
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
        // **本趟 pass 自己的**「成功进环境」名表（G-31/G-92 第二刀 ✓）。
        entered: std::rc::Rc::new(std::cell::RefCell::new(std::collections::HashSet::new())),
        // 调用方那一趟的名表：**只有 judge 的合成编译**会设（`TrustPlan` ✓），
        // 其余一律 `None` ⇒ 不透明快路**关** ⇒ 逐字节回到今天 ✓。
        trusted_entered: trust.and_then(|t| t.trusted_entered.clone()),
        // **G-29 第 3 棒**：judge 合成文档前缀的坐标系平移量（默认 0 ✓）。
        judge_prefix_offset,
        // **T1-A**：续编状态（`None` = 从头起跑 ⇒ 与今天逐字节相同 ✓）。
        resume,
        snapshot_state,
        resume_out: None,
    };
    // **T2-B**：`resume_walk` ⇒ 从命令边界快照续编；`snapshot_walk` ⇒ 跑完取一份
    // 尾边界快照交回调用方（两者默认 `None`/`false` ⇒ **逐字节回到今天** ✓）。
    let walk_tail = walk.run(
        units,
        &flat,
        options,
        skip,
        trust,
        &all_templates,
        closure_prefixes,
        progress,
        resume_walk.as_ref(),
        snapshot_walk,
        count_entry_commands,
    );
    // **T-K12b 的一致性观测**：把影子环境推进到"全部已 elaborate 的前缀"
    // （`finish_pass` 会把 `walk` 的字段移走 ⇒ 必须在这之前取数 ✓）。
    // `SOKO_SHADOW_CHECK=1` 时打印影子的规模与失败数，供与内核阶段对照
    // ——"影子可不可信"就是靠这条观测来判的（下一步升级成断言 ✓）。
    let shadow_decls = if shadow_experiment {
        walk.shadow_env().map_or(0, |sh| sh.declaration_count())
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
        builder: &mut walk.builder,
        out: walk.out,
        report,
        ops: walk.ops,
        cmd_hovers: walk.cmd_hovers,
        decl_states: walk.decl_states,
        failed_cmds,
        kernel_checks,
        defs_override,
    });
    // **切片 1b**：把 prelude 登记表从 walk 取回 ⇒ 交回调用方（session 跨趟复用）。
    let tables = PassTables {
        known: walk.known,
        inductives: walk.inductives,
        defs: walk.defs,
    };
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
        // **T-D3：把观测升级成断言**（2026-09-25 round 212 ✓）。
        // 设计 `docs/design/e2-plan.md` 的 T-D3 原文是"下一步升级成断言" ✓ ——
        // 影子走查（`Walk::shadow_env` / `shadow_check_and_add` ✓）与对照比较（T-K12b ✓）
        // **早已存在** ✓，缺的只是"**不一致就判红**" ✗。
        //
        // 只在 `SOKO_SHADOW_CHECK=1`（**既有的 opt-in 开关** ✓，默认关 ✓）下失败
        // => 默认路径**零变化零风险** ✓；开关打开时不一致 = **影子不可信** ✓，
        // 那正是要判红的事情 ✓（`shadow_failed` 已去重、只比影子覆盖的命令 ✓，
        // 见上面的注释 —— 假差异的两个来源都排掉了 ✓）。
        //
        // ⚠ **先打印再失败** ✓：编译路径上 panic 会被 `quiet_catch` 捕获并转成诊断 ✓，
        // 只 panic 的话上面那行关键信息会丢 ✗。
        // **断言另设开关**（2026-09-25 round 219）：`SOKO_SHADOW_CHECK=1` 只观测
        // （恢复它原本的用途：打开看 SHADOW 观测行），`SOKO_SHADOW_STRICT=1` 才断言。
        // 原因：影子与内核阶段的不等价是**已知**的（见本函数上方 :768-772 的注释：
        // 差在增量记账 skip/trust/pass1-pass2 => 影子偏严），若在 CHECK 下就 panic，
        // 那个开关就再也无法用来观测了。
        if std::env::var("SOKO_SHADOW_STRICT").is_ok() && shadow_failed != kernel_failed {
            eprintln!(
                "SHADOW MISMATCH（影子与内核阶段的失败表不一致 ⇒ 影子不可信 ✗）：\
                 shadow_failed={shadow_failed:?} kernel_failed={kernel_failed:?} \
                 差分只在影子覆盖的命令上 ✓（见 shadow_covered 的注释 ✓）"
            );
            panic!(
                "T-D3 影子一致性断言失败：shadow_failed != kernel_failed \
                 （SOKO_SHADOW_CHECK=1 时这是硬判据 ✓）"
            );
        }
    }
    (
        pass,
        walk.builder,
        tables,
        walk.resume_out.take(),
        walk_tail,
    )
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
    // **T2-B0 的判据读数**（2026-10-09）：同 `display_notations` —— 闭包文本的纯函数、
    // 今天每刀重算 ✗。
    DEF_SPANS_BUILDS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    DEF_SPANS_UNITS.fetch_add(units.len() as u64, std::sync::atomic::Ordering::Relaxed);
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
        by_root: None,
        hints: Vec::new(),
        ty_text: None,
        val_text: None,
    }
}

/// Infer a type per recorded sub-expression (in its binder scope) and render
/// it as text. Panics (kernel rejection on intermediate sub-terms) are caught
/// per node so one bad sub-term cannot kill the hover map.
///
/// **E04（0.74.0）**：这里的文本是**显示面**（编辑器 / Infoview 的 `expr : type` 行）
/// ⇒ 必须与另外三处（声明 `ty_text`、Infoview `⊢`、`by` 步进的 goals）**走同一个
/// 折叠入口** `display.fold` ✓。以前它是内核 pp **直出** ✗ ⇒ 悬停里漏出
/// `Set.subset α A B` / `forall …` 这类点形式（用户看得见 ✗）。判据：
/// `crates/front/src/compile/tests.rs::hover_types_are_folded_like_the_other_display_surfaces`。
pub(crate) fn resolve_hovers(
    env: &sokonanoda::util::ExportFile<'_>,
    display: &crate::display::DisplayNotations,
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
                    // **E04**：pp 之后**必须过折叠**（`$N` 还原在前、折叠在后——
                    // 折叠是纯文本改写，与松散变量名无关 ✓）。
                    Ok(t) => display.fold(&name_loose_bvars(&t, &node.scope_names)),
                    // infer_under_binders panic（delta 展开限制）：保留 span、
                    // text 置空——LSP 层的括号回退仍能定位到正确的子表达式，
                    // hover 显示源码切片（不带类型后缀）。
                    // ⚠ **静默降级是这条路的病**（2026-10-10 用户实测的 `#check`
                    // hover「只有符号、没有类型」）：panic 的文本被吞成空串 ⇒
                    // 上层只看到"没有类型"✗。内核侧已按 Lean 4 补齐层元变量
                    // （`level.rs::leq_core`，合法输入不再走到这里），这一臂留作
                    // 安全网；`SOKO_HOVER_PANIC_TRACE=1` 时把原因打出来（诊断出口，
                    // 与 `SOKO_HOVER_TRACE` / `SOKO_POS_TRACE` 同一纪律）。
                    Err(p) => {
                        if std::env::var("SOKO_HOVER_PANIC_TRACE").is_ok() {
                            let msg = p
                                .downcast_ref::<String>()
                                .cloned()
                                .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
                                .unwrap_or_else(|| "<non-string panic>".to_string());
                            eprintln!("[hover-panic] span={:?} msg={}", node.span, msg);
                        }
                        String::new()
                    }
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

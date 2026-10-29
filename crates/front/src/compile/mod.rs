//! Compile a parsed `.sokonanoda` file into kernel declarations and run the
//! complete sokonanoda kernel over them.

mod check;
// **B2-P4a**：声明级元变量档的判据（在 `ctx`/重放上，不在进程环境变量上）——
// `judge` 的缓存键要按它分槽（开闸时的 pp 文本带 `⟨meta#N⟩`，与闸门关共槽会互相污染）。
pub(crate) use check::meta_exit::gate_path_active;
mod dialect;
pub(crate) mod elab;
mod error;
// `ErrorKind::hint()` 的文案表（从 `error.rs` 拆出来腾判据 ⑨ 的行数预算，2026-10-16）。
mod error_hint;
// 命名实参（`g (α := Nat) 5`，用户 2026-10-16 第 ② 条）。
pub(crate) mod eqsubst;
mod eqsubst_diff;
mod event;
mod expected;
mod expected_spine;
mod goals;
mod mvar_handle;
// 实参位的洞（`_`，a14）：`_` → 新 binder ⇒ `apply` 把它变成子目标（Lean 4 对照在文件头）。
pub mod hints;
pub(crate) mod hole;
mod implicit;
// P1-F1：按需从**内核声明类型**取参数望远镜（P1-F2 的内核级 `elab_app` 用）。
mod kernel_app;
mod kernel_tel;
mod named_arg;
// IA-5 的错误文案（**单独一个文件** ⇒ `elab.rs`/`implicit.rs` 都受判据 ⑨ 冻结，
// **只许减不许增** ✗ ⇒ 文案长在那边会当场判红 ✓）。
// **a15**：`by.rs`（在 `compile` 之外）要读 `cases_scrutinee_type` ⇒ 这一处放开到 crate。
pub(crate) mod implicit_msg;
mod level;
mod meta;
mod pp_shape;
mod prelude;
mod report;
mod scope;
mod units;
mod warning;

pub mod cache;

pub use check::by_calls_total;
/// **G-85**：把前缀**源码**规范化成**环境身份**（判定缓存的键用它 ✓，判定合成仍用原文 ✓）。
pub use check::canonical_prefix_id;
/// 同上，但**解析失败返回 `None`** ✓（增量身份的"片段探针"用 ✓，见 `check/walk.rs` ✓）。
pub use check::canonical_prefix_id_checked;
/// **A4a（2026-10-08）的判据读数**：闭包前缀累加次数（冷开 2、检查点复用后 0）。
pub use check::closure_prefix_builds_total;
pub(crate) use check::WalkCheckpoint;
// **T2-B0 的两条读数**（2026-10-09）—— 与上面那条同形 ✓。
pub use check::{
    check_document, check_document_with, closure_accumulated_over, closure_prefixes_for,
    compile_all_with, compile_fol, compile_fol_with, display_notations,
    display_notations_from_commands, fold_for_display, prelude_shape, render_expr, PreludeShape,
};
pub use check::{
    closure_module_compiles_total, elaborated_commands_total, module_compiles_total,
    note_module_compile,
};
pub(crate) use check::{
    closure_prefixes_and_total, install_all_preludes, run_incremental, run_pass_with,
    top_level_def_spans, top_level_def_spans_over, KernelFailed, PassTables, ResumeState,
    TrustPlan,
};
pub use check::{
    def_spans_builds_total, def_spans_units_total, notation_table_builds_total,
    notation_table_units_total,
};
/// **方言**（S1）：[`CompileOptions::dialect`] 的类型。S3/S5 的 CLI 旗标与编辑器设置
/// 要能**命名**它 ⇒ 从 `dialect`（`mod`）公开再导出 ✓。
///
/// S4（2026-10-20）再加三个**来源**：文件指令解析器（[`explicit_dialect`]）、
/// 环境变量的表态（[`dialect_from_env`]）与五档合并点（[`DialectSources`]）——
/// 调用方（CLI 现在、LSP 在 S5）只往 `DialectSources` 里填格子，**不许**自己写 if 链 ✓。
pub use dialect::{dialect_from_env, explicit_dialect, Dialect, DialectSources};
pub(crate) use elab::canonical_ctor_name;
pub use error::{CompileError, CompileStage, ErrorKind};
pub use event::{CheckEvent, CompileOutput, CompileStats};
pub use goals::{probe_sub_goal_types, probe_sub_goal_types_with};
pub use hints::{attach_hints, attach_hints_to_report, source_hints};
/// **telescope 解析次数**（A4b 的判据读数）：先读数、后谈签名级缓存 ✓。
pub use implicit::telescope_parses_total;
pub use prelude::{
    explicit_prelude_mode, prelude_def_span, prelude_eq_src, prelude_l1_src,
    prelude_mode_from_source, prelude_override_error, prelude_override_state, prelude_source,
    prelude_source_path, quot_types_src, CompileOptions, PreludeMode, PRELUDE_EQ_SRC,
    PRELUDE_L1_SRC, PRELUDE_NAMES, PRELUDE_NEVER_YIELDS,
};
pub use report::{
    ByGoalState, ByStepState, CheckInfo, DeclKind, DeclState, DeclStatus, DocumentReport,
    GoalBinder, HoleMode, HoverType, PrintInfo, ResolvedTarget, SubGoal, REPORT_SHAPE,
};
pub(crate) use scope::{join_ns, NamespaceScope, OpenEntry};
/// **`TcCache` 构造次数**（2026-10-08 端到端 profiling 的新读数）：判据用它数
/// "一次按键构造了几次 `TcCache`"（每次预分配 ≈ 4 MiB + 20 张表 ⇒ 实测 61.8 µs/次）。
pub use sokonanoda::util::tc_cache_builds_total;
pub use units::{
    compile_all_units, compile_all_units_with_progress, split_report, unit_ranges, ProgressSink,
    ProgressTick, SourceUnit,
};
pub use warning::{collect_warnings, CompileWarning, WarningKind, RESERVED_SORT_NAMES};

#[cfg(test)]
mod tests;
// `Eq.subst` 的谓词位（用户 2026-10-15 亲报 `SUB-eqsubst-clue-order-clash`）：
// 用例落在 `tests/`（`docs-lint` 判据 ⑨ 把测试聚合文件排除在外 ✓ —— 与
// `lib.rs` 的 `parser_boundary_tests` 同一条口径 ✓）。
#[cfg(test)]
#[path = "tests/eqsubst_predicate.rs"]
mod eqsubst_predicate_tests;
// P1-F1：内核 telescope 的形状 / 全课程库一致性扫描 / 暖路径（产物装载）。
#[cfg(test)]
#[path = "tests/kernel_tel.rs"]
mod kernel_tel_tests;
// P1-F2b2：strict + 闸门开 ⇒ 应用走内核 `elab_app`（线程局部覆盖）。
#[cfg(test)]
#[path = "tests/kernel_app.rs"]
mod kernel_app_tests;

//! Compile a parsed `.sokonanoda` file into kernel declarations and run the
//! complete sokonanoda kernel over them.

mod check;
pub(crate) mod elab;
mod error;
mod event;
mod goals;
pub mod hints;
mod implicit;
mod level;
mod meta;
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
    GoalBinder, HoverType, PrintInfo, ResolvedTarget, SubGoal, REPORT_SHAPE,
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

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
pub use check::{
    check_document, check_document_with, closure_prefixes_for, compile_all_with, compile_fol,
    compile_fol_with, display_notations, display_notations_from_commands, fold_for_display,
    prelude_shape, render_expr, PreludeShape,
};
pub use check::{closure_module_compiles_total, module_compiles_total, note_module_compile};
pub(crate) use check::{
    run_incremental, run_pass_with, top_level_def_spans, top_level_def_spans_over, KernelFailed,
    PassTables, TrustPlan,
};
pub(crate) use elab::canonical_ctor_name;
pub use error::{CompileError, CompileStage, ErrorKind};
pub use event::{CheckEvent, CompileOutput, CompileStats};
pub use goals::{probe_sub_goal_types, probe_sub_goal_types_with};
pub use hints::{attach_hints, attach_hints_to_report, source_hints};
pub use prelude::{
    explicit_prelude_mode, prelude_def_span, prelude_mode_from_source, prelude_source,
    prelude_source_path, CompileOptions, PreludeMode, PRELUDE_EQ_SRC, PRELUDE_L1_SRC,
    PRELUDE_NAMES, PRELUDE_NEVER_YIELDS,
};
pub use report::{
    ByGoalState, ByStepState, CheckInfo, DeclKind, DeclState, DeclStatus, DocumentReport,
    GoalBinder, HoverType, ResolvedTarget, SubGoal, REPORT_SHAPE,
};
pub(crate) use scope::{join_ns, NamespaceScope};
pub use units::{
    compile_all_units, compile_all_units_with_progress, split_report, unit_ranges, ProgressSink,
    ProgressTick, SourceUnit,
};
pub use warning::{collect_warnings, CompileWarning, WarningKind, RESERVED_SORT_NAMES};

#[cfg(test)]
mod tests;

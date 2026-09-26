//! Compile a parsed `.sokonanoda` file into kernel declarations and run the
//! complete sokonanoda kernel over them.

mod check;
pub(crate) mod elab;
mod error;
mod event;
mod goals;
pub mod hints;
mod implicit;
mod prelude;
mod report;
mod scope;
mod units;
mod warning;

pub mod cache;

pub use check::{
    check_document, check_document_with, compile_all_with, compile_fol, compile_fol_with,
    display_notations_from_commands, fold_for_display, prelude_shape, render_expr, PreludeShape,
};
pub(crate) use check::{run_incremental, top_level_def_spans, TrustPlan};
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
    GoalBinder, HoverType, ResolvedTarget, SubGoal,
};
pub(crate) use scope::{join_ns, NamespaceScope};
pub use units::{compile_all_units, split_report, unit_ranges, SourceUnit};
pub use warning::{collect_warnings, CompileWarning, WarningKind, RESERVED_SORT_NAMES};

#[cfg(test)]
mod tests;

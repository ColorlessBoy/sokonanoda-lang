//! Compile a parsed `.sokonanoda` file into kernel declarations and run the
//! complete sokonanoda kernel over them.

mod check;
mod elab;
mod error;
mod event;
mod goals;
pub mod hints;
mod prelude;
mod report;
mod warning;

pub mod cache;

pub use check::{
    check_document, check_document_with, compile_all_units, compile_all_with, compile_fol,
    compile_fol_with, render_expr, split_report, unit_ranges, SourceUnit,
};
pub(crate) use check::{run_incremental, top_level_def_spans, TrustPlan};
pub use error::{CompileError, CompileStage, ErrorKind};
pub use event::{CheckEvent, CompileOutput, CompileStats};
pub use goals::probe_sub_goal_types;
pub use hints::{attach_hints, attach_hints_to_report, source_hints};
pub use prelude::{
    explicit_prelude_mode, prelude_mode_from_source, CompileOptions, PreludeMode, PRELUDE_NAMES,
};
pub use report::{
    ByGoalState, ByStepState, CheckInfo, DeclKind, DeclState, DeclStatus, DocumentReport,
    GoalBinder, HoverType, ResolvedTarget, SubGoal,
};
pub use warning::{collect_warnings, CompileWarning, WarningKind, RESERVED_SORT_NAMES};

#[cfg(test)]
mod tests;

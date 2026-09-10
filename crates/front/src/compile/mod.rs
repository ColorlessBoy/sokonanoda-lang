//! Compile a parsed `.sokonanoda` file into kernel declarations and run the
//! complete sokonanoda kernel over them.

mod check;
mod elab;
mod error;
mod event;
pub mod hints;
mod prelude;
mod report;

pub use check::{check_document, check_document_with, compile_fol, compile_fol_with, render_expr};
pub(crate) use check::{run_incremental, TrustPlan};
pub use error::{CompileError, CompileStage, ErrorKind};
pub use event::{CheckEvent, CompileOutput, CompileStats};
pub use hints::{attach_hints, attach_hints_to_report, source_hints};
pub use prelude::{prelude_mode_from_source, CompileOptions, PreludeMode, PRELUDE_NAMES};
pub use report::{
    ByStepState, DeclKind, DeclState, DeclStatus, DocumentReport, GoalBinder, HoverType,
    ResolvedTarget, SubGoal,
};

#[cfg(test)]
mod tests;

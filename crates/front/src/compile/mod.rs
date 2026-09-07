//! Compile a parsed `.sokonanoda` file into kernel declarations and run the
//! complete sokonanoda kernel over them.

mod check;
mod elab;
mod error;
mod event;
mod prelude;
mod report;

pub use check::{check_document, check_document_with, compile_fol, compile_fol_with, render_expr};
pub(crate) use check::{run_incremental, TrustPlan};
pub use error::{CompileError, CompileStage, ErrorKind};
pub use event::{CheckEvent, CompileOutput, CompileStats};
pub use prelude::{prelude_mode_from_source, CompileOptions, PreludeMode};
pub use report::{DeclKind, DeclState, DeclStatus, DocumentReport, GoalBinder, HoverType};

#[cfg(test)]
mod tests;

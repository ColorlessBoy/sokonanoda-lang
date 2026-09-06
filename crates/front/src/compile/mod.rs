//! Compile a parsed `.sokonanoda` file into kernel declarations and run the
//! complete sokonanoda kernel over them.

mod check;
mod elab;
mod error;
mod event;
mod prelude;
mod report;

pub use check::{check_document, compile_fol, render_expr};
pub use error::{CompileError, CompileStage, ErrorKind};
pub use event::{CheckEvent, CompileOutput};
pub use report::{DeclKind, DeclState, DeclStatus, DocumentReport, HoverType};

#[cfg(test)]
mod tests;

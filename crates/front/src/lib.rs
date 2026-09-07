//! The restricted `.sokonanoda` front-end.
//!
//! This crate deliberately implements only the grammar points exposed by the
//! teaching curriculum. The syntax whitelist is the curriculum: adding a
//! grammar point here means adding a lesson for it.

pub mod compile;
pub mod proof;
pub mod semantic;
pub mod session;

mod ast;
mod diagnostic;
mod parser;
mod span;
mod token;

pub use ast::{Binder, BinderKind, Command, CtorDecl, Expr, FolFile, IotaRule, RecDecl, SortKind};
pub use diagnostic::{Diagnostic, DiagnosticKind, Result};
pub use parser::{parse, Parser};
pub use span::{Pos, Span};
pub use token::{tokenize, Lexer, Token, TokenKind};

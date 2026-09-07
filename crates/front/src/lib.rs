//! The restricted `.sokonanoda` front-end.
//!
//! This crate deliberately implements only the grammar points exposed by the
//! teaching curriculum. The syntax whitelist is the curriculum: adding a
//! grammar point here means adding a lesson for it.

// CompileError 携带消息 + span + 期望/实际两端文本，略超 clippy 默认的
// 128 字节 Result 阈值。教学编译器的错误路径不是热点，装箱反而增加分配，
// crate 级显式豁免并在此注明取舍。
#![allow(clippy::result_large_err)]

pub mod compile;
pub mod judge;
pub mod proof;
pub mod references;
pub mod semantic;
pub mod session;
pub mod suggest;

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

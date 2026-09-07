//! Placeholder:
//! ```ignore
//! Doc comment example
//! ```
#![allow(clippy::too_many_arguments)]
// lang：上游快照自带该 deny，但上游代码未过此 lint。降为 warn，避免冻结
// 快照被 lint 搅动；教学 crates 的严格门禁通过各自的 [lints] 表实现。
#![warn(clippy::cast_possible_truncation)]

pub mod conv;
pub mod builder;
pub mod debug_printer;
pub mod env;
pub mod eval;
pub mod expr;
pub mod inductive;
pub mod infer;
pub mod level;
pub mod name;
pub mod parser;
pub mod pretty_printer;
pub mod quot;
pub mod quote;
pub mod relevance;
pub mod tc;
#[cfg(test)]
mod tests;
pub mod util;
pub mod value;

pub(crate) const STACK_SIZE: usize = 2 * 1024 * 1024 * 1024;


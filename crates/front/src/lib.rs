// 本文件提到的 `docs/**` 指针都指向**已收口删除**的设计文档（原文在 git 历史里）：要读原文用 `git log --all -- <path>`。doc-refs: history-only
//! The restricted `.sokonanoda` front-end.
//!
//! This crate deliberately implements only the grammar points exposed by the
//! teaching curriculum. The syntax whitelist is the curriculum: adding a
//! grammar point here means adding a lesson for it.
//!
//! The whitelist covers both **notation** (`∧ ∨ ↔ ¬ →`, built in; `∃`/set
//! symbols are declared per course library) and the **tactic set** the `by`
//! engine accepts. The authoritative lists live with the designs, not here:
//! notation in `docs/design/notation-subset.md`, tactics in
//! `docs/design/by-tactics.md` (which tactic a lesson may use is a curriculum
//! decision — see `docs/design/course-lean-style.md` §9).

// CompileError 携带消息 + span + 期望/实际两端文本，略超 clippy 默认的
// 128 字节 Result 阈值。教学编译器的错误路径不是热点，装箱反而增加分配，
// crate 级显式豁免并在此注明取舍。
#![allow(clippy::result_large_err)]

pub mod by;
pub mod commands;
pub mod compile;
pub mod depgraph;
pub mod display;
pub mod judge;
pub mod lean4_universe;
pub mod notation;
pub mod notation_input;
pub mod project;
pub mod proof;
pub mod query;
pub mod references;
pub mod semantic;
pub mod session;
pub mod spine;
pub mod suggest;
pub mod tactics;

mod ast;
mod diagnostic;
// 引擎身份摘要（CACHE-KEY-NO-ENGINE-IDENTITY）：编译期由 `build.rs` 注入
// `SOKO_ENGINE_DIGEST`，生产路径的消费点是 `compile::cache::build_stamp`
// ⇒ 折进**全局缓存键**（算法与"覆盖哪些目录"见模块自己的头注释 ✓）。
mod engine_digest;
mod parser;
mod reference_docs;
mod span;
mod token;

pub use ast::{
    Binder, BinderKind, Command, CtorDecl, Expr, FolFile, IotaRule, NotationAssoc, NotationDecl,
    OpenFilter, RecDecl, SortKind, Tactic,
};
pub use diagnostic::{Diagnostic, DiagnosticKind, Result};
pub use parser::{
    parse, parse_fragment, parse_recovering, parse_recovering_with_inherited, parse_with_inherited,
    Parser, RecoveredFile,
};
pub use span::{Pos, Span};
pub use tactics::{TacticDoc, TACTIC_DOCS};
pub use token::{tokenize, Lexer, Token, TokenKind};

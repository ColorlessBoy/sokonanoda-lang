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

mod apply_probe;
pub mod by;
pub mod by_defer;
pub mod commands;
pub mod compile;
pub mod depgraph;
pub mod display;
// **判据的输入面**（生成式夹具）：把「用户会输入什么」变成判据的常规输入 ——
// 过程态 / 未关目标 / 差分变体 / 边界变体（`AGENT-DOES-NOT-PRODUCE-USER-INPUTS`）。
// 它只产出源文本，**不进判定路径** ✓；消费方见 `crates/front/tests/` 与
// `crates/cli/tests/process_state_entry.rs`。
pub mod fixtures;
pub mod judge;
pub mod lean4_universe;
mod level_text;
mod meta_parse;
mod meta_registry;
pub mod notation;
pub mod notation_input;
pub mod project;
pub mod proof;
// `proof.rs` 的无渲染依赖小工具（腾判据 ⑨ 的预算，2026-10-16）。
mod proof_support;
pub mod query;
pub mod references;
pub mod semantic;
// 语义着色的词法前缀 + 名字表（从 `semantic.rs` 拆出来腾判据 ⑨ 的预算，2026-10-16）。
mod semantic_names;
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
// `Expr` 的遍历/查询方法（从 `ast.rs` 拆出来腾判据 ⑨ 的行数预算，
// 2026-10-16；先例 `prefix_recovery.rs`）—— `ast.rs` 只许减不许增 ✗。
mod expr;
mod parser;
// 判卷**合成前缀**的解析（严格优先、恢复式兜底）—— `judge.rs` 在 `docs-lint`
// 判据 ⑨ 的存量基线上（**只许减不许增**）⇒ 实现落在新文件里，`judge.rs` 那边
// 只把 `parse_prefix` 的函数体换成一行转发（净增 0 行 ✓）。
mod prefix_recovery;
mod reference_docs;
mod span;
// `parser.rs` 的**边界用例**（`SUB-parse-error-kills-whole-file`）：那个文件在
// `docs-lint` 判据 ⑨ 的存量基线上（**只许减不许增**）⇒ 用例落在 `src/tests/`
// （判据 ⑨ 明确排除测试聚合文件 ✓）。
#[cfg(test)]
#[path = "tests/parser_boundary.rs"]
mod parser_boundary_tests;
mod token;
mod token_scan;

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

//! 文档级报告：每个声明的练习状态与 hover 类型。

use super::error::CompileError;
use crate::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeclKind {
    Definition,
    Theorem,
    Axiom,
    Inductive,
    Example,
}

impl DeclKind {
    pub fn as_str(self) -> &'static str {
        match self {
            DeclKind::Definition => "def",
            DeclKind::Theorem => "theorem",
            DeclKind::Axiom => "axiom",
            DeclKind::Inductive => "inductive",
            DeclKind::Example => "example",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeclStatus {
    /// The declaration is an open exercise (`???` in the value position).
    Open,
    /// The declaration passed the complete kernel.
    Checked,
    /// Elaboration or the kernel rejected it.
    Failed,
}

/// One declaration of a `.sokonanoda` document, with its exercise status.
#[derive(Debug, Clone)]
pub struct DeclState {
    pub kind: DeclKind,
    pub name: Option<String>,
    pub span: Span,
    pub status: DeclStatus,
    /// Present when `status == Failed`.
    pub error: Option<CompileError>,
    /// For an open exercise: the declared goal type, rendered as source text.
    pub goal: Option<String>,
}

/// A hover answer for one source span (`span -> inferred type text`).
#[derive(Debug, Clone)]
pub struct HoverType {
    pub span: Span,
    pub text: String,
}

#[derive(Debug, Clone, Default)]
pub struct DocumentReport {
    pub decls: Vec<DeclState>,
    pub hovers: Vec<HoverType>,
    /// Parse-level diagnostics live on the `parse` result; this holds
    /// elab/kernel failures that are not attached to a declaration.
    pub errors: Vec<CompileError>,
}

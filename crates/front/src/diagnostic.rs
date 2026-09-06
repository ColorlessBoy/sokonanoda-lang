//! 解析期诊断：`DiagnosticKind`/`Diagnostic` 及其机器码与教学提示。

use crate::span::Span;

#[derive(Debug, Clone, PartialEq)]
pub enum DiagnosticKind {
    UnexpectedEof,
    UnexpectedToken { found: String, expected: String },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub kind: DiagnosticKind,
    pub span: Span,
    pub message: String,
}

impl Diagnostic {
    pub(crate) fn new(kind: DiagnosticKind, span: Span, message: String) -> Self {
        Self {
            kind,
            span,
            message,
        }
    }

    /// Stable machine code for the diagnostic (used by `--json`).
    pub fn code(&self) -> &'static str {
        match self.kind {
            DiagnosticKind::UnexpectedEof => "unexpected-eof",
            DiagnosticKind::UnexpectedToken { .. } => "unexpected-token",
        }
    }

    /// Pipeline stage of this diagnostic.
    pub fn stage_code(&self) -> &'static str {
        "parse"
    }

    /// First teaching hint for parse errors.
    pub fn hint(&self) -> &'static str {
        match self.kind {
            DiagnosticKind::UnexpectedEof => {
                "输入到这里就结束了。检查是不是漏写了 `:=` 的值、右括号或 `end`。"
            }
            DiagnosticKind::UnexpectedToken { .. } => {
                "这里的写法不符合当前课程语法。检查命令拼写、括号配对，以及是否多写了还没学过的符号。"
            }
        }
    }
}

pub type Result<T> = std::result::Result<T, Diagnostic>;

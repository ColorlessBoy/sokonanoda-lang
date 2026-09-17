//! 解析期诊断：`DiagnosticKind`/`Diagnostic` 及其机器码与教学提示。

use crate::span::Span;

#[derive(Debug, Clone, PartialEq)]
pub enum DiagnosticKind {
    UnexpectedEof,
    UnexpectedToken {
        found: String,
        expected: String,
    },
    /// `import` 写法不合法：缺名字、一行写了多个、或名字后面跟了别的东西。
    ImportMalformed {
        detail: String,
    },
    /// 模块名不满足标识符规则（`-`、数字开头、空分量…）。`hint` 由
    /// `project::ModuleNameError::hint` 提供，逐原因定制。
    ImportNotAModuleName {
        module: String,
        message: String,
        hint: &'static str,
    },
    /// `import` 出现在声明之后（官方 Lean 同款规则）。
    ImportMustPrecedeDeclarations,
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
            DiagnosticKind::ImportMalformed { .. } => "import-malformed",
            DiagnosticKind::ImportNotAModuleName { .. } => "import-not-a-valid-module-name",
            DiagnosticKind::ImportMustPrecedeDeclarations => "import-must-precede-declarations",
        }
    }

    /// Pipeline stage of this diagnostic.
    pub fn stage_code(&self) -> &'static str {
        "parse"
    }

    /// First teaching hint for parse errors.
    pub fn hint(&self) -> &'static str {
        match &self.kind {
            DiagnosticKind::UnexpectedEof => {
                "输入到这里就结束了。检查是不是漏写了 `:=` 的值、右括号或 `end`。"
            }
            DiagnosticKind::UnexpectedToken { .. } => {
                "这里的写法不符合当前课程语法。检查命令拼写、括号配对，以及是否多写了还没学过的符号。"
            }
            DiagnosticKind::ImportMalformed { .. } => {
                "`import` 一行只写一个点分模块名（例如 `import Lesson.Logic`），并且必须写在文件最上方。"
            }
            DiagnosticKind::ImportNotAModuleName { hint, .. } => hint,
            DiagnosticKind::ImportMustPrecedeDeclarations => {
                "官方 Lean 的规则：`import` 必须写在**任何声明之前**。把这行移到文件开头。"
            }
        }
    }
}

pub type Result<T> = std::result::Result<T, Diagnostic>;

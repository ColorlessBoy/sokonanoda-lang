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
    /// 字符串字面量没有闭合（记法命令的符号）。span 指向**开引号**。
    UnterminatedString,
    /// 记法命令形状不合法：缺优先级、缺 `=>`、符号是标识符词/空串、重复声明
    /// 同一符号。`detail` 是逐原因的正文（`docs/design/notation-subset.md` N1）。
    NotationShape {
        detail: String,
    },
    /// 用了**没有声明过**的记法符号（`a ∈ A` 但本文件里没有 `infix … " ∈ "`）。
    /// hint 给「先声明」与「点名写法」两条出路。
    NotationUnknownSymbol {
        symbol: String,
    },
    /// `end` 的名字与最近的未闭合 `namespace` 不同名（或根本没有可闭合的
    /// `namespace`）。G-05，设计 `docs/design/namespace-open.md` N1/N2。
    NamespaceMismatch {
        /// 写出来的 `end` 名字。
        found: String,
        /// 期望的名字（最近的未闭合 `namespace`）；没有时 `None`。
        expected: Option<String>,
    },
    /// 文件结束仍有未闭合的 `namespace`（span 指回那条 `namespace`）。
    NamespaceUnclosed {
        name: String,
    },
    /// `namespace` / `end` / `open` 的形状不合法（缺名字、名字不是标识符）。
    NamespaceShape {
        detail: String,
    },
    /// **集合字面量**（第三刀 §12.4）形状不合法：空 `{}`、三个及以上元素、
    /// 缺 `}`。`detail` 是逐原因的正文。
    SetLiteralShape {
        detail: String,
    },
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
            DiagnosticKind::UnterminatedString => "unterminated-string",
            DiagnosticKind::NotationShape { .. } => "notation-shape",
            DiagnosticKind::NotationUnknownSymbol { .. } => "notation-unknown-symbol",
            DiagnosticKind::NamespaceMismatch { .. } => "parse-namespace-mismatch",
            DiagnosticKind::NamespaceUnclosed { .. } => "parse-namespace-unclosed",
            DiagnosticKind::NamespaceShape { .. } => "parse-namespace-shape",
            DiagnosticKind::SetLiteralShape { .. } => "set-literal-shape",
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
            DiagnosticKind::UnterminatedString => {
                "字符串没有闭合：记法命令里的符号要写在一对引号之间，例如 infix:50 \" ∈ \" => Set.mem。"
            }
            DiagnosticKind::NotationShape { .. } => {
                "记法命令的形状是：infix:50 \" ∈ \" => Set.mem（infixl/infixr 同形，N 取 1–1000）；零元记法写 notation \"∅\" => Set.empty（不写优先级）。"
            }
            DiagnosticKind::NotationUnknownSymbol { .. } => {
                "这个符号还没有在本文件里声明过。先在它前面写一行记法命令（例如 infix:50 \" ∈ \" => Set.mem），或者改用点名写法（Set.mem α a A）。"
            }
            DiagnosticKind::NamespaceMismatch { .. } => {
                "`end` 的名字要与最近的 `namespace` 一模一样（`namespace Foo` 用 `end Foo` 收）。检查是不是写错了名字，或者中间少了/多了 `end`。"
            }
            DiagnosticKind::NamespaceUnclosed { .. } => {
                "这个 `namespace` 一直没有闭合。在文件末尾（或块结束处）补一行 `end <名字>`，名字与 `namespace` 那行相同。"
            }
            DiagnosticKind::NamespaceShape { .. } => {
                "作用域命令的形状是：namespace Foo（开块）、end Foo（收块，名字必须写出）、open Foo（短名可用，可加点分名字 A.B）。`open` 还能挑名字：open Foo (a b)（只要这两个）、open Foo hiding a b（挡掉这两个）、open Foo renaming a => b（改名），以及只影响一条命令的 open Foo in <命令>；export Foo 同形，但导入本文件的文件也看得到。"
            }
            DiagnosticKind::SetLiteralShape { .. } => {
                "集合字面量的形状是 {a}（单元素）或 {a, b}（两元素）：元素之间用 `,` 隔开，最后用 `}` 收尾；空集写 Set.empty α，三个及以上用 Set.pair 点名嵌套。"
            }
        }
    }
}

pub type Result<T> = std::result::Result<T, Diagnostic>;

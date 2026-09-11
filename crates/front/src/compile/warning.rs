//! 语法级警告：按声明名判定的教学提示（不参与内核判定）。

use crate::ast::{Command, FolFile};
use crate::references::decl_name_span;
use crate::Span;

/// 解析器硬编码为排序的名字（`crates/front/src/parser.rs`）：表达式位置的
/// 这些标识符一律解析为内置排序，永不查环境，因此同名顶层声明不可能被
/// 任何引用命中。
pub const RESERVED_SORT_NAMES: [&str; 3] = ["Prop", "Sort", "Type"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WarningKind {
    ReservedDeclarationName,
}

impl WarningKind {
    pub fn code(self) -> &'static str {
        match self {
            WarningKind::ReservedDeclarationName => "reserved-declaration-name",
        }
    }

    /// 第一版教学提示：学习者读到 warning 时的下一步（与错误 hint 同风格）。
    pub fn hint(self) -> &'static str {
        match self {
            WarningKind::ReservedDeclarationName => {
                "换一个名字即可；如果这里要表达的是内建的 Prop / Sort / Type，它们本来就存在，不需要再声明。"
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompileWarning {
    pub kind: WarningKind,
    pub message: String,
    pub span: Span,
}

impl CompileWarning {
    /// Stable machine code (used by `--json` and the protocol).
    pub fn code(&self) -> &'static str {
        self.kind.code()
    }

    /// First teaching hint for this warning.
    pub fn hint(&self) -> &'static str {
        self.kind.hint()
    }
}

/// 顶层声明名命中保留排序名时产出 warning（纯语法，与内核结果无关）。
/// span 收窄到名字 token；拿不到时退回整条声明的 span。
pub fn collect_warnings(file: &FolFile) -> Vec<CompileWarning> {
    let mut warnings = Vec::new();
    for command in &file.commands {
        let (name, span) = match command {
            Command::Def { name, span, .. }
            | Command::Theorem { name, span, .. }
            | Command::Axiom { name, span, .. }
            | Command::InductiveBlock { name, span, .. } => (name, span),
            Command::Example { .. }
            | Command::Check { .. }
            | Command::Reduce { .. }
            | Command::Print { .. } => continue,
        };
        if !RESERVED_SORT_NAMES.contains(&name.as_str()) {
            continue;
        }
        warnings.push(CompileWarning {
            kind: WarningKind::ReservedDeclarationName,
            message: format!(
                "声明名 `{name}` 与内置排序同名：所有 `{name}` 的引用都指向内置排序，这个声明不会被用到。"
            ),
            span: decl_name_span(&file.src, *span, name).unwrap_or(*span),
        });
    }
    warnings
}

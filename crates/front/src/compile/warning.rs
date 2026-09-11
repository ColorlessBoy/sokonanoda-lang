//! 语法级警告：按声明名判定的教学提示（不参与内核判定）。

use crate::ast::{Command, FolFile};
use crate::references::decl_name_span;
use crate::Span;

/// 内核已经定义、不能再声明的名字（`Prop` / `Sort` / `Type`）：表达式位置
/// 的这些标识符一律指向内核定义的那个，不会去查环境里有没有同名声明，
/// 因此同名顶层声明不可能被任何引用命中。
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
                "删掉这一行即可；要写命题或类型，直接用内核已经有的 Prop / Sort / Type。"
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

/// 顶层声明名撞上内核已定义的名字时产出 warning（纯语法，与内核结果无关）。
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
        let message = if name == "Prop" {
            "`Prop` 内核已经定义过了，不能再声明一次。Prop 在形式化证明里地位特殊：所有命题都住在 Prop 里，代码里每个 `Prop` 指的都是内核定义的那个，这一行声明出来的名字不会被用到。".to_string()
        } else {
            format!(
                "`{name}` 内核已经定义过了，不能再声明一次；代码里每个 `{name}` 指的都是内核定义的那个，这一行声明出来的名字不会被用到。"
            )
        };
        warnings.push(CompileWarning {
            kind: WarningKind::ReservedDeclarationName,
            message,
            span: decl_name_span(&file.src, *span, name).unwrap_or(*span),
        });
    }
    warnings
}

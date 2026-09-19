//! 语法级警告：按声明名判定的教学提示（不参与内核判定）。
use serde::{Deserialize, Serialize};

use crate::ast::{Command, FolFile};
use crate::references::decl_name_span;
use crate::Span;

/// 内核已经定义、不能再声明的名字（`Prop` / `Sort` / `Type`）：表达式位置
/// 的这些标识符一律指向内核定义的那个，不会去查环境里有没有同名声明，
/// 因此同名顶层声明不可能被任何引用命中。
pub const RESERVED_SORT_NAMES: [&str; 3] = ["Prop", "Sort", "Type"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WarningKind {
    ReservedDeclarationName,
    /// `import` 的模块里还有未完成的练习：它们对下游不可见（洞不污染环境）。
    ImportHasOpenExercises,
    /// 值位里"多出来"的 `sorry`：前面的项已经完成了证明，它接在一个不是
    /// 函数的项后面，填什么都不可能是良类型的应用。与"真缺口"（洞有期望
    /// 类型）是两件事，见 `docs/design/redundant-sorry.md`。
    RedundantSorry,
}

impl WarningKind {
    pub fn code(self) -> &'static str {
        match self {
            WarningKind::ReservedDeclarationName => "reserved-declaration-name",
            WarningKind::ImportHasOpenExercises => "import-has-open-exercises",
            WarningKind::RedundantSorry => "redundant-sorry",
        }
    }

    /// 第一版教学提示：学习者读到 warning 时的下一步（与错误 hint 同风格）。
    pub fn hint(self) -> &'static str {
        match self {
            WarningKind::ReservedDeclarationName => {
                "删掉这一行即可；要写命题或类型，直接用内核已经有的 Prop / Sort / Type。"
            }
            WarningKind::ImportHasOpenExercises => {
                "被导入文件里还有 `sorry`：这些声明对下游不可见（未完成的洞不进入环境），下游看不到它们的名字。"
            }
            WarningKind::RedundantSorry => {
                "删掉这一行 sorry，这条声明就会通过内核检查；若还想继续写，请把它换成真正缺少的那部分。"
            }
        }
    }

    /// 这条 warning 是**内核终审**过的，还是每次 update 由 [`collect_warnings`]
    /// 重算的语法级提示？
    ///
    /// 内核终审过的（[`WarningKind::RedundantSorry`]）必须按命令进会话快照
    /// 跨版本复用——增量编辑时信任前缀不会重跑探针，只有快照记得它；
    /// 语法级的每次重算，不能进快照（否则会重复报）。
    pub fn is_kernel_verified(self) -> bool {
        match self {
            WarningKind::ReservedDeclarationName => false,
            // 项目层警告是语法级重算的（不来自内核终审）。
            WarningKind::ImportHasOpenExercises => false,
            WarningKind::RedundantSorry => true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
            | Command::Print { .. }
            | Command::Import { .. }
            // 记法命令不是声明（设计 N6）：没有声明名可查，也就不会有
            // 「占了内核保留名」这类 warning。G-05 的 namespace/end/open 同理
            // （它们是作用域命令，声明名加前缀已经在 parser 里落定）。
            | Command::Notation { .. }
            | Command::Namespace { .. }
            | Command::End { .. }
            | Command::Open { .. } => continue,
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

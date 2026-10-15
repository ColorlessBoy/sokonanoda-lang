//! 检查事件流与批量编译输出（events + errors）。

use super::error::CompileError;
use super::warning::CompileWarning;
use crate::Span;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CheckEvent {
    DeclarationChecked {
        name: String,
    },
    ExampleChecked,
    TypeChecked {
        text: String,
        span: Span,
        /// **`#check <裸 def 名>` 的声明卡片**（2026-10-15 用户新需求，见
        /// `REQUIREMENTS.md` §9 那条）：`def <名字> : <类型> := <定义体>`，
        /// 类型与定义体都过 `display.fold`（记法）+ 类型那一半过
        /// [`crate::display::lean4_universe_text`]（`Type 0` → `Type`，与
        /// Infoview 声明卡片**同一条**规则）。
        ///
        /// ⚠ **`text` 一个字节都没动** ✗ —— 判定路径（`judge::judge_type_of` ⇒
        /// `pick_type_checked` ⇒ 记法目标签名）读的是 `text`，它必须继续是
        /// **类型**（`sig` 快路的源级签名，见 `walk::check`）✓。
        /// 本字段只服务**显示**出口（`--json` 的 `human` / 终端输出 /
        /// 报告的 `checks[]`）✓。
        ///
        /// `None` = 不是"裸 def 名"（`theorem`/`axiom`/`inductive`/构造子/递归子、
        /// 以及一切非裸标识符的 `#check`）⇒ 显示退回既有的 `<源文本>: <类型>` ✓。
        #[serde(default)]
        decl: Option<String>,
    },
    Reduced {
        text: String,
        span: Span,
    },
    Printed {
        name: String,
        text: String,
    },
    ExerciseOpen {
        name: Option<String>,
    },
}

impl CheckEvent {
    /// `expr.typed` 的**显示文本**（2026-10-15 用户新需求）：`#check <裸 def 名>`
    /// ⇒ **声明卡片**（`def … := …`）；其余 ⇒ 内核类型文本。
    ///
    /// **唯一映射**（✗ 别在 `session::assemble_report` 与
    /// `kernel_phase` 的报告组装里各写一份 —— 那正是"同一份事件两个面漂移"的
    /// 温床）：`None` = 这条事件不是 `expr.typed`。
    ///
    /// ⚠ 判定路径**不读它** ✗：`judge::pick_type_checked` 读的是
    /// [`CheckEvent::TypeChecked::text`]（类型）✓。
    pub fn type_display(&self) -> Option<&str> {
        match self {
            CheckEvent::TypeChecked { text, decl, .. } => Some(decl.as_deref().unwrap_or(text)),
            _ => None,
        }
    }
}

/// 一次编译的性能计数（I8 增量的可验证性：改第 i 个声明只内核重查后缀）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompileStats {
    /// `try_check_declar` 的实际调用次数（受信任前缀不计入）。
    pub kernel_checks: usize,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CompileOutput {
    pub events: Vec<CheckEvent>,
    /// 与 `events` 平行：每条事件归属于哪条命令（索引）。
    pub event_cmds: Vec<usize>,
    pub errors: Vec<CompileError>,
    /// 与 `errors` 平行：每条错误归属于哪条命令（索引）。跨文件编译时
    /// **必须**用它归因——不同文件的 offset 不在同一个坐标空间里。
    pub error_cmds: Vec<usize>,
    /// 警告（语法级 + pass 2 内核终审过的 `redundant-sorry`）。不影响
    /// `ok()` / 退出码。
    pub warnings: Vec<CompileWarning>,
    /// 与 `warnings` 平行：每条警告归属于哪条命令（索引）。跨文件编译时
    /// **必须**用它归因——不同文件的 offset 不在同一个坐标空间里。
    pub warning_cmds: Vec<usize>,
    pub stats: CompileStats,
}

impl CompileOutput {
    pub fn ok(&self) -> bool {
        self.errors.is_empty()
    }

    /// 记录一条事件及其归属命令（两数组严格平行，永不失配）。
    pub(crate) fn push_event(&mut self, cmd: usize, event: CheckEvent) {
        self.events.push(event);
        self.event_cmds.push(cmd);
    }

    /// 记录一条错误及其归属命令（两数组严格平行，永不失配）。
    pub(crate) fn push_error(&mut self, cmd: usize, error: CompileError) {
        self.errors.push(error);
        self.error_cmds.push(cmd);
    }

    /// 记录一条警告及其归属命令（两数组严格平行，永不失配）。
    pub(crate) fn push_warning(&mut self, cmd: usize, warning: CompileWarning) {
        self.warnings.push(warning);
        self.warning_cmds.push(cmd);
    }
}

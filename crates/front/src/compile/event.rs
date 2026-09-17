//! 检查事件流与批量编译输出（events + errors）。

use super::error::CompileError;
use super::warning::CompileWarning;
use crate::Span;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CheckEvent {
    DeclarationChecked { name: String },
    ExampleChecked,
    TypeChecked { text: String, span: Span },
    Reduced { text: String, span: Span },
    Printed { name: String, text: String },
    ExerciseOpen { name: Option<String> },
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
    /// 语法级警告（如声明名撞内核已定义的名字）。不影响 `ok()` / 退出码。
    pub warnings: Vec<CompileWarning>,
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
}

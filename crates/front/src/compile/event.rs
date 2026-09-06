//! 检查事件流与批量编译输出（events + errors）。

use super::error::CompileError;
use crate::Span;

#[derive(Debug, Clone, PartialEq)]
pub enum CheckEvent {
    DeclarationChecked { name: String },
    ExampleChecked,
    TypeChecked { text: String, span: Span },
    Reduced { text: String, span: Span },
    Printed { name: String, text: String },
    ExerciseOpen { name: Option<String> },
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct CompileOutput {
    pub events: Vec<CheckEvent>,
    pub errors: Vec<CompileError>,
}

impl CompileOutput {
    pub fn ok(&self) -> bool {
        self.errors.is_empty()
    }
}

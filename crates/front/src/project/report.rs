//! 项目报告：闭包编译的结果（逐模块报告 + 归因到文件/行的项目级诊断）。

use std::path::PathBuf;

use super::super::compile::{
    CompileError, CompileOutput, CompileWarning, DeclStatus, DocumentReport, ErrorKind, WarningKind,
};
use crate::Span;

/// 项目级诊断的种类：错误映射到 `ErrorKind`、警告映射到 `WarningKind`，
/// 于是它们既能进 `docs/protocol.md` 的码表，也能直接挂进逐模块报告。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectKind {
    Error(ErrorKind),
    Warning(WarningKind),
}

impl ProjectKind {
    pub fn code(self) -> &'static str {
        match self {
            ProjectKind::Error(kind) => kind.code(),
            ProjectKind::Warning(kind) => kind.code(),
        }
    }

    pub fn hint(self) -> &'static str {
        match self {
            ProjectKind::Error(kind) => kind.hint(),
            ProjectKind::Warning(kind) => kind.hint(),
        }
    }

    pub fn is_error(self) -> bool {
        matches!(self, ProjectKind::Error(_))
    }
}

/// 一条项目级诊断：**归属到某个模块**（`import` 行的 span 属于导入方）。
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectDiagnostic {
    pub kind: ProjectKind,
    /// 面向用户的正文（可覆盖 `kind` 的默认 hint）。
    pub message: String,
    /// 归属模块名（导入方）。
    pub module: String,
    /// 归属模块的路径（stdin / 未落盘文本为 `None`）。
    pub path: Option<PathBuf>,
    /// 位置：`import` 行的 span（清单错误为文件头 0）。
    pub span: Option<Span>,
}

impl ProjectDiagnostic {
    pub fn code(&self) -> &'static str {
        self.kind.code()
    }

    /// 教学提示：项目层诊断优先用 `kind` 的通用 hint（`ErrorKind::hint`）。
    pub fn hint(&self) -> &'static str {
        self.kind.hint()
    }
}

/// 一个模块的编译结果。
#[derive(Debug, Clone)]
pub struct ModuleReport {
    /// 模块名（`import` 名，点分）。
    pub name: String,
    pub path: PathBuf,
    /// 该模块 import 的模块名（书写顺序，去重）。
    pub imports: Vec<String>,
    pub report: DocumentReport,
    pub events: CompileOutput,
}

impl ModuleReport {
    /// 未完成的练习数（`sorry` 洞）。
    pub fn open_exercises(&self) -> usize {
        self.report
            .decls
            .iter()
            .filter(|decl| decl.status == DeclStatus::Open)
            .count()
    }
}

/// 一次项目编译的全部结果。
#[derive(Debug, Clone)]
pub struct ProjectReport {
    /// 入口文件路径（`--text` 时为约定的占位路径）。
    pub entry: PathBuf,
    /// 模块根（清单目录或入口文件目录）。
    pub root: PathBuf,
    /// 生效的清单（没有清单时为 `None`，即零配置模式）。
    pub manifest: Option<PathBuf>,
    /// 编译过的模块，**拓扑序、入口在最后**（被阻断的模块不在列表里）。
    pub modules: Vec<ModuleReport>,
    /// 项目级诊断（错误 + 警告），已按 `module` 归属。
    pub diagnostics: Vec<ProjectDiagnostic>,
    /// 清单 `requires` 与当前二进制的版本不一致时的提示（v1 只警告，不阻断）。
    pub requires_warning: Option<String>,
}

impl ProjectReport {
    /// 入口模块的报告（单文档消费者用它）。
    pub fn entry_module(&self) -> Option<&ModuleReport> {
        self.modules.last()
    }

    pub fn entry_report(&self) -> Option<&DocumentReport> {
        self.entry_module().map(|module| &module.report)
    }

    pub fn module(&self, name: &str) -> Option<&ModuleReport> {
        self.modules.iter().find(|module| module.name == name)
    }

    /// 有没有错误级诊断（警告不算）。
    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(|diag| diag.kind.is_error())
    }

    /// "完全干净"：没有任何诊断（错误或警告）、也没有清单版本提示。
    ///
    /// 只有干净的项目才进缓存——缓存条目里只有**入口**的报告与事件，带诊断的
    /// 项目回放不出依赖模块的诊断，而冷跑/热跑必须逐字节一致（A1 的多文件版）。
    pub fn is_clean(&self) -> bool {
        !self.has_errors()
            && self.diagnostics.is_empty()
            && self.requires_warning.is_none()
            && self
                .modules
                .iter()
                .all(|module| module.report.errors.is_empty() && module.report.warnings.is_empty())
    }

    /// 把项目级诊断挂进对应模块的报告：错误进 `errors`、警告进 `warnings`，
    /// 于是 CLI / `--json` / LSP / `query` 都能像看普通诊断一样看到它们
    /// （入口文件的 `import` 行因此会有红/黄标记）。
    pub(crate) fn attach_diagnostics(&mut self) {
        let entry_index = self.modules.len().saturating_sub(1);
        for diag in &self.diagnostics {
            let index = self
                .modules
                .iter()
                .position(|module| module.name == diag.module)
                .unwrap_or(entry_index);
            let Some(module) = self.modules.get_mut(index) else {
                continue;
            };
            match diag.kind {
                ProjectKind::Error(kind) => {
                    module.report.errors.push(CompileError::new(
                        kind,
                        diag.message.clone(),
                        diag.span.unwrap_or_default(),
                    ));
                }
                ProjectKind::Warning(kind) => {
                    if let Some(span) = diag.span {
                        module.report.warnings.push(CompileWarning {
                            kind,
                            message: diag.message.clone(),
                            span,
                        });
                    }
                }
            }
        }
        // `events` 是报告的"批处理视图"（CLI/`--json` 打印它）：挂完项目级
        // 诊断后必须同步，否则 import 行的红标记只在报告里、打印不出来。
        for module in &mut self.modules {
            module.events.errors = module.report.errors.clone();
            module.events.warnings = module.report.warnings.clone();
        }
    }
}

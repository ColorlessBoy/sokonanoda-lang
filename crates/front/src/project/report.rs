//! 项目报告：闭包编译的结果（逐模块报告 + 归因到文件/行的项目级诊断）。

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::super::compile::{
    CompileError, CompileOutput, CompileWarning, DeclStatus, DocumentReport, ErrorKind, WarningKind,
};
use crate::Span;

/// 项目级诊断的种类：错误映射到 `ErrorKind`、警告映射到 `WarningKind`，
/// 于是它们既能进 `docs/protocol.md` 的码表，也能直接挂进逐模块报告。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

/// 一个模块在这次项目编译里的**状态**（`soko/project` 视图与扩展项目树用）。
///
/// 三者必须可区分：`blocked` 的模块报告是空的，但"上游没编译成功"与"模块自己
/// 就是空的"是两回事——消费者（编辑器/agent）要靠它决定说什么话。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModuleStatus {
    /// 参与编译（报告里**可能仍有错误**，错误数看 `report.errors`）。
    Compiled,
    /// 闭包加载期就失败：文件找不到 / 解析错误 / `import` 环。
    LoadFailed,
    /// 没参与编译（上游加载失败），或编译过但结果被上游的编译失败丢弃。
    Blocked,
}

impl ModuleStatus {
    /// 协议里的稳定机器码（`docs/protocol.md`）。
    pub fn code(self) -> &'static str {
        match self {
            ModuleStatus::Compiled => "compiled",
            ModuleStatus::LoadFailed => "load-failed",
            ModuleStatus::Blocked => "blocked",
        }
    }
}

/// 一个模块的编译结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleReport {
    /// 模块名（`import` 名，点分）。
    pub name: String,
    pub path: PathBuf,
    /// 该模块 import 的模块名（书写顺序，去重）。
    pub imports: Vec<String>,
    /// **编译时用的源文本**（入口可能是未落盘的中间态；依赖是读盘那一刻的内容）。
    ///
    /// 跨文件引用/改名（LSP）必须按"编译器看到的那份文本"计算名字 token 的
    /// span——重新读盘可能与报告不一致。
    pub source: String,
    pub report: DocumentReport,
    pub events: CompileOutput,
    /// 见 [`ModuleStatus`]。`blocked` 的模块报告为空，状态由 `compile_plan` 填。
    pub status: ModuleStatus,
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// **入口可见的记法表**（T-D11）：符号 → 声明点 + 模块名。单文件编译时
    /// 就是本文件的记法（`Closure` 的同一份数据，单文件路径下为空——那条路
    /// 由 `notation_input::symbol_at` 自己扫本文件）。
    pub notations: Vec<crate::ast::NotationDecl>,
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
        // **`requires_warning` 不算"不干净"**（T-A05 / G-24）。
        //
        // 它是"清单声明的 `requires` 与当前二进制版本不一致"的提示——一个
        // **可回放的确定性事实**：同一条目回放时把警告一起放回来，用户看到的
        // 东西逐字相同（`requires_warning` 是 `ProjectReport` 的字段，
        // T-A03 起随条目一起缓存）。把它算成"不干净"的后果是**整个项目永不
        // 写缓存**：`courses/set-theory` 的 `requires = "0.61"` 让 35 个文件的
        // `build` 热跑只命中 1 个（实测冷 2m29.8s / 热 2m30.2s，见
        // `docs/PERF.md`），而那个警告本身只是一行提示。
        //
        // 保留在判据里的仍然是"**诊断**"——错误与真正的 warning 会改变用户
        // 看到的东西、且归因依赖具体文件，那些不该被回放成缓存。
        !self.has_errors()
            && self.diagnostics.is_empty()
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

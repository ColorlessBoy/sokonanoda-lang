//! 内核真相查询层：**编辑器无关**的类型化查询。
//!
//! 这是 `soko/*` 自定义请求、`sokonanoda query` 子命令与 MCP 工具**共用的唯一
//! 真相**（`docs/design/agent-query-channel.md` H6-A）。任何一个适配器都**不得**
//! 自己重算"光标处是哪个目标""下一个洞在哪"——那会产生第二份真相，违反
//! `REQUIREMENTS.md` §2 第 4 条（判定永远走内核，禁止近似实现）。
//!
//! 设计要点：
//! - 位置在真相层用**字节 offset**（唯一、无编码歧义）；行/列只在适配器层出现
//!   （LSP 用 0-based UTF-16，CLI/MCP 用 1-based，见 [`line_col_of`]）。
//! - "正常的没有"（证明已闭合、没有下一个洞）返回 `None`/空数组；"问不出来"
//!   返回 [`QueryError`]。两者绝不混用。
//! - 所有文本来自完整内核（pretty print）或 `crate::semantic` 的唯一分类。

mod types;

pub use types::{
    Answer, BinderInfo, CheckCounts, CheckSummary, CodeActionInfo, DeclHeader, DeclInfo,
    FailedDecl, GoalInfo, HoleInfo, LocatedHole, QueryError, ReduceAnswer, RunInfo, StateAnswer,
    SubGoalInfo, WarningInfo,
};

use crate::compile::{
    compile_all_with, probe_sub_goal_types, ByGoalState, CompileOptions, DeclState, DeclStatus,
    DocumentReport, PreludeMode,
};
use crate::semantic::{self, SemanticKind};
use crate::session::Session;
use crate::span::Span;
use crate::Diagnostic;

/// 一份被查询的文档：文本 + 会话式编译状态 + 最近的报告。
///
/// 这是 LSP 的 `Doc` 与 CLI/MCP 共用的载体——查询逻辑全部挂在这里，适配器只做
/// 坐标转换与序列化（LSP 侧 `Doc` 现在是它的薄包装）。
pub struct QueryDoc {
    pub text: String,
    /// 会话式编译（I8）：持有上一版本快照，编辑只重查受影响后缀。
    pub session: Session,
    /// prelude 模式（`Full` / `Bare`，可由文件注释指令覆盖）。
    pub mode: PreludeMode,
    /// 最近一次编译的报告；`None` = 尚未编译过。
    pub report: Option<DocumentReport>,
    /// 最近一次 parse 诊断（报告为空时用它解释"为什么问不出来"）。
    pub parse_error: Option<Diagnostic>,
    /// 文档版本（随每次 `set_text` 递增），供消费者丢弃过期答案。
    pub version: u64,
}

impl Default for QueryDoc {
    fn default() -> Self {
        Self::new()
    }
}

impl QueryDoc {
    pub fn new() -> Self {
        Self {
            text: String::new(),
            session: Session::new(CompileOptions::default()),
            mode: PreludeMode::Full,
            report: None,
            parse_error: None,
            version: 0,
        }
    }

    /// 换文本并重编译（版本递增）。`mode` 为 `None` 时沿用当前模式。
    pub fn set_text(&mut self, text: &str, version: u64, mode: Option<PreludeMode>) {
        if let Some(mode) = mode {
            if mode != self.mode {
                self.mode = mode;
                self.session = Session::new(CompileOptions { prelude: mode });
            }
        }
        self.version = version;
        self.text = text.to_string();
        let update = self.session.update(text, version);
        self.parse_error = update.parse_error;
        self.report = Some(update.report);
    }

    /// 当前 prelude 模式对应的编译选项。
    fn options(&self) -> CompileOptions {
        CompileOptions { prelude: self.mode }
    }

    /// 文档的声明类型表（每次查询算一次，供 runs 分类复用）。
    fn decl_kinds(&self) -> Vec<(String, SemanticKind)> {
        semantic::declaration_kinds(&self.text)
    }

    /// 把一段内核文本切成 wire runs（着色单一来源）。
    fn runs(
        &self,
        decls: &[(String, SemanticKind)],
        text: &str,
        binders: &[String],
    ) -> Vec<RunInfo> {
        semantic::tag_runs(text, decls, binders)
            .into_iter()
            .map(|run| RunInfo {
                text: run.text,
                kind: run.kind.map(|k| k.as_str().to_string()),
            })
            .collect()
    }

    // ── check ───────────────────────────────────────────────────────────────

    /// 整文件判卷摘要。**与 `--json` 事件流同源**（同一个 `front::session` +
    /// 同一个 `compile_all_with`），契约测试断言两者计数一致（设计文档 A4）。
    pub fn check(&self) -> CheckSummary {
        let (output, _) = compile_all_with(&parse_or_empty(&self.text), &self.options());
        let mut counts = CheckCounts::default();
        for event in &output.events {
            use crate::compile::CheckEvent::*;
            match event {
                DeclarationChecked { .. } => counts.decl_checked += 1,
                ExampleChecked => counts.example_checked += 1,
                ExerciseOpen { .. } => counts.exercise_open += 1,
                TypeChecked { .. } => counts.expr_typed += 1,
                Reduced { .. } => counts.expr_reduced += 1,
                Printed { .. } => counts.decl_printed += 1,
            }
        }
        let failed = output
            .errors
            .iter()
            .map(|e| FailedDecl {
                name: None,
                code: e.kind.code().to_string(),
                message: e.message.clone(),
                start: e.span.start.offset,
                end: e.span.end.offset,
            })
            .collect();
        let warnings = output
            .warnings
            .iter()
            .map(|w| WarningInfo {
                code: w.code().to_string(),
                message: w.message.clone(),
                hint: Some(w.hint().to_string()),
                start: w.span.start.offset,
                end: w.span.end.offset,
            })
            .collect();
        CheckSummary {
            version: self.version,
            counts,
            failed,
            warnings,
        }
    }

    // ── state（Lean `goalsAt?` 语义）─────────────────────────────────────────

    /// 光标处的目标状态。语义与 `soko/stateAt` **完全一致**（同一实现）：
    /// 光标在某 tactic 的 span 内 → **进入**该 tactic 之前的状态；否则停在最后
    /// 一条在光标前结束的 tactic 之后；首个 tactic 之前 → 根状态。
    pub fn state_at(&self, cursor: usize) -> Result<StateAnswer, QueryError> {
        if cursor > self.text.len() {
            return Err(QueryError::PositionOutOfRange);
        }
        let report = self.report.as_ref().ok_or(QueryError::NotParsable)?;
        let Some(d) = report
            .decls
            .iter()
            .find(|d| d.span.start.offset <= cursor && cursor <= d.span.end.offset)
        else {
            return Err(QueryError::OutsideDeclarations);
        };
        let selection = select_state_at(d, cursor);
        // 一次 parse，然后每个 goal/binder 纯分类（goal-rendering §2.1）。
        let decls = self.decl_kinds();
        let goal = |g: &ByGoalState| {
            let names: Vec<String> = g.binders.iter().map(|b| b.name.clone()).collect();
            GoalInfo {
                goal: g.ty.clone(),
                goal_runs: self.runs(&decls, &g.ty, &names),
                binders: g
                    .binders
                    .iter()
                    .map(|b| BinderInfo {
                        name: b.name.clone(),
                        ty: b.ty.clone(),
                        ty_runs: self.runs(&decls, &b.ty, &names),
                    })
                    .collect(),
            }
        };
        let goals: Vec<GoalInfo> = selection.goals.iter().map(goal).collect();
        let first = goals.first().cloned();
        Ok(StateAnswer {
            version: self.version,
            decl: Some(DeclHeader {
                name: decl_name(d),
                kind: d.kind.as_str().to_string(),
                status: status_str(d.status).to_string(),
                start: d.span.start.offset,
                end: d.span.end.offset,
            }),
            goal: first.as_ref().map(|g| g.goal.clone()),
            goal_runs: first
                .as_ref()
                .map(|g| g.goal_runs.clone())
                .unwrap_or_default(),
            binders: first
                .as_ref()
                .map(|g| g.binders.clone())
                .unwrap_or_default(),
            goals,
            span: selection.span.map(|s| (s.start.offset, s.end.offset)),
            step: selection.step,
            total: selection.total,
        })
    }

    // ── goals（声明级）────────────────────────────────────────────────────

    /// 每个声明的类型/状态/开放目标/洞。`probe: true` 时用请求期内核探针补
    /// 子洞期望类型（`docs/design/spine-meta-a.md`）。
    pub fn goals(&self, probe: bool) -> Vec<DeclInfo> {
        let report = if probe {
            self.probed_report()
        } else {
            self.report.clone().unwrap_or_default()
        };
        let decls = self.decl_kinds();
        report
            .decls
            .iter()
            .map(|d| {
                let name = decl_name(d);
                let binder_names: Vec<String> = d.binders.iter().map(|b| b.name.clone()).collect();
                let ty_runs = d
                    .ty_text
                    .as_deref()
                    .map(|ty| self.runs(&decls, ty, &binder_names))
                    .unwrap_or_default();
                let open = d.status == DeclStatus::Open;
                DeclInfo {
                    name,
                    kind: d.kind.as_str().to_string(),
                    status: status_str(d.status).to_string(),
                    start: d.span.start.offset,
                    end: d.span.end.offset,
                    ty: d.ty_text.clone(),
                    ty_runs,
                    goals: if open {
                        d.by_steps
                            .last()
                            .map(|s| s.goals.iter().map(|g| g.ty.clone()).collect())
                            .unwrap_or_else(|| d.goal.clone().into_iter().collect())
                    } else {
                        Vec::new()
                    },
                    goal: d.goal.clone(),
                    binders: d
                        .binders
                        .iter()
                        .map(|b| BinderInfo {
                            name: b.name.clone(),
                            ty: b.ty.clone(),
                            ty_runs: self.runs(&decls, &b.ty, &binder_names),
                        })
                        .collect(),
                    hole: if open {
                        d.holes.first().map(|s| (s.start.offset, s.end.offset))
                    } else {
                        None
                    },
                    holes: if open {
                        d.holes
                            .iter()
                            .enumerate()
                            .map(|(index, span)| HoleInfo {
                                start: span.start.offset,
                                end: span.end.offset,
                                id: format!("{}:{index}", decl_name(d)),
                            })
                            .collect()
                    } else {
                        Vec::new()
                    },
                    sub_goals: if open {
                        d.sub_goals
                            .iter()
                            .map(|sub| SubGoalInfo {
                                start: sub.span.start.offset,
                                end: sub.span.end.offset,
                                ty: sub.ty.clone(),
                            })
                            .collect()
                    } else {
                        Vec::new()
                    },
                    // 内核判定的下一步建议来自 LSP 侧的 code-action 层（需要
                    // judge 上下文）；CLI/MCP 侧目前留空，见 H6-A 的 as-built。
                    code_actions: Vec::new(),
                }
            })
            .collect()
    }

    /// 请求期内核探针后的报告：只补开放练习里 `sub_goals[i].ty == None` 的项
    /// （`docs/design/spine-meta-a.md` §2/§4）。**绝不进 keystroke 路径**。
    fn probed_report(&self) -> DocumentReport {
        let Some(report) = &self.report else {
            return DocumentReport::default();
        };
        let needs_probe = report
            .decls
            .iter()
            .any(|d| d.status == DeclStatus::Open && d.sub_goals.iter().any(|s| s.ty.is_none()));
        if !needs_probe {
            return report.clone();
        }
        let mut report = report.clone();
        for d in &mut report.decls {
            if d.status != DeclStatus::Open || !d.sub_goals.iter().any(|s| s.ty.is_none()) {
                continue;
            }
            let probed = probe_sub_goal_types(&self.text, &self.options(), d.span);
            for sub in &mut d.sub_goals {
                if sub.ty.is_none() {
                    if let Some(ty) = probed
                        .iter()
                        .find(|(offset, _)| *offset == sub.span.start.offset)
                        .map(|(_, ty)| ty.clone())
                    {
                        sub.ty = Some(ty);
                    }
                }
            }
        }
        report
    }

    // ── holes（可寻址）─────────────────────────────────────────────────────

    /// 全部洞（文件序），带**稳定 id** 与期望类型。
    ///
    /// `id`（`<declName>:<index>`）是程序化消费者的唯一稳定引用——同一源码位置
    /// 可能有多个子目标，`soko/nextHole` 的按位置导航在那种情形下不可用
    /// （`docs/protocol.md` 的 Known limitation）。
    pub fn holes(&self) -> Vec<LocatedHole> {
        let decls = self.goals(true);
        let mut out: Vec<LocatedHole> = decls
            .iter()
            .flat_map(|d| {
                d.holes.iter().map(|h| LocatedHole {
                    id: h.id.clone(),
                    start: h.start,
                    end: h.end,
                    // 子洞按位置对齐期望类型（几处可能同址，故按位置索引找）。
                    ty: d
                        .sub_goals
                        .iter()
                        .find(|s| s.start == h.start && s.end == h.end)
                        .and_then(|s| s.ty.clone()),
                    decl: d.name.clone(),
                })
            })
            .collect();
        out.sort_by_key(|h| (h.start, h.end));
        out
    }

    /// 相对 `from` 的下一个（`forward`）或上一个洞——服务端定位，客户端不扫文本。
    pub fn next_hole(&self, from: usize, forward: bool) -> Option<LocatedHole> {
        let holes = self.holes();
        if forward {
            holes.into_iter().find(|h| h.start > from)
        } else {
            holes.into_iter().rev().find(|h| h.start < from)
        }
    }

    // ── hints ─────────────────────────────────────────────────────────────

    /// 光标所在声明的 `-- soko:hint` 阶梯（无状态：从不数剩余条数）。
    pub fn hints_at(&self, cursor: usize) -> Vec<String> {
        let Some(report) = self.report.as_ref() else {
            return Vec::new();
        };
        report
            .decls
            .iter()
            .find(|d| d.span.start.offset <= cursor && cursor <= d.span.end.offset)
            .map(|d| d.hints.clone())
            .unwrap_or_default()
    }

    // ── reduce ────────────────────────────────────────────────────────────

    /// 对给定表达式求值（与 REPL `#reduce` 同一真相）。
    pub fn reduce(&self, expr: &str) -> Option<ReduceAnswer> {
        let src = format!("{}\n#reduce {expr}\n", self.text);
        let (output, _) = compile_all_with(&parse_or_empty(&src), &self.options());
        output.events.iter().find_map(|e| match e {
            crate::compile::CheckEvent::Reduced { text, .. } => Some(ReduceAnswer {
                value: text.clone(),
                ty: None,
            }),
            _ => None,
        })
    }
}

/// 解析源文本；失败时给一个空文件（`check` 会带着 parse 诊断返回）。
fn parse_or_empty(src: &str) -> crate::ast::FolFile {
    crate::parse(src).unwrap_or(crate::ast::FolFile {
        commands: Vec::new(),
        src: String::new(),
    })
}

/// 声明名（匿名 `example` 用 `example@<line>` 形式，与既有协议一致）。
pub fn decl_name(d: &DeclState) -> String {
    match &d.name {
        Some(n) => n.clone(),
        None => format!("{}@{}", d.kind.as_str(), d.span.start.line),
    }
}

/// 声明状态的稳定文本（协议 wire 用的就是它）。
pub fn status_str(status: DeclStatus) -> &'static str {
    match status {
        DeclStatus::Checked => "checked",
        DeclStatus::Open => "open",
        DeclStatus::Failed => "failed",
    }
}

/// 光标处的状态选择（Lean `goalsAt?` 语义的唯一实现）。
pub struct StateSelection {
    pub goals: Vec<ByGoalState>,
    pub span: Option<Span>,
    pub step: i64,
    pub total: usize,
}

/// 光标处的状态选择（**Lean `goalsAt?` 语义的唯一实现**，协议原文见
/// `docs/protocol.md` §`soko/stateAt`）：
///
/// - 光标落在某条 tactic 的 span 内（**半开区间** `start <= cursor < end`：光标
///   恰在 tactic 末尾算"之后"，不算"之内"）→ 该 tactic **执行前**的状态，
///   即第 `i-1` 条执行后的状态（`i == 0` 时为根状态）；
/// - 否则取"最后一条在光标前（含恰好结束）结束的 tactic"之后的状态；
/// - 根状态（`step: -1`）：**声明类型的内核渲染文本**（`ty_text`，未知时退回走查
///   的剩余目标）+ **空 binders**，`span` = 声明范围。
///
/// 这三条都是协议规定、且 LSP 客户端（VS Code Infoview / 练习树）依赖的行为；
/// 真相层必须与之逐字一致——先前这里的闭区间与"根状态带 binders/剩余目标"是
/// 错的（`docs/design/agent-query-channel.md` 的 H6-A 一致性契约正是为此）。
pub fn select_state_at(d: &DeclState, cursor: usize) -> StateSelection {
    let root = || StateSelection {
        goals: d
            .ty_text
            .clone()
            .or_else(|| d.goal.clone())
            .map(|ty| ByGoalState {
                ty,
                binders: Vec::new(),
            })
            .into_iter()
            .collect(),
        span: Some(d.span),
        step: -1,
        total: d.by_steps.len(),
    };
    if d.by_steps.is_empty() {
        return root();
    }
    let selected = match d
        .by_steps
        .iter()
        .position(|s| s.span.start.offset <= cursor && cursor < s.span.end.offset)
    {
        Some(i) => i as i64 - 1,
        None => d
            .by_steps
            .iter()
            .rposition(|s| s.span.end.offset <= cursor)
            .map(|i| i as i64)
            .unwrap_or(-1),
    };
    let Some(step) = usize::try_from(selected)
        .ok()
        .filter(|i| *i < d.by_steps.len())
    else {
        return root();
    };
    let s = &d.by_steps[step];
    StateSelection {
        goals: s.goals.clone(),
        span: Some(s.span),
        step: selected,
        total: d.by_steps.len(),
    }
}

/// 位置换算：字节 offset → **1-based** 行/列（列按 UTF-16 code unit，与 LSP 的
/// `character` 口径一致）。适配器负责把它转成自己要的基数。
pub fn line_col_of(text: &str, offset: usize) -> (usize, usize) {
    let mut line = 1usize;
    let mut col = 1usize;
    for (i, ch) in text.char_indices() {
        if i >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += ch.len_utf16();
        }
    }
    (line, col)
}

/// 位置换算：**1-based** 行/列（列按 UTF-16 code unit）→ 字节 offset。
/// 越界时返回 `None`（调用方把它变成 [`QueryError::PositionOutOfRange`]）。
pub fn offset_of_line_col(text: &str, line: usize, col: usize) -> Option<usize> {
    if line == 0 || col == 0 {
        return None;
    }
    let mut cur_line = 1usize;
    let mut cur_col = 1usize;
    for (i, ch) in text.char_indices() {
        if cur_line == line && cur_col == col {
            return Some(i);
        }
        if ch == '\n' {
            if cur_line == line {
                // 请求的行在这一行的换行处结束：夹到行尾。
                return Some(i);
            }
            cur_line += 1;
            cur_col = 1;
        } else {
            cur_col += ch.len_utf16();
        }
    }
    if cur_line == line {
        return Some(text.len());
    }
    None
}

/// 把报告里的洞 span 转成 offset 区间（供 `holes`/`next_hole` 复用）。
pub fn span_offsets(span: Span) -> (usize, usize) {
    (span.start.offset, span.end.offset)
}

#[cfg(test)]
mod tests;

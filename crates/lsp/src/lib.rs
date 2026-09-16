//! `sokonanoda-lsp`: a language server for `.sokonanoda` teaching files.
//!
//! Feedback philosophy (docs/design/infrastructure.md):
//! - diagnostics per declaration with stable codes and teaching hints;
//! - hover shows the inferred type of the expression under the cursor
//!   (from the front-end type map) or the goal of an open exercise `sorry`;
//! - document symbols / code lenses expose exercise state
//!   (open / solved / failed);
//! - code actions turn the first proof step into text and are ordered by
//!   kernel-verified first (see docs/design/hints-suggestions.md);
//! - rename / references / inlay hints follow LSP 3.17
//!   (docs/design/rename-inlay.md).
//!
//! The crate is also a library so the single `sokonanoda` binary can host
//! the server (`sokonanoda lsp`, the gleam pattern): call
//! [`run`] from any front-end.

mod actions;
mod hints;
mod inlay;
mod render;
#[cfg(test)]
mod testutil;

use actions::hole_range;
use render::{
    bracket_hover, decl_at, decl_name, definition_at, diagnostic_from_compile,
    diagnostic_from_parse, expr_hover, highlight_uses, hover_type_at, range_of, scope_names_at,
    semantic_kind_at, status_label, symbol_kind,
};
use serde::{Deserialize, Serialize};
use sokonanoda_front::compile::cache::{self, CachedCompile};
use sokonanoda_front::compile::{
    prelude_mode_from_source, ByStepState, CompileOptions, DeclState, DeclStatus, DocumentReport,
    GoalBinder, HoverType, PreludeMode,
};
use sokonanoda_front::semantic::{
    semantic_tokens as front_semantic_tokens, SemanticKind, SemanticSpan,
};
use sokonanoda_front::session::Session;
use std::sync::Mutex;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

#[derive(Debug)]
struct Doc {
    text: String,
    /// 会话式编译（I8）：持有上一版本的声明快照，编辑只重查受影响后缀。
    session: Session,
    mode: PreludeMode,
    report: Option<DocumentReport>,
    parse_error: Option<sokonanoda_front::Diagnostic>,
    /// The document's LSP version; versioned `WorkspaceEdit`s (rename) must
    /// carry it for atomic client-side application.
    version: i32,
}

impl Doc {
    fn new() -> Self {
        Self {
            text: String::new(),
            session: Session::new(CompileOptions::default()),
            mode: PreludeMode::Full,
            report: None,
            parse_error: None,
            version: 0,
        }
    }
}

/// 请求期 kernel 探针后的报告（design spine-meta-a.md §2/§4）：只把开放练习
/// 里 `sub_goals[i].ty == None` 的项交给 `front::probe_sub_goal_types`
/// 按洞 span 覆盖填充。**绝不进 didChange / keystroke 路径**——此函数只在
/// `soko/goals` / inlay / hover 请求里调用；无待填类型时零探针（只 clone）。
fn probed_report(doc: &Doc) -> DocumentReport {
    let Some(report) = &doc.report else {
        return DocumentReport::default();
    };
    let needs_probe = report
        .decls
        .iter()
        .any(|d| d.status == DeclStatus::Open && d.sub_goals.iter().any(|s| s.ty.is_none()));
    if !needs_probe {
        return report.clone();
    }
    let options = CompileOptions { prelude: doc.mode };
    let mut report = report.clone();
    for d in &mut report.decls {
        if d.status != DeclStatus::Open || !d.sub_goals.iter().any(|s| s.ty.is_none()) {
            continue;
        }
        let probed = sokonanoda_front::compile::probe_sub_goal_types(&doc.text, &options, d.span);
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

struct Backend {
    client: Client,
    doc: Mutex<Doc>,
}

/// LSP legend：front 的 [`SemanticKind`] 全部映射到标准 `SemanticTokenType`。
/// 顺序即 wire 上 `tokenType` 的下标；测试通过常量解析下标，重排是安全的。
fn semantic_token_types() -> Vec<SemanticTokenType> {
    vec![
        SemanticTokenType::KEYWORD,
        SemanticTokenType::TYPE,
        SemanticTokenType::NUMBER,
        SemanticTokenType::MACRO,
        SemanticTokenType::FUNCTION,
        SemanticTokenType::VARIABLE,
        SemanticTokenType::ENUM_MEMBER,
        SemanticTokenType::PARAMETER,
    ]
}

fn semantic_token_options() -> SemanticTokensOptions {
    SemanticTokensOptions {
        work_done_progress_options: WorkDoneProgressOptions::default(),
        legend: SemanticTokensLegend {
            token_types: semantic_token_types(),
            token_modifiers: vec![],
        },
        range: None,
        full: Some(SemanticTokensFullOptions::Bool(true)),
    }
}

/// front kind → legend 下标（语言知识在 front，这里只查表）。
fn token_type_index(kind: SemanticKind) -> u32 {
    let ty = match kind {
        SemanticKind::Keyword => SemanticTokenType::KEYWORD,
        SemanticKind::Sort | SemanticKind::InductiveName | SemanticKind::InductiveUse => {
            SemanticTokenType::TYPE
        }
        SemanticKind::Number => SemanticTokenType::NUMBER,
        SemanticKind::Hole => SemanticTokenType::MACRO,
        SemanticKind::DefName
        | SemanticKind::DefUse
        | SemanticKind::TheoremName
        | SemanticKind::TheoremUse => SemanticTokenType::FUNCTION,
        SemanticKind::AxiomName | SemanticKind::AxiomUse => SemanticTokenType::TYPE,
        SemanticKind::UnknownIdent => SemanticTokenType::VARIABLE,
        SemanticKind::CtorName | SemanticKind::CtorUse => SemanticTokenType::ENUM_MEMBER,
        SemanticKind::Binder => SemanticTokenType::PARAMETER,
    };
    semantic_token_types()
        .iter()
        .position(|t| *t == ty)
        .expect("legend covers every SemanticKind") as u32
}

/// 把 front 的（字节 offset 坐标、已排序）span 编码成 LSP 的相对 UTF-16 编码。
///
/// `deltaLine` 相对前一个 token 的行；`deltaStart` 在同一行时相对前一个
/// token 的起点，换行后是该行内的绝对 UTF-16 偏移。所有位置都按 UTF-16
/// code unit 计数（不是字节、也不是 char），首个虚拟“前一个 token”位于
/// (line 0, utf16 0)，因此首 token 无需特判。
fn encode_semantic_tokens(text: &str, spans: &[SemanticSpan]) -> Vec<SemanticToken> {
    let mut spans: Vec<SemanticSpan> = spans.to_vec();
    spans.sort_by_key(|s| s.span.start.offset);

    let mut line_starts = vec![0usize];
    for (i, b) in text.bytes().enumerate() {
        if b == b'\n' {
            line_starts.push(i + 1);
        }
    }

    let mut data = Vec::with_capacity(spans.len());
    let mut prev_line = 0usize;
    let mut prev_start = 0usize;
    for s in spans {
        let (from, to) = (s.span.start.offset, s.span.end.offset);
        if to <= from || to > text.len() {
            continue;
        }
        let line = line_starts.partition_point(|&start| start <= from) - 1;
        let line_start = line_starts[line];
        let start_utf16: usize = text[line_start..from].chars().map(char::len_utf16).sum();
        let length_utf16: usize = text[from..to].chars().map(char::len_utf16).sum();
        let delta_line = line - prev_line;
        let delta_start = if delta_line == 0 {
            start_utf16 - prev_start
        } else {
            start_utf16
        };
        data.push(SemanticToken {
            delta_line: delta_line as u32,
            delta_start: delta_start as u32,
            length: length_utf16 as u32,
            token_type: token_type_index(s.kind),
            token_modifiers_bitset: 0,
        });
        prev_line = line;
        prev_start = start_utf16;
    }
    data
}

impl Backend {
    fn new(client: Client) -> Self {
        Self {
            client,
            doc: Mutex::new(Doc::new()),
        }
    }

    async fn refresh(&self, uri: Url, text: String, version: Option<i32>) {
        // 原地复用会话（I8 增量的关键）：prelude 模式变化时才重建。
        // 教学文档量级小，锁内同步编译可接受（此前也是同步全量编译）。
        let diagnostics = {
            let mut doc = self.doc.lock().expect("doc lock");
            let mode = prelude_mode_from_source(&text);
            if doc.mode != mode {
                doc.session = Session::new(CompileOptions { prelude: mode });
                doc.mode = mode;
            }
            let options = CompileOptions { prelude: mode };
            let lsp_version = version.unwrap_or(0).max(0) as u64;
            // Shared persistent compile cache (`front::compile::cache`): the LSP
            // and the CLI read/write the *same* on-disk entries, so warming the
            // cache once (`sokonanoda build`, a prior `check`/`course`) also
            // warms the editor. A hit only replays a report the kernel already
            // produced for exactly this (compiler version, build, prelude mode,
            // text) — nothing is ever *inferred* from the cache. Unit tests skip
            // it (`cfg!(test)`), matching the CLI's policy, so `cargo test` never
            // touches the developer's real cache.
            let cached = if cfg!(test) {
                None
            } else {
                cache::load(&text, &options)
            };
            if let Some(entry) = cached {
                doc.text = text;
                doc.version = version.unwrap_or(doc.version);
                doc.parse_error = None;
                let diagnostics = report_diagnostics(&entry.report);
                doc.report = Some(entry.report);
                diagnostics
            } else {
                let update = doc.session.update(&text, lsp_version);
                doc.text = text;
                doc.version = version.unwrap_or(doc.version);
                match update.parse_error {
                    Some(diag) => {
                        let diagnostic = diagnostic_from_parse(&diag);
                        doc.parse_error = Some(diag);
                        doc.report = None;
                        vec![diagnostic]
                    }
                    None => {
                        doc.parse_error = None;
                        let report = update.report;
                        let diagnostics = report_diagnostics(&report);
                        if !cfg!(test) {
                            cache::store(
                                &doc.text,
                                &options,
                                &CachedCompile {
                                    report: report.clone(),
                                    output: None,
                                },
                            );
                        }
                        doc.report = Some(report);
                        diagnostics
                    }
                }
            }
        };
        let _ = self
            .client
            .publish_diagnostics(uri, diagnostics, version)
            .await;
    }

    // ---- I9 goal 视图协议：结构化 goal 请求（coq-lsp `proof/goals` 模式）----

    /// 组 `soko/goals` 的 wire 数据。`probe` = 是否跑请求期 kernel 探针填
    /// 函数 spine 子洞的期望类型（`nextHole` 只看洞 span，用 `false` 不引入
    /// 内核成本）。
    fn goal_decls(&self, probe: bool) -> Option<(String, Vec<GoalDeclInfo>)> {
        let doc = self.doc.lock().expect("doc lock");
        doc.report.as_ref()?;
        let report = if probe {
            probed_report(&doc)
        } else {
            doc.report.clone().unwrap_or_default()
        };
        let decls_names = sokonanoda_front::semantic::declaration_kinds(&doc.text);
        let decls = report
            .decls
            .iter()
            .map(|d| {
                let name = decl_name(d);
                let binder_names: Vec<String> = d.binders.iter().map(|b| b.name.clone()).collect();
                let ty_runs = d
                    .ty_text
                    .as_deref()
                    .map(|ty| runs_of(ty, &decls_names, &binder_names))
                    .unwrap_or_default();
                GoalDeclInfo {
                    name: name.clone(),
                    kind: d.kind.as_str().to_string(),
                    status: status_str(d.status).to_string(),
                    range: range_of(d.span),
                    ty: d.ty_text.clone(),
                    ty_runs,
                    goal: d.goal.clone(),
                    goals: match d.status {
                        DeclStatus::Open => d
                            .by_steps
                            .last()
                            .map(|s| s.goals.iter().map(|g| g.ty.clone()).collect())
                            .unwrap_or_else(|| d.goal.clone().into_iter().collect()),
                        _ => Vec::new(),
                    },
                    binders: d
                        .binders
                        .iter()
                        .map(|b| GoalBinderInfo {
                            name: b.name.clone(),
                            ty: b.ty.clone(),
                            ty_runs: runs_of(&b.ty, &decls_names, &binder_names),
                        })
                        .collect(),
                    hole: match d.status {
                        DeclStatus::Open => hole_range(&doc.text, d),
                        _ => None,
                    },
                    holes: match d.status {
                        DeclStatus::Open => d
                            .holes
                            .iter()
                            .enumerate()
                            .map(|(index, span)| HoleInfo {
                                range: range_of(*span),
                                id: format!("{name}:{index}"),
                            })
                            .collect(),
                        _ => Vec::new(),
                    },
                    sub_goals: match d.status {
                        DeclStatus::Open => d
                            .sub_goals
                            .iter()
                            .map(|sub| SubGoalInfo {
                                range: range_of(sub.span),
                                ty: sub.ty.clone(),
                            })
                            .collect(),
                        _ => Vec::new(),
                    },
                }
            })
            .collect();
        Some((doc.text.clone(), decls))
    }

    async fn goals(&self, params: GoalsParams) -> Result<GoalsResponse> {
        let _ = params;
        let decls = self
            .goal_decls(true)
            .map(|(_, decls)| decls)
            .unwrap_or_default();
        Ok(GoalsResponse { decls })
    }

    /// 服务器自述：版本 + 进程号。`sokonanoda: restart server` 用它在重启前后
    /// 各问一次，让「旧进程确实退出、新进程确实是新版本」变成**可见的事实**
    /// 而不是一句口头保证——扩展更新后跑着旧版服务器正是用户最常见的困惑
    /// （docs/vscode-dev-guide.md §5.6）。
    async fn version(&self, _params: serde_json::Value) -> Result<serde_json::Value> {
        Ok(serde_json::json!({
            "version": env!("CARGO_PKG_VERSION"),
            "pid": std::process::id(),
        }))
    }

    async fn next_hole(&self, params: NextHoleParams) -> Result<Option<Range>> {
        let Some((text, decls)) = self.goal_decls(false) else {
            return Ok(None);
        };
        let forward = params.forward.unwrap_or(true);
        let cursor = position_to_offset(&text, params.position);
        let mut holes: Vec<(usize, Range)> = decls
            .iter()
            .flat_map(|d| {
                d.holes
                    .iter()
                    .map(|h| (range_start_offset(&text, &h.range), h.range))
                    .collect::<Vec<_>>()
            })
            .collect();
        holes.sort_by_key(|(off, _)| *off);
        let found = if forward {
            holes.iter().find(|(off, _)| *off > cursor)
        } else {
            holes.iter().rev().find(|(off, _)| *off < cursor)
        };
        Ok(found.map(|(_, range)| *range))
    }

    /// Hint ladder for the declaration at the cursor (docs/design/hints-
    /// suggestions.md). Stateless: the client owns progressive disclosure.
    async fn hints(&self, params: hints::HintsParams) -> Result<hints::HintsResponse> {
        let doc = self.doc.lock().expect("doc lock");
        Ok(hints::hints_for(&doc, params))
    }

    /// Per-tactic goal state at the cursor (`soko/stateAt`,
    /// docs/design/by-tactics.md §6). Lean `goalsAt?` semantics: a cursor
    /// inside a tactic shows the state **entering** that tactic; otherwise
    /// the state after the last tactic that ended before it. The response
    /// carries the document version so clients drop stale answers.
    async fn state_at(&self, params: StateAtParams) -> Result<StateAtResponse> {
        let doc = self.doc.lock().expect("doc lock");
        let version = doc.version;
        let Some(report) = doc.report.as_ref() else {
            return Ok(StateAtResponse::empty(version));
        };
        let cursor = position_to_offset(&doc.text, params.position);
        let Some(d) = report
            .decls
            .iter()
            .find(|d| d.span.start.offset <= cursor && cursor <= d.span.end.offset)
        else {
            return Ok(StateAtResponse::empty(version));
        };
        let selection = select_state_at(d, cursor);
        // One parse of the document, then pure classification per goal/binder
        // (docs/design/goal-rendering.md §2.1).
        let decls = sokonanoda_front::semantic::declaration_kinds(&doc.text);
        let first = selection.goals.first();
        let first_names: Vec<String> = first
            .map(|g| g.binders.iter().map(|b| b.name.clone()).collect())
            .unwrap_or_default();
        Ok(StateAtResponse {
            version,
            decl: Some(StateDeclInfo {
                name: decl_name(d),
                kind: d.kind.as_str().to_string(),
                status: status_str(d.status).to_string(),
                range: range_of(d.span),
            }),
            // Single-value fields kept for older clients: the current goal
            // (first of `goals`) and its hypotheses.
            goal: first.map(|g| g.ty.clone()),
            goal_runs: first
                .map(|g| runs_of(&g.ty, &decls, &first_names))
                .unwrap_or_default(),
            binders: first
                .map(|g| {
                    g.binders
                        .iter()
                        .map(|b| GoalBinderInfo {
                            name: b.name.clone(),
                            ty: b.ty.clone(),
                            ty_runs: runs_of(&b.ty, &decls, &first_names),
                        })
                        .collect()
                })
                .unwrap_or_default(),
            goals: selection
                .goals
                .iter()
                .map(|g| {
                    let names: Vec<String> = g.binders.iter().map(|b| b.name.clone()).collect();
                    StateGoalInfo {
                        goal: g.ty.clone(),
                        goal_runs: runs_of(&g.ty, &decls, &names),
                        binders: g
                            .binders
                            .iter()
                            .map(|b| GoalBinderInfo {
                                name: b.name.clone(),
                                ty: b.ty.clone(),
                                ty_runs: runs_of(&b.ty, &decls, &names),
                            })
                            .collect(),
                    }
                })
                .collect(),
            span: selection.span.map(range_of),
            step: selection.step,
            total: selection.total,
        })
    }
}

// ---- soko/* 自定义请求的 wire 类型（docs/protocol.md）----

#[derive(Debug, Deserialize)]
struct GoalsParams {
    #[serde(rename = "textDocument")]
    #[allow(dead_code)]
    text_document: TextDocumentIdentifier,
    #[allow(dead_code)]
    #[serde(default)]
    position: Option<Position>,
}

#[derive(Debug, Serialize)]
struct RunInfo {
    text: String,
    /// [`sokonanoda_front::semantic::SemanticKind::as_str`]; absent = plain
    /// connector (whitespace/punctuation), drawn unstyled.
    #[serde(skip_serializing_if = "Option::is_none")]
    kind: Option<String>,
}

#[derive(Debug, Serialize)]
struct GoalBinderInfo {
    name: String,
    ty: String,
    /// Semantic runs of `ty` (`docs/design/goal-rendering.md` §2.1): the same
    /// classification the editor's semantic tokens use, so hover and the
    /// Infoview can never drift.
    ty_runs: Vec<RunInfo>,
}

/// Classify `text` into wire runs. `decls` is the document's declaration table
/// (computed once per request); `binders` are the names in scope.
fn runs_of(text: &str, decls: &[(String, SemanticKind)], binders: &[String]) -> Vec<RunInfo> {
    sokonanoda_front::semantic::tag_runs(text, decls, binders)
        .into_iter()
        .map(|run| RunInfo {
            text: run.text,
            kind: run.kind.map(|k| k.as_str().to_string()),
        })
        .collect()
}

#[derive(Debug, Serialize)]
struct HoleInfo {
    range: Range,
    /// Stable per (declaration, hole order) within a document version
    /// (`<declName>:<index>`, docs/protocol.md); anonymous examples use the
    /// `example@<line>` name form.
    id: String,
}

#[derive(Debug, Serialize)]
struct GoalDeclInfo {
    name: String,
    kind: String,
    status: String,
    range: Range,
    /// The declaration's kernel-rendered type (signature), when known — used by
    /// the Infoview declaration list as a small hint (`docs/protocol.md`).
    #[serde(skip_serializing_if = "Option::is_none")]
    ty: Option<String>,
    /// Semantic runs of `ty` (same single source as `goal_runs`, §2.1).
    ty_runs: Vec<RunInfo>,
    goal: Option<String>,
    /// Every open goal after the last recorded tactic (current goal first),
    /// or the single walked remaining goal for non-`by` open exercises.
    /// Empty for non-open declarations. Powers the multi-goal exercise panel.
    goals: Vec<String>,
    binders: Vec<GoalBinderInfo>,
    hole: Option<Range>,
    /// Every `sorry` in the answer (main hole + constructor-spine sub-holes),
    /// as `{range, id}` objects (`id` = `<declName>:<index>`).
    holes: Vec<HoleInfo>,
    /// Expected types for the sub-holes, positionally aligned with `holes`
    /// subset that came from a constructor spine (server-side walk).
    sub_goals: Vec<SubGoalInfo>,
}

#[derive(Debug, Serialize)]
struct SubGoalInfo {
    range: Range,
    ty: Option<String>,
}

#[derive(Debug, Serialize)]
struct GoalsResponse {
    decls: Vec<GoalDeclInfo>,
}

#[derive(Debug, Deserialize)]
struct NextHoleParams {
    #[serde(rename = "textDocument")]
    #[allow(dead_code)]
    text_document: TextDocumentIdentifier,
    position: Position,
    #[serde(default)]
    forward: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct StateAtParams {
    #[serde(rename = "textDocument")]
    #[allow(dead_code)]
    text_document: TextDocumentIdentifier,
    position: Position,
}

#[derive(Debug, Serialize)]
struct StateDeclInfo {
    name: String,
    kind: String,
    status: String,
    range: Range,
}

/// One open goal inside `soko/stateAt`'s `goals` array.
#[derive(Debug, Serialize)]
struct StateGoalInfo {
    goal: String,
    /// Semantic runs of `goal` (`docs/design/goal-rendering.md` §2.1).
    goal_runs: Vec<RunInfo>,
    binders: Vec<GoalBinderInfo>,
}

/// `soko/stateAt` response (docs/protocol.md): the goal state at the cursor,
/// plus enough declaration info for the client to label and reveal it.
#[derive(Debug, Serialize)]
struct StateAtResponse {
    version: i32,
    decl: Option<StateDeclInfo>,
    /// `None` = no remaining goals (the proof is closed at this position).
    /// Single-value, equal to `goals[0].goal`; kept for older clients.
    goal: Option<String>,
    /// Semantic runs of `goal` (= `goals[0].goal_runs`); kept for older clients.
    goal_runs: Vec<RunInfo>,
    /// Hypotheses of `goal` (single-value, equal to `goals[0].binders`).
    binders: Vec<GoalBinderInfo>,
    /// Every remaining goal, current goal first (`[]` = closed). Clients that
    /// render goal lists should prefer this over the single `goal`.
    goals: Vec<StateGoalInfo>,
    /// The tactic's range (root state: the declaration's range).
    span: Option<Range>,
    /// Index of the selected per-tactic state; `-1` = root.
    step: i64,
    total: usize,
}

impl StateAtResponse {
    fn empty(version: i32) -> Self {
        Self {
            version,
            decl: None,
            goal: None,
            goal_runs: Vec::new(),
            binders: Vec::new(),
            goals: Vec::new(),
            span: None,
            step: -1,
            total: 0,
        }
    }
}

/// Wire status string shared by `soko/goals` and `soko/stateAt`.
fn status_str(status: DeclStatus) -> &'static str {
    match status {
        DeclStatus::Open => "open",
        DeclStatus::Checked => "checked",
        DeclStatus::Failed => "failed",
    }
}

/// One goal of the selected state: its rendered type and in-scope hypotheses.
struct SelectedGoal {
    ty: String,
    binders: Vec<GoalBinder>,
}

/// The declaration state selected for one cursor offset (`soko/stateAt`).
struct StateSelection {
    /// Every open goal at this position, current goal first (empty = closed).
    goals: Vec<SelectedGoal>,
    span: Option<sokonanoda_front::Span>,
    /// Index of the selected per-tactic state; `-1` = root (before any tactic).
    step: i64,
    total: usize,
}

impl StateSelection {
    fn root(d: &DeclState) -> Self {
        // Root goal: the full declared type (kernel-rendered) when known,
        // falling back to the remaining goal for plain open exercises.
        let goals = d
            .ty_text
            .clone()
            .or_else(|| d.goal.clone())
            .map(|ty| SelectedGoal {
                ty,
                binders: Vec::new(),
            })
            .into_iter()
            .collect();
        Self {
            goals,
            span: Some(d.span),
            step: -1,
            total: 0,
        }
    }
}

/// Select the state at `cursor` (Lean `goalsAt?` semantics).
///
/// - No `by` steps: the declaration's remaining goal/context (the best the
///   walk recovered; `step = -1`).
/// - With steps: a cursor inside a tactic's span `[start, end)` shows the
///   state **entering** that tactic (`steps[i-1]` after-state, or the root
///   for the first tactic); otherwise the state after the last tactic whose
///   span ends at or before the cursor (root when none has).
fn select_state_at(d: &DeclState, cursor: usize) -> StateSelection {
    if d.by_steps.is_empty() {
        let goals = d
            .goal
            .clone()
            .map(|ty| SelectedGoal {
                ty,
                binders: d.binders.clone(),
            })
            .into_iter()
            .collect();
        return StateSelection {
            goals,
            span: Some(d.span),
            step: -1,
            total: 0,
        };
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
    let mut selection = StateSelection::root(d);
    selection.total = d.by_steps.len();
    if let Some(step) = usize::try_from(selected)
        .ok()
        .filter(|i| *i < d.by_steps.len())
    {
        let s: &ByStepState = &d.by_steps[step];
        selection.goals = s
            .goals
            .iter()
            .map(|g| SelectedGoal {
                ty: g.ty.clone(),
                binders: g.binders.clone(),
            })
            .collect();
        selection.span = Some(s.span);
        selection.step = selected;
    }
    selection
}

/// 0-based LSP position → byte offset（与本服务器的 char 计数约定一致）。
fn position_to_offset(text: &str, position: Position) -> usize {
    let mut offset = 0usize;
    for (i, line) in text.lines().enumerate() {
        if i == position.line as usize {
            let char_idx = text[offset..]
                .char_indices()
                .nth(position.character as usize)
                .map(|(i, _)| i)
                .unwrap_or(line.len());
            return offset + char_idx.min(line.len());
        }
        offset += line.len() + 1;
    }
    text.len()
}

fn range_start_offset(text: &str, range: &Range) -> usize {
    position_to_offset(text, range.start)
}

/// Hover on a `by` tactic shows the goal state **entering** that tactic
/// (Lean Infoview-style, user request): every remaining goal with the
/// hypotheses in scope, computed from the per-tactic snapshot the front
/// already records (`by_steps`) — no re-check, no text scan. The whole tactic
/// span is the trigger; the goal view's hypotheses make term hovers redundant
/// inside it.
fn tactic_goal_hover(
    report: &DocumentReport,
    text: &str,
    offset: usize,
    decls: &[(String, SemanticKind)],
) -> Option<Hover> {
    let d = report
        .decls
        .iter()
        .find(|d| d.span.start.offset <= offset && offset <= d.span.end.offset)?;
    let step_index = d
        .by_steps
        .iter()
        .position(|s| s.span.start.offset <= offset && offset <= s.span.end.offset)?;
    let step = &d.by_steps[step_index];
    // `select_state_at` on the tactic's start returns the *entering* state.
    let selection = select_state_at(d, step.span.start.offset);
    let tactic_text = text
        .get(step.span.start.offset..step.span.end.offset)
        .unwrap_or("")
        .trim();
    // Header: the tactic itself + its 1-based position. The tactic is a
    // `sokonanoda` code block too, so its own syntax is highlighted (same fence
    // language as the goal state below, docs/design/goal-rendering.md §7).
    let mut value = code_block(tactic_text);
    if selection.total > 0 {
        value.push_str(&format!(
            "\ntactic {}/{}\n",
            step_index + 1,
            selection.total
        ));
    } else {
        value.push('\n');
    }
    if selection.goals.is_empty() {
        value.push_str("\n已无剩余目标 ✓\n");
    } else {
        let n = selection.goals.len();
        for (i, goal) in selection.goals.iter().enumerate() {
            value.push('\n');
            if n > 1 {
                value.push_str(&format!("**目标 {}/{}**\n", i + 1, n));
            }
            value.push_str(&goal_block(decls, &goal.binders, &goal.ty));
            value.push('\n');
        }
    }
    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value,
        }),
        range: Some(range_of(step.span)),
    })
}

/// Diagnostics for a compiled [`sokonanoda_front::compile::DocumentReport`]:
/// elab/kernel errors, one WARNING per `sorry` (Lean-4 aligned), and
/// syntax-level warnings. Shared by the fresh-compile and cache-hit paths so a
/// cached open produces exactly the same diagnostics as a recompile.
fn report_diagnostics(report: &sokonanoda_front::compile::DocumentReport) -> Vec<Diagnostic> {
    let mut diagnostics: Vec<_> = report.errors.iter().map(diagnostic_from_compile).collect();
    if report
        .decls
        .iter()
        .any(|d| d.status == DeclStatus::Open && !d.holes.is_empty())
    {
        for d in report.decls.iter().filter(|d| d.status == DeclStatus::Open) {
            let name = d.name.as_deref().unwrap_or("(anonymous)");
            diagnostics.push(Diagnostic {
                range: range_of(d.span),
                severity: Some(DiagnosticSeverity::WARNING),
                code: Some(NumberOrString::String("sorry".to_string())),
                source: Some("sokonanoda".to_string()),
                message: format!(
                    "declaration '{}' uses 'sorry' (exercise not yet solved)",
                    name
                ),
                ..Diagnostic::default()
            });
        }
    }
    for warning in &report.warnings {
        diagnostics.push(Diagnostic {
            range: range_of(warning.span),
            severity: Some(DiagnosticSeverity::WARNING),
            code: Some(NumberOrString::String(warning.code().to_string())),
            source: Some("sokonanoda".to_string()),
            message: format!("{}\n\n提示：{}", warning.message, warning.hint()),
            ..Diagnostic::default()
        });
    }
    diagnostics
}

/// Language id used by **every** markdown code fence the server emits, so the
/// editor colours it with the `sokonanoda` TextMate grammar (which is
/// contract-tested against `front::semantic`) — the single source for any
/// `.sokonanoda` text the client renders (docs/design/goal-rendering.md §7).
const CODE_LANG: &str = "sokonanoda";

/// A fenced `sokonanoda` code block for editor markdown (hover / completion
/// docs / any place that shows language text).
fn code_block(text: &str) -> String {
    format!("```{CODE_LANG}\n{}\n```", text.trim_end_matches('\n'))
}

/// Render a goal state (hypotheses + `⊢ goal`) as one `sokonanoda` code block —
/// the same line model as the tactic hover and the Infoview. The fence text is
/// the **text projection of the front goal runs** ([`goal_runs`] +
/// [`runs_to_text`]), i.e. exactly the block the Infoview colours from the
/// `soko/stateAt` `ty_runs`/`goal_runs`, so the two can never drift.
fn goal_block(decls: &[(String, SemanticKind)], binders: &[GoalBinder], goal: &str) -> String {
    let hyps: Vec<(String, String)> = binders
        .iter()
        .map(|b| (b.name.clone(), b.ty.clone()))
        .collect();
    let runs = sokonanoda_front::semantic::goal_runs(&hyps, goal, decls);
    code_block(&sokonanoda_front::semantic::runs_to_text(&runs))
}

/// Build an LSP `Hover` from a resolved expression hover, carrying the
/// expression's source range so the editor highlights exactly what is shown.
fn hover_markup(res: render::HoverResolved) -> Hover {
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: code_block(&res.content),
        }),
        range: Some(range_of(res.range)),
    }
}

/// 半截表达式的 goal-state hover（I13-S5，用户需求）：值写了一半、内核
/// 拒绝时（如 `And.intro b a` 还差两个前提），hover 不只给报错——把推断
/// 出的**剩余目标**列出来（`⊢ b`、`⊢ a`）。
///
/// 性能边界：只在 **hover 请求时**计算（不在按键路径上），且 `judge_infer`
/// 有缓存——同一位置重复悬停零成本；指纹含前缀文本，其它位置的编辑会
/// 失效缓存（保守但正确）。
fn half_expression_goals_hover(
    report: &DocumentReport,
    text: &str,
    offset: usize,
    decls: &[(String, SemanticKind)],
) -> Option<Hover> {
    use sokonanoda_front::compile::CompileOptions;
    use sokonanoda_front::proof::{parse_expr_text, peel_pi_layers, render_expr};
    use sokonanoda_front::{parse, Command, Expr};

    let d = report
        .decls
        .iter()
        .find(|d| d.span.start.offset <= offset && offset <= d.span.end.offset)?;
    if d.status != DeclStatus::Failed {
        return None;
    }
    let error = d.error.as_ref()?;
    if error.code() != "kernel-rejected" {
        return None;
    }
    // 值文本 = 声明切片里最后一个 `:=` 之后的部分（表达式语法不含 `:=`）。
    let slice = &text[d.span.start.offset..d.span.end.offset];
    let (_head, value) = slice.rsplit_once(":=")?;
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    // 值自己的 lambda 链 = 判定上下文（学习者命名的 binder 原样可用）。
    let value_expr = parse_expr_text(value).ok()?;
    let mut binders: Vec<sokonanoda_front::judge::GoalBinderSpec> = Vec::new();
    let mut body = &value_expr;
    while let Expr::Lambda {
        binders: bs,
        body: b,
        ..
    } = body
    {
        for b in bs {
            binders.push(sokonanoda_front::judge::GoalBinderSpec {
                name: b.name.clone(),
                ty: b.ty.as_deref().map(render_expr),
            });
        }
        body = b;
    }
    if binders.is_empty() {
        return None; // 没有引入 binder 的半截表达式：诊断已足够
    }
    let term_text = render_expr(body);
    // 声明目标 = 声明类型剥掉值已消耗的层数。
    let file = parse(slice).ok()?;
    let ty = match file.commands.first()? {
        Command::Theorem { ty, .. } | Command::Def { ty, .. } => ty.clone(),
        _ => return None,
    };
    let goal_ty = peel_pi_layers(&ty, binders.len())?;
    let goal_text = render_expr(&goal_ty);

    // 问内核：这一项在上下文里的类型（推断，不是判定；有缓存）。
    let prefix = &text[..d.span.start.offset];
    let inferred = sokonanoda_front::judge::judge_infer(
        prefix,
        &CompileOptions::default(),
        &binders,
        &term_text,
    )
    .ok()?;
    let inferred_expr = parse_expr_text(&inferred).ok()?;
    let mut goals: Vec<String> = Vec::new();
    let mut cur = &inferred_expr;
    loop {
        match cur {
            Expr::Arrow {
                domain, codomain, ..
            } => {
                goals.push(render_expr(domain));
                cur = codomain;
            }
            Expr::Forall { binders, body, .. } => {
                for binder in binders {
                    goals.push(render_expr(
                        binder.ty.as_deref().unwrap_or_else(|| body.as_ref()),
                    ));
                }
                cur = body;
            }
            _ => break,
        }
    }
    if goals.is_empty() {
        return None; // 不是部分应用：诊断已足够
    }
    let codomain = render_expr(cur);
    let (headline, tail) = if codomain == goal_text {
        (
            format!(
                "这一项的结论已经对上目标：\n{}\n还差 {} 个前提：",
                code_block(&goal_text),
                goals.len()
            ),
            "\n\n继续把前提补上，或用 `by` / `fun` 继续写。".to_string(),
        )
    } else {
        (
            format!(
                "这一项的类型是：\n{}\n与目标对不上：\n{}",
                code_block(&inferred),
                code_block(&goal_text)
            ),
            String::new(),
        )
    };
    let goal_block = code_block(
        &goals
            .iter()
            .map(|g| {
                let runs = sokonanoda_front::semantic::goal_runs(&[], g, decls);
                sokonanoda_front::semantic::runs_to_text(&runs)
            })
            .collect::<Vec<_>>()
            .join("\n"),
    );
    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: format!("{headline}\n\n{goal_block}{tail}"),
        }),
        range: Some(range_of(d.span)),
    })
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                document_highlight_provider: Some(OneOf::Left(true)),
                selection_range_provider: Some(SelectionRangeProviderCapability::Simple(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                rename_provider: Some(OneOf::Right(RenameOptions {
                    prepare_provider: Some(true),
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                })),
                references_provider: Some(OneOf::Left(true)),
                inlay_hint_provider: Some(OneOf::Right(InlayHintServerCapabilities::Options(
                    InlayHintOptions {
                        resolve_provider: Some(false),
                        work_done_progress_options: WorkDoneProgressOptions::default(),
                    },
                ))),
                code_lens_provider: Some(CodeLensOptions {
                    resolve_provider: Some(false),
                }),
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions {
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                    resolve_provider: Some(false),
                    trigger_characters: None,
                    all_commit_characters: None,
                    completion_item: None,
                }),
                folding_range_provider: Some(FoldingRangeProviderCapability::Simple(true)),
                semantic_tokens_provider: Some(semantic_token_options().into()),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {}

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.refresh(
            params.text_document.uri,
            params.text_document.text,
            Some(params.text_document.version),
        )
        .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        // FULL sync delivers the whole text; the last change is the final state.
        if let Some(change) = params.content_changes.into_iter().last() {
            self.refresh(
                params.text_document.uri,
                change.text,
                Some(params.text_document.version),
            )
            .await;
        }
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        if let Some(text) = params.text {
            self.refresh(params.text_document.uri, text, None).await;
        }
    }

    async fn did_close(&self, _: DidCloseTextDocumentParams) {}

    async fn semantic_tokens_full(
        &self,
        _: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        // 始终对当前存储的文本重新计算：解析失败时 front 的
        // semantic_tokens 自身退化为纯词法分类，绝不复用过期报告。
        let text = {
            let doc = self.doc.lock().expect("doc lock");
            doc.text.clone()
        };
        let spans = front_semantic_tokens(&text);
        Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
            result_id: None,
            data: encode_semantic_tokens(&text, &spans),
        })))
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let doc = self.doc.lock().expect("doc lock");
        if doc.report.is_none() {
            return Ok(None);
        }
        // 请求期探针：洞期望类型（含函数 spine 的 None 项）在 hover 时补齐。
        let report = probed_report(&doc);
        let report = &report;
        let pos = params.text_document_position_params.position;
        let offset = position_to_offset(&doc.text, pos);
        // Declaration table computed once per hover request and reused by every
        // goal-block builder below (front goal runs need it for classification).
        let decls = sokonanoda_front::semantic::declaration_kinds(&doc.text);
        // `by` tactic hover: show the goal state entering the tactic under the
        // cursor (Lean Infoview-style, user request). Before keyword suppression
        // below, because tactic words (intro/exact/…) are keywords.
        if let Some(hover) = tactic_goal_hover(report, &doc.text, offset, &decls) {
            return Ok(Some(hover));
        }
        // 半截表达式的 goal-state（内核拒绝 + 有可推断的部分应用）。
        // 只在 hover 请求时计算（不在按键路径），judge_infer 有缓存。
        if let Some(hover) = half_expression_goals_hover(report, &doc.text, offset, &decls) {
            return Ok(Some(hover));
        }
        // 关键字（fun/=>/theorem/axiom…）上不吐类型行：那一行的悬停信息
        // 应该来自名字/表达式，而不是把关键字所在的某个节点硬塞过来。
        if let Some(kind) = semantic_kind_at(&doc.text, pos.line, pos.character) {
            if matches!(kind, SemanticKind::Keyword) {
                return Ok(None);
            }
        }
        // 括号优先：光标在 ( / ) 上 → 显示括号组包住的表达式及其类型
        //（`(表达式)` 的悬停 = `表达式 : 类型`）。必须先于精确命中——
        // 外层 lambda 行的 span 覆盖整个值表达式，会遮住括号组。
        if let Some(res) = bracket_hover(&doc.text, &report.hovers, pos.line, pos.character) {
            return Ok(Some(hover_markup(res)));
        }
        if let Some(h) = hover_type_at(&report.hovers, pos.line, pos.character) {
            // 学习者需求：显示「表达式 : 类型」——表达式从源码按 span 切片
            //（括号平衡成良构），并返回表达式范围供编辑器高亮。
            return Ok(Some(hover_markup(expr_hover(&doc.text, h))));
        }
        // 邻近回退：光标 ±2 字符内命中的最小外层表达式（运算符、空白
        // 边缘等结构符号也能看到所属类型）。数据来自 hover 表（span 嵌套）。
        {
            const TOLERANCE: usize = 2;
            let nearest = report
                .hovers
                .iter()
                .filter(|h| {
                    let start = h.span.start.offset;
                    let end = h.span.end.offset;
                    // 精确包含（已被上面处理，这里补边界附近）
                    start <= offset + TOLERANCE && offset.saturating_sub(TOLERANCE) < end
                })
                .min_by_key(|h| {
                    let len = h.span.end.offset - h.span.start.offset;
                    let dist = if offset >= h.span.start.offset && offset <= h.span.end.offset {
                        0
                    } else if offset < h.span.start.offset {
                        h.span.start.offset - offset
                    } else {
                        offset - h.span.end.offset
                    };
                    (dist, len)
                });
            if let Some(h) = nearest {
                return Ok(Some(hover_markup(expr_hover(&doc.text, h))));
            }
        }
        if let Some(d) = decl_at(&report.decls, pos.line, pos.character) {
            let signature = match &d.ty_text {
                Some(ty) => format!("{} {} : {}", d.kind.as_str(), decl_name(d), ty),
                None => format!("{} {}", d.kind.as_str(), decl_name(d)),
            };
            // Signature and goal state are `.sokonanoda` text → fenced blocks so
            // the editor highlights them (docs/design/goal-rendering.md §7).
            let mut value = code_block(&signature);
            match d.status {
                DeclStatus::Open => {
                    // 光标正落在某个 `sorry` 上：先给这个洞的精确期望类型
                    //（超量应用走查经 def 展开算出，如 `(And.right a (Not a)
                    // x) sorry` 的洞期望 `a`，而不是整个声明类型）。
                    let hole_ty = d
                        .sub_goals
                        .iter()
                        .find(|s| s.span.start.offset <= offset && offset <= s.span.end.offset);
                    match (&d.goal, hole_ty) {
                        (Some(goal), Some(sg)) if sg.ty.is_some() => value.push_str(&format!(
                            "\n此处 `sorry` 的期望类型：\n{}\n\n剩余目标：\n{}",
                            code_block(sg.ty.as_deref().unwrap_or_default()),
                            goal_block(&decls, &d.binders, goal)
                        )),
                        (Some(goal), _) => {
                            value.push_str(&format!(
                                "\n目标：\n{}",
                                goal_block(&decls, &d.binders, goal)
                            ));
                        }
                        (None, _) => value.push_str("\n待作答"),
                    }
                    value.push_str("\n\n在 `sorry` 处填写一个类型为目标的项。");
                }
                DeclStatus::Checked => value.push_str("\n\n已通过内核检查"),
                DeclStatus::Failed => value.push_str("\n\n未通过，见诊断"),
            };
            return Ok(Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value,
                }),
                range: Some(range_of(d.span)),
            }));
        }
        Ok(None)
    }

    async fn selection_range(
        &self,
        params: SelectionRangeParams,
    ) -> Result<Option<Vec<SelectionRange>>> {
        // 优先级可视化（学习者需求）：光标放在某个符号/运算符上，
        // 逐级放大选中"先结合"的表达式。数据来自 hover 表——每个
        // AST 节点（含箭头/应用）都有行，span 天然嵌套。
        let doc = self.doc.lock().expect("doc lock");
        let Some(report) = &doc.report else {
            return Ok(None);
        };
        let mut out = Vec::with_capacity(params.positions.len());
        for pos in &params.positions {
            let offset = position_to_offset(&doc.text, *pos);
            let mut rows: Vec<&HoverType> = report
                .hovers
                .iter()
                .filter(|h| {
                    h.span.start.offset <= offset
                        && offset < h.span.end.offset.max(h.span.start.offset + 1)
                })
                .collect();
            // 内层在前（span 小的先选中）；同一 span 只留一条。
            rows.sort_by_key(|h| h.span.end.offset - h.span.start.offset);
            rows.dedup_by(|a, b| a.span == b.span);
            // SelectionRange 的 root 是最内层选择、parent 向外指：
            // 从最外层往里建链，最后一个处理的（最小 span）成为 root。
            let mut chain: Option<SelectionRange> = None;
            let mut last_range: Option<Range> = None;
            for h in rows.into_iter().rev() {
                let range = range_of(h.span);
                if last_range == Some(range) {
                    continue;
                }
                last_range = Some(range);
                chain = Some(SelectionRange {
                    range,
                    parent: chain.map(Box::new),
                });
            }
            out.push(chain.unwrap_or(SelectionRange {
                range: Range {
                    start: *pos,
                    end: *pos,
                },
                parent: None,
            }));
        }
        Ok(Some(out))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let doc = self.doc.lock().expect("doc lock");
        let Some(report) = &doc.report else {
            return Ok(None);
        };
        let pos = params.text_document_position_params.position;
        let Some(target) = definition_at(&report.hovers, pos.line, pos.character) else {
            return Ok(None);
        };
        Ok(Some(GotoDefinitionResponse::Scalar(Location {
            uri: params.text_document_position_params.text_document.uri,
            range: range_of(target.span()),
        })))
    }

    async fn document_highlight(
        &self,
        params: DocumentHighlightParams,
    ) -> Result<Option<Vec<DocumentHighlight>>> {
        let doc = self.doc.lock().expect("doc lock");
        let Some(report) = &doc.report else {
            return Ok(None);
        };
        let pos = params.text_document_position_params.position;
        let Some(ranges) = highlight_uses(&report.hovers, pos.line, pos.character) else {
            return Ok(None);
        };
        Ok(Some(
            ranges
                .into_iter()
                .map(|range| DocumentHighlight {
                    range,
                    kind: Some(DocumentHighlightKind::TEXT),
                })
                .collect(),
        ))
    }

    async fn document_symbol(
        &self,
        _: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let doc = self.doc.lock().expect("doc lock");
        let Some(report) = &doc.report else {
            return Ok(None);
        };
        let symbols = report
            .decls
            .iter()
            .map(|d| DocumentSymbol {
                name: decl_name(d),
                detail: Some(format!("{} — {}", d.kind.as_str(), status_label(d.status))),
                kind: symbol_kind(d.kind),
                tags: None,
                #[allow(deprecated)] // lsp-types field is deprecated in favor of `tags`
                deprecated: None,
                range: range_of(d.span),
                selection_range: range_of(d.span),
                children: None,
            })
            .collect();
        Ok(Some(DocumentSymbolResponse::Nested(symbols)))
    }

    async fn code_lens(&self, _: CodeLensParams) -> Result<Option<Vec<CodeLens>>> {
        let doc = self.doc.lock().expect("doc lock");
        let Some(report) = &doc.report else {
            return Ok(None);
        };
        let lenses = report
            .decls
            .iter()
            .map(|d| CodeLens {
                range: range_of(d.span),
                command: Some(Command {
                    title: status_label(d.status).to_string(),
                    command: "sokonanoda.status".to_string(),
                    arguments: None,
                }),
                data: None,
            })
            .collect();
        Ok(Some(lenses))
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let doc = self.doc.lock().expect("doc lock");
        let mut items: Vec<CompletionItem> = Vec::new();
        // In-scope binders at the cursor (smallest enclosing hover row);
        // outside any hover span the list stays keyword/prelude-only.
        let pos = params.text_document_position.position;
        // In-scope binders at the cursor (smallest enclosing hover row);
        // outside any hover span the list stays keyword/prelude-only.
        if let Some(report) = &doc.report {
            if let Some(names) = scope_names_at(&report.hovers, pos.line, pos.character) {
                for name in names {
                    if name.is_empty() {
                        continue; // anonymous arrow binder
                    }
                    items.push(CompletionItem {
                        label: name.clone(),
                        kind: Some(CompletionItemKind::VARIABLE),
                        detail: Some("本域 binder".to_string()),
                        ..Default::default()
                    });
                }
            }
        }
        // Keywords (single source: front::semantic).
        for keyword in sokonanoda_front::semantic::keywords() {
            items.push(CompletionItem {
                label: (*keyword).to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                ..Default::default()
            });
        }
        // Sorts (Prop / Type / Sort).
        for sort in sokonanoda_front::semantic::sorts() {
            items.push(CompletionItem {
                label: (*sort).to_string(),
                kind: Some(CompletionItemKind::STRUCT),
                detail: Some("宇宙".to_string()),
                ..Default::default()
            });
        }
        // Trusted prelude names (no DeclState exists for them).
        for name in sokonanoda_front::compile::PRELUDE_NAMES {
            items.push(CompletionItem {
                label: (*name).to_string(),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some("prelude".to_string()),
                ..Default::default()
            });
        }
        // The document's own declarations (anonymous examples excluded).
        if let Some(report) = &doc.report {
            for decl in &report.decls {
                let Some(name) = &decl.name else {
                    continue;
                };
                if name.starts_with('_') {
                    continue; // internal names (_example_N)
                }
                items.push(CompletionItem {
                    label: name.clone(),
                    kind: Some(match decl.kind {
                        sokonanoda_front::compile::DeclKind::Inductive => {
                            CompletionItemKind::STRUCT
                        }
                        sokonanoda_front::compile::DeclKind::Axiom => CompletionItemKind::CONSTANT,
                        _ => CompletionItemKind::FUNCTION,
                    }),
                    detail: Some(format!(
                        "{} · {}",
                        decl.kind.as_str(),
                        status_label(decl.status)
                    )),
                    // Signature as a `sokonanoda` fence so the docs popup is
                    // highlighted like every other surface (§7).
                    documentation: decl.ty_text.as_ref().map(|ty| {
                        Documentation::MarkupContent(MarkupContent {
                            kind: MarkupKind::Markdown,
                            value: code_block(&format!("{} {} : {ty}", decl.kind.as_str(), name)),
                        })
                    }),
                    ..Default::default()
                });
            }
        }
        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn folding_range(&self, _: FoldingRangeParams) -> Result<Option<Vec<FoldingRange>>> {
        let doc = self.doc.lock().expect("doc lock");
        let Some(report) = &doc.report else {
            return Ok(None);
        };
        let ranges = report
            .decls
            .iter()
            .filter_map(|d| {
                // 0-based inclusive lines; clamp the end when the span ends
                // at a line start (trailing newline).
                let start = d.span.start.line.saturating_sub(1) as u32;
                let end = if d.span.end.column <= 1 {
                    d.span.end.line.saturating_sub(2)
                } else {
                    d.span.end.line.saturating_sub(1)
                } as u32;
                (start < end).then(|| FoldingRange {
                    start_line: start,
                    end_line: end,
                    kind: Some(FoldingRangeKind::Region),
                    ..Default::default()
                })
            })
            .collect();
        Ok(Some(ranges))
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let doc = self.doc.lock().expect("doc lock");
        let Some(report) = &doc.report else {
            return Ok(None);
        };
        Ok(actions::code_actions(
            params.text_document.uri.clone(),
            &doc.text,
            doc.mode,
            report,
            params.range.start,
        ))
    }

    async fn prepare_rename(
        &self,
        params: TextDocumentPositionParams,
    ) -> Result<Option<PrepareRenameResponse>> {
        let doc = self.doc.lock().expect("doc lock");
        let Some(report) = &doc.report else {
            return Ok(None);
        };
        Ok(render::prepare_rename(&doc.text, report, params.position))
    }

    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        let doc = self.doc.lock().expect("doc lock");
        let Some(report) = &doc.report else {
            return Err(tower_lsp::jsonrpc::Error::invalid_params(
                "当前文档无法解析，不能改名",
            ));
        };
        render::rename(&doc.text, doc.version, report, params)
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let doc = self.doc.lock().expect("doc lock");
        let Some(report) = &doc.report else {
            return Ok(None);
        };
        Ok(render::find_references(
            params.text_document_position.text_document.uri.clone(),
            &doc.text,
            report,
            params.text_document_position.position,
            params.context.include_declaration,
        ))
    }

    async fn inlay_hint(&self, params: InlayHintParams) -> Result<Option<Vec<InlayHint>>> {
        let doc = self.doc.lock().expect("doc lock");
        if doc.report.is_none() {
            return Ok(None);
        }
        // 请求期探针补齐函数 spine 子洞的期望类型（inlay 是惰性请求）。
        let report = probed_report(&doc);
        let _ = params.range;
        Ok(Some(inlay::document_hints(&doc.text, &report)))
    }
}

/// Run the LSP server over stdio. Library entry so the single `sokonanoda`
/// binary can host the server via `sokonanoda lsp` (gleam pattern); the
/// `sokonanoda-lsp` binary calls this too. stdout carries only LSP frames.
#[tokio::main]
pub async fn run() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let (service, socket) = LspService::build(Backend::new)
        .custom_method("soko/goals", Backend::goals)
        .custom_method("soko/nextHole", Backend::next_hole)
        .custom_method("soko/hints", Backend::hints)
        .custom_method("soko/stateAt", Backend::state_at)
        .custom_method("soko/version", Backend::version)
        .finish();
    Server::new(stdin, stdout, socket).serve(service).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{
        call, code_of, did_open, handshake, lsp_pos, notify, offset_of, position_json, shutdown,
        test_service, type_step, wait_diagnostics, TypedStep, URI,
    };
    use serde_json::json;
    use tower_lsp::jsonrpc::Request as RpcRequest;
    use tower_lsp::ClientSocket;

    const VALID: &str = "def id : Prop -> Prop := fun (x : Prop) => x\n";
    const EXERCISE: &str = "example : Prop -> Prop := sorry\n";
    const KERNEL_BAD: &str = "def bad : Prop -> Type := fun (x : Prop) => x\n";
    const PARSE_BAD: &str = "def broken : Prop :=\n";

    #[tokio::test]
    async fn initialize_advertises_core_capabilities() {
        let (mut service, _socket) = test_service();
        handshake(&mut service).await;
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn did_open_valid_file_publishes_no_diagnostics() {
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, VALID).await;
        let params = wait_diagnostics(&mut socket, "diagnostics after didOpen").await;
        assert_eq!(params.uri.as_str(), URI);
        assert!(
            params.diagnostics.is_empty(),
            "valid file must publish no diagnostics, got {:?}",
            params.diagnostics
        );
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn did_open_kernel_rejected_file_publishes_coded_diagnostic() {
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, KERNEL_BAD).await;
        let params = wait_diagnostics(&mut socket, "kernel diagnostics").await;
        assert_eq!(
            params.diagnostics.len(),
            1,
            "expected exactly one kernel rejection, got {:?}",
            params.diagnostics
        );
        let diag = &params.diagnostics[0];
        assert_eq!(code_of(diag), "kernel-rejected");
        assert!(
            diag.message.contains("提示："),
            "teaching hint expected in diagnostic message: {:?}",
            diag.message
        );
        assert_eq!(diag.severity, Some(DiagnosticSeverity::ERROR));
        assert_ne!(
            diag.range.start, diag.range.end,
            "kernel rejection must have a non-empty range"
        );
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn did_open_parse_error_publishes_parse_code() {
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, PARSE_BAD).await;
        let params = wait_diagnostics(&mut socket, "parse diagnostics").await;
        assert_eq!(
            params.diagnostics.len(),
            1,
            "expected exactly one parse error, got {:?}",
            params.diagnostics
        );
        let diag = &params.diagnostics[0];
        let code = code_of(diag);
        assert!(
            code == "unexpected-token" || code == "unexpected-eof",
            "expected a parse-stage code, got {code}"
        );
        assert_eq!(diag.severity, Some(DiagnosticSeverity::ERROR));
        assert_eq!(diag.source.as_deref(), Some("sokonanoda"));
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn hover_returns_inferred_type() {
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, VALID).await;
        let _ = wait_diagnostics(&mut socket, "didOpen diagnostics").await;

        // The `x` occurrence inside the lambda body (0-based position).
        let pos = lsp_pos(VALID, VALID.rfind('x').expect("body `x` exists"));
        let result = call(
            &mut service,
            RpcRequest::build("textDocument/hover")
                .params(json!({
                    "textDocument": {"uri": URI},
                    "position": position_json(pos),
                }))
                .id(2)
                .finish(),
        )
        .await
        .expect("hover must answer");
        let hover: Option<Hover> = serde_json::from_value(result).expect("valid Hover");
        let hover = hover.expect("hover must resolve inside the lambda body");
        let markup = match hover.contents {
            HoverContents::Markup(markup) => markup,
            other => panic!("expected markup contents, got {other:?}"),
        };
        assert_eq!(markup.kind, MarkupKind::Markdown);
        assert!(
            markup.value.contains("Prop"),
            "inferred type expected in hover markup: {:?}",
            markup.value
        );
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn hover_on_universe_applied_eq_prelude_constant_shows_signature() {
        // 用户原始症状（playground.sokonanoda:233）：hover `Eq.subst.{1}` 只显示
        // 源码切片。根因是内核 pp 对开项推断 panic、hover 文本被吞空；修复后
        // 必须显示 `Eq.subst.{1} : forall … p a …` 的完整签名。
        let src = concat!(
            "theorem eq_symm_nat : (a : Nat) -> (b : Nat) -> Eq.{1} Nat a b -> Eq.{1} Nat b a :=\n",
            "  fun (a : Nat) (b : Nat) (h : Eq.{1} Nat a b) =>\n",
            "    Eq.subst.{1} Nat (fun (x : Nat) => Eq.{1} Nat x a) a b h (Eq.refl.{1} Nat a)\n",
        );
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, src).await;
        let params = wait_diagnostics(&mut socket, "didOpen diagnostics").await;
        assert!(
            params.diagnostics.is_empty(),
            "valid theorem must publish no diagnostics: {:?}",
            params.diagnostics
        );
        let offset = offset_of(src, "Eq.subst.{1}");
        let result = call(
            &mut service,
            RpcRequest::build("textDocument/hover")
                .params(json!({
                    "textDocument": {"uri": URI},
                    "position": position_json(lsp_pos(src, offset)),
                }))
                .id(2)
                .finish(),
        )
        .await
        .expect("hover must answer");
        let hover: Option<Hover> = serde_json::from_value(result).expect("valid Hover");
        let hover = hover.expect("hover must resolve on `Eq.subst.{1}`");
        let HoverContents::Markup(markup) = hover.contents else {
            panic!("expected markup hover");
        };
        assert!(
            markup.value.contains("Eq.subst.{1} :"),
            "hover must show the signature, got: {:?}",
            markup.value
        );
        assert!(
            markup.value.contains("p a"),
            "hover signature must mention the dependent codomain, got: {:?}",
            markup.value
        );
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn hover_on_sorry_in_overapplied_spine_shows_hole_expected_type() {
        // 用户案例（playground 练习 5，0.25.0）：`(And.right a (Not a) x)
        // sorry` 的 hover 必须显示洞的精确期望类型 `a`（经 def `Not` 展开
        // `Not a` ⇒ `a -> False`），而不是整个声明类型。剩余目标 `False`。
        let src = "axiom False : Prop\n\
                   axiom And : Prop -> Prop -> Prop\n\
                   axiom And.right : (a : Prop) -> (b : Prop) -> And a b -> b\n\
                   def Not : Prop -> Prop := fun (a : Prop) => a -> False\n\
                   theorem and_not_absurd : (a : Prop) -> And a (Not a) -> False :=\n\
                     fun (a : Prop) => fun (x : And a (Not a)) => (And.right a (Not a) x) sorry\n";
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, src).await;
        let _ = wait_diagnostics(&mut socket, "didOpen diagnostics").await;

        let pos = lsp_pos(src, offset_of(src, "sorry") + 2);
        let result = call(
            &mut service,
            RpcRequest::build("textDocument/hover")
                .params(json!({
                    "textDocument": {"uri": URI},
                    "position": position_json(pos),
                }))
                .id(4)
                .finish(),
        )
        .await
        .expect("hover must answer");
        let hover: Option<Hover> = serde_json::from_value(result).expect("valid Hover");
        let hover = hover.expect("hover must resolve on the sorry");
        let markup = match hover.contents {
            HoverContents::Markup(markup) => markup,
            other => panic!("expected markup contents, got {other:?}"),
        };
        // 洞的精确期望类型（def 展开后的箭头定义域）。
        assert!(
            markup.value.contains("期望类型：") && markup.value.contains("```sokonanoda\na"),
            "hole expected type must be `a` in a sokonanoda fence: {:?}",
            markup.value
        );
        // 剩余目标 = 声明类型剥掉两层 lambda（goal 代码块内 `⊢ False`）。
        assert!(
            markup.value.contains("剩余目标：") && markup.value.contains("⊢ False"),
            "remaining goal must be `False`: {:?}",
            markup.value
        );
        // 上下文假设完整（goal 代码块内的 `x : And a (Not a)`）。
        assert!(
            markup.value.contains("x : And a (Not a)"),
            "{:?}",
            markup.value
        );
        // 不再把整个声明类型当目标展示。
        assert!(
            !markup.value.contains("目标：\n```sokonanoda\nforall"),
            "declared type must not be shown as the goal: {:?}",
            markup.value
        );
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn hover_on_hole_shows_goal() {
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, EXERCISE).await;
        let _ = wait_diagnostics(&mut socket, "didOpen diagnostics").await;

        let pos = lsp_pos(EXERCISE, offset_of(EXERCISE, "sorry") + 1);
        let result = call(
            &mut service,
            RpcRequest::build("textDocument/hover")
                .params(json!({
                    "textDocument": {"uri": URI},
                    "position": position_json(pos),
                }))
                .id(3)
                .finish(),
        )
        .await
        .expect("hover must answer");
        let hover: Option<Hover> = serde_json::from_value(result).expect("valid Hover");
        let hover = hover.expect("hover must resolve on the hole");
        let markup = match hover.contents {
            HoverContents::Markup(markup) => markup,
            other => panic!("expected markup contents, got {other:?}"),
        };
        assert!(
            markup.value.contains("目标"),
            "goal label expected in hover markup: {:?}",
            markup.value
        );
        assert!(
            markup.value.contains("Prop -> Prop"),
            "goal text expected in hover markup: {:?}",
            markup.value
        );
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn document_symbols_list_declarations() {
        let src = format!("{VALID}{EXERCISE}");
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, &src).await;
        let _ = wait_diagnostics(&mut socket, "didOpen diagnostics").await;

        let result = call(
            &mut service,
            RpcRequest::build("textDocument/documentSymbol")
                .params(json!({"textDocument": {"uri": URI}}))
                .id(4)
                .finish(),
        )
        .await
        .expect("documentSymbol must answer");
        let symbols: Option<DocumentSymbolResponse> =
            serde_json::from_value(result).expect("valid DocumentSymbolResponse");
        let DocumentSymbolResponse::Nested(symbols) =
            symbols.expect("document symbols must be returned")
        else {
            panic!("expected nested document symbols");
        };
        let id = symbols
            .iter()
            .find(|s| s.name == "id")
            .expect("symbol for `id`");
        assert!(
            id.detail.as_deref().unwrap_or_default().contains("solved"),
            "checked def detail should carry the status label, got {:?}",
            id.detail
        );
        let exercise = symbols
            .iter()
            .find(|s| s.name.starts_with("example"))
            .expect("symbol for the open example");
        assert!(
            exercise
                .detail
                .as_deref()
                .unwrap_or_default()
                .contains("exercise: open"),
            "example detail should carry the status label, got {:?}",
            exercise.detail
        );
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn code_lens_reflects_exercise_status() {
        let src = format!("def ok : Prop -> Prop := fun (x : Prop) => x\n{EXERCISE}");
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, &src).await;
        let _ = wait_diagnostics(&mut socket, "didOpen diagnostics").await;

        let result = call(
            &mut service,
            RpcRequest::build("textDocument/codeLens")
                .params(json!({"textDocument": {"uri": URI}}))
                .id(5)
                .finish(),
        )
        .await
        .expect("codeLens must answer");
        let lenses: Option<Vec<CodeLens>> = serde_json::from_value(result).expect("valid CodeLens");
        let lenses = lenses.expect("code lenses must be returned");
        assert_eq!(lenses.len(), 2, "one lens per declaration: {:?}", lenses);
        let titles: Vec<&str> = lenses
            .iter()
            .filter_map(|lens| lens.command.as_ref().map(|cmd| cmd.title.as_str()))
            .collect();
        assert!(
            titles.iter().any(|t| t.contains("solved")),
            "checked def lens should read solved, got {titles:?}"
        );
        assert!(
            titles.iter().any(|t| t.contains("exercise: open")),
            "open exercise lens should read open, got {titles:?}"
        );
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn code_lens_ranges_match_each_declaration() {
        // codeLens 的 range 必须精确覆盖每个声明（checked def 与 open
        // exercise 各一），命令 id 由扩展消费（sokonanoda.status）。
        const CHECKED: &str = "def ok : Prop -> Prop := fun (x : Prop) => x";
        const OPEN: &str = "example : Prop -> Prop := sorry";
        let src = format!("{CHECKED}\n{OPEN}\n");
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, &src).await;
        let _ = wait_diagnostics(&mut socket, "codeLens diagnostics").await;

        let result = call(
            &mut service,
            RpcRequest::build("textDocument/codeLens")
                .params(json!({"textDocument": {"uri": URI}}))
                .id(6)
                .finish(),
        )
        .await
        .expect("codeLens must answer");
        let lenses: Option<Vec<CodeLens>> = serde_json::from_value(result).expect("valid CodeLens");
        let lenses = lenses.expect("code lenses must be returned");
        assert_eq!(lenses.len(), 2, "one lens per declaration: {lenses:?}");

        let checked = &lenses[0];
        let command = checked.command.as_ref().expect("lens carries a command");
        assert_eq!(command.command, "sokonanoda.status");
        assert!(
            command.title.contains("solved"),
            "checked def lens title: {:?}",
            command.title
        );
        assert_eq!(
            checked.range.start,
            lsp_pos(&src, 0),
            "checked lens starts at the declaration"
        );
        assert_eq!(
            checked.range.end,
            lsp_pos(&src, CHECKED.len()),
            "checked lens ends at the declaration's last token"
        );

        let open_start = CHECKED.len() + 1;
        let open = &lenses[1];
        let command = open.command.as_ref().expect("lens carries a command");
        assert_eq!(command.command, "sokonanoda.status");
        assert!(
            command.title.contains("exercise: open"),
            "open exercise lens title: {:?}",
            command.title
        );
        assert_eq!(
            open.range.start,
            lsp_pos(&src, open_start),
            "open lens starts at the exercise declaration"
        );
        assert_eq!(
            open.range.end,
            lsp_pos(&src, open_start + OPEN.len()),
            "open lens ends at the exercise's last token"
        );
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn code_action_offers_intro_on_open_exercise() {
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, EXERCISE).await;
        let _ = wait_diagnostics(&mut socket, "didOpen diagnostics").await;

        let hole = offset_of(EXERCISE, "sorry");
        let hole_start = lsp_pos(EXERCISE, hole);
        let result = call(
            &mut service,
            RpcRequest::build("textDocument/codeAction")
                .params(json!({
                    "textDocument": {"uri": URI},
                    "range": {"start": position_json(hole_start), "end": position_json(hole_start)},
                    "context": {"diagnostics": []},
                }))
                .id(6)
                .finish(),
        )
        .await
        .expect("codeAction must answer");
        let actions: Option<CodeActionResponse> =
            serde_json::from_value(result).expect("valid CodeActionResponse");
        let actions = actions.expect("code actions must be returned");
        assert_eq!(
            actions.len(),
            1,
            "expected one intro quick-fix, got {:?}",
            actions
        );
        let action = match &actions[0] {
            CodeActionOrCommand::CodeAction(action) => action,
            other => panic!("expected a CodeAction, got {other:?}"),
        };
        assert_eq!(action.kind, Some(CodeActionKind::QUICKFIX));
        assert!(action.title.contains("引入"), "title: {:?}", action.title);
        let edit = action.edit.as_ref().expect("intro action carries an edit");
        let changes = edit.changes.as_ref().expect("changes map");
        let edits = changes
            .get(&Url::parse(URI).expect("test uri parses"))
            .expect("edit targets our uri");
        assert_eq!(edits.len(), 1);
        let text_edit = &edits[0];
        // The hole sits at 0-based (line, col); the edit must span exactly it.
        assert_eq!(
            text_edit.range.start, hole_start,
            "edit must start exactly at the hole, got {:?}",
            text_edit.range
        );
        assert_eq!(
            text_edit.range.end.character - text_edit.range.start.character,
            "sorry".len() as u32,
            "edit must span exactly the 3-char hole, got {:?}",
            text_edit.range
        );
        assert_eq!(
            text_edit.range.start.line, text_edit.range.end.line,
            "hole edit must stay on one line, got {:?}",
            text_edit.range
        );
        assert!(
            text_edit.new_text.starts_with("fun ("),
            "intro replacement must start a lambda, got {:?}",
            text_edit.new_text
        );
        assert!(
            text_edit.new_text.ends_with("sorry"),
            "intro replacement must keep the hole, got {:?}",
            text_edit.new_text
        );
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn did_change_recomputes_diagnostics() {
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, VALID).await;
        let first = wait_diagnostics(&mut socket, "initial diagnostics").await;
        assert!(first.diagnostics.is_empty());

        notify(
            &mut service,
            "textDocument/didChange",
            json!({
                "textDocument": {"uri": URI, "version": 2},
                "contentChanges": [{"text": KERNEL_BAD}],
            }),
        )
        .await;
        let second = wait_diagnostics(&mut socket, "diagnostics after didChange").await;
        assert_eq!(
            second.diagnostics.len(),
            1,
            "edited file must be re-checked, got {:?}",
            second.diagnostics
        );
        assert_eq!(code_of(&second.diagnostics[0]), "kernel-rejected");
        shutdown(&mut service).await;
    }

    const BARE_OK: &str =
        "-- sokonanoda:prelude none\ndef id : Prop -> Prop := fun (x : Prop) => x\n";
    const BARE_NAT: &str = "-- sokonanoda:prelude none\ndef two : Nat := 2\n";
    const FULL_NAT: &str = "def two : Nat := 2\n";

    #[tokio::test]
    async fn directive_bare_file_without_nat_checks_clean() {
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, BARE_OK).await;
        let params = wait_diagnostics(&mut socket, "bare ok diagnostics").await;
        assert!(
            params.diagnostics.is_empty(),
            "bare Prop-level file must be clean: {:?}",
            params.diagnostics
        );
    }

    #[tokio::test]
    async fn directive_bare_file_loses_nat() {
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, BARE_NAT).await;
        let params = wait_diagnostics(&mut socket, "bare nat diagnostics").await;
        assert!(
            params
                .diagnostics
                .iter()
                .any(|d| d.code == Some(NumberOrString::String("elab-unknown-identifier".into()))),
            "bare file must not know Nat: {:?}",
            params.diagnostics
        );
    }

    #[tokio::test]
    async fn full_mode_still_has_nat_without_directive() {
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, FULL_NAT).await;
        let params = wait_diagnostics(&mut socket, "full nat diagnostics").await;
        assert!(
            params.diagnostics.is_empty(),
            "Nat prelude must be present without the directive: {:?}",
            params.diagnostics
        );
    }

    // I9 goal 视图：hover 显示可用假设；assumption/exact code action。
    #[tokio::test]
    async fn hover_on_partial_hole_lists_hypotheses() {
        let src = "example : (a : Prop) -> a -> a := fun (a : Prop) => fun (h : a) => sorry\n";
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, src).await;
        let _ = wait_diagnostics(&mut socket, "partial hole diagnostics").await;

        let hole = offset_of(src, "sorry");
        let pos = lsp_pos(src, hole);
        let result = call(
            &mut service,
            RpcRequest::build("textDocument/hover")
                .params(json!({
                    "textDocument": {"uri": URI},
                    "position": position_json(pos),
                }))
                .id(20)
                .finish(),
        )
        .await
        .expect("hover must answer");
        let hover: Option<Hover> = serde_json::from_value(result).expect("valid hover");
        let hover = hover.expect("hover at the hole");
        let HoverContents::Markup(markup) = hover.contents else {
            panic!("expected markup hover");
        };
        assert!(
            markup.value.contains("目标：") && markup.value.contains("⊢ a"),
            "hover shows goal: {}",
            markup.value
        );
        assert!(
            markup.value.contains("a : Prop") && markup.value.contains("h : a"),
            "hover lists hypotheses: {}",
            markup.value
        );
    }

    #[tokio::test]
    async fn code_action_offers_exact_for_matching_hypothesis() {
        let src = "example : (a : Prop) -> a -> a := fun (a : Prop) => fun (h : a) => sorry\n";
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, src).await;
        let _ = wait_diagnostics(&mut socket, "partial hole diagnostics").await;

        let hole = offset_of(src, "sorry");
        let hole_start = lsp_pos(src, hole);
        let result = call(
            &mut service,
            RpcRequest::build("textDocument/codeAction")
                .params(json!({
                    "textDocument": {"uri": URI},
                    "range": {"start": position_json(hole_start), "end": position_json(hole_start)},
                    "context": {"diagnostics": []},
                }))
                .id(21)
                .finish(),
        )
        .await
        .expect("codeAction must answer");
        let actions: Option<CodeActionResponse> =
            serde_json::from_value(result).expect("valid CodeActionResponse");
        let actions = actions.expect("code actions for a closable goal");
        let exact = actions
            .iter()
            .find_map(|a| match a {
                CodeActionOrCommand::CodeAction(action) => {
                    if action.title.contains("exact h") {
                        Some(action)
                    } else {
                        None
                    }
                }
                CodeActionOrCommand::Command(_) => None,
            })
            .expect("an `exact h` action must be offered");
        let edit = exact.edit.as_ref().expect("exact action carries an edit");
        let changes = edit.changes.as_ref().expect("changes map");
        let edits = changes
            .get(&Url::parse(URI).expect("test uri parses"))
            .expect("edit targets our uri");
        assert_eq!(edits.len(), 1);
        assert_eq!(
            edits[0].new_text, "h",
            "exact fills the hole with the hypothesis"
        );
        assert_eq!(edits[0].range.start, hole_start, "edit targets the hole");
    }

    #[tokio::test]
    async fn code_action_intro_still_offered_without_matching_hypothesis() {
        let src = "example : Prop -> Prop := sorry\n";
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, src).await;
        let _ = wait_diagnostics(&mut socket, "hole diagnostics").await;

        let hole = offset_of(src, "sorry");
        let hole_start = lsp_pos(src, hole);
        let result = call(
            &mut service,
            RpcRequest::build("textDocument/codeAction")
                .params(json!({
                    "textDocument": {"uri": URI},
                    "range": {"start": position_json(hole_start), "end": position_json(hole_start)},
                    "context": {"diagnostics": []},
                }))
                .id(22)
                .finish(),
        )
        .await
        .expect("codeAction must answer");
        let actions: Option<CodeActionResponse> =
            serde_json::from_value(result).expect("valid CodeActionResponse");
        let actions = actions.expect("intro action without a matching hypothesis");
        let titles: Vec<&str> = actions
            .iter()
            .filter_map(|a| match a {
                CodeActionOrCommand::CodeAction(action) => Some(action.title.as_str()),
                CodeActionOrCommand::Command(_) => None,
            })
            .collect();
        assert!(
            titles.iter().any(|t| t.contains("引入")),
            "intro must still be offered: {titles:?}"
        );
        assert!(
            !titles.iter().any(|t| t.contains("exact")),
            "no exact action when no hypothesis matches: {titles:?}"
        );
        let _ = hole_start;
    }

    // F8 语义着色：能力 + UTF-16 编码 + 端到端分类。

    #[test]
    fn every_semantic_kind_maps_to_a_legend_entry() {
        // `token_type_index` has an `.expect`, but make the totality explicit:
        // every `SemanticKind::ALL` value resolves to an index inside the legend.
        let legend = semantic_token_types();
        for kind in SemanticKind::ALL {
            let index = token_type_index(*kind) as usize;
            assert!(
                index < legend.len(),
                "{kind:?} maps out of the legend (index {index}, len {})",
                legend.len()
            );
        }
    }

    /// 把相对 delta 编码还原成绝对 (line, start_utf16, length, token_type)。
    /// 解码逻辑独立实现（按 LSP 规范），用来交叉检验编码器。
    fn absolutize(tokens: &[SemanticToken]) -> Vec<(u32, u32, u32, SemanticTokenType)> {
        let legend = semantic_token_types();
        let mut out = Vec::new();
        let (mut line, mut start) = (0u32, 0u32);
        for t in tokens {
            line += t.delta_line;
            if t.delta_line == 0 {
                start += t.delta_start;
            } else {
                start = t.delta_start;
            }
            out.push((line, start, t.length, legend[t.token_type as usize].clone()));
        }
        out
    }

    async fn request_semantic_tokens(service: &mut LspService<Backend>) -> Vec<SemanticToken> {
        let result = call(
            service,
            RpcRequest::build("textDocument/semanticTokens/full")
                .params(json!({"textDocument": {"uri": URI}}))
                .id(30)
                .finish(),
        )
        .await
        .expect("semanticTokens/full must answer");
        let result: Option<SemanticTokensResult> =
            serde_json::from_value(result).expect("valid SemanticTokensResult");
        match result.expect("tokens must be returned") {
            SemanticTokensResult::Tokens(tokens) => tokens.data,
            other => panic!("expected full tokens, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn semantic_tokens_full_classifies_def_example_hole() {
        let src = "def two : Nat := 2\nexample : Sort 1 := sorry\n";
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, src).await;
        let _ = wait_diagnostics(&mut socket, "semantic tokens diagnostics").await;

        let tokens = request_semantic_tokens(&mut service).await;
        assert_eq!(
            absolutize(&tokens),
            vec![
                (0, 0, 3, SemanticTokenType::KEYWORD),   // def
                (0, 4, 3, SemanticTokenType::FUNCTION),  // two（声明）
                (0, 10, 3, SemanticTokenType::VARIABLE), // Nat（未知标识符）
                (0, 17, 1, SemanticTokenType::NUMBER),   // 2
                (1, 0, 7, SemanticTokenType::KEYWORD),   // example（换行后绝对起点）
                (1, 10, 4, SemanticTokenType::TYPE),     // Sort
                (1, 15, 1, SemanticTokenType::NUMBER),   // 1
                (1, 20, 5, SemanticTokenType::MACRO),    // sorry（UTF-16 长度 3）
            ],
            "full token stream for {src:?}"
        );
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn semantic_tokens_full_handles_non_ascii_identifiers() {
        let src = "def α_id : Prop -> Prop := fun (x : Prop) => x\n";
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, src).await;
        let _ = wait_diagnostics(&mut socket, "semantic tokens diagnostics").await;

        let tokens = request_semantic_tokens(&mut service).await;
        assert_eq!(
            absolutize(&tokens),
            vec![
                (0, 0, 3, SemanticTokenType::KEYWORD),    // def
                (0, 4, 4, SemanticTokenType::FUNCTION),   // α_id（α 是 BMP，1 个 UTF-16 单元）
                (0, 11, 4, SemanticTokenType::TYPE),      // Prop
                (0, 19, 4, SemanticTokenType::TYPE),      // Prop
                (0, 27, 3, SemanticTokenType::KEYWORD),   // fun（按 UTF-16 是 27，按字节会是 28）
                (0, 32, 1, SemanticTokenType::PARAMETER), // x
                (0, 36, 4, SemanticTokenType::TYPE),      // Prop
                (0, 45, 1, SemanticTokenType::PARAMETER), // x（") => x"）
            ],
            "positions must be UTF-16 code units, not bytes/chars: {src:?}"
        );
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn semantic_tokens_highlight_axiom_connectives_as_types() {
        // And/Or 这类 axiom 是教学语言的逻辑类型/命题：声明与使用都映射到
        // TYPE（此前落到 VARIABLE，主题里几乎无色）。
        let src = "axiom And : Prop -> Prop -> Prop\n#check And\n";
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, src).await;
        let _ = wait_diagnostics(&mut socket, "semantic tokens diagnostics").await;

        let tokens = request_semantic_tokens(&mut service).await;
        assert_eq!(
            absolutize(&tokens),
            vec![
                (0, 0, 5, SemanticTokenType::KEYWORD), // axiom
                (0, 6, 3, SemanticTokenType::TYPE),    // And（声明 = AxiomName）
                (0, 12, 4, SemanticTokenType::TYPE),   // Prop
                (0, 20, 4, SemanticTokenType::TYPE),   // Prop
                (0, 28, 4, SemanticTokenType::TYPE),   // Prop
                (1, 0, 6, SemanticTokenType::KEYWORD), // #check
                (1, 7, 3, SemanticTokenType::TYPE),    // And（使用 = AxiomUse）
            ],
            "axiom connective names must be type-colored: {src:?}"
        );
        shutdown(&mut service).await;
    }

    #[test]
    fn encoder_counts_utf16_units_for_supplementary_identifiers() {
        // 🦀 是增补平面字符：1 char = 2 UTF-16 单元；按 char 计数会得到 9。
        let src = "def 🦀x : Prop := Prop\n";
        let spans = sokonanoda_front::semantic::semantic_tokens(src);
        let tokens = encode_semantic_tokens(src, &spans);
        assert_eq!(
            absolutize(&tokens),
            vec![
                (0, 0, 3, SemanticTokenType::KEYWORD),  // def
                (0, 4, 3, SemanticTokenType::FUNCTION), // 🦀x：起点 4，长度 2+1=3
                (0, 10, 4, SemanticTokenType::TYPE),    // Prop：UTF-16 绝对起点 10（char 会是 9）
                (0, 18, 4, SemanticTokenType::TYPE),    // Prop
            ],
        );
    }

    #[test]
    fn encoder_emits_nothing_for_untokenizable_text() {
        // 词法错误截断后仍产出已收集部分的 token；纯标点行不产出 token。
        let src = "-- 只有注释\n: :\n";
        let spans = sokonanoda_front::semantic::semantic_tokens(src);
        assert!(encode_semantic_tokens(src, &spans).is_empty());
        let empty: Vec<SemanticSpan> = Vec::new();
        assert!(encode_semantic_tokens("", &empty).is_empty());
    }

    // ---- I9 goal 视图协议：soko/goals 与 soko/nextHole ----

    async fn request_goals(service: &mut LspService<Backend>) -> serde_json::Value {
        call(
            service,
            RpcRequest::build("soko/goals")
                .params(json!({"textDocument": {"uri": URI}, "position": null}))
                .id(40)
                .finish(),
        )
        .await
        .expect("soko/goals must answer")
    }

    #[tokio::test]
    async fn goals_request_lists_open_exercise_with_hole_range() {
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, EXERCISE).await;
        let _ = wait_diagnostics(&mut socket, "goals diagnostics").await;

        let result = request_goals(&mut service).await;
        let decls = result
            .get("decls")
            .and_then(|d| d.as_array())
            .expect("goals response carries decls");
        assert_eq!(decls.len(), 1, "one open exercise: {result:?}");
        let decl = &decls[0];
        assert_eq!(decl["status"], "open");
        assert_eq!(decl["goal"], "Prop -> Prop");
        // The declaration's own type ships with the wire so the Infoview can
        // show it as a hint; runs reconstruct it exactly (§2.1).
        let ty = decl["ty"].as_str().expect("declared type");
        assert_eq!(ty, "Prop -> Prop", "declared type text");
        let runs = decl["ty_runs"].as_array().expect("ty_runs array");
        assert_eq!(
            runs.iter()
                .map(|r| r["text"].as_str().unwrap_or_default())
                .collect::<String>(),
            ty,
            "ty_runs reconstruct ty"
        );
        assert!(
            runs.iter().any(|r| r["kind"].as_str() == Some("sort")),
            "`Prop` in the type is a sort: {runs:?}"
        );
        let hole = decl["hole"].as_object().expect("hole range present");
        let start = hole["start"].as_object().expect("hole start");
        let expected = lsp_pos(EXERCISE, offset_of(EXERCISE, "sorry"));
        assert_eq!(start["line"], expected.line, "hole line (0-based)");
        assert_eq!(start["character"], expected.character, "hole character");
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn goals_request_lists_every_open_goal_after_apply() {
        // 练习面板的多目标：`soko/goals` 的每声明 `goals` 数组给出最后一步的
        // 全部未闭合目标（当前在前），非 by 练习回退为单个走查目标。
        let src = "axiom And : Prop -> Prop -> Prop\n\
                   axiom P : Prop\n\
                   axiom Q : Prop\n\
                   axiom And.intro : (a : Prop) -> (b : Prop) -> a -> b -> And a b\n\
                   theorem both : And P Q := by apply And.intro; sorry\n";
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, src).await;
        let _ = wait_diagnostics(&mut socket, "goals diagnostics").await;

        let result = request_goals(&mut service).await;
        let decl = result["decls"]
            .as_array()
            .expect("decls")
            .iter()
            .find(|d| d["name"] == "both")
            .expect("the `both` declaration");
        assert_eq!(decl["status"], "open");
        let goals = decl["goals"].as_array().expect("goals array");
        assert_eq!(
            goals,
            &vec![json!("P"), json!("Q")],
            "both goals, current first"
        );
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn next_hole_navigates_between_two_holes() {
        let src = format!("{EXERCISE}example : Prop := sorry\n");
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, &src).await;
        let _ = wait_diagnostics(&mut socket, "next hole diagnostics").await;

        // 从文件头向前：命中第一个洞。
        let first = ask_next_hole(
            &mut service,
            Position {
                line: 0,
                character: 0,
            },
            true,
        )
        .await;
        let first_range: Option<Range> = serde_json::from_value(first).expect("valid hole range");
        let first_range = first_range.expect("a hole ahead of (0,0)");
        assert_eq!(first_range.start.line, 0, "first hole is on line 0");
        // 从第一个洞再向前：命中第二个洞（line 1）。
        let second = ask_next_hole(&mut service, first_range.start, true).await;
        let second_range: Option<Range> = serde_json::from_value(second).expect("valid hole range");
        let second_range = second_range.expect("a second hole ahead of the first");
        assert_eq!(second_range.start.line, 1, "second hole is on line 1");
        // 从第二个洞向后：回到第一个洞。
        let back = ask_next_hole(&mut service, second_range.start, false).await;
        let back_range: Option<Range> = serde_json::from_value(back).expect("valid hole range");
        assert_eq!(
            back_range.expect("a hole behind").start.line,
            0,
            "backward navigation returns to the first hole"
        );
        shutdown(&mut service).await;
    }

    async fn ask_next_hole(
        service: &mut LspService<Backend>,
        pos: Position,
        forward: bool,
    ) -> serde_json::Value {
        call(
            service,
            RpcRequest::build("soko/nextHole")
                .params(json!({
                    "textDocument": {"uri": URI},
                    "position": position_json(pos),
                    "forward": forward,
                }))
                .id(41)
                .finish(),
        )
        .await
        .expect("soko/nextHole must answer")
    }

    // ---- soko/stateAt：光标处 tactic 目标（docs/design/by-tactics.md §6）----

    async fn ask_state_at(
        service: &mut LspService<Backend>,
        src: &str,
        offset: usize,
    ) -> serde_json::Value {
        call(
            service,
            RpcRequest::build("soko/stateAt")
                .params(json!({
                    "textDocument": {"uri": URI},
                    "position": position_json(lsp_pos(src, offset)),
                }))
                .id(43)
                .finish(),
        )
        .await
        .expect("soko/stateAt must answer")
    }

    const BY_OPEN: &str = "axiom And : Prop -> Prop -> Prop\n\
         theorem open : (a : Prop) -> And a a -> a := by intro a; intro h\n";

    #[tokio::test]
    async fn state_at_inside_a_tactic_shows_the_entering_state() {
        // 光标停在 tactic `intro h` 上：学习者要看到的是「这条 tactic 进来时的目标」。
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, BY_OPEN).await;
        let _ = wait_diagnostics(&mut socket, "stateAt diagnostics").await;

        let result = ask_state_at(&mut service, BY_OPEN, offset_of(BY_OPEN, "intro h")).await;
        assert_eq!(result["decl"]["name"], "open");
        assert_eq!(result["decl"]["kind"], "theorem");
        assert_eq!(result["decl"]["status"], "open");
        assert_eq!(
            result["step"], 0,
            "entering the second tactic = after step 0"
        );
        assert_eq!(result["total"], 2);
        assert_eq!(result["goal"], "And a a -> a");
        let binders = result["binders"].as_array().expect("binders array");
        assert_eq!(binders.len(), 1);
        assert_eq!(binders[0]["name"], "a");
        assert_eq!(binders[0]["ty"], "Prop");
        assert!(result["span"].is_object(), "highlight range present");
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn state_at_carries_semantic_runs_for_goals_and_hypotheses() {
        // docs/design/goal-rendering.md §2.1: `soko/stateAt` ships the same
        // classification the editor's semantic tokens use, so the Infoview can
        // colour identically instead of inventing its own rules.
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, BY_OPEN).await;
        let _ = wait_diagnostics(&mut socket, "stateAt diagnostics").await;

        let result = ask_state_at(&mut service, BY_OPEN, offset_of(BY_OPEN, "intro h")).await;
        let goal = result["goal"].as_str().expect("goal");
        assert_eq!(
            reconstruct_runs(&result["goal_runs"]),
            goal,
            "runs must reconstruct the goal text exactly"
        );
        let binder = &result["binders"][0];
        assert_eq!(
            reconstruct_runs(&binder["ty_runs"]),
            binder["ty"].as_str().expect("binder ty")
        );
        let goal_kinds = run_kinds(&result["goal_runs"]);
        assert!(
            goal_kinds.contains(&"binder"),
            "the hypothesis `a` must classify as a binder: {goal_kinds:?}"
        );
        assert!(
            goal_kinds.contains(&"axiom_use"),
            "`And` is the declared axiom: {goal_kinds:?}"
        );
        let binder_kinds = run_kinds(&binder["ty_runs"]);
        assert!(
            binder_kinds.contains(&"sort"),
            "the hypothesis type `Prop` is a sort: {binder_kinds:?}"
        );
        assert_eq!(
            result["goals"][0]["goal_runs"], result["goal_runs"],
            "single-value runs mirror goals[0]"
        );
        shutdown(&mut service).await;
    }

    fn reconstruct_runs(runs: &serde_json::Value) -> String {
        runs.as_array()
            .expect("runs array")
            .iter()
            .map(|run| run["text"].as_str().expect("run text"))
            .collect()
    }

    fn run_kinds(runs: &serde_json::Value) -> Vec<&str> {
        runs.as_array()
            .expect("runs array")
            .iter()
            .filter_map(|run| run["kind"].as_str())
            .collect()
    }

    #[tokio::test]
    async fn hover_goal_text_equals_the_state_at_run_projection() {
        // One content producer (docs/design/goal-rendering.md §2.1): the hover's
        // `sokonanoda` fence text must be the text projection of the very runs
        // `soko/stateAt` hands the Infoview (`goals[].binders[].ty_runs` +
        // `goals[].goal_runs`), so hover and Infoview can never drift.
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, BY_OPEN).await;
        let _ = wait_diagnostics(&mut socket, "stateAt diagnostics").await;

        let at = offset_of(BY_OPEN, "intro h");
        let state = ask_state_at(&mut service, BY_OPEN, at).await;
        let goals = state["goals"].as_array().expect("goals array");
        assert!(!goals.is_empty(), "entering `intro h` has an open goal");
        let mut projected = Vec::new();
        for goal in goals {
            let mut block = String::new();
            for binder in goal["binders"].as_array().expect("binders array") {
                block.push_str(binder["name"].as_str().expect("binder name"));
                block.push_str(" : ");
                block.push_str(&reconstruct_runs(&binder["ty_runs"]));
                block.push('\n');
            }
            block.push_str("⊢ ");
            block.push_str(&reconstruct_runs(&goal["goal_runs"]));
            projected.push(block);
        }
        let markup = hover_markup_at(&mut service, BY_OPEN, at).await;
        for block in &projected {
            assert!(
                markup.contains(block.as_str()),
                "hover must contain the stateAt projection:\nhover={markup:?}\nblock={block:?}"
            );
        }
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn state_at_after_the_last_tactic_shows_the_remaining_goal() {
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, BY_OPEN).await;
        let _ = wait_diagnostics(&mut socket, "stateAt diagnostics").await;

        let after = offset_of(BY_OPEN, "intro h") + "intro h".len();
        let result = ask_state_at(&mut service, BY_OPEN, after).await;
        assert_eq!(result["step"], 1);
        assert_eq!(result["goal"], "a");
        let binders = result["binders"].as_array().expect("binders array");
        assert_eq!(binders.len(), 2, "a and h are both in context");
        assert_eq!(binders[1]["name"], "h");
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn state_at_lists_all_open_goals_after_apply() {
        // `apply And.intro` 开出两个子目标；`soko/stateAt` 必须一次给出全部
        // （当前目标在首位），兼容单值字段仍等于 `goals[0]`。
        let src = "axiom And : Prop -> Prop -> Prop\n\
                   axiom P : Prop\n\
                   axiom Q : Prop\n\
                   axiom And.intro : (a : Prop) -> (b : Prop) -> a -> b -> And a b\n\
                   theorem both : And P Q := by apply And.intro; sorry\n";
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, src).await;
        let _ = wait_diagnostics(&mut socket, "stateAt diagnostics").await;

        // 光标停在 `sorry` 上：进入 sorry 的状态 = `apply` 执行后。
        let result = ask_state_at(&mut service, src, offset_of(src, "sorry")).await;
        assert_eq!(
            result["step"], 0,
            "entering the second tactic = after apply"
        );
        assert_eq!(result["total"], 2);
        let goals = result["goals"].as_array().expect("goals array");
        assert_eq!(goals.len(), 2, "both apply sub-goals are listed");
        assert_eq!(goals[0]["goal"], "P");
        assert_eq!(goals[1]["goal"], "Q");
        // 兼容单值字段 = 当前（首个）目标。
        assert_eq!(result["goal"], "P");
        assert_eq!(result["goal"], goals[0]["goal"]);
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn hover_on_a_tactic_shows_the_entering_goal_state() {
        // 用户需求：hover 每个 tactic → 中间 goal state（Lean Infoview 式）。
        // 进入某 tactic 的状态 = 上一步执行后（`soko/stateAt` 同一语义）。
        let src = "axiom And : Prop -> Prop -> Prop\n\
                   axiom And.intro : (a : Prop) -> (b : Prop) -> a -> b -> And a b\n\
                   axiom P : Prop\n\
                   axiom Q : Prop\n\
                   theorem both : And P Q := by apply And.intro; sorry\n";
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, src).await;
        let _ = wait_diagnostics(&mut socket, "tactic hover diagnostics").await;

        // hover `apply`：进入它时目标 = 根状态 `And P Q`。
        let at = offset_of(src, "apply And.intro");
        let hover = hover_opt_at(&mut service, src, at)
            .await
            .expect("hover on `apply` must answer");
        let HoverContents::Markup(markup) = hover.contents else {
            panic!("expected markup hover");
        };
        assert!(
            markup.value.contains("⊢ And P Q"),
            "tactic hover shows the entering goal: {:?}",
            markup.value
        );
        // Presentation: the tactic itself + `sokonanoda` code fences so both
        // the tactic and the goal state are syntax-highlighted.
        assert!(
            markup.value.contains("```sokonanoda\napply And.intro"),
            "hover header renders the tactic as a highlighted code block: {:?}",
            markup.value
        );
        assert!(
            markup.value.contains("```sokonanoda"),
            "hover goal state is a highlighted code fence: {:?}",
            markup.value
        );

        // hover `sorry`：进入它时有 apply 开出的两个子目标 P、Q。
        let at_sorry = offset_of(src, "sorry");
        let hover2 = hover_opt_at(&mut service, src, at_sorry)
            .await
            .expect("hover on `sorry` must answer");
        let HoverContents::Markup(m2) = hover2.contents else {
            panic!("expected markup hover");
        };
        assert!(m2.value.contains("⊢ P"), "hover: {:?}", m2.value);
        assert!(m2.value.contains("⊢ Q"), "hover: {:?}", m2.value);
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn state_at_on_the_by_keyword_returns_the_root_goal() {
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, BY_OPEN).await;
        let _ = wait_diagnostics(&mut socket, "stateAt diagnostics").await;

        let result = ask_state_at(&mut service, BY_OPEN, offset_of(BY_OPEN, "by intro")).await;
        assert_eq!(result["step"], -1, "before the first tactic = root state");
        assert_eq!(result["total"], 2);
        let goal = result["goal"].as_str().expect("root goal is the full type");
        assert!(
            goal.contains("And"),
            "root goal is the declared type: {goal}"
        );
        assert!(result["binders"]
            .as_array()
            .expect("binders array")
            .is_empty());
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn state_at_without_by_steps_returns_the_declaration_goal() {
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, EXERCISE).await;
        let _ = wait_diagnostics(&mut socket, "stateAt diagnostics").await;

        let result = ask_state_at(&mut service, EXERCISE, offset_of(EXERCISE, "sorry")).await;
        assert_eq!(result["decl"]["status"], "open");
        assert_eq!(result["goal"], "Prop -> Prop");
        assert_eq!(result["step"], -1);
        assert_eq!(result["total"], 0);
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn state_at_outside_any_declaration_is_empty() {
        let src = "-- 讲解注释\naxiom True : Prop\n";
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, src).await;
        let _ = wait_diagnostics(&mut socket, "stateAt diagnostics").await;

        let result = ask_state_at(&mut service, src, 0).await;
        assert!(result["decl"].is_null(), "no declaration at the cursor");
        assert!(result["goal"].is_null());
        assert!(result["span"].is_null());
        assert_eq!(result["step"], -1);
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn code_action_exact_uses_kernel_defeq_not_text_match() {
        // h 的类型是 `a -> False`，剩余目标渲染为 `Not a`：文本不同但
        // definitional equal —— 文本比对给不出建议，kernel 判定可以。
        let src = "axiom False : Prop\n\
                   def Not : Prop -> Prop := fun (a : Prop) => a -> False\n\
                   example : (a : Prop) -> (a -> False) -> Not a := fun (a : Prop) => fun (h : a -> False) => sorry\n";
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, src).await;
        let _ = wait_diagnostics(&mut socket, "defeq exact diagnostics").await;

        let hole = offset_of(src, "sorry");
        let hole_start = lsp_pos(src, hole);
        let result = call(
            &mut service,
            RpcRequest::build("textDocument/codeAction")
                .params(json!({
                    "textDocument": {"uri": URI},
                    "range": {"start": position_json(hole_start), "end": position_json(hole_start)},
                    "context": {"diagnostics": []},
                }))
                .id(42)
                .finish(),
        )
        .await
        .expect("codeAction must answer");
        let actions: Option<CodeActionResponse> =
            serde_json::from_value(result).expect("valid CodeActionResponse");
        let actions = actions.expect("kernel-defeq hypothesis must yield an action");
        let exact = actions
            .iter()
            .find_map(|a| match a {
                CodeActionOrCommand::CodeAction(action) => {
                    if action.title.contains("exact h") {
                        Some(action)
                    } else {
                        None
                    }
                }
                CodeActionOrCommand::Command(_) => None,
            })
            .expect("an `exact h` action must be offered (kernel judges a -> False ≡ Not a)");
        let edit = exact.edit.as_ref().expect("exact action carries an edit");
        let edits = edit
            .changes
            .as_ref()
            .and_then(|c| c.get(&Url::parse(URI).expect("uri")))
            .expect("edit targets our uri");
        assert_eq!(edits[0].new_text, "h");
    }

    // ---- I9 第二段：refine / 多洞 ----

    const AND_EXERCISE: &str = "axiom And : Prop -> Prop -> Prop\n\
axiom And.intro : (a : Prop) -> (b : Prop) -> a -> b -> And a b\n\
theorem and_intro_rule : (a : Prop) -> (b : Prop) -> a -> b -> And a b := sorry\n";

    const AND_MULTI_HOLE: &str = "axiom And : Prop -> Prop -> Prop\n\
axiom And.intro : (a : Prop) -> (b : Prop) -> a -> b -> And a b\n\
theorem and_intro_rule : (a : Prop) -> (b : Prop) -> a -> b -> And a b := \
fun (a : Prop) => fun (b : Prop) => fun (ha : a) => fun (hb : b) => And.intro sorry sorry\n";

    async fn code_actions_for(service: &mut LspService<Backend>, src: &str) -> Vec<CodeAction> {
        let hole = offset_of(src, "sorry");
        let hole_start = lsp_pos(src, hole);
        let result = call(
            service,
            RpcRequest::build("textDocument/codeAction")
                .params(json!({
                    "textDocument": {"uri": URI},
                    "range": {"start": position_json(hole_start), "end": position_json(hole_start)},
                    "context": {"diagnostics": []},
                }))
                .id(50)
                .finish(),
        )
        .await
        .expect("codeAction must answer");
        let actions: Option<CodeActionResponse> =
            serde_json::from_value(result).expect("valid CodeActionResponse");
        actions
            .expect("code actions")
            .into_iter()
            .filter_map(|a| match a {
                CodeActionOrCommand::CodeAction(action) => Some(action),
                CodeActionOrCommand::Command(_) => None,
            })
            .collect()
    }

    #[tokio::test]
    async fn code_action_offers_kernel_shaped_refine_skeleton() {
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, AND_EXERCISE).await;
        let _ = wait_diagnostics(&mut socket, "refine diagnostics").await;

        let actions = code_actions_for(&mut service, AND_EXERCISE).await;
        let refine = actions
            .iter()
            .find(|a| a.title.contains("refine And.intro a b sorry sorry"))
            .expect("refine skeleton with auto-filled parameters must be offered");
        let edit = refine.edit.as_ref().expect("refine carries an edit");
        let edits = edit
            .changes
            .as_ref()
            .and_then(|c| c.get(&Url::parse(URI).expect("uri")))
            .expect("edit targets our uri");
        assert_eq!(edits[0].new_text, "And.intro a b sorry sorry");
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn goals_request_carries_sub_holes_with_expected_types() {
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, AND_MULTI_HOLE).await;
        let _ = wait_diagnostics(&mut socket, "multi-hole diagnostics").await;

        let result = request_goals(&mut service).await;
        let decls = result["decls"].as_array().expect("decls array");
        let decl = &decls[decls.len() - 1];
        assert_eq!(decl["status"], "open");
        let holes = decl["holes"].as_array().expect("holes array");
        assert_eq!(holes.len(), 2, "two spine holes: {result:?}");
        assert!(
            holes[0]["range"].is_object(),
            "holes are {{range, id}} objects, not bare ranges: {result:?}"
        );
        assert_eq!(holes[0]["id"], "and_intro_rule:0", "named decl id form");
        assert_eq!(holes[1]["id"], "and_intro_rule:1");
        let sub_goals = decl["sub_goals"].as_array().expect("sub_goals array");
        assert_eq!(sub_goals.len(), 2);
        assert_eq!(
            sub_goals[0]["ty"], "a",
            "parameter hole expects the goal's own argument"
        );
        assert_eq!(sub_goals[1]["ty"], "b");
        assert_eq!(
            holes[0]["range"], sub_goals[0]["range"],
            "holes stay positionally aligned with sub_goals"
        );
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn goals_request_probes_preceding_hole_penetration() {
        // B′ 把「前置实参是洞」的子洞类型留成 null；请求期 kernel 探针把
        // 第一个洞提升为局部 `_h0 : Prop`，第二个洞（`Not a`）得 `Not _h0`。
        // wire 形状不变，只是 sub_goals[i].ty 从 null 变成文本。
        let src = "axiom False : Prop\n\
                   def Not : Prop -> Prop := fun (a : Prop) => a -> False\n\
                   axiom h : (a : Prop) -> Not a -> a\n\
                   theorem t : (a : Prop) -> Not a -> a :=\n\
                     fun (a : Prop) => fun (na : Not a) => h (sorry) (sorry)\n";
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, src).await;
        let _ = wait_diagnostics(&mut socket, "probe diagnostics").await;

        let result = request_goals(&mut service).await;
        let decls = result["decls"].as_array().expect("decls array");
        let decl = &decls[decls.len() - 1];
        assert_eq!(decl["status"], "open");
        let sub_goals = decl["sub_goals"].as_array().expect("sub_goals array");
        assert_eq!(sub_goals.len(), 2, "{result:?}");
        assert_eq!(sub_goals[0]["ty"], "Prop");
        assert_eq!(
            sub_goals[1]["ty"], "Not _h0",
            "the second hole's expected type comes from the kernel probe"
        );
        // 契约不变：客户端不得文本扫洞——id 仍在 holes 里，位置对齐。
        assert_eq!(decl["holes"][1]["id"], "t:1");
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn goals_request_probes_one_level_nested_hole() {
        // 一层嵌套洞 `h (g sorry)`：廉价 walk 给出内层 sorry 的精确 span，
        // 请求期探针填上 `g` 的定义域。
        let src = "axiom g : (a : Prop) -> Prop\n\
                   axiom h : (b : Prop) -> Prop\n\
                   theorem t : Prop := h (g sorry)\n";
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, src).await;
        let _ = wait_diagnostics(&mut socket, "nested probe diagnostics").await;

        let result = request_goals(&mut service).await;
        let decls = result["decls"].as_array().expect("decls array");
        let decl = &decls[decls.len() - 1];
        let holes = decl["holes"].as_array().expect("holes array");
        assert_eq!(holes.len(), 1, "the inner sorry is the hole: {result:?}");
        let sub_goals = decl["sub_goals"].as_array().expect("sub_goals array");
        assert_eq!(sub_goals[0]["ty"], "Prop");
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn goals_request_ids_holes_by_decl_and_order() {
        // id = "<declName>:<index>" (docs/protocol.md): named declarations use
        // their name; anonymous examples use the `example@<line>` name form
        // (render::decl_name); the index counts holes in offset order.
        let src = "theorem named : Prop := sorry\nexample : Prop := sorry\n";
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, src).await;
        let _ = wait_diagnostics(&mut socket, "hole id diagnostics").await;

        let result = request_goals(&mut service).await;
        let decls = result["decls"].as_array().expect("decls array");
        assert_eq!(decls.len(), 2, "one entry per declaration: {result:?}");
        let named = &decls[0];
        let holes = named["holes"].as_array().expect("holes array");
        assert_eq!(holes.len(), 1);
        assert_eq!(holes[0]["id"], "named:0", "named declaration id form");
        let hole_start = holes[0]["range"]["start"].as_object().expect("hole start");
        let expected = lsp_pos(src, offset_of(src, "sorry"));
        assert_eq!(hole_start["line"], expected.line);
        assert_eq!(hole_start["character"], expected.character);
        let anon = &decls[1];
        let holes = anon["holes"].as_array().expect("holes array");
        assert_eq!(anon["name"], "example@2", "anonymous example name form");
        assert_eq!(holes[0]["id"], "example@2:0", "anonymous example id form");
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn next_hole_traverses_sub_holes_within_one_declaration() {
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, AND_MULTI_HOLE).await;
        let _ = wait_diagnostics(&mut socket, "next hole diagnostics").await;

        let first = ask_next_hole(
            &mut service,
            Position {
                line: 0,
                character: 0,
            },
            true,
        )
        .await;
        let first_range: Option<Range> = serde_json::from_value(first).expect("range");
        let first_range = first_range.expect("first sub-hole");
        let second = ask_next_hole(&mut service, first_range.start, true).await;
        let second_range: Option<Range> = serde_json::from_value(second).expect("range");
        let second_range = second_range.expect("second sub-hole in the same declaration");
        assert_ne!(
            first_range.start, second_range.start,
            "the two sub-holes are distinct positions"
        );
        shutdown(&mut service).await;
    }

    // ---- 行业基线补全：completions / folding ----

    async fn request_completions(service: &mut LspService<Backend>) -> Vec<CompletionItem> {
        let result = call(
            service,
            RpcRequest::build("textDocument/completion")
                .params(json!({
                    "textDocument": {"uri": URI},
                    "position": {"line": 0, "character": 0},
                }))
                .id(60)
                .finish(),
        )
        .await
        .expect("completion must answer");
        let response: Option<CompletionResponse> =
            serde_json::from_value(result).expect("valid CompletionResponse");
        match response.expect("completions must be returned") {
            CompletionResponse::Array(items) => items,
            CompletionResponse::List(list) => list.items,
        }
    }

    #[test]
    fn code_fences_always_use_the_sokonanoda_language() {
        // docs/design/goal-rendering.md §7: one language id for every rendered
        // code block, so the single TM grammar colours all of them.
        assert_eq!(code_block("x : Nat"), "```sokonanoda\nx : Nat\n```");
        assert!(!code_block("x").contains("```text"));
    }

    #[tokio::test]
    async fn completion_lists_keywords_sorts_prelude_and_declarations() {
        let src = "def two : Nat := 2\nexample : Sort 1 := sorry\n";
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, src).await;
        let _ = wait_diagnostics(&mut socket, "completion diagnostics").await;

        let items = request_completions(&mut service).await;
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        for expected in [
            "def", "fun", "#check", "Prop", "Sort", "Nat", "Nat.add", "Eq.refl", "two",
        ] {
            assert!(
                labels.contains(&expected),
                "completion must list `{expected}`: {labels:?}"
            );
        }
        assert!(
            !labels.iter().any(|l| l.starts_with("_example")),
            "internal names must not be offered: {labels:?}"
        );
        let two = items.iter().find(|i| i.label == "two").expect("two");
        assert_eq!(two.kind, Some(CompletionItemKind::FUNCTION));
        assert!(
            two.detail.as_deref().is_some_and(|d| d.contains("def")),
            "detail carries kind/status: {:?}",
            two.detail
        );
        // 文档里的签名是 `sokonanoda` 代码块（与 hover 同一围栏语言，§7）。
        let documentation = match &two.documentation {
            Some(Documentation::MarkupContent(markup)) => markup.value.clone(),
            other => panic!("expected markdown documentation, got {other:?}"),
        };
        assert!(
            documentation.contains("```sokonanoda") && documentation.contains("def two : Nat"),
            "signature docs must be a sokonanoda fence: {documentation:?}"
        );
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn folding_ranges_cover_multiline_declarations_only() {
        let src = "def one : Nat :=\n  1\nexample : Sort 1 := sorry\n";
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, src).await;
        let _ = wait_diagnostics(&mut socket, "folding diagnostics").await;

        let result = call(
            &mut service,
            RpcRequest::build("textDocument/foldingRange")
                .params(json!({"textDocument": {"uri": URI}}))
                .id(61)
                .finish(),
        )
        .await
        .expect("foldingRange must answer");
        let ranges: Option<Vec<FoldingRange>> =
            serde_json::from_value(result).expect("valid FoldingRange");
        let ranges = ranges.expect("folding ranges");
        assert_eq!(
            ranges.len(),
            1,
            "only the two-line declaration folds: {ranges:?}"
        );
        assert_eq!(ranges[0].start_line, 0);
        assert_eq!(ranges[0].end_line, 1);
        shutdown(&mut service).await;
    }

    // ---- 导航基线：go-to-definition / documentHighlight / binder 补全 ----

    async fn goto_definition_at(
        service: &mut LspService<Backend>,
        src: &str,
        offset: usize,
    ) -> Option<GotoDefinitionResponse> {
        let result = call(
            service,
            RpcRequest::build("textDocument/definition")
                .params(json!({
                    "textDocument": {"uri": URI},
                    "position": position_json(lsp_pos(src, offset)),
                }))
                .id(70)
                .finish(),
        )
        .await
        .expect("textDocument/definition must answer");
        serde_json::from_value(result).expect("valid GotoDefinitionResponse")
    }

    #[tokio::test]
    async fn goto_definition_jumps_from_use_to_binder() {
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, VALID).await;
        let _ = wait_diagnostics(&mut socket, "goto def diagnostics").await;

        let body_x = VALID.rfind('x').expect("body `x` exists");
        let target = goto_definition_at(&mut service, VALID, body_x).await;
        let GotoDefinitionResponse::Scalar(location) =
            target.expect("binder use must resolve to a definition")
        else {
            panic!("expected a scalar definition location");
        };
        let binder_at = VALID.find("(x : Prop)").expect("binder text exists");
        assert_eq!(location.range.start, lsp_pos(VALID, binder_at));
        assert_eq!(
            location.range.end,
            lsp_pos(VALID, binder_at + "(x : Prop)".len())
        );
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn goto_definition_jumps_from_check_use_to_declaration() {
        let src = format!("{VALID}#check id\n");
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, &src).await;
        let _ = wait_diagnostics(&mut socket, "goto def decl diagnostics").await;

        let use_id = src.find("#check id").expect("#check id exists") + "#check ".len();
        let target = goto_definition_at(&mut service, &src, use_id).await;
        let GotoDefinitionResponse::Scalar(location) =
            target.expect("top-level use must resolve to a definition")
        else {
            panic!("expected a scalar definition location");
        };
        let decl_end = "def id : Prop -> Prop := fun (x : Prop) => x".len();
        assert_eq!(location.range.start, lsp_pos(&src, 0));
        assert_eq!(location.range.end, lsp_pos(&src, decl_end));
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn goto_definition_returns_none_without_resolution() {
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, VALID).await;
        let _ = wait_diagnostics(&mut socket, "goto def none diagnostics").await;

        let target = goto_definition_at(&mut service, VALID, offset_of(VALID, "Prop")).await;
        assert!(
            target.is_none(),
            "prelude names have no source definition: {target:?}"
        );
        shutdown(&mut service).await;
    }

    async fn document_highlight_at(
        service: &mut LspService<Backend>,
        src: &str,
        offset: usize,
    ) -> Option<Vec<DocumentHighlight>> {
        let result = call(
            service,
            RpcRequest::build("textDocument/documentHighlight")
                .params(json!({
                    "textDocument": {"uri": URI},
                    "position": position_json(lsp_pos(src, offset)),
                }))
                .id(71)
                .finish(),
        )
        .await
        .expect("textDocument/documentHighlight must answer");
        serde_json::from_value(result).expect("valid document highlight response")
    }

    #[tokio::test]
    async fn document_highlight_lists_all_uses_of_one_definition() {
        let src = "def double : Nat -> Nat := fun (n : Nat) => n + n\n";
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, src).await;
        let _ = wait_diagnostics(&mut socket, "highlight diagnostics").await;

        let body = src.find("n + n").expect("body uses exist");
        let expected = vec![lsp_pos(src, body), lsp_pos(src, body + 4)];

        // 从使用点请求：该定义的所有使用点都高亮。
        let highlights = document_highlight_at(&mut service, src, body)
            .await
            .expect("uses of `n` must highlight");
        assert_eq!(highlights.len(), 2, "both `n` uses: {highlights:?}");
        let starts: Vec<Position> = highlights.iter().map(|h| h.range.start).collect();
        assert_eq!(starts, expected);
        assert!(
            highlights
                .iter()
                .all(|h| h.kind == Some(DocumentHighlightKind::TEXT)),
            "highlights are text-level: {highlights:?}"
        );

        // 从 binder 定义处请求：同样高亮全部使用点。
        let binder = src.find("(n : Nat)").expect("binder exists");
        let highlights = document_highlight_at(&mut service, src, binder)
            .await
            .expect("highlighting the binder finds its uses");
        let starts: Vec<Position> = highlights.iter().map(|h| h.range.start).collect();
        assert_eq!(starts, expected, "binder highlight lists all uses");
        shutdown(&mut service).await;
    }

    async fn request_completions_at(
        service: &mut LspService<Backend>,
        pos: Position,
    ) -> Vec<CompletionItem> {
        let result = call(
            service,
            RpcRequest::build("textDocument/completion")
                .params(json!({
                    "textDocument": {"uri": URI},
                    "position": position_json(pos),
                }))
                .id(72)
                .finish(),
        )
        .await
        .expect("completion must answer");
        let response: Option<CompletionResponse> =
            serde_json::from_value(result).expect("valid CompletionResponse");
        match response.expect("completions must be returned") {
            CompletionResponse::Array(items) => items,
            CompletionResponse::List(list) => list.items,
        }
    }

    #[tokio::test]
    async fn completion_offers_in_scope_binders() {
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, VALID).await;
        let _ = wait_diagnostics(&mut socket, "completion binder diagnostics").await;

        let pos = lsp_pos(VALID, VALID.rfind('x').expect("body `x` exists"));
        let items = request_completions_at(&mut service, pos).await;
        let binder = items
            .iter()
            .find(|i| i.label == "x")
            .expect("in-scope binder `x` must be offered");
        assert_eq!(binder.kind, Some(CompletionItemKind::VARIABLE));
        assert!(
            binder
                .detail
                .as_deref()
                .is_some_and(|d| d.contains("binder")),
            "detail marks the binder scope: {:?}",
            binder.detail
        );
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn hover_on_a_half_expression_shows_the_remaining_goals() {
        // 用户需求：半截表达式（`And.intro b a` 还差两个前提）的 hover 不只给
        // 报错——把推断出的剩余目标列成 `⊢ b`、`⊢ a`。按需计算 + judge 缓存，
        // 不在按键路径上。
        let src = "axiom And : Prop -> Prop -> Prop\n\
                   axiom And.intro : (a : Prop) -> (b : Prop) -> a -> b -> And a b\n\
                   theorem and_swap : (a : Prop) -> (b : Prop) -> And a b -> And b a :=\n\
                     fun (a : Prop) => fun (b : Prop) => fun (x : And a b) => And.intro b a\n";
        let (mut service, _socket) = open_and_wait(src).await;
        let at = src.rfind("And.intro").expect("value occurrence");
        let hover = hover_opt_at(&mut service, src, at)
            .await
            .expect("hover on the half expression must answer");
        let HoverContents::Markup(markup) = hover.contents else {
            panic!("expected markup hover");
        };
        assert!(
            markup.value.contains("还差 2 个前提"),
            "hover: {:?}",
            markup.value
        );
        assert!(markup.value.contains("⊢ b"), "hover: {:?}", markup.value);
        assert!(markup.value.contains("⊢ a"), "hover: {:?}", markup.value);
        assert!(
            markup.value.contains("```sokonanoda"),
            "half-expression goals use the highlighted fence: {:?}",
            markup.value
        );
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn funapply_expansion_is_not_offered_outside_its_own_line() {
        // 与 `intro` 同一条边界：跨行不命中，免得在后面的声明上误弹。
        let src = "axiom P : Prop\naxiom Q : Prop\ntheorem t (h : Q -> P) : P :=\n  funapply h\n\ntheorem u : P := h sorry\n";
        let (mut service, _socket) = open_and_wait(src).await;
        // 光标落在下下个声明上：不该再给 `funapply` 的展开项。
        let items =
            request_completions_at(&mut service, lsp_pos(src, offset_of(src, "h sorry"))).await;
        assert!(
            items
                .iter()
                .all(|i| i.filter_text.as_deref() != Some("funapply")),
            "no funapply expansion on a later declaration: {items:?}"
        );
        shutdown(&mut service).await;
    }

    // ── 性能测试（I13-S5c）：阈值断言抓交互级回归 ──────────────────

    /// 生成 N 条声明 + 1 条 open 练习的画布源码。
    fn perf_canvas(n: usize) -> String {
        let mut lines = vec!["axiom P : Prop".to_string(), "axiom proofP : P".to_string()];
        for i in 1..=n {
            lines.push(format!("theorem solved_{i} : P := proofP"));
        }
        lines.push("theorem exercise : P := sorry".to_string());
        lines.join("\n") + "\n"
    }

    #[tokio::test]
    async fn perf_did_change_latency() {
        // didChange → 诊断落地的每次延迟 < 50ms（50 声明文件）。
        // 抓的是「编辑→反馈」的用户可感延迟。
        let src = perf_canvas(50);
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, &src).await;
        let _ = wait_diagnostics(&mut socket, "perf: initial").await;

        let mut cur = src.clone();
        let mut version = 1i32;
        // 编辑最后一条 open 练习的值（模拟学习者在文件末尾做题）
        let old_line = "theorem exercise : P := sorry";
        let new_line = "theorem exercise : P := proofP";
        let offset = cur.find(old_line).expect("exercise line");
        let step: TypedStep = (offset, old_line.len(), new_line);
        type_step(&mut service, &mut socket, &mut cur, &mut version, step).await;

        let start = std::time::Instant::now();
        // 再触发一次编辑（恢复原文→再编辑），量测 round-trip
        let step_back: TypedStep = (offset, new_line.len(), old_line);
        type_step(&mut service, &mut socket, &mut cur, &mut version, step_back).await;
        let elapsed = start.elapsed().as_millis();
        println!("PERF lsp didChange round-trip: {elapsed}ms (threshold 50ms)");
        assert!(
            elapsed < 50,
            "didChange round-trip took {elapsed}ms (threshold 50ms)"
        );
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn perf_completion_and_hover_latency() {
        // completion + hover 请求延迟 < 10ms（50 声明文件）。
        let src = perf_canvas(50);
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, &src).await;
        let _ = wait_diagnostics(&mut socket, "perf: initial").await;

        let at = offset_of(&src, "sorry");
        // completion
        let start = std::time::Instant::now();
        let _ = request_completions_at(&mut service, lsp_pos(&src, at)).await;
        let c_ms = start.elapsed().as_millis();
        println!("PERF lsp completion: {c_ms}ms (threshold 10ms)");
        assert!(c_ms < 10, "completion took {c_ms}ms (threshold 10ms)");
        // hover
        let start = std::time::Instant::now();
        let _ = hover_opt_at(&mut service, &src, at).await;
        let h_ms = start.elapsed().as_millis();
        println!("PERF lsp hover: {h_ms}ms (threshold 10ms)");
        assert!(h_ms < 10, "hover took {h_ms}ms (threshold 10ms)");
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn perf_state_at_latency() {
        // 光标移动路径 `soko/stateAt`（0.40.0 起带 goal_runs/ty_runs）延迟
        // < 10ms（50 声明文件）——防止「每次移动都全量重解析」之类的回归。
        let src = perf_canvas(50);
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, &src).await;
        let _ = wait_diagnostics(&mut socket, "perf: initial").await;

        let at = offset_of(&src, "sorry");
        let start = std::time::Instant::now();
        let result = ask_state_at(&mut service, &src, at).await;
        let elapsed = start.elapsed().as_millis();
        assert!(
            result.get("goals").and_then(|g| g.as_array()).is_some(),
            "stateAt response shape: {result:?}"
        );
        println!("PERF lsp stateAt: {elapsed}ms (threshold 10ms)");
        assert!(
            elapsed < 10,
            "soko/stateAt took {elapsed}ms (threshold 10ms)"
        );
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn perf_goals_view_latency() {
        // goal 视图（soko/goals，教学核心特性）延迟 < 10ms（50 声明文件）。
        let src = perf_canvas(50);
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, &src).await;
        let _ = wait_diagnostics(&mut socket, "perf: initial").await;

        let start = std::time::Instant::now();
        let result = call(
            &mut service,
            tower_lsp::jsonrpc::Request::build("soko/goals")
                .params(serde_json::json!({
                    "textDocument": {"uri": "file:///perf.sokonanoda"},
                    "position": null
                }))
                .id(90)
                .finish(),
        )
        .await
        .expect("soko/goals must answer");
        let elapsed = start.elapsed().as_millis();
        // 响应必须真的带 decls（防止测了个错误响应）
        assert!(
            result.get("decls").and_then(|d| d.as_array()).is_some(),
            "goals response shape: {result:?}"
        );
        println!("PERF lsp goals view: {elapsed}ms (threshold 10ms)");
        assert!(elapsed < 10, "soko/goals took {elapsed}ms (threshold 10ms)");
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn version_request_reports_version_and_pid() {
        // `sokonanoda: restart server` 在重启前后各问一次 soko/version：
        // 旧 pid 消失 + 新 pid 出现 + 版本号变化，把「旧进程退出、新进程是
        // 新版本」变成可验证的事实。
        let (mut service, _socket) = open_and_wait(EXERCISE).await;
        let result = call(
            &mut service,
            RpcRequest::build("soko/version")
                .params(json!({}))
                .id(91)
                .finish(),
        )
        .await
        .expect("soko/version must answer");
        assert_eq!(result["version"], env!("CARGO_PKG_VERSION"));
        let pid = result["pid"].as_u64().expect("pid is a number");
        assert!(pid > 0, "a real process id: {result:?}");
        shutdown(&mut service).await;
    }

    // ---- 优先级可视化：selectionRange（学习者需求）----

    const DEMO_K: &str =
        "theorem demo_K : (a : Prop) -> a -> a :=\n  fun (a : Prop) => fun (h : a) => h\n";

    async fn selection_range_at(
        service: &mut LspService<Backend>,
        src: &str,
        offset: usize,
    ) -> Option<SelectionRange> {
        let pos = lsp_pos(src, offset);
        let result = call(
            service,
            RpcRequest::build("textDocument/selectionRange")
                .params(json!({
                    "textDocument": {"uri": URI},
                    "positions": [position_json(pos)],
                }))
                .id(70)
                .finish(),
        )
        .await
        .expect("selectionRange must answer");
        let response: Option<Vec<SelectionRange>> =
            serde_json::from_value(result).expect("valid SelectionRange");
        response.expect("array").into_iter().next()
    }

    #[tokio::test]
    async fn selection_range_grows_from_arrow_to_enclosing_type() {
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, DEMO_K).await;
        let _ = wait_diagnostics(&mut socket, "selection range diagnostics").await;

        // 光标放在类型里第二个箭头 `a -> a` 的 `->` 上（该段先结合）。
        let arrow_at = DEMO_K.find("a -> a").expect("inner arrow exists") + 2;
        let inner = selection_range_at(&mut service, DEMO_K, arrow_at)
            .await
            .expect("arrow position must yield a chain");
        // 第一级：正好是 `a -> a`（内层函数类型，先结合）。
        let start_off = DEMO_K.find("a -> a").expect("span start");
        assert_eq!(
            inner.range.start,
            lsp_pos(DEMO_K, start_off),
            "innermost selection must be the arrow expression itself"
        );
        assert_eq!(inner.range.end, lsp_pos(DEMO_K, start_off + "a -> a".len()));
        // 更大的层级存在，且逐级包住内层。
        let mut cur = &inner;
        let mut levels = 1usize;
        while let Some(parent) = cur.parent.as_ref() {
            assert!(
                parent.range.start < inner.range.start,
                "each level must start at-or-before the inner one: {:?} vs {:?}",
                parent.range.start,
                inner.range.start
            );
            levels += 1;
            cur = parent;
        }
        assert!(
            levels >= 2,
            "chain must reach the enclosing type, got {levels}"
        );
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn hover_on_keyword_returns_none() {
        // 学习者反馈：光标在 fun/=>/theorem 上应该安静，而不是把某个
        // 节点的类型行硬塞过来。
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, VALID).await;
        let _ = wait_diagnostics(&mut socket, "keyword hover diagnostics").await;

        let fun_at = VALID.find("fun").expect("fun exists");
        let pos = lsp_pos(VALID, fun_at);
        let result = call(
            &mut service,
            RpcRequest::build("textDocument/hover")
                .params(json!({
                    "textDocument": {"uri": URI},
                    "position": position_json(pos),
                }))
                .id(80)
                .finish(),
        )
        .await
        .expect("hover must answer");
        let hover: Option<Hover> = serde_json::from_value(result).expect("valid hover");
        assert!(hover.is_none(), "keyword hover must be silent");
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn sorry_produces_warning_not_error() {
        // Lean 4 对齐：含 sorry 的声明产出 warning（不是 error），
        // 让学习者知道"文件编译但有缺口"。
        let src = "theorem t : True := sorry\n";
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, src).await;
        let diags = wait_diagnostics(&mut socket, "sorry warning").await;
        let sorry = diags
            .diagnostics
            .iter()
            .find(|d| d.code == Some(NumberOrString::String("sorry".to_string())))
            .expect("sorry warning must exist");
        assert_eq!(sorry.severity, Some(DiagnosticSeverity::WARNING));
        assert!(
            sorry.message.contains("uses 'sorry'"),
            "{:?}",
            sorry.message
        );
    }

    #[tokio::test]
    async fn non_sorry_errors_are_not_warnings() {
        // 非 sorry 的 kernel 拒绝仍然是 error（不被 sorry warning 稀释）。
        let src = "def bad : Prop -> Type := fun (x : Prop) => x\n";
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, src).await;
        let diags = wait_diagnostics(&mut socket, "non-sorry diagnostics").await;
        assert!(diags
            .diagnostics
            .iter()
            .any(|d| d.severity == Some(DiagnosticSeverity::ERROR)));
        assert!(!diags
            .diagnostics
            .iter()
            .any(|d| d.code == Some(NumberOrString::String("sorry".to_string()))));
    }

    #[tokio::test]
    async fn reserved_declaration_name_is_a_warning_not_an_error() {
        // `axiom Prop : Sort 1` 能通过内核，但这个名字永不被引用（`Prop`
        // 内核已经定义过，代码里的 `Prop` 都指内核那个）——编辑器给出
        // WARNING 级提示。
        let src = "axiom Prop : Sort 1\n";
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, src).await;
        let diags = wait_diagnostics(&mut socket, "reserved name warning").await;
        let warning = diags
            .diagnostics
            .iter()
            .find(|d| {
                d.code
                    == Some(NumberOrString::String(
                        "reserved-declaration-name".to_string(),
                    ))
            })
            .expect("reserved-declaration-name warning must exist");
        assert_eq!(warning.severity, Some(DiagnosticSeverity::WARNING));
        assert!(
            warning.message.contains("内核已经定义过了"),
            "warning carries the teaching message: {:?}",
            warning.message
        );
        assert!(
            !diags
                .diagnostics
                .iter()
                .any(|d| d.severity == Some(DiagnosticSeverity::ERROR)),
            "the declaration itself must not produce an error: {:?}",
            diags.diagnostics
        );
    }

    #[tokio::test]
    async fn hover_on_bracket_shows_enclosing_expression_type() {
        // 学习者需求：光标在括号上能看到内容。`(a : Prop)` 这种 binder
        // 标注组没有单一表达式行，回退显示组内最大的行（`Prop : Type 0`）；
        // 真正的表达式组 `(And.right a (Not a) h)` 由专门的括号测试覆盖。
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, DEMO_K).await;
        let _ = wait_diagnostics(&mut socket, "bracket hover diagnostics").await;

        // 光标放在 `(a : Prop)` 的 `(` 上（第 0 行 offset 17）。
        let paren = DEMO_K.find('(').expect("paren exists");
        let pos = lsp_pos(DEMO_K, paren);
        let result = call(
            &mut service,
            RpcRequest::build("textDocument/hover")
                .params(json!({
                    "textDocument": {"uri": URI},
                    "position": position_json(pos),
                }))
                .id(90)
                .finish(),
        )
        .await
        .expect("hover must answer");
        let hover: Option<Hover> = serde_json::from_value(result).expect("valid hover");
        let hover = hover.expect("bracket position must have hover (fallback)");
        let HoverContents::Markup(m) = hover.contents else {
            panic!("expected markup");
        };
        assert!(
            m.value.contains("Prop"),
            "bracket hover should show a type from the group: {:?}",
            m.value
        );
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn hover_on_operator_shows_enclosing_type() {
        // 光标在 `->` 上（第 0 行 offset 28）→ 应显示内层函数类型。
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, DEMO_K).await;
        let _ = wait_diagnostics(&mut socket, "operator hover diagnostics").await;

        let arrow = DEMO_K.find("->").expect("arrow exists");
        let pos = lsp_pos(DEMO_K, arrow);
        let result = call(
            &mut service,
            RpcRequest::build("textDocument/hover")
                .params(json!({
                    "textDocument": {"uri": URI},
                    "position": position_json(pos),
                }))
                .id(91)
                .finish(),
        )
        .await
        .expect("hover must answer");
        let hover: Option<Hover> = serde_json::from_value(result).expect("valid hover");
        let hover = hover.expect("operator position must have hover (proximity fallback)");
        let HoverContents::Markup(m) = hover.contents else {
            panic!("expected markup");
        };
        assert!(
            m.value.contains("->") || m.value.contains("Prop"),
            "operator hover should show type: {:?}",
            m.value
        );
        shutdown(&mut service).await;
    }

    // ---- 括号 hover：`(表达式)` 的 ( 与 ) 都显示 `表达式 : 类型` ----

    /// 用户指定的括号 hover 语料（and_not_absurd：括号应用 + Not 展开 +
    /// `$N` 还原三个痛点都在这一行里）。
    const AND_NOT_ABSURD: &str = concat!(
        "axiom False : Prop\n",
        "axiom And : Prop -> Prop -> Prop\n",
        "axiom And.intro : (a : Prop) -> (b : Prop) -> a -> b -> And a b\n",
        "axiom And.left : (a : Prop) -> (b : Prop) -> And a b -> a\n",
        "axiom And.right : (a : Prop) -> (b : Prop) -> And a b -> b\n",
        "def Not : Prop -> Prop := fun (a : Prop) => a -> False\n",
        "theorem and_not_absurd : (a : Prop) -> And a (Not a) -> False :=\n",
        "  fun (a : Prop) (h : And a (Not a)) => ",
        "(And.right a (Not a) h) (And.left a (Not a) h)\n",
    );

    async fn hover_markup_at(
        service: &mut LspService<Backend>,
        src: &str,
        offset: usize,
    ) -> String {
        hover_markup_opt_at(service, src, offset)
            .await
            .expect("position must have hover")
    }

    async fn hover_markup_opt_at(
        service: &mut LspService<Backend>,
        src: &str,
        offset: usize,
    ) -> Option<String> {
        let pos = lsp_pos(src, offset);
        let result = call(
            service,
            RpcRequest::build("textDocument/hover")
                .params(json!({
                    "textDocument": {"uri": URI},
                    "position": position_json(pos),
                }))
                .id(95)
                .finish(),
        )
        .await
        .expect("hover must answer");
        let hover: Option<Hover> = serde_json::from_value(result).expect("valid hover");
        hover.map(|h| match h.contents {
            HoverContents::Markup(m) => m.value,
            other => panic!("expected markup hover, got {other:?}"),
        })
    }

    /// 组内配对 `)` 的偏移：`(And.right a (Not a) h)` 里第一个 ` h)` 的 `)`。
    fn group_close(src: &str, group_open: usize) -> usize {
        group_open + src[group_open..].find(" h)").expect("group closing paren") + 2
    }

    /// The full `Hover` (contents + range) at a byte offset, for range assertions.
    async fn hover_opt_at(
        service: &mut LspService<Backend>,
        src: &str,
        offset: usize,
    ) -> Option<Hover> {
        let pos = lsp_pos(src, offset);
        let result = call(
            service,
            RpcRequest::build("textDocument/hover")
                .params(json!({
                    "textDocument": {"uri": URI},
                    "position": position_json(pos),
                }))
                .id(96)
                .finish(),
        )
        .await
        .expect("hover must answer");
        serde_json::from_value(result).expect("valid hover")
    }

    async fn open_and_wait(src: &str) -> (LspService<Backend>, ClientSocket) {
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, src).await;
        let _ = wait_diagnostics(&mut socket, "bracket suite diagnostics").await;
        (service, socket)
    }

    #[tokio::test]
    async fn hover_on_brackets_of_and_right_group_shows_step_type() {
        // 用户样例 1：`(And.right a (Not a) h)` 的 `(` 与 `)` 都显示
        // `And.right a (Not a) h : Not a`——`Not` 保持折叠、`a` 是真名。
        let (mut service, _socket) = open_and_wait(AND_NOT_ABSURD).await;
        let group = AND_NOT_ABSURD.find("(And.right").expect("group exists");
        let close = group_close(AND_NOT_ABSURD, group);
        for offset in [group, close] {
            let markup = hover_markup_at(&mut service, AND_NOT_ABSURD, offset).await;
            assert!(
                markup.contains("And.right a (Not a) h : Not a"),
                "bracket {offset} must show the step type: {markup:?}"
            );
        }
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn hover_on_brackets_of_and_left_group_shows_step_type() {
        // 用户样例 2：`(And.left a (Not a) h)` 的括号显示
        // `And.left a (Not a) h : a`。
        let (mut service, _socket) = open_and_wait(AND_NOT_ABSURD).await;
        let group = AND_NOT_ABSURD.find("(And.left").expect("group exists");
        let close = group_close(AND_NOT_ABSURD, group);
        for offset in [group, close] {
            let markup = hover_markup_at(&mut service, AND_NOT_ABSURD, offset).await;
            assert!(
                markup.contains("And.left a (Not a) h : a"),
                "bracket {offset} must show the step type: {markup:?}"
            );
        }
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn hover_on_closing_bracket_never_shows_neighbor_signature() {
        // 回归：`)` 上曾因「起点 ±2」邻近回退命中右侧邻居，显示
        // `And.left : forall …`。现在必须显示本组内容。
        let (mut service, _socket) = open_and_wait(AND_NOT_ABSURD).await;
        let group = AND_NOT_ABSURD.find("(And.right").expect("group exists");
        let close = group_close(AND_NOT_ABSURD, group);
        let markup = hover_markup_at(&mut service, AND_NOT_ABSURD, close).await;
        assert!(
            !markup.contains("And.left :"),
            "closing bracket must not leak the neighbor's signature: {markup:?}"
        );
        assert!(
            markup.contains("And.right a (Not a) h"),
            "closing bracket must show its own group: {markup:?}"
        );
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn hover_on_nested_parens_shows_innermost_expression() {
        // `((p))` 的四个括号都显示最内层表达式 `p : P`（括号透明）。
        let src = "axiom P : Prop\naxiom p : P\ntheorem t : P := ((p))\n";
        let (mut service, _socket) = open_and_wait(src).await;
        let outer_open = src.find("((").expect("outer paren exists");
        let inner_open = outer_open + 1;
        let inner_close = outer_open + 3;
        let outer_close = outer_open + 4;
        for offset in [outer_open, inner_open, inner_close, outer_close] {
            let markup = hover_markup_at(&mut service, src, offset).await;
            assert!(
                markup.contains("p : P"),
                "bracket {offset} must show the innermost expression: {markup:?}"
            );
        }
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn hover_on_bracket_inside_comment_falls_back() {
        // 注释里的括号没有配对语义——不得 panic，也不得张冠李戴；
        // 注释位置安静（None）或给出邻近的合理类型都算正确。
        let src = "axiom P : Prop\naxiom p : P\n-- (unbalanced (note)\ntheorem t : P := p\n";
        let (mut service, _socket) = open_and_wait(src).await;
        let comment_paren = src.find("(note").expect("comment paren exists");
        let markup = hover_markup_opt_at(&mut service, src, comment_paren).await;
        match markup {
            None => {}
            Some(m) => assert!(
                m.contains("p : P") || m.contains("Prop"),
                "comment bracket falls back to a sane hover: {m:?}"
            ),
        }
        shutdown(&mut service).await;
    }

    // ---- 重构后：binder 名不再整段 lambda 溢出；binder 标注组展示声明；
    //      所有 hover 都带高亮范围（range 覆盖光标） ----

    #[tokio::test]
    async fn hover_on_binder_name_shows_its_type_not_the_lambda() {
        // hover 到 binder 名字 `a`：显示 `a : Prop`，绝不吐整段 lambda。
        let src = "axiom True : Prop\n\
                   def f : Prop -> Prop := fun (a : Prop) => a\n";
        let (mut service, _socket) = open_and_wait(src).await;
        let a_name = src.find("(a").expect("binder") + 1;
        let markup = hover_markup_at(&mut service, src, a_name).await;
        assert!(
            markup.contains("a : Prop"),
            "hovering the binder name must show its type: {markup:?}"
        );
        assert!(
            !markup.contains("fun (a : Prop) => a"),
            "must not spill the whole lambda: {markup:?}"
        );
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn hover_on_binder_name_h_shows_declaration() {
        // hover 到 `h` 的 binder 名字：`h : And a (Not a)`（binder 声明）。
        let (mut service, _socket) = open_and_wait(AND_NOT_ABSURD).await;
        let h_name = AND_NOT_ABSURD.find("(h").expect("binder h") + 1;
        let markup = hover_markup_at(&mut service, AND_NOT_ABSURD, h_name).await;
        assert!(
            markup.contains("h : And a (Not a)"),
            "binder name must show its declaration: {markup:?}"
        );
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn hover_on_binder_annotation_bracket_shows_declaration() {
        // `(h : And a (Not a))` 的 `(` / `)`：显示 `h : And a (Not a)`
        //（不再截断成 `And a (Not a : Prop`）。
        let (mut service, _socket) = open_and_wait(AND_NOT_ABSURD).await;
        let group_open = AND_NOT_ABSURD.find("(h : And a (Not a))").expect("group");
        let group_close = group_open + "(h : And a (Not a))".len() - 1;
        for offset in [group_open, group_close] {
            let markup = hover_markup_at(&mut service, AND_NOT_ABSURD, offset).await;
            assert!(
                markup.contains("h : And a (Not a)"),
                "bracket {offset} must show the declaration: {markup:?}"
            );
            assert!(
                !markup.contains("And a (Not a :"),
                "must not be truncated: {markup:?}"
            );
        }
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn hover_on_binder_annotation_prop_shows_declaration() {
        // `(a : Prop)` 的 `(`：显示 `a : Prop`。
        let src = "axiom True : Prop\n\
                   def f : Prop -> Prop := fun (a : Prop) => a\n";
        let (mut service, _socket) = open_and_wait(src).await;
        let group_open = src.find("(a : Prop)").expect("group");
        let markup = hover_markup_at(&mut service, src, group_open).await;
        assert!(
            markup.contains("a : Prop"),
            "binder annotation bracket must show the declaration: {markup:?}"
        );
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn hover_returns_range_highlighting_the_expression() {
        // 括号 hover 与普通表达式 hover 都返回非空 range，且 range 覆盖光标。
        let (mut service, _socket) = open_and_wait(AND_NOT_ABSURD).await;
        // 括号：`(And.right a (Not a) h)` 的 `(`。
        let group = AND_NOT_ABSURD.find("(And.right").expect("group");
        let hov = hover_opt_at(&mut service, AND_NOT_ABSURD, group)
            .await
            .expect("bracket hover");
        let range = hov.range.expect("bracket hover must carry a range");
        let cursor = lsp_pos(AND_NOT_ABSURD, group);
        assert!(
            range.start <= cursor && cursor <= range.end,
            "bracket range must contain the cursor: {range:?}"
        );
        // 普通表达式：hover 定理体内的 `And.right`（`(And.right` 之后那个）。
        let and_right = AND_NOT_ABSURD.find("(And.right").expect("group") + 1;
        let hov2 = hover_opt_at(&mut service, AND_NOT_ABSURD, and_right)
            .await
            .expect("expression hover");
        let range2 = hov2.range.expect("expression hover must carry a range");
        let cursor2 = lsp_pos(AND_NOT_ABSURD, and_right);
        assert!(
            range2.start <= cursor2 && cursor2 <= range2.end,
            "expression range must contain the cursor: {range2:?}"
        );
        shutdown(&mut service).await;
    }
}

#[cfg(test)]
mod by_sorry_range_tests {
    use crate::testutil::{did_open, handshake, shutdown, test_service, wait_diagnostics};
    use sokonanoda_front::parse;
    use tower_lsp::lsp_types::NumberOrString;

    #[tokio::test]
    async fn by_sorry_warning_does_not_swallow_following_comments() {
        // 回归：`:= by sorry` 的声明 span 曾取「最后一个 tactic 之后的下一个
        // token 起点」——注释被词法器跳过，span 一路跨到下一个 theorem/EOF，
        // 把中间的多行注释整个包进黄色波浪线。现在 span 止于最后一个 tactic。
        let src = "axiom And : Prop -> Prop -> Prop\n\
                   theorem a : (a : Prop) -> And a a -> a := by sorry\n\
                   -- 练习 14 注释\n\
                   -- soko:hint 思路：funapply\n\
                   theorem b : (a : Prop) -> (b : Prop) -> a -> Or a b := by sorry\n";
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, src).await;
        let params = wait_diagnostics(&mut socket, "w").await;
        let sorry_warnings: Vec<_> = params
            .diagnostics
            .iter()
            .filter(|d| matches!(&d.code, Some(NumberOrString::String(c)) if c == "sorry"))
            .collect();
        assert_eq!(sorry_warnings.len(), 2, "two by-sorry declarations warn");
        for d in sorry_warnings {
            assert_eq!(
                d.range.start.line, d.range.end.line,
                "warning must be a single line (declaration), not swallow comments: {d:?}"
            );
            // 注释在第 2/3 行；声明在第 1 行与第 4 行，警告绝不能越到注释行。
            assert!(
                d.range.start.line != 2 && d.range.start.line != 3,
                "warning leaked onto a comment line: {d:?}"
            );
        }
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn by_sorry_hole_points_at_sorry_token() {
        // 回归：by 引擎降级出的尾部 Hole 曾用 Span::default()（offset 0），
        // 洞位/跳洞全错位。现在止于 `sorry` tactic 的 span。
        let src = "axiom And : Prop -> Prop -> Prop\n\
                   theorem a : (a : Prop) -> And a a -> a := by sorry\n";
        let file = parse(src).unwrap();
        let report = sokonanoda_front::compile::check_document(&file);
        let d = report
            .decls
            .iter()
            .find(|d| d.name.as_deref() == Some("a"))
            .unwrap();
        assert_eq!(d.status, crate::DeclStatus::Open);
        assert!(!d.holes.is_empty(), "open by-sorry must carry a hole");
        let hole = &d.holes[0];
        let expected = src.find("sorry").unwrap();
        assert_eq!(
            hole.start.offset, expected,
            "hole must be at the `sorry` token, not offset 0"
        );
    }
}

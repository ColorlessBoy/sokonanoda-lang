//! `sokonanoda-lsp`: a language server for `.sokonanoda` teaching files.
//!
//! Feedback philosophy (docs/design-infrastructure.md):
//! - diagnostics per declaration with stable codes and teaching hints;
//! - hover shows the inferred type of the expression under the cursor
//!   (from the front-end type map) or the goal of an open exercise `sorry`;
//! - document symbols / code lenses expose exercise state
//!   (open / solved / failed);
//! - code actions turn the first proof step into text and are ordered by
//!   kernel-verified first (see docs/design-hints-suggestions.md);
//! - rename / references / inlay hints follow LSP 3.17
//!   (docs/design-rename-inlay.md).
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
    decl_at, decl_name, definition_at, diagnostic_from_compile, diagnostic_from_parse,
    highlight_uses, hover_type_at, range_of, scope_names_at, semantic_kind_at, status_label,
    symbol_kind,
};
use serde::{Deserialize, Serialize};
use sokonanoda_front::compile::{
    prelude_mode_from_source, CompileOptions, DeclStatus, DocumentReport, HoverType, PreludeMode,
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
        SemanticKind::AxiomName | SemanticKind::AxiomUse | SemanticKind::UnknownIdent => {
            SemanticTokenType::VARIABLE
        }
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
            let lsp_version = version.unwrap_or(0).max(0) as u64;
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
                    // A report carries the same errors as its decl states;
                    // emit them once per failing declaration.
                    let mut diagnostics: Vec<_> = update
                        .report
                        .errors
                        .iter()
                        .map(diagnostic_from_compile)
                        .collect();
                    // Lean 4 对齐：含 sorry 的声明产出 warning（不是 error），
                    // 让学习者看到"文件编译但有缺口"。
                    if update
                        .report
                        .decls
                        .iter()
                        .any(|d| d.status == DeclStatus::Open && !d.holes.is_empty())
                    {
                        for d in update
                            .report
                            .decls
                            .iter()
                            .filter(|d| d.status == DeclStatus::Open)
                        {
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
                    doc.report = Some(update.report);
                    diagnostics
                }
            }
        };
        let _ = self
            .client
            .publish_diagnostics(uri, diagnostics, version)
            .await;
    }

    // ---- I9 goal 视图协议：结构化 goal 请求（coq-lsp `proof/goals` 模式）----

    fn goal_decls(&self) -> Option<(String, Vec<GoalDeclInfo>)> {
        let doc = self.doc.lock().expect("doc lock");
        let report = doc.report.as_ref()?;
        let decls = report
            .decls
            .iter()
            .map(|d| {
                let name = decl_name(d);
                GoalDeclInfo {
                    name: name.clone(),
                    kind: d.kind.as_str().to_string(),
                    status: match d.status {
                        DeclStatus::Open => "open".to_string(),
                        DeclStatus::Checked => "checked".to_string(),
                        DeclStatus::Failed => "failed".to_string(),
                    },
                    range: range_of(d.span),
                    goal: d.goal.clone(),
                    binders: d
                        .binders
                        .iter()
                        .map(|b| GoalBinderInfo {
                            name: b.name.clone(),
                            ty: b.ty.clone(),
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
            .goal_decls()
            .map(|(_, decls)| decls)
            .unwrap_or_default();
        Ok(GoalsResponse { decls })
    }

    async fn next_hole(&self, params: NextHoleParams) -> Result<Option<Range>> {
        let Some((text, decls)) = self.goal_decls() else {
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

    /// Hint ladder for the declaration at the cursor (docs/design-hints-
    /// suggestions.md). Stateless: the client owns progressive disclosure.
    async fn hints(&self, params: hints::HintsParams) -> Result<hints::HintsResponse> {
        let doc = self.doc.lock().expect("doc lock");
        Ok(hints::hints_for(&doc, params))
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
struct GoalBinderInfo {
    name: String,
    ty: String,
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
    goal: Option<String>,
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
        let Some(report) = &doc.report else {
            return Ok(None);
        };
        let pos = params.text_document_position_params.position;
        // 关键字（fun/=>/theorem/axiom…）上不吐类型行：那一行的悬停信息
        // 应该来自名字/表达式，而不是把关键字所在的某个节点硬塞过来。
        if let Some(kind) = semantic_kind_at(&doc.text, pos.line, pos.character) {
            if matches!(kind, SemanticKind::Keyword) {
                return Ok(None);
            }
        }
        let offset = position_to_offset(&doc.text, pos);
        if let Some(h) = hover_type_at(&report.hovers, pos.line, pos.character) {
            // 学习者需求：显示「表达式 : 类型」——表达式从源码按 span 切片。
            // type 为空时（infer panic 降级）只显示表达式本身。
            let end = h.span.end.offset.max(h.span.start.offset + 1);
            let expr = &doc.text[h.span.start.offset..end.min(doc.text.len())];
            let content = if h.text.is_empty() {
                expr.trim().to_string()
            } else {
                format!("{} : {}", expr.trim(), h.text)
            };
            return Ok(Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: format!("```text\n{}\n```", content),
                }),
                range: None,
            }));
        }
        // 回退：光标 ±2 字符内命中的最小外层表达式（括号、运算符等
        // 结构符号也能看到所属类型）。数据来自 hover 表（span 嵌套）。
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
                let expr = &doc.text[h.span.start.offset..h.span.end.offset.min(doc.text.len())];
                return Ok(Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: format!("```text\n{} : {}\n```", expr.trim(), h.text),
                    }),
                    range: None,
                }));
            }
        }
        if let Some(d) = decl_at(&report.decls, pos.line, pos.character) {
            let signature = match &d.ty_text {
                Some(ty) => format!("`{} {} : {}`\n", d.kind.as_str(), decl_name(d), ty),
                None => format!("**{} {}**\n", d.kind.as_str(), decl_name(d)),
            };
            let value = match d.status {
                DeclStatus::Open => {
                    let mut text = match &d.goal {
                        Some(goal) => format!("{}目标：`{}`\n", signature, goal),
                        None => format!("{}待作答\n", signature),
                    };
                    if !d.binders.is_empty() {
                        text.push_str("\n已引入假设：\n");
                        for b in &d.binders {
                            text.push_str(&format!("- `{}` : `{}`\n", b.name, b.ty));
                        }
                    }
                    text.push_str("\n在 `sorry` 处填写一个类型为目标的项。");
                    text
                }
                DeclStatus::Checked => {
                    format!("{}已通过内核检查", signature)
                }
                DeclStatus::Failed => {
                    format!("{}未通过，见诊断", signature)
                }
            };
            return Ok(Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value,
                }),
                range: None,
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
        let Some(report) = &doc.report else {
            return Ok(None);
        };
        let _ = params.range;
        Ok(Some(inlay::hole_hints(&doc.text, report)))
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
        .finish();
    Server::new(stdin, stdout, socket).serve(service).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{
        call, code_of, did_open, handshake, lsp_pos, notify, offset_of, position_json, shutdown,
        test_service, wait_diagnostics, URI,
    };
    use serde_json::json;
    use tower_lsp::jsonrpc::Request as RpcRequest;

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
        assert!(action.title.contains("intro"), "title: {:?}", action.title);
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
            markup.value.contains("目标：`a`"),
            "hover shows goal: {}",
            markup.value
        );
        assert!(
            markup.value.contains("`a` : `Prop`") && markup.value.contains("`h` : `a`"),
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
            titles.iter().any(|t| t.contains("intro")),
            "intro must still be offered: {titles:?}"
        );
        assert!(
            !titles.iter().any(|t| t.contains("exact")),
            "no exact action when no hypothesis matches: {titles:?}"
        );
        let _ = hole_start;
    }

    // F8 语义着色：能力 + UTF-16 编码 + 端到端分类。

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
        let hole = decl["hole"].as_object().expect("hole range present");
        let start = hole["start"].as_object().expect("hole start");
        let expected = lsp_pos(EXERCISE, offset_of(EXERCISE, "sorry"));
        assert_eq!(start["line"], expected.line, "hole line (0-based)");
        assert_eq!(start["character"], expected.character, "hole character");
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
            other => panic!("expected an array completion response, got {other:?}"),
        }
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
            other => panic!("expected an array completion response, got {other:?}"),
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
    async fn hover_on_bracket_shows_enclosing_expression_type() {
        // 学习者需求：光标在括号/运算符上能看到所属表达式的类型
        // （此前括号位置 hover 返回 None，因为括号不在任何子表达式 span 内）。
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
            m.value.contains("->"),
            "bracket hover should show enclosing expression type: {:?}",
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
}

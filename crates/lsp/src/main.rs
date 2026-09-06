//! `sokonanoda-lsp`: a language server for `.sokonanoda` teaching files.
//!
//! Feedback philosophy (docs/design-infrastructure.md):
//! - diagnostics per declaration with stable codes and teaching hints;
//! - hover shows the inferred type of the expression under the cursor
//!   (from the front-end type map) or the goal of an open exercise `???`;
//! - document symbols / code lenses expose exercise state
//!   (open / solved / failed);
//! - code actions turn the first step of a proof into text
//!   (`intro` shows that tactics only build a lambda).

use sokonanoda_front::compile::{
    check_document, DeclKind, DeclState, DeclStatus, DocumentReport, HoverType,
};
use sokonanoda_front::parse;
use sokonanoda_front::Span;
use std::collections::HashMap;
use std::sync::Mutex;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

#[derive(Default)]
struct Doc {
    uri: Option<Url>,
    text: String,
    report: Option<DocumentReport>,
    parse_error: Option<sokonanoda_front::Diagnostic>,
}

struct Backend {
    client: Client,
    doc: Mutex<Doc>,
}

impl Backend {
    fn new(client: Client) -> Self {
        Self {
            client,
            doc: Mutex::new(Doc::default()),
        }
    }

    async fn refresh(&self, uri: Url, text: String) {
        let (doc, diagnostics) = {
            let mut doc = Doc {
                uri: Some(uri.clone()),
                text: text.clone(),
                ..Doc::default()
            };
            let mut diagnostics = Vec::new();
            match parse(&text) {
                Ok(file) => {
                    let report = check_document(&file);
                    // A report carries the same errors as its decl states; emit
                    // them once per failing declaration.
                    for err in &report.errors {
                        diagnostics.push(diagnostic_from_compile(err));
                    }
                    doc.report = Some(report);
                }
                Err(diag) => {
                    diagnostics.push(diagnostic_from_parse(&diag));
                    doc.parse_error = Some(diag);
                }
            }
            (doc, diagnostics)
        };
        *self.doc.lock().expect("doc lock") = doc;
        let _ = self
            .client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }
}

fn diagnostic_from_compile(err: &sokonanoda_front::compile::CompileError) -> Diagnostic {
    Diagnostic {
        range: range_of(err.span),
        severity: Some(DiagnosticSeverity::ERROR),
        code: Some(NumberOrString::String(err.code().to_string())),
        code_description: None,
        source: Some("sokonanoda".to_string()),
        message: format!("{}\n\n提示：{}", err.message, err.hint()),
        related_information: None,
        tags: None,
        data: None,
    }
}

fn diagnostic_from_parse(diag: &sokonanoda_front::Diagnostic) -> Diagnostic {
    Diagnostic {
        range: range_of(diag.span),
        severity: Some(DiagnosticSeverity::ERROR),
        code: Some(NumberOrString::String(diag.code().to_string())),
        code_description: None,
        source: Some("sokonanoda".to_string()),
        message: format!("{}\n\n提示：{}", diag.message, diag.hint()),
        related_information: None,
        tags: None,
        data: None,
    }
}

fn range_of(span: Span) -> Range {
    Range {
        start: Position {
            line: span.start.line.saturating_sub(1) as u32,
            character: span.start.column.saturating_sub(1) as u32,
        },
        end: Position {
            line: span.end.line.saturating_sub(1) as u32,
            character: span.end.column.saturating_sub(1) as u32,
        },
    }
}

fn pos_within_span(line: u32, character: u32, span: Span) -> bool {
    let l = line as usize + 1;
    let c = character as usize + 1;
    let after_start = (l, c) > (span.start.line, span.start.column)
        || (l, c) >= (span.start.line, span.start.column);
    let before_end = (l, c) < (span.end.line, span.end.column);
    after_start && before_end
}

fn hover_type_at<'a>(hovers: &'a [HoverType], line: u32, character: u32) -> Option<&'a HoverType> {
    // smallest span containing the position wins
    hovers
        .iter()
        .filter(|h| pos_within_span(line, character, h.span))
        .min_by_key(|h| {
            (h.span.end.offset - h.span.start.offset)
                .try_into()
                .unwrap_or(u64::MAX)
        })
}

fn decl_at<'a>(decls: &'a [DeclState], line: u32, character: u32) -> Option<&'a DeclState> {
    decls
        .iter()
        .find(|d| pos_within_span(line, character, d.span))
}

fn symbol_kind(kind: DeclKind) -> SymbolKind {
    match kind {
        DeclKind::Definition => SymbolKind::FUNCTION,
        DeclKind::Theorem => SymbolKind::KEY,
        DeclKind::Axiom => SymbolKind::PROPERTY,
        DeclKind::Inductive => SymbolKind::STRUCT,
        DeclKind::Example => SymbolKind::CONSTANT,
    }
}

fn status_label(status: DeclStatus) -> &'static str {
    match status {
        DeclStatus::Open => "exercise: open",
        DeclStatus::Checked => "solved ✓",
        DeclStatus::Failed => "failed",
    }
}

fn decl_name(d: &DeclState) -> String {
    match &d.name {
        Some(n) => n.clone(),
        None => format!("{}@{}", d.kind.as_str(), d.span.start.line),
    }
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
                document_symbol_provider: Some(OneOf::Left(true)),
                code_lens_provider: Some(CodeLensOptions {
                    resolve_provider: Some(false),
                }),
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
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
        self.refresh(params.text_document.uri, params.text_document.text)
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.into_iter().next() {
            self.refresh(params.text_document.uri, change.text).await;
        }
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        if let Some(text) = params.text {
            self.refresh(params.text_document.uri, text).await;
        }
    }

    async fn did_close(&self, _: DidCloseTextDocumentParams) {}

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let doc = self.doc.lock().expect("doc lock");
        let Some(report) = &doc.report else {
            return Ok(None);
        };
        let pos = params.text_document_position_params.position;
        if let Some(h) = hover_type_at(&report.hovers, pos.line, pos.character) {
            return Ok(Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: format!("```text\n{}\n```", h.text),
                }),
                range: None,
            }));
        }
        if let Some(d) = decl_at(&report.decls, pos.line, pos.character) {
            let value = match d.status {
                DeclStatus::Open => match &d.goal {
                    Some(goal) => format!(
                        "**{} {}** — 目标：`{}`\n\n在 `???` 处填写一个类型为目标的项。",
                        d.kind.as_str(),
                        decl_name(d),
                        goal
                    ),
                    None => format!("**{} {}** — 待作答", d.kind.as_str(), decl_name(d)),
                },
                DeclStatus::Checked => {
                    format!("**{} {}** — 已通过内核检查", d.kind.as_str(), decl_name(d))
                }
                DeclStatus::Failed => {
                    format!("**{} {}** — 未通过，见诊断", d.kind.as_str(), decl_name(d))
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

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let doc = self.doc.lock().expect("doc lock");
        let Some(report) = &doc.report else {
            return Ok(None);
        };
        let pos = params.range.start;
        let Some(d) = decl_at(&report.decls, pos.line, pos.character) else {
            return Ok(None);
        };
        if d.status != DeclStatus::Open {
            return Ok(None);
        }
        let Some(goal_text) = &d.goal else {
            return Ok(None);
        };
        let Ok(goal_expr) = sokonanoda_front::proof::parse_expr_text(goal_text) else {
            return Ok(None);
        };
        let intros = match &goal_expr {
            sokonanoda_front::Expr::Forall { binders, .. } => binders.len(),
            sokonanoda_front::Expr::Arrow { .. } => 1,
            _ => 0,
        };
        if intros == 0 {
            return Ok(None);
        }
        let mut actions: Vec<CodeActionOrCommand> = Vec::new();
        if let Some(edit) = intro_edit(params.text_document.uri.clone(), &doc.text, d, goal_text) {
            actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                title: format!("intro {} 个 binder（把证明写成 lambda 的第一步）", intros),
                kind: Some(CodeActionKind::QUICKFIX),
                diagnostics: None,
                edit: Some(edit),
                command: None,
                is_preferred: None,
                disabled: None,
                data: None,
            }));
        }
        if actions.is_empty() {
            Ok(None)
        } else {
            Ok(Some(actions))
        }
    }
}

/// Replace the first `???` inside the declaration with `fun (x : T) => ???`,
/// using the same "tactics build a lambda" machinery as the REPL `#prove`.
fn intro_edit(uri: Url, text: &str, d: &DeclState, goal_text: &str) -> Option<WorkspaceEdit> {
    let decl_src = &text[d.span.start.offset..d.span.end.offset];
    let hole_rel = decl_src.find("???")?;
    let hole_off = d.span.start.offset + hole_rel;
    let (hl, hc) = offset_to_line_col(text, hole_off);
    let mut state = sokonanoda_front::proof::ProofState::start(goal_text).ok()?;
    state.intro("x").ok()?;
    // The intro step is just "peel one binder and keep the hole":
    //   fun (x : T) => ???
    let replacement = state.lambda_text();
    let range = Range {
        start: Position {
            line: hl as u32,
            character: hc as u32,
        },
        end: Position {
            line: hl as u32,
            character: (hc + 3) as u32,
        },
    };
    let mut changes = HashMap::new();
    changes.insert(
        uri,
        vec![TextEdit {
            range,
            new_text: replacement,
        }],
    );
    Some(WorkspaceEdit {
        changes: Some(changes),
        ..Default::default()
    })
}

fn offset_to_line_col(text: &str, offset: usize) -> (usize, usize) {
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
            col += 1;
        }
    }
    (line, col)
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let (service, socket) = LspService::new(Backend::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}

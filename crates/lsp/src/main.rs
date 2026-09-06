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

mod actions;
mod render;

use actions::intro_edit;
use render::{
    decl_at, decl_name, diagnostic_from_compile, diagnostic_from_parse, hover_type_at, range_of,
    status_label, symbol_kind,
};
use sokonanoda_front::compile::{check_document, DeclStatus, DocumentReport};
use sokonanoda_front::parse;
use std::sync::Mutex;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

#[derive(Default)]
struct Doc {
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

    async fn refresh(&self, uri: Url, text: String, version: Option<i32>) {
        let (doc, diagnostics) = {
            let mut doc = Doc {
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
            .publish_diagnostics(uri, diagnostics, version)
            .await;
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

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let (service, socket) = LspService::new(Backend::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}

#[cfg(test)]
mod tests {
    use super::Backend;
    use futures::StreamExt;
    use serde_json::{json, Value};
    use std::time::Duration;
    use tower::Service;
    use tower::ServiceExt;
    use tower_lsp::jsonrpc::Request as RpcRequest;
    use tower_lsp::lsp_types::*;
    use tower_lsp::{ClientSocket, LspService};

    /// Guard only: any server→client message must arrive within this budget.
    const TIMEOUT: Duration = Duration::from_secs(2);
    const URI: &str = "file:///test.sokonanoda";

    const VALID: &str = "def id : Prop -> Prop := fun (x : Prop) => x\n";
    const EXERCISE: &str = "example : Prop -> Prop := ???\n";
    const KERNEL_BAD: &str = "def bad : Prop -> Type := fun (x : Prop) => x\n";
    const PARSE_BAD: &str = "def broken : Prop :=\n";

    /// 0-based LSP position for a char offset in an (ASCII) source text.
    fn lsp_pos(src: &str, offset: usize) -> Position {
        let before = &src[..offset];
        let line = before.matches('\n').count() as u32;
        let character = (offset - before.rfind('\n').map(|i| i + 1).unwrap_or(0)) as u32;
        Position { line, character }
    }

    fn offset_of(src: &str, needle: &str) -> usize {
        src.find(needle)
            .unwrap_or_else(|| panic!("`{needle}` not found in `{src}`"))
    }

    fn position_json(pos: Position) -> Value {
        json!({"line": pos.line, "character": pos.character})
    }

    /// Drive one request/notification through the service (no stdio involved).
    async fn call(service: &mut LspService<Backend>, req: RpcRequest) -> Option<Value> {
        let resp = service
            .ready()
            .await
            .expect("service ready")
            .call(req)
            .await
            .expect("service call succeeded");
        resp.map(|resp| match resp.into_parts() {
            (_, Ok(result)) => result,
            (_, Err(err)) => panic!("json-rpc error response: {err:?}"),
        })
    }

    async fn notify(service: &mut LspService<Backend>, method: &'static str, params: Value) {
        let req = RpcRequest::build(method).params(params).finish();
        service
            .ready()
            .await
            .expect("service ready")
            .call(req)
            .await
            .expect("notification processed");
    }

    /// initialize/shutdown handshake: checks the advertised capabilities once.
    async fn handshake(service: &mut LspService<Backend>) {
        let init = RpcRequest::build("initialize")
            .params(json!({"capabilities": {}}))
            .id(1)
            .finish();
        let result = call(service, init).await.expect("initialize must answer");
        let result: InitializeResult =
            serde_json::from_value(result).expect("valid InitializeResult");
        let caps = result.capabilities;
        assert_eq!(
            caps.text_document_sync,
            Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
            "full text sync expected"
        );
        assert_eq!(
            caps.hover_provider,
            Some(HoverProviderCapability::Simple(true))
        );
        assert_eq!(caps.document_symbol_provider, Some(OneOf::Left(true)));
        assert_eq!(
            caps.code_lens_provider,
            Some(CodeLensOptions {
                resolve_provider: Some(false)
            })
        );
        assert_eq!(
            caps.code_action_provider,
            Some(CodeActionProviderCapability::Simple(true))
        );
    }

    async fn shutdown(service: &mut LspService<Backend>) {
        let req = RpcRequest::build("shutdown").id(i64::MAX).finish();
        let result = call(service, req).await;
        assert!(result.is_some(), "shutdown must answer");
    }

    async fn did_open(service: &mut LspService<Backend>, text: &str) {
        notify(
            service,
            "textDocument/didOpen",
            json!({"textDocument": {
                "uri": URI, "languageId": "sokonanoda", "version": 1, "text": text
            }}),
        )
        .await;
    }

    /// Read the next server→client message, failing with context if it never comes.
    async fn next_socket(socket: &mut ClientSocket, waiting_for: &str) -> RpcRequest {
        tokio::time::timeout(TIMEOUT, socket.next())
            .await
            .unwrap_or_else(|_| panic!("timed out after {TIMEOUT:?} waiting for {waiting_for}"))
            .unwrap_or_else(|| panic!("server socket closed while waiting for {waiting_for}"))
    }

    /// Drain server→client messages until a publishDiagnostics for our URI arrives.
    async fn wait_diagnostics(
        socket: &mut ClientSocket,
        waiting_for: &str,
    ) -> PublishDiagnosticsParams {
        loop {
            let msg = next_socket(socket, waiting_for).await;
            if msg.method() != "textDocument/publishDiagnostics" {
                continue;
            }
            let params: PublishDiagnosticsParams =
                serde_json::from_value(msg.params().cloned().unwrap_or_else(|| json!(null)))
                    .expect("valid PublishDiagnosticsParams");
            if params.uri.as_str() == URI {
                return params;
            }
        }
    }

    fn code_of(diag: &Diagnostic) -> &str {
        match &diag.code {
            Some(NumberOrString::String(code)) => code,
            other => panic!("expected a string diagnostic code, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn initialize_advertises_core_capabilities() {
        let (mut service, _socket) = LspService::new(Backend::new);
        handshake(&mut service).await;
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn did_open_valid_file_publishes_no_diagnostics() {
        let (mut service, mut socket) = LspService::new(Backend::new);
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
        let (mut service, mut socket) = LspService::new(Backend::new);
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
        let (mut service, mut socket) = LspService::new(Backend::new);
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
        let (mut service, mut socket) = LspService::new(Backend::new);
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
        let (mut service, mut socket) = LspService::new(Backend::new);
        handshake(&mut service).await;
        did_open(&mut service, EXERCISE).await;
        let _ = wait_diagnostics(&mut socket, "didOpen diagnostics").await;

        let pos = lsp_pos(EXERCISE, offset_of(EXERCISE, "???") + 1);
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
        let (mut service, mut socket) = LspService::new(Backend::new);
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
        let (mut service, mut socket) = LspService::new(Backend::new);
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
        let (mut service, mut socket) = LspService::new(Backend::new);
        handshake(&mut service).await;
        did_open(&mut service, EXERCISE).await;
        let _ = wait_diagnostics(&mut socket, "didOpen diagnostics").await;

        let hole = offset_of(EXERCISE, "???");
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
            "???".len() as u32,
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
            text_edit.new_text.ends_with("???"),
            "intro replacement must keep the hole, got {:?}",
            text_edit.new_text
        );
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn did_change_recomputes_diagnostics() {
        let (mut service, mut socket) = LspService::new(Backend::new);
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
}

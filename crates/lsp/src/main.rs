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

use actions::{exact_binder, intro_edit};
use render::{
    decl_at, decl_name, diagnostic_from_compile, diagnostic_from_parse, hover_type_at, range_of,
    status_label, symbol_kind,
};
use sokonanoda_front::compile::{
    check_document_with, prelude_mode_from_source, CompileOptions, DeclStatus, DocumentReport,
};
use sokonanoda_front::parse;
use sokonanoda_front::semantic::{
    semantic_tokens as front_semantic_tokens, SemanticKind, SemanticSpan,
};
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
                    // The file-level `-- sokonanoda:prelude none` directive
                    // decides whether the trusted prelude is installed.
                    let options = CompileOptions {
                        prelude: prelude_mode_from_source(&text),
                    };
                    let report = check_document_with(&file, &options);
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
                DeclStatus::Open => {
                    let mut text = match &d.goal {
                        Some(goal) => format!(
                            "**{} {}** — 目标：`{}`\n",
                            d.kind.as_str(),
                            decl_name(d),
                            goal
                        ),
                        None => format!("**{} {}** — 待作答\n", d.kind.as_str(), decl_name(d)),
                    };
                    if !d.binders.is_empty() {
                        text.push_str("\n已引入假设：\n");
                        for b in &d.binders {
                            text.push_str(&format!("- `{}` : `{}`\n", b.name, b.ty));
                        }
                    }
                    text.push_str("\n在 `???` 处填写一个类型为目标的项。");
                    text
                }
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
        let mut actions: Vec<CodeActionOrCommand> = Vec::new();
        // Close the goal with a hypothesis whose type matches it, if any.
        if let Some((edit, binder)) = exact_binder(params.text_document.uri.clone(), &doc.text, d) {
            actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                title: format!("exact {binder}（用假设 {binder} 直接结束证明）"),
                kind: Some(CodeActionKind::QUICKFIX),
                diagnostics: None,
                edit: Some(edit),
                command: None,
                is_preferred: None,
                disabled: None,
                data: None,
            }));
        }
        // Peel one binder: intro turns the next proof step into a lambda.
        if let Some(goal_text) = &d.goal {
            let Ok(goal_expr) = sokonanoda_front::proof::parse_expr_text(goal_text) else {
                if actions.is_empty() {
                    return Ok(None);
                }
                return Ok(Some(actions));
            };
            let intros = match &goal_expr {
                sokonanoda_front::Expr::Forall { binders, .. } => binders.len(),
                sokonanoda_front::Expr::Arrow { .. } => 1,
                _ => 0,
            };
            if intros > 0 {
                if let Some(edit) =
                    intro_edit(params.text_document.uri.clone(), &doc.text, d, goal_text)
                {
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
            }
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
        assert_eq!(
            caps.semantic_tokens_provider,
            Some(SemanticTokensServerCapabilities::SemanticTokensOptions(
                semantic_token_options()
            )),
            "full semantic tokens with the shared legend expected"
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

    use super::*;

    const BARE_OK: &str =
        "-- sokonanoda:prelude none\ndef id : Prop -> Prop := fun (x : Prop) => x\n";
    const BARE_NAT: &str = "-- sokonanoda:prelude none\ndef two : Nat := 2\n";
    const FULL_NAT: &str = "def two : Nat := 2\n";

    #[tokio::test]
    async fn directive_bare_file_without_nat_checks_clean() {
        let (mut service, mut socket) = LspService::new(Backend::new);
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
        let (mut service, mut socket) = LspService::new(Backend::new);
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
        let (mut service, mut socket) = LspService::new(Backend::new);
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
        let src = "example : (a : Prop) -> a -> a := fun (a : Prop) => fun (h : a) => ???\n";
        let (mut service, mut socket) = LspService::new(Backend::new);
        handshake(&mut service).await;
        did_open(&mut service, src).await;
        let _ = wait_diagnostics(&mut socket, "partial hole diagnostics").await;

        let hole = offset_of(src, "???");
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
        let src = "example : (a : Prop) -> a -> a := fun (a : Prop) => fun (h : a) => ???\n";
        let (mut service, mut socket) = LspService::new(Backend::new);
        handshake(&mut service).await;
        did_open(&mut service, src).await;
        let _ = wait_diagnostics(&mut socket, "partial hole diagnostics").await;

        let hole = offset_of(src, "???");
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
        let src = "example : Prop -> Prop := ???\n";
        let (mut service, mut socket) = LspService::new(Backend::new);
        handshake(&mut service).await;
        did_open(&mut service, src).await;
        let _ = wait_diagnostics(&mut socket, "hole diagnostics").await;

        let hole = offset_of(src, "???");
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
        let src = "def two : Nat := 2\nexample : Sort 1 := ???\n";
        let (mut service, mut socket) = LspService::new(Backend::new);
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
                (1, 20, 3, SemanticTokenType::MACRO),    // ???（UTF-16 长度 3）
            ],
            "full token stream for {src:?}"
        );
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn semantic_tokens_full_handles_non_ascii_identifiers() {
        let src = "def α_id : Prop -> Prop := fun (x : Prop) => x\n";
        let (mut service, mut socket) = LspService::new(Backend::new);
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
}

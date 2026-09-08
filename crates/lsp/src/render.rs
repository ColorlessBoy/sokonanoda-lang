//! Rendering helpers: convert front-end compile/parse errors, spans and
//! declarations into LSP diagnostics, ranges, hovers and symbols.

use sokonanoda_front::compile::{
    DeclKind, DeclState, DeclStatus, DocumentReport, HoverType, ResolvedTarget,
};
use sokonanoda_front::references::{binder_name_span, decl_name_span, references_for, resolve_at};
use sokonanoda_front::semantic::SemanticKind;
use sokonanoda_front::{tokenize, Span, TokenKind};
use tower_lsp::lsp_types::*;

pub(crate) fn diagnostic_from_compile(err: &sokonanoda_front::compile::CompileError) -> Diagnostic {
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

pub(crate) fn diagnostic_from_parse(diag: &sokonanoda_front::Diagnostic) -> Diagnostic {
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

pub(crate) fn range_of(span: Span) -> Range {
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

pub(crate) fn pos_within_span(line: u32, character: u32, span: Span) -> bool {
    let l = line as usize + 1;
    let c = character as usize + 1;
    let after_start = (l, c) > (span.start.line, span.start.column)
        || (l, c) >= (span.start.line, span.start.column);
    let before_end = (l, c) < (span.end.line, span.end.column);
    after_start && before_end
}

pub(crate) fn hover_type_at(hovers: &[HoverType], line: u32, character: u32) -> Option<&HoverType> {
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

pub(crate) fn decl_at(decls: &[DeclState], line: u32, character: u32) -> Option<&DeclState> {
    decls
        .iter()
        .find(|d| pos_within_span(line, character, d.span))
}

/// The name use point under the cursor resolves to this definition
/// (smallest enclosing use wins; prelude/unknown idents stay unresolved).
pub(crate) fn definition_at(
    hovers: &[HoverType],
    line: u32,
    character: u32,
) -> Option<ResolvedTarget> {
    hover_type_at(hovers, line, character)
        .and_then(|h| h.resolution.as_ref())
        .cloned()
}

/// All use points of the definition at the cursor: taken from the smallest
/// use containing the cursor, or from a use whose definition contains it
/// (cursor on the binder / declaration itself).
pub(crate) fn highlight_uses(
    hovers: &[HoverType],
    line: u32,
    character: u32,
) -> Option<Vec<Range>> {
    let from_use = hover_type_at(hovers, line, character).and_then(|h| h.resolution.as_ref());
    let def_span = match from_use {
        Some(target) => Some(target.span()),
        None => hovers.iter().find_map(|h| {
            let span = h.resolution.as_ref()?.span();
            pos_within_span(line, character, span).then_some(span)
        }),
    }?;
    let uses: Vec<Range> = hovers
        .iter()
        .filter(|h| {
            h.resolution
                .as_ref()
                .is_some_and(|target| target.span() == def_span)
        })
        .map(|h| range_of(h.span))
        .collect();
    (!uses.is_empty()).then_some(uses)
}

/// The in-scope binder names (outermost first) at the cursor, from the
/// smallest enclosing hover row. Anonymous binders carry empty names.
/// 光标处的语义 token 类别（front::semantic 的 token 流为准）。
pub(crate) fn semantic_kind_at(text: &str, line: u32, character: u32) -> Option<SemanticKind> {
    let spans = sokonanoda_front::semantic::semantic_tokens(text);
    let offset = line_col_to_offset(text, line, character);
    spans
        .iter()
        .find(|s| {
            s.span.start.offset <= offset && offset < s.span.end.offset.max(s.span.start.offset + 1)
        })
        .map(|s| s.kind)
}

fn line_col_to_offset(text: &str, line: u32, character: u32) -> usize {
    let mut offset = 0usize;
    for (i, l) in text.lines().enumerate() {
        if i == line as usize {
            let within = text[offset..]
                .char_indices()
                .nth(character as usize)
                .map(|(o, _)| o)
                .unwrap_or(l.len());
            return offset + within.min(l.len());
        }
        offset += l.len() + 1;
    }
    text.len()
}

pub(crate) fn scope_names_at(hovers: &[HoverType], line: u32, character: u32) -> Option<&[String]> {
    hover_type_at(hovers, line, character).map(|h| h.scope_names.as_slice())
}

pub(crate) fn symbol_kind(kind: DeclKind) -> SymbolKind {
    match kind {
        DeclKind::Definition => SymbolKind::FUNCTION,
        DeclKind::Theorem => SymbolKind::KEY,
        DeclKind::Axiom => SymbolKind::PROPERTY,
        DeclKind::Inductive => SymbolKind::STRUCT,
        DeclKind::Example => SymbolKind::CONSTANT,
    }
}

pub(crate) fn status_label(status: DeclStatus) -> &'static str {
    match status {
        DeclStatus::Open => "exercise: open",
        DeclStatus::Checked => "solved ✓",
        DeclStatus::Failed => "failed",
    }
}

pub(crate) fn decl_name(d: &DeclState) -> String {
    match &d.name {
        Some(n) => n.clone(),
        None => format!("{}@{}", d.kind.as_str(), d.span.start.line),
    }
}

// ---- rename / references（docs/design-rename-inlay.md §2–§3）----

/// The definition's name token span for a resolved target: a binder's own
/// first identifier, or the declaration's defining name token. Token-precise
/// (front lexical helpers), never a text scan of the identifier spelling.
fn definition_name_span(text: &str, target: &ResolvedTarget) -> Option<Span> {
    match target {
        ResolvedTarget::Binder(binder) => binder_name_span(text, *binder),
        ResolvedTarget::Declaration { name, span } => decl_name_span(text, *span, name),
    }
}

/// `textDocument/prepareRename`：光标处可改名目标的名字子 span + 原名。
/// 解析不到（prelude 名、匿名 example、声明外）→ `None`。
pub(crate) fn prepare_rename(
    text: &str,
    report: &DocumentReport,
    position: Position,
) -> Option<PrepareRenameResponse> {
    let target = resolve_at(&report.hovers, position.line, position.character)?;
    let name_span = definition_name_span(text, &target)?;
    let placeholder = text
        .get(name_span.start.offset..name_span.end.offset)?
        .to_string();
    Some(PrepareRenameResponse::RangeWithPlaceholder {
        range: range_of(name_span),
        placeholder,
    })
}

/// `textDocument/rename`：语义改写（use→def 反向分组 + 定义名 token）。
/// 位置不可解析 / 新名非法 → `Err`（ResponseError，绝不返回空 edit）。
pub(crate) fn rename(
    text: &str,
    version: i32,
    report: &DocumentReport,
    params: RenameParams,
) -> tower_lsp::jsonrpc::Result<Option<WorkspaceEdit>> {
    if !is_valid_new_name(&params.new_name) {
        return Err(tower_lsp::jsonrpc::Error::invalid_params(format!(
            "「{}」不是合法标识符：只能包含字母/数字/下划线/非 ASCII 字符，且不能以数字开头",
            params.new_name
        )));
    }
    let position = params.text_document_position.position;
    let Some(target) = resolve_at(&report.hovers, position.line, position.character) else {
        return Err(tower_lsp::jsonrpc::Error::invalid_params(
            "这里没有可以改名的名字",
        ));
    };
    // The definition's name token plus every use point, from the semantic
    // use→def map — comments/strings with the same spelling stay untouched.
    let mut spans: Vec<Span> = Vec::new();
    if let Some(name) = definition_name_span(text, &target) {
        spans.push(name);
    }
    spans.extend(references_for(&report.hovers, &target));
    spans.sort_by_key(|s| s.start.offset);
    spans.dedup_by_key(|s| s.start.offset);
    let edits: Vec<_> = spans
        .into_iter()
        .map(|span| {
            OneOf::Left(TextEdit {
                range: range_of(span),
                new_text: params.new_name.clone(),
            })
        })
        .collect();
    if edits.is_empty() {
        // Defensive: a resolved target always contributes a definition site.
        return Ok(None);
    }
    Ok(Some(WorkspaceEdit {
        document_changes: Some(DocumentChanges::Edits(vec![TextDocumentEdit {
            text_document: OptionalVersionedTextDocumentIdentifier::new(
                params.text_document_position.text_document.uri,
                version,
            ),
            edits,
        }])),
        ..Default::default()
    }))
}

/// `textDocument/references`：目标全部使用点（`include_declaration` 时
/// 定义名 Location 插在最前；按 offset 排序去重）。
pub(crate) fn find_references(
    uri: Url,
    text: &str,
    report: &DocumentReport,
    position: Position,
    include_declaration: bool,
) -> Option<Vec<Location>> {
    let target = resolve_at(&report.hovers, position.line, position.character)?;
    let mut spans: Vec<Span> = Vec::new();
    if include_declaration {
        spans.extend(definition_name_span(text, &target));
    }
    spans.extend(references_for(&report.hovers, &target));
    spans.sort_by_key(|s| s.start.offset);
    spans.dedup_by_key(|s| s.start.offset);
    Some(
        spans
            .into_iter()
            .map(|span| Location {
                uri: uri.clone(),
                range: range_of(span),
            })
            .collect(),
    )
}

/// A rename target must lex as exactly one identifier token spanning the
/// whole string (`9x` lexes as number + ident, spaces split tokens — both
/// rejected without touching the kernel).
fn is_valid_new_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    let Ok(tokens) = tokenize(name) else {
        return false;
    };
    let mut meaningful = tokens.iter().filter(|t| t.kind != TokenKind::Eof);
    let Some(first) = meaningful.next() else {
        return false;
    };
    if meaningful.next().is_some() {
        return false;
    }
    matches!(&first.kind, TokenKind::Ident(text) if text == name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{
        call, did_open, handshake, lsp_pos, offset_of, position_json, test_service,
        wait_diagnostics, URI,
    };
    use crate::Backend;
    use serde_json::json;
    use tower::Service;
    use tower::ServiceExt;
    use tower_lsp::jsonrpc::Request as RpcRequest;
    use tower_lsp::{ClientSocket, LspService};

    const REFS: &str = "def id : Prop -> Prop := fun (x : Prop) => x\n#check id\n";
    // def-name `id` at (0,4)..(0,6); the `#check id` use at (1,7)..(1,9).

    const NO_USE: &str = "def id : Prop -> Prop := fun (x : Prop) => x\n";
    // No use points anywhere: prelude names stay unresolved.

    const DOUBLE: &str = "def double : Nat -> Nat := fun (n : Nat) => n + n\n";
    // binder name `n` at (0,32); body uses at (0,44) and (0,48).

    const SHADOW: &str = "def f : Prop -> Prop -> Prop := fun (x : Prop) => fun (x : Prop) => x\n";
    // outer binder name at (0,37); inner binder name at (0,55), its body use
    // at (0,68).

    async fn setup(src: &str) -> (LspService<Backend>, ClientSocket) {
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, src).await;
        let _ = wait_diagnostics(&mut socket, "setup diagnostics").await;
        (service, socket)
    }

    async fn references_at(
        service: &mut LspService<Backend>,
        src: &str,
        offset: usize,
        include_declaration: bool,
    ) -> Option<Vec<Location>> {
        let result = call(
            service,
            RpcRequest::build("textDocument/references")
                .params(json!({
                    "textDocument": {"uri": URI},
                    "position": position_json(lsp_pos(src, offset)),
                    "context": {"includeDeclaration": include_declaration},
                }))
                .id(100)
                .finish(),
        )
        .await
        .expect("textDocument/references must answer");
        serde_json::from_value(result).expect("valid references response")
    }

    async fn prepare_at(
        service: &mut LspService<Backend>,
        src: &str,
        offset: usize,
    ) -> Option<PrepareRenameResponse> {
        let result = call(
            service,
            RpcRequest::build("textDocument/prepareRename")
                .params(json!({
                    "textDocument": {"uri": URI},
                    "position": position_json(lsp_pos(src, offset)),
                }))
                .id(101)
                .finish(),
        )
        .await
        .expect("textDocument/prepareRename must answer");
        serde_json::from_value(result).expect("valid prepareRename response")
    }

    fn rename_request(src: &str, offset: usize, new_name: &str) -> RpcRequest {
        RpcRequest::build("textDocument/rename")
            .params(json!({
                "textDocument": {"uri": URI},
                "position": position_json(lsp_pos(src, offset)),
                "newName": new_name,
            }))
            .id(102)
            .finish()
    }

    async fn rename_at(
        service: &mut LspService<Backend>,
        src: &str,
        offset: usize,
        new_name: &str,
    ) -> Option<WorkspaceEdit> {
        let result = call(service, rename_request(src, offset, new_name))
            .await
            .expect("textDocument/rename must answer");
        serde_json::from_value(result).expect("valid rename response")
    }

    /// Like `call`, but returns the error instead of panicking — needed to
    /// assert ResponseError behaviour (invalid rename params).
    async fn call_raw(
        service: &mut LspService<Backend>,
        req: RpcRequest,
    ) -> Option<Result<serde_json::Value, tower_lsp::jsonrpc::Error>> {
        let resp = service
            .ready()
            .await
            .expect("service ready")
            .call(req)
            .await
            .expect("service call succeeded");
        resp.map(|resp| match resp.into_parts() {
            (_, Ok(result)) => Ok(result),
            (_, Err(err)) => Err(err),
        })
    }

    /// The single document's edits of a rename answer, version included.
    fn document_edits(edit: WorkspaceEdit) -> (i32, Vec<(Range, String)>) {
        let Some(DocumentChanges::Edits(docs)) = edit.document_changes else {
            panic!(
                "expected documentChanges edits, got {:?}",
                edit.document_changes
            );
        };
        assert_eq!(docs.len(), 1, "one document touched: {docs:?}");
        assert_eq!(docs[0].text_document.uri.as_str(), URI);
        let edits = docs[0]
            .edits
            .iter()
            .map(|edit| match edit {
                OneOf::Left(edit) => (edit.range, edit.new_text.clone()),
                OneOf::Right(other) => panic!("no annotations expected, got {other:?}"),
            })
            .collect();
        let version = match &docs[0].text_document {
            OptionalVersionedTextDocumentIdentifier {
                version: Some(v), ..
            } => *v,
            other => panic!("renamed document must be versioned, got {other:?}"),
        };
        (version, edits)
    }

    // ---- references ----

    #[tokio::test]
    async fn references_from_a_use_point_list_the_use() {
        let src = REFS;
        let (mut service, _socket) = setup(src).await;
        let use_at = offset_of(src, "#check id") + "#check ".len();
        let refs = references_at(&mut service, src, use_at + 1, false)
            .await
            .expect("a resolved use point answers with its references");
        assert_eq!(refs.len(), 1, "no declaration requested: {refs:?}");
        assert_eq!(refs[0].range.start, lsp_pos(src, use_at));
        assert_eq!(refs[0].range.end, lsp_pos(src, use_at + 2));
    }

    #[tokio::test]
    async fn references_include_declaration_puts_the_definition_first() {
        let src = REFS;
        let (mut service, _socket) = setup(src).await;
        let use_at = offset_of(src, "#check id") + "#check ".len();
        let refs = references_at(&mut service, src, use_at + 1, true)
            .await
            .expect("references with the declaration");
        assert_eq!(refs.len(), 2, "definition name + use: {refs:?}");
        assert_eq!(refs[0].range.start, lsp_pos(src, 4), "def-name first");
        assert_eq!(refs[0].range.end, lsp_pos(src, 6));
        assert_eq!(refs[1].range.start, lsp_pos(src, use_at), "use second");
    }

    #[tokio::test]
    async fn references_from_the_definition_site_match_the_use_site() {
        let src = REFS;
        let (mut service, _socket) = setup(src).await;
        let use_at = offset_of(src, "#check id") + "#check ".len();
        // Cursor on the def name: same bidirectional answer.
        let refs = references_at(&mut service, src, offset_of(src, "id") + 1, true)
            .await
            .expect("the declaration name resolves as a definition point");
        assert_eq!(refs.len(), 2);
        assert_eq!(refs[0].range.start, lsp_pos(src, 4));
        assert_eq!(refs[1].range.start, lsp_pos(src, use_at));
    }

    #[tokio::test]
    async fn references_without_a_target_return_none() {
        let (mut service, _socket) = setup(NO_USE).await;
        let refs = references_at(&mut service, NO_USE, offset_of(NO_USE, "Prop") + 1, true).await;
        assert!(refs.is_none(), "prelude names have no references: {refs:?}");
    }

    // ---- prepareRename ----

    #[tokio::test]
    async fn prepare_rename_on_a_binder_covers_the_name_token() {
        let src = DOUBLE;
        let (mut service, _socket) = setup(src).await;
        // Cursor inside the binder name `n` (0,32)..(0,33).
        let resp = prepare_at(&mut service, src, 32)
            .await
            .expect("the binder is renameable");
        let PrepareRenameResponse::RangeWithPlaceholder { range, placeholder } = resp else {
            panic!("expected range with placeholder, got {resp:?}");
        };
        assert_eq!(range.start, lsp_pos(src, 32));
        assert_eq!(range.end, lsp_pos(src, 33));
        assert_eq!(placeholder, "n");
    }

    #[tokio::test]
    async fn prepare_rename_on_a_declaration_covers_the_name_token() {
        let src = REFS;
        let (mut service, _socket) = setup(src).await;
        let resp = prepare_at(&mut service, src, offset_of(src, "id") + 1)
            .await
            .expect("the declaration name is renameable");
        let PrepareRenameResponse::RangeWithPlaceholder { range, placeholder } = resp else {
            panic!("expected range with placeholder, got {resp:?}");
        };
        assert_eq!(range.start, lsp_pos(src, 4));
        assert_eq!(range.end, lsp_pos(src, 6));
        assert_eq!(placeholder, "id");
    }

    #[tokio::test]
    async fn prepare_rename_is_null_for_prelude_names() {
        let (mut service, _socket) = setup(NO_USE).await;
        let resp = prepare_at(&mut service, NO_USE, offset_of(NO_USE, "Prop") + 1).await;
        assert!(resp.is_none(), "prelude names cannot be renamed: {resp:?}");
    }

    // ---- rename ----

    #[tokio::test]
    async fn rename_rewrites_binder_and_all_uses_with_version() {
        let src = DOUBLE;
        let (mut service, _socket) = setup(src).await;
        // Cursor on the first body use.
        let edit = rename_at(&mut service, src, 44, "k")
            .await
            .expect("a valid rename answers with an edit");
        let (version, edits) = document_edits(edit);
        assert_eq!(version, 1, "the didOpen version must be carried");
        let offsets = [32, 44, 48]; // binder name + both uses
        assert_eq!(edits.len(), offsets.len(), "edits: {edits:?}");
        for ((range, new_text), offset) in edits.iter().zip(offsets) {
            assert_eq!(range.start, lsp_pos(src, offset), "edit at {offset}");
            assert_eq!(range.end, lsp_pos(src, offset + 1));
            assert_eq!(new_text, "k");
        }
    }

    #[tokio::test]
    async fn rename_respects_shadowing_inner_binder_only() {
        let src = SHADOW;
        let (mut service, _socket) = setup(src).await;
        // Cursor on the inner body use; the inner binder shadows the outer.
        let edit = rename_at(&mut service, src, 68, "y")
            .await
            .expect("the inner binder is renameable");
        let (_, edits) = document_edits(edit);
        let offsets = [55, 68]; // inner binder name + its use
        assert_eq!(edits.len(), offsets.len(), "edits: {edits:?}");
        for ((range, new_text), offset) in edits.iter().zip(offsets) {
            assert_eq!(range.start, lsp_pos(src, offset));
            assert_eq!(new_text, "y");
        }
        assert!(
            edits
                .iter()
                .all(|(range, _)| range.start != lsp_pos(src, 37)),
            "the outer binder name (0,37) must stay untouched: {edits:?}"
        );
    }

    #[tokio::test]
    async fn rename_does_not_touch_comments() {
        let src = "def double : Nat -> Nat := fun (n : Nat) => n + n -- n 是个数\n";
        let (mut service, _socket) = setup(src).await;
        let edit = rename_at(&mut service, src, 44, "k")
            .await
            .expect("a valid rename answers with an edit");
        let (_, edits) = document_edits(edit);
        assert_eq!(
            edits.len(),
            3,
            "binder name + two uses, never the commented `n`: {edits:?}"
        );
    }

    #[tokio::test]
    async fn rename_rejects_invalid_new_names() {
        let src = DOUBLE;
        let (mut service, _socket) = setup(src).await;
        for bad in ["9x", "x y", ""] {
            let response = call_raw(&mut service, rename_request(src, 44, bad)).await;
            match response {
                Some(Err(err)) => assert!(
                    err.message.contains("不是合法标识符"),
                    "teaching message expected, got {err:?}"
                ),
                other => panic!("`{bad}` must be rejected, got {other:?}"),
            }
        }
    }

    #[tokio::test]
    async fn rename_without_a_target_is_an_error() {
        let (mut service, _socket) = setup(NO_USE).await;
        let response = call_raw(
            &mut service,
            rename_request(NO_USE, offset_of(NO_USE, "Prop") + 1, "k"),
        )
        .await;
        match response {
            Some(Err(err)) => {
                assert!(
                    err.message.contains("这里没有可以改名的名字"),
                    "teaching message expected, got {err:?}"
                );
            }
            other => panic!("an unresolvable position must be an error: {other:?}"),
        }
    }
}

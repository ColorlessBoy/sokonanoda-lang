//! Rendering helpers: convert front-end compile/parse errors, spans and
//! declarations into LSP diagnostics, ranges, hovers and symbols.

use sokonanoda_front::compile::{DeclKind, DeclState, DeclStatus, HoverType};
use sokonanoda_front::Span;
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

pub(crate) fn hover_type_at<'a>(
    hovers: &'a [HoverType],
    line: u32,
    character: u32,
) -> Option<&'a HoverType> {
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

pub(crate) fn decl_at<'a>(
    decls: &'a [DeclState],
    line: u32,
    character: u32,
) -> Option<&'a DeclState> {
    decls
        .iter()
        .find(|d| pos_within_span(line, character, d.span))
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

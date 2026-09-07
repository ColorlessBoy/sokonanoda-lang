//! Name→source-position helpers for rename / find-references (seeded by the
//! integration pass; find-references consumers live with the LSP layer).
//!
//! Everything here is **token-precise**: spans come from the lexer over the
//! exact source slice, never from text scanning of identifiers.

use crate::compile::{HoverType, ResolvedTarget};
use crate::span::Span;
use crate::token::{tokenize, TokenKind};

/// The name token's span (document coordinates) inside the defining command
/// that declares `name` (`def`/`theorem`/`axiom` and inductive ctors/recs).
/// The first identifier matching `name` after the leading keyword is the
/// defining occurrence — declaration names precede their types by grammar.
pub fn decl_name_span(doc: &str, decl_span: Span, name: &str) -> Option<Span> {
    let slice = slice_of(doc, decl_span)?;
    first_ident_span(slice, doc, decl_span.start.offset, Some(name))
}

/// The binder name token's span (document coordinates) inside a binder's
/// source span (`(x : Prop)` / `{x : A}`); the slice's first identifier.
pub fn binder_name_span(doc: &str, binder_span: Span) -> Option<Span> {
    let slice = slice_of(doc, binder_span)?;
    first_ident_span(slice, doc, binder_span.start.offset, None)
}

/// The target under the cursor, resolving in both directions:
/// - a use point resolves through its recorded resolution (the smallest
///   enclosing use wins, so shadowed inner binders take precedence);
/// - a definition point (cursor inside the binder or the defining command)
///   resolves to itself, found through any use whose resolution's definition
///   span covers the cursor — the same bidirectional rule as the LSP
///   document highlight (crates/lsp/src/render.rs::highlight_uses).
///
/// `None` when the cursor sits on a prelude name or outside any name.
pub fn resolve_at(hovers: &[HoverType], line: u32, character: u32) -> Option<ResolvedTarget> {
    // Use point: the smallest enclosing hover's own resolution.
    let smallest = hovers
        .iter()
        .filter(|h| pos_within(line, character, h.span))
        .min_by_key(|h| h.span.end.offset.saturating_sub(h.span.start.offset));
    if let Some(target) = smallest.and_then(|h| h.resolution.clone()) {
        return Some(target);
    }
    // Definition point: a use of the definition under the cursor points back.
    hovers.iter().find_map(|h| {
        let target = h.resolution.clone()?;
        pos_within(line, character, target.span()).then_some(target)
    })
}

/// LSP (0-based) position inside a front (1-based) span.
fn pos_within(line: u32, character: u32, span: Span) -> bool {
    let (l, c) = (line as usize + 1, character as usize + 1);
    (l, c) >= (span.start.line, span.start.column) && (l, c) < (span.end.line, span.end.column)
}

/// All use points of `target`: spans of name uses whose resolution is
/// `target`, ascending by offset. The definition site itself is *not*
/// included (callers append it for `include_declaration`).
pub fn references_for(hovers: &[HoverType], target: &ResolvedTarget) -> Vec<Span> {
    let mut spans: Vec<Span> = hovers
        .iter()
        .filter(|h| h.resolution.as_ref() == Some(target))
        .map(|h| h.span)
        .collect();
    spans.sort_by_key(|s| s.start.offset);
    spans.dedup_by_key(|s| s.start.offset);
    spans
}

fn slice_of(doc: &str, span: Span) -> Option<&str> {
    let start = span.start.offset.min(doc.len());
    let end = span.end.offset.clamp(start, doc.len());
    doc.get(start..end)
}

/// Shift a slice-relative token span into document coordinates (offset and
/// line/column all recomputed against `doc`).
fn rebase(mut span: Span, doc: &str, base: usize) -> Span {
    span.start.offset += base;
    span.end.offset += base;
    let (line, column) = line_col_of(doc, span.start.offset);
    span.start.line = line;
    span.start.column = column;
    let (line, column) = line_col_of(doc, span.end.offset);
    span.end.line = line;
    span.end.column = column;
    span
}

fn line_col_of(doc: &str, offset: usize) -> (usize, usize) {
    let before = &doc[..offset.min(doc.len())];
    let line = before.matches('\n').count() + 1;
    let column = offset - before.rfind('\n').map(|i| i + 1).unwrap_or(0) + 1;
    (line, column)
}

fn first_ident_span(slice: &str, doc: &str, base: usize, name: Option<&str>) -> Option<Span> {
    let tokens = tokenize(slice).ok()?;
    for (position, token) in tokens.iter().enumerate() {
        let TokenKind::Ident(text) = &token.kind else {
            continue;
        };
        // Skip the leading keyword ident (`def`/`theorem`/`axiom`/`ctor`…):
        // the defining name is never the first token of a command slice.
        // Binder slices start with a paren/brace, so nothing is skipped there.
        if position == 0 {
            continue;
        }
        let matches = match name {
            Some(want) => text == want,
            None => true,
        };
        if matches {
            return Some(rebase(token.span, doc, base));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::span::Pos;

    const DOC: &str = "def id : Prop -> Prop := fun (x : Prop) => x\n#check id\n";
    // offsets: "def " 0..4, "id" 4..6, "(x : Prop)" 29..39, "x" 30..31,
    // "#check " 45..52, final "id" 52..54.

    fn span(start: usize, end: usize) -> Span {
        let (sl, sc) = line_col_of(DOC, start);
        let (el, ec) = line_col_of(DOC, end);
        Span {
            start: Pos {
                offset: start,
                line: sl,
                column: sc,
            },
            end: Pos {
                offset: end,
                line: el,
                column: ec,
            },
        }
    }

    #[test]
    fn decl_name_span_covers_the_name_token() {
        let span = decl_name_span(DOC, span(0, 44), "id").expect("name token found");
        assert_eq!(&DOC[span.start.offset..span.end.offset], "id");
        assert_eq!(span.start.offset, 4);
        assert_eq!(span.end.offset, 6);
        assert_eq!((span.start.line, span.start.column), (1, 5));
    }

    #[test]
    fn binder_name_span_is_the_first_ident() {
        let span = binder_name_span(DOC, span(29, 39)).expect("binder name found");
        assert_eq!(&DOC[span.start.offset..span.end.offset], "x");
        assert_eq!(span.start.offset, 30);
    }

    #[test]
    fn references_for_collects_use_points_in_order() {
        let report = crate::compile::check_document(&crate::parser::parse(DOC).expect("parses"));
        let def_span = report
            .decls
            .iter()
            .find(|d| d.name.as_deref() == Some("id"))
            .expect("the id declaration")
            .span;
        let target = ResolvedTarget::Declaration {
            name: "id".to_string(),
            span: def_span,
        };
        let uses = references_for(&report.hovers, &target);
        assert_eq!(uses.len(), 1, "the `#check id` use: {uses:?}");
        assert_eq!(&DOC[uses[0].start.offset..uses[0].end.offset], "id");
        assert_eq!(uses[0].start.offset, 52);
    }

    #[test]
    fn references_for_empty_without_resolutions() {
        let report = crate::compile::check_document(&crate::parser::parse(DOC).expect("parses"));
        let target = ResolvedTarget::Declaration {
            name: "ghost".to_string(),
            span: Span::default(),
        };
        assert!(references_for(&report.hovers, &target).is_empty());
    }

    // ---- resolve_at：光标处双向解析（rename / find-references 共用）----
    // 光标坐标是 LSP 0-based（行号、行内字符），与 front 的行内 1-based 列对齐。

    const SHADOW: &str =
        "def f : Prop -> Prop -> Prop := fun (x : Prop) => fun (x : Prop) => x\n#check f\n";
    // offsets (line 0): outer binder "(x : Prop)" 36..46, inner binder 54..64,
    // inner body use `x` 68..69.

    const CROSS: &str =
        "def f : Prop -> Prop := fun (x : Prop) => x\ndef g : Prop -> Prop := fun (x : Prop) => x\n";
    // offsets: f's binder 28..38 (line 0); g's binder 72..82, g's body use
    // 86..87 — both on line 1, the use at line-character 42.

    fn check_report(doc: &str) -> crate::compile::DocumentReport {
        crate::compile::check_document(&crate::parser::parse(doc).expect("parses"))
    }

    #[test]
    fn resolve_at_resolves_a_use_point_to_its_target() {
        let report = check_report(DOC);
        // `#check id` sits on line 1; `id` starts at line-character 7.
        let target = resolve_at(&report.hovers, 1, 7).expect("use point resolves");
        let ResolvedTarget::Declaration { name, span } = target else {
            panic!("a top-level use resolves to its declaration, got {target:?}");
        };
        assert_eq!(name, "id");
        assert_eq!(span.start.offset, 0, "the whole defining command");
    }

    #[test]
    fn resolve_at_resolves_the_definition_under_the_cursor() {
        let report = check_report(DOC);
        // Cursor on the binder `(x : Prop)`: the body use points back at it.
        let target = resolve_at(&report.hovers, 0, 30).expect("binder resolves");
        assert!(
            matches!(target, ResolvedTarget::Binder(s) if s.start.offset == 29),
            "the binder's own span, got {target:?}"
        );
    }

    #[test]
    fn resolve_at_is_none_for_prelude_names() {
        // No use points anywhere: a cursor on the type-position `Prop`
        // (line-character 9..13) has nothing to resolve through.
        let report = check_report("def id : Prop -> Prop := fun (x : Prop) => x\n");
        assert!(resolve_at(&report.hovers, 0, 9 + 1).is_none());
    }

    #[test]
    fn resolve_at_inner_binder_wins_over_outer_shadow() {
        let report = check_report(SHADOW);
        let target = resolve_at(&report.hovers, 0, 68).expect("inner body use resolves");
        let ResolvedTarget::Binder(inner) = target else {
            panic!("shadowed use must resolve to its binder, got {target:?}");
        };
        assert_eq!(
            inner.start.offset, 54,
            "the inner binder, not the outer (36..46)"
        );
        let uses = references_for(&report.hovers, &ResolvedTarget::Binder(inner));
        assert_eq!(uses.len(), 1, "only the inner use: {uses:?}");
        assert_eq!(uses[0].start.offset, 68);
        // The outer binder must not inherit the inner use.
        let outer = ResolvedTarget::Binder(span(36, 46));
        assert!(
            references_for(&report.hovers, &outer).is_empty(),
            "the outer shadowed binder has no uses of its own"
        );
    }

    #[test]
    fn references_for_same_named_binders_in_different_decls_stay_separate() {
        let report = check_report(CROSS);
        // Cursor inside g's body `x` (second line, character 42).
        let target = resolve_at(&report.hovers, 1, 42).expect("g's body use resolves");
        let ResolvedTarget::Binder(g_binder) = target else {
            panic!("g's use resolves to g's binder, got {target:?}");
        };
        assert_eq!(g_binder.start.offset, 72, "g's own binder, not f's");
        let uses = references_for(&report.hovers, &ResolvedTarget::Binder(g_binder));
        assert_eq!(uses.len(), 1, "only g's use: {uses:?}");
        assert_eq!(uses[0].start.offset, 86);
        let f_target = ResolvedTarget::Binder(span(28, 38));
        let f_uses = references_for(&report.hovers, &f_target);
        assert_eq!(f_uses.len(), 1, "f's own body use only: {f_uses:?}");
        assert_eq!(
            f_uses[0].start.offset, 42,
            "f's binder keeps its own use and never g's (86)"
        );
    }
}

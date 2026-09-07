//! Inlay hints: the expected type of every open-exercise hole, rendered
//! right after the `???` (docs/design-rename-inlay.md §4).
//!
//! Read-only information only — no `textEdits` on hints (rust-analyzer
//! lesson: interactive inlays are expensive and rarely wanted).

use sokonanoda_front::compile::{DeclState, DeclStatus, DocumentReport};
use sokonanoda_front::Span;
use tower_lsp::lsp_types::*;

/// One hint per hole: sub-hole types come from the server-side walk
/// (`sub_goals`, matched by span); a lone main hole shows the remaining
/// goal. Hints without a known type are skipped (labels are never empty).
pub(crate) fn hole_hints(text: &str, report: &DocumentReport) -> Vec<InlayHint> {
    let _ = text;
    let mut hints = Vec::new();
    for d in &report.decls {
        if d.status != DeclStatus::Open {
            continue;
        }
        for hole in &d.holes {
            let Some(label) = hint_label(d, hole) else {
                continue;
            };
            hints.push(InlayHint {
                position: end_position(*hole),
                label,
                kind: Some(InlayHintKind::TYPE),
                text_edits: None,
                tooltip: Some(goal_tooltip(d)),
                padding_left: Some(true),
                padding_right: None,
                data: None,
            });
        }
    }
    hints
}

/// The hint label for one hole: `": <expected type>"`, or `None` when the
/// walk could not produce a type for it.
fn hint_label(d: &DeclState, hole: &Span) -> Option<InlayHintLabel> {
    let ty = if let Some(sub) = d.sub_goals.iter().find(|s| s.span == *hole) {
        sub.ty.clone()
    } else if d.holes.len() == 1 {
        d.goal.clone()
    } else {
        None
    }?;
    Some(InlayHintLabel::String(format!(": {ty}")))
}

/// The hole's end as a 0-based LSP position (front spans are 1-based).
fn end_position(span: Span) -> Position {
    Position {
        line: span.end.line.saturating_sub(1) as u32,
        character: span.end.column.saturating_sub(1) as u32,
    }
}

/// Goal-view tooltip for one hole: remaining goal plus the hypotheses the
/// lambda binders introduced so far (same data as the hover's Open branch).
fn goal_tooltip(d: &DeclState) -> InlayHintTooltip {
    let mut value = String::new();
    if let Some(goal) = &d.goal {
        value.push_str(&format!("剩余目标：`{goal}`\n"));
    }
    if !d.binders.is_empty() {
        value.push_str("\n已引入假设：\n");
        for b in &d.binders {
            value.push_str(&format!("- `{}` : `{}`\n", b.name, b.ty));
        }
    }
    InlayHintTooltip::MarkupContent(MarkupContent {
        kind: MarkupKind::Markdown,
        value,
    })
}

#[cfg(test)]
mod tests {
    use super::super::testutil::{
        call, did_open, handshake, lsp_pos, offset_of, position_json, test_service,
        wait_diagnostics, URI,
    };
    use serde_json::json;
    use tower_lsp::jsonrpc::Request as RpcRequest;
    use tower_lsp::lsp_types::*;

    const EXERCISE: &str = "example : Prop -> Prop := ???\n";
    const CHECKED: &str = "def id : Prop -> Prop := fun (x : Prop) => x\n";
    const ALL_CHECKED: &str = "def id : Prop -> Prop := fun (x : Prop) => x\ndef two : Nat := 2\n";
    const PARTIAL: &str =
        "example : (a : Prop) -> a -> a := fun (a : Prop) => fun (h : a) => ???\n";
    const AND_MULTI_HOLE: &str = "axiom And : Prop -> Prop -> Prop\n\
axiom And.intro : (a : Prop) -> (b : Prop) -> a -> b -> And a b\n\
theorem and_intro_rule : (a : Prop) -> (b : Prop) -> a -> b -> And a b := \
fun (a : Prop) => fun (b : Prop) => fun (ha : a) => fun (hb : b) => And.intro ??? ???\n";

    async fn ask_inlay(
        service: &mut tower_lsp::LspService<crate::Backend>,
        src: &str,
    ) -> Option<Vec<InlayHint>> {
        let result = call(
            service,
            RpcRequest::build("textDocument/inlayHint")
                .params(json!({
                    "textDocument": {"uri": URI},
                    "range": {
                        "start": {"line": 0, "character": 0},
                        "end": position_json(lsp_pos(src, src.len())),
                    },
                }))
                .id(90)
                .finish(),
        )
        .await
        .expect("textDocument/inlayHint must answer");
        serde_json::from_value(result).expect("valid inlay hint response")
    }

    fn label_of(hint: &InlayHint) -> &str {
        match &hint.label {
            InlayHintLabel::String(label) => label,
            other => panic!("expected a string label, got {other:?}"),
        }
    }

    fn tooltip_of(hint: &InlayHint) -> &str {
        match &hint.tooltip {
            Some(InlayHintTooltip::MarkupContent(markup)) => &markup.value,
            other => panic!("expected a markup tooltip, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn single_hole_hint_shows_goal_type_right_after_the_hole() {
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, EXERCISE).await;
        let _ = wait_diagnostics(&mut socket, "inlay diagnostics").await;

        let hints = ask_inlay(&mut service, EXERCISE)
            .await
            .expect("hints array");
        assert_eq!(hints.len(), 1, "one hint for the lone hole: {hints:?}");
        let hint = &hints[0];
        assert_eq!(label_of(hint), ": Prop -> Prop");
        assert_eq!(hint.kind, Some(InlayHintKind::TYPE));
        assert_eq!(hint.padding_left, Some(true));
        let after_hole = lsp_pos(EXERCISE, offset_of(EXERCISE, "???") + "???".len());
        assert_eq!(hint.position, after_hole, "hint sits right after `???`");
        assert!(
            tooltip_of(hint).contains("剩余目标：`Prop -> Prop`"),
            "tooltip carries the remaining goal: {}",
            tooltip_of(hint)
        );
    }

    #[tokio::test]
    async fn spine_hints_carry_sub_goal_types_in_offset_order() {
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, AND_MULTI_HOLE).await;
        let _ = wait_diagnostics(&mut socket, "spine diagnostics").await;

        let hints = ask_inlay(&mut service, AND_MULTI_HOLE)
            .await
            .expect("hints array");
        assert_eq!(hints.len(), 2, "two spine holes: {hints:?}");
        assert_eq!(label_of(&hints[0]), ": a", "first sub-hole type: {hints:?}");
        assert_eq!(
            label_of(&hints[1]),
            ": b",
            "second sub-hole type: {hints:?}"
        );
        let first = offset_of(AND_MULTI_HOLE, "???");
        let second = AND_MULTI_HOLE[first + 3..]
            .find("???")
            .expect("second hole exists")
            + first
            + 3;
        assert_eq!(hints[0].position, lsp_pos(AND_MULTI_HOLE, first + 3));
        assert_eq!(hints[1].position, lsp_pos(AND_MULTI_HOLE, second + 3));
    }

    #[tokio::test]
    async fn checked_declaration_produces_no_hints() {
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, CHECKED).await;
        let _ = wait_diagnostics(&mut socket, "checked diagnostics").await;

        let hints = ask_inlay(&mut service, CHECKED).await.expect("hints array");
        assert!(
            hints.is_empty(),
            "checked declarations must not hint: {hints:?}"
        );
    }

    #[tokio::test]
    async fn partial_answer_hint_lists_goal_and_hypotheses() {
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, PARTIAL).await;
        let _ = wait_diagnostics(&mut socket, "partial diagnostics").await;

        let hints = ask_inlay(&mut service, PARTIAL).await.expect("hints array");
        assert_eq!(hints.len(), 1, "one hint for the lone hole: {hints:?}");
        let hint = &hints[0];
        assert_eq!(label_of(hint), ": a", "remaining goal as the type");
        let tooltip = tooltip_of(hint);
        assert!(tooltip.contains("剩余目标：`a`"), "tooltip: {tooltip}");
        assert!(tooltip.contains("假设"), "tooltip: {tooltip}");
        assert!(tooltip.contains("`h` : `a`"), "tooltip: {tooltip}");
    }

    #[tokio::test]
    async fn hole_free_document_yields_an_empty_hint_array() {
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, ALL_CHECKED).await;
        let _ = wait_diagnostics(&mut socket, "all-checked diagnostics").await;

        let hints = ask_inlay(&mut service, ALL_CHECKED)
            .await
            .expect("hints array");
        assert!(hints.is_empty(), "no holes, no hints: {hints:?}");
    }
}

//! Code-action helpers: build quick-fix text edits for open exercises
//! (`intro` turns the first proof step into a lambda; `exact`/`assumption`
//! closes the goal with a hypothesis whose type matches it).

use sokonanoda_front::compile::{DeclState, GoalBinder};
use std::collections::HashMap;
use tower_lsp::lsp_types::*;

/// Locate the first `???` inside the declaration; returns the 0-based LSP
/// range covering the hole.
fn hole_range(text: &str, d: &DeclState) -> Option<Range> {
    let decl_src = &text[d.span.start.offset..d.span.end.offset];
    let hole_rel = decl_src.find("???")?;
    let hole_off = d.span.start.offset + hole_rel;
    let (hl, hc) = offset_to_line_col(text, hole_off);
    // `offset_to_line_col` is 1-based (matching our Spans); LSP wants 0-based.
    Some(Range {
        start: Position {
            line: (hl - 1) as u32,
            character: (hc - 1) as u32,
        },
        end: Position {
            line: (hl - 1) as u32,
            character: (hc - 1 + 3) as u32,
        },
    })
}

fn edit_on_hole(uri: Url, range: Range, new_text: String) -> WorkspaceEdit {
    let mut changes = HashMap::new();
    changes.insert(uri, vec![TextEdit { range, new_text }]);
    WorkspaceEdit {
        changes: Some(changes),
        ..Default::default()
    }
}

/// Replace the first `???` inside the declaration with `fun (x : T) => ???`,
/// using the same "tactics build a lambda" machinery as the REPL `#prove`.
pub(crate) fn intro_edit(
    uri: Url,
    text: &str,
    d: &DeclState,
    goal_text: &str,
) -> Option<WorkspaceEdit> {
    let range = hole_range(text, d)?;
    let mut state = sokonanoda_front::proof::ProofState::start(goal_text).ok()?;
    state.intro("x").ok()?;
    // The intro step is just "peel one binder and keep the hole":
    //   fun (x : T) => ???
    let replacement = state.lambda_text();
    Some(edit_on_hole(uri, range, replacement))
}

/// A hypothesis whose type matches the remaining goal closes it: find a
/// binder in the goal context with `ty == goal` and replace the hole with its
/// name (the kernel stays the judge — this edit merely proposes the term).
pub(crate) fn exact_binder(uri: Url, text: &str, d: &DeclState) -> Option<(WorkspaceEdit, String)> {
    let goal = d.goal.as_deref()?;
    let binder: &GoalBinder = d.binders.iter().find(|b| b.ty == goal)?;
    let range = hole_range(text, d)?;
    let edit = edit_on_hole(uri, range, binder.name.clone());
    Some((edit, binder.name.clone()))
}

pub(crate) fn offset_to_line_col(text: &str, offset: usize) -> (usize, usize) {
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

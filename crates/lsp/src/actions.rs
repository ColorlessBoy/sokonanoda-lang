//! Code-action helpers: build quick-fix text edits for open exercises
//! (`intro` turns the first proof step into a lambda).

use sokonanoda_front::compile::DeclState;
use std::collections::HashMap;
use tower_lsp::lsp_types::*;

/// Replace the first `???` inside the declaration with `fun (x : T) => ???`,
/// using the same "tactics build a lambda" machinery as the REPL `#prove`.
pub(crate) fn intro_edit(
    uri: Url,
    text: &str,
    d: &DeclState,
    goal_text: &str,
) -> Option<WorkspaceEdit> {
    let decl_src = &text[d.span.start.offset..d.span.end.offset];
    let hole_rel = decl_src.find("???")?;
    let hole_off = d.span.start.offset + hole_rel;
    // `offset_to_line_col` is 1-based (matching our Spans); LSP wants 0-based.
    let (hl, hc) = offset_to_line_col(text, hole_off);
    let mut state = sokonanoda_front::proof::ProofState::start(goal_text).ok()?;
    state.intro("x").ok()?;
    // The intro step is just "peel one binder and keep the hole":
    //   fun (x : T) => ???
    let replacement = state.lambda_text();
    let range = Range {
        start: Position {
            line: (hl - 1) as u32,
            character: (hc - 1) as u32,
        },
        end: Position {
            line: (hl - 1) as u32,
            character: (hc - 1 + 3) as u32,
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

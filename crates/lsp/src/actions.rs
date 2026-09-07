//! Code-action helpers: build quick-fix text edits for open exercises
//! (`intro` turns the first proof step into a lambda; `exact` closes the goal
//! with a hypothesis the kernel judges to match it).
//!
//! I9：`exact` 的假设匹配走 front::judge（合成完整声明交完整 kernel 裁决），
//! 文本比对已删除（REQUIREMENTS §2.8）。

use sokonanoda_front::compile::{CompileOptions, DeclState};
use sokonanoda_front::judge::{judge_terms, GoalBinderSpec, Judgement, OpenGoalSpec};
use std::collections::HashMap;
use tower_lsp::lsp_types::*;

/// Locate the first `???` inside the declaration; returns the 0-based LSP
/// range covering the hole.
pub(crate) fn hole_range(text: &str, d: &DeclState) -> Option<Range> {
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

pub(crate) fn edit_on_hole(uri: Url, range: Range, new_text: String) -> WorkspaceEdit {
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

/// A hypothesis the kernel judges defeq to the remaining goal closes it:
/// judge every written binder (innermost first, one pipeline run) and return
/// the first match's name, or `None` when no hypothesis closes the goal.
/// The declaration's universe parameters ride along so `Sort u` goals can be
/// judged (the judge synthesizes a declaration with the same universes).
pub(crate) fn exact_binder(
    prefix_src: &str,
    options: &CompileOptions,
    d: &DeclState,
) -> Option<String> {
    let goal = d.goal.as_deref()?;
    let spec = OpenGoalSpec {
        universe: d.universe.clone(),
        ty: goal.to_string(),
        binders: d
            .binders
            .iter()
            .map(|b| GoalBinderSpec {
                name: b.name.clone(),
                ty: Some(b.ty.clone()),
            })
            .collect(),
    };
    let mut names: Vec<String> = d.binders.iter().map(|b| b.name.clone()).collect();
    names.reverse();
    let refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
    let judgements = judge_terms(prefix_src, options, &spec, &refs);
    judgements
        .iter()
        .position(|j| matches!(j, Judgement::Match))
        .map(|i| names[i].clone())
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

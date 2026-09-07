//! Code-action helpers: map next-step suggestions into quick-fix text edits
//! for open exercises (docs/design-hints-suggestions.md §4.3).
//!
//! 建议生成在 `front::suggest`（kernel 验证优先：exact → rfl → refine →
//! intro；第一条标 preferred）。这里只做映射：
//! * 洞位来自 walk 的 `DeclState.holes`（按洞下标取，绝不扫描文本）；
//! * 替换文本来自 kernel 验证过的候选（exact 的假设名、rfl 的项）或结构
//!   模板（refine / intro）；
//! * 判定永远走 kernel，禁止文本比对（REQUIREMENTS §2.8）。

use super::render::decl_at;
use sokonanoda_front::compile::{
    CompileOptions, DeclState, DeclStatus, DocumentReport, PreludeMode,
};
use sokonanoda_front::suggest::{self, SuggestionKind};
use std::collections::HashMap;
use tower_lsp::lsp_types::*;

/// The code-action entry: suggestions in order (exact → rfl → refine →
/// intro), the first one marked preferred.
pub(crate) fn code_actions(
    uri: Url,
    text: &str,
    mode: PreludeMode,
    report: &DocumentReport,
    pos: Position,
) -> Option<CodeActionResponse> {
    let d = decl_at(&report.decls, pos.line, pos.character)?;
    if d.status != DeclStatus::Open {
        return None;
    }
    let options = CompileOptions { prelude: mode };
    // 建议判定需要看到声明本身：给到该声明结束为止的文本。
    let src = &text[..d.span.end.offset.min(text.len())];
    let mut actions: Vec<CodeActionOrCommand> = Vec::new();
    for suggestion in suggest::suggest(src, &options, d) {
        match &suggestion.kind {
            SuggestionKind::Exact { binder, hole } => {
                let Some(range) = hole_range_at(text, d, *hole) else {
                    continue;
                };
                let title = if d.holes.len() <= 1 {
                    format!("exact {binder}（用假设 {binder} 直接结束证明）")
                } else {
                    format!("exact {binder}（用假设 {binder} 补第 {} 个洞）", hole + 1)
                };
                push_action(
                    &mut actions,
                    title,
                    edit_on_hole(uri.clone(), range, binder.clone()),
                );
            }
            SuggestionKind::Rfl { term } => {
                let Some(range) = hole_range(text, d) else {
                    continue;
                };
                push_action(
                    &mut actions,
                    "Eq.refl …（两边本来就是同一个值，rfl 即可）".to_string(),
                    edit_on_hole(uri.clone(), range, term.clone()),
                );
            }
            SuggestionKind::Refine => {
                let Some(edit) = refine_edit(uri.clone(), text, d) else {
                    continue;
                };
                let template = d.refine_template.clone().unwrap_or_default();
                push_action(
                    &mut actions,
                    format!("refine {template}（按构造子拆分子目标）"),
                    edit,
                );
            }
            SuggestionKind::Intro => {
                let Some(goal_text) = &d.goal else {
                    continue;
                };
                let Some(intros) = intro_count(goal_text) else {
                    continue;
                };
                let Some(edit) = intro_edit(uri.clone(), text, d, goal_text) else {
                    continue;
                };
                push_action(
                    &mut actions,
                    format!("intro {intros} 个 binder（把证明写成 lambda 的第一步）"),
                    edit,
                );
            }
        }
    }
    if actions.is_empty() {
        return None;
    }
    // 恰好第一条标 preferred（TryThis / rust-analyzer 惯例）。
    if let Some(CodeActionOrCommand::CodeAction(first)) = actions.first_mut() {
        first.is_preferred = Some(true);
    }
    Some(actions)
}

fn push_action(actions: &mut Vec<CodeActionOrCommand>, title: String, edit: WorkspaceEdit) {
    actions.push(CodeActionOrCommand::CodeAction(CodeAction {
        title,
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: None,
        edit: Some(edit),
        command: None,
        is_preferred: None,
        disabled: None,
        data: None,
    }));
}

/// intro 层数：goal 是 Forall/Arrow 时可剥的 binder 数；`None` 表示不适用。
fn intro_count(goal_text: &str) -> Option<usize> {
    let goal = sokonanoda_front::proof::parse_expr_text(goal_text).ok()?;
    let intros = match &goal {
        sokonanoda_front::Expr::Forall { binders, .. } => binders.len(),
        sokonanoda_front::Expr::Arrow { .. } => 1,
        _ => 0,
    };
    (intros > 0).then_some(intros)
}

/// Locate the `hole`-th `???` inside the declaration (0-based, from the
/// walk-derived `DeclState.holes`); returns the 0-based LSP range covering
/// the hole. The server derives hole positions from the walk — never by
/// scanning text.
pub(crate) fn hole_range_at(text: &str, d: &DeclState, hole: usize) -> Option<Range> {
    let span = d.holes.get(hole)?;
    let (hl, hc) = offset_to_line_col(text, span.start.offset);
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

/// The first hole (smallest offset; the walk orders holes by offset).
pub(crate) fn hole_range(text: &str, d: &DeclState) -> Option<Range> {
    let index = d
        .holes
        .iter()
        .enumerate()
        .min_by_key(|(_, span)| span.start.offset)
        .map(|(i, _)| i)?;
    hole_range_at(text, d, index)
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

/// Replace the first `???` with the constructor skeleton the walk recovered
/// from the document (auto-filled parameters + one `???` per proof field),
/// e.g. `And.intro a b ??? ???`. The suggestion is structural (from the
/// declaration's own axiom/ctor shape); the kernel stays the judge for
/// whatever the learner writes into the sub-holes.
pub(crate) fn refine_edit(uri: Url, text: &str, d: &DeclState) -> Option<WorkspaceEdit> {
    let template = d.refine_template.as_deref()?;
    let range = hole_range(text, d)?;
    Some(edit_on_hole(uri, range, template.to_string()))
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

#[cfg(test)]
mod tests {
    use crate::testutil::{
        call, did_open, handshake, lsp_pos, offset_of, position_json, shutdown, test_service,
        wait_diagnostics, URI,
    };
    use crate::Backend;
    use serde_json::json;
    use tower_lsp::jsonrpc::Request as RpcRequest;
    use tower_lsp::lsp_types::*;
    use tower_lsp::{ClientSocket, LspService};

    /// handshake + didOpen + 等到首轮诊断（报告已就绪）。
    async fn opened(src: &str) -> (LspService<Backend>, ClientSocket) {
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, src).await;
        let _ = wait_diagnostics(&mut socket, "diagnostics").await;
        (service, socket)
    }

    /// 光标放在第一个 `???` 上的 codeAction 响应（只保留 CodeAction）。
    async fn code_actions_for(service: &mut LspService<Backend>, src: &str) -> Vec<CodeAction> {
        let hole_start = lsp_pos(src, offset_of(src, "???"));
        let result = call(
            service,
            RpcRequest::build("textDocument/codeAction")
                .params(json!({
                    "textDocument": {"uri": URI},
                    "range": {"start": position_json(hole_start), "end": position_json(hole_start)},
                    "context": {"diagnostics": []},
                }))
                .id(90)
                .finish(),
        )
        .await
        .expect("codeAction must answer");
        let actions: Option<CodeActionResponse> =
            serde_json::from_value(result).expect("valid CodeActionResponse");
        actions
            .expect("code actions")
            .into_iter()
            .filter_map(|a| match a {
                CodeActionOrCommand::CodeAction(action) => Some(action),
                CodeActionOrCommand::Command(_) => None,
            })
            .collect()
    }

    /// 第一个编辑的 (起点, 替换文本)。
    fn first_edit(action: &CodeAction) -> (Position, String) {
        let edit = action.edit.as_ref().expect("action carries an edit");
        let changes = edit.changes.as_ref().expect("changes map");
        let edits = changes.values().next().expect("one document's edits");
        let edit = edits.first().expect("one edit");
        (edit.range.start, edit.new_text.clone())
    }

    fn titles_of(actions: &[CodeAction]) -> Vec<&str> {
        actions.iter().map(|a| a.title.as_str()).collect()
    }

    const SPINE_WHOLE_DOC: &str = "axiom And : Prop -> Prop -> Prop\n\
axiom And.intro : (a : Prop) -> (b : Prop) -> a -> b -> And a b\n\
theorem t : (a : Prop) -> (b : Prop) -> (whole : And a b) -> (ha : a) -> (hb : b) -> And a b := \
fun (a : Prop) => fun (b : Prop) => fun (whole : And a b) => fun (ha : a) => fun (hb : b) => \
And.intro a b ??? ???\n";

    #[tokio::test]
    async fn code_action_spine_hole_exact_targets_its_own_hole() {
        let (mut service, _socket) = opened(SPINE_WHOLE_DOC).await;
        let actions = code_actions_for(&mut service, SPINE_WHOLE_DOC).await;
        let first_hole = lsp_pos(SPINE_WHOLE_DOC, offset_of(SPINE_WHOLE_DOC, "???"));
        let second_hole = lsp_pos(
            SPINE_WHOLE_DOC,
            SPINE_WHOLE_DOC.rfind("???").expect("2nd hole"),
        );
        // 假设 ha : a 恰是第 1 个子洞的期望类型 → 洞位正确。
        let ha = actions
            .iter()
            .find(|a| a.title.contains("exact ha"))
            .expect("exact ha offered for the first sub-hole");
        let (start, text) = first_edit(ha);
        assert_eq!(start, first_hole, "ha targets the first spine hole");
        assert_eq!(text, "ha");
        assert!(ha.title.contains("补第 1 个洞"), "title: {:?}", ha.title);
        // 假设 hb : b 恰是第 2 个子洞的期望类型。
        let hb = actions
            .iter()
            .find(|a| a.title.contains("exact hb"))
            .expect("exact hb offered for the second sub-hole");
        let (start, text) = first_edit(hb);
        assert_eq!(start, second_hole, "hb targets the second spine hole");
        assert_eq!(text, "hb");
        assert!(hb.title.contains("补第 2 个洞"), "title: {:?}", hb.title);
        // 回归：匹配外层 goal 的假设不得被塞进子洞。
        assert!(
            !actions.iter().any(|a| a.title.contains("exact whole")),
            "outer-goal hypothesis must not be suggested for a sub-hole: {:?}",
            titles_of(&actions)
        );
        shutdown(&mut service).await;
    }

    const EQ_GOAL_DOC: &str =
        "theorem eq_t : (a : Nat) -> Eq.{1} Nat a a := fun (a : Nat) => ???\n";

    #[tokio::test]
    async fn code_action_rfl_only_for_eq_goals() {
        let (mut service, _socket) = opened(EQ_GOAL_DOC).await;
        let actions = code_actions_for(&mut service, EQ_GOAL_DOC).await;
        let rfl = actions
            .iter()
            .find(|a| a.title.contains("rfl"))
            .expect("an Eq goal gets a kernel-verified rfl");
        assert!(
            rfl.title.contains("两边本来就是同一个值"),
            "title: {:?}",
            rfl.title
        );
        let (start, text) = first_edit(rfl);
        assert_eq!(text, "Eq.refl.{1} Nat a", "kernel-verified rfl term");
        assert_eq!(
            start,
            lsp_pos(EQ_GOAL_DOC, offset_of(EQ_GOAL_DOC, "???")),
            "rfl targets the hole"
        );
        shutdown(&mut service).await;

        // 非 Eq goal 不出 rfl。
        let arrow_src = "example : Prop -> Prop := ???\n";
        let (mut service, _socket) = opened(arrow_src).await;
        let actions = code_actions_for(&mut service, arrow_src).await;
        assert!(
            !actions.iter().any(|a| a.title.contains("rfl")),
            "non-Eq goal must not get rfl: {:?}",
            titles_of(&actions)
        );
        shutdown(&mut service).await;
    }

    const TRIPLE_DOC: &str = "axiom And : Prop -> Prop -> Prop\n\
axiom And.intro : (a : Prop) -> (b : Prop) -> a -> b -> And a b\n\
theorem t : (a : Prop) -> (b : Prop) -> (k : a -> b -> And a b) -> a -> b -> And a b := \
fun (a : Prop) => fun (b : Prop) => fun (k : a -> b -> And a b) => ???\n";

    #[tokio::test]
    async fn code_action_marks_exactly_the_first_as_preferred() {
        let (mut service, _socket) = opened(TRIPLE_DOC).await;
        let actions = code_actions_for(&mut service, TRIPLE_DOC).await;
        assert_eq!(
            actions.len(),
            3,
            "exact + refine + intro: {:?}",
            titles_of(&actions)
        );
        assert_eq!(actions[0].is_preferred, Some(true), "first is preferred");
        for action in &actions[1..] {
            assert_eq!(action.is_preferred, None, "only the first is preferred");
        }
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn code_action_orders_exact_refine_intro() {
        let (mut service, _socket) = opened(TRIPLE_DOC).await;
        let actions = code_actions_for(&mut service, TRIPLE_DOC).await;
        let titles = titles_of(&actions);
        assert!(titles[0].contains("exact"), "exact first: {titles:?}");
        assert!(titles[1].contains("refine"), "refine second: {titles:?}");
        assert!(titles[2].contains("intro"), "intro last: {titles:?}");
        shutdown(&mut service).await;
    }
}

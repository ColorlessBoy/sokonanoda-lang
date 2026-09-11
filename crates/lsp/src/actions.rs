//! Code-action helpers: map next-step suggestions into quick-fix text edits
//! for open exercises (docs/design/hints-suggestions.md §4.3).
//!
//! 建议生成在 `front::suggest`（kernel 验证优先：exact → rfl → refine →
//! intro；第一条标 preferred）。这里只做映射：
//! * 洞位来自 walk 的 `DeclState.holes`（按洞下标取，绝不扫描文本）；
//! * 替换文本来自 kernel 验证过的候选（exact 的假设名、rfl 的项）或结构
//!   模板（refine / intro）；
//! * 判定永远走 kernel，禁止文本比对（REQUIREMENTS §2.8）。
//!
//! kernel 拒绝的失败声明（`DeclStatus::Failed`）没有洞，编辑目标是整个
//! 值位（tokenize 定位 `:=` 与值首）。`front::suggest` 的失败声明建议梯子：
//! kernel 验证过的 `Eq.refl` 整值替换（Eq 形状声明）→ 保留已写 lambda
//! 前缀的部分重置（Reset）→ 整值重启骨架（Restart）——都映射为值位
//! 整体替换（docs/design/kernel-taxonomy.md §2）。

use super::render::decl_at;
use sokonanoda_front::compile::{
    CompileOptions, DeclState, DeclStatus, DocumentReport, PreludeMode,
};
use sokonanoda_front::suggest::{self, SuggestionKind};
use sokonanoda_front::{tokenize, TokenKind};
use std::collections::HashMap;
use tower_lsp::lsp_types::*;

/// The code-action entry: suggestions in order (exact → rfl → refine →
/// intro), the first one marked preferred. A kernel-rejected declaration
/// offers at most one action: the restart skeleton for the whole value.
pub(crate) fn code_actions(
    uri: Url,
    text: &str,
    mode: PreludeMode,
    report: &DocumentReport,
    pos: Position,
) -> Option<CodeActionResponse> {
    let d = decl_at(&report.decls, pos.line, pos.character)?;
    if d.status == DeclStatus::Checked {
        return None;
    }
    let options = CompileOptions { prelude: mode };
    // 建议判定需要看到声明本身：给到该声明结束为止的文本。
    let src = &text[..d.span.end.offset.min(text.len())];
    let mut actions: Vec<CodeActionOrCommand> = Vec::new();
    if d.status == DeclStatus::Failed {
        if d.error.is_some() {
            let decl_src =
                &text[d.span.start.offset.min(text.len())..d.span.end.offset.min(text.len())];
            let suggestions = suggest::suggest(src, Some(decl_src), &options, d);
            // 三类失败声明建议都替换整个值位。
            if let Some(range) = value_range_at(text, d) {
                for suggestion in suggestions {
                    match &suggestion.kind {
                        SuggestionKind::Rfl { term } => {
                            push_action(
                                &mut actions,
                                "Eq.refl …（内核验证：两边就是同一个值，直接替换）".to_string(),
                                edit_on_hole(uri.clone(), range, term.clone()),
                            );
                        }
                        SuggestionKind::Reset { new_text } => {
                            push_action(
                                &mut actions,
                                format!(
                                    "保留 fun 前缀，只重置主体为 sorry：{}（从剩余目标继续）",
                                    restart_summary(new_text)
                                ),
                                edit_on_hole(uri.clone(), range, new_text.clone()),
                            );
                        }
                        SuggestionKind::Restart { skeleton } => {
                            push_action(
                                &mut actions,
                                format!(
                                    "用目标形态重启：{}（先搭骨架，内核逐层判）",
                                    restart_summary(skeleton)
                                ),
                                edit_on_hole(uri.clone(), range, skeleton.clone()),
                            );
                        }
                        _ => {}
                    }
                }
            }
        }
    } else {
        for suggestion in suggest::suggest(src, None, &options, d) {
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
                // 失败声明专属的建议不会出现在开放练习里。
                SuggestionKind::Reset { .. } | SuggestionKind::Restart { .. } => {}
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

/// 失败声明标题里的骨架摘要：超 40 字符截断加 `…`。
fn restart_summary(skeleton: &str) -> String {
    let mut out: String = skeleton.chars().take(RESTART_SUMMARY_CHARS).collect();
    if skeleton.chars().count() > RESTART_SUMMARY_CHARS {
        out.push('…');
    }
    out
}

const RESTART_SUMMARY_CHARS: usize = 40;

/// The failed declaration's value span: from the first token after `:=`
/// (the lexer skips whitespace/comments) to the declaration span's end —
/// the parser ends the span at the value's last token, so the trailing
/// newline is already excluded. Multi-line values are replaced as a whole.
/// Located by tokenizing the declaration command; never by scanning text.
fn value_range_at(text: &str, d: &DeclState) -> Option<Range> {
    let start = d.span.start.offset.min(text.len());
    let end = d.span.end.offset.min(text.len());
    let decl_src = text.get(start..end)?;
    let tokens = tokenize(decl_src).ok()?;
    let colon_eq = tokens.iter().position(|t| t.kind == TokenKind::ColonEq)?;
    let value_tok = tokens.get(colon_eq + 1)?;
    if value_tok.kind == TokenKind::Eof {
        return None;
    }
    range_at_offsets(text, start + value_tok.span.start.offset, end)
}

/// Byte offsets → 0-based LSP range（`offset_to_line_col` 是 1 基）。
fn range_at_offsets(text: &str, start: usize, end: usize) -> Option<Range> {
    let (sl, sc) = offset_to_line_col(text, start);
    let (el, ec) = offset_to_line_col(text, end);
    Some(Range {
        start: Position {
            line: (sl - 1) as u32,
            character: (sc - 1) as u32,
        },
        end: Position {
            line: (el - 1) as u32,
            character: (ec - 1) as u32,
        },
    })
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

/// Locate the `hole`-th `sorry` inside the declaration (0-based, from the
/// walk-derived `DeclState.holes`); returns the 0-based LSP range covering
/// the hole. The server derives hole positions from the walk — never by
/// scanning text.
pub(crate) fn hole_range_at(text: &str, d: &DeclState, hole: usize) -> Option<Range> {
    let span = d.holes.get(hole)?;
    let (hl, hc) = offset_to_line_col(text, span.start.offset);
    let (el, ec) = offset_to_line_col(text, span.end.offset);
    // `offset_to_line_col` is 1-based (matching our Spans); LSP wants 0-based.
    Some(Range {
        start: Position {
            line: (hl - 1) as u32,
            character: (hc - 1) as u32,
        },
        end: Position {
            line: (el - 1) as u32,
            character: (ec - 1) as u32,
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

/// Replace the first `sorry` inside the declaration with `fun (x : T) => sorry`,
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
    //   fun (x : T) => sorry
    let replacement = state.lambda_text();
    Some(edit_on_hole(uri, range, replacement))
}

/// Replace the first `sorry` with the constructor skeleton the walk recovered
/// from the document (auto-filled parameters + one `sorry` per proof field),
/// e.g. `And.intro a b sorry sorry`. The suggestion is structural (from the
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

    /// 光标放在第一个 `sorry` 上的 codeAction 响应（只保留 CodeAction）。
    async fn code_actions_for(service: &mut LspService<Backend>, src: &str) -> Vec<CodeAction> {
        let hole_start = lsp_pos(src, offset_of(src, "sorry"));
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

    /// 第一个编辑的完整 (range, 替换文本)。
    fn first_edit_full(action: &CodeAction) -> (Range, String) {
        let edit = action.edit.as_ref().expect("action carries an edit");
        let changes = edit.changes.as_ref().expect("changes map");
        let edits = changes.values().next().expect("one document's edits");
        let edit = edits.first().expect("one edit");
        (edit.range, edit.new_text.clone())
    }

    /// 光标放在 `offset` 上的 codeAction 响应（`None` = 无建议）。
    async fn code_actions_at(
        service: &mut LspService<Backend>,
        src: &str,
        offset: usize,
    ) -> Option<Vec<CodeAction>> {
        let pos = lsp_pos(src, offset);
        let result = call(
            service,
            RpcRequest::build("textDocument/codeAction")
                .params(json!({
                    "textDocument": {"uri": URI},
                    "range": {"start": position_json(pos), "end": position_json(pos)},
                    "context": {"diagnostics": []},
                }))
                .id(80)
                .finish(),
        )
        .await
        .expect("codeAction must answer");
        let actions: Option<CodeActionResponse> =
            serde_json::from_value(result).expect("valid CodeActionResponse");
        actions.map(|resp| {
            resp.into_iter()
                .filter_map(|a| match a {
                    CodeActionOrCommand::CodeAction(action) => Some(action),
                    CodeActionOrCommand::Command(_) => None,
                })
                .collect()
        })
    }

    fn titles_of(actions: &[CodeAction]) -> Vec<&str> {
        actions.iter().map(|a| a.title.as_str()).collect()
    }

    const SPINE_WHOLE_DOC: &str = "axiom And : Prop -> Prop -> Prop\n\
axiom And.intro : (a : Prop) -> (b : Prop) -> a -> b -> And a b\n\
theorem t : (a : Prop) -> (b : Prop) -> (whole : And a b) -> (ha : a) -> (hb : b) -> And a b := \
fun (a : Prop) => fun (b : Prop) => fun (whole : And a b) => fun (ha : a) => fun (hb : b) => \
And.intro a b sorry sorry\n";

    #[tokio::test]
    async fn code_action_spine_hole_exact_targets_its_own_hole() {
        let (mut service, _socket) = opened(SPINE_WHOLE_DOC).await;
        let actions = code_actions_for(&mut service, SPINE_WHOLE_DOC).await;
        let first_hole = lsp_pos(SPINE_WHOLE_DOC, offset_of(SPINE_WHOLE_DOC, "sorry"));
        let second_hole = lsp_pos(
            SPINE_WHOLE_DOC,
            SPINE_WHOLE_DOC.rfind("sorry").expect("2nd hole"),
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
        "theorem eq_t : (a : Nat) -> Eq.{1} Nat a a := fun (a : Nat) => sorry\n";

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
            lsp_pos(EQ_GOAL_DOC, offset_of(EQ_GOAL_DOC, "sorry")),
            "rfl targets the hole"
        );
        shutdown(&mut service).await;

        // 非 Eq goal 不出 rfl。
        let arrow_src = "example : Prop -> Prop := sorry\n";
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
fun (a : Prop) => fun (b : Prop) => fun (k : a -> b -> And a b) => sorry\n";

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

    // ---- 失败声明的重启骨架（docs/design/kernel-taxonomy.md §2）----

    const FAILED_DEF_EQ: &str = "example : (a : Prop) -> a -> a := fun (x : Prop) => 1\n";

    #[tokio::test]
    async fn code_action_restarts_failed_decl_over_the_whole_value_span() {
        let (mut service, _socket) = opened(FAILED_DEF_EQ).await;
        let cursor = offset_of(FAILED_DEF_EQ, "=> 1");
        let actions = code_actions_at(&mut service, FAILED_DEF_EQ, cursor)
            .await
            .expect("a kernel-rejected decl must offer the restarts");
        // 答案以 lambda 开头：Reset（保留前缀）在前，Restart（整值骨架）在后。
        assert_eq!(
            actions.len(),
            2,
            "reset + restart: {:?}",
            titles_of(&actions)
        );
        let reset = &actions[0];
        assert!(
            reset.title.contains("保留 fun 前缀"),
            "title: {:?}",
            reset.title
        );
        assert!(
            reset.title.contains("从剩余目标继续"),
            "title: {:?}",
            reset.title
        );
        assert_eq!(
            reset.is_preferred,
            Some(true),
            "the first failed-decl action is preferred"
        );
        let (_, reset_text) = first_edit_full(reset);
        assert_eq!(
            reset_text, "fun (x : Prop) => sorry",
            "the reset keeps the written lambda prefix"
        );
        let restart = &actions[1];
        assert!(
            restart.title.contains("用目标形态重启"),
            "title: {:?}",
            restart.title
        );
        assert!(
            restart
                .title
                .contains("fun (a : Prop) => fun (x : a) => sorry"),
            "the skeleton summary is in the title: {:?}",
            restart.title
        );
        assert!(
            restart.title.contains("先搭骨架，内核逐层判"),
            "title: {:?}",
            restart.title
        );
        assert_eq!(
            restart.is_preferred, None,
            "only the first action is preferred"
        );
        // 编辑目标 = 整个值位：从 `:=` 后第一个 token 到声明 span 末尾
        // （不含结尾换行），多行值也整体替换。
        let (range, new_text) = first_edit_full(restart);
        assert_eq!(
            range.start,
            lsp_pos(FAILED_DEF_EQ, offset_of(FAILED_DEF_EQ, "fun (x")),
            "the edit starts at the first value token"
        );
        assert_eq!(
            range.end,
            lsp_pos(
                FAILED_DEF_EQ,
                FAILED_DEF_EQ.rfind('\n').expect("trailing newline")
            ),
            "the edit ends at the declaration span's end (before the newline)"
        );
        assert_eq!(new_text, "fun (a : Prop) => fun (x : a) => sorry");
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn code_action_failed_decl_without_peelable_type_gets_none() {
        // kernel 拒绝但类型是 Prop（非 Pi）：没有可剥的望远镜，无建议。
        let src = "def bad : Prop := 1\n";
        let (mut service, _socket) = opened(src).await;
        let actions = code_actions_at(&mut service, src, offset_of(src, "1")).await;
        assert!(
            actions.is_none(),
            "a non-Pi failed decl must get no action: {actions:?}"
        );
        shutdown(&mut service).await;

        // elab 失败（unknown identifier）同样没有可剥的类型：无建议。
        let src = "example : Prop := undefined_name\n";
        let (mut service, _socket) = opened(src).await;
        let actions = code_actions_at(&mut service, src, offset_of(src, "undefined_name")).await;
        assert!(
            actions.is_none(),
            "an elab-failed decl must get no action: {actions:?}"
        );
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn code_action_restart_replaces_multiline_values_as_a_whole() {
        let src = "example : (a : Prop) -> a -> a :=\n  fun (x : Prop) => 1\n";
        let (mut service, _socket) = opened(src).await;
        let cursor = offset_of(src, "=> 1");
        let actions = code_actions_at(&mut service, src, cursor)
            .await
            .expect("a kernel-rejected decl must offer the restarts");
        let restart = actions
            .iter()
            .find(|a| a.title.contains("用目标形态重启"))
            .expect("the whole-value restart action");
        let (range, new_text) = first_edit_full(restart);
        assert_eq!(
            range.start,
            lsp_pos(src, offset_of(src, "fun (x")),
            "the edit starts at the first value token, on the value's own line"
        );
        assert_eq!(
            range.end,
            lsp_pos(src, src.rfind('\n').expect("trailing newline")),
            "the edit ends at the declaration span's end"
        );
        assert_eq!(new_text, "fun (a : Prop) => fun (x : a) => sorry");
        // 部分重启同样覆盖整个值位（换行后的缩进不进 new_text）。
        let reset = actions
            .iter()
            .find(|a| a.title.contains("保留 fun 前缀"))
            .expect("the prefix-preserving reset action");
        let (_, reset_text) = first_edit_full(reset);
        assert_eq!(reset_text, "fun (x : Prop) => sorry");
        shutdown(&mut service).await;
    }

    // ---- 失败声明的 kernel 验证 rfl（Eq 形状声明）----

    const FAILED_EQ_LAMBDA: &str = "example : Eq.{1} Nat 2 2 := fun (x : Nat) => 3\n";

    #[tokio::test]
    async fn code_action_failed_eq_decl_offers_kernel_verified_rfl_first() {
        let (mut service, _socket) = opened(FAILED_EQ_LAMBDA).await;
        let cursor = offset_of(FAILED_EQ_LAMBDA, "=> 3");
        let actions = code_actions_at(&mut service, FAILED_EQ_LAMBDA, cursor)
            .await
            .expect("a kernel-rejected Eq decl must offer the verified rfl");
        assert_eq!(actions.len(), 2, "rfl + reset: {:?}", titles_of(&actions));
        let rfl = &actions[0];
        assert!(rfl.title.contains("内核验证"), "title: {:?}", rfl.title);
        assert!(rfl.title.contains("直接替换"), "title: {:?}", rfl.title);
        assert_eq!(rfl.is_preferred, Some(true), "the verified rfl is first");
        // 编辑目标 = 整个值位，替换文本恰为 kernel 验证过的 rfl 项。
        let (range, new_text) = first_edit_full(rfl);
        assert_eq!(new_text, "Eq.refl.{1} Nat 2");
        assert_eq!(
            range.start,
            lsp_pos(FAILED_EQ_LAMBDA, offset_of(FAILED_EQ_LAMBDA, "fun (x")),
            "the edit starts at the first value token"
        );
        assert_eq!(
            range.end,
            lsp_pos(
                FAILED_EQ_LAMBDA,
                FAILED_EQ_LAMBDA.rfind('\n').expect("trailing newline")
            ),
            "the edit ends at the declaration span's end"
        );
        // Reset 其次：保留已写的 lambda 前缀。
        let reset = &actions[1];
        assert!(
            reset.title.contains("保留 fun 前缀"),
            "title: {:?}",
            reset.title
        );
        assert_eq!(reset.is_preferred, None, "only the first is preferred");
        let (_, reset_text) = first_edit_full(reset);
        assert_eq!(reset_text, "fun (x : Nat) => sorry");
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn code_action_failed_eq_decl_with_non_lambda_answer_gets_only_rfl() {
        let src = "example : Eq.{1} Nat 2 2 := 3\n";
        let (mut service, _socket) = opened(src).await;
        let cursor = offset_of(src, "3");
        let actions = code_actions_at(&mut service, src, cursor)
            .await
            .expect("a kernel-rejected Eq decl must offer the verified rfl");
        assert_eq!(
            actions.len(),
            1,
            "the rfl replaces the whole value; no restart applies: {:?}",
            titles_of(&actions)
        );
        let rfl = &actions[0];
        assert!(rfl.title.contains("内核验证"), "title: {:?}", rfl.title);
        assert_eq!(rfl.is_preferred, Some(true));
        let (range, new_text) = first_edit_full(rfl);
        assert_eq!(new_text, "Eq.refl.{1} Nat 2");
        assert_eq!(
            range.start,
            lsp_pos(src, offset_of(src, "3")),
            "the edit starts at the value"
        );
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn code_action_failed_eq_decl_kernel_rejected_rfl_is_dropped() {
        // 2 ≢ 3：候选被内核拒绝，Eq 形状声明又不给重启——无建议。
        let src = "example : Eq.{1} Nat 2 3 := 5\n";
        let (mut service, _socket) = opened(src).await;
        let cursor = offset_of(src, "5");
        let actions = code_actions_at(&mut service, src, cursor).await;
        assert!(
            actions.is_none(),
            "a kernel-rejected rfl candidate must not be offered: {actions:?}"
        );
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn code_action_non_lambda_answer_gets_only_restart() {
        let src = "example : (a : Prop) -> a -> a := 1\n";
        let (mut service, _socket) = opened(src).await;
        let cursor = offset_of(src, "1");
        let actions = code_actions_at(&mut service, src, cursor)
            .await
            .expect("a kernel-rejected decl must offer the restart");
        assert_eq!(
            actions.len(),
            1,
            "a non-lambda answer has no prefix to keep: {:?}",
            titles_of(&actions)
        );
        let restart = &actions[0];
        assert!(
            restart.title.contains("用目标形态重启"),
            "title: {:?}",
            restart.title
        );
        assert_eq!(restart.is_preferred, Some(true));
        let (_, new_text) = first_edit_full(restart);
        assert_eq!(new_text, "fun (a : Prop) => fun (x : a) => sorry");
        shutdown(&mut service).await;
    }

    #[test]
    fn restart_summary_truncates_at_40_chars() {
        let short = "fun (a : Prop) => fun (x : a) => sorry";
        assert_eq!(
            super::restart_summary(short),
            short,
            "short skeletons pass through"
        );
        let long = "fun (a : Prop) => fun (b : Prop) => fun (c : Prop) => sorry";
        let summary = super::restart_summary(long);
        assert!(summary.ends_with('…'), "truncated summaries end with …");
        assert_eq!(
            summary.chars().count(),
            40 + 1,
            "40 characters plus the ellipsis"
        );
        assert!(long.starts_with(summary.trim_end_matches('…')));
    }
}

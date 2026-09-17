use super::*;

#[tokio::test]
async fn code_lens_reflects_exercise_status() {
    let src = format!("def ok : Prop -> Prop := fun (x : Prop) => x\n{EXERCISE}");
    let (mut service, mut socket) = test_service();
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
async fn code_lens_ranges_match_each_declaration() {
    // codeLens 的 range 必须精确覆盖每个声明（checked def 与 open
    // exercise 各一），命令 id 由扩展消费（sokonanoda.status）。
    const CHECKED: &str = "def ok : Prop -> Prop := fun (x : Prop) => x";
    const OPEN: &str = "example : Prop -> Prop := sorry";
    let src = format!("{CHECKED}\n{OPEN}\n");
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, &src).await;
    let _ = wait_diagnostics(&mut socket, "codeLens diagnostics").await;

    let result = call(
        &mut service,
        RpcRequest::build("textDocument/codeLens")
            .params(json!({"textDocument": {"uri": URI}}))
            .id(6)
            .finish(),
    )
    .await
    .expect("codeLens must answer");
    let lenses: Option<Vec<CodeLens>> = serde_json::from_value(result).expect("valid CodeLens");
    let lenses = lenses.expect("code lenses must be returned");
    assert_eq!(lenses.len(), 2, "one lens per declaration: {lenses:?}");

    let checked = &lenses[0];
    let command = checked.command.as_ref().expect("lens carries a command");
    assert_eq!(command.command, "sokonanoda.status");
    assert!(
        command.title.contains("solved"),
        "checked def lens title: {:?}",
        command.title
    );
    assert_eq!(
        checked.range.start,
        lsp_pos(&src, 0),
        "checked lens starts at the declaration"
    );
    assert_eq!(
        checked.range.end,
        lsp_pos(&src, CHECKED.len()),
        "checked lens ends at the declaration's last token"
    );

    let open_start = CHECKED.len() + 1;
    let open = &lenses[1];
    let command = open.command.as_ref().expect("lens carries a command");
    assert_eq!(command.command, "sokonanoda.status");
    assert!(
        command.title.contains("exercise: open"),
        "open exercise lens title: {:?}",
        command.title
    );
    assert_eq!(
        open.range.start,
        lsp_pos(&src, open_start),
        "open lens starts at the exercise declaration"
    );
    assert_eq!(
        open.range.end,
        lsp_pos(&src, open_start + OPEN.len()),
        "open lens ends at the exercise's last token"
    );
    shutdown(&mut service).await;
}

#[tokio::test]
async fn code_action_offers_intro_on_open_exercise() {
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, EXERCISE).await;
    let _ = wait_diagnostics(&mut socket, "didOpen diagnostics").await;

    let hole = offset_of(EXERCISE, "sorry");
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
    assert!(action.title.contains("引入"), "title: {:?}", action.title);
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
        "sorry".len() as u32,
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
        text_edit.new_text.ends_with("sorry"),
        "intro replacement must keep the hole, got {:?}",
        text_edit.new_text
    );
    shutdown(&mut service).await;
}

#[tokio::test]
async fn code_action_offers_exact_for_matching_hypothesis() {
    let src = "example : (a : Prop) -> a -> a := fun (a : Prop) => fun (h : a) => sorry\n";
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, src).await;
    let _ = wait_diagnostics(&mut socket, "partial hole diagnostics").await;

    let hole = offset_of(src, "sorry");
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
    let src = "example : Prop -> Prop := sorry\n";
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, src).await;
    let _ = wait_diagnostics(&mut socket, "hole diagnostics").await;

    let hole = offset_of(src, "sorry");
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
        titles.iter().any(|t| t.contains("引入")),
        "intro must still be offered: {titles:?}"
    );
    assert!(
        !titles.iter().any(|t| t.contains("exact")),
        "no exact action when no hypothesis matches: {titles:?}"
    );
    let _ = hole_start;
}

// F8 语义着色：能力 + UTF-16 编码 + 端到端分类。

#[tokio::test]
async fn code_action_exact_uses_kernel_defeq_not_text_match() {
    // h 的类型是 `a -> False`，剩余目标渲染为 `Not a`：文本不同但
    // definitional equal —— 文本比对给不出建议，kernel 判定可以。
    let src = "axiom False : Prop\n\
               def Not : Prop -> Prop := fun (a : Prop) => a -> False\n\
               example : (a : Prop) -> (a -> False) -> Not a := fun (a : Prop) => fun (h : a -> False) => sorry\n";
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, src).await;
    let _ = wait_diagnostics(&mut socket, "defeq exact diagnostics").await;

    let hole = offset_of(src, "sorry");
    let hole_start = lsp_pos(src, hole);
    let result = call(
        &mut service,
        RpcRequest::build("textDocument/codeAction")
            .params(json!({
                "textDocument": {"uri": URI},
                "range": {"start": position_json(hole_start), "end": position_json(hole_start)},
                "context": {"diagnostics": []},
            }))
            .id(42)
            .finish(),
    )
    .await
    .expect("codeAction must answer");
    let actions: Option<CodeActionResponse> =
        serde_json::from_value(result).expect("valid CodeActionResponse");
    let actions = actions.expect("kernel-defeq hypothesis must yield an action");
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
        .expect("an `exact h` action must be offered (kernel judges a -> False ≡ Not a)");
    let edit = exact.edit.as_ref().expect("exact action carries an edit");
    let edits = edit
        .changes
        .as_ref()
        .and_then(|c| c.get(&Url::parse(URI).expect("uri")))
        .expect("edit targets our uri");
    assert_eq!(edits[0].new_text, "h");
}

#[tokio::test]
async fn code_action_offers_kernel_shaped_refine_skeleton() {
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, AND_EXERCISE).await;
    let _ = wait_diagnostics(&mut socket, "refine diagnostics").await;

    let actions = code_actions_for(&mut service, AND_EXERCISE).await;
    let refine = actions
        .iter()
        .find(|a| a.title.contains("refine And.intro a b sorry sorry"))
        .expect("refine skeleton with auto-filled parameters must be offered");
    let edit = refine.edit.as_ref().expect("refine carries an edit");
    let edits = edit
        .changes
        .as_ref()
        .and_then(|c| c.get(&Url::parse(URI).expect("uri")))
        .expect("edit targets our uri");
    assert_eq!(edits[0].new_text, "And.intro a b sorry sorry");
    shutdown(&mut service).await;
}

#[tokio::test]
async fn funapply_expansion_is_not_offered_outside_its_own_line() {
    // 与 `intro` 同一条边界：跨行不命中，免得在后面的声明上误弹。
    let src = "axiom P : Prop\naxiom Q : Prop\ntheorem t (h : Q -> P) : P :=\n  funapply h\n\ntheorem u : P := h sorry\n";
    let (mut service, _socket) = open_and_wait(src).await;
    // 光标落在下下个声明上：不该再给 `funapply` 的展开项。
    let items = request_completions_at(&mut service, lsp_pos(src, offset_of(src, "h sorry"))).await;
    assert!(
        items
            .iter()
            .all(|i| i.filter_text.as_deref() != Some("funapply")),
        "no funapply expansion on a later declaration: {items:?}"
    );
    shutdown(&mut service).await;
}

// ── 性能测试（I13-S5c）：阈值断言抓交互级回归 ──────────────────

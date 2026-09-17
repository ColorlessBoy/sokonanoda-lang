use super::*;

#[tokio::test]
async fn hover_on_bracket_shows_enclosing_expression_type() {
    // 学习者需求：光标在括号上能看到内容。`(a : Prop)` 这种 binder
    // 标注组没有单一表达式行，回退显示组内最大的行（`Prop : Type 0`）；
    // 真正的表达式组 `(And.right a (Not a) h)` 由专门的括号测试覆盖。
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, DEMO_K).await;
    let _ = wait_diagnostics(&mut socket, "bracket hover diagnostics").await;

    // 光标放在 `(a : Prop)` 的 `(` 上（第 0 行 offset 17）。
    let paren = DEMO_K.find('(').expect("paren exists");
    let pos = lsp_pos(DEMO_K, paren);
    let result = call(
        &mut service,
        RpcRequest::build("textDocument/hover")
            .params(json!({
                "textDocument": {"uri": URI},
                "position": position_json(pos),
            }))
            .id(90)
            .finish(),
    )
    .await
    .expect("hover must answer");
    let hover: Option<Hover> = serde_json::from_value(result).expect("valid hover");
    let hover = hover.expect("bracket position must have hover (fallback)");
    let HoverContents::Markup(m) = hover.contents else {
        panic!("expected markup");
    };
    assert!(
        m.value.contains("Prop"),
        "bracket hover should show a type from the group: {:?}",
        m.value
    );
    shutdown(&mut service).await;
}

#[tokio::test]
async fn hover_on_operator_shows_enclosing_type() {
    // 光标在 `->` 上（第 0 行 offset 28）→ 应显示内层函数类型。
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, DEMO_K).await;
    let _ = wait_diagnostics(&mut socket, "operator hover diagnostics").await;

    let arrow = DEMO_K.find("->").expect("arrow exists");
    let pos = lsp_pos(DEMO_K, arrow);
    let result = call(
        &mut service,
        RpcRequest::build("textDocument/hover")
            .params(json!({
                "textDocument": {"uri": URI},
                "position": position_json(pos),
            }))
            .id(91)
            .finish(),
    )
    .await
    .expect("hover must answer");
    let hover: Option<Hover> = serde_json::from_value(result).expect("valid hover");
    let hover = hover.expect("operator position must have hover (proximity fallback)");
    let HoverContents::Markup(m) = hover.contents else {
        panic!("expected markup");
    };
    assert!(
        m.value.contains("->") || m.value.contains("Prop"),
        "operator hover should show type: {:?}",
        m.value
    );
    shutdown(&mut service).await;
}

#[tokio::test]
async fn hover_on_brackets_of_and_right_group_shows_step_type() {
    // 用户样例 1：`(And.right a (Not a) h)` 的 `(` 与 `)` 都显示
    // `And.right a (Not a) h : Not a`——`Not` 保持折叠、`a` 是真名。
    let (mut service, _socket) = open_and_wait(AND_NOT_ABSURD).await;
    let group = AND_NOT_ABSURD.find("(And.right").expect("group exists");
    let close = group_close(AND_NOT_ABSURD, group);
    for offset in [group, close] {
        let markup = hover_markup_at(&mut service, AND_NOT_ABSURD, offset).await;
        assert!(
            markup.contains("And.right a (Not a) h : Not a"),
            "bracket {offset} must show the step type: {markup:?}"
        );
    }
    shutdown(&mut service).await;
}

#[tokio::test]
async fn hover_on_brackets_of_and_left_group_shows_step_type() {
    // 用户样例 2：`(And.left a (Not a) h)` 的括号显示
    // `And.left a (Not a) h : a`。
    let (mut service, _socket) = open_and_wait(AND_NOT_ABSURD).await;
    let group = AND_NOT_ABSURD.find("(And.left").expect("group exists");
    let close = group_close(AND_NOT_ABSURD, group);
    for offset in [group, close] {
        let markup = hover_markup_at(&mut service, AND_NOT_ABSURD, offset).await;
        assert!(
            markup.contains("And.left a (Not a) h : a"),
            "bracket {offset} must show the step type: {markup:?}"
        );
    }
    shutdown(&mut service).await;
}

#[tokio::test]
async fn hover_on_closing_bracket_never_shows_neighbor_signature() {
    // 回归：`)` 上曾因「起点 ±2」邻近回退命中右侧邻居，显示
    // `And.left : forall …`。现在必须显示本组内容。
    let (mut service, _socket) = open_and_wait(AND_NOT_ABSURD).await;
    let group = AND_NOT_ABSURD.find("(And.right").expect("group exists");
    let close = group_close(AND_NOT_ABSURD, group);
    let markup = hover_markup_at(&mut service, AND_NOT_ABSURD, close).await;
    assert!(
        !markup.contains("And.left :"),
        "closing bracket must not leak the neighbor's signature: {markup:?}"
    );
    assert!(
        markup.contains("And.right a (Not a) h"),
        "closing bracket must show its own group: {markup:?}"
    );
    shutdown(&mut service).await;
}

#[tokio::test]
async fn hover_on_nested_parens_shows_innermost_expression() {
    // `((p))` 的四个括号都显示最内层表达式 `p : P`（括号透明）。
    let src = "axiom P : Prop\naxiom p : P\ntheorem t : P := ((p))\n";
    let (mut service, _socket) = open_and_wait(src).await;
    let outer_open = src.find("((").expect("outer paren exists");
    let inner_open = outer_open + 1;
    let inner_close = outer_open + 3;
    let outer_close = outer_open + 4;
    for offset in [outer_open, inner_open, inner_close, outer_close] {
        let markup = hover_markup_at(&mut service, src, offset).await;
        assert!(
            markup.contains("p : P"),
            "bracket {offset} must show the innermost expression: {markup:?}"
        );
    }
    shutdown(&mut service).await;
}

#[tokio::test]
async fn hover_on_bracket_inside_comment_falls_back() {
    // 注释里的括号没有配对语义——不得 panic，也不得张冠李戴；
    // 注释位置安静（None）或给出邻近的合理类型都算正确。
    let src = "axiom P : Prop\naxiom p : P\n-- (unbalanced (note)\ntheorem t : P := p\n";
    let (mut service, _socket) = open_and_wait(src).await;
    let comment_paren = src.find("(note").expect("comment paren exists");
    let markup = hover_markup_opt_at(&mut service, src, comment_paren).await;
    match markup {
        None => {}
        Some(m) => assert!(
            m.contains("p : P") || m.contains("Prop"),
            "comment bracket falls back to a sane hover: {m:?}"
        ),
    }
    shutdown(&mut service).await;
}

// ---- 重构后：binder 名不再整段 lambda 溢出；binder 标注组展示声明；
//      所有 hover 都带高亮范围（range 覆盖光标） ----

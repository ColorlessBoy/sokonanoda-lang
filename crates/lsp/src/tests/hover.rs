use super::*;

#[tokio::test]
async fn hover_returns_inferred_type() {
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, VALID).await;
    let _ = wait_diagnostics(&mut socket, "didOpen diagnostics").await;

    // The `x` occurrence inside the lambda body (0-based position).
    let pos = lsp_pos(VALID, VALID.rfind('x').expect("body `x` exists"));
    let result = call(
        &mut service,
        RpcRequest::build("textDocument/hover")
            .params(json!({
                "textDocument": {"uri": URI},
                "position": position_json(pos),
            }))
            .id(2)
            .finish(),
    )
    .await
    .expect("hover must answer");
    let hover: Option<Hover> = serde_json::from_value(result).expect("valid Hover");
    let hover = hover.expect("hover must resolve inside the lambda body");
    let markup = match hover.contents {
        HoverContents::Markup(markup) => markup,
        other => panic!("expected markup contents, got {other:?}"),
    };
    assert_eq!(markup.kind, MarkupKind::Markdown);
    assert!(
        markup.value.contains("Prop"),
        "inferred type expected in hover markup: {:?}",
        markup.value
    );
    shutdown(&mut service).await;
}

#[tokio::test]
async fn hover_on_universe_applied_eq_prelude_constant_shows_signature() {
    // 用户原始症状（playground.sokonanoda:233）：hover `Eq.subst.{1}` 只显示
    // 源码切片。根因是内核 pp 对开项推断 panic、hover 文本被吞空；修复后
    // 必须显示 `Eq.subst.{1} : forall … p a …` 的完整签名。
    let src = concat!(
        "theorem eq_symm_nat : (a : Nat) -> (b : Nat) -> Eq.{1} Nat a b -> Eq.{1} Nat b a :=\n",
        "  fun (a : Nat) (b : Nat) (h : Eq.{1} Nat a b) =>\n",
        "    Eq.subst.{1} Nat (fun (x : Nat) => Eq.{1} Nat x a) a b h (Eq.refl.{1} Nat a)\n",
    );
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, src).await;
    let params = wait_diagnostics(&mut socket, "didOpen diagnostics").await;
    assert!(
        params.diagnostics.is_empty(),
        "valid theorem must publish no diagnostics: {:?}",
        params.diagnostics
    );
    let offset = offset_of(src, "Eq.subst.{1}");
    let result = call(
        &mut service,
        RpcRequest::build("textDocument/hover")
            .params(json!({
                "textDocument": {"uri": URI},
                "position": position_json(lsp_pos(src, offset)),
            }))
            .id(2)
            .finish(),
    )
    .await
    .expect("hover must answer");
    let hover: Option<Hover> = serde_json::from_value(result).expect("valid Hover");
    let hover = hover.expect("hover must resolve on `Eq.subst.{1}`");
    let HoverContents::Markup(markup) = hover.contents else {
        panic!("expected markup hover");
    };
    assert!(
        markup.value.contains("Eq.subst.{1} :"),
        "hover must show the signature, got: {:?}",
        markup.value
    );
    assert!(
        markup.value.contains("p a"),
        "hover signature must mention the dependent codomain, got: {:?}",
        markup.value
    );
    shutdown(&mut service).await;
}

#[tokio::test]
async fn hover_on_sorry_in_overapplied_spine_shows_hole_expected_type() {
    // 用户案例（playground 练习 5，0.25.0）：`(And.right a (Not a) x)
    // sorry` 的 hover 必须显示洞的精确期望类型 `a`（经 def `Not` 展开
    // `Not a` ⇒ `a -> False`），而不是整个声明类型。剩余目标 `False`。
    let src = "axiom False : Prop\n\
               axiom And : Prop -> Prop -> Prop\n\
               axiom And.right : (a : Prop) -> (b : Prop) -> And a b -> b\n\
               def Not : Prop -> Prop := fun (a : Prop) => a -> False\n\
               theorem and_not_absurd : (a : Prop) -> And a (Not a) -> False :=\n\
                 fun (a : Prop) => fun (x : And a (Not a)) => (And.right a (Not a) x) sorry\n";
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, src).await;
    let _ = wait_diagnostics(&mut socket, "didOpen diagnostics").await;

    let pos = lsp_pos(src, offset_of(src, "sorry") + 2);
    let result = call(
        &mut service,
        RpcRequest::build("textDocument/hover")
            .params(json!({
                "textDocument": {"uri": URI},
                "position": position_json(pos),
            }))
            .id(4)
            .finish(),
    )
    .await
    .expect("hover must answer");
    let hover: Option<Hover> = serde_json::from_value(result).expect("valid Hover");
    let hover = hover.expect("hover must resolve on the sorry");
    let markup = match hover.contents {
        HoverContents::Markup(markup) => markup,
        other => panic!("expected markup contents, got {other:?}"),
    };
    // 洞的精确期望类型（def 展开后的箭头定义域）。
    assert!(
        markup.value.contains("期望类型：") && markup.value.contains("```sokonanoda\na"),
        "hole expected type must be `a` in a sokonanoda fence: {:?}",
        markup.value
    );
    // 剩余目标 = 声明类型剥掉两层 lambda（goal 代码块内 `⊢ False`）。
    assert!(
        markup.value.contains("剩余目标：") && markup.value.contains("⊢ False"),
        "remaining goal must be `False`: {:?}",
        markup.value
    );
    // 上下文假设完整（goal 代码块内的 `x : And a (Not a)`）。
    assert!(
        markup.value.contains("x : And a (Not a)"),
        "{:?}",
        markup.value
    );
    // 不再把整个声明类型当目标展示。
    assert!(
        !markup.value.contains("目标：\n```sokonanoda\nforall"),
        "declared type must not be shown as the goal: {:?}",
        markup.value
    );
    shutdown(&mut service).await;
}

#[tokio::test]
async fn hover_on_hole_shows_goal() {
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, EXERCISE).await;
    let _ = wait_diagnostics(&mut socket, "didOpen diagnostics").await;

    let pos = lsp_pos(EXERCISE, offset_of(EXERCISE, "sorry") + 1);
    let result = call(
        &mut service,
        RpcRequest::build("textDocument/hover")
            .params(json!({
                "textDocument": {"uri": URI},
                "position": position_json(pos),
            }))
            .id(3)
            .finish(),
    )
    .await
    .expect("hover must answer");
    let hover: Option<Hover> = serde_json::from_value(result).expect("valid Hover");
    let hover = hover.expect("hover must resolve on the hole");
    let markup = match hover.contents {
        HoverContents::Markup(markup) => markup,
        other => panic!("expected markup contents, got {other:?}"),
    };
    assert!(
        markup.value.contains("目标"),
        "goal label expected in hover markup: {:?}",
        markup.value
    );
    assert!(
        markup.value.contains("Prop -> Prop"),
        "goal text expected in hover markup: {:?}",
        markup.value
    );
    shutdown(&mut service).await;
}

#[tokio::test]
async fn hover_on_partial_hole_lists_hypotheses() {
    let src = "example : (a : Prop) -> a -> a := fun (a : Prop) => fun (h : a) => sorry\n";
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, src).await;
    let _ = wait_diagnostics(&mut socket, "partial hole diagnostics").await;

    let hole = offset_of(src, "sorry");
    let pos = lsp_pos(src, hole);
    let result = call(
        &mut service,
        RpcRequest::build("textDocument/hover")
            .params(json!({
                "textDocument": {"uri": URI},
                "position": position_json(pos),
            }))
            .id(20)
            .finish(),
    )
    .await
    .expect("hover must answer");
    let hover: Option<Hover> = serde_json::from_value(result).expect("valid hover");
    let hover = hover.expect("hover at the hole");
    let HoverContents::Markup(markup) = hover.contents else {
        panic!("expected markup hover");
    };
    assert!(
        markup.value.contains("目标：") && markup.value.contains("⊢ a"),
        "hover shows goal: {}",
        markup.value
    );
    assert!(
        markup.value.contains("a : Prop") && markup.value.contains("h : a"),
        "hover lists hypotheses: {}",
        markup.value
    );
}

#[test]
fn code_fences_always_use_the_sokonanoda_language() {
    // docs/design/goal-rendering.md §7: one language id for every rendered
    // code block, so the single TM grammar colours all of them.
    assert_eq!(code_block("x : Nat"), "```sokonanoda\nx : Nat\n```");
    assert!(!code_block("x").contains("```text"));
}

#[tokio::test]
async fn hover_on_a_half_expression_shows_the_remaining_goals() {
    // 用户需求：半截表达式（`And.intro b a` 还差两个前提）的 hover 不只给
    // 报错——把推断出的剩余目标列成 `⊢ b`、`⊢ a`。按需计算 + judge 缓存，
    // 不在按键路径上。
    let src = "axiom And : Prop -> Prop -> Prop\n\
               axiom And.intro : (a : Prop) -> (b : Prop) -> a -> b -> And a b\n\
               theorem and_swap : (a : Prop) -> (b : Prop) -> And a b -> And b a :=\n\
                 fun (a : Prop) => fun (b : Prop) => fun (x : And a b) => And.intro b a\n";
    let (mut service, _socket) = open_and_wait(src).await;
    let at = src.rfind("And.intro").expect("value occurrence");
    let hover = hover_opt_at(&mut service, src, at)
        .await
        .expect("hover on the half expression must answer");
    let HoverContents::Markup(markup) = hover.contents else {
        panic!("expected markup hover");
    };
    assert!(
        markup.value.contains("还差 2 个前提"),
        "hover: {:?}",
        markup.value
    );
    assert!(markup.value.contains("⊢ b"), "hover: {:?}", markup.value);
    assert!(markup.value.contains("⊢ a"), "hover: {:?}", markup.value);
    assert!(
        markup.value.contains("```sokonanoda"),
        "half-expression goals use the highlighted fence: {:?}",
        markup.value
    );
    shutdown(&mut service).await;
}

#[tokio::test]
async fn hover_on_keyword_returns_none() {
    // 学习者反馈：光标在 fun/=>/theorem 上应该安静，而不是把某个
    // 节点的类型行硬塞过来。
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, VALID).await;
    let _ = wait_diagnostics(&mut socket, "keyword hover diagnostics").await;

    let fun_at = VALID.find("fun").expect("fun exists");
    let pos = lsp_pos(VALID, fun_at);
    let result = call(
        &mut service,
        RpcRequest::build("textDocument/hover")
            .params(json!({
                "textDocument": {"uri": URI},
                "position": position_json(pos),
            }))
            .id(80)
            .finish(),
    )
    .await
    .expect("hover must answer");
    let hover: Option<Hover> = serde_json::from_value(result).expect("valid hover");
    assert!(hover.is_none(), "keyword hover must be silent");
    shutdown(&mut service).await;
}

#[tokio::test]
async fn hover_on_binder_name_shows_its_type_not_the_lambda() {
    // hover 到 binder 名字 `a`：显示 `a : Prop`，绝不吐整段 lambda。
    let src = "axiom True : Prop\n\
               def f : Prop -> Prop := fun (a : Prop) => a\n";
    let (mut service, _socket) = open_and_wait(src).await;
    let a_name = src.find("(a").expect("binder") + 1;
    let markup = hover_markup_at(&mut service, src, a_name).await;
    assert!(
        markup.contains("a : Prop"),
        "hovering the binder name must show its type: {markup:?}"
    );
    assert!(
        !markup.contains("fun (a : Prop) => a"),
        "must not spill the whole lambda: {markup:?}"
    );
    shutdown(&mut service).await;
}

#[tokio::test]
async fn hover_on_binder_name_h_shows_declaration() {
    // hover 到 `h` 的 binder 名字：`h : And a (Not a)`（binder 声明）。
    let (mut service, _socket) = open_and_wait(AND_NOT_ABSURD).await;
    let h_name = AND_NOT_ABSURD.find("(h").expect("binder h") + 1;
    let markup = hover_markup_at(&mut service, AND_NOT_ABSURD, h_name).await;
    assert!(
        markup.contains("h : And a (Not a)"),
        "binder name must show its declaration: {markup:?}"
    );
    shutdown(&mut service).await;
}

#[tokio::test]
async fn hover_on_binder_annotation_bracket_shows_declaration() {
    // `(h : And a (Not a))` 的 `(` / `)`：显示 `h : And a (Not a)`
    //（不再截断成 `And a (Not a : Prop`）。
    let (mut service, _socket) = open_and_wait(AND_NOT_ABSURD).await;
    let group_open = AND_NOT_ABSURD.find("(h : And a (Not a))").expect("group");
    let group_close = group_open + "(h : And a (Not a))".len() - 1;
    for offset in [group_open, group_close] {
        let markup = hover_markup_at(&mut service, AND_NOT_ABSURD, offset).await;
        assert!(
            markup.contains("h : And a (Not a)"),
            "bracket {offset} must show the declaration: {markup:?}"
        );
        assert!(
            !markup.contains("And a (Not a :"),
            "must not be truncated: {markup:?}"
        );
    }
    shutdown(&mut service).await;
}

#[tokio::test]
async fn hover_on_binder_annotation_prop_shows_declaration() {
    // `(a : Prop)` 的 `(`：显示 `a : Prop`。
    let src = "axiom True : Prop\n\
               def f : Prop -> Prop := fun (a : Prop) => a\n";
    let (mut service, _socket) = open_and_wait(src).await;
    let group_open = src.find("(a : Prop)").expect("group");
    let markup = hover_markup_at(&mut service, src, group_open).await;
    assert!(
        markup.contains("a : Prop"),
        "binder annotation bracket must show the declaration: {markup:?}"
    );
    shutdown(&mut service).await;
}

#[tokio::test]
async fn hover_returns_range_highlighting_the_expression() {
    // 括号 hover 与普通表达式 hover 都返回非空 range，且 range 覆盖光标。
    let (mut service, _socket) = open_and_wait(AND_NOT_ABSURD).await;
    // 括号：`(And.right a (Not a) h)` 的 `(`。
    let group = AND_NOT_ABSURD.find("(And.right").expect("group");
    let hov = hover_opt_at(&mut service, AND_NOT_ABSURD, group)
        .await
        .expect("bracket hover");
    let range = hov.range.expect("bracket hover must carry a range");
    let cursor = lsp_pos(AND_NOT_ABSURD, group);
    assert!(
        range.start <= cursor && cursor <= range.end,
        "bracket range must contain the cursor: {range:?}"
    );
    // 普通表达式：hover 定理体内的 `And.right`（`(And.right` 之后那个）。
    let and_right = AND_NOT_ABSURD.find("(And.right").expect("group") + 1;
    let hov2 = hover_opt_at(&mut service, AND_NOT_ABSURD, and_right)
        .await
        .expect("expression hover");
    let range2 = hov2.range.expect("expression hover must carry a range");
    let cursor2 = lsp_pos(AND_NOT_ABSURD, and_right);
    assert!(
        range2.start <= cursor2 && cursor2 <= range2.end,
        "expression range must contain the cursor: {range2:?}"
    );
    shutdown(&mut service).await;
}

/// **记法符号的 hover**（D5，用户要求「hover 内容提示用户如何输入对应符号」）。
///
/// 改前实测：本文件声明的符号（`⊗`）hover **完全静默**——`front::semantic` 把
/// 已声明的记法符号归进 `SemanticKind::Keyword`，LSP 的「关键字不吐类型行」闸门
/// 把它一起吞掉了。内建 `∧` 走另一条路（有反应），import 来的符号因 X15 没有报告
/// ——三种形态行为不一致。这条分支把三种统一。
#[tokio::test]
async fn hover_on_a_locally_declared_notation_symbol_explains_it() {
    let src = concat!(
        "def myop (a b : Prop) : Prop := a\n",
        "infix:50 \" ⊗ \" => myop\n",
        "theorem t (a b : Prop) (h : a ⊗ b) : a ⊗ b := h\n",
    );
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, src).await;
    let _ = wait_diagnostics(&mut socket, "notation hover diagnostics").await;

    // **符号本身**的位置（`find` 会先命中 `infix` 行里字符串字面量中的那个）。
    let pos = lsp_pos(src, src.rfind('⊗').expect("use site exists"));
    let result = call(
        &mut service,
        RpcRequest::build("textDocument/hover")
            .params(json!({
                "textDocument": {"uri": URI},
                "position": position_json(pos),
            }))
            .id(2)
            .finish(),
    )
    .await
    .expect("hover must answer");
    let hover: Option<Hover> = serde_json::from_value(result).expect("valid Hover");
    let hover = hover.expect("a declared notation symbol must not be silent");
    let HoverContents::Markup(markup) = hover.contents else {
        panic!("expected markup hover");
    };
    assert!(
        markup.value.contains("记法符号"),
        "hover must say what this is: {:?}",
        markup.value
    );
    assert!(
        markup.value.contains("myop"),
        "hover must show what the symbol expands to: {:?}",
        markup.value
    );
    // **T-D03 形态①：本文件声明的**——目标解析得出来（本文件的声明行），
    // 所以"原始类型"那一行要在。
    assert!(
        markup.value.contains("myop : Prop -> Prop -> Prop"),
        "本文件声明的记法也要给原始类型：{:?}",
        markup.value
    );
    shutdown(&mut service).await;
}

/// **T-D17 的判据**：hover 的 `range` **只覆盖符号本身**（不是整段表达式、
/// 也不是 `None`）。
///
/// 为什么较真：以前 `range: None` ⇒ 客户端按"光标词"自己高亮，而客户端的分词
/// 规则与我们的词法不是一回事——`⁻¹'`/`×ˢ` 这种多字符符号、`𝒫` 这种星平面符号
/// 都会歪。给了精确 range 之后，编辑器画出来的框就是符号本身。
#[tokio::test]
async fn a_notation_hover_range_covers_only_the_symbol() {
    // 多字符符号（`⁻¹'` 三个字符）最能暴露"按光标词"的不准。
    let src = "def Set (α : Type) : Type := α -> Prop\n\
def Set.preimage (α : Type) (f : α -> α) (A : Set α) : Set α := A\n\
infixr:80 \" ⁻¹' \" => Set.preimage\n\
theorem t (α : Type) (f : α -> α) (A : Set α) (h : f ⁻¹' A = f ⁻¹' A) : f ⁻¹' A = f ⁻¹' A := h\n";
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, src).await;
    let _ = wait_diagnostics(&mut socket, "notation range").await;

    // 光标停在最后一个 `⁻¹'` 的**中间**（第二个字符上）——"按光标词"最容易歪的位置。
    // ⚠ 必须落在**字符边界**上：`⁻` 是 3 字节，`+1` 会切在它中间（`lsp_pos` 直接 panic）。
    let at = src.rfind("⁻¹'").expect("use site") + "⁻".len();
    let pos = lsp_pos(src, at);
    let result = call(
        &mut service,
        RpcRequest::build("textDocument/hover")
            .params(json!({
                "textDocument": {"uri": URI},
                "position": position_json(pos),
            }))
            .id(4)
            .finish(),
    )
    .await
    .expect("hover must answer");
    let hover: Option<Hover> = serde_json::from_value(result).expect("valid Hover");
    let hover = hover.expect("a notation symbol must not be silent");
    let range = hover.range.expect("T-D17：记法 hover 必须给精确 range");
    let symbol_start = lsp_pos(src, src.rfind("⁻¹'").expect("use site"));
    assert_eq!(
        (range.start.line, range.start.character),
        (symbol_start.line, symbol_start.character),
        "range 从符号的第一个字符开始：{range:?}"
    );
    assert_eq!(
        range.end.character - range.start.character,
        "⁻¹'".chars().count() as u32,
        "range 正好覆盖 `⁻¹'` 三个字符（不是整段表达式）：{range:?}"
    );
    shutdown(&mut service).await;
}

/// **T-D03 形态②：语言内建的**——`∧` 不在任何源文本里（parser 硬编码），
/// 目标从内建表来；签名问内核。
#[tokio::test]
async fn hover_on_a_builtin_notation_symbol_shows_the_raw_type() {
    let src = "theorem t (a b : Prop) (h : a ∧ b) : a ∧ b := h
";
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, src).await;
    let _ = wait_diagnostics(&mut socket, "builtin notation hover").await;

    let pos = lsp_pos(src, src.rfind('∧').expect("use site"));
    let result = call(
        &mut service,
        RpcRequest::build("textDocument/hover")
            .params(json!({
                "textDocument": {"uri": URI},
                "position": position_json(pos),
            }))
            .id(2)
            .finish(),
    )
    .await
    .expect("hover must answer");
    let hover: Option<Hover> = serde_json::from_value(result).expect("valid Hover");
    let hover = hover.expect("a builtin notation symbol must not be silent");
    let HoverContents::Markup(markup) = hover.contents else {
        panic!("expected markup hover");
    };
    assert!(
        markup.value.contains("展开成 `And`"),
        "内建记法要说得出目标：{:?}",
        markup.value
    );
    assert!(
        markup.value.contains("And : "),
        "内建记法也要给原始类型：{:?}",
        markup.value
    );
    // **T-D20 的落地决定**：内建 / prelude 目标（`∧`→`And`）**没有源码声明**，
    // `definition` 只能返回 `null`（`top_level_def_spans` 按构造排除 prelude）
    // ⇒ **hover 必须把原因说出来**：沉默的"跳不动"看起来像坏了。
    assert!(
        markup.value.contains("没有源码声明"),
        "内建记法要说明为什么跳不动（T-D20）：{:?}",
        markup.value
    );
    shutdown(&mut service).await;
}

/// 内建符号 `∧` 也必须给「怎么输入」（它在输入法表里）。
#[tokio::test]
async fn hover_on_a_builtin_symbol_teaches_how_to_type_it() {
    let with_notation = "theorem and_self (a b : Prop) (h : a ∧ b) : a ∧ b := h\n";
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, with_notation).await;
    let diags = wait_diagnostics(&mut socket, "builtin hover diagnostics").await;
    assert!(
        diags.diagnostics.is_empty(),
        "fixture must compile: {diags:?}"
    );

    let pos = lsp_pos(with_notation, offset_of(with_notation, "∧"));
    let result = call(
        &mut service,
        RpcRequest::build("textDocument/hover")
            .params(json!({
                "textDocument": {"uri": URI},
                "position": position_json(pos),
            }))
            .id(2)
            .finish(),
    )
    .await
    .expect("hover must answer");
    let hover: Option<Hover> = serde_json::from_value(result).expect("valid Hover");
    let hover = hover.expect("hover on `∧` must resolve");
    let HoverContents::Markup(markup) = hover.contents else {
        panic!("expected markup hover");
    };
    assert!(
        markup.value.contains("\\and"),
        "hover must teach the abbreviation: {:?}",
        markup.value
    );
    shutdown(&mut service).await;
}

/// `=` 是内建记法但**没有缩写**（Lean 也没有）⇒ hover 要说"直接打"，
/// 不能沉默（沉默会让学习者以为有缩写而反复试）。
#[tokio::test]
async fn hover_on_equality_says_it_is_typed_directly() {
    let src = "theorem eq_self (A : Prop) (h : A = A) : A = A := h\n";
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, src).await;
    let diags = wait_diagnostics(&mut socket, "equality hover diagnostics").await;
    assert!(
        diags.diagnostics.is_empty(),
        "fixture must compile: {diags:?}"
    );

    // **`=` 本身**的位置（`offset_of(src, "A = A")` 给的是那个 `A`）。
    let pos = lsp_pos(src, offset_of(src, "A = A") + 2);
    let result = call(
        &mut service,
        RpcRequest::build("textDocument/hover")
            .params(json!({
                "textDocument": {"uri": URI},
                "position": position_json(pos),
            }))
            .id(2)
            .finish(),
    )
    .await
    .expect("hover must answer");
    let hover: Option<Hover> = serde_json::from_value(result).expect("valid Hover");
    let hover = hover.expect("hover on `=` must resolve");
    let HoverContents::Markup(markup) = hover.contents else {
        panic!("expected markup hover");
    };
    assert!(
        markup.value.contains("记法符号"),
        "hover must say what this is: {:?}",
        markup.value
    );
    assert!(
        markup.value.contains("Eq"),
        "hover must show the expansion target: {:?}",
        markup.value
    );
    assert!(
        markup.value.contains("直接打"),
        "hover must say there is no abbreviation: {:?}",
        markup.value
    );
    shutdown(&mut service).await;
}

/// **`import` 来的记法符号要给出原始类型**（T-D02，用户第 6 条反馈
/// 「hover 信息也没有对应的原始类型」）。
///
/// 这条踩到**闭包**：`∈` 声明在 `SetLib.sokonanoda` 里，入口只是 `import` 了它
/// ——`notation_input::symbol_at` 只看本文件 + 内建 ⇒ 目标永远是 `None`
/// ⇒ 既说不出"展开成什么"，也拿不到签名（实测：hover 里连"展开成"那一行都没有）。
/// 修法：目标解析加一条**闭包前缀**的回退（`symbol_at_with_sources`），
/// 签名走 `judge_type_of_constant`。
///
/// 那一行**故意不折记法**：它叫"**原始**类型"，给的就是"底下站着什么"——
/// 折成 `A ⊆ B` 反而把它要回答的问题盖掉了。
#[tokio::test]
async fn hover_on_an_imported_notation_symbol_shows_the_raw_type() {
    let dir = std::env::temp_dir().join(format!(
        "sokonanoda-hover-notation-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp project");
    std::fs::write(
        dir.join("SetLib.sokonanoda"),
        "def Set (α : Type) : Type := α -> Prop\n\
def Set.mem (α : Type) (a : α) (A : Set α) : Prop := A a\n\
infix:50 \" ∈ \" => Set.mem\n",
    )
    .expect("write lib");
    let entry = dir.join("Canvas.sokonanoda");
    let src = "import SetLib\n\ntheorem mem_self (α : Type) (a : α) (A : Set α) (h : a ∈ A) : a ∈ A := h\n";
    std::fs::write(&entry, src).expect("write entry");
    let uri = Url::from_file_path(&entry).expect("file url");

    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    testutil::did_open_at(&mut service, &uri, src).await;
    let _ = testutil::wait_diagnostics_for(&mut socket, &uri, "notation raw type").await;

    let pos = lsp_pos(src, src.rfind('∈').expect("use site"));
    let result = call(
        &mut service,
        RpcRequest::build("textDocument/hover")
            .params(json!({
                "textDocument": {"uri": uri},
                "position": position_json(pos),
            }))
            .id(2)
            .finish(),
    )
    .await
    .expect("hover must answer");
    let hover: Option<Hover> = serde_json::from_value(result).expect("valid Hover");
    let hover = hover.expect("an imported notation symbol must not be silent");
    let HoverContents::Markup(markup) = hover.contents else {
        panic!("expected markup hover");
    };
    assert!(
        markup.value.contains("展开成 `Set.mem`"),
        "import 来的记法也要说出展开目标：{:?}",
        markup.value
    );
    assert!(
        markup
            .value
            .contains("Set.mem : forall (α : Type 0), α -> Set α -> Prop"),
        "hover 必须给出原始类型：{:?}",
        markup.value
    );
    let _ = std::fs::remove_dir_all(&dir);
}

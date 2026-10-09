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
    // docs/protocol.md §`soko/stateAt`: one language id for every rendered
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
        markup
            .value
            .contains("myop : (a : Prop) -> (b : Prop) -> Prop"),
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
    // **E10 之后的事实（2026-10-10 更正）**：内建记法**有**声明点（prelude 里那行
    // `-- sokonanoda:builtin-notation "∧" => And` 登记注释）⇒ `definition` **有**
    // 落点 ⇒ hover **不许**再说「没有源码声明，`F12` 无处可跳」—— 那是 T-D20 时代
    // 的话，今天与 `definition` 当场自相矛盾 ✗（用户 2026-10-10 反馈）。
    assert!(
        markup
            .value
            .contains("`F12` 落到 `prelude/Prelude.sokonanoda`"),
        "内建记法要说清**落点在哪**（E10：声明点是 prelude 的登记注释行）：{:?}",
        markup.value
    );
    assert!(
        !markup.value.contains("没有源码声明") && !markup.value.contains("无处可跳"),
        "E10 之后「没有源码声明 / 无处可跳」是**假话**（F12 真的跳得到）✗：{:?}",
        markup.value
    );
    shutdown(&mut service).await;
}

/// **用户反馈（2026-10-10）**：`"a = b"` 里的 `=` —— hover 与 F12 必须**一致**。
///
/// 以前 `=` 的 hover 说「内建记法（内核 prelude）：**没有源码声明**，`F12` 无处可跳」，
/// 而它确实是 `null` ⇒ 文案当时是真的、但那**不是**用户要的行为（他要跳得到）。
/// 本轮给 `=` 补了 prelude 登记行 ⇒ 两件事一起变：F12 有落点（见
/// `navigation::goto_definition_on_the_equals_notation_lands_in_the_prelude`）、
/// hover 说清落点 ✓。**这条钉的是用户实际悬停的那个字符**（`a = b` 的 `=` 本身 ✓）。
#[tokio::test]
async fn hover_on_the_equals_notation_no_longer_claims_there_is_nothing_to_jump_to() {
    let src = "theorem t (a b : Prop) (h : a = b) : a = b := h\n";
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, src).await;
    let _ = wait_diagnostics(&mut socket, "equals notation hover").await;

    // 用户点的是**第一个 `=`**（`h : a = b` 里那个）—— 不是"能跑通的位置"。
    let pos = lsp_pos(src, src.find(" = ").expect("the `=` the user clicks") + 1);
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
    let hover = hover.expect("`=` must not be silent");
    let HoverContents::Markup(markup) = hover.contents else {
        panic!("expected markup hover");
    };
    assert!(
        markup.value.contains("展开成 `Eq`"),
        "`=` 的 hover 要说得出展开目标：{:?}",
        markup.value
    );
    assert!(
        markup
            .value
            .contains("`F12` 落到 `prelude/Prelude.sokonanoda`"),
        "`=` 的 hover 要说清落点（与 F12 一致）：{:?}",
        markup.value
    );
    assert!(
        !markup.value.contains("没有源码声明") && !markup.value.contains("无处可跳"),
        "`=` 现在**有**登记行 ⇒ 旧文案是假话 ✗：{:?}",
        markup.value
    );
    shutdown(&mut service).await;
}

/// **用户反馈（2026-10-10）的第二跳**：prelude 登记注释里的**目标名** —— 以前
/// `-- sokonanoda:builtin-notation "∧" => And` 的 `And` 上 hover 是 `null`（注释不
/// 产生 token ✗）⇒ 用户「点击 `And` 无法跳转」的同一条根因。
///
/// 这条同时钉**同族横排**的两半（诚实说明 ≠ 静默 ✗）：
///   * `And`（内建记法的目标）⇒ 给名字 + **签名**（prelude 名字解析得出来 ✓）；
///   * `Set.singleton`（`builtin-sugar` 的目标，**卷 I 课程库**常量）⇒ 名字 +
///     **说明为什么给不出签名、也没有落点** ✓（不是静默 `null` ✓）。
#[tokio::test]
async fn hover_on_a_comment_registration_target_name_explains_itself() {
    let path = sokonanoda_front::compile::prelude_source_path().expect("prelude 源落盘路径");
    // ⚠ **规范化**：`prelude_source_path()` 带 `..`，而 URI 过一趟 JSON（`json!`）会被
    // URL 规范**归一化** ⇒ 测试手里的 `uri` 与服务端发布诊断用的 `uri` 不相等，
    // `wait_diagnostics_for` 会**空等到超时** ✗（2026-10-10 实测踩到）。
    let path = path.canonicalize().expect("canonicalize prelude");
    let text = std::fs::read_to_string(&path).expect("read prelude");
    let uri = Url::from_file_path(&path).expect("file url");

    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    testutil::did_open_at(&mut service, &uri, &text).await;
    let _ = testutil::wait_diagnostics_for(&mut socket, &uri, "prelude hover").await;

    // `needle` 一律取到**目标名之前**（它的结尾就是用户点的那个字符 ✓）⇒
    // 光标偏移 = needle 起点 + `needle.len()`（按**字节**；`∧` 是 3 字节，
    // `offset_of` 也是字节偏移 ✓）。
    let hover_at = |needle: &str| {
        let pos = lsp_pos(&text, offset_of(&text, needle) + needle.len());
        RpcRequest::build("textDocument/hover")
            .params(json!({
                "textDocument": {"uri": uri},
                "position": position_json(pos),
            }))
            .id(7)
            .finish()
    };

    // ① 内建记法的目标名 `And`（用户实际点的那个字符位置 ✓）。
    let result = call(
        &mut service,
        // `needle` 的**结尾**就是目标名的起点（用户点的那个字符 ✓）。
        hover_at("-- sokonanoda:builtin-notation \"∧\" => "),
    )
    .await
    .expect("hover must answer");
    let hover: Option<Hover> = serde_json::from_value(result).expect("valid Hover");
    let hover = hover.expect("注释登记行里的目标名不许静默（第二跳）✗");
    let HoverContents::Markup(markup) = hover.contents else {
        panic!("expected markup hover");
    };
    assert!(
        markup.value.contains("`And` —— 记法的目标"),
        "要说清它是记法的目标：{:?}",
        markup.value
    );
    assert!(
        markup.value.contains("`And : "),
        "prelude 名字解析得出来 ⇒ 必须给签名：{:?}",
        markup.value
    );

    // ② `builtin-sugar` 的目标名 `Set.singleton`（卷 I 库常量，本文件闭包里没有）。
    let result = call(
        &mut service,
        hover_at("-- sokonanoda:builtin-sugar \"{a}\" => "),
    )
    .await
    .expect("hover must answer");
    let hover: Option<Hover> = serde_json::from_value(result).expect("valid Hover");
    let hover = hover.expect("`Set.singleton` 上不许静默（要说明原因）✗");
    let HoverContents::Markup(markup) = hover.contents else {
        panic!("expected markup hover");
    };
    assert!(
        markup.value.contains("`Set.singleton` —— 记法的目标"),
        "要说清它是记法的目标：{:?}",
        markup.value
    );
    assert!(
        markup.value.contains("没有声明"),
        "解析不出签名时必须**说明原因**（诚实说明 ≠ 静默）：{:?}",
        markup.value
    );
    shutdown(&mut service).await;
}

/// **E2 的同族（2026-10-10）**：`-- sokonanoda:builtin-rust "Nat / …"` 里登记的名字
/// （`Nat` / `Bool` 两族）在**源文本里没有声明位置** ⇒ `F12` 如实没有落点
/// （`prelude_def_span` 返回 `None`，**不许编位置** ✗）。但 hover **不许静默**：
/// 说清它是内核内建、为什么没有落点 ✓。
#[tokio::test]
async fn hover_on_a_builtin_rust_registry_name_explains_the_missing_span() {
    let path = sokonanoda_front::compile::prelude_source_path().expect("prelude 源落盘路径");
    // ⚠ **规范化**：`prelude_source_path()` 带 `..`，而 URI 过一趟 JSON（`json!`）会被
    // URL 规范**归一化** ⇒ 测试手里的 `uri` 与服务端发布诊断用的 `uri` 不相等，
    // `wait_diagnostics_for` 会**空等到超时** ✗（2026-10-10 实测踩到）。
    let path = path.canonicalize().expect("canonicalize prelude");
    let text = std::fs::read_to_string(&path).expect("read prelude");
    let uri = Url::from_file_path(&path).expect("file url");

    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    testutil::did_open_at(&mut service, &uri, &text).await;
    let _ = testutil::wait_diagnostics_for(&mut socket, &uri, "builtin-rust hover").await;

    let offset = offset_of(&text, "builtin-rust \"Nat /");
    let pos = lsp_pos(&text, offset + "builtin-rust \"".len());
    let result = call(
        &mut service,
        RpcRequest::build("textDocument/hover")
            .params(json!({
                "textDocument": {"uri": uri},
                "position": position_json(pos),
            }))
            .id(9)
            .finish(),
    )
    .await
    .expect("hover must answer");
    let hover: Option<Hover> = serde_json::from_value(result).expect("valid Hover");
    let hover = hover.expect("内建家族的登记名不许静默（E2）✗");
    let HoverContents::Markup(markup) = hover.contents else {
        panic!("expected markup hover");
    };
    assert!(
        markup.value.contains("`Nat` —— 内核内建"),
        "要说清它是内核内建：{:?}",
        markup.value
    );
    assert!(
        markup.value.contains("没有它的声明位置"),
        "要说清为什么 `F12` 没有落点（E2 如实登记）：{:?}",
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

/// **用户反馈（2026-10-01）：「`α` 没有快捷输入」**（设计
/// `docs/design/notation-input.md` D5）。
///
/// 希腊字母是**标识符**不是记法符号（D4）⇒ 它走的是普通表达式 hover
/// （`α : Prop`），`notation_symbol_hover` 那条**根本不会触发**（它只认 `Sym`）。
/// 所以「怎么输入」这一行必须单独补上去，而且**不能**说成"记法符号"。
#[tokio::test]
async fn hover_on_a_greek_identifier_teaches_how_to_type_it() {
    let src = "theorem alpha_self (α : Prop) (h : α) : α := h\n";
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, src).await;
    let diags = wait_diagnostics(&mut socket, "greek hover diagnostics").await;
    assert!(
        diags.diagnostics.is_empty(),
        "fixture must compile: {diags:?}"
    );

    let pos = lsp_pos(src, offset_of(src, "α : Prop"));
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
    let hover = hover.expect("hover on the binder `α` must resolve");
    let HoverContents::Markup(markup) = hover.contents else {
        panic!("expected markup hover");
    };
    assert!(
        markup.value.contains("\\alpha"),
        "hover must teach the abbreviation: {:?}",
        markup.value
    );
    assert!(
        markup.value.contains("\\a"),
        "hover must list the single-letter alias: {:?}",
        markup.value
    );
    assert!(
        !markup.value.contains("记法符号"),
        "`α` 是标识符，不是记法符号（说成记法符号会误导）：{:?}",
        markup.value
    );
    assert!(
        markup.value.contains("Prop"),
        "输入提示**追加**在类型行之后，不抢主线：{:?}",
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
            .contains("Set.mem : (α : Type) -> (a : α) -> (A : Set α) -> Prop"),
        "hover 必须给出原始类型：{:?}",
        markup.value
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// **T-D50 / G-37 的判据**：hover 在 `infix … => myop` 的**目标名**上 ⇒ 要给签名。
///
/// ⚠ **为什么单独一条**：这条 hover 走
/// `judge_type_of_constant("", …)`（**空前缀**，`lib.rs` 的「记法的目标」分支）
/// —— 空前缀**合成不出** `myop` 的声明 ⇒ 它**只能靠编译期那次调用留下的缓存
/// 副作用**（`elab_notation` 展开 `⊗` 时用**单元自己的文本**问过一次同名常量，
/// 而那张缓存的键**不含前缀**）。
///
/// 就地档（`type_of_constant_prefer_inplace`）若**不写穿**那张缓存，这条 hover
/// 就会**静默少一行签名** ✗ —— 与 `b734114d` 抓到的那条回归**同族**（那条走的是
/// 「记法符号」分支，已被 §11.18 的空前缀修法覆盖；**本条是第二个消费者，至今没有
/// 判据**）。修法见 `judge.rs::type_of_constant_cache_put` 的写穿 ✓。
#[tokio::test]
async fn hover_on_a_notation_target_name_shows_its_signature() {
    let src = concat!(
        "def myop (a b : Prop) : Prop := a\n",
        "infix:50 \" ⊗ \" => myop\n",
        "theorem t (a b : Prop) (h : a ⊗ b) : a ⊗ b := h\n",
    );
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, src).await;
    let _ = wait_diagnostics(&mut socket, "notation target hover diagnostics").await;
    // `=> myop` 里那个名字（**声明行**，不是使用点）。
    let at = src.find("=> myop").expect("target line exists") + "=> ".len();
    let pos = lsp_pos(src, at);
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
    let hover = hover.expect("a notation target name must not be silent");
    let HoverContents::Markup(markup) = hover.contents else {
        panic!("expected markup hover");
    };
    assert!(
        markup.value.contains("记法的目标"),
        "hover must say what this is: {:?}",
        markup.value
    );
    assert!(
        markup
            .value
            .contains("myop : (a : Prop) -> (b : Prop) -> Prop"),
        "记法目标名必须给出原始签名（空前缀 ⇒ 只能靠编译期缓存副作用）：{:?}",
        markup.value
    );
    shutdown(&mut service).await;
}

// ─────────────────────────────────────────────────────────────────────────
// 用户反馈三（2026-10-10）：tactic 里的**名字**上 hover ⇒ goal state 之后
// **再加一条分割线与该名字的类型行**。契约 `docs/protocol.md`
// §Tactic goal-state hover；判据分三层（wire / 词法 / 反向逐字节）。
// ─────────────────────────────────────────────────────────────────────────

/// 夹具：`apply <常量>`，常量在**被 import 的库**里 —— 与用户现场
/// `courses/set-theory/units/I.1/unit01-sets-membership.sokonanoda:65`
/// （`apply Set.ext`，`Set.ext` 在课程库里）同形。
fn apply_set_ext_fixture() -> (std::path::PathBuf, Url, String) {
    let dir = std::env::temp_dir().join(format!(
        "sokonanoda-hover-tactic-name-{}-{}",
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
axiom Set.ext {α : Type} {A B : Set α} : (forall (x : α), A x ↔ B x) -> Eq.{1} (Set α) A B\n\
infix:50 \" ∈ \" => Set.mem\n",
    )
    .expect("write lib");
    let src = "import SetLib\n\n\
theorem ext_test (α : Type) (A B : Set α) (h : forall (x : α), A x ↔ B x) :\n    \
Eq.{1} (Set α) A B := by\n  apply Set.ext\n  exact h\n";
    let entry = dir.join("Canvas.sokonanoda");
    std::fs::write(&entry, src).expect("write entry");
    let uri = Url::from_file_path(&entry).expect("file url");
    (dir, uri, src.to_string())
}

/// 光标在 **`apply` 关键字**上时的输出 —— **改动前**（2026-10-10，
/// `5afd1938` + 同夹具）的 wire 字节，137 字节。反向判据的基线：
/// 「关键字上 ⇒ 不加那一行」必须**逐字节**成立（不是"看着差不多"）。
const APPLY_KEYWORD_BASELINE: &str = "\
```sokonanoda
apply Set.ext
```
tactic 1/2

```sokonanoda
α : Type
A : Set α
B : Set α
h : (x : α) → (A x) ↔ (B x)
⊢ ((Set α) = A) B
```
";

/// **判据（wire / 用户动作）**：光标**正好落在 `Set.ext` 的字符上**（用户实际点的
/// 那个字符）⇒ hover = goal state + 分割线 + 折记法的签名行。
///
/// ⚠ 三件**都要**断言，而且**按顺序**：只断言"有 `Set.ext` 字样"会被 goal state
/// 自己骗过（硬规则 0(a)：断言用户实际看到的那一段）。
#[tokio::test]
async fn hover_on_a_tactic_constant_name_shows_goal_state_then_its_signature() {
    let (dir, uri, src) = apply_set_ext_fixture();
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    testutil::did_open_at(&mut service, &uri, &src).await;
    let params = testutil::wait_diagnostics_for(&mut socket, &uri, "tactic name hover").await;
    assert!(
        params.diagnostics.is_empty(),
        "夹具必须干净（诊断=0）：{:?}",
        params.diagnostics
    );

    // 用户动作：鼠标停在 `Set.ext` 的**字符**上（第 2 个字符，`e`）。
    let at = offset_of(&src, "Set.ext") + 1;
    let pos = lsp_pos(&src, at);
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
    let hover = hover.expect("hover on a tactic name must answer");
    let HoverContents::Markup(markup) = hover.contents else {
        panic!("expected markup hover");
    };
    let value = &markup.value;
    let goal = value
        .find("⊢ ((Set α) = A) B")
        .unwrap_or_else(|| panic!("goal state 必须还在：{value:?}"));
    let divider = value
        .find("\n---\n")
        .unwrap_or_else(|| panic!("分割线必须在：{value:?}"));
    let signature = value
        .find("`Set.ext : {α : Type} → {A : Set α} → {B : Set α} → ((x : α) → (A x) ↔ (B x)) → ")
        .unwrap_or_else(|| panic!("名字签名行（折记法）必须在：{value:?}"));
    assert!(
        goal < divider && divider < signature,
        "顺序必须是 goal state → 分割线 → 签名行：{value:?}"
    );
    // `range` 决策（`docs/protocol.md`）：**保持整条 tactic** —— 点 `apply`
    // 关键字也给 goal state（上一条判据），改成名字的 span 会与它冲突。
    let range = hover.range.expect("tactic hover 带 range");
    assert_eq!(range.start, lsp_pos(&src, offset_of(&src, "apply Set.ext")));
    assert_eq!(
        range.end,
        lsp_pos(&src, offset_of(&src, "Set.ext") + "Set.ext".len())
    );
    let _ = std::fs::remove_dir_all(&dir);
    shutdown(&mut service).await;
}

/// **反向判据（逐字节）**：光标在 **tactic 关键字** `apply` 上 ⇒ 没有类型行、
/// 没有分割线，输出与改动前**逐字节相同**。
///
/// 这条挡的是"整条 tactic 一律加一行"的退化；`range` 也必须还是整条 tactic。
#[tokio::test]
async fn hover_on_a_tactic_keyword_is_byte_identical_to_the_old_output() {
    let (dir, uri, src) = apply_set_ext_fixture();
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    testutil::did_open_at(&mut service, &uri, &src).await;
    let _ = testutil::wait_diagnostics_for(&mut socket, &uri, "tactic keyword hover").await;

    let at = offset_of(&src, "apply") + 1;
    let pos = lsp_pos(&src, at);
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
    let hover = hover.expect("hover on `apply` must still answer with the goal state");
    let HoverContents::Markup(markup) = hover.contents else {
        panic!("expected markup hover");
    };
    assert_eq!(
        markup.value, APPLY_KEYWORD_BASELINE,
        "关键字上的输出必须与改动前逐字节相同（不许加行、不许改顺序）"
    );
    assert!(
        !markup.value.contains("---"),
        "关键字上不许出现分割线：{:?}",
        markup.value
    );
    let range = hover.range.expect("tactic hover 带 range");
    assert_eq!(range.start, lsp_pos(&src, offset_of(&src, "apply Set.ext")));
    assert_eq!(
        range.end,
        lsp_pos(&src, offset_of(&src, "Set.ext") + "Set.ext".len())
    );
    let _ = std::fs::remove_dir_all(&dir);
    shutdown(&mut service).await;
}

/// **横向排查（局部假设 / 正在引入的绑元）**：
/// * `exact hp` 的 `hp` 是**进入态的假设** ⇒ 给类型行（与 goal block 同源）；
/// * `intro hp` 的 `hp` 是**这条 tactic 正在引入的绑元** ⇒ **不给** —— 它不在
///   进入态里，也不该去编一个同名常量的签名（诚实省略：拿不到干净类型就不编）。
#[tokio::test]
async fn hover_on_a_tactic_local_name_gives_its_type_but_not_the_binder_being_introduced() {
    let src = "axiom P : Prop\n\
               axiom Q : Prop\n\
               theorem t : P -> Q -> P := by\n  \
               intro hp\n  \
               intro hq\n  \
               exact hp\n";
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, src).await;
    let _ = wait_diagnostics(&mut socket, "tactic local name diagnostics").await;

    // `exact hp`：进入态的假设里有 `hp : P`。
    let exact_at = offset_of(src, "exact hp") + "exact ".len();
    let hover = hover_opt_at(&mut service, src, exact_at)
        .await
        .expect("hover on `exact hp` must answer");
    let HoverContents::Markup(markup) = hover.contents else {
        panic!("expected markup hover");
    };
    assert!(
        markup.value.contains("hp : P"),
        "goal block 里应有 `hp : P`：{:?}",
        markup.value
    );
    assert!(
        markup.value.contains("\n---\n\n`hp : P`"),
        "局部假设也要给分割线 + 类型行：{:?}",
        markup.value
    );

    // `intro hp`：`hp` 是这条 tactic 引入的，不在进入态 ⇒ 不编。
    let intro_at = offset_of(src, "intro hp") + "intro ".len();
    let hover = hover_opt_at(&mut service, src, intro_at)
        .await
        .expect("hover on `intro hp` must answer");
    let HoverContents::Markup(markup) = hover.contents else {
        panic!("expected markup hover");
    };
    assert!(
        !markup.value.contains("---"),
        "正在引入的绑元名上不许加类型行：{:?}",
        markup.value
    );
    shutdown(&mut service).await;
}

/// **横向排查（词法层）**：tactic 里哪些字符算「名字」——判据走**语言自己的
/// lexer**（不是文本扫描）：数字 / 括号 / 字符串 / 记法符号 / `_` / 关键字 /
/// `sorry` 都不是名字；**点分名是一个 `Ident`** ⇒ 整段命中。
#[test]
fn tactic_name_at_accepts_only_plain_identifiers() {
    let text = "apply Set.ext; exact (h); use \"x\"; intro _; exact 0; exact ∈; assumption; sorry";
    let span = sokonanoda_front::Span::new(
        sokonanoda_front::Pos {
            offset: 0,
            line: 1,
            column: 1,
        },
        sokonanoda_front::Pos {
            offset: text.len(),
            line: 1,
            column: text.len() + 1,
        },
    );
    let name_at = |needle: &str| {
        let at = offset_of(text, needle);
        crate::tactic_name_at(text, span, at)
    };
    assert_eq!(
        name_at("Set.ext").as_deref(),
        Some("Set.ext"),
        "点分名在词法层是一个 Ident（用户点 `.` 也算在名字上）"
    );
    assert_eq!(
        name_at("h)").as_deref(),
        Some("h"),
        "光标在括号里的名字上 ⇒ 仍是名字"
    );
    assert_eq!(name_at("("), None, "括号不是名字");
    assert_eq!(name_at("\"x\""), None, "字符串不是名字");
    assert_eq!(name_at("_"), None, "`_` 是占位符，不是名字");
    assert_eq!(name_at("0"), None, "数字不是名字");
    assert_eq!(name_at("∈"), None, "记法符号不是名字");
    assert_eq!(
        name_at("sorry"),
        None,
        "`sorry` 是 tactic 关键字（不在语义词表里）"
    );
    // **全部** tactic 关键字逐个过一遍（不是只挑两个）：它们都不能变成"名字"
    // —— 否则 `apply` 上也会冒出类型行，与"关键字上逐字节不变"直接冲突。
    for kw in [
        "intro",
        "exact",
        "apply",
        "assumption",
        "rfl",
        "constructor",
        "left",
        "right",
        "use",
        "exfalso",
        "cases",
        "have",
        "with",
        "sorry",
    ] {
        let span = sokonanoda_front::Span::new(
            sokonanoda_front::Pos {
                offset: 0,
                line: 1,
                column: 1,
            },
            sokonanoda_front::Pos {
                offset: kw.len(),
                line: 1,
                column: kw.len() + 1,
            },
        );
        assert_eq!(
            crate::tactic_name_at(kw, span, 1),
            None,
            "`{kw}` 是 tactic 关键字，不是名字"
        );
    }
}

/// 横向：**本文件**声明的常量（`apply soko_local_op`）也要给签名 —— 前缀取的是
/// **闭包 + 本文件**（`judge_prefix_with_entry`），不是只有闭包。
///
/// 为什么单独一条：`§11.17` 那次回归就是"目标在本文件 ⇒ 空前缀必然查不到"
/// ⇒ 整行消失。这条钉住它（改回只看闭包 ⇒ 必红）。
///
/// ⚠ **名字必须全局唯一**（这里 `soko_local_op`，不是 `myop`）：`judge_type_of_constant`
/// 的缓存键只有 `(options, name)`（`judge.rs` 的既定设计：常量签名与使用者无关）
/// ⇒ **同一个测试进程里两个同名不同签名的夹具会互相投毒**。实测（2026-10-10）：
/// 本用例起初用 `myop`（与 `hover_on_a_notation_target_name_shows_its_signature`
/// 的夹具同名）⇒ 单跑绿、与 `hover` 全批一起跑**红**，拿到的正是另一个夹具的
/// `myop : (a : Prop) → (b : Prop) → Prop`。换唯一名即可（**不是**改那张缓存：
/// 那是 `crates/front` 的判定缓存，键的设计有其成本理由，不在本条的范围里）。
#[tokio::test]
async fn hover_on_a_tactic_name_declared_in_the_same_file_shows_its_signature() {
    let src = "axiom P : Prop\n\
               axiom soko_local_op : P -> P\n\
               theorem t (h : P) : P := by\n  \
               apply soko_local_op\n  \
               exact h\n";
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, src).await;
    let _ = wait_diagnostics(&mut socket, "same-file tactic name diagnostics").await;

    let at = offset_of(src, "apply soko_local_op") + "apply ".len() + 1;
    let hover = hover_opt_at(&mut service, src, at)
        .await
        .expect("hover on `apply soko_local_op` must answer");
    let HoverContents::Markup(markup) = hover.contents else {
        panic!("expected markup hover");
    };
    assert!(
        markup.value.contains("\n---\n\n`soko_local_op : P → P`"),
        "本文件声明的常量也要给分割线 + 折记法的签名：{:?}",
        markup.value
    );
    shutdown(&mut service).await;
}

/// 签名编辑夹具（单文件：不需要项目清单）。
fn signature_edit_fixture() -> (std::path::PathBuf, Url, String) {
    const BEFORE: &str = "def myop : Nat := 0\n\ndef t1 : Nat := by\n  exact myop\n";
    let dir = std::env::temp_dir().join(format!(
        "sokonanoda-hover-signature-edit-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    let entry = dir.join("Canvas.sokonanoda");
    std::fs::write(&entry, BEFORE).expect("write entry");
    let uri = Url::from_file_path(&entry).expect("file url");
    (dir, uri, BEFORE.to_string())
}

/// **判据（wire / 用户动作）**：把常量**签名改了**之后再 hover，名字那一行必须是**新**签名。
///
/// 缺陷（2026-10-10 实测，真 LSP）：`judge_type_of_constant` 的缓存键只有
/// `(prelude 模式, 名字)`、是**进程级**的、**没有任何失效路径** ⇒ 编辑后 hover 出的是
/// **旧签名**（同一份 hover 里 goal state 已经是新的 ⇒ 自相矛盾 ✗：实测
/// `⊢ Bool` 与 `` `myop : Nat` `` 同时出现）。修法 = hover 路径改走**前缀键**的
/// `judge_type_of`（`lib.rs::hover_type_of_constant`，零性能代价：未命中时它本来就走这条）。
///
/// **反向验证**：把 `hover_type_of_constant` 换回 `judge_type_of_constant` ⇒ 本用例判红
/// （实测第二次 hover 仍是 `` `myop : Nat` ``，而文件里已是 `Bool`）。
#[tokio::test]
async fn hover_on_a_tactic_name_reflects_an_edited_signature() {
    const AFTER: &str = "def myop : Bool := Bool.true\n\ndef t1 : Bool := by\n  exact myop\n";
    let (dir, uri, before) = signature_edit_fixture();
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    testutil::did_open_at(&mut service, &uri, &before).await;
    let params = testutil::wait_diagnostics_for(&mut socket, &uri, "signature edit hover").await;
    assert!(
        params.diagnostics.is_empty(),
        "夹具必须干净（诊断=0）：{:?}",
        params.diagnostics
    );

    // 第一次 hover：文件里是 `def myop : Nat := 0`。
    let pos = lsp_pos(
        &before,
        offset_of(&before, "exact myop") + "exact ".len() + 1,
    );
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
    let HoverContents::Markup(markup) = hover.expect("hover on the tactic name").contents else {
        panic!("expected markup hover");
    };
    assert!(
        markup.value.contains("`myop : Nat`"),
        "第一次 hover 必须给当时的签名：{:?}",
        markup.value
    );

    // 用户动作：**把签名改掉**（编辑器里的普通编辑 ⇒ didChange）。
    notify(
        &mut service,
        "textDocument/didChange",
        json!({
            "textDocument": {"uri": uri, "version": 2},
            "contentChanges": [{"text": AFTER}],
        }),
    )
    .await;
    let params = testutil::wait_diagnostics_for(&mut socket, &uri, "hover after edit").await;
    assert!(
        params.diagnostics.is_empty(),
        "编辑后的夹具也必须干净：{:?}",
        params.diagnostics
    );

    // 第二次 hover：文件里已是 `def myop : Bool := Bool.true` ⇒ 名字那行必须跟着变。
    let pos = lsp_pos(AFTER, offset_of(AFTER, "exact myop") + "exact ".len() + 1);
    let result = call(
        &mut service,
        RpcRequest::build("textDocument/hover")
            .params(json!({
                "textDocument": {"uri": uri},
                "position": position_json(pos),
            }))
            .id(3)
            .finish(),
    )
    .await
    .expect("hover must answer");
    let hover: Option<Hover> = serde_json::from_value(result).expect("valid Hover");
    let HoverContents::Markup(markup) = hover.expect("hover on the tactic name").contents else {
        panic!("expected markup hover");
    };
    assert!(
        markup.value.contains("`myop : Bool`"),
        "编辑后 hover 必须给**新**签名（旧签名 = 假话 ✗）：{:?}",
        markup.value
    );
    assert!(
        !markup.value.contains("`myop : Nat`"),
        "编辑后不许再出现旧签名：{:?}",
        markup.value
    );

    shutdown(&mut service).await;
    let _ = std::fs::remove_dir_all(&dir);
}

use super::*;

#[test]
fn every_semantic_kind_maps_to_a_legend_entry() {
    // `token_type_index` has an `.expect`, but make the totality explicit:
    // every `SemanticKind::ALL` value resolves to an index inside the legend.
    let legend = semantic_token_types();
    for kind in SemanticKind::ALL {
        let index = token_type_index(*kind) as usize;
        assert!(
            index < legend.len(),
            "{kind:?} maps out of the legend (index {index}, len {})",
            legend.len()
        );
    }
}

#[tokio::test]
async fn semantic_tokens_full_classifies_def_example_hole() {
    let src = "def two : Nat := 2\nexample : Sort 1 := sorry\n";
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, src).await;
    let _ = wait_diagnostics(&mut socket, "semantic tokens diagnostics").await;

    let tokens = request_semantic_tokens(&mut service).await;
    assert_eq!(
        absolutize(&tokens),
        vec![
            (0, 0, 3, SemanticTokenType::KEYWORD),   // def
            (0, 4, 3, SemanticTokenType::FUNCTION),  // two（声明）
            (0, 10, 3, SemanticTokenType::VARIABLE), // Nat（未知标识符）
            (0, 17, 1, SemanticTokenType::NUMBER),   // 2
            (1, 0, 7, SemanticTokenType::KEYWORD),   // example（换行后绝对起点）
            (1, 10, 4, SemanticTokenType::TYPE),     // Sort
            (1, 15, 1, SemanticTokenType::NUMBER),   // 1
            (1, 20, 5, SemanticTokenType::MACRO),    // sorry（UTF-16 长度 3）
        ],
        "full token stream for {src:?}"
    );
    shutdown(&mut service).await;
}

#[tokio::test]
async fn semantic_tokens_full_handles_non_ascii_identifiers() {
    let src = "def α_id : Prop -> Prop := fun (x : Prop) => x\n";
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, src).await;
    let _ = wait_diagnostics(&mut socket, "semantic tokens diagnostics").await;

    let tokens = request_semantic_tokens(&mut service).await;
    assert_eq!(
        absolutize(&tokens),
        vec![
            (0, 0, 3, SemanticTokenType::KEYWORD),    // def
            (0, 4, 4, SemanticTokenType::FUNCTION),   // α_id（α 是 BMP，1 个 UTF-16 单元）
            (0, 11, 4, SemanticTokenType::TYPE),      // Prop
            (0, 19, 4, SemanticTokenType::TYPE),      // Prop
            (0, 27, 3, SemanticTokenType::KEYWORD),   // fun（按 UTF-16 是 27，按字节会是 28）
            (0, 32, 1, SemanticTokenType::PARAMETER), // x
            (0, 36, 4, SemanticTokenType::TYPE),      // Prop
            (0, 45, 1, SemanticTokenType::PARAMETER), // x（") => x"）
        ],
        "positions must be UTF-16 code units, not bytes/chars: {src:?}"
    );
    shutdown(&mut service).await;
}

#[tokio::test]
async fn semantic_tokens_highlight_axiom_connectives_as_types() {
    // And/Or 这类 axiom 是教学语言的逻辑类型/命题：声明与使用都映射到
    // TYPE（此前落到 VARIABLE，主题里几乎无色）。
    let src = "axiom And : Prop -> Prop -> Prop\n#check And\n";
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, src).await;
    let _ = wait_diagnostics(&mut socket, "semantic tokens diagnostics").await;

    let tokens = request_semantic_tokens(&mut service).await;
    assert_eq!(
        absolutize(&tokens),
        vec![
            (0, 0, 5, SemanticTokenType::KEYWORD), // axiom
            (0, 6, 3, SemanticTokenType::TYPE),    // And（声明 = AxiomName）
            (0, 12, 4, SemanticTokenType::TYPE),   // Prop
            (0, 20, 4, SemanticTokenType::TYPE),   // Prop
            (0, 28, 4, SemanticTokenType::TYPE),   // Prop
            (1, 0, 6, SemanticTokenType::KEYWORD), // #check
            (1, 7, 3, SemanticTokenType::TYPE),    // And（使用 = AxiomUse）
        ],
        "axiom connective names must be type-colored: {src:?}"
    );
    shutdown(&mut service).await;
}

#[test]
fn encoder_counts_utf16_units_for_supplementary_identifiers() {
    // 🦀 是增补平面字符：1 char = 2 UTF-16 单元；按 char 计数会得到 9。
    let src = "def 🦀x : Prop := Prop\n";
    let spans = sokonanoda_front::semantic::semantic_tokens(src);
    let tokens = encode_semantic_tokens(src, &spans);
    assert_eq!(
        absolutize(&tokens),
        vec![
            (0, 0, 3, SemanticTokenType::KEYWORD),  // def
            (0, 4, 3, SemanticTokenType::FUNCTION), // 🦀x：起点 4，长度 2+1=3
            (0, 10, 4, SemanticTokenType::TYPE),    // Prop：UTF-16 绝对起点 10（char 会是 9）
            (0, 18, 4, SemanticTokenType::TYPE),    // Prop
        ],
    );
}

#[test]
fn encoder_emits_nothing_for_untokenizable_text() {
    // 词法错误截断后仍产出已收集部分的 token；纯标点行不产出 token。
    let src = "-- 只有注释\n: :\n";
    let spans = sokonanoda_front::semantic::semantic_tokens(src);
    assert!(encode_semantic_tokens(src, &spans).is_empty());
    let empty: Vec<SemanticSpan> = Vec::new();
    assert!(encode_semantic_tokens("", &empty).is_empty());
}

/// **回归（2026-10-01，用户反馈「输入几行代码后，整份代码的颜色全乱了」）**：
/// `didChange` 之后**立刻**问语义 token，答的必须是**当前缓冲区**的文本。
///
/// 病根：`semantic_tokens_full` 原来读 `docs.text()` —— 那是**上一次编译用的**
/// 文本，在编译装回之前一直落后于用户缓冲区。而 token 的消费者是客户端：VS Code
/// 把它画到**当前**缓冲区上 ⇒ 编辑点之后**每个 token 都错位**，输入的行越多错得
/// 越远（真进程最小复现：在最前面插一行后，首 token 仍答 `(0,0,7)`，而 `theorem`
/// 已经在新文本的第 2 行）。
///
/// 判据是**位置**，不是"有没有 token"：插一行之后 `theorem` 必须落在**第 2 行**。
/// 答旧文本时它落在第 1 行 —— **反向验证**就是这条会红。
#[tokio::test]
async fn semantic_tokens_follow_the_buffer_not_the_last_compile() {
    let v1 = "theorem a (P : Prop) (h : P) : P := h\n";
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, v1).await;
    let _ = wait_diagnostics(&mut socket, "semantic tokens diagnostics").await;

    // 在最前面插一行，**不等诊断** —— 这正是"击键那一刻"的形状（编译还在飞）。
    let v2 = "-- a brand new comment line\ntheorem a (P : Prop) (h : P) : P := h\n";
    did_change(&mut service, 2, v2).await;

    let abs = absolutize(&request_semantic_tokens(&mut service).await);
    assert_eq!(
        abs.first().cloned(),
        Some((1, 0, 7, SemanticTokenType::KEYWORD)),
        "`theorem` 必须在**第 2 行**（v2 的第 1 行是注释）；\
         答 (0, 0, 7) 就是拿了 v1 的 token ⇒ 客户端整体错位：{abs:?}"
    );
    assert!(
        abs.iter().all(|&(line, ..)| line >= 1),
        "v2 的第 1 行是注释，不该有任何 token 落在它上面：{abs:?}"
    );
}

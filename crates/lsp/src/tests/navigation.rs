use super::*;

#[tokio::test]
async fn document_symbols_list_declarations() {
    let src = format!("{VALID}{EXERCISE}");
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, &src).await;
    let _ = wait_diagnostics(&mut socket, "didOpen diagnostics").await;

    let result = call(
        &mut service,
        RpcRequest::build("textDocument/documentSymbol")
            .params(json!({"textDocument": {"uri": URI}}))
            .id(4)
            .finish(),
    )
    .await
    .expect("documentSymbol must answer");
    let symbols: Option<DocumentSymbolResponse> =
        serde_json::from_value(result).expect("valid DocumentSymbolResponse");
    let DocumentSymbolResponse::Nested(symbols) =
        symbols.expect("document symbols must be returned")
    else {
        panic!("expected nested document symbols");
    };
    let id = symbols
        .iter()
        .find(|s| s.name == "id")
        .expect("symbol for `id`");
    assert!(
        id.detail.as_deref().unwrap_or_default().contains("solved"),
        "checked def detail should carry the status label, got {:?}",
        id.detail
    );
    let exercise = symbols
        .iter()
        .find(|s| s.name.starts_with("example"))
        .expect("symbol for the open example");
    assert!(
        exercise
            .detail
            .as_deref()
            .unwrap_or_default()
            .contains("exercise: open"),
        "example detail should carry the status label, got {:?}",
        exercise.detail
    );
    shutdown(&mut service).await;
}

#[tokio::test]
async fn completion_lists_keywords_sorts_prelude_and_declarations() {
    let src = "def two : Nat := 2\nexample : Sort 1 := sorry\n";
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, src).await;
    let _ = wait_diagnostics(&mut socket, "completion diagnostics").await;

    let items = request_completions(&mut service).await;
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    for expected in [
        "def", "fun", "#check", "Prop", "Sort", "Nat", "Nat.add", "Eq.refl", "two",
    ] {
        assert!(
            labels.contains(&expected),
            "completion must list `{expected}`: {labels:?}"
        );
    }
    assert!(
        !labels.iter().any(|l| l.starts_with("_example")),
        "internal names must not be offered: {labels:?}"
    );
    let two = items.iter().find(|i| i.label == "two").expect("two");
    assert_eq!(two.kind, Some(CompletionItemKind::FUNCTION));
    assert!(
        two.detail.as_deref().is_some_and(|d| d.contains("def")),
        "detail carries kind/status: {:?}",
        two.detail
    );
    // 文档里的签名是 `sokonanoda` 代码块（与 hover 同一围栏语言，§7）。
    let documentation = match &two.documentation {
        Some(Documentation::MarkupContent(markup)) => markup.value.clone(),
        other => panic!("expected markdown documentation, got {other:?}"),
    };
    assert!(
        documentation.contains("```sokonanoda") && documentation.contains("def two : Nat"),
        "signature docs must be a sokonanoda fence: {documentation:?}"
    );
    shutdown(&mut service).await;
}

#[tokio::test]
async fn folding_ranges_cover_multiline_declarations_only() {
    let src = "def one : Nat :=\n  1\nexample : Sort 1 := sorry\n";
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, src).await;
    let _ = wait_diagnostics(&mut socket, "folding diagnostics").await;

    let result = call(
        &mut service,
        RpcRequest::build("textDocument/foldingRange")
            .params(json!({"textDocument": {"uri": URI}}))
            .id(61)
            .finish(),
    )
    .await
    .expect("foldingRange must answer");
    let ranges: Option<Vec<FoldingRange>> =
        serde_json::from_value(result).expect("valid FoldingRange");
    let ranges = ranges.expect("folding ranges");
    assert_eq!(
        ranges.len(),
        1,
        "only the two-line declaration folds: {ranges:?}"
    );
    assert_eq!(ranges[0].start_line, 0);
    assert_eq!(ranges[0].end_line, 1);
    shutdown(&mut service).await;
}

#[tokio::test]
async fn goto_definition_jumps_from_use_to_binder() {
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, VALID).await;
    let _ = wait_diagnostics(&mut socket, "goto def diagnostics").await;

    let body_x = VALID.rfind('x').expect("body `x` exists");
    let target = goto_definition_at(&mut service, VALID, body_x).await;
    let GotoDefinitionResponse::Scalar(location) =
        target.expect("binder use must resolve to a definition")
    else {
        panic!("expected a scalar definition location");
    };
    let binder_at = VALID.find("(x : Prop)").expect("binder text exists");
    assert_eq!(location.range.start, lsp_pos(VALID, binder_at));
    assert_eq!(
        location.range.end,
        lsp_pos(VALID, binder_at + "(x : Prop)".len())
    );
    shutdown(&mut service).await;
}

#[tokio::test]
async fn goto_definition_jumps_from_check_use_to_declaration() {
    let src = format!("{VALID}#check id\n");
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, &src).await;
    let _ = wait_diagnostics(&mut socket, "goto def decl diagnostics").await;

    let use_id = src.find("#check id").expect("#check id exists") + "#check ".len();
    let target = goto_definition_at(&mut service, &src, use_id).await;
    let GotoDefinitionResponse::Scalar(location) =
        target.expect("top-level use must resolve to a definition")
    else {
        panic!("expected a scalar definition location");
    };
    let decl_end = "def id : Prop -> Prop := fun (x : Prop) => x".len();
    assert_eq!(location.range.start, lsp_pos(&src, 0));
    assert_eq!(location.range.end, lsp_pos(&src, decl_end));
    shutdown(&mut service).await;
}

#[tokio::test]
async fn goto_definition_returns_none_without_resolution() {
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, VALID).await;
    let _ = wait_diagnostics(&mut socket, "goto def none diagnostics").await;

    let target = goto_definition_at(&mut service, VALID, offset_of(VALID, "Prop")).await;
    assert!(
        target.is_none(),
        "prelude names have no source definition: {target:?}"
    );
    shutdown(&mut service).await;
}

#[tokio::test]
async fn document_highlight_lists_all_uses_of_one_definition() {
    let src = "def double : Nat -> Nat := fun (n : Nat) => n + n\n";
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, src).await;
    let _ = wait_diagnostics(&mut socket, "highlight diagnostics").await;

    let body = src.find("n + n").expect("body uses exist");
    let expected = vec![lsp_pos(src, body), lsp_pos(src, body + 4)];

    // 从使用点请求：该定义的所有使用点都高亮。
    let highlights = document_highlight_at(&mut service, src, body)
        .await
        .expect("uses of `n` must highlight");
    assert_eq!(highlights.len(), 2, "both `n` uses: {highlights:?}");
    let starts: Vec<Position> = highlights.iter().map(|h| h.range.start).collect();
    assert_eq!(starts, expected);
    assert!(
        highlights
            .iter()
            .all(|h| h.kind == Some(DocumentHighlightKind::TEXT)),
        "highlights are text-level: {highlights:?}"
    );

    // 从 binder 定义处请求：同样高亮全部使用点。
    let binder = src.find("(n : Nat)").expect("binder exists");
    let highlights = document_highlight_at(&mut service, src, binder)
        .await
        .expect("highlighting the binder finds its uses");
    let starts: Vec<Position> = highlights.iter().map(|h| h.range.start).collect();
    assert_eq!(starts, expected, "binder highlight lists all uses");
    shutdown(&mut service).await;
}

#[tokio::test]
async fn completion_offers_in_scope_binders() {
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, VALID).await;
    let _ = wait_diagnostics(&mut socket, "completion binder diagnostics").await;

    let pos = lsp_pos(VALID, VALID.rfind('x').expect("body `x` exists"));
    let items = request_completions_at(&mut service, pos).await;
    let binder = items
        .iter()
        .find(|i| i.label == "x")
        .expect("in-scope binder `x` must be offered");
    assert_eq!(binder.kind, Some(CompletionItemKind::VARIABLE));
    assert!(
        binder
            .detail
            .as_deref()
            .is_some_and(|d| d.contains("binder")),
        "detail marks the binder scope: {:?}",
        binder.detail
    );
    shutdown(&mut service).await;
}

#[tokio::test]
async fn selection_range_grows_from_arrow_to_enclosing_type() {
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, DEMO_K).await;
    let _ = wait_diagnostics(&mut socket, "selection range diagnostics").await;

    // 光标放在类型里第二个箭头 `a -> a` 的 `->` 上（该段先结合）。
    let arrow_at = DEMO_K.find("a -> a").expect("inner arrow exists") + 2;
    let inner = selection_range_at(&mut service, DEMO_K, arrow_at)
        .await
        .expect("arrow position must yield a chain");
    // 第一级：正好是 `a -> a`（内层函数类型，先结合）。
    let start_off = DEMO_K.find("a -> a").expect("span start");
    assert_eq!(
        inner.range.start,
        lsp_pos(DEMO_K, start_off),
        "innermost selection must be the arrow expression itself"
    );
    assert_eq!(inner.range.end, lsp_pos(DEMO_K, start_off + "a -> a".len()));
    // 更大的层级存在，且逐级包住内层。
    let mut cur = &inner;
    let mut levels = 1usize;
    while let Some(parent) = cur.parent.as_ref() {
        assert!(
            parent.range.start < inner.range.start,
            "each level must start at-or-before the inner one: {:?} vs {:?}",
            parent.range.start,
            inner.range.start
        );
        levels += 1;
        cur = parent;
    }
    assert!(
        levels >= 2,
        "chain must reach the enclosing type, got {levels}"
    );
    shutdown(&mut service).await;
}

/// **G-39 的判据（T-D23 挖出来的真 bug）**：**import 进来的用户自定义记法符号**
/// 在使用它的文件里**必须**认得出、跳得回**声明它的模块**。
///
/// 病：`notation_at` 原来用 `notation_input::symbol_at`（只看**输入表 + 本文件
/// 声明**）⇒ `⊗` 这种用户自己定的符号不在输入表里，"import 它的文件"里一律
/// `null`。既有跨文件用例没抓到，是因为它用的 `∈` **恰好在输入表里**（`\in`）
/// ——**夹具选得太顺手，把这条路遮住了**。
///
/// 修：`notation_at` 改用**闭包感知**的 `symbol_at_with_sources`（把闭包记法表里
/// 的符号都喂给词法）。判据（真 LSP 探针 `docs/gaps/repro/G39-…js` 也钉着同一条）：
/// definition 落到**库**文件的那一行 `infix`。
#[tokio::test]
async fn an_imported_user_notation_symbol_resolves_into_its_module() {
    let dir = std::env::temp_dir().join(format!(
        "sokonanoda-imported-notation-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp project");
    let lib = dir.join("Lib.sokonanoda");
    std::fs::write(
        &lib,
        "def Lib.op (a b : Prop) : Prop := a\ninfix:60 \" ⊗ \" => Lib.op\n",
    )
    .expect("write lib");
    let src = "import Lib\n\ntheorem t (a b : Prop) (h : a ⊗ b) : a ⊗ b := h\n";
    let entry = dir.join("Canvas.sokonanoda");
    std::fs::write(&entry, src).expect("write entry");
    let uri = Url::from_file_path(&entry).expect("file url");
    let lib_uri = Url::from_file_path(&lib).expect("lib url");

    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    // **必须用 URI 感知的那对 helper**（`did_open_at`/`wait_diagnostics_for`）：
    // 单文件版（`did_open`）不会把文档挂到工作区上 ⇒ **没有项目闭包** ⇒ 量到的
    // 还是"单文件里没有这个符号"。（实测：用单文件版时这条测试假红。）
    testutil::did_open_at(&mut service, &uri, src).await;
    let _ = testutil::wait_diagnostics_for(&mut socket, &uri, "imported user notation").await;

    let pos = lsp_pos(src, src.rfind('⊗').expect("use site"));
    let result = call(
        &mut service,
        RpcRequest::build("textDocument/definition")
            .params(json!({
                "textDocument": {"uri": uri},
                "position": position_json(pos),
            }))
            .id(2)
            .finish(),
    )
    .await
    .expect("definition must answer");
    assert!(
        !result.is_null(),
        "import 的用户记法符号必须跳得回去（G-39）：{result}"
    );
    let location: Location = serde_json::from_value(result).expect("scalar location");
    assert_eq!(
        location.uri, lib_uri,
        "落点是**声明它的模块**（库的那一行 infix）"
    );
    shutdown(&mut service).await;
}

/// **T-D22 的判据**：**不在作用域**的记法符号 ⇒ 导航**不编答案**（`null`）。
///
/// 现场：`notation_input` **刻意忽略 scoping**（`docs/design/notation-input.md`
/// §10 偏差 3）——那是给**输入提示**用的：输入法是全局的，`\in` 在任何地方
/// 都该提示得出来，跟"这个文件 import 了谁"无关。
///
/// 但**导航**是另一回事：问"这个符号是谁定的"必须**以作用域为准**
/// （闭包记法表 + 本文件声明 + 内建）。这份用例把边界钉住：一个**没被 import**
/// 的模块里声明的符号，在这个文件里 `F12`/hover 都答不上来——**不许**因为
/// "词法上认得出它是个符号"就编出一个定义来。
#[tokio::test]
async fn a_notation_symbol_out_of_scope_is_not_resolved() {
    let dir = std::env::temp_dir().join(format!(
        "sokonanoda-scoped-notation-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp project");
    // 库**声明了**这个符号，但入口**没有 import** 它。
    std::fs::write(
        dir.join("SetLib.sokonanoda"),
        "def Set (α : Type) : Type := α -> Prop\n\
def Set.mem (α : Type) (a : α) (A : Set α) : Prop := A a\n\
infix:50 \" ∈ \" => Set.mem\n",
    )
    .expect("write lib");
    let src = "theorem t (α : Type) (a : α) (A : Set α) (h : a ∈ A) : a ∈ A := h\n";
    let entry = dir.join("Canvas.sokonanoda");
    std::fs::write(&entry, src).expect("write entry");
    let uri = Url::from_file_path(&entry).expect("file url");

    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, src).await;
    let _ = wait_diagnostics(&mut socket, "out-of-scope notation").await;

    let pos = lsp_pos(src, src.rfind('∈').expect("use site"));
    let result = call(
        &mut service,
        RpcRequest::build("textDocument/definition")
            .params(json!({
                "textDocument": {"uri": uri},
                "position": position_json(pos),
            }))
            .id(2)
            .finish(),
    )
    .await
    .expect("definition must answer (possibly null)");
    assert!(
        result.is_null(),
        "不在作用域的记法符号不许编出定义（T-D22）：{result}"
    );
    shutdown(&mut service).await;
}

/// **T-D30 的判据（LSP 侧）**：光标在 `(h : a ∈ A)` 的 **`∈`** 上时，
/// `documentHighlight` 不得返回任何东西，`rename` 不得改 `h`。
///
/// 改前实测：`highlight_uses` 的"包含光标即命中"回退会命中**外层 binder**
/// ——binder 的 span 覆盖**整段类型标注** ⇒ `∈` 被解析成 `h`，面板上会把 `h`
/// 的每一处都点亮、`rename` 会去改 `h`。同一个病在 front 的
/// `references::resolve_at` 里也有一份（两条都在 T-D30 修）。
#[tokio::test]
async fn a_notation_symbol_does_not_resolve_to_the_enclosing_binder() {
    let src = "def Set (α : Type) : Type := α -> Prop\n\
def Set.mem (α : Type) (a : α) (A : Set α) : Prop := A a\n\
infix:50 \" ∈ \" => Set.mem\n\
theorem t (α : Type) (a : α) (A : Set α) (h : a ∈ A) : a ∈ A := h\n";
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, src).await;
    let _ = wait_diagnostics(&mut socket, "T-D30 diagnostics").await;

    // 第 4 行（0-based 3）里 `(h : a ∈ A)` 那个 `∈`。
    let line_text = src.lines().nth(3).expect("第四行");
    let line = 3usize;
    let col = line_text.find('∈').expect("那一行有 ∈");
    let offset = src.lines().take(line).map(|l| l.len() + 1).sum::<usize>() + col;

    // **T-D24 之后**：`∈` 上给的是**这个符号自己的每一处**（以前直接返回空 ✗）。
    // 但 T-D30 的**真正意图**一条都不能破：**绝不能点亮外层 binder `h`**
    // （binder 的 span 覆盖整段类型标注 `(h : a ∈ A)`）。所以这里逐段核对：
    // 每一段被点亮的文本都必须**就是符号本身**。
    let highlights = document_highlight_at(&mut service, src, offset).await;
    let highlights = highlights.unwrap_or_default();
    // 别自己把 (line, character) 换成字节——`∈`/`α` 是多字节，手算必然错位
    // （实测：算出来是空格）。用仓库那条既有映射（按构造与 LSP 同口径）。
    let offset_at = |line: u32, character: u32| -> usize {
        crate::position_to_offset(src, Position::new(line, character))
    };
    assert!(
        highlights.len() >= 2,
        "T-D24：`∈` 上应给出它自己的每一处（本文件里 ≥2 处）：{highlights:?}"
    );
    for h in &highlights {
        let start = offset_at(h.range.start.line, h.range.start.character);
        let end = offset_at(h.range.end.line, h.range.end.character);
        assert_eq!(
            &src[start..end],
            "∈",
            "点亮的必须是符号本身，绝不能是外层 binder `h`（T-D30）：{h:?}"
        );
    }

    // `rename` 在 `∈` 上必须被**拒绝**（`InvalidParams`）——改前它会改掉外层
    // binder `h`（实测：两处编辑，其中一处正是行尾那个 `h`）。
    let req = RpcRequest::build("textDocument/rename")
        .params(json!({
            "textDocument": {"uri": URI},
            "position": position_json(lsp_pos(src, offset)),
            "newName": "k",
        }))
        .id(9)
        .finish();
    use tower::Service;
    use tower::ServiceExt;
    let err = service
        .ready()
        .await
        .expect("service ready")
        .call(req)
        .await
        .expect("rename answers")
        .expect("an error response")
        .into_parts()
        .1
        .expect_err("记法符号上没有可以改名的名字");
    assert!(
        err.message.contains("没有可以改名的名字"),
        "拒绝的理由要说清：{err:?}"
    );
    shutdown(&mut service).await;
}

/// **T-D40 的 LSP 层**：`go to definition` 在**记法符号**上跳到**声明它的模块**里
/// 那一行（`infix:50 " ∈ " => Set.mem`）。
///
/// 这是 T-D10 加的记法分支的判据：`definition_at` 只认"名字的使用点"，
/// 而记法符号的 `resolution` 是 `None` ⇒ 以前直接返回 `null`
/// （e2e 用例 #7 覆盖的是同一条路的真宿主版本）。
#[tokio::test]
async fn goto_definition_on_a_notation_symbol_lands_on_its_declaration() {
    let dir = std::env::temp_dir().join(format!(
        "sokonanoda-def-notation-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp project");
    let lib = dir.join("SetLib.sokonanoda");
    std::fs::write(
        &lib,
        "def Set (α : Type) : Type := α -> Prop\n\
def Set.mem (α : Type) (a : α) (A : Set α) : Prop := A a\n\
infix:50 \" ∈ \" => Set.mem\n",
    )
    .expect("write lib");
    let entry = dir.join("Canvas.sokonanoda");
    let src = "import SetLib\n\ntheorem mem_self (α : Type) (a : α) (A : Set α) (h : a ∈ A) : a ∈ A := h\n";
    std::fs::write(&entry, src).expect("write entry");
    let uri = Url::from_file_path(&entry).expect("file url");
    let lib_uri = Url::from_file_path(&lib).expect("lib url");

    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    testutil::did_open_at(&mut service, &uri, src).await;
    let _ = testutil::wait_diagnostics_for(&mut socket, &uri, "notation definition").await;

    let pos = lsp_pos(src, src.rfind('∈').expect("use site"));
    let result = call(
        &mut service,
        RpcRequest::build("textDocument/definition")
            .params(json!({
                "textDocument": {"uri": uri},
                "position": position_json(pos),
            }))
            .id(3)
            .finish(),
    )
    .await
    .expect("definition must answer");
    let location: Option<GotoDefinitionResponse> =
        serde_json::from_value(result).expect("valid definition response");
    let location = location.expect("记法符号必须有跳转目标（以前返回 null）");
    let (uri, range) = match location {
        GotoDefinitionResponse::Scalar(location) => (location.uri, location.range),
        other => panic!("expected a single location: {other:?}"),
    };
    assert_eq!(uri, lib_uri, "要跳到**声明它的模块**");
    // 声明那一行（0-based 2）里的 `infix`。
    assert_eq!(range.start.line, 2, "落在 `infix` 那一行：{range:?}");
    assert_eq!(range.start.character, 0, "从行首开始：{range:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// **E05 的判据（T-D50 / 缺口 G-37 的「跳转那一半」）**：光标落在**记法声明的
/// 目标名**上（`prefix:70 " 𝒫 " => Set.powerset` 里的 `Set.powerset`）⇒
/// 落点必须是 **`def powerset` 那一行**，**不许是光标自己那一行** ✗✓。
///
/// 现场（2026-09-27 实测真课程库 `courses/set-theory/lib/Set.sokonanoda`）：
/// `Set.powerset` **应落 L81 实落 L125**、`Set.compl` **应落 L79 实落 L126** ——
/// 落的都是光标自己那一行 ⇒ F12 **视觉上等于没反应** ✗。
/// 根因：`crates/lsp/src/lib.rs` 的记法目标分支把 `project_definition` 返回的
/// **真定义 span** 用 `_` 丢掉，`range` 用了**光标处**那个 span（就是目标名自己）✗。
///
/// ⚠ 判据断言的是**落点行号 == `def` 那一行**这个**具体值** ✓ —— 只断言"非 null"
/// 是**看不见**这个 bug 的（自跳也是非 null ✗，G-37 的 status 就这样被误标成
/// `fixed` 过 ✓）。真课程库那两条的**端到端**核对由复现件
/// `docs/gaps/repro/G37-notation-decl-target-not-a-use-point.js` 承担 ✓（它要起真
/// LSP、跑真课程闭包；这里用**小项目夹具**是为了单测能秒级反馈 ✓）。
///
/// **反向验证**：把 `range_of(def_span)` 换回光标那个 span ⇒ 本用例判红 ✓。
#[tokio::test]
async fn goto_definition_on_a_notation_target_name_lands_on_the_definition() {
    let dir = std::env::temp_dir().join(format!(
        "sokonanoda-def-notation-target-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp project");
    // 记法声明与目标**必须在同一个模块（入口）里**：单文件 / 非入口模块下
    // `project_definition()` 返回 `None`（`crates/front/src/query/mod.rs:377`）
    // ⇒ 测到的就不是本条这个 bug 了 ✗✓（实测：合成夹具把记法声明放 lib 里 ⇒ 得到 null ✗）。
    // ⚠ 目标名是**限定名**（`Set.powerset`），而 `project_definition()` 按
    //   `decl.name` **逐字**匹配（`crates/front/src/query/mod.rs:383`）⇒ 声明必须
    //   在 `namespace Set` 里（真课程库就是这种写法）✗✓ —— 写成裸 `def powerset`
    //   会得到 null（那是"测不到这条 bug"，不是"bug 没了"✗）。
    // 行号（0-based）：0 = import · 2 = `namespace Set` · 3 = `def powerset`
    // · 4 = 记法声明（**光标在这行**）· 5 = `end Set`。
    let src = "import SetLib\n\
               \n\
               namespace Set\n\
               def powerset (α : Type) (A : Set α) : Set α := fun (a : α) => A a\n\
               prefix:70 \" 𝒫 \" => Set.powerset\n\
               end Set\n";
    // ⚠ **必须有清单**：`project_definition()` 只在**项目模式**下工作
    //   （`crates/front/src/query/mod.rs:377`「单文件模式返回 None」）——
    //   真课程库有 `courses/set-theory/sokonanoda.toml` ✓，夹具没有就永远是 null ✗✓。
    std::fs::write(
        dir.join("sokonanoda.toml"),
        "entry = \"Canvas.sokonanoda\"\n",
    )
    .expect("write manifest");
    let entry = dir.join("Canvas.sokonanoda");
    std::fs::write(&entry, src).expect("write entry");
    let lib = dir.join("SetLib.sokonanoda");
    std::fs::write(
        &lib,
        "def Set (α : Type) : Type := α -> Prop\n\
         def Set.mem (α : Type) (a : α) (A : Set α) : Prop := A a\n\
         infix:50 \" ∈ \" => Set.mem\n",
    )
    .expect("write lib");
    let uri = Url::from_file_path(&entry).expect("file url");

    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    testutil::did_open_at(&mut service, &uri, src).await;
    let _ = testutil::wait_diagnostics_for(&mut socket, &uri, "notation target definition").await;

    // 光标落在记法声明的**目标名**上（`=>` 后面的 `Set.powerset`）。
    let decl_line = 4usize;
    let want_line = 3u32; // `def powerset`
    let column = src
        .lines()
        .nth(decl_line)
        .expect("notation line")
        .find("Set.powerset")
        .expect("target name on the notation line");
    let result = call(
        &mut service,
        RpcRequest::build("textDocument/definition")
            .params(json!({
                "textDocument": {"uri": uri},
                "position": {"line": decl_line, "character": column},
            }))
            .id(4)
            .finish(),
    )
    .await
    .expect("definition must answer");
    let location: Option<GotoDefinitionResponse> =
        serde_json::from_value(result).expect("valid definition response");
    let location = location.expect("记法声明的目标名必须有跳转目标（G-37 前半：不许 null）");
    let (landed, range) = match location {
        GotoDefinitionResponse::Scalar(location) => (location.uri, location.range),
        other => panic!("expected a single location: {other:?}"),
    };
    assert_eq!(landed, uri, "定义就在这份文件里");
    assert_ne!(
        range.start.line, decl_line as u32,
        "**自跳**（落在光标自己那一行 = F12 视觉上没反应）✗：{range:?}"
    );
    assert_eq!(
        range.start.line, want_line,
        "必须落在 `def powerset` 那一行（0-based {want_line}）：{range:?}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// **E10 的判据（用户可见的那一跳）**：在**学生文件**里对**内建记法** `∧` 按 `F12`
/// ⇒ 必须落在 **prelude 源**里那条声明行上（`-- sokonanoda:builtin-notation "∧" => And` ✓）。
///
/// 前置（两条 front 接缝判据，已绿 ✓）：闭包记法表**带**内建（含指令行 span）
/// · 词法在光标处**认得出** `∧` ⇒ `notation_at` 才答得上 ✓；本条钉的是 **LSP 那一跳**：
/// `module: None` 的内建要走 `prelude_source_path()` ✓
///（以前 `let module = module?` 把内建直接丢掉 ⇒ F12 毫无反应 ✗）。
///
/// ⚠ **夹具必须满足两条**（E10 后半四次红**真正的成因**，探针实测 ✓）：
/// ① 有 `sokonanoda.toml`；② **源里必须有 `import` 行** —— LSP 侧"是不是项目"看的是
/// `project::source_has_import_line`/`is_project_source` ✓，光有清单不够 ✗
///（缺 import 时 `project_modules() => None` ⇒ `notation_at` 第一行
/// `self.project.as_ref()?` 直接断掉 ⇒ 落到后续分支 = 学生文件自跳 ✗）。
///
/// **反向验证**：删掉 `lib.rs` 里 `module.is_none() && span.start.offset != 0` 那段 ⇒ 判红 ✓。
#[tokio::test]
async fn goto_definition_on_a_builtin_notation_lands_in_the_prelude() {
    let dir = std::env::temp_dir().join(format!(
        "sokonanoda-def-builtin-notation-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    std::fs::write(
        dir.join("sokonanoda.toml"),
        "entry = \"Canvas.sokonanoda\"\n",
    )
    .expect("manifest");
    std::fs::write(
        dir.join("SetLib.sokonanoda"),
        "def Set (α : Type) : Type := α -> Prop\n",
    )
    .expect("lib");
    let src = "import SetLib\n\ntheorem and_comm (a b : Prop) (h : a ∧ b) : b ∧ a :=\n  And.intro b a (And.right a b h) (And.left a b h)\n";
    let entry = dir.join("Canvas.sokonanoda");
    std::fs::write(&entry, src).expect("entry");
    let uri = Url::from_file_path(&entry).expect("url");

    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    testutil::did_open_at(&mut service, &uri, src).await;
    let _ = testutil::wait_diagnostics_for(&mut socket, &uri, "builtin notation definition").await;

    let offset = src.find('∧').expect("symbol");
    let before = &src[..offset];
    let line = before.matches('\n').count() as u32;
    let character = before
        .rsplit('\n')
        .next()
        .map(|s| s.chars().count())
        .unwrap_or(0) as u32;
    let result = call(
        &mut service,
        RpcRequest::build("textDocument/definition")
            .params(json!({"textDocument": {"uri": uri}, "position": {"line": line, "character": character}}))
            .id(9)
            .finish(),
    )
    .await
    .expect("definition answers");
    let location: Option<GotoDefinitionResponse> =
        serde_json::from_value(result).expect("valid response");
    let location = location.expect("内建记法必须有跳转目标（E10：指令登记的声明点）✗");
    let (landed, range) = match location {
        GotoDefinitionResponse::Scalar(location) => (location.uri, location.range),
        other => panic!("expected a single location: {other:?}"),
    };
    let path = landed.to_file_path().expect("file url");
    let text = std::fs::read_to_string(&path).expect("read the landed file");
    assert!(
        text.contains("-- sokonanoda:builtin-notation"),
        "落点必须是 **prelude 源**（里面有 E10 的指令行）：{}",
        path.display()
    );
    let landed_line = text
        .lines()
        .nth(range.start.line as usize)
        .expect("landed line");
    assert!(
        landed_line
            .trim_start()
            .starts_with("-- sokonanoda:builtin-notation \"∧\" =>"),
        "必须落在 `∧` 那条指令行上 ✗（实际：{landed_line:?}）"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// **用户反馈（2026-10-10）**：`"a = b"` 里的 `=` 按 `F12` 必须**有落点**。
///
/// 现场：`=` 是**三处故意特例**的叠加（`LEXER_NATIVE_SYMBOLS` 剔除 + 旧注释不登记
/// + `span.start.offset != 0` 的闸门）⇒ `definition` **无声返回 `null`** ✗
/// （既不报错也不跳，用户原话：「等于号没有跳转」）。本轮按**与另外 5 个内建记法
/// 对称**的方案修：prelude 登记区补一行 `-- sokonanoda:builtin-notation "=" => Eq` ✓。
///
/// 这条钉的是**单文件模式**（无 `import` / 无清单）—— `Query::notation_at` 只在项目
/// 模式下工作 ⇒ 走的是**项目无关**的那条兜底（`notation::builtin_declaration_span` ✓）。
/// 项目模式那一半见下一条（`goto_definition_on_the_equals_notation_in_a_project_...`）。
///
/// **反向验证**：删掉 prelude 里那行登记 ⇒ `builtin_declaration_span("=")` = `None`
/// ⇒ 本用例判红（`=` 必须有跳转目标）✓。
#[tokio::test]
async fn goto_definition_on_the_equals_notation_lands_in_the_prelude() {
    let dir = std::env::temp_dir().join(format!(
        "sokonanoda-def-equals-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    // ⚠ **故意**不写 `sokonanoda.toml`、不写 `import` ⇒ 单文件模式 ✓。
    let src = "theorem eq_self (a b : Prop) (h : a = b) : a = b := h\n";
    let entry = dir.join("Solo.sokonanoda");
    std::fs::write(&entry, src).expect("entry");
    let uri = Url::from_file_path(&entry).expect("url");

    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    testutil::did_open_at(&mut service, &uri, src).await;
    let _ = testutil::wait_diagnostics_for(&mut socket, &uri, "equals notation definition").await;

    // 用户实际点的字符位置：`h : a = b` 里的那个 `=`（不是"能跑通的位置" ✗）。
    let pos = lsp_pos(src, src.find(" = ").expect("the `=` the user clicks") + 1);
    let result = call(
        &mut service,
        RpcRequest::build("textDocument/definition")
            .params(json!({"textDocument": {"uri": uri}, "position": position_json(pos)}))
            .id(11)
            .finish(),
    )
    .await
    .expect("definition answers");
    let location: Option<GotoDefinitionResponse> =
        serde_json::from_value(result).expect("valid response");
    let location = location.expect("`=` 必须有跳转目标（本轮修的正是它）✗");
    let (landed, range) = match location {
        GotoDefinitionResponse::Scalar(location) => (location.uri, location.range),
        other => panic!("expected a single location: {other:?}"),
    };
    let path = landed.to_file_path().expect("file url");
    let prelude_path = sokonanoda_front::compile::prelude_source_path().expect("prelude 路径");
    assert_eq!(
        path.canonicalize().expect("canonicalize landed"),
        prelude_path.canonicalize().expect("canonicalize prelude"),
        "`=` 的落点必须是 **prelude 源**（E10 的登记行在那里）✗"
    );
    let text = std::fs::read_to_string(&path).expect("read the landed file");
    let landed_line = text
        .lines()
        .nth(range.start.line as usize)
        .expect("landed line");
    // **断言落点行号**（只断言"非 null"会放过原地跳 ✗ —— E05/G-37 的教训）：
    // 落点行必须是 `=` 的登记行，且**不是**用户点的那一行（自跳）✗。
    assert!(
        landed_line
            .trim_start()
            .starts_with("-- sokonanoda:builtin-notation \"=\" => Eq"),
        "必须落在 `=` 那条登记行上 ✗（实际：{landed_line:?}）"
    );
    assert_ne!(
        landed_line.trim_start(),
        src.lines().next().expect("entry line").trim_start(),
        "落点不许是用户点的那一行（原地跳）✗"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// **同上，项目模式那一半**（用户原话：「有 import / 单文件都一样」✗ ⇒ 两边都要修 ✓）：
/// 有清单 + `import` 行时，`notation_at` 会给出内建记法的 span（E10 那条路 ✓），
/// 但它**必须不是 offset 0** —— 那正是本轮给 `=` 补登记行要修的闸门 ✓。
#[tokio::test]
async fn goto_definition_on_the_equals_notation_in_a_project_lands_in_the_prelude() {
    let dir = std::env::temp_dir().join(format!(
        "sokonanoda-def-equals-project-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    std::fs::write(
        dir.join("sokonanoda.toml"),
        "entry = \"Canvas.sokonanoda\"\n",
    )
    .expect("manifest");
    std::fs::write(
        dir.join("SetLib.sokonanoda"),
        "def Set (α : Type) : Type := α -> Prop\n",
    )
    .expect("lib");
    let src = "import SetLib\n\ntheorem eq_self (a b : Prop) (h : a = b) : a = b := h\n";
    let entry = dir.join("Canvas.sokonanoda");
    std::fs::write(&entry, src).expect("entry");
    let uri = Url::from_file_path(&entry).expect("url");

    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    testutil::did_open_at(&mut service, &uri, src).await;
    let _ = testutil::wait_diagnostics_for(&mut socket, &uri, "equals project definition").await;

    let pos = lsp_pos(src, src.find(" = ").expect("the `=` the user clicks") + 1);
    let result = call(
        &mut service,
        RpcRequest::build("textDocument/definition")
            .params(json!({"textDocument": {"uri": uri}, "position": position_json(pos)}))
            .id(12)
            .finish(),
    )
    .await
    .expect("definition answers");
    let location: Option<GotoDefinitionResponse> =
        serde_json::from_value(result).expect("valid response");
    let location = location.expect("项目模式下 `=` 也必须有跳转目标 ✗");
    let (landed, range) = match location {
        GotoDefinitionResponse::Scalar(location) => (location.uri, location.range),
        other => panic!("expected a single location: {other:?}"),
    };
    let path = landed.to_file_path().expect("file url");
    let text = std::fs::read_to_string(&path).expect("read the landed file");
    let landed_line = text
        .lines()
        .nth(range.start.line as usize)
        .expect("landed line");
    assert!(
        landed_line
            .trim_start()
            .starts_with("-- sokonanoda:builtin-notation \"=\" => Eq"),
        "必须落在 `=` 那条登记行上 ✗（实际：{landed_line:?}）"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// **用户反馈（2026-10-10）的第二跳**：prelude 登记注释里的**目标名**按 `F12` 必须跳。
///
/// 现场（用户原话）：「prelude 里的 `-- sokonanoda:builtin-notation "∧" => And`
/// 是注释，点击 `And` 无法跳转」✗ —— 注释**不产生 token** ⇒
/// `notation_input::notation_target_at` 的词法扫描永远认不出它 ⇒ F12 / hover /
/// documentHighlight 三条全 `null` ✗（与仓库白纸黑字的**两跳模型**矛盾：
/// `docs/gaps/repro/G23-notation-navigation.js`「`∈` → 记法声明 → 定义」）。
///
/// 这条钉的是**用户实际点的字符位置**（那行注释里 `And` 的字符上 ✓）与**落点行号**
/// （`inductive And` 那一行 ✓ —— 只断言"非 null"会放过原地跳 ✗）。
///
/// **反向验证**：撤掉 `comment_directive_target_at` 那一半（`notation_target_at`
/// 只剩声明形态）⇒ 本用例判红（`And 必须有跳转目标`）✓。
#[tokio::test]
async fn goto_definition_on_a_comment_registration_target_lands_on_its_definition() {
    let path = sokonanoda_front::compile::prelude_source_path().expect("prelude 源落盘路径");
    // ⚠ **规范化**：`prelude_source_path()` 带 `..`，而 URI 过一趟 JSON（`json!`）会被
    // URL 规范**归一化** ⇒ 测试手里的 `uri` 与服务端发布诊断用的 `uri` 不相等，
    // `wait_diagnostics_for` 会**空等到超时** ✗（2026-10-10 实测踩到）。
    let path = path.canonicalize().expect("canonicalize prelude");
    let text = std::fs::read_to_string(&path).expect("read prelude");
    let uri = Url::from_file_path(&path).expect("file url");
    let want_line = text
        .lines()
        .position(|line| line.trim_start().starts_with("inductive And "))
        .expect("`inductive And` 在 prelude 里") as u32;
    let clicked_line = text
        .lines()
        .position(|line| {
            line.trim_start()
                .starts_with("-- sokonanoda:builtin-notation \"∧\" =>")
        })
        .expect("`∧` 的登记行在 prelude 里") as u32;

    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    testutil::did_open_at(&mut service, &uri, &text).await;
    let _ = testutil::wait_diagnostics_for(&mut socket, &uri, "comment target definition").await;

    // 用户实际点的字符：`=>` 之后的 `And` 的第一个字符 ✓。
    let needle = "-- sokonanoda:builtin-notation \"∧\" => ";
    let pos = lsp_pos(&text, offset_of(&text, needle) + needle.len());
    assert_eq!(
        pos.line, clicked_line,
        "夹具前提：光标必须落在**那条注释行**上（第 {clicked_line} 行）"
    );
    let result = call(
        &mut service,
        RpcRequest::build("textDocument/definition")
            .params(json!({"textDocument": {"uri": uri}, "position": position_json(pos)}))
            .id(13)
            .finish(),
    )
    .await
    .expect("definition answers");
    let location: Option<GotoDefinitionResponse> =
        serde_json::from_value(result).expect("valid response");
    let location = location.expect("注释登记行里的目标名必须有跳转目标（第二跳）✗");
    let (landed, range) = match location {
        GotoDefinitionResponse::Scalar(location) => (location.uri, location.range),
        other => panic!("expected a single location: {other:?}"),
    };
    let landed_path = landed.to_file_path().expect("file url");
    assert_eq!(
        landed_path.canonicalize().expect("canonicalize landed"),
        path.canonicalize().expect("canonicalize prelude"),
        "`And` 的定义就在 prelude 源里（它不是模块 ⇒ 不走 project_definition）"
    );
    let landed_text = std::fs::read_to_string(&landed_path).expect("read landed");
    let landed_line = landed_text
        .lines()
        .nth(range.start.line as usize)
        .expect("landed line");
    assert_eq!(
        range.start.line, want_line,
        "必须落在 `inductive And` 那一行（0-based {want_line}）✗ —— 实际：{landed_line:?}"
    );
    assert!(
        landed_line.trim_start().starts_with("inductive And "),
        "落点行必须是 `inductive And` 的声明行 ✗（实际：{landed_line:?}）"
    );
    assert_ne!(
        range.start.line, clicked_line,
        "**自跳**（落在光标自己那一行 = F12 视觉上没反应）✗"
    );
}

/// **G-55 的判据（2026-10-07 用户授权"本轮做"；语义定案 = ①+②+③ 累积式）**。
///
/// 现场（E08 实测，真课程库 11 条）：`textDocument/documentHighlight` 对记法**目标名**
/// **全部返回 `null`** ✗ —— 目标名在 AST 里不是使用点、也没有 `resolution`，
/// 而反查要先"至少有一个使用处"起头（`render::highlight_uses`）。
///
/// **设计问题的答案**（台账 G-55 `today`：「目标名的『同一个定义』包含哪些位置？」）
/// ⇒ **① + ② + ③ 全都要**，因为**目标名是"引用"，不是"引入处"**（记法声明引入的是
/// **符号**，不是那个名字）：
///   * **① 光标处这个名字** —— **地板**：永远在结果里 ⇒ 永不 `null`、且**一定包含
///     用户实际点的那个位置** ✓（目标在闭包外时这就是全部答案 ⇒ **不依赖 G-54** ✓）；
///   * **② 本文件里展开到这个目标的记法符号的每一处** —— `𝒫 A` 与 `Set.powerset α A`
///     指的是**同一个定义**，符号只是写法不同（与 T-D24 在符号上的答案同一份词法）；
///   * **③ 目标在本文件里可解析时**：**定义名**那一处 + **点名使用处**。
/// **用户语义一句话**：点这个名字 ⇒ 看到"这个词（连同它的各种写法）指向同一个定义的
/// 所有位置"。
///
/// ⚠ **与用户倾向 ① 的差异（写清理由 ✓）**：只答 ① 在 VS Code 里是**可见退化** ——
/// 语言服务器的非 `null` 结果会**取代**编辑器自己的文本级兜底高亮（VS Code 1.138：
/// `documentHighlightProvider` 的 `*` 兜底 provider + `first non-null` 合并；本仓
/// `.vscode-test` 里的 bundle 实测），而今天那行上文本兜底正好点亮 `def powerset`
/// 那一处 ⇒ 只答一个词等于把它灭掉 ✗；语义上把"引用"当"引入处"也说不通 ✗。
/// ① 作为**地板**保留 ✓（完整理由与证据：`docs/design/notation-subset.md` §20）。
///
/// **判据（用户动作，三层里的 wire 层）**：在**记法声明行上那个名字**（用户实际点的
/// 位置）请求 ⇒ **非 `null`**、**含该位置**、且含 ②③ 的**每一处**（精确集合 ⇒
/// 不许混入别的、也不许重复）。
#[tokio::test]
async fn document_highlight_on_a_notation_target_lists_the_same_definition() {
    let dir = std::env::temp_dir().join(format!(
        "sokonanoda-hl-notation-target-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp project");
    std::fs::write(
        dir.join("sokonanoda.toml"),
        "entry = \"Canvas.sokonanoda\"\n",
    )
    .expect("write manifest");
    // 目标**在闭包内**（`Set.powerset` 就在本文件里）—— ③ 那两层才有东西可答。
    // ⚠ 必须有 `import` 行 + 清单：`project_definition()` 只在**项目模式**下工作
    //   （E05 的夹具注释，`crates/front/src/query/mod.rs:817`）✗✓。
    // 行号（0-based）：0 = import · 2 = `namespace Set` · 3 = `def powerset`
    // · 4 = 记法声明（**光标在这行的目标名上**）· 5 = 用它的定理 · 6 = `end Set`。
    let src = "import SetLib\n\
               \n\
               namespace Set\n\
               def powerset (α : Type) (A : Set α) : Set α := fun (a : α) => A a\n\
               prefix:70 \" 𝒫 \" => Set.powerset\n\
               theorem t (α : Type) (A : Set α) (h : 𝒫 A = Set.powerset α A) : 𝒫 A = Set.powerset α A := h\n\
               end Set\n";
    let entry = dir.join("Canvas.sokonanoda");
    std::fs::write(&entry, src).expect("write entry");
    std::fs::write(
        dir.join("SetLib.sokonanoda"),
        "def Set (α : Type) : Type := α -> Prop\n",
    )
    .expect("write lib");
    let uri = Url::from_file_path(&entry).expect("file url");

    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    testutil::did_open_at(&mut service, &uri, src).await;
    let _ = testutil::wait_diagnostics_for(&mut socket, &uri, "notation target highlight").await;

    // **用户实际点的那个位置**：声明行上目标名的第一个字符（`Set.powerset` 的 `S`）。
    let clicked = lsp_pos(src, offset_of(src, "=> Set.powerset") + 3);
    let result = call(
        &mut service,
        RpcRequest::build("textDocument/documentHighlight")
            .params(json!({
                "textDocument": {"uri": uri},
                "position": position_json(clicked),
            }))
            .id(6)
            .finish(),
    )
    .await
    .expect("documentHighlight must answer");
    let highlights: Option<Vec<DocumentHighlight>> =
        serde_json::from_value(result).expect("valid highlight response");
    let highlights = highlights.expect(
        "记法声明的目标名必须给出高亮（G-55：不许 `null` ✗ —— E08 的防漂移用例\n\
         就是钉住那个 `null` 的，本轮按设计改成断言新语义 ✓）",
    );

    // **(a) 用户动作判据**：结果必须**包含用户实际点的那个位置**。
    assert!(
        highlights.iter().any(|h| h.range.start == clicked),
        "结果必须包含光标处那个名字（用户实际点的位置）✗：clicked={clicked:?} · {highlights:?}"
    );

    // 精确集合（按位置排序）：① 名字自己 · ③ 定义名 · ②③ 定理那行上的每一处。
    let mut expected: Vec<Position> =
        vec![clicked, lsp_pos(src, offset_of(src, "def powerset") + 4)];
    let theorem_line = 5usize;
    let line_start: usize = src
        .split('\n')
        .take(theorem_line)
        .map(|l| l.len() + 1)
        .sum();
    let line = src.lines().nth(theorem_line).expect("theorem line");
    for needle in ["𝒫", "Set.powerset"] {
        for (i, _) in line.match_indices(needle) {
            expected.push(lsp_pos(src, line_start + i));
        }
    }
    expected.sort_by_key(|p| (p.line, p.character));
    let starts: Vec<Position> = highlights.iter().map(|h| h.range.start).collect();
    assert_eq!(
        starts, expected,
        "「同一个定义」的位置集合不对 ✗ —— 期望 ①名字 + ③定义名 + ②符号每一处 + ③点名使用处：{highlights:?}"
    );
    assert!(
        highlights
            .iter()
            .all(|h| h.kind == Some(DocumentHighlightKind::TEXT)),
        "highlights are text-level: {highlights:?}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// **G-55 的横向排查守卫**（`AGENTS.md` 验证纪律第 0 条 (b)：同类问题**不许修单点**，
/// 「收到一条 UI 反馈 ⇒ 先问同类还有哪些，一次改齐 + 落**可查**的守卫」✓）。
///
/// 同一类「光标处**不是使用点**」的位置逐条钉住（清单 = `docs/design/notation-subset.md`
/// §20 与台账 G-55 的 `notes`）：
///
/// | 位置 | 现状 | 本条覆盖？ |
/// |---|---|---|
/// | ① 记法声明的**目标名** | **非 null**，含光标处 + 符号每一处（+ 闭包内时定义名与点名使用处） | **✓ 本条**（G-55） |
/// | ② 记法声明**字符串里的符号**（`" 𝒫 "`） | 仍 `null` | ✗ **另一条**：符号的语义由 T-D24 定在**使用处**；声明行那一处是符号的**引入处**，答它是另一个问题，且要新的词法 span API |
/// | ③ **没有使用处的声明名**（`def unused` / `axiom ax`） | 仍 `null` | ✗ **另一条**：定义名没有 hover 行，而反查要先有一个使用处 ⇒ 0 使用处就空手（Lean 的答案是「定义处 + 使用处」⇒ 真答案在**定义名自己**身上） |
/// | ④ `inductive` / `ctor` 名 | **非 null**，但 range 里混入类型位/返回位 | ✗ **另一条**（精度问题，不是"答不上"） |
///
/// 判据的作用：**谁将来动了这一类位置，这条就判红** ⇒ 必须回来同步清单与台账
/// （E08 那条"断言当前行为"的防漂移用例正是这么用的 —— 本轮 G-55 真做时它按设计判红 ✓）。
#[tokio::test]
async fn non_use_positions_are_pinned_as_a_class() {
    let src = "def Set (α : Type) : Type := α -> Prop\n\
               namespace Set\n\
               def powerset (α : Type) (A : Set α) : Set α := fun (a : α) => A a\n\
               prefix:70 \" 𝒫 \" => Set.powerset\n\
               theorem t (α : Type) (A : Set α) (h : 𝒫 A = 𝒫 A) : 𝒫 A = 𝒫 A := h\n\
               end Set\n\
               def unused : Nat := 1\n\
               axiom ax : Prop\n\
               inductive W : Type\n\
               ctor mk : W\n\
               end\n";
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, src).await;
    let _ = wait_diagnostics(&mut socket, "class sweep diagnostics").await;

    // ① 本条覆盖：记法声明的目标名 ⇒ 非 null 且**含用户实际点的那个位置**。
    let clicked_offset = offset_of(src, "=> Set.powerset") + 3;
    let clicked = lsp_pos(src, clicked_offset);
    let covered = document_highlight_at(&mut service, src, clicked_offset)
        .await
        .expect("① 记法目标名：本条覆盖 ⇒ 不许 null（G-55）");
    assert!(
        covered.iter().any(|h| h.range.start == clicked),
        "① 必须含光标处那个名字（用户实际点的位置）：{covered:?}"
    );
    assert!(
        covered.len() > 1,
        "① 还要答上这个文件里 `𝒫` 的每一处（② 那一层）：{covered:?}"
    );

    // ② 记法声明字符串里的符号：仍 null（另一条，理由见上表）。
    assert!(
        document_highlight_at(&mut service, src, offset_of(src, "\" 𝒫 \"") + 2)
            .await
            .is_none(),
        "② 记法声明字符串里的符号**本轮不覆盖**（另一条）—— 若你把它做出来了，\
         请回来更新本条与 docs/design/notation-subset.md §20 的清单 ✓"
    );

    // ③ 没有使用处的声明名：仍 null（另一条）。
    for needle in ["def unused", "axiom ax"] {
        let offset = offset_of(src, needle) + needle.find(' ').expect("space") + 1;
        assert!(
            document_highlight_at(&mut service, src, offset)
                .await
                .is_none(),
            "③ `{needle}`（0 使用处）**本轮不覆盖**（另一条）—— 做出来了请同步清单 ✓"
        );
    }

    // ④ `inductive`/`ctor` 名：答得上（非 null）—— 精度是另一条。
    for needle in ["inductive W", "ctor mk"] {
        let offset = offset_of(src, needle) + needle.find(' ').expect("space") + 1;
        assert!(
            document_highlight_at(&mut service, src, offset)
                .await
                .is_some(),
            "④ `{needle}` 现状是**答得上**（非 null）；若它变成 null，那是回退 ✗"
        );
    }
}

/// **E07 的判据（落点语义定案的那一半）**：记法声明的目标名**不在闭包里**时
/// （真课程库 `lib/Set.sokonanoda` 的 `Set.image`/`Set.preimage`/`Set.prod` —— 它们
/// 定义在 `lib/Image.sokonanoda:32,37`，那份文件**不在** `lib/Set.sokonanoda` 的
/// 闭包里）⇒ `textDocument/definition` **诚实返回 `null`** ✓：
/// **不编一个位置** ✗、**也不自跳** ✗（自跳正是 E05 修掉的那个假动作）。
///
/// **E07 定案（2026-09-27 深夜，用户授权自决）**：
/// ① **落点语义与编辑器内 F12 保持一致** —— 这条分支就是编辑器 F12 走的那条
///    （`textDocument/definition` 的唯一入口）⇒ "一致"是**构造上成立**的 ✓：
///    闭包里 ⇒ 落定义行（E05 ✓）；闭包外 ⇒ `null`（本用例 ✓）。
///    "扩到整个项目/课程仓去找"要先编译闭包外的模块，代价与收益不成比例 ⇒
///    **只登记、不实现** ✓（台账 G-37 + PLAN §E07）。
/// ② **高亮颜色沿用现有目标样式** —— 实测**已经一致** ✓：真课程库那 11 条目标名
///    在 front 层全是 `SemanticKind::DefUse`（防漂移判据
///    `crates/front/src/semantic.rs::every_notation_target_name_gets_the_same_colour` ✓）。
///    ⚠ 计划里"这 3 条落 `UnknownIdent` → `variable`、另 8 条是 `function`"是
///    **过时描述** ✗✓（T-D50 的 `or_insert(DefUse)` 已经改掉了 ✓）。
#[tokio::test]
async fn a_notation_target_out_of_the_closure_is_not_fabricated() {
    let dir = std::env::temp_dir().join(format!(
        "sokonanoda-def-notation-outside-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp project");
    std::fs::write(
        dir.join("sokonanoda.toml"),
        "entry = \"Canvas.sokonanoda\"\n",
    )
    .expect("write manifest");
    // `Set.image` **没有任何模块声明它**（真场景：它声明在闭包外的 `lib/Image`）。
    let src = "def Set (α : Type) : Type := α -> Prop\n\
               infixr:80 \" '' \" => Set.image\n";
    let entry = dir.join("Canvas.sokonanoda");
    std::fs::write(&entry, src).expect("write entry");
    let uri = Url::from_file_path(&entry).expect("file url");

    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    testutil::did_open_at(&mut service, &uri, src).await;
    let _ =
        testutil::wait_diagnostics_for(&mut socket, &uri, "out-of-closure notation target").await;

    let decl_line = 1usize;
    let column = src
        .lines()
        .nth(decl_line)
        .expect("notation line")
        .find("Set.image")
        .expect("target name");
    let result = call(
        &mut service,
        RpcRequest::build("textDocument/definition")
            .params(json!({
                "textDocument": {"uri": uri},
                "position": {"line": decl_line, "character": column},
            }))
            .id(5)
            .finish(),
    )
    .await
    .expect("definition must answer");
    let location: Option<GotoDefinitionResponse> =
        serde_json::from_value(result).expect("valid definition response");
    assert!(
        location.is_none(),
        "闭包外的记法目标必须**诚实为 null**（不编位置、也不自跳）✗：{location:?}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// 为什么它和 `∈` 不是一条路：`∈` 是 `infix:` 声明出来的**记法**，跳转走
/// `notation_at`（查记法表 ✓）；`{a}` 是**内建语法**（`ast::Expr::SetLiteral`），
/// 根本不在记法表里 ⇒ 那条分支够不着，而 `definition_at` 只读 hover 的
/// `resolution`，`elab.rs` 当年给集合字面量记的是 `resolution: None`
/// ⇒ F12 直接 `null`。修法：记它指向展开目标（`Set.singleton` / `Set.pair`）的
/// `ResolvedTarget::Declaration`，真实位置由既有回填给。
#[tokio::test]
async fn goto_definition_on_a_set_literal_lands_on_its_expansion_target() {
    let dir = std::env::temp_dir().join(format!(
        "sokonanoda-def-setlit-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp project");
    let lib = dir.join("SetLib.sokonanoda");
    std::fs::write(
        &lib,
        "def Set (\u{3b1} : Type) : Type := \u{3b1} -> Prop\n\
def Set.singleton (\u{3b1} : Type) (a : \u{3b1}) : Set \u{3b1} := fun (x : \u{3b1}) => x = a\n",
    )
    .expect("write lib");
    let entry = dir.join("Canvas.sokonanoda");
    let src = "import SetLib\n\ntheorem sing_eq (\u{3b1} : Type) (a : \u{3b1}) : Set.singleton \u{3b1} a = {a} := sorry\n";
    std::fs::write(&entry, src).expect("write entry");
    let uri = Url::from_file_path(&entry).expect("file url");
    let lib_uri = Url::from_file_path(&lib).expect("lib url");

    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    testutil::did_open_at(&mut service, &uri, src).await;
    // **夹具自检前置断言**（纪律：编排出来的夹具先证明它自己是好的 ——
    // 2026-09-26 实测踩过 5 轮：`({a} : Prop)` 非法、删了 `∈` 却没定义
    // `Set.mem`、`{a}` 缺期望类型，全是夹具自身坏，与产品无关）。
    let diags = testutil::wait_diagnostics_for(&mut socket, &uri, "set literal definition").await;
    assert!(
        !diags
            .diagnostics
            .iter()
            .any(|d| d.severity == Some(DiagnosticSeverity::ERROR)),
        "夹具不许有**错误级**诊断（`sorry` 警告是合法状态）：{:?}",
        diags.diagnostics
    );

    // 光标落在**左花括号**上：`{a}` 内部那个 `a` 是局部变量（它自己有一条
    // `ResolvedTarget::Binder` 的 hover 行，"最小的使用点胜出"⇒ 在 `a` 上按 F12
    // 应该跳**变量**，那是 Lean 的行为）。符号位是括号 ⇒ 判据取 `{`。
    let pos = lsp_pos(src, src.find("{a}").expect("set literal use site"));
    let result = call(
        &mut service,
        RpcRequest::build("textDocument/definition")
            .params(json!({
                "textDocument": {"uri": uri},
                "position": position_json(pos),
            }))
            .id(3)
            .finish(),
    )
    .await
    .expect("definition must answer");
    let location: Option<GotoDefinitionResponse> =
        serde_json::from_value(result).expect("valid definition response");
    let location = location.expect("`{a}` 必须有跳转目标（A3 之前返回 null）");
    let (uri, range) = match location {
        GotoDefinitionResponse::Scalar(location) => (location.uri, location.range),
        other => panic!("expected a single location: {other:?}"),
    };
    assert_eq!(uri, lib_uri, "要跳到声明 `Set.singleton` 的模块");
    assert_eq!(
        range.start.line, 1,
        "落在 `def Set.singleton` 那一行：{range:?}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// **A4 判据**（2026-09-26 用户报告第 4 条）：prelude 要像 Lean 4 —— 有内嵌代码
/// 且**可跳转**。`Or` / `And` / `Iff` / `False` 这些**内置前奏**的名字，F12 必须
/// 落到**前奏源文件里的真 span**。
///
/// 判据不是"返回了个位置就算" ✗ —— 要**读回那个位置**、断言那一行**就是**
/// `Or` 的声明行 ✓（否则"跳到哪都算绿"）。
///
/// 为什么以前跳不了：prelude 名字的 hover 行按设计 `resolution: None`
/// （`report.rs`：prelude 没有定义位置）⇒ `definition_at` 答不上来 ⇒ F12 静默无反应 ✗。
///
/// **反向验证**：把 `goto_definition` 里那段 prelude 分支删掉 ⇒ 本判据判红
/// （实测返回 `null`）。
#[tokio::test]
async fn goto_definition_on_a_prelude_name_lands_in_the_prelude_source() {
    let src = "theorem use_or (a b : Prop) : Or a b -> Or a b := fun (h : Or a b) => h\n";
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, src).await;
    let _ = wait_diagnostics(&mut socket, "didOpen (prelude definition)").await;

    let at = src.find("Or").expect("source mentions Or") as u32;
    let result = call(
        &mut service,
        RpcRequest::build("textDocument/definition")
            .params(json!({
                "textDocument": {"uri": URI},
                "position": {"line": 0, "character": at},
            }))
            .id(9)
            .finish(),
    )
    .await
    .expect("definition must answer");
    let location: Option<GotoDefinitionResponse> =
        serde_json::from_value(result).expect("valid GotoDefinitionResponse");
    let Some(GotoDefinitionResponse::Scalar(location)) = location else {
        panic!("`Or` 上 F12 必须给一个**标量位置**（A4 之前是 null ✗），实际 = {location:?}");
    };
    let path = location.uri.to_file_path().expect("file url");
    assert!(
        path.file_name().and_then(|n| n.to_str()) == Some("Prelude.sokonanoda"),
        "跳转必须落在**前奏源文件**上，实际 = {}",
        path.display()
    );
    let text = std::fs::read_to_string(&path).expect("prelude source must be readable");
    let line = text
        .lines()
        .nth(location.range.start.line as usize)
        .expect("range line must exist in the prelude source");
    assert!(
        line.starts_with("inductive Or "),
        "range 必须指向 `Or` 的**声明行**，实际第 {} 行 = {line:?}",
        location.range.start.line
    );
    assert_eq!(
        location.range.start.character, 0,
        "声明行必须从行首开始（span 来自真 parser ✓）"
    );

    // 用户第 4 条要的是"**真实位置与签名**"两件 ⇒ hover 那一半也要有断言
    // （它以前就不是静默的，但没有任何测试钉住"prelude 名字上 hover 有内容"）。
    let hover = call(
        &mut service,
        RpcRequest::build("textDocument/hover")
            .params(json!({
                "textDocument": {"uri": URI},
                "position": {"line": 0, "character": at},
            }))
            .id(10)
            .finish(),
    )
    .await
    .expect("hover must answer");
    let value = hover
        .get("contents")
        .and_then(|c| c.get("value"))
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    assert!(
        value.contains("Or"),
        "prelude 名字上 hover 必须给出签名（含名字本身），实际 = {value:?}"
    );
}

/// **G-108 判据**（2026-10-10 用户实测）：**一处报错（`sorry` / 失败的 tactic）
/// 不许打死同一份文件里正确代码的 F12** ——
/// `theorem … := by apply h; exact Eq.refl a; sorry` 里 `Eq.refl` 的跳转以前是
/// `null` ✗。
///
/// 为什么（`walk.rs`）：开放练习那条路**只 elaborate 签名**（`open_signature`），
/// `lower_value`/`build_*` 的 `Err` 臂**把已收到的 hover 行丢掉** ⇒ 证明体里的名字
/// **一个 hover 行都没有** ⇒ `definition_at` 无从下手、prelude 回退也切不出名字 ✗。
/// 修法是给那段补**词法行**（`expr: None`，不推类型、**不改判定** ✓）。
///
/// 这里走**用户实际按的那条路**（`textDocument/definition`）钉两端：
/// * `Eq.refl`（prelude 名字）⇒ 必须落到 `Prelude.sokonanoda`；
/// * `lib_id`（本文件名字）⇒ 必须落到本文件里那条声明的**行首**。
///
/// **反向验证**：把 `walk.rs::push_lexical_hover_rows` 里那几行 push 撤掉 ⇒
/// 本判据判红（实测：`B_redundant_sorry` 那份只剩签名行，`Eq.refl` 返回 `null`）。
#[tokio::test]
async fn goto_definition_inside_a_failing_or_open_proof_still_lands() {
    // 四种状态各一段（用户的 B/C 都在里面；`A` 是对照组）。
    let cases = [
        (
            "A_clean",
            "theorem t (a : Nat) : lib_id a = lib_id a := by\n  exact Eq.refl (lib_id a)\n",
        ),
        (
            "B_redundant_sorry",
            "theorem t (a : Nat) : lib_id a = lib_id a := by\n  exact Eq.refl (lib_id a)\n  sorry\n",
        ),
        (
            "B2_open",
            "theorem t (a : Nat) : And (lib_id a = lib_id a) True := by\n  constructor\n  exact Eq.refl (lib_id a)\n  sorry\n",
        ),
        (
            "C_failed_tactic",
            "theorem t (a : Nat) : lib_id a = lib_id a := by\n  exact Nat.zero\n  exact Eq.refl (lib_id a)\n",
        ),
    ];
    for (tag, body) in cases {
        let src = format!("def lib_id (n : Nat) : Nat := n + 1\n{body}");
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, &src).await;
        let _ = wait_diagnostics(&mut socket, &format!("didOpen ({tag})")).await;

        // offset → (line, character)；全部按**源码自己**算，别写死行号。
        let pos_of = |at: usize| {
            let line = src[..at].matches('\n').count();
            let col = at - (src[..at].rfind('\n').map(|i| i + 1).unwrap_or(0));
            (line, col)
        };
        // ① prelude 名字：必须落到前奏源文件。
        let at = src.find("Eq.refl").expect("source mentions Eq.refl") + 3;
        let (line, col) = pos_of(at);
        let result = call(
            &mut service,
            RpcRequest::build("textDocument/definition")
                .params(json!({
                    "textDocument": {"uri": URI},
                    "position": {"line": line, "character": col},
                }))
                .id(11)
                .finish(),
        )
        .await
        .expect("definition must answer");
        let location: Option<GotoDefinitionResponse> =
            serde_json::from_value(result).expect("valid GotoDefinitionResponse");
        let Some(GotoDefinitionResponse::Scalar(location)) = location else {
            panic!(
                "[{tag}] `Eq.refl` 上 F12 必须有位置（G-108 之前是 null ✗），实际 = {location:?}"
            );
        };
        let path = location.uri.to_file_path().expect("file url");
        assert!(
            path.file_name().and_then(|n| n.to_str()) == Some("Prelude.sokonanoda"),
            "[{tag}] `Eq.refl` 必须落到前奏源，实际 = {}",
            path.display()
        );

        // ② 本文件名字（体里最后一处 `lib_id`）：必须落到那条声明的行首。
        let local_at = src.rfind("lib_id").expect("source mentions lib_id") + 2;
        let (line, col) = pos_of(local_at);
        let result = call(
            &mut service,
            RpcRequest::build("textDocument/definition")
                .params(json!({
                    "textDocument": {"uri": URI},
                    "position": {"line": line, "character": col},
                }))
                .id(12)
                .finish(),
        )
        .await
        .expect("definition must answer");
        let location: Option<GotoDefinitionResponse> =
            serde_json::from_value(result).expect("valid GotoDefinitionResponse");
        let Some(GotoDefinitionResponse::Scalar(location)) = location else {
            panic!("[{tag}] 体里的 `lib_id` 上 F12 必须有位置（G-108），实际 = {location:?}");
        };
        assert_eq!(
            location.uri.as_str(),
            URI,
            "[{tag}] `lib_id` 必须跳回**本文件**（同一个 URI），实际 = {}",
            location.uri
        );
        assert_eq!(
            location.range.start.line, 0,
            "[{tag}] `lib_id` 的声明在第 0 行（`def lib_id …`），实际 = {location:?}"
        );
        assert_eq!(
            location.range.start.character, 0,
            "[{tag}] 落点必须在行首（span 来自真 parser ✓），实际 = {location:?}"
        );
    }
}

/// **G-108 的悬停面**（2026-10-10 用户追问：「hover 只剩名字是啥意思，东西坏了吗？」）：
/// 报错/`sorry` 那段源码里的名字，悬停必须给**声明卡片**（与别处 hover 同一个名字
/// **同形**），不许只给一个光秃秃的名字。
///
/// 为什么会有"只剩名字"那一版：修 F12 时给那段补的是**词法行**（不 elaborate ⇒
/// 没有类型）⇒ 渲染只能显示**源码切片**（就一个名字）—— 比修 F12 之前（落回整张
/// 声明卡片）**信息更少** ✗。现在词法行带 `lexical` 标记，LSP 侧看到"切片是个
/// 标识符"就改答 `declaration_card_hover`（自带 prelude 的合成 `#print` 回退 ✓）。
///
/// 判据（真 `textDocument/hover`）：
/// * 失败 tactic **之后**那行里的 `Eq.refl`（prelude 名字）⇒ 卡片（含 `Eq.refl` 与 `:`）；
/// * 同一段里的**局部 binder 名**（`a`）⇒ 仍是源码切片（查不到卡片 ⇒ 诚实回落 ✓，
///   不编一张假卡片 ✗）。
#[tokio::test]
async fn hover_inside_a_failing_proof_shows_the_declaration_card() {
    let src = "def lib_id (n : Nat) : Nat := n + 1\n\
               theorem t (a : Nat) : lib_id a = lib_id a := by\n\
               \x20 exact Nat.zero\n\
               \x20 exact Eq.refl (lib_id a)\n";
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, src).await;
    let _ = wait_diagnostics(&mut socket, "didOpen (hover in failing proof)").await;

    let pos_of = |at: usize| {
        let line = src[..at].matches('\n').count();
        let col = at - (src[..at].rfind('\n').map(|i| i + 1).unwrap_or(0));
        (line, col)
    };
    // ① prelude 名字（`exact Eq.refl …` 里的 `Eq.refl`）：必须是**卡片**。
    let at = src.rfind("Eq.refl").expect("source mentions Eq.refl") + 3;
    let (line, col) = pos_of(at);
    let result = call(
        &mut service,
        RpcRequest::build("textDocument/hover")
            .params(json!({
                "textDocument": {"uri": URI},
                "position": {"line": line, "character": col},
            }))
            .id(21)
            .finish(),
    )
    .await
    .expect("hover must answer");
    let hover: Option<Hover> = serde_json::from_value(result).expect("valid Hover");
    let value = match hover.map(|h| h.contents) {
        Some(HoverContents::Markup(m)) => m.value,
        other => panic!("expected markup hover, got {other:?}"),
    };
    assert!(
        value.contains("Eq.refl") && value.contains(':'),
        "报错段里的 `Eq.refl` 悬停必须是**声明卡片**（含 `Eq.refl` 与类型），实际 = {value:?}"
    );
    assert!(
        !value.trim().eq("```sokonanoda\nEq.refl\n```"),
        "不许只给一个光秃秃的名字（那就是用户追问的「只剩名字」✗），实际 = {value:?}"
    );
    // ② 局部 binder 名（`(lib_id a)` 里的 `a`）：查不到卡片 ⇒ 诚实地回落源码切片。
    let a_at = src.rfind("lib_id a)").expect("source mentions lib_id a") + "lib_id ".len();
    let (line, col) = pos_of(a_at);
    let result = call(
        &mut service,
        RpcRequest::build("textDocument/hover")
            .params(json!({
                "textDocument": {"uri": URI},
                "position": {"line": line, "character": col},
            }))
            .id(22)
            .finish(),
    )
    .await
    .expect("hover must answer");
    let hover: Option<Hover> = serde_json::from_value(result).expect("valid Hover");
    let value = match hover.map(|h| h.contents) {
        Some(HoverContents::Markup(m)) => m.value,
        other => panic!("expected markup hover, got {other:?}"),
    };
    assert!(
        !value.contains("axiom") && !value.contains("def "),
        "局部 binder 名**不许**编出一张声明卡片（它没有声明），实际 = {value:?}"
    );
}

// ---- P6：F12 落到 tactic 文档（设计 `docs/design/tactic-docs.md` §5 P6）----

/// 碰「进程级环境变量」与「仓库真源改名」的两个判据互斥。
///
/// 环境变量（`SOKONANODA_DOCS_DIR`）与真文件改名都是**进程全局**状态 ⇒ 与
/// `testutil.rs::HEAVY_LOCK` 同款：判据自己串行，不动任何断言 ✓。
static DOCS_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// 进程级环境变量的**存-改-还**（照 `scripts/gap.py::clean_env` 的纪律：判据跑完
/// 必须回到原值，否则同一进程里后面的判据会被污染 ✗）。
///
/// `Drop` 里还原 ⇒ **panic 也还原** ✓（同一条纪律见 `HiddenRepoDoc`）。
struct EnvGuard {
    key: &'static str,
    old: Option<std::ffi::OsString>,
}

impl EnvGuard {
    fn set(key: &'static str, value: &std::path::Path) -> Self {
        let old = std::env::var_os(key);
        std::env::set_var(key, value);
        Self { key, old }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match self.old.take() {
            Some(value) => std::env::set_var(self.key, value),
            None => std::env::remove_var(self.key),
        }
    }
}

/// 把仓库真源**临时改名藏起来**：模拟"安装形态"（① 档那个**编译期**路径上
/// 没有这份文件 —— 发布产物在用户机器上就是如此）。
///
/// ⚠ **为什么需要它**（设计 §4.9.3 的一个直接推论）：五档解析里 ① 是编译期路径，
/// 只要判据跑在开发树里它就**必然**命中 ⇒ ② 档（插件目录）在开发树里**不可观测**。
/// 要判"② 档本身成立"，只能把 ① 档那份文件挪开；**顺序不改**（开发机上 ① 先命中
/// 是有意为之：改文档立即生效 ✓）。
///
/// `Drop` 的**第一件事**就是还原（照注入纪律 `trap restore EXIT` 的写法：崩了也
/// 还原 ✓）；判据自己还会在还原后核对内容与 `TACTIC_DOCS[i].markdown` 逐字节相同 ✓。
struct HiddenRepoDoc {
    original: std::path::PathBuf,
    hidden: std::path::PathBuf,
}

impl HiddenRepoDoc {
    fn hide(original: std::path::PathBuf) -> Self {
        assert!(
            original.is_file(),
            "本判据只在开发树里跑：① 档的仓库真源必须存在（{}）",
            original.display()
        );
        let hidden = original.with_extension("md.p6-hidden");
        std::fs::rename(&original, &hidden).expect("把仓库真源临时改名");
        Self { original, hidden }
    }

    fn restore(&mut self) {
        if self.hidden.is_file() {
            std::fs::rename(&self.hidden, &self.original).expect("把仓库真源改回来");
        }
    }
}

impl Drop for HiddenRepoDoc {
    fn drop(&mut self) {
        self.restore();
    }
}

/// F12 的答案 → **标量位置**（不是标量就当场判红，顺带把 `null` 说清楚）。
fn definition_location(target: Option<GotoDefinitionResponse>, at: &str) -> Location {
    let Some(GotoDefinitionResponse::Scalar(location)) = target else {
        panic!("`{at}` 上 F12 必须给一个**标量位置**，实际 = {target:?}");
    };
    location
}

/// 位置的落点 → 真实文件路径（`file:` 之外一律判红：本仓只有 `file:` 一种 URI 形态）。
fn definition_file(location: &Location) -> std::path::PathBuf {
    location.uri.to_file_path().unwrap_or_else(|_| {
        panic!(
            "落点必须是 `file:` URI（零自定义 scheme），实际 = {}",
            location.uri
        )
    })
}

/// **14 条 tactic 各一条真能过的写法**（每条 tactic 一行，`anchor` = 该 tactic 所在
/// 那个**唯一**片段）。
///
/// 与 `hover.rs::EVERY_TACTIC_SOURCE` 是**同一份夹具的两个副本**（那边验 hover 摘要、
/// 这边验 F12 落点）：tactic 必须**真的在目标上成立**，`by_steps` 才有那一步 ——
/// 实测：`intro` 用在不匹配的目标上 ⇒ 该声明**零 `by_steps`**（`soko/stateAt` 答
/// `step:-1 / total:0`）⇒ 光标位置根本不在任何步骤里 ✗（这也是判据不能拿"随便一条
/// 目标 + 关键字"当夹具的原因）。
const EVERY_TACTIC_SOURCE: &str = "\
example (a b : Prop) : a → b → a := by
  intro ha hb
  exact ha

example : True := by
  exact True.intro

example (a b : Prop) (h : a) (g : a → b) : b := by
  apply g
  exact h

example (a : Prop) (h : a) : a := by
  assumption

example (n : Nat) : n = n := by
  rfl

example (n : Nat) : Nat := by
  match n with
  | Nat.zero => Nat.zero
  | Nat.succ k => k

example (a : Prop) (h : a) : a ∧ a := by
  constructor
  exact h
  exact h

example (a b : Prop) (h : a) : a ∨ b := by
  left
  exact h

example (a b : Prop) (h : b) : a ∨ b := by
  right
  exact h

inductive Sigma (α : Type) (p : α → Prop) : Prop
ctor mk (w : α) (h : p w) : Sigma α p
end

example (α : Type) (a : α) : Sigma α (fun (x : α) => x = x) := by
  use a
  sorry

example (a : Prop) (h : False) : a := by
  exfalso
  exact h

example (a b : Prop) (h : a ∧ b) : b ∧ a := by
  cases h with
  | intro ha hb => exact And.intro hb ha

example : True := by
  have h : True := True.intro
  exact True.intro

example : True := by
  sorry
";

/// 夹具里那条 tactic 的**关键字**偏移（`anchor` 必须在夹具里唯一）。
fn tactic_keyword_offset(src: &str, anchor: &str) -> usize {
    assert_eq!(
        src.matches(anchor).count(),
        1,
        "anchor 在夹具里必须唯一：{anchor:?}"
    );
    let start = src.find(anchor).expect("anchor 在夹具里");
    let line = anchor.rsplit('\n').next().unwrap_or(anchor);
    let indent = line.len() - line.trim_start().len();
    start + anchor.len() - line.len() + indent
}

/// **P6 判据 ①（含 ③）**：**全部 14 条** tactic 关键字上按 F12 ⇒ 落到**那一条**的
/// 参考文档。
///
/// 每条断言五件：ⓐ 落点是**真文件**（存在 + 可读 —— 物化失败必须 `None` 而不是死
/// 指针 ✗，判据 ③）；ⓑ 内容含 `# <关键字>`；ⓒ `range.start.line` **就是**那一行；
/// ⓓ `range` == **关键字 token 的 span**（D5）；ⓔ **文件名与 §1.1 基线表一致**
/// （防"全跳到同一篇"的假绿 ✗）。
///
/// ⚠ 每条测**两个位置**：关键字的**头一个字符**与**最后一个字符** —— 用户点的就是
/// 那两个字（E27 的教训：拿"能跑通的位置"代替"用户实际点的位置" ✗）。
/// ⚠ 14 条**逐个**跑（只测用户点名的 `intro`/`exact`/`rfl` 等于没测横向 ✗）；清单与
/// 表**双向**对齐（漏一条 / 多一条 / 顺序漂了都判红 ✓）。
#[tokio::test]
async fn goto_definition_on_every_tactic_keyword_lands_in_its_reference_file() {
    let _serial = DOCS_LOCK.lock().await;
    // (anchor, 关键字) —— 顺序 = 夹具顺序 = 表序。
    const CASES: &[(&str, &str)] = &[
        (
            "example (a b : Prop) : a → b → a := by\n  intro ha hb",
            "intro",
        ),
        ("example : True := by\n  exact True.intro", "exact"),
        (
            "example (a b : Prop) (h : a) (g : a → b) : b := by\n  apply g",
            "apply",
        ),
        (
            "example (a : Prop) (h : a) : a := by\n  assumption",
            "assumption",
        ),
        ("example (n : Nat) : n = n := by\n  rfl", "rfl"),
        ("example (n : Nat) : Nat := by\n  match n with", "match"),
        (
            "example (a : Prop) (h : a) : a ∧ a := by\n  constructor",
            "constructor",
        ),
        ("example (a b : Prop) (h : a) : a ∨ b := by\n  left", "left"),
        (
            "example (a b : Prop) (h : b) : a ∨ b := by\n  right",
            "right",
        ),
        (
            "example (α : Type) (a : α) : Sigma α (fun (x : α) => x = x) := by\n  use a",
            "use",
        ),
        (
            "example (a : Prop) (h : False) : a := by\n  exfalso",
            "exfalso",
        ),
        (
            "example (a b : Prop) (h : a ∧ b) : b ∧ a := by\n  cases h with",
            "cases",
        ),
        (
            "example : True := by\n  have h : True := True.intro",
            "have",
        ),
        ("example : True := by\n  sorry", "sorry"),
    ];
    let listed: Vec<&str> = CASES.iter().map(|(_, name)| *name).collect();
    let tabled: Vec<&str> = sokonanoda_front::TACTIC_DOCS
        .iter()
        .map(|d| d.name)
        .collect();
    assert_eq!(
        listed, tabled,
        "14 条清单必须逐条等于 `front::tactics` 表（顺序也要一致）—— 覆盖基线是 14 条 ✓"
    );

    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, EVERY_TACTIC_SOURCE).await;
    let _ = wait_diagnostics(&mut socket, "didOpen (every tactic F12)").await;

    for (anchor, name) in CASES {
        let doc = sokonanoda_front::tactics::doc(name).expect("表里有它");
        let kw = tactic_keyword_offset(EVERY_TACTIC_SOURCE, anchor);
        assert!(
            EVERY_TACTIC_SOURCE[kw..].starts_with(name),
            "夹具里 `{name}` 的关键字偏移算错了：实际 = {:?}",
            &EVERY_TACTIC_SOURCE[kw..kw + name.len()]
        );
        // 用户实际点的是关键字的**头一个字符**与**最后一个字符**。
        for offset in [kw, kw + name.len() - 1] {
            let at = format!("`{name}` 的第 {} 个字符", offset - kw + 1);
            let location = definition_location(
                goto_definition_at(&mut service, EVERY_TACTIC_SOURCE, offset).await,
                &at,
            );
            let path = definition_file(&location);
            // ⓐ 落点是真文件（判据 ③：物化失败必须 `None`，不许给死指针）。
            assert!(
                path.is_file(),
                "{at}：F12 落点必须**真实存在**，实际 = {}",
                path.display()
            );
            let text = std::fs::read_to_string(&path).unwrap_or_else(|err| {
                panic!(
                    "{at}：F12 落点必须**可读**（{err}），实际 = {}",
                    path.display()
                )
            });
            // ⓔ 文件名与 §1.1 基线表一致（防"全跳到同一篇"）。
            let expected_name = format!("{name}.md");
            assert_eq!(
                path.file_name().and_then(|n| n.to_str()),
                Some(expected_name.as_str()),
                "{at}：落点文件名必须是 §1.1 基线表里的 `{expected_name}`，实际 = {}",
                path.display()
            );
            // ⓑ 内容含 `# <关键字>`，且与表里内嵌的那一份**逐字节相同**（单一源）。
            assert!(
                text.contains(&format!("# {name}")),
                "{at}：`{}` 的内容必须含 `# {name}`，实际开头 = {:?}",
                path.display(),
                text.lines().next().unwrap_or_default()
            );
            assert_eq!(
                text, doc.markdown,
                "{at}：落点内容必须与 `TACTIC_DOCS` 里内嵌的那一份逐字节相同（一份源）"
            );
            // ⓒ `range` 指向 `# <关键字>` 那一行。
            let line = text
                .lines()
                .nth(location.range.start.line as usize)
                .unwrap_or_else(|| {
                    panic!(
                        "{at}：range.start.line = {} 超出落点文件（{} 行）",
                        location.range.start.line,
                        text.lines().count()
                    )
                });
            assert_eq!(
                line.trim_end(),
                format!("# {name}"),
                "{at}：range 必须指向 `# {name}` 那一行（不是文件第 0 行、更不是原地跳）"
            );
            // ⓓ `range` == **落点文档里**那个名字 token 的 span（D5）——
            // `Location.range` 是**目标文件**的坐标（拿源文件里光标那个 token 的
            // 行列去比，会把光标停到 `.md` 的第 N 行空白处 ✗，实测过一次）。
            let heading = format!("# {name}");
            let heading_start = text.find(&heading).expect("落点文件有 `# <name>` 标题");
            let name_at = heading_start + "# ".len();
            assert_eq!(
                location.range.start,
                lsp_pos(&text, name_at),
                "{at}：range.start 必须 == 落点文档里标题名 token 的起点"
            );
            assert_eq!(
                location.range.end,
                lsp_pos(&text, name_at + name.len()),
                "{at}：range.end 必须 == 落点文档里标题名 token 的终点"
            );
        }
    }
    shutdown(&mut service).await;
}

/// **P6 判据 ④**：**项位**的 `match` 上按 F12 **仍走既有链** —— 不许被文档分支抢走 ✗。
///
/// 依据：文档分支的触发条件是**两条**（设计 §4.4 D5）：token 是这一步的关键字
/// **且**它落在某条 `by` 步骤的 span 里。值位 `match`（`parser.rs::is_expr_keyword`
/// 认的那个写法）不满足第二条 ⇒ 答案必须与**没有这条分支时**同一个（关键字既不是
/// 名字也不是记法 ⇒ `null`）。
#[tokio::test]
async fn goto_definition_does_not_steal_term_positions() {
    let _serial = DOCS_LOCK.lock().await;
    let src = "def pick (n : Nat) : Nat := match n with\n  \
               | Nat.zero => Nat.zero\n  \
               | Nat.succ k => k\n";
    let kw = src.find("match").expect("源码里有值位 match");
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, src).await;
    let _ = wait_diagnostics(&mut socket, "didOpen (term-position match)").await;
    for offset in [kw, kw + "match".len() - 1] {
        let target = goto_definition_at(&mut service, src, offset).await;
        assert!(
            target.is_none(),
            "值位 `match` 的第 {} 个字符上 F12 必须仍走既有链（关键字没有落点 ⇒ null），\
             实际 = {target:?} —— 若你把它接到 tactic 文档上，那是**抢项位** ✗",
            offset - kw + 1
        );
    }
    shutdown(&mut service).await;
}

/// **P6 判据 ⑥（正反两半）**：**插件目录那一档（② 档）单独判**（设计 §4.9 D10）。
///
/// 三步一条线（同一个 `SOKONANODA_DOCS_DIR` = 临时目录，里面放着 `rfl.md`）：
/// ① 开发树里 ① 档先命中（**有意为之**：改文档立即生效）⇒ 落点 = 仓库真源；
/// ② 把那份真源**临时藏起来**（模拟安装形态：① 档不存在）⇒ 落点必须是**插件目录
///    里那一份**（不是缓存、不是仓库），内容与 `TACTIC_DOCS[i].markdown` **逐字节相同**；
/// ③ **反向的那一半**：把插件目录里那份删掉 ⇒ 解析**继续往下走**（③ 档物化），
///    答一个**真实存在**的文件 —— **不是**死指针 ✗。
///
/// ⚠ 环境变量是**进程全局**的 ⇒ `EnvGuard` 存-改-还；真源改名走 `HiddenRepoDoc`
/// 的 `Drop` ⇒ **两条都是 panic 也还原** ✓。还原后本判据自己核对真源**逐字节**
/// 回到了 `TACTIC_DOCS[i].markdown` ✓（"改回来过"不是"改回来对了"）。
#[tokio::test]
async fn goto_definition_uses_the_plugin_doc_dir_when_the_repo_source_is_absent() {
    let _serial = DOCS_LOCK.lock().await;
    let doc = sokonanoda_front::tactics::doc("rfl").expect("表里有 rfl");
    let src = "theorem t (a : Nat) : a = a := by\n  rfl\n";
    let kw = src.find("rfl").expect("源码里有 rfl");

    // 插件目录：一份**与源逐字节相同**的假文档（R24：两份都是派生物 ⇒ 不许漂移）。
    let plugin_dir = std::env::temp_dir().join(format!("soko-p6-docs-{}", std::process::id()));
    std::fs::create_dir_all(&plugin_dir).expect("建插件目录");
    let plugin_doc = plugin_dir.join("rfl.md");
    std::fs::write(&plugin_doc, doc.markdown).expect("写插件那份文档");
    let _env = EnvGuard::set("SOKONANODA_DOCS_DIR", &plugin_dir);

    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, src).await;
    let _ = wait_diagnostics(&mut socket, "didOpen (plugin docs tier)").await;

    // ① 开发树：① 档（仓库真源）先命中 —— 顺序是设计定的（§4.9.3），不是巧合。
    let first = definition_location(goto_definition_at(&mut service, src, kw).await, "rfl");
    let repo_doc = definition_file(&first);
    assert!(
        repo_doc.is_file() && repo_doc != plugin_doc,
        "开发树里 ① 档必须**先**命中（插件目录设了也不抢）：实际 = {}",
        repo_doc.display()
    );

    // ② 藏起真源 ⇒ ② 档（插件目录）必须命中。
    let mut hidden = HiddenRepoDoc::hide(repo_doc.clone());
    let second = definition_location(goto_definition_at(&mut service, src, kw).await, "rfl");
    let landed = definition_file(&second);
    assert_eq!(
        landed,
        plugin_doc,
        "① 档不存在时必须落到 `SOKONANODA_DOCS_DIR` 那一份（不是缓存、不是仓库）：实际 = {}",
        landed.display()
    );
    assert_eq!(
        std::fs::read_to_string(&landed).expect("插件那份必须可读"),
        doc.markdown,
        "插件那份的内容必须与 `TACTIC_DOCS[i].markdown` 逐字节相同（R24：一份源）"
    );

    // ③ 反向：插件目录里那份**不存在** ⇒ 继续往下走到 ③ 档物化，**不是**死指针 ✗。
    std::fs::remove_file(&plugin_doc).expect("删掉插件那份");
    let third = definition_location(goto_definition_at(&mut service, src, kw).await, "rfl");
    let materialized = definition_file(&third);
    assert_ne!(
        materialized, plugin_doc,
        "插件那份已经不存在了 ⇒ 绝不许把**不存在的路径**当答案（死指针 ✗）"
    );
    assert!(
        materialized.is_file(),
        "② 档不成立 ⇒ 必须继续物化出一个**真实存在**的文件，实际 = {}",
        materialized.display()
    );
    assert_eq!(
        std::fs::read_to_string(&materialized).expect("物化那份必须可读"),
        doc.markdown,
        "物化出来的内容必须与表里那份逐字节相同（幂等写入：与源一致 ✓）"
    );
    match sokonanoda_front::compile::cache::root() {
        // ③ 档：缓存可写 ⇒ 必须落在缓存根下（路径稳定 ⇒ 编辑器里的打开文档/书签不漂）。
        Some(root) if materialized.starts_with(&root) => {}
        // ④ 档：缓存**不可写**（受限沙箱 / 只读 HOME）⇒ 落到临时物化 —— 这正是
        // R10 设计的兜底（prelude 同款三段兜底），也是判据 ③ 要的"**存在且可读**" ✓。
        Some(root) => assert!(
            materialized.starts_with(std::env::temp_dir()),
            "缓存根（{}）下没写成 ⇒ 只许落到 ④ 档临时物化，实际 = {}",
            root.display(),
            materialized.display()
        ),
        None => assert!(
            materialized.starts_with(std::env::temp_dir()),
            "缓存被禁 ⇒ 必须落到 ④ 档临时物化，实际 = {}",
            materialized.display()
        ),
    }

    // 还原真源并**核对内容**（Drop 也做这件事；这里显式做一次是为了能断言 ✓）。
    hidden.restore();
    assert!(repo_doc.is_file(), "真源必须改回来：{}", repo_doc.display());
    assert_eq!(
        std::fs::read_to_string(&repo_doc).expect("真源必须可读"),
        doc.markdown,
        "改回来的真源必须与内嵌的那一份逐字节相同"
    );
    // 还原后再问一次：开发树又回到 ① 档（证明"藏起来"确实是唯一变量 ✓）。
    let after = definition_location(goto_definition_at(&mut service, src, kw).await, "rfl");
    assert_eq!(
        definition_file(&after),
        repo_doc,
        "真源回来后 ① 档必须重新命中"
    );

    let _ = std::fs::remove_dir_all(&plugin_dir);
    shutdown(&mut service).await;
}

/// **P6 判据 ⑦**（父会话追加，G-108 那条线的同一个立场）：**文件里有失败声明 /
/// 诊断时，tactic 关键字上的 F12 仍必须落到文档**。
///
/// G-108 修的是"一处报错不许打死同一份文件里正确代码的 F12"（对**名字**）；
/// P6 的新分支必须守同一条线：文档落点与判定结果**无关**（只查 `by_steps` 的
/// span + 表）⇒ 文件里有没有红，都不该让它静默 ✗。
///
/// 判据自己先断言"这份文件**确实有**诊断"（否则这条判据会退化成 ① 的重复 ✓），
/// 再对 `apply` 的**头一个 / 最后一个字符**各问一次。
///
/// **反向验证**：在文档分支前加一句"有诊断 ⇒ `return Ok(None)`" ⇒ 本判据判红 ✓。
#[tokio::test]
async fn goto_definition_on_a_tactic_keyword_survives_a_failing_declaration() {
    let _serial = DOCS_LOCK.lock().await;
    // 第一段：`apply h` 之后接 `sorry`（用户报的那种形状）；第二段：一条真失败的声明。
    let src = "theorem t (a : Prop) (h : a) : a := by\n  apply h\n  sorry\n\
               theorem broken : Prop := by\n  exact Nat.zero\n";
    let kw = src.find("apply").expect("源码里有 apply");
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, src).await;
    let diagnostics = wait_diagnostics(&mut socket, "didOpen (failing declaration)").await;
    assert!(
        !diagnostics.diagnostics.is_empty(),
        "本判据的前提是这份文件**真的有**诊断（否则它退化成判据 ① 的重复 ✗），实际 = {diagnostics:?}"
    );

    for offset in [kw, kw + "apply".len() - 1] {
        let location = definition_location(
            goto_definition_at(&mut service, src, offset).await,
            "apply（文件里有诊断）",
        );
        let path = definition_file(&location);
        assert_eq!(
            path.file_name().and_then(|n| n.to_str()),
            Some("apply.md"),
            "有诊断时 `apply` 的 F12 仍必须落到它的文档，实际 = {}",
            path.display()
        );
        assert!(path.is_file(), "落点必须真实存在：{}", path.display());
    }
    shutdown(&mut service).await;
}

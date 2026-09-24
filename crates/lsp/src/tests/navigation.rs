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

    let highlights = document_highlight_at(&mut service, src, offset).await;
    assert!(
        highlights.as_ref().is_none_or(|h| h.is_empty()),
        "`∈` 上不该点亮任何名字（尤其不该是外层 binder `h`）：{highlights:?}"
    );

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

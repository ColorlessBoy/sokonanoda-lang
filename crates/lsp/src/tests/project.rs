//! 多文件项目进 LSP（I16 P5）：被 import 的声明可见、缺失的 import 报错、
//! 定义能跳到被导入模块里。
//!
//! 余项（设计 §4.9 登记）：**依赖变更后自动重编译其它打开文档**（跨文件失效）
//! 还没有测试——第一版实现会在 tower-lsp 的串行通知 + 客户端 socket 缓冲下挂住。
//! 当前语义：变更的文档自己重编译；其它打开文档重发上次诊断；入口在下一次
//! **自身**编辑时会看到新环境（项目模式每次都重编译整个闭包）。

use super::*;
use tower_lsp::lsp_types::Url;

fn tmp_dir(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sokonanoda-lsp-project-{tag}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn write(dir: &std::path::Path, name: &str, text: &str) -> Url {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create parent");
    }
    std::fs::write(&path, text).expect("write file");
    Url::from_file_path(&path).expect("file url")
}

const LOGIC: &str = "axiom And : Prop -> Prop -> Prop\n\
axiom And.intro : forall (a b : Prop), a -> b -> And a b\n\
axiom And.left : forall (a b : Prop), And a b -> a\n\
axiom And.right : forall (a b : Prop), And a b -> b\n";

const CANVAS: &str = "import Logic\n\n\
theorem and_swap (a b : Prop) (h : And a b) : And b a := And.intro b a (And.right a b h) (And.left a b h)\n\
theorem swap_back (a b : Prop) (h : And a b) : And a b := and_swap b a (and_swap a b h)\n";

#[tokio::test]
async fn an_imported_module_is_visible_to_the_entry() {
    let dir = tmp_dir("visible");
    let root = Url::from_directory_path(&dir).expect("dir url");
    let (mut service, mut socket) = test_service();
    testutil::handshake_with_root(&mut service, &root).await;

    let _logic = write(&dir, "Logic.sokonanoda", LOGIC);
    let canvas = write(&dir, "Canvas.sokonanoda", CANVAS);
    testutil::did_open_at(&mut service, &canvas, CANVAS).await;
    let diags = testutil::wait_diagnostics_for(&mut socket, &canvas, "canvas diagnostics").await;
    assert!(
        diags.diagnostics.is_empty(),
        "the imported axioms are in scope, so the theorem checks: {:?}",
        diags.diagnostics
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn a_missing_import_is_a_diagnostic_not_a_crash() {
    let dir = tmp_dir("missing");
    let root = Url::from_directory_path(&dir).expect("dir url");
    let (mut service, mut socket) = test_service();
    testutil::handshake_with_root(&mut service, &root).await;

    let canvas = write(&dir, "Canvas.sokonanoda", CANVAS);
    testutil::did_open_at(&mut service, &canvas, CANVAS).await;
    let diags = testutil::wait_diagnostics_for(&mut socket, &canvas, "missing import").await;
    assert!(
        diags
            .diagnostics
            .iter()
            .any(|diag| testutil::code_of(diag) == "import-not-found"),
        "the import line carries the error: {:?}",
        diags.diagnostics
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn definition_jumps_into_the_imported_module() {
    let dir = tmp_dir("goto");
    let root = Url::from_directory_path(&dir).expect("dir url");
    let (mut service, mut socket) = test_service();
    testutil::handshake_with_root(&mut service, &root).await;

    let logic = write(&dir, "Logic.sokonanoda", LOGIC);
    let canvas = write(&dir, "Canvas.sokonanoda", CANVAS);
    testutil::did_open_at(&mut service, &canvas, CANVAS).await;
    let _ = testutil::wait_diagnostics_for(&mut socket, &canvas, "canvas diagnostics").await;

    // 光标落在 `And.right` 这一用的名字上。
    let offset = testutil::offset_of(CANVAS, "And.right");
    let pos = testutil::lsp_pos(CANVAS, offset + 2);
    let req = RpcRequest::build("textDocument/definition")
        .params(serde_json::json!({
            "textDocument": {"uri": canvas},
            "position": testutil::position_json(pos),
        }))
        .id(2)
        .finish();
    let result = testutil::call(&mut service, req)
        .await
        .expect("definition answers");
    let location: tower_lsp::lsp_types::Location =
        serde_json::from_value(result).expect("a Location");
    assert_eq!(
        location.uri, logic,
        "the definition lives in the imported module"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn definition_stays_in_the_entry_for_local_names() {
    let dir = tmp_dir("local");
    let root = Url::from_directory_path(&dir).expect("dir url");
    let (mut service, mut socket) = test_service();
    testutil::handshake_with_root(&mut service, &root).await;

    let _logic = write(&dir, "Logic.sokonanoda", LOGIC);
    let canvas = write(&dir, "Canvas.sokonanoda", CANVAS);
    testutil::did_open_at(&mut service, &canvas, CANVAS).await;
    let _ = testutil::wait_diagnostics_for(&mut socket, &canvas, "canvas diagnostics").await;

    // 项目模式不能把**本文件**的跳转弄丢：`and_swap` 声明在入口里，
    // 光标落在 `swap_back` 体内对它的使用上，目标必须仍是入口自己。
    let use_site = testutil::offset_of(CANVAS, "and_swap b a");
    let pos = testutil::lsp_pos(CANVAS, use_site + 3);
    let req = RpcRequest::build("textDocument/definition")
        .params(serde_json::json!({
            "textDocument": {"uri": canvas},
            "position": testutil::position_json(pos),
        }))
        .id(3)
        .finish();
    let result = testutil::call(&mut service, req)
        .await
        .expect("definition answers");
    let location: tower_lsp::lsp_types::Location =
        serde_json::from_value(result).expect("a Location");
    assert_eq!(
        location.uri, canvas,
        "a name declared in the entry resolves to the entry, not to a module"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// 跨文件引用的公共夹具：打开入口，光标落在 `And.intro` 的使用点上。
async fn cross_file_fixture(tag: &str) -> (LspService<Backend>, Url, Url, std::path::PathBuf) {
    let dir = tmp_dir(tag);
    let root = Url::from_directory_path(&dir).expect("dir url");
    let (mut service, mut socket) = test_service();
    testutil::handshake_with_root(&mut service, &root).await;
    let logic = write(&dir, "Logic.sokonanoda", LOGIC);
    let canvas = write(&dir, "Canvas.sokonanoda", CANVAS);
    testutil::did_open_at(&mut service, &canvas, CANVAS).await;
    let _ = testutil::wait_diagnostics_for(&mut socket, &canvas, "canvas diagnostics").await;
    (service, logic, canvas, dir)
}

fn position_of_use(text: &str, needle: &str) -> serde_json::Value {
    testutil::position_json(testutil::lsp_pos(
        text,
        testutil::offset_of(text, needle) + 3,
    ))
}

#[tokio::test]
async fn references_cross_files_include_the_declaring_module() {
    let (mut service, logic, canvas, dir) = cross_file_fixture("refs").await;
    let req = RpcRequest::build("textDocument/references")
        .params(serde_json::json!({
            "textDocument": {"uri": canvas},
            "position": position_of_use(CANVAS, "And.intro b a"),
            "context": {"includeDeclaration": true},
        }))
        .id(4)
        .finish();
    let result = testutil::call(&mut service, req)
        .await
        .expect("references answers");
    let locations: Vec<tower_lsp::lsp_types::Location> =
        serde_json::from_value(result).expect("a Location list");
    let uris: Vec<&Url> = locations.iter().map(|location| &location.uri).collect();
    assert_eq!(
        uris,
        vec![&logic, &canvas],
        "定义在 Logic、使用在 Canvas，定义排最前"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn rename_rewrites_the_dependency_and_the_entry() {
    let (mut service, logic, canvas, dir) = cross_file_fixture("rename").await;
    let req = RpcRequest::build("textDocument/rename")
        .params(serde_json::json!({
            "textDocument": {"uri": canvas},
            "position": position_of_use(CANVAS, "And.intro b a"),
            "newName": "And.mk",
        }))
        .id(5)
        .finish();
    let result = testutil::call(&mut service, req)
        .await
        .expect("rename answers");
    let edit: tower_lsp::lsp_types::WorkspaceEdit =
        serde_json::from_value(result).expect("a WorkspaceEdit");
    let Some(tower_lsp::lsp_types::DocumentChanges::Edits(edits)) = edit.document_changes else {
        panic!("expected document changes: {edit:?}");
    };
    let touched: Vec<&Url> = edits.iter().map(|edit| &edit.text_document.uri).collect();
    assert_eq!(
        touched,
        vec![&logic, &canvas],
        "闭包里两份文件都要改：定义与使用"
    );
    let texts: Vec<String> = edits
        .iter()
        .flat_map(|edit| edit.edits.iter())
        .map(|change| match change {
            tower_lsp::lsp_types::OneOf::Left(text_edit) => text_edit.new_text.clone(),
            other => panic!("expected a plain TextEdit, got {other:?}"),
        })
        .collect();
    assert!(texts.iter().all(|text| text == "And.mk"), "{texts:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn rename_rejects_a_name_that_already_exists_in_the_project() {
    let (mut service, _logic, canvas, dir) = cross_file_fixture("rename-collision").await;
    let req = RpcRequest::build("textDocument/rename")
        .params(serde_json::json!({
            "textDocument": {"uri": canvas},
            "position": position_of_use(CANVAS, "And.intro b a"),
            "newName": "And.left",
        }))
        .id(6)
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
        .expect_err("renaming into an existing project name must be rejected");
    assert!(
        err.message.contains("同名声明"),
        "the message explains the collision: {err:?}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn editing_a_dependency_refreshes_the_open_entry() {
    let dir = tmp_dir("invalidate");
    let root = Url::from_directory_path(&dir).expect("dir url");
    let (mut service, mut socket) = test_service();
    testutil::handshake_with_root(&mut service, &root).await;

    let logic = write(&dir, "Logic.sokonanoda", LOGIC);
    let canvas = write(&dir, "Canvas.sokonanoda", CANVAS);
    // 依赖先打开、入口后打开（编辑器里的常见顺序）。这两步都可能一次发多份诊断，
    // 所以用"边处理边排空"的通知（见 testutil::notify_with_drain 的说明）。
    let opened = testutil::did_open_at_drained(&mut service, &mut socket, &logic, LOGIC).await;
    assert_eq!(opened.len(), 1, "只有刚打开的这份要发诊断");
    let opened = testutil::did_open_at_drained(&mut service, &mut socket, &canvas, CANVAS).await;
    assert_eq!(
        opened.len(),
        1,
        "依赖没变 ⇒ 不重复发它的诊断（publish-on-change）"
    );
    assert!(
        opened[0].diagnostics.is_empty(),
        "{:?}",
        opened[0].diagnostics
    );

    // 改**依赖**（未落盘）：把 `And.intro` 改名。入口里的使用点必须立刻报未知标识符——
    // 这就是跨文件失效：不重编译下游的话，入口会停在"全绿"的旧状态。
    let renamed = LOGIC.replace("And.intro", "And.mk");
    let msgs =
        testutil::did_change_at_drained(&mut service, &mut socket, &logic, 2, &renamed).await;
    let broken = msgs
        .iter()
        .find(|params| params.uri == canvas)
        .unwrap_or_else(|| panic!("the entry must be republished: {msgs:?}"));
    assert!(
        broken
            .diagnostics
            .iter()
            .any(|diag| testutil::code_of(diag) == "elab-unknown-identifier"),
        "the entry must see the renamed dependency: {:?}",
        broken.diagnostics
    );

    // 依赖改回来 ⇒ 入口重新变干净（证明是真的重编译，而不是"一旦报错就锁死"）。
    let msgs = testutil::did_change_at_drained(&mut service, &mut socket, &logic, 3, LOGIC).await;
    let healed = msgs
        .iter()
        .find(|params| params.uri == canvas)
        .expect("the entry is republished when the dependency heals");
    assert!(
        healed.diagnostics.is_empty(),
        "the entry recovers once the dependency does: {:?}",
        healed.diagnostics
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// 请求某文档的符号名列表（documentSymbol）。
async fn symbol_names(service: &mut LspService<Backend>, uri: &Url) -> Vec<String> {
    let req = RpcRequest::build("textDocument/documentSymbol")
        .params(serde_json::json!({"textDocument": {"uri": uri}}))
        .id(20)
        .finish();
    let result = testutil::call(service, req).await.expect("symbols answer");
    let symbols: Vec<serde_json::Value> = serde_json::from_value(result).expect("a symbol list");
    symbols
        .into_iter()
        .filter_map(|symbol| symbol["name"].as_str().map(str::to_string))
        .collect()
}

/// 请求某文档的 goal 视图声明名列表（`soko/goals`）。
async fn goal_names(service: &mut LspService<Backend>, uri: &Url) -> Vec<String> {
    let req = RpcRequest::build("soko/goals")
        .params(serde_json::json!({"textDocument": {"uri": uri}}))
        .id(21)
        .finish();
    let result = testutil::call(service, req).await.expect("goals answer");
    result["decls"]
        .as_array()
        .map(|decls| {
            decls
                .iter()
                .filter_map(|decl| decl["name"].as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// 多文档下每个请求都必须答**它自己那份**文档——尤其是刚被"顺带刷新"过的场景。
///
/// 回归：`refresh` 早先刷新下游时用 `focus(&other)` 改文本，活跃文档被留在下游
/// 文件上；此后不带 URI 逻辑的请求（symbols / goals / codeLens / hover）会答出
/// **另一份**文档的结果。修法是 `Docs::set_text_at`（不动活跃文档）+
/// `Docs::focus_request`（带 URI 的请求各自聚焦）。
#[tokio::test]
async fn each_request_answers_for_its_own_document() {
    let dir = tmp_dir("focus");
    let root = Url::from_directory_path(&dir).expect("dir url");
    let (mut service, mut socket) = test_service();
    testutil::handshake_with_root(&mut service, &root).await;

    let logic = write(&dir, "Logic.sokonanoda", LOGIC);
    let canvas = write(&dir, "Canvas.sokonanoda", CANVAS);
    let _ = testutil::did_open_at_drained(&mut service, &mut socket, &logic, LOGIC).await;
    let _ = testutil::did_open_at_drained(&mut service, &mut socket, &canvas, CANVAS).await;

    // 改依赖 ⇒ 入口（**不是**请求目标）在后台被重编译。这一步最容易把活跃文档
    // 挪到入口上；之后对 Logic 的请求必须仍然答 Logic。
    let renamed = LOGIC.replace("And.left", "And.left_renamed");
    let _ = testutil::did_change_at_drained(&mut service, &mut socket, &logic, 2, &renamed).await;

    let logic_names = symbol_names(&mut service, &logic).await;
    assert!(
        logic_names.iter().any(|name| name == "And.left_renamed"),
        "the request for Logic must answer Logic's own symbols: {logic_names:?}"
    );
    assert!(
        !logic_names.iter().any(|name| name == "and_swap"),
        "Logic must not answer with the entry's declarations: {logic_names:?}"
    );
    let canvas_names = symbol_names(&mut service, &canvas).await;
    assert!(
        canvas_names.iter().any(|name| name == "and_swap"),
        "the request for the entry must answer the entry: {canvas_names:?}"
    );

    // 自定义请求（goal 视图）同样按 URI 取文档。
    let logic_goals = goal_names(&mut service, &logic).await;
    assert!(
        logic_goals.iter().any(|name| name == "And"),
        "goal view for Logic: {logic_goals:?}"
    );
    assert!(
        !logic_goals.iter().any(|name| name == "and_swap"),
        "goal view for Logic must not list the entry's declarations: {logic_goals:?}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// 编辑器里的模块根必须与 CLI 同一套发现规则（最近 `sokonanoda.toml` → 入口目录）。
///
/// 回归：LSP 早先把 `initialize` 的**工作区根**当模块根传下去（等于跳过清单发现），
/// 于是"工作区里嵌套的项目"——VS Code 打开仓库根、再打开
/// `course/unit11-project/Canvas.sokonanoda`——会报 `import-not-found`，而同一个
/// 文件在 CLI 下编译正常（2026-09-18 真二进制实测）。
#[tokio::test]
async fn a_nested_project_resolves_against_its_own_manifest() {
    let ws = tmp_dir("nested-ws");
    // 工作区根**没有**清单；项目在子目录里，清单与模块都在那儿。
    let root = Url::from_directory_path(&ws).expect("dir url");
    let (mut service, mut socket) = test_service();
    testutil::handshake_with_root(&mut service, &root).await;

    let _logic = write(&ws, "proj/Lib.sokonanoda", "axiom Q : Prop\n");
    let _manifest = write(&ws, "proj/sokonanoda.toml", "name = \"proj\"\n");
    let entry_text = "import Lib\n\ntheorem v : Q -> Q := fun (h : Q) => h\n";
    let entry = write(&ws, "proj/Main.sokonanoda", entry_text);
    testutil::did_open_at(&mut service, &entry, entry_text).await;
    let diags = testutil::wait_diagnostics_for(&mut socket, &entry, "nested entry").await;
    assert!(
        diags.diagnostics.is_empty(),
        "the entry must resolve `Lib` through its own manifest (workspace root has none): {:?}",
        diags.diagnostics
    );
    let _ = std::fs::remove_dir_all(&ws);
}

/// 项目入口也有 quick-fix（回归：`front::suggest` 的判定与建议材料都要看得见
/// **被导入**的声明——判据前缀 + 闭包级 refine 模板表）。
///
/// 修复前：同一个文件放进单文件给得出 `refine And.intro a b sorry sorry`，
/// 放进项目入口是 `null`（2026-09-18 真 LSP 探针实测）。
#[tokio::test]
async fn code_actions_work_in_a_project_entry() {
    let dir = tmp_dir("project-actions");
    let root = Url::from_directory_path(&dir).expect("dir url");
    let (mut service, mut socket) = test_service();
    testutil::handshake_with_root(&mut service, &root).await;

    let _logic = write(&dir, "Logic.sokonanoda", LOGIC);
    let entry_text = "import Logic\n\n\
theorem and_intro_x (a b : Prop) (h : a) (k : b) : And a b :=\n  sorry\n";
    let entry = write(&dir, "Main.sokonanoda", entry_text);
    testutil::did_open_at(&mut service, &entry, entry_text).await;
    let _ = testutil::wait_diagnostics_for(&mut socket, &entry, "project entry").await;

    let offset = testutil::offset_of(entry_text, "sorry");
    let pos = testutil::lsp_pos(entry_text, offset + 1);
    let req = RpcRequest::build("textDocument/codeAction")
        .params(serde_json::json!({
            "textDocument": {"uri": entry},
            "range": {"start": testutil::position_json(pos), "end": testutil::position_json(pos)},
            "context": {"diagnostics": []},
        }))
        .id(30)
        .finish();
    let result = testutil::call(&mut service, req)
        .await
        .expect("code actions answer");
    let actions: Vec<serde_json::Value> = serde_json::from_value(result).expect("an action list");
    let titles: Vec<String> = actions
        .iter()
        .filter_map(|action| action["title"].as_str().map(str::to_string))
        .collect();
    assert!(
        titles
            .iter()
            .any(|title| title.contains("refine And.intro")),
        "the imported constructor must produce a refine quick-fix: {titles:?}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// 编辑器**外**的改动也要刷新打开的项目文档（`workspace/didChangeWatchedFiles`）。
///
/// 场景：`git checkout` / 脚本 / 另一个编辑器改了被 import 的模块——扩展早就声明了
/// `**/*.sokonanoda` 的 watcher，但服务端此前忽略这个通知，于是入口一直显示旧诊断，
/// 要重开文件才刷新（0.57.0 编辑器审计 RISK）。
#[tokio::test]
async fn an_external_change_to_a_dependency_refreshes_the_open_entry() {
    let dir = tmp_dir("watched");
    let root = Url::from_directory_path(&dir).expect("dir url");
    let (mut service, mut socket) = test_service();
    testutil::handshake_with_root(&mut service, &root).await;

    let logic = write(&dir, "Logic.sokonanoda", LOGIC);
    let canvas = write(&dir, "Canvas.sokonanoda", CANVAS);
    let opened = testutil::did_open_at_drained(&mut service, &mut socket, &canvas, CANVAS).await;
    assert!(opened.iter().all(|params| params.diagnostics.is_empty()));

    // 磁盘上改坏依赖（模拟编辑器外的操作；打开的缓冲区不参与这条路径）。
    let logic_path = logic.to_file_path().expect("file path");
    std::fs::write(&logic_path, LOGIC.replace("And.intro", "And.mk")).expect("rewrite dep");

    let published = testutil::notify_with_drain(
        &mut service,
        &mut socket,
        "workspace/didChangeWatchedFiles",
        serde_json::json!({"changes": [{"uri": logic, "type": 2}]}),
    )
    .await;
    let canvas_after = published
        .iter()
        .find(|params| params.uri == canvas)
        .unwrap_or_else(|| panic!("the entry must be re-published: {published:?}"));
    assert!(
        canvas_after
            .diagnostics
            .iter()
            .any(|diag| testutil::code_of(diag) == "elab-unknown-identifier"),
        "the entry must see the on-disk change: {:?}",
        canvas_after.diagnostics
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// `soko/goals` / `soko/stateAt` 回显请求的文档身份（uri + version）：多文档下
/// 客户端据此丢弃"答的是另一份文档"的过期响应（docs/protocol.md）。
#[tokio::test]
async fn custom_responses_echo_the_requested_document_identity() {
    let dir = tmp_dir("echo");
    let root = Url::from_directory_path(&dir).expect("dir url");
    let (mut service, mut socket) = test_service();
    testutil::handshake_with_root(&mut service, &root).await;

    let _logic = write(&dir, "Logic.sokonanoda", LOGIC);
    let canvas = write(&dir, "Canvas.sokonanoda", CANVAS);
    testutil::did_open_at(&mut service, &canvas, CANVAS).await;
    let _ = testutil::wait_diagnostics_for(&mut socket, &canvas, "canvas diagnostics").await;

    let goals = testutil::call(
        &mut service,
        RpcRequest::build("soko/goals")
            .params(serde_json::json!({"textDocument": {"uri": canvas}}))
            .id(40)
            .finish(),
    )
    .await
    .expect("goals answers");
    assert_eq!(goals["uri"], serde_json::json!(canvas.as_str()));
    assert_eq!(goals["version"], serde_json::json!(1));

    let state = testutil::call(
        &mut service,
        RpcRequest::build("soko/stateAt")
            .params(serde_json::json!({
                "textDocument": {"uri": canvas},
                "position": {"line": 0, "character": 0},
            }))
            .id(41)
            .finish(),
    )
    .await
    .expect("stateAt answers");
    assert_eq!(state["uri"], serde_json::json!(canvas.as_str()));
    let _ = std::fs::remove_dir_all(&dir);
}

/// `soko/project`（0.58.0 批次 4）：这个文档所在闭包的只读状态视图。
///
/// 契约：身份回显（uri/version）+ `project` 的模块表（拓扑序、入口标记、状态）
/// 与 `reason`。视图与 CLI `query project` 同源（`front::query::project_view`），
/// 字段名就是 `docs/protocol.md` 的那一份。
async fn project_answer(service: &mut LspService<Backend>, uri: &Url) -> serde_json::Value {
    let req = RpcRequest::build("soko/project")
        .params(serde_json::json!({"textDocument": {"uri": uri}}))
        .id(22)
        .finish();
    testutil::call(service, req).await.expect("project answer")
}

#[tokio::test]
async fn project_request_describes_the_closure_of_the_requested_document() {
    let dir = tmp_dir("view");
    let root = Url::from_directory_path(&dir).expect("dir url");
    let (mut service, mut socket) = test_service();
    testutil::handshake_with_root(&mut service, &root).await;

    let logic = write(&dir, "Logic.sokonanoda", LOGIC);
    let canvas = write(&dir, "Canvas.sokonanoda", CANVAS);
    let _ = testutil::did_open_at_drained(&mut service, &mut socket, &logic, LOGIC).await;
    let _ = testutil::did_open_at_drained(&mut service, &mut socket, &canvas, CANVAS).await;

    let answer = project_answer(&mut service, &canvas).await;
    assert_eq!(
        answer["uri"].as_str(),
        Some(canvas.as_str()),
        "the answer echoes the requested document: {answer}"
    );
    assert!(answer["version"].as_i64().unwrap_or(0) >= 1);
    assert_eq!(answer["reason"], serde_json::Value::Null);
    let project = &answer["project"];
    assert_eq!(project["entry"], "Canvas");
    let modules = project["modules"].as_array().expect("modules");
    assert_eq!(
        modules
            .iter()
            .map(|module| module["name"].as_str().unwrap_or_default())
            .collect::<Vec<_>>(),
        vec!["Logic", "Canvas"],
        "topological order, entry last: {project}"
    );
    assert_eq!(modules[1]["entry"], true);
    assert_eq!(modules[0]["status"], "compiled");
    assert_eq!(project["counts"]["modules"], 2);
    assert_eq!(project["counts"]["errors"], 0, "{project}");

    // 依赖文档**自己**不是入口：它是一个单文件（无 `import`）⇒ 答 reason。
    let logic_answer = project_answer(&mut service, &logic).await;
    assert_eq!(logic_answer["uri"].as_str(), Some(logic.as_str()));
    assert_eq!(logic_answer["project"], serde_json::Value::Null);
    assert_eq!(logic_answer["reason"], "no-imports");
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn project_request_follows_the_unsaved_buffer_and_reports_failures() {
    let dir = tmp_dir("view-edit");
    let root = Url::from_directory_path(&dir).expect("dir url");
    let (mut service, mut socket) = test_service();
    testutil::handshake_with_root(&mut service, &root).await;

    let _logic = write(&dir, "Logic.sokonanoda", LOGIC);
    let canvas = write(&dir, "Canvas.sokonanoda", CANVAS);
    let _ = testutil::did_open_at_drained(&mut service, &mut socket, &canvas, CANVAS).await;

    // 未落盘的编辑：把 import 改成不存在的模块（编辑器里的当前状态就是真相）。
    let broken = CANVAS.replace("import Logic", "import Missing");
    let published =
        testutil::did_change_at_drained(&mut service, &mut socket, &canvas, 2, &broken).await;
    assert!(
        published
            .iter()
            .flat_map(|params| &params.diagnostics)
            .any(|diag| testutil::code_of(diag) == "import-not-found"),
        "the unsaved edit is what the compiler sees: {published:?}"
    );

    let answer = project_answer(&mut service, &canvas).await;
    let project = &answer["project"];
    let entry = project["modules"]
        .as_array()
        .expect("modules")
        .iter()
        .find(|module| module["entry"] == true)
        .cloned()
        .expect("entry module");
    assert_eq!(
        entry["status"], "load-failed",
        "the entry's own import is missing: {project}"
    );
    assert!(
        entry["message"]
            .as_str()
            .unwrap_or_default()
            .contains("Missing"),
        "the view names the missing module: {entry}"
    );
    assert!(
        project["diagnostics"]
            .as_array()
            .expect("diagnostics")
            .iter()
            .any(|diag| diag["code"] == "import-not-found"),
        "project-level diagnostics are part of the view: {project}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

// ── G-20 / X15：记法随 import 传播时的单文件 parse 失败**不是**诊断 ──────────

/// 库模块：声明一个数学符号（`∈`），与 `courses/set-theory/lib/Set.sokonanoda`
/// 同形状（**显式**前导类型参数 ⇒ 记法路径要自己补它）。
const NOTATION_LIB: &str = "\
def Set (α : Type) : Type := α -> Prop\n\
def Set.mem (α : Type) (a : α) (A : Set α) : Prop := A a\n\
infix:50 \" ∈ \" => Set.mem\n";

/// 入口：用了**库声明**的记法 ⇒ 单文件 parse **必然**失败（`∈` 不在本文件里），
/// 但闭包编译是好的。这就是课程单元的常态。
const NOTATION_CANVAS: &str = "\
import SetLib\n\n\
theorem mem_self (α : Type) (a : α) (A : Set α) (h : a ∈ A) : a ∈ A := h\n";

/// 入口（带洞）：课程练习的常态——用库记法 + `:= by` + `sorry`。
const NOTATION_EXERCISE: &str = "\
import SetLib\n\n\
theorem open_one (α : Type) (a : α) (A : Set α) : a ∈ A := by\n\
  sorry\n";

/// **X15 回归**：闭包编译成功时，单文件 parse 失败不得吃掉项目报告。
///
/// 改前实测（真 LSP over stdio，0.61.0）：编辑器发一条**假**的
/// `notation-unknown-symbol`，且 `documentSymbol` / `hover` 全部回答 `null`
/// ——而同一份文本走 CLI 判卷 `exit 0`。
#[tokio::test]
async fn imported_notation_keeps_the_report_and_the_diagnostics_honest() {
    let dir = tmp_dir("notation-scope");
    let root = Url::from_directory_path(&dir).expect("dir url");
    let (mut service, mut socket) = test_service();
    testutil::handshake_with_root(&mut service, &root).await;

    let _lib = write(&dir, "SetLib.sokonanoda", NOTATION_LIB);
    let canvas = write(&dir, "Canvas.sokonanoda", NOTATION_CANVAS);
    testutil::did_open_at(&mut service, &canvas, NOTATION_CANVAS).await;
    let diags = testutil::wait_diagnostics_for(&mut socket, &canvas, "canvas diagnostics").await;

    // ① 不得有假诊断：闭包把 `∈` 带进来了，这份文本是干净的。
    assert!(
        diags.diagnostics.is_empty(),
        "imported notation must not produce a fake diagnostic: {:?}",
        diags.diagnostics
    );

    // ② hover 必须活着（用户要的「hover 提示怎么输入符号」就落在这条通道上）。
    let pos = lsp_pos(NOTATION_CANVAS, testutil::offset_of(NOTATION_CANVAS, "∈"));
    let result = call(
        &mut service,
        RpcRequest::build("textDocument/hover")
            .params(json!({
                "textDocument": {"uri": canvas},
                "position": position_json(pos),
            }))
            .id(2)
            .finish(),
    )
    .await
    .expect("hover must answer");
    let hover: Option<Hover> = serde_json::from_value(result).expect("valid Hover");
    let hover = hover.expect("hover must resolve on a symbol that came in through `import`");
    // ③ 而且它必须**教怎么输入**（D5 的用户要求）。`∈` 由库声明 ⇒ 展开目标在
    //    别的文件里（这里给不出），但「怎么打」来自 `front::notation_input` 的表，
    //    与作用域无关。
    let HoverContents::Markup(markup) = hover.contents else {
        panic!("expected markup hover");
    };
    assert!(
        markup.value.contains("\\in"),
        "hover on an imported symbol must teach the abbreviation: {:?}",
        markup.value
    );

    // ③ documentSymbol 必须非空（改前是 `null`：报告被丢掉了）。
    let result = call(
        &mut service,
        RpcRequest::build("textDocument/documentSymbol")
            .params(json!({"textDocument": {"uri": canvas}}))
            .id(3)
            .finish(),
    )
    .await
    .expect("documentSymbol must answer");
    let symbols: Option<DocumentSymbolResponse> =
        serde_json::from_value(result).expect("valid DocumentSymbolResponse");
    let count = match symbols {
        Some(DocumentSymbolResponse::Nested(items)) => items.len(),
        Some(DocumentSymbolResponse::Flat(items)) => items.len(),
        None => 0,
    };
    assert!(
        count > 0,
        "the rescued report must still drive documentSymbol"
    );

    // ④ **声明栏的数据源必须非空**（G-22）。夹具恰好就是 G-22 的形状
    //    （入口用库记法 ⇒ 单独 parse 必然失败、闭包好），而 G-20 当年只钉了
    //    ①诊断 ②hover ③documentSymbol ——**漏了 `soko/goals`**，
    //    于是「目标栏好、声明栏空」这个不对称活了很久（计划 T-B04）。
    let result = call(
        &mut service,
        RpcRequest::build("soko/goals")
            .params(json!({"textDocument": {"uri": canvas}}))
            .id(4)
            .finish(),
    )
    .await
    .expect("soko/goals must answer");
    // `GoalsResponse` 只有 `Serialize`（它是服务端的响应类型）⇒ 这里读 Value。
    let decls = result
        .get("decls")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    assert!(
        !decls.is_empty(),
        "项目入口的声明栏必须非空（G-22）：`parsable()` 以前把它判死了；实际 = {result:?}"
    );
    assert_eq!(
        decls.len(),
        count,
        "声明栏与 documentSymbol 必须同源同数（都来自 report.decls）"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// **老契约不许被放宽**（设计 R-5）：入口**自己**有语法错误时闭包也失败，
/// 那时仍要发那条 parse 错误（而不是发一个空报告让文件看起来是好的）。
#[tokio::test]
async fn a_genuinely_broken_entry_still_reports_the_parse_error() {
    let dir = tmp_dir("notation-broken");
    let root = Url::from_directory_path(&dir).expect("dir url");
    let (mut service, mut socket) = test_service();
    testutil::handshake_with_root(&mut service, &root).await;

    let _lib = write(&dir, "SetLib.sokonanoda", NOTATION_LIB);
    let broken = "import SetLib\n\ntheorem t (α : Type) (a : α) (A : Set α) : a ∈ A :=\n";
    let canvas = write(&dir, "Broken.sokonanoda", broken);
    testutil::did_open_at(&mut service, &canvas, broken).await;
    let diags = testutil::wait_diagnostics_for(&mut socket, &canvas, "broken diagnostics").await;
    assert!(
        !diags.diagnostics.is_empty(),
        "a real syntax error must still be reported"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// **G-22 的同族**：`alt+n`（`soko/nextHole`）在**项目入口**上必须能定位到洞。
///
/// 它经由 `next_hole → holes → goals(true)` ⇒ 撞的是**同一条** `usable()` 判据
/// （计划 T-B06）。改前：项目文件里 `alt+n` 完全没反应（`nextHole` 答 `null`），
/// 而 `stateAt`（目标栏）正常——用户看到的正是这个不对称。
#[tokio::test]
async fn next_hole_reaches_a_project_entry() {
    let dir = tmp_dir("notation-next-hole");
    let root = Url::from_directory_path(&dir).expect("dir url");
    let (mut service, mut socket) = test_service();
    testutil::handshake_with_root(&mut service, &root).await;

    let _lib = write(&dir, "SetLib.sokonanoda", NOTATION_LIB);
    let entry = write(&dir, "Exercise.sokonanoda", NOTATION_EXERCISE);
    testutil::did_open_at_drained(&mut service, &mut socket, &entry, NOTATION_EXERCISE).await;

    let result = call(
        &mut service,
        RpcRequest::build("soko/nextHole")
            .params(json!({
                "textDocument": {"uri": entry},
                "position": {"line": 0, "character": 0},
                "forward": true,
            }))
            .id(1)
            .finish(),
    )
    .await
    .expect("soko/nextHole must answer");
    assert!(
        !result.is_null(),
        "项目入口的 alt+n 必须能定位到洞（G-22 同族），实际 = {result:?}"
    );
    // 洞落在 `sorry` 那一行（第 4 行，0 基 3）。
    // 注意响应的形状：**扁平的** `{start, end}`（不是 `{range: {start, end}}`
    // ——`soko/goals` 里的洞才是后者，两者别混）。
    let line = result
        .get("start")
        .and_then(|start| start.get("line"))
        .and_then(|line| line.as_u64());
    assert_eq!(line, Some(3), "洞必须在 `sorry` 那一行，实际 = {result:?}");

    let _ = std::fs::remove_dir_all(&dir);
}

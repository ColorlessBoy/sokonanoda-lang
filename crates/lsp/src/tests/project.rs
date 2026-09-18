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

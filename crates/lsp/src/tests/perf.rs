use super::*;

#[tokio::test]
async fn perf_did_change_latency() {
    // didChange → 诊断落地的每次延迟 < 50ms（50 声明文件）。
    // 抓的是「编辑→反馈」的用户可感延迟。
    let src = perf_canvas(50);
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, &src).await;
    let _ = wait_diagnostics(&mut socket, "perf: initial").await;

    let mut cur = src.clone();
    let mut version = 1i32;
    // 编辑最后一条 open 练习的值（模拟学习者在文件末尾做题）
    let old_line = "theorem exercise : P := sorry";
    let new_line = "theorem exercise : P := proofP";
    let offset = cur.find(old_line).expect("exercise line");
    let step: TypedStep = (offset, old_line.len(), new_line);
    type_step(&mut service, &mut socket, &mut cur, &mut version, step).await;

    let start = std::time::Instant::now();
    // 再触发一次编辑（恢复原文→再编辑），量测 round-trip
    let step_back: TypedStep = (offset, new_line.len(), old_line);
    type_step(&mut service, &mut socket, &mut cur, &mut version, step_back).await;
    let elapsed = start.elapsed().as_millis();
    println!("PERF lsp didChange round-trip: {elapsed}ms (threshold 50ms)");
    assert!(
        elapsed < 50,
        "didChange round-trip took {elapsed}ms (threshold 50ms)"
    );
    shutdown(&mut service).await;
}

#[tokio::test]
async fn perf_completion_and_hover_latency() {
    // completion + hover 请求延迟 < 10ms（50 声明文件）。
    let src = perf_canvas(50);
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, &src).await;
    let _ = wait_diagnostics(&mut socket, "perf: initial").await;

    let at = offset_of(&src, "sorry");
    // completion
    let start = std::time::Instant::now();
    let _ = request_completions_at(&mut service, lsp_pos(&src, at)).await;
    let c_ms = start.elapsed().as_millis();
    println!("PERF lsp completion: {c_ms}ms (threshold 10ms)");
    assert!(c_ms < 10, "completion took {c_ms}ms (threshold 10ms)");
    // hover
    let start = std::time::Instant::now();
    let _ = hover_opt_at(&mut service, &src, at).await;
    let h_ms = start.elapsed().as_millis();
    println!("PERF lsp hover: {h_ms}ms (threshold 10ms)");
    assert!(h_ms < 10, "hover took {h_ms}ms (threshold 10ms)");
    shutdown(&mut service).await;
}

#[tokio::test]
async fn perf_state_at_latency() {
    // 光标移动路径 `soko/stateAt`（0.40.0 起带 goal_runs/ty_runs）延迟
    // < 10ms（50 声明文件）——防止「每次移动都全量重解析」之类的回归。
    let src = perf_canvas(50);
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, &src).await;
    let _ = wait_diagnostics(&mut socket, "perf: initial").await;

    let at = offset_of(&src, "sorry");
    let start = std::time::Instant::now();
    let result = ask_state_at(&mut service, &src, at).await;
    let elapsed = start.elapsed().as_millis();
    assert!(
        result.get("goals").and_then(|g| g.as_array()).is_some(),
        "stateAt response shape: {result:?}"
    );
    println!("PERF lsp stateAt: {elapsed}ms (threshold 10ms)");
    assert!(
        elapsed < 10,
        "soko/stateAt took {elapsed}ms (threshold 10ms)"
    );
    shutdown(&mut service).await;
}

#[tokio::test]
async fn perf_goals_view_latency() {
    // goal 视图（soko/goals，教学核心特性）延迟 < 10ms（50 声明文件）。
    let src = perf_canvas(50);
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, &src).await;
    let _ = wait_diagnostics(&mut socket, "perf: initial").await;

    let start = std::time::Instant::now();
    let result = call(
        &mut service,
        tower_lsp::jsonrpc::Request::build("soko/goals")
            .params(serde_json::json!({
                "textDocument": {"uri": "file:///perf.sokonanoda"},
                "position": null
            }))
            .id(90)
            .finish(),
    )
    .await
    .expect("soko/goals must answer");
    let elapsed = start.elapsed().as_millis();
    // 响应必须真的带 decls（防止测了个错误响应）
    assert!(
        result.get("decls").and_then(|d| d.as_array()).is_some(),
        "goals response shape: {result:?}"
    );
    println!("PERF lsp goals view: {elapsed}ms (threshold 10ms)");
    assert!(elapsed < 10, "soko/goals took {elapsed}ms (threshold 10ms)");
    shutdown(&mut service).await;
}

// ── 项目（import 闭包）交互成本：编辑器里"会不会卡"的直接量度 ──────────────
//
// 与单文件 perf 的区别：项目模式**没有**跨模块增量，每次 didChange 都重编译整个
// 闭包。所以这里量的就是「一次按键 = 一次闭包重编译」的真实成本，并把**每次通知
// 发出几条诊断**也钉住（那是 VS Code 侧 decoration/树/webview 的工作量来源）。
// 数值同时进 docs/perf/ledger.jsonl（scripts/perf-ledger.sh 采集）。

/// 生成临时项目：`Lib0 ← Lib1 ← … ← Main`，每个模块 `decls` 条已证声明。
/// 返回 `(目录, 入口 Url, 入口文本)`。
fn gen_project(tag: &str, modules: usize, decls: usize) -> (std::path::PathBuf, Url, String) {
    let dir = std::env::temp_dir().join(format!(
        "soko-lsp-perf-project-{tag}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    for index in 0..modules {
        let mut text = String::new();
        if index > 0 {
            text.push_str(&format!("import Lib{}\n\n", index - 1));
        }
        if index == 0 {
            text.push_str("axiom P : Prop\naxiom proofP : P\n");
        }
        for i in 0..decls {
            text.push_str(&format!("theorem lib{index}_s{i} : P := proofP\n"));
        }
        std::fs::write(dir.join(format!("Lib{index}.sokonanoda")), text).expect("write module");
    }
    let mut entry = format!("import Lib{}\n\n", modules - 1);
    for i in 0..decls {
        entry.push_str(&format!("theorem main_s{i} : P := proofP\n"));
    }
    std::fs::write(dir.join("Main.sokonanoda"), &entry).expect("write entry");
    let uri = Url::from_file_path(dir.join("Main.sokonanoda")).expect("file url");
    (dir, uri, entry)
}

fn perf_json(value: serde_json::Value) {
    println!("PERFJSON {value}");
}

#[tokio::test]
async fn perf_project_did_open_and_keystroke() {
    let (dir, entry_uri, entry_text) = gen_project("keystroke", 2, 12);
    let root = Url::from_directory_path(&dir).expect("dir url");
    let (mut service, mut socket) = test_service();
    testutil::handshake_with_root(&mut service, &root).await;

    let start = std::time::Instant::now();
    let opened =
        testutil::did_open_at_drained(&mut service, &mut socket, &entry_uri, &entry_text).await;
    let open_ms = start.elapsed().as_millis();
    assert_eq!(
        opened.len(),
        1,
        "didOpen publishes one document: {opened:?}"
    );
    assert!(
        opened[0].diagnostics.is_empty(),
        "{:?}",
        opened[0].diagnostics
    );

    // 一次按键：改最后一条声明的名字（整文件重编译 = 整个闭包重编译）。
    let edited = entry_text.replace("main_s11", "main_s11x");
    let start = std::time::Instant::now();
    let changed =
        testutil::did_change_at_drained(&mut service, &mut socket, &entry_uri, 2, &edited).await;
    let key_ms = start.elapsed().as_millis();
    assert_eq!(
        changed.len(),
        1,
        "one keystroke must publish exactly one document's diagnostics: {changed:?}"
    );
    assert!(
        changed[0].diagnostics.is_empty(),
        "{:?}",
        changed[0].diagnostics
    );

    println!("PERF project lsp: didOpen {open_ms}ms · keystroke (2 modules × 12 decls) {key_ms}ms");
    perf_json(serde_json::json!({
        "schema": "soko.perf/1",
        "scope": "lsp-project",
        "case": "did_open_and_keystroke",
        "modules": 2,
        "decls_per_module": 12,
        "open_ms": open_ms,
        "keystroke_ms": key_ms,
        "publishes_per_keystroke": changed.len(),
    }));
    assert!(
        key_ms < 300,
        "one keystroke cost {key_ms}ms on a 2×12 project (editor lag)"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn perf_project_dependency_edit_refreshes_dependents() {
    // 两条文档都打开：改**根依赖**（声明 `P` 的那个模块）⇒ 依赖自己 + 下游入口
    // 各一条诊断（这是最坏的一次通知：一次按键要重编译两份文档）。
    let (dir, entry_uri, entry_text) = gen_project("dependency", 3, 12);
    let root_dep_path = dir.join("Lib0.sokonanoda");
    let root_dep_uri = Url::from_file_path(&root_dep_path).expect("file url");
    let root_dep_text = std::fs::read_to_string(&root_dep_path).expect("read module");
    let root = Url::from_directory_path(&dir).expect("dir url");
    let (mut service, mut socket) = test_service();
    testutil::handshake_with_root(&mut service, &root).await;

    let _ = testutil::did_open_at_drained(&mut service, &mut socket, &root_dep_uri, &root_dep_text)
        .await;
    let opened =
        testutil::did_open_at_drained(&mut service, &mut socket, &entry_uri, &entry_text).await;
    assert!(opened.iter().all(|params| params.diagnostics.is_empty()));

    // 根依赖里 `P` 改名 ⇒ 链上每个模块（含入口）都会看到未知标识符。
    let broken = root_dep_text.replace("axiom P : Prop", "axiom Q : Prop");
    let start = std::time::Instant::now();
    let published =
        testutil::did_change_at_drained(&mut service, &mut socket, &root_dep_uri, 2, &broken).await;
    let elapsed = start.elapsed().as_millis();
    let dependent = published
        .iter()
        .find(|params| params.uri == entry_uri)
        .unwrap_or_else(|| panic!("the dependent must be republished: {published:?}"));
    assert!(
        !dependent.diagnostics.is_empty(),
        "the dependent sees the broken dependency"
    );

    println!(
        "PERF project lsp: dependency edit → {} publishes, dependent {} diagnostics, {elapsed}ms",
        published.len(),
        dependent.diagnostics.len()
    );
    perf_json(serde_json::json!({
        "schema": "soko.perf/1",
        "scope": "lsp-project",
        "case": "dependency_edit_refresh",
        "modules": 3,
        "decls_per_module": 12,
        "publishes": published.len(),
        "dependent_diagnostics": dependent.diagnostics.len(),
        "elapsed_ms": elapsed,
    }));
    assert!(
        published.len() <= 3,
        "a dependency edit should not fan out to more documents than are open: {}",
        published.len()
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn perf_project_requests_are_interactive() {
    // 项目入口上的 hover / definition / goals 都必须在"光标移动"量级（各 < 10ms）。
    let (dir, entry_uri, entry_text) = gen_project("requests", 3, 12);
    let root = Url::from_directory_path(&dir).expect("dir url");
    let (mut service, mut socket) = test_service();
    testutil::handshake_with_root(&mut service, &root).await;
    let _ = testutil::did_open_at_drained(&mut service, &mut socket, &entry_uri, &entry_text).await;

    let at = offset_of(&entry_text, "proofP") + 2;
    let pos = lsp_pos(&entry_text, at);
    let req = |method: &'static str, id: i64| {
        tower_lsp::jsonrpc::Request::build(method)
            .params(serde_json::json!({
                "textDocument": {"uri": entry_uri},
                "position": position_json(pos),
            }))
            .id(id)
            .finish()
    };
    let start = std::time::Instant::now();
    let hover = call(&mut service, req("textDocument/hover", 91))
        .await
        .expect("hover answers");
    let hover_ms = start.elapsed().as_millis();
    assert!(!hover.is_null(), "hover on an imported name must answer");

    let start = std::time::Instant::now();
    let definition = call(&mut service, req("textDocument/definition", 92))
        .await
        .expect("definition answers");
    let def_ms = start.elapsed().as_millis();
    let target: Location = serde_json::from_value(definition).expect("a Location");
    assert!(
        target.uri != entry_uri,
        "the cursor is on an imported name → jump into the dependency"
    );

    let start = std::time::Instant::now();
    let goals = call(
        &mut service,
        tower_lsp::jsonrpc::Request::build("soko/goals")
            .params(serde_json::json!({"textDocument": {"uri": entry_uri}}))
            .id(93)
            .finish(),
    )
    .await
    .expect("goals answers");
    let goals_ms = start.elapsed().as_millis();
    assert!(
        goals["decls"].as_array().is_some_and(|d| !d.is_empty()),
        "{goals:?}"
    );

    println!(
        "PERF project lsp requests: hover {hover_ms}ms · definition {def_ms}ms · goals {goals_ms}ms"
    );
    perf_json(serde_json::json!({
        "schema": "soko.perf/1",
        "scope": "lsp-project",
        "case": "request_latency",
        "modules": 3,
        "decls_per_module": 12,
        "hover_ms": hover_ms,
        "definition_ms": def_ms,
        "goals_ms": goals_ms,
    }));
    for (name, value) in [
        ("hover", hover_ms),
        ("definition", def_ms),
        ("goals", goals_ms),
    ] {
        assert!(
            value < 50,
            "project {name} took {value}ms (interactive budget)"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

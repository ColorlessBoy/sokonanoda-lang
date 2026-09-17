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

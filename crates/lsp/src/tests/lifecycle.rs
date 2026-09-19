use super::*;

#[tokio::test]
async fn initialize_advertises_core_capabilities() {
    let (mut service, _socket) = test_service();
    handshake(&mut service).await;
    shutdown(&mut service).await;
}

#[tokio::test]
async fn did_open_valid_file_publishes_no_diagnostics() {
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, VALID).await;
    let params = wait_diagnostics(&mut socket, "diagnostics after didOpen").await;
    assert_eq!(params.uri.as_str(), URI);
    assert!(
        params.diagnostics.is_empty(),
        "valid file must publish no diagnostics, got {:?}",
        params.diagnostics
    );
    shutdown(&mut service).await;
}

#[tokio::test]
async fn did_open_kernel_rejected_file_publishes_coded_diagnostic() {
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, KERNEL_BAD).await;
    let params = wait_diagnostics(&mut socket, "kernel diagnostics").await;
    assert_eq!(
        params.diagnostics.len(),
        1,
        "expected exactly one kernel rejection, got {:?}",
        params.diagnostics
    );
    let diag = &params.diagnostics[0];
    assert_eq!(code_of(diag), "kernel-rejected");
    assert!(
        diag.message.contains("提示："),
        "teaching hint expected in diagnostic message: {:?}",
        diag.message
    );
    assert_eq!(diag.severity, Some(DiagnosticSeverity::ERROR));
    assert_ne!(
        diag.range.start, diag.range.end,
        "kernel rejection must have a non-empty range"
    );
    shutdown(&mut service).await;
}

#[tokio::test]
async fn did_open_parse_error_publishes_parse_code() {
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, PARSE_BAD).await;
    let params = wait_diagnostics(&mut socket, "parse diagnostics").await;
    assert_eq!(
        params.diagnostics.len(),
        1,
        "expected exactly one parse error, got {:?}",
        params.diagnostics
    );
    let diag = &params.diagnostics[0];
    let code = code_of(diag);
    assert!(
        code == "unexpected-token" || code == "unexpected-eof",
        "expected a parse-stage code, got {code}"
    );
    assert_eq!(diag.severity, Some(DiagnosticSeverity::ERROR));
    assert_eq!(diag.source.as_deref(), Some("sokonanoda"));
    shutdown(&mut service).await;
}

#[tokio::test]
async fn did_change_recomputes_diagnostics() {
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, VALID).await;
    let first = wait_diagnostics(&mut socket, "initial diagnostics").await;
    assert!(first.diagnostics.is_empty());

    notify(
        &mut service,
        "textDocument/didChange",
        json!({
            "textDocument": {"uri": URI, "version": 2},
            "contentChanges": [{"text": KERNEL_BAD}],
        }),
    )
    .await;
    let second = wait_diagnostics(&mut socket, "diagnostics after didChange").await;
    assert_eq!(
        second.diagnostics.len(),
        1,
        "edited file must be re-checked, got {:?}",
        second.diagnostics
    );
    assert_eq!(code_of(&second.diagnostics[0]), "kernel-rejected");
    shutdown(&mut service).await;
}

#[tokio::test]
async fn directive_bare_file_without_nat_checks_clean() {
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, BARE_OK).await;
    let params = wait_diagnostics(&mut socket, "bare ok diagnostics").await;
    assert!(
        params.diagnostics.is_empty(),
        "bare Prop-level file must be clean: {:?}",
        params.diagnostics
    );
}

#[tokio::test]
async fn directive_bare_file_loses_nat() {
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, BARE_NAT).await;
    let params = wait_diagnostics(&mut socket, "bare nat diagnostics").await;
    assert!(
        params
            .diagnostics
            .iter()
            .any(|d| d.code == Some(NumberOrString::String("elab-unknown-identifier".into()))),
        "bare file must not know Nat: {:?}",
        params.diagnostics
    );
}

#[tokio::test]
async fn full_mode_still_has_nat_without_directive() {
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, FULL_NAT).await;
    let params = wait_diagnostics(&mut socket, "full nat diagnostics").await;
    assert!(
        params.diagnostics.is_empty(),
        "Nat prelude must be present without the directive: {:?}",
        params.diagnostics
    );
}

// I9 goal 视图：hover 显示可用假设；assumption/exact code action。

#[tokio::test]
async fn version_request_reports_version_and_pid() {
    // `sokonanoda: restart server` 在重启前后各问一次 soko/version：
    // 旧 pid 消失 + 新 pid 出现 + 版本号变化，把「旧进程退出、新进程是
    // 新版本」变成可验证的事实。
    let (mut service, _socket) = open_and_wait(EXERCISE).await;
    let result = call(
        &mut service,
        RpcRequest::build("soko/version")
            .params(json!({}))
            .id(91)
            .finish(),
    )
    .await
    .expect("soko/version must answer");
    assert_eq!(result["version"], env!("CARGO_PKG_VERSION"));
    let pid = result["pid"].as_u64().expect("pid is a number");
    assert!(pid > 0, "a real process id: {result:?}");
    shutdown(&mut service).await;
}

#[tokio::test]
async fn sorry_produces_warning_not_error() {
    // Lean 4 对齐：含 sorry 的声明产出 warning（不是 error），
    // 让学习者知道"文件编译但有缺口"。
    // G-01 起 `theorem` 的签名必须是真命题：`True` 不是 prelude 名字
    // ⇒ 用一个显式公理当命题（原写法 `theorem t : True := sorry` 现在会被
    // 内核拒，那样测的就不是 sorry warning 了）。
    let src = "axiom True : Prop\ntheorem t : True := sorry\n";
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, src).await;
    let diags = wait_diagnostics(&mut socket, "sorry warning").await;
    let sorry = diags
        .diagnostics
        .iter()
        .find(|d| d.code == Some(NumberOrString::String("sorry".to_string())))
        .expect("sorry warning must exist");
    assert_eq!(sorry.severity, Some(DiagnosticSeverity::WARNING));
    assert!(
        sorry.message.contains("uses 'sorry'"),
        "{:?}",
        sorry.message
    );
}

#[tokio::test]
async fn non_sorry_errors_are_not_warnings() {
    // 非 sorry 的 kernel 拒绝仍然是 error（不被 sorry warning 稀释）。
    let src = "def bad : Prop -> Type := fun (x : Prop) => x\n";
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, src).await;
    let diags = wait_diagnostics(&mut socket, "non-sorry diagnostics").await;
    assert!(diags
        .diagnostics
        .iter()
        .any(|d| d.severity == Some(DiagnosticSeverity::ERROR)));
    assert!(!diags
        .diagnostics
        .iter()
        .any(|d| d.code == Some(NumberOrString::String("sorry".to_string()))));
}

#[tokio::test]
async fn reserved_declaration_name_is_a_warning_not_an_error() {
    // `axiom Prop : Sort 1` 能通过内核，但这个名字永不被引用（`Prop`
    // 内核已经定义过，代码里的 `Prop` 都指内核那个）——编辑器给出
    // WARNING 级提示。
    let src = "axiom Prop : Sort 1\n";
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, src).await;
    let diags = wait_diagnostics(&mut socket, "reserved name warning").await;
    let warning = diags
        .diagnostics
        .iter()
        .find(|d| {
            d.code
                == Some(NumberOrString::String(
                    "reserved-declaration-name".to_string(),
                ))
        })
        .expect("reserved-declaration-name warning must exist");
    assert_eq!(warning.severity, Some(DiagnosticSeverity::WARNING));
    assert!(
        warning.message.contains("内核已经定义过了"),
        "warning carries the teaching message: {:?}",
        warning.message
    );
    assert!(
        !diags
            .diagnostics
            .iter()
            .any(|d| d.severity == Some(DiagnosticSeverity::ERROR)),
        "the declaration itself must not produce an error: {:?}",
        diags.diagnostics
    );
}

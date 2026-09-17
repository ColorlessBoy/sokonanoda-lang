use crate::testutil::{did_open, handshake, shutdown, test_service, wait_diagnostics};
use sokonanoda_front::parse;
use tower_lsp::lsp_types::{DiagnosticSeverity, NumberOrString};

#[tokio::test]
async fn by_sorry_warning_does_not_swallow_following_comments() {
    // 回归：`:= by sorry` 的声明 span 曾取「最后一个 tactic 之后的下一个
    // token 起点」——注释被词法器跳过，span 一路跨到下一个 theorem/EOF，
    // 把中间的多行注释整个包进黄色波浪线。现在 span 止于最后一个 tactic。
    let src = "axiom And : Prop -> Prop -> Prop\n\
               theorem a : (a : Prop) -> And a a -> a := by sorry\n\
               -- 练习 14 注释\n\
               -- soko:hint 思路：funapply\n\
               theorem b : (a : Prop) -> (b : Prop) -> a -> Or a b := by sorry\n";
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, src).await;
    let params = wait_diagnostics(&mut socket, "w").await;
    let sorry_warnings: Vec<_> = params
        .diagnostics
        .iter()
        .filter(|d| matches!(&d.code, Some(NumberOrString::String(c)) if c == "sorry"))
        .collect();
    assert_eq!(sorry_warnings.len(), 2, "two by-sorry declarations warn");
    for d in sorry_warnings {
        assert_eq!(
            d.range.start.line, d.range.end.line,
            "warning must be a single line (declaration), not swallow comments: {d:?}"
        );
        // 注释在第 2/3 行；声明在第 1 行与第 4 行，警告绝不能越到注释行。
        assert!(
            d.range.start.line != 2 && d.range.start.line != 3,
            "warning leaked onto a comment line: {d:?}"
        );
    }
    shutdown(&mut service).await;
}

#[tokio::test]
async fn by_sorry_hole_points_at_sorry_token() {
    // 回归：by 引擎降级出的尾部 Hole 曾用 Span::default()（offset 0），
    // 洞位/跳洞全错位。现在止于 `sorry` tactic 的 span。
    let src = "axiom And : Prop -> Prop -> Prop\n\
               theorem a : (a : Prop) -> And a a -> a := by sorry\n";
    let file = parse(src).unwrap();
    let report = sokonanoda_front::compile::check_document(&file);
    let d = report
        .decls
        .iter()
        .find(|d| d.name.as_deref() == Some("a"))
        .unwrap();
    assert_eq!(d.status, crate::DeclStatus::Open);
    assert!(!d.holes.is_empty(), "open by-sorry must carry a hole");
    let hole = &d.holes[0];
    let expected = src.find("sorry").unwrap();
    assert_eq!(
        hole.start.offset, expected,
        "hole must be at the `sorry` token, not offset 0"
    );
}

/// 「多余的 `sorry`」（`docs/design/redundant-sorry.md` §6 验收 3）：答案已经
/// 写全、只多留一行 `sorry` 时，编辑器**不再**说 "not yet solved"——改给一条
/// `redundant-sorry`（含 hint），学生才知道该删的是那一行；而真缺口照旧。
#[tokio::test]
async fn redundant_sorry_replaces_the_not_yet_solved_warning() {
    let codes_of = |diags: &tower_lsp::lsp_types::PublishDiagnosticsParams| -> Vec<String> {
        diags
            .diagnostics
            .iter()
            .filter_map(|d| match &d.code {
                Some(NumberOrString::String(c)) => Some(c.clone()),
                _ => None,
            })
            .collect()
    };

    let src = "axiom A : Prop\n\
               axiom B : Prop\n\
               axiom f : A -> B\n\
               theorem t (h : A) : B := f h\n\
               \x20 sorry\n";
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, src).await;
    let diags = wait_diagnostics(&mut socket, "redundant sorry").await;
    let codes = codes_of(&diags);
    assert!(
        codes.iter().any(|c| c == "redundant-sorry"),
        "must report redundant-sorry: {codes:?}"
    );
    assert!(
        !codes.iter().any(|c| c == "sorry"),
        "不得再叠一条 not yet solved（否则学生以为是自己没做出来）: {codes:?}"
    );
    let warning = diags
        .diagnostics
        .iter()
        .find(|d| matches!(&d.code, Some(NumberOrString::String(c)) if c == "redundant-sorry"))
        .expect("redundant-sorry warning");
    assert_eq!(warning.severity, Some(DiagnosticSeverity::WARNING));
    assert!(
        warning.message.contains("提示："),
        "warning carries the teaching hint: {}",
        warning.message
    );

    // 真缺口（`f` 缺一个实参）照旧报 "not yet solved"。
    let genuine = "axiom A : Prop\n\
                   axiom B : Prop\n\
                   axiom f : A -> B\n\
                   theorem t : B := f\n\
                   \x20 sorry\n";
    let (mut service2, mut socket2) = test_service();
    handshake(&mut service2).await;
    did_open(&mut service2, genuine).await;
    let diags2 = wait_diagnostics(&mut socket2, "genuine hole").await;
    let codes2 = codes_of(&diags2);
    assert!(
        codes2.iter().any(|c| c == "sorry"),
        "a genuine hole keeps the not-yet-solved warning: {codes2:?}"
    );
    assert!(
        !codes2.iter().any(|c| c == "redundant-sorry"),
        "a genuine hole is not redundant: {codes2:?}"
    );

    shutdown(&mut service).await;
    shutdown(&mut service2).await;
}

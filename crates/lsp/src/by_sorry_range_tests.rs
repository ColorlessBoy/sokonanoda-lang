use crate::testutil::{did_open, handshake, shutdown, test_service, wait_diagnostics};
use sokonanoda_front::parse;
use tower_lsp::lsp_types::NumberOrString;

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

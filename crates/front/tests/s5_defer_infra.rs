//! **S5 第一步（2026-10-23）**：延迟义务的**基础设施与读数**（行为未接线，默认关）。
//!
//! 判据：可延迟码表只认三条 `*-unsolved`；读数能记、能读、能清；不碰环境变量
//! （`SOKO_DEFER` 的开关由 by 引擎在 `run_by` 入口读，测试里不设 ⇒ 不影响其它测试）。

use sokonanoda_front::by_defer::{deferred_report, deferred_reset, DeferredTactic};
use sokonanoda_front::Span;

#[test]
fn deferrable_codes_and_report_roundtrip() {
    deferred_reset();
    assert!(deferred_report().is_empty());

    sokonanoda_front::by_defer::record_deferred_for_test(DeferredTactic {
        node: 3,
        span: Span::default(),
        code: "elab-implicit-argument-unsolved".to_string(),
        message: "至少有一个没解出来".to_string(),
    });
    let got = deferred_report();
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].node, 3);
    assert_eq!(got[0].code, "elab-implicit-argument-unsolved");

    deferred_reset();
    assert!(deferred_report().is_empty());

    // 可延迟码表：只认三条 `*-unsolved`（Clash 通道不在内 —— §18 的第二条发现）
    assert!(sokonanoda_front::by_defer::is_deferrable_for_test(
        "elab-implicit-argument-unsolved"
    ));
    assert!(sokonanoda_front::by_defer::is_deferrable_for_test(
        "elab-notation-argument-unsolved"
    ));
    assert!(sokonanoda_front::by_defer::is_deferrable_for_test(
        "elab-binder-notation-unsolved"
    ));
    assert!(!sokonanoda_front::by_defer::is_deferrable_for_test(
        "elab-tactic-failed"
    ));
    assert!(!sokonanoda_front::by_defer::is_deferrable_for_test(
        "kernel-rejected"
    ));
}

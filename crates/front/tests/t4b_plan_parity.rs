//! **T4-B 的前置判据**（T1-B 批 2 之后，2026-10-09）：CLI 想吃的"带产物的 session 路"
//! 必须与今天那条"整条闭包一趟"的路交出**同一份 `ProjectReport`** ✓。
//!
//! ## 为什么先要这一条（而不是直接把 CLI 换过去）
//!
//! 实测（`/tmp/t1b-big`：60 条 `by` 的 `Lib` + 一个入口，改一行入口后再编）：
//! `build <file>`（今天 = [`compile_plan_prechecked`]）**0.40s** vs `query check`
//! （走 session + 产物）**0.043s** ⇒ 差 **≈9×**，那 9× 全是**库层 elaborate** ✓。
//! 但 CLI 换过去之前必须先证明"**换路不改报告**" —— 否则 `--json` 就分叉了 ✗
//! （本仓的红线：同一批输入**事件计数不变、`--json` 逐字节不变** ✓）。
//!
//! ⚠ 本文件**只证报告**；CLI 那侧还有 **`build.*` 事件流 + 进度 sink** 两件（见
//! `compile_plan_with_artifacts` 的文档 ✓）⇒ 判据过了也**不等于**可以无脑换 ✓。

use sokonanoda_front::compile::CompileOptions;
use sokonanoda_front::project::{
    compile_plan_prechecked, compile_plan_with_artifacts, plan_project,
};

/// 一份**独立**的临时模块根（pid + 纳秒 ⇒ 并发跑也不撞 ✓）。
fn temp_root(tag: &str) -> std::path::PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let root = std::env::temp_dir().join(format!("soko-t4b-{tag}-{}-{nanos}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    root
}

#[test]
fn the_session_path_and_the_single_pass_path_agree_on_the_report() {
    let root = temp_root("parity");
    std::fs::create_dir_all(&root).expect("mkdir");
    // 库层：几条带 `by` 的证明（库层 elaborate 的成本来源 ✓）。
    std::fs::write(
        root.join("Lib.sokonanoda"),
        "theorem l1 (P : Prop) (h : P) : P := by\n  exact h\n\
         theorem l2 (P Q : Prop) (hp : P) (hq : Q) : P ∧ Q := by\n  apply And.intro\n  exact hp\n  exact hq\n",
    )
    .expect("write lib");
    // 入口：import 它 + 自己的声明。
    std::fs::write(
        root.join("E.sokonanoda"),
        "import Lib\ntheorem e1 (P : Prop) : P → P := fun h => h\ntheorem e2 (P Q : Prop) (hp : P) (hq : Q) : P ∧ Q := l2 P Q hp hq\n",
    )
    .expect("write entry");
    let entry = root.join("E.sokonanoda");
    let options = CompileOptions::default();

    // ① 今天那条：整条闭包**一趟**。
    let plan_a = plan_project(&entry, None, Some(root.as_path()));
    assert!(
        plan_a.diagnostics.is_empty(),
        "夹具自检：计划期不该有诊断：{:?}",
        plan_a.diagnostics
    );
    let single = compile_plan_prechecked(plan_a, &options, None);

    // ② 带产物的 session 路（`reuse_library = false` ⇒ 不碰线程局部检查点 ✓）。
    let plan_b = plan_project(&entry, None, Some(root.as_path()));
    let session = compile_plan_with_artifacts(plan_b, &options);

    // ③ **报告逐字节相同**（真相红线）—— 序列化比较，字段一个不漏 ✓。
    let a = serde_json::to_string(&single).expect("ser single");
    let b = serde_json::to_string(&session).expect("ser session");
    assert_eq!(
        b, a,
        "**换路不许改报告** ✗：session 路与整条闭包一趟必须交出同一份 `ProjectReport` \
         （这是 CLI 敢换过去的前提 ✓）"
    );

    // ④ 两条路都真的编过（防"两边都空"的假绿 ✓）。
    assert!(
        single.modules.len() >= 2,
        "夹具自检：闭包至少两个模块（实得 {}）",
        single.modules.len()
    );
    assert!(
        single.modules.iter().all(|m| m.report.errors.is_empty()),
        "夹具自检：两条路都不该有错误"
    );

    let _ = std::fs::remove_dir_all(&root);
}

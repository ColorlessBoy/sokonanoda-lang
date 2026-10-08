//! **T3-B1 · 计划 §4.2 第 5 条（2026-10-09）的判据**：记法目标签名的**就地**路。
//!
//! ## 这条接线是什么
//!
//! `elab_notation` 每展开一个记法符号都要问一次「目标常量的类型」
//! （`judge_type_of_constant` ⇒ `judge_type_of` ⇒ **合成 `#check` + 整份前缀
//! 从零重跑** ✗）。常量就在**活环境**里 ⇒ 就地 elaborate + 内核 pp 即可 ✓
//! （实测 unit08：冷开合成趟 **37**、其中这条占 **35**）。
//!
//! ## 判据（缺一不算）
//!
//! 1. **不空转**：`type_of_constant_report().used > 0` —— **专用**计数器，
//!    不与别的档混（混了就分不清哪条接线生效 ✗）；
//! 2. **影子档**：`SOKO_JUDGE_INPLACE=shadow` ⇒ `same > 0` 且 `diff == 0`
//!    —— 签名文本会被回读成记法目标 ⇒ **文本分叉 = elaborate 分叉** ✗；
//! 3. **逐字节**：`SOKO_JUDGE_INPLACE=off` vs 默认的 `--json` 相同（手工抽样；
//!    全量在发版节点）。
//!
//! ## 为什么用真课程单元
//!
//! 记法目标签名这条路要**真实记法 + 真闭包**才踩得稳（合成夹具很难稳定命中）；
//! 课程仓可分开检出 ⇒ 找不到就跳过（同 `lsp_keystroke_structure.rs` 的口径 ✓）。

use sokonanoda_front::compile::CompileOptions;
use sokonanoda_front::judge::{
    type_of_constant_first_diff, type_of_constant_report, type_of_constant_shadow,
};
use sokonanoda_front::project::{compile_plan, plan_project};

fn course_root() -> Option<std::path::PathBuf> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("courses")
        .join("set-theory");
    dir.join("sokonanoda.toml").is_file().then_some(dir)
}

#[test]
fn type_of_constant_is_answered_in_place_and_the_shadow_agrees() {
    std::env::set_var("SOKO_JUDGE_INPLACE", "shadow");
    let Some(root) = course_root() else {
        eprintln!("跳过：找不到 courses/set-theory/sokonanoda.toml（课程仓可分开检出）");
        return;
    };
    let path = root.join("units/I.3/unit08-images-preimages.sokonanoda");
    let plan = plan_project(&path, None, None);
    let report = compile_plan(plan, &CompileOptions::default());
    assert!(
        report.diagnostics.is_empty(),
        "夹具必须编过（否则量的是错误路径 ✗）：{:?}",
        report.diagnostics
    );
    let (same, diff) = type_of_constant_shadow();
    let (used, fallback) = type_of_constant_report();
    eprintln!("PERF type-of-constant: same={same} diff={diff} used={used} fallback={fallback}");
    eprintln!("PERF first-diff: {:?}", type_of_constant_first_diff());
    assert!(
        same > 0,
        "记法目标签名的就地路一次都没比对上（same={same}）⇒ **判据空转**：\
         要么夹具没踩到 `elab_notation`，要么就地路整条没接上"
    );
    assert_eq!(
        diff, 0,
        "两条路的**签名文本必须逐字节相同**（same={same} diff={diff}）—— \
         文本会被回读成记法目标 ⇒ 分叉就是 elaborate 分叉 ✗"
    );
}

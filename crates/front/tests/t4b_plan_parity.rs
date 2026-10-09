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

/// ⭐ **`export` 这一维：今天两条路**不同**（2026-10-09 · 第 69 轮）。**
///
/// ## 怎么发现的
///
/// 把 CLI 的 `check` 那条路（**没有 progress sink** ⇒ 本来最安全）换成
/// `compile_plan_with_artifacts` 之后，`cli/tests/namespace.rs::
/// export_reaches_the_importing_file_while_open_does_not` **判红** ✗ —— 导入方报
/// `unknown identifier \`mem\`` ⇒ **`export` 没传到** ⇒ 已还原 ✓。
///
/// 根因是**本文件上面那条判据的覆盖缺口** ✗：它的夹具**没有 `export`** ⇒ 两条路在
/// "**导出传播**"这一维上不同而它看不出来 ✓。
///
/// ## 这条判据为什么写成"断言当前行为"
///
/// 按 `AGENTS.md` 的降级纪律（同 T3-B2 的先例）：**先把它钉住、别让它漂** ✓ ——
/// T4-B 把两条路对齐之后，按判据**改判**成 `assert_eq!`（**不许放宽** ✗）。
#[test]
fn the_export_dimension_is_a_known_divergence_today() {
    let root = temp_root("export");
    std::fs::create_dir_all(&root).expect("mkdir");
    std::fs::write(
        root.join("Lib.sokonanoda"),
        "namespace Set\n\
         def mem (α : Type) (a : α) (A : α -> Prop) : Prop := A a\n\
         end Set\n\
         export Set\n",
    )
    .expect("write lib");
    // 入口**不写 `open`**、直接用**导出**的短名 `mem` ⇒ 这一维才被走到 ✓。
    std::fs::write(
        root.join("E.sokonanoda"),
        "import Lib\ndef use (α : Type) (a : α) (A : α -> Prop) : Prop := mem α a A\n",
    )
    .expect("write entry");
    let entry = root.join("E.sokonanoda");
    let options = CompileOptions::default();

    let single = compile_plan_prechecked(
        plan_project(&entry, None, Some(root.as_path())),
        &options,
        None,
    );
    let session =
        compile_plan_with_artifacts(plan_project(&entry, None, Some(root.as_path())), &options);

    let errs = |r: &sokonanoda_front::project::ProjectReport| -> Vec<String> {
        r.modules
            .iter()
            .flat_map(|m| m.report.errors.iter().map(|d| d.code().to_string()))
            .collect()
    };
    let a = errs(&single);
    let b = errs(&session);
    println!("PERF t4b export: 整条一趟={a:?} session+产物={b:?}");

    // ⚠ **断言当前行为**（防漂移 ✓）：两条路在 `export` 这一维上**今天不同** ——
    // "整条一趟"认得导出的短名 ✓，"session+产物"不认 ✗。
    assert!(
        a.is_empty(),
        "夹具前提：**整条一趟**必须认得 `export` 出来的短名 ✓（实得 {a:?}）"
    );
    assert!(
        !b.is_empty(),
        "**已知分歧**：`session+产物` 这条路今天**不认** `export` 短名 ✗ —— 若这里变空，说明 \
         T4-B 已经把两条路对齐了 ✓ ⇒ 按判据**改判**成 `assert_eq!(b, a)`（**不许放宽** ✗）"
    );

    let _ = std::fs::remove_dir_all(&root);
}

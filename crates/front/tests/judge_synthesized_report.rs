//! **T3-B2 的判据读数**（`PLAN-align-lean4.md` §4.3，2026-10-09）：**合成趟**的结构计数。
//!
//! ## 为什么这条读数此前不存在（计划 §4.3 的对账）
//!
//! * `prefix=`（`PREFIX_RUNS`）**只数 `judge_infer` 的合成前缀重跑** ✗ ——
//!   不数 `judge_type_of` / `judge_pairs`（by 路径）的合成趟；
//! * `MODULE_COMPILES` 也不行：它只在 `check::run` 里按 `units.len()` 累加，
//!   而合成趟走 `run_incremental` ✗。
//!
//! ⇒ 「合成趟还剩多少」**没有出口** ⇒ T3-B2（消合成趟）**没有判据**。
//! 本文件把读数建起来并钉住"它真的被走到"（否则后面的判据空转 ✗）。
//!
//! ## 读数三项
//!
//! `synthesized_report()` = `(趟数, Σ命令数, 回退趟数)`：
//! * **趟数** = 真的跑了 `run_synthesized_incremental` 的次数；
//! * **Σ命令数** = 那些趟重新 elaborate 的**合成文档命令数**（= 要消掉的工作量）；
//! * **回退趟数** = 没有担保 ⇒ `check_document_with` / `compile_fol_with` **整份重查**。
//!
//! ## 用法
//!
//! 真课程单元（可分开检出 ⇒ 找不到就跳过，同 `lsp_keystroke_structure.rs` 的口径）：
//! `cargo test -p sokonanoda-front --test judge_synthesized_report -- --nocapture`

use sokonanoda_front::compile::CompileOptions;
use sokonanoda_front::judge::synthesized_report;
use sokonanoda_front::project::{compile_plan, plan_project};

fn course_unit(rel: &str) -> Option<std::path::PathBuf> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("courses")
        .join("set-theory");
    let entry = root.join(rel);
    entry.is_file().then_some(entry)
}

#[test]
fn synthesized_passes_are_counted_on_a_real_unit() {
    let Some(entry) = course_unit("units/I.3/unit08-images-preimages.sokonanoda") else {
        eprintln!("跳过：找不到课程单元（课程仓可分开检出）");
        return;
    };
    let before = synthesized_report();
    let plan = plan_project(&entry, None, None);
    let report = compile_plan(plan, &CompileOptions::default());
    assert!(
        report.diagnostics.is_empty(),
        "夹具必须编过（否则量的是错误路径 ✗）：{:?}",
        report.diagnostics
    );
    let (after, after_cmds, after_fb) = synthesized_report();
    let passes = after - before.0;
    let commands = after_cmds - before.1;
    let fallbacks = after_fb - before.2;
    eprintln!(
        "PERF synthesized-pass unit08: passes={passes} commands={commands} fallbacks={fallbacks}"
    );
    assert!(
        passes > 0,
        "合成趟一次都没被数到（passes={passes}）⇒ **判据空转**：要么计数器没接上，\
         要么这个夹具不触发 judge 的合成路径 ✗"
    );
    assert!(
        commands > 0,
        "合成趟命令数恒 0（commands={commands}）⇒ 计数器接错了（它数的就是重新 elaborate 的工作量）"
    );
}

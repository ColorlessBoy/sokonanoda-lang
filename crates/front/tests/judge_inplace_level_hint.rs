//! **T3-B1 ③（2026-10-09）的判据**：`level_hint_of` 的**记法形态**也走就地路。
//!
//! ## 这条接线是什么
//!
//! `spine::resolve_levels` 展开「头上不带 `.{}`」的常量（`≠` 那类）时要先算
//! **宇宙层级提示**（[`level_hint_of`]）。它有**两种形态**：
//!
//! | 形态 | 问几次 | 就地可行吗 |
//! |---|---|---|
//! | 点式（`args >= params`） | 1 次「首实参的类型」 | ✓（G-29 第 5 棒已接） |
//! | **记法**（`args < params`） | 2 次（`infer(infer(first))`） | ✗ → **✓（本条）** |
//!
//! 记法的**第二问**吃的是**文本**（第一问的结果），而就地路只收**源 AST**
//! ⇒ 以前整支回落 `judge_infer`（**合成 `#check` + 整份前缀重跑** ✗）。
//! T3-B1 ③ 把那份文本回读成 `#check` 的 AST（[`check_ast_of_text`]，与
//! `elab.rs::universe_level_text_of_operands` 的既有做法同形 ✓）再就地 ✓。
//!
//! ## 判据（缺一不算）
//!
//! 1. **不空转**：`inplace_level_hint_report()` 的 `used > 0` —— 专用计数器，
//!    **不与别的档混**（混了就分不清是哪条接线生效 ✗）；
//! 2. **逐字节**：`SOKO_JUDGE_INPLACE=off` vs 默认的 `--json` 相同 ——
//!    手工全课程抽样 24 份已验证 ✓（本条只钉"接线被走到"，全量在发版节点）。
//!
//! ## 为什么用真课程单元
//!
//! 合成夹具很难稳定踩到「记法形态 + 恰好一个宇宙参数 + 裸常量」这条组合
//! （需要真实记法 + delta 展开）；unit08 实测踩到 **3 次** ✓。课程仓可分开
//! 检出 ⇒ 找不到就**跳过**（同 `crates/lsp/tests/lsp_keystroke_structure.rs` 的口径 ✓）。

use sokonanoda_front::compile::CompileOptions;
use sokonanoda_front::judge::inplace_level_hint_report;
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
fn notation_form_level_hint_is_answered_in_place() {
    std::env::set_var("SOKO_JUDGE_INPLACE", "on");
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
    let (used, fallback) = inplace_level_hint_report();
    assert!(
        used > 0,
        "`level_hint_of` 一次都没就地答上（used={used} fallback={fallback}）⇒ **判据空转**：\
         要么记法形态那条支路没接上，要么夹具没踩到它（unit08 实测踩 3 次）"
    );
}

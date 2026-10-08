//! **§11.16 的下一步①**：把那条 hover 回归的**原因串**打出来（读数型 · 不动任何判定代码）。
//!
//! ## 背景
//!
//! LSP 的记法 hover（`crates/lsp/src/lib.rs::notation_symbol_hover`）有两行，其中
//! 「原始类型」那行来自 `judge::judge_type_of_constant(prefix, options, target)`。
//! 该用例在 `SOKO_JUDGE_INPLACE` = On（默认）时**整行消失**、`off` 时正常 ⇒
//! 就地档让这次查询失败/返回空 ✗。而上游两个影子档（type-of-constant / BY）在
//! shadow 跑时全是 0 ⇒ 看不清是哪一支。
//!
//! ⇒ 本文件在 **front 层**复现同一个调用（与 LSP 那行**同参数形状**），把
//! `judge_type_of_constant` 的结果、`type_of_constant_report()` 的 used/fallback、
//! 以及 `type_of_constant_first_diff()` 的原因串**打出来** ✓。
//!
//! ## 跑法（两次各起一个进程 —— 档位是 `OnceLock`，只能靠环境变量）
//!
//! ```text
//! cargo test -p sokonanoda-front --test inplace_notation_hover_probe -- --nocapture
//! SOKO_JUDGE_INPLACE=off cargo test -p sokonanoda-front --test inplace_notation_hover_probe -- --nocapture
//! ```
//!
//! 两次的 `PERF`/`PROBE` 行并排读 ⇒ 差别就是答案 ✓。
//!
//! ⚠ **不许**在这里断言"就地档必须失败"（那是把 bug 钉成契约 ✗）——本文件只出读数；
//! 真正的判据是 `crates/lsp` 的那条 hover 用例转绿 ✓。

use sokonanoda_front::compile::CompileOptions;
use sokonanoda_front::judge;
use sokonanoda_front::query::QueryDoc;

/// 与 LSP 用例**同一个夹具**（本文件声明的记法 `⊗` ⇒ 目标是同文件的 `myop`）。
const SRC: &str = "def myop (a b : Prop) : Prop := a\ninfix:50 \" ⊗ \" => myop\ntheorem t (a b : Prop) (h : a ⊗ b) : a ⊗ b := h\n";

#[test]
fn what_does_judge_type_of_constant_answer_for_a_local_notation_target() {
    let dir = std::env::temp_dir().join(format!("soko-inplace-notation-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    let entry = dir.join("Main.sokonanoda");
    std::fs::write(&entry, SRC).expect("write");

    let mut doc = QueryDoc::new();
    doc.path = Some(entry.clone());
    doc.set_text(SRC, 1, None);
    assert!(doc.report.is_some(), "夹具必须编过（否则量的是错误路径 ✗）");

    // **LSP 那行的同参数形状**：`prefix` 取整份源（LSP 取的是闭包前缀，本夹具无 import
    // ⇒ 前缀就是本文件 ✓），`target` = `myop`（记法的展开目标）。
    let options = CompileOptions::default();
    let slow = judge::judge_type_of_constant(SRC, &options, "myop");
    let (used, fallback) = judge::type_of_constant_report();
    let (same, diff) = judge::type_of_constant_shadow();
    println!(
        "PROBE type_of_constant(myop): slow={:?} · used={used} fallback={fallback} \
         shadow_same={same} shadow_diff={diff}",
        slow.as_ref().map(|s| s.as_str())
    );
    println!(
        "PROBE first_diff: {:?}",
        judge::type_of_constant_first_diff().map(|s| s.chars().take(200).collect::<String>())
    );
    // **记法符号本身的 type**（`⊗` 的展开目标就是 `myop`）：确认悬停点解析出的目标名。
    // **LSP 真正传的那个 prefix**（`QueryDoc::judge_prefix`）：单文件（无 import）
    // ⇒ `project_modules()` 为 `None` ⇒ **空串** ✗（入口自己的文本**不在**前缀里）。
    let lsp_prefix = doc.judge_prefix(SRC.rfind('⊗').expect("use site"));
    let lsp = judge::judge_type_of_constant(&lsp_prefix, &options, "myop");
    println!(
        "PROBE LSP-shape: prefix.len()={} judge_type_of_constant(myop)={:?}",
        lsp_prefix.len(),
        lsp.as_ref().map(|s| s.as_str())
    );
    let notation = doc.notation_at(SRC, SRC.rfind('⊗').expect("use site"));
    println!("PROBE notation_at(⊗) = {notation:?}");

    let _ = std::fs::remove_dir_all(&dir);
}

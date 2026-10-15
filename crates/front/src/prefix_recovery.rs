//! 判卷**合成前缀**的解析：先严格、失败再**恢复式**。
//!
//! ## 为什么（`SUB-parse-error-kills-whole-file` 只修了一半，2026-10-15 实测）
//!
//! `crates/front/src/project/graph.rs` 已经把**入口**的恢复式解析结果放进
//! `file.commands`（"一处语法错不许废掉整份文件" ✓）。但**判卷合成**那条路
//! （`judge.rs::parse_prefix`）仍然走**严格**的 `parse_fragment`：
//!
//! ```text
//! let full_prefix = synthesized_prefix(extra_prefix, &doc_src[..decl_start]);
//! let Ok(mut file) = parse_prefix(&full_prefix) else {
//!     return all_parse_error("前缀源码无法解析".to_string());   // ← 全部判定失效 ✗
//! };
//! ```
//!
//! ⇒ 一处 parse 错**仍然**让它**后面**每一条声明的 tactic 判定全废
//! （实测：`theorem a` / `theorem broken`（坏） / `theorem c` 三声明，
//! `c` 照常进 `commands`、照常被判定 ⇒ 但报
//! `elab-tactic-failed … 前缀源码无法解析` ✗）。用户原话的形态正是这个：
//! 「**声明列表不回来**」✗ —— 修复承诺的是"出错的那一条没有判定"，
//! 而**后面**那些好声明也被一起带走了 ✗。
//!
//! ⚠ 这与 **G-86** 是**同一个类**（"合成前缀解析失败 ⇒ 库层入口被判红"），
//! 只是触发源不同（那次是记法符号粘连在标识符上，这次是前缀里有一条坏命令）
//! ⇒ 正是"**修实例不修类**"的又一次现场。
//!
//! ## 修法（**只在失败路径上动手** ⇒ 干净输入逐字节不变 ✓）
//!
//! 先试严格的 [`crate::parse_fragment`]（**成功时与改动前逐字相同** ✓）；
//! 只有它失败时才退回 [`crate::parse_recovering_with_inherited`]。
//!
//! **为什么恢复式在这条路上语义是对的**：`parse_file_recovering` 对"文件尾未闭合
//! `namespace`"的处理本来就是**诊断而不是判决**（G-05 片段语义，见它的文档），
//! 命名空间照样保持打开 ⇒ 拼在后面的合成 `#check` / 合成声明仍落在里面 ✓；
//! 而它对坏命令的处理（克隆上试、至少前进一个 token、失败不留副作用）正是
//! `docs/design/declaration-incremental.md` 要的"逐条对齐 Lean 4 模块级恢复" ✓。
//!
//! ⚠ **词法失败照旧判红**：`parse_recovering_with_inherited` 连词法都过不去时
//! 仍返回 `Err` ⇒ 本函数返回 `Err(())` ⇒ 调用方维持原来的
//! `all_parse_error("前缀源码无法解析")` 行为，一个字不改 ✓。

use crate::FolFile;

/// 解析判卷合成前缀：**严格优先，恢复式兜底**。
///
/// 返回 `Err(())` 只在**两条路都过不去**时（词法失败）——与改动前同判 ✓。
pub(crate) fn parse_prefix_recovering(src: &str) -> Result<FolFile, ()> {
    if let Ok(file) = crate::parse_fragment(src) {
        return Ok(file);
    }
    crate::parse_recovering_with_inherited(src, &[])
        .map(|recovered| recovered.file)
        .map_err(|_| ())
}

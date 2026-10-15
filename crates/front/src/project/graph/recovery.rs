//! **入口解析失败时的恢复**（`SUB-parse-error-kills-whole-file`，用户 2026-10-15 亲报）。
//!
//! 从 `graph.rs` 拆出来（`docs-lint` 判据 ⑨：那个文件在存量基线上，**只许减不许增**）——
//! 本模块只管"入口没解析成功时，还能救回什么"，与闭包加载的其余部分无关。
//!
//! **Lean 4 对照**（`~/Documents/lean/lean4/src/Lean/Parser/Module.lean:137` 的
//! `parseCommand`）：Lean 的模块解析是**逐条命令**的，失败时 `recovering := true`、
//! "advance at least one token to prevent infinite loops"，然后**继续**解析下一条
//! （`:162-175`）⇒ 一处语法错**不会**让别的声明从 infoview 消失。我们**早就有**同一条
//! 语义（`crate::Parser::parse_file_recovering`，它的注释自己写着"语义逐条对齐 Lean 4
//! 的模块级恢复"），只是闭包这条路没用它 ✗。

use crate::ast::NotationDecl;
use crate::FolFile;

/// **恢复式**解析入口，拿回"能解析的那些命令"（失败 ⇒ 空 `FolFile`）。
///
/// ⚠ **只对入口**：依赖模块解析失败仍走 `import-module-invalid` + 阻断（它的常量本来
/// 就传不过来 ⇒ 下游判不了），协议码与语义**一字不变** ✓。
pub(super) fn recovered_entry_file(text: &str, inherited: &[NotationDecl]) -> FolFile {
    crate::parse_recovering_with_inherited(text, inherited)
        .map(|recovered| recovered.file)
        .unwrap_or(FolFile {
            commands: Vec::new(),
            src: text.to_string(),
        })
}

/// **r16（2026-10-13）**：恢复式解析一遍，列出**已经解析成功**的顶层声明名（按源序 ✓）。
///
/// 给**诊断消息**用（同一份恢复结果也进 `file.commands` 去编译，见
/// [`recovered_entry_file`]）。拿不到（词法失败 / 一条都救不回来）⇒ 空 ⇒ 消息与
/// 改动前**逐字相同** ✓。
pub(super) fn recovered_decl_names(
    text: &str,
    inherited: &[NotationDecl],
    error_at: usize,
) -> Vec<String> {
    let Ok(recovered) = crate::parse_recovering_with_inherited(text, inherited) else {
        return Vec::new();
    };
    let mut named: Vec<(usize, usize, String)> =
        crate::compile::top_level_def_spans(&recovered.file)
            .into_iter()
            .map(|(name, span)| (span.start.offset, span.end.offset, name))
            .collect();
    named.sort();
    // ⚠ **坏掉的那一条不许列**：恢复式解析会把"出错那条命令的前半截"也救回来
    // （**实测**：r16 夹具里 `broken` 连同 `by constructor` 都在恢复出来的命令表里，
    // 它的 span 在错误**之前**就结束了）⇒ 直接列会把**出错的那条**说成"已经解析成功" ✗。
    //
    // 判据（**按内容找、不猜名字** ✓）：最后一条起点在错误之前的声明，且**它的 span
    // 结束到错误之间只有空白** ⇒ 这个错误就长在它的体内 ⇒ 不列它。
    // 反例（同一判据必须放行）：项目测试的夹具里 `axiom Other.bullet` 之后还有一条
    // `infixl` 才到那个裸 `·` ⇒ 中间有非空白文本 ⇒ **保留** `Other.bullet` ✓。
    if let Some(index) = named.iter().rposition(|(start, _, _)| *start <= error_at) {
        let end = named[index].1;
        let tail = text.get(end..error_at).unwrap_or("x");
        if tail.chars().all(char::is_whitespace) {
            named.remove(index);
        }
    }
    named.into_iter().map(|(_, _, name)| name).collect()
}

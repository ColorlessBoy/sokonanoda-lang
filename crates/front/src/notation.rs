//! 记法表的**唯一重建实现**（T-C04）。
//!
//! 设计：`docs/design/notation-aware-printing.md` §3.3。这里放"从源码里收出当前
//! 生效的记法表"这件事——**只此一份实现**，两条路共用：
//!
//! * **判定路径**（`judge::judge_pairs_uncached`）：回读目标文本时要按同一张表
//!   解析，否则带记法的目标会 `parse` 失败；
//! * **显示路径**（线 C 的 print-back）：反向折叠要用同一张表，否则"展开"与
//!   "折回"两边会分叉——那正是本仓库最忌讳的"两套真相"。
//!
//! 输入是**已经解析好的命令序列**：判定路径本来就要为后面的合成声明解析前缀
//! （零额外开销），显示路径从 `ModuleReport.source` 解析一次。

use crate::ast::{Command, NotationDecl};

/// 从一段**已解析**的命令序列里收出有效的记法表。
///
/// 规则（第三刀 §12.3）：
///
/// * 按**声明顺序**收（同 target 声明了两个符号时，反向折叠取第一个 ⇒ 顺序有意义）；
/// * `scoped` 记法在**这段文本结束时**是否生效，判据是「这段里出现过
///   `open scoped <它的作用域>`」——**两遍扫描**：先把所有 `open scoped` 收齐，
///   再过滤记法。
///
/// **为什么是两遍**（G-35，2026-09-21 修）：原先是一遍，于是
/// 「先 `scoped infix` 声明、后 `open scoped`」（**正常写法**：声明在库里、
/// `open` 在使用处）收不到那条记法。而主通道（parser）**两个方向都对**：
/// `open scoped` 会激活**已声明**的 scoped 记法（`activate_scope` 的 pending
/// 表），也会记住作用域让**之后**声明的直接生效。读回通道只有一段前缀、
/// 不关心"用在哪一行"，所以正确答案就是"前缀结束时生效的那些"——那正是
/// 两遍扫描。
pub(crate) fn notation_table(commands: &[Command]) -> Vec<NotationDecl> {
    // 第一遍：这段文本里开过哪些作用域。
    let mut opened_scopes: Vec<String> = Vec::new();
    for command in commands {
        if let Command::Open {
            name, scoped: true, ..
        } = command
        {
            if !opened_scopes.contains(name) {
                opened_scopes.push(name.clone());
            }
        }
    }
    // 第二遍：按声明顺序收记法，`scoped` 的看它的作用域开没开。
    let mut notations: Vec<NotationDecl> = Vec::new();
    for command in commands {
        let Some(decl) = command.notation_decl() else {
            continue;
        };
        match &decl.scope {
            Some(scope) if !opened_scopes.contains(scope) => {}
            _ => notations.push(decl),
        }
    }
    notations
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(src: &str) -> Vec<NotationDecl> {
        let file = crate::parse(src).expect("夹具必须能解析");
        notation_table(&file.commands)
    }

    /// 按声明顺序收，符号与 target 都对得上。
    #[test]
    fn collects_notations_in_declaration_order() {
        let got = table(
            "def A : Prop := True\n\
             def B : Prop := True\n\
             infix:50 \" ∈ \" => A\n\
             infix:60 \" ⊆ \" => B\n",
        );
        let symbols: Vec<&str> = got.iter().map(|d| d.symbol.as_str()).collect();
        assert_eq!(symbols, vec!["∈", "⊆"], "顺序 = 声明顺序");
        assert_eq!(got[0].target, "A");
        assert_eq!(got[1].target, "B");
    }

    /// `scoped` 记法**没** `open scoped` 时不在表里。
    #[test]
    fn scoped_notation_is_filtered_out_without_open() {
        let got = table(
            "def A : Prop := True\n\
             namespace Foo\n\
             scoped infix:50 \" ⊕ \" => A\n\
             end Foo\n",
        );
        assert!(got.is_empty(), "没开作用域 ⇒ 不生效：{got:?}");
    }

    /// **`open scoped` 写在记法之前或之后都收得到**（G-35 修，2026-09-21）。
    ///
    /// 主通道（parser）两个方向都对：`open scoped` 既激活**已声明**的 scoped
    /// 记法，也记住作用域让**之后**声明的直接生效。读回表只关心"前缀结束时
    /// 生效的那些"，所以两边都该收——**两遍扫描**（先收齐 `open scoped`，再过滤）。
    #[test]
    fn open_scoped_after_the_notation_is_collected() {
        // `open scoped` 在**后**（正常写法：声明在库里、open 在使用处）。
        let after = table(
            "def A : Prop := True\n\
             namespace Foo\n\
             scoped infix:50 \" ⊕ \" => A\n\
             end Foo\n\
             open scoped Foo\n",
        );
        let symbols: Vec<&str> = after.iter().map(|d| d.symbol.as_str()).collect();
        assert_eq!(symbols, vec!["⊕"], "open 在记法之后也要收得到（G-35）");

        // `open scoped` 在**前**（第二次进同一个 namespace 声明的那条）。
        let before = table(
            "def A : Prop := True\n\
             namespace Foo\n\
             scoped infix:50 \" ⊕ \" => A\n\
             end Foo\n\
             open scoped Foo\n\
             namespace Foo\n\
             scoped infix:60 \" ⊗ \" => A\n\
             end Foo\n",
        );
        let symbols: Vec<&str> = before.iter().map(|d| d.symbol.as_str()).collect();
        assert_eq!(symbols, vec!["⊕", "⊗"], "两条都在（顺序 = 声明顺序）");
    }

    /// 非 `scoped` 的记法不受 `open scoped` 影响（第二刀行为）。
    #[test]
    fn plain_notation_is_always_in_the_table() {
        let got = table("def A : Prop := True\ninfix:50 \" ∈ \" => A\n");
        assert_eq!(got.len(), 1, "普通记法一直生效：{got:?}");
    }
}

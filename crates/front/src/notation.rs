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
/// 规则（第三刀 §12.3，**行为逐字节不变地**从 `judge.rs` 提出来）：
///
/// * 按**声明顺序**收（同 target 声明了两个符号时，反向折叠取第一个 ⇒ 顺序有意义）；
/// * `scoped` 记法按「这段里**已经扫到过** `open scoped <名字>`」过滤。
///
/// ⚠ **它是"扫一遍"而不是"两遍"**：`open scoped` 写在 `scoped infix` **之前**
/// 才收得到；写在之后（先声明、后 open，也就是**正常的写法**）就收不到。
/// 台账 **G-35** 记着这件事；课程今天不用 `scoped` 记法（`grep "open scoped"
/// courses/` = 0），所以它是**潜在的**，不是正在伤人的。特征化测试
/// `open_scoped_after_the_notation_does_not_bring_it_back` 钉住当前行为——
/// 显示路径（线 C）会复用这张表，两边**一起漏**至少一致，"一边漏一边不漏"才是灾难。
pub(crate) fn notation_table(commands: &[Command]) -> Vec<NotationDecl> {
    let mut opened_scopes: Vec<String> = Vec::new();
    let mut notations: Vec<NotationDecl> = Vec::new();
    for command in commands {
        if let Command::Open {
            name, scoped: true, ..
        } = command
        {
            if !opened_scopes.contains(name) {
                opened_scopes.push(name.clone());
            }
            continue;
        }
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

    /// **`open scoped` 要写在记法声明之前**——这个函数是**扫一遍**的。
    ///
    /// 这是**当前行为**（特征化，不是"正确行为"）：先遇到 `scoped` 记法时
    /// `opened_scopes` 还是空的 ⇒ 被过滤掉；后面那句 `open scoped` 补不回来。
    /// 台账 **G-35** 记着这件事——课程今天不用 `scoped` 记法
    /// （`grep "open scoped" courses/` = 0），所以它是**潜在的**。
    ///
    /// 为什么仍然值得钉：显示路径（线 C）会**复用同一张表**，两边必须同口径
    /// ——"一起漏"至少是一致的，"一边漏一边不漏"才是灾难。
    #[test]
    fn open_scoped_after_the_notation_does_not_bring_it_back() {
        // `open scoped Foo` 在**后** ⇒ 先声明的那条收不到（G-35）。
        let after = table(
            "def A : Prop := True\n\
             namespace Foo\n\
             scoped infix:50 \" ⊕ \" => A\n\
             end Foo\n\
             open scoped Foo\n",
        );
        assert!(
            after.is_empty(),
            "open 在记法之后 ⇒ 扫不到（G-35）：{after:?}"
        );

        // 同一条记法，但 `open scoped Foo` 已经在**前**（第二次进同一个 namespace）
        // ⇒ 收得到。这条同时证明"过滤"本身是好的，问题只在顺序。
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
        assert_eq!(symbols, vec!["⊗"], "open 在前 ⇒ 之后声明的那条收得到");
    }

    /// 非 `scoped` 的记法不受 `open scoped` 影响（第二刀行为）。
    #[test]
    fn plain_notation_is_always_in_the_table() {
        let got = table("def A : Prop := True\ninfix:50 \" ∈ \" => A\n");
        assert_eq!(got.len(), 1, "普通记法一直生效：{got:?}");
    }
}

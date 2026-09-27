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
/// **给 front 的消费者建表用**（T-U11 ✓ 2026-09-25 由 `pub(crate)` 放开 ✓）：
/// LSP 那侧要折 hover 文本 ✓，但它**不许自己造 arity** ✗（= 第五套实现 ✓，守卫会抓 ✓）
/// ⇒ 由 front 提供入口 ✓（见 `display::fold_for_display` ✓）。
pub fn notation_table(commands: &[Command]) -> Vec<NotationDecl> {
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

/// **内建记法**（`↔` / `∧` / `∨` / `¬` / `=` / `≠`）的声明形状。
///
/// **它们不在任何源文本里**：parser 有一张硬编码表（`parser.rs` 的
/// `BUILTIN_NOTATIONS`），词法 + 解析直接认。所以"从源里收记法"的
/// [`notation_table`] **收不到它们**——显示层必须自己补上，否则 `Iff` 永远折不成
/// `↔`（T-C20 接进生产者时实测撞到：`query goals` 的 `ty` 里 `Iff` 还在点名）。
///
/// **只给显示层用**：回读路径（judge / by）走的是 parser 的原生内建表，
/// 往里塞一份反而可能撞车。
pub(crate) fn builtin_notation_decls() -> Vec<NotationDecl> {
    let prelude = crate::compile::prelude_source();
    crate::parser::builtin_notations()
        .iter()
        .map(|(symbol, assoc, precedence, target)| NotationDecl {
            symbol: (*symbol).to_string(),
            precedence: Some(*precedence),
            assoc: *assoc,
            target: (*target).to_string(),
            // 内建记法一直生效（没有 `scoped` 形式）。
            scope: None,
            // **E10（v0.76.0）：声明点** —— 语言**故意拒绝**重新声明内建记法
            //（实测 `notation-shape`：符号 `∧` 是语言内建记法、不需要也不能重新声明 ✗✓；
            // 那条守卫防的是「记法概念分叉」，与 E11 第 ③ 条同一纪律 ✓）。
            // ⇒ 走仓库**已有**的 `-- sokonanoda:<指令>` 注释约定登记声明点
            //（**零新增语法** ✓）：span 指向 prelude 源里那一行 ✓。
            span: builtin_directive_span(prelude, symbol).unwrap_or_default(),
            // 内建仍**不属于任何模块**（prelude 不是模块）—— 落点那一跳由 LSP 侧
            // 走 prelude 源（`prelude_source_path()`）✓，不靠模块路径 ✗。
            module: None,
        })
        .collect()
}

/// **E10 的接缝判据要用它**（集成测试在 crate 外，`builtin_notation_decls` 是
/// `pub(crate)` ✗）—— 只读、无副作用 ✓。
pub fn builtin_notation_decls_for_test() -> Vec<NotationDecl> {
    builtin_notation_decls()
}

/// **E10**：内建记法在 prelude 源里的**指令行** span。
///
/// 那一行的形状是 `-- sokonanoda:builtin-notation "<符号>" => <目标>`（见
/// `compile/prelude.rs` 的 `PRELUDE_L1_SRC` 尾部 ✓）。返回**整行**的 span ——
/// 与 `infix:50 " ∈ " => Set.mem` 的 span 口径一致
/// （见 `span_tests::a_notation_decl_carries_its_own_span` ✓）。
fn builtin_directive_span(prelude: &str, symbol: &str) -> Option<crate::Span> {
    let needle = format!("-- sokonanoda:builtin-notation \"{symbol}\" =>");
    let mut offset = 0usize;
    for (index, line) in prelude.split_inclusive('\n').enumerate() {
        let trimmed = line.trim_end_matches('\n');
        if trimmed.trim_start().starts_with(&needle) {
            let indent = trimmed.len() - trimmed.trim_start().len();
            let start = offset + indent;
            let end = offset + trimmed.len();
            let pos = |at: usize, column: usize| crate::Pos {
                offset: at,
                line: index + 1,
                column,
            };
            return Some(crate::Span::new(
                pos(start, indent + 1),
                pos(end, trimmed.len() + 1),
            ));
        }
        offset += line.len();
    }
    None
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

#[cfg(test)]
mod span_tests {
    use super::*;

    /// **T-D10 的判据**：`infix:50 " ∈ " => Set.mem` 的 `span` **逐字等于那一行**。
    ///
    /// 这条是"记法跳转"的地基：只有 `target` 这个名字不够——记法跨 `import`
    /// 传播，入口里写 `∈`、声明在 `lib/Set.sokonanoda`，所以还得知道**在哪一段**。
    #[test]
    fn a_notation_decl_carries_its_own_span() {
        let src = "def Set.mem (α : Type) (a : α) (A : Set α) : Prop := A a\n\
                   infix:50 \" ∈ \" => Set.mem\n";
        let file = crate::parse(src).expect("夹具必须能解析");
        let decl = file.commands[1].notation_decl().expect("第二条是记法");
        let line = src.lines().nth(1).expect("第二行");
        assert_eq!(
            &src[decl.span.start.offset..decl.span.end.offset],
            line,
            "span 必须逐字等于声明那一行"
        );
        assert_eq!(decl.span.start.line, 2, "1-based 行号");
        // 模块名由加载层补（`absorb_notations`）——命令自己不知道。
        assert_eq!(decl.module, None);
    }

    /// **E10（v0.76.0）：内建记法现在**有**声明点** —— 在 prelude 源里那条
    /// `-- sokonanoda:builtin-notation "∧" => And` **指令行**上 ✓。
    ///
    /// 旧判据（T-D10）断言的是「内建没有声明点：`span.start.offset == 0`」✗ ——
    /// 那是**当时的真相**，也是学生按 F12 无处可去的原因 ✓。E10 换了路
    /// （不能重声明 ⇒ 用指令登记 ✓，见 `redeclaring_a_builtin_notation_is_rejected`），
    /// 所以这条判据**跟着行为一起改** ✓（改行为不改判据 = 没做完 ✗）。
    ///
    /// 断言的是**具体值**：`span` 圈出来的文本**逐字等于**那一行指令 ✓ ——
    /// 只断言"不是默认值"会放过"指到了别的行" ✗。
    #[test]
    fn builtin_notations_carry_their_directive_line_as_the_declaration_site() {
        let prelude = crate::compile::prelude_source();
        let mut checked = 0;
        for decl in builtin_notation_decls() {
            // `=` 不在 prelude 里登记（最长匹配会把 `=>` 吃坏 ✗）⇒ 它仍然没有声明点 ✓。
            if decl.symbol == "=" {
                assert_eq!(decl.span.start.offset, 0, "`=` 不登记，见 prelude 的注释 ✗");
                continue;
            }
            assert_eq!(
                decl.module, None,
                "内建不属于任何模块（prelude 不是模块）：{}",
                decl.symbol
            );
            let at = &prelude[decl.span.start.offset..decl.span.end.offset];
            assert!(
                at.starts_with(&format!(
                    "-- sokonanoda:builtin-notation \"{}\" =>",
                    decl.symbol
                )),
                "声明点必须**逐字**指向那条指令行 ✗（实际圈到：{at:?}）"
            );
            assert!(
                at.ends_with(decl.target.as_str()),
                "指令行必须以目标名结尾（{at:?} vs {}）",
                decl.target
            );
            checked += 1;
        }
        assert!(
            checked >= 5,
            "至少要覆盖 ∧/∨/↔/¬/≠ 五条（实测 {checked} 条）"
        );
    }

    /// **E10 的判据（v0.76.0）—— 先钉住"为什么不能直接声明"** ✗✓。
    ///
    /// 计划 §E10 的原话是「内建记法 `∧ ∨ ↔ ¬ →` **写进 prelude 当声明点**」。
    /// **实测：这条路走不通** ✗ —— 语言**故意**拒绝重新声明内建记法：
    ///
    /// ```text
    /// $ scripts/soko grade /tmp/t.sokonanoda --json      # 文件里写 infixr:35 " ∧ " => And
    /// {"code":"notation-shape",
    ///  "message":"符号 `∧` 是**语言内建记法**（Lean core 级的逻辑符号），不需要也不能重新声明；直接用就行"}
    /// ```
    ///
    /// 这条守卫是**对的** ✓（它防的正是"记法概念分叉" —— 与 E11 的第 ③ 条同一条纪律 ✓），
    /// 所以 **E10 要换路**：让内建记法**在不重新声明的前提下拿到声明点** ——
    /// 走仓库**已有**的 `-- sokonanoda:<指令>` 注释约定（与 E11 的 `builtin-sugar` 同一套 ✓，
    /// **零新增语法** ✓），让内建表/记法表带上 prelude 里那一行的 span ✓。
    ///
    /// ⚠ 本判据钉的是**现状（拒绝重声明）**：E10 换路实现之后它**仍然必须绿** ✓
    /// （那时"写进 prelude"用的是**指令注释**，不是 `infixr:` 声明 ✓）。
    /// 反向验证：把下面这条 `infixr:35` 改成**非内建**符号（如 `⊗`）⇒ 解析**成功** ⇒ 判红 ✓。
    #[test]
    fn redeclaring_a_builtin_notation_is_rejected() {
        let src = "def And (a b : Prop) : Prop := a\ninfixr:35 \" ∧ \" => And\n";
        assert!(
            crate::parse(src).is_err(),
            "语言**故意**拒绝重新声明内建记法（`notation-shape`）—— 这是 E10 必须换路的实测依据 ✗✓"
        );
        // 对照：**非内建**符号在同样位置是合法的 ⇒ 上面那条红不是"记法声明本身不行" ✓。
        let ok = "def And (a b : Prop) : Prop := a\ninfixr:35 \" ⊗ \" => And\n";
        assert!(
            crate::parse(ok).is_ok(),
            "非内建符号的记法声明必须照常合法（对照组）✗"
        );
    }
}

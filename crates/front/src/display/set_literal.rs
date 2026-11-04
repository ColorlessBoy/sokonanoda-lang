//! 集合字面量的**点名展开** → [`Expr::SetLiteral`]（A2）。
//!
//! 自 `display.rs` 拆出（2026-11-04，`docs-lint` ⑨：`display.rs` 在软线之上，
//! **只许减不许增** ⇒ 本模块的改动搬到这里，`display.rs` 净减）。
//!
//! 在这里补的一刀是 **issue #7 第 1 条**：`@Set.singleton α a`（内核装配的
//! **显式脊**）也要折回 `{a}` —— 见下面那段歧义说明。

use crate::ast::Expr;

/// 集合字面量的**点名展开** → [`Expr::SetLiteral`]（A2）。
///
/// `{a}` 的展开是 `Set.singleton α a`（`elab.rs` 的 `set_literal_*`），`{a, b}` 是
/// `Set.pair α a b`；第一个实参是**元素类型**（展开时补上的前导参数）⇒ 丢掉。
/// `render_expr(SetLiteral)` 本来就打成 `{a}` / `{a, b}` ✓，所以折出来的节点直接
/// 复用**既有**渲染规则，不新增第二套括号/逗号规则。
///
/// **两种 pp 形状都收**（与记法同一条规则，切片：折坏部分应用）：`Set.singleton`
/// 的 `k == 2`（开项全形 `Set.singleton α a`）与 `k == 1`（闭项省略形
/// `Set.singleton a`，**这正是课程 goal 里的形状** `Set.mem a (Set.singleton a)`）；
/// `Set.pair` 的 `k == 3` / `k == 2`。元素**从尾部取**：更小的 `k` ⇒ `None`。
pub(super) fn fold_set_literal(root: &Expr, name: &str, args: &[&Expr]) -> Option<Expr> {
    let k = args.len();
    let span = root.span();
    // **只折闭项形状**（2026-10-11 用户实测的第 3 条尾巴）。
    //
    // 原来的规则把**开项**也收（`Set.singleton α a` ⇒ `{a}`）—— 而开项与
    // 「**闭项完全应用再被应用一次**」**逐字同形**（`Set.singleton a a` = `(Set.singleton a) a`）✗
    // ⇒ 折叠按开项读法把后者折成 `{a}` ✗ **丢掉那次应用**：实测 `apply h` 之后
    // `unfold Set.mem` 的目标 `(Set.singleton a) a` 在**目标面板**与**错误消息**里
    // 都印成 `{a}` ✗（那是集合，真目标是 Prop ⇒ **是错的** ✗）。
    //
    // 折叠是**纯文本**通道 ⇒ 分不开这两种读法 ✗；**AST 才分得开**（`SetLiteral` 头
    // vs `Ident` 头 + 实参个数）⇒ 这条歧义只能在"少折一点"这一侧收掉：**开项不折** ✓
    // （它只在 pp 打出隐式实参时出现，罕见），于是整条脊不折、但**内层闭项照折** ✓
    // ⇒ `Set.singleton a a` 折成 `({a}) a` ✓（真值，且不再撒谎 ✓）。
    //
    // **显式脊（`@`）没有那条歧义**（2026-11-04 issue #7 第 1 条）：`@Set.singleton α a`
    // 是**逐位写全**的完全应用，与「闭项再被应用」不同形（后者打成
    // `Set.singleton a a`，源头没有 `@`）⇒ **折** ✓。用户现场：`intro hx` 之后目标面板
    // 印 `hx : @Set.singleton α a = ∅`（`by` 步进记下的 binder 类型来自**内核装配的
    // 显式脊**，`@` 恒在）⇒ 修前 singleton 一次都没折。
    let explicit = crate::expr::at_flag(root);
    let elements: Vec<Expr> = match (name, explicit) {
        ("Set.singleton", true) if k == 2 => vec![args[k - 1].clone()],
        ("Set.pair", true) if k == 3 => vec![args[k - 2].clone(), args[k - 1].clone()],
        ("Set.singleton", false) if k == 1 => vec![args[k - 1].clone()],
        ("Set.pair", false) if k == 2 => vec![args[k - 2].clone(), args[k - 1].clone()],
        _ => return None,
    };
    Some(Expr::SetLiteral { elements, span })
}

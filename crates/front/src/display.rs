//! **显示专用文本**：线 C（goal / 类型行用记法）的编译期护栏。
//!
//! 设计：`docs/design/notation-aware-printing.md` §3.5；消费者审计见同文 §2。
//!
//! **为什么需要它**：线 C 的 print-back 把"给人看"的文本重写成记法形式
//! （`Set.mem α a A` → `a ∈ A`），而**同一批字段里有几个同时是 judge 的输入**
//! （`DeclState.goal` / `binders[].ty` / `sub_goals[].ty` —— `suggest.rs` 的
//! `open_spec` 与 `goals.rs` 的 `binder_spec` 会拿它们去合成判定规格，再由
//! `judge_terms_uncached` 走 `parse_expr_text_with` 回读）。把两者混起来是
//! **静默改坏判卷**：学习者看到"判过了"，或者本该过的被判红。
//!
//! ⇒ 用**类型**把"给人看"与"回读"分开：
//!
//! * 没有 `Deref<Target = str>`、没有 `as_str()`、没有 `Into<String>`；
//! * 唯一的读法是 [`DisplayText::as_display_str`]。
//!
//! 于是下面两条**编译不过**——这比注释与 review 可靠：

/// **护栏 1/2：回读路径编译不过**——`parse_expr_text` 要 `&str`，而 `DisplayText`
/// 没有 `Deref<Target = str>`，所以这行过不了类型检查。
///
/// **这条对 `Deref` 敏感**：谁给 `DisplayText` 加上 `Deref`（或让它可以被当成
/// `&str` 用），这条 doctest 立刻从"编译失败"变成"编译通过" ⇒ **红**。
///
/// ```compile_fail
/// use sokonanoda_front::display::DisplayText;
/// use sokonanoda_front::proof::parse_expr_text;
///
/// let shown = DisplayText::new("Set.mem α a A");
/// let _ = parse_expr_text(&shown); // 编译错误：expected `&str`, found `&DisplayText`
/// ```
///
/// **护栏 2/2：没有 `as_str()` 这种"顺手拿回 `&str`"的口子**。
///
/// **这条对"加个访问器"敏感**：`as_str()` 一旦存在，取到 `&str` 之后就能喂给
/// 任何回读入口 ⇒ 护栏形同虚设。
///
/// ```compile_fail
/// use sokonanoda_front::display::DisplayText;
///
/// let shown = DisplayText::new("a ∈ A");
/// let _: &str = shown.as_str(); // 编译错误：no method named `as_str`
/// ```
///
/// 对照（**必须编译过**）：显示出口的读法就一条。
///
/// ```
/// use sokonanoda_front::display::DisplayText;
///
/// let shown = DisplayText::new("a ∈ A");
/// assert_eq!(shown.as_display_str(), "a ∈ A");
/// assert_eq!(shown.to_string(), "a ∈ A"); // `Display` 是给 `format!`/wire 用的
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DisplayText(String);

impl DisplayText {
    /// 包一段**只给人看**的文本。
    pub fn new(text: impl Into<String>) -> Self {
        Self(text.into())
    }

    /// 显示出口唯一的读法（`format!` / JSON wire / fenced code block 都用它）。
    pub fn as_display_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for DisplayText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<String> for DisplayText {
    fn from(text: String) -> Self {
        Self(text)
    }
}

impl From<&str> for DisplayText {
    fn from(text: &str) -> Self {
        Self(text.to_string())
    }
}

// ---- 显示期的记法折叠（T-C10 第一刀：二元 infix 族）------------------------
//
// 设计：`docs/design/notation-aware-printing.md` §3.3。这是**反向**那一步：
// 前向（elab）把 `a ∈ A` 展开成 `Set.mem α a A`，这里把它折回去。
//
// 三条纪律（都来自设计，别在这里"顺手放宽"）：
//   * **命中不了就原样返回**——不猜。折错比不折坏得多（用户看到的是错式子）。
//   * **只有"完全应用"才是记法实例**（§3.2 硬规则）：`Set.mem α a`（部分应用）
//     不许打成 `α ∈ a`。⚠ **pp 的"完全应用"有两种形状**（见
//     [`DisplayNotations`] 的「前导隐式个数」）：闭项省略前导隐式实参、开项不省略
//     ⇒ 两种实参个数都算完全应用；而**同一条脊的内层节点**不是（由脊根负责折，
//     见 [`fold_collecting_inner`] 的 `is_fun_part`）。
//   * **产物是 [`DisplayText`]**：它进不了任何回读通道（编译期保证）。

use crate::ast::{Binder, Expr, MatchArm, NotationAssoc, NotationDecl};
use crate::Span;
use std::collections::HashMap;

/// 显示期的记法表（设计 §3.3）：记法声明 + 「target 点名 → 元数」+「target 点名 →
/// 前导隐式个数」。
///
/// **元数**（arity）= 目标声明的 **telescope 层数**（含隐式 binder，T-C11）。
/// 内核 pp 打**完全应用**时有**两种形状**（`crates/kernel/src/pretty_printer.rs::
/// is_implicit_fun`）：
///
/// * **闭项**（能推断 binder 风格）⇒ pp **省略前导隐式实参** ⇒ 实参个数 = 元数 −
///   前导隐式个数（`Set.mem a A`、`Eq a b`）；
/// * **开项**（含松散变量 ⇒ `num_loose_bvars() > 0` 短路 ⇒ 不省略）⇒ 实参个数 =
///   元数（`Eq.{u} α a a`、`Set.mem α a A`）。
///
/// ⇒ 两种个数都是"完全应用"，[`fold_spine`] 都收；更少的是**部分应用**，不折。
/// 元数与隐式前缀的来源是 T-C11；这里只**消费**它们，好让折叠层能被单独测。
///
/// ⚠ **中间形状（省略了一部分前导隐式）故意不折**——实测它真实存在
/// （`Set.image β f S`：`α` 被省、`β` 留着；pp 的省略是**逐实参**判定的，
/// `pretty_printer.rs::is_implicit_fun` 对含松散变量的函数前缀会短路）。它与
/// **闭项过应用**（`Set.image f A x` = `(f '' A) x`，同一条脊多一个实参）**逐字
/// 同形** ⇒ 折它会显示错式子（`A '' x`）。按本模块第一条纪律（不猜）取"原样"：
/// 课程语料里这种声明显示点形式 `Set.image β f S`，而不是**错的**记法 ✓
/// （实测：全课程 135 条声明从"错记法"退回点形式，见切片回报）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DisplayNotations {
    table: Vec<NotationDecl>,
    arity: HashMap<String, usize>,
    /// 「target 点名 → 签名的**前导隐式 binder 个数**」（0 = 没有前导隐式）。
    /// 与 `arity` 成对使用：完全应用的两种 pp 形状就是 `arity` 与
    /// `arity - implicit_prefix` 两个实参个数。
    implicit_prefix: HashMap<String, usize>,
}

impl DisplayNotations {
    /// **T2-B0（2026-10-09）**：把"**库层那一段**"与"**入口那一段**"两张表合并成一张 ——
    /// 结果必须与"按 `库层 ++ 入口` **一次性**建表"**逐位相同** ✓
    /// （判据 `crates/front/tests/t2b0_display_merge_parity.rs` ✓）。
    ///
    /// ## 为什么不是简单的 `table.extend`
    ///
    /// 两张表**各自**都带了内建记法（`display_notations_from_commands` 会把
    /// `builtin_notation_decls()` **前插** ✓）⇒ 直接拼会**重复**内建项 ✗ ⇒
    /// 折叠时同一个符号会匹配到**多条** ⇒ 行为可能与一次性建表分叉 ✗。
    /// ⇒ 规则：`self.table`（含内建）**原样** + `other.table` **跳过它的内建前缀** ✓。
    ///
    /// 元数表：`self` 打底、`other` **覆盖**（与"入口在后"的插入顺序一致 ✓）。
    /// **隐式前缀表同规矩**（两张表必须一起合并——只合并一张会让另一张退回默认值，
    /// 折叠判据随之分叉 ✗）。
    pub fn merged_with(&self, other: &Self) -> Self {
        let builtins = crate::notation::builtin_notation_decls().len();
        let mut table = self.table.clone();
        if other.table.len() >= builtins {
            table.extend_from_slice(&other.table[builtins..]);
        }
        let mut arity = self.arity.clone();
        for (name, n) in &other.arity {
            arity.insert(name.clone(), *n);
        }
        let mut implicit_prefix = self.implicit_prefix.clone();
        for (name, n) in &other.implicit_prefix {
            implicit_prefix.insert(name.clone(), *n);
        }
        Self {
            table,
            arity,
            implicit_prefix,
        }
    }

    /// **签名不变**（既有调用/测试不受影响）：前导隐式表默认为空 = 0。
    pub fn new(table: Vec<NotationDecl>, arity: HashMap<String, usize>) -> Self {
        Self {
            table,
            arity,
            implicit_prefix: HashMap::new(),
        }
    }

    /// 链式补上「前导隐式个数」表（[`Self::new`] 的签名保持不变的原因见那里）。
    pub fn with_implicit_prefix(mut self, implicit_prefix: HashMap<String, usize>) -> Self {
        self.implicit_prefix = implicit_prefix;
        self
    }

    pub fn is_empty(&self) -> bool {
        self.table.is_empty()
    }

    /// 从记法表 + 一个"按点名问元数"的回调建起来（T-C11 的接入形状）。
    pub fn from_table(table: Vec<NotationDecl>, arity_of: impl Fn(&str) -> Option<usize>) -> Self {
        let arity = table
            .iter()
            .filter_map(|decl| Some((decl.target.clone(), arity_of(&decl.target)?)))
            .collect();
        Self {
            table,
            arity,
            implicit_prefix: HashMap::new(),
        }
    }

    /// 这个 target 名下的记法声明，**按声明顺序**（重载取第一个就是取它）。
    fn decls_for<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a NotationDecl> + 'a {
        self.table.iter().filter(move |decl| decl.target == name)
    }
}

/// 把一段**给人看**的文本按记法折回去（设计 §3.3）。
///
/// 任何一步失败都**原样返回输入**：解析不了、文本含松散变量 `$N`（pp 对
/// "binder 在被打印项之外"的变量的写法，回读只能靠猜）、表里没有这条记法、
/// 元数对不上——全都原样返回。**不猜**。
pub fn print_back(text: &str, notations: &DisplayNotations) -> DisplayText {
    if notations.is_empty() {
        return DisplayText::new(text);
    }
    // F3：pp 把"binder 在被打印项之外"的变量渲染成 `$N`，前端只能靠
    // `scope_names` 猜回名字。那种文本的**结构**不可信，不折。
    if text.contains('$') {
        return DisplayText::new(text);
    }
    let once = fold_once(text, notations);
    // **第二趟（A1 收尾，2026-09-26）**：第一趟里被"外层重渲染"吞掉的 `->`。
    //
    // 为什么需要它：`fold_spine` / App 父节点那两条路会用
    // `proof::render_expr(&folded)` **重渲染整段**，而 `render_expr` 是**回读
    // 通道的输入**（判卷靠它）⇒ 它打的永远是 ASCII `->`（改它 = 改判定文本 ✗）。
    // 于是折出来的记法里若含箭头（`↔ (∀ (x : α), (f x) = (f y)) -> x = y` 的
    // `∀` 被 `↔` 那条脊重渲染），那一处箭头又变回 `->`。
    // 实测：全课程 273 条 `ty_text`/`val_text` 里第一趟之后还剩 **9** 条带 `->`，
    // 全是这个形状。
    //
    // 为什么**两趟就够**：第二趟的输入已经是**显示文本**——折过的记法重新解析
    // 回来还是记法节点（`fold_spine` 对 Notation 头返回 `None`）⇒ 第二趟只会
    // 补**窄 span** 的箭头编辑，不会再有"整段重渲染"把它吞掉。所以这里不写
    // 循环（有界两趟 = 可终止，且没有振荡的余地）。
    if once.contains("->") {
        let twice = fold_once(&once, notations);
        if twice != once {
            return DisplayText::new(twice);
        }
    }
    DisplayText::new(once)
}

/// 一趟折叠：解析 → 收集编辑 → 按 span 拼接。失败一律**原样返回**（绝不乱切）。
fn fold_once(text: &str, notations: &DisplayNotations) -> String {
    // F2/F4：pp 会折行；`@Eq.{u, v}` 的 head 是 `UniverseApp`。parser 都容忍，
    // 解析不了就原样返回。
    let Ok(ast) = crate::proof::parse_expr_text_with(text, &notations.table) else {
        return text.to_string();
    };
    // `parse_expr_text_with` 解析的是 `"#check " + text` ⇒ AST 的 span 比 `text`
    // 多**那个前缀**。偏移量**只有一个源**：前缀常量自己（`proof::CHECK_PREFIX`）。
    //
    // **2026-09-26 修（A1 的副发现）**：这里原来"反推"成
    // `ast.span().start.offset - lead`（`lead` = `text` 的前导空白），依据是
    // "第一个非空白字符的位置就是表达式该在的位置"。**那条依据是错的**：
    // parser 给「带括号的原子」的 span **不含括号**，所以整条表达式被括号包住时
    // （`(α -> β) -> γ`、`(A ∪ B) -> C`）根节点 span 从**括号里面**开始 ⇒ 反推
    // 出来的 `base` 比真前缀**大**，于是 `splice` 的每一次 `span - base` 都偏左，
    // 落到字符中间 ⇒ `text.get(..)` 返回 `None` ⇒ **整条文本一个字节都不折**
    // （实测：`(α -> β) -> γ` 折前折后一模一样）。这正是用户看到的"混合形态"
    // 里那批带括号根节点的来源之一。
    let base = crate::proof::CHECK_PREFIX.len();
    let mut edits: Vec<(Span, String)> = Vec::new();
    let _ = fold_collecting(ast, notations, &mut edits, text, base);
    if edits.is_empty() {
        return text.to_string();
    }
    match splice(text, base, edits) {
        Some(out) => out,
        // span 换算越界（解析器换了前缀形状之类）⇒ **原样返回**，绝不乱切。
        None => text.to_string(),
    }
}

/// **唯一接口（阶段 U / T-U2）**：记法转化的三个动作，**只在这里实现** ✓。
///
/// 设计：`docs/design/notation-display.md` ✓。**三步固定顺序**：
/// `render_expr`（AST → 点形式）→ `fold`（**点形式 → 记法**，真的转化 ✓）→
/// `runs`（给**已折过**的文本打分段标签 ✓，不转化 ✗）。
/// 任何调用点都不许重做其中任一步 ✗（守卫 `scripts/audit-notation-paths.py` ✓）。
impl DisplayNotations {
    /// 唯一转化入口：**文本 → 带记法的文本** ✓。
    pub fn fold(&self, text: &str) -> String {
        // **作用域缓存命中** ⇒ 直接返回 ✓（见 `with_fold_cache` 的注释与 parity 判据 ✓）。
        if let Some(hit) =
            FOLD_CACHE.with(|c| c.borrow().as_ref().and_then(|m| m.get(text).cloned()))
        {
            return hit;
        }
        let out = print_back(text, self).as_display_str().to_string();
        FOLD_CACHE.with(|c| {
            if let Some(m) = c.borrow_mut().as_mut() {
                m.insert(text.to_string(), out.clone());
            }
        });
        out
    }

    /// 唯一转化入口（AST 版 ✓）：渲染**之后必过折叠** ✓。
    pub fn render(&self, expr: &Expr) -> String {
        self.fold(&crate::proof::render_expr(expr))
    }

    /// 唯一分段入口 ✓：给**已经折过**的文本打标签。
    ///
    /// ⚠ 入参 `folded` **必须是 `fold`/`render` 的产物** ✓ —— 传点形式文本进来就会
    /// 得到点形式分段（2026-09-25 的用户报告正是这么来的 ✗：`query::runs` 拿的是
    /// 未折过的文本 ✓）。
    ///
    /// **`self` 在这一步不参与**（T-N4，2026-09-26 ✓）：分段只看文本 + 名字表
    /// （`decls`/`binders`/`notations` 都是调用方按名字传的），不读记法表 ⇒
    /// 拿不到表的消费者（`query::runs`）用 `DisplayNotations::default()` 调它是
    /// **接口约定**，不是绕过 ✓。保留 `&self` 是为了"分段也只有这一个入口"——
    /// 它是 [`Self::fold`] / [`Self::render`] 的同族操作，三者必须同处一地 ✓。
    pub fn runs(
        &self,
        folded: &str,
        decls: &[(String, crate::semantic::SemanticKind)],
        binders: &[String],
        notations: &[String],
    ) -> Vec<crate::semantic::Run> {
        crate::semantic::tag_runs_with_notations(folded, decls, binders, notations)
    }
}

/// **`text` 与 `runs` 是同一次转化的两个投影** ✓（设计 `notation-display.md` §2 ✓）。
///
/// 不变量：`runs` 的文本拼接**逐字节等于** `text` ✓ —— 它把"① 与 ② 各算各的"
/// 这种分叉在**运行时**暴露出来 ✓（T-U5 的接缝守卫 ✓）。
pub struct Rendered {
    pub text: String,
    pub runs: Vec<crate::semantic::Run>,
}

impl Rendered {
    /// 不变量成立吗 ✓（不成立就说明有人绕过了唯一接口 ✗）。
    pub fn is_consistent(&self) -> bool {
        let mut joined = String::new();
        for run in &self.runs {
            joined.push_str(&run.text);
        }
        joined == self.text
    }
}

/// 把折出来的记法**拼回原文本**：只替换折过的那几段，其余**逐字节保留**。
///
/// **为什么不是"重渲染整棵 AST"**（`render_expr(&folded)`）：那样会把折过之外
/// 的东西也一起改样——实测最刺眼的两条是 `forall (a b : T), …` 被**拆成箭头链**、
/// `Type 0` 被重排成 `Sort 1`（`render_expr` 是回读通道的输入，它必须那样写）。
/// 用户要的是"记法"，不是"整句话换个写法"。
///
/// **span 是可靠的**：折叠**保留 span**（记法节点取被折那段的 span，操作数各自
/// 保留自己的），而它们指向的就是传进来的 `text` ⇒ 可以按 span 原地替换。
/// 从**右往左**替换，前面的偏移才不会被破坏。
fn splice(text: &str, base: usize, mut edits: Vec<(Span, String)>) -> Option<String> {
    // span 换算回 `text` 的下标；任何一处越界/不在字符边界就放弃（返回 `None`）。
    let mut ranges: Vec<(std::ops::Range<usize>, String)> = Vec::new();
    for (span, rendered) in edits.drain(..) {
        let start = span.start.offset.checked_sub(base)?;
        let end = span.end.offset.checked_sub(base)?;
        if start > end || end > text.len() {
            return None;
        }
        if !text.is_char_boundary(start) || !text.is_char_boundary(end) {
            return None;
        }
        // **括号配平**：解析器给「带括号的原子」的 span **不含那对括号**
        // （实测 `(Set.union α A B)` 的 span 只有 `Set.union α A B`）⇒
        // 外层应用节点的 span 会在最后一个 `)` **之前**结束（`Set.mem α a
        // (Set.union α A B)` 的 span 少一个 `)`）。这里把范围补齐到**括号配平**：
        // 少 `)` 就向右吃 `)`，多 `)` 就向左吃 `(`。两个方向都实测过。
        let (mut start, mut end) = (start, end);
        let slice = &text[start..end];
        let mut open = slice.matches('(').count();
        let mut close = slice.matches(')').count();
        while close > open && start > 0 && text.as_bytes()[start - 1] == b'(' {
            start -= 1;
            open += 1;
        }
        while open > close && end < text.len() && text.as_bytes()[end] == b')' {
            end += 1;
            close += 1;
        }
        if open != close {
            return None; // 配不平 ⇒ 不切（宁可原样，也不切坏）
        }
        ranges.push((start..end, rendered));
    }
    // 按 `(起点, 终点倒序)` 排：**起点相同时长的在前**。
    // 这一条是必须的——折出来的记法节点**取被折那段的 span**，而"折过的子树被
    // 应用"时上提到脊根的那一处**起点与它相同**（`Set.union α (Set.union α A B) C`
    // 的两处替换都从 0 开始）。只按起点稳定排序会让**内层排在前面**，随后"最外层"
    // 规则反而把真正的**外层丢掉**（实测：`(A ∪ B) ∪ C` 变成 `Set.union α (A ∪ B) C`）。
    ranges.sort_by_key(|(range, _)| (range.start, std::cmp::Reverse(range.end)));
    // 只保留**最外层**的替换：内层的那些已经在它渲染出来的文本里了
    // （外层是 `render_expr(折好的 AST)`，它本身带着内层的记法）。
    let mut top: Vec<(std::ops::Range<usize>, String)> = Vec::new();
    for (range, rendered) in ranges {
        let nested = top.last().is_some_and(|(outer, _)| range.start < outer.end);
        if !nested {
            top.push((range, rendered));
        }
    }
    let mut out = text.to_string();
    for (range, rendered) in top.into_iter().rev() {
        out.replace_range(range, &rendered);
    }
    Some(out)
}

/// 自底向上折：先把子项折好，父项才有机会看到已经折好的操作数；每一处折叠都
/// 记进 `edits`（`(被替换那段的 span, 换上去的文本)`）。内外都记；[`splice`]
/// 只取最外层的那些。
fn fold_collecting(
    expr: Expr,
    dn: &DisplayNotations,
    edits: &mut Vec<(Span, String)>,
    src: &str,
    base: usize,
) -> Expr {
    fold_collecting_inner(expr, dn, edits, false, false, src, base).0
}

/// 同 [`fold_collecting`]，另外回报"这棵子树变了没有"。
///
/// **为什么需要"变了没有"**：折过的子树如果**被应用**（它的父节点还是 `App`），
/// 就地替换会拼出**重解析成另一个 AST** 的文本——
/// `(Set.mem α a A) B` 的 `Set.mem α a A` 是完整的三元应用，就地换成 `a ∈ A`
/// 就得到 `a ∈ A B`，重新解析是 `Set.mem α a (A B)`（**换了个意思**）。
/// 规则：那种情况下把替换范围**上提到这条应用脊的根**，整条脊交给
/// `render_expr` 渲染——括号归它管，它本来就是干这个的。
///
/// 只上提到 `App`：`Set.mem α a A -> P` 的父节点是 `Arrow`（不是应用），
/// 就地换是安全的（`∈` 比 `->` 紧，重解析一致）⇒ 保留原文的其它部分。
///
/// **`in_spine` 是性能开关**（不是正确性开关）：只让**最外层**那条应用脊记一次。
/// 不传它的话，折点之上的**每一层** `App` 祖先都会 `render_expr` 一遍整棵子树
/// ——实测 `did_open` 因此退化 **+18~22%**（`lsp-course` 三档全中），改回 O(1) 次。
/// 记多次也不影响结果（[`splice`] 只取最外层），纯粹是白烧。
/// **语义 = "这个节点的父节点是 `App`"**（`fun` 与 `arg` 两个子节点都算——它们
/// 都在同一条脊上）。
///
/// **`is_fun_part`（切片：折坏部分应用）**：这个节点是不是某条应用脊的 `fun`
/// 子节点（即"它是同一条脊的下一层"）。
/// * `fun` 子节点 ⇒ `true`：它**属于父节点那条脊**，**不折**（[`fold_spine`] 在
///   调用点被跳过）——折叠由**脊根**负责。少了这一条，`Eq.{u} α a a` 的内层
///   `Eq.{u} α a` 会被当成完全应用折成 `α = a`，父节点再整脊重渲染 ⇒
///   `(α = a) a`（用户 2026-10-10 报的那一形）。
/// * `arg` 子节点 / 其它形状的子节点 ⇒ `false`：它**是另一棵子树**的根（或不在
///   脊上），自己的脊自己折 ⇒ `Set.subset (Set.singleton α a) A` 的内层照折 ✓。
fn fold_collecting_inner(
    expr: Expr,
    dn: &DisplayNotations,
    edits: &mut Vec<(Span, String)>,
    in_spine: bool,
    is_fun_part: bool,
    src: &str,
    base: usize,
) -> (Expr, bool) {
    let is_app = matches!(expr, Expr::App { .. });
    let mut child_changed = false;
    // **`App` 的两个子节点必须分开处理**：`map_children_with` 一视同仁，会把
    // `fun`（同一条脊的下一层）也当成"一棵独立子树"递归 ⇒ 内层被折 ✗。
    let expr = match expr {
        Expr::App {
            fun,
            arg,
            explicit_spine,
            span,
        } => Expr::App {
            fun: Box::new({
                let (out, changed) =
                    fold_collecting_inner(*fun, dn, edits, is_app, true, src, base);
                child_changed |= changed;
                out
            }),
            arg: Box::new({
                let (out, changed) =
                    fold_collecting_inner(*arg, dn, edits, is_app, false, src, base);
                child_changed |= changed;
                out
            }),
            explicit_spine,
            span,
        },
        // 其它形状的子节点一律 `is_fun_part = false`（它们不在应用脊上）。
        other => map_children_with(other, &mut |e| {
            let (out, changed) = fold_collecting_inner(e, dn, edits, is_app, false, src, base);
            child_changed |= changed;
            out
        }),
    };
    // **`forall` 关键字形状 → `∀`**（T-D51 第二步 / 缺口 G-38）。
    //
    // 为什么不能靠"查表"：`∀` 是 **parser 关键字**（`parse_forall`），**不在**
    // `BUILTIN_NOTATIONS` 里；而声明栏那个 `forall` 是**内核 pp 打的 telescope**
    // （`forall (α : Type 0) (a : α), …`），回读时落成 `Expr::Forall`——头不是
    // `Ident`，`fold_spine` 的"spine + 名字查表"那条路够不着。
    //
    // **只换关键字那 6 个字节**（`forall` → `∀`），其余**逐字节不动**。
    // 这一条是踩出来的：第一版把整个 `Forall` 节点重渲染成 `∀ binders, body`，
    // 结果 binder 分组被拆开（`(A B : Set α)` → `(A : Set α) (B : Set α)`）、
    // `Type 0` 变成 `Sort 1`——**信息反而失真**，正是 `only_the_folded_spans_change`
    // 那条测试在守的东西（线 C 的纪律：只有折过的 span 变）。
    // **只有源文本真的写着 `forall` 才折**（T-D51，实测踩到的坑）：parser 把
    // `(x : α) -> …` 也解析成 `Expr::Forall`（匿名 binder）⇒ 只看 AST 会在
    // `(x : α` 那 6 个字节上写 `∀`，括号配不平 ⇒ `splice` **整体放弃**、
    // 展示副本退回**完全不折**（`by_step_display_is_folded_but_the_judge_input_is_not`
    // 就是这么红的）。判据必须是**源文本**，不是 AST 形状。
    if let Expr::Forall {
        binders,
        body,
        span,
    } = &expr
    {
        let writes_forall = span
            .start
            .offset
            .checked_sub(base)
            .and_then(|start| src.get(start..))
            .is_some_and(|rest| rest.starts_with("forall"));
        if !writes_forall {
            // 匿名 binder 的箭头写法（`(x : α) -> …`）：**不动它**，让子节点的
            // 编辑照常生效（`return` 出去会把它们一起吞掉）。
            // **A1**：但那个 `->` 本身要折成 `→`（`forall` 分支折的是关键字，
            // 这条折的是 binder 组之后的箭头）。
            if let (Some(last), body) = (binders.last(), body) {
                if let Some(tok) =
                    arrow_token_between(src, base, last.span.end.offset, body.span().start.offset)
                {
                    edits.push((tok, "→".to_string()));
                }
            }
            return (expr, child_changed);
        }
        const KEYWORD: &str = "forall";
        let keyword = Span::new(
            span.start,
            crate::span::Pos {
                offset: span.start.offset + KEYWORD.len(),
                ..span.start
            },
        );
        // ① **关键字级编辑**：顶层（没有外层编辑盖住它）时逐字节保真。
        edits.push((keyword, "∀".to_string()));
        // ② 同时返回**折好的记法节点**：外层若因为"子节点变了"而重渲染
        //    （App 父节点那条路），用的是 AST ⇒ 没有这一条就会被画回
        //    `(x : α) -> …`（**实测踩到**：展示副本整个退回点名形式）。
        //    两条编辑都在时，`splice` 的排序让**外层**赢——外层本来就覆盖
        //    更全，正是我们要的。
        let folded = Expr::Notation {
            symbol: "∀".to_string(),
            // `∀` 背后没有常量（它就是 binder 语法本身）⇒ target 只给读的人看。
            target: "forall".to_string(),
            assoc: NotationAssoc::Binder,
            lhs: None,
            rhs: Some(Box::new(Expr::Lambda {
                binders: binders.clone(),
                body: body.clone(),
                span: *span,
            })),
            alternatives: Vec::new(),
            span: *span,
            symbol_span: *span,
        };
        return (folded, true);
    }
    if let Expr::Arrow {
        domain,
        codomain,
        span: _,
    } = &expr
    {
        // **A1（2026-09-26 用户报告第 1 条）**：内核 pp 打的是 ASCII `->`，
        // 而源里的 `→` 与它同义。不折它，Infoview 顶部就是**混合形态**
        // （`∀ (α β γ : Type 0), (α -> β) -> …`）——`∀` 折了、`->` 没折。
        // 折法是**只换 `->` 那两个字节**（与 `forall` 关键字同一条纪律：只有折过
        // 的 span 变，别的逐字节不动）。`->` 自己的 span 不在 AST 里，所以按
        // 「domain 结束 .. codomain 开始」这段空隙去找它。
        if let Some(tok) = arrow_token_between(
            src,
            base,
            domain.span().end.offset,
            codomain.span().start.offset,
        ) {
            edits.push((tok, "→".to_string()));
        }
        return (expr, child_changed);
    }
    // **脊根才折**（`is_fun_part` 的节点是同一条脊的下一层 ⇒ 它是**部分应用**，
    // 折它就会拼出 `(α = a) a`）。见 [`fold_collecting_inner`] 的文档。
    if !is_fun_part {
        if let Some(folded) = fold_spine(&expr, dn) {
            edits.push((folded.span(), crate::proof::render_expr(&folded)));
            return (folded, true);
        }
    }
    if child_changed && is_app && !in_spine {
        edits.push((expr.span(), crate::proof::render_expr(&expr)));
        return (expr, true);
    }
    (expr, child_changed)
}

/// 两个子节点之间的 `->` 记号（A1）。找不到（源里写的是 `→`，或者这一段根本
/// 不是箭头——`∀ x, p` 的 `,`）⇒ `None`，**不猜、不动**。
///
/// `Expr::Arrow` 的 span 覆盖 `domain .. codomain`，`Expr::Forall`（`(x : α) -> β`
/// 那条匿名 binder 形状）的 span 从 binder 组起、body 结束——两处的 `->` 都落在
/// 「上一个子节点结束 .. 下一个子节点开始」这段空隙里。
fn arrow_token_between(src: &str, base: usize, from: usize, to: usize) -> Option<Span> {
    let start = from.checked_sub(base)?;
    let end = to.checked_sub(base)?;
    if start > end || end > src.len() {
        return None;
    }
    let at = src.get(start..end)?.find("->")?;
    let offset = from + at;
    Some(Span::new(
        crate::span::Pos {
            offset,
            ..crate::span::Pos::default()
        },
        crate::span::Pos {
            offset: offset + 2,
            ..crate::span::Pos::default()
        },
    ))
}

/// 这一层是不是一条记法实例？是就换成 [`Expr::Notation`]，否则 `None`。
///
/// **第一刀只做二元 infix 族**（`Infix`/`Infixl`/`Infixr`）——一元前缀/后缀与
/// binder 记法的折叠留给后续环节；它们的 `arity` 与操作数位不同（一元 1 个、
/// binder 2 个且第 2 个是 lambda），一起做会把这一刀撑大。
///
/// **两种完全应用形状都收**（切片：折坏部分应用）：`k == full`（开项，pp 不省略
/// 前导隐式）与 `k == full - implicit_prefix`（闭项，pp 省略前导隐式）。
/// **操作数永远取最后 `operand_count` 个实参**——两种形状下前导实参都是类型/命题
/// 参数（`∈` 的 `α`），要丢掉；闭项形状下"最后 N 个"就是全部实参。
///
/// ⚠ **只有极大应用脊的根走到这里**（[`fold_collecting_inner`] 的 `is_fun_part`）：
/// 内层节点（`Eq α a` 在 `Eq α a a` 里）是**部分应用**，折它会拼出
/// `(α = a) a`（用户 2026-10-10 报的那一形）。
fn fold_spine(expr: &Expr, dn: &DisplayNotations) -> Option<Expr> {
    let (head, args) = crate::spine::spine_of(expr);
    let name = head_name(head)?;
    // **集合字面量（A2，2026-09-26 用户报告第 2 条）**：`{a}` / `{a, b}` 是
    // **内建语法**（`ast::Expr::SetLiteral`，展开成 `Set.singleton α a` /
    // `Set.pair α a b`）——它**不是记法声明**，`dn.arity` 里查不到它，所以照
    // `forall` 关键字那条先例单独认。判据与记法**同一条**：只有**完全应用**
    // 才是那个形状（两种 pp 形状都算，见 [`fold_set_literal`]）。
    if let Some(folded) = fold_set_literal(name, &args, expr.span()) {
        return Some(folded);
    }
    // **元数 = 全望远镜层数**（含隐式 binder）；前导隐式个数单独查。
    let full = *dn.arity.get(name)?;
    let prefix = dn.implicit_prefix.get(name).copied().unwrap_or(0);
    let k = args.len();
    // 重载：同一 target 声明了两个符号 ⇒ **取声明顺序第一个**（写进文档 + 测试）。
    let decl = dn.decls_for(name).next()?;
    // **每一种记法都要折**（T-D51 / 缺口 G-38）。以前这里只放行 infix 族
    // （注释写着"留给后续环节"），于是声明栏里 `𝒫 A` / `Aᶜ` / `∃ x, p` / `∅`
    // 全都保持点名形式——用户看到的"丢了一批符号"。
    //
    // 每种记法的**操作数位**不同（这是当初只做 infix 的原因），照 parser 的
    // 构造形状来（`notation_node` 的调用点）：
    //   Infix 族 2 个（左、右）· Prefix/Binder 1 个（在**右**）·
    //   Postfix 1 个（在**左**）· Nullary 0 个。
    let operand_count = match decl.assoc {
        NotationAssoc::Infix | NotationAssoc::Infixl | NotationAssoc::Infixr => 2,
        NotationAssoc::Prefix | NotationAssoc::Postfix | NotationAssoc::Binder => 1,
        NotationAssoc::Nullary => 0,
    };
    // §3.2 硬规则：**部分应用不是记法实例**。两种完全应用形状之外的个数一律不折
    // （`k < operand_count` 先挡掉——它同时保证下面那句不越界）。
    if k < operand_count {
        return None;
    }
    if !(k == full || k == full.saturating_sub(prefix)) {
        return None;
    }
    // 前导实参（`Set.mem α a A` 里的 `α`）**丢掉**：它们是展开时补上的隐式
    // 类型参数，源里本来就不写。闭项形状（`Set.mem a A`）下前导实参已经被 pp
    // 省掉，"最后 N 个"就是全部实参。
    let operands = &args[k - operand_count..];
    let (lhs, rhs) = match (decl.assoc, operands) {
        (NotationAssoc::Infix | NotationAssoc::Infixl | NotationAssoc::Infixr, [a, b]) => {
            (Some(Box::new((*a).clone())), Some(Box::new((*b).clone())))
        }
        (NotationAssoc::Prefix | NotationAssoc::Binder, [only]) => {
            (None, Some(Box::new((*only).clone())))
        }
        (NotationAssoc::Postfix, [only]) => (Some(Box::new((*only).clone())), None),
        (NotationAssoc::Nullary, []) => (None, None),
        _ => return None,
    };
    Some(Expr::Notation {
        symbol: decl.symbol.clone(),
        target: decl.target.clone(),
        assoc: decl.assoc,
        lhs,
        rhs,
        // 折叠出来的是**唯一的**写法（head 名字就是判据），没有候选列表。
        alternatives: Vec::new(),
        span: expr.span(),
        // 折出来的节点是**渲染产物**，没有源里的符号 token ⇒ 退化成节点 span。
        symbol_span: expr.span(),
    })
}

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
fn fold_set_literal(name: &str, args: &[&Expr], span: Span) -> Option<Expr> {
    let k = args.len();
    let elements: Vec<Expr> = match name {
        "Set.singleton" if k == 2 || k == 1 => vec![args[k - 1].clone()],
        "Set.pair" if k == 3 || k == 2 => vec![args[k - 2].clone(), args[k - 1].clone()],
        _ => return None,
    };
    Some(Expr::SetLiteral { elements, span })
}

/// spine 的头是不是一个可以当记法目标的名字（`Ident` 或 `UniverseApp`）。
fn head_name(head: &Expr) -> Option<&str> {
    match head {
        Expr::Ident { name, .. } | Expr::UniverseApp { name, .. } => Some(name.as_str()),
        _ => None,
    }
}

/// 递归到所有子项。**列全每一个变体**——漏一个位置的后果是"那里不折"
/// （安全但会让同一份文本里折一半），比"折错"好，但不该有。
///
/// 参数化成一个 `f`（而不是把折叠逻辑写进来）是为了让"折"与"折 + 记下替换"
/// **共用同一份结构知识**：两份手写的 16 变体匹配迟早会漂。
pub(crate) fn map_children_with(expr: Expr, f: &mut impl FnMut(Expr) -> Expr) -> Expr {
    match expr {
        Expr::App {
            fun,
            arg,
            explicit_spine,
            span,
        } => Expr::App {
            fun: Box::new(f(*fun)),
            arg: Box::new(f(*arg)),
            explicit_spine,
            span,
        },
        Expr::Lambda {
            binders,
            body,
            span,
        } => Expr::Lambda {
            binders: map_binders_with(binders, f),
            body: Box::new(f(*body)),
            span,
        },
        Expr::Forall {
            binders,
            body,
            span,
        } => Expr::Forall {
            binders: map_binders_with(binders, f),
            body: Box::new(f(*body)),
            span,
        },
        Expr::Arrow {
            domain,
            codomain,
            span,
        } => Expr::Arrow {
            domain: Box::new(f(*domain)),
            codomain: Box::new(f(*codomain)),
            span,
        },
        Expr::Plus { lhs, rhs, span } => Expr::Plus {
            lhs: Box::new(f(*lhs)),
            rhs: Box::new(f(*rhs)),
            span,
        },
        Expr::Let {
            binder,
            val,
            body,
            span,
        } => Expr::Let {
            binder: map_binder_with(binder, f),
            val: Box::new(f(*val)),
            body: Box::new(f(*body)),
            span,
        },
        Expr::Match {
            scrutinee,
            arms,
            span,
        } => Expr::Match {
            scrutinee: Box::new(f(*scrutinee)),
            arms: arms
                .into_iter()
                .map(|arm| MatchArm {
                    pattern: arm.pattern,
                    guard: arm.guard.map(&mut *f),
                    body: f(arm.body),
                    span: arm.span,
                })
                .collect(),
            span,
        },
        // 已经是一条记法：只折它的操作数（幂等——折叠层不该把折好的再折一次）。
        Expr::Notation {
            symbol,
            target,
            assoc,
            lhs,
            rhs,
            alternatives,
            span,
            symbol_span,
        } => Expr::Notation {
            symbol,
            target,
            assoc,
            lhs: lhs.map(|e| Box::new(f(*e))),
            rhs: rhs.map(|e| Box::new(f(*e))),
            alternatives,
            span,
            symbol_span,
        },
        Expr::SetLiteral { elements, span } => Expr::SetLiteral {
            elements: elements.into_iter().map(&mut *f).collect(),
            span,
        },
        Expr::AnonCtor { elements, span } => Expr::AnonCtor {
            elements: elements.into_iter().map(&mut *f).collect(),
            span,
        },
        // 叶子（没有子项）与 `By`（tactic 脚本，不在这里展开）。
        leaf => leaf,
    }
}

fn map_binders_with(binders: Vec<Binder>, f: &mut impl FnMut(Expr) -> Expr) -> Vec<Binder> {
    binders.into_iter().map(|b| map_binder_with(b, f)).collect()
}

fn map_binder_with(binder: Binder, f: &mut impl FnMut(Expr) -> Expr) -> Binder {
    Binder {
        name: binder.name,
        ty: binder.ty.map(|t| Box::new(f(*t))),
        style: binder.style,
        span: binder.span,
    }
}

// ---- 元数的来源（T-C11）----------------------------------------------------
//
// 设计 §3.2 的硬规则要两个数：**目标声明的 telescope 层数**（= 开项完全应用时
// `spine.len()`）与它的**前导隐式 binder 个数**。有了它们才能判断"这是记法实例"
// 还是"部分应用"：
// `Set.mem α a A`（3 = 3，开项）⇒ `a ∈ A`；`Set.mem a A`（2 = 3 − 1，闭项
// ——pp 省掉了隐式 `α`）⇒ `a ∈ A`；`Set.mem a`（1 ≠ 3 且 ≠ 2）⇒ 原样。
//
// **口径先对齐**（三处容易混）：
//   * **telescope**（= 本模块的 `arity`）= 声明类型上剥出来的 binder 总数
//     （`def Set.mem {α : Type} (a : α) (A : Set α) : Prop` ⇒ **3**）。
//     它对应**开项**完全应用的 `spine.len()`（pp 不省略隐式）。
//   * **前导隐式个数**（= `implicit_prefix`）= 签名开头**连续**的隐式 binder 数
//     （`{α : Type}` ⇒ **1**）。闭项完全应用的 `spine.len()` = telescope − 它
//     （pp 把前导隐式实参省掉了）。
//   * **操作数个数** = 记法自己写出来的位置（二元 infix ⇒ **2**）。
//   三者之差 = **前导参数**（`∈` 的 `α`），折叠时丢掉。
//
// 来源：**源级签名**，从闭包各模块的源文本（`ModuleReport.source`）与 prelude
// 源码里数出来。找不到 ⇒ `None` ⇒ 折叠层**原样返回**（不猜）。

/// 一个声明类型的 telescope 层数（binder 总数，含隐式）。
///
/// **`def f (a : T) (b : T) : U` 在 AST 里是「一个 `Forall` 带两个 binder」**
/// （实测），所以这里数的是 **binder**，不是 `Forall`/`Arrow` 节点的个数。
pub fn telescope_len(ty: &Expr) -> usize {
    let mut count = 0;
    let mut cur = ty;
    loop {
        match cur {
            Expr::Forall { binders, body, .. } => {
                count += binders.len();
                cur = body;
            }
            Expr::Arrow { codomain, .. } => {
                count += 1;
                cur = codomain;
            }
            _ => return count,
        }
    }
}

/// 折叠层**曾经的**元数口径：**显式 binder 的个数**。
///
/// ⚠ **不再是折叠用的元数**（切片：折坏部分应用，2026-10-10）：pp 对**开项**
/// **不省略**隐式实参 ⇒ 只用显式个数会把开项完全应用判成"部分应用"、并让内层
/// 部分应用被当成完全应用折掉（`Eq.{u} α a a` → `(α = a) a`）。折叠现在用
/// [`telescope_len`] + [`implicit_prefixes_in_commands`] 两个数一起判
/// （[`fold_spine`]）。这里保留这个纯函数：它仍是"显式实参层数"的定义，
/// 供诊断/对照使用（`crates/front/src/compile/elab.rs::explicit_arity` 是它在
/// 判定路径上的同名兄弟）。
pub fn explicit_arity(ty: &Expr) -> usize {
    let mut count = 0;
    let mut cur = ty;
    loop {
        match cur {
            Expr::Forall { binders, body, .. } => {
                count += binders
                    .iter()
                    .filter(|b| b.style == crate::ast::BinderKind::Explicit)
                    .count();
                cur = body;
            }
            // `A -> B` 的域是**显式**的（匿名 binder）。
            Expr::Arrow { codomain, .. } => {
                count += 1;
                cur = codomain;
            }
            _ => return count,
        }
    }
}

/// 从若干段**源文本**里收出「声明的全名 → telescope 层数」。
pub fn arities_in_sources(sources: &[&str]) -> HashMap<String, usize> {
    let mut out = HashMap::new();
    for src in sources {
        let Ok(file) = crate::parse(src) else {
            continue;
        };
        out.extend(arities_in_commands(&file.commands));
    }
    out
}

/// 同 [`arities_in_sources`]，但吃**已经解析好的**命令序列。
///
/// 编译出口（`finish_pass`）手上就是 `units[*].file.commands`——它**已经**为别
/// 的事解析过了，这里不必再解析一遍。
///
/// **名字直接用 parser 给的**：它**已经**按 `namespace`/`end` 限定好了
/// （实测 `namespace Foo` 里的 `def bar` 解析成 `name: "Foo.bar"`），与
/// `NotationDecl.target` 存的全名同一口径。自己再拼一次会得到 `Foo.Foo.bar`（踩过）。
///
/// **口径 = 全 telescope 层数**（含隐式 binder）：pp 打**开项**（含松散变量）时
/// **不省略**隐式实参 ⇒ 开项完全应用就是这个数。闭项形状（pp 省掉前导隐式）由
/// [`implicit_prefixes_in_commands`] 那张表配合 [`fold_spine`] 一起认。
pub fn arities_in_commands(commands: &[crate::ast::Command]) -> HashMap<String, usize> {
    let mut out = HashMap::new();
    for command in commands {
        match command {
            crate::ast::Command::Def { name, ty, .. }
            | crate::ast::Command::Theorem { name, ty, .. }
            | crate::ast::Command::Axiom { name, ty, .. } => {
                out.insert(name.clone(), telescope_len(ty));
            }
            // 归纳类型：`params`（`inductive And (a b : Prop)`）+ 类型上的 binder。
            crate::ast::Command::InductiveBlock {
                name, params, ty, ..
            } => {
                out.insert(name.clone(), params.len() + telescope_len(ty));
            }
            _ => {}
        }
    }
    out
}

/// 一串 binder 里**开头连续**的隐式 binder 个数；碰到第一个显式 binder 就停。
fn leading_implicit_binders(binders: &[Binder]) -> (usize, bool) {
    let mut n = 0;
    for b in binders {
        if b.style == crate::ast::BinderKind::Implicit {
            n += 1;
        } else {
            // `false` = 还没走完这串（前导隐式在更前面就断了）。
            return (n, false);
        }
    }
    (n, true)
}

/// 从若干段**源文本**里收出「声明的全名 → 前导隐式 binder 个数」。
pub fn implicit_prefixes_in_sources(sources: &[&str]) -> HashMap<String, usize> {
    let mut out = HashMap::new();
    for src in sources {
        let Ok(file) = crate::parse(src) else {
            continue;
        };
        out.extend(implicit_prefixes_in_commands(&file.commands));
    }
    out
}

/// 同 [`implicit_prefixes_in_sources`]，但吃**已经解析好的**命令序列。
///
/// **口径与 `elab.rs::leading_implicit_prefix` 一致**（同一份源级 AST 走查，
/// 零内核调用）：`{u} {α : Sort u} (a : α) …` ⇒ `2`；前导隐式一旦被显式 binder
/// 打断就停（`(a : α) {b : β} …` ⇒ `0`——pp 不会跨过显式实参省略后面的隐式）。
///
/// 归纳块：先数 `params` 的前导隐式；**全 `params` 都是隐式**时再接着数 `ty` 的
/// （与 `params.len() + telescope_len(ty)` 那条全层数口径对齐）。
pub fn implicit_prefixes_in_commands(commands: &[crate::ast::Command]) -> HashMap<String, usize> {
    let mut out = HashMap::new();
    for command in commands {
        match command {
            crate::ast::Command::Def { name, ty, .. }
            | crate::ast::Command::Theorem { name, ty, .. }
            | crate::ast::Command::Axiom { name, ty, .. } => {
                out.insert(name.clone(), leading_implicit_prefix(ty));
            }
            crate::ast::Command::InductiveBlock {
                name, params, ty, ..
            } => {
                let (n, all_implicit) = leading_implicit_binders(params);
                let n = if all_implicit {
                    n + leading_implicit_prefix(ty)
                } else {
                    n
                };
                out.insert(name.clone(), n);
            }
            _ => {}
        }
    }
    out
}

/// 声明类型望远镜的**前导隐式 binder 个数**（与
/// `crate::compile::elab::leading_implicit_prefix` 同口径；那边是 `pub(crate)`，
/// 这里要 `pub` 给 front 之外的建表路径用）。
///
/// `peel_pi` 把 `BinderKind` 丢了，所以这里自己走 `Expr::Forall`。
pub fn leading_implicit_prefix(ty: &Expr) -> usize {
    let mut n = 0;
    let mut cur = ty;
    loop {
        match cur {
            Expr::Forall { binders, body, .. } => {
                let (count, all_implicit) = leading_implicit_binders(binders);
                n += count;
                if !all_implicit {
                    return n;
                }
                cur = body;
            }
            _ => return n,
        }
    }
}

/// prelude 段的两张表（元数 + 前导隐式个数），**parse 一次就缓存**。
///
/// 为什么合并成一次：两张表来自**同一份** prelude 源文本，而它在**每次编译的
/// 出口**都要用（每个声明一次）。分别缓存会白解析一遍（源是常量，但解析不是）。
fn prelude_tables() -> &'static (HashMap<String, usize>, HashMap<String, usize>) {
    static CACHE: std::sync::OnceLock<(HashMap<String, usize>, HashMap<String, usize>)> =
        std::sync::OnceLock::new();
    CACHE.get_or_init(|| {
        let sources = [
            crate::compile::prelude_eq_src(),
            crate::compile::prelude_l1_src(),
        ];
        (
            arities_in_sources(&sources),
            implicit_prefixes_in_sources(&sources),
        )
    })
}

/// prelude 里那些记法目标（`And` / `Or` / `Not` / `Iff` / `Eq`）的 telescope 层数。
///
/// **parse 一次就缓存**（与隐式前缀表共用同一次解析）：它在**每次编译的出口**
/// 都要用（每个声明一次），而 prelude 源码是常量。线 C 的四个生产者都会经过它。
pub fn prelude_arities() -> &'static HashMap<String, usize> {
    &prelude_tables().0
}

/// prelude 里那些记法目标的**前导隐式 binder 个数**（`Eq` 的 `{α : Sort u}` ⇒ 1）。
pub fn prelude_implicit_prefixes() -> &'static HashMap<String, usize> {
    &prelude_tables().1
}

/// 把 prelude 的元数并进 `extra`（`extra` 覆盖同名项——文件自己的声明优先）。
pub fn arities_with_prelude_from(extra: HashMap<String, usize>) -> HashMap<String, usize> {
    let mut out = prelude_arities().clone();
    out.extend(extra);
    out
}

/// 把 prelude 的源文本也算进来（`And` / `Or` / `Not` / `Iff` / `Eq` / `Exists`
/// 这些记法目标住在那里）。线 C 的四个生产者都会经过它。
pub fn arities_with_prelude(sources: &[&str]) -> HashMap<String, usize> {
    arities_with_prelude_from(arities_in_sources(sources))
}

/// 把 prelude 的**前导隐式个数**并进 `extra`（`extra` 覆盖同名项）。
pub fn implicit_prefixes_with_prelude_from(
    extra: HashMap<String, usize>,
) -> HashMap<String, usize> {
    let mut out = prelude_implicit_prefixes().clone();
    out.extend(extra);
    out
}

/// 同 [`arities_with_prelude`]，收的是**前导隐式个数**表。
pub fn implicit_prefixes_with_prelude(sources: &[&str]) -> HashMap<String, usize> {
    implicit_prefixes_with_prelude_from(implicit_prefixes_in_sources(sources))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notation::notation_table;

    /// 夹具：把一段**记法声明**源码解析成表，再按给定元数建 [`DisplayNotations`]。
    ///
    /// 元数在 T-C11 之前由测试**显式给**——折叠层只消费它，好被单独测。
    /// **前导隐式表默认空（= 0）**：`arities` 给的就是"两种 pp 形状都收"的那两个数
    /// 之一（见 [`notations_with_prefixes`] 要 prefix 的场合）。
    fn notations(src: &str, arities: &[(&str, usize)]) -> DisplayNotations {
        let file = crate::parse(src).expect("夹具必须能解析");
        let table = notation_table(&file.commands);
        let map: HashMap<String, usize> = arities
            .iter()
            .map(|(n, a)| ((*n).to_string(), *a))
            .collect();
        DisplayNotations::new(table, map)
    }

    /// 夹具（切片：折坏部分应用）：**两张表都给**——`arity` = 全望远镜层数、
    /// `prefixes` = 前导隐式个数。pp 的两种完全应用形状就是这两个数。
    fn notations_with_prefixes(
        src: &str,
        arities: &[(&str, usize)],
        prefixes: &[(&str, usize)],
    ) -> DisplayNotations {
        let file = crate::parse(src).expect("夹具必须能解析");
        let table = notation_table(&file.commands);
        let map: HashMap<String, usize> = arities
            .iter()
            .map(|(n, a)| ((*n).to_string(), *a))
            .collect();
        let prefix: HashMap<String, usize> = prefixes
            .iter()
            .map(|(n, a)| ((*n).to_string(), *a))
            .collect();
        DisplayNotations::new(table, map).with_implicit_prefix(prefix)
    }

    /// **真管线形状**的夹具（切片：折坏部分应用）：与
    /// `compile::display_notations_from_commands` 逐句相同地建两张表
    /// （记法表 + 内建 splice + `arities_with_prelude_from` +
    /// `implicit_prefixes_with_prelude_from`）——折叠层的判据与真编译同源 ✓。
    fn real_pipeline_notations(sources: &[&str]) -> DisplayNotations {
        let mut commands: Vec<crate::ast::Command> = Vec::new();
        for src in sources {
            let file = crate::parse(src).expect("夹具必须能解析");
            commands.extend(file.commands);
        }
        let mut table = notation_table(&commands);
        table.splice(0..0, crate::notation::builtin_notation_decls());
        let arities = arities_with_prelude_from(arities_in_commands(&commands));
        let prefixes =
            implicit_prefixes_with_prelude_from(implicit_prefixes_in_commands(&commands));
        DisplayNotations::new(table, arities).with_implicit_prefix(prefixes)
    }

    // ---- 元数的来源（T-C11）--------------------------------------------

    /// **切片判据（2026-10-10）：pp 的两种完全应用形状都折、部分应用不折。**
    ///
    /// 用户现场：`exact Eq.refl α a` 的 hover 类型行显示
    /// `Eq.refl : {α : Sort u} → (a : α) → (α = a) a`（应为 `… → a = a`）。
    /// 两条根因都在这条测试里钉住：
    /// ① 元数口径曾是"显式 binder 个数" ⇒ **开项**（pp 不省略隐式实参）的
    ///    `Eq.{u} α a a`（3 个实参）被判成"部分应用"；
    /// ② 那条脊的**内层** `Eq.{u} α a`（2 个实参）反倒被判成"完全应用" ⇒
    ///    父节点整脊重渲染 ⇒ `(α = a) a`。
    ///
    /// 夹具走**真管线形状**（[`real_pipeline_notations`]：两张表都从源级签名
    /// 数出来 + 内建记法 splice + prelude）——折叠判据与真编译同源 ✓。
    ///
    /// ⚠ **两处与任务书逐字不同的地方**（都是"文本上不可区分"的必然结果，
    /// 不是判据放松）：`Eq α a` 与 `Set.singleton α` 在**闭项**读法下就是
    /// **完全应用**（pp 省掉前导隐式后的形状，与 `Eq A B` / `Set.singleton a`
    /// 逐字同形）⇒ 折。真正的部分应用（开项的内层节点、`k < operand_count`、
    /// 超出两种形状的 `k`）一律原样 ✓（下面每一条都点明）。
    #[test]
    fn a_full_application_folds_in_both_pp_shapes_and_partial_ones_do_not() {
        let dn = real_pipeline_notations(&[SHAPES_LIB]);

        // ① 等式族：开项全形（`Eq.{u} α a a`）与开项无宇宙标注的写法（pp 实测两种都打）
        assert_eq!(fold_text("Eq.{u} α a a", &dn), "a = a", "开项全形");
        assert_eq!(fold_text("Eq α a a", &dn), "a = a", "开项（无宇宙标注）");
        // **闭项完全应用**（pp 省掉隐式 `α` ⇒ 两个实参就是两个操作数）：
        // 与任务书的"`Eq α a` 必须原样"**不同**——那个文本在闭项读法下就是
        // `α = a`（元素叫 α 与 a），与开项部分应用 `Eq.{u} α a` 逐字同形。
        // 取"折"：否则**所有闭项等式目标**（`⊢ A = B` 显示成 `Eq A B`）都不折 ✗。
        assert_eq!(
            fold_text("Eq α a", &dn),
            "α = a",
            "闭项完全应用（两个实参就是两个操作数）"
        );
        // 真正的部分应用：实参个数连操作数都不够 ⇒ 原样 ✓
        assert_eq!(fold_text("Eq α", &dn), "Eq α", "k = 1 < 操作数 2");
        assert_eq!(fold_text("Eq", &dn), "Eq", "k = 0");

        // ② 隶属/子集：两种形状都折（**闭项形状就是课程 goal 的实测形状**）
        assert_eq!(fold_text("Set.mem α a A", &dn), "a ∈ A", "开项全形");
        assert_eq!(fold_text("Set.mem a A", &dn), "a ∈ A", "闭项省略形");
        assert_eq!(
            fold_text("Set.mem a", &dn),
            "Set.mem a",
            "部分应用（k = 1）"
        );
        assert_eq!(fold_text("Set.subset α A B", &dn), "A ⊆ B");
        assert_eq!(fold_text("Set.subset A B", &dn), "A ⊆ B");
        assert_eq!(fold_text("Set.subset A", &dn), "Set.subset A");

        // ③ 无隐式前导的目标：两种形状重合（回归基线，别被这次改动碰坏）
        assert_eq!(fold_text("And a b", &dn), "a ∧ b");
        assert_eq!(fold_text("Not a", &dn), "¬ a");
        assert_eq!(fold_text("Iff a b", &dn), "a ↔ b");

        // ④ 集合字面量特例同步放宽：元素**从尾部取**
        assert_eq!(fold_text("Set.singleton α a", &dn), "{a}", "开项全形");
        assert_eq!(fold_text("Set.singleton a", &dn), "{a}", "闭项省略形");
        assert_eq!(fold_text("Set.pair α a b", &dn), "{a, b}");
        assert_eq!(fold_text("Set.pair a b", &dn), "{a, b}");
        // 同上那条歧义：`Set.singleton α`（k = 1 = 闭项完全应用）折成 `{α}`，
        // 而 `Set.singleton a` 也折成 `{a}`——两者逐字同形，无法区分。
        assert_eq!(fold_text("Set.singleton α", &dn), "{α}", "闭项完全应用");

        // ⑤ **用户现场整串**：开项 telescope（binder 体里引用 binder）里的
        //    `Eq.{u} α a a` 必须折成 `a = a`（修前是 `(α = a) a`）
        assert_eq!(
            fold_text("{α : Sort u} -> (a : α) -> Eq.{u} α a a", &dn),
            "{α : Sort u} → (a : α) → a = a"
        );

        // ⑥ **内层实参脊仍要折**（切片不能把嵌套一起关掉）：
        //    `Set.singleton α a` 是 `⊆` 的**实参**（不是同一条脊的 `fun`）⇒ 折 ✓。
        //    ⚠ 括号来自 `render_atom`（`proof.rs`，**判定渲染器**）的"复合操作数
        //    补括号"规则——SetLiteral 也在那张表里 ⇒ `({a})`。那是既有规则，
        //    且 proof.rs 不在本切片的可写范围（改它会动判定文本）⇒ 按实测钉住。
        assert_eq!(
            fold_text("Set.subset (Set.singleton α a) A", &dn),
            "({a}) ⊆ A"
        );
        assert_eq!(
            fold_text("Set.mem a (Set.singleton a)", &dn),
            "a ∈ ({a})",
            "课程 goal 的实测形状"
        );

        // ⑦ **脊根规则**（第 4 条）：整条脊的实参个数**超出**两种完全应用形状时
        //    ⇒ 一个节点都不折（内层是这条脊的部分应用，折它会拼出 `(…) …`）。
        //    没有这一条：内层 `Set.mem a A B`（k = 3 = 全层数）会被折成 `A ∈ B`，
        //    父节点整脊重渲染 ⇒ `(A ∈ B) C`。
        assert_eq!(fold_text("Set.mem a A B C", &dn), "Set.mem a A B C");
        assert_eq!(fold_text("Set.mem α a A B", &dn), "Set.mem α a A B");
    }

    /// **折叠规则本身**（两张表显式给，不经源级扫描）：`full = 3`、`prefix = 1`
    /// ⇒ `k == 3`（开项）与 `k == 2`（闭项）都折、其余不折；操作数**从尾部取**。
    #[test]
    fn the_fold_rule_accepts_exactly_the_two_full_application_shapes() {
        let dn = notations_with_prefixes(SET_LIB, &[("Set.mem", 3)], &[("Set.mem", 1)]);
        assert_eq!(
            fold_text("Set.mem α a A", &dn),
            "a ∈ A",
            "开项全形（k = 3）"
        );
        assert_eq!(
            fold_text("Set.mem a A", &dn),
            "a ∈ A",
            "闭项省略形（k = 2）"
        );
        // k = 2 在闭项读法下就是完全应用（前导 `α` 被 pp 省掉）⇒ 折；
        // 只有连操作数都不够（k = 1）才是**两种读法都成立**的部分应用 ⇒ 原样。
        assert_eq!(fold_text("Set.mem α", &dn), "Set.mem α", "k = 1");
        assert_eq!(
            fold_text("Set.mem α a A B", &dn),
            "Set.mem α a A B",
            "k = 4 超出两种形状 ⇒ 整条脊不折（脊根规则）"
        );
        // 前缀表缺项 ⇒ 0（旧行为：只认全层数那一种形状）。
        let no_prefix = notations_with_prefixes(SET_LIB, &[("Set.mem", 3)], &[]);
        assert_eq!(fold_text("Set.mem α a A", &no_prefix), "a ∈ A");
        assert_eq!(fold_text("Set.mem a A", &no_prefix), "Set.mem a A");
    }

    /// **前缀表的来源**（切片）：`{α : Type}` 是隐式 ⇒ 1；显式 binder 打断就停；
    /// 归纳块先数 `params` 再（全隐式时）接 `ty`。
    #[test]
    fn implicit_prefixes_are_counted_from_the_signature() {
        let prefixes = implicit_prefixes_in_sources(&[SHAPES_LIB]);
        assert_eq!(prefixes.get("Set.mem").copied(), Some(1), "`{{α : Type}}`");
        assert_eq!(prefixes.get("Set.subset").copied(), Some(1));
        // 显式 binder 在开头 ⇒ 0（pp 不会跨过显式实参去省后面的隐式）。
        assert_eq!(
            implicit_prefixes_in_sources(&["def f (a : Prop) {b : Prop} : Prop := a\n"])
                .get("f")
                .copied(),
            Some(0)
        );
        // 归纳块：`params` = **全部**参数（隐式与显式都在里面）⇒ 前导隐式数到
        // 第一个显式参数为止（`{A : Type} (p : A -> Prop)` ⇒ 1）。
        let ind = "inductive Exists {A : Type} (p : A -> Prop) : Prop\n\
                   ctor intro (w : A) (h : p w) : Exists A p\n\
                   end\n";
        let p = implicit_prefixes_in_sources(&[ind]);
        assert_eq!(p.get("Exists").copied(), Some(1));
        // 与 elab 的同名函数同口径（同一份源级 AST 走查）。
        let file = crate::parse(ind).expect("夹具必须能解析");
        if let crate::ast::Command::InductiveBlock { ty, params, .. } = &file.commands[0] {
            let (n, all) = leading_implicit_binders(params);
            assert_eq!(n, 1, "`{{A : Type}}` 是前导隐式");
            assert!(!all, "`(p : A -> Prop)` 是显式 ⇒ 前导隐式到此为止");
            assert_eq!(leading_implicit_prefix(ty), 0, "`ty` = `Prop`，没有 binder");
        } else {
            panic!("夹具第一个命令必须是归纳块");
        }
        // 全隐式 `params` 时**才**接着数 `ty` 的前导隐式。
        let all_implicit = "inductive Box {A : Type} {B : Type} : Type\n\
                            ctor mk (a : A) (b : B) : Box A B\n\
                            end\n";
        assert_eq!(
            implicit_prefixes_in_sources(&[all_implicit])
                .get("Box")
                .copied(),
            Some(2)
        );
    }

    /// **判据**：`infix:50 " ∈ " => Set.mem` 的 telescope = **3**（`α` / `a` / `A`），
    /// 而 `∈` 是二元 ⇒ **前导参数 = 1**（那个 `α`）。两者之差正是折叠时丢掉的东西。
    #[test]
    fn arity_of_set_mem_counts_the_whole_telescope() {
        let arities = arities_in_sources(&[SET_LIB]);
        assert_eq!(
            arities.get("Set.mem").copied(),
            Some(3),
            "`Set.mem (α : Type) (a : α) (A : Set α)` ⇒ 3 层"
        );
        // 二元记法只写出 2 个位置 ⇒ 前导参数 1 个（`α`）。
        assert_eq!(arities["Set.mem"] - 2, 1, "前导参数 = α");
        // 同一个源里的其它目标也对得上（`Set.image` 有 4 层：α β f A）。
        assert_eq!(arities.get("Set.union").copied(), Some(3));
        assert_eq!(arities.get("Set.image").copied(), Some(4));
    }

    /// **元数 = 全 telescope 层数**（含隐式 binder）——切片（2026-10-10）改口径的判据。
    ///
    /// `axiom Eq {u} : {α : Sort u} -> α -> α -> Prop` 的 telescope 是 **3**；pp 打
    /// **开项**（含松散变量）时**不省略**隐式实参 ⇒ 打出来是 `Eq.{u} α a a`（**3** 个
    /// 实参）。旧口径（显式 binder 个数 = 2）会把这种完全应用判成"部分应用"、同时把
    /// **内层**的 `Eq.{u} α a` 判成完全应用 ⇒ `(α = a) a`（用户现场）。
    /// 闭项形状（pp 省掉隐式 α ⇒ `Eq A B`，**2** 个实参）由**前导隐式表**配合认。
    ///
    /// `Ne` 的 `α` 是**显式**的（`def Ne {u} (α : Sort u) (a b : α)`）⇒ 前导隐式 0
    /// ⇒ 两种形状重合（只有 3）。这一对（Eq 3+prefix 1 / Ne 3+prefix 0）正好把新
    /// 口径钉死。
    #[test]
    fn arity_is_the_whole_telescope_and_the_prefix_says_what_pp_elides() {
        let arities = arities_with_prelude(&[]);
        assert_eq!(
            arities.get("Eq").copied(),
            Some(3),
            "`{{α : Sort u}} -> α -> α -> Prop` ⇒ 3 层（开项 pp 写全 3 个实参）"
        );
        assert_eq!(
            prelude_implicit_prefixes().get("Eq").copied(),
            Some(1),
            "`{{α : Sort u}}` ⇒ 前导隐式 1（闭项 pp 省掉它 ⇒ `Eq A B`）"
        );
        assert_eq!(arities.get("Ne").copied(), Some(3));
        assert_eq!(
            prelude_implicit_prefixes().get("Ne").copied(),
            Some(0),
            "`(α : Sort u)` 是显式 ⇒ 不省"
        );
        // 全是显式 binder 的目标：两层数相同。
        assert_eq!(arities.get("And").copied(), Some(2));
        assert_eq!(arities.get("Not").copied(), Some(1));
        assert_eq!(arities.get("Iff").copied(), Some(2));
    }

    /// **内建记法要自己补**：`↔`/`∧`/`∨`/`¬`/`=`/`≠` 不在任何源文本里
    /// （parser 有硬编码表），"从源里收记法"收不到它们。
    ///
    /// **两张表都要**（切片：折坏部分应用）：`Eq` 的 `α` 是隐式 ⇒ 只用 arity 会把
    /// 闭项形状 `Eq A B`（pp 省掉 `α`）判成部分应用 ⇒ 这里改走**真管线形状**的夹具
    /// （[`real_pipeline_notations`]）✓。
    #[test]
    fn builtin_notations_fold_too() {
        let dn = real_pipeline_notations(&[SET_LIB]);
        assert_eq!(
            fold_text("Iff (Set.subset α A B) (Set.subset α A B)", &dn),
            "(A ⊆ B) ↔ (A ⊆ B)"
        );
        assert_eq!(fold_text("And p q", &dn), "p ∧ q");
        // **闭项形状**（隐式 α 被 pp 省掉）与**开项形状**都要折。
        assert_eq!(fold_text("Eq A B", &dn), "A = B");
        assert_eq!(fold_text("Eq.{u} α a a", &dn), "a = a");
        // 一元前缀（`¬`）**现在也折**（T-D51：四种记法都折，见
        // `every_notation_kind_folds`）；这里顺带守住"内建的前缀也走同一条路"。
        assert_eq!(fold_text("Not p", &dn), "¬ p");
    }

    /// `namespace` 里的声明要按**全名**记（`NotationDecl.target` 存的是全名）。
    #[test]
    fn arity_is_keyed_by_the_qualified_name() {
        let arities =
            arities_in_sources(&["namespace Foo\ndef bar (a : Prop) : Prop := a\nend Foo\n"]);
        assert_eq!(arities.get("Foo.bar").copied(), Some(1));
        assert_eq!(arities.get("bar"), None, "短名不该被当成全名");
    }

    /// prelude 里的记法目标（`And` / `Or` / `Not` / `Iff` / `Eq`）也要数得到
    /// ——四个生产者都会用到它们。
    #[test]
    fn prelude_targets_are_counted_too() {
        let arities = arities_with_prelude(&[]);
        assert_eq!(arities.get("And").copied(), Some(2), "`And (a b : Prop)`");
        assert_eq!(arities.get("Not").copied(), Some(1));
        assert_eq!(arities.get("Iff").copied(), Some(2));
    }

    /// 找不到 ⇒ `None` ⇒ 折叠层原样返回（不猜）。
    #[test]
    fn an_unknown_target_has_no_arity() {
        let arities = arities_in_sources(&[SET_LIB]);
        assert_eq!(arities.get("Nobody.knows"), None);
    }

    /// 端到端：**从源文本数出来的 arity 直接喂给折叠层**（T-C11 的接入形状）。
    #[test]
    fn arities_from_source_drive_the_fold() {
        let file = crate::parse(SET_LIB).expect("夹具必须能解析");
        let table = notation_table(&file.commands);
        let arities = arities_in_sources(&[SET_LIB]);
        let dn = DisplayNotations::new(table, arities);
        assert_eq!(fold_text("Set.mem α a A", &dn), "a ∈ A");
        // 部分应用（2 ≠ 3）⇒ 原样。
        assert_eq!(fold_text("Set.mem α a", &dn), "Set.mem α a");
    }

    // ---- 边界：命中不了就回退（T-C25）-----------------------------------

    /// **折叠层只做二元 infix 族**，其余形态**一律回退点名，不猜**。
    ///
    /// 四种形态都取课程库里的**真实写法**（`courses/set-theory/lib/Set.sokonanoda`）：
    /// `notation "∅" => Set.empty` / `prefix:100 " 𝒫 " => …` /
    /// `postfix:100 " ᶜ " => …` / `binder_notation "∃" => …`。
    ///
    /// 回退是**设计**不是缺陷：这些形态的操作数位不同（一元 / 零元 / binder 位），
    /// 折叠规则要各写一套；第一刀（T-C10）只做 infix 族。**猜错的代价**是把
    /// `Set.powerset α A` 打成 `𝒫 A`（丢掉 `α`）之类——那比不折坏得多。
    #[test]
    fn every_notation_kind_folds() {
        // **T-D51 / 缺口 G-38**：这条测试以前叫
        // `only_binary_infix_folds_and_the_rest_fall_back`——断言 prefix/postfix/
        // 零元/binder **不折**（"留给后续环节"）。现在四种都折，所以它是
        // **新契约**的判据（不是把断言改松：每条都从"点名"变成"记法"）。
        let arities = arities_in_sources(&[BOUNDARY_LIB]);
        let dn = DisplayNotations::new(
            crate::notation::notation_table(
                &crate::parse(BOUNDARY_LIB).expect("夹具必须能解析").commands,
            ),
            arities,
        );
        // ① 一元前缀：操作数在 `rhs`。
        assert_eq!(fold_text("Set.powerset α A", &dn), "𝒫 A");
        // ② 一元后缀：操作数在 `lhs`。
        assert_eq!(fold_text("Set.compl α A", &dn), "A ᶜ");
        // ③ 零元常量记法（无操作数）。
        assert_eq!(fold_text("Set.empty α", &dn), "∅");
        // ④ binder 位记法（`∃ x, p`）：操作数是那个 lambda。
        // 渲染**带 binder 类型**（T-D51 顺带：以前多 binder 只打名字 ⇒ 折了反而
        // 丢信息；现在 `∃ (x : α), p x`，与 Lean 一致）。
        assert_eq!(
            fold_text("Exists α (fun (x : α) => p x)", &dn),
            "∃ (x : α), p x"
        );
        // ⑤ 对照：二元 infix（证明表确实建起来了，上面四条不是"表是空的"假绿）。
        assert_eq!(fold_text("Set.mem α a A", &dn), "a ∈ A");
        // ⑥ **元数对不上仍回退**（部分应用不是记法实例）——这条是原来那条
        // 测试真正要守的边界，保留。
        assert_eq!(fold_text("Set.mem α a", &dn), "Set.mem α a");
        assert_eq!(fold_text("Set.powerset α", &dn), "Set.powerset α");
    }

    /// **部分应用不回退成"看起来像"的东西**：元数对不上就不折。
    ///
    /// `Set.mem α a`（3 元只给了 2 个）折成 `a ∈` 是荒谬的；`Set.mem α a A B`
    /// （多给一个）也**不折**——**切片（2026-10-10）改了后者的期望值**：脊根规则
    /// （[`fold_collecting_inner`] 的 `is_fun_part`）让"元数对不上的脊"里**内层**
    /// 节点也不折，所以过应用整条原样 ✓。
    ///
    /// ⚠ **为什么不再折成 `(a ∈ A) B`**：那条路要先把内层 `Set.mem α a A` 折成
    /// `a ∈ A`、再由父节点整脊重渲染补括号。同一条规则在 `Eq.{u} α a a` 上就是
    /// 用户报的 bug（内层 `Eq.{u} α a` 被当成完全应用 ⇒ `(α = a) a`）——根因是
    /// **内层节点不是一条独立的脊**，它是同一条脊的部分应用。过应用在真 pp 输出里
    /// 不出现（它不良型），所以代价只是"不折"而不是"折错"✓。
    #[test]
    fn partial_and_over_application_fall_back() {
        let dn = notations(SET_LIB, SET_ARITY);
        assert_eq!(fold_text("Set.mem α a", &dn), "Set.mem α a");
        assert_eq!(fold_text("Set.mem α a A B", &dn), "Set.mem α a A B");
        // 头不是记法目标时也不折（`Set.union` 是，`Set` 不是）。
        assert_eq!(fold_text("Set α", &dn), "Set α");
    }

    // ---- 重载的处置（T-C13）--------------------------------------------

    /// **同一 target、两个符号**（`∈` 与 `∊` 都 => `Set.mem`）：取**声明顺序第一个**。
    ///
    /// 这是反向折叠**唯一残留的歧义**（前向是"符号 → 候选目标，按期望类型选"，
    /// 反向是"目标 → 符号"，head 名字就是判据 ⇒ 天然单值；只有"一个目标配了
    /// 两个符号"时才有得选）。规则写死成"声明顺序第一个"，并且**顺序真的有意义**
    /// ——下面第二条把顺序倒过来，符号跟着变，证明判据就是顺序本身。
    #[test]
    fn one_target_with_two_symbols_takes_the_first_declared() {
        let dn = notations(
            "def Set (α : Type) : Type := α -> Prop\n\
             def Set.mem (α : Type) (a : α) (A : Set α) : Prop := A a\n\
             infix:50 \" ∈ \" => Set.mem\n\
             infix:50 \" ∊ \" => Set.mem\n",
            &[("Set.mem", 3)],
        );
        assert_eq!(fold_text("Set.mem α a A", &dn), "a ∈ A", "取声明顺序第一个");

        // 顺序倒过来 ⇒ 折出来的是另一个符号（判据确实是顺序，不是别的）。
        let flipped = notations(
            "def Set (α : Type) : Type := α -> Prop\n\
             def Set.mem (α : Type) (a : α) (A : Set α) : Prop := A a\n\
             infix:50 \" ∊ \" => Set.mem\n\
             infix:50 \" ∈ \" => Set.mem\n",
            &[("Set.mem", 3)],
        );
        assert_eq!(fold_text("Set.mem α a A", &flipped), "a ∊ A");
    }

    /// **同一符号、两个 target**（`⊕` => `AddA` 与 `AddB`）：**不是歧义**。
    ///
    /// 反向的判据是 head 名字 ⇒ 两个 head 各自折成同一个符号，都对。前向那条
    /// 路（符号 → 候选）才需要按期望类型挑，与折叠层无关。
    #[test]
    fn one_symbol_with_two_targets_folds_by_head_without_ambiguity() {
        let dn = notations(
            "def AddA (a b : Prop) : Prop := a\n\
             def AddB (a b : Prop) : Prop := b\n\
             infixl:60 \" ⊕ \" => AddA\n\
             infixl:60 \" ⊕ \" => AddB\n",
            &[("AddA", 2), ("AddB", 2)],
        );
        assert_eq!(fold_text("AddA p q", &dn), "p ⊕ q");
        assert_eq!(fold_text("AddB p q", &dn), "p ⊕ q");
    }

    // ---- 损失护栏（T-C14）----------------------------------------------

    /// **折叠是幂等的**：把折过的文本再折一次，一个字节都不变。
    ///
    /// 为什么这条是护栏：折叠层的产物会**再次**经过显示出口（例如 Infoview 收到
    /// 文本后又渲染一遍）。如果折叠不幂等，第二遍就会叠出 `(a ∈ A) ∈ B` 这种
    /// 东西——那正是"折叠把文本改坏"的形状。
    #[test]
    fn folding_is_idempotent() {
        let dn = notations(SET_LIB, SET_ARITY);
        for once in [
            "Set.mem α a A",
            "Set.subset α A (Set.union α B C)",
            "Set.image α β f (Set.image α β g A)",
            "forall (x : α), Set.mem α x (Set.union α A B)",
        ] {
            let folded = fold_text(once, &dn);
            assert_eq!(fold_text(&folded, &dn), folded, "再折一次不该变：{once}");
        }
    }

    /// **折叠的产物必须能重新解析**（它是给人看的文本，而人会把它抄回源里）。
    ///
    /// 注意这条**不是**"逐字节回读等价"：折过的文本重新解析得到的是
    /// `Expr::Notation` 节点（不是展开后的 `App`），两者**结构上不同**——那正是
    /// 记法的定义。真正的结构性护栏是 [`DisplayText`]（T-C03b）：折叠的产物
    /// **在类型上**进不了任何回读通道（`parse_expr_text(&display_text)` 编译不过）。
    /// 这条只钉"文本本身没被折坏"。
    #[test]
    fn folded_text_reparses() {
        let dn = notations(SET_LIB, SET_ARITY);
        // 重新解析要**同一张表**（`∈` 是从 `SET_LIB` 来的，空表当然解析不了）。
        let file = crate::parse(SET_LIB).expect("夹具必须能解析");
        let table = notation_table(&file.commands);
        for once in [
            "Set.mem α a A",
            "Set.subset α A (Set.union α B C)",
            "Set.image α β f (Set.image α β g A)",
        ] {
            let folded = fold_text(once, &dn);
            assert_ne!(folded, once, "夹具前提：这条确实折过了");
            assert!(
                crate::proof::parse_expr_text_with(&folded, &table).is_ok(),
                "折叠产物必须能重新解析：{once} → {folded}"
            );
        }
    }

    // ---- 逐 surface 的判别性（T-C24）------------------------------------

    /// **关掉折叠 ⇒ 每一个"靠折叠才有记法"的 surface 都回到点名**。
    ///
    /// 这条是 T-C24 的**机械判据**：`SOKO_NO_NOTATION_FOLD=1` 走的就是"空表"这
    /// 条路（`display_notations` 直接返回 `default()`）。实测关掉之后：
    ///
    /// | surface | 关掉后 |
    /// |---|---|
    /// | 生产者 1 根状态 | **红**（记法是折叠给的） |
    /// | 生产者 3 声明 `ty` | **红**（同上） |
    /// | 生产者 4 `by` 步进 | **红**（同上） |
    /// | 生产者 2 无 `by` 的开练习 | 仍绿——它的记法来自 `render_expr` 的**源级渲染**，不是折叠 |
    /// | 假设行 `binders[].ty` | 仍绿——binder 类型是**源里写的** |
    ///
    /// 后两条因此是**守护**而不是折叠的判据（T-C21/T-C23 各有一条）。
    #[test]
    fn with_the_fold_off_every_foldable_surface_is_pointwise() {
        let empty = DisplayNotations::default();
        assert_eq!(fold_text("And p q", &empty), "And p q");
        assert_eq!(fold_text("Set.mem α a A", &empty), "Set.mem α a A");
        assert_eq!(
            fold_text("Iff (Set.subset α A B) (Set.subset α A B)", &empty),
            "Iff (Set.subset α A B) (Set.subset α A B)"
        );
    }

    /// T-C25 的夹具：四种**折叠层不做**的记法形态（写法取自课程库）。
    const BOUNDARY_LIB: &str = "\
def Set (α : Type) : Type := α -> Prop\n\
def Set.empty (α : Type) : Set α := fun (x : α) => False\n\
notation \"∅\" => Set.empty\n\
def Set.powerset (α : Type) (A : Set α) : Set α := A\n\
prefix:100 \" 𝒫 \" => Set.powerset\n\
def Set.compl (α : Type) (A : Set α) : Set α := A\n\
postfix:100 \" ᶜ \" => Set.compl\n\
def Exists (α : Type) (p : α -> Prop) : Prop := p\n\
binder_notation \"∃\" => Exists\n\
def Set.mem (α : Type) (a : α) (A : Set α) : Prop := A a\n\
infix:50 \" ∈ \" => Set.mem\n";

    /// 一个最小的集合词汇 + 两条记法：`∈`（优先级 50）与 `∪`（左结合 65）。
    const SET_LIB: &str = "\
def Set (α : Type) : Type := α -> Prop\n\
def Set.mem (α : Type) (a : α) (A : Set α) : Prop := A a\n\
infix:50 \" ∈ \" => Set.mem\n\
def Set.union (α : Type) (A B : Set α) : Set α := fun (x : α) => A x\n\
infixl:65 \" ∪ \" => Set.union\n\
def Set.subset (α : Type) (A B : Set α) : Prop := forall (x : α), A x -> B x\n\
infix:50 \" ⊆ \" => Set.subset\n\
def Set.image (α : Type) (β : Type) (f : α -> β) (A : Set α) : Set β := fun (y : β) => A y\n\
infixr:80 \" '' \" => Set.image\n";

    /// 元数：`Set.mem` / `Set.union` / `Set.subset` 都是 3 层（`α` 是隐式前导）。
    const SET_ARITY: &[(&str, usize)] = &[
        ("Set.mem", 3),
        ("Set.union", 3),
        ("Set.subset", 3),
        ("Set.image", 4),
    ];

    /// **切片（2026-10-10）的夹具库**：与课程库同形——前导类型参数**隐式**
    /// （`{α : Type}`）⇒ pp 的闭项形状会省略它（`Set.mem a A`），开项形状不会
    /// （`Set.mem α a A`）。两种形状都要折，这条夹具是判据的输入。
    const SHAPES_LIB: &str = "\
def Set (α : Type) : Type := α -> Prop\n\
def Set.mem {α : Type} (a : α) (A : Set α) : Prop := A a\n\
infix:50 \" ∈ \" => Set.mem\n\
def Set.subset {α : Type} (A B : Set α) : Prop := forall (x : α), Set.mem x A -> Set.mem x B\n\
infix:50 \" ⊆ \" => Set.subset\n\
def Set.singleton {α : Type} (a : α) : Set α := fun (x : α) => x = a\n\
def Set.pair {α : Type} (a b : α) : Set α := fun (x : α) => x = a\n";

    fn fold_text(text: &str, dn: &DisplayNotations) -> String {
        print_back(text, dn).as_display_str().to_string()
    }

    /// **A1 判据**（2026-09-26 用户报告第 1 条）：内核 pp 的 `->` 必须折成 `→`。
    ///
    /// 证据：`courses/set-theory/.sokonanoda/compiled/*.json` 的 `ty_text` 里
    /// **227 条**是混合形态（`∀ (α β γ : Type 0), (α -> β) -> …`）——`∀` 折了、
    /// `->` 没折 ⇒ Infoview 顶部「目标」看起来"没记法化"。根因是
    /// `fold_collecting_inner` 只处理了 `forall` **关键字**，`Expr::Arrow` 一个
    /// 字节都没动。
    #[test]
    fn arrows_fold_to_the_unicode_arrow() {
        let dn = notations(SET_LIB, SET_ARITY);
        assert_eq!(fold_text("α -> β", &dn), "α → β");
        assert_eq!(
            fold_text("Set.mem α a A -> Set.mem α b B", &dn),
            "a ∈ A → b ∈ B"
        );
        // 右结合链 + 左操作数是复合式（括号必须留着）
        assert_eq!(fold_text("(α -> β) -> γ", &dn), "(α → β) → γ");
        assert_eq!(fold_text("α -> β -> γ", &dn), "α → β → γ");
        // 内核 pp 的 telescope 形态（A1 的原始证据形状）
        assert_eq!(
            fold_text(
                "forall (α β γ : Type 0), (β -> γ) -> (α -> β) -> α -> γ",
                &dn
            ),
            "∀ (α β γ : Type 0), (β → γ) → (α → β) → α → γ"
        );
        // 源级 `(x : α) -> β`（parser 给的是匿名 binder 的 `Forall`）
        assert_eq!(fold_text("(x : α) -> β", &dn), "(x : α) → β");
        // 域是**带括号的记法**（课程产物里残留的那一例：`mem_union_comm` 的 goal）
        assert_eq!(
            fold_text(
                "(Set.mem α a (Set.union α A B)) -> Set.mem α a (Set.union α B A)",
                &dn
            ),
            "(a ∈ (A ∪ B)) → a ∈ (B ∪ A)"
        );
        // 幂等：折过的文本再折一次逐字节不变
        let once = fold_text("forall (α : Type 0), α -> α", &dn);
        assert_eq!(fold_text(&once, &dn), once);
        // **第二趟**（A1 收尾）：`↔` 那条脊会把它的操作数**整段重渲染**，
        // 而 `render_expr` 打的是 ASCII 箭头 ⇒ 第一趟折好的 `∀ … -> …` 又被画回
        // `->`。实测这一条是课程产物里 9/273 条残留的形状。
        let swallowed = fold_text(
            "Iff (Function.Injective α β f) (forall (x : α), f x = f y -> x = y)",
            &dn,
        );
        assert!(
            swallowed.contains('→') && !swallowed.contains("->"),
            "被外层重渲染吞掉的箭头必须由第二趟补折，实际 = {swallowed}"
        );
    }

    /// **A2 判据**（2026-09-26 用户报告第 2 条）：集合字面量的**点名展开**必须折回
    /// `{a}` / `{a, b}`。
    ///
    /// 证据：产物里是 `Set.singleton (Set Nat) (∅)`，而源里写的是 `{∅}` ⇒
    /// Infoview/ty 面显示的是展开式。`{a}` 是**内建语法**（不是记法声明）⇒
    /// `fold_spine` 查表查不到它，得照 `forall` 的先例单独给一条规则。
    #[test]
    fn set_literals_fold_back_to_braces() {
        let dn = notations(SET_LIB, SET_ARITY);
        assert_eq!(
            fold_text("Set.singleton Nat zero", &dn),
            "{zero}",
            "一元集合字面量"
        );
        assert_eq!(
            fold_text("Set.pair Nat zero one", &dn),
            "{zero, one}",
            "二元集合字面量"
        );
        // **部分应用不折**（与记法同一条硬规则：只有完全应用才是那个形状）。
        // `Set.pair` 接受 `k == 3 || k == 2`（开项 / 闭项两种完全应用形状）
        // ⇒ 反例取 `k == 1`。
        assert_eq!(fold_text("Set.pair Nat", &dn), "Set.pair Nat");
        // **闭项省略形**（pp 省掉元素类型 ⇒ 只有元素）也是完全应用 ⇒ 折
        // （课程 goal 里的 `Set.mem a (Set.singleton a)` 就是这个形状）。
        assert_eq!(fold_text("Set.singleton Nat", &dn), "{Nat}");
        assert_eq!(fold_text("Set.pair Nat zero", &dn), "{Nat, zero}");
    }

    #[test]
    fn folds_a_binary_infix_and_drops_the_leading_type_argument() {
        let dn = notations(SET_LIB, SET_ARITY);
        // 内核 pp 的点名形式 → 记法形式。`α`（前导隐式类型参数）被丢掉。
        assert_eq!(fold_text("Set.mem α a A", &dn), "a ∈ A");
    }

    #[test]
    fn left_associative_notation_nests_without_extra_parens() {
        let dn = notations(SET_LIB, SET_ARITY);
        // `∪` 是 `infixl:65` ⇒ 左结合。`render_expr` 的规则是**保守补括号**，
        // 所以这里钉的是"折对了 + 括号不多不少"。
        assert_eq!(
            fold_text("Set.union α (Set.union α A B) C", &dn),
            "(A ∪ B) ∪ C"
        );
    }

    #[test]
    fn right_associative_notation_nests_to_the_right() {
        let dn = notations(SET_LIB, SET_ARITY);
        // `''` 是 `infixr:80`（右结合）：`f '' (g '' A)` 折出来是
        // `f '' (g '' A)`——右结合的记法右边**不需要**括号，但 `render_atom`
        // 是保守的（一律给复合操作数加括号）⇒ 这里钉的是"折对了 + 括号不少"。
        assert_eq!(
            fold_text("Set.image α β f (Set.image α β g A)", &dn),
            "f '' (g '' A)"
        );
        // 左结合的 `∪` 反过来：`(A ∪ B) ∪ C`。
        assert_eq!(
            fold_text("Set.union α (Set.union α A B) C", &dn),
            "(A ∪ B) ∪ C"
        );
    }

    #[test]
    fn precedence_mismatch_gets_parentheses() {
        let dn = notations(SET_LIB, SET_ARITY);
        // `∈`（50）比 `∪`（65）**松**：`a ∈ A ∪ B` 里的并集**不需要**括号，
        // 而 `(a ∈ A) ∪ (a ∈ B)` 这种形状本来就不是良型的——所以钉一条
        // **两个不同优先级嵌套**的：`Set.mem α a (Set.union α A B)`。
        assert_eq!(
            fold_text("Set.mem α a (Set.union α A B)", &dn),
            "a ∈ (A ∪ B)"
        );
        // 反向：并集的**参数**是隶属（不合法形状不会出现），改用子集：
        // `Set.subset α (Set.union α A B) C` ⇒ `(A ∪ B) ⊆ C`。
        assert_eq!(
            fold_text("Set.subset α (Set.union α A B) C", &dn),
            "(A ∪ B) ⊆ C"
        );
    }

    #[test]
    fn folds_nested_occurrences_including_inside_binders() {
        let dn = notations(SET_LIB, SET_ARITY);
        // 两层记法。
        assert_eq!(
            fold_text("Set.subset α A (Set.union α B C)", &dn),
            "A ⊆ (B ∪ C)"
        );
        // binder **体里**也折（结构走查列全了每个变体），而 **binder 的写法原样
        // 保留**——这正是"按 span 拼接、不重渲染整棵树"要的效果。
        assert_eq!(
            fold_text("forall (x : α), Set.mem α x (Set.union α A B)", &dn),
            // T-D51：`forall` 关键字也折成 `∀`（**只换关键字那 6 个字节**，
            // binder 分组与 `Type 0` 逐字节不动）。
            "∀ (x : α), x ∈ (A ∪ B)"
        );
    }

    /// **折过之后，没折的部分仍然逐字节原样**——binder 写法、`Type 0`、分组、
    /// 换行、缩进都不动，只有记法那几段被替换。
    ///
    /// 这条是 T-C20 拍板"**按 span 拼接**而不是重渲染整棵树"的判据：重渲染会把
    /// `forall (a b : T), …` 拆成箭头链、把 `Type 0` 重排成 `Sort 1`（`render_expr`
    /// 是回读通道的输入，它必须那样写）——用户要的是**记法**，不是整句换个写法。
    #[test]
    fn only_the_folded_spans_change() {
        let dn = notations(SET_LIB, SET_ARITY);
        assert_eq!(
            fold_text(
                "forall (α : Type 0) (A B : Set α), Set.subset α A B -> Set.subset α B A",
                &dn
            ),
            // `->` 自 2026-09-26（A1）起**也是**被折的 span（`→`），所以它不再
            // 属于"没折的部分"；binder 分组、`Type 0`、换行、缩进仍逐字节不动。
            "∀ (α : Type 0) (A B : Set α), A ⊆ B → B ⊆ A",
            "binder 分组与 `Type 0` 必须原样，只换记法与箭头"
        );
        // 折行与缩进也保留（pp 的长签名会折行）。
        assert_eq!(
            fold_text("forall (α : Type 0),\n  Set.mem α a A", &dn),
            "∀ (α : Type 0),\n  a ∈ A"
        );
    }

    /// **一处都没折 ⇒ 逐字节原样**（含 binder 写法、换行、空格）。
    ///
    /// 这条是防"显示漂移"的主护栏：`render_expr` 会重排 binder（`forall` →
    /// 箭头链），所以"没折也重渲染"会让**每一条不带记法的类型**都跟着改样子。
    #[test]
    fn text_without_any_fold_is_returned_byte_for_byte() {
        let dn = notations(SET_LIB, SET_ARITY);
        for untouched in [
            // 内核 pp 的多 binder 分组 + `Type 0` 的写法：**不带记法** ⇒
            // 一个字节都不许动（重渲染会把它变成 `(α : Sort 1) -> …`）。
            // ⚠ **别拿 `forall` 当"不带记法"的样本**：T-D51 起 `forall` 关键字
            // 本身也折成 `∀`（只换关键字那 6 个字节）——这条测试的夹具因此改成
            // 真的没有可折形状的文本。
            "Other.thing (α : Type 0) (A B : Set α) -> Other.thing α A B",
            // 折行 + 缩进也保留。
            "Other.thing (α : Type 0),\n  Other.thing α A B",
            // 元数对不上（部分应用）⇒ 不算折过。
            "Set.mem α a",
        ] {
            assert_eq!(fold_text(untouched, &dn), untouched, "不该动：{untouched}");
        }
    }

    #[test]
    fn a_miss_falls_back_to_the_pointwise_text() {
        let dn = notations(SET_LIB, SET_ARITY);
        // 表里没有的目标 ⇒ 原样（不猜）。
        assert_eq!(fold_text("Other.thing α a b", &dn), "Other.thing α a b");
        // **元数对不上**（部分应用 `Set.mem α a` 只有 2 个实参）⇒ 原样。
        // 这条就是 §3.2 的硬规则：折成 `α ∈ a` 会是显示错误。
        assert_eq!(fold_text("Set.mem α a", &dn), "Set.mem α a");
        // 空表 ⇒ 逐字节原样（含换行的长签名也不能被动过）。
        let empty = DisplayNotations::default();
        let long = "forall (α : Type 0) (A B : Set α),\n  Set.subset α A B";
        assert_eq!(fold_text(long, &empty), long);
    }

    #[test]
    fn loose_bvars_and_unparsable_text_are_returned_untouched() {
        let dn = notations(SET_LIB, SET_ARITY);
        // F3：`$N` 是 pp 对"binder 在被打印项之外"的写法，结构不可信。
        let loose = "Set.mem α $0 A";
        assert_eq!(fold_text(loose, &dn), loose);
        // 解析不了 ⇒ 原样。
        let broken = "Set.mem α (";
        assert_eq!(fold_text(broken, &dn), broken);
    }
    /// **量一次真实的数**（2026-09-25）：prelude 段里**本来就该有**的记法目标。
    ///
    /// 已经量到的事实（别再猜 ✗）：
    /// * `prelude_arities()` 里 `And`=2 · `Or`=2 · `Not`=1 · `Iff`=2 · `Eq`=2 ✓，
    ///   而 **`Exists` = `None`** ✗ —— 因为 `Exists` **不在 prelude 段里**，
    ///   它是课程库 `courses/set-theory/lib/Exists.sokonanoda:88` 的
    ///   **`inductive`** ✓ ⇒ 元数只能从**闭包的命令**里来 ✓；
    /// * 建表处 `crates/front/src/compile/check/mod.rs:429-442` 用的**已经是全闭包**
    ///   （`units.iter().flat_map(|u| u.file.commands)`）✓，而
    ///   `arities_in_commands` **也已经**处理 `InductiveBlock`
    ///   （`params.len() + telescope_len(ty)` = 2 ✓）✓。
    ///   ⇒ **两处看起来都对，`Exists` 却仍然不在表里** ✗ —— 下一个动作**不是**改代码，
    ///   而是**量真实管线的那张表**（下面的第二个测试就是那个量具 ✓）。
    #[test]
    fn prelude_arities_cover_their_own_targets() {
        let a = prelude_arities();
        // **切片（2026-10-10）**：口径改成"全望远镜层数" ⇒ `Eq` 是 **3**
        // （`{α : Sort u} -> α -> α -> Prop`），闭项形状由隐式前缀表配合认。
        for (name, want) in [("And", 2), ("Or", 2), ("Not", 1), ("Iff", 2), ("Eq", 3)] {
            assert_eq!(
                a.get(name),
                Some(&want),
                "prelude 自己的目标 {name} 必须元数={want}"
            );
        }
        // `Exists` **不在** prelude 段里（在课程库里）⇒ 这里为 None 是**对的** ✓
        assert_eq!(
            a.get("Exists"),
            None,
            "Exists 住在课程库，不该出现在 prelude 段里"
        );
    }

    /// **真实管线的元数表**：闭包里的 `inductive Exists` 必须进表（否则 `∃` 折不了 ✗）。
    ///
    /// 这条是"③ Infoview 点形式"的**测量入口**：它若红，就把
    /// `crates/front/src/compile/check/mod.rs:441-442` 那张表的来源打出来看
    /// （谁被收集了、`Exists` 的键长什么样），**别再从下游猜** ✗。
    #[test]
    fn closure_arities_include_an_imported_inductive() {
        // `inductive` 块里只允许 `ctor`/`rec`/`iota`/`end` ⇒ **必须写 `end`** 才能
        // 接着写 `binder_notation`（实测：漏了 `end` 直接 parse 报错 ✗）。
        let lib = "inductive Exists (A : Type) (p : A -> Prop) : Prop\n\
                   ctor intro (w : A) (h : p w) : Exists A p\n\
                   end\n\
                   binder_notation \"∃\" => Exists\n";
        let file = crate::parse(lib).expect("夹具必须能解析");
        let arities = arities_with_prelude_from(arities_in_commands(&file.commands));
        assert_eq!(
            arities.get("Exists"),
            Some(&2),
            "闭包里的 `inductive Exists (A) (p)` 必须在元数表里=2；\
             不在 ⇒ `Exists (fun …)` 永远折不成 `∃ …` ✗（实测 {arities:?}）"
        );
    }
    /// **量具 ③**（2026-09-25）：`notation_table` 收不收 **`binder_notation`**？
    ///
    /// 这是 ③ 最后一个嫌疑：课程库 `lib/Exists.sokonanoda:103` 用
    /// `binder_notation "∃" => Exists` 声明存在量词记法 ✓；若这一步**没进记法表**，
    /// `fold_spine` 连名字都找不到 ⇒ `Exists (fun …)` 静默不折 ✗ ⇒ 整条类型退回
    /// 点形式（`exists_univ` 的实测形状 ✓）。
    #[test]
    fn notation_table_collects_binder_notation() {
        let file = crate::parse("binder_notation \"∃\" => Exists\n").expect("夹具必须能解析");
        let table = crate::notation::notation_table(&file.commands);
        let hit = table.iter().find(|decl| decl.target == "Exists");
        assert!(
            hit.is_some(),
            "`binder_notation \"∃\" => Exists` 必须进记法表；表里现有目标：{:?}",
            table.iter().map(|d| d.target.as_str()).collect::<Vec<_>>()
        );
    }
    /// **用真实管线那套建表方式折同一段文本**（2026-09-25）：夹具与真实编译
    /// 唯一还不同的地方就是"**表怎么建的**" ✓ —— 这条把它对齐：
    /// `notation_table(commands)` + **splice 内建记法** + `arities_with_prelude_from(...)`
    /// （与 `crates/front/src/compile/check/mod.rs:433-442` 逐句相同 ✓）。
    /// 折得动 ⇒ 建表方式不是差别（继续找实例差异 ✓）；折不动 ⇒ **红复现到手** ✓✓。
    #[test]
    fn folds_with_the_real_pipeline_table_shape() {
        let src = "inductive Exists (A : Type) (p : A -> Prop) : Prop\n\
                   ctor intro (w : A) (h : p w) : Exists A p\n\
                   end\n\
                   binder_notation \"∃\" => Exists\n\
                   def Set.subset (α : Type) (A B : Set α) : Prop := True\n\
                   infix:50 \" ⊆ \" => Set.subset\n";
        let file = crate::parse(src).expect("夹具必须能解析");
        let commands = &file.commands;
        let mut table = crate::notation::notation_table(commands);
        // **与修好后的管线一致**（内建兜底 ⇒ append ✓）；变体 A 里保留了
        // 旧的 `splice(0..0, …)`（内建抢先）作为**永久对照** ✓ —— 它就是 ③ 的真因。
        table.extend(crate::notation::builtin_notation_decls());
        let arities = arities_with_prelude_from(arities_in_commands(commands));
        println!(
            "[real-shape] table={} decls(target=Exists)={} arity_Exists={:?} arity_keys={:?}",
            table.len(),
            table.iter().filter(|d| d.target == "Exists").count(),
            arities.get("Exists"),
            arities
                .keys()
                .filter(|k| k.ends_with("Exists"))
                .collect::<Vec<_>>()
        );
        let dn = DisplayNotations::new(table, arities);
        let text = "forall (α : Type 0), Exists (Set α) (fun (U : Set α) => forall (A : Set α), Set.subset α A U)";
        // **判据（2026-09-25 修复后转断言 ✓）**：修前这里是**原样返回** ✗ ——
        // 内建 `=` 把 `fun … =>` 的 `=` 吃掉 ⇒ 解析失败 ⇒ 整条 bail。
        // 修在 `crates/front/src/token.rs` 的声明符号匹配处（基础多字符算符更长时让路 ✓）。
        assert_eq!(
            fold_text(text, &dn),
            "∀ (α : Type 0), ∃ (U : Set α), ∀ (A : Set α), A ⊆ U",
            "λ 操作数里的 `∃` 必须折出来（用户报的 ③：凡类型含 `fun … =>` 的声明都退化 ✗）"
        );
        // 二分：到底是 **splice 内建** 还是 **prelude arities** 让它 bail ✗
        let mut ta = crate::notation::notation_table(commands);
        ta.splice(0..0, crate::notation::builtin_notation_decls());
        let dna = DisplayNotations::new(ta, arities_in_commands(commands));
        println!(
            "[A 内建+splice / 无 prelude] out: {}",
            fold_text(text, &dna)
        );
        let tb = crate::notation::notation_table(commands);
        let dnb =
            DisplayNotations::new(tb, arities_with_prelude_from(arities_in_commands(commands)));
        println!("[B 无内建 / 有 prelude] out: {}", fold_text(text, &dnb));
        let tc = crate::notation::notation_table(commands);
        let dnc = DisplayNotations::new(tc, arities_in_commands(commands));
        println!("[C 无内建 / 无 prelude] out: {}", fold_text(text, &dnc));
        // **机械二分**：一次加一条内建，看哪一条一加就 bail ✓（不再猜 ✗）
        let builtins = crate::notation::builtin_notation_decls();
        println!("[bisect] builtins 共 {} 条", builtins.len());
        for (i, b) in builtins.iter().enumerate() {
            let mut t = crate::notation::notation_table(commands);
            t.push(b.clone());
            let dn_i = DisplayNotations::new(t, arities_in_commands(commands));
            let out = fold_text(text, &dn_i);
            println!(
                "[bisect] +{:>2} target={:<14} symbol={:<4} folded={}",
                i,
                b.target,
                b.symbol,
                out != text
            );
        }
    }

    /// **③ 的窄到宽探针**（2026-09-25）：先**打印**真实折叠结果，再写断言 ✓
    /// （不猜期望值 ✗ —— 前面几轮猜的代价已经够大了）。
    #[test]
    fn narrow_to_wide_binder_fold_probe() {
        let dn = notations(
            "binder_notation \"∃\" => Exists\n",
            &[("Exists", 2), ("Set.subset", 3)],
        );
        // ① 最窄：只有 `Exists` 一层
        let a = "Exists (Set α) (fun (U : Set α) => Set.subset α A U)";
        // ② 中层：外层 `forall`
        let b = "forall (α : Type 0) (A : Set α), Set.subset α A U";
        // ③ 真实形状（unit11 的 `exists_univ`，逐字来自 pp 的原始输出）
        let c = "forall (α : Type 0), Exists (Set α) (fun (U : Set α) => forall (A : Set α), Set.subset α A U)";
        // ④ **多行**输入（真实 pp 输出里就有换行；我 trace 时把 `\n` 换成了空格 ✗）
        let d = "forall (α : Type 0),\nExists (Set α) (fun (U : Set α) => forall (A : Set α), Set.subset α A U)";
        // ⑤ 逐字复刻 `no_univ_strictly_larger` 的 pp 原文（含换行与 `Not`/`And`/`Ne`）
        let e = "forall (α : Type 0),\nNot (Exists (Set α) (fun (U : Set α) => forall (A : Set α), And (Set.subset α A U) (Ne (Set α) A U)))";
        for (label, text) in [("①窄", a), ("②中", b), ("③整", c), ("④多行", d), ("⑤No", e)]
        {
            println!("[probe {label}] in : {text}");
            println!("[probe {label}] out: {}", fold_text(text, &dn));
        }
    }
}

// ── **作用域折叠缓存**（第 100 轮 · 平行线）────────────────────────────────────────
//
// **为什么**：`fold` = `print_back(text, self)` ⇒ 逐条声明折一遍 ✓；实测（第 88 轮的控制实验：
// 同款长类型、13 条）**有 goal（要折）1.11ms vs 无 goal（不折）0.44ms** ⇒ 折叠 ≈0.67ms/13 条 ✗。
// `soko/goals` 每次请求都对**每条开放声明**折一次 ✓，而这些文本在稳态下**逐字不变** ✓
// ⇒ 一次作用域内的缓存能把这一笔吃回来 ✓。
//
// **为什么是作用域而不是全局 LRU**：`fold` 的输入除了 `text` 还有**记法表**（`self` ✓），而
// `DisplayNotations{table, arity}` **没有便宜的指纹** ✗ ⇒ 只按文本做键会在**换表**时给错答案 ✗
// （第 70 轮就是因此**没落地** ✓）。作用域内由调用方保证"**同一张表**" ✓（LSP 的
// `goal_decls` 里就是一份 doc 的一张表 ✓），并在**退出时清空** ✓ ⇒ 键只按 `text` 也**成立** ✓。
thread_local! {
    static FOLD_CACHE: std::cell::RefCell<Option<std::collections::HashMap<String, String>>> =
        const { std::cell::RefCell::new(None) };
}

/// **在作用域内缓存 `fold` 的结果**（调用方保证：作用域内 `fold` 用的都是**同一张记法表** ✓）。
///
/// 语义：命中 ⇒ 返回缓存 ✓（与重算**逐字节相同** ✓，判据 `crates/front/tests/fold_cache_parity.rs` ✓）；
/// 未开作用域/未命中 ⇒ 照旧算 ✓ ⇒ **不开作用域时行为一字不变** ✓。
pub fn with_fold_cache<T>(f: impl FnOnce() -> T) -> T {
    struct Clear;
    impl Drop for Clear {
        fn drop(&mut self) {
            FOLD_CACHE.with(|c| *c.borrow_mut() = None);
        }
    }
    FOLD_CACHE.with(|c| *c.borrow_mut() = Some(std::collections::HashMap::new()));
    let _clear = Clear;
    f()
}

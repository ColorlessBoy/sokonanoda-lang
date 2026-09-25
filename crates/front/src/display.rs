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
//   * **只有 `spine.len() == arity` 才是记法实例**（§3.2 硬规则）：
//     `Set.mem α a`（部分应用）不许打成 `α ∈ a`。
//   * **产物是 [`DisplayText`]**：它进不了任何回读通道（编译期保证）。

use crate::ast::{Binder, Expr, MatchArm, NotationAssoc, NotationDecl};
use crate::Span;
use std::collections::HashMap;

/// 显示期的记法表（设计 §3.3）：记法声明 + 「target 点名 → 元数」。
///
/// **元数**（arity）= 目标声明的 telescope 层数。只有 `spine.len() == arity`
/// 才是记法实例——`Set.mem α a` 是**部分应用**，折成 `α ∈ a` 就是显示错误。
/// 元数的来源是 T-C11；这里只**消费**它，好让折叠层能被单独测。
#[derive(Debug, Clone, Default)]
pub struct DisplayNotations {
    table: Vec<NotationDecl>,
    arity: HashMap<String, usize>,
}

impl DisplayNotations {
    pub fn new(table: Vec<NotationDecl>, arity: HashMap<String, usize>) -> Self {
        Self { table, arity }
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
        Self { table, arity }
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
    // F2/F4：pp 会折行；`@Eq.{u, v}` 的 head 是 `UniverseApp`。parser 都容忍，
    // 解析不了就原样返回。
    let Ok(ast) = crate::proof::parse_expr_text_with(text, &notations.table) else {
        return DisplayText::new(text);
    };
    // `parse_expr_text_with` 解析的是 `"#check " + text` ⇒ AST 的 span 比 `text`
    // 多一个前缀。**反推**这个偏移（而不是硬编码 `"#check ".len()`）：`text` 里
    // 第一个非空白字符的位置就是表达式该在的位置。
    let lead = text.len() - text.trim_start().len();
    let Some(base) = ast.span().start.offset.checked_sub(lead) else {
        return DisplayText::new(text);
    };
    let mut edits: Vec<(Span, String)> = Vec::new();
    let _ = fold_collecting(ast, notations, &mut edits, text, base);
    if edits.is_empty() {
        return DisplayText::new(text);
    }
    match splice(text, base, edits) {
        Some(out) => DisplayText::new(out),
        // span 换算越界（解析器换了前缀形状之类）⇒ **原样返回**，绝不乱切。
        None => DisplayText::new(text),
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
    fold_collecting_inner(expr, dn, edits, false, src, base).0
}

/// 同 [`fold_collecting`]，另外回报"这棵子树变了没有"。
///
/// **为什么需要"变了没有"**：折过的子树如果**被应用**（它的父节点还是 `App`），
/// 就地替换会拼出**重解析成另一个 AST** 的文本——
/// `(Set.mem α a A) B` 的 `Set.mem α a A` 是完整的三元应用，就地换成 `a ∈ A`
/// 就得到 `a ∈ A B`，重新解析是 `Set.mem α a (A B)`（**换了个意思**）。
/// 规则：那种情况下把替换范围**上提到这条应用脊的根**，整条脊交给
/// `render_expr` 渲染——括号归它管，它本来就是干这个的。实测得到 `(a ∈ A) B` ✓
///
/// 只上提到 `App`：`Set.mem α a A -> P` 的父节点是 `Arrow`（不是应用），
/// 就地换是安全的（`∈` 比 `->` 紧，重解析一致）⇒ 保留原文的其它部分。
///
/// **`in_spine` 是性能开关**（不是正确性开关）：只让**最外层**那条应用脊记一次。
/// 不传它的话，折点之上的**每一层** `App` 祖先都会 `render_expr` 一遍整棵子树
/// ——实测 `did_open` 因此退化 **+18~22%**（`lsp-course` 三档全中），改回 O(1) 次。
/// 记多次也不影响结果（[`splice`] 只取最外层），纯粹是白烧。
fn fold_collecting_inner(
    expr: Expr,
    dn: &DisplayNotations,
    edits: &mut Vec<(Span, String)>,
    in_spine: bool,
    src: &str,
    base: usize,
) -> (Expr, bool) {
    let is_app = matches!(expr, Expr::App { .. });
    let mut child_changed = false;
    let expr = map_children_with(expr, &mut |e| {
        let (out, changed) = fold_collecting_inner(e, dn, edits, is_app, src, base);
        child_changed |= changed;
        out
    });
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
    if let Some(folded) = fold_spine(&expr, dn) {
        edits.push((folded.span(), crate::proof::render_expr(&folded)));
        return (folded, true);
    }
    if child_changed && is_app && !in_spine {
        edits.push((expr.span(), crate::proof::render_expr(&expr)));
        return (expr, true);
    }
    (expr, child_changed)
}

/// 这一层是不是一条记法实例？是就换成 [`Expr::Notation`]，否则 `None`。
///
/// **第一刀只做二元 infix 族**（`Infix`/`Infixl`/`Infixr`）——一元前缀/后缀与
/// binder 记法的折叠留给后续环节；它们的 `arity` 与操作数位不同（一元 1 个、
/// binder 2 个且第 2 个是 lambda），一起做会把这一刀撑大。
fn fold_spine(expr: &Expr, dn: &DisplayNotations) -> Option<Expr> {
    let (head, args) = crate::spine::spine_of(expr);
    let name = head_name(head)?;
    let arity = *dn.arity.get(name)?;
    // §3.2 硬规则：只有完全应用才是记法实例。
    if args.len() != arity {
        return None;
    }
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
    // 前导实参（`Set.mem α a A` 里的 `α`）**丢掉**：它们是展开时补上的隐式
    // 类型参数，源里本来就不写。
    let operands = &args[arity - operand_count..];
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
// 设计 §3.2 的硬规则要一个数：**目标声明的 telescope 层数**（= 完全应用时
// `spine.len()`）。有了它才能判断"这是记法实例"还是"部分应用"：
// `Set.mem α a A`（3 = 3）⇒ `a ∈ A`；`Set.mem α a`（2 ≠ 3）⇒ 原样。
//
// **口径先对齐**（两处容易混）：
//   * **telescope**（= 本模块的 `arity`）= 声明类型上剥出来的 binder 总数
//     （`def Set.mem (α : Type) (a : α) (A : Set α) : Prop` ⇒ **3**）。
//     它对应 `spine.len()`。
//   * **操作数个数** = 记法自己写出来的位置（二元 infix ⇒ **2**）。
//   两者之差 = **前导参数**（`∈` 的 `α`），折叠时丢掉。
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

/// 折叠层要的**元数**：pp 文本里**完全应用**时会出现几个实参。
///
/// **等于「显式 binder 的个数」**——不是 telescope 层数。为什么：内核 pp
/// **会省略隐式参数**，而不会省略显式参数。
///
/// | 目标 | 源级签名 | telescope | 元数 | pp 形态 |
/// |---|---|---|---|---|
/// | `Set.mem` | `(α : Type) (a : α) (A : Set α)` | 3 | **3** | `Set.mem α a A`（实测） |
/// | `Eq` | `{α : Sort u} (a : α) (b : α)` | 3 | **2** | `Eq A B`（实测，隐式 `α` 被省） |
/// | `And` | `(a b : Prop)` | 2 | **2** | `And p q` |
/// | `Not` | `(A : Prop)` | 1 | **1** | `Not p` |
///
/// 用 telescope 层数会在 `Eq` 上直接失效（pp 给 2 个实参、telescope 是 3 ⇒ 永远
/// 判成"部分应用"、`=` 永远折不出来）——这是接进生产者（T-C20）时实测撞到的。
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
pub fn arities_in_commands(commands: &[crate::ast::Command]) -> HashMap<String, usize> {
    let mut out = HashMap::new();
    for command in commands {
        match command {
            crate::ast::Command::Def { name, ty, .. }
            | crate::ast::Command::Theorem { name, ty, .. }
            | crate::ast::Command::Axiom { name, ty, .. } => {
                out.insert(name.clone(), explicit_arity(ty));
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

/// prelude 里那些记法目标（`And` / `Or` / `Not` / `Iff` / `Eq`）的 telescope 层数。
///
/// **parse 一次就缓存**：它在**每次编译的出口**都要用（每个声明一次），而 prelude
/// 源码是常量。线 C 的四个生产者都会经过它。
pub fn prelude_arities() -> &'static HashMap<String, usize> {
    static CACHE: std::sync::OnceLock<HashMap<String, usize>> = std::sync::OnceLock::new();
    CACHE.get_or_init(|| {
        arities_in_sources(&[
            crate::compile::PRELUDE_EQ_SRC,
            crate::compile::PRELUDE_L1_SRC,
        ])
    })
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notation::notation_table;

    /// 夹具：把一段**记法声明**源码解析成表，再按给定元数建 [`DisplayNotations`]。
    ///
    /// 元数在 T-C11 之前由测试**显式给**——折叠层只消费它，好被单独测。
    fn notations(src: &str, arities: &[(&str, usize)]) -> DisplayNotations {
        let file = crate::parse(src).expect("夹具必须能解析");
        let table = notation_table(&file.commands);
        let map: HashMap<String, usize> = arities
            .iter()
            .map(|(n, a)| ((*n).to_string(), *a))
            .collect();
        DisplayNotations::new(table, map)
    }

    // ---- 元数的来源（T-C11）--------------------------------------------

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

    /// **元数 = 显式 binder 的个数**，不是 telescope 层数——`Eq` 是那条判据。
    ///
    /// `axiom Eq {u} : {α : Sort u} -> α -> α -> Prop` 的 telescope 是 3，但内核 pp
    /// **省略隐式参数** ⇒ 打出来是 `Eq A B`（**2** 个实参）。用 telescope 当元数
    /// 会把 `Eq A B` 永远判成"部分应用"、`=` 永远折不出来（接进生产者时实测撞到）。
    #[test]
    fn arity_counts_explicit_binders_not_the_whole_telescope() {
        let arities = arities_with_prelude(&[]);
        assert_eq!(
            arities.get("Eq").copied(),
            Some(2),
            "`Eq A B`（隐式 α 被 pp 省掉）"
        );
        // `Ne` 反过来：prelude 里它的 `α` 是**显式**的
        // （`def Ne {u} (α : Sort u) (a b : α)`）⇒ 元数 3，pp 也写 `Ne α a b`。
        // 这一对（`Eq` 2 / `Ne` 3）正好把"显式 binder 个数"这条规则钉死。
        assert_eq!(arities.get("Ne").copied(), Some(3));
        // 全是显式 binder 的目标：两者相同。
        assert_eq!(arities.get("And").copied(), Some(2));
        assert_eq!(arities.get("Not").copied(), Some(1));
    }

    /// **内建记法要自己补**：`↔`/`∧`/`∨`/`¬`/`=`/`≠` 不在任何源文本里
    /// （parser 有硬编码表），"从源里收记法"收不到它们。
    #[test]
    fn builtin_notations_fold_too() {
        let file = crate::parse(SET_LIB).expect("夹具必须能解析");
        let mut table = notation_table(&file.commands);
        table.splice(0..0, crate::notation::builtin_notation_decls());
        let dn = DisplayNotations::new(table, arities_with_prelude(&[SET_LIB]));
        assert_eq!(
            fold_text("Iff (Set.subset α A B) (Set.subset α A B)", &dn),
            "(A ⊆ B) ↔ (A ⊆ B)"
        );
        assert_eq!(fold_text("And p q", &dn), "p ∧ q");
        assert_eq!(fold_text("Eq A B", &dn), "A = B");
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
    /// （多给一个）折成 `(a ∈ A) B` 更荒谬。两条都必须原样。
    #[test]
    fn partial_and_over_application_fall_back() {
        let dn = notations(SET_LIB, SET_ARITY);
        assert_eq!(fold_text("Set.mem α a", &dn), "Set.mem α a");
        // **折过的子树被应用**时，替换范围上提到应用脊根，括号交给 `render_expr`：
        // `(Set.mem α a A) B` 折成 `(a ∈ A) B`——**不是** `a ∈ A B`
        // （后者重新解析是 `Set.mem α a (A B)`，换了个意思）。
        assert_eq!(fold_text("Set.mem α a A B", &dn), "(a ∈ A) B");
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

    fn fold_text(text: &str, dn: &DisplayNotations) -> String {
        print_back(text, dn).as_display_str().to_string()
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
            "∀ (α : Type 0) (A B : Set α), A ⊆ B -> B ⊆ A",
            "binder 分组与 `Type 0` 必须原样，只换记法"
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
    /// ⇒ **两处看起来都对，`Exists` 却仍然不在表里** ✗ —— 下一个动作**不是**改代码，
    ///   而是**量真实管线的那张表**（下面的第二个测试就是那个量具 ✓）。
    #[test]
    fn prelude_arities_cover_their_own_targets() {
        let a = prelude_arities();
        for (name, want) in [("And", 2), ("Or", 2), ("Not", 1), ("Iff", 2), ("Eq", 2)] {
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
            arities.keys().filter(|k| k.ends_with("Exists")).collect::<Vec<_>>()
        );
        let dn = DisplayNotations::new(table, arities);
        let text = "forall (α : Type 0), Exists (Set α) (fun (U : Set α) => forall (A : Set α), Set.subset α A U)";
        println!("[real-shape] out: {}", fold_text(text, &dn));
        // 二分：到底是 **splice 内建** 还是 **prelude arities** 让它 bail ✗
        let mut ta = crate::notation::notation_table(commands);
        ta.splice(0..0, crate::notation::builtin_notation_decls());
        let dna = DisplayNotations::new(ta, arities_in_commands(commands));
        println!("[A 内建+splice / 无 prelude] out: {}", fold_text(text, &dna));
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

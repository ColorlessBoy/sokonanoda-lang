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
    let _ = fold_collecting(ast, notations, &mut edits);
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
    ranges.sort_by_key(|(range, _)| range.start);
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
/// 记进 `edits`（`(被折那段的 span, 折出来的文本)`）。（`(被折那段的 span, 折出来的文本)`）。
///
/// 内外都记；[`splice`] 只取最外层的那些。
fn fold_collecting(expr: Expr, dn: &DisplayNotations, edits: &mut Vec<(Span, String)>) -> Expr {
    let expr = map_children_collecting(expr, dn, edits);
    match fold_spine(&expr, dn) {
        Some(folded) => {
            edits.push((folded.span(), crate::proof::render_expr(&folded)));
            folded
        }
        None => expr,
    }
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
    if !matches!(
        decl.assoc,
        NotationAssoc::Infix | NotationAssoc::Infixl | NotationAssoc::Infixr
    ) {
        return None;
    }
    // 前导实参（`Set.mem α a A` 里的 `α`）**丢掉**：它们是展开时补上的隐式
    // 类型参数，源里本来就不写。
    let operands = &args[arity - 2..];
    Some(Expr::Notation {
        symbol: decl.symbol.clone(),
        target: decl.target.clone(),
        assoc: decl.assoc,
        lhs: Some(Box::new(operands[0].clone())),
        rhs: Some(Box::new(operands[1].clone())),
        // 折叠出来的是**唯一的**写法（head 名字就是判据），没有候选列表。
        alternatives: Vec::new(),
        span: expr.span(),
    })
}

/// spine 的头是不是一个可以当记法目标的名字（`Ident` 或 `UniverseApp`）。
fn head_name(head: &Expr) -> Option<&str> {
    match head {
        Expr::Ident { name, .. } | Expr::UniverseApp { name, .. } => Some(name.as_str()),
        _ => None,
    }
}

/// 折 + 把每处折叠记进 `edits`。
fn map_children_collecting(
    expr: Expr,
    dn: &DisplayNotations,
    edits: &mut Vec<(Span, String)>,
) -> Expr {
    map_children_with(expr, &mut |e| fold_collecting(e, dn, edits))
}

/// 递归到所有子项。**列全每一个变体**——漏一个位置的后果是"那里不折"
/// （安全但会让同一份文本里折一半），比"折错"好，但不该有。
///
/// 参数化成一个 `f`（而不是直接调 [`fold`]）是为了让"只折"与"折 + 记下替换"
/// **共用同一份结构知识**：两份手写的 16 变体匹配迟早会漂。
fn map_children_with(expr: Expr, f: &mut impl FnMut(Expr) -> Expr) -> Expr {
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
        } => Expr::Notation {
            symbol,
            target,
            assoc,
            lhs: lhs.map(|e| Box::new(f(*e))),
            rhs: rhs.map(|e| Box::new(f(*e))),
            alternatives,
            span,
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
        // 一元前缀（`¬`）**还折不了**：第一刀只做二元 infix 族（T-C10 的范围），
        // 前缀/后缀的操作数位不同，归后续环节。
        assert_eq!(fold_text("Not p", &dn), "Not p");
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
            "forall (x : α), x ∈ (A ∪ B)"
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
            "forall (α : Type 0) (A B : Set α), A ⊆ B -> B ⊆ A",
            "binder 分组与 `Type 0` 必须原样，只换记法"
        );
        // 折行与缩进也保留（pp 的长签名会折行）。
        assert_eq!(
            fold_text("forall (α : Type 0),\n  Set.mem α a A", &dn),
            "forall (α : Type 0),\n  a ∈ A"
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
            "forall (α : Type 0) (A B : Set α), Other.thing α A B -> Other.thing α B A",
            // 折行 + 缩进也保留。
            "forall (α : Type 0) (A B : Set α),\n  Other.thing α A B",
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
}

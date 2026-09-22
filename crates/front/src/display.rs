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
    let folded = fold(ast.clone(), notations);
    // **一处都没折 ⇒ 逐字节原样返回**。
    //
    // 这条性质比"折对了"更要紧：`render_expr` 的产物是**回读通道的输入**，所以它
    // 会把内核 pp 的 `forall (a b : T), …` **拆成箭头链** `(a : T) -> (b : T) -> …`
    // （`proof.rs` 里那段注释解释了为什么必须这样）。如果无条件重渲染，那么**每一条
    // 不带记法的类型**都会跟着改样子——那不是用户要的（他要的是记法），也是本可以
    // 避免的显示漂移。有了这一行：**没有记法的文本一个字节都不动**。
    if folded == ast {
        return DisplayText::new(text);
    }
    DisplayText::new(crate::proof::render_expr(&folded))
}

/// 自底向上折：先把子项折好，父项才有机会看到已经折好的操作数。
fn fold(expr: Expr, dn: &DisplayNotations) -> Expr {
    let expr = map_children(expr, dn);
    fold_spine(&expr, dn).unwrap_or(expr)
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

/// 递归到所有子项。**列全每一个变体**——漏一个位置的后果是"那里不折"
/// （安全但会让同一份文本里折一半），比"折错"好，但不该有。
fn map_children(expr: Expr, dn: &DisplayNotations) -> Expr {
    match expr {
        Expr::App {
            fun,
            arg,
            explicit_spine,
            span,
        } => Expr::App {
            fun: Box::new(fold(*fun, dn)),
            arg: Box::new(fold(*arg, dn)),
            explicit_spine,
            span,
        },
        Expr::Lambda {
            binders,
            body,
            span,
        } => Expr::Lambda {
            binders: map_binders(binders, dn),
            body: Box::new(fold(*body, dn)),
            span,
        },
        Expr::Forall {
            binders,
            body,
            span,
        } => Expr::Forall {
            binders: map_binders(binders, dn),
            body: Box::new(fold(*body, dn)),
            span,
        },
        Expr::Arrow {
            domain,
            codomain,
            span,
        } => Expr::Arrow {
            domain: Box::new(fold(*domain, dn)),
            codomain: Box::new(fold(*codomain, dn)),
            span,
        },
        Expr::Plus { lhs, rhs, span } => Expr::Plus {
            lhs: Box::new(fold(*lhs, dn)),
            rhs: Box::new(fold(*rhs, dn)),
            span,
        },
        Expr::Let {
            binder,
            val,
            body,
            span,
        } => Expr::Let {
            binder: map_binder(binder, dn),
            val: Box::new(fold(*val, dn)),
            body: Box::new(fold(*body, dn)),
            span,
        },
        Expr::Match {
            scrutinee,
            arms,
            span,
        } => Expr::Match {
            scrutinee: Box::new(fold(*scrutinee, dn)),
            arms: arms
                .into_iter()
                .map(|arm| MatchArm {
                    pattern: arm.pattern,
                    guard: arm.guard.map(|g| fold(g, dn)),
                    body: fold(arm.body, dn),
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
            lhs: lhs.map(|e| Box::new(fold(*e, dn))),
            rhs: rhs.map(|e| Box::new(fold(*e, dn))),
            alternatives,
            span,
        },
        Expr::SetLiteral { elements, span } => Expr::SetLiteral {
            elements: elements.into_iter().map(|e| fold(e, dn)).collect(),
            span,
        },
        Expr::AnonCtor { elements, span } => Expr::AnonCtor {
            elements: elements.into_iter().map(|e| fold(e, dn)).collect(),
            span,
        },
        // 叶子（没有子项）与 `By`（tactic 脚本，不在这里展开）。
        leaf => leaf,
    }
}

fn map_binders(binders: Vec<Binder>, dn: &DisplayNotations) -> Vec<Binder> {
    binders.into_iter().map(|b| map_binder(b, dn)).collect()
}

fn map_binder(binder: Binder, dn: &DisplayNotations) -> Binder {
    Binder {
        name: binder.name,
        ty: binder.ty.map(|t| Box::new(fold(*t, dn))),
        style: binder.style,
        span: binder.span,
    }
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
        // binder **体里**也折（`map_children` 走遍每个变体）。
        //
        // ⚠ 这里同时钉住一条**超出记法的后果**：一旦发生了折叠，产物要经过
        // `render_expr` 重渲染，而它把 `forall (x : T), …` 拆成
        // `(x : T) -> …`（那是它作为**回读输入**的要求，见 `proof.rs`）。
        // ⇒ "有记法的文本会连带换一种 binder 写法"。**没有记法的文本不会**
        // ——下一条测试钉住那个性质。要不要在显示出口保留 `forall` 分组，
        // 是 T-C20（接进生产者）时要拍的决定。
        assert_eq!(
            fold_text("forall (x : α), Set.mem α x (Set.union α A B)", &dn),
            "(x : α) -> x ∈ (A ∪ B)"
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

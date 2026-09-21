//! 隐式实参（IA-1，路线 C：**风格对齐 + 唯一确定**）。
//!
//! 设计 `docs/design/implicit-arguments.md`（§3 是路线 C 的算法，§0 是安全性质）。
//!
//! 一句话：**显式实参按「风格」对齐到显式层、跳过隐式层**；被跳过的层由
//! 「后续显式实参的类型 + 整体期望类型」的头部匹配**唯一确定**；解不出就报
//! 专用错误码（`elab-implicit-argument-unsolved`），**不猜、不搜索、不回溯**。
//!
//! **安全性质**（设计 §0，P1 可独立发布的依据）：签名里没有隐式 binder 时，
//! 本模块的入口在调用方就返回 `None`，行为与今天的 `mk_app` 链**逐字节相同**。
//! `crates/cli/tests/notation.rs` 的
//! `no_course_signature_uses_an_implicit_binder` 把「课程签名今天没有隐式
//! binder」钉死——否则这条安全性质会随课程改写**悄悄失效**（设计 §0 的注）。
//!
//! 内核零改动：`BinderStyle` 在内核里只是 pp 风格，隐式 binder 与显式 binder
//! 在内核里都是**真 Pi**（`crates/kernel` 的注释自述「只被 pp 使用，不改变类型
//! 检查」）⇒ 前端把解出来的隐式实参当**普通实参**喂进 `mk_app` 即可。

use crate::ast::{BinderKind, Expr};

/// 签名的一层：名字 / 域 / 风格。
pub(crate) struct Layer {
    pub name: String,
    pub domain: Expr,
    /// 源级风格。**应用路径现在用注册表的 `implicit_prefix` 而不是它**（构造子
    /// 的参数在 kernel 类型里显式、语义上隐式）；保留它是为了让 `telescope` 的
    /// 单测仍能钉住「风格活着从签名文本里出来」。
    #[allow(dead_code)]
    pub style: BinderKind,
}

/// 把 `judge_infer` 给的签名文本拆成「带风格的 Pi 层 + 结果类型」。
///
/// 与 `elab::notation_telescope` 同一份解析（`parse_expr_text` + 逐层剥 Pi），
/// 只多带一个 `style`：内核 pp 把隐式 binder 打成 `{α : Type}`，而
/// `spine::peel_pi` 把风格丢了——路线 C 全靠它。多 binder 折叠
/// （`forall (a b : T), …`）与 `->`（匿名的显式层）都要认。
///
/// **参数名一律换成 fresh 名**（`\0soko_p0`…，NUL 前缀不可能出现在源码标识符里），
/// 并把域/结果里对**外层参数名**的引用一起代换。理由：求解器是**按名字**代换的
/// （[`solve_prefix`] 的 `sigma`），签名参数名一旦与用户变量同名就会**捕获**——
/// `congrArg {α β} … (f : α → β)` 在 `theorem t (α β : Type) (g : β → α) … :=
/// congrArg.{1} g hxy` 里把 `β` 解成用户的 `β` 而不是 `α`（实测判红）。fresh 名
/// 让模板与用户表达式永不撞名。
pub(crate) fn telescope(signature: &str) -> Option<(Vec<Layer>, Expr)> {
    let mut cur = crate::proof::parse_expr_text(signature).ok()?;
    let mut layers: Vec<Layer> = Vec::new();
    let mut sigma: std::collections::HashMap<String, Expr> = std::collections::HashMap::new();
    let mut fresh_id = 0usize;
    loop {
        match cur {
            Expr::Forall { binders, body, .. } if !binders.is_empty() => {
                for b in &binders {
                    let domain = crate::spine::substitute(b.ty.as_deref()?, &sigma);
                    let fresh = format!("\u{0}soko_p{fresh_id}");
                    fresh_id += 1;
                    sigma.insert(
                        b.name.clone(),
                        Expr::Ident {
                            name: fresh.clone(),
                            span: b.span,
                        },
                    );
                    layers.push(Layer {
                        name: fresh,
                        domain,
                        style: b.style.clone(),
                    });
                }
                cur = *body;
            }
            Expr::Arrow {
                domain, codomain, ..
            } => {
                layers.push(Layer {
                    name: String::new(),
                    domain: crate::spine::substitute(&domain, &sigma),
                    style: BinderKind::Explicit,
                });
                cur = *codomain;
            }
            other => return Some((layers, crate::spine::substitute(&other, &sigma))),
        }
    }
}

/// 前导隐式层的个数（`0` ⇒ 调用方走今天的老路）。
///
/// 应用路径改用注册表的 `implicit_prefix`（构造子的参数在 pp 里是显式的）；
/// 这个函数只被 `implicit.rs` 的单测用来钉住 `telescope` 的风格解析。
#[allow(dead_code)]
pub(crate) fn leading_implicit(layers: &[Layer]) -> usize {
    layers
        .iter()
        .take_while(|l| l.style == BinderKind::Implicit)
        .count()
}

/// 解出被跳过的前导 `k` 层。
///
/// **路线 ①（由后续显式实参的类型反解）**：`arg_tys[i]` = 第 `k + i` 层那个
/// 实参的**类型**（`None` = 拿不到）。对第 `i` 个待解的隐式层，找**后面第一个**
/// 提到它的层 `j`，把 `layers[j].domain`（已解出的参数先代进去）与
/// `arg_tys[j - k]` 头部匹配：
///   · `Set.mem {α} (a : α) …` ⇒ `α` 出现在第 `k` 层（`a : α`）⇒ 由 `a` 的类型解出；
///   · `Set.image {α β} (f : α → β) …` ⇒ `α`/`β` 都在第 `k` 层 ⇒ 由 `f` 的类型解出；
///   · `picks {α} (n : Nat) (b : α)` 调用 `picks 3 b` ⇒ `α` 在第 `k+1` 层
///     （`b : α`）⇒ 由 `b` 的类型解出（**扫全部显式层**，不只第一层）。
///
/// **路线 ②（由期望类型反解）**：参数只出现在**结果类型**里时（`Or.inl` 的 `B`、
/// `False.elim`/`absurd` 的结论、`Eq.refl` 的 `α`），把已解出的参数代进 `result`
/// 再与 `expected` 头部匹配。与记法路径的 `solve_prefix_args` 同一条机械。
///
/// 两条路都解不出 ⇒ `None`（调用方报专用错误码，**不猜**）。
///
/// 两条路都带**期望类型/实参类型的 delta 展开兜底**：`Or.inl h` 的目标常写成
/// `a ∈ A ∪ B`（`Set.mem … (Set.union …)`，两层 `def`），`And.left h` 的 `h`
/// 常写成 `a ∈ A ∩ B`——不展开到 `Or`/`And` 归纳头就匹配不上。
pub(crate) fn solve_prefix(
    layers: &[Layer],
    result: &Expr,
    k: usize,
    arg_tys: &[Option<Expr>],
    expected: Option<&Expr>,
    defs: &crate::compile::elab::DefTable,
    is_inductive: &dyn Fn(&str) -> bool,
) -> Option<Vec<Expr>> {
    let unfold = |e: &Expr| crate::spine::unfold_to_inductive(e, is_inductive, defs, 8, None);
    let mut solved: Vec<Expr> = Vec::with_capacity(k);
    for i in 0..k {
        let name = layers[i].name.clone();
        if name.is_empty() {
            return None;
        }
        let mut sigma: std::collections::HashMap<String, Expr> = std::collections::HashMap::new();
        for (j, s) in solved.iter().enumerate() {
            if !layers[j].name.is_empty() {
                sigma.insert(layers[j].name.clone(), s.clone());
            }
        }
        let mut found: Option<Expr> = None;
        // 路线 ①：后续显式层的域 vs 该层实参的类型。
        for (j, layer) in layers.iter().enumerate().skip(i + 1) {
            // `j < k` 的层也是隐式的（还没解出，也没有实参可问）。
            let Some(actual) = j
                .checked_sub(k)
                .and_then(|x| arg_tys.get(x))
                .and_then(|t| t.as_ref())
            else {
                continue;
            };
            let domain = crate::spine::substitute(&layer.domain, &sigma);
            if !crate::spine::mentions(&name, &domain) {
                continue;
            }
            let mut v = super::elab::unify_extract(&domain, actual, &name);
            if v.is_none() {
                let unfolded = unfold(actual);
                if &unfolded != actual {
                    v = super::elab::unify_extract(&domain, &unfolded, &name);
                }
            }
            if let Some(v) = v {
                found = Some(v);
                break;
            }
        }
        // 路线 ②：期望类型 vs 已代换的结果类型。
        if found.is_none() {
            if let Some(expected) = expected {
                let template = crate::spine::substitute(result, &sigma);
                found = super::elab::unify_extract(&template, expected, &name);
                if found.is_none() {
                    let unfolded = unfold(expected);
                    if &unfolded != expected {
                        found = super::elab::unify_extract(&template, &unfolded, &name);
                    }
                }
            }
        }
        solved.push(found?);
    }
    Some(solved)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_defs() -> crate::compile::elab::DefTable {
        crate::compile::elab::DefTable::new()
    }

    /// 风格必须活着从签名文本里出来（`{}` = 隐式、`()`/`->` = 显式）。
    #[test]
    fn telescope_keeps_binder_styles() {
        let (layers, result) = telescope("forall {α : Type 0}, (a : α) -> (A : α -> Prop) -> Prop")
            .expect("telescope");
        assert_eq!(layers.len(), 3);
        // 参数名是 fresh 名（防捕获），域里对 `α` 的引用也换成了 fresh 名。
        assert!(layers[0].name.starts_with('\u{0}'));
        assert!(matches!(&layers[1].domain, Expr::Ident { name, .. } if name == &layers[0].name));
        assert_eq!(layers[0].style, BinderKind::Implicit);
        assert_eq!(layers[1].style, BinderKind::Explicit);
        assert_eq!(layers[2].style, BinderKind::Explicit);
        assert!(
            matches!(result, Expr::Sort { .. }),
            "`Prop` 是 Sort，不是 Ident: {result:?}"
        );
        assert_eq!(leading_implicit(&layers), 1);
    }

    /// `(a b : T)` 的多 binder 折叠要逐层摊开，`->` 算匿名的显式层。
    #[test]
    fn telescope_unfolds_multi_binder_foralls_and_arrows() {
        // `Nat -> Nat` 是**一支箭头**（域 Nat、陪域 Nat），不是两支。
        let (layers, _) = telescope("forall (a b : Nat), Nat -> Nat").expect("telescope");
        assert_eq!(layers.len(), 3);
        assert!(layers[0].name.starts_with('\u{0}'));
        assert!(layers[1].name.starts_with('\u{0}'));
        assert_ne!(layers[0].name, layers[1].name);
        assert_eq!(layers[2].name, "");
        assert_eq!(layers[2].style, BinderKind::Explicit);
        assert_eq!(leading_implicit(&layers), 0);
    }

    /// 中间隐式（`(a) {β} (b)`）：前导隐式是 0 ⇒ 调用方走老路。
    #[test]
    fn leading_implicit_stops_at_the_first_explicit_layer() {
        let (layers, _) =
            telescope("forall (a : Nat) {β : Type 0} (b : β), Prop").expect("telescope");
        assert_eq!(leading_implicit(&layers), 0);
        let (layers, _) =
            telescope("forall {α : Type 0} (a : α) {β : Type 0} (b : β), Prop").expect("telescope");
        assert_eq!(leading_implicit(&layers), 1);
    }

    /// 路线 ①：`Set.mem {α} (a : α) …` 的 `α` 由第一个显式实参的类型解出。
    #[test]
    fn solve_prefix_reads_the_first_explicit_layer() {
        let (layers, _) = telescope("forall {α : Type 0}, (a : α) -> (A : α -> Prop) -> Prop")
            .expect("telescope");
        let actual_ty = crate::proof::parse_expr_text("Nat").expect("parse");
        let result = crate::proof::parse_expr_text("Prop").expect("parse");
        let solved = solve_prefix(
            &layers,
            &result,
            1,
            &[Some(actual_ty)],
            None,
            &empty_defs(),
            &|_| false,
        )
        .expect("solved");
        assert_eq!(solved.len(), 1);
        assert!(matches!(&solved[0], Expr::Ident { name, .. } if name == "Nat"));
    }

    /// 解不出的形状必须返回 `None`（调用方报专用码），**不许猜**。
    #[test]
    fn solve_prefix_scans_later_explicit_layers_too() {
        // `{α}` 只在第 2 层（`b : α`）出现 ⇒ 由**第 2 个**实参的类型解出
        // （`picks 3 b` 的形状）。
        let (layers, _) =
            telescope("forall {α : Type 0}, (a : Nat) -> (b : α) -> Prop").expect("telescope");
        assert_eq!(leading_implicit(&layers), 1);
        let nat = crate::proof::parse_expr_text("Nat").expect("parse");
        let prop = crate::proof::parse_expr_text("Prop").expect("parse");
        let result = crate::proof::parse_expr_text("Prop").expect("parse");
        let solved = solve_prefix(
            &layers,
            &result,
            1,
            &[Some(nat.clone()), Some(prop)],
            None,
            &empty_defs(),
            &|_| false,
        )
        .expect("solved");
        assert!(
            matches!(&solved[0], Expr::Sort { .. }),
            "`Prop` 是 Sort: {solved:?}"
        );
        // 后面没有任何层提到它 ⇒ 解不出（专用错误码，不猜）
        let (layers, result) =
            telescope("forall {α : Type 0}, (a : Nat) -> Nat").expect("telescope");
        assert!(solve_prefix(
            &layers,
            &result,
            1,
            &[Some(nat)],
            None,
            &empty_defs(),
            &|_| false
        )
        .is_none());
    }

    /// 路线 ②：参数只出现在**结果类型**里 ⇒ 由期望类型解出（`Or.inl` 的 `B`）。
    #[test]
    fn solve_prefix_reads_the_expected_type() {
        // `Or.inl {A B} (a : A) : Or A B`：`B` 不在任何后续层的域里。
        let (layers, result) =
            telescope("forall {A : Prop} {B : Prop}, (a : A) -> Or A B").expect("telescope");
        assert_eq!(leading_implicit(&layers), 2);
        let a_ty = crate::proof::parse_expr_text("Nat").expect("parse");
        // `a : Nat`，期望 `Or Nat Bool` ⇒ `A := Nat`（路线①）、`B := Bool`（路线②）
        let expected = crate::proof::parse_expr_text("Or Nat Bool").expect("parse");
        let solved = solve_prefix(
            &layers,
            &result,
            2,
            &[Some(a_ty)],
            Some(&expected),
            &empty_defs(),
            &|_| false,
        )
        .expect("solved");
        assert_eq!(solved.len(), 2);
        assert!(matches!(&solved[0], Expr::Ident { name, .. } if name == "Nat"));
        assert!(matches!(&solved[1], Expr::Ident { name, .. } if name == "Bool"));
        // 没有期望类型 ⇒ 解不出（不许猜）
        assert!(solve_prefix(
            &layers,
            &result,
            2,
            &[Some(crate::proof::parse_expr_text("Nat").unwrap())],
            None,
            &empty_defs(),
            &|_| false,
        )
        .is_none());
    }
}

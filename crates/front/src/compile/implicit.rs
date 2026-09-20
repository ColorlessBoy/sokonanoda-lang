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
    pub style: BinderKind,
}

/// 把 `judge_infer` 给的签名文本拆成「带风格的 Pi 层 + 结果类型」。
///
/// 与 `elab::notation_telescope` 同一份解析（`parse_expr_text` + 逐层剥 Pi），
/// 只多带一个 `style`：内核 pp 把隐式 binder 打成 `{α : Type}`，而
/// `spine::peel_pi` 把风格丢了——路线 C 全靠它。多 binder 折叠
/// （`forall (a b : T), …`）与 `->`（匿名的显式层）都要认。
pub(crate) fn telescope(signature: &str) -> Option<(Vec<Layer>, Expr)> {
    let mut cur = crate::proof::parse_expr_text(signature).ok()?;
    let mut layers: Vec<Layer> = Vec::new();
    loop {
        match cur {
            Expr::Forall { binders, body, .. } if !binders.is_empty() => {
                for b in &binders {
                    layers.push(Layer {
                        name: b.name.clone(),
                        domain: b.ty.as_deref()?.clone(),
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
                    domain: *domain,
                    style: BinderKind::Explicit,
                });
                cur = *codomain;
            }
            other => return Some((layers, other)),
        }
    }
}

/// 前导隐式层的个数（`0` ⇒ 调用方走今天的老路）。
pub(crate) fn leading_implicit(layers: &[Layer]) -> usize {
    layers
        .iter()
        .take_while(|l| l.style == BinderKind::Implicit)
        .count()
}

/// 解出被跳过的前导 `k` 层（路线 ①：由**后续显式层的实际值**反解）。
///
/// `arg_tys[i]` = 第 `k + i` 层那个实参的**类型**（`None` = 拿不到）。对第 `i`
/// 个待解的隐式层，找**后面第一个**提到它的层 `j`，把 `layers[j].domain`（已解出
/// 的参数先代进去）与 `arg_tys[j - k]` 头部匹配：
///   · `Set.mem {α} (a : α) …` ⇒ `α` 出现在第 `k` 层（`a : α`）⇒ 由 `a` 的类型解出；
///   · `Set.image {α β} (f : α → β) …` ⇒ `α`/`β` 都在第 `k` 层 ⇒ 由 `f` 的类型解出；
///   · `picks {α} (n : Nat) (b : α)` 调用 `picks 3 b` ⇒ `α` 在第 `k+1` 层
///     （`b : α`）⇒ 由 `b` 的类型解出（**扫全部显式层**，不只第一层）。
/// 找不到可解的层、或解不出 ⇒ `None`（调用方报专用错误码，**不猜**）。
pub(crate) fn solve_prefix(
    layers: &[Layer],
    k: usize,
    arg_tys: &[Option<Expr>],
) -> Option<Vec<Expr>> {
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
            if let Some(v) = super::elab::unify_extract(&domain, actual, &name) {
                found = Some(v);
                break;
            }
        }
        solved.push(found?);
    }
    Some(solved)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 风格必须活着从签名文本里出来（`{}` = 隐式、`()`/`->` = 显式）。
    #[test]
    fn telescope_keeps_binder_styles() {
        let (layers, result) = telescope("forall {α : Type 0}, (a : α) -> (A : α -> Prop) -> Prop")
            .expect("telescope");
        assert_eq!(layers.len(), 3);
        assert_eq!(layers[0].name, "α");
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
        assert_eq!(layers[0].name, "a");
        assert_eq!(layers[1].name, "b");
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
        let solved = solve_prefix(&layers, 1, &[Some(actual_ty)]).expect("solved");
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
        let solved = solve_prefix(&layers, 1, &[Some(nat.clone()), Some(prop)]).expect("solved");
        assert!(
            matches!(&solved[0], Expr::Sort { .. }),
            "`Prop` 是 Sort: {solved:?}"
        );
        // 后面没有任何层提到它 ⇒ 解不出（专用错误码，不猜）
        let (layers, _) = telescope("forall {α : Type 0}, (a : Nat) -> Nat").expect("telescope");
        assert!(solve_prefix(&layers, 1, &[Some(nat)]).is_none());
    }
}

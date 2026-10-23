//! `Eq.subst` 谓词位合成的**源级 AST 工具**（`super::eqsubst` 的底座）：
//! 差异路径（`Step`）· 唯一一处差异（`Diff` / `DiffResult`）· 结构相等（忽略 span）·
//! 沿路径替换子项。
//!
//! 拆出来的原因：`AGENTS.md` 硬规则 5（文件接近 ~500 行即拆分）+ `docs-lint` ⑨。
//! 这里**只做源级形状比较**，不碰判定（判定永远走内核）。

use crate::ast::Expr;

/// 从根到差异子项的路径（只覆盖 `App` / `Notation` —— 用户现场的形状）。
#[derive(Debug, Clone)]
pub(super) enum Step {
    Fun,
    Arg,
    NotationLhs,
    NotationRhs,
    /// **P2（2026-10-23）**：约束体里面（`fun (z : T) => C[z]` 的 `C`）。
    /// 只有**binder 完全同形**才走这条（名字/类型逐字相同）⇒ 保守。
    BinderBody,
    ArrowDomain,
    ArrowCodomain,
}

#[derive(Debug, Clone)]
pub(super) struct Diff {
    pub(super) a: Expr,
    pub(super) b: Expr,
    pub(super) path: Vec<Step>,
}

pub(super) enum DiffResult {
    Same,
    One(Box<Diff>),
    /// 多处差异 / 不认识的形状 ⇒ 不碰。
    Many,
}

pub(super) fn diff(a: &Expr, b: &Expr) -> DiffResult {
    if same_expr(a, b) {
        return DiffResult::Same;
    }
    match (a, b) {
        (
            Expr::App {
                fun: f1, arg: a1, ..
            },
            Expr::App {
                fun: f2, arg: a2, ..
            },
        ) => {
            let fun = diff(f1, f2);
            let arg = diff(a1, a2);
            match (fun, arg) {
                (DiffResult::Same, DiffResult::Same) => DiffResult::Same,
                (DiffResult::One(mut d), DiffResult::Same) => {
                    d.path.insert(0, Step::Fun);
                    DiffResult::One(d)
                }
                (DiffResult::Same, DiffResult::One(mut d)) => {
                    d.path.insert(0, Step::Arg);
                    DiffResult::One(d)
                }
                _ => DiffResult::Many,
            }
        }
        (
            Expr::Notation {
                symbol: s1,
                target: t1,
                lhs: l1,
                rhs: r1,
                ..
            },
            Expr::Notation {
                symbol: s2,
                target: t2,
                lhs: l2,
                rhs: r2,
                ..
            },
        ) if s1 == s2 && t1 == t2 => {
            let lhs = diff_opt(l1.as_deref(), l2.as_deref());
            let rhs = diff_opt(r1.as_deref(), r2.as_deref());
            match (lhs, rhs) {
                (DiffResult::Same, DiffResult::Same) => DiffResult::Same,
                (DiffResult::One(mut d), DiffResult::Same) => {
                    d.path.insert(0, Step::NotationLhs);
                    DiffResult::One(d)
                }
                (DiffResult::Same, DiffResult::One(mut d)) => {
                    d.path.insert(0, Step::NotationRhs);
                    DiffResult::One(d)
                }
                _ => DiffResult::Many,
            }
        }
        // **P2**：binder 同形（名字与类型逐字相同）⇒ 差异落在约束体里。
        (
            Expr::Lambda { binders: b1, body: d1, .. },
            Expr::Lambda { binders: b2, body: d2, .. },
        )
        | (
            Expr::Forall { binders: b1, body: d1, .. },
            Expr::Forall { binders: b2, body: d2, .. },
        ) if same_binders(b1, b2) => match diff(d1, d2) {
            DiffResult::Same => DiffResult::Same,
            DiffResult::One(mut d) => {
                d.path.insert(0, Step::BinderBody);
                DiffResult::One(d)
            }
            DiffResult::Many => DiffResult::Many,
        },
        // **P2**：`A -> B` 两侧（域/陪域）。
        (Expr::Arrow { domain: d1, codomain: c1, .. }, Expr::Arrow { domain: d2, codomain: c2, .. }) => {
            let domain = diff(d1, d2);
            let codomain = diff(c1, c2);
            match (domain, codomain) {
                (DiffResult::Same, DiffResult::Same) => DiffResult::Same,
                (DiffResult::One(mut d), DiffResult::Same) => {
                    d.path.insert(0, Step::ArrowDomain);
                    DiffResult::One(d)
                }
                (DiffResult::Same, DiffResult::One(mut d)) => {
                    d.path.insert(0, Step::ArrowCodomain);
                    DiffResult::One(d)
                }
                _ => DiffResult::Many,
            }
        }
        // 原子不同 ⇒ 就是这一处差异（路径为空由调用方拒绝）。
        (Expr::Ident { .. }, Expr::Ident { .. })
        | (Expr::UniverseApp { .. }, Expr::UniverseApp { .. })
        | (Expr::Sort { .. }, Expr::Sort { .. })
        | (Expr::Num { .. }, Expr::Num { .. })
        | (Expr::Hole { .. }, Expr::Hole { .. }) => DiffResult::One(Box::new(Diff {
            a: a.clone(),
            b: b.clone(),
            path: Vec::new(),
        })),
        // 不同变体 / 原子不同：整棵子树就是这一处差异（不再往下猜）。
        _ => DiffResult::One(Box::new(Diff {
            a: a.clone(),
            b: b.clone(),
            path: Vec::new(),
        })),
    }
}

fn diff_opt(a: Option<&Expr>, b: Option<&Expr>) -> DiffResult {
    match (a, b) {
        (Some(a), Some(b)) => diff(a, b),
        (None, None) => DiffResult::Same,
        _ => DiffResult::Many,
    }
}

/// 结构相等（**忽略 span**；用户现场两边来自不同源位置）。
fn same_expr(a: &Expr, b: &Expr) -> bool {
    match (a, b) {
        (Expr::Ident { name: n1, .. }, Expr::Ident { name: n2, .. }) => n1 == n2,
        (
            Expr::UniverseApp {
                name: n1,
                levels: l1,
                ..
            },
            Expr::UniverseApp {
                name: n2,
                levels: l2,
                ..
            },
        ) => n1 == n2 && l1 == l2,
        (Expr::Sort { sort: s1, .. }, Expr::Sort { sort: s2, .. }) => s1 == s2,
        (Expr::Num { value: v1, .. }, Expr::Num { value: v2, .. }) => v1 == v2,
        (Expr::Hole { .. }, Expr::Hole { .. }) => true,
        (
            Expr::App {
                fun: f1, arg: a1, ..
            },
            Expr::App {
                fun: f2, arg: a2, ..
            },
        ) => same_expr(f1, f2) && same_expr(a1, a2),
        (
            Expr::Lambda { binders: b1, body: d1, .. },
            Expr::Lambda { binders: b2, body: d2, .. },
        )
        | (
            Expr::Forall { binders: b1, body: d1, .. },
            Expr::Forall { binders: b2, body: d2, .. },
        ) => same_binders(b1, b2) && same_expr(d1, d2),
        (
            Expr::Arrow { domain: d1, codomain: c1, .. },
            Expr::Arrow { domain: d2, codomain: c2, .. },
        ) => same_expr(d1, d2) && same_expr(c1, c2),
        (
            Expr::Notation {
                symbol: s1,
                target: t1,
                lhs: l1,
                rhs: r1,
                ..
            },
            Expr::Notation {
                symbol: s2,
                target: t2,
                lhs: l2,
                rhs: r2,
                ..
            },
        ) => {
            s1 == s2
                && t1 == t2
                && same_opt(l1.as_deref(), l2.as_deref())
                && same_opt(r1.as_deref(), r2.as_deref())
        }
        _ => false,
    }
}

/// **P2**：binder 列表**逐字同形**（名字 + 书写类型 + 风格）；任何一处不同 ⇒ 不是同一
/// 个抽象（不猜 alpha 改名 —— motive 的 binder 直接进内核项，猜错就是假证明）。
fn same_binders(a: &[crate::ast::Binder], b: &[crate::ast::Binder]) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|(x, y)| {
            x.name == y.name
                && x.style == y.style
                && same_opt(x.ty.as_deref(), y.ty.as_deref())
        })
}

fn same_opt(a: Option<&Expr>, b: Option<&Expr>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => same_expr(a, b),
        (None, None) => true,
        _ => false,
    }
}

/// 沿着 `path` 把 `expr` 里的那个子项换成 `replacement`。
pub(super) fn replace_at(expr: &Expr, path: &[Step], replacement: &Expr) -> Expr {
    if path.is_empty() {
        return replacement.clone();
    }
    match (&path[0], expr) {
        (
            Step::Fun,
            Expr::App {
                fun,
                arg,
                explicit_spine,
                span,
            },
        ) => Expr::App {
            fun: Box::new(replace_at(fun, &path[1..], replacement)),
            arg: arg.clone(),
            explicit_spine: *explicit_spine,
            span: *span,
        },
        (
            Step::Arg,
            Expr::App {
                fun,
                arg,
                explicit_spine,
                span,
            },
        ) => Expr::App {
            fun: fun.clone(),
            arg: Box::new(replace_at(arg, &path[1..], replacement)),
            explicit_spine: *explicit_spine,
            span: *span,
        },
        (
            Step::NotationLhs,
            Expr::Notation {
                symbol,
                target,
                assoc,
                lhs,
                rhs,
                alternatives,
                span,
                symbol_span,
            },
        ) => Expr::Notation {
            symbol: symbol.clone(),
            target: target.clone(),
            assoc: *assoc,
            lhs: lhs
                .as_deref()
                .map(|e| Box::new(replace_at(e, &path[1..], replacement))),
            rhs: rhs.clone(),
            alternatives: alternatives.clone(),
            span: *span,
            symbol_span: *symbol_span,
        },
        (
            Step::NotationRhs,
            Expr::Notation {
                symbol,
                target,
                assoc,
                lhs,
                rhs,
                alternatives,
                span,
                symbol_span,
            },
        ) => Expr::Notation {
            symbol: symbol.clone(),
            target: target.clone(),
            assoc: *assoc,
            lhs: lhs.clone(),
            rhs: rhs
                .as_deref()
                .map(|e| Box::new(replace_at(e, &path[1..], replacement))),
            alternatives: alternatives.clone(),
            span: *span,
            symbol_span: *symbol_span,
        },
        (Step::BinderBody, Expr::Lambda { binders, body, span }) => Expr::Lambda {
            binders: binders.clone(),
            body: Box::new(replace_at(body, &path[1..], replacement)),
            span: *span,
        },
        (Step::BinderBody, Expr::Forall { binders, body, span }) => Expr::Forall {
            binders: binders.clone(),
            body: Box::new(replace_at(body, &path[1..], replacement)),
            span: *span,
        },
        (
            Step::ArrowDomain,
            Expr::Arrow { domain, codomain, span },
        ) => Expr::Arrow {
            domain: Box::new(replace_at(domain, &path[1..], replacement)),
            codomain: codomain.clone(),
            span: *span,
        },
        (
            Step::ArrowCodomain,
            Expr::Arrow { domain, codomain, span },
        ) => Expr::Arrow {
            domain: domain.clone(),
            codomain: Box::new(replace_at(codomain, &path[1..], replacement)),
            span: *span,
        },
        // 路径与形状不匹配（调用方只在 diff 给出路径时进）⇒ 原样。
        _ => expr.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Binder, BinderKind};
    use crate::span::Span;

    fn ident(name: &str) -> Expr {
        Expr::Ident { name: name.to_string(), span: Span::default() }
    }
    fn app(f: Expr, a: Expr) -> Expr {
        Expr::App { fun: Box::new(f), arg: Box::new(a), explicit_spine: false, span: Span::default() }
    }
    fn mem(x: &str, a: &str) -> Expr {
        Expr::Notation {
            symbol: "∈".to_string(),
            symbol_span: Span::default(),
            target: "Set.mem".to_string(),
            assoc: crate::ast::NotationAssoc::Infix,
            lhs: Some(Box::new(ident(x))),
            rhs: Some(Box::new(ident(a))),
            alternatives: Vec::new(),
            span: Span::default(),
        }
    }
    fn lam(body: Expr) -> Expr {
        Expr::Lambda {
            binders: vec![Binder {
                name: "z".to_string(),
                ty: Some(Box::new(ident("T"))),
                style: BinderKind::Explicit,
                span: Span::default(),
            }],
            body: Box::new(body),
            span: Span::default(),
        }
    }

    /// **P2**：差异在 binder **里面**（`fun z => z ∈ A` ≟ `fun z => z ∈ B`）⇒
    /// 仍然只算一处，路径带 `BinderBody`；沿路径替换能抽出 motive。
    #[test]
    fn deep_occurrence_under_a_binder_synthesizes_a_motive() {
        let (l, r) = (lam(mem("z", "A")), lam(mem("z", "B")));
        let DiffResult::One(d) = diff(&l, &r) else {
            panic!("binder 里的唯一差异应算一处");
        };
        assert!(matches!(d.path.first(), Some(Step::BinderBody)), "路径首步应是 BinderBody");
        let motive_body = replace_at(&r, &d.path, &ident("s"));
        let printed = crate::proof::render_expr(&motive_body);
        assert!(printed.contains("fun"), "motive 应仍是 lambda：{printed}");
        assert!(printed.ends_with("∈ s") || printed.contains("∈ s"), "差异位应换成 s：{printed}");
        // 多处独立差异 ⇒ 不猜（Many）：头部与实参同时不同
        let many_l = lam(app(ident("f"), ident("A")));
        let many_r = lam(app(ident("g"), ident("B")));
        assert!(matches!(diff(&many_l, &many_r), DiffResult::Many));
    }
}

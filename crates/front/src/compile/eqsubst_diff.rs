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
        // 路径与形状不匹配（调用方只在 diff 给出路径时进）⇒ 原样。
        _ => expr.clone(),
    }
}

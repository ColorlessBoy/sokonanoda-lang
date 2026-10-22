//! `apply Eq.subst _ premise` 的**谓词位合成**（2026-10-22 用户 unit02 现场）。
//!
//! ## 现场
//!
//! ```text
//! intro h x hx
//! apply Eq.subst _ hx
//! exact h
//! ```
//!
//! 目标 `x ∈ ∅`，`hx : x ∈ A`，`h : A = ∅`。`Eq.subst` 的 motive 是
//! `fun (s : Set α) => x ∈ s`，洞那一位是 `A = ∅`；Lean 4 会把它当**带期望类型的
//! 合成元变量**（`Elab/BuiltinTerm.lean:65`）⇒ 洞变成新目标 ✓。
//!
//! a14（`compile/hole.rs`）已经能把**实参位的洞**变成子目标，但它只覆盖
//! “`?p ?x` 里的 `?x`”/元素等式；`Set`/函数等式要解的是**高阶 motive** `p`，
//! `Eq.subst` 的隐式前缀解不出（`meta.rs` 的 `?p ?x ≟ 点名应用链` 边界，
//! 见 `tests/eqsubst_predicate.rs` 第 3 条）。用户现场于是：
//!
//! * 有末尾 `sorry` ⇒ open-exercise 模式把洞接住，但 `sorry` 自己撞“没有目标”；
//! * 没有 `sorry` ⇒ `apply` 报 `Eq.subst` 隐式参数解不出。
//!
//! ## 修法（白纸黑字）
//!
//! 不引入项元变量，也不做通用高阶合一。只在 `apply_tactic` 里识别一个**可判定**
//! 的形状：`Eq.subst _ premise`，且 `premise` 的类型与当前目标是**同一个谓词**
//! 作用在两个只差一个子项的值上（`x ∈ A` vs `x ∈ ∅`）。此时把差异子项抽成
//! binder，合成 `Eq.subst` 的 **old-style 完整前缀**：
//!
//! ```text
//! Eq.subst (Set α) (fun (s : Set α) => x ∈ s) A ∅ _ hx
//! ```
//!
//! 这条形状已有 a14 的“洞 → 子目标”机械（`hole.rs`）接住：洞的期望类型是
//! `A = ∅`，`exact h` 正好闭合它。`Eq.subst` 的 `p/a/b` 不再需要高阶反解。
//!
//! **保守边界**：差异必须**恰好一处**、差异子项必须是**局部 binder**（用于取
//! 类型）；否则原样返回 `None`，走今天的错误路径（不猜、不放宽判定）。

use crate::ast::{Binder, BinderKind, Expr};
use crate::span::Span;
use crate::spine;

/// 重写后的表达式（`None` = 形状不匹配，保持今天的行为）。
pub(crate) fn rewrite_argument_hole(
    expr: &Expr,
    goal: &Expr,
    binders: &[Binder],
    span: Span,
) -> Option<Expr> {
    let (head, args) = spine::spine_of(expr);
    if !is_eqsubst_head(head) || args.len() != 2 {
        return None;
    }
    if !matches!(args[0], Expr::Hole { .. }) {
        return None;
    }
    let premise = args[1];
    let premise_ty = match premise {
        Expr::Ident { name, .. } => binders
            .iter()
            .rev()
            .find(|b| b.name == *name)
            .and_then(|b| b.ty.as_deref().cloned())?,
        _ => return None,
    };
    let diff = match diff(&premise_ty, goal) {
        DiffResult::One(diff) if !diff.path.is_empty() => diff,
        _ => return None,
    };
    let ty = match &diff.a {
        Expr::Ident { name, .. } => binders
            .iter()
            .rev()
            .find(|b| b.name == *name)
            .and_then(|b| b.ty.as_deref().cloned())?,
        _ => return None,
    };
    let var_name = format!("_soko_eq{}", span.start.offset);
    let var = Expr::Ident {
        name: var_name.clone(),
        span,
    };
    let body = replace_at(goal, &diff.path, &var);
    let motive = Expr::Lambda {
        binders: vec![Binder {
            name: var_name,
            ty: Some(Box::new(ty.clone())),
            style: BinderKind::Explicit,
            span,
        }],
        body: Box::new(body),
        span,
    };
    // old-style 前缀：`Eq.subst (T) (fun s => C[s]) a b _ premise`。
    // `a` 来自 premise 的类型，`b` 来自目标；洞是 `a = b`。
    let new_args = [
        ty,
        motive,
        diff.a.clone(),
        diff.b.clone(),
        args[0].clone(),
        premise.clone(),
    ];
    let mut out = head.clone();
    for arg in new_args {
        out = Expr::App {
            fun: Box::new(out),
            arg: Box::new(arg),
            explicit_spine: false,
            span,
        };
    }
    Some(out)
}

fn is_eqsubst_head(head: &Expr) -> bool {
    match head {
        Expr::Ident { name, .. } => name == "Eq.subst",
        Expr::UniverseApp { name, .. } => name == "Eq.subst",
        _ => false,
    }
}

/// 从根到差异子项的路径（只覆盖 `App` / `Notation` —— 用户现场的形状）。
#[derive(Debug, Clone)]
enum Step {
    Fun,
    Arg,
    NotationLhs,
    NotationRhs,
}

#[derive(Debug, Clone)]
struct Diff {
    a: Expr,
    b: Expr,
    path: Vec<Step>,
}

enum DiffResult {
    Same,
    One(Box<Diff>),
    /// 多处差异 / 不认识的形状 ⇒ 不碰。
    Many,
}

fn diff(a: &Expr, b: &Expr) -> DiffResult {
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
fn replace_at(expr: &Expr, path: &[Step], replacement: &Expr) -> Expr {
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

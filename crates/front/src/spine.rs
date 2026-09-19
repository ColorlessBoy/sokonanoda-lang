//! Pi 望远镜 / 应用 spine 的共享机械。
//!
//! 这里的东西原先是 `by.rs` 的私有函数，值位 `apply`（`compile/apply.rs`）
//! 要用同一套——**两条 `apply` 路径必须共用一份 telescope 机械**，否则
//! 「tactic 里的 apply」与「值位 apply」会在合一强度、参数实例化上慢慢分叉
//! （`docs/design/term-apply.md` §3 明确要求）。
//!
//! 只做结构操作，不做判定：类型文本一律由 `judge::judge_infer` 问内核。

use crate::ast::MatchArm;
use crate::Expr;

/// 剥一层 Pi/Arrow 的结果：binder 名（Arrow 层为空）、域、余下的体。
pub(crate) struct PiBody {
    pub(crate) name: String,
    pub(crate) domain: Expr,
    pub(crate) body: Expr,
}

/// 从 `expr` 剥一层 Pi/Arrow，返回 (name, domain, body)。
pub(crate) fn peel_pi(expr: &Expr) -> Option<PiBody> {
    match expr {
        Expr::Forall { binders, body, .. } if !binders.is_empty() => {
            let b = &binders[0];
            let domain = b.ty.as_deref().cloned()?;
            // pp 会把 `(a : T) -> (b : T)` 折叠成 `forall (a b : T), ...`——
            // 逐 binder 剥，余下的重包成 Forall。
            let rest = if binders.len() > 1 {
                Expr::Forall {
                    binders: binders[1..].to_vec(),
                    body: body.clone(),
                    span: b.span,
                }
            } else {
                body.as_ref().clone()
            };
            Some(PiBody {
                name: b.name.clone(),
                domain,
                body: rest,
            })
        }
        Expr::Arrow {
            domain, codomain, ..
        } => Some(PiBody {
            name: String::new(),
            domain: domain.as_ref().clone(),
            body: codomain.as_ref().clone(),
        }),
        _ => None,
    }
}

/// 把 `expr` 拆成 (头, 实参列表)。
pub(crate) fn spine_of(expr: &Expr) -> (&Expr, Vec<&Expr>) {
    let mut args = Vec::new();
    let mut cur = expr;
    while let Expr::App { fun, arg, .. } = cur {
        args.push(arg.as_ref());
        cur = fun;
    }
    args.reverse();
    (cur, args)
}

pub(crate) fn same_head(a: &Expr, b: &Expr) -> bool {
    match (a, b) {
        (Expr::Ident { name: x, .. }, Expr::Ident { name: y, .. }) => x == y,
        (
            Expr::UniverseApp {
                name: x,
                levels: lx,
                ..
            },
            Expr::UniverseApp {
                name: y,
                levels: ly,
                ..
            },
        ) => x == y && lx == ly,
        _ => false,
    }
}

/// `name` 是否出现在 `expr` 的 Ident 节点里。
pub(crate) fn mentions(name: &str, expr: &Expr) -> bool {
    match expr {
        Expr::Ident { name: n, .. } => n == name,
        Expr::UniverseApp { .. } | Expr::Sort { .. } | Expr::Num { .. } | Expr::Hole { .. } => {
            false
        }
        Expr::App { fun, arg, .. } => mentions(name, fun) || mentions(name, arg),
        Expr::SetLiteral { elements, .. } => elements.iter().any(|e| mentions(name, e)),
        Expr::Lambda { binders, body, .. } | Expr::Forall { binders, body, .. } => {
            binders
                .iter()
                .any(|b| b.ty.as_deref().is_some_and(|t| mentions(name, t)))
                || mentions(name, body)
        }
        Expr::Arrow {
            domain, codomain, ..
        } => mentions(name, domain) || mentions(name, codomain),
        Expr::Plus { lhs, rhs, .. } => mentions(name, lhs) || mentions(name, rhs),
        Expr::Let {
            binder, val, body, ..
        } => {
            binder.ty.as_deref().is_some_and(|t| mentions(name, t))
                || mentions(name, val)
                || mentions(name, body)
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            mentions(name, scrutinee)
                || arms.iter().any(|arm| {
                    arm.guard.as_ref().is_some_and(|g| mentions(name, g))
                        || mentions(name, &arm.body)
                })
        }
        // 记号节点（G-04 / WO-011）：符号与目标名不是标识符，只走操作数。
        Expr::Notation { lhs, rhs, .. } => {
            lhs.as_deref().is_some_and(|e| mentions(name, e))
                || rhs.as_deref().is_some_and(|e| mentions(name, e))
        }
        Expr::By { .. } => false,
    }
}

/// 位置 spine 合一：`codomain` 与 `goal` 同头（Ident/UniverseApp 相同）时
/// 逐参数位对应，给出「`layers` 里命名 binder 名 → goal 实参」的替换 σ。
///
/// 合一强度刻意只到这里（同头 + 实参个数相等），不做高阶匹配——两条 `apply`
/// 路径共用同一强度，教学子集的边界才说得清。
pub(crate) fn unify_spine(
    codomain: &Expr,
    goal: &Expr,
    layers: &[(String, Expr)],
) -> Option<std::collections::HashMap<String, Expr>> {
    let (chead, cargs) = spine_of(codomain);
    let (ghead, gargs) = spine_of(goal);
    if !same_head(chead, ghead) || cargs.len() != gargs.len() {
        return None;
    }
    // 把 codomain 实参位与 goal 实参位对应；codomain 里的命名 binder 是
    // 类型参数，映射到 goal 实参。
    let mut sigma = std::collections::HashMap::new();
    for (name, _) in layers {
        for (carg, garg) in cargs.iter().copied().zip(gargs.iter().copied()) {
            if let Expr::Ident { name: n, .. } = carg {
                if n.as_str() == name.as_str() {
                    sigma.insert(name.clone(), garg.clone());
                }
            }
        }
    }
    Some(sigma)
}

/// 把 σ 代入 `expr` 里的命名 Ident（不改 span）。
/// 收集模式里绑定的变量名（`_`/字面量不绑定）。
pub(crate) fn collect_pattern_binds(pat: &crate::ast::Pattern, out: &mut Vec<String>) {
    match pat {
        crate::ast::Pattern::Wild { .. } | crate::ast::Pattern::Num { .. } => {}
        crate::ast::Pattern::Ident { name, args, .. } => {
            if args.is_empty() {
                out.push(name.clone());
            } else {
                for a in args {
                    collect_pattern_binds(a, out);
                }
            }
        }
    }
}

pub(crate) fn substitute(expr: &Expr, sigma: &std::collections::HashMap<String, Expr>) -> Expr {
    match expr {
        // 集合字面量（第三刀 §12.4）：逐元素代换。
        Expr::SetLiteral { elements, span } => Expr::SetLiteral {
            elements: elements.iter().map(|e| substitute(e, sigma)).collect(),
            span: *span,
        },
        Expr::Ident { name, span } => match sigma.get(name) {
            Some(repl) => repl.clone(),
            None => Expr::Ident {
                name: name.clone(),
                span: *span,
            },
        },
        Expr::Sort { sort, span } => Expr::Sort {
            sort: sort.clone(),
            span: *span,
        },
        Expr::UniverseApp { name, levels, span } => Expr::UniverseApp {
            name: name.clone(),
            levels: levels.clone(),
            span: *span,
        },
        Expr::Num { value, span } => Expr::Num {
            value: value.clone(),
            span: *span,
        },
        Expr::Hole { span } => Expr::Hole { span: *span },
        Expr::App { fun, arg, span } => Expr::App {
            fun: Box::new(substitute(fun, sigma)),
            arg: Box::new(substitute(arg, sigma)),
            span: *span,
        },
        Expr::Lambda {
            binders,
            body,
            span,
        } => Expr::Lambda {
            binders: binders.clone(),
            body: Box::new(substitute(body, sigma)),
            span: *span,
        },
        Expr::Forall {
            binders,
            body,
            span,
        } => Expr::Forall {
            binders: binders.clone(),
            body: Box::new(substitute(body, sigma)),
            span: *span,
        },
        Expr::Arrow {
            domain,
            codomain,
            span,
        } => Expr::Arrow {
            domain: Box::new(substitute(domain, sigma)),
            codomain: Box::new(substitute(codomain, sigma)),
            span: *span,
        },
        Expr::Plus { lhs, rhs, span } => Expr::Plus {
            lhs: Box::new(substitute(lhs, sigma)),
            rhs: Box::new(substitute(rhs, sigma)),
            span: *span,
        },
        Expr::Let {
            binder,
            val,
            body,
            span,
        } => Expr::Let {
            binder: binder.clone(),
            val: Box::new(substitute(val, sigma)),
            body: Box::new(substitute(body, sigma)),
            span: *span,
        },
        Expr::Match {
            scrutinee,
            arms,
            span,
        } => {
            // 模式的绑定变量对被匹配的 body/守卫是**阴影**：从代入里删掉它们，
            // 避免把模式变量错误替换掉（`docs/design/match-patterns.md` §5）。
            let scrutinee = Box::new(substitute(scrutinee, sigma));
            let arms = arms
                .iter()
                .map(|arm| {
                    let mut sigma = sigma.clone();
                    let mut binds = Vec::new();
                    collect_pattern_binds(&arm.pattern, &mut binds);
                    for name in &binds {
                        sigma.remove(name);
                    }
                    MatchArm {
                        pattern: arm.pattern.clone(),
                        guard: arm.guard.as_ref().map(|g| substitute(g, &sigma)),
                        body: substitute(&arm.body, &sigma),
                        span: arm.span,
                    }
                })
                .collect();
            Expr::Match {
                scrutinee,
                arms,
                span: *span,
            }
        }
        Expr::By { tactics, span } => Expr::By {
            tactics: tactics.clone(),
            span: *span,
        },
        // 记号节点（G-04 / WO-011）：只代换操作数。
        Expr::Notation {
            symbol,
            target,
            assoc,
            lhs,
            rhs,
            alternatives,
            span,
        } => Expr::Notation {
            symbol: symbol.clone(),
            target: target.clone(),
            assoc: *assoc,
            lhs: lhs.as_ref().map(|e| Box::new(substitute(e, sigma))),
            rhs: rhs.as_ref().map(|e| Box::new(substitute(e, sigma))),
            alternatives: alternatives.clone(),
            span: *span,
        },
    }
}

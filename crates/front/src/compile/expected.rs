//! **期望类型（bidirectional）的源级机械**（2026-10-14）—— 从 `elab.rs` **拆出来** ✓。
//!
//! 为什么单独一个模块 ✗：`elab.rs` 已经 8800+ 行、而且被 `docs-lint` 判据⑨
//! **冻结**（只许减不许增 ✗）⇒ 新增的期望类型机械**必须**有自己的家 ✓
//! （与 `implicit.rs`/`meta.rs` 同一条模块化纪律 ✓）。
//!
//! 本模块只放**源级、零内核调用**的那几件：
//! * [`mentions_names`] / [`mentions_any`] —— 遍历形状与 `crate::spine::mentions` **同一份** ✓
//!   （漏一个变体就会漏一次检查 ✗ —— 2026-10-14 实测的 `Expr::Notation` 漏检就是它 ✓）；
//! * [`drop_expected_src_layer_unfolding`] / [`drop_expected_src_layer_aligned`] ——
//!   让**期望类型**穿过 `fun` binder（源级 δ 展开 + binder 名对齐 ✓）。
//!
//! 判定的收口仍在**内核**（本模块一个内核调用都没有 ✓）。

use super::elab::{ElabCtx, ElabScope};
use crate::{Expr, Span};
use std::collections::HashMap;

/// `e` 里有没有引用 `names` 里的**裸名**（同 [`mentions_any`]，但直接收名字表 ✓）。
pub(crate) fn mentions_names(e: &Expr, names: &[String]) -> bool {
    if names.is_empty() {
        return false;
    }
    fn walk(e: &Expr, names: &[String]) -> bool {
        // ⚠ **遍历形状必须与 [`crate::spine::mentions`] 同一份** ✗→✓
        // （2026-10-14 实测的漏检：`Expr::Notation` 落到 `_ => false` ⇒
        //  `∃ (x : α), A x ∧ Eq β (f x) y` 里的裸 `y` **看不见** ✗ ⇒
        //  `suspect` 判成 `false` ⇒ 一个**已知错**的期望类型被当成对的交下去 ⇒
        //  隐式求解解出含自由名的前导参数 ⇒ 报误导性的 `unknown identifier y`
        //  —— `units/solutions/{I.11/unit31,I.13/unit37,I.6/unit109}` 三处实测 ✓。
        //  这正是本函数注释里那句「**漏一个变体就会漏一次检查**」✗ 的实例 ✓）。
        match e {
            Expr::Ident { name, .. } | Expr::UniverseApp { name, .. } => names.contains(name),
            Expr::Sort { .. } | Expr::Num { .. } | Expr::Hole { .. } | Expr::By { .. } => false,
            Expr::App { fun, arg, .. } => walk(fun, names) || walk(arg, names),
            Expr::SetLiteral { elements, .. } | Expr::AnonCtor { elements, .. } => {
                elements.iter().any(|e| walk(e, names))
            }
            Expr::NamedArg { value, .. } => walk(value, names),
            Expr::Forall { binders, body, .. } | Expr::Lambda { binders, body, .. } => {
                binders
                    .iter()
                    .any(|b| b.ty.as_deref().is_some_and(|t| walk(t, names)))
                    || walk(body, names)
            }
            Expr::Arrow {
                domain, codomain, ..
            } => walk(domain, names) || walk(codomain, names),
            Expr::Plus { lhs, rhs, .. } => walk(lhs, names) || walk(rhs, names),
            Expr::Let {
                binder, val, body, ..
            } => {
                binder.ty.as_deref().is_some_and(|t| walk(t, names))
                    || walk(val, names)
                    || walk(body, names)
            }
            Expr::Match {
                scrutinee, arms, ..
            } => {
                walk(scrutinee, names)
                    || arms.iter().any(|arm| {
                        arm.guard.as_ref().is_some_and(|g| walk(g, names)) || walk(&arm.body, names)
                    })
            }
            // 记法节点（G-04 / WO-011）：符号与目标名不是标识符，只走操作数 ✓
            // —— 与 [`crate::spine::mentions`] 逐条同款 ✓。
            Expr::Notation { lhs, rhs, .. } => {
                lhs.as_deref().is_some_and(|e| walk(e, names))
                    || rhs.as_deref().is_some_and(|e| walk(e, names))
            }
        }
    }
    walk(e, names)
}

/// `e` 里有没有引用**更早的 binder**（保守判据 ✓ —— 只看**裸名** ✓，命中就放弃 ✓）。
pub(crate) fn mentions_any(
    e: &Expr,
    scope: &ElabScope<'_>,
    earlier: &[crate::ast::Binder],
) -> bool {
    let mut names: Vec<&str> = earlier.iter().map(|b| b.name.as_str()).collect();
    names.retain(|n| !n.is_empty());
    if names.is_empty() {
        return false;
    }
    // 作用域里**同名**的那些**不是**这一组的 ⇒ 也算"更早的" ✓（保守方向安全 ✓）。
    let _ = scope;
    fn walk(e: &Expr, names: &[&str]) -> bool {
        // ⚠ 与 [`mentions_names`] / [`crate::spine::mentions`] **同一份遍历形状** ✓
        // （同一条漏检：`Expr::Notation` 落到 `_ => false` ✗ —— 见 `mentions_names`）。
        match e {
            Expr::Ident { name, .. } | Expr::UniverseApp { name, .. } => {
                names.contains(&name.as_str())
            }
            Expr::Sort { .. } | Expr::Num { .. } | Expr::Hole { .. } | Expr::By { .. } => false,
            Expr::App { fun, arg, .. } => walk(fun, names) || walk(arg, names),
            Expr::SetLiteral { elements, .. } | Expr::AnonCtor { elements, .. } => {
                elements.iter().any(|e| walk(e, names))
            }
            Expr::NamedArg { value, .. } => walk(value, names),
            Expr::Forall { binders, body, .. } | Expr::Lambda { binders, body, .. } => {
                binders
                    .iter()
                    .any(|b| b.ty.as_deref().is_some_and(|t| walk(t, names)))
                    || walk(body, names)
            }
            Expr::Arrow {
                domain, codomain, ..
            } => walk(domain, names) || walk(codomain, names),
            Expr::Plus { lhs, rhs, .. } => walk(lhs, names) || walk(rhs, names),
            Expr::Let {
                binder, val, body, ..
            } => {
                binder.ty.as_deref().is_some_and(|t| walk(t, names))
                    || walk(val, names)
                    || walk(body, names)
            }
            Expr::Match {
                scrutinee, arms, ..
            } => {
                walk(scrutinee, names)
                    || arms.iter().any(|arm| {
                        arm.guard.as_ref().is_some_and(|g| walk(g, names)) || walk(&arm.body, names)
                    })
            }
            Expr::Notation { lhs, rhs, .. } => {
                lhs.as_deref().is_some_and(|e| walk(e, names))
                    || rhs.as_deref().is_some_and(|e| walk(e, names))
            }
        }
    }
    walk(e, &names)
}

/// **本格的闸门**（2026-10-14）：`SOKO_EXPECTED_UNFOLD=0|off` ⇒ 恒 `None`
/// （= 今天的行为 ✓，反向验证用 ✓）—— 与第 4 刀 `SOKO_LEGACY_NO_EXPECTED` /
/// 第 5 刀 `SOKO_CONST_ARG_EXPECTED` 同一口径（默认开、显式关 ⇒ 回到改前 ✓）。
fn expected_unfold_enabled() -> bool {
    !matches!(
        std::env::var("SOKO_EXPECTED_UNFOLD").ok().as_deref(),
        Some("0") | Some("off")
    )
}

/// **期望类型的源级 δ 展开后再剥一层**（2026-10-14）—— 与 [`b2_whnf_peel`]
/// **同族**（同一条 `unfold_one` + `peel_expected_src` 机械），但**零内核调用** ✓。
///
/// ## 它补的是哪一格
///
/// 声明的类型写成 **def 应用**时（`Function.Surjective α β f`、`Surjective α α f`），
/// [`drop_expected_src_layer`] 剥不开 ✗（它只认**语法** `Arrow`/`Forall`）——
/// 于是 `fun (b : β) => …` 的**体**拿不到期望类型（`expected_src = None`）✗
/// ⇒ **常量头自己的隐式类型参数**（`Exists.intro` 的 `{A}`/`{p}`）**没有期望类型
/// 这条线索**可用 ⇒ 只剩"后续实参的类型"一条 ⇒ `p w ≟ (f (g b)) = b` 是非模式
/// 合一 ⇒ 解不出（或解错）⇒ `elab-implicit-argument-unsolved` / `kernel-rejected` ✗。
///
/// **与 [`const_arg_expected`] 同一族**（第 5 刀）：注册表 + 源级机械、**零内核
/// 调用、零递归**（上界 8 层 = [`b2_whnf_peel`] 同一个上界 ✓）；差别只在它补的是
/// **"常量头自己的隐式参数"**（期望类型这一条线索），而不是"下一个实参的期望类型" ✓。
///
/// ## 与 Lean 4 同构
///
/// Lean 的 `expectedType?` 是**项**（`Expr`），最终由 `ensureExpectedType` /
/// `isDefEq` 收口 ⇒ δ 展开由**合一**顺手做掉 ✓（`Elab/App.lean:1164` 明写
/// `expectedType?` 只用于**传播**，不负责判）。我们的期望类型在源级是**语法**
/// ⇒ 要**显式**展开一层（`unfold_one` 就是源级 δ，与 `meta.rs` 的 `unfold`
/// 同一份机械 ✓）。
///
/// ## 三条保守边界（宁可 `None` = 今天的行为 ✓，绝不返回**错的**期望类型 ✗）
///
/// ① 只在 [`drop_expected_src_layer`] **失败之后**才进（既有形状**零额外开销、
///    逐字节不变** ✓）；② `unfold_one` 展不动 / 展开不动 ⇒ `None` ✓；
/// ③ 最多 8 层，仍剥不开 ⇒ `None` ✓。
pub(crate) fn drop_expected_src_layer_unfolding(
    expected: Option<&Expr>,
    ctx: &ElabCtx<'_, '_>,
    src_name: &str,
) -> Option<Expr> {
    if let Some(rest) = drop_expected_src_layer_aligned(expected, src_name) {
        return Some(rest);
    }
    if !expected_unfold_enabled() {
        return None;
    }
    let mut cur = expected?.clone();
    for _ in 0..8 {
        let next = crate::spine::unfold_one(&cur, ctx.defs, None)?;
        if next == cur {
            return None;
        }
        cur = next;
        if let Some(rest) = drop_expected_src_layer_aligned(Some(&cur), src_name) {
            return Some(rest);
        }
    }
    None
}

/// **剥一层 + 把期望侧的 binder 名对齐到源侧的 binder 名**（2026-10-14）。
///
/// ## 它补的是哪一格
///
/// 期望类型的 binder 名与源里写的 binder 名**可以不同**：
/// * `theorem t : ∀ (y : Nat), P y := fun (z : Nat) => …`（源里叫 `z`、期望里叫 `y`）；
/// * **pp 往返**（`infer_type_text` / `application_arg_expected`）会**防撞改名**
///   （`x` → `x__` ✗ —— 实测 `unit109` 的期望类型里是 `x___`/`y_`/`z_`/`h_`）。
///
/// [`drop_expected_src_layer`] 只**丢**一层、**不代换** ✗ ⇒ 剥完之后剩下的期望类型里
/// 留着**已经不在作用域**的那个名字 ✗ ⇒ 它被「隐式实参求解」的**路线②**
/// （`solve_prefix_impl_alt` 的 `unify_extract(result, expected, name)` ✓）当成真期望
/// 类型用 ⇒ 解出**含自由名**的前导参数 ⇒ 组装时 elaborate 那个值 ⇒ 报
/// `unknown identifier x__`（**误导性诊断**：真正的问题在期望类型的命名 ✗）
/// —— 实测 `units/solutions/I.6/unit109-solution.sokonanoda`（两处）✓。
///
/// ## 三条保守边界（宁可 `None` = 今天的行为 ✓）
///
/// ① 名字**相同 / 有一边是空名** ⇒ 一个字都不改（返回原样 ✓）—— 既有形状
///    （`build_def` 的期望类型就是**源**类型 ⇒ 名字本来就同 ✓）**逐字节不变** ✓；
/// ② 名字不同才做一次**按名代换**（与 [`crate::spine::substitute`] 同一份机械 ✓，
///    与 `local_arg_expected` 的 `sigma` 同一条纪律 ✓）；
/// ③ 剥不开 ⇒ `None` ✓。
///
/// **与 Lean 4 的差异（白纸黑字）**：Lean 的期望类型是**项**，binder 是 de Bruijn
/// 索引、**没有"名字对不上"这件事** ✗；我们源级用名字 ⇒ 只能显式对齐 ✓。
pub(crate) fn drop_expected_src_layer_aligned(
    expected: Option<&Expr>,
    src_name: &str,
) -> Option<Expr> {
    let (name, rest) = peel_expected_src_named(expected?)?;
    if !expected_align_enabled() || name.is_empty() || src_name.is_empty() || name == src_name {
        return Some(rest);
    }
    let mut sigma: HashMap<String, Expr> = HashMap::new();
    sigma.insert(
        name,
        Expr::Ident {
            name: src_name.to_string(),
            span: Span::default(),
        },
    );
    Some(crate::spine::substitute(&rest, &sigma))
}

/// **名字对齐的闸门**（2026-10-14）：`SOKO_EXPECTED_ALIGN=0|off` ⇒ 退回今天
/// （只丢一层、**不代换** ✓，反向验证用 ✓）。
fn expected_align_enabled() -> bool {
    !matches!(
        std::env::var("SOKO_EXPECTED_ALIGN").ok().as_deref(),
        Some("0") | Some("off")
    )
}

/// [`drop_expected_src_layer`] 的**带名版**：多返回被丢掉那一层的 binder 名 ✓。
pub(crate) fn peel_expected_src_named(expected: &Expr) -> Option<(String, Expr)> {
    match expected {
        Expr::Arrow { codomain, .. } => Some((String::new(), codomain.as_ref().clone())),
        Expr::Forall { binders, body, .. } if !binders.is_empty() => {
            let rest = if binders.len() > 1 {
                Expr::Forall {
                    binders: binders[1..].to_vec(),
                    body: body.clone(),
                    span: binders[0].span,
                }
            } else {
                body.as_ref().clone()
            };
            Some((binders[0].name.clone(), rest))
        }
        _ => None,
    }
}

/// **后序结构化重写**：先递归重建子节点，再把 `f` 作用在重建后的节点上
/// （`f` 返回 `None` ⇒ 该节点原样 ✓）。span 与其余字段逐字保留 ✓。
///
/// 与 [`mentions_names`] **同一份遍历形状**（漏一个变体就会漏一次重写 ✗ ——
/// 那条注释里的 `Expr::Notation` 漏检就是同一个坑 ✓）。
pub(crate) fn rewrite_bottom_up(e: &Expr, f: &mut impl FnMut(&Expr) -> Option<Expr>) -> Expr {
    fn go(e: &Expr, f: &mut impl FnMut(&Expr) -> Option<Expr>) -> Expr {
        let rebuilt = match e {
            Expr::Ident { .. }
            | Expr::UniverseApp { .. }
            | Expr::Sort { .. }
            | Expr::Num { .. }
            | Expr::Hole { .. }
            | Expr::By { .. } => e.clone(),
            Expr::App {
                fun,
                arg,
                explicit_spine,
                span,
            } => Expr::App {
                fun: Box::new(go(fun, f)),
                arg: Box::new(go(arg, f)),
                explicit_spine: *explicit_spine,
                span: *span,
            },
            Expr::SetLiteral { elements, span } => Expr::SetLiteral {
                elements: elements.iter().map(|x| go(x, f)).collect(),
                span: *span,
            },
            Expr::AnonCtor { elements, span } => Expr::AnonCtor {
                elements: elements.iter().map(|x| go(x, f)).collect(),
                span: *span,
            },
            Expr::NamedArg { name, value, span } => Expr::NamedArg {
                name: name.clone(),
                value: Box::new(go(value, f)),
                span: *span,
            },
            Expr::Forall {
                binders,
                body,
                span,
            } => Expr::Forall {
                binders: binders.iter().map(|b| rebuild_binder(b, f)).collect(),
                body: Box::new(go(body, f)),
                span: *span,
            },
            Expr::Lambda {
                binders,
                body,
                span,
            } => Expr::Lambda {
                binders: binders.iter().map(|b| rebuild_binder(b, f)).collect(),
                body: Box::new(go(body, f)),
                span: *span,
            },
            Expr::Arrow {
                domain,
                codomain,
                span,
            } => Expr::Arrow {
                domain: Box::new(go(domain, f)),
                codomain: Box::new(go(codomain, f)),
                span: *span,
            },
            Expr::Plus { lhs, rhs, span } => Expr::Plus {
                lhs: Box::new(go(lhs, f)),
                rhs: Box::new(go(rhs, f)),
                span: *span,
            },
            Expr::Let {
                binder,
                val,
                body,
                span,
            } => Expr::Let {
                binder: rebuild_binder(binder, f),
                val: Box::new(go(val, f)),
                body: Box::new(go(body, f)),
                span: *span,
            },
            Expr::Match {
                scrutinee,
                arms,
                span,
            } => Expr::Match {
                scrutinee: Box::new(go(scrutinee, f)),
                arms: arms
                    .iter()
                    .map(|arm| crate::ast::MatchArm {
                        pattern: arm.pattern.clone(),
                        guard: arm.guard.as_ref().map(|g| go(g, f)),
                        body: go(&arm.body, f),
                        span: arm.span,
                    })
                    .collect(),
                span: *span,
            },
            Expr::Notation {
                symbol,
                symbol_span,
                target,
                assoc,
                lhs,
                rhs,
                alternatives,
                span,
            } => Expr::Notation {
                symbol: symbol.clone(),
                symbol_span: *symbol_span,
                target: target.clone(),
                assoc: *assoc,
                lhs: lhs.as_ref().map(|x| Box::new(go(x, f))),
                rhs: rhs.as_ref().map(|x| Box::new(go(x, f))),
                alternatives: alternatives.clone(),
                span: *span,
            },
        };
        f(&rebuilt).unwrap_or(rebuilt)
    }
    fn rebuild_binder(
        b: &crate::ast::Binder,
        f: &mut impl FnMut(&Expr) -> Option<Expr>,
    ) -> crate::ast::Binder {
        crate::ast::Binder {
            name: b.name.clone(),
            ty: b.ty.as_ref().map(|t| Box::new(go(t, f))),
            style: b.style.clone(),
            span: b.span,
        }
    }
    go(e, f)
}

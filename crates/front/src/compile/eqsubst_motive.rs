//! `Eq.subst` 的**谓词位合成 —— 形状 ③/④**（`apply Eq.subst h` / `exact Eq.subst h tm`）。
//!
//! 自 `eqsubst.rs` 拆出（2026-11-04，`docs-lint` ⑨：`eqsubst.rs` 越过 500 行软线 ⇒
//! 按 `AGENTS.md` 硬规则 5 拆模块）。`eqsubst.rs` 保留形状 ①/② 与分派，本模块只装
//! "从 `h` 的右端在目标里的出现抽 motive"这一族。
//!
//! 本模块在 issue #7 第 2 条（2026-11-04 用户亲报 `apply Eq.subst hx` 报
//! 「4 个隐式参数补不出来」）上补了两处，见 [`rewrite_goal_motive`] 的注释。
//!
//! ## 形状
//!
//! **③ `apply Eq.subst h`**（一个实参）⇒ 合成
//! `Eq.subst T (fun z => G[b:=z]) a b h`：类型是 `p a → p b`、`p b ≡ G`
//! ⇒ 新目标 `p a` 由用户继续证（`f x = f y` 上就是 `rfl`）。
//! **④ `exact Eq.subst h tm`** ⇒ 同一个合成体再加最后一位 `tm`（能不能被判成
//! `p a` 由内核终审）。

use super::eqsubst::{eq_endpoints, is_eqsubst_head, local_type};
use super::eqsubst_diff::replace_all;
use crate::ast::{Binder, BinderKind, Expr};
use crate::span::Span;
use crate::spine;

/// **形状 ③（P2-c，2026-10-23）**：`apply Eq.subst h`（**一个**实参）。
///
/// `h : a = b`、目标 `G` 里出现 `b` ⇒ 合成 `Eq.subst T (fun z => G[b:=z]) a b h`：
/// 它的类型是 `p a → p b`，`p b ≡ G` ⇒ 新目标 `p a` 由用户继续证
/// （`f x = f y` 上就是 `rfl`）。
///
/// 保守边界：`h` 只认局部名/`Eq.symm h`；`b` 在目标里**至少出现一次**；
/// 端点类型先取书写类型、再问 [`EndpointTy`]；否则返回 `None`（原错误路径不变）。
pub(super) fn rewrite_single_argument(
    expr: &Expr,
    goal: &Expr,
    binders: &[Binder],
    span: Span,
) -> Option<Expr> {
    let (head, args) = spine::spine_of(expr);
    if !is_eqsubst_head(head) || args.len() != 1 || matches!(args[0], Expr::Hole { .. }) {
        return None;
    }
    rewrite_goal_motive(head, args[0], goal, binders, span, None)
}

/// **形状 ③/④ 的共用体**：`premise = None` ⇒ 只给到等号（`apply`，`p a` 成为子目标）；
/// `Some(tm)` ⇒ 再补一个实参（`exact`，`tm` 必须能被判成 `p a`，内核终审）。
pub(super) fn rewrite_goal_motive(
    head: &Expr,
    eq_term: &Expr,
    goal: &Expr,
    binders: &[Binder],
    span: Span,
    premise: Option<&Expr>,
) -> Option<Expr> {
    let (eq_ty, eq_a, eq_b) = eq_endpoints(eq_term, binders)?;
    // 类型优先取右端（被替换的那一端）；右端不是局部名（`∅`）时取**左端** ——
    // 等式两端同型，左端常是局部名（`h : A = ∅` ⇒ `A` 的类型就是 `Set α`）。
    // **两端都不是局部名**时（用户现场 `hx : {a} = ∅`）用**等式自己的类型实参** ——
    // 内核形态 `@Eq.{u} T a b` 把 `T` 摆在第一位（issue #7 第 2 条），不必再问引擎。
    let ty = local_type(binders, &eq_b)
        .cloned()
        .or_else(|| local_type(binders, &eq_a).cloned())
        .or_else(|| eq_ty)?;
    let var_name = format!("_soko_eq{}", span.start.offset);
    let var = Expr::Ident {
        name: var_name.clone(),
        span,
    };
    let mut hits = 0usize;
    let mut key = eq_b.clone();
    let mut body = replace_all(goal, &key, &var, &mut hits);
    if hits == 0 {
        // **前导隐式前缀的写法差**（issue #7 第 2 条）：`intro` 进来 / 由内核文本回读
        // 出来的假设带**内核装配的显式脊**（`@Set.empty α`），而目标里是省略前导隐式
        // 的形状（`Set.empty`，pp 的闭项形态）⇒ 逐字比对 miss。⇒ 把 `eq_b` 的**前导**
        // 实参一个个剥掉再试（隐式前缀就在前面）。
        // 保守：只在逐字 0 命中时才走；合成出来的项仍由内核终审（解错 = 判红，不会静默错 ✓）。
        let (_, args) = spine::spine_of(&eq_b);
        for drop in 1..=args.len() {
            let Some(short) = strip_leading_args(&eq_b, drop) else {
                continue;
            };
            let mut h = 0usize;
            let candidate = replace_all(goal, &short, &var, &mut h);
            if h > 0 {
                key = short;
                body = candidate;
                hits = h;
                break;
            }
        }
    }
    if hits == 0 {
        return None;
    }
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
    let mut out = head.clone();
    let mut parts = vec![ty, motive, eq_a, key, eq_term.clone()];
    if let Some(p) = premise {
        parts.push(p.clone());
    }
    for arg in parts {
        out = Expr::App {
            fun: Box::new(out),
            arg: Box::new(arg),
            // **G2（2026-11-01）**：这 5–6 位是引擎拼的 `@Eq.subst` **全显式**前缀
            // （`α motive a b <等号> <premise>`，逐位对应望远镜）⇒ `@` 脊，**不是**
            // 用户写的正常形式位置实参。标 `false` 时 strict 按"位置实参只绑显式
            // 形参"数 ⇒ 谎称"你写了 5～6 个位置实参"判红（loose 走 old-style 读法才过）。
            explicit_spine: true,
            span,
        };
    }
    Some(out)
}

/// 把 `e` 的**前导** `drop` 个实参剥掉（`@Set.empty α` ⇒ `Set.empty`）。
///
/// 只在 [`rewrite_goal_motive`] 的"逐字 0 命中"兜底里用：目标里的写法是 pp 的
/// **闭项形态**（省略前导隐式实参），而假设里的类型是**内核装配的显式脊** ⇒ 两者
/// 差的就是那几位前缀。剥出来重建的节点按**源级形态**（`explicit_spine: false`）
/// 渲染，与用户写的一致 ✓。
fn strip_leading_args(e: &Expr, drop: usize) -> Option<Expr> {
    let (head, args) = spine::spine_of(e);
    if drop == 0 || drop > args.len() {
        return None;
    }
    let mut out = head.clone();
    for arg in &args[drop..] {
        out = Expr::App {
            fun: Box::new(out),
            arg: Box::new((*arg).clone()),
            explicit_spine: false,
            span: e.span(),
        };
    }
    Some(out)
}

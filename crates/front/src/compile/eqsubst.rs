//! `Eq.subst` 的**谓词位合成**（2026-10-22 / 2026-10-23 用户 unit02 现场）。
//!
//! ## 两个形状（同一个根因，同一个解）
//!
//! **① `apply Eq.subst _ premise`（洞在等号位）** —— `apply` 专用：
//!
//! ```text
//! intro h x hx
//! apply Eq.subst _ hx
//! exact h
//! ```
//!
//! **② `Eq.subst eq premise`（两个实参都给了）** —— `apply` / `exact` 通用
//! （用户 2026-10-23 亲报：`apply Eq.subst h hx` 报同一条诊断）：
//!
//! ```text
//! intro h x hx
//! apply Eq.subst h hx
//! ```
//!
//! 两个现场的语义完全一样：目标 `x ∈ ∅`、`hx : x ∈ A`、`h : A = ∅`，
//! `Eq.subst` 的 motive 是 `fun (s : Set α) => x ∈ s`。合成出来的都是同一条
//! **old-style 完整前缀**：
//!
//! ```text
//! Eq.subst (Set α) (fun (s : Set α) => x ∈ s) A ∅ <等号> hx
//! ```
//!
//! * 形状 ① 的那个洞是 `A = ∅` —— a14 的「洞 → 子目标」机械（`compile/hole.rs`）
//!   接住它（期望类型 = `A = ∅`），`exact h` 正好闭合 ✓；
//! * 形状 ② 两个实参都在 ⇒ 整条项由**内核终审**（方向/对象不对照样判红 ✓）。
//!
//! ## 为什么需要它
//!
//! `Eq.subst` 的 `p/a/b` 只能从「后续实参的类型 + 期望类型」**反解**；
//! 谓词一旦是**记法节点**（`x ∈ ?s`）或别的复合头，结构抽取器看不穿
//! （`meta.rs` 的 `?p ?x ≟ 点名应用链` 边界，见 `tests/eqsubst_predicate.rs`
//! 第 3 条），而 Miller 模式 / 高阶合一在 `docs/design/metavar-engine.md`
//! §1.2 第 16 行**明确不做**（"报好错，不搜索"）⇒ 报
//! 「4 个隐式参数补不出来 / 两条线索互相矛盾」。
//!
//! ## 保守边界（不猜、不放宽判定）
//!
//! * 不引入项元变量，也不做通用高阶合一；只识别**可判定**的形状：
//!   `Eq.subst` 恰好 2 个实参、`premise` 是**局部名**（取得到书写类型）、
//!   差异**恰好一处**、差异子项是**局部 binder**（取得到类型）；
//! * 形状 ② **只在通用反解本来解不出的形状上触发**：判据 = 差异两端都是
//!   **裸名**、且差异路径**不经过记法节点**（`p a ≟ p b` / `f x a ≟ f x b` /
//!   `mem x A ≟ mem x B` 这三种今天都能解）⇒ 那一类**不碰**（逐字节不变 ✓）；
//!   其余形状（差异端是**应用**如 `∅`、或路径穿过记法 `x ∈ ?s`）才合成；
//! * 形状不匹配 ⇒ 原样返回 `None`，走今天的错误路径。

use super::eqsubst_diff::{diff, replace_all, replace_at, same_expr, DiffResult, Step};
use crate::ast::{Binder, BinderKind, Expr};
use crate::span::Span;
use crate::spine;

/// 形状 ①：`Eq.subst _ premise` —— `apply` 专用（洞变成子目标）。
///
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
    let premise_ty = local_type(binders, premise)?;
    let synth = synthesize(premise_ty, goal, binders, span)?;
    Some(prefix(
        head,
        [
            synth.ty,
            synth.motive,
            synth.a,
            synth.b,
            args[0].clone(),
            premise.clone(),
        ],
        span,
    ))
}

/// **P2-a**：等式项的**端点**（只认局部名与 `Eq.symm` 包一层；其它 ⇒ `None`）。
fn eq_endpoints(term: &Expr, binders: &[Binder]) -> Option<(Expr, Expr)> {
    match term {
        Expr::Ident { .. } => decompose_eq(local_type(binders, term)?),
        Expr::App { fun, arg, .. } if is_head_named(fun, "Eq.symm") => {
            eq_endpoints(arg, binders).map(|(a, b)| (b, a))
        }
        _ => None,
    }
}

/// 把 `a = b`（记法）或 `Eq a b`（点名）拆成两端。
fn decompose_eq(e: &Expr) -> Option<(Expr, Expr)> {
    match e {
        Expr::Notation { target, lhs, rhs, .. } if target == "Eq" => {
            Some(((lhs.as_deref()?).clone(), (rhs.as_deref()?).clone()))
        }
        Expr::App { fun, arg, .. } => match fun.as_ref() {
            Expr::App { fun: inner, arg: lhs, .. }
                if is_head_named(inner, "Eq") && !matches!(fun.as_ref(), Expr::App { fun: f, .. } if is_head_named(f, "Eq.symm")) =>
            {
                Some(((**lhs).clone(), (**arg).clone()))
            }
            _ => None,
        },
        _ => None,
    }
}

fn is_head_named(e: &Expr, name: &str) -> bool {
    match e {
        Expr::Ident { name: n, .. } | Expr::UniverseApp { name: n, .. } => n == name,
        _ => false,
    }
}

/// 两个形状的**共同入口**（`apply` 用）：先试 ①（洞在等号位），再试 ②（等号位给了），
/// 最后试 ③（**单实参** `Eq.subst h`：从 `h` 的右端在目标里的出现抽 motive）。
pub(crate) fn rewrite(expr: &Expr, goal: &Expr, binders: &[Binder], span: Span) -> Option<Expr> {
    if !is_eqsubst_two_args(expr) {
        return rewrite_single_argument(expr, goal, binders, span);
    }
    rewrite_argument_hole(expr, goal, binders, span)
        .or_else(|| rewrite_explicit_equality(expr, goal, binders, span))
}

/// **形状 ③（P2-c，2026-10-23）**：`apply Eq.subst h`（**一个**实参）。
///
/// `h : a = b`、目标 `G` 里出现 `b`（`b` 必须是**局部名** ⇒ binder 类型现成）⇒
/// 合成 `Eq.subst T (fun z => G[b:=z]) a b h`：它的类型是 `p a → p b`，`p b ≡ G`
/// ⇒ 新目标 `p a` 由用户继续证（`f x = f y` 上就是 `rfl`）。
///
/// 保守边界：`h` 只认局部名/`Eq.symm h`（端点从书写类型取）；`b` 只认局部名；
/// `b` 在目标里**至少出现一次**；否则返回 `None`（原错误路径不变）。
fn rewrite_single_argument(
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
fn rewrite_goal_motive(
    head: &Expr,
    eq_term: &Expr,
    goal: &Expr,
    binders: &[Binder],
    span: Span,
    premise: Option<&Expr>,
) -> Option<Expr> {
    let (eq_a, eq_b) = eq_endpoints(eq_term, binders)?;
    // 类型优先取右端（被替换的那一端）；右端不是局部名（`∅`）时取**左端** ——
    // 等式两端同型，左端常是局部名（`h : A = ∅` ⇒ `A` 的类型就是 `Set α`）。
    let ty = local_type(binders, &eq_b)
        .or_else(|| local_type(binders, &eq_a))?
        .clone();
    let var_name = format!("_soko_eq{}", span.start.offset);
    let var = Expr::Ident { name: var_name.clone(), span };
    let mut hits = 0usize;
    let body = replace_all(goal, &eq_b, &var, &mut hits);
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
    let mut parts = vec![ty, motive, eq_a, eq_b, eq_term.clone()];
    if let Some(p) = premise {
        parts.push(p.clone());
    }
    for arg in parts {
        out = Expr::App {
            fun: Box::new(out),
            arg: Box::new(arg),
            explicit_spine: false,
            span,
        };
    }
    Some(out)
}

/// **`exact` 的入口**：先做**零分配**的形状检查（`Eq.subst` 恰好 2 个实参），命中才建
/// binder 列表 —— `context_binders` 会克隆每个假设的**类型 AST**，而 `exact` 是最常见的
/// tactic，别在热路径上白付 ✓（行为与直接调形状 ② 逐字节相同）。
pub(crate) fn rewrite_exact(
    expr: &Expr,
    goal: &Expr,
    binders: impl FnOnce() -> Vec<Binder>,
    span: Span,
) -> Option<Expr> {
    if !is_eqsubst_two_args(expr) {
        return None;
    }
    let binders = binders();
    rewrite_explicit_equality(expr, goal, &binders, span).or_else(|| {
        // **P2-d（2026-10-23）**：`exact Eq.subst h tm` —— `tm` 是复合项（`Eq.refl _`）
        // 时形状 ② 取不到 premise 类型；改用"目标里的右端出现"抽 motive，`tm` 原样
        // 作为最后一个实参（能不能被判成 `p a` 由内核定）。
        let (head, args) = spine::spine_of(expr);
        if !is_eqsubst_head(head) || args.len() != 2 || matches!(args[0], Expr::Hole { .. }) {
            return None;
        }
        rewrite_goal_motive(head, args[0], goal, &binders, span, Some(args[1]))
    })
}

/// 零分配：是不是 `Eq.subst` 且**恰好** 2 个实参（形状 ① / ② 的共同前提）。
fn is_eqsubst_two_args(expr: &Expr) -> bool {
    let mut cur = expr;
    let mut n = 0usize;
    while let Expr::App { fun, .. } = cur {
        cur = fun.as_ref();
        n += 1;
    }
    n == 2 && is_eqsubst_head(cur)
}

/// 形状 ②：`Eq.subst eq premise`（等号位**不是洞**）—— `apply` / `exact` 通用。
///
/// **只在通用反解本来解不出的形状上触发**（见 `generic_solves`）：差异两端都是
/// **裸名**、且差异路径不经过记法 ⇒ 那一类今天就能解、**不碰**（逐字节不变 ✓）；
/// 其余形状（差异端是**应用**、或路径穿过**记法**）才合成前缀。`apply` 与 `exact`
/// 共用同一份真相（不在两处各写一套）。
pub(crate) fn rewrite_explicit_equality(
    expr: &Expr,
    goal: &Expr,
    binders: &[Binder],
    span: Span,
) -> Option<Expr> {
    let (head, args) = spine::spine_of(expr);
    if !is_eqsubst_head(head) || args.len() != 2 {
        return None;
    }
    // 洞位走形状 ①（那是 `apply` 的「洞 → 子目标」，`exact` 用不上）。
    if matches!(args[0], Expr::Hole { .. }) {
        return None;
    }
    // **P2-a（2026-10-23，审计 row 6）**：等式项不必是**裸局部名** ——
    // 认 `h` 与 `Eq.symm h`（端点从书写类型取；其它形状仍不碰）。
    let (eq_a, eq_b) = eq_endpoints(args[0], binders)?;
    let premise = args[1];
    let premise_ty = local_type(binders, premise)?;
    let synth = synthesize(premise_ty, goal, binders, span)?;
    // **通用反解本来就能过的形状 ⇒ 不碰**：差异两端都是裸名、且路径上没有记法
    // （`p a` / `p b`、`f x a` / `f x b`、`mem x A` / `mem x B` 都是这一类）。
    // 其余形状（差异端是**应用**如 `∅`、或路径穿过**记法**如 `x ∈ ?s`）今天解不出
    // 谓词 ⇒ 才合成前缀。判据：`tests/eqsubst_predicate.rs` 的「点名形状照旧判绿」。
    if generic_solves(&synth) {
        return None;
    }
    // **P2-a**：端点方向 —— 需要 `synth.a = synth.b`（premise 侧 = goal 侧）。
    // 给了 `Eq.symm e` 且 `e` 正好是那个方向 ⇒ 用 `e`（去掉一层）；否则按需再包一层。
    let eq_term = if same_expr(&eq_a, &synth.a) && same_expr(&eq_b, &synth.b) {
        args[0].clone()
    } else if same_expr(&eq_a, &synth.b) && same_expr(&eq_b, &synth.a) {
        match args[0] {
            Expr::App { fun, arg, .. } if is_head_named(fun, "Eq.symm") => arg.as_ref().clone(),
            other => Expr::App {
                fun: Box::new(Expr::Ident { name: "Eq.symm".to_string(), span }),
                arg: Box::new(other.clone()),
                explicit_spine: false,
                span,
            },
        }
    } else {
        return None;
    };
    Some(prefix(
        head,
        [
            synth.ty,
            synth.motive,
            synth.a,
            synth.b,
            eq_term,
            premise.clone(),
        ],
        span,
    ))
}

/// 合成出来的 old-style 前缀零件。
struct Synthesized {
    /// 差异子项的类型（motive 的 binder 注解）。
    ty: Expr,
    /// `fun (s : T) => C[s]`（`C` = 目标，差异子项换成 `s`）。
    motive: Expr,
    /// `premise` 的类型里那一处（等号左边）。
    a: Expr,
    /// 目标里那一处（等号右边）。
    b: Expr,
    /// 差异路径（形状 ② 用它判「差异是否落在记法里」）。
    path: Vec<Step>,
    /// **P2-b**：这条 diff 是在**摊开记法之后**的形态上算的（motive 也从摊开形态取）。
    desugared: bool,
}

/// 把「`premise` 的类型」与「目标」之间的**唯一一处差异**抽成 binder（motive）。
fn synthesize(
    premise_ty: &Expr,
    goal: &Expr,
    binders: &[Binder],
    span: Span,
) -> Option<Synthesized> {
    // **P2-b（2026-10-23）**：原形状没有"唯一一处差异"时，**摊开记法**再试一次
    // （混合拼法：premise 写 `x ∈ A ∪ B`、goal 写 `Set.mem x (Set.union A C)`）。
    let (diff_result, _premise_ty, goal, desugared) = {
        match diff(premise_ty, goal) {
            DiffResult::One(diff) if !diff.path.is_empty() => {
                (DiffResult::One(diff), premise_ty.clone(), goal.clone(), false)
            }
            _ => {
                let (p2, g2) = (
                    crate::compile::meta::desugar_deep(premise_ty),
                    crate::compile::meta::desugar_deep(goal),
                );
                (diff(&p2, &g2), p2, g2, true)
            }
        }
    };
    let diff = match diff_result {
        DiffResult::One(diff) if !diff.path.is_empty() => diff,
        _ => return None,
    };
    // 类型从**局部 binder** 的书写类型取（差异子项必须是局部名 —— 保守边界）。
    let ty = local_type(binders, &diff.a)?.clone();
    let var_name = format!("_soko_eq{}", span.start.offset);
    let var = Expr::Ident {
        name: var_name.clone(),
        span,
    };
    let body = replace_at(&goal, &diff.path, &var);
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
    Some(Synthesized {
        ty,
        motive,
        a: diff.a,
        b: diff.b,
        path: diff.path,
        desugared,
    })
}

/// old-style 前缀：`Eq.subst (T) (fun s => C[s]) a b <等号> <premise>`。
fn prefix(head: &Expr, args: [Expr; 6], span: Span) -> Expr {
    let mut out = head.clone();
    for arg in args {
        out = Expr::App {
            fun: Box::new(out),
            arg: Box::new(arg),
            explicit_spine: false,
            span,
        };
    }
    out
}

/// 局部名（`intro` 进来的假设 / `have` 的假设）⇒ 它的**书写类型**。
fn local_type<'a>(binders: &'a [Binder], e: &Expr) -> Option<&'a Expr> {
    let Expr::Ident { name, .. } = e else {
        return None;
    };
    binders
        .iter()
        .rev()
        .find(|b| b.name == *name)
        .and_then(|b| b.ty.as_deref())
}

/// 差异路径里有没有经过**记法节点**（那正是结构抽取器看不穿的形状）。
fn path_has_notation(path: &[Step]) -> bool {
    path.iter()
        .any(|s| matches!(s, Step::NotationLhs | Step::NotationRhs))
}

/// 通用反解（`elab.rs` 的隐式反解）**本来就能过**的形状：差异两端都是**裸名**、
/// 且差异路径上**没有记法节点**。对照组（实测）：`p a ≟ p b`、`f x a ≟ f x b`、
/// `mem x A ≟ mem x B` 都判绿；`x ∈ A ≟ x ∈ ∅`（记法）、`mem x A ≟ mem x ∅`
/// （差异端是应用）都判红。
fn generic_solves(s: &Synthesized) -> bool {
    // **P2-b**：摊开之后才拿到的差异 ⇒ 通用反解**没有**这条形状的信息（它看的是
    // 原始拼法）⇒ 不许用"裸名+无记法"推断"它自己能过"。
    if s.desugared {
        return false;
    }
    !path_has_notation(&s.path)
        && matches!(&s.a, Expr::Ident { .. })
        && matches!(&s.b, Expr::Ident { .. })
}

fn is_eqsubst_head(head: &Expr) -> bool {
    match head {
        Expr::Ident { name, .. } => name == "Eq.subst",
        Expr::UniverseApp { name, .. } => name == "Eq.subst",
        _ => false,
    }
}

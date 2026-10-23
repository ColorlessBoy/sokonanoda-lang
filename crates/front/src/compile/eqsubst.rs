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

use super::eqsubst_diff::{diff, replace_at, DiffResult, Step};
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

/// 两个形状的**共同入口**（`apply` 用）：先试 ①（洞在等号位），再试 ②（等号位给了）。
pub(crate) fn rewrite(expr: &Expr, goal: &Expr, binders: &[Binder], span: Span) -> Option<Expr> {
    if !is_eqsubst_two_args(expr) {
        return None;
    }
    rewrite_argument_hole(expr, goal, binders, span)
        .or_else(|| rewrite_explicit_equality(expr, goal, binders, span))
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
    rewrite_explicit_equality(expr, goal, &binders(), span)
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
    if matches!(args[0], Expr::Hole { .. }) || !is_local(binders, args[0]) {
        return None;
    }
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
}

/// 把「`premise` 的类型」与「目标」之间的**唯一一处差异**抽成 binder（motive）。
fn synthesize(
    premise_ty: &Expr,
    goal: &Expr,
    binders: &[Binder],
    span: Span,
) -> Option<Synthesized> {
    let diff = match diff(premise_ty, goal) {
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
    Some(Synthesized {
        ty,
        motive,
        a: diff.a,
        b: diff.b,
        path: diff.path,
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

/// 是不是局部名（形状 ② 只认「等号位是局部假设」—— 窄口径）。
fn is_local(binders: &[Binder], e: &Expr) -> bool {
    match e {
        Expr::Ident { name, .. } => binders.iter().any(|b| b.name == *name),
        _ => false,
    }
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

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
use crate::compile::CompileOptions;
use crate::judge::OpenGoalSpec;
use crate::proof::parse_expr_text;
use crate::span::Span;
use crate::spine;

/// **端点类型兜底解析**（2026-11-04 issue #7 第 2 条）：
/// 局部 binder 的**书写类型**优先（零成本、绝大多数现场）；拿不到时问**调用方**。
///
/// 为什么要它：形状 ③ 原来只认"端点里有**裸局部名**"⇒ 类型现成。用户现场
/// `intro hx`（`hx : {a} = ∅`，目标 `a ∈ ∅`）里两端都是**复合项**（`{a}` 与 `∅`）
/// ⇒ `local_type` 两头都拿不到 ⇒ 整条形状放弃 ⇒ 报「4 个隐式参数补不出来」✗。
/// 而 `apply` 手上有引擎/探针（`infer_type_text_inplace` / `judge_infer`），
/// 任意小项的类型都算得出来 ⇒ 由它兜底（算不出就返回 `None`，原错误路径不变 ✓）。
pub(crate) type EndpointTy<'a> = &'a mut dyn FnMut(&Expr) -> Option<Expr>;

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
    ty_of: EndpointTy<'_>,
) -> Option<Expr> {
    let (head, args) = spine::spine_of(expr);
    if !is_eqsubst_head(head) || args.len() != 1 || matches!(args[0], Expr::Hole { .. }) {
        return None;
    }
    rewrite_goal_motive(head, args[0], goal, binders, span, None, ty_of)
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
    ty_of: EndpointTy<'_>,
) -> Option<Expr> {
    let (eq_a, eq_b) = eq_endpoints(eq_term, binders)?;
    // 类型优先取右端（被替换的那一端）；右端不是局部名（`∅`）时取**左端** ——
    // 等式两端同型，左端常是局部名（`h : A = ∅` ⇒ `A` 的类型就是 `Set α`）。
    // **两端都不是局部名**时（用户现场 `hx : {a} = ∅`）问调用方要（issue #7 第 2 条）。
    let ty = local_type(binders, &eq_b)
        .cloned()
        .or_else(|| local_type(binders, &eq_a).cloned())
        .or_else(|| ty_of(&eq_b))
        .or_else(|| ty_of(&eq_a))?;
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

/// **`apply` 侧的入口**（`by.rs::apply_tactic` 的唯一调用点）。
///
/// 把"改写用户写的项"与"问端点类型"合在一处：形状 ③ 的兜底要问类型，而问类型要用
/// 引擎的地面件（`InplaceEnv` / `ElabCtx` / `judge_infer`）——把它们摊在 `by.rs` 里会让
/// 那个已在软线之上的文件再涨（`docs-lint` ⑨）⇒ `by.rs` 只留一行调用 ✓。
#[allow(clippy::too_many_arguments)]
pub(crate) fn rewrite_apply<'a>(
    expr: &Expr,
    goal: &Expr,
    binders: &[Binder],
    span: Span,
    env: &mut Option<&mut crate::compile::elab::InplaceEnv<'_, 'a>>,
    ctx: &crate::compile::elab::ElabCtx<'a, '_>,
    binder_srcs: &[(String, Expr)],
    spec: &OpenGoalSpec,
    prefix_src: &str,
    options: &CompileOptions,
) -> Option<Expr> {
    // ⚠ **只走就地路**（`infer_type_text_inplace`）：慢路要合成 `#check` 文本，而
    // "把项渲染成文本"必须走唯一记法接口（`DisplayNotations`）——这里没有显示上下文
    // ⇒ 不引第二条渲染路（`scripts/audit-notation-paths.py` 会抓 ✓）。就地路答不出时
    // 返回 `None`，形状 ③ 照旧落回原来的诊断（不倒退 ✓）。
    let _ = (prefix_src, options);
    let mut ty_of = |target: &Expr| -> Option<Expr> {
        let text = env.as_deref_mut().and_then(|e| {
            crate::compile::elab::infer_type_text_inplace(
                e,
                ctx,
                binder_srcs,
                target,
                spec.binders.len(),
                None,
            )
            .ok()
        });
        text.and_then(|text| parse_expr_text(&text).ok())
    };
    super::eqsubst::rewrite(expr, goal, binders, span, &mut ty_of)
}

//! **期望类型里「带前导隐式形参的记法头」的前导实参补齐 + 源级 δ 展开**
//! （SPINE-N，2026-10-14）—— 与 `expected.rs` **同族**（同一份「源级、零内核调用」
//! 纪律 ✓），单独一个文件是因为 `expected.rs` 已 270 行、而 `docs-lint` 判据⑨
//! 的软线是 **500 行** ✓。
//!
//! ## 它补的是哪一格
//!
//! `theorem t : … → (r • r) a c := fun … => Exists.intro a (And.intro …)`
//! 的**自然写法**：`Exists.intro` 的前导隐式 `p` 只能由**期望类型**解出
//! （`implicit::solve_prefix_impl_alt` 的路线②）。路线②的第一步是
//! `unify_extract(Exists α ?p, expected, p)` —— `expected` 是**记法节点**
//! `(r • r) a c`、头与 `Exists` 对不上 ⇒ 落到**展开兜底**
//! （`unfold_to_inductive`）。而记法节点里**没有前导隐式实参**
//! （`Rel.comp` 的 `A`/`B`/`C` 不在源 AST 里）⇒ `Short` 读法只代换显式形参
//! `r`/`s` ⇒ 展开结果**留着签名形参裸名**：`∃ (b : B), (r a b) ∧ (r b c)` ✗
//! ⇒ `p := fun (b : B) => …` 带着自由名交下去 ⇒ 组装时报
//! `unknown identifier B`（或与作用域同名时内核报「期望 `A`，实际是 `B`」）✗。
//!
//! ## 机械（与 Lean 4 同构）
//!
//! ① 解**前导实参**（[`solve_leading_args`]）：与 `notation_prefix_args` 同一份
//!    「头部匹配 + 从实参位提取裸变量」，但操作数类型**只从源级线索取**
//!    （[`operand_type_src`]：书写类型 / λ 注解 / **注册表签名**）——
//!    ⚠ **绝不问内核** ✗：那条路要 `judge_infer` ⇒ **重编前缀** ⇒ 又落回本钩子
//!    ⇒ 无限递归（上一轮实测 `exit=134` / `thread 'main' has overflowed its stack` ✗）。
//! ② 拼**全显式应用脊**（`(r • r) a c` ⇒ `Rel.comp α α α r r a c`）。
//! ③ 按 `UnfoldAlign::Old` **源级 δ 展开**（全形参都有实参 ⇒ 逐位对齐 ✓）。
//!
//! Lean 4 对照（本机 `~/Documents/lean/lean4`）：`Elab/App.lean:1455` 的
//! `consumeImplicits` 给常量头自己的前导隐式 binder 建元变量并**写进项**，之后
//! δ 展开在 **de Bruijn 层**由 `whnfForall`/`isDefEq` 顺手做掉
//! （`:1164` 明写 `expectedType?` 只**传播**、不判）⇒ Lean 里**不存在**
//! 「形参名没代换到」✗。我们的期望类型在源级是**语法**（记法节点只带显式操作数）
//! ⇒ 只能显式补这一步 ✓。
//!
//! ## 三条保守边界（宁可 `None` = 今天的行为 ✓）
//!
//! ① **只在今天真的坏掉时**才动手：今天的展开结果**到得了归纳头、且不留未代换的
//!    签名形参** ⇒ 一步都不进 ✓（既有绿形态**逐字节不变** ✓）；
//! ② 解不出前导实参 / 展开到不了归纳头 / 展开结果仍漏名 ⇒ `None` ✓；
//! ③ 闸门 `SOKO_EXPECTED_SPINE=0|off` ⇒ 恒 `None`（反向验证 ✓）。

use super::elab::{ElabCtx, ElabScope, KnownTable};
use super::implicit::Layer;
use crate::ast::BinderKind;
use crate::{Expr, Span};
use std::collections::HashMap;

/// **本格的闸门**（2026-10-14）：`SOKO_EXPECTED_SPINE=0|off` ⇒ 恒 `None`
/// （= 今天的行为 ✓，反向验证用 ✓）。
fn enabled() -> bool {
    !matches!(
        std::env::var("SOKO_EXPECTED_SPINE").ok().as_deref(),
        Some("0") | Some("off")
    )
}

/// **前导实参补齐的下限**（只对 `implicit_prefix >= 本值` 的记法头动手 ✓）。
///
/// 为什么要有它：`k = 1` 的记法（`∈`、`⊆`、`=` 那一族）**铺满整份课程**，
/// 而它们唯一的那个前导位就是类型变量 `α` —— 今天「留着不代换」在**大多数**
/// 位置**恰好**与作用域里的同名类型变量重合（实测：`lib/Rel:225` 同形，
/// 那里漏出的 `B` 恰是作用域里的类型参数 ⇒ 展开结果**恰好是对的** ✓）。
/// ⇒ 那一族**不是这一格**，动它只有风险没有收益 ✓。
const MIN_PREFIX: usize = 1;

/// 期望类型的**源级展开兜底**（本模块的唯一入口）：今天的展开坏掉时，用
/// 「补前导实参 + `Old` 读法展开」给出一条**干净**的归纳头形态 ✓。
///
/// 返回 `None` ⇒ 调用方**原样**用今天的期望类型（绝不更差 ✓）。
pub(crate) fn expected_to_inductive(
    e: &Expr,
    ctx: &ElabCtx<'_, '_>,
    scope: &ElabScope<'_>,
    known: &KnownTable,
) -> Option<Expr> {
    if !enabled() {
        return None;
    }
    // ⚠ 看的是**脊的头**（`(r • r) a c` 的顶层是 `App` ✗，头才是记法节点 ✓）。
    let (head, _) = crate::spine::spine_of(e);
    let Expr::Notation {
        target, lhs, rhs, ..
    } = head
    else {
        return None;
    };
    let operands: Vec<&Expr> = lhs.iter().chain(rhs.iter()).map(|b| b.as_ref()).collect();
    if operands.is_empty() {
        return None;
    }
    let (layers, params) = layers_of(target, known, ctx)?;
    let k = known.get(target.as_str())?.implicit_prefix();
    if k < MIN_PREFIX || k > layers.len() || params.len() != layers.len() {
        return None;
    }
    // ① **今天漏了哪些形参**（精确判据 ✓）：`Short` 读法（记法路径的常态）把操作数
    //    逐位对到**显式**形参 `params[k..]`、右对齐 ⇒ **没被任何操作数对上**的那些
    //    就是"留着裸名"的候选：`params[..k]` ∪ `params[k..][..offset]` ✓。
    //    与 scope-free 的"漏名判据"的区别：那个同名分不开 ✗（上一轮实测两次），
    //    这个**只看没被代换的那些位置** ✓。
    let explicit = &params[k..];
    let offset = explicit.len().saturating_sub(operands.len());
    let mut leak: Vec<String> = params[..k].to_vec();
    leak.extend(explicit[..offset].iter().cloned());
    leak.retain(|n| !n.is_empty());
    if leak.is_empty() {
        return None;
    }
    let is_inductive = |n: &str| ctx.inductives.contains_key(n);
    let today = crate::spine::unfold_to_inductive(
        e,
        &is_inductive,
        ctx.defs,
        8,
        None,
        crate::spine::UnfoldAlign::Short,
    );
    // 那些位**真的漏进结果里**了 ⇒ 今天坏掉 ✓；没漏（恰好同名/恰好不用）⇒ 一个字不动 ✓。
    if !mentions_names(&today, &leak) {
        return None;
    }
    // ② **递归**把期望类型里所有「带前导隐式形参的记法头」重写成全显式应用脊
    //    （操作数也要重写 ✓ —— 嵌套的记法头（`x ∈ f '' A` 里的 `f '' A`）不重写，
    //    展开的第二层仍会漏名 ✗，实测：`∃ (x : α), ((∅) x) ∧ …` 里那个 `α` 就是它）。
    let mut changed = false;
    let spine = crate::compile::expected::rewrite_bottom_up(e, &mut |n| {
        let r = notation_head_spine(n, ctx, scope, known);
        changed |= r.is_some();
        r
    });
    if !changed {
        return None;
    }
    // ③ 按 `Old` 展开（我们拼出的实参个数**正好**是形参数 ⇒ 逐位对齐、`offset == 0`
    //    ✓ ⇒ 每一层展开都**没有未代换的形参** ✓）。
    let out = crate::spine::unfold_to_inductive(
        &spine,
        &is_inductive,
        ctx.defs,
        8,
        None,
        crate::spine::UnfoldAlign::Old,
    );
    if out == spine {
        return None;
    }
    // 出口：展开结果必须**到得了归纳头** ✓（上一轮实测的教训：错位结果的头**也**是
    // `Exists` ⇒ 只看头不够 —— 但那条在本格不成立：本格的代换是**全覆盖**的 ✓）。
    let (out_head, _) = crate::spine::head_and_args(&out)?;
    if !is_inductive(out_head) {
        return None;
    }
    Some(out)
}

/// 记法目标的**望远镜**（`layers` = fresh 名，供求解 ✓）与**声明的真形参名**
/// （`params`，供漏名判据 ✓）。
///
/// ⚠ 真名必须从 `DefTable` 取 —— `implicit::telescope` 给的是 **fresh 名**
/// （`\0soko_p0`…，防捕获 ✓）⇒ 拿它当漏名表**永远查不到** ✗（实测：
/// `today = ∃ (b : B), …` 判成"没漏" ✗）。
fn layers_of(
    target: &str,
    known: &KnownTable,
    ctx: &ElabCtx<'_, '_>,
) -> Option<(Vec<Layer>, Vec<String>)> {
    let (layers, _) = crate::compile::implicit::telescope(known.get(target)?.signature()?)?;
    let params = ctx.defs.get(target)?.params.clone();
    Some((layers, params))
}

/// **单个**记法节点 ⇒ 全显式应用脊（`(r • r)` ⇒ `Rel.comp α α α r r` ✓）。
///
/// ⚠ **不带 spine 上的富余实参**（`a`/`c`）—— 那些由**外层的 `App` 节点**保留 ✓
/// （`rewrite_bottom_up` 只换这一个节点 ⇒ 整条脊仍是 `App(App(脊, a), c)` ✓）。
/// 不是记法节点 / 前导实参解不出 ⇒ `None`（原样 ✓）。
fn notation_head_spine(
    n: &Expr,
    ctx: &ElabCtx<'_, '_>,
    scope: &ElabScope<'_>,
    known: &KnownTable,
) -> Option<Expr> {
    let Expr::Notation {
        target,
        lhs,
        rhs,
        span,
        ..
    } = n
    else {
        return None;
    };
    let (layers, params) = layers_of(target, known, ctx)?;
    let k = known.get(target.as_str())?.implicit_prefix();
    if k < MIN_PREFIX || k > layers.len() || params.len() != layers.len() {
        return None;
    }
    let operands: Vec<&Expr> = lhs.iter().chain(rhs.iter()).map(|b| b.as_ref()).collect();
    if operands.is_empty() {
        return None;
    }
    let solved = solve_leading_args(&layers, &operands, k, ctx, scope, known)?;
    let mut out = Expr::Ident {
        name: target.clone(),
        span: *span,
    };
    for a in solved.iter().chain(operands.iter().copied()) {
        out = Expr::App {
            fun: Box::new(out),
            arg: Box::new(a.clone()),
            explicit_spine: true,
            span: *span,
        };
    }
    Some(out)
}

/// **零内核调用**的前导实参求解（与 `notation_prefix_args` 同一份对齐规则 ✓）。
fn solve_leading_args(
    layers: &[Layer],
    operands: &[&Expr],
    k: usize,
    ctx: &ElabCtx<'_, '_>,
    scope: &ElabScope<'_>,
    known: &KnownTable,
) -> Option<Vec<Expr>> {
    // ⚠ **只认 `missing == k` 这一读** ✗→✓（实测踩到）：记法的操作数**就是**显式形参
    // ⇒ 要补的**正好**是前导隐式的那 `k` 位 ✓。放宽成"从最大候选往下试"会挑出
    // `missing < k` 的**错位读法**（`(g ∘ f) '' A` ⇒ 解出 1 位、拼出只有 3 个实参的
    // `Set.image α (g ∘ f) A` ✗ —— 脊**缺一位** ⇒ `Old` 逐位对齐整体错位 ✗）。
    if operands.len() + k != layers.len() {
        return None;
    }
    solve_leading_exact(layers, k, operands, ctx, scope, known)
}

/// 固定 `missing` 时解前导参数；某一位解不出（或**没有名字**）⇒ `None`。
fn solve_leading_exact(
    layers: &[Layer],
    missing: usize,
    operands: &[&Expr],
    ctx: &ElabCtx<'_, '_>,
    scope: &ElabScope<'_>,
    known: &KnownTable,
) -> Option<Vec<Expr>> {
    if operands.len() > layers.len().saturating_sub(missing) {
        return None;
    }
    let mut solved: Vec<Expr> = Vec::with_capacity(missing);
    for i in 0..missing {
        let name = layers[i].name.clone();
        if name.is_empty() {
            return None;
        }
        let mut sigma: HashMap<String, Expr> = HashMap::new();
        for (j, v) in solved.iter().enumerate() {
            if !layers[j].name.is_empty() {
                sigma.insert(layers[j].name.clone(), v.clone());
            }
        }
        let mut found: Option<Expr> = None;
        // ① 由操作数解出：第一个提到 `name` 的**后续** binder 的域 vs 该操作数的类型。
        for (j, layer) in layers.iter().enumerate().skip(i + 1) {
            let Some(slot) = j.checked_sub(missing).filter(|s| *s < operands.len()) else {
                continue;
            };
            let domain = crate::spine::substitute(&layer.domain, &sigma);
            if !crate::spine::mentions(&name, &domain) {
                continue;
            }
            if let Some(v) = extract_param(&domain, operands[slot], &name, ctx, scope, known) {
                found = Some(v);
                break;
            }
        }
        solved.push(found?);
    }
    Some(solved)
}

/// **源级 δ 展开**（与求解器其它几处同一份机械：`unfold_to_inductive` + `Short`）✓。
fn unfold_src(e: &Expr, ctx: &ElabCtx<'_, '_>) -> Expr {
    crate::spine::unfold_to_inductive(
        e,
        &|n| ctx.inductives.contains_key(n),
        ctx.defs,
        8,
        None,
        crate::spine::UnfoldAlign::Short,
    )
}

/// 从**一个操作数**解 `name`：三条线索，全部**零内核调用** ✓。
///
/// ① 操作数的**源级类型**（[`operand_type_src`]）与模板头部匹配 —— 失败时按
///    **δ 展开兜底**（模板侧 / 实参侧 / 两侧；与 `solve_prefix_args_impl` 的 R5
///    同款：`Set`/`Rel` 是 **def** ⇒ 模板 `Set α` 与实参类型 `Two -> Prop` 头对不上 ✗）；
/// ② 操作数是**带标注的 λ**：模板的箭头逐层对到 binder 标注上
///    （[`unify_lambda_against`]）—— `Rel A B` 是 **def**、`fun (a : α) => …` 是
///    λ ⇒ ① 两条线索都拿不到（λ 的陪域要问内核 ✗）而这一条**只用源里的标注** ✓。
fn extract_param(
    domain: &Expr,
    operand: &Expr,
    name: &str,
    ctx: &ElabCtx<'_, '_>,
    scope: &ElabScope<'_>,
    known: &KnownTable,
) -> Option<Expr> {
    if let Some(actual) = operand_type_src(ctx, scope, known, operand) {
        if let Some(v) = super::elab::unify_extract(domain, &actual, name) {
            return Some(v);
        }
        let d = unfold_src(domain, ctx);
        let a = unfold_src(&actual, ctx);
        if d != *domain {
            if let Some(v) = super::elab::unify_extract(&d, &actual, name) {
                return Some(v);
            }
        }
        if a != actual {
            if let Some(v) = super::elab::unify_extract(domain, &a, name) {
                return Some(v);
            }
        }
        if d != *domain && a != actual {
            if let Some(v) = super::elab::unify_extract(&d, &a, name) {
                return Some(v);
            }
        }
    }
    unify_lambda_against(domain, operand, name, ctx)
}

/// 模板（δ 展开后的**箭头链**）逐层对到 λ 的 binder 标注上，抽出 `name` ✓。
///
/// `Rel A B`（⇒ `A → B → Prop`）对 `fun (a : α) => fun (b : β) => …`
/// ⇒ 第 1 层 `A ≟ α`、第 2 层 `B ≟ β` ✓。
///
/// ⚠ 只在**每一层 binder 都有标注**时走（没有标注就没有源级线索 ⇒ 放弃 ✓）；
/// 上界 8 层（与 `b2_whnf_peel`/`unfold_to_inductive` 同一个上界 ✓）。
fn unify_lambda_against(
    domain: &Expr,
    operand: &Expr,
    name: &str,
    ctx: &ElabCtx<'_, '_>,
) -> Option<Expr> {
    let mut t = domain.clone();
    let mut o = operand.clone();
    for i in 0..8 {
        let Expr::Lambda { binders, body, .. } = &o else {
            return None;
        };
        let binder = binders.first()?;
        let ty = binder.ty.as_deref()?;
        // 模板这一层：`Arrow` / 单 binder 的 `Forall`；都不是就 δ 展开一层再看。
        let (dom, cod) = match crate::spine::peel_pi(&t) {
            Some(pi) => (pi.domain, pi.body),
            None => {
                if i == 0 {
                    t = unfold_src(&t, ctx);
                    continue;
                }
                return None;
            }
        };
        if let Some(v) = super::elab::unify_extract(&dom, ty, name) {
            return Some(v);
        }
        // 推进：λ 去掉第一个 binder（多 binder 折成一层 `Lambda` ✓）。
        o = if binders.len() > 1 {
            Expr::Lambda {
                binders: binders[1..].to_vec(),
                body: body.clone(),
                span: o.span(),
            }
        } else {
            (**body).clone()
        };
        t = cod;
    }
    None
}

/// 操作数的**类型**（源级 AST）——**只从源级线索取，零内核调用** ✓。
///
/// 与 `elab::operand_type_expr` 的差别**只有一条**：最后那条 `infer_type_text`
/// 兜底（问内核 ⇒ 重编前缀 ⇒ 递归 ✗）换成了**注册表签名** ✓。
fn operand_type_src(
    ctx: &ElabCtx<'_, '_>,
    scope: &ElabScope<'_>,
    known: &KnownTable,
    operand: &Expr,
) -> Option<Expr> {
    // ① 局部变量：**书写类型**（`r : Rel α α` ⇒ `Rel α α` ✓；pp 会丢隐式实参 ✗）。
    if let Expr::Ident { name, .. } = operand {
        if let Some(src) = scope.source_type_of(name) {
            return Some(src);
        }
    }
    // ② λ：源级 binder 注解拼出来的箭头类型（与 `elab::lambda_source_type` 同一份 ✓）。
    if let Some(src) = super::elab::lambda_source_type(operand) {
        return Some(src);
    }
    // ③ 常量（`f1` 这类 `def`）：**注册表签名文本** ⇒ Π 望远镜 ⇒ 箭头链 ✓。
    const_type_src(ctx, scope, known, operand)
}

/// 常量操作数的类型：注册表的**源级签名**（零内核调用 ✓）。
///
/// 只在「签名全是**显式** binder、且**非依赖**」时给出箭头链
/// （`def f1 (t : Two) : Two` ⇒ `Two → Two` ✓）—— 其余形状 `None`
/// （= 今天的行为：拿不到类型 ⇒ 那一处不补前导实参 ⇒ 期望类型原样 ✓）。
///
/// **`Forall` 归一成 `Arrow`**（任务点名的那一条 ✓）：签名文本是
/// `(t : Two) -> Two`（`Forall`），而模板（`Set.image` 的 `f : α → β`）是
/// `Arrow` ⇒ 不归一两边头对不上 ✗。
fn const_type_src(
    ctx: &ElabCtx<'_, '_>,
    scope: &ElabScope<'_>,
    known: &KnownTable,
    operand: &Expr,
) -> Option<Expr> {
    let (head, args) = crate::spine::spine_of(operand);
    let raw = match head {
        Expr::Ident { name, .. } | Expr::UniverseApp { name, .. } => name.as_str(),
        _ => return None,
    };
    // 遮蔽：局部名不是常量（局部那一档已经答过 ✓）。
    if scope.local_names().iter().any(|n| n == raw) {
        return None;
    }
    let canonical =
        super::elab::resolve_known(known, ctx.ns, raw, head.span(), scope.local_names()).ok()?;
    let (layers, result) =
        crate::compile::implicit::telescope(known.get(&canonical)?.signature()?)?;
    if args.len() > layers.len() {
        return None;
    }
    // **全显式脊**（本模块自己拼出来的形态：`Set.image γ γ g (…)` ✓）：实参**逐位**
    // 对全部形参 ⇒ 类型就是**结果类型的代换** ✓（零内核调用）。
    if args.len() == layers.len() {
        let mut sigma: HashMap<String, Expr> = HashMap::new();
        for (i, a) in args.iter().enumerate() {
            if !layers[i].name.is_empty() {
                sigma.insert(layers[i].name.clone(), (*a).clone());
            }
        }
        return Some(crate::spine::substitute(&result, &sigma));
    }
    let mut sigma: HashMap<String, Expr> = HashMap::new();
    for (i, a) in args.iter().enumerate() {
        if layers[i].style != BinderKind::Explicit {
            return None;
        }
        if !layers[i].name.is_empty() {
            sigma.insert(layers[i].name.clone(), (*a).clone());
        }
    }
    // 剩下的层必须**全是显式**、且**都不出现在**后面的域/结果里（非依赖 ✓）。
    let rest = &layers[args.len()..];
    if rest.iter().any(|l| l.style != BinderKind::Explicit) {
        return None;
    }
    let tail = crate::spine::substitute(&result, &sigma);
    let tail_names: Vec<String> = rest
        .iter()
        .map(|l| l.name.clone())
        .filter(|n| !n.is_empty())
        .collect();
    if mentions_names(&tail, &tail_names)
        || rest.iter().any(|l| {
            let d = crate::spine::substitute(&l.domain, &sigma);
            mentions_names(&d, &tail_names)
        })
    {
        return None;
    }
    let mut ty = tail;
    for layer in rest.iter().rev() {
        let domain = crate::spine::substitute(&layer.domain, &sigma);
        ty = Expr::Arrow {
            domain: Box::new(domain),
            codomain: Box::new(ty),
            span: Span::default(),
        };
    }
    Some(ty)
}

use super::expected::mentions_names;

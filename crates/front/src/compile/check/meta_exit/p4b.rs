//! **B2-P4b（2026-10-26）**：应用结点的**组装点** —— ① 结果类型 ≟ 期望类型那条方程
//! （Lean `Elab/App.lean:363/405` 的 `isDefEq expectedType fTypeBody`）②「先实参、再隐式
//! 实参 + 装前 zonk」的顺序（让那条方程**来得及**把载体换成真项）。
//!
//! ## 为什么需要（P4a 的实测读数）
//!
//! P0 形状 `Set.Equiv (Set.image Set.singleton A) B` 里，外层 `Set.Equiv` 的前导
//! `α` **只能**由内层项的结果类型给（`Set.image … : Set β`）；反过来内层
//! `Set.image` 的 `β` 又只能由外层的**期望类型** `Set ?α` 给。⇒ 两层之间必须有
//! 一条方程把它们**连在同一个 store 里**，而且必须在**装项之前**解出来。
//!
//! 两条都缺的时候实测（P4a 收口）：`unsolved=0`（方程都解了）项里却还留着载体
//! `\0soko_m{id}` ⇒ `add_declar` 硬拒 ⇒ 声明出口只能重放闸门关 ⇒ 红。
//!
//! * **方程**：今天只有**引擎档**有（`implicit::solve_prefix_meta_alt` 的路线 ④），
//!   而**贪心档先跑**（`solve_prefix_impl_alt`）⇒ 它一成功引擎一步都不进 ⇒ 那条方程
//!   只活在贪心求解器的**局部 `sigma`** 里（store 什么都不知道 ✗）⇒ 用
//!   [`app_result_equation`] 在贪心出口补一条；
//! * **顺序**：今天的组装是「先装隐式实参的项、再 elaborate 实参」—— 隐式实参的值
//!   此刻往往还是**载体** ⇒ 装项那一刻就成了核侧 `Expr::Meta` ⇒ **写死了** ✗。
//!   改成「先实参（它们把隐式位的值解出来）、再隐式实参」并在装之前 `zonk`
//!   （[`assemble_app`]）⇒ 载体不进项 ✓。
//!
//! ## 三条硬约束（改之前先读）
//!
//! 1. **闸门关（`SOKO_DECL_MVARS` 默认关）/ 没有 store ⇒ 一步都不进**：`assemble_app` 的
//!    闸门关分支就是**今天那两段逐字**（顺序、期望类型、`solved_out` 全不动）⇒ `--json`
//!    逐字节不变 ✓（7 个基线文件 + `/tmp/p4a-base` 对拍）；`app_result_equation` 一次都不跑 ✓。
//! 2. **`Tri::No` 不落盘**：与 `checkpointDefEq` 同款（失败即回滚）—— 期望类型是**源级
//!    近似**，判 `No` 只说明"这条近似对不上"，**不许**用它造新的拒绝面 ✓。
//! 3. **`?p a…` 形状不进方程**（[`applied_meta_head`]）：引擎的结构分解是**按头部**做的
//!    （`meta.rs::unify_body_inner` 的 `App/App` 臂）⇒ `And (P m) X ≟ ?p m` 会把 `?p`
//!    赋成**部分应用** `And (P m)` ⇒ 类型对不上（实测：`courses/set-theory/lib/Order`
//!    的 `hasMin_intro` 被内核拒成「期望 `α → Prop`，实际 `Type → Type`」）。真正的
//!    修法是 **pattern 合一**（HO3/HO4，设计 `metavar-pattern.md` §3.2）⇒ 本片**不碰**
//!    那条路，方程只负责「刚性头 ≟ 刚性头」。
//!
//! 设计 ⇒ `docs/design/metavar-engine.md` §2.10 / §4 B2 · `docs/design/metavar-pattern.md`
//! §3.4/§7（B2-P4b，**无内核改动**）。
//!
//! ⚠ 为什么组装点在本文件、而不在 `elab.rs`：`elab.rs` 受判据 ⑨ 冻结（8843 行，改动前
//! 8837）⇒ 把**整段**组装搬出来是唯一能既加机制又不涨债的写法（`elab.rs` 反而变小 ✓）。

use std::collections::HashMap;

use sokonanoda::util::ExprPtr;

use crate::ast::Expr;
use crate::compile::elab::{
    elab_expr, DefTable, ElabCtx, ElabScope, HoverNode, KnownTable, UnivMap,
};
use crate::compile::error::CompileError;
use crate::compile::implicit::Layer;
use crate::compile::meta::{decl_mvars_enabled, DeclMVarStore, Tri};

/// **P4b 的那条方程**：`结果(前导值 + 实际实参代换) ≟ expected`。
///
/// `solved` = 前导 `k` 层解出来的值（按层序）；`arg_vals` = 写出来的实参本身
/// （源级：与 `solve_prefix_meta_alt` 的 `arg_vals` 同一条约定 —— 显式层的形式名
/// 映射到**实际实参**，不是它们的类型 ✓）。`layers[k + j]` 对应 `arg_vals[j]`。
///
/// `decl_store == None`（闸门关 / 非声明通道）/ `expected == None`（实参位没有期望
/// 类型）⇒ **不调用**（零开销、零行为变化 ✓）。`result_ty` 就是 Lean 的
/// `fTypeBody.instantiate1 arg` 做完之后的**结果类型** ✓。
#[allow(clippy::too_many_arguments)]
pub(crate) fn app_result_equation(
    decl_store: Option<&DeclMVarStore>,
    layers: &[Layer],
    result: &Expr,
    k: usize,
    solved: &[Expr],
    arg_vals: &[&Expr],
    expected: Option<&Expr>,
    defs: &DefTable,
    is_inductive: &dyn Fn(&str) -> bool,
) {
    let (Some(store), Some(expected)) = (decl_store, expected) else {
        return;
    };
    if !decl_mvars_enabled() {
        return;
    }
    let result_ty = result_after_args(layers, result, k, solved, arg_vals);
    // 只有带载体的应用才需要这条方程（全具体值走旧路，避免误伤 def-headed 形状）。
    // 载体可能在 solved、result 或 expected 里（P0 的 `Set ?α` 就只在 expected/层域里）。
    if !(solved.iter().any(has_carrier) || has_carrier(&result_ty) || has_carrier(expected)) {
        return;
    }
    // 刚性头对刚性头（见模块头第 3 条：`?p a…` 不进方程）。
    if applied_meta_head(&result_ty) || applied_meta_head(expected) {
        return;
    }
    let unfold = |e: &Expr| {
        crate::spine::unfold_to_inductive(
            e,
            is_inductive,
            defs,
            8,
            None,
            crate::spine::UnfoldAlign::Short,
        )
    };
    let mut guard = store.park();
    if !guard.is_shared() {
        return;
    }
    let tri = {
        let mut meta = guard.meta_ctx(&unfold);
        meta.unify(&result_ty, expected)
    };
    // `No` ⇒ 回滚（源级近似对不上，不许造拒绝面）；`Yes`/`Undef` ⇒ 赋值与待定约束留下 ✓。
    if tri != Tri::No {
        guard.commit();
    }
}

/// `solved` 里是否真的带**载体**（`\0soko_m{id}`）。只有这种应用才需要
/// 「先实参、后隐式」的重排 + 结果方程；全具体值 ⇒ 走旧顺序（G-73 `And.left hpo a`
/// 这种 def-headed 应用曾被重排误伤）。
fn has_carrier(e: &Expr) -> bool {
    if let Expr::Ident { name, .. } = e {
        if crate::compile::meta::is_meta_name(name) {
            return true;
        }
    }
    let mut found = false;
    let _ = crate::spine::map_children(e, &mut |child| {
        if !found && has_carrier(child) {
            found = true;
        }
        child.clone()
    });
    found
}

/// **`?p a…` 形状的闸门**（P4b 实测；**健全性**）：见模块头第 3 条。
fn applied_meta_head(e: &Expr) -> bool {
    let (head, args) = crate::spine::spine_of(e);
    !args.is_empty() && crate::compile::meta::meta_of_expr(head).is_some()
}

/// 结果类型：先按「前导各层的**值**」代换，再按「显式各层的**实际实参**」代换。
fn result_after_args(
    layers: &[Layer],
    result: &Expr,
    k: usize,
    solved: &[Expr],
    arg_vals: &[&Expr],
) -> Expr {
    let mut sigma: HashMap<String, Expr> = HashMap::new();
    for (i, value) in solved.iter().enumerate() {
        if let Some(layer) = layers.get(i) {
            if !layer.name.is_empty() {
                sigma.insert(layer.name.clone(), value.clone());
            }
        }
    }
    for (j, arg) in arg_vals.iter().enumerate() {
        if let Some(layer) = layers.get(k + j) {
            if !layer.name.is_empty() {
                sigma.insert(layer.name.clone(), (*arg).clone());
            }
        }
    }
    crate::spine::substitute(result, &sigma)
}

/// 组装一次应用的**全部上下文引用**（`elab.rs` 的调用点因此只写一行结构体字面量 ——
/// 那条路径受 ⑨ 冻结，参数表写平会逼 rustfmt 一参一行 ✗）。
pub(crate) struct Asm<'a, 'b, 'x> {
    pub builder: &'x mut sokonanoda::builder::EnvBuilder<'a>,
    pub scope: &'x mut ElabScope<'a>,
    pub univ: &'x UnivMap<'a>,
    pub known: &'x KnownTable,
    pub hovers: &'x mut Vec<HoverNode<'a>>,
    pub ctx: &'x ElabCtx<'a, 'b>,
}

/// **应用结点的唯一组装点**（`elab_expr` 的 `Expr::App` 主路）。
///
/// * **闸门关 / 没有 store** ⇒ 跑**今天那两段**（逐字：先隐式实参、再实参、再 `solved_out`）
///   ⇒ 逐字节不变 ✓；
/// * **闸门开 + 有 store** ⇒ 先实参、再隐式实参，并在装隐式实参之前 `zonk`
///   （[`zonk_value`]）⇒ 刚解出来的值直接落成真项，载体不进项 ✓。
///
/// 两条路的**期望类型**是同一份（`substitute(layers[i].domain, sigma)`，sigma 进度也同一份）
/// ⇒ 每个实参拿到的期望类型逐字相同，差别只有 elaborate 的先后 ✓。
#[allow(clippy::too_many_arguments)]
pub(crate) fn assemble_app<'a>(
    asm: &mut Asm<'a, '_, '_>,
    head: ExprPtr<'a>,
    layers: &[Layer],
    k: usize,
    solved: &[Expr],
    args: &[&Expr],
    mut solved_out: Option<&mut Vec<Expr>>,
) -> Result<ExprPtr<'a>, CompileError> {
    let Asm {
        builder,
        scope,
        univ,
        known,
        hovers,
        ctx,
    } = asm;
    let (builder, scope, hovers) = (&mut **builder, &mut **scope, &mut **hovers);
    let (univ, known, ctx) = (&**univ, *known, *ctx);
    let mut sigma: HashMap<String, Expr> = HashMap::new();
    // ── 闸门开 **且有载体**：先实参、再隐式实参（装前 zonk）──
    let carries = solved.iter().any(has_carrier)
        || layers.iter().any(|l| has_carrier(&l.domain))
        || args.iter().any(|a| has_carrier(a));
    if ctx.metas.is_some() && decl_mvars_enabled() && carries {
        return reordered(
            builder, scope, univ, known, hovers, ctx, head, layers, k, solved, args, &mut sigma,
            solved_out,
        );
    }
    // ── 闸门关 / 没有 store：**今天那两段逐字** ──
    let mut out = head;
    for (j, s) in solved.iter().enumerate() {
        // **T-N13**：期望类型 = 该层的域（pp 会丢第一个隐式实参 ⇒ 不给就停在 Pi 上 ⇒ 形状不符）。
        let expected_src = crate::spine::substitute(&layers[j].domain, &sigma);
        if !layers[j].name.is_empty() {
            sigma.insert(layers[j].name.clone(), s.clone());
        }
        let t = crate::compile::pp_shape::elab_engine_value(
            builder,
            s,
            scope,
            univ,
            known,
            hovers,
            None,
            Some(&expected_src),
            ctx,
        )?;
        out = builder.mk_app(out, t);
    }
    for (i, a) in args.iter().enumerate() {
        let li = k + i;
        let expected_src = crate::spine::substitute(&layers[li].domain, &sigma);
        let t = elab_expr(
            builder,
            a,
            scope,
            univ,
            known,
            hovers,
            None,
            Some(&expected_src),
            ctx,
        )?;
        out = builder.mk_app(out, t);
        if !layers[li].name.is_empty() {
            sigma.insert(layers[li].name.clone(), (*a).clone());
        }
    }
    if let Some(sink) = solved_out.as_mut() {
        sink.extend(
            layers
                .iter()
                .take(k)
                .filter(|l| !l.name.is_empty())
                .filter_map(|l| sigma.get(&l.name).cloned()),
        );
    }
    Ok(out)
}

/// 闸门开的装配顺序（见模块头）。期望类型与今天**逐字同一份**，只是 elaborate 的先后变了。
#[allow(clippy::too_many_arguments)]
fn reordered<'a>(
    builder: &mut sokonanoda::builder::EnvBuilder<'a>,
    scope: &mut ElabScope<'a>,
    univ: &UnivMap<'a>,
    known: &KnownTable,
    hovers: &mut Vec<HoverNode<'a>>,
    ctx: &ElabCtx<'a, '_>,
    head: ExprPtr<'a>,
    layers: &[Layer],
    k: usize,
    solved: &[Expr],
    args: &[&Expr],
    sigma: &mut HashMap<String, Expr>,
    mut solved_out: Option<&mut Vec<Expr>>,
) -> Result<ExprPtr<'a>, CompileError> {
    // ① 隐式实参的**期望类型**（与今天同一个 sigma 进度：此刻 sigma 只有更早的隐式值）。
    let mut impl_expected: Vec<Expr> = Vec::with_capacity(k);
    for (j, value) in solved.iter().enumerate() {
        impl_expected.push(crate::spine::substitute(&layers[j].domain, sigma));
        if !layers[j].name.is_empty() {
            sigma.insert(layers[j].name.clone(), value.clone());
        }
    }
    // ② **先** elaborate 写出来的实参（这一步会把隐式位的值解出来）。
    let mut arg_terms: Vec<ExprPtr<'a>> = Vec::with_capacity(args.len());
    for (i, arg) in args.iter().enumerate() {
        let li = k + i;
        let expected_src = crate::spine::substitute(&layers[li].domain, sigma);
        let term = elab_expr(
            builder,
            arg,
            scope,
            univ,
            known,
            hovers,
            None,
            Some(&expected_src),
            ctx,
        )?;
        arg_terms.push(term);
        if !layers[li].name.is_empty() {
            sigma.insert(layers[li].name.clone(), (*arg).clone());
        }
    }
    // ③ **再**装隐式实参：装之前 zonk ⇒ 刚刚解出来的值直接落成真项（不留载体 ✓）。
    let mut out = head;
    for (j, value) in solved.iter().enumerate() {
        let zonked = zonk_value(ctx, value);
        let term = crate::compile::pp_shape::elab_engine_value(
            builder,
            &zonked,
            scope,
            univ,
            known,
            hovers,
            None,
            Some(&impl_expected[j]),
            ctx,
        )?;
        out = builder.mk_app(out, term);
    }
    for term in arg_terms {
        out = builder.mk_app(out, term);
    }
    if let Some(sink) = solved_out.as_mut() {
        sink.extend(
            layers
                .iter()
                .take(k)
                .filter(|l| !l.name.is_empty())
                .filter_map(|l| sigma.get(&l.name).cloned()),
        );
    }
    Ok(out)
}

/// 用**当前** store 的值把 `e` 里的元变量代掉（`e` 仍是载体就原样返回）。
///
/// store 被外层求解占着（scratch）⇒ 原样返回（嵌套求解看不见外层，与 P1 的边界一致 ✓）；
/// **不改 store**（`zonk` 是只读的；`Drop` 的回滚因此是无操作 ✓）。
fn zonk_value<'a>(ctx: &ElabCtx<'a, '_>, e: &Expr) -> Expr {
    let Some(store) = ctx.metas.and_then(|m| m.store.as_ref()) else {
        return e.clone();
    };
    let unfold = |x: &Expr| {
        crate::spine::unfold_to_inductive(
            x,
            &|n| ctx.inductives.contains_key(n),
            ctx.defs,
            8,
            None,
            crate::spine::UnfoldAlign::Short,
        )
    };
    let mut guard = store.park();
    if !guard.is_shared() {
        return e.clone();
    }
    guard.meta_ctx(&unfold).zonk(e)
}

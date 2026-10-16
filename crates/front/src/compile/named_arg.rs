//! **命名实参**（`g (α := Nat) 5`）—— 用户 2026-10-16 第 ② 条
//! （队列 `SUB-implicit-named-args`）。
//!
//! 用户原话（逐字）：「……真不如**对齐 lean**：**只有 `@` 形式接收隐式参数，正常形式
//! 不接受隐式参数**，并且**支持显式地指定参数名字**。」
//!
//! **Lean 4 对照**（本机源码 `~/Documents/lean/lean4/src/Lean/Elab/App.lean` ✓）：
//! * `:753` `main` 的主循环**先**问 `findBinderName? s.namedArgs binderName`
//!   （`:87` = 按 binder 名线性查找）—— **命中就用它、且不消耗位置实参**，
//!   **无论 binder 是隐式、strict 隐式、instance 隐式还是显式**（那条 `match binfo`
//!   在 `| none =>` 分支里 ⇒ 命名实参**优先于风格** ✓）；
//! * `:1073` `getNextArg?` 同款：**先命名、后位置**，且位置只在 `binderInfo.isExplicit`
//!   时才吃（这条注释正是「隐式实参**不是用命名实参给的**就拿不到」的出处）；
//! * `:238` 在 `namedArgs` 有剩余时抛 `throwInvalidNamedArg`（`:39`：一个码 + 列出
//!   **可用形参名** + code action）⇒ 我们沿用同一条契约（见 `implicit_msg`）✓。
//!
//! **我们怎么做**：形参名取**源级签名文本**（`implicit_msg::source_layer_names`，
//! 与 `implicit::telescope` 同序 ✓）；把命名层从望远镜里**摘掉**、值代换进后面各层，
//! 剩下的事**完全复用**既有求解器（`implicit::solve_prefix_with_args`）⇒ 隐式求解
//! 那套安全性质（解不出就报错、不猜不搜索）**一个字节都不动** ✓。
//!
//! **与 Lean 的差异（如实 ✗）**：
//! ① 头必须是**已知常量**（有源级签名可查）—— 局部假设（`intro`/`have` 进来的）
//!    还绑不了名（Lean 用 `forall` 的 binder 名，我们还没有那条路）；
//! ② 富余位置实参（落到**结果类型**上的那一档，路线 ③）暂不与命名实参组合
//!    ⇒ 那种形状 `return Ok(None)` 交回老路（老路给它自己的诊断 ✓，**不静默**）。
//!
//! 两条差异都**只影响"命名实参 + 那种形状"**：没有命名实参时本模块**一次都不进** ✓
//! ⇒ 既有语料逐字节不变 ✓。

use std::collections::HashMap;

use crate::ast::Expr;
use crate::compile::elab::{elab_expr, ElabCtx, ElabScope, HoverNode, KnownTable, UnivMap};
use crate::compile::error::{CompileError, ErrorKind};
use crate::compile::implicit::{self, Layer};
use sokonanoda::builder::EnvBuilder;
use sokonanoda::util::ExprPtr;

/// 这条应用脊上有没有命名实参（**零成本**：没有就是一次线性扫描 ✓）。
pub(crate) fn has_named(args: &[&Expr]) -> bool {
    args.iter().any(|a| matches!(a, Expr::NamedArg { .. }))
}

/// 见 [`has_named`]：直接从**整条应用节点**问（钩子在 `spine_of` 之前要用它 ✓）。
pub(crate) fn spine_has_named(expr: &Expr) -> bool {
    has_named(&crate::spine::spine_of(expr).1)
}

/// 把命名实参应用落到内核项上。返回 `Ok(None)` ⇒ 交回老路（**绝不静默**）。
#[allow(clippy::too_many_arguments)]
pub(crate) fn apply<'a>(
    builder: &mut EnvBuilder<'a>,
    expr: &Expr,
    head_src: &Expr,
    ty_text: &str,
    layers: &[Layer],
    result: &Expr,
    k: usize,
    args: &[&Expr],
    scope: &mut ElabScope<'a>,
    univ: &UnivMap<'a>,
    known: &KnownTable,
    hovers: &mut Vec<HoverNode<'a>>,
    expected_src: Option<&Expr>,
    ctx: &ElabCtx<'a, '_>,
    mut solved_out: Option<&mut Vec<Expr>>,
) -> Result<Option<ExprPtr<'a>>, CompileError> {
    let app_span = expr.span();
    let Some(names) = crate::compile::implicit_msg::source_layer_names(ty_text) else {
        return Ok(None);
    };
    // ── 1) 按**源级形参名**分配（Lean `App.lean:753` 的 `findBinderName?` ✓）──────
    let mut given: Vec<Option<Expr>> = vec![None; layers.len()];
    let mut positional: Vec<Expr> = Vec::new();
    for a in args {
        match a {
            Expr::NamedArg { name, value, span } => {
                let target = names.iter().position(|(n, _)| n == name);
                let Some(j) = target.filter(|j| *j < layers.len()) else {
                    return Err(CompileError::elab(
                        ErrorKind::ElabInvalidNamedArgument,
                        crate::compile::implicit_msg::unknown_named_arg_message(
                            name, &names, ty_text,
                        ),
                        *span,
                    ));
                };
                if given[j].is_some() {
                    return Err(CompileError::elab(
                        ErrorKind::ElabInvalidNamedArgument,
                        crate::compile::implicit_msg::duplicate_named_arg_message(name, ty_text),
                        *span,
                    ));
                }
                given[j] = Some((**value).clone());
            }
            other => positional.push((*other).clone()),
        }
    }
    // ── 2) 缩简望远镜：命名层**摘掉**，值代换进后面各层 ✓ ──────────────────────
    let mut sigma: HashMap<String, Expr> = HashMap::new();
    let mut red: Vec<Layer> = Vec::new();
    let mut red_src: Vec<usize> = Vec::new();
    for j in 0..layers.len() {
        let domain = crate::spine::substitute(&layers[j].domain, &sigma);
        match given[j].clone() {
            Some(v) => {
                if !layers[j].name.is_empty() {
                    sigma.insert(layers[j].name.clone(), v);
                }
            }
            None => {
                red.push(Layer {
                    name: layers[j].name.clone(),
                    domain,
                    style: layers[j].style.clone(),
                });
                red_src.push(j);
            }
        }
    }
    let result_sub = crate::spine::substitute(result, &sigma);
    let red_k = red_src.iter().take_while(|&&j| j < k).count();
    if positional.len() > red.len() - red_k {
        return Ok(None); // 富余实参（路线 ③）暂不与命名实参组合 ⇒ 交回老路 ✓
    }
    // ── 3) 解**剩下**的隐式前缀（原料 = 位置实参的类型；求解器本体零改动 ✓）──────
    let pos_refs: Vec<&Expr> = positional.iter().collect();
    let solved: Vec<Expr> = if red_k == 0 {
        Vec::new()
    } else {
        let arg_tys =
            crate::compile::pp_shape::arg_types(&pos_refs, ctx, scope, known, &mut *builder);
        let is_inductive = |n: &str| ctx.inductives.contains_key(n);
        match implicit::solve_prefix_with_args(
            &red,
            &result_sub,
            red_k,
            &arg_tys,
            &pos_refs,
            expected_src,
            ctx.defs,
            &is_inductive,
        ) {
            Some(v) => v,
            None => {
                return Err(CompileError::elab(
                    ErrorKind::ElabImplicitArgumentUnsolved,
                    crate::compile::implicit_msg::named_arg_unsolved_message(ty_text, k),
                    app_span,
                ));
            }
        }
    };
    // ── 4) 逐层定值：命名 ✓ → 隐式（解出来的）✓ → 位置（**按剩余显式层的顺序** ✓）──
    let mut values: Vec<Option<Expr>> = given.clone();
    for (i, j) in red_src.iter().enumerate() {
        if *j < k {
            values[*j] = solved.get(i).cloned();
        }
    }
    let mut it = positional.iter();
    for (j, slot) in values.iter_mut().enumerate() {
        if slot.is_none() && j >= k {
            if let Some(v) = it.next() {
                *slot = Some((*v).clone());
            }
        }
    }
    let last_filled = match values.iter().rposition(|v| v.is_some()) {
        Some(j) => j,
        None => return Ok(None),
    };
    if values[..=last_filled].iter().any(|v| v.is_none()) {
        return Ok(None); // 中间有洞（位置实参不够够到后面的命名层）⇒ 交回老路 ✓
    }
    // ── 5) 装配：与主路径**同一口径**（每层都吃「代入后」的期望类型 ✓）───────────
    let mut out = elab_expr(
        builder, head_src, scope, univ, known, hovers, None, None, ctx,
    )?;
    let mut sigma2: HashMap<String, Expr> = HashMap::new();
    let mut prefix_vals: Vec<Expr> = Vec::new();
    for j in 0..=last_filled {
        let Some(value) = values[j].clone() else {
            return Ok(None);
        };
        let expected = crate::spine::substitute(&layers[j].domain, &sigma2);
        let term = elab_expr(
            builder,
            &value,
            scope,
            univ,
            known,
            hovers,
            None,
            Some(&expected),
            ctx,
        )?;
        out = builder.mk_app(out, term);
        if !layers[j].name.is_empty() {
            sigma2.insert(layers[j].name.clone(), value.clone());
        }
        if j < k {
            prefix_vals.push(value);
        }
    }
    if let Some(sink) = solved_out.as_mut() {
        sink.extend(prefix_vals);
    }
    Ok(Some(out))
}

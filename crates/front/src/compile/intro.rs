//! 值位 `funintro` 关键字的降低：把声明类型最外层的 Pi 望远镜全部剥成显式
//! lambda，末端一个洞（`fun (a : Prop) => … => sorry`）。
//!
//! 只做结构性展开，不做判定：降低后的值与普通的部分作答走同一条
//! `open_goal` 流水线，填洞结果仍由完整内核终审（REQUIREMENTS §2 第 8 条）。
//! 骨架文本（`render_expr` 产物）同时回传，作为编辑器补全项的单一事实源。
//!
//! **组合**（I13-S3，`docs/design/value-keywords-v2.md` §3.2）：答案里可以再
//! 嵌关键字——`funintro (funapply X)` = 剥掉全部 binder 后，`funapply X` 面对剥
//! 完的最终目标降低为部分应用（`X <σ 实参> sorry …`）。内核永远看不到关键字。

use super::apply::lower_at;
use super::error::{CompileError, ErrorKind};
use super::CompileOptions;
use crate::judge::GoalBinderSpec;
use crate::proof::{fresh_name, peel_pi_layers, render_expr};
use crate::{Binder, BinderKind, Expr, Span};
use std::collections::HashSet;

/// 值位恰为 `funintro` 时返回 `(降低后的 lambda + 洞, 显式骨架文本)`；
/// 其它值原样透传（`None`）。目标没有可剥的 binder 时返回教学错误。
/// 声明 binder 会把值包成 lambda 链，因此这里沿链下降到体部再处理。
///
/// `src` / `span_start` / `options` 透传给组合形态里的 `funapply` 降低
/// （它要问内核推断被应用项的类型）。
pub(crate) fn lower_intro_val(
    ty: &Expr,
    val: &Expr,
    src: &str,
    span_start: usize,
    options: &CompileOptions,
) -> Result<Option<(Expr, String)>, CompileError> {
    lower_inner(
        ty,
        val,
        &mut Vec::new(),
        &mut Vec::new(),
        src,
        span_start,
        options,
    )
}

/// `outer` 是**外层 lambda 已占用**的 binder 名（含声明 binder与学习者写的
/// lambda）。intro 展开骨架的合成 binder 必须避开它们——嵌套形态下骨架
/// 只替换 intro token，若与外层撞名会产生 `fun (x : Q) => fun (x : Q) => …`
/// 这样的遮蔽（内层可用但极易误读）。
///
/// `context` 同步收集**作用域内的 binder**（外层链 + funintro 剥出的层），
/// 供组合形态里的 `funapply` 问内核推断类型时构造合成 fun 链。
#[allow(clippy::too_many_arguments)]
fn lower_inner(
    ty: &Expr,
    val: &Expr,
    outer: &mut Vec<String>,
    context: &mut Vec<GoalBinderSpec>,
    src: &str,
    span_start: usize,
    options: &CompileOptions,
) -> Result<Option<(Expr, String)>, CompileError> {
    match val {
        Expr::Intro { answer, span } => {
            // `funintro` 与 `funintro <answer>` 都走这里：前者末端是洞（练习），
            // 后者把答案填进骨架末端——答案本身还可以是 `funapply`/`funintro`
            // 节点（组合，见 `peel_all_pi`）。
            let expr = peel_all_pi(
                ty,
                *span,
                answer.as_deref(),
                outer,
                context,
                src,
                span_start,
                options,
            )?;
            let skeleton = render_expr(&expr);
            Ok(Some((expr, skeleton)))
        }
        // 声明 binder 降级后的 lambda 链：沿链消耗类型层，在体尾处理关键字。
        // 骨架只覆盖关键字自身的展开（补全替换的是关键字 token）。
        Expr::Lambda {
            binders,
            body,
            span,
        } => {
            let Some(rest_ty) = peel_pi_layers(ty, binders.len()) else {
                return Ok(None);
            };
            let pushed = binders.len();
            outer.extend(binders.iter().map(|b| b.name.clone()));
            context.extend(binders.iter().map(|b| GoalBinderSpec {
                name: b.name.clone(),
                ty: b.ty.as_deref().map(render_expr),
            }));
            match lower_inner(&rest_ty, body, outer, context, src, span_start, options)? {
                Some((new_body, skeleton)) => {
                    outer.truncate(outer.len() - pushed);
                    Ok(Some((
                        Expr::Lambda {
                            binders: binders.clone(),
                            body: Box::new(new_body),
                            span: *span,
                        },
                        skeleton,
                    )))
                }
                None => {
                    outer.truncate(outer.len() - pushed);
                    Ok(None)
                }
            }
        }
        _ => Ok(None),
    }
}

/// 全剥 `ty` 最外层的 Forall binder / Arrow 域，包成一层层单 binder 的
/// `Expr::Lambda`；匿名层用生成器约定的基名 `x`（`x2` 防撞，与
/// `suggest::restart_skeleton` 同源）。每个合成节点的 span 都是 `funintro`
/// token——洞需要它，binder 本身不需要更细的位置。
///
/// 剥出的每层 binder 追加进 `context`（作用域顺序：外层在前），供组合形态的
/// `funapply` 推断类型。
#[allow(clippy::too_many_arguments)]
fn peel_all_pi(
    ty: &Expr,
    hole: Span,
    answer: Option<&Expr>,
    outer: &[String],
    context: &mut Vec<GoalBinderSpec>,
    src: &str,
    span_start: usize,
    options: &CompileOptions,
) -> Result<Expr, CompileError> {
    let mut used: HashSet<String> = outer.iter().cloned().collect();
    let mut layers: Vec<(String, Option<Box<Expr>>, BinderKind)> = Vec::new();
    let mut cur = ty;
    loop {
        match cur {
            Expr::Forall { binders, body, .. } => {
                for binder in binders {
                    let base = if binder.name.is_empty() {
                        "x"
                    } else {
                        binder.name.as_str()
                    };
                    let name = fresh_name(base, &mut used);
                    layers.push((name, binder.ty.clone(), binder.style.clone()));
                }
                cur = body;
            }
            Expr::Arrow {
                domain, codomain, ..
            } => {
                let name = fresh_name("x", &mut used);
                layers.push((name, Some(domain.clone()), BinderKind::Explicit));
                cur = codomain;
            }
            _ => break,
        }
    }
    if layers.is_empty() {
        return Err(CompileError::elab(
            ErrorKind::ElabIntroNotAFunction,
            "值位 `funintro` 需要目标至少是一层函数（`A -> B` 或 `forall …`）",
            hole,
        ));
    }
    // 剥出的层进入作用域：组合形态里 `funapply` 的推断要用它们。
    context.extend(layers.iter().map(|(name, ty, _)| GoalBinderSpec {
        name: name.clone(),
        ty: ty.as_deref().map(render_expr),
    }));
    // 末端按答案形态分派（组合，I13-S3）：
    //   * `None` → 洞（练习）；
    //   * `funapply <term>` → 对**剥完 binder 后的最终目标** `cur` 做部分应用
    //     降低——这是「funintro 到内核那边就是 fun 链、funapply 到内核那边就
    //     是 `X <实参> sorry …`」的实现点；
    //   * 嵌套 `funintro` → 递归（最终目标不是函数时由内层报
    //     `elab-intro-not-a-function`）；
    //   * 其它表达式 → 原样（隐式替换，内核终审）。
    let mut body = match answer {
        None => Expr::Hole { span: hole },
        Some(Expr::Apply { term, span }) => {
            let (app, _) = lower_at(
                term.as_deref(),
                *span,
                cur,
                context,
                src,
                span_start,
                options,
            )?;
            app
        }
        Some(inner @ Expr::Intro { .. }) => {
            let mut inner_outer: Vec<String> = used.iter().cloned().collect();
            let Some((inner_expr, _)) = lower_inner(
                cur,
                inner,
                &mut inner_outer,
                context,
                src,
                span_start,
                options,
            )?
            else {
                // `answer` 已匹配 `Expr::Intro`，lower_inner 必然命中 Intro 分支。
                unreachable!("nested funintro always lowers")
            };
            inner_expr
        }
        Some(other) => other.clone(),
    };
    for (name, ty, style) in layers.into_iter().rev() {
        body = Expr::Lambda {
            binders: vec![Binder {
                name,
                ty,
                style,
                span: hole,
            }],
            body: Box::new(body),
            span: hole,
        };
    }
    Ok(body)
}

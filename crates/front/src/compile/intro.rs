//! 值位 `intro` 关键字的降低：把声明类型最外层的 Pi 望远镜全部剥成显式
//! lambda，末端一个洞（`fun (a : Prop) => … => sorry`）。
//!
//! 只做结构性展开，不做判定：降低后的值与普通的部分作答走同一条
//! `open_goal` 流水线，填洞结果仍由完整内核终审（REQUIREMENTS §2 第 8 条）。
//! 骨架文本（`render_expr` 产物）同时回传，作为编辑器补全项的单一事实源。

use super::error::{CompileError, ErrorKind};
use crate::proof::{fresh_name, render_expr};
use crate::{Binder, BinderKind, Expr, Span};
use std::collections::HashSet;

/// 值位恰为 `intro` 时返回 `(降低后的 lambda + 洞, 显式骨架文本)`；
/// 其它值原样透传（`None`）。目标没有可剥的 binder 时返回教学错误。
pub(crate) fn lower_intro_val(
    ty: &Expr,
    val: &Expr,
) -> Result<Option<(Expr, String)>, CompileError> {
    let Expr::Intro { span } = val else {
        return Ok(None);
    };
    let expr = peel_all_pi(ty, *span)?;
    let skeleton = render_expr(&expr);
    Ok(Some((expr, skeleton)))
}

/// 全剥 `ty` 最外层的 Forall binder / Arrow 域，包成一层层单 binder 的
/// `Expr::Lambda`；匿名层用生成器约定的基名 `x`（`x2` 防撞，与
/// `suggest::restart_skeleton` 同源）。每个合成节点的 span 都是 `intro`
/// token——洞需要它，binder 本身不需要更细的位置。
fn peel_all_pi(ty: &Expr, hole: Span) -> Result<Expr, CompileError> {
    let mut used: HashSet<String> = HashSet::new();
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
            "值位 `intro` 需要目标至少是一层函数（`A -> B` 或 `forall …`）",
            hole,
        ));
    }
    let mut body = Expr::Hole { span: hole };
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

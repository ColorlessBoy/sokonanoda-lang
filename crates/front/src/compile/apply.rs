//! 值位 `funapply` 关键字的降低：把目标「倒过来」消费——拿一个证明/函数接到
//! 目标上，把它的前提留成洞（`funapply h` → `h sorry`）。
//!
//! 与 `funintro` 的关键差别（`docs/design/term-apply.md` §1）：`funintro` 只需要
//! **声明类型**（parse 后已是 AST，纯 front 侧、值不进内核）；`funapply` 需要
//! **被应用名字的类型望远镜**，而 front 侧没有可用的类型表——局部假设只有
//! 渲染后的文本、`GoalTemplates` 丢了 codomain、内核无类型查询 API。因此这里
//! 走 `by` 引擎已验证的 [`judge_infer`] 路线：内核只用来**推断类型**，不做
//! 判定；判定仍在填洞后的合成声明上（`crate::judge`）。这是对 `intro`
//! 「值不进内核」的**刻意偏离**，理由见设计文档。
//!
//! telescope 机械（剥 Pi / spine 合一 / 替换）与 `by` 的 tactic `apply`
//! **共用** [`crate::spine`]——两条 `apply` 路径的合一强度不许分叉。

use super::error::{CompileError, ErrorKind};
use crate::ast::Expr;
use crate::compile::CompileOptions;
use crate::judge::{judge_infer, GoalBinderSpec, Judgement};
use crate::proof::{parse_expr_text, render_expr};
use crate::spine::{mentions, peel_pi, unify_spine};
use crate::{Binder, Span};

/// 值位恰为 `apply <term>`（或在声明 binder 降级出的 lambda 链尾）时，
/// 返回 `(降低后的值, 展开骨架文本)`。
///
/// 实参里**已给出**的前置实参按位置消耗（`apply f a` 只把剩下的前提留成洞），
/// 这就是「等价部分表达式」的形态。
pub(crate) fn lower_apply_val(
    ty: &Expr,
    val: &Expr,
    src: &str,
    span_start: usize,
    options: &CompileOptions,
) -> Result<Option<(Expr, String)>, CompileError> {
    // 沿 lambda 链（声明 binder 由 `wrap_decl_binders` 降级而来）收集上下文
    // 假设，同时把声明类型同步剥掉对应层，得到「实参项要面对的目标」。
    //
    // **链必须重建**：降低产物里仍要带着这些 lambda，否则局部假设会变成自由
    // 标识符（elab 报 unknown identifier），`open_goal` 的局部假设覆盖层也
    // 拿不到它们的类型。`intro` 的降低同样走「沿链下降 + 重建」
    // （`compile/intro.rs` 的 `Expr::Lambda` 分支）。
    let mut context: Vec<GoalBinderSpec> = Vec::new();
    let mut chain: Vec<Binder> = Vec::new();
    let mut goal_ty = ty.clone();
    let mut cur = val;
    loop {
        match cur {
            Expr::Lambda { binders, body, .. } => {
                for _ in binders {
                    let Some(pi) = peel_pi(&goal_ty) else {
                        // 类型层不够剥：不是 `apply` 该出现的形态，交回上层。
                        return Ok(None);
                    };
                    goal_ty = pi.body;
                }
                context.extend(binders.iter().map(|b| GoalBinderSpec {
                    name: b.name.clone(),
                    ty: b.ty.as_deref().map(render_expr),
                }));
                chain.extend(binders.iter().cloned());
                cur = body;
            }
            Expr::Apply { term, span } => {
                let (expr, skeleton) = lower_at(
                    term.as_deref(),
                    *span,
                    &goal_ty,
                    &context,
                    src,
                    span_start,
                    options,
                )?;
                if chain.is_empty() {
                    return Ok(Some((expr, skeleton)));
                }
                let wrapped = Expr::Lambda {
                    binders: chain,
                    body: Box::new(expr),
                    span: Span::new(val.span().start, span.end),
                };
                return Ok(Some((wrapped, skeleton)));
            }
            _ => return Ok(None),
        }
    }
}

/// 把 `apply <term>` 降低成 `term <已给实参> <前提洞…>`。
#[allow(clippy::too_many_arguments)]
fn lower_at(
    term: Option<&Expr>,
    span: Span,
    goal_ty: &Expr,
    context: &[GoalBinderSpec],
    src: &str,
    span_start: usize,
    options: &CompileOptions,
) -> Result<(Expr, String), CompileError> {
    let Some(term) = term else {
        return Err(CompileError::elab(
            ErrorKind::ElabApplyNeedsATerm,
            "`funapply` 后面要跟一个证明或函数，例如 `funapply h`…`funapply (f a)`".to_string(),
            span,
        ));
    };
    let term_text = render_expr(term);

    // 问内核要类型（推断，不是判定）。
    let prefix = src.get(..span_start).unwrap_or("");
    let f_ty = judge_infer(prefix, options, context, &term_text).map_err(|j| match j {
        // 名字打错是最常见的失败，保留既有的稳定错误码。
        Judgement::Error { code, message } if code == "elab-unknown-identifier" => {
            CompileError::elab(ErrorKind::ElabUnknownIdentifier, message, span)
        }
        Judgement::Error { message, .. } => {
            CompileError::elab(ErrorKind::ElabApplyNotApplicable, message, span)
        }
        Judgement::Mismatch { .. } => CompileError::elab(
            ErrorKind::ElabApplyNotApplicable,
            format!("无法推断 `{term_text}` 的类型"),
            span,
        ),
        Judgement::Match => unreachable!("judge_infer never returns Match"),
    })?;
    let f_ty_expr = parse_expr_text(&f_ty).map_err(|e| {
        CompileError::elab(
            ErrorKind::ElabApplyNotApplicable,
            format!("无法解析 `{term_text}` 的类型：{e}"),
            span,
        )
    })?;

    // 剥 Pi 链得 (前提, 结论)。
    let mut layers: Vec<(String, Expr)> = Vec::new();
    let mut cursor = f_ty_expr;
    let codomain = loop {
        let Some(pi) = peel_pi(&cursor) else {
            break cursor;
        };
        layers.push((pi.name, pi.domain));
        cursor = pi.body;
    };

    // 位置 spine 合一：结论与目标同头时，结论里出现的命名 binder 是类型参数
    // （由目标实参填充），其余前提留成洞。合一强度与 tactic `apply` 一致。
    let sigma = unify_spine(&codomain, goal_ty, &layers).ok_or_else(|| {
        CompileError::elab(
            ErrorKind::ElabApplyNotApplicable,
            format!("`{term_text}` 的结论与当前目标对不上"),
            span,
        )
    })?;

    // 实参项的每个 telescope 层：出现在结论里的命名 binder 是**类型参数**
    // （由目标实参经 σ 填充，学习者不用写），其余是**前提**，留成洞。
    //
    // 注意 `term` 的类型已经吃掉了学习者手写的实参（`funapply f p` 推出来的是
    // `Q p -> P p`），所以这里不需要、也不能再按「已给实参个数」去跳层——
    // 结论之后剩下的每一层前提都是洞。
    let mut app = term.clone();
    for (name, _domain) in &layers {
        let arg = if !name.is_empty() && mentions(name, &codomain) {
            sigma.get(name).cloned().unwrap_or_else(|| Expr::Ident {
                name: name.clone(),
                span,
            })
        } else {
            Expr::Hole { span }
        };
        app = Expr::App {
            fun: Box::new(app),
            arg: Box::new(arg),
            span,
        };
    }
    let skeleton = render_expr(&app);
    Ok((app, skeleton))
}

//! **隐式参数只许前导**（用户 2026-10-29 拍板：「显式参数后面跟隐式参数直接报错，不允许」）。
//!
//! 每条 Π 链（声明自己的签名、归纳参数 + 类型、构造子、高阶参数的类型）里，`{x : T}` 只能写在
//! **所有显式层之前**：`(a : A) {b : B}`、`A → {b : B} → …`、`(a : A) : ∀ {b : B}, …` 一律报
//! `elab-implicit-binder-after-explicit`，这条声明不 elaborate、不进环境。
//!
//! **为什么**：strict 的绑定（P1-F2，对照 Lean `elabAppArgs`）按内核 telescope 逐层分派，而注册表的
//! `implicit_prefix` / `explicit_arity` 只描述**前导**隐式 ⇒ 允许中间隐式时两边对同一声明给出不同的
//! 显式层数（P1-F1 扫描里的潜在分歧）。直接拒掉 ⇒ 这类分歧**按构造不存在**。
//! ⚠ 偏离 Lean 4：Lean 逐层看 binder 档（`Lean/Elab/App.lean:942-946`），隐式可以在任意位置。
//!
//! 纯源级走查、零内核调用。拆成独立模块：`walk.rs` 受 `docs-lint` ⑨ 冻结，那边只留一行调用。

use crate::compile::check::failed_state;
use crate::compile::error::{CompileError, ErrorKind};
use crate::compile::report::DeclKind;
use crate::{Binder, BinderKind, Command, Expr, Span};

use super::{CmdCtx, Walk};

/// 违例 ⇒ 报错 + 记一条 Failed 声明，返回 `true`（调用方跳过这条命令）。
///
/// 已判过的会话（`trusted`）第一次就报过了 ⇒ 只跳过、不重复报（与 `skip` 的口径一致）。
pub(super) fn reject(walk: &mut Walk<'_, '_>, c: &CmdCtx<'_>, command: &Command) -> bool {
    let Some((at, message)) = first_violation(command) else {
        return false;
    };
    if c.trusted {
        return true;
    }
    let (kind, name, span) = match command {
        Command::Def { name, span, .. } => (DeclKind::Definition, Some(name.clone()), *span),
        Command::Theorem { name, span, .. } => (DeclKind::Theorem, Some(name.clone()), *span),
        Command::Axiom { name, span, .. } => (DeclKind::Axiom, Some(name.clone()), *span),
        Command::Example { span, .. } => (DeclKind::Example, None, *span),
        Command::InductiveBlock { name, span, .. } => {
            (DeclKind::Inductive, Some(name.clone()), *span)
        }
        _ => return false,
    };
    let err = CompileError::elab(ErrorKind::ElabImplicitBinderAfterExplicit, message, at);
    walk.out.push_error(c.idx, err.clone());
    walk.decl_states
        .push(failed_state(kind, name, span, err, c.idx));
    true
}

/// 命令里**第一处**违例：`(那个隐式 binder 的 span, 报错文案)`；不是声明 / 没有违例 ⇒ `None`。
pub(super) fn first_violation(command: &Command) -> Option<(Span, String)> {
    match command {
        Command::Def { ty, .. }
        | Command::Theorem { ty, .. }
        | Command::Axiom { ty, .. }
        | Command::Example { ty, .. } => chain(ty, None),
        Command::InductiveBlock {
            params,
            ty,
            constructors,
            recursor,
            ..
        } => {
            // 归纳类型自己：参数 + 类型是同一条链。
            let mut prev = None;
            if let Some(v) = layers(params, &mut prev) {
                return Some(v);
            }
            if let Some(v) = chain(ty, prev) {
                return Some(v);
            }
            // 构造子：归纳参数在前面当隐式（`implicit_prefix`）⇒ 构造子自己的 binder 从前导段起算。
            for ctor in constructors {
                let mut prev = None;
                if let Some(v) = layers(&ctor.binders, &mut prev) {
                    return Some(v);
                }
                if let Some(v) = chain(&ctor.result, prev) {
                    return Some(v);
                }
            }
            recursor.as_ref().and_then(|rec| chain(&rec.ty, None))
        }
        _ => None,
    }
}

/// 一条 Π 链。`prev` = 这条链上**最近的显式层**的显示名（`None` = 还在前导隐式段里）。
fn chain(e: &Expr, mut prev: Option<String>) -> Option<(Span, String)> {
    match e {
        Expr::Forall { binders, body, .. } => {
            if let Some(v) = layers(binders, &mut prev) {
                return Some(v);
            }
            chain(body, prev)
        }
        // 匿名 `→` 的左边是一层显式参数；左边自己的 Π 是另一条链（高阶参数）。
        Expr::Arrow {
            domain, codomain, ..
        } => chain(domain, None).or_else(|| chain(codomain, Some("`→` 左边那一层".into()))),
        // 不在链上的位置：子项里的 Π 各自是一条新链（`∃`/`fun` 里的 `∀`、应用的实参）。
        Expr::App { fun, arg, .. } => chain(fun, None).or_else(|| chain(arg, None)),
        Expr::Lambda { binders, body, .. } => binders
            .iter()
            .find_map(|b| b.ty.as_deref().and_then(|t| chain(t, None)))
            .or_else(|| chain(body, None)),
        _ => None,
    }
}

/// 一组 binder：先判档（隐式且前面已有显式 ⇒ 违例），再查 binder 自己的类型（新链）。
fn layers(binders: &[Binder], prev: &mut Option<String>) -> Option<(Span, String)> {
    for b in binders {
        match (&b.style, prev.as_deref()) {
            (BinderKind::Implicit, Some(after)) => return Some((b.span, message(b, after))),
            (BinderKind::Implicit, None) => {}
            (BinderKind::Explicit, _) => *prev = Some(format!("`{}`", b.name)),
        }
        if let Some(v) = b.ty.as_deref().and_then(|t| chain(t, None)) {
            return Some(v);
        }
    }
    None
}

fn message(b: &Binder, after: &str) -> String {
    let ty =
        b.ty.as_deref()
            .map(crate::proof::render_expr)
            .unwrap_or_else(|| "_".into());
    format!(
        "隐式参数 `{{{} : {ty}}}` 写在了显式参数 {after} 之后 —— 隐式参数只能写在签名最前面（所有显式参数之前）",
        b.name
    )
}

#[cfg(test)]
#[path = "../../tests/binder_order.rs"]
mod tests;

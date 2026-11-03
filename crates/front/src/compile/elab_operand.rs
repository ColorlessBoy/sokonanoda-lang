//! 操作数类型的**源级**推导（从 `elab.rs` 搬出 —— 那个文件在行数上限，只许减）。

use crate::ast::Expr;
use crate::span::Span;

/// `fun (h : P) => h` 的**书写类型** `P -> P`，纯源级走查（**零内核调用**）。
///
/// 只认**一条**形状：所有 binder 都有注解，且体是某个 binder 的**裸名引用**
/// （`fun (h : P) => h` —— 课程证明项里压倒性的多数，`Iff.intro`/`And.intro`
/// 的实参全是它）。体的类型就是**那个 binder 的注解**，于是整条类型 =
/// `T₀ → T₁ → … → Tₙ₋₁ → T_idx` ✓ —— 不需要问内核，也就不会被 pp 的隐式实参
/// 丢失污染 ✓。
///
/// 其余形状（`fun (x : α) => f x` 之类）返回 `None` ⇒ 调用方照旧问内核，
/// **不比今天差** ✓。
pub(crate) fn lambda_source_type(operand: &Expr) -> Option<Expr> {
    let Expr::Lambda { binders, body, .. } = operand else {
        return None;
    };
    if binders.is_empty() {
        return None;
    }
    let mut tys: Vec<Expr> = Vec::with_capacity(binders.len());
    for binder in binders {
        tys.push(binder.ty.as_deref()?.clone());
    }
    let Expr::Ident { name, .. } = body.as_ref() else {
        return None;
    };
    let idx = binders.iter().position(|b| &b.name == name)?;
    let mut ty = tys[idx].clone();
    for domain in tys.iter().rev() {
        ty = Expr::Arrow {
            domain: Box::new(domain.clone()),
            codomain: Box::new(ty),
            span: Span::default(),
        };
    }
    Some(ty)
}

//! `proof.rs` 的**无渲染依赖**的小工具（从 `proof.rs` 拆出来，2026-10-16）。
//!
//! **为什么单独一个文件**：`proof.rs` 在 `scripts/docs-lint.py` 判据 ⑨ 的**存量冻结**
//! 清单里（**只许减不许增** ✗）。本轮 `Expr::NamedArg`（用户 2026-10-16 第 ② 条）要往
//! `render_expr` 的 match 里加一行 ⇒ 按 `prefix_recovery.rs` 的先例，把这两个**不调用
//! `render_expr`** 的工具搬出来（`scripts/audit-notation-paths.py` 把 `render_expr(`
//! 的调用点钉在 `proof.rs` 定义处 ⇒ 搬出去的片段**不能**碰它 ✗），`proof.rs` 只留
//! `pub use` 转发 ✓。
//!
//! ⚠ `render_binder_notation` / `render_pattern` **不能**搬（它们调 `render_expr`）✗ ——
//! 第一版把它们搬去 `expr.rs` ⇒ `audit-notation-paths` 当场判「新增 2 处绕过」✗（实测）。

use crate::ast::Expr;
use crate::span::Span;

/// binder 名防撞（外层优先保留原名）：生成骨架（失败声明重启、值位
/// `intro` 展开）时，同名或匿名层的默认名 `x` 撞上已有名字就追加序号
/// （`x` → `x2` → …）。命名约定只有一处，两个生成器共用。
pub(crate) fn fresh_name(base: &str, used: &mut std::collections::HashSet<String>) -> String {
    let mut name = base.to_string();
    let mut n = 2;
    while !used.insert(name.clone()) {
        name = format!("{base}{n}");
        n += 1;
    }
    name
}
/// 从 `ty` 最外层剥 `n` 层 Pi/Forall binder，返回剩余类型；层数不够返回
/// `None`。声明 binder 的 `intro` 递归降低与 `by` 引擎的初始上下文共用。
pub fn peel_pi_layers(ty: &Expr, n: usize) -> Option<Expr> {
    if n == 0 {
        return Some(ty.clone());
    }
    match ty {
        Expr::Forall { binders, body, .. } if !binders.is_empty() => {
            if binders.len() >= n {
                if binders.len() == n {
                    Some(body.as_ref().clone())
                } else {
                    // 同一 Forall 节点里还有剩的 binder：重包余下的（与
                    // `by::peel_pi` 的逐层剥法同构）。
                    Some(Expr::Forall {
                        binders: binders[n..].to_vec(),
                        body: body.clone(),
                        span: Span::default(),
                    })
                }
            } else {
                peel_pi_layers(body, n - binders.len())
            }
        }
        Expr::Arrow { codomain, .. } => peel_pi_layers(codomain, n - 1),
        _ => None,
    }
}

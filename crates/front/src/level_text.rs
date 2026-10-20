//! 类型文本的**层归一**（自 `judge.rs` 拆出，`AGENTS.md` 硬规则 5）。
//! 只装 `?u.<id>` → 层参数名的归一；剥层仍在 `judge::peel_binders`。

use crate::ast::Expr;
use crate::Span;
/// **1b**：把内核 pp 的**未解层元变量**（`?u.<id>`）归一成层参数名（`u_soko<id>`）。
///
/// 为什么需要：`peel_binders` 是**文本剥层**（第一步 `parse_expr_text`），遇到 `?` 直接
/// `break` ⇒ 整条上下文 telescope 会被当成被应用项的层（σ 按名字撞车）⇒ 子目标错位
/// （实测：`apply Eq.symm` 在带假设的上下文里给出 `x = x`）。层本身不需要在这里解：
/// 装配时按期望类型重新解。`?` 不是源语法（`token.rs` 给的是「??? 已移除」教学文案）
/// ⇒ 不回读就一定是硬错误。
///
/// ⚠ **守卫**：归一后若仍含 `?`（别的元变量形态）⇒ **原样返回**（逐字节等于改动前）。
/// 自由层名 `u_soko*` **不许**外泄进子目标类型 —— 见 `by.rs::apply_type_fits` 的同名守卫。
pub(crate) fn normalize_level_mvars(text: &str) -> std::borrow::Cow<'_, str> {
    // **回退开关**：`SOKO_APPLY_LEVEL_TEXT=0` ⇒ 逐字节回到改动前（1b 单独可关）。
    static OFF: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    if *OFF.get_or_init(|| std::env::var("SOKO_APPLY_LEVEL_TEXT").ok().as_deref() == Some("0")) {
        return std::borrow::Cow::Borrowed(text);
    }
    if !text.contains("?u.") {
        return std::borrow::Cow::Borrowed(text);
    }
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len() + 8);
    let mut i = 0;
    while i < bytes.len() {
        if text[i..].starts_with("?u.") {
            let mut j = i + 3;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            if j > i + 3 {
                out.push_str("u_soko");
                out.push_str(&text[i + 3..j]);
                i = j;
                continue;
            }
        }
        let ch = text[i..].chars().next().expect("非空");
        out.push(ch);
        i += ch.len_utf8();
    }
    if out.contains('?') {
        return std::borrow::Cow::Borrowed(text);
    }
    std::borrow::Cow::Owned(out)
}

/// 剥掉 `expr` 的第一个 binder（多 binder Forall 去掉首个、单 binder 去 body、
/// Arrow 去 codomain），返回余下结构。
pub(crate) fn peel_one_binder(expr: &Expr) -> Option<Expr> {
    match expr {
        Expr::Forall { binders, body, .. } if !binders.is_empty() => {
            if binders.len() > 1 {
                Some(Expr::Forall {
                    binders: binders[1..].to_vec(),
                    body: body.clone(),
                    span: Span::default(),
                })
            } else {
                Some(body.as_ref().clone())
            }
        }
        Expr::Arrow { codomain, .. } => Some(codomain.as_ref().clone()),
        _ => None,
    }
}

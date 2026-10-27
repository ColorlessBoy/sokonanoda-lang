//! 折叠加工用的 span 小工具（自 `display.rs` 拆出，2026-10-27）。
//!
//! 拆出理由：`display.rs` 已在软线之上（`docs-lint` ⑨：既有超标文件只许减不许增），
//! 而 `@` 修复需要一段 span 逻辑；把这两条纯函数搬出来，`display.rs` 净减。

use crate::ast::Expr;
use crate::span::{Pos, Span};

/// 把折叠范围向左扩到 `@`（若紧邻）。
///
/// parser 的 `@` 只置位 `explicit_spine`，**不进表达式 span**（`parse_app` 先
/// `bump` 再解析头）⇒ 折叠从头上开始替换时，`@` 会留在文本里（`@Set.mem α a A`
/// 折成 `@a ∈ A`）。记法写法是源级形态、不需要 `@`，所以这里把它（以及它与头
/// 之间可能的空白）一起覆盖掉。
pub(super) fn fold_span_including_explicit_marker(span: Span, src: &str, base: usize) -> Span {
    let Some(rel) = span.start.offset.checked_sub(base) else {
        return span;
    };
    if rel > src.len() {
        return span;
    }
    let bytes = src.as_bytes();
    let mut at = rel;
    while at > 0 && bytes[at - 1].is_ascii_whitespace() {
        at -= 1;
    }
    if at == 0 || bytes[at - 1] != b'@' {
        return span;
    }
    let mut widened = span;
    widened.start.offset = base + at - 1;
    widened
}

/// 两个子节点之间的 `->` 记号（A1）。找不到（源里写的是 `→`，或者这一段根本
/// 不是箭头——`∀ x, p` 的 `,`）⇒ `None`，**不猜、不动**。
///
/// `Expr::Arrow` 的 span 覆盖 `domain .. codomain`，`Expr::Forall`（`(x : α) -> β`
/// 那条匿名 binder 形状）的 span 从 binder 组起、body 结束——两处的 `->` 都落在
/// 「上一个子节点结束 .. 下一个子节点开始」这段空隙里。
pub(super) fn arrow_token_between(src: &str, base: usize, from: usize, to: usize) -> Option<Span> {
    let start = from.checked_sub(base)?;
    let end = to.checked_sub(base)?;
    if start > end || end > src.len() {
        return None;
    }
    let at = src.get(start..end)?.find("->")?;
    let offset = from + at;
    Some(Span::new(
        Pos {
            offset,
            ..crate::span::Pos::default()
        },
        Pos {
            offset: offset + 2,
            ..crate::span::Pos::default()
        },
    ))
}

/// **显式脊只收 `full` 那一形**：`@Set.mem α a` 与闭项完全应用 `Set.mem a A`
/// 的实参个数同形（都是 `full - prefix`），但前者还缺一个操作数；折它会把类型
/// 参数当第一个操作数（`α ∈ a` ✗）。`display.rs` 的 `fold_spine` 用它挡掉
/// `explicit_spine && k != full`。
pub(super) fn explicit_spine_blocks_fold(expr: &Expr, k: usize, full: usize) -> bool {
    matches!(
        expr,
        Expr::App {
            explicit_spine: true,
            ..
        }
    ) && k != full
}

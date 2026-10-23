//! parser 的小工具：`·` 子弹解析 + “最后一个已消费 token 的 end”（自 `parser.rs` 拆出，
//! `parser.rs` 在行数棘轮红线上）。

use super::{Parser, Span, Tactic};
use crate::diagnostic::Result;
use crate::span::Pos;

/// 解析 `· <缩进 tactic 块>`。
///
/// **第一颗 tactic 也必须比 `·` 更深**：`parse_nested_tactic_sequence` 的第一条是
/// 无条件解析的（它没有“起点列”参数）⇒ 空 `·` 后面同列的下一颗 `·` 会被吞成子
/// tactic（实测：空子弹报成下一条 `exact` 的目标错）。同列/更浅 ⇒ 这一块是空的
/// （引擎报“没证完”）。
pub(super) fn parse_bullet(p: &mut Parser) -> Result<Tactic> {
    let tok = p.bump();
    let first_deeper =
        p.tactic_keyword_ahead() && p.peek().span.start.column > tok.span.start.column;
    let tactics = if first_deeper {
        p.parse_nested_tactic_sequence(tok.span.start.column)?
    } else {
        Vec::new()
    };
    let end = tactics.last().map(|t| t.span().end).unwrap_or(tok.span.end);
    Ok(Tactic::Bullet {
        tactics,
        span: Span::new(tok.span.start, end),
    })
}

/// **命令/声明的 span 终点 = 最后一个已经消费的 token 的 end**。
///
/// 为什么不能用 `val.span().end`：`parse_atom` 的 `(e)` 返回内部表达式（span 停在
/// 右括号**之前**）⇒ `apply (…)` 的最后 `))` 落在声明 span 之外 ⇒
/// `query::decl_at_cursor` 的空白尾巴看到 `)` 判 `outside-declarations`
/// （unit01 用户实测：光标在最后空行，面板说“不在任何声明内”）。
///
/// `cursor` 指向**下一个未消费**的 token（`peek` 不消费）⇒ `cursor - 1` 就是值位的
/// 最后一个 token（包括右括号）。注释已被词法器跳过，不会把 trailing comment 吃进来。
pub(super) fn last_consumed_end(p: &Parser, fallback: Pos) -> Pos {
    p.tokens
        .get(p.cursor.saturating_sub(1))
        .map(|t| t.span.end)
        .unwrap_or(fallback)
}

/// **S3（2026-10-23）**：内部回读通道的解析入口 —— 词法额外认 `?m<digits>`
/// （用户通道不变：`?` 仍报「??? 已移除」）。
///
/// 只在 judge 的合成文档 / 元变量回灌用；判定语义不变（认出来的也只是 `Ident`，
/// 由上层当元变量引用处理）。
pub(crate) fn parse_with_inherited_meta(
    src: &str,
    inherited: &[crate::ast::NotationDecl],
) -> Result<crate::ast::FolFile> {
    let mut symbols = super::scan_notation_symbols(src);
    for symbol in super::lexer_builtin_symbols() {
        if !symbols.contains(&symbol) {
            symbols.push(symbol);
        }
    }
    for decl in inherited {
        if !symbols.contains(&decl.symbol) {
            symbols.push(decl.symbol.clone());
        }
    }
    let tokens = crate::token::tokenize_meta_aware(src, &symbols)?;
    super::Parser::with_inherited(tokens, inherited).parse_file(src)
}

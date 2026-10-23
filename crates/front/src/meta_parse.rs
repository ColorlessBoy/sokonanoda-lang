//! **S3**：内部回读通道的表达式解析（`?m<digits>` 也认；用户通道不变）。
#![allow(dead_code)] // S3/S5 接线前只有单测与 judge 用
use crate::ast::{Command, Expr};
use crate::proof::{ProofError, CHECK_PREFIX};

/// **S3（2026-10-23）**：内部回读，`?m<digits>` 也认（**用户通道不变**）。
pub fn parse_expr_text_meta_with(
    text: &str,
    inherited: &[crate::ast::NotationDecl],
) -> Result<Expr, ProofError> {
    let file =
        crate::parser::util::parse_with_inherited_meta(&format!("{CHECK_PREFIX}{text}"), inherited)
            .map_err(|e| ProofError::Parse(e.message))?;
    match file.commands.into_iter().next() {
        Some(Command::Check { expr, .. }) => Ok(expr),
        _ => Err(ProofError::Parse("could not parse expression".to_string())),
    }
}

pub fn parse_expr_text_meta(text: &str) -> Result<Expr, ProofError> {
    let file =
        crate::parser::util::parse_with_inherited_meta(&format!("{CHECK_PREFIX}{text}"), &[])
            .map_err(|e| ProofError::Parse(e.message))?;
    match file.commands.into_iter().next() {
        Some(Command::Check { expr, .. }) => Ok(expr),
        _ => Err(ProofError::Parse("could not parse expression".to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proof::parse_expr_text;
    /// **S3**：`?m<digits>` 只在**内部通道**是 `Ident`；用户通道照旧拒绝。
    #[test]
    fn meta_ref_parses_only_on_the_internal_channel() {
        let e = parse_expr_text_meta("?m12").expect("内部通道应认 ?m12");
        match &e {
            Expr::Ident { name, .. } => assert_eq!(name, "?m12"),
            other => panic!("应是 Ident，实际 {other:?}"),
        }
        assert!(parse_expr_text("?m12").is_err(), "用户通道不许认 ?m12");
    }
}

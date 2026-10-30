//! 记法目标的宇宙层级文本（拆自 `elab.rs`，docs-lint ⑨：原样搬移，行为不变）。

use super::elab::{universe_level_text_of_operands, ElabCtx, ElabScope, InplaceEnv, KnownTable};
use crate::Expr;
use sokonanoda::builder::EnvBuilder;

/// 记法目标的**宇宙层级文本**（`Eq.{u}` 那一档）：目标只带 1 个宇宙参数时由
/// **操作数**解（`a : Set α` ⇒ `Set α : Type 0` ⇒ `u = 1`），其余位默认 `"0"`；
/// 目标没有宇宙参数 ⇒ 空表（调用方据此走"零层级"那条）。
///
/// 两条路（k==0 的老路与隐式档）共用这一份；解不出时**保持既有行为**（全 0，
/// 与不写 `.{u}` 的点名写法同判），见调用点那段注释。
pub(super) fn notation_level_texts<'a>(
    builder: &mut EnvBuilder<'a>,
    uparams: &[String],
    operands: &[&Expr],
    ctx: &ElabCtx<'a, '_>,
    scope: &ElabScope<'a>,
    known: &KnownTable,
) -> Vec<String> {
    if uparams.is_empty() {
        return Vec::new();
    }
    let mut texts: Vec<String> = uparams.iter().map(|_| "0".to_string()).collect();
    if uparams.len() == 1 {
        if let Some(text) = universe_level_text_of_operands(
            operands,
            ctx,
            scope,
            Some(&mut InplaceEnv {
                builder: &mut *builder,
                known,
            }),
        ) {
            texts[0] = text;
        }
    }
    texts
}

//! **r22（2026-10-13）**：`by` 块里**失败那条之后**其它独立失败的诊断探针
//!（自 `by.rs` 拆出：那个文件在行数棘轮红线上，`AGENTS.md` 硬规则 5 —— 同
//! `apply_probe.rs` 的由来；`docs-lint.py` ⑨）。
//!
//! Lean 语义：一条 tactic 失败 ⇒ 它被**回滚**，块**继续**在剩下的目标上跑后面的
//! tactic ⇒ 每条失败各报一条（`by exact hh; exact hh` 报**两条**）。真判定一个字
//! 都不动：在克隆出来的引擎状态上重放前缀、再逐条试，产物**只**当额外诊断。
use crate::ast::{Binder, Expr, Tactic};
use crate::by::{open_node, run_tactics, ByStep, GoalNode};
use crate::compile::elab::{DefTable, InductiveTable};
use crate::compile::{CompileError, CompileOptions};
use crate::span::Span;

/// **r22（2026-10-13）**：失败那条之后**其它独立失败**也收出来
///（Lean 语义：失败的 tactic 回滚、块继续 ⇒ 每条失败各一条诊断）。
/// 真判定不动：在克隆状态上重放前缀再逐条试，产物**只**当额外诊断。
#[allow(clippy::too_many_arguments)]
pub(crate) fn collect_remaining_tactic_errors<'a>(
    tactics: &[Tactic],
    failing: usize,
    root_ty: &Expr,
    initial_binders: &[Binder],
    universe: &[String],
    prefix_src: &str,
    options: &CompileOptions,
    inductives: &InductiveTable<'_>,
    defs: &DefTable,
    ctx: &crate::compile::elab::ElabCtx<'a, '_>,
) -> Vec<CompileError> {
    if failing >= tactics.len() {
        return Vec::new();
    }
    let mut nodes: Vec<GoalNode> = vec![open_node(root_ty.clone(), initial_binders.to_vec())];
    let mut worklist: Vec<usize> = vec![0];
    let mut steps: Vec<ByStep> = Vec::new();
    let mut holes: Vec<Span> = Vec::new();
    let mut redundant = Vec::new();
    if run_tactics(
        &tactics[..failing],
        &mut nodes,
        &mut worklist,
        &mut steps,
        None,
        universe,
        prefix_src,
        options,
        inductives,
        defs,
        ctx,
        None,
        &mut holes,
        &mut redundant,
    )
    .is_err()
    {
        // 前缀重放不成功（理论上不该发生）⇒ 不猜，交回空 ✓。
        return Vec::new();
    }
    let mut errors: Vec<CompileError> = Vec::new();
    for index in failing + 1..tactics.len() {
        let mut probe_nodes = nodes.clone();
        let mut probe_worklist = worklist.clone();
        let mut probe_steps: Vec<ByStep> = Vec::new();
        let mut probe_holes: Vec<Span> = Vec::new();
        let mut probe_redundant = Vec::new();
        match run_tactics(
            &tactics[index..index + 1],
            &mut probe_nodes,
            &mut probe_worklist,
            &mut probe_steps,
            None,
            universe,
            prefix_src,
            options,
            inductives,
            defs,
            ctx,
            None,
            &mut probe_holes,
            &mut probe_redundant,
        ) {
            Ok(()) => {
                nodes = probe_nodes;
                worklist = probe_worklist;
            }
            Err(error) => errors.push(error),
        }
    }
    errors
}

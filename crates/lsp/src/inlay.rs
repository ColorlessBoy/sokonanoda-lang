//! Inlay hints: the expected type of every open-exercise hole, rendered
//! right after the `sorry` (docs/design/rename-inlay.md §4).
//!
//! Read-only information only — no `textEdits` on hints (rust-analyzer
//! lesson: interactive inlays are expensive and rarely wanted).
//!
//! **`#check` 的结果不再是 inlay**（2026-10-09 用户实测 ①）：它以前在表达式
//! 后面常显 `: Type 0`（`document_hints` 里的第二个循环）。用户报「**inline
//! 提示已不需要**，#check 尾部仍带且看不全、无意义」——命令输出在 Infoview 的
//! 「命令输出」块里**已经完整可见**（含记法与高亮，见 `front::query` 的
//! `messages_at`），行内那一截只会被行宽截断、还和编辑器自己的类型提示抢位置。
//! ⇒ 这里**只**留洞的期望类型（学习者在 `sorry` 上真正需要的那一条）。

use sokonanoda_front::compile::{DeclState, DeclStatus, DocumentReport};
use sokonanoda_front::Span;
use tower_lsp::lsp_types::*;

/// All editor inlay hints for a document: **only** hole hints (expected types).
///
/// `#check`/`#print` 的输出**不走这条路**（见模块头：用户 2026-10-09 拍板去掉
/// 行内提示）——它们在 Infoview 的「命令输出」块里，`report.checks` 仍然是
/// 真相（`soko/stateAt.messages` 读它），只是不再变成 inlay。
pub(crate) fn document_hints(text: &str, report: &DocumentReport) -> Vec<InlayHint> {
    hole_hints(text, report)
}

/// One hint per hole: sub-hole types come from the server-side walk
/// (`sub_goals`, aligned **by position** with `holes`); a lone main hole
/// shows the remaining goal. Hints without a known type are skipped
/// (labels are never empty).
pub(crate) fn hole_hints(text: &str, report: &DocumentReport) -> Vec<InlayHint> {
    let _ = text;
    let mut hints = Vec::new();
    for d in &report.decls {
        if d.status != DeclStatus::Open {
            continue;
        }
        for (index, hole) in d.holes.iter().enumerate() {
            let Some(label) = hint_label(d, index, hole) else {
                continue;
            };
            hints.push(InlayHint {
                position: end_position(*hole),
                label,
                kind: Some(InlayHintKind::TYPE),
                text_edits: None,
                tooltip: Some(goal_tooltip(d)),
                padding_left: Some(true),
                padding_right: None,
                data: None,
            });
        }
    }
    hints
}

/// The hint label for one hole: `": <expected type>"`, or `None` when the
/// walk could not produce a type for it.
///
/// 洞的类型**按位置顺序**对齐 `sub_goals`，不能用 span 反查：`by apply imp`
/// 留下的多个未解子目标在源码里只有同一个位置（`by.rs` 的 `assemble` 给每个
/// 叶子洞传的是同一个 `hole_span`），按 span 反查会让每一处都命中**第一个**
/// 子目标——第二个子目标的期望类型永远显示不出来。
/// 回归：`by_apply_sub_goals_keep_their_own_expected_types`。
fn hint_label(d: &DeclState, index: usize, hole: &Span) -> Option<InlayHintLabel> {
    let ty = if d.sub_goals.len() == d.holes.len() {
        // spine 走查逐洞 push（`goals.rs:494-506` / `555-565`），一一对应。
        d.sub_goals.get(index).and_then(|s| s.ty.clone())
    } else if let Some(sub) = d.sub_goals.iter().find(|s| s.span == *hole) {
        sub.ty.clone()
    } else if d.holes.len() == 1 {
        d.goal.clone()
    } else {
        None
    }?;
    Some(InlayHintLabel::String(format!(": {ty}")))
}

/// The hole's end as a 0-based LSP position (front spans are 1-based).
fn end_position(span: Span) -> Position {
    Position {
        line: span.end.line.saturating_sub(1) as u32,
        character: span.end.column.saturating_sub(1) as u32,
    }
}

/// Goal-view tooltip for one hole: remaining goal plus the hypotheses the
/// lambda binders introduced so far (same data as the hover's Open branch).
fn goal_tooltip(d: &DeclState) -> InlayHintTooltip {
    let mut value = String::new();
    if let Some(goal) = &d.goal {
        value.push_str(&format!("剩余目标：`{goal}`\n"));
    }
    if !d.binders.is_empty() {
        value.push_str("\n已引入假设：\n");
        for b in &d.binders {
            value.push_str(&format!("- `{}` : `{}`\n", b.name, b.ty));
        }
    }
    InlayHintTooltip::MarkupContent(MarkupContent {
        kind: MarkupKind::Markdown,
        value,
    })
}

#[cfg(test)]
mod tests {
    use super::super::testutil::{
        call, did_open, handshake, lsp_pos, offset_of, position_json, test_service,
        wait_diagnostics, URI,
    };
    use serde_json::json;
    use tower_lsp::jsonrpc::Request as RpcRequest;
    use tower_lsp::lsp_types::*;

    const EXERCISE: &str = "example : Prop -> Prop := sorry\n";
    const CHECKED: &str = "def id : Prop -> Prop := fun (x : Prop) => x\n";
    const ALL_CHECKED: &str = "def id : Prop -> Prop := fun (x : Prop) => x\ndef two : Nat := 2\n";
    const PARTIAL: &str =
        "example : (a : Prop) -> a -> a := fun (a : Prop) => fun (h : a) => sorry\n";
    const AND_MULTI_HOLE: &str = "axiom And : Prop -> Prop -> Prop\n\
axiom And.intro : (a : Prop) -> (b : Prop) -> a -> b -> And a b\n\
theorem and_intro_rule : (a : Prop) -> (b : Prop) -> a -> b -> And a b := \
fun (a : Prop) => fun (b : Prop) => fun (ha : a) => fun (hb : b) => And.intro sorry sorry\n";

    /// `by apply imp` 会留下**两个**未解子目标（imp 的两个前提），而它们
    /// 在源码里只有**同一个**位置——`apply` 那一刻。`assemble` 给树里每个
    /// 叶子洞传的都是同一个 `hole_span`（`by.rs:372-397` + `269-271`），
    /// 所以 `holes = [s, s]`、`sub_goals[i].span` 也全是 `s`。
    ///
    /// 这直接暴露了 inlay 的旧实现对洞类型的**反查方式**：用
    /// `sub_goals.iter().find(span == hole)`——两个洞都会命中**第一个**
    /// 子目标，于是两处提示都是 `: p`，第二处的 `: q` 永远不显示。
    /// 见 `docs/design/real-input-tests.md` §4.1。
    const BY_APPLY_TWO_SUBGOALS: &str = "axiom And : Prop -> Prop -> Prop\n\
axiom And.intro : (a : Prop) -> (b : Prop) -> a -> b -> And a b\n\
axiom p : Prop\n\
axiom q : Prop\n\
axiom imp : p -> q -> And p q\n\
theorem t : And p q := by apply imp\n";

    async fn ask_inlay(
        service: &mut tower_lsp::LspService<crate::Backend>,
        src: &str,
    ) -> Option<Vec<InlayHint>> {
        let result = call(
            service,
            RpcRequest::build("textDocument/inlayHint")
                .params(json!({
                    "textDocument": {"uri": URI},
                    "range": {
                        "start": {"line": 0, "character": 0},
                        "end": position_json(lsp_pos(src, src.len())),
                    },
                }))
                .id(90)
                .finish(),
        )
        .await
        .expect("textDocument/inlayHint must answer");
        serde_json::from_value(result).expect("valid inlay hint response")
    }

    fn label_of(hint: &InlayHint) -> &str {
        match &hint.label {
            InlayHintLabel::String(label) => label,
            other => panic!("expected a string label, got {other:?}"),
        }
    }

    fn tooltip_of(hint: &InlayHint) -> &str {
        match &hint.tooltip {
            Some(InlayHintTooltip::MarkupContent(markup)) => &markup.value,
            other => panic!("expected a markup tooltip, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn single_hole_hint_shows_goal_type_right_after_the_hole() {
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, EXERCISE).await;
        let _ = wait_diagnostics(&mut socket, "inlay diagnostics").await;

        let hints = ask_inlay(&mut service, EXERCISE)
            .await
            .expect("hints array");
        assert_eq!(hints.len(), 1, "one hint for the lone hole: {hints:?}");
        let hint = &hints[0];
        assert_eq!(label_of(hint), ": Prop -> Prop");
        assert_eq!(hint.kind, Some(InlayHintKind::TYPE));
        assert_eq!(hint.padding_left, Some(true));
        let after_hole = lsp_pos(EXERCISE, offset_of(EXERCISE, "sorry") + "sorry".len());
        assert_eq!(hint.position, after_hole, "hint sits right after `sorry`");
        assert!(
            tooltip_of(hint).contains("剩余目标：`Prop -> Prop`"),
            "tooltip carries the remaining goal: {}",
            tooltip_of(hint)
        );
    }

    #[tokio::test]
    async fn spine_hints_carry_sub_goal_types_in_offset_order() {
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, AND_MULTI_HOLE).await;
        let _ = wait_diagnostics(&mut socket, "spine diagnostics").await;

        let hints = ask_inlay(&mut service, AND_MULTI_HOLE)
            .await
            .expect("hints array");
        assert_eq!(hints.len(), 2, "two spine holes: {hints:?}");
        assert_eq!(label_of(&hints[0]), ": a", "first sub-hole type: {hints:?}");
        assert_eq!(
            label_of(&hints[1]),
            ": b",
            "second sub-hole type: {hints:?}"
        );
        let first = offset_of(AND_MULTI_HOLE, "sorry");
        let second = AND_MULTI_HOLE[first + "sorry".len()..]
            .find("sorry")
            .expect("second hole exists")
            + first
            + "sorry".len();
        assert_eq!(
            hints[0].position,
            lsp_pos(AND_MULTI_HOLE, first + "sorry".len())
        );
        assert_eq!(
            hints[1].position,
            lsp_pos(AND_MULTI_HOLE, second + "sorry".len())
        );
    }

    #[tokio::test]
    async fn function_argument_hole_hint_shows_instantiated_binder_type() {
        // 用户原始需求（playground.sokonanoda:233）：`Eq.subst` 的谓词实参
        // 写成 sorry 后，inlay 显示实例化后的 `Nat -> Prop`。
        let src = concat!(
            "theorem eq_symm_nat : (a : Nat) -> (b : Nat) -> Eq.{1} Nat a b -> Eq.{1} Nat b a :=\n",
            "  fun (a : Nat) (b : Nat) (h : Eq.{1} Nat a b) =>\n",
            "    Eq.subst.{1} Nat (sorry) a b h (Eq.refl.{1} Nat a)\n",
        );
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, src).await;
        let _ = wait_diagnostics(&mut socket, "function-hole diagnostics").await;

        let hints = ask_inlay(&mut service, src).await.expect("hints array");
        assert_eq!(hints.len(), 1, "one hint for the lone hole: {hints:?}");
        assert_eq!(label_of(&hints[0]), ": Nat -> Prop");
        let after_hole = lsp_pos(src, offset_of(src, "sorry") + "sorry".len());
        assert_eq!(
            hints[0].position, after_hole,
            "hint sits right after `sorry`"
        );
    }

    #[tokio::test]
    async fn probed_sub_goal_hint_shows_the_kernel_expected_type() {
        // 前置洞穿透（design spine-meta-a.md）：B′ 对「前置实参是洞」的
        // 子洞给 None；请求期 kernel 探针补齐 → inlay 显示 `: Not _h0`。
        let src = "axiom False : Prop\n\
                   def Not : Prop -> Prop := fun (a : Prop) => a -> False\n\
                   axiom h : (a : Prop) -> Not a -> a\n\
                   theorem t : (a : Prop) -> Not a -> a :=\n\
                     fun (a : Prop) => fun (na : Not a) => h (sorry) (sorry)\n";
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, src).await;
        let _ = wait_diagnostics(&mut socket, "probe inlay diagnostics").await;

        let hints = ask_inlay(&mut service, src).await.expect("hints array");
        let labels: Vec<&str> = hints.iter().map(label_of).collect();
        assert_eq!(
            labels,
            vec![": Prop", ": Not _h0"],
            "the kernel probe's type must reach the inlay: {hints:?}"
        );
    }

    #[tokio::test]
    async fn nested_hole_hint_shows_the_inner_expected_type() {
        // 一层嵌套洞 `h (g sorry)`：洞 span 是内层 sorry，类型来自探针。
        let src = "axiom g : (a : Prop) -> Prop\n\
                   axiom h : (b : Prop) -> Prop\n\
                   axiom P : Prop\n\
                   theorem t : P := h (g sorry)\n";
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, src).await;
        let _ = wait_diagnostics(&mut socket, "nested inlay diagnostics").await;

        let hints = ask_inlay(&mut service, src).await.expect("hints array");
        assert_eq!(hints.len(), 1, "one nested hole: {hints:?}");
        assert_eq!(label_of(&hints[0]), ": Prop");
    }

    #[tokio::test]
    async fn check_results_are_not_inlay_hints() {
        // **2026-10-09 用户实测 ①**：`#check` 的行内提示（表达式后面那截 `: Type 0`）
        // **已不需要** —— 「`#check` 尾部仍带且看不全、无意义」✗。命令输出在
        // Infoview 的「命令输出」块里完整可见（记法 + 高亮），inlay 这一截只会被
        // 行宽截断。
        //
        // ⚠ **正对照必须有**（否则"没有提示"可能是因为**整条 inlay 路坏了** ✗）：
        // 同一份文本里再放一个 `sorry` 洞 ⇒ 洞的期望类型提示照旧在，`#check`
        // 的**一个都不许有**。
        let src = "#check Nat\n#check (Nat -> Nat)\nexample : Prop -> Prop := sorry\n";
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, src).await;
        let params = wait_diagnostics(&mut socket, "check diagnostics").await;
        assert_eq!(
            params
                .diagnostics
                .iter()
                .map(|d| format!("{:?}", d.code.clone()))
                .collect::<Vec<_>>(),
            vec![r#"Some(String("sorry"))"#.to_string()],
            "`#check` 不产生诊断；唯一的诊断是正对照那个 `sorry` 洞的提醒"
        );
        let hints = ask_inlay(&mut service, src).await.expect("hints array");
        assert_eq!(
            hints.len(),
            1,
            "只有洞那条提示；`#check` 一条都不许有（修前这里会是 3 条）: {hints:?}"
        );
        assert_eq!(
            label_of(&hints[0]),
            ": Prop -> Prop",
            "剩下的是洞的期望类型"
        );
        let after_hole = lsp_pos(src, offset_of(src, "sorry") + "sorry".len());
        assert_eq!(
            hints[0].position, after_hole,
            "洞的提示仍紧跟在 `sorry` 之后（正对照）"
        );
        // 反向取证：`#check` 的两个表达式后面**一个提示都没有**。
        for needle in ["Nat\n", "(Nat -> Nat)\n"] {
            let after = lsp_pos(src, offset_of(src, needle) + needle.len() - 1);
            assert!(
                !hints.iter().any(|h| h.position == after),
                "`{needle}` 之后不许有 inlay 提示: {hints:?}"
            );
        }
    }

    #[tokio::test]
    async fn checked_declaration_produces_no_hints() {
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, CHECKED).await;
        let _ = wait_diagnostics(&mut socket, "checked diagnostics").await;

        let hints = ask_inlay(&mut service, CHECKED).await.expect("hints array");
        assert!(
            hints.is_empty(),
            "checked declarations must not hint: {hints:?}"
        );
    }

    #[tokio::test]
    async fn partial_answer_hint_lists_goal_and_hypotheses() {
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, PARTIAL).await;
        let _ = wait_diagnostics(&mut socket, "partial diagnostics").await;

        let hints = ask_inlay(&mut service, PARTIAL).await.expect("hints array");
        assert_eq!(hints.len(), 1, "one hint for the lone hole: {hints:?}");
        let hint = &hints[0];
        assert_eq!(label_of(hint), ": a", "remaining goal as the type");
        let tooltip = tooltip_of(hint);
        assert!(tooltip.contains("剩余目标：`a`"), "tooltip: {tooltip}");
        assert!(tooltip.contains("假设"), "tooltip: {tooltip}");
        assert!(tooltip.contains("`h` : `a`"), "tooltip: {tooltip}");
    }

    #[tokio::test]
    async fn by_apply_sub_goals_keep_their_own_expected_types() {
        // 两个未解子目标共用同一个位置时，提示必须**按洞的位置顺序**分别取
        // 自己的期望类型，不能用「span 反查 sub_goals」——那会让两处都显示
        // 第一个子目标的类型（本用例正是为此而红过）。
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, BY_APPLY_TWO_SUBGOALS).await;
        let _ = wait_diagnostics(&mut socket, "by apply inlay diagnostics").await;

        let hints = ask_inlay(&mut service, BY_APPLY_TWO_SUBGOALS)
            .await
            .expect("hints array");
        let labels: Vec<&str> = hints.iter().map(label_of).collect();
        assert_eq!(
            labels,
            vec![": p", ": q"],
            "each sub-goal must show its own premise: {hints:?}"
        );
        // 两个洞在源码里确实同址（`apply` 那一刻），这是 by 引擎的已知限制：
        // soko/nextHole 无法在同址的多个子目标之间导航，程序化消费应以
        // soko/goals 的 hole id 为身份。见 docs/design/real-input-tests.md §4.1。
        assert_eq!(
            hints[0].position, hints[1].position,
            "the two sub-goals share one source position"
        );
    }

    #[tokio::test]
    async fn decl_binder_hole_shows_the_codomain_goal() {
        // 声明级 binder：`:= sorry` 的剩余目标直接是 codomain，上下文是
        // 声明 binder（inlay 不必经过 lambda 前缀）。
        let src = "axiom And : Prop -> Prop -> Prop\n\
                   theorem and_swap2 (a : Prop) (b : Prop) (h : And a b) : And b a := sorry\n";
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, src).await;
        let _ = wait_diagnostics(&mut socket, "decl-binder inlay diagnostics").await;

        let hints = ask_inlay(&mut service, src).await.expect("hints array");
        assert_eq!(hints.len(), 1, "one hint for the lone hole: {hints:?}");
        assert_eq!(label_of(&hints[0]), ": And b a");
        let tooltip = tooltip_of(&hints[0]);
        assert!(
            tooltip.contains("剩余目标：`And b a`"),
            "tooltip: {tooltip}"
        );
        assert!(tooltip.contains("`h` : `And a b`"), "tooltip: {tooltip}");
    }

    #[tokio::test]
    async fn hole_free_document_yields_an_empty_hint_array() {
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, ALL_CHECKED).await;
        let _ = wait_diagnostics(&mut socket, "all-checked diagnostics").await;

        let hints = ask_inlay(&mut service, ALL_CHECKED)
            .await
            .expect("hints array");
        assert!(hints.is_empty(), "no holes, no hints: {hints:?}");
    }
}

//! `front::query` 的单测：真相层的语义与边界。
//!
//! 这里钉的是**语义**（Lean `goalsAt?` 选择、洞 id 稳定性、错误与"正常没有"的
//! 区分、坐标换算），而不是 wire 形状——后者由 `crates/cli/tests/query.rs` 的
//! CLI≡LSP 一致性契约守住。

use super::*;

/// 一个带 `by` 块、多 sub-goal、多洞的画布（与协议文档的例子同形）。
const CANVAS: &str = "\
axiom And : Prop -> Prop -> Prop
axiom And.intro : (a : Prop) -> (b : Prop) -> a -> b -> And a b
axiom Or : Prop -> Prop -> Prop

theorem and_swap : (a : Prop) -> (b : Prop) -> And a b -> And b a := by
  intro a
  intro b
  intro h
  apply And.intro
  sorry
  sorry

theorem open_one (a : Prop) : And a a := sorry
";
fn doc(text: &str) -> QueryDoc {
    let mut doc = QueryDoc::new();
    doc.set_text(text, 1, None);
    doc
}

#[test]
fn state_at_root_before_any_tactic() {
    let doc = doc(CANVAS);
    // `theorem and_swap` 那一行的第 1 列 = 根状态（第一条 tactic 之前）。
    let offset = CANVAS.find("theorem and_swap").expect("decl start");
    let state = doc.state_at(offset).expect("root state is answerable");
    assert_eq!(
        state.decl.as_ref().map(|d| d.name.as_str()),
        Some("and_swap")
    );
    assert_eq!(state.step, -1, "before the first tactic = the root state");
    // 协议 `soko/stateAt`：根状态（`step: -1`）= **完整声明类型的内核渲染文本**
    // + **空 binders**，`span` = 声明范围（`docs/protocol.md`；VS Code 客户端
    // 依赖这一条）。这不是"走查后的剩余目标"——那属于 tactic 之后的状态。
    assert_eq!(
        state.goal.as_deref(),
        Some("forall (a b : Prop), And a b -> And b a"),
        "the root goal is the declared type, kernel-rendered"
    );
    assert!(
        state.binders.is_empty(),
        "the root state has no hypotheses yet: {:?}",
        state.binders
    );
    assert_eq!(
        state.span,
        state.decl.as_ref().map(|d| (d.start, d.end)),
        "the root span is the declaration's range"
    );
}

/// 没有 `by` 块的**半成品**证明（lambda 前缀 + `sorry`）：协议要求退回声明自己的
/// 剩余目标与上下文（`step: -1`、`total: 0`），而不是"根状态"（那是有 tactic 的
/// 声明在第一条之前的状态：目标 = 声明类型、binders 为空）。
///
/// 这是 `docs/protocol.md` §`soko/stateAt` 的原话（"For a declaration without a
/// `by` block both are `-1`/`0` and the response falls back to the declaration's
/// remaining goal/context"）；丢掉 binders 会让 Infoview 在半成品证明上看不到
/// 已经引入的假设。
#[test]
fn state_at_open_declaration_without_a_by_block_keeps_its_context() {
    let text = "example : (a : Prop) -> a -> a := fun (a : Prop) => fun (h : a) => sorry\n";
    let doc = doc(text);
    let state = doc.state_at(0).expect("answerable");
    assert_eq!(state.step, -1, "no tactics → the declaration-level state");
    assert_eq!(state.total, 0, "no tactics → no per-tactic states");
    assert_eq!(
        state.goals.len(),
        1,
        "the half-written proof still has a goal: {:?}",
        state.goals
    );
    assert_eq!(state.goal.as_deref(), Some("a"), "the remaining goal");
    let names: Vec<&str> = state.binders.iter().map(|b| b.name.as_str()).collect();
    assert_eq!(
        names,
        vec!["a", "h"],
        "the introduced hypotheses must survive: {:?}",
        state.binders
    );
}

/// 没有 `by` 块且**已闭合**的声明（`axiom`/已证完的声明）：协议规定 `goals: []`
/// （wire 上 `goal: null` = "证明已闭合"）。若这里返回声明类型，客户端会把一条
/// 已解决的声明显示成"还剩一个目标"。
#[test]
fn state_at_closed_declaration_without_a_by_block_has_no_goal() {
    let doc = doc(CANVAS);
    let offset = CANVAS.find("axiom And :").expect("decl start");
    let state = doc.state_at(offset).expect("answerable");
    assert_eq!(state.step, -1);
    assert_eq!(state.total, 0);
    assert!(
        state.goals.is_empty(),
        "a closed proof has no goals: {:?}",
        state.goals
    );
    assert!(state.goal.is_none(), "no goal on a closed declaration");
    assert!(state.binders.is_empty());
}

#[test]
fn state_at_inside_a_tactic_shows_the_entering_state() {
    let doc = doc(CANVAS);
    // 光标落在 `apply And.intro` 内部 → 进入该 tactic **之前**的状态
    // （三条 intro 已跑完：a、b、h 都在上下文里，目标还是 `And b a`）。
    let offset = CANVAS.find("apply And.intro").expect("tactic") + 3;
    let state = doc.state_at(offset).expect("answerable");
    assert_eq!(
        state.step, 2,
        "`apply` is the 4th tactic → entering state is 2"
    );
    assert_eq!(state.goals.len(), 1);
    assert_eq!(state.goals[0].goal, "And b a");
    let names: Vec<&str> = state.goals[0]
        .binders
        .iter()
        .map(|b| b.name.as_str())
        .collect();
    assert_eq!(names, vec!["a", "b", "h"], "the intros are in scope");
}

#[test]
fn state_at_after_apply_lists_every_sub_goal() {
    let doc = doc(CANVAS);
    // 最后一个 `sorry` 之前 → `apply And.intro` 之后，两个子目标都在。
    let apply_end = CANVAS.find("apply And.intro").expect("tactic") + "apply And.intro".len();
    let offset = CANVAS[apply_end..]
        .find("sorry")
        .map(|i| apply_end + i + 1)
        .expect("the first sorry after apply");
    let state = doc.state_at(offset).expect("answerable");
    assert_eq!(state.step, 3, "`apply` is tactic index 3 → its state is 3");
    assert_eq!(state.total, 6, "3 intros + apply + 2 sorrys");
    assert_eq!(
        state.goals.len(),
        2,
        "`apply And.intro` leaves two goals; both must be visible: {:?}",
        state.goals.iter().map(|g| &g.goal).collect::<Vec<_>>()
    );
    // 单值字段恒等于 goals[0]（老客户端契约）。
    assert_eq!(state.goal.as_deref(), Some(state.goals[0].goal.as_str()));
    assert_eq!(state.binders, state.goals[0].binders);
}

#[test]
fn state_at_outside_declarations_is_an_error_not_an_empty_answer() {
    let doc = doc("-- 只有注释\n");
    let err = doc.state_at(0).expect_err("no declaration at offset 0");
    assert_eq!(err, QueryError::OutsideDeclarations);
    assert_eq!(err.code(), "outside-declarations");
}

#[test]
fn state_at_past_the_end_is_out_of_range() {
    let doc = doc(CANVAS);
    assert_eq!(
        doc.state_at(CANVAS.len() + 10).expect_err("past EOF"),
        QueryError::PositionOutOfRange
    );
}

#[test]
fn holes_are_addressable_and_stably_identified() {
    let doc = doc(CANVAS);
    let holes = doc.holes();
    assert_eq!(
        holes.len(),
        3,
        "two sub-goal holes after `apply` plus the open theorem's hole: {holes:?}"
    );
    // `id` = `<declName>:<index>`，文档版本内稳定且唯一。
    let ids: Vec<&str> = holes.iter().map(|h| h.id.as_str()).collect();
    let mut unique = ids.clone();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(unique.len(), ids.len(), "hole ids must be unique: {ids:?}");
    assert!(
        ids.iter().any(|id| id.starts_with("and_swap:")),
        "hole ids carry the declaration name: {ids:?}"
    );
    assert!(
        ids.iter().any(|id| id.starts_with("open_one:")),
        "the second declaration's hole is listed too: {ids:?}"
    );
    // 文件序。
    assert!(holes.windows(2).all(|w| w[0].start <= w[1].start));
}

#[test]
fn next_hole_walks_forward_and_backward() {
    let doc = doc(CANVAS);
    let holes = doc.holes();
    // 实测布局（文件序）：`apply And.intro` 之后的两个子目标洞在最前且**同址**
    // （`assemble` 给每个叶子洞同一个 hole_span），`open_one` 的洞在其后。
    let ids: Vec<&str> = holes.iter().map(|h| h.id.as_str()).collect();
    assert_eq!(
        ids,
        vec!["and_swap:0", "and_swap:1", "open_one:0"],
        "{holes:?}"
    );
    assert_eq!(
        holes[0].start, holes[1].start,
        "the two sub-goals of one `apply` share a source position: {holes:?}"
    );
    // 从第一个洞出发 → 下一个是本文件里位置更靠后的那组（同址组算一步）。
    // 第一个洞（`and_swap` 的子目标组）之后，位置更靠后的是 `open_one` 的洞。
    let found = doc
        .next_hole(holes[0].start, true)
        .expect("another hole exists");
    assert_eq!(found.id, "open_one:0");
    // 同址组内**按位置无法逐个前进**：这正是协议记录的已知限制
    // （`docs/protocol.md` 的 `soko/nextHole` 一节）——组里的第二个洞只能整组
    // 跨过，要逐个寻址就用 `holes[i].id`。
    assert_eq!(
        doc.next_hole(holes[0].start + 1, true).map(|h| h.id),
        Some("open_one:0".to_string()),
        "the same-site pair is stepped over as a group"
    );
    // 往回走：从 `open_one` 的位置回退到子目标组。
    let back = doc
        .next_hole(holes[2].start, false)
        .expect("a hole before it");
    assert_eq!(
        back.id, "and_swap:1",
        "backward lands on the group's last hole"
    );
    assert!(
        doc.next_hole(holes[0].start, false).is_none(),
        "nothing before the first hole"
    );
}

#[test]
fn hints_attach_to_the_declaration_at_the_cursor() {
    let src = "\
-- soko:hint 先看目标最外层的箭头
-- soko:hint 目标形态：拆成 fun (a : Prop) => …
theorem t (a : Prop) : a -> a := sorry
";
    let doc = doc(src);
    let at_decl = src.find("theorem t").expect("decl");
    let hints = doc.hints_at(at_decl);
    assert_eq!(hints.len(), 2, "both ladder steps: {hints:?}");
    assert!(hints[0].contains("最外层的箭头"));
    // 光标在注释里（不属于任何声明）→ 正常为空。
    let comment_only = self::doc("-- 只有注释\n");
    assert!(comment_only.hints_at(0).is_empty());
}

#[test]
fn check_counts_match_the_event_stream() {
    let doc = doc(CANVAS);
    let summary = doc.check();
    // 两条 axiom + 两条 theorem 通过；两个 `exact sorry` 让第一条 theorem 保持开放。
    assert!(summary.counts.decl_checked >= 2, "{:?}", summary.counts);
    assert!(
        summary.counts.exercise_open >= 1,
        "open exercises are counted: {:?}",
        summary.counts
    );
    assert_eq!(summary.version, 1);
    // 与 `--json` 事件流同源：计数必须等于直接编译的事件数。
    let (output, _) = crate::compile::compile_all_with(
        &crate::parse(CANVAS).expect("parse"),
        &CompileOptions::default(),
    );
    let decl_checked = output
        .events
        .iter()
        .filter(|e| matches!(e, crate::compile::CheckEvent::DeclarationChecked { .. }))
        .count();
    assert_eq!(summary.counts.decl_checked, decl_checked);
    assert!(
        summary.failed.is_empty(),
        "no kernel rejections: {:?}",
        summary.failed
    );
}

#[test]
fn check_reports_kernel_failures() {
    let open = doc("theorem bad : Prop -> Prop := sorry\n");
    // `sorry` 是合法开放状态，不是失败。
    assert!(open.check().failed.is_empty());
    let rejected = doc("example : Prop := 1\n");
    let summary = rejected.check();
    assert!(
        !summary.failed.is_empty(),
        "a kernel-rejected declaration must be reported: {:?}",
        summary
    );
}

#[test]
fn goals_lists_every_declaration_with_its_type() {
    let doc = doc(CANVAS);
    let goals = doc.goals(false);
    assert_eq!(goals.len(), 5, "3 axioms + 2 theorems: {goals:?}");
    let swap = goals
        .iter()
        .find(|d| d.name == "and_swap")
        .expect("and_swap listed");
    assert_eq!(swap.kind, "theorem");
    assert_eq!(swap.status, "open");
    assert!(swap.ty.is_some(), "the signature is kernel-rendered");
    assert!(
        swap.holes.len() >= 2,
        "both spine holes are listed: {:?}",
        swap.holes
    );
    assert!(
        !swap.ty_runs.is_empty(),
        "ty_runs carry the semantic classification"
    );
    let axiom = goals.iter().find(|d| d.name == "And").expect("And listed");
    assert_eq!(axiom.kind, "axiom");
    assert_eq!(axiom.status, "checked");
    assert!(
        axiom.goals.is_empty(),
        "checked declarations have no open goals"
    );
}

#[test]
fn probe_fills_sub_goal_types_that_the_walk_cannot_determine() {
    let doc = doc(CANVAS);
    let with_probe = doc.goals(true);
    let swap = with_probe
        .iter()
        .find(|d| d.name == "and_swap")
        .expect("and_swap");
    assert!(
        swap.sub_goals.iter().any(|s| s.ty.is_some()),
        "the request-time kernel probe fills expected types: {:?}",
        swap.sub_goals
    );
}

#[test]
fn reduce_returns_the_kernel_normal_form() {
    let doc = doc("def two : Nat := 2\n");
    let answer = doc.reduce("1 + 1").expect("reduce answers");
    assert_eq!(answer.value.trim(), "2", "kernel normal form, not text");
}

#[test]
fn line_col_round_trips_with_utf16_columns() {
    // 含多字节与 BMP 外字符（emoji 占 2 个 UTF-16 code unit）。
    let text = "abc\nαβ🙂x\nlast";
    for offset in 0..=text.len() {
        if !text.is_char_boundary(offset) {
            continue;
        }
        let (line, col) = line_col_of(text, offset);
        assert_eq!(
            offset_of_line_col(text, line, col),
            Some(offset),
            "round trip at offset {offset} ({line}:{col})"
        );
    }
    // 越界行 → None（调用方转成位置错误）。
    assert_eq!(offset_of_line_col(text, 99, 1), None);
    assert_eq!(offset_of_line_col(text, 0, 1), None);
}

#[test]
fn status_and_name_helpers_match_the_protocol_vocabulary() {
    assert_eq!(status_str(DeclStatus::Checked), "checked");
    assert_eq!(status_str(DeclStatus::Open), "open");
    assert_eq!(status_str(DeclStatus::Failed), "failed");
    // 匿名 example 用 `example@<line>`（协议既有形式）。
    let doc = doc("example : Prop := sorry\n");
    let goals = doc.goals(false);
    assert_eq!(goals.len(), 1);
    assert!(
        goals[0].name.starts_with("example@"),
        "anonymous examples are named by kind@line: {}",
        goals[0].name
    );
}

#[test]
fn prelude_mode_switch_recompiles_in_bare_mode() {
    let mut doc = QueryDoc::new();
    doc.set_text("axiom P : Prop\n", 1, None);
    assert!(doc.check().failed.is_empty());
    // 切到 Bare 后 `Nat` 之类不再存在——这里只断言切换本身不 panic 且版本前进。
    doc.set_text("axiom P : Prop\n", 2, Some(PreludeMode::Bare));
    assert_eq!(doc.version, 2);
    assert!(doc.goals(false).len() == 1);
}

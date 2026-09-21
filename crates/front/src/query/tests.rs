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
    let holes = doc.holes().expect("the canvas parses");
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
    let holes = doc.holes().expect("the canvas parses");
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
        .expect("the canvas parses")
        .expect("another hole exists");
    assert_eq!(found.id, "open_one:0");
    // 同址组内**按位置无法逐个前进**：这正是协议记录的已知限制
    // （`docs/protocol.md` 的 `soko/nextHole` 一节）——组里的第二个洞只能整组
    // 跨过，要逐个寻址就用 `holes[i].id`。
    assert_eq!(
        doc.next_hole(holes[0].start + 1, true)
            .expect("the canvas parses")
            .map(|h| h.id),
        Some("open_one:0".to_string()),
        "the same-site pair is stepped over as a group"
    );
    // 往回走：从 `open_one` 的位置回退到子目标组。
    let back = doc
        .next_hole(holes[2].start, false)
        .expect("the canvas parses")
        .expect("a hole before it");
    assert_eq!(
        back.id, "and_swap:1",
        "backward lands on the group's last hole"
    );
    assert!(
        doc.next_hole(holes[0].start, false)
            .expect("the canvas parses")
            .is_none(),
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
    // `Prop -> Prop` 是 Type 层的 Pi，做 `theorem` 签名会被内核拒（G-01）；
    // 这里要的是"合法开放练习" ⇒ 用 `example`（签名只需是个类型）。
    let open = doc("example : Prop -> Prop := sorry\n");
    // `sorry` 是合法开放状态，不是失败。
    assert!(open.check().failed.is_empty());
    let rejected = doc("example : Prop := 1\n");
    let summary = rejected.check();
    assert!(
        !summary.failed.is_empty(),
        "a kernel-rejected declaration must be reported: {:?}",
        summary
    );
    // G-10 回归：parse 诊断与内核拒绝是**两类**失败，谁也不吞谁。这里只分辨
    // `code`（摘要层的失败条目今天没有 `stage`，见 WO-003 的修法 A/B）。
    // 语法错误夹具：`infix:50 " e " => mem` 曾经是"随便一条解析不了的文本"，
    // 0.59.0 起它是一条**有意义**的 `notation-shape` 诊断（记法符号不能是标识符
    // 词，G-04 / WO-011）——这里要的是通用 parse 错误，所以换成括号不配对。
    let unparsable = doc("def p : Prop := (a\n");
    let summary = unparsable.check();
    assert_eq!(summary.failed.len(), 1, "{summary:?}");
    assert_eq!(
        summary.failed[0].code, "unexpected-token",
        "the parse code, not a kernel code: {summary:?}"
    );
}

/// G-10：解析失败必须作为诊断出现在 `check().failed` 里。`ok:true` 只表示
/// "问出来了"（答案就是"这份文本解析不了"），不能吞成"全零 + 没有失败"的假绿；
/// `--json` 事件流今天就是对的（`stage:"parse"`），摘要视图必须追上它。
///
/// 退出码由 `failed` 的数量推导（`crates/cli/src/query.rs`），所以"`failed` 非空"
/// 这一条同时就是"exit 1"的根据。
#[test]
fn check_reports_a_parse_error_instead_of_all_zeros() {
    // 夹具见上一条测试的注释：`infix:50 " e " => mem` 现在是 `notation-shape`
    // 诊断，不再是通用 parse 错误。
    let bad = doc("def p : Prop := (a\n");
    let summary = bad.check();
    assert_eq!(
        summary.failed.len(),
        1,
        "exactly the parse diagnostic: {summary:?}"
    );
    let diag = &summary.failed[0];
    assert_eq!(diag.code, "unexpected-token");
    assert_eq!(
        diag.name, None,
        "解析失败时没有可信的声明名（硬凑一个会误导 agent）: {diag:?}"
    );
    assert_eq!(
        (diag.start, diag.end),
        (19, 19),
        "the byte-offset span: {diag:?}"
    );
    assert!(
        !diag.message.is_empty(),
        "the parse message travels verbatim: {diag:?}"
    );
    assert_eq!(
        summary.counts,
        CheckCounts::default(),
        "nothing was actually checked — the counts stay honestly zero"
    );
    assert_eq!(summary.version, 1);
}

/// G-10 的**缓存路径不变量**：`set_cached_entry` 显式把 `parse_error` 置 `None`
/// （缓存命中的文档按"能解析"处理）。这安全，靠三条合起来：
///   ① 只有带 `import` 的文档才走缓存（`crates/cli/src/query.rs::load_document`）；
///   ② `has_imports` 对解析失败的文本返回 `false`（`parse(..).unwrap_or(false)`）
///      ⇒ 坏文本必然走 `set_text`，`parse_error` 必然被写；
///   ③ 缓存只写 clean 产物（`crates/cli/src/project_cache.rs::store_if_clean`）。
/// 这里把"进得了缓存的文本必然能解析"这条**隐式**前提钉死：谁将来把带诊断的结果
/// 也缓存，这条测试先红，而不是让 G-10 的假绿悄悄复活。
#[test]
fn a_cache_entry_can_never_carry_a_parse_error() {
    let mut cached = QueryDoc::new();
    let src = "import Lib\n\ndef two : Nat := 2\n";
    assert!(
        crate::parse(src).is_ok(),
        "a cache hit's text parses by construction (has_imports only sees parsed files)"
    );
    cached.set_cached_entry(
        src,
        1,
        DocumentReport::default(),
        crate::compile::CompileOutput::default(),
        // 单文件条目：不带项目报告（T-A03 起项目条目才带）。
        None,
    );
    assert!(
        cached.parse_error.is_none(),
        "cache hits never carry a parse error"
    );
    assert!(
        cached.check().failed.is_empty(),
        "and `check` stays quiet for a clean cache hit"
    );
    // 真正的坏文本走 `set_text`（CLI 两条输入通道的合流点）：必然被记录。
    let mut fresh = QueryDoc::new();
    fresh.set_text("infix:50 \" e \" => mem\n", 1, None);
    assert!(
        fresh.parse_error.is_some(),
        "bad text always sets `parse_error`"
    );
    assert_eq!(fresh.check().failed.len(), 1);
}

/// G-17：解析失败的源文本对 `goals`/`holes` 也必须"问不出来"（`NotParsable`），
/// 不能返回空数组 + `ok:true` 假装"这份画布没有声明、没有洞"。
///
/// 协议把"正常的没有"（空数组 / `None`）与"问不出来"（`QueryError`）严格分开
/// （`docs/design/agent-query-channel.md` §4.1）——解析失败属于后者。
#[test]
fn goals_and_holes_report_a_parse_error_instead_of_empty_answers() {
    let bad = doc("infix:50 \" e \" => mem\n");
    for probe in [false, true] {
        assert_eq!(
            bad.goals(probe).expect_err("no declaration is knowable"),
            QueryError::NotParsable,
            "probe = {probe}"
        );
    }
    assert_eq!(
        bad.holes().expect_err("no hole is knowable"),
        QueryError::NotParsable
    );
    assert_eq!(
        bad.next_hole(0, true).expect_err("not parsable"),
        QueryError::NotParsable
    );
    assert_eq!(QueryError::NotParsable.code(), "not-parsable");
    // 对照组：「正常的没有」仍是空数组 / `None`，不是错误。
    let empty = doc("-- 只有注释\n");
    assert!(empty.goals(false).expect("parsable").is_empty());
    assert!(empty.holes().expect("parsable").is_empty());
    assert_eq!(empty.next_hole(0, true).expect("parsable"), None);
}

#[test]
fn goals_lists_every_declaration_with_its_type() {
    let doc = doc(CANVAS);
    let goals = doc.goals(false).expect("the canvas parses");
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
    let with_probe = doc.goals(true).expect("the canvas parses");
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
    let goals = doc.goals(false).expect("parsable");
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
    assert!(doc.goals(false).expect("parsable").len() == 1);
}

/// 项目模式下的判据前缀与子洞探针（I16 待办批次 1）。
///
/// 单文件文档：前缀为空（行为与今天逐字节相同）。
/// 项目文档：前缀 = 依赖模块的源码（去 `import` 行）——`suggest` 的 quick-fix 与
/// 子洞探针都靠它看见被导入的名字；探针不再因为"项目模式"被整段跳过。
#[test]
fn project_documents_expose_a_judge_prefix_and_probe_sub_goals() {
    let dir = std::env::temp_dir().join(format!("soko-query-probe-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    std::fs::write(
        dir.join("Logic.sokonanoda"),
        "axiom And : Prop -> Prop -> Prop\n\
axiom And.intro : (a : Prop) -> (b : Prop) -> a -> b -> And a b\n",
    )
    .expect("write dep");
    let entry_text = "import Logic\n\n\
theorem spine_x (a b : Prop) (h : a) (k : b) : And a b :=\n\
  And.intro a b sorry sorry\n";
    let entry = dir.join("Main.sokonanoda");
    std::fs::write(&entry, entry_text).expect("write entry");

    // 单文件：没有闭包前缀。
    let single = doc("theorem t : Prop -> Prop := fun (p : Prop) => p\n");
    assert_eq!(single.judge_prefix(0), "");

    // 项目：前缀含依赖声明、不含 `import` 行。
    let mut project = QueryDoc::new();
    project.path = Some(entry);
    project.set_text(entry_text, 1, None);
    let prefix = project.judge_prefix(0);
    assert!(prefix.contains("axiom And.intro"), "{prefix}");
    assert!(!prefix.contains("import Logic"), "{prefix}");

    // 探针在项目模式下也生效：`And.intro` 的两个 spine 洞拿到期望类型。
    let goals = project.goals(true).expect("the project parses");
    let spine = goals
        .iter()
        .find(|d| d.name == "spine_x")
        .expect("spine_x listed");
    assert!(
        spine.sub_goals.iter().filter(|s| s.ty.is_some()).count() >= 2,
        "the probe must fill the imported constructor's sub-goal types: {:?}",
        spine.sub_goals
    );
    let _ = std::fs::remove_dir_all(&dir);
}

// ── 项目视图（`query project` / `soko/project`，0.58.0 批次 4）───────────────
//
// 语义守护：模块状态三分（compiled / load-failed / blocked）、拓扑序 + 入口标记、
// 单文件是"另一种合法状态"（`None` + reason）而不是错误。
// 设计：`docs/design/project-view.md`。

fn project_dir(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("soko-query-project-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

/// 视图里的路径是 `canonicalize` 过的（macOS 上 `/var` → `/private/var`），
/// 断言也要走同一条路，否则测试只在某些平台上红。
fn canon(path: &std::path::Path) -> String {
    std::fs::canonicalize(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .display()
        .to_string()
}

/// 写一组文件并返回"打开入口"的 QueryDoc（`set_text` 之前设好 path）。
fn project_doc(dir: &std::path::Path, entry: &str, files: &[(&str, &str)]) -> QueryDoc {
    let mut entry_path = None;
    for (name, text) in files {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create parent");
        }
        std::fs::write(&path, text).expect("write module");
        if *name == entry {
            entry_path = Some(path);
        }
    }
    let path = entry_path.expect("entry file written");
    let text = std::fs::read_to_string(&path).expect("read entry");
    let mut doc = QueryDoc::new();
    doc.path = Some(path);
    doc.set_text(&text, 1, None);
    doc
}

#[test]
fn project_view_lists_the_closure_with_statuses() {
    let dir = project_dir("ok");
    let doc = project_doc(
        &dir,
        "Main.sokonanoda",
        &[
            ("Lib.sokonanoda", "def lib_value : Nat := 2\n"),
            (
                "Main.sokonanoda",
                "import Lib\n\ndef two : Nat := lib_value\n\ntheorem later (a : Prop) : a -> a := by\n  intro h\n  exact h\n\nexample : Nat := sorry\n",
            ),
        ],
    );
    let view = doc.project_view().expect("a project view");
    assert_eq!(view.entry, "Main");
    assert_eq!(view.root, canon(&dir));
    assert_eq!(
        view.manifest, None,
        "zero-config: no manifest, root = entry dir"
    );
    assert_eq!(
        view.modules
            .iter()
            .map(|module| (module.name.as_str(), module.status.as_str(), module.entry))
            .collect::<Vec<_>>(),
        vec![("Lib", "compiled", false), ("Main", "compiled", true)],
        "topological order, entry last and marked"
    );
    assert!(
        view.modules[0].path.ends_with("Lib.sokonanoda"),
        "module paths are absolute file paths: {}",
        view.modules[0].path
    );
    assert_eq!(view.modules[1].imports, vec!["Lib"]);
    assert_eq!(view.counts.modules, 2);
    assert_eq!(view.counts.compiled, 2);
    assert_eq!(view.counts.failed, 0);
    assert_eq!(view.counts.open_exercises, 1);
    assert_eq!(
        view.counts.errors, 0,
        "clean project: {:?}",
        view.diagnostics
    );
    assert!(view.diagnostics.is_empty());
    assert_eq!(doc.project_view_reason(), "available");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn project_view_reports_the_manifest_when_one_exists() {
    let dir = project_dir("manifest");
    let doc = project_doc(
        &dir,
        "src/Main.sokonanoda",
        &[
            ("sokonanoda.toml", "[project]\nname = \"demo\"\n"),
            ("src/Lib.sokonanoda", "def lib_value : Nat := 2\n"),
            (
                "src/Main.sokonanoda",
                "import Lib\n\ndef two : Nat := lib_value\n",
            ),
        ],
    );
    let view = doc.project_view().expect("a project view");
    assert_eq!(
        view.manifest.as_deref(),
        Some(canon(&dir.join("sokonanoda.toml")).as_str()),
        "the manifest walks up from the entry file"
    );
    assert_eq!(view.root, canon(&dir));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn project_view_separates_load_failure_from_being_blocked() {
    let dir = project_dir("broken");
    let doc = project_doc(
        &dir,
        "Main.sokonanoda",
        &[
            ("A.sokonanoda", "def a : Nat := 1\n"),
            // B 自己加载失败（它 import 的模块不存在）——根因在 B。
            ("B.sokonanoda", "import Missing\n\ndef b : Nat := 1\n"),
            // C 只是被 B 拖住——它是受害者，不是根因。
            ("C.sokonanoda", "import B\n\ndef c : Nat := 1\n"),
            (
                "Main.sokonanoda",
                "import A\nimport B\nimport C\n\ndef main_value : Nat := a\n",
            ),
        ],
    );
    let view = doc.project_view().expect("a project view");
    let status = |name: &str| {
        view.modules
            .iter()
            .find(|module| module.name == name)
            .map(|module| (module.status.clone(), module.message.clone()))
            .unwrap_or_else(|| panic!("module {name} missing"))
    };
    assert_eq!(status("A").0, "compiled", "A is untouched");
    let (b_status, b_message) = status("B");
    assert_eq!(b_status, "load-failed", "B's own import is missing");
    let names_the_cause = b_message
        .as_deref()
        .is_some_and(|message| message.contains("Missing"));
    assert!(
        names_the_cause,
        "the root cause must name the missing module: {b_message:?}"
    );
    let (c_status, c_message) = status("C");
    assert_eq!(c_status, "blocked", "C only suffers from B");
    let points_upstream = c_message
        .as_deref()
        .is_some_and(|message| message.contains('B'));
    assert!(
        points_upstream,
        "the blocked module points at its broken dependency: {c_message:?}"
    );
    assert!(view.counts.failed >= 1 && view.counts.blocked >= 1);
    assert!(
        view.diagnostics
            .iter()
            .any(|diag| diag.code == "import-not-found"),
        "the missing module is a project diagnostic: {:?}",
        view.diagnostics
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn project_view_is_none_for_single_files_with_a_reason() {
    let single = doc(CANVAS);
    assert!(single.project_view().is_none());
    assert_eq!(single.project_view_reason(), "no-imports");

    // 有 `import` 但定位不到入口（stdin / `--text` 且没有 `--root`）。
    let mut text_only = QueryDoc::new();
    text_only.set_text("import Lib\n\ndef two : Nat := 2\n", 1, None);
    assert!(text_only.project_view().is_none());
    assert_eq!(text_only.project_view_reason(), "no-path");

    // 解析不了：先修语法，和"单文件"是两回事。
    let mut broken = QueryDoc::new();
    broken.set_text("theorem : : :\n", 1, None);
    assert!(broken.project_view().is_none());
    assert_eq!(broken.project_view_reason(), "parse-error");
}

/// 「多余的 `sorry`」标记（`docs/design/redundant-sorry.md`）：洞级"这是多写的
/// 一行"必须能从查询层问出来，且与真缺口区分开。判定来自**同一条** kernel 终审
/// warning（不另算），所以标记与 warning 永不漂移。
#[test]
fn holes_carry_the_redundant_sorry_mark() {
    let src = "axiom A : Prop\n\
               axiom B : Prop\n\
               axiom f : A -> B\n\
               theorem t (h : A) : B := f h\n\
               \x20 sorry\n\
               theorem genuine (h : A) : B := f\n\
               \x20 sorry\n";
    let doc = doc(src);
    let holes = doc.holes().expect("the canvas parses");
    assert_eq!(holes.len(), 2, "one hole per declaration: {holes:?}");
    let marked: Vec<(&str, bool)> = holes.iter().map(|h| (h.id.as_str(), h.redundant)).collect();
    assert_eq!(
        marked,
        vec![("t:0", true), ("genuine:0", false)],
        "the leftover line is redundant; the missing argument is not: {holes:?}"
    );
    // `state`/`goals` 的洞是同一份数据（同一个 `HoleInfo`）。
    let goals = doc.goals(false).expect("the canvas parses");
    let t = goals.iter().find(|d| d.name == "t").expect("decl t");
    assert!(t.holes.iter().all(|h| h.redundant), "{:?}", t.holes);
    let genuine = goals
        .iter()
        .find(|d| d.name == "genuine")
        .expect("decl genuine");
    assert!(
        genuine.holes.iter().all(|h| !h.redundant),
        "{:?}",
        genuine.holes
    );
}

// ── G-22：`usable()` 是「声明栏能不能用」的唯一判据 ──────────────────────────
//
// 记法随 `import` 传播之后，"用库记法的单元**单文件必然 parse 失败**"是常态，
// 而闭包是好的。这条判据以前在 `check` 与 LSP 各写了一遍、`goals` 漏了
// ⇒ 项目入口的 `soko/goals` 恒为空（声明栏空、`alt+n` 没反应）。

const NOTATION_LIB: &str = "def Set (α : Type) : Type := α -> Prop\n\
def Set.mem (α : Type) (a : α) (A : Set α) : Prop := A a\n\
infix:50 \" ∈ \" => Set.mem\n";

#[test]
fn an_entry_whose_closure_compiles_is_usable_even_if_it_does_not_parse_alone() {
    let dir = project_dir("usable-rescued");
    let entry_text = "import lib.Set\n\n\
theorem mem_self (α : Type) (a : α) (A : Set α) (h : a ∈ A) : a ∈ A := h\n";
    // 入口必须在**模块根**下：没有 `sokonanoda.toml` 时模块根 = 入口所在目录，
    // 而 `import lib.Set` 是相对模块根解析的（入口放子目录会变成 `import-not-found`
    // ——实测踩过）。真课程有清单，所以 `units/` 那种布局才成立。
    let doc = project_doc(
        &dir,
        "Main.sokonanoda",
        &[
            ("lib/Set.sokonanoda", NOTATION_LIB),
            ("Main.sokonanoda", entry_text),
        ],
    );

    // 前提：这份文件**自己**确实解析不了（`∈` 来自 import）——否则这条测试没意义。
    assert!(
        doc.parse_error.is_some(),
        "夹具前提不成立：入口单独 parse 竟然成功了（记法没有随 import 传播？）"
    );
    assert!(
        doc.project_entry_compiled(),
        "夹具前提不成立：闭包没编译成功"
    );

    // G-22：声明栏的数据源必须**答得出来**，而不是 `NotParsable`。
    let goals = doc.goals(false).expect("闭包好 ⇒ goals 必须可用（G-22）");
    assert!(
        goals.iter().any(|d| d.name == "mem_self"),
        "声明栏必须列出 mem_self，实际 = {:?}",
        goals.iter().map(|d| d.name.clone()).collect::<Vec<_>>()
    );
    // 同族：`holes` / `nextHole` 经由 `goals` ⇒ 一起恢复。
    doc.holes().expect("holes 与 goals 同一条判据");
    doc.next_hole(0, true)
        .expect("nextHole 与 goals 同一条判据");
}

#[test]
fn an_entry_whose_closure_also_fails_stays_not_parsable() {
    // 老契约（G-17）不许被 G-22 的修法放宽：**真的**解析不了时仍要 `NotParsable`。
    let dir = project_dir("usable-really-broken");
    let doc = project_doc(
        &dir,
        "Main.sokonanoda",
        &[("Main.sokonanoda", "theorem t : Prop -> := sorry\n")],
    );
    assert!(doc.parse_error.is_some(), "夹具前提：入口有真语法错误");
    assert!(
        !doc.project_entry_compiled(),
        "夹具前提：闭包也失败（入口 LoadFailed）"
    );
    assert_eq!(
        doc.goals(false).unwrap_err(),
        QueryError::NotParsable,
        "闭包也失败 ⇒ 必须仍是 NotParsable（G-17 契约）"
    );
    assert_eq!(doc.holes().unwrap_err(), QueryError::NotParsable);
}

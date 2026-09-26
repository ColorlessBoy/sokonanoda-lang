use super::*;

#[tokio::test]
async fn goals_request_lists_open_exercise_with_hole_range() {
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, EXERCISE).await;
    let _ = wait_diagnostics(&mut socket, "goals diagnostics").await;

    let result = request_goals(&mut service).await;
    let decls = result
        .get("decls")
        .and_then(|d| d.as_array())
        .expect("goals response carries decls");
    assert_eq!(decls.len(), 1, "one open exercise: {result:?}");
    let decl = &decls[0];
    assert_eq!(decl["status"], "open");
    assert_eq!(decl["goal"], "Prop → Prop");
    // The declaration's own type ships with the wire so the Infoview can
    // show it as a hint; runs reconstruct it exactly (§2.1).
    let ty = decl["ty"].as_str().expect("declared type");
    assert_eq!(ty, "Prop → Prop", "declared type text");
    let runs = decl["ty_runs"].as_array().expect("ty_runs array");
    assert_eq!(
        runs.iter()
            .map(|r| r["text"].as_str().unwrap_or_default())
            .collect::<String>(),
        ty,
        "ty_runs reconstruct ty"
    );
    assert!(
        runs.iter().any(|r| r["kind"].as_str() == Some("sort")),
        "`Prop` in the type is a sort: {runs:?}"
    );
    // **T-A5 / R-2 ②**：目标也要在 wire 里带 runs（父 `goal_runs` + 子
    // `goals_runs`）。扩展的声明卡片照 `tok-*` 渲染目标行；这里漏一个字段，
    // 用户看到的就是"目标不高亮"（R-1 同形的静默降级）。
    let goal = decl["goal"].as_str().expect("open goal text");
    assert_eq!(
        reconstruct_runs(&decl["goal_runs"]),
        goal,
        "goal_runs 必须逐字节重建 goal"
    );
    assert!(
        !run_kinds(&decl["goal_runs"]).is_empty(),
        "目标 runs 必须有 kind（否则只能画纯文本）：{:?}",
        decl["goal_runs"]
    );
    let goals = decl["goals"].as_array().expect("goals array");
    let goals_runs = decl["goals_runs"]
        .as_array()
        .expect("goals_runs array (父子对齐)");
    assert_eq!(
        goals_runs.len(),
        goals.len(),
        "goals_runs 必须与 goals 按位置对齐"
    );
    for (text, runs) in goals.iter().zip(goals_runs) {
        assert_eq!(
            reconstruct_runs(runs),
            text.as_str().expect("goal text"),
            "goals_runs[i] 必须重建 goals[i]"
        );
    }
    let hole = decl["hole"].as_object().expect("hole range present");
    let start = hole["start"].as_object().expect("hole start");
    let expected = lsp_pos(EXERCISE, offset_of(EXERCISE, "sorry"));
    assert_eq!(start["line"], expected.line, "hole line (0-based)");
    assert_eq!(start["character"], expected.character, "hole character");
    shutdown(&mut service).await;
}

#[tokio::test]
async fn goals_request_lists_every_open_goal_after_apply() {
    // 练习面板的多目标：`soko/goals` 的每声明 `goals` 数组给出最后一步的
    // 全部未闭合目标（当前在前），非 by 练习回退为单个走查目标。
    let src = "axiom And : Prop -> Prop -> Prop\n\
               axiom P : Prop\n\
               axiom Q : Prop\n\
               axiom And.intro : (a : Prop) -> (b : Prop) -> a -> b -> And a b\n\
               theorem both : And P Q := by apply And.intro; sorry\n";
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, src).await;
    let _ = wait_diagnostics(&mut socket, "goals diagnostics").await;

    let result = request_goals(&mut service).await;
    let decl = result["decls"]
        .as_array()
        .expect("decls")
        .iter()
        .find(|d| d["name"] == "both")
        .expect("the `both` declaration");
    assert_eq!(decl["status"], "open");
    let goals = decl["goals"].as_array().expect("goals array");
    assert_eq!(
        goals,
        &vec![json!("P"), json!("Q")],
        "both goals, current first"
    );
    // T-A5：多目标时**每一个**子目标都要有自己的 runs，且与 `goals` 对齐
    // （声明卡片按 `目标 i/n` 逐行上色；错位会让颜色贴到别的目标上）。
    let goals_runs = decl["goals_runs"]
        .as_array()
        .expect("goals_runs array (父子对齐)");
    assert_eq!(goals_runs.len(), goals.len(), "一个目标一段 runs");
    assert_eq!(reconstruct_runs(&goals_runs[0]), "P", "当前目标在前");
    assert_eq!(reconstruct_runs(&goals_runs[1]), "Q");
    // 父字段：`goal`/`goal_runs` 也要成对（老客户端的单值回退）。
    let single = decl["goal"].as_str().expect("single goal text");
    assert_eq!(reconstruct_runs(&decl["goal_runs"]), single);
    shutdown(&mut service).await;
}

#[tokio::test]
async fn next_hole_navigates_between_two_holes() {
    let src = format!("{EXERCISE}example : Prop := sorry\n");
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, &src).await;
    let _ = wait_diagnostics(&mut socket, "next hole diagnostics").await;

    // 从文件头向前：命中第一个洞。
    let first = ask_next_hole(
        &mut service,
        Position {
            line: 0,
            character: 0,
        },
        true,
    )
    .await;
    let first_range: Option<Range> = serde_json::from_value(first).expect("valid hole range");
    let first_range = first_range.expect("a hole ahead of (0,0)");
    assert_eq!(first_range.start.line, 0, "first hole is on line 0");
    // 从第一个洞再向前：命中第二个洞（line 1）。
    let second = ask_next_hole(&mut service, first_range.start, true).await;
    let second_range: Option<Range> = serde_json::from_value(second).expect("valid hole range");
    let second_range = second_range.expect("a second hole ahead of the first");
    assert_eq!(second_range.start.line, 1, "second hole is on line 1");
    // 从第二个洞向后：回到第一个洞。
    let back = ask_next_hole(&mut service, second_range.start, false).await;
    let back_range: Option<Range> = serde_json::from_value(back).expect("valid hole range");
    assert_eq!(
        back_range.expect("a hole behind").start.line,
        0,
        "backward navigation returns to the first hole"
    );
    shutdown(&mut service).await;
}

#[tokio::test]
async fn goals_request_carries_sub_holes_with_expected_types() {
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, AND_MULTI_HOLE).await;
    let _ = wait_diagnostics(&mut socket, "multi-hole diagnostics").await;

    let result = request_goals(&mut service).await;
    let decls = result["decls"].as_array().expect("decls array");
    let decl = &decls[decls.len() - 1];
    assert_eq!(decl["status"], "open");
    let holes = decl["holes"].as_array().expect("holes array");
    assert_eq!(holes.len(), 2, "two spine holes: {result:?}");
    assert!(
        holes[0]["range"].is_object(),
        "holes are {{range, id}} objects, not bare ranges: {result:?}"
    );
    assert_eq!(holes[0]["id"], "and_intro_rule:0", "named decl id form");
    assert_eq!(holes[1]["id"], "and_intro_rule:1");
    let sub_goals = decl["sub_goals"].as_array().expect("sub_goals array");
    assert_eq!(sub_goals.len(), 2);
    assert_eq!(
        sub_goals[0]["ty"], "a",
        "parameter hole expects the goal's own argument"
    );
    assert_eq!(sub_goals[1]["ty"], "b");
    assert_eq!(
        holes[0]["range"], sub_goals[0]["range"],
        "holes stay positionally aligned with sub_goals"
    );
    shutdown(&mut service).await;
}

#[tokio::test]
async fn goals_request_probes_preceding_hole_penetration() {
    // B′ 把「前置实参是洞」的子洞类型留成 null；请求期 kernel 探针把
    // 第一个洞提升为局部 `_h0 : Prop`，第二个洞（`Not a`）得 `Not _h0`。
    // wire 形状不变，只是 sub_goals[i].ty 从 null 变成文本。
    let src = "axiom False : Prop\n\
               def Not : Prop -> Prop := fun (a : Prop) => a -> False\n\
               axiom h : (a : Prop) -> Not a -> a\n\
               theorem t : (a : Prop) -> Not a -> a :=\n\
                 fun (a : Prop) => fun (na : Not a) => h (sorry) (sorry)\n";
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, src).await;
    let _ = wait_diagnostics(&mut socket, "probe diagnostics").await;

    let result = request_goals(&mut service).await;
    let decls = result["decls"].as_array().expect("decls array");
    let decl = &decls[decls.len() - 1];
    assert_eq!(decl["status"], "open");
    let sub_goals = decl["sub_goals"].as_array().expect("sub_goals array");
    assert_eq!(sub_goals.len(), 2, "{result:?}");
    assert_eq!(sub_goals[0]["ty"], "Prop");
    assert_eq!(
        sub_goals[1]["ty"], "Not _h0",
        "the second hole's expected type comes from the kernel probe"
    );
    // 契约不变：客户端不得文本扫洞——id 仍在 holes 里，位置对齐。
    assert_eq!(decl["holes"][1]["id"], "t:1");
    shutdown(&mut service).await;
}

#[tokio::test]
async fn goals_request_probes_one_level_nested_hole() {
    // 一层嵌套洞 `h (g sorry)`：廉价 walk 给出内层 sorry 的精确 span，
    // 请求期探针填上 `g` 的定义域。
    let src = "axiom g : (a : Prop) -> Prop\n\
               axiom h : (b : Prop) -> Prop\n\
               axiom P : Prop\n\
               theorem t : P := h (g sorry)\n";
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, src).await;
    let _ = wait_diagnostics(&mut socket, "nested probe diagnostics").await;

    let result = request_goals(&mut service).await;
    let decls = result["decls"].as_array().expect("decls array");
    let decl = &decls[decls.len() - 1];
    let holes = decl["holes"].as_array().expect("holes array");
    assert_eq!(holes.len(), 1, "the inner sorry is the hole: {result:?}");
    let sub_goals = decl["sub_goals"].as_array().expect("sub_goals array");
    assert_eq!(sub_goals[0]["ty"], "Prop");
    shutdown(&mut service).await;
}

#[tokio::test]
async fn goals_request_ids_holes_by_decl_and_order() {
    // id = "<declName>:<index>" (docs/protocol.md): named declarations use
    // their name; anonymous examples use the `example@<line>` name form
    // (render::decl_name); the index counts holes in offset order.
    // G-01 起 `theorem` 的签名必须是**真命题**：`Eq.{1} Nat 0 0` 是 prelude 里
    // 现成的命题，不用额外加声明（原来写的 `Prop` 会被内核拒）。
    let src = "theorem named : Eq.{1} Nat 0 0 := sorry\nexample : Prop := sorry\n";
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, src).await;
    let _ = wait_diagnostics(&mut socket, "hole id diagnostics").await;

    let result = request_goals(&mut service).await;
    let decls = result["decls"].as_array().expect("decls array");
    assert_eq!(decls.len(), 2, "one entry per declaration: {result:?}");
    let named = &decls[0];
    let holes = named["holes"].as_array().expect("holes array");
    assert_eq!(holes.len(), 1);
    assert_eq!(holes[0]["id"], "named:0", "named declaration id form");
    let hole_start = holes[0]["range"]["start"].as_object().expect("hole start");
    let expected = lsp_pos(src, offset_of(src, "sorry"));
    assert_eq!(hole_start["line"], expected.line);
    assert_eq!(hole_start["character"], expected.character);
    let anon = &decls[1];
    let holes = anon["holes"].as_array().expect("holes array");
    assert_eq!(anon["name"], "example@2", "anonymous example name form");
    assert_eq!(holes[0]["id"], "example@2:0", "anonymous example id form");
    shutdown(&mut service).await;
}

#[tokio::test]
async fn next_hole_traverses_sub_holes_within_one_declaration() {
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, AND_MULTI_HOLE).await;
    let _ = wait_diagnostics(&mut socket, "next hole diagnostics").await;

    let first = ask_next_hole(
        &mut service,
        Position {
            line: 0,
            character: 0,
        },
        true,
    )
    .await;
    let first_range: Option<Range> = serde_json::from_value(first).expect("range");
    let first_range = first_range.expect("first sub-hole");
    let second = ask_next_hole(&mut service, first_range.start, true).await;
    let second_range: Option<Range> = serde_json::from_value(second).expect("range");
    let second_range = second_range.expect("second sub-hole in the same declaration");
    assert_ne!(
        first_range.start, second_range.start,
        "the two sub-holes are distinct positions"
    );
    shutdown(&mut service).await;
}

/// **`soko/goals` 的 `decl.goal` 必须带记法**（T-C21 的判据）。
///
/// 这条路（不带 `by` 的开练习 = 生产者 2）本来就好，但此前**零测试**——线 C 的
/// 其它环节都在动显示文本，先把它钉在 **wire 上**（前端单测在
/// `crates/front/src/query/tests.rs`，这里钉的是"过了 `query_map` 之后还在"）。
#[tokio::test]
async fn goals_keep_notation_in_the_goal_text() {
    let src = "def Set (α : Type) : Type := α -> Prop\n\
               def Set.mem (α : Type) (a : α) (A : Set α) : Prop := A a\n\
               infix:50 \" ∈ \" => Set.mem\n\
               def Set.subset (α : Type) (A B : Set α) : Prop := forall (x : α), A x -> B x\n\
               infix:50 \" ⊆ \" => Set.subset\n\
               theorem open_subset (α : Type) (A B : Set α) : A ⊆ B -> (∀ (a : α), a ∈ A -> a ∈ B) := sorry\n";
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, src).await;
    let _ = wait_diagnostics(&mut socket, "goals diagnostics").await;

    let result = request_goals(&mut service).await;
    let decl = result["decls"]
        .as_array()
        .expect("decls")
        .iter()
        .find(|d| d["name"] == "open_subset")
        .expect("open_subset listed");
    let goal = decl["goal"]
        .as_str()
        .expect("an open exercise carries a goal");
    assert!(
        goal.contains('⊆'),
        "`soko/goals` 的 goal 必须带记法：{goal}"
    );
    assert!(
        goal.contains('∈'),
        "`soko/goals` 的 goal 必须带记法：{goal}"
    );
    assert!(
        !goal.contains("Set.subset") && !goal.contains("Set.mem"),
        "点名形式不该出现在 goal 里：{goal}"
    );
    shutdown(&mut service).await;
}

/// **R-1 的判据**：`soko/goals` 的声明条目必须带 `value` / `value_runs` ✓
/// —— 这是 Infoview `def` 卡片第二行（`:= <值>`）的**唯一**数据来源 ✓。
///
/// 修复前的形态是"三段式断链" ✗：前端算好了 `value` ✓、CLI 的 `query goals`
/// 也给了 ✓，**唯独 LSP 的 `decl_info` 漏映射** ✗ ⇒ 扩展的
/// `Array.isArray(decl.value_runs)` 恒为 false ⇒ 那一行**静默消失** ✓✗
/// （扩展是"有就渲染"的宽容实现，所以 e2e 也抓不到 ✗）。
///
/// **防分叉守卫**：`value_runs` 必须与 `ty_runs` 走**同一个** `run_info` 实现 ✓
/// —— 判据是"把 runs 拼起来 == 原文" ✓（`ty_runs` 的测试就是这么写的 ✓），
/// 两条路共用一份实现就不会再分叉 ✓。
#[tokio::test]
async fn goals_request_carries_the_declaration_value_for_defs() {
    const SRC: &str = "def my_id (a : Prop) : Prop := a\n";
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, SRC).await;
    let _ = wait_diagnostics(&mut socket, "value diagnostics").await;

    let result = request_goals(&mut service).await;
    let decls = result
        .get("decls")
        .and_then(|d| d.as_array())
        .expect("goals response carries decls");
    let decl = decls
        .iter()
        .find(|d| d["name"] == "my_id")
        .expect("the def shows up in the declaration list");

    let value = decl["value"].as_str().expect("def carries its value");
    assert!(
        value.contains('a'),
        "value is the body after `:=`, got {value:?}"
    );
    let runs = decl["value_runs"]
        .as_array()
        .expect("value_runs array (Infoview renders this)");
    assert!(!runs.is_empty(), "value_runs must not be empty");
    assert_eq!(
        runs.iter()
            .map(|r| r["text"].as_str().unwrap_or_default())
            .collect::<String>(),
        value,
        "value_runs 必须能重建 value（与 ty_runs 同款守卫，防两条路分叉）"
    );
}

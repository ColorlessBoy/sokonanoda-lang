use super::*;

#[tokio::test]
async fn state_at_inside_a_tactic_shows_the_entering_state() {
    // 光标停在 tactic `intro h` 上：学习者要看到的是「这条 tactic 进来时的目标」。
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, BY_OPEN).await;
    let _ = wait_diagnostics(&mut socket, "stateAt diagnostics").await;

    let result = ask_state_at(&mut service, BY_OPEN, offset_of(BY_OPEN, "intro h")).await;
    assert_eq!(result["decl"]["name"], "open");
    assert_eq!(result["decl"]["kind"], "theorem");
    assert_eq!(result["decl"]["status"], "open");
    assert_eq!(
        result["step"], 0,
        "entering the second tactic = after step 0"
    );
    assert_eq!(result["total"], 2);
    // 线 C（T-C22）：`by` 步进的展示副本带记法 ⇒ `And a a -> a` 打成 `a ∧ a -> a`。
    assert_eq!(result["goal"], "a ∧ a → a");
    let binders = result["binders"].as_array().expect("binders array");
    assert_eq!(binders.len(), 1);
    assert_eq!(binders[0]["name"], "a");
    assert_eq!(binders[0]["ty"], "Prop");
    assert!(result["span"].is_object(), "highlight range present");
    shutdown(&mut service).await;
}

#[tokio::test]
async fn state_at_carries_semantic_runs_for_goals_and_hypotheses() {
    // docs/design/goal-rendering.md §2.1: `soko/stateAt` ships the same
    // classification the editor's semantic tokens use, so the Infoview can
    // colour identically instead of inventing its own rules.
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, BY_OPEN).await;
    let _ = wait_diagnostics(&mut socket, "stateAt diagnostics").await;

    let result = ask_state_at(&mut service, BY_OPEN, offset_of(BY_OPEN, "intro h")).await;
    let goal = result["goal"].as_str().expect("goal");
    assert_eq!(
        reconstruct_runs(&result["goal_runs"]),
        goal,
        "runs must reconstruct the goal text exactly"
    );
    let binder = &result["binders"][0];
    assert_eq!(
        reconstruct_runs(&binder["ty_runs"]),
        binder["ty"].as_str().expect("binder ty")
    );
    let goal_kinds = run_kinds(&result["goal_runs"]);
    assert!(
        goal_kinds.contains(&"binder"),
        "the hypothesis `a` must classify as a binder: {goal_kinds:?}"
    );
    // **线 C（T-C22/T-C30）之后**：目标折成 `a ∧ a -> a` ⇒ 这里不再有 `And` 的
    // `axiom_use` 运行（`∧` 是**记法符号**），取而代之的是 `keyword`
    // ——T-C30 把记法符号喂进了 runs 分类（与源里 `Command::Notation` 同一条规则）。
    assert!(
        goal_kinds.contains(&"keyword"),
        "记法符号要着成 `keyword`：{goal_kinds:?}"
    );
    assert!(
        !goal_kinds.contains(&"unknown_ident"),
        "记法符号不该被当成未知标识符：{goal_kinds:?}"
    );
    let binder_kinds = run_kinds(&binder["ty_runs"]);
    assert!(
        binder_kinds.contains(&"sort"),
        "the hypothesis type `Prop` is a sort: {binder_kinds:?}"
    );
    assert_eq!(
        result["goals"][0]["goal_runs"], result["goal_runs"],
        "single-value runs mirror goals[0]"
    );
    shutdown(&mut service).await;
}

#[tokio::test]
async fn hover_goal_text_equals_the_state_at_run_projection() {
    // One content producer (docs/design/goal-rendering.md §2.1): the hover's
    // `sokonanoda` fence text must be the text projection of the very runs
    // `soko/stateAt` hands the Infoview (`goals[].binders[].ty_runs` +
    // `goals[].goal_runs`), so hover and Infoview can never drift.
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, BY_OPEN).await;
    let _ = wait_diagnostics(&mut socket, "stateAt diagnostics").await;

    let at = offset_of(BY_OPEN, "intro h");
    let state = ask_state_at(&mut service, BY_OPEN, at).await;
    let goals = state["goals"].as_array().expect("goals array");
    assert!(!goals.is_empty(), "entering `intro h` has an open goal");
    let mut projected = Vec::new();
    for goal in goals {
        let mut block = String::new();
        for binder in goal["binders"].as_array().expect("binders array") {
            block.push_str(binder["name"].as_str().expect("binder name"));
            block.push_str(" : ");
            block.push_str(&reconstruct_runs(&binder["ty_runs"]));
            block.push('\n');
        }
        block.push_str("⊢ ");
        block.push_str(&reconstruct_runs(&goal["goal_runs"]));
        projected.push(block);
    }
    let markup = hover_markup_at(&mut service, BY_OPEN, at).await;
    for block in &projected {
        assert!(
            markup.contains(block.as_str()),
            "hover must contain the stateAt projection:\nhover={markup:?}\nblock={block:?}"
        );
    }
    shutdown(&mut service).await;
}

#[tokio::test]
async fn state_at_after_the_last_tactic_shows_the_remaining_goal() {
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, BY_OPEN).await;
    let _ = wait_diagnostics(&mut socket, "stateAt diagnostics").await;

    let after = offset_of(BY_OPEN, "intro h") + "intro h".len();
    let result = ask_state_at(&mut service, BY_OPEN, after).await;
    assert_eq!(result["step"], 1);
    assert_eq!(result["goal"], "a");
    let binders = result["binders"].as_array().expect("binders array");
    assert_eq!(binders.len(), 2, "a and h are both in context");
    assert_eq!(binders[1]["name"], "h");
    shutdown(&mut service).await;
}

#[tokio::test]
async fn state_at_lists_all_open_goals_after_apply() {
    // `apply And.intro` 开出两个子目标；`soko/stateAt` 必须一次给出全部
    // （当前目标在首位），兼容单值字段仍等于 `goals[0]`。
    let src = "axiom And : Prop -> Prop -> Prop\n\
               axiom P : Prop\n\
               axiom Q : Prop\n\
               axiom And.intro : (a : Prop) -> (b : Prop) -> a -> b -> And a b\n\
               theorem both : And P Q := by apply And.intro; sorry\n";
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, src).await;
    let _ = wait_diagnostics(&mut socket, "stateAt diagnostics").await;

    // 光标停在 `sorry` 上：进入 sorry 的状态 = `apply` 执行后。
    let result = ask_state_at(&mut service, src, offset_of(src, "sorry")).await;
    assert_eq!(
        result["step"], 0,
        "entering the second tactic = after apply"
    );
    assert_eq!(result["total"], 2);
    let goals = result["goals"].as_array().expect("goals array");
    assert_eq!(goals.len(), 2, "both apply sub-goals are listed");
    assert_eq!(goals[0]["goal"], "P");
    assert_eq!(goals[1]["goal"], "Q");
    // 兼容单值字段 = 当前（首个）目标。
    assert_eq!(result["goal"], "P");
    assert_eq!(result["goal"], goals[0]["goal"]);
    shutdown(&mut service).await;
}

#[tokio::test]
async fn hover_on_a_tactic_shows_the_entering_goal_state() {
    // 用户需求：hover 每个 tactic → 中间 goal state（Lean Infoview 式）。
    // 进入某 tactic 的状态 = 上一步执行后（`soko/stateAt` 同一语义）。
    let src = "axiom And : Prop -> Prop -> Prop\n\
               axiom And.intro : (a : Prop) -> (b : Prop) -> a -> b -> And a b\n\
               axiom P : Prop\n\
               axiom Q : Prop\n\
               theorem both : And P Q := by apply And.intro; sorry\n";
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, src).await;
    let _ = wait_diagnostics(&mut socket, "tactic hover diagnostics").await;

    // hover `apply`：进入它时目标 = 根状态 `And P Q`。
    let at = offset_of(src, "apply And.intro");
    let hover = hover_opt_at(&mut service, src, at)
        .await
        .expect("hover on `apply` must answer");
    let HoverContents::Markup(markup) = hover.contents else {
        panic!("expected markup hover");
    };
    // 线 C（T-C20）之后目标文本带记法（`And P Q` → `P ∧ Q`）。
    assert!(
        markup.value.contains("⊢ P ∧ Q"),
        "tactic hover shows the entering goal, notation-folded: {:?}",
        markup.value
    );
    // Presentation: the tactic itself + `sokonanoda` code fences so both
    // the tactic and the goal state are syntax-highlighted.
    assert!(
        markup.value.contains("```sokonanoda\napply And.intro"),
        "hover header renders the tactic as a highlighted code block: {:?}",
        markup.value
    );
    assert!(
        markup.value.contains("```sokonanoda"),
        "hover goal state is a highlighted code fence: {:?}",
        markup.value
    );

    // hover `sorry`：进入它时有 apply 开出的两个子目标 P、Q。
    let at_sorry = offset_of(src, "sorry");
    let hover2 = hover_opt_at(&mut service, src, at_sorry)
        .await
        .expect("hover on `sorry` must answer");
    let HoverContents::Markup(m2) = hover2.contents else {
        panic!("expected markup hover");
    };
    assert!(m2.value.contains("⊢ P"), "hover: {:?}", m2.value);
    assert!(m2.value.contains("⊢ Q"), "hover: {:?}", m2.value);
    shutdown(&mut service).await;
}

#[tokio::test]
async fn state_at_on_the_by_keyword_returns_the_root_goal() {
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, BY_OPEN).await;
    let _ = wait_diagnostics(&mut socket, "stateAt diagnostics").await;

    let result = ask_state_at(&mut service, BY_OPEN, offset_of(BY_OPEN, "by intro")).await;
    assert_eq!(result["step"], -1, "before the first tactic = root state");
    assert_eq!(result["total"], 2);
    let goal = result["goal"].as_str().expect("root goal is the full type");
    // **线 C（T-C20）之后**：根状态也要过记法折叠 ⇒ `And a a -> a` 变成
    // `a ∧ a -> a`。这条断言因此从"含 `And`"改成"含 `∧`"——用户看的就是它
    // （T-C01 实测：学习者的光标在 tactic 上，看到的是根状态），而"goal 里没有
    // 记法"正是用户报的那条。
    assert!(
        goal.contains('∧'),
        "root goal is the declared type, notation-folded: {goal}"
    );
    assert!(result["binders"]
        .as_array()
        .expect("binders array")
        .is_empty());
    shutdown(&mut service).await;
}

#[tokio::test]
async fn state_at_without_by_steps_returns_the_declaration_goal() {
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, EXERCISE).await;
    let _ = wait_diagnostics(&mut socket, "stateAt diagnostics").await;

    let result = ask_state_at(&mut service, EXERCISE, offset_of(EXERCISE, "sorry")).await;
    assert_eq!(result["decl"]["status"], "open");
    // **A1（2026-09-26）**：无 `by` 的开放练习，wire 的 `goal` 是**显示副本**
    // ⇒ 必须过唯一接口的折叠（`->` ⇒ `→`）。以前这里是 `"Prop -> Prop"` ✗
    // —— 那正是用户看到的「Infoview 顶部 `⊢` 后面没记法化」。
    assert_eq!(result["goal"], "Prop → Prop");
    // **不变量**：`goal_runs` 的文本拼接**逐字节等于** `goal`（同一次转化 ✓）。
    let joined: String = result["goal_runs"]
        .as_array()
        .expect("goal_runs")
        .iter()
        .map(|r| r["text"].as_str().unwrap_or_default())
        .collect();
    assert_eq!(joined, result["goal"].as_str().unwrap_or_default());
    assert_eq!(result["step"], -1);
    assert_eq!(result["total"], 0);
    shutdown(&mut service).await;
}

#[tokio::test]
async fn state_at_outside_any_declaration_is_empty() {
    let src = "-- 讲解注释\naxiom True : Prop\n";
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, src).await;
    let _ = wait_diagnostics(&mut socket, "stateAt diagnostics").await;

    let result = ask_state_at(&mut service, src, 0).await;
    assert!(result["decl"].is_null(), "no declaration at the cursor");
    assert!(result["goal"].is_null());
    assert!(result["span"].is_null());
    assert_eq!(result["step"], -1);
    shutdown(&mut service).await;
}

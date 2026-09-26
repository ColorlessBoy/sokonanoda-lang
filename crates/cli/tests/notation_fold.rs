//! 线 C（记法折叠）的**判别性**端到端：`SOKO_NO_NOTATION_FOLD=1` 关掉折叠之后，
//! 四个 surface 各自**必须**怎么变（T-C24 的机械判据，设计
//! `docs/design/vscode-editor-feedback-plan.md` §T-C24）。
//!
//! 为什么这条测试不能只写成单测：`crates/front/src/display.rs` 那条用的是
//! `DisplayNotations::default()`——它证明的是**折叠层**在空表下原样返回，而
//! **碰不到开关本身**（`display_notations` 里那个 `if`）。开关要是被谁删了、
//! 或者哪条路绕过了 `display_notations`，那条单测照样绿。这里跑**真二进制**，
//! 所以覆盖的是"从环境变量到 wire 字段"的整条路——与 `judge_batch.rs` 对
//! `SOKO_NO_JUDGE_BATCH` 的做法同构。
//!
//! 三个断言组，对应设计里那张表：
//!
//! | # | 断言 | 覆盖的 surface |
//! |---|---|---|
//! | 1 | 关掉 ⇒ **回到点名** | 生产者 1 根状态 · 生产者 3 声明 `ty` · 生产者 4 `by` 步进 |
//! | 2 | 关掉 ⇒ **一个字节不变** | 生产者 2 无 `by` 的开练习 · 假设行 `binders[].ty`（源级渲染） |
//! | 3 | 关掉 ⇒ `grade --json` **逐字节相同** | 判定（折叠只许动展示副本） |
//!
//! 第 2 组是**守护**而不是折叠的判据：那两个 surface 的记法来自 `render_expr`
//! 的源级渲染，不经过折叠。它们同时是**行程开关**——谁把生产者 2 改成走内核 pp
//! 折叠，第 2 组会红，逼他回来更新设计里那张表。

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::Value;

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// 画布：一条 `⊆` / 一条 `∈` / 一个内建 `↔`，四个 surface 各来一处。
///
/// * `ext_pattern` 带 `by` —— 根状态（生产者 1）、`ty`（生产者 3）、
///   `apply` 之后的步进（生产者 4）都长在它身上；
/// * `open_subset` 不带 `by` —— 生产者 2 的源级渲染；
/// * `h` 的类型写在**源里** —— 假设行（`binders[].ty`）。
const CANVAS: &str = "\
def Set (α : Type) : Type := α -> Prop
def Set.mem (α : Type) (a : α) (A : Set α) : Prop := A a
infix:50 \" ∈ \" => Set.mem
def Set.subset (α : Type) (A B : Set α) : Prop := forall (x : α), A x -> B x
infix:50 \" ⊆ \" => Set.subset
axiom Set.ext (α : Type) (A B : Set α) : (∀ (x : α), x ∈ A ↔ x ∈ B) -> A = B

theorem ext_pattern (α : Type) (A B : Set α) (h : ∀ (x : α), x ∈ A ↔ x ∈ B) : A = B := by
  apply Set.ext
  exact h

theorem open_subset (α : Type) (A B : Set α) : A ⊆ B -> (∀ (a : α), a ∈ A -> a ∈ B) := sorry
";

fn canvas_file(tag: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "sokonanoda-notation-fold-{tag}-{}-{}.sokonanoda",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::write(&path, CANVAS).expect("write canvas");
    path
}

/// 跑一条子命令，返回 `(退出码, stdout)`。`fold` 决定是否带上关掉折叠的开关。
///
/// 缓存目录按开关**分开**（与 `judge_batch.rs` 同理）：否则第二次调用会命中
/// 第一次写下的缓存条目，这条测试就变成在测缓存而不是测开关。
fn run(args: &[&str], fold: bool) -> (i32, String) {
    let cache = std::env::temp_dir().join(format!(
        "sokonanoda-notation-fold-cache-{}-{}-{}",
        if fold { "on" } else { "off" },
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let mut command = Command::new(env!("CARGO_BIN_EXE_sokonanoda"));
    command
        .args(args)
        .env("SOKONANODA_CACHE_DIR", &cache)
        .env("SOKONANODA_NO_CACHE", "1");
    if !fold {
        command.env("SOKO_NO_NOTATION_FOLD", "1");
    }
    let output = command.output().expect("spawn sokonanoda");
    let _ = std::fs::remove_dir_all(&cache);
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).into_owned(),
    )
}

/// 跑一条 `query` 子命令并解析成 JSON（折叠开关由 `fold` 决定）。
fn query_json(args: &[&str], fold: bool) -> (Value, i32) {
    let mut full = vec!["query"];
    full.extend_from_slice(args);
    let (code, text) = run(&full, fold);
    let value: Value = serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("query must print one JSON object ({e}): {text}"));
    (value, code)
}

/// `query goals` 里某个声明的字段。
fn goal_field(path: &Path, name: &str, field: &str, fold: bool) -> String {
    let (value, code) = query_json(
        &["goals", "--file", path.to_str().expect("utf-8 path")],
        fold,
    );
    assert_eq!(code, 0, "query goals must succeed: {value}");
    let decl = value["data"]
        .as_array()
        .expect("data is a list")
        .iter()
        .find(|d| d["name"] == name)
        .unwrap_or_else(|| panic!("`{name}` must be listed: {value}"));
    decl[field]
        .as_str()
        .unwrap_or_else(|| panic!("`{name}.{field}` must be a string: {decl}"))
        .to_string()
}

/// `query state` 在 1 基 `(line, col)` 处（CLI 用 1 基，列按 UTF-16 数）。
fn state_at(path: &Path, line: usize, col: usize, fold: bool) -> Value {
    let (value, code) = query_json(
        &[
            "state",
            "--file",
            path.to_str().expect("utf-8 path"),
            "--line",
            &line.to_string(),
            "--col",
            &col.to_string(),
        ],
        fold,
    );
    assert_eq!(code, 0, "query state must succeed at {line}:{col}: {value}");
    value["data"].clone()
}

/// 行号（1 基）——夹具里 `needle` 所在的那一行。
fn line_of(needle: &str) -> usize {
    CANVAS
        .lines()
        .position(|l| l.contains(needle))
        .map(|i| i + 1)
        .unwrap_or_else(|| panic!("the fixture must contain {needle:?}"))
}

/// 断言：**开着折叠时是记法、关掉之后回到点名**。
///
/// `what` 是失败信息里的 surface 名（"声明 ty" / "根状态" / "`by` 步进"），
/// `point` 是那个点名的头（`Set.subset` / `Iff` …）。
fn assert_pointwise_when_off(what: &str, folded: &str, pointwise: &str, point: &str) {
    assert!(
        !folded.contains(point),
        "{what}：开着折叠时不该出现点名 `{point}`：{folded}"
    );
    assert!(
        pointwise.contains(point),
        "{what}：关掉折叠后必须回到点名 `{point}`：{pointwise}"
    );
    assert_ne!(folded, pointwise, "{what}：开关必须真的改变这段文本");
}

#[test]
fn the_fold_switch_turns_every_foldable_surface_pointwise() {
    let path = canvas_file("foldable");

    // ---- 生产者 3：声明卡片的 `ty` -------------------------------------
    // `ext_pattern` 的类型里有 `↔` / `∈` / `=`（内建记法）——关掉之后是
    // `Iff` / `Set.mem` / `Eq`，正是用户报 G-26 时看到的样子。
    let ty_on = goal_field(&path, "ext_pattern", "ty", true);
    let ty_off = goal_field(&path, "ext_pattern", "ty", false);
    assert!(
        ty_on.contains('↔') && ty_on.contains('∈') && ty_on.contains(" = "),
        "声明 ty 开着折叠时应当带记法：{ty_on}"
    );
    assert!(
        !ty_off.contains('↔') && !ty_off.contains('∈') && !ty_off.contains(" = "),
        "声明 ty 关掉折叠后不该还有记法：{ty_off}"
    );
    for point in ["Iff", "Set.mem", "Eq"] {
        assert!(
            ty_off.contains(point),
            "声明 ty 关掉折叠后应当是点名 `{point}`：{ty_off}"
        );
    }

    // `open_subset` 的 `ty` 用的是自定义记法 ⇒ 点名是 `Set.subset`。
    let subset_ty_on = goal_field(&path, "open_subset", "ty", true);
    let subset_ty_off = goal_field(&path, "open_subset", "ty", false);
    assert!(
        subset_ty_on.contains('⊆') && subset_ty_on.contains('∈'),
        "声明 ty 应当带自定义记法：{subset_ty_on}"
    );
    assert_pointwise_when_off(
        "声明 ty（自定义记法）",
        &subset_ty_on,
        &subset_ty_off,
        "Set.subset",
    );

    // ---- 生产者 1：根状态（第一条 tactic 之前） -------------------------
    let root_on = state_at(&path, line_of("theorem ext_pattern"), 3, true);
    let root_off = state_at(&path, line_of("theorem ext_pattern"), 3, false);
    assert_eq!(root_on["step"], -1, "声明行 = 根状态");
    let root_on = root_on["goal"].as_str().expect("root goal").to_string();
    let root_off = root_off["goal"].as_str().expect("root goal").to_string();
    assert!(
        root_on.contains('↔') && root_on.contains('∈'),
        "根状态开着折叠时应当带记法：{root_on}"
    );
    assert!(
        !root_off.contains('↔') && !root_off.contains('∈') && root_off.contains("Iff"),
        "根状态关掉折叠后应当是点名：{root_off}"
    );
    assert_ne!(root_on, root_off, "开关必须真的改变根状态");

    // ---- 生产者 4：`by` 步进（`apply Set.ext` 之后的子目标） -------------
    // 光标落在 `exact h` 上 = 进入它时的状态 = `apply` 出来的那个子目标。
    let step_on = state_at(&path, line_of("exact h"), 3, true);
    let step_off = state_at(&path, line_of("exact h"), 3, false);
    let step_on = step_on["goal"].as_str().expect("by-step goal").to_string();
    let step_off = step_off["goal"].as_str().expect("by-step goal").to_string();
    assert!(
        step_on.contains('↔') && step_on.contains('∈'),
        "`by` 步进开着折叠时应当带记法：{step_on}"
    );
    assert!(
        !step_off.contains('↔') && !step_off.contains('∈') && step_off.contains("Iff"),
        "`by` 步进关掉折叠后应当是点名：{step_off}"
    );
    assert_ne!(step_on, step_off, "开关必须真的改变 `by` 步进");

    let _ = std::fs::remove_file(&path);
}

#[test]
fn source_rendered_surfaces_ignore_the_fold_switch() {
    let path = canvas_file("source-rendered");

    // ---- 生产者 2：不带 `by` 的开练习 ----------------------------------
    // 这条 `goal` 走 `render_expr` 的**源级渲染**（T-C01 实测），记法来自源文本
    // 本身。
    //
    // **判据在 2026-09-26（A1/T-N5）收紧了，不是放松了**：以前断言的是
    // 「开关对它**逐字节无影响**」✗ —— 那一条与用户报告第 1 条冲突：wire 的
    // `goal` 是**显示副本**，必须过唯一接口（`->` 是 `→` 的**词法别名**，
    // 显示面统一打 `→`）。⇒ 现在断言的是**更强的**逐项判据：
    //   **开关两态之间唯一的差别只能是箭头别名归一化**（把 `->` 换成 `→`），
    //   **不许增删任何记法符号**。这样既保住了老守卫要保的东西（记法来自源、
    //   不因折叠而增删），又钉住了新增的那一处归一化。
    let open_on = goal_field(&path, "open_subset", "goal", true);
    let open_off = goal_field(&path, "open_subset", "goal", false);
    assert_eq!(
        open_on,
        open_off.replace("->", "→"),
        "开关两态只许差箭头别名（`->` ⇒ `→`），不许动别的：\n开 {open_on}\n关 {open_off}"
    );
    assert_eq!(
        open_on.matches('⊆').count(),
        open_off.matches('⊆').count(),
        "记法不许因折叠而增减：\n开 {open_on}\n关 {open_off}"
    );
    assert!(
        open_on.contains('⊆') && open_on.contains('∈'),
        "生产者 2 的 goal 必须带记法：{open_on}"
    );
    assert!(
        open_on.contains('→') && !open_on.contains("->"),
        "显示副本的箭头必须是 `→`（A1）：{open_on}"
    );

    // ---- 假设行：`binders[].ty` ---------------------------------------
    // binder 的类型写在**源里**（`h : ∀ (x : α), x ∈ A ↔ x ∈ B`）⇒ 同样不经过
    // 折叠。T-C23 的守护也在这里从真二进制侧再钉一遍。
    let binder_ty = |fold: bool| -> String {
        let state = state_at(&path, line_of("exact h"), 3, fold);
        state["binders"]
            .as_array()
            .expect("binders")
            .iter()
            .find(|b| b["name"] == "h")
            .unwrap_or_else(|| panic!("`h` must be in scope: {state}"))["ty"]
            .as_str()
            .expect("binder ty")
            .to_string()
    };
    let (binder_on, binder_off) = (binder_ty(true), binder_ty(false));
    assert_eq!(
        binder_on,
        binder_off.replace("->", "→"),
        "开关两态只许差箭头别名（同上）：\n开 {binder_on}\n关 {binder_off}"
    );
    assert!(
        binder_on.contains('∈') && binder_on.contains('↔'),
        "假设行的类型必须带记法：{binder_on}"
    );

    let _ = std::fs::remove_file(&path);
}

/// 洞的期望类型画布（A0 的"立判据"组，2026-09-26 T-N16）。
///
/// 它同时踩到**两条路**：记法符号（`∈`/`↔`）来自**源级渲染**（记法写在源里），
/// 而箭头来自 `Set.ext` 的签名 ⇒ 洞的期望类型是"源级渲染"的正身 ✓。
const HOLE_CANVAS: &str = "\
def Set (α : Type) : Type := α -> Prop
def Set.mem (α : Type) (a : α) (A : Set α) : Prop := A a
infix:50 \" ∈ \" => Set.mem
def Set.subset (α : Type) (A B : Set α) : Prop := forall (x : α), A x -> B x
infix:50 \" ⊆ \" => Set.subset
axiom Set.ext (α : Type) (A B : Set α) : (∀ (x : α), x ∈ A ↔ x ∈ B) -> A = B

theorem hole_surface (α : Type) (A B : Set α) : A ⊆ B -> A = B :=
  Set.ext α A B sorry
";

/// **A0 的最后一格**：洞的期望类型（`sub_goals[].ty`）。
///
/// 这 9 处的产物分两路走：
/// * **显示副本**（wire 的 `SubGoalInfo.ty` + LSP hover 的「此处 `sorry` 的期望
///   类型」）⇒ **必须过唯一接口** ✓ —— 它以前是 `sub.ty.clone()` 的**直通**
///   （用户看到 `(x : α) -> …` 这种混合形态 ✗），2026-09-26 改成折一份克隆 ✓；
/// * **真相层**（`DeclState.sub_goals[].ty`）⇒ **一个字节都不许折** ✗ ——
///   `suggest.rs::hole_goal_text` 把它当 `OpenGoalSpec.ty` **回读**去算
///   exact/rfl 建议，折了就是**改判定**（内核红线）。
///
/// 本判据钉住第一路（开关两态必须不同、关掉必须回到点形式）；第二路由
/// `crates/front/src/compile/tests.rs::hole_expected_type_is_judge_input_and_stays_raw`
/// 直接钉真相字段本身 ✓（比"`grade --json` 逐字节相同"更直接 —— 后者在
/// 建议是**按需**算的前提下碰不到这个字段）。
#[test]
fn hole_expected_types_fold_on_the_display_copy() {
    let path = std::env::temp_dir().join(format!(
        "sokonanoda-hole-surface-{}-{}.sokonanoda",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::write(&path, HOLE_CANVAS).expect("write hole canvas");
    let holes_ty = |fold: bool| -> String {
        let (value, code) = query_json(
            &["holes", "--file", path.to_str().expect("utf-8 path")],
            fold,
        );
        assert_eq!(code, 0, "query holes must succeed: {value}");
        value["data"]["holes"][0]["ty"]
            .as_str()
            .unwrap_or_else(|| panic!("洞必须有期望类型：{value}"))
            .to_string()
    };
    let (on, off) = (holes_ty(true), holes_ty(false));
    assert!(
        on.contains('→') && !on.contains("->"),
        "洞的期望类型是**显示副本** ⇒ 箭头必须是 `→`（A1 的别名归一化）：{on}"
    );
    assert!(
        off.contains("->") && !off.contains('→'),
        "关掉折叠后必须回到 ASCII 点形式：{off}"
    );
    assert!(
        on.contains('∈') && on.contains('↔') && off.contains('∈') && off.contains('↔'),
        "记法符号来自**源文本**，开关不该动它们：\n开 {on}\n关 {off}"
    );
    assert_ne!(
        on, off,
        "开关必须真的改变洞的期望类型（以前是直通 ⇒ 两态相同）"
    );
    // 判定侧：`grade --json` 两态逐字节相同（与"最重要的一条"同口径）。
    let (code_on, grade_on) = run(
        &["grade", "--json", path.to_str().expect("utf-8 path")],
        true,
    );
    let (code_off, grade_off) = run(
        &["grade", "--json", path.to_str().expect("utf-8 path")],
        false,
    );
    assert_eq!(code_on, code_off, "开关不该改判卷退出码");
    assert_eq!(
        grade_on, grade_off,
        "折叠只许动展示副本：判卷事件流必须逐字节相同"
    );
    let _ = std::fs::remove_file(&path);
}

/// **最重要的一条**：开关只许动**展示副本**，判定必须一个字节不变。
///
/// 折叠误伤判定输入的后果是静默的——`apply` 的子目标回读会失败、声明变红，
/// 或者更糟：判定"照过"但依据的是被改写过的文本。所以这里不是"计数一致"而是
/// **`--json` 事件流逐字节相同**（与 `judge_batch.rs` 同一把尺子）。
#[test]
fn the_fold_switch_never_changes_the_judge() {
    let path = canvas_file("judge");
    let path = path.to_str().expect("utf-8 path");
    let (code_on, out_on) = run(&["grade", "--json", path], true);
    let (code_off, out_off) = run(&["grade", "--json", path], false);
    assert_eq!(
        code_on, code_off,
        "退出码在折叠开关下不同（{code_on} vs {code_off}）"
    );
    assert_eq!(
        out_on, out_off,
        "事件流在折叠开关下不同——折叠误伤了判定：\n--- 开 ---\n{out_on}\n--- 关 ---\n{out_off}"
    );
    assert_eq!(code_on, 0, "夹具应当判绿：{out_on}");
    let _ = std::fs::remove_file(path);
}

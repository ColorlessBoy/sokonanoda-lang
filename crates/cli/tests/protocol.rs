//! Protocol-conformance tests for `--json` (docs/protocol.md): the JSON Lines
//! event stream is a machine contract, so every batch run must use the closed
//! event vocabulary with stable payload fields.

use std::io::Write;
use std::process::{Command, Output, Stdio};

use serde_json::Value;

use common::EVENT_VOCABULARY as VOCABULARY;

mod common;

fn spawn_json(args: &[&str]) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_sokonanoda"));
    cmd.arg("--json").args(args);
    cmd
}

fn run_json_stdin(input: &str) -> (Output, Vec<Value>) {
    run_json_stdin_env(input, &[])
}

/// 同 [`run_json_stdin`]，另注入环境变量（**先摘掉继承来的 `SOKO_STRICT_AT`** ⇒
/// 档位由用例自己定，不随开发机/CI 的外部环境漂移 ✓）。
fn run_json_stdin_env(input: &str, envs: &[(&str, &str)]) -> (Output, Vec<Value>) {
    let mut cmd = spawn_json(&[]);
    cmd.env_remove("SOKO_STRICT_AT");
    for (k, v) in envs {
        cmd.env(k, v);
    }
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn sokonanoda --json");
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(input.as_bytes())
        .expect("write stdin");
    let out = child.wait_with_output().expect("wait");
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    (out, parse_event_stream(&stdout))
}

fn run_json_file(path: &str) -> (Output, Vec<Value>) {
    let out = spawn_json(&[path])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run sokonanoda --json on file");
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    (out, parse_event_stream(&stdout))
}

fn parse_event_stream(stdout: &str) -> Vec<Value> {
    stdout
        .lines()
        .enumerate()
        .map(|(i, line)| {
            serde_json::from_str(line).unwrap_or_else(|e| {
                panic!(
                    "--json stdout line {} is not valid JSON ({e}):\n  raw: {line:?}\n  full stdout:\n{stdout}",
                    i + 1
                )
            })
        })
        .collect()
}

fn event_type(event: &Value) -> &str {
    event.get("type").and_then(|v| v.as_str()).unwrap_or("")
}

fn non_empty_str<'a>(event: &'a Value, field: &str, context: &str) -> &'a str {
    event
        .get(field)
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| panic!("{context}: event missing non-empty {field:?} field: {event}"))
}

fn diagnostics(events: &[Value]) -> Vec<&Value> {
    events
        .iter()
        .filter(|e| event_type(e) == "diagnostic")
        .collect()
}

fn assert_diagnostic_shape(event: &Value, context: &str) {
    let stage = non_empty_str(event, "stage", context);
    assert!(
        matches!(stage, "parse" | "elab" | "kernel"),
        "{context}: diagnostic stage {stage:?} is outside parse/elab/kernel: {event}"
    );
    non_empty_str(event, "code", context);
    non_empty_str(event, "message", context);
    let span = event
        .get("span")
        .unwrap_or_else(|| panic!("{context}: diagnostic missing span: {event}"));
    for side in ["start", "end"] {
        let point = span
            .get(side)
            .unwrap_or_else(|| panic!("{context}: diagnostic span missing {side:?}: {event}"));
        for field in ["line", "column", "offset"] {
            assert!(
                point.get(field).and_then(|v| v.as_u64()).is_some(),
                "{context}: diagnostic span.{side}.{field} missing or not an integer: {event}"
            );
        }
    }
}

#[test]
fn every_example_stream_is_closed_vocabulary() {
    let examples = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples");
    let mut files: Vec<_> = std::fs::read_dir(examples)
        .expect("read examples dir")
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map(|ext| ext == "sokonanoda")
                .unwrap_or(false)
        })
        .collect();
    files.sort_by_key(|e| e.file_name());
    assert!(!files.is_empty(), "no .sokonanoda files under {examples}");

    for file in files {
        let path = file.path();
        let name = file.file_name().to_string_lossy().into_owned();
        let (out, events) = run_json_file(path.to_str().expect("utf-8 path"));
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            out.status.success(),
            "lesson {name} failed under --json:\nstderr:\n{stderr}"
        );

        for (i, event) in events.iter().enumerate() {
            let context = format!("lesson {name} event {}", i + 1);
            let t = event_type(event);
            assert!(
                VOCABULARY.contains(&t),
                "{context}: type {t:?} outside the closed vocabulary: {event}"
            );
            non_empty_str(event, "human", &context);
            if t == "diagnostic" {
                assert_diagnostic_shape(event, &context);
            }
        }
    }
}

#[test]
fn lesson01_golden_event_sequence() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/lesson-01.sokonanoda"
    );
    let (out, events) = run_json_file(path);
    assert!(
        out.status.success(),
        "lesson-01 must pass the kernel, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let types: Vec<&str> = events.iter().map(event_type).collect();
    assert_eq!(
        types,
        ["decl.checked", "expr.typed", "exercise.open"],
        "lesson-01 emits exactly this ordered event sequence"
    );

    assert_eq!(events[0]["name"], "id", "decl.checked carries the name");
    assert_eq!(
        events[1]["inferred_type"], "Prop -> Prop",
        "expr.typed carries the inferred type"
    );
    assert_eq!(
        events[1]["text"], "id",
        "expr.typed text is the exact source slice"
    );
}

#[test]
fn lesson02_golden_subsequence() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/lesson-02.sokonanoda"
    );
    let (out, events) = run_json_file(path);
    assert!(
        out.status.success(),
        "lesson-02 must pass the kernel, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let types: Vec<&str> = events.iter().map(event_type).collect();
    for expected in [
        "decl.checked",
        "expr.typed",
        "expr.reduced",
        "exercise.open",
    ] {
        assert!(
            types.contains(&expected),
            "lesson-02 stream must contain {expected:?}; got {types:?}"
        );
    }

    assert!(
        events
            .iter()
            .any(|e| event_type(e) == "decl.checked" && e["name"] == "two"),
        "decl.checked {{name: two}} missing: {events:?}"
    );
    assert!(
        events
            .iter()
            .any(|e| event_type(e) == "expr.typed" && e["inferred_type"] == "Nat"),
        "expr.typed {{inferred_type: Nat}} missing: {events:?}"
    );
    assert!(
        events
            .iter()
            .any(|e| event_type(e) == "expr.reduced" && e["value"] == "5"),
        "expr.reduced {{value: 5}} missing (2 + 3 must reduce to 5): {events:?}"
    );
}

#[test]
fn kernel_rejection_diagnostic_shape() {
    let (out, events) = run_json_stdin("def bad : Prop -> Type := fun (x : Prop) => x\n");
    assert!(!out.status.success(), "kernel rejection must exit non-zero");

    let diags = diagnostics(&events);
    assert_eq!(
        diags.len(),
        1,
        "exactly one diagnostic for a kernel rejection: {events:?}"
    );
    let d = diags[0];
    assert_eq!(d["stage"], "kernel", "rejection stages as kernel: {d}");
    assert_eq!(d["code"], "kernel-rejected");
    assert_eq!(
        d["span"]["start"]["line"], 1,
        "rejection span starts on the offending line: {d}"
    );
    assert!(
        d.get("hint")
            .and_then(|h| h.as_str())
            .is_none_or(|s| !s.is_empty()),
        "kernel diagnostics carry a teaching hint per docs/protocol.md: {d}"
    );
    assert_diagnostic_shape(d, "kernel rejection");
}

#[test]
fn parse_error_diagnostic_shape() {
    let (out, events) = run_json_stdin("def broken : Prop :=\n");
    assert!(!out.status.success(), "parse error must exit non-zero");

    let diags = diagnostics(&events);
    assert_eq!(
        diags.len(),
        1,
        "exactly one diagnostic for a parse error: {events:?}"
    );
    let d = diags[0];
    assert_eq!(d["stage"], "parse");
    assert!(
        matches!(
            d["code"].as_str(),
            Some("unexpected-token") | Some("unexpected-eof")
        ),
        "parse code must be a documented parse-stage code: {d}"
    );
    assert_eq!(
        d["span"]["start"]["line"], 2,
        "the parse error points at the line where input ran out: {d}"
    );
    assert_diagnostic_shape(d, "parse error");
    assert!(
        d.get("hint")
            .and_then(|v| v.as_str())
            .map(|h| !h.is_empty())
            .unwrap_or(false),
        "parse diagnostics carry a hint per docs/protocol.md: {d}"
    );
}

/// **a1（2026-10-17）：诊断必须带得到源位置** —— `line=0 col=0` 是"谁都定位不了" ✗。
///
/// **为什么既有守卫咬不住它** ✗：[`assert_diagnostic_shape`] 只断言 `line`/`column`/
/// `offset` **是整数**（`Span::default()` 全 0 也满足 ✗）⇒ 严格档下
/// `courses/set-theory/lib/Set.sokonanoda` **13 条诊断里 8 条**报 `line=0` 而全绿 ✗。
///
/// **真因**：`by` 引擎把 tactic 装配成项时，`NodeKind::Apply` 产出的 `Expr::App`
/// 一律 `Span::default()` ⇒ 之后 `elab_expr` 在它上面报的每条诊断都没有位置。
///
/// **形状**（a1 当时的夹具，逐字）：目标头是一个 **def**（`MySet.mem`）⇒ `constructor`
/// 展开到 `And`/`Or` 的构造子，那条**装配出来**的应用才踩到这个洞 ✓
/// （直接写 `A ↔ A ∧ B` 时 `Iff` 走 `builtin_constructor_of`，装配路径不同 ⇒ 不踩 ✓）。
///
/// **Lean 4 对照**（`Init/Prelude.lean:4783`）：`SourceInfo.synthetic pos endPos`
/// ——「synthetic syntax is annotated with **a source span from the original syntax**」，
/// 元程序产出的语法**照样带源位置**，不是零位置 ✓。
///
/// **⚠ 2026-10-18（a7）：原夹具的"前提"过期了** —— 它靠**严格档拒绝**来逼出那条装配
/// 出来的应用，而那个拒绝**本身就是 bug**：`a4`（`assemble` 的装配脊标 `@`）+
/// `a5`（`derive_recursor` / pp 回读同族）修好之后，**同一个夹具现在编得过**
/// （4 条 `decl.checked`、0 条诊断）⇒ 旧的 `!out.status.success()` 不成立 ⇒ 用例判红 ✗。
/// **那是"代码变好了"，不是回归** ✓ —— 三条证据：① CI 日志里它**编过**且 4 条
/// `decl.checked`；② 把 `a4` 的 `*explicit = true` 还原 ⇒ 那条诊断**原样回来**
/// （`And.intro`、4 个位置实参 vs 2 个显式层、span `10:3` = `constructor` 那一行
/// —— 正是原用例断言的位置 ✓）；③ 用户判据本身是「严格档下课程标准库**全部编过**」
/// ⇒ 编过**就是**目标 ✓。（⚠ `docs/CI-FAILURES.md` 的台账条目**本轮未追加** ——
/// 本轮边界是"只改测试"，已如实记进 `.cache/ux/fix-a7/VERDICT.md` §8 ✓。）
///
/// ⇒ 夹具拆成两半，**两半都不许删** ✗：
/// * **①** 原夹具（`MySet`）⇒ 严格档下**必须编过** —— 它记着 a4/a5 修好的那件事；
/// * **②** 新夹具（`exfalso`）⇒ **本用例的牙**：同一道严格闸门在**另一条装配出来的项**
///   （`assemble` 造出 `False.elim <目标> <证明>`）上仍然判红 ⇒ 那条诊断的 span 取自
///   `assemble` 产出的 `Expr::App` ⇒ **必须是真位置** ✓。
///   （`exfalso` 与 `constructor` 走的是同一支 `NodeKind::Apply`，只是它的脊**没被**标 `@`
///   ⇒ 仍然踩闸 ✓ —— 如实记下：这条判红是**同族的遗留**，`constructor` 那半已修 ✓。）
///
/// **反向验证**：把 `by.rs` 的 `assemble` 还原成 `Span::default()` ⇒ ② 的位置当场从
/// `2:3` 变回 `line=0 col=0` ⇒ 本用例判红 ✓（2026-10-18 实测 ✓）。
#[test]
fn synthesized_tactic_terms_keep_a_real_source_position() {
    // ① 前提夹具（a1 原样）：严格档下**必须编过** —— `constructor` 展开到 `And`/`Or`
    //    的构造子，那条装配出来的应用以前被严格档误判成"用户写了位置隐式实参" ✗。
    let accepted = "\
def MySet (α : Type) : Type := α → Prop

namespace MySet
def mem {α : Type} (a : α) (A : MySet α) : Prop := A a
def union {α : Type} (A B : MySet α) : MySet α := fun (x : α) => A x ∨ B x
end MySet

theorem t {α : Type} (a : α) (A B : MySet α) :
    MySet.mem a (MySet.union A B) ↔ MySet.mem a A ∨ MySet.mem a B := by
  constructor
  intro h
  exact h
  intro h
  exact h
";
    let (out, events) = run_json_stdin_env(accepted, &[("SOKO_STRICT_AT", "1")]);
    assert!(
        out.status.success(),
        "严格档必须接受这条（装配脊是 `@` 形态，不是用户写的位置隐式实参 —— a4/a5）：{events:?}"
    );
    assert_eq!(
        diagnostics(&events).len(),
        0,
        "编过就不该有诊断：{events:?}"
    );
    assert!(
        events
            .iter()
            .any(|e| event_type(e) == "decl.checked" && e["name"] == "t"),
        "严格档接受 ⇒ `t` 必须真的被判定过（`decl.checked`，不是静默跳过）：{events:?}"
    );

    // ② 牙：同族另一条**仍踩**严格闸门的项 ⇒ 它的诊断必须带**真**位置。
    //
    // ⚠ **2026-10-18（接口 agent · 在 a7 的 ② 之上再修一次）**：a7 当时把 ② 的夹具
    //   选成了 `exfalso` + `exact h`（`h : False`），理由是「`exfalso` 与 `constructor`
    //   走同一支 `NodeKind::Apply`，只是它的脊**没被**标 `@`」。**那条理由过期了** ✗：
    //   a4（`assemble` 的装配脊标 `@`）+ a5（`derive_recursor` / pp 回读同族）修好之后，
    //   `exfalso` 那条**也**编得过 —— 而它**类型上本来就对**（`exfalso` 把目标 `P` 变成
    //   `False`，`h : False` ⇒ `exact h` 成立 ✓）⇒ 实测 **`decl.checked` / 0 诊断** ✗
    //   ⇒ 旧的 `!out.status.success()` 不成立 ⇒ 本用例在 CI 上判红 ✗（10 条红之一）。
    //   ⇒ 换成**用户手写的**位置喂隐式（`And.intro P Q h1 h2`）：它**确实**踩闸门 ✓
    //   （row 7 之后实测根因码 `elab-implicit-argument-unsolved` · `line=2 col=3` ✓），
    //   而**本用例的牙一个字没松** ——
    //   仍然是「诊断位置必须是**真的**（不是 `Span::default()` 那串 0 ✗）」✓。
    //   ⚠ **如实**：夹具从「装配出来的项」换成「用户手写的项」⇒ **"装配路径"这一半的覆盖
    //   暂时没有对应用例** ✗（要找一条"既装配出来、又仍踩闸门"的夹具 —— 本轮没找到）。
    let rejected = "\
theorem t (P Q : Prop) (h1 : P) (h2 : Q) : P ∧ Q := by
  exact And.intro P Q h1 h2
";
    let (out, events) = run_json_stdin_env(rejected, &[("SOKO_STRICT_AT", "1")]);
    assert!(!out.status.success(), "严格档下这条必须判红：{events:?}");

    let diags = diagnostics(&events);
    assert_eq!(diags.len(), 1, "恰好一条诊断：{events:?}");
    let d = diags[0];
    assert_eq!(d["code"], "elab-implicit-argument-unsolved", "{d}");
    assert_diagnostic_shape(d, "严格档拒绝");
    // ⭐ 本用例的牙：位置必须是**真的**（不是 `Span::default()` 那串 0 ✗）。
    let start = &d["span"]["start"];
    assert_ne!(
        (start["line"].as_u64(), start["column"].as_u64()),
        (Some(0), Some(0)),
        "诊断位置丢了（`line=0 col=0`）—— 用户定位不了 ✗：{d}"
    );
    assert_eq!(
        start["line"], 2,
        "诊断必须落在 `exact` 那一行（1-based 2 = 源第 2 行）：{d}"
    );
    assert_eq!(
        start["column"], 3,
        "诊断必须落在 `exact` 那一列（1-based 3 = `e`）：{d}"
    );
    // span 的**终点**也要在同一个记号里 ⇒ 钉住"整条记号的 span"，
    // 而不只是"起点不是 0"（零宽 span 也算位置真 ✗）。
    // ⚠ 终点随 ② 的夹具一起换了（`exact And.intro P Q h1 h2` 那一行 = 2 空格 + 26 字符 ⇒ 28）。
    assert_eq!(
        (
            d["span"]["end"]["line"].as_u64(),
            d["span"]["end"]["column"].as_u64()
        ),
        (Some(2), Some(28)),
        "诊断 span 的终点必须收在 `exact And.intro P Q h1 h2` 记号之后（1-based 2:28）：{d}"
    );
}

/// 记法家族的新诊断码（G-04 / WO-011）：都落在 `parse` / `elab` 分级里，
/// `ok:false` 与退出码语义**不变**（`docs/protocol.md` 的 error staging）。
#[test]
fn notation_diagnostics_stage_as_parse_and_elab() {
    // parse 家族三码：未闭合字符串 / 命令形状 / 未声明符号。
    let parse_cases = [
        ("infix:50 \" ∈ => mem\n", "unterminated-string"),
        ("infix \" ∈ \" => mem\n", "notation-shape"),
        ("def p : Prop := a ∈ b\n", "notation-unknown-symbol"),
    ];
    for (text, expected) in parse_cases {
        let (out, events) = run_json_stdin(text);
        assert!(!out.status.success(), "{expected} must be a rejection");
        let diags = diagnostics(&events);
        assert_eq!(diags.len(), 1, "{expected}: {events:?}");
        assert_eq!(diags[0]["stage"], "parse", "{expected}: {:?}", diags[0]);
        assert_eq!(diags[0]["code"], expected, "{:?}", diags[0]);
        assert_diagnostic_shape(diags[0], expected);
        assert!(
            !non_empty_str(diags[0], "hint", expected).is_empty(),
            "{expected} carries a teaching hint"
        );
    }
    // elab 家族两码：未知目标 / 补不出类型参数。
    //
    // as-built 边界（记在这里，不当成没发生）：记号路径先解操作数、再补类型
    // 参数，所以**操作数本身**是未定义标识符时报的是既有的
    // `elab-unknown-identifier`（那条诊断更准确）——`...-argument-unsolved`
    // 留给"操作数都合法、但类型参数补不出来"的形状（例如 `#check ∅`）。
    let elab_cases = [
        (
            "def mem : Prop -> Prop -> Prop := fun (a : Prop) => fun (b : Prop) => a\n\
             infix:50 \" ∈ \" => mem\n\
             def p : Prop := a ∈ b\n",
            "elab-unknown-identifier",
        ),
        (
            "def mem : Prop -> Prop -> Prop := fun (a : Prop) => fun (b : Prop) => a\n\
             infix:50 \" ∈ \" => men\n\
             def p (a : Prop) (b : Prop) : Prop := a ∈ b\n",
            "elab-notation-unknown-target",
        ),
    ];
    for (text, expected) in elab_cases {
        let (out, events) = run_json_stdin(text);
        assert!(!out.status.success(), "{expected} must be a rejection");
        let diags = diagnostics(&events);
        assert_eq!(diags.len(), 1, "{expected}: {events:?}");
        assert_eq!(diags[0]["stage"], "elab", "{expected}: {:?}", diags[0]);
        assert_eq!(diags[0]["code"], expected, "{:?}", diags[0]);
        assert_diagnostic_shape(diags[0], expected);
    }
}

#[test]
fn open_exercise_is_success_state() {
    let (out, events) = run_json_stdin("example : Prop -> Prop := sorry\n");
    assert_eq!(
        out.status.code(),
        Some(0),
        "an open exercise is a successful state, not an error; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        events.iter().any(|e| event_type(e) == "exercise.open"),
        "exercise.open must be emitted for an unfilled sorry: {events:?}"
    );
    assert!(
        diagnostics(&events).is_empty(),
        "an open exercise must not produce diagnostics: {events:?}"
    );
}

/// G-01 / WO-004：开练习的**签名**也受内核检查（与值位错误同罪）。
///
/// 与上面 `open_exercise_is_success_state` 构成**一组通过/失败边界对照**：
/// 合法签名（`Prop -> Prop` 做 `example`）⇒ exit 0 + `exercise.open` + 无诊断；
/// 坏签名 ⇒ exit 1 + 一条诊断 + **没有** `exercise.open`。以后谁把两边一起
/// 放松（比如又把签名错误吞掉），这条对照就会红。
#[test]
fn open_exercise_with_a_bad_signature_is_a_diagnostic() {
    // (源文本, 诊断 code, stage, 签名在源文本里的切片)
    let cases = [
        (
            "theorem t : 3 := sorry\n",
            "kernel-expected-sort",
            "kernel",
            "3",
        ),
        (
            "theorem t : Nat := sorry\n",
            "kernel-theorem-not-prop",
            "kernel",
            "Nat",
        ),
        (
            "theorem t : Bogus := sorry\n",
            "elab-unknown-identifier",
            "elab",
            "Bogus",
        ),
        (
            "def d : 3 := sorry\n",
            "kernel-expected-sort",
            "kernel",
            "3",
        ),
        (
            "example : 3 := sorry\n",
            "kernel-expected-sort",
            "kernel",
            "3",
        ),
        // `by` 路径与直接值位共用同一处签名检查。
        (
            "theorem t : 3 := by sorry\n",
            "kernel-expected-sort",
            "kernel",
            "3",
        ),
    ];
    for (src, code, stage, sig) in cases {
        let (out, events) = run_json_stdin(src);
        assert_eq!(
            out.status.code(),
            Some(1),
            "{src:?} must fail (a bad signature is not an open exercise); stderr: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let diags = diagnostics(&events);
        assert_eq!(
            diags.len(),
            1,
            "{src:?}: exactly one diagnostic: {events:?}"
        );
        assert_eq!(diags[0]["code"], code, "{src:?}: {events:?}");
        assert_eq!(diags[0]["stage"], stage, "{src:?}");
        // 诊断 span 落在**签名**上（G-01：签名受检，报在签名自身的范围）。
        let start = diags[0]["span"]["start"]["offset"].as_u64().unwrap() as usize;
        let end = diags[0]["span"]["end"]["offset"].as_u64().unwrap() as usize;
        assert_eq!(
            &src[start..end],
            sig,
            "{src:?}: span must cover the signature"
        );
        assert!(
            !events.iter().any(|e| event_type(e) == "exercise.open"),
            "{src:?}: a rejected signature must not open an exercise: {events:?}"
        );
    }
}

#[test]
fn reserved_declaration_name_warns_but_stays_successful() {
    let (out, events) = run_json_stdin("axiom Prop : Sort 1\n");
    assert!(
        out.status.success(),
        "a warning must not fail the run; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let warnings: Vec<&Value> = events
        .iter()
        .filter(|e| event_type(e) == "warning")
        .collect();
    assert_eq!(warnings.len(), 1, "exactly one warning: {events:?}");
    let w = warnings[0];
    assert_eq!(w["code"], "reserved-declaration-name");
    non_empty_str(w, "message", "warning");
    non_empty_str(w, "hint", "warning");
    assert_eq!(
        w["span"]["start"]["line"], 1,
        "warning points at the declaration line: {w}"
    );
    assert!(
        events
            .iter()
            .any(|e| event_type(e) == "decl.checked" && e["name"] == "Prop"),
        "the declaration itself still passes the kernel: {events:?}"
    );
}

#[test]
fn redundant_sorry_warns_and_the_declaration_stays_open() {
    // 用户实测形状（`playground.sokonanoda` 326–328）：答案写全了、只多留一行
    // `sorry`（`docs/design/redundant-sorry.md`）。
    let src = "axiom A : Prop\n\
               axiom B : Prop\n\
               axiom f : A -> B\n\
               theorem t (h : A) : B := f h\n\
               \x20 sorry\n";
    let (out, events) = run_json_stdin(src);
    assert!(
        out.status.success(),
        "a warning must not fail the run; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let warnings: Vec<&Value> = events
        .iter()
        .filter(|e| event_type(e) == "warning")
        .collect();
    assert_eq!(warnings.len(), 1, "exactly one warning: {events:?}");
    let w = warnings[0];
    assert_eq!(w["code"], "redundant-sorry");
    non_empty_str(w, "message", "warning");
    non_empty_str(w, "hint", "warning");
    let start = w["span"]["start"]["offset"].as_u64().expect("start offset") as usize;
    let end = w["span"]["end"]["offset"].as_u64().expect("end offset") as usize;
    assert_eq!(
        &src[start..end],
        "sorry",
        "warning span narrows to the redundant `sorry` token: {w}"
    );
    // 语义不变：它仍然是一条**开放练习**（不是 error，也不改 exercise.open）。
    assert!(
        events
            .iter()
            .any(|e| event_type(e) == "exercise.open" && e["name"] == "t"),
        "exercise.open semantics must not change: {events:?}"
    );
    assert!(
        diagnostics(&events).is_empty(),
        "a redundant sorry is a warning, not a diagnostic: {events:?}"
    );

    // 删掉那一行 → 完整通过内核，零 warning。
    let fixed = src.replace(" sorry\n", "");
    let (out, events) = run_json_stdin(&fixed);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        events
            .iter()
            .any(|e| event_type(e) == "decl.checked" && e["name"] == "t"),
        "deleting the leftover line must check the declaration: {events:?}"
    );
    assert!(
        events.iter().all(|e| event_type(e) != "warning"),
        "the fixed file carries no warning: {events:?}"
    );
}

#[test]
fn protocol_document_lists_every_emitted_type() {
    let doc = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/protocol.md"
    ))
    .expect("read docs/protocol.md");
    for t in VOCABULARY {
        assert!(
            doc.contains(t),
            "docs/protocol.md no longer documents event type {t:?}"
        );
    }
}

#[test]
fn elab_unknown_identifier_diagnostic() {
    let (out, events) = run_json_stdin("#check mystery\n");
    assert!(
        !out.status.success(),
        "unknown identifier must exit non-zero"
    );

    let diags = diagnostics(&events);
    assert_eq!(
        diags.len(),
        1,
        "exactly one diagnostic for an unknown identifier: {events:?}"
    );
    let d = diags[0];
    assert_eq!(d["stage"], "elab", "unknown identifier stages as elab: {d}");
    assert_eq!(d["code"], "elab-unknown-identifier");
    assert!(
        d.get("hint")
            .and_then(|h| h.as_str())
            .is_none_or(|s| !s.is_empty()),
        "elab diagnostics carry a teaching hint per docs/protocol.md: {d}"
    );
    assert_diagnostic_shape(d, "elab unknown identifier");
}

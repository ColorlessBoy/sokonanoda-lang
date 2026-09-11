use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

static TEMP_HOME_COUNTER: AtomicU64 = AtomicU64::new(0);

fn run(input: &str) -> std::process::Output {
    run_args(&[], Some(input))
}

fn run_repl(input: &str) -> std::process::Output {
    run_repl_in(&temp_home(), input)
}

fn run_repl_in(home: &Path, input: &str) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_sokonanoda"))
        .args(["repl"])
        .env("HOME", home)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn sokonanoda");
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(input.as_bytes())
        .expect("write stdin");
    child.wait_with_output().expect("wait")
}

fn temp_home() -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sokonanoda-cli-home-{}-{}",
        std::process::id(),
        TEMP_HOME_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp home");
    dir
}

fn run_args(args: &[&str], input: Option<&str>) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_sokonanoda"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn sokonanoda");
    if let Some(input) = input {
        child
            .stdin
            .as_mut()
            .expect("stdin")
            .write_all(input.as_bytes())
            .expect("write stdin");
    }
    child.wait_with_output().expect("wait")
}

#[test]
fn cli_checks_a_valid_file_via_stdin() {
    let out = run("def id : Prop -> Prop := fun (x : Prop) => x\n\
         #check id\n");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("checked declaration id"));
    assert!(stdout.contains("id: Prop -> Prop"));
}

#[test]
fn cli_rejects_a_bad_declaration() {
    let out = run("def bad : Prop -> Type := fun (x : Prop) => x\n");
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("error[kernel-rejected]:"));
}

#[test]
fn cli_checks_by_tactic_blocks() {
    // 第十九轮：`by` 块 —— intro/exact/assumption/apply/rfl 逐 tactic 判定
    // 走 kernel，搭出的证明项与手写项等价。
    let src = concat!(
        "axiom True : Prop\n",
        "axiom True.intro : True\n",
        "axiom And : Prop -> Prop -> Prop\n",
        "axiom And.intro : (a : Prop) -> (b : Prop) -> a -> b -> And a b\n",
        "axiom Or : Prop -> Prop -> Prop\n",
        "axiom Or.inl : (a : Prop) -> (b : Prop) -> a -> Or a b\n",
        "theorem k : (a : Prop) -> a -> a := by intro a; intro h; assumption\n",
        "theorem ai : (a : Prop) -> (b : Prop) -> a -> b -> And a b := by ",
        "intro a; intro b; intro ha; intro hb; apply And.intro; exact ha; exact hb\n",
        "theorem orl : (a : Prop) -> (b : Prop) -> a -> Or a b := by ",
        "intro a; intro b; intro ha; apply Or.inl; exact ha\n",
        "theorem r : Eq.{1} Nat (1 + 1) 2 := by rfl\n",
    );
    let out = run(src);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    for name in ["k", "ai", "orl", "r"] {
        assert!(
            stdout.contains(&format!("checked declaration {name}")),
            "expected {name} to be checked:\n{stdout}"
        );
    }
}

#[test]
fn cli_by_tactic_partial_block_is_open_exercise() {
    // 未写完的 by 块 = 合法 Open 状态（尾部 sorry），与「部分作答」同语义。
    let src = "axiom And : Prop -> Prop -> Prop\n\
               theorem open : (a : Prop) -> And a a -> a := by intro a; intro h\n";
    let out = run(src);
    assert!(
        out.status.success(),
        "partial by must still exit 0:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(String::from_utf8_lossy(&out.stdout).contains("exercise open"));
}

#[test]
fn cli_rejects_uninhabited_dependent_codomain() {
    // conv 快路径 soundness 修复的端到端守护：`(A : Sort 1) -> A` 不可居住，
    // 身份 lambda 的类型是 `(A : Sort 1) -> Sort 1`，必须被拒绝（官方 Lean 同）。
    let out = run("def bad : (A : Sort 1) -> A := fun (A : Sort 1) => A\n");
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("error[kernel-rejected]:"));
}

#[test]
fn cli_accepts_identity_over_sort() {
    // 对照：非依赖的身份函数仍然通过（修复不得过度拒绝）。
    let out = run("def id0 : (A : Sort 1) -> Sort 1 := fun (A : Sort 1) => A\n");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(String::from_utf8_lossy(&out.stdout).contains("checked declaration id0"));
}

#[test]
fn cli_reports_parse_errors_with_positions() {
    let out = run("def broken : Prop :=\n");
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("2:1: error[parse]:"), "stderr: {stderr}");
}

#[test]
fn cli_checks_nat_and_reduces_addition() {
    let out = run("def two : Nat := 1 + 1\n\
         #reduce 1 + 2\n");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("checked declaration two"));
    assert!(stdout.contains("1 + 2 => 3"), "stdout: {stdout}");
}

#[test]
fn cli_prints_definitions() {
    let out = run("def id : Prop -> Prop := fun (x : Prop) => x\n#print id\n");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("#print id"));
    assert!(stdout.contains("def id : Prop -> Prop := fun (x : Prop) => x"));
}

#[test]
fn repl_accumulates_declarations_and_checks_them() {
    let out = run_repl(
        "def id : Prop -> Prop := fun (x : Prop) => x\n\
         #check id\n",
    );
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("checked declaration id"),
        "stdout: {stdout}"
    );
    assert!(stdout.contains("id: Prop -> Prop"), "stdout: {stdout}");
}

#[test]
fn cli_help_is_self_documenting() {
    let out = run_args(&["--help"], None);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("sokonanoda repl"));
    assert!(stdout.contains("#check"));
    assert!(stdout.contains("#print"));
}

#[test]
fn repl_env_and_help_are_available() {
    let out = run_repl(
        "help\n\
         def id : Prop -> Prop := fun (x : Prop) => x\n\
         #env\n",
    );
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("#check <expr>"));
    assert!(stdout.contains("user declarations:"));
    assert!(stdout.contains("  id"));
}

#[test]
fn cli_checks_universe_polymorphic_declarations() {
    let out = run("def id {u} : forall (α : Sort u), α -> α :=\n\
         fun (α : Sort u) => fun (a : α) => a\n\
         def id0 : (α : Prop) -> α -> α :=\n\
         fun (α : Prop) => id.{0} α\n\
         #print id\n");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("checked declaration id"));
    assert!(stdout.contains("checked declaration id0"));
    assert!(stdout.contains("def id.{u}"), "stdout: {stdout}");
    assert!(stdout.contains("Sort u"), "stdout: {stdout}");
}

#[test]
fn cli_rejects_undeclared_universe_variable() {
    let out = run("def bad : forall (α : Sort u), α -> α :=\n\
         fun (α : Sort u) => fun (a : α) => a\n");
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("universe variable"));
}

#[test]
fn cli_checks_ported_py_fol_core() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/py-fol-core.sokonanoda"
    );
    let out = run_args(&[path], None);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("checked declaration id"));
    assert!(stdout.contains("checked declaration and_comm_iff"));
    assert!(stdout.contains("checked declaration or_comm_iff"));
    assert!(stdout.contains("checked declaration Eq_symm"));
    assert!(stdout.contains("checked declaration Eq_trans"));
}

#[test]
fn cli_prints_expression_then_type() {
    let out = run("#check Sort 1\n");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(stdout.trim(), "Sort 1: Type 1");
}

#[test]
fn cli_accepts_type_with_level_as_sort_succ() {
    // Lean 记法：`Type 0` = `Sort 1`；`#check` 显示它的类型为 `Type 1`。
    let out = run("#check Type 0\n");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(stdout.trim(), "Type 0: Type 1");
}

#[test]
fn cli_checks_ported_nat_fol_and_reduces_add() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/py-nat.sokonanoda"
    );
    let out = run_args(&[path], None);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("checked declaration add"));
    assert!(stdout.contains("add two two => succ (succ (succ (succ zero)))"));
}

#[test]
fn repl_prove_shows_partial_lambda_and_checks_done() {
    let out = run_repl(
        "#prove {a : Prop} -> a -> a\n\
         intro a\n\
         intro h\n\
         exact h\n\
         done\n",
    );
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("lambda: fun {a : Prop} => sorry"));
    assert!(stdout.contains("lambda: fun {a : Prop} => fun (h : a) => sorry"));
    assert!(stdout.contains("lambda: fun {a : Prop} => fun (h : a) => h"));
    assert!(stdout.contains("checked example"), "stdout: {stdout}");
}

#[test]
fn repl_prove_assumption_resolves_goal() {
    let out = run_repl(
        "#prove {a : Prop} -> a -> a\n\
         intro a\n\
         intro h\n\
         assumption\n\
         done\n",
    );
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("lambda: fun {a : Prop} => fun (h : a) => h"));
    assert!(stdout.contains("checked example"), "stdout: {stdout}");
}

#[test]
fn json_mode_emits_structured_events() {
    let out = run_args(
        &["--json"],
        Some("def id : Prop -> Prop := fun (x : Prop) => x\n#check id\n"),
    );
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let mut types = Vec::new();
    for line in stdout.lines() {
        let value: serde_json::Value = serde_json::from_str(line).expect("each line is JSON");
        let t = value
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        types.push(t.clone());
        match t.as_str() {
            "decl.checked" => {
                assert_eq!(value["name"], "id");
            }
            "expr.typed" => {
                assert_eq!(value["text"], "id");
                assert_eq!(value["inferred_type"], "Prop -> Prop");
                assert!(value.get("span").is_some(), "expr.typed must carry a span");
            }
            _ => {}
        }
    }
    assert_eq!(types, vec!["decl.checked", "expr.typed"]);
}

#[test]
fn json_mode_types_universe_applied_eq_prelude_constants() {
    // 回归（2026-09-10）：`#check (Eq.subst.{1})` / `#check (Eq.refl.{1})`
    // 曾因内核 pp 对开项推断 panic 而误报 kernel-rejected。现在必须给出
    // expr.typed 事件与完整签名（依赖 codomain `p a` 可打印）。
    let out = run_args(
        &["--json"],
        Some("#check (Eq.subst.{1})\n#check (Eq.refl.{1})\n"),
    );
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let events: Vec<serde_json::Value> = stdout
        .lines()
        .map(|line| serde_json::from_str(line).expect("each line is JSON"))
        .collect();
    assert_eq!(events.len(), 2, "two #check events expected: {events:?}");
    for (value, text) in events.iter().zip(["Eq.subst.{1}", "Eq.refl.{1}"]) {
        assert_eq!(value["type"], "expr.typed");
        assert_eq!(value["text"], text);
        let ty = value["inferred_type"].as_str().expect("inferred_type");
        assert!(!ty.is_empty(), "{text} must infer a type");
    }
    assert!(
        events[0]["inferred_type"]
            .as_str()
            .expect("subst type")
            .contains("p a"),
        "Eq.subst's inferred type must mention the dependent codomain: {:?}",
        events[0]
    );
}

#[test]
fn json_mode_reports_kernel_stage_for_rejections() {
    let out = run_args(
        &["--json"],
        Some("def bad : Prop -> Type := fun (x : Prop) => x\n"),
    );
    assert!(!out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let value: serde_json::Value =
        serde_json::from_str(stdout.lines().next().expect("a diagnostic line"))
            .expect("diagnostic is JSON");
    assert_eq!(value["type"], "diagnostic");
    assert_eq!(
        value["stage"], "kernel",
        "rejection should be staged as kernel: {value}"
    );
    assert_eq!(value["code"], "kernel-rejected");
}

#[test]
fn json_mode_reports_parse_stage_for_lex_errors() {
    let out = run_args(&["--json"], Some("def broken : Prop :=\n"));
    assert!(!out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let value: serde_json::Value =
        serde_json::from_str(stdout.lines().next().expect("a diagnostic line"))
            .expect("diagnostic is JSON");
    assert_eq!(value["type"], "diagnostic");
    assert_eq!(value["stage"], "parse");
    assert_eq!(value["code"], "unexpected-token");
}

#[test]
fn json_mode_open_exercise_is_a_machine_event() {
    let out = run_args(&["--json"], Some("example : Prop -> Prop := sorry\n"));
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let value: serde_json::Value =
        serde_json::from_str(stdout.lines().next().expect("an event line")).expect("event is JSON");
    assert_eq!(value["type"], "exercise.open");
}

#[test]
fn json_mode_function_argument_hole_is_an_open_exercise() {
    // 方案一（2026-09-10）：已知函数（含 Eq prelude）的直接实参洞是合法
    // 练习状态，不再报 elab-hole-misplaced。
    let out = run_args(
        &["--json"],
        Some(concat!(
            "theorem eq_symm_nat : (a : Nat) -> (b : Nat) -> Eq.{1} Nat a b -> Eq.{1} Nat b a :=\n",
            "  fun (a : Nat) (b : Nat) (h : Eq.{1} Nat a b) =>\n",
            "    Eq.subst.{1} Nat (sorry) a b h (Eq.refl.{1} Nat a)\n",
        )),
    );
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(stdout.lines().count(), 1, "one event line: {stdout}");
    let value: serde_json::Value =
        serde_json::from_str(stdout.lines().next().expect("an event line")).expect("event is JSON");
    assert_eq!(value["type"], "exercise.open");
    assert_eq!(value["name"], "eq_symm_nat");
}

#[test]
fn human_errors_carry_the_pipeline_stage() {
    let out = run("def bad : Prop -> Type := fun (x : Prop) => x\n");
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("error[kernel-rejected]:"),
        "stderr: {stderr}"
    );
}

#[test]
fn json_mode_example_checked_has_no_name() {
    let out = run_args(
        &["--json"],
        Some("example : Prop -> Prop := fun (x : Prop) => x\n"),
    );
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let value: serde_json::Value =
        serde_json::from_str(stdout.lines().next().expect("an event line")).expect("event is JSON");
    assert_eq!(value["type"], "example.checked");
}

#[test]
fn bare_flag_compiles_without_prelude() {
    let out = run_args(
        &["--bare"],
        Some("def id : Prop -> Prop := fun (x : Prop) => x\n"),
    );
    assert!(
        out.status.success(),
        "bare Prop-level file must check: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(String::from_utf8_lossy(&out.stdout).contains("checked declaration id"));
}

#[test]
fn bare_flag_loses_nat_entirely() {
    let out = run_args(&["--bare"], Some("#reduce 1 + 1\n"));
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("error[elab-unknown-identifier]:"),
        "bare mode must not know Nat.add: {stderr}"
    );
}

#[test]
fn file_directive_selects_bare_prelude() {
    let src = "-- sokonanoda:prelude none\ndef id : Prop -> Prop := fun (x : Prop) => x\n";
    let out = run(src);
    assert!(
        out.status.success(),
        "directive-driven bare file must check: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    // Same file WITHOUT the directive gets the prelude and still checks.
    let out = run("def id : Prop -> Prop := fun (x : Prop) => x\n");
    assert!(out.status.success());
}

#[test]
fn file_directive_bare_loses_nat() {
    let src = "-- sokonanoda:prelude none\ndef two : Nat := 2\n";
    let out = run(src);
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("error[elab-unknown-identifier]:"),
        "bare file must not know Nat: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn eq_prelude_is_available_by_default() {
    let out = run("theorem refl_two : Eq.{1} Nat 2 2 := Eq.refl.{1} Nat 2\n");
    assert!(
        out.status.success(),
        "Eq prelude must be installed in full mode: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(String::from_utf8_lossy(&out.stdout).contains("checked declaration refl_two"));
}

// ---- 内核错误分类学：kernel-* 细粒度错误码（e2e）----

#[test]
fn cli_classifies_theorem_not_prop() {
    let out = run("theorem t : Nat := 1\n");
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("error[kernel-theorem-not-prop]:"),
        "stderr: {stderr}"
    );
}

#[test]
fn cli_classifies_expected_sort() {
    let out = run("def x : 1 := 1\n");
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("error[kernel-expected-sort]:"),
        "stderr: {stderr}"
    );
}

#[test]
fn cli_classifies_expected_pi() {
    let out = run("def bad : Nat := ((fun (x : Nat) => x) 1) 2\n");
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("error[kernel-expected-pi]:"),
        "stderr: {stderr}"
    );
}

#[test]
fn cli_classifies_check_apply_to_non_function() {
    // #check 直通内核求值路径：对非函数继续应用 → kernel-expected-pi
    // （panic 经 quiet_catch 降级为诊断，绝不能崩掉进程）。
    let out = run("#check (fun (x : Nat) => x) 1 2\n");
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("error[kernel-expected-pi]"),
        "stderr: {stderr}"
    );
}

#[test]
fn cli_classifies_iota_rules_out_of_order() {
    // 内核冷路径分诊 e2e：iota 规则顺序写反 → kernel-rec-rule-mismatch
    // （此前是裸 assert_eq 的指针调试输出）。
    let src = "inductive MyNat : Type\n\
         ctor z : MyNat\n\
         ctor s (n : MyNat) : MyNat\n\
         rec MyNat.rec {u} : (motive : (n : MyNat) -> Sort u) -> (mz : motive z) -> (ms : (n : MyNat) -> motive n -> motive (s n)) -> (n : MyNat) -> motive n\n\
         iota s := fun (motive : (n : MyNat) -> Sort u) => fun (mz : motive z) => fun (ms : (n : MyNat) -> motive n -> motive (s n)) => fun (n : MyNat) => ms n (MyNat.rec.{u} motive mz ms n)\n\
         iota z := fun (motive : (n : MyNat) -> Sort u) => fun (mz : motive z) => fun (ms : (n : MyNat) -> motive n -> motive (s n)) => mz\n\
         end\n";
    let out = run(src);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("error[kernel-rec-rule-mismatch]:"),
        "stderr: {stderr}"
    );
}

#[test]
fn cli_classifies_missing_iota_rule() {
    let src = "inductive MyNat : Type\n\
         ctor z : MyNat\n\
         ctor s (n : MyNat) : MyNat\n\
         rec MyNat.rec {u} : (motive : (n : MyNat) -> Sort u) -> (mz : motive z) -> (ms : (n : MyNat) -> motive n -> motive (s n)) -> (n : MyNat) -> motive n\n\
         iota z := fun (motive : (n : MyNat) -> Sort u) => fun (mz : motive z) => fun (ms : (n : MyNat) -> motive n -> motive (s n)) => mz\n\
         end\n";
    let out = run(src);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("error[kernel-rec-rule-mismatch]:"),
        "stderr: {stderr}"
    );
}

#[test]
fn cli_prints_its_version() {
    // 业内标配：`--version` 零门槛自省（发布自动化的前置）。
    let out = run_args(&["--version"], None);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.trim().starts_with("sokonanoda "), "stdout: {stdout}");
}

#[test]
fn repl_prove_undo_steps_back_and_reports_empty_history() {
    let out = run_repl(
        "#prove (a : Prop) -> a -> a\n\
         intro a\n\
         intro h\n\
         undo\n\
         undo\n\
         undo\n",
    );
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    // 第一次 undo：回到只剩 binder a 的状态。
    assert!(stdout.contains("goal: a -> a"), "stdout: {stdout}");
    assert!(
        stdout.contains("lambda: fun (a : Prop) => sorry"),
        "stdout: {stdout}"
    );
    // 第二次 undo：回到 #prove 的初始状态。
    assert!(
        stdout.contains("goal: (a : Prop) -> a -> a"),
        "stdout: {stdout}"
    );
    // 第三次 undo：没有可撤销的证明步（stderr）。
    assert!(
        stderr.contains("error: 没有可撤销的证明步"),
        "stderr: {stderr}"
    );
}

#[test]
fn repl_appends_nonempty_inputs_to_history() {
    let home = temp_home();
    let out = run_repl_in(&home, "1 + 1\n#check Nat\n\n");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let history = std::fs::read_to_string(home.join(".sokonanoda_history")).expect("history file");
    let lines: Vec<&str> = history.lines().collect();
    assert_eq!(lines, vec!["1 + 1", "#check Nat"]);
}

#[test]
fn repl_history_missing_home_is_silently_disabled() {
    let missing = std::env::temp_dir().join(format!(
        "sokonanoda-cli-missing-home-{}-{}",
        std::process::id(),
        TEMP_HOME_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&missing);
    let out = run_repl_in(
        &missing,
        "def id : Prop -> Prop := fun (x : Prop) => x\n#check id\n",
    );
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("checked declaration id"),
        "stdout: {stdout}"
    );
}

#[test]
fn repl_history_accumulates_across_sessions() {
    let home = temp_home();
    let first = run_repl_in(&home, "def id : Prop -> Prop := fun (x : Prop) => x\n");
    let second = run_repl_in(&home, "#check Nat\n");
    assert!(
        first.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&first.stderr)
    );
    assert!(
        second.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&second.stderr)
    );
    let history = std::fs::read_to_string(home.join(".sokonanoda_history")).expect("history file");
    let lines: Vec<&str> = history.lines().collect();
    assert_eq!(
        lines,
        vec!["def id : Prop -> Prop := fun (x : Prop) => x", "#check Nat"]
    );
}

#[test]
fn cli_accepts_non_recursive_inductive_block() {
    // is_recursive 镜像修复：内核按构造子自算，front 传同值 —— 非递归块
    // （Bool/Unit/Empty 的前置）此前在内核 assert 崩溃。
    let src = "inductive Unit : Type\n\
         ctor unit : Unit\n\
         rec Unit.rec {u} : (motive : (x : Unit) -> Sort u) -> (mz : motive unit) -> (x : Unit) -> motive x\n\
         iota unit := fun (motive : (x : Unit) -> Sort u) => fun (mz : motive unit) => mz\n\
         end\n\
         def u : Unit := unit\n\
         #reduce Unit.rec.{1} (fun (x : Unit) => Nat) 1 u\n";
    let out = run(src);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("=> 1"),
        "iota on the non-recursive ctor must compute: {stdout}"
    );
}

#[test]
fn cli_accepts_bool_with_auto_derived_recursor() {
    // 无 rec 的归纳块自动派生 recursor（显式 rec 优先不变）：Bool + not +
    // 一次 #reduce 的 e2e 冒烟。
    let src = "inductive Bool : Type\n\
         ctor tt : Bool\n\
         ctor ff : Bool\n\
         end\n\
         def not : Bool -> Bool := fun (b : Bool) => Bool.rec.{1} (fun (x : Bool) => Bool) ff tt b\n\
         #reduce not tt\n";
    let out = run(src);
    assert!(
        out.status.success(),
        "auto-derived recursor must compile: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("checked declaration not"),
        "stdout: {stdout}"
    );
    assert!(stdout.contains("=> ff"), "stdout: {stdout}");
}

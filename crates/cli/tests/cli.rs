use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

static TEMP_HOME_COUNTER: AtomicU64 = AtomicU64::new(0);

const COURSE_MANIFEST: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../course/course.json");

/// A unique, empty compile-cache dir for one spawned binary. Tests isolate the
/// shared on-disk cache so a run never observes another test's entries; pass
/// the same dir to [`run_args_with_cache`] when a hit is the thing under test.
fn cache_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sokonanoda-cli-cache-{tag}-{}-{}",
        std::process::id(),
        TEMP_HOME_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

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
        .env("SOKONANODA_CACHE_DIR", cache_dir("repl"))
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
    run_args_with_cache(args, input, &cache_dir("args"))
}

/// Like [`run_args`], but pins the compile cache to `cache` so a test can
/// observe a warm hit across invocations.
fn run_args_with_cache(args: &[&str], input: Option<&str>, cache: &Path) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_sokonanoda"))
        .args(args)
        .env("SOKONANODA_CACHE_DIR", cache)
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

/// 人类视图（`docs/design/redundant-sorry.md` §6）：多留一行 `sorry` 时，
/// stdout 仍是 `exercise open`（语义不变），stderr 多一行
/// `行:列: warning[redundant-sorry]: …`，退出码保持 0。
#[test]
fn cli_redundant_sorry_warns_on_stderr_and_stays_successful() {
    let src = "axiom A : Prop\n\
               axiom B : Prop\n\
               axiom f : A -> B\n\
               theorem t (h : A) : B := f h\n\
               \x20 sorry\n";
    let out = run(src);
    assert!(
        out.status.success(),
        "a warning must not change the exit code:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("exercise open"), "{stdout}");
    assert!(
        !stdout.contains("redundant-sorry"),
        "warnings are stderr-only in the human view: {stdout}"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    let warn_line = stderr
        .lines()
        .find(|l| l.contains("warning[redundant-sorry]"))
        .unwrap_or_else(|| panic!("human view needs one redundant-sorry warning line: {stderr}"));
    assert!(
        warn_line.starts_with("5:"),
        "warning points at the leftover `sorry` line: {warn_line}"
    );
}

#[test]
fn cli_by_newline_separated_tactics_check_via_kernel() {
    // 换行也能分隔 tactic：末尾的 `;` 可以省略，判定仍走 kernel。
    let src = concat!(
        "axiom And : Prop -> Prop -> Prop\n",
        "axiom And.intro : (a : Prop) -> (b : Prop) -> a -> b -> And a b\n",
        "theorem nl : (a : Prop) -> (b : Prop) -> a -> b -> And a b := by\n",
        "  intro a\n",
        "  intro b\n",
        "  intro ha\n",
        "  intro hb\n",
        "  apply And.intro\n",
        "  exact ha\n",
        "  exact hb\n",
    );
    let out = run(src);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("checked declaration nl"), "{stdout}");
}

#[test]
fn cli_by_match_tactic_checks_via_kernel() {
    // `match` 作为 tactic：以当前目标为期望类型，臂体是项。
    let src = "\
inductive Color : Type
ctor red : Color
ctor green : Color
end
def swap (c : Color) : Color := by match c with | red => green | green => red
";
    let out = run(src);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("checked declaration swap"), "{stdout}");
}

#[test]
fn cli_value_funintro_is_no_longer_a_keyword() {
    // `funintro` 已从语言中移除（docs/design/remove-funintro.md）：它现在是
    // 一个普通标识符，值位引用会因未定义而报错，不再是合法 Open 练习。
    let out = run("theorem t : (a : Prop) -> a := funintro\n");
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("error[elab-unknown-identifier]:"),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn cli_decl_binders_compile_and_open() {
    // 官方 Lean 风格：声明级 binder——`:= sorry` 的目标就是 codomain，
    // 正文直接写、不用 fun；两种形态（Open / 闭合）都要过。
    let src = "axiom And : Prop -> Prop -> Prop\n\
               axiom And.intro : (a : Prop) -> (b : Prop) -> a -> b -> And a b\n\
               axiom And.left : (a : Prop) -> (b : Prop) -> And a b -> a\n\
               axiom And.right : (a : Prop) -> (b : Prop) -> And a b -> b\n\
               theorem and_swap2 (a : Prop) (b : Prop) (h : And a b) : And b a := sorry\n\
               theorem and_swap3 (a : Prop) (b : Prop) (h : And a b) : And b a := \
               And.intro b a (And.right a b h) (And.left a b h)\n";
    let out = run(src);
    assert!(
        out.status.success(),
        "declaration binders must compile:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("exercise open"), "{stdout}");
    assert!(stdout.contains("checked declaration and_swap3"), "{stdout}");
}

#[test]
fn cli_untyped_decl_binder_is_a_parse_error() {
    let out = run("theorem t (a) : Prop := Prop\n");
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("显式类型"), "stderr: {stderr}");
}

#[test]
fn cli_axiom_decl_binders_compile() {
    // WO-008 / G-13：axiom 的参数表（1/2/3/4 号写法）+ 一条 `:= sorry` 练习
    // + 一条消费该公理的闭合定理，全部要走内核判过（比结果，不比文本）。
    // 注意判据是**内核**：闭命题直接重推会被内核展开成 `Sort(0)`，所以消费
    // 定理一律把命题放在前提里（与既有 `cli_decl_binders_compile_and_open` 同款）。
    // G-01 起开练习的签名也受检：`Baz α β` 是 Type（`Sort 1`），不是 Prop
    // ⇒ 练习位用 `def`（`theorem` 只接命题，与 checked 路径同判）。
    let src = "axiom Foo (α : Type) : Prop\n\
               axiom Bar {α : Type} : Prop\n\
               axiom Baz (α β : Type) : Sort 1\n\
               axiom Qux {u} (α : Sort u) : Prop\n\
               theorem use_foo (α : Type) (h : Foo α) : Foo α := h\n\
               def practice (α β : Type) : Baz α β := sorry\n";
    let out = run(src);
    assert!(
        out.status.success(),
        "axiom declaration binders must compile:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    for name in ["Foo", "Bar", "Baz", "Qux", "use_foo"] {
        assert!(
            stdout.contains(&format!("checked declaration {name}")),
            "missing checked {name}: {stdout}"
        );
    }
    assert!(stdout.contains("exercise open"), "{stdout}");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.contains("error[parse]"), "stderr: {stderr}");
}

#[test]
fn cli_axiom_untyped_decl_binder_is_a_parse_error() {
    let out = run("axiom Foo (α) : Prop\n");
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("显式类型"), "stderr: {stderr}");
}

#[test]
fn cli_axiom_curried_forms_still_compile() {
    // 防回归：7/8/9 号写法（柯里化 + 宇宙参数 + 无 binder）逐字不变。
    let src = "axiom Foo : (α : Type) -> Prop\n\
               axiom Bar {u} : (α : Sort u) -> Prop\n\
               axiom Plain : Prop\n";
    let out = run(src);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    for name in ["Foo", "Bar", "Plain"] {
        assert!(
            stdout.contains(&format!("checked declaration {name}")),
            "{stdout}"
        );
    }
}

#[test]
fn cli_space_separated_and_split_universe_params_compile() {
    // WO-009 表 3/4/9/11：{u v} / {u} {v} 在 def/theorem/axiom 上都要过。
    let src = "def idTwo {u v} (α : Sort u) (β : Sort v) (a : α) : α := a\n\
               theorem thmTwo {u v} (α : Sort u) (β : Sort v) (a : α) :\
               Eq.{u} α a a := Eq.refl.{u} α a\n\
               axiom AxTwo {u} {v} : Sort u\n";
    let out = run(src);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("checked declaration idTwo"), "{stdout}");
    assert!(stdout.contains("checked declaration thmTwo"), "{stdout}");
    assert!(stdout.contains("checked declaration AxTwo"), "{stdout}");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.contains("error[parse]"), "stderr: {stderr}");
}

#[test]
fn cli_universe_param_name_cannot_end_with_a_dot() {
    // G-18：`def f.{u}` 曾把名字静默吃成 `f.`；现在必须报 parse 错。
    let out = run("def f.{u} (α : Sort u) : α -> α := fun a => a\n");
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("声明名"), "stderr: {stderr}");
    assert!(stderr.contains("error[parse]"), "stderr: {stderr}");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(!stdout.contains("checked declaration f."), "{stdout}");
}

#[test]
fn cli_untyped_lambda_binder_is_inferred_from_the_argument() {
    // I6：应用位置的 `fun x => …` 从实参类型推断 binder（kernel-backed）。
    let src = "\
def k : Nat := (fun x => x) 1
#reduce (fun x y => x) 3 4
";
    let out = run(src);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("checked declaration k"), "{stdout}");
    assert!(stdout.contains("=> 3"), "{stdout}");
}

#[test]
fn cli_decl_binders_feed_the_by_engine() {
    let src = "axiom And : Prop -> Prop -> Prop\n\
               axiom And.left : (a : Prop) -> (b : Prop) -> And a b -> a\n\
               theorem by_ctx (a : Prop) (b : Prop) (h : And a b) : a := \
               by exact And.left a b h\n";
    let out = run(src);
    assert!(
        out.status.success(),
        "by with declaration binders must compile:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(String::from_utf8_lossy(&out.stdout).contains("checked declaration by_ctx"));
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
    // I16：import 与项目根开关必须自文档化（用户第一次遇到 import 报错时查这里）。
    assert!(stdout.contains("import Logic"), "stdout: {stdout}");
    assert!(stdout.contains("--root <dir>"), "stdout: {stdout}");
    assert!(stdout.contains("--no-project"), "stdout: {stdout}");
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
    // 实测（规范名让源 `Nat` 的 succ 链走内核 NatRed 快路径 ⇒ 混合表示）。
    assert!(stdout.contains("add two two => Nat.succ (Nat.succ (Nat.succ 1))"));
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
fn json_mode_let_declaration_and_open_exercise() {
    // Phase 1 值位 `let`（docs/design/elaborator-let-match.md §7.2）：含 `let` 的
    // 文档照常给出 `decl.checked`（闭合）与 `exercise.open`（值位洞）。
    let out = run_args(
        &["--json"],
        Some(concat!(
            "def two : Nat := let one : Nat := Nat.succ Nat.zero; one + one\n",
            "example : Nat := let x : Nat := sorry; x\n",
        )),
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
    assert_eq!(events.len(), 2, "two events expected: {events:?}");
    assert_eq!(events[0]["type"], "decl.checked");
    assert_eq!(events[0]["name"], "two");
    assert_eq!(events[1]["type"], "exercise.open");
}

#[test]
fn json_mode_unannotated_let_infers_or_reports_hint() {
    // Phase 2：无注解 `let` 由内核推断值类型；推断成功即 checked。
    let ok = run_args(&["--json"], Some("def one : Nat := let x := Nat.zero; x\n"));
    assert!(ok.status.success(), "unannotated let must infer: {ok:?}");
    // 推断不出（值位洞）→ 教学错误 `elab-let-type-query-failed`，带 let hint。
    let out = run_args(&["--json"], Some("def bad : Nat := let x := sorry; x\n"));
    assert!(!out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let value: serde_json::Value =
        serde_json::from_str(stdout.lines().next().expect("a diagnostic line"))
            .expect("diagnostic is JSON");
    assert_eq!(value["type"], "diagnostic");
    assert_eq!(value["stage"], "elab");
    assert_eq!(value["code"], "elab-let-type-query-failed");
    let hint = value["hint"].as_str().expect("diagnostic carries a hint");
    assert!(
        hint.contains("let"),
        "hint must teach the let spelling: {hint}"
    );
}

#[test]
fn json_mode_let_checks_and_reduces() {
    // `#check`/`#reduce` 与 `let` 交互：推断类型与 zeta 归约结果都要正确。
    let out = run_args(
        &["--json"],
        Some(concat!(
            "#check (let x : Nat := 1; x)\n",
            "#reduce (let x : Nat := 1; x + 2)\n",
        )),
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
    assert_eq!(events.len(), 2, "two events expected: {events:?}");
    assert_eq!(events[0]["type"], "expr.typed");
    assert_eq!(events[0]["text"], "let x : Nat := 1; x");
    assert_eq!(events[0]["inferred_type"], "Nat");
    assert_eq!(events[1]["type"], "expr.reduced");
    assert_eq!(events[1]["text"], "let x : Nat := 1; x + 2");
    assert_eq!(events[1]["value"], "3");
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

// ---- L1 prelude（docs/design/prelude-l1-proposal.md P2）：CLI e2e ----

/// 只用 L1 名字、且不自己声明任何 L1 名字的文件（否则整族让位）。
const L1_E2E_SRC: &str = "\
def l1_and (a b : Prop) (ha : a) (hb : b) : And b a := And.intro b a hb ha
def l1_or (a b c : Prop) (f : a -> c) (g : b -> c) (h : Or a b) : c := Or.elim a b c f g h
def l1_iff (a b : Prop) (h : Iff a b) : b -> a := Iff.mpr a b h
def l1_absurd (a b : Prop) (ha : a) (hna : Not a) : b := absurd a b ha hna
def l1_true : True := True.intro
def l1_eq_symm (a b : Nat) (h : Eq.{1} Nat a b) : Eq.{1} Nat b a := Eq.symm.{1} Nat a b h
def l1_congr_arg (f : Nat -> Nat) (a b : Nat) (h : Eq.{1} Nat a b) : Eq.{1} Nat (f a) (f b) :=
  congrArg.{1} Nat Nat f a b h
";

#[test]
fn l1_prelude_is_available_in_full_mode() {
    let out = run_args(&["--json"], Some(L1_E2E_SRC));
    assert!(
        out.status.success(),
        "L1 prelude must install by default: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let events = json_events(&String::from_utf8_lossy(&out.stdout));
    let checked = events
        .iter()
        .filter(|e| e["type"] == "decl.checked")
        .count();
    assert_eq!(checked, 7, "every L1 probe declaration must be checked");
}

#[test]
fn l1_prelude_is_absent_under_bare_flag() {
    let out = run_args(&["--bare"], Some(L1_E2E_SRC));
    assert!(!out.status.success(), "Bare mode must not install L1");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("error[elab-unknown-identifier]:"),
        "bare mode must report unknown identifiers: {stderr}"
    );
}

#[test]
fn l1_prelude_is_absent_under_the_bare_directive() {
    let out = run(&format!("-- sokonanoda:prelude none\n{L1_E2E_SRC}"));
    assert!(
        !out.status.success(),
        "the bare directive must not install L1"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("error[elab-unknown-identifier]:"),
        "the bare directive must report unknown identifiers: {stderr}"
    );
}

/// 同一份文件的两个视图必须一致：`query check` 的 `decl_checked` 与事件流。
#[test]
fn l1_query_check_counts_match_the_event_stream() {
    let path = std::env::temp_dir().join(format!(
        "sokonanoda-cli-l1-{}-{}.sokonanoda",
        std::process::id(),
        TEMP_HOME_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::write(&path, L1_E2E_SRC).expect("write L1 canvas");
    let out = Command::new(env!("CARGO_BIN_EXE_sokonanoda"))
        .args([
            "query",
            "check",
            "--file",
            path.to_str().expect("utf-8 path"),
            "--compact",
        ])
        .env("SOKONANODA_CACHE_DIR", cache_dir("l1-query"))
        .output()
        .expect("run query check");
    assert!(
        out.status.success(),
        "query check must answer: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let value: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("query check emits one JSON object");
    let stream = run_args(&["--json"], Some(L1_E2E_SRC));
    let events = json_events(&String::from_utf8_lossy(&stream.stdout));
    let checked = events
        .iter()
        .filter(|e| e["type"] == "decl.checked")
        .count();
    assert_eq!(
        value["data"]["counts"]["decl_checked"].as_u64().unwrap() as usize,
        checked,
        "query check and --json must agree on decl.checked for L1"
    );
    let _ = std::fs::remove_file(&path);
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
    assert!(stdout.contains("=> Bool.ff"), "stdout: {stdout}");
}

#[test]
fn cli_accepts_prop_inductive_with_type_parameter() {
    // G-03 / WO-006：`inductive Bar (A : Type) : Prop` + `ctor mk (a : A)`。
    // 派生 recursor 的宇宙参数必须**逐字镜像**内核 `large_elim_test`：
    // `a` 不是参数也不是索引 ⇒ 不 large eliminate ⇒ recursor 无宇宙参数、
    // motive 落在 `Prop`。修前内核拒（`left:1/right:0`）。
    let src = "inductive Bar (A : Type) : Prop\n\
         ctor mk (a : A) : Bar A\n\
         end\n\
         #check Bar.rec\n";
    let out = run(src);
    assert!(
        out.status.success(),
        "a Prop inductive with a Type parameter must compile: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    // 签名断言：`#check Bar.rec` 的输出里 motive 落在 `Prop`（不是 `Sort u`）。
    // 这是本缺口的本质——recursor 的**形状**，不是"声明得过"。
    assert!(
        stdout.contains("motive : Bar A -> Prop"),
        "the derived recursor's motive must land in Prop: {stdout}"
    );
    assert!(
        !stdout.contains("Sort u"),
        "the derived recursor must not be universe-polymorphic: {stdout}"
    );
}

#[test]
fn cli_prop_recursor_without_universe_parameter_computes() {
    // 同一条块上再走一次 `match`：0 级 recursor 常量与 iota 规则都要真的对
    // （不只是"声明得过"）。结果类型写成 `Bar A`（Prop 值），motive codomain
    // 才会落在 `Sort 0`。
    let src = "inductive Bar (A : Type) : Prop\n\
         ctor mk (a : A) : Bar A\n\
         end\n\
         def bar_id (A : Type) (b : Bar A) : Bar A :=\n\
         \x20 match b with\n\
         \x20 | mk a => b\n\
         #check bar_id\n";
    let out = run(src);
    assert!(
        out.status.success(),
        "match on the G-03 shape must compile: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Bar"), "stdout: {stdout}");
}

#[test]
fn cli_rejects_universe_argument_on_a_zero_universe_recursor() {
    // 反例守卫（判别性）：同一条块若**错误地**拿到 1 个宇宙参数，下面这句
    // `Bar.rec.{0}` 就会通过。它必须报 arity 0 —— 把 recursor 的形状钉死。
    let src = "inductive Bar (A : Type) : Prop\n\
         ctor mk (a : A) : Bar A\n\
         end\n\
         #check Bar.rec.{0}\n";
    let out = run(src);
    assert!(
        !out.status.success(),
        "`Bar.rec.{{0}}` must be an arity error on a 0-universe recursor"
    );
    // 诊断走 stderr（stdout 只有 `checked declaration Bar`）。
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("elab-universe-arity"),
        "a 0-universe recursor must reject a universe argument: {stderr}"
    );
}

// ---- match（design docs/design/match.md，v1）----

const COLOR_ENUM: &str = "inductive Color : Type\n\
     ctor red : Color\n\
     ctor green : Color\n\
     end\n";

#[test]
fn cli_match_enum_checks_via_kernel() {
    // 值位 `match` 对非递归源内枚举分情况，判定照旧走完整内核 →
    // 照常产出 `decl.checked`。
    let src = format!(
        "{COLOR_ENUM}\
         def swap (c : Color) : Color := match c with\n\
         | red => green\n\
         | green => red\n"
    );
    let out = run(&src);
    assert!(
        out.status.success(),
        "match must compile: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("checked declaration swap"),
        "stdout: {stdout}"
    );
}

#[test]
fn cli_match_sorry_branch_is_open_exercise() {
    // 分支里的 sorry 是合法 Open 状态（目标类型就是声明的结果类型 Color）。
    let src = format!(
        "{COLOR_ENUM}\
         example (c : Color) : Color := match c with\n\
         | red => green\n\
         | green => sorry\n"
    );
    let out = run(&src);
    assert!(
        out.status.success(),
        "a sorry branch must stay exit 0: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("exercise open"),
        "stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}

#[test]
fn cli_match_non_exhaustive_reports_code() {
    // 漏写 green 分支 → 稳定的教学错误码 elab-match-non-exhaustive。
    let src = format!(
        "{COLOR_ENUM}\
         def f (c : Color) : Color := match c with\n\
         | red => green\n"
    );
    let out = run(&src);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("error[elab-match-non-exhaustive]:"),
        "stderr: {stderr}"
    );
}

#[test]
fn cli_match_reduces_through_kernel() {
    // #reduce 走 match 降低出的 recursor：swap red 归约到 green。
    let src = format!(
        "{COLOR_ENUM}\
         def swap (c : Color) : Color := match c with\n\
         | red => green\n\
         | green => red\n\
         #reduce swap red\n"
    );
    let out = run(&src);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("swap red => Color.green"),
        "stdout: {stdout}"
    );
}

/// The course-style explicit `Nat` (inductive + `rec`/`iota`), mirroring
/// `course/unit6-induction-recursion-1.sokonanoda`.
const MATCH_NAT: &str = "inductive Nat : Type\n\
     ctor zero : Nat\n\
     ctor succ (n : Nat) : Nat\n\
     rec Nat.rec {u} : (motive : (n : Nat) -> Sort u) -> (mz : motive zero) -> (ms : (n : Nat) -> motive n -> motive (succ n)) -> (n : Nat) -> motive n\n\
     iota zero := fun (motive : (n : Nat) -> Sort u) => fun (mz : motive zero) => fun (ms : (n : Nat) -> motive n -> motive (succ n)) => mz\n\
     iota succ := fun (motive : (n : Nat) -> Sort u) => fun (mz : motive zero) => fun (ms : (n : Nat) -> motive n -> motive (succ n)) => fun (n : Nat) => ms n (Nat.rec.{u} motive mz ms n)\n\
     end\n";

#[test]
fn cli_match_recursive_inductive_inserts_the_ih() {
    // Phase 2：递归源内归纳的 match 在递归字段后自动得到归纳假设 `ih`
    // （类型 = 结果类型 Nat），branch 里直接引用；递归无需自引用。
    let src = format!(
        "{MATCH_NAT}\
         def addM (a b : Nat) : Nat := match a with\n\
         | zero => b\n\
         | succ m => succ ih\n"
    );
    let out = run(&src);
    assert!(
        out.status.success(),
        "recursive match must compile: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("checked declaration addM"),
        "stdout: {stdout}"
    );
}

#[test]
fn cli_match_recursive_sorry_branch_is_open_exercise() {
    // 递归分支里的 sorry 仍是合法 Open 状态（目标类型就是声明的结果类型 Nat）。
    let src = format!(
        "{MATCH_NAT}\
         example (a b : Nat) : Nat := match a with\n\
         | zero => b\n\
         | succ m => sorry\n"
    );
    let out = run(&src);
    assert!(
        out.status.success(),
        "a sorry recursive branch must stay exit 0: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("exercise open"),
        "stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}

#[test]
fn cli_match_recursive_reduces_through_the_ih() {
    // #reduce 走 match 降低出的 Nat.rec：addM two three 归约到五层 succ。
    let src = format!(
        "{MATCH_NAT}\
         def two : Nat := succ (succ zero)\n\
         def three : Nat := succ two\n\
         def addM (a b : Nat) : Nat := match a with\n\
         | zero => b\n\
         | succ m => succ ih\n\
         #reduce addM two three\n"
    );
    let out = run(&src);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("addM two three => Nat.succ (Nat.succ 3)"),
        "stdout: {stdout}"
    );
}

#[test]
fn cli_match_on_prelude_nat_checks_and_reduces() {
    // prelude Nat（文件未自带 `inductive Nat`）现在也能 `match`：arms 用点号
    // ctor `Nat.zero`/`Nat.succ`，递归字段后自动有 IH。`#reduce` 经 recursor
    // 归约出的结果可能是不合并的一元链（与 numeral def-eq），此处钉住该形状。
    let src = "\
def pred (n : Nat) : Nat := match n with
| Nat.zero => Nat.zero
| Nat.succ k => k
def addN (a b : Nat) : Nat := match a with
| Nat.zero => b
| Nat.succ k => Nat.succ ih
#reduce addN (Nat.succ (Nat.succ Nat.zero)) (Nat.succ Nat.zero)
";
    let out = run(src);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("checked declaration pred"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("checked declaration addN"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("=> Nat.succ (Nat.succ 1)"),
        "prelude-Nat recursion must reduce: {stdout}"
    );
}

// ---- prelude Bool（0.41.0，镜像 Nat）----

#[test]
fn cli_match_on_prelude_bool_checks_and_reduces() {
    // The trusted `Bool` prelude (no source `inductive Bool`) is matchable with
    // the dotted constructors `Bool.true`/`Bool.false`; `#reduce` runs the
    // derived `Bool.rec` iota rules.
    let src = "\
def bnot (b : Bool) : Bool := match b with
| Bool.true => Bool.false
| Bool.false => Bool.true
def band (a b : Bool) : Bool := match a with
| Bool.true => b
| Bool.false => Bool.false
#reduce bnot Bool.true
#reduce band Bool.true Bool.false
";
    let out = run(src);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("checked declaration bnot"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("checked declaration band"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("=> Bool.false"),
        "prelude-Bool elimination must reduce: {stdout}"
    );
}

// ---- 模式编译器 v1：通配 / 嵌套 / 字面量 / 守卫（0.42.0）----

#[test]
fn cli_match_nested_and_wildcard_patterns_check_via_kernel() {
    // 嵌套模式（同一构造子多条 arm）+ 通配兜底；判定走完整内核。
    let src = "\
inductive Inner : Type
ctor ia : Inner
ctor ib : Inner
end
inductive Outer : Type
ctor oi (i : Inner) : Outer
ctor on : Outer
end
def flip (o : Outer) : Inner := match o with
| oi ia => ib
| oi _ => ia
| on => ia
#reduce flip (oi ia)
#reduce flip (oi ib)
";
    let out = run(src);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("checked declaration flip"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("=> Inner.ib"),
        "nested `oi ia` must pick the first arm: {stdout}"
    );
    assert!(
        stdout.contains("=> Inner.ia"),
        "nested `oi ib` must fall to `oi _`: {stdout}"
    );
}

#[test]
fn cli_match_nat_literals_check_via_kernel() {
    // 字面量模式脱糖为 succ^k zero；与 `Nat.succ k` 混排。
    let src = "\
def is_zero (n : Nat) : Bool := match n with
| 0 => Bool.true
| _ => Bool.false
#reduce is_zero 0
#reduce is_zero 2
";
    let out = run(src);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("checked declaration is_zero"),
        "stdout: {stdout}"
    );
    assert!(stdout.contains("=> Bool.true"), "stdout: {stdout}");
    assert!(stdout.contains("=> Bool.false"), "stdout: {stdout}");
}

#[test]
fn cli_match_guard_falls_through_to_the_next_arm() {
    let src = "\
def pick (a b : Bool) : Bool := match a with
| Bool.true if b => Bool.false
| _ => a
#reduce pick Bool.true Bool.true
#reduce pick Bool.true Bool.false
";
    let out = run(src);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("checked declaration pick"),
        "stdout: {stdout}"
    );
    assert!(stdout.contains("=> Bool.false"), "stdout: {stdout}");
    assert!(stdout.contains("=> Bool.true"), "stdout: {stdout}");
}

// ---- 参数化归纳（docs/design/parameterized-inductives.md，v1）----

/// Non-indexed parameterized source inductive (`inductive Option (A : Type)`);
/// arms match on it with the parameter read from the scrutinee's written type.
const OPTION_ENUM: &str = "inductive Option (A : Type) : Type\n\
     ctor none : Option A\n\
     ctor some (a : A) : Option A\n\
     end\n";

#[test]
fn cli_match_parameterized_option_checks_via_kernel() {
    // 参数化归纳 `Option A` + match：参数实例从 scrutinee 的书写类型取，
    // 判定照旧走完整内核 → 照常产出 `decl.checked`。
    let src = format!(
        "{OPTION_ENUM}\
         def fromOption (x : Option Nat) (d : Nat) : Nat := match x with\n\
         | none => d\n\
         | some a => a\n"
    );
    let out = run(&src);
    assert!(
        out.status.success(),
        "parameterized match must compile: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("checked declaration fromOption"),
        "stdout: {stdout}"
    );
}

#[test]
fn cli_match_parameterized_sorry_branch_is_open_exercise() {
    // some 分支里的 sorry 是合法 Open 状态（目标类型就是声明结果类型 Nat）。
    let src = format!(
        "{OPTION_ENUM}\
         example (x : Option Nat) : Nat := match x with\n\
         | none => Nat.zero\n\
         | some a => sorry\n"
    );
    let out = run(&src);
    assert!(
        out.status.success(),
        "a sorry parameterized branch must stay exit 0: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("exercise open"),
        "stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}

#[test]
fn cli_match_parameterized_without_written_params_reports_code() {
    // scrutinee 是应用 `some Nat Nat.zero`（非局部变量）→ 拿不到参数实例 →
    // 稳定的教学错误码 elab-match-parameterized-unsupported。
    let src = format!(
        "{OPTION_ENUM}\
         def bad : Nat := match (some Nat Nat.zero) with\n\
         | none => Nat.zero\n\
         | some a => a\n"
    );
    let out = run(&src);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("error[elab-match-parameterized-unsupported]:"),
        "stderr: {stderr}"
    );
}

#[test]
fn cli_match_parameterized_reduces_through_kernel() {
    // #reduce 走 match 降低出的 Option.rec：some Nat 1 取出后就是 1。
    let src = format!(
        "{OPTION_ENUM}\
         def fromOption (x : Option Nat) (d : Nat) : Nat := match x with\n\
         | none => d\n\
         | some a => a\n\
         #reduce fromOption (some Nat (Nat.succ Nat.zero)) Nat.zero\n"
    );
    let out = run(&src);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("fromOption (some Nat (Nat.succ Nat.zero)) Nat.zero => 1"),
        "stdout: {stdout}"
    );
}

#[test]
fn cli_match_dependent_motive_checks_via_kernel() {
    // 依赖 motive：结果类型 `P n` 随 scrutinee 变化；分支期望分别是
    // `P zero` / `P (succ k)`，succ 支的 `ih : P k`（依赖 IH）。这里用
    // 声明 binder 形式（P/hz/hs 都是声明 binder）钉住 decl-binder 路径。
    let body = concat!(
        "theorem nat_induction (P : Nat -> Prop) (hz : P zero)\n",
        "    (hs : (k : Nat) -> P k -> P (succ k)) (n : Nat) : P n :=\n",
        "  match n with\n",
        "  | zero => hz\n",
        "  | succ k => hs k ih\n",
        "theorem nat_induction_open (P : Nat -> Prop) (hz : P zero)\n",
        "    (hs : (k : Nat) -> P k -> P (succ k)) (n : Nat) : P n :=\n",
        "  match n with\n",
        "  | zero => hz\n",
        "  | succ k => sorry\n",
    );
    let src = format!("{MATCH_NAT}{body}");
    let out = run(&src);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("checked declaration nat_induction"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("exercise open (fill the sorry)"),
        "dependent branch sorry must stay open: {stdout}"
    );
}

// ---- 带索引归纳（0.47.0，docs/design/indexed-inductives.md）----

#[test]
fn cli_indexed_vec_checks_and_reduces() {
    // 带索引归纳 `Vec (A : Type) : Nat -> Type`：声明 + 派生 recursor +
    // 常量 motive 的 match（长度），判定与归约全走内核。
    let src = "\
inductive Vec (A : Type) : Nat -> Type
ctor vnil : Vec A 0
ctor vcons (a : A) (n : Nat) (v : Vec A n) : Vec A (Nat.succ n)
end
def vlen (A : Type) (n : Nat) (v : Vec A n) : Nat :=
  match v with
  | vnil => 0
  | vcons a m w => Nat.succ ih
#reduce vlen Nat 2 (vcons Nat 1 1 (vcons Nat 2 0 (vnil Nat)))
";
    let out = run(src);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("checked declaration Vec"), "{stdout}");
    assert!(stdout.contains("checked declaration vlen"), "{stdout}");
    assert!(
        stdout.contains("=> Nat.succ (Nat.succ 0)"),
        "indexed recursion must reduce through Vec.rec: {stdout}"
    );
}

#[test]
fn cli_arrow_style_indexed_field_derives_and_reduces() {
    // H6-C 回归（ROADMAP I15）：ctor 的**索引实参写在结果的箭头链里**
    // （`W A n -> W A (Nat.succ n)`）时，递归子派生曾丢掉索引实参、被内核拒
    // （`assert_nonnested_recursors_def_eq`）；课程里的 `Le`/`Even` 因此只能手写
    // `rec`/`iota`。这里从 CLI 端到端钉住"能派生 + 能算"。
    let src = "\
inductive W (A : Type) : Nat -> Type
ctor wnil : W A 0
ctor wcons (a : A) (n : Nat) : W A n -> W A (Nat.succ n)
end
def wlen (A : Type) (n : Nat) (w : W A n) : Nat :=
  match w with
  | wnil => 0
  | wcons a m t => Nat.succ ih
#reduce wlen Nat 2 (wcons Nat 1 1 (wcons Nat 2 0 (wnil Nat)))
";
    let out = run(src);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("checked declaration W"), "{stdout}");
    assert!(stdout.contains("checked declaration wlen"), "{stdout}");
    assert!(
        stdout.contains("=> Nat.succ (Nat.succ 0)"),
        "arrow-style indexed recursion must reduce through W.rec: {stdout}"
    );
}

#[test]
fn cli_single_constructor_prop_derives_recursor() {
    // H6-C 另一半：`install_inductive_block` 曾把 `is_k` 写死成 `false`，于是
    // **单构造子 `Prop`** 的归纳被内核以
    // `recursor declares the wrong k-reduction flag (left: false, right: true)` 拒。
    let out = run("\
inductive True2 : Prop
ctor trivial2 : True2
end
#check True2.rec
");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("checked declaration True2"), "{stdout}");
    assert!(
        stdout.contains("True2.rec"),
        "the derived recursor must be printed: {stdout}"
    );
}

#[test]
fn cli_inductive_accepts_multi_name_binder_groups() {
    // H6-C（解析器缺口）：`inductive` 的参数与 ctor 字段此前只吃单名 binder，
    // 而 Pi/λ/∀/声明 binder 早就支持 `(A B : Prop)`——agent 是主要作者，
    // 合法子集被拒的代价最大。
    //
    // 顺带钉住 K 目标判据：`Both` 的 `mk` **字段数恰好等于参数数**（2 = 2），
    // 按"字段数 vs 参数数"比对就会误判成 K 目标，被内核以
    // `recursor declares the wrong k-reduction flag` 拒掉整个块（这一层就是这么
    // 抓到这个回归的；front 单测见
    // `compile::tests::single_constructor_prop_with_fields_is_not_a_k_target`）。
    let out = run("\
inductive Both (A B : Prop) : Prop
ctor mk (a : A) (b : B) : Both A B
end
inductive Pair2 (A : Prop) : Prop
ctor mk2 (a b : A) : Pair2 A
end
theorem both_symm (A B : Prop) (h : Both A B) : Both B A :=
  match h with
  | mk a b => mk B A b a
theorem pair2_refl (A : Prop) (a : A) : Pair2 A := mk2 A a a
");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    for name in ["Both", "Pair2", "both_symm", "pair2_refl"] {
        assert!(
            stdout.contains(&format!("checked declaration {name}")),
            "{stdout}"
        );
    }
}

// ---- persistent compile cache: `sokonanoda build` warms it; `course` stays
// stable whether an entry is cold or warm (docs/protocol.md). ----

fn json_events(stdout: &str) -> Vec<serde_json::Value> {
    stdout
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

fn write_canvas(tag: &str) -> PathBuf {
    let dir = temp_home();
    let file = dir.join(format!("{tag}.sokonanoda"));
    std::fs::write(&file, "def id : Prop -> Prop := fun (x : Prop) => x\n").expect("write canvas");
    file
}

#[test]
fn cli_build_warms_and_reuses_cache() {
    let file = write_canvas("build-warm");
    let path = file.to_str().expect("utf-8 path");
    let cache = cache_dir("build-warm");

    let first = run_args_with_cache(&["build", "--json", path], None, &cache);
    assert!(
        first.status.success(),
        "first build must succeed, stderr: {}",
        String::from_utf8_lossy(&first.stderr)
    );
    let events = json_events(&String::from_utf8_lossy(&first.stdout));
    let summary = events
        .iter()
        .find(|e| e["type"] == "build.summary")
        .expect("build.summary must be emitted");
    assert_eq!(summary["compiled"], 1, "cold build compiles: {summary}");
    assert_eq!(summary["hit"], 0, "cold build cannot hit: {summary}");

    let second = run_args_with_cache(&["build", "--json", path], None, &cache);
    assert!(
        second.status.success(),
        "second build must succeed, stderr: {}",
        String::from_utf8_lossy(&second.stderr)
    );
    let events = json_events(&String::from_utf8_lossy(&second.stdout));
    let file_event = events
        .iter()
        .find(|e| e["type"] == "build.file")
        .expect("build.file must be emitted");
    assert_eq!(
        file_event["status"], "hit",
        "second build reuses the entry: {file_event}"
    );
    let summary = events
        .iter()
        .find(|e| e["type"] == "build.summary")
        .expect("build.summary must be emitted");
    assert_eq!(summary["hit"], 1, "second build hits: {summary}");
    assert_eq!(
        summary["compiled"], 0,
        "second build compiles nothing: {summary}"
    );
    assert_eq!(
        summary["failed"], 0,
        "second build fails nothing: {summary}"
    );
}

#[test]
fn cli_build_clean_removes_entries() {
    let file = write_canvas("build-clean");
    let path = file.to_str().expect("utf-8 path");
    let cache = cache_dir("build-clean");

    let built = run_args_with_cache(&["build", path], None, &cache);
    assert!(
        built.status.success(),
        "warming build must succeed, stderr: {}",
        String::from_utf8_lossy(&built.stderr)
    );

    let cleaned = run_args_with_cache(&["build", "--clean", "--json"], None, &cache);
    assert!(
        cleaned.status.success(),
        "build --clean must succeed, stderr: {}",
        String::from_utf8_lossy(&cleaned.stderr)
    );
    let events = json_events(&String::from_utf8_lossy(&cleaned.stdout));
    let clean = events
        .iter()
        .find(|e| e["type"] == "build.clean")
        .expect("build.clean must be emitted");
    assert!(
        clean["removed"].as_u64().unwrap_or(0) > 0,
        "clean removes the warmed entry: {clean}"
    );

    let again = run_args_with_cache(&["build", "--clean", "--json"], None, &cache);
    let events = json_events(&String::from_utf8_lossy(&again.stdout));
    let clean = events
        .iter()
        .find(|e| e["type"] == "build.clean")
        .expect("build.clean must be emitted");
    assert_eq!(clean["removed"], 0, "a second clean finds nothing: {clean}");
}

#[test]
fn cli_course_is_stable_with_a_warm_cache() {
    let cache = cache_dir("course-warm");
    let first = run_args_with_cache(&["course", COURSE_MANIFEST, "--json"], None, &cache);
    assert!(
        first.status.success(),
        "course must succeed, stderr: {}",
        String::from_utf8_lossy(&first.stderr)
    );
    let second = run_args_with_cache(&["course", COURSE_MANIFEST, "--json"], None, &cache);
    assert!(
        second.status.success(),
        "course with a warm cache must succeed, stderr: {}",
        String::from_utf8_lossy(&second.stderr)
    );

    let summary = |out: &std::process::Output| {
        json_events(&String::from_utf8_lossy(&out.stdout))
            .into_iter()
            .find(|e| e["type"] == "course.summary")
            .expect("course.summary must be emitted")
    };
    let cold = summary(&first);
    let warm = summary(&second);
    assert_eq!(
        cold, warm,
        "a warm cache must not change the course summary (cold {cold} vs warm {warm})"
    );
    assert_eq!(cold["checked"], 86, "golden checked total: {cold}");
    assert_eq!(cold["open"], 65, "golden open total: {cold}");
}

// ---- 构造子命名空间（G-02 / WO-005；design docs/design/ctor-namespace.md）----

#[test]
fn cli_ctor_names_are_namespaced() {
    // 复现件同构：两个块各 `ctor mk`，规范名 `P1.mk`/`P2.mk` 并存不冲突。
    // 修前：`duplicate declaration mk` + `unknown identifier P1.mk`（exit 1）。
    let src = "inductive P1 (A : Type) : Type\n\
         ctor mk (a : A) : P1 A\n\
         end\n\
         inductive P2 (A : Type) : Type\n\
         ctor mk (a : A) : P2 A\n\
         end\n\
         def first (A : Type) (a : A) : P1 A := P1.mk A a\n\
         def second (A : Type) (a : A) : P2 A := P2.mk A a\n\
         #check P1.mk\n";
    let out = run(src);
    assert!(
        out.status.success(),
        "namespaced ctors must compile: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("checked declaration P1") && stdout.contains("checked declaration P2"),
        "both blocks must check: {stdout}"
    );
    assert!(
        stdout.contains("P1.mk: "),
        "the canonical name must be resolvable and checkable: {stdout}"
    );
}

#[test]
fn cli_grades_the_g02_repro_clean() {
    // 「缺口即测试」：复现件本身从此是 e2e 的一行（WO-005 验收 CLI 第 3 条）。
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/gaps/repro/G02-ctor-namespace.sokonanoda"
    );
    let out = run_args(&[path], None);
    assert!(
        out.status.success(),
        "the repro must grade clean: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("checked declaration first"),
        "`first` (using `P1.mk`) must check: {stdout}"
    );
    assert!(
        !stdout.contains("diagnostic"),
        "no diagnostics expected: {stdout}"
    );
}

#[test]
fn cli_keeps_bare_ctor_aliases_working() {
    // R2 兼容 e2e：课程/示例原文不改（裸名 `none`/`some`）仍 exit 0。
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../course/unit7-induction-recursion-2.sokonanoda"
    );
    let out = run_args(&[path], None);
    assert!(
        out.status.success(),
        "the bare-name course canvas must stay green: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let py_nat = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/py-nat.sokonanoda"
    );
    let out = run_args(&[py_nat], None);
    assert!(
        out.status.success(),
        "examples/py-nat.sokonanoda must stay green: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn cli_reports_an_ambiguous_bare_ctor_alias() {
    // R2：两个类型各声明一次裸名 `mk` ⇒ 裸名不可解析，报稳定的新码
    // `elab-ambiguous-ctor-alias`（**不是** unknown identifier），消息点名两个候选。
    let src = "inductive P1 : Type\n\
         ctor mk : P1\n\
         end\n\
         inductive P2 : Type\n\
         ctor mk : P2\n\
         end\n\
         def bad : P1 := mk\n";
    let out = run(src);
    assert!(!out.status.success(), "the bare name is ambiguous");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("error[elab-ambiguous-ctor-alias]:"),
        "stderr: {stderr}"
    );
    assert!(
        stderr.contains("P1.mk") && stderr.contains("P2.mk"),
        "both candidates are named: {stderr}"
    );
}

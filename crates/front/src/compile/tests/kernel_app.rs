//! **P1-F2b2 判据**：strict + 闸门开（线程局部覆盖，不碰进程环境）时，应用走内核 `elab_app`。
//!
//! 计数用线程局部的 `TEST_SOLVED` / `TEST_FALLBACK`（全局原子计数会被并行测试串扰）；编译跑在
//! `on_compile_stack` 的新线程里 ⇒ 覆盖与读数都在那个线程内做。

use super::kernel_app::{TEST_FALLBACK, TEST_OVERRIDE, TEST_SOLVED};
use super::tests::on_compile_stack;
use super::{compile_fol_with, CompileOptions, Dialect, PreludeMode};
use crate::parse;

const HEAD: &str = "\
axiom U : Type
axiom P : U -> Prop
axiom u : U
axiom hu : P u
inductive Ex (A : Type) (q : A -> Prop) : Prop
  ctor intro (w : A) (hw : q w) : Ex A q
end
";

/// `(errors, solved, fallback)`：在新编译栈线程里、闸门强制为 `on`、strict 方言下编 `src`。
fn run(src: &str, on: bool) -> (Vec<String>, u64, u64) {
    let src = src.to_string();
    on_compile_stack(move || {
        TEST_OVERRIDE.with(|c| c.set(Some(on)));
        TEST_SOLVED.with(|c| c.set(0));
        TEST_FALLBACK.with(|c| c.set(0));
        let file = parse(&src).unwrap_or_else(|e| panic!("parse: {e:?}\n{src}"));
        let options = CompileOptions {
            prelude: PreludeMode::Full,
            dialect: Dialect::Strict,
        };
        let out = compile_fol_with(&file, &options);
        let errors = out.errors.iter().map(|e| format!("{e:?}")).collect();
        (
            errors,
            TEST_SOLVED.with(|c| c.get()),
            TEST_FALLBACK.with(|c| c.get()),
        )
    })
}

#[test]
fn exists_intro_against_typed_goal_goes_through_kernel() {
    let src = format!("{HEAD}theorem t1 : Ex U P := Ex.intro u hu\n");
    let (errors, solved, _) = run(&src, true);
    assert_eq!(errors, Vec::<String>::new());
    assert!(solved > 0, "Ex.intro u hu 应由内核解出（solved={solved}）");
    // 反向：闸门关 ⇒ 一次都不试，判卷同样通过。
    let (errors, solved, fallback) = run(&src, false);
    assert_eq!(errors, Vec::<String>::new());
    assert_eq!((solved, fallback), (0, 0), "闸门关 ⇒ 内核路径一行不跑");
}

#[test]
fn and_intro_under_fun_binders_goes_through_kernel() {
    let src = "theorem t2 : (a : Prop) -> (b : Prop) -> a -> b -> And a b := \
               fun (a : Prop) => fun (b : Prop) => fun (h1 : a) => fun (h2 : b) => And.intro h1 h2\n";
    let (errors, solved, _) = run(src, true);
    assert_eq!(errors, Vec::<String>::new());
    assert!(
        solved > 0,
        "fun 下的 And.intro h1 h2 应由内核解出（solved={solved}）"
    );
}

#[test]
fn unsolved_implicit_falls_back_to_old_path() {
    // `A` 只出现在隐式位、结果里也没有 ⇒ 内核报 `Unsolved` ⇒ 回落老路（诊断仍由老路给）。
    let src = "axiom g : {A : Type} -> Nat -> Prop\ndef t3 : Prop := g 0\n";
    let (errors_on, solved, fallback) = run(src, true);
    assert_eq!(solved, 0);
    assert!(fallback > 0, "应计一次回落（fallback={fallback}）");
    let (errors_off, _, _) = run(src, false);
    assert_eq!(errors_on, errors_off, "P1：回落后诊断与闸门关逐字相同");
}

#[test]
fn failed_arg_elaboration_does_not_leak_binders_into_scope() {
    // `Ex.elim2 h (fun b hb => …)`：内核路径先 elaborate 实参 `fun …`（无期望类型）⇒ 失败、回落。
    // `elab_expr` 的 λ 臂在 `?` 早退时不 truncate ⇒ 若回落前不恢复深度，老路在多出的 `b, hb` 下
    // elaborate ⇒ `rejected: loose bvar in infer`（Rel.inv_comp_apply 的最小形）。
    let src = format!(
        "{HEAD}axiom Ex.elim2 : {{A : Type}} -> {{q : A -> Prop}} -> {{b : Prop}} -> Ex A q -> ((a : A) -> q a -> b) -> b\n\
         theorem r1 {{B : Type}} (r : B -> Prop) (s : B -> Prop) (h : Ex B (fun (x : B) => And (r x) (s x))) : \
         Ex B (fun (x : B) => And (s x) (r x)) :=\n  \
         Ex.elim2 h (fun (b : B) (hb : And (r b) (s b)) => Ex.intro b (And.intro (And.right hb) (And.left hb)))\n"
    );
    // P2-10b 起这条带注解 λ 会被推迟、体对着域解出（不回落）⇒ 这里关掉 P2-10b，仍测回落路径的作用域纪律
    // （开着的行为见 `kernel_app_lambda_tests`）。
    let (errors, solved, fallback) = super::kernel_app_lambda_tests::run_typed_lambda(&src, false);
    assert_eq!(errors, Vec::<String>::new());
    assert!(
        solved > 0 && fallback > 0,
        "solved={solved} fallback={fallback}"
    );
}

/// **P2-1**：λ 实参带**无注解** binder（`fun b hb => …`）⇒ 内核路径推迟它、按域 elaborate（Lean
/// `elabAndAddNewArg`）⇒ `Ex.elim2` 这一层也由内核解出；判卷与闸门关逐字相同。
fn untyped_lambda_src() -> String {
    format!(
        "{HEAD}axiom Ex.elim2 : {{A : Type}} -> {{q : A -> Prop}} -> {{b : Prop}} -> Ex A q -> ((a : A) -> q a -> b) -> b\n\
         theorem r2 {{B : Type}} (r : B -> Prop) (h : Ex B r) : Ex B r :=\n  \
         Ex.elim2 h (fun a ha => Ex.intro a ha)\n"
    )
}

/// 同 [`run`]（闸门强制开），另外设推迟开关（`postpone = false` ⇒ P2-1 之前的行为）。
fn run_postpone(src: &str, postpone: bool) -> (Vec<String>, u64, u64) {
    let src = src.to_string();
    on_compile_stack(move || {
        super::kernel_app_postpone::TEST_NO_POSTPONE.with(|c| c.set(!postpone));
        TEST_OVERRIDE.with(|c| c.set(Some(true)));
        TEST_SOLVED.with(|c| c.set(0));
        TEST_FALLBACK.with(|c| c.set(0));
        let file = parse(&src).unwrap_or_else(|e| panic!("parse: {e:?}\n{src}"));
        let options = CompileOptions {
            prelude: PreludeMode::Full,
            dialect: Dialect::Strict,
        };
        let out = compile_fol_with(&file, &options);
        let errors = out.errors.iter().map(|e| format!("{e:?}")).collect();
        (
            errors,
            TEST_SOLVED.with(|c| c.get()),
            TEST_FALLBACK.with(|c| c.get()),
        )
    })
}

#[test]
fn untyped_lambda_arg_is_postponed_and_solved_by_kernel() {
    let src = untyped_lambda_src();
    let (errors_off, _, _) = run(&src, false);
    assert_eq!(errors_off, Vec::<String>::new());
    let (errors, solved, _) = run_postpone(&src, true);
    assert_eq!(errors, errors_off, "判卷与闸门关逐字相同");
    // 外层 `Ex.elim2 h (fun a ha => …)` 由内核解出（内层 `Ex.intro a ha` 在 λ 体里只有内核期望、
    // 没有源级期望 ⇒ 仍可能回落老路，不在本条判据内）。
    let (_, solved_np, _) = run_postpone(&src, false);
    assert!(
        solved > solved_np,
        "推迟开 ⇒ 内核多解出外层应用（on={solved} off={solved_np}）"
    );
}

#[test]
fn untyped_lambda_arg_falls_back_without_postponement() {
    // 反向验证：推迟关掉 ⇒ 无注解 λ 的应用回落（P2-1 之前的行为），判卷仍由老路给出。
    let src = untyped_lambda_src();
    let (errors, solved_np, fallback) = run_postpone(&src, false);
    assert_eq!(errors, Vec::<String>::new());
    assert!(fallback > 0, "关掉推迟 ⇒ 必须回落（fallback={fallback}）");
    let (_, solved, _) = run_postpone(&src, true);
    assert!(solved > solved_np, "on={solved} off={solved_np}");
}

/// **P2-2**：要两轮的推迟。`K2 p h u`：`p`、`h` 都是无注解 λ（推迟）；第 1 轮 `p` 的域 `U -> Prop` 就绪、
/// `h` 的域 `(x : U) -> ?p x` 卡在占位元变量上；补上 `p` ⇒ 第 2 轮 `h` 的域就绪 ⇒ 内核解出。
fn two_round_src() -> String {
    format!(
        "{HEAD}axiom hP : (x : U) -> P x\n\
         axiom K2 : {{α : Type}} -> (p : U -> Prop) -> (h : (x : U) -> p x) -> α -> Prop\n\
         def k2 : Prop := K2 (fun x => P x) (fun x => hP x) u\n"
    )
}

/// 同 [`run`]（闸门强制开），另外把推迟轮数上限设成 `rounds`（`None` = 生产值）。
fn run_rounds(src: &str, rounds: Option<usize>) -> (Vec<String>, u64, u64) {
    let src = src.to_string();
    on_compile_stack(move || {
        super::kernel_app_postpone::TEST_MAX_ROUNDS.with(|c| c.set(rounds));
        TEST_OVERRIDE.with(|c| c.set(Some(true)));
        TEST_SOLVED.with(|c| c.set(0));
        TEST_FALLBACK.with(|c| c.set(0));
        let file = parse(&src).unwrap_or_else(|e| panic!("parse: {e:?}\n{src}"));
        let options = CompileOptions {
            prelude: PreludeMode::Full,
            dialect: Dialect::Strict,
        };
        let out = compile_fol_with(&file, &options);
        let errors = out.errors.iter().map(|e| format!("{e:?}")).collect();
        (
            errors,
            TEST_SOLVED.with(|c| c.get()),
            TEST_FALLBACK.with(|c| c.get()),
        )
    })
}

#[test]
fn blocked_postponed_arg_is_solved_in_second_round() {
    let src = two_round_src();
    let (errors_off, _, _) = run(&src, false);
    assert_eq!(errors_off, Vec::<String>::new());
    let (errors, solved, fallback) = run_rounds(&src, None);
    assert_eq!(errors, errors_off, "判卷与闸门关逐字相同");
    assert_eq!(fallback, 0, "多轮推迟 ⇒ K2 不回落（solved={solved}）");
    // 反向：只许 1 轮（P2-1 的行为）⇒ `h` 一直卡着 ⇒ 回落。
    let (errors1, solved1, fallback1) = run_rounds(&src, Some(1));
    assert_eq!(errors1, errors_off);
    assert!(
        fallback1 > 0 && solved > solved1,
        "1 轮 ⇒ 必须回落（rounds=∞: solved={solved}；rounds=1: solved={solved1} fallback={fallback1}）"
    );
}

/// **P2-2 (c)**：零元隐式记法 `∅` 作推迟位，只有内核给的域 ⇒ 按域解（不回落）。
#[test]
fn zero_ary_implicit_notation_reads_kernel_domain() {
    let src = "def Set (α : Type) : Type := α -> Prop\n\
               def Set.empty {α : Type} : Set α := fun (x : α) => False\n\
               notation \"∅\" => Set.empty\n\
               def Set.mem {α : Type} (a : α) (s : Set α) : Prop := s a\n\
               axiom U : Type\naxiom u : U\n\
               def m : Prop := Set.mem u ∅\n";
    let (errors_off, _, _) = run(src, false);
    assert_eq!(errors_off, Vec::<String>::new());
    let (errors, solved, fallback) = run(src, true);
    assert_eq!(errors, errors_off);
    assert_eq!(fallback, 0, "Set.mem u ∅ 不回落（solved={solved}）");
}

/// **P2-3 (1)**：推迟位是**全带注解**的 λ、体要期望（`∅`）⇒ 内核先拿 binder 类型对齐域（Lean
/// `Binders.lean:417` `propagateExpectedType`）⇒ `α` 在体之前就解出 ⇒ `Set.preimage` 不回落。
const TYPED_LAMBDA_SRC: &str = "def Set (α : Type) : Type := α -> Prop\n\
     def Set.empty {α : Type} : Set α := fun (x : α) => False\n\
     notation \"∅\" => Set.empty\n\
     def Set.univ {α : Type} : Set α := fun (x : α) => True\n\
     def Set.mem {α : Type} (a : α) (s : Set α) : Prop := s a\n\
     def Set.preimage {α β : Type} (f : α -> β) (B : Set β) : Set α := fun (x : α) => B (f x)\n\
     axiom U : Type\naxiom u : U\n\
     def p : Prop := Set.mem u (Set.preimage (fun (x : U) => ∅) (@Set.univ (Set U)))\n";

/// 同 [`run`]（闸门强制开），另外设 P2-3 binder 合一开关（`unify = false` ⇒ P2-3 之前的行为）。
fn run_binder_unify(src: &str, unify: bool) -> (Vec<String>, u64, u64) {
    let src = src.to_string();
    on_compile_stack(move || {
        super::kernel_app_postpone::TEST_NO_BINDER_UNIFY.with(|c| c.set(!unify));
        TEST_OVERRIDE.with(|c| c.set(Some(true)));
        TEST_SOLVED.with(|c| c.set(0));
        TEST_FALLBACK.with(|c| c.set(0));
        let file = parse(&src).unwrap_or_else(|e| panic!("parse: {e:?}\n{src}"));
        let options = CompileOptions {
            prelude: PreludeMode::Full,
            dialect: Dialect::Strict,
        };
        let out = compile_fol_with(&file, &options);
        let errors = out.errors.iter().map(|e| format!("{e:?}")).collect();
        (
            errors,
            TEST_SOLVED.with(|c| c.get()),
            TEST_FALLBACK.with(|c| c.get()),
        )
    })
}

#[test]
fn typed_lambda_binders_solve_implicit_before_body() {
    let (errors_off, _, _) = run(TYPED_LAMBDA_SRC, false);
    assert_eq!(errors_off, Vec::<String>::new());
    let (errors, solved, fallback) = run_binder_unify(TYPED_LAMBDA_SRC, true);
    assert_eq!(errors, errors_off, "判卷与闸门关逐字相同");
    assert_eq!(
        fallback, 0,
        "带注解 λ 的 binder 类型解出 α ⇒ 不回落（solved={solved}）"
    );
}

/// **P2-3 (2)**：`∩` 的操作数是带注解 λ 的应用（unit12 的 `(f '' A) ∩ (f '' B)` 形状）。P2-2 时操作数的 `α`
/// 卡死 ⇒ 操作数回落、`∩` 跟着 argelab 回落；(1) 之后操作数第一趟就解出 ⇒ `∩` 与外层都不回落 ⇒ (2) 不需要新代码。
const NOTATION_OPERANDS_SRC: &str = "def Set (α : Type) : Type := α -> Prop\n\
     def Set.empty {α : Type} : Set α := fun (x : α) => False\n\
     notation \"∅\" => Set.empty\n\
     def Set.inter {α : Type} (A B : Set α) : Set α := fun (x : α) => And (A x) (B x)\n\
     infixl:70 \" ∩ \" => Set.inter\n\
     def Set.mem {α : Type} (a : α) (s : Set α) : Prop := s a\n\
     def Set.preimage {α β : Type} (f : α -> β) (B : Set β) : Set α := fun (x : α) => B (f x)\n\
     axiom U : Type\naxiom u : U\naxiom S : Set (Set U)\n\
     def q : Prop := Set.mem (@Set.empty U) (Set.preimage (fun (x : Set U) => ∅) S ∩ Set.preimage (fun (x : Set U) => ∅) S)\n";

#[test]
fn notation_over_typed_lambda_operands_does_not_fall_back() {
    let (errors_off, _, _) = run(NOTATION_OPERANDS_SRC, false);
    assert_eq!(errors_off, Vec::<String>::new());
    let (errors, solved, fallback) = run_binder_unify(NOTATION_OPERANDS_SRC, true);
    assert_eq!(errors, errors_off, "判卷与闸门关逐字相同");
    assert_eq!(
        fallback, 0,
        "∩ 的操作数第一趟解出 ⇒ 不回落（solved={solved}）"
    );
    // 反向：关掉 (1) ⇒ 操作数 `α` 卡死 ⇒ 回落。
    let (errors_np, _, fallback_np) = run_binder_unify(NOTATION_OPERANDS_SRC, false);
    assert_eq!(errors_np, errors_off);
    assert!(
        fallback_np > 0,
        "关掉 binder 合一 ⇒ 必须回落（fallback={fallback_np}）"
    );
}

#[test]
fn typed_lambda_without_binder_unify_falls_back() {
    // 反向验证：不交 binder 类型 ⇒ `α` 只能从体来、体 `∅` 又要期望 ⇒ 卡死、回落（P2-2 的行为）。
    let (errors, _, fallback) = run_binder_unify(TYPED_LAMBDA_SRC, false);
    assert_eq!(errors, Vec::<String>::new());
    assert!(
        fallback > 0,
        "关掉 binder 合一 ⇒ 必须回落（fallback={fallback}）"
    );
}

/// **P2-4**：`a ≠ b`，只有第二个操作数定类型（unit12 `flawed_equalities_refuted` 的形状）。`≠` ⇒ `Ne α a b`
/// （`α` 显式、由记法补）走内核：`@Set.univ (Set U)` 第一趟解出 `α`，`(fun _ => ∅) '' S` 推迟、按域 elaborate。
const NE_SECOND_OPERAND_SRC: &str = "def Set (α : Type) : Type := α -> Prop\n\
     def Set.empty {α : Type} : Set α := fun (x : α) => False\n\
     notation \"∅\" => Set.empty\n\
     def Set.univ {α : Type} : Set α := fun (x : α) => True\n\
     def Set.image {α β : Type} (f : α -> β) (A : Set α) : Set β := fun (y : β) => True\n\
     infixr:80 \" '' \" => Set.image\n\
     axiom U : Type\naxiom S : Set (Set U)\n\
     theorem ne2 : (fun (x : Set U) => ∅) '' S ≠ @Set.univ (Set U) := sorry\n";

/// 同 [`run`]（闸门强制开），另外设 P2-4 记法路由开关（`route = false` ⇒ P2-4 之前的行为）。
fn run_notation(src: &str, route: bool) -> (Vec<String>, u64, u64) {
    let src = src.to_string();
    on_compile_stack(move || {
        super::kernel_app_notation::TEST_NO_NOTATION.with(|c| c.set(!route));
        // §9.39：反向臂（关路由）要的是"修前老路"的拒绝 —— 那条拒绝来自老路的层级缺陷
        // （`universe_level_text_of_operands` 遇到推不出的操作数就整体放弃）⇒ 一并回到修前行为。
        super::elab::p2l_level_operands_tests::TEST_LEVEL_ABORT_ON_UNKNOWN.with(|c| c.set(!route));
        TEST_OVERRIDE.with(|c| c.set(Some(true)));
        TEST_SOLVED.with(|c| c.set(0));
        TEST_FALLBACK.with(|c| c.set(0));
        let file = parse(&src).unwrap_or_else(|e| panic!("parse: {e:?}\n{src}"));
        let options = CompileOptions {
            prelude: PreludeMode::Full,
            dialect: Dialect::Strict,
        };
        let out = compile_fol_with(&file, &options);
        let errors = out.errors.iter().map(|e| format!("{e:?}")).collect();
        (
            errors,
            TEST_SOLVED.with(|c| c.get()),
            TEST_FALLBACK.with(|c| c.get()),
        )
    })
}

#[test]
fn ne_typed_by_second_operand_goes_through_kernel() {
    let (errors, solved, fallback) = run_notation(NE_SECOND_OPERAND_SRC, true);
    assert_eq!(errors, Vec::<String>::new(), "记法走内核 ⇒ 接受");
    // 反向：关掉记法路由 ⇒ 老路先无期望地定第一个操作数的类型 ⇒ 判红（P2-4 之前 unit12 的拒绝）。
    let (errors_off, solved_off, fallback_off) = run_notation(NE_SECOND_OPERAND_SRC, false);
    assert!(
        !errors_off.is_empty(),
        "关掉记法路由 ⇒ 必须判红（实际 0 条诊断）"
    );
    // 剩下的 2 次回落（探针实测，已还原）：都是 `Set.image (fun (x : Set U) => ∅) S`、2 个实参、
    // `Unsolved ["β"]` —— `Ne` 内核 `run` 的**第一趟无期望** elaborate 第一个操作数（P2-1 的"先试、失败再推迟"），
    // 每次 elaborate 类型各一次（共 2 次）。随后它被推迟、按域 `Set (Set U)` 再 elaborate ⇒ 解出（`Ne` Solved）。
    // 不是记法路由的 bug；但那次老路回落会做 `judge_infer`（未命中计 `unsolved`）⇒ 是剩余的工作量来源。
    // ⇒ 不要求 0，要求少于老路（关路由时同一个 `''` 回落 5 次）。
    assert!(
        solved > solved_off && fallback < fallback_off,
        "≠ 走内核：on solved={solved} fallback={fallback}；off solved={solved_off} fallback={fallback_off}"
    );
}

/// **P2-5**：同 [`NE_SECOND_OPERAND_SRC`] 的形状（名字另起 ⇒ 不与别的测试共享 `judge_infer` 缓存）。`≠` 的内核
/// 第一趟无期望地 elaborate 第一个操作数 `(fun _ => ∅) '' S25`：它自己走内核、`Unsolved ["β"]`。P2-5 之前它接着回落
/// 老路（`judge_infer` 未命中，归因 `unsolved`），然后才被推迟、按域解出 ⇒ 那次未命中白做。
const PROBE_SRC: &str = "def Set (α : Type) : Type := α -> Prop\n\
     def Set.empty {α : Type} : Set α := fun (x : α) => False\n\
     notation \"∅\" => Set.empty\n\
     def Set.univ {α : Type} : Set α := fun (x : α) => True\n\
     def Set.image {α β : Type} (f : α -> β) (A : Set α) : Set β := fun (y : β) => True\n\
     infixr:80 \" '' \" => Set.image\n\
     axiom U25 : Type\naxiom S25 : Set (Set U25)\n\
     theorem ne25 : (fun (x : Set U25) => ∅) '' S25 ≠ @Set.univ (Set U25) := sorry\n";

/// `(errors, solved, fallback, unsolved 未命中)`：闸门强制开、strict；`probe = false` ⇒ P2-5 之前的行为。
fn run_probe(src: &str, probe: bool) -> (Vec<String>, u64, u64, u64) {
    let src = src.to_string();
    on_compile_stack(move || {
        super::kernel_app_probe::TEST_NO_PROBE.with(|c| c.set(!probe));
        super::kernel_app_miss::TEST_UNSOLVED_MISSES.with(|c| c.set(0));
        TEST_OVERRIDE.with(|c| c.set(Some(true)));
        TEST_SOLVED.with(|c| c.set(0));
        TEST_FALLBACK.with(|c| c.set(0));
        let file = parse(&src).unwrap_or_else(|e| panic!("parse: {e:?}\n{src}"));
        let options = CompileOptions {
            prelude: PreludeMode::Full,
            dialect: Dialect::Strict,
        };
        let out = compile_fol_with(&file, &options);
        let errors = out.errors.iter().map(|e| format!("{e:?}")).collect();
        (
            errors,
            TEST_SOLVED.with(|c| c.get()),
            TEST_FALLBACK.with(|c| c.get()),
            super::kernel_app_miss::TEST_UNSOLVED_MISSES.with(|c| c.get()),
        )
    })
}

#[test]
fn probe_pass_skips_speculative_old_path_for_nested_arg() {
    // 基准 = 闸门开、探测关（P2-5 之前）。⚠ 不能拿闸门关作基准：这个形状闸门关时本来就判红（P2-4 的
    // reject→accept，见 `ne_typed_by_second_operand_goes_through_kernel`；单独进程实测同样 exit 1）。
    let (errors_base, _, _, _) = run_probe(PROBE_SRC, false);
    assert_eq!(errors_base, Vec::<String>::new());
    let (errors, solved, _, misses) = run_probe(PROBE_SRC, true);
    assert_eq!(errors, errors_base, "探测不改判卷");
    assert!(solved > 0, "≠ 由内核解出（solved={solved}）");
    assert_eq!(
        misses, 0,
        "探测中的内层应用不回落老路 ⇒ 0 次 unsolved 未命中"
    );
}

#[test]
fn probe_pass_off_pays_unsolved_miss() {
    // 反向验证：关掉探测（P2-5 之前）⇒ 内层 `''` 在第一趟里回落老路 ⇒ 至少一次 unsolved 未命中；判卷不变。
    let (errors, _, fallback, misses) = run_probe(PROBE_SRC, false);
    assert_eq!(errors, Vec::<String>::new());
    assert!(
        misses > 0 && fallback > 0,
        "关掉探测 ⇒ 必须付出 unsolved 未命中（misses={misses} fallback={fallback}）"
    );
    let (_, _, fallback_on, _) = run_probe(PROBE_SRC, true);
    assert!(
        fallback_on <= fallback,
        "探测只减不增：on={fallback_on} off={fallback}"
    );
}

/// **P2-5 回归**（unit12 `flawed_equalities_refuted` 第二个合取支的自包含最小形 c2）：探测把某个内层应用的老路
/// 换成推迟后，外层没解出 ⇒ 外层更高一层回落老路 ⇒ 撞老路的 `Sort(0)/Sort(1)` 拒绝。修法：切过且外层不是
/// `Solved` ⇒ 关掉切、整段重做一次（= 今天的行为）。base（P2-4，闸门开）单独进程实测接受。
pub(super) const C2_SRC: &str = "def Set (α : Type) : Type := α -> Prop\n\
     def Set.empty {α : Type} : Set α := fun (x : α) => False\n\
     notation \"∅\" => Set.empty\n\
     def Set.univ {α : Type} : Set α := fun (x : α) => True\n\
     def Set.singleton {α : Type} (a : α) : Set α := fun (x : α) => x = a\n\
     def Set.inter {α : Type} (A B : Set α) : Set α := fun (x : α) => And (A x) (B x)\n\
     infixl:70 \" ∩ \" => Set.inter\n\
     def Set.image {α β : Type} (f : α -> β) (A : Set α) : Set β := fun (y : β) => True\n\
     infixr:80 \" '' \" => Set.image\n\
     theorem c2 : (fun (_ : Set Nat) => ∅) '' ({∅} ∩ {(@Set.univ Nat)}) ≠ \
     ((fun (_ : Set Nat) => ∅) '' {∅}) ∩ ((fun (_ : Set Nat) => ∅) '' {(@Set.univ Nat)}) := sorry\n";

/// 同 [`run_probe`]，另外设重做开关（`redo = false` ⇒ 切过也不重做）。
fn run_redo(src: &str, redo: bool) -> Vec<String> {
    let src = src.to_string();
    on_compile_stack(move || {
        super::kernel_app_probe::TEST_NO_REDO.with(|c| c.set(!redo));
        // §9.39：同上 —— 不重做的反向臂吃的是老路层级缺陷的拒绝 ⇒ 回到修前行为。
        super::elab::p2l_level_operands_tests::TEST_LEVEL_ABORT_ON_UNKNOWN.with(|c| c.set(!redo));
        TEST_OVERRIDE.with(|c| c.set(Some(true)));
        let file = parse(&src).unwrap_or_else(|e| panic!("parse: {e:?}\n{src}"));
        let options = CompileOptions {
            prelude: PreludeMode::Full,
            dialect: Dialect::Strict,
        };
        let out = compile_fol_with(&file, &options);
        out.errors.iter().map(|e| format!("{e:?}")).collect()
    })
}

#[test]
fn probe_cut_then_unsolved_outer_redoes_without_probe() {
    assert_eq!(
        run_redo(C2_SRC, true),
        Vec::<String>::new(),
        "闸门开 + 探测 + 重做 ⇒ 与 base 同判：接受"
    );
}

#[test]
fn probe_cut_without_redo_rejects_c2() {
    // 反向验证：不重做 ⇒ 切掉的老路结果找不回来 ⇒ 判红（提交 83ae03ac 的 unit12 回归）。
    assert!(
        !run_redo(C2_SRC, false).is_empty(),
        "关掉重做 ⇒ 必须判红（否则本测试咬不住 c2 回归）"
    );
}

/// **P2-11a-A**：裸隐式常量实参（`a ∈ univ`：`univ : {α} → Set α`）推迟到域就绪再 elaborate（Lean
/// `elabAndAddNewArg` + `addImplicitArg`）⇒ `Set.mem` / `Set.subset` 由内核解出，不再 `ArgMismatch` 回落。
const BARE_IMPLICIT_SRC: &str = "def Set (α : Type) : Type := α -> Prop\n\
     def Set.univ {α : Type} : Set α := fun (x : α) => True\n\
     def Set.mem {α : Type} (a : α) (s : Set α) : Prop := s a\n\
     infix:50 \" ∈ \" => Set.mem\n\
     def Set.subset {α : Type} (A B : Set α) : Prop := (x : α) -> Set.mem x A -> Set.mem x B\n\
     infix:50 \" ⊆ \" => Set.subset\n\
     theorem m1 {α : Type} (a : α) : a ∈ Set.univ := True.intro\n\
     theorem s1 {α : Type} (A : Set α) : A ⊆ Set.univ := fun (x : α) (h : x ∈ A) => True.intro\n";

/// 同 [`run`]（闸门强制开），另外设裸常量推迟开关（`bare = false` ⇒ P2-11a-A 之前的行为）。
fn run_bare(src: &str, bare: bool) -> (Vec<String>, u64, u64) {
    let src = src.to_string();
    on_compile_stack(move || {
        super::kernel_app_postpone::TEST_NO_BARE_POSTPONE.with(|c| c.set(!bare));
        TEST_OVERRIDE.with(|c| c.set(Some(true)));
        TEST_SOLVED.with(|c| c.set(0));
        TEST_FALLBACK.with(|c| c.set(0));
        let file = parse(&src).unwrap_or_else(|e| panic!("parse: {e:?}\n{src}"));
        let options = CompileOptions {
            prelude: PreludeMode::Full,
            dialect: Dialect::Strict,
        };
        let out = compile_fol_with(&file, &options);
        let errors = out.errors.iter().map(|e| format!("{e:?}")).collect();
        (
            errors,
            TEST_SOLVED.with(|c| c.get()),
            TEST_FALLBACK.with(|c| c.get()),
        )
    })
}

#[test]
fn bare_implicit_const_arg_is_postponed_and_solved() {
    let (errors_off, _, _) = run(BARE_IMPLICIT_SRC, false);
    assert_eq!(errors_off, Vec::<String>::new());
    let (errors, solved, fallback) = run_bare(BARE_IMPLICIT_SRC, true);
    assert_eq!(errors, errors_off, "判卷与闸门关逐字相同");
    assert_eq!(fallback, 0, "a ∈ univ / A ⊆ univ 不回落（solved={solved}）");
    assert!(solved > 0);
}

#[test]
fn bare_implicit_const_arg_falls_back_without_postponement() {
    // 反向验证：关掉裸常量推迟 ⇒ 内核拿到 `{α} → Set α` 对 `Set α` ⇒ ArgMismatch 回落（判卷仍由老路给）。
    let (errors, _, fallback) = run_bare(BARE_IMPLICIT_SRC, false);
    assert_eq!(errors, Vec::<String>::new());
    assert!(fallback > 0, "关掉推迟 ⇒ 必须回落（fallback={fallback}）");
}

#[test]
fn local_shadowing_bare_name_is_not_postponed() {
    // 局部 `univ` 遮蔽同名常量 ⇒ 不是常量 ⇒ 不推迟；开关开/关判卷相同。
    let src = format!(
        "{BARE_IMPLICIT_SRC}theorem l1 {{α : Type}} (a : α) (univ : Set α) (h : a ∈ univ) : a ∈ univ := h\n"
    );
    let on = run_bare(&src, true);
    let off = run_bare(&src, false);
    assert_eq!(on.0, Vec::<String>::new());
    assert_eq!(on.0, off.0);
}

/// **P2-11c**：同 [`run`]（strict），另外设宇宙多态头开关（`heads = false` ⇒ P2-11c 之前的闸门），并读
/// `untried_universe_app_head` 的线程局部计数。返回 `(errors, solved, fallback, untried)`。
fn run_universe(src: &str, on: bool, heads: bool) -> (Vec<String>, u64, u64, u64) {
    run_universe_full(src, on, heads, true)
}

/// 同 [`run_universe`]，另设 `Name.{…}` 头开关（`uapp = false` ⇒ P2-11e 之前）。开关必须在编译线程里设。
fn run_universe_full(src: &str, on: bool, heads: bool, uapp: bool) -> (Vec<String>, u64, u64, u64) {
    use super::kernel_app_hint::{
        TEST_NO_UNIVERSE_APP, TEST_NO_UNIVERSE_HEADS, TEST_UNTRIED_UNIVERSE_APP,
    };
    let src = src.to_string();
    on_compile_stack(move || {
        TEST_NO_UNIVERSE_HEADS.with(|c| c.set(!heads));
        TEST_NO_UNIVERSE_APP.with(|c| c.set(!uapp));
        TEST_UNTRIED_UNIVERSE_APP.with(|c| c.set(0));
        TEST_OVERRIDE.with(|c| c.set(Some(on)));
        TEST_SOLVED.with(|c| c.set(0));
        TEST_FALLBACK.with(|c| c.set(0));
        let file = parse(&src).unwrap_or_else(|e| panic!("parse: {e:?}\n{src}"));
        let options = CompileOptions {
            prelude: PreludeMode::Full,
            dialect: Dialect::Strict,
        };
        let out = compile_fol_with(&file, &options);
        let errors = out.errors.iter().map(|e| format!("{e:?}")).collect();
        (
            errors,
            TEST_SOLVED.with(|c| c.get()),
            TEST_FALLBACK.with(|c| c.get()),
            TEST_UNTRIED_UNIVERSE_APP.with(|c| c.get()),
        )
    })
}

/// `Eq.symm h`：头 `Eq.symm` 是宇宙多态的 `Ident`（`=` 只在签名里，值里没有记法）。
const EQ_SYMM_SRC: &str = "theorem s1 (a b : Nat) (h : a = b) : b = a := Eq.symm h\n";
/// `Eq.subst h hp`：宇宙多态头 + 高阶隐式 `p`（`?p a =?= P a` 的模式合一）。
const EQ_SUBST_SRC: &str =
    "theorem s2 (P : Nat -> Prop) (a b : Nat) (h : a = b) (hp : P a) : P b := Eq.subst h hp\n";

#[test]
fn universe_polymorphic_heads_are_solved_by_kernel() {
    let (errors_off, ..) = run_universe(EQ_SYMM_SRC, false, true);
    assert_eq!(errors_off, Vec::<String>::new());
    let (errors, solved, fallback, _) = run_universe(EQ_SYMM_SRC, true, true);
    assert_eq!(errors, errors_off, "判卷与闸门关逐字相同");
    assert_eq!(fallback, 0, "Eq.symm h 不回落（solved={solved}）");
    assert!(solved > 0);
}

#[test]
fn higher_order_motive_head_stays_gated() {
    // `Eq.subst` 的 motive 是高阶隐式（内核无 motive 推断，试了只会 Unsolved 再付一遍老路）⇒ 仍挡在门外，
    // 计 `fallback_unsupported_universe_poly_ho_motive`；判卷与闸门关逐字相同。
    let (errors_off, ..) = run_universe(EQ_SUBST_SRC, false, true);
    assert_eq!(errors_off, Vec::<String>::new());
    // 签名里的 `=`（P2-11e 起进内核）不算 ⇒ 关掉 `Name.{…}` 头，只看 `Eq.subst`。
    let (errors, solved, fallback, _) = run_universe_full(EQ_SUBST_SRC, true, true, false);
    assert_eq!(errors, errors_off);
    assert_eq!((solved, fallback), (0, 1), "Eq.subst 挡一次、不进内核");
    assert_eq!(
        super::kernel_app_hint::universe_gate("Eq.subst", &["u".to_string()]),
        Some("universe-poly ho motive")
    );
}

#[test]
fn universe_polymorphic_heads_fall_back_with_old_gate() {
    // 反向验证：`TEST_NO_UNIVERSE_HEADS` 恢复 P2-11c 之前的闸门 ⇒ 宇宙多态头回落、不再 solved。
    for src in [EQ_SYMM_SRC, EQ_SUBST_SRC] {
        let (errors, solved, fallback, _) = run_universe(src, true, false);
        assert_eq!(errors, Vec::<String>::new(), "老路判卷仍通过：{src}");
        assert_eq!(solved, 0, "{src}");
        assert!(
            fallback > 0,
            "旧闸门 ⇒ 必须回落（fallback={fallback}）：{src}"
        );
    }
}

/// **P2-11e**：同 [`run_universe`]（宇宙多态头放行），另设 `Name.{…}` 头开关（`uapp = false` ⇒ P2-11e 之前）。
fn run_uapp(src: &str, on: bool, uapp: bool) -> (Vec<String>, u64, u64, u64) {
    run_universe_full(src, on, true, uapp)
}

/// 最小 `Set`（同 `p2l_level_operands` 的夹具）：m5 族的层只能由右边定。
const UAPP_LIB: &str = "def Set (α : Type) : Type := α → Prop\n\
     def Set.empty {α : Type} : Set α := fun (x : α) => False\n\
     def Set.univ {α : Type} : Set α := fun (x : α) => True\n\
     def Set.singleton {α : Type} (a : α) : Set α := fun (x : α) => x = a\n\
     notation \"∅\" => Set.empty\n";

/// `=` / `≠` / 只由右边定层的 `=`（m5 族）/ 用户手写 `Eq.{1}`。
fn uapp_cases() -> Vec<String> {
    vec![
        "def e1 (a b : Nat) : Prop := a = b\n".to_string(),
        "def e2 (a b : Nat) : Prop := a ≠ b\n".to_string(),
        "theorem e3 (a b : Nat) (h : a = b) : b = a := Eq.symm h\n".to_string(),
        format!("{UAPP_LIB}theorem m5 : {{∅}} = @Set.univ (Set Nat) := sorry\n"),
        // m5 的左边写成点名（无需期望就能定型）⇒ 回落 0（= 只有库时的基线）：m5 剩下的回落都在 `{∅}`。
        format!(
            "{UAPP_LIB}theorem m5v : (@Set.singleton (Set Nat) ∅) = @Set.univ (Set Nat) := sorry\n"
        ),
        format!("{UAPP_LIB}def e4 (A : Set Nat) : Prop := Eq.{{1}} A A\n"),
    ]
}

#[test]
fn universe_app_heads_are_solved_by_kernel() {
    for src in uapp_cases() {
        let (errors_off, ..) = run_uapp(&src, false, true);
        assert_eq!(errors_off, Vec::<String>::new(), "{src}");
        // 基线 = P2-11e 之前（`Name.{…}` 头不试）。m5 的 `{∅}` 内层应用本来就回落（与本片无关）⇒ 比基线。
        let base = run_uapp(&src, true, false);
        let (errors, solved, fallback, untried) = run_uapp(&src, true, true);
        assert_eq!(errors, errors_off, "判卷与闸门关逐字相同：{src}");
        assert_eq!(untried, 0, "`Name.{{…}}` 头全部进内核路径：{src}");
        if src.contains("theorem m5 ") {
            // 实测（P2-11e）：基线 5 次回落、新 4 次，全是 `{∅}`（`Set.singleton`，unsolved，`{∅}` 的 span），
            // 没有一次是 `Eq` 头；`Eq.{1}` 头进内核后少一趟老路里的 `{∅}` 重试 ⇒ 必须严格少于基线。
            assert!(
                fallback < base.2,
                "m5 回落严格少于基线（{fallback} vs {}）",
                base.2
            );
        } else {
            // 其余用例实测基线与新都是 0 回落。
            assert_eq!((fallback, base.2), (0, 0), "{src}");
        }
        assert!(solved > 0, "{src}");
        // `≠`（`Ne` 的 `α` 显式）走 P2-4 记法路径、本来就不是 `Name.{…}` 头；其余基线里都有未试的头。
        if !src.contains('≠') {
            assert!(base.3 > 0, "基线里有未试的 `Name.{{…}}` 头：{src}");
            assert!(
                solved > base.1,
                "多解出的就是 `Name.{{…}}` 头（{solved} vs 基线 {}）：{src}",
                base.1
            );
        }
    }
}

#[test]
fn universe_app_judgement_unchanged_in_loose() {
    // loose 档：闸门（`effective`）只认 strict ⇒ 本路径一行不跑；判卷与 strict 闸门开相同。
    for src in uapp_cases() {
        let file = parse(&src).unwrap();
        let (errors, solved, fallback) = on_compile_stack(move || {
            TEST_OVERRIDE.with(|c| c.set(Some(true)));
            TEST_SOLVED.with(|c| c.set(0));
            TEST_FALLBACK.with(|c| c.set(0));
            let options = CompileOptions {
                prelude: PreludeMode::Full,
                dialect: Dialect::Loose,
            };
            let out = compile_fol_with(&file, &options);
            let errors: Vec<String> = out.errors.iter().map(|e| format!("{e:?}")).collect();
            (
                errors,
                TEST_SOLVED.with(|c| c.get()),
                TEST_FALLBACK.with(|c| c.get()),
            )
        });
        assert_eq!(errors, Vec::<String>::new(), "{src}");
        assert_eq!((solved, fallback), (0, 0), "loose 不走内核路径：{src}");
    }
}

#[test]
fn universe_app_head_is_untried_with_old_gate() {
    // 反向验证：`TEST_NO_UNIVERSE_APP` 恢复 P2-11e 之前 ⇒ `Name.{…}` 头只计观测数、不试；闸门关 ⇒ 一次都不计。
    let src = "def e1 (a b : Nat) : Prop := a = b\n";
    let (errors, solved, fallback, untried) = run_uapp(src, true, false);
    assert_eq!(errors, Vec::<String>::new());
    assert!(untried > 0, "`a = b` 必须被观测到（untried={untried}）");
    assert_eq!((solved, fallback), (0, 0), "观测计数不改控制流");
    let (errors_off, solved, fallback, untried) = run_uapp(src, false, true);
    assert_eq!(errors_off, errors);
    assert_eq!((solved, fallback, untried), (0, 0, 0), "闸门关 ⇒ 一行不跑");
}

#[test]
fn universe_app_bad_shape_stays_untried() {
    // 实参形状不收（洞）的 `=` ⇒ 不试、不计 `fallback_shape`（试了只会立刻回落再付老路），计 untried；
    // 判卷与闸门关相同。实测放开时 13 个文件 `fallback_shape` 0→103。
    // 操作数里嵌一个无注解 λ（不在顶层 ⇒ 不可推迟 ⇒ 形状不收）。
    let src = "def e7 (f : (Nat -> Nat) -> Nat) : Prop := f (fun x => x) = f (fun x => x)\n";
    let (errors_off, ..) = run_uapp(src, false, true);
    let (errors, _, fallback, untried) = run_uapp(src, true, true);
    assert_eq!(errors, errors_off);
    assert_eq!(fallback, 0, "形状不收的 `Name.{{…}}` 头不回落");
    assert!(
        untried > 0,
        "形状不收的 `=` 计 untried（untried={untried}）"
    );
}

#[test]
fn universe_app_wrong_arity_stays_untried() {
    // 层数不符（`Eq.{0, 1}`）⇒ 不试（老路报 `ElabUniverseArity`），计 untried；判卷与闸门关相同。
    let src = "def e5 (a b : Nat) : Prop := Eq.{0, 1} a b\n";
    let (errors_off, ..) = run_uapp(src, false, true);
    assert!(!errors_off.is_empty(), "层数不符必须报错");
    let (errors, solved, fallback, untried) = run_uapp(src, true, true);
    assert_eq!(errors, errors_off);
    assert_eq!((solved, fallback), (0, 0));
    assert!(untried > 0);
}

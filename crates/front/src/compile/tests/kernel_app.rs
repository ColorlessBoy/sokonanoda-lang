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
    let (errors, solved, fallback) = run(&src, true);
    assert_eq!(errors, Vec::<String>::new());
    assert!(
        solved > 0 && fallback > 0,
        "solved={solved} fallback={fallback}"
    );
}

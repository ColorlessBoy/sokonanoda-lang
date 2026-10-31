//! **P2-12 S0 判据**：内核回落后老路结局计数（`kernel_app_census`）。线程局部读数；闸门强制开、strict。

use super::kernel_app::TEST_OVERRIDE;
use super::kernel_app_census::TEST_SETTLED;
use super::kernel_app_lambda::TEST_NO_TYPED_LAMBDA;
use super::tests::on_compile_stack;
use super::{compile_fol_with, CompileOptions, Dialect, PreludeMode};
use crate::parse;

/// `(errors, [(结局, 原因)])`；`typed = false` ⇒ 关掉 P2-10b（让带注解 λ 实参回落）。
fn run(src: &str, on: bool, typed: bool) -> (Vec<String>, Vec<(&'static str, &'static str)>) {
    let src = src.to_string();
    on_compile_stack(move || {
        TEST_OVERRIDE.with(|c| c.set(Some(on)));
        TEST_NO_TYPED_LAMBDA.with(|c| c.set(!typed));
        TEST_SETTLED.with(|v| v.borrow_mut().clear());
        let file = parse(&src).unwrap_or_else(|e| panic!("parse: {e:?}\n{src}"));
        let options = CompileOptions {
            prelude: PreludeMode::Full,
            dialect: Dialect::Strict,
        };
        let out = compile_fol_with(&file, &options);
        let errors = out.errors.iter().map(|e| format!("{e:?}")).collect();
        (errors, TEST_SETTLED.with(|v| v.borrow().clone()))
    })
}

fn n(v: &[(&str, &str)], tag: &str, why: Option<&str>) -> usize {
    v.iter()
        .filter(|(t, w)| *t == tag && why.is_none_or(|x| x == *w))
        .count()
}

const C1: &str = "axiom g : {A : Type} -> Nat -> Prop\ndef t : Prop := g 0\n";

/// `Eq.subst h hp`：高阶 motive 头挡在门外（`unsupported`）⇒ 回落、老路接下（接受）。
const RESCUED: &str =
    "theorem s2 (P : Nat -> Prop) (a b : Nat) (h : a = b) (hp : P a) : P b := Eq.subst h hp\n";

#[test]
fn c1_unsolved_fallback_is_counted_as_old_err() {
    let (errors, v) = run(C1, true, true);
    assert!(!errors.is_empty(), "c1 本来就拒绝");
    assert!(
        n(&v, "err", Some("unsolved")) >= 1,
        "c1 ⇒ old_err_unsolved ≥ 1：{v:?}"
    );
    assert_eq!(n(&v, "ok", None), 0, "c1 ⇒ old_ok = 0：{v:?}");
    // 闸门关 ⇒ 一条都不记、判卷相同。
    let (errors_off, v_off) = run(C1, false, true);
    assert_eq!(errors, errors_off);
    assert!(v_off.is_empty(), "{v_off:?}");
}

#[test]
fn rescued_fallback_is_counted_as_old_ok() {
    let (errors, v) = run(RESCUED, true, true);
    assert_eq!(errors, Vec::<String>::new());
    assert!(n(&v, "ok", None) >= 1, "老路接下 ⇒ old_ok ≥ 1：{v:?}");
}

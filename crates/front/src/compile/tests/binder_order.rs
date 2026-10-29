//! **隐式参数只许前导**（用户 2026-10-29）：[`first_violation`] 的形状 + `walk` 接线的行为。
//!
//! 判据：违例 ⇒ 恰好一条 `elab-implicit-binder-after-explicit`、那条声明**不**出 `decl.checked`、
//! 后面的声明照常判；前导隐式 / 高阶参数自己的前导隐式 ⇒ 零诊断。

use super::first_violation;
use crate::compile::compile_fol;
use crate::compile::error::ErrorKind;
use crate::compile::event::CheckEvent;

fn compile(src: &str) -> (Vec<ErrorKind>, Vec<String>, Vec<String>) {
    let file = crate::parse(src).unwrap_or_else(|e| panic!("parse: {e:?}\n{src}"));
    let out = compile_fol(&file);
    let kinds = out.errors.iter().map(|e| e.kind).collect();
    let messages = out.errors.iter().map(|e| e.message.clone()).collect();
    let checked = out
        .events
        .iter()
        .filter_map(|e| match e {
            CheckEvent::DeclarationChecked { name } => Some(name.clone()),
            _ => None,
        })
        .collect();
    (kinds, messages, checked)
}

/// 每条：(源码, 被拒的声明名)。后面都跟一条无关的 `ok_after` —— 它必须照常判过。
const REJECTED: &[(&str, &str)] = &[
    ("def f1 (a : Prop) {b : Prop} : Prop := a\n", "f1"),
    ("axiom ax2 : Prop -> {b : Prop} -> b\n", "ax2"),
    (
        "theorem t3 : forall (x : Prop), forall {y : Prop}, x -> x := \
         fun (x : Prop) => fun {y : Prop} => fun (h : x) => h\n",
        "t3",
    ),
    (
        "def g4 (a : Prop) : forall {b : Prop}, b -> b := fun {b : Prop} => fun (h : b) => h\n",
        "g4",
    ),
    // 高阶参数自己的类型是另一条链：链里 `→` 之后的隐式一样不许。
    ("axiom hk5 : (f : Prop -> {x : Prop} -> x) -> Prop\n", "hk5"),
    // 构造子的 binder。
    (
        "inductive V6 : Type\nctor mk (a : V6) {b : V6} : V6\nend\n",
        "V6",
    ),
    // 归纳参数（参数 + 类型是同一条链）。
    (
        "inductive W7 (A : Type) {B : Type} : Type\nctor mk (a : A) : W7 A\nend\n",
        "W7",
    ),
];

const OK_AFTER: &str = "def ok_after : Type := Prop\n";

#[test]
fn an_implicit_binder_after_an_explicit_layer_is_rejected_and_the_decl_stays_out() {
    for (src, name) in REJECTED {
        let full = format!("{src}{OK_AFTER}");
        let (kinds, messages, checked) = compile(&full);
        assert_eq!(
            kinds,
            vec![ErrorKind::ElabImplicitBinderAfterExplicit],
            "{name}: 恰好一条 binder 顺序诊断（{messages:?}）\n{full}"
        );
        assert!(
            messages[0].contains("隐式参数") && messages[0].contains("之后"),
            "{name}: 文案要点名隐式参数与它前面的显式层：{}",
            messages[0]
        );
        assert!(
            !checked.iter().any(|c| c == name),
            "{name}: 被拒的声明不许出 decl.checked（{checked:?}）"
        );
        assert!(
            checked.iter().any(|c| c == "ok_after"),
            "{name}: 后面的声明必须照常判过（{checked:?}）"
        );
    }
}

#[test]
fn the_message_names_the_implicit_binder_and_the_explicit_layer_before_it() {
    let (_, messages, _) = compile(REJECTED[0].0);
    assert!(
        messages[0].contains("`{b : Prop}`") && messages[0].contains("`a`"),
        "{}",
        messages[0]
    );
    let (_, messages, _) = compile(REJECTED[1].0);
    assert!(messages[0].contains("`→` 左边那一层"), "{}", messages[0]);
}

#[test]
fn leading_implicits_and_nested_leading_implicits_are_accepted() {
    for src in [
        "def ok1 {a : Prop} (h : a) : a := h\n",
        "axiom ok2 : {a : Prop} -> {b : Prop} -> a -> a\n",
        "theorem ok3 {a : Prop} (h : a) : forall (b : Prop), b -> a := \
         fun (b : Prop) => fun (hb : b) => h\n",
        // 高阶参数的类型是新链：它自己的**前导**隐式合法。
        "axiom ok4 : (f : {x : Prop} -> x -> x) -> Prop\n",
    ] {
        let file = crate::parse(src).unwrap_or_else(|e| panic!("parse: {e:?}\n{src}"));
        assert_eq!(
            file.commands.iter().find_map(first_violation),
            None,
            "合法签名被判成违例\n{src}"
        );
        let (kinds, messages, _) = compile(src);
        assert!(
            !kinds.contains(&ErrorKind::ElabImplicitBinderAfterExplicit),
            "合法签名报了 binder 顺序诊断：{messages:?}\n{src}"
        );
    }
}

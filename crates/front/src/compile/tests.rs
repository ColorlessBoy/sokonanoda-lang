//! compile 模块测试：自原 compile.rs 原样迁移，断言不变。

use super::*;
use crate::parse;
use crate::Span;

fn py_core() -> String {
    std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/py-fol-core.sokonanoda"
    ))
    .expect("read py-fol-core.sokonanoda")
}

fn py_core_checks(extra: &str) {
    let full = format!("{}\n{}", py_core(), extra);
    let file = parse(&full).unwrap_or_else(|e| panic!("parse failed: {e:?}"));
    let out = compile_fol(&file);
    assert_eq!(out.errors, vec![], "extra: {extra}");
}

fn py_core_rejects(extra: &str) {
    let full = format!("{}\n{}", py_core(), extra);
    let file = parse(&full).unwrap_or_else(|e| panic!("parse failed: {e:?}"));
    let out = compile_fol(&file);
    assert!(!out.errors.is_empty(), "expected rejection: {extra}");
}

#[test]
fn checks_a_valid_file_end_to_end() {
    let file = parse("def id : Prop -> Prop := fun (x : Prop) => x\n#check id\n").unwrap();
    let out = compile_fol(&file);
    assert_eq!(out.errors, vec![]);
    assert_eq!(out.events.len(), 2);
    assert!(matches!(
        &out.events[0],
        CheckEvent::DeclarationChecked { name } if name == "id"
    ));
    assert!(matches!(
        &out.events[1],
        CheckEvent::TypeChecked { text, .. } if text == "Prop -> Prop"
    ));
}

#[test]
fn accepts_open_exercise() {
    let file = parse("example : Prop -> Prop := sorry\n").unwrap();
    let out = compile_fol(&file);
    assert_eq!(out.errors, vec![]);
    assert_eq!(out.events, vec![CheckEvent::ExerciseOpen { name: None }]);
}

#[test]
fn checks_dependent_forall_with_lambda() {
    let file = parse(
        "theorem t : ∀ (P : Prop), P -> P :=\n\
         fun (P : Prop) (hp : P) => hp\n",
    )
    .unwrap();
    let out = compile_fol(&file);
    assert_eq!(out.errors, vec![]);
    assert_eq!(
        out.events,
        vec![CheckEvent::DeclarationChecked { name: "t".into() }]
    );
}

#[test]
fn checks_axioms_and_theorems_over_axioms() {
    let file = parse(
        "axiom p : Prop\n\
         theorem t : p -> p := fun (h : p) => h\n",
    )
    .unwrap();
    let out = compile_fol(&file);
    assert_eq!(out.errors, vec![]);
    assert_eq!(
        out.events,
        vec![
            CheckEvent::DeclarationChecked { name: "p".into() },
            CheckEvent::DeclarationChecked { name: "t".into() },
        ]
    );
}

#[test]
fn reports_kernel_rejection_with_span() {
    let file = parse("def bad : Prop -> Type := fun (x : Prop) => x\n").unwrap();
    let out = compile_fol(&file);
    assert!(!out.errors.is_empty(), "expected a kernel rejection");
    assert!(out.errors[0].span.start.line >= 1);
}

#[test]
fn checks_nat_literals_and_reduces_addition() {
    let file = parse(
        "def two : Nat := 1 + 1\n\
         #check two\n\
         #reduce 1 + 1\n",
    )
    .unwrap();
    let out = compile_fol(&file);
    assert_eq!(out.errors, vec![]);
    assert_eq!(out.events.len(), 3);
    assert!(matches!(
        &out.events[1],
        CheckEvent::TypeChecked { text, .. } if text == "Nat"
    ));
    assert!(matches!(
        &out.events[2],
        CheckEvent::Reduced { text, .. } if text == "2"
    ));
}

#[test]
fn checks_from_scratch_fol_proofs() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/fol-basics.sokonanoda"
    );
    let src = std::fs::read_to_string(path).expect("read fol-basics.sokonanoda");
    let file = parse(&src).expect("parse fol-basics.sokonanoda");
    let out = compile_fol(&file);
    assert_eq!(
        out.errors,
        vec![],
        "from-scratch FOL proofs should all check, got {:?}",
        out.errors
    );
    assert!(!out.events.is_empty());
}

#[test]
fn checks_universe_levels_and_function_type_types() {
    let file = parse(
        "#check Sort 2\n\
         #check (fun (α : Sort 2) => α)\n",
    )
    .unwrap();
    let out = compile_fol(&file);
    assert_eq!(out.errors, vec![]);
    assert_eq!(out.events.len(), 2);
    assert!(matches!(
        &out.events[0],
        CheckEvent::TypeChecked { text, .. } if text == "Type 2"
    ));
    assert!(matches!(
        &out.events[1],
        CheckEvent::TypeChecked { text, .. } if text == "Type 1 -> Type 1"
    ));
}

#[test]
fn document_report_carries_check_results() {
    // 编辑器常驻展示 #check 结果（inlay hint）的数据源：LSP 不需要再
    // 跑一遍内核，直接消费报告里的文本与表达式 span。
    let file = parse("#check Nat\n#check (Nat -> Nat)\n#check Prop\n").unwrap();
    let report = check_document(&file);
    let texts: Vec<&str> = report.checks.iter().map(|c| c.text.as_str()).collect();
    assert_eq!(texts, vec!["Type 0", "Type 0", "Type 0"]);
    assert!(
        report
            .checks
            .iter()
            .all(|c| c.span.start.offset < c.span.end.offset),
        "check spans point at the checked expression: {:?}",
        report.checks
    );
    assert!(report.errors.is_empty(), "errors: {:?}", report.errors);
}

#[test]
fn prints_definitions_without_panicking_on_open_bodies() {
    let file = parse(
        "def id : Prop -> Prop := fun (x : Prop) => x\n\
         #print id\n",
    )
    .unwrap();
    let out = compile_fol(&file);
    assert_eq!(out.errors, vec![]);
    let printed = out.events.iter().find_map(|event| match event {
        CheckEvent::Printed { name, text } => Some((name.as_str(), text.as_str())),
        _ => None,
    });
    assert_eq!(
        printed,
        Some(("id", "def id : Prop -> Prop := fun (x : Prop) => x"))
    );
}

#[test]
fn checks_universe_polymorphic_id_declaration() {
    let file = parse(
        "def id {u} : forall (α : Sort u), α -> α :=\n\
         fun (α : Sort u) => fun (a : α) => a\n",
    )
    .expect("parse universe-polymorphic declaration");
    let out = compile_fol(&file);
    assert_eq!(out.errors, vec![]);
}

#[test]
fn checks_explicit_universe_application() {
    let file = parse(
        "def id {u} : forall (α : Sort u), α -> α :=\n\
         fun (α : Sort u) => fun (a : α) => a\n\
         def id2 {u} : forall (α : Sort u), α -> α := id.{u}\n",
    )
    .expect("parse explicit universe application");
    let out = compile_fol(&file);
    assert_eq!(out.errors, vec![]);
}

#[test]
fn checks_plain_use_defaults_universe_to_zero() {
    let file = parse(
        "def id {u} : forall (α : Sort u), α -> α :=\n\
         fun (α : Sort u) => fun (a : α) => a\n\
         #check id\n",
    )
    .expect("parse plain use");
    let out = compile_fol(&file);
    assert_eq!(out.errors, vec![]);
    assert!(out
        .events
        .iter()
        .any(|event| matches!(event, CheckEvent::TypeChecked { .. })));
}

#[test]
fn checks_explicit_literal_universe_application() {
    let file = parse(
        "def id {u} : forall (α : Sort u), α -> α :=\n\
         fun (α : Sort u) => fun (a : α) => a\n\
         def id0 : (α : Prop) -> α -> α :=\n\
         fun (α : Prop) => id.{0} α\n",
    )
    .expect("parse literal universe application");
    let out = compile_fol(&file);
    assert_eq!(out.errors, vec![]);
}

#[test]
fn checks_axiom_with_two_universe_params() {
    let file = parse(
        "axiom cast {u, v} :\n\
         forall (α : Sort u), forall (β : Sort v), α -> β\n",
    )
    .expect("parse two universe params");
    let out = compile_fol(&file);
    assert_eq!(out.errors, vec![]);
}

#[test]
fn rejects_undeclared_universe_variable() {
    let file = parse(
        "def bad : forall (α : Sort u), α -> α :=\n\
         fun (α : Sort u) => fun (a : α) => a\n",
    )
    .expect("parse undeclared universe");
    let out = compile_fol(&file);
    assert!(!out.errors.is_empty());
}

#[test]
fn rejects_wrong_number_of_universe_arguments() {
    let file = parse(
        "def id {u} : forall (α : Sort u), α -> α :=\n\
         fun (α : Sort u) => fun (a : α) => a\n\
         def bad {u} : forall (α : Sort u), α -> α := id.{u, 0}\n",
    )
    .expect("parse wrong universe count");
    let out = compile_fol(&file);
    assert!(!out.errors.is_empty());
}

#[test]
fn checks_at_marker_and_implicit_binders_in_py_fol_style() {
    let file = parse(
        "def id {u} : forall {α : Sort u}, forall (a : α), α :=\n\
         fun {α : Sort u} => fun (a : α) => a\n\
         def id0 : forall (α : Prop), forall (a : α), α :=\n\
         fun (α : Prop) => @id.{0} α\n",
    )
    .expect("parse @id.{0} and implicit binders");
    let out = compile_fol(&file);
    assert_eq!(out.errors, vec![]);
}

#[test]
fn parses_implicit_binder_styles() {
    let file = parse("#check fun {x : Prop} => fun (y : Prop) => x\n").unwrap();
    assert_eq!(file.commands.len(), 1);
}

#[test]
fn checks_ported_py_fol_core() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/py-fol-core.sokonanoda"
    );
    let src = std::fs::read_to_string(path).expect("read py-fol-core.sokonanoda");
    let file = parse(&src).expect("parse py-fol-core.sokonanoda");
    let out = compile_fol(&file);
    assert_eq!(out.errors, vec![]);
}

#[test]
fn py_mt_and_not_and_of_not_left() {
    py_core_checks(
        "theorem mt_ : {a : Prop} -> {b : Prop} -> (f : a -> b) -> (hb : Not b) -> Not a :=\n\
         fun {a : Prop} => fun {b : Prop} => fun (f : a -> b) => fun (hb : Not b) => fun (ha : a) => hb (f ha)\n\
         theorem not_and_left_ : {a : Prop} -> (b : Prop) -> (ha : Not a) -> Not (And a b) :=\n\
         fun {a : Prop} => fun (b : Prop) => fun (ha : Not a) => fun (x : And a b) => ha (@And.left a b x)\n",
    );
}

#[test]
fn py_or_elim_and_not_or_intro() {
    py_core_checks(
        "theorem or_elim_ : {a : Prop} -> {b : Prop} -> {c : Prop} -> (t : Or a b) -> (left : a -> c) -> (right : b -> c) -> c :=\n\
         fun {a : Prop} => fun {b : Prop} => fun {c : Prop} => fun (t : Or a b) => fun (left : a -> c) => fun (right : b -> c) => @Or.rec a b (fun (x : Or a b) => c) left right t\n\
         theorem not_or_intro_ : {a : Prop} -> {b : Prop} -> (ha : Not a) -> (hb : Not b) -> Not (Or a b) :=\n\
         fun {a : Prop} => fun {b : Prop} => fun (ha : Not a) => fun (hb : Not b) => fun (x : Or a b) => @Or.rec a b (fun (y : Or a b) => False) (fun (l : a) => ha l) (fun (r : b) => hb r) x\n",
    );
}

#[test]
fn py_and_imp_is_iff() {
    py_core_checks(
        "theorem and_imp_iff_ : {a : Prop} -> {b : Prop} -> {c : Prop} -> Iff (And a b -> c) (a -> b -> c) :=\n\
         fun {a : Prop} => fun {b : Prop} => fun {c : Prop} =>\n\
           @Iff.intro (And a b -> c) (a -> b -> c)\n\
             (fun (h : And a b -> c) => fun (ha : a) => fun (hb : b) => h (@And.intro a b ha hb))\n\
             (fun (h : a -> b -> c) => fun (x : And a b) => @And.rec a b (fun (y : And a b) => c) (fun (ha : a) => fun (hb : b) => h ha hb) x)\n",
    );
}

#[test]
fn py_iff_refl_and_imp_swap() {
    py_core_checks(
        "theorem iff_refl_ : (a : Prop) -> Iff a a :=\n\
         fun (a : Prop) => @Iff.intro a a (fun (h : a) => h) (fun (h : a) => h)\n\
         theorem imp_swap_ : {a : Prop} -> {b : Prop} -> {c : Prop} -> Iff (a -> b -> c) (b -> a -> c) :=\n\
         fun {a : Prop} => fun {b : Prop} => fun {c : Prop} => @Iff.intro (a -> b -> c) (b -> a -> c) (@flip.{0, 0, 0} a b c) (@flip.{0, 0, 0} b a c)\n",
    );
}

#[test]
fn py_accepts_shadowed_binders() {
    py_core_checks(
        "theorem shadow_ : (a : Prop) -> (a : Prop) -> a -> a :=\n\
         fun (a : Prop) => fun (a : Prop) => fun (h : a) => h\n",
    );
}

#[test]
fn py_rejects_constructor_type_mismatch() {
    py_core_rejects("example : Or True True := True.intro\n");
}

#[test]
fn py_rejects_function_domain_mismatch() {
    py_core_rejects("example : False -> True := fun (h : True) => True.intro\n");
}

#[test]
fn py_rejects_or_constructor_on_wrong_side() {
    py_core_rejects("example : Or True False := @Or.inl False True True.intro\n");
}

#[test]
fn py_rejects_self_application() {
    py_core_rejects("example : (x : Prop) -> Prop := fun (x : Prop) => x x\n");
}

#[test]
fn py_accepts_eta_application_chain() {
    py_core_checks(
        "example : (a : Prop) -> (a -> a) -> a -> a :=\n\
         fun (a : Prop) => fun (f : a -> a) => fun (x : a) => f x\n",
    );
}

#[test]
fn py_accepts_or_rec_with_unrelated_motive() {
    py_core_checks(
        "theorem deep_or_rec_ : {a : Prop} -> {b : Prop} -> {c : Prop} -> (h : Or a b) -> (hc : c) -> c :=\n\
         fun {a : Prop} => fun {b : Prop} => fun {c : Prop} => fun (h : Or a b) => fun (hc : c) =>\n\
           @Or.rec a b (fun (x : Or a b) => c) (fun (ha : a) => hc) (fun (hb : b) => hc) h\n",
    );
}

#[test]
fn py_eq_symm_trans_are_in_ported_core() {
    py_core_checks(
        "theorem eq_symm_test_ {u} : {α : Sort u} -> (a : α) -> (b : α) -> (h : @Eq.{u} α a b) -> @Eq.{u} α b a :=\n\
         fun {α : Sort u} => fun (a : α) => fun (b : α) => fun (h : @Eq.{u} α a b) => @Eq_symm.{u} α a b h\n\
         theorem eq_trans_test_ {u} : {α : Sort u} -> (a : α) -> (b : α) -> (c : α) -> (h1 : @Eq.{u} α a b) -> (h2 : @Eq.{u} α b c) -> @Eq.{u} α a c :=\n\
         fun {α : Sort u} => fun (a : α) => fun (b : α) => fun (c : α) => fun (h1 : @Eq.{u} α a b) => fun (h2 : @Eq.{u} α b c) => @Eq_trans.{u} α a b c h1 h2\n",
    );
}

#[test]
fn py_propext_and_self_eq_checks() {
    py_core_checks(
        "theorem and_self_eq_ : (p : Prop) -> @Eq.{1} Prop (And p p) p :=\n\
         fun (p : Prop) => @propext (And p p) p (@Iff.intro (And p p) p (@And.left p p) (fun (h : p) => @And.intro p p h h))\n",
    );
}

#[test]
fn py_congr_arg_prop_level_checks() {
    py_core_checks(
        "theorem congrArg_prop_test_ {u} : {α : Sort u} -> (β : Prop) -> (f : α -> β) -> (a1 : α) -> (a2 : α) -> (h : @Eq.{u} α a1 a2) -> @Eq.{0} β (f a1) (f a2) :=\n\
         fun {α : Sort u} => fun (β : Prop) => fun (f : α -> β) => fun (a1 : α) => fun (a2 : α) => fun (h : @Eq.{u} α a1 a2) => @congrArg.{u} α β f a1 a2 h\n",
    );
}

#[test]
fn py_rejects_congr_arg_outside_prop_universe() {
    py_core_rejects(
        "theorem congrArg_bad_ {u, v} : {α : Sort u} -> {β : Sort v} -> (f : α -> β) -> (a1 : α) -> (a2 : α) -> (h : @Eq.{u} α a1 a2) -> @Eq.{v} β (f a1) (f a2) :=\n\
         fun {α : Sort u} => fun {β : Sort v} => fun (f : α -> β) => fun (a1 : α) => fun (a2 : α) => fun (h : @Eq.{u} α a1 a2) => @Eq.rec.{u, v} α a1 (fun (x : α) => @Eq.{v} β (f a1) (f x)) (@Eq.refl.{v} β (f a1)) a2 h\n",
    );
}

#[test]
fn py_not_imp_of_and_not_is_in_core() {
    py_core_checks(
        "theorem not_imp_use_ : {a : Prop} -> {b : Prop} -> (x : And a (Not b)) -> Not (a -> b) :=\n\
         fun {a : Prop} => fun {b : Prop} => fun (x : And a (Not b)) => @not_imp_of_and_not_ a b x\n",
    );
}

#[test]
fn py_or_iff_left_of_imp_is_in_core() {
    py_core_checks(
        "theorem or_left_use_ : {b : Prop} -> {a : Prop} -> (hb : b -> a) -> Iff (Or a b) a :=\n\
         fun {b : Prop} => fun {a : Prop} => fun (hb : b -> a) => @or_iff_left_of_imp_ b a hb\n",
    );
}

/// 非递归归纳块（Bool/Unit/Empty 的前置）：内核按构造子自算 is_recursive，
/// front 必须镜像同规则（此前恒传 true → 内核断言崩溃）。
#[test]
fn non_recursive_inductive_block_compiles_and_reduces() {
    let src = r#"
inductive Unit : Type
ctor unit : Unit
rec Unit.rec {u} : (motive : (x : Unit) -> Sort u) -> (mz : motive unit) -> (x : Unit) -> motive x
iota unit := fun (motive : (x : Unit) -> Sort u) => fun (mz : motive unit) => mz
end
def u : Unit := unit
#reduce Unit.rec.{1} (fun (x : Unit) => Nat) 1 u
"#;
    let file = parse(src).expect("parse non-recursive inductive block");
    let out = compile_fol(&file);
    assert_eq!(
        out.errors,
        vec![],
        "non-recursive block must pass the kernel: {:?}",
        out.errors
    );
    assert!(
        out.events
            .iter()
            .any(|e| matches!(e, CheckEvent::Reduced { text, .. } if text == "1")),
        "iota on the non-recursive ctor must compute: {:?}",
        out.events
    );
}

/// 无 rec 的归纳块自动派生 recursor：非递归块（Unit）零错误，且
/// `Unit.rec` 的 iota 规则能在内核上归约（与第十三轮显式 rec 同形状）。
#[test]
fn auto_derived_recursor_non_recursive_compiles() {
    let src = r#"
inductive Unit : Type
ctor unit : Unit
end
def u : Unit := unit
#reduce Unit.rec.{1} (fun (x : Unit) => Nat) 1 u
"#;
    let file = parse(src).expect("parse rec-less block");
    let out = compile_fol(&file);
    assert_eq!(
        out.errors,
        vec![],
        "auto-derived recursor must compile: {:?}",
        out.errors
    );
    assert!(
        out.events
            .iter()
            .any(|e| matches!(e, CheckEvent::Reduced { text, .. } if text == "1")),
        "iota on the non-recursive ctor must compute: {:?}",
        out.events
    );
}

/// 无 rec 的 Bool 块：派生的 recursor 给出 if-then-else 语义。
#[test]
fn auto_derived_recursor_bool_computes() {
    let src = r#"
inductive Bool : Type
ctor tt : Bool
ctor ff : Bool
end
def not : Bool -> Bool := fun (b : Bool) => Bool.rec.{1} (fun (x : Bool) => Bool) ff tt b
#reduce not tt
#reduce not ff
"#;
    let file = parse(src).expect("parse Bool block");
    let out = compile_fol(&file);
    assert_eq!(out.errors, vec![], "errors: {:?}", out.errors);
    assert!(
        out.events
            .iter()
            .any(|e| matches!(e, CheckEvent::Reduced { text, .. } if text == "ff")),
        "`not tt` must reduce to ff: {:?}",
        out.events
    );
    assert!(
        out.events
            .iter()
            .any(|e| matches!(e, CheckEvent::Reduced { text, .. } if text == "tt")),
        "`not ff` must reduce to tt: {:?}",
        out.events
    );
}

/// 递归块无 rec（Nat 加法）：派生规则含递归字段后的自调用，加法可归约。
#[test]
fn auto_derived_recursor_recursive_nat_adds() {
    let src = r#"
inductive Nat : Type
ctor zero : Nat
ctor succ (n : Nat) : Nat
end
def one : Nat := succ zero
def two : Nat := succ one
def add : Nat -> Nat -> Nat :=
  fun (m : Nat) => fun (n : Nat) =>
    Nat.rec.{1} (fun (x : Nat) => Nat) n
      (fun (k : Nat) => fun (ih : Nat) => succ ih) m
#reduce add two two
"#;
    let file = parse(src).expect("parse rec-less Nat block");
    let out = compile_fol(&file);
    assert_eq!(out.errors, vec![], "errors: {:?}", out.errors);
    assert!(
        out.events.iter().any(|e| matches!(
            e,
            CheckEvent::Reduced { text, .. } if text == "succ (succ (succ (succ zero)))"
        )),
        "expected the 4-deep succ chain for `add two two`, got {:?}",
        out.events
    );
}

#[test]
fn explicit_inductive_block_compiles() {
    let src = r#"
inductive MyNat : Type
ctor z : MyNat
ctor s (n : MyNat) : MyNat
rec MyNat.rec {u} : (motive : (n : MyNat) -> Sort u) -> (mz : motive z) -> (ms : (n : MyNat) -> motive n -> motive (s n)) -> (n : MyNat) -> motive n
iota z := fun (motive : (n : MyNat) -> Sort u) => fun (mz : motive z) => fun (ms : (n : MyNat) -> motive n -> motive (s n)) => mz
iota s := fun (motive : (n : MyNat) -> Sort u) => fun (mz : motive z) => fun (ms : (n : MyNat) -> motive n -> motive (s n)) => fun (n : MyNat) => ms n (MyNat.rec.{u} motive mz ms n)
end
def oneMyNat : MyNat := s z
def myAdd : MyNat -> MyNat -> MyNat :=
  fun (m : MyNat) => fun (n : MyNat) =>
    MyNat.rec.{1} (fun (x : MyNat) => MyNat) n (fun (k : MyNat) => fun (ih : MyNat) => s ih) m
#reduce MyNat.rec.{1} (fun (n : MyNat) => MyNat) z (fun (n : MyNat) => fun (ih : MyNat) => s n) z
#reduce MyNat.rec.{1} (fun (n : MyNat) => MyNat) z (fun (n : MyNat) => fun (ih : MyNat) => s n) (s z)
#reduce myAdd (s (s z)) (s z)
"#;
    let file = parse(src).expect("parse explicit inductive block");
    let out = compile_fol(&file);
    assert_eq!(out.errors, vec![]);
    assert!(
        out.events.iter().any(|e| matches!(
            e,
            CheckEvent::Reduced { text, .. } if text == "z"
        )),
        "events: {:?}",
        out.events
    );
    assert!(
        out.events.iter().any(|e| matches!(
            e,
            CheckEvent::Reduced { text, .. } if text == "s z"
        )),
        "events: {:?}",
        out.events
    );
    assert!(
        out.events.iter().any(|e| matches!(
            e,
            CheckEvent::Reduced { text, .. } if text == "s (s (s z))"
        )),
        "events: {:?}",
        out.events
    );
}

#[test]
fn explicit_nat_block_overrides_builtin_prelude() {
    let src = r#"
inductive Nat : Type
ctor zero : Nat
ctor succ (n : Nat) : Nat
rec Nat.rec {u} : (motive : (n : Nat) -> Sort u) -> (mz : motive zero) -> (ms : (n : Nat) -> motive n -> motive (succ n)) -> (n : Nat) -> motive n
iota zero := fun (motive : (n : Nat) -> Sort u) => fun (mz : motive zero) => fun (ms : (n : Nat) -> motive n -> motive (succ n)) => mz
iota succ := fun (motive : (n : Nat) -> Sort u) => fun (mz : motive zero) => fun (ms : (n : Nat) -> motive n -> motive (succ n)) => fun (n : Nat) => ms n (Nat.rec.{u} motive mz ms n)
end
def oneNat : Nat := succ zero
#reduce Nat.rec.{1} (fun (n : Nat) => Nat) zero (fun (n : Nat) => fun (ih : Nat) => succ n) (succ zero)
"#;
    let file = parse(src).expect("parse explicit Nat block");
    let out = compile_fol(&file);
    assert_eq!(out.errors, vec![]);
    assert!(
        out.events.iter().any(|e| matches!(
            e,
            CheckEvent::Reduced { text, .. } if text == "succ zero"
        )),
        "events: {:?}",
        out.events
    );
}

#[test]
fn ported_nat_fol_add_two_two_reduces() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/py-nat.sokonanoda"
    );
    let src = std::fs::read_to_string(path).expect("read py-nat.sokonanoda");
    let file = parse(&src).expect("parse py-nat.sokonanoda");
    let out = compile_fol(&file);
    assert_eq!(out.errors, vec![]);
    assert!(
        out.events.iter().any(|e| matches!(
            e,
            CheckEvent::Reduced { text, .. } if text == "succ (succ (succ (succ zero)))"
        )),
        "events: {:?}",
        out.events
    );
}

#[test]
fn named_arrow_is_forall_with_explicit_binder() {
    let file = parse(
        "def id {u} : (α : Sort u) -> α -> α :=\n\
         fun {α : Sort u} => fun (a : α) => a\n",
    )
    .expect("parse named arrow");
    let out = compile_fol(&file);
    assert_eq!(out.errors, vec![]);
}

#[test]
fn named_arrow_supports_implicit_binders() {
    let file = parse(
        "def id0 : {a : Prop} -> a -> a :=\n\
         fun {a : Prop} => fun (h : a) => h\n",
    )
    .expect("parse implicit named arrow");
    let out = compile_fol(&file);
    assert_eq!(out.errors, vec![]);
}

#[test]
fn document_report_tracks_open_checked_failed_decls() {
    let file = parse(
        "def ok : Prop -> Prop := fun (x : Prop) => x\n\
         example : Nat := sorry\n\
         def bad : Prop -> Type := fun (x : Prop) => x\n",
    )
    .expect("parse");
    let report = check_document(&file);
    assert_eq!(report.decls.len(), 3, "decls: {:?}", report.decls);
    assert_eq!(report.decls[0].status, DeclStatus::Checked);
    assert_eq!(report.decls[0].name.as_deref(), Some("ok"));
    assert_eq!(report.decls[1].status, DeclStatus::Open);
    assert_eq!(report.decls[1].goal.as_deref(), Some("Nat"));
    assert_eq!(report.decls[2].status, DeclStatus::Failed);
    assert_eq!(
        report.decls[2].error.as_ref().map(|e| e.code()),
        Some("kernel-rejected")
    );
    // an open exercise must not make later valid declarations fail
    let file2 = parse(
        "def f : Prop -> Prop := fun (x : Prop) => x\n\
         theorem ex : (a : Prop) -> a -> a := sorry\n\
         example : Prop -> Prop := fun (x : Prop) => x\n",
    )
    .expect("parse2");
    let out = compile_fol(&file2);
    assert_eq!(out.errors, vec![], "errors: {:?}", out.errors);
    assert!(out
        .events
        .iter()
        .any(|e| matches!(e, CheckEvent::ExerciseOpen { name: Some(n) } if n == "ex")));
    assert!(out
        .events
        .iter()
        .any(|e| matches!(e, CheckEvent::ExampleChecked)));
}

#[test]
fn document_report_produces_hover_types_for_subexpressions() {
    let file = parse(
        "def two : Nat := 1 + 1\n\
         def id : Prop -> Prop := fun (x : Prop) => x\n",
    )
    .expect("parse");
    let report = check_document(&file);
    assert!(
        report.hovers.iter().any(|h| h.text == "Nat"),
        "expected a Nat hover, got {:?}",
        report
            .hovers
            .iter()
            .map(|h| h.text.clone())
            .collect::<Vec<_>>()
    );
    assert!(
        report.hovers.iter().any(|h| h.text == "Prop -> Prop"),
        "expected a function-type hover, got {:?}",
        report
            .hovers
            .iter()
            .map(|h| h.text.clone())
            .collect::<Vec<_>>()
    );
    assert!(
        report.hovers.iter().any(|h| h.text == "Prop"),
        "expected a Prop hover for the bound variable x, got {:?}",
        report
            .hovers
            .iter()
            .map(|h| h.text.clone())
            .collect::<Vec<_>>()
    );
}

#[test]
fn every_error_kind_has_stable_code_and_hint() {
    let _all_variants_listed = |kind: ErrorKind| {
        matches!(
            kind,
            ErrorKind::ElabUnknownIdentifier
                | ErrorKind::ElabUnknownConstant
                | ErrorKind::ElabUnknownUniverseLevel
                | ErrorKind::ElabUniverseArity
                | ErrorKind::ElabUntypedBinder
                | ErrorKind::ElabHoleMisplaced
                | ErrorKind::ElabDuplicateDeclaration
                | ErrorKind::ElabTooManyBinders
                | ErrorKind::ElabNatLiteralDisabled
                | ErrorKind::ElabInvalidNatLiteral
                | ErrorKind::ElabTooManyCtorFields
                | ErrorKind::ElabUnknownCtorForIota
                | ErrorKind::KernelExpectedSort
                | ErrorKind::KernelExpectedPi
                | ErrorKind::KernelTheoremNotProp
                | ErrorKind::KernelNonPositive
                | ErrorKind::KernelCtorResultMismatch
                | ErrorKind::KernelCtorArgInvalidApp
                | ErrorKind::KernelCtorArgNotType
                | ErrorKind::KernelCtorArgTooLarge
                | ErrorKind::KernelRecRuleMismatch
                | ErrorKind::KernelRejected
                | ErrorKind::KernelInternal
        )
    };
    let all = [
        ErrorKind::ElabUnknownIdentifier,
        ErrorKind::ElabUnknownConstant,
        ErrorKind::ElabUnknownUniverseLevel,
        ErrorKind::ElabUniverseArity,
        ErrorKind::ElabUntypedBinder,
        ErrorKind::ElabHoleMisplaced,
        ErrorKind::ElabDuplicateDeclaration,
        ErrorKind::ElabTooManyBinders,
        ErrorKind::ElabNatLiteralDisabled,
        ErrorKind::ElabInvalidNatLiteral,
        ErrorKind::ElabTooManyCtorFields,
        ErrorKind::ElabUnknownCtorForIota,
        ErrorKind::KernelExpectedSort,
        ErrorKind::KernelExpectedPi,
        ErrorKind::KernelTheoremNotProp,
        ErrorKind::KernelNonPositive,
        ErrorKind::KernelCtorResultMismatch,
        ErrorKind::KernelCtorArgInvalidApp,
        ErrorKind::KernelCtorArgNotType,
        ErrorKind::KernelCtorArgTooLarge,
        ErrorKind::KernelRecRuleMismatch,
        ErrorKind::KernelRejected,
        ErrorKind::KernelInternal,
    ];
    for kind in all {
        let code = kind.code();
        assert!(!code.is_empty(), "{kind:?} has an empty code");
        match kind.stage() {
            CompileStage::Elab => {
                assert!(
                    code.starts_with("elab-") && !code.starts_with("kernel-"),
                    "{kind:?} is an elab kind but its code is `{code}`"
                );
            }
            CompileStage::Kernel => {
                assert!(
                    code.starts_with("kernel-") && !code.starts_with("elab-"),
                    "{kind:?} is a kernel kind but its code is `{code}`"
                );
            }
        }
        let hint = kind.hint();
        assert!(!hint.is_empty(), "{kind:?} has an empty hint");
        assert!(
            hint.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c)),
            "{kind:?} hint is not Chinese text: {hint}"
        );
    }
}

#[test]
fn document_report_states_in_source_order() {
    let file = parse(
        "def ok : Prop -> Prop := fun (x : Prop) => x\n\
         example : Nat := sorry\n\
         def bad : Prop -> Type := fun (x : Prop) => x\n\
         theorem later : (a : Prop) -> a -> a := fun (a : Prop) => fun (h : a) => h\n",
    )
    .expect("parse");
    let report = check_document(&file);
    assert_eq!(report.decls.len(), 4, "decls: {:?}", report.decls);
    let offsets: Vec<usize> = report.decls.iter().map(|d| d.span.start.offset).collect();
    let mut sorted = offsets.clone();
    sorted.sort();
    assert_eq!(
        offsets, sorted,
        "decls not sorted by offset: {:?}",
        report.decls
    );
    let statuses: Vec<DeclStatus> = report.decls.iter().map(|d| d.status).collect();
    assert_eq!(
        statuses,
        vec![
            DeclStatus::Checked,
            DeclStatus::Open,
            DeclStatus::Failed,
            DeclStatus::Checked,
        ]
    );
    assert_eq!(report.decls[0].name.as_deref(), Some("ok"));
    assert_eq!(report.decls[1].status, DeclStatus::Open);
    assert_eq!(report.decls[2].name.as_deref(), Some("bad"));
    let failed_error = report.decls[2]
        .error
        .as_ref()
        .expect("failed decl carries error");
    assert_eq!(failed_error.code(), "kernel-rejected");
    assert_eq!(report.decls[3].name.as_deref(), Some("later"));
    assert_eq!(report.errors.len(), 1, "errors: {:?}", report.errors);
    assert_eq!(report.errors[0].code(), "kernel-rejected");
}

#[test]
fn open_exercise_does_not_pollute_env() {
    let file = parse(
        "example : Prop -> Prop := sorry\n\
         theorem ok : (a : Prop) -> a -> a := fun (a : Prop) => fun (h : a) => h\n",
    )
    .expect("parse");
    let out = compile_fol(&file);
    assert_eq!(
        out.errors,
        vec![],
        "open exercise must not poison the environment: {:?}",
        out.errors
    );
    assert!(out
        .events
        .iter()
        .any(|e| matches!(e, CheckEvent::ExerciseOpen { name: None })));
    assert!(out
        .events
        .iter()
        .any(|e| matches!(e, CheckEvent::DeclarationChecked { name } if name == "ok")));
}

#[test]
fn hover_map_covers_subexpressions() {
    let src = "def add1 : Nat -> Nat := fun (n : Nat) => n + 1\n";
    let file = parse(src).unwrap();
    let report = check_document(&file);
    assert!(!report.hovers.is_empty(), "hovers: {:?}", report.hovers);
    for hover in &report.hovers {
        assert!(
            hover.binder || !hover.text.is_empty(),
            "empty hover text: {:?}",
            hover
        );
        assert!(
            hover.span.start.offset <= hover.span.end.offset && hover.span.end.offset <= src.len(),
            "hover span outside the file: {:?}",
            hover
        );
    }
    assert!(
        report.hovers.iter().any(|h| h.text == "Nat -> Nat"),
        "expected the outermost `Nat -> Nat` hover, got {:?}",
        report
            .hovers
            .iter()
            .map(|h| h.text.clone())
            .collect::<Vec<_>>()
    );
    assert!(
        report.hovers.iter().any(|h| h.text == "Nat"),
        "expected a `Nat` hover over the `n + 1` sub-expression, got {:?}",
        report
            .hovers
            .iter()
            .map(|h| h.text.clone())
            .collect::<Vec<_>>()
    );
}

#[test]
fn hover_rows_type_universe_applied_eq_prelude_constants() {
    // 用户原始场景（playground.sokonanoda:233）：`Eq.subst.{1}` 的 hover
    // 曾只剩源码切片——内核 pp 的 `is_implicit_fun` 对开项推断 panic，
    // front 的 catch_unwind 把类型文本吞成空。修复后必须带完整签名。
    let src = concat!(
        "theorem eq_symm_nat : (a : Nat) -> (b : Nat) -> Eq.{1} Nat a b -> Eq.{1} Nat b a :=\n",
        "  fun (a : Nat) (b : Nat) (h : Eq.{1} Nat a b) =>\n",
        "    Eq.subst.{1} Nat (fun (x : Nat) => Eq.{1} Nat x a) a b h (Eq.refl.{1} Nat a)\n",
    );
    let report = check_document(&parse(src).expect("parse"));
    let hover = report
        .hovers
        .iter()
        .find(|h| &src[h.span.start.offset..h.span.end.offset] == "Eq.subst.{1}")
        .expect("hover row for `Eq.subst.{1}`");
    assert!(
        !hover.text.is_empty(),
        "`Eq.subst.{{1}}` hover must carry a type after the pp fix"
    );
    assert!(
        hover.text.contains("p a"),
        "hover type must mention the dependent codomain `p a`, got: {}",
        hover.text
    );
    let refl = report
        .hovers
        .iter()
        .find(|h| &src[h.span.start.offset..h.span.end.offset] == "Eq.refl.{1}")
        .expect("hover row for `Eq.refl.{1}`");
    assert!(
        !refl.text.is_empty(),
        "`Eq.refl.{{1}}` hover must carry a type after the pp fix"
    );
}

/// The definition span recorded for the use point starting at `offset`
/// (`None` when the name use did not resolve to a source definition).
fn resolved_def_at(report: &DocumentReport, offset: usize) -> Option<Span> {
    report
        .hovers
        .iter()
        .filter(|h| h.span.start.offset == offset)
        .find_map(|h| h.resolution.as_ref().map(|target| target.span()))
}

#[test]
fn definitions_map_resolves_binder_and_top_level_uses() {
    let src = "def id : Prop -> Prop := fun (x : Prop) => x\n#check id\n";
    let file = parse(src).unwrap();
    let report = check_document(&file);

    let body_x = src.rfind('x').unwrap();
    let binder = resolved_def_at(&report, body_x).expect("body `x` must resolve to its binder");
    let binder_at = src.find("(x : Prop)").unwrap();
    assert_eq!(binder.start.offset, binder_at);
    assert_eq!(binder.end.offset, binder_at + "(x : Prop)".len());

    let use_id = src.find("#check id").unwrap() + "#check ".len();
    let decl = resolved_def_at(&report, use_id).expect("`#check id` must resolve to the def");
    assert_eq!(decl.start.offset, 0);
    assert_eq!(
        decl.end.offset,
        "def id : Prop -> Prop := fun (x : Prop) => x".len()
    );
}

#[test]
fn definitions_map_skips_unknown_and_prelude_names() {
    let src = "axiom p : Prop\n#check p\n#check missing\n";
    let file = parse(src).unwrap();
    let report = check_document(&file);

    let use_p = src.find("#check p").unwrap() + "#check ".len();
    let decl = resolved_def_at(&report, use_p).expect("file-local axiom must resolve");
    assert_eq!(decl.start.offset, 0);
    assert_eq!(decl.end.offset, "axiom p : Prop".len());

    let prop = src.find("Prop").unwrap();
    assert_eq!(
        resolved_def_at(&report, prop),
        None,
        "prelude names have no source definition"
    );

    let missing = src.find("missing").unwrap();
    assert_eq!(
        resolved_def_at(&report, missing),
        None,
        "unknown idents stay unresolved"
    );
}

#[test]
fn definitions_map_shadows_inner_binder() {
    let src = "def f : Prop -> Prop -> Prop := fun (x : Prop) => fun (x : Prop) => x\n";
    let file = parse(src).unwrap();
    let report = check_document(&file);

    let body_x = src.rfind('x').unwrap();
    let inner = resolved_def_at(&report, body_x).expect("inner `x` must resolve");
    let inner_at = src.rfind("(x : Prop)").unwrap();
    let outer_at = src.find("(x : Prop)").unwrap();
    assert_eq!(
        inner.start.offset, inner_at,
        "inner x binds to the inner binder"
    );
    assert_eq!(inner.end.offset, inner_at + "(x : Prop)".len());
    assert_ne!(inner.start.offset, outer_at);
}

#[test]
fn hover_rows_carry_in_scope_binder_names() {
    let src = "def id : Prop -> Prop := fun (x : Prop) => x\n";
    let file = parse(src).unwrap();
    let report = check_document(&file);

    let body_x = src.rfind('x').unwrap();
    let row = report
        .hovers
        .iter()
        .find(|h| h.span.start.offset == body_x && h.span.end.offset == body_x + 1)
        .expect("hover row for the body `x`");
    assert_eq!(row.scope_names, vec!["x".to_string()]);
}

#[test]
fn render_expr_round_trips() {
    use crate::proof::parse_expr_text;

    let cases = [
        ("Prop", "Prop"),
        ("f x", "f x"),
        ("fun (x : Prop) => x", "fun (x : Prop) => x"),
        ("(x : Prop) -> x", "(x : Prop) -> x"),
        ("1 + 1", "1 + 1"),
        ("sorry", "sorry"),
        ("@Eq.{u, v}", "@Eq.{u, v}"),
    ];
    for (source, expected) in cases {
        let expr = parse_expr_text(source).unwrap_or_else(|e| panic!("parse {source}: {e:?}"));
        assert_eq!(render_expr(&expr), expected, "source: {source}");
    }
}

#[test]
fn kernel_rejection_message_is_not_a_panic_trace() {
    // A kernel rejection without a conv mismatch (theorem type is not Prop)
    // keeps the raw kernel message; def-eq mismatches get the teaching text
    // (see kernel_rejection_reports_expected_and_actual).
    let file = parse("theorem bad : Prop := fun (x : Prop) => x\n").unwrap();
    let out = compile_fol(&file);
    assert_eq!(out.errors.len(), 1, "errors: {:?}", out.errors);
    let message = &out.errors[0].message;
    assert!(
        !message.contains("panicked at"),
        "panic trace leaked into the learner-facing message: {message}"
    );
    assert!(
        message.contains("rejected") || message.contains("kernel error"),
        "message is neither a rejection nor a kernel error: {message}"
    );
    // 内核错误分类学后，theorem 非 Prop 有了专属细粒度码。
    assert_eq!(out.errors[0].code(), "kernel-theorem-not-prop");
}

#[test]
fn protocol_doc_lists_every_error_code() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/protocol.md");
    let doc = std::fs::read_to_string(path).expect("read docs/protocol.md");
    let all = [
        ErrorKind::ElabUnknownIdentifier,
        ErrorKind::ElabUnknownConstant,
        ErrorKind::ElabUnknownUniverseLevel,
        ErrorKind::ElabUniverseArity,
        ErrorKind::ElabUntypedBinder,
        ErrorKind::ElabHoleMisplaced,
        ErrorKind::ElabDuplicateDeclaration,
        ErrorKind::ElabTooManyBinders,
        ErrorKind::ElabNatLiteralDisabled,
        ErrorKind::ElabInvalidNatLiteral,
        ErrorKind::ElabTooManyCtorFields,
        ErrorKind::ElabUnknownCtorForIota,
        ErrorKind::KernelExpectedSort,
        ErrorKind::KernelExpectedPi,
        ErrorKind::KernelTheoremNotProp,
        ErrorKind::KernelNonPositive,
        ErrorKind::KernelCtorResultMismatch,
        ErrorKind::KernelCtorArgInvalidApp,
        ErrorKind::KernelCtorArgNotType,
        ErrorKind::KernelCtorArgTooLarge,
        ErrorKind::KernelRecRuleMismatch,
        ErrorKind::KernelRejected,
        ErrorKind::KernelInternal,
    ];
    let _all_variants_listed = |kind: ErrorKind| {
        matches!(
            kind,
            ErrorKind::ElabUnknownIdentifier
                | ErrorKind::ElabUnknownConstant
                | ErrorKind::ElabUnknownUniverseLevel
                | ErrorKind::ElabUniverseArity
                | ErrorKind::ElabUntypedBinder
                | ErrorKind::ElabHoleMisplaced
                | ErrorKind::ElabDuplicateDeclaration
                | ErrorKind::ElabTooManyBinders
                | ErrorKind::ElabNatLiteralDisabled
                | ErrorKind::ElabInvalidNatLiteral
                | ErrorKind::ElabTooManyCtorFields
                | ErrorKind::ElabUnknownCtorForIota
                | ErrorKind::KernelExpectedSort
                | ErrorKind::KernelExpectedPi
                | ErrorKind::KernelTheoremNotProp
                | ErrorKind::KernelNonPositive
                | ErrorKind::KernelCtorResultMismatch
                | ErrorKind::KernelCtorArgInvalidApp
                | ErrorKind::KernelCtorArgNotType
                | ErrorKind::KernelCtorArgTooLarge
                | ErrorKind::KernelRecRuleMismatch
                | ErrorKind::KernelRejected
                | ErrorKind::KernelInternal
        )
    };
    let undocumented: Vec<&str> = all
        .iter()
        .map(|kind| kind.code())
        .filter(|code| !doc.contains(code))
        .collect();
    assert!(
        undocumented.is_empty(),
        "error codes absent from docs/protocol.md (document them): {undocumented:?}"
    );
    for parse_code in ["unexpected-token", "unexpected-eof"] {
        assert!(
            doc.contains(parse_code),
            "parse code {parse_code} missing from docs/protocol.md"
        );
    }
}

#[test]
fn perf_smoke_native_and_iota_reduce() {
    let started = std::time::Instant::now();

    let big = "1234567890123456789012345678901234567890";
    let file = parse(&format!("#reduce {big} + 1\n")).unwrap();
    let out = compile_fol(&file);
    assert_eq!(
        out.errors,
        vec![],
        "bignum native reduce failed: {:?}",
        out.errors
    );
    let expected_big = {
        use num_bigint::BigUint;
        let n: BigUint = big.parse().expect("parse big literal");
        format!("{}", n + BigUint::from(1u32))
    };
    assert!(
        out.events.iter().any(|e| matches!(
            e,
            CheckEvent::Reduced { text, .. } if *text == expected_big
        )),
        "expected a Reduced event with `{expected_big}`, got {:?}",
        out.events
    );

    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/py-nat.sokonanoda"
    );
    let src = std::fs::read_to_string(path).expect("read py-nat.sokonanoda");
    let file = parse(&src).expect("parse py-nat.sokonanoda");
    let out = compile_fol(&file);
    assert_eq!(out.errors, vec![], "errors: {:?}", out.errors);
    assert!(
        out.events.iter().any(|e| matches!(
            e,
            CheckEvent::Reduced { text, .. } if text == "succ (succ (succ (succ zero)))"
        )),
        "expected the 4-deep succ chain for `add two two`, got {:?}",
        out.events
    );

    let elapsed = started.elapsed();
    assert!(
        elapsed.as_secs() < 30,
        "perf smoke exceeded the 30s debug-build canary: {elapsed:?}"
    );
}

// ---------------------------------------------------------------------------
// I6: prelude 可选化、Eq 三件套、binder 推断、partial hole
// ---------------------------------------------------------------------------

#[test]
fn eq_prelude_refl_checks_at_type_universe() {
    let file = parse("theorem refl_two : Eq.{1} Nat 2 2 := Eq.refl.{1} Nat 2\n").unwrap();
    let out = compile_fol(&file);
    assert_eq!(out.errors, vec![], "{:?}", out.errors);
}

#[test]
fn eq_prelude_symm_derivable_from_subst() {
    let file = parse(
        "theorem eq_symm_nat : (a : Nat) -> (b : Nat) -> Eq.{1} Nat a b -> Eq.{1} Nat b a :=\n\
         fun (a : Nat) => fun (b : Nat) => fun (h : Eq.{1} Nat a b) =>\n\
           Eq.subst.{1} Nat (fun (x : Nat) => Eq.{1} Nat x a) a b h (Eq.refl.{1} Nat a)\n",
    )
    .unwrap();
    let out = compile_fol(&file);
    assert_eq!(out.errors, vec![], "{:?}", out.errors);
}

#[test]
fn file_declaring_eq_takes_over_the_eq_prelude() {
    // All-or-nothing: a file that declares its own Eq installs none of the
    // prelude Eq block (no duplicate-declaration error, prelude refs absent).
    let file = parse("axiom Eq {u} : {α : Sort u} -> α -> α -> Prop\n").unwrap();
    let out = compile_fol(&file);
    assert_eq!(out.errors, vec![], "{:?}", out.errors);
}

#[test]
fn bare_mode_installs_nothing() {
    let options = CompileOptions {
        prelude: PreludeMode::Bare,
    };
    // Prop-level definitions still work without any prelude...
    let file = parse("def id : Prop -> Prop := fun (x : Prop) => x\n").unwrap();
    let out = compile_fol_with(&file, &options);
    assert_eq!(out.errors, vec![], "{:?}", out.errors);
    // ...but Nat is gone entirely.
    let file = parse("#reduce 1 + 1\n").unwrap();
    let out = compile_fol_with(&file, &options);
    assert!(out
        .errors
        .iter()
        .any(|e| e.kind == ErrorKind::ElabUnknownIdentifier));
}

#[test]
fn prelude_directive_reads_bare_and_full() {
    assert_eq!(
        prelude_mode_from_source("-- sokonanoda:prelude none\ndef x : Prop := sorry\n"),
        PreludeMode::Bare
    );
    assert_eq!(
        prelude_mode_from_source("-- 课程\n-- sokonanoda:prelude bare\n"),
        PreludeMode::Bare
    );
    assert_eq!(
        prelude_mode_from_source("def x : Prop := sorry\n"),
        PreludeMode::Full
    );
    assert_eq!(
        prelude_mode_from_source("-- sokonanoda:prelude full\n"),
        PreludeMode::Full
    );
}

#[test]
fn infers_untyped_binder_from_declared_arrow() {
    let file = parse("def add1 : Nat -> Nat := fun n => n + 1\n#check add1\n").unwrap();
    let out = compile_fol(&file);
    assert_eq!(out.errors, vec![], "{:?}", out.errors);
    assert!(matches!(
        &out.events[1],
        CheckEvent::TypeChecked { text, .. } if text == "Nat -> Nat"
    ));
}

#[test]
fn infers_dependent_binders_from_declared_forall() {
    let file = parse(
        "def idd {u} : forall (α : Sort u), α -> α := fun α => fun a => a\n\
         #reduce idd.{1} Nat 2\n",
    )
    .unwrap();
    let out = compile_fol(&file);
    assert_eq!(out.errors, vec![], "{:?}", out.errors);
    assert!(matches!(
        &out.events[1],
        CheckEvent::Reduced { text, .. } if text == "2"
    ));
}

#[test]
fn untyped_binder_without_expected_type_is_rejected() {
    let file = parse("#check fun x => x\n").unwrap();
    let out = compile_fol(&file);
    assert!(out
        .errors
        .iter()
        .any(|e| e.kind == ErrorKind::ElabUntypedBinder));
}

#[test]
fn untyped_binder_past_the_declared_telescope_is_rejected() {
    let file = parse("def f : Nat -> Nat := fun n => fun m => n\n").unwrap();
    let out = compile_fol(&file);
    assert!(out
        .errors
        .iter()
        .any(|e| e.kind == ErrorKind::ElabUntypedBinder));
}

#[test]
fn partial_hole_reports_the_remaining_goal() {
    let file = parse("example : (a : Prop) -> a -> a := fun (a : Prop) => sorry\n").unwrap();
    let out = compile_fol(&file);
    assert_eq!(out.errors, vec![]);
    assert!(matches!(&out.events[0], CheckEvent::ExerciseOpen { .. }));
    let report = check_document(&file);
    assert_eq!(report.decls[0].status, DeclStatus::Open);
    assert_eq!(report.decls[0].goal.as_deref(), Some("a -> a"));
}

#[test]
fn partial_hole_with_inferred_binder_reports_goal() {
    let file = parse("def add1 : Nat -> Nat := fun n => sorry\n").unwrap();
    let report = check_document(&file);
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    assert_eq!(report.decls[0].status, DeclStatus::Open);
    assert_eq!(report.decls[0].goal.as_deref(), Some("Nat"));
}

#[test]
fn hole_outside_the_lambda_tail_is_rejected() {
    let file = parse("def bad : Nat -> Nat := fun (n : Nat) => n + sorry\n").unwrap();
    let out = compile_fol(&file);
    assert!(out
        .errors
        .iter()
        .any(|e| e.kind == ErrorKind::ElabHoleMisplaced));
}

#[test]
fn bare_names_of_native_nat_terminate_in_reduce() {
    // I6 验收项：裸名不会被 delta 无限展开（原生快路径边界守护）。
    let file = parse("#reduce Nat.add\n#reduce Nat.succ\n").unwrap();
    let out = compile_fol(&file);
    assert_eq!(out.errors, vec![], "{:?}", out.errors);
    assert!(matches!(
        &out.events[0],
        CheckEvent::Reduced { text, .. } if text == "Nat.add"
    ));
    assert!(matches!(
        &out.events[1],
        CheckEvent::Reduced { text, .. } if text == "Nat.succ"
    ));
}

#[test]
fn partial_hole_records_introduced_binders() {
    let file = parse("example : (a : Prop) -> a -> a := fun (a : Prop) => fun (h : a) => sorry\n")
        .unwrap();
    let report = check_document(&file);
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    let d = &report.decls[0];
    assert_eq!(d.status, DeclStatus::Open);
    assert_eq!(d.goal.as_deref(), Some("a"));
    assert_eq!(
        d.binders,
        vec![
            GoalBinder {
                name: "a".to_string(),
                ty: "Prop".to_string(),
            },
            GoalBinder {
                name: "h".to_string(),
                ty: "a".to_string(),
            },
        ]
    );
}

#[test]
fn partial_hole_untyped_binder_borrows_declared_type() {
    let file = parse("def f : Nat -> Nat := fun n => sorry\n").unwrap();
    let report = check_document(&file);
    let d = &report.decls[0];
    assert_eq!(
        d.binders,
        vec![GoalBinder {
            name: "n".to_string(),
            ty: "Nat".to_string()
        }]
    );
    assert_eq!(d.goal.as_deref(), Some("Nat"));
}

#[test]
fn kernel_rejection_reports_expected_and_actual() {
    let file = parse("def bad : Prop -> Type := fun (x : Prop) => x\n").unwrap();
    let out = compile_fol(&file);
    assert_eq!(out.errors.len(), 1, "errors: {:?}", out.errors);
    let err = &out.errors[0];
    assert_eq!(err.kind, ErrorKind::KernelRejected);
    assert!(
        err.message.contains("期望") && err.message.contains("实际"),
        "message should be the Chinese teaching text with both sides: {}",
        err.message
    );
    assert!(
        !message_looks_like_panic_trace(&err.message),
        "panic trace leaked into the learner-facing message: {}",
        err.message
    );
    let expected = err
        .expected
        .as_deref()
        .expect("kernel rejection should carry the expected side");
    let actual = err
        .actual
        .as_deref()
        .expect("kernel rejection should carry the actual side");
    assert!(!expected.is_empty(), "expected side is empty");
    assert!(!actual.is_empty(), "actual side is empty");
    assert_ne!(
        expected, actual,
        "sides should differ: {expected} vs {actual}"
    );
}

fn message_looks_like_panic_trace(message: &str) -> bool {
    message.contains("panicked at") || message.contains("RUST_BACKTRACE")
}

#[test]
fn kernel_failed_declaration_frees_its_name() {
    // Check-then-add: a kernel-rejected declaration must not occupy its name.
    // `uses_bad` would typecheck in pass 1 (the rejected decl is still in the
    // env); the recomputed pass must report it as unknown instead.
    let file = parse(
        "def bad : Prop -> Type := fun (x : Prop) => x\n\
         def uses_bad : Prop -> Type := bad\n",
    )
    .unwrap();
    let out = compile_fol(&file);
    assert!(
        out.errors
            .iter()
            .any(|e| e.kind == ErrorKind::KernelRejected),
        "the bad decl must stay rejected: {:?}",
        out.errors
    );
    assert!(
        out.errors
            .iter()
            .any(|e| e.kind == ErrorKind::ElabUnknownIdentifier),
        "uses_bad must fail with unknown-identifier, not inherit pass-1 success: {:?}",
        out.errors
    );
    assert!(
        out.events
            .iter()
            .all(|e| !matches!(e, CheckEvent::DeclarationChecked { .. })),
        "no declaration may be reported checked: {:?}",
        out.events
    );
}

// ---- I9 余项：开放声明携带宇宙参数（goal 视图 / tactic 判定可用）----

#[test]
fn open_exercise_carries_universe_params() {
    let report = check_document(&parse("def id {u} : {α : Sort u} -> (a : α) -> α := fun {α : Sort u} => fun (a : α) => a\n\
         theorem eq_symm {u} : {α : Sort u} -> (a : α) -> (b : α) -> Eq.{u} α a b -> Eq.{u} α b a := sorry\n").expect("parse"));
    let open = report
        .decls
        .iter()
        .find(|d| d.status == DeclStatus::Open)
        .expect("open exercise");
    assert_eq!(open.universe, vec!["u".to_string()]);
    assert!(
        open.goal
            .as_deref()
            .is_some_and(|goal| goal.contains("Sort u") && goal.contains("Eq.{u}")),
        "remaining goal keeps its Sort u shape: {:?}",
        open.goal
    );
    // 非开放声明不携带（无需）。
    let checked = report
        .decls
        .iter()
        .find(|d| d.name.as_deref() == Some("id"))
        .expect("checked id");
    assert!(checked.universe.is_empty());
}

#[test]
fn judge_uses_carried_universe_for_sort_u_goals() {
    // 判定链路端到端：目标引用 Sort u，判定规格必须携带 {u}，
    // 否则合成声明里 Sort u 是未声明宇宙变量。
    let open = crate::judge::OpenGoalSpec {
        universe: vec!["u".to_string()],
        ty: "α".to_string(),
        binders: vec![
            crate::judge::GoalBinderSpec {
                name: "α".to_string(),
                ty: Some("Sort u".to_string()),
            },
            crate::judge::GoalBinderSpec {
                name: "a".to_string(),
                ty: Some("α".to_string()),
            },
        ],
    };
    let judgements = crate::judge::judge_terms(
        "def id {u} : {α : Sort u} -> (a : α) -> α := fun {α : Sort u} => fun (a : α) => a\n",
        &CompileOptions::default(),
        &open,
        &["a"],
    );
    assert_eq!(judgements[0], crate::judge::Judgement::Match);
}

// ---- 内核错误分类学：refine_kernel_kind 消息族 → ErrorKind ----

#[test]
fn refine_kernel_kind_classifies_kernel_message_families() {
    use super::error::refine_kernel_kind;
    let cases: &[(&str, ErrorKind)] = &[
        (
            "rejected: expected a sort, got: Nat",
            ErrorKind::KernelExpectedSort,
        ),
        ("expected a sort, got: Nat", ErrorKind::KernelExpectedSort),
        // conv 站点的同名消息（is_prop_type）措辞不同，但共享
        // `expected a sort` 前缀，必须落进同一个族。
        (
            "rejected: expected a sort in conversion, got: ($0 3)",
            ErrorKind::KernelExpectedSort,
        ),
        ("expected a sort in conversion, got: ($0 3)", ErrorKind::KernelExpectedSort),
        (
            "rejected: expected a pi type, got: Nat",
            ErrorKind::KernelExpectedPi,
        ),
        (
            "expected a pi type, got: Nat -> Nat",
            ErrorKind::KernelExpectedPi,
        ),
        // eval 求值路径的“对非函数继续应用”消息也归 expected-pi 族。
        (
            "rejected: spine_type_with_value: expected Pi",
            ErrorKind::KernelExpectedPi,
        ),
        // 显式消去子（rec/iota）规则与内核推导不一致。
        (
            "rejected: iota rule is not listed in constructor declaration order",
            ErrorKind::KernelRecRuleMismatch,
        ),
        (
            "iota rule count does not match the constructor count: 1 iota rules for 2 constructors",
            ErrorKind::KernelRecRuleMismatch,
        ),
        (
            "rejected: imported recursor rule does not match the reconstructed rule",
            ErrorKind::KernelRecRuleMismatch,
        ),
        (
            "imported inductive block contains an underived recursor",
            ErrorKind::KernelRecRuleMismatch,
        ),
        (
            "rejected: theorem type must be Prop (sort 0): Nat",
            ErrorKind::KernelTheoremNotProp,
        ),
        (
            "rejected: non-positive occurrence",
            ErrorKind::KernelNonPositive,
        ),
        ("non-positive occurrence", ErrorKind::KernelNonPositive),
        (
            "rejected: constructor must return a full application of the inductive being declared",
            ErrorKind::KernelCtorResultMismatch,
        ),
        (
            "constructor must return a full application of the inductive being declared",
            ErrorKind::KernelCtorResultMismatch,
        ),
        (
            "rejected: recursive occurrence in constructor is not a valid application of the inductives being declared",
            ErrorKind::KernelCtorArgInvalidApp,
        ),
        (
            "recursive occurrence in constructor is not a valid application of the inductives being declared",
            ErrorKind::KernelCtorArgInvalidApp,
        ),
        (
            "rejected: constructor argument is not a type",
            ErrorKind::KernelCtorArgNotType,
        ),
        (
            "constructor argument is not a type: NotASort",
            ErrorKind::KernelCtorArgNotType,
        ),
        (
            "rejected: Constructor argument was too large for the corresponding inductive type",
            ErrorKind::KernelCtorArgTooLarge,
        ),
        (
            "Constructor argument was too large for the corresponding inductive type",
            ErrorKind::KernelCtorArgTooLarge,
        ),
        // def_eq 双侧消息由 check.rs 的 parse_def_eq_mismatch 单独解析，
        // 分类器必须维持 KernelRejected，不得重写。
        (
            "rejected: def_eq failed: def_eq mismatch expected: Prop | actual: Type",
            ErrorKind::KernelRejected,
        ),
        (
            "def_eq failed: def_eq mismatch expected: Prop | actual: Type",
            ErrorKind::KernelRejected,
        ),
        (
            "def_eq mismatch expected: Prop | actual: Type",
            ErrorKind::KernelRejected,
        ),
    ];
    for (msg, kind) in cases {
        assert_eq!(&refine_kernel_kind(msg), kind, "message: {msg}");
    }
}

#[test]
fn refine_kernel_kind_internal_shapes_stay_kernel_internal() {
    use super::error::refine_kernel_kind;
    for msg in [
        "rejected: assertion failed: x == y",
        "assertion failed: self.is_valid_ind_app_v(st, parent_ind_name, depth, cur)",
        "called `Option::unwrap()` on a `None` value",
        "called `Result::unwrap()` on an `Err` value",
        "internal error: entered unreachable code",
    ] {
        assert_eq!(
            refine_kernel_kind(msg),
            ErrorKind::KernelInternal,
            "message: {msg}"
        );
    }
}

#[test]
fn kernel_fine_grained_kinds_stage_as_kernel_with_codes() {
    let kinds = [
        ErrorKind::KernelExpectedSort,
        ErrorKind::KernelExpectedPi,
        ErrorKind::KernelTheoremNotProp,
        ErrorKind::KernelNonPositive,
        ErrorKind::KernelCtorResultMismatch,
        ErrorKind::KernelCtorArgInvalidApp,
        ErrorKind::KernelCtorArgNotType,
        ErrorKind::KernelCtorArgTooLarge,
        ErrorKind::KernelRecRuleMismatch,
    ];
    let codes = [
        "kernel-expected-sort",
        "kernel-expected-pi",
        "kernel-theorem-not-prop",
        "kernel-inductive-non-positive",
        "kernel-ctor-result-mismatch",
        "kernel-ctor-arg-invalid-app",
        "kernel-ctor-arg-not-type",
        "kernel-ctor-arg-too-large",
        "kernel-rec-rule-mismatch",
    ];
    for (kind, code) in kinds.into_iter().zip(codes) {
        assert_eq!(kind.stage(), CompileStage::Kernel);
        assert_eq!(kind.code(), code);
        assert!(!kind.hint().is_empty(), "missing hint for {code}");
    }
}

#[test]
fn pipeline_classifies_theorem_not_prop() {
    let file = parse("theorem t : Nat := 1\n").unwrap();
    let out = compile_fol(&file);
    assert!(
        out.errors
            .iter()
            .any(|e| e.kind == ErrorKind::KernelTheoremNotProp),
        "errors: {:?}",
        out.errors
            .iter()
            .map(|e| (&e.kind, &e.message))
            .collect::<Vec<_>>()
    );
}

#[test]
fn pipeline_classifies_expected_sort() {
    let file = parse("def x : 1 := 1\n").unwrap();
    let out = compile_fol(&file);
    assert!(
        out.errors
            .iter()
            .any(|e| e.kind == ErrorKind::KernelExpectedSort),
        "errors: {:?}",
        out.errors
            .iter()
            .map(|e| (&e.kind, &e.message))
            .collect::<Vec<_>>()
    );
}

#[test]
fn pipeline_classifies_ctor_result_mismatch() {
    // 递归构造子让块走到 check_ctor；rec 块是教学语法必备（无 rec 的块会
    // 自动派生 recursor，mk 的结果类型错误仍由内核优先报出）。
    let file = parse(
        "inductive Bad : Type\n\
         ctor base : (b : Bad) -> Bad\n\
         ctor mk : Nat\n\
         rec Bad.rec {u} : (motive : (x : Bad) -> Sort u) -> (m0 : (b : Bad) -> motive (base b)) -> (m1 : motive mk) -> (x : Bad) -> motive x\n\
         iota base := fun (motive : (x : Bad) -> Sort u) => fun (m0 : (b : Bad) -> motive (base b)) => fun (m1 : motive mk) => fun (b : Bad) => m0 b\n\
         iota mk := fun (motive : (x : Bad) -> Sort u) => fun (m0 : (b : Bad) -> motive (base b)) => fun (m1 : motive mk) => m1\n\
         end\n",
    )
    .unwrap();
    let out = compile_fol(&file);
    assert!(
        out.errors
            .iter()
            .any(|e| e.kind == ErrorKind::KernelCtorResultMismatch),
        "errors: {:?}",
        out.errors
            .iter()
            .map(|e| (&e.kind, &e.message))
            .collect::<Vec<_>>()
    );
}

#[test]
fn pipeline_classifies_iota_rules_out_of_order() {
    // 内核冷路径分诊：iota 规则顺序写反是学习者错误，必须给出稳定码
    // kernel-rec-rule-mismatch（此前落进裸 assert_eq 的指针调试输出）。
    let file = parse(
        "inductive MyNat : Type\n\
         ctor z : MyNat\n\
         ctor s (n : MyNat) : MyNat\n\
         rec MyNat.rec {u} : (motive : (n : MyNat) -> Sort u) -> (mz : motive z) -> (ms : (n : MyNat) -> motive n -> motive (s n)) -> (n : MyNat) -> motive n\n\
         iota s := fun (motive : (n : MyNat) -> Sort u) => fun (mz : motive z) => fun (ms : (n : MyNat) -> motive n -> motive (s n)) => fun (n : MyNat) => ms n (MyNat.rec.{u} motive mz ms n)\n\
         iota z := fun (motive : (n : MyNat) -> Sort u) => fun (mz : motive z) => fun (ms : (n : MyNat) -> motive n -> motive (s n)) => mz\n\
         end\n",
    )
    .unwrap();
    let out = compile_fol(&file);
    let err = out
        .errors
        .first()
        .expect("out-of-order iota rules must be rejected");
    assert_eq!(err.code(), "kernel-rec-rule-mismatch");
    assert!(
        !err.hint().is_empty(),
        "rec-rule mismatch must carry a hint"
    );
    assert!(
        err.message
            .contains("iota rule is not listed in constructor declaration order"),
        "message: {}",
        err.message
    );
}

#[test]
fn pipeline_classifies_missing_iota_rule() {
    // 少写一条 iota 规则同样是学习者错误，与顺序错误共用一个族。
    let file = parse(
        "inductive MyNat : Type\n\
         ctor z : MyNat\n\
         ctor s (n : MyNat) : MyNat\n\
         rec MyNat.rec {u} : (motive : (n : MyNat) -> Sort u) -> (mz : motive z) -> (ms : (n : MyNat) -> motive n -> motive (s n)) -> (n : MyNat) -> motive n\n\
         iota z := fun (motive : (n : MyNat) -> Sort u) => fun (mz : motive z) => fun (ms : (n : MyNat) -> motive n -> motive (s n)) => mz\n\
         end\n",
    )
    .unwrap();
    let out = compile_fol(&file);
    let err = out
        .errors
        .first()
        .expect("an incomplete iota rule set must be rejected");
    assert_eq!(err.code(), "kernel-rec-rule-mismatch");
    assert!(
        !err.hint().is_empty(),
        "rec-rule mismatch must carry a hint"
    );
    assert!(
        err.message
            .contains("iota rule count does not match the constructor count"),
        "message: {}",
        err.message
    );
}

// ---- 多洞（multi-hole）与 refine 模板（I9 第二段）----

const AND_SKELETON: &str = "\
axiom True : Prop\n\
axiom True.intro : True\n\
axiom False : Prop\n\
axiom And : Prop -> Prop -> Prop\n\
axiom And.intro : (a : Prop) -> (b : Prop) -> a -> b -> And a b\n\
axiom And.left : (a : Prop) -> (b : Prop) -> And a b -> a\n\
axiom And.right : (a : Prop) -> (b : Prop) -> And a b -> b\n";

#[test]
fn constructor_spine_holes_are_multi_hole_open_exercises() {
    let report = check_document(
        &parse(&format!(
            "{AND_SKELETON}example : (a : Prop) -> (b : Prop) -> And a b -> And b a := \
         fun (a : Prop) => fun (b : Prop) => fun (h : And a b) => And.intro sorry sorry\n"
        ))
        .expect("parse"),
    );
    let open = report
        .decls
        .iter()
        .find(|d| d.status == DeclStatus::Open)
        .expect("multi-hole answer must stay an open exercise");
    assert_eq!(open.holes.len(), 2, "two spine holes");
    assert_eq!(open.sub_goals.len(), 2);
    // 字段类型经结果头参数实例化：`And b a` 里 a:=b、b:=a。
    assert_eq!(open.sub_goals[0].ty.as_deref(), Some("b"));
    assert_eq!(open.sub_goals[1].ty.as_deref(), Some("a"));
    assert!(open.refine_template.is_none(), "already refined");
    assert!(
        report
            .errors
            .iter()
            .all(|e| e.kind != ErrorKind::ElabHoleMisplaced),
        "spine holes are legal: {:?}",
        report.errors
    );
}

// ---- 函数实参洞（function-spine holes，2026-09-10）----

/// 找到唯一 open 练习（含洞 span 文本校验辅助）。
fn open_exercise(report: &DocumentReport, expect_holes: usize) -> &DeclState {
    let open = report
        .decls
        .iter()
        .find(|d| d.status == DeclStatus::Open)
        .expect("open exercise");
    assert_eq!(open.holes.len(), expect_holes, "holes: {:?}", open.holes);
    open
}

#[test]
fn eq_subst_argument_hole_expects_instantiated_binder_type() {
    // 用户原始需求（playground.sokonanoda:233）：谓词实参改写成 sorry 后，
    // 洞的期望类型是 `Nat -> Prop`（binder `{p : α -> Prop}` 中 α:=Nat）。
    let src = concat!(
        "theorem eq_symm_nat : (a : Nat) -> (b : Nat) -> Eq.{1} Nat a b -> Eq.{1} Nat b a :=\n",
        "  fun (a : Nat) (b : Nat) (h : Eq.{1} Nat a b) =>\n",
        "    Eq.subst.{1} Nat (sorry) a b h (Eq.refl.{1} Nat a)\n",
    );
    let report = check_document(&parse(src).expect("parse"));
    let open = open_exercise(&report, 1);
    assert_eq!(open.sub_goals.len(), 1);
    assert_eq!(
        open.sub_goals[0].ty.as_deref(),
        Some("Nat -> Prop"),
        "sub_goals: {:?}",
        open.sub_goals
    );
    assert_eq!(
        &src[open.holes[0].start.offset..open.holes[0].end.offset],
        "sorry"
    );
    assert!(
        report.errors.is_empty(),
        "function argument holes are legal: {:?}",
        report.errors
    );
}

#[test]
fn later_function_hole_uses_preceding_arguments() {
    // 末位洞期望 `p a`：p、a 来自前置实参的 AST 替换。
    let src = concat!(
        "theorem t : (p : Nat -> Prop) -> (a : Nat) -> Eq.{1} Nat a a -> p a :=\n",
        "  fun (p : Nat -> Prop) (a : Nat) (h : Eq.{1} Nat a a) =>\n",
        "    Eq.subst.{1} Nat p a a h (sorry)\n",
    );
    let report = check_document(&parse(src).expect("parse"));
    let open = open_exercise(&report, 1);
    assert_eq!(open.sub_goals[0].ty.as_deref(), Some("p a"));
}

#[test]
fn eq_refl_argument_hole_expects_the_type_argument() {
    let src =
        "theorem t : (a : Nat) -> Eq.{1} Nat a a :=\n  fun (a : Nat) => Eq.refl.{1} Nat (sorry)\n";
    let report = check_document(&parse(src).expect("parse"));
    let open = open_exercise(&report, 1);
    assert_eq!(open.sub_goals[0].ty.as_deref(), Some("Nat"));
}

#[test]
fn function_hole_reports_substituted_universe_sort() {
    // 洞在 α 位：调用点 `. {1}` 把模板里的 `Sort u` 替换成 `Sort 1`。
    let src =
        "theorem t : (a : Nat) -> Eq.{1} Nat a a :=\n  fun (a : Nat) => Eq.refl.{1} (sorry) a\n";
    let report = check_document(&parse(src).expect("parse"));
    let open = open_exercise(&report, 1);
    assert_eq!(open.sub_goals[0].ty.as_deref(), Some("Sort 1"));
}

#[test]
fn function_hole_after_another_hole_has_no_expected_type() {
    // 前置实参本身是洞：`p a` 无法实例化 → ty = None（面板显示 `?`）。
    let src = concat!(
        "theorem t : (a : Nat) -> (b : Nat) -> Eq.{1} Nat a b -> Eq.{1} Nat b a :=\n",
        "  fun (a : Nat) (b : Nat) (h : Eq.{1} Nat a b) =>\n",
        "    Eq.subst.{1} Nat (sorry) a b h (sorry)\n",
    );
    let report = check_document(&parse(src).expect("parse"));
    let open = open_exercise(&report, 2);
    assert_eq!(open.sub_goals[0].ty.as_deref(), Some("Nat -> Prop"));
    assert_eq!(open.sub_goals[1].ty, None);
}

#[test]
fn source_axiom_argument_holes_use_the_function_telescope() {
    // 源内 axiom：`False.rec`（结果不是族应用）与 `Or.inr`（多构造子族，
    // 既有 ctor 表每族只留第一个构造子）都走函数模板。
    let src = concat!(
        "axiom False : Prop\n",
        "axiom False.rec : (P : Prop) -> False -> P\n",
        "axiom Or : Prop -> Prop -> Prop\n",
        "axiom Or.inl : (a : Prop) -> (b : Prop) -> a -> Or a b\n",
        "axiom Or.inr : (a : Prop) -> (b : Prop) -> b -> Or a b\n",
        "example : (a : Prop) -> (b : Prop) -> b -> Or a b :=\n",
        "  fun (a : Prop) (b : Prop) (hb : b) => Or.inr a b (sorry)\n",
        "example : (P : Prop) -> False -> P :=\n",
        "  fun (P : Prop) (h : False) => False.rec (sorry) (sorry)\n",
    );
    let report = check_document(&parse(src).expect("parse"));
    let opens: Vec<&DeclState> = report
        .decls
        .iter()
        .filter(|d| d.status == DeclStatus::Open)
        .collect();
    assert_eq!(opens.len(), 2, "two open exercises");
    assert_eq!(opens[0].sub_goals[0].ty.as_deref(), Some("b"));
    assert_eq!(opens[1].sub_goals[0].ty.as_deref(), Some("Prop"));
    assert_eq!(opens[1].sub_goals[1].ty.as_deref(), Some("False"));
    assert!(
        report.errors.is_empty(),
        "source axiom holes are legal: {:?}",
        report.errors
    );
}

#[test]
fn user_defined_function_argument_hole_gets_its_binder_type() {
    let src = concat!(
        "def add1 : Nat -> Nat := fun n => n + 1\n",
        "theorem t : Nat -> Nat := fun (n : Nat) => add1 (sorry)\n",
    );
    let report = check_document(&parse(src).expect("parse"));
    let open = open_exercise(&report, 1);
    assert_eq!(open.sub_goals[0].ty.as_deref(), Some("Nat"));
}

#[test]
fn nested_function_hole_is_still_misplaced() {
    // v1 边界：嵌套洞（实参是含洞的 lambda）不恢复目标，仍报 misplaced。
    let src = concat!(
        "theorem eq_symm_nat : (a : Nat) -> (b : Nat) -> Eq.{1} Nat a b -> Eq.{1} Nat b a :=\n",
        "  fun (a : Nat) (b : Nat) (h : Eq.{1} Nat a b) =>\n",
        "    Eq.subst.{1} Nat (fun (x : Nat) => sorry) a b h (Eq.refl.{1} Nat a)\n",
    );
    let report = check_document(&parse(src).expect("parse"));
    assert!(
        report
            .errors
            .iter()
            .any(|e| e.kind == ErrorKind::ElabHoleMisplaced),
        "nested holes stay misplaced in v1: {:?}",
        report.errors
    );
}

#[test]
fn single_hole_with_ctor_goal_gets_refine_template() {
    let report = check_document(&parse(&format!(
        "{AND_SKELETON}theorem and_intro_rule : (a : Prop) -> (b : Prop) -> a -> b -> And a b := sorry\n"
    )).expect("parse"));
    let open = report
        .decls
        .iter()
        .find(|d| d.status == DeclStatus::Open)
        .expect("open");
    // 结果头参数（a、b）被目标确定 → 自动填入；证明字段成为 sorry
    assert_eq!(
        open.refine_template.as_deref(),
        Some("And.intro a b sorry sorry")
    );
    assert_eq!(open.holes.len(), 1);
    assert!(open.sub_goals.is_empty());
}

#[test]
fn mixed_spine_args_keep_state_open_with_expected_types() {
    let report = check_document(
        &parse(&format!(
            "{AND_SKELETON}example : And True False := And.intro True sorry\n"
        ))
        .expect("parse"),
    );
    assert!(
        !report
            .errors
            .iter()
            .any(|e| e.kind == ErrorKind::ElabHoleMisplaced),
        "mixed constructor application stays an open exercise: {:?}",
        report.errors
    );
    let open = report
        .decls
        .iter()
        .find(|d| d.status == DeclStatus::Open)
        .expect("open");
    assert_eq!(open.holes.len(), 1);
    assert_eq!(open.sub_goals.len(), 1);
    // 字段 `b` 在目标 `And True False` 下实例化为 False。
    assert_eq!(open.sub_goals[0].ty.as_deref(), Some("False"));
}

#[test]
fn sub_goal_field_types_substitute_compound_binders() {
    // 复合字段类型（`And a b`）里的模板 binder 名同样替换为 goal 自己的
    // 实参：目标 `Pair True False` 下子洞类型是 `And True False`，而不是
    // 原样渲染的模板名 `And a b`（旧实现只处理裸 Ident 字段类型）。
    let report = check_document(
        &parse(
            "axiom Pair : Prop -> Prop -> Prop\n\
             axiom Pair.mk : (a : Prop) -> (b : Prop) -> And a b -> Pair a b\n\
             example : Pair True False := Pair.mk True False sorry\n",
        )
        .expect("parse"),
    );
    let open = report
        .decls
        .iter()
        .find(|d| d.status == DeclStatus::Open)
        .expect("ctor-spine hole stays an open exercise");
    assert_eq!(open.holes.len(), 1);
    assert_eq!(open.sub_goals.len(), 1);
    assert_eq!(
        open.sub_goals[0].ty.as_deref(),
        Some("(And True) False"),
        "compound field type must show the goal's own arguments: {:?}",
        open.sub_goals[0].ty
    );
    assert!(
        !report
            .errors
            .iter()
            .any(|e| e.kind == ErrorKind::ElabHoleMisplaced),
        "spine holes stay legal: {:?}",
        report.errors
    );
}

#[test]
fn sub_goal_field_types_respect_binder_shadowing() {
    // 遮蔽守卫（innermost wins，与 elab 同）：字段
    // `forall (a : Prop), And a b` 的内层 binder `a` 遮蔽模板 binder `a`，
    // 其作用域内不再替换（内层 a 保持）；未被遮蔽的 `b` 仍替换为 goal 实参。
    let report = check_document(
        &parse(
            "axiom True : Prop\n\
             axiom False : Prop\n\
             axiom And : Prop -> Prop -> Prop\n\
             axiom And.mk : (a : Prop) -> (b : Prop) -> \
             (h : forall (a : Prop), And a b) -> And a b\n\
             example : And True False := And.mk True False sorry\n",
        )
        .expect("parse"),
    );
    let open = report
        .decls
        .iter()
        .find(|d| d.status == DeclStatus::Open)
        .expect("ctor-spine hole stays an open exercise");
    assert_eq!(open.sub_goals.len(), 1);
    assert_eq!(
        open.sub_goals[0].ty.as_deref(),
        Some("(a : Prop) -> (And a) False"),
        "shadowed inner `a` stays, unshadowed `b` is substituted: {:?}",
        open.sub_goals[0].ty
    );
}

#[test]
fn spine_holes_without_a_known_template_stay_open() {
    // 没有兄弟 axiom/ctor 模板也能合法多洞（子目标类型缺省）。
    let report = check_document(
        &parse("example : (A : Prop -> Prop) -> A -> A := fun (A : Prop -> Prop) => sorry\n")
            .expect("parse"),
    );
    let open = report
        .decls
        .iter()
        .find(|d| d.status == DeclStatus::Open)
        .expect("single hole stays open");
    assert_eq!(open.holes.len(), 1);
    assert!(open.refine_template.is_none());
}

// ---- sorry（与官方 Lean 同义的占位符，等价 sorry）----

#[test]
fn sorry_is_a_hole_and_stays_an_open_exercise() {
    let report = check_document(
        &parse("theorem t : Prop -> Prop := fun (x : Prop) => sorry\n").expect("parse"),
    );
    let open = report
        .decls
        .iter()
        .find(|d| d.status == DeclStatus::Open)
        .expect("sorry keeps the exercise open");
    assert_eq!(open.holes.len(), 1);
    assert!(
        report.errors.is_empty(),
        "sorry must not be an error: {:?}",
        report.errors
    );
}

#[test]
fn sorry_works_in_constructor_spines_and_partial_answers() {
    let src = format!(
        "{AND_SKELETON}example : (a : Prop) -> (b : Prop) -> And a b -> And b a := \
         fun (a : Prop) => fun (b : Prop) => fun (h : And a b) => And.intro sorry sorry\n"
    );
    let report = check_document(&parse(&src).expect("parse"));
    let open = report
        .decls
        .iter()
        .find(|d| d.status == DeclStatus::Open)
        .expect("multi-hole spine with sorry stays open");
    assert_eq!(open.holes.len(), 2);
    // 洞 span 指向两处 sorry 文本（供跳洞/面板使用）。
    let hole_texts: Vec<&str> = open
        .holes
        .iter()
        .map(|sp| &src[sp.start.offset..sp.end.offset])
        .collect();
    assert_eq!(hole_texts, vec!["sorry", "sorry"]);
}

#[test]
fn sorry_outside_the_answer_tail_is_still_misplaced() {
    let report = check_document(
        &parse("def bad : Nat -> Nat := fun (n : Nat) => sorry + 1\n").expect("parse"),
    );
    assert!(
        report
            .errors
            .iter()
            .any(|e| e.kind == ErrorKind::ElabHoleMisplaced),
        "sorry in a non-tail position follows the sorry rules: {:?}",
        report.errors
    );
}

#[test]
fn hover_rows_name_loose_bvars_instead_of_indices() {
    // 学习者痛点：lambda 下的类型曾渲染成 "1 -> 1"（松散变量打成了
    // de Bruijn 序号）。现在 $N 会被作用域名字替换。
    let src = "theorem demo_K : (a : Prop) -> a -> a :=\n  fun (a : Prop) => fun (h : a) => h\n";
    let report = check_document(&parse(src).expect("parse"));
    for h in &report.hovers {
        assert!(
            !h.text.contains('$'),
            "hover text must not leak de Bruijn indices: {:?}",
            h.text
        );
    }
}

// ---- 括号 hover 语料：and_not_absurd（用户指定样例）----

#[test]
fn hover_rows_of_and_not_absurd_show_real_names() {
    // 用户指定语料：括号应用 + Not 定义展开 + `$N` 还原，全部集中在
    // 一条 and_not_absurd 里。所有 hover 行必须用真名（零 `$`），
    // 关键行的类型精确匹配（kernel 判定，pp 只负责还原名字）。
    let src = concat!(
        "axiom False : Prop\n",
        "axiom And : Prop -> Prop -> Prop\n",
        "axiom And.intro : (a : Prop) -> (b : Prop) -> a -> b -> And a b\n",
        "axiom And.left : (a : Prop) -> (b : Prop) -> And a b -> a\n",
        "axiom And.right : (a : Prop) -> (b : Prop) -> And a b -> b\n",
        "def Not : Prop -> Prop := fun (a : Prop) => a -> False\n",
        "theorem and_not_absurd : (a : Prop) -> And a (Not a) -> False :=\n",
        "  fun (a : Prop) (h : And a (Not a)) => ",
        "(And.right a (Not a) h) (And.left a (Not a) h)\n",
    );
    let report = check_document(&parse(src).expect("parse"));
    for h in &report.hovers {
        let slice = &src[h.span.start.offset..h.span.end.offset];
        assert!(
            !h.text.contains('$'),
            "leaked de Bruijn index in {slice:?}: {:?}",
            h.text
        );
    }
    // 行定位：AST span 不含括号，应用链的起点 = 括号后第一个字符。
    let right_group = src.find("(And.right").expect("right group") + 1;
    let left_group = src.find("(And.left").expect("left group") + 1;
    let right_app_end = right_group + "And.right a (Not a) h".len();
    let left_app_end = left_group + "And.left a (Not a) h".len();
    // 部分应用的 span 止于 `(Not a)` 的 `a`（AST span 不含括号）
    let right_partial_end = right_group + "And.right a (Not a".len();
    let h_of_right = right_group + "And.right a (Not a) ".len();
    let row = |start: usize, end: usize| {
        report
            .hovers
            .iter()
            .find(|h| h.span.start.offset == start && h.span.end.offset == end)
            .unwrap_or_else(|| panic!("no hover row at {start}..{end}"))
    };
    // 用户样例 1：`(And.right a (Not a) h)` → `Not a`（Not 保持折叠）
    assert_eq!(row(right_group, right_app_end).text, "Not a");
    // 用户样例 2：`And.left a (Not a) h` → `a`
    assert_eq!(row(left_group, left_app_end).text, "a");
    // 假设的使用：`h` → `And a (Not a)`
    assert_eq!(row(h_of_right, h_of_right + 1).text, "And a (Not a)");
    // 部分应用：`And.right a (Not a)` → `And a (Not a) -> Not a`
    assert_eq!(
        row(right_group, right_partial_end).text,
        "And a (Not a) -> Not a"
    );
    // 整条应用链 → `False`
    assert_eq!(row(right_group, left_app_end).text, "False");
    // lambda 整体 → 带真名的 forall
    let lambda = report
        .hovers
        .iter()
        .find(|h| src[h.span.start.offset..h.span.end.offset].starts_with("fun (a : Prop) (h"))
        .expect("lambda row");
    assert_eq!(lambda.text, "forall (a : Prop), And a (Not a) -> False");
}

#[test]
fn hover_rows_of_partial_applications_use_scope_names() {
    // 部分应用的类型里，scope binder 的引用全部还原为真名
    //（此前是 `$3 -> $4 -> And $3 $4` 这样的索引残渣）。
    let src = concat!(
        "axiom And : Prop -> Prop -> Prop\n",
        "axiom And.intro : (a : Prop) -> (b : Prop) -> a -> b -> And a b\n",
        "axiom And.left : (a : Prop) -> (b : Prop) -> And a b -> a\n",
        "axiom And.right : (a : Prop) -> (b : Prop) -> And a b -> b\n",
        "theorem t : (a : Prop) -> (b : Prop) -> And a b -> And b a :=\n",
        "  fun (a : Prop) (b : Prop) (h : And a b) => ",
        "And.intro b a (And.right a b h) (And.left a b h)\n",
    );
    let report = check_document(&parse(src).expect("parse"));
    for h in &report.hovers {
        let slice = &src[h.span.start.offset..h.span.end.offset];
        assert!(
            !h.text.contains('$'),
            "leaked de Bruijn index in {slice:?}: {:?}",
            h.text
        );
    }
    let row = |start: usize, end: usize| {
        report
            .hovers
            .iter()
            .find(|h| h.span.start.offset == start && h.span.end.offset == end)
            .unwrap_or_else(|| panic!("no hover row at {start}..{end}"))
    };
    // And.intro b a：两个实参的顺序如实显示（b 先、a 后）
    let intro = src.find("And.intro b a").expect("And.intro b a exists");
    assert_eq!(
        row(intro, intro + "And.intro b a".len()).text,
        "b -> a -> And b a"
    );
    // And.right a b：剩余类型里的 scope 引用是真名
    let arb = src.find("And.right a b").expect("And.right a b exists");
    assert_eq!(row(arb, arb + "And.right a b".len()).text, "And a b -> b");
    // 全应用：And.right a b h : b
    assert_eq!(row(arb, arb + "And.right a b h".len()).text, "b");
}

#[test]
fn hover_rows_have_lambda_binder_declaration_rows() {
    // 每个 lambda binder 都有一条 `binder: true` 的声明行，span = 完整标注
    // `(a : Prop)` / `(h : And a (Not a))`——LSP 靠它避免整段 lambda 溢出。
    let src = concat!(
        "axiom False : Prop\n",
        "axiom And : Prop -> Prop -> Prop\n",
        "axiom And.left : (a : Prop) -> (b : Prop) -> And a b -> a\n",
        "axiom And.right : (a : Prop) -> (b : Prop) -> And a b -> b\n",
        "def Not : Prop -> Prop := fun (a : Prop) => a -> False\n",
        "theorem and_not_absurd : (a : Prop) -> And a (Not a) -> False :=\n",
        "  fun (a : Prop) (h : And a (Not a)) => ",
        "(And.right a (Not a) h) (And.left a (Not a) h)\n",
    );
    let report = check_document(&parse(src).expect("parse"));
    let a_span = src.find("(a : Prop)").expect("(a : Prop)");
    assert!(
        report.hovers.iter().any(|h| {
            h.binder
                && h.span.start.offset == a_span
                && h.span.end.offset == a_span + "(a : Prop)".len()
        }),
        "missing binder row for (a : Prop)"
    );
    let h_span = src.find("(h : And a (Not a))").expect("(h ...)");
    assert!(
        report.hovers.iter().any(|h| {
            h.binder
                && h.span.start.offset == h_span
                && h.span.end.offset == h_span + "(h : And a (Not a))".len()
        }),
        "missing binder row for (h : And a (Not a))"
    );
}

#[test]
fn hover_rows_include_pi_binder_rows() {
    // Forall（Pi）的 binder 也必须有声明行：`(P : Prop) -> False -> P` 的 `(P : Prop)`。
    let src = "axiom False : Prop\naxiom my_rec : (P : Prop) -> False -> P\n";
    let report = check_document(&parse(src).expect("parse"));
    let p_span = src.find("(P : Prop)").expect("(P : Prop)");
    assert!(
        report.hovers.iter().any(|h| {
            h.binder
                && h.span.start.offset == p_span
                && h.span.end.offset == p_span + "(P : Prop)".len()
        }),
        "missing binder row for (P : Prop)"
    );
}

#[test]
fn sorry_is_highlighted_as_a_hole() {
    use crate::semantic::{semantic_tokens, SemanticKind};
    let spans = semantic_tokens("theorem t : Prop := sorry\n");
    let sorry = spans
        .iter()
        .find(|s| src_slice_of("theorem t : Prop := sorry\n", s.span) == "sorry")
        .expect("sorry token exists");
    assert_eq!(sorry.kind, SemanticKind::Hole);
}

fn src_slice_of(src: &str, span: crate::Span) -> &str {
    &src[span.start.offset..span.end.offset]
}
// ---- 声明签名 hover（ty_text）----

#[test]
fn decl_state_carries_kernel_rendered_signature() {
    let src = "axiom True : Prop\n\
               axiom And : Prop -> Prop -> Prop\n\
               axiom And.intro : (a : Prop) -> (b : Prop) -> a -> b -> And a b\n\
               theorem t : True := sorry\n";
    let report = check_document(&parse(src).expect("parse"));
    let and_intro = report
        .decls
        .iter()
        .find(|d| d.name.as_deref() == Some("And.intro"))
        .expect("And.intro");
    let ty = and_intro
        .ty_text
        .as_deref()
        .expect("axiom carries signature");
    assert!(
        ty.contains("And a b") && ty.contains("forall") || ty.contains("->"),
        "signature should be the kernel-rendered type: {ty}"
    );
    // 开放练习：ty_text 来自 elaborated type
    let t = report
        .decls
        .iter()
        .find(|d| d.name.as_deref() == Some("t"))
        .expect("open theorem");
    assert_eq!(t.ty_text.as_deref(), Some("True"), "open decl signature");
}
#[test]
fn multi_binder_lambda_parses_and_checks() {
    // 学习者写法（与官方 Lean 同款）：多 binder lambda，
    // 曾在判卷时因编辑中间态误报 parse 错误（F5 防回归）。
    let src = "axiom And : Prop -> Prop -> Prop\n\
               axiom And.intro : (a : Prop) -> (b : Prop) -> a -> b -> And a b\n\
               theorem t : (a : Prop) -> (b : Prop) -> a -> b -> And a b := \
               fun (a : Prop) (b : Prop) (ha : a) (hb : b) => And.intro a b ha hb\n";
    let report = check_document(&parse(src).expect("parse"));
    let t = report
        .decls
        .iter()
        .find(|d| d.name.as_deref() == Some("t"))
        .expect("multi-binder theorem");
    assert_eq!(t.status, DeclStatus::Checked);
    assert!(report.errors.is_empty(), "{:?}", report.errors);
}
#[test]
fn hover_rows_exist_for_proposition_and_axioms() {
    // 验证 axiom 声明和演示行有 hover 行。
    // 注意：lambda 体内应用链的子表达式 hover 行因 infer_under_binders
    // 对需要 delta 展开的中间类型（如 Not a）panic 而缺失——这是内核
    // infer 函数的已知限制，修复需要 kernel 侧改动。
    let src = concat!(
        "axiom And : Prop -> Prop -> Prop\n",
        "axiom And.intro : (a : Prop) -> (b : Prop) -> a -> b -> And a b\n",
        "axiom And.left : (a : Prop) -> (b : Prop) -> And a b -> a\n",
        "axiom And.right : (a : Prop) -> (b : Prop) -> And a b -> b\n",
        "theorem t : (a : Prop) -> (b : Prop) -> a -> b -> And a b :=\n",
        "  fun (a : Prop) (b : Prop) (ha : a) (hb : b) => And.intro a b ha hb\n",
    );
    let report = check_document(&parse(src).expect("parse"));
    // 类型部分（axiom/theorem 类型）的 hover 行应该存在
    assert!(
        report.hovers.iter().any(|h| h.text.contains("Prop")),
        "should have hover rows for type sub-expressions"
    );
    assert!(
        report.hovers.iter().any(|h| h.text.contains("And a b")),
        "should have hover rows mentioning And a b"
    );
}
#[test]
fn debug_hover_bracket_coverage() {
    // 诊断用：看 hover 表在括号位置是否有行覆盖
    let src = "theorem demo_K : (a : Prop) -> a -> a :=\n  fun (a : Prop) => fun (h : a) => h\n";
    let report = check_document(&parse(src).expect("parse"));
    // 打印全部 hover 行
    for h in &report.hovers {
        let slice = &src[h.span.start.offset..h.span.end.offset.min(src.len())];
        eprintln!(
            "ROW {}..{} {:?} src={:?}",
            h.span.start.offset, h.span.end.offset, h.text, slice
        );
    }
    // 括号位置：line 0 的 `(` 在 byte offset（需要换算）
    // `theorem demo_K : ` = 17 chars，`(` 在 17
    // line 0 的 `)` = `)` of `(a : Prop)` at offset 26
    let parens: Vec<(usize, char)> = src
        .char_indices()
        .filter(|(_, c)| *c == '(' || *c == ')')
        .take(8)
        .collect();
    eprintln!("brackets: {:?}", parens);
    for (off, c) in &parens {
        let containing: Vec<&crate::compile::HoverType> = report
            .hovers
            .iter()
            .filter(|h| h.span.start.offset <= *off && *off < h.span.end.offset)
            .collect();
        eprintln!("  hover@{} ({}): {} rows", off, c, containing.len());
        for h in &containing {
            eprintln!(
                "    {}..{} {:?}",
                h.span.start.offset, h.span.end.offset, h.text
            );
        }
    }
}

// ---- by-tactic 块（第十九轮：intro/exact/apply/assumption/rfl）----

#[test]
fn parses_by_block_into_tactics() {
    let src =
        "axiom True : Prop\naxiom True.intro : True\ntheorem t : True := by exact True.intro\n";
    let file = parse(src).unwrap();
    let crate::Command::Theorem { val, .. } = &file.commands[2] else {
        panic!("expected theorem")
    };
    match val {
        crate::Expr::By { tactics, .. } => {
            assert_eq!(tactics.len(), 1);
            assert!(matches!(tactics[0], crate::Tactic::Exact { .. }));
        }
        _ => panic!("expected a `by` value, got {val:?}"),
    }
}

#[test]
fn rejects_unknown_tactic_in_by_block() {
    let src = "theorem t : Prop := by nope\n";
    assert!(parse(src).is_err(), "unknown tactic must be rejected");
}

#[test]
fn by_block_with_intro_exact_checks() {
    let src = concat!(
        "axiom True : Prop\n",
        "axiom And : Prop -> Prop -> Prop\n",
        "axiom And.intro : (a : Prop) -> (b : Prop) -> a -> b -> And a b\n",
        "axiom And.left : (a : Prop) -> (b : Prop) -> And a b -> a\n",
        "axiom And.right : (a : Prop) -> (b : Prop) -> And a b -> b\n",
        "theorem and_swap : (a : Prop) -> (b : Prop) -> And a b -> And b a := by ",
        "intro a; intro b; intro h; exact And.intro b a (And.right a b h) (And.left a b h)\n",
    );
    let report = check_document(&parse(src).unwrap());
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    let d = report
        .decls
        .iter()
        .find(|d| d.name.as_deref() == Some("and_swap"))
        .unwrap();
    assert_eq!(d.status, DeclStatus::Checked);
}

#[test]
fn by_block_with_assumption_checks() {
    let src =
        "axiom True : Prop\ntheorem k : (a : Prop) -> a -> a := by intro a; intro h; assumption\n";
    let report = check_document(&parse(src).unwrap());
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    let d = report
        .decls
        .iter()
        .find(|d| d.name.as_deref() == Some("k"))
        .unwrap();
    assert_eq!(d.status, DeclStatus::Checked);
}

#[test]
fn by_block_with_apply_and_rfl_checks() {
    let src = concat!(
        "axiom And : Prop -> Prop -> Prop\n",
        "axiom And.intro : (a : Prop) -> (b : Prop) -> a -> b -> And a b\n",
        "axiom Or : Prop -> Prop -> Prop\n",
        "axiom Or.inl : (a : Prop) -> (b : Prop) -> a -> Or a b\n",
        "theorem or_left : (a : Prop) -> (b : Prop) -> a -> Or a b := by ",
        "intro a; intro b; intro ha; apply Or.inl; exact ha\n",
        "theorem ai : (a : Prop) -> (b : Prop) -> a -> b -> And a b := by ",
        "intro a; intro b; intro ha; intro hb; apply And.intro; exact ha; exact hb\n",
        "theorem r : Eq.{1} Nat (1 + 1) 2 := by rfl\n",
    );
    let report = check_document(&parse(src).unwrap());
    assert!(report.errors.is_empty(), "{:?}", report.errors);
}

#[test]
fn partial_by_block_is_open_exercise() {
    let src =
        "axiom And : Prop -> Prop -> Prop\ntheorem open : (a : Prop) -> And a a -> a := by intro a; intro h\n";
    let report = check_document(&parse(src).unwrap());
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    let d = report
        .decls
        .iter()
        .find(|d| d.name.as_deref() == Some("open"))
        .unwrap();
    assert_eq!(d.status, DeclStatus::Open);
}

#[test]
fn partial_by_block_records_per_step_states() {
    // Phase 2（soko/stateAt）：每个 tactic 记录执行后的 goal + 上下文 + 源码 span。
    let src =
        "axiom And : Prop -> Prop -> Prop\ntheorem open : (a : Prop) -> And a a -> a := by intro a; intro h\n";
    let report = check_document(&parse(src).unwrap());
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    let d = report
        .decls
        .iter()
        .find(|d| d.name.as_deref() == Some("open"))
        .unwrap();
    assert_eq!(d.by_steps.len(), 2, "one state per tactic");
    // step 0 = `intro a` 执行后：binder a : Prop，目标剩 `And a a -> a`
    // （render_expr 给应用作函数位置补括号：`(And a) a`）。
    let s0 = &d.by_steps[0];
    assert_eq!(s0.goal.as_deref(), Some("(And a) a -> a"));
    assert_eq!(s0.binders.len(), 1);
    assert_eq!(s0.binders[0].name, "a");
    assert_eq!(s0.binders[0].ty, "Prop");
    assert_eq!(&src[s0.span.start.offset..s0.span.end.offset], "intro a");
    // step 1 = `intro h` 执行后：h : And a a，目标剩 `a`。
    let s1 = &d.by_steps[1];
    assert_eq!(s1.goal.as_deref(), Some("a"));
    assert_eq!(s1.binders.len(), 2);
    assert_eq!(s1.binders[1].name, "h");
    assert_eq!(s1.binders[1].ty, "(And a) a");
    assert_eq!(&src[s1.span.start.offset..s1.span.end.offset], "intro h");
}

#[test]
fn checked_by_block_records_closed_final_step() {
    let src =
        "axiom True : Prop\ntheorem t : (a : Prop) -> a -> a := by intro a; intro h; exact h\n";
    let report = check_document(&parse(src).unwrap());
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    let d = report
        .decls
        .iter()
        .find(|d| d.name.as_deref() == Some("t"))
        .unwrap();
    assert_eq!(d.status, DeclStatus::Checked);
    assert_eq!(d.by_steps.len(), 3);
    assert_eq!(d.by_steps[0].goal.as_deref(), Some("a -> a"));
    assert_eq!(d.by_steps[2].goal, None, "all goals closed by `exact`");
}

#[test]
fn wrong_exact_reports_tactic_error() {
    let src = "axiom True : Prop\ntheorem t : (a : Prop) -> a := by intro a; exact True\n";
    let report = check_document(&parse(src).unwrap());
    assert!(!report.errors.is_empty(), "wrong exact must error");
    assert_eq!(report.errors[0].code(), "elab-tactic-failed");
}

#[test]
fn parses_multi_name_binder_group() {
    // 内核 pp 会把 `(a : Prop) -> (b : Prop)` 折叠成 `(a b : Prop)`；
    // 引擎读回类型文本需要多名字 binder 组。
    let src = "axiom P : (a b : Prop) -> Prop\n";
    let file = parse(src).unwrap();
    assert_eq!(file.commands.len(), 1);
}

// ---- by-tactic 错误路径与空 by 块 ----

#[test]
fn empty_by_block_is_open_exercise() {
    // `:= by` 后面什么都没有：从零开始的练习，目标 = 整个声明类型。
    let src = "axiom True : Prop\ntheorem t : (a : Prop) -> a -> a := by\n";
    let report = check_document(&parse(src).unwrap());
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    let d = report
        .decls
        .iter()
        .find(|d| d.name.as_deref() == Some("t"))
        .unwrap();
    assert_eq!(d.status, DeclStatus::Open);
}

#[test]
fn intro_on_non_function_goal_is_a_tactic_error() {
    // 目标 True 没有箭头可拆，intro 应报教学错误而非崩溃。
    let src = "axiom True : Prop\ntheorem t : True := by intro x\n";
    let report = check_document(&parse(src).unwrap());
    assert!(!report.errors.is_empty());
    assert_eq!(report.errors[0].code(), "elab-tactic-failed");
}

#[test]
fn assumption_without_a_match_is_a_tactic_error() {
    // 目标 a，上下文只有 b : Prop，没有类型为 a 的假设。
    let src = "axiom True : Prop\ntheorem t : (a : Prop) -> (b : Prop) -> a := by intro a; intro b; assumption\n";
    let report = check_document(&parse(src).unwrap());
    assert!(!report.errors.is_empty());
    assert_eq!(report.errors[0].code(), "elab-tactic-failed");
}

#[test]
fn rfl_on_non_eq_goal_is_a_tactic_error() {
    // rfl 只对 Eq 形状的目标有效。
    let src = "axiom True : Prop\ntheorem t : (a : Prop) -> a -> a := by intro a; intro h; rfl\n";
    let report = check_document(&parse(src).unwrap());
    assert!(!report.errors.is_empty());
    assert_eq!(report.errors[0].code(), "elab-tactic-failed");
}

#[test]
fn apply_with_mismatched_function_is_a_tactic_error() {
    // apply Or.inl 的目标必须是 Or 形状；目标是真命题时头不匹配。
    let src = concat!(
        "axiom True : Prop\n",
        "axiom Or : Prop -> Prop -> Prop\n",
        "axiom Or.inl : (a : Prop) -> (b : Prop) -> a -> Or a b\n",
        "theorem t : (a : Prop) -> a -> True := by intro a; intro h; apply Or.inl\n",
    );
    let report = check_document(&parse(src).unwrap());
    assert!(!report.errors.is_empty());
    assert_eq!(report.errors[0].code(), "elab-tactic-failed");
}

#[test]
fn by_block_with_sorry_placeholder_is_open_exercise() {
    // `:= by intro a; intro h; sorry`：sorry 把当前目标留空 → 合法 Open 状态。
    let src = "axiom True : Prop\ntheorem t : (a : Prop) -> a -> a := by intro a; intro h; sorry\n";
    let report = check_document(&parse(src).unwrap());
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    let d = report
        .decls
        .iter()
        .find(|d| d.name.as_deref() == Some("t"))
        .unwrap();
    assert_eq!(d.status, DeclStatus::Open);
}

#[test]
fn reserved_declaration_name_produces_a_warning() {
    // `axiom Prop : Sort 1` 能通过内核，但这个名字永不被引用（所有 `Prop`
    // 都解析为内置排序）——产出语法级 warning，不是 error。
    let src = "axiom Prop : Sort 1\n";
    let file = parse(src).unwrap();
    let out = compile_fol(&file);
    assert_eq!(out.errors, vec![], "warning must not be an error");
    assert_eq!(out.warnings.len(), 1, "{:?}", out.warnings);
    let w = &out.warnings[0];
    assert_eq!(w.code(), "reserved-declaration-name");
    assert_eq!(
        &src[w.span.start.offset..w.span.end.offset],
        "Prop",
        "warning span covers the name token"
    );
    assert!(!w.hint().is_empty());

    let report = check_document(&file);
    assert_eq!(report.warnings.len(), 1);
    assert_eq!(report.warnings[0].code(), "reserved-declaration-name");
}

#[test]
fn ordinary_declaration_names_have_no_warning() {
    let src = "axiom True : Prop\naxiom And : Prop -> Prop -> Prop\n";
    let out = compile_fol(&parse(src).unwrap());
    assert_eq!(out.errors, vec![]);
    assert!(out.warnings.is_empty(), "{:?}", out.warnings);
}

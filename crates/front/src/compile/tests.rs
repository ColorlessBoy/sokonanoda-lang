//! compile 模块测试：自原 compile.rs 原样迁移，断言不变。

use super::*;
use crate::parse;

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
    let file = parse("example : Prop -> Prop := ???\n").unwrap();
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
         example : Nat := ???\n\
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
         theorem ex : (a : Prop) -> a -> a := ???\n\
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
         example : Nat := ???\n\
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
        "example : Prop -> Prop := ???\n\
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
        assert!(!hover.text.is_empty(), "empty hover text: {:?}", hover);
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
fn render_expr_round_trips() {
    use crate::proof::parse_expr_text;

    let cases = [
        ("Prop", "Prop"),
        ("f x", "f x"),
        ("fun (x : Prop) => x", "fun (x : Prop) => x"),
        ("(x : Prop) -> x", "(x : Prop) -> x"),
        ("1 + 1", "1 + 1"),
        ("???", "???"),
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
    assert_eq!(out.errors[0].code(), "kernel-rejected");
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
        prelude_mode_from_source("-- sokonanoda:prelude none\ndef x : Prop := ???\n"),
        PreludeMode::Bare
    );
    assert_eq!(
        prelude_mode_from_source("-- 课程\n-- sokonanoda:prelude bare\n"),
        PreludeMode::Bare
    );
    assert_eq!(
        prelude_mode_from_source("def x : Prop := ???\n"),
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
    let file = parse("example : (a : Prop) -> a -> a := fun (a : Prop) => ???\n").unwrap();
    let out = compile_fol(&file);
    assert_eq!(out.errors, vec![]);
    assert!(matches!(&out.events[0], CheckEvent::ExerciseOpen { .. }));
    let report = check_document(&file);
    assert_eq!(report.decls[0].status, DeclStatus::Open);
    assert_eq!(report.decls[0].goal.as_deref(), Some("a -> a"));
}

#[test]
fn partial_hole_with_inferred_binder_reports_goal() {
    let file = parse("def add1 : Nat -> Nat := fun n => ???\n").unwrap();
    let report = check_document(&file);
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    assert_eq!(report.decls[0].status, DeclStatus::Open);
    assert_eq!(report.decls[0].goal.as_deref(), Some("Nat"));
}

#[test]
fn hole_outside_the_lambda_tail_is_rejected() {
    let file = parse("def bad : Nat -> Nat := fun (n : Nat) => n + ???\n").unwrap();
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
    let file =
        parse("example : (a : Prop) -> a -> a := fun (a : Prop) => fun (h : a) => ???\n").unwrap();
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
    let file = parse("def f : Nat -> Nat := fun n => ???\n").unwrap();
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

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
    let src = "def bad : Prop -> Type := fun (x : Prop) => x\n";
    let file = parse(src).unwrap();
    let out = compile_fol(&file);
    assert!(!out.errors.is_empty(), "expected a kernel rejection");
    let e = &out.errors[0];
    assert_eq!(e.code(), "kernel-rejected");
    // 收紧（G-15 / WO-010）：span 必须**逐字**等于出错的那条命令，而不只是
    // `line >= 1`（旧断言等于没断言——任何漂移都能过）。按**字节**切片，
    // 因为 `span.offset` 是字节偏移（`crates/front/src/token.rs`）。
    assert_eq!(
        &src[e.span.start.offset..e.span.end.offset],
        "def bad : Prop -> Type := fun (x : Prop) => x"
    );
}

/// G-15 的真实判据（WO-010）：内核拒绝的 span == **出错命令**的范围，不许溢到
/// 相邻声明上。三条声明、坏的夹在中间；span 一旦漂到 `good`/`after` 就红。
#[test]
fn kernel_rejection_span_is_the_failing_command_range() {
    let src = "def good : Prop -> Prop := fun (p : Prop) => p\n\
               def bad : Prop -> Type := fun (x : Prop) => x\n\
               def after : Prop -> Prop := fun (p : Prop) => p\n";
    let file = parse(src).unwrap();
    let out = compile_fol(&file);
    assert_eq!(
        out.errors.len(),
        1,
        "exactly the middle declaration: {:?}",
        out.errors
    );
    let e = &out.errors[0];
    assert_eq!(e.code(), "kernel-rejected");
    assert_eq!(
        &src[e.span.start.offset..e.span.end.offset],
        "def bad : Prop -> Type := fun (x : Prop) => x"
    );
    assert_eq!(e.span.start.line, 2, "the command starts on line 2");
    assert_eq!(e.span.end.line, 2, "and ends on it: no overflow");
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
    // 名字不叫 `cast`：`cast` 自 B8 扩族起是 prelude 名字（Lean core 的
    // `cast h a = Eq.mp h a`），声明它会**整族让位**——这条测的是"两个宇宙
    // 参数的公理"，用一个文件自己的名字才测得到它（L-03 设计 §4）。
    let file = parse(
        "axiom transport {u, v} :\n\
         forall (α : Sort u), forall (β : Sort v), α -> β\n",
    )
    .expect("parse two universe params");
    let out = compile_fol(&file);
    assert_eq!(out.errors, vec![]);
}

#[test]
fn checks_axiom_with_decl_binders() {
    // WO-008 / G-13：axiom 的 binder 糖与柯里化等价（1/2/3/4 号写法都要 checked）。
    for src in [
        "axiom Foo (α : Type) : Prop\n",
        "axiom Foo (α β : Type) : Prop\n",
        "axiom Foo {α : Type} : Prop\n",
        "axiom Foo {u} (α : Sort u) : Prop\n",
    ] {
        let out = compile_fol(&parse(src).expect("parse axiom with decl binders"));
        assert_eq!(out.errors, vec![], "{src:?}");
    }
}

#[test]
fn axiom_decl_binders_match_arrow_style_outcomes() {
    // 同一条公理的两种拼写各写一份，再用一条闭合定理消费两者：
    // 四条都必须 Checked（比结果，不比文本——硬规则 4）。
    let src = "axiom And : Prop -> Prop -> Prop\n\
         axiom And.intro : (a : Prop) -> (b : Prop) -> a -> b -> And a b\n\
         axiom And.left : (a : Prop) -> (b : Prop) -> And a b -> a\n\
         axiom And.right : (a : Prop) -> (b : Prop) -> And a b -> b\n\
         axiom And.comm (a : Prop) (b : Prop) : And a b -> And b a\n\
         axiom And.comm_arrow : (a : Prop) -> (b : Prop) -> And a b -> And b a\n\
         theorem use_them (a : Prop) (b : Prop) (h : And a b) : And b a := \
         And.comm a b h\n\
         theorem use_them_arrow (a : Prop) (b : Prop) (h : And a b) : And b a := \
         And.comm_arrow a b h\n";
    let report = check_document(&parse(src).expect("parse"));
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    for name in ["And.comm", "And.comm_arrow", "use_them", "use_them_arrow"] {
        let d = report
            .decls
            .iter()
            .find(|d| d.name.as_deref() == Some(name))
            .unwrap_or_else(|| panic!("decl {name}"));
        assert_eq!(d.status, DeclStatus::Checked, "{name} must check");
    }
}

#[test]
fn checks_space_separated_and_split_universe_params() {
    // WO-009 表 3/4/6/9/11：{u v}、{u} {v}、{u} {α : Type}、theorem、axiom 都要 checked。
    for src in [
        "def f {u v} (α : Sort u) (β : Sort v) (a : α) : α := a\n",
        "def f {u} {v} (α : Sort u) (β : Sort v) (a : α) : α := a\n",
        "def f {u} {α : Type} (a : α) : α := a\n",
        "theorem t {u v} (α : Sort u) (β : Sort v) (a : α) : Eq.{u} α a a := Eq.refl.{u} α a\n",
        "axiom A {u} {v} : Sort u\n",
    ] {
        let out = compile_fol(&parse(src).expect("parse universe params"));
        assert_eq!(out.errors, vec![], "{src:?}");
    }
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
            .any(|e| matches!(e, CheckEvent::Reduced { text, .. } if text == "Bool.ff")),
        "`not tt` must reduce to Bool.ff: {:?}",
        out.events
    );
    assert!(
        out.events
            .iter()
            .any(|e| matches!(e, CheckEvent::Reduced { text, .. } if text == "Bool.tt")),
        "`not ff` must reduce to Bool.tt: {:?}",
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
            CheckEvent::Reduced { text, .. } if text == "Nat.succ (Nat.succ (Nat.succ 1))"
        )),
        "expected the measured mixed NatLit form for `add two two`, got {:?}",
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
            CheckEvent::Reduced { text, .. } if text == "MyNat.z"
        )),
        "events: {:?}",
        out.events
    );
    assert!(
        out.events.iter().any(|e| matches!(
            e,
            CheckEvent::Reduced { text, .. } if text == "MyNat.s MyNat.z"
        )),
        "events: {:?}",
        out.events
    );
    assert!(
        out.events.iter().any(|e| matches!(
            e,
            CheckEvent::Reduced { text, .. } if text == "MyNat.s (MyNat.s (MyNat.s MyNat.z))"
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
            CheckEvent::Reduced { text, .. } if text == "1"
        )),
        "explicit Nat + canonical ctors reduce through the Nat fast path: {:?}",
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
            CheckEvent::Reduced { text, .. } if text == "Nat.succ (Nat.succ (Nat.succ 1))"
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
                | ErrorKind::ElabAmbiguousCtorAlias
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
                | ErrorKind::ImportNotFound
                | ErrorKind::ImportCycle
                | ErrorKind::ImportDependencyFailed
                | ErrorKind::ImportNameCollision
                | ErrorKind::ImportPreludeConflict
                | ErrorKind::ManifestInvalid
                | ErrorKind::ImportModuleInvalid
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
        ErrorKind::ElabAmbiguousCtorAlias,
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
        ErrorKind::ImportNotFound,
        ErrorKind::ImportCycle,
        ErrorKind::ImportDependencyFailed,
        ErrorKind::ImportNameCollision,
        ErrorKind::ImportPreludeConflict,
        ErrorKind::ManifestInvalid,
        ErrorKind::ImportModuleInvalid,
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
            CompileStage::Import => {
                // 项目层两种家族：import-*（模块解析）与 manifest-*（清单）。
                assert!(
                    code.starts_with("import-") || code.starts_with("manifest-"),
                    "{kind:?} is a project kind but its code is `{code}`"
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

// ---- 值位 `funapply`（目标「倒过来」消费，前提留洞）----

#[test]
fn funapply_of_an_unknown_name_reports_the_identifier() {
    // 名字打错是最常见的失败：保留既有的 `elab-unknown-identifier` 码。
    let src = "axiom P : Prop\ntheorem t : P := funapply nope\n";
    let report = check_document(&parse(src).expect("parse"));
    assert_eq!(report.errors.len(), 1, "{:?}", report.errors);
    assert_eq!(report.errors[0].code(), "elab-unknown-identifier");
}

#[test]
fn by_block_apply_still_uses_the_tactic_engine() {
    // `by` 块里的 `apply` 走 tactic 引擎（另一条路径），语义不受值位关键字影响。
    let src = "axiom P : Prop\naxiom Q : Prop\n\
               axiom impl : Q -> P\n\
               axiom q : Q\n\
               theorem t : P := by apply impl\n";
    let report = check_document(&parse(src).expect("parse"));
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    let d = report
        .decls
        .iter()
        .find(|d| d.name.as_deref() == Some("t"))
        .expect("decl t");
    // `apply impl` 留下一个未解子目标 `Q` → 练习态（Open），且**不带**值位骨架。
    assert_eq!(d.status, DeclStatus::Open);
    assert_eq!(d.holes.len(), 1);
    assert!(!d.by_steps.is_empty(), "the tactic step is recorded");
    // 同一份证明写全就是 Closed——两条路径都由内核终审。
    let src = "axiom P : Prop\naxiom Q : Prop\n\
               axiom impl : Q -> P\n\
               axiom q : Q\n\
               theorem t : P := by apply impl; exact q\n";
    let report = check_document(&parse(src).expect("parse"));
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    let d = report
        .decls
        .iter()
        .find(|d| d.name.as_deref() == Some("t"))
        .expect("decl t");
    assert_eq!(
        d.status,
        DeclStatus::Checked,
        "closing the sub-goal checks it"
    );
}

#[test]
fn overapplied_spine_through_def_shows_hole_expected_type() {
    // 用户案例（playground L216，0.25.0 精确化）：`(And.right a (Not a) x)
    // sorry` —— And.right 全量应用的结果是 `Not a`（def 展开为 `a ->
    // False`），`sorry` 是它的函数实参 → 洞的期望类型是 `a`，不是整个
    // 声明类型，也不是 `Not a`。walk 借 def 体展开一步得到。
    let src = "axiom False : Prop\n\
               axiom And : Prop -> Prop -> Prop\n\
               axiom And.right : (a : Prop) -> (b : Prop) -> And a b -> b\n\
               def Not : Prop -> Prop := fun (a : Prop) => a -> False\n\
               theorem and_not_absurd : (a : Prop) -> And a (Not a) -> False :=\n\
                 fun (a : Prop) => fun (x : And a (Not a)) => (And.right a (Not a) x) sorry\n";
    let report = check_document(&parse(src).expect("parse"));
    assert!(
        report.errors.is_empty(),
        "no errors expected: {:?}",
        report.errors
    );
    let d = report
        .decls
        .iter()
        .find(|d| d.name.as_deref() == Some("and_not_absurd"))
        .expect("decl");
    assert_eq!(d.status, DeclStatus::Open);
    assert_eq!(d.holes.len(), 1);
    // 剩余目标 = 声明类型剥掉两层 lambda 后的 `False`。
    assert_eq!(d.goal.as_deref(), Some("False"), "goal: {:?}", d.goal);
    // 洞的期望类型 = `Not a` 展开后的箭头定义域 `a`。
    assert_eq!(
        d.sub_goals.len(),
        1,
        "the hole carries a precise expected type"
    );
    assert_eq!(
        d.sub_goals[0].ty.as_deref(),
        Some("a"),
        "expected type of the sorry: {:?}",
        d.sub_goals[0].ty
    );
}

#[test]
fn sorry_in_argument_position_within_open_exercise_is_accepted() {
    // 用户案例 B：`(And.right a (Not a) x) sorry` —— sorry 在参数位置（不在
    // 值位开头也不在 lambda 尾的位置）。open_goal 的 spine
    // 走查因超量应用（通过 `Not` def 间接获得函数类型）无法分解，但值里有
    // 洞 → fallback 生成 generic open exercise（整值 = 一个洞）。
    let src = "axiom P : Prop\n\
               axiom False : Prop\n\
               axiom Not : Prop -> Prop\n\
               axiom And : Prop -> Prop -> Prop\n\
               axiom And.left : (a : Prop) -> (b : Prop) -> And a b -> a\n\
               axiom And.right : (a : Prop) -> (b : Prop) -> And a b -> b\n\
               theorem t : (a : Prop) -> And a (Not a) -> False :=\n\
                 fun (a : Prop) => fun (x : And a (Not a)) => (And.right a (Not a) x) sorry\n";
    let report = check_document(&parse(src).expect("parse"));
    assert!(
        report.errors.is_empty(),
        "sorry in argument position within an open exercise must not error: {:?}",
        report.errors
    );
    let d = report
        .decls
        .iter()
        .find(|d| d.name.as_deref() == Some("t"))
        .expect("decl t");
    assert_eq!(d.status, DeclStatus::Open, "the exercise is in progress");
    assert_eq!(d.holes.len(), 1, "one hole for the whole value");
}

#[test]
fn incomplete_application_without_sorry_shows_remaining_goals_on_hover() {
    // 用户案例 A：`And.intro b a`（不完整、无 sorry）→ 内核拒绝，但 hover
    // 显示推断出的剩余目标（half_expression_goals_hover 在 LSP 层处理；
    // 这里验证 front 侧的 Failed 状态和错误码正确，LSP 层有对应测试）。
    let src = "axiom And : Prop -> Prop -> Prop\n\
               axiom And.intro : (a : Prop) -> (b : Prop) -> a -> b -> And a b\n\
               theorem t : (a : Prop) -> (b : Prop) -> And a b -> And b a :=\n\
                 fun (a : Prop) => fun (b : Prop) => fun (x : And a b) => And.intro b a\n";
    let report = check_document(&parse(src).expect("parse"));
    assert_eq!(report.errors.len(), 1, "{:?}", report.errors);
    assert_eq!(report.errors[0].code(), "kernel-rejected");
    let d = report
        .decls
        .iter()
        .find(|d| d.name.as_deref() == Some("t"))
        .expect("decl t");
    assert_eq!(d.status, DeclStatus::Failed);
}

#[test]
fn by_in_a_lambda_tail_enters_tactic_mode() {
    // 用户诉求：`by` 也能在 lambda 里直接进 tactic 模式——拆完 binder 后
    // 用 tactic 继续是主流程。降低走 `split_by_value`（沿链收集 binder），
    // 引擎拿到的初始上下文就是这些 binder。
    let src = "axiom P : Prop\naxiom Q : Prop\naxiom proofP : P\n\
               theorem t : Q -> P := fun (x : Q) => by exact proofP\n";
    let report = check_document(&parse(src).expect("parse"));
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    let d = report
        .decls
        .iter()
        .find(|d| d.name.as_deref() == Some("t"))
        .expect("decl t");
    assert_eq!(d.status, DeclStatus::Checked, "the kernel judges the fill");
    assert!(!d.by_steps.is_empty(), "the tactic step is recorded");
}

#[test]
fn by_in_a_lambda_tail_with_intro_and_exact() {
    // tactic 序列也能在 lambda 尾跑：intro 消一层、exact 收尾。
    let src = "axiom P : Prop\naxiom Q : Prop\naxiom proofP : P\n\
               theorem t : Q -> (Q -> P) := fun (x : Q) => by intro h; exact proofP\n";
    let report = check_document(&parse(src).expect("parse"));
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    let d = report
        .decls
        .iter()
        .find(|d| d.name.as_deref() == Some("t"))
        .expect("decl t");
    assert_eq!(d.status, DeclStatus::Checked);
    assert_eq!(d.by_steps.len(), 2, "two tactic steps: {d:?}");
}

#[test]
fn by_sorry_in_a_lambda_tail_is_an_open_exercise() {
    // 与值位 `by sorry` 同语义：占位 → Open，且洞指向 by 块。
    let src = "axiom P : Prop\naxiom Q : Prop\n\
               theorem t : Q -> P := fun (x : Q) => by sorry\n";
    let report = check_document(&parse(src).expect("parse"));
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    let d = report
        .decls
        .iter()
        .find(|d| d.name.as_deref() == Some("t"))
        .expect("decl t");
    assert_eq!(d.status, DeclStatus::Open);
    assert_eq!(d.holes.len(), 1);
    assert_eq!(d.goal.as_deref(), Some("P"));
}

// ---- 声明级 binder（Lean 风格：theorem f (a : A) : B := v）----

const AND_PRELUDE: &str = "axiom And : Prop -> Prop -> Prop\n\
     axiom And.intro : (a : Prop) -> (b : Prop) -> a -> b -> And a b\n\
     axiom And.left : (a : Prop) -> (b : Prop) -> And a b -> a\n\
     axiom And.right : (a : Prop) -> (b : Prop) -> And a b -> b\n";

#[test]
fn decl_binders_open_exercise_reports_codomain_and_context() {
    let src = format!(
        "{AND_PRELUDE}\
         theorem and_swap2 (a : Prop) (b : Prop) (h : And a b) : And b a := sorry\n"
    );
    let report = check_document(&parse(&src).expect("parse"));
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    let d = report
        .decls
        .iter()
        .find(|d| d.name.as_deref() == Some("and_swap2"))
        .expect("decl and_swap2");
    assert_eq!(d.status, DeclStatus::Open);
    assert_eq!(d.goal.as_deref(), Some("And b a"));
    let binders: Vec<(&str, &str)> = d
        .binders
        .iter()
        .map(|b| (b.name.as_str(), b.ty.as_str()))
        .collect();
    assert_eq!(
        binders,
        vec![("a", "Prop"), ("b", "Prop"), ("h", "And a b")]
    );
    assert_eq!(d.holes.len(), 1);
    assert_eq!(
        &src[d.holes[0].start.offset..d.holes[0].end.offset],
        "sorry"
    );
}

#[test]
fn decl_binders_closed_body_needs_no_lambdas() {
    let src = format!(
        "{AND_PRELUDE}\
         theorem and_swap3 (a : Prop) (b : Prop) (h : And a b) : And b a := \
         And.intro b a (And.right a b h) (And.left a b h)\n"
    );
    let report = check_document(&parse(&src).expect("parse"));
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    let d = report
        .decls
        .iter()
        .find(|d| d.name.as_deref() == Some("and_swap3"))
        .expect("decl and_swap3");
    assert_eq!(d.status, DeclStatus::Checked);
}

#[test]
fn decl_binders_feed_the_by_engine_context() {
    let src = format!(
        "{AND_PRELUDE}\
         theorem by_ctx (a : Prop) (h : a) : a := by assumption\n"
    );
    let report = check_document(&parse(&src).expect("parse"));
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    let d = report
        .decls
        .iter()
        .find(|d| d.name.as_deref() == Some("by_ctx"))
        .expect("decl by_ctx");
    assert_eq!(d.status, DeclStatus::Checked);
}

#[test]
fn decl_binders_match_arrow_style_outcomes() {
    let src = format!(
        "{AND_PRELUDE}\
         theorem arrow_style : (a : Prop) -> (b : Prop) -> And a b -> And b a := \
         fun (a : Prop) => fun (b : Prop) => fun (h : And a b) => \
         And.intro b a (And.right a b h) (And.left a b h)\n\
         theorem binder_style (a : Prop) (b : Prop) (h : And a b) : And b a := \
         And.intro b a (And.right a b h) (And.left a b h)\n"
    );
    let report = check_document(&parse(&src).expect("parse"));
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    for name in ["arrow_style", "binder_style"] {
        let d = report
            .decls
            .iter()
            .find(|d| d.name.as_deref() == Some(name))
            .expect("decl");
        assert_eq!(d.status, DeclStatus::Checked, "{name} must check");
    }
}

#[test]
fn decl_binders_disambiguate_universe_params_from_implicit_binders() {
    let report =
        check_document(&parse("def id_univ {u} (A : Sort u) (x : A) : A := x\n").expect("parse"));
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    assert_eq!(report.decls[0].status, DeclStatus::Checked);
    let report = check_document(
        &parse("theorem implicit_decl {a : Prop} : a -> a := fun (h : a) => h\n").expect("parse"),
    );
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    assert_eq!(report.decls[0].status, DeclStatus::Checked);
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
        ("f x y", "f x y"),
        ("f (g x)", "f (g x)"),
        ("And a b", "And a b"),
        ("fun (x : Prop) => x", "fun (x : Prop) => x"),
        ("(x : Prop) -> x", "(x : Prop) -> x"),
        ("1 + 1", "1 + 1"),
        ("sorry", "sorry"),
        // IA-1：`@` 成了真语义（关闭隐式实参插入）。**没有实参**时 `@f` 与
        // `f` 的项相同 ⇒ 渲染丢掉那个孤立的 `@`（判卷通道往返要求"渲染 →
        // 回读"不改含义）；**带实参**时 `@` 必须打回来（下面两条）。
        ("@Eq.{u, v}", "Eq.{u, v}"),
        ("@f a b", "@f a b"),
        ("@f.{u} a b", "@f.{u} a b"),
        // Forall 作箭头 domain：必须补括号（judge_infer 往返健壮性）。
        (
            "((k : Nat) -> P k -> P (succ k)) -> Nat -> Prop",
            "((k : Nat) -> P k -> P (succ k)) -> Nat -> Prop",
        ),
    ];
    for (source, expected) in cases {
        let expr = parse_expr_text(source).unwrap_or_else(|e| panic!("parse {source}: {e:?}"));
        assert_eq!(render_expr(&expr), expected, "source: {source}");
        // 渲染结果必须能再解析回同一渲染（往返稳定）。
        let back = parse_expr_text(&render_expr(&expr))
            .unwrap_or_else(|e| panic!("reparse {source}: {e:?}"));
        assert_eq!(render_expr(&back), expected, "round-trip: {source}");
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
        ErrorKind::ElabAmbiguousCtorAlias,
        ErrorKind::ElabTacticFailed,
        ErrorKind::ElabApplyNeedsATerm,
        ErrorKind::ElabApplyNotApplicable,
        ErrorKind::ElabMatchBadArm,
        ErrorKind::ElabMatchNotInductive,
        ErrorKind::ElabMatchNoExpectedType,
        ErrorKind::ElabMatchRecursiveUnsupported,
        ErrorKind::ElabMatchNonExhaustive,
        ErrorKind::ElabMatchParameterizedUnsupported,
        ErrorKind::ElabLetTypeQueryFailed,
        ErrorKind::ElabNotationUnknownTarget,
        ErrorKind::ElabNotationArgumentUnsolved,
        ErrorKind::ElabImplicitArgumentUnsolved,
        ErrorKind::ElabNotationAmbiguous,
        ErrorKind::ElabNotationNoCandidate,
        ErrorKind::ElabBinderNotationUnsolved,
        ErrorKind::ElabSetLiteralUnknownTarget,
        ErrorKind::KernelExpectedSort,
        ErrorKind::KernelExpectedPi,
        ErrorKind::KernelTheoremNotProp,
        ErrorKind::KernelPropNotCumulative,
        ErrorKind::KernelNonPositive,
        ErrorKind::KernelCtorResultMismatch,
        ErrorKind::KernelCtorArgInvalidApp,
        ErrorKind::KernelCtorArgNotType,
        ErrorKind::KernelCtorArgTooLarge,
        ErrorKind::KernelRecRuleMismatch,
        ErrorKind::KernelRejected,
        ErrorKind::KernelInternal,
        ErrorKind::ImportNotFound,
        ErrorKind::ImportCycle,
        ErrorKind::ImportDependencyFailed,
        ErrorKind::ImportNameCollision,
        ErrorKind::ImportPreludeConflict,
        ErrorKind::ManifestInvalid,
        ErrorKind::ImportModuleInvalid,
    ];
    // Genuine exhaustiveness guard: a `match` with no wildcard arm fails to
    // compile when a new `ErrorKind` variant is added, forcing this list (and
    // the `all` array above) to be updated. A `matches!` would silently fall
    // through to `_ => false` and never catch the omission.
    let _all_variants_listed = |kind: ErrorKind| match kind {
        ErrorKind::ElabUnknownIdentifier => {}
        ErrorKind::ElabUnknownConstant => {}
        ErrorKind::ElabUnknownUniverseLevel => {}
        ErrorKind::ElabUniverseArity => {}
        ErrorKind::ElabUntypedBinder => {}
        ErrorKind::ElabHoleMisplaced => {}
        ErrorKind::ElabDuplicateDeclaration => {}
        ErrorKind::ElabTooManyBinders => {}
        ErrorKind::ElabNatLiteralDisabled => {}
        ErrorKind::ElabInvalidNatLiteral => {}
        ErrorKind::ElabTooManyCtorFields => {}
        ErrorKind::ElabUnknownCtorForIota => {}
        ErrorKind::ElabAmbiguousCtorAlias => {}
        ErrorKind::ElabTacticFailed => {}
        ErrorKind::ElabApplyNeedsATerm => {}
        ErrorKind::ElabApplyNotApplicable => {}
        ErrorKind::ElabMatchBadArm => {}
        ErrorKind::ElabMatchNotInductive => {}
        ErrorKind::ElabMatchNoExpectedType => {}
        ErrorKind::ElabMatchRecursiveUnsupported => {}
        ErrorKind::ElabMatchNonExhaustive => {}
        ErrorKind::ElabMatchParameterizedUnsupported => {}
        ErrorKind::ElabLetTypeQueryFailed => {}
        ErrorKind::ElabNotationUnknownTarget => {}
        ErrorKind::ElabNotationArgumentUnsolved => {}
        ErrorKind::ElabImplicitArgumentUnsolved => {}
        ErrorKind::ElabNotationAmbiguous => {}
        ErrorKind::ElabNotationNoCandidate => {}
        ErrorKind::ElabBinderNotationUnsolved => {}
        ErrorKind::ElabSetLiteralUnknownTarget => {}
        ErrorKind::ElabAnonCtorNoExpectedType => {}
        ErrorKind::KernelExpectedSort => {}
        ErrorKind::KernelExpectedPi => {}
        ErrorKind::KernelTheoremNotProp => {}
        ErrorKind::KernelPropNotCumulative => {}
        ErrorKind::KernelNonPositive => {}
        ErrorKind::KernelCtorResultMismatch => {}
        ErrorKind::KernelCtorArgInvalidApp => {}
        ErrorKind::KernelCtorArgNotType => {}
        ErrorKind::KernelCtorArgTooLarge => {}
        ErrorKind::KernelRecRuleMismatch => {}
        ErrorKind::KernelRejected => {}
        ErrorKind::KernelInternal => {}
        ErrorKind::ImportNotFound => {}
        ErrorKind::ImportCycle => {}
        ErrorKind::ImportDependencyFailed => {}
        ErrorKind::ImportNameCollision => {}
        ErrorKind::ImportPreludeConflict => {}
        ErrorKind::ManifestInvalid => {}
        ErrorKind::ImportModuleInvalid => {}
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
            CheckEvent::Reduced { text, .. } if text == "Nat.succ (Nat.succ (Nat.succ 1))"
        )),
        "expected the measured mixed NatLit form for `add two two`, got {:?}",
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

// ---------------------------------------------------------------------------
// L1 prelude（docs/design/prelude-l1-proposal.md P1）：逻辑与等式骨架
// ---------------------------------------------------------------------------

/// 一段只用 L1 名字的完整文件：B1–B7 各用一次（`And.intro`/`Or.elim`/
/// `Iff.mp`/`absurd`/`Eq.symm`），并且不自己声明任何 L1 名字（否则整族让位）。
const L1_USER_SRC: &str = "\
def l1_and (a b : Prop) (ha : a) (hb : b) : And b a := And.intro b a hb ha
def l1_or (a b c : Prop) (f : a -> c) (g : b -> c) (h : Or a b) : c := Or.elim a b c f g h
def l1_iff (a b : Prop) (h : Iff a b) : b -> a := Iff.mpr a b h
def l1_absurd (a b : Prop) (ha : a) (hna : Not a) : b := absurd a b ha hna
def l1_true : True := True.intro
def l1_false (h : False) : False := h
def l1_false_elim (c : Prop) (h : False) : c := False.elim c h
def l1_not_intro (a : Prop) (f : a -> False) : Not a := Not.intro a f
def l1_not_elim (a c : Prop) (h : Not a) (ha : a) : c := Not.elim a c h ha
def l1_iff_refl (a : Prop) : Iff a a := Iff.refl a
def l1_iff_symm (a b : Prop) (h : Iff a b) : Iff b a := Iff.symm a b h
def l1_iff_trans (a b c : Prop) (h1 : Iff a b) (h2 : Iff b c) : Iff a c := Iff.trans a b c h1 h2
def l1_and_elim (a b c : Prop) (f : a -> b -> c) (h : And a b) : c := And.elim a b c f h
def l1_and_left (a b : Prop) (h : And a b) : a := And.left a b h
def l1_and_right (a b : Prop) (h : And a b) : b := And.right a b h
def l1_or_inl (a b : Prop) (ha : a) : Or a b := Or.inl a b ha
def l1_or_inr (a b : Prop) (hb : b) : Or a b := Or.inr a b hb
def l1_eq_symm (a b : Nat) (h : Eq.{1} Nat a b) : Eq.{1} Nat b a := Eq.symm.{1} Nat a b h
def l1_eq_trans (a b c : Nat) (h1 : Eq.{1} Nat a b) (h2 : Eq.{1} Nat b c) : Eq.{1} Nat a c :=
  Eq.trans.{1} Nat a b c h1 h2
def l1_congr_arg (f : Nat -> Nat) (a b : Nat) (h : Eq.{1} Nat a b) :
    Eq.{1} Nat (f a) (f b) := congrArg.{1} f h
";

fn l1_names_missing(errors: &[CompileError]) -> Vec<String> {
    errors
        .iter()
        .filter(|e| {
            matches!(
                e.kind,
                ErrorKind::ElabUnknownIdentifier | ErrorKind::ElabUnknownConstant
            )
        })
        .map(|e| e.message.clone())
        .collect()
}

/// P1 的核心：Full 模式下 L1 全族可用（0 errors），且每条声明都真的过内核。
#[test]
fn l1_prelude_is_available_in_full_mode() {
    let file = parse(L1_USER_SRC).unwrap();
    let out = compile_fol(&file);
    assert_eq!(
        out.errors,
        vec![],
        "L1 prelude must install in Full mode; unknown-identifier leftovers: {:?}",
        l1_names_missing(&out.errors)
    );
    let checked = out
        .events
        .iter()
        .filter(|e| matches!(e, CheckEvent::DeclarationChecked { .. }))
        .count();
    assert_eq!(checked, 20, "every L1 probe declaration must be checked");
}

/// `Or`/`And` 是**真归纳块**：`match` 的两种模式拼写（点号名与裸名）都必须过。
/// 这条钉住 `InductiveTable` 的注册（`Or.elim` 本身走的是派生 `Or.rec`）。
#[test]
fn l1_or_is_a_real_inductive_for_match() {
    for patterns in [
        (
            "| Or.inl ha => Or.inr b a ha",
            "| Or.inr hb => Or.inl b a hb",
        ),
        ("| inl ha => Or.inr b a ha", "| inr hb => Or.inl b a hb"),
    ] {
        let src = format!(
            "def l1_or_comm (a b : Prop) (h : Or a b) : Or b a :=\n\
             match h with\n{}\n{}\n",
            patterns.0, patterns.1
        );
        let file = parse(&src).unwrap();
        let out = compile_fol(&file);
        assert_eq!(
            out.errors,
            vec![],
            "patterns {:?}: {:?}",
            patterns,
            out.errors
        );
    }
}

/// 让位是**族粒度 + 依赖闭包**（设计 §2.2 的依赖表：B6→B3、B5→B2、B7→Eq）：
/// 文件声明 `And`（B3）⇒ 依赖它的 `Iff.*`（B6）一起不装，而独立的
/// `Or.elim`（B4）/`Eq.symm`（B7）仍在。
///
/// 注意方向：依赖边是「B6 用 `And.left`」而不是反过来，所以声明 `Iff` 只让位
/// B6、**不**让位 B3（提案 §3.2 的措辞把这条写反了；§2.2 的表是规范）。
#[test]
fn l1_family_yield_is_dependency_closed() {
    // 文件只声明 B3 的族头 `And`（不透明定义即可：重点是让位，不是展开形状）。
    let src = "\
inductive And (a b : Prop) : Prop\n\
ctor And.intro (ha : a) (hb : b) : And a b\n\
end\n\
def probe_or (a b c : Prop) (f : a -> c) (g : b -> c) (h : Or a b) : c := Or.elim a b c f g h\n\
def probe_eq (a b : Nat) (h : Eq.{1} Nat a b) : Eq.{1} Nat b a := Eq.symm.{1} Nat a b h\n";
    let out = compile_fol(&parse(src).unwrap());
    assert_eq!(
        out.errors,
        vec![],
        "Or/Eq families must survive a B3 yield: {:?}",
        out.errors
    );
    // 反向：`Iff.mpr` 随 B3 的依赖闭包让位 ⇒ 用它必须报未知标识符。
    let src = format!("{src}def probe_iff (a b : Prop) (h : Iff a b) : b -> a := Iff.mpr a b h\n");
    let out = compile_fol(&parse(&src).unwrap());
    assert!(
        l1_names_missing(&out.errors)
            .iter()
            .any(|m| m.contains("Iff")),
        "B6 must be gone once B3 yielded: {:?}",
        out.errors
    );
    // 对照（同一条依赖边的另一头）：只声明 `Iff` ⇒ 只让位 B6，B3 照常装着。
    let src = "\
def Iff (A B : Prop) : Prop := A -> B\n\
def probe_and (a b : Prop) (ha : a) (hb : b) : And a b := And.intro a b ha hb\n";
    let out = compile_fol(&parse(src).unwrap());
    assert_eq!(
        out.errors,
        vec![],
        "a B6 yield must not take B3 down with it: {:?}",
        out.errors
    );
}

/// 只声明族里的**一个**名字，整族让位（不是单名）。
#[test]
fn l1_yield_needs_the_whole_family() {
    // `And.left` 只声明名字、不提 `And`（否则文件自己就先撞上让位后的空环境）。
    let file = parse(
        "def And.left (x : Prop) : Prop := x\n\
         def probe (a b : Prop) (ha : a) (hb : b) : And a b := And.intro a b ha hb\n",
    )
    .unwrap();
    let out = compile_fol(&file);
    assert!(
        l1_names_missing(&out.errors)
            .iter()
            .any(|m| m.contains("And")),
        "declaring And.left alone must yield the whole B3 family: {:?}",
        out.errors
    );
}

/// 让位触发集合 = 整个闭包的顶层名字并集，且必须**含构造子与递归子**
/// （设计 §2.3-1：`taken` 从 `user_top_level_names` 换成
/// `top_level_def_spans_over` 的键集）。
#[test]
fn l1_taken_includes_ctors_and_recursors() {
    let file = parse(
        "inductive Pair : Prop\n\
         ctor Or.inl : Pair\n\
         end\n\
         def probe (a b : Prop) (ha : a) : Or a b := Or.inl a b ha\n",
    )
    .unwrap();
    let out = compile_fol(&file);
    assert!(
        l1_names_missing(&out.errors)
            .iter()
            .any(|m| m.contains("Or")),
        "a file-declared `ctor Or.inl` must take the B4 family over: {:?}",
        out.errors
    );
}

/// 白名单 ↔ 实际安装防漂移：`PRELUDE_NAMES` 必须逐条真的在环境里。
#[test]
fn prelude_names_match_installs() {
    // 设计 §3.3 的守卫：数字变了必须是有意为之（review 时一眼看见）。
    // 12（Nat/Bool/Eq）+ 30（L1：28 条声明 + 派生的 And.rec/Or.rec）
    // + 5（B8：Eq.rec/Eq.ndrec/Eq.mp/Eq.mpr/cast，L-03）
    // + 2（B9：`Ne`/`Ne.intro`，L2.3 的 `≠` 目标）= 49。
    assert_eq!(
        super::PRELUDE_NAMES.len(),
        49,
        "PRELUDE_NAMES drifted: {:?}",
        super::PRELUDE_NAMES
    );
    for name in super::PRELUDE_NAMES {
        // `#check` 对已安装的名字给 `expr.typed`；未知名字给 diagnostic。
        let out = compile_fol(&parse(&format!("#check {name}\n")).unwrap());
        assert!(
            out.errors.is_empty(),
            "PRELUDE_NAMES lists `{name}` but the prelude does not install it: {:?}",
            out.errors
        );
    }
}

/// `Eq.symm` 不只**可导出**（既有测试），它现在**已安装**。
#[test]
fn eq_symm_is_installed() {
    let out = compile_fol(
        &parse("def probe (a b : Nat) (h : Eq.{1} Nat a b) : Eq.{1} Nat b a := Eq.symm.{1} Nat a b h\n")
            .unwrap(),
    );
    assert_eq!(out.errors, vec![], "{:?}", out.errors);
}

/// **L-03 / B8**：`Eq.rec` 装上后，**Type 层重写**在 Full 模式下真的可用。
/// 三个用途各钉一次，都是 `Eq.subst`（motive 只能落 Prop）做不到的：
/// ① 沿 `Eq.{1} Nat m n` 把 `Vec A m` 搬到 `Vec A n`（motive 落 `Sort 1`）；
/// ② `Eq.mp`/`Eq.mpr`/`cast` 搬两个类型（**宇宙多态**：`{u} (α β : Sort u)`）；
/// ③ `Eq.ndrec` 走 Lean core 的非依赖消去子。
#[test]
fn eq_rec_transports_at_type_level() {
    let src = "\
inductive Vec (A : Type) : Nat -> Type\n\
ctor Vec.nil : Vec A Nat.zero\n\
ctor Vec.cons (n : Nat) (a : A) (v : Vec A n) : Vec A (Nat.succ n)\n\
end\n\
def Vec.cast (A : Type) (m n : Nat) (h : Eq.{1} Nat m n) (v : Vec A m) : Vec A n :=\n\
  @Eq.rec.{1, 1} Nat m (fun (k : Nat) => Vec A k) v n h\n\
def id_mp (A : Type) : A -> A := Eq.mp.{1} A A (Eq.refl.{2} Type A)\n\
def id_mpr (A : Type) : A -> A := Eq.mpr.{1} A A (Eq.refl.{2} Type A)\n\
def id_cast (A : Type) : A -> A := cast.{1} A A (Eq.refl.{2} Type A)\n\
def nd_symm (a b : Nat) (h : Eq.{1} Nat a b) : Eq.{1} Nat b a :=\n\
  Eq.ndrec.{1, 0} Nat a (fun (x : Nat) => Eq.{1} Nat x a) (Eq.refl.{1} Nat a) b h\n";
    let out = compile_fol(&parse(src).expect("parse B8 use"));
    assert_eq!(
        out.errors,
        vec![],
        "Type-level rewriting must work through the prelude's Eq.rec/Eq.ndrec/Eq.mp/Eq.mpr/cast: {:?}",
        out.errors
    );
}

/// **L-03 / B8 + 层级算术**：`Eq.mp`/`Eq.mpr`/`cast` 是**宇宙多态**的——
/// `{u} (α β : Sort u) (h : @Eq.{u+1} (Sort u) α β)`（Lean core 的签名）。
/// 这条同时钉住层级算术的两个位置：`Sort (u+1)`（类型位）与 `.{u+1}`（宇宙实参）。
#[test]
fn eq_mp_is_universe_polymorphic() {
    let src = "\
def poly_mp {u} (α β : Sort u) (h : @Eq.{u+1} (Sort u) α β) (a : α) : β :=\n\
  Eq.mp.{u} α β h a\n\
def poly_mpr {u} (α β : Sort u) (h : @Eq.{u+1} (Sort u) α β) (b : β) : α :=\n\
  Eq.mpr.{u} α β h b\n\
def poly_cast {u} (α β : Sort u) (h : @Eq.{u+1} (Sort u) α β) (a : α) : β :=\n\
  cast.{u} α β h a\n\
def step {u} (α : Sort u) (a : α) : Sort (u+1) := Sort u\n";
    let out = compile_fol(&parse(src).expect("parse polymorphic Eq.mp"));
    assert_eq!(
        out.errors,
        vec![],
        "Eq.mp/Eq.mpr/cast must be universe polymorphic (design §2 as-built): {:?}",
        out.errors
    );
    // 边界（as-built）：本语言不给隐式实参、也不给宇宙推断 ⇒ **裸写** `Eq.mp`
    // 仍按 u=0 实例化（与 `Eq.symm`/`Eq.rec` 同一条既有规则）。Type 0 的调用
    // 形状因此从 0.60.0 的 `Eq.mp α β h` 变成 `Eq.mp.{1} α β h`（签名变更）。
    let out = compile_fol(
        &parse("def id_mp (A : Type) : A -> A := Eq.mp A A (Eq.refl.{2} Type A)\n").unwrap(),
    );
    assert!(
        out.errors.iter().any(|e| e.message.contains("Sort(0)")),
        "bare Eq.mp must instantiate u := 0 (no universe inference): {:?}",
        out.errors
    );
}

/// **L-03 / B8 的让位**：文件自己声明 `Eq.rec` ⇒ B8 整族（含 `Eq.ndrec`/`Eq.mp`/
/// `Eq.mpr`/`cast`）让位，而 `Eq` prelude 与 B7（`Eq.symm`）照常装着——B8 的依赖
/// 是 EQ（`Eq` 公理族），不是 B7。这条同时钉住"族粒度"与"依赖方向"。
#[test]
fn eq_rec_family_yields_when_the_file_declares_it() {
    let src = "\
axiom Eq.rec {u, v} : {α : Sort u} -> (a : α) -> (motive : (anon : α) -> Sort v) -> (ha : motive a) -> (b : α) -> (h : @Eq.{u} α a b) -> motive b\n\
def probe_symm (a b : Nat) (h : Eq.{1} Nat a b) : Eq.{1} Nat b a := Eq.symm.{1} Nat a b h\n";
    let out = compile_fol(&parse(src).expect("parse B8 yield"));
    assert_eq!(
        out.errors,
        vec![],
        "B7 and the Eq prelude must survive a B8 yield: {:?}",
        out.errors
    );
    // 反向：B8 让位后 `Eq.mp` 必须报未知标识符（整族让位，不是单名）。
    let src = format!("{src}#check Eq.mp\n");
    let out = compile_fol(&parse(&src).expect("parse B8 yield probe"));
    assert!(
        l1_names_missing(&out.errors)
            .iter()
            .any(|m| m.contains("Eq.mp")),
        "declaring Eq.rec alone must yield the whole B8 family: {:?}",
        out.errors
    );
    // `cast`/`Eq.ndrec` 也在 B8 里：声明 `cast` 同样整族让位（设计 §2 的族规则）。
    let src = "\
axiom cast {u} : {α : Sort u} -> {β : Sort u} -> @Eq.{u+1} (Sort u) α β -> α -> β\n\
def probe_symm (a b : Nat) (h : Eq.{1} Nat a b) : Eq.{1} Nat b a := Eq.symm.{1} Nat a b h\n\
#check Eq.mp\n";
    let out = compile_fol(&parse(src).expect("parse B8 cast yield"));
    assert!(
        l1_names_missing(&out.errors)
            .iter()
            .any(|m| m.contains("Eq.mp")),
        "declaring `cast` alone must yield the whole B8 family: {:?}",
        out.errors
    );
}

/// 建议材料层（`GoalTemplates::new_for`）也吃 L1：开放练习里对 prelude 的
/// `And.intro` 做 spine 应用时，子洞期望类型必须由模板算出来（设计 §4.1 的
/// `goals.rs` 行）。**边界（as-built）**：归纳块的 `CtorTemplate.result_arg_names`
/// 一直是空的（既有行为，见 `ctor_spine_accepts_both_spellings` 的注释），
/// 所以这里断言的是 `sub_goals` 的期望类型，不是 `refine_template`。
#[test]
fn l1_ctor_templates_feed_sub_goal_types() {
    let src = "def probe (a b : Prop) : And a b := And.intro sorry sorry\n";
    let report = check_document(&parse(src).expect("parse"));
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    let open = report
        .decls
        .iter()
        .find(|d| d.name.as_deref() == Some("probe"))
        .expect("open");
    let tys: Vec<Option<&str>> = open.sub_goals.iter().map(|s| s.ty.as_deref()).collect();
    assert_eq!(
        tys,
        vec![Some("a"), Some("b")],
        "the prelude's And.intro must feed the ctor spine template"
    );
}

/// Bare 模式（`--bare` 与 `-- sokonanoda:prelude none`）没有 L1。
#[test]
fn bare_mode_has_no_l1() {
    let options = CompileOptions {
        prelude: PreludeMode::Bare,
    };
    let file = parse("def probe (a b : Prop) (ha : a) (hb : b) : And a b := And.intro a b ha hb\n")
        .unwrap();
    let out = compile_fol_with(&file, &options);
    assert!(!out.errors.is_empty(), "Bare mode must not install L1");
}

/// 项目模式：**依赖模块**声明 B3（`inductive And`）⇒ 入口也没有 prelude 的
/// `And.elim`（让位是闭包级的，设计 §2.2）。注意入口能看见依赖声明的 `And`
/// 本身——被隐藏的是 prelude 那一族的**其余名字**（族粒度，不是单名）。
#[test]
fn l1_yield_is_closure_wide() {
    use crate::project::compile_project;
    use std::path::PathBuf;

    let dir = std::env::temp_dir().join(format!("soko-front-l1-closure-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp dir");
    std::fs::write(
        dir.join("Dep.sokonanoda"),
        "inductive And (a b : Prop) : Prop\nctor And.intro (ha : a) (hb : b) : And a b\nend\n",
    )
    .expect("write Dep");
    std::fs::write(
        dir.join("Main.sokonanoda"),
        "import Dep\n\ndef probe (a b c : Prop) (f : a -> b -> c) (h : And a b) : c :=\n  And.elim a b c f h\n",
    )
    .expect("write Main");
    let entry: PathBuf = dir.join("Main.sokonanoda");
    let report = compile_project(&entry, None, &CompileOptions::default(), None);
    let entry_errors: Vec<&'static str> = report
        .entry_module()
        .map(|m| m.report.errors.iter().map(|e| e.code()).collect())
        .unwrap_or_default();
    assert!(
        entry_errors.contains(&"elab-unknown-identifier"),
        "the closure-level yield must hide prelude `And.elim` from the entry: {entry_errors:?}"
    );
    // 对照：同一入口在单文件模式（没有依赖模块占用 B3）里照常通过。
    let solo = crate::parse(
        "def probe (a b c : Prop) (f : a -> b -> c) (h : And a b) : c := And.elim a b c f h\n",
    )
    .unwrap();
    let out = compile_fol(&solo);
    assert_eq!(out.errors, vec![], "{:?}", out.errors);
    let _ = std::fs::remove_dir_all(&dir);
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
fn untyped_binder_is_inferred_from_the_application_argument() {
    // I6（非依赖）：应用位置没有期望类型时，从实参类型推断 lambda binder。
    let src = "def k : Nat := (fun x => x) 1\n#reduce (fun x => x) 2\n";
    let out = compile_fol(&parse(src).expect("parse"));
    assert_eq!(out.errors, vec![], "errors: {:?}", out.errors);
    assert!(out
        .events
        .iter()
        .any(|e| matches!(e, CheckEvent::DeclarationChecked { name } if name == "k")));
    assert!(
        out.events
            .iter()
            .any(|e| matches!(e, CheckEvent::Reduced { text, .. } if text == "2")),
        "{:?}",
        out.events
    );
}

#[test]
fn curried_untyped_binders_are_inferred_from_the_arguments() {
    let src = "def k : Nat := (fun x y => y) 1 2\n#reduce (fun x y => x) 3 4\n";
    let out = compile_fol(&parse(src).expect("parse"));
    assert_eq!(out.errors, vec![], "errors: {:?}", out.errors);
    assert!(
        out.events
            .iter()
            .any(|e| matches!(e, CheckEvent::Reduced { text, .. } if text == "3")),
        "{:?}",
        out.events
    );
}

#[test]
fn untyped_binder_still_errors_with_too_few_arguments() {
    // 只有一个实参、两个未注解 binder：第二个无从推断 → 仍报 untyped-binder。
    let out = compile_fol(&parse("def k : Nat := (fun x y => x) 1\n").expect("parse"));
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
        // G-21（2026-09-21）：**项落在类型位**——内核把出错那一侧渲染成本地变量
        // （`$k`），另一侧是 `Sort(n)`。点名调用漏了前导类型参数
        // （`Set.mem a A` 少了 `α`）就是这个形状；归类成 expected-sort，
        // 由那条 hint 负责把「漏了哪个参数」说给学习者。
        (
            "rejected: def_eq failed: def_eq mismatch expected: Sort(1) | actual: $2",
            ErrorKind::KernelExpectedSort,
        ),
        // 对照：两边都是具体 sort 时仍走 L-06 的 Prop-not-cumulative 分类，
        // 不能被上面那条吞掉（顺序有讲究）。
        (
            "rejected: def_eq failed: def_eq mismatch expected: Sort(1) | actual: Sort(0)",
            ErrorKind::KernelPropNotCumulative,
        ),
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
        // 分类器默认维持 KernelRejected，**只有**「要 Type、给了 Prop」这一个
        // 能精确命名的形状（L-06 无累积性）另给专用码。
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
        // L-06：期望 Sort(n>0)（数据/Type）、实际 Sort(0)（Prop）⇒ 无累积性。
        (
            "rejected: def_eq failed: def_eq mismatch expected: Sort(1) | actual: Sort(0)",
            ErrorKind::KernelPropNotCumulative,
        ),
        (
            "def_eq mismatch expected: Sort(1) | actual: Sort(0)",
            ErrorKind::KernelPropNotCumulative,
        ),
        (
            "def_eq mismatch expected: Sort(2) | actual: Sort(0)",
            ErrorKind::KernelPropNotCumulative,
        ),
        // **范围（有意收窄）**：`Pi` 形状的同一个现象保持通用码——它同时是
        // CLI/LSP/扩展契约测试用来代表"通用内核拒绝"的夹具（钉 code/stage/span），
        // 加宽要动那些夹具，留给专门迁移（设计 §3）。
        (
            "def_eq mismatch expected: Pi (x : Prop), Sort(1) | actual: Pi (x : Prop), Sort(0)",
            ErrorKind::KernelRejected,
        ),
        (
            "def_eq mismatch expected: Pi (x : Nat), Sort(1) | actual: Pi (x : Nat), Sort(0)",
            ErrorKind::KernelRejected,
        ),
        // 反方向（期望 Prop、实际 Type）**故意不分类**：它同时也是普通的
        // 「该写 Prop 却写了 Type」，命名成"大消去"会误标（设计 §4）。
        (
            "def_eq mismatch expected: Sort(0) | actual: Sort(1)",
            ErrorKind::KernelRejected,
        ),
        (
            "def_eq mismatch expected: Pi (x : Exists A p), Sort(0) | actual: Pi (_ : Exists A p), Sort(1)",
            ErrorKind::KernelRejected,
        ),
        // 两侧都非 0（宇宙层级本身写错）与变量层级（`Sort(u)`）都留给通用码。
        (
            "def_eq mismatch expected: Sort(2) | actual: Sort(1)",
            ErrorKind::KernelRejected,
        ),
        (
            "def_eq mismatch expected: Sort(1) | actual: Sort(u)",
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
        ErrorKind::KernelPropNotCumulative,
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
        "kernel-prop-not-cumulative",
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

/// **L-06 ①**：`def T : Type := <Prop 值>` 今天有**专用码 + 人话 hint**，
/// 而不是裸 `kernel-rejected`（内核消息一字不改，只换分类与提示；设计
/// `docs/design/prop-cumulativity-boundary.md` §3）。
#[test]
fn pipeline_classifies_prop_where_type_was_required() {
    for src in ["def T : Type := True\n", "def T : Type := And True True\n"] {
        let out = compile_fol(&parse(src).expect("parse Prop-as-Type"));
        let err = out
            .errors
            .first()
            .unwrap_or_else(|| panic!("no error for {src:?}"));
        assert_eq!(
            err.kind,
            ErrorKind::KernelPropNotCumulative,
            "{src:?} → {:?}",
            out.errors
        );
        assert_eq!(err.code(), "kernel-prop-not-cumulative");
        assert!(err.hint().contains("累积性"), "hint: {}", err.hint());
        assert_eq!(err.expected.as_deref(), Some("Sort(1)"), "{err:?}");
        assert_eq!(err.actual.as_deref(), Some("Sort(0)"), "{err:?}");
        // 消息保持内核原样（只加分类，不改判据）。
        assert_eq!(err.message, "类型不匹配：期望 `Sort(1)`，实际是 `Sort(0)`");
    }
}

/// 通用路径的**端到端对照**：不是 L-06 形状的 def-eq 不匹配仍是
/// `kernel-rejected`，并且照旧带 `expected`/`actual`（新码只接管它精确命名的
/// 那一个形状；管道是共用的）。
#[test]
fn generic_def_eq_mismatch_keeps_kernel_rejected() {
    let out = compile_fol(&parse("def bad : Nat := True\n").unwrap());
    assert_eq!(out.errors.len(), 1, "{:?}", out.errors);
    let err = &out.errors[0];
    assert_eq!(err.kind, ErrorKind::KernelRejected, "{err:?}");
    assert_eq!(err.code(), "kernel-rejected");
    assert!(
        err.expected
            .as_deref()
            .unwrap_or_default()
            .starts_with("Nat"),
        "{err:?}"
    );
    assert_eq!(err.actual.as_deref(), Some("Sort(0)"), "{err:?}");
}

/// 专用码的**判别性对照**：反方向（该写 `Prop` 却写了 `Type`）与宇宙层级本身
/// 写错都必须留在通用 `kernel-rejected`，否则 hint 会误标；正常路径不受影响。
#[test]
fn prop_not_cumulative_code_does_not_catch_other_kernel_gaps() {
    // 该写 Prop、写了 Type：形状是 Sort(0) vs Sort(1)，**不是** L-06 ①。
    let out = compile_fol(&parse("def f : Prop := Nat\n").unwrap());
    assert!(!out.errors.is_empty());
    assert!(
        out.errors
            .iter()
            .all(|e| e.kind != ErrorKind::KernelPropNotCumulative),
        "the mirror direction must stay generic: {:?}",
        out.errors
    );
    // Pi 形状的**同一个现象**（把命题当函数的返回类型）保持通用码：它同时是
    // CLI/LSP/扩展契约测试用来代表"通用内核拒绝"的夹具（钉 code/stage/span），
    // 加宽要动那些夹具（设计 §3）。
    let out = compile_fol(&parse("def bad : Prop -> Type := fun (x : Prop) => x\n").unwrap());
    assert_eq!(out.errors.len(), 1, "{:?}", out.errors);
    assert_eq!(
        out.errors[0].kind,
        ErrorKind::KernelRejected,
        "{:?}",
        out.errors
    );
    // 正常路径：Type 值交 Type 结论，照常 checked。
    let out = compile_fol(&parse("def U : Type := Nat\n").unwrap());
    assert_eq!(out.errors, vec![], "{:?}", out.errors);
}

/// **L-06 ②**：`Exists.elim` 的 `Q` 只能是 `Prop`——想**取数据**的引理被内核
/// 拒绝（`Exists.rec` 的 motive 只到 `Sort 0`）。这条把边界本身钉进测试：
/// `Exists.elim`（常值 Prop motive）checked，`Exists.witness`（Type motive）
/// 判红，且**保持通用码**（见上一条测试的判别性理由）。
#[test]
fn exists_eliminator_motive_stays_in_prop() {
    let src = "\
inductive Exists (A : Type) (p : A -> Prop) : Prop\n\
ctor intro (w : A) (h : p w) : Exists A p\n\
end\n\
def Exists.elim (A : Type) (p : A -> Prop) (Q : Prop) (h : Exists A p) (f : (w : A) -> p w -> Q) : Q :=\n\
  Exists.rec A p (fun (_ : Exists A p) => Q) f h\n\
def Exists.witness (A : Type) (p : A -> Prop) (h : Exists A p) : A :=\n\
  Exists.rec A p (fun (_ : Exists A p) => A) (fun (w : A) (hw : p w) => w) h\n";
    let out = compile_fol(&parse(src).expect("parse Exists probe"));
    assert_eq!(out.errors.len(), 1, "{:?}", out.errors);
    let err = &out.errors[0];
    assert_eq!(err.kind, ErrorKind::KernelRejected, "{err:?}");
    let expected = err.expected.as_deref().unwrap_or_default();
    let actual = err.actual.as_deref().unwrap_or_default();
    assert!(expected.ends_with("Sort(0)"), "{err:?}");
    assert!(actual.ends_with("Sort(1)"), "{err:?}");
    // `Exists.elim` 本身（Q : Prop）必须 checked：边界只在"取数据"那一侧。
    let checked = out
        .events
        .iter()
        .filter(|e| matches!(e, CheckEvent::DeclarationChecked { name } if name == "Exists.elim"))
        .count();
    assert_eq!(checked, 1, "events: {:?}", out.events);
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
    // `theorem` 要求签名是 Prop（G-01 起开练习也受检），
    // 这里考的是参数位置洞的 binder 类型 ⇒ 用 `def`。
    let src = concat!(
        "def add1 : Nat -> Nat := fun n => n + 1\n",
        "def t : Nat -> Nat := fun (n : Nat) => add1 (sorry)\n",
    );
    let report = check_document(&parse(src).expect("parse"));
    let open = open_exercise(&report, 1);
    assert_eq!(open.sub_goals[0].ty.as_deref(), Some("Nat"));
}

#[test]
fn nested_function_hole_becomes_generic_open_exercise() {
    // 0.23.0：嵌套洞不再报 misplaced——fallback 生成 generic open
    // exercise（整值 = 一个洞）。宽松行为优于报错（用户反馈）。
    let src = concat!(
        "theorem eq_symm_nat : (a : Nat) -> (b : Nat) -> Eq.{1} Nat a b -> Eq.{1} Nat b a :=\n",
        "  fun (a : Nat) (b : Nat) (h : Eq.{1} Nat a b) =>\n",
        "    Eq.subst.{1} Nat (fun (x : Nat) => sorry) a b h (Eq.refl.{1} Nat a)\n",
    );
    let report = check_document(&parse(src).expect("parse"));
    assert!(
        !report
            .errors
            .iter()
            .any(|e| e.kind == ErrorKind::ElabHoleMisplaced),
        "nested holes should be caught by the fallback, not error: {:?}",
        report.errors
    );
}

// ---- spine meta 方案 A：请求期 kernel 探针（design spine-meta-a.md）----

/// 对声明 `name` 跑探针，返回 `(洞起点 offset, 类型文本)`。
fn probe_for(doc: &str, name: &str) -> Vec<(usize, String)> {
    let report = check_document(&parse(doc).expect("parse"));
    let d = report
        .decls
        .iter()
        .find(|d| d.name.as_deref() == Some(name))
        .expect("declaration");
    probe_sub_goal_types(doc, &CompileOptions::default(), d.span)
}

fn probed_ty(doc: &str, name: &str, hole_offset: usize) -> Option<String> {
    probe_for(doc, name)
        .into_iter()
        .find(|(o, _)| *o == hole_offset)
        .map(|(_, ty)| ty)
}

#[test]
fn probe_fills_defeq_alias_domain_after_a_preceding_hole() {
    // defeq 别名 + 前置洞穿透：`h : (a : Prop) -> Not a -> a`，第二个实参
    // 期望 `Not <第一个洞的期望类型>`。B′ 的 AST 替换看到前置洞只能给
    // None；探针把第一个洞提升为局部 binder `_h0 : Prop` 后让内核算。
    let src = "axiom False : Prop\n\
               def Not : Prop -> Prop := fun (a : Prop) => a -> False\n\
               axiom h : (a : Prop) -> Not a -> a\n\
               theorem t : (a : Prop) -> Not a -> a :=\n\
                 fun (a : Prop) => fun (na : Not a) => h (sorry) (sorry)\n";
    let report = check_document(&parse(src).expect("parse"));
    let d = report
        .decls
        .iter()
        .find(|d| d.name.as_deref() == Some("t"))
        .expect("decl t");
    // B′ 快路径不变：第一个洞有类型，第二个是 None（既有断言语义）。
    assert_eq!(d.sub_goals[0].ty.as_deref(), Some("Prop"));
    assert_eq!(d.sub_goals[1].ty, None);
    let second = d.sub_goals[1].span.start.offset;
    assert_eq!(probed_ty(src, "t", second).as_deref(), Some("Not _h0"));
}

#[test]
fn probe_fills_dependent_field_via_substitution() {
    // 依赖字段经替换：`Eq.subst.{1} Nat (sorry) a b h (sorry)` 的末位洞
    // 期望 `p a`；p 是前置洞（合成名 `_h1`）。探针让内核推断部分应用
    // `Eq.subst Nat _h1 a b h` 的类型 `_h1 a -> _h1 b`，剥域得 `_h1 a`。
    let src = concat!(
        "theorem t : (p : Nat -> Prop) -> (a : Nat) -> (b : Nat) -> p a -> p b :=\n",
        "  fun (p : Nat -> Prop) (a : Nat) (b : Nat) (h : p a) =>\n",
        "    Eq.subst.{1} Nat (sorry) a b h (sorry)\n",
    );
    let report = check_document(&parse(src).expect("parse"));
    let d = report
        .decls
        .iter()
        .find(|d| d.name.as_deref() == Some("t"))
        .expect("decl t");
    assert_eq!(d.sub_goals[0].ty.as_deref(), Some("Nat -> Prop"));
    assert_eq!(d.sub_goals[1].ty, None);
    let second = d.sub_goals[1].span.start.offset;
    assert_eq!(probed_ty(src, "t", second).as_deref(), Some("_h1 a"));
}

#[test]
fn probe_fills_one_level_nested_hole_expected_type() {
    // 一层嵌套洞 `h (g sorry)`：廉价 walk 给出**内层** sorry 的 span，
    // 类型交给请求期探针（`g` 的定义域）。目标宣称成真命题 `P`
    // （G-01 起 `theorem` 的签名必须真的是 Prop，不能再拿 `Prop` 当目标）。
    let src = "axiom g : (a : Prop) -> Prop\n\
               axiom h : (b : Prop) -> Prop\n\
               axiom P : Prop\n\
               theorem t : P := h (g sorry)\n";
    let report = check_document(&parse(src).expect("parse"));
    let d = report
        .decls
        .iter()
        .find(|d| d.name.as_deref() == Some("t"))
        .expect("decl t");
    assert_eq!(d.status, DeclStatus::Open);
    assert_eq!(d.holes.len(), 1, "the inner sorry is the hole");
    assert_eq!(d.sub_goals.len(), 1);
    assert_eq!(d.sub_goals[0].ty, None, "B′ leaves the nested hole unknown");
    assert_eq!(
        &src[d.holes[0].start.offset..d.holes[0].end.offset],
        "sorry"
    );
    assert_eq!(
        probed_ty(src, "t", d.holes[0].start.offset).as_deref(),
        Some("Prop")
    );
}

#[test]
fn probe_deeper_than_one_level_falls_back_to_none() {
    // v1 上限：超过一层的嵌套不识别为精确子洞，走既有 generic fallback
    // （整值 = 一个洞，sub_goals 为空），探针也返回空——绝不比 B′ 差。
    // 目标同样用真命题 `P`（见上一个测试的说明）。
    let src = "axiom k : (a : Prop) -> Prop\n\
               axiom g : (a : Prop) -> Prop\n\
               axiom h : (b : Prop) -> Prop\n\
               axiom P : Prop\n\
               theorem t : P := h (g (k sorry))\n";
    let report = check_document(&parse(src).expect("parse"));
    let d = report
        .decls
        .iter()
        .find(|d| d.name.as_deref() == Some("t"))
        .expect("decl t");
    assert_eq!(d.status, DeclStatus::Open);
    assert_eq!(d.holes.len(), 1, "generic fallback: the whole value");
    assert!(d.sub_goals.is_empty(), "{:?}", d.sub_goals);
    assert!(
        probe_for(src, "t").is_empty(),
        "deeper nesting must stay a fallback, not a guess"
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
            "axiom And : Prop -> Prop -> Prop\n\
             axiom True : Prop\n\
             axiom False : Prop\n\
             axiom Pair : Prop -> Prop -> Prop\n\
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
        Some("And True False"),
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
        Some("(a : Prop) -> And a False"),
        "shadowed inner `a` stays, unshadowed `b` is substituted: {:?}",
        open.sub_goals[0].ty
    );
}

#[test]
fn spine_holes_without_a_known_template_stay_open() {
    // 没有兄弟 axiom/ctor 模板也能合法多洞（子目标类型缺省）。
    // 注意 `A` 必须是**命题**（`(A : Prop -> Prop) -> A -> A` 里的 `A` 是函数，
    // 不是类型——G-01 起这种签名会被内核拒）。
    let report = check_document(
        &parse("example : (A : Prop) -> A -> A := fun (A : Prop) => sorry\n").expect("parse"),
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
        &parse("theorem t : (a : Prop) -> a -> a := fun (a : Prop) => fun (h : a) => sorry\n")
            .expect("parse"),
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
        ty.contains("And a b") && ty.contains("forall") || ty.contains("→"),
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
fn by_block_with_match_tactic_checks() {
    // `match` 作为 tactic（臂体是项），以当前目标为期望类型判定。
    let src = concat!(
        "inductive Color : Type\nctor red : Color\nctor green : Color\nend\n",
        "def swap (c : Color) : Color := by match c with | red => green | green => red\n",
    );
    let report = check_document(&parse(src).unwrap());
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    let d = report
        .decls
        .iter()
        .find(|d| d.name.as_deref() == Some("swap"))
        .unwrap();
    assert_eq!(d.status, DeclStatus::Checked);
}

#[test]
fn by_block_with_exact_match_checks() {
    // 等价写法：`by exact match …`（judge 合成文件保留前缀，宇宙查询可用）。
    let src = concat!(
        "inductive Color : Type\nctor red : Color\nctor green : Color\nend\n",
        "def swap (c : Color) : Color := by exact match c with | red => green | green => red\n",
    );
    let report = check_document(&parse(src).unwrap());
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    let d = report
        .decls
        .iter()
        .find(|d| d.name.as_deref() == Some("swap"))
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
fn by_block_carries_the_declaration_universe_parameters() {
    // 目标里出现 `Sort u` / `Eq.{u}` 时，判定合成的声明必须带上声明的宇宙
    // 参数 `{u}`，否则内核报「universe variable `u` is not declared in this
    // declaration」——宇宙多态定理（卷 I 的 `Set.{u}` 遍地都是）就没法写 tactic。
    let src = concat!(
        "theorem Eq.flip {u} : {α : Sort u} -> (a : α) -> (b : α) -> ",
        "Eq.{u} α a b -> Eq.{u} α b a := by\n",
        "  intro α a b h\n",
        "  exact Eq.subst.{u} α (fun (x : α) => Eq.{u} α x a) a b h (Eq.refl.{u} α a)\n",
    );
    let report = check_document(&parse(src).unwrap());
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    let d = report
        .decls
        .iter()
        .find(|d| d.name.as_deref() == Some("Eq.flip"))
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
    // （应用链左结合，函数位置不补括号）。
    //
    // **线 C（T-C22）之后**：这是**展示副本**，过一遍记法折叠 ⇒ `And a a` 打成
    // `a ∧ a`。判定输入（引擎手里的 AST）没动——见
    // `query::tests::by_step_display_is_folded_but_the_judge_input_is_not`。
    let s0 = &d.by_steps[0];
    assert_eq!(s0.goals.len(), 1);
    assert_eq!(s0.goals[0].ty, "a ∧ a → a");
    assert_eq!(s0.goals[0].binders.len(), 1);
    assert_eq!(s0.goals[0].binders[0].name, "a");
    assert_eq!(s0.goals[0].binders[0].ty, "Prop");
    assert_eq!(&src[s0.span.start.offset..s0.span.end.offset], "intro a");
    // step 1 = `intro h` 执行后：h : And a a，目标剩 `a`。
    let s1 = &d.by_steps[1];
    assert_eq!(s1.goals.len(), 1);
    assert_eq!(s1.goals[0].ty, "a");
    assert_eq!(s1.goals[0].binders.len(), 2);
    assert_eq!(s1.goals[0].binders[1].name, "h");
    // 假设行的类型也过折叠（同样是展示副本）。
    assert_eq!(s1.goals[0].binders[1].ty, "a ∧ a");
    assert_eq!(&src[s1.span.start.offset..s1.span.end.offset], "intro h");
}

#[test]
fn apply_records_all_open_goals_current_first() {
    // 多目标（本轮修复）：`apply And.intro` 开出两个子目标，per-step 状态
    // 必须记下全部（当前目标在首位），否则 goal 面板只能显示一个。
    let src = "axiom And : Prop -> Prop -> Prop\n\
               axiom P : Prop\n\
               axiom Q : Prop\n\
               axiom And.intro : (a : Prop) -> (b : Prop) -> a -> b -> And a b\n\
               theorem both : And P Q := by apply And.intro\n";
    let report = check_document(&parse(src).unwrap());
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    let d = report
        .decls
        .iter()
        .find(|d| d.name.as_deref() == Some("both"))
        .unwrap();
    let step = &d.by_steps[0];
    let tys: Vec<&str> = step.goals.iter().map(|g| g.ty.as_str()).collect();
    assert_eq!(
        tys,
        vec!["P", "Q"],
        "both apply sub-goals recorded, current first"
    );
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
    assert_eq!(d.by_steps[0].goals[0].ty, "a → a");
    assert!(
        d.by_steps[2].goals.is_empty(),
        "all goals closed by `exact`"
    );
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
    // `axiom Prop : Sort 1` 能通过内核，但这个名字永不被引用（`Prop` 内核
    // 已经定义过，代码里的 `Prop` 都指内核那个）——产出语法级 warning，
    // 不是 error。
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

#[test]
fn type_with_level_is_lean_sort_succ() {
    // Lean 记法：`Type n` = `Sort (n + 1)`；`Type 0` = `Sort 1`。
    let src = "axiom T0 : Type 0\naxiom T2 : Type 2\n\
               def idT : Type -> Type := fun (x : Type) => x\n";
    let out = compile_fol(&parse(src).unwrap());
    assert_eq!(out.errors, vec![], "{:?}", out.errors);

    // `#check Type 0`：内核打印 `Type 1`（`Sort 1` 的类型是 `Sort 2`）。
    let file = parse("#check Type 0\n#check Type 2\n").unwrap();
    let out = compile_fol(&file);
    let texts: Vec<&str> = out
        .events
        .iter()
        .filter_map(|e| match e {
            CheckEvent::TypeChecked { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(texts, vec!["Type 1", "Type 3"]);
}

// ---- Phase 1: 值位 `let`（docs/design/elaborator-let-match.md，S1–S3）----

#[test]
fn let_definition_checks_through_kernel() {
    let src = "def two : Nat := let one : Nat := Nat.succ Nat.zero; one + one\n";
    let out = compile_fol(&parse(src).unwrap());
    assert_eq!(out.errors, vec![], "{:?}", out.errors);
    assert!(matches!(
        out.events.first(),
        Some(CheckEvent::DeclarationChecked { name }) if name == "two"
    ));
}

#[test]
fn let_type_mismatch_is_kernel_rejected() {
    // 注解写 Nat，值却是 Prop：前端不做等价性检查，完整内核拒绝。
    let src = "def bad : Nat := let x : Nat := Prop; x\n";
    let out = compile_fol(&parse(src).unwrap());
    assert_eq!(out.errors.len(), 1, "{:?}", out.errors);
    assert_eq!(out.errors[0].code(), "kernel-rejected");
}

#[test]
fn let_without_annotation_infers_the_value_type() {
    // Phase 2：无注解 `let` 由内核推断值类型（judge_infer，复用有界缓存）。
    let src = "def one : Nat := let x := Nat.zero; x\n\
               def id2 : Nat -> Nat := let f := fun (n : Nat) => n; f\n";
    let out = compile_fol(&parse(src).unwrap());
    assert!(out.errors.is_empty(), "{:?}", out.errors);
    for name in ["one", "id2"] {
        assert!(
            out.events
                .iter()
                .any(|e| matches!(e, CheckEvent::DeclarationChecked { name: n } if n == name)),
            "{name} must check through the kernel"
        );
    }
}

#[test]
fn unannotated_let_that_cannot_be_inferred_reports_a_let_specific_error() {
    // 推断不出值类型（值位洞 / 无类型 binder）→ 教学错误，提示补类型标注。
    let src = "def bad : Nat := let x := sorry; x\n";
    let out = compile_fol(&parse(src).unwrap());
    assert_eq!(out.errors.len(), 1, "{:?}", out.errors);
    assert_eq!(out.errors[0].code(), "elab-let-type-query-failed");
    assert!(
        out.errors[0].message.contains("let"),
        "let-specific message expected, got {:?}",
        out.errors[0].message
    );
    assert!(out.errors[0].hint().contains("let"));
}

#[test]
fn let_value_is_not_in_scope_for_itself() {
    // `x` 的类型/值都在引入 x 之前 elaborate，值位引用 x 必须是未知标识符。
    let src = "def bad : Nat := let x : Nat := x; x\n";
    let out = compile_fol(&parse(src).unwrap());
    assert_eq!(out.errors.len(), 1, "{:?}", out.errors);
    assert_eq!(out.errors[0].code(), "elab-unknown-identifier");
}

#[test]
fn let_scope_shadows_and_restores() {
    let src = "def shadow : Nat := let x : Nat := 1; let x : Nat := 2; x\n";
    let out = compile_fol(&parse(src).unwrap());
    assert_eq!(out.errors, vec![], "{:?}", out.errors);
}

#[test]
fn let_expected_type_infers_untyped_lambda() {
    // 值的期望类型 = binder 注解，`fun y => y` 借它推断出 y : Nat。
    let src = "def applied : Nat := let f : Nat -> Nat := fun y => y; f 3\n";
    let out = compile_fol(&parse(src).unwrap());
    assert_eq!(out.errors, vec![], "{:?}", out.errors);
}

#[test]
fn let_works_in_check_and_reduce() {
    let src = "#check (let x : Nat := 1; x)\n#reduce (let x : Nat := 1; x + 2)\n";
    let out = compile_fol(&parse(src).unwrap());
    assert_eq!(out.errors, vec![], "{:?}", out.errors);
    assert!(matches!(
        &out.events[0],
        CheckEvent::TypeChecked { text, .. } if text == "Nat"
    ));
    assert!(matches!(
        &out.events[1],
        CheckEvent::Reduced { text, .. } if text == "3"
    ));
}

#[test]
fn let_value_hole_reports_subgoal_of_annotated_type() {
    let src = "example : Nat := let x : Nat := sorry; x\n";
    let report = check_document(&parse(src).unwrap());
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    let d = &report.decls[0];
    assert_eq!(d.status, DeclStatus::Open);
    assert_eq!(d.goal.as_deref(), Some("Nat"));
    assert_eq!(d.holes.len(), 1);
    assert_eq!(
        &src[d.holes[0].start.offset..d.holes[0].end.offset],
        "sorry"
    );
    assert_eq!(d.sub_goals.len(), 1, "the value hole is a sub-goal");
    assert_eq!(d.sub_goals[0].ty.as_deref(), Some("Nat"));
}

#[test]
fn let_body_hole_opens_with_binder_in_context() {
    let src = "example : Nat := let x : Nat := Nat.zero; sorry\n";
    let report = check_document(&parse(src).unwrap());
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    let d = &report.decls[0];
    assert_eq!(d.status, DeclStatus::Open);
    assert_eq!(d.goal.as_deref(), Some("Nat"));
    assert_eq!(d.holes.len(), 1);
    assert_eq!(
        d.binders,
        vec![GoalBinder {
            name: "x".to_string(),
            ty: "Nat".to_string(),
        }]
    );
}

#[test]
fn let_body_hole_under_lambda_reports_both_binders() {
    let src = "example : Nat -> Nat := fun (n : Nat) => let m : Nat := n + n; sorry\n";
    let report = check_document(&parse(src).unwrap());
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    let d = &report.decls[0];
    assert_eq!(d.status, DeclStatus::Open);
    assert_eq!(d.goal.as_deref(), Some("Nat"));
    assert_eq!(
        d.binders,
        vec![
            GoalBinder {
                name: "n".to_string(),
                ty: "Nat".to_string(),
            },
            GoalBinder {
                name: "m".to_string(),
                ty: "Nat".to_string(),
            },
        ]
    );
}

#[test]
fn let_binder_hover_and_go_to_definition() {
    let src = "def id2 : Nat := let x : Nat := 1; x\n";
    let report = check_document(&parse(src).unwrap());
    let decl = src.find("x : Nat").expect("binder annotation");
    assert!(
        report.hovers.iter().any(|h| {
            h.binder && h.span.start.offset == decl && h.span.end.offset == decl + "x : Nat".len()
        }),
        "expected a `x : Nat` binder row, got {:?}",
        report
            .hovers
            .iter()
            .map(|h| (h.span.start.offset, h.span.end.offset, h.binder))
            .collect::<Vec<_>>()
    );
    let use_off = src.rfind('x').expect("body x");
    let resolved = report.hovers.iter().find_map(|h| {
        (h.span.start.offset <= use_off && use_off < h.span.end.offset && h.resolution.is_some())
            .then(|| h.resolution.clone())
            .flatten()
    });
    assert!(
        matches!(resolved, Some(ResolvedTarget::Binder(_))),
        "body `x` must resolve to the let binder, got {resolved:?}"
    );
}

#[test]
fn let_and_beta_expansion_agree_when_open() {
    // 契约（zeta 等价）：`let x : T := v; body` 与其 beta 展开
    // `(fun (x : T) => body) v` 的 status/goal/洞数一致——降低只做结构，
    // 判定走内核。
    let let_src = "example : Nat := let x : Nat := sorry; x\n";
    let beta_src = "example : Nat := (fun (x : Nat) => x) sorry\n";
    let let_report = check_document(&parse(let_src).unwrap());
    let beta_report = check_document(&parse(beta_src).unwrap());
    let let_d = &let_report.decls[0];
    let beta_d = &beta_report.decls[0];
    assert_eq!(let_d.status, DeclStatus::Open);
    assert_eq!(let_d.status, beta_d.status);
    assert_eq!(let_d.goal, beta_d.goal);
    assert_eq!(let_d.holes.len(), beta_d.holes.len());
}

#[test]
fn let_and_beta_expansion_agree_when_closed() {
    let let_src = "def two : Nat := let one : Nat := Nat.succ Nat.zero; one + one\n";
    let beta_src = "def two : Nat := (fun (one : Nat) => one + one) (Nat.succ Nat.zero)\n";
    let let_out = compile_fol(&parse(let_src).unwrap());
    let beta_out = compile_fol(&parse(beta_src).unwrap());
    assert_eq!(let_out.errors, vec![], "{:?}", let_out.errors);
    assert_eq!(beta_out.errors, vec![], "{:?}", beta_out.errors);
    assert_eq!(let_out.events, beta_out.events);
}

// ---- match（design docs/design/match.md，v1）----

/// 源内非递归枚举，无显式 rec：recursor 由前端派生（`Color.rec.{u}`）。
const COLOR_ENUM: &str = "\
inductive Color : Type
ctor red : Color
ctor green : Color
ctor blue : Color
end
";

fn color_enum() -> &'static str {
    COLOR_ENUM
}

#[test]
fn match_enum_swap_checks_with_kernel() {
    let src = format!(
        "{}\n\
         def swap (c : Color) : Color := match c with\n\
         | red => green\n\
         | green => red\n\
         | blue => blue\n",
        color_enum()
    );
    let out = compile_fol(&parse(&src).expect("parse match"));
    assert_eq!(out.errors, vec![], "errors: {:?}", out.errors);
    assert!(out
        .events
        .iter()
        .any(|e| matches!(e, CheckEvent::DeclarationChecked { name } if name == "swap")));
}

#[test]
fn match_reorders_arms_by_constructor_declaration_order() {
    // 用户乱序写分支：与声明序（red, green, blue）等价，内核判定通过。
    let src = format!(
        "{}\n\
         def swap (c : Color) : Color := match c with\n\
         | blue => blue\n\
         | green => red\n\
         | red => green\n",
        color_enum()
    );
    let out = compile_fol(&parse(&src).expect("parse match"));
    assert_eq!(out.errors, vec![], "errors: {:?}", out.errors);
}

#[test]
fn match_single_ctor_struct_checks() {
    let src = format!(
        "{}\n\
         inductive Wrap : Type\n\
         ctor wrap (c : Color) : Wrap\n\
         end\n\
         def unwrap (w : Wrap) : Color := match w with | wrap c => c\n",
        color_enum()
    );
    let out = compile_fol(&parse(&src).expect("parse match"));
    assert_eq!(out.errors, vec![], "errors: {:?}", out.errors);
    assert!(out
        .events
        .iter()
        .any(|e| matches!(e, CheckEvent::DeclarationChecked { name } if name == "unwrap")));
}

#[test]
fn match_prop_result_checks() {
    // R = Prop ⇒ motive 的宇宙层级为 1（judge_infer(Prop) = Type）。
    let src = format!(
        "{}\n\
         axiom True : Prop\n\
         axiom False : Prop\n\
         def isGreen (c : Color) : Prop := match c with\n\
         | red => True\n\
         | green => False\n\
         | blue => False\n",
        color_enum()
    );
    let out = compile_fol(&parse(&src).expect("parse match"));
    assert_eq!(out.errors, vec![], "errors: {:?}", out.errors);
    assert!(out
        .events
        .iter()
        .any(|e| matches!(e, CheckEvent::DeclarationChecked { name } if name == "isGreen")));
}

#[test]
fn match_sorry_branch_is_open_with_result_type() {
    let src = format!(
        "{}\n\
         example (c : Color) : Color := match c with\n\
         | red => green\n\
         | green => red\n\
         | blue => sorry\n",
        color_enum()
    );
    let report = check_document(&parse(&src).expect("parse match"));
    assert_eq!(report.errors, vec![], "errors: {:?}", report.errors);
    let decl = report
        .decls
        .iter()
        .find(|d| d.status == DeclStatus::Open)
        .expect("the example should be open");
    assert_eq!(decl.goal.as_deref(), Some("Color"));
    assert_eq!(decl.holes.len(), 1);
    assert_eq!(decl.sub_goals.len(), 1);
    assert_eq!(decl.sub_goals[0].ty.as_deref(), Some("Color"));
}

#[test]
fn match_non_exhaustive_reports_code() {
    let src = format!(
        "{}\n\
         def f (c : Color) : Color := match c with\n\
         | red => green\n\
         | green => red\n",
        color_enum()
    );
    let out = compile_fol(&parse(&src).expect("parse match"));
    assert_eq!(out.errors[0].code(), "elab-match-non-exhaustive");
}

#[test]
fn match_unknown_bare_name_is_a_binding() {
    // Lean 语义：未知的裸名按**绑定变量**处理（不是构造子）。变量模式不可反驳，
    // 所以它匹配一切、后面的 arm 不可达，整个 match 通过覆盖性检查。
    let src = format!(
        "{}\n\
         def f (c : Color) : Color := match c with\n\
         | purple => green\n\
         | green => red\n\
         | blue => blue\n\
         #reduce f blue\n",
        color_enum()
    );
    let out = compile_fol(&parse(&src).expect("parse match"));
    assert_eq!(out.errors, vec![], "errors: {:?}", out.errors);
    assert!(
        out.events
            .iter()
            .any(|e| matches!(e, CheckEvent::Reduced { text, .. } if text == "Color.green")),
        "a variable pattern shadows later arms: {:?}",
        out.events
    );
}

#[test]
fn match_unknown_ctor_with_args_reports_bad_arm() {
    // 带子模式的未知名字不可能是绑定 ⇒ bad-arm，消息点名 culprit。
    let src = format!(
        "{}\n\
         def f (c : Color) : Color := match c with\n\
         | purple x => red\n\
         | red => red\n\
         | green => red\n\
         | blue => blue\n",
        color_enum()
    );
    let out = compile_fol(&parse(&src).expect("parse match"));
    assert_eq!(out.errors[0].code(), "elab-match-bad-arm");
    assert!(
        out.errors[0].message.contains("purple"),
        "message should name the bad ctor: {:?}",
        out.errors[0]
    );
}

#[test]
fn match_duplicate_ctor_falls_through_in_order() {
    // 模式编译器起，同一构造子可以出现多条 arm：**有序、首个匹配者胜**
    // （docs/design/match-patterns.md §4）。
    let src = format!(
        "{}\n\
         def f (c : Color) : Color := match c with\n\
         | red => green\n\
         | red => blue\n\
         | green => red\n\
         | blue => blue\n\
         #reduce f red\n",
        color_enum()
    );
    let out = compile_fol(&parse(&src).expect("parse match"));
    assert_eq!(out.errors, vec![], "errors: {:?}", out.errors);
    assert!(
        out.events
            .iter()
            .any(|e| matches!(e, CheckEvent::Reduced { text, .. } if text == "Color.green")),
        "the first matching arm must win: {:?}",
        out.events
    );
}

const NAT2_ENUM: &str = "\
inductive Nat2 : Type
ctor z : Nat2
ctor s (n : Nat2) : Nat2
end
";

#[test]
fn match_recursive_inductive_uses_the_induction_hypothesis() {
    // Phase 2：递归归纳可用——递归字段后自动得到归纳假设 `ih`（类型 = 结果类型），
    // branch 里按名引用；用它写出的递归函数/证明无需自引用。
    let src = format!(
        "{NAT2_ENUM}\n\
         def pred (n : Nat2) : Nat2 := match n with\n\
         | z => z\n\
         | s k => k\n\
         def add (a b : Nat2) : Nat2 := match a with\n\
         | z => b\n\
         | s k => s ih\n\
         def two : Nat2 := s (s z)\n\
         #reduce add two two\n"
    );
    let out = compile_fol(&parse(&src).expect("parse match"));
    assert_eq!(out.errors, vec![], "errors: {:?}", out.errors);
    for name in ["pred", "add"] {
        assert!(
            out.events
                .iter()
                .any(|e| matches!(e, CheckEvent::DeclarationChecked { name: n } if n == name)),
            "{name} must check through the kernel"
        );
    }
    assert!(
        out.events.iter().any(|e| matches!(
            e,
            CheckEvent::Reduced { text, .. }
                if text == "Nat2.s (Nat2.s (Nat2.s (Nat2.s Nat2.z)))"
        )),
        "recursion via the IH must compute: {:?}",
        out.events
    );
}

#[test]
fn match_without_expected_type_reports_code() {
    // `#check` 位置没有期望类型 → 无法定 motive。
    let src = format!(
        "{}\n\
         #check match red with | red => green | green => red | blue => blue\n",
        color_enum()
    );
    let out = compile_fol(&parse(&src).expect("parse match"));
    assert_eq!(out.errors[0].code(), "elab-match-no-expected-type");
}

#[test]
fn match_and_handwritten_recursor_agree() {
    // 等价契约：同一枚举的 `match` 与手写 `<Ind>.rec.{level}` 应用判定一致
    // （都过内核；归约到同一构造子）。
    let src = format!(
        "{}\n\
         def swapM (c : Color) : Color := match c with\n\
         | red => green\n\
         | green => red\n\
         | blue => blue\n\
         def swapR (c : Color) : Color :=\n\
           Color.rec.{{1}} (fun (_ : Color) => Color) green red blue c\n\
         #reduce swapM red\n\
         #reduce swapR red\n",
        color_enum()
    );
    let out = compile_fol(&parse(&src).expect("parse match"));
    assert_eq!(out.errors, vec![], "errors: {:?}", out.errors);
    let reduced: Vec<&str> = out
        .events
        .iter()
        .filter_map(|e| match e {
            CheckEvent::Reduced { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(reduced, vec!["Color.green", "Color.green"]);
}

#[test]
fn match_scrutinee_ctor_uses_kernel_type_query() {
    // scrutinee 是顶层构造子（不在局部 scope）：走 judge_infer 类型查询找归纳头。
    let src = format!(
        "{}\n\
         def alwaysGreen : Color := match red with\n\
         | red => green\n\
         | green => green\n\
         | blue => green\n",
        color_enum()
    );
    let out = compile_fol(&parse(&src).expect("parse match"));
    assert_eq!(out.errors, vec![], "errors: {:?}", out.errors);
    assert!(out
        .events
        .iter()
        .any(|e| matches!(e, CheckEvent::DeclarationChecked { name } if name == "alwaysGreen")));
}

#[test]
fn match_prop_enum_uses_small_elimination_recursor() {
    // 多构造子 Prop 块 ⇒ 派生 recursor 无宇宙参数（`Two.rec`，不是 `Two.rec.{u}`）。
    let src = "\
inductive Two : Prop
ctor t1 : Two
ctor t2 : Two
end
def pick (x : Two) : Two := match x with | t1 => t1 | t2 => t2
";
    let out = compile_fol(&parse(src).expect("parse match"));
    assert_eq!(out.errors, vec![], "errors: {:?}", out.errors);
    assert!(out
        .events
        .iter()
        .any(|e| matches!(e, CheckEvent::DeclarationChecked { name } if name == "pick")));
}

// ---- match on the built-in prelude `Nat`（design docs/design/match.md §10）----

#[test]
fn match_prelude_nat_pred_checks() {
    // 文件未自带 `inductive Nat`：prelude 里的 `Nat` 被登记进 InductiveTable，
    // `match` 降低为 `Nat.rec.{1}`；分支用点号构造子名 `Nat.zero`/`Nat.succ`。
    let src = "def pred (n : Nat) : Nat := match n with\n\
               | Nat.zero => Nat.zero\n\
               | Nat.succ k => k\n";
    let out = compile_fol(&parse(src).expect("parse match"));
    assert_eq!(out.errors, vec![], "errors: {:?}", out.errors);
    assert!(out
        .events
        .iter()
        .any(|e| matches!(e, CheckEvent::DeclarationChecked { name } if name == "pred")));
}

#[test]
fn match_prelude_nat_add_checks_and_reduces() {
    // 递归分支拿到归纳假设 `ih`（与源内递归归纳同一 Phase-2 逻辑）；
    // `#reduce` 走降低出的 `Nat.rec`（内核原生 NatLit 快路径混入一元链，
    // 故 2 + 1 呈现为 `Nat.succ (Nat.succ 1)`，语义即 3）。
    let src = "def add (a b : Nat) : Nat := match a with\n\
               | Nat.zero => b\n\
               | Nat.succ k => Nat.succ ih\n\
               #reduce add (Nat.succ (Nat.succ Nat.zero)) (Nat.succ Nat.zero)\n";
    let out = compile_fol(&parse(src).expect("parse match"));
    assert_eq!(out.errors, vec![], "errors: {:?}", out.errors);
    assert!(out
        .events
        .iter()
        .any(|e| matches!(e, CheckEvent::DeclarationChecked { name } if name == "add")));
    assert!(
        out.events.iter().any(
            |e| matches!(e, CheckEvent::Reduced { text, .. } if text == "Nat.succ (Nat.succ 1)")
        ),
        "`add 2 1` must reduce through `Nat.rec`: {:?}",
        out.events
    );
}

// ---- match on the built-in prelude `Bool`（0.41.0）----

#[test]
fn bare_prelude_nat_names_stay_terminating() {
    // Boundary pinned in docs/architecture.md §5.4: `Nat.add` is a self-referential
    // placeholder reached only through the native fast path when *applied*; a bare
    // `#reduce`/`#check` must terminate and keep the constant (no delta loop).
    let src = "#reduce Nat.add\n#check Nat.add\n#check Nat.succ\n";
    let out = compile_fol(&parse(src).expect("parse checks"));
    assert_eq!(out.errors, vec![], "errors: {:?}", out.errors);
    assert!(
        out.events
            .iter()
            .any(|e| matches!(e, CheckEvent::Reduced { text, .. } if text == "Nat.add")),
        "bare `#reduce Nat.add` must terminate as the constant: {:?}",
        out.events
    );
    let types: Vec<&str> = out
        .events
        .iter()
        .filter_map(|e| match e {
            CheckEvent::TypeChecked { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(
        types,
        vec!["Nat -> Nat -> Nat", "Nat -> Nat"],
        "{:?}",
        out.events
    );
}

#[test]
fn prelude_bool_is_available_without_a_source_block() {
    // `Bool`/`Bool.true`/`Bool.false`/`Bool.rec` come from the trusted prelude
    // (installed like `Nat`); a plain file can `#check` them.
    let src = "#check Bool\n#check Bool.true\n#check Bool.false\n#check Bool.rec\n";
    let out = compile_fol(&parse(src).expect("parse checks"));
    assert_eq!(out.errors, vec![], "errors: {:?}", out.errors);
}

#[test]
fn match_prelude_bool_not_checks_and_reduces() {
    // Non-recursive prelude inductive: `match` lowers to `Bool.rec.{1}` with the
    // dotted constructor names, and `#reduce` runs the derived iota rule.
    let src = "def bnot (b : Bool) : Bool := match b with\n\
               | Bool.true => Bool.false\n\
               | Bool.false => Bool.true\n\
               #reduce bnot Bool.true\n\
               #reduce bnot Bool.false\n";
    let out = compile_fol(&parse(src).expect("parse match"));
    assert_eq!(out.errors, vec![], "errors: {:?}", out.errors);
    assert!(out
        .events
        .iter()
        .any(|e| matches!(e, CheckEvent::DeclarationChecked { name } if name == "bnot")));
    let reduced: Vec<&str> = out
        .events
        .iter()
        .filter_map(|e| match e {
            CheckEvent::Reduced { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(
        reduced,
        vec!["Bool.false", "Bool.true"],
        "`bnot` must reduce through `Bool.rec`: {:?}",
        out.events
    );
}

#[test]
fn prelude_bool_definitions_compose() {
    // A boolean `and` written with the prelude constructors checks by the real
    // kernel (both branches are total), and reducing it exercises `Bool.rec`.
    let src = "def band (a b : Bool) : Bool := match a with\n\
               | Bool.true => b\n\
               | Bool.false => Bool.false\n\
               #reduce band Bool.true Bool.false\n\
               #reduce band Bool.true Bool.true\n";
    let out = compile_fol(&parse(src).expect("parse match"));
    assert_eq!(out.errors, vec![], "errors: {:?}", out.errors);
    let reduced: Vec<&str> = out
        .events
        .iter()
        .filter_map(|e| match e {
            CheckEvent::Reduced { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(reduced, vec!["Bool.false", "Bool.true"], "{:?}", out.events);
}

#[test]
fn explicit_bool_block_yields_to_the_source_declaration() {
    // A file that declares its own `inductive Bool` owns the name entirely: no
    // duplicate-declaration panic, and the prelude's `Bool.true` is absent (the
    // source ctors are bare `tt`/`ff`).
    let src = "\
inductive Bool : Type
ctor tt : Bool
ctor ff : Bool
rec Bool.rec {u} : (motive : (b : Bool) -> Sort u) -> (mt : motive tt) -> (mf : motive ff) -> (b : Bool) -> motive b
iota tt := fun (motive : (b : Bool) -> Sort u) => fun (mt : motive tt) => fun (mf : motive ff) => mt
iota ff := fun (motive : (b : Bool) -> Sort u) => fun (mt : motive tt) => fun (mf : motive ff) => mf
end
def negate (b : Bool) : Bool := match b with
| tt => ff
| ff => tt
#reduce negate tt
";
    let out = compile_fol(&parse(src).expect("parse source Bool"));
    assert_eq!(out.errors, vec![], "errors: {:?}", out.errors);
    assert!(
        out.events
            .iter()
            .any(|e| matches!(e, CheckEvent::Reduced { text, .. } if text == "Bool.ff")),
        "source `Bool` must reduce with its own ctors: {:?}",
        out.events
    );
}

// ---- 模式编译器 v1：通配 / 嵌套 / 字面量 / 守卫（docs/design/match-patterns.md）----

/// 两个嵌套枚举，供嵌套模式测试。
const NESTED_ENUMS: &str = "\
inductive Inner : Type
ctor ia : Inner
ctor ib : Inner
end
inductive Outer : Type
ctor oi (i : Inner) : Outer
ctor on : Outer
end
";

#[test]
fn match_wildcard_falls_through_by_order() {
    let src = format!(
        "{}\n\
         def f (c : Color) : Color := match c with\n\
         | red => green\n\
         | _ => blue\n\
         #reduce f red\n\
         #reduce f green\n",
        color_enum()
    );
    let out = compile_fol(&parse(&src).expect("parse match"));
    assert_eq!(out.errors, vec![], "errors: {:?}", out.errors);
    let reduced: Vec<&str> = out
        .events
        .iter()
        .filter_map(|e| match e {
            CheckEvent::Reduced { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(
        reduced,
        vec!["Color.green", "Color.blue"],
        "{:?}",
        out.events
    );
}

#[test]
fn match_nested_patterns_use_the_inner_values() {
    // `oi (ia|ib)` 同一构造子两条 arm：嵌套模式由编译器生成内层 match。
    let src = format!(
        "{NESTED_ENUMS}\n\
         def f (o : Outer) : Inner := match o with\n\
         | oi ia => ib\n\
         | oi ib => ia\n\
         | on => ia\n\
         #reduce f (oi ia)\n\
         #reduce f (oi ib)\n\
         #reduce f on\n",
    );
    let out = compile_fol(&parse(&src).expect("parse match"));
    assert_eq!(out.errors, vec![], "errors: {:?}", out.errors);
    let reduced: Vec<&str> = out
        .events
        .iter()
        .filter_map(|e| match e {
            CheckEvent::Reduced { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(
        reduced,
        vec!["Inner.ib", "Inner.ia", "Inner.ia"],
        "{:?}",
        out.events
    );
}

#[test]
fn match_nested_wildcard_binds_and_defaults() {
    // 外层绑定 + 内层通配混用：`oi _` 覆盖未列出的内层值。
    let src = format!(
        "{NESTED_ENUMS}\n\
         def g (o : Outer) : Inner := match o with\n\
         | oi ia => ib\n\
         | oi _ => ia\n\
         | on => ia\n\
         #reduce g (oi ib)\n",
    );
    let out = compile_fol(&parse(&src).expect("parse match"));
    assert_eq!(out.errors, vec![], "errors: {:?}", out.errors);
    assert!(
        out.events
            .iter()
            .any(|e| matches!(e, CheckEvent::Reduced { text, .. } if text == "Inner.ia")),
        "{:?}",
        out.events
    );
}

#[test]
fn match_nat_literals_desugar_to_ctors() {
    let src = "def f (n : Nat) : Nat := match n with\n\
               | 0 => Nat.succ Nat.zero\n\
               | _ => Nat.zero\n\
               #reduce f 0\n\
               #reduce f 2\n";
    let out = compile_fol(&parse(src).expect("parse match"));
    assert_eq!(out.errors, vec![], "errors: {:?}", out.errors);
    assert!(
        out.events
            .iter()
            .any(|e| matches!(e, CheckEvent::Reduced { text, .. } if text == "Nat.zero")),
        "`f 2` takes the wildcard arm: {:?}",
        out.events
    );
}

#[test]
fn match_literal_can_be_combined_with_succ_patterns() {
    // `| 0 =>` 与 `| Nat.succ k =>` 混排（两个不同构造子）。
    let src = "def pred (n : Nat) : Nat := match n with\n\
               | 0 => 0\n\
               | Nat.succ k => k\n\
               #reduce pred 0\n\
               #reduce pred 3\n";
    let out = compile_fol(&parse(src).expect("parse match"));
    assert_eq!(out.errors, vec![], "errors: {:?}", out.errors);
}

#[test]
fn match_guard_falls_through_to_the_next_arm() {
    // `Bool.true if b`：守卫为真 → false；为假 → 落到 `_`（返回 a）。
    let src = "def g (a b : Bool) : Bool := match a with\n\
               | Bool.true if b => Bool.false\n\
               | _ => a\n\
               #reduce g Bool.true Bool.true\n\
               #reduce g Bool.true Bool.false\n";
    let out = compile_fol(&parse(src).expect("parse match"));
    assert_eq!(out.errors, vec![], "errors: {:?}", out.errors);
    let reduced: Vec<&str> = out
        .events
        .iter()
        .filter_map(|e| match e {
            CheckEvent::Reduced { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(reduced, vec!["Bool.false", "Bool.true"], "{:?}", out.events);
}

#[test]
fn match_guard_without_a_fallback_is_non_exhaustive() {
    let src = "def g (a b : Bool) : Bool := match a with\n\
               | Bool.true if b => Bool.false\n\
               | Bool.false => Bool.true\n";
    let out = compile_fol(&parse(src).expect("parse match"));
    assert_eq!(out.errors[0].code(), "elab-match-non-exhaustive");
}

#[test]
fn match_nested_arity_mismatch_reports_bad_arm() {
    let src = format!(
        "{NESTED_ENUMS}\n\
         def f (o : Outer) : Inner := match o with\n\
         | oi ia ib => ia\n\
         | on => ia\n"
    );
    let out = compile_fol(&parse(&src).expect("parse match"));
    assert_eq!(out.errors[0].code(), "elab-match-bad-arm");
}

/// 文件自带 `inductive Nat` 时 prelude 不装，源块自己登记。**语义在本刀变了**
/// （WO-005 第 5 条）：分支名仍是**源名** `zero`/`succ`（R3 不动），但内核里的
/// ctor 已是规范名 `Nat.zero`/`Nat.succ` ⇒ 归约走内核 name cache 的 NatRed 快
/// 路径，输出变成**混合表示**（`docs/architecture.md` §5.4 记过这个现象）。
/// 修前的断言是 `succ (succ (succ zero))`；新值**实测**自真实二进制。
#[test]
fn match_source_inductive_nat_reduces_through_the_nat_fast_path() {
    let src = "\
inductive Nat : Type
ctor zero : Nat
ctor succ (n : Nat) : Nat
rec Nat.rec {u} : (motive : (n : Nat) -> Sort u) -> (mz : motive zero) -> (ms : (n : Nat) -> motive n -> motive (succ n)) -> (n : Nat) -> motive n
iota zero := fun (motive : (n : Nat) -> Sort u) => fun (mz : motive zero) => fun (ms : (n : Nat) -> motive n -> motive (succ n)) => mz
iota succ := fun (motive : (n : Nat) -> Sort u) => fun (mz : motive zero) => fun (ms : (n : Nat) -> motive n -> motive (succ n)) => fun (n : Nat) => ms n (Nat.rec.{u} motive mz ms n)
end
def addS (a b : Nat) : Nat := match a with
| zero => b
| succ k => succ ih
#reduce addS (succ (succ zero)) (succ zero)
";
    let out = compile_fol(&parse(src).expect("parse match"));
    assert_eq!(out.errors, vec![], "errors: {:?}", out.errors);
    assert!(
        out.events.iter().any(
            |e| matches!(e, CheckEvent::Reduced { text, .. } if text == "Nat.succ (Nat.succ 1)")
        ),
        "source `Nat` + bare match arms must reduce through the Nat fast path: {:?}",
        out.events
    );
}

// ---- 依赖 motive（v1；design docs/design/match-dependent-motive.md）----

const DEP_PROP: &str = "\
axiom P : Nat -> Prop
axiom hz : P Nat.zero
axiom hs : (k : Nat) -> P k -> P (Nat.succ k)
";

#[test]
fn match_dependent_motive_checks_through_kernel() {
    // 结果类型 `P n` 依赖 scrutinee：motive = fun t => P t，分支期望
    // P Nat.zero / P (Nat.succ k)，递归字段的 IH : P k（依赖 IH）。
    let src = format!(
        "{DEP_PROP}\
         theorem foo (n : Nat) : P n := match n with\n\
         | Nat.zero => hz\n\
         | Nat.succ k => hs k ih\n"
    );
    let out = compile_fol(&parse(&src).expect("parse dependent match"));
    assert_eq!(out.errors, vec![], "errors: {:?}", out.errors);
    assert!(out
        .events
        .iter()
        .any(|e| matches!(e, CheckEvent::DeclarationChecked { name } if name == "foo")));
}

#[test]
fn match_dependent_motive_sees_declaration_binders() {
    // 结果类型 `P n` 里的 `P` 是**声明 binder**（不是顶层 axiom）：本文件自带
    // 归纳 Nat + 全 binder 望远镜，`infer_expected_level` 的 judge 查询必须带上
    // `P`/`n` 的书写类型，同时不能被无关的函数型 binder
    // （`hs : (k : Nat) -> P k -> P (succ k)`）的重渲染破坏。
    let src = "\
inductive Nat : Type
ctor zero : Nat
ctor succ (n : Nat) : Nat
end

theorem nat_induction (P : Nat -> Prop) (hz : P zero)
    (hs : (k : Nat) -> P k -> P (succ k)) (n : Nat) : P n :=
  match n with
  | zero => hz
  | succ k => hs k ih
";
    let out = compile_fol(&parse(src).expect("parse declaration-binder dependent match"));
    assert_eq!(out.errors, vec![], "errors: {:?}", out.errors);
    assert!(out
        .events
        .iter()
        .any(|e| matches!(e, CheckEvent::DeclarationChecked { name } if name == "nat_induction")));
}

#[test]
fn match_dependent_motive_branches_get_instantiated_expected_type() {
    // 依赖分支里的 `sorry`：洞期望 = R[x := Ctor …]，不是常量 `P n`。
    let src = format!(
        "{DEP_PROP}\
         theorem foo (n : Nat) : P n := match n with\n\
         | Nat.zero => sorry\n\
         | Nat.succ k => sorry\n"
    );
    let report = check_document(&parse(&src).expect("parse dependent match"));
    assert_eq!(report.errors, vec![], "errors: {:?}", report.errors);
    let decl = report
        .decls
        .iter()
        .find(|d| d.status == DeclStatus::Open)
        .expect("the theorem should be open");
    assert_eq!(decl.goal.as_deref(), Some("P n"));
    let tys: Vec<&str> = decl
        .sub_goals
        .iter()
        .filter_map(|sub| sub.ty.as_deref())
        .collect();
    assert!(
        tys.contains(&"P Nat.zero"),
        "zero branch expected type must be `P Nat.zero`: {tys:?}"
    );
    assert!(
        tys.contains(&"P (Nat.succ k)"),
        "succ branch expected type must be `P (Nat.succ k)`: {tys:?}"
    );
}

#[test]
fn match_dependent_ih_has_instantiated_type() {
    // 依赖 IH 的用法：`hs2` 需要 `P k`（IH）与 k；若 IH 仍是常量 `P n`
    // 则内核会拒绝。
    let src = "\
axiom P : Nat -> Prop
axiom hz : P Nat.zero
axiom hs2 : (k : Nat) -> P k -> P (Nat.succ k)
theorem bar (n : Nat) : P n := match n with
| Nat.zero => hz
| Nat.succ k => hs2 k ih
";
    let out = compile_fol(&parse(src).expect("parse dependent match"));
    assert_eq!(out.errors, vec![], "errors: {:?}", out.errors);
    assert!(out
        .events
        .iter()
        .any(|e| matches!(e, CheckEvent::DeclarationChecked { name } if name == "bar")));
}

#[test]
fn match_dependent_non_variable_scrutinee_stays_constant_motive() {
    // 非变量 scrutinee `Nat.succ n`：不触发依赖 motive，分支期望仍是常量
    // `P n`（用假设 `h` 填充两支），不报错。
    let src = format!(
        "{DEP_PROP}\
         theorem foo (n : Nat) (h : P n) : P n := match Nat.succ n with\n\
         | Nat.zero => h\n\
         | Nat.succ k => h\n"
    );
    let out = compile_fol(&parse(&src).expect("parse constant match"));
    assert_eq!(out.errors, vec![], "errors: {:?}", out.errors);
}

#[test]
fn match_dependent_non_dependent_result_regresses_to_constant_motive() {
    // 结果类型 `Nat` 不提到 scrutinee：常量 motive，与既有行为一致。
    let src = "\
def f (n : Nat) : Nat := match n with
| Nat.zero => Nat.zero
| Nat.succ k => k
";
    let out = compile_fol(&parse(src).expect("parse non-dependent match"));
    assert_eq!(out.errors, vec![], "errors: {:?}", out.errors);
}

#[test]
fn match_dependent_motive_composes_with_parameterized_inductive() {
    // 参数化归纳（Box）+ 依赖结果：motive 域 = `Box Nat`，分支构造子项
    // = `box Nat a`，期望 `P (box Nat a)`。
    let src = "\
inductive Box (A : Type) : Type
ctor box (a : A) : Box A
end
axiom P : Box Nat -> Prop
axiom h : (a : Nat) -> P (box Nat a)
theorem unbox (b : Box Nat) : P b := match b with
| box a => h a
";
    let out = compile_fol(&parse(src).expect("parse dependent parameterized match"));
    assert_eq!(out.errors, vec![], "errors: {:?}", out.errors);
    assert!(out
        .events
        .iter()
        .any(|e| matches!(e, CheckEvent::DeclarationChecked { name } if name == "unbox")));
}

// ---- 参数化归纳（非带索引，v1；design docs/design/parameterized-inductives.md）----

const OPTION_ENUM: &str = "\
inductive Option (A : Type) : Type
ctor none : Option A
ctor some (a : A) : Option A
end
";

#[test]
fn parameterized_option_checks_and_derives_recursor() {
    // 派生递归子 `Option.rec.{u}` 由前端合成，经完整内核判定（含 def_eq 重建
    // 断言）；参数在内核望远镜最外层。
    let src = format!(
        "{OPTION_ENUM}\n\
         #check none Nat\n\
         #check some Nat\n\
         #check Option.rec\n"
    );
    let out = compile_fol(&parse(&src).expect("parse Option"));
    assert_eq!(out.errors, vec![], "errors: {:?}", out.errors);
    let texts: Vec<&str> = out
        .events
        .iter()
        .filter_map(|e| match e {
            CheckEvent::TypeChecked { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert!(
        texts.iter().any(|t| t == &"Option Nat"),
        "`none Nat : Option Nat`: {texts:?}"
    );
    assert!(
        texts.iter().any(|t| t == &"Nat -> Option Nat"),
        "`some Nat : Nat -> Option Nat`: {texts:?}"
    );
    assert!(
        texts.iter().any(|t| t.contains("motive")),
        "`Option.rec` renders its derived type: {texts:?}"
    );
}

#[test]
fn parameterized_explicit_rec_and_iota_pass_kernel() {
    // 显式单参数 rec/iota：params 最外层，经内核 def_eq 重建断言。
    let src = "\
inductive Option (A : Type) : Type
ctor none : Option A
ctor some (a : A) : Option A
rec Option.rec {u} :
  (A : Type) ->
  (motive : (t : Option A) -> Sort u) ->
  (m_none : motive (none A)) ->
  (m_some : (a : A) -> motive (some A a)) ->
  (t : Option A) -> motive t
iota none := fun (A : Type) => fun (motive : (t : Option A) -> Sort u) => fun (m_none : motive (none A)) => fun (m_some : (a : A) -> motive (some A a)) => m_none
iota some := fun (A : Type) => fun (motive : (t : Option A) -> Sort u) => fun (m_none : motive (none A)) => fun (m_some : (a : A) -> motive (some A a)) => fun (a : A) => m_some a
end
";
    let out = compile_fol(&parse(src).expect("parse explicit rec"));
    assert_eq!(out.errors, vec![], "errors: {:?}", out.errors);
}

#[test]
fn parameterized_explicit_iota_wrong_value_is_rejected() {
    // `iota none` 返回了 `m_some`（形状不符）⇒ 内核规则重建断言拒绝。
    let src = "\
inductive Option (A : Type) : Type
ctor none : Option A
ctor some (a : A) : Option A
rec Option.rec {u} :
  (A : Type) ->
  (motive : (t : Option A) -> Sort u) ->
  (m_none : motive (none A)) ->
  (m_some : (a : A) -> motive (some A a)) ->
  (t : Option A) -> motive t
iota none := fun (A : Type) => fun (motive : (t : Option A) -> Sort u) => fun (m_none : motive (none A)) => fun (m_some : (a : A) -> motive (some A a)) => m_some
iota some := fun (A : Type) => fun (motive : (t : Option A) -> Sort u) => fun (m_none : motive (none A)) => fun (m_some : (a : A) -> motive (some A a)) => fun (a : A) => m_some a
end
";
    let out = compile_fol(&parse(src).expect("parse explicit rec"));
    assert_eq!(out.errors[0].code(), "kernel-rec-rule-mismatch");
}

#[test]
fn match_on_parameterized_option_instantiates_field_type() {
    // `some a` 的 `a : A` 被 scrutinee 的书写参数 `Nat` 代入；分支体 `a` 因此
    // 具有类型 Nat，且归约经派生的 `Option.rec`。
    let src = format!(
        "{OPTION_ENUM}\n\
         def orElse (x : Option Nat) (d : Nat) : Nat := match x with\n\
         | none => d\n\
         | some a => a\n\
         #reduce orElse (some Nat (Nat.succ Nat.zero)) Nat.zero\n"
    );
    let out = compile_fol(&parse(&src).expect("parse match"));
    assert_eq!(out.errors, vec![], "errors: {:?}", out.errors);
    assert!(out
        .events
        .iter()
        .any(|e| matches!(e, CheckEvent::DeclarationChecked { name } if name == "orElse")));
    assert!(
        out.events.iter().any(
            |e| matches!(e, CheckEvent::Reduced { text, .. } if text.contains('1') || text.contains("succ"))
        ),
        "`orElse (some Nat 1) 0` must reduce through `Option.rec`: {:?}",
        out.events
    );
}

#[test]
fn match_parameterized_without_written_params_reports_code() {
    // scrutinee 是顶层常量（不在局部 scope），拿不到书写参数实例 → 干净报错。
    let src = format!(
        "{OPTION_ENUM}\n\
         def opt : Option Nat := none Nat\n\
         def f : Nat := match opt with\n\
         | none => Nat.zero\n\
         | some a => a\n"
    );
    let out = compile_fol(&parse(&src).expect("parse match"));
    assert_eq!(out.errors[0].code(), "elab-match-parameterized-unsupported");
}

#[test]
fn parameterized_recursive_list_derives_recursor_and_matches() {
    // 递归 + 参数：派生递归子的自调用必须带上 params；`match` 的递归字段
    // 自动获得归纳假设 `ih`。
    let src = "\
inductive List (A : Type) : Type
ctor nil : List A
ctor cons (a : A) (as : List A) : List A
end
def len (l : List Nat) : Nat := match l with
| nil => Nat.zero
| cons a as => Nat.succ ih
#reduce len (cons Nat Nat.zero (nil Nat))
";
    let out = compile_fol(&parse(src).expect("parse List"));
    assert_eq!(out.errors, vec![], "errors: {:?}", out.errors);
    assert!(out
        .events
        .iter()
        .any(|e| matches!(e, CheckEvent::DeclarationChecked { name } if name == "len")));
    assert!(
        out.events
            .iter()
            .any(|e| matches!(e, CheckEvent::Reduced { text, .. } if text.contains("succ"))),
        "`len [0]` must reduce through the parameterized `List.rec`: {:?}",
        out.events
    );
}

#[test]
fn match_dependent_motive_with_function_typed_binder_round_trips_safely() {
    // judge_infer 的 render→parse 往返健壮性：结果类型 `Q hs n` 引用「类型为
    // 依赖函数」的 binder `hs`，Arrow domain 位的 Forall 必须补括号，否则
    // 望远镜被腐蚀 → level 查询失败。此前该形状报 `elab-match-no-expected-type`。
    let src = "\
inductive Nat : Type
ctor zero : Nat
ctor succ (n : Nat) : Nat
end
axiom P : Nat -> Prop
axiom Q : ((k : Nat) -> P k -> P (succ k)) -> Nat -> Prop
theorem t (hs : (k : Nat) -> P k -> P (succ k)) (hz : Q hs zero)
    (hstep : (k : Nat) -> Q hs k -> Q hs (succ k)) (n : Nat) : Q hs n :=
  match n with
  | zero => hz
  | succ k => hstep k ih
";
    let out = compile_fol(&parse(src).expect("parse"));
    assert!(out.errors.is_empty(), "{:?}", out.errors);
    assert!(out
        .events
        .iter()
        .any(|e| matches!(e, CheckEvent::DeclarationChecked { name } if name == "t")));
}

// ---- 带索引归纳（v1；design docs/design/indexed-inductives.md）----

/// `Vec (A : Type) : Nat -> Type`（1 参数 + 1 索引）。
const INDEXED_VEC: &str = "\
inductive Vec (A : Type) : Nat -> Type
ctor vnil : Vec A 0
ctor vcons (a : A) (n : Nat) (v : Vec A n) : Vec A (Nat.succ n)
end
";

#[test]
fn indexed_vec_checks_and_derives_recursor() {
    let src = format!("{INDEXED_VEC}#check Vec\n#check Vec.rec\n#check vcons\n");
    let out = compile_fol(&parse(&src).expect("parse indexed"));
    assert_eq!(out.errors, vec![], "errors: {:?}", out.errors);
    assert!(out
        .events
        .iter()
        .any(|e| matches!(e, CheckEvent::DeclarationChecked { name } if name == "Vec")));
    let types: Vec<&str> = out
        .events
        .iter()
        .filter_map(|e| match e {
            CheckEvent::TypeChecked { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    // `Vec : Type -> Nat -> Type`（参数 + 索引都在类型里）。
    assert!(
        types
            .iter()
            .any(|t| t.contains("Nat") && t.contains("Type")),
        "Vec's type must carry its index: {types:?}"
    );
    assert!(
        types
            .iter()
            .any(|t| t.contains("motive") && t.contains("vnil") && t.contains("vcons")),
        "Vec.rec must mention both constructors: {types:?}"
    );
}

// ---- derive_recursor 的箭头写法字段（docs/design/agent-query-channel.md H6-C / TODO A）----

/// 索引归纳的字段**写在结果的箭头链里**（`ctor b (n : Nat) : P n -> P (succ n)`）
/// 时，派生出的 recursor 的 minor 结论曾丢掉索引实参，被内核拒。
/// 对照：同一形状改成**具名字段** `(n : Nat) (h : P n)` 一直是通过的
/// —— 两者只差 `ctor_indices`（`derive_recursor` 用了只认 Ident/App 的 `src_spine`
/// 去读箭头链的头部）。
const INDEXED_ARROW_FIELD: &str = "\
inductive P : Nat -> Prop
ctor pz : P 0
ctor ps (n : Nat) : P n -> P (Nat.succ n)
end
";

#[test]
fn indexed_inductive_with_arrow_style_field_derives_recursor() {
    let src = format!("{INDEXED_ARROW_FIELD}#check P.rec\n");
    let out = compile_fol(&parse(&src).expect("parse indexed arrow form"));
    assert_eq!(
        out.errors,
        vec![],
        "arrow-style indexed fields must derive a kernel-accepted recursor: {:?}",
        out.errors
    );
    assert!(
        out.events
            .iter()
            .any(|e| matches!(e, CheckEvent::DeclarationChecked { name } if name == "P")),
        "the inductive itself must be checked"
    );
    let types: Vec<&str> = out
        .events
        .iter()
        .filter_map(|e| match e {
            CheckEvent::TypeChecked { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert!(
        types
            .iter()
            .any(|t| t.contains("motive") && t.contains("pz")),
        "P.rec must mention the constructors: {types:?}"
    );
}

/// 同一形状的**具名**字段孪生体：这是修复前就已经通过的对照，用来钉住
/// "两种写法必须等价"。
#[test]
fn indexed_inductive_with_named_field_derives_recursor() {
    let src = "\
inductive P2 : Nat -> Prop
ctor p2z : P2 0
ctor p2s (n : Nat) (h : P2 n) : P2 (Nat.succ n)
end
#check P2.rec
";
    let out = compile_fol(&parse(src).expect("parse indexed named form"));
    assert_eq!(out.errors, vec![], "errors: {:?}", out.errors);
}

/// iota 规则也要跟着对：派生的 recursor 必须真的能**算**（不只类型对）。
///
/// 这里必须是 **`Type` 值**的索引族：`Prop` 值 + 多构造子的归纳在内核里只允许
/// 消去到 `Prop`（singleton elimination），所以匹配到 `Nat` 会被内核正当地拒
/// （`期望 Pi (i : Nat), Pi (x : P i), Sort(0)`）——用 `Prop` 族写这个测试会
/// 误把"正确的拒绝"当成回归。字段故意写成**箭头链**（`W A n -> W A (succ n)`），
/// 这正是修（`src_spine` → `spine_of_codomain`）之前丢掉索引实参的形状。
#[test]
fn arrow_style_indexed_recursor_reduces() {
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
    let out = compile_fol(&parse(src).expect("parse"));
    assert_eq!(out.errors, vec![], "errors: {:?}", out.errors);
    assert!(
        out.events.iter().any(|e| matches!(
            e,
            CheckEvent::Reduced { text, .. } if text == "Nat.succ (Nat.succ 0)"
        )),
        "the derived recursor must compute on an arrow-style indexed field: {:?}",
        out.events
    );
}

// ── 派生 recursor 的 large-elimination 判据（G-03 / WO-006）────────────────────
//
// 前端曾用 `is_prop_block_ty(ty) && constructors.len() > 1` 近似内核的
// `large_elim_test`（`crates/kernel/src/inductive.rs`）。两者只在"单构造子 Prop
// 且该构造子确实 large-eliminate"时巧合一致；不一致时前端声明的 recursor
// 宇宙参数个数 ≠ 内核算出的 `st.rec_uparams` 个数，内核在
// `subst_expr_levels`（`expr.rs:381-394`）的 `assert_eq!` 上炸成 panic 载荷
// `left: 1 / right: 0` —— 学习者看到的是内部断言，没有任何可行动的提示。
//
// 修法是**问内核**（设计 `docs/design/prop-large-elim-mirror.md` §3）：把块临时
// 交给内核探一次，从拒绝载荷里读出它自己算的个数，再按答案派生。
//
// 判据是**判别性**的，不许用近似替换（WO-006 的 P1–P14 对拍表）：
//   · 不能按"字段类型语法上是不是 Prop"——P10 `P -> Q`、P11 `forall (x : Nat), P`、
//     P13 `Named`、P14 `Rel 0` 都**是** Prop 值，近似会把它们从 1 个宇宙参数改成
//     0 个，换成镜像方向的 `left:0/right:1`；
//   · 不能按"字段数 vs 参数数"或"有没有索引"——P3 与 P9 是一对判别性形状。

/// 复现件形状（`docs/gaps/repro/G03-prop-type-param-inductive.sokonanoda:29-31`）：
/// `Prop` 结果 + `Type` 参数 + **恰好一个**构造子 + 构造子自有字段。
///
/// 内核对它算 `large_elim_test_aux == false`（自有字段 `a` 不是 Prop 值、也不在
/// 结果实参 `[A]` 里）⇒ `rec_uparams` 为空 ⇒ 派生的 recursor **不许**带宇宙参数。
#[test]
fn prop_type_param_single_ctor_derives_a_zero_universe_recursor() {
    let src = "\
inductive Bar (A : Type) : Prop
ctor mk (a : A) : Bar A
end
#check Bar.rec
";
    let out = compile_fol(&parse(src).expect("parse G-03 shape"));
    assert_eq!(out.errors, vec![], "errors: {:?}", out.errors);
    assert!(
        out.events
            .iter()
            .any(|e| matches!(e, CheckEvent::DeclarationChecked { name } if name == "Bar")),
        "the G-03 block must be checked: {:?}",
        out.events
    );
    // 形状断言（关键）：只断 `decl.checked` 不够——本缺口的本质是 recursor 的
    // **形状**错了。`#check Bar.rec` 的类型里不许出现宇宙参数，motive 必须落在
    // `Prop`（内核对这个块给的就是 0 级 recursor）。
    let types: Vec<&str> = out
        .events
        .iter()
        .filter_map(|e| match e {
            CheckEvent::TypeChecked { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    let rec_ty = types
        .iter()
        .find(|t| t.contains("motive") && t.contains("Bar"))
        .unwrap_or_else(|| panic!("Bar.rec must be type-checked: {types:?}"));
    assert!(
        !rec_ty.contains("{u}") && !rec_ty.contains("Sort u"),
        "a non-large-eliminating Prop block must derive a recursor with no universe parameter: {rec_ty}"
    );
    assert!(
        rec_ty.contains("-> Prop"),
        "the derived motive must land in Prop: {rec_ty}"
    );
}

/// **表驱动对拍**：WO-006 的整份语料（`Named`/`Rel` + P1–P14，共 16 条声明）。
///
/// 修前 13 checked / 3 failed（失败恰为 P1/P2/P3），修后必须 16/16。
/// 每一条都是内核判据的一侧边界，别删条目——判别性就在这里。
#[test]
fn derived_recursor_universe_mirrors_the_kernel_large_elim_test() {
    let src = "\
def Named : Prop := forall (x : Nat), forall (y : Nat), Eq.{1} Nat x y
def Rel (n : Nat) : Prop := forall (m : Nat), Eq.{1} Nat m n
inductive P1 (A : Type) : Prop
ctor c1 (a : A) : P1 A
end
inductive P2 (A : Type) : Prop
ctor c2 (a : A) (b : A) : P2 A
end
inductive P3 (A : Type) : A -> Prop
ctor c3 (a : A) (b : A) : P3 A a
end
inductive P4 (A : Type) : Prop
ctor c4 : P4 A
end
inductive P5 (P : Prop) : Prop
ctor c5 (p : P) : P5 P
end
inductive P6 (A : Type) (P : A -> Prop) : Type
ctor c6 (a : A) (h : P a) : P6 A P
end
inductive P7 (A : Type) : Prop
ctor c7a (a : A) : P7 A
ctor c7b : P7 A
end
inductive P8 (n : Nat) : Prop
ctor c8 : P8 n
end
inductive P9 (A : Type) : A -> Prop
ctor c9 (a : A) : P9 A a
end
inductive P10 (A : Type) (P Q : Prop) : Prop
ctor c10 (h : P -> Q) : P10 A P Q
end
inductive P11 (A : Type) (P : Prop) : Prop
ctor c11 (f : forall (x : Nat), P) : P11 A P
end
inductive P12 (A : Type) : Prop
end
inductive P13 (A : Type) : Prop
ctor c13 (h : Named) : P13 A
end
inductive P14 (A : Type) : Prop
ctor c14 (h : Rel 0) : P14 A
end
";
    let out = compile_fol(&parse(src).expect("parse the WO-006 corpus"));
    assert_eq!(
        out.errors,
        vec![],
        "the whole corpus must mirror the kernel: {:?}",
        out.errors
    );
    let checked: Vec<&str> = out
        .events
        .iter()
        .filter_map(|e| match e {
            CheckEvent::DeclarationChecked { name } => Some(name.as_str()),
            _ => None,
        })
        .collect();
    for name in [
        "Named", "Rel", "P1", "P2", "P3", "P4", "P5", "P6", "P7", "P8", "P9", "P10", "P11", "P12",
        "P13", "P14",
    ] {
        assert!(
            checked.contains(&name),
            "`{name}` must be checked (16/16 after the fix): {checked:?}"
        );
    }
}

/// 判别性反例，**成对**点名（`docs/architecture.md` §8 gotcha 0b）：
///
/// * **P3 vs P9** —— 两条都是"`A -> Prop` + 一个字段"，只差字段**是不是结果的
///   索引**。内核 `large_elim_test_aux` 的判据是"非 Prop 字段必须是
///   `params ++ indices` 的**语法**成员"：P9 的 `a` 就是索引 `a`（⇒ 可 large
///   eliminate ⇒ 1 个宇宙参数），P3 的 `b` 不在 `[A, a]` 里（⇒ 0 个）。
///   按"看起来像同一类"的近似写就会让其中一条翻面。
/// * **P10/P11/P13/P14** —— 字段类型分别是 `P -> Q`、`forall (x : Nat), P`、
///   具名 `def … : Prop`、具名 Prop 定义的应用，**都**是 Prop 值（⇒ 1 个宇宙
///   参数）。按"字段类型语法上是不是 `Prop`"近似就会把它们全部翻成 0 个。
///
/// 两组合起来钉住：判据只能来自内核本身，不能来自源码形状。
#[test]
fn discriminative_pairs_pin_the_large_elim_test_to_the_kernel() {
    // P3 / P9：索引成员判据（语法，不展开定义）。两者形状只差一个字段是否
    // 出现在结果实参里，所以 recursor 的**宇宙参数个数**是唯一判别式：
    // `#check` 会把 `u` 实例化成 0 并打印成 `Prop`，文本比对分辨不出来，
    // 于是用 `.{0}` 这个显式宇宙应用去问内核真实 arity。
    let p3p9 = "\
inductive P3 (A : Type) : A -> Prop
ctor c3 (a : A) (b : A) : P3 A a
end
inductive P9 (A : Type) : A -> Prop
ctor c9 (a : A) : P9 A a
end
#check P3.rec.{0}
#check P9.rec.{0}
";
    let out = compile_fol(&parse(p3p9).expect("parse P3/P9"));
    // P3：`b` 既不是参数也不是索引 ⇒ 不 large eliminate ⇒ recursor 有 **0** 个
    // 宇宙参数 ⇒ `P3.rec.{0}` 是 arity 错误。
    assert!(
        out.errors
            .iter()
            .any(|e| e.code() == "elab-universe-arity" && e.message.contains("P3.rec")),
        "P3's non-index field `b` must keep the recursor at Prop (0 universe params): {:?}",
        out.errors
    );
    // P9：`a` **就是**索引 ⇒ large eliminate ⇒ **1** 个宇宙参数 ⇒ `.{0}` 合法。
    assert!(
        !out.errors.iter().any(|e| e.message.contains("P9.rec")),
        "P9's field `a` is the index, so its recursor is universe-polymorphic: {:?}",
        out.errors
    );
    assert!(
        out.events.iter().any(|e| matches!(
            e,
            CheckEvent::TypeChecked { text, .. } if text.contains("P9")
        )),
        "P9.rec.{{0}} must be type-checked: {:?}",
        out.events
    );

    // P10/P11/P13/P14：字段类型都是 Prop **值** ⇒ 都 large eliminate ⇒ 每个
    // recursor 都带 1 个宇宙参数，`.{0}` 全部合法。
    let prop_fields = "\
def Named : Prop := forall (x : Nat), forall (y : Nat), Eq.{1} Nat x y
def Rel (n : Nat) : Prop := forall (m : Nat), Eq.{1} Nat m n
inductive P10 (A : Type) (P Q : Prop) : Prop
ctor c10 (h : P -> Q) : P10 A P Q
end
inductive P11 (A : Type) (P : Prop) : Prop
ctor c11 (f : forall (x : Nat), P) : P11 A P
end
inductive P13 (A : Type) : Prop
ctor c13 (h : Named) : P13 A
end
inductive P14 (A : Type) : Prop
ctor c14 (h : Rel 0) : P14 A
end
#check P10.rec.{0}
#check P11.rec.{0}
#check P13.rec.{0}
#check P14.rec.{0}
";
    let out = compile_fol(&parse(prop_fields).expect("parse Prop-field shapes"));
    assert_eq!(
        out.errors,
        vec![],
        "Prop-typed fields must keep large elimination: {:?}",
        out.errors
    );
    for ind in ["P10", "P11", "P13", "P14"] {
        assert!(
            out.events.iter().any(|e| matches!(
                e,
                CheckEvent::TypeChecked { text, .. } if text.contains(ind)
            )),
            "{ind}.rec.{{0}} must be type-checked: {:?}",
            out.events
        );
    }
}

/// 修好后 G-03 形状走**0 级 recursor 分支**（`elab.rs` 的
/// `rec_universe_arity == 0`）：`match` 消去到 `Prop` 必须真的算出来。
///
/// 这条守住"派生 recursor 不只是声明得过"——0 级常量与 iota 规则都得对。
///
/// 注意结果类型写成 `Bar A`（一个 **Prop 值**）而不是裸 `Prop`：motive 的
/// codomain 必须是 `Sort 0`，写 `Prop` 会要求 motive 落在 `Sort 1`，那与
/// large-elimination 无关，是另一件事（既有行为，基线同样拒绝）。
#[test]
fn zero_universe_prop_recursor_computes_through_match() {
    let src = "\
inductive Bar (A : Type) : Prop
ctor mk (a : A) : Bar A
end
def bar_id (A : Type) (b : Bar A) : Bar A :=
  match b with
  | mk a => b
";
    let out = compile_fol(&parse(src).expect("parse G-03 match"));
    assert_eq!(
        out.errors,
        vec![],
        "a Prop-motived match on the G-03 shape must compile: {:?}",
        out.errors
    );
    assert!(
        out.events
            .iter()
            .any(|e| matches!(e, CheckEvent::DeclarationChecked { name } if name == "bar_id")),
        "the match-based elimination must be checked: {:?}",
        out.events
    );
}

/// **单构造子 `Prop`** 的 recursor 曾被内核拒：
/// `recursor declares the wrong k-reduction flag (left: false, right: true)` ——
/// `install_inductive_block` 把 `is_k` 写死成 `false`（elab.rs:471）。
#[test]
fn single_constructor_prop_derives_recursor() {
    let src = "\
inductive True2 : Prop
ctor trivial2 : True2
end
#check True2.rec
";
    let out = compile_fol(&parse(src).expect("parse single-ctor prop"));
    assert_eq!(
        out.errors,
        vec![],
        "a single-constructor Prop needs the K flag set: {:?}",
        out.errors
    );
    assert!(out
        .events
        .iter()
        .any(|e| matches!(e, CheckEvent::DeclarationChecked { name } if name == "True2")));
}

/// **K 目标判据必须逐字镜像内核，不能按"像不像单例"猜**。
///
/// 内核的判据是 `pi_telescope_size(ctor.ty) == local_params.len()`
/// （`crates/kernel/src/inductive.rs:1268-1276`）加上 `Prop` 与"只有这一个归纳"；
/// 而构造子的内核类型是 `forall (params ++ fields), result`，所以那个等式 ⟺
/// **构造子没有自己的字段**（结果箭头链上的字段也算字段）。
///
/// 这里 `mk` 的**字段数恰好等于参数数**（2 = 2）：按"字段数 vs 参数数"比对就会
/// 误判成 K 目标，内核立刻以 `recursor declares the wrong k-reduction flag`
/// 拒掉整个块。这条是 H6-C 的 `is_k` 修复自己引入的回归，被 CLI e2e 层抓住
/// （`crates/cli/tests/cli.rs::cli_inductive_accepts_multi_name_binder_groups`）。
#[test]
fn single_constructor_prop_with_fields_is_not_a_k_target() {
    let src = "\
inductive Both (A B : Prop) : Prop
ctor mk (a : A) (b : B) : Both A B
end
#check Both.rec
";
    let out = compile_fol(&parse(src).expect("parse single-ctor prop with fields"));
    assert_eq!(
        out.errors,
        vec![],
        "a ctor with fields is not a K target, even when field count == param count: {:?}",
        out.errors
    );
    assert!(out
        .events
        .iter()
        .any(|e| matches!(e, CheckEvent::DeclarationChecked { name } if name == "Both")));
}

/// 反向边界：**带索引**但单构造子、且构造子没有自己的字段（索引是常量
/// `Q 0`）时，内核算出的 `is_k` 是 **`true`**（`pi_telescope_size = 0` 等于
/// 参数个数 `0`）。曾按"有索引 ⇒ 不是 K 目标"多判了一层，同样被内核拒。
#[test]
fn single_constructor_indexed_prop_without_fields_is_a_k_target() {
    let src = "\
inductive Q : Nat -> Prop
ctor q : Q 0
end
#check Q.rec
";
    let out = compile_fol(&parse(src).expect("parse constant-index prop"));
    assert_eq!(out.errors, vec![], "errors: {:?}", out.errors);
    assert!(out
        .events
        .iter()
        .any(|e| matches!(e, CheckEvent::DeclarationChecked { name } if name == "Q")));
}

#[test]
fn match_on_indexed_vec_computes_with_a_constant_motive() {
    let src = format!(
        "{INDEXED_VEC}\
         def vlen (A : Type) (n : Nat) (v : Vec A n) : Nat :=\n\
           match v with\n\
           | vnil => 0\n\
           | vcons a m w => Nat.succ ih\n\
         #reduce vlen Nat 0 (vnil Nat)\n\
         #reduce vlen Nat 2 (vcons Nat 1 1 (vcons Nat 2 0 (vnil Nat)))\n"
    );
    let out = compile_fol(&parse(&src).expect("parse match"));
    assert_eq!(out.errors, vec![], "errors: {:?}", out.errors);
    let reduced: Vec<&str> = out
        .events
        .iter()
        .filter_map(|e| match e {
            CheckEvent::Reduced { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(
        reduced,
        vec!["0", "Nat.succ (Nat.succ 0)"],
        "{:?}",
        out.events
    );
}

#[test]
fn match_field_types_follow_the_user_binder_names() {
    // 字段类型引用前面的字段（`v : Vec A n`），而用户改名为 `w`/`m`：
    // 结果类型里对 `n` 的引用必须跟着改名，否则 de Bruijn 指错。
    let src = format!(
        "{INDEXED_VEC}\
         def head_or (A : Type) (d : A) (n : Nat) (v : Vec A n) : A :=\n\
           match v with\n\
           | vnil => d\n\
           | vcons a m w => a\n"
    );
    let out = compile_fol(&parse(&src).expect("parse match"));
    assert_eq!(out.errors, vec![], "errors: {:?}", out.errors);
    assert!(out
        .events
        .iter()
        .any(|e| matches!(e, CheckEvent::DeclarationChecked { name } if name == "head_or")));
}

// ── 多余的 `sorry`（用户实测反馈，`docs/design/redundant-sorry.md`）────────────
//
// 学生把答案写在 `:=` 右边、却保留了原来那行 `sorry` 时，整条声明被解析成
// `项 sorry`——洞变成了多出来的实参。它**不是**"还没证明出来"：前面的项往往
// 已经完成了证明。判定 = sound 候选（实参超出望远镜且结果展不开箭头）
// + kernel 终审（删掉该实参后整条声明必须能过）。

/// `f h` 已经证明了目标，后面多留了一行 `sorry`（用户 playground 326–328 的形状）。
#[test]
fn a_leftover_sorry_after_a_complete_term_is_reported_as_redundant() {
    let src = "axiom A : Prop\n\
               axiom B : Prop\n\
               axiom f : A -> B\n\
               theorem t (h : A) : B := f h\n\
               \x20 sorry\n";
    let out = compile_fol(&parse(src).unwrap());
    assert_eq!(out.errors, vec![], "warning must not be an error");

    let codes: Vec<&str> = out.warnings.iter().map(|w| w.code()).collect();
    assert!(
        codes.contains(&"redundant-sorry"),
        "多出来的 sorry 必须被单独指出来（否则学生以为是自己没证出来）：{codes:?}"
    );
    let w = out
        .warnings
        .iter()
        .find(|w| w.code() == "redundant-sorry")
        .expect("redundant-sorry warning");
    assert_eq!(
        &src[w.span.start.offset..w.span.end.offset],
        "sorry",
        "warning span 收窄到那个 sorry token"
    );
    assert!(!w.hint().is_empty());

    // 语义不变：它仍然是一条开放练习（练习状态与洞的既有行为都不动）。
    assert!(
        out.events
            .iter()
            .any(|e| matches!(e, CheckEvent::ExerciseOpen { name: Some(n) } if n == "t")),
        "exercise.open 语义不得改变：{:?}",
        out.events
    );
}

/// λ 体里同样的形状（`fun (h : A) => f h` 后又接了一个 sorry）。
#[test]
fn a_leftover_sorry_in_a_lambda_body_is_reported_as_redundant() {
    let src = "axiom A : Prop\n\
               axiom B : Prop\n\
               axiom f : A -> B\n\
               theorem t : A -> B := fun (h : A) => f h\n\
               \x20 sorry\n";
    let out = compile_fol(&parse(src).unwrap());
    assert_eq!(out.errors, vec![]);
    assert!(
        out.warnings.iter().any(|w| w.code() == "redundant-sorry"),
        "{:?}",
        out.warnings
    );
}

/// 真缺口：`f` 需要一个 `A`，洞就是缺的那块——**不得**被说成多余。
#[test]
fn a_genuine_argument_hole_is_not_reported_as_redundant() {
    let src = "axiom A : Prop\n\
               axiom B : Prop\n\
               axiom f : A -> B\n\
               theorem t (h : A) : B := f\n\
               \x20 sorry\n";
    let out = compile_fol(&parse(src).unwrap());
    assert_eq!(out.errors, vec![]);
    let codes: Vec<&str> = out.warnings.iter().map(|w| w.code()).collect();
    assert!(
        !codes.contains(&"redundant-sorry"),
        "真缺的实参不能被误报成多余：{codes:?}"
    );
}

/// 真缺口：第二个证明还没写（`And.intro A B ha` 之后缺 `B` 的证明）。
#[test]
fn a_genuine_second_proof_hole_is_not_reported_as_redundant() {
    let src = "axiom A : Prop\n\
               axiom B : Prop\n\
               axiom ha : A\n\
               axiom And : Prop -> Prop -> Prop\n\
               axiom And.intro : (a b : Prop) -> a -> b -> And a b\n\
               theorem t : And A B := And.intro A B ha\n\
               \x20 sorry\n";
    let out = compile_fol(&parse(src).unwrap());
    assert_eq!(out.errors, vec![]);
    let codes: Vec<&str> = out.warnings.iter().map(|w| w.code()).collect();
    assert!(
        !codes.contains(&"redundant-sorry"),
        "第二项证明是缺的，不能被误报成多余：{codes:?}"
    );
}

/// kernel 终审的护栏：项本身**不是**目标类型时，删掉 sorry 也过不了，
/// 因此不许说"多余的 sorry"（保守，绝不误报）。
#[test]
fn a_leftover_sorry_whose_term_does_not_prove_the_goal_is_not_reported() {
    let src = "axiom A : Prop\n\
               axiom B : Prop\n\
               axiom C : Prop\n\
               axiom g : A -> C\n\
               theorem t (h : A) : B := g h\n\
               \x20 sorry\n";
    let out = compile_fol(&parse(src).unwrap());
    assert_eq!(out.errors, vec![]);
    let codes: Vec<&str> = out.warnings.iter().map(|w| w.code()).collect();
    assert!(
        !codes.contains(&"redundant-sorry"),
        "删掉 sorry 之后过不了内核，就不能说它多余：{codes:?}"
    );
}

/// **可见前缀护栏**（`docs/design/redundant-sorry.md` §8.3）：与上面的正例
/// 逐字同形，只把 `g` 挪到练习**后面**声明。探针的终审用
/// `EnvLimit::ByIndex(env_before)`，前瞻引用照样看不见 ⇒ 删掉 sorry 还是过不了
/// 内核（那条声明本体就是前瞻引用）⇒ **不得**说它多余。
/// 谁哪天把限界放宽成"整份环境可见"，这条会红。
#[test]
fn a_leftover_sorry_with_a_forward_reference_is_not_reported_as_redundant() {
    let src = "axiom A : Prop\n\
               axiom B : Prop\n\
               theorem t (h : A) : B := g h\n\
               \x20 sorry\n\
               axiom g : A -> B\n";
    let out = compile_fol(&parse(src).unwrap());
    let codes: Vec<&str> = out.warnings.iter().map(|w| w.code()).collect();
    assert!(
        !codes.contains(&"redundant-sorry"),
        "前瞻引用不在可见前缀里，终审必须失败：{codes:?}"
    );
}

/// **跨单元归因**（0.58.0 合并轮）：`redundant-sorry` 是 pass 2 现算的、
/// 带**命令下标**的 warning；`split_report` 靠 `warning_cmds` 把它放回产生它的
/// 模块。丢掉归因（例如只按单元重算语法级警告）时，项目入口"多写了一行 sorry"
/// 会静默消失——这条测试钉住两个方向：内核终审的归**产生它的单元**，
/// 语法级的归**它所在的单元**（同单元内语法级在前、内核终审在后）。
#[test]
fn warnings_are_attributed_to_the_unit_that_produced_them() {
    let dep_src = "axiom Prop : Prop\n\
                   axiom A : Prop\n\
                   axiom B : Prop\n\
                   axiom f : A -> B\n\
                   theorem t (h : A) : B := f h\n\
                   \x20 sorry\n";
    let entry_src = "import Dep\n\
                     \n\
                     theorem u (h : A) : B := f h\n\
                     \x20 sorry\n";
    let dep = parse(dep_src).unwrap();
    let entry = parse(entry_src).unwrap();
    let units = [
        SourceUnit::single("Dep", &dep),
        SourceUnit::single("Main", &entry),
    ];
    let (out, reports) = compile_all_units(&units, &CompileOptions::default());
    assert_eq!(out.errors, vec![], "两条声明都该是开放练习，不是错误");
    assert_eq!(
        out.warnings.len(),
        3,
        "依赖 = 语法级 1 + 内核终审 1，入口 = 内核终审 1：{:?}",
        out.warnings
    );
    assert_eq!(reports.len(), 2);

    // 依赖单元：语法级（`axiom Prop`）在前、内核终审（多余的 sorry）在后。
    let dep_codes: Vec<&str> = reports[0].warnings.iter().map(|w| w.code()).collect();
    assert_eq!(
        dep_codes,
        vec!["reserved-declaration-name", "redundant-sorry"],
        "依赖单元的警告必须属于依赖：{dep_codes:?}"
    );
    assert_eq!(
        &dep_src[reports[0].warnings[1].span.start.offset..reports[0].warnings[1].span.end.offset],
        "sorry",
        "span 是依赖文件的坐标"
    );

    // 入口单元：只有它自己的那一条，且 span 落在入口文件里。
    let entry_codes: Vec<&str> = reports[1].warnings.iter().map(|w| w.code()).collect();
    assert_eq!(entry_codes, vec!["redundant-sorry"], "{entry_codes:?}");
    assert_eq!(
        &entry_src
            [reports[1].warnings[0].span.start.offset..reports[1].warnings[0].span.end.offset],
        "sorry",
        "入口的 sorry 属于入口文件"
    );

    // 与扁平输出一致（同一份真相，`warning_cmds` 严格平行）。
    assert_eq!(out.warning_cmds.len(), out.warnings.len());
    assert!(
        out.warning_cmds[0] < dep.commands.len(),
        "第一条（依赖的语法级警告）属于依赖的命令区间"
    );
}

// ── G-01 / WO-004：开练习的签名必须过 elaborate ────────────────────────────
//
// 值位是 `sorry` 不再让签名免检：签名 elaborate 不了、签名不是一个类型、
// 或 `theorem` 的签名不是 Prop，都必须与「还没做」区分开——报一条诊断、
// 不再发 `exercise.open`。判定全部走内核（探针不入环境）；
// 合法开放练习（签名本来就过得去）一个字节都不变。

/// 开练习的签名不是类型（`3 : Nat`）⇒ `kernel-expected-sort`，
/// 与值位写真实值的孪生声明（`theorem t : 3 := 3`）同一条内核判据、同一个 code；
/// 诊断 span 取**签名**的源范围（不是整条命令）。
#[test]
fn open_theorem_with_non_type_signature_is_rejected() {
    let src = "theorem t : 3 := sorry\n";
    let file = parse(src).expect("parse");
    let out = compile_fol(&file);
    assert_eq!(out.errors.len(), 1, "errors: {:?}", out.errors);
    assert_eq!(out.errors[0].code(), "kernel-expected-sort");
    let span = out.errors[0].span;
    assert_eq!(
        &file.src[span.start.offset..span.end.offset],
        "3",
        "诊断 span 必须落在签名上"
    );
    assert!(
        !out.events
            .iter()
            .any(|e| matches!(e, CheckEvent::ExerciseOpen { .. })),
        "签名不过就不能再发 exercise.open：{:?}",
        out.events
    );
}

/// 签名里的未定义名：`declared_ty` 的 elaborate 失败必须上报（原来是 `.ok()` 吞掉）。
#[test]
fn open_theorem_with_unknown_identifier_in_signature_is_rejected() {
    let src = "theorem t : Bogus := sorry\n";
    let file = parse(src).expect("parse");
    let out = compile_fol(&file);
    assert_eq!(out.errors.len(), 1, "errors: {:?}", out.errors);
    assert_eq!(out.errors[0].code(), "elab-unknown-identifier");
    let span = out.errors[0].span;
    assert_eq!(&file.src[span.start.offset..span.end.offset], "Bogus");
    assert!(
        !out.events
            .iter()
            .any(|e| matches!(e, CheckEvent::ExerciseOpen { .. })),
        "{:?}",
        out.events
    );
}

/// 三处吞错点都要覆盖：`def` 与 `example` 的开路径同样受检。
#[test]
fn open_def_and_example_with_non_type_signature_are_rejected() {
    for src in ["def d : 3 := sorry\n", "example : 3 := sorry\n"] {
        let file = parse(src).expect("parse");
        let out = compile_fol(&file);
        assert_eq!(out.errors.len(), 1, "{src}: {:?}", out.errors);
        assert_eq!(out.errors[0].code(), "kernel-expected-sort", "{src}");
        let span = out.errors[0].span;
        assert_eq!(&file.src[span.start.offset..span.end.offset], "3", "{src}");
        assert!(
            !out.events
                .iter()
                .any(|e| matches!(e, CheckEvent::ExerciseOpen { .. })),
            "{src}: {:?}",
            out.events
        );
    }
}

/// `by` 路径（`theorem t : 3 := by sorry`）与直接值位共用同一处签名检查。
#[test]
fn open_theorem_by_sorry_shares_the_signature_gate() {
    let file = parse("theorem t : 3 := by sorry\n").expect("parse");
    let out = compile_fol(&file);
    assert_eq!(out.errors.len(), 1, "errors: {:?}", out.errors);
    assert_eq!(out.errors[0].code(), "kernel-expected-sort");
    assert!(!out
        .events
        .iter()
        .any(|e| matches!(e, CheckEvent::ExerciseOpen { .. })));
}

/// 签名是类型但不是 Prop：内核的 theorem 规则必须生效（镜像
/// `theorem t : Nat := 3` 的 checked 路径）。
#[test]
fn open_theorem_with_non_prop_signature_is_rejected() {
    for src in [
        "theorem t : Nat := sorry\n",
        // `forall (α : Type), α -> α` 是 Type 层的 Pi，不是 Prop。
        "theorem t : forall (α : Type), α -> α := sorry\n",
        "theorem t : Prop -> Type := sorry\n",
    ] {
        let file = parse(src).expect("parse");
        let out = compile_fol(&file);
        assert_eq!(out.errors.len(), 1, "{src}: {:?}", out.errors);
        assert_eq!(out.errors[0].code(), "kernel-theorem-not-prop", "{src}");
        let span = out.errors[0].span;
        assert_eq!(
            &file.src[span.start.offset..span.end.offset],
            src.trim_end_matches('\n')
                .split_once(" : ")
                .expect("signature")
                .1
                .trim_end_matches(" := sorry"),
            "{src}: 诊断 span 必须落在签名上"
        );
        assert!(!out
            .events
            .iter()
            .any(|e| matches!(e, CheckEvent::ExerciseOpen { .. })));
    }
}

/// 防修过头：签名本来就合法的开放练习一条诊断都不许多，事件仍是 `exercise.open`。
#[test]
fn legal_open_exercises_still_pass_the_signature_gate() {
    for src in [
        "example : Prop -> Prop := sorry\n",
        "example : Nat := sorry\n",
        "theorem t : (a : Prop) -> a -> a := sorry\n",
        "theorem t : forall (a : Prop), a -> a := sorry\n",
        "theorem t : forall (α : Type) (A : α -> Prop), (forall (x : α), A x) -> (forall (x : α), A x) := sorry\n",
        "def d : Nat -> Nat := sorry\n",
        // 宇宙参数必须在签名的作用域里（与 checked 路径同一套 elaborate）：
        // 原来这里传空宇宙表，`{u}` 签名连 `ty_text` 都渲染不出来。
        "theorem t {u} (α : Sort u) (a : α) : forall (P : α -> Prop), P a -> P a := sorry\n",
        "def d {u} (α : Sort u) (a : α) : α := sorry\n",
    ] {
        let file = parse(src).expect("parse");
        let out = compile_fol(&file);
        assert_eq!(out.errors, vec![], "{src}");
        assert!(
            out.events
                .iter()
                .any(|e| matches!(e, CheckEvent::ExerciseOpen { .. })),
            "{src}: {:?}",
            out.events
        );
    }
}

/// 合法开放练习照样**不进环境**（`tests.rs` 既有语义），且签名检查不改这一点。
#[test]
fn signature_gate_keeps_open_exercises_out_of_the_env() {
    let file = parse(
        "theorem open_one : (a : Prop) -> a -> a := sorry\n\
         theorem ok : (a : Prop) -> a -> a := fun (a : Prop) => fun (h : a) => h\n",
    )
    .expect("parse");
    let out = compile_fol(&file);
    assert_eq!(out.errors, vec![], "{:?}", out.errors);
    assert!(out.events.iter().any(
        |e| matches!(e, CheckEvent::ExerciseOpen { name } if name.as_deref() == Some("open_one"))
    ));
    assert!(out
        .events
        .iter()
        .any(|e| matches!(e, CheckEvent::DeclarationChecked { name } if name == "ok")));
}

/// 声明级 report：签名不过 ⇒ 状态是 Failed（不是 Open），错误与 `out.errors` 同一条。
#[test]
fn signature_failure_marks_the_declaration_failed() {
    let file = parse("theorem t : 3 := sorry\n").expect("parse");
    let report = check_document(&file);
    assert_eq!(report.decls.len(), 1, "{:?}", report.decls);
    assert_eq!(report.decls[0].status, DeclStatus::Failed);
    assert_eq!(
        report.decls[0].error.as_ref().map(|e| e.code()),
        Some("kernel-expected-sort")
    );
}

// ---- 构造子命名空间（G-02 / WO-005；design docs/design/ctor-namespace.md）----

/// R1 的最小复现同构：两个块各写 `ctor mk`，规范名 `P1.mk`/`P2.mk` 并存不冲突。
/// 修前：第二个 `ctor mk` 撞 `elab-duplicate-declaration`，`P1.mk` 报
/// `elab-unknown-identifier`（复现件 docs/gaps/repro/G02-ctor-namespace.sokonanoda）。
#[test]
fn ctor_names_are_namespaced_after_their_inductive() {
    let src = "\
inductive P1 (A : Type) : Type
ctor mk (a : A) : P1 A
end

inductive P2 (A : Type) : Type
ctor mk (a : A) : P2 A
end

def first (A : Type) (a : A) : P1 A := P1.mk A a
def second (A : Type) (a : A) : P2 A := P2.mk A a
";
    let out = compile_fol(&parse(src).expect("parse namespaced ctors"));
    assert_eq!(out.errors, vec![], "errors: {:?}", out.errors);
    for name in ["P1", "P2", "first", "second"] {
        assert!(
            out.events
                .iter()
                .any(|e| matches!(e, CheckEvent::DeclarationChecked { name: n } if n == name)),
            "`{name}` must check: {:?}",
            out.events
        );
    }
}

/// R1：ctor 名**已含点**则原样（不重复加前缀）——prelude 与既有显式写法靠这条。
#[test]
fn ctor_name_with_a_dot_is_kept_verbatim() {
    let src = "\
inductive Foo : Type
ctor Foo.bar : Foo
end
def f : Foo := Foo.bar
";
    let out = compile_fol(&parse(src).expect("parse dotted ctor"));
    assert_eq!(out.errors, vec![], "errors: {:?}", out.errors);
    assert!(
        !out.errors.iter().any(|e| e.message.contains("Foo.Foo.bar")),
        "the prefix must not be doubled: {:?}",
        out.errors
    );
}

/// R2：唯一的裸名解析为别名（既有课程/示例零改动继续绿），且**别名解析到
/// 规范名**（不是造出第二个内核常量）。
#[test]
fn a_unique_bare_ctor_name_resolves_through_the_alias() {
    let src = "\
inductive Wrap : Type
ctor mk : Wrap
end
def w : Wrap := mk
#print mk
";
    let out = compile_fol(&parse(src).expect("parse bare alias"));
    assert_eq!(out.errors, vec![], "errors: {:?}", out.errors);
    let printed = out.events.iter().find_map(|event| match event {
        CheckEvent::Printed { name, text } => Some((name.as_str(), text.as_str())),
        _ => None,
    });
    // 实测（真实二进制）：别名解析到规范名 ⇒ 打印的正是规范声明。
    assert_eq!(
        printed,
        Some(("Wrap.mk", "constructor Wrap.mk : Wrap")),
        "the alias must print the canonical declaration: {:?}",
        out.events
    );
}

/// R2：裸名被两个构造子占用 ⇒ 不可解析，报稳定的新码
/// `elab-ambiguous-ctor-alias`（**不是** unknown identifier）。
#[test]
fn a_duplicated_bare_ctor_name_is_ambiguous() {
    let src = "\
inductive P1 (A : Type) : Type
ctor mk (a : A) : P1 A
end
inductive P2 (A : Type) : Type
ctor mk (a : A) : P2 A
end
def first (A : Type) (a : A) : P1 A := mk A a
";
    let out = compile_fol(&parse(src).expect("parse ambiguous alias"));
    assert_eq!(
        out.errors.first().map(|e| e.code()),
        Some("elab-ambiguous-ctor-alias"),
        "errors: {:?}",
        out.errors
    );
    let message = &out.errors[0].message;
    assert!(
        message.contains("P1.mk") && message.contains("P2.mk"),
        "the message must name both candidates: {message}"
    );
}

/// R2：真实声明优先于构造子别名（裸名被 def 占用时，def 照常解析）。
#[test]
fn a_real_declaration_wins_over_a_bare_ctor_alias() {
    let src = "\
inductive Wrap : Type
ctor mk : Wrap
end
def mk : Wrap := Wrap.mk
def w : Wrap := mk
";
    let out = compile_fol(&parse(src).expect("parse shadowing decl"));
    assert_eq!(out.errors, vec![], "errors: {:?}", out.errors);
}

/// R1 + 派生 recursor：无显式 rec 的块，派生的 iota 规则名必须是**规范名**
/// （内核断言 `rule.ctor_name == ctor.name`；漏改 = 派生 rec 一律对不上规则）。
/// 顺带实测归约形态：源 `Nat` 的 ctor 叫 `Nat.succ` ⇒ 内核 name cache 的
/// NatRed 快路径把一元链折成 NatLit（`docs/architecture.md` §5.4）。
#[test]
fn derived_recursor_uses_canonical_ctor_names() {
    let src = "\
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
";
    let out = compile_fol(&parse(src).expect("parse derived recursor"));
    assert_eq!(out.errors, vec![], "errors: {:?}", out.errors);
    // 实测（真实二进制）：规范名让 `succ` 链走原生 Nat 快路径 —— 混合表示。
    assert!(
        out.events.iter().any(|e| matches!(
            e,
            CheckEvent::Reduced { text, .. } if text == "Nat.succ (Nat.succ (Nat.succ 1))"
        )),
        "expected the measured mixed NatLit form, got {:?}",
        out.events
    );
}

/// R3 对照：显式 `rec` + `iota` 规则继续按**源名**匹配（`iota zero :=` 里的
/// `zero` 不是 ctor 名，是源级匹配键），且规范名让归约走 NatLit。
#[test]
fn explicit_iota_rules_still_match_by_source_name() {
    let src = "\
inductive Nat : Type
ctor zero : Nat
ctor succ (n : Nat) : Nat
rec Nat.rec {u} : (motive : (n : Nat) -> Sort u) -> (mz : motive zero) -> (ms : (n : Nat) -> motive n -> motive (succ n)) -> (n : Nat) -> motive n
iota zero := fun (motive : (n : Nat) -> Sort u) => fun (mz : motive zero) => fun (ms : (n : Nat) -> motive n -> motive (succ n)) => mz
iota succ := fun (motive : (n : Nat) -> Sort u) => fun (mz : motive zero) => fun (ms : (n : Nat) -> motive n -> motive (succ n)) => fun (n : Nat) => ms n (Nat.rec.{u} motive mz ms n)
end
def oneNat : Nat := succ zero
#reduce Nat.rec.{1} (fun (n : Nat) => Nat) zero (fun (n : Nat) => fun (ih : Nat) => succ n) (succ zero)
";
    let out = compile_fol(&parse(src).expect("parse explicit rec"));
    assert_eq!(out.errors, vec![], "errors: {:?}", out.errors);
    assert!(
        out.events
            .iter()
            .any(|e| matches!(e, CheckEvent::Reduced { text, .. } if text == "1")),
        "explicit iota + canonical ctors must reduce through the Nat fast path: {:?}",
        out.events
    );
}

/// R3：`match` 的分支可以写**规范名**，也可以继续写源名（迁移期两种都能跑）。
#[test]
fn match_arms_accept_both_spellings() {
    let canonical = "\
inductive Color : Type
ctor red : Color
ctor green : Color
end
def swap (c : Color) : Color := match c with
| Color.red => Color.green
| Color.green => Color.red
#reduce swap red
";
    let out = compile_fol(&parse(canonical).expect("parse canonical arms"));
    assert_eq!(out.errors, vec![], "errors: {:?}", out.errors);
    assert!(
        out.events
            .iter()
            .any(|e| matches!(e, CheckEvent::Reduced { text, .. } if text == "Color.green")),
        "canonical arm spelling must reduce: {:?}",
        out.events
    );

    let bare = "\
inductive Color : Type
ctor red : Color
ctor green : Color
end
def swap (c : Color) : Color := match c with
| red => green
| green => red
#reduce swap red
";
    let out = compile_fol(&parse(bare).expect("parse bare arms"));
    assert_eq!(out.errors, vec![], "errors: {:?}", out.errors);
}

/// hover/goto 回填（`check/kernel_phase.rs` 的 name→def-span 表）：
/// 规范名与前缀名两种写法都要指回定义。
#[test]
fn hover_resolution_uses_the_canonical_definition_span() {
    let src = "\
inductive Wrap : Type
ctor mk : Wrap
end
def viaPrefix : Wrap := Wrap.mk
def viaAlias : Wrap := mk
";
    let report = check_document(&parse(src).expect("parse hover"));
    assert!(report.errors.is_empty(), "errors: {:?}", report.errors);
    let resolved: Vec<&str> = report
        .hovers
        .iter()
        .filter_map(|h| match h.resolution.as_ref() {
            Some(ResolvedTarget::Declaration { name, span }) if span.start.offset > 0 => {
                Some(name.as_str())
            }
            _ => None,
        })
        .collect();
    assert!(
        resolved.iter().filter(|n| **n == "Wrap.mk").count() >= 2,
        "both spellings must resolve to the canonical name: {resolved:?}"
    );
}

/// 构造子 spine 的**规范名拼写**（R1）：`Pair.mk sorry sorry` 与裸 `mk sorry sorry`
/// 都拿到同样的子洞期望类型（`goals.rs` 的 `funcs` 两种拼写都登记）。
///
/// 注意（as-built，与 WO 范围表的差异）：归纳块的 `CtorTemplate.result_arg_names`
/// 一直是空的（只有 axiom 视图填），所以 `refine_template` 对源内归纳构造子本来
/// 就是 `None` —— 本刀不改这条既有边界，只保证骨架一旦产出就用规范名
/// （`canonical_name`；axiom 视图里两者相同）。
#[test]
fn ctor_spine_accepts_both_spellings() {
    let spellings = [
        "def probe (A B : Type) : Pair A B := Pair.mk sorry sorry\n",
        "def probe (A B : Type) : Pair A B := mk sorry sorry\n",
    ];
    for body in spellings {
        let src = format!(
            "inductive Pair (A B : Type) : Type\n\
ctor mk (a : A) (b : B) : Pair A B\n\
end\n{body}"
        );
        let report = check_document(&parse(&src).expect("parse spine"));
        assert!(report.errors.is_empty(), "errors: {:?}", report.errors);
        let open = report
            .decls
            .iter()
            .find(|d| d.status == DeclStatus::Open)
            .expect("open");
        let tys: Vec<Option<&str>> = open.sub_goals.iter().map(|s| s.ty.as_deref()).collect();
        assert_eq!(
            tys,
            vec![Some("A"), Some("B")],
            "both spellings must resolve the ctor template: {body:?}"
        );
    }
}

/// `#print` 走别名：`#print mk` 打印的是规范声明（不是 unknown declaration）。
#[test]
fn print_resolves_a_bare_ctor_alias_to_the_canonical_name() {
    let src = "\
inductive Wrap : Type
ctor mk : Wrap
end
#print mk
#print Wrap.mk
";
    let out = compile_fol(&parse(src).expect("parse prints"));
    assert_eq!(out.errors, vec![], "errors: {:?}", out.errors);
    let printed: Vec<(&str, &str)> = out
        .events
        .iter()
        .filter_map(|event| match event {
            CheckEvent::Printed { name, text } => Some((name.as_str(), text.as_str())),
            _ => None,
        })
        .collect();
    assert_eq!(printed.len(), 2, "{:?}", out.events);
    assert!(
        printed[0].1.contains("Wrap.mk") && printed[1].1.contains("Wrap.mk"),
        "both spellings print the canonical declaration: {printed:?}"
    );
}

// ---- 记法（G-04 / WO-011，docs/design/notation-subset.md §2 N4/N7）--------
//
// 判据一律走内核（REQUIREMENTS §2.8）：等价性看**事件序列**，护城河看**内核
// 拒绝**——不做文本比对。

/// 课程库形状的最小夹具（与 `courses/set-theory/lib/Set.sokonanoda` 同签名：
/// `α` 是**显式**前导参数，所以记法展开必须自己补它）。
const NOTATION_LIB: &str = "\
def Set (α : Type) : Type := α -> Prop\n\
def Set.mem (α : Type) (a : α) (A : Set α) : Prop := A a\n\
def Set.subset (α : Type) (A B : Set α) : Prop := forall (x : α), A x -> B x\n\
def Set.empty (α : Type) : Set α := fun (x : α) => False\n";

/// 事件序列（类型 + 名字/文本），用于「两种写法判卷一致」的**逐一**比对。
fn event_shapes(out: &CompileOutput) -> Vec<String> {
    out.events.iter().map(|e| format!("{e:?}")).collect()
}

fn compile_ok(src: &str) -> CompileOutput {
    let file = parse(src).unwrap_or_else(|e| panic!("parse: {e:?}\n{src}"));
    let out = compile_fol(&file);
    assert_eq!(out.errors, vec![], "errors: {:?}\n{src}", out.errors);
    out
}

#[test]
fn notation_and_pointful_spellings_compile_identically() {
    // N7 教学契约：同一命题的两种写法——点名 `Set.mem α a A` 与记法 `a ∈ A`
    // ——**判卷结果一致**（事件序列逐一相等）。
    let pointful = format!(
        "{NOTATION_LIB}\
         def p (α : Type) (a : α) (A : Set α) : Prop := Set.mem α a A\n\
         def s (α : Type) (A B : Set α) (h : Set.subset α A B) : Set.subset α A B := h\n\
         def e (α : Type) : Set α := Set.empty α\n"
    );
    let notation = format!(
        "{NOTATION_LIB}\
         infix:50 \" ∈ \" => Set.mem\n\
         infix:50 \" ⊆ \" => Set.subset\n\
         notation \"∅\" => Set.empty\n\
         def p (α : Type) (a : α) (A : Set α) : Prop := a ∈ A\n\
         def s (α : Type) (A B : Set α) (h : A ⊆ B) : A ⊆ B := h\n\
         def e (α : Type) : Set α := ∅\n"
    );
    let pointful_out = compile_ok(&pointful);
    let notation_out = compile_ok(&notation);
    assert_eq!(
        event_shapes(&pointful_out),
        event_shapes(&notation_out),
        "the two spellings must produce the same events"
    );
    assert!(
        pointful_out
            .events
            .iter()
            .any(|e| matches!(e, CheckEvent::DeclarationChecked { .. })),
        "the fixture must actually check declarations"
    );
}

#[test]
fn notation_nullary_completes_the_leading_type_parameter_from_the_expected_type() {
    // `notation "∅" => Set.empty`：`Set.empty : (α : Type) → Set α`，零操作数
    // ⇒ 只能从**期望类型**补 `α`（设计 N4.2 ②）。
    let src = format!(
        "{NOTATION_LIB}\
         notation \"∅\" => Set.empty\n\
         def e (α : Type) : Set α := ∅\n"
    );
    let out = compile_ok(&src);
    assert!(
        out.events
            .iter()
            .any(|e| matches!(e, CheckEvent::DeclarationChecked { name } if name == "e")),
        "the notation must check: {:?}",
        out.events
    );
}

#[test]
fn notation_nullary_without_an_expected_type_is_unsolved() {
    // `#check ∅`（无期望类型）⇒ `elab-notation-argument-unsolved`（设计 N4.2）。
    let src = format!(
        "{NOTATION_LIB}\
         notation \"∅\" => Set.empty\n\
         #check ∅\n"
    );
    let file = parse(&src).expect("parse");
    let out = compile_fol(&file);
    let codes: Vec<&str> = out.errors.iter().map(|e| e.code()).collect();
    assert_eq!(
        codes,
        vec!["elab-notation-argument-unsolved"],
        "errors: {:?}",
        out.errors
    );
    assert!(
        out.errors[0].hint().contains("点名写法"),
        "the hint must teach the pointful spelling: {}",
        out.errors[0].hint()
    );
}

#[test]
fn notation_unknown_target_is_a_dedicated_diagnostic() {
    // 目标名解析发生在**使用点**（`check/walk.rs` 对 `Command::Notation`
    // 是 no-op：记法命令不 elaborate、不产 PendingOp——设计 N6）。
    let src = format!(
        "{NOTATION_LIB}\
         infix:50 \" ∈ \" => Set.men\n\
         def p (α : Type) (a : α) (A : Set α) : Prop := a ∈ A\n"
    );
    let file = parse(&src).expect("parse");
    let out = compile_fol(&file);
    assert_eq!(
        out.errors.iter().map(|e| e.code()).collect::<Vec<_>>(),
        vec!["elab-notation-unknown-target"],
        "errors: {:?}",
        out.errors
    );
    assert!(
        out.errors[0].message.contains("Set.men"),
        "the message names the missing target: {}",
        out.errors[0].message
    );
}

#[test]
fn pointful_application_without_the_leading_type_parameter_is_still_rejected() {
    // **护城河**（设计 N4.3）：补全只挂在记号展开路径上——点名写法省 `α`
    // 今天被内核拒绝，改后必须**仍**被拒绝（同 stage）。
    //
    // **2026-09-21（G-21）重钉的是诊断码**：这条拒绝以前落进泛化的
    // `kernel-rejected`，现在被 `classify_term_in_type_position` 认出形状
    // （**项落在类型位**）并归到 `kernel-expected-sort`，提示直接说出
    // 「漏了前导类型参数 + 可以用记法」。护城河本身一个字没改。
    let src = format!(
        "{NOTATION_LIB}\
         def p (α : Type) (a : α) (A : Set α) : Prop := Set.mem a A\n"
    );
    let file = parse(&src).expect("parse");
    let out = compile_fol(&file);
    assert_eq!(
        out.errors.iter().map(|e| e.code()).collect::<Vec<_>>(),
        vec!["kernel-expected-sort"],
        "the moat must hold: {:?}",
        out.errors
    );
    assert_eq!(out.errors[0].stage(), crate::compile::CompileStage::Kernel);
    let hint = out.errors[0].hint();
    assert!(
        hint.contains("前导类型参数"),
        "the hint must name the cause (G-21): {hint:?}"
    );
}

/// **T-D15 的判据**：记法 hover 行的 `resolution` 在**报告装配之后**仍是
/// `ResolvedTarget::Notation`——没有被 `kernel_phase` 的"回填顶层声明 span"
/// 改写成 `def Set.mem` 的 span（这是 D5 记下的坑）。
///
/// 装配那一步（`kernel_phase.rs`）只对 `Declaration` 变体回填：
/// ```ignore
/// if let Some(ResolvedTarget::Declaration { name, .. }) = &node.resolution { … }
/// ```
/// ⇒ 新变体天然不受影响，但**"天然"不是判据**，所以钉一条测试。
#[test]
fn a_notation_hover_row_keeps_its_notation_resolution_after_assembly() {
    let src = format!(
        "{NOTATION_LIB}\
         infix:50 \" ∈ \" => Set.mem\n\
         def p (α : Type) (a : α) (A : Set α) : Prop := a ∈ A\n"
    );
    let file = parse(&src).expect("parse");
    let report = check_document(&file);
    let notation_start = src.find("a ∈ A").expect("notation text");
    let symbol_start = src[notation_start..]
        .find('∈')
        .expect("symbol in the notation")
        + notation_start;
    let symbol_span = crate::span::Span::new(
        crate::span::Pos {
            offset: symbol_start,
            line: 0,
            column: 0,
        },
        crate::span::Pos {
            offset: symbol_start + '∈'.len_utf8(),
            line: 0,
            column: 0,
        },
    );
    // 找**那条记法行**：它的 `resolution` 是 `Notation` 变体（这正是 T-D15 加的）。
    let (row, symbol, span) = report
        .hovers
        .iter()
        .find_map(|h| match h.resolution.as_ref() {
            Some(crate::compile::ResolvedTarget::Notation { symbol, span, .. }) => {
                Some((h, symbol.clone(), *span))
            }
            _ => None,
        })
        .expect("必须有一条记法行的 resolution 是 Notation 变体");
    assert_eq!(symbol, "∈");
    assert_eq!(
        (span.start.offset, span.end.offset),
        (symbol_span.start.offset, symbol_span.end.offset),
        "resolution 的 span 是**使用处那个符号自己**（T-D14 的 symbol_span）"
    );
    let _ = notation_start;
    // 反向：它**不能**是 `Set.mem` 的声明 span（D5 的坑）。
    let def_span = report
        .decls
        .iter()
        .find(|d| d.name.as_deref() == Some("Set.mem"))
        .map(|d| d.span);
    if let Some(def_span) = def_span {
        assert_ne!(
            row.resolution.as_ref().map(|r| r.span()),
            Some(def_span),
            "记法的 resolution 不得被回填成 `Set.mem` 的声明 span"
        );
    }
}

#[test]
fn notation_records_a_hover_row_covering_the_whole_notation() {
    // 记号节点整段 `lhs sym rhs` 一条 hover 行（设计 §4）。
    let src = format!(
        "{NOTATION_LIB}\
         infix:50 \" ∈ \" => Set.mem\n\
         def p (α : Type) (a : α) (A : Set α) : Prop := a ∈ A\n"
    );
    let file = parse(&src).expect("parse");
    let report = check_document(&file);
    let decl = report
        .decls
        .iter()
        .find(|d| d.name.as_deref() == Some("p"))
        .expect("decl p");
    let span = decl.span;
    let notation_start = src.find("a ∈ A").expect("notation text");
    assert!(
        report.hovers.iter().any(|h| {
            h.span.start.offset == notation_start
                && h.span.end.offset == notation_start + "a ∈ A".len()
                && h.span.start.offset >= span.start.offset
        }),
        "a hover row must cover the whole `a ∈ A`: {:?}",
        report
            .hovers
            .iter()
            .map(|h| (h.span.start.offset, h.span.end.offset))
            .collect::<Vec<_>>()
    );

    // **T-D14**：AST 侧的 `Expr::Notation.symbol_span` **只覆盖 `∈`**，与上面那条
    // "hover 行覆盖整段"**不矛盾**——两者回答的是不同的问题：
    // hover 行是"这一段表达式是什么类型"，`symbol_span` 是"光标是不是压在符号上"。
    // 补这条断言是因为 `Expr::Notation` 加了字段（AST 变更），而这段测试正好在
    // 同一个夹具上（计划 T-D14 的"风险"一节点名了它）。
    let mut symbol_spans = Vec::new();
    for command in &file.commands {
        // 夹具里 `a ∈ A` 在 `def p` 的**值**里（不是类型）。
        let crate::ast::Command::Def { val, .. } = command else {
            continue;
        };
        let mut visit = |e: crate::ast::Expr| {
            if let crate::ast::Expr::Notation { symbol_span, .. } = &e {
                symbol_spans.push(*symbol_span);
            }
            e
        };
        visit(val.clone());
        let _ = crate::display::map_children_with(val.clone(), &mut visit);
    }
    assert!(
        symbol_spans
            .iter()
            .any(|s| &src[s.start.offset..s.end.offset] == "∈"),
        "AST 里那条记法的 `symbol_span` 只覆盖 `∈`：{symbol_spans:?}"
    );
}

#[test]
fn notation_command_emits_no_events_and_is_not_a_declaration() {
    // N6：记法命令不产生 decl.checked/exercise.open/expr.typed/expr.reduced，
    // 也不进声明表。
    let src = format!("{NOTATION_LIB}infix:50 \" ∈ \" => Set.mem\nnotation \"∅\" => Set.empty\n");
    let file = parse(&src).expect("parse");
    let out = compile_fol(&file);
    assert_eq!(out.errors, vec![]);
    assert_eq!(
        out.events.len(),
        4,
        "only the four declarations are events: {:?}",
        out.events
    );
    let report = check_document(&file);
    assert_eq!(
        report.decls.len(),
        4,
        "notation commands must not appear in the declaration table: {:?}",
        report.decls.iter().map(|d| &d.name).collect::<Vec<_>>()
    );
}

// ---- namespace / open（G-05，docs/design/namespace-open.md §1 N3–N6）--------
//
// 判据一律走内核：等价性看**事件序列**，解析顺序看**内核接受的类型**，
// 找不到看**稳定错误码 + hint**——不做文本比对。

/// 命名空间夹具：`A.x : Prop`（不可当 `Type` 用）与 `A.B.x : Type`
/// （可以）——解析顺序 ① 的「最长前缀优先」因此由内核判定。
const NAMESPACE_ORDER_LIB: &str = "\
namespace A\n\
def x : Prop := forall (p : Prop), p -> p\n\
namespace B\n\
def x : Type := Prop\n\
def y : Type := x\n\
end B\n\
end A\n";

#[test]
fn namespace_short_names_and_pointful_names_grade_identically() {
    // N3/N7 教学契约：`namespace A` 里的短名与外面的点名**是同一个全局名**
    // ⇒ 事件序列逐一相等。
    let inside = "\
namespace A\n\
def mem (α : Type) (a : α) (s : α -> Prop) : Prop := s a\n\
def use (α : Type) (a : α) (s : α -> Prop) : Prop := mem α a s\n\
end A\n";
    let outside = "\
def A.mem (α : Type) (a : α) (s : α -> Prop) : Prop := s a\n\
def A.use (α : Type) (a : α) (s : α -> Prop) : Prop := A.mem α a s\n";
    let inside_out = compile_ok(inside);
    let outside_out = compile_ok(outside);
    assert_eq!(
        event_shapes(&inside_out),
        event_shapes(&outside_out),
        "both spellings must produce the same events"
    );
    assert!(
        inside_out
            .events
            .iter()
            .any(|e| matches!(e, CheckEvent::DeclarationChecked { name } if name == "A.mem")),
        "the declared name must be the global one: {:?}",
        inside_out.events
    );
}

#[test]
fn namespace_resolution_prefers_the_longest_namespace_prefix() {
    // ① 当前命名空间链从内到外、最长前缀优先：`A.B.y := x` 取 `A.B.x`（Type）。
    // 若错误地取 `A.x`（Prop），内核会拒绝——所以这条断言由内核判定。
    let out = compile_ok(NAMESPACE_ORDER_LIB);
    assert!(
        out.events
            .iter()
            .any(|e| matches!(e, CheckEvent::DeclarationChecked { name } if name == "A.B.y")),
        "A.B.y must check: {:?}",
        out.events
    );
}

#[test]
fn namespace_resolution_falls_back_to_the_exact_name() {
    // ② 精确名：命名空间里没有 `A.y`，但根上有 `y`。
    let src = "\
def y : Type := Prop\n\
namespace A\n\
def z : Type := y\n\
end A\n";
    let out = compile_ok(src);
    assert!(out
        .events
        .iter()
        .any(|e| matches!(e, CheckEvent::DeclarationChecked { name } if name == "A.z")));
}

#[test]
fn open_makes_a_prefix_omissible_and_sees_later_declarations() {
    // ③ `open A` 把 `A.` 加进可省略前缀集合；它是**集合**不是快照，
    // 所以 `open` 之后（甚至之前）声明的 `A.x` 都享受（设计 N4）。
    let src = "\
open A\n\
namespace A\n\
def x : Type := Prop\n\
end A\n\
def y : Type := x\n";
    let out = compile_ok(src);
    assert!(out
        .events
        .iter()
        .any(|e| matches!(e, CheckEvent::DeclarationChecked { name } if name == "y")));
}

#[test]
fn open_order_decides_between_two_candidates() {
    // ③ open 按**出现顺序**：先开的先试。
    let first_b = "\
namespace A\n\
def x : Prop := forall (p : Prop), p -> p\n\
end A\n\
namespace B\n\
def x : Type := Prop\n\
end B\n\
open B\n\
open A\n\
def y : Type := x\n";
    let first_a = "\
namespace A\n\
def x : Prop := forall (p : Prop), p -> p\n\
end A\n\
namespace B\n\
def x : Type := Prop\n\
end B\n\
open A\n\
open B\n\
def y : Type := x\n";
    let out = compile_ok(first_b);
    assert!(
        out.events
            .iter()
            .any(|e| matches!(e, CheckEvent::DeclarationChecked { name } if name == "y")),
        "`open B` first must win (B.x : Type): {:?}",
        out.events
    );
    // 反过来先开 A ⇒ 取 A.x : Prop ⇒ 内核拒绝（证明顺序真的在起作用）。
    let file = parse(first_a).expect("parse");
    let out = compile_fol(&file);
    assert!(
        !out.errors.is_empty(),
        "the reversed open order must pick A.x (Prop) and be rejected: {:?}",
        out.events
    );
}

#[test]
fn a_namespace_reference_that_is_nowhere_is_the_ordinary_unknown_identifier() {
    let src = "\
namespace A\n\
def x : Type := Prop\n\
end A\n\
def y : Type := nope\n";
    let file = parse(src).expect("parse");
    let out = compile_fol(&file);
    assert_eq!(
        out.errors.iter().map(|e| e.code()).collect::<Vec<_>>(),
        vec!["elab-unknown-identifier"],
        "errors: {:?}",
        out.errors
    );
    assert!(
        out.errors[0].hint().contains("open"),
        "the hint must mention `open`: {}",
        out.errors[0].hint()
    );
}

#[test]
fn namespace_coexists_with_ctor_namespaces() {
    // G-02 的规范名与 N4 的命名空间解析是同一条 `known` 路径：
    // `Wrap.Box.mk` 是规范名，裸名 `mk` 仍是唯一别名；`open Wrap` 后
    // 短名 `Box` 也能解析。
    let src = "\
namespace Wrap\n\
inductive Box : Type\n\
ctor mk : Box\n\
end\n\
def b : Box := mk\n\
end Wrap\n\
open Wrap\n\
def c : Box := Wrap.Box.mk\n";
    let out = compile_ok(src);
    let names: Vec<&str> = out
        .events
        .iter()
        .filter_map(|e| match e {
            CheckEvent::DeclarationChecked { name } => Some(name.as_str()),
            _ => None,
        })
        .collect();
    assert!(
        names.contains(&"Wrap.Box"),
        "canonical inductive name: {names:?}"
    );
    assert!(names.contains(&"Wrap.b"), "inside the namespace: {names:?}");
    assert!(names.contains(&"c"), "outside with `open Wrap`: {names:?}");
}

#[test]
fn a_by_block_inside_a_namespace_resolves_short_names() {
    // 判卷合成（前缀 + 合成 `#check`）必须容忍未闭合的 `namespace`
    // （设计 §4.1/§4.2）：`exact` 走 judge_terms，`apply` 走 judge_infer。
    let src = "\
namespace A\n\
def mem (α : Type) (a : α) (s : α -> Prop) : Prop := s a\n\
theorem t (α : Type) (a : α) (s : α -> Prop) : mem α a s -> mem α a s := by intro h; exact h\n\
theorem u (α : Type) (a : α) (s : α -> Prop) : mem α a s -> mem α a s := by intro h; apply h\n\
end A\n";
    let out = compile_ok(src);
    let names: Vec<&str> = out
        .events
        .iter()
        .filter_map(|e| match e {
            CheckEvent::DeclarationChecked { name } => Some(name.as_str()),
            _ => None,
        })
        .collect();
    assert!(
        names.contains(&"A.t"),
        "exact inside a namespace: {names:?}"
    );
    assert!(
        names.contains(&"A.u"),
        "apply inside a namespace: {names:?}"
    );
}

#[test]
fn namespace_commands_emit_no_events_and_are_not_declarations() {
    // N6：三条命令不产生事件、不进声明表（与 import/记法同族）。
    let src = "\
namespace A\n\
def x : Type := Prop\n\
end A\n\
open A\n";
    let file = parse(src).expect("parse");
    let out = compile_fol(&file);
    assert_eq!(out.errors, vec![]);
    assert_eq!(
        out.events.len(),
        1,
        "only `A.x` is an event: {:?}",
        out.events
    );
    let report = check_document(&file);
    assert_eq!(
        report.decls.len(),
        1,
        "namespace/end/open must not enter the declaration table: {:?}",
        report.decls.iter().map(|d| &d.name).collect::<Vec<_>>()
    );
    assert_eq!(report.decls[0].name.as_deref(), Some("A.x"));
}

#[test]
fn print_check_and_reduce_inside_a_namespace_resolve_short_names() {
    // `#print` / `#check` / `#reduce` 与 `Expr::Ident` 共用同一个解析点
    // （`resolve_known`）——命名空间里的短名对它们同样有效；`#print` 的输出
    // 是**规范名**（内核 pp），不是源里的短名。
    let src = "\
namespace A\n\
def mem (α : Type) (a : α) (s : α -> Prop) : Prop := s a\n\
#print mem\n\
#check mem\n\
end A\n";
    let out = compile_ok(src);
    assert!(
        out.events
            .iter()
            .any(|e| matches!(e, CheckEvent::Printed { name, .. } if name == "A.mem")),
        "`#print mem` must resolve to the global name: {:?}",
        out.events
    );
    // `#check mem`：解析成功才有 `TypeChecked`（未知标识符会走错误通道，
    // 已被 `compile_ok` 的 0 错误断言挡住）。
    assert_eq!(
        out.events
            .iter()
            .filter(|e| matches!(e, CheckEvent::TypeChecked { .. }))
            .count(),
        1,
        "`#check mem` must resolve: {:?}",
        out.events
    );
}

#[test]
fn hover_resolution_inside_a_namespace_points_at_the_global_declaration() {
    // 编辑器反馈通道（hover/goto-definition）读的是 `ResolvedTarget::Declaration`
    // 的**规范名**，所以命名空间里的短名引用会指回全局声明（goto 才能跳对文件）。
    let src = "\
namespace A\n\
def mem (α : Type) (a : α) (s : α -> Prop) : Prop := s a\n\
def use (α : Type) (a : α) (s : α -> Prop) : Prop := mem α a s\n\
end A\n";
    let file = parse(src).expect("parse");
    let report = check_document(&file);
    let resolved: Vec<&str> = report
        .hovers
        .iter()
        .filter_map(|h| match &h.resolution {
            Some(ResolvedTarget::Declaration { name, .. }) => Some(name.as_str()),
            _ => None,
        })
        .collect();
    assert!(
        resolved.contains(&"A.mem"),
        "the short-name use must resolve to `A.mem`: {resolved:?}"
    );
}

// ---- 第二刀：open 的子句 / `open … in` / `export`（设计 §N7/N8）------------
//
// 判据与上面同款：正例看**内核接受了什么**（事件里的名字），反例看**稳定错误
// 码**，警告看 `WarningKind`/code/hint——不做文本比对。

#[test]
fn open_only_hiding_and_renaming_resolve_the_right_names() {
    // 三条子句的**正例**：`only` 留下的、`hiding` 之外的、改名后的短名都能用，
    // 而且解析到的是同一个全局名（`A.x`）。
    let src = "\
namespace A\n\
def x : Type := Prop\n\
def w : Type := Prop\n\
end A\n\
open A (x)\n\
def use_only : Type := x\n\
open A hiding w\n\
def use_hiding : Type := x\n\
open A renaming x => zz\n\
def use_renaming : Type := zz\n";
    let out = compile_ok(src);
    let names: Vec<&str> = out
        .events
        .iter()
        .filter_map(|e| match e {
            CheckEvent::DeclarationChecked { name } => Some(name.as_str()),
            _ => None,
        })
        .collect();
    for expected in ["A.x", "A.w", "use_only", "use_hiding", "use_renaming"] {
        assert!(names.contains(&expected), "missing {expected}: {names:?}");
    }
}

#[test]
fn open_only_keeps_the_other_short_names_out() {
    // 反例：`open A (x)` 之后 `w` 不是候选 ⇒ 普通 `elab-unknown-identifier`。
    let src = "\
namespace A\n\
def x : Type := Prop\n\
def w : Type := Prop\n\
end A\n\
open A (x)\n\
def use : Type := w\n";
    let file = parse(src).expect("parse");
    let out = compile_fol(&file);
    let codes: Vec<&str> = out.errors.iter().map(|e| e.code()).collect();
    assert_eq!(
        codes,
        vec!["elab-unknown-identifier"],
        "events: {:?}",
        out.events
    );
}

#[test]
fn open_renaming_takes_the_original_short_name_away() {
    // 反例：`renaming x => zz` 之后原短名 `x` 不再是候选（改名是替换，不是新增）。
    let src = "\
namespace A\n\
def x : Type := Prop\n\
end A\n\
open A renaming x => zz\n\
def ok : Type := zz\n\
def bad : Type := x\n";
    let file = parse(src).expect("parse");
    let out = compile_fol(&file);
    let codes: Vec<&str> = out.errors.iter().map(|e| e.code()).collect();
    assert_eq!(
        codes,
        vec!["elab-unknown-identifier"],
        "events: {:?}",
        out.events
    );
}

#[test]
fn open_in_is_local_to_that_one_command() {
    // 正例：`open A in def …` 里短名可用；反例：命令结束即撤销（下一行同一条
    // 引用必须报未知标识符）。这条测试就是「局部」二字的判据。
    let src = "\
namespace A\n\
def x : Type := Prop\n\
end A\n\
open A in def inside : Type := x\n\
def outside : Type := x\n";
    let file = parse(src).expect("parse");
    let out = compile_fol(&file);
    assert!(
        out.events
            .iter()
            .any(|e| matches!(e, CheckEvent::DeclarationChecked { name } if name == "inside")),
        "the wrapped declaration must check: {:?}",
        out.events
    );
    let codes: Vec<&str> = out.errors.iter().map(|e| e.code()).collect();
    assert_eq!(
        codes,
        vec!["elab-unknown-identifier"],
        "the local open must not leak past its command: {:?}",
        out.events
    );
}

#[test]
fn open_in_body_is_still_a_real_declaration_for_templates_and_hover() {
    // `open … in <声明>` 包住的声明**照样是声明**：进 `top_level_def_spans`
    // （hover/goto 回填）与声明表（事件里的名字是全局名）。
    let src = "\
namespace A\n\
def x : Type := Prop\n\
end A\n\
open A in def inside : Type := x\n\
def use : Type := inside\n";
    let out = compile_ok(src);
    assert!(
        out.events
            .iter()
            .any(|e| matches!(e, CheckEvent::DeclarationChecked { name } if name == "inside")),
        "events: {:?}",
        out.events
    );
    let file = parse(src).expect("parse");
    let report = check_document(&file);
    let resolved: Vec<&str> = report
        .hovers
        .iter()
        .filter_map(|h| match &h.resolution {
            Some(ResolvedTarget::Declaration { name, .. }) => Some(name.as_str()),
            _ => None,
        })
        .collect();
    assert!(
        resolved.contains(&"inside"),
        "the wrapped declaration must be a hover target: {resolved:?}"
    );
}

#[test]
fn a_by_block_inside_a_local_open_sees_the_same_scope() {
    // §9.2 边界 3：`open A in def … := by …` 的 `by` 引擎合成的是「前缀源码 +
    // 合成命令」，前缀里必须补上那行 open（源码原文）——否则 `apply` 的文本
    // 对齐会拿源里的短名去比内核 pp 的全名。这里 `apply h` 的目标是短名
    // `mem α a s`，只有补了 open 才能被 `judge_render_type` 规范化成 `A.mem …`。
    let src = "\
namespace A\n\
def mem (α : Type) (a : α) (s : α -> Prop) : Prop := s a\n\
end A\n\
open A in theorem t (α : Type) (a : α) (s : α -> Prop) : mem α a s -> mem α a s := by intro h; apply h\n";
    let out = compile_ok(src);
    assert!(
        out.events
            .iter()
            .any(|e| matches!(e, CheckEvent::DeclarationChecked { name } if name == "t")),
        "apply inside a local open must see the same scope: {:?}",
        out.events
    );
}

#[test]
fn export_short_names_are_visible_to_later_declarations() {
    // `export` 的文件内一半：与 `open` 逐字相同（对后续命令生效）。
    let src = "\
namespace A\n\
def x : Type := Prop\n\
end A\n\
export A\n\
def use : Type := x\n";
    let out = compile_ok(src);
    assert!(out
        .events
        .iter()
        .any(|e| matches!(e, CheckEvent::DeclarationChecked { name } if name == "use")));
}

#[test]
fn open_shadowed_names_warn_but_do_not_fail() {
    // N8：两个 `open` 都提供 `x` ⇒ 一条 `open-shadowed-name` **warning**
    // （不是 error）：判定照旧（`y : Type` 取 `A.x`），`ok()` 仍为真。
    let src = "\
namespace A\n\
def x : Type := Prop\n\
end A\n\
namespace B\n\
def x : Prop := forall (p : Prop), p -> p\n\
end B\n\
open A\n\
open B\n\
def y : Type := x\n";
    let out = compile_ok(src);
    assert!(out.ok(), "a shadow warning must not fail the file");
    let warnings: Vec<&CompileWarning> = out
        .warnings
        .iter()
        .filter(|w| w.kind == WarningKind::OpenShadowedName)
        .collect();
    assert_eq!(warnings.len(), 1, "warnings: {:?}", out.warnings);
    assert_eq!(warnings[0].code(), "open-shadowed-name");
    assert!(
        warnings[0].message.contains("A.x") && warnings[0].message.contains("B.x"),
        "the message must name both candidates: {}",
        warnings[0].message
    );
    assert!(!warnings[0].hint().is_empty(), "a hint is required");
    // span 收窄到 `open B` 的名字 token。
    let src_text = &src[warnings[0].span.start.offset..warnings[0].span.end.offset];
    assert_eq!(src_text, "B", "the warning must point at the name token");
}

#[test]
fn a_short_name_colliding_with_the_root_declaration_warns() {
    // 「短名与全局名撞车」：候选 ②（精确名）在 `open` 之前，所以这条 `open`
    // 对 `x` 等于没写——必须有一条 warning 说清楚（而不是静默）。
    let src = "\
def x : Type := Prop\n\
namespace A\n\
def x : Prop := forall (p : Prop), p -> p\n\
end A\n\
open A\n\
def y : Type := x\n";
    let out = compile_ok(src);
    let warnings: Vec<&CompileWarning> = out
        .warnings
        .iter()
        .filter(|w| w.kind == WarningKind::OpenShadowedName)
        .collect();
    assert_eq!(warnings.len(), 1, "warnings: {:?}", out.warnings);
    assert!(
        warnings[0].message.contains("根"),
        "the message must say the root name wins: {}",
        warnings[0].message
    );
}

#[test]
fn a_non_colliding_open_produces_no_warning() {
    // 反例（负向钉住"别乱报警"）：短名各不相同 ⇒ 一条 warning 都没有。
    let src = "\
namespace A\n\
def x : Type := Prop\n\
end A\n\
namespace B\n\
def y : Type := Prop\n\
end B\n\
open A\n\
open B\n\
def use : Type := x\n";
    let out = compile_ok(src);
    assert!(
        !out.warnings
            .iter()
            .any(|w| w.kind == WarningKind::OpenShadowedName),
        "no collision ⇒ no warning: {:?}",
        out.warnings
    );
}

#[test]
fn open_and_export_commands_stay_event_free() {
    // N6 对第二刀的延伸：子句与 `export` 同样不是声明（零事件），
    // `open … in <声明>` 只有那条声明的事件。
    let src = "\
namespace A\n\
def x : Type := Prop\n\
end A\n\
open A (x)\n\
open A hiding x\n\
open A renaming x => zz\n\
export A\n\
open A in def y : Type := zz\n";
    let out = compile_ok(src);
    assert_eq!(
        event_kinds(&out),
        vec!["checked A.x".to_string(), "checked y".to_string()],
        "scope commands must not produce events: {:?}",
        out.events
    );
}

// ---- abbrev（G-08，docs/design/abbrev.md）---------------------------------
//
// 判据一律走内核：`abbrev` 与 `def` 的等价性看**事件序列**，别名透明看
// **内核接受的类型**——不做文本比对。

/// 别名透明的靶子：类型位互换 + `#reduce` 展开 + 项层 `rfl`。
const ABBREV_LIB: &str = "\
def Set (α : Type) : Type := α -> Prop\n\
def Set.mem (α : Type) (a : α) (A : Set α) : Prop := A a\n\
def Set.empty (α : Type) : Set α := fun (x : α) => False\n";

/// 事件形状（**去掉 span**）：`def` 版与 `abbrev` 版的源码偏移必然不同
/// （`abbrev` 每个声明多三个字符），要比的是事件本身，不是坐标。
fn event_kinds(out: &CompileOutput) -> Vec<String> {
    out.events
        .iter()
        .map(|e| match e {
            CheckEvent::DeclarationChecked { name } => format!("checked {name}"),
            CheckEvent::ExampleChecked => "example".to_string(),
            CheckEvent::TypeChecked { text, .. } => format!("typed {text}"),
            CheckEvent::Reduced { text, .. } => format!("reduced {text}"),
            CheckEvent::Printed { name, text } => format!("printed {name} {text}"),
            CheckEvent::ExerciseOpen { name } => format!("open {name:?}"),
        })
        .collect()
}

#[test]
fn abbrev_and_def_compile_identically() {
    // G-08 的核心判据：把同一份画布里的 `def` 全换成 `abbrev`，事件序列必须
    // **逐一相等**（`abbrev` 是同语义拼写，不是新语义）。
    let spelled_def = format!(
        "{ABBREV_LIB}\
         def alias : Type := Set Nat\n\
         theorem t (α : Type) : Eq.{{1}} (Set α) (Set.empty α) (fun (x : α) => False) := by rfl\n\
         #reduce Set\n"
    );
    let spelled_abbrev = format!(
        "{}\
         abbrev alias : Type := Set Nat\n\
         theorem t (α : Type) : Eq.{{1}} (Set α) (Set.empty α) (fun (x : α) => False) := by rfl\n\
         #reduce Set\n",
        ABBREV_LIB.replace("def ", "abbrev ")
    );
    let def_out = compile_ok(&spelled_def);
    let abbrev_out = compile_ok(&spelled_abbrev);
    assert_eq!(
        event_kinds(&def_out),
        event_kinds(&abbrev_out),
        "abbrev must produce exactly the same events as def"
    );
    assert!(
        def_out
            .events
            .iter()
            .any(|e| matches!(e, CheckEvent::DeclarationChecked { name } if name == "alias")),
        "the fixture must actually check the alias: {:?}",
        def_out.events
    );
}

#[test]
fn abbrev_is_transparent_in_type_positions() {
    // Lean `abbrev` 的卖点：`Set α` 与 `α -> Prop` 在 elaboration 里互换，
    // **不需要** `show`/`change`（设计 §1.1）。`def` 今天已经做到，`abbrev`
    // 必须一字不差地继承。
    let src = "\
abbrev Set (α : Type) : Type := α -> Prop\n\
abbrev Set.mem (α : Type) (a : α) (A : Set α) : Prop := A a\n\
def A1 (α : Type) (f : α -> Prop) : Set α := f\n\
def A2 (α : Type) (A : Set α) : α -> Prop := A\n\
def A3 (α : Type) (a : α) (f : α -> Prop) : Prop := Set.mem α a f\n";
    let out = compile_ok(src);
    for name in ["Set", "Set.mem", "A1", "A2", "A3"] {
        assert!(
            out.events
                .iter()
                .any(|e| matches!(e, CheckEvent::DeclarationChecked { name: n } if n == name)),
            "`{name}` must check through the alias: {:?}",
            out.events
        );
    }
}

#[test]
fn abbrev_unfolds_under_reduce_and_by_rfl() {
    // `#reduce` 展开（设计 §1.2）+ 项层 `rfl` 判等（设计 §1.4）——两条都走内核。
    let src = "\
abbrev Set (α : Type) : Type := α -> Prop\n\
abbrev Set.empty (α : Type) : Set α := fun (x : α) => False\n\
theorem t (α : Type) : Eq.{1} (Set α) (Set.empty α) (fun (x : α) => False) := by rfl\n\
#reduce Set\n";
    let out = compile_ok(src);
    assert!(
        out.events
            .iter()
            .any(|e| matches!(e, CheckEvent::DeclarationChecked { name } if name == "t")),
        "the kernel must accept the rfl proof through the alias: {:?}",
        out.events
    );
    let reduced: Vec<&str> = out
        .events
        .iter()
        .filter_map(|e| match e {
            CheckEvent::Reduced { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(
        reduced,
        vec!["fun (α : Type 0) => α -> Prop"],
        "`#reduce Set` must unfold the alias: {:?}",
        out.events
    );
}

#[test]
fn abbrev_with_a_sorry_value_is_an_open_exercise_like_def() {
    // 教学契约：`abbrev` 与 `def` 一样，值位 `sorry` ⇒ `exercise.open`，
    // 签名仍然受检（G-01）。
    let src = "abbrev Set (α : Type) : Type := sorry\n";
    let file = parse(src).expect("parse");
    let out = compile_fol(&file);
    assert_eq!(out.errors, vec![], "errors: {:?}", out.errors);
    assert!(
        out.events
            .iter()
            .any(|e| matches!(e, CheckEvent::ExerciseOpen { .. })),
        "a `sorry` value must open an exercise: {:?}",
        out.events
    );
}

// ---- 一元记法 prefix/postfix（G-04 第二刀，设计 notation-subset.md §10.1）----
//
// 判据一律走内核：两种写法的一致性看**事件序列**，优先级看**内核接受的项**
// （`Eq … := by rfl` 只在两边真的同一个项时通过），护城河看**内核拒绝**。

/// 第二刀的靶子：一元记法 + `''`（撇号符号）+ 带 2 个前导参数的目标。
const UNARY_LIB: &str = "\
def Set (α : Type) : Type := α -> Prop\n\
axiom Set.union : (α : Type) -> Set α -> Set α -> Set α\n\
axiom Set.compl : (α : Type) -> Set α -> Set α\n\
axiom Set.powerset : (α : Type) -> Set α -> Set (Set α)\n\
axiom Set.image : (α : Type) -> (β : Type) -> (α -> β) -> Set α -> Set β\n";

#[test]
fn prefix_postfix_and_pointful_spellings_compile_identically() {
    let pointful = format!(
        "{UNARY_LIB}\
         def p (α : Type) (A : Set α) : Set (Set α) := Set.powerset α A\n\
         def c (α : Type) (A : Set α) : Set α := Set.compl α A\n\
         def i (α β : Type) (f : α -> β) (A : Set α) : Set β := Set.image α β f A\n"
    );
    let notation = format!(
        "{UNARY_LIB}\
         prefix:100 \" 𝒫 \" => Set.powerset\n\
         postfix:100 \" ᶜ \" => Set.compl\n\
         infixr:80 \" '' \" => Set.image\n\
         def p (α : Type) (A : Set α) : Set (Set α) := 𝒫 A\n\
         def c (α : Type) (A : Set α) : Set α := Aᶜ\n\
         def i (α β : Type) (f : α -> β) (A : Set α) : Set β := f '' A\n"
    );
    let pointful_out = compile_ok(&pointful);
    let notation_out = compile_ok(&notation);
    assert_eq!(
        event_kinds(&pointful_out),
        event_kinds(&notation_out),
        "the unary spellings must produce the same events"
    );
    assert!(
        notation_out
            .events
            .iter()
            .any(|e| matches!(e, CheckEvent::DeclarationChecked { name } if name == "i")),
        "the two-leading-parameter target must be completed: {:?}",
        notation_out.events
    );
}

#[test]
fn prefix_precedence_decides_where_the_operand_stops() {
    // `prefix:N` 的操作数按 `parse_operators(N)` 解析 ⇒ N 越大绑得越紧。
    // 用 `by rfl` 钉分组：`Eq … := by rfl` 只在两边真的同一个项时通过。
    let src = format!(
        "{UNARY_LIB}\
         infixl:65 \" ∪ \" => Set.union\n\
         prefix:100 \" 𝒫 \" => Set.powerset\n\
         theorem tight (α : Type) (A B : Set α) :\
             Eq.{{1}} (Set (Set α)) (𝒫 A ∪ 𝒫 B) (Set.union (Set α) (𝒫 A) (𝒫 B)) := by rfl\n"
    );
    let out = compile_ok(&src);
    assert!(
        out.events
            .iter()
            .any(|e| matches!(e, CheckEvent::DeclarationChecked { name } if name == "tight")),
        "𝒫 at 100 must bind tighter than ∪ at 65: {:?}",
        out.events
    );
}

#[test]
fn postfix_precedence_decides_where_it_binds() {
    let src = format!(
        "{UNARY_LIB}\
         infixl:65 \" ∪ \" => Set.union\n\
         postfix:100 \" ᶜ \" => Set.compl\n\
         theorem right (α : Type) (A B : Set α) :\
             Eq.{{1}} (Set α) (A ∪ Bᶜ) (Set.union α A (Set.compl α B)) := by rfl\n\
         theorem left (α : Type) (A B : Set α) :\
             Eq.{{1}} (Set α) (Aᶜ ∪ B) (Set.union α (Set.compl α A) B) := by rfl\n"
    );
    let out = compile_ok(&src);
    for name in ["right", "left"] {
        assert!(
            out.events
                .iter()
                .any(|e| matches!(e, CheckEvent::DeclarationChecked { name: n } if n == name)),
            "`{name}` must group the way the precedence table says: {:?}",
            out.events
        );
    }
}

#[test]
fn a_loose_postfix_binds_outside_the_binary_operator() {
    // 同一个符号换成 `postfix:50`（比 `∪` 的 65 松）⇒ `A ∪ Bᶜ` 读成
    // `(A ∪ B)ᶜ`。这条把"N 越大绑得越紧"从两个方向钉住。
    let src = format!(
        "{UNARY_LIB}\
         infixl:65 \" ∪ \" => Set.union\n\
         postfix:50 \" ᶜ \" => Set.compl\n\
         theorem loose (α : Type) (A B : Set α) :\
             Eq.{{1}} (Set α) (A ∪ Bᶜ) (Set.compl α (Set.union α A B)) := by rfl\n"
    );
    let out = compile_ok(&src);
    assert!(
        out.events
            .iter()
            .any(|e| matches!(e, CheckEvent::DeclarationChecked { name } if name == "loose")),
        "a postfix looser than ∪ must wrap the whole sum: {:?}",
        out.events
    );
}

#[test]
fn unary_notation_emits_no_events_and_is_not_a_declaration() {
    // N6 继续有效：一元记法命令也不产生事件、不进声明表。
    let src = format!(
        "{UNARY_LIB}\
         prefix:100 \" 𝒫 \" => Set.powerset\n\
         postfix:100 \" ᶜ \" => Set.compl\n"
    );
    let file = parse(&src).expect("parse");
    let out = compile_fol(&file);
    assert_eq!(out.errors, vec![]);
    let declarations = out
        .events
        .iter()
        .filter(|e| matches!(e, CheckEvent::DeclarationChecked { .. }))
        .count();
    assert_eq!(
        declarations, 5,
        "only the five axioms are declarations: {:?}",
        out.events
    );
    let report = check_document(&file);
    assert_eq!(
        report.decls.len(),
        5,
        "notation commands are not declarations"
    );
}

#[test]
fn unary_notation_with_two_leading_parameters_is_completed_from_the_operands() {
    // `Set.image : (α) → (β) → (α → β) → Set α → Set β`：两个前导参数都只能
    // 从操作数解出（`α` 来自 `Set α` 操作数、`β` 来自 `f` 的箭头陪域）。
    // 这条同时是**下溢回归**：`j - missing` 在 `missing == 2` 时曾经 panic
    // （第一刀只测过 `missing == 1`）。
    let src = format!(
        "{UNARY_LIB}\
         infixr:80 \" '' \" => Set.image\n\
         def i (α β : Type) (f : α -> β) (A : Set α) : Set β := f '' A\n"
    );
    let out = compile_ok(&src);
    assert!(
        out.events
            .iter()
            .any(|e| matches!(e, CheckEvent::DeclarationChecked { name } if name == "i")),
        "both leading parameters must be solved: {:?}",
        out.events
    );
}

#[test]
fn rfl_on_a_notation_goal_keeps_its_grouping() {
    // 回归：`rfl` 候选文本 `Eq.refl.{1} α a` 里的 `a` 是**原子位**，记法操作数
    // 必须带括号——否则 `Eq.refl.{1} (Set α) Aᶜ ∪ B` 会被读成
    // `(Eq.refl.{1} (Set α) Aᶜ) ∪ B`（第二刀实测：记法操作数上的 `by rfl` 全红）。
    let src = format!(
        "{UNARY_LIB}\
         infixl:65 \" ∪ \" => Set.union\n\
         postfix:100 \" ᶜ \" => Set.compl\n\
         theorem t (α : Type) (A B : Set α) :\
             Eq.{{1}} (Set α) (Aᶜ ∪ B) (Set.union α (Set.compl α A) B) := by rfl\n"
    );
    let out = compile_ok(&src);
    assert!(
        out.events
            .iter()
            .any(|e| matches!(e, CheckEvent::DeclarationChecked { name } if name == "t")),
        "the rfl candidate must keep the notation operand parenthesised: {:?}",
        out.events
    );
}

#[test]
fn a_by_block_whose_goal_carries_notation_still_judges() {
    // 回归（第一刀就有的边界）：`by` 块的目标文本走 `render_expr` +
    // `parse_expr_text` 往返，重解析必须认识记法——判卷通道现在从**前缀源码**
    // 里收记法声明（`judge.rs`）。
    let src = format!(
        "{UNARY_LIB}\
         infixl:65 \" ∪ \" => Set.union\n\
         postfix:100 \" ᶜ \" => Set.compl\n\
         theorem t (α : Type) (A B : Set α) :\
             Eq.{{1}} (Set α) (Aᶜ ∪ B) (Aᶜ ∪ B) ->\
             Eq.{{1}} (Set α) (Aᶜ ∪ B) (Aᶜ ∪ B) := by\n\
             intro h2\n\
             exact h2\n"
    );
    let file = parse(&src).expect("parse");
    let out = compile_fol(&file);
    assert_eq!(
        out.errors,
        vec![],
        "a by block over a notation goal must judge: {:?}",
        out.errors
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// 第三刀（G-04 剩余项，`docs/design/notation-subset.md` §12）
// ═══════════════════════════════════════════════════════════════════════════

/// 第三刀的课程形状夹具：集合 + `Exists`（真归纳，与 `lib/Exists` 同形）。
const THIRD_CUT_LIB: &str = "\
def Set (α : Type) : Type := α -> Prop\n\
def Set.mem (α : Type) (a : α) (A : Set α) : Prop := A a\n\
def Set.singleton (α : Type) (a : α) : Set α := fun (x : α) => Eq.{1} α x a\n\
def Set.pair (α : Type) (a b : α) : Set α := fun (x : α) => Or (Eq.{1} α x a) (Eq.{1} α x b)\n\
inductive Exists (A : Type) (p : A -> Prop) : Prop\n\
ctor intro (w : A) (h : p w) : Exists A p\n\
end\n";

fn third_cut_codes(src: &str) -> Vec<&'static str> {
    let file = parse(src).unwrap_or_else(|e| panic!("parse: {e:?}\n{src}"));
    compile_fol(&file).errors.iter().map(|e| e.code()).collect()
}

/// §12.4：`{a}` / `{a, b}` 展开成点名形式 `Set.singleton α a` / `Set.pair α a b`，
/// 与点名写法**事件序列逐一相等**。
#[test]
fn set_literals_expand_to_the_pointful_singleton_and_pair() {
    let pointful = format!(
        "{THIRD_CUT_LIB}\
         def one (α : Type) (a : α) : Set α := Set.singleton α a\n\
         def two (α : Type) (a b : α) : Set α := Set.pair α a b\n"
    );
    let literals = format!(
        "{THIRD_CUT_LIB}\
         def one (α : Type) (a : α) : Set α := {{a}}\n\
         def two (α : Type) (a b : α) : Set α := {{a, b}}\n"
    );
    let pointful_out = compile_ok(&pointful);
    let literals_out = compile_ok(&literals);
    assert_eq!(
        event_shapes(&pointful_out),
        event_shapes(&literals_out),
        "`{{a}}` / `{{a, b}}` must expand to the pointful spelling"
    );
}

/// §12.4：没有 `Set.singleton` 的文件报**专用**诊断（hint 教 import 或点名）。
#[test]
fn a_set_literal_without_the_library_is_a_dedicated_diagnostic() {
    let codes = third_cut_codes("#check {1}\n");
    assert_eq!(codes, vec!["elab-set-literal-unknown-target"], "{codes:?}");
}

/// §12.4：元素个数由**期望类型**解出（`{∅}` 这类嵌套记法走这条路）。
#[test]
fn a_set_literal_solves_the_element_type_from_the_expected_type() {
    let src = format!(
        "{THIRD_CUT_LIB}\
         notation \"∅\" => Set.empty\n\
         def Set.empty (α : Type) : Set α := fun (x : α) => False\n\
         def e (α : Type) : Set (Set α) := {{∅}}\n"
    );
    let out = compile_ok(&src);
    assert!(
        out.events
            .iter()
            .any(|e| matches!(e, CheckEvent::DeclarationChecked { name } if name == "e")),
        "the nested nullary notation must be solved from the expected type: {:?}",
        out.events
    );
}

/// §12.1：`∃ (x : A), p` 与点名 `Exists A (fun (x : A) => p)` 判卷一致。
/// 记法表示的是**命题本身**（`∃ x, p : Prop`），所以两边都写在类型位。
#[test]
fn binder_notation_expands_to_the_pointful_exists() {
    let pointful = format!(
        "{THIRD_CUT_LIB}\
         def p : Prop := Exists Nat (fun (n : Nat) => Eq.{{1}} Nat n n)\n"
    );
    let notation = format!(
        "{THIRD_CUT_LIB}\
         binder_notation \"∃\" => Exists\n\
         def p : Prop := ∃ (n : Nat), Eq.{{1}} Nat n n\n"
    );
    let pointful_out = compile_ok(&pointful);
    let notation_out = compile_ok(&notation);
    assert_eq!(
        event_shapes(&pointful_out),
        event_shapes(&notation_out),
        "`∃ (n : Nat), …` must expand to the pointful `Exists` application"
    );
}

/// §12.1：一段式的 binder **必须**带类型标注——本子集不引入元变量与一般合一
/// （与语言里既有的 `∀ x, p` 同规则），裸 `∃ x, p` 报专用诊断。
#[test]
fn a_one_stage_binder_without_an_annotation_is_a_dedicated_diagnostic() {
    let src = format!(
        "{THIRD_CUT_LIB}\
         binder_notation \"∃\" => Exists\n\
         def p : Prop := ∃ n, Eq.{{1}} Nat n n\n"
    );
    let codes = third_cut_codes(&src);
    assert_eq!(codes, vec!["elab-binder-notation-unsolved"], "{codes:?}");
}

/// §12.1：两段式 `∃ x ∈ s, p` = `Exists A (fun (x : A) => And (x ∈ s) p)`，
/// binder 的类型由 `∈` 反解；`∀ x ∈ s, p` = `∀ x, x ∈ s -> p`。
#[test]
fn two_stage_binders_expand_to_guard_and_implication() {
    let src = format!(
        "{THIRD_CUT_LIB}\
         infix:50 \" ∈ \" => Set.mem\n\
         binder_notation \"∃\" => Exists\n\
         def all (α : Type) (s : Set α) (p : α -> Prop) : Prop :=\n\
           ∀ x ∈ s, p x\n\
         def some (α : Type) (s : Set α) (p : α -> Prop) : Prop :=\n\
           ∃ x ∈ s, p x\n"
    );
    let pointful = format!(
        "{THIRD_CUT_LIB}\
         def all (α : Type) (s : Set α) (p : α -> Prop) : Prop :=\n\
           forall (x : α), Set.mem α x s -> p x\n\
         def some (α : Type) (s : Set α) (p : α -> Prop) : Prop :=\n\
           Exists α (fun (x : α) => And (Set.mem α x s) (p x))\n"
    );
    let notation_out = compile_ok(&src);
    let pointful_out = compile_ok(&pointful);
    assert_eq!(
        event_shapes(&pointful_out),
        event_shapes(&notation_out),
        "the two-stage binders must expand to the pointful guard forms"
    );
}

/// §12.1：两段式里反解不出 binder 类型 ⇒ **专用**诊断（不猜）。
#[test]
fn a_two_stage_binder_without_a_solvable_guard_is_a_dedicated_diagnostic() {
    let src = format!(
        "{THIRD_CUT_LIB}\
         binder_notation \"∃\" => Exists\n\
         axiom rel (β : Type) (x : β) (n : Nat) : Prop\n\
         infix:50 \" ≈ \" => rel\n\
         def some (n : Nat) (p : Nat -> Prop) : Prop :=\n\
           ∃ x ≈ n, p x\n"
    );
    let codes = third_cut_codes(&src);
    assert_eq!(codes, vec!["elab-binder-notation-unsolved"], "{codes:?}");
}

/// §12.2：同符号同形状的重复声明 = **重载**，按期望类型选候选。
#[test]
fn notation_overloads_pick_the_candidate_whose_result_matches_the_expected_type() {
    let src = format!(
        "{THIRD_CUT_LIB}\
         def Bag (α : Type) : Type := α -> Prop\n\
         def Bag.singleton (α : Type) (a : α) : Bag α := fun (x : α) => Eq.{{1}} α x a\n\
         prefix:100 \" ι \" => Set.singleton\n\
         prefix:100 \" ι \" => Bag.singleton\n\
         def s (α : Type) (a : α) : Set α := ι a\n\
         def b (α : Type) (a : α) : Bag α := ι a\n"
    );
    let out = compile_ok(&src);
    for name in ["s", "b"] {
        assert!(
            out.events
                .iter()
                .any(|e| matches!(e, CheckEvent::DeclarationChecked { name: n } if n == name)),
            "`{name}` must check through the overload: {:?}",
            out.events
        );
    }
}

/// §12.2：选不出（≥2 个候选都说得通）⇒ `elab-notation-ambiguous` + 人话 hint。
#[test]
fn an_ambiguous_overload_is_a_dedicated_diagnostic() {
    let src = format!(
        "{THIRD_CUT_LIB}\
         def Bag (α : Type) : Type := α -> Prop\n\
         def Bag.singleton (α : Type) (a : α) : Bag α := fun (x : α) => Eq.{{1}} α x a\n\
         prefix:100 \" ι \" => Set.singleton\n\
         prefix:100 \" ι \" => Bag.singleton\n\
         #check ι 1\n"
    );
    let file = parse(&src).expect("parse");
    let out = compile_fol(&file);
    let codes: Vec<&str> = out.errors.iter().map(|e| e.code()).collect();
    assert_eq!(codes, vec!["elab-notation-ambiguous"], "{:?}", out.errors);
    assert!(
        out.errors[0].hint().contains("点名形式"),
        "the hint must teach the pointful spelling: {}",
        out.errors[0].hint()
    );
}

/// §12.2：一个候选都对不上期望类型 ⇒ `elab-notation-no-candidate`（消息列出
/// 每个候选的结果类型）。
#[test]
fn an_overload_with_no_matching_candidate_is_a_dedicated_diagnostic() {
    let src = format!(
        "{THIRD_CUT_LIB}\
         def Bag (α : Type) : Type := α -> Prop\n\
         def Bag.singleton (α : Type) (a : α) : Bag α := fun (x : α) => Eq.{{1}} α x a\n\
         prefix:100 \" ι \" => Set.singleton\n\
         prefix:100 \" ι \" => Bag.singleton\n\
         def bad (α : Type) (a : α) : Nat := ι a\n"
    );
    let file = parse(&src).expect("parse");
    let out = compile_fol(&file);
    let codes: Vec<&str> = out.errors.iter().map(|e| e.code()).collect();
    assert_eq!(
        codes,
        vec!["elab-notation-no-candidate"],
        "{:?}",
        out.errors
    );
    assert!(
        out.errors[0].message.contains("Set.singleton")
            && out.errors[0].message.contains("Bag.singleton"),
        "the message must list every candidate: {}",
        out.errors[0].message
    );
}

/// §12.2：同一符号的**不同形状**（这里 `notation` vs `infix`）仍然是错误。
#[test]
fn two_shapes_on_one_symbol_are_still_a_parse_error() {
    let file = parse("prefix:100 \" ι \" => Set.singleton\ninfix:50 \" ι \" => Set.mem\n");
    let err = file.expect_err("two shapes on one symbol");
    assert_eq!(err.code(), "notation-shape");
    assert!(err.message.contains("形状"), "message: {}", err.message);
}

/// §12.3：`scoped` 记法默认**不生效**（未 `open scoped` 前用它是
/// `notation-unknown-symbol`），`open scoped Foo` 之后生效。
#[test]
fn scoped_notation_needs_open_scoped() {
    let inactive = format!(
        "{THIRD_CUT_LIB}\
         namespace Foo\n\
         scoped infix:50 \" ∈ \" => Set.mem\n\
         end Foo\n\
         def p (α : Type) (a : α) (A : Set α) : Prop := a ∈ A\n"
    );
    let err = parse(&inactive).expect_err("a scoped notation is inactive by default");
    assert_eq!(
        err.code(),
        "notation-unknown-symbol",
        "message: {}",
        err.message
    );

    let active = format!(
        "{THIRD_CUT_LIB}\
         namespace Foo\n\
         scoped infix:50 \" ∈ \" => Set.mem\n\
         end Foo\n\
         open scoped Foo\n\
         def p (α : Type) (a : α) (A : Set α) : Prop := a ∈ A\n"
    );
    let out = compile_ok(&active);
    assert!(
        out.events
            .iter()
            .any(|e| matches!(e, CheckEvent::DeclarationChecked { name } if name == "p")),
        "`open scoped Foo` must activate the notation: {:?}",
        out.events
    );
}

/// §12.3：`open scoped` **不**打开名字前缀（与 `open` 区分）。
#[test]
fn open_scoped_does_not_open_the_namespace_for_names() {
    let src = format!(
        "{THIRD_CUT_LIB}\
         namespace Foo\n\
         def bar : Prop := True\n\
         end Foo\n\
         open scoped Foo\n\
         def p : Prop := bar\n"
    );
    let file = parse(&src).expect("parse");
    let out = compile_fol(&file);
    let codes: Vec<&str> = out.errors.iter().map(|e| e.code()).collect();
    assert_eq!(codes, vec!["elab-unknown-identifier"], "{:?}", out.errors);
}

/// §12.3：`scoped` 写在 `namespace` 外面是**专用** parse 错。
#[test]
fn scoped_outside_a_namespace_is_a_parse_error() {
    let err = parse("scoped infix:50 \" ∈ \" => Set.mem\n").expect_err("scoped needs a namespace");
    assert_eq!(err.code(), "notation-shape");
    assert!(
        err.message.contains("namespace"),
        "message: {}",
        err.message
    );
}

/// §12.5：前缀记法在实参位免括号后，**点名形式与括号形式都照旧**。
#[test]
fn a_prefix_notation_argument_keeps_the_parenthesised_spelling_working() {
    let src = format!(
        "{THIRD_CUT_LIB}\
         def Set.powerset (α : Type) (A : Set α) : Set (Set α) := fun (B : Set α) => True\n\
         prefix:100 \" 𝒫 \" => Set.powerset\n\
         def f (α : Type) (X : Set (Set α)) : Prop := True\n\
         def a (α : Type) (A : Set α) : Prop := f α (𝒫 A)\n\
         def b (α : Type) (A : Set α) : Prop := f α 𝒫 A\n"
    );
    let out = compile_ok(&src);
    for name in ["a", "b"] {
        assert!(
            out.events
                .iter()
                .any(|e| matches!(e, CheckEvent::DeclarationChecked { name: n } if n == name)),
            "`{name}` must check: {:?}",
            out.events
        );
    }
}

/// §12.3 跨 `import`：被导入模块声明的 `scoped` 记法**挂起**，入口要自己
/// `open scoped <作用域名>` 才生效（继承表 + 作用域栈的合成行为）。
#[test]
fn an_inherited_scoped_notation_needs_the_entrys_own_open_scoped() {
    let library = "namespace Foo\n\
                   scoped infix:50 \" ∈ \" => Set.mem\n\
                   end Foo\n";
    let library_file = parse(&format!(
        "def Set (α : Type) : Type := α -> Prop\n\
         def Set.mem (α : Type) (a : α) (A : Set α) : Prop := A a\n\
         {library}"
    ))
    .expect("parse library");
    let inherited: Vec<crate::ast::NotationDecl> = library_file
        .commands
        .iter()
        .filter_map(|command| command.notation_decl())
        .collect();
    assert_eq!(inherited.len(), 1, "the library declares one notation");
    assert_eq!(inherited[0].scope.as_deref(), Some("Foo"));

    let entry = "def p (α : Type) (a : α) (A : Set α) : Prop := a ∈ A\n";
    let err = crate::parser::parse_with_inherited(entry, &inherited)
        .expect_err("an inherited scoped notation is suspended");
    assert_eq!(err.code(), "notation-unknown-symbol");

    let opened = format!("open scoped Foo\n{entry}");
    crate::parser::parse_with_inherited(&opened, &inherited)
        .expect("`open scoped Foo` activates the inherited notation");
}

/// **静音窗口必须是线程局部的**（2026-09-23，`docs/CI-FAILURES.md`）。
///
/// 背景：`quiet_catch` / `resolve_hovers` 要把内核 panic 转成 `Err`，所以它们
/// 暂时**不打** panic 日志。旧实现是进程全局的 `take_hook`/`set_hook`，而自
/// T-A30 起编译跑在**后台任务**里、多份文档的编译可以并发 ⇒ A 线程静音期间
/// B 线程的 panic 也会被吞掉。CI 上
/// `perf_project_dependency_edit_refreshes_dependents` 与
/// `editing_a_dependency_refreshes_the_open_entry` 因此「FAILED 但日志里连一句
/// `panicked` 都没有」，整轮排查只能靠猜。
///
/// 判据两条：① 别的线程看到的层数是 0（它的 panic 照常打印）；② 可嵌套、离开
/// 作用域（含 unwind）自动恢复——否则一次 panic 会让**这个线程此后的所有 panic
/// 全部失声**。
#[test]
fn the_quiet_panic_window_is_thread_local_and_nests() {
    assert_eq!(super::check::quiet_depth(), 0, "起点不静音");
    {
        let _outer = super::check::quiet();
        assert_eq!(super::check::quiet_depth(), 1);
        assert!(
            super::check::panic_is_quiet(),
            "本线程在静音区里：hook 应当吞掉这条 panic"
        );
        let (other_depth, other_quiet) =
            std::thread::spawn(|| (super::check::quiet_depth(), super::check::panic_is_quiet()))
                .join()
                .expect("the probe thread joins");
        assert_eq!(other_depth, 0, "别的线程不在静音区里");
        assert!(
            !other_quiet,
            "**这条是本测试的要害**：静音只对本线程生效——别的线程的 panic 必须照常可见\
             （旧实现是进程全局的 take_hook/set_hook，会把并发线程的 panic 一起吞掉）"
        );
        let caught = super::check::quiet_catch(|| panic!("boom"));
        assert!(caught.is_err(), "panic 仍然被转成 Err（契约不变）");
        assert_eq!(
            super::check::quiet_depth(),
            1,
            "内层退出后回到外层，而不是把整个线程弄哑"
        );
    }
    assert_eq!(super::check::quiet_depth(), 0, "离开作用域自动恢复");
}

/// **A3 的根因判据**（2026-09-26 用户报告第 3 条）：**开放练习的签名**也必须
/// 有 hover 行。
///
/// 为什么这条比"`{a}` 能不能跳"更根本：学生手里的文件**绝大多数**是没解出来的
/// （值位是 `sorry`）⇒ 如果签名一条 hover 行都没有，那么 hover / F12 / 高亮 /
/// 引用在**学生最常待的地方全部失效**。实测（修之前）：整份报告的 hover 行
/// **最大结束偏移 = 113**，而 `theorem sing_eq …` 的签名从 **116** 起 ——
/// 一条都没有。根因：`walk.rs::open_signature` 把签名 hover 收进一个**局部**
/// `Vec` 然后丢掉。
#[test]
fn an_open_declaration_records_hovers_for_its_signature() {
    let src = "def Set (\u{3b1} : Type) : Type := \u{3b1} -> Prop\n\
def Set.singleton (\u{3b1} : Type) (a : \u{3b1}) : Set \u{3b1} := fun (x : \u{3b1}) => x = a\n\
theorem sing_eq (\u{3b1} : Type) (a : \u{3b1}) : Set.singleton \u{3b1} a = {a} := sorry\n";
    let file = parse(src).expect("parse");
    let report = check_document(&file);
    assert!(
        report
            .decls
            .iter()
            .any(|d| d.name.as_deref() == Some("sing_eq") && d.status == DeclStatus::Open),
        "夹具前提：`sing_eq` 必须是开放练习（`sorry`），实际 = {:?}",
        report
            .decls
            .iter()
            .map(|d| (&d.name, &d.status))
            .collect::<Vec<_>>()
    );
    let sig_start = src.find("theorem sing_eq").expect("decl start");
    assert!(
        report
            .hovers
            .iter()
            .any(|h| h.span.start.offset >= sig_start),
        "开放练习的**签名**必须有 hover 行（A3 之前一条都没有；最大的结束偏移 = {:?}）",
        report.hovers.iter().map(|h| h.span.end.offset).max()
    );
    // 而且**集合字面量**那一行要指向它的展开目标（A3 的正题）。
    let lit = src.find("{a}").expect("set literal");
    assert!(
        report.hovers.iter().any(|h| {
            h.span.start.offset <= lit
                && lit < h.span.end.offset
                && matches!(
                    &h.resolution,
                    Some(ResolvedTarget::Declaration { name, .. }) if name == "Set.singleton"
                )
        }),
        "`{{a}}` 必须记一条指向 `Set.singleton` 的 hover（A3 的判据）"
    );
}

/// **R5 判据**（A5，2026-09-26 用户要求）：**集合字面量 / 零元记法嵌套在记法里**
/// 时，前导类型参数必须解得出来。
///
/// 这是 `courses/set-theory/units/unit12-synthesis.sokonanoda` 里那 **5 个**
/// `-- soko:notation-ok: R5` 标记的根因 —— 学习者被迫把整条式子写成点名形式
/// （`Set.singleton (Set Nat) ∅` …）✗。
///
/// 病灶是 `elab.rs::solve_prefix_args` 的**两条路线都缺 delta 展开**
/// （`implicit::solve_prefix` 早就有，记法这条没有）：
/// * **路线 ①**（由实参类型反解）：`Set.inter` 的 α 层域是 `Set α`（`Set` 是
///   **def**），而操作数的类型文本是**箭头形态**（`Set (Set Nat)` 的 pp 就是
///   `Set Nat -> Prop`）⇒ `App(Set, α)` 与 `Arrow{…}` **头对不上** ⇒
///   `elab-notation-argument-unsolved`；
/// * **路线 ②**（由期望类型反解）：`∅` 的模板是 `Set α`，期望是 `Nat -> Prop`
///   （= `Set Nat`）⇒ 同样头对不上 ⇒ 「记法 `∅` 展开成 `Set.empty` 时补不出
///   前面的类型参数」。
///
/// 修法是**只加解、不改既有解**：先按原样试，失败才把**模板侧**（必要时还有
/// 实参/期望侧）δ 展开再试。
///
/// **反向验证**：把两处兜底删掉 ⇒ 本判据当场判红（实测：`elab-notation-argument-unsolved`）。
#[test]
fn notation_solves_leading_parameters_when_the_expected_type_is_an_arrow() {
    let src = "\
axiom Exists (\u{3b1} : Type) (p : \u{3b1} -> Prop) : Prop\n\
def Set (\u{3b1} : Type) : Type := \u{3b1} -> Prop\n\
def Set.empty (\u{3b1} : Type) : Set \u{3b1} := fun (x : \u{3b1}) => False\n\
def Set.univ (\u{3b1} : Type) : Set \u{3b1} := fun (x : \u{3b1}) => True\n\
def Set.singleton (\u{3b1} : Type) (a : \u{3b1}) : Set \u{3b1} := fun (x : \u{3b1}) => x = a\n\
def Set.inter (\u{3b1} : Type) (A B : Set \u{3b1}) : Set \u{3b1} := fun (x : \u{3b1}) => A x \u{2227} B x\n\
def Set.image (\u{3b1} \u{3b2} : Type) (f : \u{3b1} -> \u{3b2}) (A : Set \u{3b1}) : Set \u{3b2} := fun (y : \u{3b2}) => Exists \u{3b1} (fun (x : \u{3b1}) => f x = y)\n\
notation \"\u{2205}\" => Set.empty\n\
infixl:70 \" \u{2229} \" => Set.inter\n\
infixr:80 \" '' \" => Set.image\n\
theorem r5 :\n\
    (fun (_ : Set Nat) => Set.empty Nat) '' ({\u{2205}} \u{2229} {(Set.univ Nat)})\n\
      \u{2260} ((fun (_ : Set Nat) => Set.empty Nat) '' {\u{2205}}) \u{2229} ((fun (_ : Set Nat) => Set.empty Nat) '' {(Set.univ Nat)}) := by\n\
  sorry\n";
    let out = compile_ok(src);
    assert!(
        out.events
            .iter()
            .any(|e| matches!(e, CheckEvent::ExerciseOpen { name: Some(n) } if n == "r5")),
        "R5 家族必须解得出来（修前是 elab-notation-argument-unsolved）：{:?}",
        out.events
    );
}

/// **A4 判据（真相层，2026-09-26 用户报告第 4 条）**：prelude 的源文本与它的
/// span 表必须**同源** —— 每个在源里有文字的 prelude 名字，`prelude_def_span`
/// 都要指向**那一行**（不是"返回了个 span 就算" ✗：要**读回那一行**、断言它确实
/// 在声明这个名字 ✓）。
///
/// 另一半同样重要：**没有源文字的名字要正好是已知的那 9 个**（`Nat`/`Bool` 家族
/// 是 Rust AST 手搓的，**今天确实没有定义位置**）⇒ 谁给它们补上源文字，这条会
/// 提醒他把名字从"无源"名单里划掉 ✓（否则 F12 会静默继续跳不了）。
#[test]
fn prelude_source_and_its_span_table_agree() {
    let src = crate::compile::prelude_source();
    assert!(
        src.starts_with(crate::compile::PRELUDE_EQ_SRC),
        "前奏源必须以 Eq 三件套开头（与喂进编译的是同一份字节 ✓）"
    );
    assert!(
        src.contains(crate::compile::PRELUDE_L1_SRC),
        "前奏源必须**逐字包含** L1 源文本"
    );

    let mut with_source: Vec<&str> = Vec::new();
    let mut without_source: Vec<&str> = Vec::new();
    for name in crate::compile::PRELUDE_NAMES {
        match crate::compile::prelude_def_span(name) {
            Some(span) => {
                with_source.push(name);
                let start = span.start.offset;
                let end = span.end.offset;
                assert!(
                    start < end && end <= src.len(),
                    "`{name}` 的 span 越界：{span:?}"
                );
                let line_start = src[..start].rfind('\n').map(|i| i + 1).unwrap_or(0);
                let line_end = src[start..]
                    .find('\n')
                    .map(|i| start + i)
                    .unwrap_or(src.len());
                let line = &src[line_start..line_end];
                assert!(
                    !line.trim_start().starts_with("--"),
                    "`{name}` 的 span 落在注释上：{line:?}"
                );
                // 派生名（`And.rec` / `Or.rec`）退到**所属归纳块**的块头行
                // ⇒ 那一行里有的是块名（`And`），不是 `rec` ✓。
                let last = name.rsplit('.').next().unwrap_or(name);
                let head = name.rsplit_once('.').map(|(h, _)| h).unwrap_or(name);
                let head_last = head.rsplit('.').next().unwrap_or(head);
                assert!(
                    line.contains(last) || line.contains(head_last),
                    "`{name}` 的 span 必须指向它的**声明行**（或所属归纳块的块头行），实际 = {line:?}"
                );
            }
            None => without_source.push(name),
        }
    }
    assert!(
        with_source.len() >= 30,
        "有源文字的 prelude 名字太少了（{} 个）：F12 会大面积跳不了",
        with_source.len()
    );
    assert_eq!(
        without_source,
        vec![
            "Nat",
            "Nat.zero",
            "Nat.succ",
            "Nat.rec",
            "Nat.add",
            "Bool",
            "Bool.true",
            "Bool.false",
            "Bool.rec",
        ],
        "无源文字的名字必须正好是 Nat/Bool 家族（手搓 AST，今天确实没有定义位置）"
    );
}

/// **A0 的"立判据"组**（T-N16，2026-09-26）：洞的期望类型（`sub_goals[].ty`）
/// 是**判定输入** ⇒ 真相字段**一个字节都不许折** ✗。
///
/// 为什么：`suggest.rs::hole_goal_text` 把它当 `OpenGoalSpec.ty` **回读**去算
/// exact/rfl 建议 —— 折了就是**改判定**（内核红线）。
///
/// ⚠ 这条判据**必须直接读真相字段**：显示副本（wire 的 `SubGoalInfo.ty` / LSP
/// hover 的「此处 `sorry` 的期望类型」）是**另做的克隆**、**已经折了** ✓
/// ⇒ `grade --json` 的"两态逐字节相同"在"建议**按需**算"的前提下**碰不到**
/// 这个字段，拿它当判据是**咬不住的守卫** ✗（本仓的纪律：守卫必须能咬住已知 bug）。
///
/// **判据断言的是字段本身、不是某一条路** ⇒ 两条产出路（源级 `instantiate_binder_type`
/// 与请求期内核探针 `probe_arg_type`）**任何一条**被折都会判红 ✓
/// （本夹具走源级那条 —— 实测它的 `ty` 由 `instantiate_binder_type` 给出；
/// 探针那条只在源级算不出时才填，本条判据不依赖它也能咬住 ✓）。
///
/// **反向验证**：在 `goals.rs` 的 `instantiate_binder_type` 产出处把 `->` 换成 `→`
/// ⇒ 当场判红（失败信息就是折过的那串 ✓）；还原 ⇒ 绿 ✓。
#[test]
fn hole_expected_type_is_judge_input_and_stays_raw() {
    let src = "\
def Set (\u{3b1} : Type) : Type := \u{3b1} -> Prop\n\
def Set.mem (\u{3b1} : Type) (a : \u{3b1}) (A : Set \u{3b1}) : Prop := A a\n\
infix:50 \" \u{2208} \" => Set.mem\n\
def Set.subset (\u{3b1} : Type) (A B : Set \u{3b1}) : Prop := forall (x : \u{3b1}), A x -> B x\n\
infix:50 \" \u{2286} \" => Set.subset\n\
axiom Set.ext (\u{3b1} : Type) (A B : Set \u{3b1}) : (\u{2200} (x : \u{3b1}), x \u{2208} A \u{2194} x \u{2208} B) -> A = B\n\
theorem hole_surface (\u{3b1} : Type) (A B : Set \u{3b1}) : A \u{2286} B -> A = B :=\n\
  Set.ext \u{3b1} A B sorry\n";
    let mut doc = crate::query::QueryDoc::new();
    doc.set_text(src, 1, None);
    let report = doc.probed_report();
    let decl = report
        .decls
        .iter()
        .find(|d| d.name.as_deref() == Some("hole_surface"))
        .unwrap_or_else(|| panic!("`hole_surface` 必须在报告里：{:?}", report.decls.len()));
    let raw = decl
        .sub_goals
        .first()
        .and_then(|s| s.ty.as_deref())
        .unwrap_or_else(|| panic!("洞必须有期望类型（含探针）：{:?}", decl.sub_goals));
    assert!(
        !raw.contains('\u{2192}'),
        "**真相字段不许做显示归一化**（`suggest.rs` 回读它算建议 ⇒ 折了就是改判定）：{raw}"
    );
    assert!(
        raw.contains("->"),
        "真相字段要保持**源级点形式**的箭头（它是 `render_expr` 的产物）：{raw}"
    );
    // **对照组**：同一个洞、同一份源 —— **显示副本必须折** ✓
    //（两条一起看才说明"折的是克隆、不是真相字段"）。
    let folded = doc.fold_display(raw);
    assert!(
        folded.contains('\u{2192}') && !folded.contains("->"),
        "显示副本必须折成 `\u{2192}`（A0/A1）：{folded}"
    );
}

/// **B3-① 判据**（缺口 G-40，2026-09-26）：签名带**前导隐式 binder** 的常量
/// **裸着写**（零实参）时，也要能从**期望类型**补出那些参数。
///
/// 为什么它是独立一条：`try_implicit_application` 只挂在 `Expr::App` 臂上，
/// 而 `∅` 展开成的是**光秃秃的 `Set.empty`** —— 它根本不是 `App` ✗ ⇒ 钩子永远
/// 够不着 ⇒ 词项停在 `{α : Type} → Set α` 那个 Pi 上 ⇒ `rfl` 判不出来 ✗、
/// 课程库也就没法把前导类型参数改成隐式 ✗（真库实测：改成 `{α}` 后课程门禁从
/// `328 checked · 0 判负` 掉到 `249 / 7`）。
///
/// **反向验证**：把 `Ident` 臂里那次 `try_bare_implicit_constant` 去掉 ⇒ 判据红 ✓。
#[test]
fn a_bare_constant_with_implicit_binders_takes_them_from_the_expected_type() {
    let src = "\
def Set (\u{3b1} : Type) : Type := \u{3b1} -> Prop\n\
def Set.mem {\u{3b1} : Type} (a : \u{3b1}) (A : Set \u{3b1}) : Prop := A a\n\
infix:50 \" \u{2208} \" => Set.mem\n\
def Set.empty {\u{3b1} : Type} : Set \u{3b1} := fun (x : \u{3b1}) => False\n\
notation \"\u{2205}\" => Set.empty\n\
theorem bare_constant (\u{3b1} : Type) (a : \u{3b1}) : (a \u{2208} \u{2205}) = False := by rfl\n";
    let out = compile_ok(src);
    assert!(
        out.events
            .iter()
            .any(|e| matches!(e, CheckEvent::DeclarationChecked { name } if name == "bare_constant")),
        "`∅`（裸 `Set.empty`）必须能从期望类型补出 α 并判绿（修前是 rfl 判红 / 词项停在 Pi 上）：{:?}",
        out.events
    );
}

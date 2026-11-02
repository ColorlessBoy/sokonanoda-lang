//! Regressions from the external Lean Kernel Arena corpus.
//!
//! The fixtures live in `test_resources/{NestedRecRuleNFields,UnsafeDef,
//! InductiveUndeclaredUniverse,NestedNameCapture,UnitRecRule}` and come from
//! `leanprover/lean-kernel-arena` (Apache-2.0, revision
//! `d4f15adc61e131de91b11a7bfa827045df86d193` for the first two,
//! `d7f74a9e67459e2c251d608f524dc5efc6dc223c` for the others); see each
//! fixture's README.
//!
//! These tests are deliberately *not* gated on `LEAN_KERNEL_ARENA`: they are
//! the self-contained reproducers for the behaviours the release gate depends
//! on (a lying nested iota rule is rejected; unsafe/partial definitions decline
//! instead of panicking; an inductive block may not mention a universe
//! parameter it never declared; an admitted iota rule may not capture a
//! block-local name; an admitted iota rule that differs from the reconstructed
//! one is rejected even when a unit-like type is involved).

use crate::util::{Config, Decline};
use std::panic::{self, AssertUnwindSafe};
use std::path::Path;
use stumpalo::Arena;

fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "panic".to_string()
    }
}

#[test]
fn nested_rec_rule_field_count_lie_is_rejected() {
    let cfg = Config::try_from(Path::new("test_resources/NestedRecRuleNFields/config.json"))
        .expect("fixture config");
    let arena = Arena::new();
    let (ef, _) = cfg.to_export_file(arena.as_arena_ref()).expect("parse export");
    let payload = panic::catch_unwind(AssertUnwindSafe(|| ef.check_all_declars()))
        .expect_err("a lying nested iota rule must be rejected");
    let msg = panic_message(payload);
    assert!(
        msg.starts_with("iota rule field count does not match the constructor telescope"),
        "expected a stable field-count rejection, got {msg:?}"
    );
}

#[test]
fn unsafe_definitions_decline_instead_of_panicking() {
    let cfg = Config::try_from(Path::new("test_resources/UnsafeDef/config.json"))
        .expect("fixture config");
    let arena = Arena::new();
    let err = match cfg.to_export_file(arena.as_arena_ref()) {
        Ok(_) => panic!("an unsafe definition must be declined"),
        Err(e) => e,
    };
    let decline = err
        .downcast_ref::<Decline>()
        .expect("must be a controlled decline, not a panic or a parse error");
    assert!(
        decline.0.contains("unsafe and partial definitions are not supported"),
        "unexpected decline message: {}",
        decline.0
    );
}

/// `bad/other/inductive-undeclared-universe`: the block's `levelParams` is `[]`
/// while `U`'s type is `Sort u`, so `u` is a level parameter the block never
/// declared. At `u := 0` that makes `U` a proposition with two proofs and lets
/// the export type `U.a` and `U.b` as the same `P.{0}`.
///
/// The guard that catches this is `infer`'s `all_uparams_defined` assertion on
/// the `Sort`/`Const` arms, which only runs when the checker has a
/// `DeclarInfo`. The block used to be checked with `None` (dead guard), so this
/// test is the in-repo reproducer for the fix: it fails before it and passes
/// after.
#[test]
fn inductive_type_may_not_mention_an_undeclared_universe_parameter() {
    let cfg = Config::try_from(Path::new("test_resources/InductiveUndeclaredUniverse/config.json"))
        .expect("fixture config");
    let arena = Arena::new();
    let (ef, _) = cfg.to_export_file(arena.as_arena_ref()).expect("parse export");
    let payload = panic::catch_unwind(AssertUnwindSafe(|| ef.check_all_declars()))
        .expect_err("an inductive block that mentions an undeclared universe parameter must be rejected");
    let msg = panic_message(payload);
    assert_eq!(msg, "universe parameter not declared by the current declaration");
}

/// `bad/bugs/nested-name-capture`: the exported `Shadow.rec` rule for
/// `Shadow.node` names `_nested.Cov_1.rec` / `_nested.Cov_1.mk`, declarations
/// that exist only in the nested-inductive block's temporary extension. The
/// file then defines a fresh `def _nested.Cov_1.rec` with the same name, which
/// hijacks the admitted rule and lets the export prove `False`.
///
/// Restoring (`_nested.Cov_1.rec ↦ Shadow.rec_1`) and comparing with `def_eq`
/// does not catch this: the block-local recursor computes the same value on the
/// same major premise, so both rules are convertible. This test is the in-repo
/// reproducer for the name check: it fails before it and passes after.
#[test]
fn nested_recursor_rule_may_not_name_a_block_local_constant() {
    let cfg = Config::try_from(Path::new("test_resources/NestedNameCapture/config.json"))
        .expect("fixture config");
    let arena = Arena::new();
    let (ef, _) = cfg.to_export_file(arena.as_arena_ref()).expect("parse export");
    let payload = panic::catch_unwind(AssertUnwindSafe(|| ef.check_all_declars()))
        .expect_err("a recursor rule that names a block-local constant must be rejected");
    let msg = panic_message(payload);
    assert_eq!(msg, "imported recursor rule refers to a block-local name");
}

/// `bad/bugs/unit-rec-rule` (LKA, K3): `PUnit.rec`'s exported iota rule for
/// `PUnit.unit` returns the *constructor* (`fun {motive} (unit) => PUnit.unit`)
/// instead of the minor premise (`… => unit`). With that rule `badR`'s type
/// reduces to the term `PUnit.unit`, so the file proves `False`.
///
/// `inductive.rs` already reconstructs the rule itself and compares it with
/// `def_eq`; that comparison was a **no-op** because our unit-like eta shortcut
/// returned `true` as soon as *one* side's type was unit-like (`PUnit.unit`
/// has type `PUnit`), so `def_eq(Prop, PUnit.unit)` also held. Lean 4
/// (`src/kernel/type_checker.cpp:1035-1047 is_def_eq_unit_like`) requires
/// `is_def_eq_core(t_type, infer_type(s))` before allowing the shortcut, and
/// this fixture pins that: it is accepted before the fix and rejected (at the
/// imported-rule comparison) after it.
#[test]
fn unit_like_eta_may_not_swallow_a_lying_recursor_rule() {
    let cfg = Config::try_from(Path::new("test_resources/UnitRecRule/config.json")).expect("fixture config");
    let arena = Arena::new();
    let (ef, _) = cfg.to_export_file(arena.as_arena_ref()).expect("parse export");
    let payload = panic::catch_unwind(AssertUnwindSafe(|| ef.check_all_declars()))
        .expect_err("an imported iota rule that differs from the reconstructed one must be rejected");
    let msg = panic_message(payload);
    assert_eq!(msg, "imported recursor rule does not match the reconstructed rule");
}

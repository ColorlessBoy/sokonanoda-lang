//! Regressions from the external Lean Kernel Arena corpus.
//!
//! The fixtures live in `test_resources/{NestedRecRuleNFields,UnsafeDef}` and
//! come from `leanprover/lean-kernel-arena` (Apache-2.0, revision
//! `d4f15adc61e131de91b11a7bfa827045df86d193`); see each fixture's README.
//!
//! These tests are deliberately *not* gated on `LEAN_KERNEL_ARENA`: they are
//! the self-contained reproducers for the two behaviours the release gate
//! depends on (a lying nested iota rule is rejected; unsafe/partial definitions
//! decline instead of panicking).

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

//! Imported iota rules: the "admitted rule == reconstructed rule" check, plus the
//! **name hygiene** check that def-eq cannot express (K2, 2026-11-02).
//!
//! ## Why this is not in `inductive.rs`
//!
//! `inductive.rs` is a ⑨ **only-shorter** file (baseline 2224 lines) ⇒ new code must
//! live elsewhere. Rust allows an `impl` block for the same type in another module, so
//! `assert_imported_expr_matches` moved here **with** the check it now performs.
//!
//! ## What the name check adds
//!
//! The imported (admitted) `RecRule::val` is stored in the environment exactly as the
//! export wrote it. If it names a constant that exists **only while the inductive block
//! is checked** (`_nested.X`, `_nested.X.mk`, `_nested.X.rec`, …), then after the block
//! that name is free to be defined by the file — which silently changes what the
//! admitted rule means. Lean 4 prevents this structurally: `inductive.cpp`'s
//! `restore_nested` rewrites such names in every rule it adds to the environment.
//!
//! Our kernel restores the rules it reconstructs (`_nested.X.rec ↦ Base.rec_N` in
//! `restore_recursor1`) and compares the two sides with `def_eq`. That comparison is
//! blind to the difference: the block-local recursor lives in the temporary extension
//! and computes the same value on the same major premise as the restored recursor, so
//! both sides are convertible (a single-constructor structure such as `Cov` lets
//! structure eta swap `a` for `Cov.mk a.0`). Hence this explicit **name** check.
//!
//! The predicate is exact, not a `_nested.`/prefix match: a constant is block-local iff
//! the temporary extension holds it **and** the persistent environment as of this
//! block's `EnvLimit` cutoff does not. The block's own imported
//! types/constructors/recursors are all in the persistent environment, so they are
//! never flagged; a name the file itself declared earlier stays legitimate.

use crate::tc::TypeChecker;
use crate::util::ExprPtr;

impl<'x, 't: 'x, 'p: 't> TypeChecker<'x, 't, 'p> {
    /// An admitted rule is stored as the export wrote it, so it may not name a
    /// constant that exists only inside this block's temporary extension.
    fn assert_imported_rule_has_no_block_local_names(&mut self, val: ExprPtr<'t>) {
        let env = self.env;
        assert!(
            !self.ctx.find_const(val, |n| env.get_temp_declar(&n).is_some() && env.get_old_declar(&n).is_none()),
            "imported recursor rule refers to a block-local name"
        );
    }

    /// Moved verbatim from `inductive.rs`, plus the name check (see the module docs).
    pub(crate) fn assert_imported_expr_matches(&mut self, imported: ExprPtr<'t>, reconstructed: ExprPtr<'t>) {
        self.assert_imported_rule_has_no_block_local_names(imported);
        self.tc_cache.clear();
        assert!(
            self.def_eq_core(imported, reconstructed),
            "imported recursor rule does not match the reconstructed rule"
        );
    }
}

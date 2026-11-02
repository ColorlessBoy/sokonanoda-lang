# InductiveUndeclaredUniverse

Regression fixture from `leanprover/lean-kernel-arena`
`tests/bad/other/inductive-undeclared-universe` (Apache-2.0), revision
`d7f74a9e67459e2c251d608f524dc5efc6dc223c` (2026-11-01).

The block declares `inductive U : Sort u` with `levelParams: []` — the level
parameter `u` is never declared by the block. At `u := 0` the type `U` is a
proposition with two distinct proofs, so the export then proves `P.{0} := U.a`
and `P.{0} := U.b` for the same `P`.

A sound kernel must reject it: every universe parameter that appears in an
inductive block (type, constructors, recursor) has to be declared by that block,
the same standard `def`/`theorem`/`axiom` are held to. Lean 4 rejects it in
`type_checker::check_level` (`invalid reference to undefined universe level
parameter 'u'`), reached from `inductive.cpp`'s `check_inductive_types` via
`tc().check(type, m_lparams)`.

The kernel's own guard for this lives in `infer.rs`'s `Sort`/`Const` arms
(`all_uparams_defined`); it is only armed when the checker is given a
`DeclarInfo`. Inductive blocks used to pass `None` (`util.rs::with_tc`), which
made the guard dead code for the block — the regression this fixture pins.

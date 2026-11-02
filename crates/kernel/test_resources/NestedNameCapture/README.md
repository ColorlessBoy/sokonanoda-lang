# NestedNameCapture

Regression fixture from `leanprover/lean-kernel-arena`
`tests/bad/bugs/nested-name-capture` (Apache-2.0), revision
`d7f74a9e67459e2c251d608f524dc5efc6dc223c` (2026-11-01).

`Shadow` is a nested inductive: `Shadow.node : Cov Shadow`, where `Cov` is an
ordinary structure. While checking the block the kernel creates temporary
declarations (`_nested.Cov_1`, `_nested.Cov_1.mk`, `_nested.Cov_1.rec`) that
exist only inside the block's environment extension. The export's `Shadow.rec`
rule for `Shadow.node` names exactly those temporaries:

```
fun (motive_1) (motive_2) (base) (node) (mk) (a : Cov Shadow) =>
  node a (_nested.Cov_1.rec … (_nested.Cov_1.mk (proj a).0))
```

After the block the file declares a **fresh** `def _nested.Cov_1.rec … :=
evil` with the same name. The admitted rule now denotes that definition instead
of the recursor, so `(fold sample).2` computes `evil : P B` for a false `P` and
the file proves `False`. A checker must not let an admitted rule refer to a
constant that existed only while the block was checked (Lean 4 rejects it
because `inductive.cpp`'s `restore_nested` rewrites the rules it adds to the
environment).

Our kernel already restores the rules it reconstructs (`_nested.Cov_1.rec ↦
Shadow.rec_1`) and compares them with `def_eq`. That comparison cannot see the
difference: the block-local recursor is in the temporary extension and its iota
rule computes the same value on the same major premise, so both sides are
convertible (structure eta expands the major `a` to `Cov.mk (proj a).0`). The
missing check was a **name** check on the admitted side — this fixture pins it:
the kernel now rejects the export with `imported recursor rule refers to a
block-local name` (`inductive.rs`, `assert_imported_rule_has_no_block_local_names`).

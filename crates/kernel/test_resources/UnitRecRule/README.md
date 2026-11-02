# UnitRecRule

Regression fixture from `leanprover/lean-kernel-arena`
`tests/bad/bugs/unit-rec-rule` (Apache-2.0), revision
`d7f74a9e67459e2c251d608f524dc5efc6dc223c` (2026-11-01).

`PUnit` is declared the way the prelude declares it (`inductive PUnit : Sort u`,
constructor `PUnit.unit`, recursor `PUnit.rec`), but the exported iota rule of
`PUnit.rec` for `PUnit.unit` returns the **constructor** instead of the minor
premise:

```
PUnit.rec.{u_1,u} : {motive : PUnit → Sort u_1} → (unit : motive PUnit.unit) → (t : PUnit) → motive t
rule PUnit.unit (nfields = 0):  fun {motive} (unit) => PUnit.unit   -- must be `… => unit`
```

With that rule, `badR : PUnit.rec (fun _ => Prop) EmptyProp PUnit.unit` has
type `PUnit.unit` (a term, not a type) while its value is also `PUnit.unit`,
so a type and a term are definitionally equal and `boom : EmptyProp` holds.
An export's iota rules are **not** authoritative: a checker must derive the
rule itself and reject an export whose rule differs from it
(`src/kernel/inductive.cpp:705-748 mk_rec_rules`).

Our kernel already reconstructs the rules and compares them (`inductive.rs`,
`assert_imported_expr_matches`), but the comparison was a no-op: the
unit-like shortcut in conversion returned `true` as soon as *one* side's type
was a zero-field structure, without looking at the other side. So
`def_eq(Prop, PUnit.unit)` held (`PUnit.unit`'s type, `PUnit`, is unit-like)
and the lying rule was accepted.

Lean 4 allows that shortcut only after both types are compared
(`src/kernel/type_checker.cpp:1035-1047 is_def_eq_unit_like`):

```cpp
return is_def_eq_core(t_type, infer_type(s));   // :1047
```

The kernel now does the same (in `conv_eta.rs`): a unit-like type is a
*precondition*, not a verdict. The export is rejected with
`imported recursor rule does not match the reconstructed rule` — the same
station that already rejects `bad/bugs/nat-rec-rules`.

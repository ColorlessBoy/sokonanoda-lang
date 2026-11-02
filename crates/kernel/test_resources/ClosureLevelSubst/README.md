# ClosureLevelSubst

Regression fixture from `leanprover/lean-kernel-arena`
`tests/bad/bugs/closure-level-subst` (Apache-2.0), revision
`d7f74a9e67459e2c251d608f524dc5efc6dc223c` (2026-11-01).

All declarations are `def`/`theorem` — no inductive, no axiom:

```
def A : Type := Prop → Prop → Prop
def Family.{u} : Sort (max 1 (u+1)) := A → Sort u
def good  : Family.{1} := fun _ => A      -- A → Type
def lower : Family.{0} := good            -- the only false declaration
def lowA  : Prop := lower seed            -- `lower seed : Sort 0 = Prop`
def a : lowA := fun x _ => x
def b : lowA := fun _ y => y
def D (f : lowA) : Prop := f TrueTy FalseTy
theorem boom : D b := pa                  -- pa : TrueTy, read as `D b`
```

`good` and `Family.{0}` are the **same function body** (`A → Sort u`) evaluated
in two **empty** environments that differ only in the universe substitution
(`u := 1` vs `u := 0`). Our conversion test's `Pi`/`Lam` shortcut compared the
closures by body pointer + domain pointer + `envs_ptr_equal` + closure kind, and
`envs_ptr_equal` returned `true` for *any* two `Env::Nil`s without looking at
`lsub` — even though `Env::Nil` is exactly the carrier of the universe
substitution (`eval.rs::lsub_base`, `value.rs::LevelSub { ks, vs }`). So
`Family.{1}` and `Family.{0}` were judged definitionally equal, `lower`
type-checked, `lowA` reduced to `A`, `D b` reduced to `FalseTy`, and
`boom : False` was accepted.

Lean 4 keeps universe instances inside the expression (`is_def_eq_core`,
`src/kernel/type_checker.cpp:1047`; `Level::is_equivalent`,
`src/kernel/level.h:139`), so `Sort 1 ≢ Sort 0` and the two empty environments
are not interchangeable. The kernel now requires the two `Nil` environments to
carry the **same** universe substitution (pointer equality of the interned
`LevelSub`, which is content equality inside one `TcCtx`: `intern_level_sub`
dedups `(ks, vs)` and a `LevelSub` is only those two interned slices). The
comparison lives in `conv_env_eq.rs` and only ever *allows* the shortcut — a
mismatch falls back to the authoritative value-by-value comparison, so the fix
can only add work, never reject a convertible pair.

The export is now rejected at `lower`:

```
def_eq failed: def_eq mismatch expected: Family.[0] | actual: Family.[1]
```

This test fails before the fix (the export is accepted) and passes after it.

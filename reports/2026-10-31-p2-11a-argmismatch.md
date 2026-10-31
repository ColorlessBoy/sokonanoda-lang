# P2-11a `fallback_argmismatch` root causes (read-only; base 98c1983d)

Binaries: branch `/tmp/cli-98c1983d-2fdb470c229e81db` (sha16 checked = 2fdb470c229e81db).
Probes (worktree only, removed): v3 `/tmp/cli-p211a-f05abe4da968b699` (raw printer),
v5 `/tmp/cli-p211a-ef75df1983285dd9` (+ re-eval defeq). The probe sits at
`elab_app.rs:273-276` right before `return ArgMismatch` (after the gate), gated by `P211A_PROBE=1`.
Probe line count == `fallback_argmismatch` in every file (measured: 12/16/13/1/16/26) ⇒ all are the immediate gate;
the postponed gate (:343) never fired.

## Q1 Repros (all rc 0, 0 diags, gate on, strict) — measured
Baselines: an import-only file already counts the library's own sites (`import lib.Set` = 12, `import lib.Rel` = 1).
That is why 251 files × lib sites ≈ 5.5k occurrences from ≤38 distinct sites.
| file (`/tmp/p211a-course/units/p211a/`) | argmismatch | vs baseline |
|---|---|---|
| `empty.sokonanoda` (import lib.Set) | 12 | — |
| `mem.sokonanoda` (`a ∈ univ`) | 16 | +4 (all head Set.mem, arg#1 = Set.univ) |
| `mem_ctl.sokonanoda` (`a ∈ (@Set.univ α)`) | 13 | +1 |
| `sub.sokonanoda` (`A ⊆ univ`) | 16 | +4 (2× Set.subset arg#1 = Set.univ) |
| `pow.sokonanoda` (`A ∈ 𝒫 univ`) | 26 | +14 (9× Set.powerset arg#0 = Set.univ) |
| `rel_empty.sokonanoda` (import lib.Rel) | 1 | — (= lib/Rel.sokonanoda:322 `Rel.inv_mono_iff`) |
| `iff.sokonanoda` (copy of that Iff.intro) | 2 | +1 |
| `iffn.sokonanoda` (same shape, **no `Rel.inv`/no def**) | 2 | +1 |
Controls that add 0: `iffd`, `myid` (delta of `Rel.inv` under λ binders, flat and nested), so plain delta is fine.
`dlam*`/`m1` go through `id`, which is universe-polymorphic and never reaches the kernel path, so they don't count as evidence.

## Q2 Root causes
**A. Set.mem / Set.subset / Set.powerset (measured).** The arg is a **bare constant**.
- Probe (f05abe4d): `head=Set.mem arg#1=Set.univ DOM=(Set #1) INF=Π{Sort}, (Set #0)`. The same holds for subset and powerset.
- Chain: `kernel_app.rs:406` elaborates each explicit arg with expected `None`. That reaches `elab.rs:5612` `try_bare_implicit_constant`, which by design does nothing without an expected type ("没有期望类型就不动", comment at elab.rs:5604-5606). So the kernel receives `Set.univ : {α : Type} → Set α`.
- `elab_app.rs:272` `infer_value(arg)` gives the Pi. A Pi vs `Set α` is never defeq, so the gate at :273 fails. `app_stuck_on_meta(dom)` is false (dom is `Set α`, not a meta), so it returns `ArgMismatch` at :276.
- In 5/9 powerset readings the domain is `Set (?1073741824 #1 #0)`: the outer α is a session meta. This matters for Q4.
- The +1 in `mem_ctl` is a Set.mem/univ reading from the theorem's own elaboration path; the reading is the same as in `mem` (not split further).

**B. Iff.intro (measured mechanism, inferred locus).** The census hypothesis "def needs delta" is **refuted**: `iffn` has no def and still fails.
- Probe: `DOM` and `INF` print **textually identical** after `quote + instantiate_mvars`.
- v5 at the same gate: `rawraw_retry=false`, but `requoted_both=true rawdom_vs_reat=true redom_vs_rawat=true`. So comparing the **raw `Value`s** fails, while re-evaluating *either* side from its quoted, meta-instantiated form succeeds.
- Inferred cause: `dom` was built by `apply_closure(..., mv)` with session metas (`?a ?b` of Iff.intro) that `app_propagate` (:245) assigned **afterwards**. The stale `Value::Meta` sits inside Pi closures/spines. `def_eq_at` follows assignments only at the head being forced, not inside captured closure bodies compared under binders.
- Locus: `elab_app.rs:273` (`is_def_eq_at(d, dom, at)`) with dom from :235/:282. It only shows up for λ args whose type is a nested Pi over the propagated metas.

## Q3 Lean 4 (v4.35.0)
- A: Lean never gives a bare `univ` the type `{α} → Set α` in arg position. `elabAppArgs` runs even for a zero-arg ident and instantiates leading implicits (`addImplicitArg`, `Elab/App.lean:745`; the `processExplicitArg` / implicit branches at :766-857). Then `elabAndAddNewArg` (:433-443) elaborates each arg **with the binder type as expected type** and `ensureArgType`s it.
- B: Lean's `isDefEq` instantiates assigned mvars as it goes (`Meta/ExprDefEq.lean:1402` `instantiateMVars` on app heads; `:146/:268` `isAssigned` checks; `whnfCore` `Meta/WHNF.lean:621` follows assigned mvars at every head). An `Expr` has no stale closure, so this failure mode can't happen there.

## Q4 Proposed fixes (design only)
**Fix A, front-side (preferred, ~6 lines)** in `kernel_app.rs::run_once` first pass (:398-417).
- When an arg is a bare `Expr::Ident`/`Notation` (e.g. `∅`) resolving to a decl with `implicit_prefix() > 0`, push `None` (postpone). Then `postpone::elab_ready` elaborates it against the kernel domain, i.e. `elabAndAddNewArg`. `try_bare_implicit_constant` then sees an expected type and fills α.
- Alternative kernel-side (~25 lines): at :272, if `infer(arg)` has leading implicit Pi binders and `dom` is not a Pi, apply fresh session metas, matching Lean `addImplicitArg`. That's riskier because it changes term construction inside the kernel session.
- Risk: low. Only an extra round, and loose is untouched. Expected type flows only on the strict+gate path, and `app_recheck` still guards the output.
- Interaction: when the domain is `Set ?α`, which happens when the head's own α is still a session meta (the 5 powerset readings), the hole comes back `blocked`. It then needs **P2-10** (blocked domains with metas passed as hints) to resolve rather than ending in `fallback_unsolved`. **P2-9** (outer metas in `expected`) is what lets the inner `univ` session accept `Set ?outer`. Land A after P2-9/P2-10, or accept some argmismatch → unsolved migration in between.

**Fix B, kernel-side (~4 lines)** in `elab_session` before the gate at :273: zonk `dom` (`eval(d, env, instantiate_mvars(quote(d, dom)))`) when any session meta has been assigned since `dom` was built. The v5 reading `redom_vs_rawat=true` shows zonking dom alone suffices.
- The cleaner long-term fix is to make `def_eq_at` force assigned metas inside closures. It touches the hot defeq path, so it needs the kernel triple (repro + reverse check + per-file byte-identical).
- Risk: Fix B accepts only pairs that are defeq after instantiation, so judgement stays the same. It costs one quote/eval per explicit arg, so gate it on `metas assigned > 0`.
- No interaction with P2-9/10 beyond the fact that those add more assigned/outer metas, making the stale-value case *more* frequent. Land B **before** P2-9.

## Q5 Expected effect (inferred from samples, not re-measured over 251 files)
- Fix A removes the Set.mem 5132 / Set.subset 68 / Set.powerset 38 sites whose arg is a bare implicit constant (`univ`, probably `∅`). The powerset/nested cases whose α is a session meta become `blocked`, and clear only with P2-10.
- `Set.mem_powerset_iff` (4) and `Set.ext` (1) were not sampled; they are probably the same bare-const shape. Re-run the census probe to confirm.
- Fix B removes the Iff.intro 330 sites (λ args under propagated metas; the lib/Rel:322 / lib/Order:127 family). It may also remove any Set.* site of the same stale-meta form.
- Remaining after A+B: anything not matching either probe signature; re-census with probe `f05abe4d`-style output to list them.

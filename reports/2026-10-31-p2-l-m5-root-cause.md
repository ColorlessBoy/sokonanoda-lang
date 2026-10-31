# P2-L: loose-path root cause for m5 (`{∅} = @Set.univ (Set Nat)`)

Binaries: base `cli-p2aa-441e185c8995ce30` (HEAD 140d1ca0). Probes built in a private detached worktree
(removed): `cli-p2l-bcf0b7463aab5bdd` (probe at k==0 tail, never hit for Eq), `cli-p2l-a43dcff31e0f06c1`
(probe in the implicit branch, after `notation_level_texts`), `cli-p2l-f1c584547c50711c` (= a43d + candidate fix).
Repo checkout untouched (git status clean).

## Q1 Where the decision is made (measured)
- `=`/`≠` lower in `elab_notation` (crates/front/src/compile/elab.rs:1575). `Eq`/`Ne` have `implicit_prefix > 0`,
  so they take the implicit branch at elab.rs:1641-1662: it computes `level_texts` first (elab.rs:1647,
  `notation_levels.rs:13`), then `elab_notation_implicit` (elab.rs:1769) builds `Eq.{<level>} lhs rhs` as a source
  `UniverseApp` and hands it to `elab_expr`. The type argument `α` itself is solved later by `try_implicit_application`;
  the bug is in the universe `u` of `Eq.{u}`, which is fixed before either operand is elaborated.
- Deciding step: `universe_level_text_of_operands` (elab.rs:2258). It loops over operands, but the first statement is
  `let ty_text = infer_type_text(...)?;` (elab.rs:2267-2274). The `?` returns `None` from the whole function as soon
  as one operand's type can't be inferred standalone, so later operands are never asked. `notation_level_texts`
  then keeps its default `"0"` (notation_levels.rs:25), and the doc comment says "逐个操作数试", which the code doesn't do.

## Q2 What m5 picks (measured, probe a43dcff31e0f06c1, after the decision)
| file | operands | operand types (infer_type_text) | level_texts |
|---|---|---|---|
| m5 | `{∅}`, `@Set.univ (Set Nat)` | None, `Set Nat -> Prop` | `["0"]` |
| l_h (swapped) | `@Set.univ (Set Nat)`, `{∅}` | `Set Nat -> Prop`, None | `["1"]` |
- `{∅}` alone has no expected type, so `Set.singleton ?α ∅` can't be inferred; that `None` hits the `?` → `Eq.{0}`.
  With `α := Set (Set Nat) : Type` the kernel sees `Eq.{0}` wants `α : Sort 0`: `expected Sort(0) | actual Sort(1)`.
- Swapped: operand 0 infers first try → `u = 1`; the `None` for `{∅}` is never reached. Causality (measured): with
  only `?` → `continue` changed (f1c58454), m5 gets `["1"]` and is accepted.

## Q3 Minimal family (fixtures `/tmp/p2c-course/units/p2c/l_{a..k}.sokonanoda`; loose; base 441e185c / fix f1c58454)
| shape | base | fix | first diag (base) |
|---|---|---|---|
| a `{∅} = @Set.univ (Set Nat)` (= m5) | reject | accept | Sort(0)/Sort(1) |
| b `(B : Set (Set Nat)) {∅} = B` | reject | accept | same |
| c `∅ = @Set.univ Nat` | reject | accept | same |
| k `(B : Set Nat) ∅ = B` | reject | accept | same |
| e `{∅} ≠ @Set.univ (Set Nat)` | reject | accept | same |
| d `(a : Set Nat) {a} = B` | accept | accept | - (`{a}` infers alone) |
| j `(n : Nat) {n} = B` | accept | accept | - |
| f `{∅} ∈ @Set.univ (Set (Set Nat))` | accept | accept | - (`Set.mem` has no universe param) |
| g `{∅} ⊆ @Set.univ (Set Nat)` | accept | accept | - (same) |
| h/i RHS-only `X = {∅}` | accept | accept | - |
Family = an `Eq`/`Ne` (universe-polymorphic notation target) whose first operand is `∅` or a set literal whose element
type needs the expected type. `∈`/`⊆` aren't affected because their targets have no universe params.

## Q4 Lean 4 (source `~/Documents/lean/lean4` @ eb6cafb, read only, Lean not run; behavior inferred)
- `elabBinRelCore` (Elab/Extra.lean:497) builds trees for both sides first (`toTree` :183, with
  `synthesizeSyntheticMVars (postpone := .yes)`), then `analyze tree none` (:534).
- `analyze.go` (:256-314): for every leaf, `inferType`; `isUnknown type` → set `hasUnknown` and continue (:275).
  An unknown operand never aborts the scan; the known side supplies `maxType`.
- Then either `applyCoe tree maxType` (:551) or the default `ensureHasType lhsType rhs` + `elabAppArgs` (:543). `Eq`'s
  universe is a level mvar solved by unification, so it's never committed from the first operand.
- Minimal alignment for loose: match `isUnknown → continue`, i.e. skip operands whose type can't be inferred
  standalone instead of aborting. The full alignment (level mvar + unify) is the strict/kernel path (P2-11c).

## Q5 Proposed fix (design; measured in a throwaway worktree only)
- `crates/front/src/compile/elab.rs` `universe_level_text_of_operands` (:2267-2274): change
  `let ty_text = infer_type_text(..)?;` to `let Some(ty_text) = infer_type_text(..) else { continue };`.
  About 3 lines. Lean4: Extra.lean:275 `isUnknown → hasUnknown` (skip, don't abort).
- Other callers (elab.rs:7859 `&[first]`, :7875 `&refs` of solved implicits): single-element calls are unchanged by
  construction. The `refs` call can now try later implicits after an unknown first one, so it only adds solutions.
- Judgement-change scope (measured): all 357 tracked `.sokonanoda`, loose, cold caches, run from the repo checkout,
  base 441e185c vs fix f1c58454: per-file stdout sha (progress lines stripped) and diag count are identical, 357/357
  (`diff` empty). The only empty-output hash is `lib/Logic`, which is expected. No tracked file changes verdict, so there
  are no accept→reject flips (and no reject→accept either, since no tracked file has this shape in a failing position).
- Heuristic (grep, not type-aware): 65 lines in 22 tracked files have `{…}`/`∅` directly before `=`/`≠`. All already
  pass on base, because their literals have typed elements (like `{a}`, `{n}`). The "LHS fixed only by RHS" count in
  tracked files is effectively 0, consistent with the identical corpus.
- Not measured: the strict-dialect corpus with this fix (the function is shared by both dialects), the LSP path,
  and fmt/clippy/tests.
- Lists: `/tmp/p2l-b.s` (base) and `/tmp/p2l-f.s` (fix).

## Q6 Strict, gate on (`SOKO_STRICT_KERNEL_APP=1`, `scripts/strict/fallback-one.sh`)
- Base 441e185c: rc=1, same Sort(0)/Sort(1) diag. KERNEL_APP tried=2177 solved=2169 fallback_unsolved=8 (all lib
  noise), and no `universe-polymorphic head` count (measured).
- Inferred from code: the implicit branch builds the head as `Expr::UniverseApp{Eq,["0"]}`, and
  `kernel_app::attempt` (kernel_app.rs:~247) only takes an `Expr::Ident` head → `Err(None)` (not tried, not counted).
  So m5 never reaches the kernel path or the `universes.is_empty()` gate (kernel_app.rs:268). The level was already
  wrong before strict sees it.
- Fix f1c58454 under strict + gate: rc=0, 0 diags, same counters (measured). This shared front fix also fixes m5 in
  strict; P2-11c is still needed for its own counters, not for m5.

Fixtures: `/tmp/p2c-course/units/p2c/m5.sokonanoda`, `l_a`..`l_k.sokonanoda`. Probe/fix binaries `/tmp/cli-p2l-*`.

## [0.53.0] - 2026-09-16

### Changed

- **Course restructured to 8 units (P2 of the syllabus redesign)** —
  - `by` tactics moved to **unit 4** (right after functions/arrows) so the tactic
    toolbox and the Infoview goal state arrive early instead of at the end;
  - the over-loaded induction unit is **split into Ⅰ/Ⅱ**: Ⅰ = explicit
    `inductive`/`rec`/`iota`, recursion via hand-written `Nat.rec`, `match` on a
    non-recursive inductive, recursive `match` with the auto-inserted IH;
    Ⅱ = parameterized `Option A`, dependent `match` (= induction), nested /
    literal / wildcard / guard patterns, indexed `Vec A n`.
  - Totals: **units 8, checked 58, open 45, failed 0** (`checked` +1 because each
    half redeclares its own `inductive Nat`).
- Docs, skills and the extension README were synced; hand-written unit counts were
  removed from the root README and the website prose (the number is generated from
  `course/course.json`).

## [0.52.0] - 2026-09-16

### Changed

- **Course content revision (P1 of the syllabus redesign)** — see
  `docs/design/course-syllabus.md`:
  - U4 (universes) gained two "read `#check` output / judge the type" exercises and a
    predict-then-prove `#reduce` exercise (it was previously 0 checked / 0 reduced);
  - U6 lost `by_ex5`, an exact duplicate of `by_ex1`;
  - U3's `three_args` / `body_uses_let` are no longer ambiguous (the expected return is
    pinned);
  - hint ladders no longer contain complete answers — the "key piece" line states the
    trigger condition and which lemma/constructor to use;
  - a stale in-file claim in U5 ("`match` only handles non-recursive inductives") is gone.
- **Course test hardening**: new `solution_covers_every_canvas_exercise` and
  `en_solutions_match_chinese_event_counts` (canvases *and* solutions are now compared,
  including `expr.typed`, which caught a real EN-solution drift in U4).

## [0.51.0] - 2026-09-16

### Added

- **Newline-separated tactics** — inside `by` blocks you may now separate tactics
  with a newline instead of `;` (Lean style), and mix both. A tactic's expression
  ends when the next line starts with a tactic keyword, so `exact f` followed by
  `apply g` on the next line stays two tactics. No indentation sensitivity.

### Fixed

- **`sokonanoda gate` refuses to run with a stale binary** — the playground anchor
  compiles with the binary's *embedded* compiler, so a stale download cache (e.g.
  v0.27.0 while the repo is newer) would silently gate with old logic and report a
  bogus source error. `gate` now compares its own version against the repo's
  `Cargo.toml` and exits 3 with an actionable message on mismatch.

## [0.50.0] - 2026-09-15

### Changed

- **The Infoview has its own fixed colour palette** (per dark/light/high-contrast),
  tuned to look like VS Code's default Dark+/Light+ token colours. Every
  `SemanticKind` is now guaranteed to be coloured — `Prop`/`Type`/`Sort` no longer
  degrade to the plain foreground because a theme lacked a `symbolIcon` variable.
  Rationale: VS Code exposes no stable API for editor token colours, and scraping
  theme JSON was judged too costly; see `docs/design/highlighting.md` §3b.

## [0.49.0] - 2026-09-15

### Added

- **`sokonanoda build`** — compile files (or a directory) and persist the kernel
  artifacts in a shared, content-addressed cache, so later opens/builds skip
  recompiling. `--clean` clears it, `--json` reports hit/compiled/failed. The
  course view and `sokonanoda --json` now reuse the same cache. Design:
  `docs/design/compile-cache.md`.

### Fixed

- **Infoview panel is now reliably visible**: the view no longer carries a
  `when` clause (VS Code hides an *empty* extension container, which is why it
  "couldn't be popped open"), the extension is activated `onView`, and the host
  registers the webview provider **before** the (possibly slow) server
  resolution, which is what made the panel appear only after toggling the side
  bar. `sokonanoda: 打开目标面板` also opens the auxiliary bar.
- **The panel never shows a silent blank**: it renders a skeleton on load and a
  live status line (`编译中…` / `已就绪 · N 个声明` / `等待 .sokonanoda 文件`).
- **Declaration rows** no longer pretend to jump (it did not work reliably);
  each now shows the declaration's line number (`L12`) in small text next to the
  name, plus its type.
- **One highlighting source**: hover goal text is now the projection of the same
  semantic runs as the Infoview, and `SemanticKind` maps to TextMate scopes, CSS
  classes and LSP token types from a single table guarded by exhaustive tests.
  See `docs/design/highlighting.md` (markdown can only be TextMate-coloured, so
  hover and Infoview colours are close but not identical by design).

## [0.48.0] - 2026-09-15

### Added

- **Persistent compile cache** — the language server now caches each file's
  kernel report keyed by (compiler version, prelude mode, source text), so
  reopening an unchanged `.sokonanoda` file skips recompiling; opening files
  stays fast as a workspace grows. The kernel remains the only judge.
  Disable with `SOKONANODA_NO_CACHE=1`. Design:
  `docs/design/compile-cache.md`.

### Fixed

- **Infoview declaration types wrap** instead of being truncated, the goal line
  starts with `⊢`, and clicking a declaration now actually jumps the editor to
  it (the document is resolved from the click, not from `activeTextEditor`).

## [0.47.0] - 2026-09-15

### Added

- **Indexed inductive declarations** — types may now carry indices alongside
  parameters, e.g. `inductive Vec (A : Type) : Nat -> Type` with
  `ctor vnil : Vec A zero` / `ctor vcons (a : A) (n : Nat) (v : Vec A n) : Vec A (succ n)`.
  The recursor is derived (motive abstracts the indices), and `match` works for
  results that do not depend on the index (e.g. computing the length). Design:
  `docs/design/indexed-inductives.md`.

## [0.46.0] - 2026-09-15

### Added

- **`match` as a tactic** — inside a `by` block you can now write
  `match c with | red => green | green => red` (arm bodies are terms, exactly like
  the value-position `match`), judged against the current goal. `by exact match …`
  works too. The judge now keeps the real source prefix, so a `match`'s universe
  query succeeds inside judgements.

## [0.45.0] - 2026-09-15

### Added

- **Binder type inference at application sites** — an unannotated `fun x => …`
  used as a function (`(fun x => x) 1`, `(fun x y => x) 1 2`) now infers its
  binder types from the argument types (kernel-checked), so fewer annotations
  are needed. Positions with neither an expected type nor arguments still
  require the annotation.

## [0.44.0] - 2026-09-15

### Added

- **Infoview declaration types + click to jump** — the declaration list now
  shows each declaration's type as a small, dim, syntax-coloured line under its
  name (coloured from the same `front::semantic` runs as the goal state, never
  re-tokenized), keeping the one-declaration-per-line layout. Clicking a
  declaration now also moves the editor to it.

## [0.43.0] - 2026-09-15

### Changed

- **One highlighting path everywhere** — every place the editor shows
  `.sokonanoda` text now uses the same `sokonanoda` markdown code fence, so it
  is coloured by the single TextMate grammar: expression/signature hovers (were
  plain `text`), declaration hovers, the tactic and half-expression hover
  headers, completion documentation, and the exercise-tree tooltips. Blocks
  that a client cannot markdown-render (diagnostics, inlay hints, tree
  descriptions, code-action titles) stay plain text, as documented in
  `docs/design/goal-rendering.md` §7.

## [0.42.0] - 2026-09-15

### Added

- **Richer `match` patterns** — value-position matches now support wildcards
  (`_`), nested constructor patterns (`| some (succ k) =>`), natural-number
  literals (`| 0 =>`, `1`, …; desugared to `succ^k zero`), and `Bool` guards
  (`| succ k if p =>`). Arms are ordered and the first match wins, so the same
  constructor may appear in several arms. Unknown bare names bind a variable
  (Lean semantics). Design: `docs/design/match-patterns.md`.

## [0.41.0] - 2026-09-15

### Added

- **Prelude `Bool`** — `Bool`, `Bool.true`, `Bool.false` and the derived
  `Bool.rec` are now installed as a trusted, non-recursive inductive (the same
  way as `Nat`), so `match b with | Bool.true => … | Bool.false => …` checks and
  reduces through the real kernel. A file that declares its own `inductive Bool`
  keeps it (the prelude steps aside).

## [0.40.0] - 2026-09-15

### Added

- **Infoview in the right side bar** — the goal panel now lives in its own
  `sokonanoda` container on the **secondary (right) side bar** instead of
  sharing the Explorer, so it can sit next to your proof. Requires VS Code
  **1.106+** (extension-contributed secondary-side-bar containers), and
  `sokonanoda: 打开目标面板 (Infoview)` reveals it.
- **Unified goal highlighting** — goal and hypothesis text in the Infoview is
  now coloured from the **same single source** as the editor's semantic tokens
  (`front::semantic`): `soko/stateAt` ships `goal_runs` / `ty_runs` (ordered
  `{text, kind}` fragments) and the webview renders them with theme-aware
  colours. No more re-tokenizing, no more drift between hover and the panel.
  Design: `docs/design/goal-rendering.md`.

### Changed

- The TextMate grammar now mirrors `front::semantic` exactly (hover code fences
  highlight `let`, `match`, `with`, `by`, `intro`/`exact`/`apply`/`assumption`/
  `rfl`, `#check`/`#reduce`/`#print`, `forall`/`∀`); the hard-coded `Nat`
  keyword is gone, and a contract test keeps the two lists equal.

### Fixed

- Opening the Infoview no longer shows the confusing
  「目标面板 (Infoview) 暂时不可用…」 warning: the view renders on demand and
  degrades silently to the Explorer tree's 「当前光标处」 group if a webview
  cannot be shown.
- Marketplace description trimmed under 300 characters (it was truncated
  mid-word) with a regression guard.

## [0.39.1] - 2026-09-14

### Fixed

- **`judge_infer` type round-trip robustness** — `render_expr` now parenthesises
  a `forall`/arrow/lambda when it sits in the **domain** of an arrow, so the
  kernel-type → text → AST round trip used by `judge_infer` (and thus the
  dependent-`match` motive level query, suggestions and hover) no longer
  corrupts a telescope containing a function-typed binder. Previously a
  dependent `match` whose result type referenced such a binder failed with
  `elab-match-no-expected-type`.

## [0.39.0] - 2026-09-14

### Added

- **`match` dependent motive** — when the result type mentions the (local
  variable) scrutinee, the motive becomes `fun t => R[x := t]`, each arm's
  expected type is the instantiated `R[x := <constructor term>]`, and a
  recursive arm's induction hypothesis has the dependent type `R[x := <field>]`.
  This makes the natural induction principle expressible:
  `theorem nat_induction (P : Nat -> Prop) (hz : P zero) (hs : …) (n : Nat) : P n := match n with | zero => hz | succ k => hs k ih`.
  Non-dependent matches are unchanged. Design:
  `docs/design/match-dependent-motive.md`.

## [0.38.0] - 2026-09-14

### Added

- **Parameterized inductive declarations (non-indexed)** — declarations may
  now take parameters, e.g.
  `inductive Option (A : Type) : Type` / `ctor none : Option A` /
  `ctor some (a : A) : Option A` / `end`, with the derived recursor. `match`
  works on them (`match o with | none => … | some a => …`); the type arguments
  come from the scrutinee's written source type, and the field type is
  instantiated (`some a` gives `a : A`). Indexed inductives,
  universe-polymorphic parameters, nested/mutual blocks and nested/guard
  patterns remain out of scope. Design:
  `docs/design/parameterized-inductives.md`.

## [0.37.0] - 2026-09-14

### Added

- **`sokonanoda watch` client commands** — the watch process now reads JSON
  Lines commands on stdin: `{"type":"ping","id":N}` replies with a `pong`,
  and `subscribe`/`unsubscribe` filter which files emit events in
  `--workspace` mode. Malformed input yields an `error` event and the stream
  keeps running; stdin EOF does not stop monitoring. Design:
  `docs/design/compiler-service-events.md`.

## [0.36.0] - 2026-09-14

### Added

- **`match` on the prelude `Nat`** — the built-in `Nat` is now a real trusted
  inductive (constructors `Nat.zero`/`Nat.succ` plus a derived `Nat.rec`), so
  `match` works on it directly:
  `def pred (n : Nat) : Nat := match n with | Nat.zero => Nat.zero | Nat.succ k => k`
  (arms use the dotted constructor names). Recursive branches get the induction
  hypothesis `ih`, exactly like source recursive inductives. Note: `#reduce`
  through `Nat.rec` may print an unary chain (e.g. `Nat.succ (Nat.succ 1)`) that
  is definitionally equal to the numeral. Design: `docs/design/match.md`.

## [0.35.1] - 2026-09-14

### Infrastructure

- **Release integrity** — every Release now ships a `SHA256SUMS` manifest
  covering the 8 LSP tarballs, 8 CLI tarballs and 9 VSIXes, plus SLSA
  build-provenance attestations (`actions/attest-build-provenance`) for all of
  them. Verify with `sha256sum -c SHA256SUMS` and
  `gh attestation verify <file> -R ColorlessBoy/sokonanoda-lang`. See
  `docs/RELEASE.md` §6. Asset count is now 26.

## [0.35.0] - 2026-09-14

### Added

- **`match` on recursive inductives (induction hypotheses)** — for a source
  recursive `inductive`, each recursive constructor field now gets an
  auto-inserted induction hypothesis named `ih` (`ih2`, …) typed as the match
  result; the branch body can reference it, so recursive functions/proofs are
  written through the recursor without self-reference
  (`def add (a b : Nat) : Nat := match a with | zero => b | succ m => succ ih`).
  Still out of scope: dependent motives, parameterized/indexed inductives, the
  prelude `Nat`/`Eq`, `match`-as-tactic and nested/guard/literal patterns.
  Design: `docs/design/match.md`.

## [0.34.0] - 2026-09-14

### Added

- **Unannotated `let`** — `let x := v; body` now infers the binder type from
  the value via the kernel (`judge_infer`, reusing its bounded cache); the type
  annotation is optional. When the value alone cannot determine the type (e.g.
  a `sorry` value), it reports the teaching error `elab-let-type-query-failed`
  asking for an explicit annotation.

## [0.33.1] - 2026-09-14

### Changed

- **Tactic hover presentation** — the tactic hover now names the tactic and its
  position, and renders each goal state in a `sokonanoda` code fence, so the
  hypotheses/goal are monospaced, aligned and syntax-highlighted (the extension
  ships the grammar). Multi-goal states show a `目标 i/n` header per goal. The
  half-expression goal hover uses the same fence.

## [0.33.0] - 2026-09-14

### Added

- **`match` (v1)** — pattern matching on **source-declared, non-recursive**
  `inductive` types: `match e with | Ctor x … => body | …`. Each constructor
  is covered once (any order; reordered to declaration order), the result type
  is the expected type at the match position, and lowering goes to
  `<Ind>.rec.{level} (fun _ => R) minors … e` (the universe level is derived
  from the expected type). Kernel-frozen and kernel-judged. Recursive
  inductives, dependent motives, parameterized inductives and the prelude
  `Nat`/`Eq` are explicit errors in v1. Course coverage added to the induction
  unit. Design: `docs/design/match.md`.

## [0.32.1] - 2026-09-14

### Changed

- **Incremental edits stop re-checking unchanged suffixes (early-cutoff)** —
  the session now compares each command's elaborated environment contribution
  (structural signature: type **and** body, so delta unfolding stays correct)
  and reuses the remaining snapshots once the signature matches. In the common
  case a one-line edit drops the kernel re-checks to 1 instead of the whole
  suffix. Conservative and sound: multi-edits, changed bodies and kernel
  rejections fall back to the previous suffix re-check. Design:
  `docs/design/early-cutoff.md`.
- Documented an opt-in external performance/soundness baseline against the
  Lean Kernel Arena (`scripts/perf-arena.sh`, `docs/PERF.md`); not part of CI.

## [0.32.0] - 2026-09-14

### Added

- **Kernel-driven expected types for refine sub-holes (spine meta, 方案 A)** —
  the goal view / inlay now fill sub-hole expected types that the syntax walk
  left empty, by probing the kernel at request time (`judge_infer` + its
  bounded cache; never on the keystroke path). Covered: preceding-hole
  penetration (`f sorry sorry`, the second using the first's expected type),
  one-level nested holes (`f (g sorry)`), and syntactic def-eq domains.
  Deeper nesting and def-wrapped *result* types still fall back to the old
  behaviour. Design: `docs/design/spine-meta-a.md`.

## [0.31.0] - 2026-09-14

### Added

- **`sokonanoda: doctor`** — a read-only self-check that reports which server
  is in use and its `source`, the running vs extension version, a stale
  extension host, ignored overrides, the download cache and old-version
  buildup. It runs automatically after activation and `restart server`, and
  offers a "Run doctor" action on problems.

### Changed

- **Bundled server is now forced by default** — `sokonanoda.serverPath` /
  `SOKONANODA_LSP_BIN` and workspace `target/{debug,release}` builds are
  ignored unless the new `sokonanoda.serverOverride` setting (default
  `false`) is enabled. A stale local build can no longer silently override
  the server that ships inside the extension. Design:
  `docs/design/extension-server-policy.md`.

## [0.30.0] - 2026-09-14

### Added

- **Infoview panel (webview)** — a Lean-Infoview-style goal panel in the
  side bar (`sokonanoda.infoview`, command `sokonanoda.openInfoview`) rendering
  every goal with its hypotheses, the `by` progress and the running server
  version. The extension host remains the only LSP client (it posts
  `soko/stateAt` / `soko/goals` / `soko/version` snapshots); cursor moves post
  only the light `state` message (no `soko/goals`, no rebuild), and the panel
  falls back silently to the existing tree group when a webview is
  unavailable. Design: `docs/design/webview-infoview.md`.

## [0.29.0] - 2026-09-14

### Added

- **Compiler service event stream** — `sokonanoda watch` now opens with a
  `service.hello` handshake (`{protocol, engine, pid}`) and its canonical
  change event is `file.didChange` (`file.changed` stays as a one-minor
  deprecation alias). New flags: `--doc <file>` (single document) and
  `--workspace <root>` (every `*.sokonanoda` under the root, one session per
  file, per-file versions, events carry `file`). Design:
  `docs/design/compiler-service-events.md`.

## [0.28.0] - 2026-09-14

### Added

- **Local bindings: `let x : T := v; body`** — the teaching language now
  supports `let` at any term position (also inside `#check`, `#reduce`, `fun`
  bodies and parentheses). It elaborates to the kernel's own `Let` (zeta), so
  the value is still judged by the complete kernel; the type annotation is
  required for now. Includes course coverage in the functions/arrows unit and
  goal-view support for `sorry` in the value or body. Design:
  `docs/design/elaborator-let-match.md` (`match` is a later milestone).

## [0.27.1] - 2026-09-14

### Fixed

- **`sokonanoda: restart server` no longer silently keeps a stale server** —
  it now re-resolves through the same path as activation, including the
  version-pinned download fallback, and aborts with a message instead of
  restarting the old binary when no usable server is found (a stale cache
  marker is rejected, not reused). It also detects a newer installed extension
  and tells you to reload the window, since extension-code upgrades cannot be
  picked up by a server restart.

## [0.27.0] - 2026-09-14

### Added

- **Multi-goal display** — after a tactic that opens several sub-goals (for
  example `apply And.intro`), the cursor goal view and the exercise panel now
  list **every** remaining goal, current first, each with its own hypotheses,
  instead of showing only the top goal. The server records the full open-goal
  list per tactic step and exposes it as `goals[]` in `soko/stateAt` and
  `soko/goals`; the single `goal`/`binders` fields remain for older clients.
- **Tactic goal-state hover** — hovering a `by` tactic (anywhere in its source
  span) shows the goal state entering that tactic — every remaining goal with
  its hypotheses, Infoview-style. It reuses the per-tactic snapshot
  (`by_steps`), so no re-check happens on hover.
- **Tactic keywords are highlighted** — `by`, `exact`, `assumption` and `rfl`
  are now classified as keywords (previously they colored as plain
  identifiers).

### Removed

- **Value-position keywords** — `funintro` (and the earlier `funapply`) are
  gone. The value position now accepts only a plain expression or a
  `by <tactic>; …` block; `funintro` is no longer a keyword, so it parses as an
  ordinary identifier. The completion/hover expand button, the
  `sokonanoda.expandIntro` command, and the `elab-intro-not-a-function` error
  code were removed with it. The `by` tactic `intro` is unchanged.

## [0.26.0] - 2026-09-14

### Added

- **Course unit 7 — quantifiers (`forall` & `exists`)** — a new bilingual
  canvas with seven kernel-graded exercises: universal introduction
  (`fun`) and elimination (application), existential witnesses and
  `Exists.elim`, the two distribution laws with `And`, `∀ → ∃` on a
  non-empty domain, and existential monotonicity. The unit carries its own
  logic skeleton and `Exists` axioms, plus an English mirror, agent answer
  keys, golden event counts, and a `#reduce` self-test. The learner canvas
  `playground.sokonanoda` gained the same lesson.

## [0.25.0] - 2026-09-14

### Fixed

- **`sorry` hover shows the precise expected type** (user report on
  playground exercise 5) — for `(And.right a (Not a) x) sorry` the hover
  now says the hole expects `a` (computed by unfolding `Not a` via the
  `Not` definition to `a -> False` and taking the arrow domain), instead
  of the whole declared type. The goal walk now handles **over-applied
  spines**: arguments beyond a function's declared telescope are matched
  against its result type, unfolding simple `def` bodies one step at a
  time. The remaining goal (`False`) and the local hypotheses (`a`,
  `x : And a (Not a)`) are shown alongside.

## [0.24.0] - 2026-09-13

### Development / Infrastructure

- **Performance tests are now routine** (user requirement: performance is
  the project's lifeline) — 6 threshold-sentinel tests run on every push:
  compiler scaling (400 blocks ≤ 12× the time of 50; O(n²) trips it),
  incremental editing (<50ms per keystroke, only the edited block is
  kernel-checked), and LSP interaction latency (didChange <50ms;
  completion/hover/goal view <10ms). Every push also uploads a
  `perf-report` artifact (version + commit) so regressions can be traced
  to the change that caused them. No user-facing behavior change;
  the version bump exists so each round of work has its own perf report.

## [0.23.0] - 2026-09-13

### Added
- **Half-expression goal state** — hovering a partial application the kernel
  rejected (e.g. `And.intro b a` against `And b a`) now lists the inferred
  remaining goals (`|- b`, `|- a`) instead of only the error. Computed on
  hover only, with a bounded fingerprint cache.

### Performance
- **Judge results are cached** (fingerprint-keyed, capped at 128) — the
  by-block tactics `apply`/`exact` infer types through a full document-prefix
  recompile on every keystroke; unrelated edits now hit the cache. This was
  the same O(n²) pattern that got value-position `funapply` removed.

## [0.22.0] - 2026-09-13

### Removed
- **Value-position `funapply`** — its lowering asked the kernel to infer the
  applied term's type, which re-compiled the entire document prefix on every
  keystroke (O(n²)); the interaction was unusably laggy (user report). The
  by-block tactic `apply` is unchanged. Course unit 6's funapply aside and
  exercises were removed with it (goldens reverted to (13, 6, 0) / 33-27).

### Changed
- **`funintro` skeleton lands with `sorry` selected** — accepting the
  completion inserts the skeleton as a snippet whose trailing `sorry` is
  preselected, so the next input overwrites it directly.

## [0.21.0] - 2026-09-13

### Added
- **`funintro` (funapply X) composition** — the keywords are now first-class
  expressions: `funintro (funapply And.intro)` peels every remaining binder and
  lowers `funapply And.intro` against the final goal
  (`And.intro b a sorry sorry`). Pure front-end; the kernel never sees a
  keyword.
- **Completion while typing** — the 「替换源代码」 item now appears from the
  first keystroke of the keyword (previously the popup was empty until the
  whole expression compiled). While the keyword is incomplete the item
  completes the word; once the argument compiles it upgrades to the full
  skeleton replacement.

### Changed
- **Breaking (teaching surface): value-position keywords renamed** —
  `intro` → `funintro`, `apply` → `funapply`, to remove the ambiguity with the
  tactic versions inside `by` blocks (which are unchanged). Course unit 6 and
  the playground were updated in the same release. Error codes keep their
  historical names (`elab-intro-not-a-function`, `elab-apply-*`); the
  human-readable hints use the new names.

## [0.20.0] - 2026-09-13

### Added
- **`intro` / `apply` now work inside a `fun` body** — the natural place a
  learner reaches after introducing some binders by hand:
  `theorem t : Q -> P := fun (x : Q) => apply proofP` (implicit replacement)
  and `theorem and_swap : … := fun (a : Prop) => fun (b : Prop) => fun (x :
  And a b) => intro` (introduces the rest). Previously the keywords were only
  recognised at the very start of the value, and inside a `fun` body `apply`
  was an ordinary identifier (`unknown identifier`).
- **`by` also works at the tail of a `fun` body** —
  `theorem t : Q -> P := fun (x : Q) => by exact proofP` enters tactic mode
  with the lambda binders as the initial context; `by_steps`, goal view and
  kernel judging behave exactly like a value-position `by`.

### Fixed
- Command-palette entries no longer print `sokonanoda: sokonanoda: …` — the
  titles of the four commands that carried a `category` lost their redundant
  `sokonanoda:` prefix (`restart server`, `揭示下一条提示`, the two expand
  commands).

## [0.19.0] - 2026-09-13

### Added
- **`soko/version`** — the server reports its version and process id. The
  `sokonanoda: restart server` command now asks before and after, so the
  receipt shows `0.16.2 (pid 1001) → 0.19.0 (pid 2002)`: proof that the old
  process died and the new one is the new version.

### Changed
- The command palette entry is now `sokonanoda: restart server` (was
  `sokonanoda: 重启语言服务器`).

### Added
- **Value-position `apply`** — the second teaching keyword next to `intro`.
  `theorem t (h : Q -> P) : P := apply h` applies a proof/function to the
  goal and leaves its premises as holes (`h sorry`). Type parameters are
  filled from the goal automatically; premises you already supply
  (`apply (f p)`) are not duplicated. Completion, hover with the
  「展开为 apply 骨架」 button, and the "keeping it is equivalent" note all
  mirror `intro`'s; the editor command is `sokonanoda.expandApply`.
- New teaching error codes `elab-apply-needs-a-term` and
  `elab-apply-not-applicable`.
- **`intro` accepts an optional answer** — `theorem t : Q -> P := intro proofP`
  implicitly replaces the keyword with `fun (x : Q) => proofP`, so finishing a
  proof no longer requires expanding first. The answer must prove the final
  goal; the kernel still judges it.
- The hover expand-button payload now follows VS Code's command-link shape
  (a JSON **array** of arguments). The previous object payload made the
  button click silently do nothing; the client handler accepts both.

### Fixed
- Inlay hints no longer show the **first** sub-goal's type on every hole of
  a `by apply …` declaration that leaves several sub-goals open — those
  holes share one source position, so the lookup had to be positional
  (`[": p", ": p"]` → `[": p", ": q"]`).


## [0.17.0] - 2026-09-12

### Added
- **Hover `intro` → one-click expansion.** The hover on a value-position
  `intro` now carries an 「展开为 fun 骨架」 button that applies the same
  in-place edit as accepting the Tab completion — no need to catch the
  suggestion popup. The payload (document, hole range, skeleton) is computed
  by the language server and applied verbatim, so the button and Tab can
  never drift apart.

### Fixed
- The `intro` expansion (completion **and** hover) now survives the caret
  sitting just after the keyword — a trailing space on the same line, or the
  caret resting at the end of the token. Previously only the exact token byte
  range matched, so a wrapped `:=` … `intro` line looked like it "stopped
  working".

### Changed
- The `intro` hover now says outright that **keeping `intro` is equivalent**
  to the expanded `fun … => sorry` skeleton — expanding is a convenience,
  not a required step.

## [0.16.2] - 2026-09-12

### Added
- Hovering the value-position `intro` keyword now shows its explicit expansion
  (`fun (a : Prop) => … => sorry`), so the skeleton stays visible even when
  the completion suggestion is not accepted.

### Fixed
- Regression coverage for the real interaction: type `intro` and accept the
  selected suggestion — the token is replaced by the explicit skeleton. Also
  covers the end-of-token completion position fixed in 0.16.1.

## [0.16.1] - 2026-09-12

### Fixed
- The value-position `intro` expansion completion now appears when the caret
  is at the **end** of the keyword — the moment you finish typing it. The
  declaration lookup used an end-exclusive range, so the item was only
  offered with the caret strictly inside the token, which never happens while
  typing.

## [0.16.0] - 2026-09-12

### Added
- **Restart the language server in place** — the new command
  `sokonanoda: 重启语言服务器` re-resolves the server binary (rebuilt
  workspace build, refreshed cache, or a changed `sokonanoda.serverPath`) and
  restarts the client, so the editor picks it up without reloading the
  window. Extension-code updates still apply on window reload.

## [0.15.0] - 2026-09-12

### Added
- **Declaration binders (Lean-style)** — `theorem f (a : A) (h : B a) : C := v`
  is now valid: the declared type becomes the matching arrow telescope and the
  value gets its `fun`s wrapped automatically. Writing `:= sorry` reports the
  codomain goal with the binders already in context (no `intro` needed), a
  closed body needs no `fun`s either, and `by` blocks start from that context.
  Universe params and implicit binders stay unambiguous
  (`{u}` vs `{x : T}`).
- Course unit 1 gained a side-by-side section (arrow spelling vs declaration
  binders) plus exercise 6.

## [0.14.0] - 2026-09-12

### Added
- **Value-position `intro`** — write `intro` as the whole value
  (`theorem t : (a : Prop) -> a -> a := intro`) and the compiler unrolls every
  remaining binder into `fun … => sorry` (anonymous layers are named `x`,
  `x2`, …). It is a legal open exercise; the kernel still judges every fill.
  A goal with no binder left is rejected as `elab-intro-not-a-function`.
- **Expansion completion** — when the caret is on that `intro` keyword the
  completion list offers `intro（展开为 fun 骨架）`; accepting it replaces the
  keyword in place with the explicit skeleton. The hole's inlay hint shows the
  remaining goal right after `intro` (e.g. `: And b a`).

### Fixed
- Pretty-printed goals and binders no longer wrap application heads in a
  redundant parenthesis (`(And a) b` → `And a b`), matching the kernel's own
  printer and the learner's source.
- Comment-only edits no longer leave stale hole / sub-goal spans in
  incremental sessions; they are remapped together with the other cached
  spans.

## [0.13.0] - 2026-09-11

### Added
- **Onboarding lives in the CLI** — `sokonanoda` now ships
  `version` / `doctor` / `setup` / `update` / `grade` / `gate` subcommands
  (plus the existing `lsp`), replacing `scripts/soko.sh`. `setup` / `update`
  fetch the version-pinned CLI + LSP with a built-in downloader (no shell,
  cross-platform); `version` / `doctor` report the cached binaries' markers
  (`--json`); `grade` batches the `--json` judge view.

### Changed
- The cache version marker (`<version> <target>`) is shared with the VS Code
  extension and the opencode plugin; a stale or missing cache is refreshed
  from the pinned release (never `latest`).

### Removed
- `scripts/soko.sh` (superseded by the binary subcommands).

## [0.12.0] - 2026-09-11

### Added
- **`Type n` universe notation** — `Type n` is now accepted as
  `Sort (n + 1)`, Lean's spelling: `Type 0` = `Sort 1`, `Type 1` = `Sort 2`,
  and a bare `Type` stays `Sort 1`. `Type u` (a universe variable) is not
  supported — use `Sort u`.

### Fixed
- Reworded the `reserved-declaration-name` warning to plain language: it now
  says `Prop` / `Sort` / `Type` are already defined by the kernel (and notes
  `Prop`'s special role in formal proofs) instead of the coined term
  "built-in sort".

## [0.11.0] - 2026-09-11

### Added
- **Warns on declarations that reuse a kernel-defined name** — `Prop`, `Sort`
  and `Type` are already defined by the kernel, so a top-level declaration
  with one of those names still compiles but is never used. The editor now
  shows a `reserved-declaration-name` warning on the name, and the CLI emits
  a matching `warning` event.

## [0.10.0] - 2026-09-10

### Added
- **`#check` results stay visible** — like Lean's Infoview, the editor now
  shows each `#check` result as an inlay hint right after the checked
  expression (`#check Nat` → `Nat : Type 0`). The result comes from the
  same kernel pass that grades exercises, and survives incremental edits.

## [0.9.1] - 2026-09-10

### Fixed
- **Greek binder letters stay plain** — VS Code's confusable-character
  highlight (Trojan Source protection, on by default) drew a box around
  `α`/`β`/`γ` in `.sokonanoda` files. The extension now ships a
  `[sokonanoda]` configuration default that turns
  `editor.unicodeHighlight.ambiguousCharacters` off for this language only
  (the same approach VS Code itself uses for plaintext and markdown). Your
  global settings are untouched.

## [0.9.0] - 2026-09-10

### Added
- **CLI bundled too** — each platform package now ships the `sokonanoda` CLI
  next to the language server, so the 「课程」course map works out of the box
  (no separate `cargo build`, no PATH setup). Standalone per-platform CLI
  tarballs are also published on GitHub Releases for headless / agent use
  (`sokonanoda-cli-<triple>.tar.gz`, version-pinned).

### Changed
- CLI discovery is now: bundled `bin/<target>/sokonanoda` → workspace
  `target/{debug,release}` build → `PATH`.

## [0.8.0] - 2026-09-10

### Added
- **More platforms** — bundled packages now also cover `linux-arm64`,
  `alpine-x64`, `alpine-arm64` and `win32-arm64` (nine packages in total, like
  other major language extensions), so ARM and Alpine users get the
  zero-download experience too. Linux binaries are now built with a **glibc
  2.28 floor** (VS Code's own Linux minimum), fixing installs on older
  distributions; Alpine binaries are statically linked musl.

### Changed
- The universal fallback package now only serves platforms without a bundled
  build (e.g. Linux armhf).

## [0.7.0] - 2026-09-10

### Added
- **Bundled language server** — the `sokonanoda-lsp` binary now ships inside
  the extension as a platform-specific package (macOS arm64/x86_64, Linux
  x86_64, Windows x86_64). Installing the extension is now truly zero setup:
  no GitHub download on first use, works offline, and the kernel version is
  always the one the extension was built and tested with — no more client /
  server version skew.

### Changed
- Server discovery is now: `sokonanoda.serverPath` / `SOKONANODA_LSP_BIN` →
  bundled `bin/<target>/` → workspace `target/` build → version-pinned cached
  download. A lost executable bit on the bundled binary is repaired
  automatically.
- The fallback download (used only by the universal package for platforms
  without a bundled build) is pinned to this extension's own release tag
  instead of `releases/latest`, so it can never pull a newer, incompatible
  server.

## [0.6.0] - 2026-09-10

### Added
- **Goals at cursor** (「当前光标处」): with the caret inside a declaration the
  exercise tree now shows the goal, the hypotheses in scope and your `by`
  progress at that position — move the cursor and the panel follows
  (debounced ~200 ms). Clicking the goal reveals the corresponding tactic.
  Powered by the new `soko/stateAt` request; position → tactic selection is
  decided by the server, the client only renders.

## [0.5.2] - 2026-09-09

### Fixed
- **Wrong `by`-block / `sorry` span ranges**: `parse_by_block` ended the block
  span at "the next token" — comments are skipped by the lexer, so the span
  ballooned across trailing multi-line comments into the next declaration (or
  EOF), and the `sorry` warning squiggle covered those comment lines. The block
  span now ends at the last tactic, and the unclosed-goal hole points at the
  `sorry` token instead of offset 0.

## [0.5.1] - 2026-09-09

### Fixed
- **Stale language-server binary after extension updates**: the auto-downloaded
  `sokonanoda-lsp` was cached forever with no version check, so after upgrading
  the extension it still ran an older server (e.g. `by sorry` reported as an
  unknown tactic). The download is now version-tracked: when the extension
  version changes, the server is re-downloaded from the latest GitHub Release.
- **Axiom connectives highlight as types**: `And`/`Or`/`True`/`False` (declared
  via `axiom`) now map to the semantic-token `type` color instead of the nearly
  invisible `variable` color, matching Lean's treatment of the logical
  vocabulary.

## [0.5.0] - 2026-09-09

### Added
- **`by` tactic blocks** (Lean-style proofs): `theorem t : T := by intro a; exact h`
  with five tactics — `intro` / `exact` / `apply` / `assumption` / `rfl` — and a
  `by sorry` placeholder for incomplete proofs. Every tactic is kernel-judged.
  New course unit 6 (zh + en) teaches it; playground gains two `by` exercises.
- Hover now returns a highlight range so the editor shows which expression a
  hover describes (well-formed, paren-balanced expressions, real binder names).

## [0.4.2] - 2026-09-09

### Changed
- Marketplace listing rewritten to match reality: zero-setup story (the
  server downloads itself from GitHub Releases on first use, rust-analyzer
  model), full feature inventory, agent-skills promotion (teacher/dev/ci)
  and the `opencode.json` wiring; stale "build the server with cargo"
  quick start removed
- Keywords refreshed for marketplace discovery

### Docs
- New hard rule in docs/vscode-dev-guide.md §7: the three marketplace
  files (README.md / description / CHANGELOG.md) must be updated in the
  same commit whenever install story, feature set, feedback behavior or
  agent integration changes

## [0.4.1] - 2026-09-09

### Fixed
- Bracket hover: `(expr)` shows `expr : type` on both `(` and `)` (paren
  matching with `--` comment skipping); no longer leaks the neighbor's
  signature on `)` (was `And.left : forall …` bug)
- Hover types show real binder names instead of de Bruijn indices
  (`$3 -> $4` was leaking on partially applied functions)
- `Not a` stays folded in hover (was unfolded to `a -> False`)
- Double hover fallbacks consolidated; `goto-def` on `)` no longer jumps
  to a neighbor identifier

### Changed
- Kernel display layer (cold path, documented in architecture.md §6):
  pp binder-name seeding for scope variables; `infer_under_binders` quotes
  without `force_all`

## [0.4.0] - 2026-09-09

### Added
- Hover proximity fallback: brackets/operators show enclosing expression type
- VS Code integration tests (@vscode/test-electron, 4 cases + CI xvfb)
- Multi-binder lambda regression test

### Fixed
- Hover loose bvars resolved to binder names (was "1 -> 1")
- Keyword hover suppressed (fun/=>/theorem silent)
- Conv soundness fix (kernel eval/infer closure conflation)

### Changed
- Hover precision limitation documented: infer_under_binders panics on
  delta-unfolding types (Not a), affected rows dropped (宁缺毋滥)

## [0.3.0] - 2026-09-09

### Added
- Hover on brackets/operators shows enclosing expression type (proximity fallback)
- Declaration hover shows full kernel-rendered signature (ty_text)
- Hover on sub-expressions shows `expr : type` (precedence visible)
- Keyword hover suppressed (clean, no noise on fun/=>/theorem)
- Multi-binder lambda support confirmed and regression-tested
- `sorry` highlight in semantic tokens and TextMate grammar

### Fixed
- Hover loose bvars resolved to binder names (was showing "1 -> 1")
- Hover rows with unresolvable names dropped (宁缺毋滥，不展示乱码)
- Conv soundness fix (upstream kernel bug: eval/infer closure conflation)

### Changed
- `???` removed; `sorry` is the sole placeholder (Lean 4 parity)


## [0.2.1] - 2026-09-08

### Changed
- CI auto-publishes to VS Code Marketplace on tag push
- Version bump to test release pipeline
## [0.2.0] - 2026-09-07

### Added
- Go-to-definition, document highlight, binder completions
- Exercise panel (练习 tree) with goal/hypotheses/hole navigation
- `alt+n` / `alt+shift+n` hole navigation
- Semantic highlighting
- `soko/goals` + `soko/nextHole` custom LSP requests
- Inlay hints (expected types at hole positions)
- Rename + find references
- Hint ladders (`-- soko:hint` directives, revealed one at a time)
- `sorry` as placeholder (Lean 4 parity, ??? removed)
- Multi-hole constructor spines with refine suggestions

### Changed
- Declaration hover shows full kernel-rendered signature
- Hover on sub-expressions shows `expr : type` (precedence visible)
- Keyword hover suppressed (clean, no noise)
- `sorry` produces warning-level diagnostic (not error)

### Fixed
- Conv soundness fix (upstream kernel bug: eval/infer closure conflation)
- Loose bvar rendering (hover showed "1 -> 1" instead of named binders)
- VSIX packaging (vscode-languageclient now properly bundled)

### Removed
- `???` placeholder (replaced by `sorry` for Lean 4 parity)

# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-09-07

### Added

- Initial LSP feedback channel for `.sokonanoda` files: diagnostics, hover, goal view, CodeLens, semantic tokens, and code actions.

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

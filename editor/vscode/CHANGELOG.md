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

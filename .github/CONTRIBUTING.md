# Contributing

Thanks for helping improve sokonanoda-lang.

## Before you start

- Read [`AGENTS.md`](AGENTS.md) and [`docs/ONBOARDING.md`](docs/ONBOARDING.md).
- Install the pinned toolchain with `scripts/soko setup`; validate with `scripts/soko doctor --json`.
- For user-facing changes, update the matching docs/skills/tests in the same change.

## Pull requests

- Keep changes focused; one coherent change per PR.
- Run the relevant checks locally before pushing:
  - `scripts/soko gate --fast` for ordinary code changes; `scripts/soko gate` before a batch/release.
  - `python3 scripts/notation-lint.py` for course Lean-style syntax.
  - `python3 scripts/docs-lint.py` for docs-budget and expiry rules.
  - `python3 courses/analysis/tools/check.py --json` for the analysis course.
- Do not commit generated caches or `dist/` outputs unless the task explicitly requires a committed artifact.
- Describe the user-visible result, the checks you ran, and any remaining limitation in the PR body.

## License

By contributing, you agree that your contributions are licensed under the Apache License 2.0 (see [`LICENSE`](LICENSE)).

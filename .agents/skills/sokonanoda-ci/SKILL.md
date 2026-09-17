---
name: sokonanoda-ci
description: Ship and monitor GitHub Actions for sokonanoda-lang without repeat failures - exact local-verification commands (exit codes, no grep masks), workflow pitfalls (download-artifact v4 layout, tag-pinned workflow files, gh release idempotency, vsce flags), run triage with the gh CLI, and the CI-FAILURES.md ledger duty. Load this when pushing, tagging, releasing, or debugging CI/Actions failures in this repository. The full manual lives at skills/sokonanoda-ci/SKILL.md from the repository root.
---

# sokonanoda-ci (DeepSeek Harness entry)

The complete operating manual for this role is the repository file
`skills/sokonanoda-ci/SKILL.md` (relative to the sokonanoda-lang repository root
— resolve it against the repository root, e.g. the `.git` root of the workspace,
not against this entry's directory).

Read it **before** pushing, tagging, releasing, or triaging a failing run. It is
the single source of truth for:

- the exact local verification commands that must match CI, with real exit codes
  and **no grep masking**;
- the GitHub Actions pitfall ledger (artifact layouts, tag-pinned workflow
  files, `gh release` idempotency, `vsce` flags, `bash -e` semantics,
  skipped-step false greens);
- the release flow (bump both versions → push main → `ci.yml` auto-tag →
  dispatched `release.yml`) and when a manual tag is the emergency path;
- the `gh` triage routine and the duty to append every CI failure to
  `docs/CI-FAILURES.md`.

> On DeepSeek Harness, `scripts/soko gate` is the local gate (it needs cargo and
> calls it directly). No harness plugin runs it for you, and exit codes must be
> read from the command itself, never from a pipe's tail.

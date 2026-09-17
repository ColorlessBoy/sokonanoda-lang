---
name: sokonanoda-dev
description: Develop and extend the sokonanoda-lang teaching compiler stack (Rust workspace kernel/front/cli/lsp) safely - the frozen kernel, TDD three-layer testing, modularization limits, docs-first workflow and CI gates. Load this when taking over development, adding syntax or compiler features, or touching CI/docs in this repository. The full manual lives at skills/sokonanoda-dev/SKILL.md from the repository root.
---

# sokonanoda-dev (DeepSeek Harness entry)

The complete operating manual for this role is the repository file
`skills/sokonanoda-dev/SKILL.md` (relative to the sokonanoda-lang repository
root — resolve it against the repository root, e.g. the `.git` root of the
workspace, not against this entry's directory).

Read it **before** acting, then follow it exactly. It is the single source of
truth for:

- the takeover reading order (`AGENTS.md` → `REQUIREMENTS.md` →
  `docs/HANDOVER.md` → `STATUS.md` → `ROADMAP.md` → `docs/README.md` →
  `docs/architecture.md` → `docs/design/`);
- the inviolable hard rules (frozen kernel, no official Lean toolchain, teaching
  syntax as a real Lean 4 subset, kernel-only judgement, structured feedback);
- the TDD three-layer workflow (front unit → CLI end-to-end → corpus/golden);
- the environment commands (`scripts/soko doctor --json`, `scripts/soko setup`,
  `scripts/soko gate` for contributors);
- the closing obligations (update `STATUS.md`, append new user requirements to
  `REQUIREMENTS.md` §9 with a date, design-first docs).

> On DeepSeek Harness there is no project-level plugin or hook: scripts under
> `scripts/` are invoked directly, and `lean`/`lake`/`elan`/`leanc` must never be
> used (`REQUIREMENTS.md` §2 rule 9).

---
name: sokonanoda-prep
description: Curriculum preparation (备课) role for sokonanoda — audit and improve knowledge-point organization and the question bank in courses/set-theory/knowledge/data, validate with validate.py / coverage.py / verify_batch.py / engine.py selftest+simulate, and report coverage and batch deltas. Load this when the user wants to add or improve knowledge points, lessons, cards, or exercises for the course. The full manual lives at skills/sokonanoda-prep/SKILL.md from the repository root.
---

# sokonanoda-prep (DeepSeek Harness entry)

The complete operating manual for this role is the repository file
`skills/sokonanoda-prep/SKILL.md` (relative to the sokonanoda-lang repository
root — resolve it against the repository root, e.g. the `.git` root of the
workspace, not against this entry's directory).

Read it **before** acting, then follow it exactly. It is the single source of
truth for:

- the read-first set (`courses/set-theory/knowledge/README.md`,
  `CONTENT-PLAN.md`, `DESIGN.md`, `data/*.json`, `docs/design/course-stdlib.md`);
- the baseline audit and the batch workflow (staging → `verify_batch.py` →
  merge → `validate.py` / `coverage.py` / `engine.py selftest` / `simulate` /
  `scripts/notation-lint.py`);
- question-writing rules (stable ids, `source` verification, distractors,
  rubric for short/explain/proof, notation style);
- knowledge-point organization rules (`kind`, `prereq` DAG, `saturation`,
  `keyPoint`, split/merge synchronization);
- the report template and the hard boundary: content assets only — no kernel,
  engine or renderer edits.

Inviolable rules (restated here so a mis-load is still safe):

1. Curriculum truth comes from the course sources and `source` lines — inspect
   them; never judge by eyeballing prose.
2. Only edit content assets under `courses/set-theory/knowledge/` plus the
   necessary unit references; send kernel/engine/renderer work to
   `sokonanoda-dev`.
3. Every batch must pass `validate.py`, `coverage.py`, `engine.py selftest`,
   `engine.py simulate`, and `scripts/notation-lint.py` before delivery.
4. `out/` and `reports/` are generated artifacts — never hand-edit them.
5. Large changes (≥20 questions, splitting/merging KPs, chapter-plan changes)
   present the plan first and land only after user confirmation.

> On DeepSeek Harness this skill is loaded with `/sokonanoda-prep`; on opencode
> the same body is mounted as the primary `prep` agent (see
> `.opencode/agent/prep.md`). Both resolve relative paths against the repository
> root. DSH has no project-level agent definitions — the skill is the role.

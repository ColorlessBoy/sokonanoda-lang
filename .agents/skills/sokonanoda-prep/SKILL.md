---
name: sokonanoda-prep
description: Curriculum preparation (备课) role for sokonanoda — audit and improve knowledge-point organization and the question bank in courses/set-theory/knowledge/data, keep the derived code-exercise dataset in courses/exercise-dataset in sync (build.py --check / check.py), validate with validate.py / coverage.py / verify_batch.py / engine.py selftest+simulate, and report coverage and batch deltas. Load this when the user wants to add or improve knowledge points, lessons, cards, exercises, or the code-exercise dataset for the course. The full manual lives at skills/sokonanoda-prep/SKILL.md from the repository root.
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
  `scripts/notation-lint.py` / `knowledge/text_lint.py`);
- text axioms X1–X10 and the `text_lint.py` gate (red/green fixtures, false-positive
  governance, shrink-only backlog baseline) in `references/text-axioms.md`;
- card-coverage gate (`scripts/card-lint.py`): lemmas named in a question's options must
  appear in the knowledge card that `explain_for` actually pushes; false-positive
  governance, known limits and the backlog workflow in `references/card-coverage.md`;
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
   `engine.py simulate`, `scripts/notation-lint.py`, `knowledge/text_lint.py`, and
   `scripts/card-lint.py` (errors 0; backlog baselines shrink-only) before delivery.
4. `out/` and `reports/` are generated artifacts — never hand-edit them.
5. Large changes (≥20 questions, splitting/merging KPs, chapter-plan changes)
   present the plan first and land only after user confirmation.

> On DeepSeek Harness this skill is loaded with `/sokonanoda-prep`; on opencode
> the same body is mounted as the primary `prep` agent (see
> `.opencode/agent/prep.md`). Both resolve relative paths against the repository
> root. DSH has no project-level agent definitions — the skill is the role.

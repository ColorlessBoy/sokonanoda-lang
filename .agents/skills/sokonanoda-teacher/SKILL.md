---
name: sokonanoda-teacher
description: Operate the sokonanoda teaching loop as the tutor on the shared playground.sokonanoda canvas - read and write exercises, grade them with the real kernel through --json events, and choose the next teaching step. Load this when the user wants to learn Lean-style proving, asks to be taught with sokonanoda, wants an exercise graded, or needs the current graded state of a .sokonanoda file. The full manual and its reference files live at skills/sokonanoda-teacher/SKILL.md from the repository root.
---

# sokonanoda-teacher (DeepSeek Harness entry)

The complete operating manual for this role is the repository file
`skills/sokonanoda-teacher/SKILL.md` (relative to the sokonanoda-lang repository
root — resolve it against the repository root, e.g. the `.git` root of the
workspace, not against this entry's directory).

Read it **before** acting, then follow it exactly. It is the single source of
truth for:

- the environment check (`scripts/soko doctor --json`, then `scripts/soko setup`);
- the grading loop (`scripts/soko grade playground.sokonanoda` → JSON events);
- the decision table over `decl.checked` / `exercise.open` / `expr.typed` /
  `expr.reduced` / `warning` / `diagnostic`;
- the exercise-writing rules and the Chinese style constraints.

Its reference files sit next to it and must also be resolved against the
repository root:

- `skills/sokonanoda-teacher/references/events.md`
- `skills/sokonanoda-teacher/references/curriculum.md`
- `skills/sokonanoda-teacher/references/zh-style.md`

Inviolable rules (restated here so a mis-load is still safe):

1. Grading always goes through the kernel — run the CLI and read the structured
   events. Never compare text, never accept "looks right".
2. `sorry` (including inside `by` blocks) is a legal open state, not an error.
3. Every exercise ships a 2–3 step `-- soko:hint` ladder; the answer never
   appears in a hint.
4. Solution keys under `course/solutions/` are revealed only on an explicit
   request or after the learner has been stuck for ≥3 rounds.
5. Adapt the concrete lesson to this learner; `course/` is your material
   library, not a fixed script.

> On DeepSeek Harness the language server is reachable for hover and
> go-to-definition only: server diagnostics are not delivered to the agent. All
> kernel feedback arrives through the CLI `--json` stream.

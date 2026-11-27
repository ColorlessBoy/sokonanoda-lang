# FAQ — likely questions and honest answers

Use these in HN/PH/V2EX/知乎 replies. Keep replies short in-thread; link the
relevant section instead of pasting the whole answer. Adjust nothing here without
checking the claim ledger in `README.md`.

## 1. Is this Lean 4?

No. Sokonanoda is a **teaching subset** of Lean 4 syntax and semantics, checked by
a Lean-4-compatible kernel. It is not a Lean distribution and is not affiliated
with the Lean project. If you want Lean, use Lean.

## 2. Can I use mathlib / import my Lean files?

No. There is no mathlib and no Lean import path. The language covers the syntax
and tactics the three courses use (intro, exact, apply, rfl, cases, induction-style
recursion, `by` blocks, etc.). Real Lean files will not parse.

## 3. How is grading different from an LLM or a regex?

Grading never calls a model. The CLI/extension elaborates the declaration and runs
the kernel; the result is `decl.checked`, `exercise.open` or a `diagnostic`. The
goal panel is the same kernel's `query state` output, recomputed at the cursor.
Text matching was deliberately removed from the teaching tactics path.

## 4. Is the kernel proven correct?

No kernel is. This one is an independent Rust implementation, a fork of
[intgrah/sokonanoda](https://github.com/intgrah/sokonanoda) pinned at `7b51784`
(Apache-2.0). It is tested against the Lean Kernel Arena corpus: on
`master@30e64c1b2`, 135/138 accept and 90/92 reject, with 3 performance timeouts
and 2 unsupported cases recorded in the repo. Treat it like any other kernel: a
large test surface, not a proof of correctness.

## 5. Why not just use Lean + elan / Lean4Web?

Use Lean for real work; it has mathlib and a mature ecosystem. This exists for the
first hour and for classrooms: one extension, no toolchain, guided goals, instant
grading. Those are different products with different trade-offs.

## 6. Does it work offline? Which platforms?

Yes. The VS Code extension ships the language server and CLI per platform:
macOS arm64/x64, Linux glibc and musl (x64/arm64), Windows x64/arm64, plus a
universal fallback VSIX. No runtime download and no account.

## 7. Is it free? What licence?

Free and open source, Apache-2.0. Courses and the exercise dataset are in the same
repository.

## 8. Does it collect data or send my code anywhere?

The grading path is local. The extension has no telemetry in the proof-checking
path and no account. If you use the optional agent skills, the model provider you
configure is a separate choice; that is documented in the DSH plugin README.

## 9. Can an AI agent use it?

Yes. The repo ships agent skills for teaching and development, and `--json` exposes
stable events (`decl.checked` / `exercise.open` / `diagnostic`). A DSH plugin adds
quiz cards and code-exercise grading. The grading itself still goes through the
kernel.

## 10. Why are the screenshots/courses Chinese?

The project is Chinese-first: the author teaches in Chinese and the courses were
written for that. The site is bilingual, and the intro course has a full English
mirror. Set theory and analysis are Chinese-only today. English course coverage is
the main gap and the fairest criticism of an English launch.

## 11. How big are the courses?

298 units / 2,022 exercises, all with kernel-checked solutions:

- Quick start: 11 units / 66 exercises (English mirror available)
- Set theory (Vol. I): 112 units / 910 exercises
- Analysis (Vol. II–III): 175 units / 1,046 exercises

There are also 409 quiz questions over 195 knowledge points.

## 12. Who is it for?

Learners who want to try formal proofs without a toolchain, teachers who want
auto-graded proof exercises, and developers interested in a kernel-graded teaching
stack. It is not for research-level formalisation.

## 13. Why Rust? Why a fork?

The upstream kernel is a Rust implementation of the Lean 4 kernel; keeping the
kernel crate and its tests made the arena results comparable and the provenance
clear. The new work is the teaching front end, LSP, CLI, courses and tooling.

## 14. Can I contribute a course / an English translation?

Yes — issues and PRs are welcome. English course coverage and more tactics are the
highest-value contributions. Read `AGENTS.md` and `docs/ONBOARDING.md` first; the
repo has hard rules for course changes and kernel changes.

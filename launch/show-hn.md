# Show HN — v0.100.0

Status: **ready to post**. Nothing here is submitted automatically; the owner
posts from an account with some history (a brand-new account is likely to be
flagged). Read `README.md` for the ordering and the claim ledger.

## Recommended post

- **Type:** link post
- **URL:** https://colorlessboy.github.io/sokonanoda-lang/
- **Title (pick one):**

| # | Title | Chars |
|---|---|---:|
| 1 | `Show HN: Sokonanoda – learn formal proofs without installing Lean` | 65 |
| 2 | `Show HN: A Lean-style proof language for learners, no toolchain` | 63 |
| 3 | `Show HN: Learn formal proofs in VS Code, checked by a real kernel` | 65 |

Use **1** unless the owner prefers not to lead with the product name.

## First comment (post immediately after submitting)

> Author here. I built Sokonanoda because every time I tried to teach formal
> proofs, the first session disappeared into tooling instead of proofs. The goal
> was to make minute one be "write a proof and get an answer".
>
> What it is: a Lean-4-style teaching language checked by a real kernel. Open a
> `.sokonanoda` file in VS Code, put the cursor on a `sorry`, and the panel shows
> the goal state computed by the kernel. Fill it in, save, and the declaration is
> either checked or rejected. No text matching, no LLM judging.
>
> What it is not: Lean. It is a deliberately small subset — no mathlib, no
> importing Lean files, and only the tactics the courses use. If you want real
> Lean, use Lean.
>
> The kernel is a fork of IntGrah's sokonanoda kernel (Apache-2.0), an
> independent Rust implementation of the Lean 4 kernel; the teaching front end,
> LSP, CLI, courses and tooling are the new part. On the Lean Kernel Arena corpus
> (`master@30e64c1b2`) it accepts 135/138 and rejects 90/92; the misses are 3
> performance timeouts and 2 unsupported cases tracked in the repo.
>
> There are three courses — intro logic, set theory, analysis — 298 units and
> 2,022 exercises in total, all with kernel-checked solutions. The extension
> ships the LSP and CLI binaries for macOS/Linux/Windows, so it works offline.
>
> Known gaps: courses are Chinese-first (the intro course has an English mirror;
> the other two do not), the tactic set is small, and there is no mathlib. I'd
> especially like feedback on the teaching flow and on what a "real Lean user"
> misses most.
>
> Site: https://colorlessboy.github.io/sokonanoda-lang/
> Repo: https://github.com/ColorlessBoy/sokonanoda-lang

## Text-post version (only if a link post is rejected)

Use the same title, then this body, with the site URL on its own line:

> I built a self-contained teaching language for formal proofs. It is a small
> Lean-4-style subset, but every exercise is checked by a real kernel, not text
> matching. The VS Code extension ships the language server and CLI, so there is
> nothing to install but the extension — no Lean, no Rust, works offline.
>
> Three courses, 298 units, 2,022 exercises with kernel-checked solutions. The
> kernel is a fork of the independent sokonanoda kernel (Apache-2.0, Rust), and it
> passes 135/138 accept / 90/92 reject on the Lean Kernel Arena corpus.
>
> It is not a Lean replacement: no mathlib, no Lean imports, limited tactics.
> Courses are Chinese-first, with an English mirror for the intro course.
>
> https://colorlessboy.github.io/sokonanoda-lang/

## Posting rules that matter on HN

- Do **not** ask for upvotes, share the link in group chats for votes, or create a
  second account. HN penalises voting rings and duplicate submissions.
- One submission only. If it does not get traction, do not repost the same URL;
  wait at least a few weeks and only with a materially different angle.
- Reply to technical questions in the thread. Link the FAQ below rather than
  repeating long answers; keep replies short and specific.
- Do not edit the title after submission to add keywords. Fix typos only.
- If criticism is right, say so and file an issue. HN rewards that more than
  defensiveness.

## Likely questions (short replies; full versions in `faq.md`)

| Question | One-line reply |
|---|---|
| "Is this Lean?" | No — a teaching subset that uses a Lean-4-compatible kernel. No mathlib, no Lean imports. |
| "Why not just use Lean?" | Use Lean for real work. This is for the first hour: zero install, guided goals, graded exercises. |
| "Is the kernel proven correct?" | No kernel is; this one is an independent Rust implementation with an arena corpus and a public test suite. |
| "How do you grade?" | The kernel checks the declaration. `--json` exposes `decl.checked` / `exercise.open` / `diagnostic`. |
| "Did an LLM write it?" | "The grading path never calls a model. The stack is public with tests and CI — judge the artifact. I'll be explicit about which parts were agent-assisted." |
| "Why Chinese screenshots?" | The courses are Chinese-first; the intro course has an English mirror. English course coverage is the main gap. |
| "Can I use my Lean files?" | No. The syntax is a subset and there is no Lean import path. |
| "Does it phone home?" | No. The extension is offline; no account and no telemetry in the grading path. |

**Owner note on the AI question:** answer it honestly and specifically — the kit
cannot know the actual split, so do not deny agent assistance if it happened. The
verifiable claim is narrow: the grading path never calls a model, and every exercise
is checked by the kernel. Do not paste an answer that implies a hand-written split
that is not true.

# Product Hunt — v0.100.0

Status: **ready to schedule**. The owner creates the product page and launches from
the maker account; nothing here is auto-submitted. See `README.md` for ordering.

## Listing

| Field | Value |
|---|---|
| Name | `Sokonanoda` |
| Tagline (≤60) | `Learn formal proofs in VS Code, no Lean install` (47) |
| Website | https://colorlessboy.github.io/sokonanoda-lang/ |
| GitHub | https://github.com/ColorlessBoy/sokonanoda-lang |
| Marketplace | https://marketplace.visualstudio.com/items?itemName=sokonanoda-lang.sokonanoda |
| Pricing | Free · open source (Apache-2.0) |
| Topics | Education · Developer Tools · Open Source · Science · Productivity |
| Thumbnail | `assets/icon-240.png` (240×240) |
| Gallery | `assets/ph-1-hero.png`, `assets/ph-2-kernel.png`, `assets/ph-3-courses.png` (all 1270×760) |
| Video / GIF | `assets/demo-vscode.gif` (843×390, 7 s) if the gallery accepts a GIF; otherwise use `portrait.png` as the fourth image |

## Description (≤260 chars)

> A Lean-4-style language for learning formal proofs. Write a line, see the
> kernel-computed goal, get graded by a real type checker — not text matching. The
> VS Code extension ships everything: no Lean/Rust install, works offline. 2,022
> exercises.

**244 characters.** Re-check with `python3 -c` before pasting; Product Hunt's
counter counts the em dash and spaces.

## First comment (from the maker account, as soon as it goes live)

> Hi Product Hunt! I built Sokonanoda because teaching formal proofs kept starting
> with an installation session instead of a proof. The extension ships the
> language server and CLI, so the first screen is a lesson and the first action is
> writing a proof.
>
> The part I care about is the feedback loop: put the cursor on a `sorry` and the
> panel shows the goal state that the kernel actually computed. Fill it, save, and
> the declaration is accepted or rejected by a complete Lean-4-compatible type
> checker — no regex, no LLM judging.
>
> It is deliberately not Lean: no mathlib, no importing Lean files, and only the
> tactics the courses use. If you want real Lean, use Lean. This is for the first
> hour and for classrooms where installing a toolchain is the barrier.
>
> Three courses (intro logic, set theory, analysis), 298 units, 2,022 exercises
> with kernel-checked solutions. Free, Apache-2.0, offline after install.
>
> I'd love feedback on the teaching flow: where do you get stuck, and what would
> make the next step obvious? I'll be here all day.

## Gallery captions

1. **Write the proof. See the kernel's answer.** — a real VS Code capture of the
   published extension, with the goal panel on the right.
2. **No heuristics. The kernel decides.** — `decl.checked` / `exercise.open` /
   `diagnostic` are the same events the editor, CLI and course gate consume.
3. **2,022 exercises. Every one kernel-checked.** — the three courses and their
   unit/exercise counts.

## Scheduling notes

- Launch goes live at **12:01 a.m. Pacific**. The first 4–6 hours matter most;
  block the day for replies.
- Create the product page **at least 2–3 days ahead** so the gallery processes and
  the maker profile is complete. Add all three gallery images, the 240×240 icon,
  and the demo GIF if accepted.
- Do not offer incentives, ask for upvotes, or coordinate a voting group. Ask for
  **feedback** instead; PH penalises vote solicitation.
- Reply to every comment in the first 12 hours. Short, specific answers beat
  repeated marketing copy; link `faq.md` only when the answer is long.
- Do not launch the same day as Hacker News. The recommended order in `README.md`
  separates them so each gets a full day of replies.

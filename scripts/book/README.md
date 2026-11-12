# Interactive analysis book (v2) — M1+M2, 72 units

> **Status:** the C8.1 reading product, prototype v2. One self-contained HTML
> page built from `courses/analysis/` — the cover, volume/chapter TOC, all
> current units, and the solutions appendix. The previous v1 sample
> (II.1 only, 19 units) is superseded by this builder.

The book currently renders **7 chapters / 72 units** from the course manifest:
II.1 (19 units), II.2 (9), II.3 (6), II.4 (10), II.5 (10), II.6 (14),
II.7 (4), plus **343 exercises**, **1016 declarations**, **1179 hints**, and
**343 solutions** (plus 38 solution-only declarations). All counts are read
from the source tree; nothing is hardcoded by chapter or unit count.

## Build / check

From the repository root:

```bash
python3 scripts/build_book_proto.py --out dist/book/book-v2
python3 scripts/build_book_proto.py --check --out dist/book/book-v2
python3 scripts/build_book_proto.py --selftest
```

`dist/book/` is ignored except for its `.gitignore`; the committed review copy
lives in `scripts/book/preview/index.html`:

```bash
python3 scripts/build_book_proto.py --out scripts/book/preview
python3 scripts/build_book_proto.py --check --check-preview
```

Preview link (raw.githack, branch build):

<https://raw.githack.com/ColorlessBoy/sokonanoda-lang/agent/book-prototype/scripts/book/preview/index.html>

`--check` validates the generated file against the live course manifest. It
does not judge proofs and never edits `courses/**`.

## What is on the page

- **Cover** with live counts (chapters / units / declarations / exercises /
  hints / solutions), reading guide, and scope note.
- **TOC** grouped by volume and chapter, with per-chapter unit counts and a
  link to every unit; stable anchors `#chapter-ii-1` … `#chapter-ii-7`,
  `#unit-1` … `#unit-72`, `#appendix-solutions`.
- **Chapter headings** (`II.1` … `II.7`) with chapter-level counts.
- **Per-unit sections**: unit title (Chinese + English), local/global unit
  position, declaration/exercise/hint counts, prev/next unit navigation, and a
  stable `#unit-N` anchor.
- **Prose comments** (`-- ...`) become paragraphs, numbered headings, bullet
  lists, and decorative rules.
- **Declarations** (`def` / `theorem` / `axiom` / `inductive`) as cards:
  signature, full definition code for `def`, collapsible proof steps for
  `theorem`, full code for `axiom` / `inductive`.
- **`soko:hint` groups** as collapsible hint panels (label + body).
- **Exercise cards** for declarations containing `sorry`: the full exercise
  body (including the hole), hints, an inline solution toggle, and a link to
  the matching solutions-appendix entry.
- **Solutions appendix** grouped by unit with one collapsible entry per
  exercise, plus the solution-only declarations that do not appear on the
  canvas.

## Reading controls

- **并读 / 直觉 / 形式** layer switch (`body[data-view]`). 直觉 keeps prose,
  hints, formulas, and target statements; 形式 keeps declarations, proofs,
  exercise code, and solutions, hiding prose/hints; 并读 shows both.
- **章节** select jumps to any chapter; plain links still work without JS.
- **展开全部 / 折叠全部** operate on every `<details>` (proofs, hints,
  solutions, appendix entries).
- **打印 / PDF** calls `window.print()`; `beforeprint` opens every panel so
  the printed book includes collapsed proofs, hints, and solutions, and
  `afterprint` restores the reader's screen state.
- **localStorage** remembers the layer choice (`soko-book-v2-view`); failures
  under `file://` or private mode are swallowed, and the control still works
  for the session.
- **Mobile/desktop** tested at 390 px and 1440 px: no horizontal overflow.
  At ≤620 px the chapter select is hidden (the TOC remains), the toolbar wraps,
  and prev/next navigation stacks.

## Math

- Unicode math runs in prose get a serif `.math` span; display formulas in
  comments become `.math-display` blocks.
- A small stdlib helper converts simple fractions, `x²` / `x^2` superscripts,
  and relation/arrow symbols to native MathML (`<mfrac>`, `<msup>`, `<mo>`).
- There is **no CDN, no web font, no front-end math library**. If a display
  line is code-shaped (`:=`, `fun`, `match`, `@…`) or too complex, it falls
  back to a readable Unicode / monospace display block; the check verifies
  that the fallback still contains the formula text. No content is dropped.

## Print

`@page { size: A4; margin: 16mm 14mm 17mm; }`. Cover and TOC end with page
breaks; every chapter starts on a new page, and each subsequent unit gets its
own page; the appendix starts on its own page. Code and display math switch to
`white-space: pre-wrap` so nothing is clipped. With all panels open the
current book prints to **1012 A4 pages** (measured with headless Chrome
`Page.printToPDF`); print the whole book or use the browser's page range.

## Check contract

`--check` fails on any of:

- generated unit count ≠ `course.json` unit count, or per-chapter unit counts
  that do not match the manifest;
- unit files present under `courses/analysis/units/**` but absent from
  `course.json` (so a new unit cannot be silently ignored);
- an empty unit, a unit without declarations/exercises, or an exercise with
  no solution matched by declaration name;
- unparsed top-level source lines (reported as `unit N: unparsed source lines`);
- TOC/prev/next/appendix links that do not resolve, duplicate `id`s, or
  missing chapter/unit/appendix anchors;
- zero declarations / exercises / hints / solutions, or solution count ≠
  exercise count;
- any `http://` / `https://`, `<link>`, protocol-relative `src`/`href`, or CSS
  `@import` in the output;
- missing A4 print CSS or layer/control hooks;
- output that differs from a fresh render (stale build);
- formula fallbacks that drop their text, or MathML helper regressions.

`--selftest` additionally proves the checker bites: injected external URL,
broken anchor, duplicate `id`, and stale output must each be rejected.

## Evidence

Headless Chrome (CDP) checks at 1440×1000 and 390×844 assert:

- exactly 72 unit sections, 7 chapter sections, 343 exercise cards;
- 72 TOC unit links and 7 TOC chapter links, all anchors resolve;
- no duplicate `id`s, no empty unit, `documentElement.scrollWidth <= innerWidth`;
- 并读/直觉/形式 hide/show the right layers;
- 展开全部 opens all 2571 `<details>`, 折叠全部 closes them;
- the chapter select changes the hash; localStorage restores the layer after
  reload; print media hides the toolbar and applies page breaks.

Representative screenshots are committed in `scripts/book/screenshots/`
(`cover.png`, `toc.png`, `unit-ii7.png`, `formal.png`, `mobile.png`). They are
regenerated with `node scripts/book/capture.mjs` after a build; the script
prints the DOM assertion report and writes the same filenames to
`/tmp/book-v2-*.png`.

## Limits / known gaps

- No search, no reading-progress tracking, no per-exercise URL state beyond
  `#unit-N` / `#ex-N-K`.
- Not a proof assistant: exercises and solutions are presented, not judged.
  The course gate remains the judge.
- Math coverage is intentionally small: only simple fractions/superscripts
  are structured. Everything else stays Unicode or monospace.
- Print proofing at ~1000 pages is fine in Chrome but not yet checked on other
  PDF engines.
- If future M3 units are added to `course.json` and the unit files, they appear
  automatically. If unit files appear without a manifest entry, `--check`
  fails loudly rather than silently omitting them.

## Files

- `scripts/build_book_proto.py` — discovery, parser, renderer, `--check`,
  `--selftest`, preview check.
- `scripts/book/style.css` — screen + A4 print styles (inlined by the builder).
- `scripts/book/app.js` — layers, chapter jump, details controls, print pass,
  localStorage memory (inlined).
- `scripts/book/capture.mjs` — headless Chrome CDP DOM test + screenshots.
- `scripts/book/preview/index.html` — committed self-contained review copy.
- `scripts/book/screenshots/` — committed evidence images.
- `dist/book/.gitignore` — keeps generated pages out of git.

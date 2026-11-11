# II.1 interactive HTML book prototype

> Scope: **prototype**, not the full C8.1 book build. It renders the existing
> 19-unit II.1 canvas (`courses/analysis/units/II.1/`) and its solution files
> (`courses/analysis/units/solutions/II.1/`) as one static, dependency-free
> HTML page. It does not judge proofs, call the kernel, or edit course files.

## Build / check

From the repository root:

```bash
python3 scripts/build_book_proto.py --out dist/book/proto-II.1
python3 scripts/build_book_proto.py --check --out dist/book/proto-II.1
```

The output is `dist/book/proto-II.1/index.html`; `dist/book/` ignores generated
pages, so the HTML is intentionally not committed. `--check` validates the
already-generated file (it fails if the file is missing, stale, or outside the
prototype contract) and also runs a direct fraction / superscript MathML
self-test.

This is **not** the course gate. The course gate remains
`python3 courses/analysis/tools/check.py` (with a fresh/pinned CLI), and the
prototype does not modify `courses/**`.

Evidence screenshots for the prototype PR are generated with headless Chrome to
`/tmp/book-proto-*.png` and an A4 print sample to `/tmp/book-proto-print.pdf`;
they are not committed.

## What is on the page

- Cover, chapter TOC, and 19 unit sections.
- Unit `title` + `title_en` from `course.json`.
- `-- ...` comment lines as prose paragraphs; decorative comments become
  rules, and numbered headings such as `27.1 ...` become subsection headings.
- `def` / `theorem` / `axiom` / `inductive` declarations as cards: signature,
  full definition code, and a collapsible proof panel whose body is split into
  top-level steps.
- `-- soko:hint` comment groups as collapsible hints (label + body).
- Declarations containing `sorry` as exercise cards with statement, hints,
  and an exercise/solution toggle.
- A solutions appendix grouped by unit, including the three solution-only
  declarations that are not on the canvas (`mul_zero`, `mul_succ`,
  `demo_leRep_zero_le`).
- Source fingerprints in the footer; no dates or absolute host paths, so the
  generated page is deterministic for a given source tree.

## Interaction

- **并读 / 直觉 / 形式** toolbar switch (`body[data-view]`):
  - 直觉 keeps prose, hints, and target statements; hides proof/definition code.
  - 形式 keeps declarations, proofs, and exercise statements; hides prose/hints.
  - 并读 shows both layers.
- Native `<details>` for proofs, hints, and solutions. `展开全部` /
  `折叠全部` operate on every details element; `打印 / PDF` calls `window.print()`.
- TOC links are plain fragment links, so they work without JavaScript.
- `localStorage` remembers the last layer choice when available (file:// safe;
  failures are swallowed). `beforeprint` opens every collapsed panel, and
  `afterprint` restores the reader's state.

## Math

- Unicode math runs in prose get a serif `<span class="math">`.
- Simple display formulas in comments become a `.math-display` block.
- A tiny stdlib helper converts simple `(a+b)/(c+d)` fractions and Unicode
  superscripts / `x^2` to native MathML (`<mfrac>`, `<msup>`). There is no
  external math library, no CDN, and no web font.
- Course comments are mostly Unicode; the self-test in `--check` keeps the
  fraction/superscript helper from silently rotting.

## Print

`@page` is A4. Cover, TOC, every unit, and the appendix get explicit page
breaks; code and display math switch to wrapping (`white-space: pre-wrap`,
`overflow: visible`) so nothing is clipped. A print pass opens all `<details>`
so collapsed proofs/hints/solutions are included.

## Check contract

`--check` fails on any of:

- not exactly 19 units;
- a unit without title / `title_en` / content blocks;
- an exercise with no solution matched by declaration name;
- a TOC/fragment href that does not resolve, or duplicate `id`s;
- any `http://` / `https://`, `<link>`, or `<script src=...>` in the output;
- missing `@page` / `break-before: page` print rules;
- output that differs from a fresh render (stale build).

## Limits / not in this prototype

- No kernel integration: exercises are presented, not judged.
- Not a full MathML converter; only simple fractions/superscripts are
  structured. No LaTeX input, no automatic MathML for arbitrary Unicode.
- No search, progress tracking, per-exercise navigation, or language toggle.
- The parser is source-shape based (top-level declarations, comment groups,
  `soko:hint`, `sorry`). It is deliberately scoped to the current II.1 files.
- The HTML is a static artifact; rerun the build after course edits.

## Files

- `scripts/build_book_proto.py` — parser, renderer, and `--check`.
- `scripts/book/style.css` — screen + A4 print styles (inlined by the builder).
- `scripts/book/app.js` — layer toggle, details controls, print pass (inlined).
- `dist/book/.gitignore` — keeps generated pages out of git.

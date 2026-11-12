# Interactive analysis book (v2) — M1+M2, 72 units

> **Status:** the C8.1 reading product, prototype v2. One self-contained HTML
> page built from `courses/analysis/` — the cover, volume/chapter TOC, all
> current units, and the solutions appendix. The previous v1 sample
> (II.1 only, 19 units) is superseded by this builder.

The book currently renders **7 chapters / 72 units** from the course manifest:
II.1 (19 units), II.2 (9), II.3 (6), II.4 (10), II.5 (10), II.6 (14),
II.7 (4), plus **343 exercises**, **1016 declarations**, **1179 hints**, and
**343 solutions** (plus 38 solution-only declarations). The in-page search
index has **1365 entries** (7 chapters, 72 units, 1016 declarations/exercises,
160 prose headings, 72 appendix sections, 38 solution-only declarations). All
counts are read from the source tree; nothing is hardcoded by chapter or unit
count.

## Build / check

From the repository root:

```bash
python3 scripts/build_book_proto.py --out dist/book/book-v2
python3 scripts/build_book_proto.py --check --out dist/book/book-v2
python3 scripts/build_book_proto.py --check --check-preview
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
- **Search box** in the topbar: a static in-page index of chapter/unit titles,
  declaration and exercise names, declaration types (`def` / `theorem` /
  `axiom` / `inductive`), prose headings, appendix sections, and solution-only
  declarations.
- **Chapter headings** (`II.1` … `II.7`) with chapter-level counts.
- **Per-unit sections**: unit title (Chinese + English), local/global unit
  position, declaration/exercise/hint counts, a **标记已读 / 已读** toggle, a
  **复制链接** control, prev/next unit navigation, and a stable `#unit-N`
  anchor.
- **Prose comments** (`-- ...`) become paragraphs, numbered headings, bullet
  lists, and decorative rules. Inline `**bold**` / `__bold__` / `` `code` ``
  are rendered; literal markdown markers never reach the page or the search
  results.
- **Every unit starts with 「本单元讲什么」**: the first meaningful prose
  paragraph is lifted, unchanged, into a lead block. File-header boilerplate
  (靶子 / 依赖 / L3 自带 / 允许的 tactic / 写法要点) and exact heading echoes are
  moved into one collapsed **编写注记** panel per unit — moved, never deleted.
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

- **并读 / 直觉 / 形式** layer switch (`body[data-view]`). The page opens in
  **并读** (not 形式); proof steps and full code panels are **collapsed by
  default**, so the first read is prose and signatures. 直觉 keeps prose,
  hints, formulas, and target statements; 形式 keeps declarations, proofs,
  exercise code, and solutions, hiding prose/hints; 并读 shows both.
- **搜索** filters the embedded index by case-insensitive substring. Results
  show a kind badge, title, and unit context. ArrowUp/ArrowDown move the active
  option (combobox + listbox + `aria-activedescendant`), Enter or a click opens
  the target and updates the hash, Escape closes. The status line is
  `role="status" aria-live="polite"` and reports the full hit count; at most
  10 options are rendered, with an explicit 「仅显示前 10 条」 note when
  truncated. There is an empty state and no network request of any kind.
- **阅读进度**: every unit header has a `标记已读 / 已读` toggle
  (`aria-pressed`). The topbar shows `已读 x / 72` (total derived from the DOM)
  plus an `aria progressbar`; the TOC has the same count, a bar, and a
  **重置进度** control that first expands an inline confirmation
  (`确认重置` / `取消`) — one click can never clear progress. Marking is
  explicit; time-on-page and scrolling are not used. State lives in
  `localStorage["soko-book-progress-v1"]` as a sorted array of unit numbers and
  survives reload. If storage is unavailable the control still works for the
  session and the settings area says so.
- **章节** select jumps to any chapter; plain links still work without JS.
  Every same-page link (TOC, prev/next, appendix, chapter links) is routed
  through `history.pushState` + focus so no reload happens; `hashchange` and
  `popstate` restore the target on back/forward.
- **展开全部 / 折叠全部** operate on every `<details>` (proofs, hints,
  solutions, appendix entries).
- **打印 / PDF** calls `window.print()`; `beforeprint` opens every panel so
  the printed book includes collapsed proofs, hints, and solutions, and
  `afterprint` restores the reader's screen state. Search, progress, unit
  actions, and navigation are `no-print`.
- **localStorage** remembers the layer choice (`soko-book-v2-view`) and the
  progress set; failures under `file://` or private mode are swallowed, and
  the controls still work for the session.
- **Mobile/desktop** tested at 390 px and 1440 px: no horizontal overflow,
  including an open search panel. At ≤620 px the chapter select is hidden
  (the TOC remains), the search box takes its own row, the toolbar wraps, and
  prev/next navigation stacks.

## Readability pass (2026-11-12)

The owner's directive for C8.1 is **「可读性排在第一位」**. This pass is
presentation-only: no course file is edited and no feature is added.

- **Typography**: body 18.5 px (18 px on narrow screens), line-height 1.7,
  68ch measure for prose, paragraphs spaced, code 0.78 em (~14.4 px) with a distinct
  paper panel, and a clear h2 → h3 hierarchy. The sticky topbar keeps the
  layer switch, search, progress, and chapter jump in reach; print stays A4.
- **Default**: `body[data-view="both"]`, proofs/code collapsed. A lead block
  per unit carries the author's opening paragraph; the formal cards are
  visually secondary to the prose.
- **No parser junk**: inline `**` / `__` are rendered as `<strong>` in prose,
  headings, and hint labels; the search index stores plain text (no `**`,
  `__`, or backticks); `soko:hint` itself appears only in the cover reading
  guide; separator lines are `<hr>` or stripped, never prose.
- **Engineering notes**: 26 units have course-engineering paragraphs; they
  live in a collapsed `编写注记` panel at the end of the unit (and are hidden in
  print, which is for reading).
- **Checks**: `--check` now also fails on visible `**`/`__`, separator-only
  prose lines, adjacent duplicate prose paragraphs, a missing unit intro, a
  proof/code panel open by default, and prose paragraphs over 1200 characters
  (legitimate formula-bearing paragraphs are exempt). `--selftest` proves each
  new check bites. `capture.mjs` additionally asserts 18–19.5 px body type,
  ~1.7 line-height, a 64–76ch measure, smaller code, a sticky chapter/search
  bar, no visible junk, and no adjacent duplicates at 1440 px and 390 px.

## Deep links / per-unit URLs

Stable anchors: `#top`, `#toc`, `#chapter-ii-N`, `#unit-N`, `#appendix-unit-N`,
`#appendix-solutions`, plus declaration/exercise/appendix-entry ids used by
search results. On load a hash scrolls below the sticky toolbar and focuses the
target. Each unit has **复制链接**, which copies the absolute URL with the
unit's hash; when `navigator.clipboard` is unavailable or denied (headless,
`file://`, some browsers) it falls back to a temporary textarea, and if that
also fails it shows a selected read-only field with the URL — it never fails
silently. Search, TOC, chapter select, and prev/next all use
`pushState`/`replaceState`; back/forward works through `popstate`/`hashchange`.

## Book CI

`.github/workflows/book.yml` runs on `pull_request`/`push` touching
`scripts/book/**`, `scripts/build_book_proto.py`, `courses/analysis/**`,
`scripts/notation-lint.py`, or the workflow itself, plus `workflow_dispatch`.
It uses Python 3 only (no browser/PDF/npm) and runs exactly the local
equivalent:

```bash
python3 scripts/build_book_proto.py --out dist/book/book-v2
python3 scripts/build_book_proto.py --check --out dist/book/book-v2
python3 scripts/build_book_proto.py --check --check-preview
python3 scripts/build_book_proto.py --selftest
python3 scripts/notation-lint.py --root courses/analysis
```

The generated `index.html` is uploaded as the `book-v2-html` artifact.
Observed on the PR that added it (#35): push run 43133044100 and pull_request
run 43133052960 both succeeded, reporting 72 units / 1365 search entries and
uploading a 301,534-byte artifact. `workflow_dispatch` remains for manual runs.
Fork PRs may evaluate workflows with different base-branch approval rules, so
the local commands above remain the source of truth.

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
- a missing search input/placeholder/combobox/listbox/live region, a missing
  embedded search index, or an index that does not equal the live book model
  (including anchors that do not resolve);
- missing per-unit progress toggles (one per unit), missing topbar/TOC counts,
  progress bars, reset controls, the `soko-book-progress-v1` key, or the
  pushState/hashchange wiring;
- missing copy-link controls (one per unit) or the visible copy status;
- output that differs from a fresh render (stale build);
- formula fallbacks that drop their text, or MathML helper regressions;
- a visible literal `**` / `__` in unit text, or markdown markers left in the
  search index display;
- a separator-only prose line (`^[=\-─]{10,}$`) in a unit;
- adjacent duplicate prose paragraphs (after whitespace/markdown
  normalisation), or a prose paragraph over 1200 characters without formula
  symbols;
- a unit without a 「本单元讲什么」 intro element, a proof/code panel open by
  default, or a default layer other than 并读.

`--selftest` additionally proves the checker bites: injected external URL,
broken anchor, duplicate `id`, removed search index, removed copy-link control,
removed unit intro, visible `**`, separator-only line, duplicate prose,
over-long prose, and stale output must each be rejected.

## Evidence

Headless Chrome (CDP) checks at 1440×1000 and 390×844 assert:

- exactly 72 unit sections, 7 chapter sections, 343 exercise cards;
- 72 TOC unit links and 7 TOC chapter links, all anchors resolve;
- no duplicate `id`s, no empty unit, `documentElement.scrollWidth <= innerWidth`
  at 1440 px and 390 px (also with the mobile search panel open);
- 并读/直觉/形式 hide/show the right layers;
- 展开全部 opens all 2597 `<details>`, 折叠全部 closes them;
- search: 「极限」 returns 1 hit and declares its unit; ArrowDown+Enter and a
  click both land on the result target (#ex-68-6, inside unit 68) and close the
  panel; ArrowDown/ArrowDown/ArrowUp moves option 0 → 1 → 0; Escape closes and
  clears; a declaration name returns that declaration first; `theorem` and
  `THEOREM` both return 415 hits capped at 10 options with the 更多 note;
  「定理」 returns 421; a nonsense query shows the empty state and a 0 count;
- progress: marking units 1 and 2 gives `已读 2 / 72` (topbar and TOC),
  `aria-valuenow="2"`, `aria-pressed="true"`, and
  `["1","2"]` in `soko-book-progress-v1`; reload keeps 2; unmark/re-mark goes
  1 → 2; reset shows a confirmation first (still 2), then clears to 0 and
  survives reload;
- URLs: loading `#unit-42` scrolls it below the toolbar, focuses it, syncs the
  chapter select; next updates to `#unit-43`, history back returns to
  `#unit-42`; a TOC link updates to `#unit-5` and direct `location.hash =
  "#unit-7"` (hashchange) scrolls/focuses; copy link reports success or shows
  the selected URL fallback; `#chapter-ii-7` and `#appendix-unit-42` land and
  focus correctly;
- the chapter select changes the hash; localStorage restores the layer after
  reload; print media hides the toolbar and applies page breaks;
- **readability**: every unit has a visible 「本单元讲什么」 intro; body type is
  18.5 px with a ~1.7 line-height and a 68ch measure; code is smaller than body
  text; proofs/code start collapsed; no visible `**` / `__` or separator-only
  line; no adjacent duplicate prose paragraph; the search index has no
  markdown markers; the toolbar is sticky;
- zero `http(s)` requests, zero uncaught exceptions, zero console errors.

Representative screenshots are committed in `scripts/book/screenshots/`
(`cover.png`, `toc.png`, `search.png`, `progress.png`, `unit-url.png`,
`formal.png`, `unit-ii7.png`, `mobile.png`, plus the readability samples
`readable-unit-ii1.png`, `readable-unit-ii4.png`, `readable-mobile.png`).
They are regenerated with `node scripts/book/capture.mjs` after a build; the
script prints the DOM assertion report and writes the original
`/tmp/book-v2-*.png` files plus `/tmp/book-readable-unit-ii1.png`,
`/tmp/book-readable-unit-ii4.png`, `/tmp/book-readable-unit-ii7.png`, and
`/tmp/book-readable-mobile.png`. It also writes `/tmp/book-v2-print.pdf`
(1022 A4 pages with every panel open).

## Limits / known gaps

- Search is literal case-insensitive substring matching: no fuzzy matching, no
  pinyin, no ranking, and prose paragraphs are not indexed (only declaration
  names, titles, and prose headings). The index is embedded, so a book with
  very many more units grows the page rather than fetching at runtime.
- Reading progress is local-only: no accounts, no cross-device sync, and
  clearing browser storage clears it. There is no export/import yet.
- Copy-link uses the clipboard API when available; the `file://` fallback shows
  a selected read-only field instead of copying automatically on some engines.
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
- `.github/workflows/book.yml` — fast Python-only book CI (build/check/
  preview/check/selftest/notation-lint + HTML artifact).
- `dist/book/.gitignore` — keeps generated pages out of git.

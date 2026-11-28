# dsh-inline-image

Generic image fences for DSH Web: one image, or a swipeable deck of pages.

## One image

```markdown
```dsh-image
{"path":"cb-step1-v2.png","alt":"f 和 g 的映射"}
```
```

## Image deck (swipe left / right)

```markdown
```dsh-images
{"title":"连续统假设","pages":[
  {"path":"ch-01.png","caption":"封面：无穷之间，还有没有第三档？"},
  {"path":"ch-02.png","darkPath":"ch-02-d.png","caption":"一样多 = 能一一配对"},
  {"path":"ch-03.png","caption":"对角线：D 不在名单里"}]}
```
```

- one page per slide, native horizontal scroll with scroll-snap (touch, trackpad,
  mouse wheel over the strip);
- dots, page counter, and `‹` / `›` buttons; `ArrowLeft` / `ArrowRight` plus
  `Home` / `End` / `PageUp` / `PageDown` when the deck has focus;
- the caption under the deck follows the page in view, and a polite
  `aria-live` region announces the same page/caption for screen readers;
- click a page (or its `⤢` button) to open the full-screen lightbox: pinch or
  wheel to zoom (100–400%), drag to pan while zoomed, double-click toggles
  250%, `Esc` closes, arrows page, `+` / `-` / `0` zoom. Focus enters the
  close button and returns to the deck when the lightbox closes;
- the deck prefetches only the previous and next page, and the first page is
  the only eager image; later pages stay `loading="lazy"`;
- `ratio` is optional (CSS `aspect-ratio`, default `1147 / 588`), so mixed-size
  pages do not jump;
- malformed JSON, a missing page or a broken image fails visibly, keeps the
  path in the error, and offers a retry that bypasses the browser cache;
- mobile: bottom safe area, ≥24px dot targets, larger touch buttons, and edge
  fades while more pages exist in that direction;
- light/dark themes use the DSH card, border, label and photo-mask tokens; a page
  without a dark plate keeps its white plate and visible border in dark mode
  (unchanged), a page with one swaps to it (below);
- `prefers-reduced-motion` turns off smooth scrolling, fades and zoom
  transitions;
- when the Host reports a newer `client.js` age than the running bundle, the
  deck shows a “刷新页面” hint instead of silently running stale code. (If the
  fence renders as a plain code block, the plugin did not load at all; use the
  Plugins page notice or reload.)

## Dark plates (`darkPath`)

`dsh-images` pages — and the single `dsh-image` fence — accept an optional
`darkPath` beside `path`. The two files must be the **same canvas and same
layout**; only the plate colour differs. Convention `<topic>-lNN.png` (light) /
`<topic>-dNN.png` (dark), e.g. `{"path":"x-l01.png","darkPath":"x-d01.png"}`.

| state | image source |
|---|---|
| light theme, `darkPath` present | `path` (light theme always uses `path`) |
| dark theme, `darkPath` present | `darkPath` |
| dark theme, no `darkPath` (also blank / non-string) | `path` — old behavior unchanged |
| theme switches at runtime | re-picks between them (watches `<body data-ds-dark-theme>` with a MutationObserver and `prefers-color-scheme` with the media query) |
| broken `darkPath` | falls back to `path` (never a broken plate); a broken `path` still shows the failure card and its retry |

Dark is active when `document.body` carries `data-ds-dark-theme` (the DSH theme
setting) **or** the OS dark preference matches. Deck prefetch and the lightbox
make the same choice as the visible slide, and a dark plate gets a dark letterbox
background instead of the white one.

The Host serves only files inside the configured workspace roots, only for
owner-authenticated requests, and only image media types. The Client renders the
images inside the Chat transcript; it does not read local files itself.

The route is intentionally independent of any course/plugin-specific state.
`GET /dsh-inline-image/health?path=…` is the content-free diagnostic; it also
reports `clientAgeMs`, the age of this plugin's Client bundle.

## Tests

The bundled seven-page sample (`public/ch-01.png … ch-07.png`, one 1147×588
canvas) is the regression fixture:

```bash
cd dsh/plugins/dsh-inline-image
node test/run.mjs   # host route + client deck contract; 0 = all pass
```

`test/host.test.mjs` covers the image route (7 pages, retry query, HEAD, 405,
traversal 404, 415, 413, health age). `test/client.test.mjs` renders the real
`DeckFence` / `ImageFence` through a dependency-free hook runtime and covers the
7 pages, dots, counter, caption/live region, eager first page, keyboard paging,
±1 prefetch, failure/retry, lightbox zoom/focus, the HMR hint, malformed input,
and the `darkPath` five states (light/dark/no-darkPath/runtime switch/broken
fallback) plus theme-aware prefetch and lightbox. The two `s7-light.png` /
`s7-dark.png` fixtures are the same 1147×588 canvas with a white / `#141414`
plate pair. CI runs the same command in the `dsh-inline-image-tests` job.

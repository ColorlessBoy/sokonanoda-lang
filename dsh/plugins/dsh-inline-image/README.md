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
  {"path":"ch-02.png","caption":"一样多 = 能一一配对"},
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
- light/dark themes use the DSH card, border, label and photo-mask tokens; the
  image plate stays white and keeps a visible border in dark mode;
- `prefers-reduced-motion` turns off smooth scrolling, fades and zoom
  transitions;
- when the Host reports a newer `client.js` age than the running bundle, the
  deck shows a “刷新页面” hint instead of silently running stale code. (If the
  fence renders as a plain code block, the plugin did not load at all; use the
  Plugins page notice or reload.)

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
`DeckFence` through a dependency-free hook runtime and covers the 7 pages, dots,
counter, caption/live region, eager first page, keyboard paging, ±1 prefetch,
failure/retry, lightbox zoom/focus, the HMR hint and malformed input. CI runs
the same command in the `dsh-inline-image-tests` job.

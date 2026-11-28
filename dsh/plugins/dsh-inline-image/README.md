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
- dots, page counter, and `‹` / `›` buttons; `ArrowLeft` / `ArrowRight` when the
  deck has focus;
- the caption under the deck follows the page in view;
- `ratio` is optional (CSS `aspect-ratio`, default `1147 / 588`), so mixed-size
  pages do not jump;
- malformed JSON or a missing page fails visibly instead of crashing the chat.

The Host serves only files inside the configured workspace roots, only for
owner-authenticated requests, and only image media types. The Client renders the
images inside the Chat transcript; it does not read local files itself.

The route is intentionally independent of any course/plugin-specific state.

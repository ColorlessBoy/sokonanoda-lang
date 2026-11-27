# dsh-inline-image

Generic inline image fence for DSH Web.

```markdown
```dsh-image
{"path":"cb-step1-v2.png","alt":"f 和 g 的映射"}
```
```

The Host serves only files inside the configured workspace roots, only for
owner-authenticated requests, and only image media types. The Client renders the
image inside the Chat transcript; it does not read local files itself.

The route is intentionally independent of any course/plugin-specific state.

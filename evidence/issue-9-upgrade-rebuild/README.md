# issue #9 — extension upgrade ⇒ rebuild suggestion (before / after)

Same scenario in both captures: a copy of `courses/quick-start/unit11-project` opened in
VS Code 1.145.0 with `Exercises.sokonanoda` as the active editor, isolated profile and
extensions directory, Default Dark Modern, window 1400×900, off-screen window captured with
`screencapture -l <window-id>` (no other system permissions).

- **before** = extension code at `28d2f84a` (released 0.98.3). Opening the project after the
  (simulated) upgrade shows **no rebuild suggestion**.
- **after** = ticket branch `agent/upgrade-rebuild-prompt-02b104497a5a`. Same scenario,
  **a notification suggests rebuilding the project** with **确认 / 取消**.

The upgrade is simulated identically in both profiles by seeding the extension's global
state baseline with `extensionVersion = 0.98.2`; the before code does not read that key, the
after code does. This is the same state the extension writes on a real upgrade. Both captures
use the same LSP/CLI binary build (only the extension JavaScript differs).

OCR readout of `after-branch-rebuild-prompt.png` (macOS Vision, zh-Hans first):

```
sokonanoda 已升级到v0.98.3（之前 v0.98.2）。建议 rebuild
项目「work」，让编译缓存与新版本对齐。
Source: sokonanoda language support
确认
取消
```

`after-branch-rebuild-prompt-buttons.png` is a 3× crop of that notification.

Note: the before/after images intentionally have no other VS Code windows, user settings or
private material; the workspace is the repository's own teaching fixture copied to a temp dir.

---
description: 强制把缓存的 CLI + LSP 刷新到当前版本（二进制内嵌下载器；幂等）
agent: build
---

从**任意目录**运行：

```bash
sokonanoda update
sokonanoda version --json
```

`update` 按 `sokonanoda` 自身的版本强制重下 CLI + LSP；随后用
`version --json` 确认 `cli.match` 与 `lsp.match` 都变成 `true`，向用户汇报
`version` / `target` / `cache`。

纪律：下载 URL 必须锁定 `v${version}`，**禁用 `releases/latest`**；不要要求
用户安装 Rust/cargo（用户/agent 路径零工具链依赖）。

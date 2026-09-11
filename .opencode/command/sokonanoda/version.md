---
description: 打印版本、平台与缓存里实际安装的 CLI/LSP 版本（--json 机器可读；零 cargo）
agent: build
---

从**任意目录**运行：

```bash
sokonanoda version --json
```

汇报 `version`（本二进制版本）、`target`、以及 `cli` / `lsp` 的 `present`、
`marker`（缓存里实际的 `<version> <target>`）与 `match`。

- `match` 全为 `true` = 缓存与版本一致，无需更新；
- 任一 `match` 为 `false`（或 `present` 为 `false`）→ 下一步跑
  `/sokonanoda/update`。

命令**不修改**任何东西（只读）；不要用 `releases/latest`，不要要求用户安装
Rust/cargo。

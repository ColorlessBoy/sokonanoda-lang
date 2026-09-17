---
description: 打印版本、平台与缓存里实际安装的 CLI/LSP 版本（--json 机器可读；零 cargo）
agent: build
---

从**任意目录**运行：

```bash
scripts/soko version --json
```

汇报 `version`（仓库版本）、`target`、以及 `cli` / `lsp` 的 `path`、`source`
（解析来源：`repo-build` / `cache` / `download` / `cache(STALE…)`）与 `marker`。

- `source` 不含 `STALE` 且有 `path` = 就绪，无需更新；
- 任一 `source` 为 `cache(STALE…)` 或 `path` 缺失 → 下一步跑
  `/sokonanoda/update`。

命令**不修改**任何东西（只读）；不要用 `releases/latest`，不要要求用户安装
Rust/cargo。

---
description: 环境就绪诊断（只读；--json 机器可读；退出码 0=就绪 3=未就绪）
agent: build
---

从**任意目录**运行：

```bash
ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
bash "$ROOT/scripts/soko.sh" doctor --json; echo "EXIT=$?"
```

汇报字段：`ready` / `version` / `target` / `cache` / `cli` / `lsp` /
`launcher` / `plugin` / `cargo`。退出码 `0` = 就绪；`3` = 环境未就绪
（下一步跑 `/sokonanoda/setup`）。

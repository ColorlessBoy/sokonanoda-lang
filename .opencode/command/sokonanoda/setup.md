---
description: 安装/更新与仓库版本锁定的 CLI + LSP 二进制（零 cargo，幂等）
agent: build
---

从**任意目录**运行单一环境入口（已有且版本匹配会秒过）：

```bash
ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
bash "$ROOT/scripts/soko.sh" setup
bash "$ROOT/scripts/soko.sh" doctor --json
```

向用户汇报 doctor 的 `ready` / `version` / `target` / `cache`；失败时贴出脚本
报错。不要使用 `releases/latest`，不要要求用户安装 Rust/cargo。

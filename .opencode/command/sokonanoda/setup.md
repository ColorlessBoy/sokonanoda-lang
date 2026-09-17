---
description: 安装/更新与仓库版本锁定的 CLI + LSP 二进制（二进制内嵌下载器；幂等）
agent: build
---

从**仓库根目录**运行（`scripts/soko` 是 harness 中立启动器；opencode 插件也
会把缓存目录注入 PATH，此时 `sokonanoda …` 等价）：

```bash
scripts/soko setup
scripts/soko doctor --json
```

向用户汇报 `doctor` 的 `ready` / `version` / `target` / `cache`；失败时贴出
二进制报错。不要使用 `releases/latest`，不要要求用户安装 Rust/cargo。

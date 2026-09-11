---
description: 安装/更新与仓库版本锁定的 CLI + LSP 二进制（二进制内嵌下载器；幂等）
agent: build
---

从**任意目录**运行（`sokonanoda` 由 opencode 插件 provision 到缓存并注入 PATH）：

```bash
sokonanoda setup
sokonanoda doctor --json
```

向用户汇报 `doctor` 的 `ready` / `version` / `target` / `cache`；失败时贴出
二进制报错。不要使用 `releases/latest`，不要要求用户安装 Rust/cargo。

---
description: 强制把缓存的 CLI + LSP 刷新到当前版本（二进制内嵌下载器；幂等）
agent: build
---

从**任意目录**运行：

```bash
scripts/soko update
scripts/soko version --json
```

`update` 按仓库 `Cargo.toml` 的版本强制重下 CLI + LSP。**退出码有语义**：

- `0` = 缓存写成功了；
- **`3` = 缓存没写成**（stderr 打 `cache NOT refreshed` + 每个 `download: <原因>`
  + 这次实际用的回退）。有可用回退**不代表**刷新成功——把失败原因原样汇报，
  不要让用户以为已经修好。

随后用 `version --json` 复核：`cli.source` / `lsp.source` 不含 `cache(STALE…)`
**只说明有能用的二进制**（版本匹配的仓库构建会排在缓存前面），唯一可信的判据是
缓存 `marker` 与缓存二进制自述版本都等于 `Cargo.toml` 版本。向用户汇报
`version` / `target` / `cache` / 两个 `source` / `marker`。

纪律：下载 URL 必须锁定 `v${version}`，**禁用 `releases/latest`**；不要要求
用户安装 Rust/cargo（用户/agent 路径零工具链依赖）。

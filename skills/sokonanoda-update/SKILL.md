---
name: sokonanoda-update
description: Refresh the version-pinned sokonanoda CLI + LSP cache to this checkout's version and prove it with version --json. Use when a stale cache makes the launcher refuse to run (exit 3), when doctor reports ready:false, or when a solver suddenly reports syntax errors a newer checkout should accept.
---

# sokonanoda-update：把缓存刷新到仓库版本

## 何时用

启动器**版本不符就拒绝执行**——这是本项目历史上最常见的故障源。出现下列
任一现象就跑本命令：

- `scripts/soko doctor --json` 报 `ready: false`；
- `scripts/soko version --json` 里 `cli.source` 或 `lsp.source` 含 `STALE`；
- 任何 `scripts/soko <子命令>` 直接 **exit 3** 并提示 "refusing to run a
  stale binary"（注意：`update` 自己的 exit 3 含义不同——那是"缓存没写成"，
  见下）；
- 判卷结果出现"旧编译器看不懂新语法"式的解析错误（文件本身没改过）。

## 命令（从仓库根或任意目录）

```bash
scripts/soko update
scripts/soko version --json
```

`update` 按 `Cargo.toml` 的版本强制重下 CLI + LSP 到缓存（幂等、跨平台、
零 cargo）。**它的契约是"缓存被写成功"，所以：**

- 写成功 → exit `0`，两个 `source` 是 `download(forced)`（或 `download`）；
- **没写成功 → exit `3`**，stderr 打 `cache NOT refreshed` + 每个失败的
  `download: <原因>`，并说明这次实际用的是哪个回退（`repo-build` /
  `override` / `cache`）。回退可用**不代表**刷新成功，两者必须分开读。

### ⚠️ 别把"能跑"当成"刷好了"

解析链是**版本匹配的仓库构建 → 缓存**，所以仓库里做过 `cargo build`/`cargo test`
时，`update` 常常报 `[repo-build]`。`source` 不含 `STALE` **只说明有能用的二进制**，
不说明缓存被刷新。唯一可信的判据是下面那组 `marker` / 缓存二进制自述版本。

## 判据（缺一不可）

1. `update` 的退出码是 `0`（exit `3` = 缓存没写成，先按 stderr 的 `download:`
   那行修：见下）；
2. 缓存标记文件与缓存的二进制**自己**都等于仓库版本：

   ```bash
   cat ~/.local/share/sokonanoda/bin/sokonanoda.version        # <version> <target>
   ~/.local/share/sokonanoda/bin/sokonanoda --version          # sokonanoda <version>
   grep -m1 '^version' Cargo.toml
   ```

   （缓存目录可被 `SOKONANODA_CACHE_DIR` 覆盖，用 `version --json` 的 `cache`
   字段拿真实路径。）
3. `scripts/soko doctor --json` 的 `ready` 为 `true`（退出码 `0`）。

## 常见失败：缓存写不进去（实测踩过）

`download: EPERM: operation not permitted, copyfile … -> …/sokonanoda/bin/sokonanoda`
= **下载与解包都成功了，只卡在最后一步写缓存**。典型原因：缓存目录只读、磁盘满、
或**当前进程被沙箱限制**（例如在 DeepSeek Harness 里以 workspace-write 运行——
`~/.local/share` 在允许范围之外）。处置：

- 让**用户在自己终端**跑一次 `scripts/soko update`（沙箱外）；或
- 把 `SOKONANODA_CACHE_DIR` 指到一个可写目录；或
- 直接接受回退：`[repo-build]` / `[override]` 的二进制语义与缓存版**完全一致**
  （同一个内核、同一个前端），只是没能刷新缓存——判卷、出题照常进行。

不要为了刷新缓存去安装/构建 Rust，也不要改用 `releases/latest`。

## 纪律

- 下载 URL 一律锁定 `v${version}`：**禁用 `releases/latest`**——那会让新
  客户端配上旧服务器。
- 不得要求用户安装 Rust/cargo（`REQUIREMENTS.md` §2 第 9 条）：`update` 只
  消费 Release 资产，"去构建一份"不是替代路径。
- 汇报实测值（退出码、`version` / `target` / `cache`、两个 `source`、以及
  缓存 `marker`），不要只说"已更新"；exit `3` 时必须把 `download:` 那行贴出来。

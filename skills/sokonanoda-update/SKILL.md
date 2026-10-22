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

> **VS Code 的 standalone CLI 不走本命令**：扩展升级时会自动把
> `~/.local/share/sokonanoda/bin/sokonanoda` 换成新 VSIX 自带版本；`update`
> 只刷新启动器缓存，不会把旧 CLI 带到新扩展版本。

## 命令（从仓库根或任意目录）

```bash
scripts/soko update
scripts/soko version --json
```

`update` 按**版本钉**（`SOKONANODA_VERSION` → `sokonanoda-version.txt` →
`sokonanoda.toml` 的 `requires`（完整 `x.y.z`）→ `Cargo.toml`）强制重下
CLI + LSP 到缓存（幂等、跨平台、零 cargo）。**它的契约是"缓存被写成功"，所以：**

- 写成功 → exit `0`，两个 `source` 是 `download(forced)`（或 `download`）；
- **没写成功 → exit `3`**，stderr 打 `cache NOT refreshed` + 每个失败的
  `download: <原因>` **和实际请求的 `url:`**，并说明这次实际用的是哪个回退
  （`repo-build` / `override` / `cache`）。回退可用**不代表**刷新成功，两者必须分开读。

### `HTTP 404` ≠ 网络问题（2026-10-15 实测修好的那个）

`download: fetch failed: HTTP 404` 说明**那个 URL 上没有资产**，不是连不上网
（stderr 现在会打 `url:` 那行 —— 先看它，别去查代理）。两种可能：

1. **URL 本身拼错了**（`SUB-soko-update-cli-404` 的根因，已修）：旧实现把版本
   **对象**插进 URL ⇒ `…/download/v[object Object]/…` ⇒ curl 把 `[` 当 glob、
   Node fetch 拿 404。现在 `download()` 只接受 `x.y.z` 字符串、拒绝非版本；
   判据 `crates/cli/tests/launcher.rs` 的 `update_forces_a_refresh_at_the_pinned_version`
   （钉住"**每次请求都带 `/v<pin>/`、恰好两次**"）。
2. **这个版本真的没有该资产**（还没发版/资产名变了）：按 stderr 的
   `gh release view v<ver> --json assets --jq '.assets[].name'` 对照；真缺就
   `cargo build --release -p sokonanoda-cli --locked` 用仓库构建，别装 Rust 之外
   的东西、也别改 `releases/latest`。

### 扩展目录堆积是**另一件事**：`scripts/soko prune`

`update` 只修**缓存**两半。`~/.vscode/extensions/` 下的多个 sokonanoda 版本是
**VS Code 的簿记**（`code --install-extension` 不删旧版，只写进 `.obsolete`）——
`update` 碰不到它们。一条命令收口：

```bash
scripts/soko prune            # dry-run：列 keep/remove + 悬空 .obsolete + 两半 marker
scripts/soko prune --yes      # 真清：多余扩展目录 + 悬空 .obsolete + 对齐两半 marker
scripts/soko prune --check    # 只判定：有可清的就 exit 3（可当门禁）
```

⚠ 默认 **dry-run**（删除不可逆）；`extensions.json` 说 VS Code **正在用**的那份
**永不删**；只动 `sokonanoda-lang.sokonanoda-*` 目录与它自己的 `.obsolete` 条目，
别的扩展与别的条目一个字节都不动。`prune` **不装新版**：盘上没有与版本钉匹配的
扩展时它会说出来并给 `code --install-extension` 的出路（静默会让人以为"统一了" ✗）。

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

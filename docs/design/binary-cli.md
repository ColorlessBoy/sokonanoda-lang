# 环境能力进二进制：`sokonanoda env`（去掉 `soko.sh`）

> 触发（2026-09-11，用户要求）：`scripts/soko.sh` 是 bash 脚本，macOS/Linux
> 可用但 Windows 不可用、维护面大。用户要求把环境能力做成**二进制 CLI**
> （`sokonanoda` 的子命令），拒绝 `soko.sh`。方案选定「CLI 内嵌下载器」
> （minreq + rustls + flate2 + tar），setup/update 真正自包含、跨平台。

## 1. 命令面（`sokonanoda` 子命令）

| 子命令 | 行为 | 退出码 |
|---|---|---|
| `sokonanoda version [--json]` | 只读：本二进制版本 + host target + 缓存里 CLI/LSP 的标记与是否匹配 | 0 |
| `sokonanoda doctor [--json]` | 只读就绪诊断（version/target/cache/cli/lsp） | 0 就绪 / 3 未就绪 |
| `sokonanoda setup [--force]` | 按本二进制版本下载 CLI + LSP 到缓存（幂等） | 0 / 3 |
| `sokonanoda update` | = `setup --force`：强制刷新到本版本 | 0 / 3 |
| `sokonanoda grade <file...>` | 判卷（等价 `--json`，多个文件） | 0 / 1 |
| `sokonanoda lsp` | 起语言服务器（既有） | 0 |
| `sokonanoda gate` | 贡献者 CI 门禁（shell 到 cargo；既有语义） | 0 / 1 / 3 |

约定：`SOKONANODA_CACHE_DIR`、`SOKONANODA_OFFLINE=1`、
`SOKONANODA_RELEASE_BASE`（测试/自托管覆盖下载基址，默认 GitHub Releases）。
版本严格锁定 `v<本二进制版本>`，**禁用 `latest`**。缓存版本标记与
`VSIX`/插件一致：`<version> <vsce-target>`，文件 `<name>.version`。

## 2. 为什么内嵌下载器能成立（bootstrap 边界）

- 二进制一旦在手，`version/doctor/update/setup/lsp/grade/gate` 全部自包含，
  跨平台（Rust 已按 host target 编好，含 Windows `.exe`）。
- **首次获取**仍由 opencode 插件（Node，跨平台）或 VSIX 自带二进制完成——
  这不是 shell 脚本，且本就不该由 CLI 承担（鸡生蛋）。调研见
  `docs/notes/rust-cross-platform-binary.md`。
- 目标 triple 用 build.rs 在编译期钉死（`cargo:rustc-env=SOKONANODA_TARGET`），
  比运行期 `uname` 猜测可靠；下载资产名 `<pkg>-<TARGET>.tar.gz`。

## 3. 取舍

- **依赖**：`minreq`（`https-rustls`，webpki 根，免系统 TLS）+ `flate2` +
  `tar`。纯 Rust/可交叉编译优先；代价是发布二进制变大、8 平台交叉构建需
  重新验证（release `workflow_dispatch` 干跑）。
- **不做 shell 依赖**：不 shell 到 `curl`/`tar`（Windows 无 `curl`，且要求
  是"二进制程序"）。
- **删除 `scripts/soko.sh`**；`.opencode/command/sokonanoda/*` 改调
  `sokonanoda <sub>`（插件已把缓存目录注入 PATH）；LSP shim 改成
  「解析仓库构建/缓存 → exec `sokonanoda lsp`」的极小包装（非 opencode
  harness 用）。
- `gate` 需要 cargo，属贡献者路径（shell 到 cargo）；用户/agent 路径不碰它。

## 4. 验收标准

1. `sokonanoda version --json` / `doctor --json` 与旧脚本字段兼容（version/
   target/cache/cli/lsp），空缓存 doctor exit 3、标记匹配 exit 0；
2. `sokonanoda setup`（网络）与 `update` 能把版本锁定的 CLI+LSP 落到缓存并写
   标记；离线时报可操作错误、exit 3；
3. 下载只走 `v<version>`，永不 `latest`；
4. `sokonanoda grade` 多文件、`sokonanoda lsp`、`sokonanoda gate` 行为不回归；
5. `scripts/soko.sh` 删除；三层测试（cli 单测 + CLI e2e + 契约测试）全绿；
6. release `workflow_dispatch` 干跑：8 平台 build 通过（验证 rustls/tar 交叉
   编译）。

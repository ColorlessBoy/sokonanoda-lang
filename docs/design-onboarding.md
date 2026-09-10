# 环境配置与入门（设计）

> 日期：2026-09-10。触发：用户反馈「项目没有把如何配置好环境写清楚，让 code
> agent 搞了好久，流程没有理顺；要调研优秀实践」。本文是调研结论 + 单一入口
> 设计（as-built：`scripts/soko.sh` + `/sokonanoda/*` 命令 + 自动 provisioning
> 插件）。

## 1. 问题（as-is）

1. **没有单一入口**：环境配置片段散落在根 README、teacher/dev 两个 skill、
   `.opencode/command/setup.md`、`server.js`/launcher 里，彼此有复制粘贴的
   漂移风险（同一段 curl|tar 出现在 4 处）。
2. **用户/agent 与贡献者路径混在一起**：文档同时出现 Release 二进制与
   `cargo run`/`cargo build`，agent 难以判断该走哪条；历史事故：为使用仓库
   而想装 Rust。
3. **没有诊断入口**：agent 出问题时只能猜（二进制在不在？版本对不对？
   launcher 有没有执行位？），没有一条命令给出机器可读的结论与退出码。
4. **opencode 命令命名太通用**：`/check`、`/setup` 与常见词冲突，且 setup
   需要 agent 主动记得执行。

## 2. 调研结论（优秀实践）

- **两扇门**（uv/ruff/ripgrep/deno 共识）：README 顶部只讲用户安装
  （严格无 cargo）；贡献者在 CONTRIBUTING/AGENTS 的独立小节，prereq 只写
  rustup + 构建命令。
- **单一 bootstrap + doctor**：`mise doctor` / `npm doctor` / `specgit doctor
  --json`（带 `code`/退出码契约，`--json` 是唯一解析面）；小仓库常用
  `just doctor`/`script/bootstrap`。关键是**幂等、只读、退出码语义明确**。
- **AGENTS.md 是地图不是手册**（OpenAI harness engineering）：一屏之内给
  「一条 setup、一条验证、禁止项」，细节链接到 docs；避免巨石 AGENTS.md。
- **opencode 机制（源码确认）**：
  - 命令支持**嵌套目录**：`.opencode/command/sokonanoda/setup.md` →
    `/sokonanoda/setup`（`/ns name` 空格形式不存在）；
  - 插件工厂在启动时运行（失败软着陆、不阻塞会话），`shell.env` 钩子可给
    所有 shell 注入环境变量（PATH）；
  - 自定义 LSP 直接 spawn、无 shell、无预启动钩子——所以仓库 launcher shim
    必须存在，但可以瘦成 3 行。
- **发布物命名**：Rust triple 用于 tarball、vsce target 用于 VSIX；下载一律
  版本锁定；后续可加 `install.sh` + `SHA256SUMS` + GitHub attestations
  （cargo-dist 模式）。

## 3. 设计（to-be，已落地）

### 3.1 单一环境入口 `scripts/soko.sh`

| 命令 | 用途 | 退出码 |
|---|---|---|
| `setup [--force]` | 幂等下载版本锁定的 CLI+LSP 到缓存（`~/.local/share/sokonanoda/bin`）；版本标记 = `<version> <target>` | 0 / 3 |
| `doctor [--json]` | 只读诊断：version/target/cache/cli/lsp/launcher/plugin/cargo | 0 就绪 / 3 未就绪 |
| `grade <file...>` | 缺二进制自动补齐后执行 CLI `--json` | 0 / 3 |
| `gate` | 贡献者 CI 门禁（fmt/clippy/test/playground），需要 cargo | 0 / 1 / 3 |
| `lsp` | 编辑器解析链（env → 仓库构建 → 扩展自带 → 缓存 → 版本锁定下载 → 编译） | 0 / 3 |

约定：`0` 成功；`3` 环境未就绪（**不是任务失败**）；`2` 用法错误；`1`
内部/门禁失败。`--json` 输出是稳定解析面；人类文本可读即可。
可覆盖项：`SOKONANODA_CACHE_DIR`、`SOKONANODA_OFFLINE=1`、
`SOKONANODA_LSP_BIN`。

### 3.2 opencode 层（薄）

- **命令命名空间**：`.opencode/command/sokonanoda/{setup,doctor,check,gate,round}.md`
  → `/sokonanoda/setup`、`/sokonanoda/doctor`、`/sokonanoda/check`、
  `/sokonanoda/gate`、`/sokonanoda/round`；旧的扁平 `/check` `/setup`
  `/gate` `/round` 删除。命令体只调用 `scripts/soko.sh`，不复制逻辑。
- **LSP launcher**：`.opencode/lsp/sokonanoda-lsp.sh` 瘦身为 3 行 shim，
  `exec bash scripts/soko.sh lsp`；解析/下载逻辑只存在于脚本里。
- **插件**：`.opencode/plugin/sokonanoda.ts` 启动时 best-effort 跑
  `soko.sh setup`（幂等，已就绪约毫秒级；失败不阻塞），并用 `shell.env`
  把缓存目录注入 PATH，agent 可直接用 `sokonanoda`。
- `opencode.json` 的 `lsp` 仍指 shim（opencode 无 shell、cwd 可能是子目录）。

### 3.3 文档分工（消除漂移）

- 根 `README.md`：**用户门**（VS Code 扩展 / 手动版本锁定下载），无 cargo；
  贡献者另起一节。
- `AGENTS.md` 顶部新增 `## Setup` 一屏：`scripts/soko.sh setup` + `doctor` +
  禁止项（latest、为使用仓库装 Rust）。
- `skills/sokonanoda-teacher`：环境节 = `scripts/soko.sh setup` + `doctor`，
  全篇零 cargo。
- `skills/sokonanoda-dev`：贡献者路径 = rustup + `cargo build` +
  `scripts/soko.sh gate`。
- 唯一保留的手工下载片段在根 README（给仓库之外的用户）；其余全部引用脚本。

## 4. 测试

- `scripts/soko.sh doctor --json`（exit 3 → 伪造缓存后 exit 0）、`setup`
  离线报错、`lsp` 无 cargo 命中扩展自带 bin、fake-curl 版本锁定下载
  （见 `crates/cli/tests/opencode.rs`，unix）。
- 契约：命令必须嵌套命名且 cargo-free；launcher 只含 shim（引用
  `scripts/soko.sh`）；插件文件存在且只引用脚本。

## 5. 后续（未做）

- `install.sh`（repo 根 + Release 附件，`curl raw.../vX.Y.Z/install.sh | sh`）+
  `SHA256SUMS` + `actions/attest@v4`/immutable releases（cargo-dist 模式）。
- `.devcontainer/devcontainer.json`（Rust 镜像 + `postCreateCommand: scripts/soko.sh gate`）。
- `rust-toolchain.toml` 钉工具链（当前跟随 stable）。
- `cargo binstall` / `mise github:` 作为包管理器备选写进 README。

# AGENTS.md — sokonanoda-lang

给任何 code agent 的项目入口（DeepSeek Harness / opencode / Claude Code 等原生
读取本文件）。按序读完再动手：

1. `REQUIREMENTS.md` —— 用户全部要求的**权威总账**（硬规则、新要求追加到 §9）；
2. `docs/HANDOVER.md` —— **交接汇总**（现在在哪、还剩什么、怎么继续）；
3. `STATUS.md` —— 当前进度（最新一轮在最上）；
4. `ROADMAP.md` §10 —— 待办与验收标准；
5. `docs/architecture.md` —— 流水线与内核 gotchas（§8 必读）。

文档已分层：入口/权威在仓库根（`README.md`/`AGENTS.md`/`ROADMAP.md`/
`REQUIREMENTS.md`/`STATUS.md`），开发者参考在 `docs/` 顶层，设计与调研笔记在
`docs/design/`、`docs/notes/`；完整地图见 **`docs/README.md`**。对外官网在
`site/`（数据由 `scripts/gen-site-data.py` 生成，永不手写版本号）。
harness 适配（各 harness 能用什么、缺什么）见 **`docs/design/deepseek-harness.md`**。

## Setup（用户/agent 零 cargo；一条命令，任何 harness 都能用）

```bash
scripts/soko setup                        # 版本锁定的 CLI + LSP → 缓存（幂等）
scripts/soko doctor --json                # 就绪诊断；0=就绪 3=未就绪
scripts/soko grade playground.sokonanoda  # 判卷（--json 事件流）
scripts/soko grade course/unit11-project/Exercises.sokonanoda  # 多文件项目（import 闭包）
scripts/soko grade --root <模块根> <入口.sokonanoda>  # 显式模块根（默认：最近 sokonanoda.toml，否则入口目录）
scripts/soko grade --no-project <文件>    # 忽略 sokonanoda.toml，模块根 = 入口目录
scripts/soko query check --file playground.sokonanoda   # 同一判卷的单 JSON 摘要
scripts/soko query state --file playground.sokonanoda --line 327 --col 4
scripts/soko version --json               # 仓库版本 + 解析来源 + 缓存标记
scripts/soko update                       # 强制刷新到仓库版本
```

- **判卷有两个视图，同一份真相**：`grade --json` = 全量事件流（既有消费者不变），
  `query <op>` = 计数/目标/洞的**单 JSON 对象**（`check`/`state`/`goals`/`holes`/
  `hints`/`reduce`）。要问"某处还差什么"就用 `query state`，别自己扫事件流。
  契约见 `docs/protocol.md`；计数一致性由 `crates/cli/tests/query.rs` 钉死。
  `ok:false` **不是**空结果；退出码 0=答上了 / 1=有内核拒绝 / 2=用法错误。
- **DeepSeek Harness** 里那六个查询还包成 MCP 工具
  （`mcp__sokonanoda__{check,state,goals,holes,hints,reduce}`，需
  `dsh web --patch ./dsh/cordis.patch.yml`）：有工具就直接调，别绕 shell。

- **`scripts/soko` 是 harness 中立的启动器**（零依赖 Node，跨平台、无 bash）：
  解析顺序 = `$SOKONANODA_BIN` → 版本**匹配**的仓库构建 → 缓存（标记
  `<version> <target>` 必须等于 `Cargo.toml` 版本）→ VS Code 扩展自带 →
  按仓库版本锁定下载；其余子命令原样转发给 `sokonanoda` CLI。
  缓存**过期就拒绝运行并提示**（它是历史上最常见的故障源）。
- 已经有 `sokonanoda` 在 PATH 上时，上表的 `scripts/soko …` 可换成
  `sokonanoda …`（等价）；DSH 里没有项目级 PATH 注入，所以文档一律先给启动器形式。
- 环境能力本身是 `sokonanoda` 二进制的子命令（内嵌下载器，跨平台；旧的
  `scripts/soko.sh` 已删除）。
- opencode 额外有：`/sokonanoda/setup` `/sokonanoda/update`
  `/sokonanoda/version` `/sokonanoda/doctor` `/sokonanoda/check`
  `/sokonanoda/gate`，以及启动插件自动 provision。
- DeepSeek Harness 额外有：技能目录自动发现（`.agents/skills/`），技能名即
  `/sokonanoda-teacher` 等命令；编辑器 LSP 需显式
  `dsh web --patch ./dsh/cordis.patch.yml`（详见 `dsh/README.md`）。
- 贡献者（需要 Rust）：`scripts/soko gate`（= fmt + clippy + test + playground
  锚点）或 `cargo build/test`（见 `skills/sokonanoda-dev`）。
- 网络受限时设代理（`HTTPS_PROXY=http://127.0.0.1:7890` 之类），启动器会把它
  交给 `curl` 下载。
- 禁止：`releases/latest`、为使用仓库安装 Rust/cargo（REQUIREMENTS §2 第 9 条）。

## 角色技能（Agent Skills）

- **当老师（产品主循环）**：加载 `sokonanoda-teacher`——画布
  `playground.sokonanoda` 出题/判卷/决策的完整操作手册（正文
  `skills/sokonanoda-teacher/SKILL.md`）。DeepSeek Harness 里直接输入
  `/sokonanoda-teacher`。
- **做开发**：加载 `sokonanoda-dev`——冻结内核、TDD 三层、文档先行。
- **推代码/发布/查 CI**：加载 `sokonanoda-ci`——本地验证纪律
  （退出码、无 grep 掩膜）、workflow 陷阱、`gh` 排错三板斧、失败必录。

> 技能在 DeepSeek Harness 下经 `.agents/skills/<name>/SKILL.md` 被自动发现
> （薄入口，正文仍以 `skills/<name>/SKILL.md` 为唯一源）；`crates/cli/tests/dsh.rs`
> 守住两边不漂移。

## 硬规则速记（全文见 REQUIREMENTS §2/§3）

1. kernel 冻结快照：不改语义、不动热路径；bugfix 带三层回归测试；
2. 不调用官方 Lean 工具链（lean/lake/lean4export/leanc/elan）——opencode 由
   `opencode.json` 权限 deny 强制；**DSH 用 `dsh/hooks/hooks.json` 的
   `PreToolUse` 拦截（需在 profile 插一行启用，见 `dsh/README.md`），未启用时
   退回文档纪律**；
3. 教学语法是真实 Lean 4 的子集；新增语法 = 课程 + 测试 + 白名单三件套；
4. 判定永远走 kernel——**禁止文本比对**（tactic 判定范例：`front::judge`）；
5. 模块化：文件接近 ~500 行即拆分；公开 API 用 re-export 保持稳定；
6. **用户/agent 路径零工具链依赖**：获取与运行只用 Release 二进制或平台
   插件，不把 cargo/Rust 当使用前提（cargo 仅贡献者开发需要；见
   REQUIREMENTS §2 第 9 条）。

## 命令（贡献者：需要 Rust；用户/agent 用 `scripts/soko` / `sokonanoda` 子命令）

```bash
scripts/soko gate   # = CI 门禁：fmt + clippy + test + playground 锚点
# 注意：gate 的 anchor 用**运行中二进制**的内嵌编译器；若它与仓库版本不一致
# （旧缓存/旧构建），gate 会直接 exit 3 —— 先 `scripts/soko update`，或用
# `cargo run -q -p sokonanoda-cli --bin sokonanoda -- --json playground.sokonanoda`。
# 或手动：
cargo fmt -p sokonanoda-front -p sokonanoda-cli -p sokonanoda-lsp -- --check
# 禁止 `cargo fmt --all`：会重排**冻结内核**（kernel 快照不得改动）。只 fmt 教学 crates，或直接 `scripts/soko gate`。
cargo clippy --workspace --all-targets
cargo test --workspace --locked
cargo run -q -p sokonanoda-cli --bin sokonanoda -- --json playground.sokonanoda
```

编辑器/agent 反馈通道：`.sokonanoda` 文件的 LSP 诊断由完整 kernel 判定。
两种接线：

- **opencode**：启动插件自动接线（解析原生 `sokonanoda-lsp`——仓库构建 /
  VS Code 扩展自带 / 缓存 / 版本锁定下载（`fetch`+`tar`，跨平台、零 bash）
  ——并改写 `lsp.command`；`shell.env` 注入 PATH；`opencode.json` 不含 lsp
  命令）；另有 `skills/` 自动加载、`/sokonanoda/*` 命令、`teacher` 主 agent、
  Lean 工具链命令 deny。
- **DeepSeek Harness**：技能与 `/sokonanoda-*` 命令自动可用
  （`.agents/skills/`），但 LSP 需显式启用
  `dsh web --patch ./dsh/cordis.patch.yml`；且**服务端诊断不会被投递给
  agent**——判卷一律走 CLI `--json`（`dsh/README.md`、
  `docs/design/deepseek-harness.md`）。
- 其他 harness 可用 `.opencode/lsp/sokonanoda-lsp.sh` shim →
  `scripts/soko lsp`。

goal 视图走自定义请求
`soko/goals` / `soko/hints` / `soko/nextHole` / `soko/stateAt` /
`soko/version`（服务器自述 {version,pid}，重启命令用）
（`docs/protocol.md`）——**目前只有 VS Code 扩展与 opencode 消费它们**，
DSH 侧无消费者（属于 `docs/design/deepseek-harness.md` 的 H5 backlog）。
每个 release 仍正常产出各平台
`sokonanoda-lsp-<triple>.tar.gz` 与 `sokonanoda-cli-<triple>.tar.gz`
（各 8 个；CLI tarball 里就是可直接执行的二进制，agent 无需 cargo）与
VSIX（9 个，平台包内嵌 LSP 与 CLI），供自动下载与 headless 手动安装；
**下载一律按仓库版本锁定，禁用 `latest`**。
**发版已全自动**：bump 两处版本 → push main → `ci.yml` 的 auto-tag 自动打
tag 并 dispatch release（见 `docs/RELEASE.md`；手动推 tag 仅应急）。

## VS Code 扩展改动

改 `editor/vscode/` 下的任何文件前，先读 `docs/vscode-dev-guide.md`
（版本纪律 / 测试三层 / 常见坑）。版本号必须随功能改动同步 bump。

## CI 失败记录

每次 CI 红了，在 `docs/CI-FAILURES.md` 追加一条（原因/修复/预防）。
同一类失败不犯第二次。

## 收尾义务

- 落 commit 前更新 `STATUS.md`（只保留最近 3 轮，旧轮归档
  `docs/STATUS-ARCHIVE.md`；网站进度页自动读最新轮标题）；
- 用户新要求追加进 `REQUIREMENTS.md` §9 并注明日期（冲突以该文件为准）；
- 设计先行：新功能先写设计进 `docs/`，再动手；多用 subagent 并行调研。
- **VS Code + skills 同步**：任何用户可见改动（命令/键位/视图/反馈/语法/协议/发布形态）
  必须**同一轮**更新 `editor/vscode/`（README/CHANGELOG/package.json）**与** `skills/`
  三个技能 + 本文 + `docs/vscode-dev-guide.md`；测试见 `crates/cli/tests/skill.rs`。
- **harness 同步（opencode 与 DeepSeek Harness 都算一等公民）**：改 `skills/` 正文后
  确认 `.agents/skills/` 入口仍指向它（`crates/cli/tests/dsh.rs` 会挡住漂移）；
  改环境/判卷命令后用 **`scripts/soko`** 形式（harness 中立），opencode 专属
  （插件/`/sokonanoda/*`/teacher agent）与 DSH 专属（`dsh/cordis.patch.yml`）
  各自在同一轮同步；能力差异以 `docs/design/deepseek-harness.md` 为准。
- **code agent 适配是一等公民**：每个开发计划先问「agent 怎么用/怎么验证」——提供
  `--json` 结构化输出、把能力写进 skills、命令可直接执行、`docs/HANDOVER.md` 同步。
- **skill 写法**：命令用**确切可执行的一条命令**（`scripts/soko version --json`），
  少用 token、少用"你应该考虑…"式散文；skill 是给 agent 执行的操作手册。

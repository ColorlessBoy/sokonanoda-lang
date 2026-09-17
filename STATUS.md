# 当前状态与进度日志（agents 先读这里）

> 快照：2026-09-17（第八十八轮：内核真相查询通道设计 + 两个 TODO 改挂；未 bump 版本）
> 仓库：`sokonanoda-lang`；权威计划 = `ROADMAP.md`；**用户要求总账 = `REQUIREMENTS.md`（先读）**；
> **文档地图 = `docs/README.md`**（入口/权威在仓库根，开发者参考在 `docs/` 顶层，
> 设计在 `docs/design/`，调研笔记在 `docs/notes/`）；
> 架构/内核 = `docs/architecture.md`；协议 = `docs/protocol.md`；测试地图 = `docs/TESTING.md`；
> 经验台账 = `docs/LESSONS.md`；CI 失败台账 = `docs/CI-FAILURES.md`；发布 = `docs/RELEASE.md`；
> agent 入口 = `AGENTS.md` + `skills/`；**harness 适配 = `docs/design/deepseek-harness.md`**；
> **agent 查询通道 = `docs/design/agent-query-channel.md`**（ROADMAP I15）。

## 一句话

`.sokonanoda` = **纯声明式教学文件（无 `#` 命令）+ 完整 sokonanoda 内核 + LSP 反馈通道**。
练习 = 带 `sorry` 洞的 `def name : T` / `theorem name : T` / `example : T` 声明。
CLI/REPL 的 `#check` 等只是调试/自测工具，不是文件格式。

## 本轮进度（2026-09-17，第八十八轮：内核真相查询通道设计 + 两个 TODO 改挂）

> 用户：「H5 backlog 里从 MCP 诊断通道入手……这个你来设计一下开发文档，从根上正确
> 解决。同时看一下前人留下的两个 TODO，需要更新一下」。**本轮只出设计 + 改挂，
> 不动实现、不 bump 版本。**

1. **根因判断（为什么不能直接写 MCP server）**：内核真相今天**只有 LSP 一条出口**，
   而且选择/判定逻辑长在 LSP 适配器内部（`goal_decls`/`state_at`/`next_hole` 在
   `crates/lsp/src/lib.rs`，该文件 **4256 行**、远超 ~500 行红线）。直接写 MCP 会
   要么反向依赖 LSP、要么复制出**第二份真相**（违反"判定永远走 kernel"硬规则）。
2. **设计（`docs/design/agent-query-channel.md`，ROADMAP I15 / H6-A…H6-E）**：
   顺序不可颠倒的三层——① 真相层 `front::query`（`check`/`state`/`goals`/`holes`/
   `hints`/`reduce`，编辑器无关的类型化查询）；② 传输：`sokonanoda query <op>`
   （**单 JSON 对象**、零配置、所有 harness 通用、`--text` 支持未落盘中间态）
   + `scripts/soko mcp` / `dsh/mcp/server.js`（MCP stdio 六工具，只转发 CLI）；
   ③ **同一轮把 LSP 改为调用真相层**（顺带把 4256 行降到 ≤1200）。
3. **关键设计点**：`QueryError`/`QueryAnswer` 把"正常的没有"与"问不出来"分开
   （今天 LSP 用 `goal:null`+默认字段混合表达，agent 无法区分——这正是 agent 侧
   只能整文件扫事件流的根源）；位置在真相层用 offset、适配器转坐标（MCP 表面用
   `line`/`character` 与 DSH `lsp` 工具一致）；`query check` 是 `--json` 事件流的
   **新增摘要视图**，事件流契约**只增不改**；MCP **默认关闭**（DSH 视 MCP server
   为沙箱外可信代码，项目不替用户扩大信任面）。
4. **防两套真相的硬门禁**：契约测试断言 `query state` ≡ `soko/stateAt`、
   `query goals` ≡ `soko/goals`（字段级）、`query check` 计数 ≡ `--json` 事件计数，
   外加 `rg` 断言"LSP 侧不得残留查询实现"。
5. **两个 TODO 改挂**（用户要求）：`docs/HANDOVER.md` §3 E 的
   ①索引递归 `Prop` 的 recursor 自动派生被内核拒（`Le`/`Even` 靠课程手写
   `rec`/`iota`）②`inductive` 参数不吃多名字 binder 组 `(A B : Prop)`，
   从孤立 front 待办**改挂 H6-C**——它们决定查询通道"真相"的完整性与 agent
   （主要作者）写出的合法子集会不会被拒；要求**先有"修复前红"的复现测试**，
   按 TDD 三层 + 课程 golden 同步。ROADMAP I15、HANDOVER §3 表头/§3 E 已同步。
6. **待调研补齐**（设计文档 §9，已派 subagent 取源码证据）：DSH MCP client 的完整
   schema/传输/工具命名/失败语义与路径解析、项目侧可交付性，以及两个 TODO 的
   精确根因（哪一行 IH 形状不对、parser 单名路径清单）。
7. **验收口径 A1–A7**：真相唯一（LSP 无残留实现）、CLI/MCP 可用、CLI≡LSP 字段级
   一致、结构债达标（LSP ≤1200 行、`front::query*` ≤500 行/文件）、两个 TODO 带
   反向测试、全量回归绿且既有契约测试**只增不改**。
8. **本轮产物**：`docs/design/agent-query-channel.md`（新）+ `ROADMAP.md` I15 +
   `docs/design/deepseek-harness.md`（H5 的 B1/B2 指向新设计）+ `docs/HANDOVER.md`
   §3/§3E + `docs/README.md` + `REQUIREMENTS.md` §9（八十八）+ 本文。

## 本轮进度（2026-09-17，第八十七轮：DeepSeek Harness 适配落地 —— H0–H4）

> 用户确认「按 H0 → H1 → H2 → H3 → H4 开始实现」，按
> `docs/design/deepseek-harness.md` 五个阶段全部落地，版本 0.54.0 → **0.55.0**。

1. **H0 技能上架**：`.agents/skills/{sokonanoda-teacher,dev,ci}/SKILL.md` 三个
   **薄入口**（正文唯一源仍是 `skills/<name>/SKILL.md`，入口写明按仓库根解析）；
   新增 `crates/cli/tests/dsh.rs`（6 测试：入口↔正文双向、kebab-case 名、DSH
   frontmatter 白名单、指向正文且路径存在、`dsh/cordis.patch.yml` 形状、
   `scripts/soko` 解析链）。**实测**：DSH 会话里三个技能自动出现，`/sokonanoda-*`
   直接可用。
2. **H1 二进制可达**：新增 **`scripts/soko`**（零依赖 Node、跨平台、可执行位入
   git）——DSH 无 PATH 注入也无项目钩子，项目必须有一个可 commit 的入口。
   解析链 = `$SOKONANODA_BIN` → **版本匹配**（跑 `--version` 校验）的仓库构建 →
   缓存（marker 必须等于 `Cargo.toml` 版本）→ VS Code 扩展自带 → 版本锁定下载；
   **缓存过期直接拒绝运行**；网络受限经 `curl` 走 `HTTPS_PROXY`，失败给可诊断原因。
   `AGENTS.md` Setup 改 harness 中立；三个技能命令统一为 `scripts/soko …`。
   实测：`setup` 把本机缓存 0.16.2/0.20.0 → 0.55.0，`doctor --json` `ready:true`，
   `grade playground.sokonanoda` 出内核事件。
3. **H2 LSP 接线**：`dsh/cordis.patch.yml`（`lsp` + `lsp-stdio` + `tool-lsp`，
   `extensionToLanguage[".sokonanoda"]`，command 指向 `scripts/soko`）+
   `dsh/README.md`。**实测**：以仓库为 workspace 启动 DSH 会话，`lsp` 工具 hover
   `playground.sokonanoda:201:9` 返回内核打印的
   `theorem and_swap : forall (a b : Prop), And a b -> And b a`。
   踩到并记录三条新事实：`!!js` **必须单行**、`baseUrl` 是 profile 目录（不能用
   它推导仓库路径）、`lsp` 工具只在会话 workspace 内解析 `file_path`。
4. **H3 命令与角色**：`sokonanoda-teacher` §0 吸收角色与五条不可违反规则；
   `.opencode/agent/teacher.md` 瘦身为指针，**并修掉它里面违反零 cargo 硬规则的
   `cargo run` 判卷命令**；7 个 opencode 命令统一走 `scripts/soko`（`opencode.rs`
   改为"gate 之外的命令必须 cargo-free + 全部走启动器"）。
5. **H4 治理**：`dsh/hooks/{hooks.json,refuse-lean-toolchain.js}` 实现官方 Lean
   工具链 deny（命令位匹配：拦 `lake build`/`$(lean …)`、放行 `grep lean`，
   12 例实测）；`AGENTS.md` 硬规则第 2 条写明两 harness 的 deny 形态；
   `skills/README.md` 重写为多 harness 安装矩阵；`docs/design/onboarding.md` §6
   DSH 对照表；`site/assets/agent-prompt.js` 安装 prompt 改 `scripts/soko` 并说明
   DSH；VS Code README/CHANGELOG/package.json 同步。
6. **验收**：`cargo test --workspace --locked` 全绿（21 个测试目标，含新增
   `dsh.rs`）；`cargo fmt --check` 绿；`clippy` 仅 kernel 既有 warning；
   `scripts/soko gate` **PASS**；site 生成与卫生检查绿。版本 **0.55.0**。
7. **本机环境坑（非仓库问题）**：Xcode 27 许可未接受时 `xcrun`/`ar` 被系统拦，
   `cargo` 链接必失败；绕过用
   `DEVELOPER_DIR=/Library/Developer/CommandLineTools cargo test …`，根治是
   `sudo xcodebuild -license accept`。已记入 `docs/HANDOVER.md` §5。
8. **待做**：`docs/design/deepseek-harness.md` §5 H5 backlog（Infoview 客户端插件 /
   诊断通道 / 启动钩子 / npm 插件包）；§3 E 的两个 front 缺口仍在。

## 本轮进度（2026-09-17，第八十六轮：DeepSeek Harness 适配——只出计划）

> 用户接手项目：「很多地方还没适配 deepseek harness，先理解项目、分析要适配哪里、
> 列一下计划文档」。**本轮只做调研 + 设计，不动实现、不 bump 版本。**

1. **审计结论**：产品内核（kernel / `.sokonanoda` 前端 / `--json` 事件 / 三个技能）
   与 harness 无关、可直接移植；要适配的是**接线层**——技能发现路径、斜杠命令、
   编辑器 LSP 接线、环境与二进制可达性、工具链 deny、文档与契约测试的单 harness 假设。
   **不需要改任何 Rust 语义代码**（Rust 侧唯一新增是契约测试 `crates/cli/tests/dsh.rs`）。
2. **差距 G1–G10**：技能不能被 DSH 发现（P0）/ 七个 `/sokonanoda/*` 命令不存在 /
   无 teacher 主 agent / `.sokonanoda` 无 LSP 接线 / 二进制不在 PATH 且 DSH 禁止项目
   改 PATH（P0）/ Lean 工具链 deny 无对应物 / 33 处文档与契约测试只认 opencode /
   `AGENTS.md` 的 code-agent 适配原则缺 DSH 条目 / 无项目级 provisioning /
   用户级技能环境噪音。
3. **DSH 侧关键事实（逐条带源码行号，文档 §1.2 共 23 条）**：技能根扫描含
   `<repo>/.dsh/skills`(rank 100) 与 `.agents/skills`(200)；**技能名本身即斜杠命令**
   （`/name` 注入正文，零 profile 配置）；frontmatter 路由词只能写 `description`
   （`whenToUse` 是 camelCase 且只进人类 `/` 选单，**旧 camelCase 的
   `disableModelInvocation` 等会让整条技能被丢弃**）；**LSP 不在任何 shipped bundle**，
   且 DSH 的 LSP 只有 4 项只读操作，**`publishDiagnostics` 被显式丢弃**、`soko/*`
   无消费者；工具调用 PATH 不可由项目配置（唯一例外是 LSP 自己的 `env`）；patch 为
   顶层 YAML 数组（`- id:` 整块替换 config、会丢 `!!js`；空文件会 boot 失败），
   `--patch` 可叠且**无项目级自动发现**；hooks 桥只有一个进程级 `configPath`、
   **不做项目发现**；**符号链接是官方同款做法**（DSH 仓库自用
   `.claude/skills -> ../.agents/skills`，watcher 默认跟随）。
4. **计划 H0–H4（每阶段独立可验收）+ backlog H5**：H0 技能上架（`.agents/skills/`
   放软链或薄网关、正文唯一留在 `skills/`、新增 `crates/cli/tests/dsh.rs` 守卫）→
   H1 二进制可达（新增零依赖 Node 启动器 `scripts/soko`，解析链与 opencode 插件同语义
   + marker 版本守卫；`AGENTS.md` Setup 改 harness 中立）→ H2 LSP 接线
   （项目自带 `dsh/cordis.patch.yml` + `--patch` 用法，显式写清诊断不在通道内）→
   H3 命令与角色并入技能（opencode 命令与 teacher agent 正文移进
   `sokonanoda-teacher`）→ H4 治理（deny 形态、33 处文档去 opencode 单一化、门面同步）。
5. **决策 D-1…D-6 与验收 A1–A6** 已列（技能进 DSH 的方式 / 启动器形态与
   REQUIREMENTS（三十二）删除 `scripts/soko.sh` 的边界 / 是否自动 provisioning /
   deny 形态 / `soko/*` 处置 / 版本号策略）。
6. **实测现状**：`sokonanoda` 不在 PATH；缓存为旧版（marker `0.16.2 darwin-arm64`
   vs 仓库 **0.54.0**），`doctor --json` 报 `ready:false`——历史「版本漂移致环境未就绪」
   的故障模式当前正在发生，H1 的 marker 守卫正针对它。
7. **验收**：本轮产物 = `docs/design/deepseek-harness.md` + `REQUIREMENTS.md` §9（八十六）
   + `docs/HANDOVER.md` §3 F/§5/§6 + `docs/README.md` 设计清单 + 本文；不改代码，
   不跑 gate（无代码改动）。

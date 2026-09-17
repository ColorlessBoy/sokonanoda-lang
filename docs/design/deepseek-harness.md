# DeepSeek Harness 适配（设计 + 计划）

> 状态：**设计已定稿，实现未开始**（本文件是 `H0–H4` 的执行依据）。
> 触发：用户 2026-09-17「接手这个项目，但很多地方还没适配 deepseek harness，
> 先理解项目、分析要适配哪里、列计划文档」。
> 权威顺序：`REQUIREMENTS.md`（要求总账）> 本文件（DSH 适配方案）> `AGENTS.md`。
> 关联：`docs/design/onboarding.md`（opencode 启动插件 + 二进制子命令，本文是其
> DSH 对应篇）、`docs/design/binary-cli.md`（环境能力在 `sokonanoda` 二进制里）、
> `skills/README.md`（三个技能的安装矩阵）、`AGENTS.md`（收尾义务）。
> **调研底稿**：`docs/notes/dsh-project-assets.md`（第八十六轮两个 subagent 的
> DSH 源码勘察记录，逐条带 `path:line`；本文 §1.2 即其结论的精简版）。

---

## 0. 一句话结论

本项目的**产品内核（kernel + `.sokonanoda` 前端 + `--json` 事件 + 三个 Agent
Skill）与 harness 无关，直接可用**；需要适配的是**接线层**：技能发现路径、
斜杠命令、编辑器 LSP 接线、环境/二进制可达性、工具链 deny、以及若干只提
opencode 的文档与契约测试。

**本仓库不需要为了 DSH 改一行 Rust 语义代码**，唯一可能新增的 Rust 侧工作是
`crates/cli/tests/dsh.rs` 契约测试；另有一个 Node 启动器（非 cargo、非 Rust）。

---

## 1. 事实基线（本文件的依据）

### 1.1 本仓库里"agent 接线层"的全部资产（审计结果）

| 资产 | 位置 | 作用 |
|---|---|---|
| agent 入口 | `AGENTS.md` | 阅读顺序、硬规则速记、命令、收尾义务 |
| 三个技能 | `skills/sokonanoda-{teacher,dev,ci}/SKILL.md` | Agent Skill（frontmatter `name`+`description`；teacher 另有 `references/`） |
| 启动插件 | `.opencode/plugins/sokonanoda.ts`（301 行） | provision CLI/LSP（env → 仓库构建 → VS Code 扩展自带 → 缓存 → 版本锁定下载）、改写 `lsp.sokonanoda.command`、`shell.env` 注入 PATH |
| 非 opencode shim | `.opencode/lsp/sokonanoda-lsp.sh` | → `sokonanoda lsp` |
| 斜杠命令 | `.opencode/command/sokonanoda/{setup,update,version,doctor,check,gate,round}.md` | `/sokonanoda/*`，都是薄封装（调二进制子命令） |
| 主 agent | `.opencode/agent/teacher.md` | 「老师」角色（mode: primary，Tab 切换） |
| 项目配置 | `opencode.json` | skills 路径、Lean 工具链 deny、watcher ignore、关 Rust 格式化 |
| 契约测试 | `crates/cli/tests/{skill,opencode,protocol,extension}.rs` | 守护上述文件不漂移 |
| 环境事实 | `~/.local/share/sokonanoda/bin` 缓存 + `Cargo.toml` 版本 0.54.0 | 版本锁定下载的目标 |

实测状态（本次接手时）：`sokonanoda` **不在 PATH**；缓存里是 **0.16.2/0.20.0 旧版**
（`./target/release/sokonanoda doctor --json` → `ready:false`、`version_match:false`），
仓库版本已是 **0.54.0**。即"版本漂移导致环境未就绪"这一历史故障模式当前正在发生。

### 1.2 DSH 侧已核实的事实（逐条带证据）

| # | 事实 | 证据 |
|---|---|---|
| D1 | DSH 自动加载项目指令：`$DSH_HOME/AGENTS.md` + 「项目根到会话 cwd」链上的每个 `AGENTS.md`/`CLAUDE.md`（根 = 最近的含 `.git` 祖先，`projectRootMarkers` 默认 `['.git']`），另有 `AGENTS.local.md`/`CLAUDE.local.md` 叠加；`dsh-base` 以 `maxBytes: 65536` 挂载；注入形式是一条 durable **user 角色**消息（`<system-reminder>` 框架）；**子目录的 `AGENTS.md` 是 touch-driven**（agent 读/写/编辑到该子树后才加载） | `packages/context/agent-instructions/README.md`；`src/config.ts:11-14,41-46`；`src/render.ts:12-14,98`；`tests/agent-instructions.spec.ts:361-367` |
| D2 | 技能发现根（rank 小者胜）：`<root>/.dsh/skills`(100) → `<root>/.agents/skills`(200) → `customSkillDirs`(300) → `<dshHome>/skills`(400) → `<agentsHome>/skills`(500，`$DSH_AGENTS_HOME` 或 `~/.agents`) → bundled(600)；bundle = `<name>/SKILL.md`（或平铺 `<name>.md`），**只扫一层，不支持嵌套 `**/SKILL.md`** | `packages/skill/skill-filesystem/src/index.ts:36-40,250-262,723-733`；`docs/subsystems/skills.md`「Local discovery priority」 |
| D3 | SKILL.md frontmatter：必填 `name`（kebab-case `^[a-z0-9]+(?:-[a-z0-9]+)*$`，须等于目录名）、`description`；可选 **`whenToUse`（camelCase！）**、`metadata`、`disable-model-invocation`、`user-invocable`（后两个 kebab-case）。**旧 camelCase 的 `disableModelInvocation`/`modelInvocable`/`userInvocable` 会抛错并整个丢弃该技能**。另：**模型目录只带 `name`+`description`（截断 500 字符），`whenToUse` 只进人类 `/` 选单** → 路由提示必须写进 `description` | `packages/skill/skill-filesystem/src/index.ts:814-839,995-1010`；`packages/skill/tool-skill/src/index.ts:50-58`；`packages/api/session-controller/src/skill-catalog.ts:83` |
| D4 | 技能包内**相对引用文件可用**：bundle 目录被作为 `resourceBase` 下发，`skill` 工具渲染「Base directory for this skill: …」；**资源子文件变更不算目录变更**（不触发重扫） | `packages/skill/skill-filesystem/src/index.ts:729,745`；`packages/skill/skill/src/index.ts:170-198` |
| D5 | **技能名即斜杠命令**：消息里出现 `/name`（`^|\s` + kebab-case + 词边界）时，`tool-skill` 注入该技能正文；对人可调用（`user-invocable`）的技能在 Web GUI 的 `/` 选单里列出。**项目自带、零 profile 配置**；与宿主命令重名时**命令胜**；子 agent 会话里该来源被禁用 | `packages/skill/tool-skill/src/index.ts:195-203,409`；`packages/client/ui-skill/src/client/index.ts:1-11,141-145`；`packages/client/ui-skill/README.md:12` |
| D6 | LSP 不是任何 shipped bundle 的一部分（base/web/headless/sdk/acp 里都没有 `lsp` 行）：必须由用户 profile patch 或 `--patch` overlay 显式挂载 | `grep -l lsp packages/bundle/*/cordis.patch.yml` 无命中 |
| D7 | `@deepseek-ai/dsh-lsp-stdio` 的 `servers.<id>`：`command`（绝对路径或 PATH 名）、`args`、`extensionToLanguage`（小写点号扩展名 → language id）、`env`、`maxStderrBytes`、`shutdownTimeoutMs` 等 | `docs/config-catalog.md:1558-1598`；`packages/lsp/lsp-stdio/README.md` |
| D8 | **DSH 的 LSP 只做 4 项只读操作**：`goToDefinition`/`findReferences`/`goToImplementation`/`hover`；`tool-lsp` 的 schema 就这 4 个 | `packages/lsp/tool-lsp/README.md`；`packages/lsp/lsp/src/types.ts:17` |
| D9 | **服务端 `publishDiagnostics` 被明确忽略**（"a server→client notification (e.g. diagnostics, logs): ignored by this MVP host"）；`workspace/applyEdit` 被拒绝；自定义请求（如 `soko/*`）不在能力内 | `packages/lsp/lsp-stdio/src/connection.ts:248`；`packages/lsp/lsp-stdio/README.md` |
| D10 | patch 格式：顶层 YAML 数组；非 `insert` 条目**必须**有 `id`（未知 id 跳过、`name` 是断言）；`- id: <row>` 覆写既有行（**整块替换 `config`，不深合并**）；`- insert: [ {id,name,config,…} ]` 追加到根列表，`insert` 带 `id` 则追加进该 group 行的 `config` 数组；支持 `!!js` | `vendor/include/src/index.ts:43-141`；`packages/boot/app-boot/README.md:169`；`apps/cli/config/examples/*/cordis.yml` |
| D11 | `--patch <path>` 可重复，按序叠在 profile 用户层与 `$DSH_HOME/cordis.patch.yml` 之后；**没有**"项目自带 patch 自动发现"机制 | `apps/cli/reference/README.md:9`；`apps/cli/src/args.ts:147,182` |
| D12 | 工具调用的环境里 **PATH 不可被项目配置**：项目/家目录 `.env` 都列在 `BOOTSTRAP_NAMES`（含 `PATH`/`HOME`/代理等，只有启动环境能设）；`shell-env` 只收 `DSH_*` 命名空间，工具 schema 不暴露 `env`。**唯一例外**：LSP 服务器自身的 `servers.<id>.env` 会与"洗净后的父环境"合并，且参与可执行文件解析 | `packages/boot/app-boot/src/index.ts:102-105,185-189`；`packages/shell/shell-env/README.md`；`packages/shell/tool-bash/README.md:82`；`packages/lsp/lsp-stdio/src/index.ts:139-147,347-357` |
| D13 | 拦截扩展点：`agent/created`(await)、`agent/pre-step`、`tools/pre-execute`(waterfall，可 `allow`,`deny`,`ask`)、`ctx.tools.guard()`(仅 deny)、`tools/execute`、`tools/post-execute`、`tools/result`、`agent/turn-stopping`；"桥能做的普通插件都能做" | `.agents/notes/implemented/feature/2026-06-30-interception-extension-points.md:9`；`packages/core/tools/src/index.ts:582-591,686-692` |
| D14 | 人类命令（`/xxx`）由**插件注册**（`CommandDefinition`），不是 markdown 文件 | `docs/subsystems/commands.md` |
| D15 | 子 agent 走 `subagent`/`subagent_fork` 工具（in-process spawn/fork，host 层注册表 + preset 提供工具），`workflow` 做大规模编排 | `packages/preset/agent-presets/presets/standard/agent.cordis.yml`「delegation and workflows」节 |
| D16 | preset 与用户级插件都在 `$DSH_HOME`（`~/.dsh/.agent-presets`、profile `node_modules`）；**项目级没有 agent 定义目录**（`<root>/.dsh/agents` 不存在）。子 agent 侧没有"agent 类型"概念：`subagent` 工具的入参只有 `provider`/`model`/`reasoning_effort`/`run_in_background`，**没有 `agent_type`**；行级可配 `persona`/`toolFilter`/`agentOptions{provider,model,reasoningEffort,maxTokens}`/`maxDepth`/`backgroundMode` | `packages/preset/agent-presets/README.md`、`src/discovery.ts:51,60`、`src/preset.ts:18`；`packages/subagent/subagent/src/types.ts:344-346`；`packages/subagent/tool-subagent/src/index.ts:50-125,389-421` |
| D17 | hooks 桥只读**一个进程级 `configPath`**，加载时解析一次，相对路径按**进程启动目录**解析；**不做** Claude Code 那种项目/用户/插件分层发现与热重载（源码留有 TODO）；`SessionStart` 是 detached 执行（可能错过首个请求） | `packages/hooks/hooks-claude-code/README.md:70,174-182`；`src/index.ts:49` |
| D18 | 项目把桥接进 DSH 的正规路径有三条：用户 profile 里插一条 `configPath` 指向仓库；把仓库发布为 npm bundle（`"dsh": {"bundle": {"patch": "./cordis.patch.yml"}}` + `dsh plugin add`）；在 patch 里插一行**本地 `.mjs` 插件**（相对 `name:` 按 patch 文件所在目录解析） | `docs/user/develop/basic/publish.md:42,80-110`；`apps/cli/config/examples/github-review/cordis.yml:8-9`；`apps/cli/reference/README.md:51` |
| D19 | 除 LSP 外，程序化能力的另一条通道是 **MCP**（`dsh-mcp-client` 已作为 CLI 依赖随附，但默认不启用任何 server；把 MCP 工具暴露成 `mcp__<server>__<tool>`）——是 G4「诊断/`soko/*` 进不了 agent」的备选工程路径 | `docs/user/guide/mcp-memory.md`；`apps/cli/reference/README.md:105` |
| D20 | **symbolic-link 兼容是官方同款做法**：DSH 仓库自己用 `.claude/skills -> ../.agents/skills`，一份技能同时喂 DSH 与 Claude Code；skill watcher 默认 `watchFollowSymlinks: true` | DSH 仓库 `.claude/skills`（实测 symlink）；`packages/skill/skill-filesystem/src/index.ts:87` |
| D21 | **patch 实战坑**：① 空文件/只有注释的 patch 会让 **boot 失败**（要禁用该层写 `[]`）；② 没匹配到任何行的 patch 会带层标签报出来（可自查是否生效）；③ 用户 patch 整块替换 config 时会**丢掉其中的 `!!js` 表达式**；④ 自查命令 `dsh --profile <name> --dump-config` | `packages/boot/app-boot/README.md:55,65`；`packages/boot/cmdline/README.md:135`；`docs/user/develop/basic/publish.md:124` |
| D22 | preset 可以自带技能目录（用 `!!js` 把 preset 自身目录当 `customSkillDirs`，注册进**该 preset 的** registry 层，装到哪都自解析）——若将来要发 DSH preset，这是最便携的封装 | `packages/preset/agent-presets/presets/cordis/agent.cordis.yml:255-268` |
| D23 | 客户端插件的硬约束：缺 bundle 会让**整个 fiber FAIL**（聚合 `AggregateError`，不是静默）；包元数据（含"不是 client 包"的否定结论）缓存**到进程重启**；组合 URL 有 3 KiB 切分上限。一个 npm 包可**同时**声明 `dsh.bundle.patch`（宿主层）与 `dsh.client`（浏览器半）——正是"一个包给宿主 handler + 浏览器 view"的推荐封装 | `docs/subsystems/client-modules.md:77-85`；`packages/util/package-manifest/src/types.ts:69-81`；`docs/user/develop/basic/publish.md:19-27` |
| D24 | **`!!js` 的两条实测事实**（H2 落地时踩到）：① 表达式必须留在**同一行**——折叠成多行的标量会带着换行进求值器，结果不再是字符串（schema 报 `expected string but got [object Object]`）；② `!!js` 的求值上下文里 **`baseUrl` 是 profile/家目录**（`$DSH_HOME/profiles/`），**不是 patch 文件所在目录**，所以不能用它推导仓库路径 | 实测：`dsh --profile headless --patch …`；`vendor/include/src/index.ts:188`（`ctx.baseUrl` 由 include 路径设定）与 `packages/boot/app-boot/src/index.ts:879`（根上下文再覆盖一次） |
| D25 | `lsp` 工具要求**会话 workspace root**（`header.cwd`）内有源文件：`file_path` 相对该 root 解析，绝对路径外的文件读不到。所以用 DSH 时必须把 DSH 的工作区选成/启动于本仓库根 | `packages/lsp/tool-lsp/README.md`（`LSP_WORKSPACE_REQUIRED`）；H2 实测（workspace 指向别处时报 `could not be read`） |

> 未核实项（不阻塞 H0–H3）：DSH 是否计划在 `publishDiagnostics` 上做推送、客户端插件能否注入
> 自定义 LSP 方法。二者都属 H4 backlog，不作为承诺。

---

## 2. 差距清单（按"用户能不能用起来"排序）

> 记为 **G1…G10**；每个差距给出：现状 → DSH 事实 → 后果。

### G1 技能不能被 DSH 发现（P0，阻断教学主循环）
- 现状：技能只在 `<repo>/skills/`，opencode 靠 `opencode.json` 的 `skills.paths` 找到。
- DSH：只扫 `.dsh/skills` / `.agents/skills` / `customSkillDirs`（D2），`skills/` 不在列表。
- 后果：DSH 会话里 `skill` 工具目录为空 → agent 不知道有 `sokonanoda-teacher`，
  「当老师」这条产品主循环直接断掉。

### G2 七个 `/sokonanoda/*` 斜杠命令不存在（P1）
- 现状：`.opencode/command/sokonanoda/*.md`。
- DSH：人类命令必须由插件注册（D14），markdown 命令文件不被读取；但**技能名本身
  就是斜杠命令**（D5）。
- 后果：`/sokonanoda/check`、`/sokonanoda/round` 等习惯动作在 DSH 里无人应答。

### G3 「老师」主 agent 不存在（P1）
- 现状：`.opencode/agent/teacher.md`（primary，Tab 切换）。
- DSH：agent 定义无项目级目录（D16）；teacher 的正文其实**几乎全是**「加载
  `sokonanoda-teacher` 技能并遵守它」。
- 后果：DSH 里没有 teacher 角色可切；需把角色正文并入技能（DSH 官方推荐做法）。

### G4 `.sokonanoda` 在 DSH 里没有 LSP 接线（P0'，能力降级但不阻断）
- 现状：opencode 启动插件在 `config` 钩子里改写 `lsp.sokonanoda.command`。
- DSH：LSP 不在任何 bundle（D6），要靠 profile patch / `--patch` 显式挂 `lsp-stdio`
  + `tool-lsp`；而且**只有 4 个只读操作**（D8），**诊断被忽略**（D9）。
- 后果：
  - 好消息：`hover`（内核给出的类型/目标态文本）能到 agent；`goToDefinition`/
    `findReferences` 在 `.sokonanoda` 上比纯 grep 精确。
  - 坏消息：**内核诊断不会进 agent 上下文，也不会进 Web UI**；`soko/goals`、
    `soko/stateAt`、`soko/nextHole`、`soko/hints`、inlay、code action、semantic
    tokens 在 DSH 侧目前**全部无消费者**。
  - 结论：DSH 里 agent 的判卷反馈**必须走 CLI `--json`（bash）**；这本来就是
    teacher 技能的主路径，所以不是架构问题，只是必须写清"别指望 LSP 诊断"。

### G5 二进制不在 PATH，DSH 又无法注入 PATH（P0，阻断一切）
- 现状：opencode 插件用 `shell.env` 把缓存目录塞进 PATH。
- DSH：项目配置不许改工具调用的 PATH（D12：`PATH` 在 `BOOTSTRAP_NAMES` 里，
  项目/家目录 `.env` 都设不了；`shell-env` 只收 `DSH_*`；`tool-bash` 的 schema
  不暴露 `env`）。
- 后果：DSH 会话里 `sokonanoda` 是 `command not found`（本次接手实测如此），
  连 `sokonanoda setup` 都跑不起来 → 先有鸡还是先有蛋。
- 可用的三条出路：① **显式路径**（`<repo>/scripts/soko …` 或缓存绝对路径），
  这是零配置、可 commit 的路，也是 H1 选的；② 用户把缓存目录加进**启动 dsh 的
  shell** 的 PATH（DSH 从父进程继承 PATH 并只做凭据清洗）；③ 用户跑一次
  `scripts/install.sh`（已存在）后自行保证 PATH。
- **唯一例外**：LSP 服务器自身的 `servers.<id>.env` 会覆盖解析与 spawn 环境
  （D12），所以 LSP 的 `command` 可以用 `env.PATH` 兜底；**工具调用没有这个例外**。
- 附带事实：缓存里现在是旧版（0.16.2/0.20.0 vs 仓库 0.54.0），`doctor` 报未就绪。

### G6 Lean 工具链 deny 在 DSH 里没有对应物（P2，硬规则需要落地）
- 现状：`opencode.json` 的 `permission.bash` deny `lean*`/`lake*`/`elan*`/`leanc*`。
- DSH：**没有**命令黑名单机制——沙箱只管文件效果、审批是会话级
  `ask`/`never`、`tools.restrict({allow,deny})` 过滤的是**工具名**而非命令文本；
  能做命令级拦截的是 ① `tools/pre-execute` waterfall 扩展点（可 `deny`）②
  Claude Code hooks 桥的 `PreToolUse`（payload 带 `tool_input`，可返回
  `permissionDecision: "deny"`）（D13）。
- 限制：桥只有**一个进程级 `configPath`**，**不读项目的 `.claude/settings.json`**
  （D17），所以仓库不能自带；要么用户在 profile 里插一行指向仓库的 hooks 文件，
  要么做 native 插件（D18）。
- 后果：DSH 里 agent 可以执行 `lean`/`lake`（当前机器上没装，属于潜在违规）；
  默认情况下硬规则从"配置强制"退化为"文档自觉"。

### G7 文档与契约测试只认 opencode（P2，会持续误导下一个 agent）
- 现状：全仓 33 个文件提到 `opencode`；`AGENTS.md` 的 Setup 章节、`skills/README.md`
  的安装矩阵、`docs/HANDOVER.md`、`site/` 的安装 prompt、`crates/cli/tests/opencode.rs`
  都假定 opencode 是唯一 agent harness。
- 后果：DSH 里的 agent 读到 `AGENTS.md` 会照着 opencode 路径操作（`.opencode/plugins`、
  `/sokonanoda/check`），然后失败。

### G8 `AGENTS.md` 内嵌 code agent 适配原则没有 DSH 条目（P2）
- 现状：`AGENTS.md`「code agent 适配是一等公民」+ `REQUIREMENTS.md` §9 第（八十二）条
  要求「每个开发计划先问 agent 怎么用/怎么验证」。
- DSH：这条原则当前只覆盖 opencode。
- 后果：后续每轮开发都可能继续只适配 opencode，DSH 侧债务累积。

### G9 项目级自动 provisioning 在 DSH 里没有等价路径（P2）
- 现状：opencode 启动插件在会话启动时自动下载/解析二进制。
- DSH：**项目无法自带任何启动钩子**——patch 只能由用户显式 `--patch` 或写进 profile
  （D11/D17）；hooks 桥的 `configPath` 是进程级且按启动目录解析，不做项目发现；
  `SessionStart` 还是 detached 执行（D17）。
- 后果：DSH 首次使用多一步人工（跑一次 `scripts/soko setup` 或 `install.sh`）；
  但也**消除了 opencode 插件那种"启动即联网"的隐式行为**（对硬规则反而是收益，
  见 §6 决策 D-3）。

### G10 用户级技能环境干扰（P3，运维）
- 事实：`~/.agents/skills/lean4` 是指向 `~/.codex/skills/lean4` 的**断链软链**（目标不存在），
  DSH 会扫描 `~/.agents/skills`。
- 后果：当前只是让技能观察"不完整"（`complete:false`，不缓存），不影响本仓库技能；
  但一个叫 `lean4` 的技能名与硬规则语义冲突，建议清理。
- 另：`~/.agents/skills/{lark-*,mineru,mmx-cli}` 会出现在 sokonanoda 会话的技能目录里
  （噪音，非错误）。若在意，用 preset 的技能过滤或 `disable-model-invocation` 处理。

---

## 3. 资产可移植性判定表

| 资产 | DSH 可移植性 | 处置 |
|---|---|---|
| kernel / front / cli / lsp（Rust） | 完全可移植（与 harness 无关） | 不改 |
| `--json` 事件协议、`soko/*` 请求 | 协议可移植；`soko/*` 暂无 DSH 消费者（MCP 是备选，D19） | 不动协议；H5 才谈客户端 |
| `AGENTS.md` | 原生支持（D1，且 `CLAUDE.md` 一并读） | 改写 Setup 章节为 harness 中立 + 增加 DSH 段；不变动其结构 |
| `skills/*/SKILL.md` | 格式兼容（`name`+`description` 即合法）；相对 `references/` 受支持（D4） | 需加薄网关到 DSH 能扫的根；路由词写进 `description` |
| `.opencode/command/**` | 不等价（D14）；但技能名即命令（D5） | 内容并入技能 + 保留薄命令（opencode） |
| `.opencode/agent/teacher.md` | 不等价（D16：无项目级 agent/preset 根） | 角色正文并入 `sokonanoda-teacher` 技能 |
| `.opencode/plugins/sokonanoda.ts` | 无等价（DSH 插件是 npm/TS 包 + profile） | 抽出跨 harness 的**二进制解析器**（Node 启动器），DSH 侧用 patch/绝对路径 |
| `opencode.json` 的 deny | 无等价（G6：DSH 只有工具名过滤与会话级审批） | `tools/pre-execute` 插件或 hooks 桥（桥不做项目发现，D17） |
| `opencode.json` 的 formatter 关闭 | DSH 不做 Rust 自动格式化 | 无需适配 |
| `crates/cli/tests/opencode.rs` | 断言 opencode 专属内容 | 拆成 `opencode.rs` + 新 `dsh.rs` |
| VS Code 扩展 | 与 harness 无关 | 不改 |
| todo/goal/plan 面 | `dsh-base` 已挂，零配置（`todo_write`/`create_goal`/`exit_plan_mode`） | 教学/开发流程可直接依赖；在技能里写明用法 |
| 客户端 UI（Infoview 等价物） | 需 out-of-tree npm 包 + 预构建 `lib/client.js`；**无项目级自动加载** | H5 backlog（B1）；届时要自研 host 路由，不能用 in-repo typed remote |

---

## 4. 目标态（DSH 会话里应该长什么样）

```
用户：dsh web  （workspace = sokonanoda-lang 仓库根）
  │
  ├─ 自动：AGENTS.md 注入（D1）→ agent 知道读 REQUIREMENTS/HANDOVER/STATUS/ROADMAP
  ├─ 自动：.agents/skills/ 三个技能进目录 + /sokonanoda-teacher 可用（D2/D5）
  ├─ 可选：--patch ./dsh/cordis.patch.yml → lsp 工具对 .sokonanoda 可 hover/跳定义
  ├─ 必需：agent 能执行 `scripts/soko grade playground.sokonanoda`（G5 的解）
  │        → 内核判定 → --json 事件 → 出题/判卷/决定下一步（teacher 技能）
  ├─ 内置：todo/goal/plan 面（dsh-base 已挂）→ 教学轮/开发轮的状态追踪
  └─ 纪律：lean/lake/elan/leanc 要么被 hooks 拦，要么文档显式禁止（G6）
```

一句话验收：**在 DSH 里打开本仓库，只说一句"按 AGENTS.md 接手，然后当我的
Lean 老师"，agent 能自己把环境弄就绪并跑出一次判卷。**（见 §7 验收 A1）

---

## 5. 分阶段计划

> 原则：每阶段**独立可交付、可验收**；先解阻断项（G1/G5），再补接线（G2/G3/G4），
> 最后治理（G6/G7/G8）与远期（H4）。版本纪律：用户可见的接入形态变化 = minor bump
> （H2 的 `--patch` 属于可见新能力）；纯文档/测试不 bump（沿用 R82 先例）。

### H0 —— 技能上架（解 G1；P0）
0. **落点三选一**（决策 D-1）：
   - **(a) 符号链接**：`.agents/skills/<name> -> ../../skills/<name>`（git 记 mode 120000）。
     **零重复**（正文与 `references/` 都在原位，相对解析天然成立），DSH 官方仓库
     自用同款做法（D20，`.claude/skills -> ../.agents/skills`），watcher 默认跟随。
     风险：部分 harness/工具链对软链目录支持不齐；Windows 需要开发者模式或 admin。
   - **(b) 薄网关文件**：`.agents/skills/<name>/SKILL.md` 真文件，正文一行指向
     `skills/<name>/SKILL.md`。跨平台/跨 harness 最稳，代价是一份薄文件 + 镜像守卫。
   - **(c) `customSkillDirs` 指 `skills/`**：零重复、零镜像，但要用户改 `$DSH_HOME`。
   **建议**：若只保证本机 + macOS/Linux，选 (a)；若要把仓库当通用分发物，选 (b)；
   (c) 无论如何写进文档当可选加速。三者都要有契约测试守住。
1. **单一事实源**：无论 (a)/(b)，正文只存在 `skills/<name>/SKILL.md` 一处；
   `references/` 只存在 `skills/<name>/references/` 一处。
2. 三个技能 frontmatter 只补 `description` 里的路由词（**不要指望 `whenToUse`**：
   它只进人类 `/` 选单，模型目录只读 `name`+`description`，D3）；键名一律 kebab-case
   （`whenToUse` 是唯一 camelCase 例外，本仓库不需要用）。
3. `skills/README.md` 增加「DeepSeek Harness」安装矩阵一节：打开仓库即发现、
   `/sokonanoda-teacher` 即命令；`customSkillDirs` 只是可选加速。
4. 契约测试：`crates/cli/tests/dsh.rs` 断言
   - 每个 `skills/<name>/SKILL.md` 在 `.agents/skills/` 有对应入口（软链或网关，双向、无孤儿）；
   - 入口 frontmatter 有 `name`（== 目录名，kebab-case）与 `description`（非空）；
   - 若是网关：(i) 正文含 `skills/<name>/SKILL.md` 字面路径且该文件存在；
     (ii) 全部 frontmatter 键 ∈ DSH 白名单（`name`/`description`/`whenToUse`/`metadata`/
     `disable-model-invocation`/`user-invocable`），挡 camelCase 回归（D3）。
5. 验收：`cargo test -p sokonanoda-cli --test dsh` 绿；手工在 DSH 里确认
   `/` 选单出现三个技能名、`/sokonanoda-teacher` 加载正文成功、正文给出的
   `references/` 路径可读。

### H1 —— 二进制可达（解 G5；P0）
1. 新增 **`scripts/soko`**（Node ESM，零依赖、跨平台、可 commit、可执行位入 git）：
   解析顺序 = `$SOKONANODA_BIN` → `target/{release,debug}/sokonanoda`（按 mtime）→
   `~/.local/share/sokonanoda/bin/sokonanoda`（缓存）→ VS Code 扩展内嵌（按目录名
   版本号取最高、跳过 `.obsolete`）→ 版本锁定下载（`$SOKONANODA_OFFLINE=1` 可关）
   → 报错并给出可复制的下一步。**语义与 `.opencode/plugins/sokonanoda.ts` 的解析链
   完全一致**，抽出的正是它的 `findRepoRoot`/`repoBuild`/`extensionServer`/`markerMatches`。
   - 子命令直通：`scripts/soko doctor --json`、`scripts/soko grade playground.sokonanoda`。
   - **版本守卫**：缓存 marker（`<version> <target>`）与 `Cargo.toml` 不一致时，
     默认**不静默使用**，而是打印"缓存是 X，仓库是 Y，跑 `scripts/soko update`"。
2. `AGENTS.md` Setup 章节改为 harness 中立：先给 `scripts/soko …` 一条命令，
   再写 opencode 等价物与 DSH 等价物。
3. teacher/dev 技能里的命令统一改成 `scripts/soko …`（零 cargo 保持）。
4. 契约测试（`dsh.rs` 同文件）：`scripts/soko` 存在、可执行、`--help` 0 退出；
   解析链关键字齐全（防止有人把扩展自带/离线开关删掉）。
5. 验收：干净环境（PATH 无 `sokonanoda`）里 `scripts/soko doctor --json` 能报
   `ready:true`；`scripts/soko grade playground.sokonanoda` 与二进制直接调用逐字节同输出。

### H2 —— LSP 接线（解 G4 的"可用部分"；P1）✅ 已落地
1. **`dsh/cordis.patch.yml`**（顶层 YAML 数组；项目自带、用户显式 `--patch`）：
   一个 `insert:` 加三行——`@deepseek-ai/dsh-lsp` + `@deepseek-ai/dsh-lsp-stdio`
   + `@deepseek-ai/dsh-tool-lsp`（base profile 已有 `fs`/`subprocess`，无需补）。
   - `servers.sokonanoda`：`command` = `<repo>/scripts/soko`，`args: ['lsp']`，
     `extensionToLanguage: { ".sokonanoda": "sokonanoda" }`；
     `shutdownTimeoutMs: 10000`（冷缓存首编译不触发拆链）。
   - 路径解析：`!!js` 里用 `$SOKO_REPO ?? process.cwd()` + `path.join`；
     **不能用 `baseUrl`**（它是 profile 目录）、**必须单行**（D24）。
   - 实测探测：`playground.sokonanoda` ~22KB 远低于 4MB 上限；无需 `env.PATH`。
2. `dsh/README.md` 写清用法与边界（见 H4 的第 4 条）。
3. 文档显式写清能力边界（D8/D9）：DSH 下 LSP 只给 hover/跳定义/找引用/找实现；
   **服务端诊断不在通道内（`publishDiagnostics` 被显式丢弃），`soko/*` 无消费者，
   判卷一律走 `scripts/soko grade --json`**。
4. 契约测试（`crates/cli/tests/dsh.rs`）：patch 顶层是数组、`id`+`name` 齐、
   `extensionToLanguage` 含 `.sokonanoda`、指向仓库内启动器、无机器绝对路径。
5. **验收（已通过）**：以仓库为 workspace、加 `--patch` 启动 DSH 会话，`lsp`
   工具 hover `playground.sokonanoda:201:9` 返回内核打印的
   `theorem and_swap : forall (a b : Prop), And a b -> And b a` + “已通过内核检查”。

### H3 —— 命令与角色（解 G2/G3；P1）
1. 把 7 个 opencode 命令的**内容**并入技能（命令正文本身就是"跑哪条命令 + 怎么汇报"）：
   - `setup/update/version/doctor` → `sokonanoda-dev` 的「环境」节（或 teacher 的 §1）；
   - `check` → `sokonanoda-teacher` 的判卷节（已有）；
   - `gate`/`round` → `sokonanoda-dev` 的工作流节（已有，补 DSH 注意点）。
2. `.opencode/agent/teacher.md` 的角色正文（"第一步：用 skill 工具加载
   `sokonanoda-teacher`；不可违反 1–5 条"）**移入 `sokonanoda-teacher/SKILL.md` §0**；
   `.opencode/agent/teacher.md` 保留为薄壳（opencode 用户不变），并注明"DSH 无此文件，
   角色由技能承载"。
3. `AGENTS.md` 角色技能节改写：`/sokonanoda-teacher`、`/sokonanoda-dev`、
   `/sokonanoda-ci` 是 DSH 的原生入口（技能名即斜杠命令，D5）。
4. 契约测试：三个技能正文含 DSH 命令词汇（`/sokonanoda-*`）；`skill.rs` 现有
   「事件/方法词汇封闭」守卫保持绿。
5. 验收：DSH 里输入 `/sokonanoda-teacher` 即进入老师角色；`/sokonanoda-dev` 给出
   开发 SOP。

### H4 —— 治理与收尾（解 G6/G7/G8；P2）
1. **工具链 deny**（决策见 §6 D-4）：候选三条——
   (a) `tools/pre-execute` 的 native 插件（最干净，但要成一个插件包）；
   (b) Claude Code hooks 桥 `PreToolUse` + 仓库内 hook 脚本（解析 `tool_input.command`
   拦 `lean*`/`lake*`/`elan*`/`leanc*`；但**必须由用户在 profile 里插一行指向它**，
   因为桥不做项目发现，D17）；
   (c) 仅 `AGENTS.md` 文档禁令 + 一轮实测记录。
   无论选哪条，都要在 `AGENTS.md` 写明"DSH 侧的 deny 现状"。
2. **文档去 opencode 单一化**（33 个文件）：
   - `AGENTS.md`、`skills/README.md`、`docs/HANDOVER.md`、`docs/README.md`、
     `REQUIREMENTS.md`（§9 条目已加）、`README.md` 的 harness 描述；
   - `site/agents.html` / `site/assets/agent-prompt.js`：安装 prompt 增加
     "若用 DSH：打开仓库即可，技能在 `.dsh/skills`，判卷走 `scripts/soko`"；
   - `docs/design/onboarding.md` 增补 §「DSH 侧对照」并指向本文。
3. **门面同步**（硬规则）：`editor/vscode/README.md` + `CHANGELOG.md`（若 H2 视为
   用户可见能力）+ `STATUS.md`（新轮置顶）+ `REQUIREMENTS.md` §9 + `docs/HANDOVER.md`。
4. 验收：`rg -c opencode` 剩余命中都是"历史/as-built"或"opencode 专属"语境，
   不再有"唯一 harness"式陈述；`sokonanoda gate` 全绿。

### H5 —— 远期（不做承诺，入 backlog）

> **2026-09-17 更新**：B2「诊断通道」**已升级为独立设计与计划**——
> `docs/design/agent-query-channel.md`（ROADMAP **I15** / H6-A…H6-E）：不再"等 DSH
> 支持 `publishDiagnostics`"，而是把**内核真相查询层**从 LSP 里抽出来
> （`front::query`），再上 CLI `query` 与 MCP 两个薄传输。B1/B3/B4 仍留在下面。

- **B1 DSH 客户端插件复刻 Infoview**：把 `soko/goals`/`soko/stateAt` 接到 Web GUI
  的自定义视图（D9 证明现有 LSP 通道到不了；需要 DSH 客户端插件 + host handler）。
  → 将消费 `front::query` 的结论（I15 H6-A），不必再走 LSP 自定义请求。
- **B2 诊断通道**：**→ 已并入 I15**（`docs/design/agent-query-channel.md`）：
  ① CLI `sokonanoda query check/state/goals/holes/hints/reduce`（单 JSON、零配置、
     所有 harness 通用）；② MCP server（`scripts/soko mcp` + `dsh/mcp/server.js`，
     DSH 官方通道，默认关闭需 opt-in）；③ 追踪 DSH LSP 是否支持诊断投递作为补充。
- **B3 启动钩子**：`SessionStart` hook 自动 `setup`（注意 detached 语义，D17）。
- **B4 `dsh` 插件包**：把 CLI 解析器 + `tools/pre-execute` 拦截 + 自定义命令做成
  一个 npm 包，**同一个包同时声明 `dsh.bundle.patch`（宿主层）与 `dsh.client`
  （浏览器半）**（D23 的推荐封装），发布为 bundle（`dsh plugin add`，D18），
  从 `dsh/cordis.patch.yml` 一行引入。若要发 preset，用 D22 的自带技能目录写法。

---

## 6. 需要拍板的决策点

| # | 决策 | 候选 | 建议 |
|---|---|---|---|
| D-1 | 技能如何进 DSH | (a) `.agents/skills/<name>` **符号链接**到 `skills/<name>`（零重复、官方同款，D20；Windows 有软链限制）(b) 薄网关 `SKILL.md` 真文件（跨平台最稳，需镜像守卫）(c) 用户 profile 加 `customSkillDirs: [<repo>/skills]`（零重复，需一次性配置）(d) 直接 `--patch` 一个 `.dsh/repo.patch.yml` 同时带上技能与 LSP 行 | **本机/macOS/Linux 用 (a)，要跨平台分发用 (b)**；(c)/(d) 写进文档当可选加速，不要求用户改 `$DSH_HOME` |
| D-2 | 二进制启动器形态 | (a) Node `scripts/soko`（跨平台、可复用、DSH 有 Node）(b) POSIX sh（Windows 不可用，且仓库已有"删掉 soko.sh"的决定）(c) 什么都不加，文档写绝对路径 | **(a)**；REQUIREMENTS（三十二）删的是**面向用户的** `scripts/soko.sh`，这里是**agent/编辑器接入层**，性质不同，需在 §9 注明这条边界 |
| D-3 | 是否要项目级自动 provisioning | (a) 不做，`scripts/soko` 首次运行自行下载（显式、可离线）(b) 用 hooks 桥在 SessionStart 自动 setup | **(a)**：符合"下载按版本锁定 + 用户可见"；自动联网属隐式行为，opencode 插件那条路是历史包袱 |
| D-4 | Lean 工具链 deny 形态 | (a) `tools/pre-execute` native 插件（最干净，需成插件包）(b) hooks 桥 `PreToolUse` + 仓库内 hook 脚本（需用户在 profile 插一行指向它，因为桥**不做项目发现**）(c) 仅文档禁令 | **(b) 先试**（成本最低、可立即验证），不行再 (c) 并记录；(a) 归入 H5 B4 |
| D-5 | `soko/*` 自定义请求怎么办 | (a) DSH 侧不管，判卷走 CLI（推荐）(b) MCP server 承载（D19）(c) H5 客户端插件复刻 Infoview | **(a)**；把 (b)/(c) 记 backlog，不阻塞 H0–H4 |
| D-6 | 版本号 | (a) H2 落地时 bump minor（新增 DSH 接入形态）(b) 全部不 bump | **(a)**：`--patch` 文件是用户可见新能力，按 `AGENTS.md` 门面同步规则走 |

---

## 7. 验收标准（可执行）

- **A1（主循环）**：DSH + workspace=仓库根，用户说"按 AGENTS.md 接手并当我的老师"，
  agent 能（1）跑通 `scripts/soko doctor --json` 报 `ready:true`；（2）跑
  `scripts/soko grade playground.sokonanoda` 得到事件计数；（3）说出下一个练习的
  名字与剩余目标。**零 cargo。**
- **A2（技能）**：DSH 技能目录含 `sokonanoda-teacher`/`-dev`/`-ci`；`/sokonanoda-teacher`
  能加载 teacher 正文；teacher 的 `references/{events,curriculum,zh-style}.md` 可通过
  正文给出的路径读到。
- **A3（LSP）**：`dsh web --patch ./dsh/cordis.patch.yml` 后，对 `playground.sokonanoda`
  的身份符 hover 出内核类型；`goToDefinition` 命中声明。**文档同时写明诊断不在通道内。**
- **A4（回归）**：`cargo test --workspace --locked` 全绿（含新 `dsh.rs`）；
  `sokonanoda gate` PASS；opencode 侧原有断言不退化（`opencode.rs` 仍绿）。
- **A5（文档）**：`AGENTS.md` Setup 章节不再假定 opencode 唯一；`REQUIREMENTS.md` §9
  有本轮条目；`docs/HANDOVER.md` §3/§5 反映 DSH 现状与 gotchas。
- **A6（反漂移）**：`skills/` 与 `.dsh/skills/` 的网关一一对应由测试守护；
  `dsh/cordis.patch.yml` 的形状由测试守护；文档里不再出现"opencode 是唯一 harness"。

---

## 8. 风险

| 风险 | 影响 | 缓解 |
|---|---|---|
| 技能双份（`skills/` + `.agents/skills/`）漂移 | agent 读到旧指令 | 首选软链（D20，零双份）；用网关时**只有一行指针**（正文唯一在 `skills/`）+ 契约测试双向断言（A6） |
| DSH 快速迭代（developer preview，会 break） | patch 形状/技能根可能变 | 事实全部标注证据行号；`dsh.rs` 只断言"我们自己的文件形状"，不断言 DSH 内部行为 |
| 把"LSP 诊断"当既有能力写进文档 | 下一个 agent 白等诊断 | `AGENTS.md`/技能/H2 文档三处显式写"诊断走 CLI `--json`" |
| hooks 桥字段能力不足（D-4） | deny 退化 | 先小样验证再全量；退化路径已写进计划 |
| 版本漂移（当前已发生：缓存 0.16.2/0.20.0 vs 仓库 0.54.0） | `doctor` 报未就绪、判卷结果与源码不符 | H1 的启动器**内置 marker 校验**，不一致就停下报错而不是静默跑旧二进制 |
| 用户 profile 被改坏 | DSH 起不来 | H2 一律给 `--patch` 一次性路径；改 profile 的指引强调"整块替换 config、会丢 `!!js`、空文件会 boot 失败"（D21） |
| 软链在 Windows/某些工具链下不生效 | 技能不被发现 | D-1 给 (b) 真文件网关作为跨平台退路，且两者共用同一份契约测试 |

---

## 9. as-built（实现进度）

> 2026-09-17 第八十七轮：**H0–H4 全部落地**，版本 0.54.0 → **0.55.0**。
> 验收证据见下表末列。

| 阶段 | 状态 | 证据 |
|---|---|---|
| 调研 + 本设计 | ✅ 2026-09-17（第八十六轮） | 本文 §1 全部带 DSH 源码行号 |
| H0 技能上架 | ✅ 完成 | `.agents/skills/{teacher,dev,ci}/SKILL.md` 三个薄入口；`crates/cli/tests/dsh.rs` 6 个测试（入口↔正文双向、kebab-case 名、DSH frontmatter 白名单、指向正文且路径存在、patch 形状、启动器链）；**DSH 会话里已实测自动出现三个技能 + `/sokonanoda-*` 命令** |
| H1 二进制可达 | ✅ 完成 | `scripts/soko`（零依赖 Node，可执行位入 git）：`doctor --json` → `ready:true`；`setup` 实测把缓存从 0.16.2/0.20.0 刷到 0.55.0；`grade playground.sokonanoda` 输出内核事件；`version` 报 `[repo-build]` 并拒绝旧缓存；`AGENTS.md` Setup 改 harness 中立；三个技能命令统一为 `scripts/soko …` |
| H2 LSP 接线 | ✅ 完成 | `dsh/cordis.patch.yml`（`dsh --dump-config` 确认解析）；**DSH headless 会话实测 `lsp` 工具 hover `playground.sokonanoda:201:9` → `theorem and_swap : forall (a b : Prop), And a b -> And b a` + “已通过内核检查”**；`dsh/README.md` 写清用法与"诊断不在通道内"；D24/D25 两条实测坑已记录 |
| H3 命令与角色 | ✅ 完成 | teacher 技能 §0 吸收角色与五条不可违反规则；`.opencode/agent/teacher.md` 瘦身为指针（并修掉其中违反零 cargo 硬规则的 `cargo run` 判卷命令）；7 个 opencode 命令改用 `scripts/soko`；`AGENTS.md` 角色技能节改写；`opencode.rs` 改为"gate 之外的命令必须 cargo-free + 全部走启动器" |
| H4 治理 | ✅ 完成 | `dsh/hooks/{hooks.json,refuse-lean-toolchain.js}`（命令位匹配，`lake build`/`$(lean …)` 拦、`grep lean` 放行，实测 12 例）；`AGENTS.md` 硬规则第 2 条写明两 harness 的 deny 形态；`skills/README.md` 重写为多 harness 安装矩阵；`docs/design/onboarding.md` §6 DSH 对照表；site 安装 prompt 改 `scripts/soko` + DSH 说明；VS Code README/CHANGELOG/package.json 与版本同步 |
| H5 backlog | ⬜ 未做 | B1–B4 见上文 |
| 回归 | ✅ 完成 | `cargo test --workspace --locked` 全绿（21 个测试目标，含新增 `dsh.rs`）；`cargo fmt --check` 绿；`cargo clippy --workspace --all-targets` 仅 kernel 既有 warning；`scripts/soko gate` **PASS**；`gen-site-data.py` + `check-site.py` 绿 |

### 落地时新增的实测事实（已并入 §1.2）

- **D24**：`!!js` 必须单行；`baseUrl` 是 profile 目录，不能用它推导仓库路径。
- **D25**：`lsp` 工具只在会话 workspace 内解析 `file_path`，DSH 的工作区/启动目录
  必须是本仓库根（从别处启动用 `SOKO_REPO`）。
- **本机环境坑（不属 DSH）**：Xcode 27 的许可未接受时 `xcrun`/`ar` 全被拦，
  `cargo` 链接必失败。绕过（不改系统设置）：
  `DEVELOPER_DIR=/Library/Developer/CommandLineTools cargo test …`。
  根治：`sudo xcodebuild -license accept`。

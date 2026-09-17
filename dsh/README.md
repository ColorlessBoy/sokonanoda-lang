# DeepSeek Harness 接入（`dsh/`）

这里是 **DeepSeek Harness (DSH)** 的接入文件。设计、证据与完整差距清单见
`docs/design/deepseek-harness.md`；本文只讲**怎么用**。

## 一分钟接入

```bash
# 1) 环境（版本锁定的 CLI + LSP → 缓存；幂等；零 cargo）
scripts/soko setup
scripts/soko doctor --json     # 期望 "ready": true

# 2) 让 DSH 认识这张画布（LSP：hover / 跳定义 / 找引用）
cd <这个仓库>
SOKO_REPO=$PWD dsh web --patch ./dsh/cordis.patch.yml
```

- **工作区必须是本仓库根**：`lsp` 工具只在会话 workspace 内解析 `file_path`，
  DSH 启动目录默认就是 workspace root，所以"`cd` 进仓库再启动"即可；
  从别处启动时用 `SOKO_REPO=<仓库绝对路径>` 告诉 patch 去哪找 `scripts/soko`。
- patch 里的 `!!js` 路径表达式**必须单行**、且**不能用 `baseUrl`**
  （DSH 的 `baseUrl` 是 `$DSH_HOME/profiles/`，不是 patch 文件目录）——
  这两条都是实测踩出来的，见 `docs/design/deepseek-harness.md` §1.2 D24。

技能与斜杠命令**不需要任何配置**：DSH 自动扫仓库根的 `.agents/skills/`，
技能名就是命令。打开会话后直接输入 `/sokonanoda-teacher` 开始上课；
`/sokonanoda-update` 与 `/sokonanoda-doctor` 是两个人工运维命令（环境过期 /
就绪诊断），模型目录里看不到它们——那是刻意的。

## 你会得到什么 / 不会得到什么

| 能力 | DSH | 说明 |
|---|---|---|
| `/sokonanoda-teacher`、`/sokonanoda-dev`、`/sokonanoda-ci` | ✅ 零配置 | 技能名的斜杠命令；模型目录也能自动发现 |
| `/sokonanoda-update`、`/sokonanoda-doctor` | ✅ 零配置 | **人工**运维命令（`disable-model-invocation`）：刷新缓存 / 只读就绪诊断。DSH 命令名不允许 `/`，所以 opencode 的 `/sokonanoda/update` 在这里写作 `/sokonanoda-update`。`update` 的退出码有语义：`0` = 缓存写成了，**`3` = 缓存没写成**（stderr 给 `download:` 原因，即使有可用回退也照报）——DSH 的沙箱常让 `~/.local/share` 不可写，这时按 `skills/sokonanoda-update/SKILL.md` 的"常见失败"处置 |
| 模型的 `lsp` 工具 hover `.sokonanoda` | ✅ 需 `--patch` | hover 文本来自内核 pretty printer（真类型/真目标态） |
| 跳定义 / 找引用 / 找实现 | ✅ 需 `--patch` | 比纯文本 grep 精确 |
| **编辑器内诊断（`publishDiagnostics`）** | ❌ | DSH 的 LSP host 明确忽略服务端通知（`packages/lsp/lsp-stdio/src/connection.ts`） |
| **`soko/goals`、`soko/stateAt`、`soko/nextHole`、`soko/hints`** | ❌ | 自定义请求需要宿主插件；见设计文档 H5 backlog |
| 判卷（内核判定 + 结构化事件） | ✅ 走 CLI | `scripts/soko grade playground.sokonanoda --json` |

> **判卷永远走 CLI `--json` 或 `query`**，别等 LSP 诊断。这是 DSH 与
> opencode/VS Code 最大的能力差异，`AGENTS.md`、三个技能与设计文档都已写明。

## 内核真相查询（`query` 与 MCP）

判卷有两种粒度，都是**同一个内核**产出的（`front::query` 是唯一真相）：

```bash
# 摘要/单点查询：一个 JSON 对象，agent 一次解析
scripts/soko query check  --file playground.sokonanoda
scripts/soko query state  --file playground.sokonanoda --line 327 --col 4
scripts/soko query holes  --file playground.sokonanoda --direction next --offset 20460
scripts/soko query hints  --file playground.sokonanoda --line 323 --col 3
scripts/soko query reduce --file playground.sokonanoda --expr '1 + 1'

# 全量事件流（既有通道，opencode/CI/脚本在用）
scripts/soko grade playground.sokonanoda --json
```

**两个视图的计数由契约测试钉死一致**（`crates/cli/tests/query.rs`），所以不会
出现"两套真相"。`query` 的契约见 `docs/protocol.md`（信封 `soko.query/1`、
退出码语义、"`ok:false` 不是空结果"的区分）。

要给**模型**用（而不是你自己跑 shell），`dsh/cordis.patch.yml` 里的 MCP 行把它
包成六个工具：

| MCP 工具（DSH 里看到的名字） | 转发到 |
|---|---|
| `mcp__sokonanoda__check` | `query check` |
| `mcp__sokonanoda__state` | `query state`（Lean `goalsAt?` 语义） |
| `mcp__sokonanoda__goals` | `query goals` |
| `mcp__sokonanoda__holes` | `query holes`（稳定 id + 导航 + `redundant` 标记） |
| `mcp__sokonanoda__hints` | `query hints`（`-- soko:hint` 阶梯） |
| `mcp__sokonanoda__reduce` | `query reduce` |

实测（本机 DSH headless）：模型调用 `mcp__sokonanoda__state`
（`playground.sokonanoda:327:4`）拿到 `Exists Person P` —— 目标文本由内核渲染，
模型不需要自己扫源码猜。

**信任边界（要知情）**：MCP server 是 DSH 沙箱之外的**可信可执行代码**
（`@deepseek-ai/dsh-mcp-client` 用 SDK 直接 spawn），所以它默认**关闭**，
需要上面那行 `--patch` 才生效；只想要 LSP 的话删掉 `mcp-sokonanoda` 块即可。
server 本身只是 JSON 转发器（`dsh/mcp/server.js`，零依赖、无 Lean 逻辑），
且 MCP 工具仍走常规工具流水线，guard/hook 可以按 `mcp__sokonanoda__*` 名字拦。

## 常驻安装（可选）

一次性 `--patch` 每次启动都要带。想常驻，把 `dsh/cordis.patch.yml` 里的
`insert:` 列表**整体**复制进：

```
$DSH_HOME/profiles/web/cordis.patch.yml     # 只影响 web profile
$DSH_HOME/cordis.patch.yml                  # 影响所有 profile
```

三个注意点（都是 DSH 的语义，别踩）：

1. **`- id:` 覆写是整块替换** —— 覆写某行的 `config` 不会深合并，而且会丢掉
   其中的 `!!js` 表达式；所以复制 `insert:` 列表，别写覆写行；
2. patch 文件**不能为空或只有注释** —— 会让 DSH 启动失败；要禁用这一层写 `[]`；
3. 自查是否生效：`dsh --profile web --dump-config` 会打印每行由哪个文件提供、
   被哪些 overlay 改过。

## 技能入口形态（`.agents/skills/`）

DSH 扫的根只有 `<repo>/.dsh/skills` 与 `<repo>/.agents/skills`（不递归），
所以本仓库在 `.agents/skills/<name>/SKILL.md` 放了**薄入口**：合法 frontmatter +
一句"正文在 `skills/<name>/SKILL.md`，按仓库根解析"。

- 正文唯一源仍是 `skills/<name>/SKILL.md`（opencode / Claude Code 走同一份）；
- 入口与正文的对应关系、frontmatter 键的合法性、`scripts/soko` 的解析链、
  本目录 patch 文件的形状，全部由 `crates/cli/tests/dsh.rs` 守住；
- 三个入口的 `description` 是**模型唯一可见的路由字段**（DSH 的 `whenToUse`
  只进人类 `/` 选单），所以路由词写在 `description` 里。

## 禁用官方 Lean 工具链（硬规则落地）

`REQUIREMENTS.md` §2 第 2 条禁止调用 `lean`/`lake`/`lean4export`/`leanc`/`elan`。
opencode 用 `opencode.json` 的权限规则强制；**DSH 没有命令模式策略**，等价手段是
Claude Code 风格的 `PreToolUse` hook：

仓库已自带 `dsh/hooks/hooks.json` + `dsh/hooks/refuse-lean-toolchain.js`
（命中即退出码 2，理由回给模型）。但 DSH 的 hooks 桥**只读一个进程级
`configPath`，不做项目发现**，所以要用户在 profile 里加一行：

```yaml
# $DSH_HOME/profiles/web/cordis.patch.yml   （或 $DSH_HOME/cordis.patch.yml）
- insert:
    - id: hooks-claude-code
      name: '@deepseek-ai/dsh-hooks-claude-code'
      config:
        configPath: /abs/path/to/sokonanoda-lang/dsh/hooks/hooks.json
        projectDir: /abs/path/to/sokonanoda-lang
```

一行验证（应退出码 2 并给出理由）：

```bash
echo '{"tool_name":"bash","tool_input":{"command":"lake build"}}' | node dsh/hooks/refuse-lean-toolchain.js; echo "EXIT=$?"
```

匹配的是**命令位**的 `lean|lake|leanc|lean4export|elan`（可带 `.exe`，允许
`cd x && lake …`、`$(lean …)`），所以 `grep -rn lean docs/` 这类**不会**被误拦。
没开这个 hook 时，这条硬规则在 DSH 里退回为文档纪律（`AGENTS.md` 硬规则速记）。

## 换 harness 时怎么办

| 你用什么 | 环境 | 技能 | 诊断反馈 |
|---|---|---|---|
| DeepSeek Harness | `scripts/soko` | 自动（`.agents/skills/`）+ `/sokonanoda-*` | 仅 hover/跳转；判卷走 CLI |
| opencode | 启动插件自动 provision + PATH；或 `scripts/soko` | 自动（`opencode.json` 的 `skills.paths`）+ `/sokonanoda/*` | 完整 LSP 诊断 |
| VS Code | `scripts/soko` 或扩展内置二进制 | 不适用（人用） | 完整 LSP 诊断 + Infoview |
| 其他 / headless | `scripts/soko` | 手动加载 `skills/<name>/SKILL.md` | 判卷走 CLI `--json` |

`scripts/soko` 是**所有 harness 通用**的那一条命令：它解析版本匹配的二进制
（仓库构建 → 缓存 → VS Code 扩展自带 → 版本锁定下载），缓存过期就拒绝运行。

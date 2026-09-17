# 内核真相查询通道（设计 + 计划：`query` 子命令 / MCP server）

> 状态：**设计定稿，实现未开始**。本文取代 `docs/design/deepseek-harness.md` §5 H5
> 的 B2（"诊断通道：MCP 或等 DSH 支持 publishDiagnostics"），并把它**从根上重做**：
> 不是"再写一个 MCP server"，而是先把**内核真相查询层**从 LSP 里抽出来。
> 触发：用户 2026-09-17「这个你来设计一下开发文档，从根上正确解决。同时看一下前人
> 留下的两个 TODO，需要更新一下」。
> 权威顺序：`REQUIREMENTS.md`（要求总账）> 本文 > `AGENTS.md`。
> 关联：`docs/protocol.md`（`--json` 事件 + `soko/*` 契约）、
> `docs/design/deepseek-harness.md`（harness 接线，H0–H4 已落地）、
> `docs/design/goal-rendering.md`（semantic runs 单一分类源）、
> `docs/design/infrastructure.md`（LSP-first 总体设计）、`docs/HANDOVER.md` §3 E（两个 TODO）。

---

## 0. 一句话结论

**"内核真相"目前只有一条出口：LSP。** 而 DSH 的 LSP host 明确丢弃服务端诊断、也不
调用自定义请求（`docs/design/deepseek-harness.md` §1.2 D8/D9），于是 DSH 里的 agent
只能靠 `sokonanoda --json <file>` 读全量事件流来自己重建状态——既啰嗦（整文件事件）
又要求 agent 自己实现"光标处是哪个目标""下一个洞在哪"这类逻辑。

正确的解法分三层，**顺序不能颠倒**：

1. **真相层**（Rust，新增 `front::query`）：把"问内核"这件事做成一组**编辑器无关的
   类型化查询**（`check` / `state` / `goals` / `holes` / `hints` / `reduce`），
   只返回结论，不返回渲染细节。**真相只有一份，任何适配器都不得自己算。**
2. **传输层**（两个薄适配器）：`sokonanoda query <op>`（单 JSON 对象，零配置、
   所有 harness 通用）+ `scripts/soko mcp`（MCP stdio server，OS 中立、
   给 DSH 的 `mcp__*` 工具用）。
3. **现有 LSP**：改为调用同一个真相层，**不再自己算**——这一步同时偿还
   `crates/lsp/src/lib.rs` **4256 行**（远超 ~500 行红线）的结构债。

一句话：**先把真相抽出来，再谈传输**。反过来做（先写 MCP server 再让它调 LSP 或
复制 LSP 逻辑）会立刻产生第二份真相，违反硬规则（判定永远走 kernel，禁止任何
"看起来一样"的近似实现）。

---

## 1. 现状与差距

### 1.1 真相今天长在哪（审计）

| 能力 | 今天的实现位置 | 消费者 |
|---|---|---|
| 整文件判卷事件 | `crates/cli/src/{check,json_report}.rs` + `front::session` | CLI `--json`、opencode、DSH、测试 |
| 声明级 goal 列表 | `crates/lsp/src/lib.rs:295 goal_decls` + `:373 goals` | VS Code 练习树 / Infoview |
| 光标处状态（Lean `goalsAt?`） | `crates/lsp/src/lib.rs:429 state_at` + `:724 select_state_at` | Infoview、tactic hover、`soko/stateAt` |
| 洞导航 | `crates/lsp/src/lib.rs:393 next_hole` | VS Code「下一个洞」 |
| 提示阶梯 | `crates/lsp/src/hints.rs` | VS Code hover/命令 |
| 半表达式/开项类型 | `crates/lsp/src/lib.rs` hover 分支 + `front::goals::probe_sub_goal_types` | hover |
| semantic runs | `front::semantic`（✅ 已是唯一分类源） | goal/ty 渲染 |

**问题**：除了最后一行，其余全部**长在 LSP 适配器内部**，并且把
"选择/判定"（真相）与"LSP 类型构造"（表示）焊在一起。CLI 与 MCP 今天无法复用它们。

### 1.2 差距清单

| # | 差距 | 后果 |
|---|---|---|
| Q1 | 真相不可复用：`goal_decls` / `state_at` / `next_hole` 都在 LSP 里 | 第二份实现一旦出现就是"两套真相"；MCP 只能重写或反向依赖 LSP |
| Q2 | `crates/lsp/src/lib.rs` **4256 行**（REQUIREMENTS §4 红线 ~500） | 每次加能力都在加深欠账，且无法单元测试纯逻辑 |
| Q3 | agent 侧只有"全文件事件流" | 问"光标处目标是什么"必须整文件判卷 + 自己在事件里找；token 浪费、易误判 |
| Q4 | DSH 拿不到诊断（D9） | 内核反馈只能靠 agent 主动跑 CLI，**没有一条"提问式"通道** |
| Q5 | 两个 front 缺口（HANDOVER §3 E） | 索引递归 `Prop` 的 recursor 派生被内核拒（课程只能手写 `rec`/`iota`）；`inductive` 参数不吃多名字组 |
| Q6 | 查询类能力**零契约测试**（协议测试只覆盖 `--json` 事件与 `soko/*` 的 LSP 形状） | 新通道若与 LSP 结论不一致，没有测试会红 |

---

## 2. 需求（本设计的正确性标准）

1. **单一真相**：每个结论只有一处实现；LSP、CLI、MCP 都调用它，且**永不重算**
   （例如"光标处是哪个目标"的语义只在 `front::query::state_at` 里定义一次）。
2. **零配置可达**：任何 harness（含没有 MCP 的）都能用 `sokonanoda query …` 拿到
   结论，且零 cargo、版本锁定（沿用 `scripts/soko` 与硬规则 §2.9）。
3. **机器可读且稳定**：单 JSON 对象、字段只增不改、带 `version` 供丢弃过期答案；
   事件流（`sokonanoda --json <file>`）**契约不变**（已有消费者）。
4. **不新增判定路径**：所有结论仍由完整 kernel 产出（硬规则 §2.4）；新增代码禁止
   文本比对、禁止近似启发式判定。
5. **可测试**：真相层走 front 单测；CLI 走端到端；MCP 走"声明即契约"的快照测试；
   LSP 与 CLI/MCP **逐字段一致**由契约测试钉死（防两套真相）。
6. **顺手清债**：抽层后 `crates/lsp/src/lib.rs` 必须下降（目标 ≤ ~1200 行，
   查询/渲染逻辑外移），`crates/front/src/query.rs` 自身 ≤ ~500 行。

---

## 3. 架构

### 3.1 分层

```
                      ┌───────────────────────────────┐
   真相（唯一）        │  front::query  （新增，纯逻辑） │
                      │  check / state / goals / holes │
                      │  / hints / reduce  → 类型化结构 │
                      └───────────┬───────────────────┘
                                  │ 复用（不是反向依赖）
        ┌─────────────────────────┼─────────────────────────┐
        ▼                         ▼                         ▼
  cli::query                lsp::{goals,…}            （未来：客户端插件）
  `sokonanoda query <op>`   soko/* 自定义请求
        │
        ▼
  scripts/soko mcp  →  MCP stdio server  →  DSH `mcp__sokonanoda__*`
```

- `front::query` 只依赖 `front` 自身（`session`/`semantic`/`goals`/`by`/`parser`）
  与 kernel 公开 API；**不依赖 `lsp`、不依赖任何传输类型**。
- `cli::query` 负责参数解析、JSON 序列化、退出码。
- `lsp` 的 `soko/*` 处理函数改为：调 `front::query` → 把结构映射成 LSP 形状
  （`Range`、`goal_runs` 等）。**映射层保持薄**，语义零改动。
- MCP server（Node，`dsh/mcp/server.js`）只做协议与 JSON Schema，**逻辑全部转发**
  给 `scripts/soko query …`。

### 3.2 文件布局（模块化硬规则）

| 文件 | 职责 | 预估行数 |
|---|---|---|
| `crates/front/src/query.rs` | 类型化查询：参数/结果结构 + `state_at`/`goals`/`holes`/`hints`/`check` 的**选择与判定** | ~450 |
| `crates/front/src/query/render.rs` | 把 `semantic` runs 与文本组装成结果字段（从 LSP `render.rs` 平移） | ~250 |
| `crates/cli/src/query.rs` | 子命令解析 + JSON 输出 + 退出码表 | ~300 |
| `crates/lsp/src/lib.rs` | **下降**：删掉查询/渲染实现，只留协议映射与生命周期 | 4256 → ≤1200 |
| `dsh/mcp/server.js` | MCP stdio：`tools/list`、`tools/call`，JSON Schema 声明 | ~220 |
| `crates/cli/tests/query.rs` | CLI 端到端 + 与 LSP 一致性契约 | ~250 |

> `front::query` 若逼近 500 行，按 op 拆 `query/{state,goals,holes,hints}.rs`，
> 公开 API 用 re-export 保持稳定（REQUIREMENTS §4）。

---

## 4. 真相层：`front::query`

### 4.1 公共类型（草案）

```rust
/// 查询失败的**机器可判**原因（不是 panic，也不是空结果）。
pub enum QueryError {
    /// 源文本无法解析到可回答的程度（含 parse 诊断）。
    NotParsable,
    /// 光标不在任何声明内。
    OutsideDeclarations,
    /// 该位置没有洞（`holes`/`next_hole` 的正常空结果是 `None`，不是错误）。
    NoHoleAtCursor,
    /// 文档没有打开的目标（`state.goals` 为空的正常情形，不是错误）。
    NoOpenGoals,
}

/// 一次查询的可序列化结果。`version` 供消费者丢弃过期答案（沿用 soko/stateAt 语义）。
pub struct QueryAnswer<T> { pub version: u64, pub data: T }
```

**关键设计选择**：`None`/空数组表达"正常的没有"，`QueryError` 表达"问不出来"。
今天 LSP 用 `goal: null` + 默认字段混合表达两者，agent 无法区分——**这正是 Q3 的
根源**，新通道必须分开。

### 4.2 六个操作（语义 = 今天的 LSP 行为，逐条对照）

| op | 语义（唯一真相） | 结果要点 |
|---|---|---|
| `check` | 整文件判卷（= CLI `--json` 的**汇总**，不改变事件流契约） | 各事件计数、每个开放练习的 `name`/`goals[]`/`holes[].id`、诊断 `{stage,code,message,span,hint}`、`warning` |
| `state` | 光标处状态，**Lean `goalsAt?`**：光标在某 tactic 的 span 内 → **进入**该 tactic 之前的状态；否则停在"最后一条在光标前结束的 tactic"之后；首个 tactic 之前 → 根状态 | `decl{name,kind,status,range}`、`goals[{goal,goal_runs,binders[]}]`（**全部**剩余目标，当前在前）、`step`/`total`、`span` |
| `goals` | 声明级：一个文档里每个声明的类型/状态/开放目标/洞；**含请求期内核探针**补的子洞期望类型（`front::goals::probe_sub_goal_types`） | `decls[{name,kind,status,range,ty,ty_runs,goals[],holes[{range,id}],sub_goals[{range,ty}],code_actions[]}]` |
| `holes` | 洞导航与寻址 | `holes[{id,range,ty,decl}]`（文件序）+ `next`/`prev`（相对给定 `--offset`），**id 是唯一稳定引用**（`soko/nextHole` 的同址限制在文档里已注明，这里以 id 为准） |
| `hints` | 画布里的 `-- soko:hint` 阶梯（无状态；协议从不数剩余条数） | `hints[string]` + 所属声明名 |
| `reduce` | 对给定表达式求值（= REPL `#reduce` 的真相） | `value`（内核 pretty print）、`type` |

> `check` 与 `sokonanoda --json <file>` 的关系：**同一份判卷，两种视图**。
> `check` 是"摘要 + 可寻址"，`--json` 是"全量事件流"，二者由同一个
> `front::session::Session` 产出，**契约测试必须断言两者一致**（§7 A4）。

### 4.3 位置语义

- 输入位置用**字节 offset**（真相层）还是 `line`/`character`（LSP/UTF-16）？
  真相层用 **offset**（唯一、无编码歧义）；适配器负责转换：
  CLI 接受 `--offset` 或 `--line N --col M`（1-based，UTF-16 与 LSP 对齐，
  由 CLI 转换），MCP 用 `line`/`character`（与 DSH `lsp` 工具一致，避免模型两套坐标）。
- 显式拒绝"客户端自己扫源码找洞"：`holes` 与 `state` 的定位全部服务端算
  （ocaml-lsp 教训，已写在 `protocol.md`）。

---

## 5. 传输 A：CLI `query`（零配置，所有 harness 通用）

### 5.1 命令形状

```bash
# 输入：--file <path>（磁盘）或 --text <src>（未落盘的中间态）或 stdin（'-'）
sokonanoda query check  --file playground.sokonanoda
sokonanoda query state  --file playground.sokonanoda --line 201 --col 9
sokonanoda query goals  --file playground.sokonanoda --probe
sokonanoda query holes  --file playground.sokonanoda [--offset 4103] [--direction next|prev]
sokonanoda query hints  --file playground.sokonanoda --line 201 --col 9
sokonanoda query reduce --file playground.sokonanoda --text '1 + 1'
```

- **单 JSON 对象**输出（不是 NDJSON）：agent 直接 `json.loads` 即可，不用拼行。
- `--compact` 关掉缩进；`--version` 输出真相层协议版本。
- `--text` 是给"agent 手上有文本但还没落盘"的场景（对应 LSP 的 didChange 内存态），
  也是**避免读写竞态**的关键：agent 可以先问再写。

### 5.2 退出码（沿用仓库既有约定）

| code | 含义 |
|---|---|
| 0 | 查询成功（**含 `check` 有开放练习**，与 `--json` 一致：`sorry` 是合法状态） |
| 1 | `check` 有内核拒绝的声明（`failed > 0`） |
| 2 | 用法错误（未知 op / 缺参数 / 位置越界） |
| 3 | 环境未就绪或二进制不可用（与 `doctor`/启动器一致） |

**判据永远是 JSON 内容，不是退出码**（`skills/sokonanoda-teacher` 已如此要求）。

### 5.3 契约（新增，写入 `docs/protocol.md`）

- 顶层恒有 `{"schema": "soko.query/1", "op": …, "version": …, "ok": bool, "data": …}`；
- `ok:false` 时必有 `error: {"code": "<QueryError 名>", "message": …}`；
- 字段**只增不改**；改名视为破坏性变更（需 minor bump + 文档 + 测试三件套）。

---

## 6. 传输 B：MCP server（`scripts/soko mcp`）

### 6.1 为什么 MCP 是对的补充（以及它的边界）

- DSH 的 `publishDiagnostics` 被显式丢弃（D9），**LSP 这条路在 DSH 里走不通**；
  MCP 是 DSH 官方支持的、能把结构化能力交给模型的通道（`dsh-mcp-client`）。
- 但 MCP **不是唯一必要通道**：CLI `query` 已经可用，且零配置。MCP 的价值是
  ① 让模型不必记命令行、由 schema 引导参数；② 结果为**一等工具结果**，
  进入 DSH 的 tool-call 展示与审计。
- MCP server 的**唯一职责**是协议与 schema：它 `spawn` `scripts/soko query …`
  并转发 JSON。**它不解析 Lean、不判卷、不做文本处理**——保证没有第二份真相。

### 6.2 工具映射（工具名 + 模型可见描述 = 契约）

| MCP 工具 | 转发 | 关键参数（JSON Schema） |
|---|---|---|
| `soko_check` | `query check` | `file?`, `text?`, `bare?` |
| `soko_state` | `query state` | `file`/`text`, `line`, `character`（1-based UTF-16，与 DSH `lsp` 工具同坐标） |
| `soko_goals` | `query goals` | `file`/`text`, `probe?` |
| `soko_holes` | `query holes` | `file`/`text`, `offset?`, `direction?`（enum `next`/`prev`） |
| `soko_hints` | `query hints` | `file`/`text`, `line`, `character` |
| `soko_reduce` | `query reduce` | `file`/`text`, `expr` |

- 每个工具的 `description` **必须写清何时用**（模型路由），并显式说明
  "`sorry` 是合法开放状态""判定走完整内核"，避免模型把开放练习报成错误。
- 返回：文本内容 = 子进程的 JSON 对象（原样），**不二次包装**；子进程退出码
  2/3 映射为 MCP 工具错误（`isError: true`），1 作为正常结果（内含失败清单）。

### 6.3 接线（写入 `dsh/README.md`，作为 `--patch` 的一部分）

**已核实**（`docs/config-catalog.md:1600-1650`、`docs/user/guide/mcp-memory.md`、
`apps/cli/config/examples/mcp-memory/*.cordis.yml`）：

- 插件是 **`@deepseek-ai/dsh-mcp-client`**，`config` 是 `StdioConfig | StreamableHttpConfig`
  的**顶层联合**（没有嵌套 key）：`transport`（`'stdio'` / `'streamable-http'`）、
  `serverName`、`command`、`args`、`env`、**`cwd`**、`toolCallTimeoutMs`、
  `failOnStartupError`、`maxInstructionBytes`、`reconnect`。
- 模型看到的工具名 = **`mcp__<serverName>__<rawName>`**；`serverName` 必须匹配
  `[A-Za-z0-9_-]{1,32}` 且进程内唯一。
- stdio 桥会**先剥掉凭据形状的环境变量与全部 `DSH_*`** 再启动子进程；其余环境继承。
- DSH 只负责"启动命令/连接 URL + 发现工具"，**不下载 server、不装依赖**
  （与我们的硬规则一致：server 就是仓库里已有的 Node 脚本）。

于是 DSH 侧的一行是（与 LSP 行同处一个 overlay，**默认关闭**）：

```yaml
- insert:
    - id: mcp-sokonanoda
      name: '@deepseek-ai/dsh-mcp-client'
      config:
        transport: stdio
        serverName: sokonanoda
        command: node
        args: ['scripts/soko-mcp.js']      # 相对 cwd
        cwd: !!js process.env.SOKO_REPO ?? process.cwd()   # 单行（D24）
        toolCallTimeoutMs: 60000
        failOnStartupError: true          # server 起不来就报错，别静默无工具
```

**`cwd` 比 `command` 的绝对路径更干净**：`cwd` 把仓库根交给子进程，于是
`scripts/soko` 的 `process.cwd()` 回退分支直接命中（与 LSP 行同样的解析逻辑，
但不需要 `!!js` 拼路径）。代价：`cwd` 也要用 `!!js`，仍然**必须单行**
（`docs/design/deepseek-harness.md` §1.2 D24 的实测结论）。

**MCP server 在仓库里的位置**：`dsh/mcp/server.js`（与 LSP 接线、hooks 同属于
`dsh/` 这个"harness 接线"目录），由 `scripts/soko mcp` 统一入口转发（保持
"所有 harness 入口都经 `scripts/soko`"的纪律）；`args` 里引用它时按 `cwd` 相对化。

- **默认关闭**：MCP server 是"沙箱之外的可信可执行代码"（DSH 的明确态度），
  所以它属于**用户显式 opt-in**（`--patch` 或 profile 行），不写进默认接线。
- `scripts/soko mcp` 必须在二进制缺失时**退出非零并给出可行动错误**
  （让 DSH 的 MCP 客户端报出可读原因，而不是静默无工具）。

---

## 7. 两个 TODO（HANDOVER §3 E）—— 与本轮的关系与更新

用户要求"看一下前人留下的两个 TODO，需要更新一下"。二者都不是独立小修：
**它们直接决定查询通道给出的"真相"是否完整**，因此并入本计划的 H6-C 阶段，
不再挂在 `front` 的零散待办里。

| TODO | 现状 | 与本设计的关系 | 处置 |
|---|---|---|---|
| **A. `derive_recursor` 派生不了「索引递归 `Prop`」的 recursor**（`Le`/`Even` 省略 `rec` 时 IK 形状被内核拒；`Or`（非索引 Prop）、`Vec`（索引 Type）正常） | 课程 #9 手写 `rec`/`iota` 规避 | `goals`/`holes` 的期望类型与 `check` 的失败诊断都会经过 recursor/iota 路径；派生错误会让"真相"在课程最常见的关系类归纳上失真 | **修 front 派生逻辑**（kernel 冻结不动），按 TDD 三层 + 课程用例；见 H6-C 与 §8 |
| **B. `inductive` 参数不吃多名字 binder 组**（`(A B : Prop)`；Pi/箭头位已支持） | 课程只能写 `(A : Prop) (B : Prop)` | 解析器能力缺口会让 agent 写出的合法 Lean 子集被拒——**agent 是主要作者**，这个缺口对查询通道的可用性影响更大 | **修 parser**（含同类缺口全量排查），见 H6-C 与 §8 |

> 更新动作（本轮已做）：`docs/HANDOVER.md` §3 E 两条标注"并入 ROADMAP I15 /
> `docs/design/agent-query-channel.md` H6-C"，不再作为孤立 front 待办。

---

## 8. 分阶段计划

> 依赖顺序硬约束：**真相层先于一切适配器**（§0）。每阶段独立可验收、可发布。

### H6-A —— 真相层 + CLI `query`（P0）
1. `front::query`：`QueryError`/`QueryAnswer` + `check`/`state`/`goals`/`holes`/`hints`/`reduce`，
   语义逐条对照 §4.2 的 LSP 行为（**先把 LSP 的实现平移过来，再删 LSP 侧重复**）。
2. `front::query::render`：从 `crates/lsp/src/render.rs` 平移 runs/文本组装。
3. `cli::query` + `sokonanoda query …` + 退出码表 + `--compact`/`--text`。
4. `docs/protocol.md` 新增"`query` 子命令"一节（§5.3 的契约）。
5. **LSP 改为调用真相层**（本阶段就做完，否则欠债翻倍）：`soko/*` 处理函数只做
   映射；`crates/lsp/src/lib.rs` 目标 ≤1200 行。
6. 测试：front 单测（每个 op 的正常/边界/错误）；CLI e2e（`crates/cli/tests/query.rs`）；
   **一致性契约**：同一文件同一光标，`query state` 与 `soko/stateAt` 字段级一致
   （`goal`/`goals[]`/`binders`/`step`/`total`），`query goals` 与 `soko/goals` 一致。

### H6-B —— MCP 传输 + DSH 接线（P1）
1. `dsh/mcp/server.js`：MCP stdio（`initialize`/`tools/list`/`tools/call`），
   六工具 schema，全部转发 `scripts/soko query …`；`--help` 可自描述。
2. `scripts/soko mcp` 转发入口（复用既有解析链；二进制缺失→非零+可行动错误）。
3. `dsh/cordis.patch.yml` 增 MCP 行（默认关闭，注释写清"可信可执行、按需开启"）；
   `dsh/README.md` 增"内核真相查询"一节：**CLI 优先、MCP 可选**，并给出一句话验收。
4. 测试（`crates/cli/tests/dsh.rs` 扩展）：server.js 存在、六个工具名与
   `query` op 一一对应、schema 里有 `line`/`character`、无硬编码机器路径；
   手工验收：DSH 会话里模型调用 `soko_state` 拿到当前目标。
5. 版本 minor bump（新增用户可见能力）。

### H6-C —— 两个 TODO（P2，可与 A/B 并行但**必须在发布前**）

> **已核实的根因（2026-09-17，就地读代码，行号已复核）**：

**TODO B（parser 多名字 binder 组）——根因明确、修法明确**

- `parse_inductive_block`（`crates/front/src/parser.rs:357`）在 `:` 之前循环
  `while matches!(peek, LParen | LBrace) { params.push(self.parse_binder()?) }`，
  而 `parse_binder`（同文件 `:1063`）**只读一个名字**——所以 `(A B : Prop)` 在读到
  `A` 后期望 `:`，撞上 `B` 就报错。
- 同文件 `:723` 的 `parse_binder_group` 已经支持多名字，且返回 `BinderGroup{names, ty, style, span}`
  （`names: Vec<String>`、`ty: Expr`）；`:659`（`parse_arrow` 的 `(a b c : T) -> body`）
  与 `:1035` 已在用，展开成逐名字链——**这就是"Pi 位已支持"的原因**。
- 修法：inductive 参数改用 `parse_binder_group()`，把 `group.names` 摊平成
  `Vec<Binder>`（每个名字一个 `Binder{name, ty: Some(group.ty.clone()), style: group.style}`，
  span 取 group 的；**注意 `Binder.ty` 是 `Option<Box<Expr>>`，`ast.rs:196`**）。
- 同类缺口一并排查（`parse_ctor` 同文件 `:397` 同样只认 `LParen` + `parse_binder`）：
  修复要把 **inductive 参数**与 **ctor 字段**两处都换成组感知；`let`/`match` 的 binder
  路径需逐个确认（调研第 4 条列出）。

**TODO A（索引递归 `Prop` 的 recursor 派生）——可疑点已定位，形状待最终确认**

- `derive_recursor` 在 `crates/front/src/compile/elab.rs:2492`；决定"小消去"的判据是
  `:2499`：`let small_elim = is_prop_block_ty(ty) && constructors.len() > 1;`
- `is_prop_block_ty`（`:2429`）**已经**会剥掉索引望远镜（注释明写"带索引归纳的 `ty`
  是索引望远镜"）——所以 `Le : Nat -> Nat -> Prop`、`Even : Nat -> Prop` 都判定为 Prop 块，
  **且二者都恰好有 2 个 ctor，`constructors.len() > 1` 也为真** → `small_elim = true`。
  也就是说：**问题不在"是否走小消去"，而在小消去分支里生成的
  motive/IH/索引实参形状**（`Or` 非索引、`Vec` 索引但 `Type`（`small_elim = false`）
  都不触发该分支，与现象吻合）。

**已用发布版二进制实测（2026-09-17，可复制的复现）**

- **TODO B 复现（parse 阶段就红）**：

  ```sokonanoda
  inductive Pair2 (A B : Prop) : Prop      -- 报 `expected binder type, found Ident("B")` @ 1:20
  ctor mk2 (a : A) (b : B) : Pair2 A B
  end
  ```
  对照组 `(A : Prop) (B : Prop)` 完全通过（`checked declaration Pair1`）。

- **TODO A 复现（内核阶段红）**：把 `course/unit9-relations-connectives.sokonanoda`
  的 `inductive Even : Nat -> Prop` + 两个 ctor **保留、去掉 `rec Even.rec` 块**，
  再 `end`：

  ```sokonanoda
  inductive Even : Nat -> Prop
  ctor even_zero : Even Nat.zero
  ctor even_succ (n : Nat) : Even n -> Even (Nat.succ (Nat.succ n))
  end
  ```

  实测（在自带 `inductive Nat` 的 Bare 上下文里）：

  ```
  kernel-rejected: 类型不匹配：期望 `Pi (motive : Pi (i : Nat), Pi (x : Even $0), Sort 0),
    Pi (m0 : (($0 Nat.zero) even_zero)), Pi (m1 : Pi (n_ : Nat), Pi (x_ : Even $0),
    Pi (ih : (($3 $1) $0)), ($4 ((even_succ …`，
    实际是 `Pi (motive : Pi ( : Nat), Pi (t : Even $0), Sort 0),
    Pi (even_zero : (($0 Nat.zero) even_zero)), Pi (even_succ : Pi (n : Nat), Pi ( : Even $0),
    Pi (v_1_0 : (($3 $1) $0)), (($4 (…`
  ```

  **差异确实落在 IH/实参片段**（`Pi (m0 : …) Pi (m1 : …)` vs
  `Pi (even_zero : …) Pi (even_succ : …)`，以及 `((… $3 $1) $0)` 之后的形状）。
  手写 `rec Even.rec : (motive : (n : Nat) -> Even n -> Prop) -> …`（course #9 的做法）
  则通过——**所以根因在派生器产出的这条 telescope，而不是索引或 ctor 本身**。
- **下一步（H6-C 第 1 条）**：把上面两个类型分别用内核 `debug_print` 打印出来逐段对齐
  （哪个 binder 多/少、哪个 de Bruijn 索引错位），再定修法。**不要**先动
  `constructors.len() > 1` 这条判据——`Le`/`Even` 已经满足它，改它必然误伤。

1. **先写复现测试**（两件都要求"修复前红"）：TODO B → `inductive Foo (A B : Prop)` 的
   parser 单测；TODO A → 上面那段 `Even` 形状（省略 `rec`）作为 front 单测/CLI e2e 的
   输入，断言派生成功且判定走通。
2. 修 TODO B（parser，含 ctor 字段与同类路径），补 front 单测 + 课程同步。
3. 修 TODO A（front 派生逻辑到内核接受的 IH 形状），**三层回归**
   （front 单测 + CLI e2e + 课程语料），并把课程 #9 的手写 `rec`/`iota` 收回为自动派生
   （golden 会变，按纪律同轮同步并记录理由）。
4. 两个修复都不得触碰 kernel（冻结）；若发现必须改 kernel 才能修，**停下并回设计**
   （那是范围变更，不是 bugfix）。

> **顺带记录一个 Bare 模式口径**（实测）：文件自带 `inductive N2` 时，**点号构造子名
> `N2.z2` 不可用**（`unknown identifier`），要用裸名（Bare 模式正是这样教的）；
> 点号形式只对 prelude 内建（`Nat.zero`/`Bool.true`）成立。课程语料与 `match`
> 分支写法以现有文档为准，此处仅备查，**不属于本轮改动**。

### H6-D —— 文档/门面收尾（P2）
1. `AGENTS.md` 命令段增 `query`（agent 首选查询方式）；teacher 技能补
   "问目标用 `query state`，别整文件扫事件"；dev 技能补"真相层不得绕过"。
2. `docs/ARCHITECTURE`/`TESTING`/`HANDOVER`/`ROADMAP` 同步；`REQUIREMENTS.md` §9 追加。
3. 门面同步（硬规则）：VS Code README/CHANGELOG/package.json（若 LSP 行为无变化，
   只在 CHANGELOG 记"内部重构，无行为变更"）+ `site/` 若提到 agent 用法。

### H6-E —— 远期（原 H5 其余项，不做承诺）
- B1 Infoview 客户端插件（消费 `query goals/state`，需 out-of-tree npm 包 + 预构建 bundle）；
- B3 `SessionStart` 自动 provisioning；B4 把启动器 + Lean deny 拦截 + 命令打包为 npm 插件。

---

## 9. 待调研确认（不阻塞 H6-A；开工前补齐）

> **已就地核实（2026-09-17，本节第 1/2 条的核心部分）**：
> `@deepseek-ai/dsh-mcp-client` 的 `Config` 是 `StdioConfig | StreamableHttpConfig`
> 顶层联合，字段含 **`cwd`**（子进程工作目录）、`toolCallTimeoutMs`、
> `failOnStartupError`；工具名 = `mcp__<serverName>__<rawName>`；stdio 桥先剥
> 凭据形状变量与全部 `DSH_*`。证据：`docs/config-catalog.md:1600-1650`、
> `docs/user/guide/mcp-memory.md`、`apps/cli/config/examples/mcp-memory/*.cordis.yml`。
> 结论已写进 §6.3。**仍待确认的**：

1. ~~**DSH MCP client**：配置 schema、传输、工具命名、失败语义~~ → 已核实（见上）。
   仍需：结果内容类型（text/image/resource/structured）与**是否支持 resources/prompts**
   （`packages/mcp/mcp-resources` 的存在暗示支持，需确认模型能否 `resources/read`）、
   以及 schema/description 传递时是否有截断。
2. **路径解析**：`cwd` 的 `!!js` 求值上下文已确认与 LSP 行相同（**必须单行**）；
   仍需确认 `args` 里的相对路径是相对 `cwd` 还是相对 DSH 启动目录
   （决定 `args: ['scripts/soko-mcp.js']` 是否成立；不确定时改成 `!!js` 绝对路径）。
3. **项目侧可交付性**：**已确认 DSH 不读项目级 MCP 配置**（官方工作示例一律
   `--patch <文件>` 或并入用户 profile 层，`mcp-memory.md`「Enable one」），
   与 hooks 桥结论一致 → **用户显式 opt-in 是唯一路径**（已写进 §6.3）。
   若将来 DSH 支持项目级发现，本设计只需把同一段 YAML 挪个位置。
4. **两个 TODO 的根因**：`derive_recursor` 的 IH 形状错在哪一行、
   parser 的单名路径清单（用于一次修完同类缺口）。

---

## 10. 决策点（需拍板）

| # | 决策 | 候选 | 建议 |
|---|---|---|---|
| QD-1 | 真相层落点 | (a) `front::query`（新模块）(b) `lsp` 内抽子模块 | **(a)**：`front` 才是"编辑器无关的编译器结论"该在的地方；LSP 已是 4256 行 |
| QD-2 | CLI 输出形态 | (a) 单 JSON 对象 (b) 复用 NDJSON 事件流 | **(a)**：agent 一次解析；事件流契约保持不变（向后兼容） |
| QD-3 | MCP 默认开关 | (a) 默认关闭、`--patch` 开启 (b) 默认开启 | **(a)**：DSH 把 MCP server 当**沙箱外可信代码**；项目不得替用户默认扩大信任面 |
| QD-4 | MCP 与 CLI 的关系 | (a) MCP 转发 CLI (b) MCP 直接链接 Rust（如 napi/子进程 LSP 桥） | **(a)**：零重复、可离线验证；MCP 只是协议外壳 |
| QD-5 | 坐标系统 | (a) 真相层 offset + 适配器转换 (b) 全链路 UTF-16 | **(a)**；MCP 表面用 `line`/`character`（与 DSH `lsp` 工具一致），CLI 两种都给 |
| QD-6 | 两个 TODO 是否本轮做 | (a) 并入 H6-C（推荐）(b) 继续挂 front 待办 | **(a)**：查询通道的"真相"完整性依赖它们，且 agent 是主要作者 |
| QD-7 | 版本号 | (a) H6-A 落地 bump minor（新增 `query`）(b) 合并到 H6-B 一起 bump | **(a)**：`query` 是 agent 可见的新能力，先发布先可用 |

---

## 11. 验收标准（可执行）

- **A1（真相唯一）**：`rg -n "fn select_state_at|fn goal_decls" crates/` 只命中
  `front/src/query.rs`（LSP 侧已无实现，只剩映射）。
- **A2（CLI 可用）**：`scripts/soko query check --file playground.sokonanoda` 输出单 JSON，
  含事件计数与每个开放练习的 `goals[]`/`holes[].id`；`query state --line 201 --col 9`
  给出该处目标与假设；`query holes` 在 playground 上返回全部洞且 `id` 唯一。
- **A3（DSH MCP）**：加 `--patch` 后 DSH 会话里模型能看到 `soko_*` 工具；
  调用 `soko_state` 返回与 CLI 逐字段一致的 JSON。
- **A4（一致性，防两套真相）**：契约测试断言
  `query state` ≡ `soko/stateAt`、`query goals` ≡ `soko/goals`（字段级），
  并且 `query check` 的计数 ≡ `--json` 事件流的计数。
- **A5（结构债）**：`crates/lsp/src/lib.rs` ≤1200 行；`crates/front/src/query*.rs` ≤500 行/文件；
  `cargo clippy` 教学 crates 零 warning（`[lints] deny` 不变）。
- **A6（两个 TODO）**：`Le`/`Even` 省略 `rec` 时自动派生通过内核（课程改为依赖自动派生，
  golden 同步）；`inductive Foo (A B : Prop)` 解析通过并有三层测试；
  两个修复各有"修复前红"的复现测试（输入见 §5 H6-C 的两段可复制用例）。
- **A7（回归）**：`cargo test --workspace --locked` 全绿；`scripts/soko gate` PASS；
  既有 `--json`/`soko/*` 契约测试**不改判据**（只加，不改）。

---

## 12. 风险

| 风险 | 影响 | 缓解 |
|---|---|---|
| 抽层时**语义漂移**（LSP 原行为与 `front::query` 不一致） | 编辑器与 agent 看到不同结论 | A4 的字段级一致性契约测试是硬门禁；先平移后删除，绝不同时改语义 |
| 两套真相（有人在 LSP 里"顺手"补逻辑） | 违反硬规则、长期不可维护 | A1 的 `rg` 断言收进 `crates/cli/tests/query.rs`；dev 技能写明"真相层不得绕过" |
| `--json` 消费者被打断 | opencode/VSIX/CI 全红 | 事件流契约**只增不改**；`check` 是新增视图而非替换 |
| MCP server 逃出沙箱/泄露凭据 | 安全事故 | 默认关闭 + 文档写明"信任边界"；server 只 spawn 仓库内二进制、不读凭据 |
| 两个 TODO 修复牵动课程 golden | 大面积测试红 | 按 TDD 三层逐条修；golden 变更在同一轮一次性同步并记录理由 |
| `crates/lsp` 重构引入回归 | 编辑器体验倒退 | 现有 LSP 测试（`soko/*`、hover、inlay、code action）**保持全绿且不修改**，作为重构的安全网 |

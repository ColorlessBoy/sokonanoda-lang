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
6. **顺手清债**：抽层后 `crates/lsp/src/lib.rs` 必须下降（查询/渲染逻辑外移 +
   按职责拆文件），`crates/front/src/query*.rs` 自身 ≤ ~500 行/文件。
   **as-built（含一次我自己写错又改正的结论）**：删掉重复实现后 lib.rs 是 3988 行，
   我**没量就**判定"≤1200 是拍脑袋的目标、剩下的都是协议服务代码"，并在四处文档里
   把口径改成"不追行数"。这是错的：`wc -l` 一下就知道 3988 行里 **2638 行是
   `#[cfg(test)]` 模块**，非测试代码只有 ~1350 行。把测试模块移出文件、再抽
   `protocol.rs`（wire 类型）与 `tokens.rs`（semantic token 辅助），
   **lib.rs = 1105 行，≤1200 达标**（`4256 → 3988 → 1105`）。
   教训进 `docs/LESSONS.md`（**改验收标准之前先把被验收的东西量一遍**）。

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

| 文件 | 职责 | 预估 → **as-built** 行数 |
|---|---|---|
| `crates/front/src/query/mod.rs` | `QueryDoc` + 六个 op 的选择与判定 + 小工具（`decl_name`/`status_str`/`parse_or_empty`/`span_offsets`） | ~450 → **452** |
| `crates/front/src/query/state.rs` | `select_state_at` + `StateSelection`（`goalsAt?` 语义的唯一实现） | （原在 mod.rs）→ **93** |
| `crates/front/src/query/pos.rs` | offset ↔ 1-based 行/列（UTF-16 列）换算 | （原在 mod.rs）→ **55** |
| `crates/front/src/query/types.rs` | wire 类型（`DeclInfo`/`StateAnswer`/`QueryError`/…） | ~220 → **217** |
| `crates/front/src/query/tests.rs` | 真相层单测 | → **402** |
| `crates/cli/src/query.rs` | 子命令解析 + JSON 输出 + 退出码表 | ~300 |
| `crates/lsp/src/lib.rs` | **下降**：删掉查询/渲染实现，抽 wire 类型与 token 辅助，测试模块移出文件 | **4256 → 3988 → 1105**（≤1200 达标） |
| `crates/lsp/src/protocol.rs` | `soko/*` 自定义请求的 wire 类型（从 lib.rs 抽出） | → **159** |
| `crates/lsp/src/tokens.rs` | semantic token 的 legend/encoding（从 lib.rs 抽出；分类唯一源仍是 `front::semantic`） | → **107** |
| `crates/lsp/src/tests/`（`mod.rs` + 9 个特性文件）/ `by_sorry_range_tests.rs` | LSP 进程内 rpc 测试（0.56.0 从 lib.rs 移出、0.56.1 按特性拆分，**断言与测试名一字未改**） | → `mod.rs` **399**、最大子文件 **392** / **60** |
| `dsh/mcp/server.js` | MCP stdio：`tools/list`、`tools/call`，JSON Schema 声明 | ~220（as-built **~350**，含五个实测坑的注释） |
| `crates/cli/tests/query.rs` | CLI 端到端 + 与 LSP 一致性契约 | ~250 → **~600**（含真实 LSP 二进制的对拍与两个画布） |

> `front::query` 逼近 500 行时按"选择 / 坐标 / 类型 / 测试"拆文件，公开 API 用
> re-export 保持稳定（REQUIREMENTS §4）。**as-built 已照此执行**：`mod.rs` 曾经
> 581 行（加了 `select_state_at` 与位置换算之后），已拆出 `state.rs` 与 `pos.rs`。

---

## 4. 真相层：`front::query`

> **as-built 补充（2026-09-17，实现后回填）**：落地为
> `crates/front/src/query/{mod.rs,types.rs,tests.rs}`。两点与草案不同，都记在这里：
>
> 1. **`select_state_at` 的正确语义以协议 + LSP 为准**（我在草案里写错过，实现时被
>    一致性检查抓出来）：
>    - tactic 命中用**半开区间** `start <= cursor < end`——光标恰在某 tactic 的
>      **结束偏移**上算"在该 tactic 之后"，不算"之内"；
>    - **根状态**（`step: -1`）= `ty_text`（完整声明类型的内核渲染，退路是走查的
>      剩余目标）+ **空 binders** + `span` = 声明范围。
>    我最初写成闭区间、根状态用走查后的剩余目标 + 真实 binders——`docs/protocol.md`
>    与 VS Code 客户端依赖的是前者。**这正是"两套真相"的活样本**：当时没有任何测试
>    会红，所以 H6-A 的 A4 一致性契约不是形式主义。
> 2. 草案里的 `QueryAnswer<T>` 未落地：各 op 直接返回自己的类型（`StateAnswer` 自带
>    `version`，其余由 CLI 信封统一带），少一层包装。
> 3. **`select_state_at` 还有第二个分支要靠协议、不能靠直觉**（这一条是抽层时
>    **穷举对拍**抓出来的，不是测试抓出来的）：
>    - **没有 `by` 块**的声明（`axiom`、lambda 前缀 + `sorry` 的半成品、已证完的
>      `:=` 证明）：`step: -1`、`total: 0`，且**退回声明自己的剩余目标/上下文**
>      （`goal` + `binders`；已闭合时为 `[]` ⇒ wire `goal: null`）。
>    - 它与"**根状态**"（上面第 1 条：`ty_text` + 空 binders）是**两回事**——根状态
>      只属于"有 tactic 的声明，光标在第一条之前"。
>    - 我最初把两者合成一个 `root()`，后果是：已证的声明凭空多出一个目标（Infoview
>      显示"还剩目标"），半成品证明丢掉已引入的假设（`fun (a) (h) => sorry` 的
>      `a`/`h` 消失）。
>    - **怎么抓到的**：LSP 委派落地后，用**临时探针**把删掉前的 LSP 实现与
>      `front::query::select_state_at` 在 5 个画布的**每一个光标 offset**（0..=len）
>      上逐字段对拍，共 **709 次比较**：所有 `by` 声明完全一致，**424 处不一致全部
>      落在这一个分支**。
>    - **为什么既有测试没红**：LSP 唯一覆盖该分支的用例
>      （`state_at_without_by_steps_returns_the_declaration_goal`）用的画布没有 lambda
>      前缀，于是"剩余目标"恰好**等于**声明类型——两个语义在它上面取值相同。
>      教训：**"新增的测试通过"不等于"新语义被测试"**；一致性契约必须覆盖**分支的
>      判别性输入**（`crates/cli/tests/query.rs::query_state_and_lsp_agree_without_a_by_block`
>      现在就是这么写的，front 侧另有两条红先单测）。

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

> **as-built（0.59.0，WO-003/G-10 + G-17）**：这条原则原先有两个漏洞——解析失败时
> `check` 答"全零 + `failed: []`"、`goals`/`holes` 答空数组，都是 `ok:true`。现在
> `goals`/`holes` 走 `QueryError::NotParsable`（`ok:false`），`check` 把 parse 诊断
> 合成进 `failed[]`（`check` 是唯一例外：它的**答案**就是"这份文本解析不了"，所以
> `ok` 保持 `true`，见 §5.2 as-built）。

### 4.2 六个操作（语义 = 今天的 LSP 行为，逐条对照）

| op | 语义（唯一真相） | 结果要点 |
|---|---|---|
| `check` | 整文件判卷（= CLI `--json` 的**汇总**，不改变事件流契约） | 各事件计数、每个开放练习的 `name`/`goals[]`/`holes[].id`、诊断 `{stage,code,message,span,hint}`、`warning`。**as-built**：摘要层的 `FailedDecl` 今天只有 `{name,code,message,start,end}`（无 `stage`/`hint`），**同时**承载内核拒绝与 parse 诊断（用 `code` 区分；解析失败时 `name: null`、`counts` 全 0、退出码 1） |
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
| 1 | 这份文件被拒：`check` 有内核拒绝的声明（`failed > 0`）**或源文本解析失败**（`failed[]` 里就是 parse 诊断；`goals`/`holes` 答 `not-parsable`） |
| 2 | 用法错误（未知 op / 缺参数 / 位置越界） |
| 3 | 环境未就绪或二进制不可用（与 `doctor`/启动器一致） |

**as-built（0.59.0，WO-003/G-10 + G-17）**：解析失败不再假绿——`check` 把
`QueryDoc::parse_error` 合成进 `failed[]`（`counts` 保持全 0、`ok:true`、退出码 1），
`goals`/`holes` 走 `QueryError::NotParsable`（`ok:false` + 退出码 1）；"正常的没有"
（空数组 / `navigated: null`）仍是 `ok:true`。

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

**DSH 侧已核实的硬事实（2026-09-17 源码勘察，`path:line` 见 §9）**：

| 事实 | 对设计的影响 |
|---|---|
| 模型看到的工具名 = `mcp__<serverName>__<rawName>`；`serverName` 只能 `[A-Za-z0-9_-]{1,32}`，**没有 prefix 选项**；名字超 64 字符会被截断并附 12 位哈希 | 用 `serverName: sokonanoda` → 工具名如 `mcp__sokonanoda__state`；**原始工具名要短**（`state` 而不是 `get_goal_state_at_cursor`），哈希只在超限时出现 |
| **启动是 eager**：`apply` 里 `startConnection` + `await connection.ready`；stdio 下 SDK 会**先起一个一次性探测进程**、再起真正服务的进程 | server 必须**启动快、可被起两次**（幂等、无副作用、不要预编译/预下载）；也解释了为什么 server 只做 JSON 转发——预热成本必须接近零 |
| 连接失败 → 警告 + 重连退避（默认 500ms→30s，10 次）；`failOnStartupError: true` 在**内置 profile 里也不会让 boot 失败**，只是该行 inactive + 带标签警告 | 我们仍设 `true`（要响），但**不能依赖它阻断**；server 侧启动即失败要有清晰 stderr |
| `toolCallTimeoutMs` 默认 60000，**连接/协商/发现没有 DSH 自己的超时**（走 SDK 默认 60s） | 保持默认；我们的 server 转发是"有界的一次子进程调用"，不会挂住 |
| 模型可见内容 = `content[].text` 拼接；`structuredContent` **不进模型可见文本**（只对程序化/PTC 调用者可见） | **所有模型要读的 JSON 必须放在 `content[].text` 里**（我们本来就是这个设计：原样转发的 JSON 文本） |
| `description` 与 `inputSchema` **逐字传递、无截断**（唯一体积上限是 `maxInstructionBytes` 管 server instructions） | schema 可以写全约束；但描述要精简（进 KV cache，越短越好） |
| **server instructions 是一条独立的 system prompt 段**（`### MCP server: <name>` + 文本，上限 32768 字节） | 用它交代"`sorry` 是合法状态""判定走内核""坐标是 1-based UTF-16"——比塞进每个工具的 description 更省 token |
| 资源走 **`mcp-resources` 的三个固定工具**（`list_mcp_resources`/`list_mcp_resource_templates`/`read_mcp_resource`，都要显式 `server`）；**MCP prompts 完全不支持**；服务端**无法 push**（除 tools/list_changed） | 可选增量：把画布/课程/协议文档当 MCP resource 暴露，模型按需 `read_mcp_resource`。**不做 prompt 模板**（不支持），**不做推送**（不可能） |
| stdio 子进程 env = 洗净后的父 env + `config.env`；`/KEY\|PASSWORD\|SECRET\|TOKEN/i` 与全部 `DSH_*` **都被剥掉** | 我们的 server 只依赖 `cwd` 与仓库内相对路径，**不依赖任何环境变量**；文档里提醒 `SOKO_REPO` 若要用必须写进 `config.env` |
| **MCP server 不在沙箱内**（SDK 的 cross-spawn，不经 `ctx.subprocess`/sandbox），以用户全权运行 | 与"默认关闭 + 可信代码"的判断一致，文档必须写明这一条信任边界 |
| `--patch` 的相对路径按**DSH 启动目录**解析（`path.resolve`），`command`/`cwd` 也从不按 patch 文件目录解析；**没有项目级 MCP 自动发现** | 用户必须从仓库根启动（或 `config.env` 传 `SOKO_REPO`）；`cwd: !!js process.cwd()` 是官方同款写法 |


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
| **A. `derive_recursor` 拒绝「带索引 + 箭头写法字段」的归纳**（实测：`P : Nat -> Prop` + `ctor b (n : Nat) : P n -> P (Nat.succ n)` 被内核拒；同形状改**具名字段**即通过；索引 `Type` 一样失败；**与 `Prop` 无关**） | 课程 #9 手写 `rec`/`iota` 规避 | `goals`/`holes` 的期望类型与 `check` 的失败诊断都会经过 recursor/iota 路径；派生错误会让"真相"在课程最常见的关系类归纳上失真 | **修 front 一处**（`elab.rs:2613` 的 `src_spine` → `spine_of_codomain`）+ 顺带修 `is_k` 写死；见 H6-C |
| **B. `inductive` 参数不吃多名字 binder 组**（`(A B : Prop)`；Pi/箭头位已支持） | 课程只能写 `(A : Prop) (B : Prop)` | 解析器能力缺口会让 agent 写出的合法 Lean 子集被拒——**agent 是主要作者**，这个缺口对查询通道的可用性影响更大 | **修 parser 两处**（`parser.rs:363`/`:402` 改调 `push_binders`；AST/elab 不用动）；见 H6-C |

> 更新动作（本轮已做）：`docs/HANDOVER.md` §3 E 两条标注"并入 ROADMAP I15 /
> `docs/design/agent-query-channel.md` H6-C"，不再作为孤立 front 待办。

---

## 8. 分阶段计划

> 依赖顺序硬约束：**真相层先于一切适配器**（§0）。每阶段独立可验收、可发布。

### H6-A —— 真相层 + CLI `query`（P0）✅ 已完成（真相层 + CLI；LSP 见下）
1. `front::query`：`QueryError`/`QueryAnswer` + `check`/`state`/`goals`/`holes`/`hints`/`reduce`，
   语义逐条对照 §4.2 的 LSP 行为（**先把 LSP 的实现平移过来，再删 LSP 侧重复**）。
   ✅ `crates/front/src/query/{mod,types,state,pos,tests}.rs`（`QueryDoc` + 18 个单测）。
2. `front::query::render`：从 `crates/lsp/src/render.rs` 平移 runs/文本组装。
   ✅ 收在 `QueryDoc::runs`（唯一分类源仍是 `front::semantic`），未单开文件。
3. `cli::query` + `sokonanoda query …` + 退出码表 + `--compact`/`--text`。
   ✅ `crates/cli/src/query.rs`；退出码收敛为"有没有答案"（结构化错误 0 / 内核拒绝 1 / 用法 2）。
4. `docs/protocol.md` 新增"`query` 子命令"一节（§5.3 的契约）。✅
5. **LSP 改为调用真相层**：`soko/*` 处理函数只做映射。
   ✅ 已完成：`crates/lsp/src/query_map.rs` 只做"offset ↔ LSP `Range`/`Position`、
   `QueryError` → 空结果"的映射，语义全部来自 `front::query`；LSP 侧的
   `select_state_at` 实现已删除（`rg -n "fn select_state_at" crates/` 只命中 front）。
   行数**双达标**：删重复后 3988 行，再把两个测试模块移出文件、抽出
   `protocol.rs`（wire 类型）与 `tokens.rs`（semantic token 辅助）→ **lib.rs 1105 行**
   （≤1200）。见我中途"没量就改标准"又改正的记录（§2.6 / §11 A5 / `docs/LESSONS.md`）。
6. 测试：✅ front 单测 18 项（含 no-`by` 两条红先回归；`cargo test -p
   sokonanoda-front --lib query::`）；✅ CLI e2e
   （`crates/cli/tests/query.rs` 12 项，含**`query check` ≡ `--json` 计数**契约与
   **`query state` ≡ `soko/stateAt` 字段级**一致性，后者覆盖根状态 / tactic 之内 /
   tactic 之后 / **无 `by` 两个分支**，且**跑真实 LSP 二进制**）。
   ⚠️ 该一致性测试比较的是 `target/<profile>/sokonanoda-lsp`——**改了 front 却只跑
   `cargo test -p sokonanoda-cli` 时会拿旧二进制对拍**。这是特性（陈旧构件会当场暴露）
   也是坑（先 `cargo build --workspace` 或 `cargo test --workspace`）。
7. **刻意留在 LSP 的适配器规则**（是坐标/呈现适配，不是第二份真相；**别"顺手统一"掉**）：
   - `position_to_offset`：LSP 的 0-based、按字符计数的光标约定（真相层用字节
     offset，并另给 1-based UTF-16 的 `line_col_of`/`offset_of_line_col`）；
   - `render::decl_at`（**半开区间** `start <= p < end`）：hover 用；它由
     `hover_on_closing_bracket_never_shows_neighbor_signature` 钉死"光标在声明末尾
     不显示邻居签名"。而 `stateAt`/`hints` 的声明查找按协议是**闭区间**（含末尾，
     末行行尾的光标也算在声明内）。两者取值不同是**故意的**；
   - `range_of(Span)`：诊断 / hover / symbols / lens / folding / rename / references
     这些直接读原始 `DocumentReport` 的路径仍用它；`soko/*` 的 Range 全部走
     `query_map::range_of_offsets`（UTF-16 列，见 STATUS 第八十九轮第 7 条）。

### H6-B —— MCP 传输 + DSH 接线（P1）✅ 已完成
1. `dsh/mcp/server.js`：✅ MCP stdio（`initialize`/`tools/list`/`tools/call`），
   六工具 schema，全部转发 `scripts/soko query …`；零依赖、无 import（CJS/ESM 双兼容）。
2. `scripts/soko mcp` 转发入口 ✅（直接起 Node 脚本，不经二进制解析链 ——
   它本来就是仓库内的脚本；缺文件时非零 + 可行动错误）。
3. `dsh/cordis.patch.yml` 增 MCP 行 ✅（默认关闭，注释写清信任边界）；
   `dsh/README.md` 增"内核真相查询"一节 ✅（CLI 优先、MCP 可选、工具表、信任边界）。
4. 测试 ✅（`crates/cli/tests/dsh.rs` 的 `dsh_mcp_server_forwards_every_query_op`）。
   **实测验收**：DSH headless + `--patch`，模型调用 `mcp__sokonanoda__state`
   （`playground.sokonanoda:327:4`）拿到 `Exists Person P`。
5. 版本 bump：留到 H6-D 收尾统一做（本轮尚未发版）。

#### H6-B 实测踩到的 MCP 坑（写进 server 头注释与本节，避免后人重踩）

- **`server/discover` 探测**：DSH 的客户端在 `versionNegotiation: 'auto'` 下会先起一个
  **一次性兄弟进程**，只发 `server/discover`（2026-07-28 修订）**而不发 `initialize`**，
  然后 reap 它、再起真正服务的进程。**必须立刻用 JSON-RPC 错误（`-32601`）拒绝**：
  沉默会等满 SDK 的 60 s 超时（实测 60,058 ms vs 76 ms）；**绝不能回 `-32022`**
  （那会让它以为我们是现代版本而重试）。
- **必须 advertise `capabilities.tools`**，否则 DSH 一个工具都不注册、连 `tools/list` 都不调。
- 传输是**换行分隔 JSON**（无 `Content-Length`）；stdout 只能有 MCP 消息，日志走 stderr；
  用 `process.exitCode` 而不是 `process.exit()`（保证 stdout 刷净）。
- 模型可见的只有 `content[].text`（`structuredContent` 不进模型文本）→ 子进程的 JSON
  原样放进 `text` 是最省事且正确的做法。
- server 必须**无状态且可被起两次**（探测进程 + 服务进程，同 argv/env/cwd）：
  不写 PID 文件、不做单例假设、不依赖首消息状态。

### H6-C —— 两个 TODO（P2，可与 A/B 并行但**必须在发布前**）

> **根因已用发布版二进制实测锁定（2026-09-17）—— 并且推翻了本设计早先的三个猜测**。
> 下面每条都带 `file:line` 与可复制用例；实现时**照这里做，不要照直觉做**。

#### TODO A —— 触发条件是「**带索引 + 至少一个字段写在结果的箭头链里**」，与 `Prop` 无关

**先纠正两个曾写错的判断**（本文档早期版本与 `docs/HANDOVER.md` 都错了）：

- ❌ 不是"`small_elim`/`is_prop_block_ty` 判据不对"：`elab.rs:2499` 的
  `is_prop_block_ty(ty) && constructors.len() > 1` **在因果链之外**——索引 `Type`
  一样失败（下面矩阵第 3 行）。
- ❌ 不是"IH 形状不符"：IH 用的是**已会剥箭头的** `spine_of_codomain(field_ty)`
  （`elab.rs:2648-2650`），**IH 是对的**；错的是 **minor 结论里的索引实参**。

**实测矩阵**（发布版 0.55.0 二进制，Bare 上下文自带 `inductive Nat`）：

| 用例 | 结果 |
|---|---|
| `P : Nat -> Prop`，`ctor b (n : Nat) : P n -> P (Nat.succ n)`（**箭头写法**） | ❌ `kernel-rejected` |
| 同一形状改**具名** `ctor b (n : Nat) (h : P n) : P (Nat.succ n)` | ✅ `checked declaration P` |
| `W : Nat -> Type`，`ctor wb (n : Nat) : W n -> W (Nat.succ n)` | ❌（**与 Prop 无关**） |
| `Or2 (A : Prop) (B : Prop)`（**非索引**，箭头字段） | ✅（`num_indices = 0`，没有索引可丢） |
| `inductive T1 : Prop` + `ctor t1 : T1`（单构造子） | ❌ `recursor declares the wrong k-reduction flag`（另一个 bug，见下） |

**根因（一行代码）**：`derive_recursor` 用**只认 Ident/App 的** `src_spine` 去读
ctor 结果的索引实参——

```rust
// crates/front/src/compile/elab.rs:2613
let ctor_indices: Vec<Expr> = src_spine(&ctor.result)
```

而箭头写法下 `ctor.result` **就是** `P n -> P (Nat.succ n)`（箭头域被
`ctor_field_binders`，`elab.rs:2367-2371`，当作字段并进 `result_chain_binders`），
`src_spine` 对 `Arrow` 命中 `_ => None`（`elab.rs:1733`）→ `ctor_indices = []`
→ minor 结论退化成 `motive (C params fields)`，**索引实参被丢掉**。
失败点：内核 `assert_nonnested_recursors_def_eq` → `assert_def_eq(imported, new.ty)`
（`crates/kernel/src/inductive.rs:1706`，期望形状由 `mk_minors1group`
`inductive.rs:1418-1457` 的 `:1441-1442` 给出 `motive <ctor indices> (C …)`）。

**最小修法（front 一处，kernel 不动）**：`elab.rs:2613` 把
`src_spine(&ctor.result)` 换成 **`spine_of_codomain(&ctor.result)`**
（`elab.rs:2378-2387`，它已经会剥 `Arrow` 与 `Forall`），其余
（`.filter(head == name)` / `skip(params.len())` / `substitute_names(...)`）不动。
**不要**改 `small_elim` / binder 顺序 / motive / iota——它们已经符合内核契约。
验收依据：每个失败用例的**具名孪生体今天就通过**，两者只差 `ctor_indices`。

**同一个函数里顺带发现的第二个 bug（同一轮修掉）**：`elab.rs:471` 把
`is_k: false` **写死**，于是**单构造子 `Prop`**（如 `inductive True : Prop` /
`ctor trivial : True`）派生出的 recursor 被内核拒：
`recursor declares the wrong k-reduction flag (left: false, right: true)`
（`kernel/src/inductive.rs:661-662`，`init_k_target` 在 `:1268-1276`）。
判据应是"目标类型是 `Prop` 且只有一个构造子"（Lean 的 `K` 语义）——具体形状在
实现时按内核 `init_k_target` 对齐，**仍不动 kernel**。

#### TODO B —— parser 单名路径，修法明确

- `parse_inductive_block`（`parser.rs:357-395`）参数循环 **361-364** 与
  `parse_ctor`（`:397-413`）字段循环 **400-403** 都调**单名** `parse_binder`
  （`:1063-1109`，LParen 分支 `:1066-1079` 读完一个名字就要 `:`）。
- 组感知机制早就存在：`parse_binder_group`（`:723-753`）、
  `BinderGroup{names: Vec<String>, ty, style, span}`（`:23-28`）、
  **`push_binders`（`:1033-1048`，一个名字一个 `Binder`、共享组 span）**；
  `parse_arrow:659` / `parse_lambda:991` / `parse_forall:1013` /
  `parse_decl_binders:151` 都在用——**这就是"Pi 位能写 `(A B : Prop)`、inductive 不能"的原因**。
- **AST 与 elab/kernel 都不用改**：`Command::InductiveBlock.params: Vec<Binder>`
  （`ast.rs:235`）、`CtorDecl.binders: Vec<Binder>`（`ast.rs:259`）已是展开形态；
  `install_inductive_block` 取 `params: &[Binder]`（`elab.rs:230`）并据此算
  `num_params = params.len()`（`:238`）。
- **最小修法**：两处循环体换成 `self.push_binders(&mut params/binders)?`
  （照 `parse_decl_binders` `:148-160` 的写法，含 `(A)` 这种无类型组的显式报错）；
  每个名字得到自己的 `Binder`，类型/风格/span 共享组的值。
- **同类缺口清单（一次修完）**：① `parse_ctor` 字段（`:402`，真 bug，同修）；
  ② `parse_let`（`:480-518`）：`Expr::Let{binder}` 是**单个**，`let a b : T := v`
  连语法都不存在 → 需要 AST/脱糖决策（嵌套 let vs `Vec<Binder>`），**非 parser 替换**；
  ③ tactic `intro`（`:236-247`）只吃一个名字（Lean 的 `intro a b` 也会失败）→
  属于 by 引擎的独立特性；④ `axiom`（`:295+`）无 binder 望远镜，无需改；
  ⑤ `match` 模式（`:592-653`）已是"每个原子一个名字"（`| C a b =>` 可用），无缺口；
  ⑥ `rec`/`iota` 的类型是表达式，✅；⑦ 宇宙参数 `{u, v}` 已是多名字（`:332-355`）。
  **本轮只修 ①+inductive 参数**；②③单独立项（写进 `docs/HANDOVER.md` §3 E 备查）。

#### 实现与验收

1. **先写复现测试**（"修复前红"）：TODO B → `crates/front/src/parser.rs` 的
   `mod tests`（`:1222`，邻居 `:1449-1477`/`:1480-1492`）加参数组与 ctor 字段组、
   `{A B : Type}` 隐式组、`(A)` 仍报错；TODO A →
   `crates/front/src/compile/tests.rs` §带索引归纳（`:4537-4625`，目前只有具名的
   `INDEXED_VEC` `:4541-4544`）加**箭头写法**的 `Even`/`Le` 常量 + 用派生
   `Even.rec` 证一条 + `#reduce`（同时守住类型形状与 iota）。
2. CLI e2e：`crates/cli/tests/cli.rs` §带索引归纳（`:1521-1550`）加箭头写法孪生；
   §参数化归纳（`:1389+`）加组参数变体。
3. 修 TODO B → 修 TODO A（含 `is_k`）→ 课程简化：unit9 的 `Le`/`Even`
   去掉手写 `rec`/`iota`（`:119-138`、`:170-189` 及 EN 镜像与 2 份钥匙，
   见下），课程散文（`course/README.md:21`、unit9 的 v1 说明 `:112-115`）同步改写；
   `Or (A : Prop) (B : Prop)` 收敛成 `(A B : Prop)`（8 处文件）。
   **golden 复核**：`crates/cli/tests/course.rs:86-97`（unit9 `(13,8,0)`）、
   `course_status.rs:105-118`（78/59）——`rec`/`iota` 不产事件，计数**应当不变**；
   变了就说明改错。
4. 文档同步：`docs/architecture.md` §4.1 与参数化归纳段（`:131-135`）、
   `docs/design/indexed-inductives.md` §2/§3、`docs/HANDOVER.md` §3 E、`STATUS.md`。
5. 两者都**不碰 `crates/kernel/`**（冻结）：A 依赖内核既有的 recursor 契约，
   B 根本到不了内核。若发现必须改 kernel → **停下回设计**（范围变更）。
6. **保留**（教学用，不是 workaround）：unit6/7 手写 `Nat`/`Color` 消去子
   （非索引，派生本来就正常）。

#### H6-C as-built（实现后回填，2026-09-17）

- ✅ 两条都按上面的根因修完，课程 9 个文件简化、golden 计数不变（unit9 `(13,8,0)`、
  unit10 `(7,6,0)`、course 78/59），EN 与 CN 代码逐字节一致。
- ✅ 测试三层齐：front 单测 5 条（箭头字段派生 / 具名字段孪生 / **真 iota 归约** /
  单构造子 `Prop` / 多名字组 3 条在 `parser.rs`）+ CLI e2e 3 条；红先顺序保留。
- ⚠️ **`is_k` 的修复自己引出了一个回归，被 CLI e2e 层抓住**——这是"三层缺一不可"的
  实证：我第一版把内核判据近似成"单构造子 + 无索引 + 字段数 == 参数数"，而内核的
  判据是 `pi_telescope_size(ctor.ty) == local_params.len()`，ctor 的内核类型又是
  `forall (params ++ fields), result`，所以它等价于"**构造子没有自己的字段**"。
  两个判别性反例（都在内核那一侧被拒）：
  - `inductive Both (A B : Prop)` + `ctor mk (a : A) (b : B)`：字段数**恰好等于**
    参数数（2 = 2）→ 我的近似说 `is_k: true`，内核算 `false`；
  - `inductive Q : Nat -> Prop` + `ctor q : Q 0`：**有索引**但构造子无字段 →
    我的"有索引就不是 K 目标"说 `false`，内核算 `true`。
  现在的实现是逐字镜像（`is_prop_block_ty(ty) && ctor_field_binders(only_ctor).is_empty()`），
  两个反例各有一条单测（`single_constructor_prop_with_fields_is_not_a_k_target`、
  `single_constructor_indexed_prop_without_fields_is_a_k_target`）+ CLI e2e 里的
  `cli_inductive_accepts_multi_name_binder_groups`。
  **教训**：镜像内核的谓词时，不要写"看起来等价"的版本——把内核那行代码逐字翻译，
  并为**每个能让两个版本取不同值的输入**写一条测试（`docs/LESSONS.md` 同款方法）。
- ⚠️ `arrow_style_indexed_recursor_reduces` 第一版写成 `theorem pz_again : P 0 := pz`，
  名字承诺 iota 却没碰 recursor；现已改为 `Type` 值的箭头字段索引族 + `match` +
  `#reduce`（`Prop` 值多构造子族不允许消去到 `Type`，用它是**正确拒绝**，会误当回归）。

### H6-D —— 文档/门面收尾（P2）✅ 已完成（随实现同一轮）
1. ✅ `AGENTS.md` Setup 增 `query`（两个视图同一份真相 + 退出码 + MCP 六工具）；
   `skills/sokonanoda-teacher` 增"**先问，别扫**"；`skills/sokonanoda-dev` 增
   "**真相层不得绕过**"（新增语义必须进 `front::query`，适配器只做映射）。
2. ✅ `docs/protocol.md`（`query` 子命令一节）、`docs/TESTING.md`（真相层 + 一致性契约
   两行）、`docs/HANDOVER.md`、`ROADMAP.md` I15 as-built、`docs/LESSONS.md`
   （全输入对拍的工作方法）；`REQUIREMENTS.md` §9 追加。
3. ✅ 门面同步：VS Code `README/CHANGELOG/package.json`（扩展代码零改动；CHANGELOG
   记"内部重构 + 两处边界对齐"）+ `site/assets/agent-prompt.js` 一句；
   `dsh/README.md` 查询一节 + `dsh/cordis.patch.yml`。

### H6-E —— 远期（原 H5 其余项，不做承诺）⏳ backlog
- **B1 Infoview 客户端插件**：消费 `query goals/state` 在 DSH 里显示目标面板。
  卡点：DSH 的编辑器 UI 扩展点（webview/侧栏）与 out-of-tree npm 包 + 预构建 bundle
  的交付形态都还没勘察；且 DSH 的 LSP host 不投递诊断，插件要自己起 `query` 轮询。
- **B3 `SessionStart` 自动 provisioning**：本仓库无项目级 hook 自动发现
  （§9 第 1/2 条的同类限制），所以只能靠技能正文里的第一条命令（现状）或用户 profile
  插一行；要自动化必须先有 DSH 侧的发现机制。
- **B4 打包成 npm 插件**：把 `scripts/soko` 启动器 + `dsh/hooks/hooks.json` 的 Lean
  工具链 deny + `/sokonanoda-*` 命令合成一个 `@sokonanoda/dsh-plugin`，让用户装一个包
  就有全部接线（含 MCP 行）。需要先定发布账号/包名与版本对齐策略。

---

## 9. 已核实的 DSH 事实（原"待调研"，2026-09-17 关闭）

> 全部来自对 DSH checkout（`0d1f50007f`）的只读勘察 + 本会话实测；`path:line` 为证据。
> 设计结论已并入 §6.2/§6.3。**仍开放的**只有第 4 条（两个 TODO 的根因已另行实测锁定，
> 见 H6-C）。

1. **MCP client 配置**（`packages/mcp/mcp-client/src/index.ts:119-142`，接口 `:52-101`）：
   `transport`（`stdio`|`streamable-http`，**只有这两种**；无 SSE/WebSocket）、
   `serverName`（`/^[A-Za-z0-9_-]{1,32}$/`）、`command`/`args`/`env`/`cwd`（stdio）、
   `url`/`headers`（http）、`toolCallTimeoutMs`（默认 60000）、`failOnStartupError`
   （默认 false）、`maxInstructionBytes`（默认 32768）、`reconnect`
   （`enabled`/`initialDelayMs` 500/`maxDelayMs` 30000/`maxAttempts` 10）。
   工具名 `mcp__<serverName>__<rawName>`（`src/tools.ts:81-87`，超 64 字符加哈希）。
   资源工具来自**另一个插件** `@deepseek-ai/dsh-mcp-resources`
   （`mcp-resources/src/tools.ts:33-60`，base bundle 已挂一次，无需我们配）。
2. **启动/失败/超时**：eager 启动 + await ready（`index.ts:181,199`；stdio 先起一次性
   探测进程 `connection.ts:262`）；失败→警告+退避（`connection.ts:211-245`）；
   `failOnStartupError` 只让该行 inactive（`packages/boot/app-boot/README.md:43,80`，
   required id 列表 `app-boot/src/index.ts:711-718` 不含任何 MCP）。
3. **模型可见面**：`content[].text` 才是模型读到的（`tools.ts:461-511`；
   `structuredContent` 不进，`tools.ts:250-266` + `core/tools/src/index.ts:1802-1813`）；
   `description`/`inputSchema` 逐字无截断（`tools.ts:131-136`、`core/tools/src/index.ts:1261-1273`）；
   server instructions 成独立 system prompt 段（`connection.ts:318-319`、
   `server-context.ts:32-39`）；prompts 不支持、无 push（`mcp-client/README.md:12,209`）。
4. ~~**两个 TODO 的根因**~~ → **已实测锁定**（见 H6-C：`elab.rs:2613` 的 `src_spine`
   vs `spine_of_codomain`；`parser.rs:363/402` 的单名路径），并顺带发现 `elab.rs:471`
   的 `is_k` 写死 bug。证据方法：发布版二进制 + 孪生对照矩阵（具名/箭头、Prop/Type、
   索引/非索引、单构造子），全部可复现。

### 仍然开放的小问题（不阻塞 H6-A/B）

- `args` 里的相对路径是否相对 `cwd` 解析：勘察结论是"由 `cross-spawn`/Node 相对**子进程
  cwd** 解析"，但被标为**经验性、非文档契约**。→ **实现时规避**：`args` 用
  `!!js` 单行绝对路径（`path.join(process.env.SOKO_REPO ?? process.cwd(), 'dsh', 'mcp', 'server.js')`），
  与 `cwd` 双保险。
- `dsh-mcp-resources` 是否真能为我们的 server 暴露 resource：需要实测一次
  （设计里列为 H6-B 的可选项，不是必需）。

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
  `front/src/query/`（LSP 侧已无实现，只剩映射）。
- **A2（CLI 可用）**：`scripts/soko query check --file playground.sokonanoda` 输出单 JSON，
  含事件计数与每个开放练习的 `goals[]`/`holes[].id`；`query state --line 201 --col 9`
  给出该处目标与假设；`query holes` 在 playground 上返回全部洞且 `id` 唯一。
- **A3（DSH MCP）**：加 `--patch` 后 DSH 会话里模型能看到 `soko_*` 工具；
  调用 `soko_state` 返回与 CLI 逐字段一致的 JSON。
- **A4（一致性，防两套真相）**：契约测试断言
  `query state` ≡ `soko/stateAt`、`query goals` ≡ `soko/goals`（字段级），
  并且 `query check` 的计数 ≡ `--json` 事件流的计数。
  **as-built 加强**：`state` 的一致性必须逐个覆盖选择器的**判别性输入**
  （根状态 / tactic 之内 / tactic 之后 / 无 `by` 的开放与闭合），并跑**真实 LSP
  二进制**——只测"两边都不为空的常见路径"会漏掉 §4 as-built 3 那种分支分歧。
  **as-built（0.59.0）**：一致性还包含**失败口径**——同一份解析不了的文本，
  `query check` 的 `failed[]` 与 `grade --json` 的 parse 诊断同 code/同 span、
  退出码同为 1（`crates/cli/tests/query.rs` 的
  `query_check_reports_parse_errors_with_exit_one` /
  `query_check_matches_grade_on_a_real_course_unit`）。
- **A5（结构债）**：`crates/lsp/src/lib.rs` **≤1200 行且不再有查询/渲染的第二份实现**
  （`rg` 断言：`select_state_at`/`runs_of`/`decl_name` 等语义函数只存在于
  `front::query`）；`crates/front/src/query*.rs` ≤500 行/文件；`cargo clippy`
  教学 crates 零 warning（`[lints] deny` 不变）。
  **as-built：两项都达标** —— `lib.rs` **4256 → 3988（删重复）→ 1105 行**。
  中途我曾写下"放弃 ≤1200 行、只按重复度验收"，那是**没量就改标准**：`lib.rs` 的
  3988 行里 2638 行是测试模块，移出测试 + 抽出 `protocol.rs`/`tokens.rs` 后
  ≤1200 随手可达。正确做法是先 `wc -l` 量构成，再决定改标准还是改代码
  （`docs/LESSONS.md`；结构债由"A5 两项"共同定义：**无重复** + **单文件不越红线**）。
- **A6（两个 TODO）**：`Le`/`Even` 省略 `rec` 时自动派生通过内核（课程改为依赖自动派生，
  golden 同步）；`inductive Foo (A B : Prop)` 解析通过并有三层测试；
  两个修复各有"修复前红"的复现测试（输入见 §5 H6-C 的两段可复制用例）。
- **A7（回归）**：`cargo test --workspace --locked` 全绿；`scripts/soko gate` PASS；
  既有 `--json`/`soko/*` 契约测试**不改判据**（只加，不改）。

---

## 12. 风险

| 风险 | 影响 | 缓解 |
|---|---|---|
| 抽层时**语义漂移**（LSP 原行为与 `front::query` 不一致） | 编辑器与 agent 看到不同结论 | A4 的字段级一致性契约测试是硬门禁；先平移后删除，绝不同时改语义。**⚠️ 这个风险真的发生了**：no-`by` 分支在抽层时被判成"根状态"，而**当时没有任何测试会红**（§4 as-built 3）。补救不是"更小心"，而是把一致性测试扩到判别性输入 + 穷举对拍的工作方法（删除旧实现前，先在新旧两份实现上做全 offset 对拍） |
| 两套真相（有人在 LSP 里"顺手"补逻辑） | 违反硬规则、长期不可维护 | A1 的 `rg` 断言收进 `crates/cli/tests/query.rs`；dev 技能写明"真相层不得绕过" |
| `--json` 消费者被打断 | opencode/VSIX/CI 全红 | 事件流契约**只增不改**；`check` 是新增视图而非替换 |
| MCP server 逃出沙箱/泄露凭据 | 安全事故 | 默认关闭 + 文档写明"信任边界"；server 只 spawn 仓库内二进制、不读凭据 |
| 两个 TODO 修复牵动课程 golden | 大面积测试红 | 按 TDD 三层逐条修；golden 变更在同一轮一次性同步并记录理由 |
| `crates/lsp` 重构引入回归 | 编辑器体验倒退 | 现有 LSP 测试（`soko/*`、hover、inlay、code action）**保持全绿且不修改**，作为重构的安全网 |

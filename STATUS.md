# 当前状态与进度日志（agents 先读这里）

> 快照：2026-09-17（第八十九轮：内核真相查询通道落地 H6-A/B/C + 门面收尾；版本 **0.56.0**）
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

## 本轮进度（2026-09-17，第八十九轮：内核真相查询通道落地 —— H6-A/H6-B/H6-C + 门面收尾）

> 续第八十八轮的设计（`docs/design/agent-query-channel.md`，ROADMAP **I15**）。按
> **H6-A → H6-B → H6-C** 逐项实现并测试，收尾做 H6-D 文档/门面同步；版本
> **0.55.0 → 0.56.0**（agent 可见的新能力 `query`，QD-7）。

1. **H6-A 真相层 + CLI（`front::query` + `sokonanoda query <op>`）**：
   `crates/front/src/query/{mod.rs,types.rs,tests.rs}` = **编辑器无关的唯一真相**
   （`QueryDoc` + `check`/`state`/`goals`/`holes`/`hints`/`reduce`）；
   `QueryError{NotParsable,OutsideDeclarations,PositionOutOfRange}` 把"正常的没有"
   与"问不出来"分开（各带稳定 code + 中文 message）。`crates/cli/src/query.rs`
   输出**单 JSON 对象**（`{schema:"soko.query/1", op, version, ok, data|error}`），
   退出码 = **0 答上了（含 `ok:false` 与开放 `sorry`）/ 1 内核拒绝 / 2 用法**；
   `--text` 支持未落盘中间态。契约写进 `docs/protocol.md`。
2. **LSP 改为调用真相层 + 结构债清零（同一轮完成，A1/A5）**：
   `soko/goals`/`stateAt`/`nextHole`/`hints` 与 hover 的 tactic 视图全部改为调
   `front::query`；新增 `crates/lsp/src/query_map.rs`（**唯一的形状映射点**：
   offset↔`Range`/`Position`、`QueryError`→既有空结果），删除 `select_state_at`/
   `StateSelection`/`runs_of`/`status_str`/重复的 `decl_name`/`goal_decls` 的 75 行
   主体等 → `crates/lsp/src/lib.rs` **4256 → 3988 行**。再按模块化硬规则把两个测试
   模块移出文件（`tests.rs` 2567 / `by_sorry_range_tests.rs` 60，**断言一字未改**，
   214+6 条 assert 与 HEAD 逐行等价）并抽出 `protocol.rs`（wire 类型，159）与
   `tokens.rs`（semantic token 辅助，107）→ **lib.rs 1105 行，≤1200 达标**。
   ⚠️ **过程留档（我自己的错）**：删完重复后我曾**没量就**把"≤1200 行"作废，
   理由是"剩下的都是协议服务代码"——`wc -l` 显示 3988 行里 **2638 行是
   `#[cfg(test)]` 模块**，非测试代码只有 ~1350 行，移出测试随手就达标。教训
   （**改验收标准之前先把被验收的东西量一遍**）进 `docs/LESSONS.md`；最终口径 =
   "**无重复实现**" **且** "**单文件 ≤1200 行**"（设计文档 §2.6/§3.2/§11 A5 已改）。
3. **⚠️ 抽层真的出过一次语义漂移（本轮最重要的教训，已进 `docs/LESSONS.md`）**：
   LSP 侧 117/117 全绿的情况下，**没有 `by` 块**的声明被错误地统一成"根状态"
   （已证声明凭空多出一个目标、半成品证明 `fun (a) (h) => sorry` 丢掉已引入的假设）。
   抓出它的不是测试而是**穷举对拍**：删除旧实现前，在 5 个画布的**每一个光标
   offset**（0..=len）上比较新旧两份实现，**709 次比较 / 424 处不一致全落在这一个
   分支**。既有测试没红是因为 LSP 唯一覆盖它的用例，画布**没有 lambda 前缀**，
   "剩余目标"恰好等于声明类型——**"新测试通过"不等于"新语义被测试"**。
   修法：`by_steps.is_empty()` 单独走"声明级目标 + 上下文"（协议 `docs/protocol.md`
   原文），红先单测 2 条（开/闭两分支）钉死，并把 CLI≡LSP 一致性契约扩到这两个
   **判别性输入**。教训同时写进设计文档 §4 as-built 3 / §12 风险表。
4. **H6-B MCP 传输 + DSH 接线**：`dsh/mcp/server.js`（零依赖 stdio 桥，
   `initialize`/`tools/list`/`tools/call`，六工具全部转发 `scripts/soko query …`；
   `server/discover` **立刻**用 `-32601` 拒绝——沉默会等满 SDK 的 60 s 超时）、
   `scripts/soko mcp`、`dsh/cordis.patch.yml` 的 `mcp-sokonanoda` 行（**默认关闭**，
   注释写清信任边界：MCP server 是 DSH 沙箱外的可信代码）。
   五个实测坑写进设计文档 §H6-B（探测进程/`capabilities.tools`/换行分隔 JSON/
   只有 `content[].text` 进模型/必须无状态可重启）。**实测验收**：DSH headless
   会话里模型调用 `mcp__sokonanoda__state` 拿到目标。
5. **H6-C 两个 front 缺口修掉**（原 `docs/HANDOVER.md` §3 E）：
   ① `derive_recursor` 在"**带索引 + 字段写在结果箭头链里**"时用只认 Ident/App 的
   `src_spine` 读索引实参 → 改为已会剥箭头的 `spine_of_codomain`（`elab.rs`），
   并顺带修掉写死的 `is_k: false`（单构造子 `Prop` 归纳因此被内核拒）；
   ② `inductive` 参数/ctor 字段不吃多名字 binder 组 `(A B : Prop)` → 解析器改调
   组感知的 `push_binders`（AST/elab 未动）。**课程随之简化**：unit9/unit10 的
   `Le`/`Even` 不再手写 `rec`/`iota`、`Or (A : Prop) (B : Prop)` 收成 `(A B : Prop)`，
   中英代码逐字节一致、**golden 事件计数不变**（unit9 `(13,8,0)`、unit10 `(7,6,0)`）。
   两条修复都先有"修复前红"的复现测试（`indexed_inductive_with_arrow_style_field_derives_recursor`
   等 4 条）；CLI e2e 三条 + 解析器三条补齐三层。
   **⚠️ 追加发现（"三层缺一不可"的实证）**：`is_k` 的第一版把它近似成"单构造子 +
   无索引 + 字段数 == 参数数"，front 单测全绿，但**内核拒了两个判别性形状**——
   `Both (A B : Prop)` + `mk (a : A) (b : B)`（字段数恰好等于参数数 → 内核要
   `is_k: false`），以及反向的 `Q : Nat -> Prop` + `q : Q 0`（**有索引但无字段 →
   内核要 `is_k: true`**）。抓出它的是新加的 CLI e2e 层。现按内核
   `init_k_target` 逐字镜像（`is_prop_block_ty(ty) && ctor_field_binders(only_ctor).is_empty()`），
   两个反例各留一条单测；方法（镜像内核谓词 = 逐字翻译 + 给判别性输入写测试）进
   `docs/LESSONS.md`。另外首版 `arrow_style_indexed_recursor_reduces` 名字承诺 iota
   却没碰 recursor（`theorem pz_again : P 0 := pz`），已改为 `Type` 值索引族 +
   `match` + `#reduce`。
6. **一致性契约（A4，防两套真相）**：`crates/cli/tests/query.rs` 12 项，其中
   `query_check_counts_match_the_json_event_stream` 钉"同一份判卷两个视图"，新
   `query_state_agrees_with_the_lsp_state_at_request` / `query_state_and_lsp_agree_without_a_by_block`
   **起真实 `sokonanoda-lsp` 二进制**做字段级对拍（根状态 / tactic 之内 / tactic 之后 /
   无 `by` 的开放与闭合）。注意：它比对的 `target/<profile>/sokonanoda-lsp` 可能是旧
   构件——**改了 front 只跑单 crate 测试会拿旧二进制对拍**（先 `cargo build --workspace`），
   这是特性也是坑，已写进设计与教训台账。
7. **两处刻意的 wire 边界对齐**（此前无测试覆盖，已记录）：`soko/hints` 的声明命中
   与 `stateAt` 统一为**含末尾**（旧路径开区间：光标恰在声明末偏移/末行行尾之后返回
   `[]`，现在返回阶梯）；由 offset 换算的 `Range` 改用**UTF-16** 列（LSP 规范口径，
   与其它响应一致；BMP 文本逐字节相同，仅增补平面字符不同）。扩展侧无需改动。
8. **H6-D 同步**：`AGENTS.md` Setup（`query` 两视图 + 六个 MCP 工具）、
   `skills/sokonanoda-teacher`（"先问，别扫"）、`skills/sokonanoda-dev`（"真相层不得
   绕过"）、`dsh/README.md`（查询一节 + 信任边界）、`docs/protocol.md`、
   `docs/TESTING.md`、`docs/HANDOVER.md`、`ROADMAP.md` I15 as-built、VS Code
   README/CHANGELOG/`package.json` 版本同步、`site/` agent prompt 一句。
9. **H6-E backlog（不做承诺）**：DSH Infoview 客户端插件（消费 `query goals/state`）、
   `SessionStart` 自动 provisioning、把启动器 + Lean 工具链 deny 拦截 + `/sokonanoda-*`
   命令打成一个 npm 插件包。
10. **发布**：CI 全绿 → auto-tag `v0.56.0` → release **11 个 job 全 success**
    （8 平台 build + VSIX + **marketplace 发布一次成功** + GitHub Release），
    26 个产物；并**用发布产物实测**（下载 CLI：`query state` 与仓库一致；下载 LSP：
    无 `by` 的开放声明 `goal='a' binders=['a','h']`、已闭合 `goal=None`）。
11. **本轮产物**：`crates/front/src/query/*`、`crates/cli/src/query.rs`、
    `crates/cli/tests/query.rs`、`crates/lsp/src/query_map.rs` + `protocol.rs` +
    `tokens.rs` + `tests.rs`/`by_sorry_range_tests.rs`（lib/hints/render 收敛）、
    `dsh/mcp/server.js`、`dsh/cordis.patch.yml`、`scripts/soko`（`mcp` 分支）、
    `crates/front/src/parser.rs` + `compile/elab.rs`（H6-C）、课程 9 个文件简化、
    文档/门面同步（见第 8 条），版本 0.56.0。

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

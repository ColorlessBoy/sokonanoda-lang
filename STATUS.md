# 当前状态与进度日志（agents 先读这里）

> 快照：2026-09-17（第八十七轮：DeepSeek Harness 适配落地；0.55.0）
> 仓库：`sokonanoda-lang`；权威计划 = `ROADMAP.md`；**用户要求总账 = `REQUIREMENTS.md`（先读）**；
> **文档地图 = `docs/README.md`**（入口/权威在仓库根，开发者参考在 `docs/` 顶层，
> 设计在 `docs/design/`，调研笔记在 `docs/notes/`）；
> 架构/内核 = `docs/architecture.md`；协议 = `docs/protocol.md`；测试地图 = `docs/TESTING.md`；
> 经验台账 = `docs/LESSONS.md`；CI 失败台账 = `docs/CI-FAILURES.md`；发布 = `docs/RELEASE.md`；
> agent 入口 = `AGENTS.md` + `skills/`；**harness 适配 = `docs/design/deepseek-harness.md`**。

## 一句话

`.sokonanoda` = **纯声明式教学文件（无 `#` 命令）+ 完整 sokonanoda 内核 + LSP 反馈通道**。
练习 = 带 `sorry` 洞的 `def name : T` / `theorem name : T` / `example : T` 声明。
CLI/REPL 的 `#check` 等只是调试/自测工具，不是文件格式。

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

## 本轮进度（2026-09-16，第八十四轮：课程大纲重构 P3——锁定 10 单元）

> 续 `docs/design/course-syllabus.md` §6 P3：补齐锁定的最后两个单元。

1. **#9 关系与联结词**（`unit9-relations-connectives`）：把 `Or` 作为**真实归纳**声明
   （自动派生 `Or.rec`，`match` 降低到它）教「用」它；`Iff` 作为**定义**
   `And (A->B) (B->A)` 练定义展开；`Le`/`Even` 作为归纳关系 + 消去/归纳引理
   （PLFA inversion 套路）。8 题（T/R/L/X）。
2. **#10 读证明与综合**（`unit10-reading-proofs`）：自解释三问（Hodges/Alcock/Inglis）
   逐行读一份已证证明；formal↔informal 互译；两题「评阅错证明→写出能过内核的修正版」
   （错证明只放注释）；一题跨单元 capstone。6 题（X/R/T）。
   **判分口径**：不引入新协议事件——每道读/评阅题都要产出内核可判的声明（散文只在注释里、
   不计数）。
3. **规模**：unit9 `(13 checked, 8 open, 0 reduced)`、unit10 `(7,6,0)`；汇总
   **units=10 checked=78 open=59 failed=0**；`course_json_lists_the_ten_units_in_order`；
   `cli.rs` 缓存金值同步。
4. **门面同步**（硬规则）：teaching-session（新增「第三课：关系、联结词与读证明」键表）、
   course-status/course-bilingual/ROADMAP I7（改为「✅ 完成（锁定 10 单元）」）/
   course/README/infrastructure + teacher skills（curriculum 行 9/10、SKILL 的
   `Or`/`Iff` 归属）+ editor/vscode/README（10 单元）。
5. **记录两个产品缺口**（写入 HANDOVER §3 E + 课程大纲 §2 第 11/12 条）：
   (a) **带索引的递归 `Prop` 归纳**（`Le`/`Even`）自动派生 recursor 被内核拒（IH 形状不符），
   只能手写 `rec`/`iota`（`Or` 非索引 Prop、`Vec` 带索引 Type 均正常）——front 未冻结，**可修**；
   (b) `inductive` 的**多名字参数组** `(A B : Prop)` 不解析（Pi binder 支持）。
6. **验收**：course/course_status/skill + 全量 CLI + 手动 fmt/clippy/test 全绿；20 个 CN+EN
   画布 exit 0；双语画布与 solutions 逐项相等；版本 0.53.0 → **0.54.0**（课程达到锁定规模，minor）。

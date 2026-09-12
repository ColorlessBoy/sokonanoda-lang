# 项目要求总账（REQUIREMENTS）

> 这是用户全部要求的**权威记录**。任何 agent 接手任何任务前先读本文，
> 再读 `ROADMAP.md`（里程碑）与 `STATUS.md`（当前进度）。
> 新要求出现时追加到本文末尾并注明日期；冲突时以本文为准。

## 1. 产品愿景（不变）

用户与 code agent 共同看着同一个 `*.sokonanoda` 文件（共享画布）：
agent 从零讲课、出题；用户作答；我们自己的编译器实时给出反馈，
同一份反馈既给用户也给 agent。反馈通道以 LSP 为中心（LSP-first）。

## 2. 不可动摇的硬规则（来自 ROADMAP §1，长期有效）

1. **kernel 保持完整**，不因教学裁剪；
2. **无官方 Lean 工具依赖**（lean/lake/lean4export/leanc/elan 一律不调用）；
3. **教学语法是真实 Lean 4 的子集**，填完洞的声明放进官方 Lean 依然合法；
4. **语法白名单即课程**：parser 只认课程引入过的语法点；
5. **分层推进**：L0 编译器 → L1 服务 → L2 编辑器 → L3 agent 协作；
6. **TDD 与重复测试**：front 单测 + CLI 端到端 + 课程语料三层覆盖；
7. **反馈即功能**：类型/化简/打印/错误都结构化输出，人与模型都能无文档驱动；
8. **判定永远走 kernel**，不做文本比对（`proof.rs::assumption` 的文本比对是待替换草案）；
9. **用户/agent 使用路径零工具链依赖**（2026-09-10 用户明确）：获取与运行只
   依赖 GitHub Release 资产（`sokonanoda-cli-*.tar.gz` / `sokonanoda-lsp-*.tar.gz`）
   或平台 VSIX 插件，**不要求 Rust/cargo**；cargo 仅贡献者开发需要。面向
   用户/agent 的文档、技能与错误文案不得把 cargo 当使用前提。

## 3. 内核性能是产品优势（2026-09-06，用户要求）

- sokonanoda 内核的快（arena + hash-consing + 闭包求值，相对官方内核 10–100x）
  是本项目立身之本：**任何重构/新功能都不得触碰 `crates/kernel` 热路径**；
- 前端只消费内核公开 API；教学功能（prelude、elaborator）不得给内核加间接层；
- 守护手段：保留内核全量测试 + 新增前端 perf 冒烟测试（大数字原生归约、
  iota 深归约链），CI 跑通，防止无意中把快路径弄慢。

## 4. 工程标准：模块化、单文件不许越长越大（2026-09-06，用户要求）

- **现状不合格**：`front/lib.rs` ~1200 行、`front/compile.rs` ~2100 行，单文件巨石难维护；
- **目标结构**（公开 API 用 re-export 保持稳定，调用方不改）：
  - `crates/front/src/`：`span.rs` / `token.rs` / `ast.rs` / `diagnostic.rs` /
    `parser.rs` / `compile/{mod,error,event,report,elab,prelude,check}.rs` / `proof.rs`；
  - `crates/cli/src/`：`main.rs`（入口与参数）+ `check.rs` / `json_report.rs` /
    `repl.rs` / `help.rs`；
  - `crates/lsp/src/`：`main.rs` + `backend.rs` / `render.rs` / `actions.rs`；
  - **`crates/kernel` 保持上游快照原样**（只允许此前已注明的适配清单，见
    `docs/architecture.md` §6）；
- 新代码一律按此结构放；任何文件接近 ~500 行即考虑拆分；
- 每个模块开头一段 doc comment 说明职责；交接靠文档不靠读巨石文件。

## 5. prelude 可选：既能全加载，也能完全裸跑（2026-09-06，用户要求）

- 教学需要两种模式：**Full**（内置 Nat/Eq 等受信任基元）与 **Bare**（完全不加载
  任何 prelude，学生/课程从零构造一切，例如用显式 `inductive Nat` 块）；
- API：`compile_fol/check_document` 接受编译选项（`PreludeMode::{Full, Bare}`）；
- CLI：`--bare` 旗标（未来可扩展 `--prelude <file>` 自定义基元文件）；
- LSP：文件级注释指令 + 工作区设置（默认 Full）；
- Bare 模式下文件必须仍能通过完整内核检查（例如纯逻辑公理练习）；
- prelude 内容本身用 `.sokonanoda` 源语法书写再安装（签名与官方 Lean 一致，
  与真实 Lean 兼容；Eq/Eq.refl/Eq.subst 为 I6 第一批）。

## 6. 教学工作流（2026-09-06 确认并执行中）

- **课程层与执行层的关系（2026-09-07 用户明确）**：`course/` 只是**给大模型的
  路线图/素材库**（知识点、顺序、题池、可解性守护）；**具体执行层必须由大模型
  按每个用户灵活适配**——根据用户的错误历史、节奏、兴趣实时调整出题与讲解，
  动态画布（如 playground.sokonanoda）才是用户真正面对的表面。禁止把课程文件
  当成"用户直接消费的固定课程"。
- 画布 = 仓库根 `playground.sokonanoda`（纯声明式，无 `#` 命令，`--` 中文讲解）；
- agent（本会话的我）即老师：写定义/出题 → 用户作答 → agent 用 Release 的
  `sokonanoda` 二进制跑 `"$SOKO" --json playground.sokonanoda`（零 cargo；
  仅贡献者可用 `cargo run -q -p sokonanoda-cli --bin sokonanoda --` 等价形式）
  读结构化事件（decl.checked / exercise.open / diagnostic+code+hint）决定下一步；
- 第一课内容：①表达式与类型 ②函数与箭头 ③命题与证明项 ④等式与 rfl；
  练习判定只走 kernel；`???` 洞（含 lambda 体内的部分作答）是合法状态；
- 课程细节与解答钥匙见 `docs/teaching-session.md`。
- **课程排序哲学（2026-09-07 用户插话修正，优先级高于既有单元顺序）**：
  不要一上来教 Prop / Sort / Type——完全不直觉、没有吸引力。正确顺序是
  **逻辑先行**：先直接讲逻辑连接词与量词 Prop True False And Or Iff Forall
  Exists，让学生先"证明命题"；等到函数与函数类型出场、学生自然会问
  "函数类型的类型是什么？"——由这个**自然触发的问题**引入 Sort。
  教学顺序跟着直觉与问题触发走，而不是跟着类型论自身的知识结构走；
  产品设计同理：好的设计应该不言自明。course/ 单元顺序与 playground
  需按此重排。

## 7. 开发过程要求（2026-09-06，用户要求）

- **多用 subagent**：探索/调研/机械重构/文档起草都派 subagent 并行做；
  主会话专注核心设计与编码；产出后主会话验证（编译+测试）；
  并行任务书必须**文件集互斥**（写死允许修改清单+验收命令，公共文件的
  接线点由主会话先接好）；subagent 失败/断网 ≠ 工作丢失——先核实树状态
  与测试再决定收尾或重跑（细则见 docs/LESSONS.md 工程流程节）；
- **持续头脑风暴**：新功能先出设计方案（写进 docs），再动手；
- **文档先行、交接友好**：`REQUIREMENTS.md`（本文）、`STATUS.md`（进度日志）、
  `docs/architecture.md`（架构事实）、`docs/notes/lsp-notes.md` / `docs/notes/vscode-notes.md`
  （外部标准调研）、`docs/teaching-session.md`（教学循环）、`docs/protocol.md`（事件协议）；
  每轮进度落 commit 前先更新 STATUS；
- LSP/VS Code 集成遵循业界标准做法（tower-lsp[-server]、vscode-languageclient、
  版本化诊断、FULL sync、自定义请求承载 goal 视图——见两份 notes 文档）。

## 8. 路线对齐

- 当前执行：I6（Eq prelude + binder 推断 + partial hole + prelude 可选化）
  → playground 开课 → I7 第一门课（course/ + golden）→ I8 增量 → I9 goal 视图
  → L2/L3（编辑器打包、service 事件流、讲课 agent 深化）。
- 详细验收标准以 `ROADMAP.md` §10 为准。

## 9. 要求追加日志

- 2026-09-06（会话 1）：接手项目，先 I6 后推进至终极形态；用户要尽快在
  playground.sokonanoda 开课，agent 当老师。
- 2026-09-06（会话 1 续）：多用 subagent；多写 docs 方便交接。
- 2026-09-06（会话 1 再续）：持续调研 Rust LSP/VS Code 标准实践；
- 2026-09-06（会话 1 补）：代码必须模块化、专业化（不得单文件巨石）；
  prelude 必须可选（Full / 完全 Bare）；多用 subagent；保持内核性能优势；
  本文档即为"记住所有要求"的机制。
- 2026-09-07：**充分测试是大规模合作与多次大规模重构的重要资产**——每个环节
  （lex/parse/elab/kernel/事件/CLI/JSON 协议/LSP/语料/文档一致性/perf）都要有
  自动化测试守护；落实为 157 个测试 + `docs/TESTING.md` 测试地图（本轮完成）。
- 2026-09-07（续）：I6 落地（prelude 可选 Full/Bare + `--bare` + 注释指令；
  Eq 三件套 prelude；binder 类型推断；partial hole）→ 178 个测试全绿；
  `playground.sokonanoda` 开课（12 练习 + 教学循环文档）。
- 2026-09-07（再续）：**课程层只是给大模型提供路线图，具体执行层需要大模型
  适配各个用户、灵活调整**——course/ 是 agent 素材库，不是用户直接消费的
  固定课程；I9 goal 视图第一段（假设列表 hover + exact/intro code action）。
- 2026-09-07（四）：**LSP 增加代码高亮（semantic tokens），VS Code 插件支持
  代码高亮**——已落地；随后不再询问、全部做好测试并推送 git（用户指令）。
  I8（check-then-add/Session/watch）与 I9 kernel 显式错误同轮落地。
- 2026-09-07（五）：**课程排序哲学**（§6 课程排序哲学条目）：逻辑先行，
  Sort 由"函数类型的类型是什么"自然引出；course/ 与 playground 重排为下一轮
  任务。同轮完成：I8 真增量（TrustPlan + 快照复用 + span 重映射 + LSP 接入
  Session）、I9 judge（合成声明 kernel 裁决；exact/assumption 文本比对删除；
  soko/goals 与 soko/nextHole 协议）、内核 conv 快路径 soundness 修复
  （eval/infer 闭包混用误判，三层回归测试）、CI lint 门禁（教学 crates
  [lints] deny + kernel 冻结豁免）与 VS Code 打包 P0 修复。
- 2026-09-07（六）：**为 code agent 设计 skill 部分**：`skills/` 目录承载
  `sokonanoda-teacher`（教学循环/判卷决策表/出题规范/钥匙守则）与
  `sokonanoda-dev`（接手清单/硬规则/TDD 工作流），Agent Skill 格式
  （SKILL.md frontmatter），`crates/cli/tests/skill.rs` conformance 守护
  （frontmatter、引用路径存在、事件/方法词汇封闭且与 protocol.md 一致）。
  同轮：仓库根 `opencode.json` 把 sokonanoda-lsp 挂到 .sokonanoda 扩展名
  （opencode 等 agent 自动消费 kernel 判定诊断），根 .gitignore 收编
  node 工具产物。
- 2026-09-07（七）：**逻辑先行课程重排落地**（§6 课程排序哲学的执行）：
  course/ 5 单元更名重写（①命题与证明 → ②等式与 rfl → ③函数与箭头 →
  ④宇宙 → ⑤归纳），playground 同步为 12 题逻辑先行版本；Sort 延迟到
  "函数类型的类型"悬念揭晓。同轮：I9 余项——开放声明携带宇宙参数
  （DeclState.universe，带 {u} 练习获得 exact 建议）。
- 2026-09-07（八）：**subagent 并行开发 + 持续调研/头脑风暴**（用户指令）：
  本轮 3 个 subagent（内核错误分类学审计 / 发布流水线实现 / 业内标准差距
  审计），主会话实现多洞+refine（I9 第二段）。修复两个关键缺陷：
  #check/#reduce 无 panic 保护（可崩掉 LSP 进程）、.vscodeignore 排除
  node_modules（VSIX 仍坏）。产出：docs/design/goal-refine.md、
  docs/notes/gap-analysis.md、docs/RELEASE.md、8 个新 kernel 错误码。- 2026-09-07（九）：gap-analysis 第一批落地（业内标准补全）：completions、
  folding、--version+MSRV（主会话）；go-to-definition/document highlight/
  binder 补全（subagent）；REPL undo（subagent）。- 2026-09-07（十）：**教学文案文风硬约束（用户指令）**：拒绝翻译腔、
  AI 味、抖音味、小红书味；规范落为 skills/sokonanoda-teacher/references/
  zh-style.md（提炼 humanizer-zh/de-ai-writing/stop-slop 与维基 AI 写作
  特征清单），SKILL.md 出题规范强制引用；学习者指出的
  "被居住/被证成"翻译腔为第一案例。- 2026-09-07（十二）：**sorry → warning 诊断分级**（Lean 4 对齐）：含 sorry
  的声明产出 WARNING 级（code `sorry`），与 kernel-rejected ERROR 分离；
  VS Code 集成测试 4 用例落地（@vscode/test-electron）；Lean 4 调研确认
  点分名原子/sorry warning/hover 签名与官方对齐。
- 2026-09-09（十三）：**括号 hover 与真名还原（用户指令）**：hover 内容
  曾完全混乱，要求——(1) `(表达式)` 的 `(` 与 `)` 都显示
  `表达式 : 表达式的类型`；(2) 不得显示 `$2` 这类 de Bruijn 索引，必须
  还原为对应 binder 名字；(3) 为括号 hover 设计多个测试样例，指定语料
  `and_not_absurd`（`(And.right a (Not a) h)` → `And.right a (Not a) h :
  Not a`；`And.left a (Not a) h` → `a`）。实现见
`docs/design/hover-brackets.md`（kernel pp 播种 + 括号组匹配 +
   `Not a` 保持折叠），测试 front 2 + LSP 5 + 旧断言对齐。
- 2026-09-09（十四）：**课程双语化（用户指令）**：tutorial/教程文档要有中文
  与英文两种版本——范围 = `course/` 单元课程为主，形态 = 中文/英文各一份
  独立文件。落地：`course/en/` 英文镜像（5 单元画布 + `solutions/` 解答钥匙），
  `course.json` 增 `title_en`，`course/README.md` 补双语布局说明，CI 新增
  `en_mirrors_match_chinese_event_counts` 守卫（事件计数逐项相等 + 英文钥匙
  0 诊断 0 洞），设计见 `docs/design/course-bilingual.md`。**英文注释按语义
  重构、不按字节翻译**（用户原则 2026-09-09 修订）：英文是重新写就的自然
  教学文案，重组句子与段落、不以中文行号/行数为准；但代码与中文逐字节
一致、知识点与提示阶梯条数/顺序同构。不改 `course/` 中文文件与 docs/
   开发者文档。
- 2026-09-09（十五）：**hover 重构——良构表达式 + 高亮范围（用户指令）**：
   括号 hover 内容仍乱（「有些是包含括号的外部表达式」「`(Not a)` 与
   `(And.right a (Not a) h)` 左右括号对不上」），要求——(1) 逐字符评估
   `*.sokonanoda` 文件里所有 hover 内容，指出不合理处；(2) 把正确行为设计成
   单元测试再开发；(3) **最终要知道 hover 内容对应的表达式范围**——hover 返回
   range，编辑器高亮该表达式。根因三连：lambda/Pi 的 binder 名整段溢出、
   括号组切片截断（AST span 不含括号）、hover 不返回 range。决议：front 为
   每个 binder 记声明行（`binder: true`）、LSP 切片括号平衡成良构表达式、
   所有 hover 分支带高亮 range。实现见 `docs/design/hover-refactor.md`，
   测试 front 2 + LSP 5 + 旧断言对齐。
- 2026-09-09（十六）：**by-tactic 块（用户指令）**：实现基础 tactic，与 Lean 4
  一样用 `by` 开始；补充 assumption / rfl。首期五个：**intro / exact / apply /
assumption / rfl**，另加 `by sorry` 占位（目标保持开放，与值位 sorry 同
   语义）。语法 `theorem t : T := by <tactic>; <tactic>; …`；逐 tactic
   判定永远走 kernel（复用 `judge_terms` 合成声明 + 新 `judge_infer` 推断被应用
   函数类型）；`by` 没写完整 / 尾部 `sorry` = 合法 Open 状态。`apply` 只做位置
   spine 合一，
   不支持需要高阶合一的形状（如 `apply And.left` 时类型参数未定）。三件套：
   新增 `course/` 单元⑥（中文 + en 镜像 + 解答钥匙）与 playground 2 道 by 题
   （练习 13/14，`:= by sorry`）；
   白名单 = 解析器只认这五个 tactic + sorry。同时要求**调研并设计 VSCode 前端显示
   goal state**——设计见 `docs/design/by-tactics.md` §6（front `by_steps` +
   `soko/stateAt` + 练习树「当前光标处」goal 组），实现为 Phase 2。
- 2026-09-10（十七）：**by-tactic Phase 2 落地（上条「实现为 Phase 2」的执行）**：
   front 产出每 tactic 步状态（`DeclState.by_steps`，进 I8 快照并随注释编辑重映射
   span）；LSP 新请求 `soko/stateAt`——选择语义定为 Lean `goalsAt?`（光标在某条
   tactic 上显示**执行前**状态，符合学习者「这条 tactic 要证什么」的直觉；初稿的
   「执行后」语义作废），返回 `version` 供客户端丢弃过期响应；VS Code 练习树顶部
   「当前光标处」组（目标/假设/by 进度，选区去抖 200ms + 序号守卫），点击目标跳
   对应 tactic。扩展 0.5.2 → 0.6.0（minor：新学习能力）。同轮顺带修
   `playground.sokonanoda:7` 的 `???`→`sorry` 迁移残留文案。
- 2026-09-10（十八）：**opencode 项目配置适配（用户指令）**：仓库根
   `opencode.json` 增加 `skills.paths: ["./skills"]`（三个 skill 自动加载，
   免软链）、Lean 工具链命令 deny（`lean*`/`lake*`/`elan*`/`leanc*`，硬规则
   配置化）、watcher 忽略构建/依赖产物、Rust 自动格式化关闭（保护 kernel
   冻结快照）；新增 `.opencode/command/` 三条命令（`/gate` 本地门禁、
   `/check` 内核判卷、`/round` 开发轮 SOP）与 `.opencode/agent/teacher.md`
   主 agent（画布老师角色）。`skills/README.md` 更新安装说明；配置改动后需
   重启 opencode 生效。同轮修 VS Code 的 `opencode.json` schema 下载报错
   （工作区 `.vscode/settings.json` 信任 `https://opencode.ai`）。
- 2026-09-10（十九）：**插件自带 LSP 二进制（用户要求，Phase 1+2 已落地）**：
   用户要求像“coq/lean 的 vscode 插件”那样把 bin 打包进插件（调研纠正：官方
   Lean 4 与 VsCoq/Rocq 实际都**不**把语言服务器打进 VSIX，运行时依赖
   elan/opam），消除“装完插件再下载 GitHub”的差体验，尤其是**插件与
   `releases/latest` bin 的版本错配**。设计与行业调研见
   `docs/design/bundled-lsp.md`：per-target VSIX（`vsce package --target`；
   exec 位必须在 Linux/macOS 打包）+ universal 回退包（下载 URL 按扩展版本
   `v${version}` 锁定，禁止 latest）。**Phase 1（核心解析/打包脚本/单测/
   契约）与 Phase 2（release.yml 逐平台打包发布 + tag↔版本门禁 + exec 冒烟）
   已落地并通过 release dry-run；Phase 3 剩余为额外平台评估。**
- 2026-09-10（二十）：**平台矩阵扩展 4 → 8（用户要求）**：对标成熟插件
   （cpptools 9 / C# 8 / rust-analyzer 8，均含 alpine），新增 linux-arm64、
   alpine-x64、alpine-arm64、win32-arm64 四个平台包；Linux 二进制改
   cargo-zigbuild 并显式 glibc 2.28 地板（顺带修掉 ubuntu-24.04 原生构建
   引入 glibc 2.39 的兼容隐患），Alpine 为静态 musl；扩展运行时按
   `/etc/alpine-release` 选择 alpine 包。扩展 0.7.0 → 0.8.0。行业标准
   矩阵 = 3 OS × 2 架构 + alpine（± linux-armhf）。
- 2026-09-10（二十一）：**用户路径零工具链依赖（用户纠正"cargo run 是重大失误"）**：
   ①Release 同时发布各平台 `sokonanoda-cli-<triple>.tar.gz`（8 个，可直接执行）
   与 `sokonanoda-lsp-<triple>.tar.gz`（8 个）；②平台 VSIX 内嵌 **LSP + CLI**
   两个二进制（课程树开箱可用）；③opencode launcher 解析顺序补全为
   本地构建 → 扩展自带 → 缓存 → **版本锁定自动下载**（离线可禁）→ 编译兜底；
   ④全仓文档审计：用户/agent 面向的 README/技能/教学手册/错误文案一律改用
   二进制或 `$SOKO`，cargo 只在"贡献者/源码构建"语境出现；⑤该原则升为硬规则
   （§2 第 9 条）。扩展 0.8.0 → 0.9.0。
- 2026-09-10（二十二）：**环境配置单一入口（用户要求「流程理顺、调研优秀
   实践」）**：新增 `scripts/soko.sh`（setup/doctor/grade/gate/lsp；退出码
   0/1/2/3；`--json` 机器可读），设计见 `docs/design/onboarding.md`（3 路
   subagent 调研 OSS/agent/安装器实践）；opencode 命令迁移到命名空间
   `/sokonanoda/*`、launcher 瘦成 shim、启动插件自动 provisioning + PATH
   注入；AGENTS/技能/README 全部改为引用单一脚本（用户/agent 零 cargo）。
   同轮修复 v0.8/v0.9 Release tarball 丢可执行位（artifact 往返剥离 mode）
   并回填修复已发布的 v0.9.0 资产；本地网络需代理时用
   `HTTPS_PROXY=http://127.0.0.1:7890`。
- 2026-09-10（二十四）：**函数实参洞 + hover 开项修复（用户要求，以
   `playground.sokonanoda:233` 为例）**——①`Eq.subst.{1}` 这类宇宙应用
   常量的 hover 必须显示完整类型（此前内核 pp 的 `is_implicit_fun` 对开项
   推断 panic、类型文本被吞成空；同一 bug 让 `#check (Eq.subst.{1})` /
   `(Eq.refl.{1})` 假报 `kernel-rejected`）；②已知函数（prelude `Eq.subst`/
   `Eq.refl`、源内 axiom/def/theorem、归纳构造子）的**直接实参** `sorry`
   是合法练习状态，编辑器提示该洞期望类型（`Eq.subst.{1} Nat (sorry) …`
   → `Nat -> Prop`）；Bare 模式下文件自定义的 Eq 同样进模板。明确不做
   （v1）：嵌套洞、部分应用补参、`sorry + 1`、kernel 级 spine meta。
   实现/验收/风险取舍见 `docs/design/goal-func-spine.md`。
- 2026-09-10（二十四续）：**从零教学场景确认（用户重申）**：用户明确要在
   教学中关闭 prelude、自建 `Eq`/`Nat`。Bare 模式已支持（§5：文件注释
   `-- sokonanoda:prelude none` 或 CLI `--bare`），本轮把该工作流写进
   teacher skill 的判定细节节，并确认函数实参洞在 Bare 下用文件自定义的
   Eq 模板同样生效。
- 2026-09-10（二十五）：**VS Code 希腊字母矩形框（用户反馈）**：`α` 等
   希腊 binder 名被 VS Code 的 Trojan-Source 混淆字符高亮
   （`editor.unicodeHighlight.ambiguousCharacters`，默认开）画框。修复 =
   扩展 `contributes.configurationDefaults` 对 `[sokonanoda]` 语言关掉该项
   （与 VS Code 内置 plaintext/markdown 同法，不改用户全局设置）；仓库
   `.vscode/settings.json` 同步一份，开发态重载即生效。扩展 0.9.0 → 0.9.1
   （patch；Cargo workspace 版本同步）。零新能力、纯观感修复。
- 2026-09-10（二十六）：**`#check` 结果常驻显示（用户要求，Lean Infoview
   对照）**：`#check X` 在编辑器里不再只有悬停可见——LSP inlay 在表达式后
   常显内核结果（`#check Nat` → `Nat` 后 `: Type 0`）。front `DocumentReport`
   新增 `checks`（表达式 span + 内核打印文本，随增量快照缓存、零重编译平移）；
   LSP inlay 渲染；扩展 0.9.1 → 0.10.0（minor：新展示能力）。
- 2026-09-11（二十七）：**文档结构收敛 + 清理（用户要求）**：文档分层——
   入口/权威在仓库根（`README.md`/`AGENTS.md`/`ROADMAP.md`/`REQUIREMENTS.md`/
   `STATUS.md`），开发者参考在 `docs/` 顶层，设计在 `docs/design/`，调研笔记
   在 `docs/notes/`；新增 `docs/README.md` 文档地图（`AGENTS.md` 指向它）。
   删除零引用/过时件 `docs/notes/last-request.md` 与
   `docs/design/learner-round.md`；全仓引用路径同步；设计文档作为 as-built
   存档保留（被后续轮次取代的细节以 `STATUS.md` 为准）。
- 2026-09-11（二十八）：**内核已定义名字的声明 warning（用户要求）**：顶层
   声明名撞内核已定义的名字（`Prop`/`Sort`/`Type`）时——本编译器接受但该名字
   永不被用到（`Prop` 等始终指内核定义的那个），官方 Lean 里还会重复声明
   报错——产出**非致命 warning**。前端 `compile::warning` 双通道
   （`CompileOutput.warnings` + `DocumentReport.warnings`，纯语法、span 收窄到
   名字 token、会话零重编译路径现算）；CLI `--json` 新事件 `warning`
   （`code`/`message`/`hint`/`span`，不改退出码），人类视图 stderr；LSP 映射为
   `DiagnosticSeverity::WARNING`。设计见 `docs/design/reserved-decl-warning.md`，
   协议见 `docs/protocol.md`。
- 2026-09-11（二十九）：**warning 文案去生造词（用户要求）**：`reserved-declaration-name`
   的文案里「内置排序」是生造词，用户要求直白——改说 `Prop` 内核已经定义过、
   并点出 Prop 在形式化证明里的特殊地位，不再出现「排序」一词；同步
   front message/hint、CLI/LSP 文案、设计文档与 teacher 参考。
- 2026-09-11（三十）：**`Type n` 记法（用户要求）**：学习者在画布写
   `axiom Prop : Type 0` 报 `expected a pi type, got: Sort(2)`——本编译器
   原只认单独的 `Type`（= `Sort 1`），没实现 `Type n`。补上 Lean 记法
   `Type n = Sort (n + 1)`（`Type 0` = `Sort 1`，`Type u` 仍不支持，写
   `Sort u`）；纯解析糖，复用 `SortKind::Sort`，不碰 elaborator/内核。三件套：
   课程单元④（zh + en + 解答钥匙）写清 `Type n = Sort (n+1)` 并加
   `#check (Type 0)`；front 解析/编译单测 + CLI e2e；白名单文档同步。
   设计见 `docs/design/type-level-syntax.md`。
- 2026-09-11（三十一）：**onboarding 的 `update` / `version` 命令（用户要求）**：
  经查 `scripts/soko.sh` 原只有 `setup`/`doctor`（`setup` 兼作更新，`doctor`
  报版本），没有具名 `update` 与看版本号的命令。补：`soko.sh update`（=
  `setup --force`，强制刷新到仓库版本）与 `soko.sh version [--json]`（只读
  报告仓库版本/平台与缓存 CLI/LSP 的 `<version> <target>` 标记是否匹配）；
  opencode 补 `/sokonanoda/update`、`/sokonanoda/version`；插件按标记校验
  缓存并在重下后写标记（修掉纯缓存不随版本更新的缺陷）。契约测试与
   `AGENTS.md`/`docs/design/onboarding.md`/teacher 技能同步。
- 2026-09-11（三十二）：**环境能力进二进制、删除 `soko.sh`（用户要求）**：
  用户要求把 onboarding/环境能力做成**二进制 CLI** 并拒绝 `scripts/soko.sh`。
  落地：`sokonanoda` 新增子命令 `version`/`doctor`/`setup`/`update`/`grade`/
  `gate`（沿用既有 `lsp`）；`setup`/`update` 用内嵌下载器
  （`ureq`(rustls/ring) + `flate2` + `tar`，编译期 `TARGET` 钉死）按版本锁定
  拉取 release 资产；删除 `scripts/soko.sh`，opencode 命令改调二进制、插件与
  shim 去脚本化；缓存标记 `<version> <vsce-target>` 与 VSIX/插件一致。
  设计见 `docs/design/binary-cli.md`。首次获取二进制仍由 opencode 插件或
  VSIX 完成（不是 shell 脚本）。
- 2026-09-11（三十三）：**发版 0.13.0 + opencode 全量初始化（用户要求）**：
  用户发现缓存里的 `sokonanoda` 缺 `version`/`setup`/`update` 等子命令
  （`v0.12.0` tag 停在 onboarding 二进制提交之前，Release CLI 比仓库旧），
  要求「先发版，再继续配置」，让编译好的最新 CLI 可直接下载、不要本地
  cargo。落地：版本 0.12.0 → **0.13.0** 并推 tag `v0.13.0`，CI 编 8 平台
  资产；随后 `sokonanoda update` 按锁定 `v0.13.0` 下载编译好的 CLI+LSP，
  opencode 接线（插件/commands/agent/skills）全量就绪。
- 2026-09-12（三十四）：**opencode 插件迁移到官方 `.opencode/plugins/`（用户要求）**：
  opencode 官方项目级插件目录为复数 `.opencode/plugins/`（文档与新版行为），
  旧的单数 `.opencode/plugin/` 不再保留。落地：`sokonanoda.ts` 原样迁移
  （provision CLI/LSP + `config` 接线 `lsp.sokonanoda` + `shell.env` 注入
  PATH 的行为不变），`findRepoRoot` 仓库标记改指新路径；契约测试改读新路径
  并断言旧目录不存在；`opencode.json` 仍不写 lsp 命令。非 opencode harness
  的 `.opencode/lsp/sokonanoda-lsp.sh` shim（`skills/README` 记载的用法）保留。
- 2026-09-12（三十五）：**扩展残留清理 + 插件扩展自带 LSP 解析加固（用户要求）**：
  用户机器 `~/.vscode/extensions` 出现 0.2.1/0.4.2/0.5.3/0.12.0/0.13.0
  多版本共存，要求清理并检查插件是否缺「清理旧版本」逻辑。结论：多版本全由
  VS Code 管理（各 profile 的 extensions.json 可引用不同版本，删旧是惰性
  标记 + 下次启动），opencode 插件缓存固定路径单版本、无此缺陷；真实瑕疵是
  解析扩展自带 LSP 只按 mtime 取新、不认版本号也不跳过 VS Code 的
  `.obsolete` 待删目录。落地：解析按目录名版本号取最高（无版本号才回退
  mtime）并跳过 `.obsolete`；机器清理为默认 profile 升级到 0.13.0 + 删除
  孤儿目录（非仓库改动）。

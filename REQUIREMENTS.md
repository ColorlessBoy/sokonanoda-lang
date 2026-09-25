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
8. **判定永远走 kernel**，不做文本比对（`proof.rs::assumption` 的文本比对草案已于 2026-09-07 删除）；
9. **用户/agent 使用路径零工具链依赖**（2026-09-10 用户明确）：获取与运行只
   依赖 GitHub Release 资产（`sokonanoda-cli-*.tar.gz` / `sokonanoda-lsp-*.tar.gz`）
   或平台 VSIX 插件，**不要求 Rust/cargo**；cargo 仅贡献者开发需要。面向
   用户/agent 的文档、技能与错误文案不得把 cargo 当使用前提。
10. **课程标准库三层分界**（2026-09-18 用户确认「进硬规则」）：写课程内容时，
   每一条陈述必须先归层——**L1 prelude**（Lean core 级：逻辑与等式骨架）/
   **L2 课程标准库**（集合的词汇 + 定义展开，Mathlib 里是 `rfl` 或一行、**没有数学内容**）/
   **L3 单元练习**（一切**有数学内容**的陈述）。判据只有一条：
   **「Mathlib 有 ≠ 我们不该练；Mathlib 有且没有数学内容（纯定义展开）才归库。」**
   违反的样子 = 让学习者在证明里手写 `Eq.symm`/`Or.elim`/`mem_union` 这类东西（"暴力"）；
   证据与方案见 `docs/design/course-stdlib.md`，现场见 `docs/gaps/spike/README.md`，
   欠账见台账 `L-01…L-05`。配套的**引用纪律**：课程文档里的每句教学主张要么给出处、
   要么显式标"设计判断"；**有序对与选择公理没有任何实证研究**（实测否定结果），
   不得写成"研究表明…"（细则见 `docs/design/set-theory-syllabus.md` §1.4）。

## 3. 内核性能是产品优势（2026-09-06，用户要求）

- sokonanoda 内核的快（arena + hash-consing + 闭包求值，相对官方内核 10–100x）
  是本项目立身之本：**任何重构/新功能都不得触碰 `crates/kernel` 热路径**；
- 前端只消费内核公开 API；教学功能（prelude、elaborator）不得给内核加间接层；
- 守护手段：保留内核全量测试 + 新增前端 perf 冒烟测试（大数字原生归约、
  iota 深归约链），CI 跑通，防止无意中把快路径弄慢。

## 4. 工程标准：模块化、单文件不许越长越大（2026-09-06，用户要求）

- **现状不合格**（2026-09-06 时是 `front/lib.rs` ~1200 行 / `front/compile.rs`
  ~2100 行，均已拆完；**当前欠账**）：`crates/lsp/src/lib.rs` **3949 行**、
  `crates/front/src/compile/tests.rs` **3564 行**——后者是测试文件，暂以
  「测试可后拆」豁免，但两者都远超 ~500 行红线，拆分排期见 ROADMAP（intro
  一族 → `lsp/src/intro.rs`）；
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

- 已完成：I6 → playground 开课 → I7 第一门课 → I8（余项：依赖精确化 /
  early-cutoff）→ I9 goal 视图 → I10 值位 `apply` → I12 官网（上线）。
- **当前执行：I11 余项（真人输入测试 S2–S4：front 往返矩阵 F1–F5、LSP L3–L8、
  VS Code 手势 V2–V4）→ I8 余项 → L2/L3（编辑器打包已由 bundled-lsp 落地，
  余下为 service 事件流、讲课 agent 深化）。**
- 发布流程已自动化（ci auto-tag，见 `docs/RELEASE.md`）。
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
- 2026-09-12（三十六）：**值位 `intro` 关键字 + 展开补全（用户要求）**：
  用户要求一个确定性的「类似 intro」补全——`theorem … := intro` 时 VS Code
  建议原地替换为 `fun … => sorry` 骨架，明确拒绝 Copilot 式 AI 补全的
  「作弊感」。拍板：一次全剥（等价 Lean `intros`）、匿名 binder 名沿用
  现有生成器 `x`/`x2`。设计见 `docs/design/term-intro.md`；同日实现落地：
  值位 `intro` 全剥降低（`compile/intro.rs`，非函数目标
  `elab-intro-not-a-function`）、LSP 补全展开项（`textEdit` + 文档）、
  inlay/session 联动、课程 unit6 补充与 golden；顺带修正旧缺陷
  `render_expr` 函数位多余括号与 session 洞 span 未重映射。
- 2026-09-12（三十七）：**声明级 binder（Lean 风格，用户要求）**：用户要求
  支持 `theorem and_swap2 (a : Prop) (b : Prop) (h : And a b) : And b a :=
  sorry`——省去 `intro`/`fun` 的麻烦。设计见 `docs/design/decl-binders.md`；
  同日实现落地：parser 侧 `wrap_decl_binders` 降级为「Forall 类型 + Lambda
  值」（open-goal/内核全复用）、`by` 引擎接收声明 binder 为初始上下文、
  `intro` 沿 lambda 链递归只剥剩余 codomain；课程 unit1 两种拼写对照 +
  练习 6（zh/en + 钥匙）。发布随 0.15.0。
- 2026-09-12（三十八）：**手动重启 LSP 的 VS Code 命令（用户要求）**：用户
  遇到「扩展/LSP 更新后旧进程不生效」的困惑，要求插件提供手动重启语言
  服务器的命令。落地：`sokonanoda.restartServer`（命令面板
  「sokonanoda: 重启语言服务器」，0.19.0 起改名 restart server）先重解析二进制路径再 `client.restart()`，
  无需重载窗口（重建的仓库构建 / 刷新后的缓存 / 改 `serverPath` 都生效）；
  扩展本体升级仍需 Reload Window（README 与 `docs/vscode-dev-guide.md` §5
  写明）。发布随 0.16.0。
- 2026-09-12（三十九）：**`intro` 换行/词尾命中修复 + hover 展开按钮 +
  「不替换也等价」契约（用户要求）**：用户反馈 `playground.sokonanoda:201`
  「同一行输入 `intro` 符合预期，换行就没用了（那行会太宽）」，并要求
  ①「hover 信息加一个按钮，直接替换 `intro`，跟 tab 补全一样，防止错过
  tab 补全」；②「`intro` 也可以不被替换，直接等价于对应的 `fun` 表达式，
  这样更方便」。落地：命中区间从洞的闭区间扩到「token 之后到同一行行尾的
  空白」（`trailing_same_line_ws`，hover 与补全共用 `intro_hit`/`intro_at`；
  真实触发是光标停在词尾或尾随空格，折行只是放大器）；hover 带
  `command:sokonanoda.expandIntro?<服务端算好的 uri+range+newText>`，
  客户端新增 `sokonanoda.expandIntro`（`markdown.isTrusted` 白名单放行 +
  `contributes.commands` + 命令面板隐藏），应用的编辑与 Tab 补全完全同一份；
  hover 文案明说「不替换也完全等价」；front 新增两条契约测试
  （`intro_is_equivalent_to_typing_the_skeleton_out_by_hand`、
  `value_intro_is_layout_independent`）把等价性与排版无关性钉死。发布随
  0.17.0。**非目标不变**（`docs/design/term-intro.md` §9）：嵌套 `intro`
  （`fun … => intro`）仍不做——那里的 `intro` 是普通标识符。
- 2026-09-13（四十）：**三个大方向（用户要求，先设计后实施）**：用户提出
  ①「参考刚才的 intro 设计新的 command apply，也能触发自动补全和等价部分表达式。
  必要的时候可以要求用括号确定范围」；②「设计真人相同的输入测试，多设计测试，
  覆盖 sorry intro apply 和 by 这些 command 共存会引发的复杂」；③「增加一个
  github pages，相当于当前项目的官网，充分介绍本项目的用法、远大目标和当前进展」；
  并要求「先好好调研、头脑风暴、规划，做好文档，然后再启动分步骤计划，多用
  subagent 执行」。**本轮只落设计，不动实现**（设计先行是仓库硬规则）。产出：
  `docs/design/term-apply.md`（值位 `apply`：语法/语义/错误码/编辑器面/风险；
  **关键结论——不能照抄 `intro`**：`apply` 需要被应用名字的类型，而 front 侧
  `GoalTemplates` 丢了 codomain、局部假设不在表内、内核无类型查询 API，
  故降低走 `by` 引擎已验证的 `judge_infer` 路线，并新增「合成洞不得走
  `judge_hole_fill` 源码切片守卫」的分派修正）；
  `docs/design/real-input-tests.md`（三层输入测试：front 版本序列 / LSP
  `didOpen→didChange` 脚本 / VS Code 真实手势；含共存风险矩阵与**调研发现的
  既有缺陷**：`by` 末个 tactic 是 `apply` 且留 ≥2 子目标时所有洞 span 相同，
  导致 nextHole 跳不动、inlay 叠位）；
  `docs/design/site.md`（官网：**零构建静态 `site/` + Actions 部署**，不把
  `docs/` 设为 Pages 源；「版本/进展/单元数」一律由 `STATUS.md`/`Cargo.toml`/
  `course.json`/Releases API 生成，官网只做视图不做第五个事实源；一期零后端，
  主 CTA = 装 VS Code 扩展，WASM playground 入 backlog）。
  分阶段计划与验收标准落 `ROADMAP.md` §10 的 **I10（值位 `apply`）/ I11（真人输入
  测试）/ I12（官网）**。**阻塞项**：官网需仓库拥有者在 Settings → Pages 手动把
  Source 设为 GitHub Actions（会话内无法代做）。
- 2026-09-13（四十一）：**三个大方向落地（用户确认「三条串行进行」）**：按
  依赖顺序 I11-S0 → I11-S1 → I10 → I12 执行。①真人输入脚本基建
  （`type_step`/`char_steps`，一步一断言、零 sleep）；②修复 `by apply` 多子目标
  的 inlay 类型错配（按洞位置顺序对齐 sub_goals），`nextHole` 同址限制记入
  protocol.md；③值位 `apply` 全链（parser/AST/`compile/apply.rs`/`spine.rs`
  共用机械/局部假设覆盖层/合成洞分派/2 个错误码/LSP hover+补全/
  `sokonanoda.expandApply`）；④课程第 6 单元补「值位 `apply`」一节 + 练习 +
  钥匙（golden 刻意变更 unit6 `(13,6,0)`→`(17,10,0)`、汇总 `33/27`→`37/31`）；
  ⑤官网静态 `site/` + `gen-site-data.py` + `check-site.py` + `pages.yml`，并修
  4 处文档漂移。版本 0.17.0 → **0.18.0**。
- 2026-09-13（四十二）：**发布链自动化闭环（用户复盘触发）**：①`ci.yml`
  新增 auto-tag——main 全绿后自动打 tag 并 dispatch release，发布零人工
  （GITHUB_TOKEN 推 tag 不触发 workflow，须显式 `workflow_dispatch`，需
  `actions: write`）；②release.yml 的创建/上架步骤改按 ref 判（原按 event
  判会被 dispatch 静默 skipped）；③pages 门禁改鉴权 `gh api`。发布结果：
  v0.20.0 上 Release（25 资产）+ Marketplace + 官网上线（About 三件套已填）。
  手动推 tag 降级为应急路径（`docs/RELEASE.md`）。
- 2026-09-13（四十四）：**性能是生命线（用户明确）+ 半截表达式 goal-state
  hover + 演示例行化**：①判定结果缓存（judge_infer/terms/hole_fill 指纹
  缓存，封顶 128 条）——by 块 tactic 的 judge_infer 每键全前缀重编译与
  funapply 同根，缓存修复；②内核拒绝的半截表达式（如 `And.intro b a`）
  hover 显示推断出的剩余目标 `|- b`、`|- a`——只在 hover 请求时计算；
  ③官网演示 GIF 例行化（`gen-site-demos.py --check` 进 pages workflow）；
  ④value `funapply` 于 0.22.0 移除（见前条），GIF 与文档同步清除。
- 2026-09-13（四十五）：**性能测试例行化（用户明确）**：编译器与编辑器
  交互路径都要有性能测试、覆盖全面且细致，每版本留档可追溯。落地 =
  ①阈值断言进常规测试（缩放比哨兵抓 O(n²)：check_document 400/50 块
  ≤12×；增量编辑每键 <50ms 且 kernel_checks≤1；编辑首练习延迟不随文件
  长度超线性）②LSP 交互延迟哨兵（didChange <50ms、completion/hover/
  soko/goals <10ms）③CI "Performance report" 步骤：每次 push 产出
  带 version+SHA 的报告 artifact（perf-report），本地同口径
  `scripts/perf-report.sh`。设计原则见 `docs/PERF.md`。
- 2026-09-13（四十三）：**值位关键字 v2（用户三需求）**：①输入过程中要有
  补全替换提示（探针实测根因：keyword_at 要求 Open 态，而输入中间态必然
  Failed）；②`funintro (funapply X)` 关键字组合——纯前端实现，funintro 到
  内核就是 fun 链、funapply 到内核就是部分应用带前提洞；③值位关键字改名
  `funintro`/`funapply` 与 tactic 消歧义（by 块 tactic 不变），按钮文案改
  「替换源代码 funintro/funapply」。设计定稿 `docs/design/value-keywords-v2.md`
  （决策单 D1–D9：无别名、命令 ID 保留、KEYWORDS 保留旧词加新词、错误码
  机器码保留只改 hint）。实施按 ROADMAP I13 S1–S4 由 subagent 顺序执行。
- 2026-09-14：**官网 code agent 部分简化（用户明确）**——GitHub Pages 偏复杂，
  code agent 相关三处（首页安装卡 / 快速开始技能列表 / 给 Agent 页）统一为
  「一段安装 prompt + 一个复制按钮」；prompt 取简短委派版（克隆仓库 → 读
  `AGENTS.md` → 装环境 → `--json` 判卷开课），文本单一源
  `site/assets/agent-prompt.js`（中英），协议细节收进折叠与链接。
- 2026-09-14（续）：**VS Code 插件部分同样做减法**——首页学习者卡与快速开始
  压成两步（市场一键安装按钮 + clone 打开画布），删掉写错的
  `code --install-extension sokonanoda.vsix` 与第三步描述，离线/指定版本
  VSIX 收进小字；英文镜像同步。设计见 `docs/design/site.md` §4.4。
- 2026-09-14（再续）：**官网提供 vscode: 协议直达**——安装
  `vscode:extension/sokonanoda-lang.sokonanoda`、克隆
  `vscode://vscode.git/clone?url=…`；市场主按钮与命令行路径保留兜底，
  `check-site.py` 支持按 `scheme:` 跳过外链。设计见 `docs/design/site.md` §4.5。
- 2026-09-14（四十八）：**量词课程（用户明确）**——教程逻辑内容偏少、
  缺 forall/exists 题目，参考 Metamath 出题，并同步进 `course/`。落地 =
  ①画布第二课（∀=依赖函数、`Exists` 公理三件套、Person/someone 非空
  论域、7 题 + 三层提示 + `#reduce` 自测）②`course/unit7-quantifiers.sokonanoda`
  + 解答钥匙 + 英文镜像 + golden `(14, 7, 1)` + `course.json` unit=1..7
  ③§3 钥匙表与 §5 课程地图同步到 `docs/teaching-session.md`。
- 2026-09-14（四十九）：**多目标显示（用户报告）**——画布上
  `apply And.intro; intro x` 之后应同时看到 `P x` 和 `(x : Person) -> Q x`
  两个待证目标，当前只显示一个，属引擎层数据缺失（`ByStep` 只记 worklist
  栈顶）。修复 = ①前端每步记录**全部**未闭合目标（当前在前，各带自己的
  假设）②`soko/stateAt` 增 `goals[]`、`soko/goals` 每声明增 `goals[]`
  （单值 `goal`/`binders` 保留 = `goals[0]` 兼容旧客户端）③VS Code 两个
  目标视图渲染多目标节点。设计见 `docs/design/goal-list.md`。
- 2026-09-14（四十九·续）：**移除值位关键字 `funintro`（用户明确）**——「跟
  `funapply` 一样实现得稀里糊涂，不如直接删了」。值位从此只保留普通表达式与
  `by` 块；删语法/引擎/`intro_skeleton`/`ErrorKind::ElabIntroNotAFunction`、
  LSP 补全·hover·code action·inlay、VS Code `expandIntro` 命令、课程 unit6
  相关段落（改写为综合 `by` 练习）、文档/site/技能；by 块 tactic `intro` 不动。
  并入 0.27.0。设计 + as-built 见 `docs/design/remove-funintro.md`。
- 2026-09-14（五十）：**tactic 关键字高亮 + hover 中间 goal state（用户明确）**
  ——「`exact` 没有正确高亮；希望像 Lean 一样，在每个 tactic 上 hover 都能看到
  中间 goal state（Infoview 式），或按鼠标位置给 goal state」。落地 = ①
  `semantic::KEYWORDS` 增补 `by`/`exact`/`assumption`/`rfl`（`forall`/`sorry`
  已分别由 token/Hole 着色）②`textDocument/hover` 首插 tactic 命中：光标落在某
  tactic span 内 → 用 `by_steps` + `select_state_at` 渲染「进入该 tactic」的
  全部目标与假设（与 `soko/stateAt` 同数据、同语义，零重编译）。并入 0.27.0。
  设计见 `docs/design/tactic-hover.md`，协议见 `docs/protocol.md`。
- 2026-09-14（五十一）：**`restart server` 版本纪律修复（用户报告）**——用户
  下载 0.27.0 插件后 LSP 仍是 0.26.0，疑发布流程。核实：发布无误（v0.27.0
  darwin-arm64 VSIX 内置 LSP 实测 `soko/version = 0.27.0`，`server.js` 内置
  优先解析正确）；0.26.0 来自本地下载缓存 + `restart server` 无下载兜底、
  不校验。修 = restart 复用激活解析链（含 `v<扩展版本>` 锁定下载兜底）、
  解析不到可用服务器时报错而非静默重启旧命令/旧缓存、检测到磁盘上更新的
  扩展而宿主仍旧时提示 Reload Window。版本 **0.27.1**。见
  `docs/vscode-dev-guide.md` §5.6。
- 2026-09-14（五十二）：**TODO 清账（用户明确：全部按流程做，多用 subagent）**
  ——把 ROADMAP §10 未勾选、TESTING §5 盲区、onboarding §5 待办按依赖清账：
  ①修 `protocol_doc_lists_every_error_code` 的假穷尽守卫（`matches!` 自带
  `_`）为不可绕过的 `match` + 26 variant 数组，补 `elab-apply-*` 文档；
  ②补 codeLens/quick-fix 进程内 rpc 测试；③重建两个内核冻结 fixture
  （`RuleDomainMismatch`/`UnlistedRecursor`）并解禁对应测试；④落地
  `scripts/install.sh`（零 cargo、版本锁定、禁 latest）与 `.devcontainer`；
  ⑤为 `let`/`match`、spine-meta A、webview Infoview、compiler service
  事件流产出设计文档（实现留后续轮）。全过程 8 个 subagent 并行产出，
  主会话统一 gate。设计见 `docs/design/*`；本轮机/文/脚本改动不 bump 版本。
- 2026-09-14（五十三）：**elaborator `let`（Phase 1，I6）**——按
  `docs/design/elaborator-let-match.md` 落地值位 `let x : T := v; body`
  （任意 term 位置；kernel 冻结，降为内核 `Let`，判定走完整内核）。front
  parser/AST/elab/goal 全链 + zeta 等价契约测试；unit3 新增「局部绑定 let」
  课程（zh/en/钥匙）+ golden；CLI e2e；缺类型注解复用 `elab-untyped-binder`。
  `match` 与无注解 `let` 依设计推迟 Phase 2。版本 **0.28.0**（新语法 = minor）。
- 2026-09-14（五十四）：**编译器服务事件流（L1/L3，R57）**——`sokonanoda watch`
  开场事件规范名 `file.changed` → **`file.didChange`**（旧名保留一个 minor 的
  弃用别名）；stdout 第一行恒为 `service.hello {protocol,engine,pid}`；新增
  `--doc <file>` / `--workspace <root>`（每文件独立 session 与版本、事件带
  `file`、跨文件无全序）；有界缓冲溢出合并标 `recompiled_from:0`（可全量重
  同步）。设计 + as-built 见 `docs/design/compiler-service-events.md`；版本
  **0.29.0**（协议/功能 minor）。
- 2026-09-14（五十五）：**VS Code webview Infoview（方案 B，R56）**——新增
  `sokonanoda.infoview`（webview，与练习/课程并列）+ `sokonanoda.openInfoview`
  命令，Lean Infoview 式渲染全部目标 + 假设 + by 进度 + 服务器版本；宿主独占
  `LanguageClient` 并把 `soko/stateAt`/`soko/goals`/`soko/version` 快照 post
  给 webview（`protocol:1`）；光标移动只发轻量 `state`（不去 `soko/goals`、
  不重建），CSP+nonce、仅 `textContent`；webview 不可用时静默回落树组。树的
  「当前光标处」保留。设计 + as-built 见 `docs/design/webview-infoview.md`；
  版本 **0.30.0**（新 view+命令 minor）。
- 2026-09-14（五十六）：**VS Code 扩展强制内置 LSP + doctor 自检（用户明确）**
  ——用户报告扩展 0.29.0 后 `restart server` 仍 `0.26.0 → 0.26.0`；盘链路确认
  是用户设置 `sokonanoda.serverPath` 指向仓库陈旧 debug 构建（0.26.0），显式
  路径静默压过内置服务器。修：①解析默认 **bundled-first**（`serverPath`/env/
  工作区构建忽略并提示；贡献者用新设置 `sokonanoda.serverOverride` 显式恢复
  旧序）②新增只读 `sokonanoda.doctor`（6 项自检：解析来源/运行版本/宿主版本/
  被忽略覆盖/缓存/旧版本堆积），激活与 restart 后自动跑一次③restart 回执带
  `source=`。设计 + as-built 见 `docs/design/extension-server-policy.md`；
  版本 **0.31.0**（新设置+命令 minor）。本机止血：清 `serverPath` 设置 + 删
  8 个 `.obsolete` 旧版本。
- 2026-09-14（五十七）：**spine meta 方案 A（I9 余项，R55）**——refine/goal
  子洞期望类型改为**请求期内核探针**：`front::goals::probe_sub_goal_types`
  （重解析 + `judge_infer`，复用 128 条有界缓存）算「第 i 实参期望 = 部分应用
  类型剥最外层 Pi domain」，支持**前置洞穿透**与**一层嵌套洞**；`open_goal`
  仍 `probe=None` → 键路径零内核调用（perf 无回退）。LSP 只在 `soko/goals`/
  hover/inlay 请求期补 `None` 的 `sub_goals[i].ty`，协议形状不变。更深嵌套与
  def 包裹结果类型的 whnf 仍走 B′（需内核/pp 暴露 whnf，违反冻结）。设计 +
  as-built 见 `docs/design/spine-meta-a.md`；版本 **0.32.0**（新增公开 front API）。
- 2026-09-14（五十八）：**I8 early-cutoff 依赖精确化 + arena 基准立项（R58）**
  ——Session 增量新增**保守 sound** 的 early-cutoff：每条命令用内核结构化
  `debug_print` 渲染「环境贡献签名」（含 type + **body**，保证 delta 可观察性），
  单点编辑时签名相等即停止重查并复用尾部快照（测试实测 kernel_checks 4→1 /
  3→1 / 6→1；改 body、宇宙元数、多点编辑、内核拒绝一律退回旧后缀重查；
  prelude 形状守卫重建）。另把外部 perf/soundness 基准立项为 opt-in：
  `scripts/perf-arena.sh` + `crates/kernel/tests/arena.rs`（`LEAN_KERNEL_ARENA`
  门控，不 vendor、不进 CI、不引入官方 Lean 工具链）。设计 + as-built 见
  `docs/design/early-cutoff.md`；版本 **0.32.1**（内部性能优化 patch）。
- 2026-09-14（五十九）：**elaborator `match` v1（Phase 2 首切片，R60）**——支持
  对**源内非递归 `inductive`** 分情况：`match e with | Ctor x… => body`；裸
  ctor、按声明序重排、恰好覆盖一次；结果类型取所在位置的期望类型；降低为
  `<Ind>.rec.{level} (fun _ => R) minors… e`（level 由期望类型的 Sort 推出，
  spike 证明显式宇宙必要）。新增 5 个错误码；front +19 / CLI +4 / 课程 unit5
  小节 + golden。v1 不做：递归归纳（IH）、依赖/参数化归纳、prelude `Nat`/`Eq`、
  `match` tactic、嵌套/字面量/守卫、无注解 `let`。设计 + as-built 见
  `docs/design/match.md`；版本 **0.33.0**（新语法 minor）。
- 2026-09-14（六十）：**tactic hover 呈现升级（用户体验）**——原 hover 是
  裸行文本（无排版/高亮）。改为：表头 `` `<tactic>` · tactic k/n ``；每个目标
  包进 ` ```sokonanoda ` 代码围栏（等宽对齐 + 语法高亮，扩展自带 TM 语法），
  多目标加 `**目标 i/n**`；半截表达式 hover 同步 `⊢` + 围栏。版本 **0.33.1**
  （呈现改进 patch）。设计 as-built 见 `docs/design/tactic-hover.md` §5。
- 2026-09-14（六十一）：**无注解 `let`（Phase 2 小切片）**——`let x := v; body`
  的类型标注变为可选：`binder.ty == None` 时用当前 scope 的 `judge_binders()` +
  `judge_infer`（复用有界缓存）推断值类型并回 AST 作 binder 类型；推断失败
  （值位 `sorry` 等）报新码 `elab-let-type-query-failed` 并提示补标注。front/CLI
  测试 + 课程注释同步；kernel 零改动。版本 **0.34.0**（新能力 minor）；设计
  as-built 见 `docs/design/elaborator-let-match.md` §13。
- 2026-09-14（六十二）：**`match` 递归归纳（IH，Phase 2，R63）**——源内递归
  `inductive` 的 `match` 不再拒绝：构造子**递归字段后自动插入归纳假设** `ih`
  （避让既有名 → `ih2`…，类型 = 结果类型 R），branch 可引用，递归函数/证明经
  recursor 表达、无需自引用（`def add (a b : Nat) := match a with | zero => b |
  succ m => succ ih`）。front 测试（`add two two` 归约到 `s(s(s(s z)))`）+ CLI 3
  项 + 课程 unit5 递归 IH 演示与练习；golden (6,5,2)→(7,6,3)。仍缺：依赖 motive、
  参数化/带索引归纳、prelude `Nat`/`Eq`、`match` tactic、嵌套/守卫/字面量模式。
  版本 **0.35.0**（新能力 minor）；设计 as-built 见 `docs/design/match.md` §10。
- 2026-09-14（六十三）：**发布加固（供应链完整性）**——每个 Release 增
  `SHA256SUMS`（8 lsp + 8 cli + 9 vsix）与 SLSA 构建来源证明
  （`actions/attest-build-provenance@v2`，job 加 `id-token: write` +
  `attestations: write`）；资产 25→26。文档 `docs/RELEASE.md §6`、
  `skills/sokonanoda-ci §2.1`、`onboarding §5`（含 rust-toolchain 决策：不钉、
  跟随 stable；README 补 binstall/mise）、release 契约断言同步。版本 **0.35.1**。
- 2026-09-14（六十四）：**`match` 支持 prelude `Nat`**——内置 `Nat` 从「原生
  hack」改为经 `install_inductive_block` 装成的**真实可信归纳**（ctor
  `Nat.zero`/`Nat.succ` + 派生 `Nat.rec` + iota），注册进 `InductiveTable`；
  `match n with | Nat.zero => … | Nat.succ k => …`（点号 ctor）可用，递归字段
  自动 IH。`Nat.add` 保持原生自引用定义；`#reduce 1 + 1 => 2` 等 numeral 正常；
  经 `Nat.rec` 归约的结果可能显示为不合并一元链（与 numeral def-eq，已文档化/
  钉测试）。front +3 / CLI +1；文档 match.md/architecture/TESTING/protocol 同步。
  版本 **0.36.0**（新能力 minor）；kernel 零改动。
- 2026-09-14（六十五）：**watch stdin 客户端命令（compiler-service-events v1 面）**
  ——`sokonanoda watch`（含 `--doc`/`--workspace`）在轮询文件的同时读 stdin JSON
  Lines：`ping {id}` → `pong {id, protocol, engine}`；`subscribe`/`unsubscribe
  {file}` 过滤哪些文件发事件（默认全发兼容旧行为）；畸形/未知命令 → `error`
  事件且流不中断；stdin EOF 不杀 watch。非阻塞（后台线程 + mpsc，仅 std，零新
  依赖）。CLI 测试 +4（ping/subscribe/unsubscribe/malformed）+ 单测 2；文档
  protocol/TESTING 同步。版本 **0.37.0**（新能力 minor）；设计 as-built
  `docs/design/compiler-service-events.md` §9。
- 2026-09-14（六十六）：**参数化归纳声明（非带索引）**——`inductive Option
  (A : Type) : Type / ctor none / ctor some (a : A) / end` 现在可声明（含派生
  递归子）；`match` 可用于其上（params 取 scrutinee 书写源类型头部实参，
  字段类型代入，如 `some a` 的 `a : A`）。前端 parser/AST/install/derive_recursor
  全链；`add_inductive(num_params=params.len())`、字段/iota 计数按「不含 params」
  对齐内核断言。front +8 / CLI +4 / 课程 unit5 `Option` 小节 + golden
  (7,6,3)→(9,7,4)；新码 `elab-match-parameterized-unsupported`（拿不到 params）。
  v1 不做：带索引归纳、宇宙多态参数、互/嵌套递归、match 嵌套/守卫/字面量。
  设计 + as-built 见 `docs/design/parameterized-inductives.md`；版本 **0.38.0**。
- 2026-09-14（六十七）：**`match` 依赖 motive**——结果类型含被匹配的裸局部变量
  `x`（如 `P n`）时，motive = `fun t => R[x:=t]`（shadow-aware 替换）、分支期望 =
  `R[x:=<构造子项>]`、递归 branch 的 IH 类型 = `R[x:=<field>]`；`goals` 走查同步
  代入。由此可写归纳法：`theorem nat_induction (P : Nat -> Prop) (hz : P zero)
  (hs : …) (n : Nat) : P n := match n with | zero => hz | succ k => hs k ih`。
  顺带修 `infer_expected_level` 只纳入 `R` 依赖到的 binder（`judge_binders_for`），
  使声明 binder 形式可用。front `match_dependent_*` / CLI `cli_match_dependent_*` /
  课程 unit5 依赖 match 节 + golden (9,7,4)→(10,8,4)。版本 **0.39.0**（新能力
  minor）；设计 + as-built 见 `docs/design/match-dependent-motive.md`。已知限制：
  motive 引用「类型为以箭头结尾的依赖函数」的 binder 时 judge_infer 往返仍可能
  腐蚀 telescope。
- 2026-09-14（六十八）：**judge_infer 类型往返健壮性**——`proof::render_expr`
  的 Arrow **domain 位**改用 `render_fun_position`（Lambda/Forall/Arrow/Plus/
  Let/Match 一律补括号），修掉「内核渲染类型文本 → 前端 AST」往返把
  `(k : Nat) -> P k -> Q` 右结合误读、腐蚀 telescope 的问题；依赖 `match` 的
  level 查询、suggest、半表达式 hover 一并受益。回归 `render_expr_round_trips`
  （Forall 作 domain）+ `match_dependent_motive_with_function_typed_binder_
  round_trips_safely`。版本 **0.39.1**（健壮性 patch）；as-built 见
  `docs/design/match-dependent-motive.md` §8。
- 2026-09-15（六十九）：**统一 goal 呈现 + Infoview 落右侧（用户要求）**——
  用户两点：(1) Infoview 弹出「目标面板 (Infoview) 暂时不可用…」很困惑，希望它
  默认单独开在**右侧**；(2) tactic hover、Infoview 等各处 goal 高亮/颜色各自
  独立、不可维护，要求统一并**对齐 VS Code 代码框标准**。参照 Lean4 Infoview
  （服务器下发结构化 tag，客户端按主题渲染，分类来源唯一）。落地：
  `front::semantic` 出 `tag_runs`/`tag_expr`/`declaration_kinds` +
  `SemanticKind::{ALL, as_str}` 作单一分类源；`soko/stateAt`/`soko/goals` 下发
  `goal_runs`/`ty_runs`（旧字符串字段保留）；Infoview 渲染 `tok-<kind>` 类
  （主题变量单一映射，仍只用 textContent）；视图移入
  `viewsContainers.secondarySidebar` 的 `sokonanoda` 容器（右侧，engine
  `^1.85.0→^1.106.0`，1.106 为无需 proposed API 的首个稳定版）；删
  `waitReady`/2s 握手与「暂时不可用」，失败静默回退树组；TM 语法词表由测试锁死
  == `front::semantic`（keywords + sorts + forall，删硬编码 `Nat`）。契约测试
  （front semantic 5 + lsp state_at runs + extension 24）+ `sokonanoda gate` PASS。
  版本 **0.40.0**（新面板位置 + 协议字段，minor）；设计 as-built
  `docs/design/goal-rendering.md`。
- 2026-09-15：**市场简介超 300 字符被硬截断（用户要求修复）**——Marketplace
  详情页把 `package.json` 的 `description` 当短简介，>300 字符直接截断（无省略号、
  切在词中间）：0.39.1 的 348 字符被切在 `opencode` 中间。改短为 247 字符完整句
  （长卖点留在 `editor/vscode/README.md`），加护栏测试
  `marketplace_description_fits_the_gallery_limit`（≤300 / ASCII / 句号结尾），
  `docs/vscode-dev-guide.md` §7 记录上限与教训。随 0.40.0 发布。

- 2026-09-15（七十）：**prelude `Bool`（TODO：I6/elaborator 组）**——把 `Bool` 作为
  **真实可信归纳**加进 prelude，与 `Nat`（0.36.0）同法：`install_bool_prelude`
  调 `install_inductive_block` 装出 `Bool.true`/`Bool.false` 真构造子 + 派生
  `Bool.rec`（非递归，两分支消去子），登记进 `known` 与 `match` 的
  `InductiveTable`，于是 `match b with | Bool.true => … | Bool.false => …` 可用、
  `#reduce` 走通用 iota。文件自带 `inductive Bool` 时 prelude 让位
  （`explicit_bool` 闸；`PreludeShape` 扩四元组）。内核**零改动**（`Bool.true`/
  `Bool.false` 的 name-cache 槽位早已存在）。front +4 / CLI +1；文档
  architecture §5.4 / match.md §10 Phase 5 / TESTING / 错误文案同步。版本
  **0.41.0**（新能力 minor）。

- 2026-09-15（七十一，追踪项）：**「各个地方的高亮统一」入账**——用户指出
  0.40.0 只统一了 goal 状态（tactic/半表达式 hover + Infoview），其余渲染
  `.sokonanoda` 文本的呈现面（表达式/签名 hover 的 ` ```text ` 围栏、声明 hover
  内联签名、补全 detail/文档、诊断内嵌类型、hints/quick-fix 预览、练习树 tooltip）
  仍各自为政，且未列入 TODO。已补 `docs/HANDOVER.md §3 A″` + `ROADMAP`（I9）+ 
  `docs/design/goal-rendering.md §7`；验收原则：着色只来自 `front::semantic`。

- 2026-09-15（七十二）：**`match` 模式编译器 v1（嵌套/字面量/通配/守卫）**——把
  「每构造子一条 arm（`arm_by_ctor`）」换成**有序 arm + 列式模式编译**：模式支持
  通配 `_`、绑定变量、嵌套构造子（`| some (succ k) =>`）、Nat 字面量（`| 0 =>`/`1`/…
  脱糖为 `succ^k zero`）、`Bool` 守卫（`| succ k if p =>`，假则落到后续 arm）。
  实现走**源到源 canonical 化**（`compile_pattern_body` 生成嵌套 `Expr::Match`，
  每层复用既有 motive/IH/level/recursor 构造，不手搓 de Bruijn；参数化字段先代入
  参数）。语义：首个匹配者胜；未知裸名按 Lean 当绑定变量（带子模式才 bad-arm）；
  覆盖不全/守卫无兜底 → `elab-match-non-exhaustive`。front +8 / CLI +3 / 课程
  unit5 嵌套模式节 + 练习 9（golden `(10,8,4)→(11,9,6)`、汇总 `checked 54→55 /
  open 41→42`）。版本 **0.42.0**（新语法 minor）；设计 + as-built
  `docs/design/match-patterns.md`。已知限制：`as`/or 模式、多 scrutinee、
  `if/then/else` 表达式不做；嵌套/守卫下依赖子目标类型退回常量（保守）。

- 2026-09-15（七十三）：**呈现面高亮统一（0.43.0）**——用户追问「各个地方的高亮
  统一」；0.40.0 只统了 goal 状态。统一手段：凡渲染 `.sokonanoda` 文本一律
  ` ```sokonanoda ` markdown 围栏（LSP `code_block`/`CODE_LANG`，扩展
  `codeMarkdown`），着色只来自 `front::semantic` 的 TM 语法/语义 token。覆盖：
  表达式/签名 hover（原 ` ```text `）、声明 hover（签名/洞期望类型/目标态）、
  tactic hover 的 tactic 片段、半表达式 hover 的推断类型与目标、补全
  `documentation`、练习树 tooltip。刻意保持纯文本（VS Code 不渲染 markdown /
  行内代码无语言）：诊断消息、inlay hint、TreeItem.description、CodeAction 标题、
  散文里的单词引用。契约：`code_fences_always_use_the_sokonanoda_language`、
  `rendered_language_text_uses_the_sokonanoda_fence`。版本 **0.43.0**；见
  `docs/design/goal-rendering.md` §7。

- 2026-09-15（七十四）：**Infoview 声明类型提示 + 点击跳转（0.44.0）**——用户要求
  「声明除名字外用小字写出类型做提示，注意排版（保持每行一个声明），并支持鼠标
  点击跳转」。落地：`soko/goals` 每条声明增 `ty` + `ty_runs`（复用
  `front::semantic` runs，与 goal 同一分类源）；Infoview 声明项一行名字 + 徽标、
  下面一行 `.decl-ty`（0.78em/暗色/等宽/单行省略）按 `tok-*` 着色；点击 post
  `focusExercise` 带 `range`，扩展在聚焦练习树外把编辑器光标移到该声明并
  reveal。测试：LSP `goals_request_*` 断言 `ty`/`ty_runs`、扩展契约
  `infoview_declaration_list_shows_types_and_jumps`。版本 **0.44.0**。

- 2026-09-15（七十五）：**应用位置 binder 类型推断（0.45.0）**——ROADMAP I6
  elaborator 最后一项。`fun x => …` 在有期望类型时早已可推断；本轮补**无期望
  类型**的应用位置：`annotate_application_lambda` 展平 spine，用 kernel-backed
  `judge_infer` 从实参类型推断未注解 binder，源到源改写后交回正常路径（支持
  柯里化 `(fun x y => x) a b`）。实参不足仍报 `elab-untyped-binder`。front +3 /
  CLI +1；文档 architecture + `elaborator-let-match.md` as-built + TESTING。
  版本 **0.45.0**。

- 2026-09-15（七十六）：**`match` 作为 tactic（0.46.0）**——ROADMAP I6/§3 B 的
  “`by` 块内用 `match`”。`by` 白名单加 `match`：`match c with | p => <项> …`，
  臂体是项（同值位 match）、以当前目标为期望类型判定，等价 `exact (match …)`。
  根因修复：`judge_terms` 合成文件原 `src` 为空，`match` 宇宙查询拿不到前缀
  （`Color` 等），报 `elab-match-no-expected-type`；改为把真实 `prefix_src` 作为
  文件 `src`、合成声明 span 放到前缀之后（副产品：`by exact match …` 也可用）。
  测试：parser（白名单 + 降到 Exact）、front ×2、CLI ×1；文档 by-tactics/
  architecture/TESTING。版本 **0.46.0**。臂体内再写一串 tactic 为后续可选扩展。

- 2026-09-15（七十七）：**带索引归纳（0.47.0）**——ROADMAP I6/§3 B 最后一项。
  `inductive Vec (A : Type) : Nat -> Type`（参数 + 索引）可声明、派生 recursor、
  `match`（常量结果类型）。索引 = `ty` 在 params 之外的 Pi 望远镜（内核契约）；
  前端补 `num_indices` 元数据、index-aware `derive_recursor`（motive 先绑索引、
  major 在索引之后）、以及 match 的索引实参/索引化 motive；顺带修既有 latent
  bug（字段类型引用前面字段时按用户绑定名改名）。边界：结果类型依赖索引不做。
  front +3 / CLI +1 / 课程 unit5 带索引 Vec 节 + 练习 10（golden
  `(11,9,6)→(13,10,7)`、汇总 `checked 55→57 / open 42→43`）。版本 **0.47.0**；
  设计 as-built `docs/design/indexed-inductives.md`。

- 2026-09-15（七十八）：**编译结果缓存 + Infoview 细节（0.48.0）**——用户四项：
  (1) Infoview 类型小行允许换行（原单行省略看不全）；(2) 目标用 `⊢` 开头；
  (3) 点击 Infoview 跳转未生效（点 webview 后 `activeTextEditor` 为空）；
  (4) 设计类似 Lean4 的编译结果文件，避免文件多了打开即编译慢。落地：`.decl-ty`
  改 `pre-wrap`；goal 前缀 `⊢ `；plumb 文档 uri + `jumpToRange`（visibleTextEditors
  优先，不依赖 activeTextEditor）；`crates/lsp/src/cache.rs` 以稳定哈希
  (编译器版本, prelude 模式, 源文本) 落盘内核 `DocumentReport`，`refresh` 命中即跳过
  重编（`SOKONANODA_NO_CACHE=1` 关闭 / `SOKONANODA_CACHE_DIR` 重定位；内核仍是唯一
  判定者）。front 报告类型加 serde。版本 **0.48.0**；设计 `docs/design/compile-cache.md`。

- 2026-09-15（七十九）：**共享编译缓存 + `sokonanoda build` + Infoview 稳定/反馈 +
  高亮单一起源（0.49.0）**——用户四项：(a) Infoview 面板"点几次才出现、很不稳定"，
  要求面板 UI 必达、数据可"渲染中"/显示编译进度；(b) 去掉没生效的声明点击跳转，
  名字后加小字行号；(c) hover 高亮与 Infoview 未收拢；(d) `sokonanoda build` 之类
  命令配合缓存。落地：缓存下沉 `front::compile::cache`（report+output，key 含版本/
  构建指纹/模式/源文本）+ `build [--json|--clean]`，`course`/`--json` 复用（冷热输出
  一致）；Infoview 视图去 `when` + `visibility: visible` + `onView` 激活 + provider
  **先于**慢解析注册 + `status` 反馈 + 骨架；声明行号 `L<n>`；`SemanticKind::tm_scope`
  单一表 + hover 由 runs 投影 + TM/CSS/LSP 三处穷尽测试；平台限制写入
  `docs/design/highlighting.md`。版本 **0.49.0**。

- 2026-09-16（八十）：**Infoview 自研调色板（主题 token 解析回退，0.50.0）**——用户指出
  Infoview 里 `Type`/`Prop` 没高亮、参数色与编辑器/hover 不一致，问能否与主题自动对齐。
  调研：VS Code 无稳定 API 暴露主题 token 色（2026-06 仅有 proposal #319754/#319753）；
  唯一路线是自己复刻主题解析（读活动主题 JSON + include/tokenColors/semanticTokenColors +
  `editor.tokenColorCustomizations` + TextMate 特异性匹配）。已实现完整解析器后被用户判定
  **代价过大** → 回退。最终：**Infoview 自研固定调色板**（`--soko-*`，按
  dark/light/high-contrast 各一套、贴近 Dark+/Light+ token 色），`.tok-*` 只读
  `--soko-*`（删除 symbolIcon 优先链，修掉 `Prop` 回退成前景色的根因），任何主题必有着色；
  workbench 前景/背景仍走 `--vscode-*`。测试
  `infoview_palette_colours_every_kind_with_a_guaranteed_fallback` + `test-webview.js`。
  版本 **0.50.0**；决策与边界写入 `docs/design/highlighting.md` §3b（Infoview 与
  编辑器/hover 颜色永不逐像素相同）。

- 2026-09-16（八十一）：**`by` 块换行分隔 tactic（0.51.0）**——用户要求像 Lean4 一样
  支持「分号 **或** 回车换行」两种分隔，以便省掉行尾 `;`。规则：解析 tactic 时若下一
  token 在更晚的行且是 tactic 关键字（`intro/exact/apply/assumption/rfl/match/sorry`），
  当前表达式结束（否则 `exact f` 换行 `apply g` 会被贪婪读成应用 `f apply g`）。
  实现 `Parser.by_depth` + `starts_atom` 边界 + `parse_by_block` 双分隔；仍未引入缩进敏感。
  测试 parser 5 项 + CLI 1 项；`playground.sokonanoda` 的 `forall_and` 去掉行尾 `;` 作活样例；
  边界/歧义写入 `docs/design/by-tactics.md` §11。版本 **0.51.0**。

- 2026-09-16（八十二）：**开发流程规范（用户要求）**——(a) **VS Code 侧与 `skills/` 必须随
  用户可见改动同步更新**（门面同步从 README/description/CHANGELOG 扩展到 `skills/` +
  `AGENTS.md` + `docs/vscode-dev-guide.md`）；(b) **所有开发计划都要考虑 code agent 适配**
  （机器可读输出、skills 覆盖新能力、命令可直接执行、`AGENTS.md`/`docs/HANDOVER.md` 同步）；
  (c) **skill 里的命令要尽量少 token 且可直接执行**——能写一条确切命令就不要写成建议或
  散文描述。已落 `AGENTS.md` 收尾义务、`docs/vscode-dev-guide.md` §7 同步触发器、
  `skills/sokonanoda-dev` 工作流；并完成 0.40–0.51 三个 skill 的同步补课。

- 2026-09-16（八十三）：**课程大纲重构（设计 + P1，0.52.0）**——用户要求重新拆解
  `course/`、全面深入调研形式化证明教材并设计教学大纲。交付
  `docs/design/course-syllabus.md`：3 份并行调研（Lean 系 TPIL4/MIL/NNG/FPL/Lean4Game +
  学习者障碍研究；Coq/Agda/Isabelle/Idris + 传统证明教材 SF/PLFA/Concrete Semantics/
  TDD-Idris/Velleman/Hammack/Solow/Chartrand；本仓库现状与平台约束审计）、共享骨架与
  三处分歧、12 个可偷教学装置、障碍对策、缺失能力（rw/simp/cases/induction/have/
  structures/typeclasses/Iff/经典逻辑）的替代路线、硬约束、三套候选大纲、**锁定的 10 单元
  目标结构**（`by` 提前到 #4、拆 U5 为 #6/#7、新增 #9 关系与联结词、#10 读证明与综合）、
  P1–P4 阶段。本轮执行 **P1**（不改结构）：U4 增 2 题读 `#check` 判类型 + 1 题先预测再证；
  U6 删与 `by_ex1` 重复的 `by_ex5`；U3 去歧义；全单元 hint 去泄题（关键件只写触发条件+
  引理名）；删 U5 过期断言；solutions 同步并修 EN unit4 漂移；新增
  `solution_covers_every_canvas_exercise`、`en_solutions_match_chinese_event_counts`
  （画布**与** solutions 都比、含 `expr.typed`）；golden 同步（汇总 `open 43→45`）；
  `teaching-session.md`/`course-status.md`/`ROADMAP` I7/`course-bilingual.md`/
  `course/README.md`/`infrastructure.md` 文档漂移修正。版本 **0.52.0**。P2/P3/P4 待做。

- 2026-09-16（八十四）：**课程大纲重构 P2（重排 + 拆分，0.53.0）**——`by` 单元提到第 4
  （紧跟函数/箭头），宇宙顺延第 5，量词第 8；旧归纳单元拆成 **Ⅰ/Ⅱ**（显式归纳块+`Nat.rec`
  +`match` 非递归+递归 `match`+IH ｜ 参数化 `Option`+依赖 `match`=归纳+嵌套/字面量/通配/
  guard 模式+带索引 `Vec`），两半各自重声明 `inductive Nat` 以自足（`decl.checked` 57→58）。
  重命名 CN/EN 画布与 solutions；`course.json` 8 单元；两处 GOLDEN 与汇总、`course_json_
  lists_the_eight_units_in_order`、`cli.rs` 金值同步。门面按硬规则同步：5+ 份 docs、teacher
  skills、`editor/vscode/README.md`，并**移除根 README 与 site 三处手写单元数**（改为由
  `course/course.json` 生成，永久消除该漂移类）。验收：course/course_status/skill + 全量 CLI
  全绿；16 画布 exit 0；双语画布与 solutions 逐项相等。版本 **0.53.0**。P3（#9 关系与联结词、
  #10 读证明与综合）待做。

- 2026-09-16（八十五）：**课程大纲重构 P3（补齐锁定 10 单元，0.54.0）**——新增
  **#9 关系与联结词**（`Or` 真实归纳 + 自动 `Or.rec`；`Iff` 作为定义；`Le`/`Even`
  归纳关系 + 消去/归纳引理）与 **#10 读证明与综合**（自解释三问、formal↔informal 互译、
  评阅错证明→内核可判的修正版、跨单元 capstone）。**判分口径**：不引入新协议事件，
  每道读/评分类题都必须产出内核可判的声明（散文只在注释、不计数）。规模 unit9
  `(13,8,0)`、unit10 `(7,6,0)`；汇总 **units=10 checked=78 open=59 failed=0**；
  `course_json_lists_the_ten_units_in_order` 与 `cli.rs` 金值同步。门面同步（teaching-session
  第三课键表、course-status/course-bilingual/ROADMAP I7 标完成、course/README、infrastructure、
  teacher skills、editor/vscode/README）。**记录两个产品缺口**（HANDOVER §3 E + 课程大纲
  §2 第 11/12 条）：带索引的递归 `Prop` 归纳自动派生 recursor 被内核拒（IH 形状不符，
  `Or`/`Vec` 正常）——front 未冻结可修；`inductive` 多名字参数组不解析。版本 **0.54.0**。

- 2026-09-17（八十六）：**DeepSeek Harness 适配（用户要求）**——用户接手项目并
  明确「很多地方还没适配 deepseek harness」，要求先理解项目、分析适配点、出计划
  文档。产出 **设计 + 计划文档 `docs/design/deepseek-harness.md`**（本轮只出计划，
  不动实现）：
  - **审计结论**：产品内核（kernel / `.sokonanoda` 前端 / `--json` 事件 / 三个技能）
    与 harness 无关，可直接移植；要适配的是**接线层**——技能发现路径、斜杠命令、
    编辑器 LSP 接线、环境与二进制可达性、工具链 deny、以及只提 opencode 的文档与
    契约测试。**不需要改任何 Rust 语义代码**（唯一 Rust 侧新增是契约测试
    `crates/cli/tests/dsh.rs`）；
  - **十个差距 G1–G10**：技能不能被 DSH 发现 / 七个 `/sokonanoda/*` 命令不存在 /
    无 teacher 主 agent / `.sokonanoda` 无 LSP 接线 / 二进制不在 PATH 且 DSH 禁止
    项目改 PATH / Lean 工具链 deny 无对应物 / 33 处文档与契约测试只认 opencode /
    `AGENTS.md` 的 code-agent 适配原则缺 DSH 条目 / 无项目级 provisioning /
    用户级技能环境噪音；
  - **DSH 侧关键事实（带源码行号，见文档 §1.2）**：技能根扫描含
    `<repo>/.dsh/skills`（rank 100）与 `<repo>/.agents/skills`（200），
    **技能名本身即斜杠命令**（消息里的 `/name` 注入该技能正文，零 profile 配置），
    bundle 内相对 `references/` 受支持；frontmatter 的路由词只能写 `description`
    （`whenToUse` 是 camelCase，且只进人类 `/` 选单；**旧 camelCase 的
    `disableModelInvocation` 等会让整条技能被丢弃**）；**LSP 不在任何 shipped bundle**
    （须 profile patch 或 `--patch`），且 DSH 的 LSP 只有 4 项只读操作
    （definition/references/implementation/hover），**服务端 `publishDiagnostics`
    被显式忽略**、`workspace/applyEdit` 被拒、`soko/*` 自定义请求无消费者；
    项目/家目录 `.env` 均不得设 `PATH`，`shell-env` 只收 `DSH_*`（唯一例外是 LSP
    自己的 `servers.<id>.env`）；patch 是顶层 YAML 数组（`- id:` 覆写**整块** config
    且会丢 `!!js`；空文件会 boot 失败），`--patch` 可重复叠加且**没有项目级自动发现**；
    hooks 桥只有一个进程级 `configPath`、**不做项目发现**；**符号链接是官方同款做法**
    （DSH 仓库自用 `.claude/skills -> ../.agents/skills`，技能 watcher 默认跟随）；
  - **分阶段计划 H0–H4（每阶段独立可验收）+ H5 backlog**：H0 技能上架
    （`.agents/skills/` 放软链或薄网关，正文唯一留在 `skills/`，配契约测试防漂移）、
    H1 二进制可达（新增零依赖 Node 启动器 `scripts/soko`，解析链与 opencode 插件同
    语义 + marker 版本守卫，`AGENTS.md` Setup 改 harness 中立）、H2 LSP 接线
    （项目自带 `dsh/cordis.patch.yml` + `--patch` 用法，并**显式写清诊断不在通道内**、
    判卷一律走 CLI `--json`）、H3 命令与角色并入技能（opencode 命令与
    `.opencode/agent/teacher.md` 的正文移进 `sokonanoda-teacher`）、H4 治理
    （Lean 工具链 deny 经 hooks 桥或文档禁令、文档去 opencode 单一化、门面同步）；
  - **六个待拍板决策 D-1…D-6**、**A1–A6 验收标准**、风险表（技能双份漂移、DSH
    developer preview 变更、诊断误期待、版本漂移、profile 改坏）全部落在文档 §6–§8；
  - **本次实测现状**：`sokonanoda` 不在 PATH，缓存是旧版
    （marker `0.16.2 darwin-arm64` vs 仓库 **0.54.0**），`doctor --json` 报
    `ready:false`——历史「版本漂移致环境未就绪」的故障模式当前正在发生，H1 的
    marker 守卫正是针对它。

- 2026-09-17（八十七）：**DeepSeek Harness 适配落地（用户确认「按 H0 → H1 → H2 →
  H3 → H4 开始实现」）**——设计文档 `docs/design/deepseek-harness.md` 的五个阶段
  全部完成，版本 0.54.0 → **0.55.0**（新增用户可见的接入形态 = minor）：
  - **H0 技能上架**：`.agents/skills/{sokonanoda-teacher,dev,ci}/SKILL.md` 三个薄入口
    （正文唯一源仍是 `skills/<name>/SKILL.md`）；新增契约测试
    `crates/cli/tests/dsh.rs`（入口↔正文双向对应、DSH frontmatter 白名单、
    指向正文且路径存在、patch 形状、启动器解析链**只认版本匹配的仓库构建**）。
    实测：DSH 会话里三个技能自动出现、`/sokonanoda-*` 即命令；
  - **H1 二进制可达**：新增 **`scripts/soko`**（零依赖 Node，跨平台、可执行位入 git）
    ——DSH 无 PATH 注入也无项目钩子，必须有一个可 commit 的入口。解析链 =
    `$SOKONANODA_BIN` → **版本匹配**（跑 `--version` 校验）的仓库构建 → 缓存
    （marker 必须等于 `Cargo.toml` 版本）→ VS Code 扩展自带 → 版本锁定下载；
    **缓存过期直接拒绝运行**；网络受限时经 `curl` 走 `HTTPS_PROXY`，失败给出
    可诊断原因。`AGENTS.md` Setup 改为 harness 中立，三个技能命令统一为
    `scripts/soko …`；实测 `setup` 把本机缓存 0.16.2 → 0.55.0、`doctor --json`
    `ready:true`、`grade playground.sokonanoda` 出内核事件；
  - **H2 LSP 接线**：`dsh/cordis.patch.yml`（`lsp` + `lsp-stdio` + `tool-lsp`，
    `extensionToLanguage[".sokonanoda"]`，command 指向 `scripts/soko`）+
    `dsh/README.md`（用法、边界、常驻安装、deny 启用）。**实测**：以仓库为 workspace
    启动 DSH 会话，`lsp` 工具 hover `playground.sokonanoda:201:9` 返回内核打印的
    `theorem and_swap : forall (a b : Prop), And a b -> And b a`。同时把两条新踩的
    DSH 事实记入设计文档 §1.2：**`!!js` 必须单行**、**`baseUrl` 是 profile 目录**
    （不能用它推导仓库路径）、**`lsp` 工具只在会话 workspace 内解析 `file_path`**；
  - **H3 命令与角色**：`sokonanoda-teacher` §0 吸收角色定义与五条不可违反规则；
    `.opencode/agent/teacher.md` 瘦身为指针，**并修掉它里面违反零 cargo 硬规则的
    `cargo run` 判卷命令**；7 个 opencode 命令统一走 `scripts/soko`；
  - **H4 治理**：`dsh/hooks/{hooks.json,refuse-lean-toolchain.js}` 实现官方 Lean
    工具链 deny（命令位匹配：拦 `lake build`/`$(lean …)`、放行 `grep lean`，
    12 例实测）；`AGENTS.md` 硬规则第 2 条写明两 harness 的 deny 形态；
    `skills/README.md` 重写为多 harness 安装矩阵；`docs/design/onboarding.md`
    增 §6 DSH 对照表；`site/assets/agent-prompt.js` 安装 prompt 改 `scripts/soko`
    并说明 DSH；VS Code `README/CHANGELOG/package.json` 版本同步；
  - **验收**：`cargo test --workspace --locked` 全绿（21 个测试目标）、
    `cargo fmt --check` 绿、`clippy` 仅 kernel 既有 warning、
    `scripts/soko gate` **PASS**、site 生成与卫生检查绿。
  - **本机环境坑（非仓库问题，已记入手册）**：Xcode 27 许可未接受时
    `xcrun`/`ar` 被系统拦，`cargo` 链接必失败；绕过用
    `DEVELOPER_DIR=/Library/Developer/CommandLineTools cargo test …`，
    根治是 `sudo xcodebuild -license accept`。

- 2026-09-17（八十八）：**内核真相查询通道设计（用户要求：「从根上正确解决」+
  「更新前人留下的两个 TODO」）**——产出 **`docs/design/agent-query-channel.md`**
  （ROADMAP **I15** / H6-A…H6-E，本轮**只出设计，不动实现**）：
  - **根因判断**：内核真相今天只有 LSP 一条出口，且 `goal_decls`/`state_at`/
    `next_hole` 等**选择与判定逻辑长在 LSP 适配器内部**（`crates/lsp/src/lib.rs`
    **4256 行**，远超 §4 的 ~500 行红线），CLI/MCP 无法复用。故正确解法**顺序不可
    颠倒**：① 真相层 `front::query`（编辑器无关的类型化查询：`check`/`state`/
    `goals`/`holes`/`hints`/`reduce`）→ ② 传输（`sokonanoda query <op>` 单 JSON
    对象、零配置、所有 harness 通用；`scripts/soko mcp` + `dsh/mcp/server.js`
    MCP stdio，六工具只转发 CLI）→ ③ **同一轮把 LSP 改为调用真相层**。反过来先写
    MCP 会立刻产生第二份真相，违反硬规则（判定永远走 kernel）；
  - **关键设计**：`QueryError`/`QueryAnswer` 把"正常的没有"与"问不出来"分开
    （今天 LSP 用 `goal:null`+默认字段混合表达，agent 无法区分，是 agent 侧
    `Q3` 差距的根源）；位置真相层用 offset、适配器负责坐标转换（MCP 表面用
    `line`/`character` 与 DSH `lsp` 工具一致）；`query check` 是 `--json` 事件流的
    **新增摘要视图**，事件流契约**只增不改**；
  - **防两套真相的硬门禁**：契约测试断言 `query state` ≡ `soko/stateAt`、
    `query goals` ≡ `soko/goals`（字段级）、`query check` 计数 ≡ `--json` 事件计数；
    并有 `rg` 断言"LSP 侧不得残留查询实现"；
  - **MCP 定位**：默认**关闭**、用户显式 `--patch`/profile opt-in（DSH 把 MCP
    server 当沙箱外可信可执行代码，项目不替用户扩大信任面）；server 只做协议与
    JSON Schema，不解析 Lean、不判卷；
  - **两个 TODO 改挂**（用户明确要求）：原 `docs/HANDOVER.md` §3 E 的
    ①索引递归 `Prop` 的 recursor 自动派生被内核拒（`Le`/`Even` 现靠课程手写
    `rec`/`iota`）②`inductive` 参数不吃多名字 binder 组 `(A B : Prop)`，
    **从孤立的 front 待办改挂 H6-C**：它们直接决定查询通道的"真相"完整性、以及
    agent（主要作者）写出的合法 Lean 子集会不会被拒；两者都要求**先有"修复前红"
    的复现测试**，按 TDD 三层 + 课程 golden 同步；HANDOVER §3 E 与 ROADMAP I15
    已同步改挂；
  - **决策点 QD-1…QD-7**（真相层落点 `front::query` / CLI 单 JSON / MCP 默认关闭 /
    MCP 转发 CLI / 坐标系 / 两个 TODO 并入本轮 / 版本号策略）与 **验收 A1–A7**、
    风险表（语义漂移、两套真相、事件流消费者被打断、MCP 信任面、golden 变更、
    LSP 重构回归）落在设计文档 §10–§12。


- 2026-09-17（八十九）：**内核真相查询通道落地（H6-A/H6-B/H6-C）+ 门面收尾，0.56.0**——
  按第八十八轮的设计逐项实现并测试（用户要求：「所有任务置顶成计划一项一项完成并测试，
  多用 subagent、外部调研 + 头脑风暴」）：
  - **H6-A 真相层 + CLI**：`crates/front/src/query/{mod,types,tests}.rs` 成为
    **唯一语义源**（`check`/`state`/`goals`/`holes`/`hints`/`reduce`）；
    `QueryError` 把"正常的没有"与"问不出来"分开；`crates/cli/src/query.rs` 输出
    **单 JSON 对象**（`schema: soko.query/1`、`ok`、`data|error{code,message}`），
    退出码 **0 答上了（含 `ok:false` 与开放 `sorry`）/ 1 内核拒绝 / 2 用法**；
    契约写入 `docs/protocol.md`；
  - **LSP 改为调用真相层（同一轮）**：`soko/goals`/`stateAt`/`nextHole`/`hints` 与
    hover 的 tactic 视图全部委托；新增 `crates/lsp/src/query_map.rs` 只做形状映射；
    删除 LSP 侧的 `select_state_at`/`runs_of`/`goal_decls` 主体/重复的 `decl_name` 与
    探针逻辑，`crates/lsp/src/lib.rs` **4256 → 3988 行**；再按模块化硬规则把两个
    测试模块移出文件、抽出 `crates/lsp/src/protocol.rs`（wire 类型，159 行）与
    `tokens.rs`（semantic token 辅助，107 行）→ **lib.rs 1105 行**。
    **结构债 A5 双达标**："没有第二份实现"（`rg -n "fn select_state_at" crates/`
    只命中 front）**且** ≤1200 行。**过程需留档**：我一度没量就把"≤1200 行"作废
    （以为剩下的都是协议服务代码），量完发现 3988 行里 2638 行是 `#[cfg(test)]`
    模块——"改验收标准之前先把被验收的东西量一遍"已写进 `docs/LESSONS.md`；
  - **⚠️ 抽层出过一次语义漂移，并按"全输入对拍"抓出**：无 `by` 块的声明被错误统一成
    "根状态"（已证声明多一个目标、半成品证明丢假设）；LSP 套件 117/117 全绿也照样发生。
    方法：删除旧实现前，在 5 个画布的**每一个光标 offset** 上对拍新旧两份实现
    （709 次比较、424 处不一致全在同一分支）。修法是把该分支按协议单列（`step: -1`、
    `total: 0`，退回声明级目标 + 上下文，已闭合为空），并补 2 条 front 红先单测 +
    把 CLI≡LSP 一致性契约扩到这两个**判别性输入**。教训进 `docs/LESSONS.md` 与设计
    文档 §4 as-built 3 / §12；
  - **H6-B MCP**：`dsh/mcp/server.js`（零依赖 stdio 桥，六工具全部转发
    `scripts/soko query …`）+ `scripts/soko mcp` + `dsh/cordis.patch.yml` 的
    `mcp-sokonanoda` 行（**默认关闭**）；实测 DSH headless 会话里模型调用
    `mcp__sokonanoda__state` 成功。五个实测坑（探测进程要立刻 `-32601`、
    必须 advertise `capabilities.tools`、换行分隔 JSON、只有 `content[].text` 进模型、
    必须无状态可重启）写进设计与 `dsh/README.md`；
  - **H6-C 两个 front 缺口修掉**（原 `docs/HANDOVER.md` §3 E）：① 索引 + 字段写在结果
    箭头链里的归纳能自动派生递归子（`spine_of_codomain`）② `inductive` 参数/ctor 字段
    吃多名字 binder 组。课程 unit9/unit10 随之去掉手写 `rec`/`iota`、`Or` 参数收敛成
    `(A B : Prop)`，**golden 事件计数不变**；两条都有"修复前红"测试，并补齐 CLI e2e 层；
  - **H6-C 追加发现（三层测试的价值实证）**：修 `is_k` 的第一版把它近似成"字段数 ==
    参数数"，front 单测全绿但**内核拒了两个判别性形状**（`Both (A B : Prop)` +
    `mk (a : A) (b : B)`；以及反向的 `Q : Nat -> Prop` + `q : Q 0`，它**是** K 目标），
    被 CLI e2e 抓住。现按内核 `init_k_target` 逐字镜像
    （`is_prop_block_ty(ty) && ctor_field_binders(only_ctor).is_empty()`）并为两个反例
    各留一条测试；方法写进 `docs/LESSONS.md`；
  - **两处刻意的 wire 边界对齐**（此前无测试覆盖）：`soko/hints` 的声明命中改为与
    `stateAt` 一致的**含末尾**；由 offset 换算的 `Range` 改用 **UTF-16** 列（LSP 规范
    口径，BMP 文本逐字节相同）。扩展侧无需改动；
  - **H6-D 同步**：`AGENTS.md`、三个技能、`dsh/README.md`、`docs/protocol.md`、
    `docs/TESTING.md`、`docs/HANDOVER.md`、`docs/architecture.md`、`ROADMAP.md` I15
    as-built、`docs/LESSONS.md`、VS Code `README/CHANGELOG/package.json`、
    `site/assets/agent-prompt.js`；版本 0.55.0 → **0.56.0**；
  - **验收（全部实测）**：`cargo test --workspace --locked` **756 绿**、
    `cargo clippy --workspace --all-targets` 教学 crates 零 warning、
    `cargo fmt`（教学 crates）零 diff、`scripts/soko gate` **PASS**；
    **已发布 v0.56.0**（26 个产物：8 CLI + 8 LSP tarball + 9 VSIX + SHA256SUMS，
    marketplace 首发即成功），并**用发布产物实测**：下载的 CLI 跑 `query state`
    与仓库一致、下载的 LSP 对无 `by` 的开放声明返回 `goal='a' binders=['a','h']`、
    已闭合返回 `goal=None`。A1–A7 见设计文档 §11；已知债：
    `crates/lsp/src/tests.rs` 2567 行（拆分方案见 `docs/HANDOVER.md` §4）。

- 2026-09-17（九十）：**清除 HANDOVER §4 的 LSP 测试文件债 + 发布 0.56.1**（用户要求：
  「继续 handover 吧，完成之后再 bump」）——第八十九轮登记的两笔结构债里，
  `lib.rs ≤1200` 已在同轮清掉，本轮清剩下那笔：
  - **`crates/lsp/src/tests.rs` 2567 行 → `crates/lsp/src/tests/`**：`mod.rs` 399 行
    （31 个共享 const/fixture + `pub(crate) use` 再导出，子模块用 `use super::*;`
    取用）+ 9 个特性文件（`hover` 392 / `lenses` 366 / `navigation` 280 / `state` 262 /
    `goals` 252 / `lifecycle` 242 / `hover_brackets` 167 / `tokens` 122 / `perf` 117），
    `by_sorry_range_tests.rs`（60）原地保留；`lib.rs` 保持 **1105 行**；
  - **零语义改写**（TDD 纪律的极端情形：这次连一条断言都不能动）：107/107 顶层
    item 逐字搬移、8/8 banner 保留、规范化行流多重集 2394 == 2394、函数名 95/95
    一致、测试名各一次（76 个测试）、assert 记账 214 + 6 == 220；新增行只有模块 doc、
    `use super::*;`、`pub(crate) use` 再导出与 `mod …;`；
  - **验证**：拆分中途与最终态各一次 `cargo test -p sokonanoda-lsp --locked`
    = 117/117、fmt exit 0（首次 fmt 零改动）、clippy `crates/lsp/**` 零 warning；
    收尾 `cargo test --workspace --locked` 全绿 + `scripts/soko gate` PASS；
  - **版本 0.56.0 → 0.56.1**（内部重构、无用户可见变更）：`Cargo.toml` 与
    `editor/vscode/package.json` 同步、CHANGELOG 记"扩展行为不变"，按流程
    push main → auto-tag → release 出全部产物；
  - 文档同步：`docs/HANDOVER.md` §4（债 → 已清的最终布局）、`docs/TESTING.md` LSP 行、
    `docs/design/agent-query-channel.md` §3.2、`ROADMAP.md` I15 备注、`STATUS.md` 第九十轮。

- 2026-09-17（九十一）：**DSH 侧上架两个人工运维命令**（用户要求：「deepseek harness
  没有类似 opencode 一样的 command 机制吗？比如我这边输入 `/sokonanoda/update`
  就能执行升级命令」；同轮先按用户指示 `scripts/soko update` 把过期缓存
  0.54.0 → **0.56.1** 刷到 `ready:true`）：
  - **机制结论（本轮用当前 checkout 源码复核，不引旧笔记）**：DSH 有真命令注册表
    `ctx.commands.register`，但**只由插件注册、没有文件发现**（无
    `.opencode/command/*.md` 等价物、无 `.dsh/commands` 根）；项目仓库**零配置**
    能自带命令的唯一通道是 **skills**（技能名即 `/name`）。而命令名文法
    `^[a-z][a-z0-9_-]*$`（`packages/interaction/commands/src/index.ts:32`）、技能名
    文法 `^[a-z0-9]+(?:-[a-z0-9]+)*$`（`packages/skill/skill/src/index.ts:21`）与手势
    正则（`packages/skill/tool-skill/src/index.ts:409`）**都不允许 `/`** →
    opencode 的 `/sokonanoda/update` 在 DSH 侧只能拼成 **`/sokonanoda-update`**；
  - **人工通道已核实**：手势预置边界只查 `isUserInvocable` 后注入技能正文，
    与 `modelInvocable` 无关；`/` 菜单用 `description` 作标签（`!modelInvocable`
    时前缀 `menu.userOnly`），行数据带 `whenToUse`；
  - **落地**：`skills/sokonanoda-{update,doctor}/SKILL.md`（正文，全部走
    `scripts/soko`；判据写死"无 `STALE`、`marker` 等于 `Cargo.toml`"）+
    `.agents/skills/sokonanoda-{update,doctor}/SKILL.md`（薄入口，
    `user-invocable: true` + `disable-model-invocation: true` = **人可见、不进模型
    目录**；模型侧等价能力已在 `AGENTS.md` 与三个角色技能里）；
  - **边界**：`setup`/`version`/`check`/`gate` 暂不铺 DSH 命令形态（需要时同模式
    增补）；opencode 的 `.opencode/command/sokonanoda/*.md` 与 Rust 语义代码
    **零改动**；**不 bump 版本**（接线层改动、无产品面变化）；
  - **验收**：`cargo test -p sokonanoda-cli --test dsh --test skill` = 8 + 4 全绿。

- 2026-09-17（九十一 · 续）：**上面两个命令第一次真用就暴露了两个启动器 bug，用户拍板「1 + 2」全修**
  （用户原话：「1 + 2」——即改手册 + 改启动器）：
  - **实测现象**：在 DSH 里敲 `/sokonanoda-update`，输出 `[repo-build]`、**exit 0**，
    看起来正常，实际**一个字节都没写进缓存**；
  - **bug ①（静默成功）**：`ensure(force)` 的强制下载失败后，`resolve()` 兜底到
    "版本匹配的仓库构建"，两者都非空又不是 `cache(STALE…)`，于是只打一行
    `[repo-build]` 就 **exit 0**；`lastDownloadError` 只在"结果缺失或 STALE"分支
    才打印 → 断网、磁盘满、缓存目录只读**全都长得像成功**。修法：`ensure()` 回传
    `refreshed`（是否真的走过 download 路径），`update` 在**任一**目标未刷新时
    打 `cache NOT refreshed` + 每个失败的 `download: <原因>` + 实际使用的回退，
    并 **exit 3**；`setup`（只承诺"就绪"）行为不变；
  - **bug ②（崩栈顶掉可行动消息）**：`[cli, lsp].filter(r => r.source…)` 在
    `cli === undefined`（完全无解）时抛 `TypeError`、**exit 1 崩栈**，使紧随其后的
    "could not provide matching binaries … Next: allow network access" **永远
    不可达**（死代码）。修法：`r?.source`；
  - **根因实证**（修好后启动器自己吐出来的）：
    `download: EPERM: operation not permitted, copyfile '/tmp/sokonanoda-XXXX/sokonanoda'
    -> '~/.local/share/sokonanoda/bin/sokonanoda'`——curl 下载与 tar 解包都成功，
    只有最后写缓存被沙箱拒绝（DSH 会话的 workspace-write 不管 `~/.local/share`）；
  - **测试（红先）**：新增 `crates/cli/tests/launcher.rs` = **首个真跑 Node 的行为
    契约**（`dsh.rs` 只断言文件形状）：① 空缓存 + `SOKONANODA_OFFLINE=1` + 可用回退
    仍须 exit 3 + `cache NOT refreshed` + `download:` 原因；② 完全无解须 exit 3 +
    可行动消息而**不是** TypeError；③ `setup` 允许静默回退 exit 0。三条先红后绿
    （红：exit 0 / exit 1）；
  - **手册同步（同一轮）**：`skills/sokonanoda-update/SKILL.md` 判据改为"退出码 +
    缓存 `marker` + 缓存二进制自述版本"（**明确否掉**"`source` 不含 `STALE`"这个
    会被任何回退满足的弱证据），新增"常见失败：缓存写不进去（EPERM）"与三种处置；
    DSH 薄入口、`AGENTS.md` Setup、`skills/README.md`、`dsh/README.md`、
    `docs/TESTING.md`（新增"启动器行为契约"行）、`docs/LESSONS.md`
    （"静默成功是最坏的失败"）同步；
  - **验收**：`cargo test -p sokonanoda-cli --test launcher` = 3 全绿（先红后绿已留档）；
    真实路径复验：同一命令现在 exit 3 并打出上面那行 EPERM；全量回归与
    `scripts/soko gate` 见 `STATUS.md` 第九十一轮。
- 2026-09-17（九十一 · 再续）：**「多写了一个 sorry」与「还没证明出来」必须分开**
  （用户原话：「326和327行有问题。编译器的信息应该是 sorry 没有用，而不是
  `declaration 'exists_intro_rule' uses 'sorry' (exercise not yet solved)`。sorry
  去掉你试试，就能编译通过了。差别很大，会让用户觉得没有证明出来。」）：
  - 判定走 kernel：把候选实参从应用 spine 上删掉、按原声明类型合成一条**不入
    环境**的探针声明，整条交内核终审；过了才报 `redundant-sorry`（带 hint、span
    收窄到那个 `sorry` token）。**语义不变**：声明仍然 `exercise.open`，退出码 0
    （`docs/design/redundant-sorry.md`）；
  - 用户第二轮要求：「§8 直接做那个 5 分钟实验」→ 实验一次定位真根因（**不是**
    `NamePtr` 身份，而是 `EnvLimit::ByName(探针名)` 取到 `NO_DECL` ⇒ cutoff 0 ⇒
    空环境）；随后「继续」→ 按修正后的修法落地：内核**只加不改语义**的
    `check_declar_at`/`try_check_declar_at`（进 `docs/architecture.md` §6 适配表），
    front 传 `EnvLimit::ByIndex(env_before)`（与真实声明的 cutoff 同值 ⇒ sound）；
  - 会话/LSP 侧顺带补一个真缺口：`session.rs` 曾把内核终审过的 warning 丢掉
    （只重算语法级），现按命令进快照 + 坐标重映射，LSP 该声明不再叠
    "not yet solved"（真缺口照旧报）；
  - **验收**：kernel/front/CLI/LSP/session 五层新增 6 条测试（含"真缺口不得误报"
    与"前瞻引用不得可见"两条护栏），`cargo test --workspace --locked` 全绿、
    `scripts/soko gate` PASS；用户 playground 326–328 的形状现在报
    `warning[redundant-sorry]`；洞级 `redundant` 标记在 `query goals`/`holes` 与
    `soko/goals` 两视图一致（`crates/cli/tests/query.rs` 逐字段对拍）；
  - **版本 0.56.2（patch）已 bump**：`Cargo.toml` + `editor/vscode/package.json`
    两处 + `Cargo.lock` + `editor/vscode/CHANGELOG.md` 的 `## [0.56.2]`，`target/`
    已重建；只剩 commit + push 触发 auto-tag（`docs/HANDOVER.md` §3.0）。
- 2026-09-17（九十一）：**多文件 `import` 与项目管理（设计 + 计划，I16）**（用户要求：
  「我想增加 代码import +project管理，帮我调研一下其他语言都是怎么分别处理单文件，
  和项目。项目如何维护。sokonanoda如何实现，具体执行方案是什么」）——**本轮只出设计 +
  计划，不动实现、不 bump 版本**（沿用第八十八轮先例）。设计文档 =
  **`docs/design/imports-and-projects.md`**：
  - **调研**（§2）：Lean 4 + Lake、Coq/Rocq、Agda、Isabelle、Idris 2、Rust、
    Go、Python、JS/TS、Haskell/OCaml 的"单文件 vs 项目"做法横向对比，
    外加 LSP 的项目根发现（clangd/cargo metadata/gopls/pyright/tsconfig）与
    增量缓存/接口哈希（olean/`.hi`/dune digests/`.tsbuildinfo`）两条工程线；
    结论落到"我们采纳哪些、为什么"。
  - **用户要求的四条能力**：① 单文件**零配置不变**（45 个语料文件 + 
    `playground.sokonanoda` 行为逐字节不变，缓存键与 golden 计数不动）；
    ② `import Foo.Bar` 用**真实 Lean 4 置顶语法**、模块名↔路径用 Lean 同款规则
    （`-` 非法 → 教学化 hint）；③ 项目根 = **最近祖先的 `sokonanoda.toml`**
    （空文件也合法；`--root` 可覆盖；无清单时退化为"入口文件目录 = 模块根"，
    两文件 demo 零配置）；④ 项目维护面：闭包编译（一个 arena / 一个 `EnvBuilder` /
    拓扑序 / 闭包级 prelude 一次安装 / 失败即阻断并归因到正确文件）、
    闭包哈希缓存（Merkle：依赖变 → 下游必 miss）、`build` 项目化、
    `query` 闭包化、LSP 项目根发现 + 反向后继重编 + 跨文件跳转/引用/重命名。
  - **硬边界**：**kernel 一行不改**（跨模块声明由 front 在同一个 arena 里按序
    `add_declar`，正是 `EnvBuilder` 的既有能力）；判定仍由内核终审；
    不调用官方 Lean 工具链；用户路径零 cargo（TOML 解析编译进二进制）。
  - **量化动机**（本轮 subagent 实测）：45 个语料文件 3851 行里 **1217 行（31.6%）**
    落在"名字在 ≥2 个文件出现过"的声明块内；**71 个名字有 ≥2 种定义**、
    **20 个变体从未同单元共现**（`Or` 的 axiom/inductive 两义、`Iff` 的 def/axiom、
    `And.*` 的三种 binder 类型）——**真正值得做 import 的理由是"同名不同义
    今天无法表达"，不是省行数**（最大 5 组重复一共只省 122 行）。
  - **分阶段**：P0 设计契约（本轮）→ P1 语法/模块名/resolver → P2 闭包编译 →
    P3 CLI+协议 → P4 缓存/失效 → P5 LSP/编辑器 → P6 第 11 单元 + 门面 + 发版
    0.57.0；P7（backlog）decl 级产物、`namespace`、跨项目依赖、语料重构。
  - **验收 A1–A8**（可粘贴执行）：无 import 文件 `--json` 与 HEAD 逐字节对拍、
    零配置两文件、清单/根发现、错误归因与单条阻断、闭包 prelude 四形状、
    缓存 miss/hit 矩阵、LSP 跨文件跳转与下游刷新、第 11 单元 + 协议错误码契约。
  - **待用户拍板 Q1–Q6**（每条已给推荐）：清单格式（TOML vs JSON vs 纯标记）、
    无清单时是否允许 import、prelude 模式决策者、是否做"已检查声明"的跨进程复用、
    课程语料是否同轮重构、`watch`/`soko/project` 是否 v1 就做。

- 2026-09-18（九十二）：**多文件 `import` 与项目管理落地（I16 P0–P6，版本 0.57.0）**
  （用户要求：「新产生一个git分支吧，全部按照建议，你给我完整做完一版我看看。这个
  变化比较大。」）——第九十一轮设计文档 §8 的 **Q1–Q7 全部按推荐执行**，实现在分支
  `i16-imports-and-projects` 上分阶段落 commit：
  - **交付**：`import Foo.Bar`（置顶、模块名↔路径、`-` 非法）+ 可选
    `sokonanoda.toml`（**无清单也能 import**：模块根退化为入口文件目录，对真 Lean 的
    有意分歧）+ 闭包编译（同一 arena/`EnvBuilder`，拓扑序，**kernel 一行未改**）
    + 闭包哈希缓存（依赖变必 miss）+ CLI `--root`/`--no-project`/`build`/`query`
    + LSP 多文档与**跨文件跳转** + 单元⑪「模块与项目」与可运行两文件项目
    `course/unit11-project/`。
  - **纪律**：诊断按**命令下标**归属文件（绝不按 span）；无 `import` 的文件行为
    **逐字节不变**（A1，由 CLI e2e 对拍守住）；判定仍由内核终审，不引入官方 Lean
    工具链，用户路径零 cargo；命令面新增只在 `soko`/`sokonanoda` 二进制内。
  - **LSP（P5）全做完**：多文档 + 跨文件 `definition`/`references`/`rename` +
    **改依赖自动重编译下游**（未落盘的依赖编辑经"内存覆盖"可见、进闭包摘要；
    诊断只在真的变化时才 publish）。第一版"会挂住"的根因查明为**测试写法**
    （服务端一次通知可能连发多条诊断，测试先等通知再读 socket ⇒ 死锁），修法是
    `testutil::notify_with_drain`——教训与守护位置见 `docs/TESTING.md` §5.7。
  - **留下的 P7 项（均已登记）**：`didChangeWatchedFiles`（编辑器**外**改文件不触发
    刷新）、`soko/project`、跨文件改名的"重命名文件/模块"形态、`watch` 项目模式、
    `[deps]`、`namespace`/`open`（`docs/design/imports-and-projects.md` P7）。
  - **收尾**：版本 0.56.1 → **0.57.0**（minor，用户可见新能力；`Cargo.toml` +
    `editor/vscode/package.json` + VS Code CHANGELOG）；同一轮同步
    `docs/architecture.md`、`docs/protocol.md`、`docs/TESTING.md`、`docs/HANDOVER.md`、
    `ROADMAP.md`、`STATUS.md`、三个 skills、`AGENTS.md`、`dsh/README.md`、
    `editor/vscode/README.md`、`docs/design/deepseek-harness.md`、`site/data/site.json`；
    `scripts/soko gate` PASS + `cargo test --workspace --locked` 全绿。

- 2026-09-18（九十三）：**项目层性能例行化 + 测试扩充 + 编辑器审计修复**（用户要求：
  「各个环节的性能例行化检测并记录在案，方便后续分析检查。再多增加点项目相关的测试，
  功能和性能，包括 vscode 前端会不会卡，有没有实现不对的地方」）：
  - **性能例行化**：每个阶段（front plan/digest/compile/按键/内存覆盖/缩放、CLI
    冷/热/依赖改动必 miss/`build`+`query`、LSP 项目 didOpen/按键/改依赖刷新下游/请求
    延迟）都有阈值哨兵 + 一行 `PERFJSON`；`scripts/perf-ledger.sh` 把记录连同
    version/commit/日期/宿主/`cli_profile` 追加进 **`docs/perf/ledger.jsonl`**
    （提交进仓库，供跨版本分析"哪一环退化了"），`docs/perf/latest.json` 直读；
    CI 的 Performance report 与 `scripts/perf-report.sh` 同步收录。口径与基线见
    `docs/PERF.md`「项目层与编辑器宿主」。
  - **实测结论**：教学规模（2–5 模块 × 12 声明）一次按键 **12–46ms**（编辑器无感）；
    4×20 的项目 96–123ms；LSP 每次按键只发 1 份诊断；CLI release 冷 23.5ms / 热 4.4ms；
    键盘路径由 `vscode-languageclient` 以 250ms trailing 批量合并，扩展自身零按键开销。
  - **编辑器审计修复（两处真 bug）**：① 切文件竞态——`loadDeclarations()` 在 `await`
    之后读 `this.uri`，会产出"标签是 A 的声明、点击跳 B"的树行；现在请求发起时钉住
    URI 并丢弃过期答案。② 诊断监听器全窗口、无去抖/去重——别的扩展的诊断也会触发
    `soko/goals`，项目模式一次编辑触发 2 次 goals + 2 次 Infoview 整表重建；现在按
    URI 过滤 + 150ms 去抖 + 并发合并 + 载荷指纹去重。附带：课程树缓存一次 CLI 运行
    （30s TTL；热缓存一次 ~320ms 编译 11 个单元）、下载回退的 `execSync tar` 改异步。
  - **测试新增**：`editor/vscode/test-extension-host.js`（stub 宿主的 7 例行为测试，
    接入 `npm run test:unit`；对修复前代码 5/7 会红）、
    `crates/cli/tests/project_features.rs`（11 例 CLI 项目功能）、
    4 个 front/CLI 项目 perf 例、3 个 LSP 项目 perf 例、1 个扩展契约例
    （四个 Node 测试文件必须在 `test:unit` 在册）。
  - **顺手修**：`import my-lib` 文案双横线（`my--…` → `my-…`）；
    `docs/protocol.md` 人类输出的 `error[<stage>]` 口径；设计 §6 A2 的事件契约按实现
    改口径（入口事件 only，依赖问题走诊断，§5.1 偏差④）。
  - **门禁**：`cargo test --workspace --locked` 全绿 + `scripts/soko gate` PASS +
    `node editor/vscode/test-extension-host.js` 7/7。
- 2026-09-18（九十三·续）：**教学内容 import 化 + 再跑一遍性能**（用户要求：
  「你来测试性能一下。顺便重构教学内容呢，前后import之类的，这个是不是适合一个
  外接的子项目」）：
  - **性能实测**（`scripts/perf-ledger.sh`，11 条记录进 `docs/perf/ledger.jsonl`）：
    front 4×20 项目 compile 111ms、一次按键 130ms、缩放 4×规模 ⇒ 2.8×（线性）；
    LSP 项目按键 46ms 且每次按键 1 份诊断；CLI release 冷 31.9 / 热 3.3ms；
    新增"判据前缀"成本 82.6ms/10 处 `match`。
  - **教学内容结论**：逐字重复只占 8%（232/2974 行），**整包 import 化不做**——
    单元画布的自给自足是教学属性，且 golden/镜像/课程树都按一单元一文件钉着。
    改为新增 **`course/shared/` 共享库子项目**（规范模块 `And`/`Or`/`Nat` +
    `Demo.sokonanoda` 入口 + 清单），并用 `crates/cli/tests/course_shared.rs`
    双向守护 24/12/8 份副本与规范文本逐字一致（画布零改动、golden 零漂移）。
  - **修掉两个真 bug（同一根因）**：项目模式下 `match`/`by` 的判据前缀只含入口源码
    ⇒ 看不见被导入的名字（`elab-match-no-expected-type` /
    `elab-tactic-failed: unknown identifier`）。现在按拓扑序把依赖声明文本接进前缀；
    单文件模式逐字节不变（A1）。
  - **登记未修**：项目入口里的 quick-fix（`front::suggest` 同一根因，见 §7b）。
- 2026-09-18（九十三·再续）：**单文件 vs 项目的自动区分契约**（用户要求：「单文件和
  项目文件编译器能自动区分吗？比如单文件不会找项目配置文件，能像脚本一样直接跑」）：
  - **是，触发条件是 `import` 而不是清单**：没有 `import` 的文件走单文件流水线，
    **从不发现/读取 `sokonanoda.toml`**（同目录或祖先目录里的清单再坏也无关，
    `--root`/`--no-project` 对它都是空操作）；有 `import` 才从入口目录向上找清单，
    找不到就零配置（模块根 = 入口目录）；**依赖自己的清单永不参与**；stdin 无
    `import` 时与文件逐字节一致（脚本式直跑），带 `import` 时明确提示 `--root`。
    契约写进 `docs/design/imports-and-projects.md` §4.4b，测试
    `crates/cli/tests/single_file_vs_project.rs`（4 条）。
  - **顺带修掉编辑器/CLI 的模块根错位**：LSP 曾把 `initialize` 的工作区根当模块根
    （= 跳过清单发现）⇒ 工作区里嵌套的项目在 VS Code 报 `import-not-found`，CLI 却
    正常。现在两边同一套发现规则；回归
    `crates/lsp/src/tests/project.rs::a_nested_project_resolves_against_its_own_manifest`。

- 2026-09-18（九十四）：**待办批次 1 —— 项目入口 quick-fix + 编辑器外改动自动刷新**
  （用户：「还有没有做的TODO吗？fix修复或者优化体验的设计」→ 选定批次 1 并要求
  「按照你的计划，从上到下依次改进」）：
  - **判据前缀抽成真相层**：`judge_*` 四个合成入口增 `extra_prefix` 变体（旧签名
    委托空串 ⇒ 单文件逐字节不变）；`QueryDoc::judge_prefix(offset)` 成为唯一入口；
    `importless_source` 归一为 `project::importless_source` 一份实现。
  - **项目入口恢复 quick-fix**：`suggest_with` / `probe_sub_goal_types_with` 接前缀，
    LSP code action 传前缀；并修掉第三层根因——`GoalTemplates`（refine/intro 的
    构造子索引）原先按单单元构建，现按"拓扑序前缀 + 本单元"构建。真 LSP 探针：
    `null` → `refine And.intro a b sorry sorry`。
  - **项目模式子洞探针**：`probed_report` 不再整段跳过；`query goals --probe` 在项目
    入口给出子洞期望类型（与单文件一致）。
  - **编辑器外改动自动刷新**：LSP 实现 `workspace/didChangeWatchedFiles`（扩展早已
    声明 watcher，服务端此前忽略）：只重编译闭包里含该路径的已打开文档、缓冲区优先。
  - **验收**：`cargo test --workspace --locked` 860 passed / 0 failed；项目 perf 无回退；
    新增 LSP 2 条 + front 1 条回归测试；文档（TESTING §7b 闭环、架构 §4.5、设计 P7、
    vscode-dev-guide 坑 15、STATUS 第九十四轮、HANDOVER）同轮同步。
  - **继续**：批次 2（`query` 走项目缓存 + `goals`/`stateAt` 回显 `uri`/`version`）→
    批次 3（拆 `run_pass`）→ 批次 4（`soko/project` 项目状态可视化）。

- 2026-09-18（九十四·续）：**待办批次 2 —— `query` 走项目闭包缓存 + 协议身份回显**：
  - 新增 `crates/cli/src/project_cache.rs`：`check`/`build`/`query` 共用一份闭包摘要键；
    `QueryDoc::check()` 不再二次编译（复用 `set_text` 存下的 `CompileOutput`），
    新增 `set_cached_entry` / `compiled_output`。3×12 项目实测：`query check` 冷
    49→25ms、热 37→3.4ms；`build --json` 看到同一份键的 hit。`--text` 中间态不缓存。
  - `soko/goals` 回显 `uri`/`version`、`soko/stateAt` 回显 `uri`；VS Code 扩展据此丢弃
    "答的是另一份文档"的过期答案（stub 宿主 8/8）；协议写入 `docs/protocol.md`。
  - 验收：`cargo test --workspace --locked` 862 passed / 0 failed；台账重录
    （`docs/perf/ledger.jsonl`）。
  - 继续：批次 3（拆 `run_pass`）→ 批次 4（`soko/project` 项目状态可视化）。

- 2026-09-18（九十五）：**待办批次 3 完成（拆 `run_pass`）+ 一轮项目整理**（用户：
  「可以，前三个你先做完，把项目理干净」——批次 1/2 已在九十四轮完成，本轮收批次 3，
  然后做仓库整理）：
  - **`run_pass` 拆完（≈1174 行单函数 → 三个模块）**：闭包装配件 `compile/units.rs`；
    `check.rs` 变目录模块，尾部（内核 check-then-add + 签名/cutoff + 报告装配）
    → `check/kernel_phase.rs`，命令走查（每命令一个方法）→ `check/walk.rs`。
    只动位置不动语义；单文件仍走 `Cow::Borrowed` 前缀（A1 逐字节不变）。
  - **验收手段升级**：除 862 条测试外，做**二进制对拍**——改动前后两个 CLI 跑全部
    58 个 `.sokonanoda` 文件 + `--root` / `--no-project` / stdin /
    `query check|goals|holes`，stdout 逐字节相同。方法记入 `docs/TESTING.md`。
  - **整理（代码）**：删掉只写状态 `built_inductives`（原来只被 `let _ = …` 消费）与
    `def` 开练习路径里推空 `CmdHover` 的空操作；HANDOVER 里"项目入口 quick-fix
    仍未做"的过期段落更正；模块地图/LESSONS 同步。
  - **整理（性能台账口径）**：发现项目层 perf 套件**同进程并行**跑，把单次操作成本
    放大 3–4×（同一代码：单跑 32.4ms / 串行 33–38ms / 并行 118–152ms）；台账与
    report 脚本改 `--test-threads=1`，分阶段/缩放改 `measure_best(…, 3)`，
    `docs/PERF.md` 基线表按串行口径重写并注明"跨口径不可比"。
  - **下一批**：批次 4（`soko/project` 项目状态可视化）。

- 2026-09-18（九十六）：**待办批次 4 —— 项目状态视图（`query project` / `soko/project`
  / VS Code 项目树，0.58.0）**（承九十四/九十五的批次计划，ROADMAP I16 的 P7 项
  `soko/project` 落地）：
  - **真相层**：`project::ModuleStatus {Compiled, LoadFailed, Blocked}` 让"编译过
    （可能有错）/ 加载失败（根因）/ 被上游拖住（受害者）"三者可区分；
    `query::ProjectView` + `QueryDoc::project_view()` / `project_view_reason()`
    从**已编译的**报告派生（不重跑内核），单文件 = `project: null` + `no-imports`
    （合法状态，不是错误），路径 canonicalize 成绝对路径。
  - **三个传输同一份真相**：CLI `query project`（`soko.query/1`，恒退出 0）、
    MCP 工具 `project`（`mcp__sokonanoda__project`）、LSP `soko/project`
    （回显 uri/version，走 `focus_request` + 未保存缓冲）。
  - **VS Code 0.58.0**：资源管理器「项目」树（新模块 `editor/vscode/project-tree.js`：
    根 = 模块根 + 清单来源 + 计数，子 = 拓扑序模块 + 状态图标 + message；点击开模块/
    开清单）+ 状态栏 tooltip 项目行 + `sokonanoda: refresh project view` 命令；
    答案指名别的文档 ⇒ 丢弃（沿用 `soko/goals` 纪律）。
  - **验收**：`cargo test --workspace --locked` **871 passed / 0 failed**
    （front 466（457 + perf 3 + perf_project 6）/ cli 217 / lsp 137 / kernel 51）；
    `node editor/vscode/test-extension-host.js`
    **11/11**；`scripts/soko gate` PASS；Rust 与扩展版本同步 0.58.0。
  - **文档**：新增设计 `docs/design/project-view.md`（§9 明确不做依赖图/写操作/
    模块级缓存）；`docs/protocol.md`（`query` op 表 + `soko/project` 小节）；
    TESTING/architecture/HANDOVER/AGENTS/skills/dsh README/vscode README+CHANGELOG/
    LESSONS 同轮同步。
  - **剩余**：只有 P7 的长尾（`[deps]`、`namespace`/`open`、`watch` 项目模式、
    decl 级产物）与 `docs/HANDOVER.md` §4 登记的结构债（大文件拆分）。

- 2026-09-18（九十七）：**真 VS Code 集成测试例行化 + 结果台账**（用户要求：
  「你配置相关套件，启动 VSCode 实际验证一下，本来就应该做成例行化检测。远程不行，
  本地例行化也可以接收」）：
  - **一条命令**：`SOKO_VSCODE_TEST_VERSION=1.138.0 scripts/vscode-e2e.sh` ——
    构建 release（被测就是发布形态）→ stage 到 `bin/<target>/`（扩展 bundled-first，
    ignore 的 `bin/` 极易变旧）→ 跑真 VS Code（`@vscode/test-cli` + `@vscode/test-electron`）
    → 记账。
  - **台账**：`docs/e2e/ledger.jsonl`（`schema: soko.e2e/1`：version/commit/dirty/
    host/VS Code 版本/用例数/exit/doctor 的服务器版本行/被测 LSP 的 sha256 前 16 位/
    裁剪日志路径）+ `docs/e2e/latest.json` + `docs/e2e/logs/<date>-<sha>.log`。
  - **本轮实测**：真宿主 **14/14 全绿**（含新增 4 条：`.sokonanoda` 语言 id 守卫 +
    项目树三条——闭包 / 单文件占位 / 缺模块根因），用时 ~1s（外加 VS Code 启动）；
    doctor 自述 `0.58.0 (pid …) == 扩展 v0.58.0 (source=bundled)`。
  - **顺带修的真问题**：① `.vscode-test.mjs` 把 `--user-data-dir`/`--extensions-dir`
    指到短路径（macOS unix socket 上限 103 字符，长仓库路径原本起不来）；
    ② 扩展的 test-mode 返回钩子**提前 return 掐掉了 `client.start()`**（集成层才暴露；
    已改为末尾返回 + 注释）；③ 新增 `SOKO_E2E_LOG` 文件日志（env 开关、生产零成本）。
  - **CI 也跑这条命令（同日追加，用户：「CI如果可以做 e2e 的测试那就太好了，多用多用」）**：
    独立 `e2e` job，矩阵 `ubuntu-latest`（`xvfb-run`）+ `macos-latest`，各自钉 VS Code
    版本，跑 `scripts/vscode-e2e.sh`；`docs/e2e/` 进 artifact，`scripts/e2e-summary.py`
    渲染进 job summary；`auto-tag` 的 `needs` 加 `e2e`（e2e 红了不发版）；删掉 `test`
    job 里重复的那条 `xvfb-run npm test`。
  - **CI 编排（用户定）**：macOS 腿**只在 push 到 main 时跑**（PR/分支只跑 Ubuntu），
    `auto-tag` 仍等整个 e2e job。最低版本 1.106.0 腿**本地验证过再进 CI**：
    2026-09-18 用 `npm_config_https_proxy=http://127.0.0.1:7890`（test-electron 只认
    这两个 npm 代理变量，不读 `HTTPS_PROXY`）在 VS Code **1.106.0 上跑出 14/14**，
    台账见 `docs/e2e/ledger.jsonl`（`b0bcba3`），随后加进 CI 矩阵（ubuntu × 1.106.0，
    每个 PR 都跑）；详细命令与预置缓存备用法在 `docs/E2E.md` §5/§6。
  - **台账回提交（用户：「验证好就让 CI 往仓库追加吧」）**：main 上的收尾 job
    `e2e-ledger`（`contents: write`）把各腿 artifact 用 `scripts/e2e-merge.py`
    合并成**一条**提交推回仓库（幂等：重复条目跳过、日志按记录名回填、按日期排序；
    `concurrency: e2e-ledger` + `fetch-depth: 0`；push 失败 rebase 重试 3 次，
    冲突则打印 `UU` 文件、abort 并报红（重跑即可）；`GITHUB_TOKEN` 推的提交不触发
    workflow。两条路径用临时 bare remote + 两个 clone 本地演练过（抢占 push 成功重试 /
    同文件冲突 → 干净 abort + 退出码 1）。
    裁剪日志名带 VS Code 版本，避免矩阵内互相覆盖。
  - **版本策略（调研）**：上游 `@vscode/test-cli` 的 `version` 默认 **stable 频道**
    （官方文档/官方 sample 均不钉具体版本）——生态惯例是跟频道；但台账要求可复现，
    故本地例行默认钉 **1.138.0**、CI 矩阵显式给版本，升级流程见 `docs/E2E.md` §5；
    声明的最低版本（`^1.106.0`）腿已本地跑出 14/14（用上面那两个 npm 代理变量解决
    老版本 zip 下载中断）并加进 CI 矩阵（ubuntu × 1.106.0，每个 PR 都跑）。
  - **缓存清理**：删 `.vscode-test/vscode-darwin-arm64-1.137.0`（898MB）、旧 VSIX×5、
    `.ruff_cache/`（.vscode-test 从 1.8G → 916M）。
  - **文档**：新增 `docs/E2E.md`（手册：一条命令、四层分工、台账字段、卡住时判读、
    版本策略、环境坑、与 CI 的关系）+ `scripts/e2e-summary.py`（台账渲染，CI 与本地共用）；`docs/vscode-dev-guide.md`（测试三层 + 坑 19/20/21 + 坑 14
    更新）、`docs/TESTING.md` 集成测试小节、`AGENTS.md` 命令面、`skills/sokonanoda-dev`、
    `docs/README.md` 地图同轮同步。

- 2026-09-18（九十八）：**合并 0.56.2 线（多余的 `sorry`）+ push 主线 —— 0.58.0 发布**
  （用户：「你来搞吧，push」）：
  - **为什么必须合并**：`origin/main` 已有 0.56.2 的 3 个 commit（`dd1902d` release
    0.56.2 / `f66fee3` 发布文档 / `74cdbda` CI skill），本线 53 个 —— 强推会丢掉
    0.56.2 的功能与已发布的 `v0.56.2`。做法：scratch worktree（`/tmp/soko-merge`）
    里 merge + 全量验证，再 `merge --ff-only` 回主线（历史保持线性）。
  - **19 个文件冲突手心合并**：内核（冻结）取远端（`check_declar_at` /
    `try_check_declar_at`，"只加不改语义"）；`open_goal` 4 参 + `PendingOp::OpenExercise`
    的 `env_before`/`redundant_probes` 并入本线模块化的 `check/{mod,walk,kernel_phase}.rs`
    （0.56.2 写在旧 `check.rs` 的探针代码手工搬运）；旧 `check.rs` 在合并树里删除；
    `Cargo.lock` 取本线后由 `cargo metadata` 重生成；文档冲突一律"两条线的记录都留"
    （第九十五轮与 0.56.2 线的第九十一轮续进 `docs/STATUS-ARCHIVE.md`）。
  - **合并暴露并修掉一个真 bug（跨模块 warning 归因）**：`redundant-sorry` 是 pass 2
    现算、带命令下标的 warning，而 `split_report` 原先只按单元重算语法级 warning ⇒
    项目入口"多写了一行 `sorry`"被静默丢掉。修法：`CompileOutput` 增平行数组
    `warning_cmds` + `push_warning(cmd, w)`（与 `event_cmds`/`error_cmds` 同款不变量），
    `split_report` 按命令下标归因；语法级 warning 钉在所属单元的命令区间上（单文件
    逐字节等价）。回归 `compile::tests::warnings_are_attributed_to_the_unit_that_produced_them`。
  - **验收**：`cargo test --workspace --locked` **888 passed / 0 failed / 6 ignored**；
    `scripts/soko gate` PASS；四个纯 Node 套件 18/7/10/11；真 VS Code
    （`scripts/vscode-e2e.sh`，1.138.0 与 1.106.0）**14/14** ×2；`scripts/e2e-merge.py
    --check`、`scripts/check-site.py` 全绿。
  - **push 前 CI 预检（修掉一个 workflow 设计错误）**：三条 e2e 腿原先挤在同一个
    job 的矩阵里、用 job 级 `if: matrix.os != 'macos-latest' || …` 表达"macOS 只在
    main 上跑"——GitHub 的 contexts 表里 `jobs.<job_id>.if` **不含 `matrix`**，该条件
    要么按空值求值、要么让整个 workflow 校验失败。拆成两个 job（`e2e` = ubuntu ×
    2 版本；`e2e-macos` = macos × 1.138.0，github-only 条件、只 main），
    `auto-tag`/`e2e-ledger` 的 `needs` 同步；`scripts/e2e-merge.py` 去重键加宿主，
    避免两条 1.138.0 腿同秒完成时被当重复丢掉。合并树先推**临时预检分支**跑一遍 CI
    （workflow 能被接受 + ubuntu 两条腿 + 其它 job 全绿）再 push main。预检立刻抓到
    两件事：① workflow 校验与两条 ubuntu e2e 腿（含 runner 上现下老版本 VS Code）
    全绿、`e2e-macos` 按预期只在 main 跑；② `test` job 里 front 的缩放哨兵**假红**
    （并行口径下 400/50 = 10.9× vs 串行 7.8×，阈值 12×）——修法是把性能哨兵的采样
    口径改成"用例间串行 + best-of-N 取最小"（阈值不动），LSP 延迟断言改 best-of-3，
    记进 `docs/CI-FAILURES.md` 与 `docs/PERF.md`。
  - **发布（已完成）**：push `main` → CI 全绿（含 macOS e2e 腿第一次真跑）→
    auto-tag 打 **`v0.58.0`** → dispatch `release.yml` → **11 job success**、
    Release **26 资产**、Marketplace 收录 0.58.0；发布产物实测（下载 → `shasum -c` OK
    → `--version` = 0.58.0 → playground 报 `redundant-sorry`、`query project` 正常）。
    `e2e-ledger` 把三条 CI 腿台账自动回提交（`docs/e2e/ledger.jsonl` 共 10 条）。
    0.56.2 的功能与 tag 都保留在历史里，0.58.0 的 CHANGELOG 补记"多余的 `sorry`
    已并入"。
- 2026-09-18（第九十九轮，用户）：**基于本项目复刻一个更大的教学项目**（对标用户
  自己的 `analysis` 仓库：Lean 4 + Mathlib、124,491 行、≈5,300 声明），先做
  **「集合论」教程**；**制作过程中持续搜集 sokonanoda-lang 自身的不足，交给另一个
  agent 实现**；要求"足够多的细分的计划"。落地：
  - 计划 = **`docs/design/teaching-project.md`**（靶子标定 + 可行性实测 + 缺口分诊 +
    缺口台账协议 + 分期 P0–P7 + 验收 + 待拍板 D-1…D-6）；
  - 台账 = **`docs/gaps/`**（`ledger.jsonl` + `repro/` 最小复现 + `WO-*.md` 工作单；
    协议见设计 §6，含"缺口即测试"的防漂移机制）；
  - 本轮实测出 **10 条课程驱动缺口**（G-01…G-10，其中 5 条 blocker：`query check`
    对解析失败假绿、开练习签名不校验、构造子无命名空间、Prop+Type 参数归纳被内核
    断言拒绝、`course` 聚合不认 `import`），并给出集合论可行性的正向探针
    （`docs/gaps/repro/OK-set-spike.sokonanoda`，11 声明全绿）；
  - **落点/语法增量/双语/站点/交接方式等待用户拍板**（设计 §11 的 D-1…D-6），
    未拍板前不动 `course/` 与任何 Rust 语义代码。
- 2026-09-18（第九十九轮续，用户）：「**这个项目可能要完全单独一个文件夹，甚至本身
  一个子仓库。也需要调研更多集合论的教程。**」落地：
  - **D-1 落点已定：独立文件夹 / 独立仓库**（同级于 `sokonanoda-lang`）——设计 §3 重写
    （三种形态取舍、§3.4 课程仓骨架、§3.4 仓库边界表、§3.5 建仓清单），子仓库
    （submodule）形态留作将来的只读挂载；
  - **建仓做成一条命令**：新增 `scripts/new-course-repo.sh`（生成 README/AGENTS/版本源/
    清单/共享库 `lib/{Logic,Set,Demo}`/`units/`/`gaps/`/vendored 启动器/本地门禁/CI，
    可选 `git init`），**已实测**：样例仓 3 checked / 0 failed / exit 0，无二进制时给
    G-11 人话提示；
  - **新实测出两条独立仓库的入场券缺口**（台账 G-11/G-12，均为 blocker）：
    G-11 启动器在非 Rust 仓库没有版本源（只读 `Cargo.toml`，课程仓拒绝运行且只报裸
    `ENOENT`）；G-12 **相对路径入口 + 祖先清单 ⇒ 模块根退化成空路径**，`units/` 里
    任何 `import` 都报 `import-not-found`（同文件换绝对路径即绿，`query project`
    的 `root=''` 是证据）——正是课程仓的布局，修好前课程仓门禁一律用绝对路径；
  - **集合论教学调研已完成**（两路，~2,200 行，落 `docs/notes/settheory-survey/`）：
    教材顺序之争六组取证（Hammack/Macbeth/Avigad/Velleman/Solow/Cummings + analysis §3
    逐节解剖）、证明助手先例（MIL/MoP/L&P/FM/LPA）、**`djvelleman/stg4` 集合论游戏
    8 世界 51 关**（可偷：逐步解锁语法 / 三层 Hint / `Branch` 死路）、内核实测
    （`repro/lib.sokonanoda` 29 checked / 0 failed；探针产出台账 **G-13**：
    `axiom` 不吃 binder 参数表）；明确标注未核实项（**中文教材目录一份未取到**、
    教育研究文献未取证、Jech/Kunen 未取到、其他证明助手大面积未核实）；
  - 综合结论已落 **`docs/design/set-theory-syllabus.md`（v2，锁定 12 单元）**：
    每单元"必证/必破"、记法引入顺序与无记法替代、**课程写作红线五条（全部实测）**、
    与缺口台账的联动表、5 条待拍板 D-S1…D-S5。

- 2026-09-18（第九十九轮再续，用户）：「**你的教程出的题目，在做的时候，你会发现要补充很多
  其他的定理，边边角角的定理，这个是在其他的教程里头会认为是天然应该知道的，或者说是标准
  库里已经实现的。这个我感觉有点暴力，所以我需要你去用这个项目，然后重新做一遍，然后才能
  把这些暴力给消除掉。你要记录下这些其实是需要实现的，其实是没有的。**」落地：
  - **真做一遍**：卷 I 单元①② 试做稿落 `docs/gaps/spike/README.md`——`lib/Logic`(26) + `lib/Set`(40)
    共 **66 条标准库**、`units/` 两个单元 **16 道练习**（含解答），**全部真内核判卷 0 failed**；
  - **新缺口 7 条**：**G-14** 一个声明只允许一个宇宙层级 binder（跨宇宙引理写不出来）；
    **G-15** 内核拒绝的诊断 span 与出错声明范围不一致（试做时连修错两次才发现）；
    **L-01/L-02** prelude 缺 Lean core 的 16 条逻辑与等式骨架（True/False/And.elim/Or.elim/
    Not/absurd/Iff/Eq.symm/trans/congrArg）；**L-03** `Eq.subst` motive 只能 `α → Prop`
    ⇒ Type 层重写（`Eq.mp`/`cast`）不可表达；**L-04** 课程标准库缺 8 条 Set 定义展开引理；
    **L-05** 方法学：16 条"内容型"引理被误放进库（应当是练习）；
  - **锁定分层判据**（新设计 `docs/design/course-stdlib.md`）：**L1 prelude**（Lean core 级）/
    **L2 课程标准库**（定义展开级）/ **L3 单元练习**（有数学内容的一切）；判据 =
    **"Mathlib 有 ≠ 我们不该练；Mathlib 有且没有数学内容（纯定义展开）才归库"**；
  - **台账扩展**：`kind` 新增 **`library`**（标准库欠账，带 `owner`: `prelude`/`course-lib`/
    `exercise` 与 `lean_names`），共 20 条；`scripts/gap.py check` 全绿；
  - **调研补全**：其他证明助手（Isabelle/ZF、Metamath、Mizar、Coq、Agda、ACL2、HOL）
    已第二轮取证，结论 **"没有任何主流证明助手提供独立的集合论教学层"**（生态位确认）。

- 2026-09-18（第九十九轮三续）：**学习障碍实证调研完成**（`docs/notes/settheory-survey/
  learning-difficulties.md`，2874 行 / ~190 条来源，逐条标 [F] 全文 / [A] 逐字摘要 /
  [M] 仅元数据）。落地：`docs/design/set-theory-syllabus.md` §1.3 改写为**实证版**
  （每条教学动作指到出处）并新增 **§1.4 引用纪律**——① 有序对与选择公理**无实证研究**
  （ERIC 实测否定结果），不得写成"研究表明…"；② 不得过度断言（Sfard 1991 仅元数据、
  Tall 措辞未核实、APOS 的实证基础是微积分而非集合论）；③ 两条流行线索是幻觉
  （Piatek-Jimenez 2004、Cusi & Malara 2007 均不存在）；④ **中文语境空白**（实证全为英文）。
  试做稿同步：单元② 新增第 7 题（**单元素 `{a} ∈ {{a}}`**，实证上最脆弱的一点）——
  画布 10 题、解答 10 checked 全绿。

- 2026-09-18（第九十九轮四续，用户）：「**进硬规则，课程建在当前项目内新建一个文件夹**」落地：
  - **D-S6 → 硬规则 10**：课程标准库三层分界（L1 prelude / L2 课程标准库 / L3 单元练习）
    与判据「**Mathlib 有 ≠ 我们不该练；Mathlib 有且没有数学内容（纯定义展开）才归库**」
    写进 `REQUIREMENTS.md` §2 第 10 条（并附引用纪律）；
  - **D-1a → 课程建在语言仓内**：`courses/set-theory/`（README / AGENTS / `sokonanoda.toml` /
    `course.json` / `lib/` / `units/`+`solutions/` / `gaps/` / `tools/check.py`）；
    独立仓库形态改为"将来抽取"（清单一节 + `scripts/new-course-repo.sh` 保留）；
  - **内容搬家与改名**：`lib/Set` 改用 **Loogle 取证名**（`Set.notMem_empty`、
    `Set.mem_powerset_iff`、`Set.mem_sdiff`、`Set.Subset.refl/trans/antisymm`…）；
    **L3 拆分第一刀**——6 条内容型引理移出 lib 进单元②（台账 L-05）；
  - **门禁**：`python3 courses/set-theory/tools/check.py` = lib 自检 + 2 单元 + 2 解答
    = **22 checked · 16 open · 0 判负**。

- 2026-09-18（第九十九轮五续，用户）：「**尽可能多的拆分任务，让子代理闭环实现并测试**」。落地：
  - **两轮 workflow 共 31 个子代理**：第一轮 2 个基础库 + 10 个单元（实现）+ 5 个独立复核
    （重判/清单核查/就地修），第二轮 10 张 WO + 4 份设计/调研；
  - **课程从 2 单元长到 12 单元**：`courses/set-theory/` 现为 8 个 lib 文件
    （Logic/Set/Exists/Prod/Rel/Fun/Image/Equiv + Demo 自检）+ 12 画布 + 12 解答，
    门禁实测 **34 目标 · 308 checked · 93 open · 0 判负**；93 道练习的三段 `soko:hint` 全部可被
    hints 工具完整读出（复核发现并批量修掉"提示换行被解析器截断"的系统性缺陷）；
  - **10 张工作单落盘**：`docs/gaps/WO-001…WO-010.md`（G-11/G-12/G-10/G-01/G-02/G-03/G-06/
    G-13/G-14/G-15），台账对应 10 条转 `wo-filed` 并挂上 `wo` 指针；
  - **子代理纠正了主线 agent 的两处错误（已复核）**：① `{u, v}`（逗号）**可以**用 ⇒ G-14 降为 nice；
    ② 我记的 G-15「内核错误 span 漂移」是**假缺口**——把**字节 offset 当字符下标**切字符串造成的
    （已改名为「query check 的 failed[] 只给裸字节 offset」，教训进 `docs/LESSONS.md`）；
  - **新增 4 条缺口**（都来自实测）：G-16（启动器在版本未知时 exec 陈旧二进制，绕过"过期即拒绝"）、
    G-17（`query goals`/`holes` 对解析失败假绿）、G-18（`def f.{u}` 被静默解析成 `f.`）、
    L-06（无累积性 + `Exists.elim` 只能 Prop ⇒ 等势只能 Prop 值、取数据的引理写不出来，课程统一改数据版）；
  - **设计/调研产出**：`docs/design/prelude-l1-proposal.md`（L1 28 个名字 + 三件套 + GOLDEN 预测）、
    `docs/design/course-gate-in-ci.md`（门禁不进 golden、避开 G-10/G-12/G-15、CI 挂 test job 新 step）、
    `docs/notes/settheory-survey/chinese-textbooks.md`（中文教材/大纲：关系先于函数 6:2、
    哈工大 MOOC 函数先、复旦讲义有 Russell 逐字证据，并**更正**"徐明曜/赵春来《集合论》"查无此书）、
    `docs/design/course-stdlib.md` 更新到 8 模块真实计数；
  - **大纲回填实测偏差**：单元③ 的 D 类原稿有数学错误（`A ∈ 𝒫A` 是真命题）已勘误、
    单元⑦ 的选择公理边界、单元⑨ 的 Prop 值等势、单元⑫ 的链条缺基数一环。

- 2026-09-18（第九十九轮（语言线），用户）：「实现工作单里描述的那个缺口」——
  **G-06 / WO-007 落地：课程聚合认 `import`**（用户可见行为变化）。
  `sokonanoda course <course.json>` 对**有 `import` 的单元**走项目闭包：与
  `grade`/`query check`/`build` 同一份闭包、同一个模块根、同一个
  `ProjectPlan::digest` 摘要键；计数只取**入口模块**（依赖的声明不算单元成绩），
  `failed` = 闭包内所有模块 `events.errors` 之和 ⇒ `failed == 0` ⇔
  `grade <该单元>` exit 0。模块根 = 单元最近的 `sokonanoda.toml`（嵌套子项目优先），
  没有则回退 **`course.json` 所在目录**；清单路径先 `canonicalize`（与 cwd 无关）。
  **无 `import` 的单元逐字节走单文件** ⇒ `course/course.json` 的两处 GOLDEN
  （`crates/cli/tests/{course,course_status}.rs`）一字未改。`course.unit` /
  `course.summary` 键集与退出码契约不变（progress is not an error）。
  实测：`node scripts/soko course "$PWD/courses/set-theory/course.json" --json`
  = `{"checked":63,"failed":0,"open":93,"units":12}`（修前 `checked:1 / failed:63`，
  12 单元全假红）；`python3 courses/set-theory/tools/check.py` 仍 0。
  回归 `crates/cli/tests/course_project.rs`（8 例）；复现
  `docs/gaps/repro/G06-course-import.sh` 由 exit 0 → **exit 1**（行为已变，
  `gap.py close G-06` 的前置；版本号由主线 bump 后填）。

- 2026-09-18（第一百轮（语言线），用户）：「实现工作单里描述的那个缺口」——
  **G-10 / WO-003 落地（+ 同族 G-17 顺带修）：agent 查询通道对解析失败不再假绿**
  （用户可见行为变化）。`query check` 的 `failed[]` 现在**同时**承载内核拒绝与
  parse 诊断：解析失败时 `counts` 全 0、`failed` 恰含一条 parse 诊断
  （`name: null`、`start`/`end` = 诊断 span 字节 offset）、`ok` 仍是 `true`
  （"问出来了"——答案就是"这份文本解析不了"）、**退出码 1**，与同一份文本的
  `grade` 逐项同口径。同族的 `query goals`/`holes`（原先答空数组 + `ok:true`）
  改答 `ok:false` + `error.code:"not-parsable"` + 退出码 1；"正常的没有"
  （空数组 / `navigated: null`）仍是 `ok:true`。`query state` 的形状未动。
  协议同步在 `docs/protocol.md`（`check`/`goals`/`holes` 行 + 退出码段）；
  复现 `docs/gaps/repro/G10-query-check-parse-error.sh`（重写为修后形状）与新增
  `docs/gaps/repro/G17-query-goals-holes-parse-error.sh`，修后都 = exit 1。
  事件流与两处课程 GOLDEN 未动；退出码 0→1 是有意的契约变更 ⇒ 版本级别 minor。


- 2026-09-18（第一百轮（语言线），用户）：「实现工作单里描述的那个缺口」——
  **G-11 / WO-001 落地（+ 同文件第二缺陷 G-16）：课程仓的版本钉源链 + 「无期望版本
  绝不 exec」守卫**（工具链行为变化，非 Lean 语义）。用户可见契约三处：
  ① **版本钉不再只认 `Cargo.toml`**——链 = `$SOKONANODA_VERSION`（release tag，
  带不带 `v` 都收）→ `<repo>/sokonanoda-version.txt` → `<repo>/sokonanoda.toml`
  的 `requires`（**完整 `x.y.z` 才算钉**；`0.58` 只是约束、不能当下载锚点）→
  `<repo>/Cargo.toml`；锚点是**启动器自身所在仓库根**（vendored 到课程仓即指向
  课程仓），不是 cwd，也**不沿父目录找**；所有出现的源必须一致（`major.minor`
  口径），不一致**指名文件**报错；② **解析不出期望版本就绝不 exec** 缓存/仓库构建
  （G-16：`cache(unknown repo version)` 这条静默通道删除），exit 3 + 人话（期望版本 /
  来源 / marker / 该改哪个文件）；**缓存 marker 必须与钉一致**才可执行
  （`0.58` 约束接受同 release line，`0.58.0` 锚点只认精确标记）；③ 可观测性：
  `version --json` 新增 `version_source`/`version_constraint`/`version_error`，
  `version` 不再缺键；`doctor --json` 的 `ready` 与执行守卫同判据；`setup`/`update`
  的下载 URL 用链条解出的版本（不再出现 `v?`），并共用 CLI 已有的
  `SOKONANODA_RELEASE_BASE`（下载基址覆盖）。
  **不动**：内核/front/CLI 源码、`requires` 在 front 的 warn-only 语义、
  `sokonanoda.toml` 三键封闭、`SOKONANODA_BIN`/扩展自带/离线三条既有解析链、
  release 产物形态（永不 `latest`）。同轮同步 `.opencode/plugins/sokonanoda.ts`
  （同一条链 + 同一条拒绝规则）、`scripts/new-course-repo.sh`（撤掉 G-11 兜底文案，
  门禁判据改为"版本钉解析得出"）、`skills/` 与 `docs/design/` 相关段落、
  `AGENTS.md` Setup。三层测试落 `crates/cli/tests/launcher.rs`（8 例，其中 5 例新增：
  版本钉解析 / 陈旧缓存拒绝 / 无版本源拒绝 / 优先级与冲突 / 无网络下载路径）；
  复现 `docs/gaps/repro/G11-launcher-version-source.sh` 由 exit 0 → **exit 1**
  （行为已变，`gap.py close G-11` 的前置；版本号由主线 bump 后填）。
  事件流与两处课程 GOLDEN 未动。
- 2026-09-19：**开放练习的签名纳入类型检查（G-01 / WO-004）**——修的是违反硬规则 3
  （教学语法是真实 Lean 4 的子集，填完洞的声明放进官方 Lean 依然合法）的行为：
  值位是 `sorry` 不再让签名免检。**用户可见契约**：`def`/`theorem`/`example` 的签名
  先 elaborate（失败 ⇒ `elab-unknown-identifier` 等，原来被 `.ok()` 吞掉），再由内核
  终审「是不是一个类型」（`kernel-expected-sort`）；`theorem` 另问「是不是 Prop」
  （`kernel-theorem-not-prop`）。签名不过 ⇒ 声明 `failed`、报一条 diagnostic、
  **不发** `exercise.open`（与值位 elaborate 失败同罪；不做成 warning，因为 warning
  不改退出码，课程侧就发现不了签名腐烂）。诊断 span 取**签名自身**的源范围
  （G-15：不照抄内核消息里的 span）。**判据纪律**：`exercise.open` 计数对签名腐烂
  永远是盲的——判卷只认 `decl.checked` 与 `diagnostic`。**兼容性实测**：入门课 +
  卷 I 共 158 条开放练习签名本来就合法，全仓 105 个 `.sokonanoda` 文件逐条对拍
  （新旧二进制），**新增诊断 0 条**、`decl.checked`/`exercise.open`/`expr.reduced`
  三列不变（双 GOLDEN 未改）、课程门禁仍是 315 checked · 96 open · 0 判负。
  内核（冻结快照）未改一个字节。

- 2026-09-19：**构造子的命名空间（G-02 / WO-005）**——修的是违反硬规则 3
  （教学语法是真实 Lean 4 的子集）的缺口：构造子过去以**裸名**进环境且**全项目
  唯一**，`ctor mk` 之后只有裸 `mk`、`Pair.mk` 报 unknown identifier，两个块各写
  `ctor mk` 直接撞 `duplicate declaration mk`，于是大库只能给构造子起
  `prod_mk`/`exists_intro` 这类假唯一名。**用户可见契约**：构造子的**规范名**是
  `Ind.ctor`（源名**已含点**则原样——保护 prelude 的 `Nat.zero`/`Bool.true`），
  `#check`/`#reduce`/`#print`/hover/Infoview 目标文本/refine 骨架/语义高亮都显示
  规范名；裸名降级为**解析别名**（只在解析层、绝不进内核），唯一时可解析、重复时
  报新码 `elab-ambiguous-ctor-alias`、真实声明优先于别名；源级写法（`match` 分支、
  显式 `iota`）继续按源名匹配（R3）。**这是本子集的一条扩展、不是 Lean 语义**
  （Lean 里裸 `mk` 不可解析，除非 `open` 了命名空间），日落与 37 个文件的机械改名
  一起开 WO-005b。**兼容性**：课程内容一字不改（`course/` 与
  `courses/set-theory/` 零改动），课程门禁 315 checked · 96 open · 0 判负、
  双 GOLDEN 计数不变。**归约形态变化（实测）**：源文件自带 `inductive Nat` 时，
  ctor 叫 `Nat.succ` 会命中内核 name cache 的 `NatRed::Succ` 快路径 ⇒
  `#reduce add two two` 从 `succ (succ (succ (succ zero)))` 变成混合表示
  `Nat.succ (Nat.succ (Nat.succ 1))`（逐条实测值见 `docs/design/ctor-namespace.md`
  §2.1；文本断言按实测重钉，未做机械替换）。内核（冻结快照）未改一个字节。

- 2026-09-19（续）：**Prop 结果 + Type 参数 + 单构造子的归纳（G-03 / WO-006）**——
  修的是"本来该编过的程序编不过"：`inductive Bar (A : Type) : Prop` +
  `ctor mk (a : A) : Bar A`（就是 `Exists` 的形状，Lean 4 里完全合法）被内核以
  `rejected: assertion 'left == right' failed (left: 1, right: 0)` 拒绝，于是
  `courses/set-theory/lib/Exists.sokonanoda` 只能把 `Exists` 立成**公理三件套**。
  **根因在前端派生侧、不在内核**：`derive_recursor` 的 `small_elim` 是源码近似
  （`is_prop_block_ty && 多构造子`），而内核 `large_elim_test` 对**单构造子 Prop 块**
  还要看 `large_elim_test_aux`（非 Prop 字段必须都是结果应用的语法成员）。两者不一致
  ⇒ 前端声明 1 个宇宙参数、内核要 0 个 ⇒ `subst_expr_levels` 的长度断言炸成 panic。
  **用户可见契约**：无 `rec` 的块派生的 recursor 带**内核算出的**宇宙参数个数——
  `Bar` 这种形状是 **0** 个（motive 落 `Prop`，`Bar.rec` 不接受宇宙参数），
  而 `inductive P9 (A : Type) : A -> Prop` + `ctor c9 (a : A) : P9 A a`（字段**就是**
  结果的索引）仍是 **1** 个；字段类型是 `P -> Q` / `forall (x : Nat), P` /
  具名 `def … : Prop` 的块**保持** 1 个（这些是 Prop **值**，源码近似会误判）。
  **实现纪律**：判据逐字镜像内核那几行，且"字段类型是不是 Prop 值"这步**问真内核**
  （`judge_infer` 问排序），不写第二套近似；不做"试探 + 回退"（WO 明令禁止——那等于
  把判据推给内核，会把真正的教学错误磨成同一条断言）。**顺带修掉一个既有 oracle
  bug**：`judge_infer` 取事件流里**第一条** `TypeChecked`，而它的查询是**最后一条**
  命令——前缀里只要已有一条 `#check`（课程/playground 常见），拿回的就是旧答案；
  实测前缀有 `#check Nat` 时 P10/P13 立刻退化成 `left:0/right:1`。改为按 `event_cmds`
  取最后一条命令的事件（不做文本比对）；该 bug 同样影响 `match` 的 motive 层级查询。
  **兼容性**：`course/`（入门课）两份 GOLDEN 计数不变（本 WO 只让"今天必定失败"的
  输入变绿，不改任何已通过块的 recursor 形状）；卷 I 课程门禁 **315 checked · 96 open ·
  0 判负**，与修前**同数**（WO 正文写的 296/93 是陈旧口径，实测基线见
  `docs/design/ctor-namespace.md:73`）；`cargo test --workspace` 全绿（972 passed /
  0 failed）。复现件
  `docs/gaps/repro/G03-prop-type-param-inductive.sokonanoda` 重写成修后形状（**块本身
  逐字不动**），`scripts/soko grade` 退出码 0。**课程侧出口留到下一轮**：
  `lib/Exists.sokonanoda:68-71` 的公理三件套可升级成真归纳（`Exists` + `intro` +
  自动派生的 `Exists.rec`），名字与签名逐字不变 ⇒ `units/unit06…unit12` 预期 0 改动。
  内核（冻结快照）未改一个字节。
- 2026-09-19（第一百〇四轮，语言线）：**L-01/L-02 落地 —— L1 prelude**（设计
  `docs/design/prelude-l1-proposal.md`，台账 L-01/L-02 关账于 0.59.0）。prelude
  在 Full 模式下自带 Lean core 的逻辑与等式骨架：**30 个顶层名字**（`PRELUDE_NAMES`
  12 → 42），分 B1–B7 七族，`And`/`Or` 是**真归纳块**（点号构造子，可 `match`）。
  **让位规则**（本提案的核心）：粒度 = 族、依赖闭包（B5→B2、B6→B3、B7→Eq）、
  触发集合 = 整个闭包的顶层名字并集（含构造子/递归子）；谁声明谁拥有 ⇒ 入门课
  单元①④⑤⑧⑨⑩⑪ 的"自建逻辑骨架"教学**一个字不用改**，`course_shared.rs` 44 份
  副本一致性测试未改而全绿。`PRELUDE_NAMES`（补全/材料）与 `PRELUDE_NEVER_YIELDS`
  （碰撞检查豁免面，只含 Nat/Bool）**拆成两个常量**。parser 白名单零改动。
  **课程用例**：单元② 中英画布 + 两份解答加 `eq_symm_demo` 与"两解对照"hint；
  两处 GOLDEN **据实重算**（`unit2 = (3,5,2)`、summary `checked 85→86`），并发现
  第三处（`cli.rs` 的 warm-cache 断言）。**课程侧兜底保留**：`courses/set-theory/
  lib/Logic.sokonanoda` 的 28 条**一条没删**，文件头注明"prelude 现在自带哪些"，
  卷 I 门禁复跑 315 checked · 96 open · 0 判负（与落地前同数）。**P4（课程仓 74 处
  `inl`/`inr` 项位改点号名、`lib/Logic` 退化成空壳）未做**；**L-03 仍 open**
  （Type 层重写：`Eq.subst` 的 motive 仍是 `α -> Prop`）。三层测试：front 10 条 +
  CLI 4 条 + 复现件 2 个（`docs/gaps/repro/L01-*.sh`、`L02-*.sh`，修后形状 = exit 1）。
  内核（冻结快照）未改一个字节。

- 2026-09-19（第一百〇八轮，用户）：「**设计文档里的东西都做了吧。**」落地：把 `docs/design/` 各篇
  「未做 / 第二刀 / 残留边界」里不违反硬规则的项目全部实现——**记法第三刀**（binder 记法 `∃ x, p`、
  记法重载、`scoped`/`open scoped`、集合字面量 `{a}`/`{a,b}`、一元记法实参位免括号）、
  **`namespace`/`open` 扩展**（`only`/`hiding`/`renaming` 子句、`open … in`、`export`、遮蔽 warning）、
  **层级算术 `u+1` + `Eq.mp`/`Eq.mpr` 宇宙多态 + `cast`/`Eq.ndrec`**、**编辑器词表同轮同步**
  （`abbrev`/`prefix`/`postfix`/`binder_notation`/`scoped`）、**课程多清单聚合 + 成本台账**
  `docs/courses/ledger.jsonl`（`--ledger` 默认关）。保留为**有理由的边界**（都有实测）：累积性与
  Prop 大消去（内核冻结）、源码级 print-back（内核 pp）、`section`/`variable`（无隐式参数插入）、
  `u+v`/`max`（内核无公开构造入口）。版本 **0.61.0**（两处 + 课程 `requires`）；
  `scripts/soko gate` exit 0（**1163 passed / 0 failed**；课程 36 目标 · 329 checked · 99 open ·
  0 判负；台账门禁全绿）；内核零改动。

- 2026-09-19（第一百〇七轮，用户）：「**vscode 还是没有 sokonanoda: build 或者 sokonanoda: rebuild
  的命令。你这个剩下的没做的也要做。**」落地：
  * **编辑器命令补齐**：`sokonanoda: build`（`alt+b`）与 `sokonanoda: rebuild`（`alt+shift+b`，先
    `build --clean`）——把 CLI 的编译缓存预热/清理接进编辑器，事件（`build.file`/`build.clean`/
    `build.summary`）进 **sokonanoda build** 输出面板，跑完刷新练习/项目/课程三棵树；三层测试
    （静态契约 `crates/cli/tests/extension.rs`、stub 宿主、真 VS Code e2e 14 → 15 例）；
    版本 feature bump **0.60.0**（两处 + CHANGELOG + 课程 `requires`）。
  * **「剩下的」全部收口**：**G-05** `namespace`/`open`（含课程 `lib/Set` 迁移）、**G-07** 课程清单 v2
    （卷/章/先修/标签/配额 + 门禁 **G6 清单自洽**，v1 兼容）、**G-08** `abbrev`（实测与 `def` 同语义）、
    **L-03** Type 层重写（prelude **B8**：`Eq.rec` + `Eq.mp`/`Eq.mpr`）、**L-06** 累积性边界
    （内核性质不改 + 新码 `kernel-prop-not-cumulative` + 课程三条绕法）、**记法第二刀**
    （`prefix`/`postfix`、`𝒫`/`ᶜ`/`''`/`⁻¹'`/`×ˢ`、**跨 `import` 传播**）。台账 **24 条 = 22 `fixed`
    + 2 `workaround`（L-04 / L-06），`open` 归零**，`scripts/gap.py check` 全绿。
  * **验收**：`scripts/soko gate` exit 0（`cargo test --workspace --locked` **1107 passed**；
    课程门禁 **36 目标 · 329 checked · 99 open · 0 判负**；台账门禁全绿）；内核 `crates/kernel/**`
    一个字节未动。

- 2026-09-19（第一百〇六轮，收尾）：**0.59.0 发布收尾 —— 语言线五刀 + 课程门禁 + 站点页**。
  本条把这一版**用户可见**的变化收在一起（逐条的前置记录见上文各轮）：
  * **版本**：两处版本号 = `0.59.0`（`Cargo.toml` + `editor/vscode/package.json`，契约测试
    `crates/cli/tests/extension.rs::cargo_and_extension_versions_match` 守着），
    `Cargo.lock` 由 `cargo metadata` 跟上；课程清单 `courses/set-theory/sokonanoda.toml`
    的 `requires` 0.58 → 0.59。发布仍全自动（push main → `ci.yml` auto-tag →
    `release.yml` 26 资产 + VSIX ×9 + SLSA provenance）。
  * **签名受检（G-01 / WO-004）**：值位是 `sorry` 的 `def`/`theorem`/`example`，签名也要
    过内核的类型/Prop 判定；坏签名 = 一条 diagnostic（span 取签名自身）+ 声明 `Failed` +
    **不发** `exercise.open`。**判卷纪律随之上调**：只认 `decl.checked` 与 `diagnostic`
    ——`exercise.open` 计数对签名腐烂**永远是盲的**。
  * **构造子命名空间（G-02 / WO-005）**：`ctor mk` 的**规范名**是 `Ind.mk`（源名已含点则
    原样）；裸名保留为**闭包级解析别名**（唯一时可用，重复报 `elab-ambiguous-ctor-alias`）。
    用户可见：`#check`/`#reduce`/`#print`、hover、Infoview goal 文本、refine 骨架、
    语义高亮都改为打规范名；源文件自带 `inductive Nat` 时 `#reduce` 显示混合表示
    （`Nat.succ (Nat.succ (Nat.succ 1))`，实测已文档化）。
  * **Prop 结果 + Type 参数 + 单构造子归纳（G-03 / WO-006）**：`Exists` 形状的归纳块不再被
    内核断言拒绝；派生的 recursor 宇宙参数与内核一致（这种块 **0 个**，`Bar.rec` 不接受
    宇宙参数）。课程仓 `courses/set-theory/lib/Exists.sokonanoda` 同步从公理三件套升级为
    **真归纳**（名字与签名逐字不变 ⇒ units 的点名调用零改动）。
  * **L1 prelude（L-01/L-02）**：Full 模式自带 Lean core 的逻辑与等式骨架 **30 个名字**
    （`PRELUDE_NAMES` 12 → 42），**族粒度让位** ⇒ 自带同名声明的画布（入门课 7 个单元）
    行为逐字节不变，`course_shared.rs` 的 44 份副本一致性测试未改而全绿。
  * **用户自定义记法第一刀（G-04 / WO-011）**：`infix:N`/`infixl:N`/`infixr:N`/零元
    `notation` 可用（`∈`/`⊆`/`∅` 等）；数学符号是独立 token；**文件内作用域**（不跨
    `import`）、**不是声明**（零事件、不进声明表/goal 视图）、点名形式永久可用且两种写法
    判卷一致。`𝒫`/`ᶜ`/`''`/`⁻¹'`/`×ˢ`、跨 `import` 的记法、binder 记法与重载留第二刀。
  * **`course` 认 `import`（G-06 / WO-007）与 `query check` 同口径（G-10 + G-17 / WO-003）**：
    课程聚合对**有 `import` 的单元**走同一份项目闭包（`failed == 0` ⇔ 该单元 `grade` exit 0）；
    解析不了的文本在 `query check` 里不再假绿——`failed[]` 带 parse 诊断、
    **exit 0 → 1**（**有意的契约变更**），从此可作 `grade` 的交叉复核；`goals`/`holes`
    解析失败答 `not-parsable` + `ok:false`（"正常的没有"仍是空数组）。
  * **课程门禁（G1–G5）接进 `scripts/soko gate` 与 CI**：判据与规模无关、**不锁计数**
    （`grade` 退出码 / 目标存在 / 解答 0 open 且 checked>0 / 解答覆盖画布每个具名练习 /
    lib+Demo 0 open）；`--selftest`（判据通道自检，故意坏的单元必须被拒）、`--bisect`
    （不依赖诊断 span 的定位）、`--json`/`--report`/`--summary`/`--annotations`；探不到
    python3 ⇒ gate **exit 3**（无法判定 ≠ 绿）；CI 用当轮 `target/debug` 二进制，
    课程红自动挡住 `auto-tag` 的发布。当轮实测 **36 目标 · 355 checked · 99 open · 0 判负**。
  * **站点有卷 I 页面**：`site/set-theory.html` 显示 12 单元目录 + 每单元计数；数据由
    `scripts/gen-site-data.py` 生成（版本读 `Cargo.toml`、轮次读 `STATUS.md`、**计数由课程
    门禁实测**）——**永不手写**；`scripts/check-site.py` 绿。
  * **缺口台账成为门禁（同日主线收尾）**：「缺口即测试」从人肉纪律升级为**执行契约**——
    `scripts/soko gate` 第四步 = `python3 scripts/gap.py selftest` + `check`，`ci.yml` 的
    `test` job 同款 step `Gap ledger is consistent (docs/gaps)`（~3 s，不新建 job）。
    台账新增 `repro_expect`（`clean`/`rejected`/`exit0`/`nonzero`）：期望默认由 `status`
    推出，**但有些缺口的「修好」恰恰是判红**（G-01 = `rejected`，它钉的是「签名写错必须
    被拒」）；取值与复现类型不匹配直接判不一致，`gap.py selftest` 14 条判据钉住判定规则。
    **G-09 关账**：包装层已有稳定码 `kernel-internal` + 「这不是你的代码问题」提示，唯一
    已知可达触发路径随 G-03 关闭 ⇒ 改判 `fixed` 并撤下 `repro`（与 G-03 共用、已转绿），
    边界写进 `notes`（内核冻结下「断言永不外泄」是不变量；再现可达反例按新条目记）。
    收口后 `gap.py check` **exit 0 全绿**：24 条里 **18 条 `fixed_in = 0.59.0`**、未关账
    6 条（L-04 `workaround` + L-03/G-05/G-07/G-08/L-06，全是 `painful`/`nice`）。
  * **诊断坐标自描述（G-15 / WO-010）**：`query check` 的 `failed[]`/`warnings[]` **新增** 1 基
    `start_line`/`start_col`/`end_line`/`end_col`（**只加不删**：`start`/`end` 仍是字节 offset，
    坐标空间 = 入口文件；schema 号与事件种类都不动，双 GOLDEN 未改）——课程线与 agent 不必再猜
    单位；台账原记的「span 漂到别的声明」是**量具缺陷**（把字节 offset 当字符下标），真缺口是
    坐标没自带单位。守护三层：front 两条（span 字节切片逐字等于出错命令）、CLI e2e 一条
    （`failed[]` 行列 ≡ `grade --json` 的 span）、复现脚本重写（修前 exit 0 / 修后 exit 1）。
  * **课程跟随 prelude（P4）**：`courses/set-theory/lib/Logic.sokonanoda` 那 26 条**退化成只有
    注释的空壳**（prelude 已自带同样的 30 个名字；34 处 `import lib.Logic` 一字未改），
    课程侧 65 处项位裸名 `inl`/`inr` 改点号名 `Or.inl`/`Or.inr`（G-02 定形后裸项名不存在）；
    课程门禁 **329 checked · 99 open · 0 判负**（多出来的 26 条正是删掉的重复脚手架）。
  * **发布结果（2026-09-19 实测）**：push main → CI **7/7 job 全绿** → auto-tag `v0.59.0` →
    `release` **11 job 全 success** → GitHub Release **26 资产**（lsp ×8 / cli ×8 / vsix ×9 /
    `SHA256SUMS`）+ Marketplace **0.59.0** 已收录（01:30:07Z）。发布产物实测：下载
    `sokonanoda-cli-aarch64-apple-darwin.tar.gz` → `shasum -c` OK → `--version` = 0.59.0 →
    签名受检（`theorem t9 : 3 := sorry` ⇒ `kernel-expected-sort` + exit 1）、记法（`x ∈ A`）、
    `query check` 的行列字段、prelude 的 `And.intro`/`Or.elim`/`Iff.*`/`absurd`/`Eq.symm`
    都在包里可用（逐条实测见 `STATUS.md` 第一百〇六轮）。**首次 CI 红在 LSP 项目性能哨兵**
    （单次采样被并行邻居放大，非产品回归；对拍 + 修采样口径后复绿，见
    `docs/CI-FAILURES.md` 2026-09-19 条与 `docs/PERF.md` §采样口径）。
  * **不变的**：内核（冻结快照）**一个字节未改**；不调用官方 Lean 工具链；用户/agent 路径
    仍是零 cargo（`scripts/soko setup/grade/query/course`）。

* **2026-09-20（站点全面重构）** —— 用户原话：「项目更新了非常多个版本，我希望 site
  静态网页全面重构一下，**不要参考旧版本，旧版本没有设计感，美感，很多 ai 味**。
  我希望除了产品介绍页、项目文档，还可以设计一些功能页，即使仅仅通过网页也能让用户
  体会这个产品的一些功能（lean4 就有点一般，没有好好搞，只有一个简单的广告单）」；
  随后追加两条：
  * 「**先不做功能页，功能展示多一点也行，毕竟只是在 github pages 上，快也很重要**」
    —— 据此**不做 WASM / 不做后端**，功能展示改为「真实预计算数据 + 纯 CSS 折叠」，
    并把「快」写成可断言的体积预算；
  * 「我发现 superpower 啥的 skill 都没装，网页产品做得不够全面，可能是 skill 不够」
    —— 据此装了 26 个第三方 agent 技能（`dsh/agent-skills.json`），并**用它们回头审
    自己已写好的方案**，审出三处真问题（衬线是 AI 聚类第 1 类的核心配料、28 页下页头
    导航失效、安装 prompt 是 JS 注入的正文），以及**七处内容缺口**（搜索 / 对照页 /
    术语表 / 常见问题 / 版本历史 / 404 / 站点文件）。
  * **交付**：`site/` 从 9 页重构为 **28 页**（产品 4 / 功能 9 / 学习 4 / 使用 4 /
    过程与信任 5 / 英文 landing 1 / 样板页 1）；设计规则手册 `D1`、施工图 `D2`、
    页面施工标准 `D9`、四本事实卷宗 `C1–C4`、四份调研 `R1–R4` 全部落在
    `docs/design/site-rebuild/`。
  * **验收（一条命令，用户明确要求的「完整性与正确性」）**：
    `python3 scripts/site-verify.py` —— 完整性 3 项 + 正确性 11 项，
    **14/14 全绿、exit 0**；含 **K10 零伪造**（页面渲染的每条录制值回查生成它的 JSON，
    当前 91 条全过）与 **K12 课程计数可复现**（判据 = HEAD 的课程 + 已发布二进制，
    不是"现场跑一遍"）。
  * **退役**：`scripts/gen-site-demos.py` 与 `site/assets/demos/`（PIL 画的假 VS Code
    截图）**整体删除**——旧站用它们冒充产品界面，正是用户抱怨的那类东西；
    编辑器面板改成真 HTML/CSS。旧 `site/assets/style.css` 与 `site/assets/agent-prompt.js`
    同轮删除。
  * **不变的**：内核一个字节未改；不调用官方 Lean 工具链；用户/agent 路径仍零工具链；
    **站点写的是「已发布版本」的事实**——本仓并行开发时 `scripts/soko` 会量到未提交代码，
    这条陷阱与规避办法记在 `docs/design/site-rebuild/spec/D9-page-brief.md` §4.0。

* **2026-09-19（全课程 Lean 4 化：记法符号 + tactic 证明）** —— 用户原话：
  「我希望整个 courses 转向像 lean4 一样的可读性一点，And Or Iff Forall Exists 等等
  连接符用常规符号替换现在的 function call 形式，增加可读性。证明过程都用 tactic 过程，
  例题也用 by sorry。」随后追加：「涉及比较多的内容，可能反过来对 sokonanoda-lang
  功能本身有一定需求，你先全面调研并且计划好，保证结合计划和 subagents，不会遗漏
  什么东西。」
  * **范围（用户拍板）**：卷 I `courses/set-theory/` + 入门课 `course/` + `playground.sokonanoda`；
    语言改造深度「中等」；演示保持已证但改写成 tactic 风格、练习占位一律 `:= by sorry`；
    集合论符号**全部替换**、彻底 Lean 化。**不改 `site/`**（用户明示另有 agent 在做站点重构）。
  * **追加两条（同日，改变计划形状）**：
    ① **记法怎么敲**——「要考虑 notation 如何输入，应该像 lean4 一样 `\xxx` 替换，同时
       hover 内容提示用户如何输入对应符号」⇒ 定案为**客户端缩写改写器**（与 Lean 4
       `@leanprover/unicode-input` 同一张表）+ **hover 显示输入法**；**不做** LSP 补全
       （DSH 与 opencode 都不消费补全项，而 DSH 的 `lsp` 工具消费 hover）。设计：
       `docs/design/notation-input.md`。
    ② **隐式实参**——「如果想支持 notation，感觉 `{x : Set}` 这种隐参数自动推导的机制
       不得不实现了」⇒ 定案**路线 C（风格对齐 + 唯一确定，不引入元变量）**：内核
       `Expr`/`Value` 没有元变量槽（碰内核冻结），`elab_expr` 又在内核环境外运行
       （探针 = 整段前缀重编译）⇒ 真元变量与探针驱动都被否掉。设计：
       `docs/design/implicit-arguments.md`。**这推翻了原计划 D2 的「不做隐式实参」**
       （课程点名调用 2784 处里 **2308 处（83%）写了前导类型实参**，记法替换后它成为
       新的可读性瓶颈）。**注意契约变更**：省 `α` 的点名写法 `Set.mem a A` 将从
       「被拒」变成「合法」，被 `crates/cli/tests/notation.rs:163` 与
       `docs/design/notation-subset.md` 钉死的「护城河」话术必须同轮重钉。
  * **计划与设计**：`docs/design/course-lean-style.md`（主计划，含 R1–R4 + 追加片 R2.5、
    D1–D6、X1–X15 实测台账、L/C/F/W 工作项、subagent 分工 S1–S10）；调研底稿四份 +
    两份追加调研共 9 篇在 `docs/notes/course-lean-style/`。
  * **进展（2026-09-21）：R3 入门课改写落代码**——44 个教学文件（11 单元 × 中英 ×
    画布/解答）+ `course/unit11-project/`（4 文件）+ 规范副本 `course/shared/Nat`
    全部换成记法与 tactic 解答；单元④ 结构专项落地（补 `have` 演示与 `by_ex5`，
    画布计数 `(13,5,0)` → `(14,6,0)`）。as-built 见 `docs/design/course-lean-style.md`
    §9「R3」。
    **这一轮反过来改了语言两处**（都在 `crates/front`，内核零改动）——正是用户
    预判的「改写会对语言本身提要求」：① **判定合成声明携带声明的宇宙参数**，否则
    目标含 `Sort u`/`Eq.{u}` 的定理根本写不了 tactic（as-built
    `docs/design/by-tactics.md` §12）；② **`src_spine` 认记法节点**，否则
    `have h : B ∨ C` 之后 `cases h` 会被拒（逼学习者把块内假设写成点名形式）。
    两者都有回归测试（front 1 条 + CLI 1 条），课程语料即活样例。

  * **不变的红线**：内核 `crates/kernel/` **零改动**（`git diff --stat -- crates/kernel/` 空），
    用户/agent 路径仍零工具链，判卷一律走 kernel。
- 2026-09-21（第一百二十轮）：**官网简化成单个网页**——用户判定 28 页重构「属于灾难」，
  要求「把**所有** site 简化吧：**单个网页**，只说明：是什么，怎么安装，有什么核心特点，
  未来的计划。这几件事情」。落地：28 页 → 1 页（+ 1 份 CSS + 1 个 JS），删掉 8 份生成
  数据与 5 个生成器/检查器（约 2800 行），**设计语言原样保留**（方格纸 / 推理横线 /
  绿=内核通过过、朱=诊断与限制 / 17px 中文锚点 / 40rem 行长 / 自托管字体），
  新权威 `docs/design/site-single-page.md`，验收 `python3 scripts/check-site.py`（10 项）
  + `--browser`（真 Chrome）。
  **同轮另一条**：「把**正规的** 0.62.0 给我发布上去，耽误我正经测试了」——发版卡点
  （课程门禁单目标判卷预算被 **debug 构建**顶穿）改用 **release 构建**判卷
  （本机实测 190s → 93.5s），保留用户已抬到 600s 的内层上限；**根因未修**
  （`by` 块每走一步 tactic 都重新判定整份文档 ⇒ 代价随文件超线性，正解是前缀缓存），
  已写进站点「未来的计划」与 `STATUS.md` 第一百二十轮 §5。
- 2026-09-21（第一百二十一轮）：「**修改吧，而且性能能再恢复吗？**」——把上一轮只做了
  临时手段（抬超时 + 换 release）的 `by` 块判定性能**根因**修掉。落地：同一个 `by` 块里的
  判定不再逐步重判整份文档，而是**合成一份文档一次判完**（`judge_pairs_with` +
  乐观批次 + 不通过就严格重跑），`assumption` 因需按结论挑假设而保留当场判。
  实测（同机同二进制 A/B，开关 `SOKO_NO_JUDGE_BATCH=1`）：单元⑫ 解答 **91.2s → 28.7s**
  （3.2×）、整卷课程门禁 **4m33s → 2m56s**（1.55×），两态 **36 目标 · 328 checked ·
  99 open · 0 判负**逐项相同；判据三层（judge 单测 ×2 + CLI 端到端逐字节对拍）。
  **仍未还清**：每个带 tactic 的声明仍要重走一遍前缀（term 风格基线 3.6s），
  再往前需要**内核侧开放环境复用**（内核冻结，前端做不到）——已记入
  `docs/design/by-tactics.md` §13。

- 2026-09-21（**课程记法规则重建 + 基础类型隐式实参与 Lean 对齐**）——用户原话：
  「重新设置一个 courses 的规则，至少 notation 都要换掉，lib 和正文都换掉，不要有些
  还是老版本的。你先实现一个检查脚本，然后一个文件一个文件过。tactic 还比较费时，
  实现起来有问题，可以先保持一部分的 term，如果改成 tactic 那也先不动。」随后追加：
  「基础类型的隐变量也可以尝试和 lean 对齐。加上上一个要求，我举个例子：`Eq.{1}`
  直接就是一个等于号」。
  * **规则落地成脚本（可执行判据）**：新增 `scripts/notation-lint.py` —— 旧写法
    （逻辑连接符 / 量词 / `Eq.{u} T a b` / 集合点名叫法 / 基础类型写全前导隐式实参）
    检查器，覆盖 `courses/set-theory`（lib + units + solutions）、入门课 `course/`、
    `playground.sokonanoda`；**代码与注释都算**；`units/notation-cheatsheet*` 整文件
    豁免（教学装置，故意并列点名 ↔ 记法）；行内 `-- soko:notation-ok: <理由>` 的行
    豁免。施工手册 `docs/notes/course-lean-style/notation-rewrite-brief.md`。
  * **基础类型隐式实参对齐 Lean**（`crates/front`，内核零改动）：prelude 的
    `And`/`Or`/`Iff`/`Not`/`False`/`absurd` 与构造子改成隐式前导参数
    （`And.intro h1 h2`、`And.left h`、`Or.inl h`、`Exists.intro w hw`……）；应用路径
    的望远镜改从**注册表存的源级签名**解析（不再 `judge_infer`，消除 prelude 自举
    递归）；「实参个数 > 显式层数」判为**旧式写全**、一次装完（保持向后兼容）。
  * **边界（明说，不假装已对齐）**：宇宙多态的等式族**证明项**（`Eq.refl`/`Eq.symm`/
    `Eq.trans`/`Eq.subst`/`congrArg` 等）仍要显式宇宙与参数（应用路径的宇宙层级推断
    是独立的一刀）；`congrArg` 参数顺序改为 Lean 的 `{α β} {a b} (f) (h)`（**契约
    变更**）；`Set.univ α` 保留（零元应用不在覆盖内）。
  * **纪律**：纯记法改写**计数中性**——卷 I 门禁必须保持 `36 目标 · 328 checked ·
    99 open · 0 判负`；tactic 块不动，term 保持 term（用户明说 tactic 先不动）。

* **2026-09-21（VS Code 项目模式六条反馈：只做计划，不解决）** —— 用户原话：
  「在 sokonanoda 的 project 为 root 目录（比如 set-theory），vscode 有很多问题：
  编译很慢，没有实现编译后的文件加速 vscode 处理，新打开一个文件就有临时编译；
  infoview 的『声明』栏经常失效，有些文件有声明，有些没有，很奇怪，比如几个 unit
  教学文件就没有；infoview 里的 goal 展现没有用 notation 的方式；代码里的 notation
  不能跳转，hover 信息也没有对应的原始类型。」随后追加交付方式：
  「我希望你能拆解成尽可能多的环节。每次我让别人实现一个小环节，然后我就验收一下，
  好把控整个进度。」
  * **本次交付 = 计划，不改产品代码**：`docs/design/vscode-editor-feedback-plan.md`
    （**89 个环节 + 5 个检查点**，每环节自带一条可粘贴的判据命令与期望输出；
    按**批次**交付以避开"每次版本 bump 自动发版"）。
  * **六条根因均已实测**（不是推断）：① 项目文件在 LSP 上**每次动作从零编译整个
    import 闭包**（release 实测 unit01 1.8s / unit08 4.6s / unit12 8.5s /
    unit12 解答 36.1s），且编译期间独占文档锁；② LSP 对含 `import` 的文档**既不读
    也不写任何缓存**，项目缓存只活在 CLI crate 且形状是"一闭包一条、只存入口"；
    ③ 每份 `Doc` 各编一份闭包、项目模式**先白编一遍入口单文件**再丢弃、`set_text`
    无"文本未变即返回"短路、缓存只写"完全干净"的项目（`sorry` 是 WARNING ⇒
    教学画布永不入缓存）；④ `QueryDoc::goals` 被 `parsable()` 挡住（G-20 的补丁
    `check()`/`set_text` 都打了，只有 `goals` 漏了）⇒ 声明栏空而目标栏正常；
    ⑤ goal 文本有**四个生产者**，内核 pp 那两个必然点名；⑥ 记法使用处在 elab 里
    硬编码 `resolution: None`，而 `definition` 就是读它。
  * **顺带发现（同片代码，另行立账）**：`documentHighlight`/`references`/`rename`
    在记法符号上会误解析到外层 binder；`semantic::tag_runs` 从不填
    `Names::notations` ⇒ 记法在目标文本里不着色、导入名被标 `unknown_ident`；
    `position_to_offset` 按 `char` 而非 UTF-16 计数（`𝒫` 之后偏移）；缓存键折入了
    `current_exe()` 的秒级 mtime（CLI 与 LSP 是两个二进制 ⇒ 预热可能对 LSP 完全无效）。
  * **红线**：内核一个字节不改（`git diff --stat -- crates/kernel/` 每个检查点必须为空）；
    不调用官方 Lean 工具链；用户/agent 路径仍零 cargo。

* **2026-09-21（**内核解冻** + 环节粒度与快速反馈）** —— 用户原话：
  「你看看怎么实现小版本号升级，或者本地测试版本等，89 个最小的版本号也不是不行。
  关键是每次少做点快速反馈」；随后追加：
  「**内核完全可以改，不影响正确性，优化速度。没有其他死板的要求**」。
  * **内核解冻（本条取代旧硬规则 1 的"冻结快照"）**：`crates/kernel` **可以改**，
    含**热路径与内部表示**，目的可以是**提速**。唯一红线 = **判定正确性不变**：
    同一批输入**接受/拒绝不变、事件计数不变、golden 与 `--json` 逐字节不变**。
    每次内核改动必须带**三层回归**（kernel `tests/` + front 单测 + CLI e2e）
    + **语料对拍**（全部 `*.sokonanoda` stdout 逐字节比较 + 课程门禁计数逐项不变），
    性能改动另记 `docs/perf/ledger.jsonl`；改之前先读 `docs/architecture.md`
    §6（内核改动台账）与 §8（arena 生命周期 / panic→Result / `quiet_catch` 不可嵌套）。
    **这条解锁了两件此前判成"不做"的事**：`by` 块判定重跑整份前缀的根治、
    闭包的跨模块增量/入口间共享编译产物——它们正是"打开就卡"与
    `unit12` 解答 **36.1s** 的真正大头。已同步 `AGENTS.md` 硬规则 1、
    `docs/architecture.md` §6、`skills/sokonanoda-dev/SKILL.md` §1。
  * **环节粒度**：计划 `docs/design/vscode-editor-feedback-plan.md` 已扩到
    **100+ 个环节**（每环节一条可粘贴判据 + 一行可回退 commit），并新增
    **§0.4 快速反馈回路**（5 层：LSP 直探 / stub 宿主 / Rust 单测过滤 /
    真 VS Code 单用例 / 检查点全量）与 T-011…T-014 四个基础设施环节
    （`scripts/dev-loop.sh`、`SOKO_E2E_GREP` 单用例、版本纪律修订、回路写进文档）。
  * **版本与发版（实测 CI 逻辑后的结论）**：`ci.yml` 的 auto-tag 只在
    **"版本号对应的 tag 还不存在"** 时发版（`.github/workflows/ci.yml:61-65`）
    ⇒ **开发期不动版本号，推多少次 main 都不发版**（只跑 CI）。
    `docs/vscode-dev-guide.md` §2 的"每次 commit 必须 bump"是**约定**，CI 只强制
    `Cargo.toml` == `package.json`。**本地测试版本号只能用纯 `x.y.z`**：
    `scripts/soko:110` 的解析正则是 `^v?(\d+)\.(\d+)(?:\.(\d+))?$`，
    带后缀（`0.63.0-local`）会让启动器解析不出期望版本、按 G-16 **拒绝运行**。
    "这是哪份构建"改用 `scripts/soko version --json` 的 `source`、
    `sokonanoda: doctor` 的 `source=`、e2e 台账的 `dirty`/`lsp_sha256_16`。
  * **快速看效果（不用重装 VSIX）**：`sokonanoda.serverOverride: true` +
    `serverPath` 指向仓库 `target/debug/sokonanoda-lsp` ⇒ 每环节只需
    `cargo build -p sokonanoda-lsp -p sokonanoda-cli` + 命令面板
    `sokonanoda: restart server`（走与激活同一条解析链）。**限制**：这只换服务器
    二进制，换不了扩展代码 ⇒ 客户端改动要用 F5 开发宿主。

* **2026-09-21（每个修复环节都要有本地 e2e + 性能检测）** —— 用户原话：
  「增加本地的 e2e 测试，确保功能修复正确了」；「性能检测也补上」。
  * **三条机械判据，缺一不算完成**（写进计划的 §0.3 DoD）：
    ① **复现脚本**转绿（证明缺口没了）；
    ② **真 VS Code e2e 用例**转绿（证明真编辑器里能用；**改动前必须先跑一次
    确认它是红的**，否则这条 e2e 证明不了任何东西）；
    ③ **性能检测**：本环节场景的 `perf-check` 数字 + `perf-compare` 对上一检查点
    **无 > 25% 退化**（证明没把别的地方弄慢）。
  * **e2e 基建（计划 §0.5，T-015…T-019）**：新增最小 `import` 项目夹具
    （`sokonanoda.toml` + `lib/Set`（带 `∈`/`⊆` 记法）+ `units/u01`（带 `sorry`））；
    `testApi` 扩出 `infoview.lastDecls()/lastState()`（VS Code 测试 API 拿不到
    webview DOM ⇒ **e2e 断言载荷 + `test-webview.js` 断言渲染** 合起来才算"面板显示了"）；
    `scripts/vscode-e2e.sh` 加 `--grep` / `--profile debug` / `--no-build`
    （单用例**十几秒**，才能每环节跑）；**九条用例矩阵**（声明栏非空 / `alt+n` 跳洞 /
    重开命中缓存 / 保存不重编 / 改依赖只刷一次 / goal 含记法 / F12 跳记法声明 /
    hover 出 `Set.mem` 签名 / 批次 5 计数不变）。
    现状盲区（实测）：18 个用例**没有一个**看声明栏或 goal 文本；fixture 工作区
    **只有一个 README.md**，没有任何 `import` 项目夹具。
  * **性能检测基建（计划 §0.6，T-020…T-023）**：新增**真实课程闭包**的 perf 套件
    （七条 case：冷开 / 缓存热开 / 一次按键 / `by` 块 36.1s / 保存不重编 / 扇出 /
    `soko/project`——补上 ledger 里 `soko/project` 的漏记）；`scripts/perf-check.sh --case`
    （单场景快跑）；**`scripts/perf-compare.py` 回归比较器**（`best_ms` 退化 > 25% 判红、
    悄悄删掉哨兵也判红、宿主不同只提示不可比）——**这条最值钱**：109 个环节里
    "修 A 弄慢 B"是必然发生的，而它**只有机械判据能拦住**。
  * **现状缺口（实测）**：`docs/PERF.md` 只有人肉的"±25% 算同档"规则，**没有任何脚本
    读 ledger 判退化**；ledger 的 `scope` 只有 `front-project`/`lsp-project`/`cli-project`，
    **没有 editor-host、没有真实课程闭包**（既有数字 12–14ms vs 真实 1.8–36.1s，
    差 1–2 个数量级）。

* **2026-09-21（发版节奏：到一定程度就 bump）** —— 用户原话：
  「不行，到一定程度就 bump 一下」——否决了"整个批次一个版本"的做法。
  * **定案**：**不等批次收尾**，沿途按阈值发版。**触发（命中任一）**：
    ① 一条**用户可感知的能力**落地（不是内部重构）；② 距上次 bump 已过
    **≥ 8 个环节**；③ 用户要拿去测。**类型**：新能力 = `minor`，
    只是修好/更快 = `patch`（`docs/vscode-dev-guide.md` §2 的口诀）。
  * **bump 动作**：`Cargo.toml` 与 `editor/vscode/package.json` **两处必须相等**
    （CI 不等直接 exit 1）→ push main → auto-tag 打 tag 并 dispatch release
    → Marketplace（索引延迟约 5 分钟）。发版前 CI 要全绿。
  * **13 个 bump 点已标进计划的 §13 线性清单**（`⬆ **BUMP**` 行，平均每 ~9 个
    环节一次），查询入口 `python3 scripts/plan.py bumps`：
    批次 0 **0 个**（产物是判据与基建，零用户可见改动 ⇒ 不发版）；
    批次 1 两个 patch（声明栏服务端修好 / 客户端四缺陷 + e2e）；
    批次 2 三个（清浪费 patch / LSP 接入缓存 minor / 收尾 patch）；
    批次 3 两个（goal 显示记法 minor / 着色 + golden patch）；
    批次 4 三个（hover 原始类型 patch / F12 跳转 minor / 收尾 patch）；
    批次 5 三个（K1-a minor / K1-b minor / 收尾 patch）。
  * **两个已知代价（接受）**：① bump 后**编译缓存全失效**（键含
    `CARGO_PKG_VERSION`）⇒ 第一次打开会重编一遍；② 每次 release 跑完整流水线
    （8 LSP tarball + 8 CLI + 9 VSIX + SHA256SUMS + provenance + Marketplace），
    历史上 gallery 会间歇超时（`docs/CI-FAILURES.md`）。
  * **执行入口**：`python3 scripts/plan.py next` 会在"下一条做完要 bump"时
    直接打出来；`done <ID>` 勾进度；`check` 防清单与规格漂移。

* **2026-09-21（**速度是生命线**：编辑路径必须修）** —— 用户原话：

  > 我看到最近一次提交：「决定：② 不做……把"prelude 安装做成进程级一次性"记为
  > 遗留优化机会（属线 K）」我每次修改一下文件，就要编译 **6807ms** 吗？那我要你
  > build 有什么用？文档也有错误误导你了，速度和性能是生命线！

  事实（已量、已记成缺口 **G-29**）：项目缓存（T-A10/A11）只让**打开**变快
  （unit08 冷开 4970ms → 热开 8ms），**编辑一次仍然是 3039ms**——因为
  `project_compile` 把整条 import 闭包从零重编，依赖一个字节没变也照编。
  `build` 对此**毫无帮助**。

  拆解（unit08，4 个 import）：依赖 4 个模块 2.16s + 入口自身 2.7s。
  把 9 个 `by` 块全换成 `sorry` 只省 7% ⇒ **瓶颈是 elaboration，不是证明检查**
  ——这一点直接影响线 K 的选刀顺序（原计划把"`by` 块前缀复用"排第一）。

  **要求**：
  1. **编辑路径优先**：把线 K 的 K1（`EnvBuilder::with_env` / `snapshot`，复用
     依赖已编译好的环境）**提到批次 2 之后立刻做**，不再排到批次 5；
  2. 动内核之前先补 T-K01/K02 的护栏（语料逐字节对拍 + 验收清单）；
  3. 文档里**不许**把"每次编辑重编整条闭包"这种反复发生的大成本写成
     "遗留优化机会"——要写在最显眼处并带数字。

* **2026-09-21（**解答写项风格、当提示用**；老师据此给 tactic）** —— 用户原话：

  > tactic 先不动是我写的，但是现在看性能差太多了，solution 没必要损失性能。
  > solutions 保持项的方式，做提示。用户在做题目的时候，llm 应该能根据提示给出
  > tactic 的方式。

  **背景（实测）**：`by` 块的判定代价是 **O(前缀 × `by` 块数)**——每个 `by` 块都要
  把整份前缀重新 elaborate 一遍。`unit12-solution`（25 个 `by` 块）冷跑 **120 秒**，
  其中判定占 **68%**；同一批定理写成项风格实测快 **8.4×**，大文件 **25×**。

  **要求**：
  1. **`solutions/*.sokonanoda` 一律项风格**（lambda / 直接给证明项），**不用 `by`**。
     这**覆盖**此前"tactic 先不动"那条（它只对**画布**继续有效）。
  2. **解答同时是提示**：老师（LLM）读解答拿到骨架与关键件，再**翻译成 tactic**
     讲给学习者（`fun`↔`intro`、`f a b`↔`apply`+`exact`、`Iff.intro`↔`constructor`…），
     **不贴钥匙原文**。
  3. 写解答的硬性约束：签名逐字不变、不用 `sorry`、保留教学注释、判绿。
  4. 转换过程中撞到的语言缺陷照常记缺口（已记 **G-30**：期望类型不传播到嵌套
     应用的实参）。
  5. 规则落进 `courses/set-theory/AGENTS.md` 与 `skills/sokonanoda-teacher/SKILL.md`。

  **优先级**：与"速度是生命线"同级——它是**零内核风险**的一刀（只改课程内容），
  收益 8–25×。

* **2026-09-21（**`by` 判定重跑前缀违背热编译红线**，必须进计划修）** —— 用户原话：

  > 我还是很疑惑，lean 的 by 风格有这么耗时吗？是不是我们的 by 的实现方案有问题呢？
  > ……那要插入方案进计划里，这个违背我们的红线，也违背我们的热编译的设计初衷。

  **实测确认这是我们的架构问题**（缺口 **G-31**）：`judge_pairs_uncached` 每判定
  一批就把"整份前缀 + 合成声明"拼成新文档交给 `check_document_with`，后者从头跑
  完整流水线。`unit12-solution`（25 个 `by` 块）⇒ 前缀被重新 elaborate **25 次**、
  合计处理 **1MB ≈ 文件的 13 倍**、单次最贵 **28.6 秒**、判定占墙钟 **68%**。

  **Lean 不是这样**：`by` 是普通命令，按顺序 elaborate 一次，tactic 逐步改 goal
  state，前面的声明不重跑。

  **要求**：
  1. 这是**红线级**任务，**优先于线 K 的其余部分**（T-K12′ `EnvBuilder::with_env`
     / T-K13′ `snapshot`）；
  2. `by-tactics.md` §13 把"复用环境"记成死路——那是**内核冻结时**的结论，
     现已解冻且用户授权，**必须重新评估那条结论**；
  3. 线 L（解答改项风格）是**绕开**不是修复；K1 落地后要**重新评估**解答的写法；
  4. 判断标准：`SOKO_JUDGE_STATS` 的 `total_ms` 降到与"主 pass 一次"同量级。

* **2026-09-21（**`by` 判定重跑前缀 = 重大事故；前端 + 内核一起改**）** —— 用户原话：

  > 前端有问题，前端也一起配合改掉，这属于重大事故的 bug。如果影响性能，
  > "kernel 冻结"这种就问一下。很可能是某次我在编写课程的时候写入的规则，
  > 开发 by 的时候混入这种错误准则，太灾难了！

  **已定位那条错误准则**：`docs/design/by-tactics.md` §13 的「为什么不能缓存环境
  ——先说清楚哪条路是死的」，理由是"**`crates/kernel/` 是冻结快照（硬规则 1）**"。
  **已就地标注作废**（2026-09-21），并核对更正了它的 API 事实：
  `EnvBuilder::new` 接受外部 arena；`Env::new` / `new_w_temp_ext` 接受已有的
  `DeclarMap` 与临时扩展；内核**本来就有** `TypeChecker::check_declar`。

  **要求**：
  1. 这是**重大事故**，按事故处理：前端与内核**一起改**，不再"只在前端绕"；
  2. 修法见 `docs/design/by-judge-reuse.md` §5.1（把当前 pass 的 env 传进
     `by` 引擎，就地 `check_declar`，**取消文档合成与前缀重跑**）；
  3. 判据：`STAGE_STATS` 的 `passes` 从 **706 → 1**、`judge_ms` 从 82.7s → **接近 0**
     （判定不再是一趟流水线），`unit12-solution` 冷跑从 121.7s 降到 **10s 量级**；
  4. 全程 `scripts/kernel-diff.sh` 对拍（判定结果与诊断文本必须逐字节不变）；
  5. **凡是"因为内核冻结所以做不到"的结论，一律重新核对**——这份文档里可能还有
     同类残留（`grep -rn "冻结" docs/design/`）。

* **2026-09-21（记法消解也在重编前缀 = 同一个病的第二个入口，G-34）** ——
  承接上一条（`by` 判定重跑前缀）。把 `solutions/` 改成**项风格**之后，
  `by` 判定掉到 11 次 / 0.9s，**但 `unit12-solution` 仍然要 11.4–12.0 秒**。
  用新加的两个常驻诊断开关量出来：`SOKO_PASS_TRACE=<n>`（第 n 趟 `run_pass`
  的调用栈）显示 380 趟 pass 全部来自 `elab_notation` → `solve_prefix_args` →
  `infer_type_text` → `judge_infer`；`SOKO_JUDGE_STATS=1` 给出
  **`judge_infer` 126,105 次调用 / 363 次未命中**，而未命中一次 =
  合成 `<前缀>#check …` 把**整段前缀从零重跑一趟 pass**（G-34）。

  **要求**：
  1. 这与 G-31 是**同一个病、不同入口** ⇒ T-K20′ 的「就地拿当前 pass 的环境」
     设施必须**同时**覆盖 `judge_pairs`（`by`）与 `judge_infer`/`judge_type_of`
     （记法消解、冗余 `sorry` 探针、`application_arg_expected`、宇宙层级）；
  2. **局部能答的绝不问内核**：局部变量的类型就在 `ElabScope` 里
     （书写类型，还比内核 pp 更准——pp 会丢嵌套常量的隐式实参）。这条判据
     `implicit.rs` 早就有了，记法那条路漏了 ⇒ T-K22 已补
     （126,105 → 51,156 次调用、363 → 247 次未命中、11.4s → 7.5s）；
  3. **T-K22 是缓解不是根治**：剩下 247 次未命中来自冗余 `sorry` 探针、
     裸常量头、inductive 安装、闭包里的记法——它们**没有局部类型可拿**，
     只能靠第 1 条；
  4. 判据同时看两个计数：`JUDGE_STATS` 的 `calls` 与 `JUDGE_INFER_SPLIT` 的
     `misses`，**都要落到"一趟 pass 的量级"**；
  5. 复现件：`docs/gaps/repro/G34-notation-type-query-recompiles-prefix.sh`
     （已进 `gap.py check` ⇒ gate 与 CI）。

* **2026-09-21（编辑时的并发：编译不许挡住编辑器；连打不许变慢）** —— 用户原话：

  > 并发正确性……这个我认为完全可以优化，你调研一下其他开源项目怎么做的。
  > **编辑同一个地方，那就取消前一个编译。编译不同的地方，那代码块都不一样，
  > 触发的编译热更新的地方都不一样。不管怎么样，多次编辑不应该导致性能变差。**

  **要求**（三条都成立，实现见 `docs/design/lsp-edit-concurrency.md` §7）：
  1. **编译不许挡住消息循环**：一次长编译进行中，只读请求（`soko/stateAt` /
     hover / 目标栏）必须立刻答——用手上那份**上一次完成的状态**，而不是干等
     （clangd 的原话："methods should not block"）。判据：
     `crates/lsp/tests/lsp_edit_concurrency.rs` 的
     `read_only_requests_answer_while_a_long_compile_is_running`（改前 1277ms，
     判据 <100ms）；
  2. **同处编辑取代前一次**：同一份文档同时只有一个编译任务，编辑期间来的新版本
     只把"待编"换成最新那份（版本号校验、过期的结果丢弃）；
  3. **多次编辑不许导致性能变差**：N 次快速编辑编译趟数**不随 N 线性增长**
     （判据：`rapid_edits_coalesce_instead_of_queueing`，连打 5 个键 ≤3 趟）。
  4. **允许的代价，必须记账不许藏**：重建慢的文件（上一次编译 ≥150ms）下一次编辑
     等一个 **120ms 静默期**（clangd 的规则；`SOKO_DEBOUNCE_MS` 可覆盖），
     小文件不防抖。翻这个账要看 `docs/PERF.md` 的 T-A30 小节。
  5. **内核仍可为此改**（硬规则 1 的解冻同样适用）：只要判定结果不变。

- 2026-09-21（**goal / 假设 / 声明类型显示记法 —— 用户最初那条反馈交付**）——
  用户原话（六条编辑器反馈里的第 5 条）：

  > **infoview 里的 goal 展现没有用 notation 的方式。**

  **要求**（判据都在 `docs/design/notation-aware-printing.md` 与
  `docs/design/goal-rendering.md` §8）：
  1. **四个生产者都要用源文件的记法**：根状态（光标在 `by`/`sorry` 上）、
     无 `by` 的开练习、声明卡片、`by` 步进（含 `apply` 出来的子目标）；
     假设行的类型同样；
  2. **只换记法那几段**：binder 分组（`(A B : Set α)`）、`Type 0` 的写法、
     折行与缩进**逐字节保留**——不做整句重渲染；
  3. **判定一个字节不动**：折叠只作用在展示副本上，judge 的输入
     （`goal` / `binders[].ty` / `sub_goals[].ty`）没碰；课程门禁计数逐项不变；
  4. **折不了就原样，不猜**：一元前缀/后缀、零元、binder 位记法、元数对不上、
     `scoped` 未 `open` ⇒ 一律回退点名；
  5. **符号要有颜色**（第 5 条的延伸）：源里声明的与语言内建的记法符号都着成
     关键字色；导入名不再标成未知标识符；
  6. **诊断开关** `SOKO_NO_NOTATION_FOLD=1` 可关掉折叠（判别性测试用它）。

  **交付**：0.65.0（四个生产者 + 判别性测试 + 真宿主 e2e 用例 #6 转绿）与
  0.65.1（着色、导入名、折过的子树被应用时的语义修复）。缺口 **G-26 关账**
  （`fixed_in = 0.64.2`）；`scripts/verify-editor-issues.sh` 第 5 条**已修**。

- 2026-09-21（**BUMP 必须闭环：确认线上发版真的生效**）——用户原话：

  > **BUMP 记得要闭环执行，确认线上发版生效。**

  **要求**：
  1. 每次 bump 之后**不能只停在本地**：推 main → 等 CI 绿 → 等 auto-tag 打 tag →
     等 release workflow 产出资产 → **用 `gh release list` 核对线上确实有这一版**；
  2. **CI 红了就是发版断了**（auto-tag 只在 CI 绿时发版）：发现 main 红要**立刻**
     按 `docs/CI-FAILURES.md` 的纪律记录并修，不能让它挂着；
  3. 判据（一条命令）：`gh release list --limit 3` 的第一行版本号 == `Cargo.toml`
     的版本号；对不上就是没闭环。

  **背景（2026-09-23 实测）**：`origin/main` 已经推到 0.65.1，而线上最新发布还停在
  **v0.63.0**——中间三次推送的 CI 全红（e2e 的 known-red 用例），auto-tag 因此
  一次都没发版。**bump 在本地"完成"了、线上一步没动**，正是这条要求要防的事。

- 2026-09-23（**Infoview 声明栏的 `forall`；记法声明行的目标名不高亮 / 不能跳转**）
  ——用户原话：

  > 1. infoview里的"声明"栏，"forall" 可以用 "∀"，对应背后是不是丢了一批符号的
  >    改写呢？查一下完整的bug产生的原因，统一一起修掉；
  > 2. `prefix:100 " 𝒫 " => Set.powerset` / `postfix:100 " ᶜ " => Set.compl` /
  >    `infixr:80 " '' " => Set.image` / `infixr:80 " ⁻¹' " => Set.preimage` /
  >    `infixr:80 " ×ˢ " => Set.prod` 这部分代码在 vscode 里有两个问题，
  >    `Set.image` `Set.preimage` 和 `Set.prod` 没有高亮，另外 `Set.xxx`（ctrl+点击）
  >    不能跳转到定义。背后的 bug 机制先搞明白，然后再统一修复，这个应该是一个
  >    共性问题。

  **要求**：
  1. **先搞清机制再修**，两条都要**统一一起修**（用户明确怀疑是"一批符号的改写
     丢了"与"共性问题"——不要逐个打补丁）；
  2. 结论要写进 `docs/design/vscode-editor-feedback-plan.md`（新增环节），
     机制、判据、影响面三件套齐全；
  3. 判据必须是**可跑的一条命令**（e2e 或单测），不接受"看起来好了"。

  **机制已查明（2026-09-23 现场取证；缺口 G-37/G-38 + 复现件 + 计划 §7.6 的
  T-D50/T-D51）**：

  * 第 1 条**不止 `∀`，是"一批符号"**，而且**三层叠加**：
    ① `display.rs::fold_spine` 只放行 `Infix|Infixl|Infixr`（prefix `𝒫` /
    postfix `ᶜ` / binder `∀∃` / 零元 `∅` 全漏）；
    ② **`∀`/`∃` 不在内建记法表**（`BUILTIN_NOTATIONS` 只有 `∧ ∨ ↔ ¬ = ≠`），
    它们是 parser 级 binder 语法 ⇒ 光改过滤器也折不出来；
    ③ 声明栏那个 `forall` 是**内核 pp 打的 telescope**，不是源码里的 `∀`
    ⇒ 折叠要认 `forall (x : T), body` 这个**形状**。
    实测：`ty` = `forall (α : Type 0) (a : α) (A : Set α), a ∈ A -> a ∈ A`
    （`∈` 折了、`forall` 没折）。
  * 第 2 条**确实是共性问题**：真 LSP 探针打在五条记法声明的**目标名**上，
    `definition`/`hover`/`documentHighlight` **15 个请求全为 null**
    ⇒ 目标名**从来不是使用点**。用户看到的"只有三条没高亮"是**同一根因的第二种
    症状**：语义 token 类型号不同——在本文件里声明的名字（`Set.mem`/
    `Set.powerset`/`Set.compl`…）着色 **4 = FUNCTION**；不在本文件作用域的
    （`Set.image`/`Set.preimage`/`Set.prod`，在 `lib/Image.sokonanoda` / 单元⑤
    画布里）落成 **5 = VARIABLE**（= `UnknownIdent`），因为分类只能退回作用域查找。
  * **统一修法**：T-D50（目标名成为使用点：已知引用 + 跳转走闭包 + hover 说明）
    与 T-D51（折叠扩四种记法 + 补 `∀`/`∃` 表项）。

  **（以下是当轮写下的初步定位，保留备查）**：
  * 第 1 条的现场是 `courses/set-theory/lib/Set.sokonanoda`；声明栏文本来自
    `decl.ty`（走线 C 的折叠）。**假设**：线 C 只折 `Infix|Infixl|Infixr`
    （见 `crates/front/src/display.rs` 的 `fold_collecting`），而 `∀` 是
    **binder 记法** ⇒ `forall (a : T), …` 不折。若成立，则**不止 `∀`**：
    prefix（`𝒫`）、postfix（`ᶜ`）、binder（`∀`/`∃`）、零元（`∅`）都漏折
    ——正是用户说的"丢了一批符号"。
  * 第 2 条的现场是同一个文件的 **124–128 行**（用户逐字引用的那五行）。
    **假设**：记法声明里的**目标名**（`=> Set.image`）没有被登记成"使用点"
    ⇒ 既没有语义高亮、也没有 hover/definition。要查为什么 `Set.powerset`/
    `Set.compl`（prefix/postfix 那两条）看起来"没事"——是同样坏、还是只在
    infixr 那三条上坏（若后者成立，机制就在 parser 的 infixr 分支）。

- 2026-09-23（**`def` 的声明要显示"真正定义"**）——用户原话：

  > def 的符号，再声明里要多一行内容，对应它们的 `:=` 之后的那个真正定义，
  > 只是它们的类型已经提供不了足够的信息了。比如 Set.mem 的类型完全看不出
  > 它的本质是什么

  **要求**：Infoview 的"声明"栏里，`def`（以及 `opaque`）除了类型之外**多一行**
  `:=` 之后的真正定义；`theorem` 的证明不显示。

  **机制已查明（2026-09-23）**：内核**手里就有** value
  （`crates/kernel/src/env.rs:58` 的 `Declar::Definition { info, val, hint }`），
  只是 `Declar::info()` 没暴露 ⇒ 补一个纯访问器 `Declar::value()`；
  front 侧照 `ty_text` 的算法（`kernel_phase.rs:222` 的 `pp_expr` + 线 C 折叠）
  算 `val_text`。计划 **T-D52**（含判据与**性能必须先量**的要求——
  报告每次编译都构建，多算一次 `pp_expr` 是新增成本，超预算就改惰性）。

---

### 2026-09-25 · 用户报告：记法化"不通配"的三个现场（课程 + Infoview）

用户原话：「unit12-synthesis 没有完整的 notation 化；unit11 里 `exists_univ` 与
`no_univ_strictly_larger` 在 infoview 的声明里也没有正确 notation 化。这些 bug 都是
怎么形成的？好多"目标"都没有正确 notation 化。现在方案不是通配的，而是拆东墙补西墙的吗？」

**查证结果（三个不同成因，别混为一谈）**：

1. **记法门禁一直在崩，不是在判** ✗（**我们的回归**，R-3 引发）：`scripts/notation-lint.py`
   对 `rglob("*.sokonanoda")` 的命中直接 `read_text()`，而模块根下的产物**目录**
   `.sokonanoda/` 名字正好以 `.sokonanoda` 结尾 ⇒ `IsADirectoryError` ⇒ **崩了就不判** ✓。
   已修（只收 `is_file()` ✓，通用修法）。**教训**：新增目录/文件形态后，要扫一遍
   "按名字取文件"的所有工具（这次扫出 `verify-decl-panel.py` 早有过滤 ✓、其它无隐患 ✓）。
2. **门禁的判据是 28 条正则的"模式清单"** ✗ ⇒ 只抓当初枚举过的点形式，新记法/新写法天然漏 ✓
   —— 用户"不通配"的判断**成立** ✓。**修法（通用）**：改成**语义判据**——用 front 自己的
   解析器取每个声明的头名，若该头名在本闭包/prelude **有记法声明**而源码没用记法 ⇒ 判红 ✓；
   过渡期可从 `infix`/`notation` 声明**自动生成**模式，不再手维护清单 ✓。
3. **Infoview 里 `exists_univ` / `no_univ_strictly_larger` 显示点形式** = **记法第三刀**
   （**binder 记法**：`∃ x,` 的本质是 `Exists (fun x => …)`，**lambda 在操作数位**）✓
   —— 这不是"忘了"，而是**已登记并主动推迟**的项（`docs/design/notation-subset.md:355`
   「另立」；`REQUIREMENTS.md:1716` 留第二刀），而且**症状被预言过**
   （`REQUIREMENTS.md:2220`：「binder 记法 ⇒ `forall (a : T), …` 不折。若成立，则**不止 `∀`**」）✓。
   本次用户报告是它**第一次被确认是用户可见的** ✓：折叠失败会**整条**退回点形式（连外层
   `∀` 一起 ✗），所以 `ty` 里出现 `forall … Exists (Set α) (fun …)`、`Not (Exists … And …)` ✓
   （同文件的 `subset_univ` 等 6 条正常 ✓，因为它们的类型里没有 lambda 位的记法 ✓）。
   **另外查到**：`ty_text` 字段**全 8 条都是 `None`** ✗（不是转失败，是**没生成**）——
   卡片实际渲染走 `ty_runs`/`goal_runs`；`ty_text` 这条路径是否还需要、由谁消费，需要一并厘清 ✓。

**④ 追加查明（unit12）：你报的两条其实是同一个根因** ✓✓
`unit12-synthesis.sokonanoda:277-278` 自己写着：
「写法提醒：常函数写成 λ，而记法 `''` / `⁻¹'` **补不出 λ 操作数的前导类型参数**
（语言也没有类型标注 `(e : T)`），所以这一条的像/原像/交写**点名形式**。」
代码里逐行带 `-- soko:notation-ok: R5：…` 豁免标记 ✓，而 linter 本身就实现了这个标记
（`scripts/notation-lint.py:64/361` ✓）与内置豁免（`Set.univ α` 无记法、
零元应用不在隐式插入覆盖内、两个 cheatsheet 文件 ✓）。

⇒ 所以：**不是"忘了记法化"，而是"引擎在 λ 操作数位上补不出前导类型参数"** ✗
—— 与 ③（Infoview 里 `exists_univ` 折不了）**同一个洞** ✓：一个是**写的时候**被迫退回
点名形式（unit12 ✓），一个是**显示的时候**整条退回点名形式（Infoview ✓）。

⇒ **对"是不是拆东墙补西墙"的回答**：规则本身是通用的 ✓，但**引擎有一个有记录的洞**
（λ 操作数位 / R5 ✓），于是每遇到一次就要加一条**逐行豁免标记** ✗ —— 这就是"补丁在累积"
的真实形态 ✓。**真正的修法是补引擎（记法第三刀：让记法能在 λ 操作数位补出前导类型
参数，或给语言加类型标注 `(e : T)`）**，而不是继续加标记 ✓；补上之后 unit12 能用记法写 ✓、
Infoview 也能折 ✓（**一个修法同时消掉两个症状** ✓）。

**⑤ ③ 的根因与修法（2026-09-25 定位完成，未修）**：
`editor/vscode/src/test/extension.test.js:986-1000` 那条 e2e 用例把行为写明了 ✗：
**光标落在 `sorry`（tactic 块）内** ⇒ 目标文本走「**根状态**」生产者
（`DeclState.ty_text` ＝ **内核 pp**）⇒ **点名形式** ✗；光标在块**之后** ⇒ 另一支
（声明级目标）⇒ **记法保留** ✓。也就是说：
* 折叠能力**是有的**（`crates/front/src/display.rs` 的 `fold_spine` **已支持
  `NotationAssoc::Binder`** ✓，`crates/front/src/display.rs:338/387/397`，
  且有 `folds_nested_occurrences_including_inside_binders` ✓）；
* 但**根状态这条路没接折叠** ✗（用的是内核 pp 的原始文本）⇒ 用户光标一进 `sorry`
  就看到点形式 ✓。
**修法（下一轮）**：把根状态生产者也过一遍 `print_back(text, display_notations)`
（与声明级那支**同一条线 C** ✓）；判据 = **把那条 e2e 用例从"点名形式"翻成"记法
保留"** ✓（它是**用户可见**断言 ✓，符合"验收必须断言用户可见结果" ✓），
并在 `crates/lsp/src/tests/` 补 wire 层字段存在性 ✓。

**⑥ ③ 的落点再收窄（2026-09-25，查完 wire 层）**：
* **线 C 早就存在** ✓：`crates/front/src/compile/check/kernel_phase.rs:171-176` 写明
  "`ty_text` 是内核 pp 出来的**点名**形式 ⇒ 这里过 `print_back` 折叠"，**只作用于
  `ty_text`**（审计：它**只有给人看的消费者**）✓；
* **但 wire 上根本没有 `ty_text`** ✗：`crates/lsp/src/query_map.rs:74-91` 的声明条目只带
  `ty`（**内核 pp 原文** ✗）与 `ty_runs`（**已带记法的分段** ✓）；
  `soko/stateAt` 的目标同理只带 `goal` + `goal_runs` ✓。
* ⇒ 编辑器显示点形式 = 它用了 **`ty`/`goal` 文本字段**（内核 pp ✗），而不是
  `*_runs` ✓；而 e2e 用例（`extension.test.js:986-1000`）把"光标在 tactic 内 ⇒ 点名"
  钉成了期望 ✗。
**修法（收窄到一处）**：让**根状态那条生产者**在产出目标文本时也过 `print_back`
（与 `ty_text` 同一条线 C ✓）——**动的是"哪条路用了折叠"**，不是折叠能力本身 ✓；
判据不变：e2e 从"点名"翻成"记法保留" ✓ + wire 断言 ✓。

**⑦ ③ 的最终定位（2026-09-25，可以动手了）**：
* **折叠默认是开的** ✓：开关叫 `SOKO_NO_NOTATION_FOLD=1`，**只用来关掉**
  （`crates/front/src/compile/check/mod.rs:423-426` ✓）；线 C 的判别性端到端判据在
  `crates/cli/tests/notation_fold.rs` ✓，它逐条钉住**三个生产者**：
  **1 根状态**（`step == -1`）、3 声明 `ty`、4 `by` 步进 ✓ ——
  "根状态开着折叠时应当带记法" ✓ 是**绿的** ✓。
* ⇒ 缺口**不在 front、也不在默认值** ✗，而在**编辑器那条路**：
  wire 上 `goal`/`ty` 仍是内核 pp 原文 ✗，折叠结果只在 `goal_runs`/`ty_runs` ✓
  （`crates/lsp/src/query_map.rs:74-118`），而 e2e 用例把"光标在 tactic 内 ⇒ 点名"
  钉成了期望 ✗（`extension.test.js:986-1000`）。
**⑧ 更正 ⑦ 的结论（2026-09-25，同日实测推翻）**：**不是编辑器挑错字段** ✗。
证据（同一次 `query goals --json` 的两条对照，字段取自 `/tmp/g11.json`）：
```
subset_univ : ty = ∀ (α : Type 0) (A : Set α), A ⊆ (Set.univ α)   ← 带记法 ✓
              ty_runs 文本 = 同上（也带记法 ✓）
exists_univ : ty = forall (α : Type 0), Exists (Set α) (fun (U : Set α) => … Set.subset …  ← 点形式 ✗
              ty_runs 文本 = **与 ty 逐字相同**（也是点形式 ✗）
```
而渲染器**早就优先用 `*_runs`** ✓（`editor/vscode/media/infoview.js`：
`codeBlock("goal-ty", state && state.goal_runs, (state && state.goal) || "", "⊢ ")` ✓、
`decl-ty` 同理 ✓）⇒ **runs 里装的就是点形式** ⇒ 病在**生产者**（线 C）那一步没折叠成功 ✗。

**⇒ 真正的修法（收窄到一处）**：查**真实管线里的 arity / 记法表**为什么没让
`Exists (fun …)` 折起来 —— 单测 `folds_nested_occurrences_including_inside_binders`
是手工搭 arity 表过的 ✓，真实管线里 prelude 的 binder 记法（`∃`/`∀`）**可能没进那张表**
⇒ 修 `display_notations` / arity 来源（`crates/front/src/display.rs:686` 一带"把 prelude
源文本也算进来"的那段 ✓）。
**判据（直接、便宜、可复跑）**：
```
soko query goals --file courses/set-theory/units/unit11-universe-russell.sokonanoda
  ⇒ exists_univ 的 ty_runs 拼出来必须带 `∃`/`∀`（现在是 `Exists (fun …)` ✗）
```
外加 e2e 那条（`extension.test.js:986-1000`）从"点名"翻成"记法保留" ✓。
**判据**：e2e 从"点名形式"**翻成"记法保留"** ✓ + wire 字段存在性断言 ✓ +
`python3 scripts/audit-wire-fields.py` 守卫仍绿 ✓（A∖B 对账 ✓）。

**⑨ 给接手者的一句实话（2026-09-25）**：③ 这一条我**连续给出过三个不同结论** ✗
（"引擎缺能力" ✗ → "编辑器挑错字段" ✗ → "生产者没折成功" ✓），前两个都被下一轮实测
推翻 ✓。**教训**：不要从我的叙述接着推 ✗，**从这条测量开始** ✓：

```bash
# 一次拿到全部事实：点形式在哪一层就已经存在
SOKONANODA_BIN=<新构建> scripts/soko query goals --file \
  courses/set-theory/units/unit11-universe-russell.sokonanoda > /tmp/g.json
python3 - <<'P'
import json;d=json.load(open('/tmp/g.json'))['data']
for g in d:
    if g['name'] in ('subset_univ','exists_univ'):
        print(g['name'], '| ty =', g['ty'][:60])
        print('      | runs =', ''.join(r.get('text','') for r in (g.get('ty_runs') or []))[:60])
P
```
已知事实（都复跑过）：`subset_univ` 两层都带记法 ✓；`exists_univ` **两层都是点形式** ✗
（⇒ 病在**生产者**，不在编辑器 ✓）。**下一步要量的**：
`arities_with_prelude(&[...])` 里 `Exists` 那条**在不在、值是多少**（
`crates/front/src/display.rs:669-690` ✓）——`fold_spine` 要求
`spine.len() == arity` ✓，若 arity 记成 3 而 spine 是 2 ⇒ **静默不折** ✗（这正是
"单测手工搭表过、真实管线不过"的形状 ✓）。**先量这个数，再决定改哪一行** ✓。

**⑩ ③ 的第三轮测量（2026-09-25，两条量具已进树 ✓）**：
既然"先量再改"，我把两个数**量死了**（都写成 `crates/front/src/display.rs` 的单测 ✓，
可复跑 ✓）：
* `prelude_arities_cover_their_own_targets` ✓ **过**：prelude 段里
  `And`=2 · `Or`=2 · `Not`=1 · `Iff`=2 · `Eq`=2 ✓，而 **`Exists` 正确地不在里面**
  （它住在课程库 `courses/set-theory/lib/Exists.sokonanoda:88` 的 `inductive` ✓）。
* `closure_arities_include_an_imported_inductive` ✓ **过**：
  `arities_with_prelude_from(arities_in_commands(...))` 对
  `inductive Exists (A : Type) (p : A -> Prop)` + `ctor` + `end` + `binder_notation "∃" => Exists`
  ⇒ **`Exists` = 2 在表里** ✓✓。
⇒ **元数（arity）这条路不是病根** ✗ —— 把它从嫌疑名单划掉 ✓（这是有价值的否定结果 ✓）。
**剩下的嫌疑（下一轮量，不许猜 ✗）**：① `notation_table(&commands)` 有没有收下
**`binder_notation`** 这条（若没进**记法表**，`fold_spine` 连名字都找不到 ✗）；
② 真实 pp 出来的 `Exists (Set α) (fun …)` 的 **spine 形状**与 `arity` 是否真的相等。
**下一轮的量具（端到端，一次就能定性）**：造一个**真的** import 夹具
（lib 里 `inductive Exists` + `binder_notation "∃"`，入口用 `Exists (fun …)`），
跑真实编译，**打印 `ty_text` 前 40 字** —— 带 `∃` ⇒ 嫌疑 ① 排除 ✓；不带 ⇒ 就是它 ✓。

**⑪ ③ 第四轮测量（2026-09-25）：两个嫌疑清掉，一个夹具陷阱记下来**：
* **清掉**：`notation_table` **确实**收 `binder_notation` ✓
  （新增量具 `notation_table_collects_binder_notation` ✓ 过）；
* **清掉**：闭包里 import 来的 `inductive` **确实**进 arity 表 ✓（⑩ 的量具 ✓ 过）；
* ⚠ **夹具陷阱（我的端到端量具失败了，别学）**：我拿 `/tmp/nota5/`（自带 `lib/Exists.sokonanoda`
  + 一个 `binder_notation` ✓）跑 `query goals`，结果**连内建的 `=` 都没折**
  （`ty`/`ty_runs` 都是 `Exists Nat (fun (n : Nat) => Eq n n)` ✗）⇒
  说明那个夹具**根本没有课程上下文**（没有清单/闭包规则 ✓）⇒ **这一测不作数** ✗。
  **教训**：端到端量具必须跑在**真实课程**上下文里（或在 `courses/set-theory` 下临时加文件 ✓），
  否则量到的是夹具的贫瘠，不是产品行为 ✗。
* **剩下的那一个嫌疑（下一轮量）**：**真实 `unit11` 编译时**，
  `crates/front/src/compile/check/mod.rs:429-432` 那个 `commands`
  （`units.iter().flat_map(...)`）**到底含不含 `lib/Exists` 的命令** ✓。
  量法（便宜、不改产品）：在 `display_notations` 建表处**临时**加一个
  `SOKO_TRACE_NOTATIONS=1` 打印（表大小 + 是否含 `Exists`/`∃`），
  拿 `unit11` 跑一次 `query goals` ✓ —— 含 ⇒ 嫌疑清掉（那就去量 pp 的 spine 形状 ✓）；
  不含 ⇒ **就是它** ✓（修法：建表用的 `units` 要含闭包，而不是只有入口 ✓）。

**⑫ ③ 第五轮测量（2026-09-25）：插桩插错了生产者，但量到了更要紧的东西**：
* 我在 `crates/front/src/compile/check/mod.rs` 的建表处加了诊断开关
  `SOKO_TRACE_NOTATIONS=1`（默认零输出 ✓，已进树 ✓），拿 `unit11` 跑
  `query goals` —— **一行都没打** ✗（直连二进制、stderr 也是空 ✓）
  ⇒ **`query goals` 这条路不经过那个建表处** ✗ ⇒ 线 C 果然有多个生产者
  （文档里就写着"四个生产者" ✓），**我插错了那一个** ✗。
* 但由此量到了**更要紧的一条**：`query goals` 里 `subset_univ` 的 `ty_runs`
  **带记法**（`∀`/`⊆`/`∈`/`𝒫` ✓）而 `exists_univ` 不带（`∃` ✗）——
  而两者 `ty_text` **都是 `None`** ✗✓。结合 `kernel_phase` 的实现：
  ```
  let ty_text = quiet_catch(|| env.with_tc(EnvLimit::Empty, |tc| { … pp_expr(ty) }))
      .ok().map(|text| print_back(&text, &display)…);   // EnvLimit::Empty！
  ```
  ⇒ **线 C 的输入在 `EnvLimit::Empty`（空环境）下 pp 失败 ⇒ `ty_text = None`** ✗
  ⇒ 折叠**没发生** ✓；我们看到的 `∀`/`⊆`/`∈` 其实是**内核 pp 自己的记法** ✓
  （它认得环境里的内建/已声明记法 ✓），而 `∃`（`binder_notation` 声明的 ✓）
  **它不认** ✗ ⇒ 整条退回点形式 ✓✓。
  **这一条同时解释了"好多目标都没记法化"** ✓：只要类型里有**内核 pp 不认**的记法
  （binder 记法、库里的记法 ✓），整条就退化 ✓。
**下一轮的量法（已经很小）**：在 `quiet_catch` 的 `Err` 分支里打一行
（同样挂 `SOKO_TRACE_NOTATIONS=1` ✓）—— 看它**到底为什么失败**（空环境缺哪个
声明 ✓）。然后修法二选一：① 让 pp 用一个**够用的**环境（不是 `Empty` ✓）；
② 或把折叠**搬到 runs 生产线上**（对已经分好段的文本做记法替换 ✓）——
后者更贴近"四个生产者都该折"的设计 ✓。

**⑬ ③ 第六轮：⚠ 我的量具坏了三轮（旧二进制），以下是**唯一可信**的数据**：
* **病因（我的）**：我一直在跑 `./target/debug/sokonanoda` —— 那是 **0.67.0 时手工拷的副本** ✗，
  而 `cargo build` 的产物在 `/tmp/soko-target/debug/` ✓ ⇒ 我改的插桩**根本没进被我跑的那个二进制** ✗
  ⇒ ⑫ 里"query 不走这条路""pp 失败"等结论**全部作废** ✗（教训与 ① 同族：
  **先验证量具本身**，再看它量出来的数 ✓）。
* **可信数据（刷新二进制后，`SOKONANODA_NO_PROJECT_ARTIFACTS=1` + 全新缓存 ⇒ 真冷跑 ✓）**：
```
[trace-notations] units=4 commands=53 table=18 binding_Exists=true symbols=["∃"] arity_Exists=Some(2) arity_len=69
[trace-notations] ty pp ok
```
  ⇒ **两张表都是对的** ✓（`∃` 绑定在 ✓、`Exists` arity=2 ✓）、**pp 也没失败** ✓。
  但同一批声明里：
```
subset_univ              : ty_text=None | ty/runs = ∀ (α : Type 0) (A : Set α), A ⊆ (Set.univ α)   ← 带记法 ✓
exists_univ              : ty_text=None | ty/runs = forall (α : Type 0), Exists (Set α) (fun …)     ← **连外层 forall 都是点形式** ✗
no_univ_strictly_larger  : ty_text=None | ty/runs = forall (α : Type 0), Not (Exists (Set α) …)      ← 同上 ✗
```
  ⇒ 关键差别不是"某条记法缺失"✗，而是**内核 pp 对这两条压根没用记法**（外层 `forall` vs `∀` 就是铁证 ✓）。
* **下一轮的量具（一行，hook 已在 ✓）**：把 pp 的**原始输出**（`print_back` 之前）也打出来
  （同一个 `SOKO_TRACE_NOTATIONS` ✓）—— 这样就能一刀切开：
  **pp 没写记法** ✗ vs **`print_back` 折不动** ✗。**先拿到这一行，再决定改哪一层** ✓。

**⑭ ③ 定性完成（2026-09-25，第 45 轮）**：那一行诊断切开了二选一 ✓✓
```
subset_univ pp raw: forall (α : Type 0) (A : Set α), Set.subset α A (Set.univ α)   ← 点形式 ✗
exists_univ pp raw: forall (α : Type 0), Exists (Set α) (fun (U : Set α) => …)    ← 点形式 ✗
```
* **pp 永远出点形式** ✓（两条都是 ✗）⇒ 我们看到的 `∀`/`⊆`/`∈` 是 **`print_back` 折出来的** ✓；
* `print_back` 对 `subset_univ` **成功** ✓、对 `exists_univ` **整条放弃** ✗
  ⇒ **归属落定：`crates/front/src/display.rs` 的 `fold_spine` / `print_back`** ✓✓；
* 表都是对的 ✓（`symbols=["∃"]`、`arity_Exists=Some(2)` ✓），pp 也没失败 ✓
  ⇒ **不是"没数据"，是"折叠这条路在这个形状上放弃"** ✓。
**下一轮（复现判红，单元级）**：在 `display.rs` 的测试里用**现成的** `fold_text`
对**同一个字符串**做折叠，从窄到宽各断言一次：
① `Exists (Set α) (fun (U : Set α) => Set.subset α A U)` 单独折；
② 外层 `forall (α : Type 0), …` 折成 `∀`；
③ 整条链（`forall … , Exists (fun …) => forall …`）。
哪一步先红就是哪一步的锅 ✓ —— 然后才动 `fold_spine` ✓（**先红再改** ✓）。

**⑮ ③ 第 46 轮：三条事实把范围夹到一条缝**：
* **单测里折得动** ✓（新探针 `narrow_to_wide_binder_fold_probe`，逐字用 pp 原文）：
```
①窄  Exists (Set α) (fun (U : Set α) => …)        →  ∃ (U : Set α), …                    ✓
③整  forall …, Exists (fun …) => forall …          →  ∀ …, ∃ …, ∀ …                       ✓
④多行（含换行）                                     →  ∀ …,\n∃ (U : Set α), ∀ …             ✓
⑤`Not (Exists (fun …))`                            →  ∀ …,\nNot (∃ (U : Set α), ∀ …, …)     ✓
```
* **真实编译时表里有 `∃`** ✓：把 trace 日志按顺序相关（每次 `exists_univ pp raw` 之前那条建表行）
  ⇒ **每一次都是 `units=1 commands=44 table=18 binding_Exists=true`** ✓✓（不是我先前猜的
  `units=1 table=6` 那种贫表 ✗）。
* **可最终 `ty`/`ty_runs` 仍是折叠前的** ✗，而 `ty_text` 是 `None` ✗。
⇒ **结论**：折叠**做出来了**却**没进最终字段** ✓ —— 丢掉的位置在"折完之后、进 wire(`ty`/`*_runs`)
之前" ✓（或者 `ty`/`*_runs` 压根不是从 `ty_text` 来的 ✓）。
**下一轮那一行探针（就一处）**：紧挨 `ty_text` 算出来之后打一行
（`[trace-notations] {who} folded: {ty_text:?}` ✓）—— 它会一刀切开：
* 打出**带 `∃` 的字符串** ⇒ 折叠成功但被丢 ⇒ 顺着 `decl_states` → `report` → wire 找丢点 ✓；
* 打出 `None` ⇒ 折叠那一步本身在真实上下文里失败 ⇒ 再往里查 `print_back` 的入参 ✓。

**⑯ ③ 第 47 轮：**`print_back` 原样返回**（bail）—— 差别只在那个实例**：
```
[trace-notations] subset_univ folded = Some("∀ (α : Type 0) (A : Set α), A ⊆ (Set.univ α)")                 ✓ 折了
[trace-notations] exists_univ folded = Some("forall (α : Type 0), Exists (Set α) (fun (U : Set α) => …)")  ✗ 原样
```
⇒ `print_back` **没有抛错、也没有部分折**，而是**整体 bail** ✗；输入字符串**逐字相同**于我在
单测里折得动的那个 ✓（④多行那条 ✓）；表里 `symbols=["∃"]` ✓、`arity_Exists=Some(2)` ✓。
⇒ **差别只能在 `DisplayNotations` 实例本身** ✓（`table` 里那条 `∃` 声明的
`assoc`/`prec`，或 `arity` 的**键**，或 `builtin_notation_decls()` 里是否有同名的
**另一条**把它盖住/抢先 ✓）。
**下一行探针（就一处）**：在真实管线里把 `table` 中 **target==`Exists`** 的**全部**条目打出来
（`symbol`/`assoc`/`prec` ✓，注意可能有**多条** ✓），以及 `arity` 里以 `Exists` 结尾的**所有键** ✓
—— 与单测夹具（只有一条 `binder_notation` ✓）逐字对比，差在哪就是哪 ✓。

**⑰ ③ 第 48 轮：又夹一层，并**记下我第二次工具失误** ✗
**新事实**：`print_back` 的 bail **不是**源层解析失败造成的 ✓
（我的 `parse_ok` 探针两条都是 `false` ✗ —— 但 `print_back` 有自己的 char 级通道，
不经过 `crate::parse` ✓ ⇒ **这个探针量错了对象** ✗，别拿它当证据 ✓）。
**已确证的四条**（都在**同一次**编译里 ✓）：
1. 表齐全：`decls(target=Exists)=1 symbols=["∃"]` ✓、`arity_keys=["Exists"]` ✓；
2. `subset_univ` **折了** ✓（`∀ … ⊆ …`）而 `exists_univ` **原样返回** ✗；
3. 同一段 pp 文本（转义后肉眼逐字相同 ✓）在**单测**里折得动 ✓（窄/中/整/多行/`Not` 全过 ✓）；
4. pp 输出的**原文**（`RAWDBG`）已能打到日志 ✓。
**我的第二次工具失误** ✗：想逐字符比对"真实 pp 文本 vs 单测夹具字符串"，
却用 `bytes.decode('unicode_escape')` 解 Rust `{:?}` 的转义 ⇒ **把 UTF-8 的 `α` 弄成 `Î`**
⇒ 报了个**假的**"首字符就不同" ✗（长度差也是这么来的 ✓）。
**下一轮第一动作（别再和转义较劲 ✗）**：
* 让 `RAWDBG` 把原文**写到文件**（`/tmp/pp_<who>.txt`，`write` 而非 `eprintln` ✓）；
* 单测夹具同样**写一份文件**；两文件 `cmp` ✓ —— **字节 vs 字节**，不经任何转义 ✓；
* 若**字节相同** ⇒ 差别只可能在 `DisplayNotations` 实例 ⇒ 把该实例
  `table` 里 `target==Exists` 那条的 **`{:?}` 整条**打出来（`assoc`/`prec`/`operands` ✓）
  与夹具那条对比 ✓；若**字节不同** ⇒ 差在哪就是哪 ✓（一眼可见 ✓）。

**⑱ ③ 第 49 轮：红复现在手 ✓，元凶缩到"内建记法表的存在"** ✓
**红复现（单元级，进树 ✓）**：`crates/front/src/display.rs::folds_with_the_real_pipeline_table_shape`
—— 用**与管线逐句相同**的建表方式折那段 pp 原文，**原样返回** ✗（与真实编译一致 ✓）。
**机械二分（同一测试内，四条对照 ✓）**：
```
[real-shape  内建(splice) + prelude] → 原样返回 ✗
[A  内建(splice) / 无 prelude]        → 原样返回 ✗   ← 有内建就 bail
[B  无内建 / 有 prelude]             → ∀ …, ∃ …, A ⊆ U ✓
[C  无内建 / 无 prelude]             → ∀ …, ∃ …, A ⊆ U ✓
```
⇒ **是"内建记法表的存在"让它 bail** ✓，**与顺序无关** ✗
（我先猜"内建抢先 ⇒ 改成 append"✗ —— **实测无效、已回退** ✓：没被证实的语义改动不留 ✗）。
且 `decls(target=Exists)=1` ✓ ⇒ 内建里**没有**第二条 `Exists` ✗ ⇒ 干扰来自**别的**
内建条目（候选：`∀`/`¬`/`∧`/`=` ✓ —— 那段文本里都有 ✓）。
**下一轮（机械二分，一步到底）**：在 `folds_with_the_real_pipeline_table_shape` 里
**逐个** append 内建条目（一次一条 ✓），哪一条一加就 bail ⇒ 就是它 ✓；
拿到之后再看 `fold_spine` 为什么被它带崩 ✓（大概率是**同名 target 的两条**或
`assoc`/`prec` 冲突 ⇒ 修法随之明确 ✓）。

**⑲ ③ 真因锁定（2026-09-25，第 50–51 轮）：内建的 `=` 吃掉 `=>` 里的 `=`** ✓✓✓
机械二分（同一测试内逐条加内建 ✓）：
```
[bisect] builtins 共 6 条
  +0 And ∧ folded=true   +1 Or ∨ true   +2 Iff ↔ true   +3 Not ¬ true
  +4 Eq  =  folded=false   ← ★ 元凶
  +5 Ne  ≠  folded=true
```
**机制**：那段 pp 原文里有 **`fun (U : Set α) => …`** ✓ ⇒ 内建记法表里的 **`=`**
被喂给词法 ⇒ **`=>` 里的 `=` 被当成声明符号吃掉** ✗ ⇒ 折叠内部的解析崩 ⇒
`print_back` **整条 bail**（原样返回 ✗）。
**这不是新 bug —— 是 R-2 的同一族** ✓✓（`AGENTS.md` 的值班纪律原文：
"R-2 的真因是 `crates/front/src/semantic.rs:308` 把**内建记法表**（含 `"="`）喂给词法
⇒ `fun (x : Nat) => z` 里 `=>` 的 `=` 被当声明符号吃掉 ⇒ 整段降级成 1 个无 kind 的 run"）。
⇒ **凡类型里带 `fun … =>` 的声明都退化** ✓ —— 这正是用户说的"**好多**目标都没记法化" ✓✓。

**修法（照 R-2 的处方，别另起炉灶 ✓）**：折叠那条路（`print_back` 内部对 pp 文本的解析）
不能把**内建 `=`** 喂给词法 —— 要么按 R-2 的做法把它从那条路的词法表里摘掉 ✓，
要么让 `=>` 在词法层先被整体识别（现有修复若只在 `semantic.rs` 一处 ✓，
这里就是**同族漏网的第二处** ✓）。
**判据（已在树上 ✓，改成断言即可）**：
* `folds_with_the_real_pipeline_table_shape` ⇒ 修后必须折出 `∀ …, ∃ (U : Set α), …` ✓
  （现在打印的是"原样返回" ✗ ⇒ 转成 `assert_ne!` + 具体期望 ✓）；
* `soko query goals --file …/unit11-…` ⇒ `exists_univ` 的 `ty_runs` 必须带 `∃` ✓；
* e2e：`extension.test.js:986-1000` 从"点名"翻成"记法保留" ✓。

**⑳ ③ 的修法落点与下一个问题（2026-09-25，第 52 轮）**
**落点**：`crates/front/src/display.rs:153`
```rust
let Ok(ast) = crate::proof::parse_expr_text_with(text, &notations.table) else {
    return DisplayText::new(text);   // ← 解析失败 ⇒ 原样返回 = 我们看到的 bail ✓
};
```
`notations.table` 里带着**内建 `=`** ⇒ 词法把 `fun … =>` 里那个 `=` 吃掉 ⇒ 解析失败 ⇒ bail ✓
（与量到的 `parse_ok=false` 完全自洽 ✓；`subset_univ` 的类型**没有 lambda** ⇒ 没有 `=>` ⇒
解析通过 ⇒ 折得动 ✓ —— 两个观察同一机制 ✓）。
**下一个问题（单点）**：记法符号的匹配在哪里做、**有没有最长匹配** ✓
（`crats/front/src/lexer.rs` 里**没有**记法匹配 ✗ ⇒ 在 parser / notation 那一带 ✓）。
若有最长匹配 ⇒ `=>` 会先于 `=` 被吃掉 ✓（那这里的问题就是**表里不该带 `=`** ✗）；
若没有 ⇒ **补最长匹配**是通用修法 ✓（`=>`/`==>`/`<->` 之类都会受益 ✓）。
**判据（已在树，转断言即可 ✓）**：`folds_with_the_real_pipeline_table_shape` ✓、
`query goals` 的 `ty_runs` 带 `∃` ✓、e2e 翻绿 ✓。

**㉑ ③ 单点问题回答完毕（2026-09-25，第 53 轮）—— 修法确定** ✓
* **匹配在哪**：`crates/front/src/token.rs:612` `tokenize_with_symbols` / `:635`
  `scan_notation_symbols` ✓（`parser.rs:10` 引入 ✓；`parse_expr_text_with` → `parse_with_inherited`
  → 词法带 `inherited` 符号表 ✓）。
* **有没有最长匹配**：**有，但只在"记法符号之间"** ✓ —— `token.rs:630` 原文：
  "交给 `tokenize_with_symbols` 做**最长匹配**" ✓。
* **那为什么 `=` 还能吃掉 `=>`** ✗：因为 **`=>` 不在这张记法符号表里** ✓ ⇒
  最长匹配**不会拿它和基础算符比** ✗ ⇒ 记法 `=` 先命中、把 `=>` 的开头抢走 ✓。
  **这一条已由我的二分量到** ✓（给表里加 `Eq` ⇒ **任何含 lambda 的文本**都 bail ✗；
  其余 5 条内建都不影响 ✓）。
* **修法（通用 ✓，与 R-2 同方 ✓）**：让记法的匹配**与基础算符一起做最长匹配** ✓ ——
  即在同一位置比较"记法符号串"与"基础算符串"（`=>`、`->`、`:=` 等 ✓），**取更长的那个** ✓。
  只修 `display.rs` 那条路的表 ✗（治标：`ty_text` 好了，但 `semantic.rs` 之外的其它
  词法消费者仍会踩 ✓）；**改词法的比较规则才是通配** ✓（`=>`/`->`/`:=`/`==>` 一起受益 ✓）。
* **判据**：① `folds_with_the_real_pipeline_table_shape` 转断言 ✓；
  ② `query goals` 的 `ty_runs` 带 `∃` ✓；③ e2e 翻绿 ✓；
  ④ **新增**：`cargo test -p sokonanoda-front` 全绿（含既有的 `semantic.rs` R-2 判据 ✓
  —— 那条必须仍然绿 ✓，因为这次动的是**它上游的词法** ✓）。

**要求**：① ② 按上面的通用修法做；③④ **合并成"记法第三刀"排进计划**（引擎修，不是加标记 ✗）；③ 作为**记法第三刀**排进计划（用户在 Infoview 里
看得见它 ⇒ 不再是"可选优化" ✓），并给出判据（`∃`/`∀` 位记法折回的**真宿主 e2e 可见断言** ✓，
不只单测 ✓）。

## 2026-09-24 · 用户补充的 4 条需求（第 97 轮收到）

### R-1 `Set.mem def` 在 Infoview 里看不到第二行 `:=` 之后的数据

> **用户原话**：`Set.mem def` 在 infoview 里没看到 第二行的 `:=` 后的数据，
> 你查查 bug 产生的原因。

> **🚧 查证进展（2026-09-24，第 97 轮）** ✓：
> * T-D52 的显示链路是：内核 `Declar::value()`（`env.rs` 的纯访问器 ✓）→
>   `kernel_phase.rs` 算 `val_text`（`pp_expr` + 线 C 折叠 ✓）→
>   `front/src/query/mod.rs` 的 `DeclInfo.value/value_runs` ✓ → Infoview 卡片
>   （`.decl-val-line` ✓）。
> * 实测 `query goals --file courses/set-theory/lib/Set.sokonanoda` 的 JSON 形状是
>   `{"data":[{"name","kind","ty","ty_runs","status",…}]}` ✓ —— **第一条**（`Set` ✓）
>   的字段里**没看到 value 相关字段** ✗（但我的 dump 被截断 ✗，还不能下结论 ✓）。
> * **下一步（两条，按顺序）** ✓：
>   ① 打印 `Set.mem` 那一条的**完整 JSON** ✓，确认 `value`/`value_runs` 是否为空 ✗；
>   ② 若 query 有值 ✗ ⇒ 查 **LSP 的声明负载**是否转发了它 ✓（Infoview 走的是 LSP ✓，
>      不是 CLI 的 `query` ✓）—— 两条路分叉是这类 bug 的常见形态 ✓
>      （参考 G-39 的教训：同一个符号在两条路上认不出来 ✓）。

> **✅ 根因确认（2026-09-24，第 98 轮）—— 是 LSP 漏发字段，不是前端算不出** ✓：
> 1. **CLI 侧正常** ✓：`query goals --file courses/set-theory/lib/Set.sokonanoda`
>    里 `Set.mem` 的条目**确实带**
>    `value = 'fun (α : Type 0) (a : α) (A : Set α) => A a'` ✓ 与 `value_runs` ✓；
> 2. **扩展侧早就准备好了** ✓：`editor/vscode/media/infoview.js:282`
>    `if (decl && Array.isArray(decl.value_runs) && decl.value_runs.length > 0)`
>    ⇒ 渲染 `.decl-val-line` ✓（CSS ✓、CHANGELOG 也写了这两个字段 ✓）；
> 3. **断点在 LSP** ✗：`crates/lsp/src/query_map.rs:74` 的 `decl_info()` 把
>    `ty`/`ty_runs` 都映射了 ✓，**唯独没有 `value`/`value_runs`** ✗
>    ⇒ `GoalDeclInfo`（wire 结构体 ✓）里也没有这两个字段 ✗
>    ⇒ 扩展拿到的一直是 `undefined` ✗ ⇒ 那一行永远不出现 ✓✗。
>    **这就是"前端算好、CLI 给了、LSP 没转发"的三段式断链** ✓
>    （与 G-39 同形：同一个东西在两条路上不一致 ✓）。
>
> **修法（两处，约 10 行）** ✓：
> ① `GoalDeclInfo` 加 `value: Option<String>` + `value_runs: Vec<RunInfo>` ✓
>    （serde 字段名要与扩展读的一致 ✓：`value` / `value_runs` ✓）；
> ② `query_map::decl_info` 里 `value: decl.value, value_runs: decl.value_runs.into_iter().map(run_info).collect()` ✓。
> **判据** ✓：LSP 单测断言 `soko/goals` 的 `Set.mem` 条目带 `value` 与 `value_runs` ✓
> （并加一条"`ty_runs` 与 `value_runs` 都走 `run_info` 同一实现"的守卫 ✓，
> 防止两条路再分叉 ✓）；e2e 断言 Infoview 卡片出现 `:=` 行 ✓。

**要求**：T-D52 落地的"声明卡片多一行 `:= <值>`"**必须对 `Set.mem def` 生效** ✗
—— 现在看不到 ⇒ 是 bug ✓，**先查成因** ✓（不是先改 ✓）。

### R-2 `unit12-synthesis` 的两个 goal 很奇怪、没完全 notation 化、也没高亮

> **用户原话**：`unit12-synthesis.sokonanoda` 里的 `flawed_equalities_refuted` 和
> `project_chain` 的 goal 很奇怪，没有完全 notation 化，infoview 里的目标
> 也没有高亮。你查查 bug 产生的原因。

> **✅ 根因查明（2026-09-24，第 99 轮）—— 它其实是两个不同的问题** ✓：
> **(a) goal 没记法化** ✗ ⇒ **源码本身就没记法化** ✓：
> `courses/set-theory/units/unit12-synthesis.sokonanoda` 里那两条用的是**全显式**写法 ✓
> （`Set.image (Set Nat) (Set Nat) (fun (_ : Set Nat) => Set.empty Nat) …` ✓），
> 并且带了**豁免注释** `-- soko:notation-ok: R5：λ 操作数补不出前导类型参数` ✓
> ⇒ 记法门禁**放行**了它 ✓。**所以这是记法引擎的能力限制** ✗（**R5**：操作数是
> λ 时补不出前导类型参数 ✓），**不是显示 bug** ✓ —— goal 忠实显示了源码 ✓。
> **(b) goal 不高亮** ✗ ⇒ ~~与 R-1 同一类 bug~~ **✗ 已否证（第 2 轮，独立取证 + 我复核）**：
> **不是字段缺失** ✗。三条硬事实（都复核过 ✓）：
> ① 扩展的**声明卡片根本不画 goal** ✗（`infoview.js` 的 `renderDecls` 只画
>    `name`/`kind`/`ty_runs`/`value_runs` ✓，全文不读 `decl.goal` ✗）；
> ② 树里那行「目标」是 `TreeItem.description`（`extension.js:547/518` ✓）——
>    **纯文本，VS Code 树永远无法语义着色** ✗（平台限制 ✓）；
> ③ **真相层也没有 runs** ✗：front 的 `DeclInfo.goal` 是 `Option<String>`、
>    `goals: Vec<String>`（`crates/front/src/query/types.rs:79/82` ✓）
>    ⇒ **只给 LSP 加 `goal_runs` 是 no-op** ✗（屏幕零变化 ✓）。
> **守卫已固化** ✓：`python3 scripts/audit-wire-fields.py`（A∖B 对账：
> 扩展读了但 LSP 从不发的字段 ✓；当前 `NONE ✓ exit 0` ✓；内存回退 R-1 时它会报
> `value_runs` ✓ ⇒ 真能咬 ✓）。
>
> ~~**(b) goal 不高亮** ✗ ⇒ **与 R-1 完全同一类 bug** ✓：~~
> `GoalDeclInfo`（`crates/lsp/src/protocol.rs`）有 `goal`/`goals` ✓
> 但**没有 `goal_runs`** ✗；而 `soko/stateAt` 的 `StateGoalInfo` **有** `goal_runs` ✓
> ⇒ 声明面板的 goal 拿不到语义分段 ⇒ 前端无法高亮 ✓✗（同 R-1 的"三段式断链" ✓）。
>
> **⇒ 拆分（T-A4 的结论）** ✓：
> * **(b) 现在就修** ✓：`GoalDeclInfo` 加 `goal_runs`（+ 若需要 `goals_runs` ✓）
>   并在 `query_map::decl_info` 映射 ✓（与 R-1 同款、同判据 ✓）；
> * **(a) 重新定级** ✗：属**记法引擎特性**（R5 ✓）⇒ 单独条目 ✓
>   —— 在引擎支持之前，那两条**只能**写显式形式 ✓（豁免注释就是为此存在的 ✓）。

**要求**：① 这两个声明的 goal 文本要**完全 notation 化**（`∈`/`⊆`/`=` 等 ✓，
与课程一律写记法的口径一致 ✓）；② Infoview 的目标面板要**高亮** ✓。
**先查成因** ✓（可能与 R-1 同源：都是"报告/显示路径"上的洞 ✓）。

> **✅ 交付（2026-09-24，T-A5，0.66.0）** ✓：
> **(b) 已修** ✓ —— 真因**不是**字段缺失，是**两件**事，都落地了：
> ① **`=` 掉色**（这才是"目标面板不高亮"的真因）：`crates/front/src/semantic.rs`
>    把内建记法表（含 `"="`）喂给词法 ⇒ `fun … => …` 里 `=>` 的 `=` 被"声明符号
>    最长匹配"吃掉 ⇒ 整段**降级成 1 个无 kind 的 run** ⇒ webview 只画纯文本。
>    修法：`=`（与词法保留符号）**不交给词法**，`names.notations` 仍用完整表。
>    实测 `flawed_equalities_refuted` **1 → 313 段**、`project_chain` **1 → 216 段**、
>    对照组 `project_chain_cardinal` **94 段不变**；反例守卫：普通 `=` 仍着色 ✓；
> ② **声明卡片多一行带色的目标**：`DeclInfo` 加 `goal_runs`（父）+ `goals_runs`
>    （子，与 `goals` 按位置对齐）→ `GoalDeclInfo` 转发 → `infoview.js` 渲染
>    `.decl-goal-line`（`目标` / `目标 i/n` + `⊢ …` 的 `tok-*` span）。
>    屏幕上：**开放练习的声明卡片多一行 `⊢ <目标>`（带色）**，闭合声明零变化。
> **判据**（三层，缺一层就是洞）：front 单测（父子成对 + 对齐 + 反例）✓ ·
> LSP wire 单测（字段存在 + 重建 + 对齐）✓ · `test-webview.js` DOM（`.decl-goal-line`
> + `tok-keyword` + 闭合反例；修前 2/3 红）✓ · 真宿主 e2e（开放声明的载荷带 runs；
> 对**修前服务器**跑是 `1 failing`：`goal 必须带 goal_runs，实际 = undefined`）✓ ·
> A∖B 守卫 `scripts/audit-wire-fields.py` **改成按消费者分组**（并集会让 `goal_runs`
> 被 `soko/stateAt` 的同名字段顶包 ⇒ 漏了也不报；现在两个都报）✓。
> **(a) 未修** ✗：属**记法引擎特性 R5**（λ 操作数补不出前导类型参数）⇒ 那两条
> 仍只能写显式形式（豁免注释就是为此存在的）；本环节只把"显示/高亮"这一半收口。
> 设计 as-built：`docs/design/goal-rendering.md` §9。

### R-3 project 模式下应有编译产物目录（`.sokonanoda/`），vscode 与 code agent 共用

> **用户原话**：在 project 模式下，`sokonanoda.toml` 所在的根目录下，应该有
> `build` 的文件才对，vscode 和 code agent 都应该在这里取编译后的数据，避免
> 重复计算。比如创建一个 `.sokonanoda` 文件夹，把编译、以及以后的依赖啥的
> 都放到这个文件夹下。

**要求**：模块根下建 **`.sokonanoda/`** ✓，编译产物（以及将来的依赖等 ✓）放进去 ✓；

> **交付记录（2026-09-24，0.67.0 / T-B5）**：**项目**闭包产物已落**模块根**
> `<模块根>/.sokonanoda/compiled/<key>.json`（同格式同键；模块根下另有自忽略的
> `.gitignore` 与 `meta.json`）；**单文件**条目仍在全局缓存（键只含内容 ⇒ 那里才
> 谈得上跨项目共享）。CLI 四条命令（`build`/`grade`(`check`)/`query`/`course`）都走
> 新路径，**LSP 的读路径**同轮接上（否则 `build` 预热不再帮到编辑器 = 性能退化）；
> `--clean` **两处都清**（事件 additive：`{removed, global, project}`）；上限 32 条
> 按 mtime 淘汰；逃生门 `SOKONANODA_NO_PROJECT_ARTIFACTS=1`。判据
> `crates/cli/tests/artifacts.rs`（5 条真进程用例，两条做过修前判红 ✓）。
> 设计：`docs/design/project-artifacts.md`（含取舍与"别重踩"清单）。
> **未做**（后续阶段）：LSP 的**写**路径与 `query project` 的产物清单（T-C6）、
> `.sokonanoda/prefix/`（T-D11）、依赖目录（未设计）。
**vscode 与 code agent 都从那里取** ✓ ⇒ **避免重复计算** ✓
（这正是 T-K30 那条线的"模块级批量编译"的**用户侧理由** ✓；与 §14 的重构无关 ✓）。

### R-4 VS Code 命令名标准化

> **用户原话**：`vscode` 关于 `sokonanoda` 的命令都应该标准化一点，名字统一一点，
> `Sokonanoda: <命令>(说明)`，每个开头都大写：`Sokonanoda: Infoview (目标面板)`
> `Sokonanoda: Restart Server(重启服务器)` 啥的。

**要求**：命令标题统一成 **`Sokonanoda: <Command> (说明)`** ✓ —— 前缀固定 ✓、
命令词**首字母大写** ✓、括号里给中文说明 ✓。
**同步义务**（AGENTS.md 硬规则 ✓）：`editor/vscode/` 的 README/CHANGELOG/package.json
**与** `skills/` 三个技能 + `AGENTS.md` + `docs/vscode-dev-guide.md` **同一轮**更新 ✓，
且 `crates/cli/tests/skill.rs` / `dsh.rs` 不许漂移 ✓。

---

## 2026-09-24 · 用户拍板的**两条流程纪律**（同样是硬要求；全文见 `AGENTS.md` 同名两节）

> 为什么进这里 ✓：`AGENTS.md` 的收尾义务写着"用户新要求追加进 `REQUIREMENTS.md` §9
> 并注明日期（**冲突以该文件为准**）"——这两条是用户拍板的，却在 `AGENTS.md` 里
> 落了地、§9 里缺席 ✗（2026-09-24 补记）。编号用 **P-**（流程），与 R-（产品）分开。

### P-1 验证设计纪律（**没设计正确就是白做功**）

> **起因是两次同形事故**：① T-D52 给 Infoview 的 `def` 卡片加"第二行 `:= <值>`"，
> **e2e 全绿却用户看不见**——LSP 的 `decl_info()` 漏映射 `value`/`value_runs`，而扩展是
> 「**有就渲染**」的宽容实现 ⇒ 静默降级；② R-2 的"目标不高亮"同形（真因在 `=` 掉色）。
> **病根**：**真相**与**显示**是两条路，bug 活在**接缝**里；**数据对了 ≠ 用户看见了**。

**要求**（四条，逐条可判）：

1. 每条**用户可见**改动，先回答一句：**"屏幕上会多/少什么？那条断言在哪一层？"**
   —— 答不上来 ⇒ **验收不完整**，不许勾环节；
2. **三层各司其职**：front/CLI 单测 = **真相**（算得对）；LSP 单测 = **wire 契约**
   （**字段存在性**，不只是值）；**e2e = 用户看到的东西**（**渲染结果**，不只是"数据在"）。
   Infoview 的"看得见"还有一层：`node editor/vscode/test-webview.js`（stub DOM 里跑**真的**
   `media/infoview.js`）——三层表见 `docs/vscode-dev-guide.md` §3；
3. 凡"A 层产出、B 层消费"的字段都要 **A∖B 对账**：`python3 scripts/audit-wire-fields.py`
   （已进 `scripts/soko gate` 与 CI）。**反向验证是硬要求**：守卫必须能**咬住已知的历史
   bug** —— 回退 R-1 的 `value_runs`、或 T-A5 的 `goal_runs`/`goals_runs` 时必须报红 ✓
   （**咬不住的守卫等于没有**）；
4. **「有就渲染」是反模式**：宽容消费者会**掩盖契约破坏** ⇒ 出路是①契约测试覆盖它
   （首选），或②字段缺失时给**可见信号**（至少 `console.warn`）。

### P-2 并行与 subagent 纪律

> **两次教训**：上一版计划 subagent **太少** ⇒ 全程串行、**太慢** ✗；更早一版**太多**
> ⇒ 烧 token ✗、经验不共享 ✗、错误百出 ✗。

**要求**：① subagent 只做**可并行 / 只读 / 边界清晰**的活（调研、逐文件审计、多角度
验证、写复现件、量基准）✓；**判定相关代码与发布闭环走主线** ✗（内核改动、bump/release、
跨模块重构不外包）；② 并发上限 **2–4**，**写操作串行**（同一文件/模块同一时间一个写者）；
③ 每个 subagent 的 prompt **必须自带三样**：规则摘要（尤其陷阱清单）+ **可执行判据**
（不是"看看对不对"）+ 明确的"**不许改什么**"边界；④ 产出**验证后**才并入（复跑它给的
判据、抽查结论）；⑤ 发现**要回写文档** ⇒ 下一个 subagent/下一轮能复用；
⑥ **提交粒度不变**：一个环节一个 commit、一阶段多 commit、**阶段收尾才 push**。

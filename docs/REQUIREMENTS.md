# 项目要求总账（REQUIREMENTS）

> 这是用户全部要求的**权威记录**。任何 agent 接手任何任务前先读本文，
> 再读 `ROADMAP.md`（里程碑）与 `docs/STATUS.md`（当前进度）。
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
8. **判定永远走 kernel**，不做文本比对（`proof.rs::assumption` 的文本比对是待替换草案）。

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
- agent（本会话的我）即老师：写定义/出题 → 用户作答 → agent 跑
  `cargo run -q -p sokonanoda-cli --bin sokonanoda -- --json playground.sokonanoda`
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
- **文档先行、交接友好**：`docs/REQUIREMENTS.md`（本文）、`docs/STATUS.md`（进度日志）、
  `docs/architecture.md`（架构事实）、`docs/lsp-notes.md` / `docs/vscode-notes.md`
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
  node_modules（VSIX 仍坏）。产出：docs/design-goal-refine.md、
  docs/gap-analysis.md、docs/RELEASE.md、8 个新 kernel 错误码。- 2026-09-07（九）：gap-analysis 第一批落地（业内标准补全）：completions、
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
`docs/design-hover-brackets.md`（kernel pp 播种 + 括号组匹配 +
   `Not a` 保持折叠），测试 front 2 + LSP 5 + 旧断言对齐。
- 2026-09-09（十四）：**课程双语化（用户指令）**：tutorial/教程文档要有中文
  与英文两种版本——范围 = `course/` 单元课程为主，形态 = 中文/英文各一份
  独立文件。落地：`course/en/` 英文镜像（5 单元画布 + `solutions/` 解答钥匙），
  `course.json` 增 `title_en`，`course/README.md` 补双语布局说明，CI 新增
  `en_mirrors_match_chinese_event_counts` 守卫（事件计数逐项相等 + 英文钥匙
  0 诊断 0 洞），设计见 `docs/design-course-bilingual.md`。**英文注释按语义
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
   所有 hover 分支带高亮 range。实现见 `docs/design-hover-refactor.md`，
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
   goal state**——设计见 `docs/design-by-tactics.md` §6（front `by_steps` +
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
   `docs/design-bundled-lsp.md`：per-target VSIX（`vsce package --target`；
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

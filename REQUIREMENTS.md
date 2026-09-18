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
  - **文档**：新增 `docs/E2E.md`（手册：一条命令、四层分工、台账字段、卡住时判读、
    环境坑、与 CI 的关系）；`docs/vscode-dev-guide.md`（测试三层 + 坑 19/20/21 + 坑 14
    更新）、`docs/TESTING.md` 集成测试小节、`AGENTS.md` 命令面、`skills/sokonanoda-dev`、
    `docs/README.md` 地图同轮同步。

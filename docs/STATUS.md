# 当前状态与进度日志（agents 先读这里）

> 快照：2026-09-07（第十六轮：???→sorry 迁移 + VS Code 集成测试 + hover 纪律）
> 仓库：`sokonanoda-lang`；权威计划 = `ROADMAP.md`；**用户要求总账 = `docs/REQUIREMENTS.md`（先读）**；
> 设计 = `docs/design-round14.md`（含本轮 B′ 决议）/ `docs/design-kernel-taxonomy.md` /
> `docs/design-course-status.md` / `docs/design-hints-suggestions.md` /
> `docs/design-rename-inlay.md` / `docs/design-goal-refine.md` / `docs/design-i8-i9.md` /
> `docs/design-infrastructure.md`；
> 架构/内核 = `docs/architecture.md`；协议 = `docs/protocol.md`；测试地图 = `docs/TESTING.md`；
> 差距审计 = `docs/gap-analysis.md`；**经验台账 = `docs/LESSONS.md`**；
> 发布 = `docs/RELEASE.md`；
> agent 入口 = `AGENTS.md` + `skills/`；LSP/VS Code 调研 = `docs/lsp-notes.md` / `docs/vscode-notes.md`。

## 一句话

`.sokonanoda` = **纯声明式教学文件（无 `#` 命令）+ 完整 sokonanoda 内核 + LSP 反馈通道**。
练习 = 带 `???` 洞的 `def name : T` / `theorem name : T` / `example : T` 声明。
CLI/REPL 的 `#check` 等只是调试/自测工具，不是文件格式。

## 本轮进度（2026-09-07，第十六轮：学习者反馈验收落地）

1. **???→sorry 迁移完成**：lexer 遇 ? 报教学引导错误；全仓清扫 24 文件
   （playground/course/examples/tests/docs）；协议词表不变。
2. **hover 纪律**：显示「表达式 : 类型」+ 声明名显示完整内核签名
   （ty_text）+ 关键字悬停静默 + 松散变量 $N→binder 名字。
3. **VS Code 集成测试**：@vscode/test-electron 4 用例 + CI xvfb。
4. **Lean 4 调研确认**：点分名原子/sorry warning/hover 签名——设计与
   官方对齐。
5. 测试总量 **388 + 4 VS Code 集成测试**。

## 本轮进度（2026-09-07，第十五轮：开发清单清零，2 subagent 并行）

1. **spine meta 方案 B′（S1）**：`field_type_text` 升级为深度 AST 替换
   （`substitute_names`：模板 binder 名 → goal 实参 AST 全量替换，innermost
   wins 遮蔽守卫，命中节点 span 回填）——复合字段类型（`And a b`）现在正确
   实例化为学生上下文（`And True False`），不再原样渲染模板名；裸 Ident
   路径与失败语义不变（旧断言零改动）。kernel 渲染版（方案 A）留作远期。
2. **失败声明建议升级（S2）**：三条建议梯子——
   - **kernel 验证 rfl 替换**（新 `judge_value_replace`：值位整体替换合成
     声明交完整 kernel 裁决；Eq 形状声明验证通过才呈现，`verified: true`）；
   - **Reset**（保留已写 lambda 前缀、只重置主体为 `???`——学生类型标注
     工作保留，剩余目标由 goal 视图接管；保守形态识别：仅括号/花括号
     binder 的 `fun x =>` 链）；
   - **Restart**（整值骨架，既有）。
   首条 `is_preferred`；lib.rs 锚点全绿。
3. 测试总量 **380**；全绿；fmt/clippy 干净；playground（12 open / 0 诊断）与
   course（19/20/0）锚点不变。
4. **至此 gap-analysis Top 10 + 附加小项 + 分类学余项全部清零**；剩余仅
   运营项（release 首跑需打 tag、教学回环需真实学习者）与远期设计项
   （spine meta 方案 A）。

## 本轮进度（2026-09-07，第十四轮：hole_id + auto-derivation，2 subagent 并行）

> 设计先行：`docs/design-round14.md`（含 spine meta 的 A/B/C 方案取舍——
> 推荐方案 B 为下一轮实施项）。

1. **稳定 hole_id（P）**：`soko/goals` 的 `holes` 变 `[{range, id}]`，
   id = `<声明名>:<洞序号>`（匿名 example 用 `example@<行>`，与
   `render::decl_name` 一致）——同一版本内稳定、声明名不变时跨版本稳定，
   外部工具可引用；`soko/nextHole` 保持裸 Range；VS Code 点击统一走
   `holes[0].range`（P 核实客户端历史上只消费 `decl.hole`，无行为变化）；
   契约测试双向钉死（服务端出 id、客户端不当裸 Range 用）。
2. **归纳块 recursor 自动派生（Q）**：无显式 `rec` 的 `inductive` 块自动
   合成 recursor + iota 规则（与 py-nat 手写版同构、内核 def_eq 比对通过）：
   - 递归块（Nat 无 rec + add 闭环）、非递归块（Unit）、多构造子
     （Bool：`not tt ⇒ ff`）全部工作；
   - Prop 块退化为无宇宙参数的小消除 recursor；
   - **字段望远镜契约修正**：`num_fields`/`ctor_telescope_size_wo_params`
     改按完整 Pi 望远镜计（result 箭头链的 domain 也是字段——内核
     `check_declared_metadata` 的要求；py-nat 等既有块数值不变）；
   - 第十三轮的 `elab-missing-inductive-rec` 守卫/变体/文档条目移除
     （被本功能取代）；显式 rec 优先，py-nat/课程块零变化。
   - 教学定位：rec 块仍是单元⑤正课，auto-derivation 是其后的便利层。
3. 测试总量 **360**（front 183 / lsp 61 / cli 65 / kernel 45…）；全绿；
   fmt/clippy 干净；playground（0 诊断）/course（19/20/0）锚点不变。

## 本轮进度（2026-09-07，第十三轮：内核分类学收尾 + 基建，4 subagent 并行）

> 设计先行：`docs/design-kernel-taxonomy.md`。K（内核冷路径分诊）/
> L1（失败声明建议）/ M（criterion 基准）/ N（fuzz harness）并行，主会话
> 合并期修复 K 发现的功能性 bug（非递归归纳块）。

1. **内核错误分类学余项（K，冷路径三层回归）**：全内核 `assert_eq!` 清点
   分诊——3 处学习者可触发站点改稳定消息（`is_prop_type` 带 `got:` 渲染、
   iota 规则顺序/数量两处裸断言）；新错误家族 `kernel-rec-rule-mismatch`
   （`ErrorKind` + code + hint + protocol.md + 穷尽清单）；8+ 处 front 已
   拦截的 backstop 与真内部不变量保留 assert（internal）。热循环零改动。
2. **非递归归纳块修复（主会话，K 发现的功能 bug）**：内核按构造子 telescope
   自算 `is_recursive` 并断言一致——front 恒传 `true` 导致
   `inductive Unit/Bool` 崩溃。修复：`elab.rs` 从源码 AST 同规则镜像（含
   result 箭头链的 domain）；缺 `rec` 的块在**入环境前**报干净教学错误
   `elab-missing-inductive-rec`（check-then-add 保持；rec 块本就是白名单
   内容，auto-derivation 留作课程轮设计）。测试三层（kernel 语义 +
   front 2 + CLI 2）。
3. **失败声明建议（L1）**：`SuggestionKind::Restart`——kernel-rejected
   声明按自身类型形状生成重启骨架 `fun (x : A) => ???`（tokenize 定位
   值位，≤3 层剥 Pi，binder 防撞改名），LSP code action 整体替换值位；
   骨架落回后内核重查回到 Open（测试验证闭环）。
4. **criterion 基准（M）**：`crates/front/benches/pipeline.rs`（黑盒公开
   API）：native_bigint_reduce ~41ms / iota_deep_reduce ~14ms /
   session_suffix_recheck ~500µs；语料校验 `OnceLock` 先行。本地跑：
   `cargo bench -p sokonanoda-front --bench pipeline`（多 target 需带
   `--bench pipeline` 选择器）。
5. **fuzz harness（N）**：`fuzz/`（独立 crate，脱离 workspace，cargo-fuzz
   标准布局）——`parse_never_panics`：parse/semantic_tokens/prelude 指令/
   完整 check_document 永不 panic；`cd fuzz && cargo check`（stable）过；
   CI 不跑，用法见 `fuzz/README.md`。
6. 测试总量 **356**；全绿；fmt/clippy 干净；playground（0 诊断）与
   course（19/20/0）锚点不变。

## 本轮进度（2026-09-07，第十二轮：课程地图 + 小项，4 subagent 并行）

1. **`sokonanoda course <course.json>`（课程地图，gap #8 后端）**：聚合
   course.json 全部单元的 `decl.checked / exercise.open / failed /
   expr.reduced` 计数，JSON 视图 = 封闭新事件 `course.unit`（坏单元带
   `error` 字段）+ `course.summary`（进 protocol.md + 词汇表 + 4 个 e2e）；
   人类视图逐单元一行；**进度不是错误**（open/failed 也 exit 0）。
2. **VS Code「课程」树（gap #8 前端）**：`sokonanoda.courseMap` 视图 +
   `sokonanoda.courseRefresh` 命令——客户端跑 CLI 子进程解析 JSON Lines
   （10s 超时、并发去重、找不到 course.json 静默空树）；节点按
   open/failed 着色、点击打开单元文件；**聚合归 CLI，服务器保持单文档**
   （负断言：客户端不得引用 soko/courseStatus）。
3. **REPL 命令历史持久化（小项）**：`$HOME/.sokonanoda_history`（截尾
   1000 行；HOME 缺失静默禁用；不做行编辑——超范围另立项）；测试注入
   临时 HOME，既有 repl 测试不再污染真实家目录。
4. **course/ 五单元提示阶梯内容**：20 个 open 练习 × 3 条（共 60 条
   `-- soko:hint`：思路→目标形态→关键件，答案不进提示）；golden 逐单元
   计数不变、solutions 零诊断（注释级改动不产事件——playground 同机制）。
5. 测试总量 **335**（front 171 / lsp 56 / cli 63 / kernel 45…）；全绿；
   fmt/clippy 干净。playground 锚点不变（checked=14 / open=12 / 0 诊断）；
   course 锚点 19 checked · 20 open · 0 failed。

## 本轮进度（2026-09-07，第十一轮：教学辅助四件套，3 subagent 并行 + 2 调研）

> 设计先行：`docs/design-hints-suggestions.md`（提示阶梯/下一步建议）与
> `docs/design-rename-inlay.md`（rename/references/inlay/lsp 子命令）；主会话
> 预接线（协议、能力注册、桩、front 种子）后 4 个实现 subagent 文件集互斥并行。

1. **提示阶梯 `soko/hints`**：画布指令 `-- soko:hint <text>`（独占一行、挂到
   下一条声明，机制 `front::compile::hints` + `DeclState.hints`；注释级编辑走
   Session 零重编译路径并刷新阶梯）；LSP 自定义请求 `soko/hints`（无状态，
   揭示进度归客户端）；VS Code 练习树「提示」节点 + `sokonanoda.revealHint`
   逐条揭示（不预告剩余条数——WPI 实证）；playground 12 题全部挂上
   思路→目标形态→关键件三级阶梯（**答案绝不进提示**，遵守 teaching-session 规则）。
2. **下一步建议（按目标形状）**：`front::suggest`（每请求 ≤3 条、首条
   `is_preferred`）——exact（kernel 判定）、`Eq.refl` rfl 候选（kernel 验证后
   才呈现）、refine（模板）、intro（形状）；`front::judge::judge_hole_fill`
   把洞替换候选后整份交 kernel 终审。**顺带修复多洞错位 bug**：spine 状态下
   「匹配外层 goal 的假设」不再被塞进子洞（逐洞按 `sub_goals[i].ty` 判定）。
3. **rename + find-references**：全语义集（`resolve_at` + `references_for` +
   tokenize 精确名字 token，零文本扫描；注释/字符串天然不误伤）；prepareRename
   返回名字子 span + placeholder；rename 产出**版本化 documentChanges**，
   非法名/不可解析 → ResponseError（不返回空 edit，LSP 3.17 规范）；shadowing
   内层胜出有回归测试。
4. **inlay hints**：每个开放练习的洞尾标注期望类型（`: T`，子洞类型来自
   server 端 walk；单主洞显示剩余目标）+ markdown tooltip（目标 + 假设）；
   只读信息，无 textEdits。
5. **`sokonanoda lsp` 子命令**（单二进制分发，gleam 模式）：`crates/lsp` lib 化
   （`sokonanoda_lsp::run()`），`sokonanoda` 二进制 `lsp` 子命令拉起 stdio 服务器
   （tty 时 stderr 提示）；`sokonanoda-lsp` 二进制保留，VS Code 端不受影响。
6. 测试总量 **327**（front 171 / lsp 56 / cli 55 / kernel 45…）；全绿；
   fmt/clippy 干净（教学 crates 零警告）。playground 锚点不变：
   checked=14 / open=12 / diagnostics=0。

## 本轮进度（2026-09-07，第十轮：gap-analysis 第一批落地）

1. **行业基线 LSP 三件**（主会话）：completions（关键字/宇宙/prelude 名/
   文档声明，单源 `front::semantic::keywords()`；内部名不外泄）、folding
   range（仅多行声明）、`--version`（cli）+ workspace `rust-version = 1.96`
   （MSRV 声明，rust-analyzer 教训）。
2. **导航基线（subagent A，断网后核实其工作已完整落盘）**：go-to-definition
   （elab 记录 use→def 解析映射：局部 binder→binder span、顶层名→声明
   span；shadowing 正确——内层 `x` 解析到内层 binder）、document highlight
   （同定义全部使用点）、binder 补全（光标处 name_scopes 在域名字）。
   新 API：`ResolvedTarget`、`DocumentReport.definitions/name_scopes`、
   `HoverType.scope_names`。
3. **REPL undo（subagent B）**：`ProofState` 快照栈 + `undo`（`u`/`#undo`）；
   intro/exact 成功前入栈、失败不动；cli e2e + 4 单测。lean4game/Isabelle
   的教学基线能力。
4. 测试总量 **273**（front 141 / lsp 33 / cli 35 / kernel 45…）；全绿。

## 本轮进度（2026-09-07，第九轮：subagent 并行 ×3，主会话多洞/refine）

1. **多洞 + refine（I9 第二段，设计 `docs/design-goal-refine.md`）**：
   构造子 spine 走查——`And.intro ??? ???` 等多洞是合法 Open 状态（不再
   hole-misplaced）；子洞期望类型从文档自身的 axiom/ctor 形状实例化
   （参数位=目标自己的实参，证明位=实例化后的字段类型）；
   `DeclState.holes/sub_goals/refine_template` 贯通 LSP——**refine 建议**
   （`And.intro a b ??? ???`：参数自动填充、证明字段留洞，结构来自文档、
   kernel 终审）、`soko/goals` 携带 holes/sub_goals、nextHole 跨子洞环绕。
   front 4 + LSP 3 个新测试。
2. **内核错误分类学（审计 subagent 报告 → 实现 subagent 落地）**：8 个新
   kernel 错误码（`kernel-expected-sort` / `expected-pi` / `theorem-not-prop`
   / `inductive-non-positive` / `ctor-result-mismatch` / `ctor-arg-invalid-app`
   / `ctor-arg-not-type` / `ctor-arg-too-large`），各带中文教学提示；内核
   冷路径 5 处消息增强（两处无消息 assert 加消息、三处 `got:` 渲染）；
   分类器 `refine_kernel_kind`（含 `rejected:` 前缀剥离与 internal 兜底）。
3. **关键稳定性修复**：`#check`/`#reduce` 直通内核求值路径此前无 panic
   保护——`#check (Type) 3` 会**崩掉整个编译/LSP 进程**；现在经
   `quiet_catch` 降级为分类诊断（并接通分类器）。VSIX 真因修复：
   `.vscodeignore` 排除了 node_modules（上轮只移了 dependencies）——实测
   VSIX 从 9 文件/13KB 变为 324 文件/470KB 且含 vscode-languageclient；
   契约测试封死两处回归。
4. **发布流水线（subagent 实现）**：`release.yml`（tag 触发 + dispatch
   dry-run；Rust 双二进制 + VSIX 同 Release）+ `docs/RELEASE.md` 发布手册。
5. **业内标准差距审计（调研 subagent）**：`docs/gap-analysis.md`——Top 10
   补全清单（completions/go-to-def/folding/提示分级/undo/rename/inlay/
   章节地图/下一步建议/--version+MSRV）与反标配清单；已并入 ROADMAP L2/L3。
6. 测试总量 **245**（front 133 / lsp 26 / cli 35 / kernel 45）；全绿；
   clippy/fmt 干净。

## 本轮进度（2026-09-07，第八轮：goal 面板与跳洞）

1. **VS Code goal 面板（I9 收尾，消费 `soko/goals`）**：资源管理器新增
   "练习" 树——每个声明显示 kind·状态，开放练习展开为「目标 + 已引入
   假设」，点击直达洞位；状态栏显示未完成练习数（点击聚焦面板）；诊断
   更新即自动刷新。**`alt+n` / `alt+shift+n` 跳下一个/上一个洞**（环绕；
   位置计算全部在 server 端 `soko/nextHole`——客户端禁止文本扫洞，
   ocaml-lsp 教训落入代码约束）。
2. **客户端契约测试**（`crates/cli/tests/extension.rs`，4 个）：package.json
   声明的命令必须在 extension.js 注册、键位只指向已声明命令、客户端必须
   消费 soko/goals+nextHole 且禁止自算洞位、运行时依赖必须在 dependencies
   （VSIX P0 回归守护）、打包元数据齐全。无需 Electron 即可 CI 守护客户端。
3. **AGENTS.md**（项目指令入口）：opencode/Claude Code 等原生读取——接手
   阅读顺序、角色技能（skills/）、硬规则速记、命令清单、收尾义务。
4. 测试总量 **239**（+4 扩展契约套件）；clippy/fmt 全绿。

## 本轮进度（2026-09-07，第七轮：逻辑先行课程落地 + I9 宇宙携带）

1. **课程重排（用户课程排序哲学落地，REQUIREMENTS §6）**：course/ 5 单元与
   playground 全部重排为逻辑先行——
   ①命题与证明项（先证明命题，全程不谈 Sort）→ ②等式与 rfl（先认识数字，
   `Eq.{1}` 作为机械规则并埋下"为什么是 1"的悬念）→ ③函数与箭头（结尾埋
   "函数类型的类型？"悬念）→ ④宇宙（Sort 由悬念揭晓，回收 Prop=Sort 0）
   → ⑤归纳与递归（不变）。旧 unit1/2/3/4 文件更名重写，题目与钥匙全部
   复用既有 kernel 验证结论（Eq.symm 钥匙修正了一处缺实参的错误——被
   本仓库 LSP 实时抓出，opencode 接线的第一次实战验证）。
   同步：course.json（新文件名/标题）、course.rs golden（新计数 12,5,1 /
   2,5,2 / 1,4,1 / 0,3,0 / 4,3,1）、course/README、skill 的 curriculum.md、
   teaching-session.md §3（12 题新编号）、playground（12 练习逻辑先行为主：
   checked=14 open=12 diagnostics=0）。
2. **I9 余项——开放声明携带宇宙参数**：`DeclState.universe` 贯通
   （OpenExercise op → 报告 → LSP exact_binder → judge 合成声明），
   带 `{u}` 的开放练习（如毕业题 Eq.symm）现在能获得 exact 建议；
   上一轮记录的已知限制清除。测试：front 2 个（记录 + judge 端到端）。
3. 测试总量 **235**（+2 宇宙携带）；golden 更新为刻意变更；clippy/fmt 全绿。

## 本轮进度（2026-09-07，第六轮：agent skills）

1. **skills/ 目录（用户需求：为 code agent 设计 skill 部分）**：Agent Skill
   格式（SKILL.md frontmatter + references）：
   - `sokonanoda-teacher`：判卷接口（--json 事件读法）、3 步教学循环、
     事件决策表、出题规范（含逻辑先行哲学）、解答钥匙守则、编辑器能力
     清单、硬规则；references/events.md（事件形状 + 增量语义）与
     references/curriculum.md（题池地图 + 适配规则）；
   - `sokonanoda-dev`：接手清单（REQUIREMENTS→STATUS→ROADMAP→architecture）、
     硬规则、TDD 三层 + 文档先行 + subagent 工作流、CI 门禁形态。
   - 安装方式见 `skills/README.md`（软链到 harness 的 skills 目录）。
2. **conformance 守护**：`crates/cli/tests/skill.rs`（4 测试）——frontmatter
   合法且 name=目录名、引用的仓库路径必须存在、事件/方法词汇封闭且必须被
   `docs/protocol.md` 记载（词汇表抽到 `crates/cli/tests/common/mod.rs`，
   protocol.rs 与 skill.rs 共用）；course.json 的单元/钥匙孪生存在性校验。
   protocol.md 补上 watch 流（Session delta）词汇一节。
3. **opencode LSP 接线**：仓库根 `opencode.json` 把 `sokonanoda-lsp` 挂到
   `.sokonanoda` 扩展名（`cargo run` 启动，无预构建要求）——opencode 等
   agent 打开教学文件即自动消费 kernel 判定诊断；README 增设
   "For code agents" 一节。
4. 测试总量 **233**（+4 skill 套件）；clippy/fmt 门禁维持全绿。

## 本轮进度（2026-09-07，第五轮：I8 真增量 + I9 + 内核修复 + 工程达标）

本轮按"先调研后动手"执行（4 个并行 subagent：代码审计 / LSP 增量业界实践 /
goal 视图 UX / VSCode+CI 标准），设计文档 `docs/design-i8-i9.md`，全程 TDD。

1. **I8 真增量（front，零内核改动）**：学 Lean4/coq-lsp 的"前缀精确复用 +
   变化点后保守重算"。`run_pass` 增加 `TrustPlan`：信任前缀照常 elaborate +
   入环境但**跳过内核重查**（内核检查是贵的那一半）；失败声明不入环境
   （check-then-add 语义保持）。`Session` 快照升级为逐命令
   `{state, hovers, events, errors}`，文本不变 → 零重编译且**修复了注释/空白
   编辑导致的 span 漂移 bug**（重映射坐标）；`first_diff` 之后才重查。
   `SessionUpdate.stats.kernel_checks` 让"改第 i 个声明只重查后缀"可验证
   （测试：5 声明改第 4 → kernel_checks == 2）。LSP Backend 切换到 Session
   （此前每次编辑 2×2 遍流水线，现在前缀零内核重查）。prelude 指令变化时
   整体重建（决策依赖整文件内容，语义与全量严格等价）。
2. **I9 tactic 判定 kernel 化（`front::judge`，零 kernel 原语）**：
   合成完整声明 `def _soko_judge_k : forall binders, 剩余目标 := …术语…`
   走标准流水线，kernel 是唯一裁判。LSP `exact` 与 REPL
   `exact/apply/assumption` 全部接入；`proof.rs` 文本比对删除
   （REQUIREMENTS §2.8 清账）。defeq-但-不同文本的假设（`a -> False` vs
   `Not a`）现在能被识别。goal 视图协议：`soko/goals`（结构化多洞 goal 列表）
   与 `soko/nextHole`（server 端位置计算，ocaml-lsp 教训）两个自定义请求。
3. **内核 soundness 修复（上游 bug，本轮最重要发现）**：judge 端到端测试暴露
   conv `unify_direct` 的 Pi/Lam body-expr 快路径把 **eval 闭包与 infer 闭包**
   按体表达式指针判等（同一 `Var 0` 在两种闭包下是 `$0` vs `Sort 1`），
   `(A : Sort 1) -> A` 这种不可居住类型被身份 lambda 通过（官方 Lean 拒绝）。
   修复 = 快路径增加闭包语义守卫（`closure_ctxs_compatible`，热路径仅一个
   判别分支），回归测试三层（kernel 2 + CLI 2 + 全量语料）。
4. **工程达标（业内标准）**：CI 增加 lint job（fmt + clippy）；`actions/cache`
   → `Swatinem/rust-cache@v2`；`--locked`。lint 门禁形态：教学 crates 在各自
   `Cargo.toml` 用 `[lints.rust] warnings = "deny"` 注入严格度，kernel 冻结
   快照不参与（其 `lib.rs` 的 `deny(cast_possible_truncation)` 降为 warn，
   上游代码自身未过该 lint）。fmt 门禁只覆盖教学 crates（kernel rustfmt.toml
   需要 nightly）。VS Code 打包 P0：`vscode-languageclient` 移到 dependencies
   （此前打出的 VSIX 装上即坏）、补 repository/LICENSE/CHANGELOG/.vscodeignore、
   `vsce package` 冒烟通过。
5. **课程哲学修正（用户插话，已记录 REQUIREMENTS §6）**：逻辑先行——先讲
   True/False/And/Or/Iff/Forall/Exists 让学生在"证明命题"里建立直觉，Sort 等
   到"函数类型的类型是什么"这一自然问题出现时再引入；course/ 与 playground
   按此重排（**下一轮任务**）。
6. 测试总量 **229**（kernel 45 / front 121 / cli 40 / lsp 23），
   全绿；`cargo clippy --workspace` exit-0，教学 crates 0 警告。

## 本轮进度（2026-09-07，接手 agent 第 1–3 轮）

1. **模块化重构（用户要求：不得单文件巨石）**：`front/lib.rs`→
   `span/token/ast/diagnostic/parser + compile/{mod,error,event,report,elab,prelude,check}`；
   `cli`→`main/check/json_report/repl/help`；`lsp`→`main/render/actions`；
   公开 API 全部 re-export 保持稳定；**kernel 一行未动**（性能原则）。
2. **全流水线测试资产（157 tests 全绿，见 `docs/TESTING.md` 地图）**：
   kernel 41+2 / arena 1 / memory 1；front 49→**74**（lexer 10 / parser 10 /
   compile 51，含 ErrorKind 矩阵、DocumentReport 状态机、doc-conformance、perf 冒烟）；
   cli 21→**21+8**（新增 protocol golden：封闭事件词表、lesson-01/02 金字、
   协议文档防漂移）；**lsp 0→10**（内存内 LspService 协议级集成测试）。
3. **测试揪出并修复的真实缺陷**：
   - LSP `intro` quick-fix 行列 +1 偏移（actions.rs 1-based→LSP 0-based）；
   - publishDiagnostics 补 `version`；didChange 改取最后一个 change（FULL sync 语义）；
   - `--json` 的 elab/kernel diagnostic 补 `hint` 字段（protocol.md 本就承诺）；
   - `docs/protocol.md` 补齐 6 个缺失 elab 错误码（doc-conformance 测试守护）。
4. **文档**：新增 `docs/REQUIREMENTS.md`（用户全部要求的权威总账）、
   `docs/TESTING.md`（测试资产地图）、`docs/lsp-notes.md`、`docs/vscode-notes.md`。

## 本轮进度（2026-09-07 第四轮：I8 + I9 后半 + 语义高亮 + watch）

1. **语义高亮（F8，用户要求）**：front 新增 `semantic.rs`（keyword/sort/number/
   hole/声明名/构造子/binder 分类，声明点优先）；LSP 实现
   `textDocument/semanticTokens`（UTF-16 编码正确处理增补平面字符——修掉了
   `span.column` 是字符计数的错位隐患）；VS Code 端 vscode-languageclient
   自动注册，零配置生效。
2. **I9 kernel 显式错误**：6 处 def_eq 失败点（def-like/App 实参/let 体/
   inductive 参数/表达式）在 panic 前用 debug printer 渲染两端（≤200 字符截断），
   稳定格式 `def_eq mismatch expected: <E> | actual: <A>`；front 解析为
   「类型不匹配：期望 `E`，实际是 `A`」并填充 `CompileError.expected/actual`；
   热路径零改动，内核 41 测试全绿。
3. **I8a check-then-add（双趟）**：kernel 拒绝的声明不再占用名字——第二趟在
   干净环境中重算，依赖者得到真正的 unknown-identifier 诊断；归纳块首次纳入
   kernel 判定（`EnvBuilder` 新增 `begin/end_inductive_block` +
   `mutual_block_sizes` 记账，对齐上游 parser；`add_inductive` 返回构建的
   Declar）。由此暴露并修正了 py-nat 系 iota 规则的两处非标准写法：
   规则值必须是 `ms n (Rec.{u} motive mz ms n)`（`.{u}` 不能省）。
4. **I8b Session**（front::session）：版本号、delta 事件
   （exercise.opened/solved/failed、decl.checked/failed）、`recompiled_from`
   日志；内容未变（仅注释/空白）零重编译。
5. **I8c watch**：`sokonanoda watch <file>` 常驻监控 → `file.changed` +
   delta + 诊断的 JSON Lines 流（L1 服务层的 CLI 形态）。
6. 测试总量 **203**（kernel 43 / cli 38 / front 103 / lsp 20... 计 CLI 子套件见
   docs/TESTING.md）。

## I6 落地（2026-09-07 第二轮）

全部四件套已实现并通过 178 个测试（kernel 43 / cli 35 / front 87 / lsp 13）：

1. **prelude 可选化（用户要求）**：`CompileOptions{prelude: PreludeMode::{Full,Bare}}`；
   API `compile_fol_with/check_document_with`；CLI `--bare`；文件级注释指令
   `-- sokonanoda:prelude none`（front::prelude_mode_from_source，CLI/LSP 都认）；
   Bare 模式下 `+` 不再产生悬空 Nat.add 常量（改报 elab-unknown-identifier）。
2. **Eq 三件套 prelude**：`Eq`/`Eq.refl`/`Eq.subst` 以 `.sokonanoda` 源语法书写
   （签名与官方 Lean 一致），受信任安装；文件自带 Eq 系列则整体跳过
   （all-or-nothing，与显式 Nat 块一致）。
3. **binder 类型推断**：`elab_expr` 下传 expected（声明类型逐层剥 Pi），
   `fun n => n + 1` 免写 `(n : Nat)`；依赖情形（`forall (α : Sort u), α -> α`）
   因 de Bruijn 对齐天然支持；声明类型耗尽仍报 `elab-untyped-binder`。
4. **partial hole（部分作答）**：`???` 允许出现在 lambda 体尾部；声明保持
   Open 且 `DeclState.goal` = 剥掉已写 binders 后的剩余目标；洞在非尾部位置
   仍报 `elab-hole-misplaced`。LSP intro quick-fix 的「替换 ??? →
   fun (x : T) => ???」循环第一次真正闭环。
5. **开课**：根目录 `playground.sokonanoda`（12 练习初始全 open、0 诊断、
   exit 0）；`docs/teaching-session.md` = 开课手册 + 事件决策表 + 全部解答钥匙
   （12/12 经完整内核验证，含 `two_def` 闭环）。
6. 新增裸名 `#reduce Nat.add/Nat.succ` 边界测试（I6 验收项，防 delta 循环）。

## 本轮进度（2026-09-07 第三轮：I7 课程层 + I9 goal 视图第一段 + VS Code 修复）

1. **I7 课程层**（用户定位：course/ = agent 路线图，执行层由 agent 按用户灵活
   适配——已写入 REQUIREMENTS §6 与 teaching-session §0）：`course/` 5 单元
   画布 + `course.json` 顺序清单 + `solutions/` 解答钥匙（全部经完整内核验证
   可解）+ `crates/cli/tests/course.rs` golden（每单元 decl.checked/exercise.open/
   expr.reduced 计数钉死）+ CI 步骤。
2. **I9 goal 视图第一段**：开放声明现在携带已引入假设清单
   （`DeclState.binders: Vec<GoalBinder{name, ty}>`，goal_under_binders 同步
   记录）；LSP hover 在 `???` 上显示「目标 + 已引入假设」；code action 新增
   **`exact <假设>`**（类型与目标匹配时自动提议，洞替换为该假设名），
   与既有 `intro` 并存；LSP 测试 13→16。
3. **VS Code 薄壳修复**（依 docs/vscode-notes.md）：修复 `client.start()` 未作为
   disposable 注册的真实 bug（改为正确 start/stop 生命周期）；`sokonanoda.serverPath`
   设置 + 自动发现（workspace target/debug|release → PATH）；`alt+s` 状态命令
   （documentSymbol → 快速选择面板）；codeLens 的 `sokonanoda.status` 命令
   补了客户端 handler（此前点击报 command not found）；新增 F5 启动配置。
4. 测试总量 186（kernel 43 / cli 38 / front 89 / lsp 16）。

## 下一步（交接快照，2026-09-07；详细依据见 docs/gap-analysis.md）

> 交接要点：新机器 `git pull` 后先跑
> `cargo test --workspace --locked`（应 273 全绿）+ `cargo clippy --workspace
> --all-targets`；阅读顺序见根 `AGENTS.md`。

### 下一批候选（按投入产出比排序）

1. ~~**提示分级 `soko/hints`**~~ ✅（第十一轮）：画布 `-- soko:hint` 指令 +
   `soko/hints` 请求 + VS Code 逐条揭示；playground 12 题已挂阶梯。
   余项：course/ 五个单元的内容阶梯（教学轮补）。
2. ~~**失败洞的"下一步建议"**~~ ✅（第十一轮）：`front::suggest` 按目标形状
   （exact/rfl/refine/intro，kernel 验证优先 + is_preferred）；顺带修复多洞
   错位 bug。余项：失败声明（kernel-rejected）的针对性建议。
3. ~~**`soko/courseStatus` + VS Code 章节地图**~~ ✅（第十二轮）：聚合归
   `sokonanoda course` CLI 子命令（服务器保持单文档），VS Code「课程」树
   消费子进程 JSON Lines；学习者进度=画布自身状态（声明式文件即存储）。
4. ~~**rename + find-references**~~ ✅（第十一轮）：语义集 + 版本化
   documentChanges + ResponseError；shadowing 有回归测试。
5. ~~**inlay hints**~~ ✅（第十一轮）：洞期望类型 + tooltip；只读无 textEdits。
6. **内核错误分类学余项**：~~`conv.rs` 与 `infer.rs` 同名消息区分~~ ✅
   （第十三轮：统一 `got:` 形状、措辞区分站点）；~~`assert_eq!` 灰色地带~~ ✅
   （第十三轮全量清点分诊）；refine 子洞的 kernel 级 expected type
   （elaborator spine meta，M–L）**仍为余项**——需专门设计轮。
   另：~~归纳块 auto-derivation~~ ✅（第十四轮）。~~refine 子洞的 kernel 级
   expected type~~ ✅（第十五轮方案 B′：深度 AST 替换；方案 A 记为远期）。
7. **小项打包**：全部 ✅（lsp 子命令 / REPL 历史 / criterion / fuzz /
   hole_id）。**小项全部清零。**

### 运营/验证类

- **release.yml 首跑验证**：`git tag v0.1.0 && git push --tags` 后核对
  Release 产物（双二进制 + VSIX；见 docs/RELEASE.md §6 风险清单——
  taiki-e 多 bin 映射、softprops、gh release upload 均需首次实测）；
  `npx @vscode/vsce` 建议钉版本。
- **教学回环实战**：逻辑先行画布已就绪（course/ + playground，12 题）——
  找真实学习者走一遍 `skills/sokonanoda-teacher` 循环，回收提示分层与
  事件决策表的打磨需求。
- **opencode.json 实战核查**：LSP 经 opencode 消费的体验（已有一次实战：
  抓出 Eq.symm 钥匙错误）。

### 更远（L2/L3）

VS Code 扩展集成测试（@vscode/test-electron）、发布 marketplace、
L1 service 事件流（watch 已是 CLI 形态）、KernelError 显式化完整推进
（`CheckError::Internal` 目前无人构造）。

## 已确认的决策（用户 2026-09-06）

1. 命名练习：`def name : T` / `theorem name : T`，匿名用 `example`（官方 Lean 的
   `example` 不能带名字；不发明非 Lean 的 `example name : T`）。
2. LSP 框架：用现成 tower-lsp。
3. 范围：I1–I9 全部实现；goal 视图也进第一期。
4. 反馈目标：**足够细致、足够详细**的 LSP（能力清单见 design doc F1–F8）。

## 已完成（按 commit）

| 范围 | 内容 | 位置/commit |
|---|---|---|
| I0 地基 | `--json` 事件、错误 stage/code、语料 CI、examples 语料测试 | `2926347` |
| 文档 | architecture / research / design v1 三件套 | `bf3441b` |
| 设计 v2 | LSP-first、文件无 `#`、练习=带洞声明 | `c1db151` |
| I1 逐声明状态 | `DocumentReport`/`check_document`：open/checked/failed，练习带名字与目标；开放/失败声明不影响后续 | `f23ca71` |
| I2 错误细分 | `ErrorKind` 稳定 code（`elab-*`/`kernel-rejected`）+ 教学 hint（CLI/JSON/LSP 三处） | `f23ca71` |
| I3 类型图 v1 | elaboration 记录每个子表达式 (span, 内核项, binder 作用域)；kernel 新增 `infer_under_binders` → hover 表 | `f23ca71` |
| I4/I5 第一段 | `crates/lsp`（tower-lsp）：diagnostics/hover(类型+目标)/documentSymbol/codeLens/quick-fix `intro`；`editor/vscode` 薄壳 | `f23ca71` |
| 细节 | 事件按源码顺序输出（open 练习排队处理） | `bacb1ee` |

里程碑对照（ROADMAP 第 5 节）：M0 ✅、M1 ✅、M2 大部分（判定细节/提示在 I2 完成；
"期望目标类型 vs 实际" 的 kernel 比对待接）、M3 协议文本+JSON 已实现（service 增量待接）、
M4 课程内容 ❌、M5+（L1 service / L2 完整编辑器 / L3 agent）未开始。

## 测试现状

`cargo test --workspace` 全绿：kernel lib 41（2 ignored：缺 fixture）、arena 1、
memory_api 1、front 49、cli 21、examples 语料 1。LSP 服务器做了手动 JSON-RPC 冒烟
（initialize/didOpen 诊断 0/hover `x: Prop` 与 `???` 目标/documentSymbol/intro
quick-fix/坏声明 `kernel-rejected`）。

## 怎么跑 / 验证

```bash
cd /Users/penglingwei/Documents/lean/sokonanoda/sokonanoda-lang
cargo test --workspace
cargo run -q -p sokonanoda-cli --bin sokonanoda -- examples/lesson-01.sokonanoda
cargo run -q -p sokonanoda-cli --bin sokonanoda -- --json examples/lesson-01.sokonanoda
cargo run -q -p sokonanoda-cli --bin sokonanoda repl          # #check/#reduce/#prove 调试
cargo run -q -p sokonanoda-lsp --bin sokonanoda-lsp           # LSP（editor/vscode 使用）
```

## 待办（按依赖排序，全部已入 ROADMAP §10）

- **I6 prelude 对齐 + elaborator 推进**：Bool/Eq/rfl 等受信任基元；核对 Nat.succ/Nat.add
  占位自引用体；binder 类型推断 → `let` → `match`。验收：每个语法点 TDD 三件套。
- **I7 第一门课（M4）**：5 单元（表达式与类型 / 函数与箭头 / 命题与证明项 / 等式与 rfl /
  归纳与 match）× 3–8 练习，`course/` 目录 + golden 事件；CI 全绿。
- **I8 真正增量**：check-then-add（失败的声明不进环境），编辑一行只重查受影响后缀；
  事件带版本。
- **I9 kernel 显式错误 + goal 视图**：panic→`KernelError`（conv 差异给两端项）；
  `#prove` 逻辑入库，LSP 多洞 goal/refine/code action。
- **L2/L3（后续）**：VS Code 扩展打包（语法+进度树+goal 面板）、L1 service 事件流、
  讲课 agent 接入同一文档状态。

## 给接手 agent 的提醒

- 判定永远走 kernel，不做文本比对（`proof.rs::assumption` 的文本比对是草案，待替换）。
- 新增语法 = 课程 + 测试 + 白名单；`???` 只允许在声明值位。
- kernel 拒绝目前仍是 panic→`Result`（`try_check_declar`）；细粒度 kernel 错误是 I9。
- arena 生命周期：`EnvBuilder`/`ExportFile` 挂同一 `stumpalo::Arena`，必须活得比检查会话久。

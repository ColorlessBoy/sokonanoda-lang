# 当前状态与进度日志（agents 先读这里）

> 快照：2026-09-07（第八轮：VS Code goal 面板 + nextHole + AGENTS.md）
> 仓库：`sokonanoda-lang`；权威计划 = `ROADMAP.md`；**用户要求总账 = `docs/REQUIREMENTS.md`（先读）**；
> 设计 = `docs/design-i8-i9.md`（I8/I9）/ `docs/design-infrastructure.md`（历史）；
> 架构/内核 = `docs/architecture.md`；协议 = `docs/protocol.md`；测试地图 = `docs/TESTING.md`；
> agent 入口 = `AGENTS.md` + `skills/`（sokonanoda-teacher / sokonanoda-dev）；
> LSP/VS Code 调研 = `docs/lsp-notes.md` / `docs/vscode-notes.md`。

## 一句话

`.sokonanoda` = **纯声明式教学文件（无 `#` 命令）+ 完整 sokonanoda 内核 + LSP 反馈通道**。
练习 = 带 `???` 洞的 `def name : T` / `theorem name : T` / `example : T` 声明。
CLI/REPL 的 `#check` 等只是调试/自测工具，不是文件格式。

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

## 下一步（按 REQUIREMENTS §8 路线）

- **课程哲学落地**：course/ 5 单元与 playground 重排为"逻辑先行"
  （REQUIREMENTS §6 课程排序哲学）：单元① 逻辑连接词与证明项，Sort 由
  "函数类型的类型"问题自然引出。
- **I9 余项**：goal 视图携带声明宇宙参数（带 `{u}` 的开放声明目前无 exact
  建议）；refine/multi-hole；`soko/goals` 的 VS Code 客户端消费（goal 面板）。
- **L2/L3**：VS Code 扩展集成测试（@vscode/test-electron）与打包发布流水线；
  L1 service 事件流（watch 已是 CLI 形态）。
- **内核余项**：panic → 显式 KernelError 的完整化（def_eq mismatch 已闭环，
  其余拒绝路径仍是 panic 包装）。

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

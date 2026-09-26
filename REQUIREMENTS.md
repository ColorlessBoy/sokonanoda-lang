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

## 9. 现行要求（历史追加日志 → `docs/archive/REQUIREMENTS-ARCHIVE.md` ✓）

> **为什么拆** ✗：本节曾是 **2989 行**（2123 行日期日志 + 609 行交付报告 + R/P 条目）
> —— 而"权威总账"要的是**现行要求** ✓，不是流水账 ✗（用户 2026-09-26 要求 ✓）。
> **归档 ≠ 销毁** ✓：**原文逐字**在 `docs/archive/REQUIREMENTS-ARCHIVE.md`（**纯文本、
> 可 grep** ✓，带 **139 条索引** ✓）；**判据与守卫**仍在 `scripts/` 与
> `crates/**/tests/` 里**真跑** ✓。判据：`python3 scripts/docs-lint.py` ✓（入口 ≤800 行 ✓）。

### 9.1 产品要求（R-1…R-4，**仍生效** ✓）

> 用户 2026-09-24 提出，四条**全部交付**（0.66.0 / 0.67.0）✓；**行为契约仍然有效** ✓
> —— 交付过程与全部证据在归档里（每条给锚点 ✓）。

| # | 要求（现行规范 ✓） | 交付 | 守卫 / 判据 | 归档锚点 |
|---|---|---|---|---|
| **R-1** | Infoview 的 `def` 卡片必须显示第二行 `:= <值>`：`value`/`value_runs` **必须走 wire** ✗（LSP 漏映射 = 用户看不见而三层测试全绿 ✗） | ✅ 0.66.0 | `crates/lsp/src/tests/goals.rs` ✓ · **A∖B 对账** `python3 scripts/audit-wire-fields.py` ✓（回退 R-1 ⇒ 必须报 `value_runs` ✓） | 归档 §R-1 |
| **R-2** | 目标面板**必须高亮**（`goal_runs`/`goals_runs` 走 wire + `infoview.js` 渲染 `⊢` 行 ✓）；目标文本**完全记法化** ✓；`=` **不得**被词法当声明符号吃掉 ✗ | ✅ 0.66.0 | front 单测（父子成对/对齐/反例）✓ · LSP wire 单测 ✓ · `node editor/vscode/test-webview.js` ✓ · **真宿主 e2e** ✓ · as-built `docs/design/goal-rendering.md` §9 ✓ | 归档 §R-2 |
| **R-3** | 项目模式在**模块根**建 `.sokonanoda/`：产物落 `<模块根>/.sokonanoda/compiled/<key>.json`，**vscode 与 code agent 共用**（避免重复计算 ✓） | ✅ 0.67.0 | `crates/cli/tests/artifacts.rs`（5 条真进程 ✓）· 设计 `docs/design/project-artifacts.md` ✓ | 归档 §R-3 |
| **R-4** | VS Code 命令名统一 **`Sokonanoda: <Command> (说明)`**（前缀固定 ✓、命令词首字母大写 ✓、括号中文说明 ✓） | ✅ 0.67.0 | `crates/cli/tests/extension.rs`（声明↔注册一致 ✓）· 同步义务见 §9.2 末条 | 归档 §R-4 |

> **R-2 的 (a) 半仍**未做** ✗**：λ 操作数补不出前导类型参数 ⇒ 属**记法引擎特性**
> （记法化现场 #1/#3）⇒ 在那之前那两条只能写**显式形式** ✓（豁免注释即为此存在 ✓）。
> 详见归档 §R-2 与 `docs/design/goal-rendering.md` §9。

### 9.2 流程纪律（用户 2026-09-24 拍板；**硬要求** ✓）

**P-1 验证设计纪律**（**没设计正确就是白做功** ✗）—— 四条，逐条可判：

1. 每条**用户可见**改动先回答一句：**"屏幕上会多/少什么？那条断言在哪一层？"**
   —— 答不上来 ⇒ **验收不完整**，不许勾环节 ✗；
2. **三层各司其职** ✓：front/CLI 单测 = **真相**（算得对）· LSP 单测 = **wire 契约**
   （**字段存在性**，不只是值）· **e2e = 用户看到的东西**（**渲染结果**，不只是"数据在"）；
   Infoview 另有 `node editor/vscode/test-webview.js`（stub DOM 里跑**真的** `media/infoview.js`）；
3. 凡"A 层产出、B 层消费"的字段都要 **A∖B 对账** ✓：`python3 scripts/audit-wire-fields.py`
   （已进 gate 与 CI ✓）；**反向验证是硬要求** ✗：守卫必须能**咬住已知的历史 bug**
   （回退 R-1 的 `value_runs`、或 T-A5 的 `goal_runs`/`goals_runs` ⇒ 必须报红 ✓）；
4. **「有就渲染」是反模式** ✗：宽容消费者会**掩盖契约破坏** ⇒ 出路是
   ①契约测试覆盖它（首选 ✓）或 ②字段缺失时给**可见信号**（至少 `console.warn` ✓）。

**P-2 并行与 subagent 纪律** —— 六条：

① subagent 只做**可并行 / 只读 / 边界清晰**的活（调研、逐文件审计、多角度验证、
写复现件、量基准）✓；**判定相关代码与发布闭环走主线** ✗（内核改动、bump/release、
跨模块重构不外包）；② 并发上限 **2–4** ✓，**写操作串行** ✗（同一文件/同一模块
同一时间只允许一个写者）；③ 每个 subagent 的 prompt **必须自带三样** ✗：规则与
陷阱摘要 ✓ + **可执行判据**（不是"看看对不对"）✓ + 明确的"**不许改什么**"边界 ✓；
④ 产出**验证后**才并入 ✓（复跑它给的判据、抽查结论）；⑤ 发现**要回写文档** ✓
⇒ 下一个 subagent/下一轮能复用；⑥ **提交粒度不变** ✓：一个环节一个 commit ✓、
一阶段多 commit ✓、**阶段收尾才 push** ✓。

**同步义务**（用户可见改动 = 同一轮五处 ✗）：`editor/vscode/`（README/CHANGELOG/
package.json）**与** `skills/` 三个技能 + `AGENTS.md` + `docs/vscode-dev-guide.md` ✓
（`crates/cli/tests/skill.rs` / `dsh.rs` 挡住漂移 ✓）。

### 9.3 预算纪律（用户 2026-09-26 ✓）

**① `STATUS.md` 瘦身**（`scripts/status-lint.py` ✓，已进 gate 与 CI ✓）：只留两类 ——
顶部「**当前快照**」（≤40 行 ✓）+ **最近 ≤3 轮**（每轮只写"变了什么 / 现在的状态 /
未决项"，每段 ≤30 行 ✓）；**总行数 ≤200** ✗ · **禁词命中 = 0** ✗
（`在跑`/`进行中`/`未变`/`判据不变`/`待 CI`/`等 CI`/`⏳`）· **净增 ≤60 行**（与 `HEAD` 比 ✗）；
逐轮过程与瞬时状态进 `docs/STATUS-ARCHIVE.md`（**或干脆不写** ✓ —— CI 页面自有 ✓）；
未决项用固定措辞「**未决**」✓。

**② 文档瘦身 + 判据守护**（用户原话：「**文档太重了，还没实现多少东西文档先爆炸了**」✗
—— **落成机制，不要口号** ✗；`scripts/docs-lint.py` ✓，已进 gate 与 CI ✓）：
三类**分治** ✓ —— **活规范**（短、准）· **过程记录**（只留**结论 + 指针**，
更早的进 `docs/archive/` ✓）· **垃圾**（直接删 ✓）。**六条判据**：

① **活文档总量 ≤ 3.0 MB**（= 仓根 `*.md` + `docs/**`，**不含** `docs/archive/**` ✗）；
② **单文件 ≤ 2000 行**；③ **入口文件 ≤ 800 行**（`AGENTS.md`/`README.md`/
`REQUIREMENTS.md`/`ROADMAP.md`/`STATUS.md`/`docs/README.md`/`docs/HANDOVER.md`/
`docs/E2-HANDOVER.md`/`docs/design/e2-plan.md`）；④ **设计文档**：`docs/design/**`
**新增** ≤150 行 ✓、**既有**按 `scripts/docs-budget.json` **冻结**（**只许减不许增** ✗
—— 要放宽必须手改那份 JSON ⇒ 评审可见 ✓）；⑤ **禁垃圾**：`docs/**` 下不得有
`.tmpdir` / `.tmp` / `.tmp-<pid>` / `.DS_Store` / `*.orig` / `*.rej` / `*~`
（**含未跟踪的本地残留** ✗）；⑥ **归档可追溯**（红线 ✓）：`docs/archive/**`
每个文件必须在 `docs/archive/README.md` 里**被点名** ✓，归档总量 ≤2 MB ✓。

**红线**（不许动 ✓）：**判定正确性、红线证据与可追溯性** ✗ —— **归档 ≠ 销毁** ✓：
任何"证据/判据/复现件"必须**仍能找到** ✓（给归档路径与索引 ✓）；
**不许为了让数字变绿而删证据** ✗。**新环节的文档预算** ✓：设计先行**只写契约
不写过程** ✗（过程进 commit message 与 `STATUS.md` ✓）。

### 9.4 历史索引（一行一条；**全文见归档** ✓）

* **归档**：`docs/archive/REQUIREMENTS-ARCHIVE.md` —— **3146 行逐字原文** ✓
  （移入前 §9 = **2989 行** ✗）+ **139 条索引** ✓；**纯文本、可 grep** ✓
  （它是"要求的账本"，**可检索性优先** ✓，不 gzip）。
* **跨度**：**2026-09-06 → 2026-09-26**（接手 → 内核/课程 → 编辑器/发布 →
  课程标准库 → 站点/记法化 → E1 → E2 的 50 环节与 0.66–0.72 发版 → 预算纪律）。
* **现状要求** = 本文 §1–§8 + §9.1/§9.2/§9.3 ✓；**其余一律是历史** ✗。

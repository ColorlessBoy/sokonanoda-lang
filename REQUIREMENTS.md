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
11. **性能是不可牺牲的产品要求**（2026-09-29 用户拍板「性能的重要性强调得还不够」）：
   任何改动**都不得让端到端编译变慢**（口径 = 用户按一次 `rebuild` 要等多久，见 §3）；
   **改动前后必须用 `scripts/profile-course.py` 量**（真课程 · release · **同机同口径**，
   方差 ±2.7% ⇒ **<5% 差不许当结论**）；**变慢即判红**。
   判据**用结构计数或比值，不许用绝对毫秒**（共享 runner 上会假红，已栽三次）；
   每个性能声明**必须配一个"真会拦"的守卫**，逐条核对（§3.1 的表）。

## 3. **端到端编译速度**是产品的第一优势（2026-09-06 立；**2026-09-29 改锚点**）

⚠ 旧标题「**内核**性能是产品优势」**锚错了对象**：内核微基准快是事实，但**用户感受不到**。
实测：真课程冷编 **219.3s** 里 **≈88% 是 `judge` 的前端架构**（每次重跑整段前缀 ⇒ O(N²)）
—— **不在内核的账上**；旧措辞「不得触碰 `crates/kernel` 热路径」**字面上禁止了内核配合性能优化**。

- **优势的定义 = 用户按一次 `rebuild` 要等多久**，**不是内核微基准**；性能声明按此口径量判。
- **内核的职责 = 为性能提供能力**（可增量扩展的环境 / 只读借出 / `EnvBuilder`），**不是禁区**；「内核不可碰」改为「**内核不许无理由变慢**」——**有实测依据的性能改动鼓励做**。教学功能**不得给内核加无谓的间接层**。
- **语义红线（永远成立）**：判定语义**绝对不许变** —— 接受/拒绝不变 · 事件计数不变 ·
  `--json`/报告**逐字节不变**。核心价值是「**不发明第二套判定逻辑，由完整 kernel 当裁判**」
  ⇒ 语义一变就是换了个编译器 ✗（先例：`NatLit` **按指针比较** ⇒ "重建等价环境表"
  会改判定结果；那条路被否**不是因为它动内核，是因为它错**）。
- **反向判据**：前缀/依赖**真变了必须重算** —— 缓存住错误结果比慢严重 ✗。

**3.1 每条性能声明必须配一个"真会拦"的守卫**（与 `AGENTS.md` 同源；**对不上 ⇒ 补守卫或改声明**）：

| 声明 | 守卫 | job | 真拦吗（2026-09-29 实测） |
|---|---|---|---|
| 端到端不许变慢 | `profile-course.py`（真课程 · release · 同机） | 本地/收尾 | ⚠ **只报不拦**（CI 一轮 ~20 分钟，不每 push 跑）|
| 重复功不许涨 | `check-recompile-factor.py`（**测次数**） | `gates-fast` | ✓ **真拦**，**噪声免疫** |
| 课程质量不许退 | `courses/set-theory/tools/check.py`（G1–G5） | `gates-course`（矩阵 4 片） | ✓ **真拦**（一片红即整体红）|
| 墙钟大幅退化 | `perf-gate`（同 runner 家族基线） | `perf-gate` | ✗ **只报不拦**（绝对百分比挡不住 **7.5×** 机器差）|
| 合成夹具不许变慢 | `crates/front/tests/perf*.rs` | `test` | ⚠ 只抓数量级（20×）|

**已知缺口（如实记）**：端到端那条**没有 CI 守卫**。替代：① `check-recompile-factor.py` 守住**最大的单一病因**；② `profile-course.py` 收尾时人跑、数字进 commit message；③ **下一步**做成**同一 run 内自比**的比值判据（不是跨 run 比）才可能在 CI 真拦。

## 4. 工程标准：模块化、单文件不许越长越大（2026-09-06，用户要求）

- **当前欠账**：`crates/lsp/src/lib.rs` **3949 行**、`crates/front/src/compile/tests.rs`
  **3564 行**（后者是测试，暂豁免）—— 两者都远超 ~500 行红线，排期见 ROADMAP
  （intro 一族 → `lsp/src/intro.rs`）；
- **目标结构**（公开 API 用 re-export 保持稳定，调用方不改）：`front/src/`
  `{span,token,ast,diagnostic,parser,proof}.rs` + `compile/{mod,error,event,report,elab,prelude,check}.rs`；
  `cli/src/` `main.rs` + `{check,json_report,repl,help}.rs`；
  `lsp/src/` `main.rs` + `{backend,render,actions}.rs`；
- **`crates/kernel` 不再是"上游快照不许动"** ✗ —— 2026-09-21 用户已解冻（**含热路径，
  目的可以是提速**）；唯一红线是**判定正确性不变**（见 §3），改动台账在
  `docs/architecture.md` §6（**改内核前先读它 + §8 gotchas**）；
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
  `docs/architecture.md`（架构事实）、`docs/teaching-session.md`（教学循环）、
  `docs/protocol.md`（事件协议）；〔外部标准调研 `lsp-notes`/`vscode-notes` 已归档 ✓〕
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

**P-1 验证设计纪律** 与 **P-2 并行与 subagent 纪律** 的**正文已收敛到 `AGENTS.md`**
（同名小节，逐条可判、含陷阱与先例）——**那里是唯一真相**，此处不复制（避免两处漂移 ✗）。
一句话提要：**P-1** = 用户可见改动先答"屏幕上会多/少什么"·三层各司其职（真相/wire 契约/
用户可见结果）·接缝要有 **A∖B 对账**且**咬得住历史 bug**·「有就渲染」是反模式。
**P-2** = subagent 只做只读/边界清晰的活、并发 ≤2–4、prompt 自带三样（规则摘要 +
**可执行判据** + 不许改什么的边界）、产出验证后并入、发现回写文档、提交粒度不变。

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
`REQUIREMENTS.md`/`ROADMAP.md`/`STATUS.md`/`docs/README.md`/`docs/ONBOARDING.md`/
`docs/design/e2-plan.md`）；④ **设计文档**：`docs/design/**`
**新增** ≤150 行 ✓、**既有**按 `scripts/docs-budget.json` **冻结**（**只许减不许增** ✗
—— 要放宽必须手改那份 JSON ⇒ 评审可见 ✓）；⑤ **禁垃圾**：`docs/**` 下不得有
`.tmpdir` / `.tmp` / `.tmp-<pid>` / `.DS_Store` / `*.orig` / `*.rej` / `*~`
（**含未跟踪的本地残留** ✗）；⑥ **归档可追溯**（红线 ✓）：`docs/archive/**`
每个文件必须在 `docs/archive/README.md` 里**被点名** ✓，归档总量 ≤2 MB ✓。

**红线**（不许动 ✓）：**判定正确性、红线证据与可追溯性** ✗ —— **归档 ≠ 销毁** ✓：
任何"证据/判据/复现件"必须**仍能找到** ✓（给归档路径与索引 ✓）；
**不许为了让数字变绿而删证据** ✗。**新环节的文档预算** ✓：设计先行**只写契约
不写过程** ✗（过程进 commit message 与 `STATUS.md` ✓）。

### 9.5 2026-09-26 专项：记法 + 隐式参数 + **产品交互面**（用户拍板 ✓）

**用户原话**：「**全面一点，专门针对 notation，产品交互**。包括**多来点隐式参数，让
notation 更直观**」+「**不许新建大 plan 文件**；用 `scripts/plan.py` 加**细项**」+
每条「**先判红 → 一处一 commit → 带反向验证**」，用户可见改动要**真宿主 e2e**。

#### 「看得见的变化」清单（每条都带判据与反向验证 ✓）

| 变化 | 用户看到什么 | 判据在哪一层 |
|---|---|---|
| **A1** Infoview 的 `⊢` | 目标行不再漏折记法 | 真宿主 e2e ✓ |
| **A2/A3** `singleton`/`{a}` | `{a}` 能写、F12 能跳 | 真宿主 e2e ✓ |
| **A4** prelude 可跳 | `Or`/`And`/`Iff`/`False` 的 F12 落到前奏**真源文件** | 真宿主 e2e ✓ |
| **A5** `flawed_equalities_refuted` | 函数体里的记法也折 | front 单测 ✓ |
| **Infoview 字号** | 声明类型/值/目标回到 **1em**、**去掉双重压暗**、行高 1.5 | CSS 契约 + 反向验证 ✓ |
| **编译进度 P1–P4/P6** | 状态栏「编译中…」· Infoview **3 行**进度区 · **概览尺**标出在编的文档 · 节流 `sokonanoda.progress.throttleMs` | LSP 成对 `$/progress` + 宿主 + webview 三层 ✓ |
| **B3 隐式插入** | 记法里的前导类型参数**不用再手写**（`a ∈ A`、`f '' A`） | front 单测 + 复现件 ✓ |

#### 硬要求（沿用 §9.2 的流程纪律 ✓，专项补充）

* **一个环节一个 commit**、**带反向验证**（撤掉修复必须判红 ✓）；
* **用户可见的改动必须有真宿主 e2e**（webview DOM 与状态栏 **够不到** ⇒ 钉"载荷到达" ✓，
  边界见 S2 调研 ✓）；
* **不许为了数字好看而放宽守卫** ✗ —— 记法路径守卫是**棘轮**（基线只降不升 ✓）；
* **缺口必须进台账**（`docs/gaps/ledger.jsonl` + 可执行复现件 ✓）。

### 9.4 历史索引（一行一条；**全文见归档** ✓）

* **归档**：`docs/archive/REQUIREMENTS-ARCHIVE.md` —— **3146 行逐字原文** ✓
  （移入前 §9 = **2989 行** ✗）+ **139 条索引** ✓；**纯文本、可 grep** ✓
  （它是"要求的账本"，**可检索性优先** ✓，不 gzip）。
* **跨度**：**2026-09-06 → 2026-09-26**（接手 → 内核/课程 → 编辑器/发布 →
  课程标准库 → 站点/记法化 → E1 → E2 的 50 环节与 0.66–0.72 发版 → 预算纪律）。
* **现状要求** = 本文 §1–§8 + §9.1/§9.2/§9.3 ✓；**其余一律是历史** ✗。

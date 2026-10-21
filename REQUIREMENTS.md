# 项目要求总账（REQUIREMENTS）

> 这是用户全部要求的**权威记录**。任何 agent 接手任何任务前先读本文，
> 再读 `ROADMAP.md`（里程碑）与 `STATUS.md`（当前进度）。新要求出现时追加到本文末尾并注明日期；冲突时以本文为准。

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
   证据与方案见 `docs/design/course-stdlib.md`（试做稿原文 ⇒ `git log --all -- docs/gaps/spike/README.md`），
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

- **当前欠账**（2026-09-30 实测）：`crates/lsp/src/lib.rs` **2333 行**、
  `crates/front/src/compile/check/walk.rs` **1775 行**、`check/mod.rs` **1549 行**、
  `crates/front/src/compile/tests.rs` **8739 行**（测试，暂豁免）—— 都远超 ~500 行红线
  （⚠ I16 那句"结构债已还清"只对拆分当天成立，之后又长回来了）；
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
- 第一课内容：①表达式与类型 ②函数与箭头 ③命题与证明项 ④等式与 rfl（`rfl` 也吃两边 defeq 的 `↔`：`a ∈ ∅ ↔ False` 直接 `rfl`；不 defeq 仍判红）；
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

- 已完成：I6 → I7 第一门课 → I8（余项：依赖精确化 / early-cutoff）→ I9 goal 视图 →
  I10 值位 `apply`（**0.22.0 随 I13 整体移除**）→ I12 官网 → I14/I15/I16 → E1（124/124）。
  **I11 真人输入测试体系已废弃**（`char_steps` 基建已删，现状 = `by` 引擎）。
- **当前执行 = `docs/ONBOARDING.md` §0.2 的唯一队列**（2026-10-01：`v0.79.0` = E19 默认开，
  P 组编译提速与批次 N 全档收口；**队列已清空**，下一条线由用户定）。
- 发布流程已自动化（ci auto-tag，见 `docs/RELEASE.md`）。
- 详细验收标准以 `ROADMAP.md` §10 为准。

## 9. 现行要求（历史追加日志 → `docs/archive/REQUIREMENTS-ARCHIVE.md` ✓）

> **为什么拆** ✗：本节曾是 **2989 行**（2123 行日期日志 + 609 行交付报告 + R/P 条目）—— 而"权威总账"要的是**现行要求** ✓，不是流水账 ✗（用户 2026-09-26 要求 ✓）。**归档 ≠ 销毁** ✓：**原文逐字**在 `docs/archive/REQUIREMENTS-ARCHIVE.md`（**纯文本、可 grep** ✓，带 **139 条索引** ✓）；**判据与守卫**仍在 `scripts/` 与 `crates/**/tests/` 里**真跑** ✓。判据：`python3 scripts/docs-lint.py` ✓（入口 ≤800 行 ✓）。

### 9.22 2026-10-15 **`#check <def>` 带定义体 + 记法渲染**（用户明确要求，**有意偏离 Lean 4** ✓）
**用户原话（2026-10-15 15:22）**：
> 「我想让 **`#check Set.subset`** 也支持针对 def 的优化，例如 `#check Set.subset` 显示
> `def Set.subset : ∀ {α : Type}, Set α → Set α → Prop`
> `:= fun {α : Type 0} (A B : Set α) => ∀ (x : α), x ∈ A → x ∈ B`」

**同日 15:2x 的授权**（用户第二次拍板）：「**`#check` 我是新加需求，偏离 lean4 没问题**」
⇒ 本条是**用户新加的需求** ＋ **有意偏离 Lean 4**（硬规则 3 的"子集"边界在此**由用户显式放开** ✗→✓）；
也**取代** `docs/design/notation-subset.md` 里那条"print-back 继续显示点名形式"的旧决定（那一行已改 ✓）。

**要求（两件缺一不可 ✗）**：① `#check <裸 def 名>` 要**带定义体**（`:= …`）；
② 类型与定义体都要**记法渲染**（`∀`/`→`/`∈`，**不是** `forall`/`->`/`Set.mem`）。

**边界（一条都不许含糊）**：
* **只给 `def`** ✗ —— `theorem`/`opaque`/`axiom`/`inductive`/构造子/递归子、以及一切
  **非裸标识符**的 `#check` 一律**只打签名**（逐字节回到今天 ✓）；
* **`#print` 一个字节都不动** ✗（它的形状是既有契约）；`#check` 的
  `inferred_type` / `text`（**判定路径**读的那两个字段）也**一个字节都不动** ✗ ——
  卡片只进 `CheckEvent::TypeChecked::decl` 这个**显示**字段
  （`judge::judge_type_of` ⇒ 记法目标签名读的是 `text` ✓）；
* 记法**只在记法表里有对应符号时**才用；没有的仍走点名形式（**不许猜** ✗）；
* 卡片**与 Infoview 的声明卡片逐字节同形**（类型那一半过 `lean4_universe_text`：
  `Type 0` → `Type`；定义体那一半不过）⇒ 命令行与编辑器**终于同一个形状** ✓。

**判据**：`crates/cli/tests/check_def_card.rs`（用户那条命令逐字节 ＋ 终端输出 ＋ 只给 def 的
反向验证）· `crates/front/src/compile/tests.rs` 两条单测 · `REPORT_SHAPE` 8 → 9
（`CheckInfo::text` 的**含义**变了 ⇒ 旧产物必须整库不命中 ✓）。

### 9.23 2026-10-15 **一条清理环境的命令 + `soko update` 的 404**（用户原话 ✓）
**用户原话（17:23）**：「帮我清理环境吧，**这种版本不统一出现好多好多次了**」（同轮实测 `scripts/soko update` 报 `fetch failed: HTTP 404`，而**同一个 URL 用 curl 是 200**）。**要求**：① 给**一条**命令把"版本不统一"的**三处**一次收口 —— 多余的 VS Code 扩展目录（`code --install-extension` 不删旧版、只写 `.obsolete`；实测 **8 个**）· 指向已删目录的 `.obsolete` 残留（**15 条**）· 两半不一致的缓存 `marker`（CLI `0.83.0` / LSP `0.73.0`）；**默认 dry-run，`--yes` 才真删**（删除不可逆 ✗）；`extensions.json` 说 VS Code **正在用**的那份**永不删**，别的发布者的目录与条目**一个字节不动**。② **根治**那半：404 必须修（它就是缓存两半漂移的**直接机制** —— 强制刷新把版本**对象**插进 URL ⇒ `v[object Object]`），失败时打**实际 URL** + 给"404 是资产不存在、不是网络"的出路（不许再把人引去查代理 ✗）。③ **不许靠抬文档上限达标**（`scripts/docs-budget.json` 一个字不动 ✓）。
**落地**：`scripts/soko prune`（**不叫 `clean`** —— 那会转发给 `sokonanoda clean` 的**编译缓存** ⇒ 撞名 ✗）· `doctor` 报版本散布（`--json` 的 `versions`，**只报告、不改 `ready`**）· `download()` 归一化并**拒绝非版本**（拼不出 URL 就报错）· `ensure` 传 `.text`。**判据**：`crates/cli/tests/launcher.rs` 的 `update_forces_a_refresh_at_the_pinned_version`（每次请求都带 `/v<pin>/`、恰好两次）与 `prune_is_a_dry_run_until_yes_and_unifies_the_three_sources`（dry-run 零删除 · `--check` exit 3 · `--yes` 后 8 → 1 个扩展 · 两 marker 一致 · sokonanoda 的 `.obsolete` 条目 0，别人的条目保留）。**读数**：`STATUS.md` · `.cache/ux/fix-w3/VERDICT.md`。

### 9.24 2026-10-16 **只有 `@` 收隐式参数 · 正常形式不收 · 报错要完善**（用户原话 ✓，**对齐 Lean 4** ✓）

**用户原话（08:5x · `SUB-implicit-args-align-lean4`）**：「**报错信息要完善，指令要明确**……真不如**对齐 lean**：**只有 `@` 形式接收隐式参数，正常形式不接受隐式参数**，并且**支持显式地指定参数名字**。」**落地**：报错完善（默认档 ✓ 零接受/拒绝变更 ✓）+ 引擎语义（`SOKO_STRICT_AT=1`，**默认关** ✗：整语料 **269/294 从接受变拒绝** · **0 反向** · **25 不变**，其中 **19 个在 `docs/gaps/repro/**`** ⇒ 子任务 `SUB-implicit-args-strict-default` · `SUB-implicit-named-args`）。**红线**：默认档 `check-json-identity.py` **251/251 逐字节相同** ✓。**对照与逐文件账在 commit message** ✓。

### 9.25 2026-10-16 **支持显式指定参数名字**（`g (α := Nat) 5`，用户 §9.24 原话的后半句 ✓，**对齐 Lean 4** ✓）
用户原话（与 §9.24 同一段，逐字）：「……并且**支持显式地指定参数名字**。」改前实测：`g (α := Nat) 5` **语法就不过**（`unexpected-token` · `expected ')' , found ':='` ✗ —— parser 的 `(` 只走 `parse_expr`，`:=` 无处可去）。**要求**：应用实参位上支持 `(形参名 := 值)`，按**名字**绑定（顺序无关），对齐 Lean 4 `Elab/App.lean:753` 的 `findBinderName?`（命名实参**优先于** binder 风格与 `@`）· `Parser/Term.lean:808`（`namedArgument` 只在应用实参位）· `:39 throwInvalidNamedArg`（名字对不上 ⇒ 一个码 + **列出可用形参名**）。**边界（如实 ✗）**：① 头必须是**已知常量**（局部假设按名字绑定**未做**）；② 与「富余实参落到结果类型」（路线 ③）**未组合** ⇒ 交回老路给诊断；③ Lean 的 `..`（ellipsis）与 code action **未做**。**默认档语义一个字未动** ✓（`SOKO_STRICT_AT` 仍默认关 —— 翻默认 = 队列 `SUB-implicit-args-strict-default`，多棒工程，见 §9.24 与 `STATUS.md`）。
### 9.26 2026-10-20 **严格档翻回 opt-in（默认回到旧读法）**（用户原话：「这个隐式参数和显式参数的实现方式太愚蠢了，几乎需要重构。要么就先用普通模式发布一版，保持高速度高效率」）
⇒ **默认档 = 旧读法**（正常形式可写隐式实参）· `SOKO_STRICT_AT=1` 才是严格档。**理由（实测，同一份课程单元 `unit01-sets-membership`）**：严格档默认时 **0.62 s → 8.8 s（14×）** —— 它把 `elab::operand_type_expr` 的问法换成**全显式 pp**，而那条支路**不查判定缓存**（`pp=explicit` 是另一个键、且该分支连查都不查）+ 就地也答不出（`elab_expr(operand)`）⇒ 每次退化成「造 `#check` 文档 + 整段前缀从零重编」（16 KB 的单元重编 **12 次**）；连带 `gates-course` 从 ~10 分钟涨到 30+ 分钟、`test (sokonanoda-lsp, lib)` 撞 30 s 超时判红。**严格档本身没删** ✓（`SOKO_STRICT_AT=1` 完整可用；语料已全部迁成**两档都合法**的 `@` 写法）⇒ 等「让就地/缓存对全显式 pp 也成立」那笔**性能重构**做完，再谈翻默认。
⇒ **v2 规划 = `docs/design/strict-mode-v2.md`**（2026-10-20 第二轮）：不再给那条支路提速，而是用 **provenance** 让「`@` 规则只约束**用户写的源码**」⇒ 全显式 pp 那条内部支路**整条消失**，strict 与默认走同一条热路径（「不影响编译」由构造保证）；验收一律**结构计数 + 两档逐字节对拍**，墙钟只做同 run 比值兜底。**同轮改口径**：课程正文迁移从「加 `@`」改成「**删多余的隐变量**」（用户原话：「从加 @ 改成删多余的隐变量」）—— `@Set.univ α` → `Set.univ`、`@And.intro a b ha hb` → `And.intro ha hb`；`@` 只在确实要显式指定某个隐式位时写 ✓。
### 9.27 2026-10-20 **严格隐式参数改成「可配置参数」**（用户原话：「**严格隐式参数**我想还是变成**可配置的参数**，project toml、vscode 插件等等地方可以配置。具体分段实现路径有整理出来吗」）
⇒ 唯一依据 = **`docs/design/dialect-config.md`**：**一个值（`CompileOptions.dialect`）、五处来源、一条优先级链**（CLI 旗标 > 环境变量 > 编辑器设置 > 文件指令 `-- sokonanoda:dialect` > `sokonanoda.toml` 的 `[dialect] strict_implicit` > 默认关）。**6 段实现路径**（S1 把值搬进 options · S2 **方言进缓存键** · S3 CLI 旗标 · S4 清单/文件指令 · S5 LSP `initializationOptions` + 热切换 · S6 VS Code 设置 + 文档/技能同步），每段一条可执行判据、可单独发版、可单独回退。⚠ **最关键的一条**：现在 `SOKO_STRICT_AT` 之所以不会串味，是因为 `compile/cache/keys.rs::flags_state()` 把所有 `SOKO_*` 折进了编译缓存键 —— 一旦方言变成 toml/编辑器设置（**不经过环境变量**），它就从键里掉出来 ⇒ **S2 必须先做**，否则两档共用 `.sokonanoda/` 会互相命中。LSP 侧要求**改设置即时生效**（`workspace/didChangeConfiguration`），不是重启。 **2026-10-20 已实现 S1–S6**（六笔 commit：方言进 `CompileOptions` → 进缓存键 → CLI `--dialect` → `[dialect]`/文件指令 → LSP 热切换 → VS Code 设置 `sokonanoda.dialect.strictImplicitArgs`），默认仍是旧读法。
### 9.28 2026-10-21 **课程里多余的 `@` 清掉（delete-first 收尾）**（用户原话：「set-theory 里可以去掉的 @ 没有去掉，比如 unit01 里的 `theorem demo_subset_def … exact @Set.subset_def α A B`，编译也通过，没有问题。请问帮我查查开发记录，为什么会有这样的问题」）⇒ 口径：`@` 只在确实要显式指定某个隐式位时写；凡「前导隐式实参可由后续显式实参唯一确定」的 `@Name <implicit> <explicit…>` 一律删成 `Name <explicit…>`，loose/strict 两档都要绿。本批：`@Set.subset_def` 46 处（含 `unit01-sets-membership` 的 demo 与 `lib/Demo`）+ `@Set.ext` 44 处 = **90 处 / 49 文件**；课程门禁 **258 目标 / 2190 checked / 915 open / failed 0** 与基线逐项相同；`notation-lint` 新增 `REDUNDANT_AT` + selftest 正反例。边界：`@Eq.*`（宇宙层级）、`@And/Or/Exists`（多个前导隐式）、`@Set.univ` 等不在本批，按 delete-first 逐类推进；**不做全局 `@` 删除**。残留根因：严格档翻默认时 `fix_diag.py` 按诊断机械插 `@`（`7f42456c`/`487f9a77`）；2026-10-20 改成「删多余隐变量」后，迁移集按「严格档还红」选（16 个 USER），`unit01` 的 `@` 版两档都绿 ⇒ 从未进入清理集；`notation-lint` 白名单又不含课标库引理 ⇒ 无红灯。
### 9.29 2026-10-21 **未完成的 `sorry` 练习按 Lean 4 进环境（goal state 不再被 unknown identifier 阻断）**（用户原话：「如果前面 mem_of_subset_singleton 或 subset_of_mem_singleton 没有证明，就是报错。这个太严格了，请对齐lean4，应该是能正常有goal states，有warning就行，不需要errors，阻断 goal states」）
⇒ 对齐 Lean 4：`sorry` 声明展开成 `sorryAx`、声明照样 `addDecl` ⇒ 后续声明**可以引用它**。本仓原先「open 声明不进环境」⇒ `unit01` 的 `singleton_subset_iff` 引用未解练习时直接 `elab-unknown-identifier`、整条声明的 goal state 消失 ✗。**改法（front 一处）**：开放练习的签名 elaborate 成功后，按**同签名 `Declar::Axiom`** 进环境并登记 `known`（学习者视角：这个名字有类型、值还没证）；判定/事件不变——该声明仍是 `exercise.open`，被引用时给 **`uses-open-exercise` warning**（不是 error），跨模块 `import` 行保留 `import-has-open-exercises` warning。**验收（实跑）**：`unit01` 的 `singleton_subset_iff` 用两个 `sorry` 引理 ⇒ `exercise.open ×2 + decl.checked + uses-open-exercise`、exit 0；部分证明（第二个方向留 `sorry`）⇒ `query state` 正常给出目标 `a ∈ A → {a} ⊆ A`；canvas 5 open / 3 checked、solution 6 checked 均 exit 0。**边界（如实）**：签名不过的声明仍 Failed、**不**进环境（G-01 红线不变）；引用 `Failed` 声明仍是 `elab-unknown-identifier`。
### 9.30 2026-10-21 **第二批单前导隐式 `@`**⇒ 清掉 **86 处 / 35 文件**：`@Set.mem_univ` · `@Set.notMem_empty` · `@Set.mem_powerset_iff` · `@Set.mem_singleton_self` · `@Set.mem_singleton_iff` · `@Set.mem_empty_iff_false` · `@Set.mem_inter_iff` · `@Set.mem_union` · `@mem_sep_iff`；`notation-lint` 的 `REDUNDANT_AT` 同步扩到这些名字 + selftest **18/18**。**边界**：`@Set.univ`/`@Set.empty`（零元）、记法目标（`POINTFUL` 已管）、多前导（`And/Or/Exists`/`prod_mk`）、`@Eq.*`（宇宙层级）**不进表**；`@Set.mem_univ α` 这类**只给前导位的局部应用保留 `@`**（删了会变成未应用定理）。判据：worktree 课程门禁 **258 目标 / 2190 checked / 915 open / failed 0**。
### 9.31 2026-10-21 **unit01 收口**（`·` / apply 早报 / 命名实参 / 括号 span / 失败 UI）
⇒ `·` 按 Lean `closeUsingOrAdmit`：聚焦子目标，块末未闭合一律 **admit** 成开放练习（有 `· sorry` 时洞落在 `sorry`，否则落在 `·`；块内 tactic 报错仍照常报错）。`apply` 的坏显式实参在 direct/fallback 都报在 `apply` 行（direct 的**部分应用普通 App** 仍靠内核终审，避免 `Eq.trans h` 未解隐式误判）；命名实参 `apply Eq.subst (p := …) h` 反解隐式；theorem/def/example 的 span 含最后一个已消费 token（`)`/声明后空行仍归声明）；失败声明不再显示 🎉。判据：front lib **1026/0**、tactic docs 6/0、skill 5/0、docs-lint ✓、课程单元1 **3 checked/5 open/0 判负**。
### 9.5 2026-09-26 专项：记法 + 隐式参数 + **产品交互面**（用户拍板 ✓）

**用户原话**：「**全面一点，专门针对 notation，产品交互**。包括**多来点隐式参数，让
notation 更直观**」+「**不许新建大 plan 文件**；用 `scripts/plan.py` 加**细项**」+
每条「**先判红 → 一处一 commit → 带反向验证**」，用户可见改动要**真宿主 e2e**。

#### 「看得见的变化」清单（每条都带判据与反向验证 ✓）⇒ 逐条索引（用户报的 6 条 + 隐式参数 + 断言名）：`docs/visible-changes.md`

| 变化 | 用户看到什么 | 判据在哪一层 |
|---|---|---|
| **A1** Infoview 的 `⊢` | 目标行不再漏折记法 | 真宿主 e2e ✓ |
| **A2/A3** `singleton`/`{a}` | `{a}` 能写、F12 能跳 | 真宿主 e2e ✓ |
| **A4** prelude 可跳 | `Or`/`And`/`Iff`/`False` 的 F12 落到前奏**真源文件** | 真宿主 e2e ✓ |
| **A5** `flawed_equalities_refuted` | 函数体里的记法也折 | front 单测 ✓ |
| **Infoview 字号** | 声明类型/值/目标回到 **1em**、**去掉双重压暗**、行高 1.5 | CSS 契约 + 反向验证 ✓ |
| **编译进度 P1–P4/P6** | 状态栏「编译中…」· Infoview **3 行**进度区 · **概览尺**标出在编的文档 · 节流 `sokonanoda.progress.throttleMs` | LSP 成对 `$/progress` + 宿主 + webview 三层 ✓ |
| **B2/B3 隐式插入**（T-N13/T-N14，**2026-09-30 收口**） | 记法里的前导类型参数**不用再手写**（`a ∈ A`、`f '' A`、`A ⊆ B`、`A ∩ B`、`(∅) x`）——课标库签名已隐式 | front 单测 **6 条**（各带反向验证 ✓）+ 课程门禁 **43/377/99/0** + 逐条索引 `docs/visible-changes.md`；⚠ 本行**只覆盖记法路径** ✗ —— **课程侧适配**（`courses/set-theory` **1584 处 `soko:notation-ok` 豁免** · 实测 A 962 / B 230 / D 235 / C 4 / E 153）是**独立条目** ⇒ **排期 + 判据 + 第 0 步普查**见 **`docs/ONBOARDING.md` §0.2 #5**（2026-10-02 补记：引擎侧收口后**它从未进过队列** ⇒ 多轮断线，机制已记在那条） |
| **E19 默认开**（2026-10-01 用户拍板，`v0.79.0`） | `∅ ≈ {b}` 这类「两侧都是零元糖」的写法**不再判红**（论域跟**同形的已解兄弟**，是**选出来的**）；逃生门 `SOKO_NOTATION_METAVAR=0`/`off` 回到旧行为 | 两态判据 `crates/cli/tests/{notation,implicit}_metavar.rs`（默认绿 / 逃生门红 / **不猜**）+ 台账 G-48 `fixed` ✓ |

#### 硬要求（沿用 §9.2 的流程纪律 ✓，专项补充）

* **一个环节一个 commit**、**带反向验证**（撤掉修复必须判红 ✓）；
* **用户可见的改动必须有真宿主 e2e**（webview DOM 与状态栏 **够不到** ⇒ 钉"载荷到达" ✓，
  边界见 S2 调研 ✓）；
* **不许为了数字好看而放宽守卫** ✗ —— 记法路径守卫是**棘轮**（基线只降不升 ✓）；
* **缺口必须进台账**（`docs/gaps/ledger.jsonl` + 可执行复现件 ✓）。

### 9.6 2026-10-01 专项：**两个大项目的最终形态**（用户拍板 ✓）

**用户原话**：「**两个大项目最终都要做到最终形态**（元参数引擎 + 集合论教材扩展）」；集合论线「**作为元参数 M0 之后的后续计划**」，前置障碍（notation、性能）**均已解除**（E19 默认开 · 全课 `build` **214s → 47.8s**）✓。**只排期、不写实现** ✓。

* **IA-4 元参数引擎**：设计 + 切片 M0–M4 + 决策点 D1–D7 ⇒ `docs/design/metavar-engine.md`（拍板：**D1 = 先只做 M0** · D4 保留 E19 defaulting · D6 不新增码 · D7 缩范围）✓；
* **集合论教材线**（**M0 之后**）：**S-A** 重建卷 I 大纲 `set-theory-syllabus.md` + 修**实测 5 处**悬空引用 · **S-B** 卷 I 深化（ZFC / 序数 / 基数算术 / 选择公理，单元仍须**全绿**）· **S-C** 卷 II 分析起步（实数构造 → 分析，先出设计 + scale gate）——入口/判据 ⇒ `metavar-engine.md` §7 + `ONBOARDING.md` §0.2；撞内核墙的条目**记台账**、**不改内核判定** ✓。

### 9.7 2026-10-09 专项：官网面向新人重构（用户拍板 ✓）

用户诊断：旧首页「是给懂行的人看卖点 + 给维护者看内部状态，新人看不懂」·「标题不是招牌」·「插件安装是一条 `code` 命令、不醒目」⇒ **要求**：首页改成新人 30 秒能看懂"这是干嘛的 + 怎么开始"（招牌 + 副标题 +【安装 VS Code 插件】【开始第一课】+ **真截图** · 3 条新人向特点 · 3 步开始 · 两门教材入口 · 一段与 Lean 4 的关系），**移除**"未来的计划"整节与 8 条内部状态式特点；另建更新日志子页（从 `CHANGELOG.md` 生成）——**2026-10-09 用户改口**：「changelog 不要单独子页面了，链接到 github 对应的地方吧」⇒ 子页与生成器已删，页脚直链 `editor/vscode/CHANGELOG.md`；头图必须是**真 VS Code + 已发布插件**的截图（不许 AI 生成/摆拍）且判据落在**像素**上。设计：`docs/design/site-single-page.md`。

### 9.8 2026-10-09 追加：官网中英文自动识别（用户拍板 ✓）

**要求**：默认中文；`navigator.languages` 含 `zh*` 保持中文，否则整页换英文（英文文案字典）；**无 toggle、无重定向、无 `?lang=`**；所有文案（hero / 两个按钮 / 三卡片 / 三步 / 教材 / 与 Lean 4 的关系 / 页脚）与 changelog 子页都要有英文版。做法与判据：`site/assets/i18n.js` + 每页 `<script type="application/json" id="i18n-en">` 字典（纯数据），`i18n` 项判"字典键与 DOM 逐个对齐"、`render` 项用 `--accept-lang` 判两种语言的真渲染 ✓。

### 9.9 2026-10-10 专项：编辑器 hover 统一收口 + 命令输出不重复（用户逐条实测 ✓）

用户逐条：① tactic 名字的类型「不是从统一渲染接口获取的，也没有正常的代码高亮」；② `#check Eq.refl` 的 hover「只有一个高亮的 `Eq.refl`，没有有效的类型信息」；③ `#print Set.singleton`「没有 hover 信息弹出」；④ `def` 的 hover 要「把类型和 `:=` 后面的含义块也显示出来……跟 infoview 的声明列表对齐」；⑤「每编辑一下 `#check`/`#print` 就多重复一次……编辑 `mem_of_subset_singleton` 会影响非它块的东西，完全不符合预期」。**要求**：hover 里凡 `.sokonanoda` 语言文本（tactic 名字的类型 · 记法符号/目标名/内建登记名签名 · 声明签名与 `def` 的值）一律走**同一个围栏块接口**（`code_block`+`CODE_LANG`；**行内码不许再渲染语言文本** ✗）；`def` hover = 类型块 + `:= <值>` 块（`theorem`/`axiom`/`inductive` 一个字节都不加）；光标在 `#check`/`#print` 行 ⇒ 显示**那条命令自己的输出**（真相层 `messages_at`，与 Infoview「命令输出」块同一份真相）；命令输出**每编辑一次 +1 是 bug** ⇒ 判据「产物命中 + 编辑一次后 `stateAt.messages` 仍恰好 1 条」+「同一报告拼接两次幂等」，**编辑的爆炸半径 = 那个声明自己**（用户既有设计）；内核 pp 在未解层元变量上 panic 曾让类型被吞成空串（② 的真根因）⇒ `leq_core` 按 Lean 4 补齐（`level.h:57` / `level.cpp:507`，**无 panic 路径**）。契约 `docs/protocol.md`；读数 `docs/visible-changes.md` + `STATUS.md`；台账 **G-102…G-105**。

### 9.10 2026-10-10 专项：**tactic 文档体系**（用户拍板 ✓）
用户原话：「`intro` / `exact` / `rfl` 这几个 tactic 没有好的文档，要求补齐三样：① 代码区域 hover 显示简短 tactic 说明；② 支持 F12 跳转到对应文档；③ 实现完整 tactic 介绍文档（markdown），含如何使用、适用场景、原理（背后内核规则/判定，如 `rfl` = defEq 自反、`exact` = defeq 统一）」；同日两次补充：**范围扩大到【所有】tactic**（一个都不能少）· **要有机制保证以后新增一个 tactic 也自动要求补文档** · **文档打包在 VS Code 插件里随插件自然附带分发**（像 `prelude.lean` 一样，装插件即自带、离线、无需额外拉取或访问外部站点）。**落地**：14 条的单一真相表 `crates/front/src/tactics.rs`（白名单由表派生 · 文档 `include_str!` 内嵌 · 独立 lint `scripts/tactic-docs-lint.py` ⇒ 加了 tactic 忘补文档过不了门禁）· 14 篇正文 + 索引 `reference/tactics/`（每篇的 `sokonanoda` 例子逐块喂真内核、零诊断）· hover 一行摘要 + F12 五档落点（**插件自带那一档**随 VSIX 打包成真文件 ⇒ 离线）· 设计 `docs/design/tactic-docs.md`（十决策 · 八阶段 P1–P7 · 26 风险）。**机械判据**：`unzip -l <vsix>` 的 15 个条目逐名核对 + `stage-docs.js --check` 防漂移 + 真宿主 e2e「F12 的落点在扩展目录下」。
### 9.11 2026-10-11 两处收口：声明列表去行号 + 名字普通粗体 · 内核包版本随产品（用户实测 + 拍板 ✓）

用户原话：①「infoview 里的『声明列表』里的行号可以不需要了，去掉吧。声明的名字，超链接的蓝色和下划线也都太抢镜了，直接改成普通粗体，不带其他样式了」；②（问「`Compiling sokonanoda v0.5.0 (crates/kernel)` 为什么不是 v0.88.0」后**拍板**）内核包版本**跟产品版本一致**。**要求**：声明卡**不再画 `L<n>`**（`decl.range` 仍只服务「点名字跳到定义」✗ 删的是显示、不是数据）· 名字是**普通粗体**（不借链接色 / 下划线 / 亮灰底；手型光标只留给真按钮 ⇒ 目标面板那行的 `<span>` 不再有假的可点暗示；`<button>` 与 click 语义**一个字不动**）· `crates/kernel` 的 `version` 改 `version.workspace = true`（`scripts/bump.py` 同步 `Cargo.lock` 里那条**精确名** ⇒ 手写 `0.5.0` 的旧形态作废；cargo 日志从此打产品版本）。
### 9.12 2026-10-11 两处收口：`set_option pp.explicit true`（看得见隐式实参）· `Iff.refl` 签名对齐 Lean 4（用户实测追问 + 拍板 ✓）

用户原话：①「`#print demo_mem_def` … `:= Iff.refl` … 结果不符合预期，你先帮我查查哪里的原因。这里 `Iff.refl` 都没有输入参数，怎么能够通过内核检查呢？请问是 `#print` 打印简略内容呢？还是内核被 heck 了」；②「lean4里也是这样吗？`Iff.refl` 是隐变量？另外我想让 `#print` 打印隐变量出来，需要加配置参数吗？还是你直接默认实现呢？」。**拍板 A**：加 **Lean 兼容**的 `set_option pp.explicit true`（含 `pp.all true`）—— `#print` / `#check <表达式>` / `#reduce` 显示隐式实参；**默认逐字节不变**；只两条名字（其余给专用诊断，**不静默**）；`… in …` 本轮不做；只影响命令输出（`ty_text`/hover 保持 G-71 可回读策略，如实登记为**有意偏离** Lean）。**拍板 B**：`Iff.refl` 的签名对齐 Lean core（`(A : Prop)` **显式**；隐式版兄弟 `Iff.rfl`），`by rfl` 与洞上灯泡候选改用 `Iff.rfl`，课程 5 处 + `notation-lint` + 文档/技能同轮同步（台账 **G-113**）。设计 `docs/design/set-option-pp.md`（判据 8 条，§5）。

### 9.13 2026-10-11 三处用户实测：`rfl` 对齐 Lean 4（defeq 目标）· 新 tactic `unfold` · 失败声明的签名 F12（**同日追加「不要留尾巴 全做好」**：三条边界全收口 ✓ —— `unfold Foo at h`（点名假设；`at *` 不做，lexer 没有 `*` token）· 目标显示不再把 `(Set.singleton a) a` 印成 `{a}`（修在渲染层）· `unfold` 实参名在成功声明里可 hover/F12（成功臂补词法行，用**源**表达式）· 递归 `def` 的展开引理 **n/a**（本语言没有递归 `def`）+ 防漂移判据 ✓。
用户原话：①「`theorem mem_of_subset_singleton … := by apply h; rfl` 的 rfl 报错，这个应该是 bug」；②「`(h : {a} ⊆ A)` 鼠标移动到花括号，但是 F12 没办法跳转到 `Set.singleton`」；③「`apply h` 到这一步的目标是 `⊢ a ∈ ({a})`，但是我想增加一个 tactic，展开成 `⊢ a = a`，调研查查 lean4 有没有类似功能的 tactic」。**落地**：① `rfl` 走**源级多轮展开逼近 whnf**（对齐 Lean 的 `MVarId.applyRfl` 先 `whnfR`；候选仍交内核终审 ⇒ 零假绿面；台账 **G-114**）；② **失败声明**的签名跳转行不再丢（G-108 第二刀：签名再 elaborate 一遍只为收行，不 `add_declar`；台账 **G-115**）；③ 新增 **`unfold Foo Bar`**（Lean 同名 tactic，`Init/Tactics.lean:802` 对照；非递归 def = deltaExpand + beta；**逐处**展开 + `.visit`；**内核验收后才提交**；子集边界：不做 `at h`），tactic 真相表 **14 → 15**（正文 `reference/tactics/unfold.md` + 索引 + `semantic::KEYWORDS` + 两条 lint 判据同轮跟上）。设计 `docs/design/unfold-tactic.md`。
### 9.14 2026-10-11 用户向命令文档 `reference/commands/`（用户：「`#print` 打印隐变量的功能实现了吗」→「没看到文档有写，我在哪里能看到文档」→「补上」）：新建索引 + `check` / `reduce` / `print` / `set_option` 四篇（五节体例照 `reference/tactics/`，每个 `sokonanoda` 围栏块例子逐块喂真内核判绿）+ 根 `README.md` 命令表补 `set_option` 与索引指针 + **新守卫** `crates/front/tests/command_docs.rs`（parser 白名单 ↔ 文档双向 · 最小片段真解析 · 索引逐字 · 每篇 ≤120 行 / 目录 ≤600 行 · 例子零诊断；三条反向验证实跑）。**边界**：插件内 F12/hover 落到命令文档**未做**（与 tactic 文档体系同形的第二刀，需设计先行）；`#env` / `#prove` / `#help` 是 REPL 交互命令，不在语言命令清单内。
### 9.15–9.16 2026-10-11 两条（命令文档随 VSIX 分发 + 命令关键字 `F12` 跳文档 + `#print`/`#check` 等四篇；prelude 的**证明类声明关键字：类型是 Prop 的证明类声明一律 `theorem`**）—— 逐字原文见下面各自的历史条目（`git log --all -- REQUIREMENTS.md` ✓）。
### 9.17 2026-10-12 初学者体验三条（用户 2026-10-11 原话：①「prelude 里 def 太多，完全可以换成 theorem……对用户以及初学者非常不友好」②「**我 vscode 的 vim 模式在做题目的时候感觉全乱了**」③「`Eq.subst α (fun (y : α) => y ∈ A) a x` 基本上自动推导全坏了……**如果没有能力自动补全，那还是尽量保持显式参数**」）⇒ **①** prelude 证明类声明一律 `theorem`（见 §9.16）· **②** vim 输入四修（A 光标 / B 单字符纯插入 / C `sokonanoda.input.leader` / D `editor.autoClosingBrackets: "never"`）· **③** 隐式实参第二候选 + `apply` 结论校验。**逐字原文 ⇒ `git log --all -- REQUIREMENTS.md`** ✓

### 9.20 2026-10-12 两条追加需求（用户原话：①「在克制的前提下，增加有限的发版节点……同时又要防止修发版问题而陷入无限的新的修 bug 的分支」②「主进程需要新增一个接口，支持外部的 agent 调整」）⇒ 两者长在**同一个队列**上：**① 发版节点**（`--release open/add/check/cut/block/drop`，同一问题第 4 次由脚本拒绝 ✗）· **② 外部提交面 inbox**（`--submit` 没证据不受理 ✗ · `--triage` 受理必须带 commit ✗）· **并行基建** `scripts/agent-worktree.sh` + 队列状态单点 `SOKO_QUEUE_ROOT` + `scripts/compile-cache.py`（实测回收 50.2 G ⇒ 29.7 G ✓）。**逐字原文 ⇒ `git log --all -- REQUIREMENTS.md`** ✓

### 9.18 2026-10-12 初学者体验**长尾五条**（用户看过缺口台账后拍板「好的」）⇒ ① LSP `A1` 洞带**位置模式**（tactic 位给 `exact hp`）· ② 归因与两通道（新阶段码 `entry-parse-failed`）· ③ 内核拒绝说人话（只在前端渲染层）· ④ 补全五条 · ⑤ M-07 缓存键折进 `builtin_prelude_digest()`。**逐字原文 ⇒ `git log --all -- REQUIREMENTS.md`** ✓

### 9.21 2026-10-13 **不许再登记豁免；旧妥协一律清掉**（用户原话：「不要登记豁免，旧的开发路径上的妥协，现在都不需要妥协了」）—— ① 不再新增任何豁免 ✗；② 旧妥协要**清掉**（**修引擎**让自然的 Lean 4 写法能用 ✓，不许为绕开缺口改课程设计 ✗）；③ 清扫顺序 = **先代码后标记**（删标记本身不解决问题 ✗）。
**同轮补充**（21:57）：「**啥预算，我没有给预算上限呀？能修的都要去**」+「**那应该把老的删了才对，怎么能让新的登录不进去呢**」⇒ 预算**不是用户设的上限**；**顶格的正解 = 归档旧的腾空间** ✓（仓库约定：压成 `docs/archive/*.gz` ✓），**不是**『因此不做必需的收尾』✗。
### 9.4 历史索引（一行一条；**全文见归档** ✓）
### 9.19 2026-10-12 台账机制（用户四连要求：①「所有台账能不能放到一个文件里？任务类型不一样而已，参考现代计算机结构」②「这个事件机制不能全依赖大模型运作，而是同时依靠脚本，比如 python 脚本当成一个事件调度器」③「每个台账启动一个 subagent 做，串行一次启动一个 subagent」→ **2026-10-12 改主意**：「允许最多 4 个 subagent，类比 4 核心 cpu」④「不要只排期，直接实现」）⇒ 唯一机器入口 = `python3 scripts/ux-queue.py --next`（或 `--json`）：**事件溯源调度器**（**并发上限 4 = "4 核"**，`SOKO_QUEUE_SLOTS` 可调；但**同一模块同一时间只许一个写者** ✗ —— `--claim` 按 `owner`→资源映射（`front → crates/front/**`…，可用 `--touch` 显式声明）做**资源互斥**，槽位满或撞资源**当场判红**）—— 状态 = `docs/ux-queue-events.jsonl` 里**只追加**事件的折叠（`--claim <id>` **原子认领**并写锁（落在共享 git 目录），别人持活锁判红 ✗、死进程/超 TTL 才准 `--force`；`--advance <id> --to done --commit <sha> --reading '<读数>'` —— **没有证据不许收口** ✗；`--add-child <父> --child <新id>` **就地拆解**（子任务插在父**之前**、父转 `umbrella`、子任务没关完不许收口）；非法迁移由代码里的 `TRANSITIONS` 表拒 ✗ —— 规则不靠提示词）· **处置只有两种**：**实现** 或 `wo-filed`（带理由）—— **"排期 / 以后做"不是处置** ✗（大工程可跨轮，但每轮必须交**已合入的读数**）· `docs/ONBOARDING.md` §0.2 = **脚本渲染的摘要块**（计数与队列**同数**，手改判红 ✗）+ 粗粒度的线（首列 = 真 id ⇒ 双向对账）· 门禁：`scripts/soko gate --fast`（秒级）与 CI 的 `ledger` job 都跑 `--check`（覆盖自检 + 调度器不变量）⇒ 与来源脱钩**当场判红** ✗。**物理合成一个文件本笔未做**（12 个真实 reader/writer + CI job ⇒ 跨模块重构，按仓库纪律独立一批）。
* **归档 + 跨度**：`docs/archive/REQUIREMENTS-ARCHIVE.md` —— **3146 行逐字原文** ✓（移入前 §9 = **2989 行** ✗）+ **139 条索引** ✓；**纯文本、可 grep** ✓（它是"要求的账本"，**可检索性优先** ✓，不 gzip）；跨度 **2026-09-06 → 2026-09-26**（接手 → 内核/课程 → 编辑器/发布 → 课程标准库 → 站点/记法化 → E1 → E2 的 50 环节与 0.66–0.72 发版 → 预算纪律）。 ｜ **现状要求** = 本文 §1–§8 + §9.1/§9.2/§9.3 ✓；**其余一律是历史** ✗。

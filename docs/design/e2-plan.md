# E2 计划：以"每一阶段都能发一个完整可用的版本"为硬约束

> **⚡ 状态（2026-09-30）：E2 阶段 A–E 收口；批次 N 25/28**（`python3 scripts/plan.py` = **63/66**；
> 权威 = `python3 scripts/plan.py next`）。**原始状态行（2026-09-26）：E2 = 50/50 全部收口** ✓ —— 7 个发版点全部发出
> （0.66.0 → **0.72.0** ✓，`gh release list` 已核对 ✓）。
>
> **本文件是"现行契约 + 收口索引"** ✓（用户 2026-09-26 要求文档瘦身 ✗）：
> §0 三条硬约束 · §1 阶段总览 · §3 刹车点 · §4 与 E1 的关系 —— **仍然生效** ✓；
> §13 清单 = **一行一条**（全部 `[x]` ✓）；§14 = `plan.py` 取规格用的索引 ✓。
>
> **逐字原文**（每个环节的完整规格 / 判据 / 实测数字 / 交付记录 ✓）：
> `docs/archive/e2-plan-full-2026-09-26.md.gz`（**3105 行**；`gunzip -c … | less` ✓）。
> **归档 ≠ 销毁** ✓：判据仍在 `scripts/plan.py check` 与各环节的测试里**真跑** ✓。
>
> **接手先读 `docs/ONBOARDING.md`** ✓（开工单：状态 / 唯一队列 / 判据 / 纪律）：
> 陷阱 / 验收命令 ✓）。**下一份计划**用 `SOKO_PLAN=<新计划>` 或改 `scripts/plan.py`
> 的默认路径 ✓。

> **用户定的三条硬约束（2026-09-24）**：
> 1. **每个阶段都能推送一个完整可用的东西** ✓ —— 不允许"半成品停在树上" ✗；
> 2. **性能只升不降** ✓ —— 任何阶段若让某条路径变慢 ✗，必须**关在默认关闭的开关后** ✗；
> 3. **尽可能多步骤** ✓ —— 一步一验收，好把控进度 ✓。

---

## 0. 三条硬约束怎么落地（每条都可机械检查）

| 约束 | 落地方式 | 检查方式 |
|---|---|---|
| **每阶段可发布** ✓ | **阶段内可以有很多 commit** ✓（每个环节一个 ✓，方便回溯与验收 ✓）；**阶段收尾**才 = `scripts/soko gate` 全绿 + 课程门禁计数逐项不变 + **一次 push**（批次制 ✓）+ bump + release + 核对 | 阶段定义里写死"发版判据"一栏 ✓ |
| **性能只升不降** ✓ | ① 有收益的改动才**默认开** ✓；② 中性/前置改动**默认关**（开关 ✓）或**零调用**（惰性 API ✓）；③ 每阶段量一次 `docs/perf/ledger.jsonl` | 冷开 `unit12-solution`、`build courses/set-theory`、keystroke 三个基准数字 ✓ |
| **多步骤** ✓ | 每阶段 3–8 个环节，每环节**独立判据** | `- [ ]` 清单 + `plan.py` 可勾 ✓ |

**提交粒度（2026-09-24 用户明确）** ✓：
* **一个环节 = 一个 commit** ✓（信息里写清 `T-XX` + 做了什么 + 判据是什么 ✓）；
* **一个阶段 = 多个 commit** ✓ —— **不要求**阶段内每步都 push ✗；
* **阶段收尾**才 push 一次 ✓（跑一轮 CI ✓）⇒ bump → release → `gh release list` 核对 ✓；
* 例外：**诊断性 CI**（本地复现不了、怀疑平台差异 ✓）可以单独跑，但必须写进 `STATUS.md` ✓。

**三个基准数字（每阶段都要复量，只许变好 ✓）**：
```bash
# ① 大文件冷开（judge 密集）
SOKO_JUDGE_STATS=1 sokonanoda --json courses/set-theory/units/solutions/unit12-solution.sokonanoda
# ② 目录构建（闭包重复编译）
sokonanoda build courses/set-theory            # 基线 146.07s / 35 文件
# ③ 编辑中热编译（生命线）
SOKO_PERF_COURSE_SLOW=1 cargo test -p sokonanoda-lsp --lib perf_course -- --nocapture
```

---

## 1. 阶段总览（每行 = 一个可发布版本）

| 阶段 | 交付物（用户能感知什么） | 发版 | 性能预期 |
|---|---|---|---|
| **A 显示链路修复** | Infoview 的 `def` 卡片有 `:=` 值行 ✓；`unit12-synthesis` 的 goal 完全记法化 + 高亮 ✓ | 0.66.0（minor：用户可见修复） | 中性 |
| **B 命令与产物标准化** | VS Code 命令名统一成 `Sokonanoda: <Command> (说明)` ✓；模块根下有 `.sokonanoda/` ✓，vscode 与 code agent 共用产物 ✓ | 0.67.0 | **变好**：首次之后不再重复算 ✓ |
| **C 模块级批量编译** | `build <dir>` 从"每文件一份闭包"变成"每模块一次" ✓ | 0.68.0 | **大幅变好**：146s → 模块数量级 ✓ |
| **D 前缀复用（重型重构，5 小步）** | 大文件 `by` 密集解答的判定不再重编译整份前缀 ✓ | 0.69.0 → 0.70.0 → 0.71.0 | **每小步都更好**：12.4s → ~3s ✓ |
| **E 收尾与固化** | 文档/台账/技能同步；性能回归进 CI ✓ | 0.72.0 | 持平（防退化） |

**顺序理由** ✓：A/B 是**用户直接可感知**的修复与体验 ✓，先做 ✓；C 是**模块级批量编译**的地基 ✓，同时是 R-3 的实现 ✓；D 依赖 C 的"模块级"视野（D 的"前缀"在模块级才完整 ✓）；E 固化 ✓。

---

## 13. 执行清单
### 阶段 A：显示链路修复（R-1 / R-2）
#### 批次 A：显示链路修复（R-1 / R-2）
- [x] `T-A1` **R-1 根因已确认**（E1 第 98 轮 ✓）：`crates/lsp/src/query_map.rs:74` 的 `decl_info()` 漏映射 `value`/`value_runs` ⇒ `GoalDeclInfo` 里没有这两个字段 ⇒ 扩展恒拿 `undefined`（**前端算好、CLI 给了、LSP 没转发**的三段式断链）
- [x] `T-A2` **R-1 修**：`GoalDeclInfo` 加 `value: Option<String>` + `value_runs: Vec<RunInfo>`；`decl_info` 里映射（与 `ty_runs` 走**同一个** `run_info` ✓）
- [x] `T-A3` **R-1 判据**：LSP 单测断言 `soko/goals` 的 `Set.mem` 条目带 `value`/`value_runs` ✓ + **防分叉守卫**（`ty_runs` 与 `value_runs` 同实现 ✓）+ e2e 断言 Infoview 卡片出现 `:=` 行 ✓
- [x] `T-A4` **R-2 复现判红**：先写复现件，证明 `unit12-synthesis` 的 `flawed_equalities_refuted` / `project_chain` 的 goal 文本**未完全记法化**、且 Infoview 目标**不高亮**（照 R-1 的"三段式断链"逐段验：前端 → LSP → 扩展）
- [x] `T-A5` **R-2 修（真因已定位到行 ✓）**：`crates/front/src/semantic.rs:308` 把
- [x] `T-A6` **R-2 判据**：复现件转绿 + LSP/e2e 各一条断言 + 课程门禁计数逐项不变 ✓
- [x] `T-A7` **阶段 A 收尾**：三个基准数字复量（应持平 ✓）→ `scripts/soko gate` 全绿 → **一次 push** → CI 绿 → bump `0.66.0` → release → `gh release list` 核对 ✓
  - ⬆ **BUMP**：`minor` —— Infoview 的 def 值行与 goal 记法化/高亮修复（用户可见）
### 阶段 B：命令与产物标准化（R-4 / R-3 前半）
#### 批次 B：命令与产物标准化（R-4 / R-3 前半）
- [x] `T-B1` **R-4 现状盘点**：列出 `editor/vscode/package.json` 里**全部** `sokonanoda.*` 命令的当前标题（做一张对照表 ✓）
- [x] `T-B2` **R-4 改标题**：统一 `Sokonanoda: <Command> (说明)` ✓（前缀固定、命令词首字母大写、括号内中文说明 ✓），含 `Infoview (目标面板)`、`Restart Server (重启服务器)` 等
- [x] `T-B3` **R-4 同步四份**（AGENTS.md 硬规则）：`editor/vscode/` 的 README/CHANGELOG ✓ + `skills/` 三个技能 ✓ + `AGENTS.md` ✓ + `docs/vscode-dev-guide.md` ✓；`crates/cli/tests/skill.rs` / `dsh.rs` 不许漂移 ✓
- [x] `T-B4` **R-3 设计**：`.sokonanoda/` 目录的**契约**（放什么：编译产物 / 依赖 / 元数据；命名；清理策略；`--clean` 语义；与现有缓存 `~/.local/share/sokonanoda` 的关系 —— **模块根下的产物 vs 全局缓存**的分工 ✓）
- [x] `T-B5` **R-3 实现（CLI 侧）**：`build`/`grade` 把模块级产物写进 `<模块根>/.sokonanoda/` ✓；第二次调用**命中** ✓
- [x] `T-B6` **R-3 判据**：同一模块连跑两次 `build`，第二次**显著更快** ✓（数字进台账）+ 产物目录内容可读（`--json` 能列 ✓）+ `.gitignore` 指引 ✓
- [x] `T-B7` **阶段 B 收尾**：基准复量（① ② 应变好 ✓）→ gate 全绿 → 一次 push → CI 绿 → bump `0.67.0` → release → 核对 ✓
  - ⬆ **BUMP**：`minor` —— 命令名标准化 + `.sokonanoda/` 产物目录（vscode 与 agent 共用，首次之后不再重复算）
### 阶段 C：模块级批量编译（T-K30 的正解）
#### 批次 C：模块级批量编译（T-K30 的正解）
- [x] `T-C1` **设计**：`plan_module(root)` 的契约 —— 把模块根下**全部**文件作为**一个 unit 集**编一次 ✓、按文件给出报告 ✓、`ok(file)` 的语义与今天"以它为入口编一次"**逐项等价**（或明确记录差异 ✓）
- [x] `T-C2` **实现 API**：新增规划入口（底层 `units: &[SourceUnit]` 本来就支持多单元 ✓）
- [x] `T-C3` **等价性判据**：对同一门课，**逐文件**比较"新 API 的报告"与"旧路径（逐入口编译）的报告" —— 状态、诊断、`decl.checked` 事件**逐项相同** ✓
- [x] `T-C4` **`build <dir>` 接入**（开关 `SOKO_BUILD_MODULE_BATCH=1` 默认**关** ✓，先保证零退化 ✓）
- [x] `T-C5` **量收益**：`build courses/set-theory` 从 **146.07s** 降下来 ✓（数字进台账）；**默认打开** ✓（收益已证 ✓）
- [x] `T-C6` ~~**LSP/agent 接入**：从 `.sokonanoda/` 取模块级产物~~ ⇒ **按实测重新界定（2026-09-25）**：
- [x] `T-C7` **阶段 C 收尾**：基准复量（② 应大幅变好 ✓）→ gate 全绿 → 一次 push → CI 绿 → bump `0.68.0` → release → 核对 ✓
  - ⬆ **BUMP**：`minor` —— `build <dir>` 按模块只编一次（146s → 模块数量级）
### 阶段 D：前缀复用（重型重构；**5 个小步，每步都可发布**）
#### 批次 D：前缀复用（重型重构；**5 个小步，每步都可发布**）
- [x] `T-D1` **把 check-then-add 收进单一函数**（纯重构，**零行为变化** ✓）：`kernel_phase` 里逐条"检查→加入"的逻辑抽成一个可复用单元 ✓
- [x] `T-D2` **D1 判据**：全语料两态 `--json` **逐字节相同** ✓ + 课程计数逐项不变 ✓ + 五步全绿 ✓（**不变量**：这一步不改任何行为 ✓）
- [x] `T-D3` **walk 增量检查（开关默认关）**：walk 边 elaborate 边 `with_env` 检查并 `add_declar` ✓；`SOKO_SHADOW_CHECK=1` 才启用 ✓ ⇒ 默认路径**零变化零成本** ✓
- [x] `T-D4` **D3 判据**：开关两态 `--json` 逐字节相同 ✓ + 课程计数不变 ✓ + 事件计数不变 ✓（**开关开**时也相同 ✓ ⇒ 证明"两遍检查"语义等价 ✓）
- [x] `T-D5` **judge 接快照（开关默认关）**：`run_by`/`judge_infer` 拿 `Option<&EnvBuilder>` ⇒ `snapshot()` 查合成声明 ✓；拿不到**回退**旧路径 ✓；`SOKO_JUDGE_ENV_REUSE=1` 才启用 ✓
- [x] `T-D6` **量收益 + 默认打开**：`SOKO_JUDGE_STATS` 看 `JUDGE_INFER` 的 miss 成本是否塌下来 ✓；**收益成立才默认打开** ✓（否则保持关闭并记录 ✗）
- [x] `T-D7` **阶段 D-1 收尾**：基准 ① 复量（应大幅变好 ✓）→ gate + 四件套 → 一次 push → CI 绿 → bump **`0.69.0`（minor）** → release → 核对 ✓
  - ⬆ **BUMP**：`minor` —— judge 前缀复用第一刀：大文件 by 密集解答不再重编译整份前缀
- [x] `T-D8` **去掉重复检查**（第二刀）：`kernel_phase` 不再重查 walk 已核的声明 ✓（**只删重复** ✓，语义由 D4 的对拍保证 ✓）
- [x] `T-D9` **D8 判据 + 量收益**：四件套全过 ✓ + 基准 ① 再降 ✓（数字进台账 ✓）
- [x] `T-D10` **阶段 D-2 收尾**：bump **`0.70.0`** → release → 核对 ✓
  - ⬆ **BUMP**：`minor` —— 去掉重复检查（第二刀）
- [x] `T-D11` **跨会话复用**（第三刀）：把"已验证的前缀环境"按 `.sokonanoda/` 持久化 ✓ ⇒ **编辑中热编译**再降 ✓（生命线 ✓）
- [x] `T-D12` **D11 判据**：keystroke 基准复量 ✓ + 四件套 ✓
- [x] `T-D13` **阶段 D-3 收尾**：bump **`0.71.0`** → release → 核对 ✓
  - ⬆ **BUMP**：`minor` —— 跨会话复用已验证前缀（第三刀，编辑中热编译）
### 阶段 E：收尾与固化
#### 批次 E：收尾与固化
- [x] `T-E1` **性能回归进 CI**：把三个基准做成 CI 可跑的 smoke（阈值宽松 ✓，只抓**大幅退化** ✗）
- [x] `T-E2` **文档收口**：`architecture.md` §6 台账 / `docs/PERF.md` / 技能与 `AGENTS.md` 同步 ✓
- [x] `T-E3` **`STATUS.md` 与 `docs/ONBOARDING.md` 更新** ✓
- [x] `T-E4` **阶段 E 收尾**：bump **`0.72.0`** → release → 核对 ✓
  - ⬆ **BUMP**：`minor` —— 性能回归进 CI + 文档/技能收口
#### 批次 N：记法 × 隐式参数 × 产品交互（2026-09-26 用户专项）
- [x] `T-N1` **A1 修**：`fold` 折 `->` → `→`（`Expr::Arrow` + 匿名 binder `Forall`）+ **副发现**：`print_back` 的 `base` 反推对"整条表达式带括号"是错的 ⇒ 改用 `proof::CHECK_PREFIX`
- [x] `T-N2` **A2 修**：`Set.singleton α a` → `{a}` / `Set.pair α a b` → `{a, b}`（`fold_spine` 里照 `forall` 先例单独认内建语法）
- [x] `T-N3` **A0 取证结论**：68 条基线逐条判过（迁移 9 / 立判据 19 / 台账不做 40，结论进 `docs/design/duplication-audit.md`）+ 修正设计文档 `notation-display.md`（`render_text`/`Rendered` 与 as-built 不符）
- [x] `T-N4` **A0 迁移**：`query::runs` 改走唯一接口 + `by.rs` 8 处用户可见诊断消息改 `fold_for_display` ⇒ `--rebless` 削基线
- [x] `T-N5` **A1/A2 真宿主 e2e**：`⊢` 后文本含 `→`、`{a}` 显示为 `{a}`、`runs` 拼接 == `text`
- [x] `T-N6` **A3**：`{a}` 的 goto-definition → `Set.singleton`（front hover 节点 + LSP definition）+ e2e
- [x] `T-N7` **A4 取证 + 路线**：断链点确认 = prelude 名字的 hover 行 `resolution: None`（受信任安装、闭包里没有位置）；**路线定为真文件**（Lean 的 `Init/Prelude.lean` 同款）而不是虚拟文档 —— 本仓库只有 `file:` 一种 URI 形态 ✓
- [x] `T-N8` **A4 实现 + 判据**：`prelude_source` / `prelude_def_span` / `prelude_source_path` + F12 兜底（**必须在 `definition_at` 之后**）+ 三层判据 + 反向验证；顺带抓到并修掉「抢走被 import 模块自定义的 `Or`」这条真回归 ✓
- [x] `T-N9` **B0 取证** ✓：IA-1 as-built 三条用法实测全通（记法 `a ∈ A` · 点名短写 `Set.mem a A` · `@` 全显式），且**改签名向后兼容**（老的点名带类型实参写法照样过 ✓）；课程侧普查 **585** 处点名（`univ` 189 · `empty` 103 · `singleton` 84 · `mem` 64 · `image` 46 · …）；**⛔ 结论：B2 被 B3 挡住** —— 把库的前导类型参数改成 `{α : Type}` 会让课程门禁从 `328 checked · 0 判负` 掉到 `249 checked · 81 open · 7 判负` ✗（最小复现 + 完整证据：缺口 **G-40**）
- [x] `T-N10` **B1 判红**：R5 最小复现（`''`/`⁻¹'` 的 λ 操作数解不出前导类型参数）
- [x] `T-N11` **B1 修**：扩宽 `solve_prefix`（期望类型参与 + 逐层 deferral）
- [x] `T-N12` **B1 三件套判据**：② 反向 ✓（撤兜底 ⇒ 判据红）· ③ 红线 ✓（36/328/99/0 逐项不变）· ① 正向**部分达成**：`flawed_equalities_refuted` 从整条点名（5 标记）改成除 **λ 体内的 `∅`** 外全记法（3 标记）—— 该残例连 Lean 都要 `(e : T)` 标注，属语言级缺口
- [x] `T-N13` **B2**：课程库改隐式风格（`Set.image`/`Set.preimage` 一族 + `Function.comp`/`Rel.comp`）+ 调用点数量级下降。**三个前置已收口** ✓（G-43 第 515 轮 · G-69 第 516 轮 · G-70 第 517 轮）；**签名隐式化本身干净** ✓（四个 lib 各 0 诊断）；**批量改写证伪** ✗（脚本一次 649 处 ⇒ 16 文件判红）⇒ 剩 **10 文件 / 19 诊断**的调用点迁移，**逐站点核**（失败分类见 §阶段 N 的 T-N13 as-built）
- [x] `T-N14` **B3**（**先行**：B2 的前置）：**记法路径改走唯一钩子**（隐式档，`elab_notation_implicit`）+ **唯一钩子的实参期望类型**（先解前缀、再逐位给期望类型）⇒ **模式 B**（`Set.univ ∩ A` ⇒ `α := Type 0`）与 **模式 D**（`Set.image f A y` 一族）**都已收口** ✓；补参 hack **收窄**到 `implicit_prefix == 0` 那一档（护城河：`=` 等内建记法，隐式目标上执行次数 0）✓；全语料对拍 **644 组逐字节相同** ✓ · front **794/0** · 记法契约 **50/0** · 两条新判据带反向验证 ✓。**剩 G-43**（构造子 + lambda 实参）⇒ 归 T-N13
- [x] `T-N16` **A0 立判据收尾**：`SubGoal.ty` 那 9 处**两半分开钉** —— 显示副本（wire 克隆 + LSP hover）**必须折** + 真相字段（`DeclState.sub_goals[].ty`，`suggest.rs` 回读它算建议）**一个字节都不许折**；两条判据各带反向验证 ✓；**「59 是地板」的结论**进审计 §1.5（再降要改记账口径，不是再迁几处 ✓）
- [ ] `T-N15` **C 收尾**：台账 + 「看得见的变化」清单 + `REQUIREMENTS.md` §9（2026-09-26）+ VS Code/skills 同步
## 3. 风险与刹车点（每阶段都有一条"停下"的判据）

| 阶段 | 刹车点 | 停下后做什么 |
|---|---|---|
| A | R-2 的复现件做不出**判红** ✗ | 说明"症状"描述需要更具体 ⇒ 请用户补一个最小例子 ✓ |
| B | `.sokonanoda/` 与全局缓存的**分工**说不清 ✗ | 只做 R-4（命令名 ✓），R-3 单独出设计再议 ✓ |
| C | **等价性判据不过** ✗（T-C3） | **不接 `build`** ✗，把 API 与判据留在树上（惰性 ✓ 零成本 ✓）并记录差异 ✓ |
| D | **四件套任一不过** ✗ | 该小步**默认关** ✓（不影响已发布版本 ✓）并记录；D 可停在任一小步 ✓ |
| E | — | — |

**D 的额外护栏** ✓：D3/D5/D8 全部**先开关后默认** ✓ ⇒ 任何一步出问题，**已发布的版本都不受影响** ✓，且**性能不会倒退** ✓（默认路径零变化 ✓）。

---

## 4. 与 E1 的关系（哪些结论被继承）

| E1 的结论 | E2 怎么用 |
|---|---|
| T-K11（K1-a）**实测零收益** ✗ | 代码保留为**惰性开关** ✓；D5 复用它那条链 ✓ |
| T-K12c / T-K13 接线**存档** ⏸（`decl_idx` 墙 ✓） | **D 就是那次重构** ✓，从 D1 起重新分步 ✓ |
| T-K30 **重新定级** ✗（需新 API ✓） | **阶段 C** ✓（先设计契约、再做等价性判据 ✓） |
| T-K31 **实测无收益** ✗ | **不做** ✓（记账即可 ✓） |
| 三个内核原语（`with_env` / `snapshot` / `Clone` ✓）+ 对照器 | D 的**现成积木** ✓ |
| 四份实测账 | 每阶段的**基准** ✓（只许变好 ✓） |

---

##### T-A1 **R-1 根因已确认**（E1 第 98 轮 ✓）：`crates/lsp/src/query_map.rs:74` 的 `decl_info()` 漏映射 `value`/`value_runs` ⇒ `GoalDeclInfo` 里没有这两个字段 ⇒ 扩展恒拿 `undefined`（**前端算好、CLI 给了、LSP 没转发**的三段式断链）

已收口 ✓（E2 50/50）。**逐字规格与判据见归档** `docs/archive/e2-plan-full-2026-09-26.md.gz` §13。

##### T-A2 **R-1 修**：`GoalDeclInfo` 加 `value: Option<String>` + `value_runs: Vec<RunInfo>`；`decl_info` 里映射（与 `ty_runs` 走**同一个** `run_info` ✓）

已收口 ✓（E2 50/50）。**逐字规格与判据见归档** `docs/archive/e2-plan-full-2026-09-26.md.gz` §13。

##### T-A3 **R-1 判据**：LSP 单测断言 `soko/goals` 的 `Set.mem` 条目带 `value`/`value_runs` ✓ + **防分叉守卫**（`ty_runs` 与 `value_runs` 同实现 ✓）+ e2e 断言 Infoview 卡片出现 `:=` 行 ✓

已收口 ✓（E2 50/50）。**逐字规格与判据见归档** `docs/archive/e2-plan-full-2026-09-26.md.gz` §13。

##### T-A4 **R-2 复现判红**：先写复现件，证明 `unit12-synthesis` 的 `flawed_equalities_refuted` / `project_chain` 的 goal 文本**未完全记法化**、且 Infoview 目标**不高亮**（照 R-1 的"三段式断链"逐段验：前端 → LSP → 扩展）

已收口 ✓（E2 50/50）。**逐字规格与判据见归档** `docs/archive/e2-plan-full-2026-09-26.md.gz` §13。

##### T-A5 **R-2 修**：按复现件定位的那一段修（**先查明再改** ✗）

已收口 ✓（E2 50/50）。**逐字规格与判据见归档** `docs/archive/e2-plan-full-2026-09-26.md.gz` §13。

##### T-A6 **R-2 判据**：复现件转绿 + LSP/e2e 各一条断言 + 课程门禁计数逐项不变 ✓

已收口 ✓（E2 50/50）。**逐字规格与判据见归档** `docs/archive/e2-plan-full-2026-09-26.md.gz` §13。

##### T-A7 **阶段 A 收尾**：三个基准数字复量（应持平 ✓）→ `scripts/soko gate` 全绿 → **一次 push** → CI 绿 → bump `0.66.0` → release → `gh release list` 核对 ✓

已收口 ✓（E2 50/50）。**逐字规格与判据见归档** `docs/archive/e2-plan-full-2026-09-26.md.gz` §13。

##### T-B1 **R-4 现状盘点**：列出 `editor/vscode/package.json` 里**全部** `sokonanoda.*` 命令的当前标题（做一张对照表 ✓）

已收口 ✓（E2 50/50）。**逐字规格与判据见归档** `docs/archive/e2-plan-full-2026-09-26.md.gz` §13。

##### T-B2 **R-4 改标题**：统一 `Sokonanoda: <Command> (说明)` ✓（前缀固定、命令词首字母大写、括号内中文说明 ✓），含 `Infoview (目标面板)`、`Restart Server (重启服务器)` 等

已收口 ✓（E2 50/50）。**逐字规格与判据见归档** `docs/archive/e2-plan-full-2026-09-26.md.gz` §13。

##### T-B3 **R-4 同步四份**（AGENTS.md 硬规则）：`editor/vscode/` 的 README/CHANGELOG ✓ + `skills/` 三个技能 ✓ + `AGENTS.md` ✓ + `docs/vscode-dev-guide.md` ✓；`crates/cli/tests/skill.rs` / `dsh.rs` 不许漂移 ✓

已收口 ✓（E2 50/50）。**逐字规格与判据见归档** `docs/archive/e2-plan-full-2026-09-26.md.gz` §13。

##### T-B4 **R-3 设计**：`.sokonanoda/` 目录的**契约**（放什么：编译产物 / 依赖 / 元数据；命名；清理策略；`--clean` 语义；与现有缓存 `~/.local/share/sokonanoda` 的关系 —— **模块根下的产物 vs 全局缓存**的分工 ✓）

已收口 ✓（E2 50/50）。**逐字规格与判据见归档** `docs/archive/e2-plan-full-2026-09-26.md.gz` §13。

##### T-B5 **R-3 实现（CLI 侧）**：`build`/`grade` 把模块级产物写进 `<模块根>/.sokonanoda/` ✓；第二次调用**命中** ✓

已收口 ✓（E2 50/50）。**逐字规格与判据见归档** `docs/archive/e2-plan-full-2026-09-26.md.gz` §13。

##### T-B6 **R-3 判据**：同一模块连跑两次 `build`，第二次**显著更快** ✓（数字进台账）+ 产物目录内容可读（`--json` 能列 ✓）+ `.gitignore` 指引 ✓

已收口 ✓（E2 50/50）。**逐字规格与判据见归档** `docs/archive/e2-plan-full-2026-09-26.md.gz` §13。

##### T-B7 **阶段 B 收尾**：基准复量（① ② 应变好 ✓）→ gate 全绿 → 一次 push → CI 绿 → bump `0.67.0` → release → 核对 ✓

已收口 ✓（E2 50/50）。**逐字规格与判据见归档** `docs/archive/e2-plan-full-2026-09-26.md.gz` §13。

##### T-C1 **设计**：`plan_module(root)` 的契约 —— 把模块根下**全部**文件作为**一个 unit 集**编一次 ✓、按文件给出报告 ✓、`ok(file)` 的语义与今天"以它为入口编一次"**逐项等价**（或明确记录差异 ✓）

已收口 ✓（E2 50/50）。**逐字规格与判据见归档** `docs/archive/e2-plan-full-2026-09-26.md.gz` §13。

##### T-C2 **实现 API**：新增规划入口（底层 `units: &[SourceUnit]` 本来就支持多单元 ✓）

已收口 ✓（E2 50/50）。**逐字规格与判据见归档** `docs/archive/e2-plan-full-2026-09-26.md.gz` §13。

##### T-C3 **等价性判据**：对同一门课，**逐文件**比较"新 API 的报告"与"旧路径（逐入口编译）的报告" —— 状态、诊断、`decl.checked` 事件**逐项相同** ✓

已收口 ✓（E2 50/50）。**逐字规格与判据见归档** `docs/archive/e2-plan-full-2026-09-26.md.gz` §13。

##### T-C4 **`build <dir>` 接入**（开关 `SOKO_BUILD_MODULE_BATCH=1` 默认**关** ✓，先保证零退化 ✓）

已收口 ✓（E2 50/50）。**逐字规格与判据见归档** `docs/archive/e2-plan-full-2026-09-26.md.gz` §13。

##### T-C5 **量收益**：`build courses/set-theory` 从 **146.07s** 降下来 ✓（数字进台账）；**默认打开** ✓（收益已证 ✓）

已收口 ✓（E2 50/50）。**逐字规格与判据见归档** `docs/archive/e2-plan-full-2026-09-26.md.gz` §13。

##### T-C6 **LSP/agent 接入**：从 `.sokonanoda/` 取模块级产物 ✓（与 B5 合流 ✓）

已收口 ✓（E2 50/50）。**逐字规格与判据见归档** `docs/archive/e2-plan-full-2026-09-26.md.gz` §13。

##### T-C7 **阶段 C 收尾**：基准复量（② 应大幅变好 ✓）→ gate 全绿 → 一次 push → CI 绿 → bump `0.68.0` → release → 核对 ✓

已收口 ✓（E2 50/50）。**逐字规格与判据见归档** `docs/archive/e2-plan-full-2026-09-26.md.gz` §13。

##### T-D1 **把 check-then-add 收进单一函数**（纯重构，**零行为变化** ✓）：`kernel_phase` 里逐条"检查→加入"的逻辑抽成一个可复用单元 ✓

已收口 ✓（E2 50/50）。**逐字规格与判据见归档** `docs/archive/e2-plan-full-2026-09-26.md.gz` §13。

##### T-D2 **D1 判据**：全语料两态 `--json` **逐字节相同** ✓ + 课程计数逐项不变 ✓ + 五步全绿 ✓（**不变量**：这一步不改任何行为 ✓）

已收口 ✓（E2 50/50）。**逐字规格与判据见归档** `docs/archive/e2-plan-full-2026-09-26.md.gz` §13。

##### T-D3 **walk 增量检查（开关默认关）**：walk 边 elaborate 边 `with_env` 检查并 `add_declar` ✓；`SOKO_WALK_CHECK=1` 才启用 ✓ ⇒ 默认路径**零变化零成本** ✓

已收口 ✓（E2 50/50）。**逐字规格与判据见归档** `docs/archive/e2-plan-full-2026-09-26.md.gz` §13。

##### T-D4 **D3 判据**：开关两态 `--json` 逐字节相同 ✓ + 课程计数不变 ✓ + 事件计数不变 ✓（**开关开**时也相同 ✓ ⇒ 证明"两遍检查"语义等价 ✓）

已收口 ✓（E2 50/50）。**逐字规格与判据见归档** `docs/archive/e2-plan-full-2026-09-26.md.gz` §13。

##### T-D5 **judge 接快照（开关默认关）**：`run_by`/`judge_infer` 拿 `Option<&EnvBuilder>` ⇒ `snapshot()` 查合成声明 ✓；拿不到**回退**旧路径 ✓；`SOKO_JUDGE_ENV_REUSE=1` 才启用 ✓

已收口 ✓（E2 50/50）。**逐字规格与判据见归档** `docs/archive/e2-plan-full-2026-09-26.md.gz` §13。

##### T-D6 **量收益 + 默认打开**：`SOKO_JUDGE_STATS` 看 `JUDGE_INFER` 的 miss 成本是否塌下来 ✓；**收益成立才默认打开** ✓（否则保持关闭并记录 ✗）

已收口 ✓（E2 50/50）。**逐字规格与判据见归档** `docs/archive/e2-plan-full-2026-09-26.md.gz` §13。

##### T-D7 **阶段 D-1 收尾**：基准 ① 复量（应大幅变好 ✓）→ gate + 四件套 → 一次 push → CI 绿 → bump **`0.69.0`（minor）** → release → 核对 ✓

已收口 ✓（E2 50/50）。**逐字规格与判据见归档** `docs/archive/e2-plan-full-2026-09-26.md.gz` §13。

##### T-D8 **去掉重复检查**（第二刀）：`kernel_phase` 不再重查 walk 已核的声明 ✓（**只删重复** ✓，语义由 D4 的对拍保证 ✓）

已收口 ✓（E2 50/50）。**逐字规格与判据见归档** `docs/archive/e2-plan-full-2026-09-26.md.gz` §13。

##### T-D9 **D8 判据 + 量收益**：四件套全过 ✓ + 基准 ① 再降 ✓（数字进台账 ✓）

已收口 ✓（E2 50/50）。**逐字规格与判据见归档** `docs/archive/e2-plan-full-2026-09-26.md.gz` §13。

##### T-D10 **阶段 D-2 收尾**：bump **`0.70.0`** → release → 核对 ✓

已收口 ✓（E2 50/50）。**逐字规格与判据见归档** `docs/archive/e2-plan-full-2026-09-26.md.gz` §13。

##### T-D11 **跨会话复用**（第三刀）：把"已验证的前缀环境"按 `.sokonanoda/` 持久化 ✓ ⇒ **编辑中热编译**再降 ✓（生命线 ✓）

已收口 ✓（E2 50/50）。**逐字规格与判据见归档** `docs/archive/e2-plan-full-2026-09-26.md.gz` §13。

##### T-D12 **D11 判据**：keystroke 基准复量 ✓ + 四件套 ✓

已收口 ✓（E2 50/50）。**逐字规格与判据见归档** `docs/archive/e2-plan-full-2026-09-26.md.gz` §13。

##### T-D13 **阶段 D-3 收尾**：bump **`0.71.0`** → release → 核对 ✓

已收口 ✓（E2 50/50）。**逐字规格与判据见归档** `docs/archive/e2-plan-full-2026-09-26.md.gz` §13。

##### T-E1 **性能回归进 CI**：把三个基准做成 CI 可跑的 smoke（阈值宽松 ✓，只抓**大幅退化** ✗）

已收口 ✓（E2 50/50）。**逐字规格与判据见归档** `docs/archive/e2-plan-full-2026-09-26.md.gz` §13。

##### T-E2 **文档收口**：`architecture.md` §6 台账 / `docs/PERF.md` / 技能与 `AGENTS.md` 同步 ✓

已收口 ✓（E2 50/50）。**逐字规格与判据见归档** `docs/archive/e2-plan-full-2026-09-26.md.gz` §13。

##### T-E3 **`STATUS.md` 与 `docs/ONBOARDING.md` 更新** ✓

已收口 ✓（E2 50/50）。**逐字规格与判据见归档** `docs/archive/e2-plan-full-2026-09-26.md.gz` §13。

##### T-E4 **阶段 E 收尾**：bump **`0.72.0`** → release → 核对 ✓

已收口 ✓（E2 50/50）。**逐字规格与判据见归档** `docs/archive/e2-plan-full-2026-09-26.md.gz` §13。

##### T-U1 **统一接口的设计**：新增 `docs/design/notation-display.md`，定义唯一入口 `DisplayNotations::render(expr) -> Rendered { text, runs }`（一次产出文本与分段）；含**四套实现的去向表**、**调用白名单**、"`text` 与 `runs` 拼接必须逐字节相同"的不变量

已收口 ✓（E2 50/50）。**逐字规格与判据见归档** `docs/archive/e2-plan-full-2026-09-26.md.gz` §13。

##### T-U2 **接口落地（零行为变化）**：实现 `render`（内部固定 `render_expr` → `print_back` → `tag_runs` 三段），四处实现逐个改调它

已收口 ✓（E2 50/50）。**逐字规格与判据见归档** `docs/archive/e2-plan-full-2026-09-26.md.gz` §13。

##### T-U3 **A∖B 守卫：防止长出第五套**：新增 `scripts/audit-notation-paths.py`，扫描 `render_expr(`/`print_back(`/`tag_runs_with_notations(` 的每个调用点，不在白名单就判红；**反向验证**（指向本阶段之前的版本必须报红）；进 `scripts/soko gate` 与 CI

已收口 ✓（E2 50/50）。**逐字规格与判据见归档** `docs/archive/e2-plan-full-2026-09-26.md.gz` §13。

##### T-U4 **目标链统一（用户看得见的那条）**：`goals::open_goal` + `walk.rs:555/735/778` 全部走统一接口；判据三层齐（front 单测带 `∃`、wire `goal`/`goal_runs` 同源、**真宿主 e2e 顶部目标出现 `∃`**）

已收口 ✓（E2 50/50）。**逐字规格与判据见归档** `docs/archive/e2-plan-full-2026-09-26.md.gz` §13。

##### T-U5 **类型/值链统一**：`kernel_phase` 的 `ty_text`/`val_text` 与 `query::runs` 改走统一接口；**新增接缝守卫**：同一声明的 `text` 与 `runs` 拼接逐字节相同

已收口 ✓（E2 50/50）。**逐字规格与判据见归档** `docs/archive/e2-plan-full-2026-09-26.md.gz` §13。

##### T-U6 **词法根因的独立判据**：`token.rs` 的"基础多字符算符更长时让路"补专门用例（声明符号与 `->`/`=>` 相撞），并确认 `semantic.rs` 的 R-2 哨兵仍绿

已收口 ✓（E2 50/50）。**逐字规格与判据见归档** `docs/archive/e2-plan-full-2026-09-26.md.gz` §13。

##### T-U7 **e2e 判据入册**：`exists_fun` 用例在真 VS Code 跑绿并提交，按批次记 `docs/e2e/ledger.jsonl`；顺带核查陈旧服务器造成的假红

已收口 ✓（E2 50/50）。**逐字规格与判据见归档** `docs/archive/e2-plan-full-2026-09-26.md.gz` §13。

##### T-U8 **阶段收尾**：`docs/architecture.md` 写明唯一接口与四套实现的退役；`cargo test --workspace` + `scripts/soko gate` 全绿；一次 push → CI 绿 → bump → auto-tag → release → `gh release list` 核对

已收口 ✓（E2 50/50）。**逐字规格与判据见归档** `docs/archive/e2-plan-full-2026-09-26.md.gz` §13。

##### T-U9 **全仓"重复实现"审计**（用户要求）：三个只读 subagent 分头查 front / CLI·LSP·query / scripts·编辑器；每条结论带 file:line 或可复跑命令，报告进 `docs/design/duplication-audit.md`

已收口 ✓（E2 50/50）。**逐字规格与判据见归档** `docs/archive/e2-plan-full-2026-09-26.md.gz` §13。

##### T-U10 **审计结论收口**：每项或立刻做 / 或立守卫 / 或写台账（不做，说明理由）；新增守卫一律棘轮化（基线 + 只拦新增）

已收口 ✓（E2 50/50）。**逐字规格与判据见归档** `docs/archive/e2-plan-full-2026-09-26.md.gz` §13。

##### T-U11 **77 处绕过逐条收口**（用户要求）：硬规则 = 用户可见文本必须迁移唯一接口或补"必须折叠"判据；优先 LSP 5 处 + elab 19 处

已收口 ✓（E2 50/50）。**逐字规格与判据见归档** `docs/archive/e2-plan-full-2026-09-26.md.gz` §13。

##### T-U12 **面级 sweep 判据**（用户要求）：hover/goal/诊断/状态栏/项目树 各造含记法类型，断言无点形式；回退 T-U4/T-U5 折叠必须判红

已收口 ✓（E2 50/50）。**逐字规格与判据见归档** `docs/archive/e2-plan-full-2026-09-26.md.gz` §13。

---

## 阶段 U —— **记法转化统一接口**（2026-09-25 用户要求：「我要求完全统一接口」✓）

> **⚡ 优先级（2026-09-25 用户要求）**：**本阶段排在阶段 D/E 之前** ✓ ——
> 它修的是**用户看得见**的东西（Infoview 顶部「目标」✗），且是**架构级**要求
> （"完全统一接口" ✓）；阶段 D/E 是性能与文档收口，可以随后 ✓。
>
> **为什么单独立一个阶段**：2026-09-25 的用户报告（Infoview 顶部「目标」里 `∃` 不转化 ✗）
> 查出来的**不是一处 bug，而是四处实现** ✗：
> | # | 实现 | 干什么 | 谁在用 |
> |---|---|---|---|
> | ① | `display::print_back` | **真的转化**（文本→文本 ✓） | `ty_text`/`val_text` |
> | ② | `semantic::tag_runs_with_notations` | **只打标签**（不转化 ✗） | `query::runs` ⇒ `goal_runs`/`ty_runs`（**Infoview 渲染读它** ✓） |
> | ③ | `display::render_expr`（=`proof::render_expr`） | **只渲染**（AST→文本 ✗） | `goals::open_goal`、`walk.rs` 三处目标 |
> | ④ | 内核 pp（`info.goal`） | 点形式 ✗ | `walk.rs:555/778`（tactic 步进） |
> ⇒ "渲染"与"折叠"被拆成两步 ✓，而目标生产链（③④⇒②）**只渲染不折叠** ✗ ⇒
> 顶部「目标」永远是点形式 ✓。`AGENTS.md` 的"真相与显示是两条路"在这里升级为
> "**连显示自己都分了四条路**" ✗。细节与证据：`REQUIREMENTS.md` §9 ㉔/㉕ ✓。

- [x] `T-U1` **统一接口的设计**（设计先行 ✓）：**设计已成文 ⇒ `docs/design/notation-display.md`** ✓（唯一接口 `DisplayNotations::render`/`render_text` + 四套实现去向表 + 调用白名单 + 不变量 `runs 拼接 == text` + 落地顺序 ✓）
- [x] `T-U2` **接口落地（先零行为变化 ✓）**：实现 `render`（内部 = `render_expr` → `print_back` →
- [x] `T-U3` **A∖B 守卫：防止长出第五套** ✓：新增 `scripts/audit-notation-paths.py` —— 扫描
- [x] `T-U4` **目标链统一（用户看得见的那条 ✓）**：`goals::open_goal` + `walk.rs` 的三处目标生产
- [x] `T-U5` **类型/值链统一**：`kernel_phase` 的 `ty_text`/`val_text` 与 `query::runs`
- [x] `T-U6` **词法根因的独立判据**：给 `token.rs` 的"基础多字符算符更长时让路"补一条专门用例 ✓
- [x] `T-U7` **e2e 判据入册**：把 `exists_fun` 那条用例在**真 VS Code** 里跑绿并提交 ✓，
- [x] `T-U9` **全仓"重复实现"审计**（用户要求 ✓）：「一模一样的功能、多处实现、导致 bug」——
- [x] `T-U11` **77 处绕过逐条收口**（用户 2026-09-25 要求 ✓）：硬规则 = **凡产出用户可见
- [x] `T-U12` **面级 sweep 判据**（用户要求 ✓）—— **完成** ✓（**五个面都有处置** ✓）
- [x] `T-U10` **审计结论收口**：对每一项或"立刻做"或"立守卫"或"写进台账（不做，说明理由）"✓；
- [x] `T-U8` **阶段收尾**：`docs/architecture.md` 写明**唯一接口 + 四套实现的退役** ✓；

---

## 阶段 N —— **记法 × 隐式参数 × 产品交互**（2026-09-26 用户专项）

> **用户原话**：「全面一点，专门针对 notation，产品交互。包括多来点隐式参数，让
> notation 更直观」。⇒ 三条主线：**A** 记法铺到每一个用户可见面（唯一接口
> `DisplayNotations`）、**B** 隐式实参（IA-2：扩宽求解 + 课程库隐式化 + 收窄补参
> hack）、**C** 验收留档。**纪律**：每条先判红 → 一处一 commit → 自带判据 +
> 反向验证；用户可见面必须有**真宿主 e2e**（`editor/vscode/src/test/extension.test.js`）。

### T-N1 A1：`fold` 折 `->` → `→`（**用户报告第 1 条**）
契约：`DisplayNotations::fold` 把内核 pp 的 ASCII `->` 折成 `→`；**只换那两个
字节**（与 `forall` 关键字同一条"只有折过的 span 变"纪律）。两处都要认：
`Expr::Arrow`（`α -> β`）与**匿名 binder 的 `Forall`**（源级 `(x : α) -> β`）。
**副发现（同轮修）**：`print_back` 的 `base` 原来"反推"成
`ast.span().start.offset - 前导空白`，而 parser 给带括号原子的 span **不含括号**
⇒ 整条表达式被括号包住时（`(α -> β) -> γ`）`base` 偏大、`splice` 每次都切在字符
中间 ⇒ **一个字节都不折**。改成用唯一源 `proof::CHECK_PREFIX`。
判据：`crates/front/src/display.rs` 的 `arrows_fold_to_the_unicode_arrow`（含幂等）。

### T-N2 A2：集合字面量折回 `{a}` / `{a, b}`（**用户报告第 2 条**）
契约：`fold_spine` 里照 `forall` 先例单独认**内建语法**（`{a}` 不是记法声明，
表里查不到）：`Set.singleton α a` → `Expr::SetLiteral{[a]}`、`Set.pair α a b` →
`{[a, b]}`；**只有完全应用**才折（与记法同规则）。渲染复用既有的
`render_expr(SetLiteral)`，不新增第二套括号/逗号规则。
判据：`crates/front/src/display.rs` 的 `set_literals_fold_back_to_braces`。

### T-N3 A0：68 条基线逐条结论 + 设计文档对齐 as-built
契约：`scripts/notation-paths-baseline.txt` 的**每一条**都要有结论（迁移 / 立判据 /
台账不做），带 `文件:行号` 证据链，写进 `docs/design/duplication-audit.md`；
`docs/design/notation-display.md` 的接口形状必须与 as-built 一致（当前文档写着
`render_text` / `Rendered` / `render(...).text`，而代码里 `render`/`fold` 返回
`String`、`render_text` **不存在**、`DisplayNotations::runs` **零调用者** ⇒ 照文档
写会编译不过）。
判据：审计表 + `python3 scripts/plan.py check` 不漂移。

### T-N4 A0：把**用户可见**的绕过迁到唯一接口，并削基线
契约：`crates/front/src/query/mod.rs` 的 `runs()`（Infoview 的**每一个**目标/类型/
假设着色，7 条 wire 字段都出自它）与 `crates/front/src/by.rs` 的 **8 处用户可见
诊断消息**（`exact`/`have`/`apply`/`cases` 的报错，`prefix_src` 已在签名里 ⇒ 就地
`fold_for_display(prefix_src, &render_expr(x))`，零签名改动）改走唯一接口。
**修正既有审计**：`duplication-audit.md` 的 C 组把 `by.rs` 22 条全判成"判定侧"是
错的（8 条是显示面）。
判据：迁移后 `python3 scripts/audit-notation-paths.py --rebless` 基线**变短**；
反向验证 = 把一处改回直调 ⇒ 守卫判红。

### T-N5 A1/A2：真宿主 e2e
契约：真 VS Code 里断言 Infoview 的 `⊢` 后文本（a）含 `→`（b）`runs` 拼接**逐字节
等于** `text`；含 `{a}` 的声明在 Infoview/ty 面显示 `{a}`。
判据：`editor/vscode/src/test/extension.test.js` 两条用例名入册 `docs/e2e/ledger.jsonl`。

### T-N6 A3：`{a}` 的 goto-definition（**用户报告第 3 条**）
契约：光标落在 `{a}` 上按 F12 ⇒ 跳到 `Set.singleton` 的**声明处**（跨文件到
`courses/set-theory/lib/Set.sokonanoda`）。符号型记法（`∈`）的 goto 已有 e2e
（`extension.test.js` 的 `go to definition on a notation symbol …`）⇒ 本条补的是
**带操作数的括号记法**这一档。做法与符号记法同一条通道（front 的 hover 节点带
`ResolvedTarget::Notation`/`Declaration` + LSP 的 `project_definition`）。
判据：真宿主 e2e；反向验证 = 撤掉该分支 ⇒ 判红。

### T-N7 A4 取证 + 路线（**用户报告第 4 条**）
契约：查清 prelude 为什么跳不动（断链点 = `crates/front/src/compile/check/
kernel_phase.rs` 的 `resolution` 回填只查用户文件的 `top_level_def_spans`），并在
三条路线（物化成真文件 / 合成位置 / **只读虚拟文档**）里拍板一条。
判据：断链点有 `文件:行号` 证据；路线写清改动面与风险。

### T-N8 A4：prelude 可跳转
契约：`Or` / `And` / `Iff` / `False` 等 prelude 名的 definition 返回**真实位置**
（由 `top_level_def_spans(&parse(PRELUDE_*_SRC))` 算出的真 span，**不是文本比对**），
指向**只读**虚拟文档；hover 说明"内置前奏 + 位置"。
判据：三层各一条（front 真值表 / LSP wire 契约 / 真宿主 e2e）；**反向验证** =
注掉 prelude 分支 ⇒ 三层同时红。**已知缺口**：`Nat`/`Bool` 家族 9 名没有源码常量
（Rust AST 手搓）⇒ 要么补源码常量，要么明确记账。

### T-N9 B0：IA-1 as-built 复核
契约：不重新设计。核对 `docs/design/implicit-arguments.md` §9 的 as-built 与代码
一致：签名表 `implicit_prefix`（`compile/elab.rs`、`compile/check/walk.rs`）、
`compile/implicit.rs` 的 `telescope`/`leading_implicit`/`solve_prefix`、唯一钩子
`try_implicit_application`（`Expr::App` 臂）、`@` 真语义、错误码
`elab-implicit-argument-unsolved`；并复核课程侧 `Function.comp`/`Rel.comp` 现状。
判据：每条 as-built 都能指到 `文件:行号`；不符的写进结论。

### T-N10 B1 判红：R5 最小复现
契约：先造出**判红**的最小复现——记法 `''` / `⁻¹'` 的操作数是 λ 或"类型标注写不出
来"的形状时，`solve_prefix_args` 的路线 ① 拿不到实参类型 ⇒
`elab-notation-argument-unsolved`（`courses/set-theory/units/unit12-synthesis.sokonanoda`
的 `soko:notation-ok: R5` 标记就是这么来的）。复现件进
`crates/cli/tests/notation.rs`（或 `implicit.rs` 单测）。
判据：复现件在修复前**红**。

### T-N11 B1 修：扩宽 `solve_prefix`
契约：按设计 §3.1 把求解扩宽——(a) **期望类型**（bidirectional）参与、(b) 允许
**推迟求解**（deferral）到上下文足够、(c) 诊断给"把参数写全"的可执行引导。
**不猜、不搜索、不回溯、不引入元变量**（路线 C 的红线）。内核零改动。
判据：T-N10 的复现件转绿 + `crates/front/src/compile/implicit.rs` 单测。

### T-N12 B1 三件套判据
契约：① **正向**——`flawed_equalities_refuted` 那一族在**删掉 `soko:notation-ok`
标记**后自动记法化；② **反向**——回退该修复 ⇒ 立刻退化（判红）；③ **红线**——
判定正确性不变（全语料逐字节对拍 + 课程计数 `check.py` 逐项不变）。
判据：三条都要有可执行命令与输出。

### T-N16 A0 立判据收尾（2026-09-26）

基线 89 → 68 → **59** 之后，剩下的 59 已不是待办而是「两类别动」（§3 立判据 19 / §4 台账不做 40）。
本轮补齐**最后一格**：`goals.rs` 的 9 处 `SubGoal.ty`（审计 §3 标 ⏳ 的那一组）——
它的产物**同时**是用户可见文本（wire + hover）**和**判定输入（`suggest.rs::hole_goal_text`
把它当 `OpenGoalSpec.ty` 回读）⇒ 拆成两半：**显示副本折**（`query/mod.rs` 与 LSP hover 各做一份克隆）、
**真相字段不折**。判据两半各一条，都带反向验证；`59 是地板`的结论写进审计 §1.5。

### T-N13 B2：课程库改隐式风格
契约：`courses/set-theory/lib/Image.sokonanoda` 的 `Set.image`/`Set.preimage` 一族
（及同族的 `Function.comp`/`Rel.comp`）签名改**隐式**前导类型参数；调用点不必再写
全 `α β`。**口径**：与设计 §3.4 的护城河话术同轮改（`Set.mem a A` 从"被拒"变
"通过"是**有意**的契约变更）。
判据：课程文件里点名应用点**数量级下降**（给前后数字）+ 显示面呈记法态 + 课程
计数与判定不变（`python3 courses/set-theory/tools/check.py`）。

### T-N13 B2 as-built（2026-09-30 **收口 ✓**）

**契约**：`lib/Image` 的 `Set.image`/`Set.preimage` 一族（及同族的 `Function.comp`/`Rel.comp`）
签名改**隐式**前导类型参数；调用点不必再写全 `α β`；课程计数与判定不变。

**① 库签名隐式化**（档1，commit `98c5804f`）：`lib/Set`（12 def + 1 abbrev + 1 axiom + 13 定理；
`def Set` 保持显式 —— Lean 里它也是显式的）+ `lib/Image`（6 条）+ `Function.comp`/`Function.comp_apply`
+ `Rel.comp`/`Rel.comp_apply`；库内调用点同步改短写（`subset B A`、`univ`、`sep A P`、`Set.ext (sep …)`）。
判据：16 个 lib 逐个 `query check --root courses/set-theory` ⇒ **exit 0 / failed 0** ✓。

**② 迁移暴露的六条前端根因**（档2，commit `1d1ff0a9`；每条都有 front 判据 + **反向验证**）：

| # | 根因（一句话） | 修法 |
|---|---|---|
| ① | `cases` 的被消去项类型来自内核 pp，而 pp 丢**第一个隐式实参** ⇒ delta 展开右对齐错位（`Set.union A B x` 对成 `α := A, A := B, B := x`） | `spine::unfold_one` 分**两种读法**（Short 逐位对**显式**形参 / Old 对全部形参），`unfold_to_inductive` 先短写、到不了归纳头再旧写法 |
| ② | 零元记法在**函数位**（`(∅) x`）不被 `try_implicit_application` 认 | 认 `Expr::Notation{lhs:None,rhs:None}` 头，并按目标常量 elaborate ⇒ 走与 `Set.univ x` 同一条路线③ |
| ③ | `unify_extract` 只认「实参位**恰好**是裸变量」⇒ `𝒫 (Set.univ α)` 的 `α` 在**嵌套**位解不出 | 实参位模板**含**该变量时递归匹配；多位置命中要求取值一致（不一致 ⇒ `None`，不猜） |
| ④ | `solve_prefix` 只展开**实参侧**（记法路径 R5 两边都展开）⇒ `Set ?α`（def）对不上箭头形态的实参类型 | 模板侧也展开；⚠ 闸门：实际项**剥到底**的陪域必须是 Sort（否则 `Set.univ` 的 Pi 域会被吃成 `α := Type 0`） |
| ⑤ | 旧写法里某位是**零元隐式常量**（`Eq.trans.{1} (Set Two) … (∅) …`）⇒ 逐位「贴合」否掉 ⇒ 路线③ 抢走 ⇒ 组装错位 | 路线③ 闸门补 `starts_old_style`（开头落在 `layers[0]` 上 ⇒ 是旧写法，路线③ 让位） |
| ⑥ | 解出来的隐式实参不给期望类型 ⇒ pp 回读的裸 `Set.empty` 停在自己的 Pi 上 | 解出值也吃**该层的域**当期望类型（主路径与路线③ 两处） |

**③ 调用点迁移**（档3，commit `9e07d6c7`）：10 文件 / 19 诊断 ⇒ **0**。其中 **4 条声明**按
「**项风格 + 显式前提**」重写 —— 不是绕过判定：`by` 块的 tactic 路径在「pp 回读 +
`Eq.subst`/`And.left`/`⊆`-展开」一族上会撞**判定缝**（同一个命题两边 pp 一样而内核判不等：
`期望 C y，实际是 C y`），项风格不撞；`unit10` 的 `diag` 抽成顶层 `diag_helper`（项风格外层没有 `have`）。

**判据（逐项实测）**：

* 课程门禁 `python3 courses/set-theory/tools/check.py` ⇒ **43 目标 · 377 checked · 99 open · 0 判负**
  （基线 376 checked；**+1** 是 `unit10-solution` 新增的顶层 `diag_helper` ✓）。
* `cargo test -p sokonanoda-front` ⇒ **769/0**（新增 6 条判据，**逐条反向验证** ✓）。
* 非课程语料逐字节对拍（`grade --json` + `query check` × **172 组**）⇒ **0 差异** ✓（红线）。
* `python3 scripts/notation-lint.py` ⇒ OK（豁免 587 处，**全部带标记**）·
  `python3 scripts/gap.py check` ⇒ 全部与台账一致 ✓。

**遗留（写清楚，不装作没有）**：

1. **判定缝本身没修**：上面 4 条声明是**换写法**过的，不是根因修好。缝的形状固定为
   「tactic 路径把目标/假设过一遍 pp 回读 ⇒ 类型与项在**隐式实参或宇宙层级**上不同形」——
   与 E19/E20（给求解器加元变量）是同一片地；**复现件**留作 E19 的取证材料（`unit08-solution`
   的 `preimage_inter` 原写法：`apply Set.ext; intro x; constructor; intro h; exact And.intro (And.left h) (And.right h)`）。
2. **点名叫法没有全量迁移**：单元里仍有一批「写全前导类型参数」的调用点（带 `soko:notation-ok` 标记，
   例如 `Set.image` 的操作数是零元记法 `∅` 时没有类型来源）⇒ 契约里「调用点数量级下降」这一条
   **只做到「不再必须写」**，没做到「全部改写成短写」；逐站点迁移是后续独立工作。
3. **G-31 的复现件缺位**：它的 `repro` 指向 G-29 的脚本（只量「编辑重编闭包」，不量 `by` 前缀重跑）
   ⇒ 本轮的 `fixed` 依据是 P 组的读数（`judge_ms −83%` / `JUDGE_PREFIX runs −79%`），补它自己的判据留作后续。

### T-N14 B3：记法路径改走隐式插入，收窄补参 hack

契约：`elab_notation` 今天自己 `mk_const` + 补前导实参（绕过 `Expr::App` 的唯一
钩子）⇒ 让它复用 `try_implicit_application` 的同一条机械；`notation_prefix_args` /
`solve_prefix_args` 的调用点数量**降到 0**（给前后数字），
`implicit_prefix == 0` 时行为**逐字节不变**（免费闸门仍在）。
判据：N7 五元组相等契约（`crates/cli/tests/notation.rs`）仍绿 + 全语料对拍。

**as-built（2026-09-30 收口 ✓）**：

* **隐式档走唯一钩子**（`elab_notation_implicit`）：目标签名 `implicit_prefix > 0`
  时，把 `symbol(op₁ … opₙ)` 还原成**源级应用脊** `target op₁ … opₙ` 交给
  `elab_expr` —— 前缀由 `try_implicit_application` 解出，记法路径不再有第二套补参
  机械。**模式 B**（`Set.univ ∩ A` 把 `α` 解成 `Type 0` ⇒ `Sort(2)` 撞 `Sort(1)`）
  由此收口 ✓。
* **唯一钩子的第二个缺口**：`try_implicit_application` 原来**先** elaborate 第一个
  显式实参、再解前缀 ⇒ 该实参拿不到期望类型。`Set.inter Set.univ A` /
  `Set.union Set.empty A` 这种"实参本身是零元隐式常量"的形状因此停在 Pi 上 ✗。
  改成**先解前缀、再逐个实参给「代入后」的期望类型**（第一位与其余一视同仁）⇒
  **模式 D**（`Set.image f A y` 一族）收口 ✓。
* **调用点数字（实测）**：`notation_prefix_args` / `solve_prefix_args` 各 **1 个**
  调用点，**都留在 `implicit_prefix == 0` 那条分支里** —— 它们是**护城河**：内建
  记法（`=` `∧` `∨` `↔` `¬` `∃`，探针实测目标全是 `k=0`）与用户显式签名的目标
  必须靠"操作数反解前导**显式**参数"才写得出来（`Eq α a b`），而应用路径**不许**
  补显式前导参数（那正是 `Set.mem a A` 被拒的那条设计）。⇒ **隐式目标上的执行次数
  是 0**（这才是 B2 需要的），k==0 那一档**收窄保留** ✓。
* **红线**：全语料对拍 **644 组逐字节相同** ✓ · 课程门禁 **43 目标 / 376 checked /
  99 open / 0 判负** 逐项不变 ✓ · front **794/0** ✓ · CLI 记法契约 **50/0** ✓ ·
  两条新判据带**反向验证**（撤掉任一改动 ⇒ 当场判红）✓。
* **G-43 不在本条**（它是 T-N13/B2 的剩余）：`Iff.intro (fun (h : y ∈ f '' A) => h) …`
  这一族（构造子 + lambda 实参，binder 类型里含导入模块的 def）仍红 ✗ —— 与记法
  路径无关（`@Iff.intro` 版是绿的），复现件 `docs/gaps/repro/G43-*.sh` 仍 exit 0。

### T-N15 C 收尾：台账 + 清单 + 需求登记
契约：每条改动的数字进 `docs/perf/ledger.jsonl` 或对应台账；给一份**「看得见的
变化」清单**（逐条对应用户报的 6 条 + 隐式参数，带 e2e 断言名）；登记
`REQUIREMENTS.md` §9（日期 2026-09-26）；同轮同步 `editor/vscode/` 与 `skills/`。
判据：`python3 scripts/docs-lint.py` + `python3 scripts/plan.py check` + 课程门禁。

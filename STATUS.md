# 当前状态与进度日志（agents 先读这里）

> 快照：2026-09-21（第一百二十九轮：**逐 surface 的判别性** —— 线 C 收口并发版；
> 折叠开关 `SOKO_NO_NOTATION_FOLD=1` 实测 **3 红 3 绿**（与设计逐格一致）；
> 课程门禁 36 目标 · 328 checked · 99 open · 0 判负**逐项不变**；版本 **0.65.0**）
> 仓库：`sokonanoda-lang`；权威计划 = `ROADMAP.md`；**用户要求总账 = `REQUIREMENTS.md`（先读）**；
> **文档地图 = `docs/README.md`**（入口/权威在仓库根，开发者参考在 `docs/` 顶层，
> 设计在 `docs/design/`，调研笔记在 `docs/notes/`）；
> 架构/内核 = `docs/architecture.md`；协议 = `docs/protocol.md`；测试地图 = `docs/TESTING.md`；
> 经验台账 = `docs/LESSONS.md`；CI 失败台账 = `docs/CI-FAILURES.md`；发布 = `docs/RELEASE.md`；
> agent 入口 = `AGENTS.md` + `skills/`；**harness 适配 = `docs/design/deepseek-harness.md`**；
> **agent 查询通道 = `docs/design/agent-query-channel.md`**（ROADMAP I15）。

## 一句话

`.sokonanoda` = **纯声明式教学文件（无 `#` 命令）+ 完整 sokonanoda 内核 + LSP 反馈通道**。
练习 = 带 `sorry` 洞的 `def name : T` / `theorem name : T` / `example : T` 声明。
CLI/REPL 的 `#check` 等只是调试/自测工具，不是文件格式。

## 本轮进度（2026-09-21，第一百二十九轮：**逐 surface 的判别性** —— 线 C 收口，bump 0.65.0）

> 上一轮把记法接进了 goal / 声明类型 / `by` 步进。本轮回答一个更硬的问题：
> **这些测试真的抓得住折叠层吗？** 判据不是"看到了记法"，而是**把折叠关掉必须红**
> ——而且这条判据要**从折叠层一直钉到 wire 字段**。

1. **开关 `SOKO_NO_NOTATION_FOLD=1`**（`display_notations` 直接返回空表，仿
   `SOKO_NO_JUDGE_BATCH`）。它**只关展示**：判定读的是另一张表
   （`notation.rs::notation_table`），所以它同时是"显示改动没碰判定"的开关。
2. **两条机械判据（缺一不可）**：
   * **折叠层**（front 单测）`display::tests::with_the_fold_off_every_foldable_surface_is_pointwise`
     ——空表 ⇒ 一律点名；
   * **整条路**（CLI 真二进制 A/B，新文件 `crates/cli/tests/notation_fold.rs` **3 条**）
     ——① 三个可折 surface 关掉 ⇒ 回到点名；② 两个源级 surface 关掉 ⇒ **一个字节不变**；
     ③ `grade --json` 开关前后**逐字节相同**（判定没被碰）。
     **为什么非要第二条**：单测用的是 `DisplayNotations::default()`，它**碰不到开关本身**
     （`display_notations` 里那个 `if`）——开关被删、或有哪条路绕过 `display_notations`，
     单测照样绿。② 同时是**行程开关**：谁把生产者 2 改成走内核 pp 折叠，它就会红。
3. **实测矩阵**（`SOKO_NO_NOTATION_FOLD=1 cargo test -p sokonanoda-front --lib -- <六条>`
   → `3 passed; 3 failed`，与设计逐格一致）：

   | surface | 关掉折叠 | 读法 |
   |---|---|---|
   | 根状态 `state_at_root_before_any_tactic` | **红** | 记法是折叠给的 |
   | 声明 `ty` `a_declarations_ty_is_notation_folded_too` | **红** | 同上 |
   | `by` 步进 `by_step_display_is_folded_but_the_judge_input_is_not` | **红** | 同上 |
   | 无 `by` 的开练习 `an_open_exercise_without_by_keeps_notation_in_its_goal` | 绿 | **守护**：源级渲染（T-C21） |
   | 假设行 `state_binders_keep_notation_in_their_types` | 绿 | **守护**：源里写的类型（T-C23） |
   | 机械判据 `with_the_fold_off_every_foldable_surface_is_pointwise` | 绿 | 空表 ⇒ 一律点名 |

   后两条**故意不红**（它们的记法不是折叠给的），红才是异常。设计 §3.3e 记了全表。
4. **真宿主 e2e 用例 #6 转绿**（`goal text uses the file's notation`）：它此前是矩阵里
   三条已知红之一（断言 `infoview.lastState().goal` 含 `⊆`/`∈`）——线 C 落地后
   **第一次通过**（`1 passed / 0 failed`，server 0.65.0 bundled）。剩下两条红是线 D 的
   （记法跳定义 / hover 原始类型）。
5. **版本 0.64.2 → 0.65.0**（minor，§13 给 T-C24 标的发版点）：`Cargo.toml` +
   `editor/vscode/package.json` + `Cargo.lock` + 两处 `requires`（`course/shared`、
   `courses/set-theory`）。CHANGELOG 逐 surface 写清"以前点名 / 现在记法"，
   并写明**判定一个字节没动**。
6. **判据**：`scripts/soko gate` **exit 0**（fmt · clippy · test · playground 锚点 ·
   课程门禁 · 记法 lint 84 文件 · 版本一致 0.65.0 · 缺口台账 41 条全一致）·
   课程门禁计数**逐项不变**（36 目标 · 328 checked · 99 open · **0 判负**）·
   `plan.py check` OK（120 环节）。
7. **下一环**：T-C25（命中不了就回退：`prefix` / `postfix` / 零元 `notation` /
   binder 记法 / 重载歧义各一条）→ C-IV 着色（T-C30/T-C31/T-C32）→ 矩阵其余用例
   （T-C50）→ T-C40/T-C41 收尾。

## 本轮进度（2026-09-21，第一百二十九轮：**线 C 收口 + 0.65.0** —— 四个 surface 全部有记法）

> 承上一轮（goal 用上记法、G-26 关账），本轮把线 C 的**生产者 4**（`by` 步进）
> 补上、给假设行补守护、做逐 surface 的判别性测试，并**发 minor 0.65.0**。

1. **T-C22 `by` 步进的展示副本**：`apply` 出来的子目标来自被应用引理的**内核 pp
   望远镜** ⇒ 一直是点名（`(x : α) -> Iff (A x) (B x)`）。表整趟建一次
   （`run_pass` 的 `display_notations`），`Walk` 与 `finish_pass` **共用**；折叠点
   选在 **`by_step_states`**——它把引擎的 `ByGoal` 转成报告层 `ByStepState`，
   **那就是展示边界**，引擎手里的 AST 一个字节没动。实测 `(x : α) -> (A x) ↔ (B x)` ✓
   **判据两面都要**（计划点名的"最容易出错的地方"）：展示含记法 **且** 同一个 `by`
   块后面的 `exact h` 仍然判过（`status == "checked"`）。
   **踩到的坑**：重构时把"表为空就早退"放在了**加内建记法之前** ⇒ 没有 `infix` 的
   文件连内建的 `∧` 都没了。内建记法**永远生效**，早退不能挡在它前面。
2. **T-C23 假设行**：实测**本来就带记法**（binder 类型来自**源里写的**类型 ⇒ 源级
   渲染）。补守护（夹具刻意用**不带 `by`** 的开练习——那条走 `DeclState.binders`，
   与带 `by` 的 by-step 那份是**两条路**）。
3. **T-C24 逐 surface 的判别性**：四条 surface 测试 + 开关
   **`SOKO_NO_NOTATION_FOLD=1`**（空表）。**实测关掉后**：
   | surface | 关掉后 | 读法 |
   |---|---|---|
   | 1 根状态 / 3 声明 `ty` / 4 `by` 步进 | **红** | 记法是折叠给的 |
   | 2 无 `by` 的开练习 / 假设行 | 仍绿 | 记法来自**源级渲染**，不是折叠 ⇒ 那两条是**守护** |
   机械判据：`display::tests::with_the_fold_off_every_foldable_surface_is_pointwise`。
4. **⬆ BUMP minor → 0.65.0**：goal / 假设 / 声明类型**第一次**显示记法。CHANGELOG
   写清"只有记法那几段被替换（binder 分组 / `Type 0` / 折行逐字节保留）"、
   "判定一个字节没动"、以及诊断开关。
5. **判据**：front **703** 条全绿 · `scripts/soko gate` **PASS**（含课程门禁与缺口
   台账）· 课程计数**逐项不变**（36 目标 · 328 checked · 99 open · **0 判负**）·
   `perf-check --case perf_course` 无退化（最大 +6.8%，噪声内）· `bump.py --check`
   一致（0.65.0）· `plan.py check` OK（120 环节）。
   更新的 golden 五处（T-C22）都是预期的可见变化。
6. **线 C 到此四个生产者全部覆盖**。下一环 **T-C25**（折叠的开关与文档收口），
   之后 T-C30–T-C32（语义 run 把记法标成 `notation`）、T-C50、**T-C40/T-C41
   （⬆ BUMP patch）**。

## 本轮进度（2026-09-21，第一百二十八轮：**goal 用上记法** —— 用户报的那条关账）

> 用户最初那条：「infoview 里的 goal 展现没有用 notation 的方式」。本轮把它
> **修掉并关账**（G-26），并给已经好的那条路补上守护。

1. **T-C20 接进生产者 1+3（根状态 / 声明卡片的 `ty_text`）**：`finish_pass` 里
   建一次 `DisplayNotations`，两处 `ty_text` 各过一遍 `print_back`。只动
   `ty_text`（T-C02 的审计：它只有给人看的消费者）；`goal`/`binders[].ty`/
   `sub_goals[].ty` **一个字节没动**（它们同时喂 judge）。

   ```
   demo_subset_def | forall (α : Type 0) (A B : Set α), (A ⊆ B) ↔ ((x : α) -> A x -> B x)
   mem_of_subset   | forall (α : Type 0) (A B : Set α), A ⊆ B -> (forall (a : α), a ∈ A -> a ∈ B)
   根状态（L45）    | 同上（学习者的光标就在 tactic 上，看到的就是它）
   ```
   **`⊆` ✓ `↔` ✓ `∈` ✓，而 binder 分组、`Type 0`、折行全部原样。**
   **G-26 关账**（`fixed_in = 0.64.2`），它的复现件转绿。

2. **接进生产者时撞到的两件事**（设计里没写、实测才知道）：
   * **必须按 span 拼接，不能重渲染整棵树**——重渲染会把折过之外的东西也改样
     （`forall (a b : T),` 拆成箭头链、`Type 0` 重排成 `Sort 1`；`render_expr` 是
     回读通道的输入，它必须那样写）。改成把每处折叠记成 `(span, 文本)`、**只替换
     那几段**（取最外层、从右往左）。两处细节：`parse_expr_text_with` 的 span 多一个
     `"#check "` 前缀（**头部反推**，不硬编码）；解析器给**带括号的原子**的 span
     **不含括号** ⇒ 替换范围要**按括号配平**。
   * **内建记法要自己补**：`↔`/`∧`/`∨`/`¬`/`=`/`≠` 不在任何源文本里（parser 有
     硬编码的 `BUILTIN_NOTATIONS`）⇒ `notation_table` 收不到，`Iff` 永远折不成 `↔`。
3. **顺带修正 arity 的口径**：**元数 = 显式 binder 的个数**——内核 pp **省略隐式
   参数**。`Eq {α : Sort u} (a b : α)` ⇒ 元数 **2**（pp 是 `Eq A B`）；
   `Ne (α : Sort u) (a b : α)` ⇒ **3**。用 telescope 层数会让 `=` 永远折不出来。
4. **T-C21 给生产者 2 补三条守护**：不带 `by` 的开练习那条路本来就保留记法
   （T-C01 的实测），但**此前零测试**。补 front 两条 + LSP wire 一条，断言
   `goal`/`ty` 含记法**且不含点名**。
5. **判据**：`display` **20 条** + `query`/`goals` 新守护全绿 ·
   `scripts/soko gate` **exit 0** · 课程门禁计数**逐项不变**
   （36 目标 · 328 checked · 99 open · **0 判负**）· `perf-check --case perf_course`
   **无退化**（±2.3% 内）· `gap.py check` 全绿（G-26/G-35 已关账）。
   更新的 golden 两处（`query::tests::state_at_root_before_any_tactic` + LSP 两条
   state 用例）都是**预期的**可见变化，注释写明是线 C 的效果。
6. **还剩一处没记法**（实测，下一环 T-C22）：**`apply` 之后的子目标**——
   `apply Set.ext` 后是 `(x : α) -> Iff (A x) (B x)`（子目标来自被应用引理的
   **内核 pp 望远镜**）。那四处同时是**判定输入**，折叠只能作用在**展示副本**上。
   **未 bump**：线 C 的 patch 点在 T-C41。


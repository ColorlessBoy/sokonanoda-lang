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

## 本轮进度（2026-09-21，第一百三十轮：**线 C 的边界、着色与性能账**）

> 承上一轮（四个 surface 全部显示记法 + 0.65.0），本轮把线 C 的**边界**、
> **符号着色**、**导入名分类**收口，并查清了一处真性能退化。

1. **T-C25 边界：命中不了就回退点名，不猜。** 折叠层只做**二元 infix 族**，
   四种形态各一条测试，写法都取课程库的真实例子：`prefix:100 " 𝒫 "` /
   `postfix:100 " ᶜ "` / `notation "∅"` / `binder_notation "∃"` / 元数对不上。
   测试里带**对照**（同一夹具的二元 infix 照折）——防止"表是空的"造成假绿。
   **顺带修掉一个真 bug**：折过的子树**被应用**时就地替换会**改变语义**——
   `(Set.mem α a A) B` 折成 `a ∈ A B`，重新解析是 `Set.mem α a (A B)`。
   规则改成"**上提到应用脊根**，括号交给 `render_expr`" ⇒ `(a ∈ A) B` ✓。
2. **T-C30 记法符号有着色。** wire 上 `⊆`/`↔` 以前是裸 run。计划只写了"补
   `Names::notations`"，实测**另外两件**也得做：符号表要扫**整个闭包**（`∈`/`⊆`
   声明在 `lib/` 里）且**不能 parse**（用库记法的文件单文件 parse 必然失败）
   ⇒ 用词法级扫描；还要把符号**喂给词法**（`↔` 不在数学码点类里，不喂就切成
   `Ident` ⇒ `unknown_ident`）；**内建也要算**（它们不在任何源文本里）。
3. **T-C31 导入名不再 `unknown_ident`。** 闭包级声明表在**编译期算一次**
   （不是每次查询——`state_at` 是光标一动问一次）。实测 `Set` → `def_use` ✓。
4. **性能账（用户的生命线）**：`did_open` 三档一度 **+18~22%**，定位到 T-C25 的
   "上提到脊根"第一版让**每一层** `App` 祖先都 `render_expr` 一遍整棵子树
   （O(脊深) 次）。改成只让**最外层**记一次后，**背靠背**量折叠本身：
   unit01 +7.3% / unit08 +1.6% / unit12 **−0.05%** ⇒ **噪声内**；剩下的 +6~12%
   是**环境漂移**（同一 case 台账历史波动就有 ±7%，且关掉折叠仍在）。账写进
   `docs/PERF.md`。**教训**：显示层的"每层都做一遍"在声明上千的文件上是
   O(n·深度)，必须只做最外层。
5. **判据**：`display` **24 条** · `semantic` **26 条** · 新增 CLI 项目级测试
   （真 lib + 入口）· `gate` **PASS** · 课程计数**逐项不变**（36 目标 ·
   328 checked · 99 open · **0 判负**）· `gap.py check` 全绿。
   **未 bump**：本轮的 bump 点在 T-C41。
6. **已知剩余**（写进计划正文）：签名**自己的** binder 名（`forall (α : Type 0)
   (A B : Set α), …` 里的 `α`/`A`/`B`）仍是 `unknown_ident`——它们不在
   `DeclState.binders`（那是 goal 的 binder 列表），只存在于 `ty_text` 文本里。
7. **下一环**：T-C32（着色在 Infoview 里可见）→ T-C50（真宿主 e2e，矩阵用例 #6）
   → **T-C40/T-C41（⬆ BUMP patch）**。

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


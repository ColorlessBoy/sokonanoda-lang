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

## 本轮进度（2026-09-21，第一百三十一轮：**线 C 收口 + 0.65.1 + 排期提前 T-K20**）

> 用户最初六条反馈里的**第 5 条**（"infoview 里的 goal 展现没有用 notation 的方式"）
> 到此**整条闭环**：四个生产者 + 着色 + 真宿主 e2e + 检查点全过。

1. **T-C32 着色在 Infoview 里可见**：渲染侧本来就通（`infoview.js` 把 run 画成
   `tok-<kind>`），缺的是**测试**——补两条 webview 断言（目标行 + 声明卡片，
   记法符号各是一个 `tok-keyword` span）。扩展 `_pushState` 是
   `Object.assign({type:"state"}, state)` **全字段透传** ✓。
2. **T-C50 真宿主 e2e**：用例 #6 `goal text uses the file's notation` 本来就在
   （T-015..T-017 写的），本轮确认**转绿**（`--grep` → 1 passed；全量
   **23 passed / 2 failed**，剩的两条 #7/#8 是线 D）。
3. **T-C40 断言与 golden 更新**：不按计划给的行号审（行号早被挪走了），改成审
   `git diff 7874dd4..HEAD` 里测试文件的**每一条 golden 改动**——全程只重钉
   **5 处**，全是 `And` → `∧`，每处都先跑测试读实际输出再改；内核 pp
   **一个字节没改**（红线）；空断言扫描无命中。`cargo test --workspace --locked`
   → **exit 0**（39 suite，0 failed）。
4. **T-C41 文档 + CHANGELOG + ⬆ BUMP patch → 0.65.1**：`goal-rendering.md` §8
   as-built（四个生产者的最终行为 + "判定没动"的证据）、`notation-subset.md`
   补"渲染"一节（N1–N7 一条不变，只记显示侧的边界表）、CHANGELOG、
   REQUIREMENTS §9（第 5 条交付）、README、teacher 技能（**照面板念目标**）、
   `vscode-dev-guide.md` 两条坑。
5. **CP-C 检查点全过**：`verify-editor-issues.sh` → **已修 6 · 缺口仍在 1 ·
   环境异常 0**（第 5 条 **已修** ✓；剩的第 6 条 G-23 记法导航属线 D）·
   四生产者判别性全绿 · 课程计数**逐项不变**（36 目标 · 328 checked · 99 open ·
   **0 判负**）· `cargo test --workspace` 全绿 · `perf-compare --since c74c0046`
   **exit 0** · e2e #6 转绿。
6. **踩到的坑（已记）**：bump 之后**必须重建**——`scripts/soko` 要求仓库构建的
   版本与版本钉**匹配**，否则 exit 3，`verify-editor-issues.sh` 会把五条全报成
   「环境异常」（假红）。
7. **排期提前（用户拍板）**：线 C 的 4 条收完后**插 T-K20/T-K20′**（G-31 + G-34
   的根治设施），清单已把 `T-K20` 挪到线 D 之前（`plan.py check` 只校验集合、
   不校验顺序 ⇒ 合法）。依据：unit12 **冷编译 9.8s（release）**，其中一部分是
   judge 每批合成文档 + 整前缀重跑（实测 126k 次调用）。
8. **下一环**：**T-K20**（`docs/design/closure-incremental.md` + spike）。

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


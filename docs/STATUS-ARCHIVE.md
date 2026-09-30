# STATUS 归档（逐轮过程记录）

> **本文件是"旧轮的去处"** ✓（`AGENTS.md` §收尾义务 / `scripts/status-lint.py` 的口径 ✓）：
> `STATUS.md` 只留**最近 ≤3 轮**，更早的轮次进这里 ✓；这里再更早的进
> `docs/archive/`（gzip ✓，**归档 ≠ 销毁** ✓）。
>
> **已删（不归档，2026-09-30 激进删档）**：**第 1–104 轮**（2026-09-06 → 09-19）与更早的散段
> —— 原文 ⇒ `git log --all -- docs/STATUS-ARCHIVE.md` ✓。
> **本文件保留**：**最近 13 段**（第 494–519 轮，按**写入顺序**排，别当时间线读；**最老的段压缩成
> 结论 + git 指针** ⇒ 它是"合法的堆积"，但 **L1 层预算只许减不许增** ✓）。

## 第 519 轮（2026-09-30）：**T-N15 C 收尾 —— 「看得见的变化」索引 + 四处同步（批次 N 66/66）**

- **新增活文档 `docs/visible-changes.md`**（判据面）：每条 = **用户报的什么 → 屏幕上多/少什么 →
  哪一层的哪条断言咬住它**。覆盖用户报的 **6 条**（`build.tick` 刷屏 · Rebuild 恒 `0%` · Infoview
  版本戳陈旧 · Q1 `build.timeoutMs` · Q2 `installCli` · K1 改一行等很久）+ **隐式参数**（B2/B3）+
  记法铺面 A1–A5；⚠ **够不到的面不假装**（终端 stdout / 状态栏 / webview DOM 钉「载荷到达」那一层）。
- **登记与同步**：`REQUIREMENTS.md` §9.5 的 B3 行升级为 **B2/B3（T-N13/T-N14 收口）** + 指向新清单 ·
  `ONBOARDING.md` §0.2 队列改成「批次 N 最后 1 条」· VS Code + skills 同步（`README.md` 记法那条 ·
  `CHANGELOG.md` Unreleased · `teacher/SKILL.md` · `AGENTS.md` 硬规则 3）。
- **判据**：`docs-lint ✓` · `status-lint ✓` · `plan.py check` OK · `skill`+`dsh` 守卫 **8/0 + 5/0** ✓ ·
  课程门禁 **43/377/99/0** ✓ · `scripts/soko gate` **EXIT=0** ✓。
- **顺手修掉守卫自身的洞**（`no_course_signature_uses_an_implicit_binder`）：它原来对**带 import 的
  文件静默 `continue`**（空记法表下 `parse` 报 `NotationUnknownSymbol`）⇒ 迁移过的 4 个库里只看得到
  `lib/Set` 的 27 条 ✗。修法 = 带**继承记法表**解析（逐文件剔自声明符号）+ **解析失败也记 offender**
  ⇒ 立刻抓到 **37 条**并进白名单（**棘轮**：加签名要改清单，评审可见 ✓）。
- **端到端那条 `cases` 判据撞判定缝** ⇒ 立台账 **G-71**（`open` + **自包含复现件**，判红 = 缺口仍在 ✓，
  已进 `gap.py check` ✓）；判据主体移到真相层（夹具换纯 ∃ 形状）。
- **更正上一轮的一处误判** ✗：G-29/G-31 由 `fixed` **改回 `open`** —— 复现件的预算 `10×热开 + 200ms` 不稳定（热开 14ms ↔ 483ms）⇒ 同一天两次跑出**相反**结论；已换成**同 run 比值** `edit < 0.35 × cold`（连跑两次都判「仍在」✓），结构计数留作后续。
- **遗留**：判定缝没修（T-N13 的 4 条声明是换写法过的；pp 形态只丢**第一个**隐式实参 ⇒ 两种读法都
  不对，见 **G-71**）；点名叫法没全量迁移（仍有一批标记）；G-31 的复现件缺位。

## 第 518 轮（2026-09-30）：**T-N13 收口 —— B2 课程库改隐式风格（迁移本体 + 六条前端根因）**

- **契约**：`Set.image`/`Set.preimage` 一族 + `Function.comp`/`Rel.comp` 改隐式前导类型参数；
  调用点不必再写全 `α β`。三档 commit：`98c5804f`（库签名）→ `1d1ff0a9`（前端根因）→
  `9e07d6c7`（调用点收口 + 记账）。
- **① 库签名隐式化**：`lib/Set`（12 def + 1 abbrev + 1 axiom + 13 定理；`def Set` 保持显式）+
  `lib/Image` 6 条 + `Function.comp(_apply)` + `Rel.comp(_apply)` ⇒ 16 个 lib 逐个 **exit 0 / failed 0** ✓。
- **② 六条前端根因**（逐条带 front 判据 + **反向验证**）：① `unfold_one` 的实参对齐分短写/旧写法
  两种读法；② 零元记法在**函数位**（`(∅) x`）走唯一钩子；③ `unify_extract` 的**嵌套**实参位递归匹配；
  ④ `solve_prefix` 的**模板侧**展开（闸门：实际项剥到底的陪域必须是 Sort）；⑤ 路线③ 闸门补
  `starts_old_style`；⑥ 解出的隐式实参也吃该层的域当期望类型。
- **③ 调用点**：10 文件 / 19 诊断 ⇒ **0**；4 条声明按「项风格 + 显式前提」重写 —— `by` 块的 tactic
  路径在「pp 回读 + `Eq.subst`/`And.left`/`⊆`-展开」一族上会撞**判定缝**（两边 pp 一样而内核判不等）。
- **判据（逐项实测）**：课程门禁 **43 目标 · 377 checked · 99 open · 0 判负**（+1 = 新增的
  `diag_helper`）· `cargo test -p sokonanoda-front` **769/0** · 非课程语料对拍 **172 组 0 差异** ✓ ·
  `notation-lint` OK（豁免 587 处全带标记）· `gap.py check` 一致 ✓（**G-29/G-31 一度误改 `fixed`，
  第 519 轮已改回 `open`** —— 复现件的墙钟预算不稳定，两次跑出相反结论 ✗）。
- **遗留（写清楚）**：判定缝本身**没修**（4 条是换写法过的，缝的形状 = 「pp 回读 ⇒ 隐式实参/宇宙
  层级不同形」，与 E19/E20 同一片地，复现件留作 E19 取证）；点名叫法**没全量迁移**（仍有一批带
  标记的调用点，「数量级下降」只做到「不再必须写」）；G-31 的复现件缺位（详见 `docs/design/e2-plan.md`
  的 T-N13 as-built「遗留」三节）。

## 第 517 轮（2026-09-30）：**G-70 收口 —— 注册表签名必须「去记法」（+ 一个守卫自己的 bug）**

- **病根（B2 迁移实测）**：`KnownName::Decl::signature` 存的是 `render_expr(ty)`，而
  `implicit::telescope` 用 `parse_expr_text`（**空记法表**）把它**回读** ⇒ 签名里只要有一个
  记法节点（库定理常写成 `a ∈ univ`）解析就失败 ⇒ 望远镜 `None` ⇒ **隐式插入整条不触发** ✗
  ⇒ 实参落进隐式位（`exact Set.mem_univ x` ⇒ `期望 Set.univ α x，实际是 (a : x) -> Set.mem a Set.univ`）。
- **修法**：新增**判定路径**的去记法渲染 `proof::expand_notations` + 唯一入口
  `elab::decl_signature`（13 处 `signature:` 调用点全部改走它）⇒ 产物能被空记法表回读 ✓。
- **判据**：复现件 `G70-*.sh` **exit 1（已修）** ✓ · **反向验证** ✓（用未修的二进制跑同一件
  ⇒ exit 0 + 上面那条报错）· **未迁移**语料对拍 **644 组逐字节相同** ✓ · 课程门禁 **43/376/99/0** ✓。
- **顺带修掉一个守卫自身的 bug** ✗：`scripts/kernel-diff.sh` 的差异报告器写成 `"$file（exit …）"`
  —— bash 把全角括号并进变量名 ⇒ **恰好在「有差异」时**崩掉 ⇒ 改成 `${file}` ✓（修好后当场报出
  2/644 组差异，定位到本轮**语料**改动 ✓）。
- **⇒ B2 的三个前置（G-43 / G-69 / G-70）都已收口**；剩**调用点迁移本体**（分类表与顺序写进
  `docs/design/e2-plan.md` 的 **T-N13 as-built** ✓）。⚠ 迁移期间课程门禁会红 ⇒ **整段做完再一次过**。

## 第 516 轮（2026-09-30）：**G-69 收口 —— 归一化护栏补「逐位递归」（T-N13 的第二个前置）**

- **病根**：`by.rs::keep_if_lossless` 只比**顶层** spine 实参个数，而记法节点在 `spine_of` 里
  算 **1 个** ⇒ `Set.subset (A ∩ B) A` 与 pp 形态 `Set.subset (Set.inter A B) A` **顶层相等**、
  护栏放行 ✗ —— 可 pp 已把内层 `Set.inter` 的隐式 `α` 省掉 ⇒ `intro` 派生的假设成了**丢了参数的
  点形式** ⇒ `unfold_one` 把 `Set.inter A B x` 对成 `α := A, A := B, B := x` ✗。
- **修法**：护栏补**逐位递归**（新 `notation_positions_keep_implicit_prefix`）—— 源级记法节点在
  pp 形态里必须把该目标的**前导隐式实参**写出来（实参个数 ≥ 操作数 + `implicit_prefix`）；
  `DefInfo` 因此新增 `implicit_prefix`（两个构造点与 `KnownName::Decl` 同源 ✓）。
- **判据**：`compile/tests.rs::a_derived_hypothesis_solves_implicit_arguments_like_a_written_binder`（**反向验证** ✓：让该函数 `return true` ⇒ 当场判红）· 复现件 `G69-*.sh` **exit 1** ✓ · 对照组仍绿 ✓。
- **红线（逐项实测）**：全语料对拍 **644 组逐字节相同** ✓（今天全部记法目标 `implicit_prefix`
  都是 0 ⇒ 新判据**一次都不触发** ✓）· 课程门禁 **43/376/99/0** ✓ · front **796/0** · 记法契约 **50/0** ✓。
- **⇒ T-N13 的两个前置（G-43 / G-69）都已收口**；剩**迁移本体**（9 文件 / 19 诊断，逐站点改）。

## 第 515 轮（2026-09-30）：**G-43 收口 —— lambda 实参按「书写类型」求解（T-N13 的前置）**

- **病根（探针实测）**：**显示与判定共用一条 pp 文本** —— 内核 pp 会**丢掉第一个隐式实参**
  （`Set.image α β f A` 打成 `Set.image β f A`；后续隐式实参却留着：`unfold_apps_pp` 的
  `is_implicit_fun(fun)` **只认裸 `Const`**），而 `operand_type_expr` 把这份文本**回读成项**当
  "实参的类型" ⇒ 解出 `α := β` ✗ ⇒ `期望 Sort(1)，实际是 Pi ( : $4), $4`（**与 `import` 无关** ✓）。
- **修法（front 一处，内核零改动）**：新增 `lambda_source_type` —— lambda 的类型由**源级 binder
  注解**拼出来（`fun (h : P) => h` : `P → P`），与既有「`Ident` 取 `scope.source_type_of`」同一条
  **书写类型优先**规则 ⇒ 零内核调用、不被 pp 丢参污染 ✓；其余形状照旧问内核（不比从前差 ✓）。
- **判据**：`compile/tests.rs::a_lambda_argument_takes_its_written_type_not_the_lossy_pp_text`；**反向验证** ✓（中和成 `return None` ⇒ 当场判红）；复现件 `docs/gaps/repro/G43-*.sh` **exit 1（已修）** ✓。
- **红线（逐项实测）**：内核零改动 ✓ · 全语料对拍 **644 组逐字节相同** ✓ · 课程门禁 **43/376/99/0** 逐项相同 ✓ · front **795/0** · 记法契约 **50/0** ✓ · `gap.py check` ✓。
- **纪律教训**：本轮**两次**自造对照件被 bash 吃掉了 `''`（单引号里 `''` 会闭合引号）⇒ 量到的是 `f  A`（**另一个形状**）✗ ⇒ **对照件源码要 grep 出来核对** ✓。另：整文件批量改调用点**又一次证伪** ✗（649 处 ⇒ 16 文件判红）⇒ 只许逐站点核 ✓。

## 第 497 轮（2026-09-28）：**P1 复盘 —— 4.14× 重复编译**（G-68 + 测次数的守卫）

- 交付 **G-68** · `recompile-waste-retrospective-2026-09-28.md` · `rebuild-baseline-2026-09-28.md` ·
  复现件 · 新守卫 `scripts/check-recompile-factor.py`（**测次数不测耗时**，已进 gate + CI）。
  数字：rebuild **222.1s** · 判定 **151.2s = 68%** · **42 入口 / 174 次模块编译 = 4.14×**。
  没抓到的原因：`by_calls` 0.68.0→0.78.0 **全是 3**（不是回归）；`v0.60.0` = **4.11×**（第一天就错）；
  病根 = **每处结论都是散文，没有一条能判红**。⚠ "68%" 已被第 500 轮更正为 **88%**（嵌套累加）。

## 第 494 轮（2026-09-28）：**X2 多会话/多分支**（3/5 完成，交接 `docs/HANDOFF-0.78.md`）

- **用户拍板**：「**主解法改成 worktree，加锁只作兜底**」「**先只开一个 worktree 跑通全流程**……
  **跑通再谈并行**」「共享账本……**只有 main 能改台账**，并且**写成守卫**，别只写进文档」
  「**不许删任何已有会话/分支（要删先问我）**」「不许为跑通改判据」✓。
- **过时数据修正**（用户实测 + 本人复核）：PLAN 的 X2 段写「`target/` 205 GB、磁盘只剩 159 GB
  ⇒ 开 worktree 就爆盘」——**实测不成立** ✗：`target` = **32 GB**、`df -h /` 可用 **322 GiB** ✓
  ⇒ **独立 target 可行**（**别共享**：共享会被 cargo 加锁串行、抵消并行收益 ✗）；PLAN 本段已改 ✓。
- **§1 worktree 全流程实测 ✅**：建 → 改 → 提交 → 推**全通**；**推分支不取消 main 的 CI**
  （分支 run `36403617878` 与 main 的 `36403200616` **同期 in_progress** ✓）；**不触发 auto-tag** ✓。
  探针**已清**（本地 + 远端，远端只剩 `main` ✓）；**原有分支 `kernel/g56-indexed-inductives` 一个没动** ✓。
- **§2 共享台账守卫 ✅**（新 `scripts/check-multi-session.py`）：
  - **① 链接工作树里不许改台账**（worktree 判据 = `git rev-parse --git-dir` 含 `/worktrees/`）；
  - **② 台账与 main 不一致 ⇒ 判红**。⚠ **判据是"台账落后"而非"总提交数落后"** ——
    `e2e ledger` 每轮往 main 回提交，用总提交数判红会**每轮都红**（假红 ⇒ 守卫失效）；
  - ⚠ **文案必须分三情形**（**两轮实测才修对**）：工作区改 / 提交里改 / **只是落后**
    ⇒ **先判工作区、再判合并基**（顺序反过来会说错话）；**三种都实测判对** ✓；
  - **反向验证** ✓：`--selftest` 3 反例 + 真 worktree 三情形逐一实测 ✓。
- **§5 skill 清单 ✅**：`obra/superpowers` 加 `using-git-worktrees` + `finishing-a-development-branch`
  ⇒ 实测 **28 skills → .dsh/skills** ✓。
- **§4 假红：一条旧归因被推翻** ⚠：PLAN 写「本机 pre-push 有已知假红（**pyyaml 缺失**、
  版本钉二进制拿不到）」⇒ **实测**：`/opt/homebrew/bin/python3`（PATH 第一个）**有 yaml 6.0.3** ✓
  ⇒ **复现不出来**；且清理探针时 pre-push **完整跑过一次、全绿** ✓（`gap.py check --strict` ✓ ·
  `docs-lint --selftest` **11/11** ✓ · `editor` 43/43 ✓）。**未找到真机制 ⇒ 不改判据、不写错归因** ✓
  （下一步与候选机制见交接单 §4.2）。
- **⬜ 未完成**（交接单 §4）：e2e ledger 落后守卫（判据已想清）· 两个假红（候选机制未验证）✓。
- **门禁** ✓：`docs-lint` ✓（含 ⑦）· `plan.py check` ✓ · **内核零改动** ✓ · 工作区干净 ✓。

## 第 496 轮（2026-09-28）：**P0 纪律 + G-66 + G-67 + E30 回归 + 连推守卫**（HEAD `77617f1f`）

- **用户 18:11 / 18:24 / 18:47 / 19:00 / 19:26 五条指示全部落地** ✓；交接单
  `docs/HANDOFF-0.78.md`（312 行）已补 18:00 之后整段 ✓。
- **P0 纪律落 `main`**（`192404bd`）：`AGENTS.md` 验证设计纪律**第 0 条** —— **(a) 判据绑「用户动作」**
  （事故 E27：测试用**使用处** `∈` 能跳、用户点**声明名**返回 `null`）· **(b) 同类问题横向排查** ✓；
  PLAN 的 **E27 改如实状态**（"标 ✅ 是骗人的"）✓；台账 **G-65/G-66/G-67** ✓。
- **G-66 横向排查 7 处双重压暗**（`6d0af61c`）：`.section-title` **0.8em+0.7 ⇒ 1em、无 opacity**、
  删 `uppercase`/`letter-spacing`（对中文无益）✓；另 6 处只删 opacity ✓；**新守卫
  `check-infoview-hierarchy.py` 进 `soko gate`**（9 反例 / 5 必须判红）✓。
- **P0-b 点声明名不再弹空**（`d4da5359`）：服务端答不出 ⇒ `revealRange` 到**声明自身**
  （理由：声明名就是那个名字的定义所在处 ✓）；**符号点不动时仍如实弹提示** ✓；
  判据**绑用户动作**（编辑器真动 + `__messages` 空 + 降级前仍问服务端）· **反向验证 43/44 → 44/44** ✓。
- **G-67 项目区块重设计**（`7eb93699`）：三问（**编完了吗 / 哪个版本 / 有没有问题**）放最前；
  判据 `test-project-first-screen.js`（跑**真的** `infoview.js`，取**未收起**文本）·
  **反向验证旧版 5 条判红** ✓。
- ⚠ **E30 回归（19:26 判红）已修**（`612c6a39` + `77617f1f`）：G-67 撞掉 E30 **三条既有判据**
  （删了 `.project-warning` 独立元素 / 模块列表包进 `<details>` / facts·counts·artifacts 折进「高级」）✗
  ⇒ **教训：「重设计」≠「删掉既有交付」** ✓；修法 = **E30 元素全部原样 + 三问摘要加在最前** ✓
  （`requires_warning` 人话在前 + 原始细节在后；`编译 N` → `已编译 N/M`）；
  **没回滚 G-67、没放松 E30、没改测试** ✓。
- **连推守卫上线**（`77617f1f`）：**`scripts/check-one-run.py` 接进 `pre-push`** ——
  已有未完成的 run ⇒ **拒绝推送** ✓（用户 19:00 红线：连推三次 ⇒ HEAD 无绿证据 = 「本地绿就算绿」）✓；
  ⚠ 顺带实测：**空提交会得到「假绿」**（重活 skipped ⇒ `ci-green.py` exit 2）✗。
- **CI 真绿证据** ✓：**run `36417501766`（`77617f1f`）⇒ `ci-green.py` exit 0 ·
  重活 10/10 实跑且 success · failure 0** ✓（`editor` job = success ⇒ E30 用例过 ✓）。
- **门禁** ✓：`soko gate` exit 0 · `test-webview.js` **23/23** · `test-extension-host.js` **44/44** ·
  `docs-lint` ✓ · `gap.py check` 一致 · **`git diff crates/` 为空** ✓。
- **下一棒 = P1（rebuild 慢）**：**先确认用户说的 "rebuild" 是哪个命令**（别猜）⇒ 再量化 ⇒ 拿到数字再定改法 ✓。

## 第 498 轮（2026-09-28）：**P2 进度粒度** + **P1′ 方案与卡点**（等内核授权）

- **P2 ✅（用户 22:38/22:40 的判据）**：最小粒度从「文件」细到「**声明**」+ **心跳** ——
  CLI `build --json` 新增 `build.decl`（每命令一拍）与 `build.tick`（≤1/s，**只报已用时、不假装百分比**）。
  **实测（冷编 `courses/set-theory`，42 文件）：最长无输出间隔 39.0s → 1.98s**（事件 42 → **1727**）；
  扩展三处（状态栏/Infoview/概览尺）**渲染** + stub 判据 **45/45** ✓；`docs/protocol.md` 同步（+9 行，预算记账）。
  **判据进 gate + CI**：`scripts/check-progress-gap.py`（≤2.5s；`--selftest` **造 5s 空档必须判红** ✓）。
- **P1′ 方案文档** ✓：`docs/design/module-artifacts.md`（82 行）—— 产物三块（**内核环境** / 前端表 / 报告）·
  per-module Merkle 键 · 失效与回滚 · **要动的文件清单** · 两刀（进程内 fork → 磁盘 `.olean` 式，后者 = P3）。
- **P1′ 前端半已落** ✓：`ProjectPlan::module_keys()` + 性质判据 —— **无关模块变 ⇒ 别的模块键逐字节不变**
  （复用的收益）· **依赖变 ⇒ 下游键必变**（不许错编的红线）。判据 1 passed ✓。
- ⚠ **P1′ 卡点（已按规矩停报）**：切片的"加载产物"需要 importer **已持有依赖的内核环境**，
  而 `EnvBuilder` 无 `Clone`（`crates/kernel/src/builder.rs:26`）、`snapshot()` 只读（`:75`）、
  T-K12c 死因 = 独立环境 `add_declar` 改写共享 `decl_idx` 槽位 ⇒ 需**新增 `EnvBuilder::fork()`**
  （纯能力、不改判定路径）⇒ **内核改动，等授权**（`kernel/*` 分支 vs main 白名单）。**未硬推** ✓。
- **批编复查（同口径冷跑，已停）**：真课程切片 2 单元 **6,026ms vs 970ms（6.2× 慢）** · 4 单元 1.10× · 8 单元 1.00×
  ⇒ **平坦批编在真实形状上从不快**（合成小单元才 1.98× 快）⇒ 结论：**共享 ≠ 合并环境**，正解是**结果复用**。
- **门禁** ✓：`soko gate` exit 0 · `docs-lint` ✓（L2 随**新增文件**走一次）· `status-lint` ✓ · 内核零改动 ✓。

## 第 499 轮（2026-09-28）：**P2 进度粒度落地** + **P1′ 收益估算实测**（切片 1 先决条件已合入）

- **P2 ✅ 已落地**：`build --json` 新增 `build.decl`（每条命令一拍）+ `build.tick`（≤1/s 心跳，
  **只报已用时**）。冷编 42 文件：**最长无输出间隔 39.0s → 1.98s**，事件 **42 → 1727**；
  A/B 证明**无编译开销**（128.21s vs 124.34s）。判据 `scripts/check-progress-gap.py`
  （≤2.5s + `--selftest` 反向验证）已进 gate 与 CI；扩展三处渲染 + stub **45/45** ✓。
- **P1′ 设计**（`docs/design/module-artifacts.md`）：产物三块 · per-module Merkle 键 ·
  **§8.6 业界对照**（Lean/Lake/mathlib/Coq/rustc/Salsa，带出处 + 我们缺的 4 条）·
  **§9 做法改写**：跨 builder 播种**已被证否**（`EnvBuilder::new` 每次 `Dag::new_local`
  ⇒ `decl_idx` 槽位随 `NameNode` 走，旧表指针取不到）⇒ 改**一个 session arena + 一个 builder
  贯穿全场**（库层编一次 + 检查点 → 每入口 restore → 只走自己的命令 → `hide_declars`）。
- **切片 1a ✅ 已合入并复核**：`run_pass_in<'a>(arena, …)`（arena 提到调用方；行为零变化：
  fmt ✓ · clippy 无 error · front **757 passed** · CLI imports **21 passed**）。
- **收益估算（实测，切片 1 的输入）**：依赖占比 —— `unit08` **≈10%**（54.35/48.35s vs 5.08/5.06s）·
  `unit12` **≈25%** · `unit05` **≈50%** ⇒ **计数 174 → 42（4.14× → 1×），但墙钟只省 ≈10–25%**
  （课程总墙钟由重解答主导）。量具 `scripts/measure-rebuild.sh` **已入库**（原先在 `/tmp`，不可复现 ✗）。
- **未做（切片 1b/2/3/4）**：session 实现与 ① 逐字节等价判据 · 反例（改依赖必 miss）·
  产物落盘 · 可下载。**内核零改动** ✓。

## 第 505 轮（2026-09-30）：**开工单纠错 + 三条用户实测 UI 缺陷全部收口**
- 用户 09-30：「设定好goal，P1-d 等等后续都可以继续做的，**ONBOARDING.md 没写好**」。
- **① 开工单纠错**（`9372f798`）：合并 `NEXT.md` 时把「前缀环境」那条腿写成了 `P1-c / P1-d` ✗
  —— 取回**权威定义**：**P1-c = 收益兑现（双数字）· P1-d = 打包 + bump**，「前缀环境」
  **单列**为另一条腿。清掉 6 处指向已删 `docs/NEXT.md` 的悬空引用，补全 Q1/Q2/E19 与 §3.D。
  **新增咬得住的守卫**：`skill.rs` 只扫 `skills/`，DSH 薄入口里那处已删的 `docs/HANDOVER.md`
  **三层判据全绿** ✗ ⇒ 补 `dsh_skill_entry_referenced_repo_paths_exist`（反向验证判红 ✓）。
- **② ③ Infoview 两个"版本"**（`3e24ad24`）：**判死** —— 项目区块那个数既不是服务器版本、
  也不是清单 `requires`，而是 `<模块根>/.sokonanoda/meta.json` 的 `compiler` **产物戳**，
  且**只在建目录时写一次** ⇒ 升级后永远陈旧 ✗。修**两层**：`update_index` 每次写产物刷新戳
  （真相；作废靠键，刷新不影响缓存正确性）+ 屏幕上给裸版本号补标签（`由编译器 X 写入`、
  `server` → `服务器`）。判据两层都**反向验证过**。顺带修两个**空转**：
  `test-project-first-screen.js` 从来没被任何地方跑过（已进 `test:unit`）；它的 DOM shim
  缺 `firstChild`/`removeChild` ⇒ `clear()` 空转、判出来的"第一屏"混着**用户看不到的骨架行** ✗。
- **③ ① 终端每秒刷 `build.tick`**（`aecf2804`）：**实测** 2850 条事件里 **159 条 tick 的
  `file` 全是空串**、且**全在首条 `build.decl` 之前** ⇒ 「`file` 空不发」= **把心跳整个删掉**、
  「周期 5s」只让它慢一点；**真根因 = 心跳写 stdout，人看的终端与管道消费者共用同一个 stdout**
  ⇒ **默认不发**（`SOKO_BUILD_TICK_MS` 显式要）。⚠ 第一版判据**空转**（夹具 <1s ⇒ 连旧版
  也是 0 条），换 6.0s 夹具 + 自检后判红 ✓。
- **④ ② Rebuild 0% 突跳**（`eee2d9ed`）：**真宿主复现**（e2e 采样面板当前帧：`percent` 取值
  集合 `[null, 0, 100]`）⇒ 根因 = `build.file` 要等**全部**文件编完才按 `files` 顺序重放
  （首条 `build.decl` 在 **160.8s** 后）⇒ 屏幕从头到尾一个 `0%`。修法 = **additive 的
  `build.progress`**（每编完一个文件就报；提前发 `build.file` 会破 `--json` 确定性红线 ✗）。
  实测 42 条、跨度 **4.15s → 160.47s** ✓；**修前判红 → 修后判绿**，真宿主全量 **36/36** ✓。
- **⑤ P1-b 第二刀（`by` 路径）第 1 步：接线已试、判定分叉 ⇒ 精确回退**（`16c860f4`）：
  就地兄弟**走到了**（`used=213`）但判定与慢路不一致（`--json` 38 行不同）⇒ 触红线
  ⇒ **五个文件精确回退、零残留**；根因与下一轮切口见**附十**与「未决项」——
  **下一刀 = 按附十重做 `by` 第 1 步**（三条 UI 缺陷已全部收口，无遗留）。

## 第 500 轮（2026-09-29）：**剖面链路 + perf-gate 判红 + 入口级并行**（HEAD `afc82709`）
- **用户 09:12/09:20/09:22/10:06 四条**全部落地；**内核零改动** ✓ · 工作区干净 ✓。
- **① 剖面链路**（`11cd7272`）：`scripts/profile-course.py`（一条命令 · 四段 · JSON 进
  `docs/perf/`，schema `soko.course-profile/1`，**`profile` 字段在输出里**）· 埋点
  `SOKO_DECL_PROFILE`（逐声明事件，阈值仿 `trace.profiler.threshold`）+ `SOKO_NO_JUDGE`
  （**测量专用**，注释写明绝不许进判定路径）。**第一份数**：**219.3s**（方差 2.7%）·
  `passes` 4126 · 42/0 ✓；**judge ≈ 192s ≈ 88%**（两个 judge 入口都跳 ⇒ **26.8s** / 387 趟）。
  ⚠ **更正旧说法**：`judge_ms` 148.4s 是**嵌套累加** ⇒ "68%"低估；**大头是 `judge_infer`**
  （只跳 `by` 判定只有 **3%**）。
- **假设 A/B/C**：**A 成立**（41 模块 20 个 r>0.5、0 个 <−0.5、均值 **+0.44**）· **B 成立**
  （Top10 占 18.3%，都在后段 ⇒ 与 A 同病）· **C 不成立**（0.64s/入口）。机理 = `judge_infer`
  缓存键含**整段前缀哈希**（`judge.rs:932`）⇒ 后段全 miss ⇒ 前缀从零重跑；judge 合成
  **253513** 次 pass = 自身声明事件的 **95.8×**。
- **② perf-gate 撤"只报不拦"**（`0780f7f4`）：**三层同时失效** —— `continue-on-error` +
  每 case `|| true` + **台账只有 Darwin 记录**（CI 是 ubuntu）⇒ 按宿主过滤后**每条都
  "（无基线）"** ⇒ **撤开关也抓不到东西**（守卫空转）。修法：基线换**同 runner 家族的
  Actions cache**（`SOKO_PERF_LEDGER`，仅 `main` 更新）· 撤两处 · 阈值 50%。**反向验证：
  伪造 10× 快基线 ⇒ `exit 1`** ✓。新纪律进 `AGENTS.md`（435/435 行，**一行没涨**）。
- **③ 入口级并行**（`afc82709`）：`thread::scope` + 原子队列 · worker 自攒结果 · **输出仍在
  主线程按 `files` 顺序重放** ⇒ 确定性部分**逐字节相同** ✓（⚠ `build.tick` 215/107 —— 它带
  `elapsed_ms`，本来就与墙钟相关）。**A/B**：1 job **215.3s** → 4 jobs
  **108.7s**（**1.98×**）→ 8 jobs 108.8s → 10 jobs 125.9s。**核秒**：214.3s → **404.9s** →
  **709.7s** ⇒ **并行只省墙钟不省总功**（judge 两张全局 `Mutex` 竞争）⇒ **4 jobs 是拐点**
  ⇒ **瓶颈是单线程算法（judge 的 O(N²)），不是核数** ✓。
- **下一刀 = 架构重改**（用户 10:49 拍板，**彻底版**）：环境**可增量扩展**，judge 查常量类型变
  **查表**。⚠ **反直觉实测**：前缀复用（`SOKO_JUDGE_ENV_REUSE`）**已实现过**，收益 **< 0.3%**
  （LSP 12.8 万次判卷、命中率 99.6%）⇒ 按"收益不成立就默认关"关掉了 —— **命中率 99.6%
  ⇒ 复用只作用在已经很便宜的路径上**，**88% 全在 miss 上**。四阶段：**0 定接口 → 1 judge
  走环境 → 2 elaboration 主路径 → 3 并行下复用**；每阶段独立 commit + 独立真绿 + 独立回退。

## 第 501 轮（2026-09-29）：**分片否决 · 切片 1 三次失败全勘明 · 主线转 judge 前缀增量**（HEAD `620a0a4b`）

> **原文 ⇒ `git log --all -- docs/STATUS-ARCHIVE.md`**（2026-09-30 压缩：本段已超出"最近 12 段"，
> 只留结论）。**结论**：① 分片实测否决（单片 317.71s ≈ 全量 313.78s；课程产物缓存 628× 保留）；
> ② 切片 1 三次接线全失败 ⇒ **撤线挂起**（零件留 main）；③ 主线转**judge 前缀增量**——权威读数
> `JUDGE_PREFIX runs=3759` / `bytes=1.74 亿` · `misses 3759` ⇒ 未命中 ≈ **90%** 判定耗时，
> 靶子 = **那 3759 趟前缀重跑本身**。⚠ 当时的两份交接件（`HANDOVER-slice1.md` /
> `design/p1a-measurements.md`）已在 2026-09-30 文档清理里删掉 ⇒ **原文只在本文件的 git 历史里**
> （`git log --all -- docs/STATUS-ARCHIVE.md docs/design/p1a-measurements.md`）✓。

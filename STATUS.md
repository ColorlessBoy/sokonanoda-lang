# 当前快照（2026-09-30 · 第 519 轮）

- **🚀 `v0.78.3` = Latest** ✓（CI `36655613663` **28 success / 0 failure**（1 skipped）· release
  workflow `36656518982` success · tag `v0.78.3` · 26 资产 = 8 CLI + 8 LSP + 9 VSIX + 1 源码）。
- **P 组（编译提速）全档收口** ✓：`JUDGE_PREFIX runs` **3759 → 791**（−79%）· 全课 `build` 墙钟
  **216.14s → 47.8s**（**2.65×**）· `judge_ms` **−83%** · `--json` **0 行不同** · 影子档 `diff=0`。
  读数/开关/回退 ⇒ `docs/ONBOARDING.md` §1。
- **三条用户实测 UI 缺陷 + Q1/Q2 + K1 线（按前缀复用）已收口** ✓（第 505/506/509 轮；逐条索引 ⇒
  `docs/visible-changes.md`）。
- **计划与队列的唯一入口 = `docs/ONBOARDING.md` §0.2** ✓（2026-09-30 第 510 轮文档收敛：
  本文件只做**当前快照 + 最近 3 轮 + 未决项**，`ROADMAP.md` §10 只做**验收口径** ——
  三处不再各写一份"下一步"；ONBOARDING 257 → 180 行 · ROADMAP 706 → 480 行）。
- **本文件的判据**：`python3 scripts/status-lint.py`（≤240 行 · 禁词 0 · 每段 ≤30 · 净增 ≤60）·
  文档预算 `python3 scripts/docs-lint.py`（判据 ①–⑦，已进 `scripts/soko gate` 与 CI）。

- **文档过期日期机制**（第 513 轮，原文 ⇒ `git log --all -- STATUS.md`）✓：**每个活文档都有过期日期**
  （权威 = `scripts/docs-expiry.json`）· `git commit` 前**自动检测**（已过期/未登记 ⇒ **拒绝提交**）。
- **批次 N 进度 66/66**（第 519 轮）✓：**T-N13**（B2 课程库改隐式风格，第 518 轮）与
  **T-N15**（C 收尾，本轮）都收口 —— 「看得见的变化」逐条索引（用户报的 6 条 + 隐式参数 +
  断言名）⇒ **`docs/visible-changes.md`**（新活文档，已登记过期日 ✓）。

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

## 未决项（**只有这两条**；顺序与入口见 `docs/ONBOARDING.md` §0.2）

- ✅ **批次 N 全档收口（66/66）**：T-N13（第 518 轮）· T-N15（第 519 轮）✓ —— 逐条索引
  `docs/visible-changes.md`，遗留见 `docs/design/e2-plan.md` 的 T-N13 as-built「遗留」。
- ⬜ **E19 甲案 = `v0.79.0`**（高风险，**单独发版**）· E20 乙案：给记法求解器加**元变量**。
  **评估已写** ⇒ `docs/design/e19-evaluation.md`（收益窄、成本宽；**建议先冻结基线、先做乙案**）。
  ⚠ 开工前**重新冻结基线**（缺口根因见 `docs/design/v077-kernel-deficiencies.md` §三）。

## 硬事实（接手先读这 6 条 ✓）

- **硬规则**：内核可改，唯一红线是**判定正确性不变** ✓（`REQUIREMENTS.md` §2 · `docs/architecture.md` §6/§8）
- **批次制**：一个批次**只 push 一次** ✓；**等流水线不轮询** ✓（`gh run watch <id> --exit-status`）· **性能门禁只跟同宿主比** ✓（`AGENTS.md` §CI 节奏 / §性能回归门禁）
- **文档预算** ✓：`docs-lint` 判据 ①–⑧（**常量在脚本里**，别抄旧数字 ✗）——**接手成本**是判据 ⑦ 的**会判红的数字**（`docs-budget.json` 的 `onboarding`，上限**只许收紧** ✓）；**本文件**另受 `status-lint` 约束（≤240 行 · 净增 ≤60 · **只留最近 3 轮**，旧轮 ⇒ `docs/STATUS-ARCHIVE.md` ⇒ `.gz` ✓）

---

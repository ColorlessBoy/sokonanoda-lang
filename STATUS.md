# 当前快照（2026-10-01 · 第 524 轮）

- **🚀 `v0.79.0` = Latest** ✓（CI `36803030466` **28 success / 0 failure**（1 skipped）· release
  workflow `36803986833` success · tag `v0.79.0` · 26 资产 = 8 CLI + 8 LSP + 9 VSIX + 1 SHA256SUMS）。
- **E19 甲案收口** ✓：三刀 + **默认开**（用户 2026-10-01 拍板：默认开 · 开关**保留为逃生门**
  `SOKO_NOTATION_METAVAR=0`/`off`）⇒ `docs/design/e19-baseline.md` **§9 as-built**（两态摘要 · 交叉验证 ·
  既有判据逐条重审）；台账 **G-48 ⇒ `fixed` / `fixed_in: 0.79.0`** ✓。
- **P 组（编译提速）全档收口** ✓：`JUDGE_PREFIX runs` **3759 → 791**（−79%）· 全课 `build` **216.14s → 47.8s**
  （**2.65×**）· `judge_ms` **−83%** · `--json` **0 行不同** · 影子档 `diff=0`；读数/开关/回退 ⇒ `ONBOARDING.md` §1。
- **三条用户实测 UI 缺陷 + Q1/Q2 + K1 线（按前缀复用）已收口** ✓（第 505/506/509 轮；逐条索引 ⇒ `docs/visible-changes.md`）。
- **计划与队列的唯一入口 = `docs/ONBOARDING.md` §0.2** ✓（**队列 #1 = IA-4 元参数引擎**：
  **设计已出**（`docs/design/metavar-engine.md`）· **实现未开始** ⇒ 开工前先收 **D1**（范围）✓）。
- **本文件的判据**：`scripts/status-lint.py`（≤240 行 · 禁词 0 · 每段 ≤30 · 净增 ≤60）+ 文档预算
  `scripts/docs-lint.py`（判据 ①–⑦，已进 `scripts/soko gate` 与 CI）。

- **批次 N 进度 66/66**（第 519 轮）✓：T-N13（第 518 轮）· T-N15（第 519 轮）⇒ 逐条索引 `docs/visible-changes.md`。
- **文档过期日期机制**（第 513 轮）✓：每个活文档都有过期日期（`scripts/docs-expiry.json`）· `git commit` 前自动检测 ⇒ 已过期/未登记**拒绝提交**。

## 第 537 轮（2026-10-01）：**集合论教材线开工 —— S-A 大纲重建 + I.5 章首个单元（含两条新缺口）**

- **S-A 收口** ✓：重建 `docs/design/set-theory-syllabus.md`（旧稿已随 64 份文件清理删除，而**五处活文档仍引用它**）—— 教材对照**逐条查过目录页**（Enderton 9 章 / Halmos 25 节）。诊断：**缺口不是缺库、是缺课**（ST8/10/12/13/14 的五个 lib 模块一个单元都没吃到）；卷 I 完整版 = **12 已落地 + 10 新单元**，而 I.10（秩/超限递归/Aleph）**是内核墙**（G-56/G-58）。
- **S-B 首片** ✓：`lib/Order`（42 条）+ 单元⑬（3 演示 + 11 练习）· 解答 0 open · 门禁 **46 目标 · 436 checked · 110 open · 0 判负** · `notation-lint` ✓ · `docs-lint` ✓。
- **两条新缺口（实测 + 最小复现）**：**G-72**（同一模块单文件绿、`import` 时红 —— λ 体里第二个绑定变量丢了；**假绿**，堵所有"返回函数值"的库定义）· **G-73**（定义头类型上三条 elaborator 边界）。绕法**全部收进库**，hint 只讲数学 ✓；**I.5 的两个必破题被 G-73 挡住，如实不排**（不改题绕开 —— 那会把语言缺口伪装成教学选择）。
- **台账**：`gap.py check` 只剩 **G-34 一条既存红**（HEAD 上就红、与本轮无关）；顺带把 G-07/G-44 复现件从"写死 4 章 12 单元"改成**与规模无关的形状判据**（合法加一章不该判红）。

## 第 539 轮（2026-10-01）：**集合论线 S-B 第三片 —— I.6 章开工：`lib/Ordinal` 重定形 + 单元⑮**

- **探针抓到一个真缺陷** ✓：ST8 的 `IsOrdinal α E x := IsTransitive α E x ∧ ETrans α E ∧ EWellFounded α E`
  里那条 **`ETrans α E`（E 全局传递）让整个谓词退化** —— 全局传递下 `IsTransitive α E x` 对**任何** x 都成立
  ⇒ `IsOrdinal α E x` 与 x **无关** ✗（ZF 里 ∈ 本来就不是全局传递的）。
- **重定形**（照 Isabelle `Ord`）：`IsOrdinal α E x := 传递集 x ∧ (∀y∈x, y 传递) ∧ 环境良基`
  ⇒ 教材核心定理**「序数的元素是序数」从不可证变成直接可证** ✓；`HasLeast`/`EWellFounded`/`ETrans`
  与 `lib/Order` 重复，已删；补取用子与智能构造子。**内容型引理一条都不进库**（L-05）。
- **单元⑮** 传递集与序数（Enderton §7.6 · Halmos §19）：3 演示 + **10 练习**（T×4 / L×3 / D×3）·
  解答 0 open · 新章 I.6。**两道必破题**把上面那个退化**证出来**（全局传递 ⇒ 传递集变空话 ·
  序数谓词只剩良基那一半）；另一道：**空关系下每个元素都是序数**（前两条真空成立）。
- **门禁** **50 目标 · 474 checked · 130 open · 0 判负** ✓ · `notation-lint` ✓ · `docs-lint` ✓ ·
  `--selftest` PASS ✓ · `gap.py check` 仍只剩 **G-34 一条既存红**。

## 第 538 轮（2026-10-01）：**集合论线 S-B 第二片 —— I.5 章收口（单元⑭ 良序与良基）**

- **交付** ✓：`lib/Order` 增至 **48 条**（+`hasMin_def`/`hasMin_intro`/`hasMin_elim`/`hasMin_witness`/`isWellFounded_intro`/`EmptyRelation`）· 单元⑭ 3 演示 + **10 练习**（T×4 / L×4 / D×2）· 解答 0 open · **I.5 章收口**（单元⑬+⑭）· 门禁 **48 目标 · 458 checked · 120 open · 0 判负** ✓ · `notation-lint` ✓ · `docs-lint` ✓。
- **旗舰题**：**良基 ⇒ 没有无穷下降链**（极小元作用在"序列的像"上，再沿 `m = f n` 把"排在前面"搬过去）—— 这是良基性在教材里的头号用途 ✓。另两条硬后果：良基 ⇒ 非自反 · 子关系保持良基。
- **必破**：**空关系良基却不自反**（两道反例题）⇒ 说明"良基"与"自反/非自反"互不包含。
- **G-73 的代价继续摊薄在库里**（取用子/智能构造子）；被形状 ③ 挡住的两个必破题**仍如实不排**。
- **记账**：明细进 `courses/set-theory/README.md` 的新节「逐单元 as-built」；大纲 §8 只留状态（它受 260 行预算约束，逐轮明细放那儿会一直判红）。**下一片 = 单元⑮（传递集与序数）**，开工前先探针决定怎么讲 `lib/Ordinal` 的"谓词式 + 抽象 `(α, E)`"。

## 第 535 · 536 轮（2026-10-01）：**已按「只留最近 3 轮」压缩** —— 结论 + git 指针

- **535（S2 步 3）**：`QueryDoc` 逐命令快照 + `EntryTrust` 前缀信任 ⇒ 改最后一条 theorem **2989ms → 314ms**；本轮自复测抓出**依赖指纹缺失**（改依赖后 `publish=0` ⇒ 编辑器留陈旧诊断）并修掉。**零回归**：LSP 170/0 · front 793 · 门禁 43/377/99/0。
- **536（S6）**：`TrustPlan.trusted_extra` 不连续信任集 + 依赖图脏传播 ⇒ 目标用例重查 **4/7/8 → 1/1/4**；真 LSP 改第一条 2105 → 1665ms。**原文 ⇒ `git log --all -- STATUS.md`**（那两段）。

## 未决项（**只有这两条**；顺序与入口见 `docs/ONBOARDING.md` §0.2）

- ✅ **E19 甲案 = `v0.79.0`**（2026-10-01 **收口**）：**默认开**（逃生门 `SOKO_NOTATION_METAVAR=0` 保留）
  + 发版 ✓ ⇒ as-built `docs/design/e19-baseline.md` **§9**；台账 G-48 ⇒ `fixed` / `fixed_in: 0.79.0` ✓。
  **后继 = IA-4 元参数引擎**（设计 ✓ ⇒ `docs/design/metavar-engine.md`；开工前先收 D1）✓。
- ✅ **批次 N 全档收口（66/66）**：T-N13（第 518 轮）· T-N15（第 519 轮）✓ —— 逐条索引
  `docs/visible-changes.md`，遗留见 `docs/design/e2-plan.md` 的 T-N13 as-built「遗留」。
- ⚠ **E20 乙案不做**（用户拍板）；缺口根因 ⇒ `docs/design/v077-kernel-deficiencies.md` §三。

## 硬事实（接手先读这 6 条 ✓）

- **硬规则**：内核可改，唯一红线是**判定正确性不变** ✓（`REQUIREMENTS.md` §2 · `docs/architecture.md` §6/§8）
- **批次制**：一个批次**只 push 一次** ✓；**等流水线不轮询** ✓（`gh run watch <id> --exit-status`）· **性能门禁只跟同宿主比** ✓（`AGENTS.md` §CI 节奏 / §性能回归门禁）
- **文档预算** ✓：`docs-lint` 判据 ①–⑧（**常量在脚本里**，别抄旧数字 ✗）——**接手成本**是判据 ⑦ 的**会判红的数字**（`docs-budget.json` 的 `onboarding`，上限**只许收紧** ✓）；**本文件**另受 `status-lint` 约束（≤240 行 · 净增 ≤60 · **只留最近 3 轮**，旧轮 ⇒ `docs/STATUS-ARCHIVE.md` ⇒ `.gz` ✓）

---

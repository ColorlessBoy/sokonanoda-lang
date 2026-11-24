# CONTENT-PLAN —— 知识点 × 题目覆盖计划（卷 I 集合论）

> 一句话：**知识点是知识点，题目是题目；题目饱和性覆盖知识点。**
> 本计划的数字全部来自 `coverage.py` 的实测输出（2026-11-22），
> 不是拍脑袋估计；开发一批 → 跑一次覆盖率 → 红转绿后才算完成。

## 0. 现在在哪儿（2026-11-22 实测）

| 指标 | 实测 | 说明 |
|---|---|---|
| 知识点（`data/knowledge-points.json`） | **54** 个 | I.1 17 / I.2 20 / I.3 15 / I.6 1 / I.7 1（prep） |
| 题库（`data/questions.json`） | **78** 题 | choice 72 / short 4 / explain 2；level L1=22 L2=41 L3=15 |
| 已挂 `kps` | **98** 引用 / 综合题 **18** 道 | 一题可挂多个 KP，≥2 个算综合题 |
| 覆盖状态 | **绿 5 / 黄 2 / 红 47**（饱和率 **9.3%**） | 红 = 某个 L1/L2/L3 档低于 `saturation.min` |
| 章节综合题 | **18 / 29**（目标 ≈ KP 数 × 0.5） | I.1 6/9 · I.2 7/10 · I.3 5/8 · I.6 0/1 · I.7 0/1 |
| 硬缺口（min 不足） | **88 题** | I.1 27 / I.2 32 / I.3 29（L1 缺 22 / L2 缺 34 / L3 缺 32） |
| 到 target 的缺口 | **99 题** | I.1 28 / I.2 36 / I.3 34 / I.7 1（补完 min 后仍欠的 buffer） |

### B1 已合入实测（2026-11-22 11:32）

B1 的 40 题（I.1）已由 Lead 合入主库：题库 **78 → 118 题**（I.1 28→68 / I.2 28 / I.3 22），
kps 引用 **98 → 145**，综合题 **18 → 25**；`validate.py` OK、`engine.py selftest` 全绿、
`build_quiz.py` 重生成 118 题。覆盖变化：

| 指标 | B1 前 | B1 后 |
|---|---|---|
| 全库 绿/黄/红 | 5 / 2 / 47 | **21 / 1 / 32** |
| 饱和率 | 9.3% | **38.9%** |
| I.1 绿/黄/红 | 1 / 1 / 15 | **17 / 0 / 0** ✅ |
| I.1 缺口 | 28 | **0** |
| I.1 综合题 | 6/9 | 13/9（超额，可回填 I.2/I.3 综合题） |

下一步：B2（I.2，48 题）按 `data/_staging-b2.json` 暂存 → `verify_batch.py --chapter I.2`
（目标 I.2 红点清零）→ 合入后同口径复验。

### B2 已合入实测（2026-11-22 11:41）

B2 的 48 题（I.2）已由 Lead 合入主库：题库 **118 → 166 题**（I.1 68 / I.2 28→76 / I.3 22），
kps 引用 **145 → 196**，综合题 **25 → 28**；`validate.py` OK、`engine.py selftest` 全绿、
`build_quiz.py` 166 题。覆盖变化：

| 指标 | B2 前 | B2 后 |
|---|---|---|
| 全库 绿/黄/红 | 21 / 1 / 32 | **39 / 1 / 14** |
| 饱和率 | 38.9% | **72.2%** |
| I.2 绿/黄/红 | 2 / 0 / 18 | **20 / 0 / 0** ✅ |
| I.2 缺口 | 36 | **0** |
| I.2 综合题 | 7/10 | 10/10 ✅ |
| 剩余红 | I.2 18 + I.3 12 + prep 2 | **I.3 12 + prep 2（缺口 34）** |

下一步：B3（I.3，48 题）→ `data/_staging-b3.json` → `verify_batch.py --chapter I.3`
（目标 I.3 红点清零；另需把 I.6/I.7 prep 的 L1/L2 补上）。

### B3 已合入实测（2026-11-22 11:52）

B3 的 48 题（I.3）已由 Lead 合入主库：题库 **166 → 214 题**（I.1 68 / I.2 76 / I.3 22→70），
kps 引用 **196 → 247**，综合题 **28 → 31**；`validate.py` OK、`engine.py selftest` 全绿、
`build_quiz.py` 214 题。覆盖变化：

| 指标 | B3 前 | B3 后 |
|---|---|---|
| 全库 绿/黄/红 | 39 / 1 / 14 | **52 / 0 / 2** |
| 饱和率 | 72.2% | **96.3%** |
| I.3 绿/黄/红 | 2 / 1 / 12 | **15 / 0 / 0** ✅ |
| I.3 缺口 | 34 | **0** |
| I.3 综合题 | 5/8 | 8/8 ✅ |
| 剩余红 | I.3 12 + prep 2 | **仅 I.6/I.7 两个 prep KP** |

**B4 收口计划**（content-author 执行，product-manager 复核）：
- `_staging-b4-choice.json`：4 道 prep choice（kp.prep.ordinal 补 L1；kp.prep.choice 补 L1/L2）
  → `verify_batch.py` 目标 **红 2→0**；
- `_staging-b4-open.json`：3 道 open 题（short/explain，rubric 自判）→ Lead 按 validate + rubric 复核；
- 合入后全库应 **54 绿 / 0 黄 / 0 红**（饱和率 100%）。

### B4 已合入实测（2026-11-22 12:03）— **阶段 1 完成**

B4（4 choice prep + 3 open）由 Lead 合入：题库 **214 → 221 题**（I.1 69 / I.2 77 / I.3 75；
choice 212 + short 6 + explain 3），kps 引用 **257**、综合题 **34**。post-merge 复核：

| 指标 | 最终 |
|---|---|
| 知识点 | **54 / 54 绿、0 黄、0 红（饱和率 100.0%，内容门禁 GREEN）** |
| I.1 / I.2 / I.3 | 17/17、20/20、15/15 全绿；I.6/I.7 prep 也转绿 |
| 综合题 | 34（目标 29，超额 5） |
| open 题 | 9 道（short 6 + explain 3） |
| validate / selftest / build_quiz | OK / 全绿 / 221 题 |

**阶段 1（I.1–I.3）✅ 完成**：题库从 78 → 221 题，覆盖从 5 绿/47 红（9.3%）
到 **54 绿 / 0 红（100%）**；B1–B4 每批均经 `verify_batch.py` 结构+source 逐行+覆盖预测验收。
后续阶段（I.4–I.13）按 §2 继续：先注册章级 KP，再按 `KP × 4.5 × 1.2` 估题分批开发。

按 kind 看（当前 54 个 KP）：

| kind | KP 数 | 绿 | 红 | 说明 |
|---|---|---|---|---|
| definition | 27 | 4 | 23 | 定义型最缺 L1（回忆/展开）与 L3（辨析） |
| technique | 13 | 0 | 11 | 技巧型最缺 L2（动手用一次） |
| concept | 9 | 1 | 8 | 概念对比/边界 |
| boundary | 3 | 0 | 3 | 反例/不可证边界（排中、选择公理） |
| prep | 2 | 0 | 2 | I.6/I.7 预备（当前题库 0 题挂载） |

完整 47 条红名单见最新 `reports/coverage-<时间>.json`（`kps` 数组，`status:"red"`）。

## 1. 估算：需要开发多少题

**建议公式**（Lead 给的口径）：

```
目标题量 = KP 数 × 每 KP 平均题数 + 章节综合题
         = KP 数 × 4（按 kind 平均 4 题/KP）
         + KP 数 × 0.5（章节综合题 ≈ KP 数 × 0.5）
         ≈ KP 数 × 4.5
开发量   = 目标题量 × 1.2（返工系数：写题→核验→改 rubric 的返工）
```

两种口径都算一遍（避免把 buffer 当硬缺口）：

| 口径 | 计算 | 目标题量 | ×1.2 后 | 现有 78 题 → 需新增 |
|---|---|---|---|---|
| 建议公式（kind 平均 4 题） | 54 × 4 + 27 | 243 | **292** | **≈ 214** |
| 注册表实配 target（`saturation.target` 之和 + 综合题） | 192 + 29 | 221 | **≈ 266** | **≈ 188** |

> **规划取中位数 ≈ 200 题**（当前已注册的 I.1–I.3 + prep）作为第一阶段开发目标；
> 其中 107 题是「红→绿」的硬需求（99 target 缺口 + 9 综合题缺口，取整）……
> 另 ~90 题是 L2/L3 强化与预留 buffer（`1.2` 返工系数 + 黄转绿的余量）。

### 全课程（I.1–I.13）外推

当前只注册了 I.1–I.3 + 两个 prep KP。按每章 15–18 个 KP 估算：

| 阶段 | 章节 | 预估 KP 数 | 目标题量（×4.5） | ×1.2 | 估计新增 |
|---|---|---|---|---|---|
| 1 | I.1–I.3（已注册） | 54 | 243 | 292 | **≈ 214** |
| 2 | I.4–I.6 | ≈ 48 | 216 | 259 | ≈ 260 |
| 3 | I.7–I.9 | ≈ 48 | 216 | 259 | ≈ 260 |
| 4 | I.10–I.13 | ≈ 64 | 288 | 346 | ≈ 346 |
| 合计 | I.1–I.13 | ≈ 214 | ≈ 963 | ≈ 1156 | **≈ 1080** |

全课程终态题库约 **1100–1200 题**；每个阶段开始时**先注册该阶段的 KP**
（kind/chapter/prereq/saturation 四件套），否则 `coverage.py` 无法给该阶段记账。

## 2. 分阶段计划（每批 40–60 题，一批一次验收）

| 批次 | 范围 | 目标题量 | 重点 | 验收判据（全过才算完成） |
|---|---|---|---|---|
| **B1 ✅** | **I.1 饱和** | **40**（本文件 §3 已列清单） | 27 个 min 缺口 + 3 综合题 + 10 强化 | ✅ 已合入：I.1 17 绿/0 黄/0 红；抽检通过 |
| **B2 ✅** | I.2 饱和 | 48 | 32 min 缺口 + 3 综合题 + 13 强化 | ✅ 已合入：I.2 20 绿/0 黄/0 红、综合 10/10 |
| **B3 ✅** | I.3 饱和 | 48 | 29 min 缺口 + 3 综合题 + 16 强化 | ✅ 已合入：I.3 15 绿/0 黄/0 红、综合 8/8 |
| B4 | I.1–I.3 收口 | 43 | 黄转绿、prep（I.6/I.7）补 L1/L2、综合题补到 27/29 | 三章全绿或黄 ≤2 且有记账；综合 27+ |
| **B4** | 收口（prep + open） | 7 | 4 prep choice（补 I.6/I.7 的 L1/L2）+ 3 open（short/explain，rubric） | 全库 54 绿 / 0 黄 / 0 红 |
| **阶段 1 ✅ 小计** | I.1–I.3 | **179** | ✅ 完成：221 题、54/54 绿、0 红、饱和率 100% | |
| B5–B10 | I.4–I.6（先注册 KP） | ≈ 260 | 论域/悖论/良序；先补 KP 注册表再出题 | 新章覆盖全绿 + 抽检 source |
| B11–B16 | I.7–I.9（先注册 KP） | ≈ 260 | 选择公理/ZF/基数算术/集族 | 同上 |
| B17–B24 | I.10–I.13（先注册 KP） | ≈ 346 | 闭包/等价关系/集合代数/幂集函数空间/关系代数 | 同上 |

**每批固定动作**（写进批次 checklist）：

1. 按清单写题（open 题同时写 `rubric` + `reference`，keywords 必须来自 `reference` 原词）；
2. 每题 `kps` 至少 1 个（综合题 ≥2），`source` 落**可打开核验的文件:行号**（新行号写题时同步补）；
3. `python3 validate.py`（kps 引用/章节归属/综合题结构）；
4. `python3 coverage.py`（本章零红；红则回到第 1 步）；
5. 抽检 ≥10% 题目的 `source` 行号真实存在且答案能由该处核验（禁止凭印象改答案）；
6. `python3 engine.py selftest`（引擎自检）+ 若有新 open 题：同一批用 `engine.py answer --text <rubric 关键词>` 自测判分。

**写不进库的先记账**：如果某题必须等新引理/新单元行号（例如 I.3 的 Cantor–Bernstein
需要 `lib/Choice` 的现成结论），在 §4「暂缓题记账」表登记（题号/KP/level/source 候选/
阻塞原因），**不许先编一个假 source 混过 validate**。

## 3. 第一批（B1）题目清单 —— I.1 饱和，40 题

生成规则：每个 I.1 红 KP 先补齐 `saturation.min` 的三档缺口，再补到 `target`；
不足 40 题的部分用「综合缺口（2 KP/题，补到 9 道）」与 L2/L3 强化填满。
`预期 source` 是**落笔时要打开核验的文件**（新行号落笔时补；表中先给文件+锚点）。

| # | 题号（预留） | KP | level | 用途 | 预期 source |
|---|---|---|---|---|---|
| 1 | `I.1.mem-3` | kp.i1.mem | L2 | min 缺口 | units/I.1/unit01-sets-membership.sokonanoda（`Set.mem` 记法） |
| 2 | `I.1.subset-5` | kp.i1.subset | L3 | min 缺口 | units/I.1/unit02-subsets-empty.sokonanoda（`Set.subset_def`） |
| 3 | `I.1.subset-apply-1` | kp.i1.subset.apply | L2 | min 缺口 | units/I.1/unit02-subsets-empty.sokonanoda + lib/Set.sokonanoda |
| 4 | `I.1.subset-apply-2` | kp.i1.subset.apply | L2 | min 缺口 | units/I.1/unit02-subsets-empty.sokonanoda + lib/Set.sokonanoda |
| 5 | `I.1.subset-apply-3` | kp.i1.subset.apply | L3 | min 缺口 | units/I.1/unit02-subsets-empty.sokonanoda + lib/Set.sokonanoda |
| 6 | `I.1.inter-2` | kp.i1.inter | L2 | min 缺口 | units/I.1/unit03-union-inter-powerset.sokonanoda |
| 7 | `I.1.inter-3` | kp.i1.inter | L3 | min 缺口 | units/I.1/unit03-union-inter-powerset.sokonanoda |
| 8 | `I.1.union-4` | kp.i1.union | L2 | min 缺口 | units/I.1/unit03-union-inter-powerset.sokonanoda |
| 9 | `I.1.union-5` | kp.i1.union | L3 | min 缺口 | units/I.1/unit03-union-inter-powerset.sokonanoda |
| 10 | `I.1.sdiff-3` | kp.i1.sdiff | L1 | min 缺口 | units/I.1/unit03-union-inter-powerset.sokonanoda + unit58-sdiff-algebra.sokonanoda |
| 11 | `I.1.sdiff-4` | kp.i1.sdiff | L3 | min 缺口 | units/I.1/unit03-union-inter-powerset.sokonanoda + unit58-sdiff-algebra.sokonanoda |
| 12 | `I.1.compl-2` | kp.i1.compl | L1 | min 缺口 | units/I.1/unit03-union-inter-powerset.sokonanoda + unit60-compl-algebra.sokonanoda |
| 13 | `I.1.compl-3` | kp.i1.compl | L3 | min 缺口 | units/I.1/unit03-union-inter-powerset.sokonanoda + unit60-compl-algebra.sokonanoda |
| 14 | `I.1.powerset-3` | kp.i1.powerset | L1 | min 缺口 | units/I.1/unit03-union-inter-powerset.sokonanoda |
| 15 | `I.1.singleton-4` | kp.i1.singleton | L1 | min 缺口 | units/I.1/unit01-sets-membership.sokonanoda（`Set.mem_singleton_iff`） |
| 16 | `I.1.singleton-5` | kp.i1.singleton | L3 | min 缺口 | units/I.1/unit01-sets-membership.sokonanoda（`Set.mem_singleton_iff`） |
| 17 | `I.1.univ-1` | kp.i1.univ | L1 | min 缺口 | units/I.1/unit03-union-inter-powerset.sokonanoda（`Set.univ`） |
| 18 | `I.1.univ-2` | kp.i1.univ | L3 | 补到 target | units/I.1/unit03-union-inter-powerset.sokonanoda（`Set.univ`） |
| 19 | `I.1.ext-5` | kp.i1.ext | L1 | min 缺口 | units/I.1/unit04-extensionality-identities.sokonanoda |
| 20 | `I.1.ext-use-1` | kp.i1.ext.use | L2 | 补到 target | units/I.1/unit04-extensionality-identities.sokonanoda + unit61-ext-packing.sokonanoda |
| 21 | `I.1.subset-algebra-1` | kp.i1.subset.algebra | L2 | min 缺口 | units/I.1/unit66-subset-algebra.sokonanoda |
| 22 | `I.1.subset-algebra-2` | kp.i1.subset.algebra | L3 | min 缺口 | units/I.1/unit66-subset-algebra.sokonanoda |
| 23 | `I.1.subset-algebra-3` | kp.i1.subset.algebra | L3 | 补到 target | units/I.1/unit66-subset-algebra.sokonanoda |
| 24 | `I.1.sdiff-algebra-1` | kp.i1.sdiff.algebra | L2 | min 缺口 | units/I.1/unit58-sdiff-algebra.sokonanoda / unit59 / unit63 / unit64 / unit65 |
| 25 | `I.1.sdiff-algebra-2` | kp.i1.sdiff.algebra | L2 | min 缺口 | units/I.1/unit58-sdiff-algebra.sokonanoda / unit59 / unit63 / unit64 / unit65 |
| 26 | `I.1.sdiff-algebra-3` | kp.i1.sdiff.algebra | L3 | min 缺口 | units/I.1/unit58-sdiff-algebra.sokonanoda / unit59 / unit63 / unit64 / unit65 |
| 27 | `I.1.sdiff-algebra-4` | kp.i1.sdiff.algebra | L3 | min 缺口 | units/I.1/unit58-sdiff-algebra.sokonanoda / unit59 / unit63 / unit64 / unit65 |
| 28 | `I.1.sdiff-algebra-5` | kp.i1.sdiff.algebra | L2 | 补到 target | units/I.1/unit58-sdiff-algebra.sokonanoda / unit59 / unit63 / unit64 / unit65 |
| 29 | `I.1.classical-1` | kp.i1.classical | L2 | min 缺口 | units/I.1/unit104-classical.sokonanoda / unit105-classical-laws.sokonanoda |
| 30 | `I.1.classical-2` | kp.i1.classical | L3 | min 缺口 | units/I.1/unit104-classical.sokonanoda / unit105-classical-laws.sokonanoda |
| 31 | `I.1.classical-3` | kp.i1.classical | L2 | 补到 target | units/I.1/unit104-classical.sokonanoda / unit105-classical-laws.sokonanoda |
| 32 | `I.1.logic-1` | kp.i1.logic | L3 | min 缺口 | lib/Logic.sokonanoda（And/Or 消去与构造） |
| 33 | `I.1.logic-2` | kp.i1.logic | L3 | 补到 target | lib/Logic.sokonanoda（And/Or 消去与构造） |
| 34 | `I.1.mix-1` | kp.i1.ext.use, kp.i1.sdiff.algebra | L2 | 综合缺口 | units/I.1/unit04-extensionality-identities.sokonanoda + unit61-ext-packing.sokonanoda + units/I.1/unit58-sdiff-algebra.sokonanoda / unit59 / unit63 / unit64 / unit65 |
| 35 | `I.1.mix-2` | kp.i1.powerset, kp.i1.inter | L3 | 综合缺口 | units/I.1/unit03-union-inter-powerset.sokonanoda |
| 36 | `I.1.mix-3` | kp.i1.classical, kp.i1.logic | L2 | 综合缺口 | units/I.1/unit104-classical.sokonanoda / unit105-classical-laws.sokonanoda + lib/Logic.sokonanoda（And/Or 消去与构造） |
| 37 | `I.1.mix-4` | kp.i1.union, kp.i1.sdiff | L3 | 强化 | units/I.1/unit03-union-inter-powerset.sokonanoda + units/I.1/unit03-union-inter-powerset.sokonanoda + unit58-sdiff-algebra.sokonanoda |
| 38 | `I.1.mix-5` | kp.i1.subset.apply, kp.i1.ext.use | L2 | 强化 | units/I.1/unit02-subsets-empty.sokonanoda + lib/Set.sokonanoda + units/I.1/unit04-extensionality-identities.sokonanoda + unit61-ext-packing.sokonanoda |
| 39 | `I.1.mix-6` | kp.i1.singleton, kp.i1.empty | L3 | 强化 | units/I.1/unit01-sets-membership.sokonanoda（`Set.mem_singleton_iff`） + units/I.1/unit02-subsets-empty.sokonanoda（`Set.notMem_empty`） |
| 40 | `I.1.mix-7` | kp.i1.compl, kp.i1.sdiff.algebra | L2 | 强化 | units/I.1/unit03-union-inter-powerset.sokonanoda + unit60-compl-algebra.sokonanoda + units/I.1/unit58-sdiff-algebra.sokonanoda / unit59 / unit63 / unit64 / unit65 |
> 题号规则：`I.1.<kp 短名>-<序号>` 接现有最大序号；综合题用 `I.1.mix-<n>`（`kps` 挂 2 个）。
> B1 完成后这 40 题应让 I.1 从 15 红 / 1 绿 → 0 红（黄可有，B4 收口）；
> 验收命令：`python3 coverage.py | grep -E "^  I\.1|内容门禁"`。

### 3.1 B2 期望缺口表（I.2，合入 B1 后实测）

B2 目标：**48 题**（32 个 min 缺口 + 3 综合题缺口 + 4 题到 target + 9 题 L2/L3 强化），
合入后 I.2 应 **0 红**。content-author 的覆盖表请对照本表（数字来自 `coverage.py`，
生成时间：B1 合入后）：

| KP | kind | 现有 L1/L2/L3 | 总数/target | min | 需补（按 level） |
|---|---|---|---|---|---|
| `kp.i2.fun` | definition | 0/0/0 | 0/4 | 1/1/1 | L1×1，L2×1，L3×1，补target×1 |
| `kp.i2.fun.inverse` | technique | 0/1/0 | 1/4 | 0/2/1 | L2×1，L3×1，补target×1 |
| `kp.i2.logic` | technique | 0/1/0 | 1/4 | 0/2/1 | L2×1，L3×1，补target×1 |
| `kp.i2.rel.antisymm` | definition | 0/1/1 | 2/5 | 1/2/1 | L1×1，L2×1，补target×1 |
| `kp.i2.rel.comp` | definition | 0/0/0 | 0/3 | 1/1/1 | L1×1，L2×1，L3×1 |
| `kp.i2.fun.boundary` | boundary | 0/0/1 | 1/3 | 0/1/1 | L2×1，补target×1 |
| `kp.i2.fun.comp` | technique | 1/1/0 | 2/4 | 0/2/1 | L2×1，L3×1 |
| `kp.i2.order.linear` | concept | 0/1/0 | 1/3 | 1/1/1 | L1×1，L3×1 |
| `kp.i2.order.partial` | concept | 0/2/1 | 3/5 | 1/2/1 | L1×1，补target×1 |
| `kp.i2.order.strict` | concept | 0/2/0 | 2/4 | 1/2/1 | L1×1，L3×1 |
| `kp.i2.pair` | definition | 0/1/0 | 1/3 | 1/1/1 | L1×1，L3×1 |
| `kp.i2.prod` | definition | 0/2/0 | 2/4 | 1/2/1 | L1×1，L3×1 |
| `kp.i2.rel.inv` | definition | 0/0/0 | 0/2 | 1/1/0 | L1×1，L2×1 |
| `kp.i2.fun.bij` | concept | 0/2/0 | 2/3 | 1/1/1 | L1×1，L3×1 |
| `kp.i2.fun.inj` | concept | 1/1/2 | 4/5 | 1/2/1 | L2×1 |
| `kp.i2.rel` | definition | 2/1/0 | 3/4 | 1/1/1 | L3×1 |
| `kp.i2.rel.trans` | definition | 1/0/0 | 1/2 | 1/1/0 | L2×1 |
| `kp.i2.rel.symm` | definition | 1/0/1 | 2/2 | 1/1/0 | L2×1 |
| `kp.i2.fun.surj` | concept | 1/2/2 | 5/5 | 1/2/1 | — |
| `kp.i2.rel.refl` | definition | 2/1/0 | 3/2 | 1/1/0 | — |

> 合计：min 缺口 **32**，到 target 缺口 **36**（另综合题 7/10 → 需补 3）。
> 若某 KP 的 `需补` 列与 content-author 报的不一致，以本表为准复核。

## 4. 暂缓题记账（暂无，落笔时填）

| 题号 | KP | level | 阻塞原因 | 需要谁/什么 | 状态 |
|---|---|---|---|---|---|
| — | — | — | — | — | — |

## 5. 工具与门禁

```bash
cd courses/set-theory/knowledge

# 1) 草稿验收（content-author 交 data/_staging-b1.json，裸 JSON 数组；不改仓库）
python3 verify_batch.py --batch data/_staging-b2.json --chapter I.2   # B2（B1 已完成）
#    → 结构/15 字段/id 撞号/kps 引用/source 行号逐条核验 + 内存注入预测目标章红黄绿

# 2) 合入后（由 Lead/指定人把数组 extend 进 data/questions.json）
python3 validate.py        # 数据结构 + kps 引用 + 逻辑校验（红则 exit 1）
python3 coverage.py        # 覆盖门禁：任一 KP 红 → exit 1，并写 reports/coverage-<时间>.json
python3 coverage.py --no-report   # 只看汇总
python3 coverage.py --json        # 机器可读（CI/agent 消费）
python3 engine.py selftest        # 引擎自检（改题库后必跑）
```

- **批次协作约定（B1 起）**：content-author 只写 `data/_staging-b1.json`（40 项裸数组，
  字段 `id/kind/chapter/unit/level/tags/difficulty/variants/prereq/prompt/options/answer/
  why/source/kps`）；product-manager 用 `verify_batch.py` 复核并出覆盖预测；合入
  `data/questions.json` 后重跑 `validate.py` + `coverage.py` + `engine.py selftest`。

- `coverage.py` 的「红」只严格卡 `saturation.min`；`--fail-on-yellow` 可把黄也当红（更严）。
- 生成的报告 `reports/coverage-<时间>.json` 是**可回溯的覆盖账**：每次合入一批就留一份。
- `kps` 的章节归属规则：KP 的 `chapter` 必须等于题目的 `chapter`；例外是 `kind:"prep"`
  （I.6/I.7 预备知识点允许挂在 I.3 等前置章节的题上）——`validate.py` 已按此校验。

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
后续阶段（I.4–I.13）已由 **C9.1（2026-11-25）注册 KP**：107 个新 KP + 2 个既有 prep，
区间合计 109；bank 目标按 `KP × 4.5` = 493 题（`×1.2` 只作排期返工，不重复计入 bank）。
逐章 KP × 目标题量 × 批次见 §2.1。

阶段 1 的 54 个 KP 按 kind 看（历史读数，2026-11-22）：

| kind | KP 数 | 绿 | 红 | 说明 |
|---|---|---|---|---|
| definition | 27 | 4 | 23 | 定义型最缺 L1（回忆/展开）与 L3（辨析） |
| technique | 13 | 0 | 11 | 技巧型最缺 L2（动手用一次） |
| concept | 9 | 1 | 8 | 概念对比/边界 |
| boundary | 3 | 0 | 3 | 反例/不可证边界（排中、选择公理） |
| prep | 2 | 0 | 2 | I.6/I.7 预备（当前题库 0 题挂载） |

阶段 1 的 47 条红名单见当时的 `reports/coverage-<时间>.json`（`kps` 数组，`status:"red"`）。

### 0.1 C9.1 注册实测（2026-11-25）

I.4–I.13 的 KP 已注册（`data/knowledge-points.json` v4）：**新增 107 / 区间 109
（含 2 个既有 prep）**，课程 KP 总数 **161**。逐章实测：

| 章 | 单元 | KP（含 prep） | 新增 | bank 目标 ⌈KP×4.5⌉ | min L1/L2/L3 | 综合题目标 | 批次（题数） |
|---|---|---|---|---|---|---|---|
| I.4 | 12 | 17 | 17 | 77 | 6/26/17 | 9 | B5 40 + B6 37 |
| I.5 | 4 | 7 | 7 | 32 | 5/8/7 | 4 | B7 32 |
| I.6 | 13 | 17 | 16（+1 prep） | 77 | 12/21/16 | 9 | B8 40 + B9 37 |
| I.7 | 7 | 11 | 10（+1 prep） | 50 | 7/13/11 | 6 | B10 50 |
| I.8 | 4 | 4 | 4 | 18 | 2/4/4 | 2 | B11 18 |
| I.9 | 3 | 4 | 4 | 18 | 3/4/4 | 2 | B12 18 |
| I.10 | 12 | 16 | 16 | 72 | 11/17/16 | 8 | B13 40 + B14 32 |
| I.11 | 7 | 11 | 11 | 50 | 7/14/11 | 6 | B15 50 |
| I.12 | 7 | 8 | 8 | 36 | 5/9/8 | 4 | B16 36 |
| I.13 | 8 | 14 | 14 | 63 | 11/15/14 | 7 | B17 40 + B18 23 |
| **合计** | **77** | **109** | **107 + 2 prep** | **493** | **69/131/108** | **57** | **14 批** |

new KP 的 kind 分布（107 个）：definition 35 · concept 31 · technique 32 ·
boundary 8 · integration 1。区间 109 个里另含 2 个既有 prep。

> **为什么是 109 而不是草稿的 115**：115 是「1.49 KP/单元 × 77」的外推；逐单元
> 审计合并了 6 个与既有 KP 重复的条目（I.8 u19/u54 复述 I.3 的 `≼`/`≈`；I.9 u22
> 复述 I.3 的像/原像分配律；I.10 u78 的往返已由 `kp.i10.partition.equiv` 承载；
> I.6 u16 的 `≤` 没有单元支撑；I.6 u83 的无环性并入 `kp.i6.antisymm`），并补了
> 3 个审计发现的缺项（`kp.i7.union`、`kp.i13.kernel`、`kp.i13.wf.boundary`）。
> 实测 1.42 KP/单元，仍在原计划的 1.42–1.75 区间；**没有为凑 115 保留重复 KP**
> （重复 KP 会让 coverage 的「题挂 KP」变成同题重复记账）。

**当前覆盖（`coverage.py`，2026-11-25）**：161 KP → **绿 54 / 黄 0 / 红 107**
（饱和率 33.5%），exit 1；I.4–I.13 的 107 个新 KP 均无题，**这是 C9.1 的预期红**，
报告逐条点名（`reports/coverage-<时间>.json` 的 `kps[status=red]`）。C9.2/C9.3 按
§2.1 的批次补题后转绿；**不许**用降低 `min`、虚增 `target` 或删 KP 伪造绿色。

## 1. 估算：需要开发多少题

**口径（2026-11-25 修正）**：**bank 目标 = KP 数 × 4.5**
（`4` = 每 KP 平均题数；`0.5` = 章级综合题，一题挂 ≥2 KP）。
`×1.2` 只是**排期返工系数**（写题→核验→改 rubric），**不重复计入 bank 规模**。

| 范围 | 唯一 KP | bank 目标 | 已写题 | 覆盖状态 |
|---|---|---|---|---|
| I.1–I.3 + prep | 54 | 243 | 227 | 54/54 绿（阶段 1，2026-11-22） |
| I.4–I.13（含同一批 2 个 prep） | 109 | 493 | 0 | 0/109 绿（C9.1 预期红） |
| 全课程去重 | 161 | ≈ 725（逐章取整 728） | 227 | 54 绿 / 107 红 |

> 阶段 1 的 227 题已满足其 saturation 口径（54 绿）；bank 口径的 243 只是规划上限，
> 不是硬缺口。全课程 bank 目标按 161×4.5 ≈ 725；逐章取整合计 728（2 个 prep 在
> 区间表里各计一次）。`×1.2` 后的排期工作量约 870 题当量，不改变 bank 规模。

## 2. 分阶段计划（每批 ≤50 题，一批一次验收）

| 批次 | 范围 | 目标题量 | 重点 | 验收判据（全过才算完成） |
|---|---|---|---|---|
| **B1 ✅** | **I.1 饱和** | **40**（本文件 §3 已列清单） | 27 个 min 缺口 + 3 综合题 + 10 强化 | ✅ 已合入：I.1 17 绿/0 黄/0 红；抽检通过 |
| **B2 ✅** | I.2 饱和 | 48 | 32 min 缺口 + 3 综合题 + 13 强化 | ✅ 已合入：I.2 20 绿/0 黄/0 红、综合 10/10 |
| **B3 ✅** | I.3 饱和 | 48 | 29 min 缺口 + 3 综合题 + 16 强化 | ✅ 已合入：I.3 15 绿/0 黄/0 红、综合 8/8 |
| B4 | I.1–I.3 收口 | 43 | 黄转绿、prep（I.6/I.7）补 L1/L2、综合题补到 27/29 | 三章全绿或黄 ≤2 且有记账；综合 27+ |
| **B4** | 收口（prep + open） | 7 | 4 prep choice（补 I.6/I.7 的 L1/L2）+ 3 open（short/explain，rubric） | 全库 54 绿 / 0 黄 / 0 红 |
| **阶段 1 ✅ 小计** | I.1–I.3 | **179** | ✅ 完成：221 题、54/54 绿、0 红、饱和率 100% | |
| **阶段 2（C9.1 已注册 KP）** | I.4–I.13 | **493**（bank 口径） | 论域/悖论/序数/AC-ZF/基数/集族/闭包/集合代数/幂集/关系代数 | 每批 `verify_batch` + 收口批目标章 0 红（首波例外见 §2.1）；逐章表见 §2.1 |

### 2.1 阶段 2 批次表（C9.1 实测；C9.2/C9.3 的出题输入）

**批次边界**：常规批 **40–50 题**，任何批 **≤50**；单章目标 <40（I.8 18 · I.9 18 ·
I.12 36）整章一批；多批章的**收口批**可 <40（I.4 37 · I.6 37 · I.10 32 · I.13 23）。
**不跨章拼批**：`verify_batch.py --chapter` 按单章预测红/绿，跨章拼批会互相拖红。
KP 波次是**出题顺序**（注册已在 C9.1 一次完成）：同一波次的 KP 与该批题一起把
该章推到 0 红；不是「先注册、后出题」的两步。

**多批章的首波**（B5/B8/B13/B17）该章还有后续波，`verify_batch.py` 的「目标章
0 红」**不可能**满足 ⇒ 首波用 `--no-coverage`（结构/source/kps 引用仍逐条判），
**收口批**（B6/B9/B14/B18）必须跑完整 `--chapter` 并 0 红。这是 C9.1 一次性注册
全部 KP 的直接后果（`coverage.py` 对 0 题 KP 判红）；若改按波次注册 KP，首波也能
0 红，但注册表就只覆盖部分 KP，违背本 ticket「为 I.4–I.13 注册」的验收。

| 批次 | 章 | 题数 | KP 波次（出题顺序） | 该批验收（全过才算完成） |
|---|---|---|---|---|
| B5 | I.4 | 40 | u11–u57（10 KP） | `verify_batch --batch data/_staging-b5.json --chapter I.4 --sample 8 --no-coverage`（首波；结构/source/kps 逐条判）+ `coverage.py` 看 I.4 红 17→7（B6 收口） |
| B6 | I.4 | 37 | u87–u95（7 KP） | 同上（`_staging-b6.json`）；合入后 **I.4 0 红** |
| B7 | I.5 | 32 | 全章 7 KP | `--chapter I.5`；合入后 **I.5 0 红** |
| B8 | I.6 | 40 | u15–u80（8 新 + 1 prep） | `--chapter I.6 --sample 8 --no-coverage`（首波）+ `coverage.py`；该波 8 新 KP 转绿，I.6 红 16→8（B9 收口） |
| B9 | I.6 | 37 | u81–u111（8 KP） | 同上；合入后 **I.6 0 红** |
| B10 | I.7 | 50 | 全章 10 新 + 1 prep | `--chapter I.7`；合入后 **I.7 0 红** |
| B11 | I.8 | 18 | 全章 4 KP | `--chapter I.8`；合入后 **I.8 0 红** |
| B12 | I.9 | 18 | 全章 4 KP | `--chapter I.9`；合入后 **I.9 0 红** |
| B13 | I.10 | 40 | u24–u27（10 KP） | `--chapter I.10 --sample 8 --no-coverage`（首波）+ `coverage.py`；该波 10 KP 转绿，I.10 红 16→6（B14 收口） |
| B14 | I.10 | 32 | u43–u86（6 KP） | 同上；合入后 **I.10 0 红** |
| B15 | I.11 | 50 | 全章 11 KP | `--chapter I.11`；合入后 **I.11 0 红** |
| B16 | I.12 | 36 | 全章 8 KP | `--chapter I.12`；合入后 **I.12 0 红** |
| B17 | I.13 | 40 | u36–u40（10 KP） | `--chapter I.13 --sample 8 --no-coverage`（首波）+ `coverage.py`；该波 10 KP 转绿，I.13 红 14→4（B18 收口） |
| B18 | I.13 | 23 | u41–u52（4 KP） | 同上；合入后 **I.13 0 红** |

**每章单条验收命令**（把 `I.4` / `b5` 换成实际章/批）：

```bash
cd courses/set-theory/knowledge
# 多批章首波（B5/B8/B13/B17）加 --no-coverage；收口批（B6/B9/B14/B18）全跑
python3 verify_batch.py --batch data/_staging-b5.json --chapter I.4 --sample 8
python3 coverage.py --json | python3 -c 'import json,sys; c=json.load(sys.stdin)["chapters"]["I.4"]; print(c["red"],c["yellow"],c["green"]); sys.exit(0 if c["red"]==0 else 1)'
```

**source 可复查规则**（每题落笔即定，不允许事后补假行号）：`source` 写
`courses/set-theory` 相对路径 + `:行号`（如 `units/I.4/unit11-universe-russell.sokonanoda:83`）；
行号必须是打开文件核过的**非空真实行**，答案/`why` 能由该行（含紧邻上下文）核验；
`verify_batch.py` 只查文件/行号存在，**语义核验靠人**：每批按 level 分层抽 ≥10%
（`--sample 8` 给出样本），逐条打开核答案；写不出来源的题进 §4「暂缓题记账」，
不许编假 source 混过 validate。细则见 `SCHEMA.md` §2.5。

**lessons 同步**：每批合入时把该单元 `questionTotal` 与 `knowledgePoints` 更新到
`data/lessons.json`；`validate.py` 已把「KP title/kind 与 lessons 一致 / questionTotal
等于真实题数 / 每个 KP 至少出现在一个 lesson」钉成判据（C9.1 新增）。


**每批固定动作**（写进批次 checklist）：

1. 按清单写题（open 题同时写 `rubric` + `reference`，keywords 必须来自 `reference` 原词）；
2. 每题 `kps` 至少 1 个（综合题 ≥2），`source` 落**可打开核验的文件:行号**（新行号写题时同步补）；
3. `python3 validate.py`（kps 引用/章节归属/综合题结构）；
4. `python3 coverage.py`（**收口批**本章零红；首波见 §2.1 的 `--no-coverage` 例外；红则回到第 1 步）；
5. 抽检 ≥10% 题目的 `source` 行号真实存在且答案能由该处核验（禁止凭印象改答案）；
6. `python3 engine.py selftest`（引擎自检）+ 若有新 open 题：同一批用 `engine.py answer --text <rubric 关键词>` 自测判分。

**写不进库的先记账**：如果某题必须等新引理/新单元行号（例如 I.3 的 Cantor–Bernstein
需要 `lib/Choice` 的现成结论），在 §4「暂缓题记账」表登记（题号/KP/level/source 候选/
阻塞原因），**不许先编一个假 source 混过 validate**。

### 2.2 阶段 3：卷 II/III 分析课（C9.3，2026-11-26 起）

**题库是一份、课是多门**：卷 II/III 的 KP、题目、lesson、卡片与术语直接进本目录的
`data/`，用同一套 `validate.py` / `coverage.py` / `verify_batch.py` / `engine.py`，
**不复制第二套工具**。跨课程题目的 `source` 写仓库根相对路径
（`courses/analysis/units/…:行号`），`verify_batch.py` 会回退到仓库根解析。

**换算口径**：分析课 175 单元（卷 II 72、卷 III 103；含 2026-11-25 的单元174/175）；按
**1.5 KP/单元**估算 **266 个 KP**（区间 240–290），bank 目标按 **KP × 4.0**
（技术课每 KP 题量略低于卷 I 的 4.5）估算 **≈ 1064 题**（区间 900–1300），按
40–50 题/批约 **30 批**（区间 20–30）。最终以注册表实测为准，**不锁计数**。

| 章 | 单元 | KP 估算 | bank 目标 | 批次 | 状态 |
|---|---:|---:|---:|---:|---|
| II.1 整数：从 ℕ 到 ℤ | 19 | 29 | 116 | 3 | **B1–B4 已落盘（121 题 / 29 KP / 单元①–⑭）**；B5 待做（单元⑮–⑲） |
| II.2 有理数：从 ℤ 到 ℚ | 9 | 14 | 56 | 2 | 待做 |
| II.3 有理数的性质 | 6 | 9 | 36 | 1 | 待做 |
| II.4 有理 Cauchy 序列与实数载体 | 10 | 15 | 60 | 2 | 待做 |
| II.5 ℝ 的商、代数与序 | 10 | 15 | 60 | 2 | 待做 |
| II.6 实数的商与完备性 | 14 | 21 | 84 | 2 | 待做 |
| II.7 数系综合与读证明 | 4 | 6 | 24 | 1 | 待做 |
| III.1 序列与极限 | 11 | 17 | 68 | 2 | 待做 |
| III.2 扩展实数、上下确界与单调收敛 | 7 | 11 | 44 | 1 | 待做 |
| III.3 子序列、极限点与上下极限 | 9 | 14 | 56 | 2 | 待做 |
| III.4 Cauchy 序列与完备性 | 10 | 15 | 60 | 2 | 待做 |
| III.5 级数 | 14 | 21 | 84 | 2 | 待做 |
| III.6 连续函数 | 16 | 24 | 96 | 2 | 待做 |
| III.7 微分 | 11 | 17 | 68 | 2 | 待做 |
| III.8 Riemann 积分 | 16 | 24 | 96 | 2 | 待做 |
| III.9 综合、读证明、小项目 | 9 | 14 | 56 | 2 | 待做 |
| **合计** | **175** | **266** | **1064** | **30** | |

**C9.3 的落盘约定**（与 C9.1 的「一次注册全部 KP」不同）：

1. **KP 与题同波落盘**：每一波只注册本波要出题的 KP，并在同一改动里补 lesson
   与 `questionTotal`。这样分析课的每一章在落盘后**立即 0 红**，不会长期挂着
   预期红；C9.1 的一次性注册是它为「注册全部 I.4–I.13」这一验收目标做的选择，
   C9.3 不必复制。
2. **id 与 unit**：`II.1.<slug>-<n>`（题）、`kp.ii1.<slug>`（KP）；`unit` 取
   `course.json` 的单元标题去掉「单元」前缀（如 `① 整数与商构造`），lesson 必须
   用同一个字符串。
3. **source**：`courses/analysis/units/<章>/<文件>.sokonanoda:行号`，行号必须是
   落笔时打开文件核过的非空行；优先引用定义行 / 定理行 / 练习行 / 单元头部边界
   注释。每批按 level 分层抽 ≥10% 逐条打开核答案。
4. **边界**：不改 `courses/analysis` 的画布/解答/lib，不改 `engine.py`/kernel；
   发现课程内容缺口记 `courses/analysis` 的 gaps，不在题库里改课程事实。

#### B1–B4 已落盘实测（2026-11-26，待审）

范围：II.1 单元①–⑭（商构造、ℕ 算术、整数运算代表元与良定义性、
乘法单位元与良定义性、整数序代表元、ℕ 序与加法消去）。
新增 **29 KP / 121 题**（choice 121；L1 19 / L2 67 / L3 35；150 个 kps 引用、
综合题 24），14 个 lesson、3 张知识卡、16 条 glossary 术语。source 落在
`unit01…`–`unit14…`，逐条打开核过。

| 指标 | B1 前 | B1 后 |
|---|---|---|
| 题库 | 227 题 / 3 章 | **348 题 / 4 章** |
| II.1 覆盖 | 0 KP（未注册） | **29 绿 / 0 黄 / 0 红**，缺口 0 |
| II.1 综合题 | — | **24 / 目标 15** |
| 全库红 | 107（C9.2 预期） | **107（未变；C9.2 的 I.4–I.13 仍无题）** |

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

规格与可执行 checklist 见 **`SCHEMA.md`**（字段/约束/source 规则/批次流程；改数据前先读）。
`validate.py` 现在除结构/交叉引用外，还钉：**每个 KP 有 `keyPoint`**、**prereq 无环**、
**lessons 的 KP title/kind 与 `questionTotal` 同步**。

```bash
cd courses/set-theory/knowledge

# 1) 草稿验收（content-author 交 data/_staging-b5.json，裸 JSON 数组；不改仓库）
python3 verify_batch.py --batch data/_staging-b5.json --chapter I.4 --sample 8
#    → 结构/15 字段/id 撞号/kps 引用/source 行号逐条核验 + 内存注入预测目标章红黄绿

# 2) 合入后（由 Lead/指定人把数组 extend 进 data/questions.json）
python3 validate.py        # 结构 + kps 引用 + keyPoint/prereq 无环/lessons 同步
python3 coverage.py        # 覆盖门禁：任一 KP 红 → exit 1，并写 reports/coverage-<时间>.json
python3 coverage.py --no-report   # 只看汇总
python3 coverage.py --json        # 机器可读（CI/agent 消费）
python3 engine.py selftest        # 引擎自检（改题库后必跑）
python3 engine.py simulate --questions 50
python3 text_audit.py             # 完整句子启发式；新文本不应新增问题
python3 ../../../scripts/notation-lint.py   # 改了课程单元/画布才需要
```

- **批次协作约定（B1 起，B5 起沿用）**：content-author 只写 `data/_staging-b5.json`
  （裸数组；choice 字段 `id/kind/chapter/unit/level/tags/difficulty/variants/prereq/
  prompt/options/answer/why/source/kps`，开放题把 `options/answer` 换成
  `rubric/reference`）；product-manager 用 `verify_batch.py` 复核并出覆盖预测；
  合入 `data/questions.json` 后重跑 `validate.py` + `coverage.py` + `engine.py selftest`。

- `coverage.py` 的「红」只严格卡 `saturation.min`；`--fail-on-yellow` 可把黄也当红（更严）。
- 生成的报告 `reports/coverage-<时间>.json` 是**可回溯的覆盖账**：每次合入一批就留一份。
- `kps` 的章节归属规则：KP 的 `chapter` 必须等于题目的 `chapter`；例外是 `kind:"prep"`
  （I.6/I.7 预备知识点允许挂在 I.3 等前置章节的题上）——`validate.py` 已按此校验。

# 题库与知识点 schema（可执行规格与 checklist）

> 状态：**生效**（2026-11-25，C9.1）。本文件把 `soko.choice-bank/1` 与
> `soko.knowledge-points/1` 的字段、约束、覆盖口径和批次流程收成一份**可执行**
> 清单：每条规则要么由现有脚本判，要么给出可复跑的命令；本文件不复制题、不写题。
>
> 相关文件：`DESIGN.md`（三层设计与调度语义）· `STYLE.md`（文本风格）·
> `CONTENT-PLAN.md`（覆盖现状与批次表）· `README.md`（工作流总览）。
> 判据脚本：`validate.py` · `coverage.py` · `verify_batch.py` · `text_audit.py`。

## 0. 文件职责

| 文件 | schema | 作用 | 谁改 |
|---|---|---|---|
| `data/knowledge-points.json` | `soko.knowledge-points/1` | 知识点注册表（覆盖的记账单位） | 备课/内容线 |
| `data/questions.json` | `soko.choice-bank/1` | 题库（choice / short / explain / proof） | 备课/内容线 |
| `data/lessons.json` | `soko.lessons/1` | 课级教学环节（由 KP/题目归并） | 备课/内容线 |
| `data/cards.json` | `soko.knowledge-cards/1` | 知识卡 | 备课/内容线 |
| `data/params.json` | `soko.learn-params/1` | 调度参数 | 维护线 |
| `engine.py` | — | L2 调度引擎 | 不改（C9 线） |

**边界**：内容线只改 `data/` 与 `*.md`；不改 `engine.py`、不改 kernel、不改
`crates/`；`out/`、`reports/` 是生成物，不手改。

> **多课程（C9.3）**：本库同时承载卷 I 集合论与卷 II/III 分析课的选择题/KP；
> 分析课按波次注册 KP 并同步出题（落盘即 0 红），id 用 `II.1.slug-n` /
> `kp.ii1.slug`，其余字段与卷 I 相同。

## 1. `soko.knowledge-points/1`

### 1.1 顶层对象

```jsonc
{
  "schema": "soko.knowledge-points/1",
  "version": 4,                 // 每次改 kps 递增
  "updated": "2026-11-25",      // ISO 日期
  "saturation_policy": { "...": "见 §1.4" },
  "kinds": ["definition", "concept", "technique", "boundary", "integration", "prep"],
  "kps": [ /* §1.2 */ ]
}
```

### 1.2 单个知识点（每个新 KP 必填项）

| 字段 | 类型 | 必填 | 判据 |
|---|---|---|---|
| `id` | string | 是 | `^kp\.[a-z0-9]+(\.[a-z0-9]+)*$`，如 `kp.i4.russell`、`kp.ii1.intrel`；稳定、不重编号 |
| `chapter` | string | 是 | 必须是 `questions.json.chapters` 里的章 id（`I.1`–`I.13`、`II.1`–`III.9` 按批登记） |
| `title` | string | 是 | 非空；一句话说清“这个 KP 是什么” |
| `kind` | string | 是 | 六种枚举之一，语义见 §1.3 |
| `prereq` | string[] | 是 | 只能引用**已存在**的 KP id；不自指、不成环（`validate.py` 新查环） |
| `saturation.min` | `{L1,L2,L3}` | 是 | 恰好三个键；非负整数；三值之和 ≤ `target` |
| `saturation.target` | int | 是 | 正整数；达到即绿，低于 min 即红 |
| `keyPoint` | string | 是 | **非空**；一句话写清“真正要考的能力”（`validate.py` 新查） |
| `note` | string | 否 | 备注；不得把答案写进题干 |

**粒度**：I.4–I.13 实测 **109 KP / 77 单元（1.42 KP/单元）**。计划草稿按
1.49 KP/单元外推 115；逐单元审计合并了 6 个与既有 I.3/I.4 KP 重复的条目后为 109
（见 `CONTENT-PLAN.md` §0.1），**没有为凑下界保留重复 KP**。不要为“覆盖全绿”拆 KP；
一个 KP 必须**可教、可出题、可判分、可核验**。同义/重叠 KP 合并。

### 1.3 `kind` 的语义（选错会让 saturation 与题目形态都失衡）

| kind | 含义 | 典型题 |
|---|---|---|
| `definition` | 定义、记法、接口（“是什么/怎么展开”） | 给情境问展开成什么 |
| `concept` | 概念辨析、边界、相互翻译（“何时一致/何时失效”） | 两项对比、反例边界 |
| `technique` | 证明/使用手法（“先用哪件、顺序为何不能反”） | 给前提问下一步 |
| `boundary` | 本课证不出/需经典逻辑或 AC/反例边界 | 问卡在哪一步、为何越界 |
| `integration` | 跨章综合（多章结论串成一条链） | 综合证明/读证明 |
| `prep` | 前置章（I.6/I.7）的预备桥；题目可挂到前置章的题上 | 预备级回忆/展开 |

### 1.4 saturation（覆盖口径）

- `min` 是**硬门禁**：任一档低于 `min` ⇒ `coverage.py` 判红、exit 1。
- `target` 是**饱和目标**：达到 `target` 且各档过 `min` ⇒ 绿；两者之间为黄。
- 章级综合题目标 = `ceil(章 KP 数 / 2)`（一题挂 ≥2 个 KP 即算综合题）。
- bank 目标 ≈ `KP × 4.5` = `Σ saturation.target（≈4/KP）+ 综合题（≈0.5/KP）`；
  `×1.2` 只是**排期返工系数**，不重复计入 bank 规模（见 `CONTENT-PLAN.md` §1）。
- 惯例区间（不是硬判据，按内容定）：
  - `definition` / `concept`：`{L1:1,L2:1,L3:1}`，target 3–5；
  - `technique`：`{L1:0,L2:2,L3:1}`，target 4–5；
  - `boundary`：`{L1:0,L2:1,L3:1}`，target 3–4；
  - `prep`：`{L1:1,L2:1,L3:0}` 或 `{L1:1,L2:1,L3:1}`，target 2–3；
  - `integration`：`{L1:1,L2:1,L3:1}`，target 5。
- **不许为“全绿”虚增 target 或压低 min**。新 KP 尚无题时，允许（且必然）为红；
  在 C9.2/C9.3 补题后自然转绿。

### 1.5 prereq

- 方向 = “学这个 KP 之前必须先会什么”；只引用**已有** KP，不引未来 KP。
- 必须无环；`validate.py` 用 DFS 查环并打印环路径。
- 跨章前置（如 I.5 → I.2）允许；`prep` 只用于 I.6/I.7 的桥接。
- 拆/并 KP 时同步：`questions.kps`、`lessons.knowledgePoints`、`cards.tags`、
  `CONTENT-PLAN.md`；用 `validate.py` + `coverage.py` 验 `unknown_refs=0`。

### 1.6 keyPoint

一句话、可检验、写“能力”而不是“定义复述”。例：
`kp.i4.russell` 的 keyPoint = “指出自指在本语言里落不到类型上”，而不是
“Russell 悖论是什么”。题目 `kps` 必须与 `keyPoint` 对得上（人工抽检）。

## 2. `soko.choice-bank/1`

### 2.1 顶层对象

```jsonc
{
  "schema": "soko.choice-bank/1",
  "title": "卷 I–III 选择题题库（集合论 / 分析）",
  "version": 27,                // 每次加题递增
  "updated": "2026-11-22",
  "policy": "…",
  "chapters": { "I.1": "…", "…": "…" },
  "questions": [ /* §2.2 */ ]
}
```

### 2.2 题目公共字段

| 字段 | 类型 | 必填 | 判据 |
|---|---|---|---|
| `id` | string | 是 | `^[IVX]+\.\d+\.[a-z0-9-]+$`，如 `I.4.russell-1`、`II.1.intrel-1`；稳定、不重编号 |
| `chapter` | string | 是 | 必须在 `questions.json.chapters` |
| `unit` | string | 是 | 课程单元标签（`course.json` 标题去掉“单元”前缀）；新题必须与课程一致，否则 lesson 挂不上 |
| `level` | 1/2/3 | 是 | L1 回忆/展开 · L2 一步应用 · L3 辨析/边界 |
| `tags` | string[] | 是 | 非空字符串；知识卡按 tag 匹配 |
| `prompt` | string | 是 | 完整问句，以 `？` 结尾；不泄答案（`STYLE.md` §1） |
| `kind` | string | 否 | `choice`（默认）/`short`/`explain`/`proof` |
| `options` | string[] | choice 必填 | 2–6 项（题库惯例 4）；非空、互不重复 |
| `answer` | int | choice 必填 | 1-based 选项号；只能有一个正确项 |
| `rubric` | object | 开放题必填 | `minScore ∈ (0,1]`；`points[].any[]` 非空（`verify_batch.py` 也查） |
| `reference` | string | 开放题必填 | 参考答法；rubric 关键词必须来自它 |
| `why` | string | 是 | 2–4 句：先结论、再理由、必要时反例（`STYLE.md` §3） |
| `source` | string | 是 | `courses/set-theory` 相对路径 + `:行号`，见 §2.5 |
| `difficulty` | number | 是 | 只能是 `-1.5 / 0 / 1.5`，与 L1/L2/L3 对齐 |
| `variants` | string[] | 是 | 只能是真实题 id；不引自己 |
| `prereq` | string[] | 是 | 只能是真实题 id；不引自己 |
| `kps` | string[] | 是 | 1–4 个；引用存在的 KP；章节必须等于题目的章（`prep` 例外） |

`choice` 与开放题互斥：开放题不写 `options`/`answer`；`proof` 走内核判卷
（C9.4 接线前不进入选择题批次，schema 保留）。

### 2.3 文本与干扰项（`STYLE.md` 的可执行部分）

- 题干是完整问句，有前提/主语 + 明确提问，以 `？` 结尾；不写成“XX 是？”。
- 选项是完整陈述句（纯代码/符号选项例外，题干须明确“下面哪个表达式…”）；
  四项结构平行，长度 8–60 字；禁止“以上都对/以上都不对”。
- 一个正确答案 + 3 个可信干扰项；不要让两个选项都成立。
- **选项长度不泄漏答案**：正确项不得比最长干扰项长 15 字以上
  （2026-11-24 审计：旧库 174/219 道正确项最长）。`verify_batch.py` 对新批判红
  （`option.length-leak`），`text_audit.py` 对全库给出同一口径的报告（存量待清洗）。
- **正确项位置要打散**：存储顺序就是渲染顺序（`build_quiz.py` / 教学卡片都按
  存储顺序显示，不洗牌）⇒ 整批 ≥8 道 choice 时，同一选项位不得超过 60%
  （`verify_batch.py` 判红）。新批的选项顺序应按 id 轮换正确项位次，不能整批都放第 1 位。
- `why` 解释正确项，并点出主要干扰项的错因；不把答案写进 `prompt`。
- `why` 必须是 2–4 个完整句子（新批由 `verify_batch.py` 判红；存量由
  `text_audit.py` 的 `why.short` 报告，不判红）。
- 同一 KP 不要堆同模板题；coverage 全绿 ≠ 题库质量好。

### 2.4 开放题（short / explain）

- `rubric.minScore` 与 `rubric.points[]` 必填；每个 point 的 `any[]` 是**可判定
  的必要点**，`note` 写“为什么是这个点”。
- rubric 覆盖 `keyPoint` 的必要点，避免只匹配单个词；答案不得出现在题面提示里。
- `reference` 给参考答法；关键词必须来自 `reference` 原词。
- 合入前用 `engine.py answer --text <关键词>` 自测判分（同一批至少抽 1 题）。

### 2.5 source 可复查规则（硬）

1. 格式：`路径:行号`。卷 I 的路径相对 `courses/set-theory`（如
   `units/I.4/unit11-universe-russell.sokonanoda:73`、`lib/Set.sokonanoda:159`）。
   跨课程（卷 II/III 分析课）的题写仓库根相对路径
   `courses/analysis/units/II.1/unit01-int-quotient.sokonanoda:11`；
   `verify_batch.py` 先按课程根解析，失败再按仓库根解析。
2. 行号必须是**落笔时打开文件核过的真实行**；该行非空，且答案/`why` 能由该行
   （连同紧邻上下文）核验。**不许**用占位行号、不许凭印象写、不许写“见某文件”。
3. 一行不够就写多段 `文件:行号`；优先引用**声明行/演示行/练习行**。
4. `verify_batch.py` 只查“文件存在 + 行号不越界 + 非空”，**语义核验靠人**：
   每批按 level 分层抽 ≥10%（`--sample` 打印样本），逐条打开 source 核答案。
   同一检查也进了 `validate.py`（全库逐题核）**与 CI**（`ci.yml` 的 `gates-course`
   在重活之前跑 `validate.py`）——2026-11-24 审计发现全库有 6 道题 source 不合规
   却照样合入，所以口径不能只覆盖当批；两处共用 `source_check.py`，不写第二份。
5. 新单元的行号在写题时同步补；若某题必须等新引理/新单元而写不出 source，
   记进 `CONTENT-PLAN.md` §4「暂缓题记账」，**不许编假 source 混过 validate**。

### 2.6 id 与稳定性

- `I.<章号>.<slug>-<n>`，slug 小写 kebab；新题接现有最大序号，**不重编号**。
- `variants` 只引真实 id；同 tag 变式组用于答错后回练。
- `prereq` 指前置**题**（不是 KP）；可为空。

## 3. 覆盖门禁（`coverage.py`）

- **统计口径 = 可服务题**：`params.json` 的 `selection.exclude` 排除的题不计
  （`engine.py` 选题时就跳过它们；覆盖门禁必须同口径，否则“绿”是名义上的）。
  `--include-excluded` 可切回全库口径做诊断；报告里 `selection` 段给出排除清单与
  同一份清单里不在库中的 id（防参数漂移）。
- 红：某个 L1/L2/L3 档 < `saturation.min`（必须补；exit 1）。
- 黄：达到 min，但总数 < `target`（可补；默认不判红）。
- 绿：达到 target 且各档过 min。
- 报告落 `reports/coverage-<时间>.json`（生成物，不手改），含每 KP 的
  `counts/missing/deficit/status/question_ids` 与章级汇总。
- `--json` 机器可读；`--fail-on-yellow` 更严；`unknown_refs` 非空说明
  `validate.py` 应先修。
- **注册即红**：新 KP 没有题时必然红；C9.1 一次性注册 I.4–I.13（见
  `CONTENT-PLAN.md` 阶段 2），`coverage.py` 预期红并逐条点名；C9.2 按章补题后
  转绿。**不许**用降低 min / 虚增 target / 删 KP 来伪造绿色。

## 4. lessons 同步（隐藏契约）

`lessons.json` 复制了每个 KP 的 `title/kind` 和每单元的 `questionTotal`，
`validate.py` 现在会检查：

1. 每个 `lesson.knowledgePoints[].id` 都存在于注册表；
2. 其 `title` 与 `kind` 与注册表**逐字一致**；
3. `questionTotal` 等于 `(chapter, unit)` 下真实题目数；
4. 每个 KP 至少出现在一个 lesson 里（新 KP 必须同批补 lesson）；
5. `goal` / `intuition` 非空；完整句由 `text_audit.py` 启发式扫（不判红）。

新增 KP/单元时：同一改动里补 `lessons.json`（`unit` 用课程单元标签；无题时
`questionTotal: 0`；`cards` 可为空）。`cards.json` 与 `glossary.json` 不是
C9.1 的门禁项，按章补。

## 5. 批次工作流（每批 ≤50 题，一次只做一批；常规 40–50）

1. **选缺口**：按 `CONTENT-PLAN.md` §2.1 的批次表选章；先确认该章 KP 已注册
   （`validate.py` 绿），再写题。单章目标 <40 或收口批可小于 40（表里已标），
   但**任何批 ≤50**，且**不跨章拼批**（`verify_batch.py` 按单章判红）。
2. **草稿**：新题写进 `data/_staging-<batch>.json`（裸数组或 `{questions:[...]}`；
   草稿不算仓库内容）。
3. **草稿验收**：

   ```bash
   python3 courses/set-theory/knowledge/verify_batch.py \
     --batch data/_staging-<batch>.json --chapter I.<n> --sample 8
   ```

   结构 / source 行 / kps 引用 / 覆盖预测全过才可合入。**多批章的首波**该章还有
   后续波，`--chapter` 的「0 红」不可能满足 ⇒ 首波加 `--no-coverage`（结构/source/
   kps 引用仍逐条判），**收口批**必须全跑并 0 红；这是 C9.1 一次性注册全部 KP 的
   直接后果（见 `CONTENT-PLAN.md` §2.1）。
4. **人工抽检**：按输出的 sample 逐条打开 `source`，核答案与 `why`；再随机抽
   ≥10% 的题（含至少 1 道开放题）。
5. **合入**：追加进 `data/questions.json`（按 id 去重、保持 schema/version）；
   同批更新 `lessons.json` 的 `questionTotal` / `knowledgePoints`。
6. **收尾门禁**（全跑，全 exit 0）：

   ```bash
   python3 courses/set-theory/knowledge/validate.py
   python3 courses/set-theory/knowledge/coverage.py        # 目标章 0 红
   python3 courses/set-theory/knowledge/engine.py selftest
   python3 courses/set-theory/knowledge/engine.py simulate --questions 50
   python3 scripts/notation-lint.py                        # 改了单元/画布才需要
   python3 courses/set-theory/knowledge/text_audit.py      # 报告；新文本应 0 新增问题
   ```

   单章零红断言（机器判据，替代肉眼）：

   ```bash
   python3 courses/set-theory/knowledge/coverage.py --json |
     python3 -c 'import json,sys; c=json.load(sys.stdin)["chapters"]["I.4"]; print(c["red"],c["yellow"],c["green"]); sys.exit(0 if c["red"]==0 else 1)'
   ```

7. **报告**：before/after 计数、红黄绿、kind/level 分布、新增/更新 id、source
   抽检结果、风险与下一批；把批次读数写进 `CONTENT-PLAN.md`。

## 6. 可执行 checklist（复制到 PR 描述）

- [ ] `python3 courses/set-theory/knowledge/validate.py` → `KNOWLEDGE VALIDATE OK`，exit 0
      （含全库 `source` 的文件:行号存在性）
- [ ] 每个 KP 的 `chapter/kind/prereq/saturation/keyPoint` 齐全；prereq 无环
- [ ] `python3 courses/set-theory/knowledge/coverage.py` 跑通；本批目标章 0 红
      （按可服务题；C9.1 注册批例外：逐条列出预期红，见 `CONTENT-PLAN.md`；
      多看一行“题库共 N，排除 M”，确认口径没变）
- [ ] `verify_batch.py --batch … --chapter … --sample 8` → exit 0
      （含选项 8–60 字、正确项不超最长干扰项 15 字、`why` ≥2 句）
- [ ] source 抽检 ≥10%，逐条打开核验答案；无占位/假行号
- [ ] `text_audit.py` 没有**新增** `why.short` / `option.length*` 条目（存量可暂留）
- [ ] `engine.py selftest` + `engine.py simulate --questions 50` → exit 0
- [ ] `text_audit.py` 无新增问题；`notation-lint.py` 绿（若改课程单元）
- [ ] `lessons.json` 的 KP title/kind 与 `questionTotal` 与注册表/题库一致
- [ ] 未改 `engine.py` / kernel / `crates/`；未写题（C9.1）

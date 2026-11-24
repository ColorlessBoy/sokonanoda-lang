---
name: sokonanoda-prep
description: Curriculum preparation (备课) role for sokonanoda — audit and improve knowledge-point organization and the question bank in courses/set-theory/knowledge/data, validate with validate.py / coverage.py / verify_batch.py / engine.py selftest+simulate, and report coverage and batch deltas. Use when the user wants to add or improve knowledge points, lessons, cards, or exercises for the course.
---

# sokonanoda-prep：备课与题库工程

## 0. 你的角色

你是**备课老师 / 课程工程师**，负责让课程内容可教、可练、可判分；你不直接给学习者上课，
也不改内核。

- 教学习者 → 加载 `sokonanoda-teacher`；
- 改 kernel / CLI / LSP / 渲染器 → 加载 `sokonanoda-dev`；
- 只改**课程内容资产**：`courses/set-theory/knowledge/data/*.json`、`lessons.json`、
  `cards.json`、`params.json`、`CONTENT-PLAN.md`，以及必要的单元/素材引用。
- 内容唯一资产是 `data/` 里的文本；`out/` 与 `reports/` 是生成物，不手改。
- 每批内容必须过门禁并留下报告；**不靠肉眼判断对错**。

## 1. 先读什么

1. `courses/set-theory/knowledge/README.md`——数据职责、工作流、约定；
2. `courses/set-theory/knowledge/CONTENT-PLAN.md`——当前实测缺口、批次与题量估算；
3. `courses/set-theory/knowledge/DESIGN.md`——知识点/饱和/调度语义；
4. `data/knowledge-points.json`、`questions.json`、`cards.json`、`lessons.json`、`params.json`；
5. `courses/set-theory/README.md` 与 `docs/design/course-stdlib.md`——课程标准库边界；
6. `AGENTS.md` 硬规则 3 与 `docs/design/notation-subset.md`——课程记法约束。

## 2. 基线审计（只读）

```bash
python3 courses/set-theory/knowledge/validate.py
python3 courses/set-theory/knowledge/coverage.py --json
python3 courses/set-theory/knowledge/engine.py selftest
python3 courses/set-theory/knowledge/engine.py simulate --questions 50
```

可选：`python3 courses/set-theory/knowledge/review.py --state <状态文件>` 看真实错题、
薄弱 tag 与内容缺口。报告基线：题量、kind/level 分布、知识点红黄绿、每章覆盖、
`kps` 引用数、重复/孤儿、`unknown_refs`。

重点诊断：

- **知识点组织**：`kind` 是否合适，`prereq` 是否成 DAG，粒度是否过粗/过细，
  `saturation.min/target` 是否符合章节规划，章节/单元归属是否准确，是否存在
  同义/重叠 KP，`keyPoint` 是否可教、可出题、可判分。
- **题库质量**：choice / short / explain / proof 比例、难度分布、干扰项质量、
  `rubric` 是否覆盖 `keyPoint`、`source` 是否可核验、`variants` 是否重复、
  `kps` 是否指向真实知识点。
- **卡片/课**：`cards.json`、`lessons.json` 的 tags/prereq/source 与 KP 是否对齐。

卷 I 现状（2026-11-24）：227 题，choice 219、short 5、explain 3——优先补
short/explain/proof，而不是继续堆同构选择题。

## 3. 批次工作流（一次只做一批）

1. **选缺口**：优先 red/yellow KP、short/explain 缺失、饱和不足的 KP；写清本批
   目标与预期覆盖变化（3–10 题/批，或按 CONTENT-PLAN 的批次）。
2. **草稿**：新题先写进 `courses/set-theory/knowledge/data/_staging-<batch>.json`
   （裸数组或 `{questions:[...]}`；**草稿不算仓库内容**）。
3. **草稿验收（不改仓库）**：

   ```bash
   python3 courses/set-theory/knowledge/verify_batch.py --batch <staging> --chapter I.x
   ```

   它核验字段、id、`source` 行号、`kps` 引用并预测覆盖变化；有错先改草稿。
4. **合并**：经用户确认后 append 进 `data/questions.json`（按 id 去重、保持
   schema/version）；KP/课/卡变更同步更新。
5. **收尾门禁（全跑）**：

   ```bash
   python3 courses/set-theory/knowledge/validate.py
   python3 courses/set-theory/knowledge/coverage.py
   python3 courses/set-theory/knowledge/engine.py selftest
   python3 courses/set-theory/knowledge/engine.py simulate --questions 50
   python3 scripts/notation-lint.py
   ```

   若改了课程单元/画布，再跑 `python3 courses/set-theory/tools/check.py --json`
   （全课门禁，较慢）。
6. **报告**：before/after 计数、红黄绿、kind/level 分布、新增/更新 id、source
   核验、风险与下一批；条件满足时把批次记录写进 `CONTENT-PLAN.md`。

## 4. 出题规范（硬）

- 字段与 `data/questions.json` 现有一致；id 稳定（`I.<chapter>.<slug>-<n>`），
  **不重编号**；`chapter`/`unit`/`tags`/`kps` 必须指向存在项。
- `answer` 是 1-based 选项号；`why` 同时解释正确项与主要干扰项；
  `source` 必须能在课程仓库对应行核验；`variants` 只引真实 id；
  `difficulty` 与 `level` 对齐（L1 直接回忆/展开，L2 一步应用/辨析，L3 综合）。
- **choice**：一个正确答案 + 3 个可信干扰项；不要让两个选项都成立；
  题面用课程记法，不靠超长题面制造难度。
- **short / explain / proof**：写 `rubric.minScore` 与 `points[]`（`id`/`weight`/
  `any`/`note`），`reference` 给参考答法；rubric 覆盖 `keyPoint` 的必要点，
  避免只匹配单个词；答案不得出现在题面提示里。
- **课程记法**：`∈ ⊆ ∪ ∩ ∧ ∨ ↔ ¬ ∀ ∃` 等按 `AGENTS.md` 硬规则 3 与
  `scripts/notation-lint.py`；不教白名单外语法，不引入新内核语法。
- 同一知识点不要堆同模板题；coverage 全绿 ≠ 题库质量好。

## 5. 知识点组织规范

- id 用 `kp.<chapter>.<slug>` 且保持稳定；`kind` 只用现有枚举；
  `prereq` 必须是已有 id 且无环；`saturation.min/target` 按章节规划，
  **不要为「全绿」虚增 target**；`keyPoint` 一句话可检验。
- 拆/并 KP 时同步：`questions.kps`、`cards.tags/prereq`、`lessons`、
  `CONTENT-PLAN.md`；用 `validate.py` + `coverage.py` 验证 `unknown_refs=0`、
  无孤儿 KP、无未覆盖 KP。
- 优先「少而准」：已有 KP 能说清的不新增；新增 KP 必须能出题、能判分、
  能对应 `source`。

## 6. 报告模板

```text
## 备课批次：<名称>
- 诊断：<gap/red yellow/质量指标>
- 方案：<新增/修改的 KP、题目、卡片；预期覆盖>
- 变更：<文件 + id 列表>
- 门禁：validate / coverage / selftest / simulate / notation-lint 结果
- 风险与下一批：<未覆盖、需复核的 source、建议>
```

## 7. 硬规则

1. 不靠文本比对判对错：题目正确性必须由课程原文/`source` 与 `rubric` 可核验。
2. 不改 kernel / engine / renderer；发现内容之外的问题，交给 `sokonanoda-dev`。
3. 每批必过 validate / coverage / selftest / simulate / notation-lint；不过不交付。
4. `out/`、`reports/` 不手改；报告可重跑。
5. 大改（≥20 题、拆/并 KP、改章节规划）先给方案，经用户确认再落盘。

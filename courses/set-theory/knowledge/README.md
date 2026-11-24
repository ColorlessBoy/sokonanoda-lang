# 知识库（knowledge）

**内容（文本）是唯一资产**：知识卡、选择题、调度参数都固化在 `data/` 里，可评审、可 diff、
可逐渐优化；PNG / 聊天卡片 / 右栏面板 / 自测页都只是同一份数据的**渲染器**或**消费者**。

## 结构

| 路径 | 作用 |
|---|---|
| `DESIGN.md` | **三层自适应系统设计**（L1 交互 / L2 调度 / L3 维护）+ 阶段验收 |
| `data/knowledge-points.json` | **知识点注册表**（`soko.knowledge-points/1`；kind/chapter/prereq/`saturation.min,target` 四件套） |
| `data/questions.json` | 题库（`soko.choice-bank/1`；choice/short/explain/proof；含 difficulty/variants/prereq/**kps**） |
| `data/cards.json` | 知识卡（`soko.knowledge-cards/1`；含 tags/prereq） |
| `data/params.json` | L2 调度参数（`soko.learn-params/1`：eta/权重/阈值） |
| `engine.py` | **L2 引擎**：掌握度（Elo）+ 推荐式召回 + 去重/变式/复习/知识卡触发；含 `selftest`/`simulate` |
| `validate.py` | 三份数据的结构自检（含交叉引用、权重和、参数范围、**每题 kps 引用/章节归属**），退出码 1 = 不合格 |
| `coverage.py` | **知识点 × 题目覆盖门禁**：按 `saturation` 逐 KP 算红/黄/绿 + 章节汇总 + `reports/coverage-<时间>.json`；有红 exit 1 |
| `CONTENT-PLAN.md` | 覆盖计划：实测缺口、题量估算、分阶段批次与第一批题目清单 |
| `verify_batch.py` | 批次验收：结构/source 行号逐条核验 + 内存注入预测覆盖变化（不改仓库） |
| `review.py` | **L3 复盘**：读导出日志/状态 → 薄弱 tag、错题、下一批建议、内容缺口 |
| `render.py` | 知识卡 → `out/<id>.png`（Chrome 无头截图，2x） |
| `build_quiz.py` | 题库 → `out/quiz.html`（零延迟自测页；离线可用） |
| `draw.py` | 按 chapter/level 抽题（可 `--seed` 复现） |
| `out/` · `reports/` | 渲染产物 / 评测与复盘报告 |

## 工作流

```sh
# 数据自检（改任何 data/*.json 后必跑）
python3 courses/set-theory/knowledge/validate.py
# 知识点覆盖门禁（内容线）：红则 exit 1，报告落 reports/
python3 courses/set-theory/knowledge/coverage.py

# L2 引擎：自检 / 出下一题 / 判卷 / 快照 / 离线评测
python3 courses/set-theory/knowledge/engine.py selftest
python3 courses/set-theory/knowledge/engine.py next   --state /tmp/s.json --seed 7
python3 courses/set-theory/knowledge/engine.py answer --state /tmp/s.json --qid I.2.rel-1 --choice 2
python3 courses/set-theory/knowledge/engine.py state  --state /tmp/s.json
python3 courses/set-theory/knowledge/engine.py simulate --questions 20

# 知识卡渲染 / 自测页 / 抽题
python3 courses/set-theory/knowledge/render.py --all
python3 courses/set-theory/knowledge/build_quiz.py
python3 courses/set-theory/knowledge/draw.py --chapter I.2 --level 2 --count 1 --seed 7

# L3 复盘（Host 导出 JSONL 或直接读状态文件）
python3 courses/set-theory/knowledge/review.py --state ~/.dsh/learn/<sessionId>.json
```

## 约定

1. **不允许现场编内容**：教学/出题只从 `data/` 取；加内容 = 改 JSON → `validate.py` → 再渲染；
2. 答案必须能由课程标准库或单元原文核验（每题 `source` 可查），错项在 `why` 里说明；
3. **调度参数进 `params.json`**、带版本号；评测报告落 `reports/`，内容/参数变更可回溯；
4. 展示形态随场景换（PNG / 聊天卡 / 右栏面板 / 自测页），**数据只有一份**。

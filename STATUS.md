## 当前快照（2026-11-28 · 文本公理 lint 门禁 · #144）

#144：`courses/set-theory/knowledge/text_lint.py` 机检 X1/X3/X4/X5/X6/X7/X8（X2/X9/X10
走 `--review`），`--selftest` 每公理一红一绿；errors 即红，存量 warnings 冻结在
`text-lint-baseline.json`（file+id+field+code，双向棘轮）。公理/误报治理见
`skills/sokonanoda-prep/references/text-axioms.md`；已接 `soko gate` / CI `gates-course` /
`ci-local.sh`。清账：**15/161 → 0/107**（errors 清零；lessons/cards/glossary 0 warning，
余 107 条全在 questions why，按卷冻结）；lessons 89 条提纲式 intuition 改真讲解。

## 当前快照（2026-11-28 · 知识卡匹配改 kp/IDF 评分 · #142）

#142：`explain_for` 改 **kp 优先 → IDF 稀有标签加权 → 稳定破平**（kps 命中数 →
标签数 → 卡片 id，不依赖 `cards.json` 顺序）；`cards.json` 补 `set-pi-lemmas` /
`cardinal-le-mono` 两张真卡（正文取 live 草稿、标签只留描述性的）并为 21 张既有卡补 `kps`；`validate.py` 判卡片
`kps` 引用、`engine.py selftest` 新增 4 条 explain-* 判据并接进 CI `gates-course`。
Before/After 见 `reports/2026-11-28-card-scoring/`。

## 当前快照（2026-11-28 · 快思考调度 v2：单元连续 3 题 + 三题探针）

#138：学习引擎**只改调度与展示**（内核判定与题库不动）——同一单元先连做满 3 题才放开
（`unit_probe_target=3`，`pick` 兜底加 `unit_continuity_bonus`）；一个 KP 至少被探 3 次
（`probe_min_questions=3`）才可能结案；作答 1–2 次的 KP 记「探针中」，不再算已解/薄弱
（`map.min_probes=3`）。插件 progress 卡显示「已解 · 探针中 · 待做 · 薄弱」。判据：
`engine.py selftest`（新增 kp-min-probes / probe-min-questions / probe-unit-continuity /
unit-continuity）、真实 state 回放 3 题同单元、插件 host/client 回归、`validate.py`、
课程门禁 G1–G7；Before/After 见 `reports/2026-11-28-quiz-sched-v2/`。

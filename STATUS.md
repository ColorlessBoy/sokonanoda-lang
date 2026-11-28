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

## 当前快照（2026-11-28 · 知识卡 sections 完整渲染 · #139）

#139：`sokonanoda-dsh` 知识卡按 engine `sections` 顺序渲染**全部**段——`list` 全部
items（不再 `slice(0, 2)`）、`paragraph` 的 `text`、`table` 的 `columns`/`rows`
（与课程海报渲染器同形）都进卡；标题 / 一句话 / `definition.code` / `source` 位置
不变，长卡片换行 + 聊天滚动，不静默截断；纯文本快路径（`cards=false`）同步去掉
sections/items 截断。判据：插件 host 41 / client 47，真 DSH Web Before/After 见
`reports/2026-11-28-knowledge-card-sections/`。

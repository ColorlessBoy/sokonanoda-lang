## 当前快照（2026-11-28 · 知识卡覆盖门禁 + 存量清账 · #146）

#146：`scripts/card-lint.py` 判「选项里点名的裸引理必须出现在 `engine.explain_for` 实际推送卡里」
（`--selftest` 18 条红/绿、`--data-dir` 复现、基线 `card-lint-baseline.json` 双向棘轮；清账
**67 → 0**）。内容：新增 10 张专用卡（`cardinal-le-empty` 等）+ 扩 5 张既有卡；`I.8.le-empty-3`
由通用卡 `cardinal-le` → `cardinal-le-empty`（四条签名齐全）。路由变化 60 fallback→有卡、
0 card→fallback（明细 `reports/2026-11-28-card-coverage-gate/`）。已接 `soko gate` /
CI `gates-course` / `ci-local.sh` + 咬住证明台账；判据与已知限制见
`skills/sokonanoda-prep/references/card-coverage.md`。

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

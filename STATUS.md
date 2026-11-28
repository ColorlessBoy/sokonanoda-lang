## 当前快照（2026-11-29 · 做题内容质量门禁 · 总任务 #147）

#147 把做题内容质量做成可执行门禁：`text_lint.py` 的 options 进扫描（X11 自造术语
error 23→0；X12/X13 按基线冻结）；`card-lint.py` 新增内容规则（覆盖的引理必须给结论/
签名）；新增 `scripts/lesson-preflight.py` + fast path 出题前预检（22 题 / 409 题
0 blocker）；`dsh-images` 支持 `darkPath`；X5-splice 26→0、文本 warnings 200→173
（基线只收紧）。判据：三个 lint 的 `--selftest` + `node test/run.mjs`；均已接
`soko gate`/CI/`ci-local.sh`。Before/After 见
`reports/2026-11-28-quality-gates-umbrella/README.md`。

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



# 知识卡覆盖门禁 + 存量清账（#146）

日期：2026-11-28。本目录是 PR 的 Before/After 证据与读数存档（生成物，不是判据）。

## 判据（一条命令）

```bash
python3 scripts/card-lint.py                  # 门禁：0 处未覆盖（存量基线 0 条，只减不增）
python3 scripts/card-lint.py --selftest       # 红/绿对照 fixtures（18 条：声明解析/点名/表达式/命名空间/基线棘轮）
python3 scripts/card-lint.py --no-baseline    # 诊断：全部列出（复现/复跑用）
```

判据正文、误报治理、已知限制与补卡流程：
`skills/sokonanoda-prep/references/card-coverage.md`。

## Before / After 读数（同一棵树、同一套判据）

| 阶段 | 未覆盖 | 说明 |
|---|---|---|
| 清账前（`HEAD` 的 `cards.json`，即 #146 报告的现场） | **67 处 / 34 题** | `--data-dir <HEAD cards>` 复现；含 `I.8.le-empty-3` 的 4 条（推送 `cardinal-le`，四个引理一个都没讲） |
| 清账后（本分支） | **0 处** | 基线 `scripts/card-lint-baseline.json` = **0 条**（清账后收紧；双向棘轮：新增判红、已修未收紧也判红） |

> 读数口径说明：issue 里原型报 **45 条**（作者工作副本、未提交）；本分支按 issue 工作项 A
> **重写**，口径比原型宽/严各一处：**宽** = 应用形式的点名（`subset_trans α A B C h1 h2`）也算；
> 题目范围 = 全部有 `options` 的题；**严** = 误报治理后只收「有声明 + 命题/谓词 + 非命名空间」
> 的裸引理名（`Set` / `Rel` / `univ` / `Set.Le` 这类类型/命名空间名不计）。所以清账前是 67 而不是 45。

## 反例验证（门禁真的会拦）

```bash
T=$(mktemp -d)
cp courses/set-theory/knowledge/data/*.json "$T"/
git show 95d3cd3e:courses/set-theory/knowledge/data/cards.json > "$T/cards.json"   # 修前 main 的知识卡库
python3 scripts/card-lint.py --data-dir "$T" --no-baseline                          # rc=1，点名 I.8.le-empty-3: le_empty
rm -rf "$T"
```

本分支上这条已写进咬住证明台账：`docs/guard-bite.jsonl` 的 `scripts/card-lint.py` 行（旧 rev
`95d3cd3e` = 修前 main，**不是 `HEAD`**，避免提交后证明自己失效）。

## 选择行为变化（同一份 409 题，Before = `HEAD` cards.json，After = 本分支）

`engine.explain_for` 的评分代码**一个字没改**（#142 已改）；变化只来自卡片库。共 **73 道题**
的选择结果变了：

- **60 道 fallback → 有卡**（本批补的 10 张新卡由 `kps` 钉住，见下），例如
  `II.1.natadd-3` fallback → `nat-add-lemmas`；
- **13 道 card → card**：`I.8.le-empty-1/2/3`、`I.8.mix-1/3` 由通用卡 `cardinal-le` → 专用卡
  `cardinal-le-empty`（本单的验收线）；其余 8 道按 `kps` 落到更贴题的新卡/扩卡
  （`addwd-5`→`nat-add-lemmas`、`addgroup-2/3`/`negwd-4`→`int-add-laws`、`orderwd-3`→
  `nat-order-lemmas`、`mulcomm-3`/`mulwdsnd-2/3`→`int-mul-comm`）；
- **0 道 card → fallback**（没有把已有卡弄丢）。

逐条清单见 `routing-diff.txt`；`engine.py selftest` 全过（含既有 explain-* 判据）。

## Before / After 截图（同一 `render.py`、同视口 1000px 卡面、2x）

内容资产变化会直接改变用户看到的知识卡，按 #145 同一口径给同一张卡的改前/改后渲染
（改前用 `git show HEAD:.../cards.json` 的数据 + 未改动的 `render.py`）：

1. **本单的验收线**：`I.8.le-empty-3` 从通用卡换成专用卡。
   - [before：`cardinal-le`](before-I.8.le-empty-3.cardinal-le.png) ·
     [after：`cardinal-le-empty`](after-I.8.le-empty-3.cardinal-le-empty.png)
   - 可见变化：改前只讲 `≼` 本身（「三个层次 / 常见误区 / 怎么用」），四个选项引理
     `le_empty` / `subset_of_le_empty` / `subset_le` / `le_trans` 一条签名都没有；改后
     `definition.code` 直接给出四条签名，并逐条说「哪个方向对不上」。
2. **扩卡**：`I.2.partial-2` 的推送卡内容变化（`partial-order` 追加「同名引理速查」）。
   - [before](before-I.2.partial-2.partial-order.png) ·
     [after](after-I.2.partial-2.partial-order.png)
   - 可见变化：末尾多了 `partialOrder_refl` / `partialOrder_antisymm` /
     `partialOrder_trans` / `linearOrder_total` / `strictOrder_irrefl` / `subset_trans`
     六条取用子的签名与用法。

`engine.explain` 原样输出见 `before-*.json` / `after-*.json`，读数见 `verify.txt`。

## 本批改了什么

- 新增 `scripts/card-lint.py`（判据 + `--selftest` + `--data-dir` + 基线双向棘轮）与
  `scripts/card-lint-fixtures.json`（18 条红/绿对照）；基线
  `scripts/card-lint-baseline.json`（清账后 0 条）。
- 清账（67 → 0）：新增 10 张卡 `cardinal-le-empty` / `ordinal-basics` / `subset-lemmas` /
  `rel-transitivity` / `le-both` / `nat-add-lemmas` / `nat-mul-lemmas` / `nat-order-lemmas` /
  `int-add-laws` / `int-mul-comm`；扩 5 张既有卡（`partial-order` / `antisymmetric` /
  `linear-order` / `inverse` / `quot-lift-well-defined`）。
- 判据正文/误报治理/已知限制：`skills/sokonanoda-prep/references/card-coverage.md`；同步
  `skills/sokonanoda-prep/SKILL.md`、`.agents/skills/sokonanoda-prep/SKILL.md`、
  `.opencode/agent/prep.md`、知识库 `README.md` / `SCHEMA.md` / `DESIGN.md` / `CONTENT-PLAN.md`。
- 接线：`scripts/soko gate`（含 `--fast`）、CI `gates-course`、`scripts/ci-local.sh` 都跑
  lint + `--selftest`（缺脚本/解释器/数据 = exit 3，不静默跳过）；守卫咬住证明进
  `docs/guard-bite.jsonl` + `docs/guard-anchors.jsonl`。

## 已知限制（v1 有意划的边界，详见 `card-coverage.md` §5）

- 只查**不带命名空间**的引理名（`Set.Le.refl` / `Set.mem_image_elim` 这类限定名留待后续批次）；
- 判定按**裸名**匹配，跨模块同名引理会被当作同一个名字（`le_refl` 既是 `≼` 的自反定理也是
  `LeNat` 构造子）；
- 声明来源是 `courses/set-theory/{lib,units}` 的行首 `theorem` / `def` 正则，不是真 parser。

## 复跑

```bash
python3 scripts/card-lint.py --selftest
python3 scripts/card-lint.py
python3 courses/set-theory/knowledge/validate.py
python3 courses/set-theory/knowledge/engine.py selftest
python3 courses/set-theory/knowledge/text_lint.py
python3 scripts/notation-lint.py
python3 scripts/soko gate --fast
```

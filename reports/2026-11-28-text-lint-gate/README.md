# 文本公理 lint 门禁 + 存量清账（#144）

日期：2026-11-28。本目录是 PR 的 Before/After 证据与读数存档（生成物，不是判据）。

## 判据（一条命令）

```bash
python3 courses/set-theory/knowledge/text_lint.py            # 0 errors（存量 warnings 基线只减不增）
python3 courses/set-theory/knowledge/text_lint.py --selftest # 红/绿对照 fixtures：X1/X3/X4/X5/X6/X7/X8
```

## Before / After 读数（同一棵树、同一套判据）

| 阶段 | errors | warnings | 说明 |
|---|---|---|---|
| 清账前 | 15 | 161 | 当前树按**正规化后**判据实测（原型 v1 在作者工作副本上是 53/208，含标题/代码撑长等误报，口径不同） |
| 清账后 | **0** | **107** | errors 清零；剩余 107 条全在 `questions.json` 的 `why`（X5-long 81 / X5-splice 26），按卷冻结在 `text-lint-baseline.json` |

按文件（清账前 → 清账后）：

| 文件 | errors | warnings |
|---|---|---|
| `lessons.json` | 7 → 0 | 40 → 0 |
| `cards.json` | 2 → 0 | 8 → 0 |
| `glossary.json` | 1 → 0 | 4 → 0 |
| `questions.json` | 5 → 0 | 109 → 107（why 长句/逗号粘连，按卷冻结基线，下一内容批次逐步改短） |

> 注：上表按最终判据口径统计（占位符折 1 字、标题/列表续行/题干/术语行误报治理之后）。

## 反例验证（lint 真的会拦）

```bash
T=$(mktemp -d)
printf '{"cards":[{"id":"fixture","oneLiner":"费马（约 1659）：要证『不存在』，"}]}' > "$T/broken.json"
python3 courses/set-theory/knowledge/text_lint.py --json "$T/broken.json" --no-baseline   # exit 1
printf '{"cards":[{"id":"ok","oneLiner":"费马大约在 1659 年想到无穷递降法。"}]}' > "$T/good.json"
python3 courses/set-theory/knowledge/text_lint.py --json "$T/good.json" --no-baseline      # exit 0
```

## Before / After 截图（同渲染器、同视口 1000px 卡面、2x）

内容资产变化会直接改变用户看到的知识卡文本，所以给**同一张卡**的改前/改后渲染：

- 选择公理（`choice-axiom`）：[before](before-choice-axiom.png) · [after](after-choice-axiom.png)
- 类型与命题（`types-101`）：[before](before-types-101.png) · [after](after-types-101.png)

改前用 `HEAD:data/cards.json` + 未改动的 `render.py` 渲染；改后用工作区 `data/cards.json` 渲染。
可见差异：一句话摘要从「这件事，本身不给你…；要用它，就…」拆成两句；「先分清『有』和
『拿得到』」段补了具体对比；「两条纪律」的长句拆句后不再一行挤满；知识卡内容更易一遍读懂。

## 本批改了什么

- 新增 `courses/set-theory/knowledge/text_lint.py`（X1–X10 机检 + `--selftest` + `--review` +
  `--tex` + 基线棘轮）；`text-lint-fixtures.json`（每条公理红/绿对照）；
  `text-lint-baseline.json`（存量 warnings 冻结，只减不增）。
- 公理正文、误报治理、基线与批次流程：`skills/sokonanoda-prep/references/text-axioms.md`；
  同步 `SKILL.md`、`.agents/skills/sokonanoda-prep/SKILL.md`、`.opencode/agent/prep.md`、
  知识库 `README.md` / `STYLE.md` / `SCHEMA.md` / `CONTENT-PLAN.md`。
- 接线：`scripts/soko gate`（含 `--fast`）、CI `gates-course`、`scripts/ci-local.sh` 都跑
  lint + `--selftest`（探不到脚本/解释器 = exit 3，不静默跳过）。
- 清账：lessons 89 条 `intuition`（77 条「本单元围绕…展开，核心是：…」提纲改真讲解 +
  12 条拆句/补白话）、cards 9 处、glossary 5 处、questions 8 条 `why`。

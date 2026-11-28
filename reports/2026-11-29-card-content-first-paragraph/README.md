# #150 补验收：知识卡首段先答「它是什么」

日期：2026-11-29。分支 `agent/card-content-lint-84ee338c8685`（基于 PR #154 head `17416766`
的补丁，PR #154 合并后 rebase 到 main）。本目录是补丁的读数存档（生成物，不是判据）。

## 为什么补

#150 的期望 2 与验收清单要求：

- 期望 2：卡片**首段先回答「它是什么」**（`definition.code` 或首段的定义句），再讲怎么用；
- 验收：红/绿 fixtures 覆盖「裸名字」「有签名」「**首段没答是什么**」三态。

PR #154 的 `card-lint.py` 内容规则只覆盖了「任何一段给出结论/签名即绿」（前两态），
没有「首段」判据与第三态 fixture。本补丁在同一个 `card-lint.py` 上补判据与夹具
（`contentCases` 4 → 7 条；`--selftest` 23 → 26 条对照）。

## Before / After（同一棵树、同一套判据）

| 判据 | Before（PR #154 head `17416766`） | After（本补丁） |
|---|---|---|
| `python3 scripts/card-lint.py --selftest` | SELFTEST OK（23 条；`contentCases` 4 条） | SELFTEST OK（**26** 条；`contentCases` **7** 条） |
| 「首段没答是什么」红 fixture | **不咬**（该 fixture 判绿 ⇒ selftest 1 条失败，见下） | **咬**（红例通过） |
| `python3 scripts/card-lint.py` | 0 违例（覆盖 0 · 内容 0） | 0 违例（覆盖 0 · 内容 0）；基线仍 0 条 |
| 被覆盖的 `(qid, 引理)` 配对 | 69 | 69（49 条在 `definition.code`；20 条非 code，首次出现即定义句） |
| 需要改内容的卡片 | — | **0 张**（受影响 card id：无） |
| 目标题 `I.8.le-empty-3` 推送卡 | `cardinal-le-empty`，四条签名在 `definition.code` | 同（不变） |
| learner path（真实 state，I.8/I.9 未作 22 题） | 0 blocker / 18 warnings | 0 blocker（恒为 0；warnings 随真实 state 变：`01:24` 读数 22 题 / 0 blocker / 15 warnings） |

## 反例验证（守卫能咬住漏掉的态）

把本补丁的 fixtures 换给 **PR #154 head 的实现**（只换夹具、不换实现）：

```bash
cd /tmp/pr154-verify                       # = PR #154 head 17416766
cp <本分支>/scripts/card-lint-fixtures.json scripts/card-lint-fixtures.json
python3 scripts/card-lint.py --selftest; echo "rc=$?"
```

实测输出：

```
SELFTEST FAIL
  - 红：首段先讲怎么用、定义在后面才补（首段没答「它是什么」）：期望
    [('fixture.content-red-first-paragraph', 'foo_lemma')]，实际 []
（25 条对照通过，1 条失败）
rc=1
```

⇒ 旧实现确实漏了「首段没答是什么」；本补丁让它咬住（本分支同命令 rc=0）。

## 复现（本分支）

```bash
python3 scripts/card-lint.py --selftest && python3 scripts/card-lint.py
python3 scripts/lesson-preflight.py --selftest
python3 scripts/lesson-preflight.py --limit 22 --json   # learner path：0 blocker（warnings 随 state 变）
python3 scripts/lesson-preflight.py --all               # 409 题：0 blocker / 92 warnings
python3 scripts/card-lint.py --json-out                 # {"violations":0,"coverage":0,"content":0,...}
```

计数复现（被覆盖配对的存放位置）：

```bash
python3 - <<'PY'
import importlib.util, json, sys
from pathlib import Path
repo = Path('.').resolve()
spec = importlib.util.spec_from_file_location('card', repo/'scripts/card-lint.py')
cl = importlib.util.module_from_spec(spec); spec.loader.exec_module(cl)
sys.path.insert(0, str(repo/'courses/set-theory/knowledge'))
import engine
data = engine.load_data(repo/'courses/set-theory/knowledge/data')
cand, _ = cl.collect_declarations(repo/'courses/set-theory')
pairs = code = noncode = 0
for q in data['questions']:
    names = cl.named_lemmas(q, cand)
    if not names:
        continue
    res = engine.explain_for(engine.new_state(), data, q['id'])
    if res.get('kind') != 'knowledge-card':
        continue
    card = res['card']
    text = json.dumps(card, ensure_ascii=False)
    for name in names:
        if name not in text:
            continue
        pairs += 1
        if name in str((card.get('definition') or {}).get('code') or ''):
            code += 1
        else:
            noncode += 1
print('covered pairs:', pairs, '| in definition.code:', code, '| non-code:', noncode)
PY
# covered pairs: 69 | in definition.code: 49 | non-code: 20
```

## 边界与已知限制

- 只改判据 / fixtures / 文档；**不改 `cards.json` 内容**（新判据下 0 违例，无需改卡，
  未触碰 #147「≥20 张先给批次方案」的边界）。
- **STATUS.md 未动**：它属于入口文档，docs-lint ① 的 entry 预算只剩 **21 B**
  （134979/135000）、活文档总量只剩 60 B（4499940/4500000）；本轮 #147 快照已覆盖本补丁
  所在的批次，故不新增行（加一行会直接把 docs-lint 判红）。
- 「首段」是**文本顺序启发式**（`oneLiner` → `sections`），不读渲染后的视觉版式；
  `definition.code` 里有签名的一律算讲清楚（`definition.code` 优先于 oneLiner 的用法句）。
- 判据正文已同步到 `skills/sokonanoda-prep/references/card-coverage.md` §7
  （新增第 3 条「首段先答它是什么」、fixtures 七态、已知限制）。

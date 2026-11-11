# unit⑧ Duolingo-style lesson prototype (PR #36)

Static, single-file lesson player validating the learner-facing UX (no Lean editor).

- **v2** answered «integer 在形式化证明里的表示太模糊»: the integer-as-pair idea is
  taught explicitly with blue/red counter chips **before** negation and addition.
- **v3** answers «概念在形式化证明里的实现之间比较模糊»: **every step** now has a
  visible **「直觉 ↔ 形式」** one-line summary. Opening it shows a concept ↔ exact
  identifier matrix plus a small formatted Lean snippet (never a free-form editor).
  The collapsed formal proof at the end remains the evidence appendix.

```bash
python3 scripts/lesson_proto/build.py --out dist/lesson/unit8
python3 scripts/lesson_proto/build.py --check --out dist/lesson/unit8
python3 scripts/lesson_proto/build.py --selftest          # negative validation guard
open dist/lesson/unit8/index.html                          # macOS
```

## v3 flow — 6 steps, 4 graded, 6 mapping blocks

| # | Step | Interaction | 直觉 ↔ 形式 bridge |
|---|------|-------------|--------------------|
| 1 | 目标卡 | 点温度数轴 +3 / −3 | `x + (-x) = 0` ↔ `int_add_neg`, bridged by `Quot.sound`; note says 零元/负元在代表元层陈述 |
| 2 | **为什么成对**（无判分题） | 读蓝/红计数器卡片，继续 | `(a,b)` ↔ `Prod Nat Nat` / `@prod_mk Nat Nat a b`; 蓝=`Prod.fst`, 红=`Prod.snd`; `a−b` ↔ `intRel`; 整数 ↔ `Quot intRel` |
| 3 | 同一个整数，多种配对 | 点选 `(6,3) / (4,2) / (2,5)` | `(3,0) ~ (4,1)` ↔ `intRel` cross-sum `3+1 = 4+0`; 一个整数 = `Quot intRel` 的一个等价类 |
| 4 | 取负 = 交换计数器 | 点选 `(0,3)` | `(3,0) → (0,3)` ↔ `negRep p := (snd p, fst p)`; `negRep_wd` 说明良定义 |
| 5 | 相加 = 同色计数器合并 | 点选 `0` | `(a,b)+(c,d)` ↔ `addRep`; `(3,0)+(0,3)=(3,3)` ↔ `add_neg_rep` → `int_add_neg` via `Quot.sound` |
| 6 | 结论 / 徽章 | 点选成立的陈述 | 概念 / 形式对象 / 引理表（`intRel`, `Quot intRel`, `int_add_zero`, `negRep_wd`, `addRep`, `add_neg_rep`, `int_add_neg`, `Quot.sound`）+ collapsed formal proof |

`Quot.sound` is taught in step 5 as exactly what it is: *equal representatives give
equal integers*. The last step's table ties every concept to a formal object and a
lemma, so the learner never has to jump from chips straight into a collapsed proof.

The pair explanation is still the centerpiece: every pair visual is drawn as real
blue/red chips, cancelled pairs are struck out, and the addition board turns 3 blue +
3 red into 0. Gamification is unchanged: progress, XP (max 95), streak, first-try
bonus, 3-tier hints, 20-second symbol cards, completion badge, collapsed formal proof.

## Mapping-block contract (`unit8.json` `formal_map`)

Every step must provide:

- `summary` — the always-visible one-liner, containing `↔` and ≤ 72 chars;
- `columns` — `["直觉","形式实现","标识符"]`, or `["概念","形式对象","引理"]` on the
  conclusion step;
- `note` — the plain-language caveat under the summary;
- `rows` — at least 3 (7 on the conclusion) `{concept, formal, lemma}` rows, where
  `lemma` must be an exact identifier declared in the unit;
- `snippets` — 1–2 small formatted Lean fragments (1–12 non-empty lines each), with a
  title and caption, never a full proof or an input box.

`build.py` validates that every expected identifier appears **in a row**, not merely in
prose or a snippet. `python3 scripts/lesson_proto/build.py --selftest` proves the guard
bites: it mutates the data five ways (missing `formal_map`, wrong identifier, missing
snippets, missing rows, identifier only in prose) and each mutation must be rejected.

Manual negative mutation (must exit 1):

```bash
python3 - <<'PY'
import json
data = json.load(open("scripts/lesson_proto/unit8.json", encoding="utf-8"))
del data["steps"][4]["formal_map"]
json.dump(data, open("/tmp/unit8-bad.json", "w", encoding="utf-8"), ensure_ascii=False)
PY
python3 scripts/lesson_proto/build.py --check --data /tmp/unit8-bad.json --out /tmp/lesson-bad
# error: lesson.steps[4].formal_map is required ...  → rc=1
```

## Screenshot + acceptance (390px)

```bash
node scripts/lesson_proto/shot.mjs
# writes scripts/lesson_proto/screenshots/{first,representation-map,formal-map,complete}.png
# and /tmp/lesson-v3-{first,representation-map,formal-map,complete}.png
```

The script drives real clicks in headless Chrome and asserts:

- 390px viewport, zero document horizontal overflow and no element crossing the viewport,
  **including with the mapping block expanded**;
- zero external requests (`file:` only) and zero page/console errors;
- 6 mapping blocks exist, start collapsed, expose a one-line summary whose full text is
  also the `title`, and expand on a real summary click;
- mapping rows carry the exact identifiers, the conclusion table has the three
  concept/formal-object/lemma columns, and snippets render as `<pre><code>` with no
  `textarea`/`input`/`contenteditable`;
- the full `goal → why-pairs → pair-family → pair-swap → pair-sum → conclusion` flow,
  `window.SokoLesson.steps === 6`, completion XP `95/95`;
- a wrong answer (`(4,2)`) keeps the step, gives specific feedback, and awards no XP.

Chrome is found at the macOS default path; override with `SOKO_CHROME=/path/to/chrome`.
No npm dependencies.

## Limits

- UX prototype only: the answer key lives in `unit8.json`; no in-browser kernel grading yet.
- `build.py --check` validates the 6-step content contract (step order, answers, choices,
  blue/red card counts, cancellation rows, XP total, plus the v3 mapping contract) and
  that the hidden theorem names still exist in
  `courses/analysis/units/II.1/unit08-int-add-group.sokonanoda`.
- `dist/` is generated and not committed; `preview/index.html` is the committed copy used
  by the raw.githack review link.

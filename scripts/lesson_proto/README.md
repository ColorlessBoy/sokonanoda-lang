# unit⑧ Duolingo-style lesson prototype (PR #36)

Static, single-file lesson player validating the learner-facing UX (no Lean editor).
**v2** answers the owner feedback «integer 在形式化证明里的表示太模糊»: the
integer-as-pair idea is now taught explicitly, with blue/red counter chips, **before**
negation and addition.

```bash
python3 scripts/lesson_proto/build.py --out dist/lesson/unit8
python3 scripts/lesson_proto/build.py --check --out dist/lesson/unit8
open dist/lesson/unit8/index.html   # macOS
```

## v2 flow — 6 steps, 4 graded

| # | Step | Interaction | Math |
|---|------|-------------|------|
| 1 | 目标卡 | 点温度数轴 +3 / −3 | `x + (-x) = 0` |
| 2 | **为什么成对**（new, 无判分题） | 读蓝/红计数器卡片、点继续 | 只有自然数 ⇒ 用 `(a,b)` 表示 `a − b`；`(3,0)=3`，`(0,3)=−3` |
| 3 | **同一个整数，多种配对**（new） | 点选 `(6,3) / (4,2) / (2,5)` | `(3,0) = (4,1) = (5,2) = 3`；一蓝一红抵消；换代表元结果不变（良定义） |
| 4 | 取负 = 交换计数器 | 点选 `(0,3)` | `(3,0) → (0,3)` |
| 5 | 相加 = 同色计数器合并 | 点选 `0` | `(3,0)+(0,3)=(3,3)`；3 对抵消 ⇒ 0 |
| 6 | 结论 / 徽章 | 点选成立的陈述 | `x + (-x) = 0` → **整数加法有逆元 ✓** |

The pair explanation is the centerpiece: every pair visual is drawn as real blue/red
chips, cancelled pairs are struck out, and the addition board turns 3 blue + 3 red
into 0. Gamification is unchanged: progress, XP (max 95), streak, first-try bonus,
3-tier hints, 20-second symbol cards, completion badge, collapsed formal proof
(theorem names + plain-language notes only).

## Screenshot + acceptance (390px)

```bash
node scripts/lesson_proto/shot.mjs
# writes scripts/lesson_proto/screenshots/{first,representation,family,complete}.png
# and /tmp/lesson-v2-{first,representation,family,complete}.png
```

The script drives real clicks in headless Chrome and asserts:

- 390px viewport, zero document horizontal overflow and no element crossing the viewport;
- zero external requests (`file:` only) and zero page/console errors;
- the full `goal → why-pairs → pair-family → pair-swap → pair-sum → conclusion` flow,
  `window.SokoLesson.steps === 6`, completion XP `95/95`;
- a wrong answer (`(4,2)`) keeps the step, gives specific feedback, and awards no XP.

Chrome is found at the macOS default path; override with `SOKO_CHROME=/path/to/chrome`.
No npm dependencies.

## Limits

- UX prototype only: the answer key lives in `unit8.json`; no in-browser kernel grading yet.
- `build.py --check` validates the 6-step content contract (step order, answers, choices,
  blue/red card counts, cancellation rows, XP total) and that the hidden theorem names
  still exist in `courses/analysis/units/II.1/unit08-int-add-group.sokonanoda`.
- `dist/` is generated and not committed.

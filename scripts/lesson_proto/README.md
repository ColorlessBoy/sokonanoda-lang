# unit⑧ Duolingo-style lesson prototype (#34)

Static HTML prototype validating the learner-facing lesson UX (no Lean editor):

```bash
python3 scripts/lesson_proto/build.py --out dist/lesson/unit8
python3 scripts/lesson_proto/build.py --check --out dist/lesson/unit8
open dist/lesson/unit8/index.html   # macOS
```

- 5 interactive steps: goal/intuition -> opposite of 3 -> pair swap -> pair addition -> conclusion + badge.
- Gamification: progress, XP, streak, first-try bonus, retry, 3-tier hints, completion badge.
- Language help: 5 symbol cards (20s auto-dismiss); formal proof collapsed by default.
- No external assets; mobile-first; native buttons + keyboard.
- `unit8.json` is the future DSH client-plugin content protocol; next: package `app.js` as a DSH client plugin and replace the static answer key with WASM/CLI kernel grading.

Screenshots: `screenshots/first.png`, `screenshots/correct.png`, `screenshots/complete.png`.

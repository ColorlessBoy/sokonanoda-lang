# Launch kit — sokonanoda v0.100.0

Everything needed to launch **sokonanoda v0.100.0** (released 2026-11-27) on
Hacker News, Product Hunt and Chinese platforms, plus the visuals to go with it.

**Nothing here is auto-published.** The owner posts from their own accounts;
this kit is the reviewed copy, the assets and the runbook.

## Status at a glance

| Piece | State |
|---|---|
| v0.100.0 release | published (GitHub Release + tag) |
| VS Code Marketplace | live at 0.100.0 (verified through the Marketplace API) |
| Website | live at https://colorlessboy.github.io/sokonanoda-lang/ |
| Copy | Show HN · Product Hunt · V2EX · 知乎 · 即刻 · 小红书 (optional) |
| Visuals | 7 generated assets + the 7 s demo GIF (see `assets/`) |
| Site promo surface | hero + GIF recaptured at 0.100.0; `og:image` sharing card added |
| Not done | actual posting (needs the owner's accounts); narrated video (script only) |

## What the owner still needs

1. **Accounts.** HN (an established account is much safer than a brand-new one),
   Product Hunt maker account + product page, V2EX (confirm the account can post in
   分享创造), 知乎, 即刻. Optional: 小红书.
2. **A launch day.** Block it for replies. HN and Product Hunt each need a full day.
3. **The posts themselves.** Copy is ready to paste; do not add claims that are not
   in the claim ledger.

## Recommended order

Do **not** launch HN and Product Hunt on the same day: both need fast replies, and
you cannot give either a full day if they overlap. Suggested week:

| Day | Platform | Local time (Asia/Shanghai) | Why |
|---|---|---|---|
| Tue | **Hacker News** | 21:00–23:00 (08:00–10:00 ET) | Technical feedback first; FAQ answers can still change before the other posts. |
| Wed | **V2EX + 知乎** | 20:00–22:00 | Chinese long-form; use the HN questions to sharpen the FAQ. |
| Wed/Thu | **即刻** (+ optional 小红书) | 21:00-ish | Short social; can reuse the same screenshots. |
| Thu | **Product Hunt** | goes live 16:01 (00:01 PT); reply through the PT day | Last, with the polished story and any early traction. |

If only one launch is possible: **Hacker News first**. It is the audience most
likely to test the kernel claim and give useful criticism.

## Files

| File | Platform | Language |
|---|---|---|
| `show-hn.md` | Hacker News (Show HN) | English |
| `product-hunt.md` | Product Hunt listing + first comment | English |
| `zh-v2ex.md` | V2EX 分享创造 | Chinese |
| `zh-zhihu.md` | 知乎文章 | Chinese |
| `zh-jike.md` | 即刻 / 朋友圈 / 小红书 | Chinese |
| `faq.md` | answers for all platforms | English |
| `demo-script.md` | 45–60 s video + 15 s silent cut | English + Chinese |
| `cards/` | HTML/CSS sources + `render.sh` | — |
| `assets/` | ready-to-upload PNG/GIF | — |

### Assets

| File | Size | Use |
|---|---|---|
| `assets/og-card.png` | 1200×630 | link preview / `og:image` (also `site/assets/og-card.png`) |
| `assets/ph-1-hero.png` | 1270×760 | Product Hunt gallery 1 |
| `assets/ph-2-kernel.png` | 1270×760 | Product Hunt gallery 2 |
| `assets/ph-3-courses.png` | 1270×760 | Product Hunt gallery 3 |
| `assets/zh-cover.png` | 1200×630 | V2EX / 知乎 cover |
| `assets/portrait.png` | 1080×1350 | 即刻 / 小红书 / mobile |
| `assets/icon-240.png` | 240×240 | Product Hunt thumbnail |
| `assets/demo-vscode.gif` | 843×390, 7 s | demo loop (copy of the site GIF) |

All cards are rendered from real content: the screenshot comes from the published
extension at v0.100.0, the numbers come from the repository, and the copy lives in
the HTML. Regenerate with `cards/render.sh`.

## Pre-flight checklist

Run these before posting; they are the same facts the copy depends on:

```bash
# 1. version + tag
grep -m1 '^version' Cargo.toml
gh release view v0.100.0 --json tagName,publishedAt --jq '.tagName+" "+.publishedAt'

# 2. Marketplace really serves 0.100.0
curl -s -X POST 'https://marketplace.visualstudio.com/_apis/public/gallery/extensionquery' \
  -H 'Content-Type: application/json' -H 'Accept: application/json;api-version=7.2-preview.1' \
  --data '{"filters":[{"criteria":[{"filterType":7,"value":"sokonanoda-lang.sokonanoda"}]}],"flags":914}' \
  | python3 -c 'import json,sys; print(json.load(sys.stdin)["results"][0]["extensions"][0]["versions"][0]["version"])'

# 3. counts used in the copy
python3 - <<'PY'
import json
m = json.load(open("courses/exercise-dataset/data/manifest.json"))
print("totals", m["totals"])
for c in m["courses"]:
    print(c["id"], c["units"], c["exercises"], c["hints"])
PY

# 4. quiz bank
python3 - <<'PY'
import json
q = json.load(open("courses/set-theory/knowledge/data/questions.json"))["questions"]
k = json.load(open("courses/set-theory/knowledge/data/knowledge-points.json"))["kps"]
print("questions", len(q), "knowledge points", len(k))
PY

# 5. Lean Kernel Arena numbers
python3 -c 'import json;d=json.load(open("site/data/arena.json"));print(d["corpus"],d["accepted"],d["good"],d["rejected"],d["bad"],d["slow_timeouts"],d["known_unsupported"])'

# 6. platform packages
gh release view v0.100.0 --json assets --jq '[.assets[].name] | length'

# 7. site + docs gates
python3 scripts/check-site.py
python3 scripts/docs-lint.py
```

Expected at v0.100.0: 298 units / 2,022 exercises / 5,030 hints; 66 + 910 + 1,046;
409 questions / 195 knowledge points; arena `master@30e64c1b2` 135/138 accept and
90/92 reject (3 timeouts, 2 unsupported); 26 release assets (9 VSIX packages
including the universal fallback + 8 CLI + 8 LSP tarballs + SHA256SUMS).

## Claim ledger

| Claim in the copy | Source of truth |
|---|---|
| "v0.100.0" | `Cargo.toml` + tag + Marketplace API |
| "298 units / 2,022 exercises" | `courses/exercise-dataset/data/manifest.json` (`totals`) |
| "11 / 66, 112 / 910, 175 / 1,046" | same manifest, per course |
| "5,030 hints" | same manifest (`totals.hints`) |
| "409 quiz questions / 195 knowledge points" | `knowledge/data/{questions,knowledge-points}.json` |
| "135/138 accept, 90/92 reject" | `site/data/arena.json` (corpus `master@30e64c1b2`) |
| "8 platform packages + universal" | `gh release view v0.100.0` assets |
| "VS Code extension ships LSP + CLI, works offline" | `editor/vscode/README.md` |
| "no mathlib / no Lean imports / subset" | `README.md`, `docs/architecture.md` |
| "Apache-2.0" | `LICENSE`, `package.json` |
| "kernel is a fork of intgrah/sokonanoda @ 7b51784" | `NOTICE.md` |

If any count moves before launch, update the copy **and** the card (the numbers are
also baked into `ph-3.html`). Pin to the v0.100.0 tag if main has moved.

## Guardrails

- **No vote solicitation.** No "please upvote", no group-chat voting, no
  incentivised shares. HN and Product Hunt both penalise it, and it can get the
  product delisted.
- **One post per platform.** Do not repost the same URL after a weak launch; wait
  weeks and use a materially different angle.
- **Disclose, don't oversell.** The copy says what it is not: not Lean, no mathlib,
  Chinese-first courses, kernel not formally verified. Keep those lines.
- **No fake evidence.** Do not add testimonials, user counts, benchmarks or
  screenshots that are not in this kit. The current Marketplace install count is
  small; do not imply traction.
- **Answer in-thread.** Link `faq.md` only for long answers; short, specific replies
  do better.
- **Privacy.** Do not post private Feishu content, tokens, local paths or personal
  data. The screenshot is from the public course files and the published extension.

## Known limitations (disclose when asked)

- Not Lean: no mathlib, no Lean imports, only the tactics the courses use.
- Courses are Chinese-first; only the intro course has an English mirror.
- The kernel passes the arena corpus but is not formally verified.
- No structure/typeclass/macro support; the language is a teaching subset.
- The analysis course has explicit mathematical boundaries (e.g. the general
  antiderivative form of Newton–Leibniz is not proved at the `ratRep` layer).

## Site changes in this kit

- `site/assets/hero-vscode{,-dark}.png` recaptured at **v0.100.0** (the previous
  head image still said server 0.87.1).
- `site/assets/demo-vscode{,-dark}.gif` re-recorded at **v0.100.0** (the previous
  loop still said server 0.98.2).
- `site/index.html` now has `og:image` / `twitter:image` pointing at the sharing
  card; before this there was no preview image.
- `scripts/check-site.py` now allows and validates the one sharing-card bitmap
  (1200×630, same size budget), so an unmanaged bitmap still fails the gate.

## Regenerating

```bash
launch/cards/render.sh          # needs Chrome, ffmpeg, sips (macOS); rewrites assets/ + site og-card
node scripts/site-screenshot.mjs && node scripts/site-screenshot.mjs --theme dark
node scripts/site-demo.mjs && node scripts/site-demo.mjs --theme dark
python3 scripts/check-site.py
```

The screenshot and GIF scripts refuse a dirty worktree and verify the pixels via
Vision OCR; the card script fails if Chrome returns a wrong-sized image.

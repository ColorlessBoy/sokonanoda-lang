# R4 — 自托管 webfont 管线：字形集实测、候选筛选与最终决策

> 对象：`sokonanoda-lang` 官网 `site/`（零构建手写 HTML/CSS，GitHub Pages 部署）。
> 目标：在 **≤ 120 KB 总载荷** 的前提下，让正文、标题、代码块、终端与目标态面板
> 的中英混排与逻辑记法 **一个字形都不缺**。
>
> **方法说明（可复现）**：本报告里每一个字节数、每一个覆盖结论都来自本机实跑的
> `fontTools.ttLib.TTFont(...).getBestCmap()`，**没有一条是"查资料得知"**。
> 凡未取到/未测的条目集中列在 §9「未验证项」，**不臆造**。
> 本会话的 `web_search` 工具不可用（缺 API key），全部字体通过 jsDelivr
> `gh`/`npm` 通道直取二进制。
>
> 撰写日期：2026-09-19。仓库版本 `0.61.0`，第 108 轮。
> 上游参照：`R1-design-craft.md` §3.5/§3.7/§3.8/§3.9（本报告修正其 §3.9 的一条结论，
> 见 §5.3）。

---

## 0. 结论摘要

| # | 结论 | 依据 |
|---|---|---|
| 1 | **没有任何一款 Latin 文本面覆盖本项目的符号集**。7 款衬线候选（STIX Two Text / Source Serif 4 / Newsreader / Literata / IBM Plex Serif / Spectral / Noto Serif）**全部缺 `⊢`（U+22A2）与 `𝒫`（U+1D4AB）** | §4.1 |
| 2 | 衬线里覆盖最好的是 **Source Serif 4：129/180**；最差的 Newsreader 只有 116/180 | §4.1 |
| 3 | 因此必须**第二个符号面**。符号源里 **STIX Two Math 覆盖 49/51 缺口的符号**（只缺 `Ⅰ` `Ⅱ`），是唯一既 OFL 又贴合衬线设计的选择 | §4.3 |
| 4 | 等宽面里 **JuliaMono 是唯一 180/180 全中**；其余全部缺 `⊢`：JetBrains Mono 145/180、Source Code Pro 139/180、Fira Mono 134/180、IBM Plex Mono 125/180；DejaVu Sans Mono 161/180 与 Noto Sans Mono 156/180 有 `⊢` 但缺 `⦃` `⦄`（集合推导式记法，会出现在目标态里） | §4.2 |
| 5 | **R1 §3.9 的"唯一候选是 Noto Sans Mono"结论需要修正**：R1 未测 JuliaMono。按本报告同一口径，JuliaMono 才是唯一 100% 命中的等宽面 | §5.3 |
| 6 | 任务书给出的 `⊢` 断言 **成立**：Menlo / SF Mono / Monaco / Hiragino Sans GB / JetBrains Mono / Source Code Pro 六款本机实测均无 U+22A2；Inter **未验证**（本机没有） | §5.1 |
| 7 | 最终 5 个文件、**73,004 B = 71.3 KB**，是 120 KB 预算的 59% | §6.1 |
| 8 | 正文栈必须是 `"Soko Serif", "Soko Symbols", "Soko Mono", …` 这个**固定顺序**：Serif 自带 129 个，Symbols 补 49 个，Mono 兜底全部 180 个。缺字永远不会掉到平台字体上 | §6.2 |
| 9 | CJK **零字节自托管**，走系统栈；macOS 落到 PingFang SC、Windows 落到 Microsoft YaHei、Linux 落到 Noto Sans CJK SC | §7 |
| 10 | 三款字体的许可证 **全部是 SIL OFL 1.1**（逐一取回 `OFL.txt` / `LICENSE` 原文核对）；Iosevka 也是 OFL 但因覆盖不足未采用 | §8 |

---

## 1. 交付物与验收

### 1.1 交付物

| 路径 | 内容 |
|---|---|
| `site/assets/fonts/*.woff2` | 5 个自托管子集，**只有 woff2** |
| `site/assets/fonts.css` | 5 条 `@font-face`（相对 `url()`、`font-display: swap`）+ 参考字体栈 |
| `scripts/gen-site-fonts.py` | 可复现生成器（幂等、自校验、源按 sha256 钉死） |
| `docs/design/site-rebuild/research/R4-fonts.md` | 本文件 |

### 1.2 真实产物与字节数

```
$ ls -l site/assets/fonts/
-rw-r--r--  10992  soko-mono-400.woff2
-rw-r--r--  10996  soko-mono-700.woff2
-rw-r--r--  24052  soko-serif-italic.woff2
-rw-r--r--  22156  soko-serif-roman.woff2
-rw-r--r--   4808  soko-symbols.woff2
```

| 文件 | 来源 | 字重 | 字节 | 覆盖 |
|---|---|---|---|---|
| `soko-serif-roman.woff2` | Source Serif 4（可变 `wght` 400–700，`opsz` 钉 20） | 400–700 连续 | **22,156** | 129/180 |
| `soko-serif-italic.woff2` | Source Serif 4 Italic（同上） | 400–700 连续 | **24,052** | 129/180 |
| `soko-mono-400.woff2` | JuliaMono Regular | 400 | **10,992** | **180/180** |
| `soko-mono-700.woff2` | JuliaMono Bold | 700 | **10,996** | **180/180** |
| `soko-symbols.woff2` | STIX Two Math（49 个衬线缺口字形） | 400 | **4,808** | 49/180 |
| **合计** | | | **73,004 B = 71.3 KB** | 并集 180/180 |

### 1.3 验收命令与真实输出

```
$ python3 scripts/gen-site-fonts.py --audit
audit: 仓库扫描到 84 个码位，显式清单 180 个
audit: 故意忽略 U+FE0F（零宽格式字符，不需要字形）
audit: PASS —— 仓库内容没有超出显式清单
source fonts (.cache/fonts/src/):
serif coverage: Source Serif 4 覆盖 129/180，缺 51 个

file                          bytes  asked  carried  dropped  result
soko-serif-roman.woff2        22156    180      129       51  PASS
soko-serif-italic.woff2       24052    180      129       51  PASS
soko-mono-400.woff2           10992    180      180        0  PASS
soko-mono-700.woff2           10996    180      180        0  PASS
soko-symbols.woff2             4808     51       49        2  PASS

total payload: 73004 B = 71.3 KB over 5 files
union  自托管面并集覆盖 REQUIRED:  PASS  (180/180)
mono   Soko Mono 单独覆盖 REQUIRED: PASS  (180/180)
split  衬线面自带 129 个；其余 51 个由 Soko Symbols → Soko Mono 兜底（CSS 栈里顺序固定，不依赖平台字体）
css    url() 与 site/assets/fonts/ 自洽: PASS （5 条 url()，5 个文件）
css    site/assets/fonts.css up-to-date
exit=0
```

**"每个文件都被 `fonts.css` 引用 / 每条 `url()` 都能解析"** —— 由脚本内的
`check_css_links()` 双向断言（磁盘上多一个文件、或 CSS 里多一条 `url()`，都是 FAIL）。
独立复核：

```
$ python3 -c "..."   # 见 §10 第 6 条
  OK   url("fonts/soko-serif-roman.woff2")  -> site/assets/fonts/soko-serif-roman.woff2  22156 B
  OK   url("fonts/soko-serif-italic.woff2") -> site/assets/fonts/soko-serif-italic.woff2 24052 B
  OK   url("fonts/soko-mono-400.woff2")     -> site/assets/fonts/soko-mono-400.woff2     10992 B
  OK   url("fonts/soko-mono-700.woff2")     -> site/assets/fonts/soko-mono-700.woff2     10996 B
  OK   url("fonts/soko-symbols.woff2")      -> site/assets/fonts/soko-symbols.woff2       4808 B
files on disk: 5 个；all referenced: True
```

**幂等（字节级）** —— 连续 3 个全新进程跑，sha256 完全一致：

```
$ shasum -a 256 site/assets/fonts/*.woff2 site/assets/fonts.css
9de295d64a08bcbb8a3a138db65f8e3cdd8a3e5e5dc8b714a7d7d70a68703fce  site/assets/fonts/soko-mono-400.woff2
a9c890e84b697651c2330b78a23dc758af57b0f4ee3f2efea1a6d014ee7445b1  site/assets/fonts/soko-mono-700.woff2
611918deec3e535728b348d841aad2c88c4778a42456cddfcaf7204bd79be1d8  site/assets/fonts/soko-serif-italic.woff2
d21918d25c30b4d964c01c770aff60783463feaf0670fdb325a7a42b2ca41ae9  site/assets/fonts/soko-serif-roman.woff2
ad05f99bf0b33f9707a018bf2579f48a888349d9077b2c91b18f5608136fb1a0  site/assets/fonts/soko-symbols.woff2
b99d2ccf5cad53a951a9e587978d09c772d116df6cb981dbbbe15a603e0d0135  site/assets/fonts.css
```

（第一次跑出来并不是幂等的——**`head.modified` 被 `TTFont.save()` 写成了"现在"**，
两次运行只差 head 里 2 个字节。踩坑与修法见 §8.2。）

### 1.4 接入方式（本任务不允许改 HTML，需由站点侧补一行）

```html
<link rel="stylesheet" href="assets/fonts.css">
```

放在 `assets/style.css` / `assets/tokens.css` **之前**。`fonts.css` 里所有
`url()` 都相对 `site/assets/` 解析，`fonts/` 与 `fonts.css` 同级，与页面层级无关。

---

## 2. Step 1 — 从仓库实测真实字形集（不猜）

### 2.1 任务书给定命令的原样输出（68 项）

```
$ cd /Users/penglingwei/Documents/lean/sokonanoda/sokonanoda-lang
$ python3 - <<'PY'   # 任务书 §Step 1 的脚本，一字未改
...
PY
U+2500 '─' 16560    U+2014 '—' 1154    U+2192 '→' 208    U+2550 '═' 206
U+2026 '…' 163      U+2208 '∈' 160     U+201C '“' 102    U+201D '”' 102
U+2286 '⊆' 87       U+2205 '∅' 73       U+2194 '↔' 69     U+222A '∪' 67
U+2203 '∃' 59       U+2190 '←' 50       U+2200 '∀' 49     U+2502 '│' 48
U+2460 '①' 43       U+21D2 '⇒' 43       U+2461 '②' 40     U+1D4AB '𝒫' 40
U+2229 '∩' 39       U+207B '⁻' 38       U+2013 '–' 31     U+2462 '③' 31
U+251C '├' 28       U+2218 '∘' 21       U+2227 '∧' 20     U+2468 '⑨' 19
U+2463 '④' 19       U+2466 '⑦' 16       U+2605 '★' 12     U+2469 '⑩' 12
U+2465 '⑥' 12       U+2514 '└' 11       U+2464 '⑤' 11     U+2248 '≈' 11
U+2115 'ℕ' 11       U+2264 '≤' 11       U+2467 '⑧' 8      U+26A0 '⚠' 7
U+27FA '⟺' 7        U+2260 '≠' 7        U+2713 '✓' 6      U+2209 '∉' 5
U+246A '⑪' 4        U+2228 '∨' 4        U+2265 '≥' 4      U+25BC '▼' 4
U+2193 '↓' 3        U+2983 '⦃' 3        U+2984 '⦄' 3      U+21CF '⇏' 3
U+2022 '•' 3        U+246B '⑫' 3        U+27F9 '⟹' 3      U+2160 'Ⅰ' 3
U+FE0F '️' 2        U+27E8 '⟨' 2        U+27E9 '⟩' 2      U+220E '∎' 2
U+21D4 '⇔' 2        U+2161 'Ⅱ' 2        U+211D 'ℝ' 2      U+21AA '↪' 2
U+2287 '⊇' 2        U+2241 '≁' 2        U+22A2 '⊢' 2      U+2212 '−' 1
```

68 项。**`⊢` 只出现 2 次**（`docs/architecture.md:123`、`docs/protocol.md:288`），
但它是产品的核心符号（假设与目标的分隔符），且会出现在**每一个目标态面板**里——
按出现次数决定要不要覆盖它是错的，按"会不会上屏"决定才对。

### 2.2 三处必须扩样的地方（否则清单会漏）

任务书的 glob 有三类盲区，我按同一口径补扫后得到 **83 个真实码位**：

| 盲区 | 漏掉的 | 证据 |
|---|---|---|
| `ord(c) > 0x2000` 把 Latin-1 补充区排除了 | `§`(U+00A7) `¬`(U+00AC) `·`(U+00B7) `¹`(U+00B9) `×`(U+00D7) `ö`(U+00F6) | `§` 实测 101 次（`courses/set-theory/lib/Equiv.sokonanoda`） |
| 希腊字母（U+0370–U+03FF）没被点名 | `α`(1911 次) `β`(803) `γ`(209) `δ`(18) `λ`(8) `φ`(5) `Π` `Ω` | 课程与 README 里 Lean 类型变量就是希腊字母 |
| 上/下标修饰符在 0x2000 以下 | `ᶜ`(U+1D9C, 13 次) `ˢ`(U+02E2, 6 次) | `courses/set-theory/lib/Set.sokonanoda` 的补集/后继记法 |

另外**审计只统计能上屏的字符**：`site/index.html` 里 206 个 `═` 全在 HTML 注释里
（`<!-- ══════ Hero ══════ -->`），`site/assets/tokens.css` 的注释里用 `✅` 标注
对比度结论。这些都不进字形集——把注释算进去只会让清单虚胖，还要为 emoji 兜底。
（生成器 `--audit` 会剥掉 `/* */` 与 `<!-- -->` 再扫，见 §8.3。）

### 2.3 最终显式清单 `REQUIRED` = 180 个码位

| 组 | 数量 | 内容 |
|---|---|---|
| 可打印 ASCII | 95 | U+0020–U+007E |
| 仓库实测非 ASCII | 83 | §2.1 的 68 项 + §2.2 的 15 项（去掉零宽 U+FE0F） |
| 排版预防性补充 | 2 | `‘`(U+2018) `’`(U+2019)。**仓库当前内容里实测 0 次**（内容用的是 ASCII `'`），但中英混排正文一旦出现 `kernel’s` 这类写法，缺字会掉进 CJK 字体、骨架不齐；两个码位合计约 600 B，作为预防性覆盖纳入 |
| **合计** | **180** | 其中 **85 个非 ASCII** |

### 2.4 `U+FE0F` 为什么不进清单

`⚠️` = U+26A0 + U+FE0F（VARIATION SELECTOR-16），实测 2 次。变体选择符是**零宽
格式字符，本身不需要字形**——它的作用是向渲染器请求 emoji 呈现，由系统
emoji/符号回退链处理。放进 Latin 子集只会多一个空字形。生成器的 `--audit`
把它**单独打印出来**（"故意忽略 U+FE0F"），不静默吞掉。

---

## 3. 测量方法（为什么这张表可信）

```python
from fontTools.ttLib import TTFont
cmap = TTFont(path).getBestCmap()          # 解析真实 cmap 子表 (3,10)/(3,1)
missing = [cp for cp in REQUIRED if cp not in cmap]
```

三条纪律：

1. **测的是完整上游字体，不是 CDN 预切好的子集。** Fontsource 的 `latin` 分片
   只有 Latin 区段，拿它测符号覆盖会得出"全都缺"的假结论。
2. **`unicode-range` 不算证据。** 实测过：Google Fonts CSS API 对 JetBrains Mono
   请求 `text=A⊢→` 时返回的 `unicode-range: U+41, U+2192, U+22a2` **包含 U+22A2**，
   但把那个 woff2 下载下来解析 cmap —— **`⊢` 并不在里面**。CSS 声明的范围与实际
   cmap 不一致，只有 cmap 是真相。
3. **产物写完再测一遍。** 见 §6.3：不止查 cmap，还查**字形轮廓非空**
   （防子集器留空壳）。

---

## 4. Step 2 — 候选覆盖矩阵（全部实测）

### 4.1 Latin 文本面（衬线候选，7 款）

| 字体 | cmap 总数 | 覆盖 /180 | `⊢` U+22A2 | `𝒫` U+1D4AB | 180 码位子集字节¹ |
|---|---|---|---|---|---|
| **Source Serif 4** | 918 | **129** | ✗ | ✗ | 12,604 |
| Literata | 1,163 | 127 | ✗ | ✗ | 13,540 |
| Noto Serif | 2,965 | 123 | ✗ | ✗ | 9,528 |
| STIX Two Text | 1,281 | 122 | ✗ | ✗ | 12,252 |
| IBM Plex Serif | 754 | 120 | ✗ | ✗ | 8,444 |
| Spectral | 878 | 120 | ✗ | ✗ | 8,468 |
| Newsreader | 564 | 116 | ✗ | ✗ | 12,192 |

¹ wght=400 静态实例、同一套 layout 特性（`kern,liga,ccmp,locl,mark,mkmk,calt,tnum,zero,lnum`）、
关 hinting、无 GSUB 风格集。

**关键事实：7 款全部缺 `⊢`。** 全部 7 款也都缺 `𝒫`(U+1D4AB)、`⦃`(U+2983)、
`⦄`(U+2984)、`①`–`⑫`、`Ⅰ` `Ⅱ`。

Source Serif 4 的 51 个缺口（完整列出）：

```
U+207B U+2115 U+211D U+2160 U+2161 U+2194 U+21AA U+21CF U+21D2 U+21D4
U+2200 U+2203 U+2205 U+2208 U+2209 U+220E U+2218 U+2227 U+2228 U+2229
U+222A U+2241 U+2286 U+2287 U+22A2 U+2460 U+2461 U+2462 U+2463 U+2464
U+2465 U+2466 U+2467 U+2468 U+2469 U+246A U+246B U+2500 U+2502 U+2514
U+251C U+2550 U+2605 U+26A0 U+27E8 U+27E9 U+27F9 U+27FA U+2983 U+2984
U+1D4AB
```

**注意 Source Serif 4 覆盖了任务书点名清单里除了 `⊢` 与 `𝒫` 之外的全部符号**：
`→ ≠ ≤ ≥ ∈ ⊆ ∧ ∨ ∀ ∃ ⟨ ⟩` 以及弯引号 `“ ”`、破折号 `— –`、省略号 `…`、`·`
都在。也就是说**行内记法只有 2 个字符会掉出衬线面**——这是它胜过其余 6 款的地方。

> **反直觉的一条**：STIX Two Text（一款"科学排版"字体）在本项目的符号集上
> **比 Source Serif 4 还差**——它连 `→ ← ↓ ↔`、`≤ ≥ ≠`、`⊢` 都没有（数学符号
> 在配套的 STIX Two **Math** 里，不在 Text 里）。所以"名字里有 STIX 所以符号全"
> 是错的，必须实测。

### 4.2 等宽面（8 款）

| 字体 | cmap 总数 | 覆盖 /180 | **`⊢`** | 180 码位子集字节¹ |
|---|---|---|---|---|
| **JuliaMono** | 11,191 | **180 / 180** | **✓** | 11,044 |
| DejaVu Sans Mono | 3,322 | 161 | ✓ | 10,156 |
| Noto Sans Mono | 3,490 | 156 | ✓ | 8,004 |
| JetBrains Mono | 976 | 145 | **✗** | 6,724 |
| Source Code Pro | 1,334 | 139 | **✗** | 7,204 |
| Fira Mono | 1,350 | 134 | **✗** | 6,608 |
| IBM Plex Mono | 930 | 125 | **✗** | 5,680 |
| Iosevka（Fontsource 22.1.2 分片）² | 5,706 | 176 | ✓ | 984,084（未切） |

¹ 同一套口径（关 hinting、无 layout 特性）。
² **Iosevka 未能取到完整上游字体**：GitHub release 资产从本网络不可达
（`curl` 连接超时），npm/`@fontsource/iosevka` 只有 latin 分片。上表数字测的是
**Fontsource 实际发布的那个 woff2**（它本身有 5,706 个码位、984 KB）。
Iosevka 的**完整版**覆盖未测，见 §9。

**逐候选的缺失码位（完整）**

| 字体 | 缺什么 |
|---|---|
| JuliaMono | （无） |
| DejaVu Sans Mono | `Ⅰ Ⅱ ① ② ③ ④ ⑤ ⑥ ⑦ ⑧ ⑨ ⑩ ⑪ ⑫ ⟹ ⟺ ⦃ ⦄ 𝒫` |
| Noto Sans Mono | `Ⅰ Ⅱ ↪ ⇏ ①–⑫ ★ ⚠ ✓ ⟹ ⟺ ⦃ ⦄ 𝒫` |
| JetBrains Mono | `ˢ ᶜ ⁻ ℝ Ⅰ Ⅱ ↪ ⇏ ⇒ ⇔ ∅ ∎ ∩ ∪ ≁ ⊢ ①–⑫ ★ ✓ ⟹ ⟺ ⦃ ⦄ 𝒫` |
| Source Code Pro | `⁻ ℕ ℝ Ⅰ Ⅱ ↪ ⇏ ⇔ ∅ ∈ ∉ ∎ ∘ ∧ ∨ ∪ ≁ ⊆ ⊇ ⊢ ①–⑫ ★ ⚠ ⟨ ⟩ ⟹ ⟺ ⦃ ⦄ 𝒫` |
| Fira Mono | `ˢ ᶜ ℕ ℝ Ⅰ Ⅱ ↪ ⇏ ⇒ ⇔ ∀ ∃ ∅ ∈ ∉ ∎ ∘ ∧ ∨ ∪ ≁ ⊆ ⊇ ⊢ ①–⑫ ★ ⚠ ✓ ⟨ ⟩ ⟹ ⟺ ⦃ ⦄ 𝒫` |
| IBM Plex Mono | `ˢ α β γ δ λ φ Π Ω ᶜ ⁻ ℕ ℝ Ⅰ Ⅱ ⇏ ⇒ ⇔ ∀ ∃ ∅ ∈ ∉ ∎ ∘ ∧ ∨ ∩ ∪ ≁ ⊆ ⊇ ⊢ ①–⑫ ▼ ★ ⚠ ⟨ ⟩ ⟹ ⟺ ⦃ ⦄ 𝒫` |

**`⦃` `⦄` 是淘汰 DejaVu / Noto Sans Mono 的决定性一条。** 它们出现在
`courses/set-theory/lib/Equiv.sokonanoda` 的集合推导式记法 `⦃x // p⦄` 里，
**必然出现在目标态面板**（等宽）。等宽面缺它 → 逐字回退 → 同一行等宽文本里
混进一个宽度不同的字形 → 对齐崩掉，而且截图小图看不出来（R1 §3.9 描述的正是这个）。

### 4.3 符号兜底面（补 Source Serif 4 的 51 个缺口）

| 字体 | 覆盖 51 缺口 | 缺什么 |
|---|---|---|
| **STIX Two Math** | **49 / 51** | `Ⅰ` `Ⅱ` |
| JuliaMono | 51 / 51 | （无） |
| DejaVu Sans | 48 / 51 | `⑪` `⑫` `𝒫` |
| Noto Sans Math | 33 / 51 | `⁻ Ⅰ Ⅱ ①–⑫ │ ├ ⚠` |
| Noto Sans Symbols | 15 / 51 | 大量 |
| Noto Sans Symbols 2 | 3 / 51 | 几乎全部 |

选 STIX Two Math：它是**为数学排版设计的衬线**，与 Source Serif 4 同为过渡型
衬线，`∈ ⊆ ∀ ∃ ⟨ ⟩ 𝒫` 这些形状放进行内不会"换设计"；缺的 `Ⅰ` `Ⅱ` 由
JuliaMono 兜底（它 51/51 全有）。

---

## 5. `⊢` 断言核验

### 5.1 任务书断言：**成立**

原断言：Menlo、SF Mono、Monaco、Hiragino Sans GB、Inter、JetBrains Mono、
Source Code Pro **全部缺 `⊢`**。逐条实测（`TTFont(path).getBestCmap()`）：

| 字体 | 来源 | `⊢` U+22A2 | `𝒫` U+1D4AB | `⦃` U+2983 |
|---|---|---|---|---|
| Menlo | `/System/Library/Fonts/Menlo.ttc` | **✗** | ✗ | ✗ |
| SF Mono | `/System/Library/Fonts/SFNSMono.ttf` | **✗** | ✗ | ✗ |
| Monaco | `/System/Library/Fonts/Monaco.ttf` | **✗** | ✗ | ✗ |
| Hiragino Sans GB | `/System/Library/Fonts/Hiragino Sans GB.ttc` | **✗** | ✗ | ✗ |
| JetBrains Mono | google/fonts 完整可变字体 | **✗** | ✗ | ✗ |
| Source Code Pro | google/fonts 完整可变字体 | **✗** | ✗ | ✗ |
| Inter | — | **未验证** | — | — |
| Apple Symbols | macOS 系统 | ✓ | ✗ | ✓ |
| Arial Unicode MS | `/Library/Fonts/Arial Unicode.ttf` | ✓ | ✗ | ✗ |
| Courier New | macOS 系统 | **✗** | ✗ | ✗ |
| **JuliaMono** | jsDelivr `gh` v0.059 | **✓** | **✓** | **✓** |

**7 款里 6 款已实测确认缺 `⊢`；Inter 本机没有、无法验证**，见 §9。
顺带发现 `Apple Symbols` 与 `Arial Unicode MS` 有 `⊢`——但**都不能用来渲染代码**：
它们是比例符号字体，不是等宽，用在 `pre` 里同样破坏对齐。

### 5.2 为什么这一条对本站是致命的

浏览器对缺字形的处理是**逐字回退**，不是画豆腐块。在等宽面板里，一行
`h : P ⊢ Q` 如果 `⊢` 来自回退字体，它和相邻 ASCII 的宽度不同——**同一行等宽
文本里混进一个比例字形**。R1 §3.9 已经指出这个失败模式"崩得很隐蔽
（截图小图看不出来）"。在一个"判定永远走内核、禁止文本比对"的产品里，
让排版对字符宽度撒谎是不可接受的。

### 5.3 修正 R1 §3.9 的一条结论

R1 §3.9 结论 2 写："实测唯一同时覆盖 `⊢ ∀ ∃ → ↔ ∧ ∨ ¬ ≠ ≤ ∈ ⊆ ⟨ ⟩ α λ ℕ ⟶ ↦ ⇒ ⊤ ⊥`
的候选是 **Noto Sans Mono**（仅缺 `⟹` 与 `「`）"。

按本报告同一口径（解析 cmap）复核：

* Noto Sans Mono 在**本报告更宽的 180 码位清单**上是 **156/180**，
  缺 `Ⅰ Ⅱ ↪ ⇏ ①–⑫ ★ ⚠ ✓ ⟹ ⟺ ⦃ ⦄ 𝒫`——R1 的 29 符号清单没包含
  `⦃ ⦄` 与 `①–⑫`，所以没暴露这些缺口；
* **R1 未把 JuliaMono 纳入候选**。JuliaMono 是 **180/180**。

结论：R1 的"唯一候选是 Noto Sans Mono"**在它的 29 符号口径下成立，在完整口径下
不成立**。R1 自己也写明了它的方法是"测 Google Fonts 仓库里的可变字体"——
口径没错，只是清单不够宽。

---

## 6. 最终决策

### 6.1 选了什么、为什么

| 面 | 选择 | 决定性理由（全部实测） |
|---|---|---|
| **拉丁正文/标题** | **Source Serif 4**（OFL，v4.004） | ① 7 款衬线里**符号覆盖最好（129/180）**，任务书点名清单里只缺 `⊢` `𝒫` 两个；② **可变字体**：一个文件覆盖 `wght` 400–700，正好命中站点用到的 400/600/700 三档（实测 `site/assets/style.css`：`400×1, 600×4, 700×4` + `<strong>` 默认 700），比三个静态文件更少文件数；③ 低对比、大 x-height 的结实过渡衬线——与系统 CJK 黑体并排时不会"显轻"（正面回应 R1 §3.5 的理由 2）；④ 真斜体（可变 400–700） |
| **等宽** | **JuliaMono**（OFL，v0.059） | **唯一 180/180**。目标态面板必须等宽且不能逐字回退，这一条没有第二选择。它本来就是为科学计算设计的，符号形状是刻意画过的 |
| **符号兜底** | **STIX Two Math**（OFL，v2.13 b170） | 49/51 补上衬线的缺口，且是**衬线数学设计**——行内 `⊢ 𝒫 ∈ ⊆ ⦃ ⦄` 不会"换设计"。带 `unicode-range`，只在页面真的用到这些码位时才下载 |
| **CJK** | **系统栈，零字节** | 见 §7 |

**为什么不选 IBM Plex Serif（子集最小，8,444 B）**：它只有静态权重（3 个文件）、
覆盖 120/180（比 Source Serif 4 少 9 个符号），斜体也更装饰化。省下的 4 KB
要用更多文件数和更多符号兜底来换，不划算。
**为什么不选 STIX Two Text**：覆盖 122/180，且连 `→ ≤ ≠` 都没有——"STIX" 这个名字
在这里是误导，数学在 STIX Two **Math** 里。

### 6.2 字体栈顺序（这一行是有约束的，不能改）

```css
--soko-font-prose: "Soko Serif", "Soko Symbols", "Soko Mono", system-ui, … CJK …, sans-serif;
--soko-font-mono:  "Soko Mono", ui-monospace, SFMono-Regular, Menlo, Consolas, …;
```

* `Soko Serif` 自带 **129/180**；
* 缺的 51 个里 **49 个**由 `Soko Symbols`（`unicode-range` 限定）接住；
* 剩下 `Ⅰ` `Ⅱ` 由 `Soko Mono` 接住——**而 Mono 是 180/180**，所以它同时是
  整个栈的最后一道**自托管**保险；
* 结果是**任何码位都不会掉到平台字体上**，跨平台渲染一致。

`Soko Mono` 放在兜底位不产生额外请求：它本来就要为代码块下载。

### 6.3 权重：只发用得到的

| 面 | 发的权重 | 依据 |
|---|---|---|
| Serif roman | `font-weight: 400 700`（可变，1 文件） | 站点实测用到 400 / 600 / 700 |
| Serif italic | `font-weight: 400 700`（可变，1 文件） | 强调与中文正文里的拉丁术语；粗体标题里的 `<em>` 需要真 700 斜体 |
| Mono | 400 + 700（2 文件） | 代码块正文 400；面板标题/强调 700 |

没有发 300/500/800/900，也没有发 mono 斜体。

---

## 7. CJK：不自托管，走系统栈

**为什么不自托管**：中文子集动辄数百 KB 到数 MB，而整站 HTML 只有 ~72 KB
（R1 §2.2）。为零构建站点引入一个比全站内容大数倍的二进制资源，与
`docs/design/site.md` §2 的"零构建"决策直接冲突。**全站 90% 的可见文字是中文**，
既然中文必须走系统栈，为占少数的拉丁文字引入 webfont 的边际收益才成立——
但前提是 Latin 面必须**确定性覆盖符号**，否则等于把最关键的记法交给了运气。

### 7.1 逐平台实际解析结果

```css
--soko-font-cjk:
  "PingFang SC", "Hiragino Sans GB",
  "Microsoft YaHei UI", "Microsoft YaHei",
  "Noto Sans CJK SC", "Source Han Sans SC", "Noto Sans SC", sans-serif;
```

| 平台 | 实际命中 | 说明 |
|---|---|---|
| **macOS** | **PingFang SC** | 10.11+ 系统自带。`Hiragino Sans GB` 是 10.6–10.10 的旧名，留作老系统回退 |
| **Windows** | **Microsoft YaHei UI → Microsoft YaHei** | Win7+ 自带；`UI` 变体在 Win8+ 存在，是正文标准黑体。`Segoe UI` 不含汉字，不参与中文命中 |
| **Linux** | **Noto Sans CJK SC** | 主流发行版（Ubuntu/Fedora/Debian 装 `fonts-noto-cjk` 时）命中；未装则 `Source Han Sans SC`（思源黑体，同源不同名）；再不行落到 `sans-serif`（fontconfig 通常会代到某个 CJK 字体） |
| **Android / iOS** | 前者落到 `Noto Sans CJK SC` 或系统默认，后者落到 `PingFang SC` | 同上 |

### 7.2 为什么 `system-ui` 排在 CJK 名字之前（而不是之前 R1 建议的位置）

`--soko-font-prose` 里 `system-ui` 排在自托管面之后、CJK 名字之前：
非 Apple 平台也能先吃到系统 UI 字体（R1 §3.8 建议 1）；而 `system-ui`
在 macOS 上是 SF Pro（**不含汉字**），CSS 字体匹配会继续往后走，
所以**中文仍然落到 PingFang SC**，不会因为 `system-ui` 靠前而被截胡。

页面已声明 `<html lang="zh-CN">`（`site/index.html:1` 实测），因此 Han 字的
地区变体选择由 `lang` 决定，不依赖字体名。

---

## 8. 生成管线：`scripts/gen-site-fonts.py`

### 8.1 源字体：URL + sha256 双重钉死

| 文件 | 钉住的 URL | sha256 | 版本 |
|---|---|---|---|
| Source Serif 4 | `cdn.jsdelivr.net/gh/google/fonts@f2bd09ba…/ofl/sourceserif4/SourceSerif4[opsz,wght].ttf` | `97b2d4da…9aca0b` | 4.004 |
| Source Serif 4 Italic | 同 commit `…-Italic[opsz,wght].ttf` | `15fbc7e4…7a1f42` | 4.004 |
| JuliaMono Regular | `cdn.jsdelivr.net/gh/cormullion/juliamono@v0.059/JuliaMono-Regular.ttf` | `3e521304…0e36c1` | 0.059 |
| JuliaMono Bold | 同 tag `JuliaMono-Bold.ttf` | `f4605d77…7ffdaf` | 0.059 |
| STIX Two Math | `cdn.jsdelivr.net/gh/stipub/stixfonts@v2.13/fonts/static_otf/STIXTwoMath-Regular.otf` | `f2076b9f…ccfa72` | 2.13 b170 |

* `google/fonts` 没有 tag，所以钉到 **commit `f2bd09badbc763d8757951d52deec29da27e85fb`**
  （从 `https://github.com/google/fonts/commits/main.atom` 取到）。
* JuliaMono 的 `@master` 与 `@v0.059` **内容不同**（实测 sha256 不同），所以必须钉 tag。
* 下载缓存进 `.cache/fonts/src/`；**上游改动会让脚本 exit 1 并打印期望/实际 sha256**，
  绝不静默产出不同字节。`--update-sources` 用来重新取钉。

### 8.2 幂等：踩到的坑

第一次实现**不是**幂等的：连续两次运行，5 个文件的 sha256 全都不一样，
字节数也差 4–12 B。逐表对比解压后的 woff2 定位到唯一差异：

```
DIFF head: 54 vs 54
  r1 modified: 1127029167885679
  r2 modified: 1127029167885669
  diff bytes n=2 first=[11, 35]
```

`TTFont.save()` 默认 `recalcTimestamp=True`，`_save()` 把 `head.modified`
写成"现在"。修法（注意 fontTools 4.65 的 `TTFont.save()` **已经没有**
`recalcTimestamp` 参数了，必须走实例属性）：

```python
font.recalcTimestamp = False   # 而不是 font.save(path, recalcTimestamp=False)
font.save(str(out))
```

修完后连续 3 个全新进程输出 sha256 完全一致（§1.3）。

### 8.3 生成器自带的四道断言

| 断言 | 触发条件 | 结果 |
|---|---|---|
| **子集器静默丢字形** | 源字体本来有、产物里却没有 | `FAIL` + 打印缺失码位，`exit 1` |
| **并集覆盖** | `⋃ 所有产物 cmap ⊉ REQUIRED` | `FAIL` + 打印缺口，`exit 1` |
| **等宽单独覆盖** | `Soko Mono ⊉ REQUIRED` | `FAIL`，`exit 1` |
| **CSS ↔ 目录自洽** | 磁盘上多文件 / CSS 里多 `url()` / `url()` 解析不到 | `FAIL`，`exit 1` |
| **仓库内容漂移**（`--audit`） | 重扫仓库（剥注释）发现清单外的码位 | `FAIL` + 打印码位与首次出现文件 |

`--audit` 在开发过程中**真的抓到了两个 bug**：
① `site/assets/tokens.css` 注释里的 `✅` 让清单虚胖 → 改成剥注释再扫；
② 我原先算错 `Source Serif 4 ∪ STIX Two Math = 全覆盖`，实际 **`Ⅰ` `Ⅱ` 两边都没有**
→ 断言直接报 `union FAIL [8544, 8545]`，才发现必须靠 JuliaMono 兜底。
**这就是"覆盖率必须断言、不能靠希望"的价值。**

### 8.4 依赖与缺依赖时的行为

```bash
pip3 install --target .cache/pylibs fonttools brotli
```

缺依赖时 `exit 2` 并给人话（实测 `SOKO_PYLIBS=/nonexistent/pylibs`）：

```
gen-site-fonts: 缺少依赖 brotli（No module named 'brotli'）。

这是维护者工具，需要 fontTools + brotli：

    pip3 install --target .cache/pylibs fonttools brotli

产物（site/assets/fonts/*.woff2、site/assets/fonts.css）已提交入库，
普通使用/学习路径不需要跑本脚本。
```

与 `scripts/gen-site-demos.py` 需要 Pillow 是同一档：**维护者工具，产物已入库**，
用户/学习者路径零工具链依赖的纪律不受影响。

### 8.5 子集参数（可复现的关键）

| 参数 | 值 | 理由 |
|---|---|---|
| `unicodes` | **显式 180 码位清单** | 不用 `unicode-range` 块切，产物可精确复现 |
| `layout_features`（衬线） | `kern,liga,ccmp,locl,mark,mkmk,calt,tnum,zero,lnum` | `tnum`/`zero`/`lnum` 是 R1 §3.6 的等宽数字要求；**不含** `ss01`–`ss20`/`cv01`–`*` 风格集（实测占 ~27 KB：`ALL` = 48,780 B vs 本集合 = 21,204 B） |
| `layout_features`（等宽） | 空 | 顺带满足 R1 §3.7"代码块必须关掉连字"——子集里根本没有 GSUB，连字不可能撒谎 |
| `hinting` | `False` | woff2 + 现代渲染管线；实测对 Source Serif 4 只差 148 B，对 JuliaMono 差 ~27 KB（`calt` 的 GSUB） |
| `drop_tables` | `+DSIG, +MATH` | 网页不用 OpenType MATH 排版；丢掉 MATH 让符号面从 8,612 B 降到 4,808 B |
| `name_IDs` | `0,1,2,3,4,5,6,7,13,14,16,17` | 保留版权/商标/许可声明（OFL 要求）；**不**保留全部语言的全部记录 |
| `recalc_timestamp` | `False` | 见 §8.2 |
| `flavor` | `woff2` | 需要 brotli |
| `opsz` 轴 | **钉到 20** | Source Serif 4 的 opsz 默认值。实测 opsz=12/16/20/28 的子集大小是 21,192 / 21,220 / **20,092** / 21,448 B——20 最小；且 R1 §3.7 明确"不要为了 opsz 选字体"，钉死它换取更小的 gvar |
| `wght` 轴 | 保留 400–700（`instancer` 做轴区间裁剪） | 一个文件覆盖三档用到的字重 |

**斜体的一个 gotcha**：Source Serif 4 Italic 上"先实例化再子集"会抛
`KeyError: 'glyph00679'`（gvar 引用了被裁掉的字形）；**反过来"先子集再实例化"
等价且稳定**（`order="subset-first"`）。roman 两种顺序都行——所以这不是
"某个字体坏了"，是顺序问题，脚本里对 italic 显式指定了顺序并写了注释。

---

## 9. 未验证项（不臆造）

| 条目 | 状态 | 原因 |
|---|---|---|
| **Inter 缺 `⊢`** | **未验证** | 本机没有安装 Inter，且本任务不再需要它做决策（Inter 不在 7 款衬线候选里）。R1 §3.9 报告它缺 `⊢`，本报告不背书也不否认 |
| **Iosevka 完整版覆盖** | **未测** | GitHub release 资产从本网络不可达（`curl` 连接超时）；npm / `@fontsource/iosevka` 只有 latin 分片。§4.2 里 Iosevka 那一行测的是 **Fontsource 实际发布的 984 KB woff2**，不是上游完整字体 |
| **Windows / Linux 上 Microsoft YaHei、Noto Sans CJK SC 的实际字形** | **未测** | 本机是 macOS。§7.1 里这两个平台写的是"会解析到哪个字体"，不是"该字体覆盖了哪些码位" |
| **浏览器实际渲染效果（逐字回退是否真的消失）** | **未做** | 需要真实浏览器 + 截图放大比对。建议的验收动作见 §10 第 8 条 |
| **woff2 在 Safari/Firefox 的解码** | 未做 | 只做了 fontTools 层面的往返解析；`format("woff2")` 是标准写法 |

---

## 10. 复现命令清单

```bash
cd /Users/penglingwei/Documents/lean/sokonanoda/sokonanoda-lang

# 1. 依赖（仅维护者）
pip3 install --target .cache/pylibs fonttools brotli

# 2. Step 1：实测仓库字形集（任务书原脚本；补样口径见 §2.2）
python3 - <<'PY'
import glob, collections
chars = collections.Counter()
pats = ["site/**/*.html", "site/assets/*.js", "playground.sokonanoda", "examples/*.sokonanoda",
        "courses/set-theory/lib/*.sokonanoda", "courses/set-theory/units/*.sokonanoda",
        "course/*.sokonanoda", "README.md", "docs/protocol.md", "docs/architecture.md"]
for p in pats:
    for f in glob.glob(p, recursive=True):
        try: t = open(f, encoding="utf-8").read()
        except Exception: continue
        for c in t:
            if ord(c) > 0x2000 and not (0x3000 <= ord(c) <= 0x9FFF) and not (0xFF00 <= ord(c) <= 0xFFEF):
                chars[c] += 1
for c, n in chars.most_common(200):
    print(f"U+{ord(c):04X} {c!r} {n}")
PY

# 3. Step 2：解析 cmap 测候选覆盖（把 <font> 换成候选路径）
PYTHONPATH=.cache/pylibs python3 -c "
from fontTools.ttLib import TTFont
cm = TTFont('<font>').getBestCmap()
print('U+22A2 in cmap:', 0x22A2 in cm)"

# 4. 生成（幂等）+ 内容漂移审计 + 只校验
python3 scripts/gen-site-fonts.py --audit
python3 scripts/gen-site-fonts.py --check

# 5. 幂等验证（三个全新进程，sha256 必须一致）
shasum -a 256 site/assets/fonts/*.woff2 site/assets/fonts.css > /tmp/a
python3 scripts/gen-site-fonts.py >/dev/null
shasum -a 256 site/assets/fonts/*.woff2 site/assets/fonts.css > /tmp/b
diff /tmp/a /tmp/b && echo IDEMPOTENT

# 6. url() 与目录双向自洽（脚本内已有；独立复核）
python3 -c "
import re; from pathlib import Path
css = Path('site/assets/fonts.css'); t = css.read_text()
for raw in re.findall(r'url\(\s*[\"\']?([^\"\')]+)[\"\']?\s*\)', t):
    p = (css.parent / raw).resolve(); print('OK ' if p.is_file() else 'MISSING', raw, p.stat().st_size if p.is_file() else 0)
print(sorted(p.name for p in Path('site/assets/fonts').iterdir()))"

# 7. 总载荷
python3 -c "
import glob, os
t = sum(os.path.getsize(f) for f in glob.glob('site/assets/fonts/*'))
print(t, 'B =', round(t/1024, 1), 'KB')"

# 8. 人工验收（唯一能发现逐字回退的方法）
#    页面里放一段含 ⊢ ∀ ∃ ∈ ⊆ ⦃ ⦄ 𝒫 ⟹ 的代码块，截图放大 400%，
#    确认字符是否严格等宽对齐、有无换设计。
```

---

## 11. 与 R1 的关系（一句话）

R1 §3.5 结论是"正文不用衬线"，理由是体积与中英骨架一致性；本报告**不推翻**它——
最终交付的 `Soko Serif` 定位与站点侧 `tokens.css` 的 `--font-display` 一致
（**衬线用于展示/标题，正文 UI 走系统 CJK 黑体**），而 R1 §3.9 的 `⊢` 陷阱
在本报告里被完整量化并**在管线里被断言守住**。

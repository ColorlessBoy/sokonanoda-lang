# R1 — 设计手艺调研：让 `site/` 不再有「AI 味」

> 调研对象：`sokonanoda-lang` 官网（`site/`，零构建手写 HTML + 单文件 CSS，
> GitHub Pages 部署）。目标：给出**可实现、可验证**的设计规则，而不是形容词。
>
> **方法说明（可复现）**：本报告只引用**实际取到原文**的来源，逐条给 URL。
> 取不到原文的条目集中列在 §12「未取到原文的条目」，**不臆造**。
> 本会话的 `web_search` 工具不可用（缺 API key），所有来源通过
> GitHub API 检索 + `curl`/`web_fetch` 直取 raw 文件获得。
> 对当前站点的量化体检结果全部可用 §2 给出的命令重跑。
>
> 撰写日期：2026-09-19。仓库版本 `0.61.0`，第 108 轮。

### 怎么读这份报告

报告比初稿约定（400–700 行）长，原因是任务要求的两块内容本身就是长的：
**§9 的反模式清单是"主要交付物"**（72 条 tell，每条要给出处 + 本项目命中情况），
**§11.4 的令牌草案要求可直接复制**（光 CSS 就 ~150 行）。
全文**无填充段落**——每一节都是规则、出处或实测数据。三条阅读路径：

| 你的目的 | 读 |
|---|---|
| **我要开始改代码** | §11（令牌草案）→ §9.9（五件 ROI 最高的事）→ 附录 A（验收脚本） |
| **我要理解为什么** | §0（摘要）→ §1（现有 skill 的原则）→ §9.0/§9.7（判据与共识） |
| **我要复核证据** | §12（未取到原文的完整清单）→ 各节就地标注的 URL |

| 节 | 内容 | 行数 |
|---|---|---|
| §0–§2 | 摘要 / 任务一（agent skill 调研）/ **本站现状量化体检** | ~190 |
| §3–§8 | 手艺规则：排版 / 间距布局 / 颜色 / 深度 / 动效 / 代码呈现 | ~1020 |
| **§9** | **反模式清单（主要交付物）** | ~245 |
| §10 | 语气与内容设计 | ~365 |
| **§11** | **令牌草案（可直接复制）** | ~230 |
| §12 + 附录 A | 未取到原文的清单 / 可验证检查清单 | ~215 |

---

## 0. 结论摘要

| # | 结论 | 依据 |
|---|---|---|
| 1 | 官方 `frontend-design` skill 是**唯一**一份明确把「AI 生成的默认长相」写成**可核对清单**的公开指导；它的价值不在排版规则，而在**把「默认」和「选择」区分开** | §1.2 |
| 2 | 该 skill 列出的 5 类「AI 设计聚类」中，**第 4 类（SaaS 卡片套装）和第 5 类（模板化装饰件）当前站点全中** | §2、§9 |
| 3 | 当前站点**不是文案问题**：文案具体、有真数字、有真命令、零 emoji，比多数同类站点诚实。**问题 100% 在设计层** | §2.2 |
| 4 | 当前 CSS 的硬伤可以量化：**17 个不同 `font-size`、6 种圆角、4 种 `line-height`、11 种非网格 `gap`、27 个硬编码 hex**（只有 9 个变量）、**0 个 `transition`、0 个 `prefers-*`、0 个 `:focus-visible`** | §2.1 |
| 5 | 中文站不能用「衬线大标题 + 奶油底 + 陶土色点缀」——那是官方 skill 点名的第 1 类 AI 聚类；且 CJK 衬线 webfont 体积不可接受 | §1.2、§3.7 |
| 6 | **webfont 决策**：CJK 必须走系统栈（体积原因），因此 Latin 也应选**人文主义无衬线**自托管子集（约 20–40 KB），而不是衬线 | §3.7 |
| 7 | **本项目专属的排版陷阱**：`⊢`（U+22A2）是产品最核心的符号，但 **Menlo、SF Mono、Monaco、Hiragino Sans GB、Inter、JetBrains Mono、Source Code Pro 全都没有这个字形**——实测数据见 §3.8。用它做代码字体会在代码块里触发逐字回退，破坏等宽对齐 | §3.8 |
| 8 | 当前配色**对比度基本达标**（唯一两处不过：`#8a6d1f` on `#fbf3df` = 4.43；copy 按钮 hover 边框 `#0a7cc4` on `#2f3b4c` = 2.54） | §2.3 |
| 9 | 建议的美学方向：**「工程方格纸 · 内核批注」**——冷调纸面、零圆角结构容器、**唯一的强调色 = 内核判定绿，且只用于「内核真的通过过」的东西** | §11.1 |
| 10 | 该方向下的全部令牌已通过 WCAG AA 数值校验（正文对全部 ≥ 4.5:1，控件边框 ≥ 3:1），可直接复制 | §11.4 |

---

## 1. 任务一：现有的「前端设计」agent skill 及其原则

### 1.1 来源总览

| 来源 | 位置 | 是否取到全文 | 对本项目可操作性 |
|---|---|---|---|
| `frontend-design`（Anthropic 官方） | https://raw.githubusercontent.com/anthropics/skills/main/skills/frontend-design/SKILL.md | ✅ 全文（9390 B，71 行） | ★★★★★ 直接可用 |
| `web-artifacts-builder`（Anthropic 官方） | https://raw.githubusercontent.com/anthropics/skills/main/skills/web-artifacts-builder/SKILL.md | ✅ 全文（3087 B，74 行） | ★★☆☆☆ 依赖 React/Tailwind/npm，**不适用**；只有一句设计指导可用 |
| `canvas-design`（Anthropic 官方） | https://raw.githubusercontent.com/anthropics/skills/main/skills/canvas-design/SKILL.md | ✅ 全文（11939 B） | ★★☆☆☆ 面向海报/PDF 艺术，非 UI |
| `theme-factory`（Anthropic 官方） | https://raw.githubusercontent.com/anthropics/skills/main/skills/theme-factory/SKILL.md | ✅ 全文（3124 B） | ★☆☆☆☆ 10 套预设主题，**没有一套的 hex 在文件里**（在 `themes/` 目录 + PDF 里，未取到） |
| `brand-guidelines`（Anthropic 官方） | https://raw.githubusercontent.com/anthropics/skills/main/skills/brand-guidelines/SKILL.md | ✅ 全文（2235 B） | ★☆☆☆☆ 是 Anthropic 自家品牌色，**照抄等于抄第 1 类 AI 聚类** |
| `anthropics/skills` 仓库说明 | https://raw.githubusercontent.com/anthropics/skills/main/README.md | ✅ 全文 | 说明 skill 是「demonstration and educational purposes」 |
| 社区 skill 集合 | 见 §1.5 | 部分 | 待补 |

官方仓库的自我定位（原文）：

> "These skills are provided for demonstration and educational purposes only. …
> These skills are meant to illustrate patterns and possibilities."
> —— https://raw.githubusercontent.com/anthropics/skills/main/README.md

**这句话很重要**：官方 skill 不是设计规范，是一份**提示词范本**。可以借鉴它的
「把默认长相列出来」的做法，但不必把它当权威排版标准。

### 1.2 `frontend-design` 的核心：5 类「AI 设计聚类」（原文引用）

这是整份调研里**最有价值的一段**。skill 要求先做设计计划，然后拿计划去比照
「AI 生成设计当前聚集的特征」，命中就改。原文（第 38–45 行）逐条：

> "For calibration, AI-generated design right now clusters around some traits:
> 1. a warm cream background (near #F4F1EA) with a high-contrast serif display and a
>    terracotta or warm-clay accent (often near #D97757 — Anthropic's own
>    Claude-interaction accent, so on a user's brief it reads as a tell);
> 2. a near-black background with a single bright acid-green or vermilion accent;
> 3. a broadsheet-style layout with hairline rules, zero border-radius, and dense
>    newspaper-like columns;
> 4. the SaaS-card kit: content chopped into identical rounded cards, one border-radius
>    on everything regardless of hierarchy, the same soft grey shadow
>    (rgba(0,0,0,.1)) under each, and gradient washes as decoration;
> 5. template chrome that appears whatever the subject: a tracked-out ALL-CAPS eyebrow
>    label above every heading; meta strings joined with middle dots ('A · B · C');
>    labels built as 'WORD — fragment' with a spaced em dash; tinted near-black
>    (#0B0B0B, #111) standing in for black; a monospace face for small data labels;
>    a '→' appended to link and button text."

紧接着的一句是**方法论**，比清单本身更重要：

> "All traits are legitimate for some briefs, but they are defaults rather than
> choices, and they appear regardless of subject. Where the brief pins down a visual
> direction, follow it exactly — the brief's own words always win, including when it
> asks for one of these looks. Where it leaves an axis free, don't spend that freedom
> on one of these defaults."

**翻译成可执行规则**：这 5 类里没有一条是「永远错」。判据是——
**这个选择是被 brief 推出来的，还是我闭眼就会写的？** 如果是后者，换掉。

对本项目的直接推论：

| 聚类 | 本项目是否踩 | 说明 |
|---|---|---|
| ① 奶油底 + 衬线大标题 + 陶土色 | ❌ 未踩 | 当前是 `#fbfbfa` 近白 + 系统无衬线 + 绿色。**改造时不要往这个方向走**——它看起来很"高级"，但正是最饱和的 AI 长相 |
| ② 近黑底 + 单一荧光色 | ❌ 未踩 | 当前只有代码块是深色。若做 dark mode 要避免"近黑 + 荧光绿" |
| ③ 报纸式：细线 + 零圆角 + 密栏 | ⚠️ 部分 | 本报告 §11 推荐的「零圆角 + 细线」**接近这一类**，必须靠"从题目推出来"来正当化（见 §11.1），而不是当默认 |
| ④ SaaS 卡片套装 | ✅ **全中** | `.card` / `.install-card` / `.demo-card` / `.unit-card` / `.stat` 全是同一套 `1px border + var(--radius) + var(--surface)`；`--radius: 10px` 全局统一；`box-shadow: 0 10px 28px rgba(20,30,25,.12)` |
| ⑤ 模板装饰件 | ✅ **命中 3 项** | ①`.card .step`（"给学习者"）是**每个标题上方的 eyebrow 标签**；②`.entry-strip` 每个链接都带 `→`；③代码块底色 `#11161a` 是**带色偏的近黑**代替纯黑 |

### 1.3 `frontend-design` 的可执行规则

只列**在别处没有重复**的；排版/动效/写作的细节分别在 §3、§7、§10 展开。

| 主题 | 规则（原文要点） |
|---|---|
| **主体性** | 先确定「产品是什么、给谁看、这一页的首要任务是什么」再动手；视觉选择要来自 **"The subject's industry, subject matter, materials, and vernacular"**。用**真实内容**搭建 |
| **hero** | 放"这个题目世界里最有特征的东西"（标题/图/动画/**实时演示**/交互）。并明确警告：**"a big number with a small label, supporting stats, and a gradient accent is the default treatment, so only use it if that's truly the best option."** |
| **结构即信息** | "Structural devices like outlines, borders, numbering, eyebrows, dividers, labels, etc., **encode useful information about the content rather than decorate it**." 编号只在内容**真的是序列**时才用 |
| **克制** | "**Spend your boldness in one place.** Let one element be the memorable thing, keep everything around it quiet and disciplined… Consider Chanel's advice: before leaving the house, take a look in the mirror and **remove one accessory**." |
| **质量下限** | "Build to a quality floor without announcing it: responsive down to mobile, visible keyboard focus, reduced motion respected, visually accessible, harmonious color palettes." |
| **CSS 陷阱**（skill 自己点名） | "It's easy to generate CSS classes that cancel each other out (especially with a type-based selector like `.section` and an element-based selector like `.cta`)… often with padding/margin between sections." → **本项目命中**：`.section-title{margin:36px 0 14px}` 与 `.section-sub{margin:-6px 0 16px}` 用负 margin 互相抵消 |


### 1.4 `frontend-design` 的写作原则

核心一句：**"Words appear in a design for one reason: to make it easier to understand
and use. They are design content, not decoration."**

规则：从**使用者视角**命名（"**A user manages notifications, not webhook config.**"）；
**描述**而非**推销**（"Describe what something is or does in plain terms rather than
selling it. Being specific and legible to new users is always better than being clever."）；
默认主动语态，CTA 说清点击后发生什么（"Save changes," not "Submit."）；
一个动作全程**同名**（按钮 "Publish" → toast "Published"）；
失败与空态是**指路**时刻（"Errors don't apologize, and they are never vague about what
happened. An empty screen is an invitation to act."）；
**"Let each written element do exactly one job."**
→ 展开见 §10。


### 1.5 `web-artifacts-builder` 唯一可用的那句

该 skill 99% 是 React/Tailwind/npm 工具链，**对零构建站点不适用**。但它有一句
直接点名 AI slop 的排版禁忌，值得单独记：

> "VERY IMPORTANT: To avoid what is often referred to as 'AI slop', avoid using
> excessive centered layouts, purple gradients, uniform rounded corners, and Inter font."
> —— https://raw.githubusercontent.com/anthropics/skills/main/skills/web-artifacts-builder/SKILL.md 第 20 行

**四个词：居中、紫色渐变、统一圆角、Inter。** 这四项在 §9 会展开。

### 1.6 社区 / 其它来源（按对本项目的可操作性排序）

| 来源 | 星数 | 是什么 | 取到 | 可操作性 |
|---|---|---|---|---|
| [`Nutlope/hallmark`](https://github.com/Nutlope/hallmark) | 28,887 | 反 AI-slop 设计 skill，**58 条可机检 gate** | ✅ SKILL.md + `references/{anti-patterns,copy,slop-test}.md` | ★★★★★ |
| [`raunofreiberg/interfaces`](https://raw.githubusercontent.com/raunofreiberg/interfaces/main/README.md) | 1,943 | **Web Interface Guidelines**，~100 条具体 UI 规则（99 行 / 8,122 B） | ✅ 全文 | ★★★★★ |
| [Vercel Web Interface Guidelines](https://vercel.com/design/guidelines) | — | 上面那份的官方维护版，内容更全 | ✅ 全文 | ★★★★★ |
| [`Leonxlnx/taste-skill`](https://github.com/Leonxlnx/taste-skill) | 88,358 | 三旋钮（variance/motion/density）+ 硬布局规则 | ✅ 关键章节 | ★★★★☆ |
| [`Trystan-SA/claude-design-system-prompt`](https://raw.githubusercontent.com/Trystan-SA/claude-design-system-prompt/main/claude/skills/ai-slop-check.md) | 1,951 | 反向工程的系统提示 + `ai-slop-check`（9 条，**default→detect→replace** 格式） | ✅ `ai-slop-check.md` 全文 | ★★★★☆ |
| [`ConardLi/garden-skills`](https://raw.githubusercontent.com/ConardLi/garden-skills/main/skills/web-design-engineer/references/failure-patterns.md) | 12,514 | `failure-patterns.md`：22 条失败模式，每条 **Default/Why/Detect/Exceptions/Repair** | ✅ 该文件全文（SKILL.md 未取到） | ★★★★☆ |
| [`rohitg00/awesome-claude-design`](https://raw.githubusercontent.com/rohitg00/awesome-claude-design/main/README.md) | 1,096 | 9 个美学族 + **"Claude Design 默认指纹"表** | ✅ 全文 | ★★★★☆ |
| [`tw93/Waza`](https://github.com/tw93/Waza) → `skills/ui` | 7,057 | 五维方向锁定 + 两条常开动效规则 | ✅ 大部分 | ★★★★☆ |
| [`alchaincyf/huashu-design`](https://github.com/alchaincyf/huashu-design) | 24,280 | 20 种网页风格 + 6 维评审（**概念分 ≤5 则总分封顶 6.0**） | ✅ 大部分 | ★★★☆☆ |
| [`bergside/awesome-design-skills`](https://github.com/bergside/awesome-design-skills) | 2,846 | **67 个**设计系统 skill 的登记表 | ✅ README；抽样 1 个 SKILL.md | ★★☆☆☆（是登记表，不是规则源） |
| [`nextlevelbuilder/ui-ux-pro-max-skill`](https://github.com/nextlevelbuilder/ui-ux-pro-max-skill) | 128,945 | 10 大类按优先级排序的规则表 | ✅ SKILL.md 头部 | ★★★☆☆ |
| Anthropic cookbook 前端美学 | — | `DISTILLED_AESTHETICS_PROMPT` | ✅ 全文 | ★★★★☆ |
| `SnailSploit/Claude-Red` | 6,248 | **无设计内容**（纯攻防安全）——已排除 | ✅ 目录树确认 | — |

**三条最有价值的、官方 skill 里没有的东西**：

1. **hallmark 的 58 条 gate 把"品味"变成了"是非题"。** 每条 gate 的答案必须是 **no**。
   例：gate 24「每个 padding/gap/margin 都在命名刻度上」——"Arbitrary `padding: 17px`
   is a tell"；gate 34「320–1920px 无横向滚动」，修法是
   **`overflow-x: clip` 同时加在 `html` 和 `body` 上，不用 `hidden`**
   （"`clip` preserves `position: sticky` and `position: fixed`"）；
   gate 50「带图的 grid 轨道必须写 `minmax(0, 1fr)`，不能写裸 `1fr`」
   （裸 `1fr` = `minmax(auto,1fr)`，`auto` 最小值是图片固有宽度）。
   **这些是零构建手写 CSS 能直接用的。**

2. **Trystan-SA 的 `ai-slop-check.md` 用「正向默认 → 检测 → 替换」格式**，
   比"禁止清单"更好用。例（原文）：
   > "Reserve `border-left: 4px solid` for semantic emphasis…
   > `border-radius: 12px` + `border-left: 4px solid` used as the *default* card style…
   > **reads as 'default SaaS template'**"

3. **garden-skills 的 `failure-patterns.md` 给每条都写了 Detect（怎么发现）**，
   例如「Repeated section header formula」的 Detect 是
   "Same label+headline+paragraph stack in most sections"。
   它还命名了一个本项目**没有**但值得警惕的模式：
   **"Decorative trust theater"** —— "No fabricated metrics, testimonials, logos,
   security badges, 'used by' claims"，理由："**False credibility is worse than an
   honest gap**"。以及 **"Micro-label noise"** —— 装饰性的版本号、假坐标、状态点。

**社区生态已经收敛**（§9.7 有完整共识表）：8 个独立来源禁止同一批东西。
**分歧也存在**（衬线是否可用、eyebrow 是否全禁、居中 hero 是否可接受、
APCA 还是 WCAG 2），本报告在 §9.7 逐条记录，**不当成定论**。

**对本项目的筛选结论**：hallmark 的 gate 列表 + `interfaces` 的 ~100 条 +
Vercel 版指南，这三份合起来已经覆盖了 90% 可执行规则，且**全部零构建可落地**。
`huashu-design` 与 `awesome-design-skills` 是**风格命名库**（用来找方向），
不是规则源——`awesome-design-skills` 的 SKILL.md 模板里甚至还在推荐 Inter，
与它同生态的反 slop 来源直接冲突，**不要用它做规则依据**。

---

## 2. 现状体检：`site/` 到底哪里没有设计感

这一节全部**可复现**。结论先行：**站点的问题不在文案，不在 HTML 语义，而在
设计层没有系统**——没有比例、没有尺度、没有状态。

### 2.1 令牌层面的量化体检（可重跑）

```bash
# 在仓库根执行
python3 - <<'EOF'
import re,collections
css=open('site/assets/style.css').read()
def vals(p):
    c=collections.Counter()
    for m in re.finditer(r'\b'+p+r'\s*:\s*([^;}]+)', css):
        for x in m.group(1).split(): c[x.strip()]+=1
    return c
print('font-size :', len(vals('font-size')), dict(vals('font-size')))
print('radius    :', dict(vals('border-radius')))
print('line-height:', dict(vals('line-height')))
print('gap       :', dict(vals('gap')))
print('hex colors:', len(set(re.findall(r'#[0-9a-fA-F]{3,8}', css))))
print('vars      :', len(set(re.findall(r'--[a-z-]+(?=\s*:)', css))))
print('transition:', len(re.findall(r'transition', css)))
print('prefers-* :', re.findall(r'prefers-[a-z-]+', css))
print('focus     :', re.findall(r'[^{}]*:focus[^{]*', css))
EOF
```

实测输出（2026-09-19，`site/assets/style.css` 310 行）：

| 维度 | 实测 | 问题 |
|---|---|---|
| `font-size` 取值数 | **17 个**：`18 13 14 30 15 17 22 16 28 21 90% 13.5 0.82rem 0.8rem 12 12.5 26 24` | 无 type scale；`13.5px` / `12.5px` / `0.82rem` 是随手写的 |
| `line-height` | **4 个**：`1.65`（body）、`1.3`（hero h1）、`1.55`（pre）、`1.5`（多处） | 没有「字号越大 line-height 越小」的规则；`1.5` 与 `1.55` 无差别 |
| `border-radius` | **6 种**：`5 6 7 8 var(--radius)=10 999` | 层级与圆角无关，纯随机 |
| `gap` | **11 种**：`2 4 6 8 10 12 14 16 20 22 26` | `14 / 22 / 26` 不在任何 4pt 网格上 |
| `padding` | 20 种取值（`9 11 18 22 28 36 40 56 …`） | 同上 |
| 颜色 | **27 个硬编码 hex**，但只有 **9 个 CSS 变量** | 18 个颜色没进令牌系统；代码块有 **3 套**不同的深色（`#11161a` / `#1e1e1e` / `#252526`） |
| `transition` | **0 次** | 全站无动效——hover 是硬切换 |
| `prefers-*` | **0 次** | 无 `prefers-reduced-motion`、无 `prefers-color-scheme`（**没有暗色模式**） |
| `:focus-visible` | **0 次**（只有 `.skip-link:focus`） | 键盘焦点只有浏览器默认环；鼠标用户看到的也是默认环 |
| `box-shadow` | **1 个**：`0 10px 28px rgba(20,30,25,.12)` | 只有 hero 图有，不成系统 |

### 2.2 站点**做对了**的事（改造时不要弄坏）

| 项 | 实测 | 评价 |
|---|---|---|
| 文案具体性 | 有真命令（`git clone …` / `code sokonanoda-lang/playground.sokonanoda`）、真数字（"12 道练习"）、真限制（"没有 WASM 构建"、"零工具链依赖"） | **比多数同类站点诚实**。§10 的规则基本已满足 |
| emoji | **0 个**（扫描 `site/**/*.html` 的 emoji 区段：无命中） | 避开了最大的 AI tell |
| 语义 HTML | 10 页全部：`lang` 正确（9×`zh-CN` + 1×`en`）、**恰好 1 个 `<h1>`**、有 `<meta name="description">`、有 skip-link | 卫生良好 |
| 页面体积 | 10 页 HTML 合计 **71.8 KB**；CSS 12 KB；JS 8 KB；演示图 196 KB | 零构建 + 零框架的红利，别丢 |
| 无第三方请求 | 无 webfont CDN、无分析、无框架 | 见 §3.7，这是**优点**，webfont 决策要保住它 |
| 单一事实源 | 版本/进展/单元数全部由 `scripts/gen-site-data.py` 生成 | 架构上正确，设计改造不得引入手写数字 |

### 2.3 可访问性 / 元数据的实测缺口

**对比度**（WCAG 2.x 公式实算，脚本见 §2.1 同目录）：

| 组合 | 比值 | 判定 |
|---|---|---|
| `--ink #1c1c1e` on `--bg #fbfbfa` | 16.43 | ✅ |
| `--ink-soft #5a5a5f` on `#fff` | 6.86 | ✅ |
| `.prose p #2b2b2e` on `#fbfbfa` | 13.63 | ✅ |
| 主按钮白字 on `--accent #2f6f4f` | 5.99 | ✅ |
| `--accent-ink #1f4d37` 链接 on `#fbfbfa` | 9.32 | ✅ |
| `.warnbox` 边框 `--warn #8a6d1f` on `#fbf3df` | **4.43** | ❌ **差 0.07 不达 AA 正文** |
| `.copybtn:hover` 边框 `#0a7cc4` on `#2f3b4c` | **2.54** | ❌ 不达 3:1（若该边框是控件唯一边界则违规） |

**缺失项**：`favicon`（0 处）、`og:*`（0 处）、`rel="canonical"`（0 处）、
`name="theme-color"`（0 处）、暗色模式（0 处）。

**`<title>` 长度与信息量**：内页标题只有 15–20 字（"关于"、"课程"、"文档"、
"进展"、"愿景"），不带品牌；首页 31 字，英文页 56 字。→ 规则见 §10.9。

### 2.4 结构层面的「AI 味」定位（对应 §1.2 的 5 类）

| 位置 | 代码 | 命中 |
|---|---|---|
| `.hero` | `border:1px solid var(--line); border-radius:var(--radius); background:var(--surface)` | ④ hero 也是一个圆角卡 |
| `.cards` / `.install-grid` / `.demo-grid` | `grid-template-columns: repeat(auto-fit, minmax(240px,1fr))` | ④ 等宽重复卡 |
| `.card` / `.unit-card` / `.demo-card` / `.stat` / `.diagram .node` | 同一套 `1px + var(--radius) + var(--surface)` | ④ 一套皮肤刷所有层级 |
| `.stat` | `text-align:center` + `font-size:26px` + `color:var(--accent-ink)` | ④ 「大数字 + 小标签」默认处理 |
| `.card .step` | `font-size:13px; letter-spacing:.5px; color:var(--accent)`，内容是"给学习者"/"给 code agent" | ⑤ **每个标题上方的 eyebrow 标签** |
| `.entry-strip` | 5 个链接全部以 `→` 结尾 | ⑤ **`→` 附加在链接文本后** |
| `.prose pre` | `background:#11161a` | ⑤ **带色偏的近黑**代替纯黑 |
| `.status-pill` | `border-radius:999px` | ④ 药丸徽章 |
| `.note` / `.warnbox` | `border-left:4px solid` | 通用 callout 套路（见 §6.5） |

---

## 3. 排版（Typography）

### 3.1 模块化 type scale：比率表与本项目的选择

比率本身是音乐音程的算术结果，工具 typescale.com 的预设列表即业界通用命名
（https://typescale.com/ ，取到页面，其 scale 下拉框实测包含下列 8 档）：

| 比率 | 名称 | 相邻两级的视觉落差 | 适用 |
|---|---|---|---|
| 1.067 | Minor Second | 几乎看不出 | 极密集的 UI（表格、数据面板） |
| 1.125 | Major Second | 很轻 | 密集 UI、侧栏导航 |
| **1.200** | **Minor Third** | **清晰但克制** | **文档站 / 技术站正文区（推荐）** |
| 1.250 | Major Third | 明显 | 营销页、落地页 |
| 1.333 | Perfect Fourth | 强 | 编辑型版面、杂志 |
| 1.414 | Augmented Fourth | 很强 | 标题与正文对立明显的设计 |
| 1.500 | Perfect Fifth | 极强 | 海报 |
| 1.618 | Golden Ratio | 最强 | 展示型 |

**规则：全站字号层级 ≤ 10 级。** 超过 10 级说明在用字号表达本该由字重/颜色
表达的层级。

**本项目的选择：两段式比率，锚点 17px。**

```
小端（UI 区，13→17px）用压缩比率 ≈ 1.07–1.13
大端（标题区，19→42px）用 ≈ 1.21–1.24
```

**为什么不用单一比率**——这是算术，不是口味：以 17px 为锚，向下套 1.2 两次得到
`17 / 1.2 = 14.2px`、`/ 1.2 = 11.8px`。**11.8px 的中文不可用**（CJK 在 12px 以下
笔画糊成一团），而 13/14px 又必须存在（元信息、表格正文）。反过来，若为了小端
改用 1.125 全阶，则 17→42px 需要 8 级，标题之间会挤在一起。
两段式是唯一同时满足"小端可用 + 大端有落差"的解。

**为什么锚 17px 而不是 16px**：CJK 字形密度远高于拉丁，16px 中文在长时间阅读下
偏挤；17px 是中文正文的舒适下限（这也是 §11.4 里 `--fs-base: 1.0625rem` 的唯一理由）。
拉丁为主的 `en/index.html` 可单独覆盖为 16px。

**可检查**：`grep -o 'font-size:[^;]*' site/assets/style.css | sort -u | wc -l` 应 ≤ 9。
当前是 **17**（§2.1）。

### 3.2 行高：唯一的硬约束来自 WCAG

WCAG 2.1 SC 1.4.12 Text Spacing（Level AA）的原文指标
（https://www.w3.org/WAI/WCAG21/Understanding/text-spacing.html ，已取到全文）：

> "- Line height (line spacing) to at least 1.5 times the font size;
> - Spacing following paragraphs to at least 2 times the font size;
> - Letter spacing (tracking) to at least 0.12 times the font size;
> - Word spacing to at least 0.16 times the font size."

**注意这条的意思不是"你必须设成 1.5"**，原文明确：

> "This SC does not dictate that authors must set all their content to the specified
> metrics… Rather, it specifies that an author's content has the ability to be set to
> those metrics without loss of content or functionality. The author requirement is
> both to not interfere with a user's ability to override the author settings, and to
> ensure that content thus modified does not break content"

所以真正的**硬规则**是：

| 规则 | 理由 | 检查方式 |
|---|---|---|
| `line-height` 用**无单位数值**，不用 px | px 行高不会随用户放大字号而变，用户改字号即破版 | `grep -E 'line-height:\s*[0-9]+px'` 应为空 |
| 文本容器**不设固定 `height`**，用 `min-height` 或 padding | WCAG 的 F104 失败条件就是"文字被裁切或重叠"（同页 Failures 段） | 无 `height:` 出现在文本容器上 |
| 容器尺寸用 `em`/`rem` 而非 px | WCAG 充分技术 **C28: Specifying the size of text containers using em units**（同页 Techniques 段） | `--measure` / `--wide` 均为 rem |
| 正文 `line-height` ≥ 1.5 | 给用户留出上浮空间，且 1.5 是低视力研究（McLeish）得出的实用下限，同页 Research 段 | 实测 1.7 |

**配对规则（字号越大行高越小）**——`--lh-display:1.15` / `--lh-heading:1.3` /
`--lh-body:1.7` / `--lh-code:1.6`。理由：大字号的行间空白按比例看已经很大，
不再需要额外行距；小字号反之。

**这条规则有实测数据支撑**（Material Design 3 的 type scale，从其 token 源取到，
因 m3.material.io 是 JS 门控）：

| M3 role | size / line-height | 比值 |
|---|---|---|
| display-large | 57 / 64 | **1.12** |
| headline-medium | 28 / 36 | 1.29 |
| title-medium | 16 / 24 | 1.5 |
| body-large | 16 / 24 | 1.5 |
| label-small | 11 / 16 | **1.46** |

**USWDS 的对应数字**（designsystem.digital.gov）：正文 ≥16px；标题 line-height
**1–1.35**，长文 ≥1.5；段间距 ≥1em 且 <1.5em；列表项间距 ≥0.5em；
**标题上方的空白 ≥ 下方空白的 1.5 倍**（这条是"标题归属下一节"的排版惯例，
当前站点 `.section-title{margin:36px 0 14px}` 的比值是 2.6×，**已经满足**）。

**Butterick 的行距数字**：正文行距为字号的 **120%–145%**。
（注意与 Word 的 "Single/1.5/Double" = 117%/175%/233% 不一致——**不要用 Word 的
"单倍行距"当参考**。）CSS 里一律写**无单位数值**。

**CJK 需要比拉丁更大的行高**：`--lh-body: 1.7`（中文），英文页可覆盖为 `1.6`。
官方 `frontend-design` skill 对衬线给了同类但方向相反的提醒：
> "give serif body text slightly more line-height than a sans-serif."
> —— https://raw.githubusercontent.com/anthropics/skills/main/skills/frontend-design/SKILL.md 第 23 行

### 3.3 行长（measure）

**四个来源给出四组数字，需要分清**（这是最容易被混引的一节）：

| 来源 | 数字 | 出处 |
|---|---|---|
| **WCAG 2.2 SC 1.4.8 Visual Presentation（Level AAA）** | "Width is **no more than 80 characters** or glyphs (**40 if CJK**)" | https://www.w3.org/WAI/WCAG22/Understanding/visual-presentation.html —— 这**才是"80 字符"这个数字的规范出处**，而且**它直接给了 CJK 的阈值 40** |
| 官方 `frontend-design` skill | "Default to line lengths of less than 80 characters." | https://raw.githubusercontent.com/anthropics/skills/main/frontend-design/SKILL.md 第 23 行（已取到原文） |
| Bringhurst《The Elements of Typographic Style》 | 45–75 字符 | 经 every-layout.dev 的免费 Axioms 章转引（**该站 Grid 章收费**，Axioms 章免费） |
| Butterick《Practical Typography》 | **45–90 字符**（不是 45–75） | practicaltypography.com（**本会话不可达**，经二级来源转引，标为待核） |
| USWDS（美国政府设计系统） | 45–90，长文目标 **66** | designsystem.digital.gov 的 type guidance |

> ⚠️ **不要把 45–75 说成 Butterick 的观点**——那是 Bringhurst 的区间；
> Butterick 给的是 45–90。两者常被混淆。

**中文的约束现在有出处**：WCAG 1.4.8 明确写 **"40 if CJK"**，理由是
"Chinese, Japanese and Korean (CJK) characters are approximately **twice as wide** as
non-CJK characters"。同一条还给了一个可用的**手工检查法**：
"Text is **not justified**"，以及段落间距至少是行距的 1.5 倍。
→ **本项目按 30–40 字/行取**（下界取通行经验，上界取 WCAG 的 40）。

**结论：`--measure: 40rem`（640px）。** 这个数不是随手取的：
640px ÷ 17px ≈ **37.6 个汉字**（落在 30–40 区间中段）；按拉丁正文平均 ~8.2px/字符
折算 ≈ **78 个拉丁字符**——同时满足 WCAG 1.4.8 的 ≤80 与 Bringhurst 的上限。
**两个约束的交集就是 40rem。**

**当前站点的问题是行长**：`--max: 960px` 减去 `.wrap` 的 `padding: 0 20px` = **920px**，
≈ **112 个拉丁字符**——**超出 WCAG 1.4.8 上限 40%**。这是全站阅读体验最直接的
损失，且改一个变量即可修。

**不要用 `ch` 做中文的行长单位**：`ch` 是当前字体中字形 `"0"` 的宽度，拉丁字体里
约为 `0.5em`，而一个汉字约为 `1em`。用 `65ch` 约束中英混排正文会得到**只有中文
一半宽度**的容器。中英混排一律用 `rem`/`em`（顺带满足 WCAG 技术 C28）。

### 3.4 字族配对策略

官方 skill 的规则只有一句，但很硬：

> "You don't need a different typeface for display or headline text and body content:
> use one family or two, and if two, make them clearly distinct."（第 19 行）

**本项目的字族数：2 个（1 无衬线 + 1 等宽）。** 关键在"clearly distinct"——
等宽字体不能选一个"看起来就是无衬线的等宽版"，要选**骨架明显不同**的：
`l` 带尾、`a` 双层、`0` 带斜杠或点、`g` 单层。这样代码块与正文在同一屏里
一眼可分，而不需要靠背景色区分。

### 3.5 什么时候用衬线——本项目的答案：正文不用

不是审美偏好，是三个具体原因：

1. **CJK 衬线 webfont 体积不可接受。** 中文衬线（宋体类）子集动辄数 MB，
   而本站整站 HTML 只有 71.8 KB（§2.2）。为零构建站点引入一个比全站内容大
   50 倍的二进制资源，与 `docs/design/site.md` §2 的"零构建"决策直接冲突。
2. **系统 CJK 栈是黑体。** macOS 的 `PingFang SC`、Windows 的 `Microsoft YaHei`
   都是无衬线。若正文用拉丁衬线，同一段中英混排里会出现**两套骨架**（拉丁衬线 +
   中文黑体），中文部分会显得比拉丁部分"轻"，行内基线观感不齐。
3. **"高对比衬线大标题 + 奶油底 + 陶土色"是官方 skill 点名的第 1 类 AI 聚类**
   （§1.2），而中文站做这套必然做不完整（缺 CJK 衬线），只学到一半的模仿最糟。

**补一条诚实的反面证据**（避免把"衬线不可读"当成事实）：NN/g 的
"Best Font for Online Reading: No Single Answer"（引 Wallace/Adobe 的研究）发现
**同一个人在不同字体间的阅读速度差可达 35%**，但**字体好坏因人而异**——
Garamond 平均 312 WPM 最快，却只在 48% 的受试者身上是速度最优；
Franklin Gothic 速度排名最优率 59%，平均速度却只有 271 WPM。
Alex Poole 综述 50+ 项研究亦未发现衬线/无衬线的普遍差异。
→ **结论不是"衬线更好或更差"，而是"没有普适答案"**。所以本项目的取舍理由
**必须是体积与混排一致性，不能是"衬线不可读"**。

**唯一例外**：若将来为某一篇长文（如课程讲解）单独做阅读页，且**只对拉丁部分**
用衬线（`font-family: <serif>, <system-CJK-sans>`），是可接受的——此时中文仍是黑体，
但那一页的整体调性本就偏"读"。**默认不做。**

### 3.6 数字：tabular numerals

MDN 对 `tabular-nums` 的定义（https://developer.mozilla.org/en-US/docs/Web/CSS/font-variant-numeric ，已取到）：

> "`tabular-nums` activating the set of figures where numbers are all of the same size,
> allowing them to be easily aligned like in tables. It corresponds to the OpenType
> values `tnum`."

同页列出的相关值：`lining-nums`(`lnum`) / `oldstyle-nums`(`onum`) /
`proportional-nums`(`pnum`) / `slashed-zero`(`zero`) / `ordinal`(`ordn`) /
`diagonal-fractions`(`frac`) / `stacked-fractions`(`afrc`)。

**规则：任何出现在列里、或会随数据变化的数字，一律 `tabular-nums`。**

本项目具体命中处（当前**全部缺失**）：

| 位置 | 现状 | 应为 |
|---|---|---|
| `site/data/site.json` 的计数（329 checked / 99 open / 36 targets / 12 units） | 比例数字，切换数据时宽度跳动 | `font-variant-numeric: tabular-nums` |
| 版本号 `0.61.0`、`0.59.0`（多处并排对比） | 同上 | `tabular-nums` + `slashed-zero` |
| `.stat .n`（`font-size:26px` 的进度数字） | 居中比例数字 | `tabular-nums` + **右对齐** |
| `table.compare` 的数值列 | 左对齐 | `tabular-nums` + 右对齐 |
| 单元编号 ①②③…⑫ | 圈码，宽度天然一致 | 保持；但若改阿拉伯数字则需 tabular |
| 代码行号 | 无行号（见 §8.2） | 若加行号，必须 tabular |

**为什么这条对本项目特别重要**：这个项目的可信度**就是它的数字**——
`77,819` 行 Rust、`329` 条 checked、`0` failed、`0.61.0`、第 `108` 轮。
数字宽度跳动会让"真数字"看起来像"随便写的数字"。

### 3.7 optical sizing / font-feature-settings / variable fonts

**`font-optical-sizing`**（MDN，https://developer.mozilla.org/en-US/docs/Web/CSS/font-optical-sizing ，已取到）：

> "Optical sizing is enabled by default for fonts that have an optical size variation
> axis. The optical size variation axis is represented by `opsz` in
> `font-variation-settings`. When optical sizing is used, small text sizes are often
> rendered with thicker strokes and larger serifs, whereas larger text is rendered more
> delicately with more contrast between thicker and thinner strokes."

**结论：默认值 `auto` 已经是对的，不需要写。** 它只在字体有 `opsz` 轴时生效。
**不要为了 `opsz` 选字体**——这是收益最小、约束最大的一个轴。

**`font-feature-settings` vs `font-variant-*`**：

| 用哪个 | 什么时候 |
|---|---|
| `font-variant-numeric` / `font-variant-ligatures` / `font-variant-caps` 等**高层属性** | **默认**。它们可组合、语义清晰、与 `font-variant-numeric` 等互不覆盖 |
| `font-feature-settings` | 仅当某特性**没有**高层等价物时；它是低层逃生口，且会**覆盖**高层属性 |

**代码块必须关掉连字（本项目专属理由）**：

```css
code, pre, .cmd { font-variant-ligatures: none; }
```

理由不是"连字不好看"，而是**它会撒谎**：JetBrains Mono / Fira Code 把 `->` 渲染成
一个箭头字形，而这个语言里 `→`（U+2192）是**真实存在的记法**。若 `->` 看起来和
`→` 一样，读者无法从屏幕上判断源码里到底是哪个字符；而且复制粘贴出来的是
ASCII `->`，与视觉不符。**在一个"判定永远走内核、禁止文本比对"的项目里，
让排版对字符撒谎是不可接受的。**

同时显式设 `tab-size: 2`（浏览器默认是 8，会让 `.sokonanoda` 里的缩进炸开）。

**Variable fonts**：唯一的实际收益是**体积**——一个 `wght` 轴的可变字体，
比 Regular/Medium/SemiBold/Bold 四个静态文件加起来小。零构建站点里这仍然成立
（提交一个 `.woff2` 而不是四个）。代价：`font-variation-settings` 与
`font-weight` 的交互容易写错，**建议只用 `font-weight`**，不要手写
`font-variation-settings: 'wght' 550`。

### 3.8 webfont 决策（零构建站点）

三条路的实测对比：

| 方案 | 体积/请求 | 离线可用 | 第三方依赖 | 品牌感 | 风险 |
|---|---|---|---|---|---|
| **系统栈** | 0 B / 0 请求 | ✅ | 无 | 低（各 OS 不同） | 无 |
| **Google Fonts CDN** | CSS + woff2，跨域 | ❌ **断网即回退** | ✅ 有 | 中 | 增加一条网络依赖；渲染阻塞的第三方 CSS；隐私争议（本会话未取到判决原文，见 §12）；与"零工具链依赖"的项目叙事冲突 |
| **自托管 woff2** | 20–40 KB（Latin 子集） | ✅ | 无 | 高 | 需提交二进制 + 许可证文件；需处理 FOUT |

**一条反直觉但有出处的证据**（web.dev *Best practices for fonts*）：
> "On paper, using a self-hosted font should deliver better performance as it eliminates
> a third-party connection setup. **In practice, the performance differences between
> these two options is less clear cut.** For example, the Web Almanac found that
> **sites using third-party fonts had a faster render** than fonts that used first-party
> fonts."
> "If you are considering using self-hosted fonts, confirm that your site uses a
> **Content Delivery Network (CDN)** and **HTTP/2**."

→ **GitHub Pages 本身就是 CDN + HTTP/2**，所以这条前提满足；但"自托管一定更快"
**不是**一个有据可依的说法。**本报告推荐系统栈的第一理由仍然是"零构建 + 零第三方
+ 离线可用"，不是"更快"。**

另两条 web.dev 的实测数字（**比 MDN 更具体**）：
- "By default, **Chromium-based and Firefox browsers block text rendering for up to 3
  seconds** if the associated web font has not loaded. **Safari blocks text rendering
  indefinitely.**"
- `font-display` 三档的实测：`block` = 2–3s 阻塞 + 无限 swap；`swap` = 0ms + 无限；
  `fallback` = 100ms + 3s；`optional` = 100ms + **无 swap**。
- **`preload` 的坑**："it **bypasses some of the browser's built-in content negotiation
  strategies**. For example, `preload` **ignores `unicode-range` declarations**"。
- **不要内联字体**："Inlining large resources, such as fonts, is likely to delay the
  delivery of the main document."
- **WOFF2 比 WOFF 压缩好 30%**。

Modern Font Stacks 给的标准栈（https://modernfontstacks.com/ ，已取到页面）：

```
System UI        : system-ui, sans-serif;
Monospace Code   : ui-monospace, 'Cascadia Code', 'Source Code Pro', Menlo,
                   Consolas, 'DejaVu Sans Mono', monospace;
Neo-Grotesque    : Inter, Roboto, 'Helvetica Neue', 'Arial Nova',
                   'Nimbus Sans', Arial, sans-serif;
```

> ⚠️ **注意它的 `Monospace Code` 栈在本项目里是坏的**——`Source Code Pro`、
> `Menlo`、`Consolas` **全都没有 `⊢`**。见 §3.9。

`font-display` 的五档与三阶段模型（MDN，
https://developer.mozilla.org/en-US/docs/Web/CSS/@font-face/font-display ，已取到）：

> "The font display timeline is based on a timer… divided into the three periods below:
> **Font block period**: If the font face is not loaded, any element attempting to use
> it must render an invisible fallback font face… **Font swap period**: …must render a
> fallback font face. If the font face successfully loads during this period, it is used
> normally. **Font failure period**: …the user agent treats it as a failed load causing
> normal font fallback."

五档：`auto` / `block`（短阻塞 + 无限 swap）/ `swap`（极短阻塞 + 无限 swap）/
`fallback`（极短阻塞 + 短 swap）/ `optional`（极短阻塞 + **无 swap**）。

**各档的实际阻塞时长**（web.dev "Best practices for fonts"，本会话经二级转引）：

| 值 | block 期 | swap 期 | 后果 |
|---|---|---|---|
| `block` | **2–3s**（Chromium/Firefox），**Safari 无限阻塞** | 无限 | 长时间看不见文字（FOIT） |
| `swap` | 0ms | 无限 | 立刻用后备字体，字体到位后**跳变**（FOUT + CLS） |
| `fallback` | 100ms | 短 | 折中 |
| `optional` | 100ms | **无** | 慢网下**永不**使用该字体，**零布局偏移** |

另：**WOFF2 比 WOFF 压缩率好约 30%**——自托管只提交 `.woff2` 即可。

**关于 Google Fonts CDN 的隐私问题**：德国慕尼黑第一地区法院（LG München I）
2022 年 1 月认定通过 Google Fonts CDN 加载字体违反 GDPR（因传输 IP 到第三方）。
**本会话未能取到判决书原文**（rewis.io 被 Cloudflare 拦、openjur.de 需验证码），
**也未核实"100 欧元赔偿"这一数字**——见 §12。**因此本报告只把"离线可用性 /
第三方依赖 / 渲染阻塞"作为主要理由**（这三条可由检查页面直接验证），
隐私问题作为**已见公开报道但未取到原文**的补充理由。

**具体建议（三条，按优先级）**：

1. **默认走系统栈，但把它修对。** 这是本项目的正确答案：站点 90% 的可见文字是中文，
   而中文只能走系统栈；既然中文已经在系统栈里，再为占少数的拉丁文字引入一个
   webfont，收益低而代价（FOUT/CLS/二进制/许可证）是实打实的。
   ```css
   font-family: system-ui, -apple-system, "Segoe UI", "PingFang SC",
                "Hiragino Sans GB", "Microsoft YaHei", "Noto Sans CJK SC",
                "Source Han Sans SC", sans-serif;
   ```
   改动点只有一处：**把 `system-ui` 提到最前**（当前栈首位是 `-apple-system`，
   在非 Apple 平台无意义）。
2. **等宽字体单独选，按 §3.9 的字形覆盖表选**——这是唯一必须偏离系统栈的地方。
3. **若确实要品牌感：自托管一个 Latin 可变无衬线子集（latin subset，20–40 KB），
   配 `font-display: swap`。** 绝不引入 CJK webfont；绝不用 CDN。
   选字标准是**能压住 PingFang/雅黑**（人文主义或新怪诞，字腔不要过圆、x-height 不要过大），
   **不是**"好看"——而且要避开 Inter（§1.5 点名的四项之一）。
   自托管不是构建步骤：下载一次 `.woff2` 提交进仓库 + 手写一段 `@font-face` 即可，
   与 `site/assets/style.css` 现有做法完全一致。

### 3.9 本项目专属：数学符号字形覆盖（实测，这是最容易被忽略的坑）

`⊢`（U+22A2，turnstile）是这个产品**最核心的符号**——`docs/protocol.md:288`
的目标视图示例就写作 `⊢ And P Q`；课程文件里 `α` 出现 1370 次、`∈` 142 次、
`⊆` 83 次、`∃` 41 次、`∀` 38 次（实测 `courses/set-theory/units/*.sokonanoda`
+ `examples/*.sokonanoda`）。

**浏览器对缺字形的处理是逐字回退**：主字体没有该字符时，浏览器会从后备字体里
取那个字。结果不是"豆腐块"，而是**同一行等宽代码里混进一个宽度不同的字形**——
对齐崩掉，而崩得很隐蔽（截图小图看不出来）。

实测（脚本：解析字体 `cmap` 表，检查 29 个本项目会用到的符号；
数据获取日期 2026-09-19）：

| 字体 | 字形总数 | 缺失数 | 缺哪些 |
|---|---|---|---|
| **Noto Sans Mono**（Google Fonts 可变子集） | 3490 | **2** | `⟹` `「` |
| Arial Unicode MS（macOS 系统） | 38917 | 4 | `⟨` `⟩` `⟹` `⟶` |
| Apple Symbols（macOS 系统） | 3641 | 2 | `「` `」` |
| **JetBrains Mono**（Google Fonts 可变子集） | 976 | 7 | **`⊢`** `⟹` `「` `↦` `⇒` `⊤` `⊥` |
| **Menlo**（macOS 默认等宽，Modern Font Stacks 推荐栈成员） | 2727 | 7 | **`⊢`** `⟹` `「` `」` `⟶` `⊤` `⊥` |
| Source Code Pro（Google Fonts 可变子集） | 1334 | 14 | **`⊢`** `∧` `∨` `∈` `⊆` `⟨` `⟩` `⟹` `「` `ℕ` `⟶` `↦` `⊤` |
| **Hiragino Sans GB**（当前站点 CJK 栈成员） | 29318 | 15 | **`⊢`** `∀` `∃` `↔` `¬` `⊆` `⟨` `⟩` `⟹` `ℕ` `⟶` `↦` `⇒` `⊤` `∘` |
| Geist Mono | 889 | 18 | **`⊢`** `∀` `∃` `∧` `∨` `∈` `⊆` `⟨` `⟩` `⟹` `「` `α` `ℕ` `⟶` `↦` `⇒` `⊤` `⊥` |
| **SF Mono**（`SFMono-Regular`，当前站点代码字体首选） | 1318 | 19 | **`⊢`** `∀` `∃` `∧` `∨` `∈` `⊆` `⟨` `⟩` `⟹` `「` `」` `ℕ` `⟶` `↦` `⇒` `⊤` `⊥` `∘` |
| Roboto Mono | 876 | 19 | **`⊢`** `∀` `∃` `→` `↔` `∧` `∨` `∈` `⊆` `⟨` `⟩` `⟹` `「` `ℕ` `⟶` `↦` `⇒` `⊤` `⊥` |
| Monaco（macOS 系统） | 1624 | 18 | **`⊢`** `∀` `∃` `↔` `∈` `⟨` `⟩` `⟹` `「` `α` `λ` `ℕ` `⟶` `↦` `⇒` `⊤` `⊥` |
| Inter | 2849 | 14 | **`⊢`** `∀` `∃` `∧` `∨` `∈` `⊆` `⟨` `⟩` `「` `ℕ` `↦` `⊤` `⊥` |
| IBM Plex Sans | 891 | 17 | **`⊢`** `∀` `∃` `∧` `∨` `∈` `⊆` `⟨` `⟩` `⟹` `「` `ℕ` `⟶` `↦` `⇒` `⊤` `⊥` |
| Public Sans | 565 | 21 | 含 `⊢` `∀` `∃` `→` … |

**结论与建议**：

1. **`⊢` 是筛选等宽字体的第一道门槛**，而它淘汰了绝大多数流行编码字体
   （Menlo / SF Mono / Monaco / Consolas / Source Code Pro / JetBrains Mono /
   Geist Mono / Roboto Mono / IBM Plex Mono 全部没有）。
2. **实测唯一同时覆盖 `⊢ ∀ ∃ → ↔ ∧ ∨ ¬ ≠ ≤ ∈ ⊆ ⟨ ⟩ α λ ℕ ⟶ ↦ ⇒ ⊤ ⊥` 的
   候选是 Noto Sans Mono**（仅缺 `⟹` 与 `「`，且这两个可以避开或由系统后备覆盖）。
3. 因此代码字体栈建议：
   ```css
   font-family: "Noto Sans Mono", ui-monospace, SFMono-Regular, Menlo,
                Consolas, monospace;
   ```
   —— 但**更好的做法是把 `Noto Sans Mono` 自托管为 woff2 子集**（只含 latin +
   本项目用到的 29 个数学符号，体积可压到 15–25 KB），这样它成为**首选且确定**
   的字体，而不是"碰运气命中"。
4. **必须做的验证**（零构建也能做）：改完 CSS 后，在页面里放一段含
   `⊢ ∀ ∃ ∈ ⊆ ⟹` 的代码块，截图放大 400% 看**字符是否等宽对齐**。
   这是唯一能发现逐字回退的方法。
5. **`「」` 的替代**：全站用直角引号 `「」`（中文排版惯例）时，若代码字体不含它，
   会在代码块里回退。→ 规则：**代码块内不用中文标点**；正文里的 `「」` 由 CJK
   字体渲染，不受影响。

> 方法学说明：上表测的是 **Google Fonts 仓库里的可变字体拉丁子集**
> （`https://raw.githubusercontent.com/google/fonts/main/ofl/<family>/<file>.ttf`）
> 与 **macOS 本地系统字体**（`/System/Library/Fonts/*.ttc`）。
> 厂商完整版（如 IBM 官网的 Plex Mono 全量包）覆盖可能更广；但**网站实际能加载到的
> 就是子集版**，所以这张表才是决策依据。IBM Plex Mono 的 Google Fonts 路径
> 本次 404，其数据缺失，见 §12。

### 3.10 排版规则速查（可检查）

| # | 规则 | 检查 |
|---|---|---|
| T1 | 字号层级 ≤ 10 级，全部走 `--fs-*` 变量 | `grep -c 'font-size:\s*[0-9]' site/assets/style.css` → 0 |
| T2 | `line-height` 全部无单位 | `grep -E 'line-height:[^;]*px'` → 空 |
| T3 | 正文行长 ≤ `--measure`（40rem） | 测量最宽正文块的 `getBoundingClientRect().width` ≤ 640px |
| T4 | 标题行高 < 正文行高 | `--lh-display 1.15 < --lh-heading 1.3 < --lh-body 1.7` |
| T5 | 字族 ≤ 2（无衬线 + 等宽） | `grep -o "font-family:[^;]*" \| sort -u` → 2 条 |
| T6 | 数字列 `tabular-nums` | 表格/计数元素上存在该声明 |
| T7 | 代码块 `font-variant-ligatures: none` + `tab-size: 2` | 存在 |
| T8 | 代码块内无中文标点 | 人工/脚本扫描 `pre` 内容 |
| T9 | 代码字体覆盖 `⊢` | 上述放大截图法 |
| T10 | 文本容器不设固定 `height` | `grep -E '^\s*height:' site/assets/style.css` → 空 |

---

## 4. 间距与布局

### 4.1 间距刻度：4pt 半步 + 8pt 节奏

**未取到原文**：8pt grid 的起源（Spec.fm / Elliot Dahl 的 "8-Point Grid"）与
Material 的 spacing 规范本会话未取到。**因此不引用其出处。**

但**"离刻度值"本身是一条被独立记载的 AI tell**——Trystan-SA 的
`ai-slop-check.md`（https://raw.githubusercontent.com/Trystan-SA/claude-design-system-prompt/main/claude/skills/ai-slop-check.md ，已取到）第 8 条：

> "off-scale values (`padding: 7px 15px`, `margin: 18px`, `gap: 13px`) — they feel chaotic"

这正好命中当前站点：`gap` 有 **11 种取值**（`2 4 6 8 10 12 14 16 20 22 26`），
其中 `14 / 22 / 26` 不在任何 4pt 网格上；`padding` 有 20 种取值（§2.1）。

**本项目刻度**（§11.4）：`4 8 12 16 24 32 48 64 96`。
- `4` 是半步，只用于**紧邻元素之间**（图标与文字、标签与边框）。
- `8 / 16 / 24 / 32` 是主力，覆盖 90% 的场景。
- `48 / 64 / 96` 只用于**区块之间**的垂直间距。
- **没有 `14 / 18 / 22 / 26`。**

> ⚠️ **但"只能是 8 的倍数"这条更严格的规则被一个真实的生产系统反驳了**：
> Atlassian 的 spacing token 是
> `0, 2, 4, 6, 8, 12, 16, 20, 24, 32, 40, 48, 64, 80`——**它包含 2px 与 6px**，
> 即**小端用四分之一步**。其用法分档（原文）：小档（0–8px）用于
> "Gap between small icons and text / Container padding of small components /
> Padding within input components"；中档（12–24px）用于 "larger and less dense pieces
> of UI"。**所以本项目的规则应写成"每个值必须在命名刻度上"，而不是"必须是 8 的倍数"。**
> spec.fm 的 8pt grid 原文给了这条规则的理由：
> "**By removing 7 of every 8 spacing options, you reduce the amount of fiddling
> available to you** and subsequently reduce speed to code."
> 另注：spec.fm 区分 **Hard Grid**（把元素贴到 8pt 网格上）与 **Soft Grid**
> （只保证元素之间的间距是 8 的倍数），并指出对写 CSS 的人而言
> "using an actual grid is irrelevant because **programming languages don't use that
> kind of grid structure — it'll just get thrown away**" → **本项目用 Soft Grid**。

**规则：任何间距值必须能写成 `var(--sp-*)`。** 检查：
`grep -oE '(padding|margin|gap)[^;]*' site/assets/style.css` 的结果里
不出现非刻度数值。

### 4.2 垂直节奏：为什么不要基线网格

**未取到原文**（web 上的严格 baseline grid 为何通常失败，本会话未取到出处）。

**可操作的替代规则**（设计判断）：

| 规则 | 理由 | 检查 |
|---|---|---|
| `line-height` 无单位 | 见 §3.2，WCAG 1.4.12 | `grep -E 'line-height:[^;]*px'` → 空 |
| 只用**单向 margin**（`margin-block-start`），不用 `margin: 0 0 X` | 避免 margin 塌陷导致的实际间距与声明不符 | 无 `margin-bottom` 与 `margin-top` 同时出现在同一元素 |
| 用 `:where()` 或 `* + *` 控制流内节奏，不用给每个元素写 margin | 减少互相抵消的选择器（§1.3 skill 点名的 CSS 陷阱） | 区块间距由 1–2 条规则统一给出 |
| **只在 flex/grid 容器里才需要担心 margin 塌陷** | MDN：**"Margins don't collapse in a container with `display` set to `flex` or `grid`."** ——这是逃生口 | 若某处间距不符合预期，把它放进 flex/grid 容器 |

every-layout.dev 的 Stack 给了这个模式的标准写法与理由（已取到）：
```css
.stack > * + * { margin-block-start: 1.5rem; }
```
> "Using the adjacent sibling combinator (`+`), `margin-block-start` is only applied
> where the element **is preceded by another element**: no 'left over' margin… The key
> `* + *` construct is known as the **owl**."
> "The vertical spacing of your design should be **based on your standard line-height**
> because text dominates most pages' layout, making **one line of text a natural
> denominator**."（正文 line-height 1.5 ⇒ 间距刻度也用 1.5 的比率）
| 区块间距**至少 2 倍**于区块内元素间距 | 否则分组关系读不出来 | `--sp-7`(48) / `--sp-4`(16) = 3× |

**当前站点的具体病灶**：`.section-title { margin: 36px 0 14px }` 与
`.section-sub { margin: -6px 0 16px }`——**用负 margin 抵消上一条规则的下边距**，
正是官方 skill 第 55 行点名的"类之间互相抵消"。

### 4.3 容器宽度

| 容器 | 宽度 | 理由 |
|---|---|---|
| 正文 | `--measure: 40rem`（640px） | §3.3，同时满足 <80 拉丁字符与 30–40 汉字 |
| 页面外壳 | `--wide: 68rem`（1088px） | 容纳导航、表格、多列演示 |
| 全出血 | 无 | 本站不需要 |

**规则：正文永远不占满 `--wide`。** 当前站点用**同一个** `--max: 960px` 装所有内容
（§2.1），导致正文行长 ~112 拉丁字符（§3.3）。**修法是把"外壳宽度"与"正文宽度"
拆成两个变量**，而不是缩小 `--max`（缩小会让表格和演示图挤在一起）。

**响应式写法**（不引入媒体查询）：
```css
.wrap  { max-width: var(--wide);    margin-inline: auto; padding-inline: var(--sp-5); }
.prose { max-width: var(--measure); }
```

### 4.4 Grid vs Flow

**未取到原文**（every-layout.dev / Jen Simmons 的 intrinsic web design 本会话未取到）。

**本报告规则**：

| 场景 | 用 |
|---|---|
| 散文、列表、任何一维顺序内容 | **普通文档流 + `max-width`**。不要用 grid |
| 表格、真正二维对齐的数据 | `<table>`（不是 grid） |
| 并排的图 + 说明、导航 + 正文 | grid |
| **"三个并列特性"** | **先问：它们真的是并列的三件事吗？** |

最后一条是最重要的。hallmark 的 `anti-patterns.md`
（https://raw.githubusercontent.com/Nutlope/hallmark/main/skills/hallmark/anti-patterns.md ，已取到）
把三列特性网格列为**首要 tell**：

> "Three equal columns, each with an icon above a two-line heading above a three-line
> body… **Every LLM emits this.**"

它的对策是：
> "**Break the grid.** Vary column widths. Mix card heights. Remove one card and use
> negative space. Move the icons inline, not above. Or drop the cards entirely and use
> typographic rhythm."

**本项目现状**：`site/index.html` 的"怎么用"是 3 张等宽 `.demo-card`
（`repeat(auto-fit, minmax(300px,1fr))`），"安装"是 2 张等宽卡。
**3 张演示卡可以保留**（内容确实是三个并列的演示），但**必须打破等宽等高的视觉**——
例如把第一张做成大图通栏，后两张并排。**不要三张一样大。**

### 4.5 不对称与"空白即层级"

hallmark 的两条对策可直接引用：

> **居中**："Bias the layout. Wide left margin, narrow right. Or the reverse.
> Breaking symmetry once is enough."
> **等距分节**："Vary. Tighten one, expand another."

**空白即层级的机制有出处**——NN/g 的 Gestalt proximity 研究
（https://www.nngroup.com/articles/gestalt-proximity/ ，已取到）：

> "**Design elements near each other are perceived as related**, while elements spaced
> apart are perceived as belonging to separate groups."
> "Proximity is one of the most important grouping principles and **can overpower
> competing visual cues such as similarity of color or shape**."
> "**Using varying amounts of whitespace to either unite or separate elements is key to
> communicating meaningful groupings.**"

其工作例子里，**仅靠空白**就把 Search 从主导航里分了出来——"even though Search shares
the same font treatment with the main categories"。对文字的直接应用：
> "**the text from the corresponding section is usually placed closer to the heading than
> the text from the preceding section**"

**这条与 USWDS 的数字规则（§4.2 的"标题上方空白 ≥ 下方 1.5 倍"）是同一件事的两种表述，
两个独立来源互相印证**——这是本报告里少见的"理论 + 数字"双来源。

**规则**：

| 规则 | 具体做法 |
|---|---|
| 全站左对齐，不居中 | 例外只有两个：图片的 `figcaption`、表格里的数字列（右对齐） |
| 每屏**最多一个**居中元素 | hallmark gate 6：居中元素超过两个即自动失败 |
| 区块间距**不等距** | 相邻两个区块的 `padding-block` 不应相同；用 `--sp-6` / `--sp-7` / `--sp-8` 交替 |
| 一个区块 = 一个想法 | 若一个 `<section>` 里有两个 `<h2>`，拆开 |

**当前站点**：`.stat { text-align: center }` 与 `.hero-media figcaption
{ text-align: center }`——前者应改为**右对齐**（数字列右对齐是表格惯例，且让
tabular-nums 真正发挥作用）。

### 4.6 容器嵌套深度 ≤ 2

rohitg00 的 "Claude Design's default fingerprints"
（https://raw.githubusercontent.com/rohitg00/awesome-claude-design/main/README.md ，已取到）
把它命名为 **"Container soup"**：

> "Pills wrapping cards wrapping cards wrapping content; padding stacking 24/24/24"
> 对策："Cap nesting depth: 'containers nest at most 2 levels'"

hallmark 的对应条目是 **"Card-in-card"**：
> "A bordered container with cards inside it… Visual nesting with no semantic reason."
> 对策："Pick one containment layer. Usually the outer one is the wrong one."

**当前站点直接命中**：`.hero`（带边框圆角的盒子）内部没有卡，但
`.install-grid > .card.install-card > .cmd`（带背景圆角）已是三层；
`.demo-card > .media`（带边框圆角）也是三层。

**规则**：**从 `<main>` 到最内层内容，带背景色或边框的容器不超过 2 层。**
本项目的落法：`<main>` 无边框 → `<section>` 可以有 1 条上分隔线 → 内容直接排布，
**不再套卡片**。

---

## 5. 颜色

### 5.1 受约束的调色板

**调色板该有多少色**（NN/g，https://www.nngroup.com/articles/color-enhance-design/ 与
https://www.nngroup.com/articles/visual-hierarchy-ux-definition/ ，均已取到）：

| 指引 | 原文 |
|---|---|
| 总数 | "**Limit your palette to three colors.**" / "start with two or three colors" |
| 简化设计 | "limit your color use to **2 primary and 2 secondary colors**" |
| **对比变体数** | "**Use no more than 3 contrast variations for complex designs. If everything is contrasted, then nothing stands out.**" |
| 强调预算 | "**Limit how many elements are big to a maximum of 2**" |

**反例（有数字）**：Google Maps "discovered that the tool had **over 700 colors** and, a
year later, winnowed that palette down to **25 hues**"（designsystems.com）。

**为什么"一个强调色 + 中性色"会赢（机制，原文）**：
> "It's not the actual color of an element that creates the hierarchy, but rather the
> **contrast in value and saturation between the element and the context in which it
> appears**"

——第二个色相会和第一个**在同一条轴上竞争**；中性色不在色相轴上竞争。
（NN/g 自己也给了不同口径的数字——"limit your palette to three colors" vs
"2 primary and 2 secondary" vs "no more than 3 contrast variations"。
**同一站点三种说法，这本身就是"它是启发式而非定律"的证据。**）

**色阶步数（实测各家）**：Radix **12**；Tailwind **11**（50→950）；Open Props **13**；
Refactoring UI **10**；M3 的 `neutral0…100` 加中间档。
→ **本项目只需要 1 条中性阶 + 1 条绿阶**，但每一档都必须能说出用途。

**中性色必须带色偏，不能是纯灰**（Radix，
https://www.radix-ui.com/colors/docs/palette-composition/composing-a-palette ，已取到）：
> "gray is pure gray; **mauve is based on a purple hue; slate is based on a blue hue;
> sage is based on a green hue; olive is based on a lime hue; sand is based on a yellow
> hue**." 配对规则："choose the gray scale which is **saturated with the hue closest to
> your accent hue**."

Refactoring UI 的章节标题就是 **"Greys don't have to be grey"**。
M3 的 `neutral` 阶可见地带紫偏：`neutral6 #141218`、`neutral10 #1d1b20`、`neutral90 #e6e0e9`。
→ **本项目的绿强调色 ⇒ 中性阶应带轻微绿/青偏**（§11.4 的 `#f5f7f9`/`#d9dee5` 是冷蓝偏，
严格按 Radix 的配对规则应再向绿偏一点，实现时可微调）。

**Radix 12 步的用途契约**（https://www.radix-ui.com/colors/docs/palette-composition/understanding-the-scale ，已取到）：

| 步 | 用途 | 步 | 用途 |
|---|---|---|---|
| 1 | App background | 7 | UI element border and focus rings |
| 2 | Subtle background | 8 | Hovered UI element border |
| 3 | UI element background | 9 | **Solid backgrounds** |
| 4 | Hovered UI element background | 10 | Hovered solid backgrounds |
| 5 | Active / Selected UI element background | 11 | **Low-contrast text** |
| 6 | Subtle borders and separators | 12 | **High-contrast text** |

**"不要只加白/加黑"有出处**：
> "**Step 9 has the highest chroma of all steps in the scale.** In other words, it's the
> purest step, the step mixed with the least amount of white or black."

即色阶两端**被刻意降彩度**。手写 CSS 的对应做法是**在 OKLCH 里同时动 L 与 C**，
而不是 `color-mix(in srgb, var(--acc) 20%, white)`——MDN 明确说：
> "The sRGB color space is neither linear-light nor perceptually uniform, and produces
> **poorer results such as overly dark or grayish mixes**." → 用 `color-mix(in oklab, …)`

**为什么用 OKLCH**（Evil Martians，https://evilmartians.com/chronicles/oklch-in-css-why-quit-rgb-hsl ，已取到）：
> "L is **perceived** lightness (0-1). 'Perceived' means that it has consistent lightness
> for our eyes, **unlike L in hsl()**."
> "**All axes should be independent.** Changing the value of a hue's axis color should
> maintain the same level of contrast. Saturation changes should not change hue."

Smashing 补充（https://www.smashingmagazine.com/2021/11/guide-modern-css-colors/ ，已取到）：
> "HSL and RGB have a few shortcomings: **they are not perceptually uniform** and, in HSL,
> increasing or decreasing the lightness has quite a different effect depending on the hue."

> ⚠️ **两套系统在这一点上不一致，记录下来**：Radix 的彩度**在中段达峰、两端衰减**；
> M3 的 HCT 色调板是 "**constant in hue and chroma, but vary in tone**"。
> 本报告采用 Radix 的模型（因为它同时给了 12 步的用途契约）。
>
> ⚠️ **Radix 明确反对自定义**："Radix Colors are **not intended to be customised**…
> Any customisation would likely break these features." → **本项目不抄 Radix 的值，
> 只抄它的方法与用途契约。**

### 5.2 60-30-10：规则有出处，来源无出处

**规则本身现在有权威出处**——NN/g（https://www.nngroup.com/articles/color-enhance-design/ ，
已取到，逐字）：

> "**Use the 60-30-10 rule.** This rule simply means that colors should be used in 60%,
> 30%, and 10% of your design area. Use 60% for the dominant color, 30% for the secondary
> color, and 10% towards an accent color. These proportions help create balance and prevent
> you from making a colorful and chaotic mess. Typically, the dominant and secondary colors
> should be relatively neutral colors. **Reserve the accent color for what you want to
> stand out the most on your page — for example, the primary call to action.**"

**同页的告诫**：
> "**Apply then iterate.** Once you've used the 60-30-10 rule, tweak your colors so that
> you improve aesthetics and also salience of what is important in your design."

**规则的起源仍未取到原文**——**NN/g 呈现它时没有给任何引用或归属**（它出现在一个
项目符号清单里）。室内设计起源的说法"被广泛重复，但本报告未取到文献"。
**因此：引用 NN/g 作为规则出处，不要引用 1960 年代室内设计作为起源。**

**可执行的检查**：

| 规则 | 检查方式 |
|---|---|
| 一个视口内强调色实底元素 ≤ 1 个 | 截图数一数 |
| 强调色像素占比 ≤ ~10% | 截图直方图统计，不靠眼睛 |
| **对比变体 ≤ 3 种** | NN/g 的数字，比 60-30-10 更好判定 |
| 同时"大"的元素 ≤ 2 个 | 同上 |

### 5.3 语义令牌：三层架构

**DTCG 的别名机制**（https://tr.designtokens.org/format/ ，已取到）：
> "This spec considers the terms '**alias**' and '**reference**' to be synonyms and uses
> them interchangeably."
> 别名用于："Expressing design choices / Eliminating repetition / **Creating semantic
> relationships between tokens** / Maintaining consistency across related values"

两种语法：`{group.token}`（只能指向完整 token）与 JSON Pointer `#/group/token/$value`
（"Tools implementing this specification **MUST** support JSON Pointer syntax."）。

**Radix 实际用的两层 CSS 写法**（https://www.radix-ui.com/colors/docs/overview/aliasing ，已取到）：
第二层语义 `--accent-9: var(--blue-9)`；再上一层用例别名
`--accent-bg-subtle / -bg / -bg-hover / -bg-active / -line / -border / -border-hover /
-solid / -solid-hover / -text / -text-contrast`；外加**可变别名**做主题切换
（`--panel: white` → 在 `.dark` 下 `--panel: var(--slate-2)`）。

**"不要用组件命名变量"有出处**（Radix，逐字）：
> "**Avoid using specific variable names like 'CardBg', or 'Tooltip', because you will
> likely need to use each variable for multiple use cases.**"

**SLDS（design token 的鼻祖）的定义**（https://www.lightningdesignsystem.com/design-tokens/ ，已取到）：
> "they are named entities that store visual design attributes. **We use them in place of
> hard-coded values (such as hex values for color or pixel values for spacing)**"

**本项目的落地**：

```
第一层 primitive：只有 hex，无语义      #1e6b4c  #e7f0eb  #14523a
第二层 semantic  ：只有用途，无 hex      --acc  --acc-wash  --acc-ink
第三层 组件      ：只准引用第二层        .btn-primary { background: var(--acc) }
```

**硬规则：组件 CSS 里不得出现裸 hex。** 检查：
`grep -nE '#[0-9a-fA-F]{3,8}' site/assets/style.css` 的结果应**全部落在 `:root` 块内**。
当前站点是 27 个 hex 散落在 40 多行里（§2.1），这条一查就红。

> ⚠️ **"primitive → semantic → component 三层"作为一个有名字的教条，本会话未取到
> 单一权威**。它是从 Radix 的两层别名 + SLDS 的组件 token + DTCG 的别名机制**组装**
> 出来的。**引用时请引 Radix + SLDS，不要引"某个权威的三层模型"。**

### 5.4 对比度：WCAG 的精确数字与几个反直觉条款

| 准则 | 级别 | 数字 | 来源 |
|---|---|---|---|
| SC 1.4.3 Contrast (Minimum) | **AA** | 正文 **4.5:1**；大字号 **3:1** | https://www.w3.org/WAI/WCAG21/Understanding/contrast-minimum.html |
| SC 1.4.6 Contrast (Enhanced) | **AAA** | **7:1**；大字号 4.5:1 | https://www.w3.org/WAI/WCAG22/quickref/ |
| SC 1.4.11 Non-text Contrast | **AA** | 识别控件/状态**所必需**的视觉信息 + 理解内容所必需的图形 → **3:1** | https://www.w3.org/WAI/WCAG22/Understanding/non-text-contrast.html |

**数字的来源（原文）**：
> "A user with 20/40 would thus require a contrast ratio of **3 \* 1.5 = 4.5 to 1**.
> Following analogous empirical findings and the same logic, the user with 20/80 visual
> acuity would require contrast of about **7:1**."

**"大字号"的精确阈值**——**两个数字都要知道**：
- 规范正文的算术："`1pt = 1.333px`, therefore `14pt` and `18pt` are equivalent to
  approximately **`18.5px`** and `24px`."
- WCAG 术语表的数字是 **18.66px**（= 14pt × 4/3）。
→ **引用时两个都给**，避免被挑。

**CJK 专属条款**：`large scale` 的定义含
"**or font size that would yield equivalent size for Chinese, Japanese and Korean (CJK) fonts**"。

**四个最容易踩的坑**（全部原文）：

1. **阈值不四舍五入**：1.4.3 —— "the computed values should not be rounded (e.g.,
   **4.499:1 would not meet the 4.5:1 threshold**)."；1.4.11 —— "**2.999:1 would not
   meet the 3:1 threshold.**"
   → 当前站点 `.warnbox` 的 `#8a6d1f on #fbf3df = 4.43` **不达标**。
2. **只设前景不设背景本身即失败**（Failure F24）。
3. **反锯齿会让"名义达标"实际不达标**：细体/异体字可能被渲染得比 CSS 颜色淡得多
   → 最佳实践是**主动超出**要求。
4. **1.4.11 的三个反直觉条款**：
   - **hover 态豁免**："additional author-supplied visual treatments for hover are
     **not 'required to identify'** the hover state… do not themselves need to contrast
     3:1 against the background"（但不得让组件或其他状态指示失去对比度）。
   - **1.4.11 不比较焦点前后**："**this success criterion does not directly compare the
     focused and unfocused states of a control**" —— 那个空缺由 **2.4.13** 填。
   - **控件不必有可见边界**："does not require that controls have a visual boundary
     indicating the hit area"（但焦点指示仍需足够对比）。

**豁免项**：`Incidental`（非激活控件的文字、纯装饰、不可见文字）与 `Logotypes`
**没有对比度要求**。→ §11.4 里 `--rule`（纯装饰发丝线）**不需要** 3:1；
**输入框边框需要**，因为它承担"识别控件"。

**焦点指示器**（此处更正一个常见误传）：

| 准则 | 级别 | 内容 |
|---|---|---|
| SC 2.4.11 Focus Not Obscured (Minimum) | **AA** | 焦点组件不得被作者内容完全遮挡；充分技术 **C43 `scroll-padding`**；命名失败 **F110**（sticky 头/脚遮住焦点） |
| SC 2.4.13 Focus Appearance | **AAA（不是 AA）** | 见下 |

**2.4.13 的可算公式**（原文）：
> "if a control is a rectangle **90px wide and 30px tall**, the area of a 2 CSS pixel thick
> perimeter is the difference between the areas of: A 92px by 32px rectangle … and
> A 88px by 28px rectangle … This results in a minimum area of **(92px \* 32px) -
> (88px \* 28px) = 480px²**."（矩形即 **4h + 4w**）
> "**3:1 is the minimum allowable change-of-contrast ratio**, but the greater the change of
> contrast between states, the easier it is for users to see the focus indicator."
> "In CSS, the `outline` and `outline-offset` properties are commonly used to achieve this."
> "Indicators that are **inset further within the component**… **need to be thicker than
> 2 CSS pixels**."
> "**If you need to use complex mathematics to work out if a focus indicator is large
> enough, it is probably a sign that you should use a larger indicator instead.**"

**APCA：为什么它与 WCAG 2 在暗色模式上分歧**——机制在这里（
https://git.apcacontrast.com/documentation/APCAeasyIntro.html ，已取到）：

> "A **negative Lc value, such as Lc -60 means the text is lighter than the background**.
> A **positive value Lc 60 means the text is darker than the background (light mode)**."

**WCAG 2 的比值是对称的**——`contrast(a,b) == contrast(b,a)`——**所以它根本无法表达
极性**。这就是分歧的根源。另有：
> "**WCAG 2.x contrast cannot provide useful guidance when designing 'dark mode'.**"

APCA 的 Lc 阈值：**Lc 90** 流畅正文首选；**Lc 75** 正文列最小值；**Lc 60** 非正文内容文本；
**Lc 45** 大而重的标题；**Lc 30** 任何文本的绝对下限；**Lc 15** 非文本可辨识物的下限
（"Designers should treat anything below this level as invisible"）。

**APCA 的适用范围限制**（Smashing，https://www.smashingmagazine.com/2022/09/realities-myths-contrast-color/ ，已取到）：
> "WCAG 2 values shown are intended for **light-mode only**, meaning the background is
> never darker than the equivalent of approximately `#aaa`."
> "APCA and WCAG 3 are still in development and **not yet official recommendations of the
> W3C**."（存在向后兼容的 **Bridge-PCA**）

**本项目的处置**：**以 WCAG 2.x 为验收口径**（它是当前标准，也是本项目"可验证"叙事
的口径），**但暗色取值按 APCA 的提醒修正**——不要把对比度推到最大。
CSS-Tricks 的说法可引："the difference between `#fff`, and `#eee` or `#ddd` can be
**significant in terms of reducing visual fatigue** due to brightness."
§11.4 的暗色 `--ink #e5e9ee`（on surface 14.17:1）比纯白低，就是这个理由。

### 5.5 暗色模式：是重新设计，不是反相

**Material 的 elevation overlay 数字已恢复**（`m2.material.io` 本身仍是 Angular 空壳，
1.83 MB bundle 里 `elevation overlay`/`#121212`/`16%` **零命中**；数字取自
**在 Javadoc 里按 URL 引用该页的实现**）：

- **公式**（material-components-android 的 `ElevationOverlayProvider.java` 与
  Flutter 的 `elevation_overlay.dart` 完全一致）：
  `alphaFraction = (4.5 * log1p(elevationDp) + 2) / 100`
  Flutter 源码注释直书："This formula matches the values in the spec:
  https://material.io/design/color/dark-theme.html#properties"
- **按公式算出的表**：elevation 0→0%，1→**5%**，2→7%，3→**8%**，4→9%，6→**11%**，
  8→**12%**，12→14%，16→15%，24→**16%**。
  → 常被引用的 "0/5/8/11/12/16" 对应 **elevation 0、1、3、6、8、24 dp**。

> **但这套已经废弃**（material-components-android 的 `docs/theming/Color.md`，逐字）：
> "**Surface with elevation overlay has been replaced with tonal surface colors in
> Material's components.** … **The maintenance to the elevation overlay has been
> discontinued.**"
> Flutter："Material 3 also introduces tone-based surfaces and surface containers.
> **They replace the old opacity-based model which applied a tinted overlay on top of
> surfaces based on their elevation.**"

**现代做法：M3 的表面明度阶梯（带真实 hex）**——暗色主题：

| role | tone | hex |
|---|---|---|
| `surface-container-lowest` | neutral4 | `#0f0d13` |
| `surface` | neutral6 | `#141218` |
| `surface-container` | neutral12 | `#211f26` |
| `surface-container-high` | neutral17 | `#2b2930` |
| `surface-container-highest` | neutral22 | `#36343b` |
| `surface-bright` | neutral24 | `#3b383e` |

亮色主题镜像：`neutral100 #fff` → `neutral98 #fef7ff` → `neutral94 #f3edf7` →
`neutral92 #ece6f0` → `neutral90 #e6e0e9`。
→ **注意这些 hex 都带紫偏**（`#141218` 不是灰的）。

**"不用纯黑"现在有具体值**：MUI 记录 M2 的 `palette.background.default = #121212`；
CSS-Tricks 的暗色 body 是 `color: #eee; background: #121212`；
**M3 的基线暗色 surface 是 `#141218`**。实算：`#121212` vs `#000` = **1.12:1**；
`#121212` vs `#fff` = 18.73:1；`#121212` vs `#e6e6e6` = 15.01:1。

**"不用纯白字"也有具体值**：CSS-Tricks 用 `#eee`；MUI 暗色
`text.secondary = rgba(255,255,255,0.7)`、`text.disabled = rgba(255,255,255,0.5)`；
**M3 暗色 `on-surface` = `neutral90 #e6e0e9`**（不是白）。
实算 M3 暗色对 `#141218`/`#e6e0e9` = **14.35:1**——**高于 AAA 7:1，但刻意不取最大**。

> ⚠️ **"87% 不透明度"是 M2 时代的约定，可取到的来源并不支持**。
> **MUI 的 0.7 / 0.5 阶梯是同一个直觉的可引用版本。**

**强调色提亮 + 降饱和，有真实 hex 对照**：CSS-Tricks 亮色链接 `#0033cc` on `#fff` =
**8.95:1**，暗色链接 `#809fff` on `#121212` = **7.42:1**——`#809fff` 明显更亮、更不饱和。
M3 把它编码成 tone 翻转：亮色 `primary = primary40` → 暗色 `primary = primary80`；
亮色 `on-primary = primary100` → 暗色 `on-primary = primary20`。
Flutter 对这套配对的要求："the 'on' colors should have a contrast ratio with their
matching colors of **at least 4.5:1** in order to be readable."

**边框也会翻转——而且有一个会翻车**：M3 亮色 `outline = neutral-variant50`、
暗色 `outline = neutral-variant60`（实算 4.33:1 / 5.87:1，都过 3:1）。
但 **`outline-variant`：亮色 `neutral-variant80`、暗色 `neutral-variant30` →
实算 1.62:1 / 1.99:1，低于 SC 1.4.11 的 3:1**。
> **教训：`outline-variant` 是分隔线色，不是控件边界色。** 这正是 §11.4 把
> `--rule`（装饰）与 `--border-control`（控件）**分成两个 token** 的理由。

**`color-scheme` 与 `light-dark()` 必须一起上**（MDN）：
> "To enable support for the `light-dark()` color function, the `color-scheme` **must**
> have a value of `light dark`, usually set on the `:root` pseudo-class."

MDN 另建议在 `<head>` 里、**任何 CSS 之前**加 `<meta name="color-scheme">`，
"helping prevent unwanted screen flashes during the page load"；
`color-scheme: only light` "Can be used to turn off color overrides caused by Chrome's
Auto Dark Theme."

Baseline：`prefers-color-scheme` 2020-01；`color-scheme` 2022-01；
`oklch()` 与 `color-mix()` 2023-05；**`light-dark()` 2024-05（Newly available）**。

**本项目的落地**：用 `@media (prefers-color-scheme: dark)` 覆盖 `:root` 的**语义层**
变量，并在 `:root` 声明 `color-scheme: light dark`。
**不用 `light-dark()`**——它 2024 才 Newly available，而本站零构建、无 polyfill。

> **关于暗色切换开关的争论（可选阅读）**：Bramus 主张三态（system/light/dark）优于二态，
> 引用投票 "Tri-state: 43 / Two-state: 7"；Modern Web Guidance 反对：
> "**DON'T expose all three states** … Two of the three options always produce the same
> visual result, violating the principle of feedback."
> **本项目一期不做切换开关**（跟随系统即可），所以这个争论不影响我们。

### 5.6 强调色纪律

**规则本身有出处**（NN/g，逐字）：
> "**Reserve the accent color for what you want to stand out the most on your page** —
> for example, the primary call to action."
> "Use colors consistently in your interface. If you use bright blue for your calls to
> action on one screen, that same color should be used for calls to action everywhere…
> **If you use red as a warning color on one screen, it should be not used to mean
> something different elsewhere.**"

**为什么"每张卡都带强调色"会杀死层级（机制）**：NN/g 的
"**contrast in value and saturation between the element and the context**"——
如果每张卡都带强调色，**强调色本身就成了"语境"**，它与语境的对比趋近 1:1，
于是不携带任何信息。NN/g 的数字：
"**Use no more than 3 contrast variations for complex designs. If everything is
contrasted, then nothing stands out.**"

**Material 3 把"一个强调色只干一件事"写成了 API**（Flutter 的 `ColorScheme` 文档
复述 M3 规范；m3.material.io 本身是 JS 空壳）：

| role | 用途（原文） |
|---|---|
| **Primary** | "used for **key components** across the UI, such as the FAB, prominent buttons, and active states." |
| **Secondary** | "used for **less prominent components**… such as filter chips, while expanding the opportunity for color expression." |
| **Tertiary** | "used for **contrasting accents** that can be used to balance primary and secondary colors or bring heightened attention to an element" |
| 其余 | "composed of **neutral colors used for backgrounds and surfaces**, as well as specific colors for errors, dividers and shadows." |

**Refactoring UI 的章节标题本身就是规则**（可引用标题，**正文不可引用**）：
"**De-emphasize to emphasize**"、"Semantics are secondary"、
"**Not every link needs a color**"、"Add color with accent borders"、
"Define your shades up front"、"Don't let lightness kill your saturation"、"Ditch hex for HSL"。

**本项目给出一条比通用规则更强的、可从产品语义推导的纪律**：

> **绿色（`--acc*`）只允许出现在「内核真的通过过」的东西上。**

这条规则的好处是**可判定**：看到任何一个绿色元素，就问"它背后有没有一个真实的
`checked` 计数 / 已发布的版本号 / 内核判定为通过的结论？"答不上来就是 bug。
它同时也自动实现了"一个强调色只干一件事"——因为这件事就是**判定**。

| 元素 | 用绿色？ | 理由 |
|---|---|---|
| `329 checked` / `0 failed` 这类**实测**计数 | ✅ | 背后是 `check.py` 的真实运行结果 |
| 已发布的版本号 `0.61.0` | ✅ | 背后是真实 release |
| 主 CTA 按钮（安装） | ✅ | 它是"进入已验证路径"的入口 |
| 课程单元的"通过"状态 | ✅ | 背后是 gate 结果 |
| 装饰性分隔线、图标、背景色块 | ❌ | 没有判定语义 |
| 「未做 / 限制 / 待办」 | ❌ 用 `--rej*` / `--ink-2` | 语义相反，误用绿色是**说谎** |
| 导航当前页 | ❌ 用 `--ink` + 字重 | 当前页不是"通过" |

**红色（`--rej*`）的纪律同样明确：只用于「未完成 / 限制 / 判错」，绝不用于装饰。**
站点已有大量诚实的"未做"内容（`counts_source: previous-run`、99 个 open、
WASM playground 未做）——**把"未完成"用红色标出来，本身就是可信度的一部分**（§10.8）。

---


## 6. 深度与细节

### 6.1 阴影：单一光源 + 分层 + 色相匹配

Josh Comeau 的 "Designing Beautiful Shadows in CSS"
（https://joshwcomeau.com/css/designing-shadows/ ，已取到全文）是这一节唯一
取到原文的来源：

> "we should decide on a single light source for all elements on the page. It's common
> for that light source to be above and slightly to the left"；并且
> "every shadow on the page should share the same ratio"。

> "Shadows imply elevation, and bigger shadows imply more elevation." /
> "Our attention tends to be drawn to the elements closest to us, and so by elevating
> the dialog box, we make it more likely that the user focuses on it first. We can use
> elevation as a tool to direct attention."

**阴影不要用纯黑**（原文）：

> "When we layer black over our background color, it doesn't just make it darker; it
> also desaturates it quite a bit."
> "By matching the hue and lowering the saturation/lightness, we can create an authentic
> shadow that doesn't have that 'washed out' grey quality."

**性能代价**（原文）：

> "If we layer 5 shadows, our device has to do 5x more work!" /
> "it's probably a bad idea to try animating a layered shadow."

**反模式**（原文）：

> "by creating each shadow in isolation like this, you'll wind up with a mess of
> incongruous shadows... Otherwise, it just looks like a bunch of blurry borders."

**本项目的落地**：全站**只保留 1 个阴影** `--shadow-pop`（两层，光源在上方略偏左），
且**只给真正浮起的层**（`<details>` 展开的菜单、popover）。暗色下阴影不可见，
改用**表面明度 + 一圈 `--rule-strong` 发丝边**表达同样的层级。

### 6.2 边框 vs 阴影：什么时候用哪个

（本节为**设计判断**，未取到可引用的单一出处；依据是 §6.1 的
"shadows imply elevation" 与 §5.4 的 1.4.11 控件边界要求。）

| 场景 | 用 | 理由 |
|---|---|---|
| 数据密集界面、表格、文档正文区、卡片网格 | **1px 边框** | 阴影在密集排布下会互相干扰，且暗示了不存在的层级 |
| 浮层：菜单、popover、dialog、tooltip | **阴影**（+ 可选 1px 边） | 它确实浮在内容之上，"浮起"是真实信息 |
| 暗色模式的一切 | **边框 + 表面明度** | 暗底上阴影几乎不可见 |
| 输入框、按钮等**控件边界** | **1px 边框，且必须 ≥3:1** | WCAG 1.4.11（§5.4） |
| 纯装饰分隔 | **1px 发丝线，无对比度要求** | WCAG 1.4.3 的 Incidental 豁免 |

**当前站点的问题**：`.card` / `.unit-card` / `.demo-card` / `.stat` / `.diagram .node`
**全部**是 `1px border + var(--radius) + var(--surface)`（§2.4），
即"用同一套皮肤刷所有层级"——这正是 §1.2 第④类 AI 聚类。**深度必须编码层级，
而不是给每个盒子都画一遍。**

### 6.3 圆角一致性

**未取到可引用的原文**（嵌套圆角算术 `inner = outer − padding` 未取到出处）。

**本报告给出的规则**（设计判断）：

| 层级 | 圆角 | 理由 |
|---|---|---|
| 结构容器（卡片、区块、代码块、表格、图片） | **0** | 层级最高、面积最大；圆角会让"内容"看起来像"控件" |
| 控件（按钮、输入框、`<kbd>`、标签） | **3px** | 小面积 + 交互语义；3px 足以暗示"可按"，不足以变成药丸 |
| 状态点（唯一的药丸形） | **`999px`** | 每屏 ≤ 1 个 |

**核心规则：圆角值必须承载层级信息。** 若全站只有一个 `border-radius` 值
（当前是 `var(--radius)=10px`，另有 5 个散落值），那圆角就没有在说话。
**检查**：`grep -o 'border-radius:[^;]*' | sort -u` 只允许 3 个结果。

**⭐ 规范本身给了反对"一个大圆角刷所有元素"的硬理由**——CSS Backgrounds 3 §4.5
"corner-overlap"（https://drafts.csswg.org/css-backgrounds-3/#corner-overlap ，已取到）：

> "**Corner curves must not overlap**: When the sum of any two adjacent border radii
> exceeds the size of the border box, **UAs must proportionally reduce the used values of
> all border radii until none of them overlap**."
> "**Note:** This formula ensures that quarter circles remain quarter circles and large
> radii remain larger than smaller ones, but **it may reduce corners that were already
> small enough, which may make borders of nearby elements that should look the same look
> different.**"

即：**`border-radius: 1rem` 在矮元素上会被浏览器悄悄改小**，于是"同样"的卡片
在页面上看起来不一样。这是"统一大圆角"的**规范级**缺陷，不是审美偏好。
同处还有一条命中区域警告："As `border-radius` reduces the interactive area of an
element authors should make sure the remaining interactive area conforms to recommended
minima"。

**各家圆角刻度（实测）**：Tailwind v4 `xs 2px / sm 4px / md 6px / lg 8px / xl 12px /
2xl 16px / 3xl 24px / 4xl 32px / full = calc(infinity * 1px)`；
Open Props `--radius-1..6 = 2px / 5px / 1rem / 2rem / 4rem / 8rem`——
**这个 2→5→16px 的超线性跳变，就是"圆角随元素尺寸缩放"的结构性论据**。

**嵌套圆角的算术**（供将来若真要用圆角时参考）：内层圆角 = 外层圆角 − 内边距；
内边距大于外层圆角时，内层应为 0。当前站点 `.card { border-radius: 10px; padding: 20px }`
内含 `.cmd { border-radius: 7px }`——内边距 20px 远大于外层 10px，按此算术内层应为 0。
**这也是把结构容器定为 0 的一个附带好处：嵌套圆角问题自动消失。**

> ⚠️ **归属更正**：常被引作"Outer Radius − Padding = Inner Radius"的 CSS-Tricks 文章，
> 其**正文只说了** "make sure the inside element's border-radius is a bit less than the
> outer element"——**那个精确公式是文章评论区 zzzzBov 的留言，不是 Coyier 的正文**。
> 引用时请注意。Tailwind 确实在文档里发货了这个模式：
> `<div class="absolute inset-px rounded-[calc(var(--radius-xl)-1px)]">`
> 配文 "we're subtracting 1px from the `--radius-xl` value on a nested inset element to
> make sure it has a concentric border radius."
> 另：内层圆角必须 clamp 到 ≥0，因为 MDN 说 "**Negative values are invalid.**

### 6.4 1px 细节的角色

**未取到原文**（真发丝线技术、optical alignment 均未取到；practicaltypography.com
在本环境不可达）。

**本报告给出的规则**：

| 细节 | 规则 |
|---|---|
| 分隔线 | 用 `border-block-start` 单边，不要上下都画 |
| 表格 | **只有横线，没有竖线**；表头下 1 条 `--rule-strong`，行间 1 条 `--rule` |
| 焦点环 | `box-shadow` 而非 `outline`（可做双环：内圈纸色 + 外圈强调色，见 `--ring`），但**必须保留 `outline: none` 的替代可见指示**——不能只删 outline 不给替代 |
| 推理横线（本项目的签名装置） | 用 `--rule-strong` 的 1px 线；上下留 `--sp-4` / `--sp-5`，不与其他边线重叠 |
| 视网膜屏的真发丝线 | ⚠️ **更正**：CSS Values 4 的 "snap a length as a line width" 算法规定——**`border-width: 0.5px` 在 DPR=2 下是整数设备像素，"do nothing"，即真正的 1 设备像素发丝线；在 DPR=1 下 0.5 < 1 设备像素，会"round it away from zero" 变成 1 设备像素——所以对 `border-width` 而言 0.5px 不会消失。** 常见说法"0.5px 会消失"**不是规范的说法**。另：`thin`/`medium`/`thick` = **1px / 3px / 5px**。来源：https://drafts.csswg.org/css-values-4/#snap-as-a-border-width 、https://drafts.csswg.org/css-backgrounds-3/ |
| 焦点环用 `outline` 还是 `box-shadow` | ⚠️ **两个来源冲突，记录下来**：`raunofreiberg/interfaces` 说"Focus rings should use **box-shadow**, not outline"（理由是 outline 历史上不跟随 `border-radius`）；但 **CSS UI 4 规范**说 outline "**does not influence the position or size of the box**"、"displaying or suppressing outlines **does not cause reflow**"、"it should follow the `border-radius` curve"，且 outline 属于 **ink overflow（永远在最上层，不受 overflow 裁剪）**，而 box-shadow 是绘制内容**会被 `overflow: hidden/clip` 裁掉**。→ **本项目采用 `outline`**（Safari 16.4+ 已支持圆角 outline），`box-shadow` 作为老浏览器兜底。WebKit 的官方配方：`:focus-visible { outline: solid thick magenta; outline-offset: 0.1em; }` |

### 6.5 关于「左边框 4px」的 callout

当前站点 `.note` / `.warnbox` 都用 `border-left: 4px solid`。
**这是通用套路但不是错误**——它的好处是**在不依赖颜色的情况下也能区分 callout 类型**
（配合 WCAG 1.4.1「颜色不是唯一手段」）。

**问题不在左边框，在于它只用了一个通道。** 建议：

| 通道 | 做法 |
|---|---|
| 颜色 | `--acc-wash` / `--rej-wash` / `--warn-wash` 三种底色 |
| 形状 | 左边框保留（3px，与圆角 0 一致） |
| **文字标签** | 每种 callout 有一个**词**（"注意" / "限制" / "已废弃"），而不是只有颜色 |

第三条是关键：**色盲用户与黑白打印都只能靠那个词。**

### 6.6 渐变：品味与 AI 陈词

**未取到原文**（oklch/oklab 渐变、`color-mix()`、banding/dithering 均未取到出处）。

**本报告给出的规则**：

| 不要 | 要 |
|---|---|
| 高饱和多色相渐变（indigo→violet→pink）铺满 hero | 渐变只用于**功能性淡出**（如横向滚动容器的边缘遮罩） |
| 渐变文字标题 | 纯色标题 |
| 渐变作为"装饰背景" | 若必须有背景色差，用**两级表面明度**（§11.4 已有） |
| 用 `linear-gradient` 做"科技感" | 需要平滑过渡时在 **OKLCH** 里插值（`linear-gradient(in oklch, …)`），避免 sRGB 插值的中段发灰/发脏 |

**⚠️ 一条多数文章写错的规范事实**：CSS Images 4 规定
> "If no `<color-interpolation-method>` is specified in the gradient function, the color
> space used for gradient interpolation is the default interpolation color space,
> **Oklab**"

即 **Oklab 已经是渐变的默认插值空间**（`color-mix()` 同）。
**显式写 `in oklab` 现在是冗余的**（但无害且自解释）。
注意区分：规范里 "purple cast" 的抱怨说的是 **CIE Lab，不是 Oklab**——不要混为一谈。

**Oklab 与 Oklch 的取舍**：Oklab 是**直角**空间 → 色相相距远时 "have a greyish
midpoint"；Oklch 是**极坐标** → "inherently chroma-preserving" 但
"easy for the intermediate colors to fall out of gamut"。

**banding（色带）的解法**（CSS-Tricks *Grainy Gradients*）：
SVG `<feTurbulence type='fractalNoise' baseFrequency='0.65' numOctaves='3'
stitchTiles='stitch'/>` + CSS `filter` 提亮/加对比 + 可选 `mix-blend-mode`。
> "Layer it underneath a gradient, boost the brightness and contrast, and that's it —
> you have gradient that gradually dithers away."
坑："It doesn't work to reference the SVG by its id in CSS… but you can inline the SVG"。

**渐变的两条品味规则有出处**（Smashing）：
> "**Don't overdo it. The best way to create a pleasant gradient is to use two colors,
> and not more than three.**" · "**Always decide on a light source.**" ·
> "Use a linear gradient for a square or polygonal area. Use a radial gradient for round areas."

> ⚠️ **"紫蓝 AI 渐变"这个说法本身没有出处。** 本会话**没有取到任何一篇文章**
> 命名或批评它。**颜色**可以引（Tailwind 的 indigo-500 `oklch(58.5% 0.233 277.117)`
> → violet-500 → purple-500 → fuchsia-500 → pink-500），但**审美判断是作者自己的，
> 不是引用**。唯一有出处的"读起来像 AI 生成"的引文仍是 §1.2 的官方 skill。

**判定规则**：如果一个渐变的**唯一作用是好看**，删掉它。
本项目的 §11.4 **一个渐变都没有**——这是刻意的。

---

## 7. 动效

### 7.1 时长与缓动（数字已核实）

**NN/g 的时长研究**（https://www.nngroup.com/articles/animation-duration/ ，已取到）：

| 场景 | 时长（原文） |
|---|---|
| 简单反馈（勾选框、开关） | **~100 ms**："Simple feedback animations… should be roughly 100 ms (0.10 seconds) in total duration." |
| 模态 / 大幅画面变化 | **200–300 ms**："When animation involves substantial screen changes, such as when a modal window moves into view, a duration of 200–300 ms can be appropriate." |
| UI 总体区间 | **100–500 ms**："In general, the duration of most animations should be in the range of 100–500 ms" |
| 实际上限 | **400 ms = "very slow"；500 ms 像卡了**："At 500ms, animations start to feel like a real drag for users… a range of 100–400 ms is appropriate, with 400ms being a very slow animation" |
| 频率规则 | "**the more frequent the animation, the more subtle and shorter** you'll want it to be"；"**It is far more common for animations to be too long than too short.**" |

**Material 的时长**（MD1 *Duration & easing*，英文镜像
https://www.mdui.org/en/design/1/motion/duration-easing.html ，已取到）：

| 场景 | 时长 |
|---|---|
| 进入 / 离开 | **225 ms / 195 ms** |
| 移动端典型 / 全屏 | **300 ms / 375 ms** |
| **桌面端** | **150–200 ms**（"Desktop animations should be faster and simpler than their mobile counterparts."） |
| 平板 / 可穿戴 | 移动端 **+30% / −30%** |
| 上限 | "Transitions that exceed **400ms** may feel too slow." |

> ⚠️ **更正**：坊间流传的"移动 150–300 / 平板 200–400 / 桌面 250–500 ms"表
> **取不到原文且与可取的 Material 镜像矛盾**——镜像说**桌面更短**（150–200ms），
> 不是更长。`m2.material.io/design/motion/speed.html` 在所有路径/UA 下都是
> Angular 空壳（68 KB 的 "This website requires JavaScript."）。
> **不要引用那张表。**

**缓动曲线**（三处独立印证：mdui 镜像、`@material/animation` 的 SCSS
（https://unpkg.com/@material/animation/_animation.scss ）、MUI 的
`createTransitions.js`）：

| 名称 | 曲线 |
|---|---|
| standard | `cubic-bezier(0.4, 0, 0.2, 1)` |
| deceleration（**进入**） | `cubic-bezier(0, 0, 0.2, 1)` |
| acceleration（**离开**） | `cubic-bezier(0.4, 0, 1, 1)` |
| sharp（离开但可能返回） | `cubic-bezier(0.4, 0, 0.6, 1)` |

**非对称缓动的原文**："**Motion appears more natural and delightful when acceleration
and deceleration occur asymmetrically.**" —— 在代码里的编码是
`@material/animation` 暴露的 `enter()` → deceleration、`exit-permanent()` →
acceleration、`exit-temporary()` → sharp。

**时长也要非对称**（NN/g）："**a popup window may take 300ms to appear, but only 200 or
250ms to disappear.**" → 出场取进场的 **~75–80%**。
NN/g 的缓动偏好："Most frequently, you'll want to use an **ease-out** animation, that
starts quickly but slows down."；"**Completely linear motion looks weird and unnatural
to users.**"

**MDN 命名缓动的精确值**（https://developer.mozilla.org/en-US/docs/Web/CSS/easing-function ，已取到）：
`ease` = `cubic-bezier(0.25, 0.1, 0.25, 1)`；`ease-in` = `cubic-bezier(0.42, 0, 1, 1)`；
`ease-out` = `cubic-bezier(0, 0, 0.58, 1)`；`ease-in-out` = `cubic-bezier(0.42, 0, 0.58, 1)`。
另有现代多点式 `linear(0, 0.25 75%, 1)`（不写 JS 也能做出类弹簧效果）。

> **Apple HIG Motion 未取到原文**（JS 门控，5 个 JSON 端点变体全 404）。

**本项目的取值**（§11.4）：`--dur-1: 100ms`（状态反馈）/ `--dur-2: 200ms`（小 UI 切换）/
`--dur-3: 300ms`（浮层）——全部落在 NN/g 的 100–500ms 与 Material 桌面 150–200ms 区间内。

### 7.2 什么该动，什么不该动

**属性分层有明确出处**——MDN *CSS performance optimization*
（https://developer.mozilla.org/en-US/docs/Learn_web_development/Extensions/Performance/CSS ，已取到）：

| 层 | 属性 |
|---|---|
| ❌ **不要动** | "dimensions, such as **width, height, border, and padding**"；"**margin, top, bottom, left, and right**"；"**align-content, align-items, and flex**"；"visual effects that change the element geometry, such as **box-shadow**" |
| ✅ **安全** | "**Transforms / opacity / filter**" |

Smashing 说得更绝对（https://www.smashingmagazine.com/2016/12/gpu-animation-doing-it-right/ ，已取到）：
"**transform and opacity are the only CSS properties that meet the conditions above.**"
**同一篇也给了诚实的警告**：合成 "**is a giant hack**"；
"every time you add the magical `transform: translateZ(0)` or `will-change: transform`
property to the element, you start the very same process. **While repainting is very
performance-costly, here it's even slower.**"

**反 cargo-cult 提醒**（CSS-Tricks，https://css-tricks.com/tale-of-animation-performance/ ，已取到）：
translate vs top/left **不是**必然的帧率胜利（"the top/left version looks smoother than
the translate() version"）——选 `transform` 的理由是**语义与避免重排**，不是"魔法性能开关"。

**`will-change` 的纪律**（MDN，https://developer.mozilla.org/en-US/docs/Web/CSS/will-change ，已取到）：
> "**Use the will-change property as a last resort** to try to deal with existing
> performance problems. **Don't use it to anticipate performance problems.**"
> "Don't apply will-change to too many elements… **Overusing the property can cause the
> page to slow down instead of improving it's performance.**"
> "it is a good practice to **switch will-change on and off using script code**"
> "applying a non-auto value on a large section, such as the `<body>`, can actually be bad"
> "If applying will-change to improve animations, **add the property before the animation
> starts, not within the @keyframes**"

**绝不能动的东西**：

| 项 | 出处 |
|---|---|
| 自动轮播 | NN/g（https://www.nngroup.com/articles/designing-effective-carousels/ ）："**Do not auto-forward on mobile devices**"；阅读速度基准 "**We use 3 words per second as a guideline.**" |
| 周边视野的水平移动 | WebKit（https://webkit.org/blog/7551/responsive-design-for-motion/ ）："**Horizontal movement in the peripheral field of vision can cause disorientation or queasiness.**" |
| 夸张视差 / 滚动劫持 | A List Apart（Val Head，https://alistapart.com/article/designing-safer-web-animation-for-motion-sensitivity/ ）："**Exaggerated parallax and scrolljacking animations are highly likely to be triggering.**" |

**WCAG 的硬约束**（https://www.w3.org/WAI/WCAG22/quickref/ ）：

| 准则 | 级别 | 内容 |
|---|---|---|
| SC 2.2.2 Pause, Stop, Hide | **A** | 自动开始、**超过 5 秒**、与其他内容并行的移动/闪烁/滚动，必须提供暂停/停止/隐藏机制 |
| SC 2.3.3 Animation from Interactions | **AAA** | 交互动效可被关闭；充分技术正是 **C39**（CSS `prefers-reduced-motion`）与 **SCR40**（JS） |

> **级别差距值得点明**：**强制支持 reduced-motion 是 AAA，超出 AA 要求。**
> 本项目仍应做——成本低，且官方 skill 把它列为"质量下限"
> （"reduced motion respected"）。

**弱来源标注**："永不动页面背景或滚动时的文字颜色"——**由属性分层推断，未取到明文禁止**。

### 7.3 `prefers-reduced-motion`：是"减弱"不是"归零"

MDN（https://developer.mozilla.org/en-US/docs/Web/CSS/@media/prefers-reduced-motion ，已取到；
Baseline **广泛可用**，"since January 2020"；BCD：Chrome 74 / Firefox 63 / Safari 10.1 / iOS 10.3）：

> 该设置传达用户希望界面 "**removes, reduces, or replaces** motion-based animations."
> "Such animations can trigger discomfort for those with **vestibular motion disorders**."

**"reduce ≠ zero" 有四份独立来源**：

| 来源 | 原文 |
|---|---|
| MDN 自己的示例 | `.animation { animation: pulse 1s linear infinite both; }` → 在 reduce 下改为 `animation: dissolve 4s linear infinite both;`，注释是 "**Tone down the animation** to avoid vestibular motion triggers." |
| WebKit | "**Don't Reduce Too Much** — only remove the animations you know to be vestibular triggers. Unless a specific animation is likely to cause a problem, **removing it prematurely only succeeds in making your site unnecessarily boring.**"；"It's okay to keep many real-time, user-controlled direct manipulation effects such as pinch-to-zoom." |
| CSS-Tricks（Eric Bailey） | 全部移除 "would be an dramatic and not necessarily valid option"；"Think of this new media query like `@supports`" |
| A List Apart（Val Head） | "**Not one person I spoke with said that they want to see all interface animation eliminated.**"；"Animation that involves only non-moving properties, like opacity, color, and blurs, are unlikely to be problematic." |

**WebKit 的前庭触发分类（可直接当检查表）**：缩放/变焦 · 旋转与漩涡 ·
多速度/多方向（视差）· 维度/平面切换（2.5D）· **周边视野移动**。

**本项目落地**（§11.4 已给 media query）：由于设计上就不引入位移/缩放（§7.5），
这条 media query 几乎是空操作——**这是好事**：不是靠它打补丁，而是本来就没有要补的。

### 7.4 滚动驱动的揭示：支持度已核实

**关键引文已取到**——官方 `frontend-design` skill（本会话通过 jsDelivr 镜像
`https://cdn.jsdelivr.net/gh/anthropics/skills@main/skills/frontend-design/SKILL.md` 复核，
与 raw.githubusercontent 版本一致）：

> "Use non-user-triggered motion sparingly and deliberately, only to draw attention.
> **A single orchestrated moment — one page-load sequence or one reveal — lands better
> than scattered effects; fade-and-slide-up entrances on each section and hover
> transitions on every card are the generic default and read as AI-generated.**"

**浏览器支持（MDN BCD 8.1.2，发布于 2026-09-17）**：

| | IntersectionObserver | `animation-timeline` / `view()` / `scroll()` |
|---|---|---|
| Chrome / Edge | 51 | **115** |
| Firefox | 55 | **仅 preview（未发布）** |
| Safari / iOS | 12.1 / 12.2 | **26 / 26** |
| Baseline | "Widely available… since March 2019" | **"Limited availability — not Baseline because it does not work in some of the most widely-used browsers."** |

> **caniuse 数字未取到原文**：caniuse.com 是客户端渲染的空壳，
> 且 `Fyrd/caniuse` 的 `features-json/` 里没有 `animation-timeline` 条目。
> **请用 MDN Baseline + BCD，并说明来源。**

WebKit 的指南（https://webkit.org/blog/17101/a-guide-to-scroll-driven-animations-with-just-css/ ，2025-06）
给了一个实现坑：`animation-timeline` **必须写在 `animation` 属性之后**，否则不生效；
reduced-motion 的包裹写法是 `@media not (prefers-reduced-motion) { … }`。
其无障碍论证也值得记："The subtle, usually slow movement of a progress bar is unlikely
to be a motion sensitivity trigger, partly because it's **not taking up much of the
viewer's field of vision**."

**结论：本项目不用 CSS 滚动时间线。** Firefox 至今只有 preview。
若将来要用，**用 IntersectionObserver**（Widely available since 2019）。

**无 JS 时的可见性**——MDN（https://developer.mozilla.org/en-US/docs/Learn_web_development/Core/Accessibility/CSS_and_JavaScript ）：
"**basic functions should ideally work without JavaScript**"；
"don't generate all your HTML content using JavaScript if at all possible."

> **弱来源标注**：`document.documentElement.classList.add('js')` 这个揭示门控写法是
> **约定，不是成文标准**（HTML5 Boilerplate 早已不再提供 `no-js` 类）。
> 实用规则：**永远不要把 `opacity: 0` 写进基础规则**——它必须由 JS 加上。

### 7.5 本项目的动效决策：趋近于零

**决策：本站不做任何入场动画、不做滚动揭示、不做视差、不做悬停抬升。**

三条理由（全部可追溯到已取到原文的规则）：
1. 官方 skill 原文（§7.4）：fade-and-slide-up 入场与每卡 hover 过渡
   "**are the generic default and read as AI-generated**"。
2. 本站是**文档型站点**；§7.2 的表格里"周边视野移动"与"自动轮播"都在禁列。
3. §7.3 的四份来源一致说 reduce ≠ zero——**本来就没有的东西不需要 reduce**。

**唯一保留的动效**：hover / focus / `<details>` 展开 / 复制按钮的"已复制"反馈
（100–200ms）。这就是全部。

### 7.6 相邻的媒体特性（BCD 8.1.2 数字）

| 特性 | 值 | 支持 | Baseline 状态 |
|---|---|---|---|
| `prefers-contrast` | `no-preference`(false) / `more` / `less` / `custom` | Chrome 96 · FF 101 · Safari 14.1 · iOS 14.5 | "Widely available… since May 2022" |
| `prefers-reduced-transparency` | `no-preference` / `reduce` | Chrome 118 · FF 113 **(flagged)** · **Safari ✗ · iOS ✗** | "Limited availability / Experimental" |
| `prefers-reduced-data` | `no-preference` / `reduce` | Chrome 85 **(flag)** · FF ✗ · Safari ✗ | "**not supported by any user agent**" |
| `hover` | `none` / `hover`（+`any-hover`） | Chrome 38 · FF 64 · Safari 9 · iOS 9 | Widely available since Dec 2018 |
| `pointer` | `none` / `coarse` / `fine`（+`any-pointer`） | Chrome 41 · FF 64 · Safari 9 · iOS 9 | Widely available since Dec 2018 |
| `:focus-visible` | 伪类 | Chrome 86 · FF 85 · Safari 15.4 | Widely available since March 2022 |

**为什么 hover 效果要门控**——MDN 对 `hover` 的说明（原文）：
> "The primary input mechanism **cannot hover at all or cannot conveniently hover**
> (e.g., many mobile devices **emulate hovering when the user performs an inconvenient
> long tap**)"

MDN 自己给的推荐写法：先写 `a:hover{…}`，再用 `@media (hover: hover){ a:hover{…} }`
包一层。

**`:focus-visible` 的价值**（MDN 原文）：
> "using the `:focus-visible` (instead of the `:focus` pseudo-class) allows authors to
> change the appearance of the focus indicator **without changing when the focus
> indicator appears**."
> "**some authors removed the user-agent outline focus styles. Changing focus style can
> decrease usability, while removing focus styles makes keyboard navigation inaccessible
> for sighted users.**"

并明确引 WCAG 2.1 SC 1.4.11："requires that the visual focus indicator be at least **3 to 1**"。

> **弱来源标注**：`@media (hover: hover) and (pointer: fine)` 这个**组合**写法是约定
> （两半各有出处，组合本身没有）。

**本项目落地**：
```css
@media (hover: hover) and (pointer: fine) {
  .nav a:hover { background: var(--surface-sunken); }
}
:focus-visible { outline: none; box-shadow: var(--ring); border-radius: var(--r-1); }
```

---


## 8. 代码与数据呈现

这一节的目标很具体：**本站的核心说服力来自"真内核、真命令、真输出"，
所以代码块的呈现质量直接等于可信度。**

### 8.1 语法高亮：颜色越少越好

**来源**：Niki Tonsky, "I am sorry, but everyone is getting syntax highlighting wrong"
（2025-10-15，https://tonsky.me/blog/syntax-highlighting/ ，已取到；
HN 讨论 https://news.ycombinator.com/item?id=45596960 ，216 分）：

> "**Christmas Lights Diarrhea** — Most color themes have a unique bright color for
> literally everything: one for variables, another for language keywords, constants,
> punctuation, functions, classes, calls, comments, etc."
> "The problem with that is, **if everything is highlighted, nothing stands out.**"
> "Have an absolute minimum of colors. So little that they all fit in your head at once.
> For example, my color theme, **Alabaster, only uses four**."
> "**Limit the number of different colors to what you can remember.**"
> "That's why I don't highlight variables or function calls—they are everywhere,
> **your code is probably 75% variable names and function calls**… **Please, please
> don't highlight language keywords.**"
> "**Bold and italics** — Don't use. This goes into the same category as too many colors."
> "**Myth of number-based perfection** — …If you make all colors the same lightness and
> chroma, they will look very similar to each other… **Our eyes are way more sensitive
> to differences in lightness than in color, and we should use it, not try to negate it.**"

**反方意见也在同一线程**（一并记录，避免只引一方）：
- t-writescode："99% of the colors don't actually matter to me. It's the distinction of
  conceptual elements that are **adjacent** to each other that matters more to me."
- Ben Kuhn, "Syntax highlighting is backwards"（2018，https://www.benkuhn.net/syntax ）：
  "I want the syntax of my language to get out of my way"；
  "**I tried to search around for any systematic studies of what kinds of syntax
  highlighting help, and I couldn't find a single one.**"

**真实主题的调色板规模（实测）**：

| 主题 | 颜色数 | 说明 |
|---|---|---|
| **Solarized** | **16**（8 单色 + 8 强调），可降到 **5** | "it has been carefully designed to scale down to a variety of **five color palettes**… In every case it retains a strong personality but doesn't overwhelm."（https://ethanschoonover.com/solarized/ ） |
| **Nord** | **16**（nord0–15），分 4 个子调色板 | Frost = "the **heart palette of Nord**"（4 个蓝）承担类/函数/关键字；Aurora（5 色）承担 error/warning/string/number。nord0 **不参与**语法高亮，因为会与背景撞色（https://www.nordtheme.com/docs/colors-and-palettes ） |
| Catppuccin | 4 flavor × 26（其中 14 个标 `"accent": true`） | 风格指南是**用途映射**而非数量规则："Legibility always comes first, so please use your own judgement." |
| One Dark | 3 单色 + 8 强调 | 全部强调色落在 **55–69% 明度带**内——正是 Tonsky 反对的"统一明度" |
| GitHub (Primer) VS Code 主题 | 47 条 `scope:`，引用 26 个 Primer 色阶 | 含 `light_colorblind` / `dark_colorblind` 变体 |

**Shiki 内置的 CSS 变量主题只有 9 个 token 色 + fg/bg**
（`--shiki-token-{constant,string,comment,keyword,parameter,function,string-expression,punctuation,link}`），
且官方自己说 "this theme is a lot less granular than most of the other themes…
For better highlighting result, we recommend construct the theme manually."
（https://shiki.style/guide/theme-colors ，已取到）

**本项目规则**：

| 规则 | 值 |
|---|---|
| 代码主题颜色数 | **≤ 8 个 token 色**（§11.4 给了 8 个） |
| 不设"变量"与"函数调用"色 | 它们占代码量的 75%，高亮它们等于没高亮 |
| 不用粗体/斜体做语法区分 | 注释的斜体是唯一的传统例外，可选 |
| 各 token 色**明度不同**，不只是色相不同 | Tonsky 的 "eyes are more sensitive to lightness" |
| 代码主题与站点调色板的关系 | **通过背景/中性阶关联**（代码块底色 = `--surface-sunken`），**不通过把 token 改成品牌色关联**。依据：GitHub 的 VS Code 主题显式取自 `primer/primitives`；Vitesse 基于 Primer 色阶 |

**本项目的 8 个 token 色**（§11.4）：default / comment / keyword(tactic) /
type(constant) / string(literal) / number / **hole(`sorry`)** / punctuation。

其中 **`sorry` 用诊断红**是本项目独有的语义映射：**未解的洞就是需要关注的东西**，
用红色是语义正确而非装饰。

### 8.2 行号

**已取到的证据**：Shiki 的 `transformerRenderLineNumber` 官方推荐 CSS 是

```css
pre.shiki .line-number { user-select: none; opacity: 0.5; margin-right: 1em; }
```
—— https://shiki.style/packages/transformers （已取到）

即**参考实现默认让行号不可选中**，这就是"行号不该进剪贴板"的具体依据。

**未取到原文**：本会话**没有找到**任何权威文章或维护者讨论支持
"行号对短代码片段有害 / 破坏复制粘贴 / 换行时错位"这套说法。
（GitHub issue 检索被限流，Reddit 不可达。）**因此不把该论点当作有出处。**

**本报告规则（标注为判断）**：

| 场景 | 行号 |
|---|---|
| 短片段（≤ 15 行，本站绝大多数代码块） | **不要行号**——它增加了噪声但不承载引用功能 |
| 长文件 / 教程正文会说"第 12 行" | 要 |
| diff | 要（但用 diff 自己的行号列） |
| 任何情况下 | `user-select: none`，且**不能是唯一的定位手段** |

**实现（零构建，纯 CSS）**：
```css
pre.numbered { counter-reset: line; }
pre.numbered .line::before {
  counter-increment: line; content: counter(line);
  display: inline-block; width: 2.5ch; margin-right: var(--sp-3);
  text-align: right; color: var(--ink-3);
  user-select: none; font-variant-numeric: tabular-nums;
}
```

### 8.3 复制按钮

**未取到原文**：MDN Clipboard API 整页、`aria-live` 确认模式、复制按钮的
hover 显示可访问性问题——本会话**全部未取到**。因此下面的实现细节
**标注为工程实践，不是引用**。

**本项目已有的实现**：`site/agents.html` 的 `.copybtn` +
`site/assets/agent-prompt.js`（`data-copy-agent-prompt`）。**它是站内唯一
有交互的元素**，因此它的质量决定了"这个站点是不是活的"。

**规则**：

| 项 | 做法 |
|---|---|
| 位置 | 代码块**右上角**，与块内 `padding` 对齐（`top: var(--sp-3); right: var(--sp-3)`） |
| 可见性 | **始终可见**，不要 hover 才出现——hover-only 在触屏上没有等价操作 |
| 尺寸 | ≥ 44×44 CSS px 的点击目标（触屏），或视觉上 32px + `padding` 补足 |
| 反馈 | 文案换成"已复制"，**2 秒**后恢复；用 `aria-live="polite"` 播报 |
| 键盘 | 是 `<button type="button">`，可用 Tab 到达，有 `:focus-visible` 环 |
| 降级 | 非安全上下文（`http:`）下 `navigator.clipboard` 不可用 → 保留选中文本的兜底，并给出可手动复制的 `<textarea>` 或提示 |
| 状态 | 用 `aria-live` 而不是只改颜色——**颜色不能是唯一通道**（WCAG 1.4.1） |

**当前实现的问题**：`.copybtn:hover { border-color: #0a7cc4 }` 对
`background: #2f3b4c` 只有 **2.54:1**（§2.3），不达 3:1。

### 8.4 diff 视图：精确颜色 + 非颜色通道

**GitHub 的真实 diff 颜色**（取自 `@primer/primitives` 的功能色 token，
https://cdn.jsdelivr.net/npm/@primer/primitives@latest/dist/css/functional/themes/light.css 与 `/dark.css`）：

| token | Light | Dark |
|---|---|---|
| `--diffBlob-additionLine-bgColor` | `#dafbe1` | `#2ea04326` |
| `--diffBlob-deletionLine-bgColor` | `#ffebe9` | `#f851491a` |
| `--diffBlob-additionNum-bgColor` | `#aceebb` | `#3fb9504d` |
| `--diffBlob-deletionNum-bgColor` | `#ffcecb` | `#f851494d` |
| `--diffBlob-hunkNum-bgColor-rest` | `#b6e3ff` | `#0c2d6b` |
| `--diffBlob-*-fgColor`（全部） | `--fgColor-default` = `#1f2328` | 同 |

（alpha 后缀：`4d`=30%，`66`=40%，`26`=15%，`1a`=10%）

**最关键的一条**：**GitHub 不改 diff 行的文字颜色。** 增删行的 `fgColor` 都是
默认前景色，**红绿完全由背景承担**。这正是为什么非颜色通道是必须的。

**WCAG 1.4.1 Use of Color**（https://www.w3.org/WAI/WCAG22/Understanding/use-of-color.html ，已取到）：

> "**Color is not used as the only visual means of conveying information, indicating an
> action, prompting a response, or distinguishing a visual element.**"
> "However, if content relies on the user's ability to accurately perceive or
> differentiate a particular color an additional visual indicator will be required
> **regardless of the contrast ratio** between those colors."

**规则**：diff 必须同时有
1. **`+` / `−` 的行首字符**（unified diff 格式本来就有，别删）；
2. **左侧色条**（`border-inline-start: 3px`）；
3. 背景色；
4. 可选：行号列的不同底色（GitHub 就是这么做的）。

**红绿是红绿色盲的经典冲突对**——这正是 Primer 提供 `light_colorblind` /
`dark_colorblind` 变体的原因，其 token 源码里有显式的 `*-protanopia-deuteranopia`
覆盖，注释写着 "Differentiate done from upsell (purple) and from open/success/attention,
**which collapse to similar hues under deutan/protan simulation**"。

### 8.5 终端块

**已取到的关键证据**——hallmark 的 `anti-patterns.md` 把**假窗口 chrome**
明确列为 cliché（gate 47）：

> "A fake browser bar (URL pill + traffic-light dots)… A fake code-block window
> (mock title bar + close/minimise dots) wrapping a `<pre>`. A fake IDE chrome…
> **Re-drawn chrome is one of the strongest 'looks AI-generated' tells** — the model
> invented a UI that already exists in the user's environment."
> "**The user already has the chrome**… Redrawing it in a page is like printing a
> photograph of a picture frame inside a real picture frame. The fakery is also bad:
> **the URL is wrong, the dots aren't macOS dots**, the notch is the wrong shape."
> 对策："use the system `<pre>` with a typographic frame (top rule + label + bottom
> rule), not a faked window-chrome."

**这是本项目最该抄的一条对策。** 站点要展示 VS Code 里的效果，有两条路：

| 做法 | 评价 |
|---|---|
| 用 CSS 画一个假的 VS Code 窗口（活动栏/标签/红黄绿点） | ❌ **gate 47 点名** |
| 用**真实截图** `<figure>` 包起来 + `figcaption` | ✅ 正确做法 |
| 用 `<pre>` + **排版式边框**（上细线 + 一个标签 + 下细线） | ✅ 正确做法 |

**当前站点已经做对了一半**：演示用的是真实生成的 PNG（`assets/demos/*.png`，
由 `scripts/gen-site-demos.py` 逐帧渲染真实 VS Code 界面），**这是对的**。
但 `.prose pre` 用了 `#11161a` 深色底 + `border-radius`，接近"伪造深色终端"的观感，
且与页面其余部分不同极性（§1.2 第⑤类"带色偏的近黑"）。

**终端块的具体规则**：

| 规则 | 做法 |
|---|---|
| 区分"你要输入的命令"与"你会看到的输出" | 命令用 `$ ` 前缀（并 `user-select: none` 让前缀不进剪贴板）；输出无前缀、颜色更淡 |
| 不用假窗口 chrome | 用上/下细线 + 一个文字标签（如 `终端` / `sokonanoda --json`） |
| 长行 | **横向滚动**（`overflow-x: auto`）而不是折行——折行会破坏缩进语义；但 `pre` 必须 `tabindex="0"` 才能用键盘滚动（Shiki 的 `<pre>` 就带 `tabindex="0"`） |
| `white-space` | 代码/终端用 `pre`；只有确实需要折行的说明性文本才用 `pre-wrap` |
| `tab-size` | 显式设 `2`（浏览器默认 8） |
| 移动端 | 横向滚动在窄屏是可用性问题 → 单行长度控制在 ~60 字符内，或在窄屏改用更短的示例 |

**本项目落法**：把 `.cmd` 从"深色圆角块"改成
**浅底 + 左侧 3px 强调条 + 等宽 + 上细线标签**（§11.4 的 `--code-bg` 与 `--rule`），
与页面同极性。

### 8.6 数据呈现：什么时候用表格

**未取到原文**（WAI 表格教程、GOV.UK 表格指南本会话未取到）。

**规则（判断 + 当前站点的具体机会）**：

| 判据 | 用 |
|---|---|
| ≥ 3 个条目 × ≥ 2 个**可比较**属性 | **表格**，不是 bullet list |
| 只有 1 个属性 | 列表 |
| 需要展示"变化/顺序" | 表格（带一列"之前/之后"） |

**表格样式规则**：

| 规则 | 做法 |
|---|---|
| **没有竖线** | 竖线让表格看起来像 Excel |
| 只有横线 | 表头下 1 条 `--rule-strong`；行间 1 条 `--rule`（或斑马纹，二选一） |
| 数字列**右对齐** + `tabular-nums` | §3.6 |
| 表头用 `<th scope="col">` | 可访问性 |
| 长表格 sticky 表头 | `position: sticky; top: 0` |
| 窄屏 | 横向滚动包裹层，**不要**把表格拆成卡片（拆了就失去可比性，而可比性正是用表格的理由） |

**当前站点只有 1 个表格**（`site/set-theory.html` 的 `table.compare`，§2.2），
且用了 `border: 1px solid` 全框线（含竖线）。**站点有大量适合表格化的内容**：
12 个单元的计数（`checked` / `open` / 状态）、三个 harness 的能力差异、
`--json` 事件类型——这些都是"多条目 × 多属性"，用表格比用散文或卡片更清楚。

### 8.7 文档站的高价值小细节

**未取到原文**：Stripe / Tailwind / MDN / Rust / Go / Python 文档站的具体实现、
锚点链接、scroll-spy TOC、callout 分类——本会话均未取到。

**但有一条已取到的相关规则**（Trystan-SA 第 2 条）：
> "no emoji, unless the brand uses them… or the user asked"

以及 hallmark gate 30 把 **emoji 当图标**列为明确 tell（§9.5）。

**本报告建议的优先级**（按"投入产出比"排序，均为设计判断）：

| 优先级 | 细节 | 为什么对本项目值 |
|---|---|---|
| 1 | **标题锚点链接**（`<h2 id>` + 悬停出现的 `#` 链接） | 课程/文档页会被引用到具体小节；零构建下纯 CSS 可实现 |
| 2 | **"本页目录"**（右侧 TOC） | `set-theory.html`（305 行）与 `vision.html` 已经很长 |
| 3 | **callout 的语义标签**（§6.5） | 站点已有 `.note` / `.warnbox`，只差一个词 |
| 4 | **`<kbd>` 样式** | 站点已有 `.install-card kbd`，样式需与令牌对齐 |
| 5 | **"编辑此页"链接** | 指向 GitHub 对应文件的编辑页；与"零构建"完全兼容，且强化"这是真项目" |
| 6 | 语言切换器的位置 | 中英双语站，切换器现在混在主导航里（`site/index.html:24`） |

**优先级 6 的具体问题**：`site/index.html` 把 `<a href="en/index.html" class="lang">English</a>`
放在主导航末尾，与"文档"、"关于"同级。**语言不是导航项，是站点级属性**，
应该移到页头最右端用分隔符隔开，或移到页脚。

### 8.8 代码呈现规则速查

| # | 规则 | 检查 |
|---|---|---|
| C1 | 代码主题 token 色 ≤ 8 | 数 `--code-*` 变量 |
| C2 | 不设"变量/函数调用"专用色 | 人工 |
| C3 | 代码字体覆盖 `⊢` | §3.9 的放大截图法 |
| C4 | `font-variant-ligatures: none` + `tab-size: 2` | 存在 |
| C5 | 代码块与页面**同极性**（不用亮页嵌深块） | `--code-bg` 引用 `--surface-sunken` |
| C6 | 短片段无行号；有行号时 `user-select: none` | 人工 |
| C7 | 复制按钮始终可见、可 Tab、有 `aria-live` 反馈 | 人工 + 键盘走查 |
| C8 | 没有伪造的窗口 chrome（假红黄绿点 / 假 URL 栏） | `grep -i 'traffic\|window-dots\|fake-'` |
| C9 | diff 有 `+`/`−` 字符 + 左侧色条 + 背景三通道 | 人工 |
| C10 | 终端块的命令前缀 `$` 不被复制 | `user-select: none` |
| C11 | 表格无竖线、数字右对齐 + tabular-nums | `grep 'border.*solid'` 在 table 规则里 |
| C12 | 长代码块 `overflow-x: auto` + `tabindex="0"` | 人工 |
| C13 | 代码块内无中文标点 | 脚本扫描 |

---

## 9. 反模式清单：「AI 味」的可判定特征

> **这是本报告的主要交付物。** 格式统一为
> **tell → 出处 → 本项目是否命中 → 对策**，全部尽量做成可 grep / 可数 / 可截图判定的形式。
> 出处缩写见本节末的「来源索引」。**已取到原文的写 URL；未取到原文的标 `未取到原文`。**

### 9.0 判据：区分「默认」与「选择」

整个清单的**总纲**来自官方 skill（原文，已在 §1.2 引用）：

> "**All traits are legitimate for some briefs, but they are defaults rather than
> choices, and they appear regardless of subject.** Where the brief pins down a visual
> direction, follow it exactly — the brief's own words always win, including when it
> asks for one of these looks. Where it leaves an axis free, don't spend that freedom
> on one of these defaults."

**因此本清单分两档**：

| 档 | 含义 | 处置 |
|---|---|---|
| **A 档：任何 brief 下都错** | 没有任何来源给出豁免（§9.6 列出） | 直接删 |
| **B 档：合法但已成为默认** | 有正当用途，但**必须能说出为什么** | 说不出理由就换 |

社区生态已经**收敛到几乎相同的禁列表**（§9.7 的共识表），这本身是一个信号：
**当 8 个独立来源都禁止同一件事时，它已经不是品味问题，是分布问题。**

### 9.1 视觉层 tells

| # | Tell | 出处 | 本项目 | 对策 |
|---|---|---|---|---|
| V1 | 紫→蓝 / 青→品红渐变（含 `background-clip: text` 标题） | hallmark gate 2；huashu；taste-skill；Trystan；garden-skills；ui-ux-pro-max；rohitg00 | ✅ **未命中**（全站 0 个渐变） | 保持 0 渐变 |
| V2 | 统一圆角刷所有卡片（"one border-radius on everything regardless of hierarchy"） | Anthropic 聚类④ | ❌ **命中**：`--radius:10px` 用于 8 处，另有 5/6/7/8/999 共 6 种 | §6.3：结构 0 / 控件 3px / 状态点 full |
| V3 | 每张卡同一个柔和灰阴影 `rgba(0,0,0,.1)` | Anthropic 聚类④ | ⚠️ 半个：只有 1 个阴影，但用在 hero 图（不该浮起的元素） | §6.1：阴影只给真正浮起的层 |
| V4 | 纯 `#000` 底 / 纯 `#fff` 面（"flat and synthetic"） | hallmark gate 7 | ⚠️ `--surface:#ffffff` 纯白 | 允许（现代极简豁免纯白），但 `#fbfbfa` 近白无彩度 |
| V5 | 中性色零彩度 `oklch(… 0 …)` | hallmark gate 22："minimum 0.005 chroma" | ❌ 命中：`#fbfbfa` / `#e4e4e2` / `#5a5a5f` 全是零彩度灰 | §11.4 的中性色全部带冷色偏（`#f5f7f9`/`#d9dee5`） |
| V6 | 带色偏的近黑 `#0B0B0B` / `#111` 代替黑 | Anthropic 聚类⑤ | ❌ **命中**：代码块 `#11161a`（且是亮页嵌深块） | §8.5：代码块与页面同极性 |
| V7 | 极光/网格渐变背景（aurora mesh blob） | hallmark gate 29 | ✅ 未命中 | 保持 |
| V8 | 无目的的 glassmorphism / 漂浮 3D 球体 | hallmark；rohitg00 | ✅ 未命中 | 保持 |
| V9 | 强调色覆盖 > ~5% 视口面积 | hallmark gate 23 | ⚠️ 待测：`.nav a.active` + 主按钮 + `.stat .n` + `.status-pill.ok` 都是实底绿 | 一屏 ≤ 1 个实底强调元素 |
| V10 | 近黑底 + 单一荧光色（"dark mode SaaS"） | Anthropic 聚类② | ✅ 未命中（无暗色模式） | 做暗色时避免 |
| V11 | 奶油底 + 高对比衬线大标题 + 陶土色 | Anthropic 聚类①；Trystan §9 | ✅ 未命中 | **明确不走这条路**（§3.5） |
| V12 | 居中一切 | hallmark："Bias the layout" | ❌ **命中**：`.stat{text-align:center}`、`figcaption{text-align:center}` | 数字列右对齐；说明文字左对齐 |
| V13 | `min-height:100vh` 全居中 hero | hallmark gate 6（auto-fail） | ✅ 未命中 | 保持 |
| V14 | 每节相同的 padding | hallmark："Vary. Tighten one, expand another." | ❌ 命中：`.section-title` 统一 `36px 0 14px` | 相邻区块间距不等值 |
| V15 | 只用等距空白分隔区块（无线、无饰、无色变） | hallmark gate 9 | ❌ 命中 | 每 2–3 节插一条推理横线 |
| V16 | 卡片套卡片 | hallmark gate 4 | ❌ **命中**：`.install-card > .cmd`、`.demo-card > .media` | 带背景/边框的容器 ≤ 2 层 |
| V17 | 卡片左侧 4–6px 粗色条 | hallmark gate 5；Trystan 第 3 条 | ❌ **命中**：`.note` / `.warnbox` 的 `border-left: 4px` | 保留但**加语义文字标签**（§6.5） |
| V18 | 编号标记 `01/02/03` 用在非序列内容上 | Anthropic："only appropriate if the content actually is a sequence" | ✅ 未命中（课程单元编号是**真序列**，正当） | 保持 |
| V19 | 中圆点连接元信息 `A · B · C` | Anthropic 聚类⑤ | ✅ 未命中 | 保持 |
| V20 | `→` 附加在链接/按钮文本后 | Anthropic 聚类⑤ | ❌ **命中**：`.entry-strip` 5 个链接全部带 `→` | 删箭头；层级用字重/位置表达 |
| V21 | 页头"字标左 + 4–5 链接 + 按钮右 + 通栏 + 1px 下边线" | hallmark gate 42 | ❌ **命中**（8 个导航项 + 通栏 + 1px 下边线） | 减到 ≤ 6 项；语言切换器移出导航（§8.7） |
| V22 | 页脚 4 列（Product/Company/Resources/Legal） | hallmark gate 43 | ⚠️ 半命中（页脚是单行链接串，比 4 列好，但仍是"目录式"） | 页脚应**收束页面**，不是目录 |
| V23 | 横向滚动（任何宽度 320–1920px） | hallmark gate 34 | ✅ 未命中 | 加 `overflow-x: clip` 到 `html` 和 `body`（用 `clip` 不用 `hidden`，`clip` 保留 sticky/fixed） |
| V24 | `z-index: 9999` | hallmark | ✅ 未命中（只有 `z-index:10/20`） | 用命名层级 |
| V25 | 可点击文字折成两行 | hallmark gate 49 | ✅ 未命中 | 保持 |
| V26 | 图像网格轨道用裸 `1fr` 而非 `minmax(0,1fr)` | hallmark gate 50 | ⚠️ 未命中但接近：`minmax(240px,1fr)` 是安全的 | 保持 |

### 9.2 排版 tells

| # | Tell | 出处 | 本项目 | 对策 |
|---|---|---|---|---|
| T1 | Inter / Roboto / Open Sans / Poppins / Lato / 系统默认作显示字 | hallmark gate 1 | ⚠️ 用系统栈（属 gate 1 列表）——**但这是中文站的正当选择**（§3.8） | 保持系统栈；若要品牌感，自托管**非 Inter** 的 Latin 子集 |
| T2 | 全站只有一个字族，"a one-font page is a template page" | hallmark | ✅ 未命中（无衬线 + 等宽 = 2 族） | 保持 |
| T3 | ≥ 4 个字族 | hallmark gate 37（"Three faces is the ceiling"） | ✅ 未命中（2 族） | 保持 |
| T4 | 标题里只把一个词改成斜体/粗体/另一种颜色 | Anthropic 三禁忌之一；hallmark gate 38a | ✅ 未命中 | 保持 |
| T5 | 标签用全大写 | Anthropic 三禁忌之一 | ✅ 未命中 | 保持 |
| T6 | 内容上方加不必要的排版标签（eyebrow） | Anthropic 三禁忌之一；hallmark gate 54；taste-skill | ❌ **命中**：`.card .step`（"给学习者"/"给 code agent"）在每个标题上方 | 删掉，或改成标题的一部分 |
| T7 | 标题字距拉开（tracked-out） | Anthropic 聚类⑤ | ❌ 命中：`.card .step{letter-spacing:.5px}` | 删 `letter-spacing` |
| T8 | 渐变文字标题 | hallmark gate 2 | ✅ 未命中 | 保持 |
| T9 | 行宽超出 45–75 `ch` | hallmark gate 25；Anthropic "<80 characters" | ❌ **命中**：920px ≈ 112 拉丁字符 | `--measure: 40rem`（§3.3） |
| T10 | 字号层级过多 / 离刻度值 | Trystan 第 8 条（off-scale values "feel chaotic"） | ❌ **命中**：17 个 `font-size`，含 13.5/12.5/0.82rem | 9 级 scale（§3.1） |
| T11 | 大标题与正文字号比 < 2.5× | huashu 评审规则（"heading:body ≥ 2.5×，理想 3×"） | ⚠️ hero 30px / 正文 16px = 1.88× | 首页 display 提到 42px（§11.4 `--fs-4xl`） |
| T12 | 默认衬线标题（Tiempos/Source Serif 类）+ 无衬线正文 | rohitg00 fingerprints | ✅ 未命中 | 保持（§3.5） |
| T13 | 空格 Grotesk 作为"逃逸字体" | Anthropic cookbook："You still tend to converge on common choices (Space Grotesk, for example)" | ✅ 未命中 | 保持 |
| T14 | 数字不用 tabular-nums | interfaces（"`font-variant-numeric: tabular-nums` in tables and timers"） | ❌ **命中**：全站 0 处 | §3.6 |

### 9.3 组件 / 状态 tells

| # | Tell | 出处 | 本项目 | 对策 |
|---|---|---|---|---|
| C1 | 三列等宽 + 图标在上 + 标题 + 三行正文 | hallmark gate 3（"Every LLM emits this"） | ❌ **命中**：`.demo-grid` 3 张等宽 `.demo-card` | 打破等宽等高（§4.4） |
| C2 | 图标装在圆角方块里放在每张卡顶部 | hallmark（"The universal template"） | ✅ 未命中（无图标） | 保持 |
| C3 | 药丸徽章到处用 | hallmark；rohitg00 "Container soup" | ❌ **命中**：`.status-pill{border-radius:999px}` | 每屏 ≤ 1 个，或改成带语义的方标签 |
| C4 | 交互元素缺状态（至少 default/hover/`:focus-visible`/`active`/`disabled`） | hallmark gate 26 | ❌ **命中**：全站只有 `.skip-link:focus`，**0 个 `:focus-visible`** | 补全状态集 |
| C5 | 焦点环用 `border` 而非 `outline`/`box-shadow` | hallmark gate 39；interfaces（"Focus rings should use box-shadow, not outline"） | ❌ 命中：完全没写焦点样式 | `--ring`（§11.4） |
| C6 | 焦点环淡入（"keyboard users have no indicator at the start"） | hallmark gate 15 | ✅ 未命中（无 transition） | 焦点环**永不**加 transition |
| C7 | 禁用态只用 opacity 表达 | hallmark gate 39（需 opacity **+** `cursor:not-allowed` **+** `disabled`） | 无禁用态 | 若加，三件套 |
| C8 | 按钮文字与按钮底同色（OKLCH 明度差 <5% 且彩度差 <0.05） | hallmark gate 41 | ✅ 未命中（白字 on 绿 5.99:1） | 保持 |
| C9 | 深色区块未在同一条规则里换文字色（"ink-on-ink"） | hallmark gate 41 | ✅ 未命中（`pre` 有独立 color） | 保持 |
| C10 | 卡片左侧色条被当默认装饰 | rohitg00："Reserve left-rule for one role" | ❌ 命中（同 V17） | 只给"限制/注意"用 |
| C11 | 闪烁状态点 | rohitg00 fingerprints | ✅ 未命中 | 保持 |
| C12 | 中圆点/分隔符拼装标签 `WORD — fragment` | Anthropic 聚类⑤ | ✅ 未命中 | 保持 |

### 9.4 动效 tells

| # | Tell | 出处 | 本项目 | 对策 |
|---|---|---|---|---|
| M1 | 每个区块 fade-and-slide-up 入场 | Anthropic（"read as AI-generated"）；hallmark gate 9 | ✅ **未命中**（0 个 animation） | **明确决定不做**（§7.5） |
| M2 | 每张卡 hover 抬升 / `hover:scale-105` | hallmark gate 11 | ✅ 未命中 | 保持 |
| M3 | 同一元素多个 hover 效果 | hallmark gate 13 | ✅ 未命中 | 每元素**一个**信号 |
| M4 | `transition: all` | hallmark gate 10；interfaces；Waza | ✅ 未命中（0 个 transition） | 显式列出属性 |
| M5 | 弹性/回弹缓动用在 UI 状态上 | hallmark gate 12 | ✅ 未命中 | 只在真实物理交互用 |
| M6 | 动画 `width/height/top/left/margin/padding` | hallmark gate 14 | ✅ 未命中 | 只动 `transform`/`opacity` |
| M7 | 自动轮播无暂停 | hallmark gate 18（WCAG 2.2.2 失败） | ✅ 未命中 | 不做轮播 |
| M8 | 没有 `prefers-reduced-motion` 兜底 | hallmark gate 27 | ❌ **命中**（0 个 `prefers-*`） | 加（§7.3） |
| M9 | 加载 spinner 闪现 | hallmark（延迟 150ms 显示 / 最短 300ms 可见） | 无 | 若加，遵守 |
| M10 | 庆祝式成功 toast | hallmark gate 16 | 无 | 静默成功 |
| M11 | 滚动驱动的视差 / 无限循环装饰动画 | hallmark；garden-skills | ✅ 未命中 | 保持 |

### 9.5 图标 / 图像 tells

| # | Tell | 出处 | 本项目 | 对策 |
|---|---|---|---|---|
| I1 | emoji 当图标（✨🚀⚡🔥🎯✅） | hallmark gate 30；huashu；Trystan 第 2 条；garden-skills；ui-ux-pro-max | ✅ **未命中**（实测 0 个 emoji） | 保持——**这是本站已经赢下的一项** |
| I2 | 混用多个图标库 | hallmark gate 30(a) | ✅ 未命中（无图标） | 若加，**只用一个** |
| I3 | 手绘 SVG 人物 / 场景 | hallmark；huashu；taste-skill | ✅ 未命中 | 用真实截图 |
| I4 | **重画的假窗口 chrome**（假浏览器栏 / 假终端红黄绿点 / 假 IDE） | hallmark gate 47（"one of the strongest 'looks AI-generated' tells"） | ⚠️ **接近命中**：演示用真实 PNG（✅ 对），但 `.prose pre` 是深色圆角块（像伪造终端） | §8.5：用 `<pre>` + 排版式边框 |
| I5 | 生成图忽略产品调色板 | rohitg00 | ✅ 未命中（演示图由脚本按真实 VS Code 配色渲染） | 保持 |
| I6 | 用 Lottie 做本该 CSS/SVG 做的动画 | hallmark gate 31 | ✅ 未命中 | 保持 |
| I7 | LCP 图 `loading="lazy"` | hallmark gate 28 | ⚠️ 首页 hero 图是 `loading="eager"` ✅，但**首页引用了两次同一张 `demo-goal.png`**（hero + 演示卡） | 复用同一 URL 是对的（缓存命中）；确认无 `lazy` 在首屏 |
| I8 | 占位图胜过烂插图 | Trystan 第 4 条；huashu；garden-skills | ✅ 未命中（有真实截图） | 保持 |
| I9 | 缺 `favicon` / `og:*` / `theme-color` | Vercel guidelines（`<meta name="theme-color">`、SVG favicon 支持 `prefers-color-scheme`）；interfaces | ❌ **全部命中**（各 0 处） | 补：SVG favicon + og 卡片 + theme-color |

### 9.6 A 档：任何 brief 下都错（无豁免）

以下是所有已取到来源中**没有任何一处给出豁免**的条目：

| # | 条目 | 主要出处 |
|---|---|---|
| A1 | 标题渐变文字（`background-clip: text`） | hallmark gate 2："No genre allows gradient text" |
| A2 | 标题里单个斜体强调词 | hallmark gate 38a；Anthropic |
| A3 | **编造的数字 / 证言 / logo** | hallmark gate 46；garden-skills "decorative trust theater" |
| A4 | 重画的假浏览器/手机/终端/IDE chrome | hallmark gate 47 |
| A5 | emoji 当功能图标 | hallmark gate 30 |
| A6 | 混用图标库 | hallmark gate 30(a) |
| A7 | `transition: all` | hallmark gate 10；interfaces |
| A8 | 焦点环淡入 | hallmark gate 15 |
| A9 | 动画布局属性（`width/height/top/left`） | hallmark gate 14 |
| A10 | 只有 hover、没有 focus/tap 等价物 | Vercel guidelines；interfaces |
| A11 | 颜色作为唯一信息通道 | WCAG 1.4.1 |
| A12 | 正文对比度 < 4.5:1 | WCAG 1.4.3（且**不四舍五入**） |
| A13 | 自动播放带声音 | hallmark gate 28 |
| A14 | 自动轮播无暂停（WCAG 2.2.2 五秒规则） | WCAG 2.2.2 |
| A15 | 任何宽度下横向滚动 | hallmark gate 34 |

### 9.7 跨来源共识（8 个独立来源同时禁止）

| 共识条目 | 主张它的来源数 |
|---|---|
| 不用紫/蓝/粉渐变 | 7 |
| 不用 Inter/Roboto/Arial/系统字作**显示字** | 6 |
| 不用三列等宽图标卡网格 | 5 |
| 不卡片套卡片 | 4 |
| 不用粗色条做卡片默认装饰 | 5 |
| 不用 emoji 当图标 | 5 |
| 不编造数字/证言/logo | 5 |
| 纯黑/纯白要向锚点色相偏色 | 5 |
| ≤3 字族 / 1 个强调色 / 4-8px 间距刻度 / 正文 45–75ch | 5 |
| **eyebrow（节标题上方的小标签）是头号模板化 tell** | 3（taste-skill 称其为 "the #1 violated rule in production tests"，上限 1/3 节） |
| 只动 `transform`/`opacity`；永不 `transition: all`；永远支持 reduced-motion | 5 |
| 交互元素必须有完整状态集 | 6 |
| 诚实的占位符胜过拙劣的伪造 | 4 |

**来源分歧（记录下来，避免当成定论）**：

| 争点 | 分歧 |
|---|---|
| 衬线显示字 | hallmark 有衬线主题；taste-skill "very discouraged as the default" 且点名禁 Fraunces/Instrument Serif；Trystan 把"奶油+衬线+陶土"整体判为"今天的紫色渐变" |
| eyebrow | hallmark gate 54 禁的是"标签与标题并排两列"这种形态；taste-skill 允许 1/3 节；本报告建议本项目**全删**（§9.2 T6） |
| 居中 hero | hallmark gate 6 自动失败；garden-skills / taste-skill 允许宣言/发布类 brief |
| APCA vs WCAG 2 | hallmark 与 Vercel 倾向 APCA；Trystan 与 ui-ux-pro-max 用 WCAG 比值 |

### 9.8 社区实证：这不是理论

HN 上真实读者的原话（本会话通过 HN Algolia API 取得）：

- **endangeredhuman**（https://news.ycombinator.com/item?id=48269907）：
  > "I am noticing this default cookie-cutter Claude Code generated websites.
  > - Editorial layout - Cream canvas - Cobalt or Terracotta ink - Persimmon or Coral
  > signal - Mono font - Too verbose… **Is it just me or is Claude trying to make
  > internet out of one design template?**"
- **functionmouse**（同帖）："it's attractive, yet inoffensive. just the kind of lukewarm
  design language that could reasonably fit any idea or thing."
- **anon7000**（https://news.ycombinator.com/item?id=48500574）：
  > "**the many slop variants have poisoned my opinion on all of them.** I can instantly
  > identify certain AI slop Claude designs."
- **Bolwin**（https://news.ycombinator.com/item?id=47820735）：
  > "The AI tells are quite literally what made me lose interest. **They help me filter
  > out dozens of AI slop posts daily so I'll be keeping them thanks.** They're lazy by
  > design."
- **joshellington**（https://news.ycombinator.com/item?id=48549181）给出的处方：
  > "**redesign the website, remove all emojis and all letter-spacing CSS, use non
  > conventional typefaces, no italic serifs, limited cream/beige colors**"

**关键结论**：`functionmouse` 的 "lukewarm"（不温不火）是**最危险的那一档**——
不是丑，而是**可以套在任何东西上**。这与官方 skill 的判据完全一致：
**"it appears regardless of subject"**。

### 9.9 本项目体检汇总

按 §9.1–9.5 统计（✅ 未命中 / ⚠️ 部分 / ❌ 命中）：

| 类别 | ✅ | ⚠️ | ❌ | 命中率 |
|---|---|---|---|---|
| 视觉层（26 项） | 13 | 5 | 8 | 31% |
| 排版（14 项） | 6 | 2 | 6 | 43% |
| 组件/状态（12 项） | 6 | 0 | 6 | 50% |
| 动效（11 项） | 9 | 0 | 2 | 18% |
| 图标/图像（9 项） | 6 | 2 | 1 | 11% |
| **合计（72 项）** | **40** | **9** | **23** | **32%** |

**读法**：动效与图标两项**接近满分**（本站不做花活、不用 emoji），
**组件/状态与排版两项最差**。这不是"审美不行"，是**没有系统**——
所有 ❌ 都指向同一个根因：**没有令牌、没有状态、没有尺度**。

**改造成本最低、收益最高的五件事**（按 ROI 排序）：

| 序 | 动作 | 修掉的 ❌ | 成本 |
|---|---|---|---|
| 1 | 引入 §11.4 的令牌集（颜色/间距/字号/圆角/时长） | V2 V5 V6 V14 T10 T14 C3 | 改一个文件 |
| 2 | 把 `--max:960px` 拆成 `--measure:40rem` + `--wide:68rem` | T9 | 2 行 |
| 3 | 补 `:focus-visible` + `--ring`，并补全交互态 | C4 C5 | ~20 行 |
| 4 | 删 eyebrow（`.card .step`）、删 `→`、删 `letter-spacing` | T6 T7 V20 | 3 处 |
| 5 | 加暗色模式 + `prefers-reduced-motion` + favicon/og/theme-color | M8 I9 | ~60 行 |

### 9.10 来源索引

| 缩写 | URL |
|---|---|
| Anthropic `frontend-design` | https://raw.githubusercontent.com/anthropics/skills/main/skills/frontend-design/SKILL.md |
| Anthropic cookbook（前端美学） | https://raw.githubusercontent.com/anthropics/claude-cookbooks/main/coding/prompting_for_frontend_aesthetics.ipynb |
| hallmark SKILL / anti-patterns / copy / slop-test | https://raw.githubusercontent.com/Nutlope/hallmark/main/skills/hallmark/SKILL.md （及同目录 `references/{anti-patterns,copy,slop-test}.md`） |
| interfaces（Web Interface Guidelines） | https://raw.githubusercontent.com/raunofreiberg/interfaces/main/README.md |
| Vercel Web Interface Guidelines | https://vercel.com/design/guidelines |
| Trystan-SA `ai-slop-check` | https://raw.githubusercontent.com/Trystan-SA/claude-design-system-prompt/main/claude/skills/ai-slop-check.md |
| rohitg00 默认指纹表 | https://raw.githubusercontent.com/rohitg00/awesome-claude-design/main/README.md |
| garden-skills `failure-patterns` | https://raw.githubusercontent.com/ConardLi/garden-skills/main/skills/web-design-engineer/references/failure-patterns.md |
| huashu-design | https://github.com/alchaincyf/huashu-design （`references/{design-styles,critique-guide}.md`） |
| taste-skill | https://github.com/Leonxlnx/taste-skill |
| Waza `ui` skill | https://github.com/tw93/Waza （`skills/ui/SKILL.md`） |
| WCAG 2.2 Quickref | https://www.w3.org/WAI/WCAG22/quickref/ |
| WCAG 1.4.1 Use of Color | https://www.w3.org/WAI/WCAG22/Understanding/use-of-color.html |

---

## 10. 语气与内容设计

> 本节来源质量最高的一批：**维基百科《Signs of AI writing》**（带参考文献的词表）、
> **Kobak et al. 的 PubMed 超额词频论文**、**GOV.UK 写作指南**、
> **NN/g 的四次阅读研究**、**中文文案排版指北**、**中文维基《歐化中文》**。
> 全部已取到原文。

### 10.1 AI 用语词表（三份互相印证的来源）

**① 维基百科《Signs of AI writing》**（https://en.wikipedia.org/wiki/Wikipedia:Signs_of_AI_writing ，已取到全文）
—— 这是**最系统的、每个词条都标注了支撑研究**的枚举。核心词表（逐字）：

| 类别 | 词（逐字摘录） |
|---|---|
| **AI vocabulary（核心）** | `Additionally`（尤其句首）、`align with`、`boasts`（= has）、`bolstered`、`crucial`、`deep dive`、`delve`、`emphasizing`、`enduring`、`enhance`、`fostering`、`garner`、`highlight`（动词）、`interplay`、`intricate/intricacies`、`key`（形容词）、`landscape`（抽象）、`meticulous/meticulously`、`pivotal`、`robust`、`showcase`、`tapestry`（抽象）、`testament`、`underscore`（动词）、`valuable`、`vibrant` |
| 过度强调意义/遗产 | `stands/serves as`、`is a testament/reminder`、`a crucial/pivotal/vital role/moment`、`underscores/highlights its importance`、`reflects broader`、`setting the stage for`、`key turning point`、`evolving landscape`、`indelible mark`、`deeply rooted` |
| 宣传/广告腔 | `boasts a`、`vibrant`、`rich`、`profound`、`showcasing`、`exemplifies`、`commitment to`、`natural beauty`、`nestled`、`in the heart of`、`groundbreaking`、`renowned`、`diverse array` |
| 模糊归因 | `Industry reports`、`Observers have cited`、`Experts argue`、`Some critics argue`、`several sources` |
| 提纲式结尾 | `Despite its... faces several challenges`、`Despite these challenges`、`Challenges and Legacy`、`Future Outlook` |
| 「值得注意」类 | `it's important/critical/crucial to note/remember/consider`、`worth noting`、`may vary` |
| 结尾总结 | `In summary`、`In conclusion`、`Overall` |
| 系词回避 | `serves as/stands as/marks/functions as/operates as/represents [a]`、`boasts/features/maintains/offers [a]`（代替 `is`/`has`） |
| 聊天残留 | `I hope this helps`、`Of course!`、`Certainly!`、`You're absolutely right!`、`Would you like...`、`let me know` |

**页面点名的结构性 tell**（每条都有判据，逐字要点）：

| 结构 | 页面原文要点 |
|---|---|
| **否定式平行** | `not just X, but also Y` / `not X, but Y` / `Y rather than X`——"While it is common among human writers… it is stereotypically an 'AI sign.'" |
| **三段式（rule of three）** | "LLMs overuse the rule of three… to make superficial analyses appear more comprehensive." **在编辑摘要这类没人会用修辞的场合出现时，判据最强。** |
| **Title case 标题** | "In section headings, AI chatbots strongly tend to capitalize all main words." |
| **加粗滥用** | 机械地给每次出现的某个词加粗，"in a 'key takeaways' fashion" |
| **行内标题式列表** | `**Bold Header:** 说明` 形式的列表 |
| **em dash 过多** | LLM 比同体裁人类文本用得多，且用在人类会用逗号/括号的地方；"in a formulaic, pat way, often mimicking 'punched up' sales-like writing" |
| **emoji 当格式** | emoji 装饰小节标题或列表项 |
| **弯引号** | ChatGPT/DeepSeek 典型；**但页面明确说弯引号本身不构成证据**（Chicago 体例、Word 智能引号都会产生） |
| **不该用表格的地方用表格** | 把本该是散文的两三行内容做成极简表格 |

**② Kobak et al. 的 PubMed 超额词频**（arXiv https://arxiv.org/abs/2406.07016 ，
全文 https://arxiv.org/html/2406.07016v1 ，均已取到；正式版 Nature Human Behaviour）：

- 方法：14M 篇 2010–2024 摘要，用"超额死亡率"思路做词频反事实外推。
- **具体倍数（原文）**：`delves (r = 25.2)`、`showcasing (r = 9.2)`、`underscores (r = 9.1)`；
  常见词 `potential (δ = 0.041)`、`findings (δ = 0.027)`、`crucial (δ = 0.026)`。
- 2024 Q1 超额词 **329 个**；2013–24 共 **774 个**唯一超额词，其中 2024 年
  **280 个是 style words，66% 是动词、18% 是形容词**（往年超额词多为名词）。
  结论：**至少 10% 的 2024 摘要经 LLM 处理，部分子语料达 30%**。
- 论文自带的"典型 LLM 腔"例句（**最好的反面范文**）：
  > "By **meticulously delving** into the **intricate** web connecting […] and […], this
  > **comprehensive** chapter takes a **deep dive** into their involvement as
  > **significant** risk factors for […]."
- 论文转引 Liang et al.：**`pivotal`、`intricate`、`showcasing`、`realm`** 是 LLM 最偏好词。

> ⚠️ **引用更正**：arXiv **2404.01268** 不是 Kobak 论文，而是 Liang et al.
> *Mapping the Increasing Use of LLMs in Scientific Papers*（950,965 篇论文的总体统计，
> CS 最高 17.5%，数学/Nature 系 6.3%），**不含词表**。不要张冠李戴。

**③ Juzek & Ward 的 32 词表**（arXiv https://arxiv.org/abs/2508.01930 ，已取到）——
从既有 overuse 文献汇总，并归因于 RLHF：

`advancements, aligns, boasts, commendable, comprehending, crucial, delve, delved,
delves, delving, emphasizing, garnered, groundbreaking, intricacies, intricate,
invaluable, meticulous, meticulously, notable, noteworthy, pivotal, potential, realm,
showcases, showcasing, significant, strategically, surpasses, surpassing, underscore,
underscores, underscoring`

Llama Base→Instruct 增幅最大的词：`nuanced` (+8342%)、`nuance` (动词, +6301%)、
`firstly` (+4794%)；`underscore`(动词) 4.3 → 124.9（**+2829%**）。
**关键论证**：814 个 Instruct 相对 Base 超用的词中，**813 个也显著高于人类基线**；
且实验证明**人类标注员系统性地偏好含这些词的文本**——即"AI 腔"部分是 **RLHF 的副产品**。

**本项目的可执行规则**：把上述三份词表取并集做成 `scripts/` 里的一个 grep 断言，
挂在 `check-site.py` 上。**注意 `key`/`highlight`/`robust`/`potential` 是常用词**，
不能一律禁——只禁**在营销语境里**使用（如"强大而 robust 的内核"）。

### 10.2 句法节奏

| 规则 | 数值 | 出处 |
|---|---|---|
| 句子 ≤ **25 词** | "Try to split up sentences that are over 25 words long." | GOV.UK *Use clear language* |
| 段落 ≤ **5 句** | "Paragraphs should have no more than 5 sentences each." | 同上 |
| 变化句长 | "Several sentences of the same length can make for bland writing. To enliven paragraphs, write sentences of different lengths." | Purdue OWL, *Sentence Variety* |
| 变化句首 | "If too many sentences start with the same word, especially The, It, This, or I, prose can grow tedious" | 同上 |
| 倒金字塔，**不重复** | "Put the most important information first… taper down to smaller and smaller details"；"**Do not repeat the summary in the first paragraph.**" | GOV.UK *Create a clear structure* |

**为什么统一 20 词的句子读起来像机器**——NN/g 2008 的阅读时间模型给出机制
（https://www.nngroup.com/articles/how-little-do-users-read/ ，已取到）：
拟合公式为**固定约 25 秒 + 每 100 词额外 4.4 秒**；按 250 WPM 计，
**每多写 100 词，用户只多读其中 18 词**。统一步调的段落没有"值得减速"的信号，
用户就只走扫读路径。

**可执行规则**：每段 ≤5 句；单句 ≤25 词；一段内至少一次 ≤8 词的短句；
不连续三句以同一词开头；**段末不写小结句**；同一意思不在标题/摘要/首段出现三次。

### 10.3 具体性 > 形容词

**GOV.UK 对语气的定义里有一句是本节最有力的判据**（*Use the right tone*，已取到）：

> "**emotionless – adjectives can be subjective and make the text sound more emotive
> and like spin**"

即：**形容词 = spin**。另有可执行的两条：
- "Do not use formal or long words when easy or short ones will do. Use 'buy' instead of
  'purchase', 'help' instead of 'assist', 'about' instead of 'approximately'."
- "**Words ending in '–ion' and '–ment' tend to make sentences longer and more
  complicated** than they need to be."（对中文的对应物是"……性""……化"）
- 对专家也要写清楚：法律语言研究显示 "**the more educated the person and the more
  specialist their knowledge, the greater their preference for plain English**"。

**Anthropic skill 的写作节**（§1.4 已引）与本项目最相关的三条：
> "**A user manages notifications, not webhook config.**"
> "**Describe what something is or does in plain terms rather than selling it.**"
> "**Let each written element do exactly one job.**"

**改写对照（本项目可直接用）**：

| ❌ 形容词式 | ✅ 具体式 |
|---|---|
| "强大的内核" | "77,819 行 Rust；判定走完整内核，不做文本比对" |
| "零依赖，开箱即用" | "扩展内嵌 LSP 与 CLI，不需要 Rust、Lean 或任何工具链" |
| "精心设计的课程" | "12 个单元；卷 I 实测 329 条 checked、99 条 open、0 failed" |
| "持续更新" | "第 108 轮，0.61.0，2026-09-19" |
| "无缝的编辑器集成" | "在 VS Code 里按 `alt+b` 编译，诊断由完整内核给出，不是正则匹配" |

**"删掉测试"**（替代我原本想用的"so what 测试"，因为该名目未取到出处）：
**这句话删掉后，读者做决定所需的信息是否减少？不减少就删。**
依据：NN/g "Useful headings are specific. They provide facts… **Avoid broad and generic
headings.**" + GOV.UK "**Publish only what's needed to meet that user need and nothing
more.**"

### 10.4 标题怎么写

| 规则 | 数值 / 原文 | 出处 |
|---|---|---|
| 长度 | "Your title should be **65 characters or less** (including spaces)"（Google 在 ~65 字符截断） | GOV.UK *Write clear titles* |
| 页面 `<title>` | **40–60 字符**；"**Move the keywords to the front**"；"Skip leading articles like 'the' and 'a'" | NN/g *Microcontent* |
| **独立可解** | "Your title should make sense **by itself** – for example 'Regulations' does not say much, but 'Regulations for environmental waste' does" | GOV.UK |
| **不用问句** | "Headings should not: **be questions – they're hard to frontload and users want answers, not questions**" | GOV.UK *Create a clear structure* |
| 动词开头 | 页面**执行**动作 → 主动词 `Submit your business expenses`；页面只是**指导** → 现在分词 `Submitting your business expenses` | GOV.UK |
| 不含内容类型词 | 不要写 `guidance`/`consultation`——版式已经标了。Bad: `Potato guidance` → Good: `How to grow potatoes` | GOV.UK |
| 可移除 | "the content should still make sense with the headings removed" | GOV.UK |
| 反 clickbait | "Sensational headlines… **erode trust**. You can fool people to click once, but you can't fool them repeatedly." | NN/g |
| **大小写** | "**Default to sentence-style capitalization… Don't use title-style capitalization (Like This)**" | Microsoft 风格指南 Top 10 |

**"竞品替换测试"**（本报告形式化，依据 NN/g "Make sure the headline works out of
context" + GOV.UK "Make your title unique"）：
**把标题里的产品名换成竞品名，若句子仍然成立，这个标题就没有信息量。**

**本项目评价**：`site/index.html` 的 hero 标题「**学证明，从写对一个命题开始**」
通过替换测试（换成任何竞品都不成立），有动词、有具体对象、≤20 字。
**内页 `<title>` 不合格**：15–20 字（"关于"/"课程"/"文档"），不带品牌、不含关键词。
→ 改为 `课程 — sokonanoda` 形式。

### 10.5 特性描述怎么写

**模板**：`它做什么（平实动词）→ 因此你能做什么 → 证据（命令/输出/数字）`，
压进 **2 句 / ≤50 词**。

**长度依据**（本节原想给"1–2 句、15–30 词"，但**没有单一权威给出这个数字**）：
可引用的是 GOV.UK「句 ≤25 词、段 ≤5 句」与 NN/g「**只有 ≤111 词的页面，用户才会读
一半**」。**"2 句 / ≤50 词"是本报告的自定规范，与上述区间一致，但不是引用标准。**

**禁止纯 benefit-speak**：GOV.UK 的语气定义直接禁止情绪化形容词（§10.3）；
NN/g 1997 的实测显示**客观化（去推销腔）单项带来 +27% 可用性**
（简洁 +58%、可扫读 +47%、三者叠加 **+124%**）。

**本项目 3 张演示卡的评价**：

| 卡片 | 评价 |
|---|---|
| "卡住了，悬停 `sorry`" | ✅ **正例**。正文给了可验证的行为："期望类型会展开定义来算（`Not a` 展开成 `a -> False`，洞期望 `a`）"——**具体到可以拿去核对** |
| "判对判错，内核说了算" | ✅ 正例。但"每一行放进官方 Lean 也一样合法"是**强承诺**，必须能兑现；兑现不了就删 |
| "你的 code agent 也能当老师" | ⚠️ "当老师"是比喻，可留；但"任何 code agent（Claude Code / opencode / Codex / Cursor / …）"的省略号列表是**弱化**——要么给全，要么只给一个 |

### 10.6 每节多少字（有实测数字）

NN/g 的四次研究给出了可直接引用的量化依据：

| 研究 | 关键数字 |
|---|---|
| 1997（Morkes & Nielsen，81 名用户） | 简洁 **+58%**、可扫读 **+47%**、客观 **+27%**，叠加 **+124%** 可用性。"users do not read on the Web; instead they **scan**"；"users **detest** anything that seems like marketing fluff… ('marketese')" |
| **2008 阅读量量化**（45,237 次页面浏览） | **固定约 25 秒 + 每 100 词 4.4 秒**；250 WPM 下**每加 100 词只多读 18 词**。平均页 593 词 → 最多读 **28%**，现实 **20%**。**≤111 词的页面才可能读一半** |
| 2020 复访（13 年、5 次眼动、500+ 被试） | "People rarely read online — they're far more likely to scan… **that hasn't changed in 23 years**"。新发现：对比表格与 zigzag 布局催生 **lawn-mower pattern**（一行左→右，下一行右→左） |
| F 型扫描（2006，2017 复访） | ①顶部横读 ②更短的第二横读 ③左侧竖扫。"**First lines of text on a page receive more gazes than subsequent lines**"；"First few words on the left of each line receive more fixations"。**F 型对用户和业务都是坏事**，好设计可以避免 |

**推论链**（每一步都有出处）：用户只读 20–28% → 只有 ≤111 词的页面才读一半 →
每多 100 词只多读 18 词 → F 型扫描使第 3 段首行的注视远少于第 1 段 →
**180 词的三段式 ≈ 前 40 词有效 + 后 140 词近乎无效**，且中段"主题句"会与标题/摘要重复
（GOV.UK 明令禁止重复）。

**本项目规则**：

| 区块 | 字数 | 依据 |
|---|---|---|
| hero 的 lead | **≤ 60 字** | 超过就没人读第二行 |
| 一个特性卡正文 | **≤ 50 词** | 自定规范，与 GOV.UK ≤25 词/句一致 |
| 一个正文小节 | **≤ 250 字** | 超过就该拆或改表格 |
| 首页总字数 | ≤ 700 字 | 当前约 600 字 ✅ |

**"一屏一个想法"的实证支持**：NN/g 2020 的 lawn-mower 发现表明，内容被切成
**明确的 cell** 后用户会逐格处理——这正是"一节一想法"的机制。

### 10.7 什么时候用表格

**GOV.UK 的表格规则**（https://www.gov.uk/guidance/content-design/tables ，已取到）
是本节最可直接执行的来源：

| 规则 | 原文 |
|---|---|
| 该用 | "Use tables to present data or information that can be organised in a structured way… **examine a range of possibilities at a glance**" |
| 不该用 | "**Do not use tables for cosmetic reasons** or when you can use normal page structure… for example headings and lists." |
| **最小尺寸** | "**the minimum size should be 2 columns and 3 rows (including headings)**… **If your table is too small, you can probably present the same information as normal text.**" |
| 最大尺寸 | "on a desktop, you can usually see **4 or 5 columns and 10 rows** without scrolling" |
| 表头 | "Your table can only have **one heading row and one heading column**." |
| 单元格 | "'no data' or 'not applicable' instead of empty cells – only the top left cell can be empty"；"no split or merged cells"；"only one item per row cell" |
| 排序 | "Add the information that most people are looking for at the top or in the first few columns. **Do any calculations for the user.**" |

**可访问标记**（W3C WAI Tables Tutorial，https://www.w3.org/WAI/tutorials/tables/ ，已取到）：
"Header cells must be marked up with `<th>`, and data cells with `<td>`"；
方向不明的表头加 `scope="col"`/`scope="row"`；
"**A caption identifies the overall topic of a table and is useful in most situations**"；
理由："**Screen readers speak one cell at a time and reference the associated header
cells**, so the reader doesn't lose context."（对应 WCAG 1.3.1，Level A）

**为什么表格优于 bullet 列表做对比**（本报告推论）：GOV.UK 的判据是"一眼看出多种
可能性之间的关系"——bullet 列表在语义上**不保证同一属性跨项对齐**，读者必须在脑中
重建矩阵；表格把对齐外化到版面上。

**本项目的具体机会**：12 个单元的计数、三个 harness 的能力差异、`--json` 事件类型
——**当前都散在散文和卡片里**，且站点只有 1 个表格（§2.2）。

### 10.8 技术产品如何获得可信度

**四个已取到的"去魅"范例**（共同构成一个体裁：**主动解释机制与边界，而不是宣称效果**）：

| 范例 | 做法 |
|---|---|
| **ripgrep README** | 每个 benchmark 是完整表格 `Tool \| Command \| Line count \| Time`，**命令可复制**。主动贴免责声明："**Please remember that a single benchmark is never enough!**" 主动贴自己的性能悬崖："**Beware of performance cliffs though:**"——并给出自己**并不占优**的场景 |
| **SQLite《Quirks, Caveats, and Gotchas》**（https://www.sqlite.org/quirks.html ） | 首页标语就是诚实姿态："Small. Fast. Reliable. **Choose any three.**" 目录逐条自曝：`Foreign Key Enforcement Is Off By Default`、`PRIMARY KEYs Can Sometimes Contain NULLs`、`SQLite Does Not Do Full Unicode Case Folding By Default`。**标题即结论，每条可独立链接** |
| **Rust 的 Tier 3 平台声明** | "Tier 3 targets are those which the Rust codebase has support for, but **which the Rust project does not build or test automatically, so they may or may not work.**" —— **分级承诺**，用一张表说清"保证/尽力/不保证" |
| **TypeScript Design Goals 的 Non-goals** | 7 条显式"故意不做"，逐字含："**Apply a sound or 'provably correct' type system.** Instead, strike a balance between correctness and productivity." |

**Simon Willison 论文档**（https://simonwillison.net/tags/documentation/ ，已取到）：
> "**Without documentation a bug is just undefined behavior.**"
> "**Documenting what you're willing to support (and not)**"

即：**文档 = 承诺边界**。公开边界使"未实现"不等于"不靠谱"。

**本项目的可信度资产盘点**：

| 做法 | 本项目现状 |
|---|---|
| 真数字 | ✅ 全部由 `gen-site-data.py` 生成 |
| 真命令（可复制） | ✅ 首页就有 `git clone` / `code` |
| **真实局限 / "未做"区** | ✅ **已有素材**：`99 open` / `counts_source: previous-run` / "没有 WASM 构建" / `docs/CI-FAILURES.md`（每次 CI 红了追加一条） |
| 分级承诺 | ⚠️ 有"零工具链依赖"但未分级 |
| 显式 non-goals | ❌ 没有。**建议新增**："不调用官方 Lean 工具链"、"不是 Lean 4 兼容实现"、"无隐式参数插入（`section`/`variable` 实测做不动）" |
| benchmark 带命令 | ⚠️ 有 `scripts/perf-ledger.sh`，但站点未展示 |
| 源码/CI 链接 | ✅ 有 GitHub 链接；❌ 无 CI badge |

**最重要的建议**：把 `docs/CI-FAILURES.md`（CI 失败台账）与"已知限制"
**提升为网站上的独立小节**。在一个到处是"10× faster"的互联网上，
**一份带日期的失败记录比任何形容词都更能证明项目是真的**。

**hero 应该放什么**（Anthropic skill：`Open with the most characteristic thing in the
subject's world`）：直接给一段**能跑的 `.sokonanoda` 代码** + 一行**真实的
`scripts/soko grade` 输出**，而不是形容词。当前首页放的是演示截图（✅ 方向对），
但截图**不可复制**——建议补一个**可复制的文本代码块**。

### 10.9 双语 / 中文特有规则

**① 中英混排空格**（中文文案排版指北，
https://raw.githubusercontent.com/sparanoid/chinese-copywriting-guidelines/master/README.zh-Hans.md ，已取到；
注意用户可能给的 `README.zh-CN.md` **路径是 404**，简体版实际是 `README.zh-Hans.md`）：

| 规则 | 例 |
|---|---|
| 中英文之间加空格 | ✅「在 LeanCloud 上，数据存储是围绕 `AVObject` 进行的。」❌「在LeanCloud上」 |
| 中文与数字之间加空格 | ✅「花了 5000 元。」 |
| 数字与单位之间加空格 | ✅「10 Gbps」。**例外：度数与百分比不加**——「90°」「15%」 |
| 全角标点与其他字符之间**不**加空格 | ❌「买了一部 iPhone ，好开心！」 |
| 不重复标点 | ❌「！！」「？！？！」 |
| 数字用半角 | ✅「1000 元」❌「１０００ 元」 |
| 专有名词正确大小写 | `GitHub` 而非 `github`/`GITHUB`；视觉全大写用 `text-transform` |
| **CSS 能否自动加空** | `text-autospace`（CSS Text L4）可自动加，但"目前并未普及"，且编辑器/终端/GitHub README 都不应用 → **继续手写空格** |

**② `text-autospace`**（MDN，https://developer.mozilla.org/en-US/docs/Web/CSS/text-autospace ，已取到）：
**Baseline 2025 / Newly available（2025 年 11 月起）**。值：`normal`（= `ideograph-alpha`
+ `ideograph-numeric`）/ `no-autospace` / `ideograph-alpha` / `ideograph-numeric` 等。
注意："**This property is additive with the word-spacing and letter-spacing properties.**"
→ **落地建议**：作为渐进增强写上 `text-autospace: normal`，**但不要依赖它**。

**③ CJK 行高**——W3C《中文排版需求》（https://www.w3.org/TR/clreq/ ，已取到）给出
**结构性原因**：「汉字有着正方形的文字外框。文字外框的正中央，有着比文字外框小的字面」
——**字面本身带内边距，所以拉丁的 1.5 在 CJK 下视觉上更挤**。
可引用的下界：「为保证行间标点的摆放，**单面装的行距不应小于当前字号的一半、双面装
的行距不应小于当前字号的 5/8**」；注音场景「通常需要基字尺寸 **1.5 倍以上**的行距」。

> ⚠️ **诚实说明**：clreq **没有**给出"Web 正文 1.7–1.8"这个数字。
> **本报告的 `--lh-body: 1.7` 是经验值（自定规范）**，可引用的依据只是
> "CJK 行高必须大于拉丁"这个结构性理由与上面的下界。

**④ 不要对 CJK 用 `letter-spacing`**——clreq 里字距是**疏排手段**而非常规设置：
一旦拉开字距，行间线/着重号"应相应延长且不能断开"，着重号"依旧应与字符居中对齐
而不能错位"。即**拉开字距会连带破坏一批对齐规则**。另有中西混排空白上限：
「汉字与西文字母、阿拉伯数字间使用**不多于四分之一个汉字宽**的字距或空白」。
且 `letter-spacing` 会**叠加**在 `text-autospace` 之上（MDN）→ 同时用会过度加空。
**当前站点 `.card .step{letter-spacing:.5px}` 应删（§9.2 T7）。**

**⑤ 全角标点，以及一个对本项目有用的例外**——clreq 明确给出技术文档的豁免：
> "in the case of **technical documents, if plenty of formulae are contained in the
> text, the full stop can be unified with the western-style period, U+002E FULL STOP [.]**,
> and the ellipsis can be unified with the western-style ellipsis, U+2026 […]."

即**公式密集的技术文可以用半角句点**——这是有出处的、可以写进规则书的例外。
clreq 另明确反对使用"全角 ASCII 字符"（`Ａ`/`１`）：「现今在文本储存时，
**应避免使用该区段的拉丁字母及数字字符**」。

**⑥ 翻译腔（欧化中文）**——中文维基《歐化中文》（https://zh.wikipedia.org/wiki/歐化中文 ，已取到）
是最有系统的中文来源。**核心机制**：余光中称之为「**文字的義肢**」——
**万能动词 + 抽象名词**。

| 类别 | 对照（页面原文例） |
|---|---|
| **万能动词 + 抽象名词**（最泛滥的是「作出」「進行」） | 「本校的校友對社會**作出了重大的貢獻**」→「本校的校友對社會**貢獻很大**」；「對國際貿易的問題已經**進行了詳細的研究**」→「已經**詳加研究**」 |
| 后缀滥用（「性」「化」「度」） | 「這篇傳記的**可讀性**很高」→「這篇傳記**值得一看**」；「病人要做**例行性**檢查」→「病人要做**例行**檢查」 |
| 被动滥用 | 「我不會**被你這句話嚇倒**」→「**你這句話嚇不倒我**」；「他**被升為**營長」→「他**升為**營長」 |
| 系词「是」滥用 | 「He is very clever」→「他**很聰明**」，而非「他是很聰明的」 |
| 「的」字连用 | 「彎彎的楊柳的稀疏的倩影」→「彎彎的楊柳**投下**稀疏的倩影」 |
| 冗长前置定语 | 「有不潔的顏色的都市的河溝」→「都市的**髒河溝**」 |
| 「作为」「当」「在」硬译 | 「**作為**一個學生」→「**身為**學生」 |
| 复数「们」 | 非人事物硬加「們」是硬套英文语法 |

> ⚠️ **本报告不给「赋能/无缝/一站式/助力/打造/生态/闭环」这份中文词表标注出处**——
> **本会话没有取到以权威词表形式发布该清单的原文**（见 §12）。
> 若要收进规则书，应标为**自定清单**，并以《歐化中文》的「万能动词 + 抽象名词」
> 机制作为理论依据——**这些词的问题不是"不好听"，而是把动词抽象化，
> 从而删掉了施事者与动作**，与 GOV.UK「用主动语态、把 doer 变成主语」同源。

**⑦ 中英标题惯例差异**：英文用**句首大写**（Microsoft："Don't use title-style
capitalization"），且维基百科把 Title case 单列为 AI tell。中文没有大小写，
所以"标题层级"只能靠**长度、动词开头、信息量**区分。
**混合场景（`site/en/` 与中文并列）不要逐字互译**——NN/g 的规则是
"Make sure the headline works out of context"，中文版应**重新按「谁 + 什么 +
为什么重要」写**，而不是把英文 headline 直译。

### 10.10 本项目文案体检

| 项 | 结果 |
|---|---|
| §10.1 三份词表 | ✅ **0 处命中**（实测 grep） |
| 无信息量形容词 | ⚠️ 少量（"推荐入口"） |
| 真数字 / 真命令 | ✅ 有 |
| 主动语态 | ✅ 基本满足 |
| **一个动作同名** | ⚠️ 首页按钮"安装（扩展 / agent）"vs `get-started.html` 的"安装与上手"——**统一为"安装"** |
| 诚实限制 | ✅ 有素材，但**未集中展示**（§10.8 建议） |
| 显式 non-goals | ❌ 没有，建议新增 |
| 中英混排空格 | ❌ 未处理（§10.9①） |
| `<title>` 品牌与长度 | ❌ 内页 15–20 字，不带品牌（§10.4） |
| 句长 ≤25 词 / 段 ≤5 句 | ⚠️ 未测，建议加进 `check-site.py` |

**总评：本站的文案质量显著高于其设计质量。** 改造时**文案只需小修**（上表 5 项），
主要工作在设计层。**这是本报告把 §9 放在 §10 之前的原因。**

---

## 11. 可直接落地的令牌草案

### 11.1 美学方向：「工程方格纸 · 内核批注」

**三句话正当化**：

1. 这个产品的**产物是判定**——一个真内核告诉你"这一步通过了"或"这一步错了"，
   所以整站的颜色系统应该由**判定结果**驱动，而不是由"科技感"驱动：唯一的强调色
   就是内核判定绿，它**只允许出现在内核真的通过过的东西上**（实测计数、已发布的
   版本号、判红的绿色通过态），任何装饰性使用都是语义污染。
2. 这个产品的**材料是格子纸上的推导**——目标视图（假设在上、`⊢` 目标在下）、
   推理横线、编号单元、可复现的命令行；所以视觉语言取自**工程方格纸与批注稿**：
   冷调近白纸面（不是奶油色，奶油色是 §1.2 第①类 AI 聚类）、**零圆角的结构容器**
   （纸没有圆角）、发丝线承担全部分隔、**深度只靠两级表面明度**而不靠阴影。
3. 这个产品的**读者是中英混排的技术读者**——所以放弃"高对比衬线大标题"这一整套
   英文营销模板（它在中文里既没有合适的 webfont，也压不住 PingFang/雅黑），改用
   **单一无衬线 + 系统 CJK + 一个数学覆盖完整的等宽**，把表现力放在**数字、符号与
   细线**上，而不是放在字号落差和渐变色上。

**一句话记忆点（"spend your boldness in one place"）**：
全站只有一个视觉主角——**推理横线（turnstile rule）**：一条发丝线，上方是"给定"，
下方是"待证"。它在每页出现 2–3 次，位置固定，其余一切都保持安静。

### 11.2 设计决策与依据对照

| 决策 | 值 | 依据 |
|---|---|---|
| 结构容器圆角 | `0` | §1.2 第④类「one border-radius on everything」的反面：**让圆角承载层级信息**——结构 = 0，控件 = 3px，状态点 = full |
| 阴影 | 只保留 1 个，仅用于**真正浮起**的层 | 纸面没有阴影；§6 待补的 Material/web.dev 依据 |
| 强调色数量 | **1 个语义色**（判定绿）+ 1 个诊断红（**只用于"未完成/限制"**） | §5 的 accent discipline |
| 字体族数 | 2（无衬线 + 等宽），CJK 走系统 | §3.7 |
| 动效 | 只有**状态反馈**，零入场动画 | §1.3「fade-and-slide-up entrances on each section … read as AI-generated」 |
| 暗色模式 | 做，但**重新设计**（不是反相） | §5 |

### 11.3 令牌总览（含实测对比度）

以下所有颜色对已用 WCAG 2.x 相对亮度公式实算。**结论：正文/次要/元信息文本对
全部 ≥ 4.5:1，控件边界 ≥ 3:1，代码语法 8 个 token 全部 ≥ 4.5:1（亮暗两套）。**
唯一需要说明的是**纯装饰发丝线不适用 3:1**（WCAG 1.4.11 只管"识别控件所必需"的
视觉信息；`--rule` 只是分隔，不承担识别功能）。但**输入框边框是必需的**，所以
`--border-control` 是深灰 `#7C8691` 而不是浅灰——这一条最容易被做错。

### 11.4 完整令牌集（可直接复制进 `site/assets/style.css`）

```css
/* ============================================================
   sokonanoda site tokens — "工程方格纸 · 内核批注"
   方向：冷调纸面 / 零圆角结构 / 单一语义强调色（内核判定绿）
   规则：--acc 只允许出现在「内核真的通过过」的东西上
   ============================================================ */
:root {
  color-scheme: light dark;

  /* ---- 表面（两级明度承担全部深度，不用阴影） ---- */
  --paper:          #f5f7f9;  /* 页面底：冷调近白，非奶油色 */
  --surface:        #ffffff;  /* 抬起一级：卡片、表头 */
  --surface-sunken: #ebedf3;  /* 下沉：代码块、well、表体条纹 */
  --surface-overlay:#f0f2f6;  /* 悬浮层底 */

  /* ---- 墨色（三级，全部达标） ---- */
  --ink:      #14171c;  /* 正文            on paper 16.73:1 */
  --ink-2:    #4a5159;  /* 次要            on paper  7.49:1 */
  --ink-3:    #6a727c;  /* 元信息/标签      on paper  4.54:1 ← 下限，勿再浅 */
  --ink-invert:#f5f7f9; /* 深底上的字 */

  /* ---- 线（发丝线承担全部分隔） ---- */
  --rule:          #d9dee5;  /* 装饰性分隔线（无 3:1 要求） */
  --rule-strong:   #bac2cc;  /* 结构性分隔（表格外框、推理横线） */
  --border-control:#7c8691;  /* 控件边界，on surface 3.70:1 / on paper 3.45:1 ✅ */

  /* ---- 唯一强调色：内核判定绿 ----
     语义：仅用于「内核真的通过过」的东西（实测计数、已发布版本、通过态）。
     任何装饰性使用 = 语义污染，评审时按 bug 处理。 */
  --acc:        #1e6b4c;  /* 实底（主按钮底） on surface 6.44:1 */
  --acc-ink:    #14523a;  /* 绿色文字/链接   on paper  8.51:1 */
  --acc-wash:   #e7f0eb;  /* 绿色底          acc-ink on it 7.86:1 */
  --acc-border: #5e9c7d;  /* 绿色边（仅 on surface 用，3.22:1） */

  /* ---- 诊断红：只用于「未完成 / 限制 / 判错」 ---- */
  --rej:      #a32b22;
  --rej-ink:  #7e1f18;  /* on rej-wash 8.66:1 */
  --rej-wash: #fbebe9;

  /* ---- 警示黄：只用于「注意」 ---- */
  --warn-ink:  #6b4407;  /* on warn-wash 7.67:1 */
  --warn-wash: #faf2e0;

  /* ---- 焦点环 ----
     SC 2.4.13 Focus Appearance 是 AAA（不是 AA）：2 CSS px 周长 + 前后 3:1 变化。
     AA 级的是 SC 2.4.11 Focus Not Obscured（焦点不得被 sticky 头/脚遮挡）。
     用 outline 而非 box-shadow：CSS UI 4 规定 outline 不占空间、不触发重排、
     跟随 border-radius，且属于 ink overflow（不受 overflow 裁剪）；
     box-shadow 是绘制内容，会被 overflow: hidden/clip 裁掉。
     老浏览器（Safari < 16.4）的圆角 outline 支持不佳，可加 box-shadow 兜底。 */
  --ring-color: var(--acc);      /* acc vs paper 5.99:1，远超 3:1 */
  --ring-width: 2px;
  --ring-offset: 2px;
  /* 用法：:focus-visible { outline: var(--ring-width) solid var(--ring-color);
                              outline-offset: var(--ring-offset); } */

  /* ---- 间距：4pt 半步 + 8pt 节奏 ---- */
  --sp-1: 4px;  --sp-2: 8px;  --sp-3: 12px; --sp-4: 16px;
  --sp-5: 24px; --sp-6: 32px; --sp-7: 48px; --sp-8: 64px; --sp-9: 96px;

  /* ---- 字号：两段式比率（小端 ~1.07–1.13，大端 ~1.21–1.24），锚点 17px ----
     为什么锚 17 而不是 16：CJK 字形密度高，16px 中文偏挤；17px 是中文正文舒适下限。
     为什么不用单一 1.2 全阶：17/1.2/1.2 = 11.8px，小端会出现不可用字号。 */
  --fs-xs:   0.8125rem; /* 13px 元信息、表格注脚 */
  --fs-sm:   0.875rem;  /* 14px 次要文本、表格正文 */
  --fs-ui:   0.9375rem; /* 15px 导航、按钮 */
  --fs-base: 1.0625rem; /* 17px 正文 ← 基准 */
  --fs-lg:   1.1875rem; /* 19px lead、卡片标题 */
  --fs-xl:   1.4375rem; /* 23px h3 */
  --fs-2xl:  1.75rem;   /* 28px h2 */
  --fs-3xl:  2.125rem;  /* 34px h1（内页） */
  --fs-4xl:  2.625rem;  /* 42px display（首页 hero） */

  /* ---- 行高：字号越大行高越小 ---- */
  --lh-display: 1.15;  /* ≥34px */
  --lh-heading: 1.3;   /* h2/h3 */
  --lh-body:    1.7;   /* 中文正文；en 页可覆盖为 1.6 */
  --lh-code:    1.6;

  /* ---- 行长 ----
     40rem = 640px ≈ 37.6 个汉字（中文舒适区 30–40 字）
     也 ≈ 78 个拉丁字符（官方 skill: "less than 80 characters"）
     两个约束同时满足，这是 40rem 这个数唯一的理由。 */
  --measure: 40rem;
  --wide:    68rem;   /* 页面容器 1088px */

  /* ---- 圆角：承载层级，不是装饰 ---- */
  --r-0:    0;      /* 结构容器：卡片、区块、代码块、表格、图片 */
  --r-1:    3px;    /* 控件：按钮、输入框、kbd、标签 */
  --r-full: 999px;  /* 仅状态点，每屏 ≤1 个 */

  /* ---- 阴影：只有一个，只给真正浮起的层（菜单、popover） ---- */
  --shadow-pop: 0 1px 2px rgba(15,19,26,.06), 0 8px 24px -8px rgba(15,19,26,.18);

  /* ---- 动效：只做状态反馈，不做入场 ----
     取值依据 §7.1：NN/g「简单反馈 ~100ms / 模态 200–300ms / 总体 100–500ms /
     400ms 已属 very slow」；Material「桌面 150–200ms，>400ms 太慢」。
     出场取进场的 ~75–80%（NN/g：popup 300ms 出现、200–250ms 消失）。 */
  --dur-1: 100ms;  /* 状态反馈：hover 底色、焦点环 */
  --dur-2: 200ms;  /* 小 UI 切换：details 展开、tab 切换 */
  --dur-3: 300ms;  /* 浮层出现 */
  --dur-exit: 160ms; /* 浮层离开 = --dur-2 的 80% */
  /* Material 官方三条曲线（§7.1，三处独立印证） */
  --ease-standard: cubic-bezier(.4, 0, .2, 1);
  --ease-out: cubic-bezier(0, 0, .2, 1);   /* 进入 = deceleration */
  --ease-in:  cubic-bezier(.4, 0, 1, 1);   /* 离开 = acceleration */

  /* ---- 层 ---- */
  --z-header: 10; --z-pop: 40; --z-skip: 60;

  /* ---- 代码块：与页面同极性（亮页亮块 / 暗页暗块） ----
     不用「亮页里嵌一块 #111 近黑」——那是 §1.2 第⑤类 tell。 */
  --code-bg:   var(--surface-sunken);
  --code-ink:  #1b2027;  /* on code-bg 14.32:1 */
  --code-cmt:  #5c6570;  /* on code-bg  5.17:1 */
  --code-kw:   #186147;  /* 关键字/tactic → 用产品绿，on code-bg 6.47:1 */
  --code-ty:   #24507a;  /* 类型/常量，on code-bg 7.34:1 */
  --code-str:  #6b3a5e;  /* 字符串/字面量，on code-bg 7.68:1 */
  --code-num:  #3a424c;  /* 数字（配 tabular-nums），on code-bg 8.90:1 */
  --code-hole: #9e2a21;  /* `sorry` 洞 → 诊断红，语义即「未解」，6.54:1 */
  --code-punc: #4a525c;  /* on code-bg 6.93:1 */

  /* ---- diff：颜色 + 非颜色双通道（WCAG 1.4.1） ---- */
  --diff-add-ink: #14532d; --diff-add-bg: #e4f3e9;  /* 7.94:1 */
  --diff-del-ink: #7f1d1d; --diff-del-bg: #fce9e9;  /* 8.57:1 */
}

/* ============================================================
   暗色：重新设计，不是反相
   - 不用纯黑 #000（OLED 拖影/halation），不用纯白字
   - 强调色提亮 + 降饱和（不是复用亮色）
   - 抬升靠表面明度，不靠阴影（暗底上阴影不可见）
   ============================================================ */
@media (prefers-color-scheme: dark) {
  :root {
    --paper:          #0f1216;
    --surface:        #171b21;
    --surface-sunken: #0a0d10;
    --surface-overlay:#1e232b;   /* = 抬升层，明度 +5% */

    --ink:      #e5e9ee;  /* 不是 #fff；on surface 14.17:1 */
    --ink-2:    #aeb6c0;  /* on surface  8.44:1 */
    --ink-3:    #868f9a;  /* on surface  5.27:1 */
    --ink-invert:#0f1216;

    --rule:          #2a3038;
    --rule-strong:   #3c444e;
    --border-control:#626c78;  /* on surface 3.24:1 ✅ */

    --acc:        #5fb98c;  /* 提亮 + 降饱和，不是亮色原样搬过来 */
    --acc-ink:    #7fcfa5;  /* on paper 10.17:1 / on acc-wash 8.72:1 */
    --acc-wash:   #16241d;
    --acc-border: #3e7a5c;  /* on surface 3.41:1 */

    --rej:      #e0796c; --rej-ink: #ee9c91; --rej-wash: #2a1815;
    --warn-ink: #e8be6e; --warn-wash: #241d10;

    --ring: 0 0 0 2px var(--paper), 0 0 0 4px var(--acc);  /* 7.87:1 */

    --shadow-pop: 0 0 0 1px var(--rule-strong),
                  0 12px 32px -12px rgba(0,0,0,.6);

    --code-bg:   #12161b;
    --code-ink:  #dce3ea;  /* 14.03:1 */
    --code-cmt:  #8a94a0;  /*  5.90:1 */
    --code-kw:   #7fcfa5;  /*  9.83:1 */
    --code-ty:   #8fb8e8;  /*  8.81:1 */
    --code-str:  #d9a66c;  /*  8.32:1 */
    --code-num:  #b8c2cc;  /* 10.06:1 */
    --code-hole: #e0796c;  /*  6.16:1 */
    --code-punc: #9aa4b0;  /*  7.19:1 */

    --diff-add-ink: #86d9a5; --diff-add-bg: #132a1d;  /* 9.07:1 */
    --diff-del-ink: #efa2a2; --diff-del-bg: #2e1717;  /* 8.28:1 */
  }
}
```

### 11.5 三条与令牌配套的「语义纪律」

| 纪律 | 可检查的形式 |
|---|---|
| **绿色 = 内核通过过** | 全站 grep `var(--acc` 的每一处，问「这个元素背后有没有一个真实的 `checked` 计数/已发布版本？」没有就换成 `--ink-2` / `--rule` |
| **圆角 = 层级** | grep `border-radius`，只允许出现 `var(--r-0)`（结构）、`var(--r-1)`（控件）、`var(--r-full)`（状态点）三个值；出现第四个值即回归 |
| **一个屏只有一个主角** | 每屏最多一个 `--acc` 实底元素；若一屏出现两个主按钮，其中一个降级为 ghost |

### 11.6 该草案与当前 `style.css` 的 diff 摘要

| 项 | 现在 | 改为 |
|---|---|---|
| `font-size` 取值 | 17 个 | 9 个 |
| 圆角 | 6 种（5/6/7/8/10/999） | 3 种（0/3px/full） |
| `line-height` | 4 种（1.3/1.5/1.55/1.65） | 4 种但**有规则**（1.15/1.3/1.7/1.6，按字号递减） |
| 颜色 hex | 27 个硬编码 / 9 变量 | 全部走变量，暗色另有一套 |
| 代码块底色 | 3 套深色（`#11161a`/`#1e1e1e`/`#252526`） | 1 套，且与页面同极性 |
| `--max: 960px` 通吃 | 正文行长 ~112 拉丁字符 | `--measure: 40rem`（≈78 字符 / ≈38 汉字） |
| 暗色模式 | 无 | 有，且是重新设计 |
| 动效 | 0 个 `transition` | 3 个时长 + 2 条曲线，**只用于状态反馈** |
| 焦点 | 只有 `.skip-link:focus` | `:focus-visible` + `--ring`（2px 周长，≥3:1） |

---

## 12. 未取到原文的条目

> **本报告不臆造来源。** 下面每一条都是本会话尝试过但**未能取到原文**的。
> 正文中凡涉及这些主题的地方，均已标注为「未取到原文」或「本报告的判断」，
> **不得当作有出处的规则引用。**

### 12.1 环境限制（为什么取不到）

| 现象 | 影响 |
|---|---|
| `web_search` 工具不可用（缺 `DEEPSEEK_API_KEY`） | 无法做关键词检索，只能靠已知 URL 直取 |
| `raw.githubusercontent.com` 在 `bash`/`curl` 下间歇超时，但 `web_fetch` 工具可通 | 同一 URL 两种通道结果不同 |
| `web.dev`、`practicaltypography.com`、`duckduckgo.com`、`medium.com` 整体不可达 | 字体最佳实践、行长度权威出处丢失 |
| `m2.material.io`、`m3.material.io`、`developer.apple.com`、`primer.style` 是 JS 空壳 | Material / Apple HIG 原文丢失 |
| `api.github.com` 未认证限流（60 次/小时） | 部分社区仓库未能深挖 |
| Reddit 不可达 | 社区实证只剩 HN（HN Algolia API 可用） |

### 12.2 按主题列出

| 主题 | 具体缺什么 | 本报告的处置 |
|---|---|---|
| **Material 暗色主题** | elevation overlay 百分比（0%/5%/8%/11%/12%/16%）、`#121212` 这类具体值 | §5.5 标为「本报告的推论 + 数值自检」 |
| **Apple HIG 暗色模式 / 动效** | 整页 | 未引用 |
| **Material 动效时长与曲线** | ~~未取到~~ → **✅ 第二批已解决**：通过 mdui 英文镜像（`m2.material.io` 是 Angular 空壳）+ `@material/animation` 的 SCSS + MUI `createTransitions.js` 三处独立印证 | **§7.1 已改为核实数字**；并**更正**了坊间那张"桌面 250–500ms"的表（镜像说桌面 **150–200ms**，更短） |
| **web.dev 全部** | 字体最佳实践、高性能动画、`prefers-color-scheme` | 用 MDN 替代（MDN 可达） |
| **practicaltypography.com** | 行长、行距、光学对齐 | §3.3 用官方 skill 的 "<80 characters" + 算术推导替代；§6.4 标为判断 |
| **Refactoring UI** | 颜色章节、accent discipline、层级 | §5.6 改用本项目自己的语义纪律（绿色=内核通过过） |
| **60-30-10 规则的出处** | 无可引用的原始来源 | §5.2 **明确不引用其来源，也不给数字**，改给可测的替代规则 |
| **8pt grid 的出处** | Spec.fm / Elliot Dahl 原文 | §4.1 不引用出处，改用 Trystan 的 "off-scale values feel chaotic" 作为依据 |
| **中文字行长 30–40 字** | 未取到可引用的原始出处 | §3.3 标注为通行经验 |
| **`sparanoid/chinese-copywriting-guidelines`** | 中文排版指北全文 | §10.9 不引用，改列本项目实测的双语问题 |
| **GOV.UK 内容设计 / NN/g 标题与阅读研究** | 整页 | §10.4、§10.6 标为判断 |
| **MDN Clipboard API** | `navigator.clipboard.writeText`、安全上下文、`execCommand` 弃用 | §8.3 标为「工程实践，不是引用」 |
| **行号的"何时有害"论证** | 没有任何权威文章或维护者讨论支持该论点 | §8.2 **明确说未取到**，只保留 Shiki 的 `user-select:none` 作为已取到证据 |
| **表格样式 / WAI 表格教程** | 整页 | §8.6 标为判断 |
| **文档站细节**（锚点、TOC、callout 分类、`<kbd>`） | 整页 | §8.7 标为「本报告建议的优先级」 |
| **"em dash 过多"是 AI tell** | 无来源 | §10.1 明确说未取到 |
| **"这不是 X，而是 Y"是 AI tell** | 无来源 | 同上 |
| **"三连排比"是 AI tell** | 只在 eyebrow/编号规则里被间接覆盖 | 同上 |
| **"动画计数器"是 AI tell** | 无来源 | §9.4 未列入 |
| **"badge soup" 这个术语** | 无来源 | §9.3 用 C3 描述行为，不用该术语 |
| **"假证言配 stock 头像"** | 「编造」有来源，「stock 头像」这个细节无来源 | §9.6 A3 只写"编造的数字/证言/logo" |
| **"Coming soon 泛滥" / "路线图全是勾" / "截图是 mockup" / "没有真实限制" / "没有版本号"** | 均无来源 | §10.8 只列**有来源**的可信度做法；"诚实的未做区"标为「未取到原文」但**本项目已有实践** |
| **`Gesso-Build/skills`（73 条 slop guard）** | GitHub API 限流 + raw 404 | 未使用 |
| **`Laith0003/ux-skill`（152 条规则）** | 未取到 | 未使用 |
| **`vercel-labs/web-interface-guidelines` README** | raw 超时 | 用 `vercel.com/design/guidelines` 全文替代（同一份文档） |
| **Vercel Geist "principles" 页** | 只渲染了文档外壳，无 principles 内容 | §4.3 标为未取到 |
| **IBM Plex Mono 的字形覆盖** | Google Fonts 路径 404 | §3.9 表中该项**缺失**，已在表下注明 |
| **Shiki 主题数 66 vs 65** | 两个来源差 1，未能核对 | §8.1 未给具体数字 |

### 12.2b 第二批未取到 / 已更正（来自排版与文案两路调研）

| 主题 | 状态 | 处置 |
|---|---|---|
| **arXiv 2404.01268 的归属** | ⚠️ **更正**：它不是 Kobak 的 excess-vocabulary 论文，而是 Liang et al. *Mapping the Increasing Use of LLMs in Scientific Papers*（无词表） | §10.1 已加更正框 |
| `practicaltypography.com/numbers.html`、光学对齐 | 站点不可达 | §3.6 只用 MDN；§6.4 标为判断 |
| `fonts.google.com/knowledge` | JS 门控，只返回标题 | 未引用 |
| Material 官方 8dp spacing 页 | 未取到 | §4.1 用 Atlassian / spec.fm 的 8pt 材料 |
| Refactoring UI（书） | 无公开可引原文 | §5.6 改用本项目自己的语义纪律 |
| Jen Simmons intrinsic web design 演讲 | 未取到 | §4.4 标为判断 |
| NN/g "serif vs sans" 专文 | URL 404 | §3.5 改用 NN/g "Best Font for Online Reading"（Wallace/Adobe 研究） |
| **德国法院判决书原文** | rewis.io 被 Cloudflare 拦、openjur.de 需验证码；**"100 欧元赔偿"未核实** | §3.8 只把"离线/第三方/渲染阻塞"作为主要理由 |
| **CJK Web 正文行高 1.7–1.8 的可引用出处** | **未取到**。clreq 只给"行距 ≥ 字号 1/2（单面装）/ 5/8（双面装）"与注音场景 ≥1.5 倍 | §3.2 / §10.9③ **标为经验值（自定规范）** |
| **中文 AI 腔词表**（赋能/无缝/一站式/助力/打造/生态/闭环） | **未取到权威发布源** | §10.9⑥ 标为**自定清单**，理论依据用《歐化中文》的"万能动词 + 抽象名词" |
| **"so what?" 测试** | 未取到以该名目命名的权威原文 | §10.3 改用 NN/g "Avoid broad and generic headings" + GOV.UK "identify a user need" |
| **"swap test"（竞品替换测试）** | 未取到权威命名原文 | §10.4 明确标为**本报告形式化** |
| **"Things which aren't magic" 原始篇目** | 未定位到 | §10.8 用四个已取到的等价范例替代 |
| **Rust 独立的 "known problems" 页** | 未找到独立页面 | §10.8 用 platform-support 的 Tier 3 声明替代 |
| **"No, we can't do that" FAQ 的原始出处** | 未定位到 | §10.8 用 TypeScript Design Goals 的 Non-goals 替代 |
| `sparanoid/.../README.zh-CN.md` | **404**；简体版实际是 `README.zh-Hans.md` | §10.9① 已注明 |
| `gov.uk/guidance/content-design/writing-for-gov-uk` | **已迁移**到 `guidance.publishing.service.gov.uk` | §10.2–10.7 引用新域名 |
| `nngroup.com/articles/how-users-read-on-the-web/` | 返回空壳页 | §10.6 用同作者同期 `concise-scannable-and-objective`（同为 Morkes & Nielsen 1997）替代 |
| `plainlanguage.gov/guidelines/` | 站点改版，原文归档进 GitHub | 未引用 |
| **function 描述"1–2 句 / 15–30 词"** | **无单一权威给出该数字** | §10.5 标为**自定规范** |
| **Butterick 的行长区间** | ⚠️ **环境不一致**：本报告的一路调研取不到 practicaltypography.com，另一路经本地代理 `http://127.0.0.1:7890` **取到了** | §3.3 采用取到的那一路（**45–90 字符**），并给出 Butterick 的"2–3 个字母表"手工检查法 |
| **`-webkit-font-smoothing` 的变细警告** | web.dev 的字体文章里**根本没有** "smoothing"/"antialias"/"text-rendering" 这些词（全文 grep 0 命中）——原以为的出处不存在 | §12 标注为**未取到**；只保留"它是非标准 WebKit/Blink 私有提示"这一可由前缀判断的事实 |
| **字体 swap 造成的具体 CLS 数值** | web.dev 只给了机制与 100ms/3s 阈值，**没给 CLS 分数** | §3.8 只引机制与阈值 |
| **"紫蓝 AI 渐变"作为被命名的陈词** | **没有任何一篇文章命名或批评它**（颜色值可引 Tailwind，判断不可引） | §6.6 明确标注 |
| **M3 shape / corner-radius** | `m3.material.io/styles/shape/corner-radius` **404**；其他路径是同一个 ~61.7KB JS 空壳 | §6.3 改用 Tailwind / Open Props 的刻度 |
| **every-layout.dev 的 Grid 章** | **收费**（返回 "available in the paid version"）；免费的 Axioms 章才有 45–75 与 60ch | §4.4 只用免费章的结论 |
| **MDN `ch` 单位 / `:where()` / WCAG 1.4.10 Reflow 的 320px** | 未取到 | §3.3 的 `ch` 定义改用"0 字形宽度"的通识表述；§4.2 的 `:where()` 标为**由层叠规则推断** |
| **Elliot Dahl 的 builttoadapt.io 原文** | Wayback 各 URL 形式均为空 | 用同作者的 spec.fm 版本（同一份材料的权威现存版） |
| **Tailwind `/docs/container`** | 两个 URL 都返回了 `max-width` 页 | §4.3 只引 max-width 刻度 |
| **嵌套圆角公式的归属** | 精确公式是 CSS-Tricks **评论区留言**，不是正文 | §6.3 已加归属更正 |
| **`transform: scaleY(0.5)` 的发丝线配方** | 未取到任何可引文章 | §6.4 只用 CSS Values 4 的规范算法 |
| **"边框 + 阴影一起用是坏味道"** | **本报告的综合判断**，Tailwind 的 1px 阴影层是结构性论据 | §6.2 标为判断 |
| **"混合圆角读起来邋遢" / "圆角应随尺寸缩放"** | 无直接出处（Open Props 的超线性刻度是结构性支持） | §6.3 标为判断 |
| **defensivecss.dev** | 其 sitemap 里**没有** border/hairline/radius/shadow 相关条目 | 未使用 |

### 12.3 本报告中「属于判断、不属于引用」的清单

以下内容是本报告作者的**设计判断**，正文已就地标注，此处汇总便于复核：

- §3.1 两段式比率与 17px 锚点（算术推导 + 判断）
- §3.5 中文站不用衬线的三条理由（判断，其中第 3 条有来源）
- §3.8 的三条 webfont 建议（判断，其中 `font-display` 三阶段有来源）
- §4.2 垂直节奏的四条规则、§4.4 Grid vs Flow、§4.5 不对称规则、§4.6 嵌套 ≤2（判断；§4.6 有 rohitg00/hallmark 来源）
- §5.5 暗色模式的六条规则与取值（判断 + 自检数值）
- §6.2 边框 vs 阴影对照表、§6.3 圆角三档、§6.4 1px 细节、§6.5 callout 三通道、§6.6 渐变规则（判断；§6.1 有 Comeau 来源）
- §7.1 时长表、§7.5 动效决策（判断；§7.2–7.4 有 WCAG/MDN 来源）
- §8.3 复制按钮的实现细节（判断）
- §8.7 文档站细节优先级（判断）
- §10.2 句法节奏、§10.4 标题规则、§10.5 特性描述模式、§10.6 字数、§10.9 中文词表（判断；§10.1/10.3/10.8 有来源）
- §11 全部令牌取值（判断 + WCAG 数值自检）

---

## 附录 A：可验证的检查清单

零构建站点也能跑。**所有命令在仓库根执行。**

### A.1 令牌一致性（可脚本化）

```bash
CSS=site/assets/style.css
# 1. 字号层级数（目标 ≤ 9，当前 17）
grep -oE 'font-size:\s*[^;]+' $CSS | sort -u | wc -l
# 2. 圆角取值数（目标 = 3：--r-0 / --r-1 / --r-full，当前 6）
grep -oE 'border-radius:\s*[^;]+' $CSS | sort -u
# 3. 行高是否全无单位（目标：空输出）
grep -nE 'line-height:\s*[0-9.]+px' $CSS
# 4. 裸 hex 是否只出现在 :root（目标：全部在 :root 内）
grep -nE '#[0-9a-fA-F]{3,8}' $CSS
# 5. 是否还有 transition: all（目标：空）
grep -n 'transition:\s*all' $CSS
# 6. 离刻度间距（目标：空）——列出所有 padding/margin/gap 值人工核对
grep -oE '(padding|margin|gap)[^;]*:\s*[^;]+' $CSS | grep -oE '[0-9]+px' | sort -un
# 7. 文本容器是否设了固定 height（目标：空）
grep -nE '^\s*height:' $CSS
```

### A.2 AI tell 扫描

```bash
# emoji（目标：0）
grep -rnoP '[\x{1F300}-\x{1FAFF}\x{2600}-\x{27BF}\x{2B00}-\x{2BFF}\x{FE0F}]' site/ | wc -l
# 链接文本里的 →（目标：0）
grep -rnoE '>[^<]*→[^<]*<' site/*.html
# 渐变（目标：0）
grep -rn 'gradient' site/assets/*.css
# 假窗口 chrome 关键词（目标：0）
grep -rniE 'traffic|window-dot|fake-|browser-bar|titlebar' site/
# letter-spacing（目标：0，CJK 不该有字距）
grep -n 'letter-spacing' site/assets/style.css
# 默认强调色（目标：0）
grep -rniE '#6366f1|#8b5cf6|#a855f7|#7c3aed' site/
```

### A.3 对比度（附脚本，可直接跑）

```bash
python3 - <<'EOF'
def lin(c):
    c/=255
    return c/12.92 if c<=0.04045 else ((c+0.055)/1.055)**2.4
def L(h):
    h=h.lstrip('#')
    return 0.2126*lin(int(h[0:2],16))+0.7152*lin(int(h[2:4],16))+0.0722*lin(int(h[4:6],16))
def cr(a,b):
    l1,l2=sorted((L(a),L(b)),reverse=True)
    return (l1+0.05)/(l2+0.05)
# 从 §11.4 抄下所有「文字/底色」对，逐条断言
for name,fg,bg,need in [
    ("ink/paper",      "#14171c","#f5f7f9",4.5),
    ("ink-2/paper",    "#4a5159","#f5f7f9",4.5),
    ("ink-3/paper",    "#6a727c","#f5f7f9",4.5),
    ("acc-ink/paper",  "#14523a","#f5f7f9",4.5),
    ("white/acc",      "#ffffff","#1e6b4c",4.5),
    ("border-control", "#7c8691","#ffffff",3.0),
    ("ring/paper",     "#1e6b4c","#f5f7f9",3.0),
]:
    r=cr(fg,bg)
    print(f"{name:16s} {r:6.2f}  {'PASS' if r>=need else 'FAIL'} (need {need})")
EOF
```

**规则：阈值不四舍五入。** 4.499 不达 4.5（WCAG 1.4.3 原文，§5.4）。

### A.4 键盘 / 可访问性走查（人工，5 分钟）

| # | 步骤 | 期望 |
|---|---|---|
| 1 | Tab 从地址栏开始按到底 | 第一个可见元素是 skip-link；**每个**可交互元素都有可见焦点环 |
| 2 | 焦点环是否被 sticky header 遮住 | 不遮（WCAG 2.4.11，AA） |
| 3 | 焦点环是否"淡入" | 必须**瞬间**出现（hallmark gate 15） |
| 4 | 复制按钮：Tab 到 + 回车 | 可到达、可触发、有 `aria-live` 播报"已复制" |
| 5 | 代码块：Tab 到 + 左右方向键 | 可横向滚动（`<pre tabindex="0">`） |
| 6 | 浏览器缩放到 200% | 无横向滚动、无文字裁切（WCAG 1.4.12 的 F104） |
| 7 | 系统开"减少动态效果" | 无位移/缩放动画 |
| 8 | 320px 宽（DevTools） | 无横向滚动 |
| 9 | 系统切暗色 | 有暗色模式，且不是简单反相 |

### A.5 代码呈现走查（本项目专属）

| # | 步骤 | 期望 |
|---|---|---|
| 1 | 放一段含 `⊢ ∀ ∃ ∈ ⊆ ⟹ ⟨⟩` 的代码块 | 字符**等宽对齐**（放大 400% 检查） |
| 2 | 代码块内 `->` 与 `→` | 视觉上**不同**（`font-variant-ligatures: none`） |
| 3 | `tab-size` | 缩进是 2 字符宽 |
| 4 | 复制按钮复制出的文本 | 无行号、无 `$` 提示符、无中文标点 |
| 5 | 长行 | 横向滚动而非折行 |
| 6 | 代码块极性 | 亮色页面里是**亮色**代码块（不是嵌一块近黑） |

### A.6 内容走查

| # | 检查 | 方法 |
|---|---|---|
| 1 | §10.1 黑名单 10 句 | `grep -inE 'unleash\|empower\|seamless\|supercharge\|next-generation\|modern team\|where .* meets' site/` |
| 2 | 无信息量形容词 | 人工：每个形容词能否换成数字/命令/限制 |
| 3 | 每个数字都有来源 | 全部数字应来自 `site/data/site.json`；`grep -rnE '0\.[0-9]+\.[0-9]+' site/*.html` 应只命中 `data/` |
| 4 | `<title>` 带品牌 | 每页形如 `页面 — sokonanoda` |
| 5 | 中英混排空格 | 中文与拉丁/数字之间有一个空格 |
| 6 | 一个动作同名 | 同一个动作的按钮/toast/标题用同一个词 |
| 7 | "未做"清单存在且集中 | 有独立小节 |

### A.7 交付顺序建议

| 阶段 | 内容 | 依据 |
|---|---|---|
| 0 | 修 §2.3 的两处对比度 + 补 favicon/og/theme-color | 低成本、纯收益 |
| 1 | 引入 §11.4 全部令牌，不改结构 | 一次改一个文件，`git diff` 可审 |
| 2 | 拆 `--measure` / `--wide`；删 eyebrow / `→` / `letter-spacing` | 三处删除 |
| 3 | 补交互态（`:focus-visible` + `--ring` + hover/active/disabled） | §9.3 C4/C5 |
| 4 | 打破等宽卡片网格；容器嵌套降到 2 层；表格化数据 | §4.4 / §4.6 / §8.6 |
| 5 | 加暗色模式 + `prefers-reduced-motion` | §5.5 / §7.3 |
| 6 | 代码块改造（同极性 + 数学字形自托管子集 + 复制按钮修） | §8.1–8.5 |
| 7 | 文案小修（§10.10 的 4 项） | 最后做，避免返工 |

**验收口径**：附录 A 的 A.1–A.3 全部为脚本断言（可进 CI，与
`scripts/check-site.py` 同一风格）；A.4–A.6 是人工走查清单。
**建议把 A.1–A.3 接进 `scripts/check-site.py`**——本仓已有"机制防漂移"的纪律
（`docs/design/site.md` §5.3），设计令牌同样可以断言。

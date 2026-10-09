# 官网：面向新人的首页 + 更新日志子页（2026-10-09）

> **本文是 `site/` 的权威设计。** 它接替 2026-09-21 的「单页四节」版（是什么 /
> 怎么安装 / 核心特点 / 未来的计划），并继续取代 2026-09-20 的 28 页重构
> （`git log --all -- docs/archive/site-rebuild-2026-09-26/`，**不要照着它们新建页面** ✗）。
> 两版原文见 `git log --all -- docs/design/site-single-page.md`。

一条命令验收：

```bash
python3 scripts/check-site.py            # 10 项，exit 0 才算过
python3 scripts/check-site.py --browser  # 另加两项真 Chrome：零 404 + 版本回填 · 零横向溢出
```

## 1. 为什么又一次重构（2026-10-09 用户诊断）

用户原话三条：**「index.html 是给懂行的人看卖点 + 给维护者看内部状态，新人看不懂」**·
**「标题不是招牌」**·**「插件安装是 code 命令、不醒目」**。逐条对应旧版：

| 旧版有什么 | 为什么对新人无效 |
|---|---|
| 标题「你和 code agent 共用一张证明草稿纸…」 | 一句话三个未解释的概念，不是招牌 |
| 8 条核心特点 | 判卷通道 / agent 一等公民 / MCP 七查询 / 缺口台账 —— 是给维护者看的状态 |
| 未来的计划整节 | 里面是**性能优化史**（91 → 29 秒）与未还的债 |

⇒ 目标改成一句话能验收的：**30 秒看懂这是干嘛的 + 怎么开始**。

## 2. 现在是什么

```text
site/
  index.html             ← 首页：头图 / 它是什么 / 怎么开始 / 教材 / 与 Lean 4 的关系
  changelog.html         ← 更新日志子页（**生成物**，源 editor/vscode/CHANGELOG.md）
  assets/site.css        ← 唯一的样式表（令牌 + 组件，自带亮/暗两套）
  assets/site.js         ← 唯一的脚本（主题切换 / 版本回填 / 复制 / 按主题换头图）· i18n.js 中英文自动识别与切换 · fonts.css 自托管字体
  assets/hero-vscode{,-dark}.png ← 头图**两版配色**（真 VS Code + 已发布插件，生成物，见 §4）
  data/site.json         ← 生成物：已发布版本（**唯一**版本号来源）
  favicon.svg  robots.txt  sitemap.xml  llms.txt  .nojekyll
```

首页五节，顺序就是新人的问题顺序：**头图**（招牌一行「sokonanoda · 形式化证明教学
语言」+ 一句话 + 两个按钮【装 VS Code 插件】【开始第一课】+ 一张真截图）→ **它是什么**
（3 条，每条只回答一个"凭什么"：真内核判定 / 目标实时可见 / 语法是 Lean 4 子集）→
**怎么开始**（3 步：装插件 → `git clone` 打开第一个课程文件 → 把 `sorry` 填成证明）→
**教材**（入门课 `course/` 11 单元 · 卷 I《集合论》`courses/set-theory/` 13 章 112 个
单元画布）→ **它和 Lean 4 的关系**（写法是 Lean 4 的子集，练的写法在 Lean 4 里成立；
没有类型类 / tactic 宏 / mathlib 也说清楚）。

**中英文自动识别**（2026-10-09 追加需求）：静态 HTML 默认中文，每页页尾一份
`<script type="application/json" id="i18n-en">` 英文文案字典 + `assets/i18n.js`（**同步**执行，
换文案发生在首帧之前）—— `navigator.languages` 里**任何**一个 zh\* 保持中文，否则整页换英文；
**没有 `?lang=`、没有重定向**（用户点名）；topbar 有一个**语言按钮**（用户 2026-10-09 点名要，方便手动验证两种语言）—— 它只往 `localStorage["soko-lang"]` 写一个覆盖值，**自动识别仍是默认路径**，按钮再点一下即回到自动 ✓；按钮标签写**当前语言**（中文页「中」/ 英文页「EN」—— 2026-10-09 用户问过"是不是状态反了"，原先写"切过去会变成什么"会被读成"现在就是英文" ✗）。没有 JS 时页面就是完整中文版 ✓。

子页 `changelog.html`：最近 **7** 个版本的正文 + 全部版本直链 —— **只放 7 个**是因为站点有
120 KB 体积预算（`assets` 项判死），而整份 CHANGELOG 是 160 KB 量级的 Markdown；2026-10-09
加第二语言与 topbar 三控件后按"预算不抬、从内容侧省"一路 10 → 8 → **7**（每减一版约省 7 KB）✓。

## 3. 保留了什么，为什么

**设计语言整体保留**：方格纸背景、发丝线、零圆角结构容器、绿=内核通过过的东西 / 朱=诊断与
诚实的限制的语义纪律、17px 中文锚点、52rem 行长（2026-10-09 用户两轮实测「正文太窄、标题都是两行」⇒「正文最起码跟图片一样宽」⇒ 40 → 46 → 52rem = 与头图同宽，站点只剩一个宽度；判据钉在"中文 hero 标题 ≥1024 单行"）、自托管字体子集 —— 它们是从题目推出来的
选择（产物是判定 ⇒ 颜色由判定驱动），不是模板默认值。**被删掉的是受众错位，不是主张。**
版本号机制、主题切换、`.nojekyll`、`site.js` 的三个钩子全部沿用；新增结构只有三处：`.wide`
（截图与卡片与正文**同宽**，2026-10-09 起）、`.btn`（主按钮走**墨色**，不走绿：绿是语义色，拿去当行动色会稀释它）、`assets/i18n.js`（语言，见 §2）`.icon-toggle`（主题按钮改**图标**：文字版「跟随系统」四个字 vs「浅色/深色」两个字会让按钮宽度跳）与 `.header-controls`（topbar 右侧三个控件包一层：头部是 `align-items: baseline`，**图标按钮没有文字基线** ⇒ 与带文字的按钮按不同基线对齐、两个方框必然错位 ✗ —— 用户 2026-10-09 实测报的"不在一条水平线上"）。topbar 另有**一个 GitHub 仓库链接**（用户同日点名；文字而非图标，因为站点 120 KB 预算是硬约束）。

## 4. 头图与更新日志：两份生成物，各有判据

**头图不许 AI 生成/摆拍**（此前踩过：生成的 IDE 示意图文字会乱、布局假）：它必须是
"用户装完插件真实会看到的东西"。生成器 `scripts/site-screenshot.mjs` 拉**已发布 tag 的
VSIX** → 干净 profile 起真 VS Code → 把光标停在课程单元里 `inter_comm` 的 `sorry` 上 →
走命令面板打开目标面板 → **等**面板里出现 `⊢` 才截图 → 按 DOM 量出来的矩形裁切。

用 **CDP**（`--remote-debugging-port`）而不是 `screencapture`：后者要 macOS「屏幕录制」权限，本机实测被拒（`could not create image from display`）⇒ 不可复现。⚠ 带 `clip` 的 `Page.captureScreenshot` 会**按裁切框重排页面**（实测面板内容漂了 80+ px）⇒ 一律整窗截图 + `sips --cropOffset` 裁切。**判据落在像素上**：截完用系统 Vision OCR 回读，逐条断言图里真的有定理名、`sorry`、面板里的目标等式与假设；缺一条就**删掉图**并退出码 1。

**更新日志是生成物**：`scripts/gen-site-changelog.py` 从 `editor/vscode/CHANGELOG.md` 生成 `site/changelog.html`（带生成标记）；`check-site.py` 的 `changelog` 项**重新生成一次并逐字节比对** ⇒ "忘了重新生成"判红，而不是悄悄漂移。

## 5. 防漂移的机制

| 机制 | 在哪 | 为什么必须有 |
|---|---|---|
| 版本号只有一个来源 | `gen-site-data.py` → `site/data/site.json` → `site.js` 回填 | 站点写的是**已发布版本**的事实；手写会写出"没有 tag、没有产物"的版本 |
| 已发布 = 最新 `vX.Y.Z` tag | 同上（**不是** `Cargo.toml`） | 本仓库常有并行开发，`Cargo.toml` 会先于 tag bump |
| 更新日志逐字节对账 | `changelog` 项（重生成再比） | 生成物一样会腐烂，只是腐烂方式是"悄悄停在旧版本" |
| 头图宽高比与 `<img>` 一致 | `assets` 项（读 PNG 头 + `<img width/height>`） | 图被压扁这件事，光看页面看不出来 |
| **两版头图同尺寸**且都被页面指到 | `assets` 项（两张 PNG 的 IHDR 相等 + `<img data-hero-*>`） | 尺寸不同 ⇒ 其中一张是别的什么截的；漏指 ⇒ 那版是死图 |
| 横向溢出 | `layout` 项（`--browser`） | 实测抓到两个：CHANGELOG 里 836px 不可断行的测试路径、`.wide` 被组件 `margin` 简写覆盖 |

`check-site.py` 的 11 项（`--browser` 另加 2 项）：`pages`（恰好这两页、无内部目录）·
`sitemap`（与页面集合**双向**相等）· `links`（站内引用与锚点可解析，**含跨页锚点**）·
`css-urls` · `version`（每页有回填钩子 + 除生成物外零写死版本号）· `meta`（head 元数据齐全 +
每页只允许一段**可执行**内联 script）· `i18n`（字典合法 + **与 DOM 的 `data-i18n*` 键逐个对齐**，
漏译/多译判红；topbar 语言按钮恰好 1 个）· `markup` · `assets`（位图**只允许两版头图**、有预算、宽高比一致、两版同尺寸；html+css+js
≤ 120 KB）· `data`（与最新 tag 一致）· `changelog`（逐字节）· `render` / `layout`（真 Chrome；
`render` 用 `--accept-lang` 跑**两种浏览器语言**，判"中文读者看中文 / 其它语言看英文"）。

**发布之后**：先 `git fetch --tags`（`gen-site-data.py` 只看**本地 tag**，不 fetch 会静默停在旧版本 ✗），再跑 `gen-site-data.py` 与 `gen-site-changelog.py` 把新版本写进数据与子页（两者的 `--check` 会因此判红，这是有意的：本地绿必须意味着"仓库里写的就是线上写的"）。头图不必每次重截 —— 只在 UI 变化时重跑 `node scripts/site-screenshot.mjs [--theme dark]`（两版：同一 VSIX、同一靶子、同一裁切几何，只换 `workbench.colorTheme`）。

## 6. 已知边界

- **单页放不下细节**：tactic 白名单、诊断码表、语言子集的完整边界仍只在仓库文档里。
  页面只给"是什么 + 怎么开始"，不试图成为手册。
- **中英双语是"识别"不是"开关"**：没有切换按钮、不做两个页面、不加 `?lang=`；字典与页面文案的键对齐由 `i18n` 项判死，两种语言的渲染由 `render` 项在真浏览器里判 ✓。
- **子页只有更新日志**：不做文档站。真要加，先问"谁来同步这个事实"。
- **`--browser` 不在 CI 里跑**：收益不如留作发布前的本地门禁。

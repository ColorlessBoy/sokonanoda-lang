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
  assets/site.js         ← 唯一的脚本（主题切换 / 版本回填 / 复制）· fonts.css 自托管字体
  assets/hero-vscode.png ← 头图：**真 VS Code + 已发布插件**的截图（生成物，见 §4）
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

子页 `changelog.html`：最近 10 个版本的正文 + 全部版本直链 —— **只放 10 个**是因为站点
有 120 KB 的体积预算，而整份 CHANGELOG 是 160 KB 量级的 Markdown。

## 3. 保留了什么，为什么

**设计语言整体保留**：方格纸背景、发丝线、零圆角结构容器、绿=内核通过过的东西 /
朱=诊断与诚实的限制的语义纪律、17px 中文锚点、40rem 行长、自托管字体子集 —— 它们是
从题目推出来的选择（产物是判定 ⇒ 颜色由判定驱动），不是模板默认值。**被删掉的是受众
错位，不是主张。** 版本号机制、主题切换、`.nojekyll`、`site.js` 的三个钩子全部沿用；新增结构只有两处：`.wide`（截图与卡片比正文栏宽一档 —— 40rem 是**读**的约束，不是版面
的约束）与 `.btn`（主按钮走**墨色**，不走绿：绿是"内核真的通过过"的语义色，拿去当行动
色会把语义稀释掉）。

## 4. 头图与更新日志：两份生成物，各有判据

**头图不许 AI 生成/摆拍**（此前踩过：生成的 IDE 示意图文字会乱、布局假）：它必须是
"用户装完插件真实会看到的东西"。生成器 `scripts/site-screenshot.mjs` 拉**已发布 tag 的
VSIX** → 干净 profile 起真 VS Code → 把光标停在课程单元里 `inter_comm` 的 `sorry` 上 →
走命令面板打开目标面板 → **等**面板里出现 `⊢` 才截图 → 按 DOM 量出来的矩形裁切。

用 **CDP**（`--remote-debugging-port`）而不是 `screencapture`：后者要 macOS「屏幕录制」权限，本机实测被拒（`could not create image from display`）⇒ 不可复现。⚠ 带 `clip` 的
`Page.captureScreenshot` 会**按裁切框重排页面**（实测面板内容漂了 80+ px）⇒ 一律整窗
截图 + `sips --cropOffset` 裁切。**判据落在像素上**：截完用系统 Vision OCR 回读，逐条
断言图里真的有定理名、`sorry`、面板里的目标等式与假设；缺一条就**删掉图**并退出码 1。

**更新日志是生成物**：`scripts/gen-site-changelog.py` 从 `editor/vscode/CHANGELOG.md`
生成 `site/changelog.html`（带生成标记）；`check-site.py` 的 `changelog` 项**重新生成一次
并逐字节比对** ⇒ "忘了重新生成"判红，而不是悄悄漂移。

## 5. 防漂移的机制

| 机制 | 在哪 | 为什么必须有 |
|---|---|---|
| 版本号只有一个来源 | `gen-site-data.py` → `site/data/site.json` → `site.js` 回填 | 站点写的是**已发布版本**的事实；手写会写出"没有 tag、没有产物"的版本 |
| 已发布 = 最新 `vX.Y.Z` tag | 同上（**不是** `Cargo.toml`） | 本仓库常有并行开发，`Cargo.toml` 会先于 tag bump |
| 更新日志逐字节对账 | `changelog` 项（重生成再比） | 生成物一样会腐烂，只是腐烂方式是"悄悄停在旧版本" |
| 头图宽高比与 `<img>` 一致 | `assets` 项（读 PNG 头 + `<img width/height>`） | 图被压扁这件事，光看页面看不出来 |
| 横向溢出 | `layout` 项（`--browser`） | 实测抓到两个：CHANGELOG 里 836px 不可断行的测试路径、`.wide` 被组件 `margin` 简写覆盖 |

`check-site.py` 的 10 项（`--browser` 另加 2 项）：`pages`（恰好这两页、无内部目录）·
`sitemap`（与页面集合**双向**相等）· `links`（站内引用与锚点可解析，**含跨页锚点**）·
`css-urls` · `version`（每页有回填钩子 + 除生成物外零写死版本号）· `meta`（head 元数据
齐全 + 每页只允许一段内联 script）· `markup`（标签配对 + 无内联 `style=`）· `assets`
（位图**只允许**头图、有预算、宽高比一致；html+css+js ≤ 120 KB）· `data`（与最新 tag
一致）· `changelog`（逐字节）· `render` / `layout`（真 Chrome）。

**发布之后**：跑 `gen-site-data.py` 与 `gen-site-changelog.py` 把新版本写进数据与子页
（两者的 `--check` 会因此判红，这是有意的：本地绿必须意味着"仓库里写的就是线上写的"）。
头图不必每次重截 —— 只在 UI 变化时重跑 `node scripts/site-screenshot.mjs`。

## 6. 已知边界

- **单页放不下细节**：tactic 白名单、诊断码表、语言子集的完整边界仍只在仓库文档里。
  页面只给"是什么 + 怎么开始"，不试图成为手册。
- **中英双语已取消**（2026-09-21 起的决定，本次不改）。
- **子页只有更新日志**：不做文档站。真要加，先问"谁来同步这个事实"。
- **`--browser` 不在 CI 里跑**：收益不如留作发布前的本地门禁。

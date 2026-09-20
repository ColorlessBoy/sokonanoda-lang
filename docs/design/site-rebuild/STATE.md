# 站点重构 —— 断点续传状态

> ## ⚠️ 这批文档已降级为历史存档（2026-09-21）
>
> 用户在同一天判定 28 页重构「属于灾难」，要求**简化成单个网页**
> （是什么 / 怎么安装 / 核心特点 / 未来的计划）。**当前权威 =
> `docs/design/site-single-page.md`**；导航生成器、搜索索引、走查数据、
> 诊断码表页与 28 个页面都已删除，验收从 `site-verify.py`（18 项）变成
> `scripts/check-site.py`（10 项）。
>
> **本文仍然有效的部分**：§5 与 #13 的「站点写的是已发布版本的事实」测量陷阱、
> #12/#13 的实测修正清单——那些是**关于这台仓库的事实**，不是关于页数的。
> 下面所有「28 页」的施工计划都已作废，不要照着执行。`spec/D1-design-rules.md`
> 的设计主张被单页版**原样继承**（色值与令牌一个字没改），它仍是视觉层的参考。
>
> 用途：本文件记录「哪些已完成、哪些在跑、哪些还没开始」，让任何一次上下文丢失
> 或 subagent 崩溃之后都能从断点继续，而不是重做。**每完成一个环节就更新它。**
>
> 权威仍是 `D2-information-architecture.md`（施工图）与 `D1-design-rules.md`
> （设计规则）。本文件只是进度表。
>
> 最后更新：2026-09-19，站点重构 S0 阶段。

## 0. 一句话现状

**站点已建成，全绿**：28 页（27 中文 + 英文 landing），零构建手写 HTML/CSS/vanilla JS，
零位图、零 CDN、零第三方请求；8 份数据文件由生成器产出，版本/计数/轮次永不手写。
唯一验收 = `python3 scripts/site-verify.py`：**18/18**（CI 跑 `--quick` 的 16 项）。

**下一步不是"把站点建完"，而是三件事**：

1. **美学 A/B 仍待用户拍板**：现在实现的是用户选定的方向 ①′「格纸 · 朱批」
   （冷调纸 + 24px 格纸 + 无衬线）；另一条"字面照 brief"的暖纸 `#f7f3ec` + 衬线
   变体已建好并核验过对比度，配方 `.cache/variant/serif-warm.css`。**`.cache` 不入库**
   ——若选 B，先把配方提进 `site/assets/` 再改，别让它随缓存一起消失；
2. **0.62.0 发布后**按 §12.2 的五步刷新事实（记法三条 + 6 份 lab 数据 + K12/K16）；
3. §5 里尚未处置的实测条目（写页面时按实测，别抄仓库文档）。

## 1. 已完成 ✅

| 产物 | 说明 |
|---|---|
| `docs/design/site-rebuild/research/R1-design-craft.md` | 设计手艺调研（3078 行）：AI 聚类清单、72 条 AI 味特征、令牌草案 |
| `docs/design/site-rebuild/research/R2-docs-teardown.md` | 65 个站点拆解；Alectryon 纯 CSS 折叠机制；lean-lang.org 技术性诊断 |
| `docs/design/site-rebuild/research/R4-fonts.md` | 字体管线报告（字形覆盖逐条实测） |
| `docs/design/site-rebuild/content/C1-language.md` | 语言能力（1404 行）：语法清单、7 个 tactic、**60 错误码 + 4 警告**、诚实边界 |
| `docs/design/site-rebuild/content/C2-teaching.md` | 教学体系（1109 行）：卷 I 12 单元、96 练习、L2 库 74 声明、缺口台账 24 条 |
| `docs/design/site-rebuild/content/C3-tooling.md` | 工具链（1293 行）：VS Code 14 命令、CLI 14 子命令、agent 集成、发布 |
| `docs/design/site-rebuild/content/C4-status-roadmap.md` | 现状与路线（761 行）：快照、39 条时间线、愿景、未完成清单、45 行数字总表 |
| `docs/design/site-rebuild/spec/D1-design-rules.md` | **设计规则手册**（986 行，权威） |
| `docs/design/site-rebuild/spec/D2-information-architecture.md` | **施工图**（21 页 + 数据模型 + 工程契约 + 23 环节） |
| `site/assets/tokens.css` | 令牌（对齐 D1；含两处刻意偏离的说明） |
| `site/assets/fonts.css` + `fonts/` | 自托管子集 71.3 KB：Source Serif 4 / JuliaMono / STIX Two Math |
| `site/assets/site.js` | 渐进增强层（10.3 KB，预算 12 KB） |
| `site/_partials/{header,footer}.html` | 导航与页脚的**唯一源** |
| `scripts/gen-site-nav.py` | 导航单源写入/校验（`--check` 当前绿） |
| `scripts/check-site.py` | 卫生检查，**10 条**断言（链接/版本/导航/数据页/元数据/体积/无位图/无内联样式/turnstile/sitemap） |
| `scripts/site-shot.py` | headless Chrome 截图（1440/900/390），约 1.7 秒/张 |
| `scripts/gen-site-fonts.py` | 字体管线（可复现，幂等） |
| `scripts/fetch-agent-skills.py` + `dsh/agent-skills.json` | 第三方 agent 技能（26 个，MIT，装入 `.dsh/skills/`，不入库） |
| `site/data/site.json` | 版本/轮次/单元计数（既有生成器） |
| `site/data/walkthrough.json` | 走查页真实逐行目标状态（133 KB） |

## 2. 进行中 ⏳

| 环节 | 交付 | 状态 |
|---|---|---|
| S0-CSS-A | `site/assets/base.css` + `site.css` + `D5-css-components.md` | base/site 已落地（**由主 agent 亲手写**，原子 agent 卡死）；D5 缺 |
| S0-CSS-B | `site/assets/editor.css` + `D6-editor-components.md` | ✅ 完成（38202 B / 15 段可粘贴片段） |
| S0-DATA | `gen-site-lab.py` 扩展 + `D3-lab-data.md` | ✅ 完成（6 份数据、3 次跑字节一致、诊断复现 55/64） |
| **S0b 设计检查点** | `styleguide.html` + 截图 + 客观审计 | ✅ **已完成，待用户过目** |

## 2.1 S0b 检查点实测（可复跑）

```bash
python3 scripts/check-site.py            # styleguide 自身 0 问题（其余是旧页，待替换）
python3 scripts/site-audit.py site/styleguide.html
python3 scripts/site-shot.py site/styleguide.html --out .cache/shots
```

实测结果：

| 项 | 值 |
|---|---|
| 真视口（iframe 造） | 1440×900 / 900×900 / **390×900**，三档**均无横向溢出** |
| `.turnstile` | 3 个（记忆点已生效） |
| `<h1>` / `<img>` | 1 / 0（零位图目标达成） |
| 方格纸 | 已绘制（`body` 的 `background-image` 非 none） |
| 字号刻度 | h1 34px / h2 28px / body 17px（= `--fs-3xl` / `--fs-2xl` / `--fs-base`） |
| `⊢` 落在哪个面 | **Soko Symbols**（自托管），无平台回退 |
| 自托管字体装载 | `Soko Mono` ✓ `Soko Symbols` ✓ |
| CSS 体积 | 96.7 KB raw / **29.8 KB gzip**（预算 100 KB / 32 KB） |
| 单页 HTML | styleguide 36.5 KB / 60 KB |

**三个工具缺陷已在本轮修掉**（都会导致假绿，值得记）：

1. **`site-shot.py` 的 390px 不是真 390px**：macOS headless Chrome 把窗口宽度钳在
   500px，`--window-size=390` 产出的是「500px 布局的左侧裁切」（像素比对
   22360/22360 全同）。已改为窄屏走**定宽 iframe**，窗口开到 520。
2. **`site-audit.py` 的探针量错了文档**：脚本执行时 `contentDocument` 是初始
   `about:blank`，它 `readyState` 就是 complete 且**有 body** —— 只查这两样会量到
   空文档，得出「390 不溢出、turnstile 0」的假绿。已加 `d.URL !== "about:blank"` 校验。
3. **`site-audit.py` 超时后会挂死**：`communicate` 超时后再 `proc.stdout.read()`
   会因管道未关而永久阻塞。已改用 `TimeoutExpired.output`，并 `killpg` 收掉
   Chrome 的整个进程组。

另修：`site.css` 的 `.squiggle` 引用了**不存在的** `var(--mark)`（tokens.css 里只有
`--mark-wash`），整条颜色声明会失效；已改为 `var(--verm-ink)`。

## 3. 未开始 ⬜

- **S0b 设计检查点**：✅ 已完成（`styleguide.html` + 截图 + 客观审计），待用户过目
- **已建成页面**（5）：`styleguide.html` · `404.html` · `index.html` · `kernel.html` · `walkthrough.html`
- **在建**（S5 批次）：`language.html` · `diagnostics.html` · `editor.html` · `get-started.html` · `progress.html`
- **待建**（18）：`why` · `vision` · `compare` · `tactics` · `notation` · `projects` · `agents`
  · `course` · `set-theory` · `learn` · `glossary` · `faq` · `docs` · `search` · `releases`
  · `engineering` · `about` · `en/index`
- S22 收尾：`pages.yml` paths、`README`/`AGENTS`/`docs/README` 同步、agent-prompt 改 `_partials`
- S23 总验收：全站截图 + 体积/对比度报告 + AI 味复审

### 3.0 第二轮修掉的缺陷（工具层，都会产生**假绿**）

| 缺陷 | 后果 | 修法 |
|---|---|---|
| `check-site.py` 的版本正则 `\b0\.\d+\.\d+\b` | **假阳性**：代理示例 `127.0.0.1:7890` 被判成写死版本号 `0.0.1`；**假阴性**：`v0.61.0` 反而漏检（`\b` 在 `v` 与 `0` 之间不存在） | 改成 `(?<![\d.])0\.\d+\.\d+(?![\d.])`：拒 IP、抓 `v0.61.0` |
| `site.js` 从不为复制按钮移除 `hidden`，而 `site.css` 说 hidden 由页面给出 | 加了 hidden → 按钮**永不出现**；不加 → 无 JS 时是个按不动的**死按钮** | 改用 `html.js` 门控：无 JS 不显示，有 JS 才显示；页面不写 `hidden`（D5 已订正） |
| `base.css` 的 `text-rendering: optimizeLegibility` | 打开连字，而本语言 `->` 与 `→` 是两个东西——排版对字符撒谎；D1 明令禁止 | 删掉，并写明为什么不写 |
| `site-audit.py` 走 `file://` | `404.html` 的资源必须是站点根绝对路径（任意深度被提供），file:// 下必然 404 → 那一页永远报假红 | 审计改为**本地 HTTP 服务 site/ 为根**，与 GitHub Pages 行为一致；顺带不再需要 `--allow-file-access-from-files` |
| `.feature` 窄屏轨道写 `1fr` | 裸 `1fr` 的最小值是内容固有宽度，右栏一放长行面板就把轨道撑到 638px，390px 整页横滚 | 改 `minmax(0, 1fr)`（宽屏那条本来就写对了） |
| CSS 预算 100.0 KB 卡在实测 100.1 KB | 逼作者删注释去凑整数 | 预算改为**实测基线 + ~15%**（115 KB raw / 35 KB gzip），用途是拦回归而不是凑整 |

**⚠️ `crates/` 的 WIP 在增长**：会话开始时 +862 行 / 3 文件，现在 **+1548 行 / 14 文件**
（`parser.rs`、`token.rs`、`semantic.rs`、`compile/elab.rs`、`spine.rs`…），
有别的开发者在并行改语言。**这与站点无关，但坐实了「必须钉发布产物」**。
`check-site.py` 现在每次运行都会打印这条警告。

### 3.1 本轮修掉的缺陷（都会静默产生错误页面，值得记）

| 缺陷 | 后果 | 修法 |
|---|---|---|
| `site.css` 引用 `--mark*`，`tokens.css` 只定义 `--verm*` | CSS 自定义属性解析失败是**静默**的：`.callout--limit` / `.chip--open` / `.chip--failed` / `.diag` 全部拿到 transparent，页面"没报错"但提示框没底色 | 机械改名为 `--verm*`，并补 `--verm-border`（浅一档，3.78:1 过 3:1 控件边界线） |
| `site.css` 引用 `--lh-normal`、`--tracking-*`，从未定义 | 行高与字距声明整条失效 | 补进 `tokens.css` |
| 上述这类 bug 无人能发现 | | **`site-audit.py` 新增「语义色探针」**：抓 `.callout--*` / `.chip--*` / `.diag` 的计算值，凡是该有底色却是 `transparent`/`rgba(0,0,0,0)` 就判红 |
| 404 在任意深度被提供，相对路径会二次 404 | 错误页自己也是坏的 | `gen-site-nav.py` 支持 `<body data-root="/">` → 导航块写**站点根绝对路径** |
| 404 没有 `data-page`，生成器拒绝写 | 生成器硬报错，阻塞所有页面 | 引入显式退出标记 `data-page="none"`（缺属性仍报错——那与"忘了"无法区分） |
| `site/sitemap.xml` 是手写清单 | 手写清单在本仓漂移过 6 次 | `check-site.py` 新增第 10 条断言：sitemap 与真实页面集合**双向**相等 |

**验证状态**：`check-site.py` 10 条断言里 8 条绿；红的 2 条是 `links` 与 `sitemap`，
两者的全部失败项都是「指向尚未建好的页面」——它们现在同时充当施工清单。

## 4. 关键决定（别重新讨论）

1. **不做 WASM / 不做交互功能页**（用户拍板）。功能靠**真实预计算数据 + 纯 CSS 折叠**展示。
2. **语言范围**：中文主站 + 英文 landing（一页）。
3. **方向**：「格纸 · 朱批」。冷调纸面 + 24px 方格（只铺在推导发生处）+ 零圆角结构容器 + 发丝线。
4. **两个语义色，可审计**：绿 = 内核真的通过过；朱 = 诊断/更正/诚实的限制。看到强调色就问「背后有什么真实判定」。
5. **衬线默认撤掉**（`--font-display` = `--font-ui`），教科书感改由**构件**承担（编号小节、页边注、定理/证明块、推理横线）。字体文件保留，一行开关可逆。
6. **`--measure: 40rem`**（≈37.6 汉字 / ≈78 拉丁字符，同时满足 WCAG 1.4.8 的两个上限）。
7. **零位图**：编辑器面板是真 HTML/CSS，旧版 PIL 假截图全部退役。

## 5. 已知的仓库文档与实测不符（写页面时按实测，别抄文档）

来自 C3/C1 的实测，**都会变成网站上的假话**：

1. `scripts/soko` 文档承诺的「VS Code 扩展自带」回退是**死代码**（`extensionServer()` 从未被调用）。
2. 两个 `doctor` 结论相反：`sokonanoda doctor` → `ready:false` exit 3；`scripts/soko doctor` → `ready:true` exit 0。
3. playground 锚点计数在所有文档里都过期。**实测值取决于版本基准，两种都是真的：**
    - **`v0.61.0` 的画布 + 已发布的 0.61.0** = `30 / 2 / 4 / 2` ← **站点发布的数**
    - 工作树的画布（未提交）+ 下一版构建 = `23 / 2 / 4 / 1`
    差在下一批改动把 playground 里自建的 7 条 `And`/`Or` 公理换成了内建记法。
    ⚠️ 已发布的 0.61.0 **解析不了工作树那份画布**（只出 1 条 diagnostic）——
    所以「现场跑一遍」在这里必然是错的，必须用发布 tag 的画布 + 发布二进制。
    这条已固化为 **K16**（`site-verify.py`），判据与 K12 同款。
4. release 资产 **26** 个，不是 25（`docs/RELEASE.md:90` 过期）。
5. playground 锚点**不在 CI 里**（只在 `scripts/soko gate`）。
6. 不存在 `v0.57.0` tag（而 0.57.0 正是多文件 import 那个大版本）。
7. `docs/architecture.md:519` 仍把 `by match` 列为未做，实测 0.61.0 可用。
8. `docs/design/course-stdlib.md:109` 的「`Exists` 4 条」是旧数，实测 3 条。
9. 结构债行数比 `HANDOVER` 登记值更大（`tests.rs` 7926 / `parser.rs` 4702 / `elab.rs` 4130 / `extension.js` 1880 / `lib.rs` 1610）。
10. **不得写**：10–100x 性能对比（未测量）、用户数/下载量（无数据）。
11. `docs/protocol.md:214-216` 说人读视图打印 `error[<stage>]`——**只有 parse 是**。
    实测：parse 打 `error[parse]`，elab 打 `error[elab-unknown-identifier]`，
    kernel 打 `error[kernel-expected-pi]`，即**码**不是阶段。0.55 与 0.61 行为一致，
    属长期偏差而非新回归。
12. `docs/protocol.md:244-250` 说 `CompileError` 带 `expected`/`actual` 机器字段——
    **没有**。扫了四种内核拒绝，`--json` 事件与 `query check` 的 `failed[]` 里
    都没有这两个键，两侧只活在 `message` 字符串里。
13. **⚠️ 测量陷阱：本工作区 `crates/` 是脏的，`scripts/soko` 量的是未发布代码。**
    `crates/front/src/{ast,by,parser,judge,proof,semantic,compile/*}.rs` 有 **+862 行
    未提交 WIP**（`by` 块新增 `constructor`/`cases`/`left`/`right`/`use`/`exfalso`）。
    `scripts/soko` 的解析顺序里「仓库构建」优先，而 `target/debug/sokonanoda` 正是用
    这棵脏树编的（marker 写着 `0.55.0`，`--version` 却报 `0.61.0`）。
    **实证**：`by cases c` 在 repo build 上出 `decl.checked`，在**已发布的 0.61.0**
    上出 `parse` 阶段的 `unexpected-token`。同一份源码，两个结论。
    **我据此误判过一次**：曾把 C1 §3.1 的「七个 tactic」改成「十三个」，是错的，已改回。
    **规矩**：站点写**已发布版本**的事实；量内核行为前先钉发布产物——
    `export SOKONANODA_BIN=~/.vscode/extensions/sokonanoda-lang.sokonanoda-0.61.0-darwin-arm64/bin/darwin-arm64/sokonanoda`，
    或先确认 `git status --short crates/` 干净。
    **已核对的受影响面**（结论：已发布内容**未受影响**）：诊断码集合 HEAD 与工作树
    完全相同（99/99，增 0 删 0）；55 条已发布复现**没有一条**用到新 tactic；
    `events.json` 的样例源码不含任何 `by` 块。
14. **⚠️ `query check` 对「用了 import 来的记法」的文件报假红。** 由 `notation.html`
    作者发现，主 agent 用**钉住的 0.61.0** 独立复核。同一个文件、同一个二进制：
    - `scripts/soko query check --file courses/set-theory/units/notation-cheatsheet.sokonanoda`
      → **exit 1**，`failed[0] = notation-unknown-symbol @ 96:42`，计数 18/2/3 正常
    - `scripts/soko grade <同一文件> --json` → **exit 0**，24 条事件，**0 条 diagnostic**
    机理（与 `docs/design/notation-subset.md` §11.6/§11.7 一致）：入口先按单文件解析
    （拿不到继承来的记法表）→ 报 `notation-unknown-symbol`；随后 `project::is_project_source`
    正确改判为项目并闭包重编（计数证明重编跑过了），但**第一次的 parse 诊断留在了
    `failed[]` 里**。自己声明的记法不受影响；import 来的**名字**不受影响，只有记法。
    **站点规矩**：凡是让读者/agent 验证「用了记法的文件」，一律用
    `scripts/soko grade <file> --json`，**不要用 `query check`**。受影响的页面：
    `notation` / `language` / `agents` / `course` / `set-theory`，以及 DSH 的
    `mcp__sokonanoda__check`（同一个通道）。
    **⚠️ 版本标注（2026-09-21 语言/课程线复核）**：这条**只属于已发布版本**；
    未发布的下一批改动里**已不再复现**（同一命令、同一文件，工作区构建 exit 0、
    `failed: []`）。站点四页（faq / language / agents ×2）已加版本标注，
    措辞**不含版本字面量**（`check-site.py` 禁止）：「这是当前发布版本的行为；
    未发布的下一批已不再复现；发布之后请重测」。
    **另一处容易混的形状**（复核者提醒）：「页面自己声明了与 `import` 同名的记法」
    报的是 `import-module-invalid`「符号已经声明过记法了」——**那是正确拒绝，不是假红**。
    两者机理不同，别混为一谈。
    **尚未进 `docs/gaps/ledger.jsonl`**（G-10/G-17 是相邻但已修的两条）。
15. **print-back 限制是「位置性的」，不是全局的。** 由 `notation.html` 作者发现，
    主 agent 用钉住的 0.61.0 独立复核。渲染方式取决于**光标在哪**：
    - 值不是 `by` 块（裸 `sorry` / 项值）→ 源码级渲染，**记法保留**
    - `by` 块、第一条 tactic **之前**（`step -1`）→ 冻结内核打印器，**点名形式**
    - `by` 块、走过 ≥1 条 tactic（`step ≥ 0`）→ 源码级渲染，**记法保留**
    - `#check` / `expr.typed` → 内核打印器，**点名形式**
    同一文件的三个位置实测：`--line 4` → `h : x ∈ y`；`--line 6` →
    `forall (x y : Nat), Mem Nat x y -> True`；`--line 9` → `h : x ∈ y`。
    真实课程文件 `--line 236 --col 3` → `((∅) ∪ A) ⊆ A`。
    `site/kernel.html` §7 原先写成「目标面板一律点名形式」，**过强，已按实测改写**。
    `docs/design/notation-subset.md` §0/§13.1 与 C1 §2.7 的笼统说法同样过强。
16. D6 §1 的示例文件名 `SetTheory.lean.sokonanoda` 在仓库里**不存在**——
    真实文件是 `course/unit4-by-tactics.sokonanoda` 一类。
17. 旧的 `site/assets/agent-prompt.js` 决定**退役**：安装 prompt 只在 `agents.html`
    正文里存在一份（HTML，无 JS 可读），其他页面链接过去——不再四处内联复制。
18. **⚠️ `gen-site-data.py` 原先把「已发布版本」读成 `Cargo.toml`——那是下一个版本。**
    2026-09-21 语言/课程线提交（`08b6782`，全课程 Lean 4 化）之后实测到的连锁：
    - `Cargo.toml` 已是 `0.62.0`，而 `v0.62.0` **没有 tag、没有 Release、没有产物**；
    - 原生成器照 `Cargo.toml` 写 `version`，于是站点 28 个页脚 + 每条下载指令
      会一起指向一个**下载不到的版本**，而 `compare.html` 正明说这个数是
      「已发布的版本，不是工作树」；
    - 同一次重跑还会用**工作树**的课程 + **仓库 debug 构建**量出 `329 → 328 checked`，
      即把未发布的数写进站点。**这就是 K12 存在的理由，而它确实会因此判红**
      （实测：`git status` 干净的重跑也会，因为 build 不是 release）。
    **修法**：生成器改为从**最新 `vX.Y.Z` tag** 取 `version`，并把课程清单与判卷
    都锚到那棵树 + 那把已发布二进制（`release_version()` / `release_tree()` /
    `release_binary()`，与 K12/K16 同款）；取不到 tag 时**沿用上次写下的版本**，
    绝不退回 `Cargo.toml`。顺带把 `pages.yml` 的 checkout 改成 `fetch-depth: 0`
    ——浅克隆不带 tag，生成器就解析不出发布版本。
    **实测确认**：`version: 0.61.0（发布 tag v0.61.0）；Cargo.toml 已是 0.62.0，
    尚无 tag` + `门禁实测（v0.61.0 的课程 × 0.61.0 的二进制）36/329/99/0`，
    与站点原值逐项相同，`site-verify.py` 18/18。
    **同源的下一处**（尚未做）：`gen-site-lab.py` 的 6 份数据文件信封仍直接读
    `Cargo.toml`，发布前重跑会把未发布的 `source_commit`（08b6782）与工作树量出的
    数混进站点，K10/K16 会判红。详见 `spec/D3-lab-data.md` §1.1 的警告框。

## 6. 验证命令速查

```bash
python3 scripts/check-site.py            # 卫生检查（7 条断言 + 实测体积）
python3 scripts/gen-site-nav.py --check  # 导航/页脚与 _partials/ 一致
python3 scripts/site-shot.py             # 截图到 .cache/shots/
python3 scripts/gen-site-lab.py          # 重新生成走查数据
python3 scripts/gen-site-data.py         # 重新生成 site.json
python3 scripts/fetch-agent-skills.py --check   # 第三方技能是否在位
scripts/soko query state --file playground.sokonanoda --line 333 --col 4   # 真目标状态
```

## 7. 总验收（`scripts/site-verify.py`）

用户问「怎么验证整个 site 改动的完整性与正确性」。答案是**一条命令**：

```bash
python3 scripts/site-verify.py          # 18 项，含浏览器审计
python3 scripts/site-verify.py --quick  # 16 项，跳过两个要 Chrome 的检查
python3 scripts/site-verify.py --json   # 机器可读
```

它把已有的三个工具（`check-site.py` / `gen-site-nav.py --check` /
`site-search.py --check`）与**十条新判据**合成一份报告，分两组：

**完整性**：C1 页面清单（与 D2 站点地图逐页对账 + 孤儿检查）、C4 导航标记、C5 头部元数据。
**正确性**：K1 无写死版本、K2 CSS 令牌（含"引用了未定义令牌"）、K5 链接与预算、
K6 AI 味模式（7 类可机械判定的长相）、K7 标签配平 + 可访问性基础、
**K10 零伪造（数据保真）**、K11 出处可查、**K12 课程计数可复现**、
**K14 语义色纪律**、**K16 playground 计数可复现**、**K17 版本是发布 tag**、
K3/K4/K13 渲染审计、K15 交互实跑。

两条最值得说的判据：

- **K13 暗色可验证**：整套审计原先**只量亮色**，暗色一直是未验证状态。现在探针会在
  活页面上打 `data-theme="dark"`（切换按钮走的就是这条路）再量一遍：令牌要真的换掉、
  语义件要真的算出来。实测 28 页全部 `dark ok`，暗色令牌 `#0f1216 / #e5e9ee /
  #7fcfa5 / #ef7a5c`，0 处失效色。
- **K10 零伪造**：把页面里当作"真实内核输出"渲染的片段抽出来，回
  `site/data/*.json` 里逐条找。找不到 = 编的或数据过期。当前 **93 条录制值全部回查通过**。
  这是本站最重要的一条正确性检查——站点上所有内核输出都是预计算的，
  要么逐字来自数据文件，要么就是编的，没有第三种可能，而人眼分辨不出来。
  **它的边界（2026-09-21 补）**：原来只抽三类**逐字复制**的字段（走查目标、诊断码、
  事件类型），因此**散文里引用的数据盖不住**。实测漏掉的就是这一类：`index.html`
  写着 `` `counts_source: previous-run` ``，而生成器在有已发布二进制的机器上会实测成
  `gate` —— 页面成了一句谎话，**没有任何检查会响**（另见 §5 #18）。现在补了第二类：
  首页编辑器插画引用的 playground 计数（`decl_checked: 30`）按 **playground 事件名翻译**
  后回查 `evidence.json`（`decl.checked`），反向测试做过（改成 31 → 判红）。
  **仍不覆盖**：散文里以别的形状引用的值（路径、`infix: 50 " ∈ " => …` 这类记法语法、
  URL、doctor 输出样例）。通用版要靠手工豁免表活着，那种检查会烂掉——真正的解法是把
  这类值**挪进数据文件、由 JS 回填**（版本号就是这么做的），属于页面改版的事。
- **K12 课程计数可复现**：站点唯一一类"数字来自一次测量而非某个文件"的内容。
  判据不是"现场重跑一遍"，而是 **发布 tag 的课程 + 已发布二进制**能不能跑出页面写的数
  （`git archive v0.61.0 courses/set-theory` → 临时目录 → 钉住的 0.61.0 跑门禁）。
- **K16 playground 计数可复现**：同一个道理的另**一条腿**——`playground.sokonanoda`
  是**会随语言一起改**的仓库文件。开发中的下一批改动把它从「自建 7 条 And/Or 公理」
  改成内建记法，计数因此从 30/2/4/2 变成 23/2/4/1。只有"**该 tag 的那份画布** ×
  **发布的那把二进制**"才得到站点该写的数；用工作树现场跑会被未发布语法骗到
  （实测：0.61.0 在工作树的画布上只出 1 条 diagnostic）。
- **K17 版本是发布 tag**：前两条只管计数，**版本号本身没人管**——而 28 个页脚、
  `about` 的统计卡、每条下载指令都从它拼出来。它写错时前两条会**安静跳过**（按
  `site.json` 的版本去找已发布二进制与 tag，找不到就报"跳过：不算通过"，措辞和
  平时一模一样），于是站点可以带着一个**下载不到的版本**上线而报告全绿。
  这条判据**直接调用生成器自己的 `release_version()`**（`importlib` 按路径载入
  `gen-site-data.py`）：判据与产它的人不会各自漂移。
  **"谎话"与"落后"分开判**（2026-09-21，合入 main 前收紧过一次）：站点版本
  **没有任何 tag** ⇒ 判红（读者按它下载会 404，这就是实测到的那条病）；
  站点版本**有 tag、只是比最新的旧** ⇒ 只记一笔（`v0.61.0 落后于最新 tag v0.62.0`）。
  理由是部署安全：发布刚落地、站点数据还没重跑时，`pages.yml` 正在部署——判红会让
  **整条部署挂掉**，把一个"该重跑生成器"的待办伪装成故障。宁可绿着部署一份落后一个
  版本的站点（**旧的真话**），也不要红着不部署（**新的空白**）。
  三种情形都实测过：正确 → `v0.61.0`；临时造 `v9.9.9` → 记一笔不判红（造完即删）；
  写成没有 tag 的 `0.99.0` → 判红并指名"不是任何发布 tag"。

**当前结果：18/18 全绿，28/28 页，exit 0。**

### 7.1 为什么 K12 非有不可（一次真实的假警报）

工作树的 `courses/` 与 `crates/` 都是脏的（并行开发）。现场跑课程门禁得到
**36/321/93/2（exit 1）**，而站点写的是 **36/329/99/0** —— 看起来像站点写错了。

用 `git archive` 把当时的课程解到临时目录、配已发布的 0.61.0 二进制重跑，
得到 **36/329/99/0**，与站点**逐项相同**。所以：

- 站点发布的是**已发布版本**的计数，**是对的**；
- 差异 100% 来自工作树未提交的课程改动；
- 判据必须是"已发布的课程 + 发布二进制"，不能是"现场跑一遍"——否则每次并行开发都会误报。

这条也顺带确认了 §5 的版本钉纪律：**站点的一切事实以发布产物为准**。

### 7.2 基准从 `HEAD` 收紧到**发布 tag** + 一个把正确复现判成红的陷阱

上面最初写的是 `HEAD`。那是错的，只是当时还没咬人：站点描述**已发布版本**，而发行
之间 `HEAD` 常常装着发布版二进制**根本解析不了**的下一批改动（实测：0.61.0 在发行后的
`playground.sokonanoda` 上只出一条 diagnostic）。拿 `HEAD` 当基准，等于用一个版本不匹配
的源去核对另一个版本的产物。现在两条计数判据都先解析 `release_ref(version)`
（= `v0.61.0`，退而求其次裸 `0.61.0`），找不到 tag 就**跳过并说明**，不假装通过。

收紧之后 K12 立刻转红，而红的原因**不是**站点写错了数——是一个真实的陷阱：

> `courses/set-theory/tools/check.py` 要沿目录向上找 `scripts/soko` 才认"本检出"，
> 再从该目录的 `Cargo.toml` 读版本钉，与二进制 `--version` 比对；不一致就
> **exit 2 拒绝判卷**。临时目录只解了 `courses/set-theory`，于是它一路上溯、
> **撞到了本仓库根**，拿 HEAD 的钉（0.62.0）去卡已发布的 0.61.0 二进制，
> 报"判卷二进制与仓库版本不一致，结果不可信"。

门禁**是对的**——它那双眼睛分不出"已发布的二进制"和"走错门的工作树"。错的是我给它的
临时目录不是一个自洽的检出。修法：解 tag 时连 `scripts/soko` 与 `Cargo.toml` 一起解
（`members = ["courses/set-theory", "scripts/soko", "Cargo.toml"]`），临时目录于是有了
**自己的**钉 = 该 tag 的版本，判据才真的在说"能不能复现"。修完实测
**`v0.61.0 + 0.61.0 实测 36/329/99/0`**，与站点逐项相同。

教训（与 §5 同源，值得单独记）：**"在干净检出上复现"里的"干净检出"是一个具体的目录
形状，不是一句意图**。只要把源解到别处、却让工具自己的定位逻辑落在原仓库上，量到的
就还是原仓库——而且它还会理直气壮地报红一个正确的答案。

## 8. 第三轮修掉的缺陷（都是"写在文档里但没人检查"的规则被违反）

用户问「怎么验证整个 site 改动的完整性与正确性」。除了建 `site-verify.py`，
这一轮还修掉了五处**规则明明写着、却没人执行**的缺陷：

| 缺陷 | 为什么它能活这么久 | 修法 |
|---|---|---|
| **全站每个链接都用语义色**（`a { color: var(--acc-ink) }`，悬停转 `--verm-ink`） | tokens.css 文件头、D1、验收清单都写着「绿只给内核判定、朱只给诊断与限制」，但**没有机器判据**。链接两样都不是，于是 28 页的每一个超链接都在违反它，悬停时链接还变成"诊断"色 | 链接改为**墨色 + 下划线**（悬停时下划线变实变重）；主按钮改为**墨色实底**；导航当前页、标签页选中、搜索高亮、着重号、选区一并不用语义色。新增 **K14** 机器判据，并做过**反向测试**（故意植入违规 → 判红） |
| CSS 体积预算**只数四份文件** | 写死的文件名清单与"实际有哪些文件"是两件事。子 agent 后加的 `diagnostics.css` 一直没被计数 | `CSS_BUDGET_FILES` 改为 glob；基线因此从 100 KB 变成 **114.8 KB raw / 36.8 KB gzip**（7 份），预算按实测重设 |
| 页头在宽屏**占两行**（123px） | `.site-nav { flex-basis: 100% }` 无条件生效，导航独占第二行 | ≥60rem 时回到品牌那一行 → 页头 74px，h1 从 309px 提到 244px |
| 窄屏页头 **164px 且 sticky** | 8 条链接在 390px 要换 2–3 行，而 sticky 让它永久吃掉五分之一屏 | <60rem 时页头**不吸顶**（随内容滚走），导航间距收紧 |
| 窄屏表格**横滚没有提示** | 溢出被 `overflow-x: auto` 藏住了，截图里看不出来 | 加两侧滚动阴影（`background-attachment: local`，纯 CSS 零 JS），新增 `--shadow-scroll` 令牌 |

**还修了 `docs/README.md` 的两处路径错误**（由 `docs.html` 的作者核出）：
`HANDOVER.md` 被列在仓库根（实际在 `docs/HANDOVER.md`）、
`docs/notes/project-view.md` 不存在（实际在 `docs/design/project-view.md`）。

**新增的审计指标**：`site-audit.py` 现在报 `h1@Npx`（标题距顶端距离，用来验证页头间距）
与 `hdr sticky|static`（分断点的定位）、以及**暗色模式**（K13：在活页面上打
`data-theme="dark"` 再量一遍——整套审计原先只量亮色）。

**当前：16/16 全绿，exit 0。**

## 9. 第四轮：补上唯一会"点击"的检查（K15）

前两轮的检查**全是静态的**——读源码、量布局、查数据。它们没有一次**点过任何东西**。
所以站点上每一行 JavaScript 都可以是坏的，而其余各条全绿通过。

这不是假设：**搜索功能就是这么发布出去的**。它被审计过溢出、审计过配色、
审计过暗色，却**从未被执行过一次**。

`scripts/site-functest.py` 补上这一环：起 HTTP 服务（与 GitHub Pages 同条件），
在真浏览器里驱动真控件，断言真结果。当前 **18 项全过**：

| 用例 | 实测 |
|---|---|
| 搜索：输入「内核判卷」 | 2 条命中，结果里含关键词 |
| 搜索：输入不存在的词 | 给出空态「没有匹配…」 |
| 搜索：无 JS 兜底清单 | 被 `search.js` 藏起来（`hidden=true`） |
| 版本回填 | `data-site-version` 填成 `v0.61.0` |
| 主题切换 | 点击后 `data-theme` 由 `null` → `light`，`--paper` 跟着变 |
| 诊断页筛选 | 输入后 64 → 1 条，清空后 64/64 |
| 走查页折叠 | 勾选前 `display:none`，勾选后显示 |
| 复制按钮 | 有 JS 时可见（`display: block`） |
| **无 JS 完整性** | 5 页正文仍在（1154–19661 字），且复制/筛选/搜索控件**都不显示** |

**怎么模拟"无 JS"**：`--blink-settings=scriptEnabled=false` 会让 `--dump-dom`
**彻底不输出**（实测 stdout 长度 0），量不到东西。改成**把 `.js` 请求拦下来返回空**
——脚本开着但站点 JS 一行没跑，对页面等价，而且量得到。

**两个探针自身的 bug 也记在这里**（都会造成误报）：
1. 一上来就 `querySelector`，那时 `contentDocument` 还是初始 `about:blank` →
   一律返回 null，于是报"搜索全挂"。实际是探针没等加载。
2. 拿 `kernel.html` 测复制按钮——**那一页根本没有复制按钮**（只有 agents /
   course / notation / set-theory / styleguide 有）。报的是"页面缺按钮"，
   其实是我的假设错了。

## 10. 第五轮：可访问性与打印（都是"写了但从没量过"的东西）

上一轮补的是"从没点击过"，这一轮补的是**"从没在渲染结果上量过"**。

### 10.1 渲染后的对比度（不是令牌注释里的对比度）

`tokens.css` 的注释里每个颜色都算过对比度，但那是**设计意图**；真正决定可读性的
是最终渲染出来的前景色与背景色——中间隔着继承、半透明层、以及"某个容器忘了给
不透明底"这类问题。探针现在会**往上找有效背景**（很多元素自己是透明的），
再按 WCAG AA 判（正文 4.5:1；≥18.66px 粗体或 ≥24px 大字 3:1）。

**实测（28 页 × 1440px）：**

| 指标 | 值 |
|---|---|
| 量的渲染文本 | **1170 处** |
| 低于 WCAG AA | **0 处** |
| 全站最低对比度 | **4.54:1**（walkthrough.html 的 13px 图注） |
| 焦点不可见的页面 | **无**（每页 `outline: 2px solid`，实测宽度 2） |

### 10.2 焦点环曾经是**绿色**

渲染探针一上来就抓到一个静态检查**永远看不见**的问题：
`--ring-color: var(--acc)` —— 焦点环是绿的。焦点环是可访问性 affordance，
不是内核判定，按本站自己的纪律它不该用语义色。

改成 `--ring-color: var(--ink)`（亮色 16.7:1 / 暗色 14.6:1，对比度反而更高），
并把 **K14 扩展到令牌定义**：`--ring-color`、`--shadow-scroll` 这类令牌
**不许由语义色定义**。

> 这是本轮最有价值的一条：**静态检查看不见渲染结果**。前四轮所有检查加起来
> 都没发现焦点环是绿的，因为没人 focus 过任何东西。

### 10.3 打印样式（对照实验）

写了 `@media print` 但从没渲染过。用 `--print-to-pdf` 做**对照实验**：
复制站点、把 `base.css` 的 `@media print` 整块删掉，两边各渲一次。

| 版本 | 体积 | 页数 | 链接注解 |
|---|---|---|---|
| 打印 CSS 生效 | 528 KB | **6 页** | **0 个** |
| 打印 CSS 被中和 | 808 KB | 9 页 | 80 个 |

导航、页脚站点地图、主题按钮、skip-link 全部被正确移除（80 个链接注解归零），
省下 3 页 280 KB。**打印路径是真的能用的，不是写了就算。**

（PDF 里的中文是 CID 编码，抽不出明文；所以判据用**页数 + 链接注解数**这种
结构化信号，而不是找字符串——找字符串会得到假阴性。）

### 10.4 外链全查（125 个）

把 28 页里所有 `http(s)` 链接去重后逐个请求：

| 类别 | 数量 | 结果 |
|---|---|---|
| 真·外链（github.com 96 + marketplace 1） | **97** | **全部 200** |
| 自指链接（canonical / og:url / sitemap） | 28 | 404 —— **因为站点还没部署**，部署后即生效 |

自指链接的 404 恰好是最有力的证据：**这套站点还没上线**，只差一次提交与推送。

### 10.5 已知限制（写下来，不当成没发生）

- **没有 `og:image` 社交卡片**。全站零位图是硬规则，而主流平台不支持 SVG 作
  `og:image`。要加就得引入一张位图并给它写一条有理由的 allowlist 条目——
  这是**留给用户的取舍**，不是遗漏。
- `en/index.html` 只有一页，`hreflang` 指向中文主站；全站双语镜像**明确不做**
  （用户已选「中文主站 + 英文 landing」）。
- `_partials/` 会被 GitHub Pages 一起发布（`header.html` / `footer.html` 是纯片段，
  无敏感内容，但严格说是"发布了不该发布的文件"）。要挡住得在 upload 前删掉，
  收益不抵复杂度，故记在这里。

## 11. 第六轮：把"快"从**文件大小**变成**真实加载数字**

用户说过「快也很重要」。此前只用**文件大小**间接验证——那不等于快：请求数、
渲染阻塞资源、字体加载时机都会影响。探针现在读浏览器的 Performance API：
请求数、传输字节、TTFB、DOMContentLoaded、load，以及**每个资源的名字与大小**。

首屏（首页，本地 HTTP，1440px）：`12 req / 182 KB`，TTFB 7ms，DCL 102ms，load 104ms。

**一列出资源清单，立刻看到两处纯浪费**（光看总数永远发现不了）：

| 浪费 | 原因 | 修法 |
|---|---|---|
| **22 KB 的衬线字体** | 衬线早已不在设计里（`--font-display` 指向无衬线栈），但 `--font-symbols` 还留着 `"Soko Serif"` 作后备——浏览器因此会为任意"符号面缺字"的字符去下载它 | 从 `--font-symbols` 摘掉（落到 Soko Mono，覆盖更广）。字体文件仍留在仓库，A/B 切回时不用重新下载 |
| **12 KB 的 `site.json`** | 页脚的 `[data-site-version]` 由 `site.js` 回填，而它拉的是**整份** site.json（12 KB）。28 页每一页都要填一次版本号，为一个字符串付 12 KB | 生成器多写一份 `data/version.json`（**37 字节**），`site.js` 改拉它；`check-site.py` 断言两份版本一致 |

**效果：首页 12 req / 182 KB → 11 req / 148 KB，传输字节 −19%。**

> 这两条都是"只有量了才会发现"的类型：静态检查看得见文件**存在**，
> 看不见文件**被下载了**。

**一次差点犯错的排查**：我接着怀疑 `editor.css`（38 KB）被 11 个"用不到它"的页面
白白加载——用 `grep 'class="(editor|goal|tok-...)'` 一扫，那 11 页果然 0 命中。

但那个 grep 是**错的**：它要求类名出现在 `class="` 之后的第一位，而
`class="tok-ty"` 之外的写法（多类名、类名不在首位）全被漏掉。改成
「抽出页面用到的**全部**类名，与 editor.css 的 94 个独有类求交集」之后，
结论完全反转：**凡是引了 editor.css 的页面都真的在用**（最少的也用了 2 个
`.tok-*` 做语法着色）。0 页可摘。

差一点就因为一个粗糙的 grep 删掉 11 个页面的样式表链接。**记在这里**：
"扫一下没命中"与"确实没用"是两件事，判据要写严。

## 12. 语言/课程线的复核回信（2026-09-21）—— 逐条处置

对方用 **钉住的 0.61.0** 与 **工作树 0.62.0** 两把二进制各跑了一遍，回信五条 + 三条新发现。
**这是本仓第一次有人对着我们钉的基准做同等强度的复核**，值得逐条记。

### 12.1 五条复核的结论

| # | 我们的说法 | 复核结论 | 站点处置 |
|---|---|---|---|
| ① | `scripts/soko` 的「扩展自带」是死代码 | **确认**（`extensionServer` 只有定义、零引用；执行门禁注释里也在承诺它） | **站点已经写对了**（agents / faq / get-started 三处明写「启动器没有这条回退，解析链只有四步」）。无需改 |
| ② | 两个 `doctor` 结论相反 | **确认，但不是 bug**：`scripts/soko doctor` 答「能不能跑起来」（exit 0），`sokonanoda doctor` 答「缓存是不是当前版本」（exit 3）——两个不同的问题 | **站点已经写对了**（get-started 明写「两个 doctor 不是一个东西，以启动器那个为准」）。无需改 |
| ③ | `protocol.md` 两处与实测不符 | **两处都确认**：方括号里是**诊断码**（只有 parse 的码恰好与阶段同名）；`expected`/`actual` **不是**机器字段 | **站点已经按实测写**。无需改 |
| ④ | `query check` 对 import 记法的文件报假红 | **0.61.0 确认；0.62.0 已不复现** | ⚠️ **必须加版本标注** —— 已在 faq / language / agents（2 处）四处补上，措辞**不含版本字面量**（checker 禁止），改为「当前发布版本的行为；下一批已不再复现；发布后请重测」 |
| ⑤ | print-back 限制是位置性的 | **认同**，并指出设计文档 §0/§13.1 的笼统说法该改 | **站点已按位置性写**（kernel.html §7 是我按实测改的）。无需改 |

**复核者另外提醒的一处易混形状**：页面**自己**声明了与 `import` 同名的记法时报的是
`import-module-invalid`「符号已经声明过记法了」——**那是正确拒绝，不是假红**。两者机理不同。已记入 §5 #14。

### 12.2 三条新发现 —— **属于未发布的 0.62.0，暂不上站**

复核者给出三条记法的边界（`∅ = A` 的真正根因是**宇宙层级推断**而非操作数位；
`Eq`/`Ne` 的点名形式还要写 `.{1}`；`∅ ⊆ ∅` 报 `elab-notation-argument-unsolved`）。

**处置：记在这里，暂不写进站点。** 理由两条：

1. **它们描述的是未发布行为。** `=`/`≠` 作为**语言内建记法**是 0.62.0 的新特性
   （`docs/design/lean-style-0.62.md:21`）。我拿**已发布的 0.61.0** 试着复现，
   `∅ = A` 与 `A = ∅` 都在 `=` 处报 `unexpected-token` —— 在 0.61.0 里 `=` 根本不是
   内建记法，所以那三条边界的**前提在已发布版本上不成立**。站点写的是已发布版本的事实。
2. **我没有独立复现成功。** 我搭的最小复现工程模块根解析不了（`import Set` 报
   `import-not-found`），没有跑出复核者报告的那三个诊断。按本站「零伪造」的规矩，
   **没亲自跑出来的结论不进页面**。

**0.62.0 发布后的动作**（一次做完，别分次）：

1. 把这三条按实测补进 `notation.html`；
2. 从 **`v0.62.0` 的树**重跑 `scripts/gen-site-lab.py`（6 份数据文件）——
   它的信封 `version`/`source_commit` 直接读工作树，发布前重跑会把未发布状态混进
   站点（`spec/D3-lab-data.md` §1.1 的警告框）；
3. 更新 `kernel.html` 里逐字引用的 `source_commit` / `generated_at`（K10 会查）；
4. 重跑 `scripts/gen-site-data.py`（版本与课程计数会自动跟到 `v0.62.0`）+ K12/K16；
5. 重跑 `python3 scripts/site-verify.py` 全绿。

### 12.3 复核者发现的**我们这边确实过期**的一条

`playground` 锚点计数。复核者说 23/2/4/1，我们说 30/2/4/2 —— **两个都对，基准不同**：

- **`v0.61.0` 的画布 + 已发布 0.61.0** = `30 / 2 / 4 / 2` ← 站点写的、也是读者能复现的
- 工作树的画布（未提交）+ 0.62.0 = `23 / 2 / 4 / 1`

差在下一批改动把 playground 自建的 7 条 `And`/`Or` 公理换成了内建记法。
已固化为 **K16**（判据 = 发布 tag 的画布 + 发布二进制），§5 #3 也改写清楚了。

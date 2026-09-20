# R2 — 技术产品站与文档站拆解（设计取证）

> 为 `sokonanoda-lang` 官网重建提供**可执行**的布局与交互决策依据。
> 约束（`docs/design/site.md`）：零构建静态 HTML/CSS/vanilla-JS + GitHub Pages，**无后端**。
> 新增诉求：**功能页**——游客能在浏览器里真的体验产品，而不是看广告单
> （用户对 `lean-lang.org` 的原话：「只有一个简单的广告单」）。

## 取证方法（先说清可信度）

- 本文的**结构级事实**（首屏元素、导航层级、DOM 标记、CSS 规则、静态资源体积、接口路径） 全部来自**实抓原始 HTML/CSS/JSON/JS**（`curl -sL -x http://127.0.0.1:7890`），不是回忆。
- `web_fetch` 对本任务**不能当主力**：它对长页面**尾部截断**——`lean-lang.org/` 只回收到 导航与一句正文。凡"首屏到底有什么"的判断一律以原始 HTML 为准，并给出**可复算的数字**。
- **抓取失败/异常如实标注，不猜测**：`lean4web.leanprover-community.org` **DNS 解析失败**（curl exit 6）；`coq.github.io/doc/` **404**；`djvelleman.github.io/stg4` 及其根域 **GitHub Pages 404**；`jscoq.github.io` 只有 **71 字节**的 `meta refresh`；`exercism.org` /
  `shadertoy.com` **403**；`observablehq.com` **429**；`khanacademy.org` 课程页返回 `<title>Client Challenge</title>`（机器人挑战页）；`swift.org` 首次 curl 返回二进制乱码（服务端压缩未协商），加 `--compressed` 后正常。
- 本地 `web_search` 不可用（`DEEPSEEK_API_KEY` 未配置），**未使用搜索引擎**；所有 URL 均直接访问验证。

---

# Group A — 证明助手与形式化方法（最相关）

## lean-lang.org（产品落地页）★ 反面主样本

> 完整拆解见 **D10**；这里只列结构事实。

- **首屏 DOM 顺序**：导航 → `<section id="banner">`（**一条书的广告**）→ `<section id="why-lean" aria-label="Hero">`（SVG wordmark + `<p class="hero-tagline">` + `Install` / `Learn` 两个 CTA）。**首屏 = 导航 + 卖书 + 一句话 + 两个按钮。**
- **落地页没有 `h1`**：DOM 里第一个标题是 `h3 Trustworthy`，第一个 `h1` 在约 **68,000 字符之后**（`Get Started with Lean`）。实测 16 `<section>` / 11 `<h2>` / 4 `<h1>` / 18 `<img>`；正文 10,607 字符 / 218,969 字节（20.6×）。
- **首屏零代码、零安装命令、零版本号**：`/`、`/install/`、`/install/manual`、`/learn/`、`/use-cases/`、`/community/`、`/fro/` 全无版本号；全站无 `curl`/`brew`/`sh` 命令 token。
- **首屏代码是 JS 动画**（`initCodeAnimations`，逐 token `animationDelay`）；playground 链接**跳到另一个子域**并带 base64 载荷：`live.lean-lang.org/?from=lean#codez=<base64>`。**落地页零 `<iframe>`——域内无法体验产品。**
- **信息层级倒置**：技术内容在第 4 屏才出现；前面是 hero → 代码 tab → 三根支柱 → `Get Started with Lean`，后面才是用例卡、进展、**5 条名人证言**、赞助商。营销机械：`testimonials.js` / `gallery.js` + `glightbox` / `motion.js`。**工程卫生**：同页加载 **MathJax 与 KaTeX**，`theme.js` include **两次**。
- **`/learn` 是链接农场**：TPIL / FPIL / MIL / Language Reference / FAQ / Mathlib API / Hitchhiker's Guide / Logic and Proof / Mechanics of Proof / Founder's Blog 平铺为同侪，各挂 `READ NOW`。**没有"你是程序员/数学系/中学生 → 从 X 开始"的决策程序**；`/documentation/` 还 301 到
  `/learn/`。
- **暗色模式本身不错**（`.dark-theme{--color-surface:#121212;--color-text:#eee}`，logo 显式换两套），**但 `/install/` 的四张截图是纯浅色 PNG** → 暗色下显示四张刺眼浅色图。

## live.lean-lang.org（Lean Playground）

- **HTML 仅 2,214 字节、`<body>` 里 17 个词**：`<div id="root">` + `<noscript>` 一句 "You need to enable JavaScript to use the lean web editor, as it is a React app." ——诚实，但**零静态回退**（不说什么产品、不给截图、不给替代路径）。
- **不是 WASM**。`lean4web` README 原话： *"the Lean server is running on a web server, and not in the browser"*， 并自我限定 *"some (smallish) Lean snippets"*、*"Doodling around with Lean before installing it as a newcomer"*、*"serious Lean code
  development and larger projects are considered out-of-scope"*。
- 含义：**Lean 的"在线试一下"是服务器路线，官方自认只够"涂鸦"**，两个实例 （`live.lean-lang.org`、`lean.math.hhu.de`）都需要有人长期付服务器钱； 社区的 `lean4web.leanprover-community.org` **已经死了**。

## Theorem Proving in Lean 4（TPIL，`leanprover.github.io/theorem_proving_in_lean4/`）★ 体积取证

- 导航**一页给全**：全书 12 章 + **本章全部小节** + 上一章/下一章（实测 `←4. Quantifiers and Equality` / `6. Interacting with Lean→`，带章号）+ `Source Code` / `Report Issues`。
- **单章 = 单页**：`Tactics/` 一章 **3,467,259 字节 HTML，可读正文 91,166 字符，标记比 37.3×**；`<span>` **48,780** 个、`data-*` 属性 **32,168** 个，而装代码的 `<pre>` 只有 **5** 个。
- 那些 `data-*` 不是垃圾：`data-binding="var-_uniq.2"` / `data-verso-hover="1"` / `data-lean-context="examples"` ——**每个变量出现都挂 hover 术语表**的代价。**结论：逐 token 交互换来 37 倍体积，要做就得按需加载。**
- **客户端搜索，零后端**：`-verso-search/searchIndex.js` + `elasticlunr.min.js` + `fuzzysort.min.js` ——整个检索索引作为静态文件随站发布。**这是我们在 GitHub Pages 上做搜索的直接先例。**
- 另加载 `copybutton.js`（复制）、KaTeX、`tippy` + `popper`（术语悬浮）。 章节内小节**只能靠 `#anchor`**，没有独立 URL。

## Mathematics in Lean（`leanprover-community.github.io/mathematics_in_lean/`）

- Sphinx + RTD 主题的默认答案：左侧**全展开**目录（13 章 + 每章 2–6 小节，展到 `#anchor` 级）， 顶部 `View page source`，底部 `Next`，另有 `Index`。**没有面包屑、没有版本选择器。**
- 版本只出现在 `<title>`：`Mathematics in Lean v4.19.0 documentation`——**页面上唯一且隐蔽的一处**。
- 有 `1.1. Getting Started` 作第一章，但**第一章之前还有一道"先装 Lean + Mathlib"的墙**， 且没有"读完这章去做什么"的出口。

## rocq-prover.org（ex-Coq）★★ 最值得抄的对手

- **版本状态是导航的一等公民**：导航里直接列三条 `Latest Rocq Prover release: 9.2.0` / `Latest Rocq Platform release: 2026.07.0` / `Latest Rocq Platform Starter release: v1.1.0`，各链 `/releases/<ver>`。
- **落地页嵌真实证明会话**（`fac` 阶乘的完整脚本 + 每步目标状态 + 真实消息 `[Loading ML file ring_plugin.cmxs … done]`），用 **Alectryon** 渲染。**这是"落地页上就能看见系统在工作"的范例，而且不需要任何后端。**
- **Alectryon 的交互是纯 CSS**（我读了它的 CSS 与 JS 逐条确认）：`<input class="alectryon-toggle" id="fac-pos2-v-chk1" style="display:none" type="checkbox">` + `<label class="alectryon-input" for="fac-pos2-v-chk1">…该句代码…</label>` + 兄弟节点 `<small
  class="alectryon-output">…目标状态…</small>`； CSS `.alectryon-io .alectryon-sentence > .alectryon-toggle:checked ~ .alectryon-output { display: block }`， 另有 `:hover` 与 `.alectryon-target` 两条显示路径。**零 JS、零后端、零 WASM。**
- `alectryon.js` 只有 **6,586 字节**，且**只实现"幻灯片模式"**（`.alectryon-target` 高亮 + 滚进视野） ——可选第二层。另有 `alectryon-extra-goal-toggle`：一个 tactic 出多个子目标时逐个展开。
- **暗色模式干净**：`<body>` 内**第一个** `<script>` 同步读 `localStorage.theme`，回退 `matchMedia('(prefers-color-scheme: dark)')`，在内容渲染前挂 `.dark`；**两套 logo SVG** （`dark:hidden` / `hidden dark:inline`）；页脚给 **Light / Dark / System 三个按钮**。
- `/install` 用**二维选择器**：编辑器（`VS Code`/`Emacs`/`Vim`/`RocqIDE`）× 系统（`Linux`/`macOS`/`Windows`）， 并**诚实标注缺口**：`Currently, there is no longer a Rocq Platform binary installer for Linux.`
- 落地页带 `Releases` + `Changelog` 两个**信息段**（不是链接），各附 `See All Releases` / `See full changelog`。`Curated Resources` 六张卡**每张写清自己适合谁**：`Reference Manual — The authoritative reference … (not learning oriented)`、 `Exercises — Learn Rocq by
  solving problems`。
- 缺点：落地页 `h2` 密集（Trusted in Academia / RELIABILITY / DIVERSITY / EXTENSIBILITY / PERFORMANCE / Releases / Changelog / Users / Curated Resources），首屏仍是宣言而非"能跑的东西"。

## Agda（`agda.readthedocs.io` + `wiki.portal.chalmers.se/agda`）

- readthedocs 主题，左栏全展开：`Overview` / `Getting Started`（`What is Agda?` / `Installation` / `Troubleshooting` / `'Hello world' in Agda` / `A Taste of Agda` / `A List of Tutorials`）/ `Language Reference`（**约 40 条按字母序平铺，无分组、无折叠**）。
- **唯一像动线的部分**是 Getting Started 里的 `Hello world` → `A Taste of Agda` 两页递进示例； 其余是按特性/字母排序的参考条目。**参考手册当目录用，新手找不到入口。**
- 官方 Wiki 是 PmWiki：公开文档页顶部挂着 `View | Edit | History | Print`——**把编辑按钮给读者看**。
- Wiki 里出现 `Bengt Nordstr�m`：**Latin-1 / UTF-8 编码 bug**，二十年前的问题至今在线。
- 好的一面：HowTo 一节**按"问题"组织**（`How to input Unicode characters in Emacs`、 `How to cope with performance issues`），比按特性组织更贴近读者语言。

## Isabelle（`isabelle.in.tum.de`）

- HTML4 + `HTML Tidy` 生成 + XHTML 命名空间；带 `js/osdetect.js` + `css/osdetect.css`—— **用 JS 探测操作系统**去改下载建议；没有 JS 就得不到安装路径。
- `/documentation.html` 上**所有教程与手册都是 PDF**（`prog-prove` / `isar-ref` / `implementation` / `system` / `jedit` / `sledgehammer` / `nitpick` / `eisbach` …）—— **站内没有可浏览的 HTML 正文文档**，搜索、锚点、复制全部失效； 只有 `Theory libraries for Isabelle2025-2` 是可浏览
  HTML。
- 页面上仍挂着 **`Site Mirrors: Cambridge (.uk) / Munich (.de) / Sydney (.au) / Potsdam, NY (.us)`** ——CDN 时代已无意义的镜像列表，却占首屏导航位。
- 版本 `Isabelle2025-2` 只出现在小节标题里，**没有版本选择器、没有 changelog 入口**。

## jsCoq（`jscoq.github.io` → `coq.vercel.app`）★ "文档即游乐场"

- `jscoq.github.io` 本身是 **71 字节**：`<meta http-equiv="refresh" content="0; URL=https://coq.vercel.app">`。
- 真正的首页首屏**直接给一个可点的证明**（`A First Example: rev ∘ rev = id`），术语可点； 有 `scratchpad.html` 空白本、`examples/inf-primes.html`、`examples/sqrt_2.html`、 `/ext/sf`（**Software Foundations 全套**）、`/fun/coqoban.html`（**Çoqoban：用 Coq 证明玩推箱子**）。
- 这是"文档即游乐场"的最强形态：**每个例子页都是可编辑的 Coq 会话**，Coq 编译成 JS/WASM 跑在浏览器里。 代价是首包体积与编译时间（WASM 路线的固有税）。

## Coq/Rocq 参考手册（`coq.inria.fr/doc/V8.20.0/refman/`）

- **URL 里带版本号**——好事：可长期引用、可并存多版本。`coq.github.io/doc/` 是 **404**。
- 左栏是**全书目录全展开到 `#anchor` 级**：`Core language` → `Basic notions and conventions` → `Syntax and lexical conventions` → `Syntax conventions` → `Lexical conventions` → `Essential vocabulary` → `Settings` → `Attributes` → `Flags, Options and
  Tables` …；单页 **282 KB**。
- **没有面包屑、没有"你在哪"的进度感，只有一棵无限深的树**——"侧栏把你淹没"的教科书案例。

## TLA+（`tlapl.us` / Lamport 主页）

- 首屏第一行就是 `You'll miss a lot on this web site unless you enable Javascript in your browser.` ——**"用 JS 才能读正文"的自我声明**（诚实，但等于承认正文不可索引）。
- HTML 用 `iso-8859-1`，generator 元标签写着 `Mozilla/4.05 [en] (X11; I; OSF1 V4.0 alpha)`。
- **值得抄**：目录里**每个链接后面跟一句用途说明**—— `High-Level View — An explanation of what TLA+ is all about.`、 `Learning TLA+ — Resources for learning how to use TLA+, including an introductory video course.`、 `Advanced Topics — For those who know enough
  about TLA+ to be able to read simple specifications.` 最后一条尤其好：**明确写出"读它需要什么前置"**。
- 作者明写 *"I have retired, and I don't know if I will make any further changes to the site"* ——站点事实冻结，但**没有归档横幅**，读者要读完首段才知道。

## K Framework（`kframework.org`）

- 侧栏**一次列出全部**：`Section 1: Basic K Concepts`（22 课）+ `Section 2: Intermediate`（17 课）+ `K User Manual` + `K Cheat Sheet` + `K Tool Reference` + `Builtins`。**无折叠、无进度，39 课全平铺。**
- 提供 `pdf / epub / mobi / html` 四种导出（`./exports/K.pdf` 等）——**一本手册四份离线副本**。
- `Install K` 指向 `github.com/runtimeverification/k/releases/**latest**`——**用 `latest` 不可复现** （我们仓库明令禁止，此处为反面例证）。
- 有 `K Cheat Sheet` 单页速查——**长教程的必需品，值得我们有**。

## Natural Number Game / Lean Game Server（`adam.math.hhu.de`）★★ 可直接抄的数据模型

- React SPA，首屏 HTML 仅 **1,480 字节**；`<noscript>` **诚实**写明 "You need to enable JavaScript to use the Lean Game Server, as it is built using React." 并把 Impressum / Datenschutzerklärung 放进 `noscript`——**合规文本不依赖 JS，做得对**。
- **关卡内容是完全静态的 JSON**（我直接抓到了）：`…/data/g/leanprover-community/nng4/game.json` → **9,358 字节**；`…/data/g/leanprover-community/nng4/level__Tutorial__1.json` → **27,098 字节**。 路径模板（从其 bundle 读出）：`` `${game}/game.json` `` 与 ``
  `${game}/level__${world}__${level}.json` `` ——**一个关卡一个文件**，纯静态目录，可 CDN、可缓存、可 diff。
- **关卡 JSON 的字段就是一份教学页面的完整规格**（实测 key）：`title`（`The rfl tactic`）/ `introduction`（Markdown+LaTeX）/ `descrText`（散文陈述 `If $x$ and $q$ are arbitrary natural numbers, then $37x+q=37x+q.$`）/ `descrFormat`（**带洞的语句** `example (x q : ℕ) : 37 * x +
  q = 37 * x + q := by`）/ `conclusion`（通关后的话，并教下一步：`Now click on "Next" to learn about the rw tactic.`）/ `tactics` / `lemmas` / `definitions` / `lemmaTab` / `module` / `index` / `image` / `displayName`。
- **侧栏清单也是数据**：`tactics` / `lemmas` / `definitions` 每条带 `locked` / `new` / `proven` / `disabled` / `hidden` / `category` / `displayName`， 且带 **`altTitle` = 签名提示**（`" (a b c : ℕ) : a + b + c = a + (b + c)"`） ——**一个数组同时当解锁清单和签名速查表**。
- **`game.json` 的 `tile` 就是课程卡片规格**：`title` / `short`（一句话）/ `long`（Markdown）/ **`prerequisites: []`** / `levels: 79` / `worlds: 9` / `languages: ['en','zh','uk','it','fr']` / `image`。**`prerequisites` 与 `languages`
  是显式字段**——前置关系与多语言是数据，不是文案。
- **世界地图是图不是列表**：`worlds.edges = [["Implication","AdvAddition"],["Tutorial","Addition"],…]` + `worldSize`（Tutorial 8 / Power 10 / … / Addition 5）。**"冒险感"来自拓扑，不是美术。**
- 进度存 localStorage，`info` 明写 *"The game stores your progress in your local browser storage. If you delete it, your progress will be lost!"*，并提供 `lean4game-<game>-<date>.json` **导出/导入**（`cloud-download` / `cloud-upload` 图标）。
- **代价要认清**：判卷必须服务器（`/data/stats` 实时吐 `CPU, MEM`），前端 bundle **6,114,967 字节（6.1 MB）**。 这是"必须联网 + 必须有人付服务器钱"的路线，**与我们的零后端约束相反**—— **我们要抄的是它的数据模型，不是它的执行架构。**

## stg4（"Sets, Logic and Computation" 游戏）

- `https://djvelleman.github.io/stg4` 与 `https://djvelleman.github.io/` **双双 GitHub Pages 404**； 仓库 `github.com/djvelleman/stg4` 仍在。**教学游戏最大的风险不是设计，是托管寿命。**
- → 我们的功能页必须**仓库内自包含**（数据 + 渲染器都在 `site/`），不依赖任何第三方运行时服务。

## xenaproject.wordpress.com

- WordPress 博客做数学形式化传播：证明**博客形态也能承载严肃技术内容**， 但**无目录、无进度、无结构化导航**，靠搜索和标签，长文超过 5 屏就找不回来。
- 对我们的意义：博客式"进展/随笔"可以作**补充**，不能当**文档骨架**。

# Group B — 一流的开发者文档站 / 产品站

## docs.stripe.com ★ 信息架构满分、交付方式零分

- **首屏 h1 就是 `Documentation`**，只给两个入口：`Get started with Stripe` → `/get-started/use-cases` 与 `Explore all products` → `#products`。**"带我走"与"我自己挑"并列**，不猜用户水平。
- **按用例组织，不按模块组织**：`## Payments → Accept payments online / Collect payments with invoices / Accept payments in-person`；`## Revenue → Sell subscriptions / Offer usage-based pricing / Set up the customer portal`。读者找的是"我要做 X"，不是"你们有 Y 模块"。
- **`## Try it out` 是文档首页的一等 section**，五个真按钮（`Start a payment` / `Start a checkout session` / `Create a customer` / `Sell a product` / `Issue coupons`）；**测试凭据也印在首页**：`Test cards` 段落直接给 `4242 4242 4242 4242`。用例清单下面**紧接着就是"动手"**。
- 头部 `Search /`（带 `/` 快捷键）与 **`Ask AI`** 并列成两个一级入口；`Sign in` 带 `?redirect=…docs.stripe.com%2F`（**登录后回到原页**）。
- **Quickstart 是"一页走完"**：约 15 个编号 H3，从 `Install the Stripe Node library` 到 `Run the application`；同一段步骤**按语言在源码里堆叠多份**，切换是客户端过滤。 API 参考页槽位固定：可跑的 `curl` → 响应 JSON → 参数表（`mode (enum, required)` / `currency (enum, required conditionally)` /
  `locale (enum, optional)`）。
- **文档页有 `.md` 孪生体**，且 quickstart 的 `.md` 开头有一段**专门写给 coding agent 的指令**：`npm i -g @stripe/cli` → `stripe sandbox create --help`（"无需注册账号"的沙箱）。
- **代价（不要抄）**：`docs.stripe.com/` 单页 **1,290,605 字节 HTML，内联 `<script>` 1,111,095 字节（86%）， 外部脚本 0 个**，可读正文 **3,383 字符**——**标记比 381×**。抄它的架构，绝不抄它的交付。

## go.dev ★★★ 落地页嵌真编辑器（我们最该抄的一条）

- **`go.dev/` 首屏之后紧跟一个真编辑器**（`Try Go`）：可编辑的 `package main` / `fmt.Println("Hello, 世界")`、`Run` 按钮、输出面板显示 `Hello, World!`， 外加**一个预载 8 个程序的下来单**：`Hello World` / `Conway's Game of Life` / `Fibonacci Closure` / `Peano Integers` / `Concurrent pi`
  / `Concurrent Prime Sieve` / `Peg Solitaire Solver` / `Tree Comparison`； 还有 `Press Esc to move out of the editor`（**键盘可达性写进 UI**）与一个 `Tour` 链接。
- **关键：它是同源的（`/play`）**，所以"我跑通了 hello world" = **0 次点击 + 0 次安装**。 对照 `lean-lang.org`（跨域 `live.lean-lang.org/?from=lean`）—— **Go 把体验放在自己域内，Lean 把体验送走。这就是"产品站"与"广告单"的分界。**
- **预载 8 个程序**是"前 30 秒"的正解：游客不用想写什么，**从下拉里挑一个就跑**。
- hero 的副标题是**四条 bullet 而不是一句话**（`An open-source programming language supported by Google` / `Easy to learn and great for teams` / `Built-in concurrency and a robust standard library` / `Large ecosystem of partners, communities, and
  tools`）；两个 CTA `Get Started` / `Download`； 下载块紧跟 hero——**"试"和"装"挨着，先试后装**。
- `go.dev/doc/` 是**一整页扁平 H2 长页**（`Getting Started` / `References` / `Talks` / `Codewalks` / `Wiki` / `Non-English Documentation` …），**无侧栏、无右栏 TOC、无版本选择器**—— 靠"按体裁分组 + 浏览器查找"代替导航，对大目录有效但**不可扩展**。
- 导航下拉**每一项带一句说明**（`Go Spec — The official Go language specification`、 `Release Notes — Learn what's new in each Go release`），并有键盘提示 `Press Enter to activate/deactivate dropdown`——**下拉当带注释的站点地图用**。
- Tour of Go 是 AngularJS SPA（`ng-app="tour"` + `ng-view` + `ng-cloak`），壳仅 2,566 字节； 有 `/tour/list` 全课列表；目录是**滑出面板**（`table-of-contents` 指令）—— **目录不常驻，把宽度让给"讲解 + 代码"**。主题用 **cookie**（非 localStorage）+ CSS 前内联脚本挂 `<html data-theme>`；三图标切换各带
  `alt`。**`viewport` 带 `user-scalable=no` ——禁止双指缩放，可访问性反例。**

## bun.sh ★ 首屏就给安装命令

- 首屏顺序：公告条（可关）→ 导航 `Docs / Guides / Reference / Blog` + **页头搜索框** + `Install` 按钮 → hero `Bun is a fast JavaScript runtime & toolkit. All in one.` → 一段话 → **安装命令直接印在首屏**：`curl -fsSL https://bun.sh/install | bash`（macOS/Linux 与 Windows 两
  tab）+ `View install script ↗` + `Then follow the quickstart`。
- **版本号进 CTA**：按钮写 `Install Bun v1.4.2`，旁边一枚 `NEW Bun v1.4.2 released →` 徽章。**版本不是一个角落里的数字，是按钮的一部分。**
- 代码示例用**标签行切换场景**（`bun install` / `Express` / `Postgres` / `WebSockets`）—— 同一段"怎么用"覆盖四个场景，不写四页。
- **基准测试是动画不是图**：`warm cache · seconds (lower is better)` + `replay in real time` 开关 + 两条会跑的条（`bun v1.4 → 0.21s` vs `yarn v1.22.22 → 1.76s`）， 并附**方法论脚注**（`T3-stack app, 25 direct dependencies, ~220 packages … medians of 3`） 与 `reproduce`
  链接——**"replay"让访客自己重放，比 GIF 诚实，且可复现**。
- 缺点：**261,880 字节 HTML**，对"首屏给命令"这件事来说偏重。

## tailwindcss.com/docs ★ 三栏布局的工程细节可直接抄

- **版本号是页头第一个元素**（`v4.3`，出现在 logo/导航之前，但**链不到任何地方**）； 搜索快捷键 `⌘K` / `Ctrl K` 也印在页头。主布局是一个 grid：`grid-cols-[var(--container-2xs)_2.5rem_minmax(0,1fr)_2.5rem]` = **侧栏 + 2.5rem 沟 + 正文 + 2.5rem 沟**； 侧栏 `sticky top-14.25 bottom-0 h-full
  max-h-[calc(100dvh-…)] overflow-y-auto p-6` ——**侧栏独立滚动、不跟正文跑**。
- 侧栏分组用大写等宽小标题 + `<ul class="… border-l …">` 导轨；条目 `-ml-px` + `border-l border-transparent`，选中项**盖在导轨上**——零 JS 的选中态。**`data-autoscroll="true"`：侧栏自动滚到当前项——长侧栏的必需品，不是加分项。**
- 有面包屑（编号式：`1. Core concepts / 2. Dark mode`）；`data-tab` × 18 做安装方式切换。**`/docs/installation` 直接重定向到 `/docs/installation/using-vite`**—— 主 CTA 落到某一个具体框架配方上（对新手是"默认替你选了"）。
- **暗色模式不是反色**：`dark:border-[color-mix(in_oklab,var(--color-gray-950),white_20%)]` ——暗色边框由浅色 token **算出来**，不硬编码灰。**这是"暗色 ≠ 反相"的正确做法。**
- 落地页 hero 的"代码样例"是**源码 + 渲染结果并排的活组件**，不是截图； 只有一个 CTA 标签 `Get started`（在 hero 顶部与底部各渲染一次）；**hero 无安装命令**。

## svelte.dev ★★ 文档首页是「人格路由器」

- **`/docs` 先问"你是谁"，再给导航**——六张卡：`I'm brand new here` → `/tutorial`、 `I'm migrating an app from Svelte 4` → `v5-migration-guide`、 **`I just want to try it out` → `/playground`**、**`I'm a Large Language Model (LLM)` → `/docs/llms`**、 `I'm
  looking for the old docs` → `v4.svelte.dev`、`Help! I'm stuck` → `/chat`。**在任何目录出现之前按人格分流**，是最省读者力气的做法（我们照做：学生 / 教师 / agent 三条路）。
- **两级导航**：产品 tab（`Svelte` / `SvelteKit` / `CLI` / **`AI`**）+ 资源行 （`Tutorial` / `Packages` / `Playground` / `Blog`）。**`Tutorial` 排在 `Playground` 之前**—— 先教学、后沙盒，顺序本身就是主张；**`AI` 是文档的一等产品**。
- `/repl` 的侧栏是**教程大纲**（`Create new` / `Introduction` / `Hello world` / `Dynamic attributes` / `Styling` / `Nested components` / `HTML tags` / `Reactivity` / … / `Logic`）—— **把 REPL 和教程合成一个界面**：左边课程，右边编辑器。
- Getting started 只有四行 shell，紧接着就推向交互教程；Vite 替代方案降级到后面—— **主路径只留一条**。旧版文档用**独立域名**保活（`v4.svelte.dev`），不在导航里塞版本选择器。
- 缺点：hero 只有一个 `get started`，但**没有代码、没有安装命令、没有版本号**； hero 之后几乎全是营销（`used by companies you've heard of` → 社区 → 贡献者墙 → `Backed by Vercel`）。**已有品牌可以这么赌，新项目这么赌等于没人知道你是什么。**

## linear.app ★ 首屏就是产品本体（不是截图）

- **h1 在 DOM 里出现三次**（`The product development system for teams and agents` ×3）——响应式/滚动动画用的多副本技巧，一次渲染三种宽度，避免 JS 测量。
- 首屏不是图，是**用真按钮搭出来的产品 UI**：`switchWorkspaceButton`（`Linear`）+ `navItem` 按钮（`Pulse` / `Inbox` / `My issues` / `Reviews` / `Workspace` / `Initiatives` / `Projects` / `More` / `Favorites`）+ 一张真渲染的 issue 卡（`### Faster app launch`、`vehicle_state`、标签 `Performance` / `iOS`）。**游客可以点着走一遍产品骨架——这是"活"的最直接来源，且不需要后端**（全是前端状态）。
- 导航：`Product▾ / Resources▾ / Customers / Pricing / Now / Contact / Docs / Open app / Log in / Sign up`——**`Docs` 是一级项**，`Now` 把 changelog 当产品做。每个 section 的出口都写死（`Learn more→ → /intake`），`ingredientButton` 是可展开的**成分芯片**（`Linear Agent +` / `Customer Requests +` / `Triage +` / `Linear Asks +`）。
- **文档是 2 栏**：左栏 **17 个顶层分组**（`Getting started` / `Account` / `AI` / `Your sidebar` / `Teams` / `Issues` / `Issue properties` / `Projects` / `Initiatives` / `Cycles` / `Views` / `Find and filter` / `Linear Asks` / `Integrations` / `Analytics` / `Administration` / `Importers`），正文是 **`Popular` 4 卡网格**（`Start Guide` / `Import Issues` / `Projects` / `GitHub Automations`）+ **`Linear basics` 8 卡网格**。**无版本选择器、无右栏 TOC。**
- **持久顶部 tab 条把产品切成独立产品**：`Docs` / `Developers` / `Learn` / `Contact support`，正文列顶部有 `1. Home` 面包屑。`/developers` 按 **API 面**分组（`GraphQL API` 8 页 / `Authentication` 4 / `Agents` 5——含 `Agent Interaction Guidelines (AIG)` / `TypeScript SDK` 7 / `Guides` 3），每行带自己的内联 SVG 图标。
- **发布状态放在营销路由上**（`/changelog`，隶属 `Now` hub，tab：`All` / `Changelog` / `Product launches` / `From the team` / `From the community` / `Press`）；条目按日期（`September 14, 2026`）+ 按产品区分组的 `Fixes` / `Improvements`——**完全没有版本号**。
- 缺点：**1,295,797 字节 HTML / 10,302 字符正文 = 126×**，182 个 `<script>`、40 个外部 chunk、326 KB 内联脚本——营销页的重量级天花板。

## vercel.com — 导航反例：按组织架构分类

- `Products` 下拉 **19 项**（`AI SDK` / `AI Gateway` / `Sandbox` / `Passport` / `Connect` / `eve` / `Security` / `Content Delivery` / `Fluid Compute` / `Observability` / `Workflows` / `CI/CD` / `Next.js` / `Vercel Agent` / `Vercel Plugin` /
  `Open Source` / `Domains↗` / `v0↗`） ——**读者不可能在这里做出选择**。
- `Resources` 下拉 **17 项，混了四套分类法**：文档类 / 用例类 / 生态类 / 商务类。**一个下拉不该同时承担四种意图。** 品牌资产按钮（`Copy Wordmark` / `Download Brand Assets` / `Brand Guidelines`）也挤在主导航流里。
- 好的一面：有 `Skip to content → #geist-skip-nav`；**每页有三份文档**——HTML + **`.md` 孪生体**（YAML front-matter：`title` / `canonical_url` / **`last_updated: 2026-08-11`** / `type: how-to` / **`prerequisites: []`** / `related` / `summary`）+
  **`<slug>.graph.md`**（`Semantically closest pages` / `This page links to (17)` / `Pages that link here (7)`）， 整张图发布成 `/docs/graph.json`——**prev/next 被"计算出来的链接图"取代**。 我们的 import 闭包与 `prereqs` 已经是图，**可以生成同构的 `graph.json`**。
- getting-started 页是 **agent 优先**：先给 `Agent prompt` 代码块 （`npm i -g vercel` → `vercel login` → `npx plugins add vercel/vercel-plugin` 或 `npx skills add vercel-labs/agent-skills`），**人的路径排在后面**。
- 头部三个 tab（`Build` / `Learn` / `Getting Started`）+ `Search Docs ⌘ K` + **`Ask AI`**—— **搜索与 LLM 助手是平级页头能力**。
- **526,419 字节 HTML / 3,693 字符正文 = 143×**；66 个 `<script>`、260 KB 内联脚本。

## deno.com / docs.deno.com ★★ 版本频道与 agent 通道

- hero：`Better, faster JavaScript` + **带竞争性对比**的副标题（`Faster than Node; more stable than Bun.`）； hero 下方紧跟安装块 **`Install Deno 2.9.7`** → `curl -fsSL https://deno.land/install.sh | sh` （macOS/Linux 与 Windows 两 tab）+ `Release notes` 链接 + 明确的
  `Copy command` 按钮。
- **紧跟安装块的是"给 agent 的活"**：*"Or let your agent do it — Paste this into any coding agent: Read deno.com/agents.md and set up Deno in this project"*，并列出 `Claude Code · Codex · Gemini CLI · OpenCode · Cursor · Pi`。**我们有 `site/agents.html` 与
  `docs/design/deepseek-harness.md`——这条我们天然领先，应做成一等入口。**
- **文档页有 agent 工具条**：`Copy page` / `View Page as Markdown`（`/runtime/index.md`）/ **`Open in Claude`**（`claude.ai/new?q=…` 深链）/ `llms.txt`。
- **稳定性是一整页而不是徽章**：12 周一个 minor；四频道 `stable`/`lts`/`rc`/`canary` 各有自己的 latest； 明写规则 **"A version number does not, by itself, identify a channel"**（`2.9.3` 在 stable 与 LTS 上 同时存在但字节不同）；附 LTS 维护起止表与 `deno upgrade <channel>`。不稳定 API 用
  `--unstable-*` 门控。
- 产品家族用**双列 mega-menu** 分开：`Open Source`（Deno / Fresh / Claw Patrol / JSR）与 `Commercial`（Deno Deploy / Sandbox / Subhosting / Enterprise）——**开源与商业分栏**。
- 文档结构：4 tab 顶条 + 9 组左栏 + **真右栏 `On this page` 锚点 TOC**。**上手路径全展示且每条命令配真实输出**：`curl … | sh` → `deno --version` → `deno init my_project` （附创建的完整目录树）→ `deno -N main.ts`（附 `Listening on http://localhost:8000/`）→ `deno test`（附通过/失败输出）→ `deno
  install express` → `deno fmt` / `deno lint` / `deno task`。

## astro.build

- **文档是 4 个 tab 各带一套完全不同的侧栏**：`Tutorial` / `Guide` / `Reference` / `Ecosystem`—— **同一个产品的四种读法**（学/做/查/生态），而不是一棵树硬塞。`Reference` 按 API 面分组， 且含 **`Error reference`**。
- **升级/迁移是可浏览的产品面，不是 changelog**：`Upgrade Astro` 按大版本给指南（v7.0…v1.0）；`Migrate to Astro` 给**每个来源框架一页**（CRA / Docusaurus / Eleventy / Gatsby / GitBook / Gridsome / Hugo / Jekyll / Next.js / NuxtJS / Pelican / SvelteKit / VuePress /
  WordPress）。 → **"你从哪来"是比"你现在在哪"更好的导航维度**（对我们：从 Lean 4 来 / 从 Coq 来）。
- hero 一个 CTA `Get Started` + **紧跟一个复制芯片 `npm create astro@latest`**（点后显示 `Copied!`）； 版本是顶部横幅链接（`Astro 7.3 — Available now!`）而不是徽章。
- i18n 是**页头下拉 13 种语言**且**语言是 URL 路径段**（`/en/…`）——13 个稳定 URL，不是客户端切换； 主题是三态 `Select theme: Dark | Light | Auto`；**落地页两者都没有**。
- 缺点：一级导航下平铺 **12 项**（`Themes` / `Integrations` / `Site showcase` / `Playground` / `Tutorials` / `Community` / `Discord` / `Sponsors` / `Merch` / `Enterprise` / `Agencies New` / `Case studies`），**没有分组**，`Playground`
  混在其中且**在另一个子域**（`play.astro.build`）。322,196 字节。

## gleam.run ★ 把错误信息当卖点展示

- hero：一句话 + `Try Gleam` 按钮 + **三行真代码**（`import gleam/io` / `pub fn main() { io.println("hello, friend!") }`）。**首屏有代码，且短到能一眼读完。**
- 每个 section 都用**真实终端记录**当证据：`➜ (main) gleam add gleam_json` → `Resolving versions` → `Downloading packages` → `Downloaded 2 packages in 0.01s` → `➜ (main) gleam test` → `Compiling thoas` / `Compiled in 1.67s` → `1 tests, 0
  failures`。**带提示符、带耗时、带成功行。**
- 它主张"clear error messages"，**同一屏就把编译器报错贴出来**：`error: Unknown record field ┌─ ./src/app.gleam:8:16 │ 8 │ user.alias │ ^^^^^^ Did you mean 'name'?` → **主张与证据同屏**，这是"营销不空洞"的机械做法。
- **Cheatsheet 按"你原来写什么语言"分**：Gleam for **Elixir / Elm / Erlang / PHP / Python / Rust** users ——**做的是概念翻译，不是文本翻译**。我们的对应物是**"给 Lean 4 用户 / 给 Coq 用户"的对照表**。
- 文档首页**没有侧栏**，是人工策展的 hub；`Unofficial courses` 里对 Exercism 明确标注 *"This is a referral link and a portion of any money paid will go to supporting Gleam development"* ——**利益关系公开标注**。
- 落地页长到**自嘲**：结尾写 `## You're still here? Well, that's all this page has to say. Maybe you should go read the language tour!`——**唯一一个承认自己落地页太长的站**。

## www.rust-lang.org + doc.rust-lang.org ★ 安装说明的教科书

- hero：`Rust` / `A language empowering everyone to build reliable and efficient software.` + `Get Started` 按钮 + **按钮正下方 `Version 1.98.1`**（链到 release 博客）；顶部导航带 **11 种语言**。 落地页几乎全是营销/导航，**唯一的信息载荷就是版本号与领域链接**。
- **book 的 ch01-01 是"安装页该长什么样"的答案**：命令在正文里 （`$ curl --proto '=https' --tlsv1.2 https://sh.rustup.rs -sSf | sh`），紧跟预期输出 （`Rust is installed now. Great!`），再跟排障（linker / `xcode-select --install` / `build-essential`）， 并给替代路径（`Other Rust
  Installation Methods`）。
- **`Command Line Notation` 说明框**：明写"`$` 是提示符，不用输入；不以 `$` 开头的行是输出"。**这是新手第一次复制粘贴失败的头号原因，值得直接搬。**
- **Book 在正文里钉自己的工具链基线**（而非版本选择器）：*"This version of the text assumes you're using Rust 1.90.0 (released 2025-09-18) or later with `edition = "2024"`"*；支持离线（`rustup doc --book`）。
- book 有 **6 套主题**（`Auto / Light / Rust / Coal / Navy / Ayu`）+ 快捷键帮助浮层 （`Press ← or → to navigate between chapters` / `S` 或 `/` 搜索 / `?` 帮助）。
- `doc.rust-lang.org/std/` 头部三件套：**`Crate std` + `std 1.98.1` + `(48a229cea 2026-09-01)`** = 版本 + commit + 日期；第一节是 `How to read this documentation`——**先教怎么读文档**。
- **稳定性是行内徽章**（`Experimental`：`f16`/`f128`/`never`/`simd`/`random`…；`Deprecation planned`：冗余的 `i8`…`usize` 常量模块）；每项带 `Source` 链接； 还有 **`Summary` 按钮**把散文折叠成可扫读视图——**同一页两种密度**。

## doc.rust-lang.org/rust-by-example ★★ 每页一个可运行例子 + 一个 Activity

- 第一页的**代码注释就是页面教程**：`// You can test this code by clicking the "Run" button over there ->` / `// or if you prefer to use your keyboard, you can use the "Ctrl + Enter" shortcut.` / `// This code is editable, feel free to hack it!` →
  **用语言本身的注释教读者怎么用这个页面**，零额外 UI。
- 每页结尾有 **`Activity`**：`Click 'Run' above to see the expected output. Next, add a new line with a second println! macro so that the output shows: …` → **读 → 跑 → 改 → 对答案**，闭环四步，且"对答案"是具体字符串而不是"自己体会"。
- 正文里给**期望输出**（`$ rustc hello.rs` → `$ ./hello` → `Hello World!`）—— 静态页也能让读者自己验，因为答案印在页上。

## docs.python.org/3/ + python.org ★ 版本生命周期标注

- `Docs by version` 把**每个版本连支持状态一起给**：`Python 3.16 (in development)` / `3.15 (pre-release)` / `3.14 (stable)` / `3.12 (security-fixes)` / `3.9 (EOL)`。 → **"哪个版本能用、哪个已经死了"写在版本号旁边**——版本呈现的最佳范式，我们要照做。
- **`www.python.org` 主页没有营销 h1**（`<title>` 是 `Welcome to Python.org`）：开头是导航 + 搜索 + **一个 `>_ Launch Interactive Shell` 链接（→ `/shell/`）**，定位句反而出现在后面。**"试"排在"讲"之前。**
- **hero 是 5 张 REPL 记录的轮播**，每张把 `>>>` 会话、一个好处标题、一个文档深链绑在一起 （`Simple arithmetic`：`>>> 1 / 2` → `0.5`；`Functions Defined`：fib；…）， 每张结尾 `More about … in Python 3` 指向 `docs.python.org/3/tutorial/…` → **落地页同时在教学和路由**——又一个"用预计算内容做交互感"的先例。
- 四张卡就是全部起步路径：`Get Started` / `Download`（**`Latest: Python 3.14.7`**）/ `Docs` / `Jobs`；**落地页没有安装命令**，Getting Started 页也只有散文，**主动把安装甩给 wiki**。
- `Theme Auto Light Dark` 三态切换**在侧栏顶部与页脚各放一次**；`What's new in Python 3.14?` 是文档索引第一条。主页仅 **11,521 字节**——**大项目也能有轻首页**。

## swift.org

- 导航：`Docs / Community / Packages / Blog` + **`Install (6.4.0)`**——**版本号直接写进导航项的文案里**。
- hero：`Swift is the powerful, flexible, multiplatform programming language.` / `Fast. Expressive. Safe.` / `Install` 按钮 / `Tools for Linux, macOS, and Windows`。**无代码、无安装命令**；整页仅 8,851 字节。
- **技术内容是一个五面板 carousel，每个主张都配真代码**：`Fast` （`isASCII(utf8: Span<SIMD16<UInt8>>)`）、`Expressive`（`@main struct Describe: ParsableCommand`）、 `Safe`（`Affine2DTransformBuilder` + `mat_vec_mul`）、`Interoperable`（`import CxxStdlib` 后直接调 C++ 类型）、
  `Adaptable`（MMIO 寄存器写 `usart1.brr.modify { … }`）。**样例本身就是论据。**
- `Create using Swift` 卡片按**产出物**分（`Cloud Services` / `Command Line` / `Embedded` / `iOS apps` / `Windows apps` / `Machine Learning & AI`），每卡一句动词开头的说明。
- 安装路径最长：landing → `/install/`（**只有一个三选一的 OS 分叉，别无他物**）→ OS → 命令。
- 主题是页脚**文字标签**控制（`Color scheme preference: Light | Dark | Auto`），不是图标。
- 注：首次 curl 返回**二进制乱码**（服务端压缩未协商），加 `--compressed` 后正常——记录以备复现。

## ziglang.org ★ 版本进按钮 + 诚实的多语言标注

- **没有 h1 hero、没有 CTA 按钮**：开篇一句话 + `GET STARTED` + 版本作纯文本 `Latest Release: 0.16.0`，旁边两个链接 `Documentation`（**版本钉死的 URL** `/documentation/0.16.0/`）与 `Changes`（release notes）。**点之前就知道会拿到什么。**
- section 是**三个主张 + 各 3–4 条一行式要点**（`⚡ A Simple Language`：`No hidden control flow.` / `No hidden memory allocations.` / `No preprocessor, no macros.`；`⚡ Comptime`；`⚡ Maintain it with Zig`） ——**一行一条，可扫读、可反驳**，比形容词堆砌有用。
- 主张之后给**真代码 + 真输出**：`index.zig` + `$ zig test index.zig` → `1/1 index.test.parse integers...OK` → `All 1 tests passed.`；代码注释里写 **`// Try commenting out this line!`**——**邀请读者动手**。
- **每个样本都配"确切命令行 + 真实输出"，包括故意的编译失败**：文件名标注 （`constant_identifier_cannot_change.zig` / `invalid_doc-comment.zig`…）+ 带插入符的诊断 （`error: cannot assign to constant`）+ `2 reference(s) hidden; use '-freference-trace=6'` +
  失败测试的种子（`--seed=0xa10f955d`）。**页面上什么都不跑，但页面上没有一句未经验证的话。**
- **语言参考刻意做成单页，并把理由写在页首**：*"It is all on one page so you can search with your browser's search tool"*、*"The code samples in this document are compiled and tested as part of the main test suite of Zig"*、*"This HTML document depends on no
  external files, so you can use it offline."*
- 安装**不在落地页**：`GET STARTED` → `/learn/getting-started/`，先问 "Tagged release or development build?"，再给直下（含完整 PATH 片段）/ 包管理器 （`winget` / `choco` / `scoop` / `brew` / `port`）/ 源码编译三路。
- 页脚多语言列表写 **`English (original)`**：**明确标出哪个是原文**，翻译过期时读者知道该信谁。

## www.roc-lang.org ★★★ 与我们约束最接近的完整先例

- 落地页 HTML **25,317 字节**（对比 Stripe 1.29 MB / Linear 1.30 MB），**手写无构建** （属性不加引号、`<p>` 不闭合），外部引用只有 `/site.css`、`/compiler.js`、`/repl/roc_repl_wasm.js`。
- **首屏嵌真编译器**：`<div class="roc-interactive front-page"><pre class=roc-source>…源码…</pre> <button class=roc-run>Enable JS to Run</button></div>` → **源码在 `<pre>` 里（可索引、可复制、无 JS 也在）+ 一个按钮做渐进增强**； 按钮初始文案是 **`Enable JS to Run`**（JS 关时是诚实说明，JS 开时变
  `Run`）。**这是"功能页"在静态站上的最干净实现。**
- 落地页原话：`Browse examples built and tested with the same Roc compiler as this website.` + **`You can run the full compiler in the browser without a backend server!`** ——**把"零后端"当卖点写在页面上**。
- **例子页钉死可复现性**：`These examples are built and tested with Roc nightly-2026-09-18-1d982dc.` + `Their source comes from roc-lang/examples at e623ae7.` → **编译器版本 + 源码 commit 双钉**。 我们的 `site/data/site.json` 已在生成版本号，应同样把**示例源码的 commit** 写上去。
- 例子清单是**动词/概念命名**的 18 项（`Hello, World!` / `FizzBuzz` / `All Syntax in 1 File` / `Tuples` / `Pattern Matching on Lists` / `Error Handling Real World (Try)` / `? Operator Desugaring` / `Safe Math` …）。
- **每个形容词都带"这到底什么意思"的出口**：`Fast` → `What does fast mean here?` → `/fast`；`Friendly` → `/friendly`；`Functional` → `/functional`。**这是"去营销化"的机械手段：形容词必须能点开定义。** 而 `/fast` 页本身是工程文档： 它**自陈上限**（*"a lower performance ceiling than languages which
  support memory unsafety"*）、 **声明非目标**（*"it's a non-goal to outperform languages that support memory unsafety"*）、 并给出**构建延迟预算**（*"almost always complete in under 1 second on the median computer"*）。
- 导航顺序 `Tutorial / Install / Examples / Community / Docs / Donate`——**Tutorial 第一，Install 第二**。 安装页第一行就是 **`Roc is a Work in Progress!`**——**成熟度不藏在角落**。
- 诚实：`To get started with the language, try the tutorial!` 直接链 GitHub 上的 markdown—— **没做教程页面就不假装有**。

## elm-lang.org / elixir-lang.org / unison-lang.org / koka-lang.github.io

- **Elm**：落地页 `<title>` 写成 **`home`**——标签页与搜索结果显示为 "home"，**丢失唯一的品牌位**（反例）。 但 guide 有两招值得抄：(a) **代码先于解释**——引言两句后立刻给一个完整的 20 行 `Browser.sandbox` 加减计数器程序，并链到活编辑器，然后才承认 *"The code can definitely look unfamiliar at first, so we will get into
  how this example works soon!"*； (b) **用四条可证伪的保证代替形容词**（`No runtime errors in practice.` / `Friendly error messages.` / `Reliable refactoring.` / `Automatically enforced semantic versioning for all Elm packages`）。
- **Elixir**：hero 的代码**配着它的输出**——`"Fun to learn, delightful to ship" |> String.downcase() |> … |> Enum.frequencies()` → `%{"delightful" => 1, "fun" => 1, "learn" => 1, "ship" => 1, "to" => 2}`。 安装页是 **7 个编号小节 + 对应锚点条**，并**显式声明依赖**：`Elixir
  v1.20.4 requires Erlang 27.0 or later`。文档用 ExDoc：版本 chip `v1.20.4` + **`Copy Markdown`** + **`View Source`（链到 release tag 的 GitHub blob）** + `View llms.txt` + `Download ePub version`——**机器可读与离线都是一等公民**。
- **Unison**：每个主张（`Refactoring` / `Durable storage` / `Storing code` / `Dependencies`）都是 **一句主张 + 一个 `Learn More`**——主张与证据一一配对。代码样例自带 **`💡 All parts of the code examples are interactive. Click a dependency to read its definition and
  docs.`** （依赖名链进 `share.unison-lang.org`）——**"可点开依赖"是零后端的交互**。 Quickstart 用**时间盒**：`⏳ Three-minute quickstart guide`。 缺点：导航区塞 newsletter 表单（`Get regular updates from the Unison team`）——**首屏抢注意力**。
- **Koka** ★★★：文档首页开篇就是完整代码 + 带类型签名的输出，而**排版本身就是教学装置**—— 同一段代码**渲染两遍**：一遍普通源码，一遍**每个 token 内联标注推导出的类型与效果** （`traverse : forall<a> (xs : list<a>) -> (yield<a>) ()`、`main : () -> console ()`、 handler 的 `(() -> <console,yield<int>> ()) ->
  console ()`），**每个被标注的名字都链进 stdlib 页面**。 → **对我们的直接启示：每个 tactic 都可以标注出它在这一步"推出来的目标与假设"， 并链到该 tactic 的说明页——这正是 `query state` 已有的数据，只是没人渲染它。**
- **Koka 的成熟度声明**同样值得抄：`Note: Koka v3 is a research language that is currently under development and not ready for production use. Nevertheless, the language is stable and the compiler implements the full specification. The main things
  lacking at the moment are (async) libraries and package management.` → **"不能用在哪 + 缺什么"写清楚**，比含糊的 "beta" 强得多。

# Group C — 浏览器内游乐场 / 交互式文档

> 这一组只回答一个问题：**"活着"和"死了"的机制差别到底是什么。**
> 每站标注**执行底座**（服务器 / WASM / 预计算 / 纯前端），因为底座决定我们能不能抄。

## roc-lang.org（`/compiler.js` + `/repl/roc_repl_wasm.js`）★★★ 底座 = WASM，形态 = 渐进增强

- **交互回路**：改 `<pre class=roc-source>` 里的代码 → 点 `Run` → 输出出现在同一容器。
- **底座证据**：页面引用 `/compiler.js` 与 `/repl/roc_repl_wasm.js`，文案 `You can run the full compiler in the browser without a backend server!` → **WASM，零后端**。 （诚实标注：`compiler.js` 用 `HEAD` 返回 0 字节、`compiler.wasm` 404——**我没能测出编译器实际体积**。
  抄它的**形态**没有风险，但**不要低估 WASM 首包**。）
- **前 30 秒**：源码已**印在 HTML 里**（无 JS 也能读、能复制、能被索引），按钮文案在无 JS 时是 `Enable JS to Run`——**不是空白编辑器，而是"代码 + 一个诚实的按钮"**。
- **它为什么"活"**：因为静态页面上**真的有一段可执行的程序**，且作者把"能在浏览器里跑完整编译器" 写成了页面主张。这比任何动画都有说服力。
- **我们能不能抄**：**形态能抄满，底座不能抄**（我们内核是 Rust，不打算编 WASM 到前端）。

## Rust Playground（`play.rust-lang.org`）

- **底座 = 服务器**：meta description 自述 `A browser interface to the Rust compiler`。
- **壳极轻**：HTML 仅 **905 字节**，只有 2 个 JS chunk + 1 个 CSS；但其中一个 chunk 实测 **718,453 字节**——**首屏要等 ~700 KB JS 才出现编辑器**。
- **无 JS 回退只有一句**：`The Rust Playground requires JavaScript to be enabled.` ——诚实但零信息（不告诉你这是什么、不给替代路径）。
- 值得抄的**模式名**：编辑 → 运行（`Ctrl+Enter`）→ 下方输出面板；permalink 分享； 多档"运行"（build / test / asm / IR）——**同一个编辑器，多个问题**。
- 诚实标注：默认片段在客户端渲染，我在 718 KB 的 chunk 里没 grep 到 `Hello, world`，**未能确认其内容**。

## Go Playground（`go.dev/play/`）★★ 默认片段在 HTML 里 + 假时间回放

- **底座 = 服务器**，但**底座本身写在页面上**：*"The service receives a Go program, vets, compiles, links, and runs the program inside a sandbox, then returns the output"*； 时钟冻结在 `2009-11-10 23:00:00 UTC` 以保证输出可缓存；有 CPU/内存/时间上限。**把"我怎么跑你的代码、有什么限制"写在编辑器下面，是信任的最低成本来源。**
- **完全服务端渲染、可读**：chrome 就是 `Go 1.27 | Go 1.26 | Go dev branch | Run`、`Format` `Share`、 一个**14 项示例下拉**（`Hello, World!` / `Conway's Game of Life` / `Fibonacci Closure` / `Concurrent Prime Sieve` / `HTTP Server` / `Display Image` / `Multiple
  Files` / `Sleep` / `Test Function` …） 与 `Press Esc to move out of the editor.`
- **前 30 秒 = 编辑器里已有一个能跑的程序**：`// You can edit this code! // Click here and start typing.` + `package main` + `fmt.Println("Hello, 世界")`，配可见的 `Run` 按钮与一键示例库。**无 JS 也能看到"这个页面是干什么的"**（实测 HTML 里直接有 `package main` 与 `Hello, 世界")`）。
- **★★ "假时间回放"机制（直接印证我们的 T5）**：后端发出 playback 头 （`\x00\x00PB<time><len>`），前端拿到带 `Delay` 的 JSON `Events`，JS 客户端**按正确时序重放写入**—— 于是 `time.Sleep` 看起来是真的，却不用真的等（`go.dev/blog/playground` 记录； 该文自带免责声明 *"This article does not describe the current
  version"*）。**我们的 `--json` 事件流天然就是这个形状**：每行一个事件 + 相对时间戳 → 按时间戳逐行揭示。
- 分享是**服务端存 ID**（`/play/p/<id>`，页面先渲染 `Loading share...`），不是 URL 编码代码—— **代价是分享链接会随服务消失**（对照 `tryhaskell.org` 的结局）。
- **对我们的意义**：**"默认示例必须出现在 HTML 里"**——"关掉 JS 也有信息"的底线，成本为零。

## TypeScript Playground（`typescriptlang.org/play`）★ 零点击的类型标注

- 三个 URL 变体（`/play`、`/play/`、`/play/index.html`）对非 JS 抓取器**都返回空 body** ——整个产品在没有 JS 时不可见（与 Rust Playground 同病，比 Go Playground 差一档）。
- **底座 = 浏览器内编译器**（源码布局证据：`packages/playground-worker` 把编译器放进 web worker， 另有 `typescript-vfs` 虚拟多文件系统、`playground-examples` 示例包、`playground-handbook` 内置手册面板） ——**永远能跑，不依赖服务器**。
- **★ 活着的关键机制**：`twoslashInlays.ts` + `ts-twoslash` **在你打字时把推导出的类型内联渲染出来** ——**没有 Run 步骤的反馈**。这与 Koka 的"逐 token 标注推导类型"是同一件事， 但 TS 把它做成了实时的。
- **对我们的意义（重要）**：我们的"类型"是**每一步的目标与假设**。 因为目标是**预计算**的，我们可以做到比 TS 更彻底：**光标停在任意一行，那一行的目标与假设直接内联显示** ——不需要编译器、不需要 worker、不需要等待。**这是"零点击"在零后端约束下唯一可行的形态。**
- 其他旁挂视角：`.d.ts` 输出、AST 查看器、编译错误列表、TS→JS 对照。**同一个输入，多种视角——教学价值全在视角上。**

## Svelte REPL（`svelte.dev/repl` → `/playground/hello-world`）★ 永远不给人空白编辑器

- **`/repl` 301 到 `/playground/hello-world?show=input`**——**新访客永远落在"一个能跑的例子"上， 而不是空编辑器**；`Create new` 才去 `/playground/untitled`。
- 左侧栏是**事实上的课程**：**80+ 个具名、可深链的示例**，按概念分组 （`Hello world` / `Dynamic attributes` / `If blocks` / `Keyed each blocks` / `Tweened` / `Spring` / `Deferred transitions` / **7GUIs**：`Counter` / `Temperature` / `Flight booker` / `Timer` / `CRUD` /
  `Circle Drawer` / `Hacker News`）——**每个条目都是自己的 URL**。
- 独立教程（`/tutorial/svelte/welcome-to-svelte`）是引导序列，且带**明确逃生口**： *"If you get stuck, you can click the `solve` button in the top right of the screen."* （并注明"没有练习的小节里 solve 按钮是禁用的"），另有 `Toggle Vim mode`、 `show text` / `show editor` 布局开关、文件
  tab（`src` / `App.svelte`）。
- **两边都把教学文字服务端渲染**——我用纯抓取器读完了整个课程大纲。
- → 我们的功能页应该**左边是卷/单元/练习，右边是代码 + 目标状态**， 并且**默认落在第一个练习的第一步**，而不是一个空页面。

## Tailwind Play（`play.tailwindcss.com`）

- 服务端渲染的 chrome 很小但具体：`Share`、版本徽章 **`v4.3.3`**、 `Switch to vertical split layout / horizontal split layout / preview-only layout`、 `Toggle responsive design mode`——**没有任何 Run 按钮**。
- **版本徽章是廉价的信任信号**（你永远知道在跑哪个编译器）；**布局是一等控件**（3 种分栏 + 纯预览 + 专用响应式视口），而不是一个固定分栏。
- **交互回路是"零点击"的**：左边改 class → 右边预览**即时**变。**即时反馈本身就是回路**， 这是"活"的上限形态。
- **对我们的意义**：我们能做的最接近的"零点击"是**目标状态随光标/悬停即时切换**—— 因为目标状态是**预计算**的，切换就是显示/隐藏（纯 CSS `:hover` 即可），**比 Tailwind 还快**。
- 诚实标注：`Share` 暗示 URL 编码状态，**我没能从 HTML 验证其编码方案**。

## regex101 ★★ 空状态文案与"自动解释"

- **底座 = 服务器**（后端跑各语言的正则引擎，Flavor 可选 `PCRE2 (PHP)` 等）， 但它把**解释**做成了首屏主角。
- **空状态不空**，两句话告诉你将会发生什么：`An explanation of your regex will be automatically generated as you type.` / `Detailed match information will be displayed here automatically.` → **"这里将自动出现 X"是空状态文案的最佳写法**，我们所有功能页的初始态都该这么写。
- **多视角面板**：`Explanation` / `Match Information` / `Quick Reference` （可搜索的 token 速查，按 `All Tokens` / `Common Tokens` / `Anchors` / `Meta Sequences` / `Quantifiers` 分组）。
- **模式 tab**：`Match` / `Substitution` / `Extraction` / **`Unit Tests`** / `Pipeline`—— **允许给正则写单元测试**，把"试一试"升级成"钉住行为"。教学产品的高级形态。
- **`Regex Debugger`（带 New 徽章）**：**逐步执行**是"活"的终极机制；`Benchmark` / `Format` / `Code Generator` / `Export Matches` 都是旁挂工具。
- 首屏 logo 是 `/ ^ r e g e x [ \ - _ ] ? 1 0 1 $ / i`——**把产品名写成一个正则**，自指且立刻说明类别。

## Compiler Explorer（`godbolt.org`）

- 壳 118,192 字节；**底座 = 服务器**（几百个编译器在服务端）。
- **"前 30 秒"做成了模板面板**：`Templates` 里给 `Start` / `Previous` / `Next` → **新手不用自己想写什么，跟着模板点**。
- **两种 reset 分开**：`Reset UI layout` 与 `Reset code and UI layout`—— **"我把界面弄乱了"和"我把代码删了"是两件事**（细节但很有用）。
- 分享三档：`Share → Short Link / Full Link / **Embed in iframe**`—— **`Embed in iframe` 让第三方网站能嵌你的 playground**，是免费的传播渠道。
- 布局选项 `Compiler` / `Execution Only` / `Conformance View`——**按意图换布局，不是给拖拽自由度**。

## Flexbox Froggy（`flexboxfroggy.com`）★★ 10 KB 的完整教学游戏

- **总重 10,501 字节 HTML**，无框架（jQuery + 四个小文件：`js/levels.js` / `js/docs.js` / `js/messages.js` / `js/game.js`）——**全部关卡数据在 `js/levels.js` 里预计算**，零后端。
- **交互回路**：写 CSS → **即时**看到青蛙移动（浏览器本身就是求值器）→ `Next` 解锁下一关。 每关的"对/错"由**几何位置**判定，**不需要服务器**。
- **UI 只有四件**：`Level 1 of 1 ▾`（关卡下拉，可跳关）、`Reset`、`Next`、编辑器。**极简到没有多余按钮**——因为回路本身足够短。
- **难度分三档**：`Beginner` / `Intermediate - No Directions` / `Expert - No Directions & Random Levels` → **同一份关卡数据，三种脚手架强度**。这是"同一内容服务多种水平"的最省力做法。
- **`Colorblind Mode: Off/On`** 与 **约 40 种语言**（`English Español Français … עברית العربية فارسی हिंदी বাংলা`） ——**可访问性与多语言在 10 KB 里做到了**。出口：`Want to learn more CSS? Play Grid Garden or Anchoreum.`

## CSSBattle ★ 进度、二选一、实时感

- **首屏就有进度**：`0/4 done`（页头，永久可见）。
- **两个 CTA 之间明写 `OR`**：`Play Game🕹️` / **`OR`** / `Learn CSS🚀`—— **把"你属于哪一类"直接摆出来**，而不是并排两个按钮让用户猜差别。这是"5 个 CTA"问题的正解。
- **`0 Online`**：在线人数。**"有人和我一起"是廉价的活体信号**（我们可用"本页练习已被判过 N 次"替代）。
- **加载态有文案**：`Fetching Daily targets...` / `Loading public rooms...`——不转圈、不空白，说清在等什么。
- **判分是客观可自动化的**："the shorter your CSS, the higher your score"—— **游戏的核心指标必须是机器能算的**。我们的对应物是"内核是否接受 + 用了几个 tactic"。
- section：`Daily Targets`（**明确写 "No leaderboards, no competition"**，降低压力）、 `Public Versus Rooms`、`Latest CSS Battle`、`⛳ Code Golf`。

## Rustlings（`rustlings.rust-lang.org`）★ 用 asciinema 代替 GIF

- 站点是**静态 4,986 字节**，Quick start 四条命令直接印在首页（`cargo install rustlings` / `rustlings init` / `cd rustlings` / `rustlings`），代码块里用 `# Installation` / `# Initialization` 注释分段——**命令块自带说明**。
- **演示用 asciinema 录屏而不是 GIF**：`<script src="https://asciinema.org/a/719805.js" id="asciicast-719805" async="true">` → **文本型录制**：可选中、可复制、可搜索、体积极小、可暂停回放。**这是我们"没有后端"时替代 GIF 的最佳中间态**（比 GIF 诚实，比真交互便宜）。
- 页面明写定位：`Recommended in parallel to reading the official Rust book 📚️`—— **告诉你"什么时候用它"**，而不只是"它是什么"。
- 导航 `Rustlings / Setup / Usage / Community Exercises / Q&A`——**Setup 与 Usage 分开**，装和用是两件事。

## Glot.io（`glot.io/new/python`）★ 账号墙放晚 + 薄 chrome

- **前 30 秒 = 预填编辑器**：`print("Hello World!")`、`Snippet info` 显示 `Language: Python` 与 `main.py`，**只有两个按钮：`Run` 与 `Save`——没有注册墙**。
- **底座写在首页**：*"Code runs in a temporary container with network access disabled"*；页脚把 commit `fa6535c` 链到 `github.com/glotcode/glot`（开源）。**明说自己是什么、代码在哪。**
- 分享有**三档可见性**：*"Save public, unlisted, or secret snippets and share them when you choose"*，公开 snippet 可浏览。
- chrome 刻意做薄：`cmd+k` / `ctrl+k` 的 **`Quick actions` 面板**（*"Search actions and languages"*）+ **30+ 个按语言的入口**（`/new/assembly` … `/new/rust`）+ *"The editor supports Vim and Emacs key bindings."*
- 壳 195,970 字节 / 1,320 字符正文 = **148×**（chrome 重、正文轻的典型）。

## MDN 交互式示例 ★★ "文档即交互"的分层规范

- **底座 = 纯前端**。MDN 自己在写作规范里定义**三层**（`MDN/Writing_guidelines/Page_structures/Code_examples`）：**Static examples**（静态）→ **Live samples**（*"A macro takes code blocks from a page, combines them into an `<iframe>`, and embeds the iframe… source
  code blocks and the results side-by-side"*）→ **Interactive examples**（*"Readers can edit the source code and re-run the example"*，**每页一个**、固定高度、且 *"the UI for JavaScript is different from the UI for CSS"*）。 →
  **"可交互"是有层级的，不是二元的。我们可以只做第一层就够用。**
- **★★ 单一事实源**：live sample **从页面自己的 fenced code block 生成** （`{{EmbedLiveSample("id")}}` / `live-sample___<id>` info string）—— **源码与演示不可能漂移**。这直接对应我们 `docs/design/site.md` §3 的纪律：**功能页的演示必须由 `.sokonanoda` 源文件生成，不能手写第二份。**
- **降级是设计的一部分**：live sample 是 sandbox iframe，且**不提供 console 捕获与重置**； interactive examples 提供；归档的 `mdn/interactive-examples` README 明确写 **不支持的浏览器降级为静态示例**。
- **iframe 目标是独立页面**（`mdn.github.io/learning-area/…` 只有 `My test page` 两行， 没有 MDN chrome）——**可单独打开、可单独链接**。这正是我们 T8 要的形态。
- 实测 `Array.prototype.map` 页上是 `<pre class="brush: js interactive-example notranslate">`， 该页 **0 个 `<iframe>`**——**交互示例可以只是"普通代码块 + 一个 class"**，**改造成本最低的交互形态**，最容易做进现有 `site/` 而不新增页面类型。
- MDN 页 178,517 字节、**3,949 词服务端渲染**——交互只是文档的调味，不是全部。

## 期望值对比组件群（PGExercises / CSSBattle / SQLBolt / learn-c.org）★★

> 这一组共有一个**最高杠杆的可复用组件**：**把"期望结果"与"你的结果"并排放在一起**。
> 它不需要任何执行能力——**期望值是预计算的**，天生适配我们。

- **PGExercises**（`pgexercises.com/questions/basic/selectall.html`）：`Question` → **可折叠的 `Schema reminder`（做题期间保持展开）** → **`Expected Results` 表格** → `Your Answer`（带对勾/叉号图标）+ `Hint` `Help` `Save` `Run Query`； 答案藏在 `Answers and Discussion
  Show` 后面；有行内快捷键 （`Alt-h` 帮助 / `Alt-r` 运行 / `Alt-x` 运行选中 / `Alt-s` 按光标运行）； *"double click on each of the panes … to quickly resize them"*。 → **`Expected Results` 在 `Your Answer` 上方**：先给标准答案，再让你对。**顺序很关键。**
- **CSSBattle**（`cssbattle.dev/play/1`）：`Recreate this target / 400px x 300px` + **`Colors` 调色板（快捷键 `ctrl shift c`，色块 `#5d3a3a` / `#b5e0ba`）——值不需要猜**； 内置 **`Slide & Compare Diff`**（滑块式把你的渲染与目标对比）；`Your stats: Last score / High score` +
  `Last Submissions` 列表。**"滑块对比"是零后端的 diff。**
- **SQLBolt**（`sqlbolt.com/lesson/select_queries_introduction`）：编号任务清单 + `Stuck? Read this task's Solution.` + **硬门禁** `Solve all tasks to continue to the next lesson` （未完成时 `Finish above Tasks` 是禁用链接）。
- **learn-c.org**：**首页就嵌一个预填 hello-world 的活编辑器**，带 `Run` `Reset` `Solution`、 `Output` 面板与 **`Expected Output` 面板**，底部标注 `Powered by Sphere Engine ™`（**具名执行供应商**）。
- **对我们的意义（重要）**：我们的"期望结果"是**内核判定的结果**。 于是功能页可以这样组织：**左边给出该练习的"标准证明走法"（预计算的目标状态链）→ 中间给读者的候选写法 → 右边给出内核的真实诊断**。**全程零后端，因为"标准走法"与"典型错法"都是我们预先枚举的。**
- **VimHero**（`vim-hero.com/lessons/basic-movement`）：drill 有真仪表—— `Challenge / Mini-Game / Stats / Settings` 四个 tab、计时器 `00:00 restart`、模式徽章 `NORMAL`、 **手指图**（h=pointer, j=pointer, k=middle, l=ring）+ *"Make sure that you are using the
  correct fingers!"*； 侧栏**把每课要教的键直接印出来**（`hjkl` / `web` / `iaesc` / `dw` / `ci{` / `daw` / `Vdy`）+ `Review` / `Mega Review` 节点；`Next Ctrl+j` **键盘驱动**。 机制原话：*"Challenges are generated dynamically, allowing you to repeat them until you
  achieve mastery. Your proficiency in each skill is tracked so you always know where you stand"*；**`Free to start`，登录只在侧栏**（**账号墙放晚**）。 → **"侧栏印出每课教什么"比"第 3 课"强得多**；我们应印出**每个单元教哪些 tactic**。

## 失败与死亡（如实记录）

- **挡爬虫，导致无法分析**：`exercism.org`（`/`、`/docs`、`/tracks/rust`、练习页、 甚至 `/api/v2/tracks/rust` **全部 403 Cloudflare "Just a moment..."**）；`shadertoy.com`（`/`、`/browse`、`/howto`、`/view/…` 全 **403**）；`observablehq.com`（**429** "Vercel Security
  Checkpoint"）；`khanacademy.org`（HTTP 200 但 body 只有 `Khan Academy`；旧 API 路径 **410** "API removed"）；`nandgame.com`（**每条路径**都返回同一行 shell，连 `/sitemap.xml`、`/manifest.json`、`/app.js` 都返回 HTML shell——**只有 `/robots.txt` 是真内容**）；`go.dev/tour/*`（抓取器只拿到 header `A Tour of Go`，课程文字与编辑器全靠 JS）。 → **教训：挡爬虫 = 挡掉所有第三方拆解与 AI 引用；用 HTML shell 回答 `/sitemap.xml` 等于对爬虫交白卷。** 我们的站是静态的，没有这些问题，**应把它当卖点**。
- **SchemeWeb 已死**：`schemeweb.org` **DNS 解析失败**（getaddrinfo ENOTFOUND），`schemeweb.com` **跨域重定向到 `brandsly.com`**。两个域都没有可用的 SchemeWeb。
- **★ `tryhaskell.org`**：*"After 16 years of running, I've shut down tryhaskell.org. Sorry!"* ——**一个受人喜爱的浏览器内 REPL 死了，连同它所有的可分享状态一起消失。**
- **★ GLSL Sandbox**（`glslsandbox.com`）：gallery-first（缩略图网格 → `/e#<id>`）， 但挂着横幅 *"The server is in maintenance mode. You can not create or modify effects."* ——**能浏览、不能写，读起来就是死的**。 → 这两条合起来给出**功能页的托管纪律**：**状态必须在 URL 里或仓库里，不在服务器上**；**且要能只读降级**（我们的预计算产物天然满足这两条）。
- **Vim Adventures**：零设置钩子是 `Press any key to start!`，开局只有 `hjkl` 解锁； ~20 个具名音效（`Collect key` / `New skill` / `Blocked movement` / `Text completion` / `TaDa sound`）、 已收集键的虚拟键盘、`:set stats`，以及**硬性精通门禁** *"You can't advance to the next level
  until you master a skill!"*；拾取按键时**即时解释该键的作用**。 但它**$35 起（6 个月）/ $40（含限时 CTF）**、**canvas-only**、**直接拒绝移动端** （*"This game requires a keyboard. A physical keyboard."* 配一个 `I DO! (have a keyboard)` 逃生按钮）； 进度存在账号/游戏文件里（`:w` / `:login` /
  `:level <n>`），**不是 URL 状态**。
- **Replit** 已经**离开 edit→run 范式**：首页标题是 `AI App and Website Builder`， 文档导航是 `Chat / Build / Design / Use cases / Learn / Features`，journey 是 "Start a chat → Describe your idea → Integrations → Publish the site"； 上手是向导（Quickstarts + 按目标的
  Build paths），每页带 `Ask Assistant ⌘I`。 → **连"在线 IDE"都不再把"编辑-运行"当卖点**；我们做"编辑-判定"要更明确地说清它是什么。
- **StackBlitz** 把**底座当营销**：WebContainers = *"the first WebAssembly-based micro operating system which boots entire development environments in milliseconds… within your browser tab"*， 并给对比表（boot `Milliseconds` vs `Minutes`、`Zero network
  latency`、`Work offline`、 `Reset broken containers: Page refresh` vs `Not possible`）。**"浏览器内 vs 服务器"已经是一个需要公开声明的定位**（Glot 说 "temporary container with network access disabled"；Rust Playground 说 "no network connection between the compiler
  container and the outside world"）。**我们应明说：预计算、无后端、无网络、永远可用。**

## Group C 小结：什么让一个 playground「活着」

1. **回路越短越活**。Tailwind Play 是"改 → 即时变"（零点击，最活）；Flexbox Froggy 是
   "改 → 即时变 → Next"；Rust Playground 是"改 → 点 Run → 看输出"（差一档）；
   有"提交 → 等 → 排队"的是死的。
2. **预计算能买到"即时"**。Tailwind 的即时来自客户端引擎；**我们的即时可以来自预计算**——
   目标状态是算好的，切换就是显示/隐藏，**比任何服务器方案都快**。
3. **首屏必须有"东西"**。Roc 给可运行代码；Go Playground 把默认程序**印进 HTML**；
   Rust Playground 只给 905 字节空白 + 一句 noscript（最差）。
4. **空状态要说清"这里将出现什么"**（regex101 的两句话）——零成本、收益最大。
5. **每页要有"改完怎么知道对了"**（Rust by Example 的 `Activity` + 期望输出字符串）。
6. **进度要常驻可见**（CSSBattle 的 `0/4 done`），**难度要可分档**（Flexbox Froggy 三档脚手架）。
7. **"活着"的廉价信号**：在线人数、加载态文案、即时高亮、逐步调试（regex101 的 `Regex Debugger`）。
8. **`Embed in iframe` 是免费的传播**（Godbolt）——我们的功能页也该可嵌。
9. **两个 CTA 之间明写 `OR`**（CSSBattle）优于并排四个按钮（Linear/Vercel 的导航迷宫）。
10. **没有后端时的诚实替代**：asciinema 文本录制（Rustlings）＞ GIF ＞ 假 REPL 动画。
11. **`<pre>` 放真源码 + 按钮做渐进增强**（Roc）是"零后端功能页"的标准形态。
12. **可访问性不能丢**：Flexbox Froggy 在 10 KB 里做了色盲模式 + 40 语言；
    Tour of Go 的 `user-scalable=no` 是反例，我们要显式避免。

---

# Group D — 反模式（带实据）

## D1 用 JS 才能读正文 / 用 JS 渲染纯文本

- **`docs.stripe.com/`**：1,290,605 字节 HTML，内联 `<script>` 1,111,095 字节（86%），外部脚本 0 个， 可读正文 3,383 字符（**381×**）。
- **去脚本后的正文词数**（Googlebot UA 结果相同）：`excalidraw.com` **1 词**；`dev.epicgames.com` Unreal 5.6 文档 **3 词**；`support.hpe.com/connect/s` **0 词**；`developer.apple.com/documentation/swiftui` **35 词**；`linear.app/docs` **198 词 / 564 KB**；`docs.vmware.com` vSphere **525 词 / 806 KB**；`docs.github.com/en` **222 词**（但文章页正常）。
- **`tlapl.us`**：正文第一行自己承认 `You'll miss a lot on this web site unless you enable Javascript`。
- **`live.lean-lang.org`**：HTML 2,214 字节、**`<body>` 17 个词**，`<noscript>` 只有一句"需要 JS"，**没有任何静态回退内容**。`nandgame.com` 同理（2,300 字节壳）。
- **对照**：`go.dev/play/` 把默认程序 `package main … Hello, 世界` **印进 HTML**； Roc 把源码印进 `<pre>`。**成本为零，收益是"关掉 JS 也懂这是什么"。**

## D2 文档导航把人弄丢

- **`docs.oracle.com/javase/8/docs/api/index.html`**：`<!DOCTYPE HTML PUBLIC "-//W3C//DTD HTML 4.01 Frameset//EN">` + **三个 frame**（`packageListFrame` 20% 列、`packageFrame` 30% 行、`classFrame`）—— **"你在哪"只存在于一个 20% 宽的小框里**，没有面包屑、没有
  `rel=prev/next`、 视口里没有持久标题。**框架集的年代错误至今在线。**
- **`docs.oracle.com/en/java/javase/21/docs/api/`**：实测 `breadcrumb`=0、`rel="next"`=0、`<select>`=0； 唯一的版本线索是页脚一句 `Other versions.`。**2026 年的官方 API 文档没有面包屑。**
- **`kubernetes.io` 的版本切换器会把你踢出当前页**：`install-kubectl-linux/` 上的 `Versions` 下拉里，**`v1.37` → `https://kubernetes.io`（裸站点根）**，其余指向 `v1-36.docs.kubernetes.io`； 而文档 URL 是无版本的（`/docs/tasks/…`）——**换版本 = 换域 + 丢页面，且路径里看不出是哪个版本**。
- **`docs.rs/tokio/latest/tokio/`**：`breadcrumb`=0、`aria-current`=0、`class="current"`=0、 `rel="prev|next"`=0——**没有任何"你在哪"的标记**。 好的一面：`<h2>…<span class="version">1.53.1</span></h2>` **确实显示了版本**。
- **`coq.inria.fr/doc/V8.20.0/refman/`**：左栏把全书目录**全展开到 `#anchor` 级**，单页 282 KB，**无面包屑、无"你在哪"**。同病：`kframework.org`（39 课平铺）、`agda.readthedocs.io`（40 条字母序平铺）。
- **`isabelle.in.tum.de/documentation.html`**：**所有教程与手册都是 PDF**，站内没有可浏览的 HTML 正文； 同页还挂着 `Site Mirrors: Cambridge (.uk) / Munich (.de) / Sydney (.au) / Potsdam, NY (.us)`。
- **`vercel.com`**：`Products` 下拉 **19 项**、`Resources` 下拉 **17 项且混了四套分类法**。
- **`lean-lang.org`**：参考文档有 prev/next 但 **0 面包屑、0 版本切换器**（URL 里却写着 `/latest/`）；`/documentation/` **301 到 `/learn/`**；`/use-cases/cedar` 的 `breadcrumb`=0 且无 prev/next。
- **搜索在无 JS 时也失效**：`kubernetes.io/search/?q=deployment` 只服务 139 词（纯导航，无结果）； javadoc 的 `search.html` 直接写 `JavaScript is disabled on your browser.`（212 词、9 个脚本）。
- **对照**：Tailwind 的三级分组 + 大写小标题 + 左侧导轨 + `data-autoscroll`；Roc 的 6 项导航；**Svelte 的人格路由器**（先问你是谁）；**docs.rs 至少把版本印出来了**。

## D3 落地页纯营销、零信息

- **整页零代码的落地页（实测 `<code>`=0 且 `<pre>`=0）**：`vercel.com`（526 KB，0 处 "install"，无 semver——**一个部署平台的落地页里没有一行代码**）、 `databricks.com`（1,777 词）、`mongodb.com`（1,366 词）、`splunk.com`（1,670 词）、 `datadoghq.com`（1,665 词）、`anthropic.com`（813 词，hero 只有
  `research` / `products` 两个链接）、 `devin.ai`（514 词）、`langchain.com`（719 词）。
- **技术内容在第 N 屏之后**：`together.ai` 第一个 `<pre>`/`<code>` 出现在**第 308,855 个字符**处 （之前有 3,424 词）；`elastic.co` 第一个代码在**第 160,321 个字符**（之前 1,495 词）。
- **`svelte.dev`**：落地页**没有代码、没有安装命令、没有版本号**；主体是 `used by companies you've heard of` → 社区 → **一整墙贡献者名字** → `Backed by Vercel and countless donors`。**对已有品牌的框架可行，对新语言是自杀。**
- **`lean-lang.org`**：首屏无代码、无安装命令、无版本号（见 D10）。
- **`swift.org`**：落地页 8,851 字节，**无代码、无安装命令**（但版本至少在导航里）。
- **`unison-lang.org`**：导航区塞 newsletter 表单（`Get regular updates from the Unison team`）。
- **`www.python.org`**：没有营销 h1，定位句出现在导航/搜索/交互 shell 之后。
- **对照**：Gleam 首屏给三行代码 + `Try Gleam`；Bun 首屏给安装命令 + 版本按钮；**go.dev 首屏给一个真编辑器 + 14 个预载示例**。

## D4 假"交互"——GIF / 视频 / 假终端 / 假 live

- **假 "try it" 按钮（点进去是注册页）**：`devin.ai` 的 hero `Try Devin` → `app.devin.ai/signup`；`cursor.com` 的 `Get started` → `/login`；`vercel.com` 的 `Deploy now` → `/signup`；`together.ai` 的 `Start building` → `api.together.ai`，而该 URL **返回 HTTP
  403**。
- **`warp.dev`**：hero 面板是一个**服务端渲染的静态 mock**，却标注为 **"live"** / **"applied to all 112 agents"**；主 CTA 是 `request early access`。 同一站还带 `.sfx-toggle`（`aria-label="Mute sound effects"`，`aria-pressed="false"`） ——**默认开声音**。
- **`rustlings.rust-lang.org`** 用 asciinema（`<script src="https://asciinema.org/a/719805.js">`）—— **这是"录制"的正确形态**：文本可复制、可搜索、体积小。**GIF 是所有录制形态里最差的一种**（不可复制、不可搜索、体积大、无暂停）。
- **我们自己现在的站就是这条反模式**：`site/index.html` 的 hero 是 `assets/demos/demo-goal.png`， "怎么用"一节是 **4 张静态 PNG**（`demo-goal.png` / `demo-kernel.png` / `demo-agent.png`）， 另有 `assets/demos/demo-completion.gif`。文案写"所有演示都是扩展里真实存在的功能"，
  但读者**只能看，不能碰**。**这正是我们要在这次重建里消灭的东西。**
- **未能验证**：`shadertoy.com`（403）、`exercism.org`（403）、`observablehq.com`（429）、 Khan Academy 编辑器（机器人挑战页）——**无法判断其交互真伪，故不下结论**。

## D5 hero 里 5 个以上互相竞争的 CTA

- **`vercel.com`** 首屏 **5 个 CTA**：`Get a Demo` / `Log In` / `Sign Up` / `Deploy now` / `Talk to sales`， 其中**两对是同一目的地挂不同标签**（`Get a Demo` 与 `Talk to sales` 都去 `/contact/sales`）； 且 `Deploy now` 出现两次而**目的地不同**（hero `/signup` vs 下方 `/new`）。
  另有品牌资产按钮（`Copy Wordmark` / `Copy Logo` / `Download Brand Assets` / `Brand Guidelines`）混在主导航流。
- **`databricks.com`** 首屏 **7 个 CTA**：hero 的 `Explore the product` / `See demo`， 紧跟一排五个药丸按钮 `Explore the platform` / `Explore Lakebase` / `Explore Agent Bricks` / `Explore AI/BI` / `Explore Unity Catalog`。
- **`zoom.com`** hero 带内（h1 到下一个 h2 之间）**14 个可交互项**：`Explore products` / `Find your plan` + `Meetings` / `My Notes` / `ZoomMate` / `AI Productivity Suite` / `Phone` / `Webinars` / `Bonsai` / `Rooms` …
- **`pinecone.io`**：hero 的 `Start Building` / `Get a Demo` 之后，跟 **7 个花括号样式的伪按钮** `{Claude Code} {Cursor} {Copilot} {Codex} {Gemini} {CLI} {MCP}`——**没有 href**， 看起来像 CTA 其实是集成 tab。
- **`linear.app`**：`Product▾ / Resources▾ / Customers / Pricing / Now / Contact / Docs / Open app / Log in / Sign up`——**10 项**，其中 `Open app` 与 `Log in` 语义重叠。
- **对照**：**CSSBattle 的 `Play Game🕹️ OR Learn CSS🚀`**——两个 CTA 之间明写 `OR`； Svelte 只给 `get started` 一个主 CTA；Tailwind 只有一个 CTA 标签（渲染两次）；**`lean-lang.org` 也只有两个 CTA（`Install` / `Learn`）——它的病不在 CTA 多，在别处。**

## D6 代码块不可读 / 不可复制 / 不可验

- **TPIL ch5**（`…/theorem_proving_in_lean4/Tactics/`）：**3,467,259 字节 / 91,166 字符正文 / 37.3×**，`<span>` 48,780 个、`data-*` 32,168 个、`<pre>` 仅 5 个。**逐 token 交互不能烘进首包。**
- **`geeksforgeeks.org`**：同一选择器两条冲突规则—— `pre{overflow:auto;white-space:pre}` 之后又 `pre{overflow-x:hidden;white-space:pre-wrap;word-wrap:break-word}` → **长行在 token 中间强制折行、且无法横向滚动**；同一站三种代码字号（`code` 11pt `!important`、 `pre`
  12pt、`.CodeMirror-widget` 1.6rem）。
- **`docs.oracle.com/en/java/javase/21/docs/api/`**：`--code-font-size:14px`、 `pre.snippet{…;overflow:auto;white-space:pre}`（`--snippet-background-color:#ebecee`）、 **无折行开关、无主题、整个样式表 0 处 `prefers-color-scheme`/`color-scheme`**。
- **`isabelle.in.tum.de`**：文档是 PDF，**代码无法复制、无法被搜索**。
- **`vim-adventures.com`**：HTML 压成无空格形式、**21 个 `<h1>`**——压缩与语义化不该互斥。
- **对照**：docs.rs 的 `rustdoc-*.css` 有**用户可见的折行模式** （`:root.word-wrap-source-code … {word-break:break-all;white-space:pre-wrap}`）、 token 颜色用变量、`font-display:swap`；Tailwind 有 `⌘K` + 复制按钮；**TPIL 自己的 `copybutton.js`（做对了复制，做错了体积）**；Zig 的"命令行 +
  真输出"配对。
- **未能验证**：把源码做成图片（搜遍抓到的页面的 `<img>`，只有 UI 截图，无 carbon 类）； 固定高度裁切（抓到的 CSS 里没有 `pre{max-height…;overflow:hidden}`）。

## D7 暗色模式 = 反相浅色 / 干脆没有

- **`vercel.com`**：`.dark,.dark-theme,.invert-theme{--ds-background-100:#000;--ds-background-200:#000; --ds-gray-100:#1a1a1a;--ds-gray-1000:#ededed;--ds-gray-900:#a0a0a0;
  --ds-gray-alpha-1000:#ffffffeb;--ds-blue-900:#50a8ff;--ds-red-900:#ff5e63;…}` ——**把浅色主题的 `#fff`/`#000` 直接对调**（该文件 89 处 `#000` + 146 处 `#000000`，**0 条 `prefers-color-scheme`**，主题完全靠 class/JS 驱动）。
- **`docs.oracle.com/en/java/javase/21/docs/api/`**：**根本没有暗色模式** （0 处 `prefers-color-scheme`、0 处 `color-scheme`、无 `[data-theme]`、无开关）—— **2026 年的官方文档产品没有暗色主题。**
- **`geeksforgeeks.org`**：有 Dark Mode 开关（`aria-label="Toggle GFG Theme"`， 暗色下 `div.root[data-dark-mode=true]{--jat-section:#000;--jat-text:#9299a1;…}`）， 但文章 `pre` 用硬编码 `background-color:#e0e0e0;color:rgba(0,0,0,.9)`， 抓到的 **4 个样式表没有一个在暗色下覆盖
  `pre`/`code`**（0 匹配）。
- **`lean-lang.org`**：主题本身不错（`.dark-theme{--color-surface:#121212;--color-primary:#3b94ff; --color-text:#eee}`，源码注释还写着 *"a dedicated dark logo swap the image instead of inverting it"*），**但 `/install/` 的四张截图是纯浅色 PNG**（`vscode.png` /
  `newfile.png` / `forall.png` / `showsetup.png`， 无深色变体、无 CSS 滤镜；1×1 降采样均值 RGB = 167,218,247 / 219,229,239 / 251,251,251 / 235,241,246） ——**暗色模式下显示四张刺眼的浅色截图。"图不跟随主题"是暗色模式最常被漏掉的一环。**
- **对照**：`tailwindcss.com` 用 `--color-gray-950:#030712`（近黑非纯黑）+ 367 条 `prefers-color-scheme` + 376 处 `color-scheme`；`neovim.io` 深色 `#0f191f` / 浅色 `#e7eee8`（**两边都带色调**）；`rocq-prover.org`（内容前同步脚本 + 双 logo + Light/Dark/System 三按钮）；`doc.rust-lang.org/book`（6 套主题）；`docs.python.org` / `astro.build` / `swift.org`（三态）；

## D8 文档把安装命令藏起来

- **`lean-lang.org/install`**：**全站不存在一条可复制的安装命令**（`/`、`/install/` 的 curl/brew/sh token 数为 0）； 三步全是装 VS Code 与扩展。唯一的命令在 `/install/manual`（**两跳**）的 3-tab 控件里。
- **`nodejs.org/en/download`**：**0 个命令代码块、6 个 `<select>`**，且页面上**明写 "This page requires JavaScript"**；`/en/download/package-manager` **301 回 `/en/download`**； 原始 HTML 里 **0 处 `nvm` / `brew` / `winget`**。
- **`kubernetes.io/docs/tasks/tools/`**：**0 个代码块**；命令在按平台的子页里，包在 Bootstrap tab 控件中。 （**反证**：从 sitemap 随机取 50 个 `/docs/` URL，**49 个 200、0 个 404**——Kubernetes 不是链接腐烂的例子。）
- **`kframework.org`**：`Install K` 指向 `releases/**latest**`（不可复现）。
- **`tailwindcss.com/docs/installation`**：命令在 **18 个 `data-tab`** 之后；且该 URL **重定向到 `/docs/installation/using-vite`**（替你选了 Vite）。
- **`isabelle.in.tum.de`**：用 `js/osdetect.js` 探测系统再改建议——无 JS 就没有安装路径。
- **对照**：`bun.sh` / `deno.com` / `astro.build` 首屏给一行安装器；`doc.rust-lang.org/book/ch01-01` 命令 + 预期输出 + 排障 + 替代路径，**并解释 `$` 是提示符不用输入**。

## D9 可达性、登录墙与链接腐烂

- **文档藏在登录墙后**：`docs.gitlab.com/ee/ci/yaml/README.html` → 301 → 301 → 302 `projects.gitlab.io/auth` → 302 `gitlab.com/oauth/authorize` → 302 `gitlab.com/users/sign_in` → **403**（**5 跳，终点是登录页**）。
- **真实 404**：`docs.splunk.com/Documentation/Splunk/latest/Intro/AboutSplunk` → `help.splunk.com/en?resourceId=…` → **404**；`prometheus.io/docs/prometheus/2.0/getting_started/` → **404**； MDN `/Web/JavaScript/New_in_JavaScript` → **404**。
- **可达性倒退**：`go.dev/tour` 的 `viewport` 带 **`user-scalable=no`**（禁止双指缩放）；`vim-adventures.com` **21 个 `<h1>`**；`lean-lang.org` 首屏无产品名级 `h1` 却在整页放 **4 个 `<h1>`**。
- **★ 字体阻塞与布局抖动**：`elastic.co` 加载已废弃的 `fonts.googleapis.com/earlyaccess/notosansjapanese.css`，返回的 2,774 字节 CSS 里 **0 处 `font-display`**（浏览器默认 `auto` = FOIT + 布局抖动）；`istio.io/v1.0/docs/` 用 v1 版 `fonts.googleapis.com/css?family=Chivo…` 且无
  `display=swap`。**对照**：`lean-lang.org` 用 `…&display=swap`，tailwindcss 与 docs.rs 用 `font-display:swap`。
- **默认开声音**：`warp.dev` 带 `<button class="sfx-toggle" aria-label="Mute sound effects" aria-pressed="false">`——**除非访客主动静音，UI 音效会播**。 （**未能验证**"自动播放带声音的视频"：找到的 `<video>` 都带 `muted`—— apple.com 的 `#iphone-18-pro-animated`、nvidia.com 的
  `#video-bm-uf-gtc-dc`。）
- **对照**：`flexboxfroggy.com` 在 **10 KB** 里做了色盲模式 + 40 语言；`adam.math.hhu.de` 把 Impressum / Datenschutz 放进 `<noscript>`（**合规文本不依赖 JS**）； MDN 在不支持的浏览器里**降级为静态示例**。
- **托管寿命**：`djvelleman.github.io/stg4` 与根域 **双双 404**（仓库还在）；`lean4web.leanprover-community.org` **DNS 已死**；`coq.github.io/doc/` **404**；`jscoq.github.io` 只剩 71 字节跳转。**我们的功能页必须数据 + 渲染器都在仓库内自包含。**
- **未能验证**：`openai.com`（403）、`lovable.dev`（403 Cloudflare 挑战）、 `docs.docker.com` 经 curl（3 个 URL 超时）；**"自动播放带声音的视频"未找到实例** （apple.com / nvidia.com 的 `<video>` 都带 `muted`）。

## D10 `lean-lang.org` 专项拆解（owner 抱怨的精确化）

> owner 原话：「只有一个简单的广告单」。下面是这句话的**技术定义**。

- **首屏 DOM 顺序 = 导航 → 卖书横幅（`<section id="banner">`）→ hero（`<section id="why-lean" aria-label="Hero">`：SVG wordmark + `<p class="hero-tagline">` + `Install` / `Learn` 两个按钮）。**
- **落地页没有 `h1`**：DOM 里第一个标题是 `h3 Trustworthy`，第一个 `h1` 在约 **68,000 字符之后** （`Get Started with Lean`）。tagline 是 `<p>`。实测 16 `<section>` / 11 `<h2>` / 10 `<h3>` / 4 `<h1>` / 18 `<img>`， 正文 10,607 字符 / 218,969 字节（20.6×）。
- **首屏零代码、零安装命令、零版本号**：`/`、`/install/`、`/install/manual`、`/learn/`、`/use-cases/`、 `/community/`、`/fro/` **都没有版本号**；只有 `/doc/api/` 写 `Lean 4 4.35.0-rc2`（**一个 RC**）。
- **全站不存在可复制的安装命令**：`/install` 三步全靠 `vscode:` 深链；唯一的命令在 `/install/manual` （两跳）的 3-tab 控件里。**想要一个终端命令的用户在官网上没有路径。**
- **首屏代码是 JS 动画**（`initCodeAnimations`，逐 token `animationDelay`，tablist 带 prev/next）； 它的 playground 链接**跳到另一个子域**并带 base64 载荷 （`live.lean-lang.org/?from=lean#codez=<base64>`）。
- **落地页零 `<iframe>`** → **在 lean-lang.org 域内无法体验产品**。**体验被外包给另一个域，落地页只负责把人送走。**
- **唯一能零安装体验 Lean 的地方是一个 JS-only 的跨域 SPA**（2,214 字节、`<body>` 17 个词、 无静态回退）；而 `lean4web` 自认只够 *"some (smallish) Lean snippets"* / *"Doodling"*， *"serious Lean code development and larger projects are considered out-of-scope"*。
- **技术内容在第 4 屏才出现**：前面是 hero → 代码 tab → `Trustworthy/Powerful/Extensible` → `Get Started with Lean`；之后才是 `FEATURED PROJECTS` → `PROGRESS` → `Growth and Impact` （**5 条名人证言**）→ `Sponsors and Partners`。**营销段排在信息段前面。**
- **营销机械**：`testimonials.js`（证言轮播）、`gallery.js` + `glightbox.min.js`（灯箱画廊）、 `motion.js`（滚动动画）。**工程卫生**：同页加载 **MathJax 与 KaTeX 两套渲染器**，`theme.js` **include 两次**。
- **`/learn` 是链接农场**：TPIL / FPIL / MIL / Language Reference / FAQ / Mathlib API / Hitchhiker's Guide / Logic and Proof / Mechanics of Proof / Founder's Blog **平铺为同侪**，各挂 `READ NOW`。**没有"你是程序员/数学系/中学生 → 从 X 开始"的决策程序**；`/documentation/` 还
  301 到 `/learn/`。
- **参考文档有 prev/next，但 0 面包屑、0 版本切换器**（URL 里写着 `/latest/`）；`/install/` 的四张截图是纯浅色 PNG，**暗色模式下显示四张刺眼浅色图**。
- **它自己知道问题**：`/use-cases/*` 里挂着 `<div class="no-js-warning">If you want the full website experience, enable JS</div>`——**承认依赖 JS，但不提供回退**。
- **细节不一致**：导航的 `Roadmap` 指向 `/fro/roadmap`，页脚却指向 `/fro/roadmap/y4-1`（**两个不同的路线图 URL**）；`Install` 按钮的 class 里带着 `plausible-event-name=Home+Install+Primary`（**埋点写进标记**）；`/community/` 是 Zulip + YouTube + Stack Exchange + 四个周期性 Google Meet（带
  `Subscribe to iCal`）， 但**没有论坛或文档入口**。
- **该保留的**（拆解者的公允结论）：**两个 CTA 的 hero**、hero 里的真 Lean 代码、 `&display=swap` 字体、**带色调（非纯黑）的暗色主题**、参考文档的逐页 prev/next、FAQ 页。
- **该修的**：首屏要有真正的 `<h1>` / 产品陈述、要标版本、落地页或安装页要有安装命令、 要有**同源的 try-it（或把 playground 用 iframe 嵌进来）**、`/doc/` 要有面包屑 + 版本切换器、 截图要跟随主题。**这六条正好是我们这次重建的验收清单。**
- **对照我们自己**：我们已有 `site/data/site.json`（**CI 生成的版本号 / 单元数 / 进度**）、 `playground.sokonanoda` 画布、`--json` 内核事件流、`courses/set-theory/course.json`（含 `prereqs`）。**我们缺的从来不是数据，是把它渲染成可交互的页面。**

---

## 跨组模式（三个 Group 的收敛结论）

- **首屏给安装命令的只有 `bun.sh` / `deno.com` / `astro.build`**（都是一行安装器）；**首屏给可运行代码的只有 `go.dev` 与 `roc-lang.org`**。首屏"能跑"是稀缺品——**这是我们最大的机会**。
- **证明助手的文档普遍是"参考手册当目录"**（Agda 40 条字母序、Coq 无限深树、K 39 课平铺、Isabelle 全 PDF）。**没有一家把"新手第一步"做成按钮。**
- **版本可见性是分水岭**：Rocq 放进导航、Zig 放进 CTA、Swift 放进导航项、Python 连支持状态一起给；**Lean 落地页完全没有**。
- **"活着" = 回路短 + 有东西可碰 + 说清在等什么**；"死了" = 空白编辑器、GIF、假 live、维护模式横幅。
- **能零后端做到"活"的机制只有四种**：预计算的展开/折叠（Alectryon）、预计算的多视角（TS Playground 的 inlay）、预计算的期望值对比（PGExercises）、预计算的事件流回放（Go 的 playback 机制）。**我们已经拥有生成这四种数据的全部原料。**

---

## 值得抄的 12 个具体做法

1. **Alectryon 式纯 CSS 证明状态折叠**（`rocq-prover.org`）：`<input type=checkbox hidden>` +
   `<label>` 包住该行 + 兄弟 `<small class=output>`；CSS `.sentence > .toggle:checked ~ .output { display:block }`，
   外加 `:hover` 显示。**为什么适合我们**：零 JS / 零后端 / 零 WASM；无 JS 时 checkbox 仍可勾选，**功能页在 JS 关闭时依然能用**；而我们的目标状态本来就是预计算的。
2. **`<pre>` 放源码 + 一个渐进增强按钮**（`roc-lang.org`）：源码在 `<pre>` 里（可索引、可复制、无 JS 可读），
   按钮初始文案 `Enable JS to Run`。**为什么适合我们**：这是"零构建静态站 + 功能页"的标准形态；
   我们的 `<pre>` 里放 `.sokonanoda` 源码，按钮揭示**预计算**的目标状态与诊断。
3. **落地页嵌真编辑器 + 预载示例下拉**（`go.dev` 的 `Try Go`，8 个预载程序，**同源**）。**为什么适合我们**：**"前 30 秒"的正解是"不用想写什么，挑一个就跑"**；
   我们可以在 `site/` 内嵌一个同源的预计算走查器，把 8 个程序换成 8 个练习。
4. **关卡 JSON 的字段表**（`adam.math.hhu.de/…/level__Tutorial__1.json`）：`title` / `introduction` /
   `descrText`（散文陈述）/ `descrFormat`（带洞语句）/ `conclusion`（通关后教下一步）/
   `tactics` / `lemmas` / `definitions` / `module`。**为什么适合我们**：这几乎就是
   `playground.sokonanoda` 一个练习该有的全部字段；照它定 schema，功能页与课程页可共用一份数据。
5. **`tile` 课程卡 + 显式 `prerequisites`**（`game.json`：`title` / `short` / `long` /
   **`prerequisites: []`** / `levels` / `worlds` / `languages` / `image`）。**为什么适合我们**：`course.json` 已有 unit/章/`prereqs`（G6 还强制 `prereqs` 不悬空）——
   **直接映射成课程卡，前置关系是数据不是文案**。
6. **`worlds.edges` 世界图 + `altTitle` 签名提示**（同上）：世界不是列表而是 `edges` 图；
   侧栏每条 tactic/lemma 带 `altTitle`（`" (a b c : ℕ) : a + b + c = a + (b + c)"`）——
   **一个数组同时当解锁清单和签名速查表**，省掉一整个"速查表"页面。
7. **版本生命周期标注**（`docs.python.org/3/`）：`3.16 (in development)` / `3.15 (pre-release)` /
   `3.14 (stable)` / `3.12 (security-fixes)` / `3.9 (EOL)`——**支持状态写在版本号旁边**。**为什么适合我们**：我们同时有 CLI / LSP / VSIX / 课程四个版本口径，每个都该带状态标签。
8. **版本 + commit + 日期三件套，以及 `$` 提示符说明框**（`doc.rust-lang.org/std/` 头部
   `std 1.98.1 (48a229cea 2026-09-01)`；`book/ch01-01` 的 `Command Line Notation`）。**为什么适合我们**：`site.json` 已生成版本号，加 commit 与日期成本为零；
   而"`$` 是提示符不用输入"是新手复制粘贴失败的头号原因，**我们所有命令块都该有这一句**。
9. **版本号进 CTA 按钮 / 导航项**（`bun.sh` 的 `Install Bun v1.4.2`；`ziglang.org` 的
   `GET STARTED` + `Latest Release: 0.16.0`；`swift.org` 的导航项 `Install (6.4.0)`）。**为什么适合我们**：一个按钮同时回答"装什么"和"装到哪个版本"，
   且**版本号由 `site.json` 生成、永不手写**（符合 `docs/design/site.md` §3 的单一事实源纪律）。
10. **"形容词必须能点开定义"**（`roc-lang.org`：`Fast` → `/fast` 写 `What does fast mean here?`，
    而 `/fast` 是自陈上限、声明非目标、给构建延迟预算的工程文档）。**为什么适合我们**：官网迟早会写"真内核判卷""输入全程有提示"这类主张——
    **每个主张都必须能点开看到一个具体的、可验证的例子**，否则就是 owner 抱怨的"广告单"。
11. **示例页双钉可复现性**（`roc-lang.org/examples`：`built and tested with Roc nightly-2026-09-18-1d982dc` +
    `Their source comes from roc-lang/examples at e623ae7`）。**为什么适合我们**：功能页上每段预计算结果都是**某一次 `grade` 的产物**；
    把**二进制版本 + 源文件 commit** 印在页脚，读者才能判断它是否过期。**这是预计算路线的信任基础。**
12. **两个 CTA 之间明写 `OR` + 常驻进度 + 客观判分**（`cssbattle.dev`：`Play Game🕹️` / **`OR`** /
    `Learn CSS🚀`；页头 `0/4 done`；"the shorter your CSS, the higher your score"）。**为什么适合我们**：访客只有两类——"我想学证明"与"我想看这东西怎么做的"，**明写 `OR` 比并排四个按钮清楚**；
    进度常驻（`已判 N 题`）是廉价活体信号；**判分指标必须是机器算的**——我们的对应物是
    "内核是否接受 + 用了几个 tactic"。

> 补充（成本极低所以列出）：
> **regex101 的空状态文案**（`An explanation of your regex will be automatically generated as you type.`）
> ——我们的初始态该写"光标移到任意一行，这里会显示那一步的目标与假设"。
> **Rustlings 的 asciinema**（`<script src="https://asciinema.org/a/719805.js">`）
> ——文本型录制可复制可搜索，**是替换现有 GIF 的最低成本方案**。
> **Koka 的双重排版**（同一段代码渲染两遍，第二遍每个 token 内联标注推导出的类型与效果并链进 stdlib）
> ——**我们的 tactic 可以标注出它这一步推出来的目标与假设，并链到该 tactic 的说明页**。
> **Gleam 的"按来源语言做 cheatsheet"**——我们的对应物是"给 Lean 4 用户 / 给 Coq 用户"的对照表。

## 必须避开的 12 个具体做法

1. **用 1.1 MB 内联 JS 渲染 3.4 KB 文字**——`docs.stripe.com/`（1,290,605 字节 HTML /
   1,111,095 字节内联脚本 / 3,383 字符正文 / **381×**）。静态站最大的优势就是"不用等 JS"。
2. **逐 token 交互烘进首包**——`leanprover.github.io/theorem_proving_in_lean4/Tactics/`
   （**3,467,259 字节 / 91,166 字符 / 37.3×**，`<span>` 48,780 个、`<pre>` 仅 5 个）。
3. **导航按组织架构分类**——`vercel.com`（`Products` 19 项；`Resources` 17 项混四套分类法）；
   同病 `databricks.com`（首屏 **7 个 CTA**）、`astro.build`（一级导航 12 项平铺）。
4. **侧栏一次展到 `#anchor` 级**——`coq.inria.fr/doc/V8.20.0/refman/`（单页 282 KB，无面包屑）；
   同病 `kframework.org`（39 课平铺）、`agda.readthedocs.io`（40 条字母序平铺）。
5. **把文档做成 PDF / 藏在登录墙后**——`isabelle.in.tum.de/documentation.html`（全 PDF）；`docs.gitlab.com/ee/ci/yaml/README.html`（**5 跳后落在登录页并 403**）。
6. **安装命令不存在或藏得极深**——`lean-lang.org/install`（**0 条命令**）；`nodejs.org/en/download`（**0 个命令块 + 6 个 `<select>` + 明写 "This page requires JavaScript"**，`/en/download/package-manager` 还 301 回 `/en/download`）；`kubernetes.io/docs/tasks/tools/`（**0 个代码块**）；`tailwindcss.com/docs/installation`（命令在 18 个 `data-tab` 之后）；`kframework.org`（`releases/**latest**`，不可复现）；`isabelle.in.tum.de`（`osdetect.js` 探测系统）。
7. **把"体验"外包给另一个域**——`lean-lang.org`（落地页**零 `<iframe>`**，`Playground` 跳**跨域** `live.lean-lang.org/?from=lean#codez=<base64>`）。**落地页自己不能体验产品，就是广告单。**
8. **假 "try it"**——`devin.ai` 的 `Try Devin` → `/signup`；`cursor.com` 的 `Get started` → `/login`；`vercel.com` 的 `Deploy now` → `/signup`；`together.ai` 的 `Start building` → `api.together.ai`（**403**）；`warp.dev` 把静态 mock 标成 **"live"** 且**默认开音效**。
9. **落地页零技术信息**——`svelte.dev`（无代码、无安装命令、无版本号，主体是"知名公司在用" + 贡献者墙）。
   已有品牌可以这么赌，**新项目这么赌等于没人知道你是什么**。
10. **暗色 = 反相 / 主题机制重复 / 图不跟随**——`vercel.com`（`.dark{--ds-background-100:#000; …}` 直接对调
    `#fff`/`#000`）；`geeksforgeeks.org`（有暗色开关但 `pre` 硬编码 `#e0e0e0`）；`lean-lang.org`（**同页加载 MathJax 与 KaTeX**、`theme.js` **include 两次**、
    **`/install/` 四张纯浅色截图**）。
11. **代码块不可读**——`geeksforgeeks.org`（同一选择器 `white-space:pre` 与 `pre-wrap` 冲突 →
    **token 中间强制折行**；三种代码字号）；`isabelle.in.tum.de`（PDF 不可复制）；`vim-adventures.com`（压成无空格 HTML + **21 个 `<h1>`**）。
12. **我们自己的假演示**——`site/index.html` 现在用 `assets/demos/demo-goal.png`（hero）
    与 4 张静态 PNG（"怎么用"一节）+ `assets/demos/demo-completion.gif`。
    文案说"所有演示都是扩展里真实存在的功能"，但**读者只能看不能碰**。**这条必须在这次重建里消灭。**

> 另三条必须记住：
> **别挡爬虫**（`exercism.org` 403、`shadertoy.com` 403、`observablehq.com` 429、
> `openai.com` 403、`lovable.dev` 403——挡爬虫 = 挡掉所有第三方拆解与 AI 引用）；
> **别让托管成为单点**（`djvelleman.github.io/stg4` 404、`lean4web.leanprover-community.org` DNS 已死、
> `coq.github.io/doc/` 404）；**别禁止缩放**（`go.dev/tour` 的 `user-scalable=no`）。

## 对我们「功能页」的直接启示

> 前提：**零构建静态 HTML/CSS/vanilla-JS + GitHub Pages + 无后端**。
> 核心洞察：**我们的判卷结果是有限的、可枚举的、可预计算的**——
> 这比 Roc 的 WASM 编译器和 Lean 的服务器判卷**都便宜**，而且能买到"即时"。
> 按**性价比从高到低**排，每条给"输入 → 生成 → 渲染 → 降级"。

## T0 ★★★ 把 `--json` 事件流烘成"逐步目标状态"页（最高性价比）

- **输入**：`site/data/site.json` 里已有的 `examples` 列表（`fol-basics.sokonanoda` / `lesson-01.sokonanoda` / `lesson-02.sokonanoda` / `py-fol-core.sokonanoda` / `py-nat.sokonanoda`） + `playground.sokonanoda` 画布 + `courses/set-theory/` 的单元。
- **生成**：在 `scripts/gen-site-data.py` 里加一个 pass——对每个示例的每个 tactic 位置调 `scripts/soko query state --file X --line L --col C --json`（或直接消费 `grade --json`）， 产出 `site/data/walkthrough/<name>.json`：`{ soko_version, source_commit, steps: [ {line,
  col, tactic, goals:[{hyps:[], target}], messages:[]} ] }`。
- **渲染**：生成器**直接输出静态 HTML**（关键：**HTML 是主路径，JS 只是增强**）： 每个 tactic 一行 = `<input type=checkbox id=s3 hidden>` + `<label for=s3 class="sentence">…该行源码…</label>` + `<div class="goals">…目标与假设…</div>`； CSS 用 `:checked ~ .goals { display:block }` 与
  `:hover .goals { display:block }`——**照抄 Alectryon**。
- **JS 增强（约 80 行 vanilla）**：`←`/`→` 或 `上一步/下一步` 切换高亮行；checkbox 本身可聚焦，键盘可达。**JS 挂了，勾选展开仍然可用。**
- **为什么第一**：这是**唯一一条能把现有 4 张 PNG + 1 个 GIF 换成真东西、且完全不新增运行时**的做法； 工作量集中在生成器（半天量级），之后每个新示例**零边际成本**。
- **反模式对照**：不要学 TPIL 把逐 token 交互烘进首包（3.4 MB / 37×）。**目标状态按行挂，不按 token 挂。**

## T1 ★★★ 功能页的"关掉 JS 也能用"底线

- 同一份生成器产物同时输出**无 JS 可用**的形态：默认展开第一个练习的第一步， 其余步骤靠纯 CSS 折叠。**首屏必须能读到：这是什么 + 一个真实的命题 + 一步真实的判定。**
- 照抄 `go.dev/play/`（默认程序印进 HTML）与 `roc-lang.org`（源码在 `<pre>` 里）。
- **反面**：`live.lean-lang.org` 只有 2,214 字节 / `<body>` 17 个词 + 一句"需要 JS"；`play.rust-lang.org` 只有 905 字节 + 一句"需要 JS"。
- **成本**：几乎为零（生成器多一个模板分支），**但它决定了页面能否被搜索、被引用、被 AI 读到**。

## T2 ★★ 用"枚举答案"实现真判卷交互（不需要 WASM）

- **做法**：对每个练习预计算 **1 个正解 + 2–3 个典型错解**（我们最清楚新手会怎么错—— 这正是 `front::judge` 教学提示的来源），生成 `{ choice: "把前提顺序写反", verdict: "kernel-rejected", diagnostic: "…", hint: "…" }`。
- **交互**：页面上给几个候选写法，读者选一个 → 揭示**内核的真实诊断与教学提示**。 这就是"判卷"，但**答案空间是枚举好的，所以零后端**。
- **为什么值**：`lean-lang.org` 没有任何地方能让人看见"系统会拒绝你"；**看见一次拒绝**比看十次成功更能建立"真内核"的信任。
- **对照**：`gleam.run` 把编译器报错贴出来当卖点——同一条思路，我们更进一步做成可选。
- **成本**：中（要为每个示例准备错解），但**可以只对 3–5 个旗舰示例做**。

## T3 ★★ 课程地图：把 `prereqs` 渲染成图（数据已有）

- **输入**：`courses/set-theory/course.json`（卷/章/unit + `prereqs`；G6 已强制 `prereqs` 不悬空） + `site.json` 里每 unit 的 `checked` / `open` / `targets` / `status`。
- **渲染**：照抄 NNG 的 `worlds.edges` + `worldSize`——**世界不是列表而是图**。 用纯 SVG 或 CSS grid 画节点 + 边，节点上标 `已判 N / 目标 M`。
- **为什么值**：这是**唯一一条把已有生成数据直接变成"看起来像产品"的页面**， 且 `prereqs` 的拓扑是 CI 保证的，不会画错。
- **成本**：低-中（一个 SVG 渲染函数）。**Svelte 的人格路由器**（先问你是谁）可作为入口叠加在这张图上。

## T4 ★★ 目标状态即时跟随（"零点击"回路）

- 因为目标状态已预计算，**鼠标移到任意 tactic 行 → 右侧立即显示该步目标**， 纯 CSS `:hover` 即可（Alectryon 里就有这条规则）。
- 这是我们在"无后端"前提下能做出的**最接近 Tailwind Play 的零点击体验**。
- **成本**：零（T0 的产物加一条 CSS 规则）。

## T5 ★★ 用"可播放的事件流"替换 GIF

- 现在 `site/assets/demos/demo-completion.gif` 是死的。**我们的 `--json` 事件流本身就是时间序列**： 生成 `site/data/replay/*.json`（每行一个事件 + 相对时间戳），页面按时间戳逐行揭示 `decl.checked` / `exercise.open` / `diagnostic`，旁边同步高亮对应的编辑器行。
- **这比 asciinema 更好**（Rustlings 用 `asciinema.org/a/719805.js`）， 因为**数据我们已经有了，且是结构化的**（可以暂停、回退、单步、跳到最后）。
- **降级**：无 JS 时退化成一张**静态的完整事件清单**（仍然可读、可复制）。
- **成本**：中（一个 ~120 行的播放器 + 生成器一个 pass）。

## T6 ★★ 客户端搜索（照抄 TPIL 的静态索引模式）

- **做法**：生成 `site/data/search-index.json`（单元标题 + 目标语句 + 诊断文本 + 文档小标题）， 用 ~40 行 JS 做 substring/fuzzy 匹配。**零后端**。
- **先例**：TPIL 用 `-verso-search/searchIndex.js` + `elasticlunr.min.js` + `fuzzysort.min.js` ——**整个检索索引作为静态文件随站发布**。
- **成本**：低（索引生成是字符串拼接；匹配可以极简）。

## T7 ★★ agent 通道（我们已经领先，要做成一等入口）

- **先例**：`deno.com` 在安装块下面直接给 *"Or let your agent do it — Paste this into any coding agent: Read deno.com/agents.md…"*；`vercel.com` 的 getting-started 页**先给 `Agent prompt` 块，人的路径排后面**；`docs.stripe.com` 的 quickstart `.md` 开头有专门给 "Coding
  agents" 的沙箱指令；`svelte.dev/docs` 有 **`I'm a Large Language Model (LLM)` → `/docs/llms`** 一张卡。
- **我们的优势**：`site/agents.html` 已有安装 prompt，`docs/design/deepseek-harness.md` 已有能力差异表，`--json` 是结构化契约。**只需把它从"一个页面"提升为"首页上的一条并列路径"。**
- **低成本增强**：`/llms.txt`（把 `site/` 各页的标题 + 一句话 + URL 拼成一个纯文本索引， 照抄 Deno / Svelte / Elixir）+ 每页的"以 Markdown 查看"（我们的源本来就是纯 HTML，成本更低）。
- **成本**：低（生成器一个 pass）。

## T8 ★ 让功能页可被 iframe 嵌入

- 功能页做成一个无导航的独立 HTML（照抄 Godbolt 的 `Share → **Embed in iframe**`）， 这样课程页、博客、外部文章都能嵌我们的可交互证明。**成本**：低（同一份产物 + 一个"裸"模板）。

## T9 ✗ 明确不做（并写清理由）

- **不做 WASM 内核**：`roc-lang.org` 走通了，但代价是维护一整套前端编译栈 + 未知的首包体积 （我实测 `/compiler.js` 的 `HEAD` 返回 0 字节、`compiler.wasm` 404，**连测量都不容易**）。 我们的判卷结果**有限且可枚举**，预计算足够。
- **不做服务器判卷**：`live.lean-lang.org` 与 `adam.math.hhu.de` 都是这条路线， 前者自认只够 *"smallish snippets"* / *"Doodling"*，后者 **bundle 6.1 MB + 实时 CPU/MEM 负载**， 且社区实例（`lean4web.leanprover-community.org`）**已经死了**。**在 GitHub Pages 上做服务器判卷 =
  给未来埋一个必然失效的单点。**
- **不做 GIF / 假终端动画**：见 T5。也**不做假 "try it" 按钮**（点进去是注册页，见 D4）。

## 落地顺序建议（一轮之内可交付）

| 顺序 | 交付物 | 复用现有 | 新增代码 | 消灭的反模式 |
|---|---|---|---|---|
| 1 | `site/data/walkthrough/*.json` + 生成器 pass | `site.json` 的 `examples`、`query state --json` | 生成器 ~150 行 | 假演示（D4） |
| 2 | `site/try.html`（旗舰练习的逐步目标状态，纯 CSS 折叠） | 同上 | 模板 + CSS ~80 行 | 体验外包（D10） |
| 3 | 上一步/下一步 + 键盘 + `:hover` 跟随 | — | JS ~80 行 | 死板 |
| 4 | 3 个典型错解 + 内核真实诊断 | `front::judge` 的教学提示 | 数据 + 模板 | 只有成功没有失败 |
| 5 | 课程地图（`prereqs` 图 + 每 unit 进度） | `course.json`、`site.json` | SVG ~60 行 | 无动线 |
| 6 | 事件流回放替换 GIF | `grade --json` | 播放器 ~120 行 | GIF |
| 7 | 静态搜索索引 + `/llms.txt` + agent 入口 | 全部单元/文档标题、`site/agents.html` | JS ~40 行 + 生成器 | 找不到东西 / agent 通道被埋 |
| 8 | iframe 嵌入版 | 同 2 | 一个裸模板 | 不可传播 |

**每一层都独立可交付、可回滚，且都不引入任何构建步骤或运行时依赖**——
符合 `docs/design/site.md` §2 的方案 A 与 `REQUIREMENTS.md` §2 第 9 条
「用户/agent 路径零工具链依赖」。


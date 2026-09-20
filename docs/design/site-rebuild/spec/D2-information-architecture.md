# D2 —— 站点信息架构、内容模型与实施环节

> 本文是 `site/` 全面重构的**施工图**。设计规则见 `D1-design-rules.md`（从
> `../research/R1-design-craft.md` 提炼的可执行手册）；事实卷宗见 `../content/`
> （C1 语言 / C2 教学 / C3 工具链 / C4 现状与路线）；同类站点拆解见
> `../research/R2-docs-teardown.md`。
>
> **不参考旧版设计**（用户明确要求）。旧版的问题已被量化（R1 §2）：17 个不同
> `font-size`、6 种圆角、11 个非网格 gap、27 个硬编码色值对 9 个变量、0 个
> `transition` / `prefers-*` / `:focus-visible`、正文行宽 ≈112 拉丁字符、
> 2 处对比度不达标、无 favicon / og / canonical / theme-color / 暗色模式。
> 旧版的**文案与机制是资产**，保留；旧版的**视觉层整体废弃**。

## 0. 五条施工原则

| # | 原则 | 可判定的含义 |
|---|---|---|
| 1 | **内容为王** | 页面先有内容再谈形式。任何一页若删掉视觉只剩标语，就是没做完 |
| 2 | **每个主张都能点开看到真实证据** | 写"真内核判卷"就必须有一个真实 `--json` 事件流可看；写"零工具链"就必须有可复制的命令。R2 借自 `roc-lang.org`：*形容词必须能点开定义* |
| 3 | **零伪造** | 演示不是画的、不是 GIF：真实代码、真实内核输出、真实计数。旧版用 PIL 画的假 VS Code 截图（`scripts/gen-site-demos.py`）整体退役 |
| 4 | **快** | 每页 HTML ≤ 60 KB、CSS 合计 ≤ 45 KB、JS ≤ 8 KB、字体 ≤ 120 KB；**零图片**（编辑器面板是真 DOM），零第三方请求，零 CDN |
| 5 | **机制防漂移，不靠人肉校对** | 版本/计数一律生成；导航块单源 + 校验；体积与令牌写进 `check-site.py` 断言 |

## 1. 站点地图

21 页 + 1 个样板页。**分级**：A 理解产品 / B 看它做什么（重点，最多页）/
C 学 / D 用 / E 过程与信任。

| # | 路径 | 目的 | 内容源 | 新写量 | 阶段 |
|---|---|---|---|---|---|
| — | `styleguide.html` | **组件与令牌样板页**（设计检查点；页脚可进，不在主导航） | D1 令牌 | 全 | S0 |
| A1 | `index.html` | 一句话定位 + 核心循环一眼看懂 + 两条路径入口 | C1/C4 | 全 | S2 |
| A2 | `why.html` | 为什么需要它：问题 → 做法 → 与 Lean 4/Coq 的关系与边界 | C4 §3、C1 §1 | 全 | S8 |
| A3 | `vision.html` | 终极形态、十条硬规则与它们为什么存在、用户原话 | C4 §3 | 全 | S9 |
| B1 | `kernel.html` | **内核判卷**：一次判卷的完整真相（真事件流 / 真拒绝 / 真诊断 / 为什么禁止文本比对） | C1 §4、`docs/protocol.md` | 全 | **S1（样板页）** |
| B2 | `walkthrough.html` | **走查一题**：真实逐行目标状态，纯 CSS 折叠（Alectryon 式），零 JS 可用 | 生成数据 | 全 | S3 |
| B3 | `editor.html` | 编辑器里的一天：goal 视图 / hover / 补全 / 诊断 / code action / 项目树 | C3 §2 | 全 | S4 |
| B4 | `language.html` | 语言能力总览：语法清单（每条带真实样例）+ prelude 三层 | C1 §2/§5 | 全 | S5 |
| B5 | `tactics.html` | tactic 与证明语言：完整清单 + 真实例子 | C1 §3 | 全 | S6 |
| B6 | `notation.html` | 用户自定义记法（本语言很特别的能力） | C1 §2、`docs/design/notation-subset.md` | 全 | S10 |
| B7 | `projects.html` | 多文件 `import` 与项目管理 | C1 §2、`docs/design/imports-and-projects.md` | 全 | S11 |
| B8 | `diagnostics.html` | **诊断字典**：完整码表 + 真实 hint + 可筛选 | 生成数据 | 全 | S7 |
| B9 | `agents.html` | 给 code agent：skills / MCP / `--json` / `query` / 一段安装 prompt | C3 §4 | 全 | S12 |
| C1 | `course.html` | 入门课 11 单元 | C2 §5 | 全 | S13 |
| C2 | `set-theory.html` | 卷 I《集合论》12 单元 + 真实计数 | C2 §2/§3、生成数据 | 重写 | S14 |
| C3 | `learn.html` | 怎么学：教学循环、老师 agent、卡住时怎么办 | C2 §6/§8 | 全 | S15 |
| D1 | `get-started.html` | 安装三条路径 + 排错（含"`$` 是提示符不用输入"） | C3 §1/§7 | 重写 | S16 |
| D2 | `docs.html` | 文档索引（分层，只给入口不复写正文） | `docs/README.md` | 重写 | S17 |
| E1 | `progress.html` | 进展时间线（真实 tag + 轮次）+ 路线图 + 诚实的未完成清单 | C4 §2/§4/§5、生成数据 | 重写 | S18 |
| E2 | `engineering.html` | 工程纪律与质量证据（CI 门禁、测试数、四本台账、发布自动化、缺口台账） | C4 §6、生成数据 | 全 | S19 |
| E3 | `about.html` | 关于 / 许可 / 上游来源 / 设计规范入口 | `NOTICE.md` | 重写 | S20 |
| F1 | `en/index.html` | 英文 landing（一页，指向中文站） | 中文页提炼 | 重写 | S21 |

### 1.1 第二轮补充（装上前端设计技能后复审，2026-09-19）

把 `art-direction` / `frontend-implementation` / `visual-qa` 三个技能装上后重读
本表，发现**七处真实缺口**——每一处都用「它服务什么目的」过了一遍，没有为了凑数
而加的页：

| # | 路径 | 目的（为什么它不是凑数） | 内容源 | 阶段 |
|---|---|---|---|---|
| G1 | `search.html` | 27 页之后没有搜索就是导航失败。**静态索引 + 客户端筛选**，零后端 | 生成 `data/search.json` | S24 |
| G2 | `compare.html` | 「我学过 Lean 4 / Coq / Agda，这东西跟我有什么关系」——这是**最大的两类潜在用户**，现在全站没有一页回答 | C1 §7、`docs/notes/research.md` | S25 |
| G3 | `glossary.html` | 全站密集使用归纳类型/recursor/宇宙/tactic/elaborator 等术语，却没有一处定义。教学产品缺术语表是硬伤 | C1 §2、C2 | S26 |
| G4 | `faq.html` | C3 §7 已有**24 行安装痛点表**（离线、代理、版本缓存过期、Windows、macOS 隔离、市场不可用…），目前无处安放 | C3 §7 | S27 |
| G5 | `releases.html` | 版本历史与生命周期标注（R2 借自 `docs.python.org` 的 `3.14 (stable)` / `3.9 (EOL)` 做法） | 生成 `data/timeline.json` | S28 |
| G6 | `404.html` | GitHub Pages 支持自定义 404。21+ 页站点的基本卫生 | 少量 | S29 |
| G7 | `sitemap.xml` · `robots.txt` · `llms.txt` | 站点卫生 + **agent-first**：这个产品的用户里有 code agent，`llms.txt` 是给它读的入口（R2 拆解 Deno/Vercel/Stripe/Svelte 都有） | 生成 | S29 |

另有两项非页面缺口，同批补：

- **`site/assets/agent-prompt.js` 必须重写**：C3 §4 确认它是 4 个页面共用的唯一源，
  且三份设计文档已把它列为重写目标；当前文本承诺了 `scripts/soko` 并不具备的
  「扩展自带回退」（见 §4 红线 1）。
- **打印样式**：教学站要能被打印（D1 已列 print 一节），随 S0 的 `base.css` 一起做。

**扩充后总页数：28 页 + 1 个样板页 + 3 个站点文件。**

> 判据仍然是「它服务什么目的」。**不加**的东西也记在这里，免得下一轮又想起来：
> 用户论坛 / 评论 / 登录 / 在线判题（无后端，且与「不做功能页」的决定冲突）、
> 博客（进展页已经承担，且会变成第五条漂移线）、多语言全站镜像（用户已选
> 中文主站 + 英文 landing）。

**被删除的旧页**：`vision.html` 保留但重写；`site/agents.html` 重写为 B9；
旧 `course.html`/`progress.html`/`docs.html`/`about.html`/`get-started.html`
全部重写；`site/en/index.html` 重写。

**不做的**：WASM playground、浏览器内真判卷、后端服务（用户已拍板：不做功能页，
功能展示尽量多，快优先）。B2 的"走查"是**预计算数据 + 纯 CSS 折叠**，属于展示，
不是执行。

## 2. 内容模型：单一事实源与生成物

**铁律**：版本号 / 计数 / 进展**一律不手写**（`docs/design/site.md` §3 的既有纪律，
继续沿用并扩展）。

| 数据文件 | 内容 | 生成者 | 谁消费 |
|---|---|---|---|
| `data/site.json` | 版本、轮次、日期、入门课 11 单元、卷 I 12 单元与计数、`examples` | `scripts/gen-site-data.py`（**已有，扩展**） | 几乎每页 |
| `data/walkthrough.json` | 一题的**逐行真实目标状态**（`query state` 的真实输出，含 `goal_runs` 高亮） | `scripts/gen-site-lab.py`（新） | B2 |
| `data/events.json` | 一次判卷的**真实 `--json` 事件流**（含一条真拒绝） | 同上 | B1 |
| `data/diagnostics.json` | 诊断码字典（码 / 阶段 / 说明 / 真实 hint / 真实复现片段） | 同上（解析 `docs/protocol.md` + 实跑内核） | B8 |
| `data/timeline.json` | 真实时间线（tag + 日期 + 一句话 + 轮次标题） | 同上（读 `git tag` + `CHANGELOG` + `STATUS.md`） | E1 |
| `data/gaps.json` | 缺口台账（24 条：状态 / 严重度 / 复现 / 何时修好） | 同上（读 `docs/gaps/ledger.jsonl`） | E2 |
| `data/evidence.json` | 质量证据数字（测试数 / 门禁 / 台账条数） | 同上 | E2 |

**生成纪律**（沿用 `gen-site-data.py` 的既有做法）：
- 纯 python3 标准库，零第三方依赖；
- 源缺失或解析失败 ⇒ **该字段留空，绝不编造**；生成器打印人话原因；
- 输出确定（键排序、顺序稳定）⇒ 重跑幂等；
- **测量绝不触发工具链下载**（`SOKONANODA_OFFLINE=1`），解析不到版本钉时沿用
  上一轮产物并把 `measured_at` 留在数据里；
- **每份预计算数据都带 `version` + `source_commit`**：R2 借自 `roc-lang.org`——
  读者要能判断它是否过期。页脚印出来。

## 3. 工程契约

### 3.1 目录

```
site/
  index.html  why.html  vision.html
  kernel.html walkthrough.html editor.html language.html tactics.html
  notation.html projects.html diagnostics.html agents.html
  course.html set-theory.html learn.html
  get-started.html docs.html
  progress.html engineering.html about.html
  styleguide.html
  en/index.html
  _partials/            # 导航块与页脚块的唯一源（校验用，不被引用）
    header.html
    footer.html
  assets/
    fonts.css           # @font-face（生成物）
    fonts/*.woff2       # 自托管子集（生成物）
    tokens.css          # 设计令牌（D1 §令牌）
    base.css            # reset + 排版 + 布局原语
    site.css            # 页头/导航/页脚 + 共享组件
    editor.css          # 编辑器 / Infoview / 终端 / 事件流面板
    site.js             # 渐进增强（主题、复制、标签页、筛选、滚动高亮）
  data/                 # 全部生成物，见 §2
  favicon.svg           # 手写 SVG，无位图
  .nojekyll
```

**CSS 拆分理由**：`AGENTS.md` 硬规则 5（文件接近 ~500 行即拆分）。四份 CSS
在 HTTP/2 下是 4 个请求且跨页缓存，比一份 2000 行的文件更好维护。

### 3.2 导航单源机制（21 页不能手抄导航）

- `site/_partials/header.html` 与 `footer.html` 是**唯一源**，块内用
  `<!--#nav-->` … `<!--/#nav-->` 标记；
- `scripts/gen-site-nav.py --write` 把该块写进每个页面的同名标记之间，并按
  `<body data-page="X">` 给对应链接补 `aria-current="page"`（**不靠 JS**，
  无 JS 也正确）；`--check` 只报告不改；
- `scripts/check-site.py` 断言两边一致（等价于 `cargo fmt --check` 的角色）。

### 3.3 `check-site.py` 新增断言

| 断言 | 防的是什么 |
|---|---|
| 站内链接目标存在（**已有**） | 死链 |
| HTML 里不出现写死的 `0.x.y`（**已有**） | 版本漂移 |
| **导航/页脚块与 `_partials/` 逐字一致** | 21 页手抄漂移 |
| **每页 `data-page` 存在且与 `aria-current` 一致** | 高亮错页 |
| **每页有且只有一个 `<h1>`、有 `<title>`/`meta description`/`canonical`/`theme-color`/`og:*`** | 元数据缺失（旧站全缺） |
| **每页引用的 CSS/JS 都存在且体积在预算内** | 体积回归 |
| **不出现内联 `style=`**（除白名单） | 令牌旁路 |
| **不出现 `<img>` 位图**（favicon.svg 与 og 除外） | 假截图回流 |
| **每页含 `.turnstile` 或明确豁免** | 记忆点被稀释 |

### 3.4 体积预算（写进断言）

| 项 | 上限（原始） | 上限（gzip，实际传输） |
|---|---|---|
| 单页 HTML | 60 KB | 15 KB |
| `tokens.css` + `base.css` + `site.css` + `editor.css` | 合计 45 KB | 12 KB |
| `site.js` | 12 KB | 4 KB |
| `assets/fonts/*` 合计 | 120 KB | 120 KB（woff2 已压缩） |
| 单页首屏总字节（HTML+CSS+JS+字体） | 240 KB | 150 KB |

> 预算按**原始字节**断言（`check-site.py` 能直接量），gzip 一列是给人看的
> 真实传输量。GitHub Pages 对文本资源默认 gzip/brotli。

### 3.5 渐进增强纪律

- **无 JS 必须完全可读可导航**：折叠用 `<details>` 或 checkbox+label（R2 §Alectryon），
  标签页用 `:target`/radio，绝不靠 JS 渲染正文；
- `site.js` 只做**增强**：主题切换（默认跟随系统）、复制按钮、诊断表筛选、
  滚动高亮当前小节、`prefers-reduced-motion` 尊重；
- 零第三方脚本、零 CDN、零分析。

## 4. 事实红线（写页面时必须遵守）

来自 C4 §2.3 与 §7 的**实测警告**——写错就是造假：

1. **没有 `v0.57.0` tag**（6 个版本只有 CHANGELOG 条目没有 tag），而 0.57.0 正是
   「多文件 import + 项目管理」那个大版本 ⇒ 只能写功能，不能说有 tag。
2. 19 个版本的 CHANGELOG 日期比 tag 提交日期早一天 ⇒ 时间线**只选一种口径**并说明。
3. 2026-09-15 一天有 24 个 tag（0.29.0→0.48.0）⇒ 时间线必须聚合，不能铺开。
4. **不得写进网站**：10–100x 性能对比（那是目标表述，外部基准仍 opt-in）、
   用户数/下载量（无数据）、ROADMAP 里 10 单元时代的旧 golden 计数。
5. 结构债的行数**已经比 HANDOVER 登记值更大**（实测 `tests.rs` 7926 /
   `parser.rs` 4702 / `elab.rs` 4130 / `extension.js` 1880 / `lib.rs` 1610）
   ⇒ 引用实测值，不引用登记值。

## 5. 实施环节（每个环节 = 一个 subagent 任务 + 一次验证）

**纪律**（针对用户点名的"AI 比较急，为了完成完全忽略完美"）：
每个环节**只做一页或一件事**；完成后必须**自己跑验证命令并把真实输出贴回来**；
不通过就修，不许"基本完成"。**任何环节不得修改 `crates/`（内核冻结）**。

| 阶段 | 环节 | 交付 | 验证 |
|---|---|---|---|
| S0 | 令牌与骨架 | `tokens.css`/`base.css`/`site.css`/`editor.css`/`site.js`/`favicon.svg`/`_partials/` + `gen-site-nav.py` + `check-site.py` 扩展 | `check-site.py` 绿；`site-shot.py` 出图 |
| S0b | **设计检查点** | `styleguide.html`（每类组件至少一个真实用例） | 截图 → **交用户过目** |
| S1 | 样板页 | `kernel.html` | `check-site.py` 绿；截图 |
| S2 | 首页 | `index.html` | 同上 |
| S3 | 走查页 | `gen-site-lab.py` + `data/walkthrough.json` + `walkthrough.html` | 数据由真 CLI 产出；无 JS 可用 |
| S4 | 编辑器页 | `editor.html` | 截图三档宽度 |
| S5 | 语言页 | `language.html` | 每条语法样例都能在仓库里找到出处 |
| S6 | tactic 页 | `tactics.html` | 同上 |
| S7 | 诊断页 | `gen-site-lab.py` 扩展 + `data/diagnostics.json` + `diagnostics.html` | 码表与 `docs/protocol.md` 逐条对齐 |
| S8 | 为什么 | `why.html` | — |
| S9 | 愿景 | `vision.html` | 用户原话逐字 |
| S10 | 记法 | `notation.html` | — |
| S11 | 项目 | `projects.html` | — |
| S12 | Agent | `agents.html` | 命令可复制可执行 |
| S13 | 入门课 | `course.html` | 单元数来自 `site.json` |
| S14 | 卷 I | `set-theory.html` | 计数来自 `site.json` |
| S15 | 怎么学 | `learn.html` | — |
| S16 | 安装 | `get-started.html` | 命令逐条验证存在 |
| S17 | 文档 | `docs.html` | 链接全部存在 |
| S18 | 进展 | `gen-site-lab.py` 扩展 + `data/timeline.json` + `progress.html` | 遵守 §4 红线 |
| S19 | 工程 | `gen-site-lab.py` 扩展 + `data/gaps.json`/`evidence.json` + `engineering.html` | 数字可复现 |
| S20 | 关于 | `about.html` | — |
| S21 | 英文 landing | `en/index.html` | — |
| S22 | 收尾 | `pages.yml` 更新、`README.md`/`AGENTS.md`/`docs/README.md` 同步、删旧演示产物 | `scripts/soko gate` 绿 |
| S23 | 总验收 | 全站截图 + 体积报告 + 对比度报告 + 无 AI 味自查 | 全部断言绿 |
| S24 | 搜索 | `gen-site-lab.py` 扩展 + `data/search.json` + `search.html` | 索引覆盖全部页面标题与小节 |
| S25 | 对照页 | `compare.html`（从 Lean 4 / Coq / Agda 过来） | 每条差异能在 C1 §7 找到出处 |
| S26 | 术语表 | `glossary.html` | 每个术语指向它首次出现的页面 |
| S27 | 常见问题 | `faq.html`（C3 §7 的 24 行痛点表） | 每条排错都能追到 C3 的出处 |
| S28 | 版本历史 | `releases.html` + `data/timeline.json` | 遵守 §4 的三条时间线红线 |
| S29 | 站点文件 | `404.html` · `sitemap.xml` · `robots.txt` · `llms.txt` + 重写 `agent-prompt.js` | `check-site.py` 绿；`llms.txt` 内容与站点一致 |

**依赖**：S0 → S0b → 其余全部；S3/S7/S18/S19 共用 `gen-site-lab.py`，必须串行；
其余页面互不依赖，可并行，但**同一时间最多 2 个**（便于逐个审查质量）。

## 6. 收尾义务

- 更新 `STATUS.md`、`REQUIREMENTS.md` §9（用户新要求）、`docs/README.md`；
- `docs/design/site.md` 标注为**已被本文取代**（保留 git 历史）；
- `pages.yml` 的 `paths:` 增加 `scripts/gen-site-lab.py`、`scripts/gen-site-nav.py`、
  `scripts/gen-site-fonts.py`；
- `ROADMAP.md` §10 的 I12 追加 as-built 段。

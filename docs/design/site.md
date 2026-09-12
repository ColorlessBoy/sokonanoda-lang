# 项目官网（GitHub Pages）设计

> 触发（用户原话）：「我希望增加一个 github pages，相当于当前项目的官网，充分介绍本项目的
> 用法、远大目标和当前进展。」
>
> 现状：仓库 `ColorlessBoy/sokonanoda-lang` **尚未启用 Pages**（`GET /repos/.../pages`
> 返回 404），`.github/workflows/` 下只有 `ci.yml`、`release.yml`。

## 0. 一句话

官网 = **一个零构建的静态目录 `site/` + 一个部署 workflow**；页面上的「版本号 / 单元数 /
进展」**一律不手写**，全部由 `STATUS.md` / `Cargo.toml` / `course/course.json` /
GitHub Releases API 在 CI 里生成。官网是这些事实的**视图**，不是第五个事实源。

## 1. 为什么不能直接把 `docs/` 设为 Pages

| 问题 | 说明 |
|---|---|
| 会公开内部台账 | `docs/` 是 35 篇开发者文档，含 `CI-FAILURES.md`（失败与事故记录）、`LESSONS.md`（踩坑）、`design/`（内部取舍）。发布它等于把研发内账挂上公网 |
| Jekyll 隐式渲染 | 分支/jekyll 部署会把 md 渲染成 HTML，且不认 `README.md` 当首页（要 `index.md`），还会忽略 `_`/`.` 前缀文件——行为隐式、不可控 |
| 构建频率限制 | 目录部署有 **10 次构建/小时** 软限制，而 `docs/` 每轮都在改 |
| 与文档地图冲突 | 要在 `docs/` 放 `index.md`、`_config.yml`、`assets/`，污染 agent 的文档地图（`docs/README.md` 是分层权威） |

**结论：站点放新目录 `site/`，用 GitHub Actions 部署**（不受 10 次/时限制，且完全绕过 Jekyll）。

## 2. 托管方案决策

| 方案 | 构建依赖 | 维护成本 | 漂移风险 | 结论 |
|---|---|---|---|---|
| A **零构建手写静态 HTML/CSS**（`site/`） | 无 | 最低（编辑即提交） | 中（靠 §3 的生成机制压住） | ✅ **采用** |
| B 轻量静态生成器（Jekyll/MkDocs/mdBook/Hugo） | Ruby / Python / Go 之一 | 中（学模板 + 主题升级） | 高（把 `docs/` 的 agent 口吻直接渲染出去；多一层 md→HTML 漂移） | ✗ |
| C 重量级框架（Next/Docusaurus/Vite） | Node + 依赖树 + lockfile | 高 | 最高 | ✗ |

理由：站点内容是**介绍性长文**，不需要路由与组件化；而本项目有一条硬纪律
「用户/agent 路径零工具链依赖」（`REQUIREMENTS.md` §2 第 9 条）。官网是维护者产物，
严格说不受该条约束，但**引入一套新工具链会诱导后人「改进度先装工具」**。零构建方案下，
任何 agent 直接改 HTML 即可，与仓库气质一致。

> 代价要认：多页时导航/页脚会重复。内容量小（≤9 页）可接受；页脚用一段固定 HTML
> 片段复制，并靠 §5.3 的链接检查兜底。

## 3. 单一事实源：进展类内容**必须生成**

现状有**四个**进展口径并存：`STATUS.md`（权威流水账）、`ROADMAP.md` §9/§10、`docs/notes/gap-analysis.md`、
`editor/vscode/CHANGELOG.md` + GitHub Releases。官网若手写「进展」会立刻变成第五个漂移源
（本仓已有 6 处文档漂移实证，见 §5.1）。

**机制**：

1. `STATUS.md` 仍是唯一权威账本（`AGENTS.md` 已要求每轮更新它）；
2. 在 `STATUS.md` 顶部「## 一句话」之后加一个 **CI 校验的机器可读块**（fenced `json`），
   只放**稳定、机器可解析**的字段：
   ```json
   { "version": "0.17.0", "round": 39, "round_title": "…", "date": "2026-09-13",
     "tests": { "total": 507 } }
   ```
   人手/agent 只编辑这一个文件；块的内容由 CI 断言与 `Cargo.toml` 版本一致（避免块本身漂移）。
3. CI 的 pages workflow 里用 **python3（已具备，标准库）** 生成 `site/data/site.json`：
   - `version` ← `Cargo.toml` 的 `[workspace.package].version`（**不是**手写）；
   - `units` ← `course/course.json`（单元数、标题、是否英文镜像）；
   - `round / date / title` ← `STATUS.md` 的机器可读块；
   - `lesson files` ← `examples/*.sokonanoda` 列表。
4. **版本与下载链接在浏览器端查 GitHub Releases API**
   （`https://api.github.com/repos/ColorlessBoy/sokonanoda-lang/releases/latest`），
   页面加载后再填。这样 `README.md:72` 把下载示例硬编码成 `V=0.9.0`（实际 0.17.0）
   那类漂移**在机制上不可能发生**。

原则一句话：**官网永不手写「版本号 / 进展 / 单元数」三样。**

## 4. 信息架构

| 路径 | 目的 | 内容源 | 需新写？ |
|---|---|---|---|
| `index.html` | 一句话定位 + 三条价值主张 + 主 CTA | `docs/architecture.md` §1「30 秒版」、`editor/vscode/README.md` 首段、`docs/notes/research.md` §3 | 部分新写 |
| `get-started.html` | 三类人各一条路径：**学习者**（装 VS Code 扩展）/ **agent**（技能）/ **贡献者**（源码） | `editor/vscode/README.md`、`README.md`、`skills/README.md`、`AGENTS.md` §Setup | **重写**（现三类混排） |
| `course.html` | 6 单元导航 + 明确的「从这里开始」 | `course/course.json`、`course/README.md`、`playground.sokonanoda`、`examples/lesson-0*.sokonanoda` | 部分新写 |
| `vision.html` | 远大目标：要做什么、为什么这么做 | `ROADMAP.md` §0/§1/§2、`REQUIREMENTS.md` §1、`docs/notes/research.md` §3、`docs/architecture.md` §1 | **必须新写**（现有全是 agent 口吻，术语密集） |
| `progress.html` | 现状 + 时间线 + 路线图 | `STATUS.md`（+ `site.json`）、`ROADMAP.md` §10、`docs/notes/gap-analysis.md`、Releases API | **生成**（§3） |
| `agents.html` | 差异化卖点：给 code agent 的技能与协议 | `skills/README.md`、`AGENTS.md`、`docs/protocol.md`、`skills/sokonanoda-teacher/SKILL.md` | 部分新写 |
| `docs.html` | 深度文档入口（**只给链接，不复制正文**） | `docs/README.md` 的分层表 | 少量新写 |
| `about.html` | 上游来源与许可 | `NOTICE.md`、`README.md`、`editor/vscode/package.json`(license) | 少量新写 |
| `en/index.html` | 英文 landing（一期只做这一页） | `editor/vscode/README.md`（本就是英文门面） | 少量新写 |

### 4.1 「第一课」入口怎么做（不强求在线判题）

本仓是 Rust 原生二进制，**没有 WASM 构建**；浏览器内判题要把 `crates/kernel` +
`crates/front` 编到 WASM，是独立大工程。一期用零后端方案替代：

- 主 CTA：**装 VS Code 扩展**（平台包内嵌 LSP + CLI，这是本项目最强的零安装故事）；
- 「复制第一题」按钮：把 `examples/lesson-01.sokonanoda` 内容一键复制 + 三步图文；
- 「看它怎么判」：静态展示 `--json` 事件样例（源 `docs/protocol.md`）与 `unit1` 讲解片段；
- **WASM playground 单列 backlog**，不阻塞上线。

### 4.2 同类项目借鉴（调研产出，各一句）

| 项目 | 借鉴 |
|---|---|
| Lean 4 `lean-lang.org` | 首页先给**三条价值主张**，进展用**带日期的动态时间线** |
| Natural Number Game | 第一课必须是**一个明确的「从这里开始」按钮 + 分级单元网格** |
| Software Foundations | 首页给「入口卷」标签，明确阅读顺序 |
| Theorem Proving in Lean 4 | 每页底部「下一章 →」，降低教学路径迷路率 |
| Rocq `rocq-prover.org` | 首页直接展示**当前 release 号 + 日期 + 安装按钮**（我们用 Releases API 自动取） |
| Agda 文档站 | 反例：纯文档站缺第一课入口与目标叙事，教学项目不宜照抄 |

## 5. 落地前置：先修文档漂移（否则官网放大它）

### 5.1 已确认的漂移（调研产出，逐条可复现）

1. `README.md:72` 硬编码下载版本 `V=0.9.0`（实际 0.17.0）；
2. **课程单元数三套口径**：真实 6 个（`course/course.json` + `crates/cli/tests/course.rs` 的
   golden 含 `unit6`），但 `course/README.md:9` 写「五个」、`:51` 写 `unit = 1..5`，而同文件
   `:38` 又写「六个单元」——**文件内部自相矛盾**；`crates/cli/tests/course.rs:4` 注释仍写
   "five units"；
3. `ROADMAP.md` §8「当前只需要做一件事：执行 M0」、§9 清单、§10 的 I7「5 单元 / 顺序」均被
   「逻辑先行重排 + unit6」取代；
4. `docs/README.md:46` 把 `onboarding.md` 描述为依赖 `scripts/soko.sh`，该脚本已于
   2026-09-11 删除；`:51` 称 `decl-binders.md`「待实现」，实际已实现；
5. `docs/TESTING.md:199`、`docs/design/course-status.md:67`、`docs/teaching-session.md:11`、
   `docs/design/infrastructure.md:166/200` 仍写 5 单元；
6. `README.md` 的 Layout 段未列 `course/`、`playground.sokonanoda`、`crates/lsp`。

### 5.2 处置

- 这些**不是**本设计引入的，但官网的「课程 / 进展」页会直接引用它们；
- 因此把「修 §5.1 的 1–6」列为官网第 0 步（见 `ROADMAP.md` §10 的 I12-S0），
  与官网同批做，避免官网一上线就带着错数字。

### 5.3 防再次漂移

- **链接检查**（CI 步骤，python3 标准库）：解析 `site/**/*.html` 的站内 `href`/`src`，
  断言目标文件存在；同时断言引用的仓库内文档路径存在（对标 `skill.rs` 的
  `skill_referenced_repo_paths_exist`）。
- **数字来源检查**：断言 `site/**/*.html` 里不出现写死的形如 `0\.\d+\.\d+` 的版本号
  （除 `site/data/`），强制走 §3 的生成路径。
- 这两条都是「机制防漂移」，比人肉校对可靠——本仓的漂移史证明人肉校对不可靠。

## 6. 部署

### 6.1 新增 `.github/workflows/pages.yml`

```yaml
name: pages
on:
  push:
    branches: [main]
    paths: ["site/**", "STATUS.md", "course/course.json", "Cargo.toml",
            ".github/workflows/pages.yml"]
  workflow_dispatch:
permissions:
  contents: read
  pages: write
  id-token: write
concurrency:
  group: pages
  cancel-in-progress: false
jobs:
  deploy:
    runs-on: ubuntu-latest
    environment:
      name: github-pages
      url: ${{ steps.deployment.outputs.page_url }}
    steps:
      - uses: actions/checkout@v5
      - name: Generate site data (no new toolchain: python3 stdlib)
        run: python3 scripts/gen-site-data.py    # §3 的生成器
      - uses: actions/configure-pages@v5
      - uses: actions/upload-pages-artifact@v3
        with: { path: site }
      - id: deployment
        uses: actions/deploy-pages@v4
```

- 与现有 `ci.yml`/`release.yml` **互不影响**（三个独立 workflow；`release.yml` 顶部的
  `permissions: contents: read` 只作用自身）；
- `paths:` 过滤掉绝大多数提交，避免无意义部署。

### 6.2 一次性人工动作（**只能由仓库拥有者做**）

> Settings → Pages → **Source = GitHub Actions**。
> 不做这一步，`configure-pages` 会报 `Get Pages site failed. Not Found`。
> （本会话无法代做：需要仓库管理员身份，且工作区没有可用的 GitHub token。）

站点地址：`https://colorlessboy.github.io/sokonanoda-lang/`。自定义域名可后续在
Pages 设置里加 CNAME（GitHub 自动签发 Let's Encrypt TLS），一期不做。

### 6.3 配额（公开仓库）

站点 ≤1GB；带宽 100GB/月；部署超时 10 分钟。本仓 `.git` 打包体约 96 MiB，
站点内容远低于上限；Actions 部署不受「10 次构建/小时」限制。

## 7. 风险

| 风险 | 影响 | 缓解 |
|---|---|---|
| 照搬 `docs/` 台账文案 | 官网充满 agent 黑话（L0/I8/白名单/sorry 洞），普通人看不懂 | `vision.html` 必须新写；其它页面只从「对人友好」的源取（`architecture.md` §1、`editor/vscode/README.md`） |
| 手写进展/版本 | 立刻复制 6 vs 5 单元式漂移 | §3 的生成机制 + §5.3 的断言 |
| 双语变第二条漂移线 | 中英文不一致 | 一期只做**中文主站 + 一个英文 landing** |
| 官网成为新维护负担 | 每轮要改 5 个地方 | 页面只在「能力集变化」时改；进展/版本自动 |
| Pages 未启用导致 workflow 红 | 看起来像代码回归 | 上线前先做 §6.2；workflow 失败信息里写清「需先启用 Pages」 |

## 8. 分阶段计划

见 `ROADMAP.md` §10「I12 —— 项目官网」。验收标准与 subagent 切分在那里。

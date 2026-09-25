# 文档地图（docs/）

> 文档分三层：**仓库根**（入口与权威总账）→ **核心**（`docs/` 顶层，开发者
> 参考）→ **设计 / 笔记**（`docs/design/`、`docs/notes/`，长尾归档）。
> 接手项目先看仓库根 `AGENTS.md`，再按下方顺序读。

## 仓库根（入口与权威，与 `README.md`/`AGENTS.md`/`ROADMAP.md` 并列）

| 文档 | 作用 | 何时读 |
|---|---|---|
| `AGENTS.md` | **项目入口**：阅读顺序、硬规则速记、命令 | 第一份 |
| `README.md` | 对外门面（用户/agent 怎么用） | 对外 |
| `ROADMAP.md` | 里程碑与 §10 验收标准 | 规划 |
| `REQUIREMENTS.md` | **用户全部要求的权威总账**（硬规则、§9 追加日志） | 动手前必读；冲突以它为准 |
| `STATUS.md` | 当前进度与逐轮日志（最新在最上） | 每轮开始/收尾 |

> `HANDOVER.md` **不在仓库根**，它在 `docs/HANDOVER.md`（见下面「核心」表）。
> 站点索引页 `site/docs.html` 的作者核出过这处错——本文早先把它列在根目录，
> 那个路径会 404。

## 核心（`docs/` 顶层，开发者参考）

| 文档 | 作用 | 何时读 |
|---|---|---|
| `HANDOVER.md` | **交接汇总**：现在在哪、还剩什么、怎么继续（TODO/限制/gotchas 索引） | 接手第一份 |
| `architecture.md` | 流水线、内核机制、§6 内核改动清单、§8 gotchas | 改内核/front 前 |
| `protocol.md` | `--json` 事件、`soko/*` 自定义请求的对外契约 | 改事件/输出格式前 |
| `TESTING.md` | 测试地图（哪类改动跑哪层） | 加测试时 |
| `RELEASE.md` | 发布手册（main 全绿自动 tag、8 平台 + 9 VSIX、Marketplace） | 发版前 |
| `STATUS-ARCHIVE.md` | STATUS 的历史轮次归档（第 1–94 轮原文） | 查旧轮/缺陷修复时间线 |
| `vscode-dev-guide.md` | VS Code 扩展开发规范（版本纪律、测试三层、常见坑） | 改 `editor/vscode/` 前 |
| `LESSONS.md` | 经验台账（subagent/流程教训） | 接手/复盘 |
| `PERF.md` | 性能测试结构、阈值原则与基线 | 改动涉及热路径/验收 |
| `E2E.md` | **真 VS Code 集成测试的例行化**（`scripts/vscode-e2e.sh`、`docs/e2e/` 台账、`SOKO_E2E_LOG` 判读） | 改 `editor/vscode/` 后；真宿主回归 |
| `CI-FAILURES.md` | CI 失败台账（原因/修复/预防） | CI 红时；同类不二犯 |
| `teaching-session.md` | 教学循环与解答钥匙（agent 老师用） | 讲课时 |

## 设计记录（`docs/design/`）

已确认并落地的设计（含取舍、验收、as-built）。新功能先在这里加一篇，再动手。

- `infrastructure.md` — LSP-first 总体设计 v2
- `i8-i9.md` — 真增量（Session/TrustPlan）+ goal 视图第一段
- `goal-refine.md` / `round14.md` — 多洞/refine、hole_id、recursor 自动派生、spine meta 路线
- `goal-func-spine.md` — 函数实参洞 + hover 开项 pp 修复（第二十四轮）
- `hints-suggestions.md` — 提示阶梯 + 下一步建议
- `hover-brackets.md` / `hover-refactor.md` — 括号 hover、良构表达式 + range
- `rename-inlay.md` — rename / find-references / inlay hints / `sokonanoda lsp`
- `by-tactics.md` — `by` tactic 块 + 编辑器 goal-state
- `kernel-taxonomy.md` — 内核错误分类学 + 失败建议 + 基准/fuzz
- `course-status.md` — 课程地图 + REPL 历史
- `course-bilingual.md` — 课程英文镜像
- `bundled-lsp.md` — 插件自带 LSP 二进制（per-target VSIX + 版本锁定下载）
- `onboarding.md` — opencode 启动插件 + `sokonanoda` 二进制子命令的零脚本接入（`scripts/soko.sh` 已删除）
- `reserved-decl-warning.md` — 声明名撞内核已定义名字（`Prop`/`Sort`/`Type`）的 warning 通道
- `type-level-syntax.md` — `Type n`（= `Sort (n+1)`）记法解析糖
- `binary-cli.md` — 环境能力进 `sokonanoda` 二进制子命令（内嵌下载器），删除 `scripts/soko.sh`
- `term-intro.md` — 值位关键字设计（**已废弃**：`funintro` 于 0.27.0 移除，见 `remove-funintro.md`）
- `term-apply.md` — 值位 `funapply` 关键字（**已废弃**：0.22.0 移除）
- `value-keywords-v2.md` — 值位关键字 v2（**已废弃**：`funintro`/`funapply` 均已移除）
- `remove-funintro.md` — 移除值位关键字 `funintro`（0.27.0）
- `goal-list.md` — 多目标显示：`by` 每步记录全部剩余目标 + 协议 `goals[]`（0.27.0）
- `tactic-hover.md` — tactic 关键字高亮 + hover 中间 goal state（0.27.0）
- `elaborator-let-match.md` — elaborator `let`（Phase 1）/ `match`（Phase 2）设计（I6）
- `spine-meta-a.md` — refine 子洞的 kernel 级期望类型（方案 A，I9 余项）
- `webview-infoview.md` — VS Code webview goal 面板（方案 B）
- `course-syllabus.md` — 课程大纲重构：调研综合 + 3 套候选大纲 + 推荐与迁移计划（设计，2026-09-16）
- `highlighting.md` — 高亮分类单一起源（kind→TM scope/CSS/LSP token 表 + 平台限制，0.49.0）
- `compile-cache.md` — 编译结果落盘缓存（olean 式，LSP 打开免重编，0.48.0）
- `goal-rendering.md` — 统一 goal 呈现（结构化 tag + 单一分类源）+ Infoview 落右侧 + fallback 修复（设计）
- `match-patterns.md` — `match` 模式编译器（通配/嵌套/Nat 字面量/Bool 守卫，0.42.0）
- `match-dependent-motive.md` — `match` 依赖 motive（结果类型随 scrutinee 变化，0.39.0）
- `parameterized-inductives.md` — 非带索引参数化归纳（`Option A`/`List A`，0.38.0）
- `indexed-inductives.md` — 带索引归纳（`Vec A n`：声明 + 派生 recursor + match，0.47.0）
- `match.md` — 值位 `match` 总设计（Phase 1–6，含 as-built）
- `early-cutoff.md` — I8 依赖精确化：conservative early-cutoff 签名比较（0.32.1）
- `extension-server-policy.md` — VS Code 扩展强制内置 LSP + `sokonanoda: doctor` 自检（0.31.0）
- `compiler-service-events.md` — 编译器服务事件流（`file.didChange` 等，L1/L3）
- `real-input-tests.md` — 真人输入测试体系 + 四写法共存风险矩阵（**已废弃**：`char_steps` 基建随值位关键字一并删除）
- **`site-single-page.md`** — **官网（GitHub Pages）当前权威（2026-09-21）**：单页站点
  （是什么 / 怎么安装 / 核心特点 / 未来的计划）、`site/` 的文件清单、留在里面的三条防漂移机制、
  以及"为什么把 28 页砍成 1 页"。验收：`python3 scripts/check-site.py`（10 项，exit 0 才算过）
- `site.md`、`site-rebuild/` — **历史存档（已被 `site-single-page.md` 取代）**：
  2026-09-20 的 28 页全面重构（`STATE.md` 的实测修正清单仍然有效；`spec/D1-design-rules.md`
  的设计主张被单页版**原样继承**，色值与令牌未改）。**不要照着它们新建页面**——
  导航生成器 / 搜索索引 / 走查数据 / 诊断码表页都已随简化删除。
- `decl-binders.md` — 声明级 binder（Lean 风格）设计（已实现，0.15.0 发布）
- `deepseek-harness.md` — **DeepSeek Harness 适配（设计 + 计划 H0–H4）**：差距
  G1–G10、DSH 侧事实（技能根/斜杠命令/LSP 只有 4 项只读操作且忽略诊断/patch 形状）、
  验收 A1–A6 与待拍板决策 D-1…D-6（2026-09-17，H0–H4 已落地）
- `agent-query-channel.md` — **内核真相查询通道（设计 + 计划 H6-A…E）**：
  把真相从 LSP 抽成 `front::query`，再上 CLI `query` 与 MCP 两个薄传输；
  含两个 front 缺口（索引递归 `Prop` 的 recursor、多名字 binder 组）的改挂与修法
  （2026-09-17，实现未开始；ROADMAP I15）
- `imports-and-projects.md` — **多文件 `import` 与项目管理（调研 + 设计 + 计划 + as-built I16，0.57.0 已落地）**：
  10 个语言/证明助手的"单文件 vs 项目"横向调研、`import` 置顶语法与 Lean 同款
  模块名规则、`sokonanoda.toml` 项目根、闭包编译与闭包哈希缓存、CLI/LSP/query
  表面与第 11 单元教学计划（2026-09-17，实现未开始；ROADMAP I16）
- `redundant-sorry.md` — **多余的 `sorry`（用户实测反馈，已落地）**：值位里"学生
  已写完、只留了一行 `sorry`"被误报成"练习尚未解决"；候选规则 = 实参超出望远镜
  且结果展不开箭头，终审 = kernel（删掉该实参后整条声明能过）。§8 记录了那个
  5 分钟实验：**真根因不是 `NamePtr` 身份，而是 `EnvLimit::ByName(探针名)` 取到
  `NO_DECL` ⇒ cutoff 0 ⇒ 空环境**；修法是内核**只加不改语义**的
  `check_declar_at`/`try_check_declar_at` + front 传 `ByIndex(env_before)`
  （2026-09-17，第九十一轮续落地；三层验收见 §8.4）
- `teaching-project.md` — **第二大课「从集合论到分析」总体计划**（2026-09-18，
  只出计划；**2026-09-19 / 0.59.0 已对账**）：靶子标定（analysis 的规模与可移植项）、
  集合论可行性实测（11 声明全绿）、**24 条课程驱动缺口**的分诊（附录 A 逐条标 0.59.0
  状态）、§6 **缺口台账协议**（`docs/gaps/` 台账 + 最小复现 + 工作单 WO + 「缺口即测试」，
  已接进门禁）、分期 P0–P7（P1/P2 ✅）、验收与待拍板 D-1…D-6
- `course-stdlib.md` — **课程标准库的分层与「消除暴力」方案**（2026-09-18）：L1 prelude /
  L2 课程标准库 / L3 单元练习的三层判据（"Mathlib 有且没有数学内容才归库"）、
  L1 的 16 条清单与落地要求、L2 规范、与语言缺口的关系、落地顺序 P-C1…P-C5
- `set-theory-syllabus.md` — **卷 I《集合论》十二单元大纲（锁定）**：教材取证综合
  （Hammack/Macbeth/Avigad/Velleman/Solow/Cummings 的真实目录）、证明助手先例
  （MIL/MoP/Logic and Proof/FM/LPA）、学习障碍、每单元"必证/必破"、记法引入顺序
  与无记法替代、与缺口台账的联动（2026-09-18）
- `prop-large-elim-mirror.md` — **派生 recursor 的 large-elimination 判据逐字镜像内核**
  （G-03 / WO-006，0.59.0 落地）：`inductive Bar (A : Type) : Prop` + `ctor mk (a : A)`
  曾被内核断言拒绝（`left:1/right:0`）⇒ `Exists` 只能立成公理；根因是前端
  `small_elim` 的源码近似，修法是把它推迟到构造子 elaborate 之后并镜像
  `large_elim_test`（"字段是不是 Prop 值"问真内核）；顺带修掉 `judge_infer`
  取第一条 `TypeChecked` 的既有 oracle bug；**内核零改动**（2026-09-19）
- `ctor-namespace.md` — 构造子进入类型的命名空间（G-02 / WO-005，0.59.0 落地）：
  规范名 `Ind.ctor` + 裸名解析别名、`elab-ambiguous-ctor-alias`、归约形态实测；
  §"基线口径订正"记录了课程门禁的**实测**基线 315 checked · 96 open（2026-09-19）
- `notation-aware-printing.md` — **goal / 类型行用记法**（线 C）：§1 是**实测表**
  （四个生产者 × unit01/08/12 + 解答，逐格给出 CLI 命令与实际文本）；
  三条结论：光标在不在 tactic 上决定走哪一支（学习者的光标就在 tactic 上）、
  `apply` 之后子目标走的是 pp 望远镜（`∈`/`↔` 一起消失）、同一份声明在两个
  surface 上文本不同；§1.4 定靶（改 #1/#3，不动 `render_expr` 与内核 pp）。
  §2 是**消费者审计**（五个字段 × 全部消费者，逐条 `file:line`；结论：`ty_text`
  只给人看 ⇒ 可就地改，`goal`/`binders[].ty` 同时喂 judge ⇒ 只能在显示出口重写）。
  §3 是**权威设计**（front 侧显示边界重写 = K3-a）：为什么不走内核 pp（记法打印是
  死代码 + `pp_expr` 是 `#check` 出口）、**arity 硬规则**、落点与红线、
  `DisplayText` 编译期护栏（2026-09-21）
- `notation-subset.md` — **用户自定义记法子集**（G-04 / WO-011 第一刀，0.59.0 落地）：
- `notation-display.md` — **用户自定义记法子集**（G-04 / WO-011 第一刀，0.59.0 落地）：
  `infix:N`/`infixl:N`/`infixr:N`/零元 `notation` 四条命令、数学符号独立 token 的
  码点类、优先级梯子（`p`/`p+1`、`p+1`/`p`）、elab 内**源到源**展开 + 自动补前导
  类型参数（裸变量匹配，不引入元变量）、文件内作用域、记法**不是声明**（零事件）、
  兼容护城河（点名省 `α` 仍被拒）、与 Lean 的 7 条已知差异、第二刀清单；
  §9 是 as-built（`∅ ⊆ A` 逼出的"操作数也吃期望类型"等八条）（2026-09-19）
- `course-lean-style.md` — **全课程 Lean 4 化（记法符号 + tactic 证明）主计划**
  （2026-09-19，**进行中**）：用户拍板 D1–D6、现状实测台账 **X1–X15**（每条都是真二进制跑出来的，
  含四个前端 bug 的根因定位到行）、语言侧 L1–L4 / 课程侧 C1–C7 / 同步 F 工作项、
  分期 **R1 语言地基 → R2 引擎扩展 + 卷 I 全量 → R2.5 记法输入 + 隐式实参 → R3 入门课 + 收尾**
  → R4（可选）print-back、subagent 分工 S1–S10、§9 逐轮 as-built、§10 明确不做 N-1…N-12。
  调研底稿九篇在 `docs/notes/course-lean-style/`；**红线段：内核零改动**
- `notation-input.md` — **记法输入法（`\xxx` 缩写）+ hover 提示**（2026-09-19，配套 D5）：
  19 个符号的 **Lean 逐字缩写表**（含别名与 `supported` 标记）、四条输入路线对比
  （**推荐客户端缩写改写器 + Tab**，不做 LSP 补全——DSH/opencode 都不消费补全项，
  而 DSH 的 `lsp` 工具有 hover）、hover 落点与「插在关键字闸门之前」的关键约束、
  一个**阻断级 LSP 缺陷**（单文件 parse 失败吃掉项目报告）、P0–P3 分期与 R-1…R-7 风险
- `implicit-arguments.md` — **隐式实参**（2026-09-19，配套 D6；**推翻原 D2 的「不做」**）：
  三条路线对比（A 真元变量被内核堵死 / B 探针每次整前缀重编译 / **C 风格对齐 + 唯一确定**，
  400–600 行、零额外内核调用）、算法与六个落点文件、**关键安全性质**（无隐式 binder 的签名
  逐字节 no-op ⇒ 可独立发布且课程零改动全绿）、课程分批 B0–B7、会红的测试与**护城河契约变更**
  清单、顺带发现的两个真 bug（X14/X15）、P0–P4 分期

> 设计文档是**已落地决策的存档**（as-built）。被后续轮次取代的细节以
> `STATUS.md` 为准；确认过时且无人引用的会直接删除（保留 git 历史）。

## 调研与笔记（`docs/notes/`）

- `research.md` — 教学型形式化证明语言与基础设施调研
- `lsp-notes.md` / `vscode-notes.md` — LSP / VS Code 接入实践调研
- `gap-analysis.md` — 业内标准差距审计
- `dsh-project-assets.md` — **DeepSeek Harness 源码勘察记录**（技能根/斜杠命令/
  LSP 能力/patch 形状/hooks/子 agent/客户端插件能否被项目自带，逐条 `path:line`；
  配套设计 `docs/design/deepseek-harness.md`）
- `inductive.md` — `inductive`/`ctor`/`rec`/`iota` 讲解
- `rust-cross-platform-binary.md` — 为什么跨 OS 没有单一 Rust 二进制、引导器（`soko.sh`/插件）的角色
- `multifile-prior-art.md` — **多文件/项目模型的横向调研**（Coq/Rocq、Agda、Isabelle、
  Idris 2、Rust、Go、Python、JS/TS、Haskell/OCaml、JVM：单文件模式、清单发现、模块身份、
  产物与失效；配套设计 `docs/design/imports-and-projects.md`，I16）
- **项目状态视图**（`query project` / `soko/project` / VS Code 项目树：
  根、清单来源、闭包模块表、每模块状态与项目诊断；0.58.0 批次 4）
  —— 文档在 **`docs/design/project-view.md`**，不在 `docs/notes/` 下
  （站点索引页 `site/docs.html` 的作者核出过这处错：`docs/notes/project-view.md` 不存在，
  在任何 git 历史里也不存在）。
- **`course-lean-style/`** — **全课程 Lean 4 化的九篇调研底稿**（2026-09-19，配套设计
  `docs/design/course-lean-style.md`）：`notation-audit.md`（记法能力审计 903 行）、
  `tactic-audit.md`（tactic 能力审计 767 行）、`course-inventory.md`（卷 I 逐声明清单 1701 行）、
  `tooling-impact.md`（门禁/工具/文档影响面 657 行）、`printback-feasibility.md`（print-back 614 行）、
  `intro-course-constraints.md`（入门课结构性约束 406 行）、
  `implicit-args-plan.md`（隐式实参引擎侧 1027 行）、
  `notation-input-plan.md`（记法输入面 461 行）、
  `course-impact-implicit-args.md`（隐式实参课程侧影响面 659 行）。
  **每篇都带 `文件:行号` 证据与实测探针**（"结论不是读代码猜的"）。
  另有 **`R2-rewrite-brief.md`**——R2 改写轮次发给 subagent 的**施工说明书**
  （记法表 / tactic 白名单 / `⟨a, b⟩` 与多层展开 / 已知引擎边界 / lib 与入门课的
  额外规则 / 判卷命令 / 阻塞上报格式）。它不是调研，是操作手册；
  改课程前先读它，能省一轮返工。
  **`R3-rewrite-brief.md`**——入门课 `course/` 那一刀的施工说明书（CN/EN 同步、
  本课可用的记法集、**本课的 tactic 白名单**（不含 `constructor`/`cases`——骨架是
  自建 `axiom`）、**计数纪律**（纯记法改写 count-neutral）、单元④ 的特例、阻塞上报格式）。
  样板 = **单元①**（已按它改完并通过全部门禁）；§9 另有**subagent 半途停掉后的接管
  记录**——「没有收尾消息 ≠ 没改文件」，验收要看残留扫描而不是自述。
  **`R2-full-rewrite-brief.md`**——**卷 I `courses/set-theory/` 的收尾手册**
  （2026-09-21 第 114 轮建）：§0 逐区域现状表（哪些是机械替换已做、哪些还没）、
  §1 剩下的三类活（`Exists X (fun …)` → `∃ …`、跨行 `And` → `∧`、项模式证明 → `by`）、
  §2 纪律（门禁必须始终 328/99/0、`notation-cheatsheet` 故意并列两种写法**不许动**）、
  §3 交付格式。**背景**：卷 I 的第 110–112 轮只交付了「语言地基 + 单元② 试点」，
  「36 目标全绿」被误记成「全量改写完成」——门禁只证明**能判卷**，不证明**改写过**。
- `lean-style-0.62.md` — **全课程 Lean 4 化（0.62.0 批次）的"给站点/文档 agent"事实清单**：
  这一批用户可见的 12 项特性（记法 / tactic / 隐式实参 / 记法输入 / 判定侧修复 / 新诊断码）、
  课程内容的事实变化（两门课 + playground 的当前计数、入门课删自建 `And`/`Or` 骨架的后果）、
  以及**站点不该误解的三件事**（G-21 的报错仍半修、记法在实参位的边界、记法对照页的双写法是故意的）。
  ⚠️ 它明确标注了"工作树 = 未发布 0.62.0"，并指向 `docs/design/site-rebuild/STATE.md` #13 的测量陷阱；
  **不要**把它当已发布事实，除非 0.62.0 已发。
- `project-roots-and-incremental-caches.md` — **语言服务器根发现 + 增量缓存调研**
  （LSP 契约、rust-analyzer/clangd/tsserver/pyright/gopls/ocaml-lsp/Agda/lean4 的根发现与
  错根症状、Lake trace / GHC 指纹 / OCaml `.cmi` / Coq `.vo` digest / `.tsbuildinfo`、
  "缓存判定结果安全吗"的三条规则）
- `docs/gaps/spike/README.md`（见 `docs/gaps/README.md`）— **卷 I 试做稿**：2 个单元 + 66 条标准库，
  全部真内核判卷（0 failed），逐条标出 `L-xx` 标准库欠账
- `settheory-survey/` — **集合论教学调研**（2026-09-18，教学项目 P0 的两路调研，共 ~2,200 行）：
  `set-theory-teaching-survey.zh.md`（教材顺序之争：集合/逻辑、有序对、幂集、关系 vs 函数、
  基数 vs 选择、Russell 六组取证 + 推荐十单元）、`prior-art-report.md`（证明助手先例：
  MIL/MoP/L&P/FM/LPA、**`djvelleman/stg4` 集合论游戏 8 世界 51 关**、analysis §3 逐节解剖）、
  `proof-book-tocs.md`（Hammack/Macbeth/Avigad/Velleman/Solow/Cummings 的真实目录取证 +
  五处前提勘误）、`lean4-sets-functions-prior-art.md`（长版底稿）、
  **`learning-difficulties.md`（学习障碍实证：2874 行 / ~190 条来源、逐条核验级别
  [F]/[A]/[M]；含两条实测的否定结果——有序对与选择公理没有任何实证研究）**、
  `repro/`（内核实测：`lib.sokonanoda` 29 checked / 0 failed；调研探针 P1 产出台账 G-13）。
  结论已综合进 `docs/design/set-theory-syllabus.md`

## 关联目录

- `ROADMAP.md`（仓库根）— 里程碑与 §10 验收标准
- `AGENTS.md`（仓库根）— agent 入口 + 硬规则速记
- `skills/` — 角色技能（`sokonanoda-teacher` / `-dev` / `-ci`）
- `course/` — 入门课素材库（11 单元，agent 用，非用户直接消费）
- **`courses/set-theory/`** — **卷 I《集合论》**（第二大课，已建）：`lib/`（L2 课程标准库）+
  `units/`（画布与解答）+ `gaps/`（发现端）+ `tools/check.py`（本地门禁）；
  入口 `courses/set-theory/README.md`，判卷 `python3 courses/set-theory/tools/check.py`
- `docs/gaps/` — **课程驱动的缺口台账**（`ledger.jsonl` + `repro/` 最小复现 +
  `WO-*.md` 工作单）；协议见 `docs/design/teaching-project.md` §6
- `playground.sokonanoda`（仓库根）— 共享教学画布
- `scripts/install.sh` — 终端用户零 cargo 安装器（版本锁定 Release 资产，
  见 `docs/design/onboarding.md` §5）
- `scripts/new-course-repo.sh` — **生成「独立课程仓」骨架**（教学项目 P0.0；生成器留在
  语言仓是因为它编码版本钉约定，见 `docs/design/teaching-project.md` §3.5）
- `skills/` — 角色技能；**DeepSeek Harness 通过 skill 名即斜杠命令直接消费**
  （`/sokonanoda-teacher` 等），适配计划见 `docs/design/deepseek-harness.md`
- `.devcontainer/` — 仅贡献者的 Rust 容器（终端用户无需 Rust）

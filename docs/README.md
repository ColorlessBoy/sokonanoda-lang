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
| `HANDOVER.md` | **交接汇总**：现在在哪、还剩什么、怎么继续（TODO/限制/gotchas 索引） | 接手第一份 |

## 核心（`docs/` 顶层，开发者参考）

| 文档 | 作用 | 何时读 |
|---|---|---|
| `architecture.md` | 流水线、内核机制、§6 内核改动清单、§8 gotchas | 改内核/front 前 |
| `protocol.md` | `--json` 事件、`soko/*` 自定义请求的对外契约 | 改事件/输出格式前 |
| `TESTING.md` | 测试地图（哪类改动跑哪层） | 加测试时 |
| `RELEASE.md` | 发布手册（main 全绿自动 tag、8 平台 + 9 VSIX、Marketplace） | 发版前 |
| `STATUS-ARCHIVE.md` | STATUS 的历史轮次归档（第 1–65 轮原文） | 查旧轮/缺陷修复时间线 |
| `vscode-dev-guide.md` | VS Code 扩展开发规范（版本纪律、测试三层、常见坑） | 改 `editor/vscode/` 前 |
| `LESSONS.md` | 经验台账（subagent/流程教训） | 接手/复盘 |
| `PERF.md` | 性能测试结构、阈值原则与基线 | 改动涉及热路径/验收 |
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
- `site.md` — 项目官网（GitHub Pages）方案与信息架构（已上线：site/ + pages.yml）
- `decl-binders.md` — 声明级 binder（Lean 风格）设计（已实现，0.15.0 发布）
- `deepseek-harness.md` — **DeepSeek Harness 适配（设计 + 计划 H0–H4）**：差距
  G1–G10、DSH 侧事实（技能根/斜杠命令/LSP 只有 4 项只读操作且忽略诊断/patch 形状）、
  验收 A1–A6 与待拍板决策 D-1…D-6（2026-09-17，H0–H4 已落地）
- `agent-query-channel.md` — **内核真相查询通道（设计 + 计划 H6-A…E）**：
  把真相从 LSP 抽成 `front::query`，再上 CLI `query` 与 MCP 两个薄传输；
  含两个 front 缺口（索引递归 `Prop` 的 recursor、多名字 binder 组）的改挂与修法
  （2026-09-17，实现未开始；ROADMAP I15）

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

## 关联目录

- `ROADMAP.md`（仓库根）— 里程碑与 §10 验收标准
- `AGENTS.md`（仓库根）— agent 入口 + 硬规则速记
- `skills/` — 角色技能（`sokonanoda-teacher` / `-dev` / `-ci`）
- `course/` — 课程素材库（agent 用，非用户直接消费）
- `playground.sokonanoda`（仓库根）— 共享教学画布
- `scripts/install.sh` — 终端用户零 cargo 安装器（版本锁定 Release 资产，
  见 `docs/design/onboarding.md` §5）
- `skills/` — 角色技能；**DeepSeek Harness 通过 skill 名即斜杠命令直接消费**
  （`/sokonanoda-teacher` 等），适配计划见 `docs/design/deepseek-harness.md`
- `.devcontainer/` — 仅贡献者的 Rust 容器（终端用户无需 Rust）

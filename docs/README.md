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

## 核心（`docs/` 顶层，开发者参考）

| 文档 | 作用 | 何时读 |
|---|---|---|
| `architecture.md` | 流水线、内核机制、§6 内核改动清单、§8 gotchas | 改内核/front 前 |
| `protocol.md` | `--json` 事件、`soko/*` 自定义请求的对外契约 | 改事件/输出格式前 |
| `TESTING.md` | 测试地图（哪类改动跑哪层） | 加测试时 |
| `RELEASE.md` | 发布手册（tag 触发、8 平台 + 9 VSIX、Marketplace） | 发版前 |
| `vscode-dev-guide.md` | VS Code 扩展开发规范（版本纪律、测试三层、常见坑） | 改 `editor/vscode/` 前 |
| `LESSONS.md` | 经验台账（subagent/流程教训） | 接手/复盘 |
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
- `onboarding.md` — `scripts/soko.sh` 单一环境入口
- `reserved-decl-warning.md` — 声明名撞内核已定义名字（`Prop`/`Sort`/`Type`）的 warning 通道
- `type-level-syntax.md` — `Type n`（= `Sort (n+1)`）记法解析糖
- `binary-cli.md` — 环境能力进 `sokonanoda` 二进制子命令（内嵌下载器），删除 `scripts/soko.sh`
- `term-intro.md` — 值位 `intro` 关键字 + VS Code 展开补全（已实现）

> 设计文档是**已落地决策的存档**（as-built）。被后续轮次取代的细节以
> `STATUS.md` 为准；确认过时且无人引用的会直接删除（保留 git 历史）。

## 调研与笔记（`docs/notes/`）

- `research.md` — 教学型形式化证明语言与基础设施调研
- `lsp-notes.md` / `vscode-notes.md` — LSP / VS Code 接入实践调研
- `gap-analysis.md` — 业内标准差距审计
- `inductive.md` — `inductive`/`ctor`/`rec`/`iota` 讲解
- `rust-cross-platform-binary.md` — 为什么跨 OS 没有单一 Rust 二进制、引导器（`soko.sh`/插件）的角色

## 关联目录

- `ROADMAP.md`（仓库根）— 里程碑与 §10 验收标准
- `AGENTS.md`（仓库根）— agent 入口 + 硬规则速记
- `skills/` — 角色技能（`sokonanoda-teacher` / `-dev` / `-ci`）
- `course/` — 课程素材库（agent 用，非用户直接消费）
- `playground.sokonanoda`（仓库根）— 共享教学画布

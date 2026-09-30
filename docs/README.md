# 文档地图（docs/）

> 文档分四层：**仓库根**（入口与权威总账）→ **核心**（`docs/` 顶层，开发者
> 参考）→ **设计 / 笔记**（`docs/design/`、`docs/notes/`）→ **归档**
> （`docs/archive/`，历史层 —— **不进"活文档"预算** ✗，见文末「归档」一节）。
> 接手项目先看仓库根 `AGENTS.md`，再按下方顺序读。
>
> **预算与判据** ✓：`python3 scripts/docs-lint.py`（**判据 ①–⑦**：活文档 **≤10.0 MB** ·
> 单文件 ≤2000 行 · 入口 ≤800 行 · 新设计 ≤150 行 / 既有按 `scripts/docs-budget.json` **冻结** ·
> `docs/**` 禁垃圾 · **归档必须被索引点名** · **⑦ 接手路径**：`docs/ONBOARDING.md` 的必读表
> **每文件 + 合计**都有上限，**超标判红** ✓）—— 设计 `docs/design/docs-diet.md` ✓，
> 已进 `scripts/soko gate` 与 CI ✓。
>
> **接手先看 `docs/ONBOARDING.md`** ✓（X1，2026-09-28）：那份表是「读多少才能开工」的**唯一权威**
> （**别抄这里的数字** ✗ —— 判据是 `python3 scripts/docs-lint.py` 判据 ⑦，数字以它为准）。

## 仓库根（入口与权威，与 `README.md`/`AGENTS.md`/`ROADMAP.md` 并列）

| 文档 | 作用 | 何时读 |
|---|---|---|
| `AGENTS.md` | **项目入口**：阅读顺序、硬规则速记、命令 | 第一份 |
| `README.md` | 对外门面（用户/agent 怎么用） | 对外 |
| `ROADMAP.md` | 里程碑 + §10 待办的**验收口径与收口状态**（**队列不在这里**） | 规划 |
| `REQUIREMENTS.md` | **用户全部要求的权威总账**（硬规则、§9 追加日志） | 动手前必读；冲突以它为准 |
| `STATUS.md` | 当前快照 + **最近 3 轮** + 未决项（最新在最上；旧轮见 `STATUS-ARCHIVE.md` / `docs/archive/`） | 每轮开始/收尾 |

> **计划 / 队列的唯一入口是 `docs/ONBOARDING.md`**（开工单 §0.2）✓ —— `STATUS.md` 与
> `ROADMAP.md` §10 **不再各写一份"下一步"**（2026-09-30 收敛）；历史交接书已于 2026-09-29
> **删除（不归档）**，内容并入该文件 ✓。
> 站点索引页 `site/docs.html` 的作者核出过这处错——本文早先把它列在根目录，
> 那个路径会 404。

## 核心（`docs/` 顶层，开发者参考）

| 文档 | 作用 | 何时读 |
|---|---|---|
| `ONBOARDING.md` | **开工单**：现在在哪 / **唯一队列（§0.2）** / 判据 / 纪律 + **计划·提案类文档索引（§2）**；历史交接书已于 2026-09-29 删除（内容并入本文件） | 接手第一份 |
| `architecture.md` | 流水线、内核机制、§6 内核改动清单、§8 gotchas | 改内核/front 前 |
| `protocol.md` | `--json` 事件、`soko/*` 自定义请求的对外契约 | 改事件/输出格式前 |
| `TESTING.md` | 测试地图（哪类改动跑哪层） | 加测试时 |
| `RELEASE.md` | 发布手册（main 全绿自动 tag、8 平台 + 9 VSIX、Marketplace） | 发版前 |
| `STATUS-ARCHIVE.md` | STATUS 的历史轮次（**只留最近 12 段** ✓；`STATUS.md` 移出的段落 ⇒ `docs/archive/`，更早的 ⇒ `git log --all -- docs/STATUS-ARCHIVE.md`） | 查旧轮/缺陷修复时间线 |
| `vscode-dev-guide.md` | VS Code 扩展开发规范（版本纪律、测试三层、常见坑） | 改 `editor/vscode/` 前 |
| `LESSONS.md` | 经验台账（subagent/流程教训） | 接手/复盘 |
| `PERF.md` | 性能测试结构、阈值原则与基线 | 改动涉及热路径/验收 |
| `E2E.md` | **真 VS Code 集成测试的例行化**（`scripts/vscode-e2e.sh`、`docs/e2e/` 台账、`SOKO_E2E_LOG` 判读） | 改 `editor/vscode/` 后；真宿主回归 |
| `CI-FAILURES.md` | CI 失败台账（原因/修复/预防；**只留最近 15 条** ✓，更早 ⇒ `git log --all -- docs/CI-FAILURES.md`） | CI 红时；同类不二犯 |
| `teaching-session.md` | 教学循环与解答钥匙（agent 老师用） | 讲课时 |

## 设计记录（`docs/design/`）

已确认并落地的设计（含取舍、验收、as-built）。**2026-09-30 激进删档后只剩 17 篇**
—— 判据 = ① 被 `AGENTS.md` / `REQUIREMENTS.md` 硬规则点名；② 被**代码/契约测试**读或断言
（`crates/cli/tests/{dsh,extension,st1_boundary}.rs`、`scripts/plan.py`）；③ 仍是**未收口**那条线的
唯一权威。其余（已收口 / 已否决 / 过程记录）**已删**，原文 ⇒ `git log --all -- docs/design/<文件>` ✓。

> **计划 / 提案类文档的索引在 `docs/ONBOARDING.md` §2** ✓（讲什么 / 什么状态 / 何时读，
> 一处维护；**本表不重复列**，免得两处漂移 ✗）。

- `e2-plan.md` — **E2 计划现行契约 + 收口索引**（`scripts/plan.py` 的判据输入）
- `docs-diet.md` — **文档预算与判据**（`scripts/docs-lint.py` ①–⑦ 的口径）
- `deepseek-harness.md` — DeepSeek Harness 适配（技能根 / 斜杠命令 / LSP 能力 / patch 形状）
- `ci-parallelism.md` — CI 并行化与快慢分层（`perf-gate` 与 `gates-fast` 的分工）
- `command-naming.md` — 命令命名盘点表（被 `crates/cli/tests/extension.rs` 与 `contributes.commands` 双向比对）
- `course-stdlib.md` — 课程标准库的**三层分界**（L1 prelude / L2 课程库 / L3 单元练习）
- `course-gate-in-ci.md` — 课程门禁接进 `scripts/soko gate` 与 CI
- `teaching-project.md` — 第二大课总体计划 + **§6 缺口台账协议**（`docs/gaps/` 的权威）
- `v077-st1-boundary.md` — ST1 决策记录 + **探针↔对账表一一对应**（`crates/cli/tests/st1_boundary.rs`）
- `v077-kernel-deficiencies.md` — 未做完的四章（ST6/7/9/11）的根因定位（G-56/58/59，E19 取证）
- `notation-subset.md` — 用户自定义记法子集（`infix:N` 族 + 零元 `notation`）的边界与第二刀
- `module-artifacts.md` — 模块级产物三块 + per-module Merkle 键 + 产物落盘（未做）
- `project-artifacts.md` — 项目闭包编译产物落盘（`<模块根>/.sokonanoda/compiled/`）
- `redundant-sorry.md` — 值位里多余的 `sorry` 判据（终审 = kernel）
- `rename-inlay.md` — rename / find-references / inlay hints / `sokonanoda lsp`
- `kernel-taxonomy.md` — 内核错误分类学 + 失败建议 + 基准 / fuzz 基建
- **`site-single-page.md`** — **官网（GitHub Pages）当前权威**：单页站点 + 三条防漂移机制；
  验收 `python3 scripts/check-site.py`（10 项，exit 0 才算过）

> 设计文档是**已落地决策的存档**（as-built）。被后续轮次取代的细节以
> `STATUS.md` 为准；确认过时且无人引用的**直接删除**（保留 git 历史）✓。

## 调研与笔记（`docs/notes/`）

> **2026-09-30 激进删档后只剩 1 篇** ✓：`dsh-project-assets.md`
> —— **DeepSeek Harness 源码勘察记录**（技能根 / 斜杠命令 / LSP 能力 / patch 形状 /
> hooks / 子 agent，逐条 `path:line`），被 `crates/cli/tests/dsh.rs` 的**契约测试**钉住 ✓
> （配套设计 `docs/design/deepseek-harness.md`）。
>
> 其余笔记**全删**（结论已升格进 `docs/design/`，底稿属过程记录 ✗）：`course-lean-style/`
> 三篇（记法改写 / R3 改写 / 记法输入面的施工细则）与更早归档过的顶层九篇 +
> `settheory-survey/` 十篇 —— 原文 ⇒ `git log --all -- docs/notes/<路径>` ✓。

**其它现场记录**：`docs/gaps/spike/README.md` — 卷 I 试做稿（2 单元 + 66 条标准库，
全部真内核判卷 0 failed，逐条标 `L-xx` 欠账）。

## 归档（`docs/archive/`）—— **归档 ≠ 销毁** ✓

> **索引** ✓：`docs/archive/README.md`（**每个归档文件都在那里被点名** ——
> `scripts/docs-lint.py` 判据 ⑥ 机械检查 ✗）。读法：`gunzip -c <文件> | less` ✓。
>
> **布局** ✓：`docs/archive/<批次>-<日期>/<原文件名>.gz`（**保留原相对路径** ⇒
> 映射机械可算 ✓）。**全局规则** ✓（2026-09-30 激进删档后**一句话覆盖全部历史指针**）：
> **任何指向已删/已归档路径的引用** ⇒ `git log --all -- <原路径>` /
> `git show <旧提交>:<原路径>` 取原文 ✓ —— **历史由 git 追溯**，不再单独归档 ✗。
> 归档目录只留**最近 3 轮**（`docs/archive/README.md` 逐个点名 ✓）。
>
> **已删除（不归档）** ✗：2026-09-29 的计划/交接类（`docs/HANDOVER.md`、`docs/E2-HANDOVER.md`、
> `docs/NEXT.md`、`docs/PLAN-0.74-0.79.md`、`docs/HANDOFF-0.7x.md`、`docs/design/PLAN-appendix-*.md`）
> —— 内容已并入 `docs/ONBOARDING.md`（**接手 / 唯一队列 / 开工单**）与 `docs/STATUS-ARCHIVE.md`。
>
> **不许动的例外** ✗（有真消费者，动了就判红）：`docs/gaps/repro/**`（复现被
> `gap.py check` 在 gate + CI 三片矩阵里真跑 ✓）、`docs/protocol.md`
> （**4 个测试读它正文并断言** ✗）、`docs/perf/ledger.jsonl` / `docs/gaps/ledger.jsonl` /
> `docs/courses/ledger.jsonl`（门禁基线 ✓）。设计与判据：`docs/design/docs-diet.md` ✓。
>
> **已删（机器产物，不归档）** ✗：`docs/e2e/logs/**`（**206 个**测试日志）—— 活台账里指向它们的
> `log` 字段**同轮摘掉**（`e2e-merge.py --check` 校验"记录引用的日志必须存在" ⇒ 留着就是死指针 ✗）。

## 关联目录

- `ROADMAP.md` / `AGENTS.md`（仓库根）— 里程碑与 §10 验收 / agent 入口 + 硬规则速记
- `skills/` — 角色技能（`sokonanoda-teacher` / `-dev` / `-ci`）；**DeepSeek Harness 通过
  skill 名即斜杠命令直接消费**（`/sokonanoda-teacher` 等），适配见 `docs/design/deepseek-harness.md`
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
- `.devcontainer/` — 仅贡献者的 Rust 容器（终端用户无需 Rust）

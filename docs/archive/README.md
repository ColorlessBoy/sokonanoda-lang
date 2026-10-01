# `docs/archive/` —— 归档索引（**归档 ≠ 销毁** ✓）

> **这是什么** ✓：`docs/` 的**历史层** —— 已收口的过程记录、已发布批次的计划与调研、
> 逐轮日记。**它们不进"活文档"预算** ✗（`scripts/docs-lint.py` 判据 ① 明确排除本目录 ✓），但**必须可追溯** ✓：**本目录里每个文件都在下面被点名**（判据 ⑥ 机械检查 ✗ —— **咬不住等于没有** ✓）。
>
> **怎么读** ✓：`.md.gz` / `.jsonl.gz` / `.log.gz` ⇒ `gunzip -c <文件> | less`（或 `zcat`）；纯文本 ⇒ 直接读、直接 `grep` ✓。**原文一字未改** ✓。**总量**：2026-09-30 激进删档后 4 个 ⇒ 2026-10-01 起 **7 个**（判据 ⑥ 上限 **2.5 MB** ✓）。

## 索引

| 归档文件 | 原路径 | 内容 | 归档日 |
|---|---|---|---|
| `status-removed-round-508-2026-09-30.md.gz` | `STATUS.md` | **第 508 轮**（§3.C「前缀环境」落地 —— 全课程 `build` 2.65×；原文逐字未动 ✓） | 2026-09-30 |
| `status-removed-rounds-502-504-2026-09-30.md.gz` | `STATUS.md` | **两段已被取代的快照（2026-09-28 · 64/66 与第 491 轮）+ 第 502/503/504/506/507 轮**（原文逐字未动 ✓） | 2026-09-30 |
| `status-removed-rounds-490-493-2026-09-30.md.gz` | `docs/STATUS-ARCHIVE.md` | **第 490–493 轮**（G-56/G-64 探针 · ST16–ST19 收口 · v0.78.0 E17–E18 · X1 文档分层）—— 二次下沉（`STATUS-ARCHIVE.md` 只留最近 ~12 段）；原文逐字未动 ✓ | 2026-09-30 |
| `status-removed-rounds-500-501-2026-10-01.md.gz` | `docs/STATUS-ARCHIVE.md` | **第 500–501 轮**（剖面链路 + perf-gate 判红 + 入口级并行 · 分片否决/切片 1 三次失败/主线转 judge 前缀增量）—— 二次下沉（**L1 层预算只许减不许增** ⇒ 最老的段进 L3）；原文逐字未动 ✓ | 2026-10-01 |
| `status-removed-round-496-2026-10-01.md.gz` | `docs/STATUS-ARCHIVE.md` | **第 496 轮**（P0 纪律 + G-66/G-67 + E30 回归 + 连推守卫）—— 二次下沉（**L1 层预算只许减不许增** ✓）；原文逐字未动 ✓ | 2026-10-01 |
| `status-removed-round-494-2026-10-01.md.gz` | `docs/STATUS-ARCHIVE.md` | **第 494 轮**（X2 多会话/多分支：worktree 主解法 + 共享账本守卫；含两处实测对旧估计的更正）—— 二次下沉（为 **IA-4 设计轮**的 L1 预算腾位置，**只许减不许增** ✓）；原文逐字未动 ✓ | 2026-10-01 |
| `v077-snapshots-2026-09-30/v077-set-theory.md.gz` | `docs/design/v077-set-theory.md` | **v0.77.0「完整集合论 = kernel 压力面」的批次详情**（E12–E16 + ST1–ST19 分章计划 + 外部基准表）—— 该版本**已发布闭环** ⇒ 从活文档移出。⚠ 未做完的四章（ST6/7/9/11）**不在归档里**：它们在**活文档** `docs/design/v077-kernel-deficiencies.md` §三，仍是 E19 的取证材料 ✓ | 2026-09-30 |

## 2026-09-30 激进删档（**删除、未归档** ✗）

用户 2026-09-30 强指令：**删掉 `docs/` 下 80% 的文档**，判据 = `git ls-files docs | wc -l`。
本轮删掉的归档（**220 个**）覆盖：`site-rebuild-2026-09-26/`（17）·
`e2e-2026-09-26/logs/`（140）+ 其 `ledger-records-older.jsonl.gz` ·
`gaps-work-orders-2026-09-26/`（13）· `course-lean-style-2026-09-26/`（11）·
`settheory-survey-2026-09-26/`（10）· `notes-2026-09-26/`（9）·
`design-deprecated-2026-09-26/`（5）· `status-2026-09/`（2）·
`REQUIREMENTS-ARCHIVE.md` · 以及散篇 `.gz`（`e2-plan-full-*` /
`vscode-editor-feedback-plan-full-*` / `ci-failures-*` / `status-archive-older-rounds` /
`plan-v074-v078-asbuilt-*` / `handover-process-*` 等）。

> **全局规则** ✓（一句话覆盖全部历史指针）：**任何指向已删文件的路径**
> ⇒ 用 `git log --all -- <原路径>` / `git show <旧提交>:<原路径>` 取原文 ✓
> —— **历史由 git 追溯**，不再单独归档（用户 2026-09-30 口径：「删除 = `git rm`，
> 不进回收站、不归档到别处」）。上一轮（2026-09-29）删掉的计划/交接类
> （`docs/PLAN-0.74-0.79.md` · `HANDOVER*` · `E2-HANDOVER.md` 等 11 份）同此规则 ✓。
>
> **接手 / 计划的唯一入口 = `docs/ONBOARDING.md`**（接手路径 + 开工单 + 计划索引 §2）；
> 仍在维护的总账是 `STATUS.md`（流水）、`REQUIREMENTS.md`（权威）、
> `docs/design/e2-plan.md`（`scripts/plan.py` 的判据输入）。
> 机制：`scripts/docs-gc.py`（报告式，不自删）· `scripts/docs-expiry-check.py`（过期日期）。

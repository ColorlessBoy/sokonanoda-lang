# `docs/archive/` —— 归档索引（**归档 ≠ 销毁** ✓）

> **这是什么** ✓：`docs/` 的**历史层** —— 已收口的过程记录、已发布批次的计划与调研、逐轮日记。**不进"活文档"预算** ✗（`scripts/docs-lint.py` 判据 ① 明确排除本目录 ✓），但**必须可追溯** ✓：**本目录里每个文件都在下面被点名**（判据 ⑥ 机械检查 ✗ —— **咬不住等于没有** ✓）。**怎么读** ✓：`.md.gz` / `.jsonl.gz` / `.log.gz` ⇒ `gunzip -c <文件> | less`（或 `zcat`）；**原文一字未改** ✓。**总量**：2026-09-30 激进删档后 4 个 ⇒ 2026-10-01 起 9 个 ⇒ 2026-10-15 **20 个 / 0.13 MB**（判据 ⑥ 上限 **2.5 MB** ✓）。⚠ 本文件受判据 ⑦ 的 **L3 层 42 行**上限管 ⇒ 索引只写"点名 + 原路径 + 一句话"，细节在 `.gz` 原文里 ✓。
> **全局规则** ✓（一句话覆盖全部历史指针）：**任何指向已删/已归档路径的引用** ⇒ 用 `git log --all -- <原路径>` / `git show <旧提交>:<原路径>` 取原文 ✓（用户 2026-09-30：「删除 = `git rm`，不进回收站、不归档到别处」）。**接手 / 计划的唯一入口 = `docs/ONBOARDING.md`**；仍在维护的总账是 `STATUS.md`（流水）· `REQUIREMENTS.md`（权威）· `docs/design/e2-plan.md`（`scripts/plan.py` 的判据输入）。机制：`scripts/docs-gc.py`（报告式，不自删）· `scripts/docs-expiry-check.py`（过期日期）。

## 索引

| 归档文件 | 原路径 | 内容 | 归档日 |
|---|---|---|---|
| `status-removed-round-508-2026-09-30.md.gz` | `STATUS.md` | **第 508 轮**（§3.C「前缀环境」落地 —— 全课程 `build` 2.65×；原文逐字未动 ✓） | 2026-09-30 |
| `status-removed-rounds-502-504-2026-09-30.md.gz` | `STATUS.md` | **两段已被取代的快照（2026-09-28 · 64/66 与第 491 轮）+ 第 502/503/504/506/507 轮**（原文逐字未动 ✓） | 2026-09-30 |
| `status-removed-rounds-490-493-2026-09-30.md.gz` | `docs/STATUS-ARCHIVE.md` | **第 490–493 轮**（G-56/G-64 探针 · ST16–ST19 收口 · v0.78.0 E17–E18 · X1 文档分层）—— 二次下沉（`STATUS-ARCHIVE.md` 只留最近 ~12 段）；原文逐字未动 ✓ | 2026-09-30 |
| `status-removed-rounds-500-501-2026-10-01.md.gz` | `docs/STATUS-ARCHIVE.md` | **第 500–501 轮**（剖面链路 + perf-gate 判红 + 入口级并行 · 分片否决/切片 1 三次失败/主线转 judge 前缀增量）—— 二次下沉（**L1 层预算只许减不许增** ⇒ 最老的段进 L3）；原文逐字未动 ✓ | 2026-10-01 |
| `status-removed-round-497-2026-10-01.md.gz` | `docs/STATUS-ARCHIVE.md` | **第 497 轮**（P1 复盘 —— 4.14× 重复编译：G-68 + 测次数的守卫）—— 二次下沉；原文逐字未动 ✓ | 2026-10-01 |
| `status-removed-round-496-2026-10-01.md.gz` | `docs/STATUS-ARCHIVE.md` | **第 496 轮**（P0 纪律 + G-66/G-67 + E30 回归 + 连推守卫）—— 二次下沉（**L1 层预算只许减不许增** ✓）；原文逐字未动 ✓ | 2026-10-01 |
| `status-removed-rounds-498-499-2026-10-01.md.gz` | `docs/STATUS-ARCHIVE.md` | **第 498–499 轮**（P2 进度粒度 + P1′ 方案/收益估算）—— 二次下沉（**L1 层预算只许减不许增** ✓）；原文逐字未动 ✓ | 2026-10-01 |
| `status-removed-round-494-2026-10-01.md.gz` | `docs/STATUS-ARCHIVE.md` | **第 494 轮**（X2 多会话/多分支：worktree 主解法 + 共享账本守卫；含两处实测对旧估计的更正）—— 二次下沉（为 **IA-4 设计轮**的 L1 预算腾位置，**只许减不许增** ✓）；原文逐字未动 ✓ | 2026-10-01 |
| `v077-snapshots-2026-09-30/v077-set-theory.md.gz` | `docs/design/v077-set-theory.md` | **v0.77.0「完整集合论 = kernel 压力面」的批次详情**（E12–E16 + ST1–ST19 分章计划 + 外部基准表）—— 该版本**已发布闭环** ⇒ 从活文档移出。⚠ 未做完的四章（ST6/7/9/11）**不在归档里**：它们在**活文档** `docs/design/v077-kernel-deficiencies.md` §三，仍是 E19 的取证材料 ✓ | 2026-09-30 |
| `status-removed-rounds-2026-10-13.md.gz` | `STATUS.md` | 按 `AGENTS.md`「只保留最近 3 轮」移入的旧轮 ✓ | 2026-10-13 |
| `requirements-sec9-removed-2026-10-13.md.gz` | `REQUIREMENTS.md` §9 | 最老 3 条正文（原文）✓ | 2026-10-13 |

### `docs-diet-2026-10-15/` —— 判据 ① 回收（**活文档 10.05 MB > 10.00 MB** ✗ ⇒ 9 份已收口/被取代的活文档退场）

> **为什么只能这样** ✗：**上限不许抬**（用户 2026-09-26：「把上限从 3 MB 抬到 10 MB **不算完成** —— 要的是**少消耗注意力**」）⇒ 让旧的退场 ✓。**选材三条**（缺一条就会造出悬空指针或丢证据）：① 结论已落地/被取代（G-48/G-58 **已 `fixed`**）；② **判据 ⑧ 扫描范围内零非我可写引用**（`crates/**` · 根 `*.md` · `scripts/**` 里的活指针一律留档 —— 那几处**改不了** ✗）；③ **不是机器产物台账的输入**。原文逐字未动 ✓（`gunzip -c … | cmp -` 逐字节相等）。

| 归档文件 | 原路径 | 内容 | 归档日 |
|---|---|---|---|
| `PLAN-align-lean4-as-built.md.gz` | `docs/notes/PLAN-align-lean4-as-built.md` | **对齐 Lean 4 交互性能的 as-built 读数台账**（§11.1–§11.25）—— 主计划 §11 本来就写着「**已归档** ⇒ 本文件」⇒ 归档只是把声明兑现 ✓ | 2026-10-15 |
| `perf-lean4-interactive-incremental.md.gz` | `docs/notes/perf-lean4-interactive-incremental.md` | **Lean 4 四层缓存调研**（2026-10-07）—— 主计划自己写「§4.0/§4.1 **已被 0.83.0–0.85.2 的提交推翻** ⇒ 只当 Lean 侧机制引用」⇒ 被取代 ✓ | 2026-10-15 |
| `perf-lean4-bench.md.gz` | `docs/notes/perf-lean4-bench.md` | **Lean 4 按键墙钟实测**（2026-10-09，只读测量）—— 北极星读数已被 `docs/PERF.md` 与主计划 §12 的后续读数取代 ✓ | 2026-10-15 |
| `e19-baseline.md.gz` | `docs/design/e19-baseline.md` | **E19 甲案（元变量）开工前冻结的读数**（刀 0）—— 缺口 **G-48 已 `fixed`（`fixed_in: 0.79.0`）**，甲案已有活设计 `docs/design/metavar-engine.md` ✓ | 2026-10-15 |
| `e19-evaluation.md.gz` | `docs/design/e19-evaluation.md` | **E19 甲/乙案评估**（先冻结基线、先做乙案）—— 同 G-48 已收口 ⇒ 评估件退场 ✓ | 2026-10-15 |
| `g58-large-elimination.md.gz` | `docs/design/g58-large-elimination.md` | **G-58 大消去设计稿（待用户批准）**—— 台账 **G-58 已 `fixed`**（`Prop` 入 `Type` 出已可用）✓ | 2026-10-15 |
| `HANDOFF-course.md.gz` | `docs/notes/HANDOFF-course.md` | **课程线交接单（2026-10-03）**—— process 档，后续轮次已收口（结论落 `courses/set-theory/` 与 `docs/ONBOARDING.md` §0.2）✓ | 2026-10-15 |
| `course-implicit-census.md.gz` | `docs/notes/course-implicit-census.md` | **课程侧隐式参数普查结论（2026-10-03）**—— 逐处明细本来就在活文档 `courses/set-theory/gaps/notation-ok-census.md` + `gaps/census-ledger.json` ⇒ 本件是过程记录 ✓ | 2026-10-15 |
| `PLAN-align-lean4-rounds-20-58.md.gz` | `docs/notes/PLAN-align-lean4.md`（**切片** §13–§41 = 第 775–1580 行） | **该规划逐轮读数/负结论/落点**（第 20–58 轮）—— 2026-10-15 因判据 ① 回收，**活文档只留 §0–§12 的现行结论与判据**（主计划 §12 末尾留指针）✓ | 2026-10-15 |
| `2026-10-10-48a2cb4c-vc1.138.0.log.gz` | `docs/e2e/logs/2026-10-10-48a2cb4c-vc1.138.0.log` | **孤儿 e2e 日志**（216 份里**唯一**没被 `docs/e2e/ledger.jsonl` 任何记录引用的一份；`latest.json` 也不指它）⇒ 归档不影响 `e2e-merge.py --check` ✓ | 2026-10-15 |

> ⚠ **这批不动**的四类（不是漏了，是不该动 ✗）：① **守卫输入台账**（`docs/gaps/ledger.jsonl` · `docs/ux-queue-events.jsonl` · `docs/ux-queue-extras.jsonl` · `docs/ux-findings-2026-10-12.md` · `docs/e2e/ledger.jsonl` · `docs/perf/ledger.jsonl` —— 一字节未动，见本轮报告的五份 sha256 前后对照 ✓）；② **被活台账点名的 215 份 `docs/e2e/logs/*.log`**（`e2e-merge.py --check` 校验存在 ⇒ 动了就是死指针 ✗，即使它占 **5.2 MB**）；③ **`crates/**` / 根 `*.md` / `scripts/**` 里有活指针的活文档**（如 `docs/notes/PLAN-align-lean4.md`、`docs/design/tactic-docs.md` —— 归档会让判据 ⑧ 新增悬空引用，而那几处**改不了** ✗）；④ **`docs/inbox/*.patch`**（它们是队列 **inbox 的提交物**，`--inbox` 里状态是 `submitted` ⇒ 待受理的活件 ✗）。

## 2026-09-30 激进删档（**删除、未归档** ✗）
用户 2026-09-30 强指令：**删掉 `docs/` 下 80% 的文档**，判据 = `git ls-files docs | wc -l`。本轮删掉的归档（**220 个**）覆盖：`site-rebuild-2026-09-26/`（17）· `e2e-2026-09-26/logs/`（140）+ 其 `ledger-records-older.jsonl.gz` · `gaps-work-orders-2026-09-26/`（13）· `course-lean-style-2026-09-26/`（11）· `settheory-survey-2026-09-26/`（10）· `notes-2026-09-26/`（9）· `design-deprecated-2026-09-26/`（5）· `status-2026-09/`（2）· `REQUIREMENTS-ARCHIVE.md` · 以及散篇 `.gz`（`e2-plan-full-*` / `vscode-editor-feedback-plan-full-*` / `ci-failures-*` / `status-archive-older-rounds` / `plan-v074-v078-asbuilt-*` / `handover-process-*` 等）。上一轮（2026-09-29）删掉的计划/交接类（`docs/PLAN-0.74-0.79.md` · `HANDOVER*` · `E2-HANDOVER.md` 等 11 份）同此规则 ✓。

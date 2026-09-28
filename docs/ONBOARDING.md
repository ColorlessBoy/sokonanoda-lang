# 接手路径（X1）—— **读这几个就够开工**

> **这份文件是「接手成本」的唯一权威** ✓。判据 `python3 scripts/docs-lint.py` 的 **⑦**：
> 下面「必读路径」表里的每个文件**必须存在**、**每个不得超过它的行数上限**、
> **全表合计不得超过 `docs-budget.json` 的 `onboarding_max_lines`** ✓。
> ⇒ 「接手要读多少」从一句希望，变成**会判红的数字** ✓。
> ⚠ **不许靠抬上限达标**（用户 2026-09-26：「把上限从 3 MB 抬到 10 MB **不算完成** ——
> 要的是**少消耗注意力**，不是允许更多」）⇒ 上限**只许收紧** ✓。

## 必读路径（全读；其余文件按需查，不必通读）

| # | 文件 | 读什么 | 行数上限 |
|---|---|---|---|
| 1 | `AGENTS.md` | 硬规则速记 + 命令 + 收尾义务（**本文件由 harness 自动读**） | 430 |
| 2 | `docs/ONBOARDING.md` | **本文件**（接手路径与预算本身） | 120 |
| 3 | `REQUIREMENTS.md` | 权威总账：硬规则全文 + §9 用户新要求 | 250 |
| 4 | `docs/HANDOVER.md` | 交接汇总：现在在哪、还剩什么、怎么继续 | 240 |
| 5 | `STATUS.md` | 当前快照 + 最近几轮 | 240 |

**按需查（不进必读表，但接手第一个小时大概率会碰）**：

- `ROADMAP.md` **§10**（待办与验收标准）· `docs/architecture.md` **§6/§8**
  （内核改动台账 + gotchas —— **动内核前必读**）· `docs/protocol.md`（`--json` / `query` 契约，
  **改输出前必读**）· `docs/PERF.md`（性能结论与量具）· `docs/TESTING.md`（测试三层）；
- **当版交接书**：`docs/HANDOFF-<version>.md`（如 `docs/HANDOFF-0.77.md`）——
  写它的是上一棒，读它的就是接手的你 ✓；
- 查历史：`docs/archive/README.md`（归档索引，**归档 ≠ 销毁**，查得到 ✓）。

## 四层与预算（判据 ⑦ 同时核这里）

| 层 | 是什么 | 预算 |
|---|---|---|
| **L0 热区** | `AGENTS.md` `README.md` `ROADMAP.md` `REQUIREMENTS.md` `STATUS.md`（仓库根） | 见 `onboarding_max_lines` 与 `layer_max_lines.L0` |
| **L1 规格** | `docs/` 顶层（开发者参考：架构 / 协议 / 测试 / 性能 / 计划 / 交接） | `layer_max_lines.L1` |
| **L2 台账** | `docs/design/` `docs/notes/` `docs/gaps/` `docs/perf/` `docs/e2e/`（设计与取证） | `layer_max_lines.L2` |
| **L3 归档** | `docs/archive/`（历史层；**不进活文档预算**，但**必须在索引里点名**） | `layer_max_lines.L3` + 判据 ⑥ |

**判据分工**（全部在 `scripts/docs-lint.py`，已进 `scripts/soko gate` 与 CI）：

| # | 挡什么 |
|---|---|
| ① | 活文档总量（MB） |
| ② | 单文件行数（防"一个文件变巨石"） |
| ③ | 入口文件行数（防 `AGENTS.md`/`STATUS.md` 这类膨胀） |
| ④ | 既有非入口文档的**冻结预算**（只许减不许增） |
| ⑤ | `docs/**` 垃圾残留 |
| ⑥ | 归档**必须被索引点名**（归档 ≠ 销毁） |
| **⑦** | **本文件的必读路径**：存在性 + 每文件上限 + **合计上限** ← X1 新增 |

## 怎么用（三句话）

1. **接手**：按上表 1→5 读完，再按需查下面的清单 ✓；`ROADMAP.md` 只读 §10，不必读全文 ✓。
2. **要长大**：任何进必读表的文件要涨行 ⇒ **必须同时**在 `docs-budget.json` 里抬它的
   per-file 上限**并**说明从哪省回来（合计上限**不动**）⇒ 评审看得见 ✓。
3. **要归档**：挪进 `docs/archive/` ⇒ **必须在 `docs/archive/README.md` 点名**，否则判据 ⑥ 判红 ✓。

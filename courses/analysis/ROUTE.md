# 设计：从集合论到实分析——路线与 M0 探针（2026-11-07）

> **状态**：M0/C0 设计文档；独立课程根 `courses/analysis/`；提案与追踪见 issue #13，C0 子 issue #14。
> **靶子**：Tao《Analysis I》第 4–11 章；新增约 161 个课程单元（见 #13 §3）。
> 本文只记录 C0 的探针结论、路线、前置与风险；不重复课程单元内容。

## 1. 决策（D1–D3，2026-11-06 用户「都听你的」）

| 决策 | 结论 |
|---|---|
| D1 终点 | 做到 Riemann 积分（Tao ch4–11），不中途停在连续 |
| D2 ℝ 路线 | Cauchy 序列商；P2 完整探针红且无安全绕法时再评估 Dedekind |
| D3 落点 | 新建独立课程 `courses/analysis/`；先过 scale gate（2026-11-07 用户改口径） |

分支模型：`main`（发布口，人 merge）← `agent/real-analysis-integration`（集成口，agent 可 push）← 子分支。`main` 只在 milestone 收口接收 integration → main。

## 2. C0 探针结果（P1–P5）

| 探针 | 文件 | 声明 | 结论 | 边界 |
|---|---|---:|---|---|
| P1 | `probes/p1-int-skeleton.sokonanoda` | 11 | `Int := Quot intRel` 的商类型骨架、`Quot.sound` / `Quot.lift` / iota | `neg_wd` 是 axiom；不含加法/乘法律 |
| P2 | `probes/p2-real-skeleton.sokonanoda` | 25 | 抽象 `Q` 上 `Cauchy` / `realRel`；`Real := Quot realRel`；`Quot.lift` 良定义 + iota + `Quot.exact` | `Q` 与 `realRel_*` 是 axiom；不含 ℚ 构造与完备性 |
| P3 | `probes/p3-sum-rec.sokonanoda` | 3 | `Nat.rec` 写有限和；内置 `Nat.add` 在递归子步骤不归约，课程自定义 `myAdd` 可绕过 | 只是 workaround，不是语言缺口的定论 |
| P4 | `probes/p4-continuity-ivt.sokonanoda` | 12 | 抽象 `R` 上的 `ContinuousAt` / `Continuous` / `Between` / `IVTStatement` | IVT 只陈述，未证明 |
| P5 | `probes/p5-partition-integral.sokonanoda` | 18 | `Partition` / `upperSum` / `lowerSum` / `Integrable` / `FTCStatement` | FTC 只陈述，未证明 |

**合计 69 条声明 checked、0 diagnostic、0 open**（0.98.3 本机；0.98.4 CI 复验）。
探针不是课程单元：不进 `course.json`、不进课程门禁、不面向学习者。

复跑：

```bash
for f in courses/analysis/probes/p*.sokonanoda; do
  scripts/soko grade "$PWD/$f" || exit 1
done
```

## 3. 路线与规模

| 卷 | 章 | 内容 | 推荐单元 |
|---|---|---|---:|
| II | II.1–II.2 | ℤ / ℚ 的序对商 | 28 |
| II | II.3–II.6 | 有理数性质、Cauchy、ℝ、完备性 | 39 |
| II | II.7 | 数系综合 | 5 |
| III | III.1–III.3 | 序列、扩展实数、limsup / liminf | 29 |
| III | III.4–III.5 | 子序列、Cauchy、级数 | 22 |
| III | III.6–III.7 | 连续、微分 | 27 |
| III | III.8–III.9 | Riemann 积分、综合 | 21 |

合计 **171 单元**；按卷 I 密度约 **1,150–1,500 道练习**、**16 章**（可合并为 12–14 个教学章）。

## 4. 前置与增量

- **L2 分析库**（`Int` / `Rat` / `Real` / `Abs` / `Pow` / `Seq` / `Sum` / `Limit` / `Cont` / `Deriv` / `Integral`）随 milestone 增量，不单独占一个长阶段；
- **scale gate**：263 → 273 单元的门禁 / build / LSP / 站点要先量；超预算则拆卷或并行 `check.py`，不抬超时；
- **语言缺口**：探针撞墙先记 `docs/gaps/ledger.jsonl`；小修走独立 WO；不为了让课程变绿改判定语义。

## 5. 里程碑（乐观目标，不是承诺）

| 里程碑 | 内容 | 目标 |
|---|---|---|
| C0 | 探针 + 本文 | 2026-11-07 12:00 |
| M1 | ℤ + ℚ，28 单元 | 2026-11-12 22:00 |
| M2 | 有理数性质 + Cauchy + ℝ + 完备性，44 单元（**已完成：II.3 单元㉙–㉞ + II.4 单元㉟–㊹ + II.5 单元㊺–54 + II.6 单元55–68 + II.7 单元69–72 已落盘，44/44；68 是代表元层 Cauchy 完备性，集合版 LUB 的差额记在 #18；章配额差额随收口报告**） | 2026-11-15 |
| M3 | 序列 + 级数，51 单元（**进行中：III.1 单元73–83 + III.2 单元84–85 已落盘，13/51**） | 2026-11-18 |
| M4 | 连续 + 微分，27 单元 | 2026-11-20 |
| M5 | Riemann 积分 + 综合，21 单元 | 2026-11-22 |
| M6 | QA / 文档 / 站点 / 发布 | 2026-11-24 |

冲刺目标 2026-11-21。2026-11-07 用户选 A：M1 保持 28 单元（旧记「18 单元」是笔误，已按实际单元数校正），顺延到 11-12；M2–M6 顺延 4 天，总目标 11-24。前提：2–4 个写手并行、主 agent 串行验收、门禁不再出新的红灯；某 milestone 超过目标 1 天，按 #13 §13 橙色升级，不静默拖。

## 6. 风险

| 风险 | 应对 |
|---|---|
| ℝ 的完整构造与完备性（最大） | P2 只是骨架；M2 先做最小定理链，红了就按 D2 评估 Dedekind |
| 递归 / 有限和的摩擦 | P3 的 `myAdd` workaround 固定进 L2；必要时记缺口 |
| 规模（门禁 / build / LSP） | scale gate 实测；拆卷 / 并行；不抬上限 |
| 无 `simp` / `ring` 自动化 | 库引理 + 证明模式 + 每章读证明单元 |
| 估计偏差 | 按卷 I 实测密度；每章回写实际值，滚动修正 |

## 7. 不做什么

- Analysis II / 度量空间 / 拓扑 / 多元微积分 / 测度论 / 复分析；
- 不逐题照搬 Tao；不追 Mathlib 的自动化与类型类；
- 不要求官方 Lean 工具链；
- 第一遍不做英文镜像；
- 不改写卷 I 已交付的 112 个单元。

## 8. 参考

- `courses/analysis/probes/README.md`（探针说明）
- issue #13（提案与追踪）· #14（C0）
- `docs/design/course-stdlib.md`（L1/L2/L3 分界）
- `docs/design/teaching-project.md`（项目上游设计）

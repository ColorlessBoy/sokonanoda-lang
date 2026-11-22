# 设计：从集合论到实分析——路线与 M0 探针（2026-11-07）

> **状态**：M1–M5 已完成，分析课 171/171 单元（M5 于 2026-11-19 落盘，PR #62 已合并）；M6 的 QA 与文档已收口，发版被 Lean Kernel Arena 门禁挡住。独立课程根 `courses/analysis/`；提案与追踪见 issue #13。
> **靶子**：Tao《Analysis I》第 4–11 章；171 个课程单元（原估 161，M1 实际 28 比计划多 10）。
> 本文记录探针结论、路线、前置与风险；单元正文在 `units/`，逐单元记录在 `README.md`。

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
| M3 | 序列 + 级数，51 单元（**已完成：III.1 单元73–83 + III.2 单元84–90 + III.3 单元91–99 + III.4 单元100–109 + III.5 单元110–123 已落盘，51/51；122 是尾块判别的反方向，123 只做交换前两项，一般重排与和类不变列为边界**） | 2026-11-18 |
| M4 | 连续 + 微分，27 单元（**已完成：III.6 单元124–139 与 III.7 单元140–150 已落盘，27/27；124 是函数极限的 ε-δ 定义，125 是连续与复合封闭，126 是序列判据，127 是函数极限的加群，128 是连续函数的加群，129 是单侧极限与两侧拼合，130 是一致连续的定义与闭包，131 是一致连续沿序列搬运，132 是连续与有限极限给出的局部有界盒子，133 是连续函数的局部保号与严格不等式的稳定性，134 是闭区间上最值的条件版（上确界被取到）、单调充分条件与开区间反例，135 是严格负点集在区间内部没有最大元（保号性把负点往右推一步），136 是条件版介值定理（非正点集的最大元是内部零点）；137 是无穷远处的极限（ε 给出一条分界线 M）、平移与最终相等，恒等函数没有有限极限；138 是无穷远处极限的代数（和、取负、差、加常数）与序列判据；139 是单调函数与介值性（介值性反过来给出连续，III.6 收口）；140 开启 III.7，把曲线在一点附近最像的直线写成斜率函数（`IsSlope` 的乘积等式、`HasDerivAt` 的极限），常函数与恒等函数给出导数零与导数一，乘零、乘一与乘法交换随差分引擎进库；141 做垂直平移与取负，142 证极限值唯一，143 把极限局部化，144 约掉非零因子、收口导数值的唯一性；145 先补 ℚ 乘法对加法的分配律（两边分母差一个因子，交叉相乘后四个单项式重排），再把它接到斜率函数与导数上，得到导数的和与差；函数取 `ratRep → ratRep`，停在有理代表元层；146 立乘法的保序性（正因子不改变不等号），把 `x ≤ y` 与 `0 < z` 拆到代表元、用 `e = f + t` 把交叉乘积对齐，严格版本接非零因子消去，两个正数的积为正作推论；147 立乘法的符号表（负因子把不等号翻过来：正正得正、正负得负、负负得正），把取负与乘法的交换接起来；148 把导数符号换成单侧抬高——斜率函数在去心邻域里与 L 同号，再沿斜率等式传给函数值之差，`RisesRight` / `RisesLeft` 为极值判别备好一半；149 用它们收口费马定理——局部极值点两侧都不能抬高，故导数既不能正也不能负，`ratLeRep_total` 与 `ratLeRep_eq_or_lt` 把两个「不」收成零，极小值靠把曲线翻过来取负；150 做局部单调与导数符号的容易一半——不减挡左侧抬高、不增挡右侧抬高，于是导数分别不为负、不为正，反向的中值定理留给后面；常数倍的斜率与极限留到后续单元**） | 2026-11-20 |
| M5 | Riemann 积分 + 综合，21 单元（**已完成：III.8 单元151–166 与 III.9 单元167–171 已落盘，21/21**；151 立分割的段数、切点、端点与严格上升，152 证切点沿下标前进：`point_le_add` / `point_mono` / `point_in_closed`，153 立左闭右开的阶梯函数 `StepOn` 与三切点分割 `threePoint`，154 立矩形和 `stepSum` 及它的保号、保序，155 把矩形和对高度的加法/取负接上（`stepSum_add` / `stepSum_neg`），156 证常值函数的面积与分割无关（`sumTo_const_len` / `stepSum_const`），157 立每段上的上界与下界两摞台阶（`UpperStep` / `LowerStep`）及它们的传递、常值与取负，158 把两摞台阶乘宽求和成上和/下和（`UpperSum` / `LowerSum`）、证同一分割上下和有序并以 ε 判据定义可积（`Integrable`），常值函数可积，159 把可积性改写成只谈两摞台阶的 ε 判据（`gap_of_integrable` / `integrable_of_gap` / `integrable_iff_gap`），160 证逐点相等的函数有同一套上下界与同一个可积性（`integrable_congr_fun`），并让取负保持可积（`integrable_neg` / `integrable_neg_iff`）；151–152 的引擎收进 `lib/Integral`，154–155 的矩形和引擎收进 `lib/IntegralRect`，157 的台阶引擎收进 `lib/IntegralStep`，158 的上下和引擎收进 `lib/IntegralSum`，159 的判据引擎收进 `lib/IntegralGap`，160 的点式等价与取负引擎收进 `lib/IntegralNeg`；161 把同一个分割上的两块面积接起来——两个界逐段相加（`upperStep_add` / `lowerStep_add`），宽乘高度和按分配律拆开（`segLen_mul_add`），矩形和按函数分成两摞（`stepSum_add`），上和与下和各自分开（`upperSum_add` / `lowerSum_add`），两个差相加恰好是和的差（`stepGap_add`），故两块各小于 ε/2 时和的那个差小于 ε（`stepGap_add_lt`），同一个分割上就有可加性（`integrable_add_of_gap`）；161 的引擎收进 `lib/IntegralAdd`；162 把整条曲线抬高一个常数——两个界逐段加同一个数（`upperStep_add_const` / `lowerStep_add_const`），一段上的高加常数按分配律拆开（`segLen_mul_add_const`），矩形和与上下和各抬 `k·(b-a)`（`stepSum_add_const` / `upperSum_add_const` / `lowerSum_add_const`），两个和之差不变（`stepGap_add_const`），故可积性在垂直平移下不变（`integrable_add_const` / `integrable_add_const_iff`），常值曲线给出可算读数（`demo_upperSum_shift_const`）；162 的引擎收进 `lib/IntegralShift`；163 立有理数的阿基米德性质——比较化成整数乘法（`ratLeRep_zero_lit` / `ratLtRep_zero_num`），倍数取 n=(r+1)(m+1) 时待超过量不超过 n 倍的正数（`arch_le_lit` / `arch_le`），严格版本由「待超过量加一」收紧（`arch_lt_of_nonneg`），半的两倍给出可算读数（`demo_arch_two`），163 的引擎收进 `lib/RatArch`；164 把一致连续落到分割的每一段上——段里的点到左端点不超过段宽（`close_point_left`），于是 `f(左端点)±η` 是两条界（`upperStep_leftUpper` / `lowerStep_leftLower`），两个矩形和之差是两段 η 乘区间长度（`stepSum_left_gap`），一对台阶夹住曲线（`strip_of_mesh`），常数曲线在两点分割上读出 1（`demo_twoPoint_half`）；164 的引擎收进 `lib/IntegralStrip`；165 用阿基米德性质造等分分割——把正数除以正整数（`ratDiv` / `divDen`），行走函数 `walkPoint` 走完 N-1 步停在 b，得到 `uniformPartition` 的切点、端点与网格（`uniformPartition_isPartition` / `uniformPartition_mesh`），任给正 δ 都有每段短于 δ 的分割（`exists_mesh_partition`），三段的样本给出可算读数，165 的引擎收进 `lib/IntegralMesh`；166 收口一致连续 ⇒ 可积——阿基米德性质取 N 使 2(b−a) < N（`exists_nat_gt_double`），η=ε/N 由除法引擎给出，`strip_of_mesh` 的差 2η(b−a) 由 `div_gap_lt` 压到 ε 以下，`integrable_of_gap` 给出 `uniformlyContinuous_integrable`；边界明说：逐点连续要一致连续（Heine–Cantor）需要紧性或完备性，课程只到代表元层 Cauchy，166 的引擎收进 `lib/IntegralContinuous`；167 开启 III.9，把面积值写成 `HasIntegral`——候选值不低于任何下和、不高于任何上和且函数可积，三条投影给出两侧的界，`sub_le_of_pin` 把同一个分割夹住的两个候选的差压到分割宽度以下，可积性给的那个窄分割于是把两个候选同时夹住，唯一性收成 `ratRel`；常值曲线给出第一个读数 `k·(b-a)`，167 的引擎收进 `lib/IntegralValue`；168 让面积值随曲线取负——一段上 `-f` 的下界取负翻成 `f` 的上界（`upperStep_neg_rev`），上和反向（`lowerStep_neg_rev`），矩形和的负号由 `stepSum_neg` 搬到外面，于是 `-f` 的下和取负是 `f` 的上和（`lowerSum_neg_rev` / `upperSum_neg_rev`），两个方向铺好后 `hasIntegral_neg` 给出 `HasIntegral a b (-f) (-I)`，常值曲线的负数读数是 `-(k·(b-a))`（`hasIntegral_neg_const`），168 的引擎收进 `lib/IntegralValueNeg`；169 立面积值的符号——常值零台阶逐点垫在非负曲线下面（`lowerStep_nonneg`），零乘宽的和为零（`stepSum_zero`），于是零是下和、0 ≤ I（`hasIntegral_nonneg`）；非正曲线翻过来得 I ≤ 0（`hasIntegral_nonpos`）；候选值可沿相等替换（`hasIntegral_congr_value`），零曲线读数落到 0（`hasIntegral_zero`），169 的引擎收进 `lib/IntegralSign`；170 把面积值按逐点序比较——两条不等式相减（`sub_le_sub`），可积性给出的窄分割把 I−J 压到 U−L 以下，`hasIntegral_mono` 用 `ratLeRep_of_sub_le_all` 收成 I ≤ J；常数界 `hasIntegral_le_const` / `hasIntegral_const_le` 与合取的 `hasIntegral_bounds` 是推论，逐点相等的曲线有同一个面积值（`hasIntegral_congr_fun`），170 的引擎收进 `lib/IntegralMono`；171 收口 III.9——把整条曲线抬高 k，两个界整体减 k 又垫回原曲线（`lowerStep_sub_const` / `upperStep_sub_const`），一段上的高加 k 乘宽按分配律拆开，整条和多出 k(b-a)（`stepSum_sub_const`），于是抬高后的下和不超过 I+k(b-a)、上和不低于它（`lowerSum_add_const_le` / `upperSum_add_const_ge`），可积性由 `integrable_add_const` 给，三条并成 `hasIntegral_add_const`，常值曲线的读数是 (c+k)(b-a)（`demo_hasIntegral_shift_const`）；边界明说：面积值的加法仍缺加细引擎，FTC 要中值定理与积分函数，留作后续课程内容） | 2026-11-22 |
| M6 | QA / 文档 / 站点 / 发布（**进行中**：课程门禁 0 判负，文档与站点对齐 171/171；发版被 Lean Kernel Arena 的递归规则与性能用例挡住；书页与双语待拍板） | 2026-11-24 |

冲刺目标 2026-11-24。2026-11-07 用户选 A：M1 保持 28 单元（旧记「18 单元」是笔误，已按实际单元数校正），顺延到 11-12；M2–M6 顺延 4 天，总目标 11-24。前提：2–4 个写手并行、主 agent 串行验收、门禁不再出新的红灯；某 milestone 超过目标 1 天，按 #13 §13 橙色升级，不静默拖。

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

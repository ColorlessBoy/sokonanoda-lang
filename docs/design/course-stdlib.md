# 设计：课程标准库的分层与「消除暴力」方案（2026-09-18）

> 触发（用户）：「你的教程出的题目，在做的时候，你会发现要补充很多其他的定理，边边角角的
> 定理，这个是在其他的教程里头会认为是天然应该知道的，或者说是标准库里已经实现的。
> 这个我感觉有点暴力，所以我需要你去用这个项目重新做一遍，然后才能把这些暴力给消除掉。
> 你要记录下这些其实是需要实现的，其实是没有的。」
>
> 输入：`docs/gaps/spike/README.md`（**真写 + 真判卷**的试做稿：2 个单元 + 66 条库，全部 0 failed）、
> `docs/gaps/ledger.jsonl`（G-14/G-15、L-01…L-05）。
> 上游：`docs/design/teaching-project.md`（总体计划）、`docs/design/set-theory-syllabus.md`（卷 I 大纲）。
>
> **本文档状态（2026-09-18 晚，实测更新）**：§0–§2 不动（判据仍锁定）；§3 从「试做实测」
> 更新为**当前真实状态**（`courses/set-theory/lib/` 的 6 个模块 + 自检入口，逐模块真判卷计数）；
> §3.2 是本轮新增的「本语言逼出来的三个变形」；§4 补 G-16 与三条变形；§5/§6 更新进度。
>
> ⚠️ **行号漂移（引用本文档的 WO 注意）**：本轮更新后行号整体下移，**别再用硬编码行号定位**。
> 已知两处：`docs/gaps/WO-009` 引的「`:49`（`Eq.symm`/`Eq.trans`/`congrArg` 那行）」今天是
> **`:61`**（表内仍逐字）；`docs/gaps/WO-006` 引的「`:97` G-03 那行」今天是
> **§4 表格的 G-03 行（`:268`）**，而且该行已按新实测**改写**（补了"`Exists.elim` 的 `Q`
> 只能是 Prop"这半句）。两处 WO 落地时按**节 / 表格行名**定位，不要按行号。

## 0. 一句话

**暴力 = 把"语言/标准库该给的东西"塞进课程内容让学习者手写。** 消除办法是三层分界，
判据只有一条：

> **Mathlib 有 ≠ 我们不该练；Mathlib 有且没有数学内容（纯定义展开）才归库。**

## 1. 三层分界（锁定）

| 层 | 放什么 | 判据（可机械检查） | 归属 | 卷 I 的规模（试做实测） |
|---|---|---|---|---|
| **L1 prelude** | 逻辑与等式的骨架：`True`/`False`/`And.elim`/`Or.elim`/`Not`/`absurd`/`Iff.*`/`Eq.symm`/`Eq.trans`/`congrArg` | **Lean core 里现成**，任何教程直接拿来用 | **语言仓**（prelude 扩展，三件套：课程 + 测试 + 白名单） | 16 条（L-01/L-02） |
| **L2 课程标准库** | 集合的**词汇 + 定义展开**：`Set`/`mem`/`subset`/`empty`/`union`… + `subset_def`/`mem_union`/`mem_inter`/`not_mem_empty`/`mem_power_iff`… | Mathlib 里是 `rfl` 或一行；**没有数学内容** | **课程仓 `lib/`** | 8 条（L-04） |
| **L3 单元练习** | 一切**有数学内容**的陈述：包含三律、最小元、分配律、De Morgan、像/原像、等价关系与划分、基数… | 需要"想一下"才写得出来，哪怕 Mathlib 已有 | **课程 `units/`** | 单元①② 共 16 道（含从 lib 移回来的 6 条） |

**反例警示（试做时踩的）**：我第一遍把 L3 的 16 条（`subset_refl`/`union_subset_iff`/
`inter_comm`/`inter_union_distrib_left`…）写进了 `lib/Set.sokonanoda`——那样学习者在做
练习时会发现"答案已经在库里"。**已记入台账 L-05**：
**第一刀已落**（2026-09-18）——`subset_refl/trans/antisymm`、`empty_subset`、
`subset_empty_iff`、`eq_empty_iff_forall_notMem` 六条移进单元②；
剩下 10 条（并/交/分配/幂集单调/交换）属单元③④，等那两个单元写的时候一起移。
**课程已建在语言仓内：`courses/set-theory/`（README/AGENTS/门禁见该目录）。**
（**2026-09-18 晚复测**：L-05 那条"还剩 10 条"已经**全部执行完**——`lib/Set.sokonanoda`
今天只剩定义、公理与展开引理，见 §3 的表与 §5 的进度。）

## 2. L1 提案：prelude 应当自带的 16 条（给语言线的清单）

> **状态（2026-09-19）：已实现。** 逐字签名、让位规则、分阶段与 GOLDEN 预测见
> `docs/design/prelude-l1-proposal.md`；落地值见 `crates/front/src/compile/prelude.rs`
> 的 `PRELUDE_L1_SRC`/`L1_FAMILIES`/`PRELUDE_NAMES`。下面的表是**提案期口径**
> （16 条 = 引理口径）；落到内核是 **30 个顶层名字**（差额是族头/构造子/消去子
> 与 `And.rec`/`Or.rec`）。台账 L-01/L-02 已关账。
>
> **P4 课程仓跟随：已做（2026-09-19）。** 提案 §6 的 P4 两步都落了地：
> ① `courses/set-theory/lib/Logic.sokonanoda` 退化成**只有注释的空壳模块**
> （0 条声明；30 个名字全部由 prelude 提供；34 个 `import lib.Logic` 一字未改，
> 实测空模块仍可 import、闭环 exit 0）；② 课程侧 **65 处项位裸名** `inl`/`inr`
> （61 行 × 10 个文件）改成**点号名** `Or.inl`/`Or.inr`，另有 **27 行注释**里
> "构造子是裸名"的说法同步改写（lib + units + solutions + 记法对照页；
> `grep` 全量复核后除空壳文件头的历史说明外再无裸名）。
> 验收：`python3 courses/set-theory/tools/check.py` = **exit 0 · 36 目标 ·
> 329 checked · 99 open · 0 判负**（checked 的 −26 正是 `lib/Logic` 少掉的 26 条
> 声明；**open 一条都没变** ⇒ 没删练习、没加 `sorry`、题义未动）。
> **没有一条声明需要保留**：prelude 的 30 个名字对课程用法逐字兼容——两处定形
> 差异（`And` 由 axiom 族变真归纳、`Or` 构造子由裸名变点号名）都不改调用形状。

来源（**历史**）：`courses/set-theory/lib/Logic.sokonanoda` 当年的 26 条声明（逐条标了
L-01…L-13）——P4 之后该文件不再声明任何名字，签名一律以 `PRELUDE_L1_SRC` 为准。

| 名字（Lean core 原名） | 形式 | 备注 |
|---|---|---|
| `True` / `True.intro` | axiom | 与 `False` 对称 |
| `False` / `False.rec` / `False.elim` | axiom + def | 已有 `False` 系公理的课程不少，prelude 给全 |
| `And` / `And.intro` / `And.left` / `And.right` / `And.elim` | **真归纳块 + 点号构造子**（G-02 已由 WO-005 修好，L1 不再走 axiom 族） | 已落地：`PRELUDE_L1_SRC` 的 B3 族（`docs/design/prelude-l1-proposal.md` §1.1 第 6–10 行） |
| `Or` / `Or.inl` / `Or.inr` / `Or.elim` | **inductive** + def | 构造子已定形为**点号名** `Or.inl`/`Or.inr`（B4 族）；裸模式 `\| inl a =>` 仍被接受 |
| `Not` / `Not.intro` / `Not.elim` | def | `Not A := A → False`（与 Lean 同） |
| `absurd` | def | `a → ¬a → b` |
| `Iff` / `Iff.intro` / `Iff.mp` / `Iff.mpr` | def + def | Lean 里是 structure，我们是 `And (A→B) (B→A)` |
| `Iff.refl` / `Iff.symm` / `Iff.trans` | def | Mathlib 级，但写集合等式天天用 |
| `Eq.symm` / `Eq.trans` / `congrArg` | def | **受 G-14 限制**：当前只能同宇宙 |

**落地要求**（硬规则 3）：prelude 增量必须配 **课程用例 + 三层测试 + 白名单**；
**已定形（0.59.0）**：G-02 的结论是「构造子进类型命名空间」（`Ind.ctor`），所以 L1 的 `And`/`Or` 用点号构造子；这条不再悬置。

**副作用要提前想**：入门课（`course/` 单元①②）**故意**把这批东西当教学内容
（学习者手写 `eq_symm_nat`）。prelude 一旦自带，入门课要改成"读 prelude 的现成引理 +
自己写一遍对照"，两处 golden 会变——这正是"三件套"里的课程那一件。

**测点（2026-09-18 晚，可复跑）**：`lib/Logic.sokonanoda` 今天真判卷 **26 条声明全 checked、
0 open、0 failed**（`node scripts/soko grade "$PWD/courses/set-theory/lib/Logic.sokonanoda"`，
退出码 0）——即这份清单在课程仓里是**可运行的真库**，不是纸面提案；语言线接手时可以直接
把它当 prelude 的草稿与验收样本。

## 3. L2 规范：课程标准库 `lib/` 该有什么（**当前真实状态**）

> 表里每一行的「计数」都是**实测**，不是估计。查法一律两条命令（绝对路径 + 退出码纪律
> 见 §3.3）：
>
> ```bash
> node scripts/soko grade "$PWD/courses/set-theory/lib/<模块>.sokonanoda"   # 退出码 0 = 没有坏
> node scripts/soko query check --file "$PWD/courses/set-theory/lib/<模块>.sokonanoda"  # counts.decl_checked
> ```

| 模块 | 内容 | 计数（实测） | 名字纪律 | 依赖谁 |
|---|---|---|---|---|
| `lib/Logic.sokonanoda` | **空壳模块（P4，2026-09-19）**：**0 条声明**，只留注释——30 个名字由 prelude 自带（`PRELUDE_L1_SRC`：真伪 / `And.*` / `Or.*` / `Not.*` / `absurd` / `Iff.*` / `Eq.symm` / `Eq.trans` / `congrArg`）；历史内容是 L1 的 16 条 + `Iff.refl/symm/trans` 共 26 条 | **0** | 名字照抄 Lean core 真名（`And.intro`/`Or.inl`/`Or.inr`/`Or.elim`/`Iff.mp`…）——**签名现在以 `PRELUDE_L1_SRC` 为准**，别在本文件里复活它们 | —（空壳；历史上是根模块） |
| `lib/Set.sokonanoda` | 定义 12 条（`Set`/`mem`/`subset`/`empty`/`univ`/`singleton`/`pair`/`union`/`inter`/`sdiff`/`compl`/`powerset`）+ `Set.ext`（**公理**）+ 展开引理 10 条 | **23** | **Loogle 取证版**：`Set.notMem_empty`（大写 M）、`Set.mem_powerset_iff`（不是 power）、`Set.mem_sdiff`（不是 diff）；`Set.Subset.refl/trans/antisymm` 的点号**在 `Subset` 上**但**不在本库**（它们是 L3，在单元②④） | `lib.Logic` |
| `lib/Exists.sokonanoda` | **G-03 的公理三件套**：`Exists`/`Exists.intro`/`Exists.elim`，外加便利引理 `Exists.imp`（∃ 的函子性） | **4**（3 axiom + 1 theorem） | Lean core 真名（`Exists.intro`/`Exists.elim`）；`Exists.imp` = Mathlib 真名 | `lib.Logic` |
| `lib/Prod.sokonanoda` | G-02 的实况样本：`inductive Prod` + 构造子 `prod_mk` + 投影 `Prod.fst`/`Prod.snd` + 展开引理 `Prod.fst_mk`/`Prod.snd_mk` + 库自检 `Prod.fst_snd_mk` | **6** | ⚠️ **构造子是裸名**（全项目唯一，G-02）⇒ 只能叫 `prod_mk` 而不是 `Prod.mk`；投影/展开引理照抄 Mathlib（`Prod.fst`/`Prod.fst_mk`）。改名计划见 **§3.2-A** | `lib.Logic` |
| `lib/Rel.sokonanoda` | 关系词汇 3 条（`Rel` = `A -> B -> Prop`、`Rel.inv`、`Rel.comp`）+ `Rel.ext`（**公理**：函数外延性）+ 展开引理 3 条（`Rel.inv_apply`/`Rel.comp_apply`/`Rel.inv_inv_apply`） | **7** | Mathlib 真名（`Relation.inv`/`Relation.comp` 的语义与参数顺序逐字同款；`Rel` 按 Tao §3.3 直接展开成箭头）；`Rel.ext` 是本卷**仅有的两条外延性公理**之一 | `lib.Logic`、`lib.Exists` |
| `lib/Fun.sokonanoda` | `Function.comp`；`Injective`/`Surjective`/`Bijective`；**数据版**的 `LeftInverse`/`RightInverse`/`Inverse`；展开引理 7 条（`comp_apply` 是 `rfl` 级，其余是双向恒等） | **14** | 全部 Mathlib 真名（`Function.comp`/`Injective`/`Surjective`/`Bijective`/`LeftInverse`/`RightInverse`）；**唯一例外** `Function.Inverse` 是**设计判断**（Mathlib 没有这个名字，它有 `invFun`/`Equiv`）——理由见 §3.2-B | `lib.Logic`、`lib.Exists` |
| `lib/Image.sokonanoda` | 像 `Set.image`（用 `Exists` 写）、原像 `Set.preimage`（只有一个函数应用）；展开引理 `Set.mem_image`/`Set.mem_preimage`；包装引理 `Set.image_mono`/`Set.image_subset_iff` | **6**（2 def + 4 theorem） | Mathlib 真名，含参数顺序（`Set.image α β f A`、`Set.preimage α β f B`） | `lib.Logic`、`lib.Exists`、`lib.Set` |
| `lib/Equiv.sokonanoda` | 等势的四条件（`Set.MapsTo`/`Set.LeftInvOn`/`Set.RightInvOn`，全在 `Mathlib/Data/Set/Function.lean`）+ 等势本体 `Set.Equiv`（**Prop 值**，数据 = 一对互逆映射）+ 构造子 `Set.Equiv.mk` | **5**（3 def + 1 def + 1 theorem） | 三条条件逐字照抄 Mathlib；**`Set.Equiv` 不是 Mathlib 名**（Loogle 精确查 `Set.Equiv` = `unknown identifier`，`Set.EquivalentOn` 同样不存在）⇒ 术语收在 `Set.Equiv`、构造子照 `Equiv.mk` 命名，**标"我们自定名"**。为什么必须是 Prop 见 **§3.2-C** | `lib.Logic`、`lib.Exists`、`lib.Set` |
| `lib/Demo.sokonanoda` | **自检入口**：`import` 各模块并真的判卷（今天 3 条演示：`And` 交换、`Set.subset_def` 展开、`mem_powerset_iff` 用法） | **3** | —（入口，不是 API） | `lib.Logic`、`lib.Set`（**待补**：其余 6 个模块，见 §3.3） |

**模块合计（P4 后重算）**：`lib/` **74 条声明全 checked、0 open、0 failed**
（`--json` 的逐目标：Demo 10 · Equiv 5 · Exists 3 · Fun 14 · Image 6 · **Logic 0** ·
Prod 6 · Rel 7 · Set 23；P4 前是 100，差额 = `Logic` 的 26 条）。
其中 `Set`+`Exists`+`Prod` = **32** 条是"从 0 补出来的标准库欠账"
（L-02…L-04 + G-02/G-03 的变形产物；原本还算上 `Logic` 的 26 条，已由 prelude 接管，
所以标准库欠账**净减 26**——这正是"消除暴力"要的效果）。

**依赖图（`import` 实测）**：

```text
Logic ──┬─ Set ──┬─ Image
        │        └─ Equiv
        ├─ Exists ─┬─ Rel
        │          ├─ Fun
        │          └─ Image / Equiv
        └─ Prod（只依赖 Logic）
```

**哪些单元吃哪些模块（把"库"与"练习"钉在一起，避免库长成没人用的 API）**：

| 单元 | `import` 的库 |
|---|---|
| ①–④ 集合与隶属 / 子集 / 并交幂集 / 外延性 | `Logic`、`Set` |
| ⑤ 序对与笛卡尔积 | + `Prod` |
| ⑥ 关系 | + `Exists`、`Rel` |
| ⑦ 函数 | + `Fun` |
| ⑧ 像与原像 | + `Image` |
| ⑨ 等势 | + `Equiv` |
| ⑩ 可数与 Cantor | + `Fun`、`Equiv`（`Prod` **没吃到**：`A × B` 的基数段走的是 `Equiv` 的数据形状） |
| ⑪ 宇宙与 Russell | `Logic`、`Set`、`Exists` |
| ⑫ 综合 | `Logic`、`Set`、`Exists`、`Rel`、`Fun`、`Image` |

**每条声明都带台账编号与出处**：模块文件头是作者的实测记录（`lib/Exists` 记 G-03 的
最小复现与升级路径、`lib/Prod` 记 G-02 与 `Prod.rec.{1}` 的宇宙陷阱、`lib/Equiv` 记
"为什么不是 ∃-双射"、`lib/Fun` 记"为什么是数据版"）。**写下一个模块前先读它的文件头**——
那里是判据的现场版本，本文档只做汇总。

### 3.1 名字对照：试做稿 → **Loogle 取证版**（2026-09-18 复核）

> 来源：`docs/notes/settheory-survey/prior-art-report.md` §1（逐字声明原文 + Loogle 存在性判定）。
> **课程仓 lib/ 一律用"真名"**；试做稿的名字只作为别名出现在这张表里。

| 试做稿（spike） | Mathlib 真名（已取证） | 备注 |
|---|---|---|
| `Set.not_mem_empty` | **`Set.notMem_empty`** | 大写 M（小写查找会误判"不存在"） |
| `Set.mem_power_iff` / `Set.power` / `Set.power_mono` | **`Set.mem_powerset_iff`** / `Set.powerset` / `Set.powerset_mono` | powerset 不是 power |
| `Set.mem_diff` / `Set.diff` | **`Set.mem_sdiff`** / `Set.sdiff`（别名 `Set.diff_eq`） | Mathlib 用 sdiff |
| `Set.subset_refl` / `Set.subset_trans` / `Set.subset_antisymm` | **`Set.Subset.refl` / `Set.Subset.trans` / `Set.Subset.antisymm`**（另有 `Set.eq_of_subset_of_subset`） | 点号在 `Subset` 上；`Set.subset_antisymm` **确不存在** |
| `Set.eq_empty_iff_forall_not_mem` | **`Set.eq_empty_iff_forall_notMem`** | 同上大写 M |
| `Set.mem_inter_iff` / `Set.mem_union` / `Set.empty_subset` / `Set.subset_empty_iff` / `Set.subset_def` / `Set.univ` / `Set.union_subset_iff` / `Set.inter_subset_left/right` / `Set.subset_inter(_iff)` / `Set.mem_singleton_iff` / `Set.union_comm` / `Set.inter_comm` | 同名 ✅ | 直接照抄 |
| `Set.singleton_subset_iff` / `Set.mem_singleton_self` / `Set.inter_union_distrib_left` / `Set.diff_subset` | ⚠️ **未逐行核对** | 用之前先在 Loogle 查一次，或标成"我们自定名" |

**落地实况（2026-09-18 晚）**：这张表里**只有被 `lib/` 真正用到的名字才落了地**——
`Set.notMem_empty`、`Set.mem_powerset_iff`、`Set.mem_sdiff`、`Set.mem_singleton_iff`、
`Set.mem_singleton_self`、`Set.mem_union`、`Set.mem_inter_iff`、`Set.subset_def`、
`Set.mem_univ` 都在 `lib/Set.sokonanoda` 里；`Set.Subset.*` 一族**故意不在库里**
（L3，属单元②④）。`Set.singleton_subset_iff` / `Set.inter_union_distrib_left` /
`Set.diff_subset` 至今**没用到**，也没取证——按"不预置用不上的 API"搁置。

**已验证的"不存在"（别写进教程）**：`Set.subset_antisymm`、`Set.not_mem_empty`、
`Set.Equiv`、`Set.EquivalentOn`、`Set.Finite.card`、`Set.image_subset_image`、
`Set.compl_compl`（根命名空间才有）、`↾` 这个记法、`Mathlib/Order/Set` 目录。

**2026 年重命名坑（引用旧教程必踩）**：`Set.setOf` → `Set.ofPred`（2026-07-09，连带
`mem_setOf_eq`→`mem_ofPred_eq` 等）；`Set.restrict` → `Set.domRestrict`（2026-07-19）；
`Set.le_eq_subset`/`lt_eq_ssubset` 等已废弃——**`⊆` 现在就是 `Subset` 的语法相等**。

**命名纪律**：点名形式用 Mathlib 的名字（`Set.subset_def`、`Set.mem_union`），
G-04（`notation`）落地后只加记法层、不改名字——于是"同一命题两种写法"天然成为 X 类练习。

### 3.2 本语言逼出来的三个变形（**新增，实测**）

这三条不是设计选择，是**语言今天的样子逼出来的**：每一条都有一个"我们本来想写什么 →
内核/前端拒了什么 → 改成了什么"。写库前必读，因为它们决定了上层课程（`units/`）的写法。

#### A. G-02 ⇒ 构造子裸名且全局唯一（`prod_mk` 而不是 `Prod.mk`）

> **状态（2026-09-19，含 P4 跟随）**：G-02 已由 WO-005 修好（构造子进类型命名空间
> `Ind.ctor`）。**L1 的 `And`/`Or` 已绕过它**（`PRELUDE_L1_SRC` 直接写点号
> 构造子，设计 §1.2），**课程侧 P4 也已跟随**：`Logic` 的裸 `inl`/`inr` 连同
> 课程里 65 处项位用法一起改成点号名 `Or.inl`/`Or.inr`（裸**模式** `| inl a =>`
> 仍被接受，课程里没有模式用法）。**源语法仍受影响的只剩 `Prod`**：
> `lib/Prod.sokonanoda` 的 `prod_mk`（本文件自己的归纳块，不归 prelude 管）
> 还是裸名，改名计划见本节下文——它没有跟着 P4 动，因为 prelude 只接管
> `And`/`Or` 两族。

- **想写的（官方 Lean 4）**：`inductive Prod (A B : Type) where | mk : A → B → Prod A B`
  之后构造子自动进类型命名空间，叫 `Prod.mk`；于是 `Prod.mk`/`Subtype.mk`/`Exists.intro`
  可以同时存在。
- **实测被拒**（`docs/gaps/repro/G02-ctor-namespace.sokonanoda`，本轮复跑同款）：
  · 两个 inductive 各写 `ctor mk` ⇒ `elab-duplicate-declaration`：**`duplicate declaration mk`**；
  · 想点带前缀的名 ⇒ `elab-unknown-identifier`：**`unknown identifier \`Pair.mk\``**。
  构造子以**裸名**进环境、且名字**全项目唯一**（没有 `namespace`/`open`，G-05）。
- **变形**：`lib/Prod.sokonanoda` 的构造子只能是**假唯一名** `prod_mk`
  （`prod_mk A B a b` 即官方的 `(a, b)`）。同理 `Or` 的构造子在 `lib/Logic` 里是裸名
  `inl`/`inr`。
- **改名计划与兼容策略**（G-02/WO-005 修好之后，机械执行，一次改完）：
  1. 前缀名转正：`prod_mk A B a b` → **`Prod.mk A B a b`**；
  2. 裸名降级为**别名**（保留旧写法，别让历史画布/golden 一夜失效）；
  3. `Prod.fst_mk` / `Prod.snd_mk` 的名字**不变**（它们已经是对的）；
  4. G-02 同时解锁"两个 `ctor mk` 并存"，于是 `Exists`/`Subtype`/`Quot` 一类单构造子
     归纳可以回归官方形状（与 B 联动，见下）。
- **顺带记一条宇宙陷阱**（同文件）：`Prod` 在 `Type`、motive 落在 `Sort u`，所以
  `Prod.rec` **必须显式写 `Prod.rec.{1}`**；省略层级时前端默认 `u = 0`，内核会报
  「期望 `Sort(0)`，实际是 `Sort(1)`」。命题型归纳（`Or`/`False`）不会咬人——只在
  Type 值的消去上出现。

#### B. G-03 ⇒ `Exists` 只能是公理三件套；`Exists.elim` 的 `Q` 只能是 Prop

- **想写的**：`inductive Exists {α : Sort u} (p : α → Prop) : Prop where | intro (w : α) (h : p w)`
  （Prop 结果 + Type 参数 + 单构造子 + 自有字段）。
- **实测被拒**（`docs/gaps/repro/G03-prop-type-param-inductive.sokonanoda`）：内核断言失败
  `rejected: assertion \`left == right\` failed / left: 1 / right: 0`——不是正常的错误消息，
  是 assert 失败；同一文件里列了 5 个通过形状做二分边界（唯一失败形状就是上面那条）。
- **变形一（已是既成事实）**：`lib/Exists.sokonanoda` 立成**公理三件套**
  `axiom Exists` + `axiom Exists.intro` + `axiom Exists.elim`（4 条声明含 `Exists.imp`）。
  收益是**升级只动本文件**：G-03 修好后改成 inductive，而 `units/` 的证明文本一行不用改
  （名字与签名保持）。
- **变形二（新实测的重点）**：公理版的 `Exists.elim` 是
  `(A : Type) → (p : A -> Prop) → (Q : Prop) → Exists A p → ((w : A) → p w → Q) → Q`
  ——**结论 `Q` 被钉死在 `Prop`**（inductive 版自动派生的 `Exists.rec` 才能把 motive 提到
  证人上）。后果不是"写不出来"，而是**"取数据"的引理无法证明**：
  · **写得出、证不出**：`theorem surj_split (α β : Type) (f : α -> β)
    (h : Function.Surjective α β f) : Exists (β -> α) (fun g => Function.RightInverse α β g f)`
    本语言**接受这个陈述**（本轮实测：判卷给 `exercise open`，退出码 0——即类型是对的），
    但要证它就得对每个 `y` 用 `Exists.elim` 从 `h y` 里**取出**一个原像来装配函数 `g`，
    而那条消去的结论是个**函数值**（`β -> α` 里的一步），不在 Prop 里 ⇒ 消不掉。
    这条引理就是"满射可裂"，与选择公理等价（本语言没有 `Classical.choice`，
    也没有 `funext`/`propext`）。
  · **改成数据版陈述**：逆函数 `g` 与"它是右逆"的证据**由调用者交出来**，作为前提而不是结论：
    `theorem bijective_iff_inverse (α β : Type) (f : α -> β) (g : β -> α)
     (hg : Function.RightInverse α β g f) : Iff (Function.Bijective α β f)
     (Function.Inverse α β f g)`——单元⑦ 第 6 题证的就是这条，两边都能证。
  于是 `lib/Fun.sokonanoda` 只提供**数据版**的 `LeftInverse`/`RightInverse`/`Inverse`
  （`Inverse α β f g` 是 **Prop，g 是参数**），`Function.Inverse` 这个名字因此**不是 Mathlib
  取证名而是设计判断**（§3 表里已标）。
- **给语言线的备忘**：G-03 修复时**别只改 `Exists` 本体**——还要一并给
  `Exists.rec`（motive 可变，结论可以提到证人），否则卷 II+ 的 `Subtype`/`Sigma`
  照样卡在"取数据"上。

#### C. `And`/`Exists` 都落在 Prop + 语言没有累积性 ⇒ 等势只能用 Prop 值定义

- **想写的**：把等势立成"两个集合之间的结构"，类型写成 `Type`。
- **实测被拒**：本语言**没有累积性（no cumulativity）**，`Prop` 不能当 `Type` 用——
  `axiom P : Prop` 加一行 `def T : Type := P`，`grade` 退出码 1，
  报 **`类型不匹配：期望 Sort(1)，实际是 Sort(0)`**（`kernel-rejected`；本轮实测复现）。
  而等势的四个部件全是命题：`And`（`lib/Logic` 的公理，落 `Prop`）、`Exists`
  （`lib/Exists` 的公理，落 `Prop`）、`Set.MapsTo`/`Set.LeftInvOn`/`Set.RightInvOn`
  （`forall` 定义，落 `Prop`）。
- **变形**：`Set.Equiv α β A B : Prop`，"一对互逆映射 + 四条条件"用**嵌套 `And` + 两次
  `Exists`** 装（外层 `Exists (α -> β)`，内层 `Exists (β -> α)`；要一起绑住一对映射，
  能绑的只有 `Exists`——`Prod` 只装 Type 值，四条条件装不进去）。
  **与任务书的 `: Type` 差一档，这是实测后的设计决定**（文件头写了理由）。
- **两条收益**：本语言的 `theorem` 只能证 `Prop` ⇒ 上层画布/解答能继续用
  `theorem … := sorry` 的写法；用的时候 `Exists.elim` 的 `Q` 不许提证人
  （B 的限制）⇒ 所有"用等势"的结论都必须是只谈 `A` 与 `B` 的命题——**这正是等势该有的样子**。
- **"数据"这一层没有丢**：那一对映射被 `Exists` 的证人位装着，`Exists.elim` 两次就能拿到
  （单元⑨ 所有练习的主力动作），只是**不能**把它当成 `Type` 层的结构来返回。
- **如果哪天要立成 `Type`**：要么换 inductive 结构体（撞 G-01/G-02/G-03 三连），要么自造
  一个 Type 版合取——两者都丢掉"嵌套 `And`"这一条，且要动 `theorem` 的使用面。今天**不做**。

### 3.3 计数是怎么来的（复跑命令 + 两条纪律）

- **判"有没有坏"用 `grade` 的退出码**：`node scripts/soko grade "<绝对路径>"`，0 = 全 checked
  或有合法 open，1 = 解析/elaborate/内核拒绝。**一律给绝对路径**（G-12）。
- **`query check` 取计数**（`data.counts.decl_checked`），需要时也可用它判"有没有坏"：
  **as-built（0.59.0，G-10 已修）**——它在**解析失败**时现在返回 parse 诊断
  （`failed[]`）+ 退出码 1，与 `grade` 同口径；修前是"全零 + `ok:true` + exit 0"（假绿）。
  课程门禁的判据仍用 `grade` 的退出码，不因此换判据。
  （既有实测补充：`query check` 对**内核拒绝**一直如实报——`ok:true` 但
  `data.failed` 里带 `kernel-rejected`。）
- **课程门禁**：`python3 courses/set-theory/tools/check.py`（内部一律绝对路径 + 退出码）。
  **本轮实跑（2026-09-18 晚）**：34 个目标全绿 —— `lib/` 8 个文件 **94 checked / 0 open**，
  12 个单元 **93 open**，12 份解答 **132 checked / 0 open**；合计
  **296 checked · 93 open · 0 个被判负**。
  **P4 复测（2026-09-19，`--json`）**：**36 个目标 · 329 checked · 99 open ·
  0 判负**（`summary`：`canvas_open` 96 / `solutions_open` 0 / `lib_open` 0）。
  口径提醒：`summary.checked` 把 `lib/Demo.sokonanoda` 算两次（`lib Demo` 与
  `lib 自检` 判同一个文件），所以逐文件相加会比汇总额少 10。
- **已知欠账（本轮查出来的，别看它小）**：`lib/Demo.sokonanoda` 只 `import lib.Logic` 与
  `lib.Set`——**`Prod`/`Exists`/`Rel`/`Fun`/`Image`/`Equiv` 6 个模块还没有自检行**
  （各模块文件头都留了"应补进 Demo，但不在本次归属范围内"的 TODO）。今天门禁仍然覆盖它们
  （`check.py` 会**逐个**判 `lib/*.sokonanoda`），所以没有假绿；但"一个入口 import 全部模块"
  这条 `teaching-project.md:397` 的验收标准还差 6 行。

## 4. 与语言缺口的关系（谁挡着谁）

| 缺口 | 影响本方案的哪一条 |
|---|---|
| **G-02** 构造子无命名空间 | **已定形并已跟随**：prelude 的 `And`/`Or` 走真归纳块 + 点号构造子（`And.intro`/`Or.inl`/`Or.inr`），课程侧 P4 把 65 处项位裸名改成点号名（§2/§3.2-A）；裸**项**名已不存在，裸**模式**仍被接受。**仍然受影响的只有 `Prod`**：`lib/Prod.sokonanoda` 的构造子只能是 `prod_mk`（改名计划与兼容策略见 §3.2-A） |
| **G-03** Prop+Type 参数归纳被拒 | `Exists` 只能立公理三件套（`lib/Exists`，4 条）；**⇒ §3.2-B**：`Exists.elim` 的 `Q` 只能是 Prop ⇒ "满射 ⇒ 有右逆函数"这类**取数据**的引理写不出证明，单元⑦ 只能证数据版 |
| **无缺口号：语言没有累积性** | **⇒ §3.2-C**：`And`/`Exists` 都落 `Prop`，`def T : Type := <Prop 值>` 被内核拒（期望 `Sort(1)` 实际 `Sort(0)`）⇒ `Set.Equiv … : Prop`（等势的 `: Type` 版本立不起来）。**尚未记入 `docs/gaps/ledger.jsonl`** |
| **G-04** 无 notation | L2 只能用点名；记法作为第二遍的 X 类练习 |
| **G-17**（已登记，0.59.0 与 G-10 同轮修） | `docs/gaps/WO-003` §"不做的事"第 1 条曾记"`query goals`/`query holes` 对 parse 失败返回空数组（同族假绿，G-10 的兄弟）"⇒ 已另立为 **G-17** 并修掉（解析失败答 `ok:false` + `not-parsable` + exit 1）；`docs/gaps/WO-009` 的 `def f.{u}` 被解析成 `f.` 并静默 checked 仍是**另一条未登记**的缺口（勿混用编号） |
| **G-12** 相对路径 + 祖先清单 | 课程仓判卷必须绝对路径（§3.3 的复跑命令） |
| **G-14** 单宇宙 binder | `congrArg`/复合/像的跨宇宙版本写不出来（L1 只能同层）；`Exists` 的论域只能 `Type 0`（带宇宙层级的 `Exists` 今天写不出来） |
| **G-15** 内核错误 span 不准 | 写库/写解答时的定位成本；试做里靠二分硬扛 |
| **L-03** `Eq.subst` 只支持 Prop motive | `Eq.mp`/`cast` 不可表达；卷 II+ 会挡路（§3.2-B 是同一个病根的另一面：**Type 层的重写没有入口**） |

## 5. 落地顺序（P1 之后接进 `teaching-project.md` 的 P3/P4）

> 进度标记口径：✅ = 已在仓库里可复跑；⬜ = 未做。复跑命令见 §3.3。
> **编号注意**：本节 P-C1…P-C5 与 `teaching-project.md` §P-C 的 **P-C1…P-C6 是对齐的**
> （那份的 **P-C6** = 把课程门禁接进 `scripts/soko gate` 或 CI——**本文档不重复它**，
> 所以本轮新增项从 **P-C7** 起编号，不用 P-C6）。

1. **P-C1 ✅**：按 L-05 把 16 条 D 类从 `lib/Set` 移进 `units/`——**已全部执行完**
   （第一刀 6 条，剩余 10 条随单元③④ 一起落；今天 `lib/Set` 只剩定义/公理/展开引理）；
2. **P-C2 ✅**：写 `lib/` 规范文档进课程仓（§3 的表 + 命名纪律 + 判据）——**本轮补齐**；
   `spike/lib/*` 已按 Loogle 取证名搬运并扩展；
3. **P-C3 ✅**：向语言线提 **L1 prelude 提案**（§2 的清单 + 三件套），牵动入门课 golden，
   需一次改完。**当时的论据**：`lib/Logic` 26 条已是可运行、判卷全绿的真库，
   语言线直接拿它当了 prelude 草稿与验收样本（§2 末的"测点"）。**已落地**
   （0.59.0，P1/P2/P3；设计 `docs/design/prelude-l1-proposal.md`）；
4. **P-C4 ✅（大幅推进）**：单元 3–12 逐单元试做——**12 个单元 + 12 份解答已全部落地**，
   门禁 34 个目标全绿（296 checked · 93 open · 0 判负）。标准库随之长出 6 个模块
   （`Exists`/`Prod`/`Rel`/`Fun`/`Image`/`Equiv`），并逼出 §3.2 的三条变形；
5. **P-C5 ✅**：`gap.py list --kind library` 视图（标准库欠账单独可见）；
6. **P-C7 ⬜（本轮新增）**：把 6 个模块补进 `lib/Demo.sokonanoda` 的自检入口
   （`Prod`/`Exists`/`Rel`/`Fun`/`Image`/`Equiv` 各一行 `import` + 两条演示）——
   这是 `teaching-project.md:397` 的验收标准最后差的一块（§3.3）；
7. **P-C8 ⬜（本轮新增）**：**决定 G-16 到底记哪一条**。现在两处 WO 各自"建议 G-16"而
   指的是两件不同的事（§4 那一行）；**先登记、再引用**——登记完把 §4 那格改成正式条目
   （本文档不替台账做决定：课程线的纪律是"台账是唯一真相"）；
8. **P-C9 ⬜（本轮新增）**：**给"没有累积性"补一个缺口号**（§3.2-C 是实测复现，
   但 `docs/gaps/ledger.jsonl` 里没有这一条）。它今天只挡"等势立成 `Type`"这一件事，
   但卷 II（子类型/`Sigma`/结构体）会反复撞——值得在写卷 II 之前进台账。

9. **P-C10 ✅（2026-09-19，本文档本轮新增）**：**卷 I 跟随已落地的 L1 prelude
   （= 提案 §6 的 P4）**：`lib/Logic.sokonanoda` → 只有注释的空壳；
   课程侧 65 处项位裸名 `inl`/`inr` → `Or.inl`/`Or.inr`（+27 行注释）；
   门禁 `python3 courses/set-theory/tools/check.py` **exit 0 · 36 目标 ·
   329 checked · 99 open · 0 判负**（差值是 `Logic` 的 −26，open 不变）。
   这一条做完，"课程侧的标准库暴力"（L-01/L-02 那 26 条抄本）才算真正消除。

**还没被吃到的库（写卷 II 前先想清楚）**：`lib/Prod` 到今天**只有单元⑤ 在用**，
单元⑩ 的基数段绕开了它（`Set.Equiv` 的数据形状不需要 `Prod`）；`Rel`/`Fun`/`Image`
在单元⑫ 综合课里汇合。**别为了让库"看起来完整"而加 API**——判据仍是 §0 那一句。

## 6. 判据的自动化（下一步值得做）

"纯定义展开"可以机械识别：一个引理若**删掉名字后剩下 `Iff.intro … (fun h => h)` 或
`rfl` 级**，就属于 L2；否则 L3。写库时人工判一次即可，但可以加一条 lint：
`lib/` 里出现 `Or.elim`/`And.intro`/`Eq.subst` 之外的结构性证明 ⇒ 提示"这可能该是练习"。
（先不做，记在这里。）

**本轮新长出来的三条自动化候选**（都来自上面的实测，按性价比排序）：

1. **计数一致性 lint**（最便宜）：对每个 `lib/*.sokonanoda` 同时跑 `grade`（退出码）与
   `query check`（`counts.decl_checked`），两边的**声明数必须相等**；不等就是 G-10 那类
   假绿或解析半途失败。今天靠人工对表（§3 的计数就是这么来的）。
2. **依赖图 lint**：解析每个模块的 `import` 行，检查 §3 那张"哪些单元吃哪些模块"的表
   没有漂移（今天这张表是手工核对的——`check.py` 只判卷，不查依赖关系）。
3. **"没有累积性"陷阱 lint**（教学侧）：课程序列里出现 `def/theorem … : Type := <Prop 项>`
   就给提示——它是本语言最反 Lean 直觉的一条（§3.2-C），也是写库时最容易踩的坑。
   （G-01 修好之前，这条 lint 只能靠文本形态，不能靠 `query check`。）

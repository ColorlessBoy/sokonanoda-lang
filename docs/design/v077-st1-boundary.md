# v0.77.0 · ST1 决策记录：**哪些用类型论自身表达、哪些确实必须外挂**

> ⚠ **为什么这份 v0.77 的记录还在活文档里** ✓：`crates/cli/tests/st1_boundary.rs`
> **逐条读本文件**（`repo_root().join("docs/design/v077-st1-boundary.md")`）拿分界结论去对账内核
> ⇒ **它是活契约测试的输入**，不是快照 ✗。**改动本文件的表格 = 改判据**（两边必须一起动）。
>
> 环节：`docs/PLAN-0.74-0.79.md` 的 🚀 v0.77.0 · **ST1**（批次详情已归档 ⇒
> `docs/archive/v077-snapshots-2026-09-30/v077-set-theory.md.gz`；未做完的四章与
> G-56/58/59 根因见活文档 `docs/design/v077-kernel-deficiencies.md` §三）。
> 用户原话（2026-09-27 指正）：**「不要把造 ZF 宇宙当目标」** —— 主线是**类型论自身的表达**
> （`Set α := α → Prop` · `Quot`/`Setoid` · 宇宙层级），**只有确实写不出来的才讨论外挂**。
> 纪律：**调研必须留痕**（出处逐条给出 URL）；**不许凭记忆编**；分界由本记录钉住，
> 由 `crates/cli/tests/st1_boundary.rs` 对账（**记录 ↔ 内核**两端，见 §对账表）。

## §1 结论（一句话）

**四条分界：两条「用类型论自身表达」、一条「必须有商」、一条「两条路都要良基递归」。**

| # | 项 | 结论 | 依据（出处见 §2） |
|---|---|---|---|
| 1 | `Set α` 谓词式（分离 / 无限并交 / 幂集） | ✅ **自身表达**，**零外挂** | Mathlib `Set α := α → Prop` 就是这一层；Isabelle `down_raw`/`Union_raw` 同样只用 `V ⇒ bool` |
| 2 | **序数** | ✅ **自身表达**（写成**谓词**） | Isabelle `Ord x ≡ Transset x ∧ …`（`V ⇒ bool`）；**全体序数 `ON` 不是集合**（Burali-Forti）⇒ 序数**本来就不该是一个类型** |
| 3 | **基数** | ✅ **商已在源语言可用**（ST2 落地，2026-09-28） | Mathlib `Cardinal := Quotient Cardinal.isEquivalent`；Isabelle 用 `LEAST` 选代表绕开（代价：每条定理都带 `Ord i`）⇒ **用户 2026-09-28 拍板走路线 A**：`Quot` 装进 prelude，基数不再被商挡住 |
| 4 | **秩 rank**（及其前置：超限递归 / V 层级） | ⚠️ **两条路都要「良基递归可用」**；且 **rank 作为函数**还要商 | Isabelle `transrec ≡ wfrec {(x,y). x ∈ elts y} H`（良基性来自 `foundation` **公理**）；Mathlib `rank (h : Acc r a) : Ordinal` 走 `Acc.recOn`（**不是** `WellFounded.fix`）—— 递归本身零 `Quot`，**值域 `Ordinal` 才要商** |
| 5 | **传递闭包 / 超限递归 / ω₁ / Aleph** | ⚠️ 同上（ST6/ST9/ST11 逐章再判） | Isabelle §2.5/§2.13/§2.20/§2.21 |

**⇒ 对本仓库的直接含义**：`Set α` 谓词式**已经够用**（结论 1/2 已实测通过，见 §对账表）；
**卡住整条 v0.77 的不是"缺 ZF 公理"，是两件更小的东西** ——
**(a) 源语言里没有 `Quot`**（结论 3）、**(b) `Acc` 立不起来 ⇒ 没有良基递归**（结论 4）。

**⚠ 更新（2026-09-28，ST2 落地后）**：**(a) 已修** ✓ —— 用户拍板路线 A，`Quot`/`Quot.mk`/
`Quot.lift`/`Quot.ind`（`Declar::Quot`）+ `Quot.sound`（唯一公理）已装进 prelude，
判据在**归约**上（`Quot.lift f h (Quot.mk r a)` 必须与 `f a` 定义相等）。
**(b) 仍未修**（G-56，blocker）—— 那是 ST7/ST9 的门槛。

## §2 基准（**每条都已实查**，动手前必须再读一遍）

| 基准 | 关键事实（逐字） | 出处 |
|---|---|---|
| **Mathlib · 序数** | `structure WellOrder` + `instance Ordinal.isEquivalent : Setoid WellOrder` + `def Ordinal : Type (u + 1) := Quotient Ordinal.isEquivalent` —— **序数是良序的商**。⚠ 文件是 `Ordinal/Basic.lean`（`Ordinal/Defs.lean` **404**） | <https://raw.githubusercontent.com/leanprover-community/mathlib4/master/Mathlib/SetTheory/Ordinal/Basic.lean> |
| **Mathlib · 基数** | `instance Cardinal.isEquivalent : Setoid (Type u)`（`r α β := Nonempty (α ≃ β)`）+ `def Cardinal : Type (u + 1) := Quotient Cardinal.isEquivalent`；`Cardinal.lift` 存在正因为**没有累积性** | <https://raw.githubusercontent.com/leanprover-community/mathlib4/master/Mathlib/SetTheory/Cardinal/Defs.lean> |
| **Mathlib · 秩** | `noncomputable def rank (h : Acc r a) : Ordinal.{u} := Acc.recOn h fun a _h ih => ⨆ b : { b // r b a }, Order.succ (ih b b.2)` —— **递归零 `Quot`，值域要商** | <https://raw.githubusercontent.com/leanprover-community/mathlib4/master/Mathlib/SetTheory/Ordinal/Rank.lean> |
| **Mathlib · 谓词层** | `def Set (α : Type u) := α → Prop`（`Set.sep` / `Set.sUnion` / `Set.sInter` 就在这一层）—— **与本仓库同构** | `Mathlib/Data/Set/Defs.lean` |
| **Mathlib · ZF 层** | `structure PSet` + `instance setoid : Setoid PSet` + `def ZFSet : Type (u + 1) := Quotient PSet.setoid` —— 那是**在类型论内部造 ZF 模型**（元数学用途），**不是谓词式那一层的必需件** | `Mathlib/SetTheory/ZFC/{PSet,Basic}.lean` |
| **Lean core · `Acc` 零商** | `inductive Acc {α : Sort u} (r : α → α → Prop) : α → Prop` + `WellFounded.fix`（`src/Init/WF.lean`，全文件 `Quot` 出现 **0** 次）⇒ **良基递归本身不需要商** | <https://raw.githubusercontent.com/leanprover/lean4/master/src/Init/WF.lean> |
| **Isabelle AFP `ZFC_in_HOL` · 序数** | `definition Ord where "Ord x ≡ Transset x ∧ (∀y ∈ elts x. Transset y)"`（`V ⇒ bool`）；`lemma big_ON [simp]: "¬ small ON"`（全体序数**不是**集合） | <https://isa-afp.org/entries/ZFC_in_HOL.html> · `ZFC_in_HOL.thy` §1.4 |
| **Isabelle AFP `ZFC_in_HOL` · 基数** | `definition vcard where "vcard a ≡ (LEAST i. Ord i ∧ elts i ≈ elts a)"`；`definition Card where "Card i ≡ i = vcard i"` —— **谓词 + 选择**，全篇 `quotient` 零出现 | `ZFC_Cardinals.thy` §2.7 |
| **Isabelle AFP `ZFC_in_HOL` · 秩** | `definition transrec where "transrec H a ≡ wfrec {(x,y). x ∈ elts y} H a"`；良基性来自公理 `foundation: "wf {(x,y). x ∈ elts y}"` | `ZFC_in_HOL.thy` §1.5 · `ZFC_Cardinals.thy` §2.6 |
| **Isabelle AFP `ZFC_in_HOL` · 设计目标原话** | "the point is to have the closest possible integration with the rest of Isabelle/HOL, **minimising the amount of new notations and exploiting type classes**" | AFP 摘要（同上 URL） |
| **Lean TPiL §12.4** | `Quot`/`Quot.mk`/`Quot.ind`/`Quot.lift` 属**逻辑框架**（不算额外公理）；**只有 `Quot.sound` 是公理**；`Quotient`/`Setoid` 是它的特化 | <https://lean-lang.org/theorem_proving_in_lean4/Quantifiers_and_Equality.html> |
| **本仓库内核** | **已经内建商**：`Declar::Quot` 声明种类 + `Quot.lift`/`Quot.ind` 的 iota 归约 + `RigidHead::QuotConst`（`crates/kernel/src/quot.rs`、`eval.rs:1379-1411`）；`Quot.sound` 在 `STANDARD_AXIOMS`（`util.rs:34`） | 本仓库（**不是**外部基准） |

## §3 两条路的代价（**摆给用户，不自己拍**）

* **路线 A（Mathlib 式：商）** —— 序数/基数/`ZFSet` 都是商。
  **要付**：源语言先得有 `Quot`（= **ST2 入场券**），并且要能"把命题抬成类型"（累积性 L-06）。
  **换来**：`Ordinal`/`Cardinal` 是**一等类型**，可以当参数、当返回类型、开类型类 —— Mathlib 的
  全部序数算术都建立在这上面。
* **路线 B（Isabelle 式：谓词 + 选择）** —— 序数/基数写成 `V ⇒ bool` 谓词，基数用 `LEAST` 选代表。
  **要付**：每条定理都带 `Ord i` / `Card i` 假设；`vcard` 需要**选择**（我们连"满射可裂"都证不出来，
  见 L-06 原文）；而且 Isabelle 的 `V` 是**公理化的抽象类型**（7 条公理）——**那才是"外挂"**。
  **换来**：不需要商。
* **✅ 这一刀已由用户拍（2026-09-28）：走路线 A —— 做 ST2**，把 `Quot` 暴露到源语言。
  理由（用户核实过）：① 内核**已经内建**商（`crates/kernel/src/quot.rs` 12KB、
  `RigidHead::QuotConst` 进了 `conv.rs`、`Quot.sound` 在 `STANDARD_AXIOMS`、
  `util.rs` 按名查找四条）⇒ 缺的只是**声明没暴露**，成本远低于路线 B；
  ② 路线 B 要付的「选择代表」（Isabelle 的 `LEAST`）我们**也没有**（L-06：连满射可裂
  都证不出来），且每条定理都要带 `Ord i`/`Card i` 假设 ⇒ 更贵还更丑；
  ③ 基数（ST10）、Aleph/ω₁（ST11）是 **`Ordinal` 上的函数**，路线 B 的窄路救不了。
  **落地情况**：见 G-57（fixed_in 0.77.0）与 `docs/gaps/repro/ST2-*.sokonanoda`。
* **不拍那一刀也能走的一条窄路**（Mathlib 调研带出来的，记在这里供用户选）：
  **把 rank 写成关系/谓词**（`Rank r a o : Prop`）而不是"值域是 `Ordinal` 的函数"——
  因为 `Acc`/`WellFounded` **零 `Quot`**（Lean core 实测），而"函数值域是商类型"才要商。
  代价：序数算术的**全部**定理都要带 `Ord o` 假设（Isabelle 路线 B 的老问题）。
  **⇒ 这条窄路只能救 ST7/ST9 的一半，救不了 ST10/ST11**（基数与 Aleph 是 `Ordinal` 上的函数）。

## §4 影响面（逐条落到 ST 号）

| 分界结论 | 影响 | 落点 |
|---|---|---|
| 1 `Set α` 谓词式够用 | ST3（`{x : P}`）/ ST4（`⋃₀`/`⋂₀`）**不需要任何新机制**，只差 binder 记法 | ST3 · ST4 |
| 2 序数是谓词 | ST8（序数）**不需要商**；但"全体序数"只能当**类**（`Set V`），不能当类型 | ST8 |
| 3 基数必须有商 | **ST10 卡在 ST2**；不做 ST2 ⇒ 基数只能写成"等势关系 + 代表选择"，且要付 L-06 | ST2 · ST10 · ST11 |
| 4 良基递归 | **ST7（秩）/ ST9（超限递归）/ ST11（序型）** 全卡在这里；**这一条两条路都要**（Isabelle 靠公理、Mathlib 靠 `Acc`） | ST6 · ST7 · ST9 · ST11 |
| 缺口的**新**登记 | `Acc` 立不起来是本轮**新发现**（既有台账只有 L-06 的累积性与 `Exists.elim`，没有这一条） | `docs/gaps/ledger.jsonl` 的 **G-56** |

## §对账表

> 形状**固定 4 列**（复现件 / 判据 / `decl.checked` 数 / 逐字诊断），
> 由 `crates/cli/tests/st1_boundary.rs` 解析并**两端对账**：
> 记录里写的诊断必须在探针输出里**逐字**出现；探针输出的每条诊断也必须在记录里找到
> ⇒ 记录腐烂、或探针改行为，**两边都会判红**。
> **实测咬过一次** ✓：ST2 把 `Quot` 装进 prelude 之后，本表的
> `ST1-quot-unavailable` 行**当场判红**（记录 0 / 内核 1）⇒ 记录随之更新：
> 商那一半搬到 ST2 探针，本表只留**仍然成立**的「没有累积性」（L-06）。

| 复现件 | 判据 | checked | 逐字诊断（多条用 `;;` 分隔，**不加引号**） |
|---|---|---|---|
| ST1-predicate-set-operations.sokonanoda | positive | 7 | — |
| ST1-predicate-ordinal.sokonanoda | positive | 7 | — |
| ST1-no-cumulativity.sokonanoda | negative | 0 | kernel-rejected ;; 类型不匹配：期望 Pi (P : Sort(0)), Sort(1)，实际是 Pi (P : Sort(0)), Sort(0) |
| ST1-acc-well-founded-recursion.sokonanoda | negative | 0 | kernel-rejected ;; rejected: inductive occurrence is not applied uniformly to the block parameters and universe levels |

## §不做什么（本环节的边界）

* **不做 ST2**（商类型实现）—— 用户明确"等我对 ST1 的决策记录确认后再开"。
* **不新增语法**、**不动内核判定**、**不改课程内容** —— ST1 的产出是**判断 + 判据**，不是实现。
* 逐章铺开（ST3–ST15）等 ST1 确认后再排 —— 本记录只给"哪一章会撞在哪一条分界上"。

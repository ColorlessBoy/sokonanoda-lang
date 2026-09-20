# 调研：隐式实参 + 记法输入落地后，课程侧要改什么（影响面清单 + 计划）

> 日期：本轮。**只读调研**——仓库里一个字节都没改（探针全部落在 `/tmp`，课程树零改动）。
> 触发（用户原话）：「如果想支持 notation，感觉 '{x : Set}' 这种隐参数自动推导的机制
> 不得不实现了。」——即让 `{α : Type}` 这类隐式 binder 在**应用位自动补**，
> 从而取代记法路径那个「补前导类型参数」的 hack。
>
> 命令口径：`node scripts/soko grade <绝对路径> --json`（G-12）。
> **没有跑全量判卷**（`check.py` 只读代码不执行）。所有"实测"都在 §10 列出探针文件与结论。
>
> 上游文档：`docs/design/course-lean-style.md`（L2.9/N-1/N-8 明确写着"不做隐式实参"——
> 本调研的结论会推翻其中三条，见 §0.7）、`docs/design/notation-subset.md`（N4 补前导参数）、
> `courses/set-theory/README.md`、`courses/set-theory/AGENTS.md`。

> **⚠️ 评审修正（2026-09-19）**：本文的**计数**有几处要按复算修正——点名调用
> **2768**（本文写 2784）、hint/叙事 **712**（本文写 765）、涉及文件 **35**（本文写 30）、
> `lib/Image` **6** 条 / `lib/Equiv` **5** 条（本文写 4 / 3）；"带前导类型实参 2308" 是
> 正则近似、**不可复算**，引用时只说比例。**§8.4 那条"计数口径（复现用）"的命令是错的**
> （它没去注释，复现的是含注释那一列）。修正后的口径表在
> `docs/design/implicit-arguments.md` §5——**以那份为准**。

---

## 0. 结论速览（先看这十条）

1. **`lib/Set` 的 12 个 def + 1 axiom + 10 条引理里，只有 `def Set`（类型构造子本身）
   必须保持显式**；其余 22 条的前导 `α` 按 Mathlib 惯例都应隐式（§1 表）。
2. **调用点规模：30 个文件 / 1520 行 / 2784 处点名调用，其中 2308 处写了前导类型实参**
   （≈83%）。含注释共 3549 处（注释里的 765 处是 hint/教学叙事的同步工作量，不判卷）。
3. **`lib/Set` 改隐式参数不会让记法当场坏掉**（反直觉，已实测）：
   `notation_telescope` 的 `peel_pi` **照样剥隐式 binder**，而**内核不看 binder 风格**
   ——`Set.mem α a A`（在隐式位上写显式实参）仍然 well-typed。所以「签名改隐式 +
   记法不动」是一个**自洽的中间态**（§4.3 探针 A/B）。
4. **但那个补参 hack 不能直接删**：`elab_notation` 用 `builder.mk_app` **直接造核项、
   绕过了 `elab_expr` 的 `Expr::App` 分支**（`elab.rs:1955-1961`）——隐式插入若只做在
   App 分支里，记法路径一行都吃不到。要删 hack，必须让记法路径**改走 `elab_expr`**
   或复用新的插入例程（§4.2）。
5. **`∃ x, p x` / `∀ x, p x`（省标注）仍然不行**：隐式实参补的是**常量的参数**，
   不是 **λ binder 的类型**；`binder_notation_operand`（`elab.rs:1120-1179`）今天只有
   「标注」与「guard 反解」两个类型来源。要 `∃ x, p x` 得先有元变量/一般合一（§5）。
6. **`=` 不会因为隐式实参而解锁**：`α` 可以从操作数类型解出，但 `α : Sort u` 是
   **Sort 位的隐参数**，`u` 仍要"问内核要 sort"；加上 `=` 今天在**词法层**就报
   `expected =>, found =`。⇒ L2.4c 的 R2 spike 结论不变（§6）。
   另实测：`rfl` **不穿透 delta**（`abbrev Eq'` 也救不了），所以"单宇宙 `Eq'` 绕道"
   这条备选路也堵着（§6.3）。
7. **`course-lean-style.md` 里三条"不做"会被推翻**：D2 表格行（`:49`）、
   L2.9（`:285`）、N-1（`:660`）、N-8（`:667`）；`notation-subset.md` 的 N4.2/N4.3
   与 §13.3 同理（§7）。
8. **会红的测试共 4 条硬红 + 3 条"随课程改写而红"**，全部在
   `crates/cli/tests/notation.rs`（§8.1）；`query.rs:559` 那条钉 unit05 计数，随改写红。
9. **`check.py` 的 G1–G6 本身不会因"参数变隐式"判红**（判据与规模无关、不锁计数）；
   红的是 G1（某个单元 `grade` 不再 exit 0）与 G4（按名覆盖）——这两条是**改写质量**
   的红，不是判据的红（§8.2）。
10. **顺序上有一个硬前置**：`courses/set-theory/units/solutions/unit01-solution.sokonanoda`
    今天 **exit 1**，卡在最后一条声明（`def Set` + `namespace Set` 的 judge 重编译 bug）；
    它必须在"改 `lib/Set` 签名"之前修好，否则改完分不清是隐式实参坏了还是老 bug 坏了（§8.3）。

---

## 1. `lib/Set.sokonanoda` 全貌与目标签名表

文件 177 行；`import lib.Logic`（`:47`）；`def Set` 在**根命名空间**（`:51`）、
其余 11 个 def 与 1 条 axiom、10 条引理写在 `namespace Set`（`:53`–`:134`）里；
记法块在 `:155`–`:177`。

### 1.1 现状逐字签名（23 条）

| # | 行 | 现状签名（逐字） | 备注 |
|---|---|---|---|
| 1 | `:51` | `def Set (α : Type) : Type` | 类型构造子，**根命名空间** |
| 2 | `:55` | `def mem (α : Type) (a : α) (A : Set α) : Prop` | |
| 3 | `:57` | `def subset (α : Type) (A B : Set α) : Prop` | |
| 4 | `:59` | `def empty (α : Type) : Set α` | |
| 5 | `:61` | `def univ (α : Type) : Set α` | |
| 6 | `:63` | `def singleton (α : Type) (a : α) : Set α` | |
| 7 | `:65-66` | `def pair (α : Type) (a b : α) : Set α` | |
| 8 | `:68` | `def union (α : Type) (A B : Set α) : Set α` | |
| 9 | `:70` | `def inter (α : Type) (A B : Set α) : Set α` | |
| 10 | `:72` | `def sdiff (α : Type) (A B : Set α) : Set α` | |
| 11 | `:74` | `def compl (α : Type) (A : Set α) : Set α` | |
| 12 | `:76` | `def powerset (α : Type) (A : Set α) : Set (Set α)` | |
| 13 | `:79-80` | `axiom ext : (α : Type) -> (A B : Set α) -> (forall (x : α), Iff (A x) (B x)) -> Eq.{1} (Set α) A B` | **公理** |
| 14 | `:83-84` | `theorem subset_def (α : Type) (A B : Set α) : Iff (subset α A B) (forall (x : α), A x -> B x)` | |
| 15 | `:89-90` | `theorem mem_empty_iff_false (α : Type) (a : α) : Iff (mem α a (empty α)) False` | |
| 16 | `:95` | `theorem notMem_empty (α : Type) (a : α) : Not (mem α a (empty α))` | |
| 17 | `:98` | `theorem mem_univ (α : Type) (a : α) : mem α a (univ α)` | |
| 18 | `:100-101` | `theorem mem_singleton_iff (α : Type) (a b : α) : Iff (mem α a (singleton α b)) (Eq.{1} α a b)` | |
| 19 | `:106` | `theorem mem_singleton_self (α : Type) (a : α) : mem α a (singleton α a)` | |
| 20 | `:109-110` | `theorem mem_union (α : Type) (a : α) (A B : Set α) : Iff (mem α a (union α A B)) (Or (mem α a A) (mem α a B))` | |
| 21 | `:115-116` | `theorem mem_inter_iff (α : Type) (a : α) (A B : Set α) : Iff (mem α a (inter α A B)) (And (mem α a A) (mem α a B))` | |
| 22 | `:121-122` | `theorem mem_sdiff (α : Type) (a : α) (A B : Set α) : Iff (mem α a (sdiff α A B)) (And (mem α a A) (Not (mem α a B)))` | |
| 23 | `:128-129` | `theorem mem_powerset_iff (α : Type) (A B : Set α) : Iff (mem (Set α) B (powerset α A)) (subset α B A)` | |

记法声明（`:164`–`:177`，**11 条**，注意与 README 里"五个第二刀符号"的说法不一致——
今天 lib/Set 里其实是 **11 条**：core 级六个 `∈ ⊆ ∪ ∩ \ ∅` + Mathlib 级五个
`𝒫 ᶜ '' ⁻¹' ×ˢ`）：

```
infix:50 " ∈ " => Set.mem          infix:50 " ⊆ " => Set.subset
infixl:65 " ∪ " => Set.union       infixl:70 " ∩ " => Set.inter
infixl:70 " \ " => Set.sdiff       notation "∅" => Set.empty
prefix:100 " 𝒫 " => Set.powerset   postfix:100 " ᶜ " => Set.compl
infixr:80 " '' " => Set.image      infixr:80 " ⁻¹' " => Set.preimage
infixr:80 " ×ˢ " => Set.prod
```

> ⚠️ `'' ⁻¹' ×ˢ` 的三目标 `Set.image` / `Set.preimage` / `Set.prod` **不在本文件**：
> 前两个在 `lib/Image.sokonanoda:42/50`，`Set.prod` 是**单元⑤ 画布给的词汇**
> （`units/unit05-pairs-products.sokonanoda:100`，签名 `def Set.prod (A B : Type) (s : Set A) (t : Set B) : Set (Prod A B)`）。
> ⇒ 「`lib/Set` 改隐式」**只是 23 条**；`Image`/`Equiv`/单元⑤ 的签名是**另外三批**
> （§9.2）。

### 1.2 目标签名表（隐式 / 显式判定）

判据：**Mathlib 惯例**（`course-stdlib.md` 的"名字照抄真名"纪律）+ **本语言能不能解出**。
"解出"一栏写的是隐式位**从哪里来**：`操作数` = 从某个显式实参的类型头部反解；
`期望` = 只能从期望类型解（无操作数时唯一的路）。

| 目标签名（建议） | 隐式位 | 解出 | 与 Mathlib 对齐 | 备注 |
|---|---|---|---|---|
| `def Set (α : Type) : Type` | — | — | ✅ 同（`Set` 无 binder 风格） | **必须显式**：`Set` 单独出现时 α 无处可解 |
| `def mem {α : Type} (a : α) (A : Set α) : Prop` | `α` | 操作数 `a` | ✅ | 438/545 处调用会变短 |
| `def subset {α : Type} (A B : Set α) : Prop` | `α` | 操作数 `A` | ✅ | 162/163 |
| `def empty {α : Type} : Set α` | `α` | **期望** | ✅ | ⚠️ 无操作数；`Eq.{1} (Set α) A ∅` 这类位置仍可能解不出（§4.4） |
| `def univ {α : Type} : Set α` | `α` | **期望** | ✅ | 同上；216/216 |
| `def singleton {α : Type} (a : α) : Set α` | `α` | 操作数 `a` | ✅ | |
| `def pair {α : Type} (a b : α) : Set α` | `α` | 操作数 `a` | ✅ | |
| `def union {α : Type} (A B : Set α) : Set α` | `α` | 操作数 `A` | ✅ | |
| `def inter {α : Type} (A B : Set α) : Set α` | `α` | 操作数 `A` | ✅ | |
| `def sdiff {α : Type} (A B : Set α) : Set α` | `α` | 操作数 `A` | ✅ | |
| `def compl {α : Type} (A : Set α) : Set α` | `α` | 操作数 `A` | ✅ | |
| `def powerset {α : Type} (A : Set α) : Set (Set α)` | `α` | 操作数 `A` | ✅ | |
| `axiom ext {α : Type} (A B : Set α) (h : ∀ x, Iff (A x) (B x)) : Eq.{1} (Set α) A B` | `α` | 操作数 `A` | ⚠️ Mathlib 连 `A B` 也隐式 | **建议先只隐 `α`**：`A B` 隐式要"从期望类型 `Eq … A B` 解"，属于最难的一档；且 `apply Set.ext` 会从"两个子目标"变"一个"，要重钉课程叙事 |
| `theorem subset_def {α : Type} (A B : Set α) : …` | `α` | 操作数 `A` | ⚠️ Mathlib 是 `{s t}` 全隐式 | 同上；全隐式则 `exact Set.subset_def` 零实参（更 Lean），但要期望类型解 |
| `theorem mem_empty_iff_false {α : Type} (a : α) : …` | `α` | 操作数 `a` | ✅ | |
| `theorem notMem_empty {α : Type} (a : α) : …` | `α` | 操作数 `a` | ✅ | |
| `theorem mem_univ {α : Type} (a : α) : …` | `α` | 操作数 `a` | ✅ | |
| `theorem mem_singleton_iff {α : Type} (a b : α) : …` | `α` | 操作数 `a` | ✅ | |
| `theorem mem_singleton_self {α : Type} (a : α) : …` | `α` | 操作数 `a` | ✅ | |
| `theorem mem_union {α : Type} (a : α) (A B : Set α) : …` | `α` | 操作数 `a` | ⚠️ Mathlib 连 `A B` 隐式 | 建议只隐 `α` |
| `theorem mem_inter_iff {α : Type} (a : α) (A B : Set α) : …` | `α` | 操作数 `a` | ⚠️ 同上 | |
| `theorem mem_sdiff {α : Type} (a : α) (A B : Set α) : …` | `α` | 操作数 `a` | ⚠️ 同上 | |
| `theorem mem_powerset_iff {α : Type} (A B : Set α) : …` | `α` | 操作数 `A` | ⚠️ Mathlib `{s t}` | |

**必须显式的参数**（两类，别顺手隐掉）：

1. **类型构造子自己的参数**：`Set α` 的 `α`（`def Set (α : Type)` 无 binder 风格）。
   全课程 300+ 处 `(A : Set α)` 都靠它。
2. **"内容"实参**：元素 `a`/`b`/`x`、集合 `A`/`B`/`C`、`f`、`h`、`Q`。
   它们是每个引理的**主语**，隐掉会让 `apply`/`exact` 的目标形态失去信息。

**另外三批同类目标**（不在 `lib/Set`，但同一刀会碰）：

| 文件:行 | 签名 | 建议 |
|---|---|---|
| `lib/Exists.sokonanoda:86` | `inductive Exists (A : Type) (p : A -> Prop) : Prop` | `{A : Type}`；`p` 可隐（从 `h : Exists A p` / `f` 的头部反解），但**inductive 头部的 binder 风格**要单独确认（`:62-64` 记着"inductive 头部今天不吃宇宙 binder"，风格是否吃未测） |
| `lib/Exists.sokonanoda:87` | `ctor intro (w : A) (h : p w) : Exists A p` | `{A} {p}`：`A` 从 `w`、`p` 从 `h`（头部 `p w` 反解）。**32 处 `Exists.intro α …` 变短** |
| `lib/Exists.sokonanoda:92-93` | `def Exists.elim (A : Type) (p : A -> Prop) (Q : Prop) (h : Exists A p) (f : …) : Q` | `{A} {p}`：`A` 从 `h`、`p` 从 `h`（`Exists A p` 头部）。**40 处 `Exists.elim α …` 变短**；`Q` 保持显式（它只落 `Prop`，是教学点） |
| `lib/Image.sokonanoda:42/50/65/77` | `Set.mem_image` / `Set.mem_preimage` / `Set.image_mono` / `Set.image_subset_iff`，全部 `(α β : Type)` 前导 | `{α β}`：`α` 从 `A`/`f`、`β` 从 `f : α -> β` 的**值域反解**（比头部匹配难一档，见 §4.5） |
| `lib/Equiv.sokonanoda:95/100/105` | `Set.MapsTo` / `Set.LeftInvOn` / `Set.RightInvOn`，`(α β : Type)` | 同上；**这 342 处是调用量最大的一批** |
| `units/unit05-pairs-products.sokonanoda:100` + 解答 `:20` | `def Set.prod (A B : Type) (s : Set A) (t : Set B) : Set (Prod A B)` | `{A B}`：`A` 从 `s`、`B` 从 `t`。**注意参数名是 `A B` 不是 `α β`**——`notation_prefix_args` 的名字无关，但 `×ˢ` 的两个前导参数靠"第 3、4 个操作数"解（`notation-subset.md:423-425`），隐式后**同样的解** |
| `units/unit09-equinumerosity.sokonanoda`（`Set.Equiv`） | 单元自定义 | 单元内自定，随单元改写一起定 |
| `units/unit10-cantor.sokonanoda:96`（`Set.Countable`） | `def Set.Countable (α : Type) : Prop` | `{α}`：无操作数 ⇒ 只能从期望解；2 处调用 |

---

## 2. 调用点统计（`units/` + `units/solutions/` + `lib/`，grep 聚合）

口径：**去注释后的代码**（`sed -E 's/--.*$//'`）；"写了类型实参"= 名字后面紧跟
`α|β|γ|δ|ε|Nat|(Set |(α|(Prod |Set α|Set β` 之一（正则近似，±2%）。

| 名字 | 代码里出现 | 其中写了前导类型实参 | 名字 | 代码里出现 | 其中写了前导类型实参 |
|---|---:|---:|---|---:|---:|
| `Set.mem` | 545 | **438** | `Set.pair` | 27 | 27 |
| `Set.empty` | 227 | 167 | `Set.mem_univ` | 21 | 21 |
| `Set.image` | 225 | 164 | `Set.compl` | 20 | 19 |
| `Set.univ` | 216 | 216 | `Set.prod` | 15 | 0 ⚠️ |
| `Set.singleton` | 189 | 101 | `Set.Equiv.mk` | 14 | 14 |
| `Set.subset` | 163 | 162 | `Set.notMem_empty` | 12 | 10 |
| `Set.preimage` | 149 | 121 | `Set.mem_powerset_iff` | 8 | 8 |
| `Set.MapsTo` | 134 | **134** | `Set.mem_singleton_self` | 7 | 5 |
| `Set.inter` | 131 | 101 | `Set.mem_singleton_iff` | 6 | 6 |
| `Set.RightInvOn` | 105 | 105 | `Set.mem_preimage` | 4 | 4 |
| `Set.LeftInvOn` | 103 | 103 | `Set.mem_image` | 4 | 4 |
| `Set.union` | 89 | 88 | `Set.image_mono` | 4 | 4 |
| `Exists.elim` | 76 | 40 | `Set.subset_def` | 2 | 2 |
| `Set.Equiv` | 62 | 62 | `Set.image_subset_iff` | 2 | 2 |
| `Exists.intro` | 54 | 32 | `Set.equivOfEq` | 2 | 2 |
| `Set.sdiff` | 51 | 50 | `Set.Equiv.trans/symm/singleton/refl/of_inj_surj` | 各 2 | 各 2 |
| `Set.powerset` | 47 | 46 | `Set.Countable` | 2 | 2 |
| `Set.ext` | 42 | 34 | `Set.mem_union` | 1 | 1 |
| | | | `Set.mem_inter_iff` | 1 | 1 |
| | | | `Exists.choose` / `Exists.choose_spec` | 5 / 1 | 1 / 1 |

**汇总**：点名调用 **2784 处**，其中 **2308 处（82.9%）写了前导类型实参**——
这就是"变短"的上界。含注释共 **3549 处**（注释里 765 处是 hint 与教学叙事，
不判卷但会变成"教学习者写已经不必要的东西"，见 §7）。

⚠️ `Set.prod` 一栏是 0 不是因为它短，而是因为**它是单元⑤ 画布新给的词汇**，
调用点写成 `Set.prod A B s t`（前两个参数是 `Type` 但名字是 `A B`，没被我的正则
`α|β|…` 抓到）；实际 **12/12 处调用都写了类型实参**（另 3 处在定义/签名行）。

**按文件的改写面**（代码行数 / 调用数，只列非零）：

| 文件 | 行 | 调用 | 文件 | 行 | 调用 |
|---|---:|---:|---|---:|---:|
| `lib/Demo.sokonanoda` | 13 | 18 | `units/unit02-subsets-empty` | 12 | 27 |
| `lib/Equiv.sokonanoda` | 29 | 36 | `units/unit03-union-inter-powerset` | 21 | 52 |
| `lib/Exists.sokonanoda` | 5 | 5 | `units/unit04-extensionality-identities` | 25 | 59 |
| `lib/Image.sokonanoda` | 27 | 48 | `units/unit05-pairs-products` | 13 | 27 |
| `lib/Set.sokonanoda` | 11 | 11 | `units/unit06-relations` | 5 | 9 |
| `units/notation-cheatsheet` | 24 | 43 | `units/unit08-images-preimages` | 89 | **204** |
| `units/solutions/notation-cheatsheet-solution` | 19 | 33 | `units/unit09-equinumerosity` | 26 | 36 |
| `units/solutions/unit01-solution` | 10 | 10 | `units/unit10-cantor` | 10 | 16 |
| `units/solutions/unit02-solution` | 39 | 75 | `units/unit11-universe-russell` | 24 | 45 |
| `units/solutions/unit03-solution` | 42 | 101 | `units/unit12-synthesis` | 101 | 148 |
| `units/solutions/unit04-solution` | 38 | 91 | `units/solutions/unit08-solution` | 231 | **529** |
| `units/solutions/unit05-solution` | 32 | 61 | `units/solutions/unit09-solution` | 212 | 310 |
| `units/solutions/unit06-solution` | 49 | 67 | `units/solutions/unit10-solution` | 33 | 70 |
| `units/solutions/unit07-solution` | 15 | 18 | `units/solutions/unit11-solution` | 36 | 72 |
| `units/unit01-sets-membership` | 1 | 1 | `units/solutions/unit12-solution` | 328 | **562** |

**已经 Lean 化的只有 4 个文件**：`units/unit01-sets-membership.sokonanoda`（8 处
`∈/⊆/∧/∨`）、`units/solutions/unit01-solution.sokonanoda`（13）、
`units/notation-cheatsheet.sokonanoda`（15）、`units/solutions/notation-cheatsheet-solution.sokonanoda`（28）。
其余 26 个文件的代码里 `∈`/`⊆`/`∧`/`∨` 出现 **0 次**——所以"调用点变短"与
"记法替换"这两刀**重叠在同一批文件上**，应该**合并成一次改写**（§9）。

---

## 3. 如果参数变隐式，哪些调用点会变短（真实例子）

左 = 今天必须写的（都能判绿），右 = 隐式实参落地后能写的。全部带 `文件:行号`。

| # | 文件:行 | 今天 | 之后 |
|---|---|---|---|
| 1 | `units/unit01-sets-membership.sokonanoda:29` | `exact Set.subset_def α A B` | `exact Set.subset_def A B` |
| 2 | `units/solutions/unit01-solution.sokonanoda:21` | `exact Set.mem_singleton_self α a` | `exact Set.mem_singleton_self a` |
| 3 | `units/solutions/unit01-solution.sokonanoda:31` | `Set.mem_singleton_iff α x a` | `Set.mem_singleton_iff x a` |
| 4 | `units/unit04-extensionality-identities.sokonanoda:41` | `Set.ext α A B h` | `Set.ext A B h` |
| 5 | `units/notation-cheatsheet.sokonanoda:206` | `Set.ext α A B (fun (x : α) => Iff.intro (A x) (B x) (h1 x) (h2 x))` | `Set.ext A B (fun (x : α) => Iff.intro (A x) (B x) (h1 x) (h2 x))` |
| 6 | `units/unit02-subsets-empty.sokonanoda:55` | `theorem empty_subset (α : Type) (A : Set α) : Set.subset α (Set.empty α) A :=` | `… : Set.subset (Set.empty) A :=`（`Set.empty` 的 α 由 `Set.subset` 的第 1 个参数位期望类型给） |
| 7 | `units/unit03-union-inter-powerset.sokonanoda:50` | `Iff (Set.mem (Set α) B (Set.powerset α A)) (Set.subset α B A)` | `Iff (Set.mem B (Set.powerset A)) (Set.subset B A)` |
| 8 | `units/unit08-images-preimages.sokonanoda:130` | `(h : Set.subset α A B) : Set.subset β (f '' A) (f '' B)` | `(h : Set.subset A B) : Set.subset (f '' A) (f '' B)`（注意第 2 个 `⊆` 的 α 是 **β**，靠 `f '' A : Set β` 解出） |
| 9 | `lib/Equiv.sokonanoda:113` | `And (Set.MapsTo α β f A B)` | `And (Set.MapsTo f A B)` |
| 10 | `units/solutions/unit12-solution.sokonanoda:101` | `Exists.elim α (fun (x : α) => And (Set.mem α x (Set.univ α)) (Eq.{1} β (f x) y)) Q h f` | `Exists.elim (fun (x : α) => …) Q h f` |

> 第 8 条最能说明"隐式实参不是省字，是**让类型跟着操作数走**"：今天 `Set.subset β`
> 里的 `β` 是人工判断的，写错不报"位置错"而是报内核类型不符；隐式之后它由
> `f '' A : Set β` 唯一确定。
>
> 第 6 条的写法依赖"期望类型传播到操作数位"（`notation_operand_expected` 的既有机制），
> 是**风险最高的一档**：`Set.empty` 没有操作数，α 只能从期望来。若隐式插入只做
> "从后面的显式实参解"，第 6 条就得写成 `Set.subset (Set.empty α) A`。

---

## 4. 记法声明会变成什么样 / 补参代码能不能删

### 4.1 记法声明行**一个字都不用改**

`infix:50 " ∈ " => Set.mem` 只记「符号 + 优先级 + 目标名」，**与目标的 binder 风格无关**。
10 条声明（`lib/Set.sokonanoda:164-177`）与 `binder_notation "∃" => Exists`
（`lib/Exists.sokonanoda:100`）都保持逐字不变。

### 4.2 补参代码的位置与"能不能删"

| 代码 | 位置 | 隐式实参落地后 |
|---|---|---|
| `elab_notation` 的前导参数调用与拼装 | `crates/front/src/compile/elab.rs:1035-1063` | **保留但改写**：要么继续用（无害），要么改成"只挂操作数、把隐式插入交给新例程" |
| `notation_prefix_args` | `elab.rs:1465-1502` | **可删/可收窄**：它靠 `layers.len() - operands.len()` **猜**有几个前导参数（还要 `for missing in (1..=max_missing).rev()` 逐个试，`:1488`）。有了真隐式/显式之分，"该补几个"是**确定的**，这段启发式可以整段替换 |
| `notation_telescope` | `elab.rs:1505-1514` | **保留**（仍要读目标签名做重载选择与操作数期望类型） |
| `solve_prefix_args` | `elab.rs:1518-1567` | **可替换成通用插入例程**：① 从后续显式 binder 的域反解（`:1537-1556`）、② 从期望类型解（`:1558-1563`）——这两条就是隐式实参插入本身 |
| `notation_operand_expected` | `elab.rs:1573-1605` | **可删**：它的职责（把 telescope 第 k 个参数位的期望类型传给第 k 个操作数）本来就该由通用 App 路径承担。`notation-subset.md:274-279` 自己写着「将来做一般隐式实参时应能整段替换掉」 |
| `set_literal_prefix_args` | `elab.rs:1184-1197` | **可删**（`{∅}` 的回退解）：`Set.singleton {α} (a : α)` 的 α 由通用插入从 `a` 解，`{∅}` 的 α 由期望类型解 |
| `choose_notation_target` / `notation_result_matches` | `elab.rs:1308/1403` | **保留**（重载按结果类型选，与 binder 风格无关） |

**⚠️ 一个必须知道的实现事实（实测）**：`elab_notation` 拼核项用的是
`builder.mk_const` + `builder.mk_app`（`elab.rs:1057-1078`），**没有走
`elab_expr` 的 `Expr::App` 分支**（`elab.rs:1955-1961`，那里才是隐式插入该落地的唯一钩子）。
所以「在 App 分支加隐式插入」**不会**自动让记法路径受益——记法路径要么自己调插入例程，
要么改成"产源级 App 再交给 `elab_expr`"（`notation-subset.md:280-283` 记着第一版正是
这么写的，因为"操作数被 elaborate 两次"才改成现在这样）。

### 4.3 前后对比示例（记法声明 + 目标签名）

**今天**（`lib/Set.sokonanoda:55` + `:164`）：

```sokonanoda
def mem (α : Type) (a : α) (A : Set α) : Prop := A a
infix:50 " ∈ " => Set.mem
-- `a ∈ A` ⇒ elab_notation 先 judge_type_of 读签名
--          ⇒ notation_prefix_args 解出 α := typeof(a)
--          ⇒ mk_app(mk_app(mk_app(Const Set.mem, α), a), A)
-- 点名 `Set.mem a A` ⇒ 内核拒（缺 α）——这是 N4.3 的"护城河"
```

**之后**（签名改隐式，记法行不动）：

```sokonanoda
def mem {α : Type} (a : α) (A : Set α) : Prop := A a
infix:50 " ∈ " => Set.mem
-- `a ∈ A` ⇒ 记法路径挂操作数即可（α 由隐式插入补）
-- 点名 `Set.mem a A` ⇒ **通过**（护城河消失，这是本轮的目的）
-- 点名 `Set.mem α a A` ⇒ **仍然通过**（内核不看 binder 风格；实测见 §10 探针 A）
```

### 4.4 三条会**存活**的边界（别以为隐式实参一落地就全好）

1. **`Eq.{1} (Set α) A ∅` 里的 `∅`**：`Eq` 的操作数位今天不吃期望类型
   （`notation-cheatsheet.sokonanoda:56-59` 明写这条边界）。隐式实参只有在
   **App 路径把期望类型往下传**时才能救它；如果新机制只做"从后续显式实参反解"，
   这条边界原样存活。
2. **`#check ∅`（完全没有期望类型）**：α 没有来源 ⇒ 仍然报错。今天的错误码是
   `elab-notation-argument-unsolved`（`notation.rs:222-240` 钉着它）；改后如果走
   通用插入，错误码/阶段可能变——**要么保持同码，要么同轮改测试**。
3. **`∅` 嵌在另一个记法里、又在 `by` 块里**：**今天就是坏的**（与隐式实参无关，
   §10 探针 E）。课程 unit02/03/04 的 `∅` 全在这个形状上，这是改写前必须清的雷。

### 4.5 隐式插入本身的三个难度档（给实现排期用）

| 档 | 解出方式 | 例子 | 覆盖 |
|---|---|---|---|
| ① 易 | 从**后续显式 binder 的域头部**反解 | `Set.mem {α} (a : α) …` ⇒ `α := typeof(a)` | 绝大多数（`mem/subset/singleton/pair/union/…`） |
| ② 中 | 从**后续显式 binder 的域的值域/箭头**反解 | `Set.image {α β} (f : α -> β) …` ⇒ β 从 `f` 的箭头值域 | `image/preimage/prod/MapsTo/LeftInvOn/RightInvOn`（≈**640 处**） |
| ③ 难 | 从**期望类型**反解（无操作数或全隐式签名） | `Set.empty {α}` / `Set.ext {A B}` / `Set.subset_def {A B}` | `empty/univ`（≈**380 处**）+ 全隐式引理的调用 |

**建议**：第一刀只做 ①+②，把 `empty`/`univ` 与"全隐式引理"（`ext`/`subset_def` 的
`A B`）留到第二刀。这样 §1.2 表里标 ⚠️ 的行全部推迟，风险面立刻收敛。

---

## 5. `∃` / `∀` 与 binder 记法

**判断：`∃ x, p x`（省标注）与 `∀ x, p x` 都不会因为隐式实参而工作。**

理由（读代码 + 实测）：

1. `∀` 是**原生关键字**，走 `parse_binder_prefix`；binder 没类型时今天报
   `elab-untyped-binder`（`course-lean-style.md:393` 的 C4 第 3 条实测）。
   隐式实参补的是**常量应用的参数**，`∀` 根本不是常量。
2. `∃` 走 `binder_notation`，binder 类型的来源只有**两条**：
   `elab.rs:1120-1179` 的 `binder_notation_operand`——① binder 自己写了标注
   （`:1138-1140` 直接放行）、② 两段式由 guard 反解（`:1141-1157`）。
   一段式没标注 ⇒ **专用诊断** `elab-binder-notation-unsolved`（`:1158-1170`，
   注释逐字写着"记法不引入元变量与一般合一，裸 `∃ x, p` 的 x 类型没有来源"）。
   隐式实参**不改变这条路径**：`Exists {A} (p : A -> Prop)` 的 `A` 能从 `p` 解出，
   但 `∃ x, p x` 里 `p x` 的 `x` **连类型都没有**，`fun (x) => p x` 在
   `elab_expr` 的 Lambda 分支（`elab.rs:1989-2005`）就报 `elab-untyped-binder`——
   **轮不到 `Exists` 的参数**。
3. 要 `∃ x, p x` 需要：从 `p : α -> Prop`（上下文里已知）**合一**出 `x : α`——
   即元变量 + 一般合一。这正是 `course-lean-style.md` N-1/N-8（`:660`/`:667`）
   与 `notation-subset.md` §13.3 拒绝的那件事。

**隐式实参给 `∃` 带来的真实收益**（不是省标注，是省**点名**参数）：

| 今天 | 之后 |
|---|---|
| `Exists α (fun (x : α) => p x)` | `Exists (fun (x : α) => p x)` |
| `Exists.intro α p w hw` | `Exists.intro w hw` |
| `Exists.elim α p Q h f` | `Exists.elim Q h f` |

（`∃ (x : α), p x` 与 `∃ x ∈ s, p` 的**写法不变**，仍然要标注/guard。）

---

## 6. `=` 的处境

### 6.1 今天的三块前置

| # | 前置 | 位置 | 隐式实参能解决吗 |
|---|---|---|---|
| 1 | **词法层没有裸 `=`**：`token.rs:296-308` 的 `'='` 分支只认 `=>`，`a = b` 报 `expected =>, found =`（未声明任何记法也一样） | `crates/front/src/token.rs` | ❌ 完全无关 |
| 2 | **宇宙层**：`Eq.{u} α a b` 的 `u` 必须显式；省略时前端默认 `u = 0`，`Eq α a b`（`α : Type`）被内核拒「期望 `Sort(0)`，实际 `Sort(1)`」 | `elab.rs:1004` 给常量传**空宇宙层** | ⚠️ 部分 |
| 3 | **课程满屏 `Eq.{1}`**：代码里 **679 处**、**32 个文件** | — | ⚠️ 部分 |

### 6.2 判断：**不会显著变简单，L2.4c 的 spike 结论不变**

- `α` 确实变成可解：`Eq {α : Sort u} (a b : α)`，`α := typeof(a)`（= `Set α₀`）。
- **但 `u` 仍要单独推断**：`α : Sort u` 是 **Sort 位的隐参数**，它的"值"是一个
  **层级**，不是项。要从 `typeof(α) = Set α₀ : Sort ?u` 解出 `u = 1`，必须
  "问内核要 sort"——这正是前置 2 里 `elab.rs:1004` 那件事，隐式实参机制
  （从实参类型头部反解）**够不着**。
- 所以 `=` 仍然是"词法 token + 宇宙层推断"两件事，隐式实参只拿掉了第三件的一半。

### 6.3 实测否掉了"单宇宙 `Eq'` 绕道"（这条要记）

想法：既然课程只在 `Type 0` 上做等式，写一条单宇宙
`def Eq' (α : Type) (a b : α) : Prop := Eq.{1} α a b` 就能绕开宇宙层，隐式实参
再把 `α` 隐掉 ⇒ `A = B`。

**实测不成立**（探针 D）：

- `def Eq' … := Eq.{1} α a b` + `theorem t : Eq' α a a := by rfl`
  ⇒ **exit 1**，`` `rfl` 需要一个 `Eq α x y` 形状的目标 ``。
- 换成 `abbrev Eq' …` 同样失败（`abbrev` 也不被 `rfl` 穿透）。
- 对照：`Set.subset α A A` 上的 `intro x` **能**穿透（源级 delta 展开 `DefTable`
  生效）——所以这不是"delta 没做"，是 **`rfl` 的目标形状判定不走 delta**
  （`by.rs:243-282` 的 `Eq α x y` 形状检查）。

⇒ 若真要走 `=`，`rfl` 必须同轮扩展（"目标头是单宇宙 Eq-别名时穿透"），
否则全课程的 `rfl` 会失效。**这条与隐式实参无关，但决定了 `=` 能不能做**。

---

## 7. 课程文档同步：哪些句子会变成假话

> 纪律来源：`courses/set-theory/AGENTS.md:29-35` 的"写作纪律"要求"每句教学主张要么
> 给出处、要么标设计判断"；`course-lean-style.md:360`（C1.8）已有一份"会变假话"清单，
> 本节是**隐式实参这一刀新增的**。

### 7.1 `courses/set-theory/README.md`

| 行 | 原句（摘） | 为什么变假 | 改成 |
|---|---|---|---|
| `:85` | 「`units/notation-cheatsheet.sokonanoda` \| **记法对照页**（大纲 §4 的"第二遍"）：同一个命题的**点名形式 ↔ 数学记法**（`∈`/`⊆`/`∅`/`∪`）并排演示 + 3 道记法练习」 | 全课程 Lean 化后不再是"对照页"（C1.5 已定：改成**记法速查表**） | 记法速查表 |
| `:86` | 「`lib/Set.sokonanoda` **末尾的记法块** \| 卷 I 的五个**集合论专用**数学符号…」 | **今天 lib/Set 有 11 条记法**（`∈ ⊆ ∪ ∩ \ ∅` 已在 W1 搬进来，`:164-171`），"五个"是第二刀口径 | 十一条（六个 core 级 + 五个 Mathlib 级） |
| `:126` | 「**门禁实测（2026-09-19，0.60.0 二进制，清单 v2 之后）：36 个目标 · 329 checked · 99 open · 0 判负**」 | 单元① 改写 + 隐式实参后 checked/open 会变（单元① 解答今天还 exit 1） | 重算后回填（`check.py --json`） |
| `:147` | 「\| 1 \| I.1 \| 集合与隶属 \| 6 \| 6 \|」 | 单元① 解答今天 **5 checked + exit 1**（§8.3），不是 6 | 修好后回填 |
| `:167` | 「`a ∈ A` 与 `Set.mem α a A` 是**同一个项**，走同一个内核」 | 仍真，但"必须写 `α`"这层意思与 §4 的护城河一起失效 | 补一句"点名可省前导类型参数" |
| `:170-172` | 「**第一刀（0.59.0）**：…`∈`/`⊆`/`∪`/`∅` 这四个 core 级符号课程库**不声明**——记法对照页自己写一份（那是"第二遍"教学的一部分）」 | **W1 之后已经假了**：四个符号今天在 `lib/Set.sokonanoda:164/165/166/171` 声明 | 删/改写 |
| `:183-184` | 「**同一个符号全课程只能声明一次**（重复声明是 parse 错误）——库声明过的，单元不能再声明一遍」 | 仍真（保留） | — |
| `:185-186` | 「**一元记法在实参位要加括号**：`f (𝒫 A)` / `f (Aᶜ)`；`Eq.{1} (Set α) (Aᶜ) (…)` 里的那对括号是必须的」 | 仍真（`notation-subset.md` §13.4 明确不做免括号） | — |
| `:209-210` | 「名字与签名**逐字不变** ⇒ `units/` 里 186 处点名调用（`Exists.intro A p w hw` / `Exists.elim A p Q h f`）零改动」 | `Exists` 的 `A p` 隐式后这 186 处**正是要改的** | 改写 |
| `:33-35` | 「计数（checked / open / diagnostics）与**配额差额**只进报告与台账，从不参与判红」 | 仍真（这是 G1–G6 的判据说明，与隐式实参无关） | — |

### 7.2 `courses/set-theory/AGENTS.md`

**本文件没有一句关于记法/实参的话**（40 行里只有写作循环、判卷纪律三条、写作纪律四条、
零 cargo）。⇒ **没有会变假话的句子**，但**该新增一条纪律**（本刀的产物）：

> 建议在 `:29-35` 的"写作纪律"里加一条：**「点名形式一律省前导类型参数」**——
> `Set.mem a A` / `Set.ext A B h` / `Exists.intro w hw`，把 `α` 留给隐式插入；
> 只有"期望类型解不出"的位置（`∅` 在 `Eq` 操作数位等）才写 `Set.empty α`。

**根仓 `AGENTS.md:119-122`（硬规则 3 的"记法是例外面"）** 有一句会变假：

> `:121` 「判卷一致；边界（文件内作用域、**补前导类型参数**、第二刀未做项）见
> `docs/design/notation-subset.md`」

——"补前导类型参数"从**边界**降级成**临时实现**。同轮要改这句 + `notation-subset.md`
的 N4.2/N4.3（§7.5）。

### 7.3 `courses/set-theory/units/notation-cheatsheet.sokonanoda`

| 行 | 原句（摘） | 变假原因 |
|---|---|---|
| `:6-10` | 表格「点名形式（pointful，**你一直在写的**） … `Set.mem α a A` ↔ `a ∈ A`」 | 全课程 Lean 化后不再是"你一直在写的" |
| `:12-15` | 「课程故意**先教点名形式**…等你写顺了，再看见数学书上那套符号」 | 与 D4（`:51`「大纲 §4 改为记法为主」）正面冲突——C1.5 已定重定位 |
| `:19-23` | 「① **记法是糖**…elaborator 把它源到源降级成既有的应用形状 `Set.mem α a A`（`α` 由操作数解出后补上——这是记法路径**独有**的一步）」 | "记法路径独有"变假（隐式插入是通用机制） |
| `:40-43` | 「本文件自己声明的四条（`∈` / `⊆` / `∪` / `∅`）是**另一回事**：它们是 Lean core 级符号，课程库**没有**声明（第一刀把它们留给了各页自己写），所以本页仍自带一份」 | **已经假了**：本文件 `:86-88` 自己写着"已搬进 lib/Set"，但 `:40-43` 没删（同一文件里自相矛盾） |
| `:45-50` | 「**一条护城河（别踩）**…`Set.mem a A` ← 仍被内核拒绝（kernel-rejected）：缺 `α`；`a ∈ A` ← 通过…这不是"记法更聪明"，是设计上的边界（N4.3）」 | **整段作废**（护城河就是本刀要填的） |
| `:52-59` | 「**已知边界：`∅` 想要"期望类型"才补得出 `α`** …`Eq.{1} (Set α) A ∅` ← **拒绝**…这条边界是"不做一般隐式实参推断"的直接后果」 | "不做隐式实参"变假；边界本身**可能存活**（§4.4）——要重写成"期望类型传播的边界" |
| `:63-71` | 与 Lean core 的对照表（`notation:50 a:50 " ∈ " b:50 => Membership.mem b a` vs `infix:50 " ∈ " => Set.mem`） | 仍真（操作数顺序差异不变） |
| `:77` | 「其它单元的画布**继续用点名形式**——记法是本页单独教的第二遍，不是全卷改写」 | 变假（D4） |
| `:119-127` | 演示里 `Set.mem (Set α) A (𝒫 A)` / `Set.mem α a (Aᶜ)` / `Set.subset α A A` | 调用点变短（内容真、写法旧） |
| `:165-179` | 演示 4/5：`Set.subset α (Set.empty α) A` / `Set.notMem_empty α a` | 同上 |
| `:204-210` | 演示 8：`Set.ext α A B (fun …)` | 同上 |

### 7.4 `docs/design/course-stdlib.md`

| 行 | 原句（摘） | 变假原因 |
|---|---|---|
| `:309`（§4 表） | 「\| **G-04** 无 notation \| L2 只能用点名；记法作为第二遍的 X 类练习 \|」 | G-04 已落地（两刀），这一行是历史 |
| `:184-185` | 「**命名纪律**：点名形式用 Mathlib 的名字…G-04（`notation`）落地后只加记法层、不改名字——于是"同一命题两种写法"天然成为 X 类练习」 | 仍真（名字确实没改），但"两种写法"的教学定位变了 |
| §1 三层分界表（`:35-39`） | L2 判据「Mathlib 里是 `rfl` 或一行；**没有数学内容**」 | **判据本身不变**；变的是**L2 的写法**（隐式参数让 L2 签名更贴 Mathlib）⇒ 建议在 §3.2「本语言逼出来的三个变形」里**加一条 D**：今天的 L2 签名全是 `(α : Type)` 显式，是语言限制逼出来的第四个变形，隐式实参落地后消失 |

### 7.5 `docs/design/set-theory-syllabus.md` §4（`:206`–`:227`，注意文件里有**两个** §4）

| 行 | 原句（摘） | 变假原因 |
|---|---|---|
| `:206-226` 整张表「顺序 / 记法 / **今天怎么写** / 出现在」 | 「1 \| `x ∈ A` \| `Set.mem α x A`（= `A x`） \| 单元 1」等 10 行 | "今天怎么写"一列全部变旧（可省 `α`） |
| `:226-227` | 「教学上"先看见糖、再见记法"反而有利：G-04 落地后，同一单元补一页"同一命题两种写法"，正好是 X 类练习的素材」 | 与 D4 冲突（C1.7 已列） |
| `:247`（§5 表） | 「\| 想写 `∈`/`⊆` \| **G-04** \| 点名形式 + 画布头留 TODO \|」 | G-04 已落地 |

### 7.6 语言侧设计文档（**会被本刀推翻的"不做"**）

| 文件:行 | 原句（摘） | 处置 |
|---|---|---|
| `docs/design/course-lean-style.md:49`（D2 表） | 「**不做**隐式实参 / `h.1` 投影 / print-back（除非 §7 的 spike 判定低风险）」 | 用户新要求 ⇒ 改成"做，分两刀" |
| `:285`（L2.9） | 「隐式实参（`Or.inr ha`、`Set.ext h`）——**D2 明确不做**；若 §7 spike 判定低风险再单独立项 \| 不做（本轮）」 | 同上 |
| `:660`（N-1） | 「隐式实参 / 元变量 / 一般合一 \| D2 明确不做；会动 elaborate 核心（与"记法零语义"冲突）…」 | 拆成两条：**隐式实参做**（不动内核、不改记法语义）；**元变量/一般合一仍不做** |
| `:667`（N-8） | 「`∃ x, p x`（省略 binder 类型） \| 需要一般合一（N-1 的直接后果）；课程一律写 `∃ x : α, p x`」 | **不变**（§5：隐式实参救不了它） |
| `:240-243`（§2 目标形态的注意） | 「目标形态里的 `Or.inr ha`（**隐式实参**）属于 D2 明确不做 ⇒ 落地时的实际形态是 `exact Or.inr (B x) (A x) ha`」 | 变假：可以写 `Or.inr ha`（若 `Or` 的构造子也隐式——**注意 `Or` 在 prelude 里**，见下） |
| `docs/design/notation-subset.md:25-29`（§1 第 3 条） | 「**`∈` 藏着一个类型参数**…而本语言**不插入隐式实参**…⇒ 记法**不是纯 parser 糖**」 | 本刀的动机段，要重写为"隐式实参落地后记法回到纯糖" |
| `:97-107`（N4.2/N4.3） | 「补全只允许发生在记号展开路径」「**回归护栏（兼容性的护城河）**：`Set.mem a A`…改后必须仍被拒绝」 | **N4.3 直接作废**（本刀就是要它通过）；N4.2 收窄成"操作数期望类型" |
| `:552`（§13.3） | 「一般隐式实参推断 / 元变量 / 一般合一 …第三刀没动…直接后果：① 一段式 `∃ x, p` 的 x 类型**必须**写出来…」 | ①**不变**（§5）；"一般隐式实参推断"这一项要移出"销不掉"，单独立项 |
| `:267-268`（§8 守护表行） | 「护城河（点名省 `α` 仍被拒）」 | 删（测试同步，§8.1 T1） |
| `docs/design/course-lean-style.md:410`（C5/T1） | 「`the_shipped_course_still_uses_the_pointful_spelling` … **必然红**…要**改判据**（改成「课程用记法」），不是删测试」 | 仍有效，本刀再加一条"点名省 `α` 要通过" |

**还有一处要单独决策**：`Or` / `And` / `Iff` / `Eq` 在 **prelude**（`crates/front/src/compile/prelude.rs`
的 `PRELUDE_L1_SRC`），不在课程仓。`course-lean-style.md:240` 期望的 `Or.inr ha`
需要 **prelude 的构造子也隐式**。那属于语言侧改动（`PRELUDE_L1_SRC` 是**冻结内核之外的
前端资源**，可改，但会动两个课程的 GOLDEN 事件流——`crates/cli/tests/course.rs:86`、
`course_status.rs:68`、`cli.rs:2097`）。**建议本刀只动课程 `lib/`，prelude 另开一刀**，
否则"卷 I 改写"会连带打红入门课的 13 个测试（`course-lean-style.md:374` 的 C2.7）。

---

## 8. 测试与门禁

### 8.1 `crates/cli/tests/notation.rs`（1136 行，24 条测试）逐条

| 测试（行） | 钉住什么 | 三个触发源下的结果 |
|---|---|---|
| `the_pointful_spelling_keeps_working_and_the_moat_holds`（`#[test]` `:162`，`fn` `:163`） | `:166` 写 `def p … := Set.mem a A`（省 `α`），`:169` 断言 **exit 1**，`:175` 断言诊断是 `kernel-rejected` / stage `kernel` | **参数变隐式 ⇒ 硬红**。改法：判据反过来——省 `α` 必须 **exit 0**，并断言 `decl.checked` 里有 `p`；测试名改 `the_pointful_spelling_can_drop_the_leading_type_argument`。**同时补一条反向钉**：`Set.mem α a A`（写全）仍然 exit 0（保住兼容） |
| `notation_nullary_without_an_expected_type_reports_the_unsolved_code`（`:222`/`:223`） | `:225` `#check ∅` ⇒ `elab-notation-argument-unsolved` + stage `elab` | **参数变隐式 ⇒ 可能红**（若插入改在通用路径报错，码/阶段会变）。改法：**先决定错误码是否保持**；保持就只改注释，变了就同轮改这条 + `docs/protocol.md` 错误码表 |
| `the_shipped_course_still_uses_the_pointful_spelling`（`:556`/`:557`） | `:563-570` unit02 不许出现 `infix`/`notation` 行；`:572` 必须含 `Set.subset α A C` | **单元改写 ⇒ 硬红**（C5/T1 已列）。改法：改成"课程用记法"判据 + 新增"点名省 `α` 也判绿" |
| `a_real_course_unit_grades_identically_in_notation`（`:345`/`:346`） | `notation_variant`（`:287-313`）对 unit02 的**两行签名做逐字替换**，`assert_eq!(out.matches(pointful).count(), 1)`（`:305-309`）；`:374` 断言 `open >= 8` | **单元改写 ⇒ 硬红**（`Set.subset α A B` 那两行不再逐字存在）。改法：把替换靶子改成**改写后的实际签名**（或改成"按声明名替换"的更稳夹具）；`open >= 8` 重取 |
| `the_shipped_course_library_declares_the_five_symbols`（`:516`/`:517`） | `:520-534` `lib/Set` 必须逐字含 `prefix:100 " 𝒫 " => Set.powerset` 等 5 行 | **记法搬家 ⇒ 只在再搬时红**。W1 之后这 5 行仍在 `lib/Set.sokonanoda:173-177` ⇒ **本刀不红**。若把 `×ˢ`/`''` 的目标也搬进 lib（C4 第 7 条的推荐 (a)），要重钉 |
| `a_library_notation_works_in_the_entry_through_import`（`:440`/`:441`） | 拷贝**真 `lib/Set`**（`stage_course_unit` `:326`），入口写 `def p … := 𝒫 A`、`Set.compl α A`（`:450-453`）；反向对照两条（`:468-513`） | **参数变隐式 ⇒ 预期不红**：`Set.compl α A`（显式实参落在隐式位）**内核照样接受**（§10 探针 A/B 实测）。**但要在改 `lib/Set` 后立刻单跑这条**——它是最早暴露"显式实参不再被接受"这类策略变更的哨兵 |
| `the_shipped_course_uses_the_library_notation_in_a_demo`（`:537`/`:538`） | unit03 含 `𝒫 `、unit08 含 `'' ` | **单元改写 ⇒ 若演示被删就红**（保留演示则不红） |
| `the_shipped_course_demos_the_binder_notation`（`:757`/`:758`） | unit08 **含子串** `binder_notation`（`:766`）与 `∃ (`（`:770`） | **单元改写 ⇒ 硬红**：`binder_notation` 今天只出现在 unit08 的**注释**里（`:35`/`:134`，真声明已在 `lib/Exists.sokonanoda:100`）；unit08 一旦清注释就红。改法：判据改成"unit08 用了 `∃ (` 记法"，`binder_notation` 的守护挪到 `lib/Exists`（照 T3 的样子） |
| `set_literals_grade_like_the_pointful_singleton_and_pair`（`:617`/`:618`） | 自建 `THIRD_LIB`（`:583-591`，`Set.singleton (α : Type)` 显式） | **不红**（自包含夹具，与课程库无关） |
| `binder_notation_grades_like_the_pointful_exists`（`:633`/`:634`） | 同上自建 `inductive Exists (A : Type)` | **不红**（同上）；若把 `THIRD_LIB` 也改成隐式做对照，才需要动 |
| `two_stage_binders_grade_like_the_pointful_guard`（`:652`/`:653`） | 同上 | **不红** |
| `a_scoped_notation_grades_only_after_open_scoped`（`:674`/`:675`） | 同上 | **不红** |
| `an_overload_grades_by_expected_type_and_reports_ambiguity`（`:714`/`:715`） | 同上 | **不红**（重载只看结果类型，与 binder 风格无关） |
| `notation_and_pointful_canvases_grade_identically`（`:107`/`:108`） | 自建 `LIB`（`:76-81`，显式 `(α : Type)`） | **不红**；**建议同轮加一条隐式版夹具**（`LIB` 换 `{α : Type}`），把"两种签名都判绿"钉住 |
| `notation_emits_no_new_event_kinds`（`:135`/`:136`） | 记法命令零事件 | **不红**（N6 不变） |
| `undeclared_symbol_is_a_parse_diagnostic_with_a_teaching_hint`（`:179`/`:180`） | 未声明符号的 hint 含"点名写法" | **不红**（hint 文案可能要改，但断言只查子串） |
| `notation_command_alone_is_a_clean_grade_with_checked_declarations`（`:202`/`:203`） | G-04 复现件形状（`def mem (α : Type)`） | **不红** |
| `stdin_and_a_file_agree_on_a_notation_canvas`（`:242`/`:243`） | 同一条流水线 | **不红** |
| `the_five_second_cut_symbols_grade_clean_in_both_spellings`（`:399`/`:400`） | 自建 `SECOND_CUT_LIB`（`:384-397`，显式） | **不红** |
| `intro_accepts_several_names_at_once`（`:801`/`:802`） | `intro a b c` | **不红** |
| `the_new_sugar_tactics_grade_with_notation_goals`（`:858`/`:859`） | `constructor/left/right/use/exfalso` | **不红** |
| `cases_splits_hypotheses_with_and_without_arms`（`:942`/`:943`） | `cases` | **不红** |
| `core_logic_notation_is_built_in`（`:1036`/`:1037`） | `∧ ∨ ↔ ¬` 内建 | **不红** |
| `notation_works_inside_by_blocks`（`:1084`/`:1085`） | X1/X2/X11 + `rfl` 回归（`axiom Set.compl (α : Type)`） | **不红** |

**结论**：**硬红 4 条**（`:163`、`:557`、`:346`、`:758`）+ **可能红 1 条**（`:223`）+ **哨兵 1 条**（`:441`）。

### 8.2 `courses/set-theory/tools/check.py` 的 G1–G6

| 判据 | 定义（`check.py:9-19`） | 本刀会不会红 |
|---|---|---|
| **G1** | 每个目标 `grade` 退出码 0 | **判据不红**（与规模无关）；但**内容红**——任一单元在改写中途 parse/elab 失败就整单元红（`course-lean-style.md:200` 的"解析错误全文件连坐"）。⇒ 改写**逐文件、逐声明**推进 |
| **G2** | 目标存在（`course.json` / `lib/` / 每个画布的解答） | **不红**（不动文件结构；不新增 lib 模块 ⇒ targets 保持 36） |
| **G3** | 解答 `exercise.open == 0` 且 `decl.checked > 0` | **不红**（只要解答填完）；⚠️ 别把解答写成 `example`（`example` 不计 `checked`，`course-lean-style.md:427-431`） |
| **G4** | 解答覆盖画布每个**具名** `exercise.open.name` | **不红**（除非改写顺手改名/删题）；⚠️ 具名练习写成 `example` 会让按名覆盖**静默失效** |
| **G5** | `lib` + `Demo`：`exercise.open == 0` | **不红**（改签名不产生 `sorry`） |
| **G6** | 清单自洽（volume/chapter id、unit 归属、prereqs） | **不红**（不动 `course.json`） |

**一句话**：G1–G6 不锁计数，所以"参数变隐式"本身**不会**让门禁判红；
红只会来自**改写质量**（某个单元没改完 / 某个 `sorry` 没填 / 名字被改）。

**另有一条本刀特有的风险**：`lib/Set` 是**所有 24 个 unit/solution 的 import 依赖**。
改 `lib/Set` 的签名 = 一次性改动**全课程 30 个文件的类型检查**。⇒
**`lib/Set` 的签名必须最后改**（先改调用点，或同一次 commit 原子改完），
否则中间态下 24 个文件一起红（§9 的分批顺序就是为此）。

### 8.3 一个**必须先修的既有 bug**（否则分不清因果）

`units/solutions/unit01-solution.sokonanoda` 今天 **exit 1**。我做了前缀二分
（§10 探针 F），结论比台账上记的更精确：

- 截断到第 5 条声明（`singleton_subset_iff`）时 **exit 0、5 条全 checked**；
- 只有**最后一条** `singleton_eq_singleton_iff`（`:40-61`）失败：
  `` `exact` 类型不匹配：期望 `Sort(0)`，实际是 `(Set.[] $2)` ``；
- 全文件判卷时报出的 4 条诊断（行 20/29/36/42）**是级联噪声**
  （`course-lean-style.md:286` 的 L2.10：先前失败的声明让后续每条都报误导性诊断），
  不是真的 4 处坏。
- 把那条声明的**证明体单独搬出来**（`Eq.subst.{1} (Set α) (fun (X : Set α) => a ∈ X)
  {a} {b} h …`）**能判绿**——所以是**文件内的交互**，不是某个子表达式独立坏。

⇒ **建议**：隐式实参动 `lib/Set` 之前，先把这一条修好（另一位 agent 正在做）。
否则"改签名后 unit01 解答红了"无法归因。

### 8.4 另一条**与隐式实参无关、但会挡住改写**的实测 bug

`∅` 嵌在另一个记法里、又出现在 **`by` 块**时，**今天就是坏的**（探针 E）：

```sokonanoda
notation "∅" => Set.empty
infix:50 " ⊆ " => Set.subset
theorem t1 (α : Type) (A : Set α) : ∅ ⊆ A := by      -- ← exit 1
  intro x
  intro hx
  exact False.elim (A x) hx
```

诊断：`` `exact` 判定失败：记法 `∅` 展开成 `Set.empty` 时补不出前面的类型参数 ``
（`elab-notation-argument-unsolved`）。对照：

| 写法 | 结果 |
|---|---|
| `∅ ⊆ A`，**term 模式**（`:= fun …`） | ✅ checked |
| `∅ ⊆ A`，**`by` 块** | ❌ `elab-notation-argument-unsolved` |
| `Set.subset α (Set.empty α) A`，`by` 块 | ✅ checked |
| `(Set.empty α) ⊆ A`，`by` 块 | ✅ checked |
| `(h : ∅ ⊆ A) : ∅ ⊆ A := by exact h` | ✅ checked |

⇒ 触发条件是"**零元记法嵌在记法里 + `by` 块**"（`by` 的目标回读丢掉了 `∅` 的期望类型）。
课程 unit02/03/04/08/12 的 `∅`（代码里 227 处 `Set.empty`）几乎全在这个形状上。
**隐式实参有希望顺手修好它**（α 由通用插入从期望类型解），但**也可能不修**
（若 `by` 回读路径压根不传期望类型，通用插入同样没有来源）——**建议把它列为本刀的
第一个验收用例**。

---

## 9. 改写工作量与分批建议

### 9.1 总量

| 项 | 量 |
|---|---|
| 涉及文件 | **30** 个（5 lib + 12 units + 12 solutions + 1 cheatsheet 对） |
| 涉及代码行 | **1520** 行含点名调用 |
| 点名调用 | **2784** 处（其中 **2308** 处写了前导类型实参 ⇒ 会变短） |
| 含注释的提及 | **3549** 处（765 处在注释/hint 里，**不判卷但要同步叙事**） |
| `Eq.{1}` 代码出现 | **679** 处 / 32 文件（**本刀不动**，除非 `=` 落地） |
| 需要改签名的声明 | `lib/Set` 22 条（除 `def Set`）+ `lib/Exists` 3 条 + `lib/Image` 4 条 + `lib/Equiv` 3 条 + 单元⑤ `Set.prod` + 单元⑨/⑩ 的 `Set.Equiv`/`Set.Countable` ≈ **35 条** |

### 9.2 分批建议（先改什么、什么留最后）

> 排序原则：**依赖方向**（lib 在最上游，改一次全课程重判）+
> **风险方向**（期望类型解 > 箭头值域解 > 头部解）+ **测试接触点**
> （`notation.rs:557` 钉 unit02、`:758` 钉 unit08、`query.rs:559` 钉 unit05）。

| 批 | 内容 | 为什么这个顺序 | 验收 |
|---|---|---|---|
| **B0（前置）** | 修 unit01 解答最后一条（§8.3）；修 `∅` in `by`（§8.4）；定错误码是否保持 | 否则后面每一步的红都归因不清 | `grade unit01-solution` exit 0 |
| **B1（最上游）** | `lib/Set.sokonanoda` 22 条签名改隐式（**`def Set` 不动**）+ `lib/Exists.sokonanoda` 3 条（`Exists`/`intro`/`elim`） | 依赖最上游；**一次改完**（中间态 = 24 文件一起红） | `grade lib/Demo.sokonanoda` exit 0 + `notation.rs:441` 单跑 |
| **B2（最高频调用点）** | `lib/Image`（4 条签名 + 48 调用）、`lib/Equiv`（3 条 + 36）、`lib/Demo`（18） | 仍在 lib 内、`G5` 单独判；**箭头值域解**（②档）在这里第一次真用 | `check.py --only "lib Image" --only "lib Equiv" --only "lib Demo"` |
| **B3（试点单元，最小的）** | `units/unit01-*` + `solutions/unit01-*` + `notation-cheatsheet{,-solution}`（**已经 Lean 化的 4 个文件**） | 它们已经在记法态，只需删前导实参（1+10+24+19 行）；`unit01` **不在** `notation.rs` 的硬断言里 | `grade` 两个文件 exit 0；`check.py --only "单元 1" --only "解答 unit01"` |
| **B4（避开测试接触点）** | 单元 3、4、6、7、9、10、11 + 各自解答（`unit03` 只动调用点、**保留** `𝒫 ` 演示；`unit05` 会打红 `query.rs:559`，**放 B6**） | 先把不接触硬断言的单元清掉 | 逐目标 `check.py --only` |
| **B5（测试接触点，原子同轮）** | `units/unit02-*`（`notation.rs:557`/`:346`）、`units/unit08-*`（`notation.rs:758`）、`units/unit05-*`（`query.rs:559`）+ 同轮改这三条测试 | **必须原子**（改文件不改测试 = 必红） | `cargo test -p sokonanoda-cli --test notation --test query` |
| **B6（收尾）** | 注释/hint 里的 765 处点名写法、`README.md`、`notation-cheatsheet` 的 12 处叙事、`AGENTS.md` 新增纪律、`course-stdlib.md` §3.2-D、`syllabus` §4 表 | 不判卷，但会变成"教学习者写多余的东西" | `check.py` exit 0 + `--selftest` |
| **B7（可选，另一刀）** | `lib/Set` 的 `ext`/`subset_def` 的 `A B` 全隐式（③档期望类型解）、`Set.empty/univ` 的期望类型解、prelude 的 `Or.inr ha` | 风险最高、收益最小；且 prelude 会打红入门课 13 个测试 | 单独立项 |

### 9.3 一句话的排期结论

**本刀的真实工作量不在"改签名"（35 条），而在"改调用点"（2308 处 / 1520 行 /
30 文件）**——而且它与"记法替换"这一刀**落在同一批文件、同一批行上**。
⇒ **强烈建议合并成一次改写**（一个文件只进一次手术台），
按 B1 → B6 的顺序推进；否则同一个文件要被改两遍，且中间态两次踩"全文件连坐"。

---

## 10. 实测附录（探针与结论）

所有探针都在 `/tmp`（仓库零改动）；命令一律 `node scripts/soko grade <绝对路径> --json`。

| 探针 | 内容 | 结果 |
|---|---|---|
| **A** | `def Set.mem {α : Type} (a : α) (A : Set α)` + `infix:50 " ∈ " => Set.mem` + `def p … := a ∈ A` | **exit 0 / 3 checked**。⇒ ① 隐式 binder 今天**能声明**（只缺应用位插入）；② **记法路径的补参 hack 与隐式 binder 兼容**（`peel_pi` 照样剥、内核不看风格） |
| **A2** | `#check Set.mem`（隐式签名） | pp 输出 **`forall {α : Type 0}, α -> Set α -> Prop`**（带花括号）⇒ `notation_telescope` 的 `parse_expr_text` 要能吃 `forall {α : Type 0}, …`；另测 `def g2 : forall {α : Type 0}, α -> α` **checked** ⇒ 能吃 |
| **B** | `def Set.empty {α : Type}` / `def Set.subset {α : Type}` + `notation "∅"` + `infix " ⊆ "` + `def e (α : Type) : Set α := ∅` | `e` **checked**（零元记法的 α 由**期望类型**解，隐式签名下同样成立） |
| **C** | `def Eq' (α : Type) (a b : α) : Prop := Eq.{1} α a b` + `theorem t : Eq' α a a := by rfl` | **exit 1**：`` `rfl` 需要一个 `Eq α x y` 形状的目标 ``。换 `abbrev` 同样失败。⇒ `rfl` **不穿透 delta**（对照：`Set.subset α A A` 上的 `intro x` **能**穿透） |
| **D** | `def Set.subset …` + `infix " ⊆ "` + `theorem t : A ⊆ B -> A ⊆ B := by intro h; intro x; intro hx; exact h x hx`（两种写法：根命名空间 / `namespace Set` 包裹） | **两种都 exit 0**。⇒ `intro` 在 `⊆` 记法目标上**是好的**；`def Set` + `namespace Set` 的极简形状**也不炸**（说明 §8.3 的 bug 需要更复杂的交互） |
| **E** | `∅ ⊆ A` 的四种写法（term / `by` / `Set.subset α (Set.empty α) A` in `by` / `(Set.empty α) ⊆ A` in `by`） | **只有"`∅` 记法 + `by` 块"失败**（`elab-notation-argument-unsolved`）；`exact h`（`h : ∅ ⊆ A`）反而通过。⇒ §8.4 |
| **F** | 把 `unit01-solution.sokonanoda` 按声明边界截断后逐段判卷（复制课程树到 `/tmp`） | 截到第 5 条 ⇒ exit 0 / 5 checked；全文件 ⇒ exit 1 / 2 checked + 4 条级联诊断。⇒ §8.3 |

**没做、也不该由本调研做的**：不改任何仓库文件；不跑全量 `check.py`（慢）；
不用官方 Lean 工具链。

---

## 11. 需要拍板的四件事

| # | 问题 | 选项 | 建议 |
|---|---|---|---|
| **P1** | 隐式插入做到哪一档？ | ①只做"从后续显式实参的域头部反解" ②①+箭头值域反解 ③②+期望类型解 | **②**：覆盖 `mem/subset/union/image/MapsTo`（≈2300 处中的 1900+），把 `empty/univ/ext/subset_def` 的期望类型解推迟（③的风险与 `=` 的宇宙层同族） |
| **P2** | 显式实参写在隐式位上（`Set.mem α a A`）还收不收？ | (a) 收（内核本来就收，向后兼容最好） (b) 不收（"点名必须省 α"） | **(a)**：否则全课程 2308 处**同一 commit 必须全改完**，中途一个文件都不能漏；收的话可以分批 |
| **P3** | 记法路径的补参 hack 删不删？ | (a) 保留（它无害，且保证记法不依赖新机制） (b) 删（记法改走 `elab_expr`） | **(a) 先保留**，等新机制在 24 个单元上跑绿一整轮再删；删的时候必须同轮验证 §4.2 那张表里的 6 个函数 |
| **P4** | prelude 的 `Or`/`And` 构造子要不要也隐式？ | (a) 本刀只动课程 `lib/` (b) 连 prelude 一起 | **(a)**：prelude 改动会打红入门课 13 个测试（`course-lean-style.md:374`），不属于"卷 I 改写"这一刀 |

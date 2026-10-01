# 设计：卷 I《集合论》大纲（**S-A 重建**，2026-10-01）

> **触发**：`docs/design/metavar-engine.md` §7 的 **S-A**（「重建卷 I 大纲」）—— 旧大纲
> `docs/design/set-theory-syllabus.md` 已随 2026-09-30 的 64 份文件清理删除
> （`git log --all -- docs/design/set-theory-syllabus.md` ⇒ `dae75759`），
> 而**五处活文档仍在引用它** ⇒ 本轮重建并把引用重新锚到**存在**的文件 ✓。
>
> **上游**：`teaching-project.md`（分期 P0–P7 · §4 卷/章/单元三层 · §5 DoD 九条）·
> `course-stdlib.md`（L1/L2/L3 分层判据，硬规则 10）·
> `v077-kernel-deficiencies.md`（内核墙 G-56/58/59/64）· `v077-st1-boundary.md`（ST1 分界）。
>
> **证据标注**：**[实测]** = 本机跑过（命令与输出在 §6）；**[教材]** = 逐条查过目录页
> （URL 在 §7）；**[推断]** = 写明依据的估计。**本文件不改课程**（S-A 的判据之一）✓。

---

## 0. 一句话

**卷 I 的靶子 = Enderton《Elements of Set Theory》ch 1–4 + 6–8 + Halmos《Naive Set Theory》
§1–25**（不含实数构造，那是卷 II）。**已落地 12 单元**（教材 ch 1–3 + 基数 Ⅰ/Ⅱ 的骨架）；
**缺口 = 6 个新章 / 10 个新单元**（序关系 · 序数 · 选择公理与 ZF · 基数算术 · 集族广义积），
其中**秩 / 超限递归 / Aleph / 序型**被内核墙挡着（§5，**不是排期问题**）。

---

## 1. 教材基准（[教材]，目录页逐条查过）

### 1.1 Enderton《Elements of Set Theory》（1977，9 章）

| 章 | 节 | 我们的落点（现状） |
|---|---|---|
| 1 Introduction | Baby Set Theory · Sets—An Informal View · Classes · Axiomatic Method · Notation | 单元①（集合与隶属）· 单元⑪（论域与 Russell）✓ |
| 2 Axioms and Operations | Axioms · Arbitrary Unions and Intersections · Algebra of Sets · Epilogue | 单元③④（代数）✓ · **公理表与广义并交无单元** ⬜ |
| 3 Relations and Functions | Ordered Pairs · Relations · n-Ary Relations · Functions · Infinite Cartesian Products · Equivalence Relations · Ordering Relations | 单元⑤⑥⑦ ✓ · **n 元关系 ⬜ · 无限笛卡尔积 ⬜ · 序关系 ⬜** |
| 4 Natural Numbers | Inductive Sets · Peano's Postulates · Recursion on ω · Arithmetic · Ordering on ω | **整章无单元** ⬜（语言内建 `Nat` + `Nat.rec` 可承接） |
| 5 Construction of the Real Numbers | Integers · Rational Numbers · Real Numbers | **卷 II**（`teaching-project.md` §4.1）|
| 6 Cardinal Numbers and AC | Equinumerosity · Finite Sets · Cardinal Arithmetic · Ordering Cardinal Numbers · **Axiom of Choice** · Countable Sets · Arithmetic of Infinite Cardinals · Continuum Hypothesis | 单元⑨⑩（等势/可数/Cantor）✓ · **基数算术 ⬜ · 基数序 ⬜ · 选择公理 ⬜ · CH ⬜** |
| 7 Orderings and Ordinals | Partial Orderings · Well Orderings · Replacement Axioms · Epsilon-Images · Isomorphisms · Ordinal Numbers · Debts Paid · **Rank** | **除序数词汇外全缺** ⬜（`lib/Ordinal` 已落词汇，无单元）· **Rank 被内核墙挡** ✗ |
| 8 Ordinals and Order Types | Transfinite Recursion Again · Alephs · Ordinal Operations · Isomorphism Types · Arithmetic of Order Types · Ordinal Arithmetic | **整章被内核墙挡** ✗（G-56 + G-58）|
| 9 Special Topics | Well-Founded Relations · Natural Models · Cofinality | **整章被内核墙挡** ✗ |

### 1.2 Halmos《Naive Set Theory》（25 节）

| 节 | 我们的落点 | 节 | 我们的落点 |
|---|---|---|---|
| 1 Extension | 单元②（`Set.ext`）✓ | 14 Order | **⬜ 本轮起（单元⑬）** |
| 2 Specification | 单元③（`Set.sep`）✓ | 15 Axiom of Choice | **⬜（单元⑰）** |
| 3 Unordered Pairs | 单元②（`{a,b}`）✓ | 16 Zorn's Lemma | **⬜（形式可写，证明撞墙）** |
| 4 Unions & Intersections | 单元③ ✓ · **广义 ⋃₀/⋂₀ ⬜** | 17 Well Ordering | **⬜（单元⑭）** |
| 5 Complements and Powers | 单元③（`ᶜ`/`𝒫`）✓ | 18 Transfinite Recursion | **✗ 内核墙** |
| 6 Ordered Pairs | 单元⑤（含 Kuratowski）✓ | 19 Ordinal Numbers | **⬜（单元⑮）** |
| 7 Relations | 单元⑥ ✓ | 20 Sets of Ordinal Numbers | **⬜（单元⑯）** |
| 8 Functions | 单元⑦ ✓ | 21 Ordinal Arithmetic | **✗ 内核墙** |
| 9 Families | **⬜（单元㉑）** | 22 Schröder–Bernstein | **⬜（单元⑳）** |
| 10 Inverses & Composites | 单元⑦⑧ ✓ | 23 Countable Sets | 单元⑩ ✓ |
| 11 Numbers | **⬜（单元⑲ 的自然数侧）** | 24 Cardinal Arithmetic | **⬜（单元⑲）** |
| 12 The Peano Axioms | **⬜（单元⑲）** | 25 Cardinal Numbers | 单元⑨ + `lib/Cardinal` ✓ |
| 13 Arithmetic | **⬜（单元⑲）** | | |

**口径**：Halmos 的 §11–13（Numbers / Peano / Arithmetic）与 Enderton ch 4 同靶 ——
本语言里 `Nat` 是**内建原语**（`Nat.zero`/`Nat.succ`/`Nat.rec`/`Nat.add` 在 prelude，
[实测] `crates/front/src/compile/prelude.rs:85-90`）⇒ 这一章**不构造 ω**（那要归纳集
公理 + 一个全集类型，本语言没有），而是**验证 Peano 公理 + 用 `Nat.rec` 做递归**。

---

## 2. 现状盘点（[实测]，2026-10-01）

```bash
python3 courses/set-theory/tools/check.py --json     # 43 目标 · 377 checked · 99 open · 0 判负
```

| 资产 | 数量 | 内容 |
|---|---:|---|
| 单元画布 | 12 + 1 | unit01–unit12 + `notation-cheatsheet`（记法速查页，不是单元）|
| 解答 | 12 + 1 | `units/solutions/`，全 0 open |
| 标准库模块 | 16 | `Logic`(空壳) · `Set` 28 · `Exists` 3 · `Prod` 7 · `Rel` 7 · `Fun` 16 · `Image` 6 · `Equiv` 5 · `SUnion` 6 · `Sum` 4 · **`Ordinal` 10 · `Cardinal` 7 · `Choice` 4 · `ZF` 7 · `Extensionality` 4** · `Demo` 10 |
| 练习 | 99 | T（项填空）/ L（引理链）/ D（判真假）/ R（读评译）/ X（形式↔散文）五类 |

**关键事实（这一条决定了后续排期）**：**加粗的 5 个模块（ST8/ST10/ST12/ST13/ST14，
v0.77.0 落地）今天一个单元都没吃到** —— 它们是**只有库、没有课**。
「最完整版」的第一块缺口就在这里：**不是缺库，是缺课**。

---

## 3. 分界口径（**不改**，逐字沿用 `course-stdlib.md` §0/§1）

> **Mathlib 有 ≠ 我们不该练；Mathlib 有且没有数学内容（纯定义展开）才归库。**

| 层 | 放什么 | 本卷的落点 |
|---|---|---|
| **L1 prelude** | 逻辑与等式骨架 + `Nat`/`Bool`/`Eq`/`Quot` | 语言仓 `PRELUDE_L1_SRC`（**课程不写**）|
| **L2 课程标准库** | 词汇 + 定义展开（序关系的四个谓词、序数的七个定义…）| `courses/set-theory/lib/` |
| **L3 单元练习** | 一切**有数学内容**的陈述（三歧性、良序 ⇒ 最小元、SB 定理…）| `courses/set-theory/units/` |

**每个新单元开工前先回答一句**：这条陈述**删掉名字后是不是 `Iff.intro … (fun h => h)` 或
`rfl` 级**？是 ⇒ 进库；不是 ⇒ 进单元（`course-stdlib.md` §6 的机械判据）。

---

## 4. 卷 I 章表（**完整版**：12 已落地 + 10 新单元）

> 编号沿用清单 v2 的卷 → 章 → 单元三层（`course.json` 的 `soko.course/2`）。
> 每章 `quota.exercises` 只**报告**差额、**不判红**（G6 只判形状）。

| 章 | 标题 | 先修 | 教材靶子 | 单元 | 状态 |
|---|---|---|---|---|---|
| I.1 | 集合、子集与集合运算 | — | Enderton §2.3 · Halmos §1–5 | 1–4 | ✅ 已落地（35 题）|
| I.2 | 序对、关系与函数 | I.1 | Enderton §3.1–3.4 · Halmos §6–10 | 5–7 | ✅ 已落地（24 题）|
| I.3 | 像、原像与基数 | I.2 | Enderton §6.1/6.6 · Halmos §23 | 8–10 | ✅ 已落地（23 题）|
| I.4 | 论域、悖论与综合 | I.1 · I.3 | Enderton §1.2–1.4 | 11–12 | ✅ 已落地（14 题）|
| **I.5** | **序关系与良序** | I.2 | **Enderton §3.7 + §7.1–7.2 · Halmos §14/17** | **13–14** | ⬜ **本轮起** |
| **I.6** | **序数** | I.5 | **Enderton §7.6 · Halmos §19–20** | **15–16** | ⬜ |
| **I.7** | **选择公理与 ZF 公理体系** | I.3 · I.5 | **Enderton §2.1 + §6.5 + §7.3 · Halmos §2/15** | **17–18** | ⬜ |
| **I.8** | **基数算术** | I.3 | **Enderton §6.3–6.4 · Halmos §22/24** | **19–20** | ⬜ |
| **I.9** | **集族、广义积与 n 元关系** | I.2 · I.5 | **Enderton §2.2 + §3.3 + §3.5 · Halmos §9** | **21–22** | ⬜ |
| **I.10** | **关系闭包、等价关系与商**（新增；2026-10-01） | I.2 | **Enderton §3.5 · Halmos §10/§18** + 内核 `Quot` | **24–27** | ⬜ |
| **I.11** | **集合代数与序的完备性**（新增；2026-10-01） | I.1 · I.5 | **Halmos §5–6/§14 · Enderton §1.8/§2.2/§7.1** | **28–32** | ⬜ |
| **I.12** | **幂集与函数空间**（新增；2026-10-01） | I.1 | **Halmos §5–6/§8 · Enderton §1.8/§2.2** | **33** | ⬜ |
| I.13 | 秩与超限递归 | I.6 | Enderton §7.8 + ch 8–9 | — | ✗ **内核墙**（§5）|

### 4.1 新单元的「必证 / 必破」（每单元 DoD 九条见 `teaching-project.md` §5）

**单元⑬ 偏序、全序与严格序**（新库 `lib/Order`）
- 必证：严格序 ↔ 非严格序的**互相翻译**（`a < b ↔ a ≤ b ∧ a ≠ b` 且 `a ≤ b ↔ a < b ∨ a = b`）；
  偏序的**反对称**用法（`a ≤ b → b ≤ a → a = b`）；全序的**三歧**（`≤` 版与 `<` 版等价）；
  序的**限制**（`r` 在子集 `A` 上仍是偏序/全序）。
- 必破：`⊆` 在 `Set α` 上是**偏序但不是全序**（给两个不可比集合）；
  「对称 + 传递 ⇒ 自反」的反例（沿用单元⑥ 的形状，但换成序的语言）。

**单元⑭ 良序与良基**（`lib/Order` + `lib/Ordinal` 的 `HasLeast`/`EWellFounded`）
- 必证：良序 ⇒ **每个非空子集有最小元**（`HasLeast` 的定义展开）；良序 ⇒ **无无穷下降链**
  （用 `Nat → A` 的序列陈述，**这是本单元唯一需要 `Nat.rec` 的题**）；
  良基 ⇒ 无自反环（`¬ r a a`）；`<` 良序 ⇒ `≤` 良序（翻译题）。
- 必破：`<` 在 `Nat` 上良序、在 `Int`-式（两个方向都有元素）上**不良序**；
  「良基 ⇒ 全序」是**假**的（给偏序反例）。

**单元⑮ 传递集与序数**（`lib/Ordinal`，零新类型）
- 必证：`IsOrdinal` 的定义展开（传递 + 元素传递 + 元素良基 + 三歧）；
  序数的元素是序数；序数是传递集（L2 引理 `ordinal_isTransitive` 的**用法**）；
  后继序数的**唯一性**（`IsSuccOf` 的两条给出相等）。
- 必破：`{∅}`-式非传递集不是序数；「传递 + 三歧」**不足以**是序数（良基那一半不能省）。

**单元⑯ 序数的序与上确界**（`lib/Ordinal`）
- 必证：序数上 `∈` 的**传递性**与**三歧性**；最小元 = 交（`HasLeast` 与 `E` 的关系）；
  序数集的**上确界**（`⋃₀` 式陈述）仍是序数。
- 必破：全体序数**不是集合**（Burali-Forti 的谓词版，与单元⑪ 的 Russell 呼应）。

**单元⑰ 选择公理**（`lib/Choice`）
- 必证：**满射可裂**（`Surjective f ⇒ ∃ g, RightInverse g f`）—— 这一条在
  `course-stdlib.md` §3.2-B 里曾是**证不出来**的样板（`Exists.elim` 的 `Q` 只能是 Prop），
  `choice` 外挂之后它**变成可证**（本单元的头号结果）；
  选择函数的**存在性**（`choice_spec` 的用法）；`Nonempty` 与 `∃` 的翻译。
- 必破：「有限情形不需要 AC」—— 给一个不用 AC 就能构造选择函数的例子；
  「AC 能构造出**唯一**的选择函数」是**假**的。

**单元⑱ ZF 公理体系与正则性**（`lib/ZF` + `lib/Set` + `lib/SUnion`）
- 必证：**公理表**（哪条是公理、哪条在谓词式里退化成定理 —— 逐条指路）；
  由正则性推出 **`¬ E x x`**（没有集合属于自身）；推出**无 2-循环**（`¬ (E x y ∧ E y x)`）；
  由正则性推出 **`EWellFounded`**（非空集有 `E`-极小元）。
- 必破：`E x x` 在**没有正则性**的模型里可以成立（说明这条公理**不可省**）。

**单元⑲ 基数算术**（`lib/Cardinal` + `lib/Sum` + `lib/Prod`）
- 必证：`Cardinal` 的**良定义**（`Cardinal.sound` 的用法）；`κ + λ`（`Sum` 的商）与
  `κ · λ`（`Prod` 的商）的**交换律**；`κ + 0 = κ`、`κ · 1 = κ`（单位元）。
- 必破：`κ + λ = κ` 对**有限**基数不成立（给反例）；
  「`Cardinal` 的等价律（`refl`/`symm`/`trans`）」—— 登记 **G-62**（def 形态与展开形态不同一），
  本单元**只陈述、不证**，如实标缺口。

**单元⑳ Schröder–Bernstein 定理**（`lib/Cardinal` + `lib/Fun` + `lib/Image`）
- 必证：**Cantor–Bernstein**：两条单射给出等势（教材 Halmos §22 的核心定理）——
  在**集合层**陈述（`A ↪ B → B ↪ A → A ≈ B`），证明走 Knaster–Tarski 式的
  不动点构造（**不用 AC**，这是它的教学价值）。
- 必破：「两条满射给出等势」不成立（反例：`Nat` 上的移位）。
- ⚠ **先探针**：证明要用到 `Set.image`/`Set.preimage` 的代数与不动点，
  **开工前先判它在本语言里能不能走通**（探针结论写进本文件 §8）。

**单元㉑ 集族与广义并交**（`lib/SUnion`）
- 必证：`⋃₀`/`⋂₀` 的成员刻画；`⋃₀ {A, B} = A ∪ B`；`⋃₀ ∅ = ∅`；
  `A ∈ F → A ⊆ ⋃₀ F`；`⋂₀ F ⊆ A`（`F ∋ A`）；单调性。
- 必破：`⋂₀ ∅` 是**全集**（不是空集）—— 教材里最经典的反直觉点。

**单元㉒ 广义笛卡尔积与 n 元关系**（`lib/Fun` 的 `Set.pi`）
- 必证：`Set.pi` 的成员刻画（`Set.mem_pi` 的用法）；`∏` 在**空指标集**上是单元集；
  二元积是广义积的特例；n 元关系的**前缀编码**（`A × B × C ≅ (A × B) × C`）。
- 必破：`∏` 在**非空指标集 + 有因子为空**时是空集；空积 ≠ 空。

---

## 5. 内核墙（**不是排期，是能力**）

`v077-kernel-deficiencies.md` §三：**四章被挡**，两条 blocker 各自独立：

| 墙 | 缺口 | 挡住什么 | 状态 |
|---|---|---|---|
| 良基递归 | **G-56**（`Acc` 的 uniform 检查）+ **G-64**（带索引归纳的递归子宇宙代入）| 传递闭包 · 秩 `rank` · 超限递归 · 序型 | open（用户 2026-09-28：**别自己开工**，要修先问）|
| 大消去 | **G-58**（`Prop` 归纳消去不到 `Type`）+ **G-59**（`Type` 值归纳的默认 motive 是 `Prop`）| 任何「`Prop` 入、`Type` 出」的定义 | open（**先修 G-58**，收益最大、不涉一致性风险）|

**⇒ 排期口径**：I.13（秩/超限递归/Aleph/序型）**不排单元**，直到 §5 两条墙里
**至少 G-58 关账**；其余 5 章（I.5–I.9）**零内核依赖**，可全速推进 ✓。

---

## 6. 判据与复跑（**本文件自己不改课程**）

```bash
# S-A 的判据：五处引用都指向存在的文件
grep -rn "set-theory-syllabus" --include=*.md .        # 每处都指向本文件 ✓
python3 scripts/docs-lint.py                            # 预算/总量/接手路径 ✓
python3 scripts/docs-expiry-check.py --check             # 过期日期已登记 ✓

# 课程门禁（S-A **不动**课程 ⇒ 数字必须原样）
python3 courses/set-theory/tools/check.py --json         # 43/377/99/0
python3 scripts/notation-lint.py                         # 记法 ✓

# 新单元开工后的判据（S-B）
python3 courses/set-theory/tools/check.py                # G1–G6 全绿
node scripts/soko grade "$PWD/courses/set-theory/units/solutions/unit13-solution.sokonanoda"
```

**DoD 九条**（`teaching-project.md` §5，每个新单元逐条走）：大纲条目 → 画布（演示 + 练习
+ `soko:hint`）→ 解答（项风格、0 open）→ 判卷证据 → 进清单 → 文档 → 台账 → 门禁。

---

## 7. 出处（[教材] 的 URL）

- Enderton《Elements of Set Theory》目录页（Elsevier，9 章逐节）：
  <https://shop.elsevier.com/books/elements-of-set-theory/enderton/978-0-08-057042-6>
- Halmos《Naive Set Theory》目录页（Library of Congress，25 节）：
  <https://catdir.loc.gov/catdir/enhancements/fy0814/74010687-t.html>
- 内核不足清单（G-56/58/59/64 的复现件与源码定位）：`docs/design/v077-kernel-deficiencies.md`
- ST1 分界决策（哪些用类型论自身表达、哪些外挂）：`docs/design/v077-st1-boundary.md`
- 三层分界与「消除暴力」：`docs/design/course-stdlib.md`

---

## 8. as-built（逐轮追加，**别改上面的计划段**）

### 2026-10-01 · as-built（**逐单元明细在 `courses/set-theory/README.md`；本节只留状态与指针**）

- **S-A** ✓（本文件重建）。**S-B 已落二十二片**：I.5–I.11 章（⑬–㉜）· **新章 I.12（㉝）**
  ⇒ **I.5–I.11 七章收口**，门禁 **86 · 846 · 307 · 0** ✓
  elaborator 边界，**十个实例**）· **G-74**（无排中律）· **G-75**（无 `Quot.exact`）· **G-76**（`Nat.add`
  不在变量上展开）· **G-77**（无函数外延）。**绕法一律收进库**，**内容型引理一条不进库** ✓
- **如实不排**（等关账再补，**一律不改题绕开**）：三歧性与 `Aᶜᶜ = A`、De Morgan 的另一半（**G-74**）·
  无序对/并的唯一性（**G-73**）· Zorn 与 **Schröder–Bernstein**（**G-58**）· 商与等价类的往返（**G-75**）·
  ℕ 算术与实数构造（**G-76**）· 逐点序是偏序（**G-77**）✓
- **I.8–I.10 章（第八–十五片）**：⑳（基数算术：积）· ㉑㉒（集族/像原像与广义并交）· ㉓（广义积 ——
  **旗舰必破：选择公理 = "每个因子非空 ⇒ 积非空"**）· ㉔（关系闭包，**交集式定义 ⇒ 零递归**）·
  ㉕㉖（等价关系与划分**两方向**）· ㉗（商类型 —— 无 `Quot.exact` ⇒ **G-75**）✓
- **新章 I.11（第十六–二十片）**：㉘（集合代数 —— **De Morgan 在直觉主义下不对称**：`(A ∪ B)ᶜ = Aᶜ ∩ Bᶜ`
  整条可证、`(A ∩ B)ᶜ = Aᶜ ∪ Bᶜ` **只有一半** ⇒ **G-74**）· ㉙（确界 —— **`⋃₀ F`/`⋂₀ F` 就是上/下确界**）·
  ㉚（格与对偶 —— **对偶原理**；**格运算做不成** ⇒ **G-58**）· ㉛（单调与反射 —— **序同构保持上确界**，
  **三条假设缺一不可**）· ㉜（逐点序 —— **逐点上确界 = 逐点取**；⚠ 逐点序**不是**偏序 ⇒ **G-77**）✓
- **新章 I.12（第二十一·二十二片）**：㉝（幂集代数与特征函数，Halmos §5–6/§8）—— **`𝒫 (A ∩ B)` 与交
  完全交换** · **`𝒫 A ∪ 𝒫 B ⊆ 𝒫 (A ∪ B)` 只有一半** · **`⋃₀ (𝒫 A) = A`** · **特征函数** ⇒
  门禁 **86/846/307/0** ✓。**两件做不成的事**：反例要**区分 `Bool` 构造子**（递归子只有 Prop 动机 ⇒
  **G-58** 同源）· **`𝒫 A → (A → Bool)` 要可判定性**（**G-74**）✓
- **I.13（秩/超限递归/Aleph）仍是内核墙**（G-56/G-58）；**实数构造**的前置是 **G-76** ✓

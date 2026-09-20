# 入门课 Lean 4 风格改写：结构性事实与约束（只读调研）

> ⚠️ **2026-09-21 追记（C2.5 落地后读本文）**：用户拍板**删掉入门课的自建 `And`/`Or` 骨架**，
> prelude 的真归纳接管。本文里「`And` 永远是 axiom」「6 个单元 + playground + 11 份解答都抄了
> `shared/And`」「`AND_COPIES` 24→25 份」这类描述**描述的是 C2.5 之前的状态**，保留作调研底稿；
> 现状见 `docs/notes/course-lean-style/C25-delete-skeletons-brief.md` 与
> `docs/design/course-lean-style.md` §9 的 C2.5 as-built。

> 日期：2026-09-19。范围：`course/`（11 单元 × 中英 × 画布/解答 = 44 个教学文件）
> + 根 `playground.sokonanoda`（362 行），以及与它们同轮必须动的元数据与 CI 测试。
> 方法：`wc -l` / `grep -c` / `diff <(grep -v '^--' A) <(grep -v '^--' B)` / `read`。
> **未跑判卷、未使用 Lean 工具链、未修改任何现有文件。**
>
> 本文只回答「结构性事实与约束」，不含逐声明清单（那份在
> `docs/notes/course-lean-style/course-inventory.md`，但那是对**卷 I** 的）。
> 引擎侧的硬约束（tactic 白名单、`apply` 合一、记法可行性）已有审计，本文只在
> 需要时引用：`tactic-audit.md`、`notation-audit.md`、`tooling-impact.md`。
>
> **一句话结论**：中英 22 对文件**剔注释后代码逐字相同**（唯一例外是 unit1 多 3 个
> 空行），所以「机械替换」可以一套脚本打两边；但真正的杠杆不是替换而是**删**——
> prelude 早已自带 Lean core 级的真归纳 `And`/`Or`（`prelude.rs:201,207`），
> 各单元自己那套 `axiom And/Or` 骨架是**冗余且会让整族让位**的；删掉它，
> `And.rec`/`And.elim`/`Or.elim` 自动回来，`constructor`/`cases` 才有地基。

---

## Q1 CN/EN 对称性 —— 剔掉注释行后代码是否逐字相同

**口径等价性先确认**：`crates/cli/tests/course_shared.rs:88-94` 的 `code_only()`
是按**行内任意位置**的 `--` 切一刀再丢空行。我实测过 `course/` + `playground`
全部 `.sokonanoda`：**没有任何一行是「非行首 `--`」**（即没有行尾注释），
所以 `grep -v '^--'` 与 `code_only()` 在本语料上等价。下面结论对两者都成立。

| 对 | 结果 |
|---|---|
| `course/unit1-propositions-proofs.sokonanoda` ↔ `en/…` | **不同**：仅空行。EN 侧多 3 个空行（EN 原始行 15 / 18 / 20；`diff` 报 `14a15, 16a18, 17a20`）。再剔空行后完全一致 |
| `unit2-equality-rfl` | 相同 |
| `unit3-functions-arrows` | 相同 |
| `unit4-by-tactics` | 相同（CN 84 行 / EN 90 行，差的 6 行全在注释） |
| `unit5-universes-sort` | 相同 |
| `unit6-induction-recursion-1` | 相同 |
| `unit7-induction-recursion-2` | 相同 |
| `unit8-quantifiers` | 相同 |
| `unit9-relations-connectives` | 相同 |
| `unit10-reading-proofs` | 相同 |
| `unit11-modules-projects` | 相同 |
| `solutions/unitN-*-solution` ↔ `en/solutions/…`（11 对） | **11/11 全部相同**（含 unit2/unit5/unit6/unit7 这几个行数不同的——差异全在注释） |

**对改写的意义**：代码行可以**一套机械替换同时打中英两边**（21 对逐字同构，
unit1 只是空行），但**注释行必须分别重写**（EN 是「按语义重构的自然英文」，
不是逐行镜像，见 `course/README.md:44-46`）。所以脚本要分两段：代码段共用，
讲解段各写各的。

---

## Q2 规模

`find course -name '*.sokonanoda'` = **52 个文件 / 4337 行**（题面说的 4337 行是
这个口径）。其中 44 个教学文件 = **2993 行**；加 `playground.sokonanoda` 362 行
= 3355 行；再加 `unit11-project/`（3+1 文件 59 行）与 `shared/`（4 文件 70 行）
= 4337 行。

列含义：`DECL` = 行首 `def|theorem|example|axiom|inductive` 的行数；
`AX/IND` = `axiom`/`inductive` 行数；`BY` = `:= by` 出现次数；
`HINT` = `-- soko:hint` 行数；`RED` = 行首 `#reduce` 数。

中英两版**除行数外所有计数完全相同**（Q1 已证代码逐字同构），所以 EN 只单列行数：
unit1 **107** / unit2 **109** / unit3 **97** / unit4 **90** / unit5 **100** / unit6 **162** /
unit7 **155** / unit8 **162** / unit9 **200** / unit10 **161** / unit11 **249**。

| 文件（CN） | 行 | DECL | AX | IND | BY | HINT | RED |
|---|---|---|---|---|---|---|---|
| `course/unit1-propositions-proofs` | 92 | 20 | 11 | 0 | 0 | 18 | 1 |
| `course/unit2-equality-rfl` | 98 | 8 | 0 | 0 | 0 | 15 | 2 |
| `course/unit3-functions-arrows` | 90 | 8 | 0 | 0 | 0 | 18 | 2 |
| `course/unit4-by-tactics` | 84 | 18 | 10 | 0 | **10** | 15 | 0 |
| `course/unit5-universes-sort` | 91 | 6 | 0 | 0 | 0 | 18 | 1 |
| `course/unit6-induction-recursion-1` | 149 | 13 | 0 | 2 | 0 | 18 | 3 |
| `course/unit7-induction-recursion-2` | 140 | 11 | 0 | 3 | 0 | 12 | 4 |
| `course/unit8-quantifiers` | 142 | 22 | 11 | 0 | 0 | 21 | 1 |
| `course/unit9-relations-connectives` | 177 | 21 | 4 | 3 | 0 | 24 | 0 |
| `course/unit10-reading-proofs` | 139 | 13 | 4 | 1 | 0 | 18 | 0 |
| `course/unit11-modules-projects` | 199 | 13 | 4 | 1 | 0 | 18 | 0 |
| `course/solutions/unit1…-solution` | 41 | 20 | 11 | 0 | 0 | 0 | 1 |
| `course/solutions/unit2…` | 36 | 9 | 0 | 0 | 0 | 0 | 2 |
| `course/solutions/unit3…` | 23 | 8 | 0 | 0 | 0 | 0 | 2 |
| `course/solutions/unit4…` | 37 | **27** | **15** | 0 | 8 | 0 | 0 |
| `course/solutions/unit5…` | 26 | 6 | 0 | 0 | 0 | 0 | 1 |
| `course/solutions/unit6…` | 121 | 13 | 0 | 2 | 0 | 0 | 10 |
| `course/solutions/unit7…` | 109 | 11 | 0 | 3 | 0 | 0 | 6 |
| `course/solutions/unit8…` | 54 | 22 | 11 | 0 | 0 | 0 | 1 |
| `course/solutions/unit9…` | 76 | 21 | 4 | 3 | 0 | 0 | 0 |
| `course/solutions/unit10…` | 39 | 13 | 4 | 1 | 0 | 0 | 0 |
| `course/solutions/unit11…` | 40 | 13 | 4 | 1 | 0 | 0 | 0 |
| `course/en/solutions/*` | 25–126 | 与 CN 逐一相同 | 同 | 同 | 同 | 0 | 同 |
| **`playground.sokonanoda`** | **362** | **36** | **17** | 0 | **2** | **36** | 0 |

**读法**：`:= by` 在入门课里**只存在于 unit4（画布 10 + 解答 8）和 playground（2）**；
其余 10 个单元是**纯项模式**（0 个 `by`）。`-- soko:hint` 是每个画布 12–24 条、
解答里 0 条。

---

## Q3 自建逻辑骨架：哪些文件、axiom 还是 inductive、哪几行

| 文件 | 骨架 | 行号 | 具体 |
|---|---|---|---|
| `course/unit1-propositions-proofs` | **全 axiom** | 17–27 | `True`(17) `True.intro`(18) `False`(19) `False.rec`(20) `And`(21) `And.intro`(22) `And.left`(23) `And.right`(24) `Or`(25) `Or.inl`(26) `Or.inr`(27) |
| `course/unit4-by-tactics` | **全 axiom** | 23–32 | 同上一套去掉 `False.rec`：`True`(23) `True.intro`(24) `False`(25) `And`(26) `And.intro`(27) `And.left`(28) `And.right`(29) `Or`(30) `Or.inl`(31) `Or.inr`(32) |
| `course/unit8-quantifiers` | **全 axiom** | 15–20, 44–45, 101–103 | `True`+`True.intro`+`And` 四行(15–20)；`Person`/`someone`(44–45)；`Exists`/`.intro`/`.elim`(101–103)。**没有 `False`、没有 `Or`** |
| `course/unit9-relations-connectives` | **axiom + inductive 混合** | 19–22, 34–37, 119–122(±), 154(±) | `And` 四行 **axiom**(19–22)；`Or` **inductive**(34–37，`ctor inl`/`ctor inr` 裸名，规范名 `Or.inl`/`Or.inr`，前端派生 `Or.rec`)；`Le` **inductive**(119)；`Even` **inductive**(154) |
| `course/unit10-reading-proofs` | **axiom + inductive** | 18–21, 23–26 | `And` 四行 **axiom**(18–21)；`Or` **inductive**(23–26) |
| `course/unit11-modules-projects` | **axiom + inductive** | 17–20, 22–25 | `And` 四行 **axiom**(17–20)；`Or` **inductive**(22–25) |
| `course/unit2 / unit3 / unit5` | **无自建骨架** | — | 完全靠 prelude |
| `course/unit6-induction-recursion-1` | inductive | 21, 86 | `Nat`(21) `Color`(86) |
| `course/unit7-induction-recursion-2` | inductive | 16, 43, 121 | `Nat`(16) `Option`(43) `Vec`(121) |
| `playground.sokonanoda` | **全 axiom** | 84, 89–90, 97, 104, 109–122, 252–253, 312–314 | `Prop`(84) `True`/`False`(89–90) `True.intro`(97) `False.rec`(104) `And` 四行(109–122 区间内) `Or` 三行 `Person`/`someone`(252–253) `Exists` 三行(312–314) |
| `course/solutions/*`、`course/en/solutions/*` | 与对应画布**同一套、同序** | 从第 2 行起 | 解答钥匙是「骨架 + 全部答案」的独立文件，不是 diff |
| `course/shared/And.sokonanoda` / `Or` / `Nat` | axiom / inductive / inductive | 8–11 / 6–9 / 7–24 | 规范文本（`And` 四行；`Or` 归纳块；显式 `Nat` 块含手写 `Nat.rec` + 两条 iota） |
| `course/unit11-project/Logic.sokonanoda` | axiom | 5–8 | 又一份 `And` 四行（**未登记**，见 Q4） |

### 特别确认

- **`And` 永远是 `axiom`**——6 个单元 + playground + 11 份解答 + `shared/And` +
  `unit11-project/Logic`，**没有任何一处是 `inductive`**。所以今天的入门课里
  `And.rec` / `And.elim` **不存在**。
- **`Or` 两种形态并存**：`axiom Or` + `Or.inl`/`Or.inr`（**unit1:25–27**、
  **unit4:30–32**、playground:120–122）；`inductive Or` + 裸 `ctor inl`/`ctor inr`
  （**unit9:34–37**、**unit10:23–26**、**unit11:22–25** 及各自解答）。
  注意 `ctor inl` 的**规范名仍是 `Or.inl`**（`docs/design/ctor-namespace.md` R1：
  `canonical(Ind, ctor) = "Ind.ctor"`），裸名 `inl` 是 R2 别名——所以
  `unit9:41` 的 `inl A B a` 与 `unit10:137` 提示里的 `Or.inl` **都合法**。

### ⚠️ 改写最大的结构性杠杆：这些骨架是**冗余**的

`crates/front/src/compile/prelude.rs:195-231` 的 `PRELUDE_L1_SRC` 已经提供
**Lean core 级**的整套 L1：`True/False/And`(真归纳，201–203)/`Or`(真归纳，
207–210)/`Not`/`absurd`/`Iff`/`Eq.symm/trans/congrArg`。它默认安装
（`PreludeMode::Full` 是 default，`goals.rs:89`）。

而让位是**按族、整族**的（`prelude.rs:141-147`、`233-240`、`254-281`）：
文件自己声明 `And`（哪怕只是 `axiom And`）⇒ **B3 整族让位**，`And.rec`/`And.elim`
一起消失，只剩文件里那四条公理。这正是「`And` 是 axiom ⇒ `constructor`/`cases`
没有地基」的根因。

**推论**：把 unit1/4/8/9/10/11 和 playground 的自建骨架块**整块删掉**，prelude 的
真归纳 `And`/`Or` 自动生效，`And.rec`/`And.elim`/`Or.elim` 免费回来；
而 `And.intro`/`And.left`/`And.right`/`Or.inl`/`Or.inr` 的**签名与位置参数个数
与今天完全一致**（prelude:201-211 vs 各单元公理），所以**项模式代码理论上零改动**
（`Or.inl` 从「4 显式参」变「归纳参数 + 1 构造子参」，仍需内核实测确认一次）。
`course/unit2:45-49` 已经明说「prelude 自带等式与逻辑的整套骨架……不需要你自己造」。

---

## Q4 `course_shared.rs` 的副本清单

`crates/cli/tests/course_shared.rs`（226 行）用三张表把「规范文本 → 逐字副本」
钉死。三张表的成员**互不重叠**，都是「单元 × 中英 × 画布/解答」：

**`AND_COPIES`（22–47 行）= 24 份**：`course/shared/And.sokonanoda` 的四行 `axiom And`。
6 个单元 × 2 语言 × 2 形态：

- 画布：`course/unit1-…`、`course/unit4-by-tactics`、`course/unit8-quantifiers`、
  `course/unit9-relations-connectives`、`course/unit10-reading-proofs`、
  `course/unit11-modules-projects` + 同名的 `course/en/…` 六份
- 解答：上列 6 个名字各加 `course/solutions/…-solution.sokonanoda` 与
  `course/en/solutions/…-solution.sokonanoda`

**`OR_COPIES`（50–63 行）= 12 份**：`course/shared/Or.sokonanoda` 的
`inductive Or` 块。3 个单元（unit9/unit10/unit11）× 2 语言 × 2 形态。
**注意 unit1/unit4 的 `axiom Or` 三行不在表里**——它是另一种文本，不是这份规范文本的副本。

**`NAT_COPIES`（66–75 行）= 8 份**：`course/shared/Nat.sokonanoda` 的显式 `Nat` 块。
2 个单元（unit6/unit7）× 2 语言 × 2 形态。

**四个测试与它们钉住什么**：

| 测试（行号） | 钉住什么 | 改写后会不会红 |
|---|---|---|
| `shared_modules_compile_standalone` (130–144) | `shared/{And,Or,Nat}.sokonanoda` 各自单独 `exit 0` | 若删 `shared/And` ⇒ **编译不过/文件不存在，红** |
| `shared_demo_compiles_through_the_import_closure` (146–174) | `shared/Demo.sokonanoda` 走 import 闭包成功，且事件流含 `and_comm_demo`/`or_comm_demo`/`double` 三个名字 + 至少一条 `expr.reduced` | 若 `shared/And` 改成 inductive 或删掉，`Demo:10-11` 的 `And.intro B A (…)` 仍应成立，但 `double` 依赖 `Nat` 块；**需重跑** |
| `every_unit_copy_matches_the_canonical_module` (176–207) | **双向**：① 表里每个文件必须**逐字包含**规范文本（`contains`，不是行 diff）；② 任何**含**规范文本的文件必须**在表里**（否则报 `extra`）。扫描面 = `corpus_files()`（96–120）只遍历 `course/`、`course/en/`、`course/solutions/`、`course/en/solutions/` **四个目录的顶层** | **必红**：删掉 24 份 And 副本后 ① 全红；若只删一部分，② 还会报 `extra` |
| `the_shared_library_replaces_rather_than_duplicates_the_canvas_purpose` (209–226) | `corpus_files()` 里凡路径含 `/unit` 的文件，**代码行里不许出现 `import `** | 若改写顺手把画布/解答改成 `import` 共享库 ⇒ **红** |

### ⚠️ 盲区：And 块的真实副本是 **25 份**，不是 24

`corpus_files()` 不扫 `course/unit11-project/`。但
`course/unit11-project/Logic.sokonanoda:5-8` 与 `course/shared/And.sokonanoda:8-11`
**逐字相同**。我实测：全 52 个文件里含 And 规范文本的是 **25 个**
（24 登记 + `unit11-project/Logic.sokonanoda`）；Or 是 12+1（+`shared/Or` 自身）；
Nat 是 8+1。**改 `shared/And` 时这份 25 号副本不会被任何测试拦下**——要么把它
登记进表（需要同时扩 `corpus_files()`），要么在改写时手工同步。

---

## Q5 单元④（tactic 教学单元）

`course/unit4-by-tactics.sokonanoda`（84 行，CN；EN 90 行同代码）。

**教的 tactic 与教学顺序**（头部 `-- ` 讲解 12–19 行，**五个，顺序即白名单顺序**）：

1. `intro x`（13 行）——剥一层箭头，等价写一层 `fun`
2. `exact e`（14 行）——用项直接结束当前目标
3. `assumption`（15 行）——从已引入的假设里找类型与目标一致的
4. `apply f`（16–17 行）——目标套上 `f`，换成若干子目标
5. `rfl`（18 行）——目标是 `Eq α x y` 且两边算出一样时成立

外加 `by sorry` 占位（19 行）。语法声明为 `:= by <tactic>; <tactic>; …`（9–10 行）。

**练习**（画布上 5 道，编号**跳过 5**）：

| 练习 | 行 | 声明名 | 题目形状 |
|---|---|---|---|
| 练习 1 | 54 | `by_ex1` | `(a : Prop) -> a -> a`，起点 `:= by sorry`（从零开始） |
| 练习 2 | 60 | `by_ex2` | `(a b : Prop) -> And a b -> a` |
| 练习 3 | 66 | `by_ex3` | `(a b : Prop) -> a -> Or a b`（`apply Or.inl`/`Or.inr` 拆目标） |
| 练习 4 | 72 | `by_ex4` | `(a b : Prop) -> a -> b -> And a b`（`apply And.intro` 出两个子目标） |
| 练习 6（综合） | 83 | `by_ex6` | `(a b : Prop) -> And a b -> And b a` |

**演示 3 条**：`demo_by_assumption`(37) / `demo_by_apply`(42) / `demo_by_rfl`(46)。

**解答用了什么**（`course/solutions/unit4-by-tactics-solution.sokonanoda`，
行号 = 解答文件）：

- `by_ex1`(19)：`by intro a; intro h; assumption`
- `by_ex2`(21)：`by intro a; intro b; intro h; exact And.left a b h`（**用 `exact` 交项，
  不是 `apply`**）
- `by_ex3`(23)：`by intro a; intro b; intro ha; apply Or.inl; exact ha`
- `by_ex4`(25)：`by intro a; intro b; intro ha; intro hb; apply And.intro; exact ha; exact hb`
- `by_ex6`(27)：`by intro a; intro b; intro h; exact And.intro b a (And.right a b h) (And.left a b h)`
  —— **回退成项模式**（一条 `exact` 塞进整个项），没有真正「串起几个 tactic」

**解答里的 4 条画布上没有的遗留声明**（29–37 行）：`axiom h_s`/`Pfam`/`Qfam`/`f_dep`/
`qfam_true` + `theorem val_apply_imp`/`val_apply_dep`/`by_ex7`/`by_ex8`
（`by_ex7 : And True True`、`by_ex8 : Or True True` 都是**项模式**写的）。
即解答是画布的**超集**（CI 的 `solution_covers_every_canvas_exercise` 只要求
「画布练习名都能在解答里找到」，超集不判红）。

**对改写的意义**：这一单元要**重新教新 tactic 集**（今天的五个里，`assumption`
在真实 Lean 4 里几乎不教，`rfl` 的讲法要改；缺 `constructor`/`cases`/`rw`/`use`/
`have`/`simp`）。顺序、练习编号（缺 5）、解答的「回退成项模式」都要重排；
解答里那 9 条遗留声明是历史包袱，可一并清掉。

---

## Q6 改写后会变成假话的叙事（`文件:行号` + 原句）

### A. 「用 axiom 搭逻辑骨架」

| 位置 | 原句 |
|---|---|
| `course/unit1-propositions-proofs.sokonanoda:14` | 「我们用 axiom 搭最小逻辑骨架（与官方 Lean 的 And/Or 同构）：」 |
| `course/en/unit1-propositions-proofs.sokonanoda:17-19` | "To stay minimal we declare a small logical core with axioms; it is isomorphic to official Lean's And/Or…" |
| `course/unit8-quantifiers.sokonanoda:14` | 「逻辑骨架（复用单元①的构造子）：」 |
| `course/en/unit8-quantifiers.sokonanoda:16` | "Logic skeleton (reuses Unit 1's constructors):" |
| `course/unit4-by-tactics.sokonanoda:22` | 「逻辑骨架（复用单元①的构造子）：」 |
| `course/en/unit4-by-tactics.sokonanoda:24` | "Logic skeleton (reuses Unit 1's constructors):" |
| `course/unit9-relations-connectives.sokonanoda:18` | 「逻辑骨架（复用单元①/⑧的 And 公理）：」 |
| `course/en/unit9-relations-connectives.sokonanoda:24` | "Logic skeleton (reuses Unit 1/8's And axioms):" |
| `course/unit10-reading-proofs.sokonanoda:17` | 「骨架（And 公理 + Or 归纳类型 + Iff 定义，均为前几个单元的老朋友）：」 |
| `course/en/unit10-reading-proofs.sokonanoda:23` | "Skeleton (And axioms + Or inductive + Iff definition, all old friends…)" |
| `course/unit11-modules-projects.sokonanoda:15-16` | 「骨架（本文件扮演**被 import 的库模块**：And 公理 + Or 归纳类型 + Iff 定义…）」 |
| `course/en/unit11-modules-projects.sokonanoda:22` | "And axioms + the Or inductive + an Iff definition, all old friends…" |
| `course/unit11-modules-projects.sokonanoda:109` | 「`Logic.sokonanoda` -- 库模块：axiom And / And.intro / And.left / And.right」 |
| `course/en/unit11-modules-projects.sokonanoda:146` | "Logic.sokonanoda -- library: axiom And / And.intro / And.left / And.right" |
| `course/unit8-quantifiers.sokonanoda:97-98` / `course/en/unit8…:112` | 「官方 Lean 的 Exists 是 inductive…这里按单元①的老办法，把两条规则立成公理：」/ "…rules as axioms:" |
| `course/unit11-modules-projects.sokonanoda:170` / `course/en/unit11…:217`（举例被拒模块的代码块内） | 「axiom And : Prop -> Prop -> Prop」 |
| `playground.sokonanoda:20-21` | 「随后用一套迷你逻辑骨架把这条思想跑起来。」 |
| `playground.sokonanoda:130` | 「到这里，本课要用的公理都齐了。」 |
| `playground.sokonanoda:308-309` | 「官方 Lean 的 Exists 是 inductive…这里按第一课的老办法，把两条规则立成公理：」 |
| `course/unit2-equality-rfl.sokonanoda:45-49` | 「prelude 现在自带等式与逻辑的整套骨架（官方 Lean core 级）…」——**这句反而变成改写后的正解**，要保留并前移 |

### B. 「Or 只是公理 / 从公理升级」

| 位置 | 原句 |
|---|---|
| `course/unit9-relations-connectives.sokonanoda:9` | 「单元① 把 ∧ 与 ∨ 立成公理，只教了「怎么用」。本单元补上三块：」 |
| `course/unit9-relations-connectives.sokonanoda:25` | 「9.1 Or：从公理升级为真正的归纳类型」 |
| `course/unit9-relations-connectives.sokonanoda:27-29` | 「单元① 里 Or 只是一条公理加两条引入规则，没有消去子…」 |
| `course/en/unit9-relations-connectives.sokonanoda:12` | "Unit 1 set up ∧ and ∨ as axioms and only taught how to *use* them." |
| `course/en/unit9-relations-connectives.sokonanoda:31` | "9.1 Or: from axiom to a real inductive type" |
| `course/en/unit9-relations-connectives.sokonanoda:33` | "In Unit 1 Or was just an axiom plus two introduction rules; it had no…" |
| `course/unit11-modules-projects.sokonanoda:37` | 「不同义——单元⑨的 `Or` 是真归纳类型，单元①的 `Or` 只是公理；拼进同一个文件…」 |
| `course/en/unit11-modules-projects.sokonanoda:45-46` | "Unit 9's `Or` is a real inductive while Unit 1's `Or` is only an axiom…" |
| `course/unit11-modules-projects.sokonanoda:11.1 节整段` | 「一个名字可以指不同东西」的教学动机**整段失效**（前提是「各文件自造同名公理」） |

### C. 「不许用记法 / 一律点名」与「首期五个 tactic（白名单）」

| 位置 | 原句 |
|---|---|
| `course/unit9-relations-connectives.sokonanoda:74` | 「本教学语言**没有 ↔ 记号**，但「当且仅当」不需要新公理，它就是一个定义：」 |
| `course/en/unit9-relations-connectives.sokonanoda:85` | "The teaching language has **no ↔ notation**, but "if and only if" needs no new…" |
| `course/unit4-by-tactics.sokonanoda:12` | 「**首期五个 tactic（白名单，课程就是语法）**：」 |
| `course/en/unit4-by-tactics.sokonanoda:14` | "**First five tactics (the whitelist is the curriculum)**:…" |

> 注：CN 语料里**没有**「一律点名」「不许用记法」这类成句禁令；最接近的就是上面
> 两条「没有 ↔ 记号」。真正把「点名」制度化的在**课程之外**：
> `REQUIREMENTS.md:18`（硬规则「语法白名单即课程」）、`REQUIREMENTS.md:208-220`
> （首期五个 tactic 的原始指令）、`docs/design/by-tactics.md:5,176,179`、
> `docs/design/course-syllabus.md:225,242`、`skills/sokonanoda-teacher/references/curriculum.md:13`、
> `skills/sokonanoda-teacher/SKILL.md:260`、`crates/front/src/lib.rs:4`。
> 这些同轮不改，就会出现「文档说五个、课程教八个」的自相矛盾。

---

## Q7 必须同轮同步的元数据

### `course/course.json`（68 行）

**没有任何计数**，只有 11 条 `{file, title, title_en, unit}`。会变假话的只有标题里
的**教学法措辞**：`course.json:22` 「单元④ by 写法：tactic 证明」/
`:23` "Unit 4 — Proving with `by` (tactic blocks)"——若 unit4 重排教学内容，标题要跟着改。
`file` 名与 `unit` = 1..11 被 `course.rs:243-279` 逐条钉死，**改名即红**。

### `course/README.md`（89 行）里写死的计数与句子

| 行 | 现在写的 | 为什么变假 |
|---|---|---|
| 13–28 | 单元顺序与各单元定位的长段（含「单元⑨把单元①的 `Or` 升级为真 `inductive`」） | 「升级」叙事失效（Q6-B） |
| 34 | 「`en/unitN-*`：**与中文画布代码逐字节一致**，仅 `--` 注释语言不同」 | unit1 有 3 个空行差异；改写后若两边不同步即假话 |
| 39 | 「由 `crates/cli/tests/course_shared.rs` 双向守住 **24/12/8** 份拷贝不漂移」 | 三个数字都会变（甚至归零） |
| 44–46 | 「英文注释是按语义重构的…知识点、提示阶梯条数与顺序与中文同构」 | 提示条数若变，此句要重核 |
| 53–58 | 「换来的只有约 **8%** 的行数（**232/2974 行**是逐字重复的声明块）」 | 两个数字都变（当前实测教学文件共 2993 行，与 2974 已不一致——**这句话现在就已经是陈的**） |
| 63–64 | 「`And` **24 份**拷贝、`Or` **12 份**、显式 `Nat` 块 **8 份**」 | 同 39 行 |
| 65–67 | 「规范文本变了而某份拷贝没跟上 ⇒ 红；某个文件抄了这段却没登记 ⇒ 也红；画布里出现 `import` 同样红」 | 若删共享库，这三条守卫描述要重写 |
| 74 | 「单元⑩/⑪ 不教新语法、也不引入 `#` 命令，所以它们的 `expr.reduced` 是 **0**（golden 表如实记录）」 | golden 表若重钉，此句要跟着改 |
| 75 | 「**每个单元文件各自带所需 axiom/inductive 块**，独立编译（unit6/unit7 的显式 `inductive Nat` 块会取代该文件内的 prelude Nat）」 | 若删骨架块，**这句直接变成反话** |
| 82–89 | CI 守卫 5 条（含「事件数与 golden 表精确一致」） | 若 golden 重钉，描述要同步 |

### `course/unit11-project/*`

**没有写死的计数**；会变假话的是「公理」措辞：

| 位置 | 原句 |
|---|---|
| `unit11-project/Logic.sokonanoda:3` | 「这是「库」的那一半：教学子集里的 **And 公理**（与单元⑪ 画布、单元⑩ 同一套）。」 |
| `unit11-project/Logic.sokonanoda:5-8` | 四行 `axiom And`（**同时是 Q4 的 25 号未登记副本**） |
| `unit11-project/Canvas.sokonanoda:4-5, 8` | 「本文件自己不需要重复声明**任何一条公理**。」/「用 import 进来的**公理**建一条新导出。」 |
| `unit11-project/Exercises.sokonanoda:3, 16` | 「本文件**不声明任何公理**。」/「本文件**没有任何公理**，能用的定理只有从 Logic 进来的那几条。」 |
| `unit11-project/solutions/Exercises-solution.sokonanoda` | 两条答案都是项模式（`fun … => And.intro …`） |
| `course/unit11-modules-projects.sokonanoda:107-111` | 目录树里把 `Logic.sokonanoda` 注成「库模块：axiom And / …」 |

### `crates/cli/tests/` 里钉住入门课的测试

| 测试（文件:行） | 钉住什么 | 改写后 |
|---|---|---|
| `course.rs:101` `every_course_canvas_compiles_with_golden_event_counts` | `course/` 顶层**每个** `.sokonanoda` 必须 exit 0、stderr 无 `error[`；且 `(decl.checked, exercise.open, expr.reduced)` **精确等于** `GOLDEN`；文件数必须 == `GOLDEN.len()`（**新增/删除文件即红**） | **必红**（删骨架块 ⇒ `decl.checked` 掉、`exercise.open` 不变；改 tactic ⇒ 事件数全变） |
| `course.rs:86-98` `GOLDEN` | 11 元组：unit1 `(13,6,1)`、unit2 `(3,5,2)`、unit3 `(2,6,2)`、unit4 `(13,5,0)`、unit5 `(0,6,1)`、unit6 `(7,6,3)`、unit7 `(7,4,4)`、unit8 `(14,7,1)`、unit9 `(13,8,0)`、unit10 `(7,6,0)`、unit11 `(7,6,0)` | 必须逐单元重钉 |
| `course.rs:158` `every_solution_twin_is_fully_solved` | `solutions/` 文件数 == `GOLDEN.len()`；每份 exit 0、**0 diagnostic**、**0 `exercise.open`** | **必红**（任何一处没改对就红；这也是「改写正确性」的主要守卫） |
| `course.rs:207` `solution_covers_every_canvas_exercise` | 画布每个具名 `exercise.open.name` 必须在解答的 `decl.checked.name` 里出现（**事件流比对，不比对文本**；匿名 `example` 跳过） | 改练习名时**两边必须同名**，否则红 |
| `course.rs:243` `course_json_lists_the_eleven_units_in_order` | `course.json` 恰好 11 条、`file` 名与顺序、`unit` = 1..11 | 不动清单则绿 |
| `course.rs:287` `en_mirrors_match_chinese_event_counts` | 每个 `course/en/` 画布与中文孪生**五项事件计数逐项相等**（`decl.checked`/`exercise.open`/`expr.reduced`/`expr.typed`/`diagnostic`） | **CN 改了 EN 没改即红**——这是双语同步的唯一机器守卫 |
| `course.rs:380` `en_solutions_match_chinese_event_counts` | `course/en/solutions/` 与 `course/solutions/` 同样逐项相等 | 同上 |
| `course_status.rs:68-80` `GOLDEN` | 11 元组 `(checked, open, failed, reduced)`，注释明说「identical to the course.rs golden with failed = 0 throughout」 | **与 `course.rs::GOLDEN` 必须同时改**，否则两处互相打脸 |
| `course_status.rs:83` `course_subcommand_aggregates_the_manifest` | 逐单元四项计数 == `GOLDEN`；`course.summary` 恰一条，`units == 11`、**`checked == 86`**、**`open == 65`**、`failed == 0` | **必红**（86/65 是 11 个单元的和，逐单元一改就变） |
| `course_status.rs:124` `course_subcommand_human_view_lists_units` | 人类可读视图最后一行含 **`65`** + `checked` + `failed` | **必红**（65 变则红） |
| `cli.rs:2070` `cli_course_is_stable_with_a_warm_cache` | 冷/暖缓存 `course.summary` 相等；`checked == 86`、`open == 65`（`cli.rs:2097-2098`） | **必红**（同 86/65） |
| `course_shared.rs:130/146/176/209`（见 Q4） | 24/12/8 副本逐字一致 + `shared/*` 独立可编译 + 画布无 `import` | **必红**（除非整块删骨架并重做表） |

> 另有两个**不钉计数**但会被波及的：`course_manifest.rs:119`（v1 扁平清单事件形状
> 逐字节不变，用的是**自造 fixture**，不受影响）、`course_project.rs:210/234/242/262`
> （import 闭包的 `failed` 与 `grade`/`query check` 同判，用的是**自造 fixture**）。
> 也就是说：**改写入门课会红的是 `course.rs` + `course_status.rs` + `cli.rs` 的 86/65 +
> `course_shared.rs`，共 4 个文件 13 个测试。**

---

## Q8 改写风险 top 8

1. **`And` 是 `axiom`（6 单元 + playground 全是）⇒ 今天没有 `And.rec`/`And.elim`**，
   而白名单里根本没有 `constructor`/`cases`（`parser.rs:2841-2846` 只有
   `intro/exact/apply/assumption/rfl/match/sorry`）——**只删骨架不够，tactic 集本身要先扩**。
2. **删骨架 = 白拿 prelude 的真归纳**：`prelude.rs:201-211` 的 `And`/`Or` 签名与
   各单元公理**位置参数个数一致**，`And.intro A B ha hb` 这类项理论上零改动；
   但 `Or.inl` 从「4 显式参公理」变成「归纳参数 + 构造子参」，**必须先用内核实测一遍**，
   且要注意**整族让位**（声明了 `And` 就连 `And.rec` 一起没了）。
3. **CN/EN 双份 ×（画布 + 解答）= 4 份要同步**：代码可一套脚本打，注释必须各写各的；
   唯一的机器守卫是**事件计数逐项相等**（`course.rs:287/380`），
   注释写错它看不见，代码漏改一边立刻红。
4. **`course_shared.rs` 的 24/12/8 双向表 + `unit11-project/Logic` 的 25 号盲区**：
   删副本要同时改三张表、`corpus_files()` 的扫描面，以及「不许 `import`」那条；
   漏掉 `unit11-project/Logic.sokonanoda` 不会有任何测试报警。
5. **单元④ 要重排教学顺序**：五个 tactic（intro/exact/assumption/apply/rfl）里
   `assumption` 基本要下架、`rfl` 讲法要改，新增的 `constructor`/`cases`/`rw`/`use`/`have`
   要重排顺序；练习编号已缺 5、解答里 9 条遗留声明是包袱；`REQUIREMENTS.md:208-220`
   与三个 skill/设计文档都写着「首期五个」，同轮不改就是文档自相矛盾。
6. **`playground.sokonanoda` 的教学叙事整体建立在「用 axiom 搭骨架」上**（20–21、34–52、
   84–130、309 行），且它是**用户直接面对的画布**（`course/README.md:5-7`）——
   改它的成本高于任何一个单元，且它有 36 条 hint + 2 处 `:= by`。
7. **`And`/`Or` 的两种形态在课程内部已经不一致**（unit1/4 的 `axiom Or` vs
   unit9/10/11 的 `inductive Or`），且 unit11 的 11.1 节**把这种不一致当成教学动机**
   （`unit11:37`）——统一之后那一节要重写，否则学生读到的「为什么需要 import」失去前提。
8. **计数与 golden 三处联动**：`course.rs::GOLDEN`（11 元组 × 3 项）、
   `course_status.rs::GOLDEN`（同 11 元组 × 4 项）、`course_status.rs:118-119` 与
   `cli.rs:2097-2098` 的 **86/65**——逐单元改完必须**重新从内核取数**再钉，
   不能手算；`course.rs:115-119` 还要求「文件数 == GOLDEN 长度」，增删文件即红。

---

## 附：改写前应先落地的引擎前提（引用既有审计，不重复论证）

- **tactic 白名单今天只有 7 个关键字 / 6 个 AST 变体**，`intro a b` 是 **parse 错误**，
  `apply` 只做位置 spine 合一 ⇒ `apply And.left` 这类会失败，
  `match` 臂体只能是项：见 `docs/notes/course-lean-style/tactic-audit.md` §0/§1。
  入门课需要的 `constructor`/`cases`/`use`/`rw`/`have` **全部还没有**。
- **记法**：`∧ ∨ ↔ ¬` 今天直接可用（`infix:N " ∧ " => And` 实测 exit 0），
  `∀` 是原生关键字，`∃` 要 `binder_notation` + `Exists` 在作用域；
  **`→` 是唯一必须动 parser 的符号**（约 2 行），否则只能用 `def Imp` 别名且不支持
  依赖箭头：见 `docs/notes/course-lean-style/notation-audit.md` §1/§3/§4。
- **CN 注释里已经在用 `∧ ∨ → ↔ ∀` 这些符号**（unit9:9、unit8:10,34、unit10:54、
  unit11:11,92 等 14 处），但**代码里 0 处**——记法改写是「把注释里的符号搬进代码」，
  不是从零发明术语。

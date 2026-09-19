# 设计：L1 prelude 提案 —— 把 Lean core 的逻辑与等式骨架装进 prelude（2026-09-18）

> 触发（用户）：硬规则 10「课程标准库三层分界」——**L1 = prelude 应当自带**（`REQUIREMENTS.md` §2 第 10 条）。
> 上游：`docs/design/course-stdlib.md` §1/§2（三层判据 + L1 清单）、`docs/gaps/ledger.jsonl`
> （**L-01**/L-02/L-03、**G-13**/G-14/G-02）、`docs/gaps/spike/README.md`（真写 + 真判卷的试做）。
> 现场：`courses/set-theory/lib/Logic.sokonanoda`（**28 条手写声明**，逐条标了 L-01…L-13）。
> 下游：`docs/design/teaching-project.md` **P-C3**（向语言线提提案）。
> 本文只做设计；落地按 §6 四个阶段，每阶段能独立过 gate。
>
> ## ✅ 落地状态（2026-09-19，as-built）
>
> **P1 / P2 / P3 已实现并全绿**；**P4（课程仓跟随）也已落地**（2026-09-19，
> 见下方 as-built 末条与 §6 的 P4 行）。
>
> * P1：`PRELUDE_L1_SRC`/`L1_FAMILIES`/`PRELUDE_NEVER_YIELDS` + `install_l1_prelude`；
>   `taken` 已换成 `top_level_def_spans_over` 的键集；front 单测 7 条（见 §3.2，名字即契约）。
> * P2：`crates/cli/tests/cli.rs` 四条 e2e；`PRELUDE_NAMES` 补 30 条（**42** 条）；
>   `docs/architecture.md` §5.4.1 新增。
> * P3：单元② 中英画布 + 两份解答；两处 GOLDEN **据实重算**（见下）。
> * **两处 GOLDEN 实测值 = 预测值**：`course.rs` 的 `unit2 = (3,5,2)`；
>   `course_status.rs` 的 `unit2 = (3,5,0,2)`、summary `checked = 86`（原 85）。
>   另发现**第三处** GOLDEN：`crates/cli/tests/cli.rs` 的
>   `cli_course_is_stable_with_a_warm_cache` 也钉着 `checked = 85` ⇒ 同步改成 86。
> * 与提案的三处**出入（as-built 修正）**：
>   1. **安装顺序是先 Eq 后 L1**：B7 的定义体引用 `Eq.subst`/`Eq.refl`，必须等 Eq
>      进环境；两者读同一个 `taken`，所以先后不影响让位结果。
>   2. **新增重入闸 `L1_INSTALL_DEPTH`**（§1.2/§4.1 未预见）：装 `And` 归纳块时
>      `large_elim_test_mirror` 会经 `judge_infer` 触发**内层 `compile_fol_with`**，
>      内层又装 L1 ⇒ 无限递归（实测 `stack overflow, SIGABRT`）。计数 > 0 时直接返回。
>   3. **§3.2 的 `l1_family_yield_is_dependency_closed` 措辞方向写反了**：
>      依赖边是「B6 用 `And.left`」⇒ 声明 `And` 让位 B6（§2.2 的表是规范）；
>      实现与测试按 §2.2，并额外钉了反向对照。
> * 另外：`goals.rs` 的 `refine_template` 对归纳块构造子本来就一直是 `None`
>   （既有边界，见 `ctor_spine_accepts_both_spellings` 的注释），所以 L1 的
>   `And.intro` 走的是 **`sub_goals` 期望类型**，不是 refine 骨架。
> * 复现件（台账已挂）：`docs/gaps/repro/L01-l1-prelude-logic-skeleton.sh`、
>   `docs/gaps/repro/L02-eq-core-lemmas.sh`（修后形状 ⇒ exit 1）。
> * **P4（课程仓跟随，2026-09-19）✅**：课程侧 `courses/set-theory/lib/Logic.sokonanoda`
>   从「28 条保留的兜底副本」**退化成只有注释的空壳模块**（**0 条声明**；34 个
>   `import lib.Logic` 一字未改，实测空模块仍可 import、闭环 exit 0）；并按 §1.4 的
>   预告把项位裸名改成点号名——**实测 65 处项位（61 行 × 10 个文件）+ 27 行注释**
>   （§1.4 记的「74 行」是提案期的行数口径，今天课程更大，以 grep 全量复核为准）。
>   门禁 `python3 courses/set-theory/tools/check.py` = **exit 0 · 36 目标 ·
>   329 checked · 99 open · 0 判负**（checked 的 −26 正是 `lib/Logic` 少掉的 26 条
>   声明，**open 一条不变** ⇒ 没删练习、没加 `sorry`、题义未动）。
>   **§1.3/§1.4 的兼容性假设成立：没有一条声明需要保留**——prelude 与当年抄本的
>   两处定形差异（`And` 由 axiom 族变真归纳、`Or` 的构造子由裸名变点号名）都不改
>   调用形状，`Or.elim`/`And.left`/`Iff.mp` 一类**逐字可用**。
>   台账 L-01/L-02 无需再动（早已 fixed）；`course-stdlib.md` §2/§3/§4/§5 与
>   `teaching-project.md` 的 P-C 收尾段已同轮更新。
>
> ## ✅ B8 补记（2026-09-19，L-03 落地）
>
> L1 再加**一族**：**B8 = `Eq.rec`/`Eq.ndrec`/`Eq.mp`/`Eq.mpr`/`cast`**（Type 层重写）。
> 本文 §5 的「不做 `Eq.mp`/`Eq.mpr`/`Eq.rec`/`cast`」**作废**：实测发现内核的
> large-elimination 规则**给** Eq 形状（`EqT.rec.{1}` 的 motive 落 `Type 0`），
> 堵路的是"prelude 的 `Eq` 是公理"与两条语法边界（`inductive` 头部不吃宇宙 binder、
> 层级语法没有 `u+1`）——所以 B8 走**公理** `axiom Eq.rec {u, v}`（签名与 Lean core
> 逐字同形）+ 由它定义的四条 `def`。
> 设计/实测/残留边界见 **`docs/design/eq-type-level-rewriting.md`**；
> 复现件 `docs/gaps/repro/L03-eq-type-level.sokonanoda`（`scripts/soko grade` exit 0）。
> 三件套同轮落地：`PRELUDE_L1_SRC` + `L1_FAMILIES`（B8，`deps = ["EQ"]`）+
> `PRELUDE_NAMES` 42 → **45** + front 单测 2 条（`eq_rec_transports_at_type_level`、
> `eq_rec_family_yields_when_the_file_declares_it`）。让位/建议材料（`goals.rs`）
> 因为都按 `L1_FAMILIES` 走，**零改动**。
>
> ## ✅ B8 扩族补记（2026-09-19 同轮，层级算术 `u+1` 落地）
>
> 层级算术 `u+1` 落地（`docs/design/type-level-syntax.md` §5，parser + elab，
> 内核零改动）后，B8 从 3 条扩到 **5 条**、`PRELUDE_NAMES` 45 → **47**：
>
> - `Eq.mp`/`Eq.mpr` 从 **Type 0 实例**改成**宇宙多态**，签名与 Lean core 的
>   `def Eq.mp {α β : Sort u} (h : α = β) (a : α) : β` 逐字对齐
>   （`{u} (α β : Sort u) (h : @Eq.{u+1} (Sort u) α β)`）——**签名变更**：
>   裸写 `Eq.mp α β h` 仍按 u = 0 实例化，Type 0 的调用形状变成 `Eq.mp.{1} α β h`；
> - `cast` 装上（Lean core 里 `cast h a` 就是 `Eq.mp h a`；硬规则 3 的精神：
>   真 Lean 代码要能直接编）；
> - `Eq.ndrec` 装上（Lean core 的非依赖消去子，`def` 自 `Eq.rec`，不新增信任面）。
>
> 复现件扩到 **10 checked · 0 diagnostic**；白名单/课程/测试三件套见设计 §3。

## 0. 一句话

`lib/Logic.sokonanoda` 的 28 个名字（口径见下方"数法"）应当由 prelude 自带。本提案给出**逐字签名**、
**让位规则**（文件自己声明 ⇒ 该族整体让位）、**三件套**与**分阶段**；规范源文本（附录 A）
已用真内核判过：**exit 0 · 25 checked · 0 failed**（附录 B-1）。

**数法**：`course-stdlib.md` §2 / spike §2 记的是 **16 条**（引理口径），台账 L-01 列 14 个名字、
L-02 列 3 个（合 17）；**落到内核是 28 个顶层名字**——差额是族头/构造子/消去子
（`And`/`And.intro`/`And.left`/`And.right`、`Or`/`Or.inl`/`Or.inr`、`False.rec`）与 `Iff` 补丁
（`Iff.refl`/`Iff.symm`/`Iff.trans`）：它们没法单独存在。再加 2 个派生的 `Or.rec`/`And.rec`，
`PRELUDE_NAMES` 一共补 **30** 条。

**本提案的核心不是"抄 28 条进 prelude"，而是"让位规则"**：入门课（`course/` 单元①④⑤⑧⑨⑩⑪）
**故意**把这批东西当教学内容（学习者用 axiom 自建逻辑骨架、手写 `eq_symm_nat`/`Eq.symm`，
`REQUIREMENTS.md` §5 要求 Bare/自建两条路都在）。prelude 一旦无条件接管，这些画布会
`elab-duplicate-declaration`（实测，附录 B-2），44 份共享副本、两处 GOLDEN 与三个课程测试
（含中英镜像）都得跟着重写。让位规则让"教语言的课"与"用语言的课"各得其所。

## 1. 逐条清单

### 1.1 28 个名字与逐字建议签名

`{u}` 是唯一的宇宙 binder（G-14）；`axiom` 一律柯里化、不带 binder 参数表（G-13）；
`And`/`Or` 是**真归纳块**、构造子写**点号名**（G-02 下的可行解，见 §1.2）。

| # | Lean core 原名 | 本语言建议签名（逐字） | 形态 | 台账 |
|---|---|---|---|---|
| 1 | `True` | `axiom True : Prop` | axiom | L-01 |
| 2 | `True.intro` | `axiom True.intro : True` | axiom | L-01 |
| 3 | `False` | `axiom False : Prop` | axiom | L-01 |
| 4 | `False.rec` | `axiom False.rec : (C : Prop) -> False -> C` | axiom | L-01 |
| 5 | `False.elim` | `def False.elim (C : Prop) (h : False) : C := False.rec C h` | def | L-01 |
| 6 | `And` | `inductive And (a b : Prop) : Prop` | inductive | L-01 |
| 7 | `And.intro` | `ctor And.intro (ha : a) (hb : b) : And a b` | ctor | L-01 |
| 8 | `And.left` | `def And.left (a b : Prop) (h : And a b) : a := And.rec a b (fun (_ : And a b) => a) (fun (ha : a) (hb : b) => ha) h` | def | L-01 |
| 9 | `And.right` | `def And.right (a b : Prop) (h : And a b) : b := And.rec a b (fun (_ : And a b) => b) (fun (ha : a) (hb : b) => hb) h` | def | L-01 |
| 10 | `And.elim` | `def And.elim (a b c : Prop) (f : a -> b -> c) (h : And a b) : c := f (And.left a b h) (And.right a b h)` | def | L-01 |
| 11 | `Or` | `inductive Or (A B : Prop) : Prop` | inductive | L-01 |
| 12 | `Or.inl` | `ctor Or.inl (a : A) : Or A B` | ctor | L-01 |
| 13 | `Or.inr` | `ctor Or.inr (b : B) : Or A B` | ctor | L-01 |
| 14 | `Or.elim` | `def Or.elim (a b c : Prop) (f : a -> c) (g : b -> c) (h : Or a b) : c := Or.rec a b (fun (_ : Or a b) => c) f g h` | def | L-01 |
| 15 | `Not` | `def Not (A : Prop) : Prop := A -> False` | def | L-01 |
| 16 | `Not.intro` | `def Not.intro (A : Prop) (f : A -> False) : Not A := f` | def | L-01 |
| 17 | `Not.elim` | `def Not.elim (A C : Prop) (h : Not A) (a : A) : C := False.elim C (h a)` | def | L-01 |
| 18 | `absurd` | `def absurd (a b : Prop) (ha : a) (hna : Not a) : b := False.elim b (hna ha)` | def | L-01 |
| 19 | `Iff` | `def Iff (A B : Prop) : Prop := And (A -> B) (B -> A)` | def | L-01 |
| 20 | `Iff.intro` | `def Iff.intro (A B : Prop) (mp : A -> B) (mpr : B -> A) : Iff A B := And.intro (A -> B) (B -> A) mp mpr` | def | L-01 |
| 21 | `Iff.mp` | `def Iff.mp (A B : Prop) (h : Iff A B) : A -> B := And.left (A -> B) (B -> A) h` | def | L-01 |
| 22 | `Iff.mpr` | `def Iff.mpr (A B : Prop) (h : Iff A B) : B -> A := And.right (A -> B) (B -> A) h` | def | L-01 |
| 23 | `Iff.refl` | `def Iff.refl (A : Prop) : Iff A A := Iff.intro A A (fun (h : A) => h) (fun (h : A) => h)` | def | 补丁 |
| 24 | `Iff.symm` | `def Iff.symm (A B : Prop) (h : Iff A B) : Iff B A := Iff.intro B A (Iff.mpr A B h) (Iff.mp A B h)` | def | 补丁 |
| 25 | `Iff.trans` | `def Iff.trans (A B C : Prop) (h1 : Iff A B) (h2 : Iff B C) : Iff A C := Iff.intro A C (fun (a : A) => Iff.mp B C h2 (Iff.mp A B h1 a)) (fun (c : C) => Iff.mpr A B h1 (Iff.mpr B C h2 c))` | def | 补丁 |
| 26 | `Eq.symm` | `def Eq.symm {u} (α : Sort u) (a b : α) (h : Eq.{u} α a b) : Eq.{u} α b a := Eq.subst.{u} α (fun (x : α) => Eq.{u} α x a) a b h (Eq.refl.{u} α a)` | def | L-02 |
| 27 | `Eq.trans` | `def Eq.trans {u} (α : Sort u) (a b c : α) (h1 : Eq.{u} α a b) (h2 : Eq.{u} α b c) : Eq.{u} α a c := Eq.subst.{u} α (fun (x : α) => Eq.{u} α a x) b c h2 h1` | def | L-02 |
| 28 | `congrArg` | `def congrArg {u} (α : Sort u) (β : Sort u) (f : α -> β) (a b : α) (h : Eq.{u} α a b) : Eq.{u} β (f a) (f b) := Eq.subst.{u} α (fun (x : α) => Eq.{u} β (f a) (f x)) a b h (Eq.refl.{u} β (f a))` | def | L-02 |

`Or`/`And` 的归纳块另带派生 `Or.rec`/`And.rec`（`install_inductive_block` 自动生成，**不需要单独
写签名**）；**建议连同它们一起登记进 `PRELUDE_NAMES`**（与 `Nat.rec`/`Bool.rec` 同例），
这样补全列表与真实环境一致（共 30 条）。

### 1.2 三条硬约束下的定形决定（都有实测）

| 约束 | 决定 | 实测依据 |
|---|---|---|
| **G-13** axiom 不吃 binder 参数表 | 仅 4 条 axiom（`True`/`True.intro`/`False`/`False.rec`），全部无参数或纯箭头链 | `axiom Foo (a : Prop) : Prop` → parse `expected axiom type, found LParen`，exit 1 |
| **G-14** 一个声明只允许一个宇宙层级 binder | 只有 `Eq.symm`/`Eq.trans`/`congrArg` 用 `{u}`；`congrArg` **同层**（`β : Sort u`），跨宇宙留给 G-14 修好后的**一次签名波** | 试做时 `{u v}`/`{u} {v}` 都解析失败（台账 G-14） |
| **G-02** 构造子无命名空间 | `And`/`Or` 走**真归纳块**，构造子写字面点号名；**不写 axiom 族**（修正 `course-stdlib.md` §2 的"And 走 axiom 族"） | `ctor And.intro …` + `And.intro a b ha hb` → exit 0；而裸名 `ctor intro …` 后 `And.intro` 是 `unknown identifier` |
| 隐式实参**不自动插入**（未记台账，新发现） | 全部签名显式给参数（与现有 `Eq.subst`/课程库一致）；`{a : Prop}` 只能显式喂（`Imp a b ha hb` 可以，`Imp ha hb` 内核拒绝） | `axiom Imp : {a : Prop} -> {b : Prop} -> a -> b -> a` 后 `Imp ha hb` → `kernel-rejected`；`Imp a b ha hb` → exit 0 |

两条必须写进文档的**技术债**：
1. `congrArg` 同层 ⇒ 跨类型的 `congrArg`（像/原像结论）仍写不出来（G-14）；
2. 显式实参风格 ⇒ 填好的项**不能逐字**粘进官方 Lean（Lean core 的 `And.intro` 吃隐式参数）。
   这是既有的全局约定（`course/` 与 `lib/` 一直这么写），等隐式实参插入落地后统一成 Lean 写法——
   **那是一次签名变更，会再牵动一次 golden，届时另开提案**。

### 1.3 与入门课（`course/`）的冲突与迁移

现状（`course_shared.rs` 头部把这件事量化了）：`And` 公理 **24 份**、`Or` 块 **12 份**、显式 `Nat` 块 8 份，
散布在 6 个单元 × 中英 × 画布/解答。**让位规则下这些副本一个字都不用改**（§2）。

| 文件 | 现在自带 | 让位后 | 迁移动作 |
|---|---|---|---|
| `unit1` | `True/False/And/Or/Not` 全套公理 | **B1–B5 让位**，B6 因依赖 B3 连带让位；只剩 B7（Eq 三条） | **无**（正文加一句"这批名字 prelude 里已经有了；本单元你亲手造一遍"） |
| `unit2` | 无 L1 名字 | 拿到**完整 L1** | **本提案的课程用例**：加一个 `def` 演示 + 两条练习 hint 改成"两解对照"（§3.1） |
| `unit4` | `True/False/And/Or` | 同 unit1 | 无 |
| `unit5` | `theorem Eq.symm {u} := sorry`（**练习**） | 只让位 **B7**（Eq 三条） | 无（练习照旧手写 `Eq.subst`；这正是"教构造"的单元） |
| `unit8` | `True/And` | B1/B3 让位 | 无 |
| `unit9/10/11` | `And`/`Or`(`inductive`)/`Iff` | B3/B4/B6 让位 | 无 |
| `playground.sokonanoda` | 同上第 84–123 行 | B1–B5 让位 | **无（不变）**，gate 锚点仍绿；若想把画布升级成"用 prelude"的示范，是**第二轮的可选改动**（会改事件计数） |

**两条练习的真实冲突**：`eq_symm_nat`/`eq_trans_nat`（`unit2`）在 L1 落地后有了**一行的解**
（`fun a b h => Eq.symm a b h`）。内核无法区分"用 prelude 抄近路"与"自己设计谓词"，
所以处理方式是**把话说清**而不是假装：hint 写成"本题要练的是 `Eq.subst` 的谓词设计；
prelude 的 `Eq.symm` 是它的现成版，两种都写一遍对照"。题**不删**（`docs/teaching-session.md` U2·3 记着
它是"本场最深的一步"）。

### 1.4 与卷 I 课程标准库的冲突

`courses/set-theory/`：**32 个文件** `import lib.Logic`，**74 行**含 `inl`/`inr`
（按行计，含注释与 `-- soko:hint`；其中项位用法会在 P4 改成点号名）。
- `lib/Logic.sokonanoda` L1 落地后退化成**只有注释的空壳模块**（实测：空模块可被 `import`，闭环 exit 0），
  32 个 `import lib.Logic` **不用动**；等课程侧愿意时再删文件、删 import。
- 点号构造子与裸名的差异是**真冲突**：`ctor Or.inl` 定形后，裸**项** `inl`/`inr` 变成
  `unknown identifier`（实测）；裸**模式** `| inl a =>` 仍被接受（实测）。
  所以 74 处里的项位要改成 `Or.inl`/`Or.inr`（模式可不动，为一致性建议一起改）。
- 这件事属 §6 的 P4（课程仓跟随），不阻塞语言侧的 P1–P3。

## 2. 让位规则（本提案的核心机制）

### 2.1 为什么必须让位

今天 prelude 的让位有三条先例：`inductive Nat`/`inductive Bool` 块在文件里出现 ⇒ 整块让位
（`check/mod.rs:480-498`）；`Eq`/`Eq.refl`/`Eq.subst` 任一被文件占用 ⇒ Eq 三名整体让位
（`install_eq_prelude`，all-or-nothing）。L1 沿用同一条哲学，**谁声明谁拥有**。

无条件接管会直接砸掉入门课：实测 `axiom Nat : Type`（prelude 已装 Nat）得到
`elab-duplicate-declaration: duplicate declaration Nat`——`unit1` 自己那 12 条声明
（11 条公理 + `def Not`）会得到同一结果。

### 2.2 分族让位 + 依赖闭包

**粒度 = 族**（不是单名）：一个族要么整族来自 prelude，要么整族来自文件——避免"prelude 的 `And`
+ 文件的 `And.left`"这种静默不一致。族之间按依赖做**闭包让位**：

| 族 | 名字 | 依赖（被让位时本族也必须让位） |
|---|---|---|
| B1 真伪 | `True`, `True.intro` | — |
| B2 假与爆炸 | `False`, `False.rec`, `False.elim` | — |
| B3 且 | `And`, `And.intro`, `And.left`, `And.right`, `And.elim` | — |
| B4 或 | `Or`, `Or.inl`, `Or.inr`, `Or.elim` | — |
| B5 非 | `Not`, `Not.intro`, `Not.elim`, `absurd` | **B2** |
| B6 当且仅当 | `Iff`, `Iff.intro`, `Iff.mp`, `Iff.mpr`, `Iff.refl`, `Iff.symm`, `Iff.trans` | **B3** |
| B7 Eq 引理 | `Eq.symm`, `Eq.trans`, `congrArg` | **Eq prelude**（`Eq` 被占用时 `install_eq_prelude` 整体不装） |

触发集合 `taken` **沿用今天已有的口径**：整个闭包的顶层名字并集（`check/mod.rs:500-503`，
设计 §4.6"prelude 是整个编译单元的属性"）。命中任一名 ⇒ 该族 + 依赖它的族一起不装。

**为什么 B5 依赖 B2、B6 依赖 B3、B7 依赖 Eq**：它们的定义体直接引用被依赖的名字
（`Not.elim` 用 `False.elim`、`Iff.mp` 用 `And.left`、`Eq.symm` 用 `Eq.subst`）——
让位必须**依赖闭包**，否则 prelude 源文本自己就 elaborate 不过（`unknown identifier`）。

### 2.3 两个实测边界（落地时要一起修）

1. **`taken` 漏构造子/递归子**：现在是 `user_top_level_names`（只看 `Command::*{name}`），
   不含 `ctor`/`rec`；而 `top_level_def_spans` 含。今天无害，L1 之后会出现
   "文件在别的归纳块里写了 `ctor Or.inl` ⇒ 与 prelude 的 `Or.inl` 撞车"。
   **改法**：`taken` 换成 `top_level_def_spans_over(units)` 的键集（一个函数替换 + 一条单测）。
2. **`check_name_collisions` 的豁免面**：它按 `PRELUDE_NAMES` 跳过检查（`project/mod.rs:333`）。
   L1 名字一旦进 `PRELUDE_NAMES`，**两个模块各自声明 `True`** 就不再报友好的
   `import-name-collision`，退化成内核裸错（Eq 今天已有同样的洞）。
   **改法**：拆成两个常量——`PRELUDE_NAMES`（补全/材料，含 L1）与
   `PRELUDE_NEVER_YIELDS`（只含 Nat/Bool 家族，给碰撞检查用）。

## 3. 三件套落地计划（硬规则 3）

### 3.1 课程用例（"课程"这一件）

**单元② 等式与 rfl**（`course/unit2-equality-rfl.sokonanoda` + `course/en/...` + 两份 solution）：
1. 新增一个**已证明的**演示（`def eq_symm_demo … := Eq.symm …`，注释里点明"prelude 自带"）；
2. 正文加一段"prelude 现在自带 `Eq.symm`/`Eq.trans`/`congrArg` 与整套逻辑词汇"；
3. `eq_symm_nat`/`eq_trans_nat` 的 `-- soko:hint` 改成"两解对照"（§1.3）。

为什么放单元②：它是**第一个**不声明任何 L1 名字的单元（`grep` 实测：unit1 自带全套、
unit2 一个都不带；unit3/6/7 同样不带，但都在它之后）——只有这样的文件才拿得到完整 L1，
单元①/④/⑤⑧⑨⑩⑪ 因让位而**不能**当示范。
**顺带**：`unit1` 的正文加一句"这些骨架 prelude 里已经有了"，只改文本不改计数。

### 3.2 三层测试

| 层 | 文件 | 用例（名字即契约） |
|---|---|---|
| **front 单测** | `crates/front/src/compile/tests.rs` | `l1_prelude_is_available_in_full_mode`（`And.intro`/`Or.elim`/`Iff.mp`/`absurd` 各用一次，0 errors）；`l1_or_is_a_real_inductive_for_match`（`match h with \| Or.inl a => …` 与裸模式 `\| inl a =>` 都过——**钉住 `InductiveTable` 注册**）；`l1_family_yield_is_dependency_closed`（**方向按 §2.2**：文件声明 `And` ⇒ 依赖它的 `Iff.*` 全不在，而 `Or.elim`/`Eq.symm` 仍在；反向对照：只声明 `Iff` 不让位 `And`）；`l1_yield_needs_the_whole_family`（只声明 `And.left` ⇒ `And.intro` 也不在）；`l1_yield_is_closure_wide`（project 模式：依赖模块声明 `True` ⇒ 入口也没有 prelude `True`）；`bare_mode_has_no_l1`；`prelude_names_match_installs`（列表 ↔ 实际安装防漂移）；更新既有 `eq_prelude_symm_derivable_from_subst` 旁加一条 `eq_symm_is_installed` |
| **CLI e2e** | `crates/cli/tests/cli.rs` | Full：一个用 L1 名字的文件 `--json` exit 0 + `decl.checked` 计数；Bare（`--bare` 与 `-- sokonanoda:prelude none` 各一次）下同文件 exit 1 且错误码 `elab-unknown-identifier`；`query check` 的 `counts.decl_checked` 与事件流一致 |
| **课程 golden** | `crates/cli/tests/course.rs`、`course_status.rs`、`course_shared.rs` | 见 §4.2；**`course_shared.rs` 必须原样全绿**——它 44 份副本的一致性就是"让位"生效的证据 |

### 3.3 白名单

- **parser 白名单：不动。** L1 不引入任何新语法/关键字（`inductive`/`ctor`/`def`/`axiom` 都在），
  `docs/architecture.md` §4.1 的字面量白名单零改动；
- **prelude 白名单：`crates/front/src/compile/prelude.rs` 的 `PRELUDE_NAMES`** 补 **30** 个名字
  （28 条 + 派生的 `And.rec`/`Or.rec`；现在 12 条，落地后 `assert_eq!(PRELUDE_NAMES.len(), 42)` 之类的守卫便于 review）；
- 新增 `PRELUDE_L1_SRC`（规范源文本，附录 A）与 `PRELUDE_L1_BLOCKS`（§2.2 的族表）；
- 新增 `PRELUDE_NEVER_YIELDS`（§2.3-2）；
- **不做** `-- sokonanoda:prelude core|full` 档案指令（那是新语法，要它自己的三件套；见 §5）。

## 4. 影响面

### 4.1 代码

| 文件 | 改动 | 风险 |
|---|---|---|
| `crates/front/src/compile/prelude.rs` | `PRELUDE_L1_SRC`/`PRELUDE_L1_BLOCKS`/`PRELUDE_NEVER_YIELDS`；把 `install_eq_prelude` 泛化成 `install_prelude_block`（现在遇到非 `Axiom` 命令在 `prelude.rs:115-121` 直接 `panic!`）；`PRELUDE_NAMES` 补 30 条 | 中：def/inductive 要走 `build_def`/`install_inductive_block`（与用户声明同一条 elaborator），不能只走 `build_axiom` |
| `crates/front/src/compile/check/mod.rs` | Full 分支里先算一次 `taken`（改用 `top_level_def_spans_over`），再按族顺序装 L1、装 Eq | 低；`taken` 口径变化要配单测 |
| `crates/front/src/compile/goals.rs` | `GoalTemplates::new_for` 现在只吃 `PRELUDE_EQ_SRC`；要按**同样的让位规则**吃 L1 源文本，否则 `refine`/`intro` 建议里没有 `And.intro`/`Or.inl` | 中：`new_for` 只拿到**单文件**，而让位是**闭包级**——需把闭包级 `taken` 传进来，否则项目模式下建议与实际环境会不一致 |
| `crates/front/src/project/mod.rs` | `check_name_collisions` 改用 `PRELUDE_NEVER_YIELDS`；`PRELUDE_OWNED` **不加** L1 名字（L1 按族合法让位，不是冲突） | 低 |
| `crates/kernel` | **零改动**（硬规则 1） | — |

### 4.2 两处 GOLDEN 与会变的测试

| 断言 | 现状 | L1 落地后 |
|---|---|---|
| `course.rs::GOLDEN` 11 行 | `unit2 = (2,5,2)` | **`unit2 = (3,5,2)`**（§3.1 的 `def` 演示 +1；其余 10 行不变） |
| `course_status.rs::GOLDEN` + summary | `unit2 = (2,5,0,2)`；`checked=85`、`open=65` | `unit2 = (3,5,0,2)`；`checked=**86**`、`open=65`；human view 的 `"65"` 断言不变 |
| `course.rs::en_mirrors_match_chinese_event_counts` / `en_solutions_…` | 中英事件计数相同 | 同步改 `course/en/` 与两份 solution（中英 + 解答共 4 份文件） |
| `course.rs::solution_covers_every_canvas_exercise` | 每个 `exercise.open` 在 solution 里有同名声明 | 新演示是 `def`（非练习），不加练习 ⇒ 断言不变 |
| `course_shared.rs`（6 个测试） | 44 份副本逐字一致 + 共享子项目可判卷 | **一字不改，必须仍绿** |
| `crates/front/.../tests.rs::eq_prelude_symm_derivable_from_subst` | 断言 `Eq.symm` 可由 `Eq.subst` 推出 | 仍绿（它只证明可导出）；新增"已安装"断言 |
| `crates/cli/src/env/mod.rs` 的 gate 锚点 | 用内嵌编译器编译 `playground.sokonanoda` 要求 exit 0 | **不变**（让位 ⇒ 画布语义与计数都不动） |

### 4.3 文档要同轮改

- `docs/architecture.md`：§5.4「内置 prelude」补 L1 小节（族表 + 让位规则 + 依赖闭包）；
  §4.1 白名单行注明"prelude 名字不属于语法白名单，parser 零改动"；
- `docs/design/course-stdlib.md`：§2 指向本提案，并把 "And 走 axiom 族（因 G-02）" 更正为
  "**真归纳块 + 点号构造子**"；§4 的 G-02 行注明 L1 已绕过（源语法仍受影响）；
- `docs/design/teaching-project.md` P-C3 状态、`docs/teaching-session.md`（U2·3/U2·4 的"配方"
  加一句"prelude 现在有现成版"）、`docs/TESTING.md`（prelude 行补 L1 的测试映射）、
  `docs/HANDOVER.md`、`STATUS.md`（收尾义务）；
- **skills 与 VS Code（AGENTS 的同步义务）**：`skills/sokonanoda-teacher/SKILL.md` 的 prelude 小节
  （现列 `Nat`/`Bool`/`Eq`）要补 L1 词汇与让位规则、`references/curriculum.md` 同步；
  `editor/vscode/CHANGELOG.md` 记一行（补全列表多了 30 个名字＝用户可见改动），
  `README.md` 的 "Completions (… prelude names)" 描述仍然成立、可不动；版本号按惯例 bump。

## 5. 风险与不做的事

**风险**
1. **静默让位的可发现性**：文件写了 `And`，学习者会以为"`And.elim` 也在"，得到的却是
   `unknown identifier`。本轮只做文档；**不做**新 warning（新警告是用户可见行为，要三件套）。
2. **闭包级连锁**：项目模式下一个依赖模块声明 `True`，全闭包丢掉 B1。与今天 Eq 同病，
   文档写清；`PRELUDE_OWNED` 保持不含 L1 名字（不把它当错误）。
3. **签名二次迁移**：隐式实参插入落地后，L1 签名要改成 Lean 形状（`And.elim f h`），
   会再牵动一次课程与 golden。现在**不做**隐式，避免两次定形。
4. **`Or.inl` 与 G-02 的最终结论**：本提案按"点号名"定形；若 G-02 将来给"裸名保留为别名"，
   prelude 的正式名仍是 `Or.inl`（别名只是源语法糖），不构成再迁移。
5. **`#check`/hover/补全对 prelude 名**：prelude 是受信任安装、没有 `DeclState`，
   与今天 `Nat`/`Eq` 同状（补全靠 `PRELUDE_NAMES` 静态列表，hover 不保证）。属既有边界，本轮不扩。

**不做**
- 不装 `Exists`/`Prod`/`Subtype`/`Set`（L2 = 课程标准库的活，`course-stdlib.md` §3）；
- ~~不装 `Eq.mp`/`Eq.mpr`/`Eq.rec`/`cast`（**L-03**：`Eq.subst` 的 motive 只能落 Prop，写不出来，
  要改签名=设计先行）~~ —— **已作废（2026-09-19，B8）**：`Eq.rec`/`Eq.ndrec`/`Eq.mp`/`Eq.mpr`/`cast`
  已作为 **B8 族**装上（见文首两条 B8 补记与 `docs/design/eq-type-level-rewriting.md`）；
  `Eq.mp`/`Eq.mpr`/`cast` 是**宇宙多态**（层级算术 `u+1` 落地后，签名与 Lean core 逐字对齐），
- 不做 `notation`/infix（G-04）、不做 `namespace`/`open`（G-05）、不做隐式实参；
- 不动内核、不动 tactic 白名单、不动 `by` 块；
- 不新增 prelude 档案指令（`core`/`full`）——**备选方案 B**：如果将来"同一门课既要教自建、
  又要在别的单元用现成"的分歧变大，它比让位更显式，但代价是新语法（`PreludeMode` 第三个变体 +
  缓存形状 + CLI/LSP 指令解析）与它自己的三件套。

## 6. 分阶段（能独立过 gate 的最小步）

| 阶段 | 内容 | 独立验收 |
|---|---|---|
| **P1** front 安装 + 让位 | `PRELUDE_L1_SRC`/`BLOCKS` + `install_prelude_block`（def/inductive 都支持）+ `taken` 换成 `top_level_def_spans_over` + `PRELUDE_NEVER_YIELDS`；§3.2 的 front 单测 | `cargo test -p sokonanoda-front`；`playground.sokonanoda` 仍 exit 0 |
| **P2** CLI e2e + 白名单/文档 | `crates/cli/tests/cli.rs` 两条 e2e；`PRELUDE_NAMES` 补名字；`docs/architecture.md` §5.4 + `course-stdlib.md` §2 | `scripts/soko gate`（fmt/clippy/test + 锚点） |
| **P3** 课程用例 + 两处 GOLDEN | 单元② 中英画布与两份解答（§3.1）；同步 `course.rs`/`course_status.rs` 的 GOLDEN 与 summary（§4.2 的预测值）；`goals.rs` 模板纳入 L1 | `scripts/soko gate`；`course_shared.rs` 未改而全绿 |
| **P4 ✅**（2026-09-19）课程仓跟随（卷 I） | `lib/Logic.sokonanoda` → 只有注释的空壳（0 条声明）；课程侧 **65 处项位**裸名 `inl`/`inr` 改点号名（+27 行注释）；台账 L-01/L-02 无需再动；`course-stdlib.md` §2/§3/§4/§5 更新 | `python3 courses/set-theory/tools/check.py` = **exit 0 · 36 目标 · 329 checked · 99 open · 0 判负**；`--selftest` 也绿 |

P1 与 P2 可以合并成一轮（同一条 gate）；P3 必须单独一轮（golden 语义变更）；P4 可以更晚，
只要 prelude 先落地——**课程侧的"暴力"要到 P4 才真正消除**，这一点要在 P3 的 STATUS 里写明。

## 附录 A：L1 规范源文本（将进 `PRELUDE_L1_SRC`）

即 §1.1 的 28 条 + `Or`/`And` 的两个 `end`，逐字见 `courses/set-theory/lib/Logic.sokonanoda`
的对应行（去掉注释与 L1 之外的说明），差异只有两处：**`And` 由 axiom 族改成真归纳块**、
**`Or` 的构造子由裸名 `inl`/`inr` 改成 `Or.inl`/`Or.inr`**。文件顺序必须满足依赖：
B1/B2 → B3 → B4 → B5 → B6 → B7（`And.elim` 在 `And.left/right` 之后、`Iff` 在 `And` 之后）。

## 附录 B：实测证据（都是本轮真跑的）

1. **规范文本可判**：把 §1.1 的 25 条声明（不含 ctor）+ 两个归纳块写成自包含文件，
   `scripts/soko` **exit 0**（`checked declaration` ×25，0 error）。
2. **无条件接管会砸课程**：Full 模式下 `axiom Nat : Type` → `elab-duplicate-declaration: duplicate declaration Nat`。
3. **点号构造子可行、裸名不行**：`ctor And.intro …` 后 `And.intro a b ha hb` exit 0；
   `ctor intro …` 后 `And.intro` → `unknown identifier`（G-02 的精确边界）。
4. **`match` 两种模式都认**：对点号构造子的 `Or`，`| Or.inl a =>` 与 `| inl a =>` 都 exit 0。
5. **裸项名不再存在**：点号构造子下 `inl a a ha` → `unknown identifier \`inl\``（课程迁移的依据）。
6. **隐式实参不插入**：`axiom Imp : {a : Prop} -> {b : Prop} -> a -> b -> a` 后
   `Imp ha hb` → `kernel-rejected`；`Imp a b ha hb` → exit 0。
7. **空模块可 import**：注释-only 的 `Logic.sokonanoda` 被入口 `import`，
   `scripts/soko <入口>` exit 0（P4 的"退化成空壳"路径可行）。
8. **入门课的让位面**（`grep` 全量清点 + 真判卷）：中英各 11 个单元与 11 份解答里，
   **28 个文件**自带至少一个 L1 名字——`unit1/4/8/9/10/11` 的中英画布与解答自带
   `True/False/And/Or/Not/Iff` 族；`unit5` 的中英画布与解答自带 `theorem Eq.symm … := sorry`，
   实判 `checked 0 · open 6`，`Eq.symm` 就在 `exercise.open` 名单里（所以它必须落在 B7 的让位范围，
   否则这条教学练习会被 prelude 变成重复声明）。

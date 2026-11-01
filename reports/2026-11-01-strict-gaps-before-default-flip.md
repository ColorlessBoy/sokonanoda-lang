# 前置条件报告：**翻默认档（S7）在三条缺口修完之前不可行**

> 触发：P2-12 **S7 翻默认档**（实施单 `reports/2026-11-01-p212-s7-flip-default-spec.md`）。
> 实测走完"翻默认 + 全量判据"后，主会话 2026-11-01 拍板 **S7 不并入**；本文件是三处红线的取证。
> 分支 `strict/2026-10-29-kernel-app`，**没有 `git commit` / `git push`**；
> S7 的全部改动（30 个文件）已 `git checkout --` 还原 —— 还原时 HEAD = `236db5ad`；
> 之后主会话落了 `2e28a9f1`（D1：不翻默认档，只动 `docs/`/`reports/`）⇒ 现 HEAD = `2e28a9f1`，
> 工作区**只剩本报告一个未跟踪文件**（`git status --short` ⇒ `?? reports/2026-11-01-strict-gaps-before-default-flip.md`）。
> 内核 **0 行**、判定逻辑 **0 行**、课程文案 **0 行**、`check.py` **0 行**（本报告只读、不改）。

## §0 结论

**这三条是翻默认的前置条件；在它们修完之前，strict 只能是显式档。**
strict 作为"不再需要任何环境变量的**显式**档"上线（S6 + 发版面已完成），**翻默认留到三处分别修完之后**：

| # | 缺口 | 缺口的性质 | 语料命中 | 为什么会毁掉"翻默认" |
|---|---|---|---|---|
| 1 | **默认档常红课程门禁** `check.py`（G1+G4 ×2） | 口径/清单（非判定） | **2 个 orphan 解答**（不在 `course.json`） | 课程门禁在 `scripts/soko gate` 与 CI 里 ⇒ 翻默认后**每次推送都红** |
| 2 | strict 拒 `eqsubst` 的 **old-style 前缀重建** | front 侧重建面（`explicit_spine: false`） | 语料 **0**（但**教学形状**） | `apply Eq.subst …` 是学生最常用的等式代换写法，翻默认把"能过"变"过不了" |
| 3 | strict 把**内部元变量名**漏到用户面前（`unknown identifier `\0soko_m2``） | 引擎出口不变量（元变量不该进项）+ 报错卫生 | 语料 **0**（G-48 形状只在注释里） | 正是 §9.39 要消灭的那类"把内部名字印给学习者"的 bug，且**判定结果错**（该绿的红） |

一句话取舍：**翻默认 = 把三条既有的 strict 侧缺口一次性推给所有用户**。这不是"上线"，是"放大缺陷"。

---

## §1 发现 1：默认档会让 `check.py`（课程门禁）**常红**

### 1.1 最小复现

不是一段代码，而是**两条命令**（不需要任何 env / 旗标）：

```bash
python3 courses/set-theory/tools/check.py                 # 默认档（= S7 翻完之后是 strict）
SOKO_STRICT_AT=1 python3 courses/set-theory/tools/check.py   # HEAD 上等价的复现方式
```

判红的两个目标（两档差别只在默认值，所以这条与发现 2/3 无关）：

```text
courses/set-theory/units/solutions/I.4/unit12-solution.sokonanoda   # 第 171 行起 theorem flawed_equalities_refuted
courses/set-theory/units/solutions/I.8/unit19-solution.sokonanoda   # 第  59 行起 theorem le_empty
```

### 1.2 两档实测读数（HEAD 二进制 `097996f683cc079b`）

| 目标 | 档 | rc | 诊断码 | span | 正文关键句 |
|---|---|---|---|---|---|
| `unit12-solution` | `--dialect loose` | **0** | — | — | — |
| `unit12-solution` | `--dialect strict` | **1** | `elab-implicit-argument-unsolved` | **175:8** | `` `Ne` 的签名 `(α : Sort u) -> (a : α) -> (b : α) -> Prop` 里，前 1 个形参是隐式…`α` 由实参和期望类型都定不下来 `` |
| `unit19-solution` | `--dialect loose` | **0** | — | — | — |
| `unit19-solution` | `--dialect strict` | **1** | `elab-implicit-argument-unsolved` | **63:45** | `` `Set.notMem_empty` … 你写了 2 个位置实参，多出来的落不到任何形参上 `` |
| 同上（级联） | `--dialect strict` | — | `elab-unknown-identifier` | **66:52** | `` unknown identifier `le_empty` ``（级联，不是第二个根因） |
| 整门课 `check.py` | `SOKO_STRICT_AT=1` | **1** | — | — | `258 个目标 —— 2188 checked · 914 open · **2 个被判负**`；`✗ 解答 unit12` / `✗ 解答 unit19`，各自 **G1**（`grade` 退出码非 0）+ **G4**（解答没有覆盖画布的练习 `flawed_equalities_refuted` / `le_empty`、`subset_of_le_empty`） |

对照：同一门课在 loose 下是 `rejected 0 · targets 258 · checked 2191`（S6 读数 §4 的实测；
今天用 HEAD 二进制**逐文件**复读两个目标，`--dialect loose` 都是 rc=0）—— strict 少 **3** 个 checked =
`unit12` 1 个 + `unit19` 2 个。

### 1.3 根因

**不是判定 bug，是门禁口径**：`courses/set-theory/tools/check.py` 的 targets 由
`units/solutions/**/*-solution.sokonanoda` **递归收集**（258 个），其中**包含两个不在
`course.json` 的 orphan** —— 它们的画布 `unit12-synthesis` / `unit19-cardinal-le` 早已改名，
这两份解答留在旧路径上（S6 读数 §0 已定性）。
⇒ "112 个判卷目标不受影响"（S7 实施单 §4）**成立**，但 `check.py` 看的是 **258 个**，所以门禁红。

### 1.4 影响面

- 语料：**2 个文件**（都是 orphan，**不在 `course.json`**；`units/solutions/` 下其余 112 个目标零变化）。
- 门禁：`python3 courses/set-theory/tools/check.py` 是 `scripts/soko gate`、CI 的 `gates-course` 与
  `docs/design/course-gate-in-ci.md` 的判据 ⇒ **翻默认后课程门禁常红**（不是偶发）。

### 1.5 建议的修法方向（择一，都属"另一刀"）

1. **清单收口**：把两个 orphan 移进 `docs/archive/` 或补进 `course.json`（改名后的画布），
   让 "`check.py` 的 targets" 与 "课程的真实目标集" 一份口径；
2. **门禁口径**：`check.py` 只收 `course.json` 里出现的画布对应的解答（**需先拍板**：
   这会把"orphan 也要能编"这条隐含保证去掉）；
3. **不接受**：让 `check.py` 在 strict 下放宽 G1/G4 —— 那是把"能过"变成"看起来能过" ✗。

⚠ **本报告不改任何一项**（课程文案与门禁都不在 S7 的边界内）。

---

## §2 发现 2：strict 拒 `eqsubst` 的 **old-style 前缀重建**（把**引擎装配**的应用当成"用户写的"）

### 2.1 最小复现（零 import，A）

```sokonanoda
def Set (α : Type) : Type := α → Prop
def mem {α : Type} (a : α) (A : Set α) : Prop := A a
def empty {α : Type} : Set α := fun (_ : α) => False
infix:50 " ∈ " => mem

theorem t (α : Type) (A : Set α) (x : α) (hx : x ∈ A) (h : A = empty) : x ∈ empty := by
  apply Eq.subst _ hx
  exact h
```

最小复现（B）—— 用户 2026-10-15 亲报的原案形状（课程 `units/I.1` 练习 4；夹具原文见
`crates/front/src/compile/tests/eqsubst_predicate.rs` 的 `REPRO`）：

```sokonanoda
def Set (α : Type) : Type := α → Prop
def mem {α : Type} (a : α) (A : Set α) : Prop := A a
def subset {α : Type} (A B : Set α) : Prop := ∀ (x : α), mem x A → mem x B
def singleton {α : Type} (a : α) : Set α := fun (x : α) => x = a
infix:50 " ∈ " => mem

theorem t (α : Type) (A : Set α) (a : α) (h : a ∈ A) : subset (singleton a) A := by
  intro x hx
  unfold singleton mem at hx
  apply Eq.subst (Eq.symm hx) h
```

### 2.2 两档实测读数（HEAD 二进制 `097996f683cc079b`）

| 复现件 | 档 | rc | 诊断（按序） | `t` 是否 checked |
|---|---|---|---|---|
| A（`apply Eq.subst _ hx`） | loose | **0** | 无 | ✅ |
| A | strict | **1** | `elab-implicit-argument-unsolved` **7:3–7:22** + 级联 `elab-tactic-failed` **8:3–8:10** | ❌（不出现 `decl.checked t`） |
| B（`apply Eq.subst (Eq.symm hx) h`） | loose | **0** | 无 | ✅ |
| B | strict | **1** | `elab-implicit-argument-unsolved` **10:3–10:32** | ❌ |

**决定性证据：诊断自相矛盾地宣称"你写了 6 个位置实参"**，而源码里只写了 **2 个**；
更直接的是它给出的可照抄出路里带着**引擎自己合成的** motive 名（`_soko_eq<offset>`）：

```text
你写了 6 个位置实参，多出来的落不到任何形参上。
出路 ①（全显式：连隐式位也逐位写下来）⇒ 在函数名前加 `@`：
  `@Eq.subst α (fun (_soko_eq407 : α) => _soko_eq407 ∈ A) a x (Eq.symm hx) h`
出路 ②（让推断补：删掉喂给隐式位的那些实参，只留显式形参对应的）⇒ 按显式形参顺序只写 2 个实参。
```

`_soko_eq<offset>` 这个 binder 名**只可能来自 `crates/front/src/compile/eqsubst.rs`** ——
它在那里拼出来两次：`eqsubst.rs:178`（`rewrite_goal_motive`，洞/谓词合成）与 `eqsubst.rs:365`
（`synthesize`，old-style 前缀）。用户不可能写出这种名字
⇒ 这条拒绝的输入是**引擎装配出来的应用**，不是用户写的那 2 个实参。

### 2.3 根因定位（已到行）

`crates/front/src/compile/eqsubst.rs` 有 **三处**把重建出来的 `Expr::App` 写死成
`explicit_spine: false`（= 「正常形式」）：

| 行 | 所在 | 作用 |
|---|---|---|
| **207** | `rewrite_goal_motive`（`fn` 起于 `:164`） | 洞 / 谓词合成的五～六元前缀（复现件 A） |
| **296** | `rewrite_explicit_equality` 的 `Eq.symm` 包装（`fn` 起于 `:256`） | 把等号翻成 `Eq.symm x` |
| **398** | `fn prefix`（`:392`，注释就叫「old-style 前缀」） | `Eq.subst (T) (fun s => C[s]) a b <等号> <premise>`（复现件 B） |

严格档的绑定策略（`REQUIREMENTS.md` §9.40 的口径：正常形式的**位置实参只按显式形参逐位绑定**）
看到的是"一条 6 个位置实参、写在正常形式里的脊" ⇒ 判负。
**同一个不变量在别处是守住的**：`crates/front/src/spine.rs:1675` 的 `substitute` 明确写着
"⚠ `explicit_spine` **必须原样带过去** ✗（a17 实测…）" —— 也就是说 a17 修的是**代换**这一路，
`eqsubst.rs` 这三处**自己新建**的 `App` 没跟着修。

⚠ 与 S6 的关系：a17（2026-10-19）声称"**两档同判**"，但那是在 **S6 之前**（strict 还走老 app 路径 + 回落）成立的；
S6 让 strict = **内核路径 + 不回落**之后，这三处装配面在 strict 下就再也没有绿过 —— 只是**默认档还是 loose**，
所以没人量到。

### 2.4 影响面

- **语料命中 0**：`git ls-files 'courses/**/*.sokonanoda'` 里 `apply Eq.subst (` **0 个文件**、
  `Eq.subst _` **0 个文件**；`apply Eq.subst` 只有 `units/I.1/unit02-subsets-empty.sokonanoda:78`
  的 `apply Eq.subst h`（**1** 个显式实参、元素等式，**strict 下照旧 rc=0**，实测）。
  课程解答一律用 `@Eq.subst`（38 个文件）⇒ 课程不受影响。
- **但是教学形状**：`apply Eq.subst h` 是 unit02 练习 6 的第一问写法，`apply Eq.subst _ hx` 是
  unit02 现场（用户 2026-10-22 亲报）的写法 —— 学生把 `h` 写成 `_`、或给谓词型等式（Set / 函数 / `⊆`）
  做代换时就会撞上；撞上时**报错还是误导性的**（"你写了 6 个位置实参"，可用户写的是 2 个）。
- 测试面：**10 条 front 单测**直接红（`compile::eqsubst_predicate_tests::*` **8** 条 +
  `compile::tests::apply_argument_hole_becomes_a_subgoal` +
  `compile::tests::eq_symm_and_eq_subst_infer_their_implicit_arguments`），
  外加 `crates/cli/tests/apply_eqsubst.rs::apply_eqsubst_leaves_negation_goal`（课程现场形状）。

### 2.5 建议的修法方向

1. **首选**：`eqsubst.rs` 三处把重建脊标成 `explicit_spine: true`（与 `expr.rs:259` 的
   `@` 形态构造器同一口径）—— 那正是"引擎装配的应用不是用户写的"这条不变量；
   反向判据现成：§2.1 两件在 strict 下必须 rc=0，且 loose 逐字节不变；
2. 顺带（报错卫生）：`@Eq.subst …` 出路里出现 `_soko_eq<offset>` / `sorry` 这类内部名，
   应当像 `kernel_app_report.rs:223` 的 `sanitize` 那样清洗（反向判据 `kernel_names_never_leak`，`kernel_app_report.rs:385`）—— 否则「可照抄」是假的（用户抄不到 `_soko_eq407`）；
3. 落到守卫：给 `eqsubst` 的三条路径各补一条 **strict 臂**的 front 判据（见 §4）。

---

## §3 发现 3：strict 把**内部元变量名**漏到用户面前（`unknown identifier `\0soko_m2``）

### 3.1 最小复现（零 import，G-48 形状）

```sokonanoda
def Set (α : Type) : Type := α -> Prop
def Set.empty (α : Type) : Set α := fun (_ : α) => False
def Set.singleton (α : Type) (a : α) : Set α := fun (x : α) => x = a
def Set.Equiv {α β : Type} (A : Set α) (B : Set β) : Prop :=
  (forall (x : α), A x -> A x) ∧ (forall (y : β), B y -> B y)
notation "∅" => Set.empty
infix:50 " ≈ " => Set.Equiv

theorem n3 (β : Type) (b : β) : ¬ (∅ ≈ {b}) := by sorry
```

（这就是 `crates/cli/tests/metavar_engine.rs` 的 `N3_g48_both_nullary` 夹具；把它的 `PRELUDE` 里四条
没用到的定义/记法（`Set.mem` / `Set.image` / `Set.subset` 与 `∈` / `''` / `⊆`）删掉后**仍然复现**
⇒ 上面这一版就是能复现的最小版 ✓）

### 3.2 两档实测读数（HEAD 二进制 `097996f683cc079b`）

| 档 | rc | 诊断 | span | 正文（原样，`\0` = NUL） |
|---|---|---|---|---|
| `--dialect loose` | **0** | 无；`Set`/`Set.empty`/`Set.singleton`/`Set.Equiv`/`n3` 全部 checked | — | — |
| `--dialect strict` | **1** | **`elab-unknown-identifier`** | **0:0**（没有源码位置） | ``unknown identifier `\0soko_m2` `` |

两个独立的坏处：

1. **判定错**：M1 的硬红线是「**引擎不许丢解**」，`metavar_engine.rs:132` 的 `why` 原文写着
   "窄版靠「同形已解兄弟」，引擎靠 defaulting ⇒ **两档都必须绿**" —— strict 档下它**红**；
2. **报错卫生**：用户看到的是 `` unknown identifier `\0soko_m2` ``（NUL 在终端里渲染成空格），
   **既没有位置（span 0:0）、也没有人话**。这正是 `REQUIREMENTS.md` §9.39 要消灭的那一类
   （报告里点名的 `\0soko_m58`），只是从"报错里带"变成了"判定直接靠它"。

### 3.3 根因定位

`crates/front/src/compile/meta.rs:1-20` 的**第一条设计红线**原文：

> **元变量不进项**：引擎只在**一次求解内**活着，出口 `zonk` 之后不许留 `\0soko_m*`
> （本模块的 `discharge` 出口自检 + 调用方 `elab_expr` 遇未知名自然报错 = **双保险**）

实测说明：在 strict 路径上**第一道网（`zonk`/`discharge` 自检）漏了**，于是第二道网按设计"自然报错"：

- 名字的**来源**是引擎的元变量名 `\u{0}soko_m{id}`（`meta.rs:27` 的 `META_PREFIX`；注意与记法望远镜的
  `\0soko_mp{i}`（`elab.rs:2997`）**不是一族** —— 诊断里是 `soko_m2`，所以是**引擎元变量**，不是望远镜 fresh 名）；
- 它**应当在** `elab_expr` 的 Ident 分支被翻译成内核元项 —— 那个分支就在
  `crates/front/src/compile/elab.rs:5537-5542`：`crate::compile::meta::decl_meta_carrier(ctx.metas, name)`
  命中 ⇒ `Expr::Meta{id,…}`；**没命中** ⇒ "原样落到 unknown-identifier（不发明类型）"（注释原文）；
- 所以这条读数的准确含义是：**`\0soko_m2` 这一格在那一刻不在声明级 meta store 里**
  （要么它在一次**局部**求解的 store 里、出口没 `zonk`；要么 store 在这一路没被 park/换出）。
  哪一种是真因需要一次定点取证（见下），但两条都不改结论：**strict 路径把未 zonk 的元变量名交给了
  `elab_expr`**。
- **为什么只有 strict 红**：loose 的老 app 路径在到达这个 Ident **之前**就完成了
  defaulting/求解（`implicit.rs::solve_prefix_outcome_with_args_alt` 那一路），名字根本没逃出来。

**定点取证建议（一行判据，不改判定）**：在 `elab_expr` 那个分支的"没命中"出口加一条
`debug_assert!(!meta::is_meta_name(name))`（或 `SOKO_TRACE_META_LEAK=1` 打印 `id` + 调用栈），
两档各跑 §3.1 一次 ⇒ 立刻区分"局部 store 未 zonk"与"store 未 park"。

### 3.4 影响面

- **语料命中 0**：34 个课程文件用 `≈`，但"两侧都是零元糖"的形状**只出现在注释里**
  （`units/notation-cheatsheet.sokonanoda:116,146` 明确写「`∅ ≈ {b}` 补不出论域 ⇒ 写点名」）；
  `units/I.3/unit09-equinumerosity.sokonanoda:143` 的 `{a} ≈ {b}` 两侧都有来源 —— 该文件整份在
  `--dialect strict` 下 rc=**0**、零诊断（实测），不受这条影响。
- **但是门面**：这条形状正是 G-48 / E19 / M1–M4 一路修下来的目标形状（`metavar-m0.md` 的 13 形状之一），
  修它的机理就是为了"学生写两边都是糖的集合等式时能过" ⇒ 翻默认会让**修过的能力**看起来是坏的。
- 测试面：`crates/cli/tests/metavar_engine.rs` 3 条、`metavar_inventory.rs` 2 条、
  `implicit_metavar.rs` 1 条、`notation_metavar.rs` 1 条 —— 共 **7 条** CLI 判据会红。

### 3.5 建议的修法方向

1. **先补不变量**：引擎出口对所有**写回源级 AST**的项跑一次 `zonk`（或断言
   `!mentions(|n| is_meta_name(n))`），把"第一道网"在 strict 路径上真正装上；
2. **再加卫生网**：即使漏了也不许把 `\0soko_m*` / `\0soko_mp*` / `?10…` 印进用户可见文案 ——
   复用 `kernel_app_report.rs:223` 的 `sanitize`（已有反向判据 `kernel_names_never_leak`：喂内核名 ⇒ 输出里一个都不许剩）；
3. **判定要对**：修完之后 §3.1 在**两档都必须 rc=0**（M1 的硬红线），并把它做成判据（§4）。

---

## §4 为什么这三条没被现有守卫抓住（"咬不住的守卫等于没有"）

| 缺口 | 现存的"守卫" | 它为什么没响 |
|---|---|---|
| 1 | `check.py` G1/G4 | **它响了** —— 但只在 strict 下；默认档是 loose ⇒ CI 从来没跑过 strict 的课程门禁（S6 读数手跑过一次并如实记了红，只是当时判为"接受代价"） |
| 2 | `eqsubst_predicate.rs` 的「**两档同判**」（a17 写的） | 夹具走的是 `CompileOptions::default()` ⇒ 翻默认之前**永远只有 loose 那一档**；"两档"从来没被真的跑过 |
| 3 | `metavar_engine.rs` 的"**两档都必须绿**" | 那张表的四臂是 **`SOKO_METAVAR` 的四个开关档**（`default` / `sibling` / `engine` / `0`），**`strict` 那一列是 `SOKO_METAVAR=0`（元参数逃生门），不是方言** ⇒ 方言轴从来没有判据 |

⇒ **建议（下一刀）**：凡"两档/两读法"的判据，必须**显式**打 `--dialect {loose,strict}`
（或 `CompileOptions::with_dialect`），**不许**靠"默认档"当其中一臂 —— 否则它只测了一档，
而且翻默认的那一天会把**一组成熟判据同时打红**（本轮的 25 条就是这么来的）。

---

## §5 翻默认的**真实成本**（将来真要翻时的清单）

翻默认本身是**一行**（`Dialect::default()`）+ 设置项 `default` + 文档；但它会**同时打红 25 条既有判据**：

| 层 | 红 | 归属 |
|---|---|---|
| `sokonanoda-front --lib` | **13** | 发现 2 有 **10** 条（`eqsubst_predicate` **8** + `apply_argument_hole_becomes_a_subgoal` + `eq_symm_and_eq_subst_infer_their_implicit_arguments`）；发现 3 的同类 **2** 条（`parameterized_option_checks_and_derives_recursor`、`set_option_pp_explicit_covers_check_expressions_but_not_bare_constant_signatures`）；「位置实参喂隐式」夹具 **1** 条（`old_style_arguments_win_only_when_they_actually_fit_the_layer_domains`） |
| `sokonanoda-lsp --lib` | **1** | `tests::dialect::two_initializations_pick_different_dialects`（"缺省 ⇒ loose"那一臂） |
| `sokonanoda-cli` | **11** | `apply_eqsubst` 1 · `dialect_flag` 1 · `dialect_sources` 2 · `metavar_engine` 3 · `metavar_inventory` 2 · `implicit_metavar` 1 · `notation_metavar` 1 |
| 课程门禁 `check.py` | **2 目标** | 发现 1（G1+G4） |
| 扩展（stub + 真 e2e） | 2 条 | `test-extension-host.js` 的"默认 = false"断言、`src/test/extension.test.js` 的 `dialect setting flips…` |

⚠ 这 25 条里**大部分不是"脏测试"**：它们的夹具**本来就写的是老读法的合法形状**（`some Nat`、`wrap h`、
`{b}` 两侧零元糖、`apply Eq.subst _`）。翻默认的正确做法**不是**给它们钉 `--dialect loose`
（那等于把新默认档从判据里抹掉），而是**先修 §2/§3、再逐条把夹具迁到两档都合法的写法**，
并给每条**显式加一条 strict 臂** —— 这正是本轮停手的原因：在 §2/§3 修好之前，
钉 loose 只是把红藏起来，不钉就是红。

---

## §6 读数与复现（构建身份 + 命令）

- **二进制身份**：`097996f683cc079b` = HEAD `236db5ad` 的 `cargo build --release -p sokonanoda-cli`
  （S7 改动 `git checkout --` 还原**之后**重编；`target/release/sokonanoda`）。
  ⚠ 该二进制来自 `236db5ad` 的**工作树**；之后主会话落的 `2e28a9f1`（D1 决议）**只动 `docs/`/`reports/`**
  （`git diff --stat 236db5ad..HEAD -- crates/ editor/ skills/` 为空）⇒ 本报告所有读数对它同样成立。
  对照：翻默认时的二进制是 `7c406f2467d4d4bd`（同一源码 + 一行默认值差异）+ 文档/设置改动。
- 一次判卷（本报告所有 rc/诊断都出自这一条）：

  ```bash
  env -u SOKO_STRICT_AT SOKONANODA_NO_PROJECT_ARTIFACTS=1 SOKONANODA_NO_MODULE_ARTIFACTS=1 \
    SOKONANODA_CACHE_DIR=$(mktemp -d) <bin> grade --dialect {loose,strict} <file> --json
  ```

- 课程门禁：`SOKO_STRICT_AT=1 python3 courses/set-theory/tools/check.py`
  ⇒ `EXIT=1`、`✗` **2** 条、`258 个目标 —— 2188 checked · 914 open · 2 个被判负`。
- 复现件原文（三份，逐字）：本文件 §2.1 A/B、§3.1；以及 §1.1 的两个课程文件路径。

## §7 边界（本轮**没做**什么）

- 没有改判定逻辑（`crates/front` 只有 S7 的默认值一行，且已还原）；没有碰 `crates/kernel/**`；
  `elab.rs` 8841 行不变；
- 没有改课程文案、没有改 `courses/set-theory/tools/check.py`、没有归档任何 orphan；
- 没有改 `scripts/docs-budget.json` 的任何 cap，没有新增/删除 `docs/**`；
- 没有 `git commit` / `git push`；工作区只剩本报告一个未跟踪文件。

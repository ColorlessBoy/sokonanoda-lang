# 卷 I《集合论》改写清单：逐文件 · 逐声明

> **性质**：只读调研产物。生成方式：文本统计 + `python3 courses/set-theory/tools/check.py --json` 交叉校准；
> **未修改** `courses/set-theory/` 下任何文件。
> **用途**：保证「联结词点名 → Lean 4 数学符号」与「项模式 → `by` 块」两件改写**不遗漏任何声明**。
> **必须与本文档同轮处理的还有 §5 的元数据同步清单。**

## 0. 口径与校准（先读）

### 0.1 计数口径（已用 kernel 逐目标校准）

| 口径 | 定义 |
|---|---|
| kernel `decl.checked` | 顶层声明数 **− `example` − `sorry`**。`axiom`/`inductive` **计入**；`example` **不计入** |
| kernel `exercise.open` | 值为 `sorry` 的声明数 |
| 本文档「已证」 | 有完整证明体且体内无 `sorry` 的 `theorem`/`example`（= kernel checked 加回 example） |

**本次实测**（`python3 courses/set-theory/tools/check.py --json`，退出码 0）：

```
36 个目标 · 329 checked · 99 open · 0 判负      (canvas_open 96 · solutions_open 0 · lib_open 0)
```

本文档文本统计 **424 个顶层声明**（99 开放 + 325 非开放）。核对：325 − 6 个 `example` = 319，
再加门禁把 `lib/Demo` 判两次（`lib Demo` + `lib 自检`，各 10 条）→ **329**。**36 个目标逐目标吻合。**

### 0.2 两件改写的现状（两条都是「从零开始」，不是「迁移」）

| 任务 | 现状 |
|---|---|
| ① 点名 → Lean 4 数学符号 | 全课程**当前 0 处**使用 `∧ ∨ ↔ ∃ ∀ ¬`；已声明 **14 条记法命令 / 10 个符号**（`∈ ⊆ ∪ ∅` 只在记法对照页、`𝒫 ᶜ '' ⁻¹' ×ˢ` 在 `lib/Set`、`∃` 在单元⑧） |
| ② 项模式 → `by` 块 | 全课程**当前 0 个 `by` 块**（`grep -rn ":= by"` 零命中）；414 个声明是项模式 |

> ⚠️ 第 ② 条的硬约束：**语言侧 tactic 白名单只有 7 个**（`intro`/`exact`/`apply`/`assumption`/`rfl`/`match`/`sorry`，
> `crates/front/src/parser.rs:1240-1300`）。没有 `constructor`/`obtain`/`rcases`/`cases`/`use`/`refine`/`have`/`rw`/`simp`。
> 详见 §4.1。

> ⚠️ 第 ① 条的硬约束：**`→` 不是本语言 token**（实测 `elab-unknown-identifier: unknown identifier '→'`），
> 且没有可指向的常量名 ⇒ **记法也声明不出来**；`∀` 是原生 token（但要写类型标注）；`∃` 走 `binder_notation` 且目标必须在作用域。详见 §4.7。

### 0.3 §2 表格的列

| 列 | 含义 |
|---|---|
| `行` | 声明起始行（1-based） |
| `关键字 名字` | `def`/`theorem`/`example`/`inductive`/`axiom`/`abbrev`；`example` 无名 |
| `状态` | 已证 / 开放 / 定义 / 公理 / 归纳定义 |
| `形态·形状` | 全课程都是项模式；tactic 模式一栏为空（0 个 `by`） |
| `体行` | 从 `:=` 所在行到最后一行非注释代码的行数 |
| `逻辑构件` | 签名+体里出现的联结词与构件及次数（构件优先，最多 7 项） |
| `hint` | 该声明上方 `-- soko:hint` 条数 |

## TL;DR（十行版）

* **35 个文件 / 6550 行 / 424 个顶层声明**：已证 `theorem`/`example` **245** · 开放 99（全是裸 `:= sorry`，其中 1 个是 `def`）· `def` 71 · `axiom` 6 · `inductive` 4 · `example` 6。
* **kernel 口径已逐目标校准**：`decl.checked` = 声明数 − `example` − `sorry`；实测 `36 目标 · 329 checked · 99 open · 0 判负`（§0.1）。
* **tactic 现状 = 0 个 `by` 块**；白名单只有 7 个 tactic（无 `constructor`/`obtain`/`cases`/`use`/`rw`）⇒ 这是**从零引入**，不是迁移（§4.1）。
* **245 条已证按可改写性分三类**（本文档口径）：A 74（`intro`+`exact`/`apply` 可机械改写）· B 64（构造子，`apply` 可覆盖）· **C 107（含消去/重写，现有白名单转不干净）**；不含 `False.elim` 时是 79/64/102（§3.5 有口径对齐表）。
* **C 类热点**：`And.left/right` 85 声明 · `Exists.elim` 37 声明/76 处（最深嵌套 3 层）· `Eq.subst` 37 声明/55 处（7 处命题级重写）· `Or.elim` 18 · `congrArg` 14（5 处到 `Prop`）· `let` 5 · `match` 14 · `Nat.rec` 1（§3.5）。
* **最长 20 个项模式证明**占改写成本主体，最长 `unit12-solution:173 flawed_equalities_refuted` **156 行**（§3.4）。
* **`→` 不可用**（不是 token，也没有记法目标）；`∀` 原生但必须写类型标注；`∃` 需目标在作用域且只能在表达式开头；`∅` 在 `Eq` 操作数位解不出 `α`（课程里 **37 处**受影响）；`=` 也声明不出来（§4.7）。
* **71 个 `def`、6 个 `axiom`、4 个 `inductive` 不该改 tactic**（数据/谓词构造，改了只是 `by exact`）（§4.6）。
* **hint 294 条，其中 108 条点名项模式词汇**（`And.intro`/`Or.elim`/`Exists.elim`/`Eq.subst`…）——hint 必须与证明同轮改（§3.6）。
* **元数据会变成假话的地方**：README `:126/133/145-158/163/192-195/209` · `lib/Set:140` · `course.json`（仅结构变化）· `sokonanoda.toml` 的 `requires` · 仓库根/设计文档 20+ 处计数 · `site/data/site.json` 与 `ledger.jsonl` 必须重跑（§5）。

## 目录

| 节 | 内容 |
|---|---|
| §0 | 口径与校准（kernel 计数口径、两件改写的现状） |
| §1 | **文件级清单（A）**：35 个文件的总表 + 记法声明明细 + namespace |
| §2 | **声明级清单（B）**：424 条逐声明表（lib 9 / units 13 / solutions 13） |
| §3 | **统计汇总（C）**：总数、项模式 vs tactic、联结词签名/体分布、最长 20 个证明、C 类细分、hint |
| §4 | **改写风险点（D）**：tactic 白名单、显式消去、参数位高阶函数、`let`/`match`、重写构件、`def` 清单、记法 7 边界 |
| §5 | **文档/元数据清单（E）**：course.json / README / toml / 写死计数 / 测试 / 教学叙事 |
| §6 | 附录：工作量与建议顺序 |

---

---

---

---

## 1. 文件级清单（A）

### 1.1 总表（35 个文件）

| # | 路径 | 层 | 行数 | import | 记法声明 | 声明 | 已证 | 开放 | def | axiom | inductive | example | hint | kernel checked/open |
|---:|---|---|---:|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | `lib/Demo.sokonanoda` | lib（L2 课程标准库） | 61 | `import lib.Logic`<br>`import lib.Set`<br>`import lib.Exists`<br>`import lib.Prod`<br>`import lib.Rel`<br>`import lib.Fun`<br>`import lib.Image`<br>`import lib.Equiv` | — | 10 | 10 | 0 | 0 | 0 | 0 | 0 | 0 | 10/0 |
| 2 | `lib/Equiv.sokonanoda` | lib（L2 课程标准库） | 148 | `import lib.Logic`<br>`import lib.Exists`<br>`import lib.Set` | — | 5 | 1 | 0 | 4 | 0 | 0 | 0 | 0 | 5/0 |
| 3 | `lib/Exists.sokonanoda` | lib（L2 课程标准库） | 102 | `import lib.Logic` | — | 3 | 1 | 0 | 1 | 0 | 1 | 0 | 0 | 3/0 |
| 4 | `lib/Fun.sokonanoda` | lib（L2 课程标准库） | 141 | `import lib.Logic`<br>`import lib.Exists` | — | 14 | 7 | 0 | 7 | 0 | 0 | 0 | 0 | 14/0 |
| 5 | `lib/Image.sokonanoda` | lib（L2 课程标准库） | 91 | `import lib.Logic`<br>`import lib.Exists`<br>`import lib.Set` | — | 6 | 4 | 0 | 2 | 0 | 0 | 0 | 0 | 6/0 |
| 6 | `lib/Prod.sokonanoda` | lib（L2 课程标准库） | 73 | `import lib.Logic` | — | 6 | 3 | 0 | 2 | 0 | 1 | 0 | 0 | 6/0 |
| 7 | `lib/Rel.sokonanoda` | lib（L2 课程标准库） | 70 | `import lib.Logic`<br>`import lib.Exists` | — | 7 | 3 | 0 | 3 | 1 | 0 | 0 | 0 | 7/0 |
| 8 | `lib/Set.sokonanoda` | lib（L2 课程标准库） | 159 | `import lib.Logic` | `prefix:100 " 𝒫 " => Set.powerset`<br>`postfix:100 " ᶜ " => Set.compl`<br>`infixr:80 " '' " => Set.image`<br>`infixr:80 " ⁻¹' " => Set.preimage`<br>`infixr:80 " ×ˢ " => Set.prod` | 23 | 10 | 0 | 12 | 1 | 0 | 0 | 0 | 23/0 |
| 9 | `units/notation-cheatsheet.sokonanoda` | units（画布） | 251 | `import lib.Logic`<br>`import lib.Set` | `infix:50 " ∈ " => Set.mem`<br>`infix:50 " ⊆ " => Set.subset`<br>`infixl:65 " ∪ " => Set.union`<br>`notation "∅" => Set.empty` | 23 | 20 | 3 | 0 | 0 | 0 | 2 | 6 | 18/3 |
| 10 | `units/solutions/notation-cheatsheet-solution.sokonanoda` | units/solutions（解答钥匙） | 140 | `import lib.Logic`<br>`import lib.Set` | `infix:50 " ∈ " => Set.mem`<br>`infix:50 " ⊆ " => Set.subset`<br>`infixl:65 " ∪ " => Set.union`<br>`notation "∅" => Set.empty` | 21 | 21 | 0 | 0 | 0 | 0 | 0 | 0 | 21/0 |
| 11 | `units/solutions/unit01-solution.sokonanoda` | units/solutions（解答钥匙） | 40 | `import lib.Logic`<br>`import lib.Set` | — | 6 | 6 | 0 | 0 | 0 | 0 | 0 | 0 | 6/0 |
| 12 | `units/solutions/unit02-solution.sokonanoda` | units/solutions（解答钥匙） | 84 | `import lib.Logic`<br>`import lib.Set` | — | 10 | 10 | 0 | 0 | 0 | 0 | 0 | 0 | 10/0 |
| 13 | `units/solutions/unit03-solution.sokonanoda` | units/solutions（解答钥匙） | 88 | `import lib.Logic`<br>`import lib.Set` | — | 11 | 11 | 0 | 0 | 0 | 0 | 0 | 0 | 11/0 |
| 14 | `units/solutions/unit04-solution.sokonanoda` | units/solutions（解答钥匙） | 166 | `import lib.Logic`<br>`import lib.Set` | — | 8 | 8 | 0 | 0 | 0 | 0 | 0 | 0 | 8/0 |
| 15 | `units/solutions/unit05-solution.sokonanoda` | units/solutions（解答钥匙） | 123 | `import lib.Logic`<br>`import lib.Set`<br>`import lib.Prod` | — | 8 | 7 | 0 | 1 | 0 | 0 | 0 | 0 | 8/0 |
| 16 | `units/solutions/unit06-solution.sokonanoda` | units/solutions（解答钥匙） | 371 | `import lib.Logic`<br>`import lib.Exists`<br>`import lib.Set`<br>`import lib.Rel` | — | 23 | 13 | 0 | 10 | 0 | 0 | 0 | 0 | 23/0 |
| 17 | `units/solutions/unit07-solution.sokonanoda` | units/solutions（解答钥匙） | 126 | `import lib.Logic`<br>`import lib.Set`<br>`import lib.Exists`<br>`import lib.Fun` | — | 9 | 9 | 0 | 0 | 0 | 0 | 0 | 0 | 9/0 |
| 18 | `units/solutions/unit08-solution.sokonanoda` | units/solutions（解答钥匙） | 462 | `import lib.Logic`<br>`import lib.Exists`<br>`import lib.Set`<br>`import lib.Image` | — | 30 | 25 | 0 | 4 | 0 | 1 | 0 | 0 | 30/0 |
| 19 | `units/solutions/unit09-solution.sokonanoda` | units/solutions（解答钥匙） | 396 | `import lib.Logic`<br>`import lib.Exists`<br>`import lib.Set`<br>`import lib.Equiv` | — | 13 | 10 | 0 | 3 | 0 | 0 | 0 | 0 | 13/0 |
| 20 | `units/solutions/unit10-solution.sokonanoda` | units/solutions（解答钥匙） | 136 | `import lib.Logic`<br>`import lib.Set`<br>`import lib.Exists`<br>`import lib.Fun`<br>`import lib.Equiv` | — | 9 | 6 | 0 | 1 | 2 | 0 | 0 | 0 | 9/0 |
| 21 | `units/solutions/unit11-solution.sokonanoda` | units/solutions（解答钥匙） | 87 | `import lib.Logic`<br>`import lib.Set`<br>`import lib.Exists` | — | 5 | 5 | 0 | 0 | 0 | 0 | 0 | 0 | 5/0 |
| 22 | `units/solutions/unit12-solution.sokonanoda` | units/solutions（解答钥匙） | 584 | `import lib.Logic`<br>`import lib.Set`<br>`import lib.Exists`<br>`import lib.Equiv`<br>`import lib.Rel`<br>`import lib.Fun`<br>`import lib.Image` | — | 9 | 9 | 0 | 0 | 0 | 0 | 0 | 0 | 9/0 |
| 23 | `units/unit01-sets-membership.sokonanoda` | units（画布） | 83 | `import lib.Logic`<br>`import lib.Set` | — | 8 | 2 | 6 | 0 | 0 | 0 | 0 | 18 | 2/6 |
| 24 | `units/unit02-subsets-empty.sokonanoda` | units（画布） | 120 | `import lib.Logic`<br>`import lib.Set` | — | 11 | 1 | 10 | 0 | 0 | 0 | 0 | 30 | 1/10 |
| 25 | `units/unit03-union-inter-powerset.sokonanoda` | units（画布） | 181 | `import lib.Logic`<br>`import lib.Set` | — | 15 | 4 | 11 | 0 | 0 | 0 | 1 | 33 | 3/11 |
| 26 | `units/unit04-extensionality-identities.sokonanoda` | units（画布） | 169 | `import lib.Logic`<br>`import lib.Set` | — | 11 | 3 | 8 | 0 | 0 | 0 | 0 | 24 | 3/8 |
| 27 | `units/unit05-pairs-products.sokonanoda` | units（画布） | 202 | `import lib.Logic`<br>`import lib.Set`<br>`import lib.Prod` | — | 12 | 3 | 7 | 2 | 0 | 0 | 0 | 21 | 5/7 |
| 28 | `units/unit06-relations.sokonanoda` | units（画布） | 216 | `import lib.Logic`<br>`import lib.Exists`<br>`import lib.Set`<br>`import lib.Rel` | — | 23 | 5 | 8 | 10 | 0 | 0 | 0 | 24 | 15/8 |
| 29 | `units/unit07-functions.sokonanoda` | units（画布） | 202 | `import lib.Logic`<br>`import lib.Set`<br>`import lib.Exists`<br>`import lib.Fun` | — | 12 | 3 | 9 | 0 | 0 | 0 | 0 | 27 | 3/9 |
| 30 | `units/unit08-images-preimages.sokonanoda` | units（画布） | 351 | `import lib.Logic`<br>`import lib.Exists`<br>`import lib.Set`<br>`import lib.Image` | `binder_notation "∃" => Exists` | 27 | 13 | 9 | 4 | 0 | 1 | 3 | 27 | 15/9 |
| 31 | `units/unit09-equinumerosity.sokonanoda` | units（画布） | 192 | `import lib.Logic`<br>`import lib.Exists`<br>`import lib.Set`<br>`import lib.Equiv` | — | 13 | 3 | 7 | 3 | 0 | 0 | 0 | 21 | 6/7 |
| 32 | `units/unit10-cantor.sokonanoda` | units（画布） | 197 | `import lib.Logic`<br>`import lib.Set`<br>`import lib.Exists`<br>`import lib.Fun`<br>`import lib.Equiv` | — | 12 | 2 | 7 | 2 | 2 | 0 | 0 | 21 | 5/7 |
| 33 | `units/unit11-universe-russell.sokonanoda` | units（画布） | 203 | `import lib.Logic`<br>`import lib.Set`<br>`import lib.Exists` | — | 8 | 3 | 5 | 0 | 0 | 0 | 0 | 15 | 3/5 |
| 34 | `units/unit12-synthesis.sokonanoda` | units（画布） | 474 | `import lib.Logic`<br>`import lib.Set`<br>`import lib.Exists`<br>`import lib.Equiv`<br>`import lib.Rel`<br>`import lib.Fun`<br>`import lib.Image` | — | 13 | 4 | 9 | 0 | 0 | 0 | 0 | 27 | 4/9 |

**读法**：`lib/Logic` 是**空壳**（0 声明，只有注释）；`lib Demo` 与 `lib 自检` 是同一个文件被门禁判两次。
记法声明**不是声明**（零事件、不进声明表），所以它不改变任何计数。

### 1.2 记法声明明细（全课程 **14 条命令 / 10 个符号**，分布在 4 个文件）

| 文件 | 行 | 记法命令 | 目标名 | 目标定义在哪 |
|---|---:|---|---|---|
| `lib/Set.sokonanoda` | 155 | `prefix:100 " 𝒫 " => Set.powerset` | `Set.powerset` | `lib/Set.sokonanoda:76` |
| `lib/Set.sokonanoda` | 156 | `postfix:100 " ᶜ " => Set.compl` | `Set.compl` | `lib/Set.sokonanoda:74` |
| `lib/Set.sokonanoda` | 157 | `infixr:80 " '' " => Set.image` | `Set.image` | `lib/Image.sokonanoda:30` |
| `lib/Set.sokonanoda` | 158 | `infixr:80 " ⁻¹' " => Set.preimage` | `Set.preimage` | `lib/Image.sokonanoda:35` |
| `lib/Set.sokonanoda` | 159 | `infixr:80 " ×ˢ " => Set.prod` | `Set.prod` | **单元⑤ 画布** `units/unit05-pairs-products.sokonanoda:100`（不在 lib！） |
| `units/notation-cheatsheet.sokonanoda` | 86 | `infix:50 " ∈ " => Set.mem` | `Set.mem` | `lib/Set.sokonanoda:55` |
| `units/notation-cheatsheet.sokonanoda` | 87 | `infix:50 " ⊆ " => Set.subset` | `Set.subset` | `lib/Set.sokonanoda:57` |
| `units/notation-cheatsheet.sokonanoda` | 88 | `infixl:65 " ∪ " => Set.union` | `Set.union` | `lib/Set.sokonanoda:68` |
| `units/notation-cheatsheet.sokonanoda` | 89 | `notation "∅" => Set.empty` | `Set.empty` | `lib/Set.sokonanoda:59` |
| `units/solutions/notation-cheatsheet-solution.sokonanoda` | 24 | `infix:50 " ∈ " => Set.mem` | `Set.mem` | `lib/Set.sokonanoda:55` |
| `units/solutions/notation-cheatsheet-solution.sokonanoda` | 25 | `infix:50 " ⊆ " => Set.subset` | `Set.subset` | `lib/Set.sokonanoda:57` |
| `units/solutions/notation-cheatsheet-solution.sokonanoda` | 26 | `infixl:65 " ∪ " => Set.union` | `Set.union` | `lib/Set.sokonanoda:68` |
| `units/solutions/notation-cheatsheet-solution.sokonanoda` | 27 | `notation "∅" => Set.empty` | `Set.empty` | `lib/Set.sokonanoda:59` |
| `units/unit08-images-preimages.sokonanoda` | 141 | `binder_notation "∃" => Exists` | `Exists` | `lib/Exists.sokonanoda:86`（真归纳） |

**关键事实**：`∈ ⊆ ∪ ∅` 这 4 条**只在记法对照页（+ 它的解答）里声明**，`lib/` 里没有。
而「同一符号全课程只能声明一次」（重复声明是 parse 错）⇒ 若要让所有单元用上这 4 个符号，
必须把它们**移进 `lib/Set.sokonanoda`** 并**同时删掉记法对照页与解答里的 4 条**（否则撞车）。

### 1.3 namespace 结构

| 文件 | namespace |
|---|---|
| `lib/Set.sokonanoda` | `namespace Set` … `end Set`（**唯一**用 namespace 的文件；`def Set` 留在命名空间外） |
| 其余 34 个 | 无 namespace（全局名） |

---

## 2. 声明级清单（B）——逐文件、逐声明（424 条，源文件顺序）

### 2.1 `lib/`（L2 课程标准库，9 个文件）

#### `lib/Demo.sokonanoda` — 61 行 · 10 个声明 · import: `import lib.Logic`, `import lib.Set`, `import lib.Exists`, `import lib.Prod`, `import lib.Rel`, `import lib.Fun`, `import lib.Image`, `import lib.Equiv`

| 行 | 关键字 名字 | 状态 | 形态·形状 | 体行 | 逻辑构件 | hint |
|---:|---|---|---|---:|---|---:|
| 13 | `theorem demo_and_comm` | 已证 | 项模式 · And.intro 配对 + And.left/right 投影 | 2 | And.intro×1 And.left×1 And.right×1 And×2 | — |
| 17 | `theorem demo_subset_unfold` | 已证 | 项模式 · 直接引用 `Set.subset_def` | 2 | Iff×1 forall×1 | — |
| 21 | `theorem demo_mem_powerset` | 已证 | 项模式 · Iff.mp/mpr 单向 | 3 | Iff.mpr×1 | — |
| 27 | `theorem demo_exists_intro` | 已证 | 项模式 · Exists.intro 见证 + Eq.refl | 2 | Exists.intro×1 Eq.refl×1 Exists×1 | — |
| 30 | `theorem demo_exists_elim` | 已证 | 项模式 · Exists.elim 消去 + Exists.intro 见证 + Eq.refl | 4 | Exists.intro×1 Exists.elim×1 Eq.refl×1 | — |
| 36 | `theorem demo_prod_fst` | 已证 | 项模式 · 直接引用 `Prod.fst_mk` | 2 | — | — |
| 41 | `theorem demo_rel_inv_inv_apply` | 已证 | 项模式 · 直接引用 `Rel.inv_inv_apply` | 2 | Iff×1 | — |
| 46 | `theorem demo_fun_comp_apply` | 已证 | 项模式 · 直接引用 `Function.comp_apply` | 2 | — | — |
| 51 | `theorem demo_mem_preimage` | 已证 | 项模式 · 直接引用 `Set.mem_preimage` | 2 | Iff×1 | — |
| 56 | `theorem demo_equiv_refl` | 已证 | 项模式 · Eq.refl | 6 | Eq.refl×2 | — |

#### `lib/Equiv.sokonanoda` — 148 行 · 5 个声明 · import: `import lib.Logic`, `import lib.Exists`, `import lib.Set`

| 行 | 关键字 名字 | 状态 | 形态·形状 | 体行 | 逻辑构件 | hint |
|---:|---|---|---|---:|---|---:|
| 95 | `def Set.MapsTo` | 定义 | 项模式 · **定义体（数据/谓词构造，非证明）** | 2 | forall×1 | — |
| 100 | `def Set.LeftInvOn` | 定义 | 项模式 · **定义体（数据/谓词构造，非证明）** | 2 | forall×1 | — |
| 105 | `def Set.RightInvOn` | 定义 | 项模式 · **定义体（数据/谓词构造，非证明）** | 2 | forall×1 | — |
| 111 | `def Set.Equiv` | 定义 | 项模式 · **定义体（数据/谓词构造，非证明）** | 6 | And×3 Exists×2 | — |
| 121 | `theorem Set.Equiv.mk` | 已证 | 项模式 · And.intro 配对 + Exists.intro 见证 | 22 | And.intro×3 Exists.intro×2 And×8 Exists×2 | — |

#### `lib/Exists.sokonanoda` — 102 行 · 3 个声明 · import: `import lib.Logic`

| 行 | 关键字 名字 | 状态 | 形态·形状 | 体行 | 逻辑构件 | hint |
|---:|---|---|---|---:|---|---:|
| 86 | `inductive Exists` | 归纳定义 | —（只有签名） | 0 | Exists×2 | — |
| 92 | `def Exists.elim` | 定义 | 项模式 · **recursor 定义**（`Prod.rec`/`Exists.rec`） | 2 | Exists.elim×1 Exists.rec×1 Exists×2 | — |
| 99 | `theorem Exists.imp` | 已证 | 项模式 · Exists.elim 消去 + Exists.intro 见证 | 3 | Exists.intro×1 Exists.elim×1 Exists×3 forall×1 | — |

#### `lib/Fun.sokonanoda` — 141 行 · 14 个声明 · import: `import lib.Logic`, `import lib.Exists`

| 行 | 关键字 名字 | 状态 | 形态·形状 | 体行 | 逻辑构件 | hint |
|---:|---|---|---|---:|---|---:|
| 60 | `def Function.comp` | 定义 | 项模式 · **定义体（数据/谓词构造，非证明）** | 2 | — | — |
| 65 | `def Function.Injective` | 定义 | 项模式 · **定义体（数据/谓词构造，非证明）** | 2 | forall×2 | — |
| 70 | `def Function.Surjective` | 定义 | 项模式 · **定义体（数据/谓词构造，非证明）** | 2 | Exists×1 forall×1 | — |
| 73 | `def Function.Bijective` | 定义 | 项模式 · **定义体（数据/谓词构造，非证明）** | 2 | And×1 | — |
| 78 | `def Function.LeftInverse` | 定义 | 项模式 · **定义体（数据/谓词构造，非证明）** | 2 | forall×1 | — |
| 83 | `def Function.RightInverse` | 定义 | 项模式 · **定义体（数据/谓词构造，非证明）** | 2 | forall×1 | — |
| 88 | `def Function.Inverse` | 定义 | 项模式 · **定义体（数据/谓词构造，非证明）** | 2 | And×1 | — |
| 97 | `theorem Function.comp_apply` | 已证 | 项模式 · Eq.refl | 2 | Eq.refl×1 | — |
| 101 | `theorem Function.injective_def` | 已证 | 项模式 · Iff.intro 双向 | 5 | Iff.intro×1 Iff×1 forall×6 | — |
| 108 | `theorem Function.surjective_def` | 已证 | 项模式 · Iff.intro 双向 | 5 | Iff.intro×1 Iff×1 Exists×3 forall×3 | — |
| 115 | `theorem Function.bijective_def` | 已证 | 项模式 · Iff.intro 双向 | 5 | Iff.intro×1 And×3 Iff×1 | — |
| 123 | `theorem Function.leftInverse_def` | 已证 | 项模式 · Iff.intro 双向 | 4 | Iff.intro×1 Iff×1 forall×3 | — |
| 129 | `theorem Function.rightInverse_def` | 已证 | 项模式 · Iff.intro 双向 | 4 | Iff.intro×1 Iff×1 forall×3 | — |
| 135 | `theorem Function.inverse_def` | 已证 | 项模式 · Iff.intro 双向 | 5 | Iff.intro×1 And×3 Iff×1 | — |

#### `lib/Image.sokonanoda` — 91 行 · 6 个声明 · import: `import lib.Logic`, `import lib.Exists`, `import lib.Set`

| 行 | 关键字 名字 | 状态 | 形态·形状 | 体行 | 逻辑构件 | hint |
|---:|---|---|---|---:|---|---:|
| 30 | `def Set.image` | 定义 | 项模式 · **定义体（数据/谓词构造，非证明）** | 2 | And×1 Exists×1 | — |
| 35 | `def Set.preimage` | 定义 | 项模式 · **定义体（数据/谓词构造，非证明）** | 2 | — | — |
| 42 | `theorem Set.mem_image` | 已证 | 项模式 · Iff.intro 双向 | 5 | Iff.intro×1 And×3 Iff×1 Exists×3 | — |
| 50 | `theorem Set.mem_preimage` | 已证 | 项模式 · Iff.intro 双向 | 4 | Iff.intro×1 Iff×1 | — |
| 65 | `theorem Set.image_mono` | 已证 | 项模式 · fun 链（2 层）· Exists.elim 消去 + And.intro 配对 + And.left/right 投影 + Exists.intro 见证 | 9 | And.intro×1 And.left×1 And.right×1 Exists.intro×1 Exists.elim×1 And×3 forall×1 | — |
| 77 | `theorem Set.image_subset_iff` | 已证 | 项模式 · Iff.intro 双向 + Exists.elim 消去 + And.intro 配对 + And.left/right 投影 + Exists.intro 见证 | 14 | And.intro×1 And.left×1 And.right×1 Exists.intro×1 Exists.elim×1 Iff.intro×1 Eq.subst×1 … | — |

#### `lib/Prod.sokonanoda` — 73 行 · 6 个声明 · import: `import lib.Logic`

| 行 | 关键字 名字 | 状态 | 形态·形状 | 体行 | 逻辑构件 | hint |
|---:|---|---|---|---:|---|---:|
| 44 | `inductive Prod` | 归纳定义 | —（只有签名） | 0 | — | — |
| 49 | `def Prod.fst` | 定义 | 项模式 · **recursor 定义**（`Prod.rec`/`Exists.rec`） | 2 | Prod.rec×1 | — |
| 52 | `def Prod.snd` | 定义 | 项模式 · **recursor 定义**（`Prod.rec`/`Exists.rec`） | 2 | Prod.rec×1 | — |
| 57 | `theorem Prod.fst_mk` | 已证 | 项模式 · Eq.refl | 2 | Eq.refl×1 | — |
| 61 | `theorem Prod.snd_mk` | 已证 | 项模式 · Eq.refl | 2 | Eq.refl×1 | — |
| 68 | `theorem Prod.fst_snd_mk` | 已证 | 项模式 · And.intro 配对 | 4 | And.intro×1 And×1 | — |

#### `lib/Rel.sokonanoda` — 70 行 · 7 个声明 · import: `import lib.Logic`, `import lib.Exists`

| 行 | 关键字 名字 | 状态 | 形态·形状 | 体行 | 逻辑构件 | hint |
|---:|---|---|---|---:|---|---:|
| 36 | `def Rel` | 定义 | 项模式 · **定义体（数据/谓词构造，非证明）** | 1 | — | — |
| 39 | `def Rel.inv` | 定义 | 项模式 · **定义体（数据/谓词构造，非证明）** | 2 | — | — |
| 45 | `def Rel.comp` | 定义 | 项模式 · **定义体（数据/谓词构造，非证明）** | 3 | And×1 Exists×1 | — |
| 51 | `axiom Rel.ext` | 公理 | —（只有签名） | 0 | Rel.ext×1 Iff×1 forall×1 | — |
| 57 | `theorem Rel.inv_apply` | 已证 | 项模式 · 直接引用 `Iff.refl` | 2 | Iff×1 | — |
| 61 | `theorem Rel.comp_apply` | 已证 | 项模式 · 直接引用 `Iff.refl` | 2 | And×2 Iff×1 Exists×2 | — |
| 68 | `theorem Rel.inv_inv_apply` | 已证 | 项模式 · 直接引用 `Iff.refl` | 2 | Iff×1 | — |

#### `lib/Set.sokonanoda` — 159 行 · 23 个声明 · import: `import lib.Logic`

| 行 | 关键字 名字 | 状态 | 形态·形状 | 体行 | 逻辑构件 | hint |
|---:|---|---|---|---:|---|---:|
| 51 | `def Set` | 定义 | 项模式 · **定义体（数据/谓词构造，非证明）** | 1 | — | — |
| 55 | `def Set.mem` | 定义 | 项模式 · **定义体（数据/谓词构造，非证明）** | 1 | — | — |
| 57 | `def Set.subset` | 定义 | 项模式 · **定义体（数据/谓词构造，非证明）** | 1 | forall×1 | — |
| 59 | `def Set.empty` | 定义 | 项模式 · **定义体（数据/谓词构造，非证明）** | 1 | — | — |
| 61 | `def Set.univ` | 定义 | 项模式 · **定义体（数据/谓词构造，非证明）** | 1 | — | — |
| 63 | `def Set.singleton` | 定义 | 项模式 · **定义体（数据/谓词构造，非证明）** | 1 | — | — |
| 65 | `def Set.pair` | 定义 | 项模式 · **定义体（数据/谓词构造，非证明）** | 2 | Or×1 | — |
| 68 | `def Set.union` | 定义 | 项模式 · **定义体（数据/谓词构造，非证明）** | 1 | Or×1 | — |
| 70 | `def Set.inter` | 定义 | 项模式 · **定义体（数据/谓词构造，非证明）** | 1 | And×1 | — |
| 72 | `def Set.sdiff` | 定义 | 项模式 · **定义体（数据/谓词构造，非证明）** | 1 | And×1 Not×1 | — |
| 74 | `def Set.compl` | 定义 | 项模式 · **定义体（数据/谓词构造，非证明）** | 1 | Not×1 | — |
| 76 | `def Set.powerset` | 定义 | 项模式 · **定义体（数据/谓词构造，非证明）** | 1 | — | — |
| 79 | `axiom Set.ext` | 公理 | —（只有签名） | 0 | Iff×1 forall×1 | — |
| 83 | `theorem Set.subset_def` | 已证 | 项模式 · Iff.intro 双向 | 4 | Iff.intro×1 Iff×1 forall×3 | — |
| 89 | `theorem Set.mem_empty_iff_false` | 已证 | 项模式 · Iff.intro 双向 | 4 | Iff.intro×1 Iff×1 | — |
| 95 | `theorem Set.notMem_empty` | 已证 | 项模式 · fun 链（1 层）· 纯应用 | 2 | Not×1 | — |
| 98 | `theorem Set.mem_univ` | 已证 | 项模式 · True.intro | 1 | True.intro×1 | — |
| 100 | `theorem Set.mem_singleton_iff` | 已证 | 项模式 · Iff.intro 双向 | 4 | Iff.intro×1 Iff×1 | — |
| 106 | `theorem Set.mem_singleton_self` | 已证 | 项模式 · Eq.refl | 2 | Eq.refl×1 | — |
| 109 | `theorem Set.mem_union` | 已证 | 项模式 · Iff.intro 双向 | 4 | Iff.intro×1 Or×3 Iff×1 | — |
| 115 | `theorem Set.mem_inter_iff` | 已证 | 项模式 · Iff.intro 双向 | 4 | Iff.intro×1 And×3 Iff×1 | — |
| 121 | `theorem Set.mem_sdiff` | 已证 | 项模式 · Iff.intro 双向 | 5 | Iff.intro×1 And×3 Iff×1 Not×3 | — |
| 128 | `theorem Set.mem_powerset_iff` | 已证 | 项模式 · Iff.intro 双向 | 4 | Iff.intro×1 Iff×1 | — |

### 2.2 `units/`（画布，13 个文件：12 单元 + 记法对照页）

#### `units/notation-cheatsheet.sokonanoda` — 251 行 · 23 个声明 · import: `import lib.Logic`, `import lib.Set`

记法声明：`infix:50 " ∈ " => Set.mem`、`infix:50 " ⊆ " => Set.subset`、`infixl:65 " ∪ " => Set.union`、`notation "∅" => Set.empty`

非声明命令：`#check fun (α : Type) (A B : Set α) => A ⊆ B`

| 行 | 关键字 名字 | 状态 | 形态·形状 | 体行 | 逻辑构件 | hint |
|---:|---|---|---|---:|---|---:|
| 120 | `example example` | 已证 | 项模式 · 直接引用 `Set.mem_powerset_iff` | 2 | Iff×1 | — |
| 124 | `example example` | 已证 | 项模式 · Iff.intro 双向 | 4 | Iff.intro×1 Iff×1 Not×3 | — |
| 135 | `theorem demo_mem_pointful` | 已证 | 项模式 · fun 链（1 层）· 纯应用 | 2 | — | — |
| 138 | `theorem demo_mem_notation` | 已证 | 项模式 · fun 链（1 层）· 纯应用 | 2 | — | — |
| 144 | `theorem demo_subset_pointful` | 已证 | 项模式 · fun 链（1 层）· 纯应用 | 2 | forall×1 | — |
| 148 | `theorem demo_subset_notation` | 已证 | 项模式 · fun 链（1 层）· 纯应用 | 2 | forall×1 | — |
| 157 | `theorem demo_trans_pointful` | 已证 | 项模式 · fun 链（2 层）· 纯应用 | 2 | — | — |
| 161 | `theorem demo_trans_notation` | 已证 | 项模式 · fun 链（2 层）· 纯应用 | 2 | — | — |
| 166 | `theorem demo_empty_subset_pointful` | 已证 | 项模式 · fun 链（2 层）· False.elim 爆炸 | 2 | False.elim×1 | — |
| 170 | `theorem demo_empty_subset_notation` | 已证 | 项模式 · fun 链（2 层）· False.elim 爆炸 | 2 | False.elim×1 | — |
| 175 | `theorem demo_notMem_empty_pointful` | 已证 | 项模式 · 直接引用 `Set.notMem_empty` | 2 | Not×1 | — |
| 179 | `theorem demo_notMem_empty_notation` | 已证 | 项模式 · 直接引用 `Set.notMem_empty` | 2 | Not×1 | — |
| 187 | `theorem demo_union_left_pointful` | 已证 | 项模式 · fun 链（2 层）· Or.inl/inr 注入 | 2 | Or.inl×1 | — |
| 191 | `theorem demo_union_left_notation` | 已证 | 项模式 · fun 链（2 层）· Or.inl/inr 注入 | 2 | Or.inl×1 | — |
| 195 | `theorem demo_mem_union_pointful` | 已证 | 项模式 · fun 链（1 层）· 纯应用 | 2 | Or×1 | — |
| 199 | `theorem demo_mem_union_notation` | 已证 | 项模式 · fun 链（1 层）· 纯应用 | 2 | Or×1 | — |
| 205 | `theorem demo_ext_pointful` | 已证 | 项模式 · Iff.intro 双向 + Set.ext 外延 | 2 | Iff.intro×1 Set.ext×1 | — |
| 209 | `theorem demo_ext_notation` | 已证 | 项模式 · Iff.intro 双向 + Set.ext 外延 | 2 | Iff.intro×1 Set.ext×1 | — |
| 219 | `theorem demo_precedence_pointful` | 已证 | 项模式 · fun 链（2 层）· Or.inl/inr 注入 | 2 | Or.inl×2 Or×1 | — |
| 223 | `theorem demo_precedence_notation` | 已证 | 项模式 · fun 链（2 层）· Or.inl/inr 注入 | 2 | Or.inl×2 Or×1 | — |
| 236 | `theorem empty_union_subset` | **开放** | —（`:= sorry`） | 2 | — | 2 |
| 242 | `theorem mem_union_comm` | **开放** | —（`:= sorry`） | 2 | — | 2 |
| 250 | `theorem union_empty_right` | **开放** | —（`:= sorry`） | 2 | — | 2 |

#### `units/unit01-sets-membership.sokonanoda` — 83 行 · 8 个声明 · import: `import lib.Logic`, `import lib.Set`

| 行 | 关键字 名字 | 状态 | 形态·形状 | 体行 | 逻辑构件 | hint |
|---:|---|---|---|---:|---|---:|
| 22 | `theorem demo_subset_def` | 已证 | 项模式 · 直接引用 `Set.subset_def` | 2 | Iff×1 forall×1 | — |
| 27 | `theorem demo_mem_def` | 已证 | 项模式 · Iff.intro 双向 | 2 | Iff.intro×1 Iff×1 | — |
| 34 | `theorem mem_of_subset` | **开放** | —（`:= sorry`） | 2 | — | 3 |
| 46 | `theorem eq_of_same_elements` | **开放** | —（`:= sorry`） | 2 | Iff×1 forall×1 | 3 |
| 54 | `theorem mem_of_subset_singleton` | **开放** | —（`:= sorry`） | 2 | — | 3 |
| 62 | `theorem subset_of_mem_singleton` | **开放** | —（`:= sorry`） | 2 | — | 3 |
| 70 | `theorem singleton_subset_iff` | **开放** | —（`:= sorry`） | 2 | Iff×1 | 3 |
| 81 | `theorem singleton_eq_singleton_iff` | **开放** | —（`:= sorry`） | 2 | Iff×1 | 3 |

#### `units/unit02-subsets-empty.sokonanoda` — 120 行 · 11 个声明 · import: `import lib.Logic`, `import lib.Set`

| 行 | 关键字 名字 | 状态 | 形态·形状 | 体行 | 逻辑构件 | hint |
|---:|---|---|---|---:|---|---:|
| 21 | `theorem subset_refl` | **开放** | —（`:= sorry`） | 2 | — | 3 |
| 28 | `theorem subset_trans` | **开放** | —（`:= sorry`） | 2 | — | 3 |
| 36 | `theorem subset_antisymm` | **开放** | —（`:= sorry`） | 2 | — | 3 |
| 48 | `theorem demo_notMem_empty` | 已证 | 项模式 · 直接引用 `Set.notMem_empty` | 2 | Not×1 | — |
| 55 | `theorem empty_subset` | **开放** | —（`:= sorry`） | 2 | — | 3 |
| 62 | `theorem subset_empty_iff` | **开放** | —（`:= sorry`） | 2 | Iff×1 | 3 |
| 70 | `theorem eq_empty_iff_forall_notMem` | **开放** | —（`:= sorry`） | 2 | Iff×1 forall×1 Not×1 | 3 |
| 82 | `theorem singleton_ne_empty` | **开放** | —（`:= sorry`） | 2 | Not×1 | 3 |
| 90 | `theorem pair_subset_iff` | **开放** | —（`:= sorry`） | 2 | And×1 Iff×1 | 3 |
| 98 | `theorem pair_comm` | **开放** | —（`:= sorry`） | 2 | — | 3 |
| 118 | `theorem singleton_of_singleton` | **开放** | —（`:= sorry`） | 2 | — | 3 |

#### `units/unit03-union-inter-powerset.sokonanoda` — 181 行 · 15 个声明 · import: `import lib.Logic`, `import lib.Set`

| 行 | 关键字 名字 | 状态 | 形态·形状 | 体行 | 逻辑构件 | hint |
|---:|---|---|---|---:|---|---:|
| 39 | `theorem demo_mem_union` | 已证 | 项模式 · 直接引用 `Set.mem_union` | 2 | Or×1 Iff×1 | — |
| 44 | `theorem demo_mem_inter` | 已证 | 项模式 · 直接引用 `Set.mem_inter_iff` | 2 | And×1 Iff×1 | — |
| 49 | `theorem demo_mem_powerset` | 已证 | 项模式 · 直接引用 `Set.mem_powerset_iff` | 2 | Iff×1 | — |
| 60 | `example example` | 已证 | 项模式 · 直接引用 `Set.mem_powerset_iff` | 2 | Iff×1 | — |
| 72 | `theorem subset_union_left` | **开放** | —（`:= sorry`） | 2 | — | 3 |
| 79 | `theorem subset_union_right` | **开放** | —（`:= sorry`） | 2 | — | 3 |
| 86 | `theorem union_subset_iff` | **开放** | —（`:= sorry`） | 2 | And×1 Iff×1 | 3 |
| 98 | `theorem inter_subset_left` | **开放** | —（`:= sorry`） | 2 | — | 3 |
| 105 | `theorem inter_subset_right` | **开放** | —（`:= sorry`） | 2 | — | 3 |
| 112 | `theorem subset_inter` | **开放** | —（`:= sorry`） | 2 | — | 3 |
| 121 | `theorem subset_inter_iff` | **开放** | —（`:= sorry`） | 2 | And×1 Iff×1 | 3 |
| 133 | `theorem sdiff_subset` | **开放** | —（`:= sorry`） | 2 | — | 3 |
| 144 | `theorem powerset_mono` | **开放** | —（`:= sorry`） | 2 | — | 3 |
| 162 | `theorem union_subset_inter_false` | **开放** | —（`:= sorry`） | 2 | Not×1 | 3 |
| 179 | `theorem powerset_self_mem` | **开放** | —（`:= sorry`） | 2 | — | 3 |

#### `units/unit04-extensionality-identities.sokonanoda` — 169 行 · 11 个声明 · import: `import lib.Logic`, `import lib.Set`

| 行 | 关键字 名字 | 状态 | 形态·形状 | 体行 | 逻辑构件 | hint |
|---:|---|---|---|---:|---|---:|
| 39 | `theorem demo_ext_pattern` | 已证 | 项模式 · Set.ext 外延 | 2 | Set.ext×1 Iff×1 forall×1 | — |
| 44 | `theorem demo_union_univ` | 已证 | 项模式 · Iff.intro 双向 + Or.elim 两支 + Or.inl/inr 注入 + Set.ext 外延 | 10 | Or.inr×1 Or.elim×1 Iff.intro×1 Set.ext×1 Or×2 | — |
| 57 | `theorem demo_sdiff_self` | 已证 | 项模式 · Iff.intro 双向 + And.intro 配对 + And.left/right 投影 + False.elim 爆炸 + Set.ext 外延 | 7 | And.intro×1 And.left×1 And.right×1 Iff.intro×1 False.elim×1 Set.ext×1 And×2 … | — |
| 70 | `theorem union_comm` | **开放** | —（`:= sorry`） | 2 | — | 3 |
| 78 | `theorem inter_comm` | **开放** | —（`:= sorry`） | 2 | — | 3 |
| 99 | `theorem union_inter_idem` | **开放** | —（`:= sorry`） | 2 | And×1 | 3 |
| 110 | `theorem inter_union_distrib_left` | **开放** | —（`:= sorry`） | 2 | — | 3 |
| 122 | `theorem union_inter_self` | **开放** | —（`:= sorry`） | 2 | — | 3 |
| 133 | `theorem sdiff_sdiff` | **开放** | —（`:= sorry`） | 2 | — | 3 |
| 145 | `theorem union_empty` | **开放** | —（`:= sorry`） | 2 | — | 3 |
| 165 | `theorem union_sdiff_univ_ne_univ` | **开放** | —（`:= sorry`） | 2 | Not×1 | 3 |

#### `units/unit05-pairs-products.sokonanoda` — 202 行 · 12 个声明 · import: `import lib.Logic`, `import lib.Set`, `import lib.Prod`

| 行 | 关键字 名字 | 状态 | 形态·形状 | 体行 | 逻辑构件 | hint |
|---:|---|---|---|---:|---|---:|
| 43 | `theorem demo_fst_snd_mk` | 已证 | 项模式 · 直接引用 `Prod.fst_snd_mk` | 2 | And×1 | — |
| 60 | `theorem prod_fst_mk` | **开放** | —（`:= sorry`） | 2 | — | 3 |
| 68 | `theorem prod_snd_mk` | **开放** | —（`:= sorry`） | 2 | — | 3 |
| 76 | `theorem prod_mk_inj` | **开放** | —（`:= sorry`） | 2 | And×1 | 3 |
| 85 | `theorem prod_mk_eq_iff` | **开放** | —（`:= sorry`） | 2 | And×1 Iff×1 | 3 |
| 100 | `def Set.prod` | 定义 | 项模式 · **定义体（数据/谓词构造，非证明）** | 2 | And×1 | — |
| 107 | `theorem mem_prod_iff` | **开放** | —（`:= sorry`） | 2 | And×1 Iff×1 | 3 |
| 134 | `theorem prod_set_swap_ne` | **开放** | —（`:= sorry`） | 2 | Not×2 | 3 |
| 150 | `def Set.graph` | 定义 | 项模式 · **定义体（数据/谓词构造，非证明）** | 2 | — | — |
| 155 | `theorem demo_graph_mem` | 已证 | 项模式 · Iff.intro 双向 | 4 | Iff.intro×1 Iff×1 | — |
| 168 | `theorem demo_mem_pair` | 已证 | 项模式 · Iff.intro 双向 | 4 | Iff.intro×1 Or×3 Iff×1 | — |
| 198 | `theorem kura_degenerate` | **开放** | —（`:= sorry`） | 2 | — | 3 |

#### `units/unit06-relations.sokonanoda` — 216 行 · 23 个声明 · import: `import lib.Logic`, `import lib.Exists`, `import lib.Set`, `import lib.Rel`

| 行 | 关键字 名字 | 状态 | 形态·形状 | 体行 | 逻辑构件 | hint |
|---:|---|---|---|---:|---|---:|
| 48 | `def Reflexive` | 定义 | 项模式 · **定义体（数据/谓词构造，非证明）** | 1 | forall×1 | — |
| 50 | `def Symmetric` | 定义 | 项模式 · **定义体（数据/谓词构造，非证明）** | 1 | forall×1 | — |
| 52 | `def Transitive` | 定义 | 项模式 · **定义体（数据/谓词构造，非证明）** | 1 | forall×1 | — |
| 55 | `def EmptyRelation` | 定义 | 项模式 · **定义体（数据/谓词构造，非证明）** | 1 | — | — |
| 58 | `theorem demo_eq_reflexive` | 已证 | 项模式 · fun 链（1 层）· Eq.refl | 2 | Eq.refl×1 | — |
| 62 | `theorem demo_empty_symmetric` | 已证 | 项模式 · fun 链（3 层）· 纯应用 | 2 | — | — |
| 66 | `theorem demo_empty_transitive` | 已证 | 项模式 · fun 链（5 层）· 纯应用 | 3 | — | — |
| 82 | `theorem rel_inv_inv` | **开放** | —（`:= sorry`） | 2 | — | 3 |
| 90 | `theorem rel_comp_assoc` | **开放** | —（`:= sorry`） | 2 | — | 3 |
| 103 | `theorem transitive_inv` | **开放** | —（`:= sorry`） | 2 | Iff×1 | 3 |
| 115 | `def EquivClass` | 定义 | 项模式 · **定义体（数据/谓词构造，非证明）** | 1 | — | — |
| 121 | `theorem mem_equivClass_self` | **开放** | —（`:= sorry`） | 2 | — | 3 |
| 129 | `theorem equivClass_eq_iff` | **开放** | —（`:= sorry`） | 2 | Iff×1 | 3 |
| 145 | `def classes` | 定义 | 项模式 · **定义体（数据/谓词构造，非证明）** | 2 | Exists×1 | — |
| 148 | `def IsPartition` | 定义 | 项模式 · **定义体（数据/谓词构造，非证明）** | 8 | And×4 Exists×2 forall×3 Not×3 | — |
| 161 | `theorem classes_isPartition` | **开放** | —（`:= sorry`） | 2 | — | 3 |
| 176 | `def Nat.isZero` | 定义 | 项模式 · **match 分支定义** | 3 | match×1 | — |
| 180 | `def Nat.pred` | 定义 | 项模式 · **match 分支定义** | 3 | match×1 | — |
| 184 | `theorem Nat.zero_ne_succ` | 已证 | 项模式 · fun 链（1 层）· Eq.subst 重写 + True.intro | 3 | Eq.subst×1 True.intro×1 Not×1 | — |
| 188 | `theorem Nat.succ.inj` | 已证 | 项模式 · congrArg 同余 | 2 | congrArg×1 | — |
| 193 | `def nearStep` | 定义 | 项模式 · **定义体（数据/谓词构造，非证明）** | 2 | Or×2 | — |
| 202 | `theorem nearStep_counterexample` | **开放** | —（`:= sorry`） | 2 | And×2 Not×1 | 3 |
| 213 | `theorem not_symm_trans_implies_refl` | **开放** | —（`:= sorry`） | 2 | forall×1 Not×1 | 3 |

#### `units/unit07-functions.sokonanoda` — 202 行 · 12 个声明 · import: `import lib.Logic`, `import lib.Set`, `import lib.Exists`, `import lib.Fun`

| 行 | 关键字 名字 | 状态 | 形态·形状 | 体行 | 逻辑构件 | hint |
|---:|---|---|---|---:|---|---:|
| 68 | `theorem demo_graph_single_valued` | 已证 | 项模式 · fun 链（5 层）· Eq.symm + Eq.trans | 4 | Eq.symm×1 Eq.trans×1 forall×3 | — |
| 76 | `theorem demo_comp_apply` | 已证 | 项模式 · 直接引用 `Function.comp_apply` | 2 | — | — |
| 81 | `theorem demo_bijective_def` | 已证 | 项模式 · 直接引用 `Function.bijective_def` | 2 | And×1 Iff×1 | — |
| 96 | `theorem comp_assoc` | **开放** | —（`:= sorry`） | 2 | — | 3 |
| 105 | `theorem injective_comp` | **开放** | —（`:= sorry`） | 2 | — | 3 |
| 114 | `theorem surjective_comp` | **开放** | —（`:= sorry`） | 2 | — | 3 |
| 127 | `theorem leftInverse_injective` | **开放** | —（`:= sorry`） | 2 | — | 3 |
| 135 | `theorem surjective_of_rightInverse` | **开放** | —（`:= sorry`） | 2 | — | 3 |
| 147 | `theorem bijective_iff_inverse` | **开放** | —（`:= sorry`） | 2 | Iff×1 | 3 |
| 156 | `theorem injective_of_comp_injective` | **开放** | —（`:= sorry`） | 2 | — | 3 |
| 184 | `theorem swap_exists_forall` | **开放** | —（`:= sorry`） | 2 | Exists×2 forall×2 | 3 |
| 198 | `theorem swap_converse_false` | **开放** | —（`:= sorry`） | 2 | Exists×2 forall×3 Not×1 | 3 |

#### `units/unit08-images-preimages.sokonanoda` — 351 行 · 27 个声明 · import: `import lib.Logic`, `import lib.Exists`, `import lib.Set`, `import lib.Image`

记法声明：`binder_notation "∃" => Exists`

| 行 | 关键字 名字 | 状态 | 形态·形状 | 体行 | 逻辑构件 | hint |
|---:|---|---|---|---:|---|---:|
| 53 | `theorem demo_mem_image` | 已证 | 项模式 · 直接引用 `Set.mem_image` | 2 | And×1 Iff×1 Exists×1 | — |
| 60 | `theorem demo_mem_preimage` | 已证 | 项模式 · 直接引用 `Set.mem_preimage` | 2 | Iff×1 | — |
| 72 | `theorem preimage_union` | **开放** | —（`:= sorry`） | 2 | — | 3 |
| 81 | `theorem preimage_compl` | **开放** | —（`:= sorry`） | 2 | — | 3 |
| 90 | `theorem preimage_inter` | **开放** | —（`:= sorry`） | 2 | — | 3 |
| 103 | `theorem image_union` | **开放** | —（`:= sorry`） | 2 | — | 3 |
| 112 | `theorem image_subset_iff` | **开放** | —（`:= sorry`） | 2 | Iff×1 | 3 |
| 118 | `theorem image_mono` | 已证 | 项模式 · 直接引用 `Set.image_mono` | 2 | — | — |
| 129 | `example example` | 已证 | 项模式 · 直接引用 `Set.image_mono` | 2 | — | — |
| 143 | `example example` | 已证 | 项模式 · 纯应用 | 2 | And×1 | — |
| 156 | `inductive Two` | 归纳定义 | —（只有签名） | 0 | — | — |
| 162 | `def isAa` | 定义 | 项模式 · **match 分支定义** | 3 | match×1 | — |
| 167 | `theorem two_aa_ne_bb` | 已证 | 项模式 · fun 链（1 层）· Eq.subst 重写 + True.intro | 3 | Eq.subst×1 True.intro×1 Not×1 | — |
| 172 | `def f1` | 定义 | 项模式 · **match 分支定义** | 3 | match×1 | — |
| 177 | `theorem f1_eq_aa` | 已证 | 项模式 · match 分支 + Eq.refl | 3 | Eq.refl×2 match×1 | — |
| 182 | `def U01` | 定义 | 项模式 · **定义体（数据/谓词构造，非证明）** | 1 | Or×1 | — |
| 188 | `def image_preimage` | 定义 | 项模式 · **定义体（数据/谓词构造，非证明）** | 1 | Exists×1 | — |
| 192 | `theorem inter_singletons_empty` | 已证 | 项模式 · `Set.ext` + `Iff.intro` + 投影 + `False.elim` + `Eq.symm/trans`（画布演示） | 16 | And.left×1 And.right×1 Iff.intro×1 False.elim×2 Eq.symm×1 Eq.trans×1 Set.ext×1 | — |
| 211 | `theorem image_empty` | 已证 | 项模式 · Iff.intro 双向 + Exists.elim 消去 + And.left/right 投影 + False.elim 爆炸 + Set.ext 外延 | 15 | And.left×1 Exists.elim×1 Iff.intro×1 False.elim×2 Set.ext×1 And×2 | — |
| 229 | `theorem image_inter_singletons_empty` | 已证 | 项模式 · congrArg 同余 + Eq.trans | 9 | congrArg×1 Eq.trans×1 | — |
| 244 | `example example` | 已证 | 项模式 · And.intro 配对 + Exists.intro 见证 + Eq.refl | 9 | And.intro×3 Exists.intro×2 Eq.refl×4 And×2 | — |
| 262 | `theorem image_inter_subset` | 已证 | 项模式 · `fun y => fun hy =>` + `Exists.elim` + `And.intro` + 投影 + `Exists.intro`（画布演示） | 17 | And.intro×3 And.left×3 And.right×3 Exists.intro×2 Exists.elim×1 And×5 | — |
| 295 | `theorem not_image_inter_eq_image_inter` | **开放** | —（`:= sorry`） | 2 | Not×1 | 3 |
| 304 | `theorem image_preimage_subset` | 已证 | 项模式 · fun 链（2 层）· Exists.elim 消去 + And.left/right 投影 + Eq.subst 重写 | 9 | And.left×1 And.right×1 Exists.elim×1 Eq.subst×1 And×2 | — |
| 320 | `theorem not_image_preimage_eq` | **开放** | —（`:= sorry`） | 2 | Not×1 | 3 |
| 329 | `theorem image_preimage_image_eq_preimage` | **开放** | —（`:= sorry`） | 2 | — | 3 |
| 337 | `theorem image_preimage_eq_of_subset_image_univ` | **开放** | —（`:= sorry`） | 2 | — | 3 |

#### `units/unit09-equinumerosity.sokonanoda` — 192 行 · 13 个声明 · import: `import lib.Logic`, `import lib.Exists`, `import lib.Set`, `import lib.Equiv`

| 行 | 关键字 名字 | 状态 | 形态·形状 | 体行 | 逻辑构件 | hint |
|---:|---|---|---|---:|---|---:|
| 48 | `theorem Set.Equiv.refl` | 已证 | 项模式 · Eq.refl | 6 | Eq.refl×2 | — |
| 63 | `theorem Set.Equiv.of_inj_surj` | 已证 | 项模式 · 直接引用 `Set.Equiv.mk` | 5 | forall×1 | — |
| 88 | `def Nat.isZero` | 定义 | 项模式 · **match 分支定义** | 3 | match×1 | — |
| 92 | `def Nat.pred` | 定义 | 项模式 · **match 分支定义** | 3 | match×1 | — |
| 96 | `theorem Nat.succ_ne_zero` | 已证 | 项模式 · fun 链（1 层）· Eq.subst 重写 + True.intro | 3 | Eq.subst×1 True.intro×1 Not×1 | — |
| 100 | `def nonzero` | 定义 | 项模式 · **定义体（数据/谓词构造，非证明）** | 1 | Not×1 | — |
| 113 | `theorem Set.Equiv.symm` | **开放** | —（`:= sorry`） | 2 | — | 3 |
| 123 | `theorem Set.Equiv.trans` | **开放** | —（`:= sorry`） | 2 | — | 3 |
| 133 | `theorem Set.Equiv.singleton` | **开放** | —（`:= sorry`） | 2 | — | 3 |
| 143 | `theorem Set.equivOfEq` | **开放** | —（`:= sorry`） | 2 | — | 3 |
| 153 | `theorem not_equiv_empty_singleton` | **开放** | —（`:= sorry`） | 2 | Not×1 | 3 |
| 164 | `theorem equiv_nonempty_iff` | **开放** | —（`:= sorry`） | 2 | Iff×1 Exists×2 | 3 |
| 178 | `theorem proper_subset_counterexample` | **开放** | —（`:= sorry`） | 2 | And×2 Not×1 | 3 |

#### `units/unit10-cantor.sokonanoda` — 197 行 · 12 个声明 · import: `import lib.Logic`, `import lib.Set`, `import lib.Exists`, `import lib.Fun`, `import lib.Equiv`

| 行 | 关键字 名字 | 状态 | 形态·形状 | 体行 | 逻辑构件 | hint |
|---:|---|---|---|---:|---|---:|
| 46 | `def fake_enum` | 定义 | 项模式 · **定义体（数据/谓词构造，非证明）** | 1 | — | — |
| 48 | `theorem demo_fake_enum_escapes` | 已证 | 项模式 · `fun h =>` + `Eq.subst` 命题级重写（假枚举逃逸演示） | 7 | Eq.subst×1 Eq.symm×1 Eq.refl×1 Not×5 | — |
| 68 | `theorem demo_diag` | 已证 | 项模式 · `fun h =>` + `congrArg` 命题级 + `Eq.subst` + `absurd`（对角线演示） | 16 | absurd×1 Eq.subst×2 congrArg×2 Not×11 | — |
| 96 | `def Set.Countable` | **开放** | —（`:= sorry`） | 2 | — | 3 |
| 111 | `theorem nat_equiv_nat` | **开放** | —（`:= sorry`） | 2 | — | 3 |
| 118 | `theorem diag_mem_iff` | **开放** | —（`:= sorry`） | 2 | Iff×1 Not×2 | 3 |
| 127 | `theorem cantor` | **开放** | —（`:= sorry`） | 2 | Not×1 | 3 |
| 140 | `theorem powerset_nat_not_countable` | **开放** | —（`:= sorry`） | 2 | Not×1 | 3 |
| 149 | `theorem no_surjection_powerset` | **开放** | —（`:= sorry`） | 2 | Exists×1 Not×1 | 3 |
| 170 | `axiom Exists.choose` | 公理 | —（只有签名） | 0 | Exists×1 | — |
| 172 | `axiom Exists.choose_spec` | 公理 | —（只有签名） | 0 | Exists×1 | — |
| 194 | `theorem choice_split` | **开放** | —（`:= sorry`） | 2 | Exists×1 | 3 |

#### `units/unit11-universe-russell.sokonanoda` — 203 行 · 8 个声明 · import: `import lib.Logic`, `import lib.Set`, `import lib.Exists`

| 行 | 关键字 名字 | 状态 | 形态·形状 | 体行 | 逻辑构件 | hint |
|---:|---|---|---|---:|---|---:|
| 72 | `theorem subset_univ` | 已证 | 项模式 · fun 链（2 层）· 纯应用 | 2 | — | — |
| 79 | `theorem univ_subset_iff` | 已证 | 项模式 · Iff.intro 双向 + Eq.subst 重写 + Set.ext 外延 + Eq.symm | 13 | Iff.intro×2 Eq.subst×1 Eq.symm×1 Set.ext×1 Iff×1 | — |
| 97 | `theorem exists_univ` | 已证 | 项模式 · Exists.intro 见证 | 3 | Exists.intro×1 Exists×1 forall×2 | — |
| 122 | `theorem no_univ_strictly_larger` | **开放** | —（`:= sorry`） | 2 | And×1 Exists×1 forall×1 Not×2 | 3 |
| 144 | `theorem mem_powerset_univ` | **开放** | —（`:= sorry`） | 2 | — | 3 |
| 169 | `theorem univ_mem_univ_of_sets` | **开放** | —（`:= sorry`） | 2 | — | 3 |
| 188 | `theorem univ_top_iff` | **开放** | —（`:= sorry`） | 2 | And×1 Iff×1 | 3 |
| 201 | `theorem powerset_univ_eq_univ_of_sets` | **开放** | —（`:= sorry`） | 2 | — | 3 |

#### `units/unit12-synthesis.sokonanoda` — 474 行 · 13 个声明 · import: `import lib.Logic`, `import lib.Set`, `import lib.Exists`, `import lib.Equiv`, `import lib.Rel`, `import lib.Fun`, `import lib.Image`

| 行 | 关键字 名字 | 状态 | 形态·形状 | 体行 | 逻辑构件 | hint |
|---:|---|---|---|---:|---|---:|
| 97 | `theorem demo_rel_comp_assoc` | 已证 | 项模式 · `Rel.ext` + `fun a d => Iff.intro` 双向 + `Exists.elim`×4 + `And.intro`/投影重组（画布同题演示） | 58 | And.intro×4 And.left×4 And.right×4 Exists.intro×4 Exists.elim×4 Iff.intro×1 Rel.ext×1 … | — |
| 161 | `theorem demo_injective_unpack` | 已证 | 项模式 · fun 链（3 层）· 纯应用 | 2 | forall×2 | — |
| 169 | `theorem demo_fun_comp_apply` | 已证 | 项模式 · 直接引用 `Function.comp_apply` | 2 | — | — |
| 190 | `theorem xlat_compl_union` | **开放** | —（`:= sorry`） | 2 | — | 3 |
| 204 | `theorem xlat_rel_inv_comp` | **开放** | —（`:= sorry`） | 2 | — | 3 |
| 217 | `theorem xlat_injective_comp` | **开放** | —（`:= sorry`） | 2 | — | 3 |
| 230 | `theorem xlat_preimage_comp` | **开放** | —（`:= sorry`） | 2 | — | 3 |
| 251 | `theorem fix_image_preimage` | **开放** | —（`:= sorry`） | 2 | — | 3 |
| 262 | `theorem fix_image_inter` | **开放** | —（`:= sorry`） | 2 | — | 3 |
| 278 | `theorem flawed_equalities_refuted` | **开放** | —（`:= sorry`） | 2 | And×1 Not×2 | 3 |
| 332 | `theorem demo_equiv_comp` | 已证 | 项模式 · `Exists.elim` 四连（深度 3：消 f1→g1→f2→g2）+ `And.left/right` 逐层取分量 + `congrArg` 组合左右逆 | 101 | And.left×8 And.right×8 Exists.elim×4 congrArg×2 Eq.trans×2 And×37 Exists×9 | — |
| 445 | `theorem project_chain` | **开放** | —（`:= sorry`） | 2 | And×1 | 3 |
| 469 | `theorem project_chain_cardinal` | **开放** | —（`:= sorry`） | 2 | And×1 Not×1 | 3 |

### 2.3 `units/solutions/`（解答钥匙，13 个文件）

#### `units/solutions/notation-cheatsheet-solution.sokonanoda` — 140 行 · 21 个声明 · import: `import lib.Logic`, `import lib.Set`

记法声明：`infix:50 " ∈ " => Set.mem`、`infix:50 " ⊆ " => Set.subset`、`infixl:65 " ∪ " => Set.union`、`notation "∅" => Set.empty`

| 行 | 关键字 名字 | 状态 | 形态·形状 | 体行 | 逻辑构件 | hint |
|---:|---|---|---|---:|---|---:|
| 35 | `theorem demo_mem_pointful` | 已证 | 项模式 · fun 链（1 层）· 纯应用 | 2 | — | — |
| 38 | `theorem demo_mem_notation` | 已证 | 项模式 · fun 链（1 层）· 纯应用 | 2 | — | — |
| 41 | `theorem demo_subset_pointful` | 已证 | 项模式 · fun 链（1 层）· 纯应用 | 2 | forall×1 | — |
| 45 | `theorem demo_subset_notation` | 已证 | 项模式 · fun 链（1 层）· 纯应用 | 2 | forall×1 | — |
| 53 | `theorem demo_trans_pointful` | 已证 | 项模式 · fun 链（2 层）· 纯应用 | 2 | — | — |
| 57 | `theorem demo_trans_notation` | 已证 | 项模式 · fun 链（2 层）· 纯应用 | 2 | — | — |
| 61 | `theorem demo_empty_subset_pointful` | 已证 | 项模式 · fun 链（2 层）· False.elim 爆炸 | 2 | False.elim×1 | — |
| 65 | `theorem demo_empty_subset_notation` | 已证 | 项模式 · fun 链（2 层）· False.elim 爆炸 | 2 | False.elim×1 | — |
| 68 | `theorem demo_notMem_empty_pointful` | 已证 | 项模式 · 直接引用 `Set.notMem_empty` | 2 | Not×1 | — |
| 72 | `theorem demo_notMem_empty_notation` | 已证 | 项模式 · 直接引用 `Set.notMem_empty` | 2 | Not×1 | — |
| 79 | `theorem demo_union_left_pointful` | 已证 | 项模式 · fun 链（2 层）· Or.inl/inr 注入 | 2 | Or.inl×1 | — |
| 83 | `theorem demo_union_left_notation` | 已证 | 项模式 · fun 链（2 层）· Or.inl/inr 注入 | 2 | Or.inl×1 | — |
| 86 | `theorem demo_mem_union_pointful` | 已证 | 项模式 · fun 链（1 层）· 纯应用 | 2 | Or×1 | — |
| 90 | `theorem demo_mem_union_notation` | 已证 | 项模式 · fun 链（1 层）· 纯应用 | 2 | Or×1 | — |
| 94 | `theorem demo_ext_pointful` | 已证 | 项模式 · Iff.intro 双向 + Set.ext 外延 | 2 | Iff.intro×1 Set.ext×1 | — |
| 98 | `theorem demo_ext_notation` | 已证 | 项模式 · Iff.intro 双向 + Set.ext 外延 | 2 | Iff.intro×1 Set.ext×1 | — |
| 106 | `theorem demo_precedence_pointful` | 已证 | 项模式 · fun 链（2 层）· Or.inl/inr 注入 | 2 | Or.inl×2 Or×1 | — |
| 110 | `theorem demo_precedence_notation` | 已证 | 项模式 · fun 链（2 层）· Or.inl/inr 注入 | 2 | Or.inl×2 Or×1 | — |
| 118 | `theorem empty_union_subset` | 已证 | 项模式 · fun 链（2 层）· Or.elim 两支 + False.elim 爆炸 | 5 | Or.elim×1 False.elim×1 | — |
| 125 | `theorem mem_union_comm` | 已证 | 项模式 · fun 链（1 层）· Or.elim 两支 + Or.inl/inr 注入 | 5 | Or.inl×1 Or.inr×1 Or.elim×1 | — |
| 133 | `theorem union_empty_right` | 已证 | 项模式 · Iff.intro 双向 + Or.elim 两支 + Or.inl/inr 注入 + False.elim 爆炸 + Set.ext 外延 | 8 | Or.inl×1 Or.elim×1 Iff.intro×1 False.elim×1 Set.ext×1 | — |

#### `units/solutions/unit01-solution.sokonanoda` — 40 行 · 6 个声明 · import: `import lib.Logic`, `import lib.Set`

| 行 | 关键字 名字 | 状态 | 形态·形状 | 体行 | 逻辑构件 | hint |
|---:|---|---|---|---:|---|---:|
| 6 | `theorem mem_of_subset` | 已证 | 项模式 · fun 链（1 层）· 纯应用 | 2 | — | — |
| 10 | `theorem eq_of_same_elements` | 已证 | 项模式 · Set.ext 外延 | 2 | Set.ext×1 Iff×1 forall×1 | — |
| 14 | `theorem mem_of_subset_singleton` | 已证 | 项模式 · 直接引用 `h` | 2 | — | — |
| 18 | `theorem subset_of_mem_singleton` | 已证 | 项模式 · fun 链（2 层）· Eq.subst 重写 + Eq.symm | 3 | Eq.subst×1 Eq.symm×1 | — |
| 23 | `theorem singleton_subset_iff` | 已证 | 项模式 · Iff.intro 双向 | 4 | Iff.intro×1 Iff×1 | — |
| 29 | `theorem singleton_eq_singleton_iff` | 已证 | 项模式 · Iff.intro 双向 + Eq.subst 重写 + Set.ext 外延 + Iff.mp/mpr 单向 + Eq.symm | 11 | Iff.intro×2 Iff.mp×1 Eq.subst×1 Eq.symm×1 Eq.trans×2 Set.ext×1 Iff×1 | — |

#### `units/solutions/unit02-solution.sokonanoda` — 84 行 · 10 个声明 · import: `import lib.Logic`, `import lib.Set`

| 行 | 关键字 名字 | 状态 | 形态·形状 | 体行 | 逻辑构件 | hint |
|---:|---|---|---|---:|---|---:|
| 7 | `theorem subset_refl` | 已证 | 项模式 · fun 链（2 层）· 纯应用 | 2 | — | — |
| 10 | `theorem subset_trans` | 已证 | 项模式 · fun 链（2 层）· 纯应用 | 2 | — | — |
| 14 | `theorem subset_antisymm` | 已证 | 项模式 · Iff.intro 双向 + Set.ext 外延 | 2 | Iff.intro×1 Set.ext×1 | — |
| 18 | `theorem empty_subset` | 已证 | 项模式 · fun 链（2 层）· False.elim 爆炸 | 2 | False.elim×1 | — |
| 21 | `theorem subset_empty_iff` | 已证 | 项模式 · Iff.intro 双向 + Eq.subst 重写 + Eq.symm | 8 | Iff.intro×1 Eq.subst×1 Eq.symm×1 Iff×1 | — |
| 31 | `theorem eq_empty_iff_forall_notMem` | 已证 | 项模式 · Iff.intro 双向 + False.elim 爆炸 + Eq.subst 重写 + Eq.symm | 11 | Iff.intro×1 False.elim×1 Eq.subst×1 Eq.symm×1 Iff×1 forall×3 Not×4 | — |
| 44 | `theorem singleton_ne_empty` | 已证 | 项模式 · fun 链（1 层）· Eq.subst 重写 | 5 | Eq.subst×1 Not×1 | — |
| 51 | `theorem pair_subset_iff` | 已证 | 项模式 · `Iff.intro` 双向 + `Or.elim` 两支 + `And.intro`/投影 + `Eq.subst`/`Eq.symm` | 16 | And.intro×1 And.left×1 And.right×1 Or.inl×1 Or.inr×1 Or.elim×1 Iff.intro×1 … | — |
| 69 | `theorem pair_comm` | 已证 | 项模式 · Iff.intro 双向 + Or.elim 两支 + Or.inl/inr 注入 + Set.ext 外延 | 11 | Or.inl×2 Or.inr×2 Or.elim×2 Iff.intro×1 Set.ext×1 Or×6 | — |
| 82 | `theorem singleton_of_singleton` | 已证 | 项模式 · Eq.refl | 2 | Eq.refl×1 | — |

#### `units/solutions/unit03-solution.sokonanoda` — 88 行 · 11 个声明 · import: `import lib.Logic`, `import lib.Set`

| 行 | 关键字 名字 | 状态 | 形态·形状 | 体行 | 逻辑构件 | hint |
|---:|---|---|---|---:|---|---:|
| 9 | `theorem subset_union_left` | 已证 | 项模式 · fun 链（2 层）· Or.inl/inr 注入 | 2 | Or.inl×1 | — |
| 12 | `theorem subset_union_right` | 已证 | 项模式 · fun 链（2 层）· Or.inl/inr 注入 | 2 | Or.inr×1 | — |
| 15 | `theorem union_subset_iff` | 已证 | 项模式 · Iff.intro 双向 + Or.elim 两支 + And.intro 配对 + And.left/right 投影 + Or.inl/inr 注入 | 12 | And.intro×1 And.left×1 And.right×1 Or.inl×1 Or.inr×1 Or.elim×1 Iff.intro×1 … | — |
| 30 | `theorem inter_subset_left` | 已证 | 项模式 · fun 链（2 层）· And.left/right 投影 | 2 | And.left×1 | — |
| 33 | `theorem inter_subset_right` | 已证 | 项模式 · fun 链（2 层）· And.left/right 投影 | 2 | And.right×1 | — |
| 36 | `theorem subset_inter` | 已证 | 项模式 · fun 链（2 层）· And.intro 配对 | 2 | And.intro×1 | — |
| 42 | `theorem subset_inter_iff` | 已证 | 项模式 · Iff.intro 双向 + And.intro 配对 + And.left/right 投影 | 12 | And.intro×2 And.left×2 And.right×2 Iff.intro×1 And×3 Iff×1 | — |
| 57 | `theorem sdiff_subset` | 已证 | 项模式 · fun 链（2 层）· And.left/right 投影 | 3 | And.left×1 Not×1 | — |
| 62 | `theorem powerset_mono` | 已证 | 项模式 · fun 链（2 层）· Iff.mp/mpr 单向 | 7 | Iff.mp×1 Iff.mpr×1 | — |
| 74 | `theorem union_subset_inter_false` | 已证 | 项模式 · fun 链（1 层）· And.left/right 投影 + Or.inl/inr 注入 | 6 | And.right×1 Or.inl×1 Not×1 | — |
| 84 | `theorem powerset_self_mem` | 已证 | 项模式 · Iff.mp/mpr 单向 | 4 | Iff.mpr×1 | — |

#### `units/solutions/unit04-solution.sokonanoda` — 166 行 · 8 个声明 · import: `import lib.Logic`, `import lib.Set`

| 行 | 关键字 名字 | 状态 | 形态·形状 | 体行 | 逻辑构件 | hint |
|---:|---|---|---|---:|---|---:|
| 14 | `theorem union_comm` | 已证 | 项模式 · Iff.intro 双向 + Or.elim 两支 + Or.inl/inr 注入 + Set.ext 外延 | 13 | Or.inl×2 Or.inr×2 Or.elim×2 Iff.intro×1 Set.ext×1 Or×6 | — |
| 30 | `theorem inter_comm` | 已证 | 项模式 · Iff.intro 双向 + And.intro 配对 + And.left/right 投影 + Set.ext 外延 | 11 | And.intro×2 And.left×2 And.right×2 Iff.intro×1 Set.ext×1 And×4 | — |
| 44 | `theorem union_inter_idem` | 已证 | 项模式 · `Set.ext` + `Iff.intro` 双向 + `Or.elim` 两支 + `And.intro`/投影 + `Or.inl/inr` | 16 | And.intro×2 And.left×1 Or.inl×1 Or.elim×1 Iff.intro×2 Set.ext×2 And×3 … | — |
| 63 | `theorem inter_union_distrib_left` | 已证 | 项模式 · `Set.ext` + `Iff.intro` 双向 + `Or.elim` 两支 + `And.intro`/投影 + 分配律三支重组 | 25 | And.intro×4 And.left×4 And.right×3 Or.inl×2 Or.inr×2 Or.elim×2 Iff.intro×1 … | — |
| 92 | `theorem union_inter_self` | 已证 | 项模式 · Iff.intro 双向 + Or.elim 两支 + And.left/right 投影 + Or.inl/inr 注入 + Set.ext 外延 | 9 | And.left×1 Or.inl×1 Or.elim×1 Iff.intro×1 Set.ext×1 And×5 Or×2 | — |
| 104 | `theorem sdiff_sdiff` | 已证 | 项模式 · `Set.ext` + `Iff.intro` 双向 + `Or.elim` 两支 + `And.intro`/投影 + `Or.inl/inr`（`sdiff_sdiff`） | 26 | And.intro×3 And.left×4 And.right×4 Or.inl×1 Or.inr×1 Or.elim×1 Iff.intro×1 … | — |
| 134 | `theorem union_empty` | 已证 | 项模式 · Iff.intro 双向 + Or.elim 两支 + Or.inl/inr 注入 + False.elim 爆炸 + Set.ext 外延 | 9 | Or.inl×1 Or.elim×1 Iff.intro×1 False.elim×1 Set.ext×1 Or×2 | — |
| 148 | `theorem union_sdiff_univ_ne_univ` | 已证 | 项模式 · `fun h =>` + `And.left/right` 投影 + `Eq.subst` + `Eq.symm` | 16 | And.right×1 Eq.subst×1 Eq.symm×1 Not×2 | — |

#### `units/solutions/unit05-solution.sokonanoda` — 123 行 · 8 个声明 · import: `import lib.Logic`, `import lib.Set`, `import lib.Prod`

| 行 | 关键字 名字 | 状态 | 形态·形状 | 体行 | 逻辑构件 | hint |
|---:|---|---|---|---:|---|---:|
| 20 | `def Set.prod` | 定义 | 项模式 · **定义体（数据/谓词构造，非证明）** | 2 | And×1 | — |
| 24 | `theorem prod_fst_mk` | 已证 | 项模式 · 直接引用 `Prod.fst_mk` | 2 | — | — |
| 29 | `theorem prod_snd_mk` | 已证 | 项模式 · 直接引用 `Prod.snd_mk` | 2 | — | — |
| 34 | `theorem prod_mk_inj` | 已证 | 项模式 · fun 链（1 层）· And.intro 配对 + Eq.subst 重写 + Eq.refl | 7 | And.intro×1 Eq.subst×2 Eq.refl×2 And×1 | — |
| 45 | `theorem prod_mk_eq_iff` | 已证 | 项模式 · Iff.intro 双向 + And.left/right 投影 + Eq.subst 重写 + Eq.refl | 12 | And.left×1 And.right×1 Iff.intro×1 Eq.subst×2 Eq.refl×1 And×3 Iff×1 | — |
| 61 | `theorem mem_prod_iff` | 已证 | 项模式 · Iff.intro 双向 | 5 | Iff.intro×1 And×3 Iff×1 | — |
| 71 | `theorem prod_set_swap_ne` | 已证 | 项模式 · fun 链（1 层）· And.intro 配对 + And.left/right 投影 + Eq.subst 重写 | 12 | And.intro×1 And.left×1 Eq.subst×1 Not×2 | — |
| 90 | `theorem kura_degenerate` | 已证 | 项模式 · `Set.ext` + `Iff.intro` 双向 + `Or.elim` 两支 + `Or.inl/inr` + `Eq.symm/trans`（Kuratowski 退化情形） | 31 | Or.inl×2 Or.elim×2 Iff.intro×2 Eq.symm×1 Eq.trans×2 Set.ext×2 Or×4 | — |

#### `units/solutions/unit06-solution.sokonanoda` — 371 行 · 23 个声明 · import: `import lib.Logic`, `import lib.Exists`, `import lib.Set`, `import lib.Rel`

| 行 | 关键字 名字 | 状态 | 形态·形状 | 体行 | 逻辑构件 | hint |
|---:|---|---|---|---:|---|---:|
| 20 | `def Reflexive` | 定义 | 项模式 · **定义体（数据/谓词构造，非证明）** | 1 | forall×1 | — |
| 22 | `def Symmetric` | 定义 | 项模式 · **定义体（数据/谓词构造，非证明）** | 1 | forall×1 | — |
| 24 | `def Transitive` | 定义 | 项模式 · **定义体（数据/谓词构造，非证明）** | 1 | forall×1 | — |
| 26 | `def EmptyRelation` | 定义 | 项模式 · **定义体（数据/谓词构造，非证明）** | 1 | — | — |
| 28 | `theorem demo_eq_reflexive` | 已证 | 项模式 · fun 链（1 层）· Eq.refl | 2 | Eq.refl×1 | — |
| 31 | `theorem demo_empty_symmetric` | 已证 | 项模式 · fun 链（3 层）· 纯应用 | 2 | — | — |
| 34 | `theorem demo_empty_transitive` | 已证 | 项模式 · fun 链（5 层）· 纯应用 | 3 | — | — |
| 42 | `theorem rel_inv_inv` | 已证 | 项模式 · Eq.refl | 2 | Eq.refl×1 | — |
| 52 | `theorem rel_comp_assoc` | 已证 | 项模式 · `Rel.ext` + `fun a d => Iff.intro` 双向；每向 `fun h =>` + `Exists.elim` 两连 + `And.intro`/投影重组 | 56 | And.intro×4 And.left×4 And.right×4 Exists.intro×4 Exists.elim×4 Iff.intro×1 Rel.ext×1 … | — |
| 116 | `theorem transitive_inv` | 已证 | 项模式 · Iff.intro 双向 | 6 | Iff.intro×1 Iff×1 | — |
| 127 | `def EquivClass` | 定义 | 项模式 · **定义体（数据/谓词构造，非证明）** | 1 | — | — |
| 130 | `theorem mem_equivClass_self` | 已证 | 项模式 · 直接引用 `hrefl` | 2 | — | — |
| 139 | `theorem equivClass_eq_iff` | 已证 | 项模式 · Iff.intro 双向 + Eq.subst 重写 + Set.ext 外延 | 13 | Iff.intro×2 Eq.subst×1 Set.ext×1 Iff×1 | — |
| 161 | `def classes` | 定义 | 项模式 · **定义体（数据/谓词构造，非证明）** | 2 | Exists×1 | — |
| 164 | `def IsPartition` | 定义 | 项模式 · **定义体（数据/谓词构造，非证明）** | 8 | And×4 Exists×2 forall×3 Not×3 | — |
| 182 | `theorem classes_isPartition` | 已证 | 项模式 · `And.intro` 三层（IsPartition 三支）+ `fun C hC =>` 后 `Exists.elim`×4（深度 3）+ `let` + `Eq.subst`/`Iff.mp/mpr` | 84 | And.intro×3 And.left×1 And.right×1 Exists.intro×2 Exists.elim×4 Iff.mpr×1 Eq.subst×3 … | — |
| 277 | `def Nat.isZero` | 定义 | 项模式 · **match 分支定义** | 3 | match×1 | — |
| 281 | `def Nat.pred` | 定义 | 项模式 · **match 分支定义** | 3 | match×1 | — |
| 285 | `theorem Nat.zero_ne_succ` | 已证 | 项模式 · fun 链（1 层）· Eq.subst 重写 + True.intro | 3 | Eq.subst×1 True.intro×1 Not×1 | — |
| 289 | `theorem Nat.succ.inj` | 已证 | 项模式 · congrArg 同余 | 2 | congrArg×1 | — |
| 295 | `def nearStep` | 定义 | 项模式 · **定义体（数据/谓词构造，非证明）** | 2 | Or×2 | — |
| 303 | `theorem nearStep_counterexample` | 已证 | 项模式 · `And.intro` 两层 + `fun a b h =>` + `Or.elim` 两支（含 `let` 局部辅助）+ `Or.inl/inr` 注入 | 54 | And.intro×2 Or.inl×5 Or.inr×5 Or.elim×4 Eq.symm×3 Eq.refl×3 let×3 … | — |
| 365 | `theorem not_symm_trans_implies_refl` | 已证 | 项模式 · fun 链（1 层）· 纯应用 | 5 | forall×2 Not×1 | — |

#### `units/solutions/unit07-solution.sokonanoda` — 126 行 · 9 个声明 · import: `import lib.Logic`, `import lib.Set`, `import lib.Exists`, `import lib.Fun`

| 行 | 关键字 名字 | 状态 | 形态·形状 | 体行 | 逻辑构件 | hint |
|---:|---|---|---|---:|---|---:|
| 18 | `theorem comp_assoc` | 已证 | 项模式 · Eq.refl | 2 | Eq.refl×1 | — |
| 24 | `theorem injective_comp` | 已证 | 项模式 · fun 链（3 层）· 纯应用 | 4 | — | — |
| 32 | `theorem surjective_comp` | 已证 | 项模式 · fun 链（1 层）· Exists.elim 消去 + Exists.intro 见证 + congrArg 同余 + Eq.trans | 12 | Exists.intro×1 Exists.elim×2 congrArg×1 Eq.trans×1 Exists×2 | — |
| 48 | `theorem leftInverse_injective` | 已证 | 项模式 · fun 链（3 层）· congrArg 同余 + Eq.symm + Eq.trans | 7 | congrArg×1 Eq.symm×1 Eq.trans×2 | — |
| 58 | `theorem surjective_of_rightInverse` | 已证 | 项模式 · fun 链（1 层）· Exists.intro 见证 | 2 | Exists.intro×1 | — |
| 66 | `theorem bijective_iff_inverse` | 已证 | 项模式 · Iff.intro 双向 + And.intro 配对 + And.left/right 投影 | 14 | And.intro×2 And.left×2 And.right×1 Iff.intro×1 Iff×1 | — |
| 84 | `theorem injective_of_comp_injective` | 已证 | 项模式 · fun 链（3 层）· congrArg 同余 | 3 | congrArg×1 | — |
| 90 | `theorem swap_exists_forall` | 已证 | 项模式 · fun 链（1 层）· Exists.elim 消去 + Exists.intro 见证 | 6 | Exists.intro×1 Exists.elim×1 Exists×3 forall×4 | — |
| 104 | `theorem swap_converse_false` | 已证 | 项模式 · `fun h =>` + `Exists.elim` + `Exists.intro` + `Eq.subst`（量词交换反例） | 20 | Exists.intro×1 Exists.elim×1 Eq.subst×1 Eq.symm×1 Eq.trans×1 Eq.refl×1 Exists×5 … | — |

#### `units/solutions/unit08-solution.sokonanoda` — 462 行 · 30 个声明 · import: `import lib.Logic`, `import lib.Exists`, `import lib.Set`, `import lib.Image`

| 行 | 关键字 名字 | 状态 | 形态·形状 | 体行 | 逻辑构件 | hint |
|---:|---|---|---|---:|---|---:|
| 23 | `theorem demo_mem_image` | 已证 | 项模式 · 直接引用 `Set.mem_image` | 2 | And×1 Iff×1 Exists×1 | — |
| 28 | `theorem demo_mem_preimage` | 已证 | 项模式 · 直接引用 `Set.mem_preimage` | 2 | Iff×1 | — |
| 36 | `theorem preimage_union` | 已证 | 项模式 · `Set.ext` + `Iff.intro` 双向 + `Or.elim` 两支 + `Or.inl/inr`（原像保并） | 22 | Or.inl×2 Or.inr×2 Or.elim×2 Iff.intro×1 Set.ext×1 Or×3 | — |
| 65 | `theorem preimage_compl` | 已证 | 项模式 · Iff.intro 双向 + Set.ext 外延 | 9 | Iff.intro×1 Set.ext×1 Not×2 | — |
| 81 | `theorem preimage_inter` | 已证 | 项模式 · Iff.intro 双向 + And.intro 配对 + And.left/right 投影 + Set.ext 外延 | 14 | And.intro×2 And.left×2 And.right×2 Iff.intro×1 Set.ext×1 And×2 | — |
| 102 | `theorem image_union` | 已证 | 项模式 · `Set.ext` + `fun y => Iff.intro` 双向 + `Exists.elim`×3 + 内层 `Or.elim` 两支 + 投影 + `Or.inl/inr`/`Exists.intro` 重建 | 45 | And.intro×4 And.left×3 And.right×4 Or.inl×2 Or.inr×2 Or.elim×2 Exists.intro×4 … | — |
| 154 | `theorem image_subset_iff` | 已证 | 项模式 · 直接引用 `Set.image_subset_iff` | 2 | Iff×1 | — |
| 162 | `theorem image_inter_subset` | 已证 | 项模式 · `fun y => fun hy =>` + `Exists.elim` + `And.intro` + 投影 + `Exists.intro`（像的单调性） | 17 | And.intro×3 And.left×3 And.right×3 Exists.intro×2 Exists.elim×1 And×5 | — |
| 183 | `theorem image_mono` | 已证 | 项模式 · 直接引用 `Set.image_mono` | 2 | — | — |
| 191 | `inductive Two` | 归纳定义 | —（只有签名） | 0 | — | — |
| 198 | `def isAa` | 定义 | 项模式 · **match 分支定义** | 3 | match×1 | — |
| 202 | `theorem two_aa_ne_bb` | 已证 | 项模式 · fun 链（1 层）· Eq.subst 重写 + True.intro | 3 | Eq.subst×1 True.intro×1 Not×1 | — |
| 207 | `def f1` | 定义 | 项模式 · **match 分支定义** | 3 | match×1 | — |
| 212 | `theorem f1_eq_aa` | 已证 | 项模式 · match 分支 + Eq.refl | 3 | Eq.refl×2 match×1 | — |
| 216 | `def U01` | 定义 | 项模式 · **定义体（数据/谓词构造，非证明）** | 1 | Or×1 | — |
| 218 | `theorem bb_mem_U01` | 已证 | 项模式 · Or.inl/inr 注入 + Eq.refl | 2 | Or.inr×1 Eq.refl×1 | — |
| 221 | `theorem inter_singletons_empty` | 已证 | 项模式 · `Set.ext` + `Iff.intro` + 投影 + `False.elim` + `Eq.symm/trans` | 16 | And.left×1 And.right×1 Iff.intro×1 False.elim×2 Eq.symm×1 Eq.trans×1 Set.ext×1 | — |
| 240 | `theorem image_empty` | 已证 | 项模式 · Iff.intro 双向 + Exists.elim 消去 + And.left/right 投影 + False.elim 爆炸 + Set.ext 外延 | 15 | And.left×1 Exists.elim×1 Iff.intro×1 False.elim×2 Set.ext×1 And×2 | — |
| 257 | `theorem image_inter_singletons_empty` | 已证 | 项模式 · congrArg 同余 + Eq.trans | 9 | congrArg×1 Eq.trans×1 | — |
| 271 | `theorem aa_mem_inter_image` | 已证 | 项模式 · And.intro 配对 + Exists.intro 见证 + Eq.refl | 9 | And.intro×3 Exists.intro×2 Eq.refl×4 And×2 | — |
| 285 | `theorem not_image_inter_eq_image_inter` | 已证 | 项模式 · `fun h =>` + `Eq.subst`/`Eq.symm`/`Eq.trans` 链（反例：像不保交） | 21 | Eq.subst×1 Eq.symm×1 Eq.trans×1 Not×1 | — |
| 315 | `theorem image_preimage_subset` | 已证 | 项模式 · fun 链（2 层）· Exists.elim 消去 + And.left/right 投影 + Eq.subst 重写 | 9 | And.left×1 And.right×1 Exists.elim×1 Eq.subst×1 And×2 | — |
| 329 | `def image_preimage` | 定义 | 项模式 · **定义体（数据/谓词构造，非证明）** | 2 | Exists×1 | — |
| 332 | `theorem image_preimage_eq_aa` | 已证 | 项模式 · Exists.elim 消去 + Eq.symm + Eq.trans | 4 | Exists.elim×1 Eq.symm×1 Eq.trans×1 | — |
| 338 | `theorem bb_not_mem_image_preimage` | 已证 | 项模式 · fun 链（1 层）· Exists.elim 消去 + Eq.symm + Eq.trans | 6 | Exists.elim×1 Eq.symm×1 Eq.trans×1 Not×1 | — |
| 345 | `theorem image_preimage_eq_image` | 已证 | 项模式 · `Set.ext` + `Iff.intro` 双向 + `Exists.elim` 两连 + `And.intro`/`Exists.intro` + `Or.inl/inr` | 25 | And.intro×1 And.right×1 Or.inl×1 Exists.intro×2 Exists.elim×2 Iff.intro×1 Set.ext×1 … | — |
| 374 | `theorem eq_of_subsets` | 已证 | 项模式 · Iff.intro 双向 + Set.ext 外延 | 2 | Iff.intro×1 Set.ext×1 | — |
| 379 | `theorem image_preimage_image_eq_preimage` | 已证 | 项模式 · `Exists.elim` 两连 + `And.intro` + 投影 + `Or.inl/inr` + `Exists.intro` | 23 | And.intro×1 And.right×1 Or.inl×1 Exists.intro×2 Exists.elim×2 And×3 | — |
| 404 | `theorem not_image_preimage_eq` | 已证 | 项模式 · fun 链（1 层）· Eq.subst 重写 + Eq.symm | 11 | Eq.subst×2 Eq.symm×2 Not×1 | — |
| 418 | `theorem image_preimage_eq_of_subset_image_univ` | 已证 | 项模式 · `Set.ext` + `fun y => Iff.intro`；正向**直接引用引理** `image_preimage_subset`，反向 `Iff.mpr` + `Exists.elim` + `Exists.intro` + `Eq.subst` | 21 | And.intro×1 And.right×2 Exists.intro×1 Exists.elim×1 Iff.intro×1 Iff.mpr×1 Eq.subst×1 … | — |

#### `units/solutions/unit09-solution.sokonanoda` — 396 行 · 13 个声明 · import: `import lib.Logic`, `import lib.Exists`, `import lib.Set`, `import lib.Equiv`

| 行 | 关键字 名字 | 状态 | 形态·形状 | 体行 | 逻辑构件 | hint |
|---:|---|---|---|---:|---|---:|
| 22 | `theorem Set.Equiv.refl` | 已证 | 项模式 · Eq.refl | 6 | Eq.refl×2 | — |
| 33 | `theorem Set.Equiv.of_inj_surj` | 已证 | 项模式 · 直接引用 `Set.Equiv.mk` | 5 | forall×1 | — |
| 51 | `def Nat.isZero` | 定义 | 项模式 · **match 分支定义** | 3 | match×1 | — |
| 55 | `def Nat.pred` | 定义 | 项模式 · **match 分支定义** | 3 | match×1 | — |
| 61 | `theorem Nat.succ_ne_zero` | 已证 | 项模式 · fun 链（1 层）· Eq.subst 重写 + True.intro | 3 | Eq.subst×1 True.intro×1 Not×1 | — |
| 66 | `def nonzero` | 定义 | 项模式 · **定义体（数据/谓词构造，非证明）** | 1 | Not×1 | — |
| 72 | `theorem Set.Equiv.symm` | 已证 | 项模式 · `Exists.elim` 两连（深度 1）+ `And.left/right` 投影 + `Exists.intro` 重建（Equiv 两半对调） | 39 | And.left×3 And.right×4 Exists.elim×2 And×17 Exists×4 | — |
| 118 | `theorem Set.Equiv.trans` | 已证 | 项模式 · `Exists.elim` 四连（深度 3）+ `And.left/right` 投影 + `congrArg`×2 + `Eq.trans` 串左右逆 | 103 | And.left×8 And.right×8 Exists.elim×4 congrArg×2 Eq.trans×2 And×37 Exists×9 | — |
| 228 | `theorem Set.Equiv.singleton` | 已证 | 项模式 · Eq.refl + Eq.symm | 7 | Eq.symm×2 Eq.refl×2 | — |
| 242 | `theorem Set.equivOfEq` | 已证 | 项模式 · Eq.subst 重写 + Eq.refl + Eq.symm | 9 | Eq.subst×2 Eq.symm×1 Eq.refl×2 | — |
| 258 | `theorem not_equiv_empty_singleton` | 已证 | 项模式 · `fun h =>` + `Exists.elim` 两连 + `And.left/right` 投影 + `False.elim` 收尾 | 36 | And.left×1 And.right×1 Exists.elim×2 And×13 Exists×3 Not×1 | — |
| 301 | `theorem equiv_nonempty_iff` | 已证 | 项模式 · `Iff.intro` 双向；每向 `fun ha =>` + `Exists.elim` 两连（深度 2）+ `And.left/right` 投影 + `Exists.intro` 重建 | 54 | And.left×2 And.right×1 Exists.intro×2 Exists.elim×5 Iff.intro×1 And×21 Iff×1 … | — |
| 366 | `theorem proper_subset_counterexample` | 已证 | 项模式 · `And.intro` + `fun n =>` + `Nat.rec` 消去 + `Eq.subst` + `False.elim`（反例构造） | 28 | And.intro×2 False.elim×1 Eq.subst×1 Eq.symm×1 Eq.refl×4 Nat.rec×1 True.intro×2 … | — |

#### `units/solutions/unit10-solution.sokonanoda` — 136 行 · 9 个声明 · import: `import lib.Logic`, `import lib.Set`, `import lib.Exists`, `import lib.Fun`, `import lib.Equiv`

| 行 | 关键字 名字 | 状态 | 形态·形状 | 体行 | 逻辑构件 | hint |
|---:|---|---|---|---:|---|---:|
| 18 | `def Set.Countable` | 定义 | 项模式 · **定义体（数据/谓词构造，非证明）** | 2 | — | — |
| 23 | `theorem nat_equiv_nat` | 已证 | 项模式 · Eq.refl + True.intro | 8 | Eq.refl×2 True.intro×2 | — |
| 33 | `theorem diag_mem_iff` | 已证 | 项模式 · 直接引用 `Iff.refl` | 2 | Iff×1 Not×3 | — |
| 44 | `theorem cantor` | 已证 | 项模式 · `fun h =>` + `let MF/UF` + `Exists.elim`×2 + `congrArg` 命题级 + `absurd`（Cantor 对角线） | 45 | And.right×3 Exists.elim×2 absurd×1 Eq.subst×2 congrArg×1 Eq.symm×1 True.intro×1 … | — |
| 92 | `theorem powerset_nat_not_countable` | 已证 | 项模式 · 直接引用 `cantor` | 2 | Not×1 | — |
| 98 | `theorem no_surjection_powerset` | 已证 | 项模式 · `fun h =>` + `let` + `Exists.elim` 两连 + `congrArg` 命题级 + `absurd` | 17 | Exists.elim×2 absurd×1 Eq.subst×2 congrArg×1 Eq.symm×1 let×4 Exists×2 … | — |
| 127 | `axiom Exists.choose` | 公理 | —（只有签名） | 0 | Exists×1 | — |
| 128 | `axiom Exists.choose_spec` | 公理 | —（只有签名） | 0 | Exists×1 | — |
| 131 | `theorem choice_split` | 已证 | 项模式 · Exists.intro 见证 + let 局部定义 | 4 | Exists.intro×1 let×1 Exists×1 | — |

#### `units/solutions/unit11-solution.sokonanoda` — 87 行 · 5 个声明 · import: `import lib.Logic`, `import lib.Set`, `import lib.Exists`

| 行 | 关键字 名字 | 状态 | 形态·形状 | 体行 | 逻辑构件 | hint |
|---:|---|---|---|---:|---|---:|
| 21 | `theorem no_univ_strictly_larger` | 已证 | 项模式 · fun 链（1 层）· Exists.elim 消去 + And.left/right 投影 + Eq.refl | 12 | And.right×1 Exists.elim×1 Eq.refl×1 And×4 Exists×2 forall×4 Not×6 | — |
| 41 | `theorem mem_powerset_univ` | 已证 | 项模式 · Iff.mp/mpr 单向 | 4 | Iff.mpr×1 | — |
| 50 | `theorem univ_mem_univ_of_sets` | 已证 | 项模式 · 直接引用 `Set.mem_univ` | 2 | — | — |
| 58 | `theorem univ_top_iff` | 已证 | 项模式 · `Iff.intro` 双向 + `And.intro` + `Eq.subst` + `Set.ext` | 16 | And.intro×1 Iff.intro×2 Eq.subst×1 Eq.symm×1 Set.ext×1 And×1 Iff×2 | — |
| 80 | `theorem powerset_univ_eq_univ_of_sets` | 已证 | 项模式 · Iff.intro 双向 + Set.ext 外延 | 7 | Iff.intro×1 Set.ext×1 | — |

#### `units/solutions/unit12-solution.sokonanoda` — 584 行 · 9 个声明 · import: `import lib.Logic`, `import lib.Set`, `import lib.Exists`, `import lib.Equiv`, `import lib.Rel`, `import lib.Fun`, `import lib.Image`

| 行 | 关键字 名字 | 状态 | 形态·形状 | 体行 | 逻辑构件 | hint |
|---:|---|---|---|---:|---|---:|
| 24 | `theorem xlat_compl_union` | 已证 | 项模式 · `Set.ext` + `Iff.intro` 双向 + `Or.elim` 两支 + `And.intro`/投影 + `Or.inl/inr` | 15 | And.intro×1 And.left×1 And.right×1 Or.inl×1 Or.inr×1 Or.elim×1 Iff.intro×1 … | — |
| 43 | `theorem xlat_rel_inv_comp` | 已证 | 项模式 · `Iff.intro` 双向 + `Exists.elim` 两连 + `And.intro`/投影 + `Exists.intro` | 22 | And.intro×2 And.left×2 And.right×2 Exists.intro×2 Exists.elim×2 Iff.intro×1 Rel.ext×1 … | — |
| 69 | `theorem xlat_injective_comp` | 已证 | 项模式 · fun 链（3 层）· 纯应用 | 4 | — | — |
| 77 | `theorem xlat_preimage_comp` | 已证 | 项模式 · Set.ext 外延 | 4 | Set.ext×1 | — |
| 85 | `theorem fix_image_preimage` | 已证 | 项模式 · `Set.ext` + `Iff.intro` 双向 + `Exists.elim` 两连 + `And.intro`/`Exists.intro` + `Eq.subst` | 27 | And.intro×1 And.left×1 And.right×3 Exists.intro×1 Exists.elim×2 Iff.intro×1 Eq.subst×2 … | — |
| 116 | `theorem fix_image_inter` | 已证 | 项模式 · `Set.ext` + `fun y => Iff.intro` 双向 + `Exists.elim`×3 + `And.intro`/`Exists.intro` 重建 + `Eq.subst` | 47 | And.intro×5 And.left×6 And.right×7 Exists.intro×3 Exists.elim×3 Iff.intro×1 Eq.subst×1 … | — |
| 173 | `theorem flawed_equalities_refuted` | 已证 | 项模式 · `And.intro` 两支；每支 `fun h =>` + `let`（MF/UF）+ `Exists.elim` + `congrArg`/`Eq.subst` **命题级重写** + `absurd` | 156 | And.intro×4 And.left×3 And.right×2 Exists.intro×2 Exists.elim×2 Eq.subst×4 Eq.symm×4 … | — |
| 348 | `theorem project_chain` | 已证 | 项模式 · `And.intro` 两支：左支 `Rel.ext` + `fun a c => Iff.intro` + `Exists.elim` + `congrArg`；右支 `Set.ext` + `Iff.intro` + `Exists.elim`×2 | 69 | And.intro×5 And.left×4 And.right×4 Exists.intro×4 Exists.elim×4 Iff.intro×2 congrArg×2 … | — |
| 438 | `theorem project_chain_cardinal` | 已证 | 项模式 · `And.intro` 两支；左支 `Exists.elim`×2 + `congrArg` 命题级；右支 `fun h =>` + `Exists.elim` + `And.left/right` 投影链 | 143 | And.intro×2 And.left×7 And.right×10 Exists.elim×4 Eq.subst×5 congrArg×2 Eq.symm×3 … | — |
---

## 3. 统计汇总（C）

### 3.1 总数

| 量 | 数 |
|---|---:|
| 顶层声明总数 | **424** |
| 已证（`theorem`/`example`，体无 `sorry`） | **245** |
| 开放练习（`:= sorry`） | **99** |
| 部分作答（体里有 `sorry` 但也有别的结构） | **0**（全部 99 个洞都是裸 `sorry`，没有一个"半填"） |
| 其中 99 个开放声明里有 **1 个是 `def`**（`units/unit10-cantor.sokonanoda:96 def Set.Countable := sorry`） | — |
| `def` | 71 |
| `axiom` | 6 |
| `inductive` | 4 |
| 其中 `example`（不进 kernel `checked`） | 6 |
| kernel `checked` 合计（= 424 − 99 − 6 + Demo 重复 10） | **329** |
| `-- soko:hint` 注释条数 | **294** |

按层：

| 层 | 文件 | 行数 | 声明 | 已证 | 开放 | hint |
|---|---:|---:|---:|---:|---:|---:|
| lib/（L2 库） | 8 | 845 | 74 | 39 | 0 | 0 |
| units/（画布） | 13 | 2841 | 188 | 66 | 99 | 294 |
| units/solutions/（解答） | 13 | 2803 | 162 | 140 | 0 | 0 |

### 3.2 项模式 vs tactic 模式

| 形态 | 条数 | 说明 |
|---|---:|---|
| **tactic 模式（`:= by …`）** | **0** | 全课程一个 `by` 块都没有 |
| 项模式（`:= <expr>`）有体 | **315** | 其中 71 个是 `def` 的定义体、245 个是证明 |
| 无体（`axiom`/`inductive`） | 10 | 只有签名 |
| 开放（`:= sorry`） | 99 | 值位 `sorry`，无证明体 |

**tactic 白名单**（`crates/front/src/parser.rs:1240-1300`，设计 `docs/design/by-tactics.md` §2）：
`intro`（只吃名字，**不支持** `intro ⟨a,b⟩` 模式）/ `exact` / `apply` / `assumption` / `rfl` / `match`（臂体是项）/ `sorry`。

### 3.3 联结词统计：签名位 vs 证明体 vs 定义体

**口径**：`签名位` = `theorem`/`example` 的命题（`: … :=` 之前）；`证明体` = `:=` 之后的证明项；
`def 体` = `def` 的 `:=` 之后（**定义的值**，不是证明——`def` 的「类型位」往往只写 `Prop`/`Set α`，
所以联结词全落在体里）。`And`/`Or`/`Iff`/`Exists`/`forall`/`Not` 只数**裸名**（`And.intro` 不算 `And`）。

| 联结词 | 签名位（命题） | 证明体 | `def`（体+类型位） | axiom/inductive | 合计 | Lean 4 符号 | 可改写？ |
|---|---:|---:|---:|---:|---:|---|---|
| `And` | 46 | 404 | 19 | 0 | **469** | `∧` | 可（`infix:35 " ∧ " => And` 实测通过） |
| `Or` | 7 | 70 | 8 | 0 | **85** | `∨` | 可（`infix:30 " ∨ " => Or` 实测通过） |
| `Iff` | 73 | 1 | 0 | 2 | **76** | `↔` | 可（`infix:20 " ↔ " => Iff` 实测通过） |
| `Exists` | 27 | 109 | 15 | 6 | **157** | `∃` | 可，但**要 `binder_notation "∃" => Exists` 且 `Exists` 在作用域**（单元 1–7 未 import `lib.Exists` 的都要补） |
| `forall` | 41 | 49 | 21 | 2 | **113** | `∀` | **`∀` 是原生 token**，但必须写类型标注（`∀ (x : α), …`；`∀ x, …` 报 `elab-untyped-binder`） |
| `Not` | 65 | 102 | 10 | 0 | **177** | `¬` | 可（`prefix:40 "¬" => Not` 实测通过；注意一元记法在实参位要加括号） |

**含至少一个点名叫法的声明**：签名位 **159/424**、证明体 **178/424**（见 §0.3 口径）。

每文件的联结词点名叫法密度（签名位 + 证明体 + def 体，裸名计数）——**记法改写的工作量分布**：

| 文件 | And | Or | Iff | Exists | forall | Not | 合计 |
|---|---:|---:|---:|---:|---:|---:|---:|
| `lib/Demo.sokonanoda` | 2 | 0 | 3 | 1 | 1 | 0 | **7** |
| `lib/Equiv.sokonanoda` | 11 | 0 | 0 | 4 | 3 | 0 | **18** |
| `lib/Exists.sokonanoda` | 0 | 0 | 0 | 7 | 1 | 0 | **8** |
| `lib/Fun.sokonanoda` | 8 | 0 | 6 | 4 | 20 | 0 | **38** |
| `lib/Image.sokonanoda` | 10 | 0 | 3 | 4 | 1 | 0 | **18** |
| `lib/Prod.sokonanoda` | 1 | 0 | 0 | 0 | 0 | 0 | **1** |
| `lib/Rel.sokonanoda` | 3 | 0 | 4 | 3 | 1 | 0 | **11** |
| `lib/Set.sokonanoda` | 8 | 5 | 8 | 0 | 5 | 6 | **32** |
| `units/notation-cheatsheet.sokonanoda` | 0 | 4 | 2 | 0 | 2 | 5 | **13** |
| `units/solutions/notation-cheatsheet-solution.sokonanoda` | 0 | 4 | 0 | 0 | 2 | 2 | **8** |
| `units/solutions/unit01-solution.sokonanoda` | 0 | 0 | 3 | 0 | 1 | 0 | **4** |
| `units/solutions/unit02-solution.sokonanoda` | 3 | 6 | 3 | 0 | 3 | 5 | **20** |
| `units/solutions/unit03-solution.sokonanoda` | 6 | 0 | 2 | 0 | 0 | 2 | **10** |
| `units/solutions/unit04-solution.sokonanoda` | 39 | 30 | 0 | 0 | 0 | 23 | **92** |
| `units/solutions/unit05-solution.sokonanoda` | 8 | 4 | 2 | 0 | 0 | 2 | **16** |
| `units/solutions/unit06-solution.sokonanoda` | 28 | 12 | 2 | 8 | 13 | 16 | **79** |
| `units/solutions/unit07-solution.sokonanoda` | 0 | 0 | 1 | 10 | 13 | 1 | **25** |
| `units/solutions/unit08-solution.sokonanoda` | 35 | 8 | 3 | 4 | 0 | 6 | **56** |
| `units/solutions/unit09-solution.sokonanoda` | 91 | 0 | 1 | 33 | 1 | 6 | **132** |
| `units/solutions/unit10-solution.sokonanoda` | 6 | 0 | 1 | 8 | 17 | 18 | **50** |
| `units/solutions/unit11-solution.sokonanoda` | 5 | 0 | 2 | 2 | 4 | 6 | **19** |
| `units/solutions/unit12-solution.sokonanoda` | 89 | 3 | 0 | 15 | 0 | 27 | **134** |
| `units/unit01-sets-membership.sokonanoda` | 0 | 0 | 5 | 0 | 2 | 0 | **7** |
| `units/unit02-subsets-empty.sokonanoda` | 1 | 0 | 3 | 0 | 1 | 3 | **8** |
| `units/unit03-union-inter-powerset.sokonanoda` | 3 | 1 | 6 | 0 | 0 | 1 | **11** |
| `units/unit04-extensionality-identities.sokonanoda` | 3 | 2 | 1 | 0 | 1 | 6 | **13** |
| `units/unit05-pairs-products.sokonanoda` | 5 | 3 | 4 | 0 | 0 | 2 | **14** |
| `units/unit06-relations.sokonanoda` | 6 | 2 | 2 | 3 | 7 | 6 | **26** |
| `units/unit07-functions.sokonanoda` | 1 | 0 | 2 | 4 | 8 | 1 | **16** |
| `units/unit08-images-preimages.sokonanoda` | 13 | 1 | 3 | 2 | 0 | 3 | **22** |
| `units/unit09-equinumerosity.sokonanoda` | 2 | 0 | 1 | 2 | 1 | 4 | **10** |
| `units/unit10-cantor.sokonanoda` | 0 | 0 | 1 | 4 | 0 | 21 | **26** |
| `units/unit11-universe-russell.sokonanoda` | 2 | 0 | 2 | 2 | 3 | 2 | **11** |
| `units/unit12-synthesis.sokonanoda` | 80 | 0 | 0 | 37 | 2 | 3 | **122** |
### 3.4 最长的 20 个项模式证明（改写成本的主要来源）

> 排序依据：证明体行数（`:=` 到最后一行代码）。**全部是项模式**；「改写难点」是逐条读过的判断。

| # | 文件:行 | 声明 | 体行 | 形状（读过原文） | 改写难点 |
|---:|---|---|---:|---|---|
| 1 | `units/solutions/unit12-solution.sokonanoda:173` | `flawed_equalities_refuted` | 156 | 见 §2 同名行 | **最难**：`let` 局部定义 ×2（MF/UF）+ `congrArg` 把命题当函数用 + `Eq.subst` 命题级重写 + `absurd`。现有 tactic 无法表达 `let` 与命题级重写。 |
| 2 | `units/solutions/unit12-solution.sokonanoda:438` | `project_chain_cardinal` | 143 | 见 §2 同名行 | `Exists.elim`×2 + `congrArg` 命题级 + 投影链 43 处 `And.left/right`；两层结构跨 143 行。 |
| 3 | `units/unit12-synthesis.sokonanoda:332` | `demo_equiv_comp` | 101 | 见 §2 同名行 | `Exists.elim` **四连嵌套（深度 3）**——没有 `obtain`/`cases` 只能整段 `exact`。 |
| 4 | `units/solutions/unit09-solution.sokonanoda:118` | `Set.Equiv.trans` | 103 | 见 §2 同名行 | 同上（`Set.Equiv.trans`）：`Exists.elim` 四连 + `congrArg`×2 + `Eq.trans`；改写需 `obtain` 四次。 |
| 5 | `units/solutions/unit06-solution.sokonanoda:182` | `classes_isPartition` | 84 | 见 §2 同名行 | `let` 局部定义 + `Exists.elim`×4（深度 3）+ `Iff.mp/mpr` 单向 + `Eq.subst`。 |
| 6 | `units/solutions/unit12-solution.sokonanoda:348` | `project_chain` | 69 | 见 §2 同名行 | `Rel.ext`+`Set.ext` 双外延 + `Iff.intro` 双向 + `Exists.elim`×3 + `congrArg`。 |
| 7 | `units/solutions/unit09-solution.sokonanoda:301` | `equiv_nonempty_iff` | 54 | 见 §2 同名行 | `Iff.intro` 双向 × `Exists.elim` 两连（深度 2）；每层都要 `obtain`。 |
| 8 | `units/solutions/unit06-solution.sokonanoda:52` | `rel_comp_assoc` | 56 | 见 §2 同名行 | `Rel.ext` + `Iff.intro` 双向 + `Exists.elim`×4 + 投影/重组。 |
| 9 | `units/unit12-synthesis.sokonanoda:97` | `demo_rel_comp_assoc` | 58 | 见 §2 同名行 | **画布上也有这条**（不止解答）：`Rel.ext` + `Exists.elim`×4；画布改动会影响 `canvas_open` 计数口径。 |
| 10 | `units/solutions/unit06-solution.sokonanoda:303` | `nearStep_counterexample` | 54 | 见 §2 同名行 | `Or.elim` 两支 + `let` 局部辅助 + `Or.inl/inr`；`Or.elim` 在 tactic 里无对应物。 |
| 11 | `units/solutions/unit12-solution.sokonanoda:116` | `fix_image_inter` | 47 | 见 §2 同名行 | `Set.ext` + `Iff.intro` + `Exists.elim`×3 + `Eq.subst`。 |
| 12 | `units/solutions/unit08-solution.sokonanoda:102` | `image_union` | 45 | 见 §2 同名行 | `Set.ext` + `Iff.intro` + `Exists.elim`×3 + 内层 `Or.elim`；两层消去混在一起。 |
| 13 | `units/solutions/unit10-solution.sokonanoda:44` | `cantor` | 45 | 见 §2 同名行 | `let` ×2 + `congrArg` 命题级 + `absurd`（Cantor）；**证明依赖 `let` 绑定的类型标注**，tactic 版要重写整段。 |
| 14 | `units/solutions/unit09-solution.sokonanoda:72` | `Set.Equiv.symm` | 39 | 见 §2 同名行 | `Exists.elim` 两连 + 投影 + `Exists.intro` 重建。 |
| 15 | `units/solutions/unit08-solution.sokonanoda:418` | `image_preimage_eq_of_subset_image_univ` | 21 | 见 §2 同名行 | `Iff.intro` 双向，但反向要 `Iff.mpr` + `Exists.elim` + `Eq.subst`；正向是纯引用。 |
| 16 | `units/solutions/unit09-solution.sokonanoda:258` | `not_equiv_empty_singleton` | 36 | 见 §2 同名行 | `Exists.elim` 两连 + `False.elim`；结论是 `False`（爆炸型）。 |
| 17 | `units/solutions/unit05-solution.sokonanoda:90` | `kura_degenerate` | 31 | 见 §2 同名行 | `Set.ext` + `Iff.intro` + `Or.elim` + `Eq.symm/trans`（Kuratowski）。 |
| 18 | `units/solutions/unit09-solution.sokonanoda:366` | `proper_subset_counterexample` | 28 | 见 §2 同名行 | **唯一显式 `Nat.rec`**（归纳消去）+ `Eq.subst` + `False.elim`。 |
| 19 | `units/solutions/unit12-solution.sokonanoda:85` | `fix_image_preimage` | 27 | 见 §2 同名行 | `Set.ext` + `Iff.intro` + `Exists.elim`×2 + `Eq.subst`。 |
| 20 | `units/solutions/unit04-solution.sokonanoda:104` | `sdiff_sdiff` | 26 | 见 §2 同名行 | `Set.ext` + `Iff.intro` + `Or.elim` 两支 + 投影 + `Or.inl/inr`（`sdiff_sdiff`）。 |

### 3.5 C 类细分：107 个「现有 tactic 转不干净」的证明（口径对齐见下）

> 定义：已证的 `theorem`/`example`（**245** 条）里，证明体含**消去子或重写**（`Exists.elim`/`Or.elim`/`And.left`/`And.right`/
> `Eq.subst`/`congrArg`/`absurd`/`Nat.rec`/`Exists.rec`/`Iff.mp`/`Iff.mpr`）或 `let`/`match` 的那些。
> A 类（纯 `fun` 链 + 引理应用）、B 类（只用构造子，`apply` 可解）可机械改写。

**口径对齐**（设计文档 §1.1 写的是「246 条 / C 类 103」——差 1 的来源在这里）：

| 口径 | A | B | C | 合计 |
|---|---:|---:|---:|---:|
| 消去集**不含** `False.elim` | 79 | 64 | **102** | 245 |
| 消去集**含** `False.elim`（爆炸也要 `exfalso`，白名单里没有） | 74 | 64 | **107** | 245 |
| 再把 `def Exists.elim` 算进来（它的体也是证明项）= 设计文档的口径 | 79 | 64 | **103** | 246 |

> 本文档 §4.1 用**含 `False.elim`** 的口径（**107**），因为它更贴近「需要哪些新 tactic」这个问题；
> `False.elim` 只影响 5 条（`notation-cheatsheet:166/170`、它的解答 `:61/65`、`unit02-solution:18`）。

#### 3.5.1 `Exists.elim`（37 个声明 / 76 处）

**先说结论**：本语言的 `Exists.elim` 签名是 `(A) (p) (Q) (h) (f)`，其中 **`Q` 是常量 motive**
（`lib/Exists.sokonanoda:92`，由 `Exists.rec` 定义）——**「结论 Q 不许提到证人」是类型层面的硬约束**，
所以 76 处**全部**是「Q 不依赖证人」的形状。真正的区分在**嵌套层数**：

| 形状 | 声明数 | 说明 |
|---|---:|---|
| 1 处 · 嵌套深度 0 | 17 | |
| 2 处 · 嵌套深度 0 | 5 | |
| 2 处 · 嵌套深度 1 | 5 | |
| ≥3 处 · 嵌套深度 1 | 6 | |
| ≥3 处 · 嵌套深度 2 | 1 | |
| ≥3 处 · 嵌套深度 3 | 3 | |

**单层（1 处，17 个声明）**——最易改写，`obtain ⟨w, hw⟩ := h` 一步到位（若有 `obtain`）：

| 文件:行 | 声明 | 体行 | 结论 Q 的形状（从原文抄） |
|---|---|---:|---|
| `lib/Demo.sokonanoda:30` | `demo_exists_elim` | 4 | `Q`（自由命题变量，引理无固定结论） |
| `lib/Exists.sokonanoda:92` | `Exists.elim` | 2 | `Q`（`Exists.elim` 自己的 `Q` 参数——这是**定义**不是证明） |
| `lib/Exists.sokonanoda:99` | `Exists.imp` | 3 | `Exists A q` |
| `lib/Image.sokonanoda:65` | `Set.image_mono` | 9 | `Set.mem β y B` |
| `lib/Image.sokonanoda:77` | `Set.image_subset_iff` | 14 | `Set.mem β y B` |
| `units/solutions/unit07-solution.sokonanoda:90` | `swap_exists_forall` | 6 | `forall (x2 : α), P x2 y` |
| `units/solutions/unit07-solution.sokonanoda:104` | `swap_converse_false` | 20 | `False` |
| `units/solutions/unit08-solution.sokonanoda:162` | `image_inter_subset` | 17 | `Set.mem β y (Set.image α β f B)` |
| `units/solutions/unit08-solution.sokonanoda:240` | `image_empty` | 15 | `Eq.{1} (Set β) (Set.image α β f (Set.empty α)) (Set.empty β)` |
| `units/solutions/unit08-solution.sokonanoda:315` | `image_preimage_subset` | 9 | `Set.mem β y C` |
| `units/solutions/unit08-solution.sokonanoda:332` | `image_preimage_eq_aa` | 4 | `Eq.{1} Two y aa` |
| `units/solutions/unit08-solution.sokonanoda:338` | `bb_not_mem_image_preimage` | 6 | `False` |
| `units/solutions/unit08-solution.sokonanoda:418` | `image_preimage_eq_of_subset_image_univ` | 21 | `Exists α (fun (x : α) => And (Set.mem α x (Set.preimage α β f C)) (Eq.{1} β (f x) y))` |
| `units/solutions/unit11-solution.sokonanoda:21` | `no_univ_strictly_larger` | 12 | `False` |
| `units/unit08-images-preimages.sokonanoda:211` | `image_empty` | 15 | `Eq.{1} (Set Two) (Set.image Two Two f1 (Set.empty Two)) (Set.empty Two)` |
| `units/unit08-images-preimages.sokonanoda:262` | `image_inter_subset` | 17 | `Set.mem β y (Set.image α β f B)` |
| `units/unit08-images-preimages.sokonanoda:304` | `image_preimage_subset` | 9 | `Set.mem β y C` |

**多层（≥2 处，20 个声明）**：

| 文件:行 | 声明 | 出现 | 嵌套深度 | 体行 | 代表片段 |
|---|---|---:|---:|---:|---|
| `units/solutions/unit06-solution.sokonanoda:182` | `classes_isPartition` | 4 | 3 | 84 | `Exists.elim A (fun (a : A) => Eq.{1} (Set A) C (EquivClass A r a)) (Not (Eq.{1} (Set A) C …` |
| `units/solutions/unit09-solution.sokonanoda:118` | `Set.Equiv.trans` | 4 | 3 | 103 | `Exists.elim (α -> β) (fun (f1 : α -> β) => And (Set.MapsTo α β f1 A B) (Exists (β -> α) (f…` |
| `units/unit12-synthesis.sokonanoda:332` | `demo_equiv_comp` | 4 | 3 | 101 | `Exists.elim (α -> β) (fun (f : α -> β) => And (Set.MapsTo α β f A B) (Exists (β -> α) (fun…` |
| `units/solutions/unit09-solution.sokonanoda:301` | `equiv_nonempty_iff` | 5 | 2 | 54 | `Exists.elim α A (Exists β B) ha (fun (x : α) => fun (hx : Set.mem α x A) => Exists.elim (α…` |
| `units/solutions/unit06-solution.sokonanoda:52` | `rel_comp_assoc` | 4 | 1 | 56 | `Exists.elim B (fun (b : B) => And (r a b) (Rel.comp B C D s t b d)) (Rel.comp A C D (Rel.c…` |
| `units/solutions/unit12-solution.sokonanoda:348` | `project_chain` | 4 | 1 | 69 | `Exists.elim β (fun (b : β) => And (Eq.{1} β (f a) b) (Eq.{1} γ (g b) c)) (Eq.{1} γ (g (f a…` |
| `units/solutions/unit12-solution.sokonanoda:438` | `project_chain_cardinal` | 4 | 1 | 143 | `Exists.elim (α -> γ) (fun (f : α -> γ) => And (Set.MapsTo α γ f A C) (Exists (γ -> α) (fun…` |
| `units/unit12-synthesis.sokonanoda:97` | `demo_rel_comp_assoc` | 4 | 1 | 58 | `Exists.elim C (fun (c : C) => And (Exists B (fun (b : B) => And (r a b) (s b c))) (t c d))…` |
| `units/solutions/unit08-solution.sokonanoda:102` | `image_union` | 3 | 1 | 45 | `Exists.elim α (fun (x : α) => And (Set.mem α x (Set.union α A B)) (Eq.{1} β (f x) y)) (Or …` |
| `units/solutions/unit12-solution.sokonanoda:116` | `fix_image_inter` | 3 | 1 | 47 | `Exists.elim α (fun (x : α) => And (Set.mem α x (Set.inter α A B)) (Eq.{1} β (f x) y)) (And…` |
| `units/solutions/unit07-solution.sokonanoda:32` | `surjective_comp` | 2 | 1 | 12 | `Exists.elim β (fun (y : β) => Eq.{1} γ (g y) z) (Exists α (fun (x : α) => Eq.{1} γ (Functi…` |
| `units/solutions/unit09-solution.sokonanoda:72` | `Set.Equiv.symm` | 2 | 1 | 39 | `Exists.elim (α -> β) (fun (f : α -> β) => And (Set.MapsTo α β f A B) (Exists (β -> α) (fun…` |
| `units/solutions/unit09-solution.sokonanoda:258` | `not_equiv_empty_singleton` | 2 | 1 | 36 | `Exists.elim (α -> β) (fun (f : α -> β) => And (Set.MapsTo α β f (Set.empty α) (Set.singlet…` |
| `units/solutions/unit10-solution.sokonanoda:44` | `cantor` | 2 | 1 | 45 | `Exists.elim (α -> Set α) MF False h (fun (f : α -> Set α) => fun (hf : MF f) => let UF : P…` |
| `units/solutions/unit10-solution.sokonanoda:98` | `no_surjection_powerset` | 2 | 1 | 17 | `Exists.elim (α -> Set α) (fun (f : α -> Set α) => Function.Surjective α (Set α) f) False h…` |
| `units/solutions/unit08-solution.sokonanoda:345` | `image_preimage_eq_image` | 2 | 0 | 25 | `Exists.elim Two (fun (x : Two) => Eq.{1} Two (f1 x) y) (Set.mem Two y (Set.image Two Two f…` |
| `units/solutions/unit08-solution.sokonanoda:379` | `image_preimage_image_eq_preimage` | 2 | 0 | 23 | `Exists.elim Two (fun (x : Two) => And (Set.mem Two x (Set.preimage Two Two f1 U01)) (Eq.{1…` |
| `units/solutions/unit12-solution.sokonanoda:43` | `xlat_rel_inv_comp` | 2 | 0 | 22 | `Exists.elim B (fun (b : B) => And (r a b) (s b c)) (Exists B (fun (b : B) => And (s b c) (…` |
| `units/solutions/unit12-solution.sokonanoda:85` | `fix_image_preimage` | 2 | 0 | 27 | `Exists.elim α (fun (x : α) => And (Set.mem α x (Set.preimage α β f C)) (Eq.{1} β (f x) y))…` |
| `units/solutions/unit12-solution.sokonanoda:173` | `flawed_equalities_refuted` | 2 | 0 | 156 | `Exists.elim (Set Nat) (fun (x : Set Nat) => And (Set.mem (Set Nat) x (Set.preimage (Se…` |
#### 3.5.2 `Eq.subst`（37 个声明 / 55 处）

签名 `axiom Eq.subst {u} : {α} -> {p : α -> Prop} -> {a} -> {b} -> Eq a b -> p a -> p b`（`crates/front/src/compile/prelude.rs:163`）——
**没有 `rw`，一切重写都是显式给 motive 的 `Eq.subst`**。按出现位置分：

| 位置 | 处数 | 对应 Lean 写法 | 说明 |
|---|---:|---|---|
| **目标位**（表达式头部/分支结果） | 26 | `rw [h]` | motive 就是把目标里的某项换成变量；改写后直接闭合目标 |
| **参数位**（作为别的函数的实参） | 29 | `rw [h] at hx` 或 `exact h (by rwa [...])` | 先把假设/项搬成需要的形状，再交给外层引理 |
| 其中 **命题级重写**（`motive = fun (Q : Prop) => Q`） | 7 | 无直接对应（`Eq.mp`/`propext` 级） | **最难**：把「命题相等」当重写用，`rw` 也救不了 |

目标位（26 处，可直接 `rw`）：

| 文件:行 | 声明 | motive 片段 |
|---|---|---|
| `lib/Image.sokonanoda:89` | `Set.image_subset_iff` | `Eq.subst.{1} β (fun (z : β) => Set.mem β z B) (f x) y (And.right (A x) (Eq.{1} β…` |
| `units/solutions/unit01-solution.sokonanoda:21` | `subset_of_mem_singleton` | `Eq.subst.{1} α (fun (y : α) => A y) a x (Eq.symm.{1} α x a hx) h…` |
| `units/solutions/unit02-solution.sokonanoda:27` | `subset_empty_iff` | `Eq.subst.{1} (Set α) (fun (X : Set α) => Set.subset α X (Set.empty α)) (Set.empt…` |
| `units/solutions/unit02-solution.sokonanoda:35` | `eq_empty_iff_forall_notMem` | `Eq.subst.{1} (Set α) (fun (X : Set α) => Not (Set.mem α x X)) (Set.empty α) A (E…` |
| `units/solutions/unit02-solution.sokonanoda:62` | `pair_subset_iff` | `Eq.subst.{1} α (fun (y : α) => A y) a x (Eq.symm.{1} α x a hxa) (And.left (Set.m…` |
| `units/solutions/unit02-solution.sokonanoda:65` | `pair_subset_iff` | `Eq.subst.{1} α (fun (y : α) => A y) b x (Eq.symm.{1} α x b hxb) (And.right (Set.…` |
| `units/solutions/unit05-solution.sokonanoda:52` | `prod_mk_eq_iff` | `Eq.subst.{1} B (fun (y : B) => Eq.{1} (Prod A B) (prod_mk A B a b) (prod_mk A B …` |
| `units/solutions/unit06-solution.sokonanoda:205` | `classes_isPartition` | `Eq.subst.{1} (Set A) (fun (X : Set A) => Set.mem A a X) (EquivClass A r a) (Set.…` |
| `units/solutions/unit06-solution.sokonanoda:249` | `classes_isPartition` | `Eq.subst.{1} (Set A) (fun (X : Set A) => Set.mem A x X) C (EquivClass A r a) ha…` |
| `units/solutions/unit06-solution.sokonanoda:253` | `classes_isPartition` | `Eq.subst.{1} (Set A) (fun (X : Set A) => Set.mem A x X) D (EquivClass A r b) hb…` |
| `units/solutions/unit06-solution.sokonanoda:287` | `Nat.zero_ne_succ` | `Eq.subst.{1} Nat (fun (m : Nat) => Nat.isZero m) 0 (Nat.succ n) h True.intro…` |
| `units/solutions/unit08-solution.sokonanoda:204` | `two_aa_ne_bb` | `Eq.subst.{1} Two (fun (t : Two) => isAa t) aa bb h True.intro…` |
| `units/solutions/unit08-solution.sokonanoda:322` | `image_preimage_subset` | `Eq.subst.{1} β (fun (z : β) => Set.mem β z C) (f x) y (And.right (Set.mem α x (S…` |
| `units/solutions/unit09-solution.sokonanoda:63` | `Nat.succ_ne_zero` | `Eq.subst.{1} Nat (fun (k : Nat) => Nat.isZero k) (Nat.succ n) Nat.zero h True.in…` |
| `units/solutions/unit09-solution.sokonanoda:246` | `Set.equivOfEq` | `Eq.subst.{1} (Set α) (fun (X : Set α) => Set.mem α x X) A B h hx) (fun (y : α) =…` |
| `units/solutions/unit09-solution.sokonanoda:248` | `Set.equivOfEq` | `Eq.subst.{1} (Set α) (fun (X : Set α) => Set.mem α y X) B A (Eq.symm.{1} (Set α)…` |
| `units/solutions/unit10-solution.sokonanoda:86` | `cantor` | `Eq.subst.{1} Prop (fun (Q : Prop) => Q) (Not (f (g D) (g D))) (f (g D) (g D)) (E…` |
| `units/solutions/unit10-solution.sokonanoda:113` | `no_surjection_powerset` | `Eq.subst.{1} Prop (fun (Q : Prop) => Q) (Not (f x x)) (f x x) (Eq.symm.{1} Prop …` |
| `units/solutions/unit11-solution.sokonanoda:72` | `univ_top_iff` | `Eq.subst.{1} (Set α) (fun (X : Set α) => Set.mem α x X) (Set.univ α) A (Eq.symm.…` |
| `units/solutions/unit12-solution.sokonanoda:97` | `fix_image_preimage` | `Eq.subst.{1} β (fun (z : β) => Set.mem β z C) (f x) y (And.right (Set.mem α x (S…` |
| `units/solutions/unit12-solution.sokonanoda:472` | `project_chain_cardinal` | `Eq.subst.{1} (Set γ) (fun (X : Set γ) => Set.mem γ (f x) X) C C' hC (And.left (S…` |
| `units/unit06-relations.sokonanoda:186` | `Nat.zero_ne_succ` | `Eq.subst.{1} Nat (fun (m : Nat) => Nat.isZero m) 0 (Nat.succ n) h True.intro…` |
| `units/unit08-images-preimages.sokonanoda:169` | `two_aa_ne_bb` | `Eq.subst.{1} Two (fun (t : Two) => isAa t) aa bb h True.intro…` |
| `units/unit08-images-preimages.sokonanoda:311` | `image_preimage_subset` | `Eq.subst.{1} β (fun (z : β) => Set.mem β z C) (f x) y (And.right (Set.mem α x (S…` |
| `units/unit09-equinumerosity.sokonanoda:98` | `Nat.succ_ne_zero` | `Eq.subst.{1} Nat (fun (k : Nat) => Nat.isZero k) (Nat.succ n) Nat.zero h True.in…` |
| `units/unit11-universe-russell.sokonanoda:89` | `univ_subset_iff` | `Eq.subst.{1} (Set α) (fun (X : Set α) => Set.mem α x X) (Set.univ α) A (Eq.symm.…` |

参数位（29 处，要 `rw … at h`）：

| 文件:行 | 声明 | 片段 |
|---|---|---|
| `units/solutions/unit01-solution.sokonanoda:34` | `singleton_eq_singleton_iff` | `Eq.subst.{1} (Set α) (fun (X : Set α) => Set.mem α a X) (Set.singleton α a) (Set…` |
| `units/solutions/unit02-solution.sokonanoda:48` | `singleton_ne_empty` | `Eq.subst.{1} (Set α) (fun (X : Set α) => Set.mem α a X) (Set.singleton α a) (Set…` |
| `units/solutions/unit04-solution.sokonanoda:158` | `union_sdiff_univ_ne_univ` | `Eq.subst.{1} (Set Nat) (fun (X : Set Nat) => Set.mem Nat Nat.zero X) (Set.univ N…` |
| `units/solutions/unit05-solution.sokonanoda:39` | `prod_mk_inj` | `Eq.subst.{1} (Prod A B) (fun (p : Prod A B) => Eq.{1} A a (Prod.fst A B p)) (pro…` |
| `units/solutions/unit05-solution.sokonanoda:41` | `prod_mk_inj` | `Eq.subst.{1} (Prod A B) (fun (p : Prod A B) => Eq.{1} B b (Prod.snd A B p)) (pro…` |
| `units/solutions/unit05-solution.sokonanoda:55` | `prod_mk_eq_iff` | `Eq.subst.{1} A (fun (x : A) => Eq.{1} (Prod A B) (prod_mk A B a b) (prod_mk A B …` |
| `units/solutions/unit05-solution.sokonanoda:79` | `prod_set_swap_ne` | `Eq.subst.{1} (Set (Prod A A)) (fun (S : Set (Prod A A)) => Set.mem (Prod A A) (p…` |
| `units/solutions/unit06-solution.sokonanoda:147` | `equivClass_eq_iff` | `Eq.subst.{1} (Set A) (fun (X : Set A) => Set.mem A a X) (EquivClass A r a) (Equi…` |
| `units/solutions/unit07-solution.sokonanoda:116` | `swap_converse_false` | `Eq.subst.{1} (Set Nat) (fun (X : Set Nat) => Set.mem Nat 0 X) (Set.univ Nat) (Se…` |
| `units/solutions/unit08-solution.sokonanoda:295` | `not_image_inter_eq_image_inter` | `Eq.subst.{1} (Set Two) (fun (X : Set Two) => Set.mem Two aa X) (Set.inter Two (S…` |
| `units/solutions/unit08-solution.sokonanoda:408` | `not_image_preimage_eq` | `Eq.subst.{1} (Set Two) (fun (X : Set Two) => Set.mem Two bb X) (Set.image Two Tw…` |
| `units/solutions/unit08-solution.sokonanoda:412` | `not_image_preimage_eq` | `Eq.subst.{1} (Set Two) (fun (X : Set Two) => Set.mem Two bb X) U01 (Set.image Tw…` |
| `units/solutions/unit08-solution.sokonanoda:436` | `image_preimage_eq_of_subset_image_univ` | `Eq.subst.{1} β (fun (z : β) => Set.mem β z C) y (f x) (Eq.symm.{1} β (f x) y (An…` |
| `units/solutions/unit09-solution.sokonanoda:378` | `proper_subset_counterexample` | `Eq.subst.{1} (Set Nat) (fun (X : Set Nat) => Set.mem Nat Nat.zero X) (Set.univ N…` |
| `units/solutions/unit10-solution.sokonanoda:83` | `cantor` | `Eq.subst.{1} Prop (fun (Q : Prop) => Q) (f (g D) (g D)) (Not (f (g D) (g D))) e …` |
| `units/solutions/unit10-solution.sokonanoda:111` | `no_surjection_powerset` | `Eq.subst.{1} Prop (fun (Q : Prop) => Q) (f x x) (Not (f x x)) e p) p; let hp : f…` |
| `units/solutions/unit12-solution.sokonanoda:109` | `fix_image_preimage` | `Eq.subst.{1} β (fun (z : β) => Set.mem β z C) y (f x) (Eq.symm.{1} β (f x) y (An…` |
| `units/solutions/unit12-solution.sokonanoda:157` | `fix_image_inter` | `Eq.subst.{1} α (fun (t : α) => B t) x2 x1 (Eq.symm.{1} α x1 x2 (hf x1 x2…` |
| `units/solutions/unit12-solution.sokonanoda:217` | `flawed_equalities_refuted` | `Eq.subst.{1} (Set (Set Nat)) (fun (X : Set (Set Nat)) => Set.mem (Set Nat) (Set.…` |
| `units/solutions/unit12-solution.sokonanoda:236` | `flawed_equalities_refuted` | `Eq.subst.{1} (Set Nat) (fun (Z : Set Nat) => Set.mem Nat 0 Z) (Set.univ Nat) (Se…` |
| `units/solutions/unit12-solution.sokonanoda:264` | `flawed_equalities_refuted` | `Eq.subst.{1} (Set (Set Nat)) (fun (X : Set (Set Nat)) => Set.mem (Set Nat) (Set.…` |
| `units/solutions/unit12-solution.sokonanoda:320` | `flawed_equalities_refuted` | `Eq.subst.{1} (Set Nat) (fun (Z : Set Nat) => Set.mem Nat 0 Z) (Set.univ Nat) (Se…` |
| `units/solutions/unit12-solution.sokonanoda:481` | `project_chain_cardinal` | `Eq.subst.{1} (Set γ) (fun (X : Set γ) => Set.mem γ z X) C' C (Eq.symm.{1} (Set γ…` |
| `units/solutions/unit12-solution.sokonanoda:491` | `project_chain_cardinal` | `Eq.subst.{1} (Set γ) (fun (X : Set γ) => Set.mem γ z X) C' C (Eq.symm.{1} (Set γ…` |
| `units/solutions/unit12-solution.sokonanoda:526` | `project_chain_cardinal` | `Eq.subst.{1} Prop (fun (Q : Prop) => Q) ((Set.sdiff γ C' (fun (y : γ) => f y y))…` |
| `units/solutions/unit12-solution.sokonanoda:566` | `project_chain_cardinal` | `Eq.subst.{1} Prop (fun (Q : Prop) => Q) ((f (g (Set.sdiff γ C' (fun (y : γ) => f…` |
| `units/unit10-cantor.sokonanoda:51` | `demo_fake_enum_escapes` | `Eq.subst.{1} (Set Nat) (fun (X : Set Nat) => X k) (fake_enum k) (fun (y : Nat) =…` |
| `units/unit10-cantor.sokonanoda:76` | `demo_diag` | `Eq.subst.{1} Prop (fun (Q : Prop) => Q -> Not (f x x)) (Not (f x x)) (f x x) (co…` |
| `units/unit10-cantor.sokonanoda:81` | `demo_diag` | `Eq.subst.{1} Prop (fun (Q : Prop) => Q) (Not (f x x)) (f x x) (congrArg.{1} (Set…` |

**命题级重写 7 处**（`fun (Q : Prop) => Q`）——这 7 处是「Cantor 对角线 / 假等式反驳」的核心，改写风险最高：

| 文件:行 | 声明 |
|---|---|
| `units/solutions/unit10-solution.sokonanoda:83` | `cantor` |
| `units/solutions/unit10-solution.sokonanoda:86` | `cantor` |
| `units/solutions/unit10-solution.sokonanoda:111` | `no_surjection_powerset` |
| `units/solutions/unit10-solution.sokonanoda:113` | `no_surjection_powerset` |
| `units/solutions/unit12-solution.sokonanoda:526` | `project_chain_cardinal` |
| `units/solutions/unit12-solution.sokonanoda:566` | `project_chain_cardinal` |
| `units/unit10-cantor.sokonanoda:81` | `demo_diag` |

#### 3.5.3 `And.left` / `And.right`（42 + 43 个声明 / 98 + 106 处）

| 形状 | 声明数 | 改写对应 |
|---|---:|---|
| ① **纯投影**：只用 `And.left/right`，没有别的构造/消去 | 3 | `exact h.1` / `exact h.2`（现语言里是 `exact And.left A B h`）——**最容易** |
| ② 投影 + 构造（`And.intro`/`Exists.intro`/`Or.inl`/`Iff.intro`/`Set.ext`） | 6 | 取一支后重新打包；`apply` + `exact` 可解 |
| ③ 投影 + 消去/重写（`Exists.elim`/`Or.elim`/`Eq.subst`/`congrArg`/…） | 41 | **深层组合**：投影只是长链中的一步，转 tactic 要整段重构 |

① 纯投影的 3 个声明（可当改写样板）：

| 文件:行 | 声明 | 体行 | 原文 |
|---|---|---:|---|
| `units/solutions/unit03-solution.sokonanoda:30` | `inter_subset_left` | 2 | `fun (x : α) => fun (hx : Set.mem α x (Set.inter α A B)) => And.left (A x) (B x) hx` |
| `units/solutions/unit03-solution.sokonanoda:33` | `inter_subset_right` | 2 | `fun (x : α) => fun (hx : Set.mem α x (Set.inter α A B)) => And.right (A x) (B x) hx` |
| `units/solutions/unit03-solution.sokonanoda:57` | `sdiff_subset` | 3 | `fun (x : α) => fun (hx : Set.mem α x (Set.sdiff α A B)) => And.left (A x) (Not (B x)) hx` |

③ 深层组合里出现次数最多的 12 个声明：

| 文件:行 | 声明 | `And.left/right` 处数 | 体行 |
|---|---|---:|---:|
| `units/solutions/unit12-solution.sokonanoda:438` | `project_chain_cardinal` | 17 | 143 |
| `units/solutions/unit09-solution.sokonanoda:118` | `Set.Equiv.trans` | 16 | 103 |
| `units/unit12-synthesis.sokonanoda:332` | `demo_equiv_comp` | 16 | 101 |
| `units/solutions/unit12-solution.sokonanoda:116` | `fix_image_inter` | 13 | 47 |
| `units/solutions/unit04-solution.sokonanoda:104` | `sdiff_sdiff` | 8 | 26 |
| `units/solutions/unit06-solution.sokonanoda:52` | `rel_comp_assoc` | 8 | 56 |
| `units/solutions/unit12-solution.sokonanoda:348` | `project_chain` | 8 | 69 |
| `units/unit12-synthesis.sokonanoda:97` | `demo_rel_comp_assoc` | 8 | 58 |
| `units/solutions/unit04-solution.sokonanoda:63` | `inter_union_distrib_left` | 7 | 25 |
| `units/solutions/unit08-solution.sokonanoda:102` | `image_union` | 7 | 45 |
| `units/solutions/unit09-solution.sokonanoda:72` | `Set.Equiv.symm` | 7 | 39 |
| `units/solutions/unit08-solution.sokonanoda:162` | `image_inter_subset` | 6 | 17 |

#### 3.5.4 `congrArg`（14 个声明 / 19 处）

签名 `congrArg α β f a b (h : a = b) : f a = f b`。**用在什么位置**：

| 文件:行 | 声明 | 用法 |
|---|---|---|
| `units/solutions/unit06-solution.sokonanoda:290` | `Nat.succ.inj` | `Nat.pred` 作用在 `Nat.succ a = Nat.succ b` 上（构造子单射） |
| `units/solutions/unit07-solution.sokonanoda:45` | `surjective_comp` | 把 `f x = y` 用 `g` 推成 `g (f x) = g y`（复合满射） |
| `units/solutions/unit07-solution.sokonanoda:54` | `leftInverse_injective` | 把 `f x = f y` 用 `g` 推成 `g (f x) = g (f y)`（左逆单射） |
| `units/solutions/unit07-solution.sokonanoda:87` | `injective_of_comp_injective` | 同上（复合单射） |
| `units/solutions/unit08-solution.sokonanoda:265` | `image_inter_singletons_empty` | 把集合等式用 `Set.image f1` 推成像等式（反例） |
| `units/solutions/unit09-solution.sokonanoda:194` | `Set.Equiv.trans` | `g1` 作用在 `And.left` 取出的左逆等式上（Equiv.trans） |
| `units/solutions/unit09-solution.sokonanoda:210` | `Set.Equiv.trans` | `f2` 作用在 `And.right` 取出的右逆等式上 |
| `units/solutions/unit10-solution.sokonanoda:81` | `cantor` | **`congrArg` 到 `Prop`**：`X (g D) = f (g D) (g D)` 把命题当函数（Cantor） |
| `units/solutions/unit10-solution.sokonanoda:109` | `no_surjection_powerset` | 同上（不可数） |
| `units/solutions/unit12-solution.sokonanoda:375` | `project_chain` | `g` 作用在 `And.left` 的等式上（合成关系） |
| `units/solutions/unit12-solution.sokonanoda:420` | `project_chain` | `g` 作用在 `And.right` 的等式上 |
| `units/solutions/unit12-solution.sokonanoda:536` | `project_chain_cardinal` | **`congrArg` 到 `Prop`**（基数链） |
| `units/solutions/unit12-solution.sokonanoda:571` | `project_chain_cardinal` | 同上 |
| `units/unit06-relations.sokonanoda:189` | `Nat.succ.inj` | `Nat.pred` 构造子单射（画布演示） |
| `units/unit08-images-preimages.sokonanoda:237` | `image_inter_singletons_empty` | 集合等式 → 像等式（画布演示） |
| `units/unit10-cantor.sokonanoda:78` | `demo_diag` | **`congrArg` 到 `Prop`**（对角线演示） |
| `units/unit10-cantor.sokonanoda:83` | `demo_diag` | 同上 |
| `units/unit12-synthesis.sokonanoda:407` | `demo_equiv_comp` | `g` 作用在左逆等式上（画布演示） |
| `units/unit12-synthesis.sokonanoda:423` | `demo_equiv_comp` | `f2` 作用在右逆等式上（画布演示） |

**结论**：19 处里 **5 处是把 `congrArg` 用到 `Prop`**（`(Set α) → Prop`），这是「命题即集合」的核心手法，
没有 `rw`/`simp` 时无法用 tactic 表达，只能 `exact congrArg …`。

#### 3.5.5 其余消去子 / 重写构件

| 构件 | 声明数 | 处数 | 分布（文件:行） |
|---|---:|---:|---|
| `Or.elim` | 18 | 27 | `units/solutions/notation-cheatsheet-solution.sokonanoda:118` · `units/solutions/notation-cheatsheet-solution.sokonanoda:125` · `units/solutions/notation-cheatsheet-solution.sokonanoda:133` · `units/solutions/unit02-solution.sokonanoda:51` · `units/solutions/unit02-solution.sokonanoda:69` · `units/solutions/unit03-solution.sokonanoda:15` · `units/solutions/unit04-solution.sokonanoda:14` · `units/solutions/unit04-solution.sokonanoda:44` … |
| `False.elim` | 15 | 19 | `units/notation-cheatsheet.sokonanoda:166` · `units/notation-cheatsheet.sokonanoda:170` · `units/solutions/notation-cheatsheet-solution.sokonanoda:61` · `units/solutions/notation-cheatsheet-solution.sokonanoda:65` · `units/solutions/notation-cheatsheet-solution.sokonanoda:118` · `units/solutions/notation-cheatsheet-solution.sokonanoda:133` · `units/solutions/unit02-solution.sokonanoda:18` · `units/solutions/unit02-solution.sokonanoda:31` … |
| `absurd` | 3 | 3 | `units/solutions/unit10-solution.sokonanoda:44` · `units/solutions/unit10-solution.sokonanoda:98` · `units/unit10-cantor.sokonanoda:68` |
| `Iff.mp` | 2 | 2 | `units/solutions/unit01-solution.sokonanoda:29` · `units/solutions/unit03-solution.sokonanoda:62` |
| `Iff.mpr` | 6 | 6 | `lib/Demo.sokonanoda:21` · `units/solutions/unit03-solution.sokonanoda:62` · `units/solutions/unit03-solution.sokonanoda:84` · `units/solutions/unit06-solution.sokonanoda:182` · `units/solutions/unit08-solution.sokonanoda:418` · `units/solutions/unit11-solution.sokonanoda:41` |
| `Nat.rec` | 1 | 1 | `units/solutions/unit09-solution.sokonanoda:366` |
| `Exists.rec` | 1 | 1 | `lib/Exists.sokonanoda:92` |
| `Prod.rec` | 2 | 2 | `lib/Prod.sokonanoda:49` · `lib/Prod.sokonanoda:52` |
| `And.rec` | 0 | 0 | — |
| `Or.rec` | 0 | 0 | — |
| `False.rec` | 0 | 0 | — |
| `Eq.rec` | 0 | 0 | — |
| `Eq.mp` | 0 | 0 | — |
| `Eq.mpr` | 0 | 0 | — |
| `cast` | 0 | 0 | — |

> `Eq.mp` / `Eq.mpr` / `cast` / `Eq.rec` / `And.rec` / `Or.rec` / `False.rec` 在课程里**零使用**——
> 它们是 prelude 内部实现（`Eq.symm`/`Iff.mp`/`And.left` 的定义体），课程层不直接碰。

### 3.6 `-- soko:hint` 分布（改写 hint 是**独立工作量**）

| 文件 | hint 条数 | 含项模式词汇的条数 |
|---|---:|---:|
| `units/notation-cheatsheet.sokonanoda` | 6 | 4 |
| `units/unit01-sets-membership.sokonanoda` | 18 | 7 |
| `units/unit02-subsets-empty.sokonanoda` | 30 | 8 |
| `units/unit03-union-inter-powerset.sokonanoda` | 33 | 9 |
| `units/unit04-extensionality-identities.sokonanoda` | 24 | 9 |
| `units/unit05-pairs-products.sokonanoda` | 21 | 6 |
| `units/unit06-relations.sokonanoda` | 24 | 5 |
| `units/unit07-functions.sokonanoda` | 27 | 13 |
| `units/unit08-images-preimages.sokonanoda` | 27 | 18 |
| `units/unit09-equinumerosity.sokonanoda` | 21 | 7 |
| `units/unit10-cantor.sokonanoda` | 21 | 7 |
| `units/unit11-universe-russell.sokonanoda` | 15 | 3 |
| `units/unit12-synthesis.sokonanoda` | 27 | 12 |
| **合计** | **294** | **0** |

分布：**只有单元画布有 hint**（lib 0 / 解答 0）；99 个开放声明**全部带 hint**（96 个 ×3 条 + 记法对照页 3 个 ×2 条 = 294）。

hint 文本里点名的项模式词汇（**这些词在改成 tactic 后必须同步改**）：

| 词汇 | 出现在多少条 hint 里 |
|---|---:|
| `Set.ext` | 24 |
| `Exists.elim` | 22 |
| `Eq.subst` | 21 |
| `And.left` | 18 |
| `And.right` | 18 |
| `Or.elim` | 17 |
| `Eq.symm` | 15 |
| `And.intro` | 15 |
| `Or.inl` | 14 |
| `fun` | 14 |
| `Exists.intro` | 13 |
| `Or.inr` | 12 |
| `Eq.refl` | 12 |
| `Iff.intro` | 11 |
| `congrArg` | 7 |
| `Eq.trans` | 7 |
| `False.elim` | 5 |
| `Iff.mp` | 1 |

**结论**：294 条 hint 里 **108 条（37%）点名了项模式词汇**。若证明改 `by` 块而 hint 不改，
hint 会把学习者指向语言里已经不该用的写法——**hint 必须与证明同轮改写**。
---

## 4. 改写风险点（D）

### D.1 【最高】tactic 白名单只有 7 个，107 条证明转不干净（口径对齐见下）

**事实**（`crates/front/src/parser.rs:1240-1300`，设计 `docs/design/by-tactics.md` §2）：

```
可用：intro（只吃一个名字）· exact · apply · assumption · rfl · match（臂体是项）· sorry
没有：constructor · left · right · use · cases · obtain · rcases · have · refine · rw · simp · exfalso · by_contra
```

`intro` **不支持模式**（`intro ⟨a,b⟩` / `intro ha hna` 都 parse 错）；`docs/gaps/ledger.jsonl` 里**没有**「缺 tactic」这条缺口（24 条里没有）。

按「能否用现有 7 个 tactic 机械改写」给 **245 条已证 `theorem`/`example`** 分类（口径差异见下表）：


| 口径 | A | B | C | 合计 |
|---|---:|---:|---:|---:|
| 消去集**含** `False.elim`（本文档采用） | 74 | 64 | **107** | 245 |
| 消去集**不含** `False.elim` | 79 | 64 | 102 | 245 |
| 再把 `def Exists.elim` 算进来（= 设计文档 §1.1 的 246/103 口径） | 79 | 64 | 103 | 246 |

| 类 | 条数 | 判据 | 可机械改写？ | 体中位行数 |
|---|---:|---|---|---:|
| **A** | 74 | 纯 `fun` 链 + 引理应用 | ✅ `intro` + `exact`/`apply` | 2 |
| **B** | 64 | 只用构造子（`And.intro`/`Or.inl`/`Or.inr`/`Exists.intro`/`Iff.intro`/`True.intro`/`Eq.refl`） | ✅ `apply And.intro` / `apply Or.inl` 实测可用（`crates/cli/tests/cli.rs:121-123`） | 4 |
| **C** | 107 | 含消去/重写（`And.left/right`/`Exists.elim`/`Eq.subst`/`Or.elim`/`congrArg`/`Iff.mp/mpr`/`let`/`absurd`/`Nat.rec`/`Exists.rec`） | ❌ **只能写成 `intro h; exact Exists.elim … h (fun w hw => …)`——`by` 壳里还是原项** | 12（最长 156） |

⇒ **决策点**：要么先扩白名单（`constructor`/`left`/`right`/`use`/`cases`/`obtain`/`exfalso` + `intro` 多名字），
要么接受 C 类 107 条是「`by` + 单个 `exact <原项>`」的伪改写（设计文档 §8 R-1 已把这条写成退路）。

**逐声明清单**（C 类 107 条，`文件:行` + 声明名 + 触发的构件）：

| 文件:行 | 声明 | 体行 | 触发构件 |
|---|---|---:|---|
| `lib/Demo.sokonanoda:13` | `demo_and_comm` | 2 | And.left And.right |
| `lib/Demo.sokonanoda:21` | `demo_mem_powerset` | 3 | Iff.mpr |
| `lib/Demo.sokonanoda:30` | `demo_exists_elim` | 4 | Exists.elim |
| `lib/Exists.sokonanoda:99` | `Exists.imp` | 3 | Exists.elim |
| `lib/Image.sokonanoda:65` | `Set.image_mono` | 9 | Exists.elim And.left And.right |
| `lib/Image.sokonanoda:77` | `Set.image_subset_iff` | 14 | Exists.elim And.left And.right Eq.subst |
| `units/notation-cheatsheet.sokonanoda:166` | `demo_empty_subset_pointful` | 2 | False.elim |
| `units/notation-cheatsheet.sokonanoda:170` | `demo_empty_subset_notation` | 2 | False.elim |
| `units/solutions/notation-cheatsheet-solution.sokonanoda:61` | `demo_empty_subset_pointful` | 2 | False.elim |
| `units/solutions/notation-cheatsheet-solution.sokonanoda:65` | `demo_empty_subset_notation` | 2 | False.elim |
| `units/solutions/notation-cheatsheet-solution.sokonanoda:118` | `empty_union_subset` | 5 | Or.elim False.elim |
| `units/solutions/notation-cheatsheet-solution.sokonanoda:125` | `mem_union_comm` | 5 | Or.elim |
| `units/solutions/notation-cheatsheet-solution.sokonanoda:133` | `union_empty_right` | 8 | Or.elim False.elim |
| `units/solutions/unit01-solution.sokonanoda:18` | `subset_of_mem_singleton` | 3 | Eq.subst |
| `units/solutions/unit01-solution.sokonanoda:29` | `singleton_eq_singleton_iff` | 11 | Iff.mp Eq.subst |
| `units/solutions/unit02-solution.sokonanoda:18` | `empty_subset` | 2 | False.elim |
| `units/solutions/unit02-solution.sokonanoda:21` | `subset_empty_iff` | 8 | Eq.subst |
| `units/solutions/unit02-solution.sokonanoda:31` | `eq_empty_iff_forall_notMem` | 11 | Eq.subst False.elim |
| `units/solutions/unit02-solution.sokonanoda:44` | `singleton_ne_empty` | 5 | Eq.subst |
| `units/solutions/unit02-solution.sokonanoda:51` | `pair_subset_iff` | 16 | Or.elim And.left And.right Eq.subst |
| `units/solutions/unit02-solution.sokonanoda:69` | `pair_comm` | 11 | Or.elim |
| `units/solutions/unit03-solution.sokonanoda:15` | `union_subset_iff` | 12 | Or.elim And.left And.right |
| `units/solutions/unit03-solution.sokonanoda:30` | `inter_subset_left` | 2 | And.left |
| `units/solutions/unit03-solution.sokonanoda:33` | `inter_subset_right` | 2 | And.right |
| `units/solutions/unit03-solution.sokonanoda:42` | `subset_inter_iff` | 12 | And.left And.right |
| `units/solutions/unit03-solution.sokonanoda:57` | `sdiff_subset` | 3 | And.left |
| `units/solutions/unit03-solution.sokonanoda:62` | `powerset_mono` | 7 | Iff.mp Iff.mpr |
| `units/solutions/unit03-solution.sokonanoda:74` | `union_subset_inter_false` | 6 | And.right |
| `units/solutions/unit03-solution.sokonanoda:84` | `powerset_self_mem` | 4 | Iff.mpr |
| `units/solutions/unit04-solution.sokonanoda:14` | `union_comm` | 13 | Or.elim |
| `units/solutions/unit04-solution.sokonanoda:30` | `inter_comm` | 11 | And.left And.right |
| `units/solutions/unit04-solution.sokonanoda:44` | `union_inter_idem` | 16 | Or.elim And.left |
| `units/solutions/unit04-solution.sokonanoda:63` | `inter_union_distrib_left` | 25 | Or.elim And.left And.right |
| `units/solutions/unit04-solution.sokonanoda:92` | `union_inter_self` | 9 | Or.elim And.left |
| `units/solutions/unit04-solution.sokonanoda:104` | `sdiff_sdiff` | 26 | Or.elim And.left And.right |
| `units/solutions/unit04-solution.sokonanoda:134` | `union_empty` | 9 | Or.elim False.elim |
| `units/solutions/unit04-solution.sokonanoda:148` | `union_sdiff_univ_ne_univ` | 16 | And.right Eq.subst |
| `units/solutions/unit05-solution.sokonanoda:34` | `prod_mk_inj` | 7 | Eq.subst |
| `units/solutions/unit05-solution.sokonanoda:45` | `prod_mk_eq_iff` | 12 | And.left And.right Eq.subst |
| `units/solutions/unit05-solution.sokonanoda:71` | `prod_set_swap_ne` | 12 | And.left Eq.subst |
| `units/solutions/unit05-solution.sokonanoda:90` | `kura_degenerate` | 31 | Or.elim |
| `units/solutions/unit06-solution.sokonanoda:52` | `rel_comp_assoc` | 56 | Exists.elim And.left And.right |
| `units/solutions/unit06-solution.sokonanoda:139` | `equivClass_eq_iff` | 13 | Eq.subst |
| `units/solutions/unit06-solution.sokonanoda:182` | `classes_isPartition` | 84 | Exists.elim And.left And.right Iff.mpr Eq.subst let |
| `units/solutions/unit06-solution.sokonanoda:285` | `Nat.zero_ne_succ` | 3 | Eq.subst |
| `units/solutions/unit06-solution.sokonanoda:289` | `Nat.succ.inj` | 2 | congrArg |
| `units/solutions/unit06-solution.sokonanoda:303` | `nearStep_counterexample` | 54 | Or.elim let |
| `units/solutions/unit07-solution.sokonanoda:32` | `surjective_comp` | 12 | Exists.elim congrArg |
| `units/solutions/unit07-solution.sokonanoda:48` | `leftInverse_injective` | 7 | congrArg |
| `units/solutions/unit07-solution.sokonanoda:66` | `bijective_iff_inverse` | 14 | And.left And.right |
| `units/solutions/unit07-solution.sokonanoda:84` | `injective_of_comp_injective` | 3 | congrArg |
| `units/solutions/unit07-solution.sokonanoda:90` | `swap_exists_forall` | 6 | Exists.elim |
| `units/solutions/unit07-solution.sokonanoda:104` | `swap_converse_false` | 20 | Exists.elim Eq.subst |
| `units/solutions/unit08-solution.sokonanoda:36` | `preimage_union` | 22 | Or.elim |
| `units/solutions/unit08-solution.sokonanoda:81` | `preimage_inter` | 14 | And.left And.right |
| `units/solutions/unit08-solution.sokonanoda:102` | `image_union` | 45 | Exists.elim Or.elim And.left And.right |
| `units/solutions/unit08-solution.sokonanoda:162` | `image_inter_subset` | 17 | Exists.elim And.left And.right |
| `units/solutions/unit08-solution.sokonanoda:202` | `two_aa_ne_bb` | 3 | Eq.subst |
| `units/solutions/unit08-solution.sokonanoda:212` | `f1_eq_aa` | 3 | match |
| `units/solutions/unit08-solution.sokonanoda:221` | `inter_singletons_empty` | 16 | And.left And.right False.elim |
| `units/solutions/unit08-solution.sokonanoda:240` | `image_empty` | 15 | Exists.elim And.left False.elim |
| `units/solutions/unit08-solution.sokonanoda:257` | `image_inter_singletons_empty` | 9 | congrArg |
| `units/solutions/unit08-solution.sokonanoda:285` | `not_image_inter_eq_image_inter` | 21 | Eq.subst |
| `units/solutions/unit08-solution.sokonanoda:315` | `image_preimage_subset` | 9 | Exists.elim And.left And.right Eq.subst |
| `units/solutions/unit08-solution.sokonanoda:332` | `image_preimage_eq_aa` | 4 | Exists.elim |
| `units/solutions/unit08-solution.sokonanoda:338` | `bb_not_mem_image_preimage` | 6 | Exists.elim |
| `units/solutions/unit08-solution.sokonanoda:345` | `image_preimage_eq_image` | 25 | Exists.elim And.right |
| `units/solutions/unit08-solution.sokonanoda:379` | `image_preimage_image_eq_preimage` | 23 | Exists.elim And.right |
| `units/solutions/unit08-solution.sokonanoda:404` | `not_image_preimage_eq` | 11 | Eq.subst |
| `units/solutions/unit08-solution.sokonanoda:418` | `image_preimage_eq_of_subset_image_univ` | 21 | Exists.elim And.right Iff.mpr Eq.subst |
| `units/solutions/unit09-solution.sokonanoda:61` | `Nat.succ_ne_zero` | 3 | Eq.subst |
| `units/solutions/unit09-solution.sokonanoda:72` | `Set.Equiv.symm` | 39 | Exists.elim And.left And.right |
| `units/solutions/unit09-solution.sokonanoda:118` | `Set.Equiv.trans` | 103 | Exists.elim And.left And.right congrArg |
| `units/solutions/unit09-solution.sokonanoda:242` | `Set.equivOfEq` | 9 | Eq.subst |
| `units/solutions/unit09-solution.sokonanoda:258` | `not_equiv_empty_singleton` | 36 | Exists.elim And.left And.right |
| `units/solutions/unit09-solution.sokonanoda:301` | `equiv_nonempty_iff` | 54 | Exists.elim And.left And.right |
| `units/solutions/unit09-solution.sokonanoda:366` | `proper_subset_counterexample` | 28 | Eq.subst Nat.rec False.elim |
| `units/solutions/unit10-solution.sokonanoda:44` | `cantor` | 45 | Exists.elim And.right Eq.subst congrArg absurd let |
| `units/solutions/unit10-solution.sokonanoda:98` | `no_surjection_powerset` | 17 | Exists.elim Eq.subst congrArg absurd let |
| `units/solutions/unit10-solution.sokonanoda:131` | `choice_split` | 4 | let |
| `units/solutions/unit11-solution.sokonanoda:21` | `no_univ_strictly_larger` | 12 | Exists.elim And.right |
| `units/solutions/unit11-solution.sokonanoda:41` | `mem_powerset_univ` | 4 | Iff.mpr |
| `units/solutions/unit11-solution.sokonanoda:58` | `univ_top_iff` | 16 | Eq.subst |
| `units/solutions/unit12-solution.sokonanoda:24` | `xlat_compl_union` | 15 | Or.elim And.left And.right |
| `units/solutions/unit12-solution.sokonanoda:43` | `xlat_rel_inv_comp` | 22 | Exists.elim And.left And.right |
| `units/solutions/unit12-solution.sokonanoda:85` | `fix_image_preimage` | 27 | Exists.elim And.left And.right Eq.subst |
| `units/solutions/unit12-solution.sokonanoda:116` | `fix_image_inter` | 47 | Exists.elim And.left And.right Eq.subst |
| `units/solutions/unit12-solution.sokonanoda:173` | `flawed_equalities_refuted` | 156 | Exists.elim And.left And.right Eq.subst |
| `units/solutions/unit12-solution.sokonanoda:348` | `project_chain` | 69 | Exists.elim And.left And.right congrArg |
| `units/solutions/unit12-solution.sokonanoda:438` | `project_chain_cardinal` | 143 | Exists.elim And.left And.right Eq.subst congrArg |
| `units/unit04-extensionality-identities.sokonanoda:44` | `demo_union_univ` | 10 | Or.elim |
| `units/unit04-extensionality-identities.sokonanoda:57` | `demo_sdiff_self` | 7 | And.left And.right False.elim |
| `units/unit06-relations.sokonanoda:184` | `Nat.zero_ne_succ` | 3 | Eq.subst |
| `units/unit06-relations.sokonanoda:188` | `Nat.succ.inj` | 2 | congrArg |
| `units/unit08-images-preimages.sokonanoda:167` | `two_aa_ne_bb` | 3 | Eq.subst |
| `units/unit08-images-preimages.sokonanoda:177` | `f1_eq_aa` | 3 | match |
| `units/unit08-images-preimages.sokonanoda:192` | `inter_singletons_empty` | 16 | And.left And.right False.elim |
| `units/unit08-images-preimages.sokonanoda:211` | `image_empty` | 15 | Exists.elim And.left False.elim |
| `units/unit08-images-preimages.sokonanoda:229` | `image_inter_singletons_empty` | 9 | congrArg |
| `units/unit08-images-preimages.sokonanoda:262` | `image_inter_subset` | 17 | Exists.elim And.left And.right |
| `units/unit08-images-preimages.sokonanoda:304` | `image_preimage_subset` | 9 | Exists.elim And.left And.right Eq.subst |
| `units/unit09-equinumerosity.sokonanoda:96` | `Nat.succ_ne_zero` | 3 | Eq.subst |
| `units/unit10-cantor.sokonanoda:48` | `demo_fake_enum_escapes` | 7 | Eq.subst |
| `units/unit10-cantor.sokonanoda:68` | `demo_diag` | 16 | Eq.subst congrArg absurd |
| `units/unit11-universe-russell.sokonanoda:79` | `univ_subset_iff` | 13 | Eq.subst |
| `units/unit12-synthesis.sokonanoda:97` | `demo_rel_comp_assoc` | 58 | Exists.elim And.left And.right |
| `units/unit12-synthesis.sokonanoda:332` | `demo_equiv_comp` | 101 | Exists.elim And.left And.right congrArg |

### D.2 显式依赖类型消去（recursor 直接出现）

| recursor | 文件:行 | 声明 | 为什么难 |
|---|---|---|---|
| `Exists.rec` | `lib/Exists.sokonanoda:92` | `def Exists.elim` | **定义体**（不是证明）：`Exists.rec A p (fun (_ : Exists A p) => Q) f h`。这是整个 `∃` 消去的唯一入口，**不该**改 tactic；改它等于改库语义 |
| `Prod.rec` | `lib/Prod.sokonanoda:49` | `def Prod.fst` | 投影**定义**：`Prod.rec.{1} A B (fun _ => A) (fun a b => a) p`。可用 `by exact Prod.rec …`，但没有收益 |
| `Prod.rec` | `lib/Prod.sokonanoda:52` | `def Prod.snd` | 同上 |
| `Nat.rec` | `units/solutions/unit09-solution.sokonanoda:366` | `proper_subset_counterexample` | **课程里唯一一处显式归纳消去**（在证明体里！）：`fun (n : Nat) => Nat.rec …`。tactic 侧无 `induction`/`cases` ⇒ 只能 `exact` |

> `Or.rec` / `False.rec` / `Eq.rec` / `And.rec` 在课程里**零使用**（都是 prelude 内部实现）。

### D.3 高阶函数出现在**参数位**（不只是返回值位）

度量：证明体里 `(fun ` 的出现次数 = **作为实参传出去的 lambda**（返回值位的 `fun x => …` 不计数）。

**134 条已证声明**把 lambda 当实参（共 779 处）。Top 15：

| 文件:行 | 声明 | `(fun …)` 处数 | 典型位置 |
|---|---|---:|---|
| `units/solutions/unit12-solution.sokonanoda:438` | `project_chain_cardinal` | 62 | `Exists.elim` 的 motive/续延 · `And.intro` 的两支 · `congrArg` 的函数 |
| `units/solutions/unit12-solution.sokonanoda:173` | `flawed_equalities_refuted` | 49 | `Exists.elim` 的 motive/续延 · `And.intro` 的两支 |
| `units/unit12-synthesis.sokonanoda:97` | `demo_rel_comp_assoc` | 43 | `Exists.elim` 的 motive/续延 · `Iff.intro` 的两向 · `And.intro` 的两支 |
| `units/solutions/unit12-solution.sokonanoda:348` | `project_chain` | 26 | `Exists.elim` 的 motive/续延 · `Set.ext` 的逐点证明 · `Iff.intro` 的两向 · `And.intro` 的两支 · `congrArg` 的函数 |
| `units/solutions/unit09-solution.sokonanoda:118` | `Set.Equiv.trans` | 23 | `Exists.elim` 的 motive/续延 · `congrArg` 的函数 |
| `units/solutions/unit06-solution.sokonanoda:182` | `classes_isPartition` | 21 | `Exists.elim` 的 motive/续延 · `And.intro` 的两支 |
| `units/unit12-synthesis.sokonanoda:332` | `demo_equiv_comp` | 21 | `Exists.elim` 的 motive/续延 · `congrArg` 的函数 |
| `units/solutions/unit08-solution.sokonanoda:102` | `image_union` | 17 | `Exists.elim` 的 motive/续延 · `Set.ext` 的逐点证明 · `Iff.intro` 的两向 · `And.intro` 的两支 · `Or.elim` 的两支 |
| `units/solutions/unit09-solution.sokonanoda:301` | `equiv_nonempty_iff` | 16 | `Exists.elim` 的 motive/续延 · `Iff.intro` 的两向 |
| `units/solutions/unit06-solution.sokonanoda:52` | `rel_comp_assoc` | 15 | `Exists.elim` 的 motive/续延 · `Iff.intro` 的两向 · `And.intro` 的两支 |
| `units/solutions/unit12-solution.sokonanoda:43` | `xlat_rel_inv_comp` | 15 | `Exists.elim` 的 motive/续延 · `Iff.intro` 的两向 · `And.intro` 的两支 |
| `units/solutions/unit12-solution.sokonanoda:116` | `fix_image_inter` | 13 | `Exists.elim` 的 motive/续延 · `Set.ext` 的逐点证明 · `Iff.intro` 的两向 · `And.intro` 的两支 |
| `units/unit10-cantor.sokonanoda:68` | `demo_diag` | 12 | `congrArg` 的函数 |
| `units/solutions/unit06-solution.sokonanoda:303` | `nearStep_counterexample` | 11 | `And.intro` 的两支 · `Or.elim` 的两支 |
| `units/solutions/unit09-solution.sokonanoda:366` | `proper_subset_counterexample` | 11 | `And.intro` 的两支 |

**为什么难**：`Exists.elim A p Q h (fun w hw => …)` 里的续延 lambda 携带**两个新 binder**，
tactic 侧要 `obtain ⟨w, hw⟩ := h`（需要 `obtain` + `⟨⟩` 语法）。`Set.ext … (fun x => …)` 要 `apply Set.ext; intro x`。
没有这两样，`(fun …)` 只能原样留在 `exact` 里。

### D.4 证明里的 `let` / `match`

| 文件:行 | 声明 | 形态 | 为什么难 |
|---|---|---|---|
| `units/solutions/unit06-solution.sokonanoda:182` | `classes_isPartition` | 已证 | `let` 引入局部辅助（在 `Or.elim` 分支里）⇒ 同上 |
| `units/solutions/unit06-solution.sokonanoda:303` | `nearStep_counterexample` | 已证 | `let` 引入局部辅助（在 `Or.elim` 分支里）⇒ 同上 |
| `units/solutions/unit10-solution.sokonanoda:44` | `cantor` | 已证 | `let` 绑定**带类型标注的局部定义**（如 `let MF : (α -> Set α) -> Prop := …`）；tactic 侧无 `let`/`have` ⇒ 必须 `have` 或整段 `exact` |
| `units/solutions/unit10-solution.sokonanoda:98` | `no_surjection_powerset` | 已证 | `let` 绑定**带类型标注的局部定义**（如 `let MF : (α -> Set α) -> Prop := …`）；tactic 侧无 `let`/`have` ⇒ 必须 `have` 或整段 `exact` |
| `units/solutions/unit10-solution.sokonanoda:131` | `choice_split` | 已证 | `let` 绑定**带类型标注的局部定义**（如 `let MF : (α -> Set α) -> Prop := …`）；tactic 侧无 `let`/`have` ⇒ 必须 `have` 或整段 `exact` |
| `units/solutions/unit08-solution.sokonanoda:212` | `f1_eq_aa` | 已证 | **`match` 作为证明体**（`theorem … := match x with \| aa => … \| bb => …`）——tactic 侧有 `match` tactic 但臂体是项，等价 `exact` |
| `units/unit08-images-preimages.sokonanoda:177` | `f1_eq_aa` | 已证 | **`match` 作为证明体**（`theorem … := match x with \| aa => … \| bb => …`）——tactic 侧有 `match` tactic 但臂体是项，等价 `exact` |

另外 **12 个 `def` 用 `match` 定义**（`Nat.isZero` ×4、`Nat.pred` ×4、`isAa` ×2、`f1` ×2）——它们是**数据/谓词定义**，不是证明，见 D.6。

### D.5 `Eq.subst` / `congrArg` / `Eq.mp` / `cast`

| 构件 | 声明数 | 处数 | 风险 |
|---|---:|---:|---|
| `Eq.subst` | 37 | 55 | **无 `rw`**：26 处目标位（可类比 `rw [h]`）、29 处参数位（`rw … at h`）、其中 **7 处是命题级重写**（`fun (Q : Prop) => Q`，`rw` 也救不了）——见 §3.5.2 |
| `congrArg` | 14 | 19 | 5 处把 `congrArg` 用到 `Prop`（`(Set α) → Prop`），是「命题即集合」的核心手法 |
| `Eq.mp` / `Eq.mpr` / `cast` / `Eq.ndrec` | 0 | 0 | 课程**零使用**（prelude 内部实现） |

### D.6 `def` 清单：**不该**改成 tactic 的 71 个声明

> 判据：`def` 的体是**数据/谓词/函数的构造**，不是证明。改成 `by` 只会得到 `by exact <原体>`，
> 语义相同但可读性更差，而且会把「定义」和「证明」两种东西混在一起。**建议保留项模式。**

全课程 **71 个 `def`**，分三类：

| 类 | 个数 | 说明 | 处置 |
|---|---:|---|---|
| (a) 词汇/谓词/数据定义 | 56 | `Set.*` 词汇、`Function.*` 谓词、`Rel.comp`、`Set.prod`/`Set.graph`、`EquivClass`/`classes`、`IsPartition`… | **保留项模式** |
| (b) `match` 定义 | 12 | `Nat.isZero`×4、`Nat.pred`×4、`isAa`×2、`f1`×2 | **保留项模式**（`match` 是数据定义的正当写法） |
| (c) recursor 定义 | 3 | `Exists.elim`、`Prod.fst`、`Prod.snd` | **保留项模式**（可用 `by exact …`，但无收益） |

**逐条清单**（`文件:行` · 名字 · 类）：

| 文件:行 | 名字 | 类 | 体行 |
|---|---|---|---:|
| `lib/Equiv.sokonanoda:95` | `Set.MapsTo` | (a) 词汇/数据 | 2 |
| `lib/Equiv.sokonanoda:100` | `Set.LeftInvOn` | (a) 词汇/数据 | 2 |
| `lib/Equiv.sokonanoda:105` | `Set.RightInvOn` | (a) 词汇/数据 | 2 |
| `lib/Equiv.sokonanoda:111` | `Set.Equiv` | (a) 词汇/数据 | 6 |
| `lib/Exists.sokonanoda:92` | `Exists.elim` | (c) recursor | 2 |
| `lib/Fun.sokonanoda:60` | `Function.comp` | (a) 词汇/数据 | 2 |
| `lib/Fun.sokonanoda:65` | `Function.Injective` | (a) 词汇/数据 | 2 |
| `lib/Fun.sokonanoda:70` | `Function.Surjective` | (a) 词汇/数据 | 2 |
| `lib/Fun.sokonanoda:73` | `Function.Bijective` | (a) 词汇/数据 | 2 |
| `lib/Fun.sokonanoda:78` | `Function.LeftInverse` | (a) 词汇/数据 | 2 |
| `lib/Fun.sokonanoda:83` | `Function.RightInverse` | (a) 词汇/数据 | 2 |
| `lib/Fun.sokonanoda:88` | `Function.Inverse` | (a) 词汇/数据 | 2 |
| `lib/Image.sokonanoda:30` | `Set.image` | (a) 词汇/数据 | 2 |
| `lib/Image.sokonanoda:35` | `Set.preimage` | (a) 词汇/数据 | 2 |
| `lib/Prod.sokonanoda:49` | `Prod.fst` | (c) recursor | 2 |
| `lib/Prod.sokonanoda:52` | `Prod.snd` | (c) recursor | 2 |
| `lib/Rel.sokonanoda:36` | `Rel` | (a) 词汇/数据 | 1 |
| `lib/Rel.sokonanoda:39` | `Rel.inv` | (a) 词汇/数据 | 2 |
| `lib/Rel.sokonanoda:45` | `Rel.comp` | (a) 词汇/数据 | 3 |
| `lib/Set.sokonanoda:51` | `Set` | (a) 词汇/数据 | 1 |
| `lib/Set.sokonanoda:55` | `Set.mem` | (a) 词汇/数据 | 1 |
| `lib/Set.sokonanoda:57` | `Set.subset` | (a) 词汇/数据 | 1 |
| `lib/Set.sokonanoda:59` | `Set.empty` | (a) 词汇/数据 | 1 |
| `lib/Set.sokonanoda:61` | `Set.univ` | (a) 词汇/数据 | 1 |
| `lib/Set.sokonanoda:63` | `Set.singleton` | (a) 词汇/数据 | 1 |
| `lib/Set.sokonanoda:65` | `Set.pair` | (a) 词汇/数据 | 2 |
| `lib/Set.sokonanoda:68` | `Set.union` | (a) 词汇/数据 | 1 |
| `lib/Set.sokonanoda:70` | `Set.inter` | (a) 词汇/数据 | 1 |
| `lib/Set.sokonanoda:72` | `Set.sdiff` | (a) 词汇/数据 | 1 |
| `lib/Set.sokonanoda:74` | `Set.compl` | (a) 词汇/数据 | 1 |
| `lib/Set.sokonanoda:76` | `Set.powerset` | (a) 词汇/数据 | 1 |
| `units/solutions/unit05-solution.sokonanoda:20` | `Set.prod` | (a) 词汇/数据 | 2 |
| `units/solutions/unit06-solution.sokonanoda:20` | `Reflexive` | (a) 词汇/数据 | 1 |
| `units/solutions/unit06-solution.sokonanoda:22` | `Symmetric` | (a) 词汇/数据 | 1 |
| `units/solutions/unit06-solution.sokonanoda:24` | `Transitive` | (a) 词汇/数据 | 1 |
| `units/solutions/unit06-solution.sokonanoda:26` | `EmptyRelation` | (a) 词汇/数据 | 1 |
| `units/solutions/unit06-solution.sokonanoda:127` | `EquivClass` | (a) 词汇/数据 | 1 |
| `units/solutions/unit06-solution.sokonanoda:161` | `classes` | (a) 词汇/数据 | 2 |
| `units/solutions/unit06-solution.sokonanoda:164` | `IsPartition` | (a) 词汇/数据 | 8 |
| `units/solutions/unit06-solution.sokonanoda:277` | `Nat.isZero` | (b) match | 3 |
| `units/solutions/unit06-solution.sokonanoda:281` | `Nat.pred` | (b) match | 3 |
| `units/solutions/unit06-solution.sokonanoda:295` | `nearStep` | (a) 词汇/数据 | 2 |
| `units/solutions/unit08-solution.sokonanoda:198` | `isAa` | (b) match | 3 |
| `units/solutions/unit08-solution.sokonanoda:207` | `f1` | (b) match | 3 |
| `units/solutions/unit08-solution.sokonanoda:216` | `U01` | (a) 词汇/数据 | 1 |
| `units/solutions/unit08-solution.sokonanoda:329` | `image_preimage` | (a) 词汇/数据 | 2 |
| `units/solutions/unit09-solution.sokonanoda:51` | `Nat.isZero` | (b) match | 3 |
| `units/solutions/unit09-solution.sokonanoda:55` | `Nat.pred` | (b) match | 3 |
| `units/solutions/unit09-solution.sokonanoda:66` | `nonzero` | (a) 词汇/数据 | 1 |
| `units/solutions/unit10-solution.sokonanoda:18` | `Set.Countable` | (a) 词汇/数据 | 2 |
| `units/unit05-pairs-products.sokonanoda:100` | `Set.prod` | (a) 词汇/数据 | 2 |
| `units/unit05-pairs-products.sokonanoda:150` | `Set.graph` | (a) 词汇/数据 | 2 |
| `units/unit06-relations.sokonanoda:48` | `Reflexive` | (a) 词汇/数据 | 1 |
| `units/unit06-relations.sokonanoda:50` | `Symmetric` | (a) 词汇/数据 | 1 |
| `units/unit06-relations.sokonanoda:52` | `Transitive` | (a) 词汇/数据 | 1 |
| `units/unit06-relations.sokonanoda:55` | `EmptyRelation` | (a) 词汇/数据 | 1 |
| `units/unit06-relations.sokonanoda:115` | `EquivClass` | (a) 词汇/数据 | 1 |
| `units/unit06-relations.sokonanoda:145` | `classes` | (a) 词汇/数据 | 2 |
| `units/unit06-relations.sokonanoda:148` | `IsPartition` | (a) 词汇/数据 | 8 |
| `units/unit06-relations.sokonanoda:176` | `Nat.isZero` | (b) match | 3 |
| `units/unit06-relations.sokonanoda:180` | `Nat.pred` | (b) match | 3 |
| `units/unit06-relations.sokonanoda:193` | `nearStep` | (a) 词汇/数据 | 2 |
| `units/unit08-images-preimages.sokonanoda:162` | `isAa` | (b) match | 3 |
| `units/unit08-images-preimages.sokonanoda:172` | `f1` | (b) match | 3 |
| `units/unit08-images-preimages.sokonanoda:182` | `U01` | (a) 词汇/数据 | 1 |
| `units/unit08-images-preimages.sokonanoda:188` | `image_preimage` | (a) 词汇/数据 | 1 |
| `units/unit09-equinumerosity.sokonanoda:88` | `Nat.isZero` | (b) match | 3 |
| `units/unit09-equinumerosity.sokonanoda:92` | `Nat.pred` | (b) match | 3 |
| `units/unit09-equinumerosity.sokonanoda:100` | `nonzero` | (a) 词汇/数据 | 1 |
| `units/unit10-cantor.sokonanoda:46` | `fake_enum` | (a) 词汇/数据 | 1 |
| `units/unit10-cantor.sokonanoda:96` | `Set.Countable` | (a) 词汇/数据 | 2 |

**另有 6 个 `axiom` + 4 个 `inductive`**（同样不该动）：

| 文件:行 | 关键字 | 名字 | 说明 |
|---|---|---|---|
| `lib/Exists.sokonanoda:86` | `inductive` | `Exists` | `Exists` 真归纳（G-03/0.59.0），构造子 `Exists.intro` |
| `lib/Prod.sokonanoda:44` | `inductive` | `Prod` | `Prod` 归纳（序对） |
| `lib/Rel.sokonanoda:51` | `axiom` | `Rel.ext` | 关系外延性公理 |
| `lib/Set.sokonanoda:79` | `axiom` | `Set.ext` | 集合外延性公理（Mathlib 由 propext+funext 推出，本语言没有） |
| `units/solutions/unit08-solution.sokonanoda:191` | `inductive` | `Two` | 同上（解答本地副本） |
| `units/solutions/unit10-solution.sokonanoda:127` | `axiom` | `Exists.choose` | 同上（解答本地副本） |
| `units/solutions/unit10-solution.sokonanoda:128` | `axiom` | `Exists.choose_spec` | 同上（解答本地副本） |
| `units/unit08-images-preimages.sokonanoda:156` | `inductive` | `Two` | 两元素类型 `Two`（`aa`/`bb`），画布本地 |
| `units/unit10-cantor.sokonanoda:170` | `axiom` | `Exists.choose` | 选择公理（画布本地，为综合题准备） |
| `units/unit10-cantor.sokonanoda:172` | `axiom` | `Exists.choose_spec` | 选择公理（画布本地，为综合题准备） |

### D.7 记法改写的 7 个边界（实测，逐条给复现）

| # | 边界 | 实测 | 影响 |
|---|---|---|---|
| 1 | **`→` 不可用** | `example (A B : Prop) : A → B → A := …` → `elab-unknown-identifier: unknown identifier '→'`；`token.rs` 只把 `∀` 收成原生 token（`TokenKind::Forall`），`->` 是内建语法、**没有「箭头常量」可做记法目标** | 用户要求的 `→` **必须改语言**（设计文档 L2.1 已列为「必须」） |
| 2 | `∧ ∨ ↔ ¬` 可声明且可用 | 实测四条记法 + 5 个 example 全部 `example_checked` | 无需改语言（但设计文档 L2.2 想把它们内建化） |
| 3 | `∀` 原生，但**必须写类型标注** | `∀ (x : α), p x -> p x` ✅；`∀ x, p x -> p x` → `elab-untyped-binder: types must be written explicitly on Pi binders` | `forall (x : A), …` → `∀ (x : A), …` 是**一对一替换**，不引入风险 |
| 4 | `∃` 走 `binder_notation`，**目标必须在作用域**，且**只能在表达式开头**（实参位要加括号） | 无 `import lib.Exists` 时 `binder_notation "∃" => Exists` → `elab-notation-unknown-target`；`-> ∃ (y : α), …` → 「`∃` 是一元记法符号：它要跟自己的操作数一起写」 | 单元 1–7 若要用 `∃`，**必须补 `import lib.Exists`**；且库里声明 `∃` 后要删掉 `unit08:141` 的本地声明（重复声明 = parse 错，S4 的 T5 冲突） |
| 5 | **`∅` 在 `Eq` 的操作数位置解不出 `α`** | 记法对照页 `:52-59` 已记录：`Eq.{1} (Set α) A ∅` → `elab-notation-argument-unsolved`；`A ⊆ ∅` / `a ∈ ∅` / `A ∪ ∅` 都行 | **课程里 37 处 `Eq.{1} (Set …) … (Set.empty …)`**（如 `unit02:63/71`、`unit04:146`、`unit08:212/241`、`unit06:149`）**只能继续写 `Set.empty α`**——除非 L2.4 的 `=` 糖落地（`A = ∅` 里 `∅` 有期望类型） |
| 6 | **同一符号全课程只能声明一次** | 重复声明是 parse 错（记法对照页 `:40-43`）；`lib/Set` 已声明 `𝒫 ᶜ '' ⁻¹' ×ˢ` | `∈ ⊆ ∪ ∅` 要全课程启用 ⇒ 必须**移进 `lib/Set`** 并删掉记法对照页 + 解答里的 4 条 |
| 7 | **`×ˢ` 的目标 `Set.prod` 定义在单元⑤ 画布**（不在 lib） | `lib/Set:158` 声明 `infixr:80 " ×ˢ " => Set.prod`，目标名在**使用点**解析 | 单元 1–4 不能用 `×ˢ`（`Set.prod` 不在作用域）；要用得先把 `Set.prod` 挪进 lib |

> 附：**`=` 也声明不出来**（实测 `infix:50 "=" => Eq` 与 `infix:50 " = " => Eq` 都 parse 错，`=>` 前就炸）。
> 所以 `Eq.{1} (Set α) A B` → `A = B` **必须走语言侧的内建糖**（设计文档 L2.4），记法命令做不到。
---

## 5. 文档 / 元数据清单（E）

### 5.1 `courses/set-theory/course.json`（`soko.course/2`）

结构：**1 卷 · 4 章 · 12 单元**；章级 `prereqs` 是 **chapter id**（G6 判据：不悬空）；`quota.exercises` **只报告不判红**。

| 章 id | 章标题 | prereqs | tags | quota.exercises | unit | 画布文件 | 解答文件 |
|---|---|---|---|---:|---:|---|---|
| I.1 | 集合、子集与集合运算 |  | membership · subset · powerset | 35 | 1 | `units/unit01-sets-membership.sokonanoda` | `units/solutions/unit01-solution.sokonanoda` |
|  |  |  |  |  | 2 | `units/unit02-subsets-empty.sokonanoda` | `units/solutions/unit02-solution.sokonanoda` |
|  |  |  |  |  | 3 | `units/unit03-union-inter-powerset.sokonanoda` | `units/solutions/unit03-solution.sokonanoda` |
|  |  |  |  |  | 4 | `units/unit04-extensionality-identities.sokonanoda` | `units/solutions/unit04-solution.sokonanoda` |
| I.2 | 序对、关系与函数 | I.1 | ordered-pair · product · relation · function | 24 | 5 | `units/unit05-pairs-products.sokonanoda` | `units/solutions/unit05-solution.sokonanoda` |
|  |  |  |  |  | 6 | `units/unit06-relations.sokonanoda` | `units/solutions/unit06-solution.sokonanoda` |
|  |  |  |  |  | 7 | `units/unit07-functions.sokonanoda` | `units/solutions/unit07-solution.sokonanoda` |
| I.3 | 像、原像与基数 | I.2 | image · preimage · cardinality · cantor | 23 | 8 | `units/unit08-images-preimages.sokonanoda` | `units/solutions/unit08-solution.sokonanoda` |
|  |  |  |  |  | 9 | `units/unit09-equinumerosity.sokonanoda` | `units/solutions/unit09-solution.sokonanoda` |
|  |  |  |  |  | 10 | `units/unit10-cantor.sokonanoda` | `units/solutions/unit10-solution.sokonanoda` |
| I.4 | 论域、悖论与综合 | I.1, I.3 | universe · russell · synthesis | 14 | 11 | `units/unit11-universe-russell.sokonanoda` | `units/solutions/unit11-solution.sokonanoda` |
|  |  |  |  |  | 12 | `units/unit12-synthesis.sokonanoda` | `units/solutions/unit12-solution.sokonanoda` |

**改写会不会动 course.json？**

* **不动**：`file`/`title`/`title_en`/`unit`（文件名与标题不变）、章结构、`prereqs`、`tags`。
* **可能要动**：`quota.exercises`（35/24/23/14）——**只在练习数变化时**。当前画布实测练习数 = 35+24+23+14 = **96**（画布 `exercise.open` 合计），
  与配额逐章相等；记法对照页的 3 道练习**不进** course.json（它不是单元）。
* ⚠️ 若为记法/演示**新增 lib 模块**（设计文档 S1 的做法），门禁目标数 **36 → 37**，`summary` 全变——但这不改 course.json。

### 5.2 `courses/set-theory/README.md` 的「现状」表

| 行 | 内容 | 改写后 |
|---|---|---|
| `:124` | 标题「现状（2026-09-19，12 单元全部落地）」 | 日期 + 轮次需更新 |
| `:126` | **36 个目标 · 329 checked · 99 open · 0 判负** | **必须重算**（重跑 `check.py --json`） |
| `:127-128` | `canvas_open 96 / solutions_open 0 / lib_open 0`；「P4 前是 355 checked」 | 前三个数必须重算；历史对比句可留 |
| `:129` | 「清单 v2 不改判卷计数」 | 历史陈述，保留 |
| `:130-134` | 「数字只作现状记录…」+ `manifests: 1` / 12 单元 / ledger 第一条（36/329/99/0/20024ms/v0.60.0） | 12 单元不变；**计数与 ms 必须重算** |
| `:140-143` | 四章「计划练习 / 画布实测」（35/35、24/24、23/23、14/14） | 练习数不变则不动 |
| `:145-158` | 12 单元「练习 / 解答（checked）」表（6/6、10/10、11/11、8/8、7/8、8/23、9/9、9/30、7/13、7/9、5/5、9/9） | **解答 checked 全部要重算**（lib 改动会传导） |
| `:160-163` | 记法对照页「18 checked + 3 练习 / 解答 21 checked」 | 重算；**页面若重定位（C1.5）措辞也要改** |
| `:165-190` | §「记法（G-04 第二刀）」整节 | **前提被推翻**：D4 之后全课程都用记法，本节要重写（「第一刀/第二刀」的叙事保留，但「课程侧用法」段要改） |
| `:192-196` | `lib/` 逐模块计数（Logic 0 · Set 23 · Exists 3 · Prod 6 · Rel 7 · Fun 14 · Image 6 · Equiv 5 · Demo 10） | **全部要重算** |
| `:198-203` | `lib/Logic` 空壳 + 「34 个 import」「65 处项位裸名」 | 34 个 import 不变；**65 处裸名已不存在（改写后归零）** |
| `:205-212` | `lib/Exists` 升级 + 「`units/` 里 186 处点名调用」 | **186 处会随记法改写大幅减少**，必须重算 |
| `:214-223` | 单元 DoD 第 2/5 条（`sorry` 三段 hint / `check.py` 全绿） | 「练习 = 带 `sorry` 的声明」→ 改成 `:= by sorry` |

### 5.3 `courses/set-theory/sokonanoda.toml`

```toml
name = "set-theory"
requires = "0.61"
```

**必须同步**：`requires` 是**版本钉**（`scripts/soko` 的解析链会用它）。本轮语言侧要加 `→`/`=`/`⟨⟩`/tactic
⇒ 版本会 bump（feature ⇒ minor）⇒ **`requires` 必须改成新版本**，否则 `scripts/soko grade` 会用旧二进制判新语法、
直接 exit 3（版本不一致）。

### 5.4 写死了计数 / 会变成假话的地方（全仓）

> 与 S4 的 `docs/notes/course-lean-style/tooling-impact.md` §2.2/§7 互补：那里按「会不会红」组织，
> 这里按「**数字会不会变**」组织。

#### (1) 课程目录内

| 文件:行 | 写死了什么 | 改写后 |
|---|---|---|
| `README.md:126/127/128/133` | 36 目标 · 329 checked · 99 open · canvas_open 96 · 355（历史） | **重算** |
| `README.md:140-143` | 四章配额/实测 35·24·23·14 | 练习数不变则不动 |
| `README.md:145-158` | 12 单元「练习/解答 checked」24 个数 | **重算** |
| `README.md:163` | 记法页 18/3/21 | **重算** |
| `README.md:192-195` | lib 逐模块 0/23/3/6/7/14/6/5/10 | **重算** |
| `README.md:201-202` | 34 个 import · 65 处裸名 | 34 不变；65 归零 |
| `README.md:209` | 186 处点名调用 | **重算（会大降）** |
| `lib/Set.sokonanoda:140` | `-- 36 目标 / 329 checked / 99 open 逐项不变）` | **重算**（S4 也点了这一条） |
| `units/solutions/unit11-solution.sokonanoda:3/8` | 「5 条练习」「decl_checked 应当正好是 5」 | 声明数不变则不动，但「term 风格」措辞要改 |
| `units/solutions/unit05-solution.sokonanoda:10` | 「decl_checked = 7 条练习 + 1 条 def」 | 同上 |
| `units/solutions/unit06-solution.sokonanoda:5` | 「画布上 8 道练习…0 个 sorry」 | 同上 |
| `units/solutions/unit12-solution.sokonanoda:9` | 「声明数 = 练习数 = 9」 | 同上 |
| `units/solutions/notation-cheatsheet-solution.sokonanoda:4` | 「9 对演示 + 3 道练习」 | 同上 |
| `units/unit09-equinumerosity.sokonanoda:35` | 「本单元的 7 条练习」 | 同上 |
| `units/unit12-synthesis.sokonanoda:459` | 「9 条练习同名一一对应」 | 同上 |

#### (2) 仓库根 / 设计文档（**与课程计数同源，必须一起更新**）

| 文件:行 | 写死了什么 |
|---|---|
| `STATUS.md:51/56/123-124/132/236/255` | 36/329/99/0 · 20024ms · v0.60.0/v0.61.0 |
| `STATUS.md:189` | **355 checked（已过期）** |
| `REQUIREMENTS.md:1670/1687/1751` | 36 目标 · 329 checked · 99 open |
| `REQUIREMENTS.md:1727` | **355 checked（已过期）** |
| `REQUIREMENTS.md:1500` | 34 目标 · 308 checked · 93 open（更旧） |
| `docs/HANDOVER.md:344/403/441` | 315/96 · 355/99 · 99 |
| `docs/design/teaching-project.md:457/463/480/520/527` | 36/355/99 · 329/99 · 34/308 · 20024ms |
| `docs/design/course-stdlib.md:63/292-293/349` | 329/99 · 36 目标 |
| `docs/design/course-gate-in-ci.md:28-30/95-98/156/236/246/266` | 34/296/93 · 34/315/96 · 36/329/99/20024ms/v0.60.0 |
| `docs/design/set-theory-syllabus.md:174-180` | 93 道练习 / 308 checked |
| `docs/design/ctor-namespace.md:73/144-145` | 34 目标 · 315 checked · 96 open |
| `docs/design/course-manifest-v2.md:241/260/315/325/348` | 36/329/99 |
| `docs/design/namespace-open.md:413/501` | 36/329/99 |
| `docs/design/prelude-l1-proposal.md:44/338` | 329/99 · 65 处裸名 |
| `docs/design/notation-subset.md:498/507/675` | 36/329/99 |
| `docs/design/eq-type-level-rewriting.md:228` | 36/329/99 |
| `docs/notes/course-lean-style/tooling-impact.md` §2.2/§7 | 上述清单的另一份（S4 产出） |
| `docs/STATUS-ARCHIVE.md:36/88/135/3416/3462` | 34/315/96 · 34/308/93（**归档，不该改**） |
| `scripts/soko:912` | 注释 `34 targets`（旧） |
| `docs/TESTING.md:100` | 「36 个目标不重复解析启动器」 |

#### (3) 机器会读的（**必须重跑，不许手写**）

| 文件 | 怎么更新 |
|---|---|
| `site/data/site.json` | `python3 scripts/gen-site-data.py`（`counts_source: "gate"`，日志必须出现「门禁实测」而不是「沿用上次实测的计数」——S4 §5 的静默坑） |
| `docs/courses/ledger.jsonl` | `python3 courses/set-theory/tools/check.py --ledger` 追加一条（**旧行不改**，它是趋势点） |
| `courses/set-theory/course.json` | 仅结构变化时手改（见 §5.1） |

### 5.5 会红 / 会挡路的测试与复现件（**必须原子同轮**）

> 完整清单见 S4 `tooling-impact.md` §6/§8；这里只列**与课程内容直接相关**的：

| # | 位置 | 钉住什么 | 为什么会红 |
|---|---|---|---|
| T1 | `crates/cli/tests/notation.rs:567-582` | 「课程**仍用点名**」：禁止课程出现 `infix`/`notation` 行 + unit02 含 `Set.subset α A C` | D4 直接推翻其前提 ⇒ **改判据，不是删测试** |
| T2 | `notation.rs:308/312/384` | unit02 两行签名逐字 + `open >= 8` | 签名改写 ⇒ 重钉 |
| T3 | `notation.rs:527-544` | `lib/Set:155-159` 五行记法逐字 | 记法搬家 ⇒ 重钉 |
| T4 | `notation.rs:548-563` | unit03 的 `𝒫 ` / unit08 的 `'' ` 演示 | 演示改写 ⇒ 重钉 |
| T5 | `notation.rs:768-780` | unit08 的 `binder_notation` + `∃ (` | **与「`∃` 搬进库」正面冲突**（重复声明 = parse 错） |
| T6 | `notation.rs:451` | 拷贝真 `lib/Set` 后 `∈` 可用 | 随 T3 |
| T7 | `crates/cli/tests/query.rs:569-570` | unit05 `decl_checked==5` / `exercise_open==7` | 计数或写法变了就红（建议改成「与 `grade` 同判」） |
| T8 | `crates/cli/tests/course_manifest.rs:338-339/316-323` | 12 单元 / ≥4 章 / quota>0 | 只动结构才红 |
| T9 | `docs/gaps/repro/G07-course-manifest-v2.sh:68/84-85/146` | 1 卷/4 章/12 单元/4 prereqs/4 tags/4 quotas | 同上 |

### 5.6 课程内的「教学叙事」语句（不是数字，但会变成假话）

| 文件:行 | 原话（节选） | 为什么必须改 |
|---|---|---|
| `units/unit03-union-inter-powerset.sokonanoda:9` | 本单元只允许用什么（逐步解锁） | 「本单元只允许用」清单要换成新 tactic 集（C1.6） |
| `units/unit03-union-inter-powerset.sokonanoda:10` | 证明：term 风格 `fun`，或 by 块里的 intro / exact / apply / assumption / rfl / match / sorry | 白名单会扩；「本单元全部题目 term 风格就能做完」不再成立 |
| `units/unit03-union-inter-powerset.sokonanoda:24` | 本单元**不允许**（语言还没有 notation，台账 G-04）：∈ ⊆ ∪ ∩ \ 𝒫 ∅ 这些记号——一律点名 | **前提被 D4 推翻**（G-04 已 fixed，全课程要用记法） |
| `units/unit04-extensionality-identities.sokonanoda:7` | 本单元只允许用…（项风格或 `by` 块都行）；没有记法（G-04），一律点名 | 同上 |
| `units/unit07-functions.sokonanoda:20` | 本单元只允许用什么…本单元 9 题**全部可以用 term 风格**写完，推荐这样写 | D3 要求演示与练习都改 tactic 风格 |
| `units/unit10-cantor.sokonanoda:12` | 本单元只允许用：… | 同 unit03 |
| `units/unit11-universe-russell.sokonanoda:11` | （项风格或 `by` 块都行）；没有记法（G-04），一律点名 | 同上 |
| `units/unit12-synthesis.sokonanoda:17` | **本单元只允许用**：语言子集：term 风格（`fun (x : A) => …`）或 by 块…没有记法（G-04），一律点名 | 同上 |
| `units/unit08-images-preimages.sokonanoda:23` | （tactic 只在 `by` 块里；推荐 term 风格 `fun (x : A) => …`） | 同上 |
| `units/unit08-images-preimages.sokonanoda:0.920863309352518` | 课程门禁的 `decl.checked` 计数因此一个都不动 | 记法不是声明 ⇒ 这句仍然对；但**若把记法搬进 lib/Set，lib 的声明数不变**，也仍然对——保留 |
| `units/solutions/unit07-solution.sokonanoda:3` | 全部是 **term 风格**（没有一个 `by` 块）——与画布「推荐 term 风格」一致 | **直接变成假话** |
| `units/solutions/unit03-solution.sokonanoda:3` | 全部证明只用 term 风格（fun / Or.inl / Or.inr / Or.elim / And.intro / And.left / And.right） | 同上 |
| `units/solutions/unit12-solution.sokonanoda:9` | 全部是 **term 风格**，且**不引入任何辅助声明** | 同上 |
| `units/solutions/notation-cheatsheet-solution.sokonanoda:14` | 三题的证明都只用 term 风格（fun / Or.elim / Or.inl / Or.inr / Iff.intro / …） | 同上 |
| `units/notation-cheatsheet.sokonanoda:11.805194805194805` | 课程故意先教点名形式…其它单元的画布**继续用点名形式**——记法是本页单独教的第二遍，不是全卷改写 | **与 D4 正面冲突**：本页要重定位成「记法速查表」（C1.5） |
| `courses/set-theory/README.md:77` | 其它单元的画布**继续用点名形式**——记法是本页单独教的第二遍，不是全卷改写 | 同上 |
| `courses/set-theory/AGENTS.md:8` | 画布：演示 + 练习（`sorry`） | 改成 `:= by sorry`；写作循环第 2 步措辞 |
| `docs/design/set-theory-syllabus.md:0.912621359223301` | §4「记法引入顺序与无记法替代」 | 改成「记法为主 + 点名作为底层解释」（C1.7） |

### 5.7 一句话总结：改写后**数字会不会变**

| 数字 | 会不会变 | 原因 |
|---|---|---|
| 顶层声明数 **424** | **不变**（除非增删声明） | 记法不是声明；`:= sorry` → `:= by sorry` 不增声明 |
| **checked 329 / open 99** | **不变**（若只改写法） | `:= by sorry` 与 `:= sorry` 事件相同（设计文档 §1.2 实测）；**但**新增 lib 模块会让目标数 36→37、checked 变 |
| 目标数 **36** | **可能变** | 加 lib 模块 ⇒ 37；`example` 不计 checked，若把演示从 `theorem` 改成 `example` 会掉 checked |
| 行数 **6550** | **会变（涨）** | `by` 块通常比项模式长；README 没写行数，但设计文档 §1.1 写了 |
| hint **294** | 可能变 | 改写 hint 时若增删条数 |
| 配额 35/24/23/14 | **不变**（练习数不变） | 门禁不锁配额，差额只报告 |

---

## 6. 附录：改写工作量与建议顺序

| 工作 | 量 | 说明 |
|---|---:|---|
| 记法替换（签名 + 体） | 点名叫法 **1077 处**（And 469 · Or 85 · Iff 76 · Exists 157 · forall 113 · Not 177；含 `def` 体与 axiom/inductive） | 机械但量大；`∅`/`Eq` 边界要人工判断 |
| 证明改 `by` | **245 条已证 `theorem`/`example`**（A 74 / B 64 / C 107，含 `False.elim` 口径；不含则 79/64/102）+ 1 个 `def Exists.elim` | C 类依赖语言侧 tactic 扩展 |
| 练习占位改 `:= by sorry` | **99 条** | 零风险 |
| hint 改写 | **294 条**（108 条含项模式词汇） | 独立工作量，人工复核 |
| `def` 保留项模式 | **70 条**（另 1 个 `def Set.Countable` 是开放练习 `:= sorry`） | 明确不改 |
| axiom/inductive | **10 条** | 明确不改 |
| 元数据同步 | §5 的 4 张表 | 与改写**同 commit** |

建议顺序（与设计文档 §4 C6 的 S0–S11 一致，这里只补「清单视角」的检查点）：

1. **S0 留基线**：`check.py --json` 存 36/329/99 + 逐目标 names —— 用本文档 §1.1 的表逐项核对；
2. **S1 先立记法**（只加库、不动单元）——注意本文档 §4.7 的 7 条边界；
3. **S2 单单元试点**（建议单元①，最小且不在测试硬断言里）——用本文档 §2 的表确认「一条都没漏」；
4. **S4 铺开**：每改完一个文件，用本文档 §1.1 的「声明/已证/开放」三列对账；
5. **S6 原子改测试**：§5.5 的 T1–T9；
6. **S7 元数据**：§5.2/§5.4/§5.6 三张表逐行过。
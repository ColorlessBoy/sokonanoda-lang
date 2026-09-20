# R2 课程 Lean 化改写简报（subagent 用）

> 这是**施工说明书**，不是设计文档。设计在 `docs/design/course-lean-style.md`
> （R2 分期与 as-built）与 `docs/design/notation-subset.md`（记法语义）。
> 本文件只写"改写时手上有什么、怎么写、怎么验、怎么报"。

## 0. 一句话任务

把**一个单元**的画布 + 解答改成 Lean 4 风格：**连接符与集合符号用记法**，
**证明过程用 `by` tactic 块**。声明名、顺序、题意、题目数量**一个都不许变**。

## 1. 手上有什么（**只准用这些**，多一个都没有）

### 1.1 记法（符号 → 点名形式）

`import lib.Set` 之后可用（由课程库统一声明，**单元里不许再声明**）：

| 记法 | 点名形式 | 优先级 |
|---|---|---|
| `x ∈ A` | `Set.mem α x A` | 50 |
| `A ⊆ B` | `Set.subset α A B` | 50 |
| `A ∪ B` | `Set.union α A B` | 65（左结合） |
| `A ∩ B` | `Set.inter α A B` | 70（左结合） |
| `A \ B` | `Set.sdiff α A B` | 70（左结合） |
| `∅` | `Set.empty α` | 零元 |
| `𝒫 A` | `Set.powerset α A` | 前缀 100 |
| `Aᶜ` | `Set.compl α A` | 后缀 100 |
| `f '' A` | `Set.image α β f A` | 80（右结合） |
| `f ⁻¹' B` | `Set.preimage α β f B` | 80（右结合） |
| `A ×ˢ B` | `Set.prod α A B` | 80（右结合） |

`import lib.Exists` 之后可用：`∃ (x : α), p x`（`binder_notation "∃" => Exists`）。

**语言内建**（任何文件零声明，也不许重声明）：
`∧`（`And`，35）、`∨`（`Or`，30）、`↔`（`Iff`，20）、`¬`（`Not`，40）、
`=`（`Eq`，50）、`≠`（`Ne`，50）；`→` 是 `->` 的**词法别名**（不是记法）；
`∀` 是关键字（原生 Pi）。`A = B` / `A ≠ B` 的宇宙层级由判卷器按操作数类型解，
**两档都能写**（`A B : Prop` 与 `A B : Set α`）。

**集合字面量**（内建糖，1–2 个元素）：`{a}` = `Set.singleton α a`、
`{a, b}` = `Set.pair α a b`。空 `{}` 与 ≥3 个元素是 parse 错 ⇒ 写 `Set.empty α`
或嵌套 `Set.pair α a (Set.pair α b c)`。元素类型从期望类型/操作数类型解，
**解不出时报 `elab-set-literal-*`**：这时给外层一个期望类型（例如写进 `∈` 的
左操作数、或 `Eq` 的一边）就行。

**括号纪律**：前缀记法在**实参位**要加括号（`f (𝒫 A)`，不写 `f 𝒫 A`）；
`A ᶜ` 与 `𝒫 A` 的优先级 100，比 `∪ ∩` 紧。

### 1.2 tactic（白名单，**只有这些**）

| tactic | 形状 | 说明 |
|---|---|---|
| `intro` | `intro x` / `intro h1 h2` | 目标 `∀ x, …` / `A -> B` / `¬ A` / `A ⊆ B`（自动 delta 展开） |
| `exact` | `exact t` | 类型必须与目标**内核 defeq**；tactic 风格里最常用 |
| `apply` | `apply f` | 用 `f` 的结论对齐目标，其余参数变子目标 |
| `assumption` | `assumption` | 上下文里正好有一条同型假设 |
| `rfl` | `rfl` | **只认头是 `Eq` 的目标**（`Set.mem`/`∈` 这类 def 头不行） |
| `match` | `match h with \| ctor … => …` | 消去归纳值（等价于 `cases` 的表达式形式） |
| `constructor` | `constructor` | 目标头归纳有两个构造子时拆成两个子目标（`∧`/`↔`/`Iff`…） |
| `left` / `right` | `left` | 目标头是 `Or`（`∨`） |
| `use` | `use t` | 目标 `∃ (x : α), p x`：交出证人，剩下的目标是 `p t` |
| `exfalso` | `exfalso` | 目标换成 `False` |
| `cases` | `cases h with \| ctor … => …` | 消去假设；臂体是**缩进**块，臂内可再写 tactic |
| `have` | `have h : T := t` / `have h : T := by …` | 引入中间事实，**目标不变**；嵌套 `by` 按缩进界定 |
| `sorry` | `sorry` | 只出现在**练习**里 |

**没有的（不要写，写了就是 parse/elab 错）**：`rw`、`obtain`、`change`、`simp`、
`induction`、`exact?`、`·`（focus 点）、`calc`、`rintro`。

**已经有的两条便利**（2026-09-21 落地，直接用）：

- **匿名构造子 `⟨a, b⟩`**（L2.7）：用哪个构造子由**期望类型**决定，支持
  `∧`（`And.intro`）、`↔`（`Iff.intro`）、`∃ (x : α), p x`（`Exists.intro`）、
  `Prod`（`Prod.mk`）与**单构造子归纳**。例子：
  `exact ⟨ha, hb⟩`（目标 `A ∧ B`）、`exact ⟨w, hw⟩`（目标 `∃ (x : α), p x`）。
  **嵌套 `⟨a, ⟨b, h⟩⟩`（`∃ x, ∃ y, …`）今天不支持**（期望类型解不出内层的
  `p`）——那种形状用 `use a` / `use b` / `exact h` 分步写，或点名 `Exists.intro`。
- **多层展开**：`intro` / `constructor` / `left` / `right` / `use` 会把 def 头
  逐层展开到 Pi / 归纳头（最多 4 层）。所以 `A ∈ 𝒫 B` 上可以直接
  `intro x`、`a ∈ B ∩ C` 上可以直接 `constructor`（不必先搬成员判定引理）。
- **`intro` 会按你写的名字改名**（`intro y` 之后目标里的绑定名就是 `y`），
  `have` 在 `intro` 之后可用。

**点名形式的引理照常可用**：`exact And.intro A B ha hb`、
`exact Or.elim A B C h f g`、`exact Iff.intro A B h1 h2`、
`exact Exists.intro α p w hw`、`exact Eq.subst.{1} α (fun (x : α) => …) a b h hpa`
——**记法是连接符的糖，不是引理名的糖**：`Set.mem_singleton_self`、
`Set.notMem_empty`、`Or.inl` 这些名字照旧点名写。

## 2. 怎么写

1. **画布**：
   - 演示（`demo_*`）**保持已证**（`:= by …`，不许 `sorry`）；
   - 练习**一律** `:= by` + 缩进两格的 `sorry`（学生在这里写 tactic）；
   - `-- soko:hint` 三段（思路 / 目标形态 / 关键件）**必须跟着改写**：
     目标形态用记法写（`{a} ≠ ∅`、`A ⊆ B`、`∃ (x : α), …`）；
     关键件只写"触发条件 + 该用哪条引理"，**答案绝不进 hint**；
   - 单元开头"本单元只允许用什么"那段要跟着 tactic 白名单更新。
2. **解答**（`units/solutions/<同名>-solution.sokonanoda`）：
   - 语句与画布**逐字一致**（记法也一样）；
   - 证明用 tactic 块，**每条都要 `decl.checked`**（0 open、0 diagnostic）；
   - 顺序与画布练习一致，一条不少。
3. **不许**：改声明名 / 改题意 / 增删声明 / 声明记法（`infix`/`notation`）/
   改 `lib/` / 改 `course.json` / 改 `crates/` / 改 `docs/`。
4. 原来写在注释里的"点名形态"说明可以留（它解释语义），但**语句与证明体
   不许有点名残留**（引理名不算残留）。

## 3. 怎么验（**判据是判卷器，不是肉眼**）

```bash
# 画布：期望 exit 0；演示 decl.checked、练习 exercise.open，条数与改前相同
scripts/soko grade "$PWD/courses/set-theory/units/<unit>.sokonanoda" --json | tail -3; echo "EXIT=${PIPESTATUS[0]}"

# 解答：期望 exit 0 且 checked == 题目数、open == 0、无 diagnostic
scripts/soko grade "$PWD/courses/set-theory/units/solutions/<unit>-solution.sokonanoda" --json | tail -3; echo "EXIT=${PIPESTATUS[0]}"

# 只跑本单元的课程门禁（改完再跑）
python3 courses/set-theory/tools/check.py --only "单元 N"
python3 courses/set-theory/tools/check.py --only "解答 unitNN"
```

- **一律绝对路径**（台账 G-12：相对路径会被模块根解析坑到）；
- 不要跑 `cargo`（会与并行的构建打架）；用 `scripts/soko`；
- 判"有没有坏"看 **`grade` 的退出码**，不看事件文本；
- 改前先记下两个计数（`decl.checked` / `exercise.open`），改后必须**逐项相同**；
- 判卷器拒绝时读 `diagnostic.message`：`elab-notation-argument-unsolved` =
  前导类型参数解不出（写出点名形式或给期望类型）；
  `elab-set-literal-*` = 字面量元素类型解不出；
  `` unknown universe level `u` `` = 撞到语言缺口，**停下来报告**。

## 3.5 已知的引擎边界（**别撞**，撞了就绕）

- `cases` 落在 `constructor`/`apply` 造出的**子目标**里（臂体要组装成 `match`
  当实参）⇒ `elab-match-no-expected-type`。绕法：臂体包一层
  `have h : T := by cases …`（`have` 的标注提供期望类型）。
- `apply Set.ext`（不写类型参数）在**元素类型不是签名里的裸 `α`** 时（例如
  `Set (Set α)`）会造出坏子目标。绕法：写全 `apply Set.ext (Set α)`。
- `by` 只能出现在声明值位与 `have` 的值位，**不能**出现在实参位
  （`exact f (by intro x; …)` 是 parse 错）。绕法：先 `have` 再 `exact`。
- `∃` 的嵌套匿名构造子（见上）。
- **操作数全是闭项**的集合记法（`Set.univ Nat ∪ Set.univ Nat`）解不出前导参数
  （`∈`/`=`` 不受影响；有一个操作数是变量就没事）。绕法：那一条写点名形式。
- **`cases` 消去「谓词里含 `≠`」的 `∃`** 时，臂里的假设类型会带 `Ne.{0}`，
  后面每条 tactic 都报「期望 Sort(0)，实际是 Sort(1)」。绕法：改用
  `Exists.elim` + 一条**带类型标注的 lambda**（`exact (fun (h : ∃ …) => Exists.elim … h (fun U hU => …))`），
  或让谓词里不出现 `≠`（`A ≠ U` 写成 `Not (Eq.{1} (Set α) A U)` 也能绕）。

## 4. 卡住了怎么报（**不许糊弄**）

写不动的时候**不要**：留点名证明、给演示塞 `sorry`、删题、改题意。
按下面格式把阻塞报回来（这是最有价值的产出——它决定语言侧下一刀补什么）：

```
阻塞：<文件>:<行> <声明名>
想要的 Lean 写法：<逐字>
实际报错：<diagnostic.code + message 逐字>
今天能过的替代写法：<逐字；没有就写"没有">
猜缺什么：<例如「缺 rw」「缺 ⟨a, b⟩ 匿名构造子」「缺 obtain」>
```

## 5. 收尾（每个 subagent 都要交）

- 改动的文件清单（画布 / 解答）；
- 改前 → 改后的计数（画布、解答各一对）；
- 两条 `check.py --only` 的退出码；
- 阻塞清单（可以是空）；
- **不要**改 `STATUS.md` / 设计文档 / 台账——那是我（主 agent）的收尾工作。

## 6. `lib/` 与速查页的额外规则（I5 用）

`lib/` 是**基础设施**（L2 层），不是练习：**一个 `sorry` 都不许有**，
`check.py --only "lib <名>"` 必须 exit 0 且 `open == 0`。

1. **语句用记法**：`lib/Set` 自己声明记法（文件末尾那段 `infix`/`notation`），
   所以它**自己的定理**默认用不了——除非把那一段**移到所有 def 之后、定理之前**
   （记法是文件内作用域，声明点之后才生效）。其它 lib 模块 `import lib.Set`
   即可直接用。
2. **证明改 tactic 块**：`fun (h : …) => h` 这种一行项写成
   `by intro h; exact h`。定义（`def`）的**体**不动——那是词汇，不是证明。
3. **名字/顺序/签名（除拼写）一个都不许变**：lib 的名字被 units/ 与解答大量引用。
4. **判卷**：`scripts/soko grade "$PWD/courses/set-theory/lib/<文件>" --json`，
   期望 exit 0、`decl.checked` 与改前**相同**、`exercise.open == 0`、0 diagnostic；
   再跑 `python3 courses/set-theory/tools/check.py --only "lib <标签>"`。
5. 改完**必须**跑一遍 `python3 courses/set-theory/tools/check.py`（整门课）确认
   没有单元被带红——lib 是全课程的公共依赖。
6. 速查页（`units/notation-cheatsheet.sokonanoda` + 解答）是**对照页**：
   `demo_*_pointful` 与 `demo_*_notation` 成对出现是**故意的**（它教两种写法），
   不要删掉任何一半；要改的是**证明体**（改 tactic 风格）与**散文/hint 里过时的
   说法**（例如"本文件自己声明记法"）。

## 7. 入门课 `course/` 的额外规则（R3 用）

入门课**不是**卷 I 的翻版：它**用 axiom 搭逻辑骨架**（`axiom And` / `axiom And.intro` …），
而且 **CN/EN 双语必须逐字同构**。动手前先读
`docs/notes/course-lean-style/intro-course-constraints.md`（Q1/Q4/Q5/Q6/Q7 是硬约束）。

1. **CN 与 EN 的代码行必须逐字相同**（注释各写各的）：`crates/cli/tests/course.rs::en_mirrors_match_chinese_event_counts`
   逐项比事件计数。改 CN 不改 EN = 红。
2. **不许 `import`**：`course_shared.rs::the_shared_library_replaces_rather_than_duplicates_the_canvas_purpose`
   禁止画布/解答的代码行出现 `import `（单元⑪ 的项目目录例外，它本来就是模块课）。
3. **不许动共享骨架块**：`axiom And …` / `inductive Or …` / 显式 `Nat` 块是
   **规范文本**，在 24/12/8 份副本里逐字一致（`every_unit_copy_matches_the_canonical_module`
   双向守卫）。**只改证明与语句的拼写，不改这些块**。
4. **`∧`/`∨` 在入门课里指向本文件声明的 `And`/`Or`（axiom）**，不是 prelude 的归纳
   ⇒ `⟨a, b⟩` 在那里**可能不适用**（`⟨⟩` 要单构造子归纳）。拿不准就写点名
   `And.intro A B ha hb`——入门课的白名单本来就只教 `intro/exact/assumption/apply/rfl`
   （单元④），**不要**给入门课塞卷 I 的 tactic。
5. **事件计数会变**（`decl.checked` / `expr.reduced` 等），
   `crates/cli/tests/course.rs` 的 `GOLDEN`、`course_status.rs` 的 `GOLDEN`、
   `cli.rs` 的 86/65 都要**同轮重钉**——那是主 agent 的活，subagent 只需报告
   **每个文件改后的五元计数**。
6. `course/README.md` / `course.json` / `unit11-project/*` 里写死的计数与措辞
   （见 Q7 的清单）同轮同步。
7. 判卷命令：`scripts/soko grade "$PWD/course/unitN-….sokonanoda" --json`、
   `scripts/soko grade "$PWD/course/solutions/unitN-…-solution.sokonanoda" --json`、
   以及 `course/en/…` 的孪生。**每对 CN/EN 的五元计数必须相等**。

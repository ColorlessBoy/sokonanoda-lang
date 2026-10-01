# AGENTS.md —— 课程线 agent 手册（卷 I 集合论）

你在语言仓 `sokonanoda-lang` 内的课程目录 `courses/set-theory/` 工作。
语言侧的一切（判卷器、台账工具、缺口实现）都在同一棵树里。

## 记法规则（2026-09-21 起，硬规则，脚本判红）

**课程一律写 Lean 4 记法，不写「点名 + 前导类型/宇宙实参」的旧写法。** 判据一条命令：

```bash
python3 scripts/notation-lint.py                 # 全课程（卷 I + 入门课 + playground）
python3 scripts/notation-lint.py --root <file>   # 单文件
```

- `Eq.{1} T a b` → `a = b`（用户原话：`Eq.{1}` 直接就是一个等于号）；`Ne` 同理；
  `And/Or/Iff/Not/forall/Exists/->` → `∧ ∨ ↔ ¬ ∀ ∃ →`；`Set.*` → `∈ ⊆ ∪ ∩ \ ᶜ 𝒫 ∅ '' ⁻¹' ×ˢ {a} {a,b}`；
  **复合** → `g ∘ f`（`Function.comp`，lib/Fun）与 `r • s`（`Rel.comp`，lib/Rel）
  —— 两个都是二元算子，**应用到参数上要加括号**：`(g ∘ f) x`、`(r • s) a c`。
- 基础类型**省前导隐式实参**（对齐 Lean）：`And.intro h1 h2` / `And.left h` /
  `Or.inl h` / `Exists.intro w hw` / `Exists.elim h f`……能判绿就省。
- **代码与注释（含 `-- soko:hint`）都算**；`units/notation-cheatsheet*.sokonanoda`
  整文件豁免（它是教学装置）。
- **明说的边界**（保留点名 + 行内 `-- soko:notation-ok: <理由>`）：等式族证明项
  （`Eq.refl/symm/trans/subst`、`congrArg`）的宇宙层级；`congrArg` 参数顺序是
  Lean 的 `{α β} {a b} (f) (h)`；`Set.univ α`；
  `intro` 派生的目标/假设、`Exists`-headed def、嵌套 `Exists.elim`、
  **字面 λ 体内含零元构造**的 `''`/`⁻¹'`。
  ——**2026-09-26 更新（R5 修好，边界收窄一格 ✓）**：λ **操作数本身**能用记法
  （`(fun …) '' A` 含 `⁻¹'` ✓，2026-09-25 就成立），而**复合记法操作数**
  （`{∅} ∩ {(Set.univ Nat)}`、`(…) '' {∅}`）曾是**另一个**卡点（前导类型参数解不出
  ——`Set α` 是 def、操作数类型文本却是箭头形态，头对不上），**现已修**
  （`solve_prefix_args` 两条路线都补了 delta 展开兜底 ✓，判据
  `crates/front/src/compile/tests.rs::notation_solves_leading_parameters_when_the_expected_type_is_an_arrow` ✓）
  ⇒ `unit12-synthesis` 的 `flawed_equalities_refuted` 从**整条点名**（5 个标记）
  改成**除 λ 体外全记法**（3 个标记）✓。
  仍然卡住的只剩 **λ 体内的 `∅`**：它没有类型来源（λ 的陪域要等期望类型，而期望
  类型又依赖 λ 的类型 ⇒ 循环），Lean 同样解不了（要 `(e : T)` 标注或元变量）
  ⇒ 要么保留点名 + 行内标记（现状 ✓），要么等语言给出类型标注 `(e : T)` ✓。
  另：**注释里的点名写法同样判红**（`--` 注释、`soko:hint` 都算 ✓）
  ⇒ 讨论这条边界时别在注释里写出点形式的例子 ✗（实测踩过 ✓）；
  以及 `And.left h x` 这类**续应用**、`Set.univ α` 等（见上）。
  **`by rfl` 已能认 `=` 记法目标**（2026-09-21 修）。
  细则见 `docs/notes/course-lean-style/notation-rewrite-brief.md`。
- **画布（`units/*.sokonanoda`）里的 tactic 块不动，term 保持 term**；纯记法改写必须
  **计数中性**（判据 G1–G6 与规模无关、**不锁计数** ✗：别把某次的门禁数字写进文档 ✓
  —— 当前数字永远现跑 `python3 courses/set-theory/tools/check.py` 取 ✓）。

## 解答写法：**项风格**（2026-09-21 用户拍板，**覆盖**早先的"tactic 先不动"）

> 「tactic 先不动」是我写的，但是现在看性能差太多了，solution 没必要损失性能。

`solutions/*.sokonanoda` 一律写**项风格**（lambda / 直接给证明项），**不用 `by`**。

**为什么**：`by` 块的判定代价是 **O(前缀 × `by` 块数)**——每个 `by` 块都要把整份
前缀重新 elaborate 一遍。实测 `unit12-solution`（25 个 `by` 块）冷跑 **120 秒**，
其中判定占 **68%**；同一批定理写成项风格实测快 **8.4×**，`unit12-solution` 那种
大文件 **25×**（`docs/PERF.md` 有分阶段表）。

**解答同时是"提示"**：学习者做画布上的题卡住时，老师（LLM）读解答拿到**证明骨架
与关键件**，再用 tactic 写法讲给学习者听。两边一一对应：

| 项风格 | tactic 写法 |
|---|---|
| `fun (h : P) => e` | `intro h; exact e` |
| `f a b` | `apply f; exact a; exact b`（或 `exact f a b`） |
| `Iff.intro P Q h1 h2` | `constructor; exact h1; exact h2` |
| `Set.ext α A B (fun x => …)` | `apply Set.ext; intro x; …` |

写解答时**必须**：
1. 声明签名逐字不变（只换 `:=` 后面的证明项）；
2. 不用 `sorry`、不削弱命题；
3. 保留全部教学注释与 `-- soko:notation-ok: …` 标记；
4. 判绿（`node scripts/soko grade "<绝对路径>"` 退出码 0、无诊断）。

**已知陷阱（G-30）**：`Iff.intro` / `And.intro` 的分支里含集合字面量（`{a}`）时，
**前导 Prop 实参必须显式写出来**（`Iff.intro ({a} = {b}) (a = b) …`），否则报
`期望 Sort(0)，实际是 (Set.[] $2)`。

## 写作循环（每个单元一轮）

1. 读大纲（`docs/design/set-theory-syllabus.md` §3 的单元表）与分层判据
   （`docs/design/course-stdlib.md`）：**先决定这道题是 L2（进库）还是 L3（练习）**；
2. 写画布：演示（已证）+ 练习（`sorry`）+ `-- soko:hint` 三段；
3. 写解答并判卷：`node scripts/soko grade "$PWD/courses/set-theory/units/solutions/<file>"`；
4. `course.json` 加行；跑 `python3 courses/set-theory/tools/check.py`；
5. 撞到缺口 → `gaps/` 记一条（最小复现 + 期望的 Lean 4 语义 + 今天的表现），
   再收编进 `docs/gaps/ledger.jsonl`（`python3 scripts/gap.py list` 看全貌）；
6. 收尾：README 的"现状"表 + `docs/design/teaching-project.md` 的进度。

## 判卷纪律（三条，全部有实测教训）

- **判据用 `grade` 的退出码**判"有没有坏"——课程门禁保持这一条不变；G-10 已修
  （≥0.59.0）：`query check` 对解析失败也带 parse 诊断并 exit 1，可以拿它与 `grade`
  交叉复核（两条通道同口径）；
- **一律给绝对路径**（台账 G-12）；
- **span 可信，量具要选对**：`span.offset` 是**字节**偏移，`span.start/end.line`/`column`
  才是内核给的行列——要行号就直接用事件自带的 `line`/`column`（`query check` 的
  `failed[].start_line`/`start_col` 同一批数字），**别**拿 offset 去切字符（中文、`α`
  是多字节，量出来会偏 —— 台账 G-15 原来记的「span 漂到后面的声明」就是这么来的假象）。
  二分法（"截断到第 N 个声明再判卷"）留作解析失败/多条错误时的定位手段。

## 写作纪律（教学侧）

- **答案绝不进 hint**：关键件只写"触发条件 + 该用哪条引理"；
- **每句教学主张要么给出处、要么标"设计判断"**（大纲 §1.4 引用纪律：有序对与选择公理
  **没有**实证研究，不得写成"研究表明…"）；
- **提示密度随单元递减**（前三个单元每题都有 hint，中段只给关键件，后段只给思路）；
- 单元开头声明"本单元只允许用什么"（对应 stg4 的逐步解锁）。

## 零 cargo

课程线与语言线都用 `scripts/soko`（版本锁定、幂等）。不要为跑课程装 Rust；
需要改语言时才进贡献者路径（`scripts/soko gate`）。

## 库审计清单（2026-10-01 立；三次实证换来的）

**每一次写 `axiom` / `def`，先答三问**（答不上来就别写 ✗）：

1. **退化实例**：把它实例化到**最退化的参数**上（`fun _ _ => True`、空类型、单点类型、
   常值函数……）**还成立吗**？
   —— *实证* **L-08**：`axiom regularity` 对**任意**二元关系断言良基性 ⇒ 取 `E := fun _ _ => True`
   即得 `False` ⇒ 公理使库**不一致** ✗（已降级成定义 `IsRegular` ✓）。
2. **有没有居民**：这个 `def` 的**实例存在吗**？（构造一个最小的出来试试 ✗）
   —— *实证* **L-07**：非严格 `IsWellOrder` 配「极小」版 `HasMin` ⇒ 只在**空论域**上可满足，
   单元⑬⑭ 的良序定理**空真** ✗（已改成「最小」版 `HasLeast` + 新增 `IsStrictWellOrder` ✓）。
3. **两步配对**：**极小 vs 最小**、**存在 vs 唯一**、**子集 vs 元素**……两个方向的定义
   **配得上吗**？（第 1 问查"太强"，第 3 问查"对不上" ✗）

**报缺口（G 系列）之前必须做的两件事**（否则会报假缺口 ✗）：

- **先把库里已有的公理与引理找一遍**：`grep -rn "^axiom \|^theorem \|^def " lib/`；
  —— *实证* **G-77 误报**：单元㉜㉞㉟ 把"函数相等要外延"记成缺口 ✗，而 `lib.Extensionality`
  **一直提供** `propext`/`funext` ✓，只是没人 import 它 ✗ ⇒ **"没见过"不等于"不存在"** ✓。
- **复现件要能独立站住**：`.sokonanoda` 复现件在 `docs/gaps/repro/` 下**解析不到 `lib.*`** ✗
  （模块根不同）⇒ 需要库的复现件写成 **`.sh`**（判据 = 跑一条真判卷 ✓，退出码约定见
  `docs/gaps/README.md`）✓。

**每次往 `lib/` 加声明之后，跑一次闭包守卫**（L-09 换来的）：

```bash
bash courses/set-theory/tools/check-lib-closure.sh   # 把所有 lib 一次 import 完再判卷，必须 exit 0
```

—— *实证* **L-09**：`lib/Fun` 与 `lib/SUnion` **都定义 `Set.pi`** ✗，各自单文件都绿 ✓，
而**同时 import 就炸** ✗；课程门禁按**单元**判卷 ⇒ 只要没有单元同时 import 两者，
冲突就**永远不暴露** ✗。**"每个模块单独绿" ≠ "库是可组合的"** ✓
守卫现在**两步**：先做**跨模块重名扫描**（`awk '/^(def|theorem|axiom) /{print $2}'` + `uniq -d` ✓，
报出重名的名字与所在文件 ✓），再跑"全量 import"判卷 ✓ —— 2026-10-01 实测**无重名** ✓。

> **另一条纪律**：探针报红时**先怀疑探针自己** ✗ —— 第 573 轮就踩过：`Set.Equiv.mk`
> 判红看着像库缺陷 ✗，实际是我**少写了两个证明参数** ✓（补全后仍是
> **G-73 第十五例**：def-headed 期望类型处构造子补不出前导实参 ✗ ⇒ 绕法 = 先展开、
> 再用 `Set.Equiv.elim` ✓）。**判"库有缺陷"之前，先把参数个数与期望类型逐字对一遍** ✓

## 写作铁律：`Or.elim` 的**动机位**只放"肯定形状"的命题（2026-10-01 立的）

**两例实证**（都来自课程写作，不是猜的）：

| 动机位写成 | 结果 | 台账 |
|---|---|---|
| `False` | **判红** ✗（「期望 `Sort(0)`，实际是 …」） | **G-73 第十二例** |
| `¬ P`（= `P → …`） | **判红** ✗（同上，两种写法都试过） | **G-73 第十六例** |

**可操作规则**：`Or.elim P Q R f g h` 里的 `R` 只写**肯定、且形如 `P`' 或 `P' ∧ Q'` 的命题** ✓；
要证**否定式**或 `False` ⇒ **别用它当动机** ✗，改成两种手法之一：

1. **先拆肯定命题、再把否定前提当函数作用** ✓
   （单元㊻ `symmDiff_self_apply`：两次 `Or.elim` 动机分别是 `A x` 与 `¬ (A x)`，
   后者是**函数**位置、不是动机位 —— 两者不同 ✗✓）；
2. **`False.elim` 把目标写全** ✓（不给宇宙实参）。

> 连红 3 次就换招或降级 —— 第 576 轮就是照这条办的：单元㊼ 候选卡住 ⇒ **不硬推、记账、降级** ✓

### 配套惯用法：**绑定形态先"落地"成 λ，再继续应用**（2026-10-01 第 579 轮验证 ✓）

从 `↔`（两边是 `def` 头）里取**绑定形态**（`∀ …`）时，**一次到位的应用会判红** ✗：

    -- ✗ 判红：Iff.mp hu hx A hA   （绑定形态没"落地"就直接继续应用）
    -- ✓ 判绿：分两步
    (fun (hv : ∀ (B : Set α), B ∈ F → x ∈ B) => hv A hA)
      ((fun (hu : x ∈ ⋂F ↔ ∀ (B : Set α), B ∈ F → x ∈ B) => Iff.mp hu hx)
        (Set.mem_sInter_iff α F x))

**判据**：`courses/set-theory/units/solutions/unit48-solution.sokonanoda` 的
`sInter_subset_of_mem` / `sInter_mono`（单元㊽ 练习 9/10 ✓）。

### 第三条惯用法：**`⊆` 补不出类型参数时，按内核提示改点名**（2026-10-01 第 584 轮 ✓）

「`def` 套 `def` 头」的目标（如 `Set.sInter α (F ∪ G) ⊆ Set.sInter α F`）上，**`⊆` 记法
补不出前导类型参数** ✗ —— 而**内核会直接把出路写在诊断里**：

    记法 `⊆` 展开成 `Set.subset` 时补不出前面的类型参数：请写出点名形式

⇒ **照它写 `Set.subset α A B` 立刻判绿** ✓（行内加 `-- soko:notation-ok: …` ✓）。
**教训**：诊断说"请写出点名形式"时**就是字面意思**，别去猜第三种拼法 ✓
（判据：`unit49-solution.sokonanoda` 的 `sInter_union_subset_left` ✓）。

### 动机位规则**再收窄一格**：`∨` 套 `∧` 也不行

`Or.elim P Q (A x ∨ (B x ∧ C x)) …` 判红 ✗（单元㊿ 的 `union_inter_subset_dual`）——
动机位里的命题**不仅要是肯定的，还别在里面套 `∧`** ✗；
需要这种结论时，**拆成两条更弱的引理**（先给 `A x ∨ B x`）再组合 ✓。
**教训**：上一轮这两条被撤掉时记的是"`Iff.mp` 方向不可用" ✗ —— **其实是拼法问题** ✓；
**撤掉之前先花一分钟试第二种拼法** ✓（本轮 3 行探针就试出来了 ✓）。

## 写作铁律：**命题先求真，再求可证**（2026-10-01 立的；一轮里逮到自己 4 条假命题）

第 581 轮写单元㊾ 时，**四条候选命题里两条是假的** ✗ —— 都是"看着像对偶、其实不对偶"：

| 假命题 | 为什么不成立 |
|---|---|
| `x ∉ ⋃F → x ∈ ⋂F` | **反了**：`⋃F` 与 `⋂F` 是靠**补集**对偶的，不是靠这条蕴含 ✗（`F` 空时更明显） |
| `⋂F ∪ ⋂G ⊆ ⋂(F ∪ G)` | `x ∈ ⋂F` 对 `G` 的成员**什么也没说** ✗ —— 真命题是**反方向**的 `⋂(F ∪ G) ⊆ ⋂F` ✓ |

**规矩**：写下一个命题后，**先花 30 秒找反例**（取 `F = ∅`、取单点族、取"一个成员不含 `x`"）
✓ —— 找得到就**改命题**（改成真方向 ✓），找不到再动手证 ✓。
**为什么值得单列**：判卷器能把"证不出来"顶回来 ✓，但**证得出错误命题的证明并不存在** ✗ ——
若不先求真，浪费的是整轮时间（本轮就是：三条声明判红，其中两条根因是**命题本身错** ✗）。

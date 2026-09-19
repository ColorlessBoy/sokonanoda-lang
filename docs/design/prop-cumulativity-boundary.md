# 设计：累积性与 Prop 消去边界（L-06，2026-09-19）

> 台账：`docs/gaps/ledger.jsonl` 的 **L-06**（两条边界：① 没有累积性
> `def T : Type := <Prop 值>` 被拒；② `Exists.elim` 的 `Q` 只能是 `Prop`）。
> 姊妹篇：`docs/design/eq-type-level-rewriting.md`（L-03：Type 层重写）。
> **内核零改动**（硬规则 1）。本轮唯一的代码改动在
> `crates/front/src/compile/error.rs`：**只加**一个错误码 + hint + 分类
> （既有码语义不变），以及 `docs/protocol.md` 的错误码清单一行。
> 课程侧现状与绕法见 §5。

## 0. 一句话

两条都是**内核性质**（`Prop` 不是 `Type` 的子集；Prop 归纳不许大消去），冻结内核下
**不改**；能做且做了的是**诊断质量**：把 ① 的裸类型不匹配认成一条有名字、有人话的
教学错误（`kernel-prop-not-cumulative`），② 的边界连同课程绕法写清进本文与台账
（`workaround`）。

## 1. 边界 ①：没有累积性（实测）

```
def T : Type := True
```

| | 实测 |
|---|---|
| 0.59.0 及更早 | `kernel-rejected`：``类型不匹配：期望 `Sort(1)`，实际是 `Sort(0)` ``（exit 1） |
| 0.60.0（本轮） | **`kernel-prop-not-cumulative`** + 人话 hint（同一条内核消息，只换分类与提示；exit 1） |

同形状的其它写法实测同码：`def T : Type := And True True`（`Sort(1)` vs `Sort(0)`）。
反方向（该写 `Prop` 却写了 `Type`，`def f : Prop := Nat`）**故意不归此码**——见 §4。

**为什么是内核性质**：`Prop = Sort 0`、`Type 0 = Sort 1`，内核的类型检查要求
`Sort(n)` 与 `Sort(m)` **相等**（`def_eq`），不做 `n ≤ m` 的累积子类型。官方 Lean 4
有累积性（`Prop ⊆ Type`），所以同一段代码在 Lean 里能过——这正是台账把它记成
"库/语言边界"而不是"学习者错误"的原因。要改就是改内核判定，硬规则 1 不允许。

## 2. 边界 ②：`Exists.elim` 的 `Q` 只能是 Prop（实测）

```
inductive Exists (A : Type) (p : A -> Prop) : Prop
ctor intro (w : A) (h : p w) : Exists A p
end
def Exists.elim (A : Type) (p : A -> Prop) (Q : Prop) (h : Exists A p) (f : (w : A) -> p w -> Q) : Q :=
  Exists.rec A p (fun (_ : Exists A p) => Q) f h
def Exists.witness (A : Type) (p : A -> Prop) (h : Exists A p) : A :=
  Exists.rec A p (fun (_ : Exists A p) => A) (fun (w : A) (hw : p w) => w) h
```

实测：

- `Exists.elim`（`Q : Prop`，常值 motive）⇒ ✅ `checked declaration Exists.elim`；
- `Exists.witness`（想取数据）⇒ `kernel-rejected`：
  ``类型不匹配：期望 `Pi (x : ((Exists.[] $2) $1)), Sort(0)`，实际是 `Pi (_ : …), Sort(1)` ``（exit 1）；
- 自动派生的消去子**有 0 个宇宙参数**（不是"默认 0"）：
  `#check Exists.rec.{1}` ⇒ ``elab-universe-arity: constant `Exists.rec` expects 0 universe argument(s), got 1``。

**为什么是内核性质**：`Exists` 是 Prop、单构造子、构造子有**自有数据字段** `w : A`
（既不是参数也不是索引），内核的 `large_elim_test_aux` 判 `false` ⇒
`mk_elim_level` 给 `elim_level = 0` ⇒ motive 只能落 `Prop`。这正是官方 Lean 4 的规则
（`Exists.rec` 在 Lean core 里同样只到 `Prop`）：证明无关性要求同一个 Prop 的证明
可互换，若允许把证人取出来，"存在"就变成"所有证人相等"。课程侧文件
`courses/set-theory/lib/Exists.sokonanoda:39-55` 早已把这条写成给学习者的说明。

**对照（别混）**：`Eq` 形状（单构造子、构造子**没有**自有字段）**大消去是给的**——
`EqT.rec.{1}` 的 motive 落 `Type 0`（实测见
`docs/design/eq-type-level-rewriting.md` §1.2）。同是"Prop 归纳"，判据不同。

## 3. 本轮做了的（诊断质量，不是"修好边界"）

`crates/front/src/compile/error.rs`：

1. 新 `ErrorKind::KernelPropNotCumulative`（stage `kernel`，code
   **`kernel-prop-not-cumulative`**）+ hint：
   > 这里需要 Type（数据），但你给的是 Prop（命题）：本语言没有累积性，Prop 不是
   > Type 的子集（官方 Lean 4 有累积性，同一段代码在 Lean 里能过）。把陈述改成
   > Prop（例如等势用 Set.Equiv … : Prop 这样的命题版），或者交一个真正的 Type 值（如 Nat）。
2. 分类器 `refine_kernel_kind` 在 def-eq 双侧消息（`def_eq mismatch expected: … |
   actual: …`）里**只**认这一个形状：两侧末位排序层级都存在、期望 `Sort(n>0)`、
   实际 `Sort(0)`，**且两侧都是裸排序**（`classify_prop_sort_gap`）。其余 def-eq
   消息照旧 `kernel-rejected`。
3. `docs/protocol.md` 的错误码清单补一行（`protocol_doc_lists_every_error_code` 是
   既有契约测试，必须同步）。

**没有做的事（重要）**：没有为了让 `def T : Type := True` 过而放宽任何判定、没有伪造
类型、没有动内核（硬规则 4）。`def T : Type := True` **今天仍然判红**，只是红得有人话。

### 3.1 范围：只认**裸排序**形状（有意收窄）

`def bad : Prop -> Type := fun (x : Prop) => x` 是**同一个现象**（期望 `Pi (…), Sort(1)`、
实际 `Pi (…), Sort(0)`），但本轮**不发专用码**：

- 它是 CLI / LSP / VS Code 扩展契约测试里代表**通用内核拒绝**的夹具
  （`crates/cli/tests/cli.rs:103,716,863`、`crates/cli/tests/query.rs:670`、
  `crates/cli/tests/protocol.rs:238`、`crates/lsp/src/tests/lifecycle.rs:38,99`、
  `editor/vscode/src/test/extension.test.js:52`——它们钉 `code == "kernel-rejected"`、
  stage、span、expected/actual）；
- 本轮的目标是"**只加**错误码/hint，不改既有码语义"，加宽到 `Pi` 形状就必须同步
  迁移那 8 处夹具（不在本单允许改的文件面内）。

⇒ 台账 ① 的形状（`def T : Type := <Prop 值>`，裸排序）已经拿到专用码；`Pi` 形状
留给一次专门的夹具迁移（判别性单测 `prop_not_cumulative_code_does_not_catch_other_kernel_gaps`
把这条现状钉住）。

## 4. 为什么反方向不给专用码（判别性）

`Sort(0)` 期望 / `Sort(m>0)` 实际 这一个形状**至少有两种来源**：

- `Exists.witness`（想从 Prop 归纳里取数据）；
- 普通的类型写错：`def f : Prop := Nat`（该写命题却写了数据）、
  `def f : Nat -> Prop := fun (n : Nat) => Nat`。

两者的 def-eq 双侧渲染**逐字同形**（`Pi (…), Sort(0)` vs `Pi (…), Sort(1)`），
文本层分不开。给它一个 `kernel-prop-no-large-elim` 之类的码，就会把普通类型错误
误标成"消去子问题"，hint 反而误导。所以：**只给能精确命名的 ① 发码**，② 保持通用
`kernel-rejected`，边界写进本文与台账。

（单测 `prop_not_cumulative_code_does_not_catch_other_kernel_gaps` 与
`generic_def_eq_mismatch_keeps_kernel_rejected` 把这条判别性钉住；
`exists_eliminator_motive_stays_in_prop` 把 ② 的现状钉住。）

## 5. 课程怎么绕（L-06 的 workaround，现状）

卷 I 的绕法不是"临时将就"，而是**陈述形状的设计**，三条都在课程文件里写明：

| 绕法 | 课程里的位置 | 代价 |
|---|---|---|
| 等势写成**命题版数据**：`Set.Equiv α β A B : Prop`（内含一对互逆映射 + 四条证据），而不是 Lean/Mathlib 的 data 版 `Equiv` | `courses/set-theory/lib/Equiv.sokonanoda`（文件头 §数据形状；`Set.Equiv.mk` 吃 `f`/`g`/①②③④） | 结论是 Prop，**不能**从 `h : Set.Equiv …` 里把 `f` 取出来当函数用；要用就两次 `Exists.elim`（用完即弃） |
| 满射可裂写成**数据版**：逆函数与"它是右逆"的证据由调用者交进来 | `courses/set-theory/units/unit07-functions.sokonanoda:45-55`（"存在版"与选择公理等价 ⇒ 本语言证不出来） | 引理的陈述多两个参数；`Bijective f ↔ Exists (β -> α) …` 的"→"方向不可证 |
| 真要**取数据**时请选择公理：`Exists.choose` + `Exists.choose_spec` **成对**声明，只在单元⑩ 出现 | `courses/set-theory/units/unit10-cantor.sokonanoda:170-173`（练习 7 是全卷唯一允许用它的题） | 多一条公理假设；卷 I 其余证明一条都没用它 |

**课程一句话**：能证的都证了，形状改了而不是强度丢了——这是台账把它记成
`workaround` 而不是 `wontfix` 的原因：边界不可动，但课程有**系统性的、写在文件里的**
替代陈述。

## 6. 不做的事（明确边界）

1. **不做累积性**（`Prop ⊆ Type`）：内核判定性质，冻结快照下不改；要改就是一次
   内核语义变更（另立设计 + 三层回归），且会让 `def T : Type := True` 这类"命题当
   数据"在课程里静默通过——与教学意图相反。
2. **不做 Prop 归纳的大消去**：同上；且它是官方 Lean 的规则，本语言跟随。
3. **不给反方向（`Sort(0)` 期望）发专用码**：§4 的判别性理由。
4. **不装 `Classical.choice`/`Exists.choose` 进 prelude**：那是选择公理（L2 课程
   库的活，`docs/design/course-stdlib.md` §3），且卷 I 只在单元⑩ 正面碰一次。
5. **不改 hint 之外的既有码语义**：`kernel-rejected` 仍是通用回退；`kernel-expected-sort`
   等家族一字未动。

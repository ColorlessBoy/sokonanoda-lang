# apply

用一条函数的结论对上目标，它剩下的前提各自变成新目标。

## 怎么用

写法是 `apply <项>`。这个项的类型通常是一条函数（`→`）链：链的**结论**与当前
目标对齐，链上的**前提**挨个变成新的子目标，顺序与它们在类型里出现的顺序一致。

```sokonanoda
example (a b : Prop) (h : a ∧ b) : a ∧ b := by
  apply And.intro
  exact And.left h
  exact And.right h
```

`And.intro` 的类型是 `a → b → a ∧ b`：结论 `a ∧ b` 与目标一致，两个前提 `a`、`b`
各自变成子目标，所以后面跟两句 `exact`，按顺序一个子目标一句。

拿来对目标的项也可以是你的一条假设：

```sokonanoda
example (a b : Prop) (ha : a) (h : a → b) : b := by
  apply h
  exact ha
```

`h : a → b` 的结论是 `b`，前提 `a` 变成新子目标，正好由 `ha` 收掉。

**目标本身就是函数**（`→` / `∀`）时，`apply` 会先把引理**前面的参数**对到目标上，
剩下那几层正好是目标自己的参数 —— 于是交出去的是一个**函数**，不必先 `intro`：

```sokonanoda
def Mem (α : Type) (a : α) (A : α → Prop) : Prop := A a

def Sub (α : Type) (A B : α → Prop) : Prop :=
  ∀ (x : α), Mem α x A → Mem α x B

example (α : Type) (A B : α → Prop) (h : Sub α A B) (x : α) :
    Mem α x A → Mem α x B := by
  apply h
```

`h` 展开是 `∀ x, Mem α x A → Mem α x B`，目标是 `Mem α x A → Mem α x B`：第一层
`x` 由目标里的 `x` 定下来，这一行就是 `h x`，目标当场关掉（与 Lean 4 的 `apply` 同形）。

某个前导参数如果**在目标里没有约束**，它就变成子目标，和其它前提一起按顺序排在
目标栈上：

```sokonanoda
example (P Q : Prop) (h : ∀ (n : Nat), P → Q) : P → Q := by
  apply h
  exact 0
```

`n` 在目标 `P → Q` 里没有出现 ⇒ `apply h` 先留下 `⊢ Nat`，交给下一行的 `exact 0`。

## 什么时候用

**适用**：目标的形状与某条引理（或假设）的结论一致，你想从结论反着往回推，让它
的前提替你决定接下来要证什么。

**不适用**：你手上已经有整体证明时用 `exact`；目标是归纳类型、想按它的构造子拆
开时用 `constructor`，它替你挑构造子，不必自己写构造子的名字。

## 内核在背后判什么

`apply` 先把项的**类型**推出来，剥掉类型前面的 `→`/`∀` 层，剩下最后那一段，就是
它的结论。然后拿结论去对当前目标：按同名的头、逐个实参位对齐。

目标本身是函数时还有更早的一档：**不剥完**，只把引理前面的若干层用目标里的形状
合一出来，余下的层与目标的 Π 链逐层对齐（Lean 4 的 `MVarId.apply` 也从
`i = n - m` 这一档起试，`n`/`m` 分别是引理与目标的层数）。所以「目标是个函数」
不需要先 `intro` —— `apply` 自己会交一个函数出来。

对齐之前，两边都会先做**定义展开**：结论与目标各自沿自己的头展开若干轮（最多
4 轮，每轮顺手把展开带出来的 β 归约做掉 —— 也就是把「函数应用到实参」这一步代换
掉），所以差着几层定义的形状也能对上：

```sokonanoda
def MyTrue : Prop := True

def MyTrueAlias : Prop := MyTrue

example : MyTrueAlias := by
  apply True.intro
```

`True.intro` 的结论是 `True`；目标 `MyTrueAlias` 展开一轮是 `MyTrue`、再一轮就是
`True`，两边于是对齐。

展开之后还对不上的时候，内核还有最后一句：它会判「这条应用本身是不是已经证明了
目标」（同样是定义相等）。是就直接关掉目标，不是才报错。所以「相等」这件事永远
由内核说了算，`apply` 只负责把两边摆成能对上的形状。学习者遇到过的现场是：目标
`a ∈ ({a})` 与 `Eq.refl a` 的结论 `a = a` 之间隔着好几层展开，今天这几层走得完，
`apply Eq.refl a` 直接过。

## 常见错误与出路

| 你看到的 | 意思是 | 下一步 |
|---|---|---|
| `` `apply` 的目标不匹配：`And.intro` 的结果是 `a ∧ b`，无法对齐当前目标 `True` `` | 这条项的结论既不是目标、也不与目标定义相等（展开之后仍对不上） | 换一条结论对得上的引理；也可以先用 `have` 把中间结论写成一条假设，再 `exact` 它 |

## 相关

- [`exact`](exact.md) —— 项已经完整时的写法
- [`constructor`](constructor.md) —— 目标是归纳类型时，让它替你挑构造子
- [`use`](use.md) —— 目标是 `∃ x, p x` 时，先交证人再证剩下的

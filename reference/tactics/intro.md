# intro

引入假设：目标 `A → B`（或 `∀ x, …`）时，先假设 `A`，再证 `B`。

## 怎么用

写法是 `intro <名字>`。名字随你取，它只影响你后面怎么称呼这条假设：
`intro y` 之后，目标里的绑定名就是 `y`，不必照声明里的名字写。

```sokonanoda
example (a b : Prop) : a → b → a := by
  intro ha hb
  exact ha
```

一次可以给多个名字，**每个名字剥掉一层**：上面那句 `intro ha hb` 等于先
`intro ha`、再 `intro hb`。剥完两层，目标从 `a → b → a` 变成 `a`，正好是
`ha` 的类型，所以 `exact ha` 收尾。

## 什么时候用

**适用**：目标是函数类型（`→`）或全称量化（`∀`）。这是 `by` 块里最常见的开头。

**不适用**：目标已经不是函数形状了，比如 `a ∧ b`、`∃ x, p x`、`a = b`。
这时 `intro` 会报「当前目标不是函数」。拆 `∧` 用 `constructor`，拆 `∃` 用
`use`，证等式先试 `rfl`。

## 内核在背后判什么

`intro` 自己不判定任何东西，它做两件事。

第一件是把目标**剥一层**。剥之前会先做**定义展开**（最多 4 层），所以目标的头
是一个 `def`、展开之后才是函数类型时，`intro` 照样能过：

```sokonanoda
def MyImp (a b : Prop) : Prop := a → b

example (a : Prop) : MyImp a a := by
  intro ha
  exact ha
```

这里的目标写的是 `MyImp a a`，展开一层就是 `a → a`，`intro ha` 于是可用。
学习者因此**不需要**为了 `intro` 先手动展开定义，直接写就行。

第二件事是在证明项里包一层 lambda（λ 表达式），把假设的名字绑到后面那段证明
上（对应 Lean 的 `Expr.lam`）。剩下的目标交给后面的 tactic。

## 常见错误与出路

| 你看到的 | 意思是 | 下一步 |
|---|---|---|
| `` `intro` 需要一个函数目标（… -> … 或 forall …），当前目标不是函数 `` | 目标的形状不是 `→`/`∀` | 先看目标是什么形状：`∧` → `constructor`；`∃` → `use`；`=` → 先试 `rfl` |

## 相关

- [`exact`](exact.md) —— 假设都引完了，用它交出答案
- [`apply`](apply.md) —— 不从前提往下推，而是从结论往上找
- [`constructor`](constructor.md) —— 目标是 `∧` 时按构造子拆成两个子目标

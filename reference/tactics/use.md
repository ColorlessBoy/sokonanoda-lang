# use

对 `∃ x, p x` 交出证人 `w`，接着去证 `p w`。

## 怎么用

写法是 `use <证人>`。目标得是一个归纳类型：`use` 取它的**第一个**构造子，把证人
填进这个构造子的第一个字段，剩下的字段变成新的子目标。

```sokonanoda
-- 本页的例子自带一个单构造子归纳类型：`∃` 住在卷 I 的课程库里（`lib.Exists`），
-- 不在 prelude 里，所以例子把它需要的声明一起写出来。
inductive Pair (α : Type) (p : α → Prop) : Prop
ctor Pair.intro (w : α) (hw : p w) : Pair α p
end

example (α : Type) (p : α → Prop) (w : α) (hw : p w) : Pair α p := by
  use w
  exact hw
```

`Pair` 与课程库里的 `Exists` 同形：第一个字段是证人 `w : α`，第二个字段是 `p w`
的证明。`use w` 交掉第一个字段，目标剩下 `p w`，正好由 `hw` 收掉。

目标的头写的是 `def` 也没关系，展开之后露出归纳就能用：

```sokonanoda
inductive Pair (α : Type) (p : α → Prop) : Prop
ctor Pair.intro (w : α) (hw : p w) : Pair α p
end

def PairAlias (α : Type) (p : α → Prop) : Prop := Pair α p

example (α : Type) (p : α → Prop) (w : α) (hw : p w) : PairAlias α p := by
  use w
  exact hw
```

`use` 之后如果不去证剩下的目标，它就停在那里（和 `sorry` 一样是**合法的未完成
状态**，不是错误）。

## 什么时候用

**适用**：目标是 `∃ x, p x`（或同形的归纳），而你**已经知道**证人是谁。

**不适用**：目标有多个构造子、而你要的不是第一个 —— `use` 固定取第一个构造子，
想要 `A ∨ B` 的右边就用 `right`；证人还不知道时，先把「存在这样的 x」证成一条
`have`，再 `use` 那个项。

## 内核在背后判什么

`use w` 是**两步合一**：先按目标挑它的第一个构造子（等价于 `apply <构造子>`），
紧接着把第一个子目标（证人位）交给 `exact w`。所以交证人这一步的判定与 `exact`
**完全同源**：内核判 `w` 的类型与第一个字段的类型是否定义相等；其余字段的类型
变成新的子目标，按声明顺序排在你面前。

目标头不是归纳类型时，`use` 报的是与 `constructor` 同一条消息，只是关键字换成
`use`；证人类型不对时，报的是 `exact` 那条消息 —— 两条的原文都在下表里。

## 常见错误与出路

| 你看到的 | 意思是 | 下一步 |
|---|---|---|
| `` `use` 需要目标是**归纳类型**，但当前目标的头 `a` 不在归纳表里（它可能是公理、定义，或者是一个函数目标——先 `intro` 拆开试试） `` | 目标（展开之后）不是归纳类型 | 确认目标是 `∃`/同形归纳；是函数目标就先 `intro`，是 `∧` 就 `constructor` |
| `` `exact` 类型不匹配：期望 `α`，实际是 `β` `` | 证人的类型与归纳第一个字段的类型不一致 | 核对证人的类型；字段类型依赖前面的参数时，确认它们也对得上 |

## 相关

- [`exact`](exact.md) —— `use` 之后那一步用的就是它
- [`constructor`](constructor.md) —— 目标有多个构造子时用它挑
- [`have`](have.md) —— 证人还不知道时，先把存在性证成一条中间结论

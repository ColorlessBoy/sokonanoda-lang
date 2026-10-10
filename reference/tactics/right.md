# right

选目标归纳的第二个构造子（`A ∨ B` 上就是右边 `B`）。

## 怎么用

`right` 不带参数。目标是 `A ∨ B` 时它挑第二个构造子 `Or.inr`，目标从 `A ∨ B` 变成
`B`。

```sokonanoda
example (a b : Prop) (hb : b) : a ∨ b := by
  right
  exact hb
```

`right` 不限于 `∨`，任何归纳类型都能用：它取的始终是声明顺序里的第二个构造子。
构造子没有字段时，这一取就把目标证完了：

```sokonanoda
inductive Tri (a : Prop) : Prop
ctor Tri.first : Tri a
ctor Tri.second : Tri a
ctor Tri.third : Tri a
end

example (a : Prop) : Tri a := by
  right
```

`Tri` 有三个构造子，`right` 取第二个 `Tri.second`；它没有字段，所以目标当场证完。
在 `∨` 上取的则是 `Or.inr`，它还带一个字段 `B`，留给你接着证。

## 什么时候用

**适用**：目标是 `A ∨ B`，而你能证右边 `B`。`∨` 只有两个构造子，要左边用 `left`、
要右边用 `right`。

**不适用**：归纳只有一个构造子的时候。`right` 要的是"第二个"，没有第二个就直接
报错（`∧` 就是这种情形，见下表第一条）；这种目标用 `constructor`。

## 内核在背后判什么

`right` 和 `left`/`constructor`/`use` 共用一台机器，区别只在它要的**下标是 1**：
目标归纳的构造子按声明顺序排成一列，`right` 取第 1 号（从 0 数起，也就是第二个），
`Or` 上正是 `Or.inr`。取出构造子之后照旧走 `apply` 的判定。

取下标 1 带来一条只有 `right` 会遇到的失败：目标归纳的构造子不够两个时，它没有
第二条可取，于是报「至少要有 2 个构造子」。

## 常见错误与出路

| 你看到的 | 意思是 | 下一步 |
|---|---|---|
| `` `right` 需要目标至少有 2 个构造子，但 `And` 只有 1 个 `` | 目标归纳只有一个构造子，没有"第二个"可取 | 目标是 `∧` 这类单构造子归纳 ⇒ 用 `constructor` |
| `` `right` 需要目标是**归纳类型**，但当前目标的头 `a` 不在归纳表里（它可能是公理、定义，或者是一个函数目标——先 `intro` 拆开试试） `` | 目标（展开之后）不是归纳类型 | 是箭头就先 `intro`；目标是 `∃ x, p x` 用 `use` |
| `` `exact` 类型不匹配：期望 `b`，实际是 `a` `` | 你站在右边那个目标上，手上却只有左边的证明 | 你要给的是左边 ⇒ 把 `right` 换成 `left` |

## 相关

- [`left`](left.md) —— 取第一个构造子
- [`constructor`](constructor.md) —— 只有一个构造子的归纳用它
- [`use`](use.md) —— 同一台机器，目标是存在量化时交证人

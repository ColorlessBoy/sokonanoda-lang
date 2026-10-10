# match

对项做情形分析，臂体写的是项（等价于 `exact (match …)`）。

## 怎么用

`match` 有两个位置，写法一样、判定也一样。

tactic 位置：在 `by` 块里直接写 `match <项> with`。

```sokonanoda
example (a b : Prop) (h : a ∧ b) : a := by
  match h with
  | And.intro ha _ => ha
```

项位置：把同一个 `match` 当证明项交出去。

```sokonanoda
example (a b : Prop) (h : a ∧ b) : a := by
  exact match h with
  | And.intro ha _ => ha
```

两处**臂体都是项**（上面是 `ha`，别的例子里可能是 `Or.inr ha` 这样的项），不是
tactic 序列 —— 这是最容易写错的一点。臂里 `And.intro ha _` 是构造子的名字加字段
名，`_` 表示这个字段用不到。要写多步 tactic 的情形分析，用 `cases`，它的臂体才是
tactic。

## 什么时候用

**适用**：手上有归纳类型的值，要在**项**里分情况取不同的值 —— 或者目标要求你交出
一个项，而这个项得看构造子决定。多支的例子：

```sokonanoda
example (a b : Prop) (h : a ∨ b) : b ∨ a := by
  match h with
  | Or.inl ha => Or.inr ha
  | Or.inr hb => Or.inl hb
```

**不适用**：要拆的是**目标**（那是 `constructor`/`left`/`right` 的活）；每条分支要
写好几条 tactic 时，`cases` 更顺手。

## 内核在背后判什么

`match` **没有自己的 AST 变体**：tactic 位置上的 `match` 被解析成一条
`Tactic::Exact { expr: Expr::Match }`，也就是**一步 `exact`**，交出去的正是那个
`match` 项。所以报错里出现 `exact` 打头（内部回显写成 `exact (match …)`）不是写错
了 —— 它是在说这一步走了 `exact` 那条路。

判定照旧全由内核做：分支的构造子必须属于被匹配的那个归纳，分支要覆盖它的全部
构造子（否则报「`match` 的分支不完整」），每条臂体的类型要与目标定义相等。

## 常见错误与出路

| 你看到的 | 意思是 | 下一步 |
|---|---|---|
| `` `exact` 判定失败：unknown identifier `exact` `` | 你把臂体写成了 tactic（`=> exact ha`），内核在项的位置找这个名字 | 臂体是**项**：写成 `=> ha`；每条分支要多步 tactic 就用 `cases` |
| `` `exact` 判定失败：`match` 的分支不完整：有些取值没有对应分支 `` | 有构造子没写到 | 给每个构造子一个分支：`∧` 只有 `And.intro`，`∨` 有 `Or.inl`/`Or.inr` |
| `` `exact` 判定失败：`And` 没有构造子 `Or.inl`；可用的是：`And.intro` `` | 分支名不属于被匹配的那个归纳 | 用被匹配类型自己的构造子名 |

## 相关

- [`cases`](cases.md) —— 同一件事，臂体改成 tactic 序列
- [`exact`](exact.md) —— `match` 在 tactic 位置就是它
- [`constructor`](constructor.md) —— 目标是归纳类型时拆目标

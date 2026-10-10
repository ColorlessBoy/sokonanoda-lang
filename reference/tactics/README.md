# tactic 速查

本语言实现的全部 14 条 `by` tactic。一句话说不清的地方，点进每一篇看完整文档 ——
在编辑器里把光标放在 tactic 关键字上按 `F12`，也会直接落到对应那篇。

> 预算（本目录自己的闸，判据在 `crates/front/tests/tactic_docs.rs`）：
> 每篇 ≤120 行 · 本目录 ≤1200 行 · 本索引 ≤80 行。

| tactic | 一句话 | 文档 |
|---|---|---|
| `intro` | 引入假设：目标 `A → B`（或 `∀ x, …`）时，先假设 `A`，再证 `B`。 | [intro](intro.md) |
| `exact` | 交出证明项 `e`；内核判 `e` 的类型与当前目标是否定义相等（defeq）。 | [exact](exact.md) |
| `apply` | 用一条函数的结论对上目标，它剩下的前提各自变成新目标。 | [apply](apply.md) |
| `assumption` | 在已有假设里找一条与目标定义相等的，直接结束当前目标。 | [assumption](assumption.md) |
| `rfl` | 自反：目标是 `=` 或 `↔`，且两边定义相等时成立。 | [rfl](rfl.md) |
| `match` | 对项做情形分析，臂体写的是项（等价于 `exact (match …)`）。 | [match](match.md) |
| `constructor` | 按目标取第一个构造子，它的参数变成新目标。 | [constructor](constructor.md) |
| `left` | 选目标归纳的第一个构造子（`A ∨ B` 上就是左边 `A`）。 | [left](left.md) |
| `right` | 选目标归纳的第二个构造子（`A ∨ B` 上就是右边 `B`）。 | [right](right.md) |
| `use` | 对 `∃ x, p x` 交出证人 `w`，接着去证 `p w`。 | [use](use.md) |
| `exfalso` | 把当前目标换成 `False`；原来要证的东西留到后面用。 | [exfalso](exfalso.md) |
| `cases` | 对假设做情形分析，每个构造子一个分支。 | [cases](cases.md) |
| `have` | 在证明中间先证一条 `h : T`，当前目标不变。 | [have](have.md) |
| `sorry` | 占位：目标保持开放。练习没做完的合法状态，不是错误。 | [sorry](sorry.md) |

# `inductive` / `ctor` / `rec` / `iota` 是什么？

## 一句话回答

它们不是“特殊用途的 axiom”，而是**一个带计算规则的声明族**。

表面上看，`inductive Nat`、`ctor zero`、`rec Nat.rec` 都像在往环境里加入一些
“被信任的常量”，axiom 也可以做到这一点。差别在于：

- axiom 只承诺“这个常量存在”，不提供任何计算；
- inductive 还承诺“构造子互斥、内射，且 recursor 在构造子上按 iota 规则计算”。

在 Lean 内核里，一个归纳类型由四种信息共同构成：

| 名字 | 承担的角色 | 如果只有它 |
|---|---|---|
| `inductive Nat : Type` | 类型本身存在 | 可以当名字用，但不知道如何造元素 |
| `ctor zero : Nat` | 给出一条生成元素的规则 | 有了元素，但没有消除/递归方式 |
| `rec Nat.rec : ...` | 给定消除器（递归器）类型 | 有了“遍历规则”的形状 |
| `iota zero/succ := ...` | 递归器在构造子上的计算规则 | recursor 只是死常量，不算 |

所以 **iota 是它们和 axiom 的分界线**：axiom 从不说明自己算起来是什么，
而 `iota` 把 recursor 的行为完全定义出来。

## kernel 眼中的这四行

sokonanoda 内部把它们变成四种声明：

```text
Declar::Inductive    <-  inductive Nat : Type
Declar::Constructor  <-  ctor zero / ctor succ ...
Declar::Recursor     <-  rec Nat.rec ...
RecursorData.rec_rules <- iota zero / iota succ ...
```

当 evaluator 看到：

```text
Nat.rec motive mz ms zero
```

时，它不是“相信某个公理”，而是查 `rec_rules` 里 `zero` 的规则项，把结果计算成
`mz`。类似地：

```text
Nat.rec motive mz ms (succ n)
```

按 succ 规则计算成：

```text
ms n (Nat.rec motive mz ms n)
```

这就是 iota 归约，也是“自然数加法能算出来”的原因。

## 为什么不直接叫 axiom？

axiom 也可以给 `Nat.rec` 一个类型，但 axiom 没有任何 iota 规则，所以：

```text
Nat.rec ... zero  -- 永远算不动，会卡成中性项
```

这也能证明一些东西，但证明的是“一个没有计算规则的函数”，不是真正的自然数
归纳。教学里必须让学生看到 iota 规则，否则他们学的是“像 Lean 但不会算”。

## 正确性由谁保证？

在完整内核检查路径里，`inductive.rs` 会做这些事：

- 检查归纳类型出现方式是否正向（positivity），防止不一致；
- 检查构造子的参数/索引；
- 重建每个 recursor 规则，并与导出里的 imported rule 对比；
- 检查大消去、K、证明无关性等边界。

所以 `iota` 不是“写什么信什么”。内核只把显式规则作为输入，再独立验证它们是否
与构造子形状一致。

## kernel 里的优化

### 1. Recursor / iota 的高速缓存

`eval.rs` 里 iota 有正缓存和“卡住”缓存：

- `iota_cache`：某个已经归约过的 recursor 结果不再重复计算；
- `iota_stuck`：证明当前不是可归约的 recursor（major premise 不是构造子），
  避免反复尝试。

### 2. Recursor rule 的缓存

```text
cache_key = (rec_rule.val, universe levels)
```

同一规则表达式、同一组宇宙层级只求值一次。

### 3. `whnf_head` + `deep_reduce`

- 常规检查只需要弱头范式：把顶层的 recursor 归约掉即可；
- 教学 `#reduce` 需要完整归约，所以 sokonanoda 增加 `deep_reduce`，
  会递归进入构造子参数，让 `s (Nat.rec ...)` 也能继续算。

### 4. Nat 原生运算（与 iota 正交）

当 `nat_extension` 打开时，`Nat.add`、`Nat.mul` 等会直接在大整数上做原生运算，
而不是展开成一层层 `Nat.rec`。例如：

```text
1 + 1  =>  2
```

这里不是 iota 展开，而是 kernel 对特定名字的快速路径。

### 5. 结构/值与 spine 的 hash-consing

- 构造子、recursor、inductive 在值层都区分 head 类型；
- spine（参数链表）hash-cons；
- 指针相等性用于快速跳过重复比较。

这些优化都保持 iota 规则语义不变，只是让它更快。

## 已知分歧（elab ↔ kernel，写 inductive 用例前必读）

- front 的 `install_inductive_block` 恒传 `is_recursive: true`
  （`elab.rs`，占位简化），而内核按构造子 binder 重推递归性并在
  `inductive.rs:68` 用**无消息 `assert_eq!`** 校验——"单构造子且明显非
  递归"的块会先撞上这个 assert（报 `assertion failed: left == right`，
  被分类为 `kernel-internal`）。写教学用例时给该块加一个递归构造子
  （或参考 `examples/py-nat.sokonanoda`），不要拿它当学习者的错误诊断。
  正规修法（elab 侧按 binder 重推 is_recursive）是待办。

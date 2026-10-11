# unfold

把目标里点名的常量展开成定义体（`unfold Set.mem Set.singleton`）。

## 怎么用

`unfold` 后面写**一个或多个常量名**，它把这些名字的定义体在目标里展开出来：

```sokonanoda
def MyP (a : Prop) : Prop := a

example (a : Prop) (h : a) : MyP a := by
  unfold MyP
  exact h
```

展开之后目标从 `MyP a` 变成 `a` —— 命题本身没变，只是把 `MyP` 这层名字换成了
它的定义。

集合论里最常见的用法是把它用在集合字面量上：

```sokonanoda
def Set (α : Type) : Type := α → Prop
def Set.mem {α : Type} (a : α) (A : Set α) : Prop := A a
def Set.singleton {α : Type} (a : α) : Set α := fun (x : α) => x = a
infix:50 " ∈ " => Set.mem

example (α : Type) (a : α) : a ∈ {a} := by
  unfold Set.mem Set.singleton
  rfl
```

`a ∈ {a}` 里藏着两层：`∈` 是 `Set.mem`、`{a}` 是 `Set.singleton`。两个名字都给上，
目标就变成 `a = a`，`rfl` 一步收尾。

**展开是逐处做的**：目标里每一处该常量都换掉，不只是最外层那一处；名字给多个时
按你写的顺序逐个处理。只写内层的名字也可以，只要它在目标里出现：

```sokonanoda
def Set (α : Type) : Type := α → Prop
def Set.mem {α : Type} (a : α) (A : Set α) : Prop := A a
def Set.singleton {α : Type} (a : α) : Set α := fun (x : α) => x = a
infix:50 " ∈ " => Set.mem

example (α : Type) (a : α) : a ∈ {a} := by
  unfold Set.singleton
  exact Eq.refl a
```

这里只展开了 `{a}`（目标变成 `a ∈ (fun (x : α) => x = a)`），`Set.mem` 没动 ——
`exact` 仍然过，因为目标与原来**定义相等**。

## 什么时候用

**适用**：目标里有个名字挡住了你 —— 你知道它的定义长什么样，想让目标显示成那个
样子，好让 `rfl` / `exact` / `apply` 能对上。

**不适用**：目标里根本没有那个名字（`unfold` 会报错，不会假装成功）；或者你只是想
「化简一下」而说不清要展开谁 —— 那就先看目标，把名字点出来。

## 内核在背后判什么

`unfold` 只做**代换**：把定义体代进去。代换之后目标与原来是不是同一个命题，由内核
判**定义相等**（defeq）—— 相等才提交，不等就报错。所以 `unfold` 永远不会把目标换成
一个不等价的东西，判据也不是文本比对。

## 常见错误与出路

- `` `unfold` 没有展开 `X` ``：目标里没有 `X`，或名字拼错了。名字要用**点名写法**
  （`Set.mem`），并且必须是目标里真的出现的那个常量。
- 展开后目标变长、看不清：正常现象。接着用 `intro` / `apply` / `rfl` 往下走。
- 想展开**假设**里的常量（Lean 的 `unfold Foo at h`）：本语言不支持 `at` 位置
  （子集边界），只能展开**当前目标**里的常量。

## 相关

- `rfl`：展开之后常常一步收尾。
- `exact` / `apply`：目标换成定义体之后，用它们把项对齐上去。

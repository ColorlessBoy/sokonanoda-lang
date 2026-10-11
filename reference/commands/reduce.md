# #reduce

把表达式交给内核算到底，印出归约后的值。

## 怎么用

`#reduce <表达式>` 写一行，答案的形状是 `表达式 => 值`：

```sokonanoda
#reduce 1 + 1
```

```text
1 + 1 => 2
```

`+` 是递归定义，内核把 `1 + 1` **算**成 `2`。自己写的定义同样会展开：

```sokonanoda
def myid {a : Type} (x : a) : a := x

#reduce myid 1
```

```text
myid 1 => 1
```

值也可以是**函数** —— 算到范式就是 lambda 本身，不再往下算：

```sokonanoda
def myid {a : Type} (x : a) : a := x

#reduce (fun (n : Nat) => myid n)
```

```text
fun (n : Nat) => myid n => fun (n : Nat) => myid n
```

⚠ 答案里 `=>` **左边**那一半是**你写的源文本**（原样印），**右边**那一半才是内核算
出来的值 —— 看的是右边。

## 什么时候用

- 想确认两条表达式是否**算到同一个值**（`1 + 1` 与 `2`）。
- 递归定义、记法展开之后到底成了什么，直接看值。
- `rfl` 失败时：把两边各自 `#reduce` 一下，就知道内核为什么判不相等。

## 内核在背后做什么

elaborate 之后做**归约**：按定义展开、按递归定义计算，把结果印出来。它是**求值**，
不是判定 —— `#reduce` 从不报「证不出来」，它只报算出来的值；「这条等式成立」那种判定
归 [`rfl`](../tactics/rfl.md) / [`exact`](../tactics/exact.md)（由内核终审）。

## 常见错误与出路

| 你看到的 | 意思是 | 下一步 |
|---|---|---|
| ``error[elab-unknown-identifier]: unknown identifier `nope` `` | 表达式里的名字不在环境里 | 检查拼写 / 先把它声明出来；`sorry` 体的开放练习也还不在环境里 |

## 边界

- 印出来的值受 [`set_option`](set_option.md) 影响（值里的隐式实参会带上 `@`），但
  `=>` 左边那份**源文本**永远照你写的印。
- 表达式里的名字都必须在环境里（本语言顶层没有自由变量）—— 不在就报
  `elab-unknown-identifier`。

## 相关

- [`#check`](check.md) —— 只要类型，不算值
- [`rfl`](../tactics/rfl.md) —— 把「两边定义相等」变成一条证明
- [`set_option`](set_option.md) —— 让值里的隐式实参显形

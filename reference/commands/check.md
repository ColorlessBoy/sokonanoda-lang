# #check

把表达式交给内核，印出它推出来的类型。

## 怎么用

`#check <表达式>` 写一行，后面跟任何合法的表达式。答案的形状是 `表达式 : 类型`：

```sokonanoda
#check 1 + 1
```

```text
1 + 1: Nat
```

表达式可以是应用、lambda、带绑元的名字 —— 什么都能 `#check`：

```sokonanoda
def myid {a : Type} (x : a) : a := x

#check myid 1
#check (fun (x : Nat) => x)
```

```text
myid 1: Nat
fun (x : Nat) => x: Nat -> Nat
```

## 什么时候用

- 忘了某个东西的类型，尤其是**隐式实参被补成了什么** —— `#check` 印的是内核补完之后
  推出来的类型，不是你写的那个表达式「看上去」的类型。
- 想确认一条声明或定理的类型（不展开定义体 —— 那是 [`#print`](print.md) 的事）。
- `apply` / `exact` 失败时：先把那一项 `#check` 一下，看内核认的类型与目标差在哪。

## 内核在背后做什么

`#check` 走的是与声明体同一条 elaborate 路：推断类型、补隐式实参、定宇宙层，然后把
推出来的类型交给内核 pp 印出来。它**不**判定真假、也**不**归约到底（那是
[`#reduce`](reduce.md)）。

**两条路**（本语言有意分成两条，见 §边界）：

- `#check <表达式>`（应用、lambda、运算……）⇒ 内核 pp 出来的**类型文本**；
- `#check <裸常量>`（只写一个名字）⇒ 直接渲**源文件里的签名**（G-63：裸写法默认
  宇宙层 0，直接渲签名才不撒谎）。

## 常见错误与出路

| 你看到的 | 意思是 | 下一步 |
|---|---|---|
| ``error[elab-unknown-identifier]: unknown identifier `nope` `` | 这个名字不在环境里 | 检查拼写；还没声明；或者它是 **`sorry` 体的开放练习** —— 练习进了环境（没有洞）才 `#check` 得到 |

## 边界

- **`#check <裸常量>` 有意不受 `set_option pp.*` 影响**（它印的是签名源文本，不是内核
  pp 的类型文本）；`#check <表达式>` 受档影响，见 [`set_option`](set_option.md)。
- `#check` 不产生诊断就代表**类型推出来了**，不代表这条表达式「对」—— 判定归 `exact`
  那些 tactic（由内核终审）。

## 相关

- [`#print`](print.md) —— 要看定义体/证明项，而不只是类型
- [`#reduce`](reduce.md) —— 要算到底，而不只是类型
- [`set_option`](set_option.md) —— 让类型里的隐式实参显形

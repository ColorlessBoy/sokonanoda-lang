# #print

印出环境里某条声明的类型与定义体（证明项也在内）。

## 怎么用

`#print <名字>` 写一行，名字必须是**环境里已有的声明**：

```sokonanoda
def myid {a : Type} (x : a) : a := x

#print myid
```

```text
def myid : forall {a : Type 0}, a -> a := fun {a : Type 0} (x : a) => x
```

定理印出来的是它的**证明项** —— `by` 块最后建出来的那个 lambda：

```sokonanoda
theorem t (a : Prop) : a -> a := by intro h; exact h

#print t
```

```text
theorem t (a : Prop) : a -> a := fun (h : a) => h
```

`inductive` 印的是它的头部（构造子与递归子不展开）：

```sokonanoda
#print Nat
```

```text
inductive Nat : Type 0
```

## 什么时候用

- 「这条声明到底怎么定义的」—— 尤其 `by` 块里的 tactic 最后建出了什么项。
- 怀疑内核是不是放水时：`#print` 印的是**环境里那一份**（签名 + 体），不是源码文本。
- 看一条库声明的原文（`prelude` 或 `import` 进来的模块里的名字同样能 `#print`）。

## 内核在背后做什么

从内核环境里取那条声明，连同类型与值一起 pp 出来。默认档**省掉隐式实参**：
`@Iff.refl (@Set.mem α a A)` 会印成 `Iff.refl` —— 那**不代表实参不存在**，只是印的时候
省了（实参在项里，内核检查时一个都不少）。要看就把档打开：

```sokonanoda
def myid {a : Type} (x : a) : a := x
def uses : Nat -> Nat := fun (n : Nat) => myid n

set_option pp.explicit true
#print uses
```

```text
def uses : @Nat -> @Nat := fun (n : @Nat) => @myid @Nat n
```

## 常见错误与出路

| 你看到的 | 意思是 | 下一步 |
|---|---|---|
| ``error[elab-unknown-identifier]: unknown identifier `t` `` | 这个名字不在环境里 | 检查拼写；**`sorry` 体的开放练习还没进环境** ⇒ `#print` 答不了它，练习做完（没有洞）才进得去 |

## 边界

- 只有**环境里**的声明能 `#print`：源码里写了但没编译过去的（解析失败、签名报错、
  开放练习）都不行 —— 它读的是环境，不是文件。
- 印出来的文本受 [`set_option`](set_option.md) 影响；默认档与 Lean 4 的默认一致
  （隐式实参不印）。

## 相关

- [`#check`](check.md) —— 只要类型，不展开定义体
- [`set_option`](set_option.md) —— 让隐式实参显形（`pp.explicit` / `pp.all`）
- 设计记录：`docs/design/set-option-pp.md`（起因就是一条 `#print` 印出 `:= Iff.refl` 的追问）

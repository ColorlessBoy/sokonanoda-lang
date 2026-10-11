# set_option

调 `#print` / `#check` / `#reduce` 的显示档 —— 让隐式实参看得见。

## 怎么用

`set_option <名字> true|false` **单独写一行**，对它**之后**的命令生效（文件作用域）：

```sokonanoda
def myid {a : Type} (x : a) : a := x
def uses : Nat -> Nat := fun (n : Nat) => myid n

#print uses
set_option pp.explicit true
#print uses
```

第一条 `#print` 是默认档（隐式实参被印掉），第二条是开档后的：

```text
def uses : Nat -> Nat := fun (n : Nat) => myid n
def uses : @Nat -> @Nat := fun (n : @Nat) => @myid @Nat n
```

`@` 就是「这条常量带上了隐式实参」的记号。只支持两条名字：

| 名字 | 效果 |
|---|---|
| `pp.explicit` | 印出隐式实参（`@myid @Nat n`） |
| `pp.all` | 上一条的**超集**：再关掉记法、印出宇宙层（`Prop` 印成 `Sort 0`） |

```sokonanoda
def uses : Nat -> Nat := fun (n : Nat) => n + n

set_option pp.all true
#print uses
```

```text
def uses : @Nat -> @Nat := fun (n : @Nat) => @Nat.add n n
```

关回去写 `set_option pp.explicit false`。⚠ `pp.all` 是超集：只把 `explicit` 关掉
**不会**关掉 `all`，要单独 `set_option pp.all false`。

## 什么时候用

- `#print` 印出来「少了实参」时：`… := Iff.refl` 看着像内核没检查实参 —— 开档就看见
  `@Iff.refl (@Set.mem α a A)`（实参一直都在，只是没印）。
- 想知道内核给某个表达式**补了哪些隐式实参**、宇宙层定成了什么。
- 读库声明时：`pp.all` 连记法一起关，露出来的全是点名形式，最好搜。

## 内核在背后做什么

档被**叠加**到内核 pp 的选项上（`explicit` / `all`），这条命令执行完立刻还原 ⇒ 只有
命令输出受影响，**不带 `set_option` 的文件逐字节不变**。

默认档 = 关，与 Lean 4 同名选项的默认值一致。隐式实参本来就在项里（内核 elaborate 时
就补好了），默认档只是**印的时候省掉** —— 所以 `#print` 印出 `:= Iff.refl` 不等于
「内核没检查实参」，两者是两件事。

档随**模块**携带：库模块里设的档，对入口模块里之后的命令同样生效（`import` 闭包按
同一份前缀文本编译，档是前缀的函数）。

## 常见错误与出路

| 你看到的 | 意思是 | 下一步 |
|---|---|---|
| `` `set_option` 只支持 `pp.explicit` 与 `pp.all` 两条名字（当前写的是 `pp.notation`）；其余选项今天没有可观察的显示差异，所以不静默接受 `` | 选项名不在白名单 | 换成这两条之一。其余 `pp.*`（`pp.universes` / `pp.notation` / `pp.proofs`）今天**不可观察**，所以报错而不是静默忽略 |
| `` `set_option pp.explicit` 的值只认 `true` / `false` `` | 值写错了 | 写 `true` 或 `false` |
| `` `set_option … in …` 这一轮不做：把这一行单独写出来即可（它对本行之后的命令都生效） `` | 写成了「只对一条命令生效」的形态 | 把 `set_option` 单独写一行 |

## 边界（有意偏离 Lean 的地方）

- **只影响命令输出**：`#print` / `#check <表达式>` / `#reduce`。声明卡片、目标面板、
  hover 的文本**不受档影响**（它们有各自的可回读策略）；`#check <裸常量>` 也不受
  影响（它印的是签名源文本，见 [`#check`](check.md)）。
- **不是 Lean 的全量 `pp.*`**：只有两条名字；其余选项今天不可观察 ⇒ 给专用诊断
  （`set-option-shape`），不静默。
- `set_option … in <命令>` 不做（同 `open scoped … in …` 的先例）。

## 相关

- [`#print`](print.md) —— 主要用武之地（看定义体/证明项里的隐式实参）
- [`#check`](check.md) —— 类型那一半受档影响（裸常量不受）
- [`#reduce`](reduce.md) —— 值那一半受档影响
- 设计记录：`docs/design/set-option-pp.md`（Lean 4 源码级对照 · 判据 8 条 · 边界）

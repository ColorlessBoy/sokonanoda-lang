# 命令速查

本语言的 4 条命令：3 条 `#` 命令（`#check` / `#reduce` / `#print`）与 1 条显示档
`set_option`。它们**不是声明** —— 不引入名字、不进环境，只把答案印出来：在 CLI 里
印到终端（`--json` 里是 `expr.typed` / `decl.printed` 事件），在编辑器里印到
Infoview 的「命令输出」块（光标停在那一行时；hover 那一行也给同一份文本）。

> 预算（本目录自己的闸，判据在 `crates/front/tests/command_docs.rs`）：
> 每篇 ≤120 行 · 本目录 ≤600 行 · 本索引 ≤80 行。

| 命令 | 一句话 | 文档 |
|---|---|---|
| `#check` | 把表达式交给内核，印出它推出来的类型。 | [check](check.md) |
| `#reduce` | 把表达式交给内核算到底，印出归约后的值。 | [reduce](reduce.md) |
| `#print` | 印出环境里某条声明的类型与定义体（证明项也在内）。 | [print](print.md) |
| `set_option` | 调 `#print` / `#check` / `#reduce` 的显示档 —— 让隐式实参看得见。 | [set_option](set_option.md) |

## 它们与声明的关系

| | 声明（`def` / `theorem` / …） | 命令（`#check` / `#reduce` / `#print` / `set_option`） |
|---|---|---|
| 进环境吗 | 进（后面的代码能用这个名字） | **不进** |
| 产事件吗 | `decl.checked` / `exercise.open` … | `expr.typed` / `decl.printed`（`set_option` 不产事件） |
| 判卷算它吗 | 算 | 不算（命令的报错只影响它自己那一行） |

`set_option` 是**唯一**不是 `#` 开头的命令：它只调显示档，不印东西（见
[set_option](set_option.md)）。

> `sokonanoda repl` 里另有 `#env` / `#prove` / `#help` / `#exit` —— 那是 **REPL 自己的
> 交互命令**，不是语言命令（`.sokonanoda` 文件里写不了它们；`sokonanoda --help` 有全表）。

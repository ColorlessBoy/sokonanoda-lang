# 试做稿：卷 I 前两个单元「真做一遍」（2026-09-18）

> **状态（同日更新）：试做稿已经"转正"——内容搬进了课程目录 `courses/set-theory/`**
> （lib 改用 Loogle 取证名，6 条内容型引理按 L-05 移出 lib 进单元②）。
> 本文保留为**试做过程的记录**：它解释了每条 L-xx 是怎么被撞出来的。
> 现在的判卷入口是 `python3 courses/set-theory/tools/check.py`。

> 触发（用户）：「你的教程出的题目，在做的时候，你会发现要补充很多其他的定理，边边角角的
> 定理……这个我感觉有点暴力，所以我需要你去用这个项目重新做一遍，然后才能把这些暴力给
> 消除掉。你要记录下这些其实是需要实现的，其实是没有的。」
>
> 本文 = 这次试做的现场记录：**真写、真判卷**（内核说了算），凡是"别的教程认为天然已知/
> 标准库里已经实现"的东西，全部登记成台账条目（`docs/gaps/ledger.jsonl` 的 `kind: library`），
> 并把"暴力"定量化。分层方案见 **`docs/design/course-stdlib.md`**。

## 1. 写了什么（全部用真内核判过）

| 文件 | 行 | 判卷结果 |
|---|---|---|
| `lib/Logic.sokonanoda` | 95 | **26 checked / 0 failed / 0 warning** |
| `lib/Set.sokonanoda` | 264 | **40 checked / 0 failed / 0 warning** |
| `unit01`（现 `courses/set-theory/units/unit01-…`） | 96 | 2 demos 通过 + **6 道练习（open）** |
| `solutions/unit01-solution` | 40 | **6 checked / 0 failed**（题目可做） |
| `unit02`（现 `courses/set-theory/units/unit02-…`） | 145 | 1 demo 通过 + **10 道练习（open）** |
| `solutions/unit02-solution` | 108 | **10 checked / 0 failed**（题目可做） |

**当时的文件现在住在课程里**（试做稿只保留本文作为过程记录）：

| 试做时的路径 | 现在的家 |
|---|---|
| `docs/gaps/spike/lib/*.sokonanoda` | `courses/set-theory/lib/` |
| `docs/gaps/spike/units/*.sokonanoda` | `courses/set-theory/units/`（+ `units/solutions/`） |

复跑（**一条命令**，内部用绝对路径 + `grade` 退出码，见台账 G-12/G-10）：

```bash
python3 courses/set-theory/tools/check.py
```

## 2. 暴力在哪：**66 条库里，只有 4 类是真正"非写不可"的**

`lib/` 里 66 条声明（26 + 40），逐条按"官方 Lean 4 / Mathlib 有没有"分诊，结果：

| 类别 | 条数 | 判据 | 处置 |
|---|---|---|---|
| **A. Lean core 级**（`True`/`False`/`And.elim`/`Or.elim`/`Not`/`absurd`/`Iff`/`Eq.symm`/`Eq.trans`/`congrArg`） | **16** | core 里现成，任何 Lean 教程直接用 | 进 **prelude**（台账 L-01/L-02） |
| **B. 定义展开级**（`Set.subset_def`/`mem_union`/`mem_inter`/`mem_diff`/`mem_power_iff`/`not_mem_empty`/`mem_singleton_iff`…） | **8** | Mathlib 里是 `rfl` 或一行；没有它们每道题都要手写展开 | 进 **课程标准库**（台账 L-04） |
| **C. 语言缺失导致变形**（`congrArg` 只能同宇宙；`Set.ext` 只能作公理；`Eq.mp` 写不出来） | **3** | 不是"没写"，是"写不出来" | 立项修语言（台账 G-14 / L-03） |
| **D. 其实该当练习的内容引理**（三律、最小元、`union_subset_iff`、`inter_comm`、`inter_union_distrib_left`…） | **16** | Mathlib 有，但**有数学内容**——课程就是让学生证一遍 | **移出 lib，改成练习**（台账 L-05） |
| 其余（定义本身：`Set`/`mem`/`subset`/`empty`/`union`…） | 23 | 词汇表 | 留在 lib |

**"暴力"的定量**：卷 I 前两个单元只写了 17 道题，就已经需要 **66 条**库支撑；
其中 **24 条（A+B）本该由语言/标准库提供**，学习者一行都不该写。

## 3. 这一遍抓到的**新**缺口（不在上一轮 13 条里）

| ID | 一句话 | 证据 |
|---|---|---|
| **G-14** | 一个声明只允许**一个**宇宙层级 binder（`{u v}`、`{u} {v}` 都解析失败）⇒ 跨宇宙的 `congrArg`/复合/像写不出来 | 写 `lib/Logic.sokonanoda` 的 `congrArg` 时第一时间撞到 |
| **G-15** | **内核拒绝的诊断 span 与出错声明范围不一致**：带 import 时末端跨进下一个声明；无 import 时起点提前两行；更大文件里起点会漂到下一个 `theorem` | 我用二分找错时**连修错两次**才察觉 |
| **L-01/L-02** | prelude 缺 Lean core 的 16 条逻辑/等式骨架 | §2 表 A |
| **L-03** | prelude 的 `Eq.subst` motive 只能是 `α → Prop` ⇒ **Type 层重写不可表达**（`Eq.mp`/`Eq.rec`/`cast`） | 有最小反例（台账里） |
| **L-04** | 课程标准库缺 8 条集合定义展开引理 | §2 表 B |
| **L-05** | 方法学：**16 条内容引理被误放进库**（判据见 §4） | §2 表 D |

## 4. 分界线（这条是试做最重要的产出）

> **Mathlib 有 ≠ 我们不该练；Mathlib 有且没有数学内容（纯定义展开）才归库。**

- **L1 prelude**：逻辑与等式的骨架（A 类）。
- **L2 课程标准库**：集合的**词汇 + 定义展开**（B 类）。
- **L3 单元练习**：一切**有数学内容**的陈述（D 类），哪怕 Mathlib 早就有了——
  因为"把标准事实自己证一遍"正是这门课要做的事。

按这条线，unit02 的 `pair_subset_iff`、`pair_comm`、`eq_empty_iff_forall_not_mem`
留在练习里是对的；而 `Set.pair` 的展开、`mem_pair` 的 `rfl` 级等价应该在库里。

## 4.1 证据驱动的题目（第三轮实证调研的落地）

单元 2 的练习 7 是**按实证选的**：Hendriyanto et al. 2024（183 人）测得 `r ∈ M` / `s ∉ M`
几乎全对，但 **`{r} ∈ M` 无一人给出正确理由** —— 于是它不是"顺便带一句"，而是一道单独的
D/R 题（"学生最常写的那条连类型都不对，为什么？改正它"）。其它证据→教学动作的映射见
`docs/design/set-theory-syllabus.md` §1.3。

## 5. 还没做（下一遍的输入）

1. 把 16 条 D 类从 `lib/Set` 移进 `units/`（并同步 unit02 的练习列表）；
2. 按 L1 清单提一个 prelude 提案（**要过"课程 + 测试 + 白名单"三件套**）；
3. 单元 3–12 各写一遍试做（预计还会撞出像 G-14 这样的语言缺口，尤其是 `Prod`/`Exists`/
   `Function`/基数那几段）；
4. 把 `docs/design/course-stdlib.md` 的 L2 清单固化成课程仓的 `lib/` 规范。

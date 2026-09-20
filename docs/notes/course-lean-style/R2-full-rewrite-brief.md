# 卷 I（`courses/set-theory/`）Lean 化收尾手册

> 给执行的人工 / subagent 的**操作手册**。设计依据
> `docs/design/course-lean-style.md`（§C1 卷 I 工作项、§9 as-built）；
> 记法边界见 `docs/design/notation-subset.md`；分层判据见
> `docs/design/course-stdlib.md`。
>
> **判据只有一条**：`python3 courses/set-theory/tools/check.py` 保持
> **36 目标 · 328 checked · 99 open · 0 判负**、exit 0。**`crates/kernel/` 零改动**。

## 0. 现在到哪了（2026-09-21 实测）

第 113–114 轮已经把**机械可替换的部分**做完（脚本 + 门禁逐轮验收）：

| 区域 | 状态 |
|---|---|
| `lib/Set.sokonanoda` | ✅ 记法 + `by` 证明全做完（23 checked，`def` 体用 `→ ∀ ∧ ∨ ¬ =`，定义展开引理走 `constructor` + `intro h; exact h`） |
| `lib/` 其余 6 文件 | 部分：`->`/`forall`/`And`/`Or`/`Not` 的**机械替换**已做；`Exists X (fun …)` 与跨行 `And` 未做 |
| `units/` 13 个画布 | 部分：同上（机械替换已做，嵌套 `∃` 未做） |
| `units/solutions/` 12 个解答 | 部分：同上（**376 处**机械替换已做，嵌套 `∃` 与少量项模式证明未做） |
| `units/notation-cheatsheet*.sokonanoda` | ⛔ **不要动**：它**故意**把点名与记法并列（大纲 §4 的"第二遍"教学装置），点名形式是**内容**不是残留 |

**脚本口径**（复现机械替换的判定）：剥掉 `--` 注释后数
`->` / `forall` / `And X Y` / `Or X Y` / `Not X` / `Iff X Y`，
排除 `And.intro` 这类点号名。当前剩余 ≈ **216 处**，构成：
`Exists X (fun …)` 83、跨行 `And` 87、其余 46。

## 1. 剩下的三类活

### 1.1 `Exists X (fun (x : T) => p x)` → `∃ (x : T), p x`

`∃` 由 `lib/Exists.sokonanoda` 的 `binder_notation "∃" => Exists` 提供
（**记法声明不产生事件**，改完计数不变）。要处理的典型形状（`lib/Equiv.sokonanoda:114`）：

```
  Exists (α → β) (fun (f : α → β) =>
    And (Set.MapsTo α β f A B)
      (Exists (β → α) (fun (g : β → α) =>
        And (Set.MapsTo β α g B A)
          (∀ (x : α), ...))))
```

写成：

```
  ∃ (f : α → β), Set.MapsTo α β f A B ∧
    ∃ (g : β → α), Set.MapsTo β α g B A ∧
      ∀ (x : α), ...
```

⚠️ **边界（实测）**：`∃ (x : T), p x` 的体只能取到**一个** lambda 体；
多行体要继续用括号或换行（README/设计 §C4 第 4 条）。改完**必须逐条 `grade`**。

### 1.2 跨行 `And` → `∧`

机械脚本只处理**同一行内**、操作数是「原子名或配对括号」的形状；跨行的
（`And (Set.MapsTo …)\n  (Exists …)`）要人工改。原则与 §1.1 同：
`∧`（35）比应用紧，所以外层参数可以去掉括号，但**保留能让读者一眼分组的括号**。

### 1.3 项模式证明 → `by` 块

`lib/Demo.sokonanoda` 有 4 条（`demo_and_comm` / `demo_exists_intro` /
`demo_exists_elim` / `demo_equiv_refl`），解答里另有少量。设计 §C1.2 要求
**已证声明全写成 `by` 块**。`def`（不是证明）**保持项模式**——那是定义、不是证明。

⚠️ 已知边界：`·` 聚焦**没进语法**（实测 parse 错），多子目标用**顺序**写法
（`constructor` 后逐个子目标 `intro`/`exact`），或点名的 `Iff.intro`/`And.intro`。

## 2. 纪律

1. **只改指定文件**；`crates/`、`docs/`、`course/`（入门课）、`playground` 一行不动；
2. 判卷**一律绝对路径**：`scripts/soko grade "$PWD/courses/set-theory/<file>" --json`；
3. 改一条判一条；整片改完跑 `python3 courses/set-theory/tools/check.py`，
   必须仍是 **328 / 99 / 0**——**计数动了就是改坏了**（纯记法改写事件中性）；
4. `notation-cheatsheet*` 两个字**不要动**；
5. 撞到判卷器边界（tactic 报错而代码"看着对"）→ 别硬绕：把最小复现写进交付说明，
   能改名/换等价写法就换（课程里已有先例：`unit12-solution` 的 5 处 `cases` 绕法），
   但**不许**改 `crates/`。

## 3. 交付格式

```
文件            改动要点                        grade 退出码 / 计数
<path>          ∃ 记法 6 处、And→∧ 9 处         0 / (checked 21, open 0)
...
整门课门禁：36 目标 · 328 checked · 99 open · 0 判负（exit 0）
仍欠 / 边界：<最小复现 + 报错原文>
```

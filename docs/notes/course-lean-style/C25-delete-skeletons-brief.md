# C2.5 施工手册：删入门课自建 `And`/`Or` 骨架（用户 2026-09-21 拍板**删**）

> 设计依据 `docs/design/course-lean-style.md` §C2.5（该行原本标「需拍板（推荐删）」，
> 2026-09-21 用户选**删**）。**判据**：`cargo test -p sokonanoda-cli --test course
> --test course_status --test course_shared --test cli` 全绿 + 入门课每个文件
> `scripts/soko grade <绝对路径>` exit 0。**`crates/kernel/` 零改动**。

## 0. 为什么删（一句话）

入门课今天在**每个单元文件里手抄一遍** `axiom And` / `axiom Or` 骨架。文件一旦自己
声明了 B3/B4 族里的名字，prelude 的**整族**就让位（`prelude.rs` 的让位规则）——
于是学习者拿到的是**公理版的 And/Or**：没有 `And.rec`/`Or.rec`，`constructor`/
`cases`/`left`/`right` 在 ①④⑧ 全不可用。**把自建骨架整块删掉，prelude 的真归纳
自动回来**（`And.intro/left/right`、`Or.inl/inr` 的名字与**位置参数个数一字不变**），
于是入门课与卷 I 的写法统一，四个 tactic 也解锁。

## 1. 删什么、留什么（**逐字**）

**删除**（整块，含它自己的注释）：
- `axiom And : Prop → Prop → Prop` + `And.intro` + `And.left` + `And.right` 四行；
- `axiom Or : Prop → Prop → Prop` + `Or.inl` + `Or.inr` 三行；
- 单元⑨⑩⑪ 里的 **`inductive Or … end` 整块**（含 `ctor inl`/`ctor inr`）——
  prelude 的 `Or` 是真归纳，自动派生 `Or.rec`；
- 说明这些骨架的**注释**（改成"这些名字 prelude 自带，见下"）。

**保留**：
- `axiom True : Prop` / `axiom True.intro : True` / `axiom False : Prop`
  ——单元① 要拿它们当**`axiom` 是什么**的教学例子（设计 §C2.5 明写保留）；
- 所有**引用**这些名字的代码（`And.intro a b ha hb`、`Or.inl a b h`、`Or.elim`…）：
  prelude 的签名照旧**不给隐式实参**，调用形状一字不改。

## 2. 落在哪些文件

| 组 | 文件 | 删的块 |
|---|---|---|
| ①④⑧ | `course/unit{1,4,8}-*.sokonanoda`、`course/en/unit{1,4,8}-*.sokonanoda`、`course/solutions/unit{1,4,8}-*-solution.sokonanoda`、`course/en/solutions/…` | `axiom And*`（4 行）+ `axiom Or*`（3 行） |
| ⑨⑩⑪ | `course/unit{9,10,11}-*.sokonanoda` + EN + 解答（中英） | `axiom And*`（4 行）+ **`inductive Or … end` 整块** |
| 项目 | `course/unit11-project/Logic.sokonanoda`、`Canvas.sokonanoda`、`Exercises.sokonanoda`、`solutions/Exercises-solution.sokonanoda` | `axiom And*`（4 行） |
| 画布 | `playground.sokonanoda` | `axiom And*` + `axiom Or*` |

**不动**：卷 I（`courses/set-theory/`）、`crates/`、`course/shared/Nat.sokonanoda`。

## 3. 叙事要改的地方（**不改成假话**）

- 单元① 开头讲"逻辑骨架"的那几段：改成「`True`/`False` 这里用 `axiom` 演示
  **公理**是什么；`∧ ∨ ↔ ¬ →` 这些连接符与它们的构造子 **prelude 自带**
  （Lean core 级），不用自己造」。
- 单元⑨ 的「9.1 `Or`：从公理升级为真正的归纳类型」整节动机**失效** ⇒ 改成
  「`Or` 一直是真归纳；本单元补上它的**消去子用法**（`cases` / `Or.elim`）与
  `Iff` 定义」。
- 单元⑪ 的「单元⑨ 的 `Or` 是真归纳，单元① 的 `Or` 只是公理」对比段 ⇒ 改成
  「`import` 的名字解析」的动机（同一个名字在两处可以指不同东西——用**两个都自建**
  的例子讲，例如自定义同名 `def`），不要再用"公理 vs 归纳"当例子。
- 凡是「骨架」「自建」「升级」「本课的 `And`/`Or` 是公理」的句子都要复核。

## 4. 计数会变（**必须重取，不许手算**）

删掉 N 条 `axiom` 声明 ⇒ 该文件 `decl.checked` 少 N。**画布的 `exercise.open` 不变**
（练习声明一个没动）。重钉四处：`course.rs` 的 `GOLDEN`、`course_status.rs` 的逐单元表
+ summary、`cli.rs` 的 warm-cache 总计、`course_shared.rs` 的副本表
（`And`/`Or` 的副本表**整表删掉**，因为没有任何副本了 —— `Nat` 保留）。

⚠️ **`course/shared/{And,Or}.sokonanoda` 两个规范模块本身也要删**（没有副本可守了），
并同步 `course/shared/Demo.sokonanoda` 的 `import`（若它 import 了这两个模块）。

## 5. 解锁之后**要不要**改课程内容？

**本轮不改教学法**：单元④ 仍然教那六条 tactic（`intro/exact/assumption/apply/rfl/have`），
`constructor`/`cases`/`left`/`right` 只是**从不可用变成可用**——把它们写进课程是**下一轮**
的教学决定（设计 §C2.6 的教学顺序问题）。本轮只保证：**课程照旧全绿 + 写法统一**。

## 6. 交付格式

```
文件（相对路径）      删了什么                           grade 退出码 / (checked, open)
...
测试：cargo test -p sokonanoda-cli --test course --test course_status --test course_shared --test cli → N passed / 0 failed
仍欠 / 边界：<最小复现 + 报错原文>
```

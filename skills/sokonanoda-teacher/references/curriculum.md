# 课程素材库地图（teacher 参考）

> `course/` 只是路线图/题池/可解性守护；**执行层必须按用户灵活适配**
> （用户原则，见 `docs/teaching-session.md` §0）。用户面对的只有画布。

## 单元地图（`course/course.json` 为准）

| # | 文件 | 主题 | 关键概念 |
|---|---|---|---|
| 1 | `unit1-expressions-types.sokonanoda` | 表达式与类型 | Nat 字面量、`+` 会计算、类型也有类型（`Sort 1`） |
| 2 | `unit2-functions-arrows.sokonanoda` | 函数与箭头 | `fun`、命名箭头、高阶函数 `(Nat -> Nat) -> Nat -> Nat`、binder 推断 |
| 3 | `unit3-propositions.sokonanoda` | 命题与证明项 | 证明=项；axiom 搭 True/False/And/Or/Not；False.rec；`Eq` 三件套 |
| 4 | `unit4-equality-rfl.sokonanoda` | 等式与 rfl | `Eq.{1}`、`Eq.refl` 会计算、自己造 symm（设计谓词 p） |
| 5 | `unit5-induction-nat-rec.sokonanoda` | 显式归纳与递归 | `inductive Nat` 块、ctor/rec/iota、`Nat.rec` |

每个单元配 `solutions/unitN-*-solution.sokonanoda`（agent 专用钥匙，全部
经完整内核验证；CI golden 钉死事件计数）。

## 逻辑先行（用户原则，2026-09-07）

单元顺序**不等于**教学顺序。执行时优先从"命题与证明项"的直觉入手：
先让用户证明 `True`、`And a b -> And b a` 这类命题（逻辑骨架用 axiom 搭），
把 `Sort` 留到"函数类型的类型是什么"这个自然问题出现时再引入。
题目本身（题池）不变，变的是编排与节奏。

## 适配规则速记

- 反复出错 → 同概念变式题 / 先给填好的演示再重出题；
- 进度快 → 跳热身、合并单元；慢 → 拆小步、加提示层；
- 用户背景（程序员/学生/新手）→ 调整比喻（数据结构/游戏/代数）；
- 每单元 3–8 题的节奏感：热身 1 题 → 核心 2–4 题 → 挑战 1 题（★ 标记）。

## 新语法点入库流程（给未来的你）

新增语法 = 三件套：front 单元测试 + CLI e2e + 课程单元用例，同时更新
白名单与 `docs/architecture.md`。禁止只加编译器不加课程。

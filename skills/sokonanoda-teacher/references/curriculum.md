# 课程素材库地图（teacher 参考）

> `course/` 只是路线图/题池/可解性守护；**执行层必须按用户灵活适配**
> （用户原则，见 `docs/teaching-session.md` §0）。用户面对的只有画布。

## 单元地图（`course/course.json` 为准；逻辑先行排序，2026-09-07 重排）

| # | 文件 | 主题 | 关键概念 |
|---|---|---|---|
| 1 | `unit1-propositions-proofs.sokonanoda` | 命题与证明项 | 证明=项；axiom 骨架 True/False/And/Or/Not；False.rec；先证明命题，不谈 Sort |
| 2 | `unit2-equality-rfl.sokonanoda` | 等式与 rfl | 认识数字 Nat；`Eq.{1}` 机械规则（为什么是 1 → 单元④揭晓）；自己设计谓词 p 造 symm/trans |
| 3 | `unit3-functions-arrows.sokonanoda` | 函数与箭头 | `fun`、binder 推断、高阶函数 `(Nat -> Nat) -> Nat -> Nat`；结尾埋"函数类型的类型？"悬念 |
| 4 | `unit4-universes-sort.sokonanoda` | 宇宙 | Sort 由悬念揭晓：Prop = Sort 0（回收单元①）、Nat : Sort 1；`Eq.symm {u}` 毕业题 |
| 5 | `unit5-induction-nat-rec.sokonanoda` | 显式归纳与递归 | `inductive Nat` 块、ctor/rec/iota、`Nat.rec` |
| 6 | `unit6-by-tactics.sokonanoda` | by 写法 | `by` 块 + 五个 tactic（intro/exact/apply/assumption/rfl）；`by sorry` 占位；值位 `intro` 一次全剥（编辑器提示展开为 fun 骨架）；判定走 kernel |

每个单元配 `solutions/unitN-*-solution.sokonanoda`（agent 专用钥匙，全部
经完整内核验证；CI golden 钉死事件计数）。

## 逻辑先行（用户原则，2026-09-07；单元排序已按此重排）

题池顺序 = 新排序（命题先于宇宙）。执行时仍然按用户灵活适配：
如果用户带着"什么是 Sort"的抽象兴趣来，可以先在画布上口头展开再回到题池；
如果用户在证明练习里被 `Eq.{1}` 卡住，直接用"单元②的机械规则"话术，
不必提前剧透宇宙。题目本身不变，变的是编排、提示层与比喻。

## 适配规则速记

- 反复出错 → 同概念变式题 / 先给填好的演示再重出题；
- 进度快 → 跳热身、合并单元；慢 → 拆小步、加提示层；
- 用户背景（程序员/学生/新手）→ 调整比喻（数据结构/游戏/代数）；
- 每单元 3–8 题的节奏感：热身 1 题 → 核心 2–4 题 → 挑战 1 题（★ 标记）。

## 新语法点入库流程（给未来的你）

新增语法 = 三件套：front 单元测试 + CLI e2e + 课程单元用例，同时更新
白名单与 `docs/architecture.md`。禁止只加编译器不加课程。

# 课程素材库地图（teacher 参考）

> `course/` 只是路线图/题池/可解性守护；**执行层必须按用户灵活适配**
> （用户原则，见 `docs/teaching-session.md` §0）。用户面对的只有画布。

## 单元地图（`course/course.json` 为准；逻辑先行排序，2026-09-07 重排）

| # | 文件 | 主题 | 关键概念 |
|---|---|---|---|
| 1 | `unit1-propositions-proofs.sokonanoda` | 命题与证明项 | 证明=项；axiom 骨架 True/False/And/Or/Not；False.rec；声明级 binder（`theorem f (a : A) : B := v`）两种拼写；先证明命题，不谈 Sort |
| 2 | `unit2-equality-rfl.sokonanoda` | 等式与 rfl | 认识数字 Nat；`Eq.{1}` 机械规则（为什么是 1 → 单元⑤揭晓）；自己设计谓词 p 造 symm/trans |
| 3 | `unit3-functions-arrows.sokonanoda` | 函数与箭头 | `fun`、binder 推断、高阶函数 `(Nat -> Nat) -> Nat -> Nat`；结尾埋"函数类型的类型？"悬念 |
| 4 | `unit4-by-tactics.sokonanoda` | by 写法 | `by` 块 + tactic（intro/exact/apply/assumption/rfl）；换行分隔 tactic；`by sorry` 占位；判定走 kernel（P2 从旧单元⑥提前） |
| 5 | `unit5-universes-sort.sokonanoda` | 宇宙 | Sort 由悬念揭晓：Prop = Sort 0（回收单元①）、Nat : Sort 1；读 `#check` 输出；`Eq.symm {u}` 毕业题 |
| 6 | `unit6-induction-recursion-1.sokonanoda` | 归纳与递归 Ⅰ | 显式 `inductive Nat` 块、ctor/rec/iota、手写 `Nat.rec`；非递归枚举 `Color` 的 `match`；递归 `match` 自动 IH（P2 拆前半） |
| 7 | `unit7-induction-recursion-2.sokonanoda` | 归纳与递归 Ⅱ | 参数化 `Option`；依赖 `match`=数学归纳法；嵌套模式与通配；带索引归纳 `Vec`（P2 拆后半） |
| 8 | `unit8-quantifiers.sokonanoda` | 量词 | `forall` 引入=fun / 消去=应用；`Exists` 公理三件套（intro=证人、elim=函数，结论不提证人）；Person/someone 论域；∀/∃ 与 And 的分配、∀→∃、∃ 单调（7 题，含 ★/★★）；可紧跟单元①教学 |
| 9 | `unit9-relations-connectives.sokonanoda` | 关系与联结词 | `Or` 升级为真 `inductive`（前端自动派生 `Or.rec`；`match` 降低到它）；`Iff` 是 `def Iff A B := And (A -> B) (B -> A)`；`Le`/`Even` 带索引归纳关系 + **手写** `rec`/`iota`（自动派生对索引递归 Prop 有缺口，见 `docs/HANDOVER.md` §3）；消去/inversion 引理。练习：`or_comm` / `or_elim` / `or_id`(R) / `iff_mp` / `iff_mpr` / `le_zero`(L) / `le_trans`(X) / `even_four`（8 题） |
| 10 | `unit10-reading-proofs.sokonanoda` | 读证明与综合 | 不教新语法，练「读」：自解释三问（Hodds/Alcock/Inglis）；formal↔informal 翻译对；评阅两份被内核拒绝的错证明并写出改正版；期末小项目。练习：`or_intro_x`(X) / `iff_intro_x`(X) / `nat_eq_self_x`(X) / `or_comm_fixed`(R) / `or_right_fixed`(R) / `and_or_imp`(综合)（6 题）；成品仍必须过内核 |
| 11 | `unit11-modules-projects.sokonanoda` | 模块与项目 | `import Foo.Bar` 置顶规则；模块名 ↔ 路径（`-` 不是模块名字符）；模块根 = `--root` > 最近的 `sokonanoda.toml` > 入口目录（**无清单也能 import**）；闭包级重名/成环/依赖阻断/prelude 规则；开放练习不进导入者环境。跨文件练习在可运行项目 `course/unit11-project/`（`Logic` + `Canvas` + `Exercises`，6 题，判卷命令见该单元 11.6 节） |

每个单元配 `solutions/unitN-*-solution.sokonanoda`（agent 专用钥匙，全部
经完整内核验证；CI golden 钉死事件计数）。

## 0.40–0.51 新增语言点（可随时出题）

固定单元之后新开的能力；单元⑥/⑦ 已吸收 `match`/嵌套模式/`Vec`，其余按学习者
进度插入（出题纪律见 SKILL.md §4）：

- 值位 `let x : T := v; body`（设计 `elaborator-let-match.md`）；
- `match` 模式：`_` 通配、嵌套构造子（`some (succ k)`）、Nat 字面量
  （`| 0 =>`）、`Bool` 守卫（`| succ k if p =>`）、arm 有序首个匹配；
  递归 IH、依赖 motive（`match-patterns.md`）；
- `match` 作为 tactic（`by match c with | … => <项>`，`by-tactics.md` §2）；
- `by` 块换行分隔 tactic（`;` 或换行，可混用，无缩进敏感；`by-tactics.md` §11）；
- prelude `Bool`（`Bool.true`/`Bool.false`/`Bool.rec`，文件自带 `inductive Bool`
  时让位；`match.md` §10 Phase 5）；
- 参数化归纳（`Option`/`List`）与带索引归纳（`Vec`，v1 结果类型不依赖索引；
  `indexed-inductives.md`）；
- 应用位置 binder 类型推断（`(fun x => x) 1`、`(fun x y => x) 1 2`；
  `elaborator-let-match.md` as-built）。

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

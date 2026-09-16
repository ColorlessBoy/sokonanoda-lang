# 当前状态与进度日志（agents 先读这里）

> 快照：2026-09-16（第八十三轮：课程大纲重构 P2（重排 + 拆分）；0.53.0）
> 仓库：`sokonanoda-lang`；权威计划 = `ROADMAP.md`；**用户要求总账 = `REQUIREMENTS.md`（先读）**；
> **文档地图 = `docs/README.md`**（入口/权威在仓库根，开发者参考在 `docs/` 顶层，
> 设计在 `docs/design/`，调研笔记在 `docs/notes/`）；
> 架构/内核 = `docs/architecture.md`；协议 = `docs/protocol.md`；测试地图 = `docs/TESTING.md`；
> 经验台账 = `docs/LESSONS.md`；CI 失败台账 = `docs/CI-FAILURES.md`；发布 = `docs/RELEASE.md`；
> agent 入口 = `AGENTS.md` + `skills/`。

## 一句话

`.sokonanoda` = **纯声明式教学文件（无 `#` 命令）+ 完整 sokonanoda 内核 + LSP 反馈通道**。
练习 = 带 `sorry` 洞的 `def name : T` / `theorem name : T` / `example : T` 声明。
CLI/REPL 的 `#check` 等只是调试/自测工具，不是文件格式。

## 本轮进度（2026-09-16，第八十四轮：课程大纲重构 P3——锁定 10 单元）

> 续 `docs/design/course-syllabus.md` §6 P3：补齐锁定的最后两个单元。

1. **#9 关系与联结词**（`unit9-relations-connectives`）：把 `Or` 作为**真实归纳**声明
   （自动派生 `Or.rec`，`match` 降低到它）教「用」它；`Iff` 作为**定义**
   `And (A->B) (B->A)` 练定义展开；`Le`/`Even` 作为归纳关系 + 消去/归纳引理
   （PLFA inversion 套路）。8 题（T/R/L/X）。
2. **#10 读证明与综合**（`unit10-reading-proofs`）：自解释三问（Hodges/Alcock/Inglis）
   逐行读一份已证证明；formal↔informal 互译；两题「评阅错证明→写出能过内核的修正版」
   （错证明只放注释）；一题跨单元 capstone。6 题（X/R/T）。
   **判分口径**：不引入新协议事件——每道读/评阅题都要产出内核可判的声明（散文只在注释里、
   不计数）。
3. **规模**：unit9 `(13 checked, 8 open, 0 reduced)`、unit10 `(7,6,0)`；汇总
   **units=10 checked=78 open=59 failed=0**；`course_json_lists_the_ten_units_in_order`；
   `cli.rs` 缓存金值同步。
4. **门面同步**（硬规则）：teaching-session（新增「第三课：关系、联结词与读证明」键表）、
   course-status/course-bilingual/ROADMAP I7（改为「✅ 完成（锁定 10 单元）」）/
   course/README/infrastructure + teacher skills（curriculum 行 9/10、SKILL 的
   `Or`/`Iff` 归属）+ editor/vscode/README（10 单元）。
5. **记录两个产品缺口**（写入 HANDOVER §3 E + 课程大纲 §2 第 11/12 条）：
   (a) **带索引的递归 `Prop` 归纳**（`Le`/`Even`）自动派生 recursor 被内核拒（IH 形状不符），
   只能手写 `rec`/`iota`（`Or` 非索引 Prop、`Vec` 带索引 Type 均正常）——front 未冻结，**可修**；
   (b) `inductive` 的**多名字参数组** `(A B : Prop)` 不解析（Pi binder 支持）。
6. **验收**：course/course_status/skill + 全量 CLI + 手动 fmt/clippy/test 全绿；20 个 CN+EN
   画布 exit 0；双语画布与 solutions 逐项相等；版本 0.53.0 → **0.54.0**（课程达到锁定规模，minor）。

## 本轮进度（2026-09-16，第八十三轮：课程大纲重构 P2（重排 + 拆分 U5））

> 续 `docs/design/course-syllabus.md` §6 P2：把 `by` 提前、把过载的归纳单元拆开。

1. **重排**：`by` 单元从第 6 提到**第 4**（紧跟函数/箭头之后），宇宙顺延为第 5；
   量词为第 8。
2. **拆分**：旧「显式归纳与递归」拆成 **Ⅰ**（显式 `inductive`/`rec`/`iota` + 手写
   `Nat.rec` + `match` 非递归 + 递归 `match`+IH）与 **Ⅱ**（参数化 `Option` + 依赖
   `match`=归纳 + 嵌套/字面量/通配/guard 模式 + 带索引 `Vec`）。两半各自**重声明**
   `inductive Nat` 以保持自足（代价：`decl.checked` 57→58）。
3. **文件/清单**：`git mv` 重命名 CN/EN 画布与 CN/EN solutions（`unit6-by→unit4-by`、
   `unit4-univ→unit5-univ`、`unit5-ind→unit6/unit7-…`、`unit7-quant→unit8-quant`）；
   `course.json` 8 条，`unit` 1..8。
4. **测试**：两处 GOLDEN 改 8 行（`(13,6,1)(2,5,2)(2,6,2)(13,5,0)(0,6,1)(7,6,3)(7,4,4)(14,7,1)`）、
   汇总 `units=8 checked=58 open=45`、`course_json_lists_the_eight_units_in_order`、
   `cli.rs` 课程缓存金值 57→58。
5. **门面同步**（硬规则）：`teaching-session.md`/`course-status.md`/`course-bilingual.md`/
   `ROADMAP` I7/`course/README.md`/`infrastructure.md`/`type-level-syntax.md`/
   `term-intro.md`/`remove-funintro.md`/`skills/`（teacher 两个文件）/`editor/vscode/README.md`
   （7→8 单元，注明向锁定 10 单元演进）；并**移除**根 README 与 site 三处**手写单元数**
   （改为不带数字，数字由 `course/course.json` 生成）。
6. **验收**：course/course_status/skill + 全量 CLI 测试全绿；16 个 CN+EN 画布 exit 0；
   双语画布与 solutions 事件计数逐项相等；`solution_covers_every_canvas_exercise` 通过；
   版本 0.52.0 → **0.53.0**（结构可见变化，minor）。
7. **待做**：P3 = 新增 #9 关系与联结词、#10 读证明与综合（锁定的 10 单元）。

## 本轮进度（2026-09-16，第八十二轮：课程大纲重构 P1）

> 用户：重新拆解 course、全面调研形式化证明教材、设计教学大纲。先出设计
> （`docs/design/course-syllabus.md`），本轮执行 **P1（不改结构，先修问题）**。

1. **调研**（3 个 subagent 并行）：Lean 系（TPIL4/MIL/NNG/FPL/Lean4Game + 学习者
   障碍研究）、Coq/Agda/Isabelle/Idris + 传统证明教材（SF/PLFA/Concrete Semantics/
   TDD-Idris/Velleman/Hammack/Solow/Chartrand）+ 证明教育文献。结论：共享骨架
   （数据+递归 → 先算后证 → 蕴涵=`intro` → 分情况 → 归纳≡递归 → 引理链 → 联结词即证据）、
   三处分歧（逻辑先行 vs 计算先行 / 相等与关系谁先 / 自动化姿态，PLFA 明文禁用）、
   12 个可偷装置、首因障碍（语法词汇、不看 proof state、迁移失败、tactic bashing）。
2. **现状审计**：7 单元 golden/双语/solutions/skill 测试面 + 10 条内容/文档问题。
3. **设计锁定**（`course-syllabus.md` §0）：**10 单元**目标结构（`by` 提前到 #4；旧 U5
   拆 #6/#7；新增 #9 关系与联结词、#10 读证明与综合）+ 三套候选大纲（A 推荐 / B NNG
   游戏线 / C PLFA 式进阶）+ P1–P4 阶段与硬约束（白名单、两处 golden、双语、solutions、
   skill 锚点、CI）。
4. **P1 内容**（subagent）：U4 增 2 题「读 `#check` 判类型」+ 1 题「先预测再证」（原
   0 checked/0 reduced）；U6 删与 `by_ex1` 完全重复的 `by_ex5`；U3 两题去歧义；
   全单元 hint 去泄题（关键件只写触发条件+引理名）；删 U5 过期断言；solutions 同步 +
   修 EN unit4 漂移；新增 `solution_covers_every_canvas_exercise` +
   `en_solutions_match_chinese_event_counts`。新 golden：U4 `(0,6,1)`、U6 `(13,5,0)`，
   汇总 `units=7 checked=57 open=45 failed=0`（`cli.rs` 的 43→45 一并修）。
5. **P1 文档**（subagent）：`teaching-session.md` §3 编号/练习名对齐真实单元、删 §5
   的 `Or.rec/or_comm` 断言；`course-status.md` §4 golden 更新并标注"示例非第二真源"；
   `ROADMAP` I7 改为 7 单元 + 指向锁定 10 单元演进；`course-bilingual.md` 口径与不变式
   （画布**与** solutions 都比较、含 `expr.typed`）；`course/README.md`；本文 §2 逐条标注
   `P1 已修/P2 待做`；另修 `infrastructure.md` 两处 5 单元口径。
6. **验收**：course/course_status/skill + 全量 workspace 全绿；`sokonanoda gate` PASS；
   版本 0.51.0 → **0.52.0**（课程内容可见改进，minor）。P2（重排+拆分 U5）、P3（新增两
   单元）、P4（游戏线）待做。


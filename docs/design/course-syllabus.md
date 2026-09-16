# 设计：课程大纲重构（course syllabus，2026-09-16）

> 触发（用户）：「重新拆解一下 course 的内容，全面深入调研一下网上的各个形式化证明的
> 教材，设计一些教学大纲。」
>
> 本文 = 调研综合 + 现状审计 + 3 套候选大纲 + 推荐方案 + 迁移计划。**只设计，不改内容**；
> 落地按 §6 分阶段（每阶段都能单独过 gate/发布）。约束面（白名单/双语/golden/skill）
> 见 §4，任何改内容的计划必须先读它。

## 1. 调研综合（Lean 系 / Coq·Agda·Isabelle·Idris / 传统证明教材）

### 1.1 成功教材共享的骨架

TPIL4、MIL、NNG、Software Foundations(LF/PLF/VFA)、PLFA、Concrete Semantics、
TDD-with-Idris、Velleman/Hammack/Solow/Chartrand 的共同推进：

1. **先定义数据+模式匹配函数**，再在它上面证命题；
2. **先算后证**：第一个"定理"是定义上成立的计算式等式（`rfl`/`reflexivity`）；
3. **蕴涵/全称 = 函数 = `intro`**（propositions-as-types 的入口）；
4. **按构造子分情况**（`destruct`/`cases`/`match`/分情形方程）；
5. **归纳 ≡ 递归**（SF：「让递归尽量简单」；PLFA：「induction is recursion」）；
6. **引理链**：先证 helper 再组合（SF "proofs within proofs"）；
7. **联结词即证据**：∧=对、∨=注入、⊥/¬=到空类型的函数、∃=见证；
8. **关系用归纳定义 + 推理规则书写**；**相等本身也是归纳类型**（唯一 `refl`）；
9. 列表/树 + 高阶函数，证明通用引理；
10. 收束到应用（语义/类型安全/验证算法）；
11. **自动化是显式议题且通常后置**（SF 把 `auto/lia` 推到后面；PLFA **明文禁用**）。

### 1.2 主要分歧（决定我们的取舍）

| 分歧 | 两派 | 证据 |
|---|---|---|
| 逻辑优先 vs 计算优先 | 传统教材（Velleman/Hammack/Chartrand/Solow）逻辑/集合先行；SF 函数式先行；PLFA 归纳先于联结词；Nipkow 交织 | 各书前言/目录 |
| 相等与关系谁先 | PLFA 刻意「先用归纳定义 `≤` 并证明，再把 `≡` 定义为归纳类型」 | PLFA `Relations` → `Equality` |
| 自动化姿态 | SF 延迟；Nipkow 第一行就 `auto`（再用 Isar 补人读的证明）；**PLFA 永不**（理由：概念要学两遍、IH 生成"神秘"、同名双写法、Curry–Howard 被藏起来；且"不用 tactic 的证明并不更长"） | PLFA Preface；Wadler |
| 证明作为文本 | Agda/PLFA 的 term proof 纸面可读；Coq tactic script 需 IDE；Isabelle 用 Isar 补 | 三书序言 |
| 第一个非平凡定理 | SF/Nipkow/PLFA：算术等式；教材：数论/奇偶；CPDT/VFA：程序正确性 | 各书章节 |

### 1.3 值得"偷"的教学装置（按可移植性）

1. **PLFA 式封闭白名单**：明确列出本课允许的 tactic/构造子，并写清"为什么不用更多"。
2. **SF 式填空 + 星级 + 部分分**（`FILL IN HERE`/`Admitted`）：对我们 = `sorry` 预算 +
   星级（★/★★）。
3. **Nipkow 式「定义-证明成对」**：每个 `inductive` 后固定配 2–3 条引理（构造子消去 +
   一条递归性质）。
4. **Nipkow 式「同一命题三种证明」**：机器证明 / 简短非形式 / 传统完整归纳——对抗
   「PA→纸笔迁移不足」。
5. **Brady 式 type→define→refine**：先只写类型 + `sorry`，再逐步填。
6. **PLFA 式等式接力**（`≡-Reasoning`）：没有 `rw` 时教相等推理的最佳替身。
7. **PLFA 式 inversion 引理套路**（`inv-s≤s : suc m ≤ suc n → m ≤ n` 用 `match` 消证据）：
   把"对证据做 case/inversion"变成**必须自己写的引理**，零魔法。
8. **Solow 式关键词→技巧映射表**：`if/only if/for all/there exists/not` → tactic 序列。
9. **Velleman/Solow 式 givens/goals 两栏 scratch**：写证明前先列假设与目标。
10. **Chartrand/Selden 式 Proof Evaluations**：给学生一份**有错的证明**去评/去修。
11. **Hodds/Alcock/Inglis 自解释训练**：读证明逐行自问三问（为什么成立/用了哪条前文/
    连到哪个已知），实测 +30% 理解且持续约 3 周 → 做成"读证明 checklist"。
12. **显式依赖树 + 练习分层标签**（Hammack 的 dependency tree；PLFA 的
    `(recommended)/(stretch)`）。

### 1.4 学习障碍与对症动作（有文献支撑）

| 障碍 | 证据 | 动作 |
|---|---|---|
| 语法/词汇是第一失因（"像用中文讲法语"） | Iannone & Thoma；Tran Minh；Avigad「syntax is fiddly」 | 隐藏复杂度、先教词汇、例子充足、按题目给"本题只用这几样"清单 |
| 不看 proof state / 只当 oracle 试错 | Garnelo & Liebendörfer 2025；QED in Context (OOPSLA'25) | **先教读输出**（专门的"读目标/读报错"练习）；把 proof state 作为第一公民；延迟自动化 |
| 技能顺序 = 语法→概念→策略 | Bayman & Mayer；Garnelo & Liebendörfer 实证 | 缺哪层就补哪层：先给骨架让他只练概念，再撤骨架 |
| 迁移失败（看不出 Lean 证明 = 纸笔证明） | Iannone & Thoma；BMN22 | **同一命题写两份**（形式 + 散文）；用 `sorry` 分段（chunking） |
| 只 reactive tactic bashing | Iannone & Thoma（NNG 四学生） | 先纸笔策略、再上机器；禁止一步闭合的自动化 |
| 不会"拆包定义"/缺 strategic knowledge | Moore 1994；Weber 2001 | 每个定义配「展开后要证什么」；关键词→技巧映射；把"选 tactic"当练习 |
| 归纳的认知障碍 | Dubinsky & Lewin 1986 等 | 先 quasi-induction，再用**递归函数**具身化归纳 |
| 反证法难（资源假说） | IJRUME 2021 | 先备齐定义与引理再证；找矛盾作为搜索练习 |

### 1.5 我们**不具备**的 Lean/Coq 能力（大纲必须绕开）

`rw`/`rewrite`、`simp`/`ring`/`omega`/`lia`/`decide`、`cases`/`induction`/`have`/
`constructor`/`use`/`rcases`、`structure`/records、typeclasses/instances、imports/
namespaces、经典逻辑（`em`/`by_contra`）、`Iff`/`↔`、`Or.rec`、匿名构造子 `⟨⟩`、
`Type u`（用 `Sort u`）。

**替代路线（写进大纲）**：
- 重写 → 手写 `Eq.symm/trans/cong/subst` 链 + 散文式等式接力；
- 化简/自动判定 → `#reduce` 算闭项 + 精选算术引理库（`add_zero`/`zero_add`/`add_comm`…）
  并要求**手写引用**；teacher 只给"这一步需要哪条引理"的 hint；
- `cases`/`inversion` → `match` 分情况 + 对证据写消去引理；
- `induction` → **递归 helper 引理**（recursion = induction）；`#print` 展示归纳原理对照；
- `have` → 顶层 helper `theorem`（并把"顺序与依赖"当证明工程来教）；
- `Or`/`Iff`/`∃` → 用 `inductive` 定义（`Or` 需要自己的消去子；`Exists` 已用公理声明）；
- 依赖类型 → 把长度性质写成**普通命题**（`length (append xs ys) = …`），再用 `Vec` 做
  "类型级 vs 命题级"对照。

## 2. 现状审计（摘要；细节见 §4 与 `crates/cli/tests/course.rs`）

7 单元（`course/course.json`）：①命题与证明项 ②等式与 rfl ③函数与箭头 ④宇宙
⑤显式归纳与递归 ⑥by 写法 ⑦量词。golden：
`(13,6,1)/(2,5,2)/(2,6,2)/(0,3,0)/(13,10,7)/(13,6,0)/(14,7,1)`，汇总
`units=7 checked=57 open=43 failed=0`。

**已确认的问题（改内容时一并修）**：
1. **U5 过大**（253 行 / 10 题）：显式归纳 + iota + `match` + 递归 IH + 参数化 +
   依赖 match + 嵌套模式 + 带索引 `Vec` 挤在一单元。
2. **`by`（U6）来得太晚**：学习者到第 6 单元才见 tactic，而前 5 单元全靠 term——与
   "即时反馈/低门槛"的调研结论相反。
3. **U3 两题欠定义**（`three_args`/`body_uses_let` 返回"任意 Nat 即可"）；**`by_ex5`
   与 `by_ex1` 完全重复**且 hint 描述有误。
4. **散文常把答案写进提示**（违背 teacher skill「答案绝不写进提示」）：U1 ex2/ex5、
   U3 ex2/ex3、U5 ex2/ex3、U6 ex6。
5. **U5 内有过期断言**（L80「v1 的 match 只做非递归」与它自己教的递归 match 矛盾）。
6. **`Or` 只声明不练**（无 `Or.rec`、无 `Or` 练习；`Or.inr` 全域未用）；**`Iff`/`↔`
   完全缺失**；skill 的"逻辑先行"清单却提了 Iff。
7. **零"读证明/评阅/翻译"练习**：全是"填项/填 tactic"，没有 formal↔informal 互译、
   没有"给错证明找错"、没有"展开定义后要证什么"。
8. **U4 偏薄**（0 checked / 0 reduce，只有 `#check` 与两个 `example`），无"读取 `#check`
   输出"的练习。
9. **文档漂移**：`docs/teaching-session.md`（§3 numbering、§5 声称 U5 有 `Or.rec`/`or_comm`
   ——实际没有）、`docs/design/course-status.md` §4 旧 golden、`ROADMAP.md` I7 仍是
   5 单元/`lesson-XX` 旧计划、`course/README.md` 与 `course-bilingual.md` 的单元数口径。
10. 双语镜像的 solution 未做事件计数对比 → **EN unit4 solution 已漂移**（漏 `#check (Type 0)`）。

## 3. 三套候选大纲

> 记号：**[新]** = 新增单元/内容；**[拆]** = 拆分现有单元；**[动]** = 调整位置；
> 每单元末尾给"练习类型目标"（`T`=term 填空、`B`=by/tactic 填空、`R`=读/评阅、
> `X`=翻译 formal↔informal、`L`=引理自证）。

### 大纲 A：逻辑先行（**推荐**，保持现有读者与叙事，修补+扩展）

面向中文 transition-to-proof 读者；逻辑连接词与量词先建直觉，`by` 提前做"反馈加速器"，
归纳拆成两单元，结尾加"关系与读证明"。

| # | 单元 | 概念顺序 | 借用装置 | 练习目标 |
|---|---|---|---|---|
| 1 | 命题与证明项（现 U1） | `Prop` → `True/False` → ∧/∨/¬ 公理 → 证明=项 → 箭头↔`fun` → `False.rec` → 声明 binder | 三种证明（机器/非形式/传统）；givens/goals 两栏 | T 6 + R 2（给错项找错） |
| 2 | 等式与 `rfl`（现 U2） | `Nat` 与 `+` → `Eq/Eq.refl/Eq.subst` → `refl` 靠 conv → 手写 `symm/trans/cong` | PLFA 等式接力；`#reduce` 先算后证 | T 5 + L 3（自证 symm/trans/cong）|
| 3 | 函数与箭头（现 U3，修题） | 一切有类型 → 箭头右结合 → `fun`（可推断 binder）→ 洞 → 高阶 → `let` | type→define→refine | T 6（去歧义）+ R 1 |
| 4 | `by` 写法（现 U6，**提前**） | `by` → `intro/exact/assumption/apply/rfl/match` → 换行分隔 → 多子目标 | 关键词→技巧映射表；封闭白名单说明 | B 8（含 4 种 tactic 触发条件） |
| 5 | 宇宙（现 U4，增练） | `Sort n` 阶梯 → `Type n` 糖 → `#check` 读法 → `Eq.{1}` 由来 → 隐式宇宙 binder | 读输出专项 | R 3（从 `#check` 输出判类型）+ T 2 |
| 6 | 归纳与递归 Ⅰ（现 U5 前半 **[拆]**） | `inductive`+`ctor`+`iota` → 手写 `Nat.rec` → 递归 `def` → `match` 分情况 → 递归 `match`+IH | Nipkow 定义-证明成对；recursion=induction | T 5 + L 2（给 inductive 写消去子） |
| 7 | 归纳与递归 Ⅱ（现 U5 后半 **[拆]**） | 参数化 `Option A`/`List A` → 嵌套/字面量/通配/guard 模式 → 依赖 match=归纳法 → 带索引 `Vec` | 三种证明；`#print` 对照归纳原理 | T 6 + X 1（把递归定义翻成散文） |
| 8 | 量词（现 U7） | 命名箭头 → `forall`/`∀` → 证明=fun、使用=应用 → `Exists` 公理 → 见证与消去 | givens/goals；witness 搜索 | T 5 + L 2 |
| 9 | **[新]** 关系与联结词 | `Or`（含自写 `Or.rec`）→ `↔`（定义 + 两个方向）→ 关系归纳定义（`≤`、`Even`）→ 消去/inversion 引理 → 传递闭包式练习 | PLFA inversion 套路 | T/L 6 + R 2（评阅错误归纳）|
| 10 | **[新]** 读证明与综合 | 自解释三问 → formal↔informal 互译 → 评阅比赛 → 期末小项目（用 1–9 单元能力证一条小定理并写散文证明） | 自解释训练；Proof Evaluations；dependency tree | X 4 + R 3 + 项目 1 |

依赖树：1 → 2 → 3 → 4（`by` 需 1/2/3）→ 5（宇宙可随时插）→ 6 → 7 → 8 → 9 → 10。

### 大纲 B：NNG 式游戏化速通（计算/等式先行，逻辑后置）

把 `Nat`/`Bool` 上的等式与递归做成"世界"链，逻辑（→/∀/∧/∨/∃）推到 7 之后；每关极小、
即时反馈、背包（tactic/引理）按关解锁。

1 `rfl` 世界 → 2 等式接力（手写 symm/trans/cong）→ 3 `succ`/字面量世界 → 4 加法与归纳
（递归 `add` + `add_zero`/`zero_add`/`add_comm`）→ 5 乘法/幂 → 6 `Bool` 与 `if` 世界 →
7 蕴涵世界（`intro/exact/apply`）→ 8 全称世界 → 9 ∧/↔ 世界 → 10 ∨/∃/¬ 世界 →
11 归纳数据类型（`List`/`Option`）→ 12 关系与 `≤` → 13 带索引 `Vec` → 14 boss（小定理整证）。

- 优点：门槛最低、最像已被验证有效的 NNG；天然适配 Infoview 的即时目标态 + hint 阶梯。
- 代价：与本仓库现有文本（逻辑先行、散文式讲解）**风格冲突大**，第 1–6 单元无逻辑内容，
  `for`/`∃` 很晚；双语/课程地图/大纲叙事都要重写；对"想学证明"的读者可能显得绕。
- 适合：作为**并列的第二条入口**（"游戏线"），而不是替换主线。

### 大纲 C：PLFA 式显式（关系→相等，term-first，无自动化）

不讲 tactic 优先，把"关系归纳定义 + 手写消去引理 + 递归=归纳"作为主线；`by` 只作为
后期便捷层；强调证明**纸面可读**。

1 `Nat`/`Bool` 与递归函数 → 2 归纳（递归即归纳）→ 3 关系归纳定义（`≤`/`Even`）+
inversion 引理 → 4 相等作为归纳类型（唯一 `refl`）→ 5 同构/嵌入 → 6 `∧/∨/∃/¬`（都由
`inductive` 定义）→ 7 量词 → 8 可判定性（`Bool` vs `Prop`）→ 9 列表与高阶函数 →
10 参数化/带索引 → 11 `by` 与 tactic 的定位（读者已知 term，再看 tactic 省了什么）→
12 读证明与评阅。

- 优点：与我们"无 `rw`/无自动化"的现实**最契合**；trmin proof 可直接纸面阅读（迁移好）；
  概念全部显式（消去引理、inversion），教学价值高。
- 代价：对数论/逻辑直觉的读者偏"类型论味"；`by` 后置与现行 U6 位置冲突更大；需要写出
  `≡-Reasoning` 式散文模板作为替代。
- 适合：作为**进阶线**（给已会一点 Lean 的读者），或作为大纲 A 的 6–9 单元的深化版。

## 4. 内容改动的硬约束（任何大纲落地前必读）

- **白名单**：只用已实现语法（§1.5 列出缺什么）。`sorry` 仅限**值尾/构造子 spine/已知函数
  直接实参**，否则 `elab-hole-misplaced`；`match` 结果类型不得依赖索引。
- **golden 双改**：练习/演示增删 → `crates/cli/tests/course.rs::GOLDEN` **与**
  `course_status.rs::GOLDEN`（+ 汇总 `units/checked/open/failed`）。新增单元 → 两个 GOLDEN
  数组 + `course.rs` 文件数断言 + `course.json` 顺序表 + 汇总 `units`。
- **CN+EN 双语**：每个顶层画布都要同名 `course/en/` 镜像，`(checked, open, reduced,
  diagnostic)` 逐项相等；每份 CN solution 都要 EN 镜像且 0 诊断/0 open。
- **solutions**：每单元一份、洞全填、0 诊断/0 open（现在是"与画布不比对"，建议**新增**
  一条"solution ⊇ 画布练习名"的测试，顺带修 EN unit4 漂移）。
- **skill/docs**：`skill.rs` 要求 `course.json` 每单元都有画布 + solution，且 skill 里
  出现的 `decl.*/exercise.*/soko/*` 词必须在 `docs/protocol.md`；同步更新
  `docs/teaching-session.md`（修 §3/§5 漂移）、`course/README.md`、`docs/design/course-{status,bilingual}.md`、
  `ROADMAP.md` I7、`editor/vscode/*` 与 `skills/`（门面同步硬规则）。
- **CI**：`cargo test -p sokonanoda-cli --test course` + workspace；只对三个教学 crate 跑
  fmt（**禁 `cargo fmt --all`**，kernel 冻结）。

## 5. 推荐：大纲 A（逻辑先行 + `by` 提前 + 归纳拆分 + 关系/读证明收尾）

理由：与现有 7 单元的叙事、双语、reader 预期**迁移成本最低**；同时修掉审计出的全部
问题；`by` 提前正好利用 Infoview/hint 阶梯的即时反馈优势；把"读/评阅/翻译"补上，直接
回应调研里最被低估的"迁移失败"障碍；`Or/Iff/关系` 补齐让"逻辑先行"名实相符。

### 5.1 相对现状的具体改动（最小可执行集）

1. **[动] `by` 提前**：U6 → 新单元 4（在 3 之后、宇宙前）。改动：`course.json` 顺序、
   两个 GOLDEN 数组顺序、`course.rs` 顺序断言、各单元自述文案与依赖措辞。
2. **[拆] U5 → 两个单元**（归纳与递归 Ⅰ/Ⅱ）：golden `(13,10,7)` 拆成两行（例如
   `(7,5,4)` + `(6,5,3)`，以实际事件重算），新文件 `unit6-...`+`unit7-...` 重编号与
   双语镜像、solutions（各一份）。
3. **[新] 关系与联结词**（`Or` 含自写 `Or.rec`、`↔` 定义、`≤`/`Even` 归纳定义、
   inversion 引理）：新文件 + 双语 + solution + GOLDEN 行。
4. **[新] 读证明与综合**（自解释三问、formal↔informal 互译、Proof Evaluations、小项目）：
   新文件（可含零 `sorry` 的"阅读题"，用注释 + `#check`/`#print` 承载；练习以"评测/改写"
   为主，需要新的事件口径时**先改协议与测试**）。
5. **[修] 现有单元**：U3 两题去歧义（给定具体返回或改成"证明 × 的性质"）；删/改
   `by_ex5`；把"散文里给答案"的句子改成"只给触发条件"（对照 teacher skill 的 hint 铁律）；
   删 U5 过期断言；U4 补"读 `#check` 输出"练习。
6. **[修] 文档漂移**：`teaching-session.md`（§3 编号、§5 的 `Or.rec/or_comm` 断言）、
   `course-status.md` §4 golden、`ROADMAP.md` I7、`course/README.md`、`course-bilingual.md`。
7. **[加测] solutions 一致性**：新增 `solution_covers_every_canvas_exercise`（画布练习名 ⊂
   solution 声明名）+ CN/EN solution 事件计数对比（顺带修 EN unit4）。

### 5.2 单元级"练习类型"配额（每单元 ≥3 种，且必须有 R/X 之一）

| 类型 | 含义 | 覆盖单元（大纲 A） |
|---|---|---|
| T | term 填空（`sorry` 在值尾/spine/直接实参） | 1–9 |
| B | `by`/tactic 填空 | 4,6,7,8,9 |
| L | 自证引理（消去子/inversion/symm-trans-cong） | 2,6,7,8,9 |
| R | 读/评阅（读输出、找错、给候选证明判优劣） | 1,3,5,9,10 |
| X | formal↔informal 互译（同一命题写两份） | 7,10 |

每单元练习量目标 5–8，**星级** ★/★★ 标难度，`-- soko:hint` 保持三段（思路/目标形态/
关键件）但"关键件"只写**触发条件**与**该用哪条引理**，不写完整项。

## 6. 落地阶段（每阶段独立过 gate 且可发布）

- **P1 修补（不改结构）**：§5.1 的 5/6/7（去歧义题、删重复题、hint 不泄题、删过期断言、
  文档漂移、solutions 一致性测试 + EN unit4 修复）。golden 若变动则同步两处。
- **P2 重排 + 拆分**：`by` 提前；U5 拆两单元；重编号与双语/solutions 同步；两个 GOLDEN、
  `course.json`、顺序断言、汇总同步；skill/VS Code 门面同步。
- **P3 新增两单元**：关系与联结词、读证明与综合（含可能需要的协议/事件扩展 → 先设计）。
- **P4（可选）并列入口**：把大纲 B 做成"游戏线"（若决定做，需要 Lean4Game 式关卡元数据
  的轻量版与 Infoview 配合；另立设计）。

每阶段收尾：`sokonanoda gate` + 更新 `STATUS.md`/`REQUIREMENTS.md §9`/`docs/teaching-session.md`/
`skills/*`；版本按 minor bump（内容可见变化）并发布。

## 7. 风险与取舍

| 风险 | 取舍 |
|---|---|
| 新单元"读证明/评阅"缺少判分机制（kernel 判类型不判意图） | P3 前先设计：用"多选/改写 + `#check`/`#print` 证据"承载，或引入新事件（需协议 + 测试） |
| 拆 U5 会牵动双语/solutions/GOLDEN/skill 五处 | 一次改完、一次过 gate；拆分方案先按实际事件重算再落表 |
| `by` 提前后 U1–U3 的"term 优先"叙事要调整 | 保留 term 写法（第 4 单元讲 `by` 时对照"term 版 vs tactic 版"），不推翻前 3 单元 |
| 大纲 B/C 与本仓库叙事冲突 | 只作为并列入口/进阶线（P4/另立），不动主线 |
| 无 `rw`/自动化 → 等式单元偏"手搓" | 用 PLFA 等式接力 + 精选引理库 + teacher 只给"哪条引理"的 hint；把"手搓"当教学卖点写进 README |

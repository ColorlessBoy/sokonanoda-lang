# 当前状态与进度日志（agents 先读这里）

> 快照：2026-09-19（第一百〇六轮：**0.59.0 收尾**——语言线五刀（签名受检 /
> 构造子命名空间 / 派生 recursor 判据 / L1 prelude / 用户自定义记法）收成一个版本，
> 课程门禁接进 `scripts/soko gate` 与 CI，卷 I 上站点；版本 **0.59.0**，
> 发布由 push main → auto-tag 全自动；缺口台账门禁（`gap.py selftest` + `check`）
> 同轮接进 gate 与 CI，24 条缺口 **18 条 `fixed_in = 0.59.0`、`check` 全绿**；
> WO-010（诊断坐标）与 P4（课程跟随 prelude）同日落地）
> 仓库：`sokonanoda-lang`；权威计划 = `ROADMAP.md`；**用户要求总账 = `REQUIREMENTS.md`（先读）**；
> **文档地图 = `docs/README.md`**（入口/权威在仓库根，开发者参考在 `docs/` 顶层，
> 设计在 `docs/design/`，调研笔记在 `docs/notes/`）；
> 架构/内核 = `docs/architecture.md`；协议 = `docs/protocol.md`；测试地图 = `docs/TESTING.md`；
> 经验台账 = `docs/LESSONS.md`；CI 失败台账 = `docs/CI-FAILURES.md`；发布 = `docs/RELEASE.md`；
> agent 入口 = `AGENTS.md` + `skills/`；**harness 适配 = `docs/design/deepseek-harness.md`**；
> **agent 查询通道 = `docs/design/agent-query-channel.md`**（ROADMAP I15）。

## 一句话

`.sokonanoda` = **纯声明式教学文件（无 `#` 命令）+ 完整 sokonanoda 内核 + LSP 反馈通道**。
练习 = 带 `sorry` 洞的 `def name : T` / `theorem name : T` / `example : T` 声明。
CLI/REPL 的 `#check` 等只是调试/自测工具，不是文件格式。

## 本轮进度（2026-09-21，第一百一十八轮：用户四条指令落地——报错质量 / 写进教学 / 速查表 / 站点交接文档）

> 用户在同一轮给了四条指令：①「你改好吧，问题我没看懂」（= 上一轮报的 `Set.mem a A`
> 报错质量问题）；②「写进教学」（= 把删骨架后新解锁的 tactic 教进课程）；③「修改」
> （= 记法对照页按 C1.5 重定位）；④「不要动 site，但你实现的功能特性都写到开发文档里，
> 不要让 site agent 搞不懂」。

1. **① 报错质量：修了一半 + 如实登记另一半（G-21）**。
   - **已修（声明位）**：`error.rs` 新增 `classify_term_in_type_position`——识别内核
     「`expected: Sort(n) | actual: $k`」（**项落在类型位**）这个形状，把它从泛化的
     `kernel-rejected` 归到 `kernel-expected-sort`，并把那条 hint 改写成**指根因**：
     「点名调用漏了前导类型参数（`Set.mem a A` 应为 `Set.mem α a A`），或直接用记法
     `a ∈ A` 让它自动补 `α`」。测试：既有的内核消息分类表加两条（新形状 + 一条对照，
     确认 L-06 的 Prop-not-cumulative 没被吞）。
   - **同轮重钉了「护城河」测试**（设计早预告过这一步）：`notation.rs::the_pointful_spelling_keeps_working_and_the_moat_holds`
     以前钉 `code == "kernel-rejected"`，现在钉 `kernel-expected-sort` **并新增一条断言**——
     hint 必须同时说出「前导类型参数」与记法出路。**护城河本身没变**（省略 `α` 仍判红、
     仍在 kernel 阶段、仍是同一条声明），变精确的只是诊断码与提示。
   - **仍欠（`by` 路径）**：`… : Set.mem a A -> A a := by intro h; exact h` 还是报
     「期望 `A a`，实际是 `Set.mem a A`」这种**同形**对照。根因查明：**`by` 块跑的
     时候声明签名还没被内核检查过**（`open_signature` 只用 axiom 探针、且只在值位是洞
     时才走）。修法是「签名检查前移到 `by` 之前」，但那要确认探针不进声明表 + 不为每条
     `by` 声明付额外 elaborate，**不在本轮预算内**，故如实登记。
   - 台账 **G-21**（`kind: language`、`severity: painful`、`status: open`）+ 自断言
     repro `docs/gaps/repro/G21-omitted-type-argument.{sokonanoda,sh}`：
     脚本同时断言两半，②一旦修好就转 exit 1、`gap.py check` 会提醒关账。
     `python3 scripts/gap.py check` → **全部与台账一致** ✓。
2. **② 写进教学：单元④ 从六条 tactic 扩到八条**（subagent 执行 + 我复核）。
   - 新增 `demo_by_constructor`（`∧` 目标上 `constructor` 拆两子目标）与
     `demo_by_cases`（`∨` 假设上 `cases h with | inl … | inr …`），开头说明改成
     **八条**并写明 `left`/`right`/`use` 与 `constructor` 同族；删掉已不成立的
     「其余 tactic 随后面的单元解锁」。练习**没加**（现成的 `by_ex4`/`by_ex6` 已能吃下
     这两个 tactic；历史上专门删过重复练习），编号无跳号、解答逐名覆盖。
   - 复核：4 个文件 rc=0、诊断 0、**CN/EN 剥注释后逐字节相同**；计数
     画布 `(9,6,0)`、解答 `(15,0,0)`；四处钉子重钉（`course.rs` GOLDEN、
     `course_status.rs` 逐单元 + summary **56/66**、`cli.rs` checked 56）→
     `course`/`course_status`/`course_shared`/`cli` **114 条全绿**。
   - **subagent 挖到一条真边界（值得记）**：`use` 要求目标是**真归纳**，而单元⑧ 的
     `∃` 是那里自己声明的 **axiom** ⇒ `use 0` 在单元⑧ 判红（报错原文进了交付）。
     它没有照我的字面要求写"use 在这里可用"，而是写成实情——**这是对的做法**。
     （把单元⑧ 的 `Exists` 改成 `inductive` 就能解锁 `use`，且更贴近 Lean；
     但那是与 C2.5 同族的新一刀，**留给下一轮拍板**。）
3. **③ 记法对照页重定位成速查表**（subagent 执行）：页头补一张**全符号速查表**
   （逻辑连接符内建 + 集合论符号的记法→点名→声明形状→优先级梯子）+ 三条使用规则
   （记法是源级糖 / 点名形式永久可用 / 记法自动补前导类型参数），9 对演示与 3 道练习
   **保留**（它们是"两种写法同判"的证据），「本页的定位」改成"参考页不是单元"。
   **计数不变**：画布 18 checked / 3 open、解答 21 checked / 0 open（只动注释与版式）。
4. **④ 站点交接文档（不碰 `site/`，也不进 site-rebuild 的地盘）**：新建
   `docs/design/lean-style-0.62.md`——给站点/文档 agent 的**事实清单**：12 项用户可见
   特性（记法 / tactic / 隐式实参 / 记法输入 / 判定侧修复 / 新诊断码
   `elab-implicit-argument-unsolved`）、课程内容的事实变化（两门课 + playground 的
   当前计数、C2.5 的后果）、**站点不该误解的三件事**（G-21 半修、记法在实参位的
   `elab-notation-argument-unsolved` 边界、记法对照页的双写法是**故意**的），
   并显式标注「工作树 = 未发布 0.62.0」+ 指向 `site-rebuild/STATE.md` #13 的测量陷阱。
   已挂进 `docs/README.md` 文档地图与 `docs/HANDOVER.md` 的关键文档索引。
5. **仍欠 / 下一轮拍板项**：单元⑧ 的 `Exists` 要不要改 `inductive`（解锁 `use`，
   与 C2.5 同族）；G-21 的 `by` 路径那一半；R2.5 的 IA-2/IA-3；`judge_infer` 的宇宙参数。

## 本轮进度（2026-09-21，第一百一十七轮：收尾三查——占位、hint 词汇、护城河话术）

> C2.5 落地后，按计划的「收尾同步」逐项**复查**（每项都先量规模再决定做不做）。

1. **练习占位再查（D3 / C1.2 的最后缺口）**：全课程扫「值位不是 `by` 形式的 `sorry`」，
   发现**卷 I 单元⑧ 还有 9 处**是 `:=` 换行 `sorry`（R2 那一刀漏掉的），
   已改成 `:= by` + 续行 `sorry`（**不动行数/span**）；改完该文件仍 rc=0、
   `15 checked / 9 open` 逐项不变。**现在两门课的占位一律是 `by` 形式**。
   剩下的 13 处「项模式 `example`/`theorem`」经复核**都是画布的项模式演示**
   （设计 C2.2 有意保留：`example : True ∨ False := Or.inl …`、`example : Sort 1 := Nat`…），
   不是漏改。
2. **C1.3（hint 词汇）实测后判定"已满足、无需改"**：全卷 I 扫「教项模式写法的 hint」
   （`fun (` / `:= fun` 等）只有 **3 条**，逐条看下来**都不是证明写法**——它们是在
   指名一个 **lambda 项**（对角线集的定义 `D := C' \ (fun y => f y y)`、`Eq.subst` 的
   motive、目标的字面形状 `x ∈ (fun y => ¬ f y y)`），属于设计要求的「关键件」。
   同期统计：**92 条 hint 本来就用 tactic 词汇**。⇒ 设计表里「294 条 hint 要换词汇」
   这一项的**真实规模远小于当时的估计**，本轮判定关闭（证据在上）。
3. **C1.5 顺带发现一处话术过期 + 一条报错质量退化**：记法对照页的「护城河」一节写
   `Set.mem a A ← 仍被内核拒绝（kernel-rejected）：缺 α`。**实测（0.62.0）**：
   - `Set.mem α a A -> A a` ✅、`a ∈ A -> A a` ✅、**`Set.mem a A -> A a` ❌** ——
     结论仍成立（省略 `α` 不可用），但**症状变了**：不再报「缺 `α`」，而是在**更晚**处
     炸成一条**同形**的不匹配：`` `exact` 类型不匹配：期望 `A a`，实际是 `Set.mem a A` ``。
   - 这是 **IA-1（隐式实参路线 C）之后的报错质量退化**：根因信息从"指出缺哪个参数"
     变成"两个几乎同名的类型不符"。**结论不变、教学话术不用改，但错误信息应该修**
     —— 记为「仍欠」，**尚未进 `gap.py` 台账**（登记要配 repro 脚本并过
     `gap.py check`，留到下一轮与修复一起做）。
   - 页面已改成**实测口径**（写明 0.62.0 的报错形态 + 待登记）。
4. **验证**：改后 `notation-cheatsheet` 仍 rc=0（18 checked / 3 open 不变）；
   卷 I 门禁 328/99/0；`scripts/soko gate` 见日志（本轮末尾启动）。
5. **仍欠**：`Set.mem a A` 的报错质量（上面第 3 条，需登记台账 + 修）；
   C1.5 的「重定位成速查表」是**内容改动**（本页现在同时承担"对照"与"参考"两个角色，
   要不要拆成两页、或把 9 对演示压成一张速查表，是教学决定）；
   「删骨架后要不要把 `constructor`/`cases` 写进 ①④⑧ 的教学」（上一轮留的教学决定）；
   R2.5 的 IA-2/IA-3；`judge_infer` 的宇宙参数。

## 本轮进度（2026-09-21，第一百一十六轮：**C2.5 落地**——用户拍板删自建骨架，入门课统一到 prelude 真归纳）

> 第 115 轮把 C2.5 摆到用户面前（设计里标「需拍板，推荐删」），用户选**删**。
> 本轮把它做完：入门课 + playground + `unit11-project` 里的自建 `And`/`Or` 骨架
> **全部删除**，prelude 的真归纳接管 ⇒ `constructor`/`cases`/`left`/`right` 在
> **全课程**可用（在此之前 ①④⑧ 因为自建公理而不可用）。

1. **删了什么（逐字）**：①④⑧ 的 `axiom And`(4) + `axiom Or`(3)、⑨⑩⑪ 的
   `axiom And`(4) + **`inductive Or … end` 整块**、`unit11-project/Logic` 的
   `axiom And`(4)、`playground` 的 And/Or 共 7 条。**保留** `axiom True`/`False`
   ——单元① 仍拿它们讲「`axiom` 是什么」（设计 §C2.5 明写保留）。
   影响 **34 个文件**（11 单元 × 中英 × 画布/解答的相应部分 + 项目 4 文件 + playground）。
2. **叙事同轮改**（否则立刻变假话）：单元① 的「逻辑骨架」段改成「这四条用 `axiom`
   是给你看公理长什么样；`∧ ∨` 及其构造子 **prelude 自带**」；单元⑨ 的
   「9.1 `Or`：从公理升级为真归纳」整节动机失效 ⇒ 改成「**`Or` 的消去子**：
   一份 `A ∨ B` 的证据怎么用」；单元⑪ 的「单元① 的 `Or` 只是公理」对比段换掉；
   `playground` 的「公理都齐了」「看 `axiom Or.inl` 的类型」等悬空引用一并修好。
3. **规范副本随之退役**：`course/shared/{And,Or}.sokonanoda` 两个模块**删除**
   （没有副本可守了），`course_shared.rs` 的 `AND_COPIES`(25 份)/`OR_COPIES`(12 份)
   两张表与文件头口径同步删除；`Nat` 那 8 份照旧守。`Demo.sokonanoda` 改成
   **只 import `Nat`**，And/Or 两条演示改用 **prelude 的真归纳**写（演示名不变，
   所以 CI 断言不变）；顺带暴露一处真话：项位的裸名 `inr` 在真归纳上不存在
   （G-02 起的构造子命名空间）⇒ 必须 `Or.inr`，**模式位** `| inl a =>` 仍可用。
4. **计数重钉（内核实测，不手算）**：六个画布的 `checked` 各自减去删掉的声明数
   （unit1 13→6、unit4 14→7、unit8 14→10、unit9 13→8、unit10 7→2、unit11 7→2），
   **`open` 一个没动**（练习声明一行未改）；课程总计 **checked 87→54、open 66 不变**。
   四处钉子同步：`course.rs` 的 `GOLDEN`(6 行)、`course_status.rs` 的逐单元表 +
   summary、`cli.rs` 的 warm-cache 总计。
5. **验证（本轮实测）**：
   - **34 个文件逐个 `grade`：exit 0、诊断 0**（`unit11-project` 四个与 playground 也在内）；
   - `cargo test -p sokonanoda-cli --test course --test course_status --test course_shared --test cli` **114 条全绿**；
   - **CN/EN 22 对文件剥注释后逐字节一致**（含 ⑨⑩⑪ 与项目文件）；
   - 附带证据（子 agent 在仓库外做的探针）：只声明 `True`/`False` 的文件里
     `constructor`/`left`/`right`/`cases` 现在都能过 —— 这正是删骨架的目的。
   - `scripts/soko gate` 见下一轮记录（本轮末尾已启动）。
6. **分工与复核**：两个 subagent 分别做 ①④⑧ 与 ⑨⑩⑪+项目；我**逐个复核**（不采信
   自述）并负责 `shared/` 退役、四处计数重钉、Demo 改造与文档同步。⑨⑩⑪ 那组的
   报告还没到，但它改的文件我已复验（grade 全绿、叙事已改、CN/EN 一致）。
7. **仍欠**：C1.3 卷 I 的 hint 词汇；C1.5 速查表重定位；R2.5 的 IA-2/IA-3；
   `judge_infer` 的宇宙参数；以及「删骨架之后要不要把 `constructor`/`cases` 写进
   ①④⑧ 的教学」——那是**下一轮的教学决定**（设计 §C2.6）。

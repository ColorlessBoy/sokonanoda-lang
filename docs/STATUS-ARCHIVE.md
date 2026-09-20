# STATUS 归档（第 1–104 轮，2026-09-06 → 2026-09-19；另收 0.56.2 线的第九十一轮续
# 与第 116 轮）

> 本文件是 `STATUS.md` 的历史轮次归档——STATUS 只保留最近 3 轮，更早的进度
> 原文移到这里（一字未改，含轮次编号的历史重号）。查某轮做了什么、某缺陷
> 何时修的，先到这里 grep。当前进度仍以 `STATUS.md` 为准。

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

## 本轮进度（2026-09-19，第一百〇三轮（语言线）：WO-006 / G-03 落地 —— 派生 recursor 的宇宙参数判据镜像内核）

> 台账 blocker：`inductive Bar (A : Type) : Prop` + `ctor mk (a : A) : Bar A`
> （= `Exists` 的形状）被内核断言拒绝（`left: 1 / right: 0`），于是 `Exists` 只能立成
> **公理三件套**。工作单 `docs/gaps/WO-006-prop-type-param-inductive.md`，
> 设计 `docs/design/prop-large-elim-mirror.md`。

1. **根因**：前端 `derive_recursor` 的 `small_elim = is_prop_block_ty(ty) &&
   constructors.len() > 1` 是**源码近似**，内核 `large_elim_test` 的真值是
   「非 Prop ⇒ large；空块 ⇒ large；多构造子 ⇒ small；**单构造子 ⇒
   `large_elim_test_aux`**」。单构造子 Prop 块上两者不一致 ⇒ 前端声明 1 个宇宙参数、
   内核要 0 个 ⇒ `assert_nonnected_recursors_def_eq` → `subst_expr_levels` 的长度
   断言炸成 panic。
2. **修法（策略 A，front-only，内核零改动）**：把「派生 recursor」推迟到构造子类型
   elaborate **之后**（此时内核 Pi 望远镜在手），判据 `large_elim_test_mirror` +
   `large_elim_test_aux_mirror` 逐字镜像内核那几行；「字段类型是不是 Prop 值」这步
   交给**真内核**（`judge_infer` 问排序，与 `match` 判 motive 层级同一个 oracle）——
   因为它是**语义**问题：`P -> Q`、`forall (x : Nat), P`、具名 `def … : Prop` 都是
   Prop 值，源码近似会把 P10/P11/P13/P14 从 1 个宇宙参数改成 0 个。
   `is_k_target` / `is_prop_block_ty` 一行未动。
3. **顺带修掉一个既有 oracle bug**：`judge_infer` 取事件流里**第一条** `TypeChecked`，
   而它的查询是**最后一条**命令——前缀里只要已有一条 `#check`（课程/playground 常见），
   拿回的就是旧答案。实测：前缀有 `#check Nat` 时 P10/P13 立刻退化成
   `left:0/right:1`。改为按 `event_cmds` 取**最后一条命令**的事件（不做文本比对）。
   该 bug 同样影响 `match` 的 motive 层级查询。
4. **测试三层**：front 单测 4 条（复现形状 + 表驱动 P1–P14 语料 16/16 +
   判别性成对 P3/P9 与 P10/P11/P13/P14 不许回归 + 0 级 recursor 的 `match` 真算）+
   CLI e2e 3 条（形状断言 `motive : Bar A -> Prop`、`match` 真跑、`Bar.rec.{0}` 必须
   arity 报错）+ 课程用例（卷 I 12 单元 · 34 目标 · **315 checked · 96 open** · 0 判负，
   与修前**同数**——WO 正文写的 296/93 是陈旧口径，实测基线见
   `docs/design/ctor-namespace.md:73`）。复现件 `docs/gaps/repro/G03-…sokonanoda` 重写成修后形状
   （**块本身逐字不动**），`scripts/soko grade` 退出码 0。
5. **影响面 / golden**：`course/`（入门课）两份 GOLDEN **未变**（checked/open/reduced
   与 failed 全同）——本 WO 只让"今天必定失败"的输入变绿，不改任何已通过块的
   recursor 形状。`cargo test --workspace` 全绿。
6. **课程侧出口（下一轮，本轮明确不碰课程）**：
   `courses/set-theory/lib/Exists.sokonanoda:68-71` 的公理三件套可以升级成真归纳
   （`Exists` + `intro` 构造子 + 自动派生的 `Exists.rec`），名字与签名逐字不变 ⇒
   `units/unit06…unit12` 的证明文本预期 0 改动。前置：**G-02**（已落地，构造子
   `Exists.intro` 的规范名）与**依赖消去**（`Exists.rec` 的 motive 要能提到证人，
   见 `lib/Exists.sokonanoda:27-29` 的清单）。

## 本轮进度（2026-09-19，第一百〇二轮（语言线）：WO-005 / G-02 落地 —— 构造子进入类型的命名空间）

> 台账 blocker：构造子以**裸名**进环境且**全项目唯一**——`ctor mk` 之后只有裸 `mk`、
> `Pair.mk` 报 unknown identifier，两个块各写 `ctor mk` 直接撞
> `duplicate declaration mk`，于是 `Prod`/`Subtype`/`Exists` 这种大库只能给构造子起
> `prod_mk`/`exists_intro` 这类**假唯一名**，与真实 Lean 4 写法脱节（违反硬规则 3）。
> 工作单 `docs/gaps/WO-005-ctor-namespace.md`，设计 `docs/design/ctor-namespace.md`。

1. **R1 规范名**：源 `ctor mk` 进内核叫 `Ind.mk`（源名**已含点**则原样 ⇒ prelude 的
   `Nat.zero`/`Bool.true` 与既有显式写法零改动）。规范名是**唯一声明身份**：进
   `Declar::Constructor`、`all_ctor_names`、派生 recursor 的 minor 项与 iota 规则名
   （内核断言 `rule.ctor_name == ctor.name`，漏改 = 派生的 rec 一律对不上）、
   `top_level_def_spans`（闭包唯一性 + hover/goto 回填 + `import-name-collision`
   的第二道闸）。
2. **R2 裸名别名（子集扩展，只在解析层）**：`known` 的值类型从 `Vec<String>` 换成
   `KnownName::{Decl, Alias, Ambiguous}`；唯一裸名解析到**规范名**（绝不造第二个内核
   常量），重复裸名报新码 **`elab-ambiguous-ctor-alias`**，真实声明优先于别名。
   别名是**闭包级扁平**的（与 `check_name_collisions` 同口径；模块作用域属 G-05）。
3. **R3 源级写法不动**：`match` 分支（`| none =>`）与显式 `iota zero :=` 继续按
   **源名**匹配（`ctor_index` 两种拼写都认）；注释/文案不动。
4. **高风险实测（照 WO 要求先测后改）**：源文件自带 `inductive Nat` 时，ctor 叫
   `Nat.succ` 会命中内核 name cache 的 `NatRed::Succ` 快路径 ⇒ 归约从
   `succ (succ (succ (succ zero)))` 变成**混合表示** `Nat.succ (Nat.succ (Nat.succ 1))`
   （`apply` 把一元链折成 `NatLit`）；`Bool`/`Color` 是纯改名（`Bool.ff`/`Color.green`）；
   `Option.some` 场景不变。**逐条实测后重钉 19 处文本断言，未做机械替换**
   （表见设计文档 §2.1）。
5. **顺带照出一条 WO 范围表没写的漏项**：`references.rs::decl_name_span` 按**全名**
   匹配定义 token，而 ctor 在源里写的是裸名 ⇒ 不补回退，ctor 的
   prepareRename/goto/references 会**静默失效**（`ctor mk` 里找不到 `Wrap.mk`）。
   已补"回退匹配最后一段"（仍是词法精确匹配）+ front 回归 1 条 + LSP 回归 3 条。
6. **三层测试**：front 单测 12 条（前缀名+共存、别名唯一/歧义、真实声明优先、
   不重复加前缀、派生 rec 的 iota 名、显式 rec 源名对照、两种 match 拼写、
   hover 回填、`#print` 别名、ctor spine 两种拼写、`decl_name_span` 回退）+
   项目级 2 条（两模块各 `ctor mk` **不得**报 `import-name-collision`；跨模块同名裸名
   报歧义）+ CLI e2e 3 条（`cli_ctor_names_are_namespaced`、
   **`cli_grades_the_g02_repro_clean`**（复现件从此是"缺口即测试"）、
   `cli_keeps_bare_ctor_aliases_working`）+ 语义高亮 1 条。
7. **课程内容一字未改**（R2 别名撑住了）：`python3 courses/set-theory/tools/check.py`
   = **34 目标 · 315 checked · 96 open · 0 判负**，入门课双 GOLDEN
   （`course.rs`/`course_status.rs`）**计数未改**。**口径订正**：WO 正文写的
   296/93 是 09-18 的数字，课程之后继续生长；本刀用**同一棵树**把 `crates/` 回退到
   HEAD 重建二进制做对照，跑出同样是 **315/96/0** ⇒ 差额与本刀无关。
8. **复现件更新成"修后期望形状"**：`docs/gaps/repro/G02-ctor-namespace.sokonanoda`
   现在 exit 0 + 4 条 checked + 0 诊断，并把 R1 共存 / R2 歧义（负向断言，注释形态）
   写成契约；`python3 scripts/gap.py check` 里 G-02 如期报
   "已判卷通过 ← 台账写的是「仍有失败」，请更新"（关账前置）。
9. **文档同轮**：新建 `docs/design/ctor-namespace.md`（设计先行）、
   `docs/architecture.md` §4.2/§5.4、`docs/protocol.md`（新错误码）、
   `skills/sokonanoda-teacher`、`editor/vscode/CHANGELOG.md`（0.59.0 条目，版本号由
   主线统一 bump）、`REQUIREMENTS.md` §9、`docs/HANDOVER.md`、本文件。
10. **验收**：`cargo test --workspace --locked` 全绿（front 508 + cli 各套 + lsp 141 +
    kernel 43）、课程门禁绿、`gap.py check` 只有 G-02 如期提醒。**未跑
    `scripts/soko gate`**（仓库约定：主线收尾统一跑）；内核目录**零改动**。

## 本轮进度（2026-09-19，第一百〇一轮（语言线）：WO-004 / G-01 落地 —— 开练习的签名不再免检）

> 台账里最贵的 blocker：值位写 `sorry` 时**签名从来没过类型检查**——`theorem t : 3 :=
> sorry` 报 `exercise.open` + 0 诊断 + exit 0，与"还没做"逐字无法区分；158 条课程
> 练习的签名腐烂对判卷完全不可见（`docs/gaps/WO-004-open-exercise-signature.md`）。
> 修的是**违反硬规则 3** 的行为（填完洞的声明放进官方 Lean 必须合法）。

1. **front：三处吞错点改成"与值位同罪"**。`walk.rs` 的 `def`/`theorem`/`example` 开
   练习路径原来 `elab_expr(…).ok()` 把签名 elaborate 的 `Err` 丢掉、也从不问内核
   "它是不是类型/命题"；现在统一走新助手 `open_signature`：签名先 elaborate
   （`Err` ⇒ 既有失败通道 + `failed_state`，**不**登记 `OpenExercise`），再交给
   内核阶段终审。顺带修掉一个既有小缺陷：签名原来用**空宇宙表** elaborate，
   `{u}` 签名的 `ty_text` 渲染不出来——现在与 checked 路径共用 `make_univ_map`。
2. **内核阶段：两步终审，判据全部走内核**（冻结内核零改动）。① 探针
   `_soko_signature_probe`（同签名的 `Declar::Axiom`，**不入环境**）过
   `try_check_declar_at(ByIndex(env_before))` ⇒ `ensure_sort_v` 的消息族与 checked
   路径同源（`kernel-expected-sort`）；② `theorem` 再用内核公开判据
   `TypeChecker::is_proposition` 问"是不是 Prop"，`false` ⇒ 合成与内核同形的
   `theorem type must be Prop (sort 0): …`（`kernel-theorem-not-prop`）。顺序不能反
   （`is_prop_type` 对非类型会 panic），第 ② 步套 `quiet_catch` 且失败**保守放过**。
   签名不过 ⇒ 声明 `failed`、报一条 diagnostic、**不发** `exercise.open`
   （不做成 warning：warning 不改退出码，课程侧永远发现不了腐烂）。
3. **诊断 span 取签名自身**（`ty.span()`），不照抄内核消息里的 span（G-15）。
4. **三层测试**：front 8 条（坏签名 5 类 + 防修过头的合法边界 + env 不污染 +
   report 状态）；CLI e2e：`protocol.rs` 一对通过/失败边界（`example : Prop -> Prop
   := sorry` exit 0 vs 6 个坏签名变体 exit 1 + code + span + 无 `exercise.open`）、
   `query.rs` 钉 `query check` 与 `--json` 在同一失败形状上仍一致；课程层：真二进制
   全仓对拍。
5. **兼容性实测（本 WO 最大风险）**：**新增诊断 0 条**。入门课 + 卷 I 的 158 条开放
   练习签名本来就合法——用新旧二进制逐条对拍全仓 105 个 `.sokonanoda` 文件，
   唯一变化的只有 G-01 自己的复现件（0→1）；`courses/set-theory/tools/check.py`
   仍是 **315 checked · 96 open · 0 判负**；两处课程 GOLDEN（`course.rs` /
   `course_status.rs`）**未改**（`decl.checked`/`exercise.open`/`expr.reduced` 三列
   逐行不变）。课程级变异实测：`mem_of_subset` 的签名改成拼错引理名 ⇒
   `elab-unknown-identifier` + exit 1；结论改成 `Set α` ⇒ `kernel-theorem-not-prop`
   + exit 1（修前两者都是 6×`exercise.open` + 0 诊断 + exit 0）。
6. **被这刀照出来的既有假绿（同轮修正，均为测试夹具写错，不是新缺口）**：
   `theorem t : Prop := h (g sorry)` 拿**命题本身**当命题的证明、`theorem t :
   Nat -> Nat := …`、`theorem bad : Prop -> Prop := sorry`（`Prop -> Prop` 是
   Type 层的 Pi）、`Or`/`And`/`True`/`False` 等未声明名字——共 14 处测试夹具与
   `cli_axiom_decl_binders_compile` 的一处课程用例改用合法签名（`def` 或真命题
   `P`/`Eq.{1} Nat 0 0`）。**没有一条是"放宽检查"**：改的是夹具，判据一个字没动。
7. **复现件更新成"修后期望形状"**：`docs/gaps/repro/G01-open-exercise-signature.sokonanoda`
   补了修后预期（现在**应该** exit 1，它钉的是契约不是"能过"）；新增课程级变异体
   `docs/gaps/repro/G01-course-signature-mutations.sokonanoda`（两种签名腐烂各一条）。
8. **文档同轮**：`docs/protocol.md`（`exercise.open` 的边界 + `kernel-theorem-not-prop`
   适用面 + `redundant-sorry` 对照段）、`skills/sokonanoda-teacher`（判卷只认
   `decl.checked`/`diagnostic`，签名诊断单列一行决策）、`skills/sokonanoda-dev`
   （「签名/命名类」bug 的三层回归范例）、`AGENTS.md` 硬规则 3 补一句、
   `REQUIREMENTS.md` §9（2026-09-19）、`editor/vscode`（README + CHANGELOG 0.59.0
   条目，版本号由主线统一 bump）、`docs/design/teaching-project.md` §6.4 补
   as-built 对照表、WO 的"修好后输出"节。
9. **验收**：`cargo test --workspace --locked` 全绿（front 493 + …、cli、lsp 138、
   kernel 51）、`cargo clippy`（教学 crates）绿、`cargo fmt --check`（教学 crates）绿、
   课程门禁绿、`python3 scripts/gap.py check` 全绿。**未跑 `scripts/soko gate`**
   （仓库约定：主线收尾统一跑）。

## 本轮进度（2026-09-18，第九十九轮（语言线）：缺口台账并行清零中 —— WO-007 / G-06 落地）

- **G-06 / WO-007：`sokonanoda course` 认 `import`（本轮实跑验收）**。有 `import` 的
  单元走项目闭包（与 `grade`/`query check`/`build` 同一份闭包、同一个模块根与
  `ProjectPlan::digest` 摘要键，见 `crates/cli/src/project_cache.rs`）；计数只取
  **入口模块**，`failed` = 闭包内所有模块 `events.errors` 之和（项目级错误由
  `attach_diagnostics` 挂进某个模块）⇒ `failed == 0` ⇔ `grade <单元>` exit 0；
  模块根 = 单元最近的 `sokonanoda.toml`（嵌套子项目优先），没有则回退 `course.json`
  所在目录；清单路径先 `canonicalize` ⇒ 与 cwd 无关。**无 `import` 的单元逐字节
  走单文件** ⇒ 两处 GOLDEN（`crates/cli/tests/course.rs`、`course_status.rs`）未改。
  实测：`node scripts/soko course "$PWD/courses/set-theory/course.json" --json`
  = `{"checked":63,"failed":0,"open":93,"units":12}`（修前 `checked:1 / failed:63`，
  12 单元全假红）；`python3 courses/set-theory/tools/check.py` 仍 exit 0。
  新增 `crates/cli/tests/course_project.rs`（8 例：夹具转绿、与 `grade`/`query check`
  同判、依赖缺失/内核报错不假绿、模块根回退与嵌套优先、相对路径与 cwd 无关、
  闭包摘要与 `build` 共用）；复现 `docs/gaps/repro/G06-course-import.sh`
  从 exit 0 → **exit 1**（行为已变 = `gap.py close G-06` 的前置）。
  文档：`docs/protocol.md` Course map、`docs/design/{imports-and-projects,course-status,
  compile-cache,teaching-project}.md`、`docs/architecture.md` §4.5 消费方、
  `skills/sokonanoda-{teacher,dev}`、`AGENTS.md`、`docs/HANDOVER.md` §G。

## 本轮进度（2026-09-18，第九十八轮：合并 0.56.2 线（多余的 `sorry`）+ push 主线 —— 0.58.0 发布）

> 用户：「你来搞吧，push」——本线（I16 项目管理 → 0.58.0）与 `origin/main`
> （0.56.2 = `redundant-sorry` 线）已经**分叉**：远端 3 个 commit（`dd1902d`
> release 0.56.2 / `f66fee3` 0.56.2 发布文档 / `74cdbda` CI skill），本线 53 个。
> 不能强推（会丢掉 0.56.2 的功能与已发布的 `v0.56.2`），所以**先合并再 push**。

1. **合并过程（scratch worktree，19 个文件冲突逐个手心合并）**：`git worktree add
   /tmp/soko-merge` 从本线 HEAD 建 `i16-merge`，`git merge origin/main`；合并树验证
   全绿后再 `merge --ff-only` 回本线（主线保持线性、无 merge 提交噪音）。
   - **内核（冻结快照）**：0.56.2 只加不改语义（`check_declar_at` /
     `try_check_declar_at` 显式限界入口），`crates/kernel/{tc,util}.rs` 取远端；
     三层回归测试一并并入（`memory_api` 7 → 8）。
   - **前端**：`open_goal` 增第 4 个参数（候选"多余洞" sink，`Vec<Span>`）；
     `PendingOp::OpenExercise` 增 `env_before`（探针终审的可见前缀）+
     `redundant_probes`（pass 1 造、pass 2 查、**不入环境**）。
   - **搬进模块化的树**：0.56.2 写在旧 `check.rs`（1918 行）里的探针代码要手工搬到
     本线的 `check/{walk,kernel_phase}.rs`：`build_redundant_probes` → `walk.rs`，
     终审循环（`try_check_declar_at(…, EnvLimit::ByIndex(env_before))`）+
     warning 装配 → `kernel_phase.rs`；旧 `check.rs` 在合并树里 `git rm`，
     `Cargo.lock` 取本线后 `cargo metadata` 重生成。
2. **合并暴露的一处真 bug（已修 + 已加回归）—— warning 的跨模块归因**：
   `redundant-sorry` 是 pass 2 **现算**的 warning（带命令下标就有归因依据），
   而 `split_report` 原先只按单元**重算语法级** warning ⇒ 项目入口里"多写了一行
   `sorry`"会被静默丢掉（单文件看不出来）。修法：`CompileOutput` 增平行数组
   `warning_cmds` + `push_warning(cmd, w)`（与 `event_cmds` / `error_cmds` 同款
   不变量：两数组严格平行、永不失配），`split_report` 按**命令下标**归因
   （不猜 span——不同文件的 offset 不在同一个坐标空间）；语法级 warning 由
   `kernel_phase` 钉在所属单元的区间上。回归：
   `compile::tests::warnings_are_attributed_to_the_unit_that_produced_them`
   （依赖 2 条 = 语法级 + 内核终审、入口 1 条，span 各落在自己文件的坐标里）。
3. **验收（合并树上真跑）**：`cargo test --workspace --locked` **888 passed /
   0 failed / 6 ignored**（27 个测试目标 + 3 个 doc-test 目标；kernel 52 =
   lib 43 + arena 1 + memory_api 8；front 475 = lib 466 + perf 3 + perf_project 6；
   cli 223 = 单元 5 + 集成 17 个目标 218；lsp 138）；`cargo fmt` / clippy 干净。
   四个纯 Node 套件 18 / 7 / 10 / 11；真 VS Code 例行化（`scripts/vscode-e2e.sh`，
   1.138.0 与 1.106.0 各一轮）**14/14**；`scripts/e2e-merge.py --check`、
   `scripts/check-site.py`、`scripts/soko gate` 全绿。0.56.2 的功能在合并树上逐条
   复验：`redundant-sorry` 正例 / 真缺口反例 / 前瞻引用护栏（front 5 条）、
   `--json` warning 事件、`query goals|holes` 的洞级 `redundant` 标记、
   LSP 不再叠 "not yet solved"。
4. **push 前的 CI 预检（发现并修掉一个 workflow 设计错误）**：原先把三条 e2e 腿
   放在**同一个 job 的矩阵**里、用 job 级 `if: matrix.os != 'macos-latest' || …`
   表达"macOS 只在 main 上跑"——但 GitHub 的 contexts 可用性表里
   `jobs.<job_id>.if` **不含 `matrix`**，这个条件要么按空值求值（macOS 腿在 PR 上也
   跑），要么被判成未识别命名值让**整个 workflow 校验失败**（那样一条 CI 都不会跑）。
   现在拆成两个 job：`e2e`（ubuntu × 2 版本，每个 PR/分支 push）+
   `e2e-macos`（macos × 1.138.0，github-only 条件、只 main）；`auto-tag` 的 needs 与
   `e2e-ledger` 的 needs 同步带上两条。顺带加固 `scripts/e2e-merge.py` 的去重键
   （加 `host.system`/`machine`：ubuntu 与 macos 的 1.138.0 腿同秒完成时不会被当成
   重复条目丢掉）。合并树先推一个**临时预检分支**跑一遍 CI（workflow 校验 +
   ubuntu 两条腿 + 全部其它 job），绿了再 push main——**这一步立刻回本**：
   - 预检确认 workflow 被接受（job 级 `if` 引用 `matrix` 的写法确实不能用），
     `e2e-macos` 在分支 push 上如预期 **skipped**，两条 ubuntu e2e 腿
     （1.138.0 与 1.106.0，含 runner 上现下老版本 VS Code）**全绿**，
     `e2e-ledger` 也如预期只在 main 跑；
   - 但 `test` job 假红：`crates/front/tests/perf.rs` 的
     `check_document_scaling_is_linear` 报 ratio ≥ 12×，而同一棵树本地全量
     `888 passed / 0 failed`。本地复现定位：**并行**（cargo 默认）跑三个 perf 用例时
     400/50 比 = 10.9×，`--test-threads=1` 或单跑该用例 = 7.8×（8× 规模 ⇒ 线性）
     ——算法没回归，是同一个测试二进制里的重活互相抢 CPU 把长的那一档抬高了。
     修法（阈值不动，只改采样口径）：`front/tests/perf.rs` 加**进程内互斥锁串行** +
     **轮转 best-of-N 取最小**，每键延迟改用**中位数 + 最坏值天花板**；
     `lsp/src/tests/perf.rs` 的单文件/项目请求延迟改 **best-of-3**（那 130+ 用例
     并行的 lib 二进制里，10ms 阈值单次采样迟早会红）。台账
     `docs/CI-FAILURES.md`（2026-09-18 条）+ `docs/PERF.md` 采样口径段同步。
5. **发布**：push `main` → `ci.yml` 的 auto-tag 打 `v0.58.0` 并 dispatch
   `release.yml`（8 平台 CLI/LSP tarball + 9 个 VSIX）。`v0.56.2` 的 tag 与其
   功能都保留在历史里，0.58.0 的 CHANGELOG 补记"多余的 `sorry` 已并入"。
6. **发布结果（2026-09-18 实测）**：push `main` → CI **8/8 job 全绿**
   （lint / test / e2e ubuntu×2 / **e2e macos-latest 第一次真跑** / e2e-ledger /
   auto-tag / pages）→ auto-tag 打 **`v0.58.0`** 并 dispatch `release` →
   release **11 job 全 success** → Release **26 资产**（lsp ×8 / cli ×8 / vsix ×9 /
   `SHA256SUMS`）+ Marketplace 收录 **0.58.0**（10:21Z）。**发布产物实测**：
   下载 `sokonanoda-cli-aarch64-apple-darwin.tar.gz` → `shasum -c` **OK** →
   `--version` = 0.58.0 → 对 `playground.sokonanoda` 报出
   `warning[redundant-sorry]`（第 328 行，正是用户最初报的那一行）+
   `query project` 在 `course/unit11-project/` 上给出根与两个 `compiled` 模块。
   `e2e-ledger` 把三条 CI 腿的台账（Linux×2 + Darwin×1，各 14/14、`dirty=false`）
   自动回提交进 `docs/e2e/ledger.jsonl`（共 10 条，随后的 docs push 又追加 3 条）；
   官网进度页已换到第九十八轮。**回提交的副作用已修**：`GITHUB_TOKEN` 推的提交
   不触发 workflow ⇒ 没有 check-run、main 的 HEAD 挂黄点（`Expected — Waiting for
   status to be reported`），看起来像「CI 没成功」；现在 `e2e-ledger` 回提交成功后
   会给新提交补一条 `e2e-ledger` 成功状态（`statuses: write`），并已给已有的两个
   台账提交补上状态。那条临时预检分支上的**真红**（性能哨兵假红）已按
   `docs/CI-FAILURES.md` 修掉、该 run 也已删除；main / release / pages 现在全绿。
7. **文档**：本文件（第九十五轮移入归档 + 0.56.2 线的第九十一轮续一并归档）、
   `docs/STATUS-ARCHIVE.md`、`REQUIREMENTS.md` §9（九十八）、`docs/HANDOVER.md`、
   `docs/TESTING.md`（合并后的测试构成）、`docs/LESSONS.md`、
   `editor/vscode/CHANGELOG.md`、`docs/protocol.md`（warning 码三个并列）、
   `skills/sokonanoda-teacher/references/events.md`、
   `docs/design/deepseek-harness.md`（H3 追加行）、`docs/E2E.md` §5/§7（两个 e2e job 与
   版本升级三处）、`.github/workflows/ci.yml` 注释、`docs/CI-FAILURES.md`、
   `docs/PERF.md`（采样口径）、`scripts/gen-site-data.py`（轮次头解析改成"只认第一条
   + 解析失败即报错"——本轮标题里的全角括号曾让网站 round 静默停在 97）。

## 本轮进度（2026-09-18，第九十五轮：待办批次 3 完成 —— 拆 `run_pass` + 项目整理）

> 用户：「可以，前三个你先做完，把项目理干净」（批次 1/2 已在第九十四轮完成）。
> 本轮 = **批次 3**（`run_pass` ≈1174 行单函数 → 三个模块，三次提交，每刀只动位置）
> + **一轮仓库整理**（死代码、过期文档、模块地图、经验台账、STATUS 归档）。

1. **第一刀 —— 闭包装配件出 `check.rs`**：`SourceUnit` / `unit_ranges` /
   `split_report` / `compile_all_units` → `compile/units.rs`（108 行；单文件也走同一条路径）。
2. **第二刀 —— `run_pass` 尾部出 `check/kernel_phase.rs`**：`builder.finish()` 之后的
   内核 check-then-add + 事件/错误 + 每命令签名与 early cutoff + 报告装配（≈360 行）
   原样搬进 `finish_pass(Walked)`；`check.rs` 1918 → `check/mod.rs` **1522**。
3. **第三刀 —— 命令走查出 `check/walk.rs`**：`Walk`（可变累加器：builder /
   known_universes / inductives / out / ops / cmd_hovers / decl_states / example_idx）、
   `CmdCtx`（每命令派生的 `Cow` 前缀、模板、信任位）、每个 `Command` 变体一个方法；
   arm 里的 `continue` 改 `return`（8 个 arm 都没有内层循环）。最终
   `check/mod.rs` **791** + `walk.rs` **951** + `kernel_phase.rs` **413**；
   单文件仍走 `Cow::Borrowed` 前缀（零新增分配，A1 不变）。
4. **验收（方法论收获）**：除 `cargo test --workspace --locked` **862 passed / 0 failed**
   外，做**二进制对拍**——`git worktree` 取改动前的树，两个 CLI 对同一批输入
   （全部 58 个 `.sokonanoda` + `--root` / `--no-project` / stdin /
   `query check|goals|holes`）输出**逐字节相同**；8 个 arm 另做"逐字符同构"
   （空白无关）比较。方法与两个坑（`cargo fmt` 会重排；**两个 worktree 别共用
   `CARGO_TARGET_DIR`**——后建的树会静默覆盖前者的二进制）写进
   `docs/TESTING.md`「二进制对拍」与 `docs/LESSONS.md`。
5. **整理（死代码）**：删掉只写状态 `built_inductives`（唯一消费者是文件尾的
   `let _ = …`；顺带去掉归纳块每次的无用 `Vec` 克隆）与 `def` 开练习路径里推**空**
   `CmdHover` 的空操作（`resolve_hovers` 只读 `nodes`）——同样过二进制对拍。
6. **整理（性能台账口径）**：复盘台账发现**采样口径**问题——项目层 perf 套件在同一
   测试二进制里**并行**跑，把单次操作成本放大 3–4×（同一份代码：单跑 32.4ms /
   串行 33–38ms / 默认并行 118–152ms；LSP didOpen+按键 12+12ms vs 64+50ms）。
   修法：`scripts/perf-ledger.sh` / `perf-report.sh` 一律 `--test-threads=1`、
   `perf_project` 的分阶段/缩放改 `measure_best(…, 3)`；`docs/PERF.md` 的基线表
   按**串行口径**重写（教学规模 2/3/5 × 12 声明一次按键 **14/16/24ms**，4×20 编译
   32–38ms）并写明"跨口径不可比"；教训进 `docs/LESSONS.md`。**旧台账条目是并行口径，
   比较时先看是否落在 ±25% 内。**
7. **整理（文档）**：HANDOVER 里"项目入口 quick-fix 仍未做"的过期段落更正；§4 新增
   剩余结构债盘点（`compile/tests.rs` 4828 / `elab.rs` 2854 / `parser.rs` 2065 /
   `lsp/lib.rs` 1554 / `vscode/extension.js` 1493，按建议顺序）与
   "开练习的类型子表达式没有 hover 行"（**刻意保留现状**，含补法）；`architecture.md`
   仓库地图 + §4.2 补"阶段 ↔ 模块"对照；`TESTING.md` 新增「二进制对拍」小节 +
   精确测试构成（kernel 51 / front 462 / cli 214 / lsp 135）；本文件归档第九十二轮。
8. **批次 3 完成 ⇒ 待办只剩批次 4**：`soko/project` 项目状态可视化（协议 + VS Code
   状态/树：模块根、清单来源、闭包模块、失败模块）。

## 本轮进度（2026-09-18，第九十四轮：待办批次 1 —— 项目 quick-fix + 编辑器外改动刷新）

> 用户：「还有没有做的TODO吗？fix修复或者优化体验的设计」→ 我列出 A/B/C/D 四组未做项
> 与四个设计 → 用户选「批次 1（推荐）」并要求「按照你的计划，从上到下依次改进」。
> 本轮 = 批次 1（A1 + C3 + B3 与 A2）。

1. **判据前缀抽成真相层（C3）**：`judge.rs` 四个合成判定入口各增 `extra_prefix`
   变体（`judge_terms_with` / `judge_infer_with` / `judge_hole_fill_with` /
   `judge_value_replace_with`；旧签名委托 `""` ⇒ 单文件逐字节不变，缓存键含前缀）。
   `QueryDoc::judge_prefix(offset)` 是唯一真相入口（依赖源码去 `import` 行、拓扑序）；
   `importless_source` 从 `check.rs` 私有函数提成 `project::importless_source` 一份实现。
2. **项目入口恢复 quick-fix（A1）**：`front::suggest_with`、`probe_sub_goal_types_with`
   接前缀，LSP 的 code action 传 `doc.query().judge_prefix(...)`。真 LSP 探针：
   修复前 `null` → 修复后 `refine And.intro a b sorry sorry`（与单文件同形）。
3. **第三层根因**：`run_pass` 的 `GoalTemplates`（refine/intro 的构造子索引）按
   **单个单元**构建 ⇒ 项目入口看不见导入的构造子，建议凭空消失；现在按"拓扑序前缀 +
   本单元"的命令表构建（`new_for` 只读命令表，`src` 是占位）。
4. **项目模式子洞探针（B3）**：`probed_report` 不再因项目模式整段跳过；
   `query goals --probe` 在项目入口给出 `spine_x` 两个子洞期望类型 `a`/`b`（与单文件一致）。
5. **编辑器外改动自动刷新（A2）**：LSP 实现 `workspace/didChangeWatchedFiles`
   （扩展早已声明 `**/*.sokonanoda` watcher，服务端此前静默忽略）：只重编译
   **闭包里含该路径**的已打开文档、缓冲区优先；缺失模块也记着期望路径，所以
   "文件被创建出来"同样触发刷新。真二进制探针：模拟 `git checkout` 改坏依赖 →
   入口立刻报 `elab-unknown-identifier`。
6. **测试**：LSP `code_actions_work_in_a_project_entry`、
   `an_external_change_to_a_dependency_refreshes_the_open_entry`；front
   `project_documents_expose_a_judge_prefix_and_probe_sub_goals`。
   `cargo test --workspace --locked` **860 passed / 0 failed**；项目 perf 复测无回退
   （4×20 compile 131ms、缩放 1.9×、按键 139ms）。
7. **文档**：`TESTING.md` §7b 标闭环（三层根因 + 守护）、多文件 LSP 行扩写；
   架构 §4.5 判据前缀段改写；设计 P7 两项划掉；`vscode-dev-guide` 坑 15 更新；
   本文件与 `REQUIREMENTS.md` §9（九十四）。
8. **批次 2（同轮完成）—— `query` 走项目缓存 + 协议身份回显**：
   - `query` 与 `check`/`build` 共用 `crates/cli/src/project_cache.rs` 的闭包摘要键；
     `QueryDoc::check()` 不再二次编译（复用 `set_text` 存下的 `CompileOutput`，新增
     `set_cached_entry` / `compiled_output`）。3×12 实测：`query check` 冷 49→**25ms**、
     热 37→**3.4ms**；`build --json` 立刻看到入口是同一份键的 hit。
   - `soko/goals` 回显 `uri`+`version`、`soko/stateAt` 回显 `uri`；VS Code 扩展比对后
     丢弃不匹配答案（stub 宿主 8/8），协议写进 `docs/protocol.md`。
   - 测试：CLI `query_uses_the_same_project_cache_as_check_and_build`、LSP
     `custom_responses_echo_the_requested_document_identity`、扩展宿主
     `an answer that names another document is dropped`。
9. **批次 3（同轮起步）**：第一、二刀（`units.rs` + `check/kernel_phase.rs`）同轮完成，
   第三刀与收尾清理见**第九十五轮**。
10. **下一批**：批次 3 余下 → 批次 4（`soko/project` 项目状态可视化）。批次 3 已在
    第九十五轮完成；**批次 4 待做**。

## 本轮进度（2026-09-18，第九十三轮：项目层性能例行化 + 测试扩充 + 编辑器审计修复）

> 用户：「各个环节的性能例行化检测并记录在案，方便后续分析检查。再多增加点项目相关
> 的测试，功能和性能，包括 vscode 前端会不会卡，有没有实现不对的地方。」
> 本轮 = **可复现的性能台账**（分阶段、机器可读、提交进仓库）+ 项目层功能/性能测试
> 扩充 + 对扩展前端做了一次审计并把查出的**两处真 bug** 修掉。

1. **性能例行化（分阶段 + 台账）**：新增 `crates/front/tests/perf_project.rs`
   （plan / digest / compile / 一次按键 / 内存覆盖 / 线性缩放）、
   `crates/cli/tests/perf_project.rs`（冷、热、依赖改动必 miss、`build`+`query`）、
   `crates/lsp/src/tests/perf.rs` 的三条项目例（didOpen / 按键 / 改依赖刷新下游 /
   请求延迟）。每个阶段打印 `PERFJSON`（`schema: soko.perf/1`），
   **`scripts/perf-ledger.sh` → `docs/perf/ledger.jsonl`**（追加式、带
   version/commit/日期/宿主/`cli_profile`）；`docs/perf/latest.json` 便于直读。
   CI 的 "Performance report" 与 `scripts/perf-report.sh` 同步收录这三段。
2. **实测基线（教学规模无感）**：front 4×20 项目 compile 90–110ms、一次按键
   96–123ms、缩放线性（4× 规模 ⇒ 2.4–3.0×）；**教学规模 2/3/5 模块 × 12 声明
   一次按键 12–46ms**；LSP 项目按键 25–49ms 且**每次按键只发 1 份诊断**；
   CLI release 冷 23.5ms / 热 4.4ms；内存覆盖与读盘同价（33.1 vs 33.2ms）。
3. **扩展前端审计（两个真 bug，已修 + 已加回归）**：
   - **切文件竞态**：`loadDeclarations()` 在 `await` 之后读 `this.uri` 建树节点——
     A 的请求、切到 B 之后回来，树上那行的标签是 A 的声明、点击却是
     `revealRange(B, A 的洞)`（stub host 复现）。现在请求发起时钉住 URI、回来先比对。
   - **诊断监听器全窗口且无去抖/去重**：别的扩展（TS/ESLint）报错也会跑一整轮
     `soko/goals`；项目模式一次编辑的事件里会跑 **2 次** goals + 2 次 Infoview
     整表重建。现在按 URI 过滤（只理 `.sokonanoda`）+ 150ms 去抖 + 并发合并 +
     载荷指纹去重。
   - 顺手：课程树缓存一次 CLI 运行（30s TTL，热缓存一次 ~320ms / 11 个单元）、
     `server.js` 下载回退的 `execSync tar` 改 `await execFile`（不再冻结宿主）。
   - **新增测试层** `editor/vscode/test-extension-host.js`（stub 的
     vscode/languageclient/child_process + 假定时器跑真 `extension.js`，7 例，
     零依赖毫秒级），接入 `npm run test:unit`；对着**修复前**的代码 5/7 会红
     （证据）。`crates/cli/tests/extension.rs` 新增契约守住这四个 Node 文件都在册。
4. **项目层功能测试**：新增 `crates/cli/tests/project_features.rs`（11 例：两级嵌套
   模块名、菱形依赖 + 缓存失效、两个入口共享依赖且缓存不串台、依赖解析错误归因、
   文件/目录同名、import 位置与形态错误、`build` 逐文件状态、嵌套项目的
   `query` 计数一致、入口拒绝退出 1 而依赖 `sorry` 退出 0）。
5. **顺手修掉的缺陷**：`import my-lib` 的报错文案把横线写了两次（`my--…` →
   `my-…`，front token 层 + 单测）；`docs/protocol.md` 的人类输出口径写成
   `error[<code>]`，实际是 `error[<stage>]`；设计 §6 A2 承诺的"依赖 `decl.checked`
   事件带 module"与实现不符——按实现改口径（只输出入口事件，依赖的问题走诊断，
   §5.1 偏差④）。
6. **测试与门禁**：`cargo test --workspace --locked` 全绿（front 450+ / LSP 130+ /
   CLI 200+，含新增 4 个 perf 例、11 个功能例、7 个宿主例）；
   `node editor/vscode/test-extension-host.js` 7/7；`scripts/soko gate` PASS。
7. **用户第二轮追加：真跑一遍性能 + 教学内容 import 化**。性能：`scripts/perf-ledger.sh`
   11 条记录（front 4×20 compile 111ms / 按键 130ms；缩放 2.8×；LSP 项目按键 46ms
   且 1 份诊断；CLI release 冷 31.9 / 热 3.3 / 依赖改动后 27.7ms；新增"判据前缀"
   一条：入口 10 处 `match` 导入归纳类型 82.6ms）。教学内容：
   - 先量了一遍：44 个语料文件里**逐字重复**的声明块只有 232/2974 行（8%），
     And 公理 24 份、`Or` 块 12 份、显式 `Nat` 块 8 份（含中英与解答钥匙）。
     结论：**整包 import 化不划算**（会打破"单元自给自足"、golden/镜像/课程树契约
     全要重钉），但复制粘贴的漂移风险是真的。
   - 于是新增 **`course/shared/` 子项目**（`sokonanoda.toml` + 规范模块
     `And`/`Or`/`Nat` + 自检入口 `Demo.sokonanoda`，真的 import 并判卷），
     配 `crates/cli/tests/course_shared.rs` 的**双向漂移守护**（少了=副本没跟上、
     多了=抄了没登记、画布出现 `import` 也红）。画布一行未改，golden 零漂移。
   - **过程中挖出并修掉两个真 bug（同一根因）**：`match` 的宇宙层级、`by` tactic 的
     `apply`/`exact` 都靠 `judge_infer(prefix_src, …)` 合成前缀文件问内核，而项目
     模式的前缀只含**入口自己**的源码 ⇒ 入口里 `match` 被导入的归纳类型报
     `elab-match-no-expected-type`、`by apply And.intro`（导入的公理）报
     `elab-tactic-failed: unknown identifier`。修法：`run_pass` 按拓扑序预计算
     `closure_prefixes`（依赖源码去掉 `import` 行后相接 + 本文件前缀），
     单文件模式不构造（A1 逐字节不变）。两条回归测试入 `project/tests.rs`。
   - **发现但未修（已登记，P5 余项）**：编辑器 quick-fix 的 `front::suggest` 也只吃
     入口文本 ⇒ 项目入口里对导入名字给不出建议（同一文件放进单文件就有
     `refine And.intro …`，放进项目入口是 `null`；真 LSP 探针复现）。
     记在 `docs/TESTING.md` §7b 与 `docs/design/imports-and-projects.md` P7。
8. **用户第三轮提问：单文件与项目文件能自动区分吗（单文件不找项目配置、像脚本一样跑）**
   ——是，规则写进设计文档 **§4.4b** 并由 `crates/cli/tests/single_file_vs_project.rs`
   四条测试钉住：① 无 `import` 的文件**从不读 `sokonanoda.toml`**（同目录坏清单、
   `--root`、`--no-project` 全是空操作）；② 同一个坏清单在有 `import` 的文件上必须报
   `manifest-invalid`，但仍以入口目录把闭包编完；③ **依赖自己的清单永不参与**；
   ④ 零配置能 import、stdin 与文件逐字节一致、stdin 带 `import` 给出 `--root` 提示。
   **同时修掉一个真 bug（编辑器与 CLI 不一致）**：LSP 把 `initialize` 的**工作区根**
   当模块根传下去（等于跳过清单发现），于是 VS Code 打开仓库根、再打开
   `course/unit11-project/Canvas.sokonanoda` 会报 `import-not-found`，而同一文件在
   CLI 下正常。现在编辑器与 CLI 同一套发现规则（最近清单 → 入口目录），
   回归测试 `a_nested_project_resolves_against_its_own_manifest`。

## 本轮进度（2026-09-18，第九十二轮：I16 落地 —— `import` 闭包 + 项目管理，0.57.0）

> 用户：「新产生一个 git 分支吧，全部按照建议，你给我完整做完一版我看看。这个变化比较大。」
> 分支 **`i16-imports-and-projects`**；设计文档 §8 的 Q1–Q7 **全部按推荐执行**；
> P0–P6 全部落地（P7 = backlog）。设计 + as-built =
> **`docs/design/imports-and-projects.md`**（§5.1 有三处与设计的偏差与 P5 的实现选择）。

1. **语法与解析（P1）**：`Command::Import`（AST + token）、置顶校验、模块名合法性
   （`import 1Foo`/尾点/`-` 都被拒，`-` 给教学 hint）；3 个 parse 期错误码
   `import-malformed` / `import-not-a-valid-module-name` /
   `import-must-precede-declarations`。`import` 进 `is_reserved_command`，
   否则行首 `import` 会被当作应用实参吞掉。
2. **闭包编译（P2，核心）**：`crates/front/src/project/`（`mod`/`module_name`/`resolve`/
   `manifest`/`graph`/`report` 六文件，18 单测）。`plan_project` 定根（`--root` >
   最近 `sokonanoda.toml`（上溯止于 `.git`/HOME）> 入口目录——**无清单也能 import**，
   对真 Lean 的有意分歧）；后序 DFS 装载拓扑序（`VisitOutcome::Cycle` 保证入口最后）；
   `compile_all_units` 在**同一个 arena + 同一个 `EnvBuilder`** 按序跑完，
   import 命令先入环境 ⇒ `EnvLimit` 下标不变，**内核一行未改**。诊断**按命令下标**
   （`CompileOutput.error_cmds`）归属文件，`split_report` 还原每文件报告与事件；
   闭包级重名/prelude 冲突/依赖阻断（`import-dependency-failed` 只报一条）。
3. **CLI 与协议（P3）**：`--root` / `--no-project`、`build` 项目化、`query` 闭包内求值、
   `help` 增「multi-file projects」段；`docs/protocol.md` 补全部新码 + warning
   `import-has-open-exercises`；新增 `crates/cli/tests/imports.rs`（**12 条 e2e**，
   含 A1：无 import 文件与单文件路径逐字节一致）。
4. **缓存（P4）**：`ProjectPlan::digest(options)` = 拓扑序上每个模块 (名字, 源, imports)
   + prelude 模式的稳定哈希；`CACHE_FORMAT` 1→2；依赖改动必然 miss（e2e 实测）。
5. **LSP（P5，全做完）**：`Docs{map,order,root,active}` 多文档、`initialize` 捕获
   root、按 URI publish、`did_close` 清理、跨文件 `goto_definition` / `references` /
   `rename`（新 `project_refs.rs`：跨文件身份 = 名字、定义名 token 来自
   `front::references`、编辑按模块分组、改名成项目里已有名字先被拦下）；项目模式下
   补挂 `-- soko:hint` 阶梯（否则带 import 的入口答不出 hints）。**跨文件失效**：
   改依赖 ⇒ 含它的打开文档用"内存覆盖"（`load_closure_with_overlay`，按
   `canonicalize` 匹配）重编译重发，**未落盘的依赖编辑也可见**；诊断只在真的变化时
   才 publish（`Doc::published`）。第一版"挂住"的根因是**测试写法**（一次通知连发
   多条诊断时先等通知再读 socket ⇒ 死锁），修法是 `testutil::notify_with_drain`——
   教训写进 `docs/TESTING.md` §5.7 与架构 §8.10。`crates/lsp/src/tests/project.rs`
   8 条 e2e + `project_refs.rs` 2 条单测；真实二进制探针复核过引用/改名/依赖失效。
6. **教学面与门面（P6）**：单元⑪「模块与项目」（CN/EN + 两份 solution，199/249 行）
   + 可运行两文件项目 `course/unit11-project/`（`sokonanoda.toml` + Logic/Canvas/
   Exercises + solution）+ `course.json`/`course/README.md`；goldens 重钉
   （画布 (7,6,0) 双语、solution (12,0,0)；总计 11 单元 / checked 85 / open 65 /
   failed 0）；`site/data/site.json` 重新生成。
7. **收尾**：版本 **0.56.1 → 0.57.0**（`Cargo.toml` + `editor/vscode/package.json` +
   VS Code CHANGELOG）；文档同步 `docs/architecture.md`（新增 §4.5 项目流水线 +
   §8.10 两个坑，仓库地图指向 TESTING）、`docs/TESTING.md`（3 行项目守护 + §5.7 盲区）、
   `docs/design/compile-cache.md` §7、`ROADMAP.md` I16、`REQUIREMENTS.md` §9（九十二）、
   `docs/HANDOVER.md`、三个 skills（teacher 多文件命令 + events 新码 + curriculum 单元⑪）、
   `AGENTS.md`、`dsh/README.md`、`editor/vscode/README.md`、
   `docs/design/deepseek-harness.md`；`scripts/soko gate` PASS +
   `cargo test --workspace --locked` 全绿。
8. **没做什么（有意）**：课程语料不回填 import（除新增单元⑪）；`watch --workspace`
   与 `soko/project` 不项目化；不做跨进程 decl 复用（v1 只缓存报告）；产物仍在用户
   缓存目录；`namespace`/`open`/`[deps]` 留 P7。
9. **LSP 侧只剩 P7 项**：`didChangeWatchedFiles`（编辑器**外**改文件不触发刷新，
   要重开文件）、`soko/project`、跨文件改名的"重命名文件/模块"形态；`watch` 项目
   模式、`[deps]`、`namespace`/`open` 同样留 P7。
10. **新增一笔结构债（已登记，不静默）**：`crates/front/src/compile/check.rs`
   1717 → **1918** 行（`run_pass` 单函数 ≈1174 行）——多 unit 泛化加在这里但没趁机
   拆函数（拆它要独立一轮，事件流/增量语义不能漂）。计划与验收见
   `docs/HANDOVER.md` §4 与 `docs/design/imports-and-projects.md` P7。
## 本轮进度（2026-09-17，第九十一轮：多文件 `import` 与项目管理 —— 调研 + 设计 + 计划 I16）

> 用户：「我想增加 代码import +project管理，帮我调研一下其他语言都是怎么分别处理单文件，
> 和项目。项目如何维护。sokonanoda如何实现，具体执行方案是什么」。
> **本轮只出调研 + 设计 + 计划，不动实现、不 bump 版本**（沿用第八十八轮先例）。
> 设计文档 = **`docs/design/imports-and-projects.md`**（ROADMAP **I16**）。

1. **调研（3 个并行 subagent，全部直抓官方文档/源码；`web_search` 无 API key 故走
   `curl`/`web_fetch`）**：
   - `docs/notes/multifile-prior-art.md` —— Coq/Rocq、Agda、Isabelle、Idris 2、Rust、Go、
     Python、JS/TS、Haskell/OCaml、JVM 的"单文件 vs 项目"逐系统记录 + 5 问横向表 +
     可抄模式/反模式（每条带官方 URL）。
   - `docs/notes/project-roots-and-incremental-caches.md` —— LSP 契约（`rootUri` **可为 null**、
     `didChangeWatchedFiles`、诊断"替换不合并"、**明文允许从缓存读诊断**）、9 个服务器/
     扩展的根发现与错根症状、失效与产物（Lake trace / GHC 指纹 / OCaml `.cmi` 摘要 /
     Coq `.vo` digest / `.tsbuildinfo`）、原子写与并发、**"缓存判定结果是否安全"的三条规则**。
   - 代码接缝（只读勘察，`path:line`）：一次编译 = 一个 arena + 一个 `EnvBuilder`
     （`compile/check.rs:424-425`）；内核名字身份 = **指针地址**（`kernel/util.rs:133-142`）
     ⇒ 跨 arena 复用环境不可能；`EnvLimit` 只表达**扁平前缀环境**（`kernel/env.rs:224-234`）；
     两遍 check-then-add（`check.rs:339-359`）；judge 只吃文本（`judge.rs:137-265`）；
     LSP 单槽 `Mutex<Doc>`（`lsp/lib.rs:58-60,152-155`）。
2. **设计一句话**：把**编译单元**从「一个文件」升级为「**项目闭包**」——`import Foo.Bar`
   用真实 Lean 4 置顶语法、模块名↔路径用 Lean 同款规则（`-` 非法 → 教学 hint）、
   项目根 = 最近祖先的 `sokonanoda.toml`（**向上搜索止于 `.git`/workspace 根**，
   `--root` 覆盖，无清单退化为"入口文件目录 = 模块根"——**对真实 Lean 的刻意
   divergence**：官方 `lean` 的搜索路径里**没有**文件自己的目录、cwd 只影响模块名
   的计算，§2.1 有源码依据；Q2 保留改回严格对齐的选项）；跨模块声明由 front 在
   **同一个 arena / 同一个 `EnvBuilder`** 里按拓扑序 `add_declar`（导入声明先入表，
   索引 `0..k`），**内核一行不改**、`EnvLimit` 语义零改动。
3. **硬边界与不变式**：① 无 `import` 的文件**行为逐字节不变**（缓存键、事件流、
   两处 golden 计数全不动 —— A1 用 `--json` 对拍守住）；② 每个 `Span` 只属于一个文件
   （**否掉源码拼接方案**）；③ 判定仍由内核终审；④ 用户路径零 cargo。
4. **量化动机（实测）**：45 个语料文件 3851 行里 **1217 行（31.6%）** 落在"名字在
   ≥2 个文件出现过"的声明块内；**71 个名字有 ≥2 种定义**、**20 个变体从未同单元共现**
   （`Or` axiom vs inductive、`Iff` def vs axiom、`And.*` 三种 binder 类型）；
   `course/unit6:21-36` 与 `unit7:16-31` 是**逐字节相同的 16 行 Nat 块**（中英共 4 份）；
   `solutions/` 与画布骨架 19/19、19/19、27/27 逐一对应。结论：**值得做 import 的理由是
   "同名不同义今天无法表达"，不是省行数**（最大 5 组重复一共只省 122 行）。
5. **分阶段计划 P0–P7**：P0 设计契约（本轮）→ P1 语法/模块名/resolver（含 fuzz 一次）→
   P2 闭包编译（一次 prelude、失败阻断、诊断归因）→ P3 CLI+协议（`--root`/`build`/`query`）→
   P4 闭包哈希缓存（可选信任台账，**启用前必须换强哈希**）→ P5 LSP（多文档表、根发现、
   反向后继重编、跨文件跳转、`didChangeWatchedFiles`）→ P6 第 11 单元 + 门面 + 发版 0.57.0 →
   P7 backlog（decl 级产物、`namespace`、跨项目依赖、语料重构）。
6. **风险清单里最值钱的三条**：① **三道"静默错误"门**（`front/tests/perf.rs:108` 的
   `kernel_checks <= 1`、`cli/tests/watch.rs:292-298` 的每文件独立契约、judge/suggest
   的静默无建议）；② **CI/Pages 不会发现"画布不再自包含"**（anchor 只在本地 `soko gate`，
   `gen-site-demos.py` 只守产物新鲜）；③ `elab-duplicate-declaration` 被测试枚举过 4 次却
   **从未真正触发**——而"同名到达两次"正是天真 import 实现的第一症状。
7. **待用户拍板 Q1–Q7**：清单格式（TOML/JSON/纯标记）、无清单时是否允许 import、
   prelude 模式决策者、是否做已检查声明的跨进程复用、课程语料是否同轮重构、
   `watch`/`soko/project` 是否 v1 就做、产物位置（用户缓存目录 vs 项目内 `.soko/build`）。
8. **本轮产物**：`docs/design/imports-and-projects.md`（新）、
   `docs/notes/multifile-prior-art.md`（新）、`docs/notes/project-roots-and-incremental-caches.md`（新）、
   `ROADMAP.md` I16、`REQUIREMENTS.md` §9（九十一）、`docs/README.md`（设计/笔记索引）、
   `docs/HANDOVER.md` §3 G、本文；**零代码改动、零版本变更**。

## 本轮进度（2026-09-17，第九十一轮续：`redundant-sorry` 落地 —— 5 分钟实验定位 + 内核显式限界 + 会话快照）

> 用户：「`docs/design/redundant-sorry.md` §8 直接做那个 5 分钟实验」→ 实验一次定位
> 真根因（旧假说被推翻）；用户：「继续」→ 按修正后的修法落地并补三层验收。
> **设计到实现的全过程在 `docs/design/redundant-sorry.md`（§8.1 根因 / §8.2 实验 /
> §8.3 修法 / §8.4 验收）。**

1. **实验（决定性，三行证据）**：同一 env、同一份探针，`as_is` 报的未知常量地址
   **正好等于**环境里 `A` 的规范节点地址（身份没问题）；只把探针 `info.name` 换成
   下一条真实声明 `g`（`decl_idx = 15` = 该练习的 `env_before`）、`ty`/`val` 一字未动
   → `Ok(())`。**旧假说"`NamePtr` 身份不对"作废**。
2. **真根因**：`check_simple_declar` 用 `EnvLimit::ByName(d.info().name)` 定可见前缀，
   而 `EnvLimit::ByName` 对没进过环境的名字取 **0**（`env.rs:257-260`）；探针**故意
   不入环境** ⇒ 空环境 ⇒ 连 `A` 都 `unknown const`。真实声明没事是因为它的名字有
   `decl_idx`。
3. **kernel（只加不改语义，已进 `docs/architecture.md` §6 适配表）**：新增
   `ExportFile::check_declar_at(d, EnvLimit)` / `try_check_declar_at(d, EnvLimit)`；
   `check_declar` 保持原行为，批量路径（`run_session_inner`）显式传同一个 `ByName`
   ⇒ 行为逐字节不变、热路径零改动。回归：`crates/kernel/tests/memory_api.rs`
   `synthetic_declaration_needs_an_explicit_environment_limit`（同时钉住"名字定限界
   = 空环境"这个坑与 `ByIndex`/`ByName` 等价）。
4. **front**：`PendingOp::OpenExercise` 带上 pre-pass 已有的 `env_before`，终审改
   `try_check_declar_at(_, EnvLimit::ByIndex(env_before))`（与真实声明的 cutoff
   **同一个值** ⇒ sound：前瞻引用照样不可见），探针检查计入 `stats.kernel_checks`；
   两条验收测试摘掉 `#[ignore]`，另加一条**可见前缀护栏**（同一形状只把 `g` 挪到
   练习后面 → 前瞻引用不可见 ⇒ 不报；经验配对实测 1 条 vs 0 条）。
5. **会话/LSP 的真实缺口**：`session.rs` 每轮只用 `collect_warnings` 重算语法级
   warning，**把内核终审过的 warning 丢了**（LSP 因此看不到 `redundant-sorry`）。
   修法：`CmdSnapshot.warnings` 按 span 归属命令、随快照跨版本复用并做坐标重映射，
   `WarningKind::is_kernel_verified()` 明确区分两类；LSP 侧该声明**不再**叠
   "not yet solved"（洞 span 与 warning span 形状不同 → 用包含判定），真缺口照旧报。
6. **验收**：`cargo test --workspace --locked` **全绿**（kernel 8 / front 413 /
   lsp 118 / cli …，0 failed；6 ignored = 缺 fixture 的 kernel 老用例）；
   `scripts/soko gate` **PASS**。用户 playground 326–328 的形状现在产出
   `warning[redundant-sorry]`（span 收窄到那个 `sorry`），语义不变（仍
   `exercise.open`、退出码 0）。测试账：**新增 6 条**（kernel 1 / front 1 新 +
   2 条摘 `#[ignore]` / CLI 2 / LSP 1 / session 1）。
7. **同步 + 发版准备**：`docs/architecture.md` §6、`docs/protocol.md`（warning 码
   清单 + `holes[].redundant` 字段 + `query` 两张表）、`docs/TESTING.md`（新行 +
   总量 772）、`skills/sokonanoda-teacher/{SKILL.md,references/events.md}`、
   `dsh/mcp/server.js`（工具描述教 agent 读 `redundant`）、`dsh/README.md`、
   `editor/vscode/README.md`（"Honest warnings"）+ `extension.js` 注释。
   **版本已 bump 到 0.56.2（patch）**：`Cargo.toml` + `editor/vscode/package.json`
   两处 + `Cargo.lock`（`cargo check` 跟上）+ `editor/vscode/CHANGELOG.md`
   `## [0.56.2]`，并**重建 `target/`**（`sokonanoda 0.56.2`，启动器解析回
   `repo-build`；否则会撞 `docs/vscode-dev-guide.md` 陷阱 13）。**只剩 commit + push**
   （auto-tag `v0.56.2` → release）。
8. **洞级标记（`query` 层）**：`HoleInfo`/`LocatedHole` 加 `redundant`（判定来自
   **同一份**报告的 kernel 终审 warning，用与 LSP 相同的包含规则），`soko/goals`
   wire 同字段；`crates/cli/tests/query.rs` 的**两视图契约**逐字段对拍
   `query goals`/`holes` ≡ `soko/goals`（真缺口为对照组）。
9. **发布（全自动，本机实测）**：push `dd1902d` → `ci` 绿（lint / test / auto-tag）
   → auto-tag `v0.56.2` + dispatch `release` → **11 个 job 全 success**（build ×8、
   package-vsix、github-release、marketplace-publish；**首次没撞 Azure gallery
   超时**，此前 4 次同版本窗口都超时过）。双页核对：GitHub Release **26 资产**
   （lsp ×8 / cli ×8 / vsix ×9 / SHA256SUMS，非 draft）；Marketplace
   `lastUpdated=09:17Z`、versions 出现 `0.56.2`（索引延迟 ≈5 分钟，符合台账）。
   **发布产物实测**：下载 `sokonanoda-cli-aarch64-apple-darwin.tar.gz` →
   `shasum -a 256 -c` OK（exec 位在）→ `./sokonanoda --version` = **0.56.2** →
   对"`f h` + 多留一行 sorry"的文件 `--json` 出
   `warning[redundant-sorry]`（span 收窄到该 token）、`exercise.open` 照旧、exit 0。
10. **本轮产物**：`crates/kernel/src/{tc,util}.rs`、`crates/kernel/tests/memory_api.rs`、
   `crates/front/src/compile/{check,goals,tests,warning}.rs`、`crates/front/src/session.rs`、
   `crates/front/src/query/{mod,types,tests}.rs`、`crates/lsp/src/{lib,protocol,query_map,by_sorry_range_tests}.rs`、
   `crates/cli/tests/{protocol,cli,query}.rs`、`docs/design/redundant-sorry.md`、
   `docs/{architecture,protocol,TESTING}.md`、`skills/sokonanoda-teacher/*`、
   `dsh/{README.md,mcp/server.js}`、`editor/vscode/{README.md,CHANGELOG.md,package.json,extension.js}`、
   `Cargo.toml`/`Cargo.lock`、`REQUIREMENTS.md` §9、`STATUS.md`
   （+ 第八十九轮归档进 `docs/STATUS-ARCHIVE.md`）。

## 本轮进度（2026-09-17，第九十轮：清掉 HANDOVER §4 的 LSP 测试文件债 + 0.56.1 发布）

> 用户：「继续 handover 吧，完成之后再 bump」。本轮 = 清第八十九轮登记的两笔结构债
> 里剩下的那笔（`crates/lsp/src/tests.rs` 2567 行），然后 bump 发 **0.56.1**。

1. **测试文件拆分（零语义改动）**：`crates/lsp/src/tests.rs` 2567 行 →
   `crates/lsp/src/tests/` 一目录：`mod.rs` **399**（31 个共享 const/fixture +
   `pub(crate) use` 再导出，子模块靠 `use super::*;` 取用）+ 9 个特性文件
   （`hover` 392 / `lenses` 366 / `navigation` 280 / `state` 262 / `goals` 252 /
   `lifecycle` 242 / `hover_brackets` 167 / `tokens` 122 / `perf` 117），
   `by_sorry_range_tests.rs`（60）原地保留。`lib.rs` 仍 **1105 行**——
   `#[cfg(test)] mod tests;` 自动解析到 `tests/mod.rs`，一行未改。
2. **"移动而非改写"的证据**（这次也按上轮的标准自证）：HEAD 的 `tests.rs` 里
   **107/107 顶层 item 逐字出现在新文件**、8/8 banner 注释保留、规范化代码行
   多重集 **2394 == 2394**（only-in-old 0 / only-in-new 0）；函数名 **95/95 一致**、
   测试名各出现一次（76 个测试：72 `#[tokio::test]` + 4 `#[test]`）、
   assert 记账 **214（tests/）+ 6（by_sorry）== HEAD 的 214 + 6 = 220**。
   新增行只有 plumbing：模块 doc 4 行、`use super::*;` ×10、`pub(crate) use` 再
   导出块、`mod …;` ×9；编译期唯一被迫改动是去掉再导出里没人用的 `Value`。
3. **两轮验证**：拆分中途（全部子文件首次编译通过）与冻结最终态各跑一遍
   `cargo test -p sokonanoda-lsp --locked` = **117 passed / 0 failed**；
   `cargo fmt --check` exit 0（**首次 fmt 没有改动任何文件**）；
   `cargo clippy -p sokonanoda-lsp --all-targets` exit 0、`crates/lsp/**` 零 warning。
4. **HANDOVER §4 的债清零**：`docs/HANDOVER.md` 该条从"已知债 + 拆分方案"改为
   "0.56.1 已清 + 最终布局"；`docs/TESTING.md` 的 LSP 行、设计文档 §3.2 文件表、
   `ROADMAP.md` I15 的备注同步到 `tests/` 新路径与新行数。
5. **版本 0.56.0 → 0.56.1**（内部重构，无用户可见变更）：`Cargo.toml` +
   `editor/vscode/package.json` 两处同步、`editor/vscode/CHANGELOG.md` 记
   "内部重构（测试文件拆分），扩展行为不变"。按仓库流程 push main → `ci.yml`
   auto-tag `v0.56.1` → `release.yml` 出八平台产物 + VSIX + marketplace。
6. **验收**：`cargo test --workspace --locked` 全绿（756）、fmt 零 diff、
   clippy 教学 crates 零 warning、`scripts/soko gate` **PASS**；发布 job 全绿后
   用发布产物复验（同第八十九轮的做法）。
7. **本轮产物**：`crates/lsp/src/tests/`（10 个文件）、`docs/HANDOVER.md`、
   `docs/TESTING.md`、`docs/design/agent-query-channel.md`、`ROADMAP.md`、
   `REQUIREMENTS.md` §9、`editor/vscode/CHANGELOG.md`、两处版本号。

## 本轮进度（2026-09-17，第八十九轮：内核真相查询通道落地 —— H6-A/H6-B/H6-C + 门面收尾）

> 续第八十八轮的设计（`docs/design/agent-query-channel.md`，ROADMAP **I15**）。按
> **H6-A → H6-B → H6-C** 逐项实现并测试，收尾做 H6-D 文档/门面同步；版本
> **0.55.0 → 0.56.0**（agent 可见的新能力 `query`，QD-7）。

1. **H6-A 真相层 + CLI（`front::query` + `sokonanoda query <op>`）**：
   `crates/front/src/query/{mod.rs,types.rs,tests.rs}` = **编辑器无关的唯一真相**
   （`QueryDoc` + `check`/`state`/`goals`/`holes`/`hints`/`reduce`）；
   `QueryError{NotParsable,OutsideDeclarations,PositionOutOfRange}` 把"正常的没有"
   与"问不出来"分开（各带稳定 code + 中文 message）。`crates/cli/src/query.rs`
   输出**单 JSON 对象**（`{schema:"soko.query/1", op, version, ok, data|error}`），
   退出码 = **0 答上了（含 `ok:false` 与开放 `sorry`）/ 1 内核拒绝 / 2 用法**；
   `--text` 支持未落盘中间态。契约写进 `docs/protocol.md`。
2. **LSP 改为调用真相层 + 结构债清零（同一轮完成，A1/A5）**：
   `soko/goals`/`stateAt`/`nextHole`/`hints` 与 hover 的 tactic 视图全部改为调
   `front::query`；新增 `crates/lsp/src/query_map.rs`（**唯一的形状映射点**：
   offset↔`Range`/`Position`、`QueryError`→既有空结果），删除 `select_state_at`/
   `StateSelection`/`runs_of`/`status_str`/重复的 `decl_name`/`goal_decls` 的 75 行
   主体等 → `crates/lsp/src/lib.rs` **4256 → 3988 行**。再按模块化硬规则把两个测试
   模块移出文件（`tests.rs` 2567 / `by_sorry_range_tests.rs` 60，**断言一字未改**，
   214+6 条 assert 与 HEAD 逐行等价）并抽出 `protocol.rs`（wire 类型，159）与
   `tokens.rs`（semantic token 辅助，107）→ **lib.rs 1105 行，≤1200 达标**。
   ⚠️ **过程留档（我自己的错）**：删完重复后我曾**没量就**把"≤1200 行"作废，
   理由是"剩下的都是协议服务代码"——`wc -l` 显示 3988 行里 **2638 行是
   `#[cfg(test)]` 模块**，非测试代码只有 ~1350 行，移出测试随手就达标。教训
   （**改验收标准之前先把被验收的东西量一遍**）进 `docs/LESSONS.md`；最终口径 =
   "**无重复实现**" **且** "**单文件 ≤1200 行**"（设计文档 §2.6/§3.2/§11 A5 已改）。
3. **⚠️ 抽层真的出过一次语义漂移（本轮最重要的教训，已进 `docs/LESSONS.md`）**：
   LSP 侧 117/117 全绿的情况下，**没有 `by` 块**的声明被错误地统一成"根状态"
   （已证声明凭空多出一个目标、半成品证明 `fun (a) (h) => sorry` 丢掉已引入的假设）。
   抓出它的不是测试而是**穷举对拍**：删除旧实现前，在 5 个画布的**每一个光标
   offset**（0..=len）上比较新旧两份实现，**709 次比较 / 424 处不一致全落在这一个
   分支**。既有测试没红是因为 LSP 唯一覆盖它的用例，画布**没有 lambda 前缀**，
   "剩余目标"恰好等于声明类型——**"新测试通过"不等于"新语义被测试"**。
   修法：`by_steps.is_empty()` 单独走"声明级目标 + 上下文"（协议 `docs/protocol.md`
   原文），红先单测 2 条（开/闭两分支）钉死，并把 CLI≡LSP 一致性契约扩到这两个
   **判别性输入**。教训同时写进设计文档 §4 as-built 3 / §12 风险表。
4. **H6-B MCP 传输 + DSH 接线**：`dsh/mcp/server.js`（零依赖 stdio 桥，
   `initialize`/`tools/list`/`tools/call`，六工具全部转发 `scripts/soko query …`；
   `server/discover` **立刻**用 `-32601` 拒绝——沉默会等满 SDK 的 60 s 超时）、
   `scripts/soko mcp`、`dsh/cordis.patch.yml` 的 `mcp-sokonanoda` 行（**默认关闭**，
   注释写清信任边界：MCP server 是 DSH 沙箱外的可信代码）。
   五个实测坑写进设计文档 §H6-B（探测进程/`capabilities.tools`/换行分隔 JSON/
   只有 `content[].text` 进模型/必须无状态可重启）。**实测验收**：DSH headless
   会话里模型调用 `mcp__sokonanoda__state` 拿到目标。
5. **H6-C 两个 front 缺口修掉**（原 `docs/HANDOVER.md` §3 E）：
   ① `derive_recursor` 在"**带索引 + 字段写在结果箭头链里**"时用只认 Ident/App 的
   `src_spine` 读索引实参 → 改为已会剥箭头的 `spine_of_codomain`（`elab.rs`），
   并顺带修掉写死的 `is_k: false`（单构造子 `Prop` 归纳因此被内核拒）；
   ② `inductive` 参数/ctor 字段不吃多名字 binder 组 `(A B : Prop)` → 解析器改调
   组感知的 `push_binders`（AST/elab 未动）。**课程随之简化**：unit9/unit10 的
   `Le`/`Even` 不再手写 `rec`/`iota`、`Or (A : Prop) (B : Prop)` 收成 `(A B : Prop)`，
   中英代码逐字节一致、**golden 事件计数不变**（unit9 `(13,8,0)`、unit10 `(7,6,0)`）。
   两条修复都先有"修复前红"的复现测试（`indexed_inductive_with_arrow_style_field_derives_recursor`
   等 4 条）；CLI e2e 三条 + 解析器三条补齐三层。
   **⚠️ 追加发现（"三层缺一不可"的实证）**：`is_k` 的第一版把它近似成"单构造子 +
   无索引 + 字段数 == 参数数"，front 单测全绿，但**内核拒了两个判别性形状**——
   `Both (A B : Prop)` + `mk (a : A) (b : B)`（字段数恰好等于参数数 → 内核要
   `is_k: false`），以及反向的 `Q : Nat -> Prop` + `q : Q 0`（**有索引但无字段 →
   内核要 `is_k: true`**）。抓出它的是新加的 CLI e2e 层。现按内核
   `init_k_target` 逐字镜像（`is_prop_block_ty(ty) && ctor_field_binders(only_ctor).is_empty()`），
   两个反例各留一条单测；方法（镜像内核谓词 = 逐字翻译 + 给判别性输入写测试）进
   `docs/LESSONS.md`。另外首版 `arrow_style_indexed_recursor_reduces` 名字承诺 iota
   却没碰 recursor（`theorem pz_again : P 0 := pz`），已改为 `Type` 值索引族 +
   `match` + `#reduce`。
6. **一致性契约（A4，防两套真相）**：`crates/cli/tests/query.rs` 12 项，其中
   `query_check_counts_match_the_json_event_stream` 钉"同一份判卷两个视图"，新
   `query_state_agrees_with_the_lsp_state_at_request` / `query_state_and_lsp_agree_without_a_by_block`
   **起真实 `sokonanoda-lsp` 二进制**做字段级对拍（根状态 / tactic 之内 / tactic 之后 /
   无 `by` 的开放与闭合）。注意：它比对的 `target/<profile>/sokonanoda-lsp` 可能是旧
   构件——**改了 front 只跑单 crate 测试会拿旧二进制对拍**（先 `cargo build --workspace`），
   这是特性也是坑，已写进设计与教训台账。
7. **两处刻意的 wire 边界对齐**（此前无测试覆盖，已记录）：`soko/hints` 的声明命中
   与 `stateAt` 统一为**含末尾**（旧路径开区间：光标恰在声明末偏移/末行行尾之后返回
   `[]`，现在返回阶梯）；由 offset 换算的 `Range` 改用**UTF-16** 列（LSP 规范口径，
   与其它响应一致；BMP 文本逐字节相同，仅增补平面字符不同）。扩展侧无需改动。
8. **H6-D 同步**：`AGENTS.md` Setup（`query` 两视图 + 六个 MCP 工具）、
   `skills/sokonanoda-teacher`（"先问，别扫"）、`skills/sokonanoda-dev`（"真相层不得
   绕过"）、`dsh/README.md`（查询一节 + 信任边界）、`docs/protocol.md`、
   `docs/TESTING.md`、`docs/HANDOVER.md`、`ROADMAP.md` I15 as-built、VS Code
   README/CHANGELOG/`package.json` 版本同步、`site/` agent prompt 一句。
9. **H6-E backlog（不做承诺）**：DSH Infoview 客户端插件（消费 `query goals/state`）、
   `SessionStart` 自动 provisioning、把启动器 + Lean 工具链 deny 拦截 + `/sokonanoda-*`
   命令打成一个 npm 插件包。
10. **发布**：CI 全绿 → auto-tag `v0.56.0` → release **11 个 job 全 success**
    （8 平台 build + VSIX + **marketplace 发布一次成功** + GitHub Release），
    26 个产物；并**用发布产物实测**（下载 CLI：`query state` 与仓库一致；下载 LSP：
    无 `by` 的开放声明 `goal='a' binders=['a','h']`、已闭合 `goal=None`）。
11. **本轮产物**：`crates/front/src/query/*`、`crates/cli/src/query.rs`、
    `crates/cli/tests/query.rs`、`crates/lsp/src/query_map.rs` + `protocol.rs` +
    `tokens.rs` + `tests.rs`/`by_sorry_range_tests.rs`（lib/hints/render 收敛）、
    `dsh/mcp/server.js`、`dsh/cordis.patch.yml`、`scripts/soko`（`mcp` 分支）、
    `crates/front/src/parser.rs` + `compile/elab.rs`（H6-C）、课程 9 个文件简化、
    文档/门面同步（见第 8 条），版本 0.56.0。

## 本轮进度（2026-09-17，第八十八轮：内核真相查询通道设计 + 两个 TODO 改挂）

> 用户：「H5 backlog 里从 MCP 诊断通道入手……这个你来设计一下开发文档，从根上正确
> 解决。同时看一下前人留下的两个 TODO，需要更新一下」。**本轮只出设计 + 改挂，
> 不动实现、不 bump 版本。**

1. **根因判断（为什么不能直接写 MCP server）**：内核真相今天**只有 LSP 一条出口**，
   而且选择/判定逻辑长在 LSP 适配器内部（`goal_decls`/`state_at`/`next_hole` 在
   `crates/lsp/src/lib.rs`，该文件 **4256 行**、远超 ~500 行红线）。直接写 MCP 会
   要么反向依赖 LSP、要么复制出**第二份真相**（违反"判定永远走 kernel"硬规则）。
2. **设计（`docs/design/agent-query-channel.md`，ROADMAP I15 / H6-A…H6-E）**：
   顺序不可颠倒的三层——① 真相层 `front::query`（`check`/`state`/`goals`/`holes`/
   `hints`/`reduce`，编辑器无关的类型化查询）；② 传输：`sokonanoda query <op>`
   （**单 JSON 对象**、零配置、所有 harness 通用、`--text` 支持未落盘中间态）
   + `scripts/soko mcp` / `dsh/mcp/server.js`（MCP stdio 六工具，只转发 CLI）；
   ③ **同一轮把 LSP 改为调用真相层**（顺带把 4256 行降到 ≤1200）。
3. **关键设计点**：`QueryError`/`QueryAnswer` 把"正常的没有"与"问不出来"分开
   （今天 LSP 用 `goal:null`+默认字段混合表达，agent 无法区分——这正是 agent 侧
   只能整文件扫事件流的根源）；位置在真相层用 offset、适配器转坐标（MCP 表面用
   `line`/`character` 与 DSH `lsp` 工具一致）；`query check` 是 `--json` 事件流的
   **新增摘要视图**，事件流契约**只增不改**；MCP **默认关闭**（DSH 视 MCP server
   为沙箱外可信代码，项目不替用户扩大信任面）。
4. **防两套真相的硬门禁**：契约测试断言 `query state` ≡ `soko/stateAt`、
   `query goals` ≡ `soko/goals`（字段级）、`query check` 计数 ≡ `--json` 事件计数，
   外加 `rg` 断言"LSP 侧不得残留查询实现"。
5. **两个 TODO 改挂**（用户要求）：`docs/HANDOVER.md` §3 E 的
   ①索引递归 `Prop` 的 recursor 自动派生被内核拒（`Le`/`Even` 靠课程手写
   `rec`/`iota`）②`inductive` 参数不吃多名字 binder 组 `(A B : Prop)`，
   从孤立 front 待办**改挂 H6-C**——它们决定查询通道"真相"的完整性与 agent
   （主要作者）写出的合法子集会不会被拒；要求**先有"修复前红"的复现测试**，
   按 TDD 三层 + 课程 golden 同步。ROADMAP I15、HANDOVER §3 表头/§3 E 已同步。
6. **待调研补齐**（设计文档 §9，已派 subagent 取源码证据）：DSH MCP client 的完整
   schema/传输/工具命名/失败语义与路径解析、项目侧可交付性，以及两个 TODO 的
   精确根因（哪一行 IH 形状不对、parser 单名路径清单）。
7. **验收口径 A1–A7**：真相唯一（LSP 无残留实现）、CLI/MCP 可用、CLI≡LSP 字段级
   一致、结构债达标（LSP ≤1200 行、`front::query*` ≤500 行/文件）、两个 TODO 带
   反向测试、全量回归绿且既有契约测试**只增不改**。
8. **本轮产物**：`docs/design/agent-query-channel.md`（新）+ `ROADMAP.md` I15 +
   `docs/design/deepseek-harness.md`（H5 的 B1/B2 指向新设计）+ `docs/HANDOVER.md`
   §3/§3E + `docs/README.md` + `REQUIREMENTS.md` §9（八十八）+ 本文。

## 本轮进度（2026-09-17，第八十七轮：DeepSeek Harness 适配落地 —— H0–H4）

> 用户确认「按 H0 → H1 → H2 → H3 → H4 开始实现」，按
> `docs/design/deepseek-harness.md` 五个阶段全部落地，版本 0.54.0 → **0.55.0**。

1. **H0 技能上架**：`.agents/skills/{sokonanoda-teacher,dev,ci}/SKILL.md` 三个
   **薄入口**（正文唯一源仍是 `skills/<name>/SKILL.md`，入口写明按仓库根解析）；
   新增 `crates/cli/tests/dsh.rs`（6 测试：入口↔正文双向、kebab-case 名、DSH
   frontmatter 白名单、指向正文且路径存在、`dsh/cordis.patch.yml` 形状、
   `scripts/soko` 解析链）。**实测**：DSH 会话里三个技能自动出现，`/sokonanoda-*`
   直接可用。
2. **H1 二进制可达**：新增 **`scripts/soko`**（零依赖 Node、跨平台、可执行位入
   git）——DSH 无 PATH 注入也无项目钩子，项目必须有一个可 commit 的入口。
   解析链 = `$SOKONANODA_BIN` → **版本匹配**（跑 `--version` 校验）的仓库构建 →
   缓存（marker 必须等于 `Cargo.toml` 版本）→ VS Code 扩展自带 → 版本锁定下载；
   **缓存过期直接拒绝运行**；网络受限经 `curl` 走 `HTTPS_PROXY`，失败给可诊断原因。
   `AGENTS.md` Setup 改 harness 中立；三个技能命令统一为 `scripts/soko …`。
   实测：`setup` 把本机缓存 0.16.2/0.20.0 → 0.55.0，`doctor --json` `ready:true`，
   `grade playground.sokonanoda` 出内核事件。
3. **H2 LSP 接线**：`dsh/cordis.patch.yml`（`lsp` + `lsp-stdio` + `tool-lsp`，
   `extensionToLanguage[".sokonanoda"]`，command 指向 `scripts/soko`）+
   `dsh/README.md`。**实测**：以仓库为 workspace 启动 DSH 会话，`lsp` 工具 hover
   `playground.sokonanoda:201:9` 返回内核打印的
   `theorem and_swap : forall (a b : Prop), And a b -> And b a`。
   踩到并记录三条新事实：`!!js` **必须单行**、`baseUrl` 是 profile 目录（不能用
   它推导仓库路径）、`lsp` 工具只在会话 workspace 内解析 `file_path`。
4. **H3 命令与角色**：`sokonanoda-teacher` §0 吸收角色与五条不可违反规则；
   `.opencode/agent/teacher.md` 瘦身为指针，**并修掉它里面违反零 cargo 硬规则的
   `cargo run` 判卷命令**；7 个 opencode 命令统一走 `scripts/soko`（`opencode.rs`
   改为"gate 之外的命令必须 cargo-free + 全部走启动器"）。
5. **H4 治理**：`dsh/hooks/{hooks.json,refuse-lean-toolchain.js}` 实现官方 Lean
   工具链 deny（命令位匹配：拦 `lake build`/`$(lean …)`、放行 `grep lean`，
   12 例实测）；`AGENTS.md` 硬规则第 2 条写明两 harness 的 deny 形态；
   `skills/README.md` 重写为多 harness 安装矩阵；`docs/design/onboarding.md` §6
   DSH 对照表；`site/assets/agent-prompt.js` 安装 prompt 改 `scripts/soko` 并说明
   DSH；VS Code README/CHANGELOG/package.json 同步。
6. **验收**：`cargo test --workspace --locked` 全绿（21 个测试目标，含新增
   `dsh.rs`）；`cargo fmt --check` 绿；`clippy` 仅 kernel 既有 warning；
   `scripts/soko gate` **PASS**；site 生成与卫生检查绿。版本 **0.55.0**。
7. **本机环境坑（非仓库问题）**：Xcode 27 许可未接受时 `xcrun`/`ar` 被系统拦，
   `cargo` 链接必失败；绕过用
   `DEVELOPER_DIR=/Library/Developer/CommandLineTools cargo test …`，根治是
   `sudo xcodebuild -license accept`。已记入 `docs/HANDOVER.md` §5。
8. **待做**：`docs/design/deepseek-harness.md` §5 H5 backlog（Infoview 客户端插件 /
   诊断通道 / 启动钩子 / npm 插件包）；§3 E 的两个 front 缺口仍在。
## 本轮进度（2026-09-17，第八十六轮：DeepSeek Harness 适配——只出计划）

> 用户接手项目：「很多地方还没适配 deepseek harness，先理解项目、分析要适配哪里、
> 列一下计划文档」。**本轮只做调研 + 设计，不动实现、不 bump 版本。**

1. **审计结论**：产品内核（kernel / `.sokonanoda` 前端 / `--json` 事件 / 三个技能）
   与 harness 无关、可直接移植；要适配的是**接线层**——技能发现路径、斜杠命令、
   编辑器 LSP 接线、环境与二进制可达性、工具链 deny、文档与契约测试的单 harness 假设。
   **不需要改任何 Rust 语义代码**（Rust 侧唯一新增是契约测试 `crates/cli/tests/dsh.rs`）。
2. **差距 G1–G10**：技能不能被 DSH 发现（P0）/ 七个 `/sokonanoda/*` 命令不存在 /
   无 teacher 主 agent / `.sokonanoda` 无 LSP 接线 / 二进制不在 PATH 且 DSH 禁止项目
   改 PATH（P0）/ Lean 工具链 deny 无对应物 / 33 处文档与契约测试只认 opencode /
   `AGENTS.md` 的 code-agent 适配原则缺 DSH 条目 / 无项目级 provisioning /
   用户级技能环境噪音。
3. **DSH 侧关键事实（逐条带源码行号，文档 §1.2 共 23 条）**：技能根扫描含
   `<repo>/.dsh/skills`(rank 100) 与 `.agents/skills`(200)；**技能名本身即斜杠命令**
   （`/name` 注入正文，零 profile 配置）；frontmatter 路由词只能写 `description`
   （`whenToUse` 是 camelCase 且只进人类 `/` 选单，**旧 camelCase 的
   `disableModelInvocation` 等会让整条技能被丢弃**）；**LSP 不在任何 shipped bundle**，
   且 DSH 的 LSP 只有 4 项只读操作，**`publishDiagnostics` 被显式丢弃**、`soko/*`
   无消费者；工具调用 PATH 不可由项目配置（唯一例外是 LSP 自己的 `env`）；patch 为
   顶层 YAML 数组（`- id:` 整块替换 config、会丢 `!!js`；空文件会 boot 失败），
   `--patch` 可叠且**无项目级自动发现**；hooks 桥只有一个进程级 `configPath`、
   **不做项目发现**；**符号链接是官方同款做法**（DSH 仓库自用
   `.claude/skills -> ../.agents/skills`，watcher 默认跟随）。
4. **计划 H0–H4（每阶段独立可验收）+ backlog H5**：H0 技能上架（`.agents/skills/`
   放软链或薄网关、正文唯一留在 `skills/`、新增 `crates/cli/tests/dsh.rs` 守卫）→
   H1 二进制可达（新增零依赖 Node 启动器 `scripts/soko`，解析链与 opencode 插件同语义
   + marker 版本守卫；`AGENTS.md` Setup 改 harness 中立）→ H2 LSP 接线
   （项目自带 `dsh/cordis.patch.yml` + `--patch` 用法，显式写清诊断不在通道内）→
   H3 命令与角色并入技能（opencode 命令与 teacher agent 正文移进
   `sokonanoda-teacher`）→ H4 治理（deny 形态、33 处文档去 opencode 单一化、门面同步）。
5. **决策 D-1…D-6 与验收 A1–A6** 已列（技能进 DSH 的方式 / 启动器形态与
   REQUIREMENTS（三十二）删除 `scripts/soko.sh` 的边界 / 是否自动 provisioning /
   deny 形态 / `soko/*` 处置 / 版本号策略）。
6. **实测现状**：`sokonanoda` 不在 PATH；缓存为旧版（marker `0.16.2 darwin-arm64`
   vs 仓库 **0.54.0**），`doctor --json` 报 `ready:false`——历史「版本漂移致环境未就绪」
   的故障模式当前正在发生，H1 的 marker 守卫正针对它。
7. **验收**：本轮产物 = `docs/design/deepseek-harness.md` + `REQUIREMENTS.md` §9（八十六）
   + `docs/HANDOVER.md` §3 F/§5/§6 + `docs/README.md` 设计清单 + 本文；不改代码，
   不跑 gate（无代码改动）。

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


## 本轮进度（2026-09-16，第八十一轮：`by` 块支持换行分隔 tactic）

> 用户：能不能像 Lean4 一样用**分号或回车换行**两种分隔，从而省掉行尾的 `;`
> （举了 `playground.sokonanoda` 的 `forall_and` 为例）。

1. **难点**：`exact`/`apply` 的表达式会贪婪跨行（换行只是空白），`exact f` 换行
   `apply g` 会被读成应用 `f apply g`。
2. **规则**：解析 tactic 时（`by_depth > 0`），若下一 token 在**更晚的行**且是
   **tactic 关键字**（`intro/exact/apply/assumption/rfl/match/sorry`），当前表达式结束。
3. **实现**：`Parser.by_depth`（`parse_tactic` 包一层，Ok/Err 都减）；
   `starts_atom` 在边界处返回 false（应用不吞下一行 tactic）；`parse_by_block` 在
   `;` 或「下一行 tactic 关键字」时继续。仍未引入缩进敏感。
4. **测试**：parser 5 项（换行分隔 / 边界胜过应用 / 多行项仍是单 tactic / `;` 与换行混用 /
   不吃下一个命令）；CLI `cli_by_newline_separated_tactics_check_via_kernel`。
5. **活样例**：`playground.sokonanoda` 的 `forall_and` 去掉行尾 `;`（gate 仍跑该文件）。
6. **有意不支持/歧义**（写入 by-tactics.md §11）：同行不写 `;` 不算分隔；续行以 tactic
   关键字开头的多行项会被切开（用括号/同行规避）；`sorry` 在下一行即视为新 tactic。
7. **验收**：front 378 + CLI 全绿；`playground.sokonanoda` exit 0；`sokonanoda gate` PASS；
   版本 0.50.0 → **0.51.0**。

## 本轮进度（2026-09-16，第八十轮：Infoview 自研调色板（主题解析回退））

> 用户反馈：Infoview 里 `Type`/`Prop` 没高亮、参数色与编辑器/hover 不一致，问能否
> 与主题自动对齐。调研结论：VS Code **无稳定 API** 暴露主题 token 色（2026-06 只有
> proposal #319754/#319753）；唯一路线是自己复刻主题解析。先按此实现了完整解析器
> （`theme-colors.js` + 宿主解析主题 JSON + `colors` 消息 + 测试，18 项单测），
> **用户判定代价过大** → 回退，改为 **Infoview 自研固定调色板**。

1. **根因（已修）**：`.tok-sort` 用 `--vscode-symbolIcon-structForeground`，主题未定义
   时回退 `--vscode-foreground` → `Prop`/`Type` 看着没高亮；`.tok-binder` 用 symbolIcon
   palette，天然不同于编辑器的 parameter token 色。
2. **调研**：官方仅有两个 proposal（`ColorTheme.tokenColors`、`languages.getDocumentTokens`）；
   可行但昂贵的路线是读活动主题 JSON（含内置主题）展开 `include` + `tokenColors` +
   `semanticTokenColors` + `editor.tokenColorCustomizations` 后做 TextMate 特异性匹配。
3. **回退**：删除 `editor/vscode/theme-colors.js`、`test-theme-colors.js`、`colors` 消息
   通道与宿主解析（grep 证明零悬空引用）。
4. **落地**：Infoview 自研固定调色板——`:root` + 四个 `body[data-theme=…]` 定义
   `--soko-{type,keyword,function,variable,parameter,number,enum,macro}`（dark/light 贴近
   Dark+/Light+ token 色）；`.tok-*` **只**读 `--soko-*`（不再有 symbolIcon 优先链 →
   任何主题必有着色）；workbench 前景/背景仍走 `--vscode-*`。
5. **测试**：`infoview_palette_colours_every_kind_with_a_guaranteed_fallback`（每个
   `.tok-<kind>` 解析到 `--soko-*` 且四个主题块都定义）；`test-webview.js` 10 项。
6. **验收**：node 三套 + `cargo test -p sokonanoda-cli --test extension`（32）全绿；
   `sokonanoda gate` PASS；版本 0.49.0 → **0.50.0**。
7. **诚实边界**：Infoview 颜色与编辑器/hover **不逐像素相同**（后者是主题 token 色，
   前者是自有色板）——这是平台限制 + 用户拍板的取舍，写入 `docs/design/highlighting.md` §3b。

## 本轮进度（2026-09-15，第七十九轮：共享缓存 + `build` / Infoview 稳定与反馈 / 高亮单一起源）

> 用户三轮反馈：(a) Infoview 面板"点几次才出现、很不稳定"，怀疑是编译卡住，要求
> 面板 UI 必须保证出现、数据可显示"渲染中"/编译进度；(b) 去掉没生效的声明点击跳转，
> 名字后加小字行号；(c) hover 的高亮与 Infoview 不一样、没有收拢。另要求
> `sokonanoda build` 这类命令配合缓存。派出 5 个 subagent 分头实现（SA-1…SA-B）。

1. **共享编译缓存（SA-1）**：缓存下沉到 `crates/front/src/compile/cache.rs`，
   条目含 `report` + `output`；key = `CACHE_FORMAT|版本|二进制构建指纹|prelude 模式|源文本`；
   `SOKONANODA_CACHE_DIR`/`SOKONANODA_NO_CACHE`；原子写；`compile_all_with` 一趟出两者。
2. **`sokonanoda build`（SA-2）**：`build [--json] [--clean] [<file>|<dir>…]` 预热/清理
   缓存并打印 hit/compiled/failed；`course` 与批量 `--json` 走 `compile_cached`
   （冷热输出逐字节一致，有测试）；CLI 测试用临时 `SOKONANODA_CACHE_DIR` 隔离。
3. **Infoview 稳定性根因（SA-3）**：视图原带 `when` + 扩展容器 `hideIfEmpty: true`
   → 无激活 `.sokonanoda` 时容器整块隐藏；且 `activate()` **先 `await
   resolveServerForStart` 才注册 provider** → 期间视图无 provider（"点几次才出现"）。
   修：视图无 `when` + `visibility: visible`；`activationEvents` 加
   `onView:sokonanoda.infoview`；provider/树**同步先注册**，慢解析后置并推 `status`；
   `openInfoview` 先开辅助栏。契约测试锁死注册顺序。
4. **UI 反馈（SA-3）**：webview 载入即骨架（`正在渲染…`），宿主推 `status`
   （`编译中…`/`已就绪 · N 个声明`/`等待 .sokonanoda 文件`），绝不静默空白。
5. **声明列表（SA-3）**：去掉点击跳转；名字后小字行号 `L<n>`（1-based）+ 类型提示。
   新增 `editor/vscode/test-webview.js`（Node DOM shim 行为测试 8 项）并入 `test:unit`。
6. **高亮单一起源（SA-A/SA-B）**：`SemanticKind::{ALL, as_str, tm_scope}` 唯一表；
   `runs_to_text`/`goal_text`/`goal_runs` 单一文本生产者；hover 目标态改由 runs 投影
   （不再手搓字符串）；TM 语法补齐 `variable.parameter` 等 scope；三处穷尽测试
   （TM scope / CSS 类 / LSP legend）防漂移。设计 `docs/design/highlighting.md`，
   并**写明平台限制**：markdown 只能 TM 着色 → 颜色近似而非全等。
7. **验收**：`cargo test --workspace --locked` 全绿 + `sokonanoda gate` PASS；
   `build` 冷/热/clean/目录/课程缓存冒烟通过；版本 0.48.0 → **0.49.0**。

## 本轮进度（2026-09-15，第七十八轮：编译结果缓存 + Infoview 细节）

> 用户四项：(1) Infoview 类型小行允许换行；(2) 目标用 `⊢` 开头；(3) 点击
> Infoview 跳转没生效；(4) 设计类似 Lean4 的编译结果文件（避免文件多了打开即编译慢）。

1. **换行**：`media/infoview.css` 的 `.decl-ty` 由「单行省略」改 `pre-wrap` +
   `word-break`（类型不再看不全）。
2. **`⊢` 开头**：`infoview.js` 的 goal 代码块加前缀 `⊢ `（与 hover/树 tooltip 一致）。
3. **点击跳转修复**：点了 webview 后 `activeTextEditor` 为空，旧实现据此直接失败。
   改为 plumb 文档 uri（树的 `onDecls(decls, uri)` → `setDecls(decls, uri)` →
   webview `focusExercise{uri,range}`），扩展用 `jumpToRange`（`visibleTextEditors`
   优先、必要时 `openTextDocument`）跳转。
4. **编译结果缓存（olean 式）**：`crates/lsp/src/cache.rs` 把内核产出的
   `DocumentReport` 以稳定 FNV 哈希 `(CARGO_PKG_VERSION, prelude 模式, 源文本)`
   落盘；`refresh` 命中则跳过 `session.update`，miss 则编译并落盘。诊断由
   `report_diagnostics` 统一构造（命中/重编一致）。`SOKONANODA_NO_CACHE=1` 关闭、
   `SOKONANODA_CACHE_DIR` 重定位；front 报告类型加 serde derive。
5. **测试**：`cache.rs` 单测 3（key 稳定/作用域/format miss）；扩展契约更新
   （`⊢`/wrap/uri 跳转）。
6. **验收**：`sokonanoda gate` PASS；版本 0.47.0 → **0.48.0**（新能力 minor）；
   设计 `docs/design/compile-cache.md`。已知边界：不缓存 Session 快照、无 LRU。

## 本轮进度（2026-09-15，第七十七轮：带索引归纳）

> 续 HANDOVER §3 B / ROADMAP I6 的最后一项：`inductive Vec (A : Type) : Nat -> Type`。

1. **索引定义**（内核契约）：索引 = `ty` 在 `num_params` 之外的 Pi 望远镜
   （`inductive.rs::check_inductive_spec_0th`）；内核本支持 `num_indices`，本轮
   只补前端。设计 `docs/design/indexed-inductives.md`。
2. **安装**：`install_inductive_block` 算 `index_binders`/`num_indices`，传入
   `add_inductive`/`RecursorData`，存入 `InductiveInfo{num_indices,index_types}`；
   `is_prop_block_ty` 先剥索引望远镜。
3. **派生 recursor**：motive = `forall indices, Ind params indices -> Sort`；rec 绑定序
   `params→motive→minors→indices→target`；minor = `motive <ctor 索引> (C 字段…)`；
   iota 自调用带字段索引实参；字段名替换同时作用于字段类型与 ctor 结果索引实参。
4. **match**：从 scrutinee 书写类型取索引实参；motive 先绑索引再绑 major；
   应用 `Ind.rec params motive minors indices scrutinee`。顺带修既有 latent bug：
   字段类型引用前面字段（`v : Vec A n`）时按「字段原名→用户绑定名」substitution。
5. **边界**：结果类型依赖索引不做（sound 拒绝；另立设计）。
6. **测试/课程**：front +3、CLI +1；课程 unit5 带索引 Vec 节 + 练习 10
   （golden `(11,9,6)→(13,10,7)`、汇总 `checked 55→57 / open 42→43`）。
7. **验收**：`sokonanoda gate` PASS；版本 0.46.0 → **0.47.0**（新语法 minor）。

## 本轮进度（2026-09-15，第七十六轮：`match` 作为 tactic）

> 续 HANDOVER §3 B / ROADMAP I6：`by` 块内可用 `match`（设计与白名单此前待定）。

1. **tactic 集**：`by` 白名单加 `match`——`match c with | p => <项> …`，臂体是
   **项**（同值位 match），以当前目标为期望类型判定，语义等价 `exact (match …)`；
   `parse_tactic` 复用 `parse_match` + `tactic_keyword_ahead` 纳入 `match`。
2. **judge 修复（根因）**：`judge_terms` 合成文件原 `src: String::new()`，
   `command.span().start` 前缀切片为空 → `match` 的宇宙查询（`judge_infer` 看
   不到 `Color` 等声明）失败，报 `elab-match-no-expected-type`。改为把真实
   `prefix_src` 作为文件 `src`、合成声明 span 放到前缀之后。副产品：
   `by exact match …` 也可用。
3. **测试**：parser `match_is_a_tactic_in_a_by_block`（白名单 + 降到 Exact）；
   front `by_block_with_match_tactic_checks` / `by_block_with_exact_match_checks`；
   CLI `cli_by_match_tactic_checks_via_kernel`。
4. **文档**：`by-tactics.md` §2 表 + 0.46.0 更新、architecture、TESTING。
5. **验收**：`sokonanoda gate` PASS；版本 0.45.0 → **0.46.0**（新语法 minor）。
   注：臂体是「项」；「每个臂里再写一串 tactic」是后续可选扩展（设计 §9 留白）。

## 本轮进度（2026-09-15，第七十五轮：应用位置 binder 类型推断）

> 续 HANDOVER §3 C / ROADMAP I6：elaborator 最后一项——无期望类型时从实参
> 推断 `fun x => …` 的 binder 类型。

1. **现状**：`fun x => …` 在有期望望远镜时已能推断（`Expr::Lambda` +
   `peel_expected`）；缺的是 `(fun x => x) 1` 这类无期望的应用位置。
2. **实现**：`annotate_application_lambda`——处理 `Expr::App` 前展平 spine
   `f a1 … an`；头部是带未注解 binder 的 `Lambda` 时，用 `judge_infer`
   推断 `a_i` 类型作为 binder 注解，**源到源改写**后交回正常路径；支持
   柯里化 `(fun x y => x) a b`。
3. **边界**：实参不足以覆盖全部未注解 binder → 仍报 `elab-untyped-binder`
   （`(fun x y => x) 1`、`#check fun x => x`）。
4. **测试**：front +3（应用/柯里化/实参不足）、CLI +1；既有 `untyped_binder_*`
   回归不破。
5. **文档**：architecture §elab、`elaborator-let-match.md` as-built、TESTING。
6. **验收**：`sokonanoda gate` PASS；版本 0.44.0 → **0.45.0**（新能力 minor）。

## 本轮进度（2026-09-15，第七十四轮：Infoview 声明类型提示 + 点击跳转）

> 用户：Infoview 的「声明」除名字外，用小字写出类型做提示，注意排版（保持
> 每行一个声明）；并支持鼠标点击跳转。

1. **协议**：`soko/goals` 每条声明增 `ty`（内核渲染的声明类型）与 `ty_runs`
   （`front::semantic` runs，与 goal 同一分类源）；`docs/protocol.md` 同步。
2. **Webview 排版**：声明项改「名字 + kind/status 徽标」一行、下面一行
   `.decl-ty` 小字（0.78em、暗色、等宽、单行省略）按 `tok-*` 着色——保持
   「每行一个声明」的节奏；`codeBlock` 复用同一渲染路径。
3. **点击跳转**：点击声明 post `focusExercise` 带 `range`；扩展处理器在
   `focusDeclaration` + 聚焦练习树之外，把编辑器光标移到该声明并 reveal。
4. **测试**：LSP `goals_request_lists_open_exercise_with_hole_range` 增 `ty`/
   `ty_runs`（重建 + sort kind）断言；扩展契约
   `infoview_declaration_list_shows_types_and_jumps`（webview/css/host 三处）。
5. **验收**：`sokonanoda gate` PASS；版本 0.43.0 → **0.44.0**（新能力 minor）。

## 本轮进度（2026-09-15，第七十三轮：呈现面高亮统一）

> 用户追问「各个地方的高亮统一」后补做（HANDOVER §3 A″）：0.40.0 只统了 goal
> 状态，其余渲染 `.sokonanoda` 的面仍各自为政。原则：**着色只来自
> `front::semantic`**（语义 token + TM 语法 + runs），手段是统一 `{sokonanoda}`
> markdown 围栏。

1. **LSP**：新增 `CODE_LANG`/`code_block`/`goal_block`；`hover_markup`（表达式/
   签名 hover）由 ` ```text ` 改 ` ```sokonanoda `；声明 hover 的签名、洞期望
   类型、目标态都用代码块；tactic hover 的 tactic 片段、半表达式 hover 的
   推断类型/目标也从行内代码改成代码块；补全 `documentation` 给出签名的
   `sokonanoda` 围栏。
2. **扩展**：`codeMarkdown`/`goalTooltip`——练习树「目标」「假设」tooltip 用
   `MarkdownString.appendCodeblock(…, "sokonanoda")`。
3. **刻意保持纯文本**（VS Code 不渲染 markdown / 不给行内代码语言）：诊断消息、
   inlay hint、TreeItem.description、CodeAction 标题；hover 里「散文提到单个词」
   也保持行内代码。文档写明（`goal-rendering.md §7`）。
4. **契约**：LSP `code_fences_always_use_the_sokonanoda_language` + hover/
   completion 断言；扩展 `rendered_language_text_uses_the_sokonanoda_fence`。
5. **验收**：`sokonanoda gate` PASS；版本 0.42.0 → **0.43.0**（行为统一，minor）。

## 本轮进度（2026-09-15，第七十二轮：`match` 模式编译器 v1）

> 续 HANDOVER §3 B / ROADMAP I6：把「每构造子一条 arm」换成有序 arm + 列式
> 模式编译，支持字面量/嵌套/通配/守卫。设计 `docs/design/match-patterns.md`。

1. **AST/parser**：`Pattern { Wild, Num, Ident{name,args} }`；`MatchArm` 改
   `{pattern, guard, body}`；`parse_pattern`（递归、`(...)`、`_`、数字）；
   守卫 `if` 只在 arm 里识别（不升全局关键字）。
2. **编译器（核心）**：**源到源 canonical 化**——`compile_pattern_body` 选可反驳
   列、按构造子特化，生成嵌套 `Expr::Match`，每层仍走既有 motive/IH/level/
   recursor 构造（**不手搓 de Bruijn**）；守卫复用 prelude `Bool` 的 match。
   字段名取绑定名（canonical 幂等）、撞构造子名用新鲜名；参数化字段先代入参数
   （`some (a : A)` 在 `Option Nat` → `Nat`）。
3. **语义**：有序、首个匹配者胜；未知裸名 = 绑定变量（带子模式才 bad-arm）；
   覆盖不全/守卫无兜底 = `elab-match-non-exhaustive`；error hint 措辞更新。
4. **消费者**：`semantic`（模式绑定着色 + 守卫）、`proof::render_pattern`、
   `spine`（mentions/substitute 含守卫与模式阴影）、`goals`（hole/替身/依赖
   子目标；嵌套/守卫退回常量 R）。
5. **测试**：front +8、CLI +3；课程 unit5 增嵌套模式节 + 练习 9
   （golden `(10,8,4)→(11,9,6)`、汇总 `checked 54→55 / open 41→42`）。
6. **文档**：architecture §2/§4.1、design `match.md` §2/§10 Phase 6、
   `match-patterns.md` as-built、TESTING、protocol、CHANGELOG。
7. **验收**：`sokonanoda gate` PASS；版本 0.41.0 → **0.42.0**（新语法 minor）。
   已知限制：`as`/or 模式、多 scrutinee、`if/then/else` 表达式不做。

## 本轮进度（2026-09-15，第七十一轮：prelude `Bool`）

> 续 HANDOVER §3 C / ROADMAP I6：把 `Bool` 作为真实可信归纳加进 prelude，
> 与 `Nat`（0.36.0）同法，供 `match` 与后续布尔例子使用。

1. **安装**：`prelude.rs::install_bool_prelude` 调用既有
   `install_inductive_block`，`Bool` **非递归** → 构造子 `Bool.true`/`Bool.false`
   + 派生 `Bool.rec`（两分支、无 IH），登记进 `known` 与 `match` 的
   `InductiveTable`；`PRELUDE_NAMES` 增 4 个名字（补全/目标视图）。
2. **闸**：`check.rs::run_pass` 增 `explicit_bool`——文件自带 `inductive Bool`
   时 prelude 让位（否则重复声明 panic）；`session.rs::PreludeShape` 扩成
   `(mode, explicit_nat, explicit_bool, eq_taken)`，任一变化整体重编译。
3. **内核零改动**：`Bool.true`/`Bool.false` 的 name-cache 槽位早已存在
   （原生 `Nat.beq`/`Nat.ble` 用），归约走通用构造子 iota。
4. **测试**：front `prelude_bool_is_available_without_a_source_block` /
   `match_prelude_bool_not_checks_and_reduces`（`#reduce bnot Bool.true =>
   Bool.false`）/ `prelude_bool_definitions_compose` /
   `explicit_bool_block_yields_to_the_source_declaration`；CLI
   `cli_match_on_prelude_bool_checks_and_reduces`；既有源内 `inductive Bool`
   （`tt`/`ff`）用例继续通过=闸生效。
5. **文档**：`architecture.md §5.4`、`design/match.md §2/§10 Phase 5`、
   `TESTING.md`、`protocol` 错误文案（`Nat/Bool`）；错误提示改为
   「prelude 内建的 Nat/Bool」。
6. **验收**：`sokonanoda gate` PASS；版本 0.40.0 → **0.41.0**（新能力 minor）。

## 本轮进度（2026-09-15，第七十轮：统一 goal 呈现 + Infoview 落右侧）

> 用户：Infoview 弹出「暂时不可用」很困惑、希望默认在右侧；各处 goal
> 高亮/颜色各自独立不可维护，要求对齐 VS Code 代码框标准。参照 Lean4
> Infoview（服务器下发结构化 tag + 客户端按主题渲染）。设计
> `docs/design/goal-rendering.md`。

1. **单一分类源**：`front::semantic` 新增 `tag_runs`/`tag_expr`/
   `declaration_kinds` + `SemanticKind::{ALL, as_str}`——把任意表达式文本按
   编辑器同一套规则切成 `(text, kind)` runs。
2. **协议**：`soko/stateAt`（及 `soko/goals` 的 binders）新增 `goal_runs`/
   `ty_runs`（有序 `{text, kind?}`，`kind` 用 wire 名），旧字符串字段保留；
   `docs/protocol.md` 记录词表。
3. **Infoview 渲染**：webview 用 runs 生成 `tok-<kind>` span（不再自绘规则、
   仍仅 textContent），`infoview.css` 单一映射到主题变量；契约测试断言每个
   `SemanticKind` 都有 `.tok-*` 类。
4. **落位 + fallback**：视图移出 explorer，进
   `viewsContainers.secondarySidebar` 的 `sokonanoda` 容器（**右侧**，engine
   `^1.85.0 → ^1.106.0`，已核实 1.106 为无需 proposed API 的首个稳定版）；
   删除 `waitReady`/2s 握手与「暂时不可用」提示，失败静默回退树组。
5. **防漂移**：TM 语法（hover 代码框着色）关键词/命令/sort 列表由测试断言
   == `front::semantic`（keywords + sorts + forall），删掉硬编码 `Nat`。
6. **市场门面**：`description` 348 → 247 字符（>300 被 Marketplace 硬截断、
   切在 `opencode` 中间）+ 护栏测试；README/CHANGELOG 同步。
7. **验收**：front/LSP/cli 契约测试 + `sokonanoda gate` PASS；版本 0.39.1 →
   **0.40.0**（新面板位置 + 协议字段，minor）。



> 续 TODO（HANDOVER §3 A）：消除依赖类型判定/建议里「内核类型文本 → AST」往返
> 的括号歧义。

1. **根因**：`proof::render_expr` 的 `Arrow` 分支把 **domain** 直接 `render_expr`，
   当 domain 是 Forall/箭头时输出 `(k : Nat) -> P k -> Q` 被右结合误读；
   `judge_infer` 逐层 render→parse 剥 Pi 时腐蚀 telescope → 依赖 `match` 的
   level 查询报 `elab-match-no-expected-type`。
2. **修复**：Arrow domain 位改用 `render_fun_position`（Lambda/Forall/Arrow/
   Plus/Let/Match 一律补括号）。
3. **回归**：`render_expr_round_trips` 增「Forall 作 domain」用例（含渲染→再解析
   稳定）；`match_dependent_motive_with_function_typed_binder_round_trips_safely`
   （结果类型 `Q hs n`、`hs` 为依赖函数 binder）内核通过。
4. **影响**：`judge_infer` 的所有消费方受益（依赖 `match`、suggest、半表达式
   hover、level 查询）。
5. **验收**：`sokonanoda gate` PASS；版本 0.39.0 → **0.39.1**（健壮性 patch）；
   设计 as-built `docs/design/match-dependent-motive.md` §8；HANDOVER §3 A 勾选。

## 本轮进度（2026-09-14，第六十九轮：judge_infer 类型往返健壮性）

> 续 TODO（HANDOVER §3 A）：消除依赖类型判定/建议里「内核类型文本 → AST」往返
> 的括号歧义。

1. **根因**：`proof::render_expr` 的 `Arrow` 分支把 **domain** 直接 `render_expr`，
   当 domain 是 Forall/箭头时输出 `(k : Nat) -> P k -> Q` 被右结合误读；
   `judge_infer` 逐层 render→parse 剥 Pi 时腐蚀 telescope → 依赖 `match` 的
   level 查询报 `elab-match-no-expected-type`。
2. **修复**：Arrow domain 位改用 `render_fun_position`（Lambda/Forall/Arrow/
   Plus/Let/Match 一律补括号）。
3. **回归**：`render_expr_round_trips` 增「Forall 作 domain」用例（含渲染→再解析
   稳定）；`match_dependent_motive_with_function_typed_binder_round_trips_safely`
   （结果类型 `Q hs n`、`hs` 为依赖函数 binder）内核通过。
4. **影响**：`judge_infer` 的所有消费方受益（依赖 `match`、suggest、半表达式
   hover、level 查询）。
5. **验收**：`sokonanoda gate` PASS；版本 0.39.0 → **0.39.1**（健壮性 patch）；
   设计 as-built `docs/design/match-dependent-motive.md` §8；HANDOVER §3 A 勾选。

## 本轮进度（2026-09-14，第六十八轮：`match` 依赖 motive）

> 续 TODO：让 `match` 的结果类型随 scrutinee 变化（`P n`），从而能写出归纳法。

1. **设计** `docs/design/match-dependent-motive.md`（触发/构造/交互/风险）。
2. **前端**：scrutinee 是裸局部变量 `x` 且 `R` 含 `x` → motive = `fun t =>
   R[x:=t]`（`substitute_names`），分支期望 = `R[x:=<ctor 项>]`、IH 类型 =
   `R[x:=<field>]`；motive/分支/IH 类型在 binder 存活的 scope 里 elaborate。
   否则保持常量 motive（完全兼容）。
3. **修缺口**：`infer_expected_level` 改为只纳入 `R` 依赖到的 binder
   （`judge_binders_for`），修掉「无关函数型 binder 破坏 judge_infer 望远镜」
   导致**声明 binder 形式**（`nat_induction`）level 查询失败的问题。
4. **goal 视图**：match-arm 走查同样代入 `x := C params v…`，分支 `sorry` 期望
   `R[x:=ctor]`。
5. **测试**：front `match_dependent_*`（含声明 binder 的 `nat_induction`）；
   CLI `cli_match_dependent_motive_checks_via_kernel`；课程 unit5 加依赖 match 节
   （`nat_induction` + 练习 8）；golden `(9,7,4)→(10,8,4)`、汇总
   `checked 53→54 / open 40→41`。
6. **验收**：`sokonanoda gate` PASS；版本 0.38.0 → **0.39.0**（新能力 minor）。
7. **已知限制**：motive 引用「类型为以箭头结尾的依赖函数」的 binder 时，
   `judge_infer` 的 render→parse 往返仍可能腐蚀 telescope（需 `judge.rs` 改
   一次性解析，或 `proof::render_expr` 给 domain 位 `Forall` 加括号）。

## 本轮进度（2026-09-14，第六十七轮：参数化归纳声明）

> 续 TODO：match 参数化的前置——教学语言 `inductive` 声明支持参数（非带索引）。

1. **设计** `docs/design/parameterized-inductives.md`（含内核期望形状与风险）。
2. **前端**：parser 解析 `(A : Type)`/`{A : Type}` 参数 → `InductiveBlock.params`；
   归纳类型 `forall params, sort`；ctor `forall (params++fields), C params`；
   `add_inductive(num_params=params.len())`；`num_fields` 字段-only 计数；
   `derive_recursor` params 最外层 + motive `(t : Ind params) -> Sort u` + iota
   lambda/自调用带 params。`InductiveInfo` 增 `num_params`/`param_names`。
3. **match**：`Ind.rec.{level} <params> motive minors scrutinee`；params 取
   scrutinee **书写源类型**头部实参；字段 `src_ty` 做 params 替换后 elaborate；
   拿不到 → 新码 `elab-match-parameterized-unsupported`。
4. **测试**：front +8（parse 2 / compile 6，含 `Option`/`List` 派生递归子与
   `match`、显式 rec/iota、iota 错 → `kernel-rec-rule-mismatch`）；CLI +4；
   课程 unit5 加 `Option` 小节 + 练习 7；golden `(7,6,3)→(9,7,4)`、汇总
   `checked 51→53 / open 39→40`。
5. **文档**：`architecture.md §4.1/§8`、`TESTING.md`；设计 as-built §9。
6. **验收**：`sokonanoda gate` PASS；版本 0.37.0 → **0.38.0**（新语法 minor）。
7. **v1 边界**：带索引归纳、宇宙多态参数、互/嵌套递归、`match` 嵌套/守卫/字面量。

## 本轮进度（2026-09-14，第六十六轮：watch stdin 客户端命令）

> 续 TODO：compiler-service-events 设计的 v1 未做面（客户端→服务命令）。

1. **命令集**（stdin JSON Lines）：`ping {id}` → `pong {id, protocol, engine}`；
   `subscribe {file}`/`unsubscribe {file}` 过滤 `--workspace` 事件（首个
   subscribe 收窄白名单；默认全发兼容旧行为）；畸形/未知命令 → `error` 事件且
   流不中断。
2. **非阻塞实现**：后台线程 `stdin().lock().lines()` + `mpsc`，轮询每 300ms
   `try_recv` 排空；stdin EOF 不杀 watch；零新依赖（仅 std）。
3. **测试**：`crates/cli/tests/watch.rs` ping/subscribe/unsubscribe/malformed
   4 项 + watch.rs 单测 2 项（用 ping→pong 同步，不 sleep）。
4. **文档**：`docs/protocol.md` watch 小节、`TESTING.md`；设计 as-built
   `docs/design/compiler-service-events.md` §9。
5. **验收**：`sokonanoda gate` PASS；版本 0.36.0 → **0.37.0**（新能力 minor）。

## 本轮进度（2026-09-14，第六十五轮：`match` 支持 prelude Nat）

> 续 TODO：移除「match 只支持源内 inductive」限制，让学生直接对 prelude `Nat`
> 分情况。

1. **根因**：prelude 的 `Nat` 是「原生 hack」（无 ctor 的 add_inductive +
   `Nat.zero` 公理 + 自引用 `Nat.succ` 定义），**没有 `Nat.rec`**，`match` 无从
   降低。改为经 **`install_inductive_block` 装成真实可信归纳**（ctor `Nat.zero`
   /`Nat.succ` + 派生 `Nat.rec` + iota，`recursive=true`，`rec_universe_arity=1`），
   并注册进 `InductiveTable`；`Nat.add` 保持原生自引用定义。
2. **效果**：`match n with | Nat.zero => … | Nat.succ k => …`（点号 ctor）可用，
   递归字段后自动 IH；`#check Nat.rec` 有签名；`#reduce 1 + 1 => 2`、
   `Nat.add 2 3 => 5`、`three => 3` 均正常。**已知显示**：经 `Nat.rec` 归约的
   结果可能是不合并的一元链（如 `addN 2 1 → Nat.succ (Nat.succ 1)`，与 numeral
   def-eq），已文档化并钉测试。
3. **测试**：front +3（prelude Nat pred/add + 源内 Nat 回归）；CLI e2e +1。
4. **文档**：`match.md` §2/§10、`architecture.md` §4.1/§4.2/§5.4/§8、
   `TESTING.md`、`protocol.md`（`elab-match-not-inductive` 文案）同步。
5. **验收**：`sokonanoda gate` PASS；版本 0.35.1 → **0.36.0**（新能力 minor）。

## 本轮进度（2026-09-14，第六十四轮：发布加固）

> 续 TODO（用户指定顺序：先 match 递归 IH，再发布加固）：`onboarding.md §5`
> 剩余的发布完整性三项。

1. **`SHA256SUMS`**：`release.yml` 的 `github-release` job 对 8 lsp + 8 cli +
   9 vsix 生成校验和清单并 `--clobber` 上传（资产 25 → **26**）。
2. **SLSA provenance**：`actions/attest-build-provenance@v2` 对上述资产签发
   构建来源证明；job 加 `id-token: write` + `attestations: write`。校验
   `gh attestation verify <file> -R ColorlessBoy/sokonanoda-lang`。
3. **文档**：`docs/RELEASE.md` §6（校验与证明）、`skills/sokonanoda-ci` §2.1
   资产数 26、`onboarding.md §5` 三项勾选（含 `rust-toolchain` 决策：**不钉**，
   跟随 stable；README 补 binstall/mise）。
4. **契约**：`crates/cli/tests/extension.rs` release 契约增 `SHA256SUMS` /
   `attest-build-provenance@v2` / `attestations: write` 断言。
5. **验收**：`sokonanoda gate` PASS；版本 0.35.0 → **0.35.1**；发布后核
   Release 26 资产 + `sha256sum -c` + `gh attestation verify` 通过。

## 本轮进度（2026-09-14，第六十三轮：`match` 递归归纳（IH））

> 续 TODO（用户指定顺序：先 match Phase 2 递归 IH，再发布加固）。

1. **前端**：递归归纳不再一律拒绝；构造子**递归字段后自动插入归纳假设** `ih`
   （避开既有名 → `ih2`…），类型 = match 结果类型 R（v1 非依赖 motive），push 进
   branch scope 供引用；minor 以「字段 + IH」序列折 lambda。递归函数/证明经 IH
   表达，**无需自引用**（`def add (a b : Nat) := match a with | zero => b |
   succ m => succ ih`）。
2. **测试**：front `match_recursive_inductive_uses_the_induction_hypothesis`
   （`add two two` 经内核归约到 `s (s (s (s z)))`）；CLI recursive match 3 项。
3. **课程**：unit5 `match` 小节加递归 IH 演示 + 练习 6（`recDouble`）+ `#reduce`
   自测；golden `(6,5,2)→(7,6,3)`、汇总 `checked 50→51 / open 38→39`。
4. **文档**：`match.md` §2/§5/§6/§10、`architecture.md` §4.1/§8、`TESTING.md` 同步。
5. **验收**：`sokonanoda gate` PASS；版本 0.34.0 → **0.35.0**（新能力 minor）。
6. **仍缺（Phase 2 余项）**：依赖 motive、参数化/带索引归纳、prelude `Nat`/`Eq`、
   `match` tactic、嵌套/字面量/守卫模式。

## 本轮进度（2026-09-14，第六十二轮：无注解 `let`）

> 续 TODO 清账：把 `let` 的类型标注变为可选（Phase 2 小切片）。

1. **实现**：`Expr::Let` 的 `binder.ty == None` 时，用当前 scope 的
   `judge_binders()` + `judge_infer`（复用有界缓存）推断值类型、回 AST 作 binder
   类型（`match` 轮已把 `ElabCtx{prefix,options}` 贯通进 elab，正好复用）。
   推断失败（值位 `sorry` 等）→ 新码 `elab-let-type-query-failed`（protocol +
   穷尽清单 + hint 同步）。
2. **测试**：front `let_without_annotation_infers_the_value_type` +
   `unannotated_let_that_cannot_be_inferred_reports_a_let_specific_error`；CLI
   `json_mode_unannotated_let_infers_or_reports_hint`；课程 unit3 注释更新。
3. **验收**：`sokonanoda gate` PASS；版本 0.33.1 → **0.34.0**（新能力 minor）；
   设计 as-built `docs/design/elaborator-let-match.md` §13。

## 本轮进度（2026-09-14，第六十一轮：tactic hover 呈现升级）

> 用户反馈：tactic hover 内容有了，但只有单/两行裸文本，无排版无高亮，体验不行。

1. **表头**：`` `<tactic>` · tactic k/n ``（k/total 为 per-tactic 进度）；
   闭合显示 `已无剩余目标 ✓`。
2. **目标块**：包进 ` ```sokonanoda ` 代码围栏 → 等宽对齐 + **语法高亮**
   （扩展自带 TM 语法，hover fenced code 用该语言着色）；多目标加 `**目标 i/n**`。
3. **半截表达式 hover** 同步 `⊢` + 同款围栏。
4. **测试**：tactic hover 断言表头/围栏；half-expression 断言 `⊢`/围栏。
5. **验收**：`sokonanoda gate` PASS；版本 0.33.0 → **0.33.1**（呈现 patch）；
   设计 `docs/design/tactic-hover.md` §5 as-built。

## 本轮进度（2026-09-14，第六十轮：elaborator `match` v1）

> 续 TODO 清账（Phase 2 首切片）：按 `docs/design/match.md`（本轮 spike 定稿）
> 落地 `match`（限源内非递归 `inductive`）。

1. **可行性 spike**：手写 `Color.rec.{1} (fun _ => Color) green red c` 被内核
   接受，缺 `. {level}` 被拒 → 降低必须从期望类型 Sort 推 level。
2. **前端**：`Expr::Match`/`MatchArm`、`TokenKind::Pipe`、`parse_match`（裸 ctor、
   按声明序重排、恰好覆盖一次）；`InductiveTable` 登记表 + `ElabCtx`/`expected_src`
   贯通；降低为 `<Ind>.rec.{level} (fun _ => R) minors… e`；`goals`/`spine`/
   `proof`/`semantic`/`suggest` 同步。+19 测试（含与手写 recursor 的等价契约）。
3. **错误码**：`elab-match-{bad-arm,not-inductive,no-expected-type,recursive-unsupported,non-exhaustive}`
   （已入 protocol + 穷尽清单）。
4. **课程 + CLI**：unit5 新增「match 分情况」小节（自定义非递归枚举 + rec/iota，
   zh/en/钥匙逐字节镜像，2 练习）+ golden `(4,3,1)→(6,5,2)`、汇总
   `checked 48→50 / open 36→38`；CLI e2e +4。
5. **验收**：`sokonanoda gate` PASS；版本 0.32.1 → **0.33.0**（新语法 minor）。
6. **v1 边界（未做）**：递归归纳（IH）、依赖/参数化归纳、prelude `Nat`/`Eq`、
   `match` tactic、嵌套/字面量/守卫模式、无注解 `let`。

## 本轮进度（2026-09-14，第五十九轮：I8 early-cutoff + arena 基准立项）

> 续 TODO 清账（R58）：ROADMAP I8 验收余项「受影响后缀的依赖精确化」+ TESTING
> §5 perf 基准立项。

1. **设计** `docs/design/early-cutoff.md`（机制/soundness/测试/边界）。
2. **early-cutoff（保守 sound）**：每条命令在 `try_check_declar` 前用内核
   结构化 `debug_print` 渲染「环境贡献签名」（kind+name+宇宙+type+**body**+
   hint+ctor/recursor/iota；归纳块串联），存 `CmdSnapshot.signature`（不改
   `--json`/LSP 形状）。单点编辑时累积 `[i, j)` 签名，遇到文本不变且签名与上轮
   相同的 `j` 即停止，`[j, n)` 快照复用、内核检查跳过；任何内核拒绝或多点编辑
   一律退回旧后缀重查；prelude 形状守卫变化整文件重建。body 进签名保证 delta
   可观察性 sound。
3. **效果**（测试实测 kernel_checks）：`def one := 1 → (1)` 4→**1**；axiom 3→**1**；
   Nat 归纳块 6→**1**；改 body/宇宙元数/多点编辑不 cut（正确重查）。
4. **arena 基准立项**：`scripts/perf-arena.sh`（opt-in，`LEAN_KERNEL_ARENA`
   门控，未设给获取指引并跳过；不 vendor、不进 CI、不引入官方 Lean 工具链）+
   `docs/PERF.md`「External baseline」；TESTING §5 盲区第 6 条更新。
5. **文档/清单**：`docs/design/i8-i9.md` §4 更新（early-cutoff 已补做）；
   `ROADMAP.md` I8 余项勾选。
6. **验收**：`sokonanoda gate` PASS（front 298、perf 3、cli 105、lsp 112）；版本
   0.32.0 → **0.32.1**（内部性能优化 → patch）。

## 本轮进度（2026-09-14，第五十八轮：spine meta 方案 A）

> 续 TODO 清账：按 `docs/design/spine-meta-a.md` 落地 refine 子洞的
> kernel 级期望类型（请求期探针，内核冻结）。

1. **front**（`goals.rs`）：公开 `probe_sub_goal_types`——请求期重解析 + 带
   `judge_infer` 重跑；第 i 实参期望 = 部分应用类型剥最外层 Pi domain；
   **前置洞穿透**（`f sorry sorry` 第二个用第一个的期望）+ **一层嵌套洞**
   （`f (g sorry)`）。`open_goal` 仍 `probe=None` → 键路径零内核调用。
2. **LSP**：`probed_report` 仅在 `soko/goals`/hover/inlay 请求期补 `None` 的
   `sub_goals[i].ty`；`stateAt`/`nextHole` 不探测；协议形状/洞数不变。
3. **测试**：front 4（defeq 别名+前置洞、依赖字段、一层嵌套、更深回退）+
   LSP 4；B′ 既有断言不变；perf 无回退（goals/hover 0ms、didChange 1ms）。
4. **验收**：`sokonanoda gate` PASS；版本 0.31.0 → **0.32.0**（新增公开 front
   API → minor）。
5. **未闭环（留档）**：超量应用里「def 包裹的结果类型」whnf 展开需内核/pp 暴露
   （违反冻结）→ 仍走 B′；更深嵌套/非 spine 实参仍 `None`。

## 本轮进度（2026-09-14，第五十七轮：扩展强制内置 LSP + doctor 自检）

> 用户报告：扩展升到 0.29.0，`restart server` 仍回弹 `0.26.0 → 0.26.0`。
> 盘链路确认：用户设置里 `sokonanoda.serverPath` 指向仓库陈旧的
> `target/debug/sokonanoda-lsp`（0.26.0），显式路径优先级最高，静默压过内置
> 0.29.0 服务器。用户要求：**强制用扩展自带的 LSP**，并**自动检测所有版本问题**。

1. **设计** `docs/design/extension-server-policy.md`（含 as-built）。
2. **强制内置**：`resolveServerCommand` 增 `override`（默认 `false`）与
   `{command, source}`；默认链 = **内置 → 当前缓存 → 锁定下载兜底**，
   `serverPath`/env/工作区构建**忽略**（弹一次提示，含「打开设置/运行
   doctor」）；新增设置 `sokonanoda.serverOverride`（默认 false，restricted）
   供贡献者恢复旧序。
3. **doctor**：`sokonanoda.doctor` 只读输出 6 项自检（解析来源/运行版本/
   宿主版本/被忽略覆盖/缓存/旧版本堆积），激活与 restart 后自动跑一次，
   有问题非阻塞提示；restart 回执带 `source=`。
4. **测试**：`test-server.js` override 单测；`extension.rs` 静态契约；
   `extension.test.js` doctor 冒烟；`node --check` 全绿。
5. **止血（本机）**：移除用户设置里的 `serverPath`（备份
   `settings.json.bak-sokonanoda`）、删 8 个 `.obsolete` 旧版本（只剩 0.29.0）。
6. **验收**：`sokonanoda gate` PASS；版本 0.30.0 → **0.31.0**（新设置+命令）。

## 本轮进度（2026-09-14，第五十六轮：VS Code webview Infoview）

> 续 TODO 清账（顺序 R57→R56→R55）：按
> `docs/design/webview-infoview.md` 落地方案 B（Lean Infoview 式 webview）。

1. **view/命令**：新增 `sokonanoda.infoview`（`type: webview`，与练习/课程
   并列）+ `sokonanoda.openInfoview`；树的「当前光标处」组保留为默认与兜底
   （webview 不可用时功能零回归）。
2. **协议**（`protocol:1`）：宿主→webview `state`（`soko/stateAt`）/`decls`
   （仅诊断·切文件）/`server`（`soko/version`）/`theme`；webview→宿主
   `ready`/`reveal`/`focusExercise`；按文档 `version` 丢弃过期 `state`。
3. **安全**：CSP `default-src 'none'` + 每次随机 nonce、`localResourceRoots`
   限 media；渲染只用 `textContent`（契约负断言禁 `innerHTML`/远程 URL/eval）。
4. **性能**（吸取 goal-list §2.4 教训）：光标移动只发轻量 `state`（去抖
   200ms），绝不触发 `soko/goals` 或整树重建；provider 缓存最后快照。
5. **测试**：`crates/cli/tests/extension.rs` 静态契约（view/命令一致、资源与
   CSP nonce、负断言）；`extension.test.js` 集成 smoke；`node test-server.js`
   18/18。
6. **验收**：`sokonanoda gate` PASS；版本 0.29.0 → **0.30.0**（新 view+命令）。

## 本轮进度（2026-09-14，第五十五轮：编译器服务事件流）

> 续 TODO 清账（用户确认顺序 R57→R56→R55）：按
> `docs/design/compiler-service-events.md` 落地 watch 服务事件流。

1. **规范名**：watch 开场事件 `file.changed` → **`file.didChange`**（payload
   不变，新增稳定 `file` 字段）；`file.changed` 保留一个 minor 的弃用别名
   （`WATCH_VOCABULARY` 接受、不再发射）。
2. **握手**：stdout 第一行恒为 `service.hello {protocol:1, engine, pid}`
   （对齐 LSP `soko/version`；原纯文本 banner 移出 stdout）。
3. **作用域**：`watch <file>` / `--doc <file>` / `--workspace <root>`（互斥）；
   workspace 递归发现 `*.sokonanoda`，每文件一个 `Session` 与独立版本号、
   事件带 `file`、跨文件无全序。
4. **背压**：每文件有界缓冲（64），溢出合并为最新版本并标
   `recompiled_from: 0`（协议注明可全量重同步）。
5. **测试**：`crates/cli/tests/watch.rs` 6 项（握手/规范名/`--doc`/workspace
   独立版本/闭词汇 + `file`/protocol.md 覆盖）+ watch.rs 单测（溢出合并）；
   CLI 套件 105 pass、`skill.rs` conformance 绿。
6. **文档**：`protocol.md` watch 小节 + `TESTING.md` + teacher `events.md` +
   `help.rs` 同步。
7. **验收**：`sokonanoda gate` PASS；版本 0.28.0 → **0.29.0**（协议/功能 minor）。

## 本轮进度（2026-09-14，第五十四轮：elaborator `let`（Phase 1））

> 续第五十三轮的 TODO 清账：按 `docs/design/elaborator-let-match.md` 的
> S1–S5 落地**值位 `let`**（Phase 1）。`match` 依设计推迟到 Phase 2。

1. **设计**：`docs/design/elaborator-let-match.md` §11 切片 S1–S5 + 本文 §12 as-built。
2. **front（S1–S3）**：`Expr::Let`；`parse_expr` 识别 `let`（`starts_atom` /
   `named_group_ahead` 排除）；elab 分支（外层类型 + 期望类型 + `mk_let`
   `nondep=false`，缺注解 `elab-untyped-binder`）；`spine`/`proof`/`semantic`/
   `goals` 同步；`open_goal` 支持值位/body 洞。+21 测试（含 zeta 等价契约）。
3. **课程 + CLI（S4–S5）**：unit3 新增「局部绑定 `let`」小节（zh/en/钥匙，
   `def`/`#reduce` 逐字节镜像 + 两道 sorry 练习）；golden `unit3 (1,4,1)→(2,6,2)`、
   汇总 `checked 47→48 / open 34→36`；CLI e2e +3；`architecture.md` §4.1/§8、
   `TESTING.md` §1 同步。
4. **验收**：`sokonanoda gate` PASS（front 287、cli 97、lsp 108）。
5. **版本** 0.27.1 → **0.28.0**（新语法 = minor，Cargo + VSIX + CHANGELOG Added）。

> TODO 余项：`match`（Phase 2）、无注解 `let`、spine-meta A 实现、webview
> Infoview、事件流、perf 基准 + I8 early-cutoff、SHA256SUMS/attest。

## 本轮进度（2026-09-14，第五十三轮：TODO 清账——测试/文档债 + 大项设计）

> 用户要求：把 ROADMAP §10 未勾选项 + TESTING §5 盲区 + onboarding §5 待办
> **全部按流程做**（设计先行、TDD、多用 subagent）。本轮清掉测试/文档债、
> 重建内核 fixture、落地低风险安装子集，并为大项产出设计（实现留后续轮）。
> 全过程由 6+2 个 subagent 并行产出，主会话统一验证 + gate。

1. **穷尽守卫修复**：`protocol_doc_lists_every_error_code` 原先的 `matches!`
   自带 `_ => false`，根本守护不了。改成**不带通配分支的 `match kind {}`** +
   穷尽 `all` 数组（26 variant）；`docs/protocol.md` 补 `elab-apply-*` 两个
   漏掉的 code；`docs/TESTING.md` §5.1/§2 同步。
2. **codeLens / quick-fix 自动化**（补齐 §5.3 缺口，+3 测试）：
   `code_lens_ranges_match_each_declaration`、`code_action_refine_edit_targets_the_hole`、
   `code_action_open_goal_without_a_next_step_offers_none`；四族 quick-fix
   与 codeLens 全走进程内 rpc。
3. **内核 fixture 重建 + 解禁**（§5.2）：`RuleDomainMismatch`（伪造 iota 规则
   λ 定义域）与 `UnlistedRecursor`（未派生 recursor）两个 NDJSON fixture 人工
   构造，删掉 `crates/kernel/src/tests/util.rs` 的两个 `#[ignore]`，
   `cargo test -p sokonanoda` 两条均 pass（内核源码/语义未动）。
4. **安装/打包低风险子集**：`scripts/install.sh`（POSIX、零 cargo、版本锁定、
   禁用 latest）、`.devcontainer/devcontainer.json`（贡献者）、`docs/design/onboarding.md`
   §5 勾选与余项、README/docs 入口。
5. **大项设计（设计先行）**：`elaborator-let-match.md`（`let` Phase 1 详细 /
   `match` Phase 2 预研）、`spine-meta-a.md`（I9 余项方案 A）、`webview-infoview.md`
   （方案 B）、`compiler-service-events.md`（L1/L3 事件流）。
6. **ROADMAP 清账**：I10/I11/I13 三节陈旧「活规格」压缩为存档说明（指向
   `remove-funintro.md` 与现行 by/goal 设计），I6/I8/I9/I12 与两条未勾选项保留。
7. **验收**：`sokonanoda gate` PASS（含新解禁内核测试）；本轮机/文/脚本改动
   **未 bump 版本**（无运行时行为变化，0.27.1 保持）。

> 余项（各自独立轮，设计已就位）：`let`/`match` 实现（R54）、spine-meta A 实现
> （R55）、webview Infoview（R56）、事件流（R57）、perf 基准 + I8 early-cutoff
> （R58）、SHA256SUMS/attest immutable releases。

## 本轮进度（2026-09-14，第五十二轮：restart server 版本纪律修复）

> 用户报告：下载 0.27.0 插件后 LSP 仍报 0.26.0，疑发布流程有问题；要求
> `sokonanoda: restart server` 应校验版本、不重用旧的缓存 LSP。

1. **核实发布**：下载 `v0.27.0` 的 darwin-arm64 VSIX，内置 LSP 实测
   `soko/version = 0.27.0`；已安装 0.27.0 扩展的 `server.js` 解析到内置
   0.27.0。发布无误；0.26.0 来自本地下载缓存（`sokonanoda version` 报的是
   缓存，非运行中的服务器）与宿主未重载。
2. **定位缺陷**：`resolveServerCommand` 本就拒绝过期缓存（stale → undefined，
   有单测钉住），但 `restartServer` **没有激活路径的版本锁定下载兜底**：
   解析返回 undefined 时它保持 `serverOptions` 不变并 `client.restart()`，
   于是静默重启了旧的（可能来自过期缓存的）服务器。
3. **修复**：抽出 `resolveServerForStart`（激活/重启共用：显式路径 → 内置 →
   工作区 → 当前缓存 → `v<扩展版本>` 锁定下载）；restart 用它，拿不到可用
   服务器时报错而不重启旧命令；新增 `newestInstalledExtensionVersion`，检测到
   磁盘上更新的扩展而当前宿主仍旧时提示 `Developer: Reload Window`。
4. **测试**：CLI 契约 `restart_server_re_resolves_and_never_keeps_a_stale_command`
   钉住共用解析器 + 下载兜底 + 更新提示；`node test-server.js` 18/18 绿。
5. **版本** 0.27.0 → **0.27.1**（fix → patch）；CHANGELOG Fixed、
   `docs/vscode-dev-guide.md` §5.6 同步。

## 本轮进度（2026-09-14，第五十一轮：tactic 关键字高亮 + hover goal state）

> 用户反馈：`exact` 没有正确高亮；希望像 Lean 一样在每个 tactic 上 hover 看到
> 中间 goal state（Infoview 式），或按鼠标位置给 goal state。

1. **设计** `docs/design/tactic-hover.md`。
2. **高亮**：`semantic::KEYWORDS` 增补 `by`/`exact`/`assumption`/`rfl`（此前
   当普通标识符着色；`forall`/`sorry` 已由 token/Hole 正确处理）。
3. **hover**：`textDocument/hover` 首插 `tactic_goal_hover`——光标落在某 tactic
   span 内 → 用 `by_steps` + `select_state_at`（进入态语义，与 `soko/stateAt`
   同数据同规则）渲染全部目标与假设（多目标标 `目标 i/n`），range = 该 tactic；
   纯快照消费，零重编译零文本扫描。
4. **测试**：front semantic 断言四关键字均 Keyword；LSP
   `hover_on_a_tactic_shows_the_entering_goal_state`（hover `apply` → `⊢ And P Q`；
   hover `sorry` → `⊢ P` / `⊢ Q`）。
5. **协议**：`docs/protocol.md` 新增「Tactic goal-state hover」小节；并入 0.27.0。
6. **验收**：`sokonanoda gate` PASS。

## 本轮进度（2026-09-14，第五十轮：移除值位关键字 funintro）

> 用户评估：「`funintro` 跟 `funapply` 一样，实现起来稀里糊涂的，不如直接删了。」
> 确认按「彻底删」执行，与多目标显示并入 0.27.0。

1. **设计先行** `docs/design/remove-funintro.md`（根因/方案/测试/验收/as-built）。
2. **前端**：删 `Expr::Intro`、`parse_intro`/原子位/lambda 尾关键字分支、
   `KEYWORDS` 的 `funintro`、`compile/intro.rs`、`DeclState.intro_skeleton`、
   `ErrorKind::ElabIntroNotAFunction`；各 crate 匹配臂与测试同步。
3. **协议/客户端**：`soko/stateAt` 的 `goals` 相关不受影响；删 LSP 值位关键字
   补全/hover/code action/inlay 全路径、VS Code `sokonanoda.expandIntro`
   命令与 `markdown.isTrusted` 白名单；`docs/protocol.md` 值位关键字小节改为
   「已移除」说明。
4. **课程**：unit6（zh+en）改写「补充 funintro」段 + 练习 6 为综合 `by` 练习，
   钥匙同步；golden 计数不变。
5. **文档/site/技能**：architecture/README/TESTING/ROADMAP(I13 标废弃)/
   term-intro/value-keywords-v2(废弃横幅)/site hero 换图/gen-site-demos 删演示/
   teacher 技能表同步；历史归档与 CHANGELOG 原文保留。
6. **测试**：`cli_value_funintro_is_no_longer_a_keyword` 钉「已非关键字」；
   其余 funintro 测试全删。
7. **验收**：`sokonanoda gate`（fmt/clippy/test/playground 锚点）。

## 本轮进度（2026-09-14，第四十九轮：多目标显示）

> 用户报告：画布上 `apply And.intro; intro x` 之后应同时看到 `P x` 和
> `(x : Person) -> Q x` 两个待证目标，目前只显示一个。用户确认按完整流程修。
> 判定逻辑本就正确（`apply` 确开两个 `forall` 子目标），缺陷在**引擎数据**：
> `ByStep` 每步只记 worklist 栈顶目标，其余目标编译期即丢。随后用户报告
> VS Code 目标视图很卡，同轮修刷新路径。

1. **设计先行** `docs/design/goal-list.md`（根因 / 方案 / 测试 / 验收 / as-built）。
2. **前端**：`ByStep` 改为 `{ span, goals: Vec<ByGoal> }`，`ByGoal = { ty,
   binders }`；每步记**全部**未闭合目标（当前在首位，各带自己的假设链），
   闭合则为空。`by_step_states` / `ByGoalState` / re-export 同步。
3. **协议**：`soko/stateAt` 增 `goals: [{goal, binders}]`；`soko/goals` 每
   声明增 `goals: [String]`（by 声明取最后一步、非 by 取走查目标）。
   单值 `goal`/`binders` 保留且恒等于 `goals[0]`，旧客户端不回归。
4. **客户端多目标**：VS Code「当前光标处」与练习节点遍历多目标——>1 时渲染
   `目标 i/n` 可展开节点、各自挂假设；=1 保持现状。
5. **客户端性能**（用户报告「vscode 很卡」）：原实现每次光标移动都
   `refresh()` → 重取 `soko/goals` + 重建全部练习 TreeItem。改为
   `refresh()` 只处理诊断/切文件，光标移动走 `refreshCursor()` 复用缓存
   `declItems`，只重建光标组。
6. **测试三层**：front `apply_records_all_open_goals_current_first`（+改两旧
   用例读 `goals`）；LSP `state_at_lists_all_open_goals_after_apply` +
   `goals_request_lists_every_open_goal_after_apply`；CLI 契约
   `extension.rs` 钉客户端消费 `cursor.goals`/`decl.goals` 与 `declItems`/
   `refreshCursor` 缓存纪律。
7. **版本** 0.26.0 → **0.27.0**（Cargo + VSIX + CHANGELOG Added）；`docs/protocol.md`
   两节 + 已知限制小节、`REQUIREMENTS.md §9` 同步。
8. **验收**：`sokonanoda gate` PASS（fmt / clippy / test / playground 锚点）。

## 本轮进度（2026-09-14，第四十八轮：量词课程——单元⑦ + 画布第二课）

> 用户提出：教程里逻辑内容偏少，缺 forall/exists 题目，参考 Metamath 出题；
> 随后要求同步加进 `course/` 并提交推送。判定全部走内核。

1. **画布第二课**（`playground.sokonanoda`）：∀=依赖函数类型（引入写 fun、
   消去写应用、命名箭头等价）；`Exists` 按单元①的老办法立成公理三件套
   （intro 交证人、elim 交给函数且结论不提证人）；`Person`/`someone` 论域
   + 两个已填演示 + `#reduce` 自测 + 7 道练习（练习 6–12，含 ★/★★）各挂
   三层 `soko:hint`；7 份钥匙全部经完整内核验证。
2. **course 单元⑦**（`unit7-quantifiers.sokonanoda`）：同题重编号 1–7，自带
   逻辑骨架（True/And）与 `Exists` 公理，中文画布 + `solutions/` 钥匙 +
   英文镜像（代码逐字节一致、事件计数一致），golden `(14, 7, 1)`；
   `course.json` unit=1..7、`course/README.md` 七个单元/1..7 同步。
3. **守卫与文案**：`course.rs` golden +「seven units」；`course_status.rs`
   golden + 汇总 `units=7 / checked=47 / open=34`；根 README、site
   `index/course/en` 三处静态文案、teacher `curriculum.md` 单元表、
   `docs/teaching-session.md`（§3 新增第二课钥匙表、§5 单元⑦）同步。
4. **版本** 0.25.0 → **0.26.0**（Cargo + VSIX 两处；CHANGELOG Added）。
5. **验收**：`cargo fmt --check` 0；`cargo clippy --workspace --all-targets`
   教学 crates 0 告警（kernel 保持 warning）；`cargo test --workspace --locked`
   **545 passed / 0 failed**（8 ignored）。

## 本轮进度（2026-09-14，第四十七轮：sorry 洞期望类型精确化）

> 用户报告：练习 5 `(And.right a (Not a) x) sorry` 的 hover 显示整个声明
> 类型，应显示洞的期望类型 `a`（用户以 `((…) sorry : a)` 说明）。

1. **根因**：goal 走查（func_spine_case）只覆盖声明望远镜内的实参；
   `And.right` 全量应用后结果 `Not a`，`sorry` 是它的函数实参——超量应用
   直接 `return None` → generic fallback 用整个声明类型当目标。
2. **修复**（goals.rs）：FuncTemplate 增加 `result_ty`（望远镜剥完的残余）
   与 `def_body`（仅 def）；超量应用时把结果类型按 def 体逐步展开
   （`Not a` ⇒ `a -> False`），继续按箭头匹配剩余实参 → 洞期望 = 箭头
   定义域 `a`。剩余目标 `False`、假设 a/x 一并展示。
3. **hover**：decl_at 的 Open 分支在光标落在洞上时优先显示
   「此处 sorry 的期望类型」+「剩余目标」。
4. **测试**：front `overapplied_spine_through_def_shows_hole_expected_type`
   （goal="False"、sub_goals[0].ty="a"）+ LSP
   `hover_on_sorry_in_overapplied_spine_shows_hole_expected_type`。用户
   案例按其原话钉成单元测试。
5. 验收：545 passed / 0 failed（+2）；clippy 0；真实 LSP 协议跑
   playground 确认 hover 输出正确。版本 0.24.0 → **0.25.0**（crates 改动
   必须随 commit bump 版本——LESSONS 铁律）。

## 本轮进度（2026-09-13，第四十六轮：性能测试例行化）

> 用户要求：性能测试例行化、覆盖全面+细致（编译器 + VS Code 插件特性）、
> 每版本可见、回归时能定位到哪个改动。

1. **阈值断言哨兵**（`crates/front/tests/perf.rs` 3 个 +
   `crates/lsp/src/lib.rs` 3 个，随 `cargo test --workspace` 例行执行）：
   编译器缩放比（400/50 块 ≤12×，O(n²)=64× 必红）、增量编辑每键 <50ms
   且 kernel_checks≤1、编辑首练习不随文件长度超线性；LSP didChange
   round-trip <50ms、completion/hover/goals 各 <10ms（50 块文件）。
2. **每版本留档**：CI "Performance report" 步骤提取 PERF 行 →
   `perf-report-v<version>-<sha>.txt` artifact（每次 push 都有）；
   本地同口径 `scripts/perf-report.sh`。对比相邻版本报告即可定位回退
   场景 → git log 找改动。
3. **阈值设计原则**：只抓算法级回归（线性理论值 ×1.5 余量），CI 噪声
   不误报；绝对延迟抓用户可感劣化。设计文档 `docs/PERF.md`（含基线）。
4. 扩展层无独立计算路径——所有特性经 LSP，故覆盖在 LSP 请求层
   （didChange/completion/hover/soko-goals）。
5. 验收：cargo test --workspace 543 passed / 0 failed（+6 perf）；clippy 0。
6. 版本 0.23.0 → **0.24.0**：纯基建无功能面变化，但用户要求每轮工作
   有独立版本号（性能报告按版本对比）；CHANGELOG 以 Development/Infrastructure
   节记录。

## 本轮进度（2026-09-13，第四十五轮：性能收口 + goal-state hover）

> 用户三点：性能是生命线（排查 funapply 同源问题与内存）；半截表达式
> （`And.intro b a`）hover 给 goal state；GIF 例行化与最新更新同步。

1. **judge 结果缓存**（`judge.rs`）：judge_infer/terms/hole_fill 的每次
   调用都重编译整个文档前缀——by 块 tactic `apply`/`exact` 让该成本落在
   每一次按键（与 funapply 同根，未随其移除而消失）。指纹缓存（前缀文本 +
   options + context + terms），封顶 128 条；前缀参与指纹 → 更早编辑失效
   缓存，保守但正确。命中返回与直算逐字节一致（缓存测试钉住）。
2. **半截表达式 goal-state hover**：内核拒绝的声明（如 `And.intro b a`
   差两个前提），hover 显示推断出的剩余目标 `|- b`、`|- a`——只在 hover
   请求时计算（judge 缓存命中后零成本），不在按键路径。值文本取自声明
   切片最后一个 `:=` 之后；context = 值自己的 lambda 链（学习者命名原样）。
3. **内存审计结论**：LSP 侧单 Document 结构、报告整体替换（无历史累积）；
   judge 缓存封顶；扩展端命令/监听一次性注册——未发现无界增长。判定
   结果缓存同时消除了每次按键为每个 by 块分配整套编译报告的抖动。
4. **演示例行化**：`gen-site-demos.py --check`（帧指纹比对）进 pages
   workflow——交互/文案改了而没重跑渲染脚本时 CI 红。演示已无 funapply。
5. 版本 0.22.0 → **0.23.0**（goal-state hover = 新能力）。

## 本轮进度（2026-09-13，第四十三轮：值位关键字 v2）

> 用户三需求：①输入过程没有补全替换提示；②`funintro (funapply X)` 组合；
> ③值位关键字改名与 tactic 消歧义。设计 `docs/design/value-keywords-v2.md`
> （探针实测 + 两路调研 + 决策单 D1–D9），S1–S3 由 subagent/主会话顺序实施。

1. **S1 改名**（commit 966117c）：值位 `intro`→`funintro`、`apply`→`funapply`
   （无别名）；by 块 tactic 不动；KEYWORDS 保留旧词加新词；按钮文案
   「替换源代码 funintro/funapply」；hint 改新名、机器码沿用；课程/画布/
   编辑器/文档全扫（~30 文件）。执行事故：subagent 跑 51 文件后 429 中断且
   误跑了 kernel 全仓 fmt——已回滚 kernel、修复 7 处残留 + BY_OPEN 夹具的
   tactic 误改名。
2. **S2 输入期补全**（4cc3f70）：探针实测根因（keyword_at 要求 Open，输入
   中间态必然 Failed，前缀阶段连 decl 都没有）→ 补全两态：骨架态（原行为）+
   键入态（前缀/整词/尾随空格出「替换源代码」项，newText = 关键字全词）。
   v1 整词门控明确推翻；三条逐键真人输入测试（零 sleep）。
3. **S3 组合**（4603888）：关键字成为原子位表达式（parse_atom）；`lower_at`
   提为 pub(crate)；`lower_intro_val` 增加 src/span_start/options 与 context
   贯穿；`funintro (funapply X)` → `And.intro b a sorry sorry`（σ 实例化 +
   前提洞），内核零感知；等价性契约 + 错误面沿用既有码。
4. **S4 收口 + 全自动发布**（b255991 → auto-tag 自动打 v0.21.0 → release
   run 全绿）：Release 25 资产、Marketplace 收录 0.21.0、官网同步——**首次
   零人工发版**，上一轮的 auto-tag 链实战验证通过。

## 本轮进度（2026-09-13，第四十二轮：文档治理）

> 用户要求：当前问题解决后，清理整理项目文档，防技术债，让后续的人 / code
> agent 高效接手。三路并行只读审计（根目录五份入口 / docs 顶层 / design+notes
> 全部 35 篇）后集中修复约 35 处。

1. **发布口径统一**：`docs/RELEASE.md` 主口径改为「bump → push main → auto-tag
   自动发版」，手动推 tag 降为应急；`AGENTS.md`/`vscode-dev-guide.md`/
   `docs/README.md` 同步（此前四处口径只有 skill 写对了）。
2. **矛盾清零**：TESTING「LSP 零测试/assumption 文本比对待替换/known_missing
   未闭环」三处自相矛盾；architecture §5.5 与 §8.8 矛盾；REQUIREMENTS §2.8
   过时括号注；teaching-session「单元⑤待支持」；binary-cli 的 `env` 父命令与
   `minreq` 两处失实（横幅更正）。
3. **状态横幅**：12 篇 design/notes 补 as-built / 快照横幅（term-apply 自称
   「只落设计」实则 0.18.0 已发布——最严重漂移）；docs/README 地图三条
   「待实施」改「已实现/已上线」。
4. **协议补全**：protocol.md 增 `soko/version` 小节（five custom requests）；
   `LSP_CUSTOM_METHODS` 词汇表补第 5 项（恢复守护）。
5. **STATUS 归档**：1–39 轮原文移 `docs/STATUS-ARCHIVE.md`，本文件只留
   最近 3 轮；gen-site-data.py 只解析最新轮标题，归档无破坏（已验证）。
6. **杂项**：`.gitignore` 加 `.workbuddy/`；README 门面补值位三关键字示例；
   ROADMAP §8「执行 M0」等陈旧定位更新、I10/I12/I11-S1 打勾标注版本；
   REQUIREMENTS §4 欠账更新为当前真实数字（lsp/lib.rs 3949 行、
   compile/tests.rs 3564 行）、§8 路线改指 I11 余项、§9 补四十二。

## 本轮进度（2026-09-13，第四十一轮：发布链自动化闭环）

> 用户复盘发布流程后发现三个流水线缺口，全部修复并落地自动化。本机 `gh`（owner
> 权限）可用后，此前「只能网页手点」的动作全部自动化了。

1. **auto-tag 自动发版**（`ci.yml` 新 job）：main 上 lint/test 全绿后，若
   `Cargo.toml` 版本还没有对应 v* tag → 打 tag 并 `workflow_dispatch`
   release.yml。机制：GITHUB_TOKEN 推的 tag 不触发其它 workflow（防递归），
   `workflow_dispatch` 是例外；dispatch 需要 `actions: write`（首跑 403 教训）。
   幂等：tag 已存在即跳过；两处版本不一致直接 fail。
2. **release.yml 两处 `if: github.event_name == 'push'` 改为按 ref 判**——
   dispatch 进来核心步骤（创建 Release/上架 Marketplace）被静默 skipped 而
   job 依然绿，v0.20.0 首发实况。
3. **pages 门禁改鉴权 `gh api`**——未鉴权 curl 的假 404/403 让门禁误判
   「未启用」→ 部署步骤全 skipped，站点迟迟不上线。
4. **发布结果**：v0.20.0（tag 强移到含修复的 commit，版本契约核验）→ Release
   资产 25 个（9 VSIX + 8 LSP tar + 8 CLI tar）✓；Marketplace 已收录 0.20.0
   （首跑遇 Azure gallery 503，`gh run rerun --failed` 恢复后成功）✓；站点
   https://colorlessboy.github.io/sokonanoda-lang/ 上线（About 的 homepage
   指向它）✓；About 三件套（description/website/topics）已用 gh repo edit
   填好。
5. **教训落台账**：`docs/CI-FAILURES.md` +2（skipped 步骤把 job 抬绿；gallery
   503）；`skills/sokonanoda-ci` §1/§2.1 +4 行。

## 本轮进度（2026-09-13，第四十轮：三个大方向落地）

> 承接第三十九轮的设计（`docs/design/term-apply.md` / `real-input-tests.md` / `site.md`），
> 用户确认「三条串行进行」后按依赖顺序执行：I11-S0 → I11-S1 → I10 → I12。

1. **I11-S0 真人输入脚本基建**（`lsp/src/testutil.rs`）：`TypedStep`
   `(offset, delete, insert)`（纯输入/纯删除/**选中重打**三种形态）+ `type_step`
   （一步一次 didChange + 等诊断落地，`refresh` 每次都发诊断 → **零 sleep**）+
   `char_steps`。两条用例：逐字符敲 `intro` 只在整词给展开项；「选中重打 + 折行」
   每个中间态都能拿到展开项。**教训**：最初设计成「跑完整条脚本返回各步快照」，
   而服务器只持有最新状态——据此写出的是**看起来会通过的假测试**，已改为
   「一步一断言」并写进设计文档 §2。
2. **I11-S1 修复 `by apply` 多子目标的 inlay 类型错配**：这些子目标在源码里
   **共用同一个位置**（`assemble` 给每个叶子洞传同一个 `hole_span`），旧实现用
   span 反查 `sub_goals`，两处都显示第一个子目标的类型（实测 `[": p", ": p"]`）。
   改为**按洞的位置顺序**对齐（红→绿）。`soko/nextHole` 无法在同址子目标间导航
   属**确认为限制**（伪造互异 offset 会让 documentHighlight/selectionRange 出假
   范围），已写进 `docs/protocol.md`。
3. **I10 值位 `apply`**：`theorem t (h : Q -> P) : P := apply h` → `h sorry`。
   设计里的关键取舍全部落地——内核只用来**推断**被应用名字的类型（`judge_infer`，
   front 侧无类型表可用），telescope 机械抽到 `crates/front/src/spine.rs` 与
   tactic `apply` **共用**；局部假设（最常用场景）经 `open_goal` 的局部模板覆盖层
   被 spine 走查认出；合成洞一律走 `judge_terms` 绕开 `judge_hole_fill` 的
   sorry 守卫；类型参数由目标实参经 σ 填充；实参用 `parse_app`（括号界定范围）；
   洞 span 覆盖整个 `apply <term>`。LSP 侧 `intro_at` 泛化为 `keyword_at`（第二
   个调用方出现），hover/补全/`sokonanoda.expandApply` 全套接入。两个新错误码。
4. **I10-S4 课程**：第 6 单元新增「值位 `apply`」一节（三种写法对照 + 2 道练习 +
   钥匙），英文镜像非逐字对齐。golden **刻意**变更：unit6 事件计数
   `(13, 6, 0)` → `(17, 10, 0)`，汇总 `33/27` → `37/31`（`course.rs` 与
   `course_status.rs` 同步）。
5. **I12 官网**：零构建静态 `site/`（9 页 + CSS + `site/data/site.json`）+
   `scripts/gen-site-data.py`（python3 标准库：版本←Cargo.toml、单元←course.json、
   轮次←STATUS.md）+ `scripts/check-site.py`（站内链接 + 禁止写死版本号）+
   `.github/workflows/pages.yml`（configure-pages / upload-pages-artifact /
   deploy-pages）。**一次性前置动作仍需仓库拥有者**：Settings → Pages → Source =
   GitHub Actions。顺带修 4 处文档漂移（README 写死 `V=0.9.0`、课程单元数
   5 vs 6 的三处口径）。
6. **版本**：0.17.0 → **0.18.0**（新语法 = 新能力 → minor）。

## 本轮进度（2026-09-13，第三十九轮：三个大方向的设计——值位 `apply` / 真人输入测试 / 官网）

> 触发：用户提出三个大方向——①照 `intro` 的模式新增值位 `apply`（要补全 + 等价部分
> 表达式，必要时用括号界定范围）；②设计「真人相同输入」测试，覆盖 `sorry`/`intro`/
> `apply`/`by` 共存；③做 GitHub Pages 官网介绍用法、远大目标、当前进展。并要求
> **先调研、头脑风暴、规划、做好文档，再启动分步骤计划、多用 subagent**。
> 本轮按仓库「设计先行」硬规则，**只落设计，不动实现**。

1. **调研**：并行派了 4 路只读 subagent（`intro` 端到端触点与 `apply` 可行性 /
   LSP 触点全图 / 输入测试现状与共存风险 / Pages 方案），全部报告进设计文档的证据表；
   第一路的结论**推翻了乐观假设**（见 2）。
2. **`docs/design/term-apply.md`（值位 `apply` 设计）**：最关键的一条——**不能照抄
   `intro`**。`intro` 只需要声明类型（已在 AST），而 `apply` 需要**被应用名字的类型**，
   front 侧根本没有可用来源：`GoalTemplates` 的 `peel_type` 丢掉了 codomain
   （`goals.rs:249-267`）、局部假设（最常用的教学场景）不在表内
   （`goals.rs:531-534`）、`EnvBuilder` 无类型查询 API（`kernel/builder.rs:234`）。
   决策：降低走 `by` 引擎已验证的 `judge_infer` 路线（内核只用来**推断类型**，不做
   判定；判定仍在填洞后的合成声明上）——这是对 `intro`「值不进内核」的**刻意偏离**，
   文档里写明理由以免后人误判为违规。另定：实参用 `parse_app` 消费（括号已支持，
   无需新语法）、三个新错误码、以及一条硬风险——`apply` 展开成 `h sorry` 是 App spine，
   会产出非空 `sub_goals`，从而踩到 `judge_hole_fill` 的「洞位源码必须恰为 `sorry`」
   守卫（`judge.rs:316-322`），必须把**合成洞**路由到 `judge_terms`。
3. **`docs/design/real-input-tests.md`（真人输入测试）**：三层设计（front 版本序列 /
   LSP `didOpen→didChange` 输入脚本 / VS Code 真实手势），新增 `type_script`/`type_chars`
   基建把「输入序列」变成被测对象；给出共存风险矩阵与 flake 预算（LSP 层零 sleep）。
   **并发现一个既有缺陷**：`by` 的末个 tactic 是 `apply` 且留下 ≥2 个子目标时，
   所有洞 span 都等于「最后一个 tactic 的 span」（`by.rs:263/375` + `269-271`），
   导致 nextHole 跳不动、inlay 叠位且类型可能错——当前**零覆盖**，已排为 I11-S1
   （先红测试再修）。
4. **`docs/design/site.md`（官网）**：方案定为**零构建手写静态 `site/` + GitHub Actions
   部署**（不用 `docs/` 做 Pages 源：那会把 35 篇内部台账公开、还触发 Jekyll 隐式渲染）；
   「版本 / 进展 / 单元数」一律由 `STATUS.md` 机器可读块 + `Cargo.toml` + `course.json`
   + Releases API 生成，官网只做视图不做第五个事实源；一期零后端（主 CTA = 装 VS Code
   扩展），WASM playground 入 backlog。并列出 6 条**已确认的文档漂移**
   （README 写死 `V=0.9.0`、课程单元数「5 vs 6」三处口径矛盾、ROADMAP/文档索引陈旧）。
5. **计划**：`ROADMAP.md` §10 新增 **I10 / I11 / I12** 三个分阶段条目，含每阶段验收
   标准与 subagent 切分（文件集互斥、公共接线点由主会话先落地）；
   `REQUIREMENTS.md` §9 追加第四十条。**阻塞项**：官网需仓库拥有者在
   Settings → Pages 手动把 Source 设为 GitHub Actions，否则 workflow 报
   `Get Pages site failed. Not Found`。

## 本轮进度（2026-09-12，第三十八轮：`intro` 词尾命中修复 + hover 展开按钮 + 「不替换也等价」契约）

> 触发：用户反馈 `playground.sokonanoda:201` 上「同一行敲 `intro` 正常，把值
> 折到下一行就没用了（那行会太宽）」；并提两个新要求——hover 加一个直接替换
> `intro` 的按钮（等价 Tab 补全）、`intro` 不被替换也直接等价于 `fun` 表达式。

1. **根因（不是换行，是光标在词后）**：旧命中判据是洞的**闭区间字节范围**
   `[start, end]`。学习者敲完 `intro`，光标停在词尾、或顺手多打一个空格想
   关掉弹窗，就落到区间外——补全与 hover 一起消失。折行只是让「停在词尾」
   更容易发生（同一行 `:= intro ` 也是同一个 bug，此前 36/37 轮只修了
   补全覆盖面、hover 没同步）。
2. **修复**：命中区间扩到「token 本身 + token 之后到**同一行行尾**的空白」，
   `trailing_same_line_ws` 判据；hover 与补全抽成**同一套** helper
   （`intro_hit` / `intro_at`），编辑器不扫文本、不各自重算。跨行不算，
   免得在后面的声明上误弹。
3. **hover 展开按钮**：hover markdown 带
   `command:sokonanoda.expandIntro?<payload>`；载荷（uri + 洞 range + 骨架）
   由服务端算好，编辑器照单应用一次 `WorkspaceEdit`——与接受 Tab 补全是
   **同一份编辑**，不会分叉。客户端三处配套：`markdown.isTrusted` 只放行
   这一个命令（白名单，不是整体 `true`）、`contributes.commands` 声明、
   命令面板隐藏（`when: false`）。
4. **「不替换也等价」钉成契约**：hover 明写「不替换也完全等价」；front 新增
   `intro_is_equivalent_to_typing_the_skeleton_out_by_hand`（同 status/goal/
   binders/洞数，差别只有骨架字段）与 `value_intro_is_layout_independent`
   （同页 / 换行 / 尾随空格三种排版判定一字不差）。
5. **发布**：0.16.2 → **0.17.0**（新增命令 = 新能力 → minor），tag `v0.17.0`
   → commit `52a6f22`。`release` run 34703048635 **全绿**：8 平台 `build` →
   `package-vsix` → `github-release` → `marketplace-publish`；GitHub Release
   资产 **25 个**（lsp tarball ×8 + cli tarball ×8 + VSIX ×9），Marketplace
   `sokonanoda-lang.sokonanoda` 已上 **0.17.0**；同 tag 的 `ci` 也 success。
   发布后额外做了资产冒烟：darwin-arm64 的 lsp/cli tarball 下载→解压→
   `0o755` 保住、发布的 CLI 直接判 `playground.sokonanoda` exit 0。
   （双页核对是 2026-09-11 那次「Release 零资产」事故留下的硬性预防。）

## 本轮进度（2026-09-12，第三十七轮：`intro` hover 展开式 + Tab 接受回归）

> 触发：用户要求「不选择补全时，hover `intro` 显示展开的表达式」，并反馈
> 按 Tab 没看到替换。

1. **hover**：值位 `intro` 的 hover 显示展开后的显式表达式（token 内或
   词尾都命中），返回洞 range 高亮；放在关键字抑制之前（`intro` 现在是
   `KEYWORDS`，否则会被当普通关键字吞掉）。
2. **接受路径复现**：真实 VS Code 集成新增两个确定性用例——「接受选中
   建议 → token 原地替换成骨架」与「type `intro` → 接受」（12 passing，
   连跑两轮稳定），确认 0.16.1 的机制本身可用；用户未生效大概率是窗口
   还在跑旧扩展/LSP。
3. **实验与回退**：试过补全响应标 `is_incomplete: true` 强制客户端逐键
   重查；实测让 VS Code 的选择/接受行为退化（用例转红），已回退不采用。
4. **发布**：0.16.1 → **0.16.2**（hover 信息 patch）。

## 本轮进度（2026-09-12，第三十六轮：修复 `intro` 补全在词尾不触发）

> 触发：用户实测 `playground.sokonanoda:201` 输入完 `intro` 看不到展开补全。

1. **根因**：补全门控用 `render::decl_at`（区间**末尾排他**，`render.rs:70`），
   而输完关键字时光标恰在声明/token 末尾 → 不命中。旧 LSP 测试都从 token
   **起点**请求，掩盖了真实输入位置。
2. **修复**：门控改为字节区间（末尾含）`start <= offset <= end`，洞区间也
   含末尾；两个补全测试改为在 token 末尾请求（回归守护）。
3. **验证**：真实 LSP 探针在 `playground:201` 现在返回展开项（`textEdit`
   就是 `fun (a : Prop) => fun (b : Prop) => fun (x : And a b) => sorry`）；
   VS Code 集成新增 `value intro offers the expansion completion at the end
   of the token`（9 passing）。
4. **发布**：0.16.0 → **0.16.1**（bugfix → patch）。

## 本轮进度（2026-09-12，第三十五轮：VS Code 手动重启 LSP 命令）

> 触发：用户发现扩展/LSP 更新后旧进程不生效，要求插件提供手动重启语言
> 服务器的命令。

1. **命令**：`sokonanoda.restartServer`（命令面板「sokonanoda: 重启语言
   服务器」）——先重解析二进制路径（重建的仓库构建 / 刷新后的缓存 / 改
   `serverPath` 都生效），原地更新 `serverOptions` 后 `client.restart()`；
   无需重载窗口。扩展本体升级仍需 Reload Window（README 与开发指南写明）。
2. **门面**：README 功能清单 + `docs/vscode-dev-guide.md` §5.6 同步；
   CHANGELOG 0.16.0。
3. **测试**：静态契约 13、extension 单测 25、VSIX 冒烟、集成 8（含新
   `restart server command re-syncs open documents`）。
4. **发布**：0.15.0 → **0.16.0**（feature → minor）。
5. **发布结果**：tag `v0.16.0` → ci / release 全绿（8 平台 build +
   package-vsix + github-release 25 资产 + Marketplace `0.16.0` ×9）；
   本地缓存刷新到 0.16.0（`doctor` ready）。

## 本轮进度（2026-09-12，第三十四轮：声明级 binder（Lean 风格））

> 触发：用户要求支持官方 Lean 的
> `theorem and_swap2 (a : Prop) (b : Prop) (h : And a b) : And b a := sorry`
> ——省去 `intro`/`fun` 的麻烦。设计 `docs/design/decl-binders.md` +
> REQUIREMENTS §9（三十七），同日实现。

1. **parser**：声明 binder（显式/隐式/多名字组）解析 + 宇宙参数消歧
   （`{u}` vs `{x : T}` 的 lookahead）；`wrap_decl_binders` 降级为
   「类型 = Forall 望远镜、值 = Lambda 望远镜」，后续流水线零改动；
   无类型 binder（`(a)`）给教学 parse 错误。
2. **`by` 引擎**：`run_by` 新增 `initial_binders`——声明 binder 进根节点
   上下文、类型先剥对应层数；`split_by_value` 识别「lambda 链 → by」。
3. **`intro` 递归**：沿 lambda 链下降，只剥剩余 codomain；补全骨架只含
   剩余展开（`theorem t (a : Prop) : a -> a := intro` →
   `fun (x : a) => sorry`）。
4. **体验**：`:= sorry` 目标即 codomain、上下文含声明 binder（不用
   intro）；闭合正文直接写、不用 fun；两种拼写内核判定等价。
5. **课程**：unit1 补「声明级 binder」小节 + 练习 6（zh/en 镜像 + 钥匙），
   golden unit1 (13,6,1)。
6. **测试**：front 265 / lsp 98 / cli 51 / course 4 全绿（parser/降级/宇宙
   消歧/by 上下文/intro 组合/inlay/补全/CLI e2e）。
7. **发布**：0.14.0 → **0.15.0**（feature → minor）。
8. **发布结果**：tag `v0.15.0` → ci / release 全绿（8 平台 build +
   package-vsix + github-release 25 资产 + Marketplace `0.15.0` ×9）；
   本地缓存 `sokonanoda update` 刷新到 0.15.0（`doctor` ready）；用发布后的
   缓存二进制跑用户原例：`:= sorry` → Open、闭合正文 → checked。

## 本轮进度（2026-09-12，第三十三轮：值位 `intro` 关键字 + 展开补全）

> 触发：用户要一个「类似 intro、和 by 同级」的确定性补全（拒绝 Copilot
> 式 AI 补全的作弊感）。设计先落 `docs/design/term-intro.md` +
> REQUIREMENTS §9（三十六），同日设计 + 实现。

1. **前置修正（旧缺陷）**：`render_expr` 函数位置不再给应用头补括号
   （`(And a) b` → `And a b`，与内核 pp / 学习者源码一致；6 处被钉断言与
   `docs/protocol.md` 示例同步）；session 零重编译路径补 remap
   `holes`/`sub_goals` span，并加回归测试。
2. **语法/语义**：值位 `intro`（`Expr::Intro`，与 `by` 同挂点，`And.intro`
   不受影响）；全剥声明类型最外层 Pi 望远镜成显式 lambda + 洞
   （`compile/intro.rs`；匿名层 `x`/`x2`，命名 helper 提取为
   `proof::fresh_name` 与 `suggest::restart_skeleton` 同源）；合法 Open、
   值不进内核；非函数目标稳定码 `elab-intro-not-a-function`；
   `DeclState.intro_skeleton` 贯通。
3. **编辑器**：`textDocument/completion` 在 intro token 上门控给一项
   `intro（展开为 fun 骨架）`（`textEdit` 原地替换、PlainText、markdown
   文档）；inlay 在 intro 后直接显示剩余目标；`KEYWORDS` 加 `intro` 高亮；
   VS Code 扩展零改动。
4. **课程**：unit6 补「值位 intro」小节 + 练习 6（zh/en 镜像 + 钥匙），
   golden (13,5,0) → (13,6,0)。
5. **测试**：front 255 / lsp 96 / cli 48 / course 4 全绿；新增 parser、
   降低、命名、非函数、零内核、补全、inlay、session、CLI e2e 用例。
6. **文档**：protocol（错误码 + intro/补全一节）、architecture §4.1 值位
   关键字、TESTING 测试地图、teacher skill 决策表与课程地图。
7. **发布**：0.13.0 → **0.14.0**（feature → minor；扩展无代码改动但内嵌
   LSP 更新，VSIX 版本随 workspace 同步）；CHANGELOG 补 0.14.0 条目。
8. **发布结果**：tag `v0.14.0` → ci / release 全绿（8 平台 build +
   package-vsix + github-release 25 资产 + Marketplace `0.14.0` ×9）；
   本地缓存 `sokonanoda update` 刷新到 0.14.0（`doctor` ready）。

## 本轮进度（2026-09-12，第三十二轮：扩展残留清理 + 插件 LSP 解析加固）

> 触发：用户发现 `~/.vscode/extensions` 里 0.2.1/0.4.2/0.5.3/0.12.0/0.13.0
> 五份 sokonanoda 共存，问插件是否缺清理逻辑。排查：版本目录全由 VS Code
> 管理（profile 各自记账、惰性删旧），插件缓存固定路径单版本，无清理缺陷；
> 真正瑕疵是解析扩展自带 LSP 只按 mtime 取新（不认版本号、不跳过待删目录）。

1. **机器清理**（本机 VS Code，非仓库改动）：默认 profile 的 0.5.3 从
   Marketplace 升到 0.13.0，与 `macos` profile 共用同一目录；删除其余四个
   孤儿目录，只留 `sokonanoda-lang.sokonanoda-0.13.0-darwin-arm64`。
2. **插件加固**：`.opencode/plugins/sokonanoda.ts` 的 `extensionServer`
   改为按目录名版本号取最高（无版本号才回退 mtime），跳过 `.obsolete`
   标记待删目录；新增 `entryVersion`/`obsoleteEntries` 两个纯函数。
3. **验证**：Node 24 临时回归脚本（mock HOME + 假扩展树）10 项断言全绿
   （版本优先、obsolete 跳过、无版本回退、解析边界）。
4. **文档**：`docs/design/onboarding.md` 解析链补注；REQUIREMENTS §9（三十五）。
5. **教学**：`playground.sokonanoda` False 段讲解改写（一句一行，点明
   「零构造子 + 消去规则 = False 的完整定义」）。

## 本轮进度（2026-09-12，第三十一轮：opencode 插件迁移官方 `plugins/`）

> 触发：opencode 官方项目级插件目录改为复数 `.opencode/plugins/`（用户
> 指出机制已换，要求删旧文件并按最新 skill 描述重配）。旧单数目录
> `.opencode/plugin/` 不再使用。

1. **迁移**：`git mv .opencode/plugin/sokonanoda.ts →
   .opencode/plugins/sokonanoda.ts`，删除旧目录；插件行为不变——启动
   best-effort provision CLI/LSP、`config` 钩子接线原生
   `lsp.sokonanoda`（`opencode.json` 仍不写 lsp 命令）、`shell.env` 注入
   缓存目录到 PATH；`findRepoRoot` 的仓库标记改指
   `.opencode/plugins/sokonanoda.ts`。
2. **契约**：`crates/cli/tests/opencode.rs` 改读新路径，并新增断言
   「`.opencode/plugin`（单数）必须不存在」，防止旧目录回潮；其余
   （shim、namespaced commands、skills.paths、无 lsp 块）断言不动。
3. **文档**：REQUIREMENTS §9（三十四）、`docs/TESTING.md`、
   `docs/notes/rust-cross-platform-binary.md`、`docs/design/onboarding.md`
   路径同步；非 opencode harness 的 `.opencode/lsp/sokonanoda-lsp.sh`
   shim 按 skill 描述保留。
4. **验证（重启后）**：`opencode debug config` 确认新路径被识别
   （`plugin_origins` 指向 `.opencode/plugins/sokonanoda.ts`）且
   `lsp.sokonanoda` 已接线；三个 skill / 7 个 `/sokonanoda/*` 命令 /
   `teacher` agent 全部加载；`shell.env` 注入生效（`which sokonanoda` →
   缓存）；契约测试 8/8、skill 4/4 全绿。
5. **环境修复（重启后验证发现）**：缓存 `sokonanoda-lsp` 停在旧版
   （marker `0.5.2`，`doctor` 未就绪），且插件优先的仓库构建
   `target/debug/sokonanoda-lsp`（9-10 旧二进制）不支持 `Type n`，把画布
   `axiom Prop : Type 0` 误报为 `kernel-expected-pi` error。修复：
   `sokonanoda update` 刷新缓存到 0.13.0（`doctor` ready）+ `cargo build
   -p sokonanoda-lsp` 重建仓库构建；LSP 诊断恢复为 5×`sorry` +
   1×`reserved-declaration-name`（全 warning、零 error），与内核 CLI 一致。

## 本轮进度（2026-09-11，第三十轮：发版 0.13.0 + opencode 全量初始化）

> 触发：用户发现缓存里的 `sokonanoda` 报 `os error 2`——v0.12.0 tag 停在
> onboarding 二进制提交之前，Release 里的 CLI 比仓库旧；用户要求「先发版，
> 再继续配置」，让编译好的新 CLI 可直接下载，不要本地 cargo。

1. **版本**：`Cargo.toml` + `editor/vscode/package.json`（+ `package-lock.json`）
   由 0.12.0 → **0.13.0**；CHANGELOG 补条目；`Cargo.lock` 同步。
2. **发版**：预发布校验（fmt/clippy/test/playground）全绿后 commit + push，
   打 `v0.13.0` tag 推送；release 流水线编 8 平台 + 9 VSIX + 25 个 Release 资产。
3. **opencode 初始化**：`sokonanoda update` 按锁定 `v0.13.0` 刷新缓存
   （CLI+LSP 均为编译产物，`version`/`doctor --json` 全 match、ready）；
   三个 skill 软链到 `~/.agents/skills`（镜像 `~/.claude/skills`）；
   plugin/commands/agent/shim 校验在位。
4. **Marketplace**：`v0.13.0` 的 `marketplace-publish` 曾因 Marketplace
   认证端点（`/_apis/gallery`）后端故障连续超时（非账号风控；Release 25
   个资产不受影响，见 `docs/CI-FAILURES.md` 2026-09-11）。服务端恢复后本机
   `vsce publish --skip-duplicate` 补发 9 个 VSIX、CI rerun 转绿，
   Marketplace 已上线 `0.13.0`。

## 本轮进度（2026-09-11，第二十九轮：环境能力进二进制，删除 soko.sh）

> 触发：用户要求环境能力做成二进制 CLI，拒绝 `scripts/soko.sh`（Windows
> 不可用、维护面大）。设计见 `docs/design/binary-cli.md`。

1. **CLI 子命令**（`crates/cli/src/env/`，内嵌下载器）：`version`/`doctor`/
   `setup`/`update`/`grade`/`gate`（沿用 `lsp`）；`build.rs` 在编译期钉死
   `SOKONANODA_TARGET`，按 `<pkg>-<triple>.tar.gz` 下载并写
   `<version> <vsce-target>` 标记（与 VSIX/插件一致）；HTTP/TLS + 解压用
   `ureq`(rustls/ring) + `flate2` + `tar`，无 shell/外部工具；
   `SOKONANODA_RELEASE_BASE` 供测试/自托管覆盖，`SOKONANODA_OFFLINE` 离线。
2. **去脚本**：删除 `scripts/soko.sh`；`.opencode/command/sokonanoda/*` 改调
   `sokonanoda <sub>`；插件 `findRepoRoot` 改用 `.opencode/plugin/sokonanoda.ts`
   作仓库标记；LSP shim 改为解析二进制 + `sokonanoda lsp`。
3. **测试**：`crates/cli/tests/opencode.rs` 重写为 8 个（`version` 三态标记、
   `doctor` 退出码、`setup` 离线可行动、`update` 本地 HTTP 服务器验证版本
   锁定下载、shim 解析/失败可行动、插件与命令契约）。
4. **独立验收（subagent）**：build / `--test opencode` / 全部子命令（含真实
   网络 `setup`+`update`）/ `grade` / shim / 契约 / clippy 全过；唯一发现
   `cargo fmt` 未过（新 `src/env/` 的 7 处换行）→ 已修。
5. **文档**：新增 `docs/design/binary-cli.md`；AGENTS/README/`skills/README`/
   teacher+dev 技能/TESTING 同步；onboarding 加历史注记；调研笔记补后续。
6. **发布注意**：二进制新增 TLS+tar 依赖，需 release `workflow_dispatch`
   干跑验证 8 平台交叉构建（rustls/ring）。

## 本轮进度（2026-09-11，第二十八轮：内核已定义名字的声明 warning）

> 触发：学习者在画布写 `axiom Prop : Sort 1`。这行本编译器能通过（内核
> 把 `Prop` 当排序 `Sort 0`，不查环境），但声明出的名字永不被引用；官方
> Lean 里还会重复声明报错。用户要求保留这行、但在 VS Code 里给 warning，
> 并选定「前端产出 + CLI/LSP 两侧消费」方案。

1. **front**（新模块 `compile/warning.rs`）：`collect_warnings` 纯语法扫描
   顶层声明名，命中 `Prop`/`Sort`/`Type` 产出 `reserved-declaration-name`
   （`decl_name_span` 把 span 收窄到名字 token）；`CompileOutput.warnings`
   与 `DocumentReport.warnings` 双通道；会话零重编译路径现算，坐标随前文
   平移。
2. **CLI**：`--json` 新事件 `warning`（`{type, human, code, message, hint,
   span}`），人类视图 stderr `line:col: warning[code]: message`；不改退出码
   （只有 errors 决定成败）。`EVENT_VOCABULARY` 9→10。
3. **LSP**：`report.warnings` → `DiagnosticSeverity::WARNING`，与既有
   `sorry` warning 并列；`axiom Prop : Sort 1` 在第 79 行显示 warning。
4. **协议/文档**：`docs/protocol.md` 增 `warning` 事件与非致命语义；
   `docs/design/reserved-decl-warning.md`（取舍/验收）；teacher 参考
   `references/events.md` 同步。
5. **测试三层**：front `reserved_declaration_name_produces_a_warning` /
   `ordinary_declaration_names_have_no_warning` /
   `session_keeps_warnings_on_zero_recompile`；CLI protocol
   `reserved_declaration_name_warns_but_stays_successful`；LSP
   `reserved_declaration_name_is_a_warning_not_an_error`。
6. **验证**：`cargo fmt --check` 干净；`cargo clippy --workspace --all-targets`
   教学 crates 零违规；`cargo test --workspace --locked` 全绿（front 243 /
   LSP 93 / CLI protocol 9）。playground 锚点：checked=14 / open=5 /
   warning=1 / 0 诊断。
7. **CI 维护**：GitHub Actions Node 20 弃用提示（首次推送后 annotation）——
   升级到 node24 版本：`actions/checkout@v5`、`actions/setup-node@v5`、
   `actions/upload-artifact@v7`、`actions/download-artifact@v8`；
   `Swatinem/rust-cache@v2` 已是 node24；`mlugg/setup-zig@v2`（最新 v2.2.1）
   仍 node20、暂无替代，留观察。判断规程记入 `skills/sokonanoda-ci`。
8. **发版 0.11.0**：warning 是新诊断能力 → minor bump（`Cargo.toml`/
   `Cargo.lock`/`editor/vscode/package.json`/`package-lock.json`/
   CHANGELOG/README），tag `v0.11.0` 触发 release：新 LSP tarball +
   per-target/universal VSIX 发布到 GitHub Release 与 VS Code Marketplace。
9. **warning 文案直白化（用户反馈）**：原文案里「内置排序」是生造词——改成
   直白说法：`Prop` 内核已经定义过，并点出 Prop 在形式化证明里的特殊地位，
   不再出现「排序」这个词。同步 front message/hint、CLI/LSP 文案、设计文档
   与 teacher 参考。与 `Type n` 一起随 0.12.0 发布（0.11.1 未单独发版）。
10. **`Type n` 记法（0.12.0，minor，用户要求）**：补上 Lean 的
    `Type n = Sort (n + 1)` 解析糖（`Type 0` = `Sort 1`；单独 `Type` 仍
    `Sort 1`；`Type u` 不支持，写 `Sort u`）。纯解析糖、复用
    `SortKind::Sort`，不碰 elaborator/内核。课程单元④（zh + en + 钥匙）+
   front 解析/编译单测 + CLI e2e + 白名单文档同步。设计见
   `docs/design/type-level-syntax.md`。
11. **onboarding 补 `update` / `version`（用户要求）**：`soko.sh` 新增
    `update`（强制按仓库版本重下 = `setup --force`）与 `version [--json]`
    （只读报告仓库版本/平台 + 缓存里 CLI/LSP 的 `<version> <target>` 标记与
    是否匹配）；opencode 新增 `/sokonanoda/update`、`/sokonanoda/version`；
    插件 `downloadBinary` 现在按标记校验缓存（过期/缺失就重下，下载后写
    标记）——修掉「发新版后纯缓存不会更新」。契约测试 opencode 8→10；
    AGENTS/onboarding/teacher 文档同步。
12. **调研：为什么不用单一跨平台 Rust 二进制替代 `soko.sh`（用户提问）**：
    结论——跨 OS 的单一二进制在技术上不存在，Rust 按 target triple 编译，
    "跨平台"= 每平台一份二进制 + 一个"选对并取回"的引导器；`soko.sh` 与
    Node 插件就是引导器（rustup 也是 `rustup-init.sh` 引导）。记录见
    `docs/notes/rust-cross-platform-binary.md`（含可选的减负方向：
    自检命令下放 CLI、POSIX sh、cargo-dist/cargo-binstall）。

## 本轮进度（2026-09-11，第二十七轮：文档结构收敛）

> 触发：用户要求「把文档文件夹结构化、最重要的放外面、收敛一下」，并清理
> 过时件。原有 32 篇平铺在 `docs/`，核心与长尾混在一起，新 agent 难以定位。

1. **分层**：入口/权威移到仓库根（`README.md`/`AGENTS.md`/`ROADMAP.md`/
   `REQUIREMENTS.md`/`STATUS.md`）；开发者参考留在 `docs/` 顶层
   （architecture/protocol/TESTING/RELEASE/vscode-dev-guide/LESSONS/
   CI-FAILURES/teaching-session）；设计文档进 `docs/design/`（15 篇，
   去掉冗余 `design-` 前缀）；调研/笔记进 `docs/notes/`（5 篇）。
2. **单一地图**：新增 `docs/README.md`（仓库根/核心/设计/笔记四层，
   每篇一句话职责 + 何时读）；`AGENTS.md` 指向它；`STATUS.md` 头部原来的
   16 条设计文档长列表收敛为一行指针。
3. **清理**：删除零引用/纯历史件 `docs/notes/last-request.md`（原始 scratch）
   与 `docs/design/learner-round.md`（第一堂课的历史轮次，成果已在 STATUS）；
   修正 `docs/design/round14.md` 里悬空的 `docs/design-spine-meta.md` 引用
   （该文件从未创建，路线就在 round14 §0.4）。其余设计文档均有代码注释/
   REQUIREMENTS/STATUS 引用，作为 as-built 存档保留。
4. **引用同步**：全仓 45 个文件批量改写路径（`docs/design-*` →
   `docs/design/*`、notes 同理、根文档去 `docs/` 前缀）；契约测试
   `skill_referenced_repo_paths_exist` 守护 skill 引用可达。
5. **验证**：`skill`（4）/`protocol`（8）/`extension`（13）契约测试 +
   全量 `scripts/soko.sh gate` 通过。

## 本轮进度（2026-09-10，第二十六轮：#check 结果常驻显示，扩展 0.10.0）

> 触发：用户问「`playground.sokonanoda:276-278` 的 `#check` 在 VS Code 里
> 怎么看结果，没有 Lean 那样的 goal infoview」。现状：悬停即等价于
> `#check`（内核同一路径），但结果不常驻。要求：Lean Infoview 对照的
> 常驻展示。

1. **front**：`DocumentReport` 新增 `checks: Vec<CheckInfo { span, text }>`
   （`#check` 表达式 span + 内核 pp 的类型文本），`run_pass` 从
   `TypeChecked` 事件收集；session 的 `assemble_report` 同源——零重编译
   与部分重编译路径都保留结果并随 span 重映射平移。
2. **LSP**：`inlay::document_hints` = 洞 hints + `#check` hints
   （表达式后常显 `: <类型>`，tooltip「`#check` 的内核结果」）；inlayHint
   处理器切换入口。
3. **扩展 0.9.1 → 0.10.0**（minor：新展示能力）：`package.json` /
   `package-lock.json` / Cargo workspace / `Cargo.lock` 同步；CHANGELOG
   Added + Marketplace README 的 inlay 条目扩写。
4. **测试三层**：front `document_report_carries_check_results`（报告携带
   span+文本）；session `session_keeps_check_results_on_zero_recompile`
   （注释编辑零重编译后结果保留、坐标平移）；LSP inlay
   `check_results_appear_as_inlay_hints`（`: Type 0` ×2、位置在表达式尾）；
   集成测试 `#check results appear as inlay hints`
   （`vscode.executeInlayHintProvider`，真实 VS Code）。门禁 `gate` 通过。
5. **既有通道不变**：悬停表达式仍是「逐点查类型」；`#check` 结果现在
   常驻且与判定同源。

## 本轮进度（2026-09-10，第二十五轮：VS Code 希腊字母高亮修复，扩展 0.9.1）

> 触发：用户反馈「α 在 VS Code 里有奇怪的难看的矩形框」。根因不是渲染
> 损坏，而是 VS Code 的 Trojan-Source 防护：混淆字符高亮
> `editor.unicodeHighlight.ambiguousCharacters`（默认开）把希腊 α 视为
> 与拉丁 a 易混，画框提示；代码区默认命中（注释默认豁免，所以中文不受
> 影响）。内置的 `[plaintext]`/`[markdown]` 语言默认已关掉它。

1. **扩展级修复（只影响本语言）**：`package.json` 新增
   `contributes.configurationDefaults["[sokonanoda]"]
   .editor.unicodeHighlight.ambiguousCharacters = false`——不动用户全局
   设置，`.sokonanoda` 文件里 α/β/γ 正常显示；安全高亮在其他语言照旧。
2. **仓库开发态立即生效**：`.vscode/settings.json` 同步语言级覆盖，重载
   窗口即不用等新 VSIX。
3. **版本纪律**：扩展 0.9.0 → 0.9.1（patch：观感修复、零新能力），
   `Cargo.toml` workspace 版本同步，`Cargo.lock`/`package-lock.json` 更新；
   `CHANGELOG.md` 与 Marketplace README 功能清单同步。
4. **测试**：静态契约 +1（`manifest_disables_confusable_unicode_highlight_
   for_the_language`，TDD 先红后绿）；集成测试 +1（真实 VS Code：
   `getConfiguration("editor", {languageId})` 断言为 false，`npm test`
   5 passing）；全量门禁 `scripts/soko.sh gate` 通过。
5. **对用户**：已装 0.9.0 的实例要么重载本仓库窗口（吃 `.vscode` 覆盖），
   要么装 0.9.1 VSIX / 等下一版发布；报错时先查
   `editor.unicodeHighlight.*` 是否为其他设置覆盖。

## 本轮进度（2026-09-10，第二十四轮：函数实参洞 + hover 开项修复）

> 触发：用户以 `playground.sokonanoda:233`（`Eq.subst.{1} Nat (fun (x : Nat)
> => …) a b h …`）为例提两个需求——(1) hover `Eq.subst.{1}` 要显示完整类型
> （现在只剩源码切片）；(2) 谓词实参改写成 `(sorry)` 后应是合法练习、编辑器
> 提示 `Nat -> Prop`。评估后二者都不需要 metavariable / 内核语义改动，同轮
> 落地；设计见 `docs/design/goal-func-spine.md`。

1. **hover/#check 真 bug 修复（内核显示层）**：根因是 pp 的
   `is_implicit_fun` 开空 context 推断子项隐式风格，打印 `Eq.subst`/`Eq.refl`
   的类型时遇到开项（Var 头 `p a`、依赖实参 `Eq α a`）对松散变量 panic
   （`infer: loose bvar` / `eval: loose bvar`）；front 的 `resolve_hovers`
   catch_unwind 后把类型文本置空，LSP 只剩源码切片。修复 = 含松散变量的
   fun 项直接返回 `false`（按显式打印），闭项行为逐字节不变、热路径零改动。
   同一修复让 CLI `#check (Eq.subst.{1})` / `(Eq.refl.{1})` 不再假报
   `kernel-rejected`。
2. **函数实参洞（方案一）**：模板 machinery 从 check.rs（1816 行）抽到新
   `crates/front/src/compile/goals.rs`；模板表扩成双索引——`ctors`（族头→
   构造子，行为不变）+ `funcs`（函数名→望远镜，来源：源内 axiom/def/theorem、
   归纳构造子、Full 模式下未被文件接管的 Eq prelude）。walk 在构造子语义
   之后加函数兜底：第 i 个直接实参洞的期望类型 = binder 类型用前 i 个实参
   AST（含宇宙层级：`.{1}` → `Sort 1`）替换后渲染；前置洞未定时 `ty=null`。
   Bare 模式文件自定义的 `axiom Eq.subst` 同样生效（来源 1）。
3. **明确不做（v1）**：嵌套洞（`f (g sorry)`、含洞 lambda 实参）、部分应用
   自动补参、`sorry + 1`（`Expr::Plus`）、kernel 级 spine meta（远期项不变）。
   宽松度取舍：已知函数的直接实参洞都算 Open，形状错误推迟到填洞后的内核
   终审（与整值 `sorry` 一致）。
4. **测试 +14（三层）**：kernel `memory_api` 1（开项 pp 不 panic，已验证
   修复前必失败）；front 9（hover `Eq.subst.{1}` 带类型 + 函数洞期望类型/
   层级替换/前置洞 `None`/源内 axiom/用户 def/嵌套洞仍 misplaced）；CLI 2
   （`#check` 恢复 `expr.typed`、函数洞 `exercise.open`）；LSP 2（hover 签名、
   inlay `: Nat -> Prop`）。全量 `cargo test --workspace --locked` 与
   `scripts/soko.sh gate` 通过；协议形状未变（`sub_goals` 既有字段，仅
   `docs/protocol.md` 措辞泛化）。
5. **文档**：新增设计 `docs/design/goal-func-spine.md`；REQUIREMENTS §9、
   architecture §6 改动清单、protocol 的 `sub_goals` 说明、teacher skill 与
   teaching-session 的 `elab-hole-misplaced` 行同步。
6. **从零教学（用户重申）**：Bare 模式（`-- sokonanoda:prelude none` /
   `--bare`）工作流写进 teacher skill：自建 `inductive Nat`（zero/succ，rec
   自动派生）+ `axiom Eq`/`Eq.refl`/`Eq.subst`；实测 Bare 文件里
   `Eq.subst (sorry) …` 的 inlay 提示 `Nat -> Prop`（文件自定义模板生效）。

## 本轮进度（2026-09-10，第二十三轮：环境配置单一入口 + Release exec 修复）

> 触发：用户反馈「项目没把如何配置好环境写清楚，让 code agent 搞了好久，
> 流程没有理顺；要调研优秀实践」。3 个 subagent 并行调研（OSS onboarding /
> agent onboarding / 安装器 UX），结论落 `docs/design/onboarding.md`：
> 单一 bootstrap + doctor（机器可读、退出码契约）+ 文档只引用脚本 +
> opencode 命名空间命令 + 启动插件自动 provisioning。

1. **`scripts/soko.sh`（单一环境入口）**：`setup`（幂等、版本锁定下载
   CLI+LSP，marker=`<version> <vsce-target>`）、`doctor [--json]`（只读，
   0=就绪/3=未就绪）、`grade <file>`（缺则自动补齐后 CLI `--json`）、
   `gate`（贡献者 CI 门禁）、`lsp`（编辑器解析链：env → 仓库构建 → VS Code
   扩展自带 → 缓存 → 版本锁定下载 → 编译）。退出码契约 0/1/2/3；
   `SOKONANODA_CACHE_DIR` / `SOKONANODA_OFFLINE` / `SOKONANODA_LSP_BIN` 可覆盖。
2. **opencode 层收薄**：命令迁移到命名空间 `.opencode/command/sokonanoda/*`
   （`/sokonanoda/setup|doctor|check|gate|round`，旧扁平命令删除）；
   `.opencode/lsp/sokonanoda-lsp.sh` 瘦成 3 行 shim；新增
   `.opencode/plugin/sokonanoda.ts`（启动 best-effort 跑 setup + `shell.env`
   把缓存目录注入 PATH）。launcher 的解析/下载逻辑不再重复。
3. **文档两扇门**：AGENTS.md 顶部新增 `## Setup`（一条 setup + 一条 doctor +
   禁止项）；teacher/dev 技能的环境节改为引用脚本（零 cargo 片段从 4 处收敛
  到 1 处）；README 增加「In this repo」一条命令；`skills/README` 说明单一
   入口。
4. **测试**：`opencode.rs` 重写为 7 个（`doctor` 退出码 3→0、`setup` 离线
   可行动、`grade` 直exec CLI、launcher 命中扩展自带 bin、fake-curl 版本
   锁定下载、离线失败 exit 3、命令命名空间/插件/shim 契约）。
5. **发布资产 bug（重要）**：v0.8/v0.9 的 Release tarball 因
   `upload/download-artifact` 丢 unix mode 而全是 0644，解出不可执行；
   修：发布 job tar 前 `chmod +x` + `tar tzvf | grep '^-rwx'` 断言、
   `soko.sh`/`server.js` 解压后 chmod 兜底、**回填修复 v0.9.0 的 16 个
   tarball**（已验证 755 + `--version`）。教训记 `docs/CI-FAILURES.md`。
6. **本机验收**：`scripts/soko.sh setup` 就绪（v0.9.0，版本匹配）；
   `doctor` READY；`grade playground.sokonanoda` 正常出事件；
   opencode LSP 诊断正常（6 条）。画布的练习进度由学习者推进（open 7→6，
   未提交）。
7. **cwd 无关修复（用户复查命令体发现）**：命令体/插件最初用相对路径
   `scripts/soko.sh`，但 opencode 可从子目录启动（ctx.directory=启动目录），
   会直接找不到；命令改为 `git rev-parse --show-toplevel` 定位仓库根，插件
   `findRepoRoot` 向上查找；契约测试加断言（命令必须根无关）。
8. **opencode.json 的 lsp 块被误删（用户现场发现）**：工作区里
   `opencode.json` 丢了整个 `lsp` 段，opencode 启动即打
   `all LSPs are disabled`（不是“没安装”：doctor READY、LSP 握手正常）。
   新增契约测试 `opencode_json_wires_the_sokonanoda_lsp`（后续被第 9 条
   的插件方案取代）防静默丢失。
9. **opencode LSP 改为插件直连原生二进制（用户问「为什么需要 bash /
   其他平台支持吗」）**：实验证实插件 `config` 钩子在 LSP 启动前生效；
   插件改为纯 TS 完成解析 + 版本锁定下载（`fetch`+`tar`；Windows 10+ 自带
   tar，不需要 bash），把 `lsp.command` 直接指向二进制绝对路径；
   `opencode.json` 不再含 lsp/shell 命令；尊重用户自定义 `lsp.sokonanoda`；
   shim 仅保留给非 opencode harness。契约：插件必须含
   `config`/`fetch`/`extensions` 且不含 `"bash"`，opencode.json 无 lsp 块
   （8 个 opencode 测试全绿；opencode 实测 6 条诊断）。需重启 opencode 生效。

## 本轮进度（2026-09-10，第二十二轮：平台矩阵 4 → 8）

> 触发：用户问「其他成熟项目都加了吗」——调研确认 cpptools 9 平台、C# 8、
> rust-analyzer 8（含 alpine）；本轮把平台包从 4 扩到 8，对齐 C#。

1. **平台矩阵 4 → 8**：新增 `linux-arm64`（aarch64-gnu）、`alpine-x64` /
   `alpine-arm64`（musl 静态）、`win32-arm64`（aarch64-msvc 原生构建）。
   对应 VSIX：8 平台包 + universal 回退包。
2. **Linux 构建改 cargo-zigbuild（顺带修真实兼容缺陷）**：此前 `linux-x64`
   在 ubuntu-24.04 原生构建，二进制带 glibc 2.39 符号（VS Code 自身底线
   2.28）；现统一 `cargo zigbuild` + 显式 `.2.28` 地板，Zig 0.16.0 /
   cargo-zigbuild 0.23.4 钉死；构建期 readelf 断言 GLIBC ≤ 2.28、musl
   断言 `ldd` 非动态。
3. **运行时 Alpine 检测**：`server.js` 按 `/etc/alpine-release` 选
   `alpine-*`（与 VS Code 的 target 选择一致），下载映射 gnu→musl 对应
   triple；`scripts/stage-lsp.js` 映射表补 4 个新 triple。
4. **测试**：node 单测 16（+4 类映射/Alpine 检测/musl URL）+ 契约 12
   （server.js 8 target + Alpine；release.yml 新 target + zigbuild + 2.28）。
5. **版本 0.8.0**（minor：新平台覆盖）；README/CHANGELOG/RELEASE.md/
   vscode-dev-guide/ci skill/TESTING 同步；release.yml 冒烟扩到 9 个 VSIX。
6. **release dry-run 验收（run `34466809786` 全绿）**：8 平台构建全过——
   `linux-x64/arm64` 断言 `highest required symbol: GLIBC_2.28`，
   `alpine-x64/arm64` 断言 `not a dynamic executable`（静态），
   `win32-arm64` 原生构建成功；package-vsix 出 9 个 VSIX（8 平台 1.89–2.03MB
   + universal 0.47MB），冒烟全部 mode=755 / TargetPlatform 正确。首次跑
   抓到一个验证脚本 bug（musl 断言被 `pipefail` 反杀，二进制本身正确），
   已修并记 `docs/CI-FAILURES.md`。
7. **opencode LSP 启动修复（用户反馈「找不到可执行的 sokonanoda-lsp」）**：
   根因是 opencode 直接 spawn `command[0]`（无 shell、cwd 可能是子目录），
   原配置 `cargo run …` 依赖 PATH 里的 cargo，且首次构建/握手失败会把
   server 整个会话标 broken、不再重试。改为 `opencode.json` 指向仓库自带
   launcher `.opencode/lsp/sokonanoda-lsp.sh`，解析顺序：`SOKONANODA_LSP_BIN`
   → 仓库 `target/{release,debug}` → **VS Code 扩展自带 bin**
   （`~/.vscode*/extensions/sokonanoda-lang.sokonanoda-*/bin/<target>/`）
   → 扩展下载缓存 → 最后才 `cargo build`。新增
   `crates/cli/tests/opencode.rs` 契约 2 个（含伪造扩展目录 + 无 cargo PATH
   的端到端用例）。
8. **v0.8.0 发布完成**：release run `34468213710` 全绿；GitHub Release 17
   资产（8 tarball + 9 VSIX）；Marketplace 0.8.0 的 universal + 8 平台包
   全部上架（gallery API 核实）。linux-armhf 仍由 universal 兜底（可选）。
9. **opencode LSP launcher（用户反馈）**：`opencode.json` 改为 `bash -c exec`
   单行调用仓库 launcher；launcher 解析顺序：`SOKONANODA_LSP_BIN` → 仓库
   `target/` → VS Code 扩展自带 bin → 下载缓存 → **按仓库版本锁定自动下载
   Release**（`SOKONANODA_LSP_OFFLINE=1` 可禁）→ 编译兜底。契约测试 4 个
   （伪造扩展目录、fake-curl 下载断言无 `/latest/`、离线可行动报错）。
10. **CLI 零工具链化 + 全仓文档审计（用户纠正「cargo run 是重大失误」）**：
    - 平台包同时内嵌 `sokonanoda` CLI（课程树开箱可用，`resolveCliCommand`
     优先 bundled → workspace → PATH）；
   - Release 新增 8 个 `sokonanoda-cli-<triple>.tar.gz`（agent/headless 直接
     下载执行，**不需要 cargo**）；
   - 根 README 拆为「Use it（零工具链）」/「Build from source（贡献者）」；
     teacher 技能、teaching-session、extension 头注释与错误文案全部去
     cargo；AGENTS 命令节标注「仅贡献者需要 Rust」；
   - 原则升为硬规则 `REQUIREMENTS.md` §2 第 9 条；版本 0.8.0 → 0.9.0。
11. **v0.9.0 发布完成 + opencode 重配（用户要求）**：release run
   `34471781169` 全绿；GitHub Release **25 资产**（8 LSP tarball + 8 CLI
   tarball + 9 VSIX）；Marketplace 0.9.0 的 universal + 8 平台包全部上架
   （gallery 核实）。opencode：`/check` 改零 cargo 二进制（`$SOKO`）、新增
   `/setup`（按版本拉取 CLI+LSP，禁 latest）；契约 +1（命令文件不许出现
   源码构建命令）。扩展激活文案精准化（显式路径写错提前报错；回退下载只
   发生在 universal / 安装损坏场景，正常平台包永不联网）。

## 本轮进度（2026-09-10，第二十一轮：插件自带 LSP——bundled VSIX）

> 设计先行：`docs/design/bundled-lsp.md`（含行业调研、发布流程 as-is 与
> 版本错配根因、to-be 流水线）。触发：用户要求把 bin 打包进 VS Code 插件，
> 消除「装完插件再下载 GitHub」与**插件/latest bin 版本错配**。调研纠正：
> 官方 Lean 4 / VsCoq 均不打包（依赖 elan/opam）；正确机制是 VS Code
> platform-specific VSIX（`vsce package --target`）。

**Phase 1（核心，已落地）**：

1. **`editor/vscode/server.js`（新增，无 `vscode` 依赖可单测）**：平台→target
   映射（darwin-arm64/darwin-x64/linux-x64/win32-x64）、bundled 解析 +
   exec 位自动修复（X_OK 检测 + best-effort chmod 755，只读则回退）、
   解析顺序（setting → env → **bundled** → workspace target → 缓存）与
   下载。**下载 URL 从 `releases/latest` 改为 `releases/download/v${version}`**
   ——用户点名的版本错配根因在此修复（`extension.js:113` 旧行为）。
2. **`extension.js` 接线**：删掉本地重复的下载/发现逻辑，改 require
   `server.js`；激活文案区分「无内置二进制（回退下载）」与「平台不支持」。
3. **`scripts/stage-lsp.js`（新增）**：按 rust host/`--rust-target` 把 release
   二进制 stage 到 `bin/<target>/` + chmod 755，支持 `--package` 一键出
   host VSIX；`package:host` / `package:universal` / `clean:lsp` scripts。
4. **版本纪律**：扩展与 Rust 同步 bump **0.7.0**；`extension.rs` 新增
   `cargo_and_extension_versions_match` 契约测试（tag 前拦漂移）。
5. **测试**：node 单测 22（`test-download.js` 改为 require `server.js` 真实现，
   消灭复制漂移）+ 静态契约 +5（bundled 解析/latest 禁令/版本一致/发布
   per-target/CI stage），workspace **442 passed + 8 ignored** 全绿；
   fmt/clippy 干净；playground 锚点 20/9/0 不变。
6. **本机验收**：`npm run package:host` → VSIX 1.96MB，含
   `extension/bin/darwin-arm64/sokonanoda-lsp`（zip mode 755）、manifest
   `TargetPlatform="darwin-arm64"`；`package:universal` → 0.47MB 无 bin。
7. **门面**：README/description/CHANGELOG 0.7.0 同步；`.gitignore` 收
   `editor/vscode/bin/`；`.vscodeignore` 排除 scripts/test；CI 单测步骤改
   `npm run test:unit`。

**Phase 2（发布闭环，已落地）**：

1. **`release.yml` 重排**：新增 `package-vsix` job（needs build）——tag 版本
   门禁（`tag == Cargo.toml == package.json`）→ 下载 4 平台二进制 → 逐 target
   `stage-lsp.js` + `vsce package --target` → 4 个平台包 + universal 回退包 →
   **python zipfile 冒烟**（bin 路径/大小 >1MB/linux+darwin exec 位 755/
   manifest TargetPlatform/universal 无 bin）；github-release 附 4 tarball +
   5 VSIX；marketplace-publish 先 universal 后逐 target（每包 4 次重试）。
2. **`ci.yml` 硬化**：集成测试前 `stage-lsp.js --profile debug` 到
   `bin/linux-x64/`（集成测试走 **bundled 路径**）；新增 host VSIX 打包冒烟
   step（每次 CI 验 exec 位）。
3. **契约 +2**：release.yml 含 per-target/version gate/universal；ci.yml 含
   `test:unit` + stage + host VSIX 冒烟。
4. **文档同步**：`docs/RELEASE.md` 重写（新流水线 + dry-run + 风险）、
   `docs/vscode-dev-guide.md`（server.js/测试层/开发循环/3 条新坑）、
   `skills/sokonanoda-ci`（平台包 exec 位/发布顺序/版本门禁/ETIMEDOUT）。
5. **dry-run 验收（workflow_dispatch `34460822423` 全绿）**：4 平台构建 +
   package-vsix 全过，产出 5 个 VSIX（darwin-arm64 1.93MB / darwin-x64
   2.01MB / linux-x64 2.06MB / win32-x64 2.03MB / universal 0.47MB），
   zip 内 `bin/linux-x64/sokonanoda-lsp` mode 755、manifest
   `TargetPlatform="linux-x64"`，darwin 二进制本机 Mach-O arm64 可执行。
   抓修 2 个只存在于 CI 的路径 bug（`../../` 层数、`mkdir -p dist`），
   均已记 `docs/CI-FAILURES.md`；另有 1 次集成测试 ETIMEDOUT 间歇网络
   （同代码下一轮绿，已入台账 + CI skill）。
6. **正式发布 v0.7.0（2026-09-10）**：tag `v0.7.0` → release run
   `34462668264` 全 job 绿（含 marketplace-publish 真实发布 5 个包）；
   GitHub Release 9 资产（4 tarball + 5 VSIX）；Marketplace 经 gallery API
   核实 0.7.0 的 universal + darwin-arm64 / darwin-x64 / linux-x64 /
   win32-x64 五个包全部上架（validation 约 4–10 分钟）；tag 触发的 ci 也绿。

**Phase 3（硬化，基本完成）**：CI 集成测试已走 bundled 路径（fresh runner
= 无缓存激活的实证）；剩余为决策项——是否追加 linux-arm64 / win32-arm64 /
alpine 平台包（下轮评估，暂由 universal 回退包兜底）。
`docs/design/bundled-lsp.md` 已提交（`39ea982`）。

## 本轮进度（2026-09-10，第二十轮：光标处 goal 视图 Phase 2 落地）

> 设计先行：`docs/design/by-tactics.md` §6 修订为 as-built。承接第十九轮
> 「实现留后续轮次」的 Phase 2：front 产出 per-tactic 状态，LSP 按光标
> 选取，VS Code 练习树渲染。

1. **front：`by_steps` 产出（kernel 一行未动）**：`by::ByStep.goal` 改
   `Option<String>`（`None` = 全闭合）；`DeclState.by_steps: Vec<ByStepState>`
   （`{span, goal, binders}`）贯通 Def/Theorem/Example 的 Open 与 Checked
   分流（`lower_by_val` 返回降级值 + 步状态）；I8 session 的 `remap_prefix`
   同步平移 `by_steps` 的 span（注释级编辑零重编译后坐标不漂）。新测试 3：
   partial by 的逐步 goal/上下文/span（含 render 的应用括号形状）、checked
   by 尾步 `goal=None`、session 注释编辑重映射。
2. **LSP：`soko/stateAt`（选择全在服务端）**：请求
   `{textDocument, position}` → `{version, decl?, goal, binders, span, step,
   total}`。选取语义定为 **Lean `goalsAt?`**（比初稿「执行后」更贴合学习者）：
   光标在某 tactic span 内 → 该 tactic 的**执行前**状态（`steps[i-1]`；首条 =
   根状态，goal 用内核渲染的完整声明类型 `ty_text`）；否则取终点 ≤ 光标的
   最后一步执行后状态。无 by 块的声明退回剩余 goal/上下文。5 个协议级测试
   （进入态/末步态/根态/无 by 回退/声明外为空）；wire 词汇表 +1
   （`common/mod.rs`）；`docs/protocol.md` 新小节。
3. **VS Code：练习树「当前光标处」组**（subagent 实现，主会话验证）：
   目标（点击 `revealRange` 跳 tactic）+ 假设 + `by 进度 k/n`；选区变化
   去抖 200ms 请求 `soko/stateAt`，请求序号 + 活动文档守卫丢弃过期响应
   （响应带 version）；诊断刷新后重取。静态契约测试 +2（客户端必须消费
   `soko/stateAt`、必须挂选区监听）；**版本 0.5.2 → 0.6.0**（minor：新学习
   能力），CHANGELOG/README/工作区 Cargo.toml 同步。
4. **顺带修复**：`playground.sokonanoda:7` 的 `???`→`sorry` 迁移残留
   （原文案成了「sorry 也可以写成 sorry」的同义反复）改写为自然说明。
5. **验收**：测试总量 **437 + 8 ignored**（front 229 / lsp 89 / cli 71 /
   kernel 48）全绿；fmt/clippy 干净；playground 锚点
   `decl.checked=20 / exercise.open=9 / 0 诊断`不变；course 汇总
   `32 checked · 25 open · 0 failed` 不变；`node --check` + 扩展静态契约
   套件（7 测试）通过。commit `396adee` 已 push main 且 CI（lint + test）
   全绿；**未打 tag**（0.6.0 的发布留给用户触发）。
6. **opencode 项目配置适配（2026-09-10 用户要求，配置-only）**：
   `opencode.json` 增 `skills.paths: ["./skills"]`（三个 skill 自动加载、
   免软链）、Lean 工具链 bash deny、watcher 忽略 `target/node_modules/
   .vscode-test/learner` 等产物、cargofmt/rustfmt 自动格式化关闭（护 kernel
   冻结快照）；新增 `.opencode/command/{gate,check,round}.md` 与
   `.opencode/agent/teacher.md`（主 agent，画布老师角色）；`skills/README.md`
   与 `AGENTS.md` 同步。**配置改动需重启 opencode 才生效**；不触碰 Rust
   测试面，CI 不受影响。另修 VS Code 报错「Unable to load schema from
   https://opencode.ai/config.json … is untrusted」：工作区新增
   `.vscode/settings.json`，`json.schemaDownload.trustedDomains` 补
   `https://opencode.ai` 与 VS Code 默认域名（修后 schema 校验/补全恢复）。

## 本轮进度（2026-09-09，第十九轮：by-tactic 块 + VSCode goal-state 设计）

> ⚠️ 发布后修复：0.5.1 修「扩展自动下载的 LSP 缓存不校验版本（升级后仍跑旧
> 服务器）→ 按扩展版本号版本追踪；axiom 连接词语义 token → TYPE」。
> 0.5.2 修「`by` 块 span 终点取下一个 token 起点 → 注释被吞进警告范围；
> 尾部 Hole span=offset 0 → 止于 `sorry`」。Lean 确认 `sorry` 术语+tactic 双栖，
> `by sorry` 与 Lean 对齐、与值位 `:= sorry` 无冲突。

> 设计先行：`docs/design/by-tactics.md`。触发：用户要求「实现一些基础 tactic，
> 跟 Lean 4 一样用 `by` 开始」（补 assumption / rfl），并调研设计 VSCode 前端
> 显示 goal state。首期五个 tactic：**intro / exact / apply / assumption / rfl**，另加 `by sorry` 占位（目标保持开放，与值位 sorry 同语义）。

1. **`by` 语法 + 引擎（front 层，kernel 一行未动）**：`theorem t : T := by <tactic>; <tactic>; …`
   （`;` 分隔，教学子集不引入缩进敏感）。新 `Expr::By`/`Tactic` AST、`Semicolon`
   词法、`FolFile.src`（`parse` 存原文，`run_pass` 按声明起点切片当前缀源码）。
   引擎 `crates/front/src/by.rs`：目标树（apply 多子目标）+ 父指针收集上下文；
   逐 tactic 判定复用 `judge_terms`（kernel 唯一裁判），`apply` 用新
   `judge::judge_infer` 推断被应用函数类型 + 位置 spine 合一（codomain 中出现的
   命名 binder = 类型参数、其余 = 子目标）。`by` 没写完整 = 尾部 `sorry` →
   既有 `open_goal` 分流成 Open 练习（「部分作答」同语义）。
2. **内核类型文本可回读**：pp 把 `(a : T) -> (b : T)` 折叠成 `forall (a b : T), …`——
   parser 新增多名字 binder 组 `(a b : T)`（Lean 对齐，仅类型箭头位），
   `proof::render_expr`/`judge::judge_infer` 补 `+` 括号与逐 binder 剥层，
   引擎读回内核类型不再失真。
3. **课程三件套**：`course/` 新增单元⑥「by 写法」（中文 + `en/` 英文镜像 +
   `solutions/` 解答钥匙，全经内核验证）；`playground` 追加 2 道 by 练习题
   （open=7→9）；`course.json`、`course.rs`/`course_status.rs` golden 计数
   更新（unit6 13 checked / 5 open）。
4. **测试**：front 13 新（parse by 块/白名单拒绝未知 tactic/intro+exact/
   assumption/apply+rfl/部分 by→Open/错误 exact→`elab-tactic-failed`/多名字
binder 组/空 by→Open/intro 非函数目标/assumption 无匹配/rfl 非 Eq/apply
    目标不匹配/`by sorry` 占位→Open）+ CLI 2 新（by 端到端、部分 by→open）。
    测试总量 **426**（front 226 / lsp 81 / cli+kernel 119）；fmt/clippy 干净。
5. **VSCode goal-state（Phase 2 设计，协议先行）**：调研 vscode-lean4 Infoview /
   coq-lsp `proof/goals`——共识 = server 端按光标位置从编译期信息树取 tactic
   前后状态。设计：front 产出 `DeclState.by_steps`（每 tactic 执行后 goal+binders）
   + 新请求 `soko/stateAt`（位置感知，返回 version 供丢弃过期）+ VSCode「练习」
   树顶部「当前光标处」goal 组（方案 A，零 webview）。实现留后续轮次，
   `docs/protocol.md` 待落地时补。

## 本轮进度（2026-09-09，第十八轮：hover 重构——良构表达式 + 高亮范围）

> 设计先行：`docs/design/hover-refactor.md`。触发：用户反馈「括号 hover 内容
> 乱七八糟、有些是包含括号的外部表达式」「`(Not a)` 与 `(And.right a (Not a) h)`
> 左右括号内容对不上」，并要求——逐字符评估所有 hover、把正确行为设计成单测、
> 最终**能看到 hover 内容对应的表达式范围（高亮）**。
>
> ⚠️ 本轮与「课程双语化」并发推进；双语 agent 曾 stash 我未完成的
> `lib.rs/tests.rs`（stash@{0}）隔离验证。本轮收尾时已在工作树重建全部
> hover 改动（lib.rs 逐字节一致、front 两个 binder 测试从 stash 还原），
> 丢弃了已过期的 stash，并丢弃其中一条非本轮的 `playground two := 2`
> 实验改动（画布保持 `sorry` 未作答）。

1. **逐字符盘点（16 个 *.sokonanoda 文件，11229 行 dump）**：三类不合理——
   (a) **lambda/Pi 的 binder 名整段溢出**（977 处）：hover `fun (a : Prop) => …`
   的 binder `a` 时最小 span 是整段 lambda，把「表达式 + 整段类型」全吐出来；
   (b) **括号组切片截断**（AST span 不含括号）：`(h : And a (Not a))` 显示
   `And a (Not a : Prop`（缺右括号）；(c) **hover 不返回 range**：`range: None`，
   编辑器无法高亮「这个 hover 在说哪个表达式」。
2. **front binder 行（冷路径）**：`elab.rs` 为每个 Lambda/Forall binder 记一条
   `binder: true` 的声明行（span = 完整标注 `(a : Prop)`，expr = binder 类型，
   scope = push 前）；`HoverType.binder` 字段贯通 `report.rs`/`check.rs`
   （binder 行跳过 infer、text 置空，渲染用源码切片）。hover 到 binder 名 →
   `a : Prop`，不再整段溢出。
3. **LSP 渲染层重构（`render.rs`）**：`HoverResolved{range, content}`；
   `balanced_span` 把截断切片补成良构表达式（跳过 `--` 注释）；`expr_hover`
   统一渲染 + 平衡 span；`bracket_hover` 按「binder 标注组（span==整组）→
   组内最大表达式行」解析，高亮整组（含括号，保证覆盖光标）。
   `hover()` 所有分支（括号/精确/邻近/声明）都返回 `range`。
4. **测试**：front 2 新（lambda binder 行、Pi binder 行）+ LSP 5 新（binder 名
   不溢出、`h` binder 声明、`(h : …)` 括号显示声明不截断、`(a : Prop)` 括号
   显示声明、range 覆盖光标）+ 旧断言对齐（`hover_map_covers_subexpressions`
   允许 binder 行空 text）。测试总量 **410**（front 212 / lsp 81 / cli+kernel 117）
   全绿；fmt/clippy 干净；playground 锚点 `decl.checked=20 / exercise.open=7 /
   0 诊断` 不变。
5. **stash 协调收尾**：并发双语 agent 遗留的 stash@{0}（含我的旧 lib.rs/tests.rs
   与一条 playground 实验）已丢弃——lib.rs 工作树与 stash 逐字节一致、两个
   front 测试已从 stash 还原到工作树、playground 实验改动（`two := 2`）不保留。
   STATUS 原「并发的 hover 重构编译不过」注记已过时，本行为其解决记录。

## 本轮进度（2026-09-09，第十八轮：课程双语化）

> 设计先行：`docs/design/course-bilingual.md`。触发：用户要求 tutorial 等
> 教程文档提供中文与英文两种版本（范围 = `course/` 单元课程为主；形态 =
> 中文/英文各一份独立文件）。

1. **英文镜像（按语义重构）**：新增 `course/en/`（5 单元画布 +
   `solutions/` 解答钥匙）。英文注释是**重新写就的自然教学文案**，按英语
   语感重组句子与段落、不以中文行号/行数为准（用户修订原则：按语义重构、
   不按字节翻译）；**代码与中文逐字节一致**，`soko:hint` 阶梯条数与顺序
   同构（思路 / 目标形态 / 关键件）。中文文件原样不动，作为权威源。
2. **课程清单**：`course.json` 每条目增 `title_en`（英文标题），`file`/`unit`
   与现有 `course.rs` 断言完全兼容。
3. **CI 守卫**：`crates/cli/tests/course.rs` 新增
   `en_mirrors_match_chinese_event_counts`——`course/en/` 与 `course/` 文件
   同名一一对应，两版 `--json` 事件计数（decl.checked / exercise.open /
   expr.reduced / diagnostic）逐项相等；英文钥匙 0 诊断、0 洞。判定走 kernel
   （事件计数），禁文本比对（注释本来就允许不同）。
4. **验证**：EN 5 画布事件计数与中文 golden 表完全一致（如 unit1
   decl.checked=12/exercise.open=5/diagnostics=0），EN 钥匙全 0 诊断 0 洞；
   code 逐字节一致（脚本核对全部 10 个镜像文件）。`en_mirrors…` 守卫在
   隔离外来改动时通过（`cargo test -p sokonanoda-cli --test course` 4 测试
   全绿），我的改动 fmt/clippy 干净。
5. **边界**：不改 `course/` 中文文件、不译 `docs/` 开发者文档与根画布
   `playground.sokonanoda`（留待后续按需扩展）；不做运行时 i18n 机制。
注：仓库里曾有**并发的 hover 重构**（非本轮产物），验证时已被隔离并
    stash；该 hover 重构现已由对方收尾完成（见上方「第十八轮：hover 重构」
    条目），并发期间的 stash 已清理，`sokonanoda-lsp` 恢复编译通过。

## 本轮进度（2026-09-09，第十七轮：括号 hover + 真名还原）

> 设计先行：`docs/design/hover-brackets.md`。触发：用户反馈 VS Code hover
> 内容完全混乱 + `(表达式)` 悬停要求 + `$N` 索引必须还原真名。

1. **根因三连（全部实验证实）**：(a) pp 的 `binder_names` 从空开始 +
   `parse_binders` 对 telescope 域 lift → `name_loose_bvars` 的文本映射
   永不成立（同一变量打印 `$2`/`$3`/`$4`）；(b) `infer_under_binders` 的
   `force_all` 把 `Not a` 展开成 `a -> False`；(c) `hover_type_at` 的
   「起点 ±2」回退在 `)` 上命中右侧邻居（`And.left : forall …` 漏进括号
   悬停），且与 hover() 的 TOLERANCE 回退语义打架。
2. **kernel 显示层（冷路径，architecture §6 记档）**：pp 新增
   `seed_binder_names` + `TypeChecker::with_pp_scoped`——scope 名字预置
   `binder_names`，松散变量在**所有位置**（含 lift 域）精确还原真名
   （代数验证 `S-1-j` 恒成立）；`infer_under_binders` 去 `force_all`
   （`Not a` 保持折叠，与 `#check` 展示一致）。热路径零改动。
3. **LSP 括号组匹配**：`render::bracket_hover_at`（文本扫描配对、跳
   `--` 注释、组内最大行 = 括号包住的表达式；反向扫描扫到光标之前）；
   hover 优先级 = 关键字静默 → **括号** → 精确 → ±2 邻近 → 声明；
   删除错误的邻近回退（goto-def/高亮/补全同步受益）；
   `hover_content` 统一「空 text 或含 `$` → 只显示源码切片」。
4. **测试**：front 2 新（and_not_absurd 全语料零 `$` + 用户两条指定
   样例 + 部分应用真名；and_swap `And.intro b a : b -> a -> And b a`）+
   LSP 5 新（两组括号正反面、`)` 不漏邻居签名回归、`((p))` 四括号透明、
   注释内括号不张冠李戴）+ 旧断言对齐（`(a : Prop)` 的 `(` → 组内最大行）。
5. 测试总量 **402**（front 210 / lsp 76 / cli+kernel 116…）全绿；
   fmt/clippy 干净（kernel warning 级不变）；playground 锚点
   `decl.checked=20 / exercise.open=7 / 0 诊断`。版本 0.4.0 → **0.4.1**
   （patch：改进非新能力；CHANGELOG 已记）。
6. **CI 两连红（v0.4.1 首推）+ 修复**：(a) lint——新代码
   `int_plus_one` 触发 `-D warnings`，本地验证被 grep 掩膜+管道退出码
   双重污染造成假绿（教训入 CI-FAILURES.md，预防=跑与 CI 完全一致的
   命令）；(b) release 的 github-release——`download-artifact` v4 目录
   布局与 upload 路径不符（`vsix/` 路径从未存在；v0.4.0 同因），
   修为 `sokonanoda-vsix/sokonanoda.vsix`。marketplace-publish 本轮
   **成功**（Azure 超时确认为间歇性）。

## 本轮进度（2026-09-07，第十六轮：???→sorry 迁移 + VS Code 集成测试 + hover 纪律）

1. **???→sorry 迁移完成**：lexer 遇 ? 报教学引导错误；全仓清扫 24 文件
   （playground/course/examples/tests/docs）；协议词表不变。
2. **hover 纪律**：显示「表达式 : 类型」+ 声明名显示完整内核签名
   （ty_text）+ 关键字悬停静默 + 松散变量 $N→binder 名字。
3. **VS Code 集成测试**：@vscode/test-electron 4 用例 + CI xvfb。
4. **Lean 4 调研确认**：点分名原子/sorry warning/hover 签名——设计与
   官方对齐。
5. 测试总量 **388 + 4 VS Code 集成测试**。

## 本轮进度（2026-09-07，第十六轮：Lean 4 对齐 + VS Code 集成测试）

1. **sorry → warning 诊断分级**（Lean 4 对齐）：含 sorry 的开放练习产出
   WARNING 级诊断（code `sorry`，消息 `declaration 'X' uses 'sorry'`），
   与 kernel-rejected 的 ERROR 分离——黄色波浪线表示"编译但有缺口"，
   不再与真错误混淆。LSP 2 个新测试（warning 存在 / 非 sorry 不稀释）。
2. **VS Code 集成测试框架**（subagent 搭建）：@vscode/test-electron 4 用例
   （扩展激活/干净 0 诊断/kernel-rejected/sorry hover）；CI Linux 加
   xvfb-run；.vscode-test.mjs 配置 trust 跳过与 60s timeout。
3. **Lean 4 调研确认**（subagent）：点分名原子（我们的设计与官方一致）、
   sorry severity=warning（一致）、hover 三段式（签名+docstring+import，
   可借鉴）、错误优先原则（可借鉴：同声明已有 error 时不发 sorry warning）。
4. 测试总量 **390**（+2 sorry 测试）；全绿；clippy/fmt 干净。

## 本轮进度（2026-09-07，第十五轮：开发清单清零，2 subagent 并行）

1. **spine meta 方案 B′（S1）**：`field_type_text` 升级为深度 AST 替换
   （`substitute_names`：模板 binder 名 → goal 实参 AST 全量替换，innermost
   wins 遮蔽守卫，命中节点 span 回填）——复合字段类型（`And a b`）现在正确
   实例化为学生上下文（`And True False`），不再原样渲染模板名；裸 Ident
   路径与失败语义不变（旧断言零改动）。kernel 渲染版（方案 A）留作远期。
2. **失败声明建议升级（S2）**：三条建议梯子——
   - **kernel 验证 rfl 替换**（新 `judge_value_replace`：值位整体替换合成
     声明交完整 kernel 裁决；Eq 形状声明验证通过才呈现，`verified: true`）；
   - **Reset**（保留已写 lambda 前缀、只重置主体为 `???`——学生类型标注
     工作保留，剩余目标由 goal 视图接管；保守形态识别：仅括号/花括号
     binder 的 `fun x =>` 链）；
   - **Restart**（整值骨架，既有）。
   首条 `is_preferred`；lib.rs 锚点全绿。
3. 测试总量 **380**；全绿；fmt/clippy 干净；playground（12 open / 0 诊断）与
   course（19/20/0）锚点不变。
4. **至此 gap-analysis Top 10 + 附加小项 + 分类学余项全部清零**；剩余仅
   运营项（release 首跑需打 tag、教学回环需真实学习者）与远期设计项
   （spine meta 方案 A）。

## 本轮进度（2026-09-07，第十四轮：hole_id + auto-derivation，2 subagent 并行）

> 设计先行：`docs/design/round14.md`（含 spine meta 的 A/B/C 方案取舍——
> 推荐方案 B 为下一轮实施项）。

1. **稳定 hole_id（P）**：`soko/goals` 的 `holes` 变 `[{range, id}]`，
   id = `<声明名>:<洞序号>`（匿名 example 用 `example@<行>`，与
   `render::decl_name` 一致）——同一版本内稳定、声明名不变时跨版本稳定，
   外部工具可引用；`soko/nextHole` 保持裸 Range；VS Code 点击统一走
   `holes[0].range`（P 核实客户端历史上只消费 `decl.hole`，无行为变化）；
   契约测试双向钉死（服务端出 id、客户端不当裸 Range 用）。
2. **归纳块 recursor 自动派生（Q）**：无显式 `rec` 的 `inductive` 块自动
   合成 recursor + iota 规则（与 py-nat 手写版同构、内核 def_eq 比对通过）：
   - 递归块（Nat 无 rec + add 闭环）、非递归块（Unit）、多构造子
     （Bool：`not tt ⇒ ff`）全部工作；
   - Prop 块退化为无宇宙参数的小消除 recursor；
   - **字段望远镜契约修正**：`num_fields`/`ctor_telescope_size_wo_params`
     改按完整 Pi 望远镜计（result 箭头链的 domain 也是字段——内核
     `check_declared_metadata` 的要求；py-nat 等既有块数值不变）；
   - 第十三轮的 `elab-missing-inductive-rec` 守卫/变体/文档条目移除
     （被本功能取代）；显式 rec 优先，py-nat/课程块零变化。
   - 教学定位：rec 块仍是单元⑤正课，auto-derivation 是其后的便利层。
3. 测试总量 **360**（front 183 / lsp 61 / cli 65 / kernel 45…）；全绿；
   fmt/clippy 干净；playground（0 诊断）/course（19/20/0）锚点不变。

## 本轮进度（2026-09-07，第十三轮：内核分类学收尾 + 基建，4 subagent 并行）

> 设计先行：`docs/design/kernel-taxonomy.md`。K（内核冷路径分诊）/
> L1（失败声明建议）/ M（criterion 基准）/ N（fuzz harness）并行，主会话
> 合并期修复 K 发现的功能性 bug（非递归归纳块）。

1. **内核错误分类学余项（K，冷路径三层回归）**：全内核 `assert_eq!` 清点
   分诊——3 处学习者可触发站点改稳定消息（`is_prop_type` 带 `got:` 渲染、
   iota 规则顺序/数量两处裸断言）；新错误家族 `kernel-rec-rule-mismatch`
   （`ErrorKind` + code + hint + protocol.md + 穷尽清单）；8+ 处 front 已
   拦截的 backstop 与真内部不变量保留 assert（internal）。热循环零改动。
2. **非递归归纳块修复（主会话，K 发现的功能 bug）**：内核按构造子 telescope
   自算 `is_recursive` 并断言一致——front 恒传 `true` 导致
   `inductive Unit/Bool` 崩溃。修复：`elab.rs` 从源码 AST 同规则镜像（含
   result 箭头链的 domain）；缺 `rec` 的块在**入环境前**报干净教学错误
   `elab-missing-inductive-rec`（check-then-add 保持；rec 块本就是白名单
   内容，auto-derivation 留作课程轮设计）。测试三层（kernel 语义 +
   front 2 + CLI 2）。
3. **失败声明建议（L1）**：`SuggestionKind::Restart`——kernel-rejected
   声明按自身类型形状生成重启骨架 `fun (x : A) => ???`（tokenize 定位
   值位，≤3 层剥 Pi，binder 防撞改名），LSP code action 整体替换值位；
   骨架落回后内核重查回到 Open（测试验证闭环）。
4. **criterion 基准（M）**：`crates/front/benches/pipeline.rs`（黑盒公开
   API）：native_bigint_reduce ~41ms / iota_deep_reduce ~14ms /
   session_suffix_recheck ~500µs；语料校验 `OnceLock` 先行。本地跑：
   `cargo bench -p sokonanoda-front --bench pipeline`（多 target 需带
   `--bench pipeline` 选择器）。
5. **fuzz harness（N）**：`fuzz/`（独立 crate，脱离 workspace，cargo-fuzz
   标准布局）——`parse_never_panics`：parse/semantic_tokens/prelude 指令/
   完整 check_document 永不 panic；`cd fuzz && cargo check`（stable）过；
   CI 不跑，用法见 `fuzz/README.md`。
6. 测试总量 **356**；全绿；fmt/clippy 干净；playground（0 诊断）与
   course（19/20/0）锚点不变。

## 本轮进度（2026-09-07，第十二轮：课程地图 + 小项，4 subagent 并行）

1. **`sokonanoda course <course.json>`（课程地图，gap #8 后端）**：聚合
   course.json 全部单元的 `decl.checked / exercise.open / failed /
   expr.reduced` 计数，JSON 视图 = 封闭新事件 `course.unit`（坏单元带
   `error` 字段）+ `course.summary`（进 protocol.md + 词汇表 + 4 个 e2e）；
   人类视图逐单元一行；**进度不是错误**（open/failed 也 exit 0）。
2. **VS Code「课程」树（gap #8 前端）**：`sokonanoda.courseMap` 视图 +
   `sokonanoda.courseRefresh` 命令——客户端跑 CLI 子进程解析 JSON Lines
   （10s 超时、并发去重、找不到 course.json 静默空树）；节点按
   open/failed 着色、点击打开单元文件；**聚合归 CLI，服务器保持单文档**
   （负断言：客户端不得引用 soko/courseStatus）。
3. **REPL 命令历史持久化（小项）**：`$HOME/.sokonanoda_history`（截尾
   1000 行；HOME 缺失静默禁用；不做行编辑——超范围另立项）；测试注入
   临时 HOME，既有 repl 测试不再污染真实家目录。
4. **course/ 五单元提示阶梯内容**：20 个 open 练习 × 3 条（共 60 条
   `-- soko:hint`：思路→目标形态→关键件，答案不进提示）；golden 逐单元
   计数不变、solutions 零诊断（注释级改动不产事件——playground 同机制）。
5. 测试总量 **335**（front 171 / lsp 56 / cli 63 / kernel 45…）；全绿；
   fmt/clippy 干净。playground 锚点不变（checked=14 / open=12 / 0 诊断）；
   course 锚点 19 checked · 20 open · 0 failed。

## 本轮进度（2026-09-07，第十一轮：教学辅助四件套，3 subagent 并行 + 2 调研）

> 设计先行：`docs/design/hints-suggestions.md`（提示阶梯/下一步建议）与
> `docs/design/rename-inlay.md`（rename/references/inlay/lsp 子命令）；主会话
> 预接线（协议、能力注册、桩、front 种子）后 4 个实现 subagent 文件集互斥并行。

1. **提示阶梯 `soko/hints`**：画布指令 `-- soko:hint <text>`（独占一行、挂到
   下一条声明，机制 `front::compile::hints` + `DeclState.hints`；注释级编辑走
   Session 零重编译路径并刷新阶梯）；LSP 自定义请求 `soko/hints`（无状态，
   揭示进度归客户端）；VS Code 练习树「提示」节点 + `sokonanoda.revealHint`
   逐条揭示（不预告剩余条数——WPI 实证）；playground 12 题全部挂上
   思路→目标形态→关键件三级阶梯（**答案绝不进提示**，遵守 teaching-session 规则）。
2. **下一步建议（按目标形状）**：`front::suggest`（每请求 ≤3 条、首条
   `is_preferred`）——exact（kernel 判定）、`Eq.refl` rfl 候选（kernel 验证后
   才呈现）、refine（模板）、intro（形状）；`front::judge::judge_hole_fill`
   把洞替换候选后整份交 kernel 终审。**顺带修复多洞错位 bug**：spine 状态下
   「匹配外层 goal 的假设」不再被塞进子洞（逐洞按 `sub_goals[i].ty` 判定）。
3. **rename + find-references**：全语义集（`resolve_at` + `references_for` +
   tokenize 精确名字 token，零文本扫描；注释/字符串天然不误伤）；prepareRename
   返回名字子 span + placeholder；rename 产出**版本化 documentChanges**，
   非法名/不可解析 → ResponseError（不返回空 edit，LSP 3.17 规范）；shadowing
   内层胜出有回归测试。
4. **inlay hints**：每个开放练习的洞尾标注期望类型（`: T`，子洞类型来自
   server 端 walk；单主洞显示剩余目标）+ markdown tooltip（目标 + 假设）；
   只读信息，无 textEdits。
5. **`sokonanoda lsp` 子命令**（单二进制分发，gleam 模式）：`crates/lsp` lib 化
   （`sokonanoda_lsp::run()`），`sokonanoda` 二进制 `lsp` 子命令拉起 stdio 服务器
   （tty 时 stderr 提示）；`sokonanoda-lsp` 二进制保留，VS Code 端不受影响。
6. 测试总量 **327**（front 171 / lsp 56 / cli 55 / kernel 45…）；全绿；
   fmt/clippy 干净（教学 crates 零警告）。playground 锚点不变：
   checked=14 / open=12 / diagnostics=0。

## 本轮进度（2026-09-07，第十轮：gap-analysis 第一批落地）

1. **行业基线 LSP 三件**（主会话）：completions（关键字/宇宙/prelude 名/
   文档声明，单源 `front::semantic::keywords()`；内部名不外泄）、folding
   range（仅多行声明）、`--version`（cli）+ workspace `rust-version = 1.96`
   （MSRV 声明，rust-analyzer 教训）。
2. **导航基线（subagent A，断网后核实其工作已完整落盘）**：go-to-definition
   （elab 记录 use→def 解析映射：局部 binder→binder span、顶层名→声明
   span；shadowing 正确——内层 `x` 解析到内层 binder）、document highlight
   （同定义全部使用点）、binder 补全（光标处 name_scopes 在域名字）。
   新 API：`ResolvedTarget`、`DocumentReport.definitions/name_scopes`、
   `HoverType.scope_names`。
3. **REPL undo（subagent B）**：`ProofState` 快照栈 + `undo`（`u`/`#undo`）；
   intro/exact 成功前入栈、失败不动；cli e2e + 4 单测。lean4game/Isabelle
   的教学基线能力。
4. 测试总量 **273**（front 141 / lsp 33 / cli 35 / kernel 45…）；全绿。

## 本轮进度（2026-09-07，第九轮：subagent 并行 ×3，主会话多洞/refine）

1. **多洞 + refine（I9 第二段，设计 `docs/design/goal-refine.md`）**：
   构造子 spine 走查——`And.intro ??? ???` 等多洞是合法 Open 状态（不再
   hole-misplaced）；子洞期望类型从文档自身的 axiom/ctor 形状实例化
   （参数位=目标自己的实参，证明位=实例化后的字段类型）；
   `DeclState.holes/sub_goals/refine_template` 贯通 LSP——**refine 建议**
   （`And.intro a b ??? ???`：参数自动填充、证明字段留洞，结构来自文档、
   kernel 终审）、`soko/goals` 携带 holes/sub_goals、nextHole 跨子洞环绕。
   front 4 + LSP 3 个新测试。
2. **内核错误分类学（审计 subagent 报告 → 实现 subagent 落地）**：8 个新
   kernel 错误码（`kernel-expected-sort` / `expected-pi` / `theorem-not-prop`
   / `inductive-non-positive` / `ctor-result-mismatch` / `ctor-arg-invalid-app`
   / `ctor-arg-not-type` / `ctor-arg-too-large`），各带中文教学提示；内核
   冷路径 5 处消息增强（两处无消息 assert 加消息、三处 `got:` 渲染）；
   分类器 `refine_kernel_kind`（含 `rejected:` 前缀剥离与 internal 兜底）。
3. **关键稳定性修复**：`#check`/`#reduce` 直通内核求值路径此前无 panic
   保护——`#check (Type) 3` 会**崩掉整个编译/LSP 进程**；现在经
   `quiet_catch` 降级为分类诊断（并接通分类器）。VSIX 真因修复：
   `.vscodeignore` 排除了 node_modules（上轮只移了 dependencies）——实测
   VSIX 从 9 文件/13KB 变为 324 文件/470KB 且含 vscode-languageclient；
   契约测试封死两处回归。
4. **发布流水线（subagent 实现）**：`release.yml`（tag 触发 + dispatch
   dry-run；Rust 双二进制 + VSIX 同 Release）+ `docs/RELEASE.md` 发布手册。
5. **业内标准差距审计（调研 subagent）**：`docs/notes/gap-analysis.md`——Top 10
   补全清单（completions/go-to-def/folding/提示分级/undo/rename/inlay/
   章节地图/下一步建议/--version+MSRV）与反标配清单；已并入 ROADMAP L2/L3。
6. 测试总量 **245**（front 133 / lsp 26 / cli 35 / kernel 45）；全绿；
   clippy/fmt 干净。

## 本轮进度（2026-09-07，第八轮：goal 面板与跳洞）

1. **VS Code goal 面板（I9 收尾，消费 `soko/goals`）**：资源管理器新增
   "练习" 树——每个声明显示 kind·状态，开放练习展开为「目标 + 已引入
   假设」，点击直达洞位；状态栏显示未完成练习数（点击聚焦面板）；诊断
   更新即自动刷新。**`alt+n` / `alt+shift+n` 跳下一个/上一个洞**（环绕；
   位置计算全部在 server 端 `soko/nextHole`——客户端禁止文本扫洞，
   ocaml-lsp 教训落入代码约束）。
2. **客户端契约测试**（`crates/cli/tests/extension.rs`，4 个）：package.json
   声明的命令必须在 extension.js 注册、键位只指向已声明命令、客户端必须
   消费 soko/goals+nextHole 且禁止自算洞位、运行时依赖必须在 dependencies
   （VSIX P0 回归守护）、打包元数据齐全。无需 Electron 即可 CI 守护客户端。
3. **AGENTS.md**（项目指令入口）：opencode/Claude Code 等原生读取——接手
   阅读顺序、角色技能（skills/）、硬规则速记、命令清单、收尾义务。
4. 测试总量 **239**（+4 扩展契约套件）；clippy/fmt 全绿。

## 本轮进度（2026-09-07，第七轮：逻辑先行课程落地 + I9 宇宙携带）

1. **课程重排（用户课程排序哲学落地，REQUIREMENTS §6）**：course/ 5 单元与
   playground 全部重排为逻辑先行——
   ①命题与证明项（先证明命题，全程不谈 Sort）→ ②等式与 rfl（先认识数字，
   `Eq.{1}` 作为机械规则并埋下"为什么是 1"的悬念）→ ③函数与箭头（结尾埋
   "函数类型的类型？"悬念）→ ④宇宙（Sort 由悬念揭晓，回收 Prop=Sort 0）
   → ⑤归纳与递归（不变）。旧 unit1/2/3/4 文件更名重写，题目与钥匙全部
   复用既有 kernel 验证结论（Eq.symm 钥匙修正了一处缺实参的错误——被
   本仓库 LSP 实时抓出，opencode 接线的第一次实战验证）。
   同步：course.json（新文件名/标题）、course.rs golden（新计数 12,5,1 /
   2,5,2 / 1,4,1 / 0,3,0 / 4,3,1）、course/README、skill 的 curriculum.md、
   teaching-session.md §3（12 题新编号）、playground（12 练习逻辑先行为主：
   checked=14 open=12 diagnostics=0）。
2. **I9 余项——开放声明携带宇宙参数**：`DeclState.universe` 贯通
   （OpenExercise op → 报告 → LSP exact_binder → judge 合成声明），
   带 `{u}` 的开放练习（如毕业题 Eq.symm）现在能获得 exact 建议；
   上一轮记录的已知限制清除。测试：front 2 个（记录 + judge 端到端）。
3. 测试总量 **235**（+2 宇宙携带）；golden 更新为刻意变更；clippy/fmt 全绿。

## 本轮进度（2026-09-07，第六轮：agent skills）

1. **skills/ 目录（用户需求：为 code agent 设计 skill 部分）**：Agent Skill
   格式（SKILL.md frontmatter + references）：
   - `sokonanoda-teacher`：判卷接口（--json 事件读法）、3 步教学循环、
     事件决策表、出题规范（含逻辑先行哲学）、解答钥匙守则、编辑器能力
     清单、硬规则；references/events.md（事件形状 + 增量语义）与
     references/curriculum.md（题池地图 + 适配规则）；
   - `sokonanoda-dev`：接手清单（REQUIREMENTS→STATUS→ROADMAP→architecture）、
     硬规则、TDD 三层 + 文档先行 + subagent 工作流、CI 门禁形态。
   - 安装方式见 `skills/README.md`（软链到 harness 的 skills 目录）。
2. **conformance 守护**：`crates/cli/tests/skill.rs`（4 测试）——frontmatter
   合法且 name=目录名、引用的仓库路径必须存在、事件/方法词汇封闭且必须被
   `docs/protocol.md` 记载（词汇表抽到 `crates/cli/tests/common/mod.rs`，
   protocol.rs 与 skill.rs 共用）；course.json 的单元/钥匙孪生存在性校验。
   protocol.md 补上 watch 流（Session delta）词汇一节。
3. **opencode LSP 接线**：仓库根 `opencode.json` 把 `sokonanoda-lsp` 挂到
   `.sokonanoda` 扩展名（`cargo run` 启动，无预构建要求）——opencode 等
   agent 打开教学文件即自动消费 kernel 判定诊断；README 增设
   "For code agents" 一节。
4. 测试总量 **233**（+4 skill 套件）；clippy/fmt 门禁维持全绿。

## 本轮进度（2026-09-07，第五轮：I8 真增量 + I9 + 内核修复 + 工程达标）

本轮按"先调研后动手"执行（4 个并行 subagent：代码审计 / LSP 增量业界实践 /
goal 视图 UX / VSCode+CI 标准），设计文档 `docs/design/i8-i9.md`，全程 TDD。

1. **I8 真增量（front，零内核改动）**：学 Lean4/coq-lsp 的"前缀精确复用 +
   变化点后保守重算"。`run_pass` 增加 `TrustPlan`：信任前缀照常 elaborate +
   入环境但**跳过内核重查**（内核检查是贵的那一半）；失败声明不入环境
   （check-then-add 语义保持）。`Session` 快照升级为逐命令
   `{state, hovers, events, errors}`，文本不变 → 零重编译且**修复了注释/空白
   编辑导致的 span 漂移 bug**（重映射坐标）；`first_diff` 之后才重查。
   `SessionUpdate.stats.kernel_checks` 让"改第 i 个声明只重查后缀"可验证
   （测试：5 声明改第 4 → kernel_checks == 2）。LSP Backend 切换到 Session
   （此前每次编辑 2×2 遍流水线，现在前缀零内核重查）。prelude 指令变化时
   整体重建（决策依赖整文件内容，语义与全量严格等价）。
2. **I9 tactic 判定 kernel 化（`front::judge`，零 kernel 原语）**：
   合成完整声明 `def _soko_judge_k : forall binders, 剩余目标 := …术语…`
   走标准流水线，kernel 是唯一裁判。LSP `exact` 与 REPL
   `exact/apply/assumption` 全部接入；`proof.rs` 文本比对删除
   （REQUIREMENTS §2.8 清账）。defeq-但-不同文本的假设（`a -> False` vs
   `Not a`）现在能被识别。goal 视图协议：`soko/goals`（结构化多洞 goal 列表）
   与 `soko/nextHole`（server 端位置计算，ocaml-lsp 教训）两个自定义请求。
3. **内核 soundness 修复（上游 bug，本轮最重要发现）**：judge 端到端测试暴露
   conv `unify_direct` 的 Pi/Lam body-expr 快路径把 **eval 闭包与 infer 闭包**
   按体表达式指针判等（同一 `Var 0` 在两种闭包下是 `$0` vs `Sort 1`），
   `(A : Sort 1) -> A` 这种不可居住类型被身份 lambda 通过（官方 Lean 拒绝）。
   修复 = 快路径增加闭包语义守卫（`closure_ctxs_compatible`，热路径仅一个
   判别分支），回归测试三层（kernel 2 + CLI 2 + 全量语料）。
4. **工程达标（业内标准）**：CI 增加 lint job（fmt + clippy）；`actions/cache`
   → `Swatinem/rust-cache@v2`；`--locked`。lint 门禁形态：教学 crates 在各自
   `Cargo.toml` 用 `[lints.rust] warnings = "deny"` 注入严格度，kernel 冻结
   快照不参与（其 `lib.rs` 的 `deny(cast_possible_truncation)` 降为 warn，
   上游代码自身未过该 lint）。fmt 门禁只覆盖教学 crates（kernel rustfmt.toml
   需要 nightly）。VS Code 打包 P0：`vscode-languageclient` 移到 dependencies
   （此前打出的 VSIX 装上即坏）、补 repository/LICENSE/CHANGELOG/.vscodeignore、
   `vsce package` 冒烟通过。
5. **课程哲学修正（用户插话，已记录 REQUIREMENTS §6）**：逻辑先行——先讲
   True/False/And/Or/Iff/Forall/Exists 让学生在"证明命题"里建立直觉，Sort 等
   到"函数类型的类型是什么"这一自然问题出现时再引入；course/ 与 playground
   按此重排（**下一轮任务**）。
6. 测试总量 **229**（kernel 45 / front 121 / cli 40 / lsp 23），
   全绿；`cargo clippy --workspace` exit-0，教学 crates 0 警告。

## 本轮进度（2026-09-07 第四轮：I8 + I9 后半 + 语义高亮 + watch）

1. **语义高亮（F8，用户要求）**：front 新增 `semantic.rs`（keyword/sort/number/
   hole/声明名/构造子/binder 分类，声明点优先）；LSP 实现
   `textDocument/semanticTokens`（UTF-16 编码正确处理增补平面字符——修掉了
   `span.column` 是字符计数的错位隐患）；VS Code 端 vscode-languageclient
   自动注册，零配置生效。
2. **I9 kernel 显式错误**：6 处 def_eq 失败点（def-like/App 实参/let 体/
   inductive 参数/表达式）在 panic 前用 debug printer 渲染两端（≤200 字符截断），
   稳定格式 `def_eq mismatch expected: <E> | actual: <A>`；front 解析为
   「类型不匹配：期望 `E`，实际是 `A`」并填充 `CompileError.expected/actual`；
   热路径零改动，内核 41 测试全绿。
3. **I8a check-then-add（双趟）**：kernel 拒绝的声明不再占用名字——第二趟在
   干净环境中重算，依赖者得到真正的 unknown-identifier 诊断；归纳块首次纳入
   kernel 判定（`EnvBuilder` 新增 `begin/end_inductive_block` +
   `mutual_block_sizes` 记账，对齐上游 parser；`add_inductive` 返回构建的
   Declar）。由此暴露并修正了 py-nat 系 iota 规则的两处非标准写法：
   规则值必须是 `ms n (Rec.{u} motive mz ms n)`（`.{u}` 不能省）。
4. **I8b Session**（front::session）：版本号、delta 事件
   （exercise.opened/solved/failed、decl.checked/failed）、`recompiled_from`
   日志；内容未变（仅注释/空白）零重编译。
5. **I8c watch**：`sokonanoda watch <file>` 常驻监控 → `file.changed` +
   delta + 诊断的 JSON Lines 流（L1 服务层的 CLI 形态）。
6. 测试总量 **203**（kernel 43 / cli 38 / front 103 / lsp 20... 计 CLI 子套件见
   docs/TESTING.md）。

## I6 落地（2026-09-07 第二轮）

全部四件套已实现并通过 178 个测试（kernel 43 / cli 35 / front 87 / lsp 13）：

1. **prelude 可选化（用户要求）**：`CompileOptions{prelude: PreludeMode::{Full,Bare}}`；
   API `compile_fol_with/check_document_with`；CLI `--bare`；文件级注释指令
   `-- sokonanoda:prelude none`（front::prelude_mode_from_source，CLI/LSP 都认）；
   Bare 模式下 `+` 不再产生悬空 Nat.add 常量（改报 elab-unknown-identifier）。
2. **Eq 三件套 prelude**：`Eq`/`Eq.refl`/`Eq.subst` 以 `.sokonanoda` 源语法书写
   （签名与官方 Lean 一致），受信任安装；文件自带 Eq 系列则整体跳过
   （all-or-nothing，与显式 Nat 块一致）。
3. **binder 类型推断**：`elab_expr` 下传 expected（声明类型逐层剥 Pi），
   `fun n => n + 1` 免写 `(n : Nat)`；依赖情形（`forall (α : Sort u), α -> α`）
   因 de Bruijn 对齐天然支持；声明类型耗尽仍报 `elab-untyped-binder`。
4. **partial hole（部分作答）**：`???` 允许出现在 lambda 体尾部；声明保持
   Open 且 `DeclState.goal` = 剥掉已写 binders 后的剩余目标；洞在非尾部位置
   仍报 `elab-hole-misplaced`。LSP intro quick-fix 的「替换 ??? →
   fun (x : T) => ???」循环第一次真正闭环。
5. **开课**：根目录 `playground.sokonanoda`（12 练习初始全 open、0 诊断、
   exit 0）；`docs/teaching-session.md` = 开课手册 + 事件决策表 + 全部解答钥匙
   （12/12 经完整内核验证，含 `two_def` 闭环）。
6. 新增裸名 `#reduce Nat.add/Nat.succ` 边界测试（I6 验收项，防 delta 循环）。

## 本轮进度（2026-09-07 第三轮：I7 课程层 + I9 goal 视图第一段 + VS Code 修复）

1. **I7 课程层**（用户定位：course/ = agent 路线图，执行层由 agent 按用户灵活
   适配——已写入 REQUIREMENTS §6 与 teaching-session §0）：`course/` 5 单元
   画布 + `course.json` 顺序清单 + `solutions/` 解答钥匙（全部经完整内核验证
   可解）+ `crates/cli/tests/course.rs` golden（每单元 decl.checked/exercise.open/
   expr.reduced 计数钉死）+ CI 步骤。
2. **I9 goal 视图第一段**：开放声明现在携带已引入假设清单
   （`DeclState.binders: Vec<GoalBinder{name, ty}>`，goal_under_binders 同步
   记录）；LSP hover 在 `???` 上显示「目标 + 已引入假设」；code action 新增
   **`exact <假设>`**（类型与目标匹配时自动提议，洞替换为该假设名），
   与既有 `intro` 并存；LSP 测试 13→16。
3. **VS Code 薄壳修复**（依 docs/notes/vscode-notes.md）：修复 `client.start()` 未作为
   disposable 注册的真实 bug（改为正确 start/stop 生命周期）；`sokonanoda.serverPath`
   设置 + 自动发现（workspace target/debug|release → PATH）；`alt+s` 状态命令
   （documentSymbol → 快速选择面板）；codeLens 的 `sokonanoda.status` 命令
   补了客户端 handler（此前点击报 command not found）；新增 F5 启动配置。
4. 测试总量 186（kernel 43 / cli 38 / front 89 / lsp 16）。

## 下一步（交接快照，2026-09-07 第十六轮后；依据 = docs/notes/gap-analysis.md）

> gap-analysis Top 10 已全部清零。以下为运营验证与精选改进。

### 运营验证（需要真实使用）
- ~~**release.yml 首跑**~~ ✅（v0.4.1 起常规 tag 发布已在使用；v0.7.0 起为
  per-target VSIX 发布，见第二十一轮与 `docs/RELEASE.md`）
- **教学回环实战**：逻辑先行画布已就绪——找真实学习者走完 12 题
  （skills/sokonanoda-teacher 循环），回收提示分层与事件决策表的打磨需求
- **opencode LSP 实战**：已确认一次（抓出 Eq.symm 钥匙缺实参），
  后续在教学过程中持续观察

### 精选改进（gap-analysis 余项 + 教学反馈）
- **"错误优先"原则**：同声明已有 error 时抑制 sorry warning
  （Lean AddDecl.lean 的 `!(← MonadLog.hasErrors)` 模式）
- **正向完成信号**：全部练习解出时给绿色装饰（vscode-lean4 双勾✓✓ 模式）
- **completions**：关键字 + 作用域内名字（gap #1，Deduce 实证第一痛点）
- **go-to-definition 增强**：点分名 `And.intro` 整体跳转（已实现），
  可评估 `And` 段跳 `And`（rust-analyzer 段级导航，成本 M）
- **folding range**：声明体折叠（gap #3）

### 远期（L2/L3）
- spine meta 方案 A（kernel 渲染子洞类型）
- VS Code 扩展 marketplace 发布
- L1 service 事件流（watch 已是 CLI 形态）
## 下一批候选（按投入产出比排序）

1. ~~**提示分级 `soko/hints`**~~ ✅（第十一轮）：画布 `-- soko:hint` 指令 +
   `soko/hints` 请求 + VS Code 逐条揭示；playground 12 题已挂阶梯。
   余项：course/ 五个单元的内容阶梯（教学轮补）。
2. ~~**失败洞的"下一步建议"**~~ ✅（第十一轮）：`front::suggest` 按目标形状
   （exact/rfl/refine/intro，kernel 验证优先 + is_preferred）；顺带修复多洞
   错位 bug。余项：失败声明（kernel-rejected）的针对性建议。
3. ~~**`soko/courseStatus` + VS Code 章节地图**~~ ✅（第十二轮）：聚合归
   `sokonanoda course` CLI 子命令（服务器保持单文档），VS Code「课程」树
   消费子进程 JSON Lines；学习者进度=画布自身状态（声明式文件即存储）。
4. ~~**rename + find-references**~~ ✅（第十一轮）：语义集 + 版本化
   documentChanges + ResponseError；shadowing 有回归测试。
5. ~~**inlay hints**~~ ✅（第十一轮）：洞期望类型 + tooltip；只读无 textEdits。
6. **内核错误分类学余项**：~~`conv.rs` 与 `infer.rs` 同名消息区分~~ ✅
   （第十三轮：统一 `got:` 形状、措辞区分站点）；~~`assert_eq!` 灰色地带~~ ✅
   （第十三轮全量清点分诊）；refine 子洞的 kernel 级 expected type
   （elaborator spine meta，M–L）**仍为余项**——需专门设计轮。
   另：~~归纳块 auto-derivation~~ ✅（第十四轮）。~~refine 子洞的 kernel 级
   expected type~~ ✅（第十五轮方案 B′：深度 AST 替换；方案 A 记为远期）。
7. **小项打包**：全部 ✅（lsp 子命令 / REPL 历史 / criterion / fuzz /
   hole_id）。**小项全部清零。**

### 运营/验证类

- ~~**release.yml 首跑验证**~~ ✅（2026-09-10 v0.7.0：per-target VSIX +
  universal 回退包 5 个全部上架 Marketplace，GitHub Release 9 资产齐全；
  见 `docs/RELEASE.md`）。
- **教学回环实战**：逻辑先行画布已就绪（course/ + playground，12 题）——
  找真实学习者走一遍 `skills/sokonanoda-teacher` 循环，回收提示分层与
  事件决策表的打磨需求。
- **opencode.json 实战核查**：LSP 经 opencode 消费的体验（已有一次实战：
  抓出 Eq.symm 钥匙错误）。

### 更远（L2/L3）

VS Code 扩展集成测试（@vscode/test-electron）、发布 marketplace、
L1 service 事件流（watch 已是 CLI 形态）、KernelError 显式化完整推进
（`CheckError::Internal` 目前无人构造）。

## 已确认的决策（用户 2026-09-06）

1. 命名练习：`def name : T` / `theorem name : T`，匿名用 `example`（官方 Lean 的
   `example` 不能带名字；不发明非 Lean 的 `example name : T`）。
2. LSP 框架：用现成 tower-lsp。
3. 范围：I1–I9 全部实现；goal 视图也进第一期。
4. 反馈目标：**足够细致、足够详细**的 LSP（能力清单见 design doc F1–F8）。

## 已完成（按 commit）

| 范围 | 内容 | 位置/commit |
|---|---|---|
| I0 地基 | `--json` 事件、错误 stage/code、语料 CI、examples 语料测试 | `2926347` |
| 文档 | architecture / research / design v1 三件套 | `bf3441b` |
| 设计 v2 | LSP-first、文件无 `#`、练习=带洞声明 | `c1db151` |
| I1 逐声明状态 | `DocumentReport`/`check_document`：open/checked/failed，练习带名字与目标；开放/失败声明不影响后续 | `f23ca71` |
| I2 错误细分 | `ErrorKind` 稳定 code（`elab-*`/`kernel-rejected`）+ 教学 hint（CLI/JSON/LSP 三处） | `f23ca71` |
| I3 类型图 v1 | elaboration 记录每个子表达式 (span, 内核项, binder 作用域)；kernel 新增 `infer_under_binders` → hover 表 | `f23ca71` |
| I4/I5 第一段 | `crates/lsp`（tower-lsp）：diagnostics/hover(类型+目标)/documentSymbol/codeLens/quick-fix `intro`；`editor/vscode` 薄壳 | `f23ca71` |
| 细节 | 事件按源码顺序输出（open 练习排队处理） | `bacb1ee` |

里程碑对照（ROADMAP 第 5 节）：M0 ✅、M1 ✅、M2 大部分（判定细节/提示在 I2 完成；
"期望目标类型 vs 实际" 的 kernel 比对待接）、M3 协议文本+JSON 已实现（service 增量待接）、
M4 课程内容 ❌、M5+（L1 service / L2 完整编辑器 / L3 agent）未开始。

## 测试现状

`cargo test --workspace` 全绿：kernel lib 41（2 ignored：缺 fixture）、arena 1、
memory_api 1、front 49、cli 21、examples 语料 1。LSP 服务器做了手动 JSON-RPC 冒烟
（initialize/didOpen 诊断 0/hover `x: Prop` 与 `???` 目标/documentSymbol/intro
quick-fix/坏声明 `kernel-rejected`）。

## 怎么跑 / 验证

```bash
cd /Users/penglingwei/Documents/lean/sokonanoda/sokonanoda-lang
cargo test --workspace
cargo run -q -p sokonanoda-cli --bin sokonanoda -- examples/lesson-01.sokonanoda
cargo run -q -p sokonanoda-cli --bin sokonanoda -- --json examples/lesson-01.sokonanoda
cargo run -q -p sokonanoda-cli --bin sokonanoda repl          # #check/#reduce/#prove 调试
cargo run -q -p sokonanoda-lsp --bin sokonanoda-lsp           # LSP（editor/vscode 使用）
```

## 待办（按依赖排序，全部已入 ROADMAP §10）

- **I6 prelude 对齐 + elaborator 推进**：Bool/Eq/rfl 等受信任基元；核对 Nat.succ/Nat.add
  占位自引用体；binder 类型推断 → `let` → `match`。验收：每个语法点 TDD 三件套。
- **I7 第一门课（M4）**：5 单元（表达式与类型 / 函数与箭头 / 命题与证明项 / 等式与 rfl /
  归纳与 match）× 3–8 练习，`course/` 目录 + golden 事件；CI 全绿。
- **I8 真正增量**：check-then-add（失败的声明不进环境），编辑一行只重查受影响后缀；
  事件带版本。
- **I9 kernel 显式错误 + goal 视图**：panic→`KernelError`（conv 差异给两端项）；
  `#prove` 逻辑入库，LSP 多洞 goal/refine/code action。
- **L2/L3（后续）**：VS Code 扩展打包（语法+进度树+goal 面板）、L1 service 事件流、
  讲课 agent 接入同一文档状态。

## 给接手 agent 的提醒

- 判定永远走 kernel，不做文本比对（`proof.rs::assumption` 的文本比对是草案，待替换）。
- 新增语法 = 课程 + 测试 + 白名单；`???` 只允许在声明值位。
- kernel 拒绝目前仍是 panic→`Result`（`try_check_declar`）；细粒度 kernel 错误是 I9。
- arena 生命周期：`EnvBuilder`/`ExportFile` 挂同一 `stumpalo::Arena`，必须活得比检查会话久。

## 本轮进度（2026-09-07，接手 agent 第 1–3 轮）

1. **模块化重构（用户要求：不得单文件巨石）**：`front/lib.rs`→
   `span/token/ast/diagnostic/parser + compile/{mod,error,event,report,elab,prelude,check}`；
   `cli`→`main/check/json_report/repl/help`；`lsp`→`main/render/actions`；
   公开 API 全部 re-export 保持稳定；**kernel 一行未动**（性能原则）。
2. **全流水线测试资产（157 tests 全绿，见 `docs/TESTING.md` 地图）**：
   kernel 41+2 / arena 1 / memory 1；front 49→**74**（lexer 10 / parser 10 /
   compile 51，含 ErrorKind 矩阵、DocumentReport 状态机、doc-conformance、perf 冒烟）；
   cli 21→**21+8**（新增 protocol golden：封闭事件词表、lesson-01/02 金字、
   协议文档防漂移）；**lsp 0→10**（内存内 LspService 协议级集成测试）。
3. **测试揪出并修复的真实缺陷**：
   - LSP `intro` quick-fix 行列 +1 偏移（actions.rs 1-based→LSP 0-based）；
   - publishDiagnostics 补 `version`；didChange 改取最后一个 change（FULL sync 语义）；
   - `--json` 的 elab/kernel diagnostic 补 `hint` 字段（protocol.md 本就承诺）；
   - `docs/protocol.md` 补齐 6 个缺失 elab 错误码（doc-conformance 测试守护）。
4. **文档**：新增 `REQUIREMENTS.md`（用户全部要求的权威总账）、
   `docs/TESTING.md`（测试资产地图）、`docs/notes/lsp-notes.md`、`docs/notes/vscode-notes.md`。

## 本轮进度（2026-09-18，第九十七轮：真 VS Code 集成测试例行化 + 结果台账）

> 用户：「你配置相关套件，启动 VSCode 实际验证一下，本来就应该做成例行化检测。
> 远程不行，本地例行化也可以接收。」——原来只有 `cd editor/vscode && npm test`
> 这条"想起来才跑"的手工路径，且极易测到旧二进制；本轮把它做成**一条命令 +
> 提交进仓库的台账**，并用它真跑了一遍。

1. **一条命令**：`SOKO_VSCODE_TEST_VERSION=1.138.0 scripts/vscode-e2e.sh` ——
   构建 release（被测的就是发布形态）→ `node scripts/stage-lsp.js` stage 到
   `bin/<target>/` → `npm test`（真 VS Code + 真 LSP + 真扩展宿主）→ 记账。
   退出码 0/1/2/3（全绿/用例失败/用法/前置缺失）。
2. **台账（提交进仓库）**：`docs/e2e/ledger.jsonl`（`schema: soko.e2e/1`：
   version/commit/dirty/host/VS Code 版本/`tests{passed,failed,pending}`/exit/
   doctor 的服务器版本行/被测 LSP `sha256` 前 16 位/裁剪日志路径）+
   `docs/e2e/latest.json` + `docs/e2e/logs/<date>-<sha>.log`（doctor 块 + 用例
   清单 + 扩展接线日志 + 失败详情）。
3. **本轮实测（真宿主）**：**14/14 全绿**，用时 ~1s（外加 VS Code 启动与首次
   下载）；doctor 自述 `0.58.0 (pid …) == 扩展 v0.58.0 (source=bundled)`。
   新增 4 条用例：`.sokonanoda` 语言 id 守卫 + **项目树三条**（真 `soko/project`
   答案渲染的行：闭包 / 单文件占位 / 缺模块根因与错误图标）。
4. **过程里修掉三个真问题**：
   - `.vscode-test.mjs` 把 `--user-data-dir`/`--extensions-dir` 指到
     `<tmpdir>/soko-vscode-test`：macOS 的 unix socket 路径上限 103 字符，本仓库
     的长路径原先直接 `EINVAL` 起不来（老文档让你把扩展拷到 `/tmp/v`，现在不必）；
   - 扩展的 test-mode 返回钩子**提前 `return` 掐掉了 `client.start()`**——测试宿主里
     服务器永不启动，10 个用例集体超时（stub 层看不见这类生命周期问题）；改成
     **函数末尾**返回并写清为什么；
   - 新增 `SOKO_E2E_LOG` 文件日志（env 开关、生产零成本）：扩展宿主的 `console`
     在 `vscode-test` 输出里取不到，这条日志是 e2e 卡住时的第一现场（本轮正是靠它
     定位到上面那条）。
5. **文档**：新增 **`docs/E2E.md`**（一条命令、四层分工、台账字段、判读口径、
   环境坑、与 CI 的关系）；`docs/vscode-dev-guide.md`（测试三层 + 坑 19/20/21 +
   坑 14 更新为"配置已自解"）、`docs/TESTING.md` 集成测试小节、`AGENTS.md`
   （命令 + 扩展改动后的例行三层）、`skills/sokonanoda-dev`、`docs/README.md`
   地图、`docs/LESSONS.md`（"给扩展一条文件日志"）同轮同步。
6. **CI 也跑这条命令（0.58.0 同日）**：新增独立 **`e2e` job**（矩阵
   `ubuntu-latest` + `xvfb-run` 与 `macos-latest`，各自钉 VS Code 版本），跑的就是
   `scripts/vscode-e2e.sh`；`docs/e2e/` 上传为 artifact，`scripts/e2e-summary.py`
   的渲染写进 **job summary**；`auto-tag` 的 `needs` 加上 `e2e` ⇒ **e2e 红了不发版**。
   原来 `test` job 里那条 `xvfb-run npm test` 删除（避免同一套用例跑两遍）。
7. **CI 的 macOS 腿只在 push 到 main 时跑**（用户定：PR/分支只跑 Ubuntu，快反馈；
   main 上才加跑 macOS——真宿主差异值得守，但每个 PR 多 ~10 分钟不划算）。
   e2e job 用 job 级 `if`（`matrix.os != 'macos-latest' || push && main`），
   被跳过的腿不影响 `auto-tag` 的 `needs`。`run:` 块逐个过 `bash -n`，
   YAML 解析校验通过（GH Actions 本身推不了，没法在这里真跑）。
   最低版本 1.106.0 腿按用户规矩**先本地验证再进 CI**——同一天用
   `npm_config_https_proxy=http://127.0.0.1:7890` 验过：**VS Code 1.106.0 上
   14/14 全绿**（含项目树三条），台账 `b0bcba3`；随后把它加进 CI 矩阵
   （ubuntu × 1.106.0，每个 PR 都跑）。顺带把"test-electron 只认
   `npm_config_proxy`/`npm_config_https_proxy`、不读 `HTTPS_PROXY`"写进
   `docs/E2E.md` §5/§6。
8. **CI 台账回提交（用户：「验证好就让 CI 往仓库追加吧」）**：新增收尾 job
   `e2e-ledger`（只 main，`contents: write`）——下载各腿 artifact →
   `scripts/e2e-merge.py` 合并（**幂等**：重复条目跳过、日志按记录名回填、
   台账按 date 排序）→ 一条提交推回 main（标题带各腿结果）。为什么不是每条腿各推：
   矩阵并发改同一个 `ledger.jsonl` 会互相覆盖；push 前 rebase 重试一次，
   两次都失败就报错（不静默）；`GITHUB_TOKEN` 推的提交不再触发 workflow（不自激）；
   `e2e-ledger` **不**进 `auto-tag` 的 needs（免得与它自己推的提交互相等待）。
   日志文件名同时改成带版本（`<date>-<sha>-vc<version>.log`），否则矩阵里同一天
   同一 commit 的多个版本会互相覆盖。合并逻辑在本地用**伪造 artifact** 验过：
   追加 2 条 → 再合并 0 条（幂等）→ `--check` 排序/唯一/日志齐全。
   加固（同日）：`concurrency: e2e-ledger`（同一时刻只有一个写台账的 job）+
   `fetch-depth: 0`（浅克隆 rebase 缺 parent）+ push 重试 3 次、冲突时报出
   `UU` 文件并 abort。**两条路径都用临时 bare remote + 两个 clone 演练过**：
   ① 抢占 push → rebase → 第二次成功；② 同一文件冲突 → abort + 退出码 1、
   工作区干净（重跑即可）。
9. **版本策略（调研后决定）**：`@vscode/test-cli` 的 `version` **默认 stable 频道**
   （[官方文档](https://code.visualstudio.com/api/working-with-extensions/testing-extension)
   与官方 sample 都不钉具体版本，示例用的是 `insiders`）——生态惯例是跟频道。但这一层
   的产物是**台账**：宿主随 stable 漂就没法比历史，所以本地例行默认钉 **1.138.0**
   （`--version stable` 可跟随），CI 矩阵显式给版本；升级流程与"最低版本
   （`engines.vscode ^1.106.0`）腿待补"写在 `docs/E2E.md` §5。
10. **顺手清掉不再需要的缓存**：`editor/vscode/.vscode-test/vscode-darwin-arm64-1.137.0`
   （898MB，已换 1.138.0）、旧 VSIX×5（0.18/0.19/0.20 + universal + `sokonanoda.vsix`，
   `docs/RELEASE.md` 的发布流程会重新产出）、`.ruff_cache/`（仓库没有 ruff 配置）。

## 本轮进度（2026-09-18，第九十六轮：待办批次 4 —— 项目状态视图，0.58.0）

> 承第九十四轮定下的批次计划（用户「按照你的计划，从上到下依次改进」）：
> **批次 1/2/3 已完成，本轮做批次 4 = `soko/project` 项目状态可视化**。
> 「我在哪个项目里、根在哪、清单是谁、哪个模块拖坏了入口」以前只能靠 CLI 反复
> 跑或读文档推；现在它是一个只读、机器可判的查询，编辑器与 agent 同一份真相。

1. **真相层（front）**：新增 `project::ModuleStatus {Compiled, LoadFailed, Blocked}`
   + `ModuleReport::status`——此前"编译过（可能有错）/ 加载失败 / 被上游拖住"三者
   都表现为空报告，消费者分不清**根因与受害者**；`compile_plan` 按
   `failed`/`blocked`/`result_blocked` 三个已知集合填状态。
   `query::ProjectView`（wire）+ `QueryDoc::project_view()` /
   `project_view_reason()`：从**已编译的** `ProjectReport` 派生（不重跑内核、
   不算摘要、不碰缓存），路径 `canonicalize` 成绝对路径（CLI 与 LSP 对同一文件
   给出逐字相同答案）；单文件是**另一种合法状态**（`None` + `no-imports` /
   `no-path` / `parse-error`），不是错误。
2. **三个传输同一份真相**：CLI `query project`（`soko.query/1` 信封、
   `data = {project, reason}`、恒退出 0）+ help 行；MCP 工具 `project`
   （`mcp__sokonanoda__project`，薄转发，`dsh.rs` 契约从六工具改七工具）；
   LSP `soko/project`（回显 `uri`/`version`，走 `focus_request` + 未保存缓冲）。
3. **VS Code 0.58.0**：资源管理器新增「项目」树（新模块
   `editor/vscode/project-tree.js`：渲染与请求分离）——根 = 模块根 + **清单来源**
   （`sokonanoda.toml` 或"零配置"）+ 计数；子 = 拓扑序模块 + `入口`/`依赖` +
   声明/练习/错误 + 状态图标 + `message`（根因说出来缺哪个模块）；点击开模块、
   点根开清单；单文件一条占位行；状态栏 tooltip 加项目行（不新开 item）；
   `sokonanoda: refresh project view` 命令 + view/title 按钮；答案指名别的文档
   ⇒ 丢弃（沿用 `soko/goals` 的身份纪律）。
4. **测试（三层）**：front 4 条（闭包/清单/失败 vs 被阻断/单文件原因）；
   CLI 3 条 e2e（真二进制：字段齐全、根因 vs 受害者、`project:null`+reason）；
   LSP 2 条（身份回显 + 未落盘编辑改坏 import ⇒ 入口 `load-failed`）；
   扩展 stub 宿主 3 条（渲染闭包/单文件占位/丢弃他人答案）。
   顺手修好 stub 的两处不忠实（`MarkdownString` 吞掉构造参数——**测试因此看不见
   tooltip 内容**；`createStatusBarItem` 不返回实例）并清掉 5 行遗留 DEBUG 打印。
5. **版本与文档**：0.57.0 → **0.58.0**（Rust 与扩展同步，契约测试逼出来的）；
   新增设计 `docs/design/project-view.md`（§9 明确不做依赖图/写操作/模块级缓存）；
   `docs/protocol.md`（`query` op 表 + `soko/project` 小节）、TESTING（新行 +
   七工具）、architecture（仓库地图 + §4.5 第 7 步）、HANDOVER（LSP 能力/新字段）、
   AGENTS（命令面 + 七工具 + 自定义请求表）、skills、dsh/README、
   扩展 README/CHANGELOG、LESSONS（stub 忠实性）、本文件与 `REQUIREMENTS.md`
   §9（九十六）。
6. **验收**：`cargo test --workspace --locked` **871 passed / 0 failed**
   （front 466（457 + perf 3 + perf_project 6）/ cli 217 / lsp 137 / kernel 51）；
   `node editor/vscode/test-extension-host.js` **11/11**（另三个 Node 套件
   18/18、7/7、10/10）；`scripts/soko gate` PASS；site 数据重新生成。
7. **批次 1–4 全部完成**。剩下的只有 P7 长尾（`[deps]`、`namespace`/`open`、
   `watch` 项目模式、decl 级产物）与 `docs/HANDOVER.md` §4 的结构债清单
   （`compile/tests.rs` 4828 / `elab.rs` 2854 / `parser.rs` 2065 / `lsp/lib.rs` 1554）。

---

   （仓库约定：主线收尾统一跑）。

## 本轮进度（2026-09-18，第一百轮（语言线）：WO-003 / G-10（+ 同族 G-17）落地 —— 查询通道不再假绿）

- **G-10：`query check` 对解析失败报 parse 诊断 + exit 1**。病根是
  `front::query::QueryDoc.parse_error` **只写不读**（只在 `set_text` 写、
  `set_cached_entry` 置 `None`，全 `front::query` 没有读者）：`check` 只读
  `self.output`，于是"这份文本解析不了"被答成"全零 + `failed: []` + `ok:true` +
  exit 0"，而同一份文本走 `grade` 是对的。现在 `check` 把 `parse_error` 合成进
  `failed[]`（`name: null`、`code`/`message`/span 与 `--json` 同源；`counts` 保持
  全 0 = 诚实、`ok` 保持 `true` = "问出来了"，退出码由 `failed` 数量推导 ⇒ 1）。
  缓存路径的不变量（`set_cached_entry` 清 `parse_error` 之所以安全）钉进了 front 单测。
- **G-17（同族，顺带）：`query goals`/`holes` 对解析失败不再答空数组**。两者与
  `next_hole` 改返回 `Result<_, QueryError>`：解析失败 = `NotParsable` ⇒ CLI 答
  `ok:false` + `error.code:"not-parsable"` + exit 1（"空数组 / `None`" 仍然只表示
  "正常的没有"）。`state` 不动（它本来就 `ok:false`，WO-003 明确排除其形状变更）。
  LSP wire 不变（parse 诊断走 `publishDiagnostics`；`soko/goals` 空、`soko/nextHole`
  `null`）。
- **测试三层**：front 单测 3 条（`check_reports_a_parse_error_instead_of_all_zeros`、
  缓存不变量、`goals_and_holes_report_a_parse_error_instead_of_empty_answers`）+
  CLI e2e 4 条（`--text`/`--file` 两条通道、G-17 三调用 + 对照、坏依赖项目回归、
  真课程单元⑤ 正例 5 checked/7 open + 反例与 `grade` 同 code/span/退出码）+
  课程门禁 `python3 courses/set-theory/tools/check.py` 同数（34 目标 · 308 checked ·
  93 open · 0 判负）。复现 `docs/gaps/repro/G10-query-check-parse-error.sh`（重写成
  修后形状，exit 1）与新写的 `G17-...sh`（exit 1）。
- **契约变更（用户可见）**：`query check` 的退出码 0→1（解析失败）；`query goals`/
  `holes` 退出码 0→1 且 `ok:false`。`--json` 事件流与两处课程 GOLDEN **一字未动**
  （改动全在 `front::query` 摘要视图 + CLI 信封）⇒ minor（0.59.0，版本号由主线 bump）。
  文档 as-built：`docs/protocol.md`、`docs/design/agent-query-channel.md`、
  `dsh/mcp/server.js` 三个工具描述、`skills/sokonanoda-teacher/SKILL.md`。

## 本轮进度（2026-09-19，第一百〇四轮（语言线）：L-01/L-02 落地 —— L1 prelude 装上逻辑与等式骨架）

> 台账 blocker：prelude 只有 `Nat`/`Bool`/`Eq` 三家，Lean core 的**逻辑与等式骨架**
> 全缺——课程侧只能靠 `courses/set-theory/lib/Logic.sokonanoda` 手写 26 条兜底
> （"暴力"的根源）。设计 = `docs/design/prelude-l1-proposal.md`（本轮的提案文档，
> 已有逐条签名 / 让位规则 / 三件套计划 / GOLDEN 预测数值）。

1. **P1 安装 + 让位**：`crates/front/src/compile/prelude.rs` 新增
   `PRELUDE_L1_SRC`（规范源文本，28 条声明）、`L1_FAMILIES`（B1–B7 族表 +
   依赖边）、`install_l1_prelude`（axiom 走 `build_axiom`、def 走 `build_def`、
   归纳块走 `install_inductive_block`——**与用户声明同一条 elaborator**）。
   让位粒度 = **族**（不是单名），族间按依赖闭包（B5→B2、B6→B3、B7→Eq）；
   触发集合 `taken` 从 `user_top_level_names` 换成 **`top_level_def_spans_over`
   的键集**（补上构造子/递归子，设计 §2.3-1）。
2. **两个 as-built 修正（提案未预见，都是实测出来的）**：
   * **顺序：先 Eq 后 L1**——B7 的定义体引用 `Eq.subst`/`Eq.refl`，必须等 Eq 进环境；
   * **重入闸 `L1_INSTALL_DEPTH`**——装 `And` 归纳块时
     `large_elim_test_mirror` → `field_sort_via_kernel` → `judge_infer` →
     **内层 `compile_fol_with`**，内层又装 L1 ⇒ 无限递归（实测
     `stack overflow, SIGABRT`）。计数 > 0 时 `install_l1_prelude` 直接返回。
   另修正提案 §3.2 一处**方向写反**的措辞（依赖边是 B6 用 `And.left`，
   所以声明 `And` 让位 B6，反之不成立；§2.2 的表是规范）。
3. **P2 白名单与豁免面**：`PRELUDE_NAMES` 补 30 条（12 → **42**）；
   拆出 `PRELUDE_NEVER_YIELDS`（只含 `Nat`/`Bool` 家族）给
   `check_name_collisions`——L1 名字按族合法让位，**不能**进豁免面，否则两个模块
   各自声明 `True` 就不再报友好的 `import-name-collision`（设计 §2.3-2）。
   `goals.rs` 的 `GoalTemplates` 按**同一条让位规则**吃 L1 源文本。
   parser 白名单**零改动**（L1 不引入新语法）。
4. **P3 课程用例 + 两处 GOLDEN 据实重算**：单元②（唯一不声明 L1 名字的早期单元）
   中英画布加 `eq_symm_demo`（用 prelude 的 `Eq.symm`，一行）＋ 两题 hint 改
   "两解对照"；两份解答同步。真二进制实测：`unit2 = (3,5,2)`、
   `course_status` 的 `unit2 = (3,5,0,2)`、summary `checked 85 → **86**`
   ——**与提案 §4.2 的预测值逐字相同**。另发现**第三处** GOLDEN
   （`cli.rs::cli_course_is_stable_with_a_warm_cache` 也钉 `checked = 85`）⇒ 同步。
   `course_shared.rs`（44 份副本一致性）**一字未改而全绿**——这就是"让位"生效的证据。
5. **课程侧兜底保留（硬要求）**：`courses/set-theory/lib/Logic.sokonanoda` 的
   28 条声明**一条没删**，文件头加了"prelude 现在自带哪些（30 个名字 + 让位规则 +
   两处定形差异）"，避免两处真相打架。门禁复跑 **315 checked · 96 open · 0 判负**，
   与 L1 落地前逐字相同（让位 ⇒ 课程语义不变）。**P4（74 处 `inl`/`inr` 项位改
   点号名、`lib/Logic` 退化成空壳）未做**。
6. **复现件（台账契约）**：新增 `docs/gaps/repro/L01-l1-prelude-logic-skeleton.sh`
   与 `L02-eq-core-lemmas.sh`，四段自断言（Full 可用 / Bare 干净 / 让位依赖闭包 /
   `Or` 是真归纳块）。修前两者 exit 0（缺口仍在），修后 **exit 1**（修后形状成立）；
   `L-01`/`L-02` 已 `gap.py close --version 0.59.0`。`L-03`（Type 层重写）**仍 open**
   （B7 只装同宇宙三引理，`Eq.subst` 的 motive 仍是 `α -> Prop`），notes 已注明。
7. **三层测试**：front 7 条（`l1_prelude_is_available_in_full_mode`、
   `l1_or_is_a_real_inductive_for_match`、`l1_family_yield_is_dependency_closed`、
   `l1_yield_needs_the_whole_family`、`l1_taken_includes_ctors_and_recursors`、
   `l1_yield_is_closure_wide`、`l1_ctor_templates_feed_sub_goal_types`、
   `prelude_names_match_installs`、`eq_symm_is_installed`、`bare_mode_has_no_l1`）+
   CLI 4 条（Full / `--bare` / 注释指令 / `query check` ↔ 事件流计数一致）+
   课程 GOLDEN 两处 + 复现件两个。
8. **验收**：`cargo build -p sokonanoda-cli -p sokonanoda-front -p sokonanoda-lsp` 过；
   `cargo test -p sokonanoda-front`（522）· `-p sokonanoda-cli`（全套）· `-p sokonanoda-lsp`（141）
   全绿；`cargo fmt --check` / `cargo clippy`（教学 crates）零警告；
   `python3 courses/set-theory/tools/check.py` 绿；`python3 scripts/gap.py check` 全绿。
   **未跑 `scripts/soko gate`、未 bump 版本、未 commit**（仓库约定：主线统一）。
9. **文档同轮**：`docs/architecture.md` 新增 §5.4.1（族表 + 让位规则 + 两个 as-built
   要点 + 白名单/豁免面）+ §4.1 白名单行注明"prelude 名字不属于语法白名单"；
   `docs/design/course-stdlib.md` §2 更正"And 走 axiom 族"→"真归纳块 + 点号构造子"
   并标注已实现，§3.2-A 的 G-02 行注明 L1 已绕过；
   `docs/design/prelude-l1-proposal.md` 加 as-built 节；
   `docs/design/teaching-project.md` P-C3 ✅ / L-01 行；
   `docs/TESTING.md` 新增 L1 行；`docs/HANDOVER.md` 版本表 +1 行；
   `skills/sokonanoda-teacher/SKILL.md` + `references/curriculum.md`（L1 词汇与让位规则）；
   `editor/vscode/CHANGELOG.md` 记一行（补全列表多 30 个名字 = 用户可见改动）。

<!-- 以下由 STATUS.md 轮转追加 -->

## 本轮进度（2026-09-19，第一百〇六轮：0.59.0 收尾（语言线五刀 + 课程门禁 + 站点页））

> 本轮**只做版本与文档**：把第九十九～一百〇五轮攒下的用户可见改动收成 **0.59.0**，
> 把课程门禁接进 `scripts/soko gate` 与 CI，把卷 I 搬上站点。硬规则 1（内核冻结快照）
> 与硬规则 6（用户/agent 路径零工具链）全程未动：`crates/kernel/` 与 `crates/` 下任何
> 源码**本单零改动**（语言线代码在前几轮已落地，工作树里原样保留）；判定仍只走内核与退出码。

1. **版本 bump（0.58.0 → 0.59.0）**：`Cargo.toml` 的 `[workspace.package] version`
   与 `editor/vscode/package.json` 的 `version`（两处必须一致，契约测试
   `crates/cli/tests/extension.rs::cargo_and_extension_versions_match` 守着）；
   `cargo metadata --format-version 1` 让 `Cargo.lock` 跟上（3 个包：cli/front/lsp）。
   **课程侧版本钉同步**：`courses/set-theory/sokonanoda.toml` 的 `requires` 0.58 → 0.59
   （WO-003 / WO-007 的课程侧收尾项；记法对照页本身就要 ≥0.59.0）。
   实测：`target/debug/sokonanoda --version` = `sokonanoda 0.59.0`。
2. **这一版装了什么（全部用户可见，逐条对应轮次与设计）**：
   * **签名受检**（WO-004 / G-01，第一百〇一轮）：值位是 `sorry` 时签名也要 elaborate
     并过内核的「是不是类型 / `theorem` 的是不是 Prop」；坏签名 = 一条 diagnostic
     （span 取签名自身）+ 声明 `Failed` + **不发** `exercise.open`——判卷只认
     `decl.checked` 与 `diagnostic`，`exercise.open` 计数对签名腐烂永远是盲的；
   * **构造子命名空间**（WO-005 / G-02，第一百〇二轮，`docs/design/ctor-namespace.md`）：
     规范名 `Ind.ctor`，裸名降为解析别名（闭包内唯一；撞名报 `elab-ambiguous-ctor-alias`）；
     源文件自带 `inductive Nat` 时归约形态变化已逐条实测重钉；
   * **Prop + Type 参数 + 单构造子的归纳**（WO-006 / G-03，第一百〇三轮，
     `docs/design/prop-large-elim-mirror.md`）：派生 recursor 的宇宙参数逐字镜像内核
     （不再 panic）；**课程侧出口本轮收掉**：`courses/set-theory/lib/Exists.sokonanoda`
     从公理三件套升级成**真归纳**（`Exists.intro` 是构造子、`Exists.elim` 由自动派生的
     `Exists.rec` 定义，名字与签名逐字不变 ⇒ units/ 的点名调用零改动）；
   * **L1 prelude**（L-01/L-02，第一百〇四轮，`docs/design/prelude-l1-proposal.md`）：
     Full 模式自带 Lean core 的逻辑与等式骨架 **30 个名字**（`PRELUDE_NAMES` 12 → 42），
     **族粒度让位** ⇒ 入门课"自建骨架"的教学一个字不用改；
   * **用户自定义记法第一刀**（WO-011 / G-04，第一百〇五轮，
     `docs/design/notation-subset.md`）：`infix:N`/`infixl:N`/`infixr:N`/`notation` +
     数学符号独立 token + elab 内源到源重写（自动补前导类型参数）；**文件内作用域**、
     **零事件**、点名形式永久可用且两种写法判卷一致；`𝒫`/`''`/`⁻¹'`/`×ˢ` 留第二刀；
   * **`course` 认 `import`**（WO-007 / G-06，第九十九轮）与 **`query check` 同口径**
     （WO-003 / G-10 + G-17，第一百轮）：前者让聚合与单文件 `grade` 同判（`failed == 0`
     ⇔ `grade` exit 0），后者把"这份文本解析不了"从假绿翻成 `failed[]` + **exit 1**
     ——**这是有意的契约变更**（同口径后 `query check` 可作 `grade` 的交叉复核）。
3. **课程门禁接进本地与 CI**（唯一真相 `courses/set-theory/tools/check.py`；设计
   `docs/design/course-gate-in-ci.md`，as-built §9）：判据 **G1–G5 与规模无关**
   （`grade` 退出码 0 / 目标存在 / 解答 0 open 且 checked>0 / 解答覆盖画布每个具名练习 /
   lib+Demo 0 open）；`scripts/soko gate` 里 python3 探不到 ⇒ **exit 3**（无法判定 ≠ 绿），
   cargo 门禁绿了才跑课程门禁并把**解析到的**二进制经 `SOKONANODA_BIN` 透传；
   `ci.yml` 的 `test` job 新增 `--selftest` + `--annotations --report --summary` step
   （用当轮 `target/debug` 二进制，**不新建 job** ⇒ 课程红自动挡住 `auto-tag` 的发布）
   + `course-gate-report` artifact。
4. **站点卷 I 页面**：`site/set-theory.html`（零构建 HTML）上线；数据块由
   `scripts/gen-site-data.py` 生成——版本读 `Cargo.toml`、轮次标题读本文件、
   **计数由课程门禁实测**（`counts_source: "gate"`）——三样都不许手写；
   `scripts/check-site.py` 绿。
5. **文档同轮**：本文件轮转（第一百〇三轮移入 `docs/STATUS-ARCHIVE.md`，归档头部范围
   1–102 → **1–103**）；`docs/HANDOVER.md` 快照 → 0.59.0 + §2/§3 的 as-built 汇总；
   `courses/set-theory/README.md` 现状表**据实重算** + 补记法对照页 / Exists 升级；
   `docs/design/teaching-project.md` 附录 A 的 fixed/0.59.0 状态与 P1/P-C 收尾；
   `REQUIREMENTS.md` §9 追加本条。
6. **课程门禁实测（0.59.0 二进制）**：**36 个目标 · 355 checked · 99 open · 0 判负**
   （canvas 96 / solutions 0 / lib 0），`--selftest` exit 0。比上一条记录多 2 个目标
   （记法对照页 + 它的解答，07:5x 落地）——课程在长，所以门禁**只判形状、不锁计数**。
7. **验收**：`grep -n "^version" Cargo.toml` = `0.59.0`；
   `grep -n "\"version\"" editor/vscode/package.json` 首行 = `0.59.0`；
   `cargo test -p sokonanoda-cli --test extension` 33 passed（含版本契约）；
   `python3 courses/set-theory/tools/check.py` exit 0、`--selftest` exit 0；
   `git diff --stat crates/` 本单零改动（工作树里第九十九～一百〇五轮的语言线改动未动）。
   `scripts/soko gate` **全绿 exit 0**（含课程门禁与新增的台账门禁）；
   **未 commit**（仓库约定：由主线统一落 commit）。
8. **缺口台账收口（主线，同日）**：把「缺口即测试」从**人肉纪律**变成**门禁**——
   * `scripts/soko gate` 第四步 = `python3 scripts/gap.py selftest` + `check`；
     `ci.yml` 的 `test` job 同款 step `Gap ledger is consistent (docs/gaps)`
     （`SOKONANODA_BIN` 指当轮 `target/debug`，~3 s，**不新建 job**）。红了 =
     语言变了而台账没跟上（或修好忘了关账）；
   * **台账新增 `repro_expect`**（`clean`/`rejected`/`exit0`/`nonzero`）覆盖
     "由 status 推导期望"的默认：**有些缺口的「修好」恰恰是判红**——G-01 就是
     （复现件钉的是「签名写错必须被拒」），已写 `"repro_expect":"rejected"`；
     取值与复现类型不匹配会直接判不一致（写错的字段不会被默认推导悄悄盖过）；
     `gap.py selftest` 14 条判据钉住判定规则本身（含 4 种取值 + 2 种非法形态）；
   * **G-09 关账**：包装层早已把 `assertion failed:`/`unwrap()` 归 `kernel-internal`
     + 「这不是你的代码问题」提示（测试钉住），唯一已知可达触发路径随 G-03 关闭 ⇒
     改判 `fixed` 并**撤下 `repro`**（与 G-03 共用、已转绿），两半结论写进 `notes`；
   * 结果：`python3 scripts/gap.py check` **exit 0 全绿**，`gap.py list` =
     24 条里 18 条 `fixed_in=0.59.0`、未关账 6 条（L-04 `workaround` + 5 条
     `painful`/`nice`：L-03/G-05/G-07/G-08/L-06）；`.gitignore` 补
     `__pycache__/`（python 工具已是仓库一部分）；`docs/gaps/README.md`、
     `docs/design/teaching-project.md`、`skills/sokonanoda-{dev,ci}` 同轮同步。
   * **第一次真跑就抓到一个真 bug（本台账门禁自己的）**：`gate` 把解析到的二进制经
     `SOKONANODA_BIN` 透传给课程门禁的同时也透传给了台账门禁，而 G-11/G-16 的复现
     **测的就是启动器自己的解析链**——夹具里那个「陈旧缓存必须被拒绝」的现场被显式覆盖
     绕过，两条复现假报「缺口仍在」、gate 因此 exit 1。修法：`gap.py` 新增 `clean_env()`
     剔除 `SOKONANODA_BIN`/`SOKONANODA_LSP_BIN`（`selftest` 钉住）、`gate` 对台账那一跑
     **不透传**、复现脚本自身再加一行 `unset` 兜底。教训见 `docs/LESSONS.md`
     （「门禁注入的环境变量会短路复现夹具」）。
9. **WO-010 / G-15（诊断坐标自描述）**：`query check` 的 `failed[]`/`warnings[]` **新增**
   1 基 `start_line`/`start_col`/`end_line`/`end_col`——**只加不删**（`start`/`end` 仍是
   字节 offset、坐标空间 = 入口文件；schema 号、事件种类、双 GOLDEN 都不动）。
   台账原记的「内核 span 漂到别的声明」是**量具缺陷**（复现脚本把字节 offset 当字符下标），
   真缺口是坐标不自带单位与坐标空间。守护三层：front 两条（span 的字节切片逐字等于出错命令，
   收紧原来那条只断言 `line >= 1` 的假守护）+ CLI e2e 一条（`failed[]` 行列 ≡ `grade --json`
   的 span、依赖只以入口 `import-dependency-failed` 出现）+ 复现重写（修前 exit 0 / 修后
   exit 1，双二进制对照实测）。同轮同步 `docs/protocol.md`、`docs/TESTING.md`、
   `courses/set-theory/AGENTS.md` 的判卷纪律、`dsh/mcp/server.js` 的工具描述。
10. **P4 课程跟随 prelude**：`courses/set-theory/lib/Logic.sokonanoda` 那 26 条声明
   **退化成只有注释的空壳**（prelude 已自带同名 30 个，34 处 `import lib.Logic` 一字未改），
   课程侧 **65 处项位裸名** `inl`/`inr` 改点号名 `Or.inl`/`Or.inr`（G-02 定形后裸项名已不存在；
   裸**模式**仍被接受，为一致性一起改）。课程门禁 **36 目标 · 329 checked · 99 open ·
   0 判负**——`checked` 少掉的 26 条正是删掉的重复脚手架，`open` 一条不变（= 没删练习、
   没加 `sorry` 的机械证据）。L-01/L-02 台账 `notes` 补记 P4 已跟随。
11. **发布结果（2026-09-19 实测）**：push `main`（`347bd83`）→ 第一次 CI **红**在 LSP 项目
   哨兵（下一条），修好后重推（`3c145e9`）→ CI **7/7 job 全绿**（lint / test / e2e ubuntu×2 /
   e2e macos-latest / e2e-ledger / **auto-tag**）→ auto-tag 打 **`v0.59.0`** 并 dispatch
   `release` → release **11 job 全 success** → GitHub Release **26 资产**
   （lsp ×8 / cli ×8 / vsix ×9 / `SHA256SUMS`）+ Marketplace 收录 **0.59.0**
   （2026-09-19T01:30:07Z 索引；9 个平台 VSIX 全部 publish 成功）。
   **发布产物实测**（下载 `sokonanoda-cli-aarch64-apple-darwin.tar.gz`）：`shasum -c` **OK**
   → `--version` = `sokonanoda 0.59.0` →
   **G-01**：`theorem t9 : 3 := sorry` ⇒ `diagnostic` `kernel-expected-sort` + exit 1
   （签名受检真的在包里）；**G-04**：`infix:50 " ∈ "` 定义后 `x ∈ A` 判卷通过；
   **G-15**：`query check --compact` 的 `failed[0]` 带 `start_line/start_col/end_line/end_col`
   （1:14→1:15）且 `start/end` 仍是字节 offset；**L-01/L-02**：无 `import` 直接用
   `And.intro` / `Or.elim` / `Not.intro` / `Iff.refl` / `Iff.symm` / `Iff.trans` / `absurd` /
   `Eq.symm` 全部 `decl.checked`。`e2e-ledger` 自动把三条腿（Linux×2 + Darwin×1，各 **14/14**、
   `dirty=false`、server `0.59.0 == 扩展 v0.59.0 (bundled)`）回提交进 `docs/e2e/ledger.jsonl`；
   官网实测：进度页第一百〇六轮、`data/site.json` = `version 0.59.0` + `set_theory` 36 目标 ·
   329 checked · 99 open · 0 判负。性能基线见 `docs/perf/ledger.jsonl`（下一条）。
12. **推 main 后第一次 CI 红，已修**（run 35411049219）：`test` job 红在 LSP 项目哨兵
   `perf_project_did_open_and_keystroke`（CI 实测按键 480ms > 300ms 预算；同机单跑 17ms、
   满负载并行 86ms，且 pre-batch 与当前二进制同夹具对拍 best 26ms vs 25ms ⇒ **无产品回归**）。
   这是 2026-09-18「串行 + best-of-N」口径的**漏网用例**（当时只改了单文件延迟，项目级漏了）：
   修法是按键延迟改来回编辑 best-of-3 + 三个 project 用例加 `PROJECT_PERF_LOCK` 互相串行，
   **阈值不动**；修后满负载并行连跑 3 次 = 17/20/19ms。`auto-tag` 被这次红正确挡住
   （0.59.0 没有带着假红发出去）。台账与预防：`docs/CI-FAILURES.md`（2026-09-19 条）+
   `docs/PERF.md` §采样口径（纪律升级为"所有性能哨兵默认串行 + best-of-N"）；
   **0.59.0 性能基线已留档**：`docs/perf/ledger.jsonl`（front `keystroke_recompile_closure`
   best 35.26ms vs 0.58.0 的 33.88ms、lsp 项目按键 14ms 与 0.58.0 一致 ⇒ 五刀无开销回归）。
13. **未做 / 下一轮**：记法第二刀（`𝒫`/`ᶜ`/`''`/`⁻¹'`/`×ˢ`、跨 `import` 的记法、binder
   记法、重载）；**L-03**（`Eq.subst` 的 Type 层重写）；L-06（无累积性 + `Exists.elim`
   只能 Prop）；G-05/G-07/G-08 等 `painful` 项；课程侧小清扫（单元文件头里 9 处
   「逻辑（lib.Logic）」的来源标注改成「prelude 提供」）；发布本身全自动
   （push main → `ci.yml` auto-tag → `release.yml` 26 资产 + VSIX ×9 + SLSA provenance）。


## 本轮进度（2026-09-19，第一百〇五轮（语言线）：WO-011 / G-04 第一刀 —— 用户自定义记法 `∈`/`⊆`/`∅`）

> 台账 blocker：没有 `notation`/`infix`，集合论课程只能写前缀形式
> `Set.mem α a A` / `Set.subset α A B`，与纸笔数学（`a ∈ A`、`A ⊆ B`）迁移成本极高。
> 设计 = `docs/design/notation-subset.md`（N1–N7 + 优先级梯子 + 已知差异表 +
> 第二刀清单）。**本轮只做 Lean core 级第一刀**；`𝒫`/`''`/`⁻¹'`/`×ˢ` 留给第二刀。

1. **词法（三个 as-built 事实都实测确认）**：① 没有字符串 token ⇒ 新增
   `TokenKind::Str`（只服务记法声明里的符号文本；未闭合报 `unterminated-string`，
   span 在**开引号**）；② 数学符号**本来是标识符字符** ⇒ 新增 `TokenKind::Sym`
   （码点类 `U+2200–22FF` / `U+2A00–2AFF` / `\`，最大咬合），`is_ident_start`
   相应收窄；`∀` 的 `Forall` 分支**排在符号分支之前**，所以 `∀` 行为逐字节不变。
2. **parse**：四条命令 `infix:N` / `infixl:N` / `infixr:N` / `notation`；
   优先级插在 `parse_arrow`（最松）与 `parse_app`（最紧）之间的新梯子上
   （`parse_plus` 改为委托 `parse_operators(0)`，`+` 仍是 65 号内建、行为逐字节
   不变）。结合规则写死：`infix` = `p`/`p+1`、`infixl` = `p`/`p+1`（同级左结合）、
   `infixr` = `p+1`/`p`；**非结合同级链报错**。符号文本去空白后不能全是标识符
   字符（`notation-shape`）、未声明符号 `notation-unknown-symbol`、重复声明同一
   符号报错。**文件内作用域**：声明之后生效（不跨 `import`——第二刀）。
3. **elab（第三件 as-built：没人补前导 `Type` 参数）**：记法在 elab 内**源到源**
   重写成 `App` 形状，并**自己补前导类型参数**——只做**裸变量匹配**（不引入元变量
   /一般合一）：先从操作数类型解、再从期望类型解；解不出报
   `elab-notation-argument-unsolved`（`#check ∅` 就是这一条）。这一步是嵌套零元记法
   （`∅ ⊆ A`）能工作的关键：`∅` 的类型从外层记法给出的期望类型 `Set α` 反解。
4. **兼容护城河（实测）**：点名形式永久可用，且**省 `α` 的点名写法
   `Set.mem a A` 今天被内核拒绝、改后仍被拒绝**（同码 `kernel-rejected` 同 stage
   `kernel`）——两种写法判卷一致，不是"记法替代点名"。
5. **不新增语义面**：记法**不是声明**——不发任何事件、不进声明表、不进 goal 视图；
   `SemanticKind::ALL` 与 `tm_scope` 表**逐字节不变**（声明过的符号在语义层分类为
   `Keyword`，**未声明的符号不产生 run**，所以目标文本里的 `⊢` 仍是普通 run）。
6. **课程零改动（硬要求）**：`courses/set-theory/` **一个字节未动**，画布仍写点名
   形式；由 `the_shipped_course_still_uses_the_pointful_spelling` 钉住。
   课程门禁复跑 **315 checked · 96 open · 0 判负**，与记法落地前逐字相同。
7. **三层测试（TDD：先红后绿）**：front 词法 7 条 + parser 14 条 + elab 8 条 +
   semantic 3 条；CLI e2e `crates/cli/tests/notation.rs` **9 条**（五元计数一致 /
   无新事件种类 / 护城河 / 未声明符号的教学 hint / 复现件形状 / `#check ∅` 的
   unsolved 码 / stdin↔文件一致 / **真课程单元②的记法变体判卷一致** / 课程仍写点名
   形式）+ `protocol.rs` 一条（3 个 parse 码 + 2 个 elab 码的 stage）。
   第三层用**临时副本**（`/tmp` 复制 lib + 清单 + 改写签名的单元），课程树不动。
8. **复现件翻成"已修后形状"**：`docs/gaps/repro/G04-notation.sokonanoda` 补了使用行
   `def use (α : Type) (a : α) (A : α -> Prop) : Prop := a ∈ A`（`gap.py` 判据 =
   干净判卷 + 有 `decl.checked`）；`gap.py check` 里 G-04 已翻成「已判卷通过」。
   台账 G-04 行按 WO 要求重写 `today`/`expected_lean`/`blocks`（`blocks` **没有**
   清空：单元 6 的 `''`/`⁻¹'` 仍等第二刀）。
9. **验收**：`cargo build -p sokonanoda-cli -p sokonanoda-front -p sokonanoda-lsp` 过；
   `cargo test -p sokonanoda-front`（554 lib）· `-p sokonanoda-cli`（全套 20 个测试
   二进制）· `-p sokonanoda-lsp` 全绿；课程门禁绿；`gap.py check` 里 G-04 一致。
   **未跑 `scripts/soko gate`、未 bump 版本、未 commit**（仓库约定：主线统一）。
10. **文档同轮**：`docs/architecture.md` §4.1（新 token / 新命令 / 新 AST + 记法
    一节 N1–N7 摘要）、§5 白名单边界两条（记法是糖、是唯一例外面）；
    `docs/TESTING.md` 新增记法守护行；`docs/protocol.md` 5 个新码；
    `skills/sokonanoda-teacher`（语言能力速查 + 记法七条要点）·
    `sokonanoda-dev`（设计清单 + 白名单例外面）；`AGENTS.md` 硬规则 3 补记法例外；
    `editor/vscode/`（`tmLanguage.json` 数学符号规则 + CHANGELOG + README）；
    `docs/gaps/ledger.jsonl` G-04 行；本文件。

---

## 本轮进度（2026-09-19，第一百〇七轮：0.60.0 —— 编辑器 build/rebuild + 五个缺口收口）

> 用户要求：**「vscode 还是没有 sokonanoda: build 或者 sokonanoda: rebuild 的命令。你这个剩下的没做的也要做。」**
> 本轮把编辑器缺的命令补上，并把台账里剩下的 **G-05 / G-07 / G-08 / L-03 / L-06 与记法第二刀**全部收口——
> 结果是台账 **24 条 = 22 条 `fixed` + 2 条 `workaround`，`open` 归零**。

1. **VS Code `build` / `rebuild`（用户直接报的缺口）**：CLI 一直有 `sokonanoda build [--clean] [<file>|<dir>]`
   （预热/清理共享编译缓存），扩展从没接出来。新增 `sokonanoda: build`（`alt+b`：编当前 `.sokonanoda`
   文件——CLI 顺着 `import` 编整个闭包——否则编第一个工作区文件夹）与 `sokonanoda: rebuild`
   （`alt+shift+b`：先 `build --clean` 清缓存再重编）；两者把 CLI 的 JSON Lines（`build.file` /
   `build.clean` / `build.summary`）灌进 **sokonanoda build** 输出面板，回一行
   `N 个文件 · 编译 X · 命中 Y · 失败 Z（ms）`（带「显示输出」按钮），跑完刷新练习/项目/课程三棵树；
   项目视图标题栏也有入口。**三层测试**：静态契约
   `crates/cli/tests/extension.rs::build_and_rebuild_commands_warm_the_compile_cache`、stub 宿主、
   真 VS Code 冒烟（e2e 14 → **15 条**）。版本随 feature bump **0.60.0**（`Cargo.toml` +
   `editor/vscode/package.json` + CHANGELOG + 课程 `requires` 0.59 → 0.60）。
2. **G-05 `namespace` / `open`**（设计 `docs/design/namespace-open.md`）：`namespace A … end A` 让声明名
   自动带前缀（`def mem` ⇒ 全局名 `A.mem`；带点则拼接），`open A` 让短名可用；引用解析收敛到
   `crates/front/src/compile/scope.rs` 的候选序列（命名空间链由内到外 → 精确名 → `open` 前缀按序），
   三条命令**零事件、不进声明表**，专用 parse 码 `parse-namespace-{mismatch,unclosed,shape}`。
   课程跟随：`courses/set-theory/lib/Set.sokonanoda` 的 22 条声明去掉源级 `Set.` 前缀
   （**全局名一个没变、外部引用零改动**，门禁计数逐字不变）。实测边界写进设计 §4/§6：
   未闭合 `namespace` 会打断判卷合成路径（新增 `parser::parse_fragment`）、`apply` 的
   `unify_spine` 是文本对齐（用 `judge::judge_render_type` 先过内核 pp，未用命名空间的文件零开销）。
3. **G-07 课程清单 v2**（设计 `docs/design/course-manifest-v2.md`）：`course.json` 升成
   `soko.course/2`（1 卷 / 4 章 / 12 单元，每章 `prereqs`/`tags`/`quota.exercises`），
   **v1 扁平数组继续被接受**（入门课就是活体回归）；CLI 的 `course.unit` 只加
   `volume`/`chapter`/`tags`、`course.summary` 只加 `volumes`/`chapters`（v1 事件一个键都不多）；
   课程门禁新增 **G6 = 清单自洽**（卷/章 id 唯一、unit 恰好一章、`prereqs` 不悬空；
   **配额差额只报告不判红**，维持"只判形状、不锁计数"）；扩展课程树按卷→章→单元分组
   （v1 平铺逐字保留）、站点按卷/章渲染、`docs/protocol.md` 契约 additive。
4. **L-03 Type 层重写**（设计 `docs/design/eq-type-level-rewriting.md`）：实测内核**给** Eq 形状的大消去，
   堵路的是 prelude 的 `Eq` 是公理（无派生 recursor）+ 两条语法边界 ⇒ L1 新增 **B8 族**
   `axiom Eq.rec {u, v}`（与 Lean core 逐字同形）+ `def Eq.mp`/`Eq.mpr`，`PRELUDE_NAMES` 42 → 45，
   族让位照旧。复现件 `docs/gaps/repro/L03-eq-type-level.sokonanoda` 干净判卷（5 checked，
   含 `Vec.cast` 用 `@Eq.rec.{1, 1}` 在 `Sort 1` motive 上搬运）。残留边界（`Eq.mp` 是 Type 0 实例，
   多态版要层级算术 `u+1`）写进设计。
5. **L-06 累积性边界**（设计 `docs/design/prop-cumulativity-boundary.md`）：累积性是**内核性质**，
   冻结内核下不改（硬规则 1）；本轮把 `def T : Type := True` 的裸类型不匹配翻译成专用码
   **`kernel-prop-not-cumulative`** + 人话 hint（"本语言没有累积性；官方 Lean 4 有"），
   课程三条绕法（`Set.Equiv … : Prop` 数据化、unit07 数据版满射、unit10 `Exists.choose`）写进设计。
   台账改判 `workaround`（诊断质量那一半算 fixed）。
6. **G-08 `abbrev`**（设计 `docs/design/abbrev.md`）：实测 `def` 与 Lean `abbrev` 在本语言**无可观察差异**
   （类型位透明、`#reduce` 展开、hover 保留别名名、项层 `rfl` 成立；唯一差异 reducibility hint 在
   只有一个透明度层级的本语言里不可观察）⇒ 实现成**与 `def` 同语义的关键字**（真 Lean 子集兼容），
   AST/elab/内核零改动，白名单（parser 命令表 + CLI help + `docs/architecture.md`）同步。
7. **记法第二刀**（`docs/design/notation-subset.md` §11 as-built）：`prefix:N` / `postfix:N` 两条命令；
   卷 I 五个符号 `𝒫` / `ᶜ` / `''` / `⁻¹'` / `×ˢ` 落 `lib/Set.sokonanoda`（记法版与点名版**判卷计数逐一相等**）；
   **跨 `import` 传播**（闭包加载器改成"先收 import 边、先访问依赖、再解析自己"，依赖导出的记法表当继承表，
   重声明继承符号是 parse 错误）；顺带修掉 4 个既有缺陷（记法前导参数下溢 panic、`by.rs` 的
   `atom_text` 括号清单漏记法、`judge_infer` 读目标签名剥错 binder、`by` 块目标里的记法判不动）。
   未做（设计 §12 逐条给理由）：binder 记法、记法重载、`scoped`、集合字面量、print-back。
8. **台账收口**：24 条 = **22 `fixed` + 2 `workaround`（L-04 / L-06）、`open` 0 条**；
   `python3 scripts/gap.py check` 全绿。**验收**：`scripts/soko gate` exit 0
   （fmt/clippy/`cargo test --workspace --locked` **1107 passed**、课程门禁 **36 目标 · 329 checked ·
   99 open · 0 判负**、台账门禁全绿）；版本 0.60.0（两处）+ 课程 `requires = "0.60"`。
9. **发布结果（2026-09-19 实测）**：push main（`a948f65`）→ CI **一次全绿（8m20s）** →
   auto-tag 打 **`v0.60.0`** → `release` **全 success** → GitHub Release **26 资产**
   （lsp ×8 / cli ×8 / vsix ×9 / `SHA256SUMS`）+ Marketplace 收录 **0.60.0**。
   **发布产物实测**（下载 `sokonanoda-cli-aarch64-apple-darwin.tar.gz`）：`shasum -c` **OK**
   → `--version` = `sokonanoda 0.60.0` → 一段同时用新语法的文件干净判卷（exit 0）：
   `namespace A` + `def f` ⇒ 全局名 **`A.f`**（G-05）、`abbrev T : Type := Prop -> Prop`（G-08）、
   `def uses : T := A.f` ⇒ 3 条 `decl.checked`。官网 `data/site.json` = `version 0.60.0` +
   `set_theory` 36 目标 · 329 checked · 99 open · 0 判负。

---

## 本轮进度（2026-09-19，第一百〇八轮：0.61.0 —— 设计文档里「未做」的全部收口）

> 用户要求：**「设计文档里的东西都做了吧。」** 本轮把 `docs/design/` 各篇「未做 / 第二刀 / 残留边界」
> 里**不违反硬规则**的项目全部实现；只有两条内核冻结项（累积性、Prop 大消去）与一条内核 pp 项
> （源码级 print-back）保留为**有理由的边界**。

1. **记法第三刀**（`docs/design/notation-subset.md` §12 → as-built + §13）：**binder 记法**
   （`binder_notation`，`∃ x, p` / `∀ x, p`；命令拼写为什么不是 `notation-binder`：`-` 不是本语言
   标识符字符，实测被切碎）、**记法重载**（同符号同形状按期望类型选候选；歧义/无候选两个专用码 +
   人话 hint；重声明 import 来的符号仍是错误）、**`scoped` / `open scoped`**、**集合字面量 `{a}`/`{a,b}`**
   （新语法，`{}` 与 binder 定界符消歧）、**一元记法在实参位免括号**。课程单元⑧ 加一条 `example` 演示
   （练习数量与题意不动）。print-back 保留为「内核 pp 冻结 ⇒ 销不掉」（§13.1）。
2. **`namespace`/`open` 扩展**（`docs/design/namespace-open.md` §7 → as-built）：**子句**
   （`only`/`hiding`/`renaming` 互斥、先过滤后改名）、**`open Foo in <cmd>`**（限叶子命令，
   合成前缀补源码原文 `open` 头，做过变异验证）、**`export`**（文件内同 open + 跨 `import` 重放导出表；
   `open` 不跨）、**`open scoped`** 接线、**遮蔽 warning**（非 error）。
   `section`/`variable` 评估为**做不动**并给实测（`#check id Nat` ⇒ `Nat -> Nat`、`def u : Nat := id Nat`
   内核拒绝 ⇒ 本语言无隐式参数插入，auto-bound 落不出 Lean 体验）；`namespace` 跨文件传播澄清为"无需做"。
3. **层级算术与 Eq 多态**（`docs/design/eq-type-level-rewriting.md` §4）：宇宙层级支持**数字后缀加法**
   （`u+1`；`u+v`/`max` 给专用诊断——内核 `Level::Max` 无公开构造入口），`Eq.mp`/`Eq.mpr` 从 Type 0
   实例升级成**宇宙多态**（签名对齐 Lean core），装上 **`cast`** 与 **`Eq.ndrec`**（真 Lean 子集兼容；
   `cast` 原来是 front 单测里的自定义公理名，已改名）。`PRELUDE_NAMES` 45 → 47，B8 族让位照旧；
   L-03 复现件扩到 **10 checked / 0 diagnostic**。
4. **编辑器词表与课程工具**：`front::semantic::KEYWORDS` 与 `editor/vscode` TM 语法**同轮**加入
   `abbrev`/`prefix`/`postfix`/`binder_notation`/`scoped`（守护
   `tm_grammar_keywords_follow_the_single_source` 绿）；CLI `course` 支持**多清单聚合**
   （`course <path>… [--all]`，多份才加 `course.unit.manifest` 与 `summary.manifests`，单清单一个键不多）；
   课程门禁新增 `--ledger`（**默认关**，避免 CI 写仓库）产出成本台账 `docs/courses/ledger.jsonl`
   （已真跑一条：36/329/99/0 · 20024ms · v0.60.0）。
5. **保留的边界（有实测理由，非"没时间"）**：累积性与 Prop 大消去（内核冻结，硬规则 1）、
   源码级 print-back（类型文本由内核 pp 产出，记法不进内核）、`section`/`variable`（无隐式参数插入）、
   `u+v`/`max`（内核无公开构造入口）。
6. **验收**：`scripts/soko gate` exit 0（`cargo test --workspace --locked` **1163 passed / 0 failed**；
   课程门禁 **36 目标 · 329 checked · 99 open · 0 判负**；缺口台账门禁全绿）；版本 0.61.0（两处）+
   课程 `requires = "0.61"`；内核零改动。
7. **发布结果（2026-09-19 实测）**：push main（`f3902b3`）→ CI **一次全绿（9m18s）** → auto-tag
   打 **`v0.61.0`** → `release` **全 success** → GitHub Release **26 资产** + Marketplace 收录
   **0.61.0**。**发布产物实测**（下载 `sokonanoda-cli-aarch64-apple-darwin.tar.gz`）：`shasum -c`
   **OK** → `--version` = `sokonanoda 0.61.0` → 一段用新语法的文件判卷：`namespace N` +
   `abbrev T : Type := Prop -> Prop` + `def f : T := …` ⇒ 全局名 **`N.f`** 与 `T`/`f` 全部
   `decl.checked`（G-05 + G-08 在发布产物里可用）；`cast` 已可解析（该 smoke 文件里两条诊断来自
   它自己的宇宙层级写法，不是缺名字）。
   注：本轮 bump 后第一次 gate 曾 exit 3——`target/debug` 还是 0.60.0 而版本钉已到 0.61.0，
   启动器按 G-16 的守卫**拒绝**了缓存里的 0.55.0（守卫工作正常）；`cargo build -p sokonanoda-cli`
   重建后 gate 复绿。

---

## 本轮进度（2026-09-20，第一百〇九轮：站点全面重构 —— 28 页 + 总验收 14 项）

> 用户要求（原话）：「项目更新了非常多的版本，我希望 site 静态网页全面重构一下，
> **不要参考旧版本，旧版本没有设计感，美感，很多 ai 味**。我希望除了产品介绍页、
> 项目文档，还可以设计一些功能页……」，中途追加「**先不做功能页，功能展示多一点也行，
> 毕竟只是在 github pages 上，快也很重要**」与「我发现 superpower 啥的 skill 都没装，
> 网页产品做得不够全面，可能是 skill 不够」。本轮的产出与依据全部在
> **`docs/design/site-rebuild/`**（`STATE.md` 是入口）。

1. **旧站体检并整体废弃视觉层**：17 个不同 `font-size`、6 种圆角、11 个非网格 gap、
   27 个硬编码色值对 9 个变量、0 个 `transition` / `prefers-*` / `:focus-visible`、
   正文行宽 ≈112 拉丁字符（WCAG 1.4.8 上限 80）、2 处对比度不达标、缺
   favicon / og / canonical / theme-color / 暗色模式。**文案是资产，保留；视觉层全废。**
2. **调研**：设计手艺（R1，含 Anthropic 官方 `frontend-design` 原文与 72 条 AI 味特征）、
   65 个站点拆解（R2，含 lean-lang.org 的技术性诊断与 Alectryon 纯 CSS 折叠机制）、
   字体管线（R4，字形覆盖逐条实测）。**关键发现**：`⊢`（U+22A2）缺失于 Menlo / SF Mono /
   Monaco / Consolas / Source Code Pro / JetBrains Mono / Inter / IBM Plex Sans **以及旧站自己的
   CJK 回退 Hiragino Sans GB** —— 逐字回退会静默破坏等宽对齐。
3. **方向「格纸 · 朱批」**（D1）：冷调纸面 + 24px 方格（只铺在推导发生处）+ 零圆角结构容器
   + 发丝线；两个**语义可审计**的强调色——绿只给「内核真的通过过」的东西，朱只给
   「诊断 / 更正 / 诚实的限制」。**衬线默认撤掉**（`--font-display` = `--font-ui`）：
   用 `ai-smell-detector` 审自己的方案，「高对比衬线大标题」正是第 1 类 AI 长相的核心配料，
   教科书感改由**构件**承担（编号小节 / 页边注 / 定理证明块 / 推理横线）。
4. **28 页**：产品 4 / 功能 9 / 学习 4 / 使用 4 / 过程与信任 5 / 英文 landing 1 / 样板页 1。
   新增搜索、对照页、术语表、常见问题、版本历史、404、`sitemap.xml`/`robots.txt`/`llms.txt`。
5. **功能页靠真实预计算数据 + 纯 CSS 折叠**（不做 WASM / 不做后端）：
   `gen-site-lab.py` 产 6 份数据（走查逐行目标状态 / 真实事件流 / 诊断字典 64 码 /
   时间线 / 缺口台账 / 质量证据），**3 次跑字节一致**，诊断复现 **55/64**。
   走查页 23 个折叠块的数据经**逐字节保真核对**（0 mismatch），且**关掉 JS 仍可用**（实测）。
6. **零伪造是硬要求**：站点上所有内核输出要么逐字来自 `site/data/*.json`，要么就是编的。
   验收把它做成机器判据（K10），当前 **91 条录制值全部回查通过**。
7. **总验收一条命令**：`python3 scripts/site-verify.py` —— 完整性 3 项 + 正确性 11 项，
   **当前 14/14 全绿，exit 0**。含 K12「课程计数可复现」：判据是
   **HEAD 的课程 + 已发布二进制**能不能跑出页面写的数（现场跑会因并行开发误报）。
8. **退役**：`scripts/gen-site-demos.py` 与 `site/assets/demos/`（PIL 画的假 VS Code 截图）
   整体删除，编辑器面板改成真 HTML/CSS；旧 `site/assets/style.css` 删除；
   `site/assets/agent-prompt.js` 删除（安装 prompt 改为 `agents.html` 单一天然 HTML 源）。
9. **agent 技能**：`scripts/fetch-agent-skills.py` + `dsh/agent-skills.json` ——
   26 个第三方技能（MIT，装进 `.dsh/skills/`，载荷不入库）。**Anthropic 官方那包未装**
   （无 LICENSE，只有 "demonstration and educational purposes"）。
10. **本轮的实测修正**（全部记在 `STATE.md` §5，共 17 条）：`scripts/soko` 文档承诺的
    「扩展自带回退」是死代码；两个 `doctor` 结论相反；`docs/protocol.md` 两处与实测不符
    （`error[<stage>]` 只有 parse 是、`expected`/`actual` 字段根本不存在）；
    print-back 限制是**位置性**的而非全局；`query check` 对「用了 import 记法」的文件
    **报假红**（`grade` 同文件 exit 0）；**`by` 白名单是七不是十三**
    （工作树有 +1548 行未提交 WIP，`scripts/soko` 量到的是未发布代码）。

## 本轮进度（2026-09-21，第一百一十轮：R2 修边刀 —— 判卷器**五条**静默错；**卷 I 全绿（36/36 · 0 判负）**）

> 用户要求（总账，原话）：「把 courses/（卷 I set-theory）+ course/（入门课）+
> playground.sokonanoda 改写成 Lean 4 风格：连接符用数学符号、证明全用 tactic、
> 练习占位写 `:= by sorry`」。本轮是 **R2（引擎扩展 + 卷 I 全量）的收尾**：
> 全量改写跑完后 **36 个课程目标只剩 `unit12-solution` 一个红**，而它那 6 条判负的
> 声明「怎么看都对」。逐条缩到最小复现（一次 ~10s）后查明：**6 个「写作错误」里
> 5 个是判卷器的静默错**，另 1 个是 `cases` 的实测边界。**内核零改动**
> （`git diff --stat -- crates/kernel/` 空）；as-built 见
> `docs/design/course-lean-style.md` §9「R2 修边刀」。

1. **①`apply` 的类型参数静默填错**（最严重的一条：**悄悄生成错子目标**）。
   `Set.ext` 的 pp 签名把 `Eq` 的类型实参丢了（`Eq A B`）⇒ 类型参数 `α` 不在任何
   实参位上，旧代码按名字回退到「同名上下文变量」——而 `Set.ext` 的参数恰好就叫
   `α`：元素类型叫 `β`、上下文里另有一个 `α` 时子目标成 `y : α`，之后每条 `exact`
   都报「期望 `C y`，实际是 `C y`」（**字面相同**）；上下文里没有 `α` 时报
   `unknown identifier α`。修法 `solve_type_params`：拿「已填层的 domain ↔ 该层值的
   类型」反推（`A : Set α` 已填成 `f '' A`，后者类型 `Set β` ⇒ `α := β`）。
2. **②`cases` 在 `have … := by` 里丢字段**：嵌套 `by` 的值要**打回源码文本**再判卷，
   而 `render_pattern` 无条件给子模式加括号（`| intro b hb =>` → `| intro (b hb) =>`），
   回读时构造子只剩 1 个子模式。只给本身带子模式的子模式加括号。
3. **③`cases` 臂里裸 `Eq` 丢宇宙层级**（四条配套一起上）：pp 把 `Eq.{1} β (f a) b`
   打成裸 `Eq (f a) b`，而这条类型**不只判定要用**——`match` 的组装与最终声明判定都
   从节点上读它 ⇒ 整条声明被内核拒（「期望 `Sort(0)`，实际是 `$N`」，位置在定理那一行）。
   配套：Eq 三件套是受信任 **axiom**（没有定义体，从不进 `DefTable`）⇒ 补
   `trusted_prelude_arity` 表；还原时补回 pp 丢掉的前导类型实参；进 `fun`/`forall`
   的体先把该层 binder 加进判定上下文；`restore_universe_levels` 接到
   `canonicalize_binder_type` 上（上一轮试过并回滚的那一刀，这次连同三条配套落地）。
4. **④`cases` 字段类型留 beta redex**：`Exists` 的 `h : p w` 代成 `(fun …) b`，
   而内核不做 beta 转换 ⇒ 与 `apply` 同一条纪律，`beta_normalize` 补上。
5. **⑤`cases` 的参数代换取源记法形态 ⇒ 前导类型参数撞名静默错**（与 ① 同一类）：
   源类型里的记法节点**不带前导类型参数**（`elab` 期才算得出来），`unfold_one`
   只能右对齐 ⇒ 定义体里提到前导参数的地方**留着定义自己的 binder 名**，在调用点
   按「同名上下文变量」解析（`Set.image` 的形参就叫 `α`/`β`）——臂里的假设类型成了
   `Eq.{1} β …`（该是 `γ`），之后每条 `exact` 都把同一条命题写成两种形态。
   修法：`cases` 实参**优先取规范形态**（内核 pp：点名 + 全实参），源形态只兜底。
6. **判卷成本的真凶（上一轮遗留，已修）**：`JUDGE_CACHE_CAP` 128 → 4096。判定要
   **递归**重编译前缀（前缀里就有更早的 `by`），FIFO 128 一被挤爆就再也接不住 ⇒
   成本随声明数**指数**增长。实测 `unit04-solution`：6 条声明 8.9s、第 7 条 >60s、
   整份 >600s 不返回；修后 **7.88s / 8 条全 checked**。
7. **unit12-solution 剩的 6 条是 `cases` 的实测边界**（不是引擎缺口），课程侧改法：
   `cases hC y hy`（被消去项是应用）→ `Exists.elim` 项；`cases` 消去 **`have` 绑定的
   `∈ 像` 假设** → 先 `have hex : Exists … := hmem` 再 `cases hex`；嵌套 `cases` 的内层
   → `Exists.elim`；`cases` 消去**书写成 binder 记法 `∃`** 的假设 → 先
   `have e1 : Exists … := e` 再 `cases e1`。
8. **验证（本轮实测）**：`scripts/soko gate` **全绿**（fmt + clippy + test + playground
   锚点 + 课程门禁 + `gap.py check`）；`cargo test --workspace --locked` 全绿
   （`notation` **41**、`sokonanoda-front --lib` **661**）；课程门禁 **36 目标 / 0 判负 /
   checked 328 / open 99 / exit 0** —— **卷 I 首次全绿**。五条修复都有**旧/新二进制
   对照**回归测试（旧红新绿）。
9. **仍欠**：R2.5（隐参数路线 C / 记法可输入性收尾）与 R3（入门课 52 文件 +
   `playground` + 13 处计数钉住的测试）。**卷 I 这一站已完成。**


## 本轮进度（2026-09-21，第一百一十二轮：R3 入门课开工 —— 单元① 样板 + 骨架记法同步 + 施工手册）

> 用户要求（总账）：入门课 `course/`（52 文件 / 4337 行）也改写成 Lean 4 风格。
> 设计依据 `docs/design/course-lean-style.md` §C2（C2.1–C2.11）；本轮是 R3 的
> **第一刀**：把**样板**做出来、把**施工手册**写出来，并把会连带翻车的地方先钉住。

1. **单元① 样板（CN/EN 画布 + CN/EN 解答，4 文件）**：代码全部换记法
   （`And a b` → `a ∧ b`、`Or a b` → `a ∨ b`、`Not a` → `¬ a`、`->` → `→`），
   解答的证明体全部改成 `by` tactic 块（`intro` / `exact`），**画布的演示保持
   项模式**（设计 C2.2：单元①②③⑤–⑪ 的教学点就是项模式，§C2.6 只把单元④
   改成 `by` 单元）。**引理名照旧点名**（`And.intro`/`And.left`/`False.rec`）。
2. **关键实测：纯记法改写是 count-neutral 的** —— 单元① 画布改前改后都是
   `(checked, open, reduced) = (13, 6, 1)`，CN/EN 与画布/解答四份计数逐项一致
   ⇒ **正常改写不该动 `GOLDEN`**；动了就说明改了语义（这条写进手册的计数纪律）。
3. **骨架块的记法连带面**：`course/shared/{And,Or}.sokonanoda` 是「规范副本」，
   `course_shared.rs` 逐字守着 **24 份 And / 12 份 Or** —— 改单元① 的 And 公理块
   当场把它打红（这正是它存在的意义）。同轮把规范文本与全部副本（And 21 处 +
   Or 12 处，含**未登记的** `unit11-project/Logic.sokonanoda`）一起换记法；
   `inductive Or` 的构造子结果类型里 `∨` 也解析得了（在其自身声明块内）✓。
4. **施工手册** `docs/notes/course-lean-style/R3-rewrite-brief.md`（新，+ `docs/README.md`
   索引）：CN/EN 同步规则（代码逐字相同、注释分别润色）、本课可用记法集与优先级、
   **本课 tactic 白名单**（`intro`/`exact`/`apply`/`assumption`/`rfl`/`have`——
   ⚠️ **不含** `constructor`/`cases`/`left`/`right`/`use`/`exfalso`：本课 `And`/`Or`
   是自建骨架，`constructor` 要真归纳）、**计数纪律**（从内核取数、不许手算、
   不许为凑数删练习）、单元④ 的特例（缺 5 号、9 条遗留声明）、阻塞上报格式。
5. **本轮明确不做**：设计 §C2.5 的**删骨架**（把自建 `And`/`Or` 块删掉让 prelude 的
   真归纳回来）——它是**需拍板**项，且会打红 13 个计数测试；本轮把它留在原地，
   等样板铺开后再单独一刀（手册 §1 已写明）。
6. **验证（本轮实测）**：`cargo test --workspace --locked` **1210 passed / 0 failed**
   （含 `course.rs` 的 GOLDEN 11 条、`course_status.rs`、`cli.rs` 的 86/65、
   `course_shared.rs` 的 4 条副本/镜像检查）；单元① 四份 `grade` 退出码 0、
   CN/EN 计数逐项相同。
7. **仍欠**：R3 主体（单元②–⑪ 的记法改写 + 解答 tactic 化，按手册派 subagent，
   一次 2–3 个单元对）、单元④ 的特例刀（C2.6）、C2.5 的骨架删除（要拍板）、
   C2.9/C2.10 的约 30 处叙事 + `course/README.md` 的 24/12/8 与行数、
   C2.11 的课外语料 6 处；之后是 R2.5 的 IA-2/IA-3。

## 本轮进度（2026-09-21，第一百一十一轮：R2.5 IA-1 —— **隐式实参路线 C 落地**，课程零改动全绿）

> 用户要求（总账）：「如果想支持 notation，感觉 `{x : Set}` 这种隐参数自动推导的机制
> 不得不实现了。」设计与分期在 **`docs/design/implicit-arguments.md`**（as-built 见其 §9）；
> 本轮的期号是 **IA-1**：路线 C（**风格对齐 + 唯一确定**，纯前端、不引入元变量、
> **内核零改动**，`git diff --stat -- crates/kernel/` 空）。

1. **签名表（零内核调用）**：`KnownName::Decl` 加 `implicit_prefix`（声明望远镜的
   **前导隐式 binder 个数**，纯源级 AST 走查）；`walk.rs` 的 6 个登记点算出来，
   prelude 固定 `0`（§7 第 4 条）。它是插入路径的**免费闸门**——为 0 时一行不跑，
   这既是安全性质也是成本控制（先试过无条件 `judge_infer`，判定**递归**重编译前缀
   ⇒ 栈溢出）。
2. **路线 C**：新 `compile/implicit.rs`——带**风格**的望远镜解析、前导隐式计数、
   `solve_prefix`（按层序找"后面第一个提到它的层"，用那一层实参的**类型**头部匹配；
   顺序代入；解不出返回 `None`）。`Expr::App` 臂是**唯一钩子**。
3. **`@` 给了真语义**：`@f a b` 关闭隐式插入（Lean 语义，护城河的逃生门）。
   新 `Expr::App::explicit_spine` + parser 的 `saw_at` + 渲染只在头前打一个 `@`。
4. **唯一必须同轮改的既有行为**：`render_expr` 不再给 `UniverseApp` 凭空补 `@`
   （`@` 从"提示"变成"真语义"后，判卷通道"渲染→回读"会**改变含义**）；
   症状很绕——记法目标 `Aᶜ ∪ B` 的前导参数解不出，实为合成声明里的 `@Eq.{1}`
   被解析成显式标记。三条 round-trip 用例同轮重钉。
5. **§0 的"前提"被前提测试当场推翻**：设计说"全仓只有一处、且在注释里"，
   实测 `course/` 里有**两处刻意**的隐式签名（`Eq.symm`、`eq_refl_prop`——
   **这两道题教的就是隐式 binder**），它们只用 prelude 的 `Eq.subst`/`Eq.refl`
   ⇒ IA-1 不影响它们（1210 条测试全绿）。前提测试改成"除这两处教学例外外"。
6. **新错误码** `elab-implicit-argument-unsolved` 进 `docs/protocol.md` 码表；hint
   教"把参数写全"（点名写法永远可用）。
7. **验证（本轮实测）**：`cargo test --workspace --locked` **1210 passed / 0 failed**；
   课程门禁 **36 目标 / 0 判负 / 328 checked / 99 open** —— 与 IA-1 之前**逐项相同**
   （安全性质：课程签名没有隐式 binder ⇒ 插入路径一行不跑）；`notation.rs` 从 41 涨到
   **45** 条（前提 + 插入 + `@` + 专用错误码），`implicit.rs` 另有 5 条纯单测。
8. **仍欠**：**IA-2**（`lib/` ≈39 条签名改隐式 + units 2768 处缩短 + §6 测试重钉 +
   §3.4 护城河话术与 `notation.rs:163` 夹具）与 **IA-3**（收窄/删除记法补参 hack）；
   之后是 R3（入门课 52 文件 + `playground` + 13 处 GOLDEN）。

## 本轮进度（2026-09-21，第一百一十三轮：R3 主体落地 —— 入门课 44 文件记法 + tactic；**语言补两刀**）

> 用户要求（总账 §9 2026-09-19 条目）：入门课 `course/` 全量改写成 Lean 4 风格。
> 本轮把 R3 的**主体**做完（第 112 轮只做了单元① 样板 + 手册），并在改写过程中
> **反过来改了语言两处** —— 正是用户预判的「改写会对 sokonanoda-lang 功能本身
> 有需求」。设计依据 `docs/design/course-lean-style.md` §C2/§C4，as-built 见同文件
> §9「R3」与 `docs/design/by-tactics.md` §12。

1. **改写完成面**：**44 个教学文件**（11 单元 × 中英 × 画布/解答）+ `course/unit11-project/`
   4 个文件（**之前整目录漏改**，它不在 `course.json` 里但却是 `import` 教学与
   `scripts/soko grade` 的活样例）+ 规范副本 `course/shared/Nat.sokonanoda`。
   代码连接符 `And a b`→`a ∧ b`、`Or`→`∨`、`Not`→`¬`、`->`→`→`、`forall`→`∀`、
   `Exists`→`∃`（单元⑧ 自加 `binder_notation "∃" => Exists`，**不产生事件**）；
   **解答的证明体全部改 `by` tactic 块**（`def` 是函数定义、不是证明 ⇒ 单元③⑥⑦
   解答 0 个 `by`，这是对的）。**引理/构造子名照旧点名**（`And.intro`/`Or.inl`/
   `Exists.elim`）——记法是连接符的糖。
2. **CN/EN 逐字节一致**：每对文件用 `course_shared.rs::code_only` 的口径复核代码
   完全相同（只差 `--` 注释）；四份 `grade` 退出码 0、诊断 0、计数逐项相等。
3. **单元④ C2.6 结构专项（四条全做）**：补练习 `by_ex5`（教 `have`）+ 演示
   `demo_by_have`、删解答里 **9 条早期草稿遗留**（`h_s`/`Pfam`/`Qfam`/`f_dep`/
   `qfam_true`/`val_apply_imp`/`val_apply_dep`/`by_ex7`/`by_ex8`）、「首期五个 tactic」
   措辞改成真实白名单（六条 + 后续单元解锁表）。画布 `(13,5,0)` → **`(14,6,0)`**，
   课程总计 **checked 86→87、open 65→66**，四处钉子同步（`course.rs` 的 `GOLDEN`、
   `course_status.rs` 的逐单元表 + summary、`cli.rs` 的 warm-cache 总计）。
4. **语言刀 A：判定合成声明携带声明的宇宙参数**。症状：目标里出现 `Sort u`/`Eq.{u}`
   时 `:= by …` 报 ``universe variable `u` is not declared in this declaration`` ——
   **宇宙多态定理根本写不了 tactic**（卷 I 的 `Set.{u}` 遍地都是）。根因：by 引擎的
   `spec_of`/`spec_of_for_judge` 硬写 `OpenGoalSpec.universe = Vec::new()`。修法：
   `walk.rs` 的 `def`/`theorem`（`example` 传 `&[]`）→ `lower_value`/`lower_by_val`
   → `run_by` → `run_tactics` → `{apply,cases,ctor,exact}_tactic` → `{judge,
   judge_with_levels,spec_of,spec_of_for_judge}` 一路带上。**`universe` 为空时
   `OpenGoalSpec` 逐字段与改前相同** ⇒ 既有行为零变化。测试
   `by_block_carries_the_declaration_universe_parameters`；活样例 = 单元⑤ 解答的
   `theorem Eq.symm {u} : {α : Sort u} → … := by …`（本轮顺手把它的项模式证明改成
   tactic 块，它是课程里唯一的宇宙多态定理）。
5. **语言刀 B：`src_spine` 认记法节点**。症状：`have bc : B ∨ C := …` 之后
   `cases bc` 报「被匹配项必须是一个书写类型为 `Or …` 的局部变量」——逼学习者把
   **块内假设**写成点名形式（声明参数/`intro` 进来的有「归一成内核 pp」兜底，
   `have` 引入的没有）。修法：`elab.rs::src_spine` 增 `Expr::Notation` 分支——
   记法的**源像**就是 `target` 那条 spine，操作数按源序收集（中缀 `[lhs,rhs]`、
   前缀 `[rhs]`、后缀 `[lhs]`、零元 `[]`）。测试
   `cases_splits_a_have_bound_hypothesis_written_with_notation`。
6. **顺手补的守卫空洞**：`course_shared.rs` 的语料之前**不含 `unit11-project/`** ⇒
   它的 `Logic.sokonanoda` 抄了 And 公理块却不登记也全绿。本轮把该目录纳入扫描面
   并把该文件登记进 `AND_COPIES`（**And 24→25 份**），同时给「画布不得 import」
   那条测试加上 `unit11-project/` 例外（它**就是** import 教学样例）。
7. **`playground.sokonanoda`（用户直接面对的教师画布）**：`∃` 段补
   `binder_notation "∃" => Exists` + 目标/假设改写，叙述从「本画布没有声明这个记法，
   所以一律写点名形式」改成「已声明，所以写 `∃ (x : Person), P x`；两条规则本身照旧
   点名」。**计数与提交版逐项相同**（`decl.checked` 30 / `example.checked` 2 /
   `exercise.open` 4；warning 2→1 是前几轮修的）。
8. **文档同轮**：`docs/design/course-lean-style.md` §9 新增「R3」as-built（含两处语言
   刀的表 + 已知边界）；`docs/design/by-tactics.md` §12 新增宇宙参数 as-built（含
   **未修项**：`judge_infer` 仍不带宇宙参数 ⇒ `apply`/`cases` 的推断侧有同类边界）；
   `REQUIREMENTS.md` §9 的 2026-09-19 条目补「进展」；`docs/design/course-syllabus.md`
   单元④ 行、`skills/sokonanoda-teacher/SKILL.md`（新增「写 Lean 风格记法」与
   **全量 tactic 白名单 + 按单元解锁**两条）、`crates/front/src/lib.rs` crate 文档、
   `course/README.md` 的写死计数（24/12/8 → 25/12/8；`232/2974` → 实测
   44 文件 4408 行、288 行在重复块里 ≈6%，并注明重量办法）。
9. **验证（本轮实测）**：`cargo test --workspace --locked` 全绿；`scripts/soko gate`
   全绿（fmt + clippy + test + playground 锚点 + 课程门禁 + `gap.py check`）；
   课程门禁 **36 目标 / 0 判负 / exit 0**；`git diff --stat -- crates/kernel/` **空**。
10. **仍欠**：R3 收尾的 C2.9 叙事陈词（少数几处；「Or 从公理升级」这类只有在 C2.5
    拍板后才需要改）、**C2.5 待拍板**（删自建 `And`/`Or` 骨架换 prelude 真归纳 ⇒
    解锁 `constructor`/`cases`，代价是 13 处计数测试重钉）、`judge_infer` 的宇宙参数
    （下一刀，见 by-tactics §12）、R2.5 的 IA-2/IA-3。

## 本轮进度（2026-09-21，第一百一十四轮：**纠正记账**——卷 I 全量改写其实没做完；R2 主体收尾开工）

> 上一轮把 R3（入门课）做完后，用 R3 学到的「**残留扫描**」去复查卷 I，发现
> 第 110–112 轮把「**卷 I 全绿**（36 目标 / 328 checked / 99 open / 0 判负）」
> 记成了「卷 I **全量改写完成**」——**两者不是一回事**：课程门禁只证明每个目标
> **判卷通过**，不证明文本**改写过**。实测卷 I 仍有 **829 处**旧写法
> （`->`/`forall`/`And X Y`/`Or X Y`/`Not X`/`Iff X Y`/`Exists X (fun …)`）
> 与 **50 个值位不是 `by` 的声明**。R2 真正交付过的只有**语言地基 + 单元② 一个
> 试点**（有 `the_shipped_course_uses_the_library_notation` 守着它）。

1. **`lib/Set.sokonanoda` 改完**（卷 I 标准库的样板）：语句与 `def` 体用记法
   （`→ ∀ ∧ ∨ ¬ =`；`∈ ⊆ ∪ ∩ \ ∅ 𝒫` 是本文件自己声明的，定义体里仍点名——那是
   正确的自指边界），10 条定义展开引理全部改 `by` 块。**23 `decl.checked`**。
   顺带实测：`·` 聚焦**没进语法**（parse 错），设计 N-12 的 L3.9 未实现。
2. **语言刀 C：`cases` 的头解析认记法**（第三次"改写反过来改语言"）。把
   `def union` 体从 `Or (A x) (B x)` 改成 `A x ∨ B x`（纯可读性、语义等价）之后，
   卷 I 门禁**当场 328/0 → 326/2**：两条 `cases h`（`h : x ∈ A ∪ B`）报「被消去项
   不是归纳类型的值」。根因：`cases` 降低成 `match` 前要 `unfold_to_inductive` 把
   `def` 的**体**代进来，而头解析用的是只走 `Expr::App` 的 `spine_of` ⇒ 记法节点
   没有 `Ident` 头。修法：改用 `spine_with_notation`（记法节点的头就是 `target`；
   源实参那条路本来就在用它）。**这是卷 I 改写的前置条件**——库一改用记法，
   所有"透过 def 看归纳"的 `cases` 都会撞上。回归测试
   `notation.rs::cases_sees_through_a_definition_body_written_with_notation`；
   旧/新对照就是那次真实门禁（326/2 → 328/0）。
3. **机械记法替换 517 处**（脚本 + 每轮门禁验收）：`lib/` 67、`units/` 画布 74、
   `units/solutions/` 376。**每一轮替换后门禁都仍是 328 / 99 / 0**。
3b. **语言刀 D：binder 记法的实参位两处必须同款**（本轮第二个真 bug，被既有测试
   抓住）。`∃ (x : α), p x` 的应用形态是 `Exists α (fun …)`——域类型要占位。
   `spine::spine_with_notation` 一直有这条 binder 分支，而 `elab.rs::src_spine`
   （上一轮为「`have h : B ∨ C` 之后 `cases h`」新加的记法分支）**漏了它**。
   触发很隐蔽：**只有把 `∃` 写进 `def` 体**才会撞上（`lib/Image` 的 `Set.image`
   改成 `∃ (x : α), x ∈ A ∧ f x = y` 之后，单元⑧/⑫ 的 `cases hy` 整类打红）。
   抓住它的是**既有**两条测试（`cases_inside_a_have_block_keeps_every_constructor_field`、
   `cases_uses_the_canonical_type_so_notation_prefix_params_do_not_capture_context_names`）
   ⇒ 说明「语言刀 C 只修了一半」。as-built 见 `docs/design/by-tactics.md` §12 第三处。
3c. **一次「伪阻塞」的教训**：subagent 基于**我覆盖二进制那一瞬间**的坏环境
   （`target/release/sokonanoda` 被 SIGKILL ⇒ `scripts/soko` 回落到陈旧缓存 0.55.0
   并 exit 3）报了一条交叉阻塞，结论是「`∃` 不能用在 def 体里」。**实际用修好的
   二进制复跑它的最小复现是 `decl.checked`**。两条纪律：① 覆盖运行中二进制要
   原子（先写临时名再 `mv`），否则并行的判卷会读到半截文件；② 转述别人的失败
   报告前**自己复跑一次最小复现**——环境故障长得和真 bug 一模一样。
4. **入门课练习占位统一成 `:= by sorry`（设计 D3，R2/R3 都漏做的一条）**：
   行内 `:= sorry` 48 处 + 续行 `sorry` 56 处 = **104 处**。用「判定事件流
   （去掉绝对字节 offset）逐字节对比」证明**零语义变化**——设计 §1.2 的预测成立。
   ⚠️ 我上一轮写的 R3 手册里「练习占位照旧 `:= sorry`」那句是**错的**，已改。
5. **剩下的 216 处 + ~10 条项模式证明**需要判断力（`Exists X (fun …)` → `∃ …`
   83 处、跨行 `And` → `∧` 87 处、`lib/Demo` 的 4 条演示证明），已写手册
   `docs/notes/course-lean-style/R2-full-rewrite-brief.md`（含现状表、三类活、
   边界、验收口径）并派出 2 个 subagent 分头做（lib+画布 / 解答）。
6. **不许动的一处**：`units/notation-cheatsheet*.sokonanoda` **故意**把点名与记法
   并列（大纲 §4 的"第二遍"教学装置），点名在那里是**内容**不是残留。
   C1.5 的"重定位成速查表"是内容改动，与本轮风格改写分开做。
7. **验证（本轮实测）**：卷 I 门禁在每一轮改动后都跑，**始终 36 目标 / 328 checked /
   99 open / 0 判负**；`notation.rs` 47 条测试全绿（新增 1 条）。
8. **subagent 结果已复核（我自己复跑，不采信自述）**：lib+画布那个 agent 声称改了
   9 个文件（`lib/{Equiv,Demo,Exists,Fun,Image,Rel}` + `units/{unit06,08,12}`），
   我逐个 `grade` 复核：**9 个全 rc=0、0 诊断、计数与基线逐项相同** ✓。它列出的
   3 处同类残留我收掉 2 处（单元⑥ 的 `Or` 混用、单元⑧ 的跨行 `Iff`）。
9. **新记录的一条记法边界**（第三处收不掉的原因，值得下一个 agent 知道）：
   记法落在**实参位**、且操作数本身又是记法或集合字面量时，补不出前导类型参数，
   报 `elab-notation-argument-unsolved`（提示文案已经教用户写点名形式）：
   `Set.image Two Two f1 ({aa} ∩ {bb})` —— `∩` 的 `α` 解不出来；
   同文件里单独写 `{aa} ∩ {bb}`（有期望类型）却没问题。
   这条与设计 C4 #5（`∅` 在 `Eq` 操作数位解不出 `α`）同族，**IA-2 要一起评估**：
   签名改隐式之后，"前导类型参数"改成"从第一个显式实参的类型插入"，
   记法路径是否也走同一条插入、还是仍要期望类型，是 IA-2 的设计问题。
10. **当前残留（本轮实测）**：整卷 I **162 处** = 解答 **121**（另一个 subagent 在跑）
   + 记法对照页 **37**（**故意保留**的双写法）+ 其余 **4**。改前的总数是 **829**。
11. **仍欠**：解答那 121 处（subagent 在跑，等验收）；C1.3 的 294 条 hint 词汇；
   C1.5 速查表重定位；C1.6/C1.7 元数据与大纲同步；入门课那边的 C2.5（拍板）与
   R2.5 的 IA-2/IA-3。

## 本轮进度（2026-09-21，第一百一十五轮：卷 I 收尾验收 + C1.6/C1.7 同步）

> 承接第 114 轮的「卷 I 全量改写其实没做完」。本轮把 lib+画布那一片验收掉，
> 并做了 C1.6/C1.7 的元数据与大纲同步。

1. **卷 I 残留 829 → 47**（其中 37 处是记法对照页**故意**保留的双写法、4 处是
   `lib/Exists` 自身的声明、解答仅剩 6 处）。门禁全程保持 **36 目标 · 328 checked ·
   99 open · 0 判负**。
2. **lib+画布的 subagent 成果我逐个复核**（不采信自述）：它声称改了 9 个文件
   （`lib/{Equiv,Demo,Exists,Fun,Image,Rel}` + `units/{unit06,08,12}`），实测
   **9 个全 rc=0、0 诊断、计数与基线逐项相同** ✓。它列的 3 处同类残留我收掉 2 处。
3. **新记边界（第三处收不掉的原因）**：记法落在**实参位**、操作数本身又是记法或
   集合字面量时补不出前导类型参数 ⇒ `elab-notation-argument-unsolved`
   （`Set.image Two Two f1 ({aa} ∩ {bb})` 报 `∩` 的 `α` 解不出；同文件单独写
   `{aa} ∩ {bb}` 却没问题）。与设计 C4 #5（`∅` 在 `Eq` 操作数位）同族，
   **IA-2 要一起评估**。
4. **C1.7 大纲同步**：`docs/design/set-theory-syllabus.md` §4 原本**有两个同名标题**
   （合并残留）且整节写的是「记法分十步慢慢引入、今天先写点名形式」——G-04 落地后
   已成反话。改成 §4.1（调研底稿）+ **§4.2 as-built：课程一律用记法，点名形式退为
   「底下站着什么」的解释**，表头从「顺序 | 记法 | 今天怎么写」改成
   「记法 | 底下的点名形式（讲解时展开）」；§5 里 G-02/G-03/G-04 三行过期的
   「先写点名 + 留 TODO」处置改成「已修」。
5. **C1.6 元数据**：`courses/set-theory/README.md` 的两处写死数字更新为实测——
   `Exists.intro/elim` 点名调用 186 → **102 行 / 14 文件**（少掉的是命题位改用 `∃`
   记法），门禁数字补一条 **2026-09-21 实测 328 checked**（并注明两个数字都别在别处
   再抄，现算就跑门禁）。
6. **验证**：入门课 course 测试 6+4+4 全绿、playground `30 checked / 2 example /
   4 open`（与本轮前一致）、卷 I 门禁 328/99/0、`git diff --stat -- crates/kernel/` 空。
7. **仍欠**：解答最后 6 处（subagent 收尾中）；C1.3 的 294 条 hint 词汇；
   C1.5 速查表重定位；入门课 C2.5（**待拍板**）；R2.5 的 IA-2/IA-3；
   `judge_infer` 的宇宙参数。

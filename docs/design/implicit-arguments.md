# 设计：隐式实参（implicit arguments）

> 日期：2026-09-19。触发（用户原话）：「如果想支持 notation，感觉 `{x : Set}` 这种
> 隐参数自动推导的机制不得不实现了。」
>
> 调研底稿：`docs/notes/course-lean-style/implicit-args-plan.md`（引擎侧，1027 行）
> 与 `docs/notes/course-lean-style/course-impact-implicit-args.md`（课程侧，659 行）。
> 两篇都带 `文件:行号` 证据与实测探针。本文是**设计 + 分期计划**；as-built 追加到 §9。

---

## 0. 一句话

**语法早就支持 `{α : Type}`，缺的是「应用位插入」。** 三条路线里选 **C：风格对齐 +
唯一确定**（不引入元变量，400–600 行，内核零改动），它正好是今天记法路径那个
「补前导类型参数」hack 的**严格一般化**。

**关键安全性质**：签名里没有隐式 binder 时，行为与今天的 `mk_app` 链**逐字节相同**
⇒ **P1 可以独立发布，课程零改动也全绿**。

> **前提已被实测**（2026-09-19 评审跑的，别当成假设）：全仓**只有一处**签名写了隐式
> binder——`courses/set-theory/lib/Exists.sokonanoda:27`，而且**在注释里**
> （`{α : ...}` 出现在讲解文字中，不是真签名）。⇒ 前提今天成立。
> **P1 的第一件事就是把它钉成测试**（一条 front 单测：扫 `courses/**` 与
> `course/**` 的签名 binder 风格，出现隐式 binder 就红），否则这条"安全性质"会随着
> 课程改写**悄悄失效**，而失效的表现是"P1 落地后课程莫名其妙变红"。

---

## 1. 现状（证据见两篇底稿）

| 面 | 事实 |
|---|---|
| **语法** | `{α : Type}` 在 **6 处**全可解析：`def`/`theorem`/`axiom`/`example` 的声明 binder、`inductive` 参数、`ctor` 字段、`fun`/`forall`。风格**活着进内核**：`query goals` 对 `def id2 {α : Type} (a : α) : α := a` 返回 `forall {α : Type 0}, α -> α` |
| **缺口形状** | **不是「不能补参」，而是「只按层数差补前导参数、从不跳过隐式 binder」**。`a ∈ A` 今天能用；把 `Set.mem` 改成 `{α : Type}` 后**仍然能用**（`notation_telescope` 根本不看 `BinderKind`）。但 `Set.mem a A`（点名省参）被内核拒；层数 == 实参数时不补；隐式 binder 在**中间**时报 `elab-notation-argument-unsolved` |
| **tactic 路径不对称** | `by` 引擎**已经**在补前导类型参数（`apply Or.inl` 可用），term 路径没有 |
| **`@f`** | **no-op**：parser 吃掉 `@` 就丢（`parser.rs:2393-2406`），语料 0 处使用 |
| **内核堵死路线 A** | 内核 `Expr` 只有 `{StringLit,NatLit,Proj,Var,Sort,Const,App,Pi,Lambda,Let}`、`Value` 只有 `{Rigid,Unfold,Lam,Pi,Sort,NatLit,StrLit,Thunk}`——**没有元变量/fvar**；`BinderStyle` 的注释自述「只被 pp 使用，不改变类型检查」。**内核冻结 ⇒ 元变量只能活在前端** |
| **`elab_expr` 执行时没有内核环境** | `run_pass` 先走完 walk、最后才 `builder.finish()` → kernel_phase ⇒ 每次探针只能走 `judge_*` = **整前缀重编译**（贵） |

---

## 2. 三条路线对比

| 维度 | A 真元变量 + 合一 | B 探针驱动 | **C 风格对齐 + 唯一确定（推荐）** |
|---|---|---|---|
| 估行（生产/测试） | 1200–2500 / 1500+ | 800–1500 / 800+ | **400–600 / 400–600** |
| 每次应用的内核代价 | 0–多次 defeq | O(k·c) 次**整前缀重编译**（**推断，未 benchmark**：依据是 `run_pass` 先走完 walk、最后才 `builder.finish()`，见 §1 末行） | **0** |
| 课程覆盖 | 100%+ | ≈C（能验证不能搜索） | **≈95%**（课程 2768 处 + prelude）；**剩下那 5% 是"从期望类型解"那一档**（`∅`/`empty`/`univ`），§7 第 1 条明确推迟 ⇒ 与 §7 不矛盾：路线 C 覆盖的是"有显式实参可反解"的应用，`∅` 这类零元记法不在内（见 §8 的 X14） |
| 对既有程序 | 重写每个 App | 中 | **无隐式签名的应用逐字节 no-op** |
| 判定 | 不做 | 留作可选安全网 | **做** |

**⇒ 推荐 C。** 理由：内核不给元变量 ⇒ A 只能在前端自造一套（并要在 zonk 时把每个
App 重写成核项），代价与风险都远超收益；B 每次应用都要重编译前缀，课程 2784 处调用
会被拖垮；C 是**纯前端、零额外内核调用、可逐字节回退**的那一档。

---

## 3. 路线 C 的设计

### 3.1 算法（一句话）

**显式实参按「风格」对齐到显式层、跳过隐式层；被跳过的层由「后续显式实参的类型 +
整体期望类型」的头部匹配唯一确定；解不出就报专用错误码，不猜。**

- 「唯一确定」用**一般化的 `unify_extract`**（`elab.rs:1656-1700` 已有头部匹配的
  形制）+ **occurs check**；
- 每个被跳过的隐式层必须**恰好一个**解，否则 `elab-implicit-argument-unsolved`
  （新错误码，人话 hint 教写显式或补标注）；
- **不做**搜索、不做回溯、不引入元变量。

### 3.2 落点

| 文件 | 改什么 | 估行 |
|---|---|---|
| `crates/front/src/compile/implicit.rs`（**新建**） | 对齐 + 唯一确定 + occurs check + 单测 | 250–400 |
| `crates/front/src/compile/elab.rs` | `Expr::App` 臂接进去（**唯一钩子**，`:1955-1961`） | 15–40 |
| `crates/front/src/compile/check/walk.rs` / `goals.rs` | 签名表补 `binder_styles`（复用 `GoalTemplates` 形制，**零内核调用**） | 60–100 |
| `crates/front/src/compile/prelude.rs` | 登记 prelude 声明的 binder 风格 | 20–40 |
| `crates/front/src/compile/error.rs` | 新码 `elab-implicit-argument-unsolved` | 10 |
| `crates/front/src/parser.rs` | `@f` 给**真语义**（关闭隐式插入），不再是 no-op | 20–40 |

### 3.3 记法路径怎么办（**先保留，跑绿一整轮再删**）

今天 `elab_notation` 用 `builder.mk_app` **直接造核项**（`elab.rs:1057-1078`），
**绕过**了 `elab_expr` 的 `Expr::App` 分支——也就是绕过隐式插入的唯一钩子。
所以：

- **P1 不动记法路径**（补参 hack 与新的隐式插入并存，两条路都对）；
- P1 之后**再收窄**：把记法改走 `elab_expr`，删掉
  `notation_prefix_args`(:1465) / `solve_prefix_args`(:1518) /
  `notation_operand_expected`(:1573) / `set_literal_prefix_args`(:1184)；
  保留 `notation_telescope`(:1505) 与 `choose_notation_target`(:1308)。
- **N7 契约（点名/记法两种写法判卷五元组相等）在退化时必须委托给同一份机械**——
  否则两条路会慢慢分叉（`cli/tests/notation.rs:108-135`、`:594-616` 钉着它）。

### 3.4 护城河重谈（**R1，必须 P1 同轮改**）

今天的教学契约写着「**点名形式永久可用，两种写法判卷一致**（省 `α` 的点名写法
`Set.mem a A` 改前改后同样被拒）」。隐式实参落地后 **`Set.mem a A` 会从「被拒」变成
「通过」**——这是**有意的契约变更**，且被多处钉死：

- `crates/cli/tests/notation.rs:163`（`the_pointful_spelling_keeps_working_and_the_moat_holds`）
  ⇒ **P1 不会让它变红**（2026-09-19 评审实测纠正）：它用的是**测试自带夹具** `LIB`
  （`notation.rs:76-81`，注释明写「`α` 是**显式**前导参数」），三个 binder 全是
  `(α : Type)`；路线 C 只跳过**隐式** binder ⇒ 3 个显式 binder 配 2 个实参**仍然是
  元数错误**，测试保持绿。它只在**夹具被改写成隐式**（即 B1 之后重钉夹具）时才翻。
  ⇒ 这条不是 P1 的验收判据，而是 **B1/B5 的验收判据**；
- `docs/design/notation-subset.md:106-107`、`:25-29`、`:97-107`（N4.2/N4.3 作废）、
  `:552`、`:267-268`；
- `docs/design/course-lean-style.md`（本文档的兄弟）与课程
  `units/notation-cheatsheet.sokonanoda:45-50` 的教学话术。

---

## 4. 分阶段

**编号口径**（2026-09-19 评审修正）：本文的期号一律带 `IA-` 前缀（implicit arguments）。
主计划 §5 的 **R2.5** 用它自己的 `P0/P1/P2/P3`，两份文档从前**都叫 P0/P1/P2** 而含义不同
——现在分开：**R2.5-P0 = NI-0 + IA-0**、**R2.5-P1 = IA-1**、**R2.5-P3 = IA-2**。

| 期 | 内容 | 验收（**可独立发布**） |
|---|---|---|
| **IA-0** | 本文 + 拍板（§7 的四个问题）+ **补 G-19/G-20 台账** | 文档评审；`python3 scripts/gap.py check` exit 0 |
| **IA-1** | 路线 C + `@` + 签名表；**记法 hack 保留**；**先把"无隐式 binder"前提钉成测试**（§0 的注） | `cargo test --workspace --locked` exit 0；`scripts/soko grade "$PWD/courses/set-theory/units/unit02-subsets-empty.sokonanoda"` exit 0；`python3 courses/set-theory/tools/check.py` = **36/329/99/0 逐项不变**；`git diff --stat -- crates/kernel/` **空**；新错误码 `elab-implicit-argument-unsolved` 进 `docs/protocol.md` 的码表，**文案含"把参数写全"的可执行例子** |
| **IA-2** | 课程改写：`lib/` **≈39** 声明改隐式 + units **2768** 处缩短（分批见 §5）。**记法声明照写**（它指向名字，签名变了不用改） | 同上 + 会红的测试清单（§6）全部重钉；**§3.4 的护城河话术与 `notation.rs:163` 的夹具同轮改**（它是 B1/B5 的判据，不是 IA-1 的） |
| **IA-3** | 收窄/删除记法补参 hack（§3.3） | N7 五元组相等契约仍绿 |
| **IA-4（未排期）** | 路线 A，仅当出现 C 覆盖不到、且不能靠改签名规避的真实需求时立项 | — |

> **版本纪律（漏了就发不出去）**：`courses/set-theory/sokonanoda.toml` 的
> `requires = "0.61"` 是**硬钉**——语言版本一 bump，`scripts/soko` 的版本钉守卫
> **直接 exit 3**（不是警告）。⇒ IA-1 若含语言行为变更，**同轮** bump
> `Cargo.toml` + `editor/vscode/package.json` + `Cargo.lock` + 课程清单的 `requires`
> （主计划 §F 的 F10 有完整清单）。

---

## 5. 课程分批（**B0 是前置，必须先做**）

| 批 | 内容 | 为什么这个顺序 |
|---|---|---|
| **B0** | ① 修 `∅` 嵌在记法里 + `by` 块（**今天就坏**，见 §8）；② 拍板错误码是否保持 | 227 处 `Set.empty` 几乎全在这个形状；不修就没法验收隐式实参是否顺手修好 |
| **B1** | `lib/Set` **23** 条 + `lib/Exists` 3 条签名（`def Set` **不动**；条数按 `query goals` 实测，底稿的 22 是旧的） | `lib/Set` 是 24 个 unit/solution 的 import 依赖 ⇒ **一次改完**（虽然实测「改隐式当场不坏」，但语义变更要原子） |
| **B2** | `lib/Image`(**6**：`Set.image`/`Set.preimage`/`Set.mem_image`/`Set.mem_preimage`/`Set.image_mono`/`Set.image_subset_iff`) + `lib/Equiv`(**5**：`Set.MapsTo`/`Set.LeftInvOn`/`Set.RightInvOn`/`Set.Equiv`/`Set.Equiv.mk`) + `lib/Demo` | 与 B1 同层。⚠️ 底稿写的是 4 / 3（**数错了**，2026-09-19 评审用 `query goals` 逐个数出来的）⇒「≈35 条需改签名」应上调到 **≈39 条** |
| **B3** | 已 Lean 化的 4 个文件（unit01 对 + cheatsheet 对，54 行调用） | 试点，验证工具链。⚠️ `lib/Set.sokonanoda:164-177` 这类行号只在**工作树**成立（W1 的记法搬家把行号挪了）⇒ 定位一律按**符号名**（`infix:50 " ∈ "`），别按行号 |
| **B4** | 单元 3/4/6/7/9/10/11 + 解答（避开硬断言） | 主体 |
| **B5** | **原子同轮**：unit02(`notation.rs:557`/`:346`)、unit08(`:758`)、unit05(`query.rs:559`) + 改那三条测试 + **重钉 `notation.rs:163` 的夹具**（它今天用全显式 binder 的测试夹具 ⇒ IA-1 不会让它红；改成隐式后判据要反过来） | 这三处被测试逐字钉死。**门禁行**：每批跑 `check.py`（G1–G6，含 **G6 清单自洽**：卷章 id 唯一、unit 恰好一章、`prereqs` 不悬空）——unit 改写会动文件名与清单，**G6 就是那条护栏**（主计划 §F1/F2） |
| **B6** | 收尾：**712** 处注释/hint + `README.md`(`:85/:86/:126/:147/:170-172/:209-210`) + cheatsheet(`:19-23/:40-43/:45-50/:52-59/:77`) + `courses/set-theory/AGENTS.md` 新增「点名一律省前导类型参数」 + **根 `AGENTS.md` 的「硬规则速记」第 3 条**（⚠️ 底稿写的 `AGENTS.md:121` 是**空行**，2026-09-19 评审纠正；改按**小节名**定位，别按行号）+ `course-stdlib.md:309`+§3.2 加变形 D + `syllabus §4`(:206-227) + `course-lean-style.md` 的 N-1 + `notation-subset.md` | 文档纪律 |
| **B7（另一刀，不建议本轮）** | `ext`/`subset_def` 的 A B 也隐式、`empty`/`univ` 走期望类型解、**prelude 的 `Or.inr ha`** | 会打红入门课 13 个测试（`course.rs`/`course_status.rs`/`cli.rs` 的 GOLDEN） |

**调用点规模**（2026-09-19 评审**复算**，口径写清）：

| 量 | 底稿 | 复算 | 口径 |
|---|---|---|---|
| 点名调用（去注释） | 2784 | **2768** | 逐名字表求和，**每个名字都对得上**；差在 ±2% 内 |
| 其中带前导类型实参 | 2308（83%） | **不可复算** | 底稿自述是正则近似 ⇒ 引用时**只说比例、不说绝对值** |
| 含注释的点名调用 | 3549 | **3518** | |
| hint / 叙事里的 | 765 | **712** | |
| 涉及文件 | 30 | **35** | |

⚠️ **底稿 §8.4 那条「计数口径（复现用）」的命令是错的**：它**没去注释**，跑出来是
3454（worktree）/ 3518（HEAD）——复现的是 3549 那一列，不是 2784。引用数字时用本表。
最重：`solutions/unit12` 562、`solutions/unit08` 529、`solutions/unit09` 310、`unit08` 204。

---

## 6. 会红的测试（P1/P2 同轮改）

| 测试 | 钉住什么 | 怎么改 |
|---|---|---|
| `crates/cli/tests/notation.rs:163` | 省 α 的 `Set.mem a A` **必须 kernel-rejected**（护城河）——**用的是测试自带夹具（全显式 binder）** ⇒ IA-1 不会让它红 | **B1/B5 同轮**：夹具改隐式后判据反过来（省 α exit 0）+ 补「写全 α 仍绿」 |
| `notation.rs` `the_shipped_course_uses_the_library_notation` | 单元禁 `infix`/`notation` + 六条记法签名 + 零点名残留 + 每题 `by` | **已重钉**（R2 课程改写第一站）；IA-1 落地后再加「省参也判绿」 |
| `notation.rs` `a_real_course_unit_grades_identically_in_either_spelling` | 夹具 `pointful_variant` 逐字替换 unit02 两行签名 + `open>=8` | **已按此改**：方向反转（画布是记法版，夹具造点名版），仍同判 |
| `notation.rs:758` | unit08 含 `binder_notation` | W1 已把它搬进 `lib/Exists` ⇒ 改钉「库声明、单元不再自带」 |
| `notation.rs:223` | `#check ∅` 的 `elab-notation-argument-unsolved` 码/阶段 | 若码/阶段变则重钉 |
| `notation.rs:441` | 拷贝真 `lib/Set` | **改 lib 后第一个该单跑的哨兵**（预期不红） |
| `crates/cli/tests/query.rs:559` | unit05 的 5/7 | 随 unit05 改写重钉 |

**站点与对外文案**（**发布那轮**才改，因为站点写的是「已发布版本」的事实——
`docs/design/site-rebuild/spec/D9-page-brief.md` §4.0）：`C1-language.md:179/:1183`、
`C4-status-roadmap.md:117/:433`、`site/` 的对照页与 non-goals 都明写「没有隐式实参自动插入 /
应用是逐位显式的」⇒ P1 **未发布前它们是真话**，**发布那轮必须同轮改**（否则站点说了假话）。
另有两处**描述现状**的设计文档引用同一事实：`docs/design/namespace-open.md:238/:364`
（`section`/`variable` 做不动的论据之一就是"没有隐式参数插入"——P1 落地后这条论据**部分失效**，
但 `variable` 的 auto-bound 仍缺，结论不变，需补一句限定）。

**`check.py` 的 G1–G6 判据本身不锁计数** ⇒ 不会因为隐式实参变红；红只会来自改写质量
（某单元 `grade` ≠ 0、G4 按名覆盖、把解答写成 `example`）。

---

## 7. 需拍板（P0 的产出）

1. **插入做到哪一档**：建议**只做「从后续显式实参的域反解」+「箭头值域反解」**，
   「从期望类型解」推迟（那是 `∅`/`empty`/`univ` 那一档，最难）。
2. **显式实参写在隐式位上收不收**：**建议收**（否则 2308 处必须同一个 commit 全改完）。
3. **记法补参 hack**：**建议先保留、跑绿一整轮再删**（§3.3）。
4. **prelude 本刀不动**（`Or.inr ha` 会打红入门课 13 个 GOLDEN，见 B7）。
5. **`@f` 给不给真语义**：今天 `@` 被 parser 吃掉就丢（`parser.rs:2393-2406`，是 no-op，
   语料 0 处使用）。IA-1 顺带让它**关闭隐式插入**（Lean 语义）——20–40 行，且
   "显式写全参数"是课程护城河的逃生门，值得有。**建议做**（不做也不阻塞）。

---

## 8. 顺带发现的两个真 bug（**B0 必须先清**）

| # | 现象 | 根因 | 处置 |
|---|---|---|---|
| **X14** | **`∅`（零元记法）落在「被应用」的位置 + `by` 块**（**已修，0.61.0**，台账 **G-19**） | **实测三连**：`∅ ⊆ A := by intro x; intro hx; exact False.elim (A x) hx` ⇒ ``记法 `∅` 展开成 `Set.empty` 时补不出前面的类型参数``（exit 1）；**同一句 term 模式通过**；`Set.subset α (Set.empty α) A` 在 `by` 里也**通过**；`x ∈ ∅ → False` 在 `by` 里**也通过** | `∅` 是**零元**记法（`Notation{target:"Set.empty", operands:[]}`）。作 `∈` 的操作数时能从 `Set.mem` 的参数位拿到**期望类型 `Set α`** ⇒ 解得出；作 `⊆` 的操作数时 `intro` 先要把 `Set.subset`（**def**）delta 展开成 `∀ x, A x -> B x`，`∅` 就落进**被应用**的位置（`∅ x`），**期望类型没了** ⇒ 回读时 `Set.empty` 零实参、`α` 无解 | 与 X11/X12 同族（render→re-read 丢信息）。**实际修法**（不是设计时猜的两条）：`by.rs` 的 `Tactic::Intro` 在目标头剥不动（= 头是 def）时**先用内核 pp 的规范形态再剥**（`canonical_goal_with_spec`，点名 + 参数写全），源级 delta 展开只作退路——pp 形态下 `∅ ⊆ A` 是 `Set.subset α (Set.empty α) A`，展开成 `(Set.empty α) x` ⇒ 回读无碍。详见 `docs/gaps/WO-012`。**课程 227 处 `Set.empty` 的 B0 前置已清** || **X15** | **LSP 单文件 parse 失败吃掉项目报告** ⇒ 课程单元在编辑器里 hover/documentSymbol/goals 全死 + 假 `notation-unknown-symbol`（CLI 判卷却 exit 0） | `crates/lsp/src/lib.rs:158-166` 在 `set_text_with_overlay` **已装好项目报告**之后，因 `parse_error.is_some()` 把 `report = None`；`:78-88` 又让 parse 错误优先 | 见 `docs/design/notation-input.md` §1.1（**P0**，hover 功能的前置） |

---

## 9. as-built（**IA-1 已落**，2026-09-21）

> 实际改了什么 / 与设计的偏差 / 实测数字。**内核零改动**
> （`git diff --stat -- crates/kernel/` 空）；版本**不另 bump**：0.62.0 是本轮
> 未发布批次（NI-2 已把它从 0.61.0 抬上来），IA-1 与它同批发布。

| 项 | 实际改了什么 | 落点 |
|---|---|---|
| **签名表**（零内核调用） | `KnownName::Decl` 加 `implicit_prefix: usize`（声明望远镜的**前导隐式 binder 个数**），`leading_implicit_prefix(ty)` 纯源级 AST 走查；`walk.rs` 的 6 个登记点算出来，prelude / 内部构造点固定 `0`（§7 第 4 条：prelude 本刀不动） | `compile/elab.rs`、`compile/check/walk.rs`、`compile/prelude.rs` |
| **路线 C** | 新模块 `compile/implicit.rs`：`telescope`（**带风格**的 Pi 层，`peel_pi` 把风格丢了）、`leading_implicit`、`solve_prefix`（路线 ①：扫**全部后续显式层**，用它们的实参**类型**头部匹配；顺序代入；解不出返回 `None`）+ 5 条单测 | `compile/implicit.rs`（新） |
| **唯一钩子** | `Expr::App` 臂：头有前导隐式 binder 时，按风格对齐、解出、插入；**免费闸门**——`implicit_prefix == 0` 直接返回 `None`，走老路**逐字节不变** | `compile/elab.rs::try_implicit_application` |
| **`@` 真语义** | 新 `Expr::App::explicit_spine`；parser 见到 `@` 置 `saw_at`，`parse_app` 取走并写进这条脊的每个 App 节点；渲染时**只在头前打一个 `@`**（逐节点会打成 `@(@f a) b`） | `ast.rs`、`parser.rs`、`proof.rs` |
| **新错误码** | `elab-implicit-argument-unsolved`（stage=elab，hint 教"把参数写全"、点名写法永远可用），进 `docs/protocol.md` 码表 + `compile/tests.rs` 的 ErrorKind 穷尽表 | `compile/error.rs` |

**与设计的偏差（逐条）**：

1. **`render_expr` 不再给 `UniverseApp` 凭空补 `@`**（设计没预料到）。这是本刀
   唯一一处**必须同轮改的既有行为**：`@` 以前只是"把实参写显式"的提示，渲染
   `Eq.{1}` 时补 `@` 是无害的；`@` 变成真语义后，判卷通道"渲染 → 回读"就会
   **改变含义**（`Eq.{1} β a b` 被读成 `α := β`）。实测症状很绕：`judge` 合成的
   声明里任何 `UniverseApp` 都带上 `@` ⇒ 解析时置位 `saw_at` ⇒ 那条脊被判成
   显式 ⇒ 记法目标 `Aᶜ ∪ B` 的前导参数解不出（`a_by_block_whose_goal_carries_notation_still_judges` 变红）。
   现在：`render_expr(UniverseApp)` 打 `Name.{levels}`，用户写的 `@` 由
   `explicit_spine` 负责。`render_expr_round_trips` 的三条用例同轮重钉。
2. **§0 的"前提"只扫了卷 I，漏了入门课**（前提测试当场抓到）：`course/` 里有
   **两处刻意**的隐式签名——`Eq.symm`（单元⑤ 画布 + 解答）与 `eq_refl_prop`
   （单元② 画布 + 解答）：**这两道题教的就是隐式 binder**。它们只在 prelude 的
   `Eq.subst`/`Eq.refl` 上用（prelude 的 prefix 固定 0）⇒ IA-1 实测**不影响**它们
   （全仓 1210 条测试全绿）。前提测试改成"**除这两处教学例外外**，全仓签名没有
   隐式 binder"。
3. **`solve_prefix` 扫全部显式层**（比 §3.1 的"第一个显式实参"更宽）：
   `picks {α} (n : Nat) (b : α)` 调用 `picks 3 b` 时 `α` 只出现在**第二**个显式
   层里——只看第一层会解不出。现在按层序找"后面第一个提到它的层"，用那一层的
   实参类型解。**路线 ②（由期望类型解）仍未做**（§7 第 1 条）。
4. **tactic 里的错误码被包成 `elab-tactic-failed`**（既有口径，非本刀引入）：
   专用码只在**直接 elaborate** 的位置（签名/值位）看得见；`by` 块里报的是
   `elab-tactic-failed` + 同一段人话消息。回归测试按值位写。
5. **`@` 的渲染丢孤立 `@`**：`@f`（无实参）与 `f` 是同一个项 ⇒ 渲染丢 `@`
   （往返"不改含义"优先）；带实参时打回来。`render_expr_round_trips` 三条用例
   钉住这三种形状。

**新增回归**（`crates/cli/tests/notation.rs`，**45** 条）：
`no_course_signature_uses_an_implicit_binder`（§0 的前提，含两处教学例外）、
`implicit_arguments_are_inserted_from_the_first_explicit_argument`、
`the_at_marker_disables_implicit_insertion`、
`an_unsolvable_implicit_argument_reports_its_own_code`；
`crates/front/src/compile/implicit.rs` 另有 5 条纯单测（风格解析 / 多 binder 折叠 /
前导隐式计数 / 逐层反解 / 解不出不许猜）。

**验证（本轮实测）**：`cargo test --workspace --locked` **1210 passed / 0 failed**；
课程门禁 **36 目标 / 0 判负 / 328 checked / 99 open**（与 IA-1 之前**逐项相同**——
安全性质成立：课程签名没有隐式 binder ⇒ 插入路径一行不跑）；
`git diff --stat -- crates/kernel/` **空**。

**仍未做（下一轮）**：IA-2（`lib/` ≈39 条签名改隐式 + units 2768 处缩短 + §6 的
测试重钉 + §3.4 的护城河话术与 `notation.rs:163` 夹具）与 IA-3（收窄/删除记法
补参 hack，N7 五元组契约仍绿）。`course/` 的 B7（`ext`/`subset_def` 隐式、
prelude 的 `Or.inr ha`）会打红入门课 13 个 GOLDEN，仍**不建议本轮**。

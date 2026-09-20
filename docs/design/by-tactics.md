# by-tactic 块 + 编辑器 goal-state（第十九轮设计）

> 日期：2026-09-09。触发：用户要求「实现一些基础 tactic，跟 Lean 4 一样用 `by`
> 开始」，并补充 assumption / rfl；同时要求调研并设计 VSCode 前端如何显示 goal
> state。首期 tactic 集：**intro / exact / apply / assumption / rfl**。
> 三件套：新增语法 = 课程 + 测试 + 白名单。

## 1. 目标形态

```lean
theorem and_swap : (a : Prop) -> (b : Prop) -> And a b -> And b a := by
  intro a
  intro b
  intro h
  exact And.intro b a (And.right a b h) (And.left a b h)
```

- `theorem/def/example name : T := by <tactic 序列>`；
- tactic 之间用 `;` 分隔，允许跨行（教学子集不引入缩进敏感语法）；
- 每个 tactic 的裁决**永远走 kernel**（`front::judge` 合成声明模式，REQUIREMENTS §2.8）；
- `by` 块没写满 = 合法 Open 状态（尾部 `sorry`），与现有「部分作答」同语义。

## 2. 首期 tactic 集（白名单 = 课程）

| tactic | 语义 | 判定 |
|---|---|---|
| `intro x` | 当前目标剥一层 Pi/Arrow，引入假设 `x`（变 lambda binder） | 纯 AST 结构（非 Pi 报错） |
| `exact e` | 术语 `e` 的类型与目标 definitional equal → 填入 | `judge_terms`（kernel） |
| `apply f` | `f : A1→…→An→B`，B 与目标合一 → 换成 A1..An 子目标 | `judge_infer` 推断 f 类型 + 位置 spine 合一 + `judge_terms` 验证 |
| `assumption` | 从最内层假设起找类型与目标 defeq 者 | `judge_terms` 逐个（复用 `assumption_kernel`） |
| `rfl` | 目标为 `Eq α x y` 时造 `Eq.refl.{u} α x` | `judge_terms`（内核算两边；复用 `eq_refl_candidate`） |
| `match` | 情形分析：`match c with | p => <项> …`，以当前目标为期望类型；臂体是**项**（同值位 `match`），语义等价 `exact (match …)` | `judge_terms`（kernel） |
| `sorry` | 占位：当前目标保持开放（no-op），与值位 `sorry` 同语义 | 无 |

## 3. 引擎：把 `by` 块翻译成 lambda AST

新模块 `crates/front/src/by.rs`（~400 行上限，超则拆）。

### 3.1 目标树（apply 的子目标需要多目标）

```rust
enum GoalKind {
    Hole,                      // 未闭合：解 = 洞（sorry）
    Closed(Expr),              // exact/assumption/rfl 闭合
    Apply { f: Expr, args: Vec<usize> },  // 解 = f <子目标解…>，args 是子目标节点 id
}
struct GoalNode { ty: Expr, intros: Vec<Binder>, kind: GoalKind }
```

- `nodes: Vec<GoalNode>` + `worklist: Vec<usize>`（待解目标，DFS 序，**末尾 = 当前**）。
- `apply` 造子目标时按**反序**压 worklist（先解第一个）。
- 组装：递归求每个节点的解（`intros` 包 lambda，`Apply` 包应用，`Hole` = sorry）。
- 未闭合 → 最终 lambda 尾部是 `sorry` → 走既有 `open_goal` → OpenExercise（剩余目标 + binders 免费获得）。

### 3.2 每个 tactic 的执行

- `intro x`：当前节点 `ty` 是 `Forall`/`Arrow` → 剥一层，`intros.push`，`ty=body`。
- `exact e`：`judge_terms(prefix, options, &spec(), &[e])`（spec = 当前目标 + 路径上的 intros，
  同 `ProofState::spec`）→ Match 则 `kind=Closed(parse e)`。
- `assumption`：intros 从内到外逐个名过 `judge_terms`，首个 Match 填入。
- `rfl`：当前目标 AST 匹配 `Eq α x y` 形状 → 造 `Eq.refl.{u} α x` 文本 → `judge_terms` 裁决。
- `apply f`：见 §3.3。

### 3.3 apply（位置 spine 合一）

1. `judge_infer(prefix, options, &binders, f)` → f 的类型文本（新助手，见 §4）；
2. 解析成 AST，剥 Pi 链得 `binders_f: Vec<(name, dom)>` + `codomain: Expr`；
3. 把 `codomain` 与当前目标**位置 spine 合一**（头相同则逐参数位对应），得替换 σ
   （codomain 里出现的 Pi binder 名 → 目标实参）；
4. **出现在 codomain 的 Pi binder = 类型参数**（被 σ 填掉，不是子目标）；
   **其余 Pi binder = 子目标**，域类型 σ 代入；
5. 验证：`judge_terms` 把 `f <σ 类型参数> <sorry>*子目标数` 过一遍——结果
   `Error("elab-hole-misplaced")`（仍开）视为合法，`Mismatch` 拒绝，其它 Error 原样报；
6. 子目标按 binder 顺序入树 + 反序压 worklist。

覆盖教学常用：`Or.inl/inr`（1 子目标）、`And.intro`（2 子目标）、`True.intro`（0）、
`And.left/right`、`False.rec`。不支持的形状（头不匹配 / 依赖过深）报教学错误，
不硬猜。

### 3.4 前缀源码：FolFile 增 `src` 字段

`judge_terms` 需要 `prefix_src`。`parse(text)` 时把原文存进 `FolFile.src`；
`run_pass` 按 `command.span.start.offset` 切片 `&src[..offset]` 传引擎。
judge 合成的 FolFile `src=""`（无 by 块，用不到）。

## 4. judge.rs 新增 `judge_infer`

```rust
pub fn judge_infer(prefix_src, options, binders: &[GoalBinderSpec], term: &str)
    -> Result<String, Judgement>
```
合成 `<prefix>\n#check fun <binders> => <term>\n` 走 `check_document_with`，
取 TypeChecked 文本（`fun (b1:T1)=>…=><term>` 的 Pi 类型），剥掉 n 层 Pi 得
codomain = term 的类型文本。复用合成声明 + kernel 唯一裁判纪律。

## 5. 流水线接入（check.rs）

> **0.20.0 追加**：`by` 也识别于 **lambda 体尾部**（`fun (x : Q) => by …`）——
> `split_by_value` 沿链收集 binder 作为引擎初始上下文，其余机制不变。与值位
> `intro` / `apply` 的 lambda 尾支持（`term-apply.md` §2.4）同一批放开。

`run_pass` 命令循环的 `Command::Def/Theorem/Example` 分支：值位若是 `By` 块 →
先跑引擎得 lambda AST（可能带尾 `sorry`），当新 `val` 走既有分流：
- 有洞 → `open_goal` → `PendingOp::OpenExercise`；
- 闭合 → `build_theorem`/`build_def` → kernel 检查（`try_check_declar`）。
`build_*` 值位因此接收普通 `Expr`（引擎产物），无需改 build_* 签名。

## 6. 编辑器 goal-state（协议 + VSCode，Phase 2）

### 6.1 front 产出 per-step 状态（编译期，请求期零重算）

`DeclState` 增 `by_steps: Vec<ByStepState>`：
```rust
struct ByStepState { span: Span, goal: Option<String>, binders: Vec<GoalBinder> }
```
引擎每执行一个 tactic 记录该步执行后的 `(goal, binders)` + 该 tactic 的源码 span。
进 I8 session 快照，随诊断一起刷新。

### 6.2 LSP 新请求 `soko/stateAt`（coq-lsp `proof/goals` 模式）

**as-built（2026-09-10 实现轮定稿）**：

- 请求 `{ textDocument, position }`；
- 响应：

  ```json
  {
    "version": 5,
    "decl": {"name": "and_swap", "kind": "theorem", "status": "open", "range": {}},
    "goal": "And b a",
    "binders": [{"name": "a", "ty": "Prop"}],
    "span": {},
    "step": 1,
    "total": 3
  }
  ```

  `decl` 为 `null`（光标不在任何声明内）时其余字段为 `goal: null / binders: [] /
  span: null / step: -1 / total: 0`；`goal: null` 表示「无剩余目标」（已闭合）；
  `step` 是选中的 per-step 状态下标（`-1` = 根状态，即尚未执行任何 tactic）。
- 选取规则改为 **Lean `goalsAt?` 语义**（比本文初稿的「执行后」更贴合学习者：
  光标停在某条 tactic 上时他要看的是**这条 tactic 要证的目标**）：
  1. 光标在某 step 的 span `[start, end)` 内 → 取该 step 的**执行前**状态
     （即 `steps[i-1]` 的执行后状态；`i == 0` 时是根状态）；
  2. 否则取「终点 ≤ 光标」的最后一步的执行后状态；
  3. 都没有 → 根状态。
- 无 by 块的声明（`steps` 为空）：非 by 的 Open 练习直接返回其剩余
  `goal`/`binders`（`step = -1`）；已判定声明返回无剩余目标。根状态的目标
  用 `ty_text`（内核渲染的完整声明类型），拿不到时退回剩余 `goal`。
- 返回文档 `version`，客户端丢弃过期响应。既有 `soko/goals` 不动。

### 6.3 VSCode：方案 A（零 webview，先做）

**as-built（2026-09-10）**：练习树顶部「当前光标处」组已实现——
`onDidChangeTextEditorSelection` 去抖 200ms → `soko/stateAt`（请求序号 +
活动文档守卫丢弃过期响应；诊断刷新后重取）；组内 = 目标（点击
`sokonanoda.revealRange` 跳 tactic）+ 假设 + `by 进度 k/n`。静态契约测试
守护客户端必须消费 `soko/stateAt` 且不得文本扫洞。扩展版本 0.6.0。

设计要点（原方案）：

扩展现有「练习」树（`extension.js:166-262`）：顶部加「当前光标处」节点组
（`⊢ goal` + binders + 当前 step 范围，点击 `revealRange` 跳转）。刷新：
新增 `window.onDidChangeTextEditorSelection` 去抖 ~200ms → `soko/stateAt`；
编辑/重编译沿用 `onDidChangeDiagnostics`。方案 B（Lean Infoview 式 webview）
留后续，协议先行验证。

> **0.46.0 更新**：`match` 作为 tactic 加入白名单（臂体是项，见 §2）。同时修
> `judge_terms` 合成文件：把真实前缀作为 `src`、合成声明 span 放到前缀之后，
> 使 `match` 降低时的宇宙查询（依赖 `command.span().start` 的前缀切片）可用 ——
> 这也让 `by exact match …` 直接可用。

## 7. 课程（三件套之课程）

- `course/` 新增单元 6「by 写法」（中文），配套英文镜像（双语纪律沿用）；
  讲解 `by` 语法 + **首期五个 tactic**（当时的白名单），练习题逐题渐进（先
  intro+exact，再 assumption/rfl，最后 apply 拆目标）；
- `playground.sokonanoda` 追加 2–3 道 by 练习题（含一道 apply And.intro）；
- 白名单 = 解析器只认**当时那五个** tactic + `by` 关键字（**已扩充，见下**）。

> **as-built（R2/R3，2026-09-21）**：白名单此后长到 ~16 条（`constructor` /
> `left` / `right` / `use` / `cases` / `have` / `exfalso` / `match` / `⟨a, b⟩` …，
> 清单见 `skills/sokonanoda-teacher/SKILL.md` 与 `docs/design/course-lean-style.md`
> §L3）。**课程侧**：入门课的 by 单元现在是**单元④**（单元号在 P2 里变过），
> 而本课 `And`/`Or` 是**自建骨架** ⇒ 单元④①⑧ 仍然只教
> `intro`/`exact`/`apply`/`assumption`/`rfl`/`have`（`constructor`/`cases` 要真归纳，
> 在这三个单元会报「需要目标是归纳类型」）；`left`/`right`/`cases` 只在
> ⑨⑩⑪ 可用（那里 `Or` 是 `inductive`）。R3 实测，见
> `docs/notes/course-lean-style/R3-rewrite-brief.md` §1 与
> `docs/design/course-lean-style.md` §9「R3」。

## 8. 测试（三件套之测试）

- **front**：parse（by 块 AST、白名单拒绝未知 tactic）、引擎（intro/exact/
  assumption/rfl/apply 各语义、apply 子目标顺序、未闭合→Open）、judge_infer、
  全语料零 `$`、kernel 判定闭环；
- **CLI e2e**：含 by 的文档 `--json` 事件（decl.checked / exercise.open）、
  错误 tactic 的诊断码；
- **course golden**：单元 6 计数钉死；**协议**：`soko/stateAt` 契约测试 +
  protocol.md 追加小节；
- 既有断言对齐（若 parse 计数变化）。

## 9. 边界与不修项

- 不做 `;` **组合子语义**（Lean 的"对所有子目标施加下一 tactic"）/`<;>`、`repeat`/`first`、
  `by_cases` 等；只做上表七个；
- `apply` 不做完整高阶合一（只位置 spine），不支持时报教学错误；
- **分隔符 = `;` 或换行**（0.51.0，Lean 风格，二者可混用）。**换行边界的精确定义**：
  解析 tactic 时（`by_depth > 0`），若下一个 token 在**更晚的行**且是 **tactic 关键字**
  （`intro`/`exact`/`apply`/`assumption`/`rfl`/`match`/`sorry`），当前 tactic 的表达式就
  在此结束——这是为了让 `exact f` 换行 `apply g` 不被贪婪读成 `f apply g`。
  仍**不引入缩进敏感**；续行只要不以 tactic 关键字开头就照常拼接（多行项可用 `;` 或括号）。
- kernel 冻结：引擎全在 front 层（AST + judge 合成声明），kernel 一行不动；
- webview goal 面板（方案 B）不在本轮实现，协议先行。

## 10. 验收

- `playground`/`course` 含 by 的文档全绿，`--json` 锚点更新；
- front + CLI + course 测试全绿，fmt/clippy 干净；
- LSP `soko/stateAt` 端到端测试通过；VSCode「当前光标处」goal 组可渲染；
- STATUS / REQUIREMENTS §9 / protocol.md 更新。

## 11. as-built：换行分隔（0.51.0）

- `Parser.by_depth`：解析 tactic 期间加一（`parse_tactic` 包一层，Ok/Err 都减）。
- `starts_atom` 在 `by_depth > 0 && next_line_starts_a_tactic()` 时返回 false → 应用不吃下一
  行 tactic；`parse_by_block` 在「`;`」或「下一行以 tactic 关键字开头」时继续。
- 边界取舍（有意不支持的写法）：同一行内不写 `;` 不算分隔（`exact f apply g` 仍是应用）；
  续行**以 tactic 关键字开头**的多行项会被切开（用同一种写法或括号规避）；`sorry` 在下一行
  即视为新 tactic。注释不影响判定（按 token span 行号比较）。
- 测试：parser `by_block_newlines_separate_tactics`、`by_block_newline_boundary_beats_application`、
  `by_block_multiline_application_is_one_tactic`、`by_block_semicolons_still_work_and_mix_with_newlines`、
  `by_block_does_not_consume_the_next_command`；CLI `cli_by_newline_separated_tactics_check_via_kernel`；
  `playground.sokonanoda` 的 `forall_and` 去掉行尾 `;` 作为活样例。

## 12. as-built：判定携带声明的宇宙参数（0.62.0，R3 实测补）

**症状（R3 把解答改写成 tactic 时撞上）**：目标里出现 `Sort u` / `Eq.{u}` 时，
`theorem Eq.flip {u} : {α : Sort u} → … := by intro α a b h; exact Eq.subst.{u} …`
报 `elab-tactic-failed: universe variable `u` is not declared in this declaration`。
根因不在宇宙机制（`judge.rs` 早就把 `OpenGoalSpec.universe` 拼进合成声明的
`Command::Def { universe }`，`judge_uses_carried_universe_for_sort_u_goals` 钉着它），
而在 **by 引擎从来没把声明的宇宙参数填进 `OpenGoalSpec`**：`by.rs::spec_of` 与
`spec_of_for_judge` 都硬写 `universe: Vec::new()`。后果是**宇宙多态定理一律写不了
tactic**——卷 I 的 `Set.{u}` 遍地都是，这条不修，Lean 风格改写就是空话。

**改动（全在 front，kernel 一行不动）**：把声明的宇宙参数名一路带到判定规格：

```
walk.rs  def/theorem 的 `universe: &[String]`
  → lower_value → lower_by_val → by::run_by
  → run_tactics → {apply_tactic, cases_tactic, ctor_tactic, exact_tactic}
  → {judge, judge_with_levels, spec_of, spec_of_for_judge} → OpenGoalSpec.universe
```

- `example` 没有宇宙 binder：`walk.rs` 的 `example` 分支传 `&[]`（判定语义不变）。
- `spec_of` / `spec_of_for_judge` 的 `universe` 从「硬写空」改成 `universe.to_vec()`。
- 安全性：`universe` 为空（绝大多数声明、所有 `example`）时 `OpenGoalSpec` 与改动前
  **逐字段相同** ⇒ 既有行为零变化，回归靠全量 `cargo test --workspace`。

**测试**：front `by_block_carries_the_declaration_universe_parameters`（端到端：源文件
→ `check_document` → `DeclStatus::Checked`）；课程语料 `course/solutions/unit5-universes-sort-solution.sokonanoda`
的 `theorem Eq.symm {u} : {α : Sort u} → … := by …` 是活样例（它同时是
`GOLDEN` 计数的锚点）。

**同轮第二处（`cases` 的头解析认记法）**：`cases` 降低成 `match` 之前要
`unfold_to_inductive` 把 `def` 的**体**代进来，而体的形态取决于**库怎么写**——
`Or (A x) (B x)` 是 `App` 链，`A x ∨ B x`（记法）是记法节点。原先头解析用只走
`Expr::App` 的 `spine_of`，于是「库改用记法」会把所有 `cases h`（`h : x ∈ A ∪ B`）
整类打红（实测：卷 I 门禁 328/0 → 326/2）。改用 `spine_with_notation`（记法节点的
头就是它的 `target`）。回归：
`notation.rs::cases_sees_through_a_definition_body_written_with_notation`。

**同轮第三处（binder 记法的实参位必须两处同款）**：`∃ (x : α), p x` 的
**应用形态**是 `Exists α (fun (x : α) => p x)`——记法只写那个 lambda，而常量的
第一个参数（域 `α`）在应用里也要占位。`spine::spine_with_notation` 一直有这条
binder 分支，而 `elab.rs::src_spine`（2026-09-21 为「`have h : B ∨ C` 之后
`cases h`」新加的记法分支）**漏了它** ⇒ 只拿到 1 个实参 ⇒ 参数化归纳报「书写类型
需要显式给出 2 个参数」。触发条件很隐蔽：**只有定义体用 `∃` 写**（`lib/Image` 的
`Set.image` 改成 `∃ (x : α), x ∈ A ∧ f x = y` 之后，单元⑧ 的 `cases hy` 整类打红）。
两个函数都声称「记法节点的源像 = target(操作数…)」，**必须同款**。抓住它的是既有
的两条测试：`cases_inside_a_have_block_keeps_every_constructor_field`、
`cases_uses_the_canonical_type_so_notation_prefix_params_do_not_capture_context_names`。

**已知未修（下一刀）**：`judge_infer`（`apply` / `cases` 推断被应用函数的类型）**仍不带**
宇宙参数 —— 它合成的是 `#check fun (α : Sort u) => …`，而 `#check` 片段没有地方声明
`u`。要修得换合成策略（例如把探针包进一个带 `{u}` 的临时 `def` 再取类型），
不是加一个参数能解决的。症状：目标/假设里带宇宙变量时 `apply` 可能报未声明宇宙变量。



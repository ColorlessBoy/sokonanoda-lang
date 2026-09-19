# WO-004 开练习的签名不做类型检查（G-01）

> 台账：`docs/gaps/ledger.jsonl` → `G-01`（`kind=language` · `severity=blocker` · `status=open`）
> 复现件：`docs/gaps/repro/G01-open-exercise-signature.sokonanoda`（11 行，只有一条声明）
> 实测环境：`af737fc` · `sokonanoda 0.58.0`（`./target/release/sokonanoda --version` → `sokonanoda 0.58.0`；
> `scripts/soko version --json` → `cli.source=repo-build`，即本轮所有输出都出自本树的 0.58.0 构建）
> 证据纪律：下面每个「今天」都是本轮**亲手跑出来的输出**；只读到源码而没跑到的判断，一律标「待确认」。

## 用户可见症状 / 最小复现

**复现命令（一条，可直接粘贴）**

```bash
scripts/soko grade docs/gaps/repro/G01-open-exercise-signature.sokonanoda
```

**今天的实际输出**（关键行，0.58.0 实测）

```text
{"human":"exercise open (fill the sorry)","name":"t9","type":"exercise.open"}
# 退出码 0
```

- `scripts/soko query check --file docs/gaps/repro/G01-open-exercise-signature.sokonanoda`
  → `counts.exercise_open=1` · `failed=[]` · `warnings=[]` · `ok:true` · **退出码 0**；
- `scripts/soko query goals --file …`
  → `{"name":"t9","kind":"theorem","status":"open","ty":"3","goal":"3","holes":["t9:0"]}`——
  判卷器还给学习者一个 `⊢ 3` 的目标，仿佛这是一道正常的练习。

同族变体（同一台 0.58.0，全部同款「绿」）：

| 源文本 | 今天的事件流 | 退出码 |
|---|---|---|
| `theorem t9 : 3 := sorry`（`ty` 能 elaborate，但不是 Prop） | `exercise.open`；0 诊断 | 0 |
| `theorem t10 : Bogus := sorry`（签名里是未定义名） | `exercise.open`；0 诊断 | 0 |
| `def d1 : 3 := sorry` | `exercise.open`；0 诊断 | 0 |
| `theorem t20 : 3 := by sorry` | `exercise.open`；0 诊断 | 0 |
| `example : 3 := sorry` | `exercise.open`；0 诊断 | 0 |
| `axiom a1 : 3`（**不在**开练习路径——今天的对照） | `kernel-expected-sort` | 1 |

把**同一条签名**的值位换成真值（走 checked 路径），今天就报错：

| 源文本 | 今天的事件流 | 退出码 |
|---|---|---|
| `theorem t14 : 3 := 3` | `kernel-expected-sort`：`rejected: expected a sort, got: Nat.[]` | 1 |
| `theorem t15 : Bogus := rfl` | `elab-unknown-identifier`：``unknown identifier `Bogus` `` | 1 |
| `theorem t16 : (fun (x : Nat) => x) := fun (x : Nat) => x` | `kernel-expected-sort`：`rejected: expected a sort, got: Pi (x : Nat.[]), Nat.[]` | 1 |
| `example : 3 := 3` | `kernel-expected-sort` | 1 |

一句话：**同一条签名，值位写 `sorry` 就免检，值位写真值就报错**——这正是台账 `today` 与 `expected_lean` 的落差。

### 课程级复现（G-01 的真实代价）

把 `courses/set-theory/units/unit01-sets-membership.sokonanoda` 的 `mem_of_subset`（第 34–36 行）签名做两种变异，
放进一个带 `sokonanoda.toml` + `lib/` 软链的**临时模块根**里判卷（不往仓库里留文件；直接拿 `/tmp` 里的副本判卷会
`import-not-found`，实测它会去找 `/tmp/…/lib/Logic.sokonanoda`）：

| 变异（`mem_of_subset` 的签名） | 值位是 `sorry`（今天） | 同一变异 + 值位是真实证明（今天） |
|---|---|---|
| 无（对照） | exit 0 · `exercise.open`×6 · 诊断 0 | exit 0 · 全部 checked |
| `Set.subset` → `Set.subsets`（引理名拼错） | exit 0 · `exercise.open`×6 · **诊断 0** | **exit 1** · `elab-unknown-identifier` @ 第 6 行 |
| 结论 `Set.mem α a A -> Set.mem α a B` → `Set α`（不是 Prop） | exit 0 · `exercise.open`×6 · **诊断 0** | **exit 1** · `kernel-theorem-not-prop`：`rejected: theorem type must be Prop (sort 0): Pi (α : Sort(1)), …` @ 第 6 行 |

三份文件今天的输出**逐字相同**（`exercise.open`×6 / `decl.checked`×2 / 诊断 0 / exit 0），所以课程线拿不到任何
「签名腐烂」的信号——台账 `blocks` 里「卷 I 全部单元」「任何依赖 golden 计数发现签名腐烂的课程」说的就是这张表。

### 为什么判据必须盯 `failed`/`diagnostic`，不能盯 `exercise_open`

台账 `notes` 已经写明这一点，实测也证实了：坏签名今天照样 `exercise_open +1`。唯一能察觉「签名 elaborate 失败」的
现有信号是 `query goals` 的 `ty: null`（实测 `theorem t10 : Bogus := sorry` → `{"status":"open","ty":null,"goal":"Bogus"}`），
而 `theorem t9 : 3 := sorry` 连这个信号都没有（`ty: "3"` 是内核 pretty-print 成功的产物）。**修复后必须让诊断自己出现**，
不要依赖任何隐式信号。

### 修好后的输出（0.59.0 实测，`target/debug` 构建；修复前的对照见上节）

```text
$ scripts/soko grade docs/gaps/repro/G01-open-exercise-signature.sokonanoda
{"code":"kernel-expected-sort","hint":"这里需要写一个类型（如 Prop、Type、Nat），但你写成了一个普通的项。检查冒号/binder 后面跟的是不是类型。","message":"rejected: expected a sort, got: Nat.[]","span":{...签名 `3` 的字节区间...},"stage":"kernel","type":"diagnostic"}
# 退出码 1；**没有** exercise.open
```

同族变体逐条（0.59.0 实测）：

| 源文本 | 修后的事件流 | 退出码 |
|---|---|---|
| `theorem t9 : 3 := sorry` | `kernel-expected-sort`（span = 签名 `3`）；无 `exercise.open` | 1 |
| `theorem t10 : Bogus := sorry` | `elab-unknown-identifier`（span = `Bogus`，stage `elab`） | 1 |
| `theorem t17 : Nat := sorry` | `kernel-theorem-not-prop`：`rejected: theorem type must be Prop (sort 0): Nat.[]` | 1 |
| `def d1 : 3 := sorry` | `kernel-expected-sort` | 1 |
| `theorem t20 : 3 := by sorry` | 与 `t9` 同码（`by` 路径共用同一处签名检查） | 1 |
| `example : 3 := sorry` | `kernel-expected-sort` | 1 |
| `example : Prop -> Prop := sorry`（合法） | `exercise.open`；诊断 0 | 0 |
| `example : Nat := sorry`（合法） | `exercise.open`；诊断 0 | 0 |

**课程级变异**（把 `courses/set-theory/units/unit01-sets-membership.sokonanoda` 的
`mem_of_subset` 签名做两种变异，放进带 `lib/` 软链的临时模块根判卷）：

| 变异 | 修前 | 修后 |
|---|---|---|
| `Set.subset` → `Set.subsets`（引理名拼错） | exit 0 · `exercise.open`×6 · 0 诊断 | **exit 1** · `elab-unknown-identifier` · `exercise.open`×5 |
| 结论 `Set.mem α a A -> Set.mem α a B` → `Set α`（不是 Prop） | exit 0 · `exercise.open`×6 · 0 诊断 | **exit 1** · `kernel-theorem-not-prop` · `exercise.open`×5 |

两个变异体已入库成 `docs/gaps/repro/G01-course-signature-mutations.sokonanoda`
（自包含，课程目录里不留坏文件）。

**兼容性实测（本 WO 最大的风险，实测而非预测）**：用新旧二进制逐条对拍**全仓
105 个 `.sokonanoda` 文件**（含入门课 65 条 + 卷 I 93 条开放练习、`lib/`、`solutions/`、
`examples/`、`playground.sokonanoda`、repro）：唯一变化的就是本 WO 自己的复现件
（exit 0 → 1），**新增诊断 0 条**；`courses/set-theory/tools/check.py` 仍是
`34 个目标 —— 315 checked · 96 open · 0 个被判负`；两处课程 GOLDEN 未改。

### 期望行为

### 官方 Lean 4 里这段是什么行为

- 台账 `expected_lean`：「`theorem` 的签名必须能 elaborate（`3 : Nat` 不是 Prop ⇒ 报类型错误），签名错误不能因为值位是 sorry 而免检」；
- 硬规则 3（`REQUIREMENTS.md` §2 第 3 条）：「教学语法是真实 Lean 4 的子集，**填完洞的声明放进官方 Lean 依然合法**」。
  今天的行为直接违反它：`theorem t9 : 3 := sorry` 这道题无论学生怎么填，都不可能是一条合法 Lean 声明；
- 本仓**不允许**调用官方工具链（`REQUIREMENTS.md` §2 第 2 条），所以本 WO 不锁定 Lean 的具体报错文本（**待确认**），
  只锁定「签名必须过 elaborate + `theorem` 的类型必须是 Prop」这一语义结论；本仓自己的教学文案已经这样承诺了
  （`crates/front/src/compile/error.rs:248-250`：`theorem 的类型必须是命题（Prop 里的东西）。想定义普通值请用 def。`）。

### 本仓已有的语义承诺（不是本 WO 新发明的）

- 内核**已经**会拒：`crates/kernel/src/infer.rs:281-293` 的 `check_declar_info_v` 先 `ensure_sort_v`（`:288`），
  再对 `Declar::Theorem` 断言 `theorem type must be Prop (sort 0)`（`:292`）——开练习路径只是**从来没走到这里**；
- 前端**已经**会分类、会给教学提示：`error.rs:383-385` 把 `expected a sort…` 映射成 `KernelExpectedSort`，
  `error.rs:391-393` 把 `theorem type must be Prop` 映射成 `KernelTheoremNotProp`，两者都已有中文 hint
  （`error.rs:242-250`）与协议里的稳定 code（`docs/protocol.md:122-127`）；
- 声明种类决定检查强度：`theorem` → `Declar::Theorem`（`crates/front/src/compile/elab.rs:575`，sort + Prop），
  `def`/`example` → `Declar::Definition`（`elab.rs:612`，只要 sort），实测 `example : Nat := 3` → `example.checked`、
  `example : 3 := 3` → `kernel-expected-sort` 与此一致。

### 修好后的可观察契约（这就是验收判据）

| 源文本 | 修好后的事件流 | 退出码 |
|---|---|---|
| `theorem t9 : 3 := sorry` | `diagnostic{stage:kernel, code:kernel-theorem-not-prop}`；**不再**发 `exercise.open` | 1 |
| `theorem t10 : Bogus := sorry` | `diagnostic{stage:elab, code:elab-unknown-identifier}` | 1 |
| `def d1 : 3 := sorry` | `diagnostic{stage:kernel, code:kernel-expected-sort}` | 1 |
| `theorem t20 : 3 := by sorry` | 同 `t9`（`by` 路径与直接值位共用同一处吞错点） | 1 |
| `example : Prop -> Prop := sorry` | `exercise.open`；诊断 0（既有测试 `tests.rs:46-51`、`protocol.rs:288-300` 的锚点） | 0 |
| `example : Nat := sorry` | `exercise.open`；诊断 0（`Nat` 是 sort，合法） | 0 |

诊断的 **span 取签名的源范围**（`Command::{Def,Theorem,Example}.ty` 的 `Expr::span()`；`ty` 是含 binder 的完整 Pi 链——
`crates/front/src/ast.rs:212-236` 没有独立 binders 字段，实测 `query goals` 里 `ty` 也渲染成
`forall (α : Type 0) (A B : Set α), …`），**不要**照抄内核消息里的 span：课程线 `courses/set-theory/AGENTS.md`
已把「内核错误的 span 不可全信」写进判卷纪律（台账 G-15）。

### 「签名不过」时这条声明是什么状态（决策点，建议照既有先例走）

建议：**与值位 elaborate 失败完全同罪——声明是 Failed，不再报 `exercise.open`**。依据是既有实测先例：
`theorem t24 : A := by apply nosuchlemma` 后面再跟 `sorry`，今天只出 `elab-tactic-failed` 一条诊断、**没有**
`exercise.open`，`query check` 给 `exercise_open:0` + `failed:[{code:…}]`。也就是说本仓既有语义就是
「elaborate 不过 ⇒ 不是可做的练习，`sorry` 救不回来」，签名只是这条规则的唯一漏洞。
（对照 `protocol.md:83-91` 的 `redundant-sorry`：那是**值位已经证完**、声明仍留在 `exercise.open`——两者不冲突，
别把本 WO 做成 warning：warning 不改退出码，课程侧就永远发现不了腐烂。）

### 本教学子集的边界（哪些不做）

- 只做「签名能否 elaborate + sort/Prop 判定」，**不做**签名之外的隐式参数推断、不做自动补全/建议；
- 不引入任何新语法、不改 parser 白名单（硬规则 4）；
- 不检查 `-- soko:hint` 与题目是否匹配（那是课程侧纪律，不是语言判定）；
- 不改变合法开放练习的任何既有行为（洞、目标、hint、`sub_goals`、`refine_template`、LSP 视图一律不动）。

## 范围

### front：同一个 bug 的三份拷贝（三处都在吞错）

| 位置 | 路径的样子 | 今天的处理 |
|---|---|---|
| `crates/front/src/compile/check/walk.rs:246-257` | `def` 的 open 路径 | `elab_expr(… ty …)` 后直接 `.ok()`，`Err` 被丢掉（`declared_ty: None`），随即 `push(PendingOp::OpenExercise{…})`（`:258-282`）并 `return` |
| `crates/front/src/compile/check/walk.rs:436-447` | `theorem` 的 open 路径 | 同上（`:448-472`），另外 `:419-434` 还有 generic fallback：spine 走查分解不了但有洞 ⇒ 照样算开放练习 |
| `crates/front/src/compile/check/walk.rs:705-716` | `example` 的 open 路径 | 同上（`:717-…`） |

三处的共同形状是：`open_goal`（`crates/front/src/compile/goals.rs:236-248`）判「有洞 ⇒ 是开放练习」，
然后**只**把签名转成内核表达式用来渲染 `ty_text`，既不报它的 elaborate 失败，也从不问内核「它是不是一个类型/命题」。
进到内核阶段后，`PendingOp::OpenExercise`（`crates/front/src/compile/check/mod.rs:43-68`）只做两件事：
发 `CheckEvent::ExerciseOpen`（`kernel_phase.rs:121`）与造 `DeclState{status: Open, error: None}`（`:152-168`），
`declared_ty` 只用来渲染（`:146-151`）。**没有任何一处会失败**——这就是 0 诊断的来源。

### 内核：什么都不用改，接口已经够用（预期否）

- `crates/kernel/src/infer.rs:288/292`：`ensure_sort_v` + theorem 的 Prop 断言，已在 checked 路径生效；
- `crates/kernel/src/tc.rs:75-90`：`check_declar` / `check_declar_at` 是公开入口；
  `crates/kernel/src/util.rs:669-687`：`Env::try_check_declar_at(&Declar, EnvLimit)` 把内核 panic 收成
  `CheckError::Rejected(msg)`——**前端已经在用**：`kernel_phase.rs:128-145` 用它给「多余的 sorry」做终审；
- `crates/kernel/src/tc.rs:289-294`：`TypeChecker::is_proposition(&mut self, e) -> bool` 是**公开**的 Prop 判定，
  前端也能拿到 `TypeChecker`（`Env::with_tc`，`util.rs:703-709`，`kernel_phase.rs:147-150` 已在用）；
- 注意顺序：`is_prop_type` 对**不是类型**的值会 panic（`crates/kernel/src/conv.rs:655-663`：
  `expected a sort in conversion, got: …`），所以必须**先**确认「是个类型」，**再**问「是不是 Prop」。

**结论：是否动内核 = 否**（预期）。内核是冻结快照（硬规则 1），本 WO 不允许改 `crates/kernel`；
如果实现时发现公开 API 不够（**待确认**项，见下），正确动作是把发现写回本 WO 并升级评审，而不是顺手改内核。

### 实现骨架（首选路线，front-only）

1. **不再吞错**：三处 `elab_expr(...).ok()` 改成 `?`/`match`，把 `Err` 走既有的失败通道
   （照 `walk.rs:190-202` 的 `lower_value` 错误分支：`self.out.push_error(idx, e)` +
   `failed_state(kind, name, span, e, idx)`），然后 `return`；
2. **sort 终审**：用探针声明问内核「这条签名是不是一个类型」——`build_axiom`（`elab.rs:623-642`）是现成的
   **无值**声明构造器，造 `_soko_sig_probe` 之类的临时名，再 `env.try_check_declar_at(&probe, EnvLimit::ByIndex(env_before))`；
   失败消息原样进 `refine_kernel_kind` ⇒ `kernel-expected-sort`（探针**不入环境**，先例：
   `walk.rs:999-1028` 的 `build_redundant_probes`、`kernel_phase.rs:124-127` 的 `env_before` 说明）；
3. **Prop 终审**（只对 `theorem`，即 `PendingOp::OpenExercise.kind == DeclKind::Theorem`）：
   `env.with_tc(limit, |tc| tc.is_proposition(ty_kernel))`，`false` ⇒ 报 `kernel-theorem-not-prop`，
   消息形状与 checked 路径保持一致（内核那句 `theorem type must be Prop (sort 0): …` 由 `refine_kernel_kind`
   分类，`error.rs:391-393`）；外面套 `quiet_catch`（先例 `kernel_phase.rs:147`）以防探针内部 panic 外泄；
4. 诊断 span 用第 2 步的 `ty` 的 AST span（上文「可观察契约」）。

**待确认 → 实现时已全部实测确认**：

- ✅ `Declar::Axiom` 探针在 `try_check_declar_at(ByIndex(env_before))` 下给出的消息与 checked
  路径**同族**：`theorem t : 3 := sorry` 与 `theorem t : 3 := 3` 都是
  `rejected: expected a sort, got: Nat.[]` ⇒ 同一个 `kernel-expected-sort`；探针的
  消息原样进 `refine_kernel_kind`，span 自己换成 `ty.span()`；
- ✅ `is_proposition` 对「已是类型但不是 Prop」返回 `false` 而不 panic（`Nat`、`Prop -> Type`
  实测都是 `kernel-theorem-not-prop`）；**顺序仍然不能反**——非类型会在
  `is_prop_type → level_of_type` 处 panic，所以第 1 步（探针）先挡；
- ✅ `by` 路径不需要第四个修点：`theorem t : 3 := by sorry` 与直接值位**同码同 stage**
  （`kernel-expected-sort`），已进 front 单测与 CLI e2e；
- 额外发现（不在原 WO 里）：签名原来用**空宇宙表** elaborate，`{u}` 签名的 `ty_text`
  渲染不出来；现在与 checked 路径共用 `make_univ_map`，合法开放练习的 `ty_text`
  因此更准（无诊断变化）。

## 不做的事

- **不修 G-04**（没有 notation/infix）：本 WO 里 `theorem t : 1 = 1 := sorry` 仍然是 `parse` 阶段的
  `unexpected-token`（实测，`expected `=>`, found =`）——签名检查管不到解析层，别顺手加 `=`；
- **不修 G-10**（`query check` 对解析失败假绿）：本 WO 不动 `query check` 的任何路径；
- **不碰 `sorry` 语义**：`redundant-sorry` 探针、洞/目标视图、`exercise.open` 对**合法**练习的行为一律不变
  （`crates/front/src/compile/tests.rs:4839` 与 `crates/cli/tests/protocol.rs:367-372` 是钉住它们的锚点）；
- **不给值位加任何检查**：值位是不是好证明由内核在 checked 路径判，本 WO 只补签名那一半；
- **不改 G-15 的归因**：只在新增诊断上用 AST span 规避，不去重做内核错误的 span 映射；
- **不动课程内容**（除下面「兼容策略」兜底出来的真腐烂）；
- **不改内核**、**不新增语法**、**不改 `docs/protocol.md` 之外的对外契约字段**（`kernel-theorem-not-prop`
  本来就是既有 code，`protocol.md:126` 已登记）。

## 验收（三层）

### front 单测（`crates/front/src/compile/tests.rs`，用既有 `compile_fol` 夹具）

1. `theorem t : 3 := sorry` ⇒ `errors` 恰好 1 条、`code()=="kernel-theorem-not-prop"`、span 落在签名上、
   `events` 里**没有** `ExerciseOpen`；这条同时钉死「不再假绿」；
2. `theorem t : Bogus := sorry` ⇒ `elab-unknown-identifier`（`declared_ty` 的 `Err` 必须上报）；
3. `def d : 3 := sorry` 与 `example : 3 := sorry` ⇒ `kernel-expected-sort`（三处吞错点都要覆盖）；
4. `theorem t : 3 := by sorry` ⇒ 与 (1) 同码（`by` 路径覆盖）；
5. **防修过头**：`example : Prop -> Prop := sorry`（`tests.rs:46-51`）与 `example : Nat := sorry` 必须仍是
   `errors == []` + `ExerciseOpen`；`tests.rs:897-915` 的 `open_exercise_does_not_pollute_env` 语义不变
   （合法开放练习照样不进环境，也不新增诊断）。

### CLI e2e（`crates/cli/tests/`）

- `crates/cli/tests/protocol.rs`：新增一条「坏签名」用例（stdin `theorem t : 3 := sorry`）断言
  **exit 1** + `diagnostic.code == "kernel-theorem-not-prop"` + **没有** `exercise.open`；
  与既有 `protocol.rs:288-300`（`example : Prop -> Prop := sorry` ⇒ exit 0 + `exercise.open` + 无诊断）
  构成**一组通过/失败边界对照**，防止以后有人把两边一起放松；
- `crates/cli/tests/query.rs`：`grade --json` 事件流与 `query check` 的计数一致性（既有契约）要在
  `failed=1` 的新形状上仍成立；
- 建议补一条 `--json` 的 span 断言（诊断 span 的字节区间 == 签名区间），把 G-15 那种漂移挡在门外。

### 课程用例

- **正向（不许翻红）**：`courses/set-theory/units/unit01-sets-membership.sokonanoda` 的
  `theorem mem_of_subset`（第 34-36 行，配 `units/solutions/unit01-solution.sokonanoda:6-8` 的 checked 孪生）
  以及其余 92 条开放练习：修后仍 `exercise.open`、0 诊断，
  `python3 courses/set-theory/tools/check.py` 仍打印 `34 个目标 —— 296 checked · 93 open · 0 个被判负`（今天实测值）；
- **反向（必须变红）**：把 `mem_of_subset` 的签名改成 `Set.subsets`（引理名拼错）或把结论改成 `Set α`（不是 Prop），
  修后必须 exit 1 + 上表那两个 code（本 WO 已在「值位有真实证明」的对照文件上实测过这两个报错形状）；
  这两个变异体**不要**留在课程目录里，做成 front 夹具或 repro 的兄弟文件。

### 影响面与兼容策略（G-01 专属：这是本 WO 最大的风险）

**先给结论：修好后对既有课程语料的预期新增诊断 = 0 条。** 本轮逐条核对过（脚本比对「声明名 + 签名文本」，忽略值位）：

| 语料 | 开放练习 | 有「同名 + 同签名」的 checked 孪生 | 签名引用更早的开放练习 |
|---|---|---|---|
| `courses/set-theory/units/unit01..12` ↔ `units/solutions/*` | 93 | **93**（12/12 单元的 `import` 头也逐字一致） | **0** |
| `course/unit1..11` ↔ `course/solutions/*` | 65 | **65** | **0** |
| 合计 | **158** | **158** | **0** |

也就是说：这 158 条练习的签名在「值位有真实证明」时**已经被内核接受**（课程 checker 今天报 0 判负），
且没有任何一条签名依赖「更早的开放练习」——所以「开放练习不进环境」不会因为本修复冒出 `elab-unknown-identifier`。
既有 front/e2e 测试里的开放练习签名也都是 `Prop -> Prop`（`tests.rs:47`、`protocol.rs:290`），同样安全。

**但这是预测，不是保证**：签名的 elaborate 环境 = 单元文件自己的 import 头 + 文件里更早的 checked 声明，
与解答文件仍可能有细微差别（例如某个单元把 demo 删了）。所以实现时**必须**跑：

1. `python3 courses/set-theory/tools/check.py` → 仍是 `296 checked · 93 open · 0 个被判负`；
2. 若任一单元出现新诊断：**先判性质**——签名确实烂 ⇒ 修课程（同轮，属于本 WO 的附带修复，要写进 CHANGELOG）；
   签名只是超出本子集边界（例如依赖尚未支持的写法）⇒ 记新缺口，**不要**放宽签名检查去迁就它。

**事件计数与 golden**：

- 合法语料：`decl.checked` / `exercise.open` / `expr.reduced` **三列都不变**（预期），因此
  `crates/cli/tests/course.rs:86` 的 `GOLDEN`（11 行 `(checked, open, reduced)`，open 合计 65）与
  `crates/cli/tests/course_status.rs:68` 的 `GOLDEN`（11 行 `(checked, open, failed, reduced)`，`failed` 全 0）
  **预期不用改**；但**只要**实测有任何一行变了，这两个文件必须**同轮**一起改（双 GOLDEN，见 AGENTS.md 收尾义务）；
- 坏签名：从「`exercise.open` +1」变成「`diagnostic` +1、`exercise_open` −1、`failed` +1」，这正是课程线想要的信号；
  注意 G-01 自己的 repro 是 `.sokonanoda` 型：`scripts/gap.py` 的判据是「干净判卷 **且** 有 `decl.checked`」
  （`scripts/gap.py:71-78`），本文件两条都不满足，所以**修前修后 `gap.py check` 都是「仍有失败」**——
  它不会替你提醒（那只对 `.sh` 复现成立），关账必须靠上面三层验收（今天实测：`G-01 open sokonanoda 仍有失败`）。

## 文档同步清单

- `docs/protocol.md`：`:60` 的 `exercise.open` 语义边界（签名不过 ⇒ 不是 open）、`:122-127` 的 `kernel-theorem-not-prop`
  适用面（现在也会出现在开放练习上）、`:83-91` 的 `redundant-sorry` 对照段各补一句 —— **必须同轮**；
- `skills/sokonanoda-teacher/SKILL.md`：判卷判据从「只看 `exercise.open`」改成「`exercise.open` 的签名已受检，
  坏签名会以 diagnostic 出现」——**必须同轮**（教学主循环直接吃这条）；
- `skills/sokonanoda-dev/SKILL.md`：TDD 三层里补上「命名/签名类 bug 的三层回归范例」；
- `.agents/skills/`：只确认入口仍指向 `skills/`，**不动**（`crates/cli/tests/dsh.rs` 守着漂移）；
- `editor/vscode/`：诊断从无到有属于用户可见反馈变化 ⇒ `CHANGELOG.md` / `README.md` / `package.json` 版本
  按 `docs/vscode-dev-guide.md` 同轮处理；
- `AGENTS.md`（硬规则速记第 3 条下补一句「签名也受检」）、`docs/HANDOVER.md`、`STATUS.md`（收尾）；
- `REQUIREMENTS.md` §9：追加一条「2026-xx-xx：开放练习的签名纳入类型检查（G-01）——修的是违反硬规则 3 的行为」；
- `docs/design/teaching-project.md` §6.4：G-01 是「不看 `exercise_open` 计数」这条判据的样板，可把修好后的
  repro 行为写进去当例子；
- `docs/gaps/ledger.jsonl`：由 `close` 写 `status`/`fixed_in`（不要手改）。

## 门禁

```bash
scripts/soko gate                       # = fmt + clippy + test + playground 锚点
cargo test --workspace --locked         # 全量
python3 courses/set-theory/tools/check.py   # 课程侧：296 checked · 93 open · 0 判负
python3 scripts/gap.py check            # 台账自检（G-01 这行修前修后都应是「仍有失败」，别误判成回退）
```

- 只 fmt 教学 crates（`cargo fmt -p sokonanoda-front -p sokonanoda-cli -p sokonanoda-lsp -- --check`）；
  **禁止 `cargo fmt --all`**（会重排冻结内核）；
- `gate` 用**运行中二进制**的内嵌编译器做锚点：先 `scripts/soko version --json` 确认 `cli.source=repo-build`
  且版本 == `Cargo.toml`（本机实测 `cli.marker` 指的是**未命中**的缓存 0.55.0，实际跑的是 repo-build 0.58.0；
  若 gate 因版本不一致 exit 3，先 `scripts/soko update` 或用 `cargo run -q -p sokonanoda-cli --bin sokonanoda --`）。

## 关账

```bash
python3 scripts/gap.py close G-01 --version 0.59.0
```

- 版本：`0.58.0` → `0.59.0`（两处版本号 + CHANGELOG，见 `docs/RELEASE.md`）；
- 关账前必须**手工**确认（`close` 对本条不做拒绝检查，见上文）：repro 现在 exit 1、打出一条 diagnostic、
  `query check` 的 `failed` 非空；三门禁全绿；
- 关账后把「修好后的输出」贴进本 WO 的「用户可见症状」节（保留修复前的对照），并让台账的 `notes`
  指向本文件；
- 顺带把 `docs/gaps/repro/G01-open-exercise-signature.sokonanoda` 的注释补一行「修后预期」，
  免得下一个人以为它坏了。

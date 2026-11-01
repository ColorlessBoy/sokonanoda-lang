# P2-12 **S4b 第一刀**：`A ≼ B`（`Set.Le` 记法）为什么在内核 `elab_app` 里 `Unsolved`

> **只读定根因**（2026-11-01，只读子代理）。上游：`reports/2026-11-01-p212-s4b-census.md`（普查：`Set.Le` 一族 **121** 次
> `fallback_old_ok`）、`reports/2026-10-29-strict-replan.md` §4 的 `P2-12 S4b…n` / `S2` / `S3` 行。
> 本文件只回答一件事：**内核为什么解不出、老路凭什么接住、怎么修**。**没有改任何被跟踪的文件**（见 §6.1）。

## 0. 一句话

**卡住的不是 `β` 的类型，也不是 `Set.Le` 的宇宙层，而是记法自己补出来的前导形参 —— 它们的值由一个「元素类型未定」的操作数
（`∅`）决定，这在合一上是个**闭环**（`?β ≟ ∅` 的元素类型 `?δ`，两个都没人定）。内核 `elab_app` 要求 `Solved` = 项里没有
残留元变量 ⇒ 报 `Unsolved{Implicit{index}}`；老路（`elab_notation` 的 `notation_prefix_args` + E19 待定档）能接住，
**靠的是一条显式的「同形兄弟借用」启发式**（`fill_pending_by_shape`：两层域都是 `Type` ⇒ 把已解兄弟的值抄过来）。
把这条启发式关掉/让它借不到（`∅ ≼ ∅`、或两层域不同形），**老路也一样失败** ⇒ 这是实测，不是推断。**

## 1. 构建身份与读数来源

| 项 | 值 |
|---|---|
| 仓库 / 分支 | `/Users/penglingwei/Documents/lean/sokonanoda/sokonanoda-lang` · `strict/2026-10-29-kernel-app` |
| HEAD | `bd829f25` |
| **二进制（唯一读数来源）** | `target/release/sokonanoda`，sha256 `75dc6b34301eeba690adf5cb54fa1195bfa117248cc8400ca7bc2afc55d76238`（短 **`75dc6b34301eeba6`**），mtime 11-01 12:30 |
| 固化 | `cp target/release/sokonanoda /tmp/s4b-75dc6b34301eeba6`，此后**只用这份副本**；全程**未跑 `cargo`** |
| 工作树状态 | 并行 agent 的 **S3 未提交改动**（`elab.rs` / `kernel_app.rs` / `kernel_app_census.rs` / `kernel_app_notation.rs` / `mod.rs` / `tests/kernel_app_keys.rs` + 新增 `kernel_app_fallback.rs` 等）；这些文件的 mtime 12:18–12:28 < 二进制 12:30 ⇒ **本报告的源码引用就是这份二进制对应的树**。`crates/kernel/src/elab_app.rs` mtime 11:16、**不在 `git diff` 里** ⇒ 与 HEAD 一致 |
| 命令 | `scripts/strict/fallback-one.sh /tmp/s4b-75dc6b34301eeba6 <file>`（脚本自带：独立 `SOKONANODA_CACHE_DIR`、`-u SOKO_STRICT_AT`、关两种产物、`SOKO_STRICT_KERNEL_APP=1`、`SOKO_JUDGE_STATS=1`） |
| 位置日志 | 外层 `SOKO_KERNEL_APP_LOG=/tmp/s4b-logs/<name>.log`（`kernel_app_census::record` 追加 `<tag> <why> <head> <line>:<col>`） |
| 夹具 / 原始日志 | `/tmp/s4b-fix/min{1,2,3,4,5,6,7,8,9}.sokonanoda`、`/tmp/s4b-logs/*.log`、`/tmp/s4b-*.json`（`/tmp` 易失） |

**基线复现（measured）**——两个真文件都用本次副本逐字复现了普查数字：

| 文件 | `fallback_old_ok_notation_unsolved` | 普查值 | `fallback_old_ok_notation_unsupported_input_contains_a_meta` | `err` | rc / diags |
|---|---:|---:|---:|---:|---|
| `courses/set-theory/units/solutions/I.8/unit19-solution.sokonanoda` | **88** | 88 ✓ | 2 | 9 | 0 / 0 |
| `courses/set-theory/units/I.8/unit19-cardinal-le.sokonanoda` | **31** | 31 ✓ | 0 | 7 | 0 / 0 |

（`unit19-cardinal-le` 的日志里同时出现 `1615:43` / `1671:46` / `1679:40`，即普查 §5.2 说的 **+1530** 前缀坐标
——本轮独立复现到同一个偏移 ✓；两个文件都是 **rc=0、0 条诊断**，即「老路接住、判卷通过」。）

## Q1 复现件（measured）

**最小夹具 `/tmp/s4b-fix/min3.sokonanoda`（6 行、零 import、零课程依赖）**：

```sokonanoda
def Set (α : Type) : Type := α → Prop
def Set.Le (α β : Type) (A : Set α) (B : Set β) : Prop := True
infix:50 " ≼ " => Set.Le
def Set.empty (α : Type) : Set α := fun (x : α) => False
notation "∅" => Set.empty
theorem t3 (α : Type) (A : Set α) (h : A ≼ ∅) : True := True.intro
```

```bash
cd /tmp/s4b-fix && SOKO_KERNEL_APP_LOG=/tmp/s4b-logs/min3.log \
  scripts/strict/fallback-one.sh /tmp/s4b-75dc6b34301eeba6 min3.sokonanoda
```

读数（measured）：

```json
{"file":"min3.sokonanoda","rc":0,"diags":0,
 "KERNEL_APP.tried":3,"KERNEL_APP.solved":3,"KERNEL_APP.fallback_unsolved":2,
 "KERNEL_APP.fallback_old_ok_notation_unsolved":2,
 "KERNEL_APP.fallback_old_ok_notation_unsupported_input_contains_a_meta":4,
 "KERNEL_APP.fallback_unsupported_input_contains_a_meta":4}
```

日志（`/tmp/s4b-logs/min3.log`，同 span 同一处源码被多次 elaborate）：

```
ok notation_unsupported_input_contains_a_meta Set.Le 6:40
ok notation_unsupported_input_contains_a_meta Set.Le 6:40
ok notation_unsupported_input_contains_a_meta Set.Le 6:40
ok notation_unsupported_input_contains_a_meta Set.Le 6:40
ok notation_unsolved Set.Le 6:40
ok notation_unsolved Set.Le 6:40
```

⇒ **`fallback_old_ok_notation_unsolved ≥ 1` 复现**（6 行、自足，不需要 `courses/`）。`6:40` 就是 `A ≼ ∅` 的起点。

**同一族的最小变体（全部 measured，判据见 Q2）**：

| 夹具 | 关键写法 | `fallback_old_ok_notation_unsolved` | 老路结局 |
|---|---|---:|---|
| `min3` | `(α : Type) (A : Set α) (h : A ≼ ∅)` | **2** | `ok`（复现） |
| `min1` / `min4` / `min5` | `A ≼ A` / `A ≼ B`（两个操作数都是变量） | 0 | 内核直接 `Solved` |
| `min2` | `∅ ≼ A`（`∅` 在左） | **1** | `ok`（复现 `43:43` 那一形状） |
| **`min6`** | `A ≼ Set.empty α`（`β` 被**显式**定住） | **0** | 内核 `Solved` |
| **`min7`** | 前两层域不同形（`(α : Prop) (β : Type)`）+ `True ⊑ ∅` | 1 | **`err`（老路也失败，rc=1）** |
| **`min9`** | `∅ ≼ ∅`（两个前导位都没人定） | 1 | **`err`（老路也失败，rc=1）** |

## Q2 根因（measured）

### 2.1 内核停在哪个位：`Implicit{index}`，`β`（或 `α`）

`UnsolvedSite` 有四类（`crates/kernel/src/elab_app.rs:71-80`）：`Implicit{index}` / `Level{param}` / `External` / `Blocked{arg}`。
S1 之后内核只报下标/种类。**不需要探针**——现成开关 `SOKO_STRICT_NO_FALLBACK=1`（S3）会把内核结局渲染成人能读的诊断，
它逐位点名（`kernel_app_fallback::finish` → `kernel_app_report::report`）。对 `unit19-solution` 整份文件跑：

```bash
SOKO_STRICT_KERNEL_APP=1 SOKO_STRICT_NO_FALLBACK=1 <bin> grade --dialect strict <file> --json
```

得 **3 条诊断**，正好是普查的 **3 个真实站点**（measured）：

| 真实站点 | 源码 | 诊断点名的位 |
|---|---|---|
| `43:43` | `theorem empty_le (α : Type) (A : Set α) : ∅ ≼ A` | **`α` 由实参和期望类型都定不下来** |
| `59:46` | `theorem le_empty (α : Type) (A : Set α) (h : A ≼ ∅)` | **`β` 由实参和期望类型都定不下来** |
| `65:56` | `theorem subset_of_le_empty (α : Type) (A : Set α) (h : A ≼ ∅)` | **`β` 由实参和期望类型都定不下来** |

（原文：`` `Set.Le` 的签名 `(α : Type) -> (β : Type) -> (A : Set α) -> (B : Set β) -> Prop` 里，前 2 个形参是**隐式**的
（按位置依次是 `α`、`β`）——这里**至少有一个没解出来**；原因：`β` 由实参和期望类型都定不下来。``
——「前 2 个」= `kernel_app_notation::missing_prefix` 算出的 `missing = 2`，**与内核收到的 `implicit_prefix` 一致** ✓。
`unit19-cardinal-le` 的三处（`85:43` / `141:46` / `149:56`）是同一组形状。）

**所以：不是 `β` 的类型不对，不是 `A`/`B` 的类型不对，也不是宇宙层（`Set.Le` 没有 uparam，`Level` 一次都没报）。
是「记法替用户补的那两个前导形参」有一位**根本没值**。**

### 2.2 为什么没值：闭环

`Set.Le (α β : Type) (A : Set α) (B : Set β)`，记法 `A ≼ B` 只给 2 个操作数 ⇒ `missing = 2` ⇒ `α β` 当成隐式。
以 `h : A ≼ ∅` 为例（`A : Set α`，`∅ : Set ?δ`）：

* `?α` 由 `A : Set α` 定住（`?α := α`）—— 这一位没问题；
* `?β` 只能来自 `B` 的类型，而 `B = ∅`，`∅` 自己的元素类型 `?δ` 又只能来自 `?β` ⇒ **`?β ≟ ?δ`，两个都没人定**。

前端 `run_once` 第一趟**无期望地** elaborate 每个操作数（`kernel_app.rs:400`），`∅` 无期望解不出来 ⇒ 推迟成 `None`
（`kernel_app.rs:404-408`）⇒ 内核把它当**推迟位**造洞（`elab_app.rs:288-300`）；洞的域是 `Set ?β`，**带本会话元变量** ⇒
`app_need_args` 判 `blocked` 而**不是** `domains`（`elab_app_holes.rs:103-112`）；`domains.is_empty()` ⇒ 直接
`return AppOutcome::Unsolved { sites }`（`elab_app_holes.rs:113-118`）而不是 `NeedArgs`（= 连"下一轮再试"都没有）。

⇒ `finish` 把 `Unsolved` 计成 `Count::Unsolved`（`kernel_app_fallback.rs:107`）⇒
`pending_notation("unsolved", …)`（`kernel_app_notation.rs:124`）⇒ **`fallback_old_ok_notation_unsolved`**。

**决定性反证（measured）**：`min6` 把同一个 `∅` 换成 `Set.empty α`（`β` 被显式定住）⇒ 内核 **`Solved`、零回落**。
⇒ 内核的合一没问题；**唯一变量就是「前导形参有没有值」**。

### 2.3 老路凭什么接住：`fill_pending_by_shape` 的「同形兄弟借用」

老路 `notation_prefix_args`（`elab.rs:2802-2890`）先按 ①操作数类型 / ②期望类型解前导参数；解不出时走 **E19 待定档**
（`SOKO_NOTATION_METAVAR`，2026-10-01 起**默认开**）：
`solve_prefix_args_impl(..., allow_pending = true)` 把没解出的位记成 `None`，最后调
**`implicit::fill_pending_by_shape`**（`crates/front/src/compile/implicit.rs:599-634`）：
对每个仍是 `None` 的位 `i`，找一个**已解出**的兄弟 `j`，**两层域同形**（`spine::same_shape`，忽略 span）就把 `j` 的值抄给 `i`。

`Set.Le` 的 `α` 与 `β` 域**都是 `Type`** ⇒ 同形 ⇒ **`β := α`（抄已解出的 `α`）**。这条规则是**文档化的设计**
（`implicit.rs:585-593` 的注释，举例正是 `Set.Equiv ∅ {b}` 读作 `Set.Equiv β β ∅ {b}`），不是意外。

**三条独立实测（改一个变量，其余不动）**：

| 变量 | 夹具 | 老路能否借 | 结果 |
|---|---|---|---|
| 正常（`A ≼ ∅`，`α` 已解、`β` 待定、域同形） | `min3` | 能（`β := α`） | `ok` ✓ |
| **两个前导位都待定**（`∅ ≼ ∅`，没有已解兄弟） | **`min9`** | 不能 | **`err`，rc=1** |
| **两层域不同形**（`(α : Prop) (β : Type)`，`same_shape` 判假） | **`min7`** | 不能 | **`err`，rc=1** |
| 关掉待定档（`SOKO_NOTATION_METAVAR=0`）跑 `min3` | `min3` | 不能 | **`err`，rc=1** |

（`min9` 是最干净的一条：**关系形状一个字没改**，只把第一个操作数也换成 `∅`，老路立刻接不住。）

⇒ **结论（measured）：「老路能接住」= 一条同形兄弟借用的启发式猜出了 `β`，与内核的失败是同一件事的两面，不是内核漏了什么。**

## Q3 Lean 4（设计依据）

> ⚠ **版本不符，先记**：任务指定 v4.35.0 `eb6cafb`，但本机 `~/Documents/lean/lean4` 检出的是
> **`v4.12.0-rc1-5351-gd0493e4c1e`**（`git describe --tags`），且**没有** `eb6cafb` 这个对象（`git log eb6cafb` 失败，
> 仓库非 shallow、54562 个提交、tag 最新只到 `v4.9.1`）。下面的 **file:line 是 v4.12.0-rc1 的**，v4.35.0 里可能漂移。
> （纪律允许：只读源码，不跑 Lean 工具链。）

### 3.1 `Elab/App.lean`：`elabAppArgs` **不会**因为隐式位解不出而失败

* 隐式位只造元变量：`addImplicitArg`（`Elab/App.lean:572-588`）`mkFreshExprMVar argType`，没有"必须解出"的分支。
* `finalize`（`Elab/App.lean:410-459`）只做三件事：把隐式位的 mvar 登记进错误上下文
  （`:416-418` `registerMVarErrorImplicitArgInfo`）、试 `synthesizeAppInstMVars`、把期望类型 `isDefEq` 一次
  （`:455-457`，**失败不致命**）。**它不报错、不要求解出，项带着未赋值元变量交出去。**
* 真正的报错在**声明末尾**：`MVarErrorInfo.logError`（`Elab/Term/TermElabM.lean:855-861`）对
  `MVarErrorKind.implicitArg` 印 **"don't know how to synthesize implicit argument"** + 整个应用 + goal。

### 3.2 `Elab/Extra.lean`：`binrel%` 干脆**假设齐次**

* `elabBinRelCore`（`Elab/Extra.lean:497`）先用 `analyze tree none`（`:534`）**不要期望类型地**算左右两边的类型树，
  取 `hasUncomparable / max?`；不足时走兜底：两边都 elaborate，**`ensureHasType lhsType rhs`（`:543`）把两边逼成同一个类型**，
  再 `elabAppArgs f ... [lhs, rhs]`（`:544`）；够时用 `applyCoe tree maxType` 做**联合** coercion（`:551`）。
* 设计前提写在 `:429`：**"We assume `binrel%` elaborator is only used with homogeneous predicates."**
  ⇒ Lean 里二元关系记法的标准形状是**齐次**的（两边同一个类型），正是为了避开"关系自己的类型形参没人定"。

### 3.3 机制差异（我们的 `A ≼ B` 在 Lean 里会怎样）

| 环节 | Lean 4 | 我们（内核路径） | 我们（老路） |
|---|---|---|---|
| 记法展开 | `notation`/`macro` 是**纯宏**，`a ≼ b` ⇒ `Set.Le a b`；**不会**替用户补前导显式形参 | `missing_prefix` **补前 `missing` 个 binder**（`kernel_app_notation.rs:42-50`），这是本语言的**扩展** | 同左 |
| 关系形参求解 | `elabAppArgs` 逐位 `isDefEq`，解不出就**留着元变量**（`App.lean:572-588`、`:410-459`） | 逐位 `isDefEq`（`elab_app.rs:265-314`），末尾要求**零残留元变量**，否则 `Unsolved{sites}`（`:328-356`） | 先**文本/结构**地解前导参数，解不出走**同形兄弟借用**（`implicit.rs:599-634`） |
| 期望类型 | `binrel%` 明确**不用**期望类型算操作数类型（`Extra.lean:489-491`）；`propagateExpectedType` 遇 `Prop` **跳过**（`App.lean:393-394`） | 第一个显式位前传播一次、`Prop` 跳过（`elab_app.rs:451-495`），失败不致命 | 拿期望类型做第 ② 条线索 |
| 齐次要求 | **有**（`Extra.lean:429`） | 无（`Set.Le α β` 是异构的） | 无 |
| 解不出的最终处置 | 声明末尾报 "don't know how to synthesize implicit argument"（`TermElabM.lean:855-861`） | `Unsolved` ⇒ 回落老路（今天） | **同形猜测，接住** |

**一句话**：Lean **没有**任何机制会"接住" `A ≼ ∅` 这种形状——它要么假设齐次（`binrel%`），要么报
"don't know how to synthesize implicit argument"。**我们的老路接住它，是 Lean 没有的一条启发式**；
内核路径的拒绝反而更接近 Lean 的语义。⇒ 修法不应该是"让内核接受"，只能是**"在内核之前把题出完整"**。

## Q4 修法候选（design only）

### 候选 1（**前端**，推荐）：把记法要补的前导参数在**调用内核之前物化**

* **改哪**：`crates/front/src/compile/kernel_app_notation.rs` 的 `try_notation`（+ 可能需要给
  `kernel_app::run` 加一个 `prefix: &[Expr]` 形参，把 `run` 内部的 `postpone::take_hint` 之后那段重做一次；
  `run` 已经持有 `hint` 局部量，重试不会把提示弄丢）。**核算**：`missing_prefix` 已知 `missing`；调用现成的
  `elab::notation_prefix_args`（含 E19 待定档 + `fill_pending_by_shape`）拿到 `Vec<Expr>`（**源级 AST**）。
* **怎么做**：仅当内核回 `Unsolved` 且 **sites 全是 `Implicit{index}` 且 `index < missing`** 时重试一次：
  把解出来的 `missing` 个前导值**前插**到 `operands` 前面，`implicit_prefix` 传 **0**（于是内核把它们当普通**显式**位，
  逐位 elaborate：`app_implicit(prefix=0, Default)` ⇒ `false`，`elab_app.rs:167-169`）。
* **为什么能解出**：`?β` 不再是"没人定的元变量"，而是一个**具体的源级项**；`∅` 的推迟域 `Set ?β` 随之变成
  `Set α`（无元变量）⇒ `app_need_args` 走 `domains` 而不是 `blocked` ⇒ `NeedArgs` ⇒ 第二轮把 `∅` 对着 `Set α`
  elaborate ⇒ `Solved`。与 **`min6`（`A ≼ Set.empty α` ⇒ 内核 `Solved`）** 是同一条已经跑通的路径。
* **风险**：
  1. **语义不变、出项者变**：借用的还是同一个 `fill_pending_by_shape`（不是新猜法），但**项由内核而不是老路产出** ⇒
     开关**开**的那对语料**允许变**，每行差异要逐条归因；开关**关**的 714 行**不经过这段**（`effective()` 第一行就返回）
     ⇒ 逐字节不变。
  2. **前导值必须能在当前作用域 elaborate**：`notation_prefix_args` 借来的值可能是签名文本里的望远镜名
     （`α`/`β`，在源级**不一定**是标识符）。`min7` 就是这种情形的邻域。⇒ 实现时**先把每个前导值 `elab_expr` 试一遍，
     任一失败就放弃重试、退回今天的回落**（宁可回落，不交病项）。
  3. **只在 `Unsolved` + 全 `Implicit{<missing}` 时触发** ⇒ 今天 `Solved` 的 16378+ 次一次都不多跑（零热路径成本）。
* **要不要动内核**：**不要**。

### 候选 2（内核，纯加字段）：`AppRequest` 增加 `prefix_vals: &[Option<ExprPtr>]`

* **改哪**：`crates/kernel/src/elab_app.rs` 的 `AppRequest`（`:28-45`）+ 隐式臂（`:265-278`）：`prefix_vals[i]` 是 `Some`
  就用那个项而不是 `app_fresh_meta`；调用方（`kernel_app.rs:431-438`、`:471-478`）把候选 1 解出的值填充进去。
* **为什么能解出**：同上——把"没人定的元变量"变成"给定的项"，一步到位，且能表达"只给一部分前导位"。
* **风险**：**内核改动**要走"开关关/开两对 714 行全语料"那一套（本刀代价高）。技术上它是**纯加法**
  （空切片 ⇒ 与今天逐字节相同），可以照 P2-11 系列的"只在有新输入时不同"来论证，但流程成本不低。
* **要不要动内核**：**要**。

### 候选 3（课程层，最 Lean-faithful，但**本刀不做**）：把 6 个站点的 `∅` 写明类型

* **改哪**：`unit19-solution` / `unit19-cardinal-le` 的 6 处 `∅ ≼ A` / `A ≼ ∅` ⇒ `(∅ : Set α) ≼ A` / `A ≼ (∅ : Set α)`
  （或 `Set.empty α`）。**为什么能解出**：`β` 被 `: Set α` 直接钉住 ⇒ 就是 `min6`，内核 `Solved`，一条回落都不需要。
* **风险**：① 这是**改课程写法**，与 replan 的 S4b 明文（"先复现定根因，**别改写法**"）冲突，也和"课程一律写记法"的
  用户拍板方向相抵；② 要过 `notation-lint.py` + 整门课 `check.py`；③ 治标——同样的形状在别处还会出现。
* **要不要动内核**：不要。**建议作为"长期正确解"记在 S4b 的收尾说明里，不作为本刀的修法。**

### ⚠ 反模式（**不要**做）

把候选 1 简化成"**`Unsolved` 时直接返回 `Fell::Untried`**"（不压 census、不结算）能让
`fallback_old_ok` 的 121 次**立刻归零**，但**行为一个字都没变**（照样走老路，老路照样出项）。
这会**伪造 S6 的验收判据**（"全课程 `fallback_old_ok` = 0"），属于把指标做绿而不是把回落消掉 ⇒ 必须避免。

## Q5 预期效应（**inferred**，未实测）

> 本轮只读，**没有**实现候选 1，所以下面全部是**推断**，不是实测。

* **`fallback_old_ok`：185 → 64**（`185 − 121`）。`Set.Le` 一族 121 次（`Set.Le` 头名，表 C）全部落在 6 个真实站点，
  每个站点都是"一个操作数是 `∅`、另一个把 `α`（或 `β`）定住、两层域都是 `Type`"⇒ **`fill_pending_by_shape` 在每个站点都能借到**
  （这正是那 121 次 `ok` 的实证）⇒ 重试应当在**全部 121 次**上成功。这个数与普查 §8.3 自己的估算一致。
  ⚠ 其中 2 次是 `set.le` 的 `notation_unsupported_input_contains_a_meta`（`unit19-solution 59:46`，与 `notation_unsolved`
  同站点、不同轮次）——**它们已含在 121 里**，随同一处一起消失（inferred）。
* **`fallback_old_err`（218）不变**（inferred）：重试只在 `Unsolved` 且前端解出前导值时触发，不会把 `err` 变 `ok`。
* **`Set.mem` / `Set.subset` 那 30+14 次 `input contains a meta` 不变**（inferred）：那是操作数**自身**带元变量
  （`Unsupported` 出口），不是前导形参缺失，候选 1 的触发条件不覆盖它们 ⇒ 不宣称。
* 因此修完本刀之后的下一步上界：**64**（再消 `Set.mem` 32 / `Set.subset` 12 / `Set.Equiv` 4 … 才到普查 §8.3 的 34）。

## 6. 没能确定的部分（如实记）

### 6.1 我没有改任何东西（证据）

`git status --short` 里的 6 个 `M` + 3 个 `??` **全部是并行 agent 的 S3 未提交改动**（`elab.rs` / `kernel_app.rs` /
`kernel_app_census.rs` / `kernel_app_notation.rs` / `mod.rs` / `tests/kernel_app_keys.rs` + 新增
`kernel_app_fallback.rs` / `tests/kernel_app_no_fallback.rs` / `reports/2026-11-01-p212-s3-readings.md`）。
**本次只读：一个字节都没写进仓库的 `.rs`/`.sokonanoda`**；也没有用探针改码 ⇒ **不需要"改—复原"证据**（`git diff`
里没有一个 hunk 出自我）。本报告是唯一新增文件。全程**未跑 `cargo`**、**未 `git commit`/`push`**。
二进制 sha 全程 `75dc6b34301eeba6`（做完所有测量后再核一次仍是它 ⇒ 没有被并行编译换掉）。

### 6.2 未确定

1. **Lean 版本不符**：本机是 `v4.12.0-rc1-5351-gd0493e4c1e`，不是 v4.35.0 `eb6cafb`（该对象不存在）。Q3 的 file:line
   在 v4.35.0 里可能漂移；结论（`finalize` 不要求隐式位解出 / `binrel%` 假设齐次 / 报错在声明末尾）我判断跨版本稳定，
   但**没有**在 v4.35.0 上核过。
2. **Lean 到底会不会拒绝 `A ≼ ∅`**：**没跑 Lean**（硬规则 2 禁止）。我只证到"Lean 的 `finalize` 不报错、报错发生在
   声明末尾的 `MVarErrorInfo.logError`"，**没有**证到那条 `logUnassignedUsingErrorInfos` 在这条具体输入上真的会触发。
   另外：本语言的 `infix:50 " ≼ " => Set.Le` 在 Lean 里要写成 `Set.Le _ _ A B`（Lean 的 `notation` 不补前导显式形参），
   所以"Lean 会怎么处理"严格说还要看那种写法 ⇒ **这条按 `(inferred)` 读**。
3. **候选 1 的验收**：会不会改变现有接受/拒绝、开关开的那对语料会变几行 —— **都要实现后实测**，本轮无法给。
4. **`unit19-solution 59:46` 那一处为什么同时有 `notation_unsolved` 与 `notation_unsupported_input_contains_a_meta`**：
   我推断是"探测重做（`probe::with_redo`）+ `judge_infer` 的前缀缓冲区再 elaborate"造成的**多次尝试、不同轮次不同结局**
   （`(inferred)`）；只确认了**决定性**的那次是 `Unsolved`（`SOKO_STRICT_NO_FALLBACK` 一开就只剩它）。
5. **`1589:40` 这一行（列 40，而真实站点是 46）**：`1589−1530=59` 行对得上，但列是 binder 的起点（` (h `）而不是
   `A ≼ ∅` 的起点（实测该位置字符 = ` (h `）⇒ **同一站点、不同 span 起点**（`(inferred)`）。普查 §5.3 把 `59:46` 记成
   **57** 次（48+9），我的日志是 48+9+**1**（那条 `1589:40`）= **58**；差 1 就在这里，不影响 121 这个总数（实测）。
6. **`∅` 之外还有没有别的"元素类型未定"操作数**：本轮只证了 `∅`（`Set.empty` + 记法）。`{a}` / `Set.univ` 等是否同形
   未测（`(inferred)`：会，因为都靠 `Set α` 的前导隐式）。

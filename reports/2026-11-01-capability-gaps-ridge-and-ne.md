# 翻默认档的最后两处能力缺口：**谁在报**、**loose 凭什么过**、**能不能零行增长修**

> **只读定根因**（2026-11-01，只读子代理）。分支 `strict/2026-10-29-kernel-app`（**未切分支、未碰 main**）。
> 起点 HEAD `41d98f45`，终点 HEAD `c7204534`（期间并行 agent 落了 `6da4f88d` / `c7204534`，只动 `docs/`/`reports/`，
> `git diff --stat 41d98f45..c7204534 -- crates/ editor/ skills/` 为空）。
> 上游报告：`reports/2026-11-01-strict-gaps-before-default-flip.md`（§8 更正：这两个文件是 unit 12/19 的**正规解答**）、
> `reports/2026-11-01-g3-meta-leak-rootcause.md`、`reports/2026-11-01-p2-s4b-setle-rootcause.md`。
> **判定逻辑 0 行、测试 0 行、课程文案 0 行、`check.py` 0 行、`crates/kernel/**` 0 行**；探针全部还原（§8 有逐位证据）。

## §0 五问的答案（一句话版）

| # | 答案 | 决定性读数 |
|---|---|---|
| **Q1** | **两条都是内核路径的 `Fell::Dead`**（strict 的 no-fallback 出口），**不是老路报的**。loose 根本不进这条路径（loose 的两次判卷里 `KERNEL_APP tried=` 一行都没有） | 位置日志 `dead unsolved Ne 175:8` / `dead unsupported_input_contains_a_meta Set.notMem_empty 63:45`；`bump_dead` 只从 `fell()` 的 `Report::Error` 臂调用（`kernel_app_fallback.rs:204-207`、`:388`） |
| **Q2** | **分叉点 = 路由闸门** `kernel_app_notation.rs:113` / `kernel_app.rs:249`（`!effective(dialect)` ⇒ loose 一行都不跑）。loose 靠**老路的既有线路**：unit12 是 `solve_prefix_args_meta` 的**路线①（读操作数类型 + 合一）**，不是猜；unit19 是**路线③「富余实参落到结果上」**。**两处都不是"strict 只少走一步"能概括**——unit12 是 strict 的**重试闸门太窄**，unit19 是 strict 的**出口规则顺序**把关掉的例外又判死 | `[CG1]` 在 loose 里 **0 条**；`[CG9] text=Some("Set Nat -> Prop")`（loose）/ `text=None`（strict）；`[CG2] sites_are_prefix=false`（因为有 `Level{u}` 位） |
| **Q3** | **两个不同的根因**，只共享**出口分类**那一段（`fell` → `report`）：<br>**GAP-R（脊/`Ne`）**= 重试闸门的 site 过滤把**同生的 `Level{u}`** 当成"不是前导位"⇒ 不重试 ⇒ `Unsolved` 直接判死。<br>**GAP-N（`Set.notMem_empty`）**= `report` 规则 1（`n_args > explicit_layers`）**抢在**写明的例外（`Unsupported`）之前 ⇒ 判死，而老路的路线③本来接得住 | 两个零 import 夹具（§4.2）：`fx12` 13 行、`fx19` 10 行；控制组 `fx19b` 两档全绿 |
| **Q4** | **GAP-N：能（落点全在未冻文件，且已实测整份 `unit19-solution` 转绿）。<br>GAP-R：不能（今天）。** 只放宽闸门不够——重试拿回的是**占位符载体**，而老路在同一站点**自己也答不出**（`operand_type_expr` 在 strict 下返回 `None`），剩下那一段在 `elab.rs`/`judge.rs`（**⑨ 冻结**）⇒ 要么先拆模块，要么换一条不经过 pp 回读的路 | `SOKO_CG_FIXB=1` ⇒ unit19 `rc=0 / 0 诊断`；`SOKO_CG_FIXA=1 + A2` ⇒ unit12 仍 `rc=1`，且 `fallback_old_err_notation_unsolved=1` |
| **Q5** | **Lean 4 两个形状都过**。unit19：`elabAppArgs` 每吃一个实参前 `whnfForall`，`Not P` 展开成 `P → False` 仍是 Π ⇒ 第二个实参被正常吃掉。unit12：`Ne` 的 `α` 是隐式元变量，`finalize` **不报错**，剩下的元变量到**声明末尾**才由 `logUnassignedUsingErrorInfos` 报；而 `?δ` 会被操作数内部的合一解出 ⇒ 过 | `Elab/App.lean:253-256 / 751-776 / 572-583 / 410-459`、`TermElabM.lean:859/888`（本机 `v4.12.0-rc1-5351`，非 v4.35.0） |

---

## §1 构建身份 · 复现 · 读数口径

| 项 | 值 |
|---|---|
| 仓库 / 分支 | `/Users/penglingwei/Documents/lean/sokonanoda/sokonanoda-lang` · `strict/2026-10-29-kernel-app` |
| 起点 HEAD | `41d98f45`（`git status --short` **空**：干净树） |
| **基线二进制** | `target/release/sokonanoda`，sha256 **`b6970f54b35be0a1`**（19:28 构建）；固化副本 `/tmp/soko-b6970f54b35be0a1` |
| 判卷命令（所有 rc/诊断/计数都出自这一条） | 见下 |
| 探针构建 | `c4cbca84` → `04de2789` → `bb9cb827` → `652b52d6` → `08d9c781` → `a5bcc1fd`（每一步只加**打印**与**环境变量门控**的分支，默认关） |
| **还原后重编** | **`b6970f54b35be0a1`（与基线逐位相同）** ⇒ 树已回到读数时的状态（§8） |

```bash
env -u SOKO_STRICT_AT SOKONANODA_NO_PROJECT_ARTIFACTS=1 SOKONANODA_NO_MODULE_ARTIFACTS=1 \
    SOKONANODA_CACHE_DIR=$(mktemp -d) SOKO_JUDGE_STATS=1 \
    SOKO_KERNEL_APP_LOG=/tmp/cg/logs/<tag>.log \
    <bin> grade --dialect {loose,strict} <绝对路径> --json
```

两个目标的两档基线读数（二进制 `b6970f54b35be0a1`）：

| 目标 | 档 | rc | 诊断 | 事件流 |
|---|---|---|---|---|
| `unit12-solution`（unit 12 正规解答） | loose | **0** | 0 | 全部 `decl.checked` + `exercise.open` |
| 同上 | strict | **1** | `elab-implicit-argument-unsolved` @ **175:8–177:141** | 同上 + 1 条 `diagnostic` |
| `unit19-solution`（unit 19 正规解答） | loose | **0** | 0 | — |
| 同上 | strict | **1** | `elab-implicit-argument-unsolved` @ **63:45–63:76** + 级联 `elab-unknown-identifier` @ **66:52** | — |

统计行（`SOKO_JUDGE_STATS=1`）：

```text
# unit12 strict
KERNEL_APP tried=4252 solved=4226 fallback_unsolved=25 ... nofallback_dead=25 nofallback_kept_argmismatch_text=1
# unit12 loose   ← 没有 "KERNEL_APP tried=" 这一行（= 内核路径一次都没跑）
# unit19 strict
KERNEL_APP tried=22904 solved=22724 fallback_unsolved=105 ... fallback_old_ok_notation_unsolved=49 \
  fallback_old_ok_unsupported_input_contains_a_meta=49 ... nofallback_dead=66 \
  nofallback_kept_prefix_meta=49 nofallback_kept_input_contains_a_meta=65
# unit19 loose   ← 同样没有 "KERNEL_APP tried=" 这一行
```

⚠ **`nofallback_dead` 是全局计数器，不能单凭它断定"判死"**：它把探测/重做（P2-5 `probe::with_redo`）那几趟也算进去。
实测反例：`SOKO_CG_FIXB=1` 的 unit19 跑出 `nofallback_dead=41` 而 **rc=0、0 诊断**。
所以本报告的归属一律走**位置日志**（`log_position` 的第三列是头名、第四列是 `行:列`）+ **文案出处**（下节）。

---

## §2 Q1 —— 谁在报？

### 2.1 机制上的唯一出口

`kernel_app_fallback.rs`：

```rust
// :203-217（fell）
match report(outcome, &facts.input()) {
    Report::Error(e) => { bump_dead(&facts.head, why, e.span); Fell::Dead(e) }   // :204-207
    Report::Fallback(reason) => { bump_kept(reason, ...); Fell::Fallback(why) }  // :208-211
    Report::Defer => { bump_kept("defer", ...); Fell::Fallback(why) }             // :212-215
}
```

`bump_dead`（`:388`）是 `dead` 日志行的**唯一**生产者，而 `bump_dead` 只被 `:205` 调用
⇒ **日志里出现 `dead …` ⇔ 这一次是 `Report::Error` ⇒ `Fell::Dead` ⇒ 诊断直接交出去、老路一行不跑**。
`Fell::Dead` 的交接点：`kernel_app.rs::try_app`（`:231`）与 `kernel_app_notation.rs::try_notation`（`:220`）。

### 2.2 unit12 —— `dead unsolved Ne 175:8`

`/tmp/cg/logs/u12-strict.log`（27 行，逐字摘录其中**连续 6 行**；这一段在该文件里出现两遍）：

```text
dead unsolved Set.image 172:6
dead unsolved Set.image 175:8
dead unsolved Set.image 177:11
dead unsolved Set.image 177:74
dead unsolved Set.inter 177:11
dead unsolved Ne 175:8          ← 目标站点（记法路由，span 175:8–177:141）
```

文案归属（**决定性**）：诊断正文里的那句

> `… 原因：`α` 由实参和期望类型都定不下来。`

**只**出现在 `crates/front/src/compile/kernel_app_report.rs:186`（`implicit_why`）。
老路同一个码的正文（`elab.rs:4893` 之后的 `implicit_msg::unsolved_message`）写的是
「（本子集按「后续显式实参的类型 + 期望类型」反解）」，**不含**这句 ⇒ 这条诊断是
`report()` 的 `unsolved()` 的 `has_implicit` 臂（`kernel_app_report.rs:126-131`）产物 ⇒ `Fell::Dead`。

### 2.3 unit19 —— `dead unsupported_input_contains_a_meta Set.notMem_empty 63:45`

`/tmp/cg/logs/u19-strict.log`（294 行；下面是**摘出的 4 行**，不是连续片段）：

```text
kept unsolved Set.Le 59:46
ok notation_unsolved Set.Le 59:46
kept unsupported_input_contains_a_meta Set.mem 59:69
dead unsupported_input_contains_a_meta Set.notMem_empty 63:45    ← 目标站点
```

内核这一次的结局是 **`Unsupported("input contains a meta")`**（`why` 直接来自 `Count::Unsupported`），
按设计它是**写明的例外**（`Report::Fallback`，`kernel_app_report.rs:88`；
本文件里同样的例外已 `kept` 了 **65** 次）。它之所以变成 `Error`，是因为 `report` 的
**规则 1** 挡在前面（`kernel_app_report.rs:63-65`）：

```rust
if input.n_args > input.explicit_layers {   // ← 先手，且与 outcome 无关
    return Report::Error(... strict_rejection_message(...));
}
```

探针读数（探针只打印，见 §8）：

```text
[CG7] report n_args=2 explicit_layers=1 k=1 fixb=false outcome=input contains a meta   × 10
[CG6] app head=Set.notMem_empty prefix=1 n_args=2 outcome=Unsupported("input contains a meta")
```

`Set.notMem_empty` 的签名是 `{α : Type} -> (a : α) -> Not (Set.mem a Set.empty)`：
`k=1`、`explicit_layers=1`、`n_args=2` ⇒ 规则 1 成立 ⇒ `Error` ⇒ `Fell::Dead`。

**同一份文件里、同一个 `why` 的两种结局**（`/tmp/cg/logs/fx19-strict.log`，22 行；下面是**摘出的 3 行**，
其余行是同一批 `ok`/`kept unsolved` 的重复读数）：

```text
kept unsupported_input_contains_a_meta Set.Le 10:39     ← 记法路由（missing=2），落写明的例外 ⇒ 老路 ⇒ 过
kept unsupported_input_contains_a_meta Set.mem 10:61    ← 应用路径，n_args=2 ≤ explicit_layers=2 ⇒ 例外 ⇒ 过
dead unsupported_input_contains_a_meta Set.notMem 10:79 ← 应用路径，n_args=2 > explicit_layers=1 ⇒ 规则 1 ⇒ 死
```

### 2.4 Q1 结论

**两条都是"内核路径 + strict 恒不回落"的 `Fell::Dead`，不是老路报的。** 两条 `why` 分别是
`unsolved`（unit12）与 `unsupported_input_contains_a_meta`（unit19）。
**"strict 回落了、老路自己也解不出"这个假设不成立**：
- unit12：老路**能**解（loose 会走它，且解出 `Set Nat -> Prop`，见 §3.1）；
- unit19：老路**能**解（实测：把规则 1 让开 ⇒ 整份文件 `rc=0`、0 诊断，见 §5.2）。

---

## §3 Q2 —— 为什么 loose 能过？

### 3.1 分叉点（文件:行）= **路由闸门**，不是判定细节

`kernel_app_notation.rs:113`（记法路由第一行）与 `kernel_app.rs:249`（应用路由）：

```rust
if !effective(ctx.options.dialect) || !routing_enabled() || ... { return Ok((None, None)); }
```

`effective()`（`kernel_app.rs:57-64`）= `dialect.strict()` ⇒ **宽松档直接返回 ⇒ 老路一行不落**。
实测：loose 的两次判卷里 **`[CG1]`（记法路由入口探针）0 条、`KERNEL_APP tried=` 0 行**；
strict 的同一份文件 `[CG1]` 有几十条。**loose 与 strict 走的根本不是同一条路**。

### 3.2 unit12：loose 解出的 `α` 是**读出来的**，不是猜的

严格档 `[CG1]`：

```text
[CG1] notation head=Ne span=175:8 missing=1 operands=2
      outcome=Unsolved { sites: [Implicit { index: 0 }, Level { param: "u" }] }
[CG2] retry-gate head=Ne retry_enabled=true would_cut=false sites_are_prefix=false
```

`Ne` 是**宇宙多态**（`(α : Sort u) -> (a : α) -> (b : α) -> Prop`）。P2-12 S4b 的重试闸门
（`kernel_app_notation.rs:171-175`）要求

```rust
sites.iter().all(|s| matches!(s, UnsolvedSite::Implicit { index } if *index < missing))
```

`sites` 里有 `Level { param: "u" }` ⇒ **`all(..)` 为假 ⇒ 重试根本不跑** ⇒ `prefix_meta=false`
⇒ `notation_fell` → `fell` → `Report::Error` ⇒ Dead。
**`Level{u}` 与前导位 α 是同一件事的两面**（α : Sort u ⇒ α 一确定，u 就确定），
把它当"缺的不是前导位"是把一个**同生位**读成了外来位。

宽松档（同一站点，`[CG9]`/`[meta]` 读数）：

```text
[CG9] operand_type_expr explicit=false text=Some("Set Nat -> Prop") gate=true angle=false parsed=true in_scope=Some(Some(true))
[meta] missing=1 ids=1 solved=Some(1) unsolved=6
[meta]   #0 value=Some(Arrow { domain: App { Set, Nat }, codomain: Prop })      ← α := Set Nat -> Prop（= Set (Set Nat)）
```

链路：`elab.rs:1665` 的 `try_notation` 早退 → `elab.rs:1681` `prefix_args_cached` →
`elab.rs:2803 notation_prefix_args` → `elab.rs:3081 solve_prefix_args_impl`（窄版解不出）→
**`elab.rs:2958 solve_prefix_args_meta`**（Engine 档，`implicit.rs:413 solve_prefix_meta_alt`）
→ **路线①** `operand_type_expr`（`elab.rs:4052`）+ `meta.unify`（`elab.rs:3039`）。

⇒ **不是哪条"猜"的启发式**：
- **不是** `fill_pending_by_shape`（`implicit.rs:599-634`）：它的判据是"抄一个**已解**兄弟"，`Ne` 的
  `missing=1` 只有一格、没有兄弟 ⇒ 这条在这里**用不上**（`Set.Le` 的 `missing=2` 才用得上）；
- **不是** `elab_engine_value`（`pp_shape.rs`，那是 G3 的载体泄漏那条路）；
- 是 `solve_prefix_args_meta` 的**路线①**：把第一个操作数的类型**读出来**再合一。这是文档化的规则
  （`elab.rs:2793-2801` 的注释就是这两条路线），不是启发式。

### 3.3 unit19：loose 走的是路线③「富余实参落到结果上」

`Set.notMem_empty (f x) (hn x hx)` 是**一条脊上分两步应用**：头吃 1 个显式位，第二个实参吃的是
**结果的 Π**（`¬ (f x ∈ ∅)` = `(f x ∈ ∅) → False`）。

老路为这件事有专门的机械 `elab.rs:4275 fn surplus_layers`（注释：「把 `base` 沿 Π 展开 `surplus` 层 —— 路线③ 的
「富余实参落到**结果**上」。`Set α` 是 **def** ⇒ 不 δ 展开就看不到 `α -> Prop` 那一层」），
`elab.rs:4782` 的严格档拒绝**只在路线③试过之后**才执行（注释原文：「路线③ 已经试过…⇒ 走到这里说明结果**不是**
能再吃实参的函数」，并点名 Lean 4 `App.lean:1113 fType := body.instantiate1 arg` 是同一读法）。
⇒ **≤ 显式层数之外的那一个实参，老路本来就能吃。**

strict 为什么没走到路线③：内核路径先跑。内核这一次的结局是
`Unsupported("input contains a meta")`（**写明的例外**，本该回落），却被 `report` 规则 1 先手判死（§2.3）。

### 3.4 strict 为什么"带 meta"—— 触发者是 **S4c 的 `prefix_meta` 例外**

同一个站点的内核结局在同一份文件里**两种都有**（读数，同一 run）：

```text
[CG6] app head=Set.notMem_empty prefix=1 n_args=2 outcome=Solved(...)          ×3
[CG6] app head=Set.notMem_empty prefix=1 n_args=2 outcome=Unsupported("input contains a meta")  ×3
```

差别是**当时的作用域**。内核在任何工作之前先扫输入（`crates/kernel/src/elab_app.rs:179-187`，
`binder_tys` = 前端的 `scope.kernel_tys()`，`kernel_app.rs:435`）：

```rust
for e in inputs { if !self.ctx.unassigned_mvars(*e).is_empty() { return AppOutcome::Unsupported("input contains a meta"); } }
```

而 `unit19-solution` 的 `le_empty` 的签名里有 **`(h : A ≼ ∅)`**：这个 `≼` 站点正是 S4c 的
**占位符一族**（`kept prefix_meta`，本文件 **49** 次），老路接住它时给的前导 α 是**载体** `\0soko_m*`
（`[CG3] #0 "Ident { name: \"\\0soko_m2\" }"` + `[CG4] elab_all ok has_meta=[true,false]`）⇒
**声明类型里留着一个元变量** ⇒ 它进了 scope ⇒ 该声明体内**每一个**应用的内核输入都"含 meta"。
读数：本文件 `nofallback_kept_input_contains_a_meta=65` vs `dead ... Set.notMem_empty` **1** —— 65 个同因的
都走了写明的例外，只有这个"多一个实参"的被规则 1 判死。

---

## §4 Q3 —— 同一根因还是两个？两个最小夹具

### 4.1 结论

**两个不同的根因**（GAP-R / GAP-N），只共享**出口分类**这一段代码：

| | GAP-R（`Ne`，unit12） | GAP-N（`Set.notMem_empty`，unit19） |
|---|---|---|
| 判死的位置 | `kernel_app_notation.rs:171-175`（重试闸门的 site 过滤） | `kernel_app_report.rs:63-65`（规则 1 抢在例外之前） |
| 内核结局 | `Unsolved { Implicit{0}, Level{u} }` | `Unsupported("input contains a meta")`（**本可以回落**） |
| 为什么老路能过 | 路线① 读到操作数类型 `Set Nat -> Prop` | 路线③ 把富余实参喂给结果的 Π |
| 缺口性质 | **闸门太窄**（把同生的 `Level` 读成外来位） | **规则顺序**（拒绝文案先于写明的例外） |
| 共享的一段 | `kernel_app_fallback::fell`（`:188-217`）→ `kernel_app_report::report`（`:63`）+ `Report::Fallback` 例外表 / `key()`（`:418`） | 同左 |

（两处**都不是**内核合一的锅：unit19 在干净作用域里内核 `Solved`；unit12 的 `Ne` 有独立的
`Set.image`/`Set.inter` 同族 `Unsolved`（§7 未定项 2），但那是 S4b 的既有族，不是本刀的新缺口。）

### 4.2 两个最小夹具（**零 import**，`/tmp/cg/proj/`；原文逐字）

**夹具 A（GAP-R）`fx12.sokonanoda`** —— 13 行：

```sokonanoda
def Set (α : Type) : Type := α → Prop
def Set.empty {α : Type} : Set α := fun (_ : α) => False
def Set.singleton {α : Type} (a : α) : Set α := fun (x : α) => x = a
def Set.univ {α : Type} : Set α := fun (_ : α) => True
def Set.inter {α : Type} (A B : Set α) : Set α := fun (x : α) => A x ∧ B x
def Set.image {α β : Type} (f : α → β) (A : Set α) : Set β := fun (y : β) => True
notation "∅" => Set.empty
infixl:70 " ∩ " => Set.inter
infixr:80 " '' " => Set.image

theorem t :
    (fun (_ : Set Nat) => ∅) '' ((@Set.singleton (Set Nat) (∅)) ∩ (@Set.singleton (Set Nat) (@Set.univ Nat)))
      ≠ ((fun (_ : Set Nat) => ∅) '' (@Set.singleton (Set Nat) (∅))) ∩ ((fun (_ : Set Nat) => ∅) '' (@Set.singleton (Set Nat) (@Set.univ Nat))) := by
  sorry
```

| 夹具 | loose | strict | strict 诊断 |
|---|---|---|---|
| `fx12` | **rc=0 / 0 诊断** | **rc=1** | `elab-implicit-argument-unsolved` @ **12:6**，正文与 unit12 **逐字同形**（`Ne` 的 α） |

**夹具 B（GAP-N）`fx19.sokonanoda`** —— 10 行：

```sokonanoda
def Set (α : Type) : Type := α → Prop
def Set.empty {α : Type} : Set α := fun (_ : α) => False
def Set.mem {α : Type} (a : α) (A : Set α) : Prop := A a
def Set.notMem {α : Type} (a : α) : ¬ (Set.mem a Set.empty) := fun (h : Set.mem a Set.empty) => h
def Set.Le (α β : Type) (A : Set α) (B : Set β) : Prop := True
notation "∅" => Set.empty
infix:50 " ∈ " => Set.mem
infix:50 " ≼ " => Set.Le

theorem t (α : Type) (A : Set α) (h : A ≼ ∅) (a : α) (hne : a ∈ ∅) : False := Set.notMem a hne
```

控制组 `fx19b` = 同一份但**删掉 `(h : A ≼ ∅)` 那一个 binder**（其余一字不改）。

| 夹具 | loose | strict | strict 位置日志 |
|---|---|---|---|
| `fx19` | **rc=0 / 0 诊断** | **rc=1** | `kept … Set.Le 10:39` · `kept … Set.mem 10:61` · **`dead … Set.notMem 10:79`** |
| `fx19b`（控制） | rc=0 | **rc=0** | 无（作用域里没有载体 ⇒ 内核 `Solved`） |

⇒ 夹具 B 把 unit19 的因果链**一步钉死**：**同一个 `why`（input contains a meta）**下，
`n_args ≤ explicit_layers` 的两处走例外（过），`n_args > explicit_layers` 的那一处被规则 1 判死。
"缺的不是能力，是出口顺序"。

---

## §5 Q4 —— 能不能零行增长修？

### 5.1 ⑨ 冻结面（实测行数，还原后复测一致）

| 文件 | 行数 | ⑨ 基线 | 可否改 |
|---|---:|---:|---|
| `crates/front/src/compile/elab.rs` | **8841** | 8841 | **冻结（一行都不许增）** |
| `crates/front/src/compile/implicit.rs` | **1218** | 1218 | **冻结** |
| `crates/front/src/judge.rs` | **4748** | 4748 | **冻结** |
| `crates/front/src/compile/meta.rs` | **1848** | 1848 | **冻结** |
| `crates/front/src/compile/kernel_app_report.rs` | 395 | — | **未冻**（<500 软线） |
| `crates/front/src/compile/kernel_app_notation.rs` | 454 | — | **未冻** |
| `crates/front/src/compile/kernel_app.rs` | 488 | — | **未冻**（逼近软线） |
| `crates/front/src/compile/kernel_app_fallback.rs` | 489 | — | **未冻**（逼近软线） |

基线来自 `scripts/src-lines-baseline.json`；⑨ 的判据是 `python3 scripts/docs-lint.py --check-src-lines`
（超软线新文件 / 存量变长 / 基线过期 **三条都判红**）。**四个冻结文件恰好都在上限上 ⇒ "同文件零增长"= 只能等量替换。**

### 5.2 GAP-N（unit19）：**能，且已实测整份文件转绿**

- 落点候选：**`kernel_app_report.rs:63-65`**（规则 1）。
- 已实测（探针开关 `SOKO_CG_FIXB=1` 让规则 1 不生效、其余一字不改）：

| 目标 | 档 | rc | 诊断 |
|---|---|---|---|
| `unit19-solution` | strict | **0** | **0**（66 个 `dead` 里的致命那一个消失，其余照旧走写明的例外） |

  `KERNEL_APP … nofallback_dead=41 nofallback_kept_prefix_meta=30 nofallback_kept_input_contains_a_meta=22`
  （对比基线 `dead=66 / kept_prefix_meta=49 / kept_input_contains_a_meta=65` —— 数字变化是**回落次数**变了，不是判定口径变了）。
- 为什么这是"修能力"而不是"藏红线"：规则 1 的**本意**是逐字节复制老路的先手（`elab.rs:4782`），
  但老路的先手**在路线③之后**，`report` 把它搬到了**路线③之前**（它在出口处看不到路线③试过没有）
  ⇒ 让开规则 1 = 把判定权**交回它本来就该问的那条路**，老路失败时仍会报**同一句**文案（`strict_rejection_message`
  在 `implicit_msg.rs:167`，两边同一个函数）。
- **零行增长修法**（两个都可行）：
  1. **删掉规则 1**（净 **−n 行**）：让 `Unsupported`/`ArgMismatch` 例外表接管，真正的"实参写多了"由老路在
     `elab.rs:4782` 报同一句；
  2. 或把规则 1 收窄成"**只有在 outcome 不是写明的例外时**才抢"（与 `:80-92` 的 match 合并，仍可做到 −1 行）。
- ⚠ 风险面（必须在修那一轮实测，本刀没跑）：`report` 的规则 1 被 `crates/cli/tests/strict_diagnostics.rs`
  的 t1/c2/c5 一臂断言着文案与 span；删掉后这些用例改由老路产出 ⇒ 必须**逐字节**比对（`sanitized()` 与
  老路不过 `sanitized` 的差别是唯一可能的偏移）。

### 5.3 GAP-R（unit12）：**今天不能；两条路都不在未冻文件里**

实测三步（`SOKO_CG_FIXA` = 闸门放行 `Level{_}`；`SOKO_CG_FIXA2` = "前缀是载体 ⇒ 记成 `prefix_meta` 例外"）：

| 组合 | 结果 | 读数 |
|---|---|---|
| 基线 | rc=1 | `dead unsolved Ne 175:8` |
| **A**（只放行闸门） | rc=1 | `[CG3] notation_prefix_args ok len=1` 但 `#0 = Ident { name: "\0soko_m0" }` ⇒ **`[CG4] elab_all FAILED`** ⇒ 仍 `dead` |
| **A + A2** | rc=1 | 站点变成 `kept prefix_meta`（`nofallback_kept_prefix_meta=1`），**但老路自己失败**：`fallback_old_err_notation_unsolved=1` |

第三行的原因是**第二阶缺口**（实测）：

```text
[CG9] operand_type_expr explicit=true text=None gate=true angle=false parsed=false in_scope=None   ← strict
[CG9] operand_type_expr explicit=false text=Some("Set Nat -> Prop") … parsed=true in_scope=Some(Some(true))  ← loose
```

同一个站点、同一个操作数：**strict 下 `infer_type_text` 直接返回 `None`，loose 下返回一段可解析的类型文本**。
链路上它落在 `elab.rs:3675 infer_type_text` → `judge_infer(...)`（`elab.rs:3703`，慢路 = 合成 `#check` 文档 + 整份前缀重跑）——
**两个都在 ⑨ 冻结文件里**。而且**"pp 档"不是原因**（`elab.rs:4066` 的 `explicit` 标志，我用
`SOKO_CG_FIXA3` 把它在 strict 下强制成 `false` 复测：`[CG9] … text=None` 不变）；
`⟨meta#N⟩` 那一闸与 `pp_type_scope_safe` 也已排除（读数 `angle=false`、`parsed=false` ⇒ 根本没走到它们）。

⇒ **GAP-R 的零行增长修法给不出**。三条可能的路（都要先拍板，本刀不裁）：

1. **换一条不经过 pp 回读的路**（**最有希望，且全在未冻文件**）：不调老路的 `notation_prefix_args`，
   而是让内核自己把域下来 —— `run` 已经支持**逐位源级期望类型** `expected_srcs`
   （`kernel_app.rs:345`（`run` 形参）/ `:376`（`run_once` 形参）/ `:391`（真正用它喂位））与 `postpone::elab_ready`（`NeedArgs{domains}` 协议）。
   Lean 的次序就是"先按域 elaborate 第一个实参"（§6）。这条路不需要 `infer_type_text`，
   落点 = `kernel_app_notation.rs` 的 `retry_with_prefix`（或 `kernel_app_postpone`）。
   **未实测**，本刀只给方向与判据。
2. **把 `infer_type_text`/`judge_infer` 的那一小段拆出去**（新模块 <500 行），`elab.rs`/`judge.rs`
   同轮净减 ⇒ 才允许加新行。这是 ⑨ 的正规通道（「先拆模块」），成本 = 一次 `elab.rs` 拆分刀。
3. **接受**：把 `Ne` 这一族的 strict 判死当"已知限制"记进台账（那不是"修能力"，是把翻默认档的门槛换成文案）。

### 5.4 Q4 结论

- **GAP-N 可以零行增长修**：落点 `kernel_app_report.rs:63-65`（未冻），**整份 `unit19-solution` 已实测转绿**；
- **GAP-R 不可以（今天）**：闸门（`kernel_app_notation.rs`，未冻）**不足以**修好它，剩下的堵点在
  `elab.rs:3675` / `judge.rs`（**冻结**）⇒ 要么走新路子 1（不碰冻结文件，未实测），要么先拆 `elab.rs`。
- 因此"翻默认"的**最后一道门不是一处**：先修 GAP-N（便宜、干净），GAP-R 需要**单独一刀**设计。

---

## §6 Q5 —— Lean 4 会怎么判

> ⚠ **版本**：本机 `~/Documents/lean/lean4` 是 **`v4.12.0-rc1-5351-gd0493e4c1e`**（`git describe --tags`），
> 不是任务卡上的 `v4.35.0 eb6cafb`。下面 file:line 是 v4.12.0-rc1 的；**只读源码，未跑任何 Lean 工具链**。

### 6.1 unit19 的形状（脊上分两步应用）：**Lean 过**

`src/Lean/Elab/App.lean`：

* `ElabAppArgs.main`（`:751-776`）每吃一个实参前先 `let fType ← normalizeFunType`；
* `normalizeFunType`（`:253-256`）= `whnfForall s.fType`（**先实例化已赋值元变量再展开**）；
* `Not P` 是 `def Not (a : Prop) : Prop := a → False` ⇒ `whnfForall (¬ (f x ∈ ∅))` = `(f x ∈ ∅) → False`，
  **仍然是 Π** ⇒ 第二个实参照常被 `processExplicitArg` 吃掉；
* 只有当"还有实参没吃、而 `fType` whnf 后仍不是 Π"时才
  `synthesizePendingAndNormalizeFunType`（`:222-248`）→ 最终报 `Function expected at …`（`:248`）。

⇒ 与我们的老路**路线③**（`elab.rs:4275` / `:4782` 的注释点名 `App.lean:1113`）是**同一条读法**；
与内核的 `whnfForall`（`elab_app.rs:430` 的注释）也是同一条。
**我们的 strict 之所以拒，是出口顺序的问题，不是语义问题。**

### 6.2 unit12 的形状（`Ne` 的 α 两个操作数都"没定"）：**Lean 过**（(inferred)，见 §7）

* `Ne` 的 `α` 在 Lean 里是**隐式** binder ⇒ `addImplicitArg`（`:572-583`）只做 `mkFreshExprMVar argType`，
  **没有"必须解出"的分支**；
* `finalize`（`:410-459`）只做三件事：把隐式位的元变量登记进错误上下文
  （`:416-418` → `registerMVarErrorImplicitArgInfo`，`:84`）、`synthesizeAppInstMVars`、
  以及**非致命**地把期望类型合一一次（`:455-457` `discard <| isDefEq expectedType eType`）。
  **它不报错，项带着未赋值元变量交出去。**
* 真正的报错在**声明末尾**：`logUnassignedUsingErrorInfos`（`TermElabM.lean:888`）对
  `MVarErrorKind.implicitArg` 印 **"don't know how to synthesize implicit argument"**（`:859`）。
* 所以"Lean 过不过"取决于**那个元素类型元变量到底被解出来没有**：我们的 loose 是解出来的
  （`Set Nat`，来自 `∩` / `singleton` 的内部合一，§3.2 的读数），Lean 的 `isDefEq` 会在吃实参时
  做同一份合一 ⇒ **（inferred）过**。若它没被解出来，Lean 也会在声明末尾报同一句话。

### 6.3 `binrel%` 的齐次假设（与本刀的关系）

`Elab/Extra.lean:425-432` 的注释原文：「**We assume `binrel%` elaborator is only used with homogeneous
predicates.**」；`elabBinRelCore` 先 `analyze tree none`（`:534` 附近）取 `maxType`，不足时
`ensureHasType lhsType rhs`（`:543`）把两边逼成同一类型再 `elabAppArgs`（`:544`）。
⇒ Lean **没有**任何"接住 `A ≼ ∅`"的机制（要么假设齐次，要么声明末尾报"补不出隐式实参"）。
**我们的 `missing_prefix`（记法替用户补前导显式形参）是 Lean 没有的扩展**（`kernel_app_notation.rs:1-11` 的模块注释），
`Ne` 的 `α` 在源里是**显式**的、由记法补 —— 这正是为什么 unit12 会碰到 `Level{u}` 同生位这种形状。
⇒ 结论不变：**这两个形状在 Lean 里都是合法的；strict 的两次拒绝是"我们自己的限制"**，
所以修的方向是"让 strict 追上 loose/Lean"，**不是**让课程改写法（改课程会把
`REQUIREMENTS §9.40` 与"课程一律写记法"的用户拍板一起推翻）。

---

## §7 我没能确定的部分

1. **GAP-R 的第二阶堵点没定到行**：`infer_type_text`（`elab.rs:3675`）在 strict 下返回 `None`、
   在 loose 下返回 `Some("Set Nat -> Prop")` —— 我**只**排除了三件事（pp 档 `explicit`、`⟨meta#N⟩` 闸、
   `pp_type_scope_safe`），**没有**在 `infer_type_text`/`judge_infer` 内部加探针（两个文件都冻结，
   且 `judge_infer` 的慢路是"合成 `#check` + 整份前缀重跑"，读起来成本高）。可能是 `judge_infer` 的
   前缀重跑在 strict 下失败（原因未取），也可能是 `judge_cache_key` 的方言差异导致走了不同分支。
   **这是"翻默认"唯一还缺的一块。**
2. **内核侧没取**：`Ne` / `Set.image` / `Set.inter` 在 strict 下报 `Unsolved{Implicit{0}}`，而两个操作数
   **都不含元变量**（否则会走 `Unsupported("input contains a meta")`）——内核为什么不去做 `α ≟ typeof(arg0)`，
   我没有取证（`crates/kernel/**` 明确不许碰）。它与 S4b 已有的 `∅` 闭环族形状相同，但**不能说成同一个 bug**。
3. **GAP-N 修法的判据没跑**：只测了两份真文件里的 **2 个目标**；`crates/cli/tests/strict_diagnostics.rs` 的
   t1/c2/c5（规则 1 的既有断言）、整门 `check.py`、以及"`strict ≤ loose`"
   都要在修那一轮实测（本刀只读、不实现）。
4. **`sanitized()` 的差别**：规则 1 的产物过 `sanitized`，老路不过 ⇒ 删掉规则 1 后**文案是否逐字节相同**，
   我只做了机制判断（`strict_rejection_message` 是同一份文本、不含内核名），**没有**逐字节比。
5. **`Set.Le` 的 `fill_pending_by_shape` 我没有重测**（只引用 `reports/2026-11-01-p2-s4b-setle-rootcause.md`
   的既有读数）；本刀只测了 `missing=1` 的 `Ne` 用不上它。
6. **Lean 的 verdict 是 (inferred)**（§6.2）：我读的是 v4.12.0-rc1 源码，且**没有**跑 Lean（硬规则 2）。

---

## §8 探针清单与还原证据

### 8.1 加了哪几处（**全部 env 门控 + 只打印**）

| # | 文件（未冻？） | 位置 | 门控 | 打印 |
|---|---|---|---|---|
| CG1 | `kernel_app_notation.rs`（未冻） | `try_notation` 里 `run()` 之后 | `SOKO_CG_TRACE` | 记法头 / span / `missing` / `outcome` |
| CG2 | 同上 | 重试闸门 | `SOKO_CG_TRACE` | `retry_enabled` / `would_cut` / `sites_are_prefix` |
| CG3 | 同上 | `retry_with_prefix` 拿到前缀时 | `SOKO_CG_TRACE` | `len` vs `missing` + 每个前导值的 `Debug` |
| CG4 | 同上 | `elab_all` 结果 | `SOKO_CG_TRACE` | 失败 / `has_meta` 位图 |
| CG5 | 同上 | 重试后的 `run` | `SOKO_CG_TRACE` | 重试结局 |
| CG6 | `kernel_app.rs`（未冻） | `attempt` 交 `finish` 之前 | `SOKO_CG_TRACE` | 应用头 / `prefix` / `n_args` / `outcome` |
| CG7 | `kernel_app_report.rs`（未冻） | `report` 入口 | `SOKO_CG_TRACE` | `n_args` / `explicit_layers` / `k` / outcome |
| CG8/9 | **`elab.rs`（⑨ 冻结，临时）** | `operand_type_expr` 的 `infer_type_text` 之后 | `SOKO_CG_TRACE` | 文本 / `gate` / `angle` / `parsed` / `in_scope` |
| A | `kernel_app_notation.rs` | 重试闸门 | **`SOKO_CG_FIXA`** | 放行 `Level{_}` 位（**语义分支**，默认关） |
| A2 | 同上 | `elab_all` 失败分支 | **`SOKO_CG_FIXA2`** | 前缀含载体 ⇒ 记成 `prefix_meta`（默认关） |
| A3 | `elab.rs`（冻结，临时） | `operand_type_expr` 的 `explicit` | **`SOKO_CG_FIXA3`** | 强制 `explicit=false`（默认关） |
| B | `kernel_app_report.rs` | 规则 1 | **`SOKO_CG_FIXB`** | 让规则 1 不生效（默认关） |

**A/A2/A3/B 是"修法可行性探针"**（会改判定），一律默认关；**基线读数全部在默认关时取的**，
且已用"还原后重编 = 与基线逐位相同"证明默认路径未变。

### 8.2 还原（`git checkout` + 逐位证明）

```bash
git checkout -- crates/front/src/compile/kernel_app_notation.rs crates/front/src/compile/kernel_app.rs \
                crates/front/src/compile/kernel_app_report.rs crates/front/src/compile/elab.rs
```

三条判据（**实测**）：

```text
$ git status --short                      → 空（除本报告外；并行 agent 的改动已被他们自己 commit 到 6da4f88d / c7204534）
$ grep -rn "SOKO_CG_\|mentions_carrier" crates/ | wc -l   → 0
$ wc -l crates/front/src/compile/{elab,implicit,meta}.rs crates/front/src/judge.rs
   8841 elab.rs · 1218 implicit.rs · 4748 judge.rs · 1848 meta.rs   ← 与 ⑨ 基线逐项相同
$ shasum -a 256 target/release/sokonanoda | cut -c1-16   → b6970f54b35be0a1  ★ 与读数用的基线逐位相同
```

最后一条是最硬的证据：**还原后的树重编出来的二进制与基线二进制 sha256 相同** ⇒
探针没留下任何语义残留，本报告所有基线读数对当前树同样成立。

### 8.3 证据文件（`/tmp`，不随仓库走）

| 文件 | 内容 |
|---|---|
| `/tmp/soko-b6970f54b35be0a1` | 基线二进制（全部基线读数） |
| `/tmp/soko-cgprobe{,2..7}` | 探针构建 `c4cbca84` → `a5bcc1fd` |
| `/tmp/cg/proj/fx12.sokonanoda` · `fx19.sokonanoda` · `fx19b.sokonanoda` | §4.2 的三个夹具（`/tmp/cg/proj/sokonanoda.toml` + `lib` 软链只给带 import 的中间夹具用） |
| `/tmp/cg/logs/*.log` | `SOKO_KERNEL_APP_LOG` 位置日志（`dead` / `kept` / `ok` 三列） |
| `/tmp/cg/out/*.json` · `*.err` | 两档事件流 + `KERNEL_APP …` 统计行 + 探针 stderr |
| `/tmp/cg/orig_*.rs` | 四个被改文件的原始副本（二次保险） |

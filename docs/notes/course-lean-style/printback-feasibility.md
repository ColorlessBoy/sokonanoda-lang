# 显示期 print-back（源码级记法回显）可行性调研

> **2026-09-21 已升格**：结论与硬规则进了 `docs/design/notation-aware-printing.md` §3
> （权威）。本文留作**调研现场**——逐条的坑、实测数字、被否方案都还在这里，
> 实施时以设计文档为准。


> 状态：**调研结论（只读，未改任何代码）**。基线版本 `0.61.0`（`Cargo.toml:6`）。
> 触发问题：`courses/set-theory/` 即将全面改用数学符号（`∧ ∨ ↔ ∃ ∀ ¬ →`），
> 但学习者在 goal 面板 / hover 里看到的类型文本由**冻结内核的 pretty-printer**
> 产出点名形式（`And A B`、`Set.subset α A B`），可读性受损。
> 设计文档 `docs/design/notation-subset.md` §13.1 把「源码级 print-back」列为
> 「内核 pp 冻结 ⇒ 销不掉」的保留边界。
> 本调研要回答的是：**能不能不改内核，在前端做一层「显示期 print-back」**。
>
> 本文就是 `docs/design/course-lean-style.md` §7 **SP2** 要求的 spike 报告
> （对应其 §10 **N-3** 的「不做」判定、§1.4 的现状描述、§3 **L2.4** 的 `=` 工作项）。
>
> 全部结论都带 `文件:行号` 证据；关键事实用**真二进制实测**复核（复现命令见 §10）。
> **未调用官方 Lean 工具链**（只用 `target/debug/sokonanoda`）。

---

## 0. 结论先行

**有条件做。** 但要做的是**第三种方案**，不是 §13.1 列举的 ① 或 ②。

| 方案 | 内容 | 判定 |
|---|---|---|
| ① 改内核 pp | 违反硬规则 1 | **不可行**（`docs/design/notation-subset.md:550`） |
| ② 在 elab 保留「源 → 核」映射，把所有 pp 消费点换成源级渲染 | §13.1 否掉的那条 | **不该做**（两套真相 + 回读分叉，§2） |
| ③ **显示边界重写**：拿到内核产出的**文本**，在**它进入显示字段的最后一米**解析回 AST、按记法表渲染 | 本调研提出的方案 | **可行，推荐**（§4） |

三条支撑事实：

1. **「内核文本 → AST」的往返今天已经在生产路径上跑了**，不是新发明：
   `crates/front/src/judge.rs:526`（`judge_infer` 剥 binder 层）、
   `crates/front/src/judge.rs:597`（`judge_render_type`）、
   `crates/front/src/by.rs:106`（`canonical_goal_type`）。
2. **§13.1 的前提已经部分失效**：它说「goal/hover 的类型文本仍由冻结内核的 pp 产出
   点名形式」（`docs/design/notation-subset.md:13-14`），但实测**大部分 goal 面板内容
   今天已经回显记法**——因为 open 练习的 goal 文本走的是前端 `render_expr(源 AST)`，
   而 `render_expr` 早就把 `Expr::Notation` 打回符号（`crates/front/src/proof.rs:323-345`）。
   真正的泄漏面比 §13.1 描述的**窄得多**，而且是**可枚举的**（§1.3）。
3. **§13.1 担心的「回读路径立刻分叉」是真的**，而且能定位到具体函数
   （§6.2）；但**方案 ③ 天生不碰回读路径**——它在所有回读**之后**才发生。
   这条纪律必须用类型系统钉死（§4.4）。

**收益/成本对比（实测口径）**：最小可用约 **1 个新模块 + 5 个调用点 + 250~400 行 Rust**，
判卷事件流**一个字节不变**（§4.5、§7）。

---

## 1. 证据：类型文本的两条来源，与今天的实际表现

### 1.1 两条来源

**A. 冻结内核的 pretty-printer（点名形式）**

| 产物 | 产生点 |
|---|---|
| `CheckEvent::TypeChecked { text }`（`#check` / `#reduce` / `expr.typed` 事件） | `crates/front/src/compile/check/kernel_phase.rs:340`（`tc.with_pp(\|pp\| pp.pp_expr(ty))`）→ 事件在 `:343` |
| `DeclState.ty_text`（Open 练习） | `crates/front/src/compile/check/kernel_phase.rs:170-177` |
| `DeclState.ty_text`（普通声明） | `crates/front/src/compile/check/kernel_phase.rs:207-213` |
| `HoverType.text`（表达式 hover） | `crates/front/src/compile/check/mod.rs:766-790`（`tc.with_pp_scoped(&node.scope_names, \|pp\| pp.pp_expr(ty))` 在 `:771`，`HoverType` 在 `:784-790`） |
| pp 本体 | `crates/kernel/src/pretty_printer.rs:900`（`pub fn pp_expr`） |

**B. 前端 `render_expr`（源码级，记法感知）**

`crates/front/src/proof.rs:226` 起，`Expr::Notation` 臂在 `:323-345`，binder 记法在
`:361-371`（`render_binder_notation`），集合字面量在 `:347-354`。生产者：

- `crates/front/src/by.rs:295`（`ByGoal.ty = render_expr(&nodes[id].ty)`）、`:450`
- `crates/front/src/compile/check/walk.rs:573`、`:860`（generic open exercise）
- `crates/front/src/compile/goals.rs:1073, 1215, 1335, 1437`
- `crates/front/src/compile/check/mod.rs:360`（`by_steps` 的 binder 类型）

### 1.2 实测：同一文件、同一通道，两种拼写并存

`/tmp/pbprobe/proj/ByMain.sokonanoda`（`import lib.Set` + `infix:50 " ⊆ " => Set.subset`）：

```sokonanoda
theorem t (α : Type) (A B : Set α) : A ⊆ B -> A ⊆ A := by
  intro h
  exact fun (x : α) (hx : x ∈ A) => hx
```

| 查询 | 光标 | 结果 |
|---|---|---|
| `query state --line 9 --col 3`（声明行 / 根状态） | `step = -1` | `forall (α : Type 0) (A B : Set α), Set.subset α A B -> Set.subset α A A` ← **点名形式** |
| `query state --line 11 --col 3`（`exact` 行，即 `intro h` 之后） | `step = 0` | `A ⊆ A`，binders `[('α','Type'), ('A','Set α'), ('B','Set α'), ('h','A ⊆ B')]` ← **记法** |

**同一个文件、同一条 `soko/stateAt` 通道，学习者把光标上下移动一格，记法就消失又出现。**

代码根因（唯一）：

- `crates/front/src/query/state.rs:52-66` —— **根状态**（`step: -1`）的 `goal` 取
  `d.ty_text.clone().or_else(|| d.goal.clone())`（`:54-56`），**优先用内核 pp 文本**。
- `crates/front/src/query/state.rs:36-51` —— 没有 `by` 块的声明退回 `d.goal`（前端渲染，✓ 记法）。
- `crates/front/src/query/state.rs:86-92` —— 每条 tactic 之后退回 `by_steps[].goals`（前端渲染，✓ 记法）。

而 `docs/design/notation-subset.md:13-14` 说的「goal 面板显示点名形式」**只对根状态成立**。

### 1.3 泄漏面清单（逐条可查）

| # | 显示通道 | 字段 | 今天的文本来源 | 有记法？ |
|---|---|---|---|---|
| L1 | `soko/stateAt` 根状态（光标在声明行 / 第一条 tactic 之前） | `goal` / `goals[].goal` | `d.ty_text`（`query/state.rs:54-56`） | ✗ |
| L2 | `query goals` / `soko/goals` 的声明签名 | `DeclInfo.ty` | `d.ty_text`（`crates/front/src/query/mod.rs:481`） | ✗ |
| L3 | Infoview 声明列表的类型提示 | `decl.ty` / `decl.ty_runs` | 同 L2（`editor/vscode/media/infoview.js:244-246`） | ✗ |
| L4 | 声明名 hover 的签名 | `d.ty_text`（`crates/lsp/src/lib.rs:1145`） | 内核 pp | ✗ |
| L5 | 补全 documentation | `decl.ty_text`（`crates/lsp/src/lib.rs:1429-1433`） | 内核 pp | ✗ |
| L6 | 表达式 hover（`h : …`） | `HoverType.text`（`crates/lsp/src/render.rs:178-198`，`format!("{} : {}", expr, h.text)` 在 `:195`） | 内核 pp（`check/mod.rs:771`） | ✗ |
| L7 | `#check` / `#reduce` 输出（CLI / REPL / `expr.typed` 事件） | `TypeChecked.text` / `Reduced.text` | 内核 pp（`kernel_phase.rs:340`、`:363`） | ✗ |
| L8 | `query reduce` | `ReduceAnswer.value`（`crates/front/src/query/types.rs:195`） | 内核 pp | ✗ |
| L9 | 诊断消息里的类型（`Sort(0)`、`((And.[] $2) $1)`） | `FailedDecl.message`（`types.rs:162`）、LSP 诊断 | **内核 Debug printer**（`crates/kernel/src/conv.rs:796-820`、`debug_printer.rs:12`），**不是 pp** | 不适用（另一套文法） |
| L10 | `by` 块的 per-tactic goal，但文件里有 `namespace`/`open` | `by_steps[].goals[].ty` | 见 §1.4 —— 被「规范化」成内核文本 | ✗ |
| — | open 练习的剩余目标（无 `by`） | `DeclState.goal` | `render_expr(源 AST)` | ✓ **已回显** |
| — | per-tactic goal（无 `namespace`/`open`） | `by_steps[].goals[].ty` | `render_expr` | ✓ **已回显** |
| — | binder / 假设类型 | `GoalBinder.ty` | `render_expr` | ✓ **已回显** |

> 结论：**要修的是 L1–L8、L10；L9 是另一台打印机（Debug），print-back 碰不到它，
> 也不该碰**（`Sort(0)` 这类文本里没有点名常量可换）。

### 1.4 额外发现：`canonical_goal` 会把 per-tactic goal 也变成点名形式

`crates/front/src/compile/check/walk.rs:100-116` 按单元扫描 `namespace`/`end`/`open`/
`open … in`/`export`，结果进 `canonical_goal`（`:160`、`:288`）。为真时
`crates/front/src/by.rs:139-148` 会调 `canonical_goal_type`（`:89-107`）：
`render_expr(ty)` → `judge_render_type`（内核 pp）→ **`parse_expr_text(&text)` 回读**（`:106`）
→ 之后所有 `by_steps[].goals[].ty` 都带上点名形式。

实测（`/tmp/pbprobe/ns.sokonanoda`，文件里有 `namespace Foo`）：

| 光标 | 无 `namespace` | 有 `namespace` |
|---|---|---|
| `intro` 行（`step = -1`） | `forall (a b : Prop), And a b -> And b a` | 同左 |
| `exact` 行（`step = 0`） | **`b ∧ a`** | **`And b a`** ← 记法丢了 |

课程侧影响：`courses/set-theory/units/*.sokonanoda` 只有 `import`（`unit03-…:28-29`），
不含 `namespace`/`open` ⇒ `canonical_goal = false` ⇒ 单元里的 per-tactic goal **今天不受影响**；
`courses/set-theory/lib/Set.sokonanoda:53` 有 `namespace Set`（但库里的证明都已完成）。

---

## 2. §13.1 的原始论证：它说了什么、没说什么

原文（`docs/design/notation-subset.md:550`）：

> **内核冻结下销不掉**：类型文本由**冻结内核的 pp** 产出（硬规则 1：`crates/kernel/**`
> 一个字节不许动），而记法**不进内核**——内核只看见 `Exists α (fun …)`。要做得二选一：
> ① 改内核 pp（违反硬规则 1）；② 在 elab 保留「源 → 核」的映射、把所有 pp 消费点
> （goal 面板、hover、错误文本、`#check`、`by` 回读）换成源级渲染——那是**两套真相**，
> 且回读路径（§11.9）会立刻分叉。收益（面板好看一点）不抵成本与风险。
> **任务明确要求这一项不做**，故记在这里。

逐句判定：

| §13.1 的断言 | 判定 |
|---|---|
| 「类型文本由冻结内核的 pp 产出」 | **今天只对一部分通道成立**（§1.3：L1–L8、L10；goal/binder 已走前端 `render_expr`） |
| 「记法不进内核」 | ✅ 成立（`crates/front/src/compile/elab.rs` 的 `elab_notation` 只产出 `mk_const`/`mk_app`） |
| ① 改内核 pp ⇒ 违反硬规则 1 | ✅ 成立，`REQUIREMENTS.md` §2 第 1 条 |
| ② 在 elab 保留源→核映射 + 换掉所有 pp 消费点 ⇒ 两套真相 + 回读分叉 | ✅ **这条批评是对的**：`judge.rs:526`、`judge.rs:597`、`by.rs:106` 都是「渲染→回读」的活路径（§6.2） |
| 「收益不抵成本与风险」 | ⚠️ **这是对方案 ② 的性价比判断**，不是对「显示期重写」的判断 |

**关键判断**：§13.1 论证的是「**内核 pp 不能改**」+「**方案 ② 不该做**」。
它**没有论证**「任何形式的显示期重写都不可行」——因为「解析内核文本 → 按记法表渲染 →
只写进显示字段」这个选项**既不改内核、也不在 elab 保留映射、也不经过回读路径**，
三条反对理由一条都不适用。§13.1 提到的「错误文本、`#check`、`by` 回读」这四个消费点里，
**只有前两个是显示面**；`by` 回读恰恰是方案 ③ 明确不碰的。

> 文档债（建议同轮修）：`docs/design/notation-subset.md:13-14`、`:207`（「内核 pp
> **不会**产出记法，所以回读路径不受影响」）、`:242`、`:539` 与
> `docs/architecture.md:145`（「源码级 print-back（内核冻结下销不掉）」）都写成
> 「goal/hover 全部点名形式」，与 §1.3 的实测不符。§13.1 应改写为
> 「方案 ① ② 不做；显示边界重写见 `docs/notes/course-lean-style/printback-feasibility.md`」。

---

## 3. 问题 1：内核文本今天是否已经被解析回 AST？往返的失败模式？

### 3.1 是——三处生产路径

| 位置 | 做什么 | 失败行为 |
|---|---|---|
| `crates/front/src/judge.rs:524-534` | `judge_infer` 取 `TypeChecked.text` 后，`parse_expr_text(&t)`（`:526`）→ `peel_one_binder`（`:529`，定义 `:539-555`）→ `render_roundtrip`（`:530`，定义 `:559-575`），逐层剥掉合成的 `fun (b:T) => …` | `let Ok(e) = … else { break }`（`:526-528`）⇒ **静默 break**，返回未剥净的原始 pp 文本 |
| `crates/front/src/judge.rs:589-600` | `judge_render_type`：`judge_infer` → `parse_expr_text(&text)`（`:597`）→ 剥一层 → `render_roundtrip` | `.ok()?` ⇒ 调用方退回源 AST |
| `crates/front/src/by.rs:89-107` | `canonical_goal_type`：`render_expr(ty)` → `judge_render_type` → `parse_expr_text(&text)`（`:106`） | `.unwrap_or_else(\|_\| ty.clone())` ⇒ 退回源 AST |

另有 `crates/front/src/compile/goals.rs:507-516`（`peel_pi_domain_text`，`:508`
`parse_expr_text(ty)`）、`crates/front/src/suggest.rs:242, 448, 465`、
`crates/lsp/src/lib.rs:813, 852`。

**这证明**：内核 pp 文本对本语言的 parser 是**可解析的**（`forall (a b : Prop), …`、
多 binder 折叠、`Type 0`、换行都能吃下——实测见 §10.3）。

### 3.2 往返的已知失败模式

| # | 失败模式 | 证据 |
|---|---|---|
| F1 | **pp 会折叠相邻 binder**（`forall (a b : Prop), …`），逐层剥离必须逐**单**个 binder 剥 | `crates/front/src/judge.rs:521-523`；`crates/front/src/spine.rs:26-27` |
| F2 | **pp 文本里的换行**：长签名 pp 会折行（实测 `Set.subset` 那条 `ty` 含 `\n`） | §10.2 实测；parser 容忍（§10.3） |
| F3 | **松散变量 `$N`**：pp 把「binder 在被打印项之外」的变量渲染成 `$N`，前端只能靠 `scope_names` 猜回名字 | `crates/front/src/compile/check/mod.rs:797-804`（`name_loose_bvars`）；`crates/lsp/src/render.rs:187` 见到 `$` 就退回源码切片 |
| F4 | **`@Eq.{u, v}` 形态的显式宇宙头** 必须被记法匹配认出来（head 是 `Expr::UniverseApp` 而不是 `Expr::Ident`） | `crates/front/src/proof.rs:239-241`；往返测试 `crates/front/src/compile/tests.rs:1487` |
| F5 | **`Expr::Forall` 作箭头 domain 必须补括号**（否则右结合误读、telescope 被腐蚀）——历史 bug | `crates/front/src/proof.rs:255-266`（注释里写明「judge_infer 的 render→parse 往返因此腐蚀 telescope」）；测试 `crates/front/src/compile/tests.rs:1488-1492`；HANDOVER §3 A 记的 Arrow domain 补括号修复即此 |
| F6 | **括号清单漏项**：`by.rs::atom_text` 曾自维护一份「哪些形状要补括号」，漏了 `Let`/`Match`/`Notation`，`Eq.refl.{1} (Set α) (Aᶜ) ∪ B` 被读成 `(Eq.refl.{1} (Set α) Aᶜ) ∪ B` | `crates/front/src/by.rs:532-539`（注释「括号规则只允许有一个实现」）；`docs/design/notation-subset.md:458-462` |
| F7 | **回读时没有记法表**：`by` 块目标 `render_expr` 成文本再 `parse_expr_text` 回读，而回读片段里没有记法声明 ⇒ 第二刀起把前缀的记法表当继承表喂进去 | `docs/design/notation-subset.md:482-486`；`crates/front/src/judge.rs:247-271, 316` |
| F8 | **`judge_infer` 的回读不带记法表**：`parse_expr_text`（`:526`）而不是 `parse_expr_text_with` ⇒ **今天这是对的**（它读的是 pp 文本，不含记法）；一旦把 print-back 插到它上游，这里立刻静默 break（F 类里最危险的一条） | `crates/front/src/proof.rs:45-47`（无继承表）vs `:53-63`（有） |
| F9 | **`suggest.rs` 仍有一份自己的括号清单**，缺 `Notation`/`SetLiteral`（与 `proof.rs:437-438` 不一致） | `crates/front/src/suggest.rs:498-511` |

**往返稳定性今天有测试**：`crates/front/src/compile/tests.rs:1474` `render_expr_round_trips`
（`:1494-1501` 断言 parse→render→parse→render 稳定）。注意用例 `("And a b", "And a b")`
（`:1482`）——**它恰好钉住了「App 形状不会被自动回显成记法」这个今天的事实**；
做 print-back 时这条用例的语义要重新表述（App 形状本身仍是稳定的，回显是**另一层**函数）。

---

## 4. 问题 2：最小可行方案

### 4.1 一句话

新增一个**纯函数**：`print_back(text, &记法表, &元数表) -> String`，
**只在文本写进显示字段的前一刻**调用；任何一步失败都**原样返回输入**。

### 4.2 数据结构

```rust
// 新模块 crates/front/src/notation.rs（或 display.rs）
pub struct DisplayNotations {
    /// 生效的记法表：文件内声明 + 沿 import 边传播来的（含 scoped 过滤）
    table: Vec<NotationDecl>,
    /// target 点名 → telescope 层数（前向展开的 arity 依据）
    arity: HashMap<String, usize>,
}
```

- **`NotationDecl`**（`crates/front/src/ast.rs:153-163`）已经有 `symbol` /
  `precedence: Option<u16>` / `assoc` / `target` / `scope`——**打印所需的全部信息都在里面**，
  不需要新结构。
- **记法表的取得**（回答「怎么拿到当前文件的记法表」）：
  `QueryDoc::project_modules()`（`crates/front/src/query/mod.rs:229-233`）返回
  `&[ModuleReport]`，而 `ModuleReport` 带 **`source: String`**（编译时看到的那份文本）
  与 `imports: Vec<String>`（`crates/front/src/project/report.rs:90-105`），
  **拓扑序、入口在最后**。于是：
  `parse(source)`（`crates/front/src/parser.rs`）→ 逐条 `Command::notation_decl()`
  （`crates/front/src/ast.rs:548-568`）→ 按 import 边合并（复用
  `crates/front/src/project/graph.rs:119-129` 的 `absorb_notations` 规则，需从 `fn`
  改成 `pub(crate)`），即可**纯源码、零内核**重建有效表。
  单文件模式没有 `ModuleReport` 时退回 `parse(self.text)` 一份。
- **arity 的取得**：`NotationDecl.target` 是点名（`Set.mem` / `And` / `Not` / `Eq`），
  在「闭包所有模块的声明 AST + prelude 源码」里找同名声明，数它的 telescope 层数
  （`Expr::Forall`/`Expr::Arrow` 逐层剥，`crates/front/src/spine.rs:21-52` 的 `peel_pi`
  就是现成的）。prelude 源码是 `pub(crate) const PRELUDE_L1_SRC`
  （`crates/front/src/compile/prelude.rs:195`）与 `PRELUDE_EQ_SRC`（`:160`），
  里面就是 `inductive And (a b : Prop) : Prop`（`:201`）、`def Not (A : Prop) : Prop`（`:212`）、
  `def Iff (A B : Prop) : Prop`（`:216`）、`axiom Eq {u} : {α : Sort u} -> α -> α -> Prop`（`:161`）
  ——**可以 parse 一次缓存在 `OnceLock` 里**。
  兜底（找不到声明时）：`judge::judge_type_of`（`crates/front/src/judge.rs:358-370`，
  自带 `type_cache`，FIFO 128）。

  > **为什么 arity 必须对**：前向展开的规则是「操作数对齐到 telescope 的**最后**
  > `operands.len()` 层」（`crates/front/src/compile/elab.rs:1200-1212`），
  > 所以**只有 `spine.len() == arity` 的完全应用才是记法实例**。
  > `Set.mem α a`（部分应用，2 个实参 / arity 3）如果被回显成 `α ∈ a` 就是**显示错误**。
  > 反过来，arity 对了，print-back 就是前向展开的**精确逆**（同一份权威：前向也用
  > `judge_type_of` 读签名，见 `crates/front/src/compile/elab.rs:1194-1197`）。

### 4.3 算法

```
fn print_back(text, dn) -> String:
    if text 含 '$'                    -> return text          // F3
    let Ok(ast) = parse_expr_text_with(text, &dn.table) else { return text }   // F2/F4 容忍
    let out = rewrite(ast, dn)
    render_expr(&out)

fn rewrite(e, dn) -> Expr:
    递归到每个 App spine (head, args)   // spine_of，crates/front/src/spine.rs:55-64
    if head 是 Ident{name} 或 UniverseApp{name}:
        for decl in dn.table where decl.target == name:      // 重载：按 head 精确选，天然无歧义
            if dn.arity[name] == args.len():
                n = 操作数个数（Infix/Infixl/Infixr = 2，Prefix/Postfix = 1，Nullary = 0）
                k = args.len() - n
                return Expr::Notation { symbol: decl.symbol, assoc: decl.assoc,
                                        lhs/rhs = args[k..], target: decl.target,
                                        alternatives: vec![], span: default }
    return e
```

要点：

- **重载不是问题**（与 §14.2 相反的方向）：前向是「符号 → 候选目标，按期望类型选」，
  有歧义；**反向是「目标 → 符号」，head 名字就是判据，天然单值**。
  唯一残留歧义是「同一 target 声明了两个不同符号」（`∈` 与 `∊` 都 => `Set.mem`）：
  **取声明顺序第一个**（`Vec` 顺序），写进文档 + 一条测试。
- **括号**：`render_expr` 的既有规则是**保守补括号**——`render_atom`
  （`crates/front/src/proof.rs:427-441`）与 `render_fun_position`（`:407-420`）
  把任何 `App`/`Notation`/`Arrow`/`Lambda` 一律括起来。实测：
  `Or (And a b) c` → `(a ∧ b) ∨ c`，`And a (Or b c)` → `a ∧ (b ∨ c)`，
  `Arrow(Or(And a b) c, …)` → `((a ∧ b) ∨ c) -> …`。
  **永远不会少括号**（只会多），所以**不存在 §5 里担心的优先级歧义**；
  代价是比 Lean 略啰嗦（Lean 会打 `a ∧ b ∨ c`）。
- **binder 记法（`∃`/`∀`）**：pp 形状是 `Exists α (fun (x : α) => p x)`。
  反向需要「认出 head == `Exists`、arity 2、第 2 个实参是 `Expr::Lambda`」⇒ 构造
  `Expr::Notation { assoc: Binder, rhs: Some(lambda) }`，`render_expr` 会走
  `render_binder_notation`（`crates/front/src/proof.rs:361-371`）打出 `∃ x, p x`。
  **两段式**（`∃ x ∈ s, p` ⇒ `Exists α (fun (x:α) => And (x ∈ s) p)`，
  见 `docs/design/notation-subset.md:574-576`）反向要额外认 `And guard body` 的形状——
  属**分期 P1**，v1 只做一段式。

### 4.4 落点（哪里调用）

| 落点 | 改什么 | 覆盖的泄漏 |
|---|---|---|
| `crates/front/src/query/mod.rs:481` | `ty: d.ty_text.clone()` → 过 print-back | L2 L3 |
| `crates/front/src/query/mod.rs:402-418`（`goal` 闭包）与 `:420+` 的 `StateAnswer` 装配 | `goal` / `goals[].goal` 过 print-back（`ty_runs`/`goal_runs` 自然跟着变，因为 `self.runs(...)` 在 `:406`/`:413` 消费同一个字符串） | L1 L10 |
| `crates/front/src/query/mod.rs` 的 `reduce` 出口 | `ReduceAnswer.value` | L8 |
| `crates/lsp/src/render.rs:178-198`（`expr_hover`） | `h.text` 过 print-back。**一处覆盖两条 hover 路径**：精确命中（`crates/lsp/src/lib.rs:1114`）与邻近回退（`:1140`） | L6 |
| `crates/lsp/src/lib.rs:1145` / `:1429-1433` | 声明签名 / 补全 documentation（`format!("{} {} : {}", kind, name, ty)` 里的 `ty`） | L4 L5 |

> **记法表怎么到 LSP 层**：新增 `QueryDoc::display_notations() -> DisplayNotations`
> （`crates/front/src/query/mod.rs`，内部走 §4.2 的纯源码重建），
> LSP 侧 `doc.query().display_notations()` 一次取用——**不要在 `crates/lsp` 里重新解析源码**
> （否则又造一套真相）。

**明确不落**（红线）：

- `crates/front/src/compile/**`（`DocumentReport` / `DeclState` 一字不改）
- `crates/front/src/judge.rs`、`by.rs`、`proof.rs`、`spine.rs`、`suggest.rs`
- **`CheckEvent::TypeChecked` / `Reduced`（即 `grade --json` 的 `expr.typed` 事件）**
  ⇒ L7 里「CLI/REPL 的 `#check` 显示」留给 P1，且必须走**另一条**只给显示用的出口，
  **绝不改事件流**。

> **不碰 `DeclState` 是刻意的**：`crates/front/src/suggest.rs:410-431`
> （`hole_goal_text` / `open_spec`）**拿 `d.goal` 与 `d.binders[].ty` 去合成判定规格**
> （`suggest.rs:157-174` 调 `judge_terms_with`），而 `judge_terms_uncached` 会
> `parse_expr_text_with(&open.ty, &notations)`（`judge.rs:271`）——
> **显示字段已经在喂判定路径**。把 print-back 写进 `DeclState` 就会把这两条路搅在一起。

### 4.5 结构性护栏（把「不碰回读」变成编译错误）

让 print-back 返回**新类型**而不是 `String`：

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplayText(String);       // 没有 Deref<Target = str>，没有 as_str()
impl DisplayText { pub fn as_display_str(&self) -> &str { &self.0 } }
```

于是 `parse_expr_text(&display_text)` **编译不过**。这比注释/review 可靠，
也符合本仓库「括号规则只允许有一个实现」（`by.rs:534`）的既有纪律。

---

## 5. 问题 3：坑（逐条）

| # | 坑 | 现状/判据 | 处置 |
|---|---|---|---|
| P1 | **`→` 不是记法能表达的东西**：`->` 是词法 token（`crates/front/src/token.rs:281-295`），没有对应常量，`infix:25 " → " => ?` 无处可指 | 记法只产出 `mk_const`/`mk_app`（`docs/design/notation-subset.md:108`）⇒ 目标必须是常量 | **需要词法别名**，与 `∀` 完全同款：`crates/front/src/token.rs:277-279`（`'∀' => self.single(TokenKind::Forall, start)`）。这是**拼写增量**（真 Lean 4 子集），不是记法 ⇒ 要走「课程 + 测试 + 白名单三件套」 |
| P2 | **裸 `=` 今天在词法层就是硬错误**：`'='` 只可能是 `=>`，否则 `err_unexpected(start, "expected \`=>\`", "=")` | `crates/front/src/token.rs:296-308`；回归测试 `crates/front/src/token.rs:759`。实测：`theorem t (n : Nat) : n = n := by …` ⇒ `unexpected-token: expected \`=>\`, found =`（**未声明任何记法也一样**） | **`=` 不是「加一条 `infix` 就行」，而是一条独立的词法工作项**：`'='` 后不接 `>` 时要产出新 token（或交给声明驱动的符号表）。课程今天一律写点名 `Eq.{1} α a b`（如 `courses/set-theory/units/unit04-extensionality-identities.sokonanoda` 15 处）——`=` 只出现在 `--` 注释里 |
| P3 | `¬`（U+00AC）、`→`（U+2192）、`↔`（U+2194）**不在** `is_math_symbol` 的码点类里（`crates/front/src/token.rs:474-476`：只有 U+2200–22FF / U+2A00–2AFF / `\`），它们是**标识符字符**（`crates/front/src/token.rs:454-456`） | 但 `is_valid_notation_symbol`（`token.rs:497-501`）只拒空串 / 纯 ASCII 词 / 含 `∀ - # "`，**不要求是数学符号**；声明驱动的词法会把它们收进符号表 | ✅ **实测可用**（`§10.3`）。但要**加测试钉住**（P3 是最容易被后人「顺手收紧」的地方） |
| P4 | **`∀` 已经是 token**（`token.rs:279`），不能当记法符号（实测报 `notation-shape`，`§10.3`） | — | 打 `∀` 要靠 `render_expr(Expr::Forall)` 改成打 `∀ (a b : Prop), …`（**渲染器改动**，与记法无关）；v1 可以继续打前端既有的 `(a : Prop) (b : Prop) -> …` |
| P5 | **pp 文本的形态与前端渲染不同**：`forall (a b : T), X` → `(a : T) (b : T) -> X`；`Type 0` → `Type`（`crates/front/src/proof.rs:228-237` 打 `Type`，parser 两个拼写都收，`crates/front/src/parser.rs:2272`）；pp 的折行会被压平 | 实测 `§10.2` | **这是 print-back 的副产品，不是 bug**，但**必须在协议文档里写明**（否则消费者以为只换了符号）。`render_expr_round_trips`（`crates/front/src/compile/tests.rs:1474`）要扩用例 |
| P6 | **`$N` 松散变量**：`HoverType.text` 可能含 `$` | `crates/front/src/compile/check/mod.rs:797-804`；`crates/lsp/src/render.rs:187` 已有守卫 | print-back 见到 `$` **原样返回** |
| P7 | **pp 折行**：实测 `ty` 里含字面 `\n` | `§10.2` | parser 容忍（`§10.3`）；输出被 `render_expr` 压成单行——可接受，但要有一条测试 |
| P8 | **作用域是「文件内 + 声明之后」**（`docs/design/notation-subset.md:121-133`），而 print-back 只能用「文件结束时的表」 | 位置敏感的表需要 span→表映射 | v1 用**文件末表**（超集）：可能显示一个在**该点之后**才声明的符号。**写进文档 + 一条测试**；Lean 的 pp 是位置敏感的，这里**明说有差异** |
| P9 | **跨 import 传播**：`lib/Set.sokonanoda:155-159` 声明 5 个符号，单元里 `import lib.Set` 就能用（`docs/design/notation-subset.md:522-525`） | `crates/front/src/project/graph.rs:90-92, 364-375, 409+` | 记法表必须**沿 import 边**合并，**不能**用「闭包里所有模块的并集」（`docs/design/notation-subset.md:476`：「记法表按 import 边合并（不是"闭包里全局"）」） |
| P10 | **`scoped` 记法**：默认不生效，要 `open scoped Foo`（`docs/design/notation-subset.md:606-608`） | 前向在 `judge.rs:247-270` 按「前缀里有没有 `open scoped`」过滤 | 显示表用**同一套过滤**，否则会显示一个学员写不出来的符号 |
| P11 | **重载**：同符号多目标（`docs/design/notation-subset.md:588-600`） | 反向按 head 精确选 ⇒ **无歧义** | 唯一残留：同 target 多符号 ⇒ 取声明顺序第一个 + 测试 |
| P12 | **arity 错了就是显示错误**（`Set.mem α a` → `α ∈ a`） | `crates/front/src/compile/elab.rs:1200-1212` | 只回显 `spine.len() == arity` 的**完全应用**；arity 表建不出来时**整条记法不参与**（宁可不回显） |
| P13 | **性能**：arity 若走 `judge_type_of` 就是每个 target 一次全前缀编译（`type_cache` FIFO 128，key 含 `prefix_src` ⇒ 每次编辑全失效） | `crates/front/src/judge.rs:81`（`JUDGE_CACHE_CAP`）、`:358-370` | **arity 走纯源码**（闭包 `ModuleReport.source` + prelude 源码，`OnceLock` 缓存），**显示路径零内核调用**。这是本方案能做到「不改内核热路径」的关键 |
| P14 | **`soko/stateAt` 在光标移动路径上**（不是 hover 那种按需路径） | `crates/lsp/src/lib.rs:578-595` | 同上：print-back 必须是 O(文本长度) 的纯函数，且**结果可缓存**（按 `(version, field)`）；实测文本都很短（几十字符） |
| P15 | **循环/递归**：print-back 的输出**不能**再喂回 print-back（会怎样？） | — | `print_back` 是**幂等**的（输出是记法文本，再 parse 得 `Expr::Notation`，`rewrite` 不动它）⇒ 加一条幂等测试 |
| P16 | **`suggest.rs:498-511` 的括号清单已经与 `proof.rs:437-438` 不一致**（缺 `Notation`/`SetLiteral`） | 既有技术债 | **不在本轮修**，但登记；print-back 上线后 `sub_goals[].ty` 可能带记法，会放大这条债 |

---

## 6. 问题 4：风险判断

### 6.1 硬规则

| 硬规则 | 判定 | 理由 |
|---|---|---|
| **内核冻结**（`REQUIREMENTS.md` §2 第 1 条；`crates/kernel/**` 一字节不动） | ✅ **不违反** | print-back 全部在 `crates/front`（显示侧）与 `crates/lsp`；内核只被**读文本**，不被改 |
| **判定永远走 kernel，禁止文本比对**（§2 第 4 条） | ✅ **不违反** | print-back 是**纯格式化**，不参与任何裁决；判定仍走 `judge_terms`/`judge_infer`。且 §4.5 的 `DisplayText` 新类型让「显示文本进 parser」变成**编译错误** |
| **教学语法是真实 Lean 4 子集**（§2 第 3 条） | ✅ **不违反** | print-back 只输出**该文件自己声明过的**记法符号（学员写得出），加上既有 token。`→`（P1）要先做词法别名才允许输出——**顺序不能反**：先让学员能写，再回显 |
| **记法是例外面，不引入新语义**（G-04/0.59.0） | ✅ **不违反** | print-back 不产生事件、不进声明表、不改计数；`grade --json` 的**事件流逐字节不变**（§7.2 的三层测试钉死） |
| **`grade --json` 是判卷契约** | ✅ **不违反** | 只改 `query` / LSP 的**显示字段**；`CheckEvent` 一字不改 |

### 6.2 「判卷结果与显示不一致」的风险 —— 真实，且可定位

**危险模式**：如果 print-back 被插到「渲染 → 回读」的**上游**，回读会因为
**没有记法表**而失败，而失败是**静默降级**的：

| 站点 | 今天的失败行为 | 被污染后的后果 |
|---|---|---|
| `crates/front/src/judge.rs:526`（`judge_infer` 剥层） | `let Ok(e) = … else { break }` | 停止剥 binder ⇒ 返回**带 binder 层的类型** ⇒ `by` 的 `apply` 文本对齐（`unify_spine`，`crates/front/src/spine.rs:134-157`）错位 ⇒ **假报「类型不匹配」** |
| `crates/front/src/judge.rs:597`（`judge_render_type`） | `.ok()?` → `None` | `canonical_goal_type` 退回源 AST（`crates/front/src/by.rs:104`）⇒ 丢掉 G-05 的短名规范化 ⇒ 用 `namespace`/`open` 的文件的 `apply` 假报不匹配 |
| `crates/front/src/by.rs:106`（`canonical_goal_type`） | `.unwrap_or_else(\|_\| ty.clone())` | 同上 |
| `crates/front/src/proof.rs:45-47`（`parse_expr_text`，**无**继承表） | `Err(ProofError::Parse)` | 上层 `?` 或 `.ok()?` ⇒ 功能静默消失 |

**结论**：风险**不是**「print-back 本身危险」，而是「**放错位置**危险」。
只要坚持 §4.4 的落点表 + §4.5 的新类型护栏 + 一条「`print_back` 的调用者白名单」
测试，这条风险就被消掉。**判卷语义一个字节不变**是可达的，且可被 §7.2 的测试证明。

### 6.3 其它风险

| 风险 | 等级 | 缓解 |
|---|---|---|
| 显示文本变化导致既有测试/golden 红 | 中 | 只有**声明了记法**的文件才会变；`crates/cli/tests/query.rs:159` 的 `"And b a"` 断言所用 canvas 无记法 ⇒ 不受影响；逐个跑 `scripts/soko gate` |
| 学员看到的符号写不出来（位置敏感的 P8） | 低 | 用文件末表（超集）；文档明说 |
| 编译缓存失效 | 低 | **不改 `DocumentReport`** ⇒ 不必 bump `CACHE_FORMAT`（`crates/front/src/compile/cache.rs:23` = 2）；记法表在显示期从 `ModuleReport.source` 现算 |
| arity 找不到（目标在别处、或名字解析有歧义） | 低 | 找不到 ⇒ 该记法**不参与**回显（宁缺点名形式，不给错文本） |
| 性能 | 低 | 纯源码 + `OnceLock` + 按 `(version, field)` 记忆化 |

---

## 7. 问题 5：量级与分期

### 7.1 改动清单（最小可用 = P0）

| 文件 | 改动 | 行数（估） |
|---|---|---|
| `crates/front/src/notation.rs`（**新建**） | `DisplayNotations`（表 + arity 表，纯源码）+ `print_back` + `rewrite` + 文档 | **220–280** |
| `crates/front/src/lib.rs` | `pub mod notation;` + re-export | 2 |
| `crates/front/src/project/graph.rs:119` | `absorb_notations` → `pub(crate)`（或抽成公共 helper） | 1–15 |
| `crates/front/src/query/mod.rs` | `display_notations()` 助手（~25）+ `:481`、`:402-418`、reduce 出口三处调用 | 30–40 |
| `crates/lsp/src/render.rs:178-198` | `expr_hover` 过 print-back | 5–10 |
| `crates/lsp/src/lib.rs:1145` / `:1429-1433` | 声明签名 / 补全 documentation | 5–10 |
| `crates/front/src/compile/cache.rs` | **不动**（`CACHE_FORMAT` 保持 2） | 0 |
| **合计** | 1 个新模块 + 5 个文件 | **≈ 270–360 行 Rust** |

### 7.2 三层测试怎么落

**第 1 层（front 单测，`crates/front/src/notation.rs` 的 `mod tests`）** ~10 条：

1. `table_merges_along_import_edges` —— 用 `lib/Set` 那 5 条 + 一个不 import 的模块，
   断言后者的记法**不泄漏**（对应 `docs/design/notation-subset.md:476`）。
2. `table_honours_open_scoped` —— `scoped` 记法未 `open scoped` 时**不进**显示表（P10）。
3. `arity_comes_from_source_and_prelude` —— `Set.mem` = 3、`And` = 2、`Not` = 1、`Eq` = 3，
   **零内核调用**（断言 `judge_cache_len()` 不变）。
4. `partial_application_is_not_printed_back` —— `Set.mem α a` **原样返回**（P12）。
5. `infix_prefix_postfix_nullary_all_round_trip` —— `a ∈ A` / `𝒫 A` / `Aᶜ` / `∅`。
6. `overload_picks_by_head` —— `∧ => And` + `∧ => Or`：`And a b` → `a ∧ b`，`Or a b` → `a ∧ b` 的**兄弟候选**不误选。
7. `same_target_two_symbols_first_wins` —— 声明顺序（P11）。
8. `unparseable_and_dollar_text_is_returned_verbatim` —— `$1`、`And.[]`、乱码（P6）。
9. `print_back_is_idempotent` —— `print_back(print_back(x)) == print_back(x)`（P15）。
10. `parens_never_under_parenthesize` —— `Or (And a b) c` → `(a ∧ b) ∨ c` 且 **re-parse 结构相同**。
11. 扩 `crates/front/src/compile/tests.rs:1474` `render_expr_round_trips`：补 pp 形态
    （`forall (a b : Prop), …`、含 `\n`、`Type 0`）的用例（P5/P7）。

**第 2 层（CLI e2e，`crates/cli/tests/`）** ~3 条：

12. **`grade --json` 逐字节不变**（最重要的一条）：同一份带记法的 `by` 文件，
    在 print-back 打开/关闭两种构建下 `grade --json` 的事件流**完全相同**
    （五元计数 + 每条事件的 `text`/`inferred_type`）；写法沿用
    `crates/cli/tests/notation.rs` 既有的「两种写法计数逐一相等」范式。
13. `query goals` 的 `ty` 与 `query state` 根状态的 `goal` 含 `⊆`/`∈`，
    且与 per-tactic 状态的 `goal` **拼写一致**（直接钉死 §1.2 那条实测缺陷）。
14. `query check` / `query project` 的计数与 `grade --json` 仍逐一相等
    （`crates/cli/tests/query.rs` 既有一致性契约）。

**第 3 层（LSP + 扩展）** ~3 条：

15. `crates/lsp/src/tests/hover.rs`：表达式 hover 文本含记法；
    与 `stateAt` 的 `goal_runs` 文本投影**同源**（沿用
    `hover_goal_text_equals_the_state_at_run_projection` 的思路）。
16. `crates/lsp/src/tests/state.rs`：`soko/stateAt` 根状态与 step 0 的 `goal` 拼写一致。
17. `node editor/vscode/test-extension-host.js`：**扩展侧零改动**——
    断言 Infoview 仍只做 `textContent`（`editor/vscode/media/infoview.js:170/180-184/244-246`），
    证明这层重写**不在客户端**。

**门禁**：`scripts/soko gate`（fmt + clippy + test + playground anchor + 课程门禁 +
缺口台账）必须全绿；`python3 courses/set-theory/tools/check.py` 的
**36 目标 · 329 checked · 99 open · 0 判负** 必须**逐项相同**（显示期改动 ⇒ 计数不该动）。
另加**调用者白名单测试**（grep `print_back` 的调用点，只允许 §4.4 列出的 5 个文件）。

### 7.3 分期

| 期 | 内容 | 覆盖 | 量级 |
|---|---|---|---|
| **P0 最小可用** | `notation.rs` + 纯源码 arity + `query/mod.rs`（`DeclInfo.ty`、`StateAnswer`）+ `lsp/render.rs::expr_hover` + `lsp/lib.rs` 签名/补全 | L1–L6（**Infoview 根状态、声明签名、表达式 hover、`query goals/state`**）——**课程 90% 的观感问题** | 1 天 |
| **P1** | 一段式 binder 记法反向（`Exists α (fun (x:α) => p x)` → `∃ x, p x`）；`#check`/`#reduce` 的**显示**出口（不动事件流）；两段式 guard（`∃ x ∈ s, p`） | L7 L8 + `∃` | +0.5 天 |
| **P2** | 内建拼写：`→` 词法别名（`token.rs:279` 同款）+ `render_expr(Arrow)` 打 `→`；`∀` 打 `∀ (a b : T), …`；`Eq` → `=`（先解 `=`/`=>` 的冲突，P2 坑）；集合字面量 `{a, b}` 反向（`Set.singleton`/`Set.pair`）；位置敏感的记法表（P8） | 课程「全面数学符号」的**完整**目标 | +1.5–2 天，**且 `→` 那条要走语法三件套** |
| **P3（可选）** | `suggest.rs:498-511` 的括号清单去重（P16）；`notation3`/依赖 binder 的回显 | 技术债 | — |

---

## 8. 问题 6：如果决定「不做」

如果最终判定不划算，**替代方案按性价比排序**：

| 替代 | 内容 | 量级 | 覆盖 |
|---|---|---|---|
| **T0-a** | **把根状态改成「源级签名的渲染」**：给 `DeclState` 加一个 `ty_src: Option<String>`（= 走查时 `render_expr(源 ty)`，需要把源 `ty` 补进 `PendingOp::OpenExercise`，`crates/front/src/compile/check/mod.rs:44-83`），`crates/front/src/query/state.rs:54-56` 优先取它 | **~25 行 / 2 文件** | L1 L2 L3 L4 L5 —— **消掉「光标一动记法就消失」这个最刺眼的缺陷**，且**完全不解析内核文本** |
| **T0-b** | `canonical_goal_type` 的回读带记法表（`crates/front/src/by.rs:106` → `parse_expr_text_with(&text, &notations)`，notations 需从 `run_by` 传进来） | ~15 行 | L10（`namespace`/`open` 文件的 per-tactic goal） |
| **T1** | 课程里**两种写法并排**：`courses/set-theory/units/notation-cheatsheet.sokonanoda:86-89` 已经在做（`∈ ⊆ ∪ ∅` 的对照页）；扩到 `∧ ∨ ↔ ¬ ∃`，每个练习给「记法版 / 点名版」两行 | 课程内容 | 学员**学得到**符号，但面板仍然显示点名形式 |
| **T2** | 只在**扩展侧**做有限文本替换 | — | ❌ **不推荐**：`editor/vscode/**` 里对 goal/type 文本**零替换**是既有契约（`docs/design/goal-rendering.md:12`「客户端不"重新字符串高亮"」；`docs/design/highlighting.md:21-29`「分类与文本只有一个出处」），在 TS 里重写会**造出第三套真相**，且 opencode/DSH/CLI 拿不到 |

> **T0-a 的取舍（必须明说）**：`ty_text` 存在的理由是「内核渲染的声明签名」——
> 命名空间里的短名会被规范化成全名（`mem` → `A.mem`，见
> `crates/front/src/judge.rs:577-588` 与 `docs/design/namespace-open.md` §4.6）。
> 换成源级渲染会**丢掉这个规范化**。对本课程（单元只有 `import`、不用 `namespace`/`open`）
> 无损；对用 `namespace` 的文件，签名 hover 会显示学员写的那份拼写（其实更贴合直觉）。
> 若两条都要，就退回 P0（保留 `ty_text`，在显示边界重写它）。

---

## 9. 验收（本方案的 DoD）

1. `python3 courses/set-theory/tools/check.py` → **36 目标 · 329 checked · 99 open · 0 判负**，逐项与改前相同；
2. `grade --json` 对同一文件的**事件流逐字节相同**（含 `expr.typed` 的 `text`/`inferred_type`）；
3. `query goals` 的 `ty`、`query state` 根状态的 `goal`、per-tactic 状态的 `goal`
   **三处拼写一致**（用带 `⊆`/`∈` 的 `by` 文件断言）；
4. hover（表达式 / 声明签名 / 补全 documentation）含记法；
5. `scripts/soko gate` 全绿（含课程门禁 + 缺口台账门禁）；
6. `node editor/vscode/test-extension-host.js` 全绿且**扩展零改动**；
7. 调用者白名单测试绿（`print_back` 只在 §4.4 的 5 个文件里被调用）；
8. 文档同步：`docs/design/notation-subset.md` §13.1 改写（区分方案 ①②③）、
   `docs/architecture.md:145`、`docs/protocol.md`（**写明显示文本会被规范化**：
   符号替换 + `forall`→命名箭头 + `Type 0`→`Type` + 折行压平）、
   `docs/TESTING.md` 加守护行、`STATUS.md`。

---

## 10. 复现命令（本次调研实测）

### 10.1 环境

```bash
cd /Users/penglingwei/Documents/lean/sokonanoda/sokonanoda-lang
BIN=target/debug/sokonanoda      # 仓库已有构建；不调用 lean/lake/elan
$BIN --version
```

### 10.2 记法回显 / 根状态泄漏（§1.2）

```bash
mkdir -p /tmp/pbprobe/proj/lib
cp courses/set-theory/lib/*.sokonanoda /tmp/pbprobe/proj/lib/
cp courses/set-theory/sokonanoda.toml /tmp/pbprobe/proj/
cat > /tmp/pbprobe/proj/ByMain.sokonanoda <<'EOF'
import lib.Set
infixr:35 " ∧ " => And
infix:50 " ∈ " => Set.mem
infix:50 " ⊆ " => Set.subset

theorem t (α : Type) (A B : Set α) : A ⊆ B -> A ⊆ A := by
  intro h
  exact fun (x : α) (hx : x ∈ A) => hx
EOF
cd /tmp/pbprobe/proj
$BIN grade --json ByMain.sokonanoda                                  # → decl.checked
$BIN query state --file ByMain.sokonanoda --line 9  --col 3          # step -1 → 点名形式
$BIN query state --file ByMain.sokonanoda --line 11 --col 3          # step  0 → A ⊆ A
$BIN query goals --file ByMain.sokonanoda                            # ty=内核 pp / goal=前端渲染
```

实测输出（节选）：

```
step= -1 goal= 'forall (α : Type 0) (A B : Set α), Set.subset α A B -> Set.subset α A A'
step=  0 goal= 'A ⊆ A' binders= [('α','Type'),('A','Set α'),('B','Set α'),('h','A ⊆ B')]
```

### 10.3 符号可声明性（§5 P1/P2/P3/P4）

```bash
# 逐符号：infixr:35 " <sym> " => And   +   theorem t (a b : Prop) : a <sym> b := sorry
# ¬ U+00AC  → exercise.open   ✅      → U+2192 → exercise.open ✅（但无常量可指，P1）
# ↔ U+2194  → exercise.open   ✅      ∧ U+2227 → exercise.open ✅
# ∀ U+2200  → notation-shape（是关键字）❌
```

### 10.3b 裸 `=` 是词法硬错误（§5 P2 —— **直接影响主线 L2.4**）

```bash
printf 'theorem t (n : Nat) : n = n := by\n  exact Eq.refl.{1} Nat n\n' > eq.sokonanoda
$BIN grade --json eq.sokonanoda
# → {"code":"unexpected-token","message":"expected expected `=>`, found =", ...}
#   未声明任何记法也一样 ⇒ 不是「记法冲突」，是 token.rs:296-308 的 '=' 分支只认 '=>'

$BIN grade --json <(printf 'infix:50 " = " => Eq\n')   # → 同样失败（声明行自己的 => 被 '=' 抢掉）
$BIN grade --json <(printf 'infix:50 " ≃ " => Eq\ntheorem t (n : Nat) : n ≃ n := sorry\n')
# → exercise.open ✅（说明「符号 => Eq」这条路本身通，卡点只在 '=' 这个字符）
```

课程侧对照：`courses/set-theory/units/unit04-extensionality-identities.sokonanoda`
用 `Eq.{1}` 点名形式 15 处；全课程 `.sokonanoda` 里的裸 `=` **只出现在 `--` 注释中**
（词法跳过注释 ⇒ 今天从未被验证过）。

### 10.4 内核 pp 文本的可解析性（§3.1）

```bash
# 全部 exit 0（decl.checked）：forall 多 binder / forall + Type 0 / 类型里带字面换行
def f : forall (a b : Prop), a -> b -> a := fun (a b : Prop) (ha : a) (hb : b) => ha
def f : forall (α : Type 0), α -> α := fun (α : Type 0) (x : α) => x
def f : forall (a : Prop),
a -> a := fun (a : Prop) (h : a) => h
```

### 10.5 `canonical_goal` 污染（§1.4）

```bash
# 同一份文件加/不加 namespace Foo，看 query state 的 step 0：
#   无 namespace → 'b ∧ a'
#   有 namespace → 'And b a'      ← 记法丢失
```

---

## 11. 对主线计划 `docs/design/course-lean-style.md` 的三条直接影响

| 该计划的条目 | 本调研的修正 |
|---|---|
| **§1.4**「goal 面板 / hover / 诊断里的类型文本由冻结内核的 pp 产出，显示点名形式……学习者写 `A ⊆ B`，面板回 `Set.subset α A B`」 | **部分不成立**：`A ⊆ B` 在 goal 面板里**已经能正确回显**（open 练习的 `goal`、per-tactic goal、binder 类型都走前端 `render_expr`，`crates/front/src/proof.rs:323-345`）。真正回 `Set.subset α A B` 的是**根状态 / 声明签名 / 表达式 hover / `#check`**（§1.3 的 L1–L8）。⇒ 记法对照页那句「这是对的，不是 bug」**要改口径**：不是"面板只能显示点名形式"，而是"**光标停在声明上时**显示点名形式" |
| **§3 L2.4**「`=` 的宇宙推断机制（spike SP1）」 | **先有一个更前置的阻塞项**：裸 `=` **今天在词法层就报错**（`crates/front/src/token.rs:296-308`，回归测试 `:759`），全课程只把它写在注释里。`infix:50 " = " => Eq` 无论写在哪个文件都会失败（§10.3b）。⇒ L2.4 应拆成 **L2.4a「`=` 词法/token」** + L2.4b「宇宙推断」；SP1 的 6 条探针在 L2.4a 之前写不出来 |
| **§10 N-3 / §7 SP2**「内核 pp 的 print-back：硬规则 1；§13.1 已论证；SP2 只做复核」 | §13.1 论证的是 **①改内核 / ②elab 保留源→核映射**；**显示边界重写（③）**是它没考虑的第三条路，**不违反硬规则 1**（§6.1），**判卷事件流可零改动**（§6.2、§7.2 测试 12）。SP2 的判定标准「显示通道可隔离、判卷通道零改动、往返有测试」**三条都满足** ⇒ 按 SP2 的口径应**单独立项（R4）**。若只想要 90% 的观感收益，§8 的 **T0-a（~25 行）** 是更低成本的替代 |

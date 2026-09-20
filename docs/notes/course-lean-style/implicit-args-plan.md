# 隐式实参（implicit arguments）可行性 + 分阶段实现计划

> 调研日期：2026-09-20。触发（用户原话）：「如果想支持 notation，感觉 '{x : Set}'
> 这种隐参数自动推导的机制不得不实现了。」
>
> 本文是**只读调研**产出：**不改任何现有文件**，结论全部带 `文件:行号` 证据，
> 「今天能不能用」一律以真二进制/真内核实测为准（`scripts/soko` 的 MCP `check`/`goals`
> 查询通道，判据永远是内核，不做文本比对）。
>
> 阅读顺序建议：§0 结论 → §1 今天缺什么（实测） → §6 三条路线对比 →
> §9 分阶段计划。§2–§5 是实现者要的证据底稿，§7 是 Lean 4 语义对齐档位，
> §8 是课程改写清单。

---

## 0. 结论（TL;DR）

1. **语法早就支持了**：`{α : Type}` 在 `def`/`theorem`/`axiom`/`example` 声明 binder、
   `inductive` 参数、`ctor` 字段、`fun`/`forall` binder 六处**全部能解析**，并且
   **风格一路保留到内核**（内核 pp 打回 `forall {α : Type 0}, α -> α`，实测 §1.4）。
   缺的不是语法，是**应用位（application site）的插入**。
2. **缺口的精确形状**不是「完全不能补参」，而是「**只按位置个数补前导参数、从不跳过隐式
   binder**」：
   - 记法路径 `a ∈ A`（`Set.mem (α : Type) (a : α) (A : Set α)`，3 层 / 2 操作数）
     **今天就能用**，补参靠裸变量匹配（`elab.rs:1465-1502`）；
   - 把同一签名写成 `{α : Type}` 后 `a ∈ A` **仍然能用**（实测 §1.5）——因为补参
     逻辑只看「层数差」，根本不看 `BinderKind`；
   - 但 `Set.mem a A`（点名、层数 == 实参个数）**今天被内核拒**（实测 §1.2），
     写 `{α}` 也一样拒（`F {α} (a : α)` + `n ⊛ n` ⇒ 内核裸错，实测 §1.6）；
   - 隐式 binder 在**中间**时记法直接报 `elab-notation-argument-unsolved`（实测 §1.7）。
3. **路线 C（只做「显式实参的类型 + 期望类型唯一确定被跳过的隐式参数」这一档）是推荐路线**：
   它是现有记法 hack 的**严格一般化**（把「按个数补前导」换成「按风格对齐、跳过隐式」），
   `unify_extract`（`elab.rs:1656-1700`）与 `notation_prefix_args` 已经是它的 80%；
   估 **生产代码 400–600 行 + 测试 400–600 行**，落在一个新模块
   `crates/front/src/compile/implicit.rs`（`elab.rs` 已 4184 行，不该再长）。
4. **路线 C 的关键安全性质**：对**签名里没有隐式 binder** 的应用是**逐字节 no-op**。
   今天课程库 74 个声明、prelude 全部显式（`prelude.rs:178-180` 明写「隐式实参不自动
   插入，所以签名显式给全参数」）⇒ 3451 处点名调用、776 处记法、36 目标 / 329 checked /
   99 open **全部不变**。这让 P1 可以**独立发布**，风险面只在新写的 `{...}` 签名上。
5. **路线 A（真元变量 + 合一）在架构上被内核堵死**：内核 `Expr` 只有
   `{StringLit, NatLit, Proj, Var, Sort, Const, App, Pi, Lambda, Let}`（`expr.rs:19-70`），
   `Value` 只有 `{Rigid, Unfold, Lam, Pi, Sort, NatLit, StrLit, Thunk}`（`value.rs:97-...`）
   ——**没有元变量/未知量/fvar 任何一种形式**；内核 `BinderStyle` 的文档注释直接说
   「These are only used by the pretty printer, and do not change the behavior of type
   checking」（`expr.rs:122-124`）。所以路线 A 只能做成**前端元变量 + 交给内核前 zonk**，
   估 **1200–2500 行**，且要重写每一个 App 的 elaborate。**今天不建议做**。
6. **路线 B（探针驱动）不是一条独立路线**，是路线 C + 内核验证尾巴：它能**验证**候选、
   不能**搜索**候选；而 `elab_expr` 执行时**根本没有内核环境**（`run_pass` 先走完
   walk、最后才 `builder.finish()` → `kernel_phase::finish_pass`，`check/mod.rs:615-638`），
   每次探针都只能走 `judge_*`＝**整前缀重编译**（`judge.rs:377-388`）。留作 P2 的可选
   安全网，不做主机制。
7. **必须先决定的两件事**（P1 之前）：
   - **N4.3 护城河要重新谈判**：`Set.mem a A` 从「必须被内核拒」变成「checked」是**语义
     变更**，被两条测试钉死（`tests.rs:6584-6601`、`cli/tests/notation.rs:162-177`），
     并在 `notation-subset.md:106-107`、`units/notation-cheatsheet.sokonanoda:45-50`
     教给学习者。P1 必须同轮改文档 + 测试 + 课程话术。
   - **`@f` 今天是个 no-op**：parser 吃掉 `@` 之后直接 `finish_const`（`parser.rs:2393-2406`），
     AST 里没有任何标记。插入落地后 `@Set.mem a A` 会**悄悄变得和 `Set.mem a A` 一样**，
     与 Lean 的「关闭隐式插入」相反。语料里 `@` 出现 **0 次**（`grep -rn "@" courses/`），
     所以 P1 顺手给它真语义最便宜。
8. **明确不做**（教学子集边界，§7.3 逐条给理由）：instance implicit `[...]`（typeclass）、
   strict implicit `⦃⦄`、`_` 洞（合成）、`?m` synthetic hole、`variable`/auto-bound、
   named arguments `(x := …)`、optional/auto 参数、`..` 省略号、高阶合一、
   依赖消去中的隐式推断。

---

## 1. 今天到底缺什么（9 条实测，全部可真机复现）

口径：`mcp__sokonanoda__check`（= 内核判卷的同一份真相）。每条给「源 → 结果」。

### 1.1 `Or.inl ha` 不行，`Or.inl A B ha` 行

```sokonanoda
axiom A : Prop
axiom B : Prop
axiom ha : A
theorem t1 : Or A B := Or.inl ha          -- ✗ kernel-rejected：期望 Sort(0)，实际是 A.[]
theorem t2 : Or A B := Or.inl A B ha      -- ✓ checked
```

证据：`Or.inl : (A B : Prop) -> A -> Or A B` 是 **prelude 的显式签名**
（`crates/front/src/compile/prelude.rs:208-210`，`PRELUDE_L1_SRC` 内），
`Expr::App` 分支对 fun/arg 都传 `None` 期望类型、直接 `mk_app`
（`crates/front/src/compile/elab.rs:1955-1961`），于是 `ha` 被喂给第一个 `Prop` 形参，
内核报「期望 `Sort(0)`，实际是 `A`」。

### 1.2 记法路径能补参，点名省参被内核拒（护城河）

```sokonanoda
def Set (α : Type) : Type := α -> Prop
namespace Set
def mem (α : Type) (a : α) (A : Set α) : Prop := A a
end Set
infix:50 " ∈ " => Set.mem
axiom MyNat : Type
axiom a : MyNat
axiom A : Set MyNat
axiom h : Set.mem MyNat a A

theorem t_notation    : a ∈ A := h            -- ✓ checked（记法补出 α := MyNat）
theorem t_full_named  : Set.mem MyNat a A := h -- ✓ checked
axiom   t_short_named : Set.mem a A            -- ✗ kernel-rejected：期望 Sort(1)，实际是 MyNat.[]
```

最后一条正是设计 N4.3 的「护城河」（`docs/design/notation-subset.md:106-107`），
被 `crates/front/src/compile/tests.rs:6584-6601` 与 `crates/cli/tests/notation.rs:162-177`
钉死。

### 1.3 **一个内核拒绝的声明会毒化后面所有记法**（本次调研新发现，实现时必须记账）

把上例的失败行放在前面，后面的 `theorem t_full_named : a ∈ A := h` 会**连带失败**：

```sokonanoda
theorem t_named_short : a ∈ A := Set.mem a A     -- ✗ kernel-rejected（如上）
theorem t_named_full  : a ∈ A := h               -- ✗ elab-notation-unknown-target
                                                 --   「读不到记法 ∈ 的目标 Set.mem 的类型：类型不匹配：期望 Sort(1)，实际是 MyNat」
```

根因：`judge_type_of` 把**整个文档前缀**重新 parse + 编译（`judge.rs:377-388`），
前缀里第一条错误就让它放弃；`elab_notation` 把它降级成
`elab-notation-unknown-target`（`elab.rs:1005-1031`），**归因到错误的行**。
这不是本计划要修的 bug，但**隐式实参落地会让「写错一处 → 后面记法全红」的窗口变小**
（省参写法不再失败），值得在 P1 的回归里加一条「一处失败不连坐」的断言。

### 1.4 隐式 binder 风格已经活着到达内核

```sokonanoda
def id2 {α : Type} (a : α) : α := a
```

`mcp__sokonanoda__goals` 给的内核渲染类型是：

```
forall {α : Type 0}, α -> α
```

即 `BinderStyle::Implicit` 在 Pi 里保住了，`pretty_printer.rs:477` 用 `{}` 打印、
`478` 用 `{{}}`、`479` 用 `[]`。**内核知道风格，只是不用它做任何事**
（`crates/kernel/src/expr.rs:122-124` 的文档注释：风格「只被 pp 使用，不改变类型检查行为」）。

### 1.5 记法补参**已经**能穿过隐式前导参数（与 `notation-audit.md` 的记载需要对齐）

```sokonanoda
namespace Set
def mem {α : Type} (a : α) (A : Set α) : Prop := A a
end Set
infix:50 " ∈ " => Set.mem
axiom h : Set.mem MyNat a A
theorem t : a ∈ A := h        -- ✓ checked
```

**能过**。原因：`notation_telescope` 只 `peel_pi` 取 `(名字, 域)`
（`elab.rs:1505-1514`），**完全不看 `BinderKind`**；`{α : Type}` 与 `(α : Type)`
在这里没有区别。⇒ 结论：**「记法补参」与「隐式实参」是两件正交的事**，
今天记法缺的不是隐式支持，而是「层数差」这一条对齐规则太窄。

> 与 `docs/notes/course-lean-style/notation-audit.md:484` 的差异：那一条记的是
> `def F {α : Type} (a : α) : Prop` + `1 ⊛ 1`（**层数 == 操作数个数**）失败，
> 本文 §1.6 复现了它。两处不矛盾：记法只在「层数 > 操作数」时才补。

### 1.6 层数 == 实参个数时，隐式前导参数被当成第一个实参吃掉

```sokonanoda
def F {α : Type} (a : α) : Prop := True
infix:50 " ⊛ " => F
axiom h : F MyNat n
theorem t : n ⊛ n := h        -- ✗ kernel-rejected：期望 Sort(1)，实际是 MyNat.[]
```

`layers.len() == operands.len()` ⇒ `max_missing == 0` ⇒ 不补
（`elab.rs:1476-1482`），`n` 落到 `α` 位上。

### 1.7 隐式 binder 在中间时记法报「补不出」

```sokonanoda
def H (p : Prop) {α : Type} (a : α) : Prop := p
infix:50 " ⊛ " => H
theorem t : P ⊛ n := h        -- ✗ elab-notation-argument-unsolved
```

「前导参数」这个前提本身失效（`elab.rs:1527-1529` 的注释与实现都把补参限制在
**前缀**）。这是路线 C 要修的正题。

### 1.8 tactic 路径**已经**在补前导类型参数（两条路径今天不对称）

```sokonanoda
theorem t_by_apply : Or A B := by apply Or.inl
  exact ha                    -- ✓ checked
```

`apply` 走 `by.rs` 的 `apply_tactic`：`judge_infer` 拿函数类型 → `peel_pi_delta`
剥望远镜 → `unify_spine` 位置合一 → 把「名字出现在 codomain 或 goal 里」的层判成
类型参数、其余判成子目标（`by.rs:432-546`）。所以**「隐式实参」在教学语言里
不是新概念，只是 term 路径没有**。`spine.rs` 的模块注释本来就要求两条路径共用
一份望远镜机械（`spine.rs:1-8`），P1 应当把 term 路径接到同一份上。

### 1.9 `@f` 是 no-op（潜在语义陷阱）

```sokonanoda
theorem t : Or A B := @Or.inl ha   -- ✗ kernel-rejected，与不加 @ 完全一样
```

parser 在 `parse_atom` 里吃掉 `TokenKind::At` 后直接 `finish_const(name, span)`
（`parser.rs:2388-2406`），AST 里没有任何「显式应用」标记。`grep -rn "@" courses/`
= **0 处**，所以给 `@` 补真语义没有兼容包袱。

---

## 2. Q1 —— parser/AST：`{α : Type}` 在哪些位置语法上已支持

### 2.1 语法面（全部已支持）

| 位置 | 入口 | 行号 |
|---|---|---|
| `fun`/`forall` binder | `parse_binder` 的 `TokenKind::LBrace` 分支 | `parser.rs:2840-2853` |
| 多名字组 `{a b : T}` | `parse_binder_group`（`LParen`/`LBrace` 判风格） | `parser.rs:1950-1980` |
| `def`/`theorem`/`example` 声明 binder | `parse_decl_binders` → `wrap_decl_binders` | `parser.rs:104-130`、`1112`/`1133`/`1158` |
| `axiom` 声明 binder | `wrap_type_binders`（与上共用折叠，G-13） | `parser.rs:122-130`、`1477` |
| `inductive` 参数 / `ctor` 字段 | `parse_inductive_binders`（显式吃 `LBrace`） | `parser.rs:1601-1619`、`1621-1635` |
| `∀ x ∈ s, p` / `∃ x ∈ s, p` binder | `parse_binder_prefix` → `push_binders` | `parser.rs:2687-2727`、`2791-2808` |

AST 侧只有一个二元枚举：

```rust
// crates/front/src/ast.rs:393-406
pub enum BinderKind { Explicit, Implicit }
pub struct Binder { pub name: String, pub ty: Option<Box<Expr>>,
                    pub style: BinderKind, pub span: Span }
```

`BinderKind` 与宇宙参数组 `{u}` 的消歧规则：**名字列表后接 `}` ⇒ 宇宙组；
接 `:` ⇒ binder 组**（`decl-binders.md:41-44`，测试
`decl_binders_disambiguate_universe_params_from_implicit_binders`）。

### 2.2 `BinderKind::Implicit` 在哪里被读、被怎么用

**只有 6 处，全是「风格搬运」，没有一处参与插入决策**：

| 位置 | 用途 |
|---|---|
| `elab.rs:969-974` `kernel_binder_style` | `Explicit → BinderStyle::Default`、`Implicit → BinderStyle::Implicit` |
| `elab.rs:1758` | `peel_expected_src` 把期望类型的风格回读（给无注解 lambda binder） |
| `elab.rs:1985` | lambda binder 落 `mk_lambda` 时带风格 |
| `elab.rs:2067` | forall binder 落 `mk_pi` 时带风格 |
| `judge.rs:567` | 合成判定文本时 `{…}`/`(…)` 渲染 |
| `proof.rs:557`、`suggest.rs:297` | 骨架/hover 文本渲染 |

内核侧风格有三个值（`Default`/`Implicit`/`StrictImplicit`/`InstanceImplicit`，
`expr.rs:126-134`），**教学语言的 parser 只产出前两个**：`⦃⦄` 与 `[...]` 今天
既不可解析、也无 `BinderKind` 变体。

### 2.3 「今天能不能用」

- 写 `{α : Type}` 的**签名**：✅ 能用，且风格活着进内核（§1.4）。
- 在签名里**依赖**隐式风格做省略：❌ 不行（§1.1/§1.2/§1.6）。
- `@f`：⚠️ 能解析但**被静默丢弃**（§1.9）。

---

## 3. Q2 —— elaborator 现状：`elab_app` 不存在，App 是裸 `mk_app`

### 3.1 函数应用的 elaborate（全部代码）

```rust
// crates/front/src/compile/elab.rs:1955-1961
Expr::App { fun, arg, span } => {
    let fun = elab_expr(builder, fun, scope, univ, known, hovers, None, None, ctx)?;
    let arg = elab_expr(builder, arg, scope, univ, known, hovers, None, None, ctx)?;
    let out = builder.mk_app(fun, arg);
    record_hover(hovers, scope, *span, out, None);
    Ok(out)
}
```

**关键事实**：

1. 没有独立的 `elab_app`；App 就在 `elab_expr` 的 match 臂里，**从左到右**逐个
   `mk_app`，不做 spine 展平（展平工具在 `spine.rs:136-145` `spine_of`，但 term
   elaborate 不用它）。
2. **期望类型在这里被丢掉**：fun 与 arg 都传 `None, None`，即使上层
   `build_def`/`build_theorem` 已经把声明类型作为 `expected` 传进来了
   （`elab.rs:780-793`、`820-833`、`857-870`）。
3. **没有任何元变量/合一/延迟约束**：`elab.rs` 全文没有 `MetaVar`/`meta`/`unify`
   的定义（`grep -n "MetaVar\|meta\|unify" elab.rs` 只命中 `unify_extract` 这一个
   记法专用助手）。
4. **没有任何类型检查**：elaborate 只负责把源 AST 搬成内核项；「类型对不对」全部
   交给最后的内核（`check/kernel_phase.rs:214` `try_check_declar`）。这也是本语言
   「判定永远走 kernel」的架构结果（`REQUIREMENTS.md` §2 第 4 条）。

### 3.2 `ElabScope` / `ElabCtx` / `KnownTable` 各有什么

```rust
// crates/front/src/compile/elab.rs:277-286
pub(crate) struct ElabScope<'a> {
    names: Vec<String>,
    tys: Vec<ExprPtr<'a>>,        // 内核类型（infer_under_binders 可直接吃）
    src_tys: Vec<Option<Expr>>,   // 写出来的源类型（judge_binders 用）
    spans: Vec<Span>,
}
```

- `ElabScope::judge_binders()`（`elab.rs:315-327`）把「有名字且有书写类型」的 binder
  变成 `GoalBinderSpec`，供 `judge_infer` 合成 `#check fun … => …`；
  `judge_binders_for`（`:338-369`）只保留表达式真正依赖的 binder——因为
  `render_expr` 的 round-trip 对函数类型 binder 会错位（`:328-337` 的实测注释）。
- `ElabCtx`（`elab.rs:263-276`）只有 `prefix_src` / `options` / `inductives` / `ns`。
  **没有内核环境**——这是本计划最重要的架构约束（见 §5.1）。
- `KnownTable = HashMap<String, KnownName>`，而
  `KnownName::Decl { universes: Vec<String> }`（`elab.rs:42-58`）**只存宇宙参数个数，
  不存类型**。所以前端今天**没有**任何「本地常量签名表」；要签名只能问内核。

### 3.3 `judge_infer` 在 elaborate 里被怎么用（现有的「问内核」通道）

`elab_expr` 里 8 处调用 `judge_infer`/`judge_type_of`，都是「算不出来就问内核」：
`let` 无注解（`:2229`）、`match` 的 motive/参数（`:2886`、`:3331`、`:3373-3375`）、
记法签名与操作数类型（`:1021`、`:1197`、`:1252`、`:1335`、`:1644`）、
宇宙层推断（`:3808`）。

### 3.4 「今天能不能用」

- 应用位插入：❌ 完全没有。
- 期望类型传播到 App：❌ 有参数但被丢。
- 元变量/合一：❌ 不存在（这是 §6 三条路线的分水岭）。

---

## 4. Q3 —— 记法路径的「补前导类型参数」是怎么实现的

### 4.1 机制（四段）

1. **读目标签名**：`judge_type_of(prefix_src, options, "Set.mem")` → 内核 pp 文本
   `forall (α : Type 0), α -> Set α -> Prop`（`elab.rs:1017-1031`）。
   用 `judge_type_of` 而**不是** `judge_infer`，因为后者要合成 lambda 再逐层剥 binder，
   多 binder 折叠会让 `Set.image` 这类函数目标错位（`:1012-1016` 实测注释）。
2. **剥望远镜**：`notation_telescope`（`elab.rs:1505-1514`）用 `spine::peel_pi`
   把 `forall`/`->` 都算成层，得 `layers: Vec<(名字, 域)>` + `result`。
   **不看风格**（这是 §1.5 能过的原因）。
3. **定补几位**：`max_missing = layers.len() - operands.len()`，从大到小试，
   取第一个能完整解出的（`elab.rs:1476-1502`）。
4. **逐位求解**（`solve_prefix_args`，`elab.rs:1518-1567`）：
   - ① **由操作数解**：找第一个 `j > i` 且域里提到参数名 `n` 的层，用
     `infer_type_text`（= `judge_infer`，`:1642-1645`）拿那个操作数的类型文本，
     再 `unify_extract(层的域, 操作数类型, n)`；
   - ② **由期望类型解**：把已解出的参数代进剩余望远镜
     （`substitute_prefix_params`，`:1610-1638`），与 `expected_src` 做
     `unify_extract`。
5. **发射内核项**：`mk_const` + 逐个 `mk_app`（`elab.rs:1057-1078`），**不回读源码**
   （as-built 教训见 `notation-subset.md:280-283`）。

`unify_extract`（`elab.rs:1656-1700`）是唯一的「合一」：**模板是裸变量 ⇒ 取实际；
同头同实参个数 ⇒ 逐位找裸变量；`->`/`forall` 也算二元头**。文档明说
「不做一般合一、不引入元变量」。

### 4.2 还有两处「补参」的兄弟

- `notation_operand_expected`（`elab.rs:1573-1605`）：把已解出的前导参数代进望远镜，
  给出**每个操作数的期望类型**（`A ⊆ ∅` 里的 `∅` 靠它拿到 `Set α`）。
  `notation-subset.md:274-279` 明说「这条是『期望类型传播』在记法里的具体形态，
  **将来做一般隐式实参时应能整段替换掉**」。
- `set_literal_prefix_args`（`elab.rs:1184-1205`）：集合字面量 `{∅}` 的期望类型回退解。

### 4.3 隐式实参落地后这一段能不能删/退化？

**能退化，但不能直接删**：

- 记法补参的**语义必须逐字节保持**（N7 教学契约：点名与记法两种写法判卷一致，
  `notation-subset.md:141-147`；`cli/tests/notation.rs:108-135` 钉五元组相等）。
- 正确做法：把 `elab_notation` 的「补前导参数」换成**调用同一份新机械**
  （把目标常量 + 操作数交给 route C 的应用构造器），删掉
  `solve_prefix_args`/`notation_prefix_args`/`notation_operand_expected`/
  `set_literal_prefix_args` 四段中的重复逻辑；`fallback_prefix_args` 这个参数
  （`:1032-1049`）随之消失。
- 风险点：记法路径的**操作数先 elaborate、类型参数后补**顺序被
  `cli/tests/protocol.rs::notation_diagnostics_stage_as_parse_and_elab` 钉死
  （`notation-subset.md:114-119`），退化时必须保持诊断码与 stage 不变。

---

## 5. Q4/Q5 —— 可用的推理基础设施与内核公开面

### 5.1 最重要的架构事实：`elab_expr` 执行时**没有内核环境**

`run_pass`（`check/mod.rs:485-639`）的顺序是：

```
EnvBuilder 累积 → walk.run(...)（= 逐命令 elaborate，elab_expr 在这里跑）
                → kernel_phase::finish_pass(Walked { builder, … })   // :624
```

`builder` 直到 `finish_pass` 才 `finish()` 成 `ExportFile`。所以 elaborate 期能拿到的
**只有 `prefix_src: &str` + `options`**——这正是 `judge_*` 存在的理由，也是它贵的理由。

### 5.2 前端能拿到的「类型推断」手段与代价

| 手段 | 入口 | 能力 | 代价 |
|---|---|---|---|
| `judge_type_of` | `judge.rs:358-370` | 一个**常量/项**的类型文本（`#check`） | **整前缀重编译**（`:377-388`）；结果进 `type_cache`（128 条 FIFO，`:117-146`） |
| `judge_infer` | `judge.rs:422-452` | `term` 在 `binders` 语境下的类型文本（合成 `#check fun … => term`，再剥 n 层） | 同上；进 `judge_cache`（`:81`、`:148-157`）。**要求每个 binder 都有书写类型**（`:464-472`） |
| `judge_terms` / `judge_hole_fill` | `judge.rs:179-186`、`611-622` | 「这个项能不能填进那个洞」的 kernel 终审 | 同上 |
| `goals.rs` 探针（B′） | `goals.rs:369-427`、`433-465` | 子洞期望类型，**纯 AST 走查 + 模板替换** | 零内核调用；但无 def_eq、无洞穿透（`spine-meta-a.md:20-34`） |
| `spine::unify_spine` | `spine.rs:215-238` | 位置 spine 合一（同头 + 实参个数相等）→ σ | 纯 AST；**刻意不做高阶匹配**（`:210-214`） |
| `spine::peel_pi_delta` + `unfold_head_once` | `spine.rs:67-106` | 源级**一层 delta 展开**（看穿 `A ⊆ B`/`¬ A`/`Iff`） | 纯 AST；`DefTable` 由 walk 累积（`walk.rs:350-357`） |
| `infer_type_text` | `elab.rs:1642-1645` | 记法路径现用的「操作数类型」= `judge_infer` + `parse_expr_text` | 一次 judge + 一次 parse |

**缓存口径（成本的关键）**：两张缓存都是 `HashMap<u64, _>` + FIFO 128 条
（`judge.rs:81-97`），key = `hash(extra_prefix, prefix_src, options, binders, term)`
（`:99-106`、`:439-445`）。⇒ **同一文档状态下重复问同一个问题才命中**；
文档任何更早的编辑都会失效（保守但正确）。**「一次 judge = 一次整前缀重编译」**
是路线 B 的致命成本，也是路线 C 尽量不调 judge 的理由。

**经验量级**（本机实测，debug 构建）：
`target/debug/sokonanoda grade courses/set-theory/units/unit02-subsets-empty.sokonanoda --json`
= **0.93s**（含 import 闭包 + 该单元全部记法展开的 judge 调用）；
课程门禁 36 目标 ≈ **12.2s**（`courses/set-theory/README.md:126` 与
`docs/design/course-lean-style.md:63` 记录）。
perf 台账里的 `judge_prefix_with_imports` 是**整个项目编译**的用例（10 条 match /
2 模块，`crates/front/tests/perf_project.rs:320-363`），**不是**单次 judge 成本——
引用时别搞错。

### 5.3 内核能提供什么（不许改内核，只列**公开** API）

| API | 位置 | 说明 |
|---|---|---|
| `ExportFile::check_declar` / `check_declar_at` | `tc.rs:75-92` | 声明终审；拒绝 = **panic** |
| `ExportFile::try_check_declar` / `try_check_declar_at` | `util.rs:662-687` | 同上但 `catch_unwind` 成 `Result<(), CheckError>`；**唯一不 panic 的终审入口** |
| `ExportFile::with_tc(limit, f)` | `util.rs:703-711` | 起一个 `TypeChecker` 会话（自带 arena/cache） |
| `ExportFile::with_pp` | `util.rs:713-717` | 纯 pp 会话 |
| `TypeChecker::assert_def_eq(u, v)` | `tc.rs:274-280` | **defeq 判定**，不等即 panic ⇒ 必须包 `quiet_catch`（`check/mod.rs:713-726`） |
| `TypeChecker::is_proposition` / `is_proof` | `tc.rs:289-302` | sort 判定 |
| `TypeChecker::with_pp` / `with_pp_scoped` | `tc.rs:307-323` | 内核 pp（`seed_binder_names` 让松散变量还原真名） |
| `TypeChecker::infer_closed_type` | `quote.rs:10-18` | **闭项**的 whnf 类型（`#check` 的内核原语） |
| `TypeChecker::reduce_closed` | `quote.rs:20-26` | **闭项**完整归约（`#reduce` 的内核原语） |
| `TypeChecker::infer_under_binders(binder_tys, e)` | `quote.rs:36-55` | **开项**在 binder 语境下的类型（hover 已用：`check/mod.rs:790-796`） |
| `EnvBuilder::{mk_const, mk_app, mk_pi, mk_lambda, …}` | `builder.rs:141-232` | 纯 arena 构造，**不做任何推断** |

**没有的东西**（这是路线选择的硬约束）：

- ❌ 没有**合一器**（unifier）：`conv.rs::def_eq_core`/`def_eq_at` 是
  `pub(crate)`（`conv.rs:25`、`:52`），外部只能通过会 panic 的 `assert_def_eq` 问
  「这两个 defeq 吗」，**不能问「这个元变量该取什么」**。
- ❌ 没有**元变量**：`Expr`（`expr.rs:19-70`）= `{StringLit, NatLit, Proj, Var, Sort,
  Const, App, Pi, Lambda, Let}`；`Value`（`value.rs:97-…`）= `{Rigid, Unfold, Lam, Pi,
  Sort, NatLit, StrLit, Thunk}`。没有 `MVar`/`FVar`/`Synthetic`/`Unknown`。
- ❌ 没有公开的 `infer_value`/`eval`/`whnf_head`/`deep_reduce`（全 `pub(crate)`，
  `infer.rs:79`、`eval.rs:445`/`:928`/`:963`）。前端要 whnf 只能**源级 delta**
  （`spine.rs:83-106`）或 `reduce_closed`（闭项）。
- ❌ 风格不参与类型检查（`expr.rs:122-124`）。

**冻结纪律的准确口径**（`docs/architecture.md:467-489`）：内核「**只加不改语义**」，
**冷路径的新增公开 API 有先例**——`quote.rs` 的 `infer_closed_type`/`reduce_closed`
就是为 `#check`/`#reduce` 新增的（`architecture.md:476`），`tc.rs` 的 `with_pp`、
`util.rs` 的 `try_check_declar` 同理（`:474`、`:478`）。所以「加一个冷路径推断原语」
**不是绝对禁区**，但要按 `architecture.md` §6 记账 + 三层回归 + 明说是显示/探针用途。
本计划**默认不需要**新增内核 API；若 P2 发现需要，单独立项、单独评审。

---

## 6. Q6 —— 三条实现路线（重点）

先给一张对齐表，再逐条展开。

### 6.0 三条路线对比

| 维度 | **A：真元变量 + 合一** | **B：探针驱动** | **C：风格对齐 + 唯一确定（推荐）** |
|---|---|---|---|
| 机制 | 前端 `MetaCtx` + 一阶合一 + 延迟约束 + zonk | 枚举候选实例化，交内核 `try_check_declar_at` 验证 | 显式实参按**风格**对齐到显式层；被跳过的隐式层由「显式实参类型 + 期望类型」的**同时头部匹配**解出 |
| 需要的新结构 | `MetaVar/MetaCtx/occurs/zonk/constraint queue` | 候选枚举器 + 合成声明 + 探针缓存 | `DeclSig` 源级望远镜表（带风格）+ 一次同时匹配 |
| 估行数（生产/测试） | **1200–2500 / 1500+** | 800–1500 / 800+ | **400–600 / 400–600** |
| 每次应用的内核代价 | 0（zonk 后才碰内核），但 defeq 约束要靠 `judge_*` 或 `assert_def_eq` | **O(候选数) 次内核检查**；在 elaborate 期只能走 `judge_*`＝整前缀重编译（§5.1） | 0（除既有的记法签名查询）；解不出就报教学错误 |
| 课程覆盖（§8 的 3451 处 + prelude） | 100%+ | ≈ C（能验证不能搜索） | **≈100%**：`Set.mem a A`、`Or.inl hx`、`Set.ext h`、`∅`、`f '' A`、`{a}` 全覆盖 |
| 覆盖不到 | — | 候选不可枚举时（高阶、依赖消去） | 需要「一般合一才能定」的：`Eq` 宇宙多态目标（`docs/notes/course-lean-style/notation-audit.md:497-501` 记的内核断言）、`?m` 只能由后续约束定的情形 |
| 对既有程序的影响 | **重写每个 App** ⇒ 全量回归风险 | 中（多一层验证，失败回退） | **对无隐式签名的应用逐字节 no-op** ⇒ 风险集中在新增能力 |
| 改哪些文件 | `elab.rs` 大改 + 新 `meta.rs` + `error.rs` + 全测试 | `elab.rs` + 新 `probe.rs` + `kernel_phase.rs` | 新 `implicit.rs` + `elab.rs`（App 臂 ~15 行）+ `walk.rs`/`goals.rs`（签名表）+ `error.rs` |
| 违反内核冻结？ | 否（全前端）——**但架构上被内核没有元变量逼成「zonk 前必须全解」** | 否 | 否 |
| 教学叙事 | 「和 Lean 一样」 | 「试出来再验」 | 「Lean 的子集：隐式参数必须由**后面的显式实参或期望类型唯一确定**」 |
| 判定 | **不做**（收益/风险比最差） | 留作 P2 可选安全网 | **做** |

### 6.1 路线 A —— 真元变量 + 合一（Lean 式）

**要什么**：`MetaVarId`、`MetaCtx { assignment, kind }`、`occurs_check`、
`Expr` 级代入（复用 `spine::substitute`，`spine.rs:257-387`）、延迟约束队列
（`(meta, expected_src, span)`）、zonk（发射内核项前把 meta 全代掉）、
以及一个「解不出」的新诊断。

**为什么它在架构上比 Lean 弱一档**：Lean 的 `?m` 是**内核项的一部分**，可以在
「还不知道」的状态下继续 elaborate、最后统一求解；本语言内核**没有元变量形式**
（§5.3），所以 meta 只能活在前端 AST 里，**任何交给内核的项都必须已 zonk**。
后果：① 约束必须在 `add_declar` 之前全部解决，做不到「先放着」；
② 「两个类型是不是 defeq」只能靠 `judge_*`（贵）或 `assert_def_eq`（panic 包装、
且只在 kernel phase 有 env）；③ 前端必须自己实现合一，而它面对的是**源级 AST**，
不是内核项——`Set.subset α A B` 与 `∀ x, A x → B x` 在源级不同形（要 delta 才能对齐）。

**估行数与风险**：1200–2500 行；`elab.rs` 已 4184 行、`compile/tests.rs` 7926 行，
「每个 App 都换路径」意味着既有诊断文案/stage/事件序列全部要重新证明不变
（本仓的纪律是**逐字不变**，见 `notation-subset.md:114-119` 的先例）。
**结论：不做**。触发条件（若将来要做）：出现一个课程必需、而路线 C 明确覆盖不到的
写法，且它不能靠改签名规避。

### 6.2 路线 B —— 探针驱动

**想法**：不做合一，直接**试**：对每个被跳过的隐式位枚举候选（显式实参的类型头、
期望类型的对应位、作用域里类型匹配的变量），拼出完整应用，合成
`def _soko_probe : <期望类型> := <拼好的项>`，用
`try_check_declar_at(&probe, EnvLimit::ByIndex(env_before))`（`util.rs:669-687`，
先例：`redundant-sorry` 探针 `kernel_phase.rs:155`）判「能不能过」。

**为什么它不能当主机制**：

1. **elaborate 期没有 env**（§5.1）⇒ 探针只能走 `judge_*`，每次 = **整前缀重编译**；
   一个应用 k 个隐式位、每位 c 个候选 ⇒ `O(k·c)` 次整前缀编译。课程 3451 处调用，
   不可接受。
2. **它是验证不是搜索**：候选从哪来？如果候选来源就是「显式实参的类型 + 期望类型」，
   那它已经等于路线 C（C 直接解出唯一候选，B 只是把 C 的答案再验一遍）。
3. **内核拒绝的粒度太粗**：探针失败只告诉你「这个实例化不行」，不告诉你哪一位错；
   教学诊断会退化。

**它值得保留的部分**：P2 起可作为**发射前的可选终审**（「我解出的实例化让整条声明
过内核」）——但注意这与「最后内核本来就会判」重复，收益主要是**更早、更准的归因**。
**结论：不做主机制**。

### 6.3 路线 C —— 「隐式参数由后续显式实参 / 期望类型唯一确定」（推荐）

**一句话**：把现有记法的「按个数补前导参数」换成「**按风格对齐、跳过隐式层**」，
并给被跳过的层一个**同时头部匹配**求解器（模板变量 → 实际子项，带 occurs check）。

**语义规则（教学子集，逐条可测）**：

1. **对齐**：把应用展平成 spine `f a₁ … aₙ`。取 `f` 的望远镜
   `[(n₁,s₁,T₁) … (n_k,s_k,T_k)] → R`。显式实参**按序**对齐到**显式层**；
   隐式层被**跳过**（不消耗实参）。
2. **求解**：对每个被跳过的隐式层 `i`，收集**约束**：
   - 该层之后每一层 `j` 的**域**（以及结果 `R`）里出现 `nᵢ` 的位置 ⇒ 用那一层
     对应的**实际实参的类型**（或结果对**期望类型**）做**同时头部匹配**；
   - 匹配规则 = `unify_extract` 的一般化：模板里出现的**自由变量**（望远镜 binder 名）
     映射到实际的对应子项；**必须 occurs check**（禁止 `α := F α`）；
   - 所有约束**合并**成一个 σ（同一变量两处约束必须给出**同一个**解，否则不猜、报错）。
3. **发射**：`mk_const` + 按**原望远镜顺序**逐个 `mk_app`（隐式位放解出来的项）。
4. **失败**：解不出 ⇒ 新诊断码 `elab-implicit-argument-unsolved`，hint 教「把参数
   写全（点名形式）或补类型标注」——**绝不猜**（沿用记法路径「解不出就报，不猜」的
   既有纪律，`notation-subset.md:426`）。
5. **`@f`**：关闭规则 1 的跳过（所有层都消耗实参）——与 Lean 同义。
6. **不变量（安全性的来源）**：`f` 的望远镜里**没有隐式层** ⇒ 规则 1 退化成「逐个
   消耗」，与今天的 `mk_app` 链**逐字节相同**。

**覆盖（用课程真实例子）**：

| 课程写法 | 今天 | 路线 C |
|---|---|---|
| `Set.mem a A`（`lib/Set.sokonanoda:55`） | ✗ 内核拒（§1.2） | ✓ α ← `typeof(a)` |
| `Or.inl hx`（`prelude.rs:208-210`） | ✗（§1.1） | ✓ A ← `typeof(hx)`，B ← 期望类型 `Or A B` |
| `Set.ext h`（`lib/Set.sokonanoda:79-80`） | ✗ | ✓ `A B α` ← 期望 `A₀ = B₀` 的同时匹配 |
| `∅`（`Set.empty`，零操作数） | ✓ 仅记法 | ✓ 同一条（期望类型路） |
| `f '' A`（`Set.image`，两个前导参数） | ✓ 仅记法 | ✓ 操作数类型 + 期望类型 |
| `{a}` / `{a,b}`（集合字面量） | ✓ 仅记法 | ✓ 同一份机械 |
| `Set.subset α A C` 里省 `α` | ✗ | ✓ |
| `Or.inl (And.left hx)`（嵌套，`unit08-solution:137`） | ✗ | ✓ 参数类型由内核给出 |

**改哪些文件**：

| 文件 | 改动 | 估行 |
|---|---|---|
| `crates/front/src/compile/implicit.rs`（**新建**） | `DeclSig` 表 + 对齐 + 同时匹配 + occurs + 发射 + 诊断 | 250–400 |
| `crates/front/src/compile/elab.rs:1955-1961` | App 臂改走 spine 路径；把 `expected`/`expected_src` 传下去 | 15–40 |
| `crates/front/src/compile/check/walk.rs`（`Walk`/`CmdCtx`） | 建/穿 `DeclSig` 表（签名来自命令表，零内核调用） | 60–120 |
| `crates/front/src/compile/goals.rs`（`FuncTemplate`/`CtorTemplate`） | 加 `binder_styles: Vec<BinderKind>`（现在被丢掉：`goals.rs:24-48`、`:206-278`） | 30–60 |
| `crates/front/src/compile/prelude.rs` | 把 prelude 签名登记进同一张表（prelude 是受信任安装的 AST，已有源文本） | 40–80 |
| `crates/front/src/compile/error.rs` | 新码 `ElabImplicitArgumentUnsolved` + hint | 10–20 |
| `crates/front/src/parser.rs:2393-2406` + `ast.rs` | `@` 的真语义（AST 标记 + 各 `match` 补一处） | 40–80 |
| 测试（`compile/tests.rs`、`crates/cli/tests/`） | 三层回归 + 护城河重谈 | 400–600 |

**风险**：

1. **护城河重谈**（高，但是**有意为之**）：`Set.mem a A` 从拒到过。必须同轮改
   `tests.rs:6584-6601`、`cli/tests/notation.rs:162-177`、`notation-subset.md:106-107`、
   `course-lean-style.md:285/660`、`units/notation-cheatsheet.sokonanoda:45-50`
   （学习者话术）、`docs/TESTING.md:43`（守护行）。
2. **签名表来源**（中）：`KnownTable` 不存类型（§3.2）。两条路：(a) 复用
   `GoalTemplates`（项目模式下它已经是「拓扑序在前的单元 + 自己」的合成命令表，
   `check/mod.rs:545-561`，**零内核调用**）；(b) 新增 `DeclSig` 表。
   推荐 (a) 的形制 + 补风格字段，避免第三张表。
3. **记法路径语义必须逐字不变**（中）：两种写法判卷一致是 N7 契约
   （`cli/tests/notation.rs:108-135`）。退化时保持诊断码/stage/事件序列。
4. **`render_expr` round-trip**（中，已知坑）：`judge_infer` 的答案要 `parse_expr_text`
   回读，而 `render_expr` 不给 forall 加括号、也不带记法表
   （`elab.rs:328-337`、`course-lean-style.md:187`）。新机械尽量用**内核项**
   （`ElabScope::tys` 直接喂 `infer_under_binders`）而不是文本，可绕开这条。
5. **宇宙多态目标**（中，既有硬边界）：`elab_notation` 给常量的是**空宇宙层切片**
   （`elab.rs:1057-1059`），目标是 `{u}` 定义时内核断言 `left: 1 / right: 0`
   （`docs/notes/course-lean-style/notation-audit.md:497-501`）。隐式实参落地**不会自动修它**，`Eq`/`Eq.refl`
   这类 `{u}` 常量要另做宇宙推断（`course-lean-style.md:279` L2.4b）。
6. **一处失败连坐**（低-中）：§1.3 的毒化效应在省参写法变合法后会减轻，但没消失。

**违反内核冻结？** ❌ 不违反。全部改动在 `crates/front`；内核只被**调用**
（`judge_*` 走既有通道；若改用 `infer_under_binders` 也是既有公开 API）。
`git diff --stat -- crates/kernel/` 必须为空——这是每轮的验收项。

### 6.4 三条路线与课程场景的覆盖矩阵

| 场景 | A | B | C |
|---|---|---|---|
| `Set.mem a A`（α ← 实参类型） | ✓ | ✓ | ✓ |
| `Or.inl hx`（B ← 期望类型） | ✓ | ✓ | ✓ |
| `Set.ext h`（A B α ← 期望类型同时匹配） | ✓ | ✓ | ✓ |
| `∅` 零操作数（← 期望类型） | ✓ | ✓ | ✓ |
| `f '' A`（两个前导参数） | ✓ | ✓ | ✓ |
| `Eq.{u}` 宇宙多态目标 | ✓ | ✓ | ✗（既有边界，另立项） |
| 依赖消去里的隐式（`cases` 分支 motive） | ✓ | ✗ | ✗（不做） |
| 高阶合一（`?f x = …` 定 `?f`） | ✓ | ✗ | ✗（不做） |
| 每次应用的内核代价 | 0–多次 defeq | O(k·c) 次前缀重编译 | 0 |

---

## 7. Q7 —— Lean 4 语义取证与对齐档位

### 7.1 逐字取证（来源：Lean Language Reference, Ch. 13 Terms）

`{α : Type}` 普通隐式（verbatim）：

> Ordinary **implicit** parameters are function parameters that Lean should determine
> values for via unification. In other words, each call site should have exactly one
> potential argument value that would cause the function call as a whole to be
> well-typed. The Lean elaborator attempts to find values for all implicit arguments
> at each occurrence of a function. Ordinary implicit parameters are written in curly
> braces (`{` and `}`).

`⦃α : Type⦄` strict implicit（verbatim）：

> **Strict implicit** parameters are identical to ordinary implicit parameters, except
> Lean will only attempt to find argument values when subsequent explicit arguments are
> provided at a call site. Strict implicit parameters are written in double curly braces
> (`⦃` and `⦄`, or `{{` and `}}`).

`[α : Type]` instance implicit（verbatim）：

> Arguments for **instance implicit** parameters are found via type class synthesis.
> Instance implicit parameters are written in square brackets (`[` and `]`). Unlike the
> other kinds of implicit parameter, instance implicit parameters that are written
> without a `:` specify the parameter's type rather than providing a name.

**应用位算法（verbatim，这是本计划的语义基准）**：

> Each parameter expected by the function has a name. Recurring over the function's
> argument types, arguments are selected from the sequence of arguments as follows:
> * If the parameter's name matches the name provided for a named argument, then that
>   argument is selected.
> * **If the parameter is implicit, a fresh metavariable is created with the parameter's
>   type and selected.**
> * If the parameter is instance implicit, a fresh instance metavariable is created with
>   the parameter's type and inserted. Instance metavariables are scheduled for later
>   synthesis.
> * **If the parameter is a strict implicit parameter and there are any named or
>   positional arguments that have not yet been selected, a fresh metavariable is created
>   with the parameter's type and selected.**
> * If the parameter is explicit, then the next positional argument is selected and
>   elaborated. …

**求解时机（verbatim）**：

> Finally, instance synthesis is invoked and as many metavariables as possible are solved:
> 1. A type is inferred for the entire function application. This may cause some
>    metavariables to be solved due to unification that occurs during type inference.
> 2. The instance metavariables are synthesized. …
> 3. **If there is an expected type, it is unified with the inferred type; however,
>    errors resulting from this unification are discarded.** If the expected and inferred
>    types can be equal, unification can solve leftover implicit argument metavariables.
>    If they can't be equal, an error is not thrown because a surrounding elaborator may
>    be able to insert coercions or monad lifts.

**风格不影响内核（verbatim，与本仓内核注释互相印证）**：

> Lean's core language does not distinguish between implicit, instance, and explicit
> parameters: the various kinds of function and function type are definitionally equal.
> The differences can be observed only during elaboration.

**签名位 vs 应用位（verbatim）**：

> If the expected type of a function includes implicit parameters, but its binders do
> not, then the resulting function may end up with more parameters than the binders
> indicated in the code. This is because the implicit parameters are added automatically.

**解不出时的报错形态（verbatim 例子）**：

> ```
> don't know how to synthesize implicit argument `α`
>   @g ?m.3
> context:
> ⊢ Type
> ```

来源：
- <https://lean-lang.org/doc/reference/latest/Terms/Function-Application/>
- <https://raw.githubusercontent.com/leanprover/reference-manual/main/Manual/Terms.lean>（§ Function Types / Functions / Function Application）

### 7.2 其余特性的对齐档位

| Lean 特性 | 语义要点 | 本计划 |
|---|---|---|
| `{x : T}` 普通隐式 | 每次出现都尝试求解；解不出报错 | ✅ **做**（路线 C） |
| `⦃x : T⦄` strict implicit | 只有「后面还有未选中的实参」时才尝试求解（即 `f` 单独出现不强制解 α） | ❌ **不做**（教学子集里 `f` 单独出现就该报错，比 strict 更简单也更安全） |
| `[x : T]` instance implicit | typeclass 合成 | ❌ **不做**（本语言没有 class/instance 机制） |
| `_` 洞 | 元变量 + 合成；解不出报 `don't know how to synthesize placeholder` | ❌ **不做**（§13.3 的老边界；`Hole` 今天只允许当 `sorry` 的载体，`elab.rs:1950-1954`） |
| `?m` synthetic hole | 永不被合一解掉 | ❌ 不做 |
| `@f` | 关闭隐式插入 | ✅ **做**（便宜、且不做会**静默改变含义**，§1.9） |
| `variable` / auto-bound | 自由变量自动泛化成隐式 binder | ❌ 不做（`decl-binders.md:92/159` 已列非目标） |
| `(x : T := e)` optional / `(x := by tac)` auto | 默认值 / tactic 合成 | ❌ 不做 |
| `..` 省略号 | 缺失实参全部元变量化 | ❌ 不做 |
| named arguments `(x := e)` | 按参数名选实参 | ❌ 不做（`Terms` 的算法里它是第一条分支，但没有它不影响隐式插入） |
| 高阶合一 | `?f x = t` 定 `?f` | ❌ 不做（与 §13.3 一致） |

### 7.3 我们要对齐的「一档」

> **教学子集 D3**：隐式参数 `{x : T}` 的实参**必须**由「后续显式实参的类型」或
> 「整体期望类型」**唯一确定**；确定不了就报教学错误（`elab-implicit-argument-unsolved`），
> **绝不猜、绝不引入元变量**。`@f` 关闭插入。其余隐式形态（strict / instance /
> 洞 / variable）明确不做。

这条档位与 Lean 的差别只有一处**实质**差异：Lean 允许「暂时解不出、留 `?m` 待定」，
我们要求「立刻唯一确定」。这与语言既有的「记法解不出就报」纪律一致
（`notation-subset.md:426`），也与 `spine.rs:210-214`「合一强度刻意只到这里」一致。

---

## 8. Q8 —— 对课程的具体影响

### 8.1 规模（实测统计）

| 项 | 数 |
|---|---|
| `courses/set-theory/` 全树的**显式类型实参拼写**（模式见 §8.4） | **3451**（`lib/` 239 + `units/` 3212） |
| 全树的**记法符号出现** | **776** |
| `lib/Set.sokonanoda` 声明数（**23/23 带前导类型参数**） | 23（12 def + 1 axiom + 10 theorem） |
| `lib/Set.sokonanoda` 记法命令（**11/11 指向带前导类型参数的常量**） | 11 |
| `lib/` 全部声明 / 记法 | 74 / 12 |
| prelude 里带前导类型参数的名字 | `Or.inl/inr`、`And.intro`、`Iff.intro/mp/mpr`、`Eq.*`、`Not.*`、`absurd`…（`prelude.rs:161-232`） |
| 单文件最密 | `units/solutions/unit08-solution.sokonanoda` 549 处；`units/solutions/unit12-solution.sokonanoda` 601 处；`units/unit08-images-preimages.sokonanoda` 224 处 |
| **非注释**行使用记法的文件 | 只有 5 个（unit01 8 行、unit03 1 行、notation-cheatsheet 18、其解答 30、solutions/unit01 13）——**其余 21 个画布 0 行**，课程今天仍以点名为主 |

### 8.2 `lib/Set.sokonanoda` 会变成什么样（改写示例）

```sokonanoda
-- 现在（courses/set-theory/lib/Set.sokonanoda:55,57,59,68,79-80,164-177）
def mem (α : Type) (a : α) (A : Set α) : Prop := A a
def subset (α : Type) (A B : Set α) : Prop := forall (x : α), A x -> B x
def empty (α : Type) : Set α := fun (x : α) => False
def union (α : Type) (A B : Set α) : Set α := fun (x : α) => Or (A x) (B x)
axiom ext : (α : Type) -> (A B : Set α) ->
  (forall (x : α), Iff (A x) (B x)) -> Eq.{1} (Set α) A B
infix:50 " ∈ " => Set.mem

-- 改写后（P2）
def mem {α : Type} (a : α) (A : Set α) : Prop := A a
def subset {α : Type} (A B : Set α) : Prop := forall (x : α), A x -> B x
def empty {α : Type} : Set α := fun (x : α) => False
def union {α : Type} (A B : Set α) : Set α := fun (x : α) => Or (A x) (B x)
axiom ext {α : Type} (A B : Set α) :
  (forall (x : α), Iff (A x) (B x)) -> Eq.{1} (Set α) A B
infix:50 " ∈ " => Set.mem        -- 记法声明照写（目标换签名，记法不用改）
```

**记法还写不写？——写。** 三条理由：

1. 记法是**符号层**（`∈` vs `Set.mem`），隐式实参是**参数层**，两件事正交
   （§1.5 实测已证明记法补参与风格无关）。
2. 记法命令指向的是**名字**（`=> Set.mem`），签名从 `(α) (a) (A)` 变成
   `{α} (a) (A)` 时**记法声明一行都不用改**——补参由新机械接管。
3. 课程教学大纲已拍板「记法为主，点名作为底层解释」（`course-lean-style.md:53` D4）。

**唯一要动的是**：`Set.ext` 这类「Lean 里 `a b` 是隐式」的签名（§6.3 覆盖矩阵），
以及把 `A B` 写成隐式后**原有显式调用点**（`Set.ext α A B h`）会失效——P2 的批量
改写要按 §8.3 的清单来。

### 8.3 调用点会简化多少（5 个真实例子，逐字摘自课程）

**(1) `units/unit08-images-preimages.sokonanoda:262-264`**（全卷最密签名）

```sokonanoda
theorem image_inter_subset (α β : Type) (f : α -> β) (A B : Set α) :
    Set.subset β (Set.image α β f (Set.inter α A B))
      (Set.inter β (Set.image α β f A) (Set.image α β f B)) :=
```
→ `theorem image_inter_subset {α β : Type} (f : α -> β) (A B : Set α) : f '' (A ∩ B) ⊆ f '' A ∩ f '' B :=`
（5 个 `α`/`β` 拼写里 4 个消失）

**(2) `units/unit08-images-preimages.sokonanoda:197-199`**（一行的可读性灾难）

```sokonanoda
      Iff.intro (Set.mem Two x (Set.inter Two (Set.singleton Two aa) (Set.singleton Two bb)))
        (Set.mem Two x (Set.empty Two))
        (fun (hx : Set.mem Two x (Set.inter Two (Set.singleton Two aa) (Set.singleton Two bb))) =>
```
→ `Iff.intro (x ∈ ({aa} ∩ {bb})) (x ∈ ∅) (fun (hx : x ∈ ({aa} ∩ {bb})) => …)`
（104 字符的类型变 ~20 字符）

**(3) `units/unit04-extensionality-identities.sokonanoda:45-46`**（`Set.ext` + 三个集合运算）

```sokonanoda
    Eq.{1} (Set α) (Set.union α A (Set.univ α)) (Set.univ α) :=
  Set.ext α (Set.union α A (Set.univ α)) (Set.univ α) (fun (x : α) =>
```
→ `Eq.{1} (Set α) (A ∪ Set.univ) Set.univ := Set.ext (fun (x : α) => …)`

**(4) `units/notation-cheatsheet.sokonanoda:188`**（prelude 形状，全课程到处都是）

```sokonanoda
  fun (x : α) => fun (hx : A x) => Or.inl (A x) (B x) hx
```
→ `fun (x : α) => fun (hx : A x) => Or.inl hx`
（同文件 `:220`/`:223` 还嵌套一层：`Or.inl (Or (A x) (B x)) (C x) (Or.inl (A x) (B x) hx)`
→ `Or.inl (Or.inl hx)`）

**(5) `units/solutions/unit08-solution.sokonanoda:48`**（prelude + Set + Image 三重叠加）

```sokonanoda
              Or.inl (Set.mem α x (Set.preimage α β f B)) (Set.mem α x (Set.preimage α β f C)) hb)
```
→ `Or.inl hb`（两个 `Or` 操作数与两个 `Set.preimage` 的类型参数全部可推）

**(6) `units/unit05-pairs-products.sokonanoda:100,107-109`**（`×ˢ` 的目标声明在画布里）

```sokonanoda
def Set.prod (A B : Type) (s : Set A) (t : Set B) : Set (Prod A B) :=
  fun (p : Prod A B) => And (s (Prod.fst A B p)) (t (Prod.snd A B p))
…
    Iff (Set.mem (Prod A B) p (Set.prod A B s t))
        (And (s (Prod.fst A B p)) (t (Prod.snd A B p))) :=
```
→ `Iff (p ∈ s ×ˢ t) (s (Prod.fst p) ∧ t (Prod.snd p))`

### 8.4 计数口径（复现用）

```bash
# 显式类型实参拼写（近似；含注释，不含 Eq.{1} 的宇宙拼写）
rg -o '(Set[.](mem|subset|empty|univ|singleton|pair|union|inter|sdiff|compl|powerset|image|preimage|prod)|Set[.](MapsTo|LeftInvOn|RightInvOn)|Rel[.](inv|comp)|Function[.](comp|Injective|Surjective|Bijective|LeftInverse|RightInverse|Inverse)|Or[.](inl|inr)|And[.]intro|Iff[.]intro|Exists[.](intro|elim|imp)|Prod[.](fst|snd)|prod_mk)[[:space:]]+[A-Za-zαβγ_(]' courses/set-theory/ | wc -l
# 记法符号出现
rg -o '∈|⊆|∪|∩|𝒫|ᶜ|×ˢ|∅|'"''"'|⁻¹'"'"'' courses/set-theory/ | wc -l
```

### 8.5 会被 P2 打破的既有测试/契约（同轮必须更新）

| 文件:行 | 钉的是什么 | P1/P2 怎么办 |
|---|---|---|
| `crates/front/src/compile/tests.rs:6584-6601` | 护城河：`Set.mem a A` ⇒ 恰好 `kernel-rejected` + stage Kernel | P1 改成「现在 checked」，并把护城河改钉成**新的**边界（例如「解不出时报 `elab-implicit-argument-unsolved`」） |
| `crates/cli/tests/notation.rs:162-177` | 同上（CLI 端） | 同上 |
| `crates/cli/tests/notation.rs:556-575` | 「课程仍用点名写法」（`Set.subset α A C` 必须出现） | P2 改写后改成「课程用隐式签名」断言 |
| `crates/cli/tests/notation.rs:287-313` | `notation_variant` 夹具逐字替换 unit02 的签名 | P2 同步改夹具 |
| `crates/cli/tests/notation.rs:108-135`、`:594-616` | 点名/记法两种写法**五元组计数相等** | 保持（N7 契约不动） |
| `crates/front/src/compile/tests.rs:1730-1753`（`L1_USER_SRC`） | prelude 显式拼写 `Or.inl a b ha` | P2 若改 prelude 签名则同步 |
| `crates/front/src/compile/tests.rs:3783-3798` | `apply Or.inl`（tactic 路径） | 不动 |
| `crates/cli/tests/query.rs:555-575` | unit05 硬计数 `decl_checked == 5` / `exercise_open == 7` | P2 若改 unit05 的声明形状，计数必须逐项不变（只改写法不改声明数） |
| `docs/TESTING.md:43` | 记法守护行（列了上面所有测试） | 同轮更新 |
| `docs/design/notation-subset.md:106-107,244,552` | N4.3 护城河 / §13.3「不做」 | 同轮改写（**必须**：设计先行纪律） |
| `docs/design/course-lean-style.md:49,285,660` | D2 / L2.9 / N-1「不做隐式实参」 | 同轮改写 |
| `docs/architecture.md:145,445-446` | 「仍未做：一般隐式实参推断」「签名显式给全参数」 | 同轮改写 |
| `courses/set-theory/units/notation-cheatsheet.sokonanoda:45-50` | 教学习者「点名省 `α` 会被拒」 | 同轮改话术 |
| `docs/design/decl-binders.md:92,159` | 「不做隐式泛化 / `autoBound`」 | 若只做插入不做 auto-bound，**不用改**（仍成立） |

---

## 9. Q9 —— 分阶段计划

每阶段的验收命令一律用**绝对路径**（G-12）与带 `DEVELOPER_DIR` 的 cargo
（本机 Xcode 许可未接受）。

```bash
# 通用验收（每阶段都跑）
DEVELOPER_DIR=/Library/Developer/CommandLineTools cargo test --workspace --locked
DEVELOPER_DIR=/Library/Developer/CommandLineTools cargo fmt -p sokonanoda-front -p sokonanoda-cli -p sokonanoda-lsp -- --check
DEVELOPER_DIR=/Library/Developer/CommandLineTools cargo clippy --workspace --all-targets
scripts/soko grade "$PWD/courses/set-theory/units/unit02-subsets-empty.sokonanoda"   # exit 0
python3 courses/set-theory/tools/check.py                                            # 36/329/99/0
git diff --stat -- crates/kernel/                                                    # 必须为空
```

### P0 —— 决策与回归护栏（本轮，只写文档）

**做什么**

1. 本文（`docs/notes/course-lean-style/implicit-args-plan.md`）。
2. 在 `docs/design/notation-subset.md` §13.3 与 `docs/design/course-lean-style.md`
   §10 N-1 各加一条**指针**（「隐式实参已单独立项，见 notes/course-lean-style/implicit-args-plan.md」）
   ——**这一步会改现有文件，属于 P0 的独立小提交**，不在本次只读调研范围内。
3. 决定两件事并写进 `REQUIREMENTS.md` §9（用户拍板）：
   - 对齐档位 = §7.3 的「D3 唯一确定档」；
   - 护城河重谈（`Set.mem a A` 从拒到过）。

**验收**：`git status` 只有新笔记（+ 上述指针改动）；测试全绿（代码零改动 ⇒ 必然绿）。

**可独立发布**：✅（纯文档）。

### P1 —— 路线 C 引擎（**可独立发布**）

**做什么**（按依赖序）

1. **签名表**：给 `GoalTemplates` 的 `FuncTemplate`/`CtorTemplate` 补
   `binder_styles: Vec<BinderKind>`（`goals.rs:24-48`），并在 `walk.rs` 把它作为
   `DeclSig` 穿进 `ElabCtx`（`elab.rs:263-276`）。prelude 的 L1/Eq 源文本
   （`prelude.rs:161-232`）同法登记。
2. **新模块 `crates/front/src/compile/implicit.rs`**：
   - `flatten_spine(&Expr) -> (head, Vec<&Expr>)`（复用 `spine::spine_of`）；
   - `align(layers, args) -> Vec<Slot>`（显式层按序吃实参，隐式层跳过）；
   - `solve_implicits(...) -> Result<Vec<Expr>, CompileError>`：同时头部匹配 +
     occurs check（复用/一般化 `unify_extract`，`elab.rs:1656-1700`）；
   - `build_app(builder, head, slots, expected) -> ExprPtr`；
   - 无隐式层时**直接走今天的 `mk_app` 链**（保证 no-op）。
3. **接线**：`elab_expr` 的 App 臂（`elab.rs:1955-1961`）改走 1–2，并把
   `expected`/`expected_src` 传下去（这是 `Or.inl hx` 能成的关键）。
4. **`@f` 真语义**：AST 标记（`parser.rs:2388-2406`）+ 各 `match` 补一处。
5. **诊断**：`error.rs` 新码 `elab-implicit-argument-unsolved` + hint +
   `docs/protocol.md` 词条。
6. **记法路径退化**：`elab_notation` 的补参换成调用同一份机械；
   `notation_prefix_args`/`solve_prefix_args`/`notation_operand_expected`/
   `set_literal_prefix_args` 删除或降为薄封装；**诊断码/stage/事件序列必须不变**。
7. **测试三层**：
   - front：对齐规则 / 同时匹配 / occurs / 解不出 / `@` / **无隐式签名 no-op** /
     记法两种写法五元组相等 / 「一处失败不连坐」（§1.3）；
   - CLI e2e：新文件 `crates/cli/tests/implicit.rs`（或并入 `notation.rs`）；
   - 语料/golden：`examples/`、`playground.sokonanoda`、课程门禁。

**验收**

```bash
scripts/soko grade "$PWD/courses/set-theory/units/unit02-subsets-empty.sokonanoda"  # exit 0，计数不变
python3 courses/set-theory/tools/check.py                                           # 36/329/99/0 逐项不变
python3 courses/set-theory/tools/check.py --selftest                                # exit 0
DEVELOPER_DIR=/Library/Developer/CommandLineTools cargo test --workspace --locked    # exit 0
# 新增能力探针（建议入库到 crates/cli/tests/ 的夹具里，或 examples/）
scripts/soko grade "$PWD/target/scratch/implicit-p1.sokonanoda"                      # exit 0
```

**可独立发布**：✅。理由：对**所有**现有程序（签名无隐式 binder）逐字节 no-op；
课程一行不改也能全绿。**唯一的行为变更**是「显式签名的点名省参」与「新写的 `{...}`
签名」，前者是护城河重谈（P1 同轮改测试与文档）。

### P2 —— prelude 与课程改写（可独立发布，但是课程契约变更轮）

**做什么**

1. **prelude**：`PRELUDE_L1_SRC` 的 `Or.inl/inr`、`And.intro/left/right/elim`、
   `Iff.intro/mp/mpr/refl/symm/trans`、`Not.*`、`absurd`、`Or.elim` 的**前导
   `Prop` 参数**改成 `{...}`（`prelude.rs:196-232`）；`Eq`/`Eq.refl`/`Eq.subst`
   已经是 `{α}`（`prelude.rs:161-165`），只需确认可用（**注意 §6.3 风险 5：
   宇宙多态目标仍受空宇宙层限制，`Eq` 可能要先做 L2.4b**）。
2. **课程库**：`lib/Set.sokonanoda`（23 声明）+ `lib/{Exists,Prod,Image,Rel,Fun,Equiv,Demo}.sokonanoda`
   的签名改隐式（§8.2 示例）。
3. **调用点**：`units/**` 与 `solutions/**` 按 §8.3 的 6 类形状批量改写；
   3451 处显式拼写预计去掉大半（保留点名作为「底层解释」的教学位置）。
4. **测试/文档同轮**：§8.5 的整张表。
5. **`courses/set-theory/README.md:126` 的计数**：改写**只改写法不改声明数** ⇒
   36 目标 / 329 checked / 99 open 应当**逐项不变**；变了就是 bug。

**验收**

```bash
python3 courses/set-theory/tools/check.py            # 36 目标 · 329 checked · 99 open · 0 判负
python3 courses/set-theory/tools/check.py --json     # 计数逐项比对
python3 courses/set-theory/tools/check.py --only "单元 8" --bisect
scripts/soko course "$PWD/courses/set-theory/course.json" --json
DEVELOPER_DIR=/Library/Developer/CommandLineTools cargo test --workspace --locked
scripts/soko gate                                    # 含课程门禁 + 缺口台账
```

**可独立发布**：✅，但它是**课程可见的破坏性变更**（老写法 `Set.mem α a A` 在新签名下
仍然可用吗？——**可用**：显式给隐式位传参在 Lean 里也合法，所以旧写法不会失效，
只是多余。这一点让 P2 可以**渐进改写**，不必一刀切）。

### P3（可选，未排期）—— 元变量/一般合一（路线 A）

只有出现「路线 C 明确覆盖不到、且不能靠改签名规避」的真实课程需求时才立项。
触发时先写 spike（≤ 3 天），量三件事：① 现有 7926 行 front 测试要改多少；
② 每次应用的平均 judge 次数；③ 「解不出」的教学诊断质量是否退化。

---

## 10. 风险登记册（按严重度）

| # | 风险 | 影响 | 缓解 |
|---|---|---|---|
| R1 | 护城河重谈（`Set.mem a A` 从拒到过） | 中：学习者话术 + 2 条硬测试 + 3 份设计文档 | P0 拍板 → P1 同轮改；新护城河改钉「解不出报专用码」 |
| R2 | 记法路径语义漂移（两种写法判卷不一致） | 高：N7 契约 | 记法补参**委托**给同一份机械，保留诊断码/stage；`cli/tests/notation.rs` 五元组断言不许放宽 |
| R3 | 签名表来源选错（第三张表/内核调用） | 中：性能 + 维护 | 复用 `GoalTemplates` 形制（零内核调用，项目模式下已是闭包合成表） |
| R4 | `render_expr` round-trip 错位（`elab.rs:328-337`） | 中：静默错解 | 新机械优先用**内核项**（`ElabScope::tys` + `infer_under_binders`，`quote.rs:36-55`），少走文本 |
| R5 | 宇宙多态目标（空宇宙层，`notation-subset.md:499-513`） | 中：`Eq`/`{u}` 常量仍不可用 | 显式列为**不做**；需要时先做 L2.4b（`course-lean-style.md:279`） |
| R6 | 一处失败连坐（§1.3） | 低-中：诊断归因错行 | 不修 bug，但加回归断言；省参合法化后窗口变小 |
| R7 | `elab.rs` 继续膨胀（已 4184 行，模块化纪律 ~500 行） | 低-中 | 新代码全部进 `compile/implicit.rs`，`elab.rs` 只改 App 臂 |
| R8 | `@f` 静默改义 | 低（语料 0 处） | P1 顺手做真语义 |
| R9 | 「`{α}` 在签名里但课程没改」造成的**双轨**状态 | 低：P1 后课程仍显式，读起来不一致 | P1 发布说明写清「引擎先行、课程 P2 跟进」；两阶段都可独立发布 |

---

## 11. 复现命令（本次调研用过的）

```bash
# 1) 内核判卷通道（MCP 等价物）：把 §1 的每个片段喂给 check
#    mcp__sokonanoda__check { text: "<片段>" }        → counts + failed[]
#    mcp__sokonanoda__goals { text: "<片段>" }        → 内核渲染的 ty（§1.4 用它看 {α} 风格）

# 2) 单文件/单元判卷（绝对路径，G-12）
target/debug/sokonanoda grade "$PWD/courses/set-theory/units/unit02-subsets-empty.sokonanoda" --json

# 3) 课程门禁
python3 courses/set-theory/tools/check.py

# 4) 证据定位
grep -n "Expr::App { fun, arg, span }" crates/front/src/compile/elab.rs        # :1955
grep -n "fn elab_notation" crates/front/src/compile/elab.rs                    # :991
grep -n "fn unify_extract" crates/front/src/compile/elab.rs                    # :1656
grep -n "pub enum BinderStyle" -A 12 crates/kernel/src/expr.rs                 # :126 风格仅 pp
grep -n "pub fn infer_under_binders" crates/kernel/src/quote.rs                # :36 公开推断原语
grep -n "pub fn try_check_declar" crates/kernel/src/util.rs                    # :662 不 panic 的终审
```

---

## 12. 一页纸摘要

- **缺的不是语法，是应用位的插入**；`{α : Type}` 从 parser 到内核 pp 全线已通。
- **记法 hack 已经能穿隐式前导参数**（实测），它缺的只是「按风格对齐」这一条；
  **点名省参**才是被内核拒的那个（护城河）。
- **推荐路线 C**：显式实参按风格对齐到显式层、跳过的隐式层由「实参类型 + 期望类型」
  同时头部匹配唯一确定；解不出报专用码，**不引入元变量**。400–600 行生产代码。
- **路线 A 被内核没有元变量堵死**（只能做前端 meta + zonk，1200–2500 行，重写每个 App）；
  **路线 B 不是独立路线**（elaborate 期没有 env ⇒ 每次探针＝整前缀重编译）。
- **P1 可独立发布**：对无隐式签名的应用逐字节 no-op ⇒ 课程零改动也全绿。
- **P2 是课程改写轮**：`lib/` 74 声明改隐式、3451 处拼写大幅缩短、记法声明**照写**；
  计数应当逐项不变（36/329/99/0）。
- **最大风险 = R1 护城河重谈 + R2 记法语义漂移**，两者都在 P0 拍板、P1 同轮销账。

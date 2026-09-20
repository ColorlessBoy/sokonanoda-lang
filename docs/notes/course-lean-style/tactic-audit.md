# 课程改写前的 tactic 引擎审计（只读调研）

> 日期：2026-09-19（版本 0.61.0，HEAD `084da05`，工作树干净）。
> 触发：用户要求把 `courses/set-theory/`（35 文件 / 6550 行，今天 **100% 项模式**）
> 全部改写成 tactic（`by`）证明。本文回答「引擎今天支持到什么程度、缺什么」。
>
> **方法**：精读 `crates/front/src/by.rs`（543 行全文）、`ast.rs` 的 `Tactic`、
> `parser.rs` 的 tactic 解析、`docs/design/by-tactics.md`、`proof.rs`、`judge.rs`
> 的合成/回读、`spine.rs` 的合一、`compile/{check,goals,report}.rs` 的接入、
> `query/state.rs` + LSP 的 goal 面板；对课程做 grep 普查；**并对每条结论做实测**
> （`scripts/soko grade` / `query check` / `query state` / `--json`，临时文件全部在
> `/tmp/tact/`，仓库零改动）。实测原始记录见 §10，复现清单见 §11。
>
> **一句话结论**：今天的 `by` 引擎是一个**只能剥 Pi、只能判术语、不能改目标/上下文**
> 的 6-tactic 引擎；把课程改写成 Lean 4 风格**必须新增 tactic**（最低 `cases` 家族 +
> `constructor`/`use` + `rw`），但**纯机械改写今天就能做**——`:= <旧项>` → `:= by exact <旧项>`
> 已实测 100% 可用（§4.3），代价是学习体验（goal 面板只有一步）。

---

## 0. 一页结论

| 问题 | 结论 |
|---|---|
| 白名单 | **7 个关键字 / 6 个 AST 变体**：`intro` / `exact` / `apply` / `assumption` / `rfl` / `match`（不是独立变体，解析期降级成 `Tactic::Exact{Expr::Match}`）/ `sorry`（`parser.rs:2841-2846`，`ast.rs:293-317`） |
| `intro` 多名字 | **不支持**：`intro a b` 是 **parse 错误**，整份文件判红（实测 E01） |
| `exact` 吃任意表达式 | **是**（`parse_expr`，`parser.rs:1254-1262`）；`_` / `?_` 洞**不支持**（`token.rs:248`） |
| `apply` 合一 | **只做位置 spine 合一**：头同名 + 实参个数相等（`spine.rs:134-157`）；**两个真实 bug** 会让常见形状失败（§1.4、§6） |
| `match` 臂体 | **只能是项，不能是一串 tactic**（`parser.rs:1282-1291` → `parse_match`；实测 M2/M3/M4） |
| `by` 没写满 | **合法 Open**（尾部自动 `sorry`），`exercise.open`、退出码 0（实测 E03/E04） |
| `by sorry` vs `:= sorry` | **事件流逐字节相同**（实测 A：`diff` 为空），洞 span 各指自己的 `sorry` token |
| `sorry` 之后继续写 | **可以**，且**没有任何 warning**（多余 `sorry` 是 no-op，实测 E14/E43） |
| 空 `by` 块 | **可以**（`parser.rs:1188-1189`；实测 E03） |
| 分隔 | `;` 或换行；**缩进完全不敏感**；**同一行两个 tactic 必须写 `;`**（实测 E08/E09/E11/E13） |
| 嵌套 `by` | **不支持**（`exact by …` → `unknown identifier by`，实测 E15）；`fun (x) => by …` **支持**（`parser.rs:2459-2462`，实测 E16） |
| parse 错误的爆炸半径 | **整份文件**：一处未知 tactic ⇒ `decl_checked=0`，前面写对的声明也全丢（实测 PE1）——课程改写期最危险的性质 |
| 编辑器 goal 面板 | **可用且好用**：`ByStep` 每步记录全部剩余目标 + 上下文，`soko/stateAt` 按 Lean `goalsAt?` 语义选「进入该 tactic 的状态」，VS Code 练习树 + Infoview 已消费（§7） |
| 课程需求 | `fun` 100 处（`intro` 可吃）、`Iff.intro` 34、`Set.ext` 34、`And.intro` 12、`Or.elim` 27、`Exists.elim` 76、`Eq.subst` 55、`False.elim` 19、`match` 14、`let` 28（普查见 §4.1） |

---

## 1. Q1 —— 白名单与每个 tactic 的确切语法

### 1.1 白名单的唯一来源

```rust
// crates/front/src/parser.rs:2841-2846
fn is_tactic_keyword(name: &str) -> bool {
    matches!(
        name,
        "intro" | "exact" | "apply" | "assumption" | "rfl" | "match" | "sorry"
    )
}
```

`Tactic` 枚举只有 **6 个变体**（`crates/front/src/ast.rs:293-317`）：
`Intro{name}` / `Exact{expr}` / `Apply{expr}` / `Assumption` / `Rfl` / `Sorry`。
**没有 `Match` 变体**——`match` 在解析期就降级成 `Tactic::Exact{ Expr::Match }`
（`parser.rs:1282-1291`），所以「白名单 7 个写法 / 6 个 AST 变体」。

引擎的派发在 `by.rs:162-288`（`match tactic`）：`Intro` 168-183、`Exact` 184-215、
`Assumption` 216-242、`Rfl` 243-282、`Apply` 283-285、`Sorry` 287。

未知关键字 ⇒ `unexpected-token`（stage `parse`），消息里带白名单
（`parser.rs:1296-1298`）：

```
未知 tactic：`by` 块只支持 intro / exact / apply / assumption / rfl / match / sorry（白名单），发现 …
```

### 1.2 `intro`

| 项 | 结论 | 证据 |
|---|---|---|
| 语法 | `intro <ident>`，**恰好一个名字** | `parser.rs:1243-1252`（`self.bump()` 吃下一个 token 当名字，不检查它是不是 tactic 关键字） |
| 一次多个名字 | **不支持**，`intro a b` ⇒ parse 错误 | 实测 E01 |
| 无名字 | `intro` 会把**下一行的 tactic 关键字**当名字吃掉 | 实测 E02（`intro` 吃掉 `exact`，随后 `a` 成为游离 token） |
| 目标要求 | 目标必须是 `Forall` 或 `Arrow`，否则 `elab-tactic-failed` | `by.rs:169-174`；实测 probe `intro on non-Pi` |
| 多 binder 的 `forall` | **逐 binder 剥**，`forall (a b : Prop), …` 可以一次 `intro a` | `spine.rs:21-42`（余下重包成 `Forall`）；实测 F1 |
| 声明 binder | 自动进初始上下文，**不需要也不该再 `intro`** | `by.rs:131-160`；实测 H4 vs E01；测试 `compile/tests.rs:1259` |
| 模式 binder | `intro ⟨x,y⟩` / `intro (x,y)` **不支持**（parse 错误） | 实测 probe `intro pattern`；课程普查：**0 处模式 lambda** |

### 1.3 `exact`

- 语法：`exact <expr>`，`<expr>` 就是普通表达式（应用、lambda、`let`、`match`、
  记法、`@` 显式应用都行）——`parser.rs:1254-1262` 直接调 `parse_expr`。
- 判定：`judge_terms` 合成完整声明交**完整内核**（`by.rs:184-215` → `judge.rs:179`）。
  成功 ⇒ 节点 `Closed`；类型不匹配 ⇒ `elab-tactic-failed` + 内核渲染的期望/实际；
  未知标识符等 ⇒ `elab-tactic-failed` + 内核消息。
- **洞不支持**：`exact _` ⇒ `unknown identifier _`；`exact ?_` ⇒ 词法期就拒绝
  （`token.rs:248`：`??? 已移除：未完成的证明请写 sorry（与官方 Lean 一致）`）。
  实测 probe `exact hole _` / `exact ?_`。
- `exact let x : T := v; body` **可用**（实测 F4）——`have` 今天的替代写法。
- 实测 E30：`exact And.intro a b h1 h2` ✓；C2：`exact Set.ext α A B (fun x => …)` ✓。

### 1.4 `apply` —— 位置 spine 合一 + 两个真实 bug

**机制**（`by.rs:317-410`）：

1. `judge_infer(prefix, options, ctx_binders, "f")` 合成
   `#check fun (b1:T1)… => f` 走完整流水线，取 `TypeChecked` 文本，再剥掉
   n 层 binder（`judge.rs:422-535`）⇒ f 的类型**文本**；
2. 文本回读成 AST（`by.rs:341-350`），剥 Pi 链得 `layers: Vec<(name, domain)>`
   + `codomain`；
3. `unify_spine(codomain, goal, layers)`（`spine.rs:134-157`）：**头同名且实参个数
   相等**，然后把「codomain 实参位上等于某个 Pi binder 名的位置」映射到 goal 的
   对应实参，得 σ；**不做高阶合一、不做隐式参数推断**；
4. **出现在 codomain 里的命名 binder = 类型参数**（σ 填，填不到就退化成同名
   `Ident`）；**其余 = 子目标**（域类型 σ 代入）——`by.rs:377-398`；
5. 子目标**反序压 worklist**（先解第一个，`by.rs:399-402`）；
6. 判定仍走 kernel：子目标闭合后整体 `assemble` 成项交内核（`by.rs:414-444`）。

**实测能用的形状**（都是 `apply <ctor>` + 逐个子目标 `exact`）：

| 形状 | 实测 | 备注 |
|---|---|---|
| `apply And.intro`（2 子目标，顺序 `a` 后 `b`） | E20 ✓ / E21 反序报错 ✓ | 测试 `compile/tests.rs:3847` 钉死顺序 `["P","Q"]` |
| `apply Or.inl` / `Or.inr`（σ 从目标补 `B`） | E22 ✓ / E23 ✓ | 简单上下文里可用 |
| `apply Iff.intro` | A1 ✓（课程库） | 2 子目标 `A -> B`、`B -> A` |
| `apply Exists.intro`（课程 `lib/Exists`） | C3 ✓ | 2 子目标：证人、证据 |
| `apply <用户归纳构造子>`（3 字段） | W6 ✓（`def` 里） | 等价于 `constructor` 的手工版 |
| `apply Or.elim`（目标是**同名变量** `c`） | A3 「成功」但留 5 个垃圾子目标 | 只是撞名巧合 |
| `apply Set.ext` | A2 ✗ / C1 ✗ | **两个独立 bug，见下** |
| `apply Or.elim` / `Exists.elim` / `False.elim` / `And.left` | A4/A5/A6 ✗ 或垃圾子目标 | 消去子/投影**不能**用 `apply` 代替 `cases` |

**bug ①（渲染不可回读）**：`judge.rs:557-575` 的 `render_roundtrip` 只对**顶层**
`Forall` 逐 binder 展开成箭头链；嵌在箭头**值域位**的 `Forall` 会落到
`render_expr`（`proof.rs:251-254`）被渲染成 `(A : Prop) (B : Prop) -> …`
（**没有 `forall` 关键字**），回读必失败：

```
$ scripts/soko grade /tmp/tact/v2.sokonanoda
{"code":"elab-tactic-failed","message":"无法解析 `Or.inl` 的类型：expected `->` after binder group, found LParen"}
```

最小复现（`v2`）：

```lean
theorem t (A B : Prop) (C : Nat) (h : A) : Or A B := by
  apply Or.inl      -- ← 报「无法解析 Or.inl 的类型」
  exact h
```

内核 pp 文本（`#check fun (A B : Prop) (C : Nat) (h : A) => Or.inl` 实测）：

```
forall (A : Prop), Prop -> Nat -> A -> (forall (A B : Prop), A -> Or A B)
```

即：**未被用到的 context binder 被 pp 渲染成匿名箭头**，把类型切成两个 `forall`
组；剥 binder 时落在箭头链里，第二组 `forall` 被渲染成不可回读的文本。
→ 触发条件与「上下文里有没有被 pp 变成箭头的 binder」有关，**同一段代码换个上下文
就可能从通过变失败**（`apply Or.inl` 在 E22 通过、在 N1/v2 失败）。

**bug ②（头形状不一致）**：pp 会把 `Eq.{1} (Set α) A B` 打印成 `Eq A B`
（丢掉层级与隐式实参），而 `same_head`（`spine.rs:66-83`）要求
`Ident==Ident` 或 `UniverseApp==UniverseApp` 且层级相等 ⇒ **假报不匹配**：

```
$ scripts/soko grade /tmp/tact/course/a2.sokonanoda
{"code":"elab-tactic-failed","message":"`apply` 的目标不匹配：`Set.ext` 的结果是 `Eq A B`，无法对齐当前目标 `@Eq.{1} (Set α) A B`"}
```

课程 682 处显式 `Eq.{1}` 写法都会踩这个形状（只要用 `apply`）。
注意：`canonical_goal_type`（`by.rs:89-107`，G-05）**只对用了
`namespace`/`open` 的文件**启用（walk.rs 的 `canonical_goal`），普通课程文件不走，
所以两边文本没有对齐。

### 1.5 `assumption`

- 语法：`assumption`（无参数，`parser.rs:1272-1277`）。
- 语义：沿**当前节点的父链**收集全部 binder 名（`by.rs:217-222`），从**最内层**
  起逐个交内核判（`judge_terms`），第一个 `Match` 的填进去（`by.rs:223-241`）。
- 失败 ⇒ `elab-tactic-failed`「`assumption` 没有找到类型与目标一致的假设」（实测 E26）。
- 课程里 0 处显式使用（项模式时代用不到），但改写后是主力。

### 1.6 `rfl` —— **只在 `Eq.{u} α x y` 这种「显式层级 + 显式 α」写法上可用**

- 语法：`rfl`（无参数，`parser.rs:1278-1281`）。
- 语义：目标 AST 必须**形状匹配** `Eq α x y`（3 个显式实参！），造
  `Eq.refl.{level} α x` 交内核判（`by.rs:243-282`、`rfl_candidate` 498-530）。
- **限制**（实测）：

| 目标写法 | `by rfl` | 说明 |
|---|---|---|
| `Eq.{1} Nat (1 + 1) 2` | ✓ E41 | 课程 `course/unit4-by-tactics.sokonanoda:46` 的活样例 |
| `Eq.{1} Prop a a` | ✓ E40 | |
| `Eq a a`（无层级） | ✗ E31 | `rfl_candidate` 对裸 `Ident("Eq")` 硬编码层级 `"0"`，与内核推断不一致 |
| `Eq My A A`（`My : Type`） | ✗ E37/E39 | 同上（内核推出 `Sort(1)` 的层级） |
| `Eq Prop a a` | ✗ E36 | 报「两边不相等（期望 `Sort(0)`，实际 `Sort(1)`）」 |
| 非 `Eq` 目标 | ✗ E27/E28 | 「`rfl` 需要一个 `Eq α x y` 形状的目标」 |

→ **结论**：`rfl` 今天不是「等号两边算出来一样就成立」的通用 tactic，而是
「必须按 `Eq.{u} α x y` 逐字写全」的窄 tactic。课程 682 处 `Eq.{1}` 恰好是这种写法，
所以 `by rfl` 在课程里**多数能用**，但学生写 `Eq a a` 会莫名其妙失败（错误消息也
指向「形状」而不是「层级」）。

### 1.7 `match`（作为 tactic）

- 语法：`match <scrutinee> with | <pat> [if <guard>] => <项> …`，**臂体是项**
  （`parser.rs:1282-1291` → `parse_match`；测试 `parser.rs:3701` 断言它降级成
  `Tactic::Exact{ Expr::Match }`）。
- 臂体**不能**是一串 tactic：M3 实测在第二个 `|` 处 parse 错误；M2 实测把
  `exact Eq.refl…` 当成项 ⇒ `unknown identifier exact`。
- 正确写法（实测 M4/M5 ✓，两题等价）：

```lean
theorem t (x : Two) : Eq.{1} Two (f1 x) aa := by
  match x with
  | aa => Eq.refl.{1} Two aa
  | bb => Eq.refl.{1} Two aa
-- 或 by exact match x with | aa => … | bb => …
```

- 被匹配项必须是**已知归纳类型**（文件内 `inductive` 或 prelude 的 `Nat`/`Bool`），
  否则 `elab-tactic-failed`（probe `match arms tactic`）。

### 1.8 `sorry`（作为 tactic）

- `Tactic::Sorry` 在引擎里是**纯 no-op**（`by.rs:286-287`：注释「当前目标保持开放
  （no-op，节点仍是 Hole）」）。
- 它**不产生洞**、**不记录**、**不报 warning**；真正决定 open/checked 的是
  「`assemble` 出来的项里还有没有 `Hole` 节点」（`by.rs:414-444`，叶子 Hole 的
  span = 最后一个 tactic 的 span，`hole_span` 311-313）。
- 与 `exercise.open` 的关系：`by` 值降级后仍走既有分流——有洞 ⇒ `open_goal` ⇒
  `PendingOp::OpenExercise` ⇒ `exercise.open` 事件 + `DeclStatus::Open`
  （`compile/check/mod.rs:300-318`、`docs/design/by-tactics.md:102-106`）。
- `by sorry` 与 `:= sorry` 的**事件流逐字节相同**（实测 A），`query goals` 的
  `goal`/`holes`/`redundant` 也一致，只有洞的 **span** 各指自己的 `sorry` token
  （41-46 vs 36-41）——这是正确行为，不是不一致。
- **签名门禁一样**：`theorem t : 3 := by sorry` 与 `:= sorry` 都报
  `kernel-expected-sort`、都**不发** `exercise.open`（测试 `cli/tests/protocol.rs:366-401`）。

---

## 2. Q2 —— `by` 没写满 / `sorry` 之后继续 / 空块

| 情形 | 结果 | 实测 |
|---|---|---|
| `:= by`（空块，块尾就是 EOF） | `exercise.open`，exit 0 | E03；`parser.rs:1188-1189`；测试 `compile/tests.rs:3911` |
| `:= by intro a`（tactic 没写满） | `exercise.open`，exit 0（**尾部自动 `sorry`**） | E04；测试 `compile/tests.rs:3801` |
| `:= by sorry` | `exercise.open`，exit 0 | E05 |
| `:= sorry` | 同上，事件流逐字节相同 | E06 + 实测 A（`diff` 空） |
| `by sorry` 之后继续写 tactic 并证完 | **`decl.checked`**，**无 warning** | E14/E43（`redundant-sorry` 只覆盖**项位**多余 `sorry`，`kernel_phase.rs:161`；`by` 里的 `sorry` 是 no-op，不参与） |
| 所有目标已闭合后再写 tactic | `elab-tactic-failed`「by 块里没有待解目标」 | E29（`by.rs:164-166`、325-327） |
| `apply` 出多子目标后只解一半 | `exercise.open`，洞 span 落在块尾 | A5（`apply And.left` 留 2 个子目标）/ A3（`apply Or.elim` 留 5 个）；测试 `compile/tests.rs:3847` |

---

## 3. Q3 —— 分隔规则（`;` / 换行 / 缩进 / 一行多个 / 嵌套）

解析器：`parse_by_block`（`parser.rs:1184-1213`）+ `starts_atom` 的 `by_depth` 分支
（`parser.rs:2096-2100`）+ `next_line_starts_a_tactic`（`parser.rs:1228-1231`）。

| 写法 | 结果 | 证据 |
|---|---|---|
| 换行分隔（不带 `;`） | ✓ | E12；`parser.rs:1199` |
| `;` 分隔 | ✓ | E11；`parser.rs:1193-1196` |
| **同一行两个 tactic 不写 `;`** | ✗ parse 错误（`intro a exact a` 被读成应用/游离 token） | E08 |
| 同一行用 `;` 写多个 | ✓ | E11（`by intro a; intro h; exact h` 一行） |
| 缩进 | **完全不敏感**（0/8 空格混排都行） | E13 |
| 多行**应用**当一个 tactic | ✓（续行不以 tactic 关键字开头就照常拼接） | E17；测试 `parser.rs:3792` |
| 续行**以 tactic 关键字开头** | 会被切开（有意取舍） | `docs/design/by-tactics.md:197-201` |
| `sorry` 在下一行 | 视为新 tactic（同上） | 同上 |
| 嵌套 `by`（`exact by …`） | ✗ `unknown identifier by`（`by` 不是表达式原子） | E15 |
| `fun (x) => by …`（lambda 体尾） | ✓ | E16；`parser.rs:2459-2462`；测试 `compile/tests.rs:1153/1171/1187` |
| `example … := by` / `def … := by` | ✓ | F2（example）/ F3（def）/ D1（`def n : Nat := by exact 1` ✓） |

**注意**：`parse_by_block` 的终止条件是「下一个 token 不是 tactic 关键字」，
所以 `by intro a` 后面跟的**任何非白名单词**都会被当成「块结束」→ 报「expected a
.sokonanoda command」（E01/E08 的实际错误消息就是这样），学生看到的错误位置
**在关键字之后的那个 token 上**，而不是「未知 tactic `constructor`」。

---

## 4. Q4 —— 课程改写必须补哪些 tactic

### 4.1 课程实测需求（`courses/set-theory/`，35 文件 / 6550 行，**代码计数**已剔注释）

`:= by` 出现 **0** 次（9 个 `by` 全在注释里）——课程 **100% 项模式**，且 `:=` 后
**换行**再写证明体（392 处悬挂 `:=`）；用户描述的 `:= And.intro …` 单行形状 **0 处**。

| 今天的证明头 | 代码出现 | 文件 | 改写后需要的 tactic | 今天能用吗 |
|---|---|---|---|---|
| `fun (x : T) => fun (hx : …) => …` | **100** 个证明体 / 1210 个 lambda（**0 模式 binder**） | 32 | `intro`（逐个名字） | ✅ 可用（E12） |
| `Iff.intro A B mp mpr` | 78 用 / 34 体 | 22 | `constructor` / `apply Iff.intro` | ✅ `apply` 实测 A1 ✓ |
| `And.intro` | 85 用 / 12 体 | 17 | `constructor` / `apply And.intro` | ✅ E20 ✓ |
| `Set.ext α A B (fun x => …)` | 42 用 / 34 体 | 13 | `ext` / `apply Set.ext` | ❌ `apply` 双 bug（A2/C1）；`exact Set.ext …` ✅ C2 |
| `Exists.intro A p w hw` | 54 用 / 3 体 | 13 | `use w` / `apply Exists.intro` | ✅ `apply` 实测 C3 ✓ |
| `Exists.elim A p Q h f` | 76 用 / 6 体 | 12 | `cases` / `obtain ⟨w,hw⟩ := h` | ❌ 只能 `exact Exists.elim …` |
| `Or.elim a b c f g h` | 27 用 | 9 | `cases` / `rcases h with inl ha \| inr hb` | ❌ 只能 `exact Or.elim …` |
| `Or.inl` / `Or.inr` | 42 / 23 用 | 9 / 8 | `left` / `right` / `apply Or.inl` | ⚠️ `apply` 上下文一变就炸（N1/v2） |
| `Eq.subst α (fun x => …) a b h proof` | 55 用 | 17 | `rw [h]` / `simp` | ❌ 只能 `exact Eq.subst …` |
| `Eq.symm` / `Eq.trans` / `congrArg` / `Eq.refl` | 43 / 27 / 19 / 59 | 15 / 10 / 10 / 18 | `rw` / `calc` / `rfl` | ⚠️ `rfl` 只认 `Eq.{u} α x y`（§1.6） |
| `False.elim` / `absurd` | 19 / 3 | 8 / 2 | `exfalso` / `contradiction` | ❌（`apply False.elim` 实测 A6 失败） |
| `Not (…)`（`fun (h : ¬P) => …`，9/13 是 `fun` 形） | 177 用 | 25 | `intro h`（已有） | ✅ |
| `match x with \| … => <项>` | 14 用 | 6 | `match`（tactic） | ✅ M4 ✓（臂体必须是项） |
| `let g : T := v; body` | 28 用 | 2 | `have g : T := v` | ⚠️ 只能 `exact let …; …`（F4 ✓） |
| `Nat.rec` | **1** 用 | 1 | `induction` | 不需要 |
| `And.elim` / `False.rec` / `rfl`（项）/ `▸` / `rw` / `⟨_,_⟩` / `exists` / `induction` / `Nat.le_induction` | **0** | 0 | — | 课程今天**不依赖**这些写法 |

### 4.2 逐 tactic 评估（按**必要性**排序）

> 「今天必须写什么」= 不改引擎时该形状的唯一/最省写法（全部实测）。
> 「实现难度」= 在 `crates/front/src/by.rs` 内做的代价（内核 `crates/kernel` 零改动，
> 判定永远走 kernel 合成声明）。

| # | tactic | 课程需求 | 今天必须写 | Lean 4 会写 | 实现难度 | 必要性 |
|---|---|---|---|---|---|---|
| 1 | **`cases` / `rcases` / `obtain`** | `Exists.elim` 76 + `Or.elim` 27 + `match` 14 + `And` 解构（≈120 处） | `exact Exists.elim A p Q h (fun w hw => …)` | `obtain ⟨w, hw⟩ := h` / `rcases h with ha \| hb` | **高**：目标变换（motive）+ 分支上下文（见 §5） | **必须**：消去是课程的骨架，且没有等价的 tactic |
| 2 | **`constructor`（+ `left`/`right`）** | `Iff.intro` 34 + `And.intro` 12 + `Set.Equiv.mk` 8 + 用户归纳 | `apply Iff.intro` / `apply And.intro`（**今天可用**） | `constructor` / `left` / `right` | **中**：需要归纳表（`ElabCtx.inductives` 已有，walk.rs:325）传进引擎 + 按目标头选构造子；`left/right` 是 `apply Or.inl/inr` 的别名 | **强烈建议**：`apply <ctor>` 要求学习者背构造子名与参数，且受 §1.4 两个 bug 影响 |
| 3 | **`use` / `refine ⟨…⟩`（洞）** | `Exists.intro` 54 | `apply Exists.intro` 然后 `exact w` | `use w` / `refine ⟨w, ?_⟩` | **中**：`use` 可在 `judge_infer` 出目标头的 intro 规则后复用 `apply` 机制；`refine` 需要洞支持（今天 `?` 被显式移除，`token.rs:248`），且要**给洞算期望类型**——`goals.rs:1125-1219` 已有按望远镜走查 + 内核探针的现成机制可借 | **建议**：`use` 小、收益直接；`refine` 是 `constructor`/`cases` 的通用底座 |
| 4 | **`rw` / `simp` / `unfold` / `change`** | `Eq.subst` 55 + `Set.subset_def` 一类展开引理（`Set.subset` 176 处） | `exact Eq.subst α (fun x => …) a b h proof` | `rw [h]` / `simp [Set.subset]` | **很高**：需要「在目标里找 LHS → 抽象成 motive lambda → 换目标」，引擎今天**没有任何目标改写能力**（§5）；`simp` 需要引理集，不建议；`change` 只需「新目标与旧目标 defeq」的内核判定，**中** | **必须（窄版）**：至少 `rw [h]`（单条等式、首个出现、非依赖 motive）。`simp`/`push_neg` 建议明确不做 |
| 5 | **`have`** | 28 处 `let` + 所有多步证明的可读性 | `exact let h : T := t; …`（F4 ✓）或把中间结论拆成独立 `theorem` | `have h : T := t` | **中**：`GoalNode.intros` 只有类型没有值（`by.rs:57-62`）；judge 合成只造 `Forall`/`Lambda` 望远镜（`judge.rs:969-1022`）⇒ 需要给 `GoalBinderSpec` 加 `body`、给 `assemble` 加 `Expr::Let` 节点 | **强烈建议**：没有 `have`，任何长证明只能写成一个巨型 `exact` |
| 6 | **`exfalso` / `contradiction`** | `False.elim` 19 + `absurd` 3 | `exact False.elim <goal> h` | `exfalso` / `contradiction` | **低**：`exfalso` = 把当前目标换成 `False` 并在 `assemble` 时套一层 `False.elim`（≈30 行）；`contradiction` 需要在假设里找 `P`/`¬P` 对，用 `judge_terms` 逐个判（中） | **建议**：`exfalso` 便宜且课程常用 |
| 7 | **`ext`（集合外延）** | `Set.ext` 42 | `exact Set.ext α A B (fun x => …)` | `ext x` | **低-中**：本质就是 `apply Set.ext`——**修 §1.4 的 bug ②（头形状对齐）比新写 `ext` 更划算**；若要泛化到任意 ext 引理则需要引理名约定/表 | **建议**：优先修 `apply`，`ext` 可作为 `apply <ext 引理>` 的语法糖 |
| 8 | **`show` / `suffices`** | 0 处显式需求（但改写后可读性） | — | `show T` / `suffices h : T by …` | `show`：**中**（要判「新目标与旧目标 defeq」，可交 kernel）；`suffices`：**中-高**（= `have` + 目标替换） | **可选** |
| 9 | **`subst` / `injection` / `congr` / `funext`** | 0 处直接需求；`congrArg` 19 处可用 `rw`/`exact congrArg …` 覆盖 | `exact congrArg …` | `subst h` / `injection h` | `subst` **中**（要把等式代进上下文与目标）；`injection` **高**（需要构造子单射性表）；`funext` **做不了**：prelude 没有 `funext` 公理（课程用 `Set.ext` 公理替代，`lib/Set.sokonanoda:79`） | **不需要**（今天） |
| 10 | **`induction`** | `Nat.rec` **1** 处 | `exact Nat.rec.{0} …` | `induction n with …` | **高**（= `cases` + 归纳假设 + 目标变换） | **不需要**（除非课程扩到归纳法单元） |
| 11 | **`specialize` / `apply … at`** | **0** | — | `specialize h x` | 中（改上下文） | **不需要** |
| 12 | **`trivial` / `tauto`** | **0** | `exact True.intro` / `assumption` | `trivial` | `trivial` **低**（依次试 `True.intro`/`assumption`/`rfl`）；`tauto` **高** | **可选**（`trivial` 便宜，教学友好） |
| 13 | **`calc`** | 0（`Eq.trans` 27 可受益） | `exact Eq.trans …` | `calc a = b := …` | **中-高**：需要项级 `calc` 语法 + 链式类型检查（新语法，三件套） | **可选** |
| 14 | 组合子（`<;>` / `·` / `all_goals` / `repeat` / `try` / `first` / `done`） | 0 | 逐个目标手写 | `·` 聚焦 / `<;>` | `·` 聚焦：**中**（解析 + 目标焦点栈）；其余 **低-中** | **建议至少 `·`**：多子目标（`apply`/`cases`）之后没有聚焦语法，学习者要数「第几个目标」 |

### 4.3 今天就能走的「机械改写」路径（重要）

`exact` 吃任意项，所以**整门课今天就能逐行机械改写**，且判卷语义不变：

| 旧（项模式） | 机械改写（今天可判卷） | Lean 4 风格（需要新 tactic） |
|---|---|---|
| `:= fun (x : α) => fun (hx : …) => <body>` | `:= by intro x; intro hx; exact <body>` | 同左（今天就是 Lean 风格） |
| `:= And.intro A B ha hb` | `:= by apply And.intro; exact ha; exact hb`（E20 ✓） | `by constructor <;> assumption` |
| `:= Iff.intro A B mp mpr` | `:= by apply Iff.intro; …`（A1 ✓） | `by constructor` |
| `:= Exists.intro A p w hw` | `:= by apply Exists.intro; exact w; exact hw`（C3 ✓） | `by use w` |
| `:= Set.ext α A B (fun x => …)` | `:= by exact Set.ext α A B (fun x => …)`（C2 ✓） | `by ext x` |
| `:= Exists.elim A p Q h f` | `:= by exact Exists.elim A p Q h f`（C7 ✓） | `by obtain ⟨w, hw⟩ := h` |
| `:= match x with \| … => <项>` | `:= by match x with \| … => <项>`（M4 ✓） | 同左 |
| `:= let g := …; body` | `:= by exact let g := …; body`（F4 ✓） | `by have g := …` |

> 注意：**`apply Exists.elim` / `apply Or.elim` 不可用**（C4/A3/A4：会造出 `Prop`、
> `A -> Prop` 之类垃圾子目标，或直接报「目标不匹配」）；消去子只能用 `exact` 整项写出
> （C7/C8 ✓）。这是「机械改写能过判卷、但读起来还是项模式」的典型。

---

## 5. Q5 —— 架构约束（目标树 / context / 改动量 / 做不动的边界）

### 5.1 今天的目标树

```rust
// crates/front/src/by.rs:44-62
enum NodeKind { Hole, Closed(Expr), Apply { f: Expr, args: Vec<ApplyArg> } }
enum ApplyArg { TypeParam(Expr), SubGoal(usize) }
struct GoalNode { ty: Expr, intros: Vec<Binder>, parent: Option<usize>, kind: NodeKind }
```

- `nodes: Vec<GoalNode>` + `worklist: Vec<usize>`（末尾 = 当前目标，`by.rs:149-160`）。
- **context = 沿父链把所有 `intros` 拼起来**（`context_binders`，`by.rs:462-474`）；
  `apply` 造的子目标 `intros: Vec::new()`、`parent: Some(cur)`（`by.rs:389-394`）——
  子目标继承的是父链的 intros。
- 判定规格 `spec_of`（`by.rs:447-459`）= 当前目标类型文本 + 父链 binders ⇒
  `judge_terms` 合成 `example : forall (b1:T1)…, <goal> := <term>`
  （`judge.rs:969-999` 的 `fold_declared`），**永远交完整内核**。
- 组装 `assemble`（`by.rs:414-444`）：`Hole`→`Expr::Hole`、`Closed`→项、
  `Apply`→把子目标解当实参依次应用，最后把 `intros` 包成 lambda 链。

### 5.2 对新 tactic 意味着什么

| 需要的动作 | 今天的树/判定能不能表达 | 改动量 |
|---|---|---|
| **`cases`（改 context + 改目标）** | ❌ 两样都不能。目标 `ty` 是**源 AST**，引擎只会 `peel_pi`（语法）+ 把文本交内核判（`judge_terms`/`judge_infer`）——**没有任何「由旧目标算新目标」的机制**。可行的实现路径有两条：(a) 用 `judge_infer` 取归纳递归子 `Or.rec`/`Exists.rec` 的类型文本（内核 pp，实测 `#check` 可得），据此造 `motive := fun (_ : T) => <当前目标文本>` 再复用 `apply_tactic` 的剥 Pi/造子目标逻辑；(b) 让前端 `derive_recursor`/归纳表（`ElabCtx.inductives`，walk.rs:325）参与 | **高**：估计 300-500 行（by.rs 内）+ 新测试；子目标要「预置 intros」（分支 binder 名从递归子 minor premise 的 Pi binder 名解析）——树本身支持（`intros` 是每节点的 Vec），但今天 `apply` 建子节点时写死空 Vec |
| **`have`（加局部定义）** | ⚠️ 半能。加**假设**只需 `nodes[cur].intros.push(binder)`（`intro` 就是这么干的，by.rs:175-182）；但「带**值**的局部定义」需要：`GoalBinderSpec` 加 `body`（`judge.rs:43`）、`fold_declared`/`wrap_binders` 会造 `let`（今天只造 Forall/Lambda，`judge.rs:969-1022`）、`assemble` 会造 `Expr::Let`（今天只造 lambda，by.rs:435-442） | **中**：150-250 行，跨 by.rs + judge.rs（都是 front，内核不动） |
| **`rw`（改写目标）** | ❌ 完全不能。没有 occurrence 抽象、没有目标替换、没有「新目标 defeq 旧目标」的判定入口。`spine.rs` 只有 `mentions`/`substitute`/`unify_spine`（`spine.rs:86/176/134`），**没有把子项抽象成 lambda 的工具** | **很高**：300-600 行且最容易出微妙错误；必须每步用 `judge_terms` 终审（纪律可行，但要设计「改写候选 → 交内核判」的回路） |
| **`constructor`** | ✅ 能（本质是 `apply <ctor>`，今天实测 W6 可用），缺的只是「按目标头自动选构造子」需要的归纳表 | **中**：100-200 行 + 把 `inductives` 传进 `run_by`（改签名，调用点 walk.rs:312-317） |
| **`exfalso`** | ✅ 能（换目标 + 组装时套 `False.elim`） | **低**：~30 行 |
| **`use`** | ✅ 能（`judge_infer` 出构造子类型后复用 `apply`） | **低-中**：~100 行 |

### 5.3 已知「做不动 / 有理由不做」的边界（读码结论）

1. **没有目标改写 = 没有 `rw`/`simp`/`change`/`cases` 的现成地基**（§5.2）。这是
   引擎最大的结构缺口。
2. **context 只有 intro binder，没有「定义等式」**：`GoalBinderSpec { name, ty }`
   （`judge.rs:43-48`）没有 body；`fold_declared` 的望远镜是 `Forall`，不是 `let`
   （`judge.rs:969-999`）。⇒ `have` 的**可归约性**（Lean 里 `have h := t` 的 `h`
   能被 defeq 展开）今天表达不了。
3. **合一只有位置 spine**（`spine.rs:130-157` 的注释明说「刻意只到这里」）⇒
   `apply` 不能推断隐式参数、不能做高阶匹配；`apply Exists.elim` 这类消去子永远
   不可用。
4. **`apply` 依赖「内核 pp 文本 ↔ 源 AST 文本」对齐**，而两边今天只对
   `namespace`/`open` 文件做过规范化（`canonical_goal_type`，by.rs:89-107，
   walk.rs 的 `canonical_goal`）⇒ 普通文件里 pp 的简化（丢层级、丢隐式实参、
   合并/拆开 binder 组）会直接变成 `apply` 的假失败（§1.4 两个 bug）。
5. **`;` 没有组合子语义**（`docs/design/by-tactics.md:194-195` 明列为不修项）：
   `t1 <;> t2` 不存在，`;` 只是分隔符。⇒ 多子目标必须逐个手写。
6. **`?`/`_` 洞被显式移除**（`token.rs:248`）⇒ 没有 `refine`/`⟨_,_⟩` 的地基；
   要做 `refine` 必须先把洞重新引进来（并且要能算洞的期望类型——`goals.rs`
   的 `func_spine_case` + 内核探针是现成参考，`goals.rs:1125-1219`）。
7. **解析期是「全或无」**：一处未知 tactic ⇒ 整份文件 `decl_checked=0`（实测 PE1）。
   ⇒ 改写期任何半成品都会让整个 unit 判红，课程门禁 G1–G6 一起红。

---

## 6. Q6 —— 实测确认：本语言里会 parse 失败 / 行为不同的 Lean 4 常见写法

实测方法：统一头部 + 一行 tactic，`query check` 取 `code`/`message`（原始输出见 §10）。

| Lean 4 写法 | 本语言 | 错误码 / 现象 |
|---|---|---|
| `intro a b`（一次两个名字） | ❌ | `unexpected-token`，位置在 `b` |
| `intro`（无名） | ❌ | 吃掉下一个 token（哪怕它是 `exact`） |
| `intro ⟨a, b⟩` / `intro (a, b)`（模式） | ❌ | `unexpected-token`（`Comma`） |
| `constructor` / `left` / `right` | ❌ | `unexpected-token` |
| `cases h` / `cases h with` / `rcases` / `obtain` / `rintro` | ❌ | `unexpected-token` |
| `use w` | ❌ | `unexpected-token` |
| `refine …` / `exact ?_` | ❌ | `??? 已移除：未完成的证明请写 sorry（与官方 Lean 一致）`（`token.rs:248`） |
| `exact ⟨a, b⟩`（匿名构造子） | ❌ | `unexpected-token`（`Comma`）——课程 0 处使用 |
| `have` / `show` / `suffices` | ❌ | `unexpected-token` |
| `rw [h]` / `rw [h] at h` | ❌ | 词法期就失败（`[` 不是合法 token） |
| `simp` / `simp at h` / `unfold` / `change` | ❌ | `unexpected-token` |
| `exfalso` / `contradiction` / `by_contra` / `push_neg` | ❌ | `unexpected-token` |
| `specialize h x` | ❌ | `unexpected-token` |
| `apply h at h2` | ❌ | **能 parse 但语义不同**：`apply h at` 被读成应用 `h at` ⇒ `unknown identifier at` |
| `subst` / `injection` / `congr` / `funext` / `ext` / `induction` | ❌ | `unexpected-token` |
| `trivial` / `tauto` / `calc` | ❌ | `unexpected-token` |
| `·`（focus dot） | ❌ | `unexpected-token`（`Ident("·")`） |
| `intro x <;> exact x` | ❌ | `unexpected-token`（`<`） |
| `all_goals` / `try` / `repeat` / `first \| …` / `done` | ❌ | `unexpected-token` |
| `h.1` / `h.2`（点投影） | ❌ | 能 parse，但 `exact h.2` ⇒ `unknown identifier h.2`（课程因此全写 `And.left/right`） |
| `exact by …`（嵌套 `by`） | ❌ | `unknown identifier by` |
| 同一行两个 tactic 不写 `;` | ❌ | `unexpected-token` |
| `fun (x) => by …` / `example := by` / `def := by` | ✅ | 可用（E16/F2/F3/D1） |

---

## 7. Q7 —— 编辑器 / goal 面板（改写后的学习体验）

**数据流**（每步状态在**编译期**产生，请求期零重算）：

1. 引擎每执行一个 tactic，记录**该步执行后**的全部未闭合目标（当前目标在首位）+
   沿父链的上下文：`by.rs:289-303`（`ByStep { span, goals: Vec<ByGoal> }`，
   `by.rs:21-40`）；
2. 报告层转成 wire 形状 `ByStepState { span, goals: Vec<ByGoalState{ty,binders}> }`
   （`compile/report.rs:70-74`、`compile/check/mod.rs:345-367`），挂到 `DeclState.by_steps`
   （`report.rs:123`；`walk.rs:335/418/460/523/621/663`）；
3. 进 I8 session 快照，**注释级编辑时 span 会平移**（`session.rs:348`、测试
   `session.rs:906`）；
4. 真相层选择器 `select_state_at`（`query/state.rs:34-93`）实现 Lean `goalsAt?`
   语义：光标在某 tactic 的 `[start,end)` 内 ⇒ **进入**它的状态（`i-1` 的执行后，
   `i==0` 时是根状态）；否则取「终点 ≤ 光标的最后一步」的执行后；都没有 ⇒ 根状态
   （根状态 = 内核渲染的完整声明类型 + 空 binders）；
5. 消费方：LSP `soko/stateAt`（`lsp/lib.rs:1605` 注册、`lsp/lib.rs:621-676` 的
   hover 版）→ VS Code 练习树「当前光标处」+ Infoview（`editor/vscode/extension.js:287`
   发请求、`:456` 渲染组、`:1711` Infoview）。

**实测（E12 逐光标走查）**：

```
L=1 C=3  → goal "forall (a : Prop), a -> a"   binders []        step -1  total 3
L=2 C=3  → goal "forall (a : Prop), a -> a"   binders []        step -1  total 3   （进入 intro a）
L=2 C=10 → goal "a -> a"                      binders [a]       step  0  total 3
L=3 C=10 → goal "a"                           binders [a, h]    step  1  total 3
L=4 C=10 → goal null                          binders []        step  2  total 3   （已闭合）
```

**多子目标**（`apply And.intro` 之后，M1）：

```
L=3 → step 0, total 3, goals ["a","b"], binders [a,b,h1,h2]     ← 两个目标都在，当前目标在首位
L=4 → step 1, total 3, goals ["b"],     binders [a,b,h1,h2]
```

⇒ **结论**：`by` 证明的中间 goal **今天就能在编辑器里看到**，且是 Lean Infoview
风格（进入某条 tactic 的状态 + 假设累积 + 多目标）。这对「改写后学习体验」是**利好**：
项模式今天只能看到整个声明的一个 goal（`step:-1`），改写后每一步都有状态。
已知缺口：`apply` 的多个子目标**共享同一个源码位置**（inlay 测试
`lsp/src/inlay.rs:365` 断言 `hints[0].position == hints[1].position`），没有 `·`
聚焦语法，学生无法「点第 2 个子目标」。

另注：REPL 里另有一套**独立**的交互证明循环 `#prove`（`proof.rs` 的 `ProofState`，
支持 `intro` / `exact` / `apply` / `assumption` / `undo` / `lambda` / `done`，
`crates/cli/src/help.rs:83`），它**不是** `by` 引擎（无 `rfl`/`match`/`sorry`，
也没有 per-step 协议）——课程改写与它无关，但可以作为「逐步教学」的既有资产。

---

## 8. 测试钉住的行为（改写前必须知道的红线）

**已钉死**（改动会红）：

- 解析：换行分隔、换行边界不被应用吃掉、多行应用是一个 tactic、`;` 与换行混用、
  by 块不吃下一个命令、`match` 降级成 `Exact{Match}` —— `parser.rs:3701/3768/3777/3792/3807/3816`。
- 引擎：`apply` 子目标**顺序**（当前目标在首位）、per-step 状态与 span 逐字、
  闭合步 `goals` 为空、空 `by`/半成品/`by sorry` ⇒ Open、五种失败都是
  `elab-tactic-failed` —— `compile/tests.rs:3692-3966`、`cli.rs:109-214`。
- 多文件：`by` 通过 judge 前缀读**被 import 的类型**（`project/tests.rs:631`）。
- 命名空间：`namespace` 里短名可用、`open Foo in` 可用（`compile/tests.rs:6844/7095`）——
  这两条正是 G-05 `canonical_goal` 的护栏。
- goal 面板：`query/state.rs` 的选择语义、CLI≡LSP 一致性（`query.rs:1075/1106`）、
  LSP 的多目标与 hover（`lsp/src/tests/state.rs:4-249`）。
- **卷 I 课程也被测试钉住**（改写前必读）：
  - `cli/tests/query.rs:558-574` 把**真课程单元** `courses/set-theory/units/unit05-pairs-products.sokonanoda`
    的计数钉成 `decl_checked == 5` / `exercise_open == 7` / `failed == 0`（且 exit 0）
    ⇒ 改写该文件**必须同轮更新这条断言**（或保持计数不变）。
  - `cli/tests/notation.rs:280-383` 把 `courses/set-theory/units/unit02-subsets-empty.sokonanoda`
    当夹具：它按**文本**找「最后一个 `import ` 行」插入记法声明、并替换
    `subset_trans` / `empty_subset` 两处**签名**的点名写法，然后要求「原文件 exit 0、
    记法变体计数完全相同、`checked >= 1`、`open >= 8`、`diagnostics == 0`」
    ⇒ 改写若动了这两道题的**签名拼写/名字**或让文件判红，这条测试会红。
  - `cli/tests/course_manifest.rs:278-287` 读 `courses/set-theory/course.json`
    （v2 清单形状，G6）；`courses/set-theory/tools/check.py` 的 G1–G6
    **只判形状、不锁计数**（`check.py:2/21`），所以单元内部改写法本身不破坏 G6，
    但 G2/G3（每个目标 `grade` exit 0、解答 0 open 且 checked>0）必须保持绿。
  - 入门课 `course/`（11 单元，另一门课）的 golden 计数 `cli/tests/course.rs:101`
    钉的是 `course/unit4-by-tactics.sokonanoda`（`(13,5,0)`），**与卷 I 无关**。

**没有测试钉住**（改动无护栏，新增 tactic 必须自带三件套）：

- `by.rs` **本身零单测**（543 行，无 `mod tests`；`crates/front/tests/` 里没有
  by/tactic 测试）——引擎内部重构对测试不可见。
- `intro a b` 报错、缩进不敏感、嵌套 `by` 失败、`apply` 失败消息文本、未知 tactic 的
  **错误码**、`by sorry` vs `:= sorry` 在**合法签名**下的事件等价、`rfl` 的
  「必须显式 `Eq.{u}`」边界——**都没有测试**。

---

## 9. 迁移建议（按风险/收益排序）

1. **先修 `apply` 的两个 bug**（低成本、高收益，且不改语义）：
   (a) `render_roundtrip`（`judge.rs:557-575`）对任意位置的 `Forall` 都补 `forall`
   关键字/括号；(b) 让 `apply` 的目标也走一次内核 pp 规范化（把
   `canonical_goal_type` 从「仅 namespace/open 文件」放开到所有 `apply`，或让
   `same_head` 容忍 pp 丢层级/丢隐式实参）。**没有这一步，课程里 `Set.ext`/记法目标
   的 `apply` 会持续假失败。**
2. **再补最小 tactic 集**（按 §4.2 的 1-4）：`cases`/`obtain` → `constructor`/`use` →
   `have` → `exfalso` → 窄版 `rw`。每加一个都走硬规则 3（语法 + 测试 + 课程）。
3. **改写顺序**：先在 `lib/`（906 行、39 定理）试点——它已全证完，改写后判卷必须
   仍 `0 diagnostic / 0 exercise.open`；再改 `units/solutions/`（答案钥匙）；最后改
   `units/` 画布（99 处 `sorry` 改成 `by` 块里的 `sorry` 或空 `by`）。
4. **机械改写兜底**：任何一时补不上 tactic 的形状，先用 `by exact <原项>` 过判卷
   （§4.3 全部实测），保证「改写期判卷恒绿」，再逐步替换成真 tactic。
5. **风险清单**：
   - **parse 错误全文件连坐**（PE1）⇒ 改写必须逐文件、逐声明提交，禁止半成品进 main；
   - 课程测试夹具与断言（§8 最后一段）：`query.rs:558-574` 钉 unit05 计数、
     `notation.rs:355-383` 钉 unit02 的签名文本与 `open >= 8`、`check.py` 的
     `--selftest` 通道——改写前先跑一遍这三条，改写后同轮对齐；
   - 新增 tactic 的判定**永远**走 kernel 合成声明（REQUIREMENTS §2.8）；`rw` 这类
     「改目标」的 tactic 最容易滑向自证，必须设计成「候选新目标 → 内核 defeq 判定」；
   - 内核冻结：以上全部改动都在 front（by.rs/parser.rs/judge.rs/spine.rs/ast.rs）；
   - 编辑器侧：新 tactic 的 `span` 必须进 `ByStep`，否则 goal 面板会漏步
     （`by_steps` 是面板的唯一数据源）。

---

## 10. 附录 A —— 实测原始记录（源码 + 输出）

> 命令统一：`S=scripts/soko`；判卷 `$S grade <绝对路径>`；查询 `$S query check --file <绝对路径>`；
> 状态 `$S query state --file <绝对路径> --line N --col M`。临时文件 `/tmp/tact/`、`/tmp/tact/course/`。
> **注意（二进制新鲜度）**：仓库 `target/debug/sokonanoda` 的**缓存标记是 0.55.0**
> （`version --json` 报 `match:false`），但它是 **0.61.0 源码的仓库构建**，证据：
> (a) `git log -1 -- crates/front/src/by.rs` = 0.60.0（14:15），二进制 mtime 18:47；
> (b) `git show f3902b3 -- crates/front/src/parser.rs | grep tactic` **无 tactic 段改动**
> （0.61.0 只动了记法/namespace）；
> (c) 0.61.0 独有语法在它上面**可解析**：`binder_notation "∃" => Foo` → exit 0、
> `scoped infix:50 " @ " => Nat.add` → 报的是 0.61.0 的新诊断
> `notation-shape`「`scoped` 要写在 `namespace` 里」（0.55 会是 `unexpected-token`）、
> `{aa, bb}` 集合字面量解析通过（只在 elab 报 `unknown identifier Set`）。
> 本文所有实测均出自它 + `scripts/soko` 启动器（`source: repo-build`）。
> （`cargo build` 复验不可用：本机 Xcode 许可未同意，链接期 `cc` 失败——
> 与仓库无关，见 `[exit code]` 记录。）

### A.1 白名单 / 分隔 / 空块 / sorry（E 系列）

```lean
# E01 intro 两个名字
theorem t : (a b : Prop) -> a -> b -> a := by
  intro a b
  exact a
→ {"code":"unexpected-token","message":"expected a .sokonanoda command, found Token { kind: Ident(\"b\"), … line 2, column 11 }"}
   exit=1

# E02 intro 无名（吃掉下一行的 exact）
theorem t : (a : Prop) -> a -> a := by
  intro
  exact a
→ unexpected-token at Ident("a") line 3 column 9（`exact` 被当成 binder 名）

# E03 空 by 块 / E04 半成品 / E05 by sorry / E06 := sorry
→ 四者都是 {"human":"exercise open (fill the sorry)","name":"t","type":"exercise.open"} exit=0

# E07/E14 sorry 之后继续写
theorem t : (a : Prop) -> a -> a := by
  sorry
  intro a
  intro h
  exact h
→ {"human":"checked declaration t","name":"t","type":"decl.checked"} exit=0
  （`query check` warnings 为空：`by` 里的多余 `sorry` 不触发 redundant-sorry）

# E08 一行两个 tactic 不写 `;`
theorem t : (a : Prop) -> a -> a := by intro a exact a
→ unexpected-token at Ident("exact") line 1 column 48

# E11 一行用 `;`
theorem t : (a : Prop) -> a -> a := by intro a; intro h; exact h
→ checked declaration t

# E13 缩进不敏感
theorem t : (a : Prop) -> a -> a := by
intro a
        intro h
exact h
→ checked declaration t

# E15 嵌套 by
  exact by intro h; exact h
→ {"code":"elab-tactic-failed","message":"`exact` 判定失败：unknown identifier `by`"}

# E16 lambda 体尾的 by
def t : (a : Prop) -> a -> a := fun (a : Prop) => by intro h; exact h
→ checked declaration t

# E29 目标全闭合后再写
→ {"code":"elab-tactic-failed","message":"by 块里没有待解目标"}
```

### A.2 `apply` / `rfl` / `assumption`（E20-E41、A1-A7、C1-C4、V1-V4）

```lean
# E20 apply And.intro + 两个 exact（子目标顺序 a 后 b）
theorem t (a b : Prop) (h1 : a) (h2 : b) : And a b := by
  apply And.intro
  exact h1
  exact h2
→ checked declaration t
# E21 反序（先 exact h2）→ elab-tactic-failed（期望 a，实际 b）——顺序被钉死

# E22 apply Or.inl（σ 从目标补 B）
theorem t (a b : Prop) (h : a) : Or a b := by
  apply Or.inl
  exact h
→ checked declaration t

# E24 头不匹配
  apply Or.inl   （目标 And a b）
→ "`apply` 的目标不匹配：`Or.inl` 的结果是 `Or A B`，无法对齐当前目标 `And a b`"

# E27/E31/E36/E37/E39/E41 rfl 的边界
theorem t (a : Prop) (h : a) : Eq a a := by rfl          → ✗ "`rfl` 需要一个 `Eq α x y` 形状的目标"
theorem t (a : Prop) : Eq.{1} Prop a a := by rfl         → ✓ checked
theorem t : Eq.{1} Nat (1 + 1) 2 := by rfl               → ✓ checked
def My : Type := Nat
theorem t (A : My) : Eq.{0} My A A := by rfl             → ✗ "两边不相等（期望 `Sort(0)`，实际 `Sort(1)`）"

# V2 apply 的 pp 回读 bug（最小复现）
theorem t (A B : Prop) (C : Nat) (h : A) : Or A B := by
  apply Or.inl
  exact h
→ {"code":"elab-tactic-failed","message":"无法解析 `Or.inl` 的类型：expected `->` after binder group, found LParen"}
  （pp 文本：forall (A : Prop), Prop -> Nat -> A -> (forall (A B : Prop), A -> Or A B)）
# V1/V3/V4 同类上下文变体 → checked（说明触发条件与 pp 的分组有关，非常脆）

# A2 Set.ext 的头形状 bug（课程库）
theorem t (α : Type) (A B : Set α) : Eq.{1} (Set α) A B := by
  apply Set.ext
→ "`apply` 的目标不匹配：`Set.ext` 的结果是 `Eq A B`，无法对齐当前目标 `@Eq.{1} (Set α) A B`"
# C1 带 h1/h2 的完整课程形状 → "无法解析 `Set.ext` 的类型：expected `->` after binder group, found LParen"
# C2 exact 整项 → ✓ checked（逃生舱）
# A1 apply Iff.intro → ✓；C3 apply Exists.intro → ✓；W6 apply 用户构造子 → ✓
# A3 apply Or.elim（目标变量恰叫 c）→ exercise.open（造出 5 个垃圾子目标：
#    query goals 的 sub_goals = [Prop, Prop, None, None, None]，span 全是整条 tactic）
# A4/A6 apply Or.elim / False.elim → 目标不匹配；C4 apply Exists.elim + exact h → 报一堆怪类型
# C7/C8 消去子的机械改写（exact 整项）→ ✓ checked：
#    exact Exists.elim A p Q h f     （lib/Exists）
#    exact Or.elim a b c f g h       （prelude）
```

### A.3 `match`（M2-M5）

```lean
# M4（臂体是项）→ checked declaration t
theorem t (x : Two) : Eq.{1} Two (f1 x) aa := by
  match x with
  | aa => Eq.refl.{1} Two aa
  | bb => Eq.refl.{1} Two aa
# M3（臂体写成 tactic 串）→ parse 错误（第二个 `|`）
# M2（臂体写 `exact …`）→ "`exact` 判定失败：unknown identifier `exact`"
# M5（`by exact match …`）→ checked
```

### A.4 parse 错误的爆炸半径（PE1）

```lean
theorem good  : forall (a : Prop), a -> a := by intro a; intro h; exact h
theorem bad   : forall (a : Prop), a -> a := by
  constructor          -- ← 未知 tactic
theorem after : forall (a : Prop), a -> a := by intro a; intro h; exact h
```
```
$ scripts/soko grade /tmp/tact/pe1.sokonanoda --json
{"code":"unexpected-token","message":"expected a .sokonanoda command, found Token { kind: Ident(\"constructor\"), … line 3, column 3 }"}
exit=1
$ scripts/soko query check --file /tmp/tact/pe1.sokonanoda --compact
… "counts":{"decl_checked":0,…}   ← 连前面写对的 `good` 也一起没了
```

### A.5 Lean 4 常见写法扫描（probe，统一头部 + 一行 tactic）

（完整 52 例见 `/tmp/tact/probe2.py` 与其 `probe2/` 目录；下表是归类结果）

```
constructor/left/right/cases/rcases/obtain/rintro/use/have/show/suffices/
simp/unfold/change/exfalso/contradiction/by_contra/push_neg/specialize/
subst/injection/congr/funext/ext/induction/trivial/tauto/calc/·/all_goals/
try/repeat/first/done  →  全部 unexpected-token（stage=parse）
rw [h] / rw [h] at h     →  词法错误：expected a valid .sokonanoda token, found [
refine … / exact ?_      →  ??? 已移除：未完成的证明请写 sorry（与官方 Lean 一致）
apply h at h2            →  elab-tactic-failed: unknown identifier `at`（parse 成应用）
exact _                  →  elab-tactic-failed: unknown identifier `_`
intro _                  →  能 parse（`_` 当名字），随后按目标报「不是函数」
intro ⟨x,y⟩              →  unexpected-token（Comma）
exact ⟨b, a, h.2, h.1⟩   →  unexpected-token（Comma）
intro+exact 同一行 `;`   →  能 parse（这一行本身是合法的，报错来自测试目标形状）
```

### A.6 编辑器 goal 面板（§7 的两段实测输出已贴在正文）

---

## 11. 附录 B —— 复现清单

```bash
cd /Users/penglingwei/Documents/lean/sokonanoda/sokonanoda-lang
S=scripts/soko
mkdir -p /tmp/tact && cd /tmp/tact
# 逐条实验：把 §10 的源码片段写进 e*.sokonanoda / v*.sokonanoda / m*.sokonanoda
$S grade /tmp/tact/e01_intro_two.sokonanoda
$S query check --file /tmp/tact/e01_intro_two.sokonanoda --compact
$S grade /tmp/tact/e05_by_sorry.sokonanoda --json > a.json
$S grade /tmp/tact/e06_colon_sorry.sokonanoda --json > b.json && diff a.json b.json   # 空
$S query state --file /tmp/tact/e12_ok_newlines.sokonanoda --line 3 --col 10
# 课程形状（需要课程库）：把 courses/set-theory/lib/*.sokonanoda 复制到 /tmp/tact/course/lib/
$S grade /tmp/tact/course/c1.sokonanoda     # apply Set.ext 失败
$S grade /tmp/tact/course/c2.sokonanoda     # exact Set.ext … 通过
# Lean 4 写法扫描
python3 /tmp/tact/probe2.py
```

---

## 12. 附：一句话回答每个问题

1. **白名单**：`intro`（单名）/`exact`（任意项）/`apply`（位置 spine 合一，两个 bug）/
   `assumption`/`rfl`（只认 `Eq.{u} α x y`）/`match`（臂体是项）/`sorry`（no-op）；
   判定全部走 kernel。
2. **没写满 = 合法 Open**（尾部自动 `sorry`）；`by sorry` ≡ `:= sorry`（事件流逐字节
   相同）；`sorry` 之后可以继续写且**无 warning**；空 `by` 可以。
3. **`;` 或换行分隔、缩进不敏感、同一行多个必须 `;`、不能嵌套 `by`**（lambda 体尾可以）。
4. **必须补**：`cases`/`obtain`（≈120 处消去）、`constructor`/`use`（≈130 处构造）、
   `have`（28 处 `let` + 长证明）、`exfalso`（22 处）、窄版 `rw`（55 处 `Eq.subst`）；
   **先修 `apply` 的 pp/头形状两个 bug**（否则 `Set.ext` 34 处、记法目标都假失败）。
5. **架构约束**：目标树只有 `Hole/Closed/Apply` + 父链 intros；引擎没有目标改写能力，
   `cases`/`rw` 必须新建「motive 构造 + 目标替换 + 内核终审」的地基（`have` 中等，
   `cases` 高，`rw` 很高）。
6. **`intro a b` 确认报错**；`constructor/cases/rw/have/show/use/…` 全部 parse 失败
   （§6 全表）。
7. **中间 goal 今天就能在编辑器里看到**（`by_steps` + `soko/stateAt` + VS Code
   练习树/Infoview），多子目标也在；缺 `·` 聚焦与子目标独立位置。

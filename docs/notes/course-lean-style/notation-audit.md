# 记法审计 —— 把卷 I 改写成 Lean 4 数学符号，今天能走到哪一步

> 类型：**只读调研**（不改任何现有文件）。审计对象：`sokonanoda` **0.61.0**
> （`Cargo.toml` 的 `version`；`scripts/soko version --json` 的
> `version_source = "Cargo.toml"`，`cli.source = "repo-build"`，
> `sokonanoda --version` 实测 `sokonanoda 0.61.0`）。
> 日期：2026-09-19。判卷一律 `scripts/soko grade <绝对路径>`（台账 G-12）。
>
> 证据纪律：**实测优先于读代码**。每条结论给「文件:行号」+（能实测的）实测。
> 只读代码、没能实测的，显式标 **【代码读】**。

---

## 0. 结论速览

| # | 问题 | 结论 | 今天能不能用 |
|---|---|---|---|
| 1 | 符号 token 码点范围 | 两套：**码点类**（`U+2200–22FF`/`U+2A00–2AFF`/`\`）+ **声明驱动词法**（任意声明过的符号文本，含标识符字符） | `≠≤∘∃∧∨` 天生是 `Sym`；`→↔¬⟨⟩⁻¹×` 是 `Ident`，**声明后**变成 `Sym`；`∀` 永远是关键字 |
| 2 | `infix:N " ∧ " => And` | `And`/`Or` 是 **prelude 归纳**，`Not`/`Iff` 是 **prelude def** | ✅ **今天直接可用**（实测 `∧∨↔¬` 四条 + 四条 `def` 全 exit 0） |
| 3 | `→` 做成记法 | `→` **可以**声明成符号，但**没有目标名可指**：函数空间是 `Expr::Arrow`（语法形式），不是常量 | ❌ **今天不可用**。`=> Arrow` 报 `elab-notation-unknown-target`；只能指一个 `def Imp (A B : Prop) : Prop := A -> B`（可用但不是真 Pi，`(x:α) → β` 直接 parse 错） |
| 4 | `∀ x, p` / `∃ x, p` 确切语法 | `∀` 是关键字（原生 Pi）；`∃` 必须 `binder_notation "∃" => Exists` 且 `Exists` 在作用域里 | `∀ (x:α), p` ✅ / `∀ x : α, p` ✅ / `∀ x, p` ❌ / `∃ (x:α), p` ✅ / `∃ x : α, p` ✅ / `∃ x, p` ❌ / `∃ x y, p` ❌ / `∀ x ∈ s, p` ✅ / `∃ x ∈ s, p` ✅ |
| 5 | `prefix:40 " ¬ " => Not` / `≠` | `¬` ✅ 今天可用。`≠` 是合法符号 token，但**全仓没有 `Ne` 名字** | `¬` ✅；`≠` ❌（要加一条 `def Ne`，实测 3 行即通） |
| 6 | 作用域与传播 | 本文件「声明之后」；import 来的「从文件头就可用」；`scoped`/`open scoped` 可用；**重声明 import 来的符号 = 硬错**；两个 import 模块同符号 = **静默后者覆盖** | ✅ 基本够用，但「一个符号一个闭包只能声明一次」是硬约束 |
| 7 | 优先级/结合性 | 梯子在 `parse_arrow` 与 `parse_app` 之间；`infix`=左 `p`/右 `p+1`、`infixl` 同、`infixr`=`p+1`/`p`；`+` = **65**；`->` 比任何记法都松 | ✅ 推荐 `→25 / ↔20 / ∨30 / ∧35 / ¬40 / ≠50 / ∈50 / ⊆50 / ∪65`，**全部实测钉住** |
| 8 | print-back | **两套行为**：`query goals` 的 `goal`/`binders[].ty` 走 front 的 `render_expr` ⇒ **显示记法**；`query state` 的 `goal`、声明的 `ty`、`#check`、hover 走**冻结内核 pp** ⇒ 点名形式 | ⚠️ 半 print-back（比设计文档 §13.1 说的更乐观） |
| 9 | 同符号只能声明一次？ | 同文件同形状 = **重载**；同文件不同形状 = 错；**重声明 import 来的 = 错**；两个被 import 的模块各声明一次 = **静默覆盖**；**prelude 里写记法命令会被静默忽略** | ⚠️ 有坑 |
| 10 | 前导类型参数补全 | 只在 `telescope 层数 > 操作数个数` 时补；按「操作数类型 → 期望类型」两条路；**不支持一般合一** | ✅ 覆盖 `And/Or/Iff/Not/Ne/Set.mem/Set.subset/Set.empty/Set.image`；❌ 隐式首参、`Eq`（宇宙多态）、层数==操作数的 Type 首参 |

**一句话**：`∧ ∨ ↔ ¬ ∀ ∃` 今天就能用（`∃` 要 `binder_notation` + `Exists` 在作用域）；
**`→` 和 `≠` 与 `=` 需要语言改动**；其中 `→` 是唯一必须动 parser 的（约 2 行）。

---

## 1. 证据来源

| 来源 | 行号锚点 |
|---|---|
| `docs/design/notation-subset.md`（693 行，权威设计 + 三刀 as-built） | N1–N7 §2；词法 §3；落点 §4；已知差异 §6；第二刀 §10–§12；第三刀 §14 |
| `crates/front/src/token.rs`（949 行） | 码点类 454–501；`Sym`/`Str` 145–393；预扫描 583–643 |
| `crates/front/src/parser.rs`（4702 行） | 四条命令 701–839；优先级 842–931；登记 943–994；梯子 1718–2022；binder 2471–2602 |
| `crates/front/src/compile/prelude.rs`（696 行） | `PRELUDE_NAMES` 84–139；`PRELUDE_L1_SRC` 195–231；安装 371–408 |
| `crates/front/src/compile/elab.rs`（4130 行） | `elab_notation` 937–1026；binder 1066–1125；补参 1411–1513；期望类型 1519–1551；`unify_extract` 1602–1645；重载 1254–1384 |
| `courses/set-theory/lib/Set.sokonanoda` | 记法 155–159（五个符号） |
| `courses/set-theory/lib/Exists.sokonanoda` | `inductive Exists` 86–88 |
| `courses/set-theory/units/notation-cheatsheet.sokonanoda` | 本页四条 86–89；边界 52–59；对照 61–71 |
| `crates/cli/tests/notation.rs`（783 行） | 19 个测试，见 §11 索引 |
| 旁证（不在指定清单里，但结论必需） | `crates/front/src/by.rs:317-401`（`apply`）、`crates/front/src/judge.rs:358-415`（`judge_type_of`）、`crates/front/src/proof.rs:323`（`render_expr` 的 Notation 臂）、`crates/front/src/compile/goals.rs:1073`（goal 渲染） |

---

## 2. Q1 —— 符号 token 的码点范围

### 2.1 两套机制（不是一套）

**(a) 码点类**（`token.rs:474-476`）：

```rust
pub(crate) fn is_math_symbol(c: char) -> bool {
    matches!(c as u32, 0x2200..=0x22FF | 0x2A00..=0x2AFF) || c == '\\'
}
```

`is_ident_start` 把它排除出去（`token.rs:454-456`）：

```rust
c.is_ascii_alphabetic() || c == '_' || ((c as u32) >= 0x80 && !is_math_symbol(c))
```

⇒ 码点类内的字符**天生**是 `Sym`（最大吞噬，`lex_symbol` `token.rs:379-393`），
码点类外的 `≥0x80` 字符是**标识符字符**。

**(b) 声明驱动词法**（第二刀，`token.rs:92-98` + `208-216` + `583-643`）：
`scan_notation_symbols` 预扫描整份源码里记法命令声明的符号文本，
`next_token` 在**常规分支之前**做「声明符号最长匹配」⇒ 命中的字符序列变成 `Sym`。
所以**标识符字符也能当符号**（`𝒫`/`ᶜ`/`''`/`⁻¹'`/`×ˢ` 就是这么救回来的）。

符号合法性（parser 与预扫描共用一份，`token.rs:497-501`）：

```rust
pub(crate) fn is_valid_notation_symbol(symbol: &str) -> bool {
    !symbol.is_empty()
        && !is_ascii_word_symbol(symbol)                       // 纯 ASCII 词（in/e/Set）不行
        && lexer_reserved_symbol_char(symbol).is_none()        // ∀ - # " 不行
}
```

### 2.2 逐个符号（**实测**，`#check <符号>` 单文件判卷）

| 符号 | 码点 | 在码点类里？ | 裸用时词法给什么 | 实测诊断 |
|---|---|---|---|---|
| `∧` | U+2227 | ✅ | `Sym("∧")` | `notation-unknown-symbol`（parse） |
| `∨` | U+2228 | ✅ | `Sym("∨")` | 同上 |
| `≠` | U+2260 | ✅ | `Sym("≠")` | 同上 |
| `≤` | U+2264 | ✅ | `Sym("≤")` | 同上 |
| `∘` | U+2218 | ✅ | `Sym("∘")` | 同上 |
| `∃` | U+2203 | ✅ | `Sym("∃")` | 同上 |
| `∈` | U+2208 | ✅ | `Sym("∈")` | 同上 |
| `⊢` | U+22A2 | ✅ | `Sym("⊢")` | 同上（goal 面板的 turnstile 是渲染文本，不是源码） |
| `⨅` | U+2A05 | ✅ | `Sym("⨅")` | 同上 |
| `⨯` | U+2A2F | ✅ | `Sym("⨯")` | 同上 |
| `\` | U+005C | ✅（专门列出） | `Sym("\\")` | 同上 |
| `→` | U+2192 | ❌ | `Ident("→")` | **`elab-unknown-identifier`** |
| `↔` | U+2194 | ❌ | `Ident("↔")` | 同上 |
| `¬` | U+00AC | ❌ | `Ident("¬")` | 同上 |
| `⟨` | U+27E8 | ❌ | `Ident("⟨")` | 同上 |
| `⟩` | U+27E9 | ❌ | `Ident("⟩")` | 同上 |
| `⁻` | U+207B | ❌ | `Ident("⁻")` | 同上 |
| `×` | U+00D7 | ❌ | `Ident("×")` | 同上 |
| `⇒` | U+21D2 | ❌ | `Ident("⇒")` | 同上 |
| `⟹` | U+27F9 | ❌ | `Ident("⟹")` | 同上 |
| `∀` | U+2200 | ✅ 但在符号分支**之前** | `TokenKind::Forall` | `expected a binder, found Eof` |

**关键点**：`→ ↔ ¬ ⟨ ⟩` **不在码点类里，但这不等于不能用**——它们全都能
**声明成记法符号**（`is_valid_notation_symbol` 只拒纯 ASCII 词与 `∀ - # "`），
声明之后 `declared_symbol_ahead` 在 `is_ident_start` 分支之前命中 ⇒ `Sym`。
实测：`infix:35 " ∧ " => And` 与 `prefix:40 " ¬ " => Not` 里的 `¬` 就是这么工作的
（`¬` 是 U+00AC、标识符字符）。

**不在范围内的会怎样**：不是「parse 报错」，而是被读成**标识符** ⇒
`elab-unknown-identifier`（elab 阶段）。唯一的词法硬错是
`lexer_reserved_symbol_char` 那四个字符（`∀`/`-`/`#`/`"`）——声明它们会被
`notation-shape` 拒（`parser.rs:909-916`）。

### 2.3 最大咬合（实测）

- `lex_symbol` 连续吞噬码点类字符 ⇒ `#check True ≠≤ True` 报
  **`notation-unknown-symbol: 符号 '≠≤'`**（一个 token，不是两个）。⚠️ 陷阱。
- 但**声明过**的符号走最长声明匹配 ⇒ 声明 `∧` 与 `¬` 之后
  `True∧¬True`、`True∧¬True∧True`（**完全不写空格**）都 exit 0（实测）。
- `lex_ident` 在标识符**内部**也检查声明符号（`token.rs:416`）⇒ `Aᶜ` = `Ident(A)+Sym(ᶜ)`。

---

## 3. Q2 —— `infix:N " ∧ " => And` 与目标名

**今天就能写。实测**（`q2_and.sokonanoda`，exit 0）：

```
infix:35 " ∧ " => And
infix:30 " ∨ " => Or
infix:20 " ↔ " => Iff
prefix:40 " ¬ " => Not
def p : Prop := True ∧ True      -- decl.checked p
def q : Prop := True ∨ False     -- decl.checked q
def r : Prop := True ↔ True      -- decl.checked r
def s : Prop := ¬ True           -- decl.checked s
```

目标名的可用性（`prelude.rs`）：

| 名字 | 是 prelude 名字？ | 形态 | 证据 |
|---|---|---|---|
| `And` | ✅ `PRELUDE_NAMES` | **inductive**（`ctor And.intro`） | `prelude.rs:104`、`prelude.rs:201-203` |
| `Or` | ✅ | **inductive** | `prelude.rs:111`、`prelude.rs:207-210` |
| `Not` | ✅ | **def**（`:= A -> False`） | `prelude.rs:117`、`prelude.rs:212` |
| `Iff` | ✅ | **def**（`:= And (A->B) (B->A)`） | `prelude.rs:122`、`prelude.rs:216` |
| `Exists` | ❌ **不在 prelude** | **inductive，在课程库** | `courses/set-theory/lib/Exists.sokonanoda:86-88` |
| `Ne` | ❌ **全仓不存在** | —— | `grep -rn "\bNe\b" crates/front/src/` 零命中 |

⇒ 记法目标**可以是 inductive、也可以是 def**（两条路都实测过），
只要名字能 `resolve_known`。**目标名在使用点解析**（`elab.rs:951`）：
实测 `binder_notation "∃" => Exists` 在 `Exists` 不存在时**声明行零诊断**
（`b16.sokonanoda` exit 0）——与设计 §9.4 一致。

---

## 4. Q3 —— `→` 与 `->`

### 4.1 `->` 的确切实现位置

`->` 是**内建语法形式**，不是常量：

| 环节 | 位置 | 行为 |
|---|---|---|
| 词法 | `token.rs:151-160`（跳过空白/注释的循环里）与 `token.rs:281-295` | `-` 后跟 `>` ⇒ `TokenKind::Arrow` |
| 语法 | `parser.rs:1745-1754` | `lhs = parse_plus()`；看到 `Arrow` ⇒ `Expr::Arrow { domain, codomain }`，**右结合**（`rhs = self.parse_arrow()`） |
| 多名字 binder | `parser.rs:1719-1743` | `(a b c : T) -> body` ⇒ 逐名字 `Expr::Forall` 链 |
| 类型 | 无对应常量 | `Expr::Arrow` 在 elab 里直接变成内核 Pi |

### 4.2 `->` 能不能被记法覆盖/别名？

**不能。** 两条独立理由（都有代码/实测证据）：

1. **词法**：记法符号里不许出现 `-`（`lexer_reserved_symbol_char`，`token.rs:489-491`）
   ⇒ 无法声明 `"-"` / `"->"`；而 `->` 的 token 化发生在**声明符号匹配之前**
   （`next_token` 顶部的 `-` 分支 `token.rs:151`，符号匹配在 `token.rs:208`）。
2. **语义**：即使声明了 `→`，目标必须是**常量名**（`elab.rs:951` `resolve_known`），
   而函数空间没有常量名。

实测：

```
infixr:25 " → " => Arrow
def p : Prop := True → True
→ elab-notation-unknown-target: 记法 `→` 指向的目标 `Arrow` 不存在
```

### 4.3 `→` 今天能做到什么程度

| 写法 | 结果 | 证据 |
|---|---|---|
| `infixr:25 " → " => Imp`，`Imp (A B : Prop) : Prop := A -> B` | ✅ **可用**（`theorem t (f : A -> B) : A → B := f` exit 0；`def g (f : A → B) (h : A) : B := f h` exit 0） | `q3_alias2.sokonanoda` |
| 同上但 `Fun (α β : Type) : Type := α -> β` | ✅ 可用（`def g : Nat → Nat := fun n => n` exit 0） | `q3_alias_type.sokonanoda` |
| `(x : α) → Prop`（**依赖箭头**） | ❌ **parse 错**：`expected '->' after binder group, found Sym("→")`（`parse_arrow:1723` 只认 `TokenKind::Arrow`） | `q3_paren_arrow.sokonanoda` |
| `→` 同时指向 Prop 版与 Type 版（重载） | ❌ 不可行：两个候选的结果类型都是 `Sort`，`notation_result_matches` 对 `Sort` 头返回 false ⇒ `elab-notation-no-candidate` | `elab.rs:1349-1384` + `u3a/u3b` 实测（`Prop` 解析成 `Expr::Sort`，`parser.rs:2249`） |

⇒ **`→` 是唯一必须动 parser 的符号**（见 §12 M1）。

---

## 5. Q4 —— binder 记法的确切语法（**全部实测**）

夹具：`import lib`（`Exists` 是 `inductive Exists (A : Type) (p : A -> Prop) : Prop`）。

| 写法 | exit | 诊断/结论 | 文件 |
|---|---|---|---|
| `∃ (n : Nat), Eq.{1} Nat n n` | **0** | ✅ `decl.checked` | `b1` |
| `∃ n : Nat, Eq.{1} Nat n n`（**无括号**） | **0** | ✅ 同义 | `b2` |
| `∃ n, Eq.{1} Nat n n` | 1 | ❌ `elab-binder-notation-unsolved`（一段式必须带标注） | `b3` |
| `∃ (n : Nat) (m : Nat), …`（**多 binder**） | 1 | ❌ **内核裸错**（`类型不匹配：期望 Pi ( : Nat), Sort 0 …`），不是教学诊断 | `b8` |
| `∀ n, p` | 1 | ❌ `elab-untyped-binder` | `b4` |
| `∀ n : Nat, p` | **0** | ✅ | `b5` |
| `∀ (n : Nat), p` | **0** | ✅ | `b6` |
| `∀ n m, p`（多 binder 全裸） | 1 | ❌ `elab-untyped-binder`（第一个 binder 就报） | `b7` |
| `∀ x ∈ s, p`（`∈` 已声明） | **0** | ✅ 原生 `∀` 共享 `parse_binder_prefix` | `b10` |
| `∃ x ∈ s, p`（`∈` 已声明） | **0** | ✅ 展开成 `Exists α (fun x => And (x ∈ s) (p x))` | `b9` |
| `∃ (x : α) ∈ s, p` | **0** | ✅ 带标注的两段式 | `b11` |
| `∃ x y ∈ s, p`（多 binder 两段式） | 1 | ❌ `elab-notation-argument-unsolved`（设计 §13.2 已列为不做） | `b13b` |
| `∀ x ∈ s, p`，`∈` **未声明** | 1 | ✅ 专用 `notation-unknown-symbol`（parse，`parser.rs:2529-2533`） | `b15` |

**逗号是必需的吗？** 是。`parse_binder_prefix`（`parser.rs:2500-2539`）只在
`Comma` 或 guard 后 `Comma` 处返回；没有逗号会一路 `push_binders` 到
`unexpected end of file inside binders`。

**类型标注**：`∀` 由 elab 报 `elab-untyped-binder`；`binder_notation` 报
`elab-binder-notation-unsolved`（`elab.rs:1104-1116`），两段式由 guard 反解
（`elab.rs:1087-1103`，`guarded_binder_type`）。

**额外实测（改写必须知道）**：`∀`/`∃` 这类 binder 形式**不能做任何算子的操作数**——
`parse_expr` 只在表达式**开头**分派它们（`parser.rs:1525-1538`），
算子操作数走 `parse_operators → parse_app → parse_atom`：

| 写法 | exit | 诊断 |
|---|---|---|
| `True -> ∀ (x : Nat), True` | 1 | `unexpected-token: expected an expression, found Forall` |
| `True ↔ ∀ (x : Nat), True` | 1 | 同上 |
| `¬ ∀ (x : Nat), True` | 1 | 同上 |
| `True ∧ ∀ (x : Nat), True` | 1 | 同上 |
| `(∃ (x : Nat), …) ∧ True`（**加括号**） | **0** | ✅ |

⇒ 卷 I 写 `A ⊆ B ↔ ∀ x, x ∈ A → x ∈ B` 时，`∀` 那一侧**必须加括号**
（实测 `probe.sokonanoda` 的 `p2` 就是这么写的、exit 0）。

---

## 6. Q5 —— `¬` 与 `≠`

### 6.1 `¬`：**今天就能用**

```
prefix:40 " ¬ " => Not
def s : Prop := ¬ True                 -- exit 0
def p (a b : Prop) : Prop := ¬a ∧ ¬b   -- exit 0（40 > 35，前缀吸收正确的操作数）
```

`¬` 是 U+00AC、**标识符字符**，靠声明驱动词法变成 `Sym`。
`Not` 是 prelude 的 `def`（`prelude.rs:117`/`212`），是合法记法目标。

### 6.2 `≠`：符号合法，**没有目标**

- `≠`（U+2260）在码点类里 ⇒ 裸用是 `Sym("≠")` ⇒ `notation-unknown-symbol`（实测）。
- `infix:50 " ≠ " => Ne` ⇒ **`elab-notation-unknown-target`**（`Ne` 不存在，实测 `q5_ne.sokonanoda`）。
- **最小修法实测可行**（`q5_ne_type.sokonanoda` exit 0）：

```
def Ne (α : Type) (a b : α) : Prop := Not (Eq.{1} α a b)
infix:50 " ≠ " => Ne
def q : Prop := 1 ≠ 1
theorem same : Iff (1 ≠ 1) (Not (Eq.{1} Nat 1 1)) := …   -- exit 0，两种写法同判
```

- **宇宙多态版会炸**（实测 `n1.sokonanoda`）：
  `def Ne {u} (α : Sort u) (a b : α) : Prop := Not (Eq.{u} α a b)` + `1 ≠ 1`
  ⇒ **内核断言** `rejected: assertion 'left == right' failed / left: 1 / right: 0`。
  原因见 §11.2（`elab.rs:1004` 给常量传了**空宇宙层**）。
  `Ne.{1}` 点名写法本身是好的（`n2`/`n3` exit 0）——**只有记法路径**炸。
- 跨 import 可用（实测 `p5`：`lib/N.sokonanoda` 声明 `Ne` + `≠`，单元 `import lib.N` 后 `1 ≠ 2` exit 0）。

### 6.3 顺带：`≤` / `∘`

同样是「符号合法、目标不存在」：`Nat.le`、`Function.comp` 都不存在 ⇒
`elab-notation-unknown-target`（实测 `q5_le`/`q5_comp`）。要它们就得先有常量。

---

## 7. Q6 —— 作用域与传播

| 场景 | 行为 | 证据 |
|---|---|---|
| 本文件、声明**之后** | ✅ 生效 | 设计 N5（`notation-subset.md:121-133`） |
| 本文件、声明**之前** | ❌ `notation-unknown-symbol`（parse，不是 elab） | `u5.sokonanoda` exit 1 |
| 跨 `import`：被导入模块声明 | ✅ **入口从文件头就可用** | `u1.sokonanoda` exit 0（`import lib.Lib` + `lib/Lib.sokonanoda` 声明 `⊕`） |
| 跨 `import` 但**没 import 那个模块** | ❌ 未声明符号（按 import 边传播，不是闭包全局） | `crates/cli/tests/notation.rs:451` 的反向对照② |
| 本文件**重声明 import 来的符号** | ❌ **硬错**：`符号 '⊕' 已经声明过记法了（由 import 带进来）`（`parser.rs:952-959`） | `u2.sokonanoda` exit 1（经 `import-module-invalid` 包装） |
| 两个**被 import 的模块**各声明同一符号 | ⚠️ **静默后者覆盖**（无诊断） | `p3/units/both.sokonanoda` exit 0；`both2` 实测 B 覆盖 A |
| 同文件同符号**同形状** | ✅ **重载**（按期望类型选） | `u3.sokonanoda`：`def p` 在第二条声明之前 ⇒ 单候选 ⇒ exit 0 |
| 同文件同符号**不同形状** | ❌ `notation-shape` | `u4.sokonanoda` exit 1 |
| `scoped infix…` + `open scoped Foo` | ✅ 生效 | `s1` exit 0 |
| `scoped` 没 `open scoped` | ❌ `notation-unknown-symbol` | `s2` exit 1 |
| `scoped` 写在 `namespace` 外 | ❌ `notation-shape`（hint 教包 namespace） | `s3` exit 1 |
| `open scoped Bar`（Bar 不存在） | ⚠️ **不报错**，但记法仍不可用 | `s4` exit 1（错在符号，不在 scope 名） |
| scoped 记法跨 import | ✅ 传播但**挂起**，入口要自己 `open scoped` | `p4/units/e1` exit 0 / `e2` exit 1 |

**prelude 里声明记法会怎样？** 【代码读，无法实测（要改 prelude 才能测）】
**会被静默忽略**：`install_l1_prelude`（`prelude.rs:371-397`）只对
`command_belongs_to(command, family)` 为真的命令调 `install_l1_command`，
而该判据只认 `Command::Axiom`/`Def`/`InductiveBlock`（`prelude.rs:400-408`，
`_ => false`）。`Command::Notation` 落进 `_ => false` ⇒ **一条都不会装**。
而且 parser 的记法表根本没有「内建种子」入口——它只有
`parse_with_inherited(src, &[NotationDecl])`（`parser.rs:2796-2809`）这一条注入路。
⇒ 想让 `∧∨↔¬` 全局可用，**不能靠 prelude 写记法命令**（见 §12 M5）。

---

## 8. Q7 —— 优先级与结合性（确切规则 + 实测）

### 8.1 梯子

`parse_arrow`（最松）→ **`parse_operators`（记法 + `+`）** → `parse_app`（最紧）。
`+` 是保留项，**优先级 65**（`parser.rs:20` `const PLUS_PRECEDENCE: u16 = 65;`），
仍产出 `Expr::Plus`（`parser.rs:1905-1911`）。记法优先级范围 **1–1000**
（`parser.rs:23`）。

### 8.2 脱糖（`parser.rs:1898-1902` + `1927-1941`）

| 结合 | 左操作数要求 | 右操作数要求 | 代码 |
|---|---|---|---|
| `infix` | `p` | `p + 1` | `next_min = p + 1` |
| `infixl` | `p` | `p + 1` | 同上 |
| `infixr` | `p + 1` | `p` | `next_min = p` |

`infix` 的「不许连写」由 `parse_operand` 的**同级检查**实现
（`parser.rs:1929-1939`）。

### 8.3 实测（全部用 `Eq.refl` 做**内核级**分组判别，不用文本比对）

| 命题 | 判别式 | 结果 | 结论 |
|---|---|---|---|
| `a ∧ b ∧ c`（`infixl:35`） | `Eq.{1} Prop (a∧b∧c) (And (And a b) c)` | ✅ checked | **左结合** |
| `a ∧ b ∧ c`（`infixr:35`） | 同左式 | ❌ rejected | **右结合** |
| `a ∧ b ∨ c`（∧35 / ∨30） | `= Or (And a b) c` | ✅ | ∧ 比 ∨ 紧 |
| `a ∨ b ∧ c`（∨36 / ∧35） | `= And a (Or b c)` | ✅ | 数字大者紧 |
| `a ∧ b -> c`（∧:1） | `= (And a b) -> c` | ✅ | **`->` 比任何记法都松**（连 `:1` 都紧） |
| `f a ∧ b` | `= And (f a) b` | ✅ | **函数应用最紧** |
| `1 + 2 ⊕ 3`（⊕70） | `= Nat.add 1 (P2 2 3)`（`axiom P2`，不计算） | ✅ | ⊕(70) 比 `+`(65) 紧 |
| `1 + 2 ⊙ 3`（⊙60） | `= P2 (Nat.add 1 2) 3` | ✅ | `+`(65) 比 ⊙(60) 紧 ⇒ **`+` = 65 实测钉死** |
| 同优先级两个 `infixl:35` 符号 `a ⊕ b ⊗ c` | `= And (And a b) c` | ✅ | 同级不同符号 = **左结合** |
| 同优先级两个 `infixr:35` 符号 | `= And a (And b c)` | ✅ | 右结合 |
| 同优先级两个 `infix:35` 符号 | —— | ❌ **parse 错** | ⚠️ 见下 |

### 8.4 ⚠️ 设计文档与代码不一致（**必须记一笔**）

`docs/design/notation-subset.md:86-89` 写：

> 「不同符号同级按**左结合**处理」

**代码不是这样**：`parse_operand`（`parser.rs:1929-1939`）对
`binary_op_ahead(op.precedence)` 只比**优先级数字**、不看符号是否相同：

```rust
if op.assoc == NotationAssoc::Infix {
    if let Some(next) = self.binary_op_ahead(op.precedence) {
        if next.precedence == op.precedence {   // ← 同优先级就报错，符号可以不同
            return Err(self.error_here(&format!(
                "`{s}` 没有结合性：`a {s} b {n} c` 要写成 …")));
```

实测 `g10.sokonanoda`：两个**不同**的 `infix:35` 符号连写 ⇒
`unexpected-token: '⊕' 没有结合性：'a ⊕ b ⊗ c' 要写成 …`。
⇒ 文档该改（或代码该改），**二者必居其一**。

### 8.5 推荐优先级（Lean 4 core 值，**实测可用**）

| 符号 | 形状 | 目标 | 数字 |
|---|---|---|---|
| `→` | `infixr:25` | **需语言改动**（§12 M1） | 25 |
| `↔` | `infix:20` | `Iff` | 20 |
| `∨` | `infixl:30` | `Or` | 30 |
| `∧` | `infixl:35` | `And` | 35 |
| `¬` | `prefix:40` | `Not` | 40 |
| `≠` | `infix:50` | `Ne`（要新加，§12 M2） | 50 |
| `∈` | `infix:50` | `Set.mem` | 50（课程现状） |
| `⊆` | `infix:50` | `Set.subset` | 50（课程现状） |
| `∪` | `infixl:65` | `Set.union` | 65（课程现状） |
| `∅` | `notation` | `Set.empty` | 无 |

整套实测：`leanish.sokonanoda` 的 `t1`–`t5` 五条分组判别**全部 checked**；
真实课程库端到端 `probe.sokonanoda` **exit 0、13 条 `decl.checked`、0 诊断**。

---

## 9. Q8 —— print-back（**实测有两套行为**）

设计文档 §13.1（`notation-subset.md:550`）说「源码级 print-back **不做**，
goal/hover 显示点名形式」。**实测更细**：

| 消费点 | 显示什么 | 证据 |
|---|---|---|
| `query goals` 的 `goal` 字段 | **记法**（`"A ⊆ B"`） | 实测 `q8.sokonanoda`：`goal: "A ⊆ B"` |
| `query goals` 的 `binders[].ty` | **记法**（`h : A ⊆ B`） | 同上 |
| `query goals` 的 `ty`（声明类型） | **点名**（`Set.subset α A B -> …`） | 同上 |
| `query state` 的 `goal` | **点名**（`forall (α : Type 0) (A B : Set α), Set.subset α A B -> Set.subset α A B`） | 实测 `q8` |
| `#check` / `expr.typed` 的类型文本 | **点名** | `notation-cheatsheet.sokonanoda:95-97` 的注释 + 实测 |
| hover | 记号节点整段 `lhs sym rhs` 一条行（设计 §4 表；**本次未实测**） | `notation-subset.md:208` |

**原因（读代码）**：
- 「显示记法」的那条路 = front 自己的 `render_expr`，它有 `Expr::Notation` 臂
  （`crates/front/src/proof.rs:323-340`），而 `compile/goals.rs:1073`（及
  `1215`/`1335`/`1437`）就是 `goal: render_expr(ty)`。
- 「显示点名」的那条路 = **冻结内核的 pp**：`query state` 的 goal、声明的 `ty`、
  `#check` 的 `inferred_type` 都是内核文本。
- 所以不是「没有 print-back」，而是**只在前端渲染的源级目标上 print-back**；
  一旦经过内核 pp（`judge_type_of`/`judge_infer`/`#check`/by 回读）就回到点名形式。

**教学后果**：目标栏看到的是记法（好事），但**报错信息**与 `#check` 是点名形式
（与课程 `notation-cheatsheet.sokonanoda:30-35` 的说法一致，但比设计文档
§13.1 的一句话更乐观）。

---

## 10. Q9 —— 重复声明

| 情形 | 行为 | 证据 |
|---|---|---|
| 同文件、同符号、**同形状**（结合性 + 优先级一致） | ✅ **重载**：`Expr::Notation.alternatives` 多候选，按期望类型选 | `parser.rs:960-980`；`crates/cli/tests/notation.rs:725`；实测 `u3` |
| 同文件、同符号、**不同形状** | ❌ `notation-shape` | `parser.rs:971-980`；实测 `u4` |
| 同文件、同符号、同形状，但**期望类型是 `Prop`** | ⚠️ 两个候选都「结果类型 = `Prop`」⇒ `notation_result_matches` 对 `Sort` 头返回 false ⇒ **`elab-notation-no-candidate`**（不是 ambiguous） | `elab.rs:1349-1352`；`parser.rs:2249`（`Prop` ⇒ `Expr::Sort`）；实测 `u3a/u3b` |
| 重声明 **import 来的**符号 | ❌ 硬错 | `parser.rs:952-959`；实测 `u2` |
| 两个**被 import 的模块**各声明一次 | ⚠️ **静默后者覆盖**（无诊断、无提示） | 实测 `p3`（A 声明 `And`、B 声明 `Or`；`both2` 证明 B 生效） |
| **prelude 里声明记法** | 【代码读】静默忽略，不生效、也不与任何人冲突 | `prelude.rs:400-408` |

**课程级硬约束（实测，最重要的一条）**：把 `∈/⊆/∪/∅` 加进
`lib/Set.sokonanoda` 之后，`units/notation-cheatsheet.sokonanoda`（它自己在
86–89 行声明了同样四条）**当场判红**：

```
import-module-invalid: 符号 `∈` 已经声明过记法了（由 import 带进来）：
同一个符号在**同一文件**里可以重载，但不能覆盖 import 来的记法
```

而没声明这四条的 `unit02-subsets-empty.sokonanoda` 不受影响（仍 exit 0）。
⇒ **迁移时必须先把 cheatsheet 的本地四条删掉**，符号只能有一个声明点。

---

## 11. Q10 —— 前导类型参数补全

### 11.1 规则（`elab.rs:1411-1513`）

1. 读目标签名：`judge_type_of(prefix_src, …)`（`elab.rs:967`）⇒ 内核 pp 文本。
2. `notation_telescope`（`elab.rs:1451-1460`）用 `spine::peel_pi` 剥成
   `layers: Vec<(名字, 域)>` + `result`。
3. 操作数对齐到**最后 `operands.len()` 层**；`missing = layers.len() - operands.len()`。
   - `missing == 0` ⇒ **不补**（`elab.rs:1426-1428`）。
   - `missing > 0` ⇒ 从**最大候选往下试**（`1434-1446`），每位按
     **① 操作数类型 → ② 期望类型** 的顺序解（`solve_prefix_args` `1464-1513`）。
4. 求解是**头部匹配 + 提裸变量**（`unify_extract` `1602-1645`）：模板是裸变量
   `name` ⇒ 取实际；同头同实参个数 ⇒ 对应位；`->`/`forall` 也算二元头
   （靠 `peel_pi`，`1613-1619`）。**不做一般合一、不引入元变量**。
5. 解不出 ⇒ `elab-notation-argument-unsolved`（`elab.rs:986-993`）。
6. 补好的完整应用**直接 `mk_const` + `mk_app`**（`1003-1024`），不再回读源码。

### 11.2 边界（逐条实测）

| 形状 | 例 | 结果 |
|---|---|---|
| 0 个前导参数 | `And (a b : Prop) : Prop`，`a ∧ b` | ✅ |
| 1 个前导参数（由操作数解） | `Set.mem (α) (a) (A)`，`a ∈ A` | ✅ |
| 1 个前导参数（由**期望类型**解） | `Set.empty (α) : Set α`，`def e (α : Type) : Set α := ∅` | ✅ `a1` |
| 零元记法在**操作数位**（期望类型来自被调用者） | `A ⊆ ∅`、`∅ ⊆ A` | ✅ `a3` |
| 零元记法**无期望类型** | `Eq.{1} (Set α) A ∅` | ❌ `elab-notation-argument-unsolved`（与 `notation-cheatsheet.sokonanoda:52-59` 一致）`a2` |
| 2 个前导参数 | `Set.image (α β) (f) (A)`，`f '' A` | ✅（课程库 `probe` 的 `p9` exit 0；第二刀 as-built `notation-subset.md:445-453`） |
| 首参是 `Type` 但**层数 == 操作数个数** | `axiom G : (α : Type) -> (a : α) -> Prop`，`1 ⊛ 1` | ❌ **内核裸错**（`G 1 1` ⇒ α := 1）`a5` |
| 首参是**隐式** `{α : Type}` | `def F {α : Type} (a : α) : Prop`，`1 ⊛ 1` | ❌ 内核裸错（不跳隐式 binder）`a4` |
| 目标是**宇宙多态**常量 | `def Id2 {u} (α : Sort u) (a : α) : α`，`1 ≈ 1` | ❌ **内核断言** `left: 1 / right: 0`（空宇宙层，见下）`lvl` |
| 点名省参数（**护城河**） | `Set.mem a A` | ❌ 仍被内核拒（设计 N4.3，`a6` 实测） |

### 11.3 宇宙多态目标 = 内核断言（**新发现的硬边界**）

`elab_notation` 构造常量时给的是**空宇宙层切片**（`elab.rs:1003-1005`）：

```rust
let const_name = builder.name_from_str(&canonical);
let levels = builder.alloc_levels_slice(&[]);   // ← 永远 0 层
let mut app = builder.mk_const(const_name, levels);
```

目标若有 ≥1 个宇宙参数（`Eq`、任何 `{u}` 定义），内核收到 0 层 ⇒
`rejected: assertion 'left == right' failed / left: 1 / right: 0`
（`left` = 期望层数、`right` = 实给层数）。实测 `lvl` / `n1` / `eq2`。
**后果**：`infix:50 " = " => Eq` 这类最自然的写法今天**必然炸**（即使词法问题解决）。

---

## 12. 额外发现（问题清单外，但改写前必须知道）

### 12.1 `=` 根本不存在，而且**声明不了**

- 语言里没有中缀相等：`def p : Prop := 1 = 1` ⇒
  **`unexpected-token: expected '=>', found =`**（`token.rs:296-308`：`=` 只做 `=>` 的前缀）。
- 把 `=` 声明成记法符号**会拆掉 `=>`**：`infix:50 " = " => Eq` ⇒
  `unexpected-token: expected a valid .sokonanoda token, found >`（实测 `eq1`）。
  原因：`declared_symbol_ahead`（`token.rs:208`）排在 `'=' =>`（`token.rs:296`）
  **之前**，所以在 `=>` 的 `=` 处先命中声明的 `=`，把 `>` 剩给通用错误臂。
  （`:=` 不受影响——`:` 臂一次吃掉两个字符，`token.rs:251-259`。）
- 即使词法解决，`Eq` 是宇宙多态 ⇒ 撞 §11.3 的断言（实测 `eq2`）。
- 可行的**绕过**：monomorphic 别名 `def Eq2 (α : Type) (a b : α) : Prop := Eq.{1} α a b`
  + 另一个符号（`≐`）⇒ exit 0（实测 `eq3`）。但**符号 `=` 本身今天用不了**。

### 12.2 `apply` 不认记法（**今天就会踩**）

`by.rs:362-374`：

```rust
let goal = nodes[cur].ty.clone();
let sigma = unify_spine(&codomain, &goal, &layers).ok_or_else(|| … "`apply` 的目标不匹配" …)
```

`unify_spine` 是**语法**脊合一，不会展开 `Expr::Notation`。实测：

| 目标 | 战术 | 结果 |
|---|---|---|
| `a ∧ b`（记法） | `apply And.intro` | ❌ `apply 的目标不匹配：And.intro 的结果是 And a b，无法对齐当前目标 a ∧ b`（`k1`） |
| `b ∧ a`（记法） | `apply And.intro` | ❌ 同上（`q8d`） |
| `And a b`（点名） | `apply And.intro` | ✅（`j5`） |
| `a ∧ b`（记法） | `exact h` | ✅（`j2`/`j3`/`j4`） |
| `a ∧ b`（记法） | `assumption` | ✅（`l1`） |
| `Aᶜ`（记法） | `rfl` | ✅（`l2`） |

⇒ 四个战术里只有 `apply` 对记法敏感。**卷 I 今天零 `by` 块、零 `apply`**
（实测 `grep -c "by"` 在 `unit02` 为 0），所以**不是当前改写的拦路虎**；
但一旦改写后要用 `apply`，就是硬阻塞（见 §12 M6）。

### 12.3 一条声明失败 ⇒ 后面所有记法报**误导性**错误

`judge_type_of_uncached`（`judge.rs:372-395`）把
`prefix_src + "#check <term>"` **整体重新编译**，`report.errors` 非空就返回
**第一条**错误。⇒ 文件里只要有**任何**先前失败的声明，后面每个记法目标查询都报
`elab-notation-unknown-target: 读不到记法 '∧' 的目标 'And' 的类型：<前面那条错误>`。
实测可复现（`cascade` / `cascade2`）；失败声明**之后**的记法才受影响
（`cascade3`：记法在前 ⇒ 正常）。诊断质量 bug，不是语义 bug。

### 12.4 两个被 import 的模块同符号 = 静默覆盖（见 §10）

课程侧风险：若 `lib/Logic` 与 `lib/Set` 都声明 `∈`，单元 `import` 两者时
**没有任何诊断**，后 import 的赢。建议：符号只放一处 + 加一条测试守护。

---

## 13. 最小语言改动清单

> 目标：`∧ ∨ ↔ ∀ ∃ ¬ → ≠`（外加 `=`）全部 Lean 4 风格可用。
> 硬规则：`crates/kernel/**` 一个字节不动；下列全部落在 `front`/课程库。

| # | 项 | 改哪 | 规模 | 内核风险 | 违反内核冻结？ |
|---|---|---|---|---|---|
| **M1** | **`→` 变成真正的箭头**（唯一必须动的 parser 改动）：在 `token.rs:279` 的 `'∀' => Forall` 旁边加一臂 `'→' => self.single(TokenKind::Arrow, start)`（**必须在 `is_ident_start` 分支之前**）。这样 `(x : α) → β`、`A → B`、`Nat → Nat` 全部与 `->` 逐字同义，且 `→` 与记法系统无冲突（与 Lean 把 `→` 当内建 token 一致）。 | `crates/front/src/token.rs`（约 2 行 + 注释） | **~5–20 行**（含三层测试） | 无（只多一个 token 别名，产出既有 `TokenKind::Arrow`） | ❌ 不违反（front only） |
| **M2** | **`Ne` + `≠`**：`def Ne (α : Type) (a b : α) : Prop := Not (Eq.{1} α a b)` + `infix:50 " ≠ " => Ne`。放课程库（`lib/Logic.sokonanoda` 或 `lib/Set.sokonanoda` 末尾）即可，**不必进 prelude**（进 prelude 还要动 `PRELUDE_NAMES`/族表与 golden）。**必须 monomorphic**（`{u}` 版撞 §11.3）。 | `courses/set-theory/lib/*.sokonanoda` | **2 行** | 无 | ❌ 不违反 |
| **M3** | （可选，但 `=` 必需）**修记法路径的宇宙层**：`elab.rs:1004` 的 `alloc_levels_slice(&[])` 改成按目标常量的宇宙参数个数分配（需要从 env/内核读常量层数；`EnvBuilder` 要有对应查询）。修完 `infix:50 " ≐ " => Eq` 之类宇宙多态目标才可用；且**必须保证 0 层常量的行为逐字节不变**。 | `crates/front/src/compile/elab.rs`（+ `builder` 的小 API） | **~10–40 行** | 低–中（要小心不改变既有 0 层路径） | ❌ 不违反 |
| **M4** | （可选）**`=` 作为符号**：除 M3 外还要让 `=` 不与 `=>` 抢词法。最小做法：把 `'=' =>` 臂的 `=>` 判断**提前到声明符号匹配之前**（或在 `declared_symbol_ahead` 里排除后跟 `>` 的 `=`）。**建议直接不做**——Lean 课程里 `=` 可用 `Eq.{1}` 点名或换符号（`≐`）表达，成本收益不划算。 | `crates/front/src/token.rs` | ~5 行 | 中（动词法主路径，回归面大） | ❌ 不违反 |
| **M5** | （可选，若想让 `∧∨↔¬` **全局免声明**）**给 parser 一个内建记法种子**：`parse_with_inherited`（`parser.rs:2796`）已有注入点，把一张常量表在 `parse`/`parse_fragment`/`graph.rs:376`/`proof.rs:57` 四处喂进去；**不要**试图写在 `PRELUDE_L1_SRC` 里（§7 证明会被静默忽略）。代价：4 个调用点 + 一张表 + 测试。 | `crates/front/src/parser.rs`、`project/graph.rs`、`proof.rs` | **~30–80 行** | 低 | ❌ 不违反 |
| **M6** | （可选，**只在改写后要用 `apply` 时必需**）**让 `apply` 看见记法**：`by.rs:363` 之前把 goal（以及 `codomain`）里的 `Expr::Notation` 走一遍与 `elab_notation` 同源的**降级**，再 `unify_spine`；或改用内核级判定。 | `crates/front/src/by.rs` | **~30–80 行** | 中（tactic 判定路径，回归面 = 全部 `by` 用例） | ❌ 不违反 |
| **M7** | （可选，诊断质量）**修 §12.3 的级联**：`judge_type_of_uncached` 不要把「前缀里先前声明的错误」当成目标类型读取失败（例如只看最后一个命令的事件/错误，或让调用方带一个容错开关）。 | `crates/front/src/judge.rs` | **~10–20 行** | 无 | ❌ 不违反 |
| **M8** | （文档）修 `docs/design/notation-subset.md:86-89` 的「不同符号同级按左结合」——实测是**报错**（§8.4）。 | 文档 | 1 段 | 无 | —— |
| **M9** | （可选，`⟨a, b⟩`）**占位符记法**（Lean 的 `notation "⟨" a "," b "⟩" => …`）：今天只有 `infix/infixl/infixr/prefix/postfix/notation/binder_notation` 六种形状（`parser.rs:701-839`），**没有**带占位符的多段形状 ⇒ `⟨_,_⟩` 无法表达。要做得加一条命令 + AST + elab（约 150–300 行 + 测试）。课程若不需要 `⟨⟩` 就别做。 | `parser.rs`/`ast.rs`/`elab.rs` | **~150–300 行** | 无（降级成 `mk_app`） | ❌ 不违反 |

### 13.1 「今天零改动就能用」的清单（不用等 M1–M9）

```
infix:20 " ↔ " => Iff
infixl:30 " ∨ " => Or
infixl:35 " ∧ " => And
prefix:40 " ¬ " => Not
infix:50 " ∈ " => Set.mem
infix:50 " ⊆ " => Set.subset
infixl:65 " ∪ " => Set.union
notation "∅" => Set.empty
binder_notation "∃" => Exists        -- 需要 import lib.Exists
```

外加**语言自带的** `∀`（`∀ (x : α), p` / `∀ x : α, p` / `∀ x ∈ s, p`）。
实测端到端（`probe.sokonanoda`，真课程库 + 上面九条 + `Ne`/`Imp`）：**exit 0、
13 条 `decl.checked`、0 诊断**。

### 13.2 课程迁移的**非语言**约束（实测）

1. **一个符号一个声明点**：`lib/Set.sokonanoda:155-159` 已有五个符号；
   `notation-cheatsheet.sokonanoda:86-89` 自己声明了 `∈/⊆/∪/∅`。
   把后者搬进 lib 会让 **cheatsheet 判红**（§10 实测）⇒ 迁移第一步是
   **删掉 cheatsheet 的本地四条**。
2. **别让两个 lib 声明同一符号**（§12.4 静默覆盖）。
3. **`∀`/`∃` 做算子操作数要加括号**（§5 实测）。
4. 改完跑 `python3 courses/set-theory/tools/check.py`（记法零事件 ⇒
   36/329/99/0 应当逐项不变；`crates/cli/tests/notation.rs:567` 的
   `the_shipped_course_still_uses_the_pointful_spelling` 会挡住「画布仍在点名」，
   真要改课程时**必须同一轮更新该测试**）。

---

## 14. 不确定 / 有风险的点

1. **`→` 走 M1 的硬编码路线**会与「用户想自己声明 `→`」冲突（声明 `→` 会永远
   命中不了，`lexer_reserved_symbol_char` 目前**不**拦 `→`）。要么把 `→` 加进
   `lexer_reserved_symbol_char`（顺带给出「`→` 是内建箭头」的教学诊断），
   要么接受这个静默冲突。**未实测**（要改代码才能测）。
2. **M3（宇宙层）的内核交互**只从症状（`left: 1 / right: 0`）与代码
   （`elab.rs:1004`）推断，**没有**读内核侧断言点确认；修法需要 `EnvBuilder`
   能问常量层数——该 API 今天是否存在**未核实**。
3. **`notation_result_matches` 对 `Sort` 头恒 false** 是从「`Prop` ⇒
   `Expr::Sort`（`parser.rs:2249`）+ 代码 `elab.rs:1349-1384` + 实测
   `no-candidate`（而不是 `ambiguous`）」三方推出的，**没有**直接打印中间值。
   结论（Prop 值的重载无法消歧）是实测的；机理是推断的。
4. **hover 的 print-back 行为未实测**（要起 LSP）。本文只引设计 §4 表
   （`notation-subset.md:208`）。`query goals` vs `query state` 的差异是实测的。
5. **`apply` 之外是否还有别的消费者做语法比对**：只查了 `by.rs` 的四个战术
   （`intro`/`exact`/`apply`/`assumption`/`rfl`）。LSP 的 code action、
   `suggest`、`references` 未审。
6. **设计文档 §N3 与代码不一致**（§8.4）到底是「文档没跟上」还是「代码有 bug」
   ——本文只陈述事实，取舍留给决策者。
7. **prelude 里记法命令被忽略**是**代码读**结论（`prelude.rs:400-408`），
   没有实测（要改 `PRELUDE_L1_SRC` 才能测，属于改现有文件，本次不允许）。

---

## 15. 附录：实测记录

### 15.1 方法

- 判卷：`scripts/soko grade <绝对路径> --json`（G-12：一律绝对路径），
  用 `/tmp/notaaudit/show.py` 折叠成「exit + 事件 + 诊断」。
- 临时夹具目录 `/tmp/notaaudit`（**未触碰仓库内任何文件**）；
  课程端到端用 `/tmp/course-test`（`cp -r courses/set-theory/{lib,sokonanoda.toml}`）。
- 分组判别一律用 `Eq.refl`（内核级），**不做文本比对**。
- 二进制：`sokonanoda 0.61.0`（`target/debug/sokonanoda`；
  注意 `scripts/soko doctor --json` 报的 `marker: "0.55.0 darwin-arm64"`
  是**过期的标记文件**，`--version` 实测是 0.61.0）。

### 15.2 决定性夹具与输出

**(a) Q1 码点类（`#check <符号>`）**

| 文件 | 源码 | exit | 诊断 |
|---|---|---|---|
| `q1_2227` | `#check ∧` | 1 | `parse notation-unknown-symbol: 符号 '∧' …` |
| `q1_2203` | `#check ∃` | 1 | `parse notation-unknown-symbol: 符号 '∃' …` |
| `q1_2260` | `#check ≠` | 1 | `parse notation-unknown-symbol: 符号 '≠' …` |
| `q1_2192` | `#check →` | 1 | `elab elab-unknown-identifier: unknown identifier '→'` |
| `q1_2194` | `#check ↔` | 1 | `elab elab-unknown-identifier: unknown identifier '↔'` |
| `q1_00AC` | `#check ¬` | 1 | `elab elab-unknown-identifier: unknown identifier '¬'` |
| `q1_27E8` | `#check ⟨` | 1 | `elab elab-unknown-identifier: unknown identifier '⟨'` |
| `q1_2200` | `#check ∀` | 1 | `parse unexpected-token: expected a binder, found Eof` |
| `q1_bs` | `#check \` | 1 | `parse notation-unknown-symbol: 符号 '\' …` |
| `q1_apos` | `#check '` | 1 | `parse notation-unknown-symbol: 符号 ''' …` |
| `q1_munch` | `#check True ≠≤ True` | 1 | `parse notation-unknown-symbol: 符号 '≠≤' …`（**最大咬合**） |
| `q1b_2A05` | `#check ⨅` | 1 | `parse notation-unknown-symbol`（U+2A00–2AFF 也在类里） |
| `q1b_21D2` | `#check ⇒` | 1 | `elab elab-unknown-identifier`（U+21D2 **不在**类里） |

**(b) Q2 四条 prelude 记法**

```lean
infix:35 " ∧ " => And
infix:30 " ∨ " => Or
infix:20 " ↔ " => Iff
prefix:40 " ¬ " => Not
def p : Prop := True ∧ True
def q : Prop := True ∨ False
def r : Prop := True ↔ True
def s : Prop := ¬ True
```
⇒ `exit=0`，`decl.checked p/q/r/s`，0 诊断。

**(c) Q3 `→`**

```lean
-- q3_arrow_alias.sokonanoda
infixr:25 " → " => Arrow
def p : Prop := True → True
```
⇒ `exit=1` `elab-notation-unknown-target: 记法 '→' 指向的目标 'Arrow' 不存在`

```lean
-- q3_alias2.sokonanoda（可用形态）
def Imp (A B : Prop) : Prop := A -> B
infixr:25 " → " => Imp
theorem t (A B : Prop) (f : A -> B) : A → B := f
theorem t2 (A B : Prop) (f : A → B) : A -> B := f
def g (A B : Prop) (f : A → B) (h : A) : B := f h
```
⇒ `exit=0`（四条 `decl.checked`）

```lean
-- q3_paren_arrow.sokonanoda（依赖箭头）
infixr:25 " → " => Imp
theorem dep (α : Type) (p : α -> Prop) : (x : α) → Prop := p
```
⇒ `exit=1` `parse unexpected-token: expected '->' after binder group, found Sym("→")`

**(d) Q4 binder（见 §5 表；夹具 `b1`–`b16`）**

**(e) Q5 `≠`**

```lean
-- q5_ne_type.sokonanoda ⇒ exit=0
def Ne (α : Type) (a b : α) : Prop := Not (Eq.{1} α a b)
infix:50 " ≠ " => Ne
def q : Prop := 1 ≠ 1
theorem same : Iff (1 ≠ 1) (Not (Eq.{1} Nat 1 1)) :=
  Iff.intro (1 ≠ 1) (Not (Eq.{1} Nat 1 1)) (fun (h : 1 ≠ 1) => h) (fun (h : Not (Eq.{1} Nat 1 1)) => h)
```

```lean
-- n1.sokonanoda ⇒ exit=1（宇宙多态版）
def Ne {u} (α : Sort u) (a b : α) : Prop := Not (Eq.{u} α a b)
infix:50 " ≠ " => Ne
def q : Prop := 1 ≠ 1
```
⇒ `kernel-rejected: rejected: assertion 'left == right' failed / left: 1 / right: 0`
（`decl.checked Ne` 成功、`Ne.{1}` 点名可用（`n2`/`n3` exit 0）——**只有记法路径**炸）

**(f) Q7 分组判别（节选）**

```lean
-- g1.sokonanoda（infixl ⇒ 左结合）
infixl:35 " ∧ " => And
theorem left  (a b c : Prop) : Eq.{1} Prop (a ∧ b ∧ c) (And (And a b) c) := Eq.refl.{1} Prop (And (And a b) c)
theorem right (a b c : Prop) : Eq.{1} Prop (a ∧ b ∧ c) (And a (And b c)) := Eq.refl.{1} Prop (And a (And b c))
```
⇒ `decl.checked left`，`right` 被内核拒 ⇒ 左结合

```lean
-- h1.sokonanoda（⊕70 vs +65）
axiom P2 : Nat -> Nat -> Nat
infixl:70 " ⊕ " => P2
theorem plus_tighter : Eq.{1} Nat (1 + 2 ⊕ 3) (Nat.add 1 (P2 2 3)) := Eq.refl.{1} Nat (Nat.add 1 (P2 2 3))
```
⇒ checked（⊕ 比 `+` 紧）；`h2`（⊙60）反之 ⇒ **`+` = 65 实测钉死**

```lean
-- g10.sokonanoda（infix 同级不同符号）
infix:35 " ⊕ " => And
infix:35 " ⊗ " => And
theorem left (a b c : Prop) : Eq.{1} Prop (a ⊕ b ⊗ c) (And (And a b) c) := …
```
⇒ `exit=1` `unexpected-token: '⊕' 没有结合性：'a ⊕ b ⊗ c' 要写成 …`
（**与设计 §N3「不同符号同级按左结合」矛盾**）

**(g) Q8 print-back（`q8.sokonanoda`）**

```
import lib
infix:50 " ⊆ " => Set.subset
theorem t (α : Type) (A B : Set α) (h : A ⊆ B) : A ⊆ B := by
  sorry
```

- `query state --line 5 --col 3` ⇒
  `"goal": "forall (α : Type 0) (A B : Set α), Set.subset α A B -> Set.subset α A B"`（**点名**）
- `query goals` ⇒ `"goal": "A ⊆ B"`（**记法**）、
  `binders[].ty` 里 `h` 是 `"A ⊆ B"`（**记法**）、
  `"ty"` 是 `"forall (α : Type 0) …, Set.subset α A B -> …"`（**点名**）

**(h) Q9 重复声明 / 课程迁移实验**

```lean
-- p3/lib/A.sokonanoda: infix:50 " ⊕ " => And
-- p3/lib/B.sokonanoda: infix:50 " ⊕ " => Or
-- p3/units/both.sokonanoda:
import lib.A
import lib.B
def p : Prop := True ⊕ True        ⇒ exit=0（静默）
-- p3/units/both2.sokonanoda:
theorem t : Eq.{1} Prop (True ⊕ False) (Or True False) := Eq.refl.{1} Prop (Or True False)   ⇒ exit=0
```
⇒ **B（后 import 的）赢**，无任何诊断

```bash
cp -r courses/set-theory /tmp/course-mig
cat >> /tmp/course-mig/lib/Set.sokonanoda <<'EOF'
infix:50 " ∈ " => Set.mem
infix:50 " ⊆ " => Set.subset
infixl:65 " ∪ " => Set.union
notation "∅" => Set.empty
EOF
scripts/soko grade /tmp/course-mig/units/notation-cheatsheet.sokonanoda
```
⇒ `exit=1` `import-module-invalid: 符号 '∈' 已经声明过记法了（由 import 带进来）…`
（`unit02-subsets-empty.sokonanoda` 仍 `exit=0`）

**(i) Q10 + 端到端（`/tmp/course-test/units/probe.sokonanoda`，真课程库）**

```lean
import lib.Logic
import lib.Set
import lib.Exists
import lib.Image
def Ne (α : Type) (a b : α) : Prop := Not (Eq.{1} α a b)
def Imp (A B : Prop) : Prop := A -> B
infixr:25 " → " => Imp
infix:20 " ↔ " => Iff
infixl:30 " ∨ " => Or
infixl:35 " ∧ " => And
prefix:40 " ¬ " => Not
infix:50 " ≠ " => Ne
infix:50 " ∈ " => Set.mem
infix:50 " ⊆ " => Set.subset
notation "∅" => Set.empty
binder_notation "∃" => Exists
def p1 (α : Type) (a : α) (A : Set α) : Prop := a ∈ A ∧ ¬ (a ∈ A)
def p2 (α : Type) (A B : Set α) : Prop := A ⊆ B ↔ (∀ (x : α), x ∈ A → x ∈ B)
def p3 (α : Type) (A : Set α) : Prop := A ⊆ A ∨ A ≠ ∅
def p4 (α : Type) (A : Set α) : Prop := ∃ (x : α), x ∈ A ∧ x ∈ A
def p5 (α : Type) (s : Set α) (q : α -> Prop) : Prop := ∃ x ∈ s, q x
def p6 (α : Type) (s : Set α) (q : α -> Prop) : Prop := ∀ x ∈ s, q x
def p7 (α : Type) (A : Set α) : Set (Set α) := 𝒫 A
def p8 (α : Type) (A : Set α) : Set α := Aᶜ
def p9 (α β : Type) (f : α -> β) (A : Set α) : Set β := f '' A
theorem t1 (α : Type) (a : α) (A : Set α) (h : a ∈ A) : a ∈ A := h
theorem t2 (α : Type) (a : α) (A : Set α) (h : a ∈ A) : ¬ ¬ (a ∈ A) := fun (hn : ¬ (a ∈ A)) => hn h
```
⇒ **`exit=0`，13 条 `decl.checked`，0 诊断**

**(j) `apply` vs 记法（`k1`）**

```lean
infix:35 " ∧ " => And
theorem t (a b : Prop) (h : a ∧ b) : a ∧ b := by
  apply And.intro
  exact And.left a b h
  exact And.right a b h
```
⇒ `exit=1` `elab-tactic-failed: 'apply' 的目标不匹配：'And.intro' 的结果是 'And a b'，无法对齐当前目标 'a ∧ b'`

**(k) 声明失败后的级联（`cascade.sokonanoda`）**

```lean
theorem bad : Prop := True
infix:35 " ∧ " => And
def p : Prop := True ∧ True
```
⇒ 两条诊断：① `kernel-theorem-not-prop`（bad）；②
`elab-notation-unknown-target: 读不到记法 '∧' 的目标 'And' 的类型：rejected: theorem type must be Prop (sort 0)`（**误导**）

**(l) `=`（`eq1`/`eq2`/`eq3`）**

```lean
-- eq1 ⇒ exit=1 unexpected-token: expected a valid .sokonanoda token, found >
infix:50 " = " => Eq
def p : Prop := True
-- eq2 ⇒ exit=1 kernel-rejected: assertion 'left == right' failed / left: 1 / right: 0
infix:50 " ≐ " => Eq
def p : Prop := 1 ≐ 1
-- eq3 ⇒ exit=0（monomorphic 别名可行）
def Eq2 (α : Type) (a b : α) : Prop := Eq.{1} α a b
infix:50 " ≐ " => Eq2
def p : Prop := 1 ≐ 1
```

### 15.3 相关测试索引（`crates/cli/tests/notation.rs`，19 条）

| 行 | 测试 | 钉住什么 |
|---|---|---|
| 108 | `notation_and_pointful_canvases_grade_identically` | N7 五元计数一致 |
| 136 | `notation_emits_no_new_event_kinds` | N6 零事件 |
| 163 | `the_pointful_spelling_keeps_working_and_the_moat_holds` | N4.3 护城河 |
| 180 | `undeclared_symbol_is_a_parse_diagnostic_with_a_teaching_hint` | 未声明符号 |
| 203 | `notation_command_alone_is_a_clean_grade_with_checked_declarations` | 命令零事件 |
| 223 | `notation_nullary_without_an_expected_type_reports_the_unsolved_code` | `∅` 无期望类型 |
| 243 | `stdin_and_a_file_agree_on_a_notation_canvas` | 两通道一致 |
| 357 | `a_real_course_unit_grades_identically_in_notation` | 课程单元记法版同判 |
| 410 | `the_five_second_cut_symbols_grade_clean_in_both_spellings` | 五个符号 |
| 451 | `a_library_notation_works_in_the_entry_through_import` | 跨 import + 两条反向对照 |
| 527 | `the_shipped_course_library_declares_the_five_symbols` | 课程库声明点 |
| 548 | `the_shipped_course_uses_the_library_notation_in_a_demo` | 课程用法 |
| 567 | `the_shipped_course_still_uses_the_pointful_spelling` | **「课程零改动」契约**（要改课程必须同轮改它） |
| 628 | `set_literals_grade_like_the_pointful_singleton_and_pair` | `{a}`/`{a,b}` |
| 644 | `binder_notation_grades_like_the_pointful_exists` | `∃ (n : Nat), …` |
| 663 | `two_stage_binders_grade_like_the_pointful_guard` | `∀ x ∈ s, p` / `∃ x ∈ s, p` |
| 685 | `a_scoped_notation_grades_only_after_open_scoped` | `scoped`/`open scoped` |
| 725 | `an_overload_grades_by_expected_type_and_reports_ambiguity` | 重载（**非 Prop 值**才可消歧） |
| 768 | `the_shipped_course_demos_the_binder_notation` | 课程 `∃` 演示 |

---

## 16. 给改写决策的三句话

1. **`∧ ∨ ↔ ¬ ∀ ∃` 今天就能上**（`∃` 需要 `binder_notation` + `import lib.Exists`；
   `∀`/`∃` 嵌套时要加括号）——实测 `probe.sokonanoda` exit 0。
2. **`→` 与 `≠` 是仅有的两个缺口**，且都**不需要动内核**：`→` 在
   `token.rs` 加一个 token 别名（~2 行），`≠` 加一条 monomorphic `Ne` + 一行
   `infix`（2 行）。`=` 若也要，额外需要修 `elab.rs:1004` 的空宇宙层（~10–40 行）。
3. **真正的迁移成本不在语言，在课程侧的两条硬约束**：一个闭包内**一个符号只能
   声明一次**（重声明 import 来的符号 = 硬错，实测），以及
   `crates/cli/tests/notation.rs:567` 钉住的「课程仍在点名」契约必须同一轮更新。

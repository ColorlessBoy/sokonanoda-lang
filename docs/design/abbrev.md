# `abbrev`（可展开的类型别名）—— G-08 设计 + as-built

> 状态：**已落地（as-built）**。台账 `docs/gaps/ledger.jsonl` 的 G-08；复现件
> `docs/gaps/repro/G08-abbrev.sokonanoda`。硬规则依据：`REQUIREMENTS.md` §2 第 3
> 条（语法增量 = 课程 + 测试 + 白名单三件套）、第 4 条（判定走内核）、第 1 条
> （内核冻结快照）。`def` 的现状见 `docs/design/decl-binders.md`；本文是白名单的
> **边界文档**：`docs/architecture.md` §4.1 只列命令清单，语义规则在这里。

## 0. 一句话

`abbrev` 在本语言里**就是 `def` 的另一个拼写**：同一个 parser 入口、同一个
`Command::Def`、同一个 elaborator、同一个内核声明。加它的唯一理由是**真 Lean
子集兼容**——真 Lean 代码里的 `abbrev` 行可以原样粘进来直接编。

## 1. 先测量再决定：`def` 与 Lean 的 `abbrev` 还有没有可观察差异？

G-08 的 `notes` 原话是「等真撞上『类型位置不展开』的具体案例再修，不提前做」。
所以第一件事不是写代码，而是**把「还有没有差异」测出来**。全部实测在 0.60.0 的
真二进制上跑（`./target/debug/sokonanoda --json`，`--json` 事件流 + 退出码；判据
一律走内核，不做文本比对）：

### 1.1 类型位/项位展开：**已经透明**

```sokonanoda
def Set (α : Type) : Type := α -> Prop
def A1 (α : Type) (f : α -> Prop) : Set α := f     -- 期望 Set α、给出 α -> Prop
def A2 (α : Type) (A : Set α) : α -> Prop := A     -- 期望 α -> Prop、给出 Set α
```

实测：`decl.checked Set` / `decl.checked A1` / `decl.checked A2`，**0 条 diagnostic**。
即 `Set α` 与 `α -> Prop` 在 elaboration 里可以互换，**不需要** `show`/`change`。
这正是 Lean `abbrev` 的卖点，`def` 今天已经做到。

### 1.2 `#reduce` / `query reduce`：**展开**

```
#reduce Set      => fun (α : Type 0) => α -> Prop
#reduce Set.mem  => fun (α : Type 0) (a : α) (A : Set α) => A a
```

实测事件 `expr.reduced` 的 `text` 就是上面两条。`def` 是 delta-可展开的。

### 1.3 hover 显示：**保留别名名，不展开**

真 LSP（`sokonanoda-lsp` 走 stdio，`initialize` + `didOpen` + `textDocument/hover`）实测：

| 光标处 | hover 文本 |
|---|---|
| `Set.mem`（声明名） | `def Set.mem : forall (α : Type 0), α -> Set α -> Prop` |
| binder `A : Set α` | `A : Set α` |
| binder `f : α -> Prop` | `f : α -> Prop` |
| 标识符 `Set` | `Set : Type 0 -> Type 0` |

`#check` 同源（同一条内核 pp）：`#check Set.mem Nat` ⇒ `Nat -> Set Nat -> Prop`。
**别名名保留**——与 Lean 的 `abbrev` 一样（Lean 的 pp 也不会把 `Set α` 打成
`α → Prop`）。

### 1.4 `by` 块里 `rfl` 判等：**项层透明成立**

```sokonanoda
def Set.empty (α : Type) : Set α := fun (x : α) => False
theorem t2 (α : Type) : Eq.{1} (Set α) (Set.empty α) (fun (x : α) => False) := by rfl
```

实测 `decl.checked t2`：内核在 `rfl` 的 defeq 检查里展开了 `Set.empty`。

> **顺带测出的两条既有边界（**不是** abbrev 差异，改写 `abbrev` 也改不掉）**：
> ① 类型层判等 `Eq.{1} Type (Set α) (α -> Prop)` 在 `by rfl` 下**失败**——
> `rfl` 候选默认用宇宙层 0（`by.rs::rfl_candidate`），`Type` 需要层 1；把
> `Set` 换成任何别的东西（甚至两边写成同一个 `α -> Prop`）同样失败 ⇒ 这是
> **`rfl` 在 `Sort` 值上的既有边界**，与别名无关。② `rfl` 不做 η-展开，
> `Set.mem α` 与 `fun a A => Set.mem α a A` 不判等。

### 1.5 递归 / 非递归：**两边都不支持递归 `def`**

```
def loop (n : Nat) : Nat := loop n   =>  elab-unknown-identifier `loop`
```

本语言的 `def` **不支持递归**（递归要走 `inductive` + `match`/recursor），
所以 Lean 里 `abbrev` 与 `def` 在递归/编译产物上的差别在这里**不存在**。

### 1.6 唯一的真实差异：reducibility hint（在本语言**不可观察**）

Lean 的 `abbrev` = `@[reducible]` 的定义，`def` = semireducible：差别体现在
**默认透明度**（`simp`/`rw`/`rfl`/类型类推断在 `.reducible` 层展开谁）。
本语言**只有一个透明度层级**：

* `crates/front/src/compile/elab.rs` 里 `def` 的 `Declar::Definition` 一律带
  `ReducibilityHint::Regular(0)`（`build_def`），内核的 `conv` 在 defeq 检查里
  **无论 hint 都会展开**；
* 内核确实有 `ReducibilityHint::Abbrev`（`crates/kernel/src/env.rs`），但它只被
  `conv.rs::is_lt` 用来决定**先展开哪一边**（「hint 大的先展开」）——纯粹是
  **效率启发式**，不改变可判定性、不改变结果、不改变 pp；
* 本语言没有 `simp`/`rw`/`@[reducible]`/类型类/代码生成，所以没有任何消费者
  能观察到 hint。

**结论：`def` 与 Lean 的 `abbrev` 在本语言里没有可观察差异。** 按任务书的分支，
`abbrev` 实现成**与 `def` 同语义的关键字**。

## 2. as-built（实现）

| 层 | 文件 | 落点 |
|---|---|---|
| parser | `crates/front/src/parser.rs` | `parse_command` 新增一条 `TokenKind::Ident(kw) if kw == "abbrev" => self.parse_def()`；`is_reserved_command` 加 `"abbrev"` |
| AST / elab / 内核 | —— | **零改动**：`abbrev` 产出与 `def` **逐字段相同**的 `Command::Def`，后续流水线一字不改 |
| 白名单 | `crates/cli/src/help.rs` | 语言命令表加一行 `abbrev <name> : <type> := <value>`（注明 = `def`） |
| 白名单 | `docs/architecture.md` §4.1 | 命令清单加 `abbrev` |
| 词法高亮 | `crates/front/src/semantic.rs` + `editor/vscode/syntaxes/sokonanoda.tmLanguage.json` | **已同步（主线收尾轮，见 §4）**：`abbrev` 进 `front::semantic::KEYWORDS`，同一轮进 TM 语法的 `keywords` 词表（两份逐字相等由 `crates/cli/tests/extension.rs::tm_grammar_keywords_follow_the_single_source` 钉住）与 `declarations` 规则（`abbrev` 声明的名字照 `def` 着 `entity.name.function`）；LSP 补全吃同一份 `KEYWORDS`，因此自动跟上 |

`abbrev` 与 `def` 共享 `parse_def`，所以**自动**继承 `def` 的一切既有形状：
宇宙参数（`abbrev f {u} …`）、声明级 binder、`namespace` 前缀（`namespace A` 里
`abbrev Set` ⇒ `A.Set`）、`:= sorry` 开放练习、`#print`/`#check`/hover。

## 3. 与 Lean 的已知差异（明说，不假装对齐）

| # | 差异 | 依据 |
|---|---|---|
| 1 | **reducibility hint 不落地**：`abbrev` 与 `def` 在核心里都是 `Regular(0)` | §1.6（本语言只有一个透明度层级，hint 只影响 `conv::is_lt` 的展开顺序 ⇒ 不可观察）。不把它接进 `Declar::Definition` 是**刻意的**：接了也只是把同一个值换个标签，却要给 `Command::Def` 加字段、翻动所有匹配臂 |
| 2 | 必须写类型注解：`abbrev x := 1` 报 parse 错 | 与 `def` **共享**的既有边界（`parse_def` 的 `expect_colon`），不是 abbrev 特有 |
| 3 | 不支持递归 `abbrev` | 与 `def` 共享（§1.5） |
| 4 | `#print` 显示 `def Set …` 而不是 `abbrev Set …` | 与差异 1 同源：核心里只有一种 Definition，pp 不带 hint 字样 |
| 5 | ~~编辑器不把 `abbrev` 高亮成关键字~~ | **已销（主线收尾轮）**：§4 的同步项落地，`abbrev` 与 `def` 在编辑器里同样着色/补全 |

## 4. 同步项 —— **已销（主线收尾轮，as-built）**

> 原文（留给历史）：`front::semantic::KEYWORDS` + `editor/vscode` 的 TM 语法词表
> **同一轮**加 `abbrev`（两份必须逐字相等，`tm_grammar_keywords_follow_the_single_source`
> 是守护）；本轮 `editor/vscode/**` 与 `skills/**` 禁改，所以留给主线统一同步。
> VS Code 补全候选（如果主线愿意把 `abbrev` 放进 snippet）。

**as-built**（同一轮把 `notation-subset.md` §13.6 的记法拼写一起销账）：

| 落点 | 改动 |
|---|---|
| `crates/front/src/semantic.rs` | `KEYWORDS` 加 `abbrev`（紧跟 `def`）与记法拼写 `prefix`/`postfix`/`binder_notation`/`scoped`（与 `NOTATION_COMMANDS` 同集合）；单测 `semantic::tests::keyword_table_covers_abbrev_and_the_notation_spellings` 同时钉住「在词表里」与「着成 `Keyword`」 |
| `editor/vscode/syntaxes/sokonanoda.tmLanguage.json` | `keywords` 词表同一轮加这五个词；`declarations` 规则加 `abbrev`（声明的名字着 `entity.name.function.sokonanoda`，与 `def` 一致） |
| 补全 | 不需要额外改：LSP 的补全列表直接读 `front::semantic::keywords()`（`crates/lsp/src/lib.rs`），加词表即加补全；扩展没有自己的 snippet 表 |
| `editor/vscode/CHANGELOG.md` | `[Unreleased] / Added` 一条（**不动** `package.json` 的版本号——由主线统一 bump） |

**验证**：`cargo test -p sokonanoda-cli --test extension` ⇒ exit 0（
`tm_grammar_keywords_follow_the_single_source` 绿，两份词表逐字相等）；
`cargo test -p sokonanoda-front --lib semantic::tests` ⇒ exit 0。

## 5. 测试（TDD 三层）

1. **front 单测**（`crates/front/src/parser.rs`）：`abbrev` 与 `def` 产出**同一
   AST 形状**；`abbrev` 是保留命令（不能当声明名/变量名）；`abbrev` 与 `def`
   的**事件序列逐一相等**；类型位透明（`A1`/`A2` 两条都 `decl.checked`）；
   `#reduce` 展开；`namespace` 里加前缀。
2. **CLI e2e**（`crates/cli/tests/cli.rs`）：真二进制跑一份含 `abbrev` 的画布
   ⇒ exit 0、`decl.checked`、0 diagnostic；再把同一份画布的 `abbrev` 换成
   `def` ⇒ **五元事件计数逐一相等**；`abbrev` 当声明名 ⇒ parse 诊断 + exit 1。
3. **复现件**：`docs/gaps/repro/G08-abbrev.sokonanoda` 翻成**已修后形状**
   （补使用行：类型位透明 + `#reduce`），`scripts/gap.py check` 认 `status=fixed`
   ⇒ 必须 `clean`。

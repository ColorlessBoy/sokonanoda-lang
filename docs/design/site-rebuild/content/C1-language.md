# C1 — 语言能力事实档案（sokonanoda-lang 0.61.0）

> 用途：官网重建时描述"这门语言今天到底能做什么"的**事实底稿**。
> 纪律：每条事实带 `file:line`；代码逐字引用（不改格式）；计数来自实际命令并写明命令；
> 无法核实的一律写 **未证实**。
>
> 版本：仓库 `Cargo.toml:6` 为 `version = "0.61.0"`。
> 本文所有 JSON 事件样例都由 **0.61.0 真实二进制**跑出，命令与输出逐字抄录（见 §4.3）。

---

## 1. 一句话定位 + 30 秒模型

**一句话**：`sokonanoda-lang` 是一个独立自包含的 **Lean 4 教学编译器栈**（Rust 实现）——
底层是完整移植的 Lean 4 类型检查器（kernel），上层是受限的教学方言 `.sokonanoda`，
产品目标是"用户与 code agent 看着同一个 `.sokonanoda` 文件协作学证明"
（`docs/architecture.md:13-19`）。

**30 秒模型**（四句话）：

1. **一个 `.sokonanoda` 文件 = 一串声明**（`def` / `theorem` / `example` / `axiom` /
   `inductive … end`），声明之间用 `--` 中文注释讲课。文件是**纯声明式**的：
   `#check` / `#reduce` / `#print` / `#prove` 是 REPL 与自测的便利，
   **不属于教学文件格式**（`docs/protocol.md:15-20`）。
2. **没写出来的答案写成 `sorry`**：它是一个合法的"未完成"状态，不是错误——
   编译器为它发 `exercise.open` 事件（`docs/protocol.md:60`、`docs/architecture.md:559`）。
3. **判定永远由 kernel 说了算**：前端把源文件 elaborate 成内核声明，交给完整内核
   `try_check_declar` 终审；**禁止文本比对**（`REQUIREMENTS.md:25-26` 硬规则 8、
   `docs/architecture.md:527`）。
4. **反馈有两个人读的视图**：人读 `line:col: error[stage]: message`
   （`docs/protocol.md:37-44`），机器读 `--json` 的 JSON Lines 事件流
   （`docs/protocol.md:48-62`）；编辑器侧同一份真相走 LSP
   （诊断 / hover / Infoview / 目标视图，`docs/protocol.md:15-33`）。

**流水线**（`docs/architecture.md:21-35`，逐字形状）：

```text
 .sokonanoda 源码
   │  parse（front）
   ▼
 AST(FolFile/Command/Expr)     带 span
   │  elab（front → builder）
   ▼
 kernel Declar 序列（arena 内，按依赖顺序）
   │  EnvBuilder.finish()
   ▼
 ExportFile（完整环境 + 名字/层级/表达式 intern 表）
   │  try_check_declar / infer_closed_type / reduce_closed / with_pp
   ▼
 CheckEvent[] + CompileError[]      ← 文本行（人）/ JSON Lines（agent）
```

**为什么"填完的声明放进官方 Lean 也合法"**：教学语法是**真实 Lean 4 的子集**，
语法白名单即课程；新增语法必须同时带课程、测试与白名单三件套
（`REQUIREMENTS.md:19-21` 硬规则 3–4）。

---

## 2. 语法清单（今天真正接受的每一种形式）

### 2.1 声明：`def` / `abbrev` / `theorem` / `example` / `axiom`

`def`、`theorem` 形如 `名字 : 类型 := 值`；`example` 是不取名的匿名声明
（`playground.sokonanoda:140` 的注释原文：「example 是不取名的匿名声明，
类型和值之间用 `:=` 隔开」）。

```sokonanoda
def Not : Prop -> Prop := fun (a : Prop) => a -> False
```
—— `playground.sokonanoda:128`（`def`）

```sokonanoda
theorem prop_id: (a : Prop) -> a -> a :=
  fun (a : Prop) => fun (h : a) => h
```
—— `playground.sokonanoda:175-176`（`theorem`；注意名字后冒号前没有空格，这是仓库里的实际写法）

```sokonanoda
example : Or True False := Or.inl True False True.intro
```
—— `playground.sokonanoda:141`（`example`）

`axiom` 只有名字与类型，没有 `:= 值`：

```sokonanoda
axiom Prop : Type 0
```
—— `playground.sokonanoda:84`；`playground.sokonanoda:38-40` 的课里写明格式为
`axiom 名字 : 类型`，且「公理的类型本身必须落在某个 `Sort` 上」。

`abbrev` 是 `def` 的**同语义拼写**（共用 `parse_def`，设计 `docs/design/abbrev.md`，
`docs/architecture.md:108`）：

```sokonanoda
abbrev Set (α : Type) : Type := α -> Prop
```
—— `docs/gaps/repro/G08-abbrev.sokonanoda:16`

**不支持**：`opaque`、`instance`、`class`、`structure`、`mutual`（`docs/architecture.md:519`
列出未做项；`Declar::{Opaque,Quot}` 只存在于内核层，教学前端不暴露，`docs/architecture.md:342`）。
`example` 不能带名字（匿名）。

### 2.2 归纳块：`inductive` / `ctor` / `rec` / `iota` / `end`

归纳类型是一整个块，`inductive` 开头、`end` 收尾（`docs/architecture.md:108`）：

```sokonanoda
inductive Color : Type
ctor red : Color
ctor green : Color
rec Color.rec {u} :
  (motive : (c : Color) -> Sort u) ->
  (mr : motive red) ->
  (mg : motive green) ->
  (c : Color) -> motive c
iota red :=
  fun (motive : (c : Color) -> Sort u) =>
  fun (mr : motive red) =>
  fun (mg : motive green) => mr
iota green :=
  fun (motive : (c : Color) -> Sort u) =>
  fun (mr : motive red) =>
  fun (mg : motive green) => mg
end
```
—— `course/unit6-induction-recursion-1.sokonanoda:86-102`（逐字，含换行）

- **`rec` 可以省略**：0.58.0 起前端按内核期望形状自动派生递归子（参数在最外层），
  旧的 `elab-missing-inductive-rec` 已不存在（`docs/architecture.md:512`）。
- **参数化归纳**（非带索引）：`inductive Option (A : Type) : Type`，参数与 ctor 字段
  都吃多名字 binder 组 `(A B : Prop)`（`docs/architecture.md:206-209`）：

```sokonanoda
inductive Option (A : Type) : Type
ctor none : Option A
ctor some (a : A) : Option A
```
—— `course/unit7-induction-recursion-2.sokonanoda:43-45`

- **带索引归纳**（0.47.0）：索引 = `ty` 在 params 之外的 Pi 望远镜；
  「索引 + 字段写在结果箭头链里」自 0.56.0 起也能自动派生（`docs/architecture.md:214-221`）：

```sokonanoda
inductive Vec (A : Type) : Nat -> Type
ctor vnil : Vec A zero
ctor vcons (a : A) (n : Nat) (v : Vec A n) : Vec A (succ n)
```
—— `course/unit7-induction-recursion-2.sokonanoda:121-123`

- **构造子命名空间**（0.59.0，G-02）：构造子的规范名是 `Ind.ctor`，裸名只是解析别名
  （子集扩展）；两个类型各声明一次同名裸构造子 ⇒ `elab-ambiguous-ctor-alias`
  （`docs/protocol.md:146-148`）。

**不支持**：带索引归纳与宇宙多态参数（`{u}` 级参数）仍不支持（`docs/architecture.md:221`）；
`mutual` 块、嵌套归纳、`Quot` 的教学语法均无。

### 2.3 声明级 binder（官方 Lean 风格）

`theorem f (a : A) (h : B a) : C := v` 在 parser 里降级为
`ty = Forall{binders → C}`、`val = Lambda{binders → v}`；`by` 引擎把声明 binder
作为初始上下文，`:= sorry` 的剩余目标直接是 `C`（`docs/architecture.md:222`）。

```sokonanoda
theorem forall_elim (P : Person -> Prop) (w : Person) (h : forall (x : Person), P x) : P w := h w
```
—— `playground.sokonanoda:271`

多名字 binder 组 `(A B : Prop)` 在归纳声明里也吃（`docs/architecture.md:208-209`）：

```sokonanoda
inductive Or (A B : Prop) : Prop
ctor inl (a : A) : Or A B
ctor inr (b : B) : Or A B
```
—— `course/unit11-modules-projects.sokonanoda:22-24`

**不支持**：`(A)` 这种无类型组仍是显式报错（`docs/architecture.md:209`）；
`{x : A}` 隐式 binder **只是打印样式**——`BinderStyle`（default/implicit/strictImplicit/
instImplicit）只影响打印，不影响类型检查，且**没有隐式实参自动插入**
（`docs/architecture.md:339`、`docs/architecture.md:445-446`）。

### 2.4 类型语法：箭头、`forall`、`Sort` / `Type n`

- `(x : A) -> B` = 带 binder 的 `forall`；`{x : A} -> B` = 隐式 binder 的 forall；
  `A -> B -> C` = 匿名 binder 右结合 Pi（`docs/architecture.md:223`）。
- Pi（`A -> B` / `forall`）的 binder **必须带显式类型**；lambda 的 binder 在
  **有期望望远镜**或**应用位置**（从实参类型，0.45.0）时可省略
  （`docs/architecture.md:223`）。
- 三种 `forall` 拼法等价，`playground.sokonanoda:239-242` 逐字列出：
  `(x : Person) -> P x` / `forall (x : Person), P x` / `∀ (x : Person), P x`。
- `Type n` 是 `Sort (n+1)` 的糖（**只加解析糖**，不碰 elaborator 与内核，
  `docs/design/type-level-syntax.md:11-33`）：`Type 0` = `Sort 1`、`Type 1` = `Sort 2`、
  单独的 `Type` = `Sort 1`。仓库里的真实用法：

```sokonanoda
axiom Person : Type 0
```
—— `playground.sokonanoda:252`

- **层级算术 `u+1`**（0.61.0 落地，唯一的真语法增量）：位置有 `Sort (u+1)` /
  `Sort u+1` / `Type (u+1)` / 宇宙实参 `Eq.{u+1}`（`docs/design/type-level-syntax.md:59-74`）：

```sokonanoda
def poly_mp {u} (α β : Sort u) (h : @Eq.{u+1} (Sort u) α β) (a : α) : β :=
  Eq.mp.{u} α β h a
```
—— `docs/gaps/repro/L03-eq-type-level.sokonanoda:62-63`

**不支持**：`Type u`（裸宇宙变量）**不支持**——`Type` 后跟裸标识符必须保持**应用**语义，
需要时写 `Sort u`（`docs/design/type-level-syntax.md:28-31`、`:83-87`）；
`u+v` / `max u v` / `imax` 不在层级语法面内（`+` 右边只收数字），
parser 报专用诊断（`docs/design/type-level-syntax.md:78-82`）。

### 2.5 值位关键字：`let` / `match` / `by`

值位只有三种关键字（`docs/architecture.md:179-186`）。历史值位关键字 `funapply`（0.22.0 移除）
与 `funintro`（0.27.0 移除）**已删除**，`funintro` 现在只是一个普通标识符并报未知名
（`docs/protocol.md:259-265`）。

#### `let`（Phase 1）

`let x : T := v; body`；**v1 要求显式类型注解**，缺注解 / `let x := v` 报
`elab-untyped-binder`，message/hint 定制为 `let` 写法。`let` 与
`(fun (x : T) => body) v` 内核等价（zeta）（`docs/architecture.md:182-186`）。

```sokonanoda
def add_two_let : Nat -> Nat := fun (n : Nat) => let m : Nat := n + 2; m
```
—— `course/unit3-functions-arrows.sokonanoda:68`

**不支持**：无注解 `let`（仍是 Phase 2，`docs/architecture.md:186`）。

#### `match`（Phase 2）

语法（`docs/architecture.md:187-189`）：

```text
match e with | <pattern> [if <guard>] => body | ...
```

模式支持：通配 `_`、绑定变量、**嵌套构造子**（`some (succ k)`）、**Nat 字面量**
（`0`/`1`/… 脱糖为 `succ^k zero`）与 **Bool 守卫**（`if cond`，假则落到后续 arm）；
arm **有序、首个匹配者胜**，同一构造子可写多条（`docs/architecture.md:190-192`）。
被匹配项是**源内 `inductive`**（分支用裸构造子名）**或 prelude 内建 `Nat`**
（分支用点号名 `Nat.zero`/`Nat.succ`；`Bool` 同理 `Bool.true`/`Bool.false`）
（`docs/architecture.md:193-195`）。

**简单分情况**（`course/unit6-induction-recursion-1.sokonanoda:105-107`）：

```sokonanoda
def swap (c : Color) : Color := match c with
| red => green
| green => red
```

**嵌套模式 + 通配兜底**（`course/unit7-induction-recursion-2.sokonanoda:95-98`）：

```sokonanoda
def isZeroOpt (x : Option Nat) : Nat := match x with
| some zero => succ zero
| some _ => zero
| none => zero
```

**递归归纳的归纳假设**：递归构造子字段后自动插入归纳假设 binder（`ih`、`ih2`…，
类型为结果类型 R），branch 直接引用，递归无需自引用（`docs/architecture.md:198-200`）：

```sokonanoda
def addM (a b : Nat) : Nat := match a with
| zero => b
| succ m => succ ih
```
—— `course/unit6-induction-recursion-1.sokonanoda:135-137`

**Nat 字面量模式**（脱糖为 `succ^k zero`，与 `Nat.succ k` 混排；真实测试源）：

```sokonanoda
def is_zero (n : Nat) : Bool := match n with
| 0 => Bool.true
| _ => Bool.false
```
—— `crates/cli/tests/cli.rs:1667-1669`（`cli_match_nat_literals_check_via_kernel`）

**Bool 守卫**（守卫假则落到下一条 arm；真实测试源）：

```sokonanoda
def pick (a b : Bool) : Bool := match a with
| Bool.true if b => Bool.false
| _ => a
```
—— `crates/cli/tests/cli.rs:1686-1688`（`cli_match_guard_falls_through_to_the_next_arm`）

**依赖 motive**：当结果类型 `R` 含被匹配的**裸局部变量** `x`（如 `P n`）时，
motive = `fun t => R[x:=t]`，分支期望 = `R[x:=<构造子项>]`、IH 类型 =
`R[x:=<递归字段>]`（`docs/architecture.md:200-204`）：

```sokonanoda
theorem nat_induction (P : Nat -> Prop) (hz : P zero)
    (hs : (k : Nat) -> P k -> P (succ k)) (n : Nat) : P n :=
  match n with
  | zero => hz
  | succ k => hs k ih
```
—— `crates/cli/tests/cli.rs:1810-1814`（`cli_match_dependent_motive_checks_via_kernel`）

降低为显式 `<Ind>.rec.{level} motive minor... scrutinee`（level 由结果类型的 Sort
推出，显式宇宙实例是内核接受的必要条件）（`docs/architecture.md:196-198`）。
`match` 也是 tactic 白名单里的一员，语义等价于 `exact (match … with …)`
（`crates/front/src/parser.rs:1277-1286`）。

**不支持**：元组/记录语法（本语言无）、字符串字面量模式
（`docs/design/match-patterns.md:27`）；参数化归纳的 `match` 只支持"被匹配项是一个
局部变量，且它的类型写成 `T 参数…`"，否则 `elab-match-parameterized-unsupported`
（`docs/protocol.md:160-161`）；结果类型依赖索引不在 v1（`docs/architecture.md:218`）；
嵌套/守卫下的依赖 motive 子目标类型退回常量（保守，`docs/design/match-patterns.md:175`）；
守卫限 prelude `Bool`（`docs/design/match-patterns.md:175`）。

#### `by`（tactic 块）

值位 `by <tactic 序列>`，tactic 之间用 `;` **或换行**分隔（0.51.0）
（`docs/architecture.md:179-181`）；进内核前由 `crates/front/src/by.rs` 降级为 lambda。
完整 tactic 清单见 §3。

### 2.6 `namespace` / `open` / `export` / `scoped`（0.60.0 + 第二/三刀）

`namespace A` … `end A` 之间的**声明名**自动带前缀（`def mem` ⇒ 全局名 `A.mem`；
名字本身带点则拼接 `A.Set.mem`），前缀在 **parser** 里落定
（`docs/architecture.md:147-152`）。三条命令**都不是声明**（零事件、不进声明表，
与 `import`/记法同族）（`docs/architecture.md:154-155`）。

```sokonanoda
namespace Foo
def x : Type := Prop
end Foo

-- ③ `open` 的前缀：`x` 解析到 `Foo.x`（`open` 只影响解析，不重命名任何东西）。
open Foo
def use_short : Type := x
```
—— `docs/gaps/repro/G05-namespace-open.sokonanoda:25-31`

**解析顺序**（N4）：① 当前命名空间链从内到外（`A.B.x` → `A.x`）→ ② 精确 `x` →
③ `open` 的前缀（按 open 顺序），第一个在 `known` 里能解析的胜出
（`docs/architecture.md:152-154`、`docs/gaps/repro/G05-namespace-open.sokonanoda:20-23`）。

**`open` 的三条子句 / 局部 open / `export`**（第二刀，`docs/architecture.md:164-175`）：

```sokonanoda
open Foo (x)
def use_only : Type := x

open Foo renaming x => short
def use_renamed : Type := short

open Foo in def use_local : Type := x

export Foo
def use_exported : Type := x
```
—— `docs/gaps/repro/G05-namespace-open.sokonanoda:55-64`

- `open A (a b)`（only）、`open A hiding a b`、`open A renaming a => b` 三条**互斥**，
  先过滤后改名；原短名被改名占位 ⇒ 不再是候选（`docs/architecture.md:164-166`）。
- `open A … in <命令>` 只对紧跟的那一条**叶子命令**生效，`Walk` 用 mark/rollback 撤销
  （`docs/architecture.md:166-170`）。
- `export A [<子句>]` 文件内与 `open` **逐字相同**，额外记进 `Walk::exports`，
  单元切换时重放 ⇒ **跨 `import`**（`open` 不跨，N5）（`docs/architecture.md:170-172`）。
- `scoped <记法命令>` + `open scoped <作用域名>`：作用域名 = 声明点所在 namespace 的
  全前缀；`open scoped` **只**开记法不开名字（`docs/architecture.md:141-142`）。
  实测（0.61.0）：`scoped` 未 `open scoped` 时报 `notation-unknown-symbol` + exit 1，
  `open scoped Foo` 之后与普通记法一样判卷
  （`crates/cli/tests/notation.rs:685-721`）。

**不支持**：`open scoped` 的子命名空间传播；编辑器词表同步（`docs/architecture.md:145-146`）。

### 2.7 用户自定义记法：`infix:N` / `infixl:N` / `infixr:N` / `prefix:N` / `postfix:N` / `notation` / `binder_notation` / `scoped`

记法是**源级糖，不是新语义**：它只把源文本重写成既有的 `App` 形状，**不产生任何事件、
不进声明表、不进 goal 视图**；点名形式永久可用且两种写法判卷一致
（`docs/architecture.md:89-95`、`docs/gaps/repro/G04-notation.sokonanoda:21-22`）。

```sokonanoda
infix:50 " ∈ " => mem
infixl:65 " ∪ " => union
prefix:100 " 𝒫 " => powerset
postfix:100 " ᶜ " => compl
infixr:80 " '' " => image
```
—— `docs/gaps/repro/G04-notation.sokonanoda:37-41`（逐字，含每行的真实目标名）

零元记法不带优先级：

```sokonanoda
notation "∅" => Set.empty
```
—— `courses/set-theory/units/notation-cheatsheet.sokonanoda:89`

课程库里的五个集合论专用符号（在 `lib/Set.sokonanoda` 末尾声明、**随 `import` 传播**）：

```sokonanoda
prefix:100 " 𝒫 " => Set.powerset
postfix:100 " ᶜ " => Set.compl
infixr:80 " '' " => Set.image
infixr:80 " ⁻¹' " => Set.preimage
infixr:80 " ×ˢ " => Set.prod
```
—— `courses/set-theory/lib/Set.sokonanoda:155-159`

**binder 记法**（第三刀）：`∃ (x : α), p` ⇒ `Exists α (fun (x : α) => p)`；
两段式 `∃ x ∈ s, p` ⇒ `Exists α (fun (x : α) => And (x ∈ s) p)`，
`∀ x ∈ s, p` ⇒ `∀ x, x ∈ s -> p`（`docs/architecture.md:134-138`）：

```sokonanoda
binder_notation "∃" => Exists
```
—— `courses/set-theory/units/unit08-images-preimages.sokonanoda:141`

用法（同文件 `:144-146`）：

```sokonanoda
example (α β : Type) (f : α -> β) (A : Set α) (y : β) :
    Prop :=
  ∃ (x : α), And (Set.mem α x A) (Eq.{1} β (f x) y)
```

**规则摘要**（N1–N7，`docs/architecture.md:110-124`）：符号**必须是独立 token**
（声明里的文本去掉首尾空白后不能全是标识符字符）；优先级 `N ∈ 1..=1000`；
`infix:N` 左右是 `N`/`N+1`、`infixl:N` 是 `N`/`N+1`（同级左结合）、`infixr:N` 是
`N+1`/`N`；**文件内作用域**（声明之后、同文件生效，不跨 `import`，但记法表本身
随 import 继承）；展开时**自己补前导类型参数**（只做裸变量匹配，不引入元变量/一般合一）。

**不支持**：源码级 print-back（goal/hover 显示的是**点名形式**——类型文本由冻结内核的
pp 产出，记法不进内核；`docs/design/notation-subset.md:550`、
`courses/set-theory/units/notation-cheatsheet.sokonanoda:100-101` 有实测 `#check`）；
`notation3` / 依赖 binder；一般隐式实参推断；**后缀记法实参位免括号明确不做**
（`f Aᶜ` 今天读作 `(f A)ᶜ`，改了会悄悄重分组，`docs/architecture.md:143-144`）；
一元记法在实参位要加括号（`f (𝒫 A)`、`f (Aᶜ)`，
`courses/set-theory/units/notation-cheatsheet.sokonanoda:117-119`）。

### 2.8 集合字面量 `{a}` / `{a, b}`

**新语法**（不是记法）：`Expr::SetLiteral`，展开成点名形式 `Set.singleton α a` /
`Set.pair α a b`（目标名写死在 elab 里，与 `+` → `Nat.add` 同族）
（`docs/design/notation-subset.md:615-619`）。消歧：`{` 后面是 binder 形状
（`{x : T}` / `{x y : T}`）就**不是**字面量；否则是字面量，因此 `f {a}` 合法
（`docs/design/notation-subset.md:620-623`）。**1–2 个元素**；空 `{}` 与 ≥3 元素给
专用 parse 码 `set-literal-shape`；目标不存在（没 import 卷 I 的 `lib/Set`）⇒
`elab-set-literal-unknown-target`（`docs/design/notation-subset.md:624-627`）。

```sokonanoda
def one (α : Type) (a : α) : Set α := {a}
def two (α : Type) (a b : α) : Set α := {a, b}
```
—— `crates/cli/tests/notation.rs:637-638`（`set_literals_grade_like_the_pointful_singleton_and_pair`
里 `literals` 一侧的源文本；该测试断言它与 `Set.singleton α a` / `Set.pair α a b`
的五路计数**逐数相同**）

**诚实注记**：本仓库的 `.sokonanoda` 课程材料**尚未使用**集合字面量——
`courses/set-theory/` 里 `{a}`/`{a,b}` 只出现在注释与 hint 文本中（例如
`courses/set-theory/units/unit02-subsets-empty.sokonanoda:86`），代码里写的是点名形式
`Set.singleton` / `Set.pair`（`courses/set-theory/units/solutions/unit02-solution.sokonanoda:45`）。
所以上面这条引的是**测试源**，不是课程源。

### 2.9 `import` + `sokonanoda.toml`

文件**没有** `import` 时走单文件路径，而且**从不发现/读取 `sokonanoda.toml`**——
单文件就该像脚本一样直接跑（`docs/architecture.md:271-273`）。第一行出现
`import Foo.Bar` 后，编译单元升级为「入口 + import 闭包」（`docs/architecture.md:274-277`）。

```sokonanoda
import lib.Logic
import lib.Set
```
—— `courses/set-theory/units/unit02-subsets-empty.sokonanoda:10-11`

清单文件 `courses/set-theory/sokonanoda.toml`（逐字全文）：

```toml
# 卷 I《集合论》—— 模块根 = 本目录（与将来的独立课程仓同构）。
# 语言仓测试/脚本一律用**绝对路径**判卷（台账 G-12：相对路径 + 祖先清单会让模块根退化）。
name = "set-theory"
requires = "0.61"
```

**定位规则**：`--root` 显式指定 > 最近的 `sokonanoda.toml`（向上走到 `.git`/HOME 即停）>
入口文件所在目录——**无清单也能用 import**，这是与真 Lean 的有意分歧
（`docs/architecture.md:279-281`）。模块名 = 相对模块根的路径
（`Foo/Bar.sokonanoda` → `Foo.Bar`，`-` 不是模块名字符，`docs/architecture.md:283-284`）。

**不支持**：`import` 必须出现在所有声明之前（否则 `import-must-precede-declarations`）；
模块名里的 `-` 不合法（`import-not-a-valid-module-name`，经典情形是文件名里的连字符，
`docs/protocol.md:117-121`）；`import` 无 `open` 语义（要用被导入模块的短名得自己
`open`/`export`，`docs/architecture.md:155-159`）。

### 2.10 `#` 命令：存在，但不属于教学文件格式

`#check` / `#reduce` / `#print` 与 REPL 的 `#prove` / `#env` / `#help` 都是
REPL 与自测便利，**不是教学文件格式的一部分**——编辑器里对应的能力是
hover（类型）、命令/内联动作（化简）、目标视图（`#prove`）、诊断
（`docs/protocol.md:15-33`）。但 parser 确实认它们，课程源里也在用：

```sokonanoda
#check (Type 0)
#check (Type 1)
```
—— `course/unit5-universes-sort.sokonanoda:31-32`

`#check` 只看到它**之前**的声明（`EnvLimit::ByIndex(decl_before)`，
`docs/architecture.md:234`）。REPL 另有 `#prove`（证明草稿：`intro`/`exact`/`apply`/
`assumption`/`lambda`/`done`，`docs/architecture.md:262`、`:452-463`）。

---

## 3. tactic / 证明语言

### 3.1 `by` 块的白名单：**七个**关键字，一个不多

> **测量陷阱警告（2026-09-19，一次真实误判的产物）**
>
> 本节曾被我改成「十三个」，那是**错的**，已改回。原因值得记住：
> 本工作区 `crates/front/src/{parser,ast,by}.rs` 有 **+862 行未提交的 WIP**
> （`by` 块新增 `constructor`/`cases`/`left`/`right`/`use`/`exfalso`），而
> `scripts/soko` 的解析顺序里「仓库构建」优先，`target/debug/sokonanoda` 正是
> 用这棵修改过的树编出来的（它的 marker 甚至写着 `0.55.0`，`--version` 却报
> `0.61.0`）。于是 `scripts/soko` **量的是未发布代码**。
>
> **要量已发布的事实，必须钉到发布产物**：
> `SOKONANODA_BIN=~/.vscode/extensions/sokonanoda-lang.sokonanoda-<版本>-<平台>/bin/<平台>/sokonanoda`，
> 或者先确认 `git status --short crates/` 是干净的。站点写的是**已发布版本**的事实。

`by` 块的 tactic 白名单由 `crates/front/src/parser.rs` 的
`is_tactic_keyword` 与 `parse_tactic_inner` 的 `match` 臂**共用一个集合**：
`intro` / `exact` / `apply` / `assumption` / `rfl` / `match` / `sorry`
（`crates/front/src/parser.rs:2844-2849`；`crates/front/src/ast.rs:293-317` 的
`enum Tactic` 有 6 个变体——`match` 被解析成 `Tactic::Exact`，
见 `crates/front/src/parser.rs:1277-1286`）。

白名单外的写法给一条**点名全部合法 tactic**的 parse 错：
「未知 tactic：`by` 块只支持 intro / exact / apply / assumption / rfl / match / sorry（白名单）」
（`crates/front/src/parser.rs:1295-1298`）。tactic 之间用 `;` **或换行**分隔（0.51.0，
`docs/architecture.md:179-181`）。

`by` 块不是第二套语义：进内核前由 `crates/front/src/by.rs` **降级为 lambda**
（`docs/architecture.md:179-181`），tactic 判定走 `front::judge`（合成完整声明交完整
kernel 裁决），**编辑器路径里没有任何文本比对**
（`docs/architecture.md:527`、`docs/protocol.md:554-556`）。

### 3.2 七个 tactic 逐个 + 真实例子

| tactic | 语义（源码/文档） | 真实例子（`file:line`） |
|---|---|---|
| `intro x` | 目标剥一层箭头，引入假设 `x`（等价于写一层 `fun`） | `course/unit4-by-tactics.sokonanoda:13`（课里原文）；用法 `course/unit4-by-tactics.sokonanoda:37` |
| `exact e` | 用项 `e` 直接结束当前目标（内核判定类型匹配） | `playground.sokonanoda:280` |
| `apply f` | 目标套上 `f`：若 `f : A1 -> … -> An -> B` 且目标就是 `B`，把目标换成 `A1..An` 几个子目标 | `playground.sokonanoda:278` |
| `assumption` | 从已引入的假设里找类型与目标一致的，直接用 | `course/unit4-by-tactics.sokonanoda:37` |
| `rfl` | 目标是 `Eq α x y` 且两边算出来一样时直接成立 | `course/unit4-by-tactics.sokonanoda:46` |
| `match` | 臂体是**项**（同值位 match），语义等价于 `exact (match … with …)`；`by` 引擎以当前目标为期望类型判定 | `crates/cli/tests/cli.rs:221` |
| `sorry` | 占位——当前目标保持开放（合法 Open 状态），与声明值位的 `sorry` 同语义 | `course/unit4-by-tactics.sokonanoda:54` |

逐字代码（每条都在仓库里可跑）：

```sokonanoda
theorem demo_by_assumption : (a : Prop) -> a -> a := by intro a; intro h; assumption
```
—— `course/unit4-by-tactics.sokonanoda:37`（`intro` ×2 + `assumption`）

```sokonanoda
theorem demo_by_apply : Or True False := by apply Or.inl; exact True.intro
```
—— `course/unit4-by-tactics.sokonanoda:42`（`apply` + `exact`）

```sokonanoda
theorem demo_by_rfl : Eq.{1} Nat (1 + 1) 2 := by rfl
```
—— `course/unit4-by-tactics.sokonanoda:46`（`rfl`；课里注明「内核会把 1 + 1 算成 2」）

```sokonanoda
def swap (c : Color) : Color := by match c with | red => green | green => red
```
—— `crates/cli/tests/cli.rs:221`（`match` 作为 tactic，测试 `cli_by_match_tactic_checks_via_kernel`）

```sokonanoda
theorem by_ex1 : (a : Prop) -> a -> a := by sorry
```
—— `course/unit4-by-tactics.sokonanoda:54`（`by sorry` = 留空未完成，`course/unit4-by-tactics.sokonanoda:19` 原文）

`apply` 出多个子目标时的写法（每个子目标按顺序收尾）：

```sokonanoda
theorem ai : (a : Prop) -> (b : Prop) -> a -> b -> And a b := by 
intro a; intro b; intro ha; intro hb; apply And.intro; exact ha; exact hb
```
—— `crates/cli/tests/cli.rs:120-121`（注意第一行末尾 `by ` 后有一个尾随空格，逐字如此）

**多行 `by` 块**（换行分隔，`playground.sokonanoda:277-282`）：

```sokonanoda
theorem forall_and (P : Person -> Prop) (Q : Person -> Prop) (h : forall (x : Person), And (P x) (Q x)) : And (forall (x : Person), P x) (forall (x : Person), Q x) := by
  apply And.intro
  intro x
  exact And.left (P x) (Q x) (h x)
  intro x
  exact And.right (P x) (Q x) (h x)
```

### 3.3 项级证明构造（不用 tactic 的写法）

除 tactic 外，证明就是普通项——仓库课程的主线正是先教"证明 = 构造项"：

- **lambda**：`fun (h : a) => h`（`playground.sokonanoda:176`）；
- **应用**：`h w`（`playground.sokonanoda:271`）；
- **构造子项**：`And.intro b a (And.right a b x) (And.left a b x)`
  （`playground.sokonanoda:207`）；
- **消去子/递归子**：`False.rec`（`playground.sokonanoda:214`）、
  `Nat.rec.{u} motive mz ms n`（`course/solutions/unit6-induction-recursion-1-solution.sokonanoda:25`）；
- **`match` 作为项**（见 §2.5）；
- **等式**：`Eq.refl.{u} α a`、`Eq.subst.{u} α (fun (x : α) => …) a b h …`
  （`course/solutions/unit5-universes-sort-solution.sokonanoda:21`）；
- **`let`**：`let m : Nat := n + 2; m`（`course/unit3-functions-arrows.sokonanoda:68`）。

### 3.4 编辑器侧的"下一步建议"（不是 tactic，但构成证明语言的一部分）

LSP code actions 由 kernel 判定后给出，每个请求最多三条，第一条带 `is_preferred: true`
（`docs/protocol.md:364-376`）：`exact <hypothesis>`（内核判过与洞的期望类型 defeq）、
`Eq.refl …`（`Eq` 形目标，内核验证过才给出）、`refine <skeleton>`（从声明自身的
axiom/ctor 形状恢复的构造子骨架，如 `And.intro a b sorry sorry`）、`intro`（把下一层
binder 剥成 lambda 前缀）。另有一类针对**失败声明**的 `Restart` 建议
（`fun (x : A) => ???` 骨架，`docs/design/kernel-taxonomy.md:44-54`）。

### 3.5 明确没有的

- **没有 `simp` / `rw` / `constructor` / `cases` / `induction` / `omega` / `norm_num` /
  `exact?`** 等——白名单只有 §3.1 那七个（`crates/front/src/parser.rs:2844-2849`）。
  ⚠️ 工作区里有一份**未提交**的 WIP 正在给 `by` 块加 `constructor`/`cases`/`left`/
  `right`/`use`/`exfalso`；**站点写已发布的 0.61.0**，所以这里按七个写。
- **没有 `section` / `variable`**（`docs/architecture.md:519` 列出未做项）。
- **`#prove` 的 `lambda` / `done` 只存在于 REPL 草稿**，不是文件格式的一部分
  （`docs/architecture.md:262`、`docs/protocol.md:15-20`）。
- **`apply` 类 tactic 的缺项报错 hint 文案仍写着已删除的 `funapply`**
  （`crates/front/src/compile/error.rs:261`、`:264`）——`funapply` 早在 0.22.0 就移除了
  （`docs/protocol.md:259-265`）。这是**文案滞后**，不是语言能力；引用时不要当成现行语法。

---

## 4. 内核判定与诊断

### 4.1 判定怎么发生

1. **先 elaborate，后内核**：`compile_fol` 顺序处理每条命令，把 AST elaborate 成内核
   `Declar` 并 `builder.add_declar` 入表（`docs/architecture.md:232-237`）。
2. **elaboration 阶段已有错误就直接返回，不碰内核**（`docs/architecture.md:237`）。
3. 否则 `builder.finish()` 得到 `ExportFile`，设 `pp_options.proofs = true`
   （打印证明项本体而不是 `_`），然后逐个执行 `PendingOp`：声明走
   `env.try_check_declar(&declar)`，成功产 `decl.checked` / `example.checked`
   （`docs/architecture.md:238-242`）。
4. **内核拒绝 = panic → Result**：内核仍用 `assert!` panic 报拒绝（如 `def_eq failed`），
   `try_check_declar` 用 `catch_unwind` 包装成 `CheckError::Rejected/Internal`；
   因此**内核交互必须包 catch_unwind**，否则会崩掉编译/LSP
   （`docs/architecture.md:504-506`、`:518`）。
5. **可见前缀受控**：`EnvLimit::{Empty, ByIndex, ByName, PpUnlimited}` 用 cutoff 控制
   "可见前缀"；`#check` 只看到它之前的声明（`docs/architecture.md:234`、`:342`）。
6. **prelude 是受信任预置**：内置 `Nat`/`Bool`/`Eq`/L1 逻辑骨架
   **从不被 `try_check_declar` 重查**（不进 PendingOp，`docs/architecture.md:386`）。
7. **签名也受检**（0.59.0，G-01）：值位是 `sorry` **不免检签名**——签名 elaborate 不了、
   不是类型、或 `theorem` 的不是 Prop ⇒ 一条 diagnostic + 声明 Failed + **不发**
   `exercise.open`（`docs/protocol.md:64-70`、`REQUIREMENTS.md` 硬规则 3 补记）。
8. **内核拒绝给两半**：conv 失败的 def_eq 消息带稳定格式
   `def_eq mismatch expected: <E> | actual: <A>`，front 解析后填 `CompileError.expected/actual`
   （`docs/architecture.md:480`、`docs/protocol.md:244-250`）。

### 4.2 事件流形状

`--json` 每条事件一行 JSON（JSON Lines）。**每个事件都带 `type` 与 `human`**；
payload 字段是**只增不改**的（`docs/protocol.md:48-62`）。

| `type` | payload | 何时发 |
|---|---|---|
| `decl.checked` | `name` | 声明通过完整内核（`docs/protocol.md:55`） |
| `example.checked` | — | 填好值的 `example` 通过内核（`docs/protocol.md:56`） |
| `expr.typed` | `text`（源切片）、`inferred_type`、`span` | `#check`（`docs/protocol.md:57`） |
| `expr.reduced` | `text`、`value`、`span` | `#reduce`（`docs/protocol.md:58`） |
| `decl.printed` | `name`、`text` | `#print`（`docs/protocol.md:59`） |
| `exercise.open` | `name`（可选） | 签名 elaborate 且通过内核类型测试、值位是 `sorry`（`docs/protocol.md:60`） |
| `diagnostic` | `stage`、`code`、`message`、`span`（+ `hint`） | 任何错误（`docs/protocol.md:61`、`:217`） |
| `warning` | `code`、`message`、`hint`、`span` | 非致命 lint，**永不影响退出码**（`docs/protocol.md:62`、`:84-85`） |

人读视图（同一份真相的另一渲染，`docs/protocol.md:37-44`）：

```text
checked declaration <name>
checked example
<expr>: <type>
<expr> => <value>
exercise open (fill the sorry)
<line>:<col>: error[<stage>]: <message>
```

`<stage>` 是**流水线阶段**（`parse`/`import`/`elab`/`kernel`），不是 code；
**稳定 code 与教学 hint 只在 `--json` 里**（`docs/protocol.md:214-219`）。

### 4.3 真实 JSON 样例（0.61.0 实测，逐字）

**命令与来源**：用 VS Code 扩展自带的 0.61.0 二进制
（`/Users/penglingwei/.vscode/extensions/sokonanoda-lang.sokonanoda-0.61.0-darwin-arm64/bin/darwin-arm64/sokonanoda --version`
→ `sokonanoda 0.61.0`），把源文本从 stdin 喂进去（`--json -`）。以下输出**未经改写**。

**(a) 事件五连**（源：`def id (a : Prop) : Prop := a` / `#check id` / `#reduce 1 + 2` /
`#print id` / `example : True := sorry`）：

```json
{"human":"checked declaration id","name":"id","type":"decl.checked"}
{"human":"id: Prop -> Prop","inferred_type":"Prop -> Prop","span":{"end":{"column":10,"line":2,"offset":39},"start":{"column":8,"line":2,"offset":37}},"text":"id","type":"expr.typed"}
{"human":"1 + 2 => 3","span":{"end":{"column":14,"line":3,"offset":53},"start":{"column":9,"line":3,"offset":48}},"text":"1 + 2","type":"expr.reduced","value":"3"}
{"human":"#print id :\ndef id (a : Prop) : Prop := a","name":"id","text":"def id (a : Prop) : Prop := a","type":"decl.printed"}
{"human":"exercise open (fill the sorry)","type":"exercise.open"}
```

退出码 `0`。同一份源的人读视图（逐字）：

```text
checked declaration id
id: Prop -> Prop
1 + 2 => 3
#print id :
def id (a : Prop) : Prop := a
exercise open (fill the sorry)
```

**(b) 内核拒绝**（源：`theorem bad : True := True.intro True.intro`）：

```json
{"code":"kernel-expected-pi","hint":"你把一个不是函数的值当函数用了，或者参数给多了。检查这个位置的东西的类型是不是 … -> … 形状。","message":"rejected: expected a pi type, got: True.[]","span":{"end":{"column":44,"line":1,"offset":43},"start":{"column":1,"line":1,"offset":0}},"stage":"kernel","type":"diagnostic"}
```

退出码 `1`。

**(c) 解析错误**（源：`def x : =`）：

```json
{"code":"unexpected-token","hint":"这里的写法不符合当前课程语法。检查命令拼写、括号配对，以及是否多写了还没学过的符号。","message":"expected expected `=>`, found =","span":{"end":{"column":9,"line":1,"offset":8},"start":{"column":9,"line":1,"offset":8}},"stage":"parse","type":"diagnostic"}
```

退出码 `1`。（`message` 里那个重复的 `expected expected` 是 0.61.0 的**真实**文本，
逐字保留。）

**(d) 警告**（源：仓库根 `playground.sokonanoda`，0.61.0 `--json` 输出尾部两条）：

```json
{"code":"reserved-declaration-name","hint":"删掉这一行即可；要写命题或类型，直接用内核已经有的 Prop / Sort / Type。","human":"warning[reserved-declaration-name]: `Prop` 内核已经定义过了，不能再声明一次。…","message":"`Prop` 内核已经定义过了，不能再声明一次。…","span":{"end":{"column":11,"line":84,"offset":4430},"start":{"column":7,"line":84,"offset":4426}},"type":"warning"}
{"code":"redundant-sorry","hint":"删掉这一行 sorry，这条声明就会通过内核检查；若还想继续写，请把它换成真正缺少的那部分。","human":"warning[redundant-sorry]: 这一行的 sorry 是多余的：前面的项已经完成了证明，sorry 不能再接在这里。","message":"这一行的 sorry 是多余的：前面的项已经完成了证明，sorry 不能再接在这里。","span":{"end":{"column":8,"line":333,"offset":20823},"start":{"column":3,"line":333,"offset":20818}},"type":"warning"}
```

（第一条的 `message`/`human` 在原文里是一整句，此处用 `…` 省略了后半句；
`span` 字段逐字保留。）注意 `span.offset` 是**字节**偏移，
`span.start.line`/`column` 才是行列（`courses/set-theory/AGENTS.md` 的判卷纪律、
`docs/protocol.md:803-809`）。

### 4.4 诊断码全表（按阶段分组，含真实 hint）

计数命令：`grep -c '=> "elab-' crates/front/src/compile/error.rs` → **29**；
`grep -c '=> "kernel-' …` → **12**；`grep -cE '=> "(import-|manifest-)' …` → **7**；
`grep -c '=> "' crates/front/src/diagnostic.rs` → **12**；
`grep -n '=> "' crates/front/src/compile/warning.rs` → **4**。
合计 **60 个错误码 + 4 个警告码**。

#### `parse` 阶段（12 个，`crates/front/src/diagnostic.rs:78-90`）

| code | 含义 | hint（逐字，`crates/front/src/diagnostic.rs:97-140`） |
|---|---|---|
| `unexpected-eof` | 输入提前结束 | 「输入到这里就结束了。检查是不是漏写了 `:=` 的值、右括号或 `end`。」 |
| `unexpected-token` | 写法不符合当前课程语法 | 「这里的写法不符合当前课程语法。检查命令拼写、括号配对，以及是否多写了还没学过的符号。」 |
| `import-malformed` | `import` 行缺模块名或多写了东西 | 「`import` 一行只写一个点分模块名（例如 `import Lesson.Logic`），并且必须写在文件最上方。」 |
| `import-not-a-valid-module-name` | 模块名某一段不是标识符（经典是文件名里的 `-`） | **hint 随诊断携带**（`DiagnosticKind::ImportNotAModuleName { hint, .. } => hint`，`crates/front/src/diagnostic.rs:107-109`） |
| `import-must-precede-declarations` | `import` 出现在声明之后 | 「官方 Lean 的规则：`import` 必须写在**任何声明之前**。把这行移到文件开头。」 |
| `unterminated-string` | 记法命令里的 `"` 没有闭合（span 指向**开引号**） | 「字符串没有闭合：记法命令里的符号要写在一对引号之间，例如 infix:50 \" ∈ \" => Set.mem。」 |
| `notation-shape` | 记法命令形状不对（缺优先级、缺 `=>`、符号是标识符词、优先级越界、`notation:N` 拼法、符号重复） | 「记法命令的形状是：infix:50 \" ∈ \" => Set.mem（infixl/infixr 同形，N 取 1–1000）；零元记法写 notation \"∅\" => Set.empty（不写优先级）。」 |
| `notation-unknown-symbol` | 用了没声明过的符号 | 「这个符号还没有在本文件里声明过。先在它前面写一行记法命令（例如 infix:50 \" ∈ \" => Set.mem），或者改用点名写法（Set.mem α a A）。」 |
| `parse-namespace-mismatch` | `end <名字>` 与最近的 `namespace` 不符，或 `end` 无处可收 | 「`end` 的名字要与最近的 `namespace` 一模一样（`namespace Foo` 用 `end Foo` 收）。检查是不是写错了名字，或者中间少了/多了 `end`。」 |
| `parse-namespace-unclosed` | 文件在 `namespace` 里结束（span 指回那行 `namespace`） | 「这个 `namespace` 一直没有闭合。在文件末尾（或块结束处）补一行 `end <名字>`，名字与 `namespace` 那行相同。」 |
| `parse-namespace-shape` | `namespace`/`end`/`open`/`export` 名字缺失或形状错（含 `open` 子句与 `open … in` 体不是叶子命令） | 「作用域命令的形状是：namespace Foo（开块）、end Foo（收块，名字必须写出）、open Foo（短名可用，可加点分名字 A.B）。`open` 还能挑名字：open Foo (a b)（只要这两个）、open Foo hiding a b（挡掉这两个）、open Foo renaming a => b（改名），以及只影响一条命令的 open Foo in <命令>；export Foo 同形，但导入本文件的文件也看得到。」 |
| `set-literal-shape` | 空 `{}` 或 ≥3 元素 | 「集合字面量的形状是 {a}（单元素）或 {a, b}（两元素）：元素之间用 `,` 隔开，最后用 `}` 收尾；空集写 Set.empty α，三个及以上用 Set.pair 点名嵌套。」 |

#### `elab` 阶段（29 个，`crates/front/src/compile/error.rs:162-190`）

| code | 含义 | hint（逐字，`crates/front/src/compile/error.rs:215-303`） |
|---|---|---|
| `elab-unknown-identifier` | 名字没有定义 | 「这个名字还没有被定义。检查拼写，或确认它出现在你前面的某个声明里（练习要在解决之后才能被后面的代码引用）。若它在该命名空间里，检查前缀或加 open。」 |
| `elab-unknown-constant` | 引用不存在的常量（含带宇宙参数的） | 「这里引用了一个不存在的常量。如果它带宇宙参数，请先定义它。」 |
| `elab-unknown-universe-level` | 宇宙层级变量没在当前声明里声明 | 「这个宇宙层级变量没有在当前声明里声明。用 {u} 声明它，例如 def id {u} : ...。」 |
| `elab-universe-arity` | 宇宙参数个数不对 | 「宇宙参数个数不对。这个常量声明了几个宇宙参数，就要给几个，例如 id.{u, v}。」 |
| `elab-untyped-binder` | binder 缺类型标注（`let` 的 message/hint 定制过） | 「这个 binder 缺少类型标注。教学版本要求写全类型，例如 fun (x : Nat) => x；let 的绑定也要写类型，例如 let x : Nat := 1; x。」 |
| `elab-hole-misplaced` | `sorry` 出现在值位以外 | 「sorry 只能出现在声明的值（答案区）位置，例如 example : T := sorry。」 |
| `elab-duplicate-declaration` | 同名声明两次 | 「这个名字已经定义过了。Lean 里每个名字只能声明一次；换一个名字，或删掉前面的声明。」 |
| `elab-too-many-binders` | 嵌套 binder 超出内核可表示深度 | 「嵌套的 binder 太多，超出了内核能表示的深度。把大表达式拆成几个小定义。」 |
| `elab-nat-literal-disabled` | Nat 字面量扩展未启用 | 「数字字面量没有被启用。这个版本默认打开 Nat 扩展，如遇到此错误请联系工具作者。」 |
| `elab-invalid-nat-literal` | 不是合法自然数字面量 | 「这不是一个合法的自然数字面量。」 |
| `elab-too-many-ctor-fields` | 构造子字段太多 | 「构造子的字段太多了。」 |
| `elab-unknown-ctor-for-iota` | iota 规则引用了不存在的构造子 | 「iota 规则引用了一个不存在的构造子。检查构造子名字是否与 ctor 声明一致。」 |
| `elab-ambiguous-ctor-alias` | 裸构造子名被两个类型各声明一次 | 「这个裸构造子名被两个类型各声明了一次，无法判断是哪一个。写全前缀名（例如 `P1.mk`），或给其中一个构造子换个名字。」 |
| `elab-tactic-failed` | `by` 块里一个 tactic 失败（目标形状不匹配 / 内核拒绝） | 「`by` 块里的 tactic 失败了：请检查当前目标与已引入的假设。」 |
| `elab-apply-needs-a-term` | apply 类 tactic 后面缺要应用的项 | 「`funapply` 后面要跟一个证明或函数，例如 `funapply h`；要应用的项复杂时可以用括号界定范围，例如 `funapply (f a)`。」（**文案滞后**，见 §3.5） |
| `elab-apply-not-applicable` | 要应用的项结论不是当前目标 | 「`funapply h` 要求 `h` 的结论正好是当前目标（`h : … -> 目标`）。看看 `h` 类型的最后一段是不是当前目标；不是就换一个前提，或直接写答案。」（同上，**文案滞后**） |
| `elab-match-bad-arm` | `match` 模式写错（带子模式的未知名、字段数与子模式数不符） | 「match 的模式要写对：构造子名与字段数要对应（可以嵌套，如 some (succ k)）；不确定的名字按变量绑定处理，带子模式的未知名才报错。」 |
| `elab-match-not-inductive` | 被匹配项不是已知归纳类型 | 「match 的被匹配项必须是已知的归纳类型：本文件用 inductive 声明的类型，或 prelude 内建的 Nat/Bool（分支写 Nat.zero/Nat.succ 或 Bool.true/Bool.false）。」 |
| `elab-match-no-expected-type` | `match` 结果类型未知，定不了 motive 与宇宙 | 「match 的结果类型必须已知：把它放在有类型标注的位置（声明类型、let/fun 的 binder 注解），或由外层 match 提供。」 |
| `elab-match-recursive-unsupported` | 该递归归纳不被 `match` 支持 | 「match 暂不支持递归归纳类型（v1 只做没有归纳假设的非递归分情况）；递归定义请直接用消去子 .rec。」 |
| `elab-match-non-exhaustive` | 未覆盖全部构造子/字段位置，或带守卫的 arm 没有兜底 | 「match 要覆盖该归纳类型的每一个构造子（含每个字段位置）；带守卫的 arm 还要在后面补一条不带守卫的兜底。」 |
| `elab-match-parameterized-unsupported` | 参数化归纳的 match 拿不到参数实例 | 「参数化归纳的 match 目前只支持：被匹配项是一个局部变量，且它的类型写成 `T 参数…`（显式给出归纳的全部参数）。换成一个这样标注的变量再 match。」 |
| `elab-let-type-query-failed` | 无类型标注的 `let` 推不出绑定类型 | 「无法从值推断出 `let` 绑定的类型；补上类型标注即可，例如 `let x : Nat := 1; x`。」 |
| `elab-notation-unknown-target` | 记法命令 `=>` 后面的目标名不存在 | 「记法命令指向的目标名不存在。检查 infix/notation 行里 `=>` 后面的名字拼写（要写点名，例如 Set.mem），并确认它已经声明过。」 |
| `elab-notation-argument-unsolved` | 记号展开时补不出目标 telescope 的前导类型参数 | 「这个记法展开时补不出前面的类型参数（本子集只按操作数的类型补，不做一般推断）。改用点名写法把参数写全，例如 Set.mem α a A；或在两边都是已知类型的上下文里使用记法。」 |
| `elab-notation-ambiguous` | 记法重载：≥2 个候选都说得通 | 「同一个符号声明了多条记法（重载），这里从期望类型看不出该用哪一条。写出点名形式（例如 Set.mem α a A）就消歧了；或者把这个表达式放到一个带类型标注的位置（例如 def … : T := 这里），让期望类型能定下来。」 |
| `elab-notation-no-candidate` | 记法重载：一个候选都对不上期望类型 | 「同一个符号的几条记法候选，结果类型都对不上这里的期望类型。对照错误里列出的候选结果类型，检查是不是用错了符号，或者改用点名形式。」 |
| `elab-binder-notation-unsolved` | binder 记法里变量类型反解不出 | 「binder 记法里的变量类型解不出：`∀ x ∈ s, p` / `∃ x ∈ s, p` 的 x 类型是从 `∈` 两边反解的。给 binder 补上类型标注（例如 ∀ (x : α) ∈ s, p），或改用点名写法（forall (x : α), Set.mem α x s -> p）。」 |
| `elab-set-literal-unknown-target` | 集合字面量展开的目标 `Set.singleton`/`Set.pair` 不在本文件里 | 「集合字面量 `{a}` / `{a, b}` 展开成点名形式 Set.singleton / Set.pair，但这个文件里没有它们。先 `import` 提供它们的库（卷 I 的 lib/Set），或改用点名写法。」 |

#### `kernel` 阶段（12 个，`crates/front/src/compile/error.rs:191-202`）

| code | 含义 | hint（逐字，`crates/front/src/compile/error.rs:305-346`） |
|---|---|---|
| `kernel-rejected` | 内核说不（转换失败时带 expected/actual 两侧） | 「内核判定不成立：类型不匹配或证明项不完整。先对比期望类型与你的值的形状；最常见的错误是两边结构不同（例如期望 a -> a，却写成了返回 Nat 的项）。」 |
| `kernel-expected-sort` | 需要类型的地方出现了项 | 「这里需要写一个类型（如 Prop、Type、Nat），但你写成了一个普通的项。检查冒号/binder 后面跟的是不是类型。」 |
| `kernel-expected-pi` | 非函数值被当函数用，或应用参数给多了 | 「你把一个不是函数的值当函数用了，或者参数给多了。检查这个位置的东西的类型是不是 … -> … 形状。」 |
| `kernel-theorem-not-prop` | `theorem` 的类型不是命题（**也出现在开练习上**，见 §4.1 第 7 条） | 「theorem 的类型必须是命题（Prop 里的东西）。想定义普通值请用 def。」 |
| `kernel-prop-not-cumulative` | 要 `Sort(n)`（n≥1）却给了 `Sort(0)`：**本语言没有累积性** | 「这里需要 Type（数据），但你给的是 Prop（命题）：本语言没有累积性，Prop 不是 Type 的子集（官方 Lean 4 有累积性，同一段代码在 Lean 里能过）。把陈述改成 Prop（例如等势用 Set.Equiv … : Prop 这样的命题版），或者交一个真正的 Type 值（如 Nat）。」 |
| `kernel-inductive-non-positive` | 构造子参数里递归引用出现在负位置 | 「递归引用出现在了负位置：构造子参数里 T 出现在箭头左边（如 T → Nat）。递归引用只能写在返回类型一侧。」 |
| `kernel-ctor-result-mismatch` | 构造子返回的不是本归纳的完整应用 | 「构造子的返回类型必须是本 inductive 的完整应用——参数和索引都要补齐，比如 T A n 而不是只写 T。」 |
| `kernel-ctor-arg-invalid-app` | 构造子参数里的递归引用不是合法应用 | 「构造子参数里的递归引用 T … 不是本 inductive 的合法应用：参数/索引的个数或取值不对。」 |
| `kernel-ctor-arg-not-type` | 构造子参数类型是项不是类型 | 「构造子参数的类型本身必须是一个类型，这里写成了一个项。」 |
| `kernel-ctor-arg-too-large` | 构造子参数类型所在宇宙太大 | 「构造子参数的类型所在的宇宙太大，装不进这个 inductive。把 inductive 声明成更大的 Type，或把该参数类型改成 Prop 里的命题。」 |
| `kernel-rec-rule-mismatch` | 显式 `rec`/`iota` 规则集与内核推导的不一致 | 「显式消去子（rec/iota）与内核推导出的规则不一致：iota 规则必须按构造子声明顺序一条不落地给出，每条规则的值也要与 motive、minor 和构造子参数形状完全匹配。」 |
| `kernel-internal` | 内核 bug，**永不是学习者的错** | 「内核内部错误（这不是你的代码问题）。请把这段代码反馈给工具作者。」 |

#### `import` 阶段（7 个，`crates/front/src/compile/error.rs:203-208`）

| code | 含义 | hint（逐字，`crates/front/src/compile/error.rs:347-368`） |
|---|---|---|
| `import-not-found` | 找不到 `<模块根>/Foo/Bar.sokonanoda`（消息列出试过的路径） | 「找不到这个模块。`import Foo.Bar` 对应模块根下的 `Foo/Bar.sokonanoda`；检查名字拼写与文件位置（名字里不能有 `-`）。」 |
| `import-cycle` | import 成环（消息拼出环） | 「import 成环了：A 依赖 B、B 又（直接或间接）依赖 A。把公共部分抽到一个更底层的模块里。」 |
| `import-dependency-failed` | 被导入模块没编译过，导入者也不编译（错误落在 `import` 行） | 「被导入的模块自己还有错误，所以这里先不编译——修好那个文件的第一条错误再看这里。」 |
| `import-name-collision` | 两个模块声明了同一个顶层名 | 「同一个名字在两个模块里各声明了一次。改掉其中一个，或把它移到一个公共模块里只声明一次。」 |
| `import-prelude-conflict` | 闭包对内置 prelude 意见不一致 | 「prelude（内置基元）是整个编译单元的属性：入口文件的设置与依赖模块冲突了。统一成一种（要么都用内置 prelude，要么都在入口声明 `-- sokonanoda:prelude none`）。」 |
| `manifest-invalid` | `sokonanoda.toml` 读不了或不是合法 TOML | 「`sokonanoda.toml` 读不了或不是合法 TOML。它只需要可选的 `name` / `requires` / `src` 三个键；不想要项目就把文件删掉（单文件模式仍然可用）。」 |
| `import-module-invalid` | 被导入的文件根本没解析成功 | 「被导入的文件本身没解析成功：先打开它、修好那里的语法错误，再回来看这里（下游不会替你猜）。」 |

#### 警告（4 个，`crates/front/src/compile/warning.rs:31-34`）

| code | 含义 | hint（逐字，`crates/front/src/compile/warning.rs:44-56`） |
|---|---|---|
| `reserved-declaration-name` | 顶层声明用了 `Prop`/`Sort`/`Type`（内核已定义，这一行永不被引用；`RESERVED_SORT_NAMES` 三名，`crates/front/src/compile/warning.rs:12`） | 「删掉这一行即可；要写命题或类型，直接用内核已经有的 Prop / Sort / Type。」 |
| `import-has-open-exercises` | 被导入模块还有 `sorry`，那些名字对下游不可见 | 「被导入文件里还有 `sorry`：这些声明对下游不可见（未完成的洞不进入环境），下游看不到它们的名字。」 |
| `redundant-sorry` | 值位里"多出来"的 `sorry`（前面的项已证完）；声明**仍是** `exercise.open` | 「删掉这一行 sorry，这条声明就会通过内核检查；若还想继续写，请把它换成真正缺少的那部分。」 |
| `open-shadowed-name` | `open`/`export` 让一个短名有了两个候选（或与根上同名声明撞车），本语言静默取第一个 | 「要点名的那一个就写全前缀（`A.x`）；要让短名指向另一个候选，用 `open A hiding x` / `open A (x)` / `open A renaming x => y` 把撞车的名字挡掉。」 |

**注意**：警告**永不影响退出码**（`docs/protocol.md:62`、`:84-85`）。
`redundant-sorry` 与签名毛病是**两个方向**：前者声明仍是 `exercise.open`，
后者必须报 diagnostic（修 G-01 时明确**不**把它做成 warning，因为 warning 不改退出码，
课程侧就永远发现不了签名腐烂，`docs/protocol.md:97-100`）。

### 4.5 退出码与消费纪律

- `0` = 答上了（**一个开放的 `sorry` 练习是合法状态**）；`1` = 文件被拒
  （内核拒绝声明、**解析失败**、或 `reduce` 失败）；`2` = 用法错误
  （`docs/protocol.md:798-802`）。**判据看 JSON，不看退出码**
  （`docs/protocol.md:802`）。
- `query <op>` 是同一份真相的**单 JSON 对象**视图：`check`/`state`/`goals`/
  `holes`/`hints`/`reduce`/`project`，信封 `{"schema":"soko.query/1","op":…,"ok":…,"data":…}`
  （`docs/protocol.md:759-790`）。**`ok:false` 不是空结果**——`goal: null`
  与 `navigated: null` 都是**成功**的答案（`docs/protocol.md:792-797`）。
- 计数一致性由 `crates/cli/tests/query.rs` 钉死（`docs/protocol.md:764-765`）。

---

## 5. prelude / 标准库分层（L1 / L2 / L3）

### 5.1 三层分界与唯一判据

硬规则 10（`REQUIREMENTS.md:27-36`，逐字节选）：

> 每一条陈述必须先归层——**L1 prelude**（Lean core 级：逻辑与等式骨架）/
> **L2 课程标准库**（集合的词汇 + 定义展开，Mathlib 里是 `rfl` 或一行、**没有数学内容**）/
> **L3 单元练习**（一切**有数学内容**的陈述）。判据只有一条：
> **「Mathlib 有 ≠ 我们不该练；Mathlib 有且没有数学内容（纯定义展开）才归库。」**

违反的样子 = 让学习者在证明里手写 `Eq.symm`/`Or.elim`/`mem_union` 这类东西（"暴力"）；
现场证据见 `docs/gaps/spike/README.md`，欠账台账 `L-01…L-05`
（`REQUIREMENTS.md:34-36`、`docs/design/course-stdlib.md:24-27`、`:34`）。

### 5.2 L1 — 内置受信任 prelude

三层预置，全部以 `.sokonanoda` 源语法书写后**安装**（不重查内核）：

| 层 | 名字数 | 内容 | 常量/安装点 |
|---|---|---|---|
| Nat/Bool 基元 | **9** | `Nat`,`Nat.zero`,`Nat.succ`,`Nat.rec`,`Nat.add`,`Bool`,`Bool.true`,`Bool.false`,`Bool.rec` | `crates/front/src/compile/prelude.rs:85-93`；安装 `:551`（Nat）/ `:631`（Bool），调用点 `crates/front/src/compile/check/mod.rs:488`、`:497` |
| Eq 三件套 | **3** | `Eq`,`Eq.refl`,`Eq.subst` | `PRELUDE_EQ_SRC` `crates/front/src/compile/prelude.rs:160` |
| L1 逻辑/等式骨架 | **35** | 8 个族 B1–B8 | `PRELUDE_L1_SRC` `crates/front/src/compile/prelude.rs:195`；`L1_FAMILIES` `:254` |
| **`PRELUDE_NAMES` 合计** | **47** | 补全/材料用的白名单 | `crates/front/src/compile/prelude.rs:84` |

计数命令（在仓库根跑）：
`awk 'NR>=84 && NR<=139' crates/front/src/compile/prelude.rs | grep -c '^    "'` → **47**；
`PRELUDE_L1_SRC`（`:195`–`:231`，35 行）按 kind：
`python3 -c "…Counter(l.strip().split(' ')[0] …)"` → `axiom` **5** · `def` **23** ·
`inductive` **2** · `ctor` **3** · `end` **2**（33 条命令 + 2 个 `end`；
`And.rec`/`Or.rec` 由 `install_inductive_block` 派生登记，
`crates/front/src/compile/prelude.rs:466-493`，不在源文本里）。

`PRELUDE_L1_SRC` 逐字节选（`crates/front/src/compile/prelude.rs:196-210`）：

```sokonanoda
axiom True : Prop
axiom True.intro : True
axiom False : Prop
axiom False.rec : (C : Prop) -> False -> C
def False.elim (C : Prop) (h : False) : C := False.rec C h
inductive And (a b : Prop) : Prop
ctor And.intro (ha : a) (hb : b) : And a b
end
def And.left (a b : Prop) (h : And a b) : a := And.rec a b (fun (_ : And a b) => a) (fun (ha : a) (hb : b) => ha) h
def And.right (a b : Prop) (h : And a b) : b := And.rec a b (fun (_ : And a b) => b) (fun (ha : a) (hb : b) => hb) h
def And.elim (a b c : Prop) (f : a -> b -> c) (h : And a b) : c := f (And.left a b h) (And.right a b h)
inductive Or (A B : Prop) : Prop
ctor Or.inl (a : A) : Or A B
ctor Or.inr (b : B) : Or A B
end
```

**让位规则（谁声明谁拥有）**：粒度 = **族**，不是单名。
`family_yields`（`crates/front/src/compile/prelude.rs:313-322`）判据：本族任一名字命中
`taken`，**或**任一 `deps` 命中即整族跳过。`taken` = 整个闭包的顶层名字并集
（`top_level_def_spans_over` 的键集，含构造子与递归子，
`crates/front/src/compile/check/mod.rs:502-506`）；安装顺序**先 Eq 后 L1**
（`check/mod.rs:512-513`）。族表（`crates/front/src/compile/prelude.rs:254-310`）：

| 族 | 名字 | 依赖（被占用则本族也让位） |
|---|---|---|
| B1 真伪 | `True`, `True.intro` | — |
| B2 假与爆炸 | `False`, `False.rec`, `False.elim` | — |
| B3 且 | `And`, `And.intro`, `And.left`, `And.right`, `And.elim` (+`And.rec`) | — |
| B4 或 | `Or`, `Or.inl`, `Or.inr`, `Or.elim` (+`Or.rec`) | — |
| B5 非 | `Not`, `Not.intro`, `Not.elim`, `absurd` | B2 |
| B6 当且仅当 | `Iff`, `Iff.intro`, `Iff.mp`, `Iff.mpr`, `Iff.refl`, `Iff.symm`, `Iff.trans` | B3 |
| B7 Eq 引理 | `Eq.symm`, `Eq.trans`, `congrArg` | Eq |
| B8 类型层等式 | `Eq.rec`, `Eq.ndrec`, `Eq.mp`, `Eq.mpr`, `cast` | Eq |

`PRELUDE_NEVER_YIELDS`（**9** 个：Nat/Bool 家族，`crates/front/src/compile/prelude.rs:148`）
只用于 `check_name_collisions` 的豁免（`crates/front/src/project/mod.rs:394-396`）——
L1 名字**不在**此列，所以两个模块各自声明 `True` 仍报友好的 `import-name-collision`，
不会退化成内核裸错（`docs/architecture.md:438-441`）。

**prelude 两种模式**（`REQUIREMENTS.md:65-73`）：`PreludeMode::Full`（默认）与
`PreludeMode::Bare`（完全不装任何东西，课程从零构造一切）；CLI `--bare` 旗标，
文件里也能写 `-- sokonanoda:prelude none` 声明式指令
（`crates/front/src/compile/prelude.rs:39-43`、`crates/cli/src/help.rs:32-34`）。

### 5.3 L2 — 课程标准库 `courses/set-theory/lib/`

**906 行 · 74 条顶层声明 · 0 个 `sorry`**。

命令：`cat courses/set-theory/lib/*.sokonanoda | wc -l` → 906；
`cat courses/set-theory/lib/*.sokonanoda | grep -cE '^(def|theorem|abbrev|axiom|inductive) '` → 74；
`grep -o sorry courses/set-theory/lib/*.sokonanoda | wc -l` → 0。

| 文件 | 行数 | 顶层声明 | 文件 | 行数 | 顶层声明 |
|---|---|---|---|---|---|
| `Demo.sokonanoda` | 61 | 10 | `Logic.sokonanoda` | 61 | **0**（P4 后纯注释空壳，`:3`） |
| `Equiv.sokonanoda` | 148 | 5 | `Prod.sokonanoda` | 73 | 6 |
| `Exists.sokonanoda` | 102 | 3 | `Rel.sokonanoda` | 70 | 7 |
| `Fun.sokonanoda` | 141 | 14 | `Set.sokonanoda` | 159 | 23 |
| `Image.sokonanoda` | 91 | 6 | **合计** | **906** | **74** |

与设计文档自洽：`docs/design/course-stdlib.md:117-119` 记「`lib/` **74 条声明全 checked、
0 open、0 failed**（Demo 10 · Equiv 5 · Exists 3 · Fun 14 · Image 6 · **Logic 0** · Prod 6 · Rel 7 · Set 23）」。
（`Equiv.sokonanoda` 的 `grep -c sorry` 报 1，实为注释 `Equiv.sokonanoda:70`。）

### 5.4 L3 — 单元练习 `courses/set-theory/units/`

**2841 行 · `sorry` 出现 110 次 · 真洞 99 个 · 顶层声明 182 条 · `import` 行 44 条**。

命令：`cat courses/set-theory/units/*.sokonanoda | wc -l` → 2841；
`cat … | grep -o 'sorry' | wc -l` → 110；
`grep -n 'sorry' <f> | grep -c ':[[:space:]]*--'` 全目录 11 行（行内注释里的 `sorry`）
⇒ 真洞 110 − 11 = **99**；`cat … | grep -cE '^(def|theorem|abbrev|axiom|inductive) '` → 182；
`grep -h '^import ' courses/set-theory/units/*.sokonanoda | wc -l` → 44。

99 与 `docs/design/course-stdlib.md:62-64` 的 P4 复测「329 checked · 99 open · 0 判负」
**逐数吻合**；扣掉记法页（`notation-cheatsheet.sokonanoda`，非单元、不进 `course.json`）
的 3 题后 = 96 = 该文档的 `summary.canvas_open 96`。

**第一门课 `course/`（unit1..unit11）**：11 个单元文件、**1401 行**、`sorry` **98** 次
（`ls course/unit*.sokonanoda | wc -l` → 11；`cat course/unit*.sokonanoda | wc -l` → 1401；
`cat course/unit*.sokonanoda | grep -o 'sorry' | wc -l` → 98）。
清单 `course/course.json` 是 **v1 扁平数组**，**11** 条 = 全部 11 个单元
（`python3 -c "import json;print(len(json.load(open('course/course.json'))))"` → 11），形状：

```json
[
  {
    "file": "unit1-propositions-proofs.sokonanoda",
    "title": "单元① 命题与证明项",
    "title_en": "Unit 1 — Propositions & Proof Terms",
    "unit": 1
  },
```
—— `course/course.json:1-7`（逐字，含缩进与换行）；v2（`{"schema": "soko.course/2", …}`）见
`courses/set-theory/course.json` 与 `docs/protocol.md:655-668`。
v1 仍然合法：「入门课 `course/course.json` 就是 v1，一个字节都没改」
（`courses/set-theory/README.md:98-99`）。

**判卷命令**（逐字，`courses/set-theory/README.md:13-18`）：

```bash
python3 courses/set-theory/tools/check.py             # 判据 G1–G6 + 人读表 + 汇总
python3 courses/set-theory/tools/check.py --selftest  # 判据通道自检（故意坏文件必须被拒 + G6 清单自检 + 台账字段）
python3 courses/set-theory/tools/check.py --json      # 机器可读（含计数；CI 用 --report 落盘）
python3 courses/set-theory/tools/check.py --only "单元 5" --bisect   # 二分到第一个判红的声明
python3 courses/set-theory/tools/check.py --ledger    # 追加一条成本台账（默认 docs/courses/ledger.jsonl；默认关闭）
```

单文件判卷与查询（`courses/set-theory/README.md:75-76`）：

```bash
node scripts/soko grade "$PWD/courses/set-theory/units/unit01-sets-membership.sokonanoda"
node scripts/soko query state --file "$PWD/courses/set-theory/units/unit01-sets-membership.sokonanoda" --line 40 --col 3
```

判据 G1 只认 `grade` 的退出码（`courses/set-theory/tools/check.py:9`）；
门禁内部唯一 argv 是 `argv = [*channel.argv, "grade", "--json", str(path.resolve())]`
（`courses/set-theory/tools/check.py:215`）。

**未证实**：`course/` 那 98 次 `sorry` 是否含注释内出现**未逐行核**；
L3 没有独立 `exercise` 关键字（练习 = 带 `sorry` 的声明，
`courses/set-theory/README.md:84`），`course.json` 只给章级 `quota.exercises`
（I.1=35 / I.2=24 / I.3=23 / I.4=14，合计 **96**），**不是逐题清单**。

---

## 6. 性能事实（只写仓库里量过的数）

> 纪律：**只抄实测值，绝不估算**；每个数字带来源。找不到测量的一律写 **未证实**。

### 6.1 数字是怎么产生的（确切命令）

`scripts/perf-ledger.sh`（用法见 `scripts/perf-ledger.sh:11-13`）依次跑四套，
**一律带 `--test-threads=1`**（`scripts/perf-ledger.sh:47/50/53/57`）：

```bash
cargo test -q -p sokonanoda-front --test perf --locked -- --nocapture --test-threads=1
cargo test -q -p sokonanoda-front --test perf_project --locked -- --nocapture --test-threads=1
cargo test -q -p sokonanoda-lsp --lib --locked -- perf_ --nocapture --test-threads=1
cargo test -q --release -p sokonanoda-cli --test perf_project --locked -- --nocapture --test-threads=1
```

结果追加进 `docs/perf/ledger.jsonl`。台账顶层字段
（`scripts/perf-ledger.sh:77-91`）：`schema`（`"soko.perf-ledger/1"`）、`version`、
`commit`、`dirty`、`date`、`cli_profile`、`host{system,machine,release}`、`records[]`；
每条 record 带 `schema`（`"soko.perf/1"`）、`scope`（`front-project`/`lsp-project`/
`cli-project`）、`case` 与各指标键（`compile_ms`、`ms[]`、`best_ms`、`worst_ms`、
`keystroke_ms`、`cold_ms`、`warm_ms`、`hover_ms`、`build_ms`、`query_cold_ms`…）。

**口径警告（逐字，`docs/PERF.md:129-130`）**：

> 口径：**2026-09-18 起为串行**（`--test-threads=1` + 用例内 best-of-N）。
> 更早的基线（front compile 90–110ms、LSP 按键 25–49ms 等）是并行口径，约偏高 3–4×。

### 6.2 台账最新一条（`docs/perf/ledger.jsonl:14`，**v0.59.0**，2026-09-19T01:31:13Z）

| 用例 | 指标（逐字） |
|---|---|
| `closure_stages` | `"compile_ms": 34.23, "digest_ms": 0.004, "plan_ms": 0.26, "total_ms": 34.5` |
| `closure_compile_scaling` | `"modules": [4, 8, 16], "ms": [18.42, 31.93, 62.76], "ratio_4x": 3.41` |
| `teaching_scale_keystroke` | `"ms": [13.38, 17.62, 25.92]` |
| `judge_prefix_with_imports` | `"matches": 10, "ms": 72.65` |
| `overlay_overhead` | `"disk_ms": 34.3, "overlay_ms": 34.47` |
| `cold_warm_check` | `"cold_ms": 30.2, "warm_ms": 3.43` |
| `dependency_edit_cache_miss` | `"after_dependency_edit_ms": 23.99` |
| `query_and_build` | `"build_ms": 32.24, "query_cold_ms": 23.17, "query_warm_ms": 2.9` |
| `request_latency` | `"definition_ms": 0, "goals_ms": 0, "hover_ms": 0` |
| `did_open_and_keystroke` | `"keystroke_ms": 14, "open_ms": 14, "publishes_per_keystroke": 1` |
| `dependency_edit_refresh` | `"elapsed_ms": 3, "publishes": 2, "dependent_diagnostics": 1` |

主机一律 Darwin/arm64，`cli_profile=release`。`docs/perf/latest.json` 是第 14 条的
逐字副本（`:2-12`，records `:13-141`）。台账共 **14 条**（`wc -l docs/perf/ledger.jsonl`
→ 14；第 1–8 条是并行口径，无 `best_ms`，数值比第 9–14 条高 3–4×）。

**重要诚实点**：台账**最新一条是 v0.59.0**——仓库当前版本是 **0.61.0**，
所以 **0.60.0 / 0.61.0 没有已提交的性能测量**（未证实）。

### 6.3 `docs/PERF.md` 的当前基线（2026-09-18，v0.58.0，串行口径）

逐字表（`docs/PERF.md:132-150`）：

| 场景 | 实测（串行口径） |
| --- | --- |
| front 分阶段（4 模块 × 20 声明） | plan 0.3ms · digest ~0.004ms · **compile 32–38ms** · total ≈ 33–39ms |
| front 缩放（4/8/16 模块 × 10 声明） | 19 / 35 / 65 ms，4× 规模 ⇒ 3.0–3.4×（线性） |
| front 一次按键（4×20，全部重编译） | **best 33–39ms**（同轮 `worst` 34–70ms，作为保守上界） |
| front 教学规模一次按键（2/3/5 模块 × 12 声明） | **14 / 16 / 24ms** |
| front 内存覆盖 vs 读盘（4×20） | 38.7 vs 38.8ms（覆盖无额外成本） |
| front 判据前缀（入口 10 处 `match` 导入的归纳类型，2 模块） | 70–83ms |
| LSP 项目 didOpen / 一次按键（2×12） | 12ms / **12ms，每次按键 1 份诊断** |
| LSP 改依赖 ⇒ 下游刷新（3×12，两文档打开） | 1–2ms，2 份诊断 |
| LSP 项目 hover / definition / goals | 各 < 1ms |
| CLI 项目冷 / 热 / 依赖改动后（3×12，release） | **29.6ms / 3.4ms / 23.7ms** |
| CLI `build` / `query` 冷 / `query` 热（3×12） | 29.6ms / **20.4ms** / **3.3ms** |
| 扩展：光标移动（200ms 去抖） | 1 × `stateAt`，~1KB / 49 DOM 节点 / 0.17ms |
| 扩展：Infoview `decls` 整表重建（50 条） | 28.6KB / 1200 节点 / 1.2–2.1ms |
| 扩展：课程树一次 CLI 运行（11 单元，release 热缓存） | ~320ms |

结论句（逐字，`docs/PERF.md:203-205`）：

> 结论（串行口径）：**教学规模（2–5 个模块、每模块 ~12 条声明）一次按键 14–24ms，
> 编辑器完全无感**；4×20 的"大项目"约 33–39ms。

更早的 v0.23.0 基线（并行口径，**只能与同口径比**，`docs/PERF.md:54-64`）：
`~45/116/158 ms`（50/200/400 块）、`初始 ~47ms，每键 <5ms，kernel_checks=1`、
`每键 3-8ms…500+ 块仍 <100ms`。

### 6.4 噪声地板与比较纪律（`docs/PERF.md:157-190`）

| 跑法 | `closure_stages.compile`（4×20） | LSP didOpen / 按键（2×12） |
| --- | --- | --- |
| 只跑这一个用例 | **32.4ms** | — |
| 6 个 perf 用例 `--test-threads=1` | **33–38ms** | 12ms / 12ms |
| 6 个 perf 用例默认并行 | **118–152ms**（同一提交两次记录差 28%） | 64ms / 50ms |

逐字纪律：① 台账脚本一律 `--test-threads=1`；② 用例内部 best-of-N；
③ **比较台账数字先看是否落在 ±25% 内**；④ **优先比 `best_ms`**（`worst_ms` 这类 max
统计量能差 45%）。另外：**macOS 上刚构建出来的二进制第一次 spawn 要付 ~425ms**
（第一次写这套测试时记成了 527ms，真实值 29.8ms，`docs/PERF.md:152-155`）；
CI 假红实测 `480ms`（同机单跑 17ms / 满负载 86ms），预算最终改 **800ms**
（对最慢一次实测 2.4× 余量，`docs/PERF.md:176-187`）。
`release 冷 24ms / debug 冷 405ms`（同一 3×12 项目，`docs/PERF.md:124`）。

### 6.5 课程成本台账与 criterion 基准

- `docs/courses/ledger.jsonl:1`（**唯一一条**，v0.60.0，2026-09-19T10:18:26Z）：
  `"targets": 36, "checked": 329, "open": 99, "rejected": 0, "elapsed_ms": 20024, "solutions_open": 0`
  —— 即**整门卷 I 集合论判一遍 20024ms**。`STATUS.md:51` 复述同一组数。
- criterion 基准（**唯一一次记录**，`docs/STATUS-ARCHIVE.md:2840-2841`）：
  `native_bigint_reduce ~41ms / iota_deep_reduce ~14ms / session_suffix_recheck ~500µs`
  （命令 `cargo bench -p sokonanoda-front --bench pipeline`）；**不进 CI**
  （`docs/TESTING.md:344-345`）。

### 6.6 未证实 / 仓库里**没有**的数字

- **「相对官方内核 10–100x」未证实**：`REQUIREMENTS.md:40` 写「sokonanoda 内核的快
  （arena + hash-consing + 闭包求值，相对官方内核 10–100x）」，但仓库里
  **没有任何该比对的测量记录**；arena 集成测试是 opt-in、CI 不跑
  （`docs/PERF.md:74-76`：「未设置时自动跳过，CI 不依赖它」「语料很大且属外部仓库，
  **不 vendor**」）。网站引用这个倍数前必须先量。
- **perf 套件是"哨兵"不是"基准对比"**（逐字，`docs/PERF.md:68`）：
  「仓库内的 perf 套件是**哨兵**…不是与外部实现的**基准对比**」。
- **没有 0.60.0 / 0.61.0 的台账条目**；没有单文件（`front/tests/perf.rs`）的时间进台账
  （它只打印人类可读的 `PERF` 行，不发 `PERFJSON`）；没有内存/RSS、吞吐量、
  每次操作的 µs 级数字（唯一的 µs 数是上面那条 criterion 的 `~500µs`）。
- **没有任何提交进仓库的 perf-report 文本**（CI 以 artifact 上传，
  `.github/workflows/ci.yml:148-152`）。
- **CI 里的 perf 步骤没有带 `--test-threads=1`**（`.github/workflows/ci.yml:132,135,138,141`），
  而 `docs/PERF.md:167-168` 说两个脚本一律带——这是一处**文档与 CI 的口径不一致**，
  引用 CI 数字时要留意。

---

## 7. 诚实的边界（这门语言**故意不做**的事）

### 7.1 内核与判定

| 边界 | 事实与出处 |
|---|---|
| **内核冻结** | `crates/kernel/**` 是上游快照，**一个字节不许动**（硬规则 1，`REQUIREMENTS.md:15-16`）；`cargo fmt --all` 被禁止，因为它会重排冻结内核（`AGENTS.md` 的 gate 说明）。 |
| **没有累积性** | `Prop ⊆ Type` 在本语言**不成立**（官方 Lean 4 有）；给 `Sort(n≥1)` 的地方给 `Sort(0)` 报 `kernel-prop-not-cumulative`（`docs/protocol.md:183-186`、`docs/design/prop-cumulativity-boundary.md`）。 |
| **`by` 引擎的判定合成声明不带宇宙参数** | `by.rs::spec_of` 的 `universe: Vec::new()`，所以**目标里出现宇宙变量**时 tactic 判定失败（`Sort u` 自 0.60.0 起、`Sort (u+1)` 继承同一条；`suggest`/`judge_terms` 不受影响，`docs/architecture.md:447-450`）。 |
| **`congrArg` 只能同宇宙层级** | G-14（`docs/architecture.md:445`）。 |
| **没有源码级 print-back** | 目标/类型文本由**冻结内核的 pp** 产出，记法不进内核 ⇒ goal/hover 显示的是点名形式（`Set.mem α a A`，不是 `a ∈ A`）。要做就得改内核 pp 或维护两套真相——**明确不做**（`docs/design/notation-subset.md:550`）。 |
| **内核拒绝仍是 panic → Result** | 更细粒度的 kernel 错误分类仍是后续任务（`docs/architecture.md:518`）。 |
| **多子目标洞共享同一个源位置** | `assemble` 给每个叶子洞同一个 `hole_span`，因为那些前提没有自己的源文本 ⇒ `soko/nextHole` **不能**在子目标之间逐个跳（只能整组跳）；稳定身份是 `holes[i].id`（`docs/protocol.md:535-552`）。 |

### 7.2 语法与 elaborator

| 边界 | 事实与出处 |
|---|---|
| **没有隐式实参推断** | 隐式 binder 只是打印样式（`BinderStyle` 只影响打印，不影响类型检查，`docs/architecture.md:339`）；**隐式实参不自动插入**，签名要显式给全参数（`docs/architecture.md:445-446`）；课程英文镜像里也明写 "Implicit arguments are never auto-filled: pass them explicitly"（`course/en/unit2-equality-rfl.sokonanoda:35`）。 |
| **没有类型类 / 结构 / instance** | ROADMAP 把 `notation / macro / typeclass / 完整 tactic` 一起列为"课程未到之前不进入白名单"（`ROADMAP.md:145`）；`docs/architecture.md:519` 的未做清单含"结构/类型类"。 |
| **没有 `section` / `variable`** | 未做清单（`docs/architecture.md:519`）。 |
| **没有 macro** | 同上；`notation` 已于 0.59.0 落地，但那是记法不是 macro（`docs/architecture.md:519`）。 |
| **`sorry` 只能在声明的值位** | 别处报 `elab-hole-misplaced`（`crates/front/src/compile/error.rs:233-235`）；`???` 洞已于 2026-09-07 移除（`docs/architecture.md:107`）。 |
| **`funapply` / `funintro` 已删除** | 0.22.0 / 0.27.0 移除；`funintro` 现在只是普通标识符并报未知名（`docs/protocol.md:259-265`）。 |
| **`Type u` 不支持** | `Type` 后跟裸标识符必须保持**应用**语义；需要时写 `Sort u`（`docs/design/type-level-syntax.md:28-31`）。 |
| **层级算术只收数字后缀** | `u+v` / `max u v` / `imax` 不在语法面内（`docs/design/type-level-syntax.md:78-82`）。 |
| **带索引归纳 + 宇宙多态参数不支持** | `docs/architecture.md:214-221`。 |
| **无注解 `let` 未做** | v1 要求显式类型注解（`docs/architecture.md:182-186`）。 |
| **`match` 的四条限制** | 无元组/记录/字符串字面量模式（`docs/design/match-patterns.md:27`）；参数化归纳的 scrutinee 必须是**写成 `T 参数…` 的局部变量**（`docs/protocol.md:160-161`）；结果类型依赖索引不在 v1；嵌套/守卫下的依赖 motive 子目标类型**退回常量**（保守，`docs/design/match-patterns.md:175`）；守卫限 prelude `Bool`（同处）。 |
| **`open scoped` 子命名空间不传播** | `docs/architecture.md:145-146`。 |
| **后缀记法实参位免括号明确不做** | `f Aᶜ` 今天读作 `(f A)ᶜ`；改了会悄悄重分组。一元记法在实参位要写括号 `f (𝒫 A)`（`docs/architecture.md:143-144`、`courses/set-theory/units/notation-cheatsheet.sokonanoda:117-119`）。 |
| **`Prop` / `Sort` / `Type` 不能声明** | 声明了也给一条 warning（`reserved-declaration-name`），名字永不被引用（`crates/front/src/compile/warning.rs:12`、`docs/design/reserved-decl-warning.md`）。 |

### 7.3 工具链与产品形态

| 边界 | 事实与出处 |
|---|---|
| **没有 WASM 构建** | 官网一期"零后端，主 CTA = 装 VS Code 扩展，**WASM playground 入 backlog**"（`REQUIREMENTS.md:421-425`）；站点调研也把"没有 WASM 构建"列为必须诚实标出的真限制（`docs/design/site-rebuild/research/R1-design-craft.md:264`）。**结论：网站不能内嵌"真的能跑"的 playground**，只能展示源码 + 预录制的真实事件输出。 |
| **用户/agent 路径零工具链依赖** | 获取与运行只依赖 GitHub Release 资产或平台 VSIX，**不要求 Rust/cargo**；cargo 仅贡献者需要（硬规则 9，`REQUIREMENTS.md:22-26`）。 |
| **单文件不读 `sokonanoda.toml`** | 没有 `import` 的文件**从不发现/读取清单**，也不付任何项目开销（`docs/architecture.md:271-273`、`:533-535`）。 |
| **项目模式没有跨模块增量** | 逐字：「项目模式没有跨模块增量——真要优化，方向是"按模块复用已查环境"（设计 §1.3 已明确 v1 不做）」（`docs/PERF.md:205`）。 |
| **REPL 没有行编辑** | 方向键回溯 out of scope；历史只追加到 `$HOME/.sokonanoda_history`（截断到最近 1000 行，`docs/protocol.md:727-732`）。 |
| **自定义 LSP 请求的消费方有限** | `soko/goals`/`hints`/`nextHole`/`stateAt`/`project`/`version` 目前只有 VS Code 扩展与 opencode 消费；DeepSeek Harness 侧无消费者（`AGENTS.md`；`docs/design/deepseek-harness.md` 的 H5 backlog）。 |
| **位置列号按 char 计** | 列号今天数 `char`（`crates/front/src/token.rs`），与 LSP 的 UTF-16 `character` 对 BMP 文本一致，**每个星平面字符（emoji）会短一**；这是单独跟踪的缺口，消费者**不要**自行补偿（`docs/protocol.md:803-809`）。 |

### 7.4 已知的"文档滞后"与待盯边界（引用前必读）

1. **`docs/architecture.md:519` 仍把 "`match` tactic" 列在未做清单里**，但 0.61.0
   **实测可用**：`by match` 进白名单（`crates/front/src/parser.rs:1277-1286`，
   `is_tactic_keyword` 含 `match`，`crates/front/src/parser.rs:2844-2849`），
   并有端到端测试 `cli_by_match_tactic_checks_via_kernel`（`crates/cli/tests/cli.rs:213-228`）。
   **以代码与实测为准。**
2. **`elab-apply-needs-a-term` / `elab-apply-not-applicable` 的 hint 仍写 `funapply`**
   （`crates/front/src/compile/error.rs:261`、`:264`），而 `funapply` 已删除
   （`docs/protocol.md:259-265`）。
3. **prelude `Nat.add` 是"自引用占位"体**，语义上等价于公理 + 原生快路径；
   `#reduce Nat.add => Nat.add` 当前可终止，但这是要**长期盯住**的边界
   （`docs/architecture.md:390`）。
4. **`Nat.rec` 的 `NatLit` 快路径会给递归结果套一层未归约的一元链**，
   `deep_reduce` 不再回收 ⇒ `match` 递归结果可能呈**混合表示**
   （如 `2 + 1 => Nat.succ (Nat.succ 1)`），def-eq 上仍等于 3
   （`docs/architecture.md:390`）。
5. **源文件自带 `inductive Nat` 且 ctor 叫 `Nat.zero`/`Nat.succ` 时**，
   `#reduce` 输出从 `succ (succ …)` 变成混合表示
   `Nat.succ (Nat.succ (Nat.succ 1))`（`docs/architecture.md:391-397`）。

---

## 8. 可展示的 8 个最佳代码样例（由浅入深）

> 选材标准：**一个陌生人扫一眼就知道这门语言长什么样**；全部逐字引用仓库原文。
> 顺序 = 从最简单到最 impressive。

### 1. 最小的完整文件（7 行）

```sokonanoda
-- Lesson 1: functions and types

def id : Prop -> Prop := fun (x : Prop) => x

#check id

example : Prop -> Prop := sorry
```
—— `examples/lesson-01.sokonanoda:1-7`

**为什么好**：7 行就交代了全部产品模型——声明、`#check` 自测、`sorry` 待办、
`--` 中文/英文讲解；这是"第一屏就能读完"的样例。

### 2. 命题即类型，证明即项

```sokonanoda
theorem prop_id: (a : Prop) -> a -> a :=
  fun (a : Prop) => fun (h : a) => h
```
—— `playground.sokonanoda:175-176`

**为什么好**：类型里每一层箭头对应值里一层 `fun`——这一行把"证明 = 构造项"
讲完了，是整门课的中心思想（`playground.sokonanoda:76-79` 的课就是这么说的）。

### 3. `by` 块：换一种方式写证明

```sokonanoda
theorem demo_by_assumption : (a : Prop) -> a -> a := by intro a; intro h; assumption
theorem demo_by_apply : Or True False := by apply Or.inl; exact True.intro
theorem demo_by_rfl : Eq.{1} Nat (1 + 1) 2 := by rfl
```
—— `course/unit4-by-tactics.sokonanoda:37`、`:42`、`:46`

**为什么好**：三条一行证明把 tactic 语言的全貌摆出来（`intro`/`assumption`/
`apply`/`exact`/`rfl`），而且每条都注明"判定永远走内核：搭出来的证明和手写项等价"
（`course/unit4-by-tactics.sokonanoda:6`）。

### 4. 自己声明归纳类型，然后 `match` 它

```sokonanoda
inductive Color : Type
ctor red : Color
ctor green : Color
rec Color.rec {u} :
  (motive : (c : Color) -> Sort u) ->
  (mr : motive red) ->
  (mg : motive green) ->
  (c : Color) -> motive c
iota red :=
  fun (motive : (c : Color) -> Sort u) =>
  fun (mr : motive red) =>
  fun (mg : motive green) => mr
iota green :=
  fun (motive : (c : Color) -> Sort u) =>
  fun (mr : motive red) =>
  fun (mg : motive green) => mg
end
```
—— `course/unit6-induction-recursion-1.sokonanoda:86-102`；配 `match` 用法：

```sokonanoda
def swap (c : Color) : Color := match c with
| red => green
| green => red
```
—— `course/unit6-induction-recursion-1.sokonanoda:105-107`

**为什么好**：它同时展示"手写 recursor + iota 规则"（Lean 教科书里最劝退的一段）
与"`match` 一行搞定"两种写法——语言既有内核级的完整能力，也有教学的短路径。

### 5. 递归不用自引用：归纳假设自动插入

```sokonanoda
def addM (a b : Nat) : Nat := match a with
| zero => b
| succ m => succ ih
```
—— `course/unit6-induction-recursion-1.sokonanoda:135-137`

**为什么好**：`succ` 分支里凭空出现的 `ih` 是递归构造子字段后**自动插入**的归纳假设
（`docs/architecture.md:198-200`）——这行代码让"归纳法"从口号变成一个能直接用的名字。

### 6. 数学符号是真的：用户自定义记法 + binder 记法

```sokonanoda
infix:50 " ∈ " => Set.mem
infix:50 " ⊆ " => Set.subset
infixl:65 " ∪ " => Set.union
notation "∅" => Set.empty
```
—— `courses/set-theory/units/notation-cheatsheet.sokonanoda:86-89`

```sokonanoda
binder_notation "∃" => Exists
```
—— `courses/set-theory/units/unit08-images-preimages.sokonanoda:141`；用法（同文件 `:144-146`）：

```sokonanoda
example (α β : Type) (f : α -> β) (A : Set α) (y : β) :
    Prop :=
  ∃ (x : α), And (Set.mem α x A) (Eq.{1} β (f x) y)
```

**为什么好**：`∈`/`⊆`/`∪`/`∅`/`∃` 都是**用户声明**的源级糖，不是内建符号——
"纸笔数学的写法"与"点名形式"两种写法判卷一致，且点名形式永久可用
（`courses/set-theory/units/notation-cheatsheet.sokonanoda:130-139` 有并排演示）。

### 7. 带索引归纳类型：长度写在类型里

```sokonanoda
inductive Vec (A : Type) : Nat -> Type
ctor vnil : Vec A zero
ctor vcons (a : A) (n : Nat) (v : Vec A n) : Vec A (succ n)
end
```
—— `course/unit7-induction-recursion-2.sokonanoda:121-124`；用法：

```sokonanoda
def vlen (A : Type) (n : Nat) (v : Vec A n) : Nat := match v with
| vnil => zero
| vcons a m w => succ ih
```
—— `course/unit7-induction-recursion-2.sokonanoda:127-129`

**为什么好**：`Vec A n` 的**长度在类型里**，`vnil` 只能返回 `Vec A zero`、
`vcons` 只能返回 `Vec A (succ n)`——这是依赖类型最直观的一张名片，而这里
连 recursor 都是前端自动派生的（`docs/architecture.md:214-221`）。

### 8. Cantor 对角线法（最 impressive）

```sokonanoda
theorem demo_diag (α : Type) (f : α -> Set α) (x : α) :
    Not (Eq.{1} (Set α) (fun (y : α) => Not (f y y)) (f x)) :=
  fun (h : Eq.{1} (Set α) (fun (y : α) => Not (f y y)) (f x)) =>
    (fun (npto : Not (f x x) -> f x x) =>
      (fun (ptnp : f x x -> Not (f x x)) =>
        absurd (f x x) False
          (npto (fun (p : f x x) => ptnp p p))
          (fun (p : f x x) => ptnp p p))
        (Eq.subst.{1} Prop (fun (Q : Prop) => Q -> Not (f x x))
          (Not (f x x)) (f x x)
          (congrArg.{1} (Set α) Prop (fun (A : Set α) => A x)
            (fun (y : α) => Not (f y y)) (f x) h)
          (fun (np : Not (f x x)) => np)))
      (Eq.subst.{1} Prop (fun (Q : Prop) => Q)
        (Not (f x x)) (f x x)
        (congrArg.{1} (Set α) Prop (fun (A : Set α) => A x)
          (fun (y : α) => Not (f y y)) (f x) h))
```
—— `courses/set-theory/units/unit10-cantor.sokonanoda:68-84`

**为什么好**：这是**真数学**——对角线法（Cantor 定理的全部内容）只用
`Eq.subst` + `congrArg` + `absurd` 就证完了，没有任何 tactic 魔法；
它证明"这门教学语言不是玩具"，同时这一整段在 `courses/set-theory/` 里只是
**已写好的演示**（练习在它后面，`courses/set-theory/units/unit10-cantor.sokonanoda:90` 起）。

**陪跑（同样值得展示）**：
`playground.sokonanoda:206-207` 的 `and_swap`（消去再构造的完整推理链）、
`courses/set-theory/lib/Set.sokonanoda:53-159` 的 `namespace Set` + 五个集合论记法、
`docs/gaps/repro/L03-eq-type-level.sokonanoda:62-63` 的宇宙多态
（`@Eq.{u+1} (Sort u) α β`，0.61.0 的层级算术 `u+1`）、
`courses/set-theory/units/unit12-synthesis.sokonanoda:97-130` 的关系复合结合律
（两个方向的 ∃ 拆装，期末项目那条链的第一环）。

---

## 附：本文的核实方式与未证实清单

- **核实方式**：所有 `file:line` 引用都来自本轮实际 `read`/`grep` 的仓库文件；
  §4.3 的 JSON 与 §6 的数字来自**真实运行**（0.61.0 二进制）与**仓库里的台账文件**，
  不是估算。计数命令一律写在对应小节里。
- **未证实清单**（不要在网站上当成事实写）：
  1. 「相对官方内核 10–100x」（`REQUIREMENTS.md:40` 的主张，仓库无测量，§6.6）；
  2. 0.60.0 / 0.61.0 的性能数字（台账最新只到 v0.59.0，§6.2）；
  3. `course/` 目录 98 次 `sorry` 里有多少是注释内出现（未逐行核，§5.4）；
  4. 集合字面量 `{a}`/`{a,b}` 在**课程材料**里的实际使用（只有测试源，§2.8）；
  5. `docs/architecture.md:519` 的未做清单里 "`match` tactic" 一条**已过期**
     （0.61.0 实测可用，§7.4 第 1 条）。


# 设计：Type 层重写（`Eq.rec` / `Eq.ndrec` / `Eq.mp` / `Eq.mpr` / `cast`）—— 冻结内核下的可行解（L-03，2026-09-19）

> 台账：`docs/gaps/ledger.jsonl` 的 **L-03**；复现件
> `docs/gaps/repro/L03-eq-type-level.sokonanoda`（**修后形状 ⇒ 干净判卷**）。
> 上游：`docs/design/prelude-l1-proposal.md`（L1 族与让位规则）、
> `docs/design/prop-large-elim-mirror.md`（派生 recursor 的 large-elimination 判据）、
> `docs/design/type-level-syntax.md`（层级算术 `u+1` 的白名单/语法面）。
> 姊妹篇：`docs/design/prop-cumulativity-boundary.md`（L-06：没有累积性）。
> **内核零改动**（硬规则 1）：0.60.0 的改动只在 `crates/front/src/compile/prelude.rs`
> 与 `error.rs`（只加码/hint）；0.61.0 追加 **parser + elab 的层级算术**
> （`parser.rs` 的 `parse_level_text`、`elab.rs` 的 `level_ptr`、`proof.rs` 的
> render 括号），内核仍是一个字节没动。
>
> **0.61.0 as-built 补记（2026-09-19，同轮）**：§4 的残留边界 1–3 已销账——
> ① 层级算术 `u+1` 落地（parser + elab，内核 `Level::Succ` 走既有的
> `EnvBuilder::succ`），`Eq.mp`/`Eq.mpr` 因此改成**宇宙多态**（签名与 Lean core
> 的 `Eq.mp`/`Eq.mpr` 逐字对齐）；② `cast` 装上（Lean core 里 `cast h a` 就是
> `Eq.mp h a`）；③ `Eq.ndrec` 装上（Lean core 的非依赖消去子）。B8 从 3 条扩到
> **5 条**，`PRELUDE_NAMES` 45 → **47**。仍不做的两条（4/5）在 §4 有实测理由。

## 0. 一句话

`Eq.subst` 的 motive 只能是 `α -> Prop`，所以**类型层重写**（`Eq.mp`/`Eq.mpr`/
`Eq.rec`）写不出来。实测结论分三层：**内核的规则是给的**（Eq 形状的 Prop 归纳块
确实大消去，`EqT.rec.{1}` 的 motive 落 `Type 0`）；**prelude 的 `Eq` 是公理**、
内核不为它派生消去子；**语法又不给两条现成路**（`inductive` 头部不吃宇宙 binder、
层级语法没有 `u+1`）。因此 B8 走**公理**：`axiom Eq.rec {u, v}`（签名与 Lean core
逐字同形）+ 由它定义的四条 `def`（`Eq.ndrec`/`Eq.mp`/`Eq.mpr`/`cast`），作为 L1 的
**B8 族**安装、按族让位。层级算术 `u+1` 落地后，后三条与 Lean core 的签名逐字对齐
（**宇宙多态**）。

## 1. 缺口与实测（先测量，再决定）

### 1.1 今天写不出来（0.59.0 及更早，实测）

| 写法 | 实测 |
|---|---|
| `Eq.rec.{u, 0} α a (fun (x : α) => Eq.{u} α x a) (Eq.refl.{u} α a) b h` | `elab-unknown-constant`：``unknown constant `Eq.rec` ``（exit 1） |
| 同上换 `Eq.ndrec` | `elab-unknown-constant`：``unknown constant `Eq.ndrec` `` |
| 台账的 repro：`def Eq.mp {u} (α : Sort u) (a b : α) (h : Eq.{u} α a b) : a -> b := fun (ha : a) => Eq.subst.{u} α (fun (x : α) => x) a b h ha` | `kernel-expected-sort`：`rejected: expected a sort, got: $3`（exit 1） |
| `Eq.subst` + Prop motive（`fun (x : α) => Eq.{u} α x a`） | ✅ checked（既有能力，B7 的 `Eq.symm` 就是这么定义的） |

### 1.2 内核**给**这个规则：Eq 形状的 Prop 归纳块大消去（实测）

`Eq` 是这一类里最著名的一个：**Prop 结果、单构造子、构造子没有自有字段**
（`Eq.refl` 的望远镜里只有参数 `α`/`a`，没有字段）。内核的 `large_elim_test_aux`
（`crates/kernel/src/inductive.rs:1164`）对它判 **true**（跳过参数后没有非 Prop 字段
⇒ 空集全称真），`mk_elim_level` 于是给 recursor 加一个消去宇宙。同形状的最小复现
（`α`/`a` 是**参数**、`a` 是结果的**索引**）：

```sokonanoda
inductive EqT (α : Type) (a : α) : α -> Prop
ctor EqT.refl : EqT α a a
end
#check EqT.rec.{1}
```

实测（exit 0）：

```
EqT.rec.{1}: forall (α : Type 0) (a : α) (motive : (forall (i : α), EqT α a i -> Type 0)),
motive a (EqT.refl α a) -> (forall (i : α) (target : EqT α a i), motive i target)
```

—— motive 落 **`Type 0`**，与官方 Lean 的 `Eq.rec` 同规则。真拿它做一次类型层搬运
也过（`axiom T : Nat -> Type` + `EqT.rec.{1} … (fun (i : Nat) => fun _ => T a -> T i) …`
⇒ `checked declaration tr`）。

**对照（别混）**：`Exists`（Prop 结果 + **数据字段** `w : A`）走的是
`large_elim_test_aux` 的"非 Prop 字段必须是结果实参"判据，`w` 不是参数也不是索引
⇒ 0 个宇宙参数、motive 只到 `Prop`：

```
$ sokonanoda … #check Exists.rec.{1}
{"code":"elab-universe-arity","message":"constant `Exists.rec` expects 0 universe argument(s), got 1"}
$ #check Nat.rec.{1}
Nat.rec.{1}: forall (motive : Nat -> Type 0), …        （Type 块大消去，对照）
```

所以 L-03 的根因**不是**"内核的 recursor 被钉在 `Sort 0`"——`Eq` 形状没有。
根因是下面两条。

### 1.3 根因一：prelude 的 `Eq` 是**公理**，不是归纳块

`PRELUDE_EQ_SRC` 是 `axiom Eq {u} : {α : Sort u} -> α -> α -> Prop`（+ `Eq.refl`/
`Eq.subst`）。公理没有构造子、没有 iota、没有派生 recursor——内核**没有任何**
`Eq` 特例（`crates/kernel/src` 里没有 `Eq.rec` 的痕迹），所以 `Eq.rec` 只能是
`unknown constant`。

### 1.4 根因二：两条"派生"路今天都堵着（实测）

1. **把 `Eq` 立成归纳块 ⇒ 拿不到宇宙多态。** `inductive` 头部不吃宇宙 binder：
   `inductive Exists {u} (A : Sort u) : Prop` ⇒ parse 失败
   （`inductive 参数 需要显式类型…found LBrace`）。monomorphic 的
   `inductive Eq (α : Type) …` 替换不了 prelude 的 `Eq.{u}`（课程与 L1 到处在用
   `Eq.{u} α a b`，`α : Sort u`）。
2. **把 `Eq.mp` 直接定义出来 ⇒ 层级语法没有 `u+1`。** `Eq.mp` 要的是
   `h : @Eq.{u+1} (Sort u) α β`：
   - `Eq.{u+1}` ⇒ parse 失败：`expected , or } in universe arguments, found Plus`；
   - 改成两个独立 binder（`{u, v}` + `@Eq.{v} (Sort u) α β`）⇒ 内核不合一：
     `类型不匹配：期望 Sort(v)，实际是 Sort(u + 1)`；
   - 省掉层级 ⇒ **不推断、默认 0**：`def probe (α : Type) (a b : α) (h : Eq α a b) …`
     ⇒ `类型不匹配：期望 Sort(0)，实际是 Sort(1)`。

   即：**宇宙多态的 `Eq.mp` 当时写不出来**（这是一条独立的、更小的缺口——层级算术
   `u+1`，本文当时只在 §4 记边界，不新开台账行）。
   **0.61.0 已销账**：层级算术落地（§4.1-1），`@Eq.{u+1}` 与 `Sort (u+1)` 都能写，
   上面三条实测全部反转（`Eq.mp` 改成宇宙多态，见 §2 的 as-built 源码）。

## 2. 决定（as-built）

**做**：L1 新增 **B8 族**（`crates/front/src/compile/prelude.rs`）：

```sokonanoda
axiom Eq.rec {u, v} : {α : Sort u} -> (a : α) -> (motive : (anon : α) -> Sort v) -> (ha : motive a) -> (b : α) -> (h : @Eq.{u} α a b) -> motive b
def Eq.ndrec {u, v} (α : Sort u) (a : α) (motive : α -> Sort v) (m : motive a) (b : α) (h : @Eq.{u} α a b) : motive b := @Eq.rec.{u, v} α a motive m b h
def Eq.mp {u} (α β : Sort u) (h : @Eq.{u+1} (Sort u) α β) : α -> β := @Eq.rec.{u+1, u} (Sort u) α (fun (x : Sort u) => α -> x) (fun (a : α) => a) β h
def Eq.mpr {u} (α β : Sort u) (h : @Eq.{u+1} (Sort u) α β) : β -> α := @Eq.rec.{u+1, u} (Sort u) α (fun (x : Sort u) => x -> α) (fun (a : α) => a) β h
def cast {u} (α β : Sort u) (h : @Eq.{u+1} (Sort u) α β) (a : α) : β := Eq.mp.{u} α β h a
```

（**0.61.0 as-built**：0.60.0 的头两条是 `def Eq.mp (α β : Type) (h : @Eq.{2} Type α β)`
的 Type 0 实例；层级算术落地后改成上面这条**宇宙多态**签名。`Eq.rec` 一个字节没改。）

四条理由：

1. **签名逐字同形**：`Eq.rec {u, v}` 的宇宙序、参数序、motive 形状与 Lean core 的
   `Eq.rec.{u, v}` 完全一致；`Eq.mp`/`Eq.mpr`/`cast` 是 Lean core 的
   `{α β : Sort u} (h : α = β)` 形状（`h` 即 `@Eq.{u+1} (Sort u) α β`），
   `Eq.ndrec` 是 Lean core 的 `abbrev Eq.ndrec.{u1, u2} {α : Sort u2} {a : α}
   {motive : α → Sort u1} (m : motive a) {b : α} (h : Eq a b) : motive b` 的
   显式实参版（宇宙参数顺序按本仓 `Eq.rec` 的 (α, motive) 写——两者都带名字，
   显式给全实参时无语义差别）。
2. **信任等级不变**：prelude 的 `Eq`/`Eq.refl`/`Eq.subst` **本来就是公理**
   （`Eq.refl` 也是公理！），B8 加的是同一族、同一等级的第四条公理；其余四条都是
   `def`（**不新增信任面**），不是"为了让某个定义过而伪造类型"（硬规则 4 禁止的是后者）。
3. **仓库里已有同形先例**：`examples/py-fol-core.sokonanoda:58` 就是这条公理，
   front 单测（`py_eq_symm_trans_are_in_ported_core` 等）一直在用它
   （`@Eq.rec.{u, 0}`）——B8 只是把它从"用户自己写"变成"prelude 自带"。
4. **内核的规则背书**：§1.2 实测说明这条公理正是内核**自己**会为宇宙多态 `Eq`
   归纳块派生的东西（`EqT.rec.{1}`）；挡路的只是语法（§1.4），不是语义。

**让位（沿用 §2.2 的族规则）**：B8 的 `deps = ["EQ"]`（`Eq` 公理族被文件占用 ⇒
B8 一起让位），文件自己声明 `Eq.rec`/`Eq.ndrec`/`Eq.mp`/`Eq.mpr`/`cast` 任一 ⇒
**整族让位**（`cast` 是常见名字：文件要自带 `cast` 就得连 `Eq.mp` 一起自带——
族规则的老口径，实测见 §3 的 `eq_rec_family_yields_when_the_file_declares_it`）。
`Eq` prelude 与 B7（`Eq.symm`/`Eq.trans`/`congrArg`）**不受** B8 让位影响——实测：
文件声明 `axiom Eq.rec …` 后 `Eq.symm` 仍可用、`#check Eq.mp` 报
``unknown identifier `Eq.mp` ``。

**为什么放 L1（B8）而不是 `PRELUDE_EQ_SRC`**：`PRELUDE_EQ_SRC` 的安装器只吃
`axiom`（`install_eq_prelude` 对非公理 `panic!`），而 L1 的 `install_l1_command`
本来就同时支持 `axiom`/`def`/归纳块；更要紧的是让位与建议材料（`goals.rs`）**都按
`L1_FAMILIES` 走**，新族自动获得同一条规则，不必动 `goals.rs`。

## 3. 三件套（硬规则 3）

| 件 | 内容 |
|---|---|
| **prelude 源** | `PRELUDE_L1_SRC` 尾部五条（B8）+ `L1_FAMILIES` 的 B8 项 + `PRELUDE_NAMES` 补 5 条（42 → **47**） |
| **front 单测** | `eq_rec_transports_at_type_level`（`Vec.cast` 走 `Eq.rec.{1,1}`、`id_mp`/`id_mpr`/`id_cast` 走 `Eq.mp`/`Eq.mpr`/`cast`、`nd_symm` 走 `Eq.ndrec`，0 error）；`eq_mp_is_universe_polymorphic`（`{u} (α β : Sort u)` + `@Eq.{u+1} (Sort u) α β` + `Sort (u+1)`，并钉住裸写 `Eq.mp` 仍 u=0）；`eq_rec_family_yields_when_the_file_declares_it`（族让位 + B7 存活，声明 `cast` 也整族让位）；`prelude_names_match_installs` 计数 47 |
| **课程/消费者用例** | `docs/gaps/repro/L03-eq-type-level.sokonanoda`：`scripts/soko grade` **exit 0 · 10 `decl.checked` · 0 diagnostic**（0.60.0 是 5 条，扩到多态 `Eq.mp`/`cast`/`Eq.ndrec` 后 10 条） |
| **语法白名单（层级算术）** | `docs/design/type-level-syntax.md` §5（`u+1` 的语法面与两条边界）+ parser 单测 3 条 + 课程单元⑤注释（`course/unit5-universes-sort.sokonanoda` 与 `course/en/` 镜像，注释不改 golden 计数） |

不改内核，不改 CLI 协议（新名字只出现在 `PRELUDE_NAMES` 的补全材料里，不产生新事件）。

## 4. 已落 / 仍不做（都有实测）

### 4.1 已落（0.61.0）

1. **层级算术 `u+1`**（parser + elab，**内核零改动**）：`Sort (u+1)`、`Sort u+1`、
   `Type (u+1)`、`Eq.{u+1}`、`@Eq.rec.{u+1, u}` 全部可用——`parser.rs` 的
   `parse_level_text`（原子 (`+` 数字)*，括号可省）产出层级文本，`elab.rs` 的
   `level_ptr` 用既有的 `EnvBuilder::zero/succ/level_param` 翻译成内核层级
   （白名单与边界见 `docs/design/type-level-syntax.md` §5）。
2. **`Eq.mp`/`Eq.mpr` 改成宇宙多态**：`{u} (α β : Sort u) (h : @Eq.{u+1} (Sort u) α β)`，
   与 Lean core 的 `def Eq.mp {α β : Sort u} (h : α = β) (a : α) : β` 逐字对齐。
   **这是一次签名变更**（§4-1 的老账）：Type 0 的调用形状从 0.60.0 的
   `Eq.mp α β h` 变成 `Eq.mp.{1} α β h`——本语言不给隐式实参、也不给宇宙推断，
   裸写 `Eq.mp` 仍按 u = 0 实例化（与 `Eq.symm`/`Eq.rec` 同一条既有规则，实测：
   `Eq.mp A A h` ⇒ `类型不匹配：期望 Sort(0)，实际是 Sort(1)`）。
3. **`cast` 装上**：Lean core 里 `cast h a` 就是 `Eq.mp h a`（`h.rec a`），
   本语言按同一条定义（`def cast {u} … := Eq.mp.{u} α β h a`）。硬规则 3 的精神
   是"真 Lean 代码要能直接编"，同义名重复的代价小于编不过的代价。
   `crates/front/src/compile/tests.rs` 里把 `cast` 当自定义公理名的
   `checks_axiom_with_two_universe_params` 改成 `transport`（它测的是"两个宇宙参数"，
   用 prelude 名字会被族让位吞掉）。
4. **`Eq.ndrec` 装上**：Lean core 的 `abbrev Eq.ndrec.{u1, u2} {α : Sort u2} {a : α}
   {motive : α → Sort u1} (m : motive a) {b : α} (h : Eq a b) : motive b`，本语言写
   `def Eq.ndrec {u, v} (α : Sort u) (a : α) (motive : α -> Sort v) (m : motive a)
   (b : α) (h : @Eq.{u} α a b) : motive b := @Eq.rec.{u, v} α a motive m b h`
   （显式实参风格下的同一签名；`def` 而非 `axiom`，不新增信任面）。

### 4.2 仍不做（**明确不做**，都有实测）

1. **不把 `Eq` 改成归纳块**：那会丢掉宇宙多态（§1.4-1：`inductive` 头部今天不吃
   宇宙 binder，`inductive Exists {u} (A : Sort u) : Prop` ⇒ parse 失败），且牵动
   `Eq.subst`/`Eq.refl` 的形态与全部既有 golden——收益为零（B8 已经拿到消去子，
   §1.2 实测内核会为 Eq 形状派生的正是这条公理）。
2. **不做累积性（cumulativity）**：它是**内核性质**（`Sort u : Sort (u+1)` 的
   子类型/包含关系），prelude 与 front 都碰不到——硬规则 1 冻结内核，这条留在
   L-06（`docs/design/prop-cumulativity-boundary.md`），本设计不碰。
3. **层级加法只收数字后缀**（不做 `max u v` / `u+v`）：内核的 `Level::Max`/`IMax`
   在 `EnvBuilder` 上没有公开构造入口（只有 `zero`/`succ`/`level_param`），
   做它必须动内核（硬规则 1）。parser 对 `u+v` 报专用诊断
   （"层级加法只收数字后缀"），不静默吞。
4. **`Type u` 仍不支持**（只支持 `Type n` / `Type (层级)`）：`Type` 后跟**裸标识符**
   必须保持**应用**语义——`Eq.refl.{2} Type A` 这类"`Type` 作实参、紧跟另一个实参"
   的既有写法依赖它（实测：改成层级会把它读成 `Type A` 一个实参）。要写 `Sort u`。
5. **by 引擎的宇宙参数不在本轮**：`by.rs::spec_of` 的判定合成声明带
   `universe: Vec::new()`，所以**目标里出现宇宙变量**时 tactic 判定失败
   （实测：`def f {u} : Sort u -> Sort u := by intro x; exact x` ⇒
   ``universe variable `u` is not declared in this declaration``；
   `Sort (u+1)` 继承同一条，报 ``unknown universe level `u+1` ``）。
   这是 **0.60.0 就有的既有边界**，不是层级算术引入的；`suggest`（快速修复）
   与 `judge_terms` 走 `DeclState.universe`，不受影响。修它要另立设计
   （把声明宇宙参数穿到 by 引擎的判定规格里）。
6. **不新增 `-- sokonanoda:prelude` 档案**：B8 与 B1–B7 同族让位，够用。

## 5. 影响面

| 面 | 影响 |
|---|---|
| `PRELUDE_NAMES` | 42 → 45（0.60.0）→ **47**（0.61.0：+`cast`/`Eq.ndrec`；补全/材料列表，CLI/LSP 自动跟随） |
| `goals.rs` 建议材料 | 自动获得 B8 的模板（`Eq.rec` 走 axiom 模板、其余四条走 def 模板）——按 `L1_FAMILIES` 的让位规则，**零改动** |
| 语法面 | 新增**层级算术**（`u+1`）：parser + elab + render 括号；三件套之白名单 = `docs/design/type-level-syntax.md` §5，课程 = 单元⑤注释 |
| 既有文件 | `examples/py-fol-core.sokonanoda` 自己声明 `Eq`（整族让位）⇒ 一字不改、36 checked（实测）；`playground.sokonanoda` 锚点 exit 0（实测） |
| golden | 受信任安装**不产生事件**，所有课程/事件计数不变（实测：`cargo test --workspace --locked` **exit 0**、课程门禁 `python3 courses/set-theory/tools/check.py` **exit 0 · 36 目标 · 329 checked · 99 open · 0 判负**，与 0.60.0 逐字相同） |
| 文档 | 本文 + `prelude-l1-proposal.md` 的 B8 补记 + `type-level-syntax.md` §5 + `docs/protocol.md` 无需改（不产生新错误码） |

## 附录：本文引用的原始输出

```
# 0.59.0（B8 之前）
$ sokonanoda query check --file <Eq.rec 用法>
{"code":"elab-unknown-constant","message":"unknown constant `Eq.rec`", …}      exit 1
$ sokonanoda --json <Eq.subst + Type motive>
{"code":"kernel-expected-sort","message":"rejected: expected a sort, got: $3"}  exit 1
$ sokonanoda --json <Eq.{u+1} …>      # 层级算术之前
parse：expected , or } in universe arguments, found Plus                     exit 1

# 0.60.0（B8 之后）
$ scripts/soko grade docs/gaps/repro/L03-eq-type-level.sokonanoda
exit 0 · 5 × "type":"decl.checked" · 0 × "type":"diagnostic"

# 0.61.0（层级算术 + B8 扩族之后）
$ scripts/soko grade docs/gaps/repro/L03-eq-type-level.sokonanoda
exit 0 · 10 × "type":"decl.checked" · 0 × "type":"diagnostic"
$ sokonanoda --json <Eq.mp A A (Eq.refl.{2} Type A)>      # 裸写 = u 0（既有规则）
{"code":"kernel-rejected","message":"类型不匹配：期望 `Sort(0)`，实际是 `Sort(1)`"}   exit 1
$ sokonanoda --json <def f {u} : Sort u -> Sort u := by intro x; exact x>
{"code":"elab-tactic-failed","message":"`exact` 判定失败：universe variable `u` is not declared in this declaration"}   exit 1
```

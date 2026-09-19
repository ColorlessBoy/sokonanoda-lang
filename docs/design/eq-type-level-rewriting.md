# 设计：Type 层重写（`Eq.rec` / `Eq.mp` / `Eq.mpr`）—— 冻结内核下的可行解（L-03，2026-09-19）

> 台账：`docs/gaps/ledger.jsonl` 的 **L-03**；复现件
> `docs/gaps/repro/L03-eq-type-level.sokonanoda`（**修后形状 ⇒ 干净判卷**）。
> 上游：`docs/design/prelude-l1-proposal.md`（L1 族与让位规则）、
> `docs/design/prop-large-elim-mirror.md`（派生 recursor 的 large-elimination 判据）。
> 姊妹篇：`docs/design/prop-cumulativity-boundary.md`（L-06：没有累积性）。
> **内核零改动**（硬规则 1）：本文全部改动在 `crates/front/src/compile/prelude.rs`
> 与 `error.rs`（只加码/hint）。

## 0. 一句话

`Eq.subst` 的 motive 只能是 `α -> Prop`，所以**类型层重写**（`Eq.mp`/`Eq.mpr`/
`Eq.rec`）写不出来。实测结论分三层：**内核的规则是给的**（Eq 形状的 Prop 归纳块
确实大消去，`EqT.rec.{1}` 的 motive 落 `Type 0`）；**prelude 的 `Eq` 是公理**、
内核不为它派生消去子；**语法又不给两条现成路**（`inductive` 头部不吃宇宙 binder、
层级语法没有 `u+1`）。因此 B8 走**公理**：`axiom Eq.rec {u, v}`（签名与 Lean core
逐字同形）+ 由它定义的 `Eq.mp`/`Eq.mpr`（Type 0 实例），作为 L1 的 **B8 族**
安装、按族让位。

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

   即：**宇宙多态的 `Eq.mp` 今天写不出来**（这是一条独立的、更小的缺口——层级算术
   `u+1`，本文只在 §4 记边界，不新开台账行）。

## 2. 决定（as-built）

**做**：L1 新增 **B8 族**（`crates/front/src/compile/prelude.rs`）：

```sokonanoda
axiom Eq.rec {u, v} : {α : Sort u} -> (a : α) -> (motive : (anon : α) -> Sort v) -> (ha : motive a) -> (b : α) -> (h : @Eq.{u} α a b) -> motive b
def Eq.mp (α β : Type) (h : @Eq.{2} Type α β) : α -> β := @Eq.rec.{2, 1} Type α (fun (x : Type) => α -> x) (fun (a : α) => a) β h
def Eq.mpr (α β : Type) (h : @Eq.{2} Type α β) : β -> α := @Eq.rec.{2, 1} Type α (fun (x : Type) => x -> α) (fun (a : α) => a) β h
```

四条理由：

1. **签名逐字同形**：`Eq.rec {u, v}` 的宇宙序、参数序、motive 形状与 Lean core 的
   `Eq.rec.{u, v}` 完全一致；`Eq.mp`/`Eq.mpr` 是它的 Type 0 实例（Lean 里这两个名字
   也是"沿类型等式搬运"，只是多态）。
2. **信任等级不变**：prelude 的 `Eq`/`Eq.refl`/`Eq.subst` **本来就是公理**
   （`Eq.refl` 也是公理！），B8 加的是同一族、同一等级的第四条公理；不是"为了让某个
   定义过而伪造类型"（硬规则 4 禁止的是后者）。
3. **仓库里已有同形先例**：`examples/py-fol-core.sokonanoda:58` 就是这条公理，
   front 单测（`py_eq_symm_trans_are_in_ported_core` 等）一直在用它
   （`@Eq.rec.{u, 0}`）——B8 只是把它从"用户自己写"变成"prelude 自带"。
4. **内核的规则背书**：§1.2 实测说明这条公理正是内核**自己**会为宇宙多态 `Eq`
   归纳块派生的东西（`EqT.rec.{1}`）；挡路的只是语法（§1.4），不是语义。

**让位（沿用 §2.2 的族规则）**：B8 的 `deps = ["EQ"]`（`Eq` 公理族被文件占用 ⇒
B8 一起让位），文件自己声明 `Eq.rec`/`Eq.mp`/`Eq.mpr` 任一 ⇒ **整族让位**。
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
| **prelude 源** | `PRELUDE_L1_SRC` 尾部三条（B8）+ `L1_FAMILIES` 的 B8 项 + `PRELUDE_NAMES` 补 3 条（42 → **45**） |
| **front 单测** | `eq_rec_transports_at_type_level`（`Vec.cast` 走 `Eq.rec.{1,1}`、`id_mp`/`id_mpr` 走 `Eq.mp`/`Eq.mpr`，0 error）；`eq_rec_family_yields_when_the_file_declares_it`（族让位 + B7 存活）；`prelude_names_match_installs` 计数 45 |
| **课程/消费者用例** | `docs/gaps/repro/L03-eq-type-level.sokonanoda`：`scripts/soko grade` **exit 0 · 5 `decl.checked` · 0 diagnostic** |

不改 parser 白名单（B8 零新语法），不改内核，不改 CLI 协议（新名字只出现在
`PRELUDE_NAMES` 的补全材料里）。

## 4. 残留边界（**明确不做**，都有实测）

1. **`Eq.mp`/`Eq.mpr` 是 Type 0 实例**，不是宇宙多态。原因是层级语法没有 `u+1`
   （§1.4-2）。**通用的 `Eq.rec` 不受影响**（它在任意层级都可用，repro 的
   `Vec.cast` 就是 `.{1, 1}`）。若将来做层级算术，`Eq.mp`/`Eq.mpr` 改成多态是
   **一次签名变更**（另立设计）。
2. **不装 `cast`**：Lean core 里 `cast h a` 就是 `Eq.mp h a`（同义名）。本语言
   不装同义名（两个名字一个意思会让补全与教学重复），且 `cast` 已被 front 单测
   （`checks_axiom_with_two_universe_params`）当作自定义公理名使用。需要时写
   `Eq.mp`。
3. **不装 `Eq.ndrec`**：它是 `Eq.rec` 的参数重排版本（Lean core 里的便利名），
   本语言显式实参风格下多一个同义名收益很小；要就写 `@Eq.rec.{u, v} …`。
4. **不把 `Eq` 改成归纳块**：那会丢掉宇宙多态（§1.4-1），且牵动
   `Eq.subst`/`Eq.refl` 的形态与全部既有 golden——收益为零（B8 已经拿到消去子）。
5. **不新增 `-- sokonanoda:prelude` 档案**：B8 与 B1–B7 同族让位，够用。

## 5. 影响面

| 面 | 影响 |
|---|---|
| `PRELUDE_NAMES` | 42 → 45（补全/材料列表；CLI/LSP 自动跟随） |
| `goals.rs` 建议材料 | 自动获得 B8 的模板（`Eq.rec` 走 axiom 模板、`Eq.mp`/`Eq.mpr` 走 def 模板）——按 `L1_FAMILIES` 的让位规则，**零改动** |
| 既有文件 | `examples/py-fol-core.sokonanoda` 自己声明 `Eq`（整族让位）⇒ 一字不改、36 checked（实测）；`playground.sokonanoda` 锚点 exit 0（实测） |
| golden | 受信任安装**不产生事件**，所有课程/事件计数不变（实测：front **561** 条单测全绿、`cargo test -p sokonanoda-cli -p sokonanoda-lsp` exit 0、课程门禁 `python3 courses/set-theory/tools/check.py` **exit 0 · 36 目标 · 329 checked · 99 open · 0 判负**，与 B8 之前逐字相同） |
| 文档 | 本文 + `prelude-l1-proposal.md` 的 as-built 补记 + `docs/protocol.md` 无需改（B8 不产生新错误码） |

## 附录：本文引用的原始输出

```
# 0.59.0（B8 之前）
$ sokonanoda query check --file <Eq.rec 用法>
{"code":"elab-unknown-constant","message":"unknown constant `Eq.rec`", …}      exit 1
$ sokonanoda --json <Eq.subst + Type motive>
{"code":"kernel-expected-sort","message":"rejected: expected a sort, got: $3"}  exit 1

# 0.60.0（B8 之后）
$ scripts/soko grade docs/gaps/repro/L03-eq-type-level.sokonanoda
exit 0 · 5 × "type":"decl.checked" · 0 × "type":"diagnostic"
```

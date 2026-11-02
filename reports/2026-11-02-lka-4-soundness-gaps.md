# LKA 语料 4 条「该拒却接受」定根因（只读）

- 分支 `strict/2026-10-29-kernel-app` · HEAD `31fbf072`（未改任何源码/测试/课程）
- 语料：`target/arena/cache/`（`leanprover/lean-kernel-arena` master `d7f74a9e6`，run 41308279557，good 127 / bad 80）
- 构建身份：`target/release/sokonanoda-kernel` = **b4b9098a4ebc4591**（`target/release/sokonanoda` = 76b87ff3d885f0b5）
- 四条我自己复核过判定（同一构建）：
  ```
  bad/bugs/closure-level-subst             => exit=0  Checked 14 declarations with no errors
  bad/bugs/nested-name-capture             => exit=0  Checked 39 declarations with no errors
  bad/bugs/unit-rec-rule                   => exit=0  Checked 7 declarations with no errors
  bad/other/inductive-undeclared-universe  => exit=0  Checked 7 declarations with no errors
  ```
- **Lean 侧引用的边界（先说清）**：本机 `~/Documents/lean/lean4` 是 **v4.12.0-rc1-5351**，
  该 checkout **没有** `src/Lean/Kernel/**`（`ls` = No such file），上游 master 的 tree 里也只有
  `src/kernel/*.cpp`（我核过 API tree：`src/kernel/inductive.cpp`、`type_checker.cpp` …，
  无 `src/Lean/Kernel`）。所以下面的 Lean 位置一律引 **`src/kernel/*.cpp` 的行号**（规则与 4.34/4.35
  同源：arena 语料是 lean4export 导出的**内核输入**，内核规则就是这套 C++ 规则）。
  凡是我**没有**钉死到最后一格的，我写明「未钉死」而不是猜。

---

## 1. `bugs/closure-level-subst` —— 闭包比较不看**宇宙代换**

### 1.1 这条用例在说什么

文件里没有任何归纳类型、没有公理，全是 def/theorem：

```
def FalseTy : Prop := ∀ P : Prop, P
def TrueTy  : Prop := ∀ P : Prop, P → P
def trueProof : TrueTy := fun _ p => p
def A : Type := Prop → Prop → Prop
def seed : A := fun x _ => x
def Family.{u} : Sort (max 1 (u+1)) := A → Sort u
def good  : Family.{1} := fun _ => A
def lower : Family.{0} := good          -- ← 唯一不成立的声明
def lowA  : Prop := lower seed          -- lower seed : Sort 0 = Prop
def a : lowA := fun x _ => x
def b : lowA := fun _ y => y
def D (f : lowA) : Prop := f TrueTy FalseTy
def pa : D a := trueProof
theorem boom : D b := pa                -- := trueProof : TrueTy，被当成 D b
```

`good : A → Type`，而 `Family.{0} = A → Sort 0 = A → Prop`；`lower := good` 把前者当后者用。
两个函数类型是**同一个函数体** `A → Sort u`，在**两个空环境**里分别带 `u:=1` / `u:=0`。
一旦判等，`lowA` 归约成 `A`，`a`/`b` 是两个不同的函数，`D b` 归约成 `FalseTy` ⇒ `boom : False`。
必须拒的原因：这就是**在类型层把 `Type` 与 `Prop` 判等**，判等器一旦给出 true，后面每一步都"成立"。

### 1.2 Lean 4 在哪拒绝它

- 判等的最后一道：`~/Documents/lean/lean4/src/kernel/type_checker.cpp:1047` `type_checker::is_def_eq_core`。
  Lean 的闭包/惰性求值把宇宙实例**烘进表达式**（level 没有"环境里的代换"这一层），
  两个 `Sort` 的比较走 `src/kernel/level.h:139` `is_equivalent(lhs, rhs)`（`level.cpp` 里 `is_equivalent`
  是**结构等价**）⇒ `Sort 1` 与 `Sort 0` 永远不相等，`good` 与 `Family.{0}` 无处可逃。
- 声明层：`type_checker::check` 用 `is_def_eq(infer_type(value), declared_type)`（同一函数族），
  所以 `lower` 在 `ensure_type`/`is_def_eq` 这一步就红。

### 1.3 我们的内核在哪 —— **有这段判等，但漏了一个条件**

- 漏的条件在 `crates/kernel/src/conv.rs:42-66` 的 `envs_ptr_equal`：
  ```rust
  (Env::Nil { .. }, Env::Nil { .. }) => return true,   // ← conv.rs:50
  ```
  **两个 `Env::Nil` 一律相等，`lsub`（宇宙代换）连看都不看**。而 `Env::Nil` 正是"空环境 +
  宇宙代换"的载体：`crates/kernel/src/eval.rs:113-120` `lsub_base` 明确
  `Env::Nil { lsub, hash: key }`。`crates/kernel/src/value.rs:261` 的 env hash **含** `lsub_key`，
  `value.rs:305` 就是 `lsub_key` —— 也就是说**工具都齐了，只有 `Nil` 这条捷径没用**。
- 短路点本身：`conv.rs:199-208`（`Pi`/`Pi`）与 `conv.rs:218-228`（`Lam`/`Lam`）的
  `bx.body == by.body && ptr_eq(dx,dy) && envs_ptr_equal(bx.env, by.env) && closure_ctxs_compatible(...)`
  —— 四个条件里**没有 `lsub`**。于是"同一个 body + 两个空 env（只是 `u` 不同）"⇒ 直接 true，
  正好是 arena yaml 描述的那句话（"takes two empty environments to be equal whatever their
  universe substitutions"）。

### 1.4 代价

| 路线 | 做法 | 规模 | 级别 |
|---|---|---|---|
| 修（判等） | `envs_ptr_equal` 的 `Nil` 臂加 `lsub` 比较（`lsub_key`/指针等价，`intern_level_sub` 已按 `(ks,vs)` 去重 ⇒ 指针可当内容用），并把两个 shortcut 也补 `lsub` 一致 | +5～15 行，只动 `conv.rs`（+可能 `value.rs` 一个 helper） | **中** |
| 拒判 | 这条**没有可判的语法形状**（不是"某个特性不支持"，而是判等算错），要么真修，要么在解释器/前端层面拒"def 的类型里出现 `Sort` 家族被强转"，等于重写一遍 | —— | **大** |

**碰热路径：是。** `conv.rs` 的 `unify` 是每次判等的必经之路，`(Pi,Pi)`/`(Lam,Lam)` shortcut 是里面
最热的两条捷径之一；加比较会让"同 body 不同 lsub"的闭包掉进慢路径（正确行为），
语料/课程里凡是跨 universe 的 def 展开都会多走一次全量比较 ⇒ **要量**（`docs/perf/ledger.jsonl`）。

### 1.5 风险

- 只会**多拒**、不会多接受 ⇒ 绿转红的唯一可能是"某条现在依赖这个错误判等"的输入；
  这类输入本身就是不成立的（和本条同形）⇒ 应该红。
- 真正要小心的是**性能**与**误拒**：若 `LevelSub` 在两次展开里内容相同但指针不同（可能跨
  `with_tc` 的新 arena），用指针判等会**误拒**。所以要么用 `lsub_key`（内容）比，要么确认
  `intern_level_sub` 的 arena 生命周期覆盖两侧。**这是本条最大的实施风险**。

---

## 2. `bugs/nested-name-capture` —— 递归子规则引用了**只在检查期存在**的名字

### 2.1 这条用例在说什么

`Shadow` 是嵌套归纳（`numNested: 1`），第二个构造子 `node : Cov Shadow`：

```
inductive Cov (α) where mk : ...           -- 文件里真实存在的归纳
inductive Shadow where base | node : Cov Shadow
```

检查嵌套归纳时内核会造**临时声明** `_nested.Cov_1`、`_nested.Cov_1.mk`、`_nested.Cov_1.rec`，
块检查完就没了。这条导出的 `Shadow.rec` 的 `node` 规则里，rhs **写的正是
`_nested.Cov_1.rec` / `_nested.Cov_1.mk`**（我渲染过：见 §附录 A），
而块之后文件又声明了一个**同名**的普通定义：

```
def _nested.Cov_1.rec.{v} : ... := fun ... => evil        -- evil : P B（一个假命题的"证明"）
def fold : Shadow → Pack := Shadow.rec ... good (fun _ ih => ih) ...
def sample : Shadow := Shadow.node (Cov.mk (fun _ => Shadow.base))
theorem boom : False := (fold sample).2                    -- 拿到 False
```

第二分量本该是 `False`，靠被"劫持"的规则变成 `evil`。
必须拒的原因：**被承认的规则不许提"只在块检查期存在"的常量**（yaml 原话：
"A checker must not let an admitted rule refer to a constant that existed only while the block was
checked"）。否则导出文件可以先用临时名把规则蒙过去、再在文件后面用同名 `def` 改写它的含义。

### 2.2 Lean 4 在哪拒绝它

- `src/kernel/inductive.cpp:828` `restore_nested(...)`：把 `_nested.*` 一族名字**逐点改写**回原名，
  别名映射由 `inductive.cpp:1088-1110` `mk_aux_rec_name_map` 给出（`_nested.X.rec ↦ <main>.rec_<i>`）。
- `src/kernel/inductive.cpp:1118-1176`（`environment::add_inductive` 的 nested 分支）：
  对每个 recursor **重建** `new_rules`（`rule.get_rhs()` 先 `restore_nested`），再
  `new_env.check_name(new_rec_name)` + `new_env.add_core(...)` —— 进环境的规则是**恢复后**的那份，
  里面不可能出现 `_nested.*`；导入的规则若写成临时名，与恢复后的规则对不上 ⇒ 拒。

### 2.3 我们的内核在哪 —— **有 restore 这一整条链，但最后一格没咬住**

链路齐全：`inductive.rs:2185-2223` `restore_and_check` → `2109-2145` `check_restored_recursor1`
→ `2074-2108` `restore_recursor1` → `1892-1972` `restore_replace` → `1973-2029` `replace_f`
（`replace_f` 的第一段就是 `_nested.X.rec ↦ X.rec_<i>` 的常量改写）；
映射表 `1830-1860` `mk_specialized_rec_to_unspecialized_map` 从**新环境的 `all_ind_names`** 建，
所以 `_nested.Cov_1.rec ↦ Shadow.rec_1` 这条**应该**在表里。
最后比较是 `assert_imported_expr_matches`（`inductive.rs:428-433`）→ `def_eq_core`。

那为什么还是过了？我能给出的硬事实是：

* 被承认的那份 `Shadow.rec` 的 `node` 规则**保留了** `_nested.Cov_1.rec`（附录 A 的渲染），
  且文件后面 `def _nested.Cov_1.rec` 真的把它劫持了（`boom` 能算出来就是证据）；
* `assert_nonnested_rec_rule_def_eq`/`check_restored_recursor1` 这两处**都会**panic（我实测过同族：
  把规则的 rhs 换成别的常量 ⇒ `imported recursor rule does not match the reconstructed rule`，
  `inductive.rs:430`）⇒ 检查**在跑**，只是这一侧被恢复成了**同名**，于是相等。

**缺的东西（可实施的定义）**：比较的一侧是"用**同一批确定性临时名**重建出来的规则"，
两侧自然相等 ⇒ 这条检查对"临时名捕获"是**空的**。两个可选修法（都在 `inductive.rs`，冷路径）：

* (A) `restore_and_check` 结束时加一条**名字卫生检查**：被承认的 `Shadow.rec`/`Shadow.rec_1` 的规则里
  不许出现"本块的临时名"（`mk_specialized_rec_to_unspecialized_map` 的 key 集合，**不是**所有
  `_nested.` 前缀）⇒ 出现就 panic（稳定消息，前端可分类）；
* (B) 把"重建的规则"真的按 Lean 的式子恢复（`Shadow.rec_1` 而不是 `_nested.Cov_1.rec`）再比 ——
  等价于修 `restore_e`/`replace_f` 在当前这条 rhs 上的落点。

**未钉死的一格（如实）**：`restore_f` 明明会改写 `Const(_nested.Cov_1.rec)`，为什么这条 rhs 没被改写
（是 `restore_replace` 的遍历没下到那层、还是 `quote` 出来的表达式已经丢了应用形状），
需要一次 3 行 `eprintln` 探针才能定；本次只读纪律不许加探针，留实施轮。
但**缺的检查**与上面 (A) 的定义无关：即使那格是 bug，也应当有一条"临时名不许出现在被承认的规则里"的
独立守卫（防守纵深）。

### 2.4 代价

| 路线 | 做法 | 规模 | 级别 |
|---|---|---|---|
| 修 | 上面 (A)：名字卫生检查（key 集合已在手）＋一条反向验证（把 `_nested.` 改成别的名字必须不再捕获） | +15～30 行，只动 `inductive.rs`（+`crates/kernel/tests/`） | **小～中** |
| 拒判 | 嵌套归纳**整类拒判**会把 good 语料里所有嵌套归纳打红（`List`/`Array` 嵌套很常见）⇒ 不可行；若只对"规则里出现块内临时名"拒判，那就是 (A) 的检测本身 ⇒ 检出即 `Decline` | 与 (A) 同 → 再 +烧 1 行 allowlist | **小** |

**碰热路径：否**（只在归纳块检查时跑一次，`numNested > 0` 才进）。

### 2.5 风险

- 对**合法**嵌套导出的误伤面在于 (A) 的"临时名集合"取法：只取 `_nested.*` 前缀会把用户自己
  起名 `_nested.foo` 的导出误拒；取 `mk_specialized_rec_to_unspecialized_map` 的 key 集合才精确
  ⇒ 风险 **低**。
- 语料侧：good 里的嵌套归纳导出（legit）规则本来就写恢复名，不会命中。

---

## 3. `bugs/unit-rec-rule` —— `is_unit_inductive` 的单边捷径，把**规则比较本身**吞掉了

### 3.1 这条用例在说什么

`PUnit` 按 prelude 那样声明（`inductive PUnit : Sort u`，`PUnit.unit`，`PUnit.rec`），
但 `PUnit.rec` 的 iota 规则被写成**返回构造子**而不是**返回 minor premise**：

```
PUnit.rec.{u_1,u} : {motive : PUnit → Sort u_1} → (unit : motive PUnit.unit) → (t : PUnit) → motive t
rule PUnit.unit (nfields = 0):  fun {motive} (unit) => PUnit.unit     -- 应为 fun {motive} (unit) => unit
```

于是

```
theorem badR : PUnit.rec.{1,0} (fun _ : PUnit => Prop) EmptyProp PUnit.unit := PUnit.unit
theorem boom : EmptyProp := badR
```

`badR` 的**类型**按这条规则归约成 `PUnit.unit`（一个项），而 `badR` 的值也是 `PUnit.unit`
（类型 `PUnit`）——一个**类型**和一个**项**被判等，于是 `boom : EmptyProp` 成立。
必须拒的原因：**导出给的 iota 规则不是权威**，内核必须自己推导并拒绝与之不同的规则
（yaml 原话："A checker must derive the rule itself and reject an export whose rule differs from it"）。
注意：我们**能拒**同族的 `bugs/nat-rec-rules`（带 field 的构造子），说明"缺的不是整条检查"。

### 3.2 Lean 4 在哪拒绝它

两层，且第二层是关键：

1. `src/kernel/inductive.cpp:705-748` `mk_rec_rules(...)`：Lean **自己推导** `comp_rhs`
   （`this_minor` 应用构造子参数），导入的规则只用来比对。
2. `src/kernel/type_checker.cpp:1035-1047` `is_def_eq_unit_like(t, s)`（`1118` 调用）——
   **只有在两个类型判等之后**才允许 unit-like 捷径：
   ```cpp
   if (ctor_val.get_nfields() != 0) return false;
   return is_def_eq_core(t_type, infer_type(s));   // ← 两边类型必须相等
   ```
   `PUnit.unit` 与 `motive PUnit.unit` 是**不同**类型 ⇒ Lean 不给这条捷径 ⇒ 规则比对失败 ⇒ 拒。

### 3.3 我们的内核在哪 —— **漏了 Lean 的最后一个条件**

`crates/kernel/src/conv.rs:587-605` `try_struct_eta`：

```rust
for ty in [xt, yt].into_iter().flatten() {        // 依次看**两边各自的**类型
    if let Value::Rigid { head: RigidHead::Inductive(ind_name, _), .. } = ty_f {
        if self.is_unit_inductive(ind_name) { return true; }   // ← conv.rs:594-595 直接 true，另一边不看
        ...
```

`is_unit_inductive`（`conv.rs:701-711`）= `get_structure(name, false)` + `num_fields == 0`，
与 Lean 的前半段等价；**缺的正是 Lean 那句 `is_def_eq_core(t_type, infer_type(s))`**。
后果是**单向、无条件**的：只要有一边的类型是 unit-like（`PUnit`/`Unit`/`True`/任意单构造子零域归纳），
**任意两个值**都判等。

这条捷径正是把 §3.2 的规则比较吞掉的那把钳子——两条实测证据（同一构建，附录 B）：

* `PUnit.rec` 的规则 rhs 改成 `fun {motive} (unit) => PUnit`（一个类型，非 unit-like 类型）
  ⇒ **拒**：`imported recursor rule does not match the reconstructed rule`（`inductive.rs:430`）
  ⇒ 规则检查**在跑**；
* 保持伪造规则、只把 `badR` 的值换成 `EmptyProp`（`: Prop`）⇒ **仍然接受**
  ⇒ `def_eq(Prop, PUnit.unit)` 返回了 **true**（`PUnit.unit` 的类型 `PUnit` 是 unit-like ⇒ 捷径生效）。
* 把规则改成**正确**的 `fun {motive} (unit) => unit` ⇒ 立刻在 `badR` 上红
  （`def_eq failed: expected PUnit.rec… | actual PUnit`）⇒ **正确内核本来就会拒这个文件**。

所以：`inductive.rs:428-433` `assert_imported_expr_matches`（被 `1684-1711`
`assert_nonnested_rec_rule_def_eq` 与 `2138` 调用）**早已就位**，是被 `conv.rs` 的这条捷径
变成空操作。**这是一条独立于归纳路径的健全性漏洞**（任何 unit-like 命题的证明等于任何东西）。

### 3.4 代价

| 路线 | 做法 | 规模 | 级别 |
|---|---|---|---|
| 修 | `try_struct_eta` 的 `is_unit_inductive` 分支补上"另一边类型与它判等"（`is_def_eq_core(ty, other)`） | +3～8 行，只动 `conv.rs` | **小** |
| 拒判 | 没有廉价语法形状（"unit-like 类型"遍地都是，拒它等于拒 prelude）；只能靠修 | —— | **大** |

**碰热路径：是**（`unify_cold` 的末尾，每一次判等失败的最后一站都会走一次）。多出来的是一次
`value_type` + 一次类型判等 —— 但只有"类型是单构造子零域归纳"时才发生（语料里集中在
`Unit`/`PUnit`/`True` 上），**量级应可控**，仍须进 `docs/perf/ledger.jsonl` 量一遍。

### 3.5 风险

- 方向安全：只会**多拒**。现在能过的语料里若有人**依赖** `Unit` 判等（例如两条不同的 `Unit` 证明
  被当同一个）——那是 `Unit` 的 eta，本来就该由"两侧类型相同 ⇒ 单元素"给出，修完仍然成立。
- 唯一要盯的是 perf 与"`value_type` 在某些值上会 panic"（`is_prop_type` 那种 `expected a sort`）；
  建议按 Lean 的写法先取 `t_type`/`s_type` 再判，避免多一次推断。

---

## 4. `other/inductive-undeclared-universe` —— 归纳块的**宇宙参数守卫是死代码**

### 4.1 这条用例在说什么

```
inductive U : Sort u            -- levelParams: []（`u` 不是块的参数！）
  | a | b                       -- U.a U.b : U
  U.rec : ... → Sort u_2 ...    -- levelParams: []，只消除到 Prop
def P.{u} : Sort u := U         -- 这里才声明 u ⇒ 前面 U 的自由 u 被读成 P 自己的参数
theorem pa : P.{0} := U.a
theorem pb : P.{0} := U.b
```

`U` 的类型 `Sort u` 里的 `u` **不在块声明的 `levelParams` 里**（导出里 `types[].levelParams = []`）。
在 `u := 0` 处 `U` 变成有两个不同证明的**命题**，之后每条声明都在一个"宇宙不固定"的类型上判。
必须拒的原因：**归纳块（类型/构造子/recursor）里出现的每个宇宙参数都必须在该块的 `levelParams` 里**，
和 def/theorem/axiom 一个标准（yaml 原话；Lean 的报错是
`invalid reference to undefined universe level parameter 'u'`）。

### 4.2 Lean 4 在哪拒绝它

- `src/kernel/type_checker.cpp:76-81` `type_checker::check_level(level const & l)`：
  `get_undef_param(l, *m_lparams)` 命中就抛 `invalid reference to undefined universe level parameter '…'`
  —— 而 `check_level` 由 `infer` 的每个 `Sort`/`Const` 分支调用。
- 归纳块把 `m_lparams` 设成**本块**的 lparams 再逐条检查：
  `src/kernel/inductive.cpp:211-229` `check_inductive_types()` 里的 `tc().check(type, m_lparams)`
  （构造子同理走 `check_constructors()`，`inductive.cpp:413+`）。

### 4.3 我们的内核在哪 —— **守卫写好了，但归纳块拿到的是 `None`**

- 守卫本身：`crates/kernel/src/infer.rs:100-115`，两条
  `if let (Check, Some(info)) = (flag, self.declar_info) { assert!(self.ctx.all_uparams_defined(...),
  "universe parameter not declared by the current declaration") }`（`:101` / `:113`），
  判定函数 `level.rs:316-327` `all_uparams_defined`（`MVar` ⇒ false，也对齐 Lean）。
- **`declar_info` 只有简单声明会传**：`crates/kernel/src/tc.rs:134`
  `TypeChecker::new(..., Some(*d.info()), ...)`（`check_simple_declar`）；
  而归纳块走的是 `crates/kernel/src/util.rs:1003` `with_tc(… TypeChecker::new(..., None, …))`
  ⇒ `inductive.rs:90` 的 `tc.check_declar_info_v(d)` 里 `declar_info = None`
  ⇒ 上面两条 assert **一次都不会执行** ⇒ 整块（类型、构造子、recursor）没有任何宇宙参数检查。
- 实测（附录 C）：同一份文件里**追加**一条
  `def q : Sort u := U`（`levelParams: []`）⇒ 立刻
  `universe parameter not declared by the current declaration`（`infer.rs:101`）。
  也就是说：**守卫是活的，只是归纳块的入口没打开它**。

**缺什么**：`check_inductive_declar` 的入口要把"本块的 `DeclarInfo`"喂进去（或在块检查前显式对
`types[]`/`ctors[]`/`recs[]` 的 type 各跑一次 `all_uparams_defined`）——一行 `Some` + 可能的
"临时扩展里的类型"白名单。

### 4.4 代价

| 路线 | 做法 | 规模 | 级别 |
|---|---|---|---|
| 修 | `util.rs:1003`/`inductive.rs:90` 传 `Some(ind.info())`（或新增 `with_tc_declar_info`）＋对块内每条的 type 显式跑 `all_uparams_defined`；补一条复现件 | +10～25 行，动 `inductive.rs` + `util.rs`（+`tc.rs` 若加新入口） | **小** |
| 拒判 | 检出"块内有未声明宇宙参数"后返回 `Decline` 而不是 panic：与修**同一处检出**，只差末尾是 `panic!` 还是 `return Err(Decline)` ⇒ 再 +5 行 + 1 行 allowlist | ≈修的量 | **小** |

**碰热路径：否**（每块一次，且 `all_uparams_defined` 只走 level 树）。

### 4.5 风险

- 归纳块里**合法**引用"别的声明的宇宙参数"只可能来自**块内互递归类型**——而 Lean 里同一块的所有
  类型共享同一组 `levelParams`，所以按块判是对的。
- 真正的风险在 `specialize_nested` **造出来的**类型（`_nested.*`）：它们是内核自己生成的，可能
  引用块内参数但它们的 `DeclarInfo` 不是导入的 ⇒ 检查必须只针对**导入的** `types/ctors/recs`，
  不要扫临时扩展，否则会自伤合法嵌套归纳。**语料 good 里有嵌套归纳，这条要当心**（建议先跑
  `arena_tarball_tier` + 课程门禁）。

---

## 总表

| 用例 | Lean 拒绝它的位置（本机 v4.12 `src/kernel/`；规则同 4.34/4.35） | 我们的位置 | 缺什么 | 实现代价 | 拒判代价 | 风险 |
|---|---|---|---|---|---|---|
| `bugs/closure-level-subst` | `type_checker.cpp:1047 is_def_eq_core` + `level.h:139 is_equivalent`：宇宙实例是表达式的一部分，`Sort 1 ≢ Sort 0` | `conv.rs:42-66 envs_ptr_equal`（`:50` 两个 `Env::Nil` 无条件相等，不看 `lsub`）＋ `conv.rs:199-208`/`218-228` 的闭包捷径；`lsub` 在 `eval.rs:113-120`，键在 `value.rs:261/305` | `Env::Nil` 比较漏 `lsub`（宇宙代换） | **中**（5–15 行，**热路径**，要量 perf） | **大**（无语法形状可判） | 高：只会多拒，但可能**误拒**（`LevelSub` 指针 vs 内容）＋ perf |
| `bugs/nested-name-capture` | `inductive.cpp:828 restore_nested` + `1088-1110 mk_aux_rec_name_map` + `1118-1176` 用**恢复后**的规则重建 recursor | `inductive.rs:2185-2223 restore_and_check` → `2109-2145 check_restored_recursor1` → `2074-2108`/`1973-2029` restore → `428-433 assert_imported_expr_matches`；命名 `753-763 mk_unique_name` | 被承认的规则里**不许出现块内临时名**（现在的"重建规则"用了同一批确定性临时名 ⇒ 比较是空的） | **小～中**（15–30 行，冷路径） | **小**（同一检出 +1 行 allowlist） | 低（临时名集合取 `mk_specialized_rec_to_unspecialized_map` 的 key 就精确） |
| `bugs/unit-rec-rule` | `type_checker.cpp:1035-1047 is_def_eq_unit_like`（**两边类型必须判等**）＋ `inductive.cpp:705-748 mk_rec_rules` 自推规则 | `conv.rs:587-605 try_struct_eta`（`:594-595` 单边 `is_unit_inductive` ⇒ 直接 true）＋ `conv.rs:701-711 is_unit_inductive`；被吞掉的检查在 `inductive.rs:428-433` ← `1684-1711` | 单边捷径：**没有** `is_def_eq(t_type, s_type)` 这一步 ⇒ 规则比较变空操作（且是通用判等漏洞） | **小**（3–8 行，**热路径**，只在 unit-like 类型上多一次判等） | **大**（无廉价形状；unit-like 遍地） | 中：只会多拒；盯 perf 与 `value_type` panic |
| `other/inductive-undeclared-universe` | `type_checker.cpp:76-81 check_level`（`invalid reference to undefined universe level parameter`）← `inductive.cpp:211-229 check_inductive_types` 的 `tc().check(type, m_lparams)` | 守卫在 `infer.rs:100-115`，但归纳块 `declar_info = None`（`util.rs:1003`）⇒ `inductive.rs:90` 的检查是死代码 | 归纳块没打开宇宙参数守卫（`Some(info)` 没传） | **小**（10–25 行，冷路径） | **小**（同处检出，换 `Decline` + allowlist） | 低（只对导入的 types/ctors/recs 扫；别扫 `_nested.*` 临时扩展） |

### 哪条最便宜、哪条最贵

- **最便宜**：`other/inductive-undeclared-universe`（传一个 `Some(info)`，冷路径，风险最低，
  且守卫代码早就在、有现成的 panic 消息）。
- **最贵**：`bugs/closure-level-subst`（判等热路径 + 宇宙代换的指针/内容语义，既要 perf 又要防误拒；
  "拒判"这条路基本不存在）。
- 单看**改动行数**最便宜的是 `unit-rec-rule`（3–8 行），但它碰热路径、且修的是**通用判等**，
  所以"便宜"只在行数上；按"总代价"排序：宇宙参数 < 名字捕获 < unit-eta < 闭包 lsub。

### 同源：4 条里其实是 **2 个根 + 1 个独立缺口**

1. **判等（`conv.rs`）太宽** —— `closure-level-subst`（闭包 env 不看 `lsub`）与 `unit-rec-rule`
   （unit-like 单边捷径）是同一个子系统的**两个独立缺陷**，但**互相不蕴含**：修其中一个不会修掉另一个。
   `unit-rec-rule` 只是这条捷径的"可见后果"，它本身是通用漏洞（`Unit` 的证明等于任何东西）。
2. **"导入的 iota 规则必须等于内核自推的规则"这条检查是空的** —— `unit-rec-rule` 与
   `nested-name-capture` **同源**：两处检查（`assert_nonnested_rec_rule_def_eq`、
   `check_restored_recursor1`）都在跑，但都被"比较对象与导入侧同源"这件事废掉
   （一个被 unit-eta 判等吞掉、一个被同名临时名吞掉）。**修好 `conv.rs` 的 unit-eta，
   `unit-rec-rule` 立刻红；但 `nested-name-capture` 不会跟着好**，还要名字卫生那条。
3. **归纳块的宇宙参数守卫是死代码** —— `inductive-undeclared-universe` 独立，和上面两条无交集。

一句话：**2 条（closure / unit）在 `conv.rs` 判等，2 条（unit / nested）共享"规则比较是空的"，
但只有 `unit-rec-rule` 同时属于两边；`inductive-undeclared-universe` 是第三条独立缺口。**

### 关于"拒判"这条路的一个硬约束（实施前必须知道）

`crates/kernel/tests/arena.rs:400-405` `is_mismatch`：`Reject` 要求 `kernel_rejected == true`
（**`Declined` 不算**）。`bad/` 用例走 `arena_tarball_tier` 时唯一的合法"不判"通道是
`arena_known_unsupported.txt`：进表后判据变成"**decline 或 reject 都行，但绝不能 accepted**"
（`arena.rs:475-484`）。所以任何一条的"拒判最小做法"= **内核返回 `Decline` ＋ 在
`crates/kernel/tests/arena_known_unsupported.txt` 加一行**（那是个 4 行的 edits + 允许表清单变更，
需要用户点头，因为它是"我们不判这个形状"的公开声明）。

---

## 附录 A：`nested-name-capture` 的关键渲染（我的只读渲染器 `/tmp/render.py`）

```
type Shadow lparams=[] numParams=0 numIndices=0 numNested=1 isRec=True ctors=['Shadow.base','Shadow.node']
  ctor Shadow.node : (a : Cov Shadow) -> Shadow
rec Shadow.rec_1 ... rule ctor=Cov.mk ... rhs 尾部 = Shadow.rec.{u} motive_1 motive_2 base node mk (x x)
rec Shadow.rec  ... rule ctor=Shadow.node nfields=1
    rhs = ... (node a) ((((((_nested.Cov_1.rec.{u} motive_1) motive_2) base) node) mk) (_nested.Cov_1.mk (Cov.0 a)))
def _nested.Cov_1.rec.{v} : ... := fun ... => evil          ← 块之后同名劫持
def sample : Shadow := Shadow.node (Cov.mk (fun _ => Shadow.base))
def fold   : Shadow → Pack := Shadow.rec ... good (fun _ ih => ih) ...
thm boom   : False := (fold sample).2
```

## 附录 B：`unit-rec-rule` 的四个探针（同一构建 b4b9098a4ebc4591，输入都是 /tmp 的副本）

| 探针 | 改动 | 结果 |
|---|---|---|
| 原样 | —— | `exit=0`（接受） |
| r2 | 规则 rhs 改成 `fun {motive} (unit) => PUnit` | **拒**：`imported recursor rule does not match the reconstructed rule`（`inductive.rs:430`）⇒ 规则检查在跑 |
| fixed | 规则 rhs 改成正确的 `#0`（minor premise） | **拒**：`def_eq failed: expected PUnit.rec… | actual PUnit`（`infer.rs:320`）⇒ 正确内核本会拒这个文件 |
| a | 保持伪造规则，`badR` 的值改成 `EmptyProp` | **接受** ⇒ `def_eq(Prop, PUnit.unit) == true`（unit-eta 单边捷径） |

## 附录 C：`inductive-undeclared-universe` 的探针

| 探针 | 改动 | 结果 |
|---|---|---|
| 原样 | —— | `exit=0`（接受，7 declarations） |
| +def | 追加 `def q.{}(levelParams: []) : Sort u := U` | **拒**：`universe parameter not declared by the current declaration`（`infer.rs:101`）⇒ 守卫对 def 是活的，对归纳块是死的 |

## 附录 D：方法与边界（只读纪律）

- **没改任何仓库文件**（这份报告除外）；没碰 `crates/kernel/**`；没切分支、没 commit。
- 探针只用 `/tmp` 下的 ndjson/config 副本 + 已构建的 `target/release/sokonanoda-kernel`；
  没有重跑 185 秒的 `arena_tarball_tier`（父会话已实测），也没跑 Lean 工具链。
- 读渲染用的是 `/tmp/render.py`（lean4export 3.1 的 ndjson → 可读形式，纯只读脚本，不进仓库）。
- **未钉死**：`nested-name-capture` 的"恢复后为什么两侧仍同名"那一格（需 3 行探针）；
  其余三条的根因都有实测探针或代码路径闭环。

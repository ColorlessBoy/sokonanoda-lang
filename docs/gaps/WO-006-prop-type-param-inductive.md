# WO-006 Prop + Type 参数 + 单构造子 + 自有字段的 inductive 被内核断言拒绝（G-03）

> 台账条目：`docs/gaps/ledger.jsonl` 的 **G-03**（`kind=language`、`severity=blocker`、
> 2026-09-18 由 course-agent 登记）。
> 复现件：`docs/gaps/repro/G03-prop-type-param-inductive.sokonanoda`。
> 本文全部实测在 **0.58.0** 上跑出（`Cargo.toml:6`，HEAD `af737fc`，经 `scripts/soko`
> 解析到版本匹配的缓存二进制）。判定一律走 kernel，不引用官方 Lean 工具链（硬规则 2）。

## 用户可见症状 / 最小复现

- 复现命令（一条，可直接粘贴）：

  ```bash
  scripts/soko grade docs/gaps/repro/G03-prop-type-param-inductive.sokonanoda
  ```

- 今天的实际输出（0.58.0 实测，退出码 **1**；只贴关键行）：

  ```json
  {"type":"diagnostic","stage":"kernel","code":"kernel-rejected",
   "message":"rejected: assertion `left == right` failed\n  left: 1\n right: 0",
   "span":{"start":{"line":29,...},"end":{"line":31,...}}}
  ```

- 症状性质：**这不是教学错误消息**。`left/right` 是内核 `assert_eq!` 的 panic 载荷，
  被 front 包装成 `rejected:`（`crates/front/src/compile/error.rs:349-376`）。学习者看到的是
  内部断言、没有任何可行动的提示——这是它按 blocker 处理的理由。
- 复现件正文只有三行（`docs/gaps/repro/G03-prop-type-param-inductive.sokonanoda:29-31`）：

  ```
  inductive Bar (A : Type) : Prop
  ctor mk (a : A) : Bar A
  end
  ```

## 期望行为

- **官方 Lean 4**：这段完全合法——它就是 `Exists` 的形状（台账 `expected_lean` 逐字）：

  ```lean
  inductive Exists {α : Sort u} (p : α → Prop) : Prop where
    | intro (w : α) (h : p w)
  ```

  Prop 结果 + 单构造子 + 一个 Type 参数 + 自有字段里有一个 `α` 值。Lean 给它的 recursor
  motive 落在 `Prop`（该块**不**做 large elimination），不额外引入消去宇宙参数（只有归纳
  自己的 `u`，来自 `{α : Sort u}`）。本仓内核 `mk_elim_level`
  （`crates/kernel/src/inductive.rs:1243-1262`）对同一块给出的正是这个形状，而它来自上游
  快照——所以**"内核构造出来的 recursor"就是本 WO 的验收真值**。
- **本教学子集边界（本 WO 只做判据一致，不扩能力）**：
  - 不新增 large elimination 能力：能不能 large-eliminate 仍由内核那几行决定；本 WO 只让
    前端派生的 recursor 与它**一致**。
  - `Exists.{u}`（`{α : Sort u}`）今天写不出来 = **G-14**（单宇宙 binder），本 WO 不碰；
    课程用 `(α : Type)` 版就够（`courses/set-theory/lib/Exists.sokonanoda:43-46` 已论证）。
  - `∃` 记法 = **G-04**；构造子点名（`Exists.intro` 那种点号名）= **G-02**。两者都不在本
    WO 范围；本 WO 只要求"这个块能立起来，且派生的 recursor 与内核一致"。

## 机制：断言在哪、为什么（附一条待确认）

1. 内核先按上游语义算出该块该有什么 recursor：`large_elim_test`
   （`crates/kernel/src/inductive.rs:1200-1223`）→ 单构造子时进 `large_elim_test_aux`
   （`:1164-1197`）→ `mk_elim_level`（`:1243-1262`）：large-eliminate ⇒
   `rec_uparams = [u] ++ uparams`，否则 `rec_uparams = uparams`（本例 uparams 为空 ⇒ 0 个）。
2. 前端**自己派生** recursor（`crates/front/src/compile/elab.rs:2530-2854` `derive_recursor`），
   判据在 `:2537-2542`：`small_elim = is_prop_block_ty(ty) && constructors.len() > 1`；
   `false ⇒ universe = ["u"]`。G-03 形状里 `constructors.len() == 1`，于是前端声明了
   1 个宇宙参数，内核只要 0 个。
3. 内核把两者对拍：`check_inductive_declar`（`:11`）→ `assert_nonnested_recursors_def_eq`
   （`:1692-1706`）中 `subst_expr_levels(old.info().ty, old.info().uparams, st.rec_uparams)`
   ——左＝前端声明的 uparams 个数，右＝内核算出的 `st.rec_uparams` 个数；长度不等即
   `crates/kernel/src/expr.rs:381-392` 的 `assert_eq!(ks.len(), vs.len())` 炸掉。
4. **对称实测**（本轮，`/tmp` 探针）证明 3. 就是这条计数比较，而不是别的断言：
   把 recursor 手写成 `rec Bar.rec {u}`（1 个宇宙参数）→ 与派生路径**同一条** `left: 1 /
   right: 0`；把同一个 rec 写成 0 个宇宙参数、但块形状是**内核要求 1 个**的那种 → 
   `left: 0 / right: 1`（镜像）。两个方向合起来只有一种解释：左＝声明值、右＝内核算值。
5. **待确认**：panic 行号拿不到 backtrace —— front 把 panic hook 静音了
   （`crates/front/src/compile/check/mod.rs:659`，`set_hook(Box::new(|_| {}))`）。上面 3. 的
   落点由调用链 + 对称实测推出；实现者以"计数断言是否消失"作判据即可，不必复现 backtrace。
6. 结论：错的是**前端 `small_elim` 这个近似**，不是内核。今天 `constructors.len() > 1`
   只在"单构造子 Prop 且该构造子确实 large-eliminate"时与内核巧合一致。

## 二分钥匙：通过/失败边界对照（复现件注释的 5+1 已逐条复跑，另补 11 条）

复现件 `docs/gaps/repro/G03-prop-type-param-inductive.sokonanoda:16-22` 附了"5 条通过
1 条失败"。本轮把 5 条逐条复跑（结论与注释一致），并补测了内核判据的两侧边界。
**下表每一行都跑过**（语料见本节末；实测：16 条声明里 **13 checked / 3 failed**，
失败恰为 P1/P2/P3，退出码 1）。

| 形状要点 | 复现件注释 | 本轮实测 | 内核 `large_elim_test` ⇒ 要几个 uparam | 前端今天派生的 |
|---|---|---|---|---|
| **P1**＝复现件：`inductive P1 (A : Type) : Prop` + `ctor c1 (a : A) : P1 A` | ❌ 唯一失败形状 | ❌ `left:1/right:0` | false（自有字段 `a` 非 Prop 且不在结果实参里）⇒ **0** | 1 |
| **P2** `ctor c2 (a : A) (b : A) : P2 A`（两个非 Prop 自有字段） | —（新测） | ❌ 同一条断言 | false ⇒ **0** | 1 |
| **P3** `inductive P3 (A : Type) : A -> Prop` + `ctor c3 (a : A) (b : A) : P3 A a`（非 Prop 字段不全是索引） | —（新测） | ❌ 同一条断言 | false（`b` ∉ params+indices）⇒ **0** | 1 |
| **P4** `inductive P4 (A : Type) : Prop` + `ctor c4 : P4 A`（无自有字段） | ✅ | ✅ checked | true ⇒ 1 | 1 |
| **P5** `inductive P5 (P : Prop) : Prop` + `ctor c5 (p : P) : P5 P`（参数是 Prop） | ✅ | ✅ checked | true ⇒ 1 | 1 |
| **P6** `inductive P6 (A : Type) (P : A -> Prop) : Type` + `ctor c6 (a : A) (h : P a)`（结果是 Type） | ✅ | ✅ checked | `is_nonzero` ⇒ true ⇒ 1 | 1 |
| **P7** 两个构造子（`c7a (a : A)` / `c7b`） | ✅ | ✅ checked | 多构造子 ⇒ false ⇒ 0 | 0（small_elim） |
| **P8** `inductive P8 (n : Nat) : Prop` + `ctor c8 : P8 n`（参数非 Prop、无自有字段） | ✅ | ✅ checked | true ⇒ 1 | 1 |
| **P9** `inductive P9 (A : Type) : A -> Prop` + `ctor c9 (a : A) : P9 A a`（非 Prop 字段**正好是结果的索引**） | —（新测） | ✅ checked | **true**（`a` ∈ params+indices）⇒ 1 | 1 |
| **P10** `ctor c10 (h : P -> Q)`（字段类型是"Prop 值的箭头"） | —（must-not-regress） | ✅ checked | true（`P -> Q` 的**类型**是 Prop）⇒ 1 | 1 |
| **P11** `ctor c11 (f : forall (x : Nat), P)`（字段类型是 Prop 值的 Pi） | —（must-not-regress） | ✅ checked | true ⇒ 1 | 1 |
| **P12** `inductive P12 (A : Type) : Prop` + 无构造子（空 Prop 块） | —（新测） | ✅ checked | `[] => true` ⇒ 1 | 1 |
| **P13/P14** 字段类型是**具名** Prop 定义（`Named` / `Rel 0`） | —（must-not-regress） | ✅ checked | true ⇒ 1 | 1 |
| **R1** 显式 `rec Bar.rec {u}` 写在 P1 形状上 | —（新测） | ❌ `left:1/right:0` | 声明 1 vs 内核 0 | —（不走派生） |
| **R2** 显式 rec、0 宇宙参数，写在 P4 形状上 | —（新测） | ❌ `left:0/right:1` | 声明 0 vs 内核 1 | — |
| **R3** 显式 rec、0 宇宙参数，写在 P1 形状上 | —（新测） | ❌ **另一条**错误 `expected a sort in conversion, got: Pi (a : $0), (Bar.[] $1)` | **待确认**（计数已相等却仍在 conversion 不一致；疑 0 宇宙参数的显式 rec 另有缺陷——见「不做的事」） | — |
| **R4** `rec Bar.rec {} : …` | —（新测） | ❌ parse：`expected recursor type, found LBrace`（空宇宙列表写不出来） | — | — |

**由表得出的修复判据（三条，都可判定）**：

1. "要不要额外宇宙参数"必须**逐字镜像**内核 `large_elim_test`：Prop 块里**空构造子**或
   **多构造子**的既有行为不变；**单构造子**时才看 `large_elim_test_aux` 的结果。
2. **不能**用"字段类型语法上是不是 `Prop`"近似：P10/P11/P13/P14 今天是对的，近似会把它们
   从 1 个宇宙参数改成 0 个，立刻换成 R2 那样的 `left:0/right:1`。
3. **不能**用"字段数 vs 参数数"或"有没有索引"近似：P3 与 P9 是一对判别性形状（都"看起来
   像"同一类）；H6-C 的教训逐字写在 `crates/front/src/compile/elab.rs:2452-2456`。

用于对拍的语料（本轮原样跑过；构造子名必须唯一——今天裸名会 `duplicate declaration mk`，
那是 G-02，不是本 WO 的问题）：

```sokonanoda
def Named : Prop := forall (x : Nat), forall (y : Nat), Eq.{1} Nat x y
def Rel (n : Nat) : Prop := forall (m : Nat), Eq.{1} Nat m n
inductive P1 (A : Type) : Prop
ctor c1 (a : A) : P1 A
end
inductive P2 (A : Type) : Prop
ctor c2 (a : A) (b : A) : P2 A
end
inductive P3 (A : Type) : A -> Prop
ctor c3 (a : A) (b : A) : P3 A a
end
inductive P4 (A : Type) : Prop
ctor c4 : P4 A
end
inductive P5 (P : Prop) : Prop
ctor c5 (p : P) : P5 P
end
inductive P6 (A : Type) (P : A -> Prop) : Type
ctor c6 (a : A) (h : P a) : P6 A P
end
inductive P7 (A : Type) : Prop
ctor c7a (a : A) : P7 A
ctor c7b : P7 A
end
inductive P8 (n : Nat) : Prop
ctor c8 : P8 n
end
inductive P9 (A : Type) : A -> Prop
ctor c9 (a : A) : P9 A a
end
inductive P10 (A : Type) (P Q : Prop) : Prop
ctor c10 (h : P -> Q) : P10 A P Q
end
inductive P11 (A : Type) (P : Prop) : Prop
ctor c11 (f : forall (x : Nat), P) : P11 A P
end
inductive P12 (A : Type) : Prop
end
inductive P13 (A : Type) : Prop
ctor c13 (h : Named) : P13 A
end
inductive P14 (A : Type) : Prop
ctor c14 (h : Rel 0) : P14 A
end
```

## 范围

| 文件:行 | 事实（本轮读过） | 本 WO 的动作 |
|---|---|---|
| `crates/front/src/compile/elab.rs:2530-2555` | `derive_recursor` 的判据：`small_elim = is_prop_block_ty(ty) && constructors.len() > 1`；`universe` 与 `motive_sort` 都由它决定 | **改这里**（唯一判据源） |
| `crates/front/src/compile/elab.rs:2784-2789` | `RecDecl { universe: universe.clone(), … }` | 不改，自动跟随 |
| `crates/front/src/compile/elab.rs:2821` | iota 规则里的 `e_universe_app("<Ind>.rec", &universe, …)` | 不改，自动跟随 |
| `crates/front/src/compile/elab.rs:1663-1671` | `match` 编译：`info.rec_universe_arity == 0` 时用 0 级 recursor 常量 | 不改；修好后 G-03 形状走 0 级分支（验收覆盖） |
| `crates/front/src/compile/elab.rs:408,488` | `rec_universe_arity = recursor.universe.len()`，与判据同源 | 不改 |
| `crates/front/src/compile/elab.rs:350-360` | ctor 的**内核**望远镜已在手（`kernel_field_binders(ctor_ty)`），字段从 `params.len()` 之后开始 | 判据若搬到 elaborate 之后，就用这里 |
| `crates/front/src/compile/elab.rs:2436-2461` | `is_k_target`：**另一条**判据（镜像内核 `init_k_target`），已正确，有 3 条专门测试 | **不动**（别顺手改） |
| `crates/front/src/compile/elab.rs:2467-2490` | `is_prop_block_ty`：只判"结果排序是不是 Prop"，对 G-03 形状没错 | 不动 |
| `crates/kernel/src/inductive.rs:1164-1197` | `large_elim_test_aux`：跳过 `num_params` 个 binder，其余 domain 用 `is_prop_type` 判；非 Prop 的 domain 变量必须是 `ind + params + indices` 的子集 | **只读**（真值来源） |
| `crates/kernel/src/inductive.rs:1200-1223,1243-1262` | `large_elim_test` / `mk_elim_level`：决定 `rec_uparams` 与 `elim_level` | 只读 |
| `crates/kernel/src/inductive.rs:135,1692-1706` + `crates/kernel/src/expr.rs:381-392` | 对拍调用链与断言落点 | 只读 |
| `crates/front/src/compile/check/mod.rs:659` | panic hook 被静音（拿不到 backtrace） | 只读；解释了断言落点为何只能推断 |

**是否动内核：预期否。** 依据：内核是冻结快照，改动要进 `docs/architecture.md` §6 的清单
（该节逐条列了历史上允许的改动）；而本例的真值（large elimination 判据）属于上游语义，
`init_k_target`/`large_elim_test` 一行不改也能修——"前端与内核算得不一样、在前端修"的先例
正是 H6-C 的 `is_k`（前端曾恒 `false`，同样在前端修掉：测试
`crates/front/src/compile/tests.rs:4707-4780`、文档 `docs/TESTING.md:39`）。
若实现时发现**必须**动内核（例如前端确实拿不到字段类型的排序），先按
`docs/architecture.md` §6 走清单流程并在 WO 回复里说明理由，不要静默扩内核。

**修改策略（实现前先落 `docs/design/`，两条路都必须过上一节全部 case）**：

- **(A) 判据提前到 ctor elaborate 之后（推荐）**：`derive_recursor` 今天在 ctor 类型
  elaborate **之前**跑（调用点 `elab.rs:258-267`，而字段内核类型在 `:350-360`），而镜像
  `large_elim_test_aux` 需要"字段类型的排序"与"结果实参"这两样已 elaborate 的信息。把
  "选 `universe` / `motive_sort`"这一步挪到二者都在手之后，逐字镜像内核（含子集判据）。
- **(B) 保留顺序、给前端加一个可靠的"源码类型是否 Prop 值"查询**：必须能正确回答
  P10/P11/P13/P14（`P -> Q`、`forall (x : Nat), P`、具名 `def … : Prop`、具名 Prop 定义的
  应用），否则就是"用近似换另一个断言"。
- 两条路都**不要**做"试探 + 回退"（先按一支派生、被内核拒了再换另一支）：那等于把判据推给
  内核，会把真正的教学错误磨成同一条断言，也违反 §8 gotcha 0b 的契约（front 与内核的
  归纳块协作必须是**同规则镜像**，不是事后补救）。

## 不做的事（明确排除，防顺手扩大）

- 不实现 large elimination 相关的新能力/新语法；不改 `is_k_target`、`is_prop_block_ty`
  的既有语义（前者已正确且有测试，后者对本缺口形状没有误判）。
- 不碰 **G-14**（单宇宙 binder）→ 课程 `Exists` 升级后仍是 `(α : Type)` 版；
  不碰 **G-04**（`∃` 记法）；不碰 **G-02**（构造子命名空间，裸名照旧）。
- 不修显式 rec 的 0 宇宙参数形态（上表 **R3**：计数已相等却报
  `expected a sort in conversion`，**待确认**）。它今天不是绕行手段（R1/R2 都撞计数断言，
  R4 的 `{}` 直接 parse error），是否独立缺口由语言线复核后再登记；本 WO 只把它当**已排除
  的绕行**记录在案，验收不要求它变绿。
- 不给 `course/`（入门课）既有 11 个单元加/删练习、不改它们的诊断文本。
- 不顺带实现"自动 large elimination"或"Prop 块免 motive"之类的新语义。

## 兼容策略与同轮改动清单（课程侧）

本修法是**放宽前端派生判据**：不改名字、不改语法、不给今天能编过的程序换 recursor 形状
（论证见边界表：今天能过的那些块，前端与内核的判据本来就一致）。因此：

- **旧的裸构造子名保留**：本 WO 不引入任何改名。若与 **G-02**（构造子命名空间，WO-005）
  同轮落地，则 G-02 自己的兼容策略为准，本 WO 只提要求：`courses/set-theory/lib/Prod.sokonanoda`
  的 `prod_mk` 与 `course/` 里的裸名必须保留为别名（新增 `Type.ctor` 形式 ≠ 删掉裸名），
  同轮改动清单由 G-02 列。
- **课程侧收益（同轮或紧随其后的课程轮）**——这是 G-03 的真正出口：

  | 文件 | 动作 | 兼容约束 |
  |---|---|---|
  | `courses/set-theory/lib/Exists.sokonanoda:68-70` | 公理三件套 → 逐字写成 Lean 的 inductive（`Exists` + `intro` 构造子），`Exists.elim` 改由**自动派生的 `Exists.rec`** 定义 | `Exists` / `Exists.intro` / `Exists.elim` 的**名字与签名逐字不变**——文件头 `:20-24` 已承诺"units/ 里的证明文本一行都不用改"，这是验收判据 |
  | `courses/set-theory/lib/Exists.sokonanoda:11-29`（头注释） | 删掉"今天立不起来/公理是临时形态"的说明，改写为"0.59.0 起是归纳；`Exists.rec` 依赖消去可用" | 保留"裸名 `intro` 与 tactic 同名"（`:25-26`）的 G-02 提示 |
  | `courses/set-theory/units/unit06…unit12`（7 个单元） | **预期 0 改动**（`import lib.Exists` 与全部证明文本原样） | 升级后 `check.py` 必须仍 `failed: 0` 且 checked 不下降；任何一行要改都说明签名变了 → 回退设计 |
  | `docs/design/course-stdlib.md:97` | 表格里 G-03 那行的"`Exists` 只能公理"改为已修 | 与 `:64` 的分层规矩无关，别动 |
- **不能**让公理版与归纳版同名共存（同签名两个来源会让"哪个是真"不可判）；旧公理形态要留就
  留在历史/文档里，不留在库里。

## 验收（三层）

- **front 单测**（`crates/front/src/compile/tests.rs`，紧挨同族测试 `:4743` / `:4767`）：
  1. 复现形状（P1）`decl.checked`，且 `out.errors == vec![]`；
  2. **签名断言（关键）**：`#check P1.rec` 的 `CheckEvent::TypeChecked{text}` 里没有宇宙参数、
     motive 是 `(x : P1 A) -> Prop`。只断 `decl.checked` 不够——本缺口的本质是 recursor 的
     **形状**错了，形状对了事件才对；
  3. 表驱动跑上一节整份语料（`Named`/`Rel` + P1–P14，共 16 条声明）→ 全部 `decl.checked`
     （今天 13/16，修后 16/16）；
  4. 判别性反例单独点名（P3 与 P9 成对、P10/P11/P13/P14 不许回归），失败信息里写清
     "为什么它不是 K 目标/large-elim 判据"（照 `:4731-4741` 的注释风格）；
  5. 可选：`match` 消去（P1 形状 → Prop motive）能过，守住 `elab.rs:1663` 的 0 级分支。
- **CLI e2e**（`crates/cli/tests/cli.rs`，样式照 `cli_accepts_non_recursive_inductive_block`
  `:1047` / `cli_accepts_bool_with_auto_derived_recursor`）：
  1. 一条：复现形状 + `#check Bar.rec`，退出码 0 且输出含 `Prop`（顺便钉住 `--json` 不变）；
  2. 一条：同一块上一次 `match`/`#reduce` 的真实计算（证明派生 recursor 不只是"声明得过"）。
- **课程用例**（`courses/set-theory/` 里真实存在的文件与练习）：
  - 课程仓 **12 个单元**全部已写（`units/unit01-sets-membership…unit12-synthesis`），其中
    **7 个 `import lib.Exists`**（unit06–unit12）今天拿到的是本缺口的公理替身；
  - 语言线这一轮必须至少让**一个单元级用例**在 CI 语料里被覆盖：建议
    `courses/set-theory/units/unit10-cantor.sokonanoda` 的练习 4 **`cantor`**（`:124`）与
    练习 6 **`no_surjection_powerset`**（`:146`）——它们正是台账 `blocks` 点名的"Cantor /
    基数 Ⅱ"，两次 `Exists.elim` + 一次 `Exists.intro`；次选
    `courses/set-theory/units/unit08-images-preimages.sokonanoda` 的 **`image_union`**（`:101`）；
  - 课程门禁（本轮实测基线）：`python3 courses/set-theory/tools/check.py --json` →
    `{"schema":"soko.course.check/1","units":12,"failed":0}`，34 个 target、
    **296 checked / 93 open**，其中 unit10 = 4 checked / 6 open。升级 lib 后必须仍
    `failed: 0` 且 checked 不下降；
  - 升级后新得到的真东西：`Exists.rec`（motive 可变 ⇒ 结论可以提到证人）；公理版给不出
    ——卷 II+ 的子类型/`Sigma` 要用（`lib/Exists.sokonanoda:27-29` 已列出）。
- **影响面 / golden**：
  - `course/`（入门课 11 单元）两份 GOLDEN：`crates/cli/tests/course.rs:86`
    （checked/open/reduced）与 `crates/cli/tests/course_status.rs:68`（+failed）。
    **预期不变**：本 WO 只让"今天必定失败"的输入变绿，不改任何已通过块的 recursor 形状；
    若真变了 → 必须双改 GOLDEN 并在 WO 回复里逐条说明为何某个事件计数合法变化。
  - `courses/set-theory/` **没有** Rust golden（`crates/cli/tests/` 里 grep 不到
    `set-theory`），它由 `courses/set-theory/tools/check.py` 守——所以课程侧的"回归线"
    就是那个脚本的 `failed: 0` 与计数不降。

## 文档同步清单

| 文件 | 是否涉及 | 说明 |
|---|---|---|
| `docs/protocol.md` | 不涉及 | 无新事件/字段/错误码；`kernel-rejected` 的语义不变，`--json` 形状不动 |
| `docs/TESTING.md:39` | **必改** | 同族测试索引行（K 目标判据）旁补本 WO 的新测试名与运行命令 |
| `docs/TESTING.md:82`（调试指引） | 建议 | "`k_target`/`is_k` 红了去读内核那几行"旁补一句：派生 recursor 的**宇宙参数**红了去读 `large_elim_test`/`mk_elim_level` |
| `docs/architecture.md:377-382`（§8 gotcha 0b） | **必改（已过时）** | 末句"缺 `rec` 的块在入环境之前报 `elab-missing-inductive-rec`"在 0.58.0 自动派生之后已不成立（源码里 `elab-missing-inductive-rec` 已不存在；本 WO 的 P1–P14 全部无 `rec` 却进了内核）。同轮补上"派生判据必须与内核同规则镜像" |
| `docs/architecture.md:150-176`（§4.2） | 建议 | `install_inductive_block` 的描述今天没提"无 rec 时自动派生"；一句话补上即可，不必展开 |
| 新建设计文档 `docs/design/`（如 `prop-large-elim-mirror.md`） | **必须（设计先行）** | 记录判据镜像与"派生时机"选择（策略 A/B）、以及 P1–P14 对拍表 |
| `skills/`（teacher/dev/ci） | 预期不动 | 无新命令、无新语法、无协议变化；除非新增了要跑的门禁命令 |
| `editor/vscode/`（源码/清单/README/CHANGELOG） | 不涉及（内容） | 无新命令/键位/视图/协议值 → 扩展**内容**零改动；但版本号按发布纪律仍与 `Cargo.toml:6` 两处同步 `0.58.0 → 0.59.0`（`editor/vscode/package.json:5`） |
| `AGENTS.md` | 不涉及 | 无新命令/流程 |
| `docs/HANDOVER.md` | **必改** | 交接状态（G-03 已修、新版本号、Exists 是否已升级） |
| `STATUS.md` | **必改** | 最近 3 轮；本轮标题写 WO-006/G-03（网站进度页读最新轮标题） |
| `REQUIREMENTS.md` §9 | **必改** | 用户可见（"本来该编过的程序现在编过了"＋课程库从公理升级为归纳）：按 0.59.0 批次追加日期条目 |
| `docs/design/course-stdlib.md:97` | **课程线同轮改** | G-03 那行"`Exists` 只能公理"→ 已修 |
| `courses/set-theory/lib/Exists.sokonanoda`（`:11-29`、`:68-70`） | **课程线同轮改** | 公理三件套 → inductive（见「兼容策略」） |
| `docs/gaps/ledger.jsonl` | **必改** | `wo` 填本文件路径、`status` → `wo-filed`；修完由 `gap.py close` 写 `fixed_in` |
| `docs/gaps/repro/G03-…sokonanoda:9-14` | 建议 | 关账时在注释头补一行"0.59.0 起本文件干净判卷"，**块本身（`:29-31`）逐字不动** |

## 门禁

- `scripts/soko gate`（= fmt + clippy + test + playground 锚点）。注意：anchor 用**运行中
  二进制**的内嵌编译器，若与仓库版本不一致会直接 `exit 3` → 先 `scripts/soko update`，
  或改用 `cargo run -q -p sokonanoda-cli --bin sokonanoda -- --json playground.sokonanoda`。
- 全量 `cargo test --workspace --locked`。
- 只 fmt 教学 crates：`cargo fmt -p sokonanoda-front -p sokonanoda-cli -p sokonanoda-lsp -- --check`
  （**禁止** `cargo fmt --all`：会重排冻结内核）。
- 课程门禁：`python3 courses/set-theory/tools/check.py`（绿 = `failed: 0`）。
- 版本：`Cargo.toml:6` 与 `editor/vscode/package.json:5` 两处同步 `0.58.0 → 0.59.0`
  （AGENTS.md 的"bump 两处版本"；发布轮 auto-tag）。
- 二进制对拍（仓库惯例：一次一刀 + 对拍）：修前/修后各跑一遍复现件与 P1–P14 语料，把
  "13 checked / 3 failed → 16 checked / 0 failed"的原始 `--json` 前几行贴进 WO 回复。

## 关账

- 修好后先自证：`scripts/soko grade docs/gaps/repro/G03-prop-type-param-inductive.sokonanoda`
  **退出码 0** 且出现 `"type":"decl.checked"`（`gap.py` 对 `.sokonanoda` 复现的判据正是
  "干净判卷 + 有 checked 声明"，`scripts/gap.py:71-80`）。
- 然后：`python3 scripts/gap.py close G-03 --version 0.59.0`。
- 提醒（本轮读代码确认，别踩）：
  - G-03 的复现是 `.sokonanoda`，**没有 `.sh` 外壳**；`gap.py close` 的"仍复现就拒绝"护栏
    只对 `kind == "script"` 生效（`scripts/gap.py:198-205`）→ 关账前**自己**确认上面那条
    判卷干净，别指望工具拦你；
  - `gap.py check` 会在修好后把 G-03 标成「已判卷通过 ← 台账写的是「仍有失败」」并 `exit 1`
    （`scripts/gap.py:166-195`）——那行红就是"回来关账"的提醒；
  - 关账同一轮把台账 `wo` 指向本文件（`status: wo-filed` → 修完 `fixed`），并确认课程侧
    `lib/Exists.sokonanoda` 的升级与 `requires` 版本已同步（否则课程仓会拿到旧行为）。

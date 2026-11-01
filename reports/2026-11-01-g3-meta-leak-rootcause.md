# G3 根因：strict 把内部元变量名漏给用户（`` unknown identifier `\0soko_m2` ``）

**只读取证**（分支 `strict/2026-10-29-kernel-app`，HEAD `19edaec7`；`crates/` 未改）。
背景 = `reports/2026-11-01-strict-gaps-before-default-flip.md` **§3 发现 3**。

---

## 0. 一句话结论

strict 的**唯一**分叉点是 **`crates/front/src/compile/pp_shape.rs:123-148` 的 `elab_engine_value`**：
它在**严格档**才把 `options` 换成 loose，并且**同时把 `metas` 摘成 `None`**（`pp_shape.rs:145`）。
这个"显示形状"的 ctx 恰好被用来重 elaborate **仍是载体的值**（`Ident("\0soko_m2")`）⇒
`decl_meta_carrier` 的**第一道闸**（`meta.rs:817` 的 `let decl = ctx?`）当场失败 ⇒
落到 `resolve_known`（`elab.rs:282`）的 `ElabUnknownIdentifier`，span 取标识符自己的
`Span::default()` ⇒ **0:0**。宽松档走**同一个函数的早退分支**（`pp_shape.rs:123-135`），
用的是**调用方 ctx**（`metas` 在）⇒ 载体照常翻成核项 `Expr::Meta` ⇒ 绿。

**不是**"strict 少了一次 `zonk`"，**也不是**"strict 的出口没走 `discharge` 自检"：

* `zonk`（`meta.rs:961`）只代**已赋值**的元变量 ⇒ 对**未解**载体**本来就是 no-op**。
  读数：`reordered`（`p4b.rs:320`）调过 `zonk_value` 之后交到 `elab_engine_value` 的值，
  **仍然逐字**是 `Ident { name: "\0soko_m2", span: 0:0 }`（见 §2 的 `[G3 G]` 行）；
* `discharge` 的出口自检（`meta.rs:1425-1453`）在 **shared/Partial** 这条路上被
  `discharge_keep_unsolved`（`meta.rs:1467`）**有意**跳过（注释白纸黑字：值里可以留元变量名）；
  本该收口的是声明出口 `p4.rs:257 close_meta_store`，但本复现**走不到那里**（见 Q1 末段）。

---

## 1. 构建身份 · 复现 · 两档读数

| 用途 | 构建 | 说明 |
|---|---|---|
| 基线读数 | `097996f683cc079b` | HEAD 工作树的 `cargo build --release -p sokonanoda-cli`（= 报告 §6 的同一份）；副本 `/tmp/soko-097996f683cc079b` |
| 探针读数 | `8ea3ee482cfce9f5` | 加入 §5 的探针后的 release 构建（探针 A–F） |
| 探针读数 | `b88487d74488943d` | 再加探针 G/H 的 release 构建（**§2/§3 的读数全部出自这一份**） |

> ⚠ **并发工作树**：`b88487d74488943d` 是在**同一工作树**上构建的，当时另有 agent 正在改
> `crates/front/src/compile/{eqsubst.rs,kernel_app_report.rs,named_arg.rs}` 与
> `crates/front/src/compile/tests/eqsubst_predicate.rs`（G2 批次，未提交）⇒ 探针构建**不等于纯 HEAD**。
> 本报告所有 strict/loose 对比都取自**同一份**构建（同一次 `cargo build`），所以"分叉在哪"的结论不受影响；
> 但这两个探针身份的数字**不要**与报告 §3.2 的 `097996f683cc079b` 跨构建混比（`AGENTS.md`「探针读数必须带构建身份」）。

复现件（`/tmp/g3retest.sokonanoda`，零 import）：

```sokonanoda
def Set (α : Type) : Type := α -> Prop
def Set.empty (α : Type) : Set α := fun (x : α) => False
def Set.singleton (α : Type) (a : α) : Set α := fun (x : α) => x = a
def Set.Equiv {α β : Type} (A : Set α) (B : Set β) : Prop :=
  (forall (x : α), A x -> A x) ∧ (forall (y : β), B y -> B y)
notation "∅" => Set.empty
infix:50 " ≈ " => Set.Equiv

theorem n3 (β : Type) (b : β) : ¬ (∅ ≈ {b}) := by sorry
```

一次判卷（本报告所有 rc/诊断都出自这一条）：

```bash
env -u SOKO_STRICT_AT SOKONANODA_NO_PROJECT_ARTIFACTS=1 SOKONANODA_NO_MODULE_ARTIFACTS=1 \
  SOKONANODA_CACHE_DIR=$(mktemp -d) <bin> grade --dialect {loose,strict} /tmp/g3retest.sokonanoda --json
```

两档读数（探针构建 `b88487d74488943d`，与报告 §3.2 逐字一致）：

| 档 | rc | 事件流 |
|---|---|---|
| `loose` | **0** | `decl.checked` ×4（`Set` / `Set.empty` / `Set.singleton` / `Set.Equiv`）+ `exercise.open`（`n3`） |
| `strict` | **1** | `decl.checked` ×4 + `diagnostic`：`{"code":"elab-unknown-identifier","message":"unknown identifier \`\u0000soko_m2\`","span":{"start":{"line":0,"column":0,"offset":0},"end":{...0:0}},"stage":"elab"}` |

---

## 2. Q1 — `\0soko_m2` 是哪个 store 造的？生命周期是什么？

### 2.1 造名处（三处，缺一不可）

| 位置 | 作用 |
|---|---|
| `crates/front/src/compile/meta.rs:30` `meta_name` | 名字文本 `\0soko_m{id}`（`META_PREFIX`） |
| `crates/front/src/compile/meta.rs:930` `MetaCtx::fresh` | **分配 id**：`MetaId(store.base + store.mvars.len())`；同时把 `(ty, kind)` 压进**类型日志**（`meta.rs:934-936`） |
| `crates/front/src/compile/meta.rs:948-953` `MetaCtx::meta_expr` | 造**载体** `Expr::Ident { name: "\0soko_m{id}", span: Span::default() }` ← **span 0:0 的源头**（见 Q3） |

### 2.2 谁造的（call chain，**读数**：`[G3 B] fresh id=2` 的 backtrace，构建 `8ea3ee482cfce9f5`）

```
 1: sokonanoda_front::compile::meta::fresh_telescope_metas
 2: sokonanoda_front::compile::implicit::solve_prefix_outcome_with_args_alt
 3: sokonanoda_front::compile::elab::try_implicit_application_with
 4: sokonanoda_front::compile::elab::elab_expr
 5: sokonanoda_front::compile::elab::elab_notation
 6: sokonanoda_front::compile::elab::elab_expr
 7: sokonanoda_front::compile::elab::elab_notation
 8: sokonanoda_front::compile::elab::elab_expr
 9: sokonanoda_front::compile::elab::elab_expr
10: sokonanoda_front::compile::check::walk::open_signature
11: <sokonanoda_front::compile::check::walk::Walk>::theorem
```

转成源码位置（HEAD 行号）：

`walk.rs:1654 open_signature` → `walk.rs:2742 elab_expr`（**定理签名** `¬ (∅ ≈ {b})`）→
`elab.rs:1771 elab_notation_implicit`（`¬`）→ `elab.rs:4339 try_implicit_application_with` →
`implicit.rs:276 solve_prefix_outcome_with_args_alt` → `implicit.rs:413 solve_prefix_meta_alt` →
`implicit.rs:450 fresh_telescope_metas` → `meta.rs:930 MetaCtx::fresh` ⇒ **id = 2**。

配套读数（同一次 fresh，同一构建）：

```
[G3 B] fresh id=2 base=0 log=true store_len=2 out_of_scope=["\0soko_p1", "\0soko_p2", "\0soko_p3"] ty="Sort { sort: Type, span: (1,13)..(1,17) }"
[G3 F] implicit::solve_prefix_meta_alt k=2 shared=true ids=[2, 3] out=Partial
```

* **`k=2`** ⇒ 补 `Set.Equiv {α β : Type}` 的**两个**前导隐式位 ⇒ 造了 `m2`（α）与 `m3`（β）；
* **`shared=true`**（`[G3 C] park shared=true` = `meta.rs:649 DeclMVarStore::park` 的"槽里有 store"分支）
  ⇒ 这次求解用的是**声明级 store**（`park_decl_store(Some)`，`meta.rs:793`）；
* **`base=0` + `log=true`** ⇒ `StoreGuard::meta_ctx`（`meta.rs:753-760`）把**类型日志**接上了
  ⇒ `m2` 在 `DeclMVarStore::lookup`（`meta.rs:706`，读 log）里**可查**（读数 `log_hit=true`）。

### 2.3 生命周期：该在哪一步收掉？

| 阶段 | 实际发生 | 依据 |
|---|---|---|
| 造 | 在 decl store 里，id 2（α）、3（β） | `[G3 B]` 两行 |
| 出口（求解内） | `discharge_keep_unsolved`（`meta.rs:1467`）⇒ `Partial(out, still)` | `implicit.rs:528-531`；读数 `[G3 F] out=Partial` |
| 落地 | **commit**（不回滚）—— 注释写的"载体活到声明末尾" | `implicit.rs:536-541`；读数 `[G3 C] drop owns_slot=true commit=true mark=2 log_len_before=4` |
| `park` / 换出 | 每次求解 `park()`（`meta.rs:649`）→ `Drop` 放回（`meta.rs:769-817`） | 读数 `[G3 C] park shared=true` ×N |
| `zonk` | **no-op**：`zonk`（`meta.rs:961`）只代**已赋值**的；`m2` 没有值 ⇒ 载体原样 | 读数见 Q2 的 `[G3 G]`（进 `elab_engine_value` 的还是裸 `Ident`） |
| `discharge` 自检 | **不跑**：shared 出口走的是 `discharge_keep_unsolved`，它**有意**不做 `unsolved()` 闸、也不跑 `has_unassigned_meta` | `meta.rs:1467-1491` 的注释 |
| **本该收口** | 声明出口 `p4.rs:257 close_meta_store`（`park → unify_all → default_unresolved → unsolved → commit`）+ 第二趟重 elaborate（`p4.rs:73 build_value` 三趟） | 设计 `metavar-engine.md` §2.10 / `p4.rs` 模块头 |
| **实际没到** | 泄漏发生在**签名**（`walk.rs:2742 open_signature`）。签名失败的出口 `walk.rs:1654-1668` 直接 `push_error` + `failed_state`，**没有** `replay_after_failure`（那一份只接在 `build_value`：`p4.rs:142 / 164 / 170 / 226`） | 读数 + 代码 |

⇒ **`\0soko_m2` 的收口点不可能是"声明出口"**：签名这一路根本没有出口收口/重放。
它只能**在被交回 `elab_expr` 之前**翻成核项 `Expr::Meta`（= `decl_meta_carrier` 命中），
这正是 `meta.rs:1-20` 红线 1 写的"双保险"里的**第一道网**，只是这里它够不着（见 Q2）。

---

## 3. Q2 — 为什么 strict 留、loose 不留？分叉点在哪？

### 3.1 分叉点 = `pp_shape.rs:112-158` `elab_engine_value`

```rust
// pp_shape.rs:123-135（loose 走这支：原样用自己的 ctx，metas 在）
if !super::implicit_msg::strict_at_enabled(ctx.options) {
    return elab_expr(builder, e, scope, univ, known, hovers, expected, expected_src, ctx);
}
// pp_shape.rs:136-148（strict 走这支：换 loose 方言 + 摘掉 metas）
let loose = CompileOptions::with_dialect(ctx.options.prelude, Default::default());
let engine = ElabCtx {
    prefix_src: ctx.prefix_src,
    options: &loose,
    inductives: ctx.inductives,
    ns: ctx.ns,
    notations: ctx.notations,
    defs: ctx.defs,
    metas: None,          // ← 第 145 行：这一行就是分叉
};
elab_expr(builder, e, scope, univ, known, hovers, expected, expected_src, &engine)
```

* `strict_at_enabled` 的定义 = `options.dialect.strict()`（`implicit_msg.rs:77-79`）⇒ **只有严格档**进第二支；
* 函数用途（模块注释）：解出来的隐式实参可能是**内核 pp 文本**（"内核 pp 打的是正常形式写满实参"），
  那种文本**只在 loose 档**是合法源文本 ⇒ 所以**方言**要换。但 `metas` 与方言**正交**
  （`decl_meta_carrier` 只读 `ctx.metas` + store/log，没有任何 dialect 分支）⇒ 摘掉 `metas` 是**顺带**的，不是必需的。
* 调用点：`p4b.rs:320-331`（`reordered`）与 `p4b.rs:227-237`（非重排支）——两处都拿**求解出来的前导值**
  （`solved`，可能是载体）来 elaborate。`reordered` 先 `zonk_value`（`p4b.rs:353`，只读、不 commit），
  但未解载体 zonk 不掉。

### 3.2 两档并排读数（**同一份构建 `b88487d74488943d`**）

| 读数 | strict | loose |
|---|---|---|
| `[G3 G] elab_engine_value` 里 `strict_at` | **true** | **false**（全部 9 次） |
| `[G3 G] carrier_in_value` | **true**（`value = Ident { name: "\0soko_m2" }`） | **true**（`\0soko_m3` / `\0soko_m8` / `\0soko_m13` …） |
| `[G3 G] metas_in_ctx` | true（**进来的 ctx 是有表的**） | true |
| 建出来的 engine ctx 的 `metas` | **None** | 不建 ctx（早退，沿用调用方 ctx） |
| 紧接着的 `[G3 A] Ident …` 读数 | **`metas=None` ⇒ 报错** | `metas=Some … log_hit=true` ⇒ 命中 carrier |
| 全日志 `metas=None` 行数 | **1**（= 泄漏那一次） | **0** |

strict 的最后三行（原样）：

```
[G3 A] Ident "\0soko_m2" span=(0,0)..(0,0) | metas=Some store=true mvars=4 log_len=4 id=Some(2) log_hit=true expanding=[]
[G3 G] elab_engine_value strict_at=true carrier_in_value=true metas_in_ctx=true value=Ident { name: "\0soko_m2", span: Span { start: Pos { offset: 0, line: 0, column: 0 }, …
[G3 A] Ident "\0soko_m2" span=(0,0)..(0,0) | metas=None
```

loose 的同类三行（原样，对照）：

```
[G3 G] elab_engine_value strict_at=false carrier_in_value=true metas_in_ctx=true value=Ident { name: "\0soko_m3", span: Span { start: Pos { offset: 0, line: 0, column: 0 }, …
[G3 A] Ident "\0soko_m3" span=(0,0)..(0,0) | metas=Some store=true mvars=5 log_len=5 id=Some(3) log_hit=true expanding=[]
```

⇒ **同一条载体、同一个调用点、同一份 `solved`**；strict 里它进了一个 `metas: None` 的 ctx ⇒
`decl_meta_carrier`（`meta.rs:817-822`）三道闸的**第一道**就断：

```rust
let decl = ctx?;                       // ctx.metas == None ⇒ 直接 None
let id = decl_meta_id_of(name)?;
let (ty, kind) = decl.lookup_meta(id)?;
let guard = decl.enter_meta_expansion(id)?;
```

⇒ 原样落到 `elab.rs:5540` 之后的 `resolve_known`（`elab.rs:282` / 报错在 `elab.rs:295`），
文案 `unknown identifier \`{name}\``（`elab.rs:312-336`；本例 candidates 为空 ⇒ 无"你是不是想写"）。

### 3.3 一个加强结论：strict 还会被 kernel 路径"交回老路"

strict 下 `try_implicit_application_with` 先问内核路径（`elab.rs:4370 kapp`）；`kapp` 对**输入含载体**的应用
按**写明的例外**回落：

```
[G3 E] fell why=input contains a meta verdict=Fallback reason=input contains a meta
```

（`kernel_app_fallback.rs:203-211`；`Report::Fallback` 不报错）。⇒ strict 与 loose **最终都走**
`assemble_app`；两档的差别只剩 `elab_engine_value` 的 ctx。这排除了"strict 的某个出口没走 `discharge`"这一读。

### 3.4 为什么 loose 最终是绿的（同一份 `solved` 怎么活下来的）

loose 的读数：`[G3 F] … out=Partial` 出现 **3** 次（ids `[3,4]`、`[8,9]`、`[13,14]`），
载体 `m3/m8/m13` 都被 `carrier` 翻成 `Expr::Meta`（`log_hit=true`），随后由
`default_unresolved`（同形兄弟，`meta.rs` 的 defaulting）/声明出口收口消掉 ⇒ rc=0。

---

## 4. Q3 — `span 0:0` 是怎么来的？它**应该**来自哪里？

### 4.1 来源（两跳，都可读）

1. **载体天生没有位置**：`MetaCtx::meta_expr`（`meta.rs:948-953`）硬写 `span: Span::default()`；
   而 `discharge_keep_unsolved` 的出口也正是 `self.zonk(&self.meta_expr(*id))`（`meta.rs:1478`）
   ⇒ 任何**未解**载体交出来时 span **恒为 0:0**（读数：`[G3 G] value=Ident { … Pos { offset: 0, line: 0, column: 0 } }`）。
2. **诊断照抄标识符 span**：`Expr::Ident` 臂用 `*span`（`elab.rs:5517` 的模式绑定）
   → `decl_meta_carrier` 未命中 → `resolve_known(known, ctx.ns, name, *span, &scope.names)`
   （`elab.rs:5568`）→ `CompileError::elab(ElabUnknownIdentifier, …, span)`（`elab.rs:295`）
   ⇒ JSON 的 `span.start/end` 全 `{line:0,column:0,offset:0}`。

### 4.2 应该来自哪里（决定"span 真实"怎么验）

**使用处**（用户真的写了东西的那个结点的 span）。同一现场可用的真实 span 读数：

```
[G3 H] elab_notation symbol="¬" target="Not"        span=(9,33)..(9,43)
[G3 H] elab_notation symbol="≈" target="Set.Equiv"  span=(9,36)..(9,43)
[G3 H] elab_notation symbol="∅" target="Set.empty"  span=(9,36)..(9,37)
[G3 H] elab_notation symbol="{a}" target="Set.singleton" span=(9,40)..(9,43)
```

（第 9 行就是 `theorem n3 … : ¬ (∅ ≈ {b}) := by sorry`。）

但**现在的参数表拿不到它**：`p4b::assemble_app`（`p4b.rs:189-197`）与
`pp_shape::elab_engine_value`（`pp_shape.rs:112-122`）都**没有 span 参数**（`assemble_app` 只有
`head: ExprPtr` / `args: &[&Expr]`）。两条可行取法：

* **(a) 造载体时就盖戳**：`fresh_telescope_metas`（`meta.rs:862`）/`meta_expr` 多带一个
  "使用处 span"，让 `Discharge` 出来的载体带真实位置（改面在 `meta.rs` + `implicit.rs`/`elab.rs` 两个调用点）；
* **(b) 出口时给**：把 App/记法的 span 传进 `elab_engine_value`，在"载体解不掉"时用它报诊断
  （改面在 `elab.rs:5540` 所在的 Ident 臂之外，需要把 span 从 `elab_expr` 的 `Expr::App` 臂带到 `p4b`）。

### 4.3 修完怎么验"span 真实"

1. **正例（最硬）**：两档 rc=0 且**零诊断** ⇒ 0:0 无处可藏；
2. **负例（反向）**：构造一个"载体真的解不掉"的形状（如 `SOKO_DECL_MVARS=0` 下的同族行为，或人为把
   `solved` 留成载体），断言该诊断 **`span.start.line == 9`**（使用处行号）而**不是 0**；
3. **守卫**：一条扫描式测试 —— 凡 `message` 含 `\0`（或 `soko_m`）的诊断，`span.start.line == 0`
   即判红（`REQUIREMENTS §9.39` 的"内部名不许出现在用户文案"再落一条机械判据）。

---

## 5. Q4 — 最小修法方向（**本刀未实现**）

### 5.1 主选：让 `decl_meta_carrier` 在这条路上可达（一处、一行）

`crates/front/src/compile/pp_shape.rs:145`：

```rust
-        metas: None,
+        metas: ctx.metas,
```

理由（三条，都与"改判定"无关）：

1. `metas` 与**方言正交** —— `decl_meta_carrier` 只读 `ctx.metas` + store/log，没有 dialect 分支
   （`meta.rs:817-822`）；换方言的**理由**（内核 pp 文本只在 loose 合法）与是否接 meta 表**不相干**；
2. 这正好是题目里点名的"**换条走到 `decl_meta_carrier`**"，而且是把 strict 的 ctx 形状**对齐 loose**
   （项目方向：strict 不比 loose 更严）；
3. 一行、无新增分支 ⇒ 可回退、可反向验证。

⚠ 副作用面要在修法那一轮用探针复测（见 §7）：engine ctx 一旦有表，
`p4b::assemble_app` 的闸门 `ctx.metas.is_some() && decl_mvars_enabled() && carries`（`p4b.rs:213`）
在这条重 elaborate 里会变真（走 `reordered`），以及 `b2_meta_pair`/`pattern::note_ctx` 等
"读 `ctx.metas`"的分支。**它们都属于"与 loose 一致"的方向，但不是零风险。**

### 5.2 次选（更外科）：把载体在 `elab_engine_value` **之前**翻成核项

在 `p4b.rs:320`（`zonk_value` 之后、`elab_engine_value` 之前）判一次：若值**仍是载体**
（`decl_meta_id_of` 命中且 `lookup_meta` 有），直接用 `builder.mk_meta_with_kind(id, kind, ty)`
造核项 —— 与 `elab.rs:5540` 那两个分支**同一份逻辑**（`decl_meta_carrier` + `mk_meta_with_kind`），
不再回 `elab_expr`。代价：要在 `p4b` 里复用 carrier 的两行（或把 carrier 提成一个 plantable helper）。

### 5.3 守卫（**不是修法**，两者都该加）

* `elab.rs:5540` 的 `if let Some(...) = decl_meta_carrier(...)` **之后**、`resolve_known` 之前：
  `debug_assert!(!crate::compile::meta::is_meta_name(name), "载体漏进 elab_expr：{name}")`
  （debug/test 档咬住；**release 不咬** ⇒ 不能当修法）；
* 用户文案卫生：诊断 message 含 `\0` 即判红（release 档也生效，§4.3 第 3 条）。

### 5.4 修完后"两档都必须 rc=0"的判据长什么样

1. **复现件**：`grade --dialect {loose,strict}` 两档 **rc=0**，事件流**逐字节相同** =
   `decl.checked` ×4（`Set` / `Set.empty` / `Set.singleton` / `Set.Equiv`）+ `exercise.open`（`n3`），
   **零 `diagnostic`**；
2. **测试**：`crates/cli/tests/metavar_engine.rs` 的 `N3_g48_both_nullary` **两档都绿**，
   且**不许**给它钉 `--dialect loose`（报告 §5 的纪律：钉 loose = 把新默认档从判据里抹掉）；
3. **反向验证**：只回退 §5.1 那一行 ⇒ 上面那条测试**必须重新变红**（否则这条守卫没咬住）；
4. **span 判据**：§4.3 的 ② ③ 两条；
5. **同族回归面**：报告 §3.4 点名的两条同类 front 单测
   （`parameterized_option_checks_and_derives_recursor`、
   `set_option_pp_explicit_covers_check_expressions_but_not_bare_constant_signatures`）
   在 strict 档下要重取读数 —— 它们可能同因（本刀未复跑）。

---

## 6. 探针清单（**已全部还原**）

所有探针都**只打印**（`eprintln!`），**零语义改动**，统一由 `SOKO_G3_TRACE=1` 门控。

| # | 文件 | 位置（探针态） | 打印内容 |
|---|---|---|---|
| A | `crates/front/src/compile/elab.rs` | `Expr::Ident` 臂入口 | 载体名 + span + `probe_decl_state` + backtrace |
| D | 同上 | `notation_prefix_args` 的 `for missing` 循环 | 记法签名 / 操作数 / 层数 / 这一读的 `missing` |
| H | 同上 | `elab_notation` 入口 | 符号 / 目标 / **真实 span** |
| B | `crates/front/src/compile/meta.rs` | `MetaCtx::fresh` | id / base / log 是否接上 / store 长度 / `out_of_scope` / ty + backtrace |
| C1 | 同上 | `DeclMVarStore::park` | 本次是不是"槽里有 store"（shared） |
| C2 | 同上 | `StoreGuard::drop` | `owns_slot` / `commit` / `mark` / 回滚前的 log 长度 |
| — | 同上 | 新增 `probe_decl_state`（只在 A 里被调用） | `ctx.metas` / store / mvars / log 长度 / `log_hit` / `expanding` |
| F | `crates/front/src/compile/implicit.rs` | `solve_prefix_meta_alt` 出口 | `k` / shared / ids / `out` 通道 |
| E | `crates/front/src/compile/kernel_app_fallback.rs` | `fell` 的 `match report(...)` 两支 | `why` + verdict（`Error`/`Fallback`） |
| G | `crates/front/src/compile/pp_shape.rs` | `elab_engine_value` 入口 | `strict_at` / 值里有没有载体 / ctx 有没有表 / 值的 Debug |

**还原方式**：`git checkout -- crates/front/src/compile/{elab.rs,meta.rs,implicit.rs,kernel_app_fallback.rs,pp_shape.rs}`。

**还原判据（三条）**：

```bash
git status --short                      # 实测：?? reports/2026-11-01-g3-meta-leak-rootcause.md
                                        #      （其余 4 个 M 是**别的 agent** 的 G2 eqsubst 改动，不是本刀的）
grep -rn "SOKO_G3_TRACE\|G3-PROBE\|G3 A\]\|probe_decl_state" crates/ | wc -l    # 实测 0
wc -l crates/front/src/compile/elab.rs  # 实测 8841（docs-lint ⑨ 基线，一行不多）
python3 scripts/docs-lint.py            # 实测 rc=0：「超软线源码 47 个（基线内、无变长 ✓）」
```

（还原后的行数已实测 = HEAD 原值：`elab.rs` **8841** · `meta.rs` **1848** · `implicit.rs` **1218** ·
`pp_shape.rs` **316** · `kernel_app_fallback.rs` **489**。）

---

## 7. 我没能确定的部分

1. **§5.1 那一行的效果没有实测**（本刀只读，Q4 明确"不要实现"）⇒ "两档 rc=0"是**方向 + 判据**，
   不是已验证的结论。尤其 `p4b.rs:213` 的闸门在 engine ctx 里变真之后的连锁，需要修法那一轮用
   §6 的探针复测（预计 `reordered` 会多跑几次）。
2. **同类出口只证了这一条**。`#check`/`#reduce`（`walk.rs:2550` / `walk.rs:2617`）、
   `elab.rs:904` 的 base ctx 也都是 `metas: None`，是否同样会咬到载体，本刀**没有**做定点取证。
3. **`m3` 没有单独读数**：`k=2` 的读数是 `ids=[2,3]`，但泄漏只被 `m2` 命中（`m3` 是同一个 `solved` 的
   第二个值，推断同形）。若修完仍有 `soko_m3` 的报告，说明还有第二条路。
4. **报告 §3.4 的两条同类 front 单测**未在 strict 档复跑（只读了报告的表，没有自己量）。
5. 泄漏在**签名**这一路没有出口收口（`walk.rs:1654-1668` 无 `replay_after_failure`）——
   这是**结构性**观察；是否要在签名路也补一份闸门关重放，属于修法轮的设计决定，本刀不裁。

---

## 附：本报告的证据文件（`/tmp`，不随仓库走）

| 文件 | 内容 |
|---|---|
| `/tmp/soko-097996f683cc079b` | 基线二进制（报告 §6 的同一份） |
| `/tmp/soko-8ea3ee482cfce9f5` / `/tmp/soko-b88487d74488943d` | 两轮探针构建 |
| `/tmp/g3p-strict.err` / `/tmp/g3p-loose.err` | 探针 A–F 的两档原始日志 |
| `/tmp/g3q-strict.err` / `/tmp/g3q-loose.err` | 探针 A–H 的两档原始日志（§2/§3 的读数出处） |
| `/tmp/g3q-strict.out` / `/tmp/g3q-loose.out` | 两档 `--json` 事件流 |

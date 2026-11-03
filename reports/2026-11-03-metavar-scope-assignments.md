# C 案设计：元变量赋值**带 scope**（Lean `MetavarDecl.lctx` 的 de Bruijn 版）

> 用户 2026-11-03 拍板：「要再降违反数 ⇒ 解冻内核表示（C）」。本文是**契约**（设计先行），不含过程。
> 目标不是"再快一点"，而是让 `elab_app` 能接受**调用方在 binder 下造的元变量** ⇒ 消掉
> `input_contains_a_meta` / `prefix_meta` 一族（96 次 / 4 个文件）**唯一**的真修法。

## 1. Lean 4 对照（本机 `~/Documents/lean/lean4/src/Lean/MetavarContext.lean`）

| Lean 4 | 本内核（今天） | 差异 ⇒ 决定 |
|---|---|---|
| `MetavarDecl.lctx : LocalContext` —— 元变量**许可依赖**的自由变量表（`:291-297`） | 只有 `depth_of(id)`；`depth` **读了不用**（`mvar.rs::in_meta_scope` 的 `let _ = ...`） | 要补：赋值必须能提 meta 自己 lctx 里的 binder |
| `eAssignment : MVarId → Expr`，值是**在它自己的 lctx 里**写的表达式（`:351`） | `values[id] = ExprPtr`，**闭项存储**（`in_meta_scope` 要求 `quote(depth, v).num_loose_bvars() == 0`） | 闭项表达不了"依赖 d 个 binder" ⇒ 这正是 `?α := α` 被拒、应用退化成 `ArgMismatch` 的原因（已实测） |
| `MetavarContext.depth`「控制合一里能不能赋值」（`:336-338`） | `is_def_eq_at(depth, ...)` 已有深度 | 可复用 |
| Lean 用 **fbvar**，重放不需要移位 | 我们用 **de Bruijn** ⇒ 重放必须**移位**（`expr.rs::lift(e, cutoff, amount)`） | 新增：按 `use_depth - meta_depth` 抬 |

## 2. 目标不变式（三句话）

1. **存**：`values[id]` 存的是**在 `depth_of(id)` 这个上下文里**写的表达式 —— 它的 loose var 只许
   `< depth_of(id)`（即"只提自己 lctx 的 binder"）。
2. **写**：`unify_meta` 在深度 `depth` 下拿到值 `v` ⇒ `q = quote(depth, v)`；要求 `q` 的**每个** loose var
   都 `>= depth - d`（`d = depth_of(id)`），然后 `values[id] = lower(q, 0, depth - d)`；
   不满足 ⇒ 干净拒绝（今天的"闭项"判据是这条在 `d = 0` 时的特例 ✓）。
3. **读**：在深度 `depth` 重放 `?id` ⇒ `t = lift(values[id], 0, depth - d)`（`depth > d`），
   再 `eval(depth, env, t)`；`depth < d` ⇒ 值可能提到用点看不见的 binder ⇒ **拒绝/不可用**。

## 3. 全部重放点（`grep` 实测，6 处：4 处要移位、2 处不用）

| # | 位置 | 今天 | 要做什么 |
|---|---|---|---|
| 1 | `mvar_inst.rs:68` `inst_mvars_go` 的 `Meta` 臂 | **无 depth 参数** | 穿一个 depth（进 Pi/Lambda/Let 体 +1），`Meta` 臂按 §2.3 抬 |
| 2 | `mvar_inst.rs:193` `unassigned_go` 的 `Meta` 臂 | 无 depth 参数，**穿过赋值**看 | 同上（否则未赋值清单会错） |
| 3 | `mvar.rs:383` `eval_meta` | 有 `depth`/`env` | 抬后再 `eval` |
| 4 | `mvar_spine.rs:21` `meta_replay` | 有 `depth` | 抬后再 `eval` |
| 5 | `mvar.rs:278` `unify_meta`（已赋值那支） | 有 `depth` | 抬后再 `unify` |
| 6 | `mvar_miller.rs:76` | 只看 `value(id).is_some()` | 不动 |

## 4. ⚠ 两个真陷阱（写下来免得踩）

* **缓存键**：`inst_mvars_go` 的 `cache: FxHashMap<ExprPtr, ExprPtr>` 与 `unassigned_go` 的 `seen` 都只按
  `ExprPtr` 键 —— 移位一旦取决于 depth，同一个子项在不同 depth 下的结果**不同** ⇒ 键必须改成
  `(ExprPtr, u32)`（或 depth 里含 meta 时才用缓存）。这是本刀最容易错的一处。
* **兼容面 = 零**：按 K1c 的注释，**今天没有任何调用方在 binder 下造元变量** ⇒ 所有已登记元变量
  `depth_of == 0` ⇒ 抬 0 位 ⇒ **行为逐字节不变**。所以 714 行语料对拍必须**零差异**；
  一旦有差异，先怀疑陷阱 ①，不要改 golden。

## 5. 判据（缺一条就不算完成）

1. `scripts/strict/corpus-hashes.sh`（357 × 2 = 714 行）与基线**逐字节相同**；
2. `scripts/strict-score.py --corpus`：违反表从 7（或当时的值）**下降**，且没有新违反；
3. `scripts/strict/exception-census.py`：那 4 个文件的 `nofallback_kept_*` **归零**；
4. 计分板四门绿（`lib_strict` / `lib_loose` 17/17 · `work_le_loose` · `course_strict` 112/112）；
5. 整门课 strict 258 目标 / 2191 checked / **0 判负**；G3 内部名不泄漏；
6. 新单测：`?m`（depth 1）赋成 `Var 0` ⇒ 在 depth 1 重放 = 原样、在 depth 2 重放 = `Var 1`；
   反向：在 depth 0 重放 depth 1 的赋值 ⇒ 必须拒绝。

## 6. 切片（下一轮按顺序做，每片可独立回滚）

| 片 | 内容 | 判据 |
|---|---|---|
| C1 | §2 的三条不变式 + `MetaContext` 加 `stored_depth` 校验 + 单测（不接线） | 新单测绿、714 行不变 |
| C2 | `inst_mvars_go`/`unassigned_go` 穿 depth + 缓存键改 `(ExprPtr, u32)` | 714 行不变（此时所有 meta 仍 depth 0） |
| C3 | 四个重放点接 `lift`；`unify_meta` 存 `lower` | 714 行不变 |
| C4 | `elab_app` 恢复 `app_register_inputs`（本轮已写好又还原的那段）；`kernel_app_notation` 允许 prefix 里带元变量 | 4 个文件 `kept` 归零；`--corpus` 违反表下降 |

## 7. 施工环境（必须写在前面）

本轮工作树被**并发进程**反复改动（`ast/parser/by/walk/judge` 变长、`unit02` 的证明被改写、我的未跟踪产出
被清过一次）。C 是内核表示级改动 ⇒ **下一轮在独立 worktree 里做**（`git worktree add`），
基线二进制与对拍都在**同一目录**内取（G77：诊断带模块根绝对路径，换目录哈希就变）。

## 附录 A：C4 要用的两段代码（本轮已编译+实测生效，随后按用户约束还原）

`crates/kernel/src/elab_app_holes.rs`（新增；实测结果：`input_contains_a_meta` 22/10/9/3 → 3/0/0/0）：

```rust
    /// **P2-13a**：`elab_app` 的**输入**（binder 类型 / 头 / 实参 / λ binder 类型）里登记的**闭**元变量。
    /// `depth` = 本会话的 binder 层数（= 调用方 elaborate 这些项时的作用域深度）。
    pub(crate) fn app_register_inputs(
        &mut self,
        exprs: &[ExprPtr<'t>],
        depth: u32,
    ) -> Result<(Vec<u32>, u32), &'static str> {
        let mut found: Vec<(u32, crate::expr::MetaKind, ExprPtr<'t>)> = Vec::new();
        let mut seen = crate::util::FxHashSet::default();
        let mut stack: Vec<ExprPtr<'t>> = Vec::with_capacity(exprs.len());
        for e in exprs {
            if !self.ctx.unassigned_mvars(*e).levels.is_empty() {
                return Err("input contains a meta");
            }
            stack.push(*e);
        }
        while let Some(e) = stack.pop() {
            if !seen.insert(e) { continue; }
            match self.ctx.read_expr(e) {
                Expr::Meta { id, kind, ty, .. } => match found.iter().find(|(i, ..)| *i == id) {
                    Some((_, _, t)) if *t != ty => return Err("input contains a meta"),
                    Some(_) => {}
                    None => found.push((id, kind, ty)),
                },
                Expr::App { fun, arg, .. } => stack.extend([arg, fun]),
                Expr::Pi { binder_type, body, .. } | Expr::Lambda { binder_type, body, .. } => {
                    stack.extend([body, binder_type])
                }
                Expr::Let { data, .. } => stack.extend([data.body, data.val, data.binder_type]),
                Expr::Proj { structure, .. } => stack.push(structure),
                _ => {}
            }
        }
        found.sort_by_key(|(id, ..)| *id);
        let mut base = SESSION_META_BASE;
        for &(id, kind, ty) in &found {
            if ty.num_loose_bvars() != 0 { return Err("input contains a meta"); }
            let u = self.ctx.unassigned_mvars(ty);
            if !u.exprs.is_empty() || !u.levels.is_empty() { return Err("input contains a meta"); }
            if self.ctx.mvars.ty_of(id).is_some_and(|prev| prev != ty) {
                return Err("input contains a meta");
            }
            self.ctx.mvars.register_at(id, ty, depth);   // ← C 案落地后这一行的 depth 才真正生效
            self.ctx.mvars.register_kind(id, kind);
            base = base.max(id.checked_add(1).ok_or("input contains a meta")?);
        }
        Ok((found.into_iter().map(|(id, ..)| id).collect(), base))
    }
```

`crates/kernel/src/elab_app.rs`（`elab_app_session` 开头，替换原来"一律拒绝"的循环）：

```rust
        let inputs: Vec<ExprPtr<'t>> = binder_tys.iter().copied()
            .chain(std::iter::once(head))
            .chain(args.iter().flatten().copied())
            .chain(lam_tys.iter().flat_map(|(_, t)| t.iter().copied()))
            .collect();
        let had_input_meta = inputs.iter().any(|e| !self.ctx.unassigned_mvars(*e).is_empty());
        let (mut externals, mut base) = match expected { /* 原样保留 P2-9 那支 */ };
        if had_input_meta {
            let scope_depth = u32::try_from(binder_tys.len()).unwrap_or(u32::MAX);
            match self.app_register_inputs(&inputs, scope_depth) {
                Ok((ids, b)) => {
                    base = base.max(b);
                    for id in ids { if !externals.contains(&id) { externals.push(id); } }
                }
                Err(_) => return AppOutcome::Unsupported("input contains a meta"),
            }
        }
```

以及**出口纪律**（必需，否则会把"解不出"升级成诊断；S4c 的反向验证抓过这类）：

```rust
        if !holes.is_empty() {
            let out = self.app_need_args(d, &holes, unsolved);
            return match (had_input_meta, out) {
                (true, AppOutcome::Unsolved { .. }) => AppOutcome::Unsupported("input contains a meta"),
                (_, out) => out,
            };
        }
        if !unsolved.is_empty() {
            return if had_input_meta {
                AppOutcome::Unsupported("input contains a meta")
            } else {
                AppOutcome::Unsolved { sites: unsolved }
            };
        }
```

⚠ 光有上面三段**不够**：`in_meta_scope` 仍然会拒 `?α := α`（本轮实测：全部退化成 `argmismatch_text`）。
C1–C3 必须先把 §2 的读写移位做进去，C4 才恢复这三段。

## 8. 施工结果（2026-11-03）：C1–C3 已落，C4 **判为不可交付**

### C1–C3 ✓（`02979b5a`）

| 判据 | 读数 |
|---|---|
| 全语料 357 × {loose,strict} = 714 行逐字节 | **0 行差异**（base `4895e71c` vs C `fcdea5a6`；还原 C4 后 `5f56da37` 复测仍 0 行） |
| 内核单测 | **130/130 绿** |
| 新增判据 | C-T1 正（depth 1 的 `?s := α` + 两层 λ 下抬成 `#1`）· C-T2 反（用点 < 创建深度 ⇒ 拒）· C-T3 零变化（depth 0 只收闭项） |

### C4 ✗（实测改变判卷结论 ⇒ 还原）

接上附录 A 的接线（`app_register_inputs` + `Unsolved ⇒ Unsupported` 纪律）后：

| 文件 | C1–C3 | C1–C4 | 判定 |
|---|---|---|---|
| `G48-notation-nullary-sugar-operands` | rc=0 | rc=0 | 不变 |
| `unit19-solution` | rc=0 · kept 30/22 | rc=0 · kept 30/**3** + `argmismatch_text` 19 | 不变（但没消干净） |
| `unit19-cardinal-le` | rc=0 · kept 22/10 | rc=0 · kept 22/**0** + `argmismatch_text` 10 | 不变（同上） |
| **`G19-zero-ary-notation-applied`** | **rc=0** | **rc=1** | **破红线** |

G19 的新诊断：`elab-anon-ctor-no-expected-type` @ **43:3**（`∅ ⊆ A` 那条**声明的类型**），
即内核把输入元变量**赋值**了，但这次赋值**只在会话内**；前端 `\0soko_m{id}` 表不知道它
（`meta.rs` 冻结、没有回传通道）⇒ 同一 id 的另一处实例与内核那份**不一致** ⇒ 外层重建出的类型坏掉。

⇒ **C4 的唯一真修法 = 赋值回传**（内核把会话里的赋值表交出来，前端 `meta.rs` 应用它）。
这也是 `prefix_meta`（52 次，记法重试在占位符前缀上直接放弃）必须依赖的同一块基础设施 ——
所以 C4 与记法侧的那一刀应当**同一批**做，而不是先接一半。

## 9. 回传（用户 2026-11-03 解冻 `meta.rs`）——内核半已落，前端半是下一片

### 9.1 内核半 ✓（本轮）

* `MetaContext.externals`：`app_register_externals` / `app_register_inputs` 登记时 `note_external(id)`。
* `TypeChecker::take_external_assignments(&mut self, scope: &[String]) -> Vec<(u32, String)>`：
  会话里**调用方登记**且**已赋值**的元变量，用 `with_pp_scoped(scope, ..)` 渲染成**文本**
  —— 本仓既有纪律就是「渲染成文本后才能跨检查存活」（`with_tc_seeded` 的文档原话）；
  `scope` 播种 binder 名 ⇒ 开项印真名而不是 `$3 $2 $1`。
* `ExportFile::elab_app_collect(limit, req, scope) -> (AppOutcome, Vec<(u32, String)>)`：
  `elab_app` 的孪生出口，多交这一份表；**没有登记过元变量时第二项恒空**。
* 判据：714 行逐字节 **0 差异**（bin `093256fb` vs base `4895e71c`）· 内核 **130/130** ·
  新测试 `c4_external_assignment_is_exported_as_text`（赋值 → 文本非空、take 语义只交一次）。

### 9.2 前端半（下一片，契约）

| 步骤 | 落点（都不是冻结文件） |
|---|---|
| ① 换出口 | `kernel_app.rs::run_once` 的两处 `builder.with_env(|ef| ef.elab_app(limit, &req))` → `elab_app_collect(limit, &req, &scope_names)`；`scope_names = scope.names.clone()`（外层在前，与 `binder_tys` 同序） |
| ② 写回 | 新模块 `crates/front/src/compile/kernel_app_readback.rs`（~60 行）：`meta_parse::parse_expr_text_meta(text)` → `MetaStore::assign(MetaId(id), expr)`。store 拿法照 `elab.rs:2966` 的 `ctx.metas.and_then(|m| m.store.as_ref())` |
| ③ 失败纪律 | 解析失败 / store 已有**不同**值 / `assign` 回 `Err`（occurs·type-occurs·scope）⇒ **跳过并计数**，绝不静默覆盖（`assign` 自带三道闸门 ✓） |
| ④ 判据 | ①G19 在**重新接上 C4 之后**仍 `rc=0`；②那 4 个文件 `nofallback_kept_*` 归零；③714 行**仍然逐字节相同**（判卷结论不变）；④四门绿 + 258/2191/0 |

⚠ 关键未知（下一片一上手就要验）：内核 pp 的文本能否被 `meta_parse` 原样读回（尤其 `@` 形式、
Sort/宇宙层、以及 binder 名与前端作用域的同名遮蔽）。仓库里这条 parser 之前只吃过**类型文本**
（`elab.rs:5527` 的 `?mN` 类型），**没有吃过值文本**的判据 ⇒ 第一件事是加一条单测钉死它。

## 10. 前端半**做完了**，但结论改了：C4 的收益是 **0**（2026-11-03 晚）

按 §9.2 的契约把前端半做完（`kernel_app_readback.rs` + `elab_app_collect` 接线 + 重接 C4），
**判卷结论全保住了**（这是回传的功劳）：

| 文件 | 不写回（C4 单独） | **写回之后（C4 + read-back）** | JSON 哈希 vs 基线 |
|---|---|---|---|
| `G19` | **rc=1** ✗ | **rc=0** ✓ | **逐字节相同**（`6355c0b8`） |
| `G48` | rc=0 | rc=0 | **逐字节相同**（`7baa357f`） |
| `unit19-solution` | rc=0 | rc=0 | **逐字节相同**（`47260191`） |
| `unit19-cardinal-le` | rc=0 | rc=0 | **逐字节相同**（`580b935c`） |

⇒ 「赋值不回传」确实是 G19 变红的根因，回传把它治好了。

**但是 kept 计数一格没少，反而换了名字**：

| 文件 | C1–C3 | C4 + read-back |
|---|---|---|
| `unit19-solution` | `prefix_meta` 30 · `input_contains_a_meta` 22 | `prefix_meta` 30 · **`argmismatch_text` 19** · `input_contains_a_meta` 3 |
| `G19` / `G48` | `input_contains_a_meta` 9 / 3 | **`panic` 9 / 3**（内核裸 `panic!` = 拒绝，被 `catch_unwind` 收成 `Unsupported("panic")`） |

**结论（这是本轮最重要的技术判定）**：内核现在**接受并真的跑了**这些站点（22 → 3+19、9/3 → panic），
但**判不出** —— 19 个退化成 `ArgMismatch`（元变量合一后的类型与域不符），零散几个把内核逼进
`panic!` 拒绝。也就是说：

> **老路能过这些站点靠的是 `implicit::fill_pending_by_shape`（借同形兄弟）—— 一条 Lean 没有的启发式。
> strict 要消掉这 4 个文件，就必须在 strict 路径里**照抄那条猜法**；否则内核在信息不足时只能拒绝。
> 而 strict 的设计从一开始就写着「不新造猜法」（S4b §、S4c §）。**

⇒ 这条不是"再写点代码就能消"的活，而是**产品判定**：
1. **接受**这 4 个文件为已知偏差（例外表 + `strict-invariant-baseline.json` 的 allowlist，已写好机制），或
2. **把 `fill_pending_by_shape` 搬进 strict 的记法/应用路径**（= 承认 strict 也猜，与 §9.39 的初衷冲突），或
3. **让 loose 也拒绝**（会破课程文件，违反本轮硬约束）。

⇒ 本轮据此**还原** C4 + read-back（判卷虽不变，但收益 0、还引入 panic 站点），保留
C1–C3（零行为变化的内核能力）与回传的内核半（下一轮若要走选项 2，它是现成的地基）。

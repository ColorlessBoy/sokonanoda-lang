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

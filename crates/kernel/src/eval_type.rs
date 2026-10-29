//! `value_type`（从 `eval.rs` 拆出：判据 ⑨ 的行数预算）。
use crate::tc::TypeChecker;
use std::cell::OnceCell;
use crate::value::{self, RigidHead, Value, V};

impl<'x, 't, 'p> TypeChecker<'x, 't, 'p> {
    pub(crate) fn value_type(&mut self, depth: u32, v: V<'t>) -> V<'t> {
        let v = self.force_thunk(depth, v);
        match v {
            Value::Sort { level , ..} => {
                let s = self.ctx.succ(*level);
                value::mk_sort(self.arena, self.ctx.simplify(s))
            }
            Value::NatLit { .. } => {
                let n = self.ctx.export_file.name_cache.nat.expect("value_type: Nat name missing");
                let levels = self.ctx.alloc_levels_slice(&[]);
                value::mk_rigid_head_with_empty(self.arena, RigidHead::Inductive(n, levels), self.empty_spine())
            }
            Value::StrLit { .. } => {
                let n = self.ctx.export_file.name_cache.string.expect("value_type: String name missing");
                let levels = self.ctx.alloc_levels_slice(&[]);
                value::mk_rigid_head_with_empty(self.arena, RigidHead::Inductive(n, levels), self.empty_spine())
            }
            // **S1**：元变量的类型 = 上下文里登记的类型。
            Value::Meta { ty, spine, .. } if spine.is_empty() => crate::mvar::value_type_meta(self, depth, *ty),
            // **K1a**：`?m a ..` 的类型 = 沿 spine 实例化 `?m` 的类型。
            Value::Meta { id, ty, spine, .. } => {
                let (id, ty, spine) = (*id, *ty, *spine);
                let head_ty = crate::mvar::value_type_meta(self, depth, ty);
                let prev = crate::mvar::mk_meta(self.arena, id, ty, self.empty_spine());
                self.spine_type_with_value(depth, head_ty, prev, spine)
            }
            Value::Rigid { head, spine , ..} => {
                let head_ty = self.rigid_head_type(depth, *head);
                let prev = value::mk_rigid_head_with_empty(self.arena, *head, self.empty_spine());
                self.spine_type_with_value(depth, head_ty, prev, spine)
            }
            Value::Unfold { head, spine, .. } => {
                let head_ty = self.const_head_type(head.name, head.levels);
                let cell = &*self.arena.alloc(OnceCell::new());
                let _ = cell.set(head_ty);
                let prev =
                    value::mk_unfold_head_with_empty(self.arena, head.name, head.levels, cell, self.empty_spine());
                self.spine_type_with_value(depth, head_ty, prev, spine)
            }
            // **K1a**：闭的 Lam/Pi ⇒ quote 回项再 `infer`（`?p := fun x => P x`、`?α := A → B`
            // 的赋值前类型检查走这里）。开的（含外层 bvar）仍不支持 ⇒ 原样 panic（K1b 作用域纪律）。
            Value::Pi { .. } | Value::Lam { .. } => {
                let e = self.quote(depth, v);
                if e.num_loose_bvars() != 0 {
                    // **K1c**：开的 Lam/Pi ⇒ 从值图里收 bvar 的类型，重建 env/ctx 再 `infer`。
                    return self.open_binder_type(depth, v, e);
                }
                let (env, ctx) = (self.empty_env(), self.empty_ctx());
                self.infer_value(crate::tc::InferFlag::InferOnly, 0, env, ctx, e)
            }
            Value::Thunk { .. } => unreachable!("value_type: Thunk after force"),
        }
    }

    /// **K1c**：开的 Lam/Pi（`e` = 它在深度 `depth` 下 quote 出的项，带 loose `Var`）的类型。
    /// quote 出的每个 loose `Var` 都来自值图里某个 `RigidHead::BVar(lvl, ty)` ⇒ 收齐 `lvl ↦ ty`，
    /// 按层 0..depth 重建 env（同层同类型的 bvar 值）与 ctx，再走普通 `infer`。
    fn open_binder_type(&mut self, depth: u32, v: V<'t>, e: crate::util::ExprPtr<'t>) -> V<'t> {
        let mut tys: Vec<Option<V<'t>>> = vec![None; depth as usize];
        let mut seen: crate::util::FxHashSet<usize> = crate::util::FxHashSet::default();
        collect_bvar_tys(v, &mut tys, &mut seen);
        let (mut env, mut ctx) = (self.empty_env(), self.empty_ctx());
        for (lvl, ty) in tys.iter().enumerate() {
            let ty = match ty {
                Some(t) => *t,
                None => {
                    // 没在值图里出现的层 ⇒ `e` 不该提到它（否则是不变式被破坏，响亮报错）。
                    let idx = u16::try_from(depth as usize - 1 - lvl).expect("value_type: depth exceeds u16");
                    assert!(!self.ctx.has_loose_bvar(e, idx), "value_type: open binder mentions a bvar of unknown type");
                    value::mk_sort(self.arena, self.ctx.zero())
                }
            };
            #[allow(clippy::cast_possible_truncation)]
            let fresh = self.mk_bvar_hc(lvl as u32, ty);
            env = value::env_extend(self.arena, env, fresh);
            ctx = value::ctx_extend(self.arena, ctx, ty);
        }
        self.infer_value(crate::tc::InferFlag::InferOnly, depth, env, ctx, e)
    }
}

/// 值图里所有 `BVar(lvl, ty)` 的类型（按指针去重，DAG 线性）。
fn collect_bvar_tys<'t>(v: V<'t>, out: &mut [Option<V<'t>>], seen: &mut crate::util::FxHashSet<usize>) {
    if !seen.insert(v as *const Value<'t> as usize) {
        return;
    }
    match v {
        Value::Rigid { head, spine, .. } => {
            if let RigidHead::BVar(lvl, ty) = head {
                if let Some(slot) = out.get_mut(*lvl as usize) {
                    slot.get_or_insert(*ty);
                }
                collect_bvar_tys(ty, out, seen);
            }
            collect_spine(spine, out, seen);
        }
        Value::Unfold { spine, .. } | Value::Meta { spine, .. } => collect_spine(spine, out, seen),
        Value::Lam { body, .. } => collect_env(body.env, out, seen),
        Value::Pi { domain, body, .. } => {
            collect_bvar_tys(domain, out, seen);
            collect_env(body.env, out, seen);
        }
        Value::Thunk { env, .. } => collect_env(env, out, seen),
        Value::Sort { .. } | Value::NatLit { .. } | Value::StrLit { .. } => {}
    }
}

fn collect_spine<'t>(s: value::S<'t>, out: &mut [Option<V<'t>>], seen: &mut crate::util::FxHashSet<usize>) {
    for elim in s.to_vec() {
        if let value::ElimView::App(a) = elim.view() {
            collect_bvar_tys(a, out, seen);
        }
    }
}

fn collect_env<'t>(e: value::E<'t>, out: &mut [Option<V<'t>>], seen: &mut crate::util::FxHashSet<usize>) {
    match e {
        value::Env::Nil { .. } => {}
        value::Env::Cons { v, parent, .. } => {
            collect_bvar_tys(v, out, seen);
            collect_env(parent, out, seen);
        }
        value::Env::Framed { slots, .. } => {
            for v in slots.iter() {
                collect_bvar_tys(v, out, seen);
            }
        }
    }
}

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
            Value::Meta { ty, .. } => crate::mvar::value_type_meta(self, depth, *ty),
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
            Value::Pi { .. } | Value::Lam { .. } => panic!("value_type: Pi/Lam not supported"),
            Value::Thunk { .. } => unreachable!("value_type: Thunk after force"),
        }
    }

}

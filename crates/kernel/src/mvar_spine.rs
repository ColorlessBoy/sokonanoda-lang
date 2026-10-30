//! **带 spine 的元变量合一**（K1a/K1c；从 `mvar.rs` 拆出，`docs-lint` ⑨ 软线 500 行）。
//!
//! `?m a ..` 的重放（已赋值）、同 id 逐项比、一阶赋值与 Miller pattern 的分派；
//! `conv.rs` / `mvar.rs` 各只留一行调用。只是搬家：语义与拆出前逐字相同。
use crate::tc::TypeChecker;
use crate::util::ExprPtr;
use crate::value::{ElimView, Value, S, V};

/// 非空 spine 的元变量（`?m a ..`）⇒ `(id, ty, spine)`；空 spine / 非元变量 ⇒ `None`。
fn meta_app<'t>(v: V<'t>) -> Option<(u32, ExprPtr<'t>, S<'t>)> {
    match v {
        Value::Meta { id, ty, spine, .. } if !spine.is_empty() => Some((*id, *ty, *spine)),
        _ => None,
    }
}

impl<'x, 't, 'p> TypeChecker<'x, 't, 'p> {
    /// **K1a**：`?id` 已赋值 ⇒ 把赋值求值后**重放** `spine`（`apply`/`do_proj` 共用）。
    /// 未赋值 ⇒ `None`（调用方造卡住项）。
    pub(crate) fn meta_replay(&mut self, depth: u32, id: u32, spine: S<'t>) -> Option<V<'t>> {
        let term = self.ctx.mvars.value(id)?;
        let elims: Vec<crate::value::Elim<'t>> = spine.to_vec().into_iter().copied().collect();
        let env = self.empty_env();
        let mut f = self.eval(depth, env, term);
        for elim in elims {
            f = match elim.view() {
                ElimView::App(a) => self.apply(depth, f, a),
                ElimView::Proj { ty_name, idx } => self.do_proj(depth, ty_name, idx, f),
            };
        }
        Some(f)
    }

    /// **K1a**：带 spine 的元变量在合一里的规则（Miller 模式留给后一片）。
    /// * 已赋值 ⇒ 重放后再比；
    /// * 同 id ⇒ spine 逐项比（Lean `isDefEqMVarSelf` 的保守版）；
    /// * 另一侧是**空 spine** 元变量 ⇒ 走原 `unify_meta`（一阶：`?q := ?p a`）；
    /// * 其余（非空 spine 对刚性项/异 id 元变量应用）⇒ 干净地 `false`。
    fn meta_spine_branch(&mut self, depth: u32, x: V<'t>, y: V<'t>) -> Option<bool> {
        let (mx, my) = (meta_app(x), meta_app(y));
        if mx.is_none() && my.is_none() {
            return None;
        }
        if let Some((id, _, sp)) = mx {
            if let Some(v) = self.meta_replay(depth, id, sp) {
                return Some(self.unify::<true>(depth, v, y));
            }
        }
        if let Some((id, _, sp)) = my {
            if let Some(v) = self.meta_replay(depth, id, sp) {
                return Some(self.unify::<true>(depth, x, v));
            }
        }
        if let (Some((ix, tx, sx)), Some((iy, ty, sy))) = (mx, my) {
            if ix == iy {
                // Lean `isDefEqMVarSelf`：先逐项比；失败 ⇒ 撤销逐项比的赋值，再试赋值
                // （Miller；同 id 必被 occurs 拒 ⇒ 不做常量近似）。
                let cp = self.ctx.mvars.checkpoint();
                if self.meta_spines_eq(depth, sx, sy) {
                    return Some(true);
                }
                if self.ctx.mvars.restore(cp) {
                    self.tc_cache.clear();
                }
                return Some(self.miller_assign(depth, ix, tx, sx, y));
            }
            // **K1c**：异 id ⇒ 先试赋左、再试赋右（各自失败即回滚）。
            return Some(self.miller_assign(depth, ix, tx, sx, y) || self.miller_assign(depth, iy, ty, sy, x));
        }
        // 恰一侧非空 spine：另一侧若是空 spine 元变量 ⇒ 一阶赋值它。
        let other = if mx.is_some() { y } else { x };
        let app = if mx.is_some() { x } else { y };
        if let Value::Meta { id, ty, .. } = other {
            return Some(self.unify_meta(depth, *id, *ty, app));
        }
        // **K1c**：`?m xs =?= t` ⇒ Miller pattern（非 pattern ⇒ 干净的 false）。
        let (id, ty, sp) = mx.or(my).expect("one side is a meta app");
        Some(self.miller_assign(depth, id, ty, sp, other))
    }

    fn meta_spines_eq(&mut self, depth: u32, sx: S<'t>, sy: S<'t>) -> bool {
        let (ex, ey) = (sx.to_vec(), sy.to_vec());
        if ex.len() != ey.len() {
            return false;
        }
        let pairs: Vec<_> = ex.into_iter().copied().zip(ey.into_iter().copied()).collect();
        for (a, b) in pairs {
            let ok = match (a.view(), b.view()) {
                (ElimView::App(va), ElimView::App(vb)) => self.unify::<true>(depth, va, vb),
                (ElimView::Proj { ty_name: tx, idx: ix }, ElimView::Proj { ty_name: ty, idx: iy }) =>
                    tx == ty && ix == iy,
                _ => false,
            };
            if !ok {
                return false;
            }
        }
        true
    }

    /// `unify` 的元变量拦截（`conv.rs` 只留一行调用）。
    pub(crate) fn meta_unify_branch(&mut self, depth: u32, x: V<'t>, y: V<'t>) -> Option<bool> {
        if let Some(r) = self.meta_spine_branch(depth, x, y) {
            return Some(r);
        }
        if let Value::Meta { id, ty, .. } = &*x {
            return Some(self.unify_meta(depth, *id, *ty, y));
        }
        if let Value::Meta { id, ty, .. } = &*y {
            return Some(self.unify_meta(depth, *id, *ty, x));
        }
        None
    }

    /// `def_eq_core`/`def_eq_at` 的"元变量优先于证明无关"分支。
    pub(crate) fn meta_def_eq_branch(&mut self, depth: u32, vx: V<'t>, vy: V<'t>) -> Option<bool> {
        if matches!(&*vx, Value::Meta { .. }) || matches!(&*vy, Value::Meta { .. }) {
            return Some(self.unify::<true>(depth, vx, vy));
        }
        None
    }
}

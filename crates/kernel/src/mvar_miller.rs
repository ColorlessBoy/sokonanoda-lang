//! **K1c**：Miller pattern 赋值（Lean `Meta/ExprDefEq.lean` `processAssignment` 的 pattern 子集）。
//!
//! `?m x1..xk =?= t`，`xi` 两两不同、都是 `?m` 创建之后引入的 bvar ⇒ `?m := λxs. t`，
//! 前置 scope（`t` 只许提到 `xs`；赋值以闭项存储）+ occurs + 类型检查。
//! 其余（非 bvar 实参 / 重复实参 / `?m` 上下文里的 bvar）⇒ 干净的 `false`：**不做**一阶/常量近似。
use crate::expr::{BinderStyle, Expr, MetaKind};
use crate::tc::TypeChecker;
use crate::util::ExprPtr;
use crate::value::{ElimView, RigidHead, Value, S, V};

impl<'x, 't, 'p> TypeChecker<'x, 't, 'p> {
    /// spine 是 Miller pattern ⇒ 每个实参的 `(层, 类型)`；否则 `None`。
    fn miller_args(&mut self, depth: u32, id: u32, spine: S<'t>) -> Option<Vec<(u32, V<'t>)>> {
        let floor = self.ctx.mvars.depth_of(id);
        let elims: Vec<crate::value::Elim<'t>> = spine.to_vec().into_iter().copied().collect();
        let mut out: Vec<(u32, V<'t>)> = Vec::with_capacity(elims.len());
        for elim in elims {
            let ElimView::App(a) = elim.view() else { return None };
            let a = self.force_thunk(depth, a);
            let Value::Rigid { head: RigidHead::BVar(lvl, ty), spine: s, .. } = a else { return None };
            let (lvl, ty) = (*lvl, *ty);
            if !s.is_empty() || lvl < floor || lvl >= depth || out.iter().any(|(l, _)| *l == lvl) {
                return None;
            }
            out.push((lvl, ty));
        }
        Some(out)
    }

    /// 把在深度 `d` 下 quote 出来的 `e` 搬到 `xs.len()` 个 λ 之下：层 `xs[j]` ⇒ `Var(off + n-1-j)`；
    /// 其它 loose bvar（scope 之外）⇒ `None`。`off` = `e` 内部已走过的 binder 数。
    fn abstract_bvars(&mut self, e: ExprPtr<'t>, d: u32, xs: &[u32], off: u16) -> Option<ExprPtr<'t>> {
        if e.num_loose_bvars() <= off {
            return Some(e);
        }
        Some(match self.ctx.read_expr(e) {
            Expr::Var { dbj_idx, .. } => {
                let lvl = d.checked_sub(1 + u32::from(dbj_idx - off))?;
                let j = xs.iter().position(|x| *x == lvl)?;
                let idx = usize::from(off) + (xs.len() - 1 - j);
                self.ctx.mk_var(u16::try_from(idx).ok()?)
            }
            Expr::App { fun, arg, .. } => {
                let f = self.abstract_bvars(fun, d, xs, off)?;
                let a = self.abstract_bvars(arg, d, xs, off)?;
                self.ctx.mk_app(f, a)
            }
            Expr::Lambda { binder_name, binder_style, binder_type, body, .. } => {
                let t = self.abstract_bvars(binder_type, d, xs, off)?;
                let b = self.abstract_bvars(body, d, xs, off.checked_add(1)?)?;
                self.ctx.mk_lambda(binder_name, binder_style, t, b)
            }
            Expr::Pi { binder_name, binder_style, binder_type, body, .. } => {
                let t = self.abstract_bvars(binder_type, d, xs, off)?;
                let b = self.abstract_bvars(body, d, xs, off.checked_add(1)?)?;
                self.ctx.mk_pi(binder_name, binder_style, t, b)
            }
            Expr::Let { data, .. } => {
                let t = self.abstract_bvars(data.binder_type, d, xs, off)?;
                let v = self.abstract_bvars(data.val, d, xs, off)?;
                let b = self.abstract_bvars(data.body, d, xs, off.checked_add(1)?)?;
                self.ctx.mk_let(data.binder_name, t, v, b, data.nondep)
            }
            Expr::Proj { ty_name, idx, structure, .. } => {
                let s = self.abstract_bvars(structure, d, xs, off)?;
                self.ctx.mk_proj(ty_name, idx, s)
            }
            Expr::Sort { .. } | Expr::Const { .. } | Expr::StringLit { .. } | Expr::NatLit { .. } | Expr::Meta { .. } =>
                e,
        })
    }

    /// **K1c**：`?id spine =?= t`（`?id` 未赋值）。成功 ⇒ 赋 `?id := λxs. t` 并返回 `true`；
    /// 任何一步不成立 ⇒ 撤销本次尝试顺带做的赋值（类型检查里的 `?α`）并返回 `false`。
    pub(crate) fn miller_assign(&mut self, depth: u32, id: u32, ty: ExprPtr<'t>, spine: S<'t>, t: V<'t>) -> bool {
        if self.ctx.mvars.kind(id) == Some(MetaKind::SyntheticOpaque) || self.ctx.mvars.value(id).is_some() {
            return false;
        }
        let Some(xs) = self.miller_args(depth, id, spine) else { return false };
        let lvls: Vec<u32> = xs.iter().map(|x| x.0).collect();
        // **scope**：`t` 只许提到 `xs`（以及闭的东西）。
        let body = self.quote(depth, t);
        let Some(mut cand) = self.abstract_bvars(body, depth, &lvls, 0) else { return false };
        let anon = self.ctx.anonymous();
        for j in (0..xs.len()).rev() {
            let (lvl, bty) = xs[j];
            let q = self.quote(lvl, bty);
            // `xj` 的类型只许提到 `x0..x(j-1)`。
            let Some(q) = self.abstract_bvars(q, lvl, &lvls[..j], 0) else { return false };
            cand = self.ctx.mk_lambda(anon, BinderStyle::Default, q, cand);
        }
        // **occurs**：`?m x =?= f (?m x)` 拒绝。
        if crate::mvar::expr_mentions_meta(id, cand) {
            return false;
        }
        // **类型检查**：`infer(λxs. t) ≟ T(?m)`（Lean `isTypeCorrect` + `checkTypesAndAssign`）。
        if !self.ctx.mvars.begin_check(id) {
            return false;
        }
        let cp = self.ctx.mvars.checkpoint();
        let ok = {
            let (env, ctx) = (self.empty_env(), self.empty_ctx());
            let ct = self.infer_value(crate::tc::InferFlag::InferOnly, depth, env, ctx, cand);
            let tt = self.eval(depth, env, ty);
            self.unify::<true>(depth, ct, tt)
        };
        self.ctx.mvars.end_check(id);
        if !ok {
            if self.ctx.mvars.restore(cp) {
                self.tc_cache.clear();
            }
            return false;
        }
        // 类型检查可能赋了 `?α` ⇒ 重新实例化候选（存储里不留已赋值的链头）。
        let cand = self.ctx.instantiate_mvars(cand);
        self.ctx.mvars.assign(id, cand);
        self.invalidate_on_assign();
        true
    }
}

//! [`crate::elab_app`] 的会话内辅助（拆出以守住 ~500 行）：外部元变量登记（**P2-9**）、上下文元变量的造法、
//! 推迟位（P2-1/2/3）的 `NeedArgs` 收尾与 λ binder 预统一。公开 API 仍只在 `elab_app`。
//!
//! Lean 对照（v4.35.0）：`Elab/App.lean:433-443` `elabAndAddNewArg` 的 `expectedType` 可以含**同一 mctx**
//! 的元变量；这里会话是新的 ⇒ 外部元变量按 id 重新登记（同 `solve_metas`），本会话的 id 从外部最大 id 之上分配。
use crate::elab_app::{name_string, AppOutcome, SESSION_META_BASE};
use crate::expr::{BinderStyle, Expr};
use crate::tc::TypeChecker;
use crate::util::{ExprPtr, NamePtr};
use crate::value::{Value, E, V};

/// 第 `k` 个本会话元变量的 id（`base + k`；溢出 ⇒ `None`）。
pub(crate) fn meta_id(base: u32, k: usize) -> Option<u32> { u32::try_from(k).ok().and_then(|k| base.checked_add(k)) }

impl<'x, 't, 'p: 't> TypeChecker<'x, 't, 'p> {
    /// **P2-9**：登记 `expected` 里的外部元变量（调用方会话的 `?M : Π scope, T`，闭、深度 0）⇒ 本会话的
    /// `is_def_eq_at` 可以赋值它们（同 `solve_metas` 的 `register_at(id, ty, 0)` + `register_kind`）。
    /// 返回外部 id（按出现顺序、去重）与本会话元变量的起始 id（`max(SESSION_META_BASE, 最大外部 id + 1)`）。
    /// 层元变量 / 类型不闭 / 类型里还有元变量 / 同 id 两个类型 ⇒ 不支持。
    pub(crate) fn app_register_externals(&mut self, e: ExprPtr<'t>) -> Result<(Vec<u32>, u32), &'static str> {
        if !self.ctx.unassigned_mvars(e).levels.is_empty() {
            return Err("input contains a meta");
        }
        let mut found: Vec<(u32, crate::expr::MetaKind, ExprPtr<'t>)> = Vec::new();
        let mut seen = crate::util::FxHashSet::default();
        let mut stack = vec![e];
        while let Some(e) = stack.pop() {
            if !seen.insert(e) {
                continue;
            }
            match self.ctx.read_expr(e) {
                Expr::Meta { id, kind, ty, .. } => match found.iter().find(|(i, ..)| *i == id) {
                    Some((_, _, t)) if *t != ty => return Err("meta type clash"),
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
            if ty.num_loose_bvars() != 0 {
                return Err("open external meta");
            }
            let u = self.ctx.unassigned_mvars(ty);
            if !u.exprs.is_empty() || !u.levels.is_empty() {
                return Err("input contains a meta");
            }
            self.ctx.mvars.register_at(id, ty, 0);
            self.ctx.mvars.register_kind(id, kind);
            base = base.max(id.checked_add(1).ok_or("meta id space")?);
        }
        Ok((found.into_iter().map(|(id, ..)| id).collect(), base))
    }

    /// 上下文元变量：域 `dom`（在 `d` 个 scope binder 下）⇒ 闭的 `?id : Π xs, dom` 登记在深度 0，
    /// 返回 `?id x1..xd`（项与值）。
    pub(crate) fn app_fresh_meta(
        &mut self,
        d: u32,
        env: E<'t>,
        binder_tys: &[ExprPtr<'t>],
        dom: V<'t>,
        id: u32,
    ) -> (ExprPtr<'t>, V<'t>) {
        let anon = self.ctx.anonymous();
        let mut mty = self.quote(d, dom);
        for t in binder_tys.iter().rev() {
            mty = self.ctx.mk_pi(anon, BinderStyle::Default, *t, mty);
        }
        self.ctx.mvars.register_at(id, mty, 0);
        self.ctx.mvars.register_kind(id, crate::expr::MetaKind::Natural);
        let mut m = self.ctx.mk_meta(id, mty);
        // 调用方已拒绝 > u16 的 scope（`elab_app_session` 的 "scope too deep"）。
        let d16 = u16::try_from(binder_tys.len()).expect("scope depth checked by caller");
        for k in (0..d16).rev() {
            let v = self.ctx.mk_var(k);
            m = self.ctx.mk_app(m, v);
        }
        let mv = self.eval(d, env, m);
        (m, mv)
    }

    /// Lean `elabAndAddNewArg`：推迟位的期望类型 = 代入当前解后的域。**P2-2**（Lean `synthesizeSyntheticMVars`
    /// 的"有进展就再来一轮"）：域里还有未赋值元变量的位 ⇒ `blocked`（**P2-9**：连同带元变量的域），其余交出去；
    /// 一个都交不出 ⇒ `Unsolved`。
    pub(crate) fn app_need_args(
        &mut self,
        d: u32,
        holes: &[(usize, V<'t>, NamePtr<'t>)],
        mut unsolved: Vec<String>,
    ) -> AppOutcome {
        let (mut domains, mut blocked) = (Vec::with_capacity(holes.len()), Vec::new());
        let mut first_blocked = None;
        for &(ai, dom, bn) in holes {
            let q = self.quote(d, dom);
            let q = self.ctx.instantiate_mvars(q);
            if !self.ctx.unassigned_mvars(q).is_empty() {
                first_blocked.get_or_insert(bn);
                blocked.push((ai, self.ctx.export_owned(q)));
                continue;
            }
            domains.push((ai, self.ctx.export_owned(q)));
        }
        if domains.is_empty() {
            if let (true, Some(bn)) = (unsolved.is_empty(), first_blocked) {
                unsolved.push(name_string(self.ctx, bn));
            }
            return AppOutcome::Unsolved { binders: unsolved };
        }
        AppOutcome::NeedArgs { domains, blocked }
    }

    /// **P2-3**：推迟的 λ 位的带注解 binder 类型逐层与域的 Pi 层 `isDefEq`（Lean `propagateExpectedType`，
    /// `Binders.lean:417` `discard <| isDefEq fvarType d`：失败不致命、只为赋值）。域不再是 Pi ⇒ 停。
    pub(crate) fn app_unify_binders(&mut self, d: u32, env: E<'t>, dom: V<'t>, tys: &[ExprPtr<'t>]) {
        let (mut depth, mut env_i, mut cur) = (d, env, dom);
        for ty in tys {
            let pi = match self.weak_pi(depth, cur) {
                Some(p) => p,
                None => match self.force_pi(depth, cur) {
                    Some(p) => p,
                    None => return,
                },
            };
            let Value::Pi { domain, body, .. } = pi else { return };
            let (domain, body) = (*domain, *body);
            let bt = self.eval(depth, env_i, *ty);
            let _ = self.is_def_eq_at(depth, domain, bt);
            let fresh = self.mk_bvar_hc(depth, bt);
            env_i = crate::value::env_extend(self.arena, env_i, fresh);
            cur = self.apply_closure(depth + 1, &body, fresh, Some(domain));
            depth += 1;
        }
    }
}

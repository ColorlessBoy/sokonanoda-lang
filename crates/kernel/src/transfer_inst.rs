//! **F2a**：builder 侧的元变量代入（Lean `instantiateMVars` 的 builder 版；会话版见 `mvar_inst.rs`）。
//!
//! 赋值是**闭项**（`unify_meta`/`miller_assign` 的 scope 规则保证）⇒ 代进 binder 下不必升降 ⇒
//! 按节点指针记忆即可（结果与 binder 深度无关）。唯一要动 de Bruijn 的是 `?p a ..` 的 head-beta：
//! 实参可以是开项（Miller：`?p x`，`x` 是 bvar）⇒ 代入时按穿过的 binder 数 lift 实参、
//! 并把剩下的 loose var 降一格。纯加法，没有既有判定路径调用它。
use crate::builder::EnvBuilder;
use crate::expr::Expr;
use crate::level::Level;
use crate::util::{ExprPtr, FxHashMap, LevelPtr};
use std::collections::HashMap;

/// 赋值链/层赋值的展开上限（外部给的表可能有环）。
const FUEL: u32 = 256;

impl<'a> EnvBuilder<'a> {
    /// **F2a**：把 `e` 里的 `Expr::Meta{id}` / `Level::MVar` 换成表里的赋值（递归、顺链），
    /// `?p a ..` 且 `?p` 换成 λ ⇒ head-beta。表外的元变量原样保留（类型不展开，Lean 同）。
    pub fn instantiate_metas(
        &mut self,
        e: ExprPtr<'a>,
        exprs: &HashMap<u32, ExprPtr<'a>>,
        levels: &HashMap<u64, LevelPtr<'a>>,
    ) -> ExprPtr<'a> {
        let mut memo = FxHashMap::default();
        self.inst_metas_go(e, exprs, levels, &mut memo, 0)
    }

    /// 层的代入（顺链）。
    pub fn instantiate_level_metas(&mut self, l: LevelPtr<'a>, levels: &HashMap<u64, LevelPtr<'a>>) -> LevelPtr<'a> {
        self.inst_lvl_go(l, levels, 0)
    }

    fn inst_lvl_go(&mut self, l: LevelPtr<'a>, levels: &HashMap<u64, LevelPtr<'a>>, fuel: u32) -> LevelPtr<'a> {
        match *l.as_ref() {
            Level::Zero | Level::Param(..) => l,
            Level::Succ(a, _) => {
                let a2 = self.inst_lvl_go(a, levels, fuel);
                if a2 == a { l } else { self.succ(a2) }
            }
            Level::Max(a, b, _) => {
                let (a2, b2) = (self.inst_lvl_go(a, levels, fuel), self.inst_lvl_go(b, levels, fuel));
                if a2 == a && b2 == b { l } else { self.level_max(a2, b2) }
            }
            Level::IMax(a, b, _) => {
                let (a2, b2) = (self.inst_lvl_go(a, levels, fuel), self.inst_lvl_go(b, levels, fuel));
                if a2 == a && b2 == b { l } else { self.level_imax(a2, b2) }
            }
            Level::MVar(id, _) => match levels.get(&id) {
                Some(v) if fuel < FUEL => self.inst_lvl_go(*v, levels, fuel + 1),
                _ => l,
            },
        }
    }

    fn inst_metas_go(
        &mut self,
        e: ExprPtr<'a>,
        exprs: &HashMap<u32, ExprPtr<'a>>,
        levels: &HashMap<u64, LevelPtr<'a>>,
        memo: &mut FxHashMap<ExprPtr<'a>, ExprPtr<'a>>,
        fuel: u32,
    ) -> ExprPtr<'a> {
        if let Some(r) = memo.get(&e) {
            return *r;
        }
        let r = match *e.as_ref() {
            Expr::Var { .. } | Expr::StringLit { .. } | Expr::NatLit { .. } => e,
            Expr::Meta { id, .. } => match exprs.get(&id) {
                Some(v) if fuel < FUEL => self.inst_metas_go(*v, exprs, levels, memo, fuel + 1),
                _ => e,
            },
            Expr::Sort { level, .. } => {
                let l2 = self.inst_lvl_go(level, levels, 0);
                if l2 == level { e } else { self.mk_sort(l2) }
            }
            Expr::Const { name, levels: ls, .. } => {
                let ls2: Vec<LevelPtr<'a>> = ls.iter().map(|l| self.inst_lvl_go(*l, levels, 0)).collect();
                if ls2.iter().zip(ls.iter()).all(|(x, y)| x == y) {
                    e
                } else {
                    let ls2 = self.alloc_levels_slice(&ls2);
                    self.mk_const(name, ls2)
                }
            }
            Expr::App { .. } => {
                let mut args = Vec::new();
                let mut head = e;
                while let Expr::App { fun, arg, .. } = *head.as_ref() {
                    args.push(arg);
                    head = fun;
                }
                args.reverse();
                let head_was_meta = matches!(head.as_ref(), Expr::Meta { .. });
                let mut f = self.inst_metas_go(head, exprs, levels, memo, fuel);
                let args: Vec<ExprPtr<'a>> =
                    args.into_iter().map(|x| self.inst_metas_go(x, exprs, levels, memo, fuel)).collect();
                let mut rest = args.as_slice();
                // head-beta 只在头是被换掉的元变量时（Lean `visitApp … headBeta` 同）。
                if head_was_meta && f != head {
                    while let (Expr::Lambda { body, .. }, Some((x, tl))) = (*f.as_ref(), rest.split_first()) {
                        f = self.beta_subst(body, *x);
                        rest = tl;
                    }
                }
                let mut acc = f;
                for x in rest {
                    acc = self.mk_app(acc, *x);
                }
                acc
            }
            Expr::Lambda { binder_name, binder_style, binder_type, body, .. } => {
                let t2 = self.inst_metas_go(binder_type, exprs, levels, memo, fuel);
                let b2 = self.inst_metas_go(body, exprs, levels, memo, fuel);
                if t2 == binder_type && b2 == body { e } else { self.mk_lambda(binder_name, binder_style, t2, b2) }
            }
            Expr::Pi { binder_name, binder_style, binder_type, body, .. } => {
                let t2 = self.inst_metas_go(binder_type, exprs, levels, memo, fuel);
                let b2 = self.inst_metas_go(body, exprs, levels, memo, fuel);
                if t2 == binder_type && b2 == body { e } else { self.mk_pi(binder_name, binder_style, t2, b2) }
            }
            Expr::Let { data, .. } => {
                let crate::expr::LetData { binder_name, binder_type, val, body, nondep } = *data;
                let t2 = self.inst_metas_go(binder_type, exprs, levels, memo, fuel);
                let v2 = self.inst_metas_go(val, exprs, levels, memo, fuel);
                let b2 = self.inst_metas_go(body, exprs, levels, memo, fuel);
                if t2 == binder_type && v2 == val && b2 == body {
                    e
                } else {
                    self.mk_let(binder_name, t2, v2, b2, nondep)
                }
            }
            Expr::Proj { ty_name, idx, structure, .. } => {
                let s2 = self.inst_metas_go(structure, exprs, levels, memo, fuel);
                if s2 == structure { e } else { self.mk_proj(ty_name, idx, s2) }
            }
        };
        memo.insert(e, r);
        r
    }

    /// `body[0 := x]`：`Var 0` ⇒ `x`（穿过 `k` 个 binder 就 lift `k`），更外的 loose var 降一格。
    pub(crate) fn beta_subst(&mut self, body: ExprPtr<'a>, x: ExprPtr<'a>) -> ExprPtr<'a> {
        let mut memo = FxHashMap::default();
        self.subst_go(body, x, 0, &mut memo)
    }

    fn subst_go(
        &mut self,
        e: ExprPtr<'a>,
        x: ExprPtr<'a>,
        k: u16,
        memo: &mut FxHashMap<(ExprPtr<'a>, u16), ExprPtr<'a>>,
    ) -> ExprPtr<'a> {
        if e.num_loose_bvars() <= k {
            return e;
        }
        if let Some(r) = memo.get(&(e, k)) {
            return *r;
        }
        let r = match *e.as_ref() {
            Expr::Var { dbj_idx, .. } => {
                if dbj_idx == k {
                    self.lift_loose(x, 0, k)
                } else {
                    // `dbj_idx > k`（`< k` 已被上面的 loose 早退兜住）⇒ 被消掉的 binder 在它里面。
                    self.mk_var(dbj_idx - 1)
                }
            }
            Expr::App { fun, arg, .. } => {
                let (f, a) = (self.subst_go(fun, x, k, memo), self.subst_go(arg, x, k, memo));
                self.mk_app(f, a)
            }
            Expr::Lambda { binder_name, binder_style, binder_type, body, .. } => {
                let (t, b) = (self.subst_go(binder_type, x, k, memo), self.subst_go(body, x, k + 1, memo));
                self.mk_lambda(binder_name, binder_style, t, b)
            }
            Expr::Pi { binder_name, binder_style, binder_type, body, .. } => {
                let (t, b) = (self.subst_go(binder_type, x, k, memo), self.subst_go(body, x, k + 1, memo));
                self.mk_pi(binder_name, binder_style, t, b)
            }
            Expr::Let { data, .. } => {
                let crate::expr::LetData { binder_name, binder_type, val, body, nondep } = *data;
                let t = self.subst_go(binder_type, x, k, memo);
                let v = self.subst_go(val, x, k, memo);
                let b = self.subst_go(body, x, k + 1, memo);
                self.mk_let(binder_name, t, v, b, nondep)
            }
            Expr::Proj { ty_name, idx, structure, .. } => {
                let s = self.subst_go(structure, x, k, memo);
                self.mk_proj(ty_name, idx, s)
            }
            // loose bvar 数为 0 ⇒ 已被早退兜住。
            Expr::Sort { .. } | Expr::Const { .. } | Expr::StringLit { .. } | Expr::NatLit { .. } | Expr::Meta { .. } =>
                e,
        };
        memo.insert((e, k), r);
        r
    }

    /// loose var（`≥ cutoff`）加 `amount`。
    fn lift_loose(&mut self, e: ExprPtr<'a>, cutoff: u16, amount: u16) -> ExprPtr<'a> {
        if amount == 0 || e.num_loose_bvars() <= cutoff {
            return e;
        }
        match *e.as_ref() {
            Expr::Var { dbj_idx, .. } => self.mk_var(dbj_idx + amount),
            Expr::App { fun, arg, .. } => {
                let (f, a) = (self.lift_loose(fun, cutoff, amount), self.lift_loose(arg, cutoff, amount));
                self.mk_app(f, a)
            }
            Expr::Lambda { binder_name, binder_style, binder_type, body, .. } => {
                let t = self.lift_loose(binder_type, cutoff, amount);
                let b = self.lift_loose(body, cutoff + 1, amount);
                self.mk_lambda(binder_name, binder_style, t, b)
            }
            Expr::Pi { binder_name, binder_style, binder_type, body, .. } => {
                let t = self.lift_loose(binder_type, cutoff, amount);
                let b = self.lift_loose(body, cutoff + 1, amount);
                self.mk_pi(binder_name, binder_style, t, b)
            }
            Expr::Let { data, .. } => {
                let crate::expr::LetData { binder_name, binder_type, val, body, nondep } = *data;
                let t = self.lift_loose(binder_type, cutoff, amount);
                let v = self.lift_loose(val, cutoff, amount);
                let b = self.lift_loose(body, cutoff + 1, amount);
                self.mk_let(binder_name, t, v, b, nondep)
            }
            Expr::Proj { ty_name, idx, structure, .. } => {
                let s = self.lift_loose(structure, cutoff, amount);
                self.mk_proj(ty_name, idx, s)
            }
            Expr::Sort { .. } | Expr::Const { .. } | Expr::StringLit { .. } | Expr::NatLit { .. } | Expr::Meta { .. } =>
                e,
        }
    }
}

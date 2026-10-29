//! **F2b1**：内核级应用 elaboration（Lean `Elab/App.lean` `ElabAppArgs`，v4.35.0）。
//!
//! `f a1..an`（strict 方言）⇒ 隐式位造元变量、显式位查实参类型、期望类型先传播一次、
//! 末尾 `isDefEq(期望, 结果)`；全部用内核项（零文本）。今天**没有任何判定路径调用它**。
//!
//! Lean 对照：
//! * `App.lean:745-760` `addImplicitArg` ⇒ `mkFreshExprMVar argType`。这里是**上下文元变量**
//!   （Lean/Coq 的 lctx 的 de Bruijn 版）：域 `T`（在 d 个 scope binder 下）⇒ 闭的
//!   `?M : Π x1..xd, T`（深度 0 登记），隐式位放 `?M x1..xd` ⇒ `?M xs =?= P x` 正是 K1c 的 Miller 情形，
//!   赋值仍是闭项（不放宽闭项纪律）。
//! * `App.lean:563-608` `propagateExpectedType` ⇒ 第一个显式位之前一次；期望 whnf 是 `Prop` 跳过（594）；
//!   结果类型依赖未吃的 binder 跳过（`getResultingTypeCore?` 474/497）；失败不致命。
//!   偏离：Lean 失败后会在下一个显式位重试，这里只试一次（按 F2b1 设计）。
//! * `App.lean:433-443` 显式实参 ⇒ `ensureHasType`：这里 `infer(arg) ≟ 域`（带回滚），无 coercion。
//! * `App.lean:610-656` `finalize` ⇒ `isDefEq(期望, eType)`；未赋值隐式在**应用末尾**就报（Lean 留到声明末尾）。
//! * `App.lean:926-951` 主循环：`fType` 不是 Π ⇒ whnf 再看（`synthesizePendingAndNormalizeFunType`）；
//!   严格隐式且没有剩余实参 ⇒ finalize（`processStrictImplicitArg`）。
use crate::env::EnvLimit;
use crate::expr::{BinderStyle, Expr, MetaKind};
use crate::name::Name;
use crate::tc::{InferFlag, TypeChecker};
use crate::transfer::OwnedExpr;
use crate::util::{ExportFile, ExprPtr, FxHashMap, NamePtr, TcCtx};
use crate::value::{Value, E, V};

/// 一次应用 elaboration 的输入。全部项都在 `binder_tys` 这个 scope 之下（loose `Var` = scope binder）。
pub struct AppRequest<'s, 'p> {
    /// scope，最外层在前；第 i 个类型在前 i 个 binder 之下（与 `infer_under_binders` 同约定）。
    pub binder_tys: &'s [ExprPtr<'p>],
    /// 被调用者（P1 里是 `Const`）。
    pub head: ExprPtr<'p>,
    /// 前 k 个 binder 无论写法都按隐式处理（构造子的参数）。
    pub implicit_prefix: usize,
    /// 位置实参，已 elaborate。
    pub explicit_args: &'s [ExprPtr<'p>],
    /// 期望类型（已知时）。
    pub expected: Option<ExprPtr<'p>>,
}

/// [`ExportFile::elab_app`] 的结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppOutcome {
    /// 无元变量的项，在 scope 之下（loose `Var` = scope binder）。
    Solved(OwnedExpr),
    /// 应用末尾仍未赋值的隐式 binder 名（按 telescope 顺序）。
    Unsolved { binders: Vec<String> },
    /// 第 `arg` 个显式实参类型不对。
    ArgMismatch { arg: usize },
    /// 结果类型 ≠ 期望类型。
    ExpectedMismatch,
    /// P1 的限制。
    Unsupported(&'static str),
}

thread_local! {
    static PROPAGATIONS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// 本线程累计的**成功**期望类型传播次数（测试/计数用；判定不读它）。
pub fn propagations() -> u64 { PROPAGATIONS.with(|c| c.get()) }

/// 本会话元变量的私有 id 段（不与调用方的 id 相撞）。
pub const SESSION_META_BASE: u32 = 1 << 30;

impl<'p> ExportFile<'p> {
    /// **F2b1**：elaborate `head explicit_args..`（Lean `elabAppArgs`）。新会话 ⇒ 元变量不跨调用泄漏；
    /// 内部任何 panic ⇒ `Unsupported("panic")`（本线程静音 + `catch_unwind`，同 `is_def_eq`）。
    pub fn elab_app(&self, limit: EnvLimit<'p>, req: &AppRequest<'_, 'p>) -> AppOutcome {
        let _q = crate::quiet::quiet();
        let run = std::panic::AssertUnwindSafe(|| self.with_tc(limit, |tc| tc.elab_app_session(req)));
        std::panic::catch_unwind(run).unwrap_or(AppOutcome::Unsupported("panic"))
    }
}

fn name_string(ctx: &TcCtx<'_, '_>, n: NamePtr<'_>) -> String {
    match ctx.read_name(n) {
        Name::Anon => String::new(),
        Name::Str(pfx, sfx, _) => {
            let mut out = name_string(ctx, pfx);
            if !out.is_empty() {
                out.push('.');
            }
            out.push_str(ctx.read_string(sfx).as_ref());
            out
        }
        Name::Num(pfx, k, _) => {
            let mut out = name_string(ctx, pfx);
            if !out.is_empty() {
                out.push('.');
            }
            out.push_str(&k.to_string());
            out
        }
    }
}

impl<'x, 't, 'p: 't> TypeChecker<'x, 't, 'p> {
    /// 深度 `depth` 下的、**不 panic** 的定义等价（[`Self::is_def_eq`] 的开放版：同一套 checkpoint/回滚）。
    pub(crate) fn is_def_eq_at(&mut self, depth: u32, x: V<'t>, y: V<'t>) -> bool {
        let cp = self.ctx.mvars.checkpoint();
        let probe = self.tc_cache.probe_depth;
        let names = self.binder_names.len();
        let r = {
            let _q = crate::quiet::quiet();
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.def_eq_at(depth, x, y)))
        };
        let (ok, panicked) = match r {
            Ok(b) => (b, false),
            Err(_) => (false, true),
        };
        if !ok {
            self.tc_cache.probe_depth = probe;
            self.binder_names.truncate(names);
            if self.ctx.mvars.restore(cp) || panicked {
                self.tc_cache.clear();
            }
        }
        ok
    }

    fn app_implicit(&self, i: usize, prefix: usize, style: BinderStyle) -> bool {
        i < prefix || style != BinderStyle::Default
    }

    fn elab_app_session(&mut self, req: &AppRequest<'_, 'p>) -> AppOutcome {
        let binder_tys: Vec<ExprPtr<'t>> = req.binder_tys.to_vec();
        let args: Vec<ExprPtr<'t>> = req.explicit_args.to_vec();
        let head: ExprPtr<'t> = req.head;
        let expected: Option<ExprPtr<'t>> = req.expected;
        // **P1 限制**：输入里不许有元变量（本会话的 id 不许与调用方的相撞）。
        let inputs = binder_tys.iter().chain(std::iter::once(&head)).chain(args.iter()).chain(expected.iter());
        for e in inputs {
            if !self.ctx.unassigned_mvars(*e).is_empty() {
                return AppOutcome::Unsupported("input contains a meta");
            }
        }
        // **P1 限制**：层元变量在 defeq 里是刚性的 ⇒ 宇宙多态的头不支持。
        if let Expr::Const { name, levels, .. } = self.ctx.read_expr(head) {
            let poly = self.env.get_declar(&name).is_some_and(|d| !d.info().uparams.is_empty());
            if poly || !levels.is_empty() {
                return AppOutcome::Unsupported("universe-polymorphic head");
            }
        }
        let Ok(d16) = u16::try_from(binder_tys.len()) else { return AppOutcome::Unsupported("scope too deep") };
        // 开 scope（同 `infer_under_binders`）。
        let mut d = 0u32;
        let mut env: E<'t> = self.empty_env();
        let mut ctx = self.empty_ctx();
        for ty in &binder_tys {
            let dom = self.eval(d, env, *ty);
            let fresh = self.mk_bvar_hc(d, dom);
            env = crate::value::env_extend(self.arena, env, fresh);
            ctx = crate::value::ctx_extend(self.arena, ctx, dom);
            d += 1;
        }
        let exp_v = expected.map(|e| self.eval(d, env, e));
        let mut fty = self.infer_value(InferFlag::InferOnly, d, env, ctx, head);
        let mut term = head;
        let mut metas: Vec<(u32, NamePtr<'t>)> = Vec::new();
        let (mut i, mut ai) = (0usize, 0usize);
        let mut tried = false;
        let mut propagated = false;
        let mut postponed: Vec<(usize, V<'t>, V<'t>)> = Vec::new();
        let anon = self.ctx.anonymous();
        loop {
            let pi = match self.weak_pi(d, fty) {
                Some(p) => Some(p),
                None => self.force_pi(d, fty),
            };
            let Some(pi) = pi else {
                if ai < args.len() {
                    return AppOutcome::Unsupported("surplus");
                }
                break;
            };
            let Value::Pi { binder_name, binder_style, domain, body, .. } = pi else { unreachable!() };
            let (bn, bs, dom, body) = (*binder_name, *binder_style, *domain, *body);
            if self.app_implicit(i, req.implicit_prefix, bs) {
                // Lean `processStrictImplicitArg`：没有剩余实参 ⇒ 不实例化，直接 finalize。
                if i >= req.implicit_prefix && bs == BinderStyle::StrictImplicit && ai == args.len() {
                    break;
                }
                // 上下文元变量：`?M : Π xs, T` 闭、深度 0；隐式位放 `?M x1..xd`。
                let mut mty = self.quote(d, dom);
                for t in binder_tys.iter().rev() {
                    mty = self.ctx.mk_pi(anon, BinderStyle::Default, *t, mty);
                }
                let id = SESSION_META_BASE + u32::try_from(metas.len()).expect("meta count");
                self.ctx.mvars.register_at(id, mty, 0);
                self.ctx.mvars.register_kind(id, MetaKind::Natural);
                let mut m = self.ctx.mk_meta(id, mty);
                for k in (0..d16).rev() {
                    let v = self.ctx.mk_var(k);
                    m = self.ctx.mk_app(m, v);
                }
                let mv = self.eval(d, env, m);
                term = self.ctx.mk_app(term, m);
                metas.push((id, bn));
                fty = self.apply_closure(d, &body, mv, Some(dom));
                i += 1;
                continue;
            }
            if ai == args.len() {
                break;
            }
            if !tried {
                tried = true;
                if let Some(ev) = exp_v {
                    propagated = self.app_propagate(d, env, req.implicit_prefix, i, args.len() - ai, fty, ev);
                }
            }
            let arg = args[ai];
            let at = self.infer_value(InferFlag::InferOnly, d, env, ctx, arg);
            if !self.is_def_eq_at(d, dom, at) {
                // Lean 会把卡在未赋值元变量上的约束推迟（`postponed`）；这里推迟到 finalize 重试一次。
                if !self.app_stuck_on_meta(d, dom) {
                    return AppOutcome::ArgMismatch { arg: ai };
                }
                postponed.push((ai, dom, at));
            }
            let av = self.eval(d, env, arg);
            term = self.ctx.mk_app(term, arg);
            fty = self.apply_closure(d, &body, av, Some(dom));
            ai += 1;
            i += 1;
        }
        if let Some(ev) = exp_v {
            if !propagated && !self.is_def_eq_at(d, ev, fty) {
                return AppOutcome::ExpectedMismatch;
            }
        }
        let failed: Vec<usize> =
            postponed.into_iter().filter(|(_, dom, at)| !self.is_def_eq_at(d, dom, at)).map(|p| p.0).collect();
        let unsolved: Vec<String> = metas
            .iter()
            .filter(|(id, _)| self.ctx.mvars.value(*id).is_none())
            .map(|(_, n)| name_string(self.ctx, *n))
            .collect();
        if !unsolved.is_empty() {
            return AppOutcome::Unsolved { binders: unsolved };
        }
        if let Some(arg) = failed.first() {
            return AppOutcome::ArgMismatch { arg: *arg };
        }
        let mut memo = FxHashMap::default();
        let out = self.subst_session_metas(term, &mut memo, 0);
        if !self.ctx.unassigned_mvars(out).is_empty() {
            return AppOutcome::Unsupported("residual metas");
        }
        AppOutcome::Solved(self.ctx.export_owned(out))
    }

    /// 实参的期望域卡在**未赋值**的元变量上（`?p w`）⇒ 失败可能只是信息不够（推迟，不判错）。
    fn app_stuck_on_meta(&mut self, d: u32, dom: V<'t>) -> bool {
        match self.force_all(d, dom) {
            Value::Meta { id, .. } => self.ctx.mvars.value(*id).is_none(),
            _ => false,
        }
    }

    /// Lean `propagateExpectedType` + `getResultingTypeCore?`：`fty` 的结果类型（跳过隐式、吃掉
    /// `args_left` 个显式）不依赖被跳过的 binder ⇒ `isDefEq(期望, 结果)`（失败回滚、不致命）。
    fn app_propagate(
        &mut self,
        d: u32,
        env: E<'t>,
        prefix: usize,
        i0: usize,
        args_left: usize,
        fty: V<'t>,
        exp_v: V<'t>,
    ) -> bool {
        // Lean 594：期望是 `Prop` ⇒ 不传播。
        if let Value::Sort { level, .. } = self.force_all(d, exp_v) {
            if self.ctx.is_zero(*level) {
                return false;
            }
        }
        let mut e = self.quote(d, fty);
        let (mut k, mut left, mut i) = (0u16, args_left, i0);
        while let Expr::Pi { binder_style, body, .. } = self.ctx.read_expr(e) {
            let imp = self.app_implicit(i, prefix, binder_style);
            if imp && i >= prefix && binder_style == BinderStyle::StrictImplicit && left == 0 {
                break;
            }
            if !imp {
                if left == 0 {
                    break;
                }
                left -= 1;
            }
            e = body;
            k += 1;
            i += 1;
        }
        // 实参比 binder 多（Lean 492）或结果依赖未吃的 binder（Lean 474/497）⇒ 推迟（这里 = 放弃）。
        if left > 0 || self.ctx.has_loose_bvar_below(e, k) {
            return false;
        }
        let r = self.ctx.lower(e, 0, k);
        let rv = self.eval(d, env, r);
        let ok = self.is_def_eq_at(d, exp_v, rv);
        if ok {
            PROPAGATIONS.with(|c| c.set(c.get() + 1));
        }
        ok
    }

    /// 把本会话的元变量换成赋值：`?M a1..ak`（赋值是闭的 `λx1..xk. t`）⇒ `t[xs := as]`（`inst_open` 会 lift，
    /// 开放实参也对）。赋值是闭项 ⇒ 结果与所在 binder 深度无关 ⇒ 按指针记忆。
    fn subst_session_metas(
        &mut self,
        e: ExprPtr<'t>,
        memo: &mut FxHashMap<ExprPtr<'t>, ExprPtr<'t>>,
        fuel: u32,
    ) -> ExprPtr<'t> {
        if let Some(r) = memo.get(&e) {
            return *r;
        }
        let r = match self.ctx.read_expr(e) {
            Expr::Meta { id, .. } => match self.ctx.mvars.value(id) {
                Some(v) if fuel < 64 => self.subst_session_metas(v, memo, fuel + 1),
                _ => e,
            },
            Expr::App { .. } => {
                let mut rev_args = Vec::new();
                let mut h = e;
                while let Expr::App { fun, arg, .. } = self.ctx.read_expr(h) {
                    rev_args.push(arg);
                    h = fun;
                }
                rev_args.reverse();
                let args: Vec<ExprPtr<'t>> =
                    rev_args.into_iter().map(|a| self.subst_session_metas(a, memo, fuel)).collect();
                let assigned = match self.ctx.read_expr(h) {
                    Expr::Meta { id, .. } if fuel < 64 => self.ctx.mvars.value(id),
                    _ => None,
                };
                let (mut f, rest) = match assigned {
                    Some(v) => {
                        let v = self.subst_session_metas(v, memo, fuel + 1);
                        let (mut b, mut n) = (v, 0usize);
                        while n < args.len() {
                            let Expr::Lambda { body, .. } = self.ctx.read_expr(b) else { break };
                            b = body;
                            n += 1;
                        }
                        let f = if n > 0 { self.ctx.inst_open(b, &args[..n]) } else { v };
                        (f, &args[n..])
                    }
                    None => (self.subst_session_metas(h, memo, fuel), &args[..]),
                };
                for a in rest {
                    f = self.ctx.mk_app(f, *a);
                }
                f
            }
            Expr::Lambda { binder_name, binder_style, binder_type, body, .. } => {
                let t = self.subst_session_metas(binder_type, memo, fuel);
                let b = self.subst_session_metas(body, memo, fuel);
                self.ctx.mk_lambda(binder_name, binder_style, t, b)
            }
            Expr::Pi { binder_name, binder_style, binder_type, body, .. } => {
                let t = self.subst_session_metas(binder_type, memo, fuel);
                let b = self.subst_session_metas(body, memo, fuel);
                self.ctx.mk_pi(binder_name, binder_style, t, b)
            }
            Expr::Let { data, .. } => {
                let crate::expr::LetData { binder_name, binder_type, val, body, nondep } = *data;
                let t = self.subst_session_metas(binder_type, memo, fuel);
                let v = self.subst_session_metas(val, memo, fuel);
                let b = self.subst_session_metas(body, memo, fuel);
                self.ctx.mk_let(binder_name, t, v, b, nondep)
            }
            Expr::Proj { ty_name, idx, structure, .. } => {
                let s = self.subst_session_metas(structure, memo, fuel);
                self.ctx.mk_proj(ty_name, idx, s)
            }
            Expr::Var { .. } | Expr::Sort { .. } | Expr::Const { .. } | Expr::StringLit { .. } | Expr::NatLit { .. } =>
                e,
        };
        memo.insert(e, r);
        r
    }
}

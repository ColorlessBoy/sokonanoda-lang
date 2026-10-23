//! **元变量上下文（S1）**：类型/赋值/种类 + occurs 检查 + 合一臂。
//!
//! 本模块只放"元变量作为值"需要的东西，不动判定层的不变式：
//! * 含元变量的**声明**仍由 `EnvBuilder::add_declar` 在入口硬拒（K1）；
//! * 这里只服务"elaborate 期的中间检查"（judge 的合成声明）。
use crate::expr::MetaKind;
use crate::tc::TypeChecker;
use crate::env::EnvLimit;
use crate::util::{ExportFile, ExprPtr, FxHashMap, TcCtx};
use crate::value::{Closure, ElimView, Env, Spine, Value, E, S, V};

/// 元变量上下文：`types[id]` = 声明类型；`values[id]` = 已赋的值；`kinds[id]` = 种类。
#[derive(Default)]
pub struct MetaContext<'t> {
    types: FxHashMap<u32, ExprPtr<'t>>,
    values: FxHashMap<u32, ExprPtr<'t>>,
    kinds: FxHashMap<u32, MetaKind>,
    /// 正在做"赋值前类型检查"的 id（防递归）。
    checking: std::collections::HashSet<u32>,
}

impl<'t> MetaContext<'t> {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn register(&mut self, id: u32, ty: ExprPtr<'t>) {
        self.types.insert(id, ty);
    }
    pub fn register_kind(&mut self, id: u32, kind: MetaKind) {
        self.kinds.insert(id, kind);
    }
    pub fn kind(&self, id: u32) -> Option<MetaKind> {
        self.kinds.get(&id).copied()
    }
    pub fn ty_of(&self, id: u32) -> Option<ExprPtr<'t>> {
        self.types.get(&id).copied()
    }
    pub fn value(&self, id: u32) -> Option<ExprPtr<'t>> {
        self.values.get(&id).copied()
    }
    pub fn assign(&mut self, id: u32, v: ExprPtr<'t>) {
        self.values.insert(id, v);
    }
    fn begin_check(&mut self, id: u32) -> bool {
        self.checking.insert(id)
    }
    fn end_check(&mut self, id: u32) {
        self.checking.remove(&id);
    }
}

impl<'t, 'p: 't> TcCtx<'t, 'p> {
    /// 造一个 `Expr::Meta`（与 `EnvBuilder::mk_meta` 同哈希）。
    pub fn mk_meta(&mut self, id: u32, ty: ExprPtr<'t>) -> ExprPtr<'t> {
        let hash = crate::hash64!(crate::expr::META_HASH, id as u64, 0u64);
        let fv_mask = crate::expr::child_mask(ty);
        self.alloc_expr(crate::expr::Expr::Meta {
            id,
            kind: MetaKind::Natural,
            ty,
            fv_mask,
            hash,
        })
    }
}

/// 元变量值（`Value::Meta` 的唯一构造点）。
pub fn mk_meta<'a>(arena: &'a bumpalo::Bump, id: u32, ty: ExprPtr<'a>) -> V<'a> {
    arena.alloc(Value::Meta {
        id,
        ty,
        key: std::cell::Cell::new(0),
    })
}

/// 谓词版 occurs：`v` 里有没有满足 `pred` 的元变量 id。
fn any_meta(v: V<'_>, pred: &dyn Fn(u32) -> bool) -> bool {
    match v {
        Value::Meta { id, .. } => pred(*id),
        Value::Sort { .. } | Value::NatLit { .. } | Value::StrLit { .. } => false,
        Value::Rigid { head, spine, .. } => {
            matches!(head, crate::value::RigidHead::BVar(_, ty) if any_meta(*ty, pred))
                || spine_any(*spine, pred)
        }
        Value::Unfold { spine, .. } => spine_any(*spine, pred),
        Value::Lam {
            binder_type, body, ..
        } => expr_any(*binder_type, pred) || closure_any(*body, pred),
        Value::Pi { domain, body, .. } => any_meta(*domain, pred) || closure_any(*body, pred),
        Value::Thunk { env, expr, .. } => env_any(*env, pred) || expr_any(*expr, pred),
    }
}

fn spine_any(s: S<'_>, pred: &dyn Fn(u32) -> bool) -> bool {
    let mut cur = s;
    loop {
        match cur {
            Spine::Empty => return false,
            Spine::Snoc { prev, elim, .. } => {
                if let ElimView::App(v) = elim.view() {
                    if any_meta(v, pred) {
                        return true;
                    }
                }
                cur = prev;
            }
        }
    }
}

fn env_any(e: E<'_>, pred: &dyn Fn(u32) -> bool) -> bool {
    match e {
        Env::Nil { .. } => false,
        Env::Cons { v, parent, .. } => any_meta(v, pred) || env_any(parent, pred),
        Env::Framed { slots, .. } => slots.iter().any(|v| any_meta(v, pred)),
    }
}

fn closure_any(c: Closure<'_>, pred: &dyn Fn(u32) -> bool) -> bool {
    env_any(c.env, pred) || expr_any(c.body, pred)
}

/// 表达式树里的元变量（深扫；与 `EnvBuilder::contains_meta` 同覆盖面）。
fn expr_any(e: ExprPtr<'_>, pred: &dyn Fn(u32) -> bool) -> bool {
    use crate::expr::Expr;
    match e.as_ref() {
        Expr::Meta { id, .. } => pred(*id),
        Expr::App { fun, arg, .. } => expr_any(*fun, pred) || expr_any(*arg, pred),
        Expr::Pi {
            binder_type, body, ..
        }
        | Expr::Lambda {
            binder_type, body, ..
        } => expr_any(*binder_type, pred) || expr_any(*body, pred),
        Expr::Let { data, .. } => {
            expr_any(data.binder_type, pred)
                || expr_any(data.val, pred)
                || expr_any(data.body, pred)
        }
        Expr::Proj { structure, .. } => expr_any(*structure, pred),
        Expr::Var { .. }
        | Expr::Sort { .. }
        | Expr::Const { .. }
        | Expr::StringLit { .. }
        | Expr::NatLit { .. } => false,
    }
}

/// occurs：`v` 里有没有 `?id`。
pub fn mentions_meta(id: u32, v: V<'_>) -> bool {
    any_meta(v, &|x| x == id)
}

/// `v` 里有没有**未赋值**的元变量（缓存纪律用）。
pub fn has_unassigned_meta(v: V<'_>, ctx: &MetaContext<'_>) -> bool {
    any_meta(v, &|id| ctx.value(id).is_none())
}

impl<'x, 't, 'p> TypeChecker<'x, 't, 'p> {
    /// 合一里的元变量臂：已赋值 ⇒ 展开再比；未赋值 ⇒ occurs/kind/类型检查后赋值。
    pub(crate) fn unify_meta(&mut self, depth: u32, id: u32, ty: ExprPtr<'t>, v: V<'t>) -> bool {
        // **kind**：`SyntheticOpaque` 永不被赋值（Lean 同款）。
        if self.ctx.mvars.kind(id) == Some(MetaKind::SyntheticOpaque) {
            return false;
        }
        if let Some(term) = self.ctx.mvars.value(id) {
            if let Value::Meta { id: other, .. } = &*v {
                if *other == id {
                    return true;
                }
            }
            let v0 = self.eval(depth, self.empty_env(), term);
            return self.unify::<true>(depth, v0, v);
        }
        // **occurs**：`?m =?= f ?m` 拒绝（否则造环）。
        if mentions_meta(id, v) {
            return false;
        }
        // **类型检查**（best effort）：`T(?m) ≟ type(v)`；解不出就放弃这次赋值。
        if !self.ctx.mvars.begin_check(id) {
            return false;
        }
        let ok = {
            let vt = self.value_type(depth, v);
            let tt = self.eval(depth, self.empty_env(), ty);
            self.unify::<true>(depth, vt, tt)
        };
        self.ctx.mvars.end_check(id);
        if !ok {
            return false;
        }
        let quoted = self.quote(depth, v);
        self.ctx.mvars.assign(id, quoted);
        true
    }

    /// 缓存纪律：含**未赋值**元变量的值不许进缓存（赋值后 key 就过期）。
    pub(crate) fn value_has_unassigned_meta(&self, v: V<'t>) -> bool {
        has_unassigned_meta(v, &self.ctx.mvars)
    }
}

impl<'x, 't, 'p> TypeChecker<'x, 't, 'p> {
    /// `unify` 的元变量拦截（`conv.rs` 只留一行调用）。
    pub(crate) fn meta_unify_branch(&mut self, depth: u32, x: V<'t>, y: V<'t>) -> Option<bool> {
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

/// `eval` 的 `Expr::Meta` 臂（已赋值 ⇒ 展开；未赋值 ⇒ `Value::Meta`）。
pub(crate) fn eval_meta<'t>(
    tc: &mut TypeChecker<'_, 't, '_>,
    depth: u32,
    env: crate::value::E<'t>,
    id: u32,
    ty: ExprPtr<'t>,
) -> V<'t> {
    match tc.ctx.mvars.value(id) {
        Some(term) => tc.eval(depth, env, term),
        None => mk_meta(tc.arena, id, ty),
    }
}

/// `value_type` 的 `Value::Meta` 臂：类型 = 上下文登记的类型。
pub(crate) fn value_type_meta<'t>(
    tc: &mut TypeChecker<'_, 't, '_>,
    depth: u32,
    ty: ExprPtr<'t>,
) -> V<'t> {
    let env = tc.empty_env();
    tc.eval(depth, env, ty)
}

impl<'x, 't, 'p> TypeChecker<'x, 't, 'p> {
    pub(crate) fn def_eq_core(&mut self, x: ExprPtr<'t>, y: ExprPtr<'t>) -> bool {
        let depth = 0u32;
        let env = self.empty_env();
        let vx = self.eval(depth, env, x);
        let vy = self.eval(depth, env, y);
        // **S1**：元变量优先于 proof-irrelevance（否则 `?m ≟ p` 判真却不赋值）。
        if let Some(r) = self.meta_def_eq_branch(depth, vx, vy) {
            return r;
        }
        if self.try_proof_irrel_at(depth, vx, vy) {
            return true;
        }
        self.conv_types_at(depth, vx, vy)
    }

    /// 在**探查之外**跑一次转换判定（[`Self::conv_types_at`] / [`Self::def_eq_at`] 的入口 ✓）。
    ///
    /// 探查上下文（`probe_depth`）**不许漏进**这两个入口 ✓：它们是**独立的一次判定**
    /// （推断中途问一句「这两个类型相等吗」✓），必须从**顶层**开始 —— 否则
    /// [`Self::spine_probe`] 会走 `probe_depth > 0` 那条**全量**分支（少抄近路 ⇒ 变慢 ✗），
    /// `eval.rs::unfold_value_demand` 也会被当成探查内（`force = false` ⇒ 推迟 Nat 归约 ✗）。
    pub(crate) fn outside_probe<R>(&mut self, f: impl FnOnce(&mut Self) -> R) -> R {
        let depth = self.tc_cache.probe_depth;
        self.tc_cache.probe_depth = 0;
        let r = f(self);
        self.tc_cache.probe_depth = depth;
        r
    }


}

impl<'p> ExportFile<'p> {
    /// **S2**：带**种子**的检查 —— 前端把"这次检查可见的元变量（id/类型/种类）"传进来。
    ///
    /// 为什么需要它：`with_ctx` 每次新建 arena + `MetaContext` ⇒ 上一次检查的元变量
    /// 看不见；前端要把 handle 里的**文本**重新 elaborate 成 `ExprPtr` 再种进来。
    /// 返回类型不变（`A`），收集赋值由调用方在闭包里用 `tc.ctx.mvars.value(id)`
    /// + `tc.with_pp` 完成（渲染成文本后才能跨检查存活）。
    pub fn with_tc_seeded<F, A>(&self, limit: EnvLimit<'p>, seeds: &[(u32, ExprPtr<'p>, MetaKind)], f: F) -> A
    where
        F: FnOnce(&mut TypeChecker<'_, '_, 'p>) -> A,
    {
        self.with_tc(limit, |tc| {
            for (id, ty, kind) in seeds {
                tc.ctx.mvars.register(*id, *ty);
                tc.ctx.mvars.register_kind(*id, *kind);
            }
            f(tc)
        })
    }
}

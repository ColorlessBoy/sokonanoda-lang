//! **HO3a（2026-10-26）**：pattern 赋值的**引擎本体** —— `?m a₁…aₙ ≟ t` ⇒
//! `?m := fun a₁ … aₙ => t`（**strict** + **quasi** 两档）。
//!
//! **HO3b（2026-10-26）**：**FO 近似**（`SOKO_META_APPROX`，默认关；设计 §3.2.3）+
//! **生产 checker**（`judge_terms` 的完整内核检查，[`check`]）——近似档的**健全性**靠它。
//!
//! 设计 ⇒ `docs/design/metavar-pattern.md` §3.2/§3.3/§4（本模块是 §3.2 的实现）。
//!
//! **闸门**：`SOKO_META_PATTERN`（**HO4 起默认开**，`=0` 逃生；见 [`enabled`]；`SOKO_META_APPROX`
//! **蕴含**它）⇒ 关着时 [`MetaCtx::pattern_try`] 一次 `OnceLock` 读后立刻返回 `None`，
//! 判定路径与今天**逐字节相同**（M0 13 形状 + 7 文件 `--json` 的判据都钉着这条）。
//!
//! **安全性的根（返回值契约）**：本模块**只在真的写入了赋值时**返回 `Tri::Yes`
//! （⇒ `unify_body_inner` 当场 `return`）；其余一切返回 `Tri::Undef` ⇒
//! [`MetaCtx::pattern_try`] 折成 `None` ⇒ **落回今天那条路**。⇒ 本规则
//! **永不产生** `No`/由它造成的 `Undef`（不是"靠开关兜"）。
//!
//! **健全性**：抽象出来的 λ 必须过**调用方装的类型检查回调**（[`MetaCtx::check`]，
//! 设计 §3.3 的 `judge_terms` 完整检查）。**没有回调 ⇒ 弃权**（fail-closed）——
//! 生产实现是 [`check::judge`]（在 `elab` 的五处求解点装，见 [`note_ctx`]）；没装
//! （非声明通道 / 闸门关）⇒ 弃权 ✓。
//!
//! **不复制闸门**：occurs / type-occurs / `out_of_scope` / sort 全部只经
//! [`MetaCtx::assign`] ⇒ 只有一处写值、失败时状态天然不变（单写 ⇒ 无需回滚）。
//! ⚠ **FO 近似是唯一例外**：它先做若干 `unify`（**会**写别的元变量/待定队列）⇒
//! 自己带快照，非 `Yes` 一律 `restore`（设计 §3.2.5「失败 restore ⇒ 判定中性」）。
//!
//! ⚠ **偏离（白纸黑字）**：Lean 的 strict pattern 判据在
//! `Meta/ExprDefEq.lean:523`（条件 2 = 实参**不在** `lctx(m)` 里），其近似档
//! `:1180/:1199` 才允许实参在 `lctx` 里。本仓的元变量**在目标上下文里创建**
//! （`solve_prefix_meta` ⇒ `fresh` 与方程同一 scope）⇒ 生产可走的分支是
//! **quasi**（设计 §6 的 D11：HO4 必须把近似打开）；strict 是**参照语义 + 单测臂**。
//!
//! ⚠ **ctx 近似的边界（HO3b 刻意不做）**：`t` 里出现 `lctx(m) ∪ args` **之外**的局部名
//! ⇒ **弃权**（第 ④ 条闸）。Lean 的 `ctxApprox` 会 `elimMVarDeps` 去救，本仓**不做**——
//! 那会改元变量的声明类型（= 改判定面），而 `Undef` 的代价只是"落回今天那条路" ✓。

use std::cell::RefCell;
use std::rc::Rc;

use crate::ast::{Binder, BinderKind, Expr};
use crate::span::Span;

use super::{meta_ids_in, meta_name, meta_of, DeclMVarStore, MetaCtx, MetaId, MetaKind, Tri};

pub(crate) mod approx;
pub(crate) mod check;
pub(crate) use approx::free_idents;

/// 局部 binder 的**源类型**（**不是**核类型）：抽象值必须是源 `Expr`
/// （`Expr::Lambda` 的 binder 注解是 `Option<Box<Expr>>`），核类型没法直接当注解。
/// `ty == None`（无注解的 binder）⇒ **不许当 pattern 实参**（保守 = 安全方向）。
pub(crate) struct LocalBinder {
    pub name: String,
    pub ty: Option<Expr>,
    /// `let` 绑名。**不是 fvar**（Lean pattern 条件 1 明确排除 let 变量；
    /// 把 let 名当 fvar 抽象 = 语义错）。
    pub is_let: bool,
}

/// 一份局部上下文的**共享句柄**（`Rc` ⇒ O(1) 克隆；`MVar.lctx` / `MetaCtx.lctx` 都是它）。
pub(crate) type Lctx = Rc<Vec<LocalBinder>>;

/// 抽象 λ 的**完整类型检查**回调：`(fun args => t, m.ty) ⇒ 通过？`（设计 §3.3）。
/// `None` ⇒ 规则弃权（fail-closed）。
///
/// **为什么是 `Rc<dyn Fn>` 而不是 `&dyn Fn`**（HO3b）：检查器要在**求解点**装（那里有
/// `prefix_src`/`options`/`scope`），却在 [`MetaCtx`]（`implicit.rs` 里造）里用 ——
/// 两者之间唯一的载体是声明级 store（没有生命周期参数）⇒ 句柄必须是**共享所有权**
/// 的（`Rc` 克隆 O(1)，`StoreGuard::meta_ctx` 才拿得到）。
pub(crate) type CheckFn = Rc<dyn Fn(&Expr, &Expr) -> bool>;

/// 一处求解点的**上下文快照**（[`note_ctx`] 写、`StoreGuard::meta_ctx` 读）。
#[derive(Default)]
pub(crate) struct Scope {
    /// 局部 binder 表（`lctx(m)` 与抽象来源）。
    pub(crate) lctx: Lctx,
    /// 生产 checker（`None` ⇒ 规则弃权；闸门关 ⇒ 恒 `None` ⇒ 零分配）。
    pub(crate) check: Option<CheckFn>,
    /// checker 的**判定基座**（前缀 + options；跨求解点复用 ⇒ **不重复克隆前缀**）。
    pub(crate) base: Option<Rc<check::Base>>,
}

/// 闸门：`SOKO_META_PATTERN` —— **HO4 起默认开**（`=0` 逃生）。只读一次 ✓。
/// **`SOKO_META_APPROX` 蕴含它**（近似档是 pattern 规则的一档，不是另一条规则）。
pub(crate) fn enabled() -> bool {
    use std::sync::OnceLock;
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| env_on_default("SOKO_META_PATTERN") || approx_enabled())
}

/// 闸门：`SOKO_META_APPROX` —— **HO4 起默认开**（`=0` 逃生）。
pub(crate) fn approx_enabled() -> bool {
    use std::sync::OnceLock;
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| env_on_default("SOKO_META_APPROX"))
}

/// HO4 默认开：设了且不是 `0` ⇒ 开；**缺省也开**（`=0` 逃生）。
fn env_on_default(name: &str) -> bool {
    std::env::var(name).map_or(true, |v| v != "0")
}

/// **HO3a/HO3b**：把当前 `ElabScope` 的局部上下文 + **生产 checker** 挂到**声明级**
/// store 上（`elab` 的五处求解点各**一行**；`elab.rs` 只剩 10 行预算）。
///
/// `implicit.rs` 经 [`super::StoreGuard::meta_ctx`] **零改动**读到它（那里只剩 5 行预算）。
/// **闸门关 ⇒ 一次都不分配**（`local_binders()` 都不调）⇒ 默认路径零成本 ✓。
pub(crate) fn note_ctx(
    store: Option<&DeclMVarStore>,
    ctx: &crate::compile::elab::ElabCtx<'_, '_>,
    scope: &crate::compile::elab::ElabScope<'_>,
) {
    if !enabled() {
        return;
    }
    if let Some(store) = store {
        let lctx: Lctx = Rc::new(scope.local_binders());
        let base = check::base_for(store, ctx);
        let bound = lctx.clone();
        let judged = base.clone();
        let check: CheckFn =
            Rc::new(move |lambda: &Expr, ty: &Expr| check::judge(&judged, &bound, lambda, ty));
        store.set_scope(Scope {
            lctx,
            check: Some(check),
            base: Some(base),
        });
    }
}

/// `StoreGuard::meta_ctx` 的挂点：把 store 上的快照**拷进** `MetaCtx`（空 ⇒ 不碰 ⇒
/// 与今天逐字节相同）。`scope == None`（scratch / detached）⇒ `lctx` 空 ⇒ 规则弃权 ✓。
pub(crate) fn attach_scope(ctx: &mut MetaCtx<'_>, scope: Option<&RefCell<Scope>>) {
    let Some(scope) = scope else { return };
    let scope = scope.borrow();
    ctx.lctx = scope.lctx.clone();
    ctx.check = scope.check.clone();
}

impl DeclMVarStore {
    /// 换上一份新的上下文快照（只由 [`note_ctx`] 调；句柄共享 ⇒ 下一个 `park` 读到）。
    pub(crate) fn set_scope(&self, scope: Scope) {
        *self.scope.borrow_mut() = scope;
    }
}

/// 一处识别出来的 pattern 左部：`?m a₁…aₙ`（`n ≥ 1`；实参是**互异的局部名**）。
struct Pattern {
    meta: MetaId,
    args: Vec<String>,
}

/// `?m a₁…aₙ` 的识别（`spine::spine_of`；**只看一层脊**）。
///
/// 三条闸：头是**未赋值的**元变量（`zonk` 之后还叫这个名字）、`n ≥ 1`、
/// 每个实参都是 `lctx` 里的**非 let** 名字且**两两不同**（重复实参 ⇒ 不识别 ⇒ 弃权）。
fn as_pattern(e: &Expr, lctx: &[LocalBinder]) -> Option<Pattern> {
    let (head, args) = crate::spine::spine_of(e);
    let meta = meta_of(head)?;
    if args.is_empty() {
        return None;
    }
    let mut names: Vec<String> = Vec::with_capacity(args.len());
    for arg in args {
        let Expr::Ident { name, .. } = arg else {
            return None;
        };
        if !lctx.iter().any(|b| !b.is_let && &b.name == name) {
            return None;
        }
        if names.contains(name) {
            return None;
        }
        names.push(name.clone());
    }
    Some(Pattern { meta, args: names })
}

/// `outer` 是不是 `longer` 的**前缀**（只比名字 + let 位 —— 同一个 binder 的判据）。
fn is_prefix(outer: &[LocalBinder], longer: &[LocalBinder]) -> bool {
    outer.len() <= longer.len()
        && outer
            .iter()
            .zip(longer)
            .all(|(a, b)| a.name == b.name && a.is_let == b.is_let)
}

/// 对**全部**实参抽象：`fun a₁ … aₙ => t`（binder 注解取源类型、显式风格、默认 span）。
///
/// ⚠ **永远对全部实参抽象**（设计 §3.2.4）：绝不产 `fun _ => t` 这种丢参的**常量解**
/// ——那是"最一般解陷阱"的坏方向（`?p A ≟ ¬(x∈A)` 里 `fun _ => ¬(x∈A)` 也满足方程）。
fn abstract_over(args: &[String], tys: &[Expr], t: &Expr) -> Expr {
    let mut out = t.clone();
    for (name, ty) in args.iter().zip(tys).rev() {
        out = Expr::Lambda {
            binders: vec![Binder {
                name: name.clone(),
                ty: Some(Box::new(ty.clone())),
                style: BinderKind::Explicit,
                span: Span::default(),
            }],
            body: Box::new(out),
            span: Span::default(),
        };
    }
    out
}

/// 收 `t` 里**自由出现**的名字、以及**所有 binder 名**（遮蔽闸用）。
///
/// `bound` 是当前被内层 binder 遮住的名字栈（`Lambda`/`Forall`/`Let`/`match` 模式）；
/// 被遮住的出现**不算**自由名。`binders` 收集每一个 binder 名（含 `match` 模式绑名）
/// —— 它们撞上某个实参时抽象会**捕获** ⇒ 调用方弃权。
///
/// ⚠ 保守：`By`（tactic 脚本）不扫（解析前的形态，elaborate 之后不该出现）；`match`
/// 模式的**构造子名**也会被 `collect_pattern_binds` 当成绑名收进来 ⇒ 只会**多弃权**，
/// 不会多赋值（安全方向 ✓）。
pub(super) fn free_names(
    e: &Expr,
    bound: &mut Vec<String>,
    used: &mut Vec<String>,
    binders: &mut Vec<String>,
) {
    match e {
        Expr::Ident { name, .. } => {
            if !bound.iter().any(|b| b == name) {
                used.push(name.clone());
            }
        }
        Expr::Sort { .. }
        | Expr::UniverseApp { .. }
        | Expr::Num { .. }
        | Expr::Hole { .. }
        | Expr::By { .. } => {}
        Expr::App { fun, arg, .. } => {
            free_names(fun, bound, used, binders);
            free_names(arg, bound, used, binders);
        }
        Expr::Lambda {
            binders: bs, body, ..
        }
        | Expr::Forall {
            binders: bs, body, ..
        } => {
            for b in bs {
                if let Some(ty) = b.ty.as_deref() {
                    free_names(ty, bound, used, binders);
                }
            }
            let mark = bound.len();
            for b in bs {
                if !b.name.is_empty() {
                    binders.push(b.name.clone());
                    bound.push(b.name.clone());
                }
            }
            free_names(body, bound, used, binders);
            bound.truncate(mark);
        }
        Expr::Arrow {
            domain, codomain, ..
        }
        | Expr::Plus {
            lhs: domain,
            rhs: codomain,
            ..
        } => {
            free_names(domain, bound, used, binders);
            free_names(codomain, bound, used, binders);
        }
        Expr::Let {
            binder, val, body, ..
        } => {
            if let Some(ty) = binder.ty.as_deref() {
                free_names(ty, bound, used, binders);
            }
            free_names(val, bound, used, binders);
            let mark = bound.len();
            if !binder.name.is_empty() {
                binders.push(binder.name.clone());
                bound.push(binder.name.clone());
            }
            free_names(body, bound, used, binders);
            bound.truncate(mark);
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            free_names(scrutinee, bound, used, binders);
            for arm in arms {
                if let Some(guard) = arm.guard.as_ref() {
                    free_names(guard, bound, used, binders);
                }
                let mark = bound.len();
                let mut binds = Vec::new();
                crate::spine::collect_pattern_binds(&arm.pattern, &mut binds);
                for name in binds {
                    binders.push(name.clone());
                    bound.push(name);
                }
                free_names(&arm.body, bound, used, binders);
                bound.truncate(mark);
            }
        }
        Expr::Notation { lhs, rhs, .. } => {
            if let Some(lhs) = lhs.as_deref() {
                free_names(lhs, bound, used, binders);
            }
            if let Some(rhs) = rhs.as_deref() {
                free_names(rhs, bound, used, binders);
            }
        }
        Expr::SetLiteral { elements, .. } | Expr::AnonCtor { elements, .. } => {
            for element in elements {
                free_names(element, bound, used, binders);
            }
        }
        Expr::NamedArg { value, .. } => free_names(value, bound, used, binders),
    }
}

impl<'a> MetaCtx<'a> {
    /// **挂点**（`unify_body_inner`，在两条裸 `?m ≟ t` 之后、结构分解之前）：
    /// 只在**真的写成赋值**时返回 `Some(Tri::Yes)`；其余一律 `None` ⇒ 调用方继续走
    /// 今天那条路（结构分解 / delta / postponed）⇒ **弃权时逐字节不变** ✓。
    ///
    /// 顺序 = 严格 pattern（strict/quasi，HO3a）⇒ **FO 近似**（HO3b，**只在
    /// `SOKO_META_APPROX` 开时**）。两档都弃权 ⇒ `None`（= 落回今天那条路）。
    pub(crate) fn pattern_try(&mut self, l: &Expr, r: &Expr) -> Option<Tri> {
        if !enabled() {
            return None;
        }
        if self.pattern_assign(l, r) == Tri::Yes {
            return Some(Tri::Yes);
        }
        if approx_enabled() && self.fo_assign(l, r) == Tri::Yes {
            return Some(Tri::Yes);
        }
        None
    }

    /// 三值版（单测直接调它，不经环境闸）：`Yes` = **已写入赋值**；其余一律 `Undef`
    /// （弃权、状态零改动）。**绝不返回 `No`**（G-88 口径：判不了只许弃权）。
    pub(crate) fn pattern_assign(&mut self, l: &Expr, r: &Expr) -> Tri {
        let l = self.zonk(l);
        let r = self.zonk(r);
        // `@` 全显式脊 ⇒ 弃权（Lean 的 `@` 已关隐式插入；unit12 `project_chain` 的误配由
        // 此挡住，且对普通 `apply` 形状无影响）。
        if check::has_explicit_spine(&l) || check::has_explicit_spine(&r) {
            return Tri::Undef;
        }
        // 空 lctx = `detached` / 重入 scratch / 非声明通道 ⇒ **fail-closed**：
        // 连"哪些名字是局部变量"都判不了 ⇒ 弃权（不是猜）。
        if self.lctx.is_empty() {
            return Tri::Undef;
        }
        let lctx = self.lctx.clone();
        // 先左后右（与 Lean 的 `?m args ≟ t` / `t ≟ ?m args` 同序）。
        let (pattern, t) = match as_pattern(&l, &lctx) {
            Some(p) => (p, r),
            None => match as_pattern(&r, &lctx) {
                Some(p) => (p, l),
                None => return Tri::Undef,
            },
        };
        self.pattern_assign_one(pattern, &t)
    }

    /// 已识别出 `?m a₁…aₙ ≟ t` 之后的判据 + 抽象 + 检查 + 赋值。
    fn pattern_assign_one(&mut self, p: Pattern, t: &Expr) -> Tri {
        // ① m 必须**未赋值**、且不是洞（`SyntheticOpaque` 永不被类型约束填）。
        let (assigned, kind, m_lctx, m_ty) = {
            let Some(idx) = p.meta.0.checked_sub(self.store.base) else {
                return Tri::Undef;
            };
            let Some(m) = self.store.mvars.get(idx as usize) else {
                return Tri::Undef;
            };
            (m.value.is_some(), m.kind, m.lctx.clone(), m.ty.clone())
        };
        if assigned || kind == MetaKind::SyntheticOpaque {
            return Tri::Undef;
        }
        // ② strict / quasi 分家：`args ∩ lctx(m) = ∅` 才是 strict（设计 §3.2 条件 2）；
        //    实参全在 `lctx(m)` 里 ⇒ 生产唯一可达的 quasi（D11）。**混着 ⇒ 两档都不算**。
        let in_m = |name: &str| m_lctx.iter().any(|b| !b.is_let && b.name.as_str() == name);
        let all_in = p.args.iter().all(|a| in_m(a));
        let none_in = p.args.iter().all(|a| !in_m(a));
        if !all_in && !none_in {
            return Tri::Undef;
        }
        // ③ 实参的 binder 注解：strict 的实参是**当前** lctx 里额外注入的局部名，
        //    quasi 的实参本来就在 m 的上下文里。无书写类型 ⇒ 不抽象（保守）。
        let source: &[LocalBinder] = if all_in { &m_lctx } else { &self.lctx };
        let mut tys: Vec<Expr> = Vec::with_capacity(p.args.len());
        for arg in &p.args {
            let Some(b) = source.iter().rev().find(|b| &b.name == arg) else {
                return Tri::Undef;
            };
            if b.is_let {
                return Tri::Undef;
            }
            let Some(ty) = b.ty.clone() else {
                return Tri::Undef;
            };
            tys.push(ty);
        }
        // ④ 自由名闸：`fvars(t) ⊆ lctx(m)`（quasi）/ `lctx(m) ∪ args`（strict）
        //    ⇒ 只拒**当前上下文里真是局部变量**的名字；不是局部的名字按全局常量放行。
        let mut used = Vec::new();
        let mut binders = Vec::new();
        free_names(t, &mut Vec::new(), &mut used, &mut binders);
        let allowed: Vec<&str> = if all_in {
            m_lctx.iter().map(|b| b.name.as_str()).collect()
        } else {
            m_lctx
                .iter()
                .map(|b| b.name.as_str())
                .chain(p.args.iter().map(String::as_str))
                .collect()
        };
        for name in &used {
            let is_local =
                self.lctx.iter().any(|b| &b.name == name) || m_lctx.iter().any(|b| &b.name == name);
            if is_local && !allowed.contains(&name.as_str()) {
                return Tri::Undef;
            }
        }
        // ⑤ 遮蔽闸：`t` 内部有 binder 撞上实参 ⇒ 抽象会**捕获** ⇒ 弃权（不做重命名）。
        if binders.iter().any(|b| p.args.contains(b)) {
            return Tri::Undef;
        }
        if all_in && check::quasi_arg_free(&p.args, &used) {
            return Tri::Undef;
        }
        // ⑥ 待定约束闸（quasi）：有把它往**别的方向**拉的约束 ⇒ 弃权（设计 §3.2(iii)）。
        if all_in {
            let name = meta_name(p.meta.0);
            let blocked =
                self.store.postponed.iter().any(|(l, r)| {
                    crate::spine::mentions(&name, l) || crate::spine::mentions(&name, r)
                });
            if blocked {
                return Tri::Undef;
            }
        }
        // ⑦ 解里其他元变量的上下文必须是 `lctx(m)` 的**前缀**（否则作用域不可知 ⇒ 弃权）。
        //    抽象 λ 的**binder 注解**也在解里 ⇒ 一并查。
        for other in meta_ids_in(t)
            .into_iter()
            .chain(tys.iter().flat_map(meta_ids_in))
        {
            let Some(other_lctx) = self.lctx_of(other) else {
                return Tri::Undef;
            };
            if !is_prefix(&other_lctx, &m_lctx) {
                return Tri::Undef;
            }
        }
        // ⑧ 预算：撞了 ⇒ **弃权**（不是判否）。只在真要进内核检查时付一次。
        if self.fuel == 0 {
            return Tri::Undef;
        }
        // ⑨ 完整类型检查（[`CheckFn`]；`None` ⇒ 弃权 —— 非声明通道 / 闸门关）。
        //    目标类型先 `zonk`：`m.ty` 里**已解**的载体要换成真项（否则渲染出
        //    `\0soko_m*` 名字 ⇒ 判卷文档 parse 不过 ⇒ 假弃权）。
        let lambda = abstract_over(&p.args, &tys, t);
        let Some(check) = self.check.clone() else {
            return Tri::Undef;
        };
        self.fuel -= 1;
        if !check(&lambda, &self.zonk(&m_ty)) {
            return Tri::Undef;
        }
        // ⑩ 单写：occurs / type-occurs / out_of_scope / sort 全在 `assign` 里（原样复用）。
        match self.assign(p.meta, lambda) {
            Ok(()) => {
                // 读数钩子（`SOKO_META_DEBUG`，默认零成本）：HO4 接线时要看得见规则**真的**触发。
                // ⚠ 不用 `render_expr`（绕过唯一记法接口，`audit-notation-paths.py` 会判红）⇒ Debug。
                if std::env::var_os("SOKO_META_DEBUG").is_some() {
                    eprintln!("[pattern] ?m{:?} := {:?}", p.meta, self.value(p.meta));
                }
                Tri::Yes
            }
            Err(_) => Tri::Undef,
        }
    }

    /// 某个元变量的 `lctx`（**不在本 store 里** ⇒ `None` ⇒ 调用方弃权）。
    fn lctx_of(&self, m: MetaId) -> Option<Lctx> {
        let idx = m.0.checked_sub(self.store.base)?;
        self.store.mvars.get(idx as usize).map(|v| v.lctx.clone())
    }
}

#[cfg(test)]
mod tests;

//! **K1a**：元变量带 spine（`?p a` 是卡住项而非 panic）+ `value_type` 的 Lam/Pi 臂 +
//! 赋值后 eval 缓存失效。由 P0 spike（`Exists.intro ?α ?p`）移植而来。
use crate::builder::EnvBuilder;
use crate::env::{Declar, DeclarInfo, EnvLimit};
use crate::expr::{BinderStyle, Expr, MetaKind};
use crate::tc::TypeChecker;
use crate::util::{Config, ExprPtr};
use crate::value::Value;
use stumpalo::Arena;

/// 全部住在 builder arena 里的项。
#[derive(Clone, Copy)]
pub(super) struct Fx<'a> {
    pub(super) a: ExprPtr<'a>,        // A : Type
    pub(super) p: ExprPtr<'a>,        // P : A → Prop
    pub(super) w: ExprPtr<'a>,        // a : A
    pub(super) lam: ExprPtr<'a>,      // fun x : A => P x
    pub(super) m_alpha: ExprPtr<'a>,  // ?α : Sort 1     (id 101)
    pub(super) m_p: ExprPtr<'a>,      // ?p : ?α → Prop  (id 102)
    pub(super) sort1: ExprPtr<'a>,
    pub(super) m_p_ty: ExprPtr<'a>,
    pub(super) res_ty: ExprPtr<'a>,   // Exists ?α ?p
    pub(super) expected: ExprPtr<'a>, // Exists A (fun x => P x)
    pub(super) q: ExprPtr<'a>,      // Q : A → Prop (K1b)
}

pub(super) fn axiom<'a>(b: &mut EnvBuilder<'a>, name: &str, ups: crate::util::LevelsPtr<'a>, ty: ExprPtr<'a>) {
    let name = b.name_from_str(name);
    b.add_declar(Declar::Axiom { info: DeclarInfo { name, uparams: ups, ty } }).unwrap();
}

pub(super) fn setup<'a>(b: &mut EnvBuilder<'a>) -> Fx<'a> {
    let zero = b.zero();
    let one = b.succ(zero);
    let prop = b.mk_sort(zero);
    let sort1 = b.mk_sort(one);
    let un = b.name_from_str("u");
    let u = b.level_param(un);
    let sort_u = b.mk_sort(u);
    let us = b.alloc_levels_slice(&[u]);
    let ones = b.alloc_levels_slice(&[one]);
    let none = b.alloc_levels_slice(&[]);
    let anon = b.anonymous();
    let (na, np, nx) = (b.name_from_str("α"), b.name_from_str("p"), b.name_from_str("x"));
    // Exists.{u} : {α : Sort u} → (α → Prop) → Prop
    let v0 = b.mk_var(0);
    let a_to_prop = b.mk_pi(anon, BinderStyle::Default, v0, prop);
    let ex_body = b.mk_pi(np, BinderStyle::Default, a_to_prop, prop);
    let ex_ty = b.mk_pi(na, BinderStyle::Implicit, sort_u, ex_body);
    axiom(b, "Exists", us, ex_ty);
    // A : Type, P : A → Prop, a : A
    axiom(b, "A", none, sort1);
    let an = b.name_from_str("A");
    let a = b.mk_const(an, none);
    let a_prop = b.mk_pi(anon, BinderStyle::Default, a, prop);
    axiom(b, "P", none, a_prop);
    let pn = b.name_from_str("P");
    let p = b.mk_const(pn, none);
    axiom(b, "a", none, a);
    axiom(b, "Q", none, a_prop);
    let qn = b.name_from_str("Q");
    let q = b.mk_const(qn, none);
    let wn = b.name_from_str("a");
    let w = b.mk_const(wn, none);
    let exn = b.name_from_str("Exists");
    let ex1 = b.mk_const(exn, ones);
    // metas: ?α : Sort 1 ; ?p : ?α → Prop
    let m_alpha = b.mk_meta(101, sort1);
    let m_p_ty = b.mk_pi(anon, BinderStyle::Default, m_alpha, prop);
    let m_p = b.mk_meta(102, m_p_ty);
    // Exists.{1} ?α ?p  /  Exists.{1} A (fun x : A => P x)
    let res_ty = {
        let t = b.mk_app(ex1, m_alpha);
        b.mk_app(t, m_p)
    };
    let v0c = b.mk_var(0);
    let px = b.mk_app(p, v0c);
    let lam = b.mk_lambda(nx, BinderStyle::Default, a, px);
    let expected = {
        let t = b.mk_app(ex1, a);
        b.mk_app(t, lam)
    };
    Fx { a, p, w, lam, m_alpha, m_p, sort1, m_p_ty, res_ty, expected, q }
}

/// 在带种子（?α、?p 都登记为 Natural）的 TypeChecker 里跑 `f`；大栈线程。
pub(super) fn with_tc(f: impl for<'t> FnOnce(&mut TypeChecker<'_, 't, '_>, Fx<'t>) + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            let arena = Arena::new();
            let mut b = EnvBuilder::new(arena.as_arena_ref(), Config::default());
            let fx = setup(&mut b);
            let limit = EnvLimit::ByIndex(b.declaration_count());
            let seeds = [(101u32, fx.sort1, MetaKind::Natural), (102, fx.m_p_ty, MetaKind::Natural)];
            b.with_env(|ef| ef.with_tc_seeded(limit, &seeds, |tc| f(tc, fx)));
        })
        .unwrap()
        .join()
        .unwrap();
}

pub(super) fn app<'t>(tc: &mut TypeChecker<'_, 't, '_>, f: ExprPtr<'t>, a: ExprPtr<'t>) -> ExprPtr<'t> { tc.ctx.mk_app(f, a) }

/// **K1a-T1**：`Exists ?α ?p =?= Exists A (fun x => P x)` ⇒ true，`?α := A`，`?p ≡ λx. P x`；
/// 接着 `P a =?= ?p a` ⇒ true。（修前：`value_type: Pi/Lam not supported` panic。）
#[test]
fn k1a_t1_exists_with_lambda_expected_type() {
    with_tc(|tc, fx| {
        assert!(tc.def_eq_core(fx.res_ty, fx.expected), "Exists ?α ?p =?= Exists A (fun x => P x)");
        let va = tc.ctx.mvars.value(101).expect("?α assigned");
        let vp = tc.ctx.mvars.value(102).expect("?p assigned");
        assert!(tc.def_eq_core(va, fx.a), "?α ≡ A");
        assert!(tc.def_eq_core(vp, fx.lam), "?p ≡ fun x => P x");
        let pa = app(tc, fx.p, fx.w);
        let mpa = app(tc, fx.m_p, fx.w);
        assert!(tc.def_eq_core(pa, mpa), "P a =?= ?p a");
    });
}

/// **K1a-T2**：没有期望类型，`?p a =?= P a`，`?p` 未赋值 ⇒ false（Miller 是下一片），不 panic。
#[test]
fn k1a_t2_applied_unassigned_meta_vs_rigid_is_false() {
    with_tc(|tc, fx| {
        let mpa = app(tc, fx.m_p, fx.w);
        let pa = app(tc, fx.p, fx.w);
        assert!(!tc.def_eq_core(mpa, pa));
        assert!(!tc.def_eq_core(pa, mpa));
        assert!(tc.ctx.mvars.value(102).is_none(), "?p stays unassigned");
        // 同 id、同 spine ⇒ true（spine 逐项比）。
        let mpa2 = app(tc, fx.m_p, fx.w);
        assert!(tc.def_eq_core(mpa, mpa2));
    });
}

/// **K1a-T3**：缓存陈旧。先 eval `?p`（卡住），再赋值，再 eval ⇒ 看得见赋值
/// （spike case 3 vs 4，但**不**手动 `tc_cache.clear()`）。
#[test]
fn k1a_t3_eval_after_assign_sees_assignment() {
    with_tc(|tc, fx| {
        let env = tc.empty_env();
        let before = tc.eval(0, env, fx.m_p);
        assert!(matches!(before, Value::Meta { .. }), "unassigned ?p evaluates to a meta");
        assert!(tc.def_eq_core(fx.m_p, fx.lam), "?p := fun x => P x (also assigns ?α := A)");
        let env = tc.empty_env();
        let after = tc.eval(0, env, fx.m_p);
        assert!(matches!(after, Value::Lam { .. }), "after assignment eval must see the lambda, not a stale meta");
        let env = tc.empty_env();
        let alpha = tc.eval(0, env, fx.m_alpha);
        assert!(!matches!(alpha, Value::Meta { .. }), "?α was assigned during ?p's type check");
    });
}

/// **K1a-T4**：卡住项。`?p a`（`?p` 未赋值）的 quote/pp 不 panic，且 quote 回来就是 `?p a`。
#[test]
fn k1a_t4_stuck_meta_app_quotes_and_prints() {
    with_tc(|tc, fx| {
        let mpa = app(tc, fx.m_p, fx.w);
        let env = tc.empty_env();
        let v = tc.eval(0, env, mpa);
        assert!(matches!(v, Value::Meta { id: 102, spine, .. } if spine.len() == 1));
        let q = tc.quote(0, v);
        match tc.ctx.read_expr(q) {
            Expr::App { fun, arg, .. } => {
                assert!(matches!(tc.ctx.read_expr(fun), Expr::Meta { id: 102, .. }));
                // quote 在 TC arena 里重建 `Const a`（指针不同于 builder 的 fx.w）⇒ 按名字比。
                let (Expr::Const { name: n1, .. }, Expr::Const { name: n2, .. }) =
                    (tc.ctx.read_expr(arg), tc.ctx.read_expr(fx.w))
                else {
                    panic!("quote(?p a): arg is not the constant `a`")
                };
                assert!(n1 == n2);
            }
            other => panic!("quote(?p a) = {other:?}"),
        }
        // round-trip：再 eval 一遍仍是同一个卡住项（def_eq 判同）。
        assert!(tc.def_eq_core(q, mpa));
        let s = tc.with_pp(|pp| pp.pp_expr(q));
        assert!(s.contains("meta#102"), "pp = {s}");
        // 投影在卡住项上也卡住（不 panic）。
        let a_name = match tc.ctx.read_expr(fx.a) {
            Expr::Const { name, .. } => name,
            _ => unreachable!(),
        };
        let pr = tc.do_proj(0, a_name, 0, v);
        assert!(matches!(pr, Value::Meta { id: 102, spine, .. } if spine.len() == 2));
    });
}

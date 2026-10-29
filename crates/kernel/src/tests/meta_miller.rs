//! **K1c**：scope 纪律 + Miller pattern + restore 按内容判。夹具 = `meta_spine::setup` 再加
//! `g : A → A → Prop`、`f : Prop → Prop` 与三个元变量 `?m : A`(103)、`?f : A → A → Prop`(104)、
//! `?h : A → A`(105)、`?k : A → Prop`(106)。binder 由两侧的 λ 引入（Lam/Lam 合一下钻一层）。
use super::meta_spine::{app, axiom, setup, Fx};
use crate::builder::EnvBuilder;
use crate::env::EnvLimit;
use crate::expr::{BinderStyle, MetaKind};
use crate::tc::TypeChecker;
use crate::util::{Config, ExprPtr};
use stumpalo::Arena;

#[derive(Clone, Copy)]
struct Mx<'a> {
    g: ExprPtr<'a>,   // g : A → A → Prop
    f: ExprPtr<'a>,   // f : Prop → Prop
    m: ExprPtr<'a>,   // ?m : A          (103)
    mf: ExprPtr<'a>,  // ?f : A → A → Prop (104)
    mh: ExprPtr<'a>,  // ?h : A → A      (105)
    mk: ExprPtr<'a>,  // ?k : A → Prop   (106)
}

fn with_tc2(f: impl for<'t> FnOnce(&mut TypeChecker<'_, 't, '_>, Fx<'t>, Mx<'t>) + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            let arena = Arena::new();
            let mut b = EnvBuilder::new(arena.as_arena_ref(), Config::default());
            let fx = setup(&mut b);
            let zero = b.zero();
            let prop = b.mk_sort(zero);
            let none = b.alloc_levels_slice(&[]);
            let anon = b.anonymous();
            let a_prop = b.mk_pi(anon, BinderStyle::Default, fx.a, prop);
            let a_a_prop = b.mk_pi(anon, BinderStyle::Default, fx.a, a_prop);
            let a_a = b.mk_pi(anon, BinderStyle::Default, fx.a, fx.a);
            let prop_prop = b.mk_pi(anon, BinderStyle::Default, prop, prop);
            axiom(&mut b, "g", none, a_a_prop);
            axiom(&mut b, "f", none, prop_prop);
            let gn = b.name_from_str("g");
            let g = b.mk_const(gn, none);
            let fname = b.name_from_str("f");
            let fc = b.mk_const(fname, none);
            let mx = Mx {
                g,
                f: fc,
                m: b.mk_meta(103, fx.a),
                mf: b.mk_meta(104, a_a_prop),
                mh: b.mk_meta(105, a_a),
                mk: b.mk_meta(106, a_prop),
            };
            let limit = EnvLimit::ByIndex(b.declaration_count());
            let seeds = [
                (101u32, fx.sort1, MetaKind::Natural),
                (102, fx.m_p_ty, MetaKind::Natural),
                (103, fx.a, MetaKind::Natural),
                (104, a_a_prop, MetaKind::Natural),
                (105, a_a, MetaKind::Natural),
                (106, a_prop, MetaKind::Natural),
            ];
            b.with_env(|ef| ef.with_tc_seeded(limit, &seeds, |tc| f(tc, fx, mx)));
        })
        .unwrap()
        .join()
        .unwrap();
}

/// `fun x : A => body`（body 用 `Var 0` 指 x）。
fn lam<'t>(tc: &mut TypeChecker<'_, 't, '_>, dom: ExprPtr<'t>, body: ExprPtr<'t>) -> ExprPtr<'t> {
    let n = tc.ctx.anonymous();
    tc.ctx.mk_lambda(n, BinderStyle::Default, dom, body)
}

fn none_assigned(tc: &TypeChecker<'_, '_, '_>) -> bool {
    (101..=106).all(|id| tc.ctx.mvars.value(id).is_none())
}

/// **K1c-T1**：`Exists A (fun x => ?p x) =?= Exists A (fun x => P x)` ⇒ true，`?p ≡ λx. P x`
/// （类型检查顺带 `?α := A`），`instantiate_mvars` 给出无元变量的 `Exists A (fun x => P x)`。
#[test]
fn k1c_t1_miller_under_exists() {
    with_tc2(|tc, fx, _| {
        let ex = tc.ctx.unfold_apps_fun(fx.expected);
        let v0 = tc.ctx.mk_var(0);
        let px = app(tc, fx.m_p, v0);
        let l = lam(tc, fx.a, px);
        let lhs = {
            let t = app(tc, ex, fx.a);
            app(tc, t, l)
        };
        assert!(tc.is_def_eq(lhs, fx.expected), "Exists A (fun x => ?p x) =?= Exists A (fun x => P x)");
        let vp = tc.ctx.mvars.value(102).expect("?p assigned by Miller");
        assert!(super::meta_defeq::same(vp, fx.lam), "?p := λx. P x");
        assert!(tc.ctx.mvars.value(101).is_some(), "?α := A from the type check");
        let inst = tc.ctx.instantiate_mvars(lhs);
        assert!(tc.ctx.unassigned_mvars(inst).is_empty());
        assert!(super::meta_defeq::same(inst, fx.expected), "instantiate_mvars ⇒ Exists A (fun x => P x)");
    });
}

/// **K1c-T2**：两层 binder，`?f x y =?= g y x` ⇒ `?f := λx y. g y x`。
#[test]
fn k1c_t2_miller_two_binders() {
    with_tc2(|tc, fx, mx| {
        let (v0, v1) = (tc.ctx.mk_var(0), tc.ctx.mk_var(1));
        let fxy = {
            let t = app(tc, mx.mf, v1);
            app(tc, t, v0)
        };
        let gyx = {
            let t = app(tc, mx.g, v0);
            app(tc, t, v1)
        };
        let lhs = {
            let i = lam(tc, fx.a, fxy);
            lam(tc, fx.a, i)
        };
        let rhs = {
            let i = lam(tc, fx.a, gyx);
            lam(tc, fx.a, i)
        };
        assert!(tc.is_def_eq(lhs, rhs), "fun x y => ?f x y =?= fun x y => g y x");
        let vf = tc.ctx.mvars.value(104).expect("?f assigned");
        assert!(super::meta_defeq::same(vf, rhs), "?f := λx y. g y x");
    });
}

/// **K1c-T3**：非 pattern ⇒ false 且什么都没赋：`?p a =?= P a`（a 是常量）、`?f x x =?= g x x`（重复实参）。
#[test]
fn k1c_t3_non_patterns_are_false() {
    with_tc2(|tc, fx, mx| {
        let mpa = app(tc, fx.m_p, fx.w);
        let pa = app(tc, fx.p, fx.w);
        assert!(!tc.is_def_eq(mpa, pa), "?p a =?= P a (const arg)");
        assert!(!tc.is_def_eq(pa, mpa), "P a =?= ?p a (const arg)");
        assert!(none_assigned(tc));
        let v0 = tc.ctx.mk_var(0);
        let fxx = {
            let t = app(tc, mx.mf, v0);
            app(tc, t, v0)
        };
        let gxx = {
            let t = app(tc, mx.g, v0);
            app(tc, t, v0)
        };
        let lhs = lam(tc, fx.a, fxx);
        let rhs = lam(tc, fx.a, gxx);
        assert!(!tc.is_def_eq(lhs, rhs), "fun x => ?f x x =?= fun x => g x x (repeated arg)");
        assert!(none_assigned(tc));
    });
}

/// **K1c-T4**：scope。`?m : A` 建在顶层，`fun x => ?m =?= fun x => x` ⇒ false（`?m := x` 会逃出 binder），
/// 什么都没赋。
#[test]
fn k1c_t4_scope_violation() {
    with_tc2(|tc, fx, mx| {
        let v0 = tc.ctx.mk_var(0);
        let lhs = lam(tc, fx.a, mx.m);
        let rhs = lam(tc, fx.a, v0);
        assert!(!tc.is_def_eq(lhs, rhs), "fun x => ?m =?= fun x => x");
        assert!(!tc.is_def_eq(rhs, lhs), "fun x => x =?= fun x => ?m");
        assert!(none_assigned(tc));
        // 不提 x 的仍可赋。
        let lhs2 = lam(tc, fx.a, mx.m);
        let rhs2 = lam(tc, fx.a, fx.w);
        assert!(tc.is_def_eq(lhs2, rhs2), "fun x => ?m =?= fun x => a");
        assert!(tc.ctx.mvars.value(103).is_some());
    });
}

/// **K1c-T5**：occurs。`fun x => ?k x =?= fun x => f (?k x)` ⇒ false。
#[test]
fn k1c_t5_occurs() {
    with_tc2(|tc, fx, mx| {
        let v0 = tc.ctx.mk_var(0);
        let kx = app(tc, mx.mk, v0);
        let fkx = app(tc, mx.f, kx);
        let lhs = lam(tc, fx.a, kx);
        let rhs = lam(tc, fx.a, fkx);
        assert!(!tc.is_def_eq(lhs, rhs), "?k x =?= f (?k x)");
        assert!(none_assigned(tc));
    });
}

/// **K1c-T6**：类型检查。`?h : A → A`，`fun x => ?h x =?= fun x => P x` 的候选 `λx. P x : A → Prop` ⇒ 拒。
#[test]
fn k1c_t6_type_check() {
    with_tc2(|tc, fx, mx| {
        let v0 = tc.ctx.mk_var(0);
        let hx = app(tc, mx.mh, v0);
        let px = app(tc, fx.p, v0);
        let lhs = lam(tc, fx.a, hx);
        let rhs = lam(tc, fx.a, px);
        assert!(!tc.is_def_eq(lhs, rhs), "?h x =?= P x with ?h : A → A");
        assert!(none_assigned(tc));
        // 类型对的候选通过（同一形状，`?k : A → Prop`）。
        let kx = app(tc, mx.mk, v0);
        let lhs2 = lam(tc, fx.a, kx);
        assert!(tc.is_def_eq(lhs2, rhs), "?k x =?= P x with ?k : A → Prop");
    });
}

/// **K1c-T7**：`restore` 按内容判 —— 覆盖一个层赋值（同长度、不同值）⇒ 报"变了"。
#[test]
fn k1c_t7_restore_detects_overwrite() {
    with_tc2(|tc, _, _| {
        let zero = tc.ctx.zero();
        let one = tc.ctx.succ(zero);
        tc.ctx.mvars.assign_level(7, one);
        let cp = tc.ctx.mvars.checkpoint();
        tc.ctx.mvars.assign_level(7, zero);
        assert!(tc.ctx.mvars.restore(cp), "overwritten level assignment must count as a change");
        assert!(tc.ctx.mvars.level_value(7).is_some_and(|l| std::ptr::eq(l.as_ref(), one.as_ref())));
        let cp = tc.ctx.mvars.checkpoint();
        assert!(!tc.ctx.mvars.restore(cp), "no assignment ⇒ unchanged");
    });
}

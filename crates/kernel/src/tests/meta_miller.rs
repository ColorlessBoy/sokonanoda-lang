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

/// **K1c-T8**（item 3）：`value_type` 对**开的** Lam/Pi 不 panic。binder 下（`x : A`，深度 1）
/// `fun y : A => g x y` 的类型 = `A → Prop`；`(y : A) → g x y` 的类型 = `Prop`。
#[test]
fn k1c_t8_value_type_open_lam_pi() {
    with_tc2(|tc, fx, mx| {
        let env0 = tc.empty_env();
        let dom = tc.eval(0, env0, fx.a);
        let x = tc.mk_bvar_hc(0, dom);
        let env = crate::value::env_extend(tc.arena, env0, x);
        let (v0, v1) = (tc.ctx.mk_var(0), tc.ctx.mk_var(1));
        let gxy = {
            let t = app(tc, mx.g, v1);
            app(tc, t, v0)
        };
        let n = tc.ctx.anonymous();
        let open_lam = tc.ctx.mk_lambda(n, BinderStyle::Default, fx.a, gxy);
        let open_pi = tc.ctx.mk_pi(n, BinderStyle::Default, fx.a, gxy);
        let lv = tc.eval(1, env, open_lam);
        let lt = tc.value_type(1, lv);
        let lq = tc.quote(1, lt);
        let zero = tc.ctx.zero();
        let prop = tc.ctx.mk_sort(zero);
        let a_prop = tc.ctx.mk_pi(n, BinderStyle::Default, fx.a, prop);
        assert!(super::meta_defeq::same(lq, a_prop), "type of open `fun y => g x y` is A → Prop");
        let pv = tc.eval(1, env, open_pi);
        let pt = tc.value_type(1, pv);
        let pq = tc.quote(1, pt);
        assert!(super::meta_defeq::same(pq, prop), "type of open `(y : A) → g x y` is Prop");
    });
}

// ---------------------------------------------------------------------------
// **C 案（2026-11-03）**：元变量赋值**带 scope**（Lean `MetavarDecl.lctx` 的 de Bruijn 版）。
//
// 这一组的锚是两句话：**存的值只许提到元变量自己上下文里的 binder**，重放时**抬到用点**。
// `depth_of == 0`（今天所有调用方）时两条都退化成"闭项"⇒ 与 C 之前逐字节相同（714 行对拍已证）。
// ---------------------------------------------------------------------------

/// `?s : A` 登记在**深度 1**（= 它看得见 1 个 binder）。
fn seed_scoped<'t>(tc: &mut TypeChecker<'_, 't, '_>, ty: ExprPtr<'t>) {
    tc.ctx.mvars.register_at(107, ty, 1);
    tc.ctx.mvars.register_kind(107, MetaKind::Natural);
}

/// **C-T1（正）**：深度 1 的 `?s` 可以赋成它上下文里的 binder（`?s := x`），
/// 而且 `instantiate_mvars` 在**更深的用点**要把它抬对（`λy. λx. ?s` ⇒ `λy. λx. #1`）。
#[test]
fn c_t1_scoped_meta_takes_its_own_binder() {
    with_tc2(|tc, fx, _mx| {
        seed_scoped(tc, fx.a);
        let s = tc.ctx.mk_meta(107, fx.a);
        let v0 = tc.ctx.mk_var(0);
        // `fun x => ?s =?= fun x => x` ⇒ true（?s 建在这层 binder 之下）
        let l1 = lam(tc, fx.a, s);
        let l2 = lam(tc, fx.a, v0);
        assert!(tc.is_def_eq(l1, l2), "?s := x 必须允许");
        // 存的是**在 ?s 自己的上下文里**写的项 ⇒ `#0`（不是用点的 `#1`）
        let got = tc.ctx.mvars.value(107).expect("?s 必须被赋值");
        assert_eq!(format!("{:?}", tc.ctx.read_expr(got)), format!("{:?}", tc.ctx.read_expr(v0)));
        // 更深的用点（两层 λ）⇒ 抬一位
        let deep_inner = lam(tc, fx.a, s);
        let deep = lam(tc, fx.a, deep_inner);
        let out = tc.ctx.instantiate_mvars(deep);
        // 具体断言：两层 λ 的 body 是 `#1`（抬一位）
        let mut body = out;
        for _ in 0..2 {
            let crate::expr::Expr::Lambda { body: b, .. } = tc.ctx.read_expr(body) else {
                panic!("应是两层 λ")
            };
            body = b;
        }
        assert!(
            matches!(tc.ctx.read_expr(body), crate::expr::Expr::Var { dbj_idx: 1, .. }),
            "两层 λ 之下 ?s 必须抬成 #1，实际 {:?}",
            tc.ctx.read_expr(body)
        );
    });
}

/// **C-T2（反）**：真正的"逃出 scope"只有一条 —— **用点比创建点更浅**（`depth < depth_of`）：
/// 值可能提到用点根本看不见的 binder。夹具：`?s` 登记在**深度 2**，只在**一层** λ 下比 ⇒ 必须拒。
///
/// ⚠ 写这条时踩过一次：最初想用"`?s`（depth 1）赋成两层 λ 下的外层 binder"当反例 —— 那是**合法**的：
/// de Bruijn 里创建上下文的 binder 在更深用点上**下标整体 +1**（lambda 下降一层），
/// 所以用点 `depth` 下合法的正是**最内 `d` 个**（`i >= depth - d`），`Var(1)` 落在里面。
#[test]
fn c_t2_scoped_meta_rejects_shallower_use() {
    with_tc2(|tc, fx, _mx| {
        tc.ctx.mvars.register_at(107, fx.a, 2);
        tc.ctx.mvars.register_kind(107, MetaKind::Natural);
        let s = tc.ctx.mk_meta(107, fx.a);
        let v0 = tc.ctx.mk_var(0);
        let lhs = lam(tc, fx.a, s);
        let rhs = lam(tc, fx.a, v0);
        assert!(!tc.is_def_eq(lhs, rhs), "用点(1) 比创建深度(2) 浅 ⇒ ?s := x 必须拒");
        assert!(tc.ctx.mvars.value(107).is_none(), "被拒的赋值不许留下");
    });
}

/// **C-T3（零变化）**：深度 **0** 的元变量仍然只收**闭项**（K1c-T4 的同一条判据，
/// 这里显式钉住"generalize 之后没放松"）。
#[test]
fn c_t3_depth_zero_still_requires_closed() {
    with_tc2(|tc, fx, mx| {
        let v0 = tc.ctx.mk_var(0);
        let lhs = lam(tc, fx.a, mx.m);
        let rhs = lam(tc, fx.a, v0);
        assert!(!tc.is_def_eq(lhs, rhs), "depth 0 的 ?m 不许收 binder");
        assert!(tc.ctx.mvars.value(103).is_none());
    });
}

/// **C4 回传（内核半）**：会话里对**调用方登记**的元变量的赋值，要能渲染成文本交回去
/// （前端据此写回 `MetaStore`，同一 `\0soko_m{id}` 的所有出现才看到同一个值）。
#[test]
fn c4_external_assignment_is_exported_as_text() {
    with_tc2(|tc, fx, _mx| {
        tc.ctx.mvars.register_at(200, fx.a, 0);
        tc.ctx.mvars.register_kind(200, MetaKind::Natural);
        tc.ctx.mvars.note_external(200);
        let m = tc.ctx.mk_meta(200, fx.a);
        assert!(tc.is_def_eq(m, fx.w), "?200 =?= a 必须赋上");
        let out = tc.take_external_assignments(&[]);
        assert_eq!(out.len(), 1, "恰好一条外部赋值，实际 {out:?}");
        assert_eq!(out[0].0, 200);
        assert!(!out[0].1.is_empty(), "文本不许为空");
        // 拿第二次是空的（take 语义，会话收尾只交一次）
        assert!(tc.take_external_assignments(&[]).is_empty());
    });
}

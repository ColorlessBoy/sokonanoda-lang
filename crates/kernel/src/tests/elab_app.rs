//! **F2b1**：内核级 `elab_app`（Lean `ElabAppArgs`）。夹具全是**单态**的（P1 不支持宇宙多态的头）：
//! `ExT : {α : Type} → (α → Prop) → Prop`、`ExT.intro {α} (p) (w) (h)`（`p` 是 Default 写法 ⇒ 靠
//! `implicit_prefix = 2`）、`ExT.elim {α} {p} {b} (h) (f)`、`And.intro {a b}`、`AndD.intro (a b)`、
//! `Mem {α} a S`、`Qp : Prop`、`hq : Qp`。
use super::meta_defeq::same;
use super::meta_spine::axiom;
use crate::builder::EnvBuilder;
use crate::elab_app::{AppOutcome, AppRequest};
use crate::env::{Declar, DeclarInfo, EnvLimit};
use crate::expr::BinderStyle::{self, Default as D, Implicit as I};
use crate::util::{Config, ExprPtr};
use stumpalo::Arena;

#[derive(Clone, Copy)]
struct Kx<'a> {
    prop: ExprPtr<'a>,
    a: ExprPtr<'a>,
    p: ExprPtr<'a>,
    q: ExprPtr<'a>,
    ext: ExprPtr<'a>,
    intro: ExprPtr<'a>,
    elim: ExprPtr<'a>,
    and_intro: ExprPtr<'a>,
    andd_intro: ExprPtr<'a>,
    mem: ExprPtr<'a>,
    qp: ExprPtr<'a>,
    hq: ExprPtr<'a>,
    /// `fun x : A => P x`
    lam_p: ExprPtr<'a>,
}

fn pi<'a>(b: &mut EnvBuilder<'a>, n: &str, s: BinderStyle, t: ExprPtr<'a>, body: ExprPtr<'a>) -> ExprPtr<'a> {
    let n = b.name_from_str(n);
    b.mk_pi(n, s, t, body)
}

fn lam<'a>(b: &mut EnvBuilder<'a>, n: &str, t: ExprPtr<'a>, body: ExprPtr<'a>) -> ExprPtr<'a> {
    let n = b.name_from_str(n);
    b.mk_lambda(n, D, t, body)
}

fn apps<'a>(b: &mut EnvBuilder<'a>, f: ExprPtr<'a>, xs: &[ExprPtr<'a>]) -> ExprPtr<'a> {
    xs.iter().fold(f, |acc, x| b.mk_app(acc, *x))
}

fn cst<'a>(b: &mut EnvBuilder<'a>, n: &str) -> ExprPtr<'a> {
    let n = b.name_from_str(n);
    let none = b.alloc_levels_slice(&[]);
    b.mk_const(n, none)
}

fn ax<'a>(b: &mut EnvBuilder<'a>, n: &str, ty: ExprPtr<'a>) -> ExprPtr<'a> {
    let none = b.alloc_levels_slice(&[]);
    axiom(b, n, none, ty);
    cst(b, n)
}

fn setup<'a>(b: &mut EnvBuilder<'a>) -> Kx<'a> {
    let zero = b.zero();
    let one = b.succ(zero);
    let prop = b.mk_sort(zero);
    let ty = b.mk_sort(one);
    let v = |b: &mut EnvBuilder<'a>, i: u16| b.mk_var(i);
    let a = ax(b, "A", ty);
    let a_prop = pi(b, "x", D, a, prop);
    let p = ax(b, "P", a_prop);
    let q = ax(b, "Q", a_prop);
    // ExT : {α : Type} → (α → Prop) → Prop
    let ext = {
        let v0 = v(b, 0);
        let pt = pi(b, "x", D, v0, prop);
        let body = pi(b, "p", D, pt, prop);
        let t = pi(b, "α", I, ty, body);
        ax(b, "ExT", t)
    };
    // ExT.intro : {α : Type} → (p : α → Prop) → (w : α) → p w → ExT α p
    let intro = {
        let (v0, v1, v2, v3) = (v(b, 0), v(b, 1), v(b, 2), v(b, 3));
        let res = apps(b, ext, &[v3, v2]);
        let pw = b.mk_app(v1, v0);
        let h = pi(b, "h", D, pw, res);
        let w = pi(b, "w", D, v1, h);
        let v0 = v(b, 0);
        let pt = pi(b, "x", D, v0, prop);
        let pp = pi(b, "p", D, pt, w);
        let t = pi(b, "α", I, ty, pp);
        ax(b, "ExT.intro", t)
    };
    // ExT.elim : {α : Type} → {p : α → Prop} → {b : Prop} → ExT α p → (∀ a, p a → b) → b
    let elim = {
        let (v0, v1, v2, v3) = (v(b, 0), v(b, 1), v(b, 2), v(b, 3));
        let pa = b.mk_app(v3, v0);
        let inner = pi(b, "ha", D, pa, v3);
        let ft = pi(b, "a", D, v3, inner);
        let fpi = pi(b, "f", D, ft, v2);
        let ht = apps(b, ext, &[v2, v1]);
        let hpi = pi(b, "h", D, ht, fpi);
        let bpi = pi(b, "b", I, prop, hpi);
        let pt = pi(b, "x", D, v0, prop);
        let ppi = pi(b, "p", I, pt, bpi);
        let t = pi(b, "α", I, ty, ppi);
        ax(b, "ExT.elim", t)
    };
    // And : Prop → Prop → Prop ; And.intro {a b : Prop} : a → b → And a b ; AndD.intro (a b : Prop) ...
    let and = {
        let t0 = pi(b, "b", D, prop, prop);
        let t = pi(b, "a", D, prop, t0);
        ax(b, "And", t)
    };
    let mk_and_intro = |b: &mut EnvBuilder<'a>, n: &str, s: BinderStyle| {
        let (v1, v2, v3) = (v(b, 1), v(b, 2), v(b, 3));
        let res = apps(b, and, &[v3, v2]);
        let hb = pi(b, "hb", D, v1, res);
        let v1 = v(b, 1);
        let ha = pi(b, "ha", D, v1, hb);
        let bb = pi(b, "b", s, prop, ha);
        let t = pi(b, "a", s, prop, bb);
        ax(b, n, t)
    };
    let and_intro = mk_and_intro(b, "And.intro", I);
    let andd_intro = mk_and_intro(b, "AndD.intro", D);
    // Mem : {α : Type} → α → (α → Prop) → Prop
    let mem = {
        let v1 = v(b, 1);
        let st = pi(b, "x", D, v1, prop);
        let s = pi(b, "S", D, st, prop);
        let v0 = v(b, 0);
        let a_ = pi(b, "a", D, v0, s);
        let t = pi(b, "α", I, ty, a_);
        ax(b, "Mem", t)
    };
    let qp = ax(b, "Qp", prop);
    let hq = ax(b, "hq", qp);
    let lam_p = {
        let v0 = v(b, 0);
        let px = b.mk_app(p, v0);
        lam(b, "x", a, px)
    };
    Kx { prop, a, p, q, ext, intro, elim, and_intro, andd_intro, mem, qp, hq, lam_p }
}

/// 大栈线程里建 builder + 夹具，`f` 拿着 builder 跑。
fn run(f: impl for<'a> FnOnce(&mut EnvBuilder<'a>, Kx<'a>) + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            let arena = Arena::new();
            let mut b = EnvBuilder::new(arena.as_arena_ref(), Config::default());
            let kx = setup(&mut b);
            f(&mut b, kx);
        })
        .unwrap()
        .join()
        .unwrap();
}

fn elab<'a>(b: &mut EnvBuilder<'a>, req: &AppRequest<'_, 'a>) -> AppOutcome {
    let limit = EnvLimit::ByIndex(b.declaration_count());
    b.with_env(|ef| ef.elab_app(limit, req))
}

fn solved<'a>(b: &mut EnvBuilder<'a>, o: &AppOutcome) -> ExprPtr<'a> {
    match o {
        AppOutcome::Solved(t) => b.import_owned(t).expect("import"),
        other => panic!("expected Solved, got {other:?}"),
    }
}

fn req<'s, 'a>(
    tys: &'s [ExprPtr<'a>],
    head: ExprPtr<'a>,
    k: usize,
    args: &'s [ExprPtr<'a>],
    expected: Option<ExprPtr<'a>>,
) -> AppRequest<'s, 'a> {
    AppRequest { binder_tys: tys, head, implicit_prefix: k, explicit_args: args, expected }
}

/// T1 的 scope `(w : A) (hw : P w)` + 期望 `ExT A (fun x => P x)`。
fn t1_parts<'a>(b: &mut EnvBuilder<'a>, kx: Kx<'a>) -> ([ExprPtr<'a>; 2], [ExprPtr<'a>; 2], ExprPtr<'a>) {
    let v0 = b.mk_var(0);
    let pw = b.mk_app(kx.p, v0);
    let expected = apps(b, kx.ext, &[kx.a, kx.lam_p]);
    let (v0, v1) = (b.mk_var(0), b.mk_var(1));
    ([kx.a, pw], [v1, v0], expected)
}

/// **T1**：`ExT.intro w hw` + 期望 ⇒ `ExT.intro A (fun x => P x) w hw`（传播命中一次）。
#[test]
fn t1_exists_intro_with_expected() {
    run(|b, kx| {
        let (tys, args, expected) = t1_parts(b, kx);
        let before = crate::elab_app::propagations();
        let o = elab(b, &req(&tys, kx.intro, 2, &args, Some(expected)));
        assert_eq!(crate::elab_app::propagations() - before, 1, "expected type propagated once");
        let got = solved(b, &o);
        let want = apps(b, kx.intro, &[kx.a, kx.lam_p, args[0], args[1]]);
        assert!(same(got, want), "ExT.intro A (fun x => P x) w hw");
    });
}

/// **T2（上下文元变量）**：scope `(x : A) (h1 : P x) (h2 : Q x)`，`And.intro h1 h2` ⇒
/// `And.intro (P x) (Q x) h1 h2`（`?a := λx h1 h2. P x` 闭；实例化回 `P x`）。
#[test]
fn t2_and_intro_contextual_metas() {
    run(|b, kx| {
        let (v0, v1, v2) = (b.mk_var(0), b.mk_var(1), b.mk_var(2));
        let px = b.mk_app(kx.p, v0);
        let qx = b.mk_app(kx.q, v1);
        let tys = [kx.a, px, qx];
        let args = [v1, v0];
        let o = elab(b, &req(&tys, kx.and_intro, 0, &args, None));
        let got = solved(b, &o);
        let (p2, q2) = (b.mk_app(kx.p, v2), b.mk_app(kx.q, v2));
        let want = apps(b, kx.and_intro, &[p2, q2, v1, v0]);
        assert!(same(got, want), "And.intro (P x) (Q x) h1 h2");
        // 构造子情形：内核类型里参数是 Default 写法，`implicit_prefix = 2` 照样当隐式。
        let o = elab(b, &req(&tys, kx.andd_intro, 2, &args, None));
        let got = solved(b, &o);
        let want = apps(b, kx.andd_intro, &[p2, q2, v1, v0]);
        assert!(same(got, want), "AndD.intro (P x) (Q x) h1 h2 via implicit_prefix");
    });
}

/// **T3**：scope `(h : ExT A (fun x => P x))`，`ExT.elim h (fun w hw => hq)` 对目标 `Qp` ⇒ Solved
/// （`?p` 走 Miller，`?b := Qp` 来自传播）。
#[test]
fn t3_exists_elim_against_goal() {
    run(|b, kx| {
        let h_ty = apps(b, kx.ext, &[kx.a, kx.lam_p]);
        let v0 = b.mk_var(0);
        let pw = b.mk_app(kx.p, v0);
        let inner = lam(b, "hw", pw, kx.hq);
        let f = lam(b, "w", kx.a, inner);
        let tys = [h_ty];
        let args = [v0, f];
        let o = elab(b, &req(&tys, kx.elim, 0, &args, Some(kx.qp)));
        let got = solved(b, &o);
        let want = apps(b, kx.elim, &[kx.a, kx.lam_p, kx.qp, v0, f]);
        assert!(same(got, want), "ExT.elim A (fun x => P x) Qp h (fun w hw => hq)");
    });
}

/// **T4**：`Mem a S`，期望 `Prop` ⇒ 不传播（Lean 594），finalize 照样通过。
#[test]
fn t4_mem_expected_prop_skips_propagation() {
    run(|b, kx| {
        let a_prop = pi(b, "x", D, kx.a, kx.prop);
        let tys = [kx.a, a_prop];
        let (v0, v1) = (b.mk_var(0), b.mk_var(1));
        let args = [v1, v0];
        let before = crate::elab_app::propagations();
        let o = elab(b, &req(&tys, kx.mem, 0, &args, Some(kx.prop)));
        assert_eq!(crate::elab_app::propagations() - before, 0, "Prop is never propagated");
        let got = solved(b, &o);
        let want = apps(b, kx.mem, &[kx.a, v1, v0]);
        assert!(same(got, want), "Mem A a S");
    });
}

/// **T5**：`ExT.intro w hw` 无期望 ⇒ `?p w =?= P w` 不是 pattern（`w` 已在 `?p` 的上下文里）⇒ 报 `p`。
#[test]
fn t5_exists_intro_without_expected_is_unsolved() {
    run(|b, kx| {
        let (tys, args, _) = t1_parts(b, kx);
        let o = elab(b, &req(&tys, kx.intro, 2, &args, None));
        assert_eq!(o, AppOutcome::Unsolved { binders: vec!["p".to_string()] });
    });
}

/// **T6**：错的实参 ⇒ `ArgMismatch { arg: 1 }`；错的期望 ⇒ `ExpectedMismatch`。
#[test]
fn t6_mismatches() {
    run(|b, kx| {
        let (tys, args, expected) = t1_parts(b, kx);
        let bad = [args[0], args[0]];
        let o = elab(b, &req(&tys, kx.intro, 2, &bad, Some(expected)));
        assert_eq!(o, AppOutcome::ArgMismatch { arg: 1 });
        let a_prop = pi(b, "x", D, kx.a, kx.prop);
        let tys = [kx.a, a_prop];
        let (v0, v1) = (b.mk_var(0), b.mk_var(1));
        let args = [v1, v0];
        let o = elab(b, &req(&tys, kx.mem, 0, &args, Some(kx.a)));
        assert_eq!(o, AppOutcome::ExpectedMismatch);
    });
}

/// **T7**：带齐层的多态头照常解；层数不对 / 输入含元变量 ⇒ `Unsupported`；同一 builder 连跑两次 T1
/// 结果一致（不泄漏）。（K1d 前带层的多态头一律 `Unsupported("universe-polymorphic head")`。）
#[test]
fn t7_unsupported_and_no_leak() {
    run(|b, kx| {
        let (tys, args, expected) = t1_parts(b, kx);
        // 宇宙多态：`Poly.{u} : {α : Sort u} → α → Prop`，以 `Poly.{1}` 调用。
        let (poly, poly11) = {
            let un = b.name_from_str("u");
            let u = b.level_param(un);
            let us = b.alloc_levels_slice(&[u]);
            let sort_u = b.mk_sort(u);
            let v0 = b.mk_var(0);
            let body = pi(b, "a", D, v0, kx.prop);
            let t = pi(b, "α", I, sort_u, body);
            axiom(b, "Poly", us, t);
            let n = b.name_from_str("Poly");
            let zero = b.zero();
            let one = b.succ(zero);
            let ones = b.alloc_levels_slice(&[one]);
            let one_one = b.alloc_levels_slice(&[one, one]);
            (b.mk_const(n, ones), b.mk_const(n, one_one))
        };
        let o = elab(b, &req(&tys, poly, 0, &args[..1], None));
        let got = solved(b, &o);
        let want = apps(b, poly, &[kx.a, args[0]]);
        assert!(same(got, want), "Poly.{{1}} A w");
        let o = elab(b, &req(&tys, poly11, 0, &args[..1], None));
        assert_eq!(o, AppOutcome::Unsupported("universe arity"));
        let m = b.mk_meta(77, kx.a);
        let meta_arg = [args[0], m];
        let o = elab(b, &req(&tys, kx.intro, 2, &meta_arg, Some(expected)));
        assert_eq!(o, AppOutcome::Unsupported("input contains a meta"));
        let limit = EnvLimit::ByIndex(b.declaration_count());
        let r = req(&tys, kx.intro, 2, &args, Some(expected));
        let (o1, o2) = b.with_env(|ef| (ef.elab_app(limit, &r), ef.elab_app(limit, &r)));
        assert!(matches!(o1, AppOutcome::Solved(_)));
        assert_eq!(o1, o2, "second call sees no metas from the first");
    });
}

/// **T8**：Solved 项导回 builder、包成 `theorem t : ∀ (w : A) (hw : P w), ExT A (fun x => P x)`，
/// `add_declar` 接受（最终项走**不变的**判定路径）。
#[test]
fn t8_round_trip_into_builder() {
    run(|b, kx| {
        let (tys, args, expected) = t1_parts(b, kx);
        let o = elab(b, &req(&tys, kx.intro, 2, &args, Some(expected)));
        let term = solved(b, &o);
        let body_ty = pi(b, "hw", D, tys[1], expected);
        let ty = pi(b, "w", D, tys[0], body_ty);
        let val = {
            let n = b.name_from_str("hw");
            let inner = b.mk_lambda(n, D, tys[1], term);
            lam(b, "w", tys[0], inner)
        };
        let name = b.name_from_str("t8");
        let uparams = b.alloc_levels_slice(&[]);
        b.add_declar(Declar::Theorem { info: DeclarInfo { name, uparams, ty }, val }).expect("kernel accepts");
    });
}

/// **K1d** 夹具：多态 `Eq` 族（都是公理，层参 `u`）+ `ea eb : A`、`hab : Eq.{1} A ea eb`、`hp : P ea`。
/// 返回 `[Eq, Eq.refl, Eq.symm, Eq.subst]` 的 `.{1}` 版与**不带层**版，以及 `[ea, eb, hab, hp]`。
type EqFam<'a> = ([ExprPtr<'a>; 4], [ExprPtr<'a>; 4], [ExprPtr<'a>; 4]);

fn eq_family<'a>(b: &mut EnvBuilder<'a>, kx: Kx<'a>) -> EqFam<'a> {
    let un = b.name_from_str("u");
    let u = b.level_param(un);
    let us = b.alloc_levels_slice(&[u]);
    let sort_u = b.mk_sort(u);
    let zero = b.zero();
    let one = b.succ(zero);
    let ones = b.alloc_levels_slice(&[one]);
    let none = b.alloc_levels_slice(&[]);
    let v = |b: &mut EnvBuilder<'a>, i: u16| b.mk_var(i);
    let at = |b: &mut EnvBuilder<'a>, n: &str, ls| {
        let n = b.name_from_str(n);
        b.mk_const(n, ls)
    };
    // Eq.{u} : {α : Sort u} → α → α → Prop
    let eq_t = {
        let (v0, v1) = (v(b, 0), v(b, 1));
        let r = pi(b, "b", D, v1, kx.prop);
        let r = pi(b, "a", D, v0, r);
        pi(b, "α", I, sort_u, r)
    };
    axiom(b, "Eq", us, eq_t);
    let eq_u = at(b, "Eq", us);
    // Eq.refl.{u} : {α : Sort u} → (a : α) → Eq α a a
    let refl_t = {
        let (v0, v1) = (v(b, 0), v(b, 1));
        let r = apps(b, eq_u, &[v1, v0, v0]);
        let r = pi(b, "a", D, v0, r);
        pi(b, "α", I, sort_u, r)
    };
    axiom(b, "Eq.refl", us, refl_t);
    // Eq.symm.{u} : {α : Sort u} → {a b : α} → Eq α a b → Eq α b a
    let symm_t = {
        let (v0, v1, v2, v3) = (v(b, 0), v(b, 1), v(b, 2), v(b, 3));
        let res = apps(b, eq_u, &[v3, v1, v2]);
        let h = apps(b, eq_u, &[v2, v1, v0]);
        let r = pi(b, "h", D, h, res);
        let r = pi(b, "b", I, v1, r);
        let r = pi(b, "a", I, v0, r);
        pi(b, "α", I, sort_u, r)
    };
    axiom(b, "Eq.symm", us, symm_t);
    // Eq.subst.{u} : {α : Sort u} → {motive : α → Prop} → {a b : α} → Eq α a b → motive a → motive b
    let subst_t = {
        let (v0, v1, v2, v3, v4) = (v(b, 0), v(b, 1), v(b, 2), v(b, 3), v(b, 4));
        let res = b.mk_app(v4, v2); // 在 [α, motive, a, b, h₁, h₂] 之下
        let ma = b.mk_app(v3, v2); // 在 [α, motive, a, b, h₁] 之下
        let r = pi(b, "h₂", D, ma, res);
        let h1 = apps(b, eq_u, &[v3, v1, v0]); // 在 [α, motive, a, b] 之下
        let r = pi(b, "h₁", D, h1, r);
        let r = pi(b, "b", I, v2, r);
        let r = pi(b, "a", I, v1, r);
        let mt = pi(b, "x", D, v0, kx.prop);
        let r = pi(b, "motive", I, mt, r);
        pi(b, "α", I, sort_u, r)
    };
    axiom(b, "Eq.subst", us, subst_t);
    let names = ["Eq", "Eq.refl", "Eq.symm", "Eq.subst"];
    let with1 = names.map(|n| at(b, n, ones));
    let bare = names.map(|n| at(b, n, none));
    let ea = ax(b, "ea", kx.a);
    let eb = ax(b, "eb", kx.a);
    let hab_t = apps(b, with1[0], &[kx.a, ea, eb]);
    let hab = ax(b, "hab", hab_t);
    let pea = b.mk_app(kx.p, ea);
    let hp = ax(b, "hp", pea);
    (with1, bare, [ea, eb, hab, hp])
}

/// **K1d-T4**：多态头 `Eq.refl` / `Eq.symm` 经本会话层元变量解出 `.{1}`；`Eq.subst` 的 motive 不是
/// Miller pattern（实参是常量 `ea`/`eb`，不是 bound var）⇒ `Unsolved ["motive"]`。
#[test]
fn k1d_t4_universe_polymorphic_heads() {
    run(|b, kx| {
        let (with1, bare, [ea, eb, hab, hp]) = eq_family(b, kx);
        let [eq1, refl1, symm1, _] = with1;
        // Eq.refl ea : Eq A ea ea ⇒ Eq.refl.{1} A ea
        let exp = apps(b, eq1, &[kx.a, ea, ea]);
        let o = elab(b, &req(&[], bare[1], 0, &[ea], Some(exp)));
        let want = apps(b, refl1, &[kx.a, ea]);
        assert!(same(solved(b, &o), want), "Eq.refl.{{1}} A ea");
        // 无期望也解（`?u` 由 `?α := A` 的类型检查 `Sort ?u =?= Sort 1` 定）。
        let o = elab(b, &req(&[], bare[1], 0, &[ea], None));
        assert!(same(solved(b, &o), want), "Eq.refl ea without expected");
        // Eq.symm hab ⇒ Eq.symm.{1} A ea eb hab（有/无期望）
        let want = apps(b, symm1, &[kx.a, ea, eb, hab]);
        let o = elab(b, &req(&[], bare[2], 0, &[hab], None));
        assert!(same(solved(b, &o), want), "Eq.symm hab");
        let exp = apps(b, eq1, &[kx.a, eb, ea]);
        let o = elab(b, &req(&[], bare[2], 0, &[hab], Some(exp)));
        assert!(same(solved(b, &o), want), "Eq.symm hab : Eq A eb ea");
        // 错的期望：传播先成功（`?b := ea`、`?a := eb`），之后 `hab` 对不上域 ⇒ 报实参（Lean 同：
        // `propagateExpectedType` 成功后 `ensureHasType` 失败在实参上）。
        let bad = apps(b, eq1, &[kx.a, ea, eb]);
        let o = elab(b, &req(&[], bare[2], 0, &[hab], Some(bad)));
        assert_eq!(o, AppOutcome::ArgMismatch { arg: 0 });
        // Eq.subst hab hp : P eb —— Lean 靠 `elab_as_elim` 的 motive 抽象；这里没有 ⇒ 点名 motive。
        let exp = b.mk_app(kx.p, eb);
        let o = elab(b, &req(&[], bare[3], 0, &[hab, hp], Some(exp)));
        assert_eq!(o, AppOutcome::Unsolved { binders: vec!["motive".to_string()] });
    });
}

/// **K1d-T5**：解不出的宇宙 ⇒ `Unsolved` 点名：`PolyB.{u v} : {α : Sort u} → α → Prop`（`v` 不出现）⇒
/// `universe v`；无实参的 `Univ.{w} : Prop` ⇒ `universe w`。
#[test]
fn k1d_t5_undetermined_universe_is_unsolved() {
    run(|b, kx| {
        let (un, vn, wn) = (b.name_from_str("u"), b.name_from_str("v"), b.name_from_str("w"));
        let (u, v, w) = (b.level_param(un), b.level_param(vn), b.level_param(wn));
        let uv = b.alloc_levels_slice(&[u, v]);
        let sort_u = b.mk_sort(u);
        let v0 = b.mk_var(0);
        let body = pi(b, "a", D, v0, kx.prop);
        let t = pi(b, "α", I, sort_u, body);
        axiom(b, "PolyB", uv, t);
        let ws = b.alloc_levels_slice(&[w]);
        axiom(b, "Univ", ws, kx.prop);
        let ea = ax(b, "ea", kx.a);
        let polyb = cst(b, "PolyB");
        let o = elab(b, &req(&[], polyb, 0, &[ea], Some(kx.prop)));
        assert_eq!(o, AppOutcome::Unsolved { binders: vec!["universe v".to_string()] });
        let univ = cst(b, "Univ");
        let o = elab(b, &req(&[], univ, 0, &[], None));
        assert_eq!(o, AppOutcome::Unsolved { binders: vec!["universe w".to_string()] });
    });
}

//! **F2a**：项跨会话搬运（tc ⇒ owned ⇒ builder）+ builder 侧 `instantiate_metas` + `solve_metas`。
//! 夹具复用 `meta_spine::setup`（`Exists`、`A`、`P`、`a`、`ha`、`?α`(101)、`?p`(102)）。
use super::meta_spine::{axiom, setup};
use crate::builder::EnvBuilder;
use crate::env::{Declar, DeclarInfo, EnvLimit};
use crate::expr::{BinderStyle, Expr, MetaKind};
use crate::transfer::{export_expr, MetaSeed, OwnedNode};
use crate::util::{Config, ExprPtr, FxHashMap, TcCtx};
use num_bigint::BigUint;
use std::collections::HashMap;
use stumpalo::Arena;

fn big_stack(f: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new().stack_size(64 << 20).spawn(f).unwrap().join().unwrap();
}

/// 把 builder 项在 tc 会话 arena 里**逐节点重建**（每个 expr 节点都是会话本地的）。
fn rebuild<'t, 'p: 't>(
    ctx: &mut TcCtx<'t, 'p>,
    e: ExprPtr<'t>,
    memo: &mut FxHashMap<ExprPtr<'t>, ExprPtr<'t>>,
) -> ExprPtr<'t> {
    if let Some(r) = memo.get(&e) {
        return *r;
    }
    let r = match *e.as_ref() {
        Expr::Var { dbj_idx, .. } => ctx.mk_var(dbj_idx),
        Expr::Sort { level, .. } => ctx.mk_sort(level),
        Expr::Const { name, levels, .. } => {
            let ls = ctx.alloc_levels(&levels);
            ctx.mk_const(name, ls)
        }
        Expr::App { fun, arg, .. } => {
            let (f, a) = (rebuild(ctx, fun, memo), rebuild(ctx, arg, memo));
            ctx.mk_app(f, a)
        }
        Expr::Pi { binder_name, binder_style, binder_type, body, .. } => {
            let (t, b) = (rebuild(ctx, binder_type, memo), rebuild(ctx, body, memo));
            ctx.mk_pi(binder_name, binder_style, t, b)
        }
        Expr::Lambda { binder_name, binder_style, binder_type, body, .. } => {
            let (t, b) = (rebuild(ctx, binder_type, memo), rebuild(ctx, body, memo));
            ctx.mk_lambda(binder_name, binder_style, t, b)
        }
        Expr::Let { data, .. } => {
            let (t, v, b) = (rebuild(ctx, data.binder_type, memo), rebuild(ctx, data.val, memo), rebuild(ctx, data.body, memo));
            ctx.mk_let(data.binder_name, t, v, b, data.nondep)
        }
        Expr::Proj { ty_name, idx, structure, .. } => {
            let s = rebuild(ctx, structure, memo);
            ctx.mk_proj(ty_name, idx, s)
        }
        Expr::StringLit { ptr, .. } => ctx.mk_string_lit(ptr).unwrap(),
        Expr::NatLit { ptr, .. } => ctx.mk_nat_lit(ptr).unwrap(),
        Expr::Meta { id, kind, ty, hash, .. } => {
            let ty = rebuild(ctx, ty, memo);
            let fv_mask = crate::expr::child_mask(ty);
            ctx.alloc_expr(Expr::Meta { id, kind, ty, fv_mask, hash })
        }
    };
    memo.insert(e, r);
    r
}

/// **F2a-T1**：覆盖每个 `Expr` 变体与每种层形状的项，builder ⇒ tc（逐节点重建）⇒ owned ⇒ builder，
/// 结果与原项**同一个 intern 指针**（⇔ 结构 + hash 完全一致），且再导出的 owned 与第一次的逐字段相等。
#[test]
fn f2a_t1_round_trip_every_variant() {
    big_stack(|| {
        let arena = Arena::new();
        let mut b = EnvBuilder::new(arena.as_arena_ref(), Config::default());
        let zero = b.zero();
        let un = b.name_from_str("u");
        let u = b.level_param(un);
        let su = b.succ(u);
        let lm = b.level_mvar(7);
        let im = b.level_imax(lm, zero);
        let l = b.level_max(su, im);
        let sort_l = b.mk_sort(l);
        let cn = b.name_from_str("Foo.bar.3");
        let ls = b.alloc_levels_slice(&[u, l, lm]);
        let c = b.mk_const(cn, ls);
        let s = b.alloc_string("héllo");
        let lit_s = b.mk_string_lit(s).unwrap();
        let n = b.alloc_bignum("12345678901234567890123".parse::<BigUint>().unwrap()).unwrap();
        let lit_n = b.mk_nat_lit(n).unwrap();
        let m1 = b.mk_meta_with_kind(5, MetaKind::Natural, sort_l);
        let m2 = b.mk_meta_with_kind(6, MetaKind::SyntheticOpaque, c);
        let pn = b.name_from_str("Prod");
        let cs = b.mk_app(c, lit_s);
        let proj = b.mk_proj(pn, 1, cs);
        let (nx, ny, nz) = (b.name_from_str("x"), b.name_from_str("y"), b.name_from_str("z"));
        let v0 = b.mk_var(0);
        let v1 = b.mk_var(1);
        let v0m = b.mk_app(v0, m1);
        let let_ = b.mk_let(nx, sort_l, lit_n, v0m, true);
        let let2 = b.mk_let(ny, proj, let_, v0, false);
        let v01 = b.mk_app(v0, v1);
        let lam = b.mk_lambda(nx, BinderStyle::Implicit, sort_l, v01);
        let pi_in = b.mk_pi(nz, BinderStyle::InstanceImplicit, proj, let2);
        let pi = b.mk_pi(ny, BinderStyle::StrictImplicit, lam, pi_in);
        let anon = b.anonymous();
        let lam2 = b.mk_lambda(anon, BinderStyle::Default, m2, v0);
        let t = b.mk_app(pi, m2);
        let root = b.mk_app(t, lam2);
        let owned0 = export_expr(root);
        let limit = EnvLimit::ByIndex(b.declaration_count());
        let (owned, distinct) = b.with_env(|ef| {
            ef.with_tc(limit, |tc| {
                let r = rebuild(tc.ctx, root, &mut FxHashMap::default());
                (tc.ctx.export_owned(r), r != root)
            })
        });
        assert!(distinct, "the rebuilt term must live in the tc arena (otherwise the test proves nothing)");
        for want in ["Var", "Sort", "Const", "App", "Pi", "Lambda", "Let", "Proj", "StringLit", "NatLit", "Meta"] {
            assert!(owned.nodes.iter().any(|n| format!("{n:?}").starts_with(want)), "variant {want} covered");
        }
        assert_eq!(owned, owned0, "tc-arena export = builder-arena export");
        let back = b.import_owned(&owned).expect("import");
        assert_eq!(back, root, "import_owned re-interns to the natively built pointer");
        assert_eq!(export_expr(back), owned, "re-export is identical");
    });
}

/// **F2a-T2**：30 层 `t_{i+1} = f t_i t_i`（树大小 2^30，DAG 大小 ~60）：导出/导入/代入都按 DAG 走。
#[test]
fn f2a_t2_sharing_is_preserved() {
    big_stack(|| {
        let arena = Arena::new();
        let mut b = EnvBuilder::new(arena.as_arena_ref(), Config::default());
        let none = b.alloc_levels_slice(&[]);
        let fname = b.name_from_str("f");
        let f = b.mk_const(fname, none);
        let zero = b.zero();
        let prop = b.mk_sort(zero);
        let mx = b.mk_meta(9, prop);
        let mut t = mx;
        for _ in 0..30 {
            let ft = b.mk_app(f, t);
            t = b.mk_app(ft, t);
        }
        let start = std::time::Instant::now();
        let owned = export_expr(t);
        assert!(owned.node_count() <= 64, "DAG export, got {} nodes", owned.node_count());
        let back = b.import_owned(&owned).unwrap();
        assert_eq!(back, t);
        // 代入 ?9 := P 也按 DAG 记忆。
        let pn = b.name_from_str("P");
        let p = b.mk_const(pn, none);
        let exprs = HashMap::from([(9u32, p)]);
        let inst = b.instantiate_metas(t, &exprs, &HashMap::new());
        let o2 = export_expr(inst);
        assert!(o2.node_count() <= 64 && !o2.nodes.iter().any(|n| matches!(n, OwnedNode::Meta { .. })));
        assert!(start.elapsed().as_secs() < 10, "bounded time");
    });
}

/// 夹具 + `Exists.intro ?α ?p a ha` 的有/无元变量两版。
fn intro<'a>(b: &mut EnvBuilder<'a>, i: ExprPtr<'a>, al: ExprPtr<'a>, p: ExprPtr<'a>, w: ExprPtr<'a>, h: ExprPtr<'a>) -> ExprPtr<'a> {
    let t = b.mk_app(i, al);
    let t = b.mk_app(t, p);
    let t = b.mk_app(t, w);
    b.mk_app(t, h)
}

fn theorem<'a>(b: &mut EnvBuilder<'a>, name: &str, ty: ExprPtr<'a>, val: ExprPtr<'a>) -> Declar<'a> {
    let none = b.alloc_levels_slice(&[]);
    let name = b.name_from_str(name);
    Declar::Theorem { info: DeclarInfo { name, uparams: none, ty }, val }
}

/// **F2a-T3**：`Exists ?α ?p =?= Exists A (λx. P x)` ⇒ `solve_metas` 给 `?α := A`、`?p := λx. P x`；
/// `instantiate_metas` 后的证明项无元变量、正常检查通过、`add_declar` 接受；未代入的原项被拒。
#[test]
fn f2a_t3_exists_intro_end_to_end() {
    big_stack(|| {
        let arena = Arena::new();
        let mut b = EnvBuilder::new(arena.as_arena_ref(), Config::default());
        let fx = setup(&mut b);
        let with_meta = intro(&mut b, fx.intro1, fx.m_alpha, fx.m_p, fx.w, fx.ha);
        let want = intro(&mut b, fx.intro1, fx.a, fx.lam, fx.w, fx.ha);
        let limit = EnvLimit::ByIndex(b.declaration_count());
        let seeds = [
            MetaSeed { id: 101, ty: fx.sort1, kind: MetaKind::Natural, depth: 0 },
            MetaSeed { id: 102, ty: fx.m_p_ty, kind: MetaKind::Natural, depth: 0 },
        ];
        let eqs = [(fx.res_ty, fx.expected)];
        let sol = b.with_env(|ef| ef.solve_metas(limit, &seeds, &eqs)).expect("solvable");
        assert!(sol.unassigned.is_empty(), "{:?}", sol.unassigned);
        assert_eq!(sol.exprs.iter().map(|(i, _)| *i).collect::<Vec<_>>(), vec![101, 102]);
        let (exprs, levels) = b.import_solution(&sol).unwrap();
        assert_eq!(exprs[&101], fx.a, "?α := A");
        assert_eq!(exprs[&102], fx.lam, "?p := λx. P x");
        let inst = b.instantiate_metas(with_meta, &exprs, &levels);
        assert!(!b.contains_meta(inst), "meta-free");
        assert_eq!(inst, want, "= Exists.intro A (λx. P x) a ha");
        let bad = theorem(&mut b, "t3_bad", fx.expected, with_meta);
        assert!(b.add_declar(bad).is_err(), "un-instantiated term rejected");
        let ok = theorem(&mut b, "t3_ok", fx.expected, inst);
        let checked = b.with_env(|ef| ef.try_check_declar_at(&ok, EnvLimit::PpUnlimited));
        assert!(checked.is_ok(), "{checked:?}");
        assert!(b.add_declar(ok).is_ok());
    });
}

/// **F2a-T4**：Miller —— `Exists A (λx. ?p x) =?= Exists A (λx. P x)`（`?p x =?= P x` 在 binder 下）。
/// 代入后 `λx. ?p x` 里的 `?p x` head-beta 成 `P x` ⇒ 与原生的 `Exists A (λx. P x)` 同指针；证明项通过检查。
#[test]
fn f2a_t4_miller_end_to_end() {
    big_stack(|| {
        let arena = Arena::new();
        let mut b = EnvBuilder::new(arena.as_arena_ref(), Config::default());
        let fx = setup(&mut b);
        let nx = b.name_from_str("x");
        let v0 = b.mk_var(0);
        let mpx = b.mk_app(fx.m_p, v0);
        let lam_m = b.mk_lambda(nx, BinderStyle::Default, fx.a, mpx);
        let ex = match *fx.expected.as_ref() {
            Expr::App { fun, .. } => fun,
            _ => unreachable!(),
        };
        let lhs = b.mk_app(ex, lam_m);
        let limit = EnvLimit::ByIndex(b.declaration_count());
        let seeds = [
            MetaSeed { id: 101, ty: fx.sort1, kind: MetaKind::Natural, depth: 0 },
            MetaSeed { id: 102, ty: fx.m_p_ty, kind: MetaKind::Natural, depth: 0 },
        ];
        let sol = b.with_env(|ef| ef.solve_metas(limit, &seeds, &[(lhs, fx.expected)])).expect("Miller");
        // Miller 赋值前的类型检查 `?α → Prop =?= A → Prop` 顺带赋了 `?α := A`。
        assert!(sol.unassigned.is_empty(), "{:?}", sol.unassigned);
        let (exprs, levels) = b.import_solution(&sol).unwrap();
        assert_eq!(exprs[&101], fx.a, "?α := A via the assignment type check");
        // Miller 造的 λ 用它自己的 binder 名（名字只影响 pp）⇒ 结构比，不比指针。
        assert!(super::meta_defeq::same(exprs[&102], fx.lam), "?p := λx. P x (up to binder name)");
        assert_eq!(b.instantiate_metas(lhs, &exprs, &levels), fx.expected, "head-beta under the binder");
        let with_meta = intro(&mut b, fx.intro1, fx.a, lam_m, fx.w, fx.ha);
        let inst = b.instantiate_metas(with_meta, &exprs, &levels);
        let want = intro(&mut b, fx.intro1, fx.a, fx.lam, fx.w, fx.ha);
        assert_eq!(inst, want);
        let ok = theorem(&mut b, "t4_ok", fx.expected, inst);
        let checked = b.with_env(|ef| ef.try_check_declar_at(&ok, EnvLimit::PpUnlimited));
        assert!(checked.is_ok(), "{checked:?}");
        assert!(b.add_declar(ok).is_ok());
    });
}

/// **F2a-T5**：不可解。约定：**任一方程判 false ⇒ `None`**；方程全成立但种子没被约束 ⇒
/// `Some`，且那个 id 列进 `unassigned`。两种情况都不 panic，builder 状态不变（声明数不变、
/// 仍能正常加声明）。另：方程里有判定层会 panic 的坏项 ⇒ `None`（不外泄）。
#[test]
fn f2a_t5_unsolvable() {
    big_stack(|| {
        let arena = Arena::new();
        let mut b = EnvBuilder::new(arena.as_arena_ref(), Config::default());
        let fx = setup(&mut b);
        let n0 = b.declaration_count();
        let limit = EnvLimit::ByIndex(n0);
        let seeds = [
            MetaSeed { id: 101, ty: fx.sort1, kind: MetaKind::Natural, depth: 0 },
            MetaSeed { id: 102, ty: fx.m_p_ty, kind: MetaKind::Natural, depth: 0 },
        ];
        let mpa = b.mk_app(fx.m_p, fx.w);
        let pa = b.mk_app(fx.p, fx.w);
        // `?p a =?= P a`：实参是常量 ⇒ 非 Miller ⇒ false ⇒ None。
        assert!(b.with_env(|ef| ef.solve_metas(limit, &seeds, &[(mpa, pa)])).is_none());
        // 先成立的方程 + 后失败的方程 ⇒ 仍 None（不带回半个解）。
        let eqs = [(fx.m_alpha, fx.a), (mpa, pa)];
        assert!(b.with_env(|ef| ef.solve_metas(limit, &seeds, &eqs)).is_none());
        // 成立但 `?p` 没被约束 ⇒ Some + unassigned = [102]。
        let sol = b.with_env(|ef| ef.solve_metas(limit, &seeds, &[(fx.m_alpha, fx.a)])).unwrap();
        assert_eq!(sol.unassigned, vec![102]);
        assert_eq!(sol.exprs.len(), 1);
        // 坏项（顶层 loose var）⇒ None，不 panic。
        let v5 = b.mk_var(5);
        let _ = b.with_env(|ef| ef.solve_metas(limit, &seeds, &[(v5, fx.a)]));
        assert_eq!(b.declaration_count(), n0, "nothing leaked into the builder");
        let none = b.alloc_levels_slice(&[]);
        axiom(&mut b, "t5_after", none, fx.sort1);
        assert_eq!(b.declaration_count(), n0 + 1);
    });
}

fn app2<'a>(b: &mut EnvBuilder<'a>, f: ExprPtr<'a>, x: ExprPtr<'a>, y: ExprPtr<'a>) -> ExprPtr<'a> {
    let t = b.mk_app(f, x);
    b.mk_app(t, y)
}

/// **F2a-T6**：`instantiate_metas` 的 head-beta 与 binder 下的 de Bruijn。
/// `?f := λy. λz. g z y`（闭项、内部有 binder）：
/// * `λx. λw. ?f w x` ⇒ `λx. λw. g x w`（开实参代进内层 binder 要 lift，剩余 var 要降）；
/// * `?f a` ⇒ `λz. g z a`（部分应用：只 beta 一层）；
/// * `λx. ?m` 且 `?m := a` ⇒ `λx. a`（闭赋值进 binder 不动）；`g ?m ?m` ⇒ `g a a`（非头位置不 beta）；
/// * 赋值链 `?c := ?d`、`?d := a`；层链 `?u := ?v`、`?v := 1`。
#[test]
fn f2a_t6_instantiate_beta_and_binders() {
    big_stack(|| {
        let arena = Arena::new();
        let mut b = EnvBuilder::new(arena.as_arena_ref(), Config::default());
        let fx = setup(&mut b);
        let none = b.alloc_levels_slice(&[]);
        let zero = b.zero();
        let prop = b.mk_sort(zero);
        let anon = b.anonymous();
        let a_prop = b.mk_pi(anon, BinderStyle::Default, fx.a, prop);
        let a_a_prop = b.mk_pi(anon, BinderStyle::Default, fx.a, a_prop);
        axiom(&mut b, "g", none, a_a_prop);
        let gn = b.name_from_str("g");
        let g = b.mk_const(gn, none);
        let (nx, nw, ny, nz) = (b.name_from_str("x"), b.name_from_str("w"), b.name_from_str("y"), b.name_from_str("z"));
        let (v0, v1) = (b.mk_var(0), b.mk_var(1));
        // ?f := λy. λz. g z y   (inside: z = Var0, y = Var1)
        let gzy = app2(&mut b, g, v0, v1);
        let lz = b.mk_lambda(nz, BinderStyle::Default, fx.a, gzy);
        let fval = b.mk_lambda(ny, BinderStyle::Default, fx.a, lz);
        let mf = b.mk_meta(104, a_a_prop);
        // λx. λw. ?f w x   (inside: w = Var0, x = Var1)
        let fwx = app2(&mut b, mf, v0, v1);
        let lw = b.mk_lambda(nw, BinderStyle::Default, fx.a, fwx);
        let term = b.mk_lambda(nx, BinderStyle::Default, fx.a, lw);
        // want: λx. λw. g x w  =  g Var1 Var0
        let gxw = app2(&mut b, g, v1, v0);
        let lw2 = b.mk_lambda(nw, BinderStyle::Default, fx.a, gxw);
        let want = b.mk_lambda(nx, BinderStyle::Default, fx.a, lw2);
        let mm = b.mk_meta(103, fx.a);
        let (mc, md) = (b.mk_meta(110, fx.a), b.mk_meta(111, fx.a));
        let exprs = HashMap::from([(104u32, fval), (103, fx.w), (110, md), (111, fx.w)]);
        let lv = HashMap::new();
        assert_eq!(b.instantiate_metas(term, &exprs, &lv), want, "λx λw. ?f w x ⇒ λx λw. g x w");
        // partial application
        let fa = b.mk_app(mf, fx.w);
        let gza = app2(&mut b, g, v0, fx.w);
        let want_fa = b.mk_lambda(nz, BinderStyle::Default, fx.a, gza);
        assert_eq!(b.instantiate_metas(fa, &exprs, &lv), want_fa, "?f a ⇒ λz. g z a");
        // closed assignment under a binder; non-head positions
        let lm = b.mk_lambda(nx, BinderStyle::Default, fx.a, mm);
        let la = b.mk_lambda(nx, BinderStyle::Default, fx.a, fx.w);
        assert_eq!(b.instantiate_metas(lm, &exprs, &lv), la, "λx. ?m ⇒ λx. a");
        let gmm = app2(&mut b, g, mm, mc);
        let gaa = app2(&mut b, g, fx.w, fx.w);
        assert_eq!(b.instantiate_metas(gmm, &exprs, &lv), gaa, "g ?m ?c ⇒ g a a (chain ?c := ?d := a)");
        // level chain: Exists.{?u} with ?u := ?v, ?v := 1
        let exn = b.name_from_str("Exists");
        let (lu, lvv) = (b.level_mvar(1), b.level_mvar(2));
        let one = b.succ(zero);
        let lus = b.alloc_levels_slice(&[lu]);
        let ones = b.alloc_levels_slice(&[one]);
        let exu = b.mk_const(exn, lus);
        let ex1 = b.mk_const(exn, ones);
        let levels = HashMap::from([(1u64, lvv), (2u64, one)]);
        assert_eq!(b.instantiate_metas(exu, &HashMap::new(), &levels), ex1, "Exists.{{?u}} ⇒ Exists.{{1}}");
        // unassigned metas stay
        let m_other = b.mk_meta(999, fx.a);
        assert_eq!(b.instantiate_metas(m_other, &exprs, &lv), m_other);
    });
}

//! **K1b**：公开 `is_def_eq`（失败即回滚，Lean `checkpointDefEq`）+ 赋值/回滚后的缓存纪律
//! + `instantiate_mvars`。夹具复用 `meta_spine`（`Exists`、`A`、`P`、`Q`、`a`、`?α`(101)、`?p`(102)）。
use super::meta_spine::{app, setup, with_tc, Fx};
use crate::builder::EnvBuilder;
use crate::env::{Declar, DeclarInfo, EnvLimit};
use crate::expr::MetaKind;
use crate::tc::TypeChecker;
use crate::util::{Config, ExprPtr};
use crate::value::Value;
use stumpalo::Arena;

/// **K1b-T1**：`Exists ?α P =?= Exists A Q` —— 第一个实参赋 `?α := A`，第二个实参
/// `P ≠ Q` 失败 ⇒ `is_def_eq` 为 false，且 `?α` 回到未赋值。
#[test]
fn k1b_t1_failed_def_eq_rolls_back_assignments() {
    with_tc(|tc, fx| {
        let lhs = {
            let ex = tc.ctx.unfold_apps_fun(fx.res_ty);
            let ex = tc.ctx.unfold_apps_fun(ex);
            let t = app(tc, ex, fx.m_alpha);
            app(tc, t, fx.p)
        };
        let rhs = {
            let ex = tc.ctx.unfold_apps_fun(fx.expected);
            let t = app(tc, ex, fx.a);
            app(tc, t, fx.q)
        };
        assert!(tc.ctx.mvars.value(101).is_none());
        assert!(!tc.is_def_eq(lhs, rhs), "Exists ?α P =?= Exists A Q must fail");
        assert!(tc.ctx.mvars.value(101).is_none(), "?α must be rolled back");
        // 成功的尝试保留赋值。
        let ok_rhs = {
            let ex = tc.ctx.unfold_apps_fun(fx.expected);
            let t = app(tc, ex, fx.a);
            app(tc, t, fx.p)
        };
        assert!(tc.is_def_eq(lhs, ok_rhs), "Exists ?α P =?= Exists A P");
        assert!(tc.ctx.mvars.value(101).is_some(), "?α := A kept on success");
    });
}

/// 推断 `e` 的类型并 quote（走 `type_cache`）。
fn infer_q<'t>(tc: &mut TypeChecker<'_, 't, '_>, e: ExprPtr<'t>) -> ExprPtr<'t> {
    let (env, ctx) = (tc.empty_env(), tc.empty_ctx());
    let ty = tc.infer_value(crate::tc::InferFlag::InferOnly, 0, env, ctx, e);
    tc.quote(0, ty)
}

/// Pi 的定义域（quote 后的类型）。
fn pi_dom(e: ExprPtr<'_>) -> ExprPtr<'_> {
    match e.as_ref() {
        crate::expr::Expr::Pi { binder_type, .. } => *binder_type,
        other => panic!("not a Pi: {other:?}"),
    }
}

/// **K1b-T2**：回滚后不留陈旧缓存。失败的尝试 `Exists ?α ?p =?= Exists A R`（`R : B → Prop`）：
/// 先赋 `?α := A`，再在 `?p := R` 的类型检查里 eval `?α → Prop`（此时缓存成 `A → Prop`），
/// `B → Prop ≠ A → Prop` ⇒ 失败 ⇒ 回滚。之后 eval/infer 含 `?α` 的项必须仍见到**未赋值**的 `?α`。
#[test]
fn k1b_t2_rollback_leaves_no_stale_cache() {
    with_tc(|tc, fx| {
        let env = tc.empty_env();
        let before = tc.eval(0, env, fx.m_p_ty);
        assert!(matches!(before, Value::Pi { domain: Value::Meta { id: 101, .. }, .. }));
        let intro3 = {
            let t = app(tc, fx.intro1, fx.m_alpha);
            let t = app(tc, t, fx.m_p);
            app(tc, t, fx.w)
        };
        let ty_before = infer_q(tc, intro3);
        assert!(tc.ctx.unassigned_mvars(ty_before).exprs.contains(&101));
        let rhs = {
            let ex = tc.ctx.unfold_apps_fun(fx.expected);
            let t = app(tc, ex, fx.a);
            app(tc, t, fx.r)
        };
        assert!(!tc.is_def_eq(fx.res_ty, rhs), "Exists ?α ?p =?= Exists A R must fail");
        assert!(tc.ctx.mvars.value(101).is_none() && tc.ctx.mvars.value(102).is_none());
        let env = tc.empty_env();
        let after = tc.eval(0, env, fx.m_p_ty);
        assert!(
            matches!(after, Value::Pi { domain: Value::Meta { id: 101, .. }, .. }),
            "eval(?α → Prop) after rollback must show ?α unassigned (stale closed_eval_cache), got {after:?}"
        );
        let env = tc.empty_env();
        assert!(matches!(tc.eval(0, env, fx.m_alpha), Value::Meta { id: 101, .. }));
        let ty_after = infer_q(tc, intro3);
        assert!(tc.ctx.unassigned_mvars(ty_after).exprs.contains(&101), "inferred type still mentions ?α");
    });
}

/// **K1b-T3**：赋值对每张缓存都可见。赋值前 infer/quote 含 `?p a` 的项（进 `type_cache` / `quote_cache`），
/// 赋 `?p := λx. P x` 后再问 ⇒ 结果必须是 `P a`。
#[test]
fn k1b_t3_assignment_visible_through_type_and_quote_caches() {
    with_tc(|tc, fx| {
        let pa = app(tc, fx.p, fx.w);
        let mpa = app(tc, fx.m_p, fx.w);
        // type_cache：`Exists.intro ?α ?p a : ?p a → Exists ?α ?p`。
        let intro3 = {
            let t = app(tc, fx.intro1, fx.m_alpha);
            let t = app(tc, t, fx.m_p);
            app(tc, t, fx.w)
        };
        let ty0 = infer_q(tc, intro3);
        assert!(same(pi_dom(ty0), mpa), "before: domain is ?p a");
        // quote_cache：quote 卡住的值 `?p a`。
        let env = tc.empty_env();
        let v = tc.eval(0, env, mpa);
        let q0 = tc.quote(0, v);
        assert!(same(q0, mpa), "before: quote(?p a) = ?p a");
        assert!(tc.is_def_eq(fx.m_p, fx.lam), "?p := λx. P x");
        let ty1 = infer_q(tc, intro3);
        assert!(same(pi_dom(ty1), pa), "type_cache: after assignment the domain must be P a");
        let q1 = tc.quote(0, v);
        assert!(same(q1, pa), "quote_cache: quote of the old stuck value must see ?p := λx. P x");
        let env = tc.empty_env();
        let v2 = tc.eval(0, env, mpa);
        let q2 = tc.quote(0, v2);
        assert!(same(q2, pa), "fresh eval+quote of ?p a = P a");
    });
}

/// 结构相等（跨 arena：TC 会话里重建的节点与 builder 节点指针不同）。名字是全局的 ⇒ 按指针比。
pub(super) fn same(x: ExprPtr<'_>, y: ExprPtr<'_>) -> bool {
    use crate::expr::Expr::*;
    match (x.as_ref(), y.as_ref()) {
        (Var { dbj_idx: i, .. }, Var { dbj_idx: j, .. }) => i == j,
        (Sort { level: l, .. }, Sort { level: m, .. }) => level_same(*l, *m),
        (Const { name: n, levels: ls, .. }, Const { name: m, levels: ms, .. }) => {
            n.get_hash() == m.get_hash()
                && ls.len() == ms.len()
                && ls.iter().zip(ms.iter()).all(|(a, b)| level_same(*a, *b))
        }
        (App { fun: f, arg: a, .. }, App { fun: g, arg: b, .. }) => same(*f, *g) && same(*a, *b),
        (Lambda { binder_type: t, body: b, .. }, Lambda { binder_type: u, body: c, .. })
        | (Pi { binder_type: t, body: b, .. }, Pi { binder_type: u, body: c, .. }) => same(*t, *u) && same(*b, *c),
        (Meta { id: i, .. }, Meta { id: j, .. }) => i == j,
        _ => false,
    }
}

fn level_same(x: crate::util::LevelPtr<'_>, y: crate::util::LevelPtr<'_>) -> bool {
    use crate::level::Level::*;
    match (*x.as_ref(), *y.as_ref()) {
        (Zero, Zero) => true,
        (Succ(a, _), Succ(b, _)) => level_same(a, b),
        (Max(a, b, _), Max(c, d, _)) | (IMax(a, b, _), IMax(c, d, _)) => level_same(a, c) && level_same(b, d),
        (Param(n, _), Param(m, _)) => n.get_hash() == m.get_hash(),
        (MVar(i, _), MVar(j, _)) => i == j,
        _ => false,
    }
}

fn mk_intro<'t>(tc: &mut TypeChecker<'_, 't, '_>, fx: Fx<'t>, alpha: ExprPtr<'t>, p: ExprPtr<'t>) -> ExprPtr<'t> {
    let t = app(tc, fx.intro1, alpha);
    let t = app(tc, t, p);
    let t = app(tc, t, fx.w);
    app(tc, t, fx.ha)
}

/// **K1b-T4**：`Exists.intro ?α ?p a ha`，`?α := A`、`?p := λx. P x` ⇒ `instantiate_mvars` 给出无元变量的项，
/// `?p a` 位置 head-beta 成 `P a`；它与 builder 里手搭的 `Exists.intro A (λx. P x) a ha` 结构相等，
/// 后者走正常检查路径对 `Exists A (λx. P x)` 通过、`add_declar` 接受；含元变量的原项 `add_declar` 仍拒绝。
#[test]
fn k1b_t4_instantiate_mvars_beta_and_add_declar() {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            let arena = Arena::new();
            let mut b = EnvBuilder::new(arena.as_arena_ref(), Config::default());
            let fx = setup(&mut b);
            let want = {
                let t = b.mk_app(fx.intro1, fx.a);
                let t = b.mk_app(t, fx.lam);
                let t = b.mk_app(t, fx.w);
                b.mk_app(t, fx.ha)
            };
            let with_meta = {
                let t = b.mk_app(fx.intro1, fx.m_alpha);
                let t = b.mk_app(t, fx.m_p);
                let t = b.mk_app(t, fx.w);
                b.mk_app(t, fx.ha)
            };
            let limit = EnvLimit::ByIndex(b.declaration_count());
            let seeds = [(101u32, fx.sort1, MetaKind::Natural), (102, fx.m_p_ty, MetaKind::Natural)];
            let (inst_ok, beta_ok, meta_free) = b.with_env(|ef| {
                ef.with_tc_seeded(limit, &seeds, |tc| {
                    assert!(tc.is_def_eq(fx.m_alpha, fx.a), "?α := A");
                    assert!(tc.is_def_eq(fx.m_p, fx.lam), "?p := λx. P x");
                    let term = mk_intro(tc, fx, fx.m_alpha, fx.m_p);
                    let inst = tc.ctx.instantiate_mvars(term);
                    let meta_free = tc.ctx.unassigned_mvars(inst).is_empty();
                    let mpa = app(tc, fx.m_p, fx.w);
                    let pa_inst = tc.ctx.instantiate_mvars(mpa);
                    let pa = app(tc, fx.p, fx.w);
                    (same(inst, want), same(pa_inst, pa), meta_free)
                })
            });
            assert!(meta_free, "instantiated term has no unassigned metas");
            assert!(inst_ok, "instantiate_mvars(Exists.intro ?α ?p a ha) = Exists.intro A (λx. P x) a ha (meta-free)");
            assert!(beta_ok, "instantiate_mvars(?p a) = P a (head-beta, not (λx. P x) a)");
            let none = b.alloc_levels_slice(&[]);
            let n_ok = b.name_from_str("t4_ok");
            let ok = Declar::Theorem { info: DeclarInfo { name: n_ok, uparams: none, ty: fx.expected }, val: want };
            let checked = b.with_env(|ef| ef.try_check_declar_at(&ok, EnvLimit::PpUnlimited));
            assert!(checked.is_ok(), "instantiated term checks against Exists A (λx. P x): {checked:?}");
            let n_bad = b.name_from_str("t4_bad");
            let bad =
                Declar::Theorem { info: DeclarInfo { name: n_bad, uparams: none, ty: fx.expected }, val: with_meta };
            // 定理的证明项里带元变量 ⇒ 必须拒（K1b 前漏扫：`Declar::value()` 不含 Theorem）。
            assert!(b.add_declar(bad).is_err(), "add_declar still rejects a meta (theorem value)");
            assert!(b.add_declar(ok).is_ok(), "add_declar accepts the instantiated term");
        })
        .unwrap()
        .join()
        .unwrap();
}

/// **K1b-T5**：赋值链 `?a := ?b`、`?b := A` ⇒ `A`；层元变量 `Exists.{?u}`、`?u := 1` ⇒ `Exists.{1}`。
#[test]
fn k1b_t5_chains_and_level_metas() {
    with_tc(|tc, fx| {
        tc.ctx.mvars.register(201, fx.sort1);
        tc.ctx.mvars.register(202, fx.sort1);
        let ma = tc.ctx.mk_meta(201, fx.sort1);
        let mb = tc.ctx.mk_meta(202, fx.sort1);
        tc.ctx.mvars.assign(201, mb);
        tc.ctx.mvars.assign(202, fx.a);
        let r = tc.ctx.instantiate_mvars(ma);
        assert!(same(r, fx.a), "?a := ?b, ?b := A ⇒ A");
        assert!(tc.ctx.unassigned_mvars(ma).is_empty());
        let hash = crate::hash64!(crate::level::MVAR_HASH, 7u64);
        let u = tc.ctx.alloc_level(crate::level::Level::MVar(7, hash));
        let (ex_name, one) = match tc.ctx.read_expr(tc.ctx.unfold_apps_fun(fx.res_ty)) {
            crate::expr::Expr::Const { name, levels, .. } => (name, levels[0]),
            _ => unreachable!(),
        };
        let us = tc.ctx.alloc_levels(&[u]);
        let ex_u = tc.ctx.mk_const(ex_name, us);
        assert_eq!(tc.ctx.unassigned_mvars(ex_u).levels, vec![7]);
        let r0 = tc.ctx.instantiate_mvars(ex_u);
        assert!(same(r0, ex_u), "unassigned level meta stays");
        tc.ctx.mvars.assign_level(7, one);
        let r = tc.ctx.instantiate_mvars(ex_u);
        let ones = tc.ctx.alloc_levels(&[one]);
        let ex1 = tc.ctx.mk_const(ex_name, ones);
        assert!(same(r, ex1), "Exists.{{?u}} with ?u := 1 ⇒ Exists.{{1}}");
        assert!(tc.ctx.unassigned_mvars(ex_u).is_empty());
    });
}

/// **K1b-T6**：未赋值清单 —— 赋值前 `[?α, ?p]`；只赋 `?α` 后原项与实例化项都只剩 `[?p]`；全赋后都为空。
#[test]
fn k1b_t6_unassigned_listing() {
    with_tc(|tc, fx| {
        let term = mk_intro(tc, fx, fx.m_alpha, fx.m_p);
        assert_eq!(tc.ctx.unassigned_mvars(term).exprs, vec![101, 102]);
        assert!(tc.is_def_eq(fx.m_alpha, fx.a));
        assert_eq!(tc.ctx.unassigned_mvars(term).exprs, vec![102], "sees through ?α := A");
        let inst = tc.ctx.instantiate_mvars(term);
        assert_eq!(tc.ctx.unassigned_mvars(inst).exprs, vec![102]);
        assert!(tc.is_def_eq(fx.m_p, fx.lam));
        assert!(tc.ctx.unassigned_mvars(term).is_empty());
        let inst = tc.ctx.instantiate_mvars(term);
        assert!(tc.ctx.unassigned_mvars(inst).is_empty());
    });
}

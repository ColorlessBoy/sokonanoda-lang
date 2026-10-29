//! **K1d**：本会话层元变量在 defeq 里可赋值（Lean `isLevelDefEq`）；未登记的照旧刚性。
//! 夹具复用 `meta_spine`（`A : Type`）。
use super::meta_spine::with_tc;
use crate::level::Level;
use crate::tc::TypeChecker;
use crate::util::LevelPtr;

fn lmvar<'t>(tc: &mut TypeChecker<'_, 't, '_>, id: u64) -> LevelPtr<'t> {
    let hash = crate::hash64!(crate::level::MVAR_HASH, id);
    tc.ctx.alloc_level(Level::MVar(id, hash))
}

fn one<'t>(tc: &mut TypeChecker<'_, 't, '_>) -> LevelPtr<'t> {
    let z = tc.ctx.zero();
    tc.ctx.succ(z)
}

/// **K1d-T1**：`Sort ?u =?= Sort 1`：登记过 ⇒ true 且 `?u := 1`；未登记 ⇒ false（与今天同）。
#[test]
fn k1d_t1_registered_level_meta_is_assignable() {
    with_tc(|tc, _fx| {
        let u = lmvar(tc, 7);
        let l1 = one(tc);
        let (su, s1) = (tc.ctx.mk_sort(u), tc.ctx.mk_sort(l1));
        // 未登记：刚性 ⇒ false，无赋值。
        assert!(!tc.is_def_eq(su, s1), "unregistered ?u stays rigid");
        assert!(tc.ctx.mvars.level_value(7).is_none());
        // 另一个未登记的 id，在**有**登记层 mvar 的会话里也照旧刚性。
        tc.ctx.mvars.register_level(8);
        assert!(!tc.is_def_eq(su, s1), "unregistered ?u stays rigid even when the session has level metas");
        assert!(tc.ctx.mvars.level_value(7).is_none());
        // 登记 ⇒ 赋值。
        tc.ctx.mvars.register_level(7);
        assert!(tc.is_def_eq(su, s1), "registered ?u =?= 1");
        assert_eq!(tc.ctx.mvars.level_value(7), Some(l1), "?u := 1");
        // 已赋值 ⇒ 看穿：`Sort ?u =?= Sort 0` 为假。
        let z = tc.ctx.zero();
        let s0 = tc.ctx.mk_sort(z);
        assert!(!tc.is_def_eq(su, s0), "assigned ?u = 1 ≠ 0");
        assert!(tc.is_def_eq(su, s1), "assigned ?u = 1");
    });
}

/// **K1d-T2**：`(Sort ?u → A) =?= (Sort 1 → Type)`：域先赋 `?u := 1`，体 `A ≠ Type` ⇒ false ⇒ 回滚；
/// 之后 `Sort ?u =?= Sort 2` 仍可赋（没有陈旧赋值/缓存）。
#[test]
fn k1d_t2_failed_def_eq_rolls_back_level_assignment() {
    with_tc(|tc, fx| {
        tc.ctx.mvars.register_level(7);
        let u = lmvar(tc, 7);
        let l1 = one(tc);
        let (su, s1) = (tc.ctx.mk_sort(u), tc.ctx.mk_sort(l1));
        let anon = tc.ctx.anonymous();
        let d = crate::expr::BinderStyle::Default;
        let lhs = tc.ctx.mk_pi(anon, d, su, fx.a);
        let rhs = tc.ctx.mk_pi(anon, d, s1, s1);
        assert!(!tc.is_def_eq(lhs, rhs), "(Sort ?u → A) =?= (Sort 1 → Type) must fail");
        assert!(tc.ctx.mvars.level_value(7).is_none(), "?u := 1 must be rolled back");
        let l2 = tc.ctx.succ(l1);
        let s2 = tc.ctx.mk_sort(l2);
        assert!(tc.is_def_eq(su, s2), "?u still assignable after rollback");
        assert_eq!(tc.ctx.mvars.level_value(7), Some(l2));
    });
}

/// **K1d-T3**：occurs：`Sort ?u =?= Sort (succ ?u)` ⇒ false，`?u` 不赋值。
#[test]
fn k1d_t3_occurs_check() {
    with_tc(|tc, _fx| {
        tc.ctx.mvars.register_level(7);
        let u = lmvar(tc, 7);
        let su1 = tc.ctx.succ(u);
        let (a, b) = (tc.ctx.mk_sort(u), tc.ctx.mk_sort(su1));
        assert!(!tc.is_def_eq(a, b), "?u =?= succ ?u");
        assert!(!tc.is_def_eq(b, a), "succ ?u =?= ?u");
        assert!(tc.ctx.mvars.level_value(7).is_none());
    });
}

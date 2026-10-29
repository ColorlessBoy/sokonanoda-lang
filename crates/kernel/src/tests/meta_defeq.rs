//! **K1b**：公开 `is_def_eq`（失败即回滚，Lean `checkpointDefEq`）+ 赋值/回滚后的缓存纪律
//! + `instantiate_mvars`。夹具复用 `meta_spine`（`Exists`、`A`、`P`、`Q`、`a`、`?α`(101)、`?p`(102)）。
use super::meta_spine::{app, with_tc};

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

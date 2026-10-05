use crate::tests::util::test_ctx;
use rand::prelude::*;
use std::error::Error;

#[test]
fn leq_test0() -> Result<(), Box<dyn Error>> {
    test_ctx(None, |ctx| {
        let z = ctx.zero();
        let s = ctx.succ(z);
        let m = ctx.max(s, s);
        assert!(ctx.leq(s, m));
        assert!(ctx.leq(m, s));
        assert!(ctx.eq_antisymm(s, m));
    })
}

#[test]
fn leq_test1() -> Result<(), Box<dyn Error>> {
    test_ctx(None, |ctx| {
        let z = ctx.zero();
        let s = ctx.succ(z);
        let ss = ctx.succ(s);
        let im = ctx.imax(ss, z);
        assert!(ctx.leq(im, z));
        assert!(ctx.eq_antisymm(z, im));
    })
}

#[test]
fn leq_test2() -> Result<(), Box<dyn Error>> {
    test_ctx(None, |ctx| {
        let a = ctx.param_quick("a");
        let b = ctx.param_quick("b");
        assert!(!ctx.leq(a, b));
        assert!(!ctx.leq(b, a));
    })
}

#[test]
fn leq_test_imax_imax() -> Result<(), Box<dyn Error>> {
    test_ctx(None, |ctx| {
        let a = ctx.param_quick("a");
        let b = ctx.param_quick("b");
        let imax_a_b = ctx.imax(a, b);
        let s_imax_a_b = ctx.succ(imax_a_b);
        let ss_imax_a_b = ctx.succ(s_imax_a_b);
        assert!(ctx.leq(imax_a_b, imax_a_b));
        assert!(ctx.leq(imax_a_b, s_imax_a_b));
        assert!(ctx.leq(imax_a_b, ss_imax_a_b));
        assert!(ctx.leq(s_imax_a_b, ss_imax_a_b));
        assert!(!ctx.leq(ss_imax_a_b, imax_a_b));
        assert!(!ctx.leq(ss_imax_a_b, s_imax_a_b));
    })
}

#[test]
fn leq_test3() -> Result<(), Box<dyn Error>> {
    test_ctx(None, |ctx| {
        let a = ctx.param_quick("a");
        let b = ctx.param_quick("b");
        assert!(!ctx.leq(a, b));
        assert!(!ctx.leq(b, a));
    })
}

#[test]
fn leq_test4() -> Result<(), Box<dyn Error>> {
    test_ctx(None, |ctx| {
        for _ in 0..100 {
            let mut rng = thread_rng();
            let (small, large) = {
                let (x, y): (u8, u8) = rng.gen();
                (x.min(y), x.max(y))
            };

            let p = ctx.param_quick("p");
            let (a, b) = (ctx.level_n(p, small as u64), ctx.level_n(p, large as u64));
            assert!(ctx.leq(a, b));
        }
    })
}

#[test]
fn leq_test5() -> Result<(), Box<dyn Error>> {
    test_ctx(None, |ctx| {
        let (p, q) = (ctx.param_quick("p"), ctx.param_quick("q"));
        let mut rng = thread_rng();
        for _ in 0..100 {
            let (small, large) = {
                let (x, y): (u8, u8) = rng.gen();
                (x.min(y) as u64, x.max(y) as u64)
            };
            let lhs = {
                let (p_small, q_small) = (ctx.level_n(p, small), ctx.level_n(q, small));
                let lhs = ctx.max(p_small, q_small);
                ctx.level_n(lhs, small)
            };
            let rhs = {
                let (p_large, q_large) = (ctx.level_n(p, large), ctx.level_n(q, large));
                let rhs = ctx.max(p_large, q_large);
                ctx.level_n(rhs, large)
            };

            assert!(ctx.leq(lhs, rhs));
        }
    })
}

#[test]
fn leq_test6() -> Result<(), Box<dyn Error>> {
    test_ctx(None, |ctx| {
        let (p, q) = (ctx.param_quick("p"), ctx.param_quick("q"));
        let mut rng = thread_rng();
        for _ in 0..100 {
            let (small, large) = {
                let (x, y): (u8, u8) = rng.gen();
                (x.min(y) as u64, x.max(y) as u64)
            };
            let lhs = {
                let (p_small, q_small) = (ctx.level_n(p, small), ctx.level_n(q, small));
                let lhs = ctx.imax(p_small, q_small);
                ctx.level_n(lhs, small)
            };
            let rhs = {
                let (p_large, q_large) = (ctx.level_n(p, large), ctx.level_n(q, large));
                let rhs = ctx.imax(p_large, q_large);
                ctx.level_n(rhs, large)
            };

            assert!(ctx.leq(lhs, rhs));
        }
    })
}

#[test]
fn leq_test7() -> Result<(), Box<dyn Error>> {
    test_ctx(None, |ctx| {
        let (p, q) = (ctx.param_quick("p"), ctx.param_quick("q"));
        let mut rng = thread_rng();
        for _ in 0..100 {
            let (u, v, w) = {
                let (u, v, w): (u8, u8, u8) = rng.gen();
                (u as u64, v as u64, w as u64)
            };
            let lhs = {
                let (p_, q_) = (ctx.level_n(p, u), ctx.level_n(q, v + 1));
                let lhs = ctx.imax(p_, q_);
                ctx.level_n(lhs, w)
            };
            let rhs = {
                let (p_, q_) = (ctx.level_n(p, u), ctx.level_n(q, v + 1));
                let rhs = ctx.max(p_, q_);
                ctx.level_n(rhs, w)
            };

            assert!(ctx.eq_antisymm(lhs, rhs));
        }
    })
}

#[test]
fn eq_test1() -> Result<(), Box<dyn Error>> {
    test_ctx(None, |ctx| {
        let z = ctx.zero();
        let s = ctx.succ(z);
        let ss = ctx.succ(s);
        let m = ctx.max(s, s);
        let sm = ctx.succ(m);
        assert!(ctx.eq_antisymm(ss, sm));
    })
}

#[test]
fn eq_many_test1() -> Result<(), Box<dyn Error>> {
    // [2] == [max(1, 1) + 1]
    test_ctx(None, |ctx| {
        let z = ctx.zero();
        let s = ctx.succ(z);
        let ss = ctx.succ(s);
        let m = ctx.max(s, s);
        let sm = ctx.succ(m);
        let ups1 = ctx.alloc_levels(&[ss]);
        let ups2 = ctx.alloc_levels(&[sm]);
        assert!(ctx.eq_antisymm_many(ups1, ups2));
    })
}

#[test]
fn debug_test0() -> Result<(), Box<dyn Error>> {
    test_ctx(None, |ctx| {
        let z = ctx.zero();
        let s = ctx.succ(z);
        let ss = ctx.succ(s);
        let (z_, num) = ctx.level_succs(ss);
        assert_eq!(z_, z);
        assert_eq!(num, 2);
        assert_eq!("2", format!("{:?}", ctx.debug_print(ss)));
    })
}

#[test]
fn debug_test1() -> Result<(), Box<dyn Error>> {
    test_ctx(None, |ctx| {
        let z = ctx.zero();
        let s = ctx.succ(z);
        let m = ctx.max(s, s);
        let sm = ctx.succ(m);
        let (m_, num) = ctx.level_succs(sm);
        assert_eq!(m, m_);
        assert_eq!(num, 1);
        assert_eq!("max(1, 1) + 1", format!("{:?}", ctx.debug_print(sm)));
    })
}

// **R2a（2026-10-05）**：层合一的单测 ✓ —— 逐条对齐 Lean `Meta/LevelDefEq.lean:90-125` 的 `solve` ✓。
// ⚠ 全部**内联**（`test_ctx` 的 `'t` 不可命名 ⇒ 助手函数/闭包传不出 `LevelPtr` ✓）。
mod r2a_level_solve {
    use crate::level::{Level, LevelEq};
    use crate::tests::util::test_ctx;
    use std::error::Error;

    /// mvar vs 具体层 ⇒ **True** 且**记下赋值** ✓。
    #[test]
    fn assigns_mvar_to_concrete() -> Result<(), Box<dyn Error>> {
        test_ctx(None, |ctx| {
            let hash = crate::hash64!(crate::level::MVAR_HASH, 7u64);
            let m = ctx.alloc_level(Level::MVar(7, hash));
            let z = ctx.zero();
            let s = ctx.succ(z);
            let mut assign = Vec::new();
            assert_eq!(ctx.level_solve(m, s, &mut assign), LevelEq::True);
            assert_eq!(assign.len(), 1);
            assert_eq!(assign[0].0, 7);
        })
    }

    /// **`occurs` 闸** ✓：`u` 出现在自己的解里 ⇒ **Undef**（弃权）且**不赋值** ✓。
    #[test]
    fn occurs_check_abstains() -> Result<(), Box<dyn Error>> {
        test_ctx(None, |ctx| {
            let hash = crate::hash64!(crate::level::MVAR_HASH, 3u64);
            let m = ctx.alloc_level(Level::MVar(3, hash));
            let inside = ctx.succ(m);
            let mut assign = Vec::new();
            assert_eq!(ctx.level_solve(m, inside, &mut assign), LevelEq::Undef);
            assert!(assign.is_empty(), "occurs 命中时不许赋值");
        })
    }

    /// `zero` vs `succ` ⇒ **False**（确定不等 ✓）。
    #[test]
    fn zero_vs_succ_is_false() -> Result<(), Box<dyn Error>> {
        test_ctx(None, |ctx| {
            let z = ctx.zero();
            let s = ctx.succ(z);
            let mut assign = Vec::new();
            assert_eq!(ctx.level_solve(z, s, &mut assign), LevelEq::False);
        })
    }

    /// **右侧是 mvar ⇒ Undef** ✓（对齐 Lean：「Let `solve v u` to handle this case」✓）。
    #[test]
    fn right_mvar_abstains() -> Result<(), Box<dyn Error>> {
        test_ctx(None, |ctx| {
            let z = ctx.zero();
            let hash = crate::hash64!(crate::level::MVAR_HASH, 11u64);
            let m = ctx.alloc_level(Level::MVar(11, hash));
            let mut assign = Vec::new();
            assert_eq!(ctx.level_solve(z, m, &mut assign), LevelEq::Undef);
            assert!(assign.is_empty());
        })
    }

    /// param 之间：**同名 ⇒ True** ✓、**异名 ⇒ False** ✓。
    #[test]
    fn params_compare_by_name() -> Result<(), Box<dyn Error>> {
        test_ctx(None, |ctx| {
            let n1 = ctx.name_from_str("u");
            let n2 = ctx.name_from_str("u");
            let n3 = ctx.name_from_str("v");
            let h1 = crate::hash64!(crate::level::PARAM_HASH, n1);
            let h2 = crate::hash64!(crate::level::PARAM_HASH, n2);
            let h3 = crate::hash64!(crate::level::PARAM_HASH, n3);
            let a = ctx.alloc_level(Level::Param(n1, h1));
            let b = ctx.alloc_level(Level::Param(n2, h2));
            let c = ctx.alloc_level(Level::Param(n3, h3));
            let mut assign = Vec::new();
            assert_eq!(ctx.level_solve(a, b, &mut assign), LevelEq::True);
            assert_eq!(ctx.level_solve(a, c, &mut assign), LevelEq::False);
        })
    }
}

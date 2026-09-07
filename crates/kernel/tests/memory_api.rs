//! M0 acceptance: drive the complete kernel from memory without any export
//! file, config file or external tool.

use sokonanoda::builder::EnvBuilder;
use sokonanoda::env::{Declar, DeclarInfo, EnvLimit, ReducibilityHint};
use sokonanoda::expr::{BinderStyle, Expr};
use sokonanoda::util::{Config, ExportFile};
use stumpalo::Arena;

/// 回归测试（conv 快路径 soundness 修复）：
/// `(A : Sort 1) -> A` 不可居住，`fun (A : Sort 1) => A` 的类型是
/// `(A : Sort 1) -> Sort 1`，与声明类型的依赖 codomain `$0` 不同，
/// 内核必须拒绝。修复前 conv 的 Pi/Pi body-expr 快路径把"同一 interned
/// `Var 0` 体"的 eval 闭包与 infer 闭包误判为相等，导致该不可居住类型
/// 被接受（官方 Lean 拒绝）。
#[test]
fn dependent_codomain_is_not_inhabited_by_identity_lambda() {
    let arena = Arena::new();
    let mut b = EnvBuilder::new(arena.as_arena_ref(), Config::default());
    let one = b.succ(b.zero());
    let sort1 = b.mk_sort(one);
    let name = b.name_from_str("A");
    let var0 = b.mk_var(0);
    // declared: Pi (A : Sort 1), Var 0   （依赖 codomain，不可居住）
    let ty = b.mk_pi(name, BinderStyle::Default, sort1, var0);
    // val: Lambda (A : Sort 1), Var 0   （类型是 Pi (A : Sort 1), Sort 1）
    let val = b.mk_lambda(name, BinderStyle::Default, sort1, var0);
    let declar = Declar::Definition {
        info: DeclarInfo {
            name: b.name_from_str("t7"),
            uparams: b.alloc_levels_slice(&[]),
            ty,
        },
        val,
        hint: ReducibilityHint::Regular(0),
    };
    b.add_declar(declar.clone()).expect("add declar");
    let env = b.finish();
    let result = env.try_check_declar(&declar);
    assert!(
        result.is_err(),
        "the uninhabited dependent codomain must be rejected"
    );
}

/// 回归测试（对照）：非依赖的身份函数仍然通过——
/// `(A : Sort 1) -> Sort 1 := fun (A : Sort 1) => A` 合法。
#[test]
fn identity_over_sort_still_checks() {
    let arena = Arena::new();
    let mut b = EnvBuilder::new(arena.as_arena_ref(), Config::default());
    let one = b.succ(b.zero());
    let sort1 = b.mk_sort(one);
    let name = b.name_from_str("A");
    let var0 = b.mk_var(0);
    let codomain = b.mk_sort(one);
    let ty = b.mk_pi(name, BinderStyle::Default, sort1, codomain);
    let val = b.mk_lambda(name, BinderStyle::Default, sort1, var0);
    let declar = Declar::Definition {
        info: DeclarInfo {
            name: b.name_from_str("id0"),
            uparams: b.alloc_levels_slice(&[]),
            ty,
        },
        val,
        hint: ReducibilityHint::Regular(0),
    };
    b.add_declar(declar.clone()).expect("add declar");
    let env = b.finish();
    let result = env.try_check_declar(&declar);
    assert!(result.is_ok(), "non-dependent identity must check: {result:?}");
}

#[test]
fn empty_env_infers_and_reduces_a_closed_lambda() {
    let arena = Arena::new();
    let env = ExportFile::empty(arena.as_arena_ref(), Config::default());

    env.with_tc(EnvLimit::Empty, |tc| {
        // (fun x : Sort 0 => x)
        let prop = tc.ctx.mk_sort(tc.ctx.zero());
        let var = tc.ctx.mk_var(0);
        let lam = tc.ctx.mk_lambda(tc.ctx.anonymous(), BinderStyle::Default, prop, var);

        let ty = tc.infer_closed_type(lam);
        match tc.ctx.read_expr(ty) {
            Expr::Pi { binder_type, body, .. } => {
                assert!(matches!(tc.ctx.read_expr(binder_type), Expr::Sort { .. }));
                assert!(
                    matches!(tc.ctx.read_expr(body), Expr::Sort { .. }),
                    "the inferred codomain of an identity over Sort 0 is Sort 0"
                );
            }
            other => panic!("expected Pi type, got {other:?}"),
        }

        let printed = tc.with_pp(|pp| pp.pp_expr(ty));
        assert_eq!(printed, "Prop -> Prop", "unexpected pretty output: {printed}");

        let reduced = tc.reduce_closed(lam);
        assert!(
            matches!(tc.ctx.read_expr(reduced), Expr::Lambda { .. }),
            "identity lambda should already be in normal form"
        );
    });
}

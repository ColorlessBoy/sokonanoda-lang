//! M0 acceptance: drive the complete kernel from memory without any export
//! file, config file or external tool.

use sokonanoda::env::EnvLimit;
use sokonanoda::expr::{BinderStyle, Expr};
use sokonanoda::util::{Config, ExportFile};
use stumpalo::Arena;

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
        assert_eq!(printed, "Prop → Prop", "unexpected pretty output: {printed}");

        let reduced = tc.reduce_closed(lam);
        assert!(
            matches!(tc.ctx.read_expr(reduced), Expr::Lambda { .. }),
            "identity lambda should already be in normal form"
        );
    });
}

//! 可信内置声明（prelude）的安装：Nat 骨架与基础公理/定义。

use sokonanoda::builder::EnvBuilder;
use sokonanoda::env::{Declar, DeclarInfo, ReducibilityHint};
use sokonanoda::expr::BinderStyle;
use sokonanoda::util::ExprPtr;
use std::sync::Arc;

/// Trusted built-in base declarations. These are never re-checked by the
/// kernel: they are the axioms/inductive spine that the teaching grammar is
/// built on. The kernel's native Nat reduction is enabled purely by the
/// matching declaration names.
pub(crate) fn install_prelude(builder: &mut EnvBuilder<'_>) {
    let anon = builder.anonymous();
    let empty = builder.alloc_levels_slice(&[]);
    let type_level = builder.succ(builder.zero());
    let type_sort = builder.mk_sort(type_level);

    let nat = builder.name_from_str("Nat");
    let nat_type = builder.mk_const(nat, empty);
    builder
        .add_inductive(
            DeclarInfo {
                name: nat,
                uparams: empty,
                ty: type_sort,
            },
            false,
            0,
            0,
            Arc::from([nat]),
            Arc::from([]),
        )
        .expect("builtin Nat already present");

    add_axiom(builder, "Nat.zero", nat_type);

    let succ_arrow = builder.mk_pi(anon, BinderStyle::Default, nat_type, nat_type);
    let succ_name = builder.name_from_str("Nat.succ");
    let succ_levels = builder.alloc_levels_slice(&[]);
    let succ_self = builder.mk_const(succ_name, succ_levels);
    add_definition(builder, "Nat.succ", succ_arrow, succ_self);

    let inner_arrow = builder.mk_pi(anon, BinderStyle::Default, nat_type, nat_type);
    let add_arrow = builder.mk_pi(anon, BinderStyle::Default, nat_type, inner_arrow);
    let add_name = builder.name_from_str("Nat.add");
    let add_levels = builder.alloc_levels_slice(&[]);
    let add_self = builder.mk_const(add_name, add_levels);
    add_definition(builder, "Nat.add", add_arrow, add_self);
}

fn add_axiom<'a>(builder: &mut EnvBuilder<'a>, name: &str, ty: ExprPtr<'a>) {
    let name = builder.name_from_str(name);
    let info = DeclarInfo {
        name,
        uparams: builder.alloc_levels_slice(&[]),
        ty,
    };
    builder
        .add_declar(Declar::Axiom { info })
        .expect("duplicate builtin axiom");
}

fn add_definition<'a>(builder: &mut EnvBuilder<'a>, name: &str, ty: ExprPtr<'a>, val: ExprPtr<'a>) {
    let name = builder.name_from_str(name);
    let info = DeclarInfo {
        name,
        uparams: builder.alloc_levels_slice(&[]),
        ty,
    };
    builder
        .add_declar(Declar::Definition {
            info,
            val,
            hint: ReducibilityHint::Regular(0),
        })
        .expect("duplicate builtin definition");
}

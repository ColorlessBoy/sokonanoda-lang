//! **IA-4 K1 的硬不变式**（设计 `docs/design/metavar-engine.md` §2.11 ✓）。
//!
//! D8 = **(i) 内核占位符** ✓：内核 `Expr` 加 `Meta` 构造子 ✓，**判定层硬拒**含它的声明 ✓，
//! 前端协议**不变** ✓。本片（K1）**没有任何东西构造占位符** ✗ ⇒ **零行为变化** ✓；
//! 它给的是**结构性保证**：等 B2 真把元变量送进来时，"判定层见不到它"已经由入口挡住了 ✓。
//!
//! 本文件钉**两条**（设计 §2.11 的"共同硬不变式" ✓）：
//! ① **含占位符的声明一律拒绝** ✓（固定错误前缀 ✓，前端据此映射错误码 ✓）；
//! ② **走查必须走遍所有子项** ✗ —— 只查顶层就等于留缝 ✗（本文件专门钉**深处**那一例 ✓）。
use crate::builder::EnvBuilder;
use crate::env::{Declar, DeclarInfo};
use stumpalo::Arena;

/// 造一个最小环境 + 几个项 ✓。返回 `(arena, builder)` 的持有者 ✓。
struct Fixture<'a> {
    _arena: Box<Arena>,
    builder: EnvBuilder<'a>,
    prop: crate::util::ExprPtr<'a>,
    meta: crate::util::ExprPtr<'a>,
}

fn fixture<'a>(arena: &'a Arena) -> (EnvBuilder<'a>, crate::util::ExprPtr<'a>, crate::util::ExprPtr<'a>) {
    let mut builder = EnvBuilder::new(arena.as_arena_ref(), Default::default());
    let prop = builder.mk_sort(builder.zero());
    let meta = builder.mk_meta(7);
    (builder, prop, meta)
}

/// **硬不变式①**：类型位带占位符 ⇒ `add_declar` **必须拒绝** ✓，且消息有**固定前缀** ✓。
#[test]
fn a_declaration_whose_type_contains_a_placeholder_is_rejected() {
    let arena = Arena::new();
    let (mut builder, _prop, meta) = fixture(&arena);
    let name = builder.name_from_str("bad_ty");
    let d = Declar::Axiom {
        info: DeclarInfo {
            name,
            uparams: builder.alloc_levels_slice(&[]),
            ty: meta,
        },
    };
    let err = builder
        .add_declar(d)
        .expect_err("含占位符的声明**必须**被拒 ✗（K1 硬不变式①）");
    assert!(
        err.starts_with("declaration contains a metavariable placeholder"),
        "错误消息必须有**固定前缀** ✓（前端据此映射错误码 ✓）：{err:?}"
    );
}

/// **硬不变式①（值位）**：`Definition` 的值里带占位符 ⇒ 同样拒绝 ✓。
#[test]
fn a_definition_whose_value_contains_a_placeholder_is_rejected() {
    let arena = Arena::new();
    let (mut builder, prop, meta) = fixture(&arena);
    let name = builder.name_from_str("bad_val");
    let d = Declar::Definition {
        info: DeclarInfo {
            name,
            uparams: builder.alloc_levels_slice(&[]),
            ty: prop,
        },
        val: meta,
        hint: crate::env::ReducibilityHint::Regular(0),
    };
    let err = builder
        .add_declar(d)
        .expect_err("值位含占位符**也必须**被拒 ✗");
    assert!(err.starts_with("declaration contains a metavariable placeholder"));
}

/// ⚠ **硬不变式②（本文件最要紧的一条）**：占位符**埋在深处**也必须被找到 ✗。
///
/// 为什么专门钉它：只查顶层的实现会**放过**这一例 ⇒ 那就是"含元变量的声明能进环境"的缝 ✗
/// （设计 §2.11 要求的是**一律拒绝** ✓，不是"大多数拒绝" ✗）。
#[test]
fn a_placeholder_nested_deep_inside_the_type_is_still_rejected() {
    let arena = Arena::new();
    let (mut builder, prop, meta) = fixture(&arena);
    // `(Prop → ?m7) → Prop`：占位符在**两层里面** ✓。
    let anon = builder.name_from_str("_");
    let inner = builder.mk_pi(anon, crate::expr::BinderStyle::Default, prop, meta);
    let outer = builder.mk_pi(anon, crate::expr::BinderStyle::Default, inner, prop);
    assert!(
        builder.contains_meta(outer),
        "走查**必须**走遍所有子项 ✗ —— 只查顶层就会放过这一例 ✗"
    );
    let name = builder.name_from_str("deep");
    let d = Declar::Axiom {
        info: DeclarInfo {
            name,
            uparams: builder.alloc_levels_slice(&[]),
            ty: outer,
        },
    };
    assert!(builder.add_declar(d).is_err(), "深处带占位符 ⇒ 一样拒 ✓");
}

/// **对照组** ✓：**没有**占位符的声明**照常通过** ✓ ——
/// 否则这条守卫就是"一律拒绝"的空转 ✗（`AGENTS.md`：咬不住的守卫等于没有 ✓）。
#[test]
fn a_placeholder_free_declaration_is_still_accepted() {
    let arena = Arena::new();
    let (mut builder, prop, _meta) = fixture(&arena);
    let name = builder.name_from_str("ok_ax");
    let d = Declar::Axiom {
        info: DeclarInfo {
            name,
            uparams: builder.alloc_levels_slice(&[]),
            ty: prop,
        },
    };
    builder
        .add_declar(d)
        .expect("没有占位符的声明**必须**照常通过 ✓（否则守卫是空转 ✗）");
}

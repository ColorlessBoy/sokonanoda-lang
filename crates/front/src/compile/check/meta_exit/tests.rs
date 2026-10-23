use super::{b2_binder, decl_ctx, finish, new_meta_table, B2_UNTYPED_BINDER_MSG};
use crate::ast::Expr;
use crate::compile::elab::ElabCtx;
use crate::compile::error::ErrorKind;
use crate::compile::meta::{DeclMeta, DeclMetas};
use crate::span::Span;
use sokonanoda::builder::EnvBuilder;
use sokonanoda::env::{Declar, DeclarInfo, ReducibilityHint};
use sokonanoda::expr::BinderStyle;
use sokonanoda::util::{Config, ExprPtr};
use std::cell::RefCell;

/// `A` 的类型表达式（`A : Type` 的 `Const` 项）。
fn a_type<'a>(b: &mut EnvBuilder<'a>) -> ExprPtr<'a> {
    let name = b.name_from_str("A");
    let levels = b.alloc_levels_slice(&[]);
    b.mk_const(name, levels)
}

/// 一个**只有 `A : Type`** 的活环境（不装 prelude ⇒ 单测不依赖课程预置）。
/// `A` 的 sort 是 `Sort 1` ⇒ 与 `b2_binder` 猜的 `Sort 1` 对得上（C1）。
fn builder_with_a(arena: &stumpalo::Arena) -> EnvBuilder<'_> {
    let mut builder = EnvBuilder::new(arena.as_arena_ref(), Config::default());
    let zero = builder.zero();
    let one = builder.succ(zero);
    let ty = builder.mk_sort(one);
    let name = builder.name_from_str("A");
    let uparams = builder.alloc_levels_slice(&[]);
    builder
        .add_declar(Declar::Axiom {
            info: DeclarInfo { name, uparams, ty },
        })
        .expect("A 能进环境");
    builder
}

fn binder_span(e: &Expr, idx: usize) -> Span {
    match e {
        Expr::Lambda { binders, .. } => binders[idx].span,
        other => panic!("夹具必须是 lambda：{other:?}"),
    }
}

fn empty_ctx<'a, 'b>(
    prefix_src: &'b str,
    options: &'b crate::compile::prelude::CompileOptions,
    inductives: &'b crate::compile::elab::InductiveTable<'a>,
    ns: &'b crate::compile::scope::NamespaceScope,
    defs: &'b crate::compile::elab::DefTable,
    metas: Option<&'b RefCell<DeclMetas>>,
) -> ElabCtx<'a, 'b> {
    decl_ctx(prefix_src, options, inductives, ns, None, defs, metas)
}

/// 解出的元变量被写回源注解、声明无 meta、`add_declar` 接受 ✓。
///
/// 反向验证（施工单）：把 [`finish`] 里 `solved_in_decl` 的查找改成 `id + 1`
/// ⇒ 本条**当场红**（改写不到注解 ⇒ `finish` 返回 `Err` ✗）。
#[test]
fn solved_meta_is_rewritten_and_the_declaration_is_meta_free() {
    let arena = stumpalo::Arena::new();
    let mut builder = builder_with_a(&arena);
    let src = crate::proof::parse_expr_text("fun x => x").expect("源能解析");
    let span = binder_span(&src, 0);
    // 核项：`fun (x : ?7) => x`，`?7 : Sort 1`；声明类型 `A -> A`。
    let a = a_type(&mut builder);
    let zero = builder.zero();
    let one = builder.succ(zero);
    let sort = builder.mk_sort(one);
    let meta = builder.mk_meta(7, sort);
    let var = builder.mk_var(0);
    let bx = builder.name_from_str("x");
    let lam = builder.mk_lambda(bx, BinderStyle::Default, meta, var);
    let anon = builder.anonymous();
    let pi = builder.mk_pi(anon, BinderStyle::Default, a, a);
    let name = builder.name_from_str("f");
    let uparams = builder.alloc_levels_slice(&[]);
    let decl = Declar::Definition {
        info: DeclarInfo {
            name,
            uparams,
            ty: pi,
        },
        val: lam,
        hint: ReducibilityHint::Regular(0),
    };
    let table = new_meta_table();
    table.borrow_mut().entries.push(DeclMeta { id: 7, span });
    let options = crate::compile::prelude::CompileOptions::default();
    let ns = crate::compile::scope::NamespaceScope::new();
    let inductives = crate::compile::elab::InductiveTable::new();
    let defs = crate::compile::elab::DefTable::new();
    let ctx = empty_ctx("", &options, &inductives, &ns, &defs, Some(&table));
    let out = finish(&mut builder, decl, &src, ctx.metas, |b, rewritten| {
        // ① 注解被写回**源** AST：`fun (x : A) => x` ✓。
        match rewritten {
            Expr::Lambda { binders, .. } => match binders[0].ty.as_deref() {
                Some(Expr::Ident { name, .. }) => {
                    assert_eq!(name, "A", "解出的类型必须按 span 写回 binder 注解")
                }
                other => panic!("注解必须是 `A`：{other:?}"),
            },
            other => panic!("重写后必须仍是 lambda：{other:?}"),
        }
        // ② 第二遍交出的声明**没有**元变量 ✓。
        let a = a_type(b);
        let anon = b.anonymous();
        let ty = b.mk_pi(anon, BinderStyle::Default, a, a);
        let var = b.mk_var(0);
        let bx = b.name_from_str("x");
        let val = b.mk_lambda(bx, BinderStyle::Default, a, var);
        let name = b.name_from_str("f");
        let uparams = b.alloc_levels_slice(&[]);
        Ok(Declar::Definition {
            info: DeclarInfo { name, uparams, ty },
            val,
            hint: ReducibilityHint::Regular(0),
        })
    })
    .expect("解得出 ⇒ 出口成功");
    // ③ 硬闸的正面：出口后没有 meta，且 `add_declar` 接受 ✓。
    assert!(
        !builder.declar_contains_meta(&out),
        "出口后声明不许再有元变量"
    );
    builder.add_declar(out).expect("无 meta 的声明能进环境");
}

/// 解不出 ⇒ 逐字今天那条诊断（码 / 句 / span 三件都断言 ✓）。
#[test]
fn unsolved_meta_keeps_the_exact_untyped_binder_diagnostic() {
    let arena = stumpalo::Arena::new();
    let mut builder = builder_with_a(&arena);
    // `fun (x : A) => fun y => x`：`y` 的类型三档都剥不出 ⇒ 元变量；声明类型
    // `A -> A` 解不出它（`?y -> A` vs `A`）⇒ 必须报**今天那条**错。
    let src = crate::proof::parse_expr_text("fun (x : A) => fun y => x").expect("源能解析");
    // `y` 是**内层** lambda 的 binder（外层 binder 是 `x`）。
    let span = match &src {
        Expr::Lambda { body, .. } => match body.as_ref() {
            Expr::Lambda { binders, .. } => binders[0].span,
            other => panic!("内层必须是 lambda：{other:?}"),
        },
        other => panic!("外层必须是 lambda：{other:?}"),
    };
    let a = a_type(&mut builder);
    let zero = builder.zero();
    let one = builder.succ(zero);
    let sort = builder.mk_sort(one);
    let meta = builder.mk_meta(9, sort);
    let inner_var = builder.mk_var(1);
    let by = builder.name_from_str("y");
    let body = builder.mk_lambda(by, BinderStyle::Default, meta, inner_var);
    let bx = builder.name_from_str("x");
    let lam = builder.mk_lambda(bx, BinderStyle::Default, a, body);
    let anon = builder.anonymous();
    let pi = builder.mk_pi(anon, BinderStyle::Default, a, a);
    let name = builder.name_from_str("g");
    let uparams = builder.alloc_levels_slice(&[]);
    let decl = Declar::Definition {
        info: DeclarInfo {
            name,
            uparams,
            ty: pi,
        },
        val: lam,
        hint: ReducibilityHint::Regular(0),
    };
    let table = new_meta_table();
    table.borrow_mut().entries.push(DeclMeta { id: 9, span });
    let options = crate::compile::prelude::CompileOptions::default();
    let ns = crate::compile::scope::NamespaceScope::new();
    let inductives = crate::compile::elab::InductiveTable::new();
    let defs = crate::compile::elab::DefTable::new();
    let ctx = empty_ctx("", &options, &inductives, &ns, &defs, Some(&table));
    let err = finish(&mut builder, decl, &src, ctx.metas, |_b, _src| {
        panic!("未解 ⇒ 不许重 elaborate")
    })
    .expect_err("解不出必须报错");
    assert_eq!(err.kind, ErrorKind::ElabUntypedBinder, "码必须逐字不变");
    assert_eq!(err.message, B2_UNTYPED_BINDER_MSG, "句必须逐字不变");
    assert_eq!(err.span, span, "span 必须是那个未注解 binder");
}

/// `b2_binder` 在**开关关**（默认档）时**不造元变量**、`ctx.metas == None` 时也不造 ✓。
///
/// ⚠ 这条**按定义**只在 `SOKO_DECL_METAS` 未设时成立（与 `soko_defer_off.rs` 同款
/// 纪律：档位由**进程环境**决定 ⇒ 反向验证单独跑、不写进文件）。
#[test]
fn b2_binder_is_inert_without_the_switch() {
    let arena = stumpalo::Arena::new();
    let mut builder = builder_with_a(&arena);
    let options = crate::compile::prelude::CompileOptions::default();
    let ns = crate::compile::scope::NamespaceScope::new();
    let inductives = crate::compile::elab::InductiveTable::new();
    let defs = crate::compile::elab::DefTable::new();
    let table = new_meta_table();
    let ctx = empty_ctx("", &options, &inductives, &ns, &defs, Some(&table));
    let binder = crate::ast::Binder {
        name: "x".into(),
        ty: None,
        style: crate::ast::BinderKind::Explicit,
        span: Span::default(),
    };
    assert!(
        b2_binder(&mut builder, &ctx, &binder).is_none(),
        "默认档（SOKO_DECL_METAS 未设）不许造元变量"
    );
    assert!(table.borrow().entries.is_empty(), "默认档表必须空");
    let ctx_none = empty_ctx("", &options, &inductives, &ns, &defs, None);
    assert!(
        b2_binder(&mut builder, &ctx_none, &binder).is_none(),
        "非声明通道（#check/judge/预置库）不许造元变量"
    );
}

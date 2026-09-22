//! `pretty_printer.rs` 的基础单测（T-K32，线 C 的**前置**）。
//!
//! 为什么先补它：内核 pp 同时是 `#check` / `#reduce` / `#print` 的出口
//! （⇒ 直接进 `--json` 的 `expr.typed` / `expr.reduced` / `decl.printed`），
//! 而线 C（goal / 类型行用记法）要在**它的出口之后**做重写——没有一组钉住现状
//! 的测试，"重写改坏了 pp"与"重写本身写错了"分不开。
//!
//! 这些是**特征化测试**：断言的是**今天**的输出。输出变了必须是有意的
//! （改 pp 的 diff 里看得见），不许悄悄漂。

use sokonanoda::builder::EnvBuilder;
use sokonanoda::env::{Declar, DeclarInfo, EnvLimit};
use sokonanoda::expr::BinderStyle;
use sokonanoda::util::{Config, ExprPtr};
use stumpalo::Arena;

/// 在环境里声明一个公理 `name : ty`。
fn axiom<'a>(b: &mut EnvBuilder<'a>, name: &str, ty: ExprPtr<'a>) {
    let n = b.name_from_str(name);
    let uparams = b.alloc_levels_slice(&[]);
    b.add_declar(Declar::Axiom {
        info: DeclarInfo { name: n, uparams, ty },
    })
    .expect("add axiom");
}

/// 非依赖的箭头类型 `dom -> cod`（匿名 binder）。
fn arrow<'a>(b: &mut EnvBuilder<'a>, dom: ExprPtr<'a>, cod: ExprPtr<'a>) -> ExprPtr<'a> {
    let anon = b.anonymous();
    b.mk_pi(anon, BinderStyle::Default, dom, cod)
}

/// 建一个项并 pp 它。闭包负责先把用到的常量声明好——pp 要靠常量的类型决定
/// 隐式实参折成什么样，没声明的常量会让 `const_head_type` panic。
fn pp(build: impl for<'a> FnOnce(&mut EnvBuilder<'a>) -> ExprPtr<'a>) -> String {
    let arena = Arena::new();
    let mut b = EnvBuilder::new(arena.as_arena_ref(), Config::default());
    let expr = build(&mut b);
    let mut env = b.finish();
    // 与教学前端一致（`check/mod.rs` 的 `finish_pass` 也这么设）：`proofs = false`
    // 时 pp 会对**开项**跑 `is_proof` 推断（想把它折成 `_`），而 binder 体内的
    // 松散变量在空 context 下推不出来 ⇒ `infer: loose bvar` panic。
    env.config.pp_options.proofs = true;
    env.with_tc(EnvLimit::Empty, |tc| tc.with_pp(|pp| pp.pp_expr(expr)))
}

#[test]
fn arrows_and_foralls() {
    // `->` 与 `forall` 的选择、`{}` 隐式、以及"binder 没被用到就折成箭头"。
    let cases: Vec<(&str, for<'a> fn(&mut EnvBuilder<'a>) -> ExprPtr<'a>, &str)> = vec![
        ("非依赖 Pi", |b| {
            let prop = b.mk_sort(b.zero());
            let anon = b.anonymous();
            b.mk_pi(anon, BinderStyle::Default, prop, prop)
        }, "Prop -> Prop"),
        ("依赖 Pi", |b| {
            let prop = b.mk_sort(b.zero());
            let a = b.name_from_str("a");
            let var0 = b.mk_var(0);
            b.mk_pi(a, BinderStyle::Default, prop, var0)
        }, "forall (a : Prop), a"),
        ("隐式 binder", |b| {
            let prop = b.mk_sort(b.zero());
            let a = b.name_from_str("a");
            let var0 = b.mk_var(0);
            b.mk_pi(a, BinderStyle::Implicit, prop, var0)
        }, "forall {a : Prop}, a"),
        ("binder 没被用到 ⇒ 折成箭头", |b| {
            let prop = b.mk_sort(b.zero());
            let a = b.name_from_str("a");
            b.mk_pi(a, BinderStyle::Default, prop, prop)
        }, "Prop -> Prop"),
        ("匿名 Pi 套具名 Pi：内层要括号", |b| {
            let prop = b.mk_sort(b.zero());
            let a = b.name_from_str("a");
            let anon = b.anonymous();
            let var0 = b.mk_var(0);
            let inner = b.mk_pi(a, BinderStyle::Default, prop, var0);
            b.mk_pi(anon, BinderStyle::Default, prop, inner)
        }, "Prop -> (forall (a : Prop), a)"),
        ("同域两层：内层只用外层变量", |b| {
            let prop = b.mk_sort(b.zero());
            let a = b.name_from_str("a");
            let bb = b.name_from_str("b");
            let var0 = b.mk_var(0);
            let var1 = b.mk_var(1);
            let inner = b.mk_pi(bb, BinderStyle::Default, prop, var1);
            b.mk_pi(a, BinderStyle::Default, prop, inner)
        }, "forall (a : Prop), Prop -> a"),
    ];
    for (label, build, want) in cases {
        let got = pp(build);
        assert_eq!(got, want, "{label}：pp 输出变了");
    }
}

#[test]
fn universes() {
    // `Prop` / `Type n`。**注意这是 pp 的文本契约**：`Sort 1` 印成 `Type 0`。
    assert_eq!(pp(|b| b.mk_sort(b.zero())), "Prop", "Sort 0 印成 Prop");
    assert_eq!(
        pp(|b| {
            let one = b.succ(b.zero());
            b.mk_sort(one)
        }),
        "Type 0",
        "Sort 1 印成 Type 0",
    );
}

#[test]
fn applications_and_parentheses() {
    // 应用左结合、参数里的应用/箭头要括号。
    let app = pp(|b| {
        let prop = b.mk_sort(b.zero());
        let pp_ty = arrow(b, prop, prop);
        let ppp_ty = arrow(b, prop, pp_ty);
        axiom(b, "f", ppp_ty);
        axiom(b, "x", prop);
        axiom(b, "y", prop);
        let empty = b.alloc_levels_slice(&[]);
        let f_n = b.name_from_str("f");
        let x_n = b.name_from_str("x");
        let y_n = b.name_from_str("y");
        let f = b.mk_const(f_n, empty);
        let x = b.mk_const(x_n, empty);
        let y = b.mk_const(y_n, empty);
        let app = b.mk_app(f, x);
        b.mk_app(app, y)
    });
    assert_eq!(app, "f x y", "应用左结合、不加多余括号");

    let nested = pp(|b| {
        let prop = b.mk_sort(b.zero());
        let pp_ty = arrow(b, prop, prop);
        axiom(b, "f", pp_ty);
        axiom(b, "g", pp_ty);
        axiom(b, "x", prop);
        let empty = b.alloc_levels_slice(&[]);
        let f_n = b.name_from_str("f");
        let g_n = b.name_from_str("g");
        let x_n = b.name_from_str("x");
        let f = b.mk_const(f_n, empty);
        let g = b.mk_const(g_n, empty);
        let x = b.mk_const(x_n, empty);
        let inner = b.mk_app(g, x);
        b.mk_app(f, inner)
    });
    assert_eq!(nested, "f (g x)", "参数是应用 ⇒ 必须括号");

    let pi_arg = pp(|b| {
        let prop = b.mk_sort(b.zero());
        let anon = b.anonymous();
        let pi = b.mk_pi(anon, BinderStyle::Default, prop, prop);
        let f_ty = arrow(b, pi, prop);
        axiom(b, "f", f_ty);
        let empty = b.alloc_levels_slice(&[]);
        let f_n = b.name_from_str("f");
        let f = b.mk_const(f_n, empty);
        b.mk_app(f, pi)
    });
    assert_eq!(pi_arg, "f (Prop -> Prop)", "参数是箭头 ⇒ 必须括号");
}

#[test]
fn lambdas_and_anonymous_binders() {
    // 匿名 binder 印成空转义 `«»`（Lean 同款：那个位置本应是个标识符）。
    let got = pp(|b| {
        let prop = b.mk_sort(b.zero());
        let anon = b.anonymous();
        let var0 = b.mk_var(0);
        b.mk_lambda(anon, BinderStyle::Default, prop, var0)
    });
    assert_eq!(got, "fun («» : Prop) => «»", "匿名 binder 的空转义");
}

#[test]
fn constants_keep_their_levels_out_of_the_text() {
    // 层级实参**不出现在文本里**（`options.explicit` 默认关）：`E` 而不是 `E.{0}`。
    // 线 C 的重写要在**这个出口之后**做，所以这条是"重写不该看到 `.{…}`"的钉子。
    let got = pp(|b| {
        let prop = b.mk_sort(b.zero());
        let levels = b.alloc_levels_slice(&[b.zero()]);
        let n = b.name_from_str("E");
        axiom(b, "E", prop);
        b.mk_const(n, levels)
    });
    assert_eq!(got, "E", "层级默认不显式打印");
}

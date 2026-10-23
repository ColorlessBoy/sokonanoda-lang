use super::{diff, replace_at, DiffResult, Step};
use crate::ast::{Binder, BinderKind, Expr};
use crate::span::Span;

fn ident(name: &str) -> Expr {
    Expr::Ident {
        name: name.to_string(),
        span: Span::default(),
    }
}
fn app(f: Expr, a: Expr) -> Expr {
    Expr::App {
        fun: Box::new(f),
        arg: Box::new(a),
        explicit_spine: false,
        span: Span::default(),
    }
}
fn mem(x: &str, a: &str) -> Expr {
    Expr::Notation {
        symbol: "∈".to_string(),
        symbol_span: Span::default(),
        target: "Set.mem".to_string(),
        assoc: crate::ast::NotationAssoc::Infix,
        lhs: Some(Box::new(ident(x))),
        rhs: Some(Box::new(ident(a))),
        alternatives: Vec::new(),
        span: Span::default(),
    }
}
fn lam(body: Expr) -> Expr {
    Expr::Lambda {
        binders: vec![Binder {
            name: "z".to_string(),
            ty: Some(Box::new(ident("T"))),
            style: BinderKind::Explicit,
            span: Span::default(),
        }],
        body: Box::new(body),
        span: Span::default(),
    }
}

/// **P2**：差异在 binder **里面**（`fun z => z ∈ A` ≟ `fun z => z ∈ B`）⇒
/// 仍然只算一处，路径带 `BinderBody`；沿路径替换能抽出 motive。
#[test]
fn deep_occurrence_under_a_binder_synthesizes_a_motive() {
    let (l, r) = (lam(mem("z", "A")), lam(mem("z", "B")));
    let DiffResult::One(d) = diff(&l, &r) else {
        panic!("binder 里的唯一差异应算一处");
    };
    assert!(
        matches!(d.path.first(), Some(Step::BinderBody)),
        "路径首步应是 BinderBody"
    );
    let motive_body = replace_at(&r, &d.path, &ident("s"));
    // 结构断言（不渲染 ⇒ 不绕记法唯一接口）：`fun z => z ∈ s`
    let Expr::Lambda { body, .. } = &motive_body else {
        panic!("motive 应仍是 lambda");
    };
    let Expr::Notation { rhs, .. } = body.as_ref() else {
        panic!("lambda 体应还是记法节点");
    };
    assert!(
        matches!(rhs.as_deref(), Some(Expr::Ident { name, .. }) if name == "s"),
        "差异位应换成 s"
    );
    // 多处独立差异 ⇒ 不猜（Many）：头部与实参同时不同
    let many_l = lam(app(ident("f"), ident("A")));
    let many_r = lam(app(ident("g"), ident("B")));
    assert!(matches!(diff(&many_l, &many_r), DiffResult::Many));
}

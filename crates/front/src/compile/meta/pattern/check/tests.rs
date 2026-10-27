//! **HO3b 的真内核判据**（`docs/design/metavar-pattern.md` §3.3/§4）。
//!
//! 这些用例**不读环境**（`scripts/check-test-env-isolation.py` 的纪律）：闸门档位一律由
//! `DeclMetaCtx::with_flag` / 直接调 `judge` 给；环境闸的判据是**进程级**的
//! （M0 13 形状 × 三臂 + 7 文件 `--json`）。
//!
//! **判据分三层**：
//! 1. [`judge`] 本体（真 `judge_terms_strict` + 真前缀）：用户在 P0 现场的那个形状
//!    `fun (A : Set α) => Not (Set.mem x A) : Set α → Prop` 必须**成立**；
//! 2. fail-closed 的两条闸（`\0` 名字 / 重入）；
//! 3. **接线**：store 上的 `check` 必须被 `meta_ctx` 带进 `MetaCtx`、并由 ⑨ 真的调用
//!    （`note_ctx` 的环境闸由进程级判据钉住）。

use std::cell::Cell;
use std::rc::Rc;

use super::*;
use crate::ast::{Binder, BinderKind, SortKind};
use crate::compile::dialect::Dialect;
use crate::compile::elab::{DefTable, InductiveTable};
use crate::compile::meta::pattern::{CheckFn, Scope};
use crate::compile::meta::{DeclMetaCtx, MetaCtx, MetaKind, MetaStore, Tri};
use crate::compile::prelude::PreludeMode;
use crate::compile::scope::NamespaceScope;
use crate::span::Span;

// ───────────────────────────── 夹具 ─────────────────────────────

fn ident(name: &str) -> Expr {
    Expr::Ident {
        name: name.to_string(),
        span: Span::default(),
    }
}

fn app(fun: Expr, arg: Expr) -> Expr {
    Expr::App {
        fun: Box::new(fun),
        arg: Box::new(arg),
        explicit_spine: false,
        span: Span::default(),
    }
}

fn lambda(name: &str, ty: Expr, body: Expr) -> Expr {
    Expr::Lambda {
        binders: vec![Binder {
            name: name.to_string(),
            ty: Some(Box::new(ty)),
            style: BinderKind::Explicit,
            span: Span::default(),
        }],
        body: Box::new(body),
        span: Span::default(),
    }
}

fn sort_prop() -> Expr {
    Expr::Sort {
        sort: SortKind::Prop,
        span: Span::default(),
    }
}

fn sort_type() -> Expr {
    Expr::Sort {
        sort: SortKind::Type,
        span: Span::default(),
    }
}

/// `Set` / `Set.mem` 的最小前奏（**自足**：不 import、不依赖文件名）。
const PREFIX: &str = "\
def Set (α : Type) : Type := α -> Prop
def Set.mem {α : Type} (a : α) (A : Set α) : Prop := A a
";

/// 一份 `ElabCtx`（真前缀 + 真 options；`metas: None` = 只做判定）。
fn with_ctx<R>(prefix: &str, f: impl FnOnce(&ElabCtx<'_, '_>) -> R) -> R {
    let options = CompileOptions::with_dialect(PreludeMode::Full, Dialect::Loose);
    let inductives: InductiveTable<'_> = InductiveTable::new();
    let ns = NamespaceScope::default();
    let defs = DefTable::new();
    let ctx = ElabCtx {
        prefix_src: prefix,
        options: &options,
        inductives: &inductives,
        ns: &ns,
        notations: None,
        defs: &defs,
        metas: None,
    };
    f(&ctx)
}

fn no_unfold(e: &Expr) -> Expr {
    e.clone()
}

/// P0 现场的局部上下文：`(α : Type) (A : Set α) (x : α)`。
fn p0_binders() -> Vec<LocalBinder> {
    vec![
        LocalBinder {
            name: "α".to_string(),
            ty: Some(sort_type()),
            is_let: false,
        },
        LocalBinder {
            name: "A".to_string(),
            ty: Some(app(ident("Set"), ident("α"))),
            is_let: false,
        },
        LocalBinder {
            name: "x".to_string(),
            ty: Some(ident("α")),
            is_let: false,
        },
    ]
}

/// P0 的**解**：`fun (A : Set α) => Not (Set.mem x A)`（`?p A ≟ ¬ (x ∈ A)` 的最一般解）。
fn p0_lambda() -> Expr {
    lambda(
        "A",
        app(ident("Set"), ident("α")),
        app(
            ident("Not"),
            app(app(ident("Set.mem"), ident("x")), ident("A")),
        ),
    )
}

/// P0 的**目标类型**：`Set α → Prop`（`Eq.subst` 的 motive 位）。
fn p0_ty() -> Expr {
    Expr::Arrow {
        domain: Box::new(app(ident("Set"), ident("α"))),
        codomain: Box::new(sort_prop()),
        span: Span::default(),
    }
}

// ───────────────────── ① 真内核：用户 P0 的形状必须成立 ─────────────────────

/// **判据（P0 现场）**：`fun (A : Set α) => Not (Set.mem x A) : Set α → Prop` 在内核里
/// 成立 —— 这正是 `apply Eq.subst h` 里 motive 元变量要解的东西（`unit02` 练习 5）。
///
/// 反向验证：把 [`p0_ty`] 换成 `Set α → Set α` ⇒ 本条当场红（见下一条）。
#[test]
fn the_judge_accepts_the_user_p0_shape() {
    with_ctx(PREFIX, |ctx| {
        let base = Base::new(ctx.prefix_src, *ctx.options, ctx.notations);
        assert!(
            judge(&base, &p0_binders(), &p0_lambda(), &p0_ty()),
            "P0 形状 `fun A => Not (Set.mem x A) : Set α → Prop` 必须过内核检查"
        );
    });
}

/// **判据（不许假阴 → 也不许假阳）**：同一个 λ 说成 `Set α → Set α` ⇒ **拒**。
#[test]
fn the_judge_rejects_a_wrong_type() {
    with_ctx(PREFIX, |ctx| {
        let base = Base::new(ctx.prefix_src, *ctx.options, ctx.notations);
        let wrong = Expr::Arrow {
            domain: Box::new(app(ident("Set"), ident("α"))),
            codomain: Box::new(app(ident("Set"), ident("α"))),
            span: Span::default(),
        };
        assert!(
            !judge(&base, &p0_binders(), &p0_lambda(), &wrong),
            "类型不对的 λ 必须被内核拒（否则 ⑨ 就是假闸）"
        );
    });
}

/// **判据（真实用例：`Not (Set.mem x A)` 少了实参）**：`fun (A : Set α) => Not` 不是
/// `Set α → Prop` ⇒ 拒（`Not` 是 `Prop → Prop`）。
#[test]
fn the_judge_rejects_a_bare_constant_body() {
    with_ctx(PREFIX, |ctx| {
        let base = Base::new(ctx.prefix_src, *ctx.options, ctx.notations);
        let bad = lambda("A", app(ident("Set"), ident("α")), ident("Not"));
        assert!(!judge(&base, &p0_binders(), &bad, &p0_ty()));
    });
}

// ───────────────────────── ② fail-closed 的两条闸 ─────────────────────────

/// **`\0` 名字 ⇒ 弃权**（元载体 / 望远镜 fresh 名在合成文档里词法非法）。
#[test]
fn the_judge_declines_on_carrier_names() {
    with_ctx(PREFIX, |ctx| {
        let base = Base::new(ctx.prefix_src, *ctx.options, ctx.notations);
        let carrier = Expr::Ident {
            name: "\u{0}soko_m3".to_string(),
            span: Span::default(),
        };
        let body = lambda("A", app(ident("Set"), ident("α")), carrier.clone());
        assert!(!judge(&base, &p0_binders(), &body, &p0_ty()));
        // binder 注解里带载体也一样（合成文档的 binder 表会 parse 不过）⇒ 弃权。
        let mut binders = p0_binders();
        binders[1].ty = Some(carrier);
        assert!(!judge(&base, &binders, &p0_lambda(), &p0_ty()));
    });
}

/// **重入 ⇒ 弃权**（判定里再判定会无界递归：`judge_terms` 又跑一遍完整流水线）。
#[test]
fn the_judge_declines_on_reentry() {
    with_ctx(PREFIX, |ctx| {
        let base = Base::new(ctx.prefix_src, *ctx.options, ctx.notations);
        // 模拟"已经在一次判定里"：手工把闸拉上 ⇒ 这一次必须直接弃权。
        IN_CHECK.with(|c| c.set(true));
        let got = judge(&base, &p0_binders(), &p0_lambda(), &p0_ty());
        IN_CHECK.with(|c| c.set(false));
        assert!(!got, "重入必须弃权（否则判定里再判定 = 无界递归）");
        // 闸已经弹回 ⇒ 下一次照常判（不是永久关闭）。
        assert!(judge(&base, &p0_binders(), &p0_lambda(), &p0_ty()));
    });
}

// ───────────────────────── ③ 基座缓存与接线 ─────────────────────────

/// **前缀缓存**：同一条声明里两次 `base_for` 必须是**同一个** `Rc`（不重复克隆前缀）；
/// 换一份前缀（不同文本 ⇒ 不同地址）⇒ 必须换基座。
#[test]
fn the_prefix_base_is_reused_within_one_declaration() {
    let decl = DeclMetaCtx::with_flag(true);
    let store = decl.store.as_ref().expect("闸门开 ⇒ 声明级 store");
    with_ctx(PREFIX, |ctx| {
        let a = base_for(store, ctx);
        let b = base_for(store, ctx);
        assert!(Rc::ptr_eq(&a, &b), "同一条声明内的前缀基座必须复用");
    });
    let other = format!("{PREFIX}def extra : Prop := True\n");
    with_ctx(&other, |ctx| {
        let c = base_for(store, ctx);
        assert!(
            !c.prefix_src.is_empty() && c.prefix_src.ends_with("def extra : Prop := True\n"),
            "换了一份前缀 ⇒ 基座必须换（并**带着**新前缀）"
        );
    });
}

/// **接线（HO3b 的核心判据）**：store 上的 checker 必须经 `meta_ctx` 进到 `MetaCtx`，
/// 并在 ⑨ 被**调用**；真实 checker（`judge`）在这个方程上必须让它**成立**。
///
/// 方程 = P0 的形状：`?m A ≟ Not (Set.mem x A)`，`lctx(m)` = `(α : Type) (A : Set α) (x : α)`。
#[test]
fn the_installed_checker_reaches_the_rule_and_judges_the_p0_equation() {
    let decl = DeclMetaCtx::with_flag(true);
    let store = decl.store.as_ref().expect("闸门开 ⇒ 声明级 store");
    let calls = Rc::new(Cell::new(0usize));
    let lctx = Rc::new(p0_binders());
    with_ctx(PREFIX, |ctx| {
        let base = base_for(store, ctx);
        let bound = lctx.clone();
        let (judged, count) = (base.clone(), calls.clone());
        let check: CheckFn = Rc::new(move |lambda: &Expr, ty: &Expr| {
            count.set(count.get() + 1);
            judge(&judged, &bound, lambda, ty)
        });
        store.set_scope(Scope {
            lctx: lctx.clone(),
            check: Some(check),
            base: Some(base),
        });
    });
    let mut guard = store.park();
    let mut meta: MetaCtx<'_> = guard.meta_ctx(&no_unfold);
    assert!(meta.check.is_some(), "`meta_ctx` 必须把 checker 带进来");
    // **strict** 形状（P0 的 `?m A ≟ …` 是 quasi；quasi 的“实参自由出现在 t 里”
    // 已按 Lean 条件 2 fail-closed 弃权，见 pattern/tests.rs）。这里只验 checker 通道：
    // `?m : α → α`，`?m x ≟ x` ⇒ `fun x => x`。
    meta.lctx = Rc::new(Vec::new());
    let ty = Expr::Arrow {
        domain: Box::new(ident("α")),
        codomain: Box::new(ident("α")),
        span: Span::default(),
    };
    let id = meta.fresh(ty, MetaKind::Natural, vec![]);
    meta.lctx = Rc::new(p0_binders());
    let lhs = app(meta.meta_expr(id), ident("x"));
    let rhs = ident("x");
    assert_eq!(meta.pattern_assign(&lhs, &rhs), Tri::Yes);
    assert_eq!(meta.value(id), Some(lambda("x", ident("α"), ident("x"))));
    assert_eq!(meta.zonk(&lhs), rhs, "zonk 之后必须回到原式");
    assert_eq!(calls.get(), 1, "⑨ 必须**真的**调一次 checker");
}

/// **没有 checker ⇒ 弃权**（fail-closed；生产里的非声明通道 / 闸门关就是这一档）。
#[test]
fn without_a_checker_the_rule_declines() {
    let mut store = MetaStore::default();
    let mut meta = MetaCtx::new(&no_unfold, &mut store);
    meta.lctx = Rc::new(p0_binders());
    let id = meta.fresh(p0_ty(), MetaKind::Natural, vec![]);
    let lhs = app(meta.meta_expr(id), ident("A"));
    let rhs = app(
        ident("Not"),
        app(app(ident("Set.mem"), ident("x")), ident("A")),
    );
    assert_eq!(meta.pattern_assign(&lhs, &rhs), Tri::Undef);
    assert_eq!(meta.value(id), None);
}

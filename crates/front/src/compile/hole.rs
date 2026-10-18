//! **实参位的洞**（`_`，a14）：把 `Expr::Hole` 提升成**新 binder**（`_h<span 偏移>`）。
//!
//! **Lean 4 对照**（本机源码 `~/Documents/lean/lean4` HEAD `d0493e4c1e`）：
//! 1. `Parser/Term/Basic.lean:117` —— `@[builtin_term_parser] def hole := leading_parser "_"`：
//!    项位的 `_` 是**独立语法类**（模式位的 `_` 另有路径，`elabHole` 按 `inPattern` 分档）。
//! 2. `Elab/BuiltinTerm.lean:65 elabHole` —— `mkFreshExprMVar expectedType? kind`
//!    （项位 `kind = syntheticOpaque`）⇒ `_` = **带期望类型的合成元变量**；
//!    期望类型由应用望远镜逐位给（`Elab/App.lean:1176 elabAppArgs`）。
//! 3. `Meta/Tactic/Apply.lean:169 MVarId.apply` —— **未解**的那些元变量**变成新目标**。
//! 4. **我们的偏离（白纸黑字）**：本引擎**没有项元变量**（`meta.rs` 的元变量只服务隐式
//!    前缀，且出口 `zonk` 后**不许进项**）⇒ 用「**新 binder + 调用方包 λ**」达到同一效果：
//!    `infer_type_text_inplace_expected` 把作用域里的 binder 逐层包成 λ ⇒ 洞在类型文本里
//!    **就是一层 Π** ⇒ `by.rs::apply_tactic` 现成的「非类型参数的层 ⇒ 子目标」机械
//!    直接把它变成目标 ✓（与 `goals.rs::probe_arg_type` 的既有口径同源：前置洞提升为
//!    `_h0`,`_h1`…）。
//! 5. **两趟**（为什么不能一趟）：binder 压栈会**加长作用域**，而 de Bruijn 索引按
//!    「压栈那一刻的作用域长度」算 ⇒ 后压的洞会让**先建好的**外层引用**整体偏移** ✗。
//!    ⇒ 第一趟只**探**（洞压栈 + 记账，项丢弃），第二趟把洞 binder **预压**好再重
//!    elaborate（洞按**名字**认领已压的 binder，**不再压栈** ⇒ 索引不漂 ✓）。

use super::check::quiet_catch;
use super::elab::{
    elab_expr, ElabCtx, ElabScope, HoverNode, InplaceEnv, InplaceFail, KnownTable, UnivMap,
};
use super::error::{CompileError, ErrorKind};
use crate::ast::Expr;
use crate::span::Span;
use sokonanoda::builder::EnvBuilder;
use sokonanoda::util::ExprPtr;
use std::cell::{Cell, RefCell};

thread_local! {
    /// 提升窗口：**只有** [`probe`] 那一趟打开（窗口外 `_` 照旧报 `elab-hole-misplaced`，
    /// 开卷练习的 `sorry` 依赖那条 ✗ 不许动）。
    static LIFT: Cell<bool> = const { Cell::new(false) };
    /// 探针那一趟收下的洞：`(binder 名, 期望类型源, span)`，按**压栈顺序**（= 实参顺序）。
    static FOUND: RefCell<Vec<(String, Expr, Span)>> = const { RefCell::new(Vec::new()) };
}

/// 洞 binder 的名字：`_h<span 起点偏移>` —— 同一个项里的多个洞**互不撞名** ✓，
/// 两趟之间靠它**认领**同一个洞 ✓。
fn hole_name(span: Span) -> String {
    format!("_h{}", span.start.offset)
}

/// 项里的洞，按**应用脊的左到右**顺序（= elaborate 顺序 = 提升顺序 ✓）。
///
/// ⚠ 只走 `App`/`Hole`：提升面**已经**被 [`only_spine_holes`] 收窄到"最外层脊的
/// 直接实参位"，其余形状里不会有被提升的洞 ✓（`assemble` 也靠这条口径填洞 ✓）。
pub(crate) fn collect_holes(e: &Expr) -> Vec<Span> {
    fn walk(e: &Expr, out: &mut Vec<Span>) {
        match e {
            Expr::Hole { span } => out.push(*span),
            Expr::App { fun, arg, .. } => {
                walk(fun, out);
                walk(arg, out);
            }
            _ => {}
        }
    }
    let mut out = Vec::new();
    walk(e, &mut out);
    out
}

/// 把第 `k` 个洞换成 `with`（**就地填**，与 [`collect_holes`] 同一顺序 ✓）。
pub(crate) fn fill_nth_hole(e: &Expr, k: usize, with: &Expr) -> Expr {
    fn walk(e: &Expr, k: &mut usize, with: &Expr) -> Expr {
        match e {
            Expr::Hole { .. } if *k == 0 => with.clone(),
            Expr::Hole { .. } => {
                *k -= 1;
                e.clone()
            }
            Expr::App {
                fun,
                arg,
                explicit_spine,
                span,
            } => Expr::App {
                fun: Box::new(walk(fun, k, with)),
                arg: Box::new(walk(arg, k, with)),
                explicit_spine: *explicit_spine,
                span: *span,
            },
            other => other.clone(),
        }
    }
    let mut kk = k;
    walk(e, &mut kk, with)
}

/// **只提升「最外层应用脊的直接实参位」的洞**（`f a _ c` ✓；`f (g _)` ✗）。
///
/// 两条理由（都实测过）：
/// 1. **装配**：`by.rs` 的 `assemble` 按「脊上第 k 个洞 ↔ 第 k 个洞层」就地填
///    （`apply Eq.subst _ h` 就是这一支 ✓）——嵌套洞在脊上找不到位置 ⇒ 装配出来的
///    项会**留着生洞** ⇒ 声明变 open exercise ✗（不是假证明，但也不是用户要的 ✓）。
/// 2. **de Bruijn**：洞 binder 靠"调用方在末尾包 λ"收口；嵌在 λ 里的洞会被 λ 的
///    `truncate` 掉 ⇒ 索引指错变量 ✗✗（**静默错项**，最坏的一类）。
pub(crate) fn only_spine_holes(e: &Expr) -> bool {
    let (head, args) = crate::spine::spine_of(e);
    if crate::compile::goals::expr_has_hole(head) {
        return false;
    }
    let mut direct = 0usize;
    for a in &args {
        if matches!(a, Expr::Hole { .. }) {
            direct += 1;
        } else if crate::compile::goals::expr_has_hole(a) {
            return false;
        }
    }
    collect_holes(e).len() == direct
}

/// `by` 引擎装配 `NodeKind::Apply` 用的那一步：**`f` 里的洞由前几个子目标就地填**
/// （洞层排在类型文本最前面 —— 见 [`lift`] 的预压 binder 序 ✓），其余实参照旧**追加** ✓。
///
/// `is_subgoal[i]` = 第 `i` 个实参是子目标（`ApplyArg::SubGoal`）——只有子目标能填洞 ✓。
pub(crate) fn assemble_apply(f: &Expr, args: &[Expr], is_subgoal: &[bool], explicit: bool) -> Expr {
    let mut app = f.clone();
    let mut holes = collect_holes(f).len();
    for (arg_expr, is_sub) in args.iter().zip(is_subgoal) {
        if holes > 0 && *is_sub {
            holes -= 1;
            app = fill_nth_hole(&app, 0, arg_expr);
            continue;
        }
        app = Expr::App {
            fun: Box::new(app),
            arg: Box::new(arg_expr.clone()),
            explicit_spine: explicit,
            span: f.span(),
        };
    }
    app
}

/// `_` 的**唯一** elaborate 入口（`elab.rs` 的 `Expr::Hole` 臂一行调到这）。
///
/// 窗口外 / 没有期望类型 ⇒ 逐字等于今天那条诊断（**零回归面** ✓）。
// 8 参 = 上下文（builder/scope）+ 期望类型两态（`expected_src`/`expected`）+
// 窗口四元（`at`/`arg_index`/`is_spine`/`depth`）。打成一包只会把同一批值从
// 形参挪进结构体、调用点一行没短 ⇒ 照 `project/session.rs:1091` 的先例放行。
#[allow(clippy::too_many_arguments)]
pub(crate) fn elab_hole<'a>(
    builder: &mut EnvBuilder<'a>,
    expected_src: Option<&Expr>,
    scope: &mut ElabScope<'a>,
    univ: &UnivMap<'a>,
    known: &KnownTable,
    hovers: &mut Vec<HoverNode<'a>>,
    ctx: &ElabCtx<'a, '_>,
    span: Span,
) -> Result<ExprPtr<'a>, CompileError> {
    let misplaced = || {
        CompileError::elab(
            ErrorKind::ElabHoleMisplaced,
            "`sorry` is only allowed as the value of an open exercise",
            span,
        )
    };
    let name = hole_name(span);
    // ② 第二趟：binder **已经预压**好了 ⇒ 只引用它（**不压栈** ⇒ 索引不漂 ✓）。
    // ⚠ 这一支**必须排在 `expected_src` 判空之前**：同一个洞在不同的 elaborate 上下文里
    // 可能拿不到期望类型（内层 App 被单独 elaborate 时就是），而**预压的 binder 是权威** ✓。
    if let Some(pos) = scope.find_last(&name) {
        return Ok(builder.mk_var(scope.var_index(pos)));
    }
    if !LIFT.with(|c| c.get()) {
        return Err(misplaced());
    }
    let Some(ty_src) = expected_src else {
        return Err(misplaced());
    };
    // ① 第一趟（探针）：洞的期望类型**就是**它那一层的域（App 臂已按解出的前缀代换好 ✓）
    // ⇒ 压一个 binder，返回它自己的引用（此刻它是最后一个 ⇒ 索引 0 ✓）。
    let ty = elab_hole_type(builder, ty_src, scope, univ, known, hovers, ctx)?;
    scope.push(name.clone(), ty, Some(ty_src.clone()), span);
    FOUND.with(|f| {
        let mut v = f.borrow_mut();
        if !v.iter().any(|(n, _, _)| n == &name) {
            v.push((name.clone(), ty_src.clone(), span));
        }
    });
    Ok(builder.mk_var(0))
}

/// 洞 binder 的**类型**（源级期望类型 → 内核类型）。
///
/// ⚠ **必须能剥 `.{…}` 重试**：期望类型是**签名望远镜**里的域，形如 `Eq.{u} α a b` ——
/// 那个 `u` 是**声明自己的**宇宙参数，调用点没有 ⇒ 直接 elaborate 报
/// `unknown universe level u`（实测：第一版就卡在这，`apply Eq.subst _ h` 仍报 misplaced ✗）。
/// 剥掉后**层级由实参类型反解**（与裸常量同一条 R2b-2(a) 机械 ✓，`Eq α a x` 解出
/// `u = α 的 sort` ✓）。
fn elab_hole_type<'a>(
    builder: &mut EnvBuilder<'a>,
    ty_src: &Expr,
    scope: &mut ElabScope<'a>,
    univ: &UnivMap<'a>,
    known: &KnownTable,
    hovers: &mut Vec<HoverNode<'a>>,
    ctx: &ElabCtx<'a, '_>,
) -> Result<ExprPtr<'a>, CompileError> {
    match elab_expr(builder, ty_src, scope, univ, known, hovers, None, None, ctx) {
        Ok(ty) => Ok(ty),
        Err(first) => match strip_levels(ty_src) {
            Some(stripped) => elab_expr(
                builder, &stripped, scope, univ, known, hovers, None, None, ctx,
            ),
            None => Err(first),
        },
    }
}

/// 剥掉**常量头上的** `.{…}`（`Eq.{u} α a x` ⇒ `Eq α a x`）；没得剥 ⇒ `None`。
///
/// 只走**应用脊 + 常量头**这一形状（期望类型的形状就是它）——其余形状原样不动
/// （`None` ⇒ 报原来那条诊断，**绝不更差** ✓）。
fn strip_levels(e: &Expr) -> Option<Expr> {
    match e {
        Expr::UniverseApp { name, span, .. } => Some(Expr::Ident {
            name: name.clone(),
            span: *span,
        }),
        Expr::App {
            fun,
            arg,
            explicit_spine,
            span,
        } => {
            let fun2 = strip_levels(fun);
            let arg2 = strip_levels(arg);
            (fun2.is_some() || arg2.is_some()).then(|| Expr::App {
                fun: Box::new(fun2.unwrap_or_else(|| (**fun).clone())),
                arg: Box::new(arg2.unwrap_or_else(|| (**arg).clone())),
                explicit_spine: *explicit_spine,
                span: *span,
            })
        }
        _ => None,
    }
}

/// **第一趟（探针）+ 第二趟（预压）**：把 `operand` 里实参位的洞变成「作用域里的 binder」
/// ⇒ 调用方（`infer_type_text_inplace_expected`）包 λ 时，它们**就是类型文本里的 Π 层** ✓。
///
/// 探针失败（洞不在实参位 / 期望类型本身 elaborate 不了）⇒ **什么都不压**（第二趟照旧报
/// 今天那条诊断 ✓，绝不比今天差）。
pub(crate) fn lift<'a>(
    env: &mut InplaceEnv<'_, 'a>,
    operand: &Expr,
    scope: &mut ElabScope<'a>,
    univ: &UnivMap<'a>,
    expected: Option<&Expr>,
    ctx: &ElabCtx<'a, '_>,
) -> Result<(), InplaceFail> {
    if !crate::compile::goals::expr_has_hole(operand) {
        return Ok(());
    }
    if !only_spine_holes(operand) {
        return Ok(());
    }
    // ⚠ **只在有期望类型时**才提升：洞的期望类型由**求解器 + 项自己的期望类型**共同定出；
    // 没有期望类型那一趟（`apply` 的**第一次尝试**）会留下未解位 ⇒ 待定求解器按
    // 「同形状兄弟」合一（`?b := ?a`）⇒ 组装出**错的**项（实测：`A a` 而不是 `A x` ✗）
    // ⇒ 让那一趟**照旧失败**，由**带期望类型**的第二趟接手 ✓（判据就是 `apply Eq.subst _ h`）。
    if expected.is_none() {
        return Ok(());
    }
    let base = scope.len();
    let found = probe(env, operand, scope, univ, expected, ctx);
    scope.truncate(base);
    for (name, ty_src, span) in found {
        // 类型用**丢弃的** hover 表：期望类型来自签名（不是用户写的那处），
        // 探针/重试的中间项不该往真 hover 表里塞行 ✓。
        let mut scratch: Vec<HoverNode<'a>> = Vec::new();
        let ty = elab_hole_type(
            env.builder,
            &ty_src,
            scope,
            univ,
            env.known,
            &mut scratch,
            ctx,
        )
        .map_err(|_| InplaceFail::ElabOperand)?;
        scope.push(name, ty, Some(ty_src), span);
    }
    Ok(())
}

/// 探针那一趟：在**丢弃的**作用域上 elaborate `operand`（洞按压栈提升）并收下它们。
///
/// ⚠ **`expected` 必须与第二趟同款**：洞的期望类型由 App 臂的求解器算（它要用**期望
/// 类型**这条线索反解前缀，路线 ④）——探针不带它 ⇒ 前缀解不出 ⇒ 探针**失败** ⇒
/// 一个洞都收不到 ✗（实测：第一版就是这条，`apply Eq.subst _ h` 仍报 misplaced）。
fn probe<'a>(
    env: &mut InplaceEnv<'_, 'a>,
    operand: &Expr,
    scope: &mut ElabScope<'a>,
    univ: &UnivMap<'a>,
    expected: Option<&Expr>,
    ctx: &ElabCtx<'a, '_>,
) -> Vec<(String, Expr, Span)> {
    FOUND.with(|f| f.borrow_mut().clear());
    let guard = LiftGuard::new(true);
    let mut scratch: Vec<HoverNode<'a>> = Vec::new();
    let _ = quiet_catch(|| {
        elab_expr(
            env.builder,
            operand,
            scope,
            univ,
            env.known,
            &mut scratch,
            None,
            expected,
            ctx,
        )
    });
    drop(guard);
    FOUND.with(|f| std::mem::take(&mut *f.borrow_mut()))
}

/// RAII：进窗口 ⇒ 退出时还原**进入前**的值（可嵌套 ✓）。
struct LiftGuard(bool);

impl LiftGuard {
    fn new(on: bool) -> Self {
        let prev = LIFT.with(|c| c.replace(on));
        LiftGuard(prev)
    }
}

impl Drop for LiftGuard {
    fn drop(&mut self) {
        LIFT.with(|c| c.set(self.0));
    }
}

//! **内核 pp 形态的「操作数类型」不是合法的源级项** —— 识别 + 退回（LAST-P，2026-10-14）✓。
//!
//! ## 它挡的是哪一格
//!
//! `elab::operand_type_expr` 的最后一条路是「问内核要类型文本、再回读成项」——
//! 而内核 pp **只丢第一个隐式实参**（见 `spine::UnfoldAlign::Pp` 的长注释）：
//! `Function.comp`（签名 `{α} -> {β} -> {γ} -> (g : β -> γ) -> (f : α -> β) -> α -> γ`，
//! `implicit_prefix = 3`、**6 层**（末位 `α -> γ` 是匿名 `->` 层 ✓））渲成
//! `Function.comp β γ g f x`（`α` 丢了、`β γ g f` 还在，`x` 落在**最后一层** ✓）。
//!
//! 这条文本**回读成 `Expr` 之后是个 5 实参的脊**，而 `Function.comp` 有 **6 层**
//! ⇒ 旧写法（逐位对齐 `layers[0..]`）把它读成 `α:=β, β:=γ, γ:=g, g:=f, f:=x`
//! ⇒ **结果类型 `α → γ` 变成 `β → g`** ✗（`g` 是一个**函数**，却被当成类型用）
//! ⇒ 内核 `expected a sort, got: Pi ( : β), γ` ✗（实测：`I.4/unit12-solution:351`
//! 的 `And.intro (Exists.intro …) (And.right hx)`）。
//!
//! 而当这个「类型」被求解器当成**前导隐式实参的值**塞进项里
//! （`And.intro` 的 `h1 : a` ⇒ `a := <该类型>`）时，它就**真的进了项** ✗。
//!
//! ## 判据（**可判定、且保守** ✓）
//!
//! 一条脊只有在**同时**满足三条时才算"读不通"：
//! ① 头是**常量**（记法节点/局部名/λ 都是源级形态 ⇒ 不碰 ✗）；
//! ② `implicit_prefix >= 2` —— **只有 `k >= 2` 时 pp 形态才与源级形态不同**
//!    （`k == 1` 时"丢第一个隐式"与"短写"逐字节相同 ⇒ 旧写法照旧 ✓）；
//! ③ 实参个数**正好比层数少一**（pp 丢的就是那一个 ✓）、而**逐位对齐判据**
//!    （`elab::args_fit_layers_in_order`，与旧写法分支**同一份**）判它**不贴合**
//!    ⇒ 旧写法**可证明**是错的 ✓。
//!
//! ⚠ ③ 里**只留**「正好少一个 + 逐位读不通」两条：`layers[1..]` 的"pp 读法"
//! **判不了** ✗ —— 它的首层（被丢的那个隐式）在新对齐里没有实参可代 ⇒ 域里留着
//! fresh 名 `\0soko_p0` ⇒ 逐位判据**必然**判不贴合（实测 `fits_pp=false`）
//! ⇒ 拿它当闸门等于闸门永远不开 ✗（第一版就是这么写的，一处都没咬住）。
//!
//! ⇒ 只在这一格把该类型当作「拿不到类型」（`None`）✓ —— 求解器于是走
//! **路线②（期望类型）**，那里给的是**源级**类型（`(g (f x)) = y`）✓。
//! **不是放宽** ✗：旧写法能读通的脊（`And.intro a b ha hb`、`Set.image α f A`）
//! 判据③ 不过 ⇒ 一个字不动 ✓。
//!
//! ## 第二条：`Prop` 位**不借兄弟的值**（E19 的 defaulting）
//!
//! `meta::MetaCtx::default_unresolved` 的 E19 选择规则是「未解的位借**声明类型
//! 同形**的已解兄弟的值」。它的设计场景是**类型参数**（`Set.Equiv {α β : Type}`，
//! 两个位的域都是 `Type` ✓）。但 `And.intro {a b : Prop}` 的两个位**域也都是
//! `Prop`** ⇒ `same_shape` 判同形 ⇒ `a` 借走 `b` 的值 ✗ —— 而**命题是内容、不是
//! 形状**：两个独立的 `Prop` 位之间借用**不是推理、是编造** ✗
//! （实测：`a := b := (g (f x) = y)`，把 `(f x) ∈ f''A` 那一半丢了）。
//! ⇒ [`borrowable`] 在**同一行**里加一条：`Prop` 位不借 ✓（行数净增 0 ✓）。
//!
//! Lean 4 对照：`Elab/App.lean:1455` 的 `consumeImplicits` 给前导隐式 binder 建
//! 元变量，**未解就是错**（`synthesizeSyntheticMVars` 报 "don't know how to
//! synthesize implicit argument"），**从不借兄弟的值** ✗ ⇒ 我们的偏离（E19）必须
//! 收窄到"只在类型参数位上"才站得住 ✓。
//!
//! ## 闸门
//!
//! `SOKO_PP_SHAPE=0|off` ⇒ [`gate_on`] 恒 `false`（= 今天的行为 ✓，反向验证用 ✓）。
//! `SOKO_METAVAR=0` 照旧关掉整条 defaulting（[`borrowable`] 一次都不进 ✓）。

use super::elab::{ElabCtx, ElabScope, InplaceEnv, KnownTable};
use crate::ast::SortKind;
use crate::Expr;
use sokonanoda::builder::EnvBuilder;

/// **本格的闸门**（2026-10-14）：`SOKO_PP_SHAPE=0|off` ⇒ 恒 `false`
/// （= 今天的行为 ✓，反向验证用 ✓）。
pub(crate) fn gate_on() -> bool {
    !matches!(
        std::env::var("SOKO_PP_SHAPE").ok().as_deref(),
        Some("0") | Some("off")
    )
}

/// **E19 defaulting 的借用判据**（与 `spine::same_shape` **同一行**调用 ✓）：
/// 同形 **且** 待解位不是 `Prop` ✓。
///
/// 为什么 `Prop` 位不借：见模块头的「第二条」✓。`same_shape` 已经保证两边同形，
/// 所以只看 `ti` 一侧就够 ✓。
pub(crate) fn borrowable(ti: &Expr, tj: &Expr) -> bool {
    !matches!(
        ti,
        Expr::Sort {
            sort: SortKind::Prop,
            ..
        }
    ) && crate::spine::same_shape(ti, tj)
}

/// **每个实参的类型**（路线 ① 的原料：`arg_tys[i]` 对应第 `k + i` 层）✓ ——
/// 从 `elab.rs` 原样搬来（行数只减不增 ✓），只多一条 pp 形态判据 ✓。
///
/// 局部变量**优先取书写类型**（零内核调用，且不会像 pp 那样丢隐式实参）；
/// **pp 形态读不通的常量脊** ⇒ 当作"拿不到类型"（`None`）✓：它**不是**一个合法的
/// 源级项（拿它当实参会被旧写法错位装配）⇒ 交回路线②（期望类型，那里给的是
/// 源级类型 ✓）。闸门 `SOKO_PP_SHAPE=0|off` ✓。
pub(crate) fn arg_types<'a>(
    args: &[&Expr],
    ctx: &ElabCtx<'a, '_>,
    scope: &ElabScope<'a>,
    known: &KnownTable,
    builder: &mut EnvBuilder<'a>,
) -> Vec<Option<Expr>> {
    let mut arg_tys: Vec<Option<Expr>> = Vec::with_capacity(args.len());
    for a in args {
        let t = super::elab::operand_type_expr(
            ctx,
            scope,
            a,
            Some(&mut InplaceEnv {
                builder: &mut *builder,
                known,
            }),
        );
        arg_tys.push(match t {
            Some(t) if gate_on() && unreadable(&t, ctx, scope, known, 0) => None,
            other => other,
        });
    }
    arg_tys
}

/// 类型里有没有**读不通的 pp 形态常量脊**（有界递归 ✓）。
fn unreadable(
    t: &Expr,
    ctx: &ElabCtx<'_, '_>,
    scope: &ElabScope<'_>,
    known: &KnownTable,
    depth: u32,
) -> bool {
    if depth > 8 {
        return false;
    }
    if spine_unreadable(t, ctx, scope, known) {
        return true;
    }
    match t {
        Expr::App { fun, arg, .. } => {
            unreadable(fun, ctx, scope, known, depth + 1)
                || unreadable(arg, ctx, scope, known, depth + 1)
        }
        Expr::Arrow {
            domain, codomain, ..
        } => {
            unreadable(domain, ctx, scope, known, depth + 1)
                || unreadable(codomain, ctx, scope, known, depth + 1)
        }
        Expr::Forall { binders, body, .. } | Expr::Lambda { binders, body, .. } => {
            binders.iter().any(|b| {
                b.ty.as_deref()
                    .is_some_and(|d| unreadable(d, ctx, scope, known, depth + 1))
            }) || unreadable(body, ctx, scope, known, depth + 1)
        }
        Expr::Notation { lhs, rhs, .. } => {
            lhs.as_deref()
                .is_some_and(|e| unreadable(e, ctx, scope, known, depth + 1))
                || rhs
                    .as_deref()
                    .is_some_and(|e| unreadable(e, ctx, scope, known, depth + 1))
        }
        Expr::Let {
            binder, val, body, ..
        } => {
            binder
                .ty
                .as_deref()
                .is_some_and(|d| unreadable(d, ctx, scope, known, depth + 1))
                || unreadable(val, ctx, scope, known, depth + 1)
                || unreadable(body, ctx, scope, known, depth + 1)
        }
        _ => false,
    }
}

/// [`unreadable`] 的**单条脊**判据（三条合取 ✓，见模块头）。
fn spine_unreadable(
    t: &Expr,
    ctx: &ElabCtx<'_, '_>,
    scope: &ElabScope<'_>,
    known: &KnownTable,
) -> bool {
    let (head, args) = crate::spine::spine_of(t);
    let raw = match head {
        Expr::Ident { name, .. } | Expr::UniverseApp { name, .. } => name.as_str(),
        _ => return false,
    };
    let Ok(canonical) =
        super::elab::resolve_known(known, ctx.ns, raw, head.span(), scope.local_names())
    else {
        return false;
    };
    let Some(declared) = known.get(&canonical) else {
        return false;
    };
    if declared.implicit_prefix() < 2 {
        return false;
    }
    let Some(ty_text) = declared.signature() else {
        return false;
    };
    let Some((layers, _)) = super::implicit::telescope(ty_text) else {
        return false;
    };
    // ③-a **正好少一个**（pp 丢的就是那一个）✓。
    if args.len() + 1 != layers.len() {
        return false;
    }
    // ③-b **旧写法（逐位对齐 `layers[0..]`）可证明读不通** ✓。
    !super::elab::args_fit_layers_in_order(&layers[..args.len()], &args, ctx, scope, None)
}

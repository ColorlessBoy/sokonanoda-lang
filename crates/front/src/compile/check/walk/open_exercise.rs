//! **开放练习入环境**（2026-10-21，对齐 Lean 4 的 `sorry`）。
//!
//! Lean 4 把 `sorry` 展开成 `sorryAx`，声明本身照样 `addDecl` ⇒ 后续声明
//! **可以引用它**。本仓原先「open 声明不进环境」⇒ 引用未解练习直接
//! `elab-unknown-identifier`、整条声明的 goal state 消失 ✗。这里用**同签名
//! axiom** 表示"这个名字有类型、值还没证"，并给引用处一条 warning（不是 error）。
//!
//! 拆成独立模块的理由：`walk.rs` 已超过软线，按 `docs-lint` 判据 ⑨ 只许减不许增。

use std::collections::HashSet;

use sokonanoda::builder::EnvBuilder;
use sokonanoda::env::{Declar, DeclarInfo};
use sokonanoda::expr::Expr as KernelExpr;
use sokonanoda::util::{ExprPtr, NamePtr};

use crate::compile::elab::{KnownName, KnownTable};
use crate::compile::event::CompileOutput;
use crate::compile::warning::{CompileWarning, WarningKind};
use crate::{Expr, Span};

/// 把签名的内核项按**同签名 axiom** 装进环境，并登记「这个练习已进环境」。
///
/// ⚠ 调用点必须在签名 elaborate **成功之后**；签名不过的声明仍然 Failed、
/// 不进环境（G-01 的红线不变）。
pub(super) fn add_axiom<'a>(
    builder: &mut EnvBuilder<'a>,
    open_exercises: &mut HashSet<NamePtr<'a>>,
    entered: &crate::judge::EnteredNames,
    name: &str,
    probe: &Declar<'a>,
) -> Result<(), String> {
    let info = *probe.info();
    let name_ptr = builder.name_from_str(name);
    let axiom = Declar::Axiom {
        info: DeclarInfo {
            name: name_ptr,
            ..info
        },
    };
    builder.add_declar(axiom)?;
    open_exercises.insert(name_ptr);
    entered.borrow_mut().insert(name.to_string());
    Ok(())
}

/// **受信前缀里的开放练习**：只 elaborate 签名、按 axiom 复原进环境。
///
/// 信任前缀是"重放环境"的，不能因为不重查就把 axiom 漏掉 —— 否则前缀里
/// 一次引用就退回 `unknown identifier` ✗。
#[allow(clippy::too_many_arguments)]
pub(super) fn restore_trusted<'a>(
    builder: &mut EnvBuilder<'a>,
    open_exercises: &mut HashSet<NamePtr<'a>>,
    entered: &crate::judge::EnteredNames,
    known: &KnownTable,
    name: &str,
    universe: &[String],
    ty: &Expr,
    elab_ctx: &crate::compile::elab::ElabCtx<'a, '_>,
) {
    let mut hovers = Vec::new();
    if let Ok(decl) =
        crate::compile::elab::build_axiom(builder, name, universe, ty, known, &mut hovers, elab_ctx)
    {
        let (decl, _fresh) = crate::compile::check::level_exit::discharge_declar(builder, decl);
        let _ = add_axiom(builder, open_exercises, entered, name, &decl);
    }
}

/// 把开放练习登记进 `known`（后续声明据此解析名字并做隐式实参求解）。
///
/// ⚠ 与 [`add_axiom`] 分开：`ElabCtx` 在开放练习分支里**借住** `known`
/// （`build_redundant_probes` 还要用），所以这一步必须等 `elab_ctx` 的最后一个
/// 使用点之后才能做。
pub(super) fn remember_known(known: &mut KnownTable, name: &str, universe: &[String], ty: &Expr) {
    known.insert(
        name.to_string(),
        KnownName::Decl {
            universes: universe.to_vec(),
            implicit_prefix: crate::compile::elab::leading_implicit_prefix(ty),
            explicit_arity: crate::compile::elab::explicit_arity(ty),
            signature: Some(crate::proof::decl_signature(ty)),
        },
    );
}

/// 这条声明（类型 / 值）里引用了哪些**开放练习**（按规范名、去重）。
fn uses_open_exercises<'a>(declar: &Declar<'a>, open: &HashSet<NamePtr<'a>>) -> Vec<NamePtr<'a>> {
    let mut hits = Vec::new();
    collect_open_consts(declar.info().ty, open, &mut hits);
    // ⚠ 不能用 `Declar::value()`：它对 `Theorem` 返回 `None`（内核只在
    // `Definition`/`Opaque` 上取 `val`）⇒ 必须自己匹配 `Theorem` 的证明体 ✗。
    match declar {
        Declar::Theorem { val, .. }
        | Declar::Definition { val, .. }
        | Declar::Opaque { val, .. } => {
            collect_open_consts(*val, open, &mut hits);
        }
        _ => {}
    }
    hits
}

/// 引用了开放练习 ⇒ 发一条 **warning**（不是 error）：说明这条证明仍然
/// 依赖未完成的练习，但判定/目标状态照常工作。
pub(super) fn warn_if_uses_open_exercise<'a>(
    builder: &EnvBuilder<'a>,
    open: &HashSet<NamePtr<'a>>,
    out: &mut CompileOutput,
    declar: &Declar<'a>,
    span: Span,
    cmd: usize,
) {
    let hits = uses_open_exercises(declar, open);
    if hits.is_empty() {
        return;
    }
    let names = hits
        .iter()
        .map(|name| format!("`{}`", builder.name_to_string(*name)))
        .collect::<Vec<_>>()
        .join("、");
    out.push_warning(
        cmd,
        CompileWarning {
            kind: WarningKind::UsesOpenExercise,
            message: format!(
                "这条声明引用了还没证明的练习 {names}。Lean 4 的 `sorry` 声明照样能进环境，\
                 所以这里不是错误；但整条证明仍然依赖未完成的练习，补完它们才算真正证完。"
            ),
            span,
        },
    );
}

/// 扫描内核项里的**常量出现点**，把命中 `open` 的名字收进 `hits`（去重）。
///
/// 用途：`uses-open-exercise` warning 的**精确**判据。拿内核项而不是源文本，
/// 是因为局部 binder 可能与练习同名（文本扫描会假阳性）；内核项里的
/// `Expr::Const` 已经过了作用域解析 ✓。
fn collect_open_consts<'a>(
    expr: ExprPtr<'a>,
    open: &HashSet<NamePtr<'a>>,
    hits: &mut Vec<NamePtr<'a>>,
) {
    match *expr {
        KernelExpr::Const { name, .. } => {
            if open.contains(&name) && !hits.contains(&name) {
                hits.push(name);
            }
        }
        KernelExpr::App { fun, arg, .. } => {
            collect_open_consts(fun, open, hits);
            collect_open_consts(arg, open, hits);
        }
        KernelExpr::Pi {
            binder_type, body, ..
        }
        | KernelExpr::Lambda {
            binder_type, body, ..
        } => {
            collect_open_consts(binder_type, open, hits);
            collect_open_consts(body, open, hits);
        }
        KernelExpr::Let { data, .. } => {
            collect_open_consts(data.binder_type, open, hits);
            collect_open_consts(data.val, open, hits);
            collect_open_consts(data.body, open, hits);
        }
        KernelExpr::Proj { structure, .. } => collect_open_consts(structure, open, hits),
        KernelExpr::Var { .. }
        | KernelExpr::Sort { .. }
        | KernelExpr::StringLit { .. }
        | KernelExpr::NatLit { .. }
        | KernelExpr::Meta { .. } => {}
    }
}

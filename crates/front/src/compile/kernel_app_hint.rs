//! **P2-2 (c)**：零元隐式记法（`∅`）作内核推迟位时按域解（见 [`try_hinted_const`]）。
//! 拆自 [`super::kernel_app`]（docs-lint ⑨：源码文件 ≤ 500 行）。

use super::elab::{elab_expr, ElabCtx, ElabScope, HoverNode, KnownName, KnownTable, UnivMap};
use super::kernel_app::{count, env_on, loose_bvars, Count};
use super::kernel_app_postpone as postpone;
use crate::Expr;
use sokonanoda::builder::EnvBuilder;
use sokonanoda::elab_app::{AppOutcome, AppRequest};
use sokonanoda::env::EnvLimit;
use sokonanoda::util::ExprPtr;

/// **P2-2 (c)**：零元隐式记法（`∅` ⇒ `Set.empty`）作外层的**推迟位**——源级期望缺席、只有内核给的域
/// （[`postpone::ExpectedHint`]）⇒ 头 `canonical` 零显式实参走内核 `elab_app`，期望 = 域（Lean
/// `elabAndAddNewArg` 本来就按域 elaborate）。没有正对本节点的提示 / 闸门关 / 解不出 ⇒ `None`（调用方照报
/// 老诊断，外层随之回落，同今天）。
#[allow(clippy::too_many_arguments)]
pub(crate) fn try_hinted_const<'a>(
    builder: &mut EnvBuilder<'a>,
    canonical: &str,
    span: crate::Span,
    scope: &mut ElabScope<'a>,
    univ: &UnivMap<'a>,
    known: &KnownTable,
    hovers: &mut Vec<HoverNode<'a>>,
    ctx: &ElabCtx<'a, '_>,
) -> Option<ExprPtr<'a>> {
    if !ctx.options.dialect.strict() || !env_on() || !postpone::has_hint(span, scope) {
        return None;
    }
    let Some(KnownName::Decl { universes, .. }) = known.get(canonical) else {
        return None;
    };
    if !universes.is_empty() {
        return None;
    }
    let hint = postpone::take_hint(span, scope)?;
    let (mark, depth) = (hovers.len(), scope.len());
    let head = Expr::Ident {
        name: canonical.to_string(),
        span,
    };
    let solved = elab_expr(builder, &head, scope, univ, known, hovers, None, None, ctx)
        .ok()
        .filter(|h| matches!(**h, sokonanoda::expr::Expr::Const { .. }))
        .and_then(|head_t| {
            let expected = builder.import_owned(&hint)?;
            let binder_tys = scope.kernel_tys().to_vec();
            let limit = EnvLimit::ByIndex(builder.declaration_count());
            let req = AppRequest {
                binder_tys: &binder_tys,
                head: head_t,
                implicit_prefix: 0,
                explicit_args: &[],
                expected: Some(expected),
            };
            match builder.with_env(|ef| ef.elab_app(limit, &req)) {
                AppOutcome::Solved(t) => builder.import_owned(&t),
                _ => None,
            }
        })
        .filter(|out| loose_bvars(*out, &mut Default::default()) as usize <= scope.len());
    scope.truncate(depth);
    match solved {
        Some(out) => {
            count(Count::Solved);
            Some(out)
        }
        None => {
            count(Count::ArgElab);
            hovers.truncate(mark);
            None
        }
    }
}

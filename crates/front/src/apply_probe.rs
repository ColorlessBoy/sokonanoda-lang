//! `apply` 的**探测/守卫**（自 `by.rs` 拆出：那个文件在行数棘轮红线上，`AGENTS.md` 硬规则 5）。
//! 只装**本批新增/搬迁的判定辅助**；不含任何 `render_expr` 调用（见 `mentions_soko_level`）。

use crate::ast::{Binder, Expr, SortKind, Tactic};
use crate::by::{check_ast_of_text, def_shape, is_concrete_level, level_hint_of};
use crate::compile::elab::DefTable;
use crate::compile::CompileOptions;
use crate::judge::GoalBinderSpec;
use crate::proof::parse_expr_text;
use crate::span::Span;
use crate::spine::{head_and_args, peel_pi_delta, unify_spine};

/// **1a 的开关**：`1`/未设 = 失败边带目标重试；`0` = 逐字节回到改动前。
pub(crate) fn apply_goal_first() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var("SOKO_APPLY_GOAL_FIRST").ok().as_deref() != Some("0"))
}

/// **1a**：这份类型文本「能不能直接用」——读得回来、剥得开、结论与目标对得上。
/// 只做判定、无副作用（真正的剥层/合一在主路径上照旧跑一遍）。
pub(crate) fn apply_type_fits(
    text: &str,
    goal: &Expr,
    defs: &DefTable,
    binders: &[GoalBinderSpec],
    prefix_src: &str,
    options: &CompileOptions,
) -> bool {
    let Ok(parsed) = parse_expr_text(text) else {
        return false;
    };
    // **先不买 hint**（`level_hint_of` 可能落到 `judge_infer` ⇒ 整份前缀重编）——
    // 成功路径上这一趟必须便宜；hint 只在"无 hint 判不过"时才试（失败边才付钱）。
    fits(parsed.clone(), None, goal, defs) || {
        let hint = level_hint_of(&parsed, defs, binders, prefix_src, options);
        fits(parsed, hint, goal, defs)
    }
}

/// 剥层 + 合一 + `u_soko` 守卫；`hint` 为 `None` 时不对头做 delta 展开。
fn fits(mut t: Expr, hint: Option<String>, goal: &Expr, defs: &DefTable) -> bool {
    let mut layers: Vec<(String, Expr)> = Vec::new();
    let codomain = loop {
        let Some(pi) = peel_pi_delta(&t, defs, hint.as_deref()) else {
            break t;
        };
        layers.push((pi.name, pi.domain));
        t = pi.body;
    };
    // **1b 守卫**：归一出来的自由层名（`u_soko*`）**不许**外泄 —— 结论里出现，或
    // 某个**会变成子目标**的层（非宇宙域）的域里出现 ⇒ 这份文本不可用（走今天的失败链）。
    if mentions_soko_level(&codomain) {
        return false;
    }
    if layers
        .iter()
        .any(|(name, dom)| !name.is_empty() && !is_universe_domain(dom) && mentions_soko_level(dom))
    {
        return false;
    }
    unify_spine(&codomain, goal, &layers).is_some()
}

/// 这一层的域是不是**宇宙**（`Type` / `Type n` / `Sort u`）——`Prop` **不算**。
/// 用途见 `apply_tactic` 的类型参数判定：宇宙层不可能有子目标。
pub(crate) fn is_universe_domain(domain: &Expr) -> bool {
    matches!(
        domain,
        Expr::Sort { sort, .. } if !matches!(sort, crate::SortKind::Prop)
    )
}

/// **1a 的失败边重试**：文本「读不回来 / 结论对不上」就用**目标当期望类型**重试一次；
/// 只有新文本**真的能用**才替换 ⇒ 今天能过的输入逐字节不变。
pub(super) fn retry_with_goal(
    text: String,
    goal: &Expr,
    defs: &DefTable,
    binders: &[GoalBinderSpec],
    prefix_src: &str,
    options: &CompileOptions,
    retry: impl FnOnce() -> Option<String>,
) -> String {
    if !apply_goal_first() || apply_type_fits(&text, goal, defs, binders, prefix_src, options) {
        return text;
    }
    match retry() {
        Some(next) if apply_type_fits(&next, goal, defs, binders, prefix_src, options) => next,
        _ => text,
    }
}

/// 「读不回来」的文案**按病因分**：含 `?u.` ⇒ 层/参数没定下来（钉层或换写法）；
/// 否则是 D5 的原形状（实参没写全）。
pub(super) fn unreadable_message(term: &str, ty: &str) -> String {
    if ty.contains("?u.") {
        format!(
            "`apply {term}` 判不了：它的类型文本**读不回来** —— 这条引理的**宇宙层/参数**\
             还没定下来（类型里还有 `?u.<id>` 占位）。可以显式钉住（例如 `{term}.{{1}}`），\
             或换成把等式/前提写出来的写法再试。"
        )
    } else {
        format!(
            "`apply {term}` 判不了：它的类型文本**读不回来**（内核给的答案里还有没定下来的\
             占位）——通常是实参没写全。把 `{term}` 的实参补齐（`apply {term} …`）再试一次。"
        )
    }
}

/// `expr` 里有没有**自由层名**（`u_soko*`）—— 归一出来的名字不许外泄进结论/子目标域。
/// 不做文本渲染（那是记法显示接口的绕过点），只做结构扫描。
fn mentions_soko_level(expr: &Expr) -> bool {
    fn any_binder(binders: &[Binder]) -> bool {
        binders
            .iter()
            .any(|b| b.ty.as_deref().is_some_and(mentions_soko_level))
    }
    match expr {
        Expr::Sort {
            sort: SortKind::Level(name),
            ..
        } => name.starts_with("u_soko"),
        Expr::Sort { .. }
        | Expr::Num { .. }
        | Expr::Ident { .. }
        | Expr::Hole { .. }
        | Expr::By { .. } => false,
        Expr::UniverseApp { levels, .. } => levels.iter().any(|l| l.starts_with("u_soko")),
        Expr::App { fun, arg, .. } => mentions_soko_level(fun) || mentions_soko_level(arg),
        Expr::SetLiteral { elements, .. } | Expr::AnonCtor { elements, .. } => {
            elements.iter().any(mentions_soko_level)
        }
        Expr::Lambda { binders, body, .. } | Expr::Forall { binders, body, .. } => {
            any_binder(binders) || mentions_soko_level(body)
        }
        Expr::Arrow {
            domain, codomain, ..
        } => mentions_soko_level(domain) || mentions_soko_level(codomain),
        Expr::Plus { lhs, rhs, .. } => mentions_soko_level(lhs) || mentions_soko_level(rhs),
        Expr::Let {
            binder, val, body, ..
        } => {
            binder.ty.as_deref().is_some_and(mentions_soko_level)
                || mentions_soko_level(val)
                || mentions_soko_level(body)
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            mentions_soko_level(scrutinee)
                || arms.iter().any(|a| {
                    a.guard.as_ref().is_some_and(mentions_soko_level)
                        || mentions_soko_level(&a.body)
                })
        }
        Expr::Notation { lhs, rhs, .. } => {
            lhs.as_deref().is_some_and(mentions_soko_level)
                || rhs.as_deref().is_some_and(mentions_soko_level)
        }
        Expr::NamedArg { value, .. } => mentions_soko_level(value),
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn level_hint_of_inplace<'a>(
    expr: &Expr,
    defs: &DefTable,
    binders: &[GoalBinderSpec],
    prefix_src: &str,
    options: &CompileOptions,
    env: &mut Option<&mut crate::compile::elab::InplaceEnv<'_, 'a>>,
    ctx: Option<&crate::compile::elab::ElabCtx<'a, '_>>,
    binder_srcs: &[(String, Expr)],
) -> Option<String> {
    let (name, args) = head_and_args(expr)?;
    let (params, universes) = def_shape(defs, name)?;
    if universes == 1 && binder_srcs.len() == binders.len() {
        if let (Some(ctx), Some(first)) = (ctx, args.first()) {
            if let Some(e) = crate::compile::elab::InplaceEnv::reborrow(env) {
                let point_form = args.len() >= params;
                let text = crate::compile::elab::infer_type_text_inplace(
                    e,
                    ctx,
                    binder_srcs,
                    first,
                    binders.len(),
                    None,
                )
                .ok()
                .and_then(|first_ty| {
                    if point_form {
                        // 点式：首实参**自己**的类型就是那个 `Sort u`。
                        Some(first_ty)
                    } else {
                        // 记法形态：首实参是**项** ⇒ 还要问「那个类型的 sort」；
                        // 第二问的输入是**文本** ⇒ 回读成 `#check` 的 AST 再就地 ✓。
                        let ast = check_ast_of_text(&first_ty)?;
                        crate::compile::elab::infer_type_text_inplace(
                            e,
                            ctx,
                            binder_srcs,
                            &ast,
                            binders.len(),
                            None,
                        )
                        .ok()
                    }
                });
                if let Some(text) = text {
                    if let Some(level) = crate::compile::elab::level_text_of_sort(&text) {
                        if is_concrete_level(&level) {
                            crate::judge::stats::INPLACE_LEVEL_HINT_USED
                                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            return Some(level);
                        }
                    }
                }
            }
        }
    }
    // **任何一处不成立 ⇒ 原样回落** ✓（逐字节等同改动前 ✓）。
    crate::judge::stats::INPLACE_LEVEL_HINT_FALLBACK
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    //
    // **诊断**（`SOKO_INPLACE_WHY=1` ✓）：把"为什么没走成就地"分类打出来 ——
    // 否则回落之后 `#[track_caller]` 只会指向本行 ✗，看不出是"形态不支持"还是
    // "就地答不出" ✓（两者出路完全不同 ✗）。
    if crate::judge::inplace_why_enabled() {
        let why = if universes != 1 {
            "not-one-universe"
        } else if binder_srcs.len() != binders.len() {
            "binder-len-mismatch"
        } else if ctx.is_none() {
            "no-ctx"
        } else if crate::compile::elab::InplaceEnv::reborrow(env).is_none() {
            "no-env"
        } else {
            // 点式与记法**共用这一档**（T3-B1 ③ 起记法不再单独回落 ✓）。
            "inplace-failed"
        };
        eprintln!("BY_LEVEL_HINT_FALLBACK why={why}");
    }
    level_hint_of(expr, defs, binders, prefix_src, options)
}

/// **`apply` 的早报守卫**（2026-10-21 用户）：direct 匹配不上、且被应用项里没有洞
/// ⇒ 用**完整 kernel** 检查 `term : f_ty`。`infer_type_text` 走 `InferOnly`
/// （不查实参类型），所以 `@Eq.subst … (Eq.symm h)` 能拿到一个看似正常的类型；
/// 这里补一次 `judge_terms`（完整声明判定）⇒ 类型不匹配当场落在 `apply` 上。
///
/// 只在**失败边**跑，且只认 `Mismatch`：`judge_terms` 自己 elaborate 不了
/// （`Error`）时不拦，照旧走原来的失败链（宁可保持今天的报错，也不引入假红）。
pub(super) fn checked_apply_mismatch(
    expr: &Expr,
    term: &str,
    f_ty: &str,
    spec: &crate::judge::OpenGoalSpec,
    prefix_src: &str,
    options: &CompileOptions,
) -> Option<(String, String)> {
    if !crate::compile::hole::collect_holes(expr).is_empty() {
        return None;
    }
    let mut check_spec = spec.clone();
    check_spec.ty = f_ty.to_string();
    match crate::judge::judge_terms(prefix_src, options, &check_spec, &[term]).first() {
        Some(crate::judge::Judgement::Mismatch {
            expected, actual, ..
        }) => {
            // **函数层不早报**：`expected/actual` 是 `∀ …` 时，错位通常来自
            // 部分应用的未解隐式（`@Set.univ` / `Set.Equiv.mk` 这些都合法）——
            // 完整检查会误判；让内核终审兜底，早报只处理**非函数层**的实参错。
            if is_forall_text(expected) || is_forall_text(actual) {
                None
            } else {
                Some((expected.clone(), actual.clone()))
            }
        }
        _ => None,
    }
}

fn is_forall_text(text: &str) -> bool {
    crate::proof::parse_expr_text(text)
        .map(|e| matches!(e, Expr::Forall { .. }))
        .unwrap_or(false)
}

/// `·` 块里**直接写了** `sorry` 的 span（admit 边界；不递归进嵌套块）。
pub(super) fn direct_sorry_span(tactics: &[Tactic]) -> Option<Span> {
    tactics
        .iter()
        .find_map(|t| matches!(t, Tactic::Sorry { .. }).then(|| t.span()))
}

/// direct 匹配成功时还要不要补完整检查：**完全应用**，或带 `@` / 命名实参的
/// 应用脊（这些形状的坏实参不会被隐式反解兜住）；部分应用（`Eq.trans h`）会
/// 因类型里还剩未解隐式而误判 ⇒ 不查。
pub(super) fn needs_checked_apply(expr: &Expr, fully_applied: bool) -> bool {
    matches!(expr, Expr::App { .. }) && (fully_applied || has_explicit_spine_or_named(expr))
}

fn has_explicit_spine_or_named(expr: &Expr) -> bool {
    match expr {
        Expr::App {
            fun,
            arg,
            explicit_spine,
            ..
        } => {
            *explicit_spine || has_explicit_spine_or_named(fun) || has_explicit_spine_or_named(arg)
        }
        Expr::NamedArg { value, .. } => has_explicit_spine_or_named(value),
        _ => false,
    }
}

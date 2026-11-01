//! **P2-12 S2**：内核路径的**报错出口**（`reports/2026-10-31-p2-12-design.md` Q3 表）。
//!
//! 本模块是**纯函数**：吃 [`AppOutcome`] + 一份**源级上下文**（都是渲染好的文本/span），吐一个
//! [`Report`]。**S2 不接线**（`kernel_app::finish` 仍回落老路），S3 才把 `Report::Error` 交出去。
//! 这样 S3 的 diff 只是"换一个出口"，文案对不对可以在这里逐条钉死。
//!
//! ## 三条硬约束
//!
//! 1. **不印内核给的名字**：内核只报下标/种类（`UnsolvedSite::Implicit { index }`），源级名从
//!    `ty_text` 经 [`super::implicit_msg::layer_name`] 取；`ArgMismatch` / `ExpectedMismatch` 的两边类型
//!    里可能还带着本会话元变量（id ≥ 2^30）⇒ 一律过 [`sanitize`] 兜底（`\0soko_*` / `?<数字>` 换成
//!    「一个没解出来的形参」），并把"命中过"记下来（`debug_assert!` 用）。
//! 2. **与今天逐字节相同**：实参比显式层多（`n_args > explicit_layers`）时**必须先**报老路那句
//!    `strict_rejection_message` —— 老路（`elab.rs:4780`）也是先查这一条。t1/c2 的既有断言就是它。
//! 3. **判不了 ≠ 判成否**：只含 `External` 的 `Unsolved` 是**外层**应用的事 ⇒ [`Report::Defer`]；
//!    `ExpectedMismatch` 只是"期望没传播上" ⇒ 交回给调用方**不带期望重解**（S4 的活），这里不报。
//!
//! 写明的例外（仍走老路，S3 只计数）见 [`Report::Fallback`]：`Unsupported`、`NeedArgs`、`ArgMismatch`
//! 的正文（要内核那台打印器才算逐字节相同，留给 P2-13 之后的另开一刀）。

// S2 = **先不接线**（`reports/2026-10-31-p2-12-design.md` Q6 表）：S3 才把 `Report::Error` 交出去。
// 在接线之前这份模块没有生产调用者 ⇒ 只在本文件内放开 `dead_code`（不放松别处）。
#![allow(dead_code)]

use super::error::{CompileError, ErrorKind};
use super::implicit_msg::{layer_name, strict_rejection_message, unsolved_message};
use crate::Span;
use sokonanoda::elab_app::{AppOutcome, UnsolvedSite};

/// 报错需要的**源级上下文**（纯数据 ⇒ 单测不必起编译器）。
pub(crate) struct ReportInput<'a> {
    /// 头名的渲染（`render_msg(head)`）。
    pub head_text: &'a str,
    /// 整条应用的渲染（`render_msg(expr)`）。
    pub whole_text: &'a str,
    /// 声明的签名文本（源级形参名从这里取）。
    pub ty_text: &'a str,
    /// 前导隐式位数（`implicit_prefix`）。
    pub k: usize,
    /// 签名总层数（含隐式与显式）。
    pub total_layers: usize,
    /// 显式层数。
    pub explicit_layers: usize,
    /// 位置实参个数。
    pub n_args: usize,
    /// 整条应用的 span。
    pub span: Span,
    /// 各实参的 span（`Blocked { arg }` 要落到那个实参上）。
    pub arg_spans: &'a [Span],
}

/// 出口判定。
pub(crate) enum Report {
    /// 报这条错（S3 接线时直接 `return Err(e)`）。
    Error(CompileError),
    /// **写明的例外**：仍走老路，并计数（键 = 这个原因）。
    Fallback(&'static str),
    /// 内层不报：剩下的位属于**外层**应用（设计 Q3 ③），由外层用**它自己的** binder 名报。
    Defer,
}

/// 见模块文档。**顺序即优先级**：拒绝文案 → `Unsolved` 四种 → 其余例外。
pub(crate) fn report(outcome: &AppOutcome, input: &ReportInput<'_>) -> Report {
    // 规则 1（与老路 `elab.rs:4780` 同一个先手）：位置实参比显式层多。
    if input.n_args > input.explicit_layers {
        return Report::Error(CompileError::elab(
            ErrorKind::ElabImplicitArgumentUnsolved,
            sanitized(strict_rejection_message(
                input.head_text,
                input.whole_text,
                input.ty_text,
                input.k,
                input.total_layers,
                input.explicit_layers,
                input.n_args,
            )),
            input.span,
        ));
    }
    match outcome {
        // 内核解出了却走到报错出口 ⇒ 不是判定问题，交回（S3 不该出现；出现就别拦）。
        AppOutcome::Solved(_) => Report::Defer,
        AppOutcome::Unsolved { sites } => unsolved(sites, input),
        // 期望只是提示（Lean `propagateExpectedType` 失败 = 不传播）⇒ 不带期望重解一次，别报。
        AppOutcome::ExpectedMismatch { .. } => Report::Fallback("expected_retry"),
        // 正文要与今天逐字节相同 = 要内核那台打印器（`render_value_for_def_eq_error`）⇒ S3 保留回落。
        AppOutcome::ArgMismatch { .. } => Report::Fallback("argmismatch_text"),
        AppOutcome::Unsupported(why) => Report::Fallback(why),
        AppOutcome::NeedArgs { .. } => Report::Fallback("need_args"),
    }
}

/// `Unsolved` 的四种位（顺序同 [`AppOutcome::Unsolved`] 的约定：隐式 → 层 → 外部 → blocked）。
fn unsolved(sites: &[UnsolvedSite], input: &ReportInput<'_>) -> Report {
    let has_implicit = sites
        .iter()
        .any(|s| matches!(s, UnsolvedSite::Implicit { .. }));
    let has_level = sites
        .iter()
        .any(|s| matches!(s, UnsolvedSite::Level { .. }));
    let has_external = sites.iter().any(|s| matches!(s, UnsolvedSite::External));
    let blocked = sites.iter().find_map(|s| match s {
        UnsolvedSite::Blocked { arg } => Some(*arg),
        _ => None,
    });
    // ③ 只含外部元变量 ⇒ 那是外层的事，内层不报（否则同一条缺口会被报两遍，且内层没有源级名）。
    if !has_implicit && !has_level && blocked.is_none() && has_external {
        return Report::Defer;
    }
    // ④ 推迟位仍被元变量卡住 ⇒ 落在**那个实参**上，并点名它依赖的隐式形参。
    if let Some(arg) = blocked {
        let span = input.arg_spans.get(arg).copied().unwrap_or(input.span);
        return Report::Error(CompileError::elab(
            ErrorKind::ElabImplicitArgumentUnsolved,
            sanitized(unsolved_message(
                input.head_text,
                input.whole_text,
                input.ty_text,
                input.k,
                &blocked_why(input, arg),
            )),
            span,
        ));
    }
    // ① 有隐式位没解出 ⇒ 与默认档同一条文案（同一个 code、同一条出路 `@g <A> 0`），why 点名具体哪几个。
    if has_implicit {
        return Report::Error(CompileError::elab(
            ErrorKind::ElabImplicitArgumentUnsolved,
            sanitized(unsolved_message(
                input.head_text,
                input.whole_text,
                input.ty_text,
                input.k,
                &implicit_why(sites, input),
            )),
            input.span,
        ));
    }
    // ② 只剩宇宙层级：`= ` / `≠` 记法降成 `Eq.{u}` 时层级定不下来（K1d 的层元变量）。
    if has_level {
        let params: Vec<String> = sites
            .iter()
            .filter_map(|s| match s {
                UnsolvedSite::Level { param } => Some(format!("`{param}`")),
                _ => None,
            })
            .collect();
        return Report::Error(CompileError::elab(
            ErrorKind::ElabImplicitArgumentUnsolved,
            sanitized(unsolved_message(
                input.head_text,
                input.whole_text,
                input.ty_text,
                input.k,
                &format!(
                    "宇宙层级 {} 定不下来（`Eq`/`Ne` 的记法要先定层才算得上）：\
                     出路：写成 `Eq.{{…}}` / `@Eq.{{u}} …` 把层级写全",
                    if params.is_empty() {
                        "".to_string()
                    } else {
                        params.join("、")
                    }
                ),
            )),
            input.span,
        ));
    }
    // 只剩 External 之外的混合（例如 External + Implicit 已在上两条处理）⇒ 兜底当例外。
    Report::Fallback("unsolved_other")
}

/// ① 的 `why`：点名**具体**哪几个隐式形参（老路只说「至少有一个」，用户 2026-10-16 第 ③ 条）。
fn implicit_why(sites: &[UnsolvedSite], input: &ReportInput<'_>) -> String {
    let names: Vec<String> = sites
        .iter()
        .filter_map(|s| match s {
            UnsolvedSite::Implicit { index } => {
                Some(format!("`{}`", layer_name(input.ty_text, *index)))
            }
            _ => None,
        })
        .collect();
    if names.is_empty() {
        return "隐式形参没解出来".to_string();
    }
    format!("{} 由实参和期望类型都定不下来", names.join("、"))
}

/// ④ 的 `why`：第几个实参、它依赖哪个隐式形参。
fn blocked_why(input: &ReportInput<'_>, arg: usize) -> String {
    let first = (0..input.k)
        .map(|i| format!("`{}`", layer_name(input.ty_text, i)))
        .collect::<Vec<_>>()
        .join("、");
    let name = input
        .arg_spans
        .get(arg)
        .map(|_| String::new())
        .unwrap_or_default();
    format!(
        "第 {} 个实参{}的类型还定不下来：它依赖的隐式形参 {} 没解出来",
        arg + 1,
        name,
        if first.is_empty() {
            "（签名里没有隐式形参）".to_string()
        } else {
            first
        }
    )
}

/// 最后一道守卫：把内核漏出来的名字换成占位文字。命中 ⇒ `debug_assert!`（开发时当场可见）。
fn sanitized(text: String) -> String {
    let (out, hit) = sanitize(&text);
    debug_assert!(!hit, "P2-12：报错文字里出现了内核名 ⇒ 渲染路径有漏：{text}");
    out
}

/// `\0soko_*`（fresh 名）/ `?<数字>`（会话元变量 id）⇒ 「一个没解出来的形参」。
///
/// 返回 `(净化后, 是否命中)`。**只处理这两种**：别的形状（`$0`、`Sort(1)`）是让人读的类型写法，
/// 不能动（`kernel-rejected` 的正文就长这样）。
fn sanitize(text: &str) -> (String, bool) {
    const REPLACEMENT: &str = "一个没解出来的形参";
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut hit = false;
    let mut i = 0;
    while i < bytes.len() {
        // `\0soko_…`：源级 fresh 名（`implicit::telescope` 的产物）。
        if bytes[i] == 0 {
            let mut j = i + 1;
            while j < bytes.len() && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_') {
                j += 1;
            }
            out.push_str(REPLACEMENT);
            hit = true;
            i = j;
            continue;
        }
        // `?<数字>`：本会话元变量 id（≥ 2^30）。
        if bytes[i] == b'?' && bytes.get(i + 1).is_some_and(u8::is_ascii_digit) {
            let mut j = i + 1;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            out.push_str(REPLACEMENT);
            hit = true;
            i = j;
            continue;
        }
        // 多字节安全：按 UTF-8 字符边界推进。
        let ch = text[i..].chars().next().unwrap_or('\u{fffd}');
        out.push(ch);
        i += ch.len_utf8();
    }
    (out, hit)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `OwnedExpr` 没有 `Default` ⇒ 空节点表 + root 0（本模块只把它当**不透明载荷**）。
    fn empty_owned() -> sokonanoda::transfer::OwnedExpr {
        sokonanoda::transfer::OwnedExpr {
            nodes: Vec::new(),
            root: 0,
        }
    }

    fn span(line: usize) -> Span {
        Span::new(
            crate::Pos {
                offset: line,
                line,
                column: 3,
            },
            crate::Pos {
                offset: line,
                line,
                column: 9,
            },
        )
    }

    fn input<'a>(ty_text: &'a str, arg_spans: &'a [Span]) -> ReportInput<'a> {
        ReportInput {
            head_text: "g",
            whole_text: "g 0",
            ty_text,
            k: 1,
            total_layers: 2,
            explicit_layers: 1,
            n_args: 1,
            span: span(2),
            arg_spans,
        }
    }

    const TY: &str = "{A : Type} -> Nat -> Prop";

    fn text(r: &Report) -> String {
        match r {
            Report::Error(e) => e.message.clone(),
            Report::Fallback(why) => panic!("期望报错，却是回落 {why}"),
            Report::Defer => panic!("期望报错，却是 Defer"),
        }
    }

    /// t1 形状（`g 0`，实参比显式层多）⇒ **必须与老路那句逐字节相同**。
    #[test]
    fn t1_shape_matches_strict_rejection_verbatim() {
        let spans = [span(2)];
        let mut i = input(TY, &spans);
        i.n_args = 2;
        i.whole_text = "g 0 1";
        let outcome = AppOutcome::ArgMismatch {
            arg: 0,
            expected_domain: empty_owned(),
            actual_type: empty_owned(),
        };
        let want = strict_rejection_message("g", "g 0 1", TY, 1, 2, 1, 2);
        assert_eq!(text(&report(&outcome, &i)), want);
    }

    /// ① 隐式位 ⇒ 点名**源级**名（`A`），不是 `\0soko_p0`。
    #[test]
    fn implicit_site_names_source_level_parameter() {
        let spans = [span(2)];
        let i = input(TY, &spans);
        let outcome = AppOutcome::Unsolved {
            sites: vec![UnsolvedSite::Implicit { index: 0 }],
        };
        let msg = text(&report(&outcome, &i));
        assert!(msg.contains("`A`"), "必须点名源级形参 A：{msg}");
        assert!(!sanitize(&msg).1, "不许出现内核名：{msg}");
    }

    /// ③ 只含外部元变量 ⇒ `Defer`（内层不报，外层自有源级名）。
    #[test]
    fn external_only_is_deferred() {
        let spans = [span(2)];
        let i = input(TY, &spans);
        let outcome = AppOutcome::Unsolved {
            sites: vec![UnsolvedSite::External],
        };
        assert!(matches!(report(&outcome, &i), Report::Defer));
    }

    /// ④ blocked ⇒ 落在**那个实参**的 span 上，并点名它依赖的隐式形参。
    #[test]
    fn blocked_uses_argument_span() {
        let spans = [span(2), span(7)];
        let i = input(TY, &spans);
        let outcome = AppOutcome::Unsolved {
            sites: vec![UnsolvedSite::Blocked { arg: 1 }],
        };
        let r = report(&outcome, &i);
        let Report::Error(e) = &r else {
            panic!("blocked 必须报错")
        };
        assert_eq!(e.span, spans[1], "落在第 2 个实参上");
        assert!(e.message.contains("第 2 个实参"), "{}", e.message);
    }

    /// ② 只剩层 ⇒ 宇宙层级那条（不新增码）。
    #[test]
    fn level_only_reports_universe_wording() {
        let spans = [span(2)];
        let i = input(TY, &spans);
        let outcome = AppOutcome::Unsolved {
            sites: vec![UnsolvedSite::Level {
                param: "u".to_string(),
            }],
        };
        let r = report(&outcome, &i);
        let Report::Error(e) = &r else {
            panic!("层位必须报错")
        };
        assert!(e.message.contains("宇宙层级"), "{}", e.message);
        assert_eq!(e.kind, ErrorKind::ElabImplicitArgumentUnsolved);
    }

    /// **反向验证**：喂内核名（`\0soko_mp1` / `?1073741824`）⇒ 输出里一个都不许剩。
    #[test]
    fn kernel_names_never_leak() {
        let dirty = "期望 `\u{0}soko_mp1`，实际 `?1073741824`";
        let (clean, hit) = sanitize(dirty);
        assert!(hit, "必须认出内核名");
        assert!(!clean.contains("soko_m"), "{clean}");
        assert!(!clean.contains('?'), "{clean}");
        assert!(!clean.contains('\u{0}'), "{clean}");
        // 干净的文本原样返回（`Sort(1)` 这种不许动）。
        assert_eq!(sanitize("期望 Sort(1)").0, "期望 Sort(1)");
    }

    /// `ExpectedMismatch` 只是"期望没传播上" ⇒ 交给 S4 重解，不当错报。
    #[test]
    fn expected_mismatch_is_not_an_error() {
        let spans = [span(2)];
        let i = input(TY, &spans);
        let outcome = AppOutcome::ExpectedMismatch {
            expected: empty_owned(),
            actual: empty_owned(),
        };
        assert!(matches!(
            report(&outcome, &i),
            Report::Fallback("expected_retry")
        ));
    }
}

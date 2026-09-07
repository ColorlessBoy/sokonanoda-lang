//! kernel 判定的术语匹配（I9 goal 视图深化）。
//!
//! 设计（docs/design-i8-i9.md §2）：不发明第二套判定逻辑——把"候选术语 +
//! 已写 binders"合成一条**完整的声明**（`def _soko_judge_k : <声明类型> :=
//! fun <binders> => <术语>`），交给标准流水线（含 prelude 决策与
//! check-then-add 语义），由完整 kernel 当裁判：
//!
//! - 通过 → [`Judgement::Match`]；
//! - 内核拒绝且带 `def_eq mismatch expected/actual` → [`Judgement::Mismatch`]
//!   （"期望 X / 实际 Y"直接来自内核，呼应 Lean `exact?` 的教训：无效建议
//!   根本不该出现）；
//! - elaborate 失败 → [`Judgement::Error`]（稳定错误码 + 教学提示）。
//!
//! 注意：prelude 决策扫描的是"前缀 + 合成声明"，看不到文档后缀。若用户在
//! 目标声明之后才定义自己的 `Eq`/`Nat`（遮蔽 prelude），判定环境与文档环境
//! 可能有差别——教学文档（练习先于解答）不会出现这种形态。
//!
//! 判定永远走 kernel，不做文本比对（REQUIREMENTS §2.8）。

use crate::compile::{check_document_with, CompileOptions, DeclStatus, DocumentReport};
use crate::proof::parse_expr_text;
use crate::{Binder, BinderKind, Command, Expr, FolFile, Span};

/// 一个开放练习的判定规格：**剩余目标**（与 `DeclState.goal` /
/// `ProofState::goal_text` 同语义）、声明的宇宙参数、已写 binders
/// （名字 + 类型文本；类型必须可解析，`DeclState.binders` 恒有类型——
/// 未写时借用声明层）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenGoalSpec {
    pub universe: Vec<String>,
    pub ty: String,
    pub binders: Vec<GoalBinderSpec>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoalBinderSpec {
    pub name: String,
    pub ty: Option<String>,
}

impl OpenGoalSpec {
    /// 从 `#prove` 会话状态构造（REPL 用）。
    pub fn from_goal(universe: Vec<String>, ty: String, binders: Vec<GoalBinderSpec>) -> Self {
        Self {
            universe,
            ty,
            binders,
        }
    }
}

/// 一次 kernel 判定的结论。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Judgement {
    /// 术语的类型与剩余目标 definitional equal（kernel 判定通过）。
    Match,
    /// kernel 拒绝：期望类型与实际类型（两者都来自内核渲染）。
    Mismatch { expected: String, actual: String },
    /// 术语无法 elaborate（错误码 + 消息，教学提示同诊断管线）。
    Error { code: String, message: String },
}

/// 对开放声明 `open` 逐个判定 `terms` 是否能填进洞里。
/// 返回值与 `terms` 等长、按序对应；每次调用独立跑一遍前缀流水线。
pub fn judge_terms(
    prefix_src: &str,
    options: &CompileOptions,
    open: &OpenGoalSpec,
    terms: &[&str],
) -> Vec<Judgement> {
    let mut judgements = vec![
        Judgement::Error {
            code: "judge-not-run".to_string(),
            message: "判定未执行".to_string(),
        };
        terms.len()
    ];
    if terms.is_empty() {
        return judgements;
    }
    // 剩余目标解析失败 → 全部判为解析错误。
    let Ok(goal) = parse_expr_text(&open.ty) else {
        return vec![
            Judgement::Error {
                code: "parse".to_string(),
                message: format!("无法解析目标类型 `{}`", open.ty),
            };
            terms.len()
        ];
    };
    // 把已写 binders 折叠回声明类型：`(b1 : T1) -> (b2 : T2) -> 剩余目标`。
    // binder 名字与显隐风格不影响内核检查（只影响打印），统一折成命名箭头。
    let ty = match fold_declared(goal, &open.binders) {
        Ok(ty) => ty,
        Err(missing) => {
            return vec![
                Judgement::Error {
                    code: "elab-untyped-binder".to_string(),
                    message: format!("binder `{missing}` 缺少类型标注，无法合成判定声明"),
                };
                terms.len()
            ]
        }
    };
    let Ok(prefix_file) = parse_prefix(prefix_src) else {
        return vec![
            Judgement::Error {
                code: "parse".to_string(),
                message: "前缀源码无法解析".to_string(),
            };
            terms.len()
        ];
    };

    let mut commands = prefix_file.commands;
    let mut failed_parse: Option<usize> = None;
    for (k, term) in terms.iter().enumerate() {
        let Ok(term_expr) = parse_expr_text(term) else {
            failed_parse = Some(k);
            continue;
        };
        let val = wrap_binders(&open.binders, term_expr);
        commands.push(Command::Def {
            name: format!("_soko_judge_{k}"),
            universe: open.universe.clone(),
            ty: ty.clone(),
            val,
            span: Span::default(),
        });
    }
    let report = check_document_with(&FolFile { commands }, options);
    for (k, judgement) in judgements.iter_mut().enumerate() {
        if failed_parse == Some(k) {
            *judgement = Judgement::Error {
                code: "parse".to_string(),
                message: format!("无法解析术语 `{}`", terms[k]),
            };
            continue;
        }
        *judgement = judgement_of(&report, k);
    }
    judgements
}

fn parse_prefix(prefix_src: &str) -> Result<FolFile, ()> {
    crate::parse(prefix_src).map_err(|_| ())
}

/// 把剩余目标与已写 binders 折叠成完整声明类型：一个 Forall 望远镜
/// `forall (b1 : T1) (b2 : T2), 剩余目标`——与命名箭头的语法语义一致
/// （后一个 binder 的类型可以引用前一个，必须在同一 telescope 内 elaborate）。
/// 返回 `Err(binder_name)` 表示该 binder 缺少类型标注。
fn fold_declared(goal: Expr, binders: &[GoalBinderSpec]) -> Result<Expr, String> {
    let mut parsed = Vec::with_capacity(binders.len());
    for binder in binders {
        let Some(text) = &binder.ty else {
            return Err(binder.name.clone());
        };
        let Ok(domain) = parse_expr_text(text) else {
            return Err(binder.name.clone());
        };
        parsed.push(Binder {
            name: binder.name.clone(),
            ty: Some(Box::new(domain)),
            style: BinderKind::Explicit,
            span: Span::default(),
        });
    }
    if parsed.is_empty() {
        return Ok(goal);
    }
    Ok(Expr::Forall {
        binders: parsed,
        body: Box::new(goal),
        span: Span::default(),
    })
}

/// 把术语包上已写 binders：`fun (b1 : T1) => fun (b2 : T2) => term`。
/// 未写类型的 binder 留空，交给声明类型驱动的 binder 推断（I6）。
fn wrap_binders(binders: &[GoalBinderSpec], term: Expr) -> Expr {
    let mut term = term;
    for binder in binders.iter().rev() {
        let ty = match &binder.ty {
            Some(text) => parse_expr_text(text).ok().map(Box::new),
            None => None,
        };
        term = Expr::Lambda {
            binders: vec![Binder {
                name: binder.name.clone(),
                ty,
                style: BinderKind::Explicit,
                span: Span::default(),
            }],
            body: Box::new(term),
            span: Span::default(),
        };
    }
    term
}

/// 从合成声明的检查结果提取判定结论。
fn judgement_of(report: &DocumentReport, k: usize) -> Judgement {
    let name = format!("_soko_judge_{k}");
    let Some(state) = report
        .decls
        .iter()
        .find(|d| d.name.as_deref() == Some(name.as_str()))
    else {
        return Judgement::Error {
            code: "judge-missing".to_string(),
            message: format!("合成声明 `{name}` 没有产生状态（内部错误）"),
        };
    };
    match state.status {
        DeclStatus::Checked => Judgement::Match,
        DeclStatus::Open => Judgement::Error {
            code: "elab-hole-misplaced".to_string(),
            message: "洞不在可填写的位置".to_string(),
        },
        DeclStatus::Failed => {
            let Some(err) = &state.error else {
                return Judgement::Error {
                    code: "judge-unknown".to_string(),
                    message: "判定失败但没有错误信息（内部错误）".to_string(),
                };
            };
            match (&err.expected, &err.actual) {
                (Some(expected), Some(actual)) => Judgement::Mismatch {
                    expected: expected.clone(),
                    actual: actual.clone(),
                },
                _ => Judgement::Error {
                    code: err.code().to_string(),
                    message: err.message.clone(),
                },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile::PreludeMode;

    fn spec(ty: &str, binders: &[(&str, Option<&str>)]) -> OpenGoalSpec {
        OpenGoalSpec {
            universe: Vec::new(),
            ty: ty.to_string(),
            binders: binders
                .iter()
                .map(|(name, ty)| GoalBinderSpec {
                    name: name.to_string(),
                    ty: ty.map(|t| t.to_string()),
                })
                .collect(),
        }
    }

    #[test]
    fn matching_hypothesis_is_a_kernel_match() {
        let prefix = "axiom a : Prop\n";
        // 剩余目标 a，已写 binder h : a ⇒ 折叠出的声明类型是 (h : a) -> a。
        let open = spec("a", &[("h", Some("a"))]);
        let judgements = judge_terms(
            prefix,
            &CompileOptions::default(),
            &open,
            &["h", "fun (x : Prop) => x"],
        );
        assert_eq!(judgements[0], Judgement::Match);
        match &judgements[1] {
            Judgement::Mismatch { expected, actual } => {
                // debug printer 渲染：Prop → Sort(0)，宇宙参数带 .[] 后缀。
                assert!(expected.contains("a"), "expected: {expected}");
                assert!(actual.contains("Sort(0)"), "actual: {actual}");
            }
            other => panic!("expected mismatch, got {other:?}"),
        }
    }

    #[test]
    fn dependent_binder_types_are_judged_correctly() {
        // h : a（依赖前面的 binder a）与剩余目标 a 相同。
        let open = spec("a", &[("a", Some("Prop")), ("h", Some("a"))]);
        let judgements = judge_terms("", &CompileOptions::default(), &open, &["h"]);
        assert_eq!(judgements[0], Judgement::Match);
    }

    #[test]
    fn binder_without_type_annotation_is_reported() {
        // 声明层总会为未写类型的 binder 借来类型，所以 None 只可能是
        // 防御性输入；判定明确报错而不是静默猜测。
        let open = spec("Prop", &[("x", None)]);
        let judgements = judge_terms("", &CompileOptions::default(), &open, &["x"]);
        match &judgements[0] {
            Judgement::Error { code, message } => {
                assert_eq!(code, "elab-untyped-binder");
                assert!(message.contains('x'), "message: {message}");
            }
            other => panic!("expected untyped-binder error, got {other:?}"),
        }
    }

    #[test]
    fn defeq_but_differently_written_type_matches() {
        // `Not a` 与 `a -> False` 文本不同但 definitional equal —— 文本比对
        // 会漏掉它，kernel 判定能识别（REQUIREMENTS §2.8 的意义所在）。
        let prefix = "axiom False : Prop\ndef Not : Prop -> Prop := fun (a : Prop) => a -> False\n";
        let open = spec("Not a", &[("a", Some("Prop")), ("h", Some("a -> False"))]);
        let judgements = judge_terms(prefix, &CompileOptions::default(), &open, &["h"]);
        assert_eq!(judgements[0], Judgement::Match);
    }

    #[test]
    fn unknown_term_reports_elab_error() {
        let open = spec("Prop", &[]);
        let judgements = judge_terms("", &CompileOptions::default(), &open, &["nope"]);
        match &judgements[0] {
            Judgement::Error { code, message } => {
                assert_eq!(code, "elab-unknown-identifier");
                assert!(message.contains("nope"));
            }
            other => panic!("expected elab error, got {other:?}"),
        }
    }

    #[test]
    fn failed_prefix_declaration_keeps_its_name_free() {
        // check-then-add：前缀里被 kernel 拒绝的名字不占用。引用它的候选在
        // pass1 能通过（用的是"幽灵"声明类型），pass2 会重新 elaborate 并
        // 得到真正的 unknown-identifier —— 这正是教学想要的判定语义。
        let prefix = "def broken : Prop -> Type := fun (x : Prop) => x\n";
        let open = spec("Prop -> Type", &[]);
        let judgements = judge_terms(prefix, &CompileOptions::default(), &open, &["broken"]);
        match &judgements[0] {
            Judgement::Error { code, .. } => {
                assert_eq!(code, "elab-unknown-identifier");
            }
            other => panic!("expected unknown identifier, got {other:?}"),
        }
    }

    #[test]
    fn bare_mode_judges_without_prelude() {
        let options = CompileOptions {
            prelude: PreludeMode::Bare,
        };
        let open = spec("Prop -> Prop", &[]);
        let judgements = judge_terms("", &options, &open, &["fun (x : Prop) => x"]);
        assert_eq!(judgements[0], Judgement::Match);
        // Bare 模式下 Nat 不存在。
        let open_nat = spec("Nat", &[]);
        let judgements = judge_terms("", &options, &open_nat, &["2"]);
        match &judgements[0] {
            Judgement::Error { code, .. } => {
                assert_eq!(code, "elab-unknown-identifier");
            }
            other => panic!("expected unknown Nat in bare mode, got {other:?}"),
        }
    }

    #[test]
    fn universe_carrying_open_goal_judges() {
        // 带宇宙参数的声明：剩余目标 α，合成声明必须携带同样的 {u}。
        let prefix =
            "def id {u} : {α : Sort u} -> (a : α) -> α := fun {α : Sort u} => fun (a : α) => a\n";
        let open = OpenGoalSpec {
            universe: vec!["u".to_string()],
            ty: "α".to_string(),
            binders: vec![
                GoalBinderSpec {
                    name: "α".to_string(),
                    ty: Some("Sort u".to_string()),
                },
                GoalBinderSpec {
                    name: "a".to_string(),
                    ty: Some("α".to_string()),
                },
            ],
        };
        let judgements = judge_terms(
            prefix,
            &CompileOptions::default(),
            &open,
            &["a", "id.{u} α a"],
        );
        assert_eq!(judgements[0], Judgement::Match);
        assert_eq!(judgements[1], Judgement::Match);
    }
}

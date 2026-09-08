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
//! [`judge_value_replace`] 服务失败声明（kernel 拒绝、没有洞）：候选整体
//! 替换 `:=` 之后的值位，同名机制合成判定声明。失败声明的针对性建议
//! （`suggest` 的 Eq 形状 rfl 替换）用它做 kernel 终审。
//!
//! 判定永远走 kernel，不做文本比对（REQUIREMENTS §2.8）。

use crate::compile::{check_document_with, CompileOptions, DeclStatus, DocumentReport};
use crate::proof::parse_expr_text;
use crate::{tokenize, Binder, BinderKind, Command, Expr, FolFile, Span, Token, TokenKind};

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

/// 把 doc 中 decl_span 命令里的 hole_span 替换为候选 term，改名合成声明
/// 后走完整流水线判定（与 [`judge_terms`] 同语义：合成声明是唯一裁判）。
///
/// 与 [`judge_terms`] 的差别：判定发生在**文档里的真实命令**中。声明名换成
/// `_soko_judge_k`（`example` 声明无名字：把首 token `example` 换成
/// `def _soko_judge_k`；宇宙参数 `{u}` 等保留原样），指定洞的 `???` 换成
/// 候选，其余洞保持原样。因此命令里只要还有剩余洞，合成声明就仍是 open
/// 练习，结论如实是 [`Judgement::Error`]——多洞状态的逐洞判定由调用方
/// （`suggest`）改用 [`judge_terms`] 按子洞期望类型完成。
pub fn judge_hole_fill(
    doc_src: &str,
    options: &CompileOptions,
    decl_span: Span,
    hole_span: Span,
    candidates: &[&str],
) -> Vec<Judgement> {
    let mut judgements = vec![
        Judgement::Error {
            code: "judge-not-run".to_string(),
            message: "判定未执行".to_string(),
        };
        candidates.len()
    ];
    if candidates.is_empty() {
        return judgements;
    }
    let all_parse_error = |message: String| -> Vec<Judgement> {
        vec![
            Judgement::Error {
                code: "parse".to_string(),
                message,
            };
            candidates.len()
        ]
    };
    let decl_start = decl_span.start.offset.min(doc_src.len());
    let decl_end = decl_span.end.offset.clamp(decl_start, doc_src.len());
    let slice = &doc_src[decl_start..decl_end];
    let Ok(tokens) = tokenize(slice) else {
        return all_parse_error("声明命令切片无法分词".to_string());
    };
    // 声明名 token 的切片内区间：`example` 换成 `def _soko_judge_k`；
    // def/theorem/axiom 的名字 token 换成 `_soko_judge_k`。名字定位复用
    // `references::decl_name_span`（token 精确，绝不扫描文本）。
    let (name_start, name_end, example_keyword) =
        match decl_name_segment(doc_src, decl_span, decl_start, &tokens) {
            Ok(segment) => segment,
            Err(message) => return all_parse_error(message),
        };
    // 洞的切片内区间：必须确实落在命令里，且切片就是 `???`。
    let hole_start = hole_span.start.offset;
    let hole_end = hole_span.end.offset;
    let in_decl = hole_start >= decl_start && hole_end <= decl_end && hole_start < hole_end;
    if !in_decl || &doc_src[hole_start..hole_end] != "???" {
        return all_parse_error("洞位置不在该声明的 `???` 上".to_string());
    }
    let hole_start = hole_start - decl_start;
    let hole_end = hole_end - decl_start;
    if name_start < hole_end && hole_start < name_end {
        return all_parse_error("声明名与洞重叠，无法合成判定声明".to_string());
    }
    // 逐候选：名字段与洞段两处替换，按偏移拼接出合成命令文本。
    let mut commands: Vec<Command> = Vec::with_capacity(candidates.len());
    let mut failed_parse: Vec<Option<String>> = vec![None; candidates.len()];
    for (k, candidate) in candidates.iter().enumerate() {
        let name = format!("_soko_judge_{k}");
        let name_text = if example_keyword {
            format!("def {name}")
        } else {
            name.clone()
        };
        // 名字段在声明头、洞段在其后；仍按偏移排序，防御性处理乱序输入。
        let segments: [(usize, usize, &str); 2] = if name_start <= hole_start {
            [
                (name_start, name_end, name_text.as_str()),
                (hole_start, hole_end, (*candidate)),
            ]
        } else {
            [
                (hole_start, hole_end, (*candidate)),
                (name_start, name_end, name_text.as_str()),
            ]
        };
        let mut synth = String::with_capacity(slice.len() + name_text.len() + candidate.len());
        let mut cursor = 0usize;
        for (start, end, text) in segments {
            synth.push_str(&slice[cursor..start]);
            synth.push_str(text);
            cursor = end;
        }
        synth.push_str(&slice[cursor..]);
        match crate::parse(&synth) {
            Ok(file) => match file.commands.as_slice() {
                [parsed @ (Command::Def {
                    name: parsed_name, ..
                }
                | Command::Theorem {
                    name: parsed_name, ..
                })] if parsed_name == &name => {
                    commands.push(parsed.clone());
                }
                _ => {
                    failed_parse[k] =
                        Some(format!("合成文本不是声明 `{name}`（候选 `{candidate}`）"));
                }
            },
            Err(err) => {
                failed_parse[k] = Some(format!(
                    "无法解析合成命令（候选 `{candidate}`）：{}",
                    err.message
                ));
            }
        }
    }
    // 前缀命令表 + 合成声明，走与 judge_terms 一致的完整流水线。
    let Ok(mut file) = parse_prefix(&doc_src[..decl_start]) else {
        return all_parse_error("前缀源码无法解析".to_string());
    };
    file.commands.extend(commands);
    let report = check_document_with(&file, options);
    for (k, judgement) in judgements.iter_mut().enumerate() {
        if let Some(message) = failed_parse[k].take() {
            *judgement = Judgement::Error {
                code: "parse".to_string(),
                message,
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

/// 声明名 token 在命令切片内的区间（切片相对偏移）：`example` 返回关键字
/// token 自身（合成时换成 `def _soko_judge_k`）；def/theorem/axiom 返回
/// 名字 token（经 `references::decl_name_span` 定位，绝不扫描文本）。
/// [`judge_hole_fill`] 与 [`judge_value_replace`] 共用。第四个返回值表示
/// 声明是匿名 `example`。
fn decl_name_segment(
    doc_src: &str,
    decl_span: Span,
    decl_start: usize,
    tokens: &[Token],
) -> Result<(usize, usize, bool), String> {
    match tokens.first().map(|t| &t.kind) {
        Some(TokenKind::Ident(kw)) if kw == "example" => {
            let token = &tokens[0];
            Ok((token.span.start.offset, token.span.end.offset, true))
        }
        Some(TokenKind::Ident(kw)) if matches!(kw.as_str(), "def" | "theorem" | "axiom") => {
            let Some(TokenKind::Ident(name)) = tokens.get(1).map(|t| &t.kind) else {
                return Err("声明名缺失，无法合成判定声明".to_string());
            };
            let Some(name_span) = crate::references::decl_name_span(doc_src, decl_span, name)
            else {
                return Err(format!("找不到声明名 `{name}` 的 token"));
            };
            let start = name_span.start.offset.saturating_sub(decl_start);
            let end = name_span.end.offset.saturating_sub(decl_start);
            Ok((start, end, false))
        }
        _ => Err("只有 def/theorem/example 声明可以合成判定".to_string()),
    }
}

/// 把 doc 中 decl_span 命令里 `:=` 之后的**整个值位**替换为候选，改名合成
/// 声明后走完整流水线判定（与 [`judge_hole_fill`] 同语义：合成声明是唯一
/// 裁判）。服务失败声明（kernel 拒绝、没有洞）的针对性建议：候选被 kernel
/// 接受才值得呈现。值位起点由 tokenize 定位（`:=` 后第一个 token），
/// 终点即声明 span 末尾（解析器把 span 收在值的最后一个 token 上）。
///
/// 与 [`judge_hole_fill`] 的差别：没有洞可填——候选**整体替换值位**，
/// 宇宙参数与值位之前的命令头原样保留；`example` 声明仍按首 token 换名。
/// 解析失败一律报 [`Judgement::Error`]，绝不 panic。
pub fn judge_value_replace(
    doc_src: &str,
    options: &CompileOptions,
    decl_span: Span,
    candidates: &[&str],
) -> Vec<Judgement> {
    let mut judgements = vec![
        Judgement::Error {
            code: "judge-not-run".to_string(),
            message: "判定未执行".to_string(),
        };
        candidates.len()
    ];
    if candidates.is_empty() {
        return judgements;
    }
    let all_parse_error = |message: String| -> Vec<Judgement> {
        vec![
            Judgement::Error {
                code: "parse".to_string(),
                message,
            };
            candidates.len()
        ]
    };
    let decl_start = decl_span.start.offset.min(doc_src.len());
    let decl_end = decl_span.end.offset.clamp(decl_start, doc_src.len());
    let slice = &doc_src[decl_start..decl_end];
    let Ok(tokens) = tokenize(slice) else {
        return all_parse_error("声明命令切片无法分词".to_string());
    };
    let (name_start, name_end, example_keyword) =
        match decl_name_segment(doc_src, decl_span, decl_start, &tokens) {
            Ok(segment) => segment,
            Err(message) => return all_parse_error(message),
        };
    // 值位：`:=` 后第一个 token 起，到声明 span 末尾。
    let Some(colon_eq) = tokens.iter().position(|t| t.kind == TokenKind::ColonEq) else {
        return all_parse_error("声明没有 `:=` 值位，无法替换判定".to_string());
    };
    let Some(value_tok) = tokens.get(colon_eq + 1) else {
        return all_parse_error("声明没有值位，无法替换判定".to_string());
    };
    if value_tok.kind == TokenKind::Eof {
        return all_parse_error("声明没有值位，无法替换判定".to_string());
    }
    let value_start = value_tok.span.start.offset;
    if name_end > value_start {
        return all_parse_error("声明名与值位重叠，无法合成判定声明".to_string());
    }
    // 逐候选：名字段与值段两处替换，按偏移拼接出合成命令文本（值段延伸到
    // 切片末尾，其后没有剩余文本）。
    let mut commands: Vec<Command> = Vec::with_capacity(candidates.len());
    let mut failed_parse: Vec<Option<String>> = vec![None; candidates.len()];
    for (k, candidate) in candidates.iter().enumerate() {
        let name = format!("_soko_judge_{k}");
        let name_text = if example_keyword {
            format!("def {name}")
        } else {
            name.clone()
        };
        let mut synth = String::with_capacity(slice.len() + name_text.len() + candidate.len());
        synth.push_str(&slice[..name_start]);
        synth.push_str(&name_text);
        synth.push_str(&slice[name_end..value_start]);
        synth.push_str(candidate);
        match crate::parse(&synth) {
            Ok(file) => match file.commands.as_slice() {
                [parsed @ (Command::Def {
                    name: parsed_name, ..
                }
                | Command::Theorem {
                    name: parsed_name, ..
                })] if parsed_name == &name => {
                    commands.push(parsed.clone());
                }
                _ => {
                    failed_parse[k] =
                        Some(format!("合成文本不是声明 `{name}`（候选 `{candidate}`）"));
                }
            },
            Err(err) => {
                failed_parse[k] = Some(format!(
                    "无法解析合成命令（候选 `{candidate}`）：{}",
                    err.message
                ));
            }
        }
    }
    // 前缀命令表 + 合成声明，走与 judge_terms 一致的完整流水线。
    let Ok(mut file) = parse_prefix(&doc_src[..decl_start]) else {
        return all_parse_error("前缀源码无法解析".to_string());
    };
    file.commands.extend(commands);
    let report = check_document_with(&file, options);
    for (k, judgement) in judgements.iter_mut().enumerate() {
        if let Some(message) = failed_parse[k].take() {
            *judgement = Judgement::Error {
                code: "parse".to_string(),
                message,
            };
            continue;
        }
        *judgement = judgement_of(&report, k);
    }
    judgements
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
    use crate::compile::{check_document, DeclState, DeclStatus, PreludeMode};
    use crate::parse;

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

    // ---- judge_hole_fill：文档真实命令里的逐洞判定 ----

    /// 取文档里第一个 open 练习的 DeclState（span 与洞位都来自完整流水线）。
    fn open_decl(doc: &str) -> DeclState {
        let report = check_document(&parse(doc).expect("parses"));
        report
            .decls
            .iter()
            .find(|d| d.status == DeclStatus::Open)
            .expect("open exercise")
            .clone()
    }

    const SUB_HOLE_DOC: &str = "axiom And : Prop -> Prop -> Prop\n\
         axiom And.intro : (a : Prop) -> (b : Prop) -> a -> b -> And a b\n\
         theorem t : (a : Prop) -> (b : Prop) -> (ha : a) -> (hb : b) -> And a b := \
         fun (a : Prop) => fun (b : Prop) => fun (ha : a) => fun (hb : b) => And.intro a b ha ???\n";

    #[test]
    fn hole_fill_accepts_hypothesis_in_a_sub_hole() {
        // 剩一个子洞的 spine 状态：填对假设 ⇒ 整个证明被完整 kernel 接受；
        // 填错 ⇒ 内核给出"期望 / 实际"（类型不合的候选 → Mismatch）。
        let d = open_decl(SUB_HOLE_DOC);
        let judgements = judge_hole_fill(
            SUB_HOLE_DOC,
            &CompileOptions::default(),
            d.span,
            d.holes[0],
            &["hb", "ha"],
        );
        assert_eq!(judgements[0], Judgement::Match);
        assert!(
            matches!(judgements[1], Judgement::Mismatch { .. }),
            "type-mismatched candidate must be a kernel mismatch, got {:?}",
            judgements[1]
        );
    }

    #[test]
    fn hole_fill_renames_examples_by_replacing_the_first_token() {
        // `example` 无名字：首 token `example` 换成 `def _soko_judge_k`。
        let doc = "axiom False : Prop\n\
                   example : (h : False) -> False := fun (h : False) => ???\n";
        let d = open_decl(doc);
        let judgements = judge_hole_fill(
            doc,
            &CompileOptions::default(),
            d.span,
            d.holes[0],
            &["h", "False"],
        );
        assert_eq!(judgements[0], Judgement::Match);
        match &judgements[1] {
            Judgement::Mismatch { expected, actual } => {
                // 内核渲染：`False.[]` 保留原名，Prop 显示为 Sort(0)。
                assert!(expected.contains("False"), "expected: {expected}");
                assert!(actual.contains("Sort(0)"), "actual: {actual}");
            }
            other => panic!("expected mismatch, got {other:?}"),
        }
    }

    #[test]
    fn hole_fill_keeps_universe_params_from_the_source() {
        // def 的宇宙参数 `{u}` 原样保留在合成命令里，Sort u 目标可判定。
        let doc =
            "def idT {u} : {α : Sort u} -> (a : α) -> α := fun {α : Sort u} => fun (a : α) => a\n\
                   theorem t {u} : {α : Sort u} -> (a : α) -> Eq.{u} α a a := \
                   fun {α : Sort u} => fun (a : α) => ???\n";
        let d = open_decl(doc);
        let judgements = judge_hole_fill(
            doc,
            &CompileOptions::default(),
            d.span,
            d.holes[0],
            &["Eq.refl.{u} α a"],
        );
        assert_eq!(judgements[0], Judgement::Match);
    }

    #[test]
    fn hole_fill_with_remaining_holes_reports_open_honestly() {
        // 其余洞保持 `???` ⇒ 合成声明仍是 open 练习：kernel 没能整体裁决，
        // 结论如实为 Error（绝不把"没判过"说成 Match）。
        let doc = "axiom And : Prop -> Prop -> Prop\n\
                   axiom And.intro : (a : Prop) -> (b : Prop) -> a -> b -> And a b\n\
                   theorem t : (a : Prop) -> (b : Prop) -> (ha : a) -> (hb : b) -> And a b := \
                   fun (a : Prop) => fun (b : Prop) => fun (ha : a) => fun (hb : b) => And.intro ??? ???\n";
        let d = open_decl(doc);
        let judgements = judge_hole_fill(
            doc,
            &CompileOptions::default(),
            d.span,
            d.holes[0],
            &["ha", "hb"],
        );
        assert!(
            judgements.iter().all(
                |j| matches!(j, Judgement::Error { code, .. } if code == "elab-hole-misplaced")
            ),
            "remaining holes keep the fill open: {judgements:?}"
        );
    }

    #[test]
    fn hole_fill_reports_parse_failures_as_errors_not_panics() {
        let d = open_decl(SUB_HOLE_DOC);
        let judgements = judge_hole_fill(
            SUB_HOLE_DOC,
            &CompileOptions::default(),
            d.span,
            d.holes[0],
            &["no(", "hb"],
        );
        match &judgements[0] {
            Judgement::Error { code, .. } => assert_eq!(code, "parse"),
            other => panic!("expected parse error, got {other:?}"),
        }
        assert_eq!(judgements[1], Judgement::Match);
    }

    // ---- judge_value_replace：失败声明的值位整体替换判定 ----

    /// 取文档里第一个 failed 声明的 DeclState（span 来自完整流水线）。
    fn failed_decl(doc: &str) -> DeclState {
        let report = check_document(&parse(doc).expect("parses"));
        report
            .decls
            .iter()
            .find(|d| d.status == DeclStatus::Failed)
            .expect("failed declaration")
            .clone()
    }

    #[test]
    fn value_replace_accepts_a_kernel_verified_rfl_candidate() {
        // 匿名 example：首 token 换名后整值替换。kernel 接受的 rfl 候选
        // Match，两边不同的候选被内核以 Mismatch 拒绝。
        let doc = "example : Eq.{1} Nat 2 2 := 3\n";
        let d = failed_decl(doc);
        let judgements = judge_value_replace(
            doc,
            &CompileOptions::default(),
            d.span,
            &["Eq.refl.{1} Nat 2", "Eq.refl.{1} Nat 3"],
        );
        assert_eq!(judgements[0], Judgement::Match);
        assert!(
            matches!(judgements[1], Judgement::Mismatch { .. }),
            "3 ≢ 2 must be a kernel mismatch, got {:?}",
            judgements[1]
        );
    }

    #[test]
    fn value_replace_keeps_universe_params_and_declared_binders() {
        // def 的宇宙参数 `{u}` 与值位之前的命令头原样保留：候选 lambda
        // 引用 Sort u 必须仍可 elaborate，内核才判得出 Match。
        let doc =
            "def idT {u} : {α : Sort u} -> (a : α) -> α := fun {α : Sort u} => fun (a : α) => 1\n";
        let d = failed_decl(doc);
        let judgements = judge_value_replace(
            doc,
            &CompileOptions::default(),
            d.span,
            &[
                "fun {α : Sort u} => fun (a : α) => a",
                "fun {α : Sort u} => fun (a : α) => 2",
            ],
        );
        assert_eq!(judgements[0], Judgement::Match);
        assert!(
            matches!(judgements[1], Judgement::Mismatch { .. }),
            "Nat ≢ α must be a kernel mismatch, got {:?}",
            judgements[1]
        );
    }

    #[test]
    fn value_replace_reports_unparseable_candidates_as_errors() {
        let doc = "example : Eq.{1} Nat 2 2 := 3\n";
        let d = failed_decl(doc);
        let judgements =
            judge_value_replace(doc, &CompileOptions::default(), d.span, &["no(", "nope"]);
        match &judgements[0] {
            Judgement::Error { code, message } => {
                assert_eq!(code, "parse");
                assert!(message.contains("no("), "message: {message}");
            }
            other => panic!("expected parse error, got {other:?}"),
        }
        match &judgements[1] {
            Judgement::Error { code, .. } => assert_eq!(code, "elab-unknown-identifier"),
            other => panic!("expected elab error, got {other:?}"),
        }
    }

    #[test]
    fn value_replace_reports_declarations_without_a_value_as_errors() {
        // axiom 没有 `:=` 值位：明确报错，绝不 panic。
        let doc = "axiom bad : undefined_name\n";
        let d = failed_decl(doc);
        let judgements = judge_value_replace(doc, &CompileOptions::default(), d.span, &["Prop"]);
        match &judgements[0] {
            Judgement::Error { code, .. } => assert_eq!(code, "parse"),
            other => panic!("expected parse error, got {other:?}"),
        }
    }
}

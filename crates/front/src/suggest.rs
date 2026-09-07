//! 按目标形状的下一步建议（docs/design-hints-suggestions.md §4）。
//!
//! 优先级：exact（逐洞，kernel 判定）→ rfl（kernel 判定）→ refine → intro
//! （结构生成，kernel 在学生落笔后终审）。每请求 ≤3 条、每洞候选 ≤4 个；
//! `suggest` 内部已排好序，第一即 preferred。判定永远走 kernel，不做文本
//! 比对（REQUIREMENTS §2.8）。
//!
//! 逐洞判定的语义：
//! * 主洞（单洞、无子目标）的 exact 与 rfl 用 [`judge_terms`]（现有语义
//!   保持不变，批量判定一次成型）；
//! * spine 且只剩一个洞时用 [`judge_hole_fill`]——填好的整份证明交完整
//!   kernel 裁决（最忠实：其余实参也随文档一起被查）；
//! * 多洞 spine 的其余洞保持 `???` 会让合成声明仍是 open 练习（kernel 无从
//!   整体裁决，见 `judge_hole_fill`），所以逐洞改用 [`judge_terms`] 按
//!   `DeclState.sub_goals[i].ty`（walk 恢复的期望类型）判定
//!   "假设 ≡ 该子洞期望类型"——同样的合成声明裁判语义，判定对象换成子洞
//!   期望类型。这也顺带修复了旧 `exact` 把"匹配外层 goal 的假设"塞进子洞
//!   的错位建议：外层匹配者与子洞期望类型不合，kernel 拒绝即不出现。

use crate::compile::{render_expr, CompileOptions, DeclState, DeclStatus};
use crate::judge::{judge_hole_fill, judge_terms, GoalBinderSpec, Judgement, OpenGoalSpec};
use crate::proof::parse_expr_text;
use crate::token::{tokenize, TokenKind};
use crate::{BinderKind, Expr};

/// 建议的种类（docs/design-hints-suggestions.md §4.2）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SuggestionKind {
    /// 第 `hole` 个洞（`DeclState.holes` 下标）填该假设。
    Exact { binder: String, hole: usize },
    /// 用 `DeclState.refine_template` 拆分子目标。
    Refine,
    /// 剥一层/多层 binder。
    Intro,
    /// kernel 验证过的 `Eq.refl` 候选。
    Rfl { term: String },
    /// kernel 拒绝的失败声明：按声明类型的形状生成的重启骨架
    /// `fun (x : A) => … => ???`，替换整个值位。结构生成、kernel 在学生
    /// 下次编辑后终审（docs/design-kernel-taxonomy.md §2）。
    Restart { skeleton: String },
}

/// 一条下一步建议：`verified` 为 true 表示已经过完整 kernel 判定。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suggestion {
    pub kind: SuggestionKind,
    pub verified: bool,
}

/// 每洞 exact 的候选 binder 数上限（docs/design-hints-suggestions.md §4.2）。
const BINDER_CANDIDATES: usize = 4;

/// 每请求的建议条数上限（第一即 preferred）。
const MAX_SUGGESTIONS: usize = 3;

/// 生成与排序（每请求 ≤3 条、每洞候选 ≤4 个）。
/// `prefix_src` 是文档开头到该声明 span 结束为止的源文本（**含**声明本身）：
/// `judge_terms` 取它的声明前缀切片，`judge_hole_fill` 需要看到声明命令。
/// `decl_src` 是该声明的命令全文切片（`decl.span` 对应的源文本），只有
/// 失败声明的重启骨架需要它；Open 练习传 `None`。
pub fn suggest(
    prefix_src: &str,
    decl_src: Option<&str>,
    options: &CompileOptions,
    d: &DeclState,
) -> Vec<Suggestion> {
    if d.status == DeclStatus::Failed {
        // 失败声明没有洞可填：唯一的建议是按声明类型形状重启整个值位。
        // 骨架只是结构生成（`verified: false`），kernel 在下次编辑终审。
        return match decl_src.and_then(restart_skeleton) {
            Some(skeleton) => vec![Suggestion {
                kind: SuggestionKind::Restart { skeleton },
                verified: false,
            }],
            None => Vec::new(),
        };
    }
    if d.holes.is_empty() {
        return Vec::new();
    }
    let judge_prefix = &prefix_src[..d.span.start.offset.min(prefix_src.len())];
    let binders = top_binders(d);
    let single_hole = d.holes.len() == 1;
    let mut exact: Vec<Suggestion> = Vec::new();
    let mut rfl: Vec<Suggestion> = Vec::new();
    for (i, hole) in d.holes.iter().enumerate() {
        let Some(expected) = hole_goal_text(d, i) else {
            continue;
        };
        // 批量判定一次成型：前 4 个 binder（+ 单洞时的 rfl 候选）。
        let rfl_term = if single_hole {
            eq_refl_candidate(&expected, d)
        } else {
            None
        };
        let mut terms = binders.clone();
        let rfl_base = terms.len();
        if let Some(term) = &rfl_term {
            terms.push(term.clone());
        }
        let refs: Vec<&str> = terms.iter().map(String::as_str).collect();
        let judgements = if d.sub_goals.is_empty() {
            // 主洞：现有 judge_terms 语义保持不变。
            judge_terms(judge_prefix, options, &open_spec(d, &expected), &refs)
        } else if single_hole {
            // spine 只剩一个洞：填好的整份证明交完整 kernel 裁决。
            judge_hole_fill(prefix_src, options, d.span, *hole, &refs)
        } else {
            // 多洞 spine：按子洞期望类型逐洞判定（其余洞保持原样时 kernel
            // 无从整体裁决——见模块注释）。
            judge_terms(judge_prefix, options, &open_spec(d, &expected), &refs)
        };
        // 每洞最多 1 条 exact：第一个 kernel 通过的 binder。
        if let Some(pos) = judgements
            .iter()
            .take(binders.len())
            .position(|j| matches!(j, Judgement::Match))
        {
            exact.push(Suggestion {
                kind: SuggestionKind::Exact {
                    binder: terms[pos].clone(),
                    hole: i,
                },
                verified: true,
            });
        }
        // rfl 候选被拒即丢弃，不出现。
        if let Some(term) = rfl_term {
            if judgements.get(rfl_base) == Some(&Judgement::Match) {
                rfl.push(Suggestion {
                    kind: SuggestionKind::Rfl { term },
                    verified: true,
                });
            }
        }
    }
    let mut out = exact;
    out.extend(rfl);
    if d.refine_template.is_some() {
        out.push(Suggestion {
            kind: SuggestionKind::Refine,
            verified: false,
        });
    }
    if intro_count(d) > 0 {
        out.push(Suggestion {
            kind: SuggestionKind::Intro,
            verified: false,
        });
    }
    out.truncate(MAX_SUGGESTIONS);
    out
}

/// 重启骨架最多剥的 Pi 层数（docs/design-kernel-taxonomy.md §2）。
const SKELETON_MAX_LAYERS: usize = 3;

/// `parse_expr_text` 内部用 `#check {text}` 承载表达式，AST span 相对类型
/// 文本整体偏移了这个前缀长度；切回原文本时据此扣掉。
const CHECK_PREFIX_LEN: usize = "#check ".len();

/// 一个剥出的望远镜层：写的 binder 名（Arrow 域是空名）、类型文本与
/// binder 风格。
struct SkeletonLayer {
    name: String,
    ty_text: String,
    style: BinderKind,
}

/// 失败声明的「重启骨架」：按声明类型的形状剥 Pi/Forall 望远镜（≤3 层），
/// 生成 `fun (x : A) => … => ???`。类型位与 `:=` 都经 tokenize 定位，类型
/// AST 按 span 精确切回原文本——不扫文本、不做文本比对（REQUIREMENTS
/// §2.8）。类型不可解析或不可剥（非 Pi、binder 无显式类型）时返回 `None`
/// （不出建议）。
fn restart_skeleton(decl_src: &str) -> Option<String> {
    let ty_text = decl_type_text(decl_src)?;
    let ty = parse_expr_text(ty_text).ok()?;
    let mut layers: Vec<SkeletonLayer> = Vec::new();
    let mut cur = &ty;
    loop {
        if layers.len() >= SKELETON_MAX_LAYERS {
            break;
        }
        match cur {
            Expr::Forall { binders, body, .. } => {
                for binder in binders {
                    if layers.len() >= SKELETON_MAX_LAYERS {
                        break;
                    }
                    layers.push(SkeletonLayer {
                        name: binder.name.clone(),
                        ty_text: check_slice(ty_text, binder.ty.as_ref()?.span())?.to_string(),
                        style: binder.style.clone(),
                    });
                }
                cur = body;
            }
            Expr::Arrow {
                domain, codomain, ..
            } => {
                layers.push(SkeletonLayer {
                    name: String::new(),
                    ty_text: check_slice(ty_text, domain.span())?.to_string(),
                    style: BinderKind::Explicit,
                });
                cur = codomain;
            }
            _ => break,
        }
    }
    if layers.is_empty() {
        return None;
    }
    // binder 名防撞（外层优先保留原名）：望远镜内同名或匿名域的默认名
    // `x` 撞上已有名字时追加序号（`x` → `x2` → …）。失败声明的值位会被
    // 整体替换，骨架自身的名字是唯一需要避免的碰撞面。
    let mut used = std::collections::HashSet::new();
    let mut named: Vec<(String, &SkeletonLayer)> = Vec::with_capacity(layers.len());
    for layer in &layers {
        let base = if layer.name.is_empty() {
            "x"
        } else {
            layer.name.as_str()
        };
        let mut name = base.to_string();
        let mut n = 2;
        while !used.insert(name.clone()) {
            name = format!("{base}{n}");
            n += 1;
        }
        named.push((name, layer));
    }
    let mut skeleton = String::from("???");
    for (name, layer) in named.iter().rev() {
        let binder = match layer.style {
            BinderKind::Explicit => format!("({name} : {})", layer.ty_text),
            BinderKind::Implicit => format!("{{{name} : {}}}", layer.ty_text),
        };
        skeleton = format!("fun {binder} => {skeleton}");
    }
    Some(skeleton)
}

/// 声明的类型文本：decl 命令里 `:=` 之前最近的顶层 `:` 之后到 `:=` 之前的
/// 切片（tokenize 精确定位，`--` 注释由词法器跳过）。没有 `:=`（axiom、
/// 无 iota 的 inductive）或没有顶层 `:` 时返回 `None`。
fn decl_type_text(decl_src: &str) -> Option<&str> {
    let tokens = tokenize(decl_src).ok()?;
    let colon_eq = tokens.iter().position(|t| t.kind == TokenKind::ColonEq)?;
    let mut depth = 0usize;
    let mut colon_end = None;
    for tok in &tokens[..colon_eq] {
        match tok.kind {
            TokenKind::LParen | TokenKind::LBrace => depth += 1,
            TokenKind::RParen | TokenKind::RBrace => depth = depth.saturating_sub(1),
            TokenKind::Colon if depth == 0 && colon_end.is_none() => {
                colon_end = Some(tok.span.end.offset);
            }
            _ => {}
        }
    }
    Some(&decl_src[colon_end?..tokens[colon_eq].span.start.offset])
}

/// 把（`#check {text}` 坐标系里的）AST span 切回类型文本。
fn check_slice(ty_text: &str, span: crate::Span) -> Option<&str> {
    ty_text.get(
        span.start.offset.checked_sub(CHECK_PREFIX_LEN)?
            ..span.end.offset.checked_sub(CHECK_PREFIX_LEN)?,
    )
}

/// 洞的期望类型文本：主洞用剩余目标；spine 洞用 walk 恢复的期望类型
/// （best-effort，`None` 时该洞不出 exact/rfl 建议）。
fn hole_goal_text(d: &DeclState, hole: usize) -> Option<String> {
    if d.sub_goals.is_empty() {
        return d.goal.clone();
    }
    d.sub_goals.get(hole)?.ty.clone()
}

/// 把剩余目标折进已写 binders 的判定规格（与 `exact` 时代一致）。
fn open_spec(d: &DeclState, ty: &str) -> OpenGoalSpec {
    OpenGoalSpec {
        universe: d.universe.clone(),
        ty: ty.to_string(),
        binders: d
            .binders
            .iter()
            .map(|b| GoalBinderSpec {
                name: b.name.clone(),
                ty: Some(b.ty.clone()),
            })
            .collect(),
    }
}

/// 前 4 个 binder（最内层优先，与旧 exact 的判定顺序一致）。
fn top_binders(d: &DeclState) -> Vec<String> {
    d.binders
        .iter()
        .rev()
        .take(BINDER_CANDIDATES)
        .map(|b| b.name.clone())
        .collect()
}

/// goal 是 Forall/Arrow 时的 intro 层数；0 表示不适用。
fn intro_count(d: &DeclState) -> usize {
    let Some(goal) = &d.goal else {
        return 0;
    };
    let Ok(goal) = parse_expr_text(goal) else {
        return 0;
    };
    match &goal {
        Expr::Forall { binders, .. } => binders.len(),
        Expr::Arrow { .. } => 1,
        _ => 0,
    }
}

/// 目标形如 `Eq α x y`（或显式 `@Eq.{u} α x y`）时的 rfl 候选
/// `Eq.refl.{u} <α> <a>`：kernel 裁决两边是否本来就是同一个值。
/// 宇宙层级：目标头写明 `Eq.{u}` 时取目标自身的层级，否则取声明的
/// 首个宇宙参数（无则 0）。
fn eq_refl_candidate(expected: &str, d: &DeclState) -> Option<String> {
    let goal = parse_expr_text(expected).ok()?;
    let (head, args) = spine_of(&goal);
    let level = match head {
        Expr::Ident { name, .. } if name == "Eq" => d
            .universe
            .first()
            .cloned()
            .unwrap_or_else(|| "0".to_string()),
        Expr::UniverseApp { name, levels, .. } if name == "Eq" && levels.len() == 1 => {
            levels[0].clone()
        }
        _ => return None,
    };
    if args.len() < 3 {
        return None;
    }
    let alpha = atom_text(args[0]);
    let a = atom_text(args[1]);
    Some(format!("Eq.refl.{{{level}}} {alpha} {a}"))
}

/// 展平 `f x1 … xn` 成 `(f, [x1, …, xn])`。
fn spine_of(expr: &Expr) -> (&Expr, Vec<&Expr>) {
    let mut args = Vec::new();
    let mut cur = expr;
    while let Expr::App { fun, arg, .. } = cur {
        args.push(arg.as_ref());
        cur = fun;
    }
    args.reverse();
    (cur, args)
}

/// 实参文本：复合表达式加括号，嵌入候选后仍按原子解析。
fn atom_text(expr: &Expr) -> String {
    let s = render_expr(expr);
    match expr {
        Expr::App { .. }
        | Expr::Lambda { .. }
        | Expr::Forall { .. }
        | Expr::Arrow { .. }
        | Expr::Plus { .. } => format!("({s})"),
        _ => s,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile::{check_document, DeclStatus};
    use crate::parse;

    /// 文档里第一个 open 练习的建议（prefix 取到该声明结束，与 actions 一致）。
    fn suggest_for(doc: &str) -> Vec<Suggestion> {
        let report = check_document(&parse(doc).expect("parses"));
        let d = report
            .decls
            .iter()
            .find(|d| d.status == DeclStatus::Open)
            .expect("open exercise")
            .clone();
        let prefix = &doc[..d.span.end.offset.min(doc.len())];
        suggest(prefix, None, &CompileOptions::default(), &d)
    }

    /// 文档里第一个 failed 声明的建议（decl_src 切片与 actions 一致）。
    fn suggest_for_failed(doc: &str) -> Vec<Suggestion> {
        let report = check_document(&parse(doc).expect("parses"));
        let d = report
            .decls
            .iter()
            .find(|d| d.status == DeclStatus::Failed)
            .expect("failed declaration")
            .clone();
        let prefix = &doc[..d.span.end.offset.min(doc.len())];
        let decl_src = &doc[d.span.start.offset..d.span.end.offset.min(doc.len())];
        suggest(prefix, Some(decl_src), &CompileOptions::default(), &d)
    }

    fn kinds(suggestions: &[Suggestion]) -> Vec<SuggestionKind> {
        suggestions.iter().map(|s| s.kind.clone()).collect()
    }

    #[test]
    fn single_hole_exact_is_kernel_verified_and_first() {
        let suggestions =
            suggest_for("example : (a : Prop) -> a -> a := fun (a : Prop) => fun (h : a) => ???\n");
        assert_eq!(
            kinds(&suggestions),
            vec![SuggestionKind::Exact {
                binder: "h".to_string(),
                hole: 0
            }],
            "goal `a` has no binder to peel; the exact suggestion is kernel-verified"
        );
        assert!(suggestions[0].verified);
    }

    const SPINE_DOC: &str = "axiom And : Prop -> Prop -> Prop\n\
axiom And.intro : (a : Prop) -> (b : Prop) -> a -> b -> And a b\n\
theorem t : (a : Prop) -> (b : Prop) -> (whole : And a b) -> (ha : a) -> (hb : b) -> And a b := \
fun (a : Prop) => fun (b : Prop) => fun (whole : And a b) => fun (ha : a) => fun (hb : b) => \
And.intro a b ??? ???\n";

    #[test]
    fn spine_holes_get_per_hole_exact_not_outer_goal_matches() {
        let suggestions = suggest_for(SPINE_DOC);
        assert_eq!(
            kinds(&suggestions),
            vec![
                SuggestionKind::Exact {
                    binder: "ha".to_string(),
                    hole: 0
                },
                SuggestionKind::Exact {
                    binder: "hb".to_string(),
                    hole: 1
                },
            ],
            "each sub-hole gets its own hypothesis; the outer-goal match `whole` \
             must not be stuffed into a sub-hole"
        );
        assert!(suggestions.iter().all(|s| s.verified));
    }

    const ONE_SUB_HOLE_DOC: &str = "axiom And : Prop -> Prop -> Prop\n\
axiom And.intro : (a : Prop) -> (b : Prop) -> a -> b -> And a b\n\
theorem t : (a : Prop) -> (b : Prop) -> (ha : a) -> (hb : b) -> And a b := \
fun (a : Prop) => fun (b : Prop) => fun (ha : a) => fun (hb : b) => And.intro a b ha ???\n";

    #[test]
    fn one_sub_hole_exact_verifies_the_whole_proof() {
        let suggestions = suggest_for(ONE_SUB_HOLE_DOC);
        assert_eq!(
            kinds(&suggestions),
            vec![SuggestionKind::Exact {
                binder: "hb".to_string(),
                hole: 0
            }],
            "the last sub-hole's fill completes the proof; the kernel judges it whole"
        );
        assert!(suggestions[0].verified);
    }

    #[test]
    fn eq_goal_gets_kernel_verified_rfl() {
        // 数字等式必须显式写 Eq.{1}（Nat : Sort 1；prelude 规则）。
        let suggestions =
            suggest_for("theorem eq_t : (a : Nat) -> Eq.{1} Nat a a := fun (a : Nat) => ???\n");
        assert_eq!(
            kinds(&suggestions),
            vec![SuggestionKind::Rfl {
                term: "Eq.refl.{1} Nat a".to_string()
            }],
            "no binder matches the Eq goal; rfl is the verified next step"
        );
        assert!(suggestions[0].verified);
    }

    #[test]
    fn rfl_of_computed_sides_still_verifies() {
        // 目标两边是 Nat.add 1 1 与 2：内核把左边算成 2，rfl 判定通过；
        // 宇宙层级取自目标自身的 Eq.{1}。
        let suggestions = suggest_for("theorem plus_t : Eq.{1} Nat (Nat.add 1 1) 2 := ???\n");
        assert_eq!(
            kinds(&suggestions),
            vec![SuggestionKind::Rfl {
                term: "Eq.refl.{1} Nat ((Nat.add 1) 1)".to_string()
            }],
        );
        assert!(suggestions[0].verified);
    }

    #[test]
    fn non_eq_goal_gets_no_rfl() {
        let suggestions = suggest_for("example : Prop -> Prop := ???\n");
        let ks = kinds(&suggestions);
        assert!(
            ks.iter().all(|k| !matches!(k, SuggestionKind::Rfl { .. })),
            "an Arrow goal is not an Eq: {ks:?}"
        );
        assert!(ks.iter().any(|k| matches!(k, SuggestionKind::Intro)));
    }

    #[test]
    fn rfl_comes_after_exact_in_order() {
        // 假设能直接结束目标，也有 kernel 验证过的 rfl：exact 在前。
        let suggestions = suggest_for(
            "theorem t : (a : Nat) -> (h : Eq.{1} Nat 2 2) -> Eq.{1} Nat 2 2 := \
fun (a : Nat) => fun (h : Eq.{1} Nat 2 2) => ???\n",
        );
        assert_eq!(
            kinds(&suggestions),
            vec![
                SuggestionKind::Exact {
                    binder: "h".to_string(),
                    hole: 0
                },
                SuggestionKind::Rfl {
                    term: "Eq.refl.{1} Nat 2".to_string()
                },
            ]
        );
        assert!(suggestions.iter().all(|s| s.verified));
    }

    const TRIPLE_DOC: &str = "axiom And : Prop -> Prop -> Prop\n\
axiom And.intro : (a : Prop) -> (b : Prop) -> a -> b -> And a b\n\
theorem t : (a : Prop) -> (b : Prop) -> (k : a -> b -> And a b) -> a -> b -> And a b := \
fun (a : Prop) => fun (b : Prop) => fun (k : a -> b -> And a b) => ???\n";

    #[test]
    fn ordering_exact_refine_intro() {
        let suggestions = suggest_for(TRIPLE_DOC);
        assert_eq!(
            kinds(&suggestions),
            vec![
                SuggestionKind::Exact {
                    binder: "k".to_string(),
                    hole: 0
                },
                SuggestionKind::Refine,
                SuggestionKind::Intro,
            ],
            "kernel-verified exact first, then structural refine, then intro"
        );
        assert_eq!(
            suggestions.iter().map(|s| s.verified).collect::<Vec<_>>(),
            vec![true, false, false]
        );
    }

    #[test]
    fn suggestions_capped_at_three() {
        // 四个子洞都能 exact：4 条截到 3 条（每请求 ≤3 条）。
        let suggestions = suggest_for(
            "axiom Quad : Prop -> Prop -> Prop -> Prop -> Prop\n\
axiom Quad.mk : (a : Prop) -> (b : Prop) -> (c : Prop) -> (d : Prop) -> \
a -> b -> c -> d -> Quad a b c d\n\
theorem t : (a : Prop) -> (b : Prop) -> (c : Prop) -> (d : Prop) -> \
(ha : a) -> (hb : b) -> (hc : c) -> (hd : d) -> Quad a b c d := \
fun (a : Prop) => fun (b : Prop) => fun (c : Prop) => fun (d : Prop) => \
fun (ha : a) => fun (hb : b) => fun (hc : c) => fun (hd : d) => \
Quad.mk a b c d ??? ??? ??? ???\n",
        );
        let ks = kinds(&suggestions);
        assert_eq!(ks.len(), 3, "at most three suggestions per request: {ks:?}");
        assert!(matches!(ks[0], SuggestionKind::Exact { hole: 0, .. }));
        assert!(matches!(ks[1], SuggestionKind::Exact { hole: 1, .. }));
        assert!(matches!(ks[2], SuggestionKind::Exact { hole: 2, .. }));
    }

    // ---- 失败声明的重启骨架（docs/design-kernel-taxonomy.md §2）----

    #[test]
    fn failed_decl_gets_one_restart_skeleton_shaped_like_its_type() {
        let suggestions =
            suggest_for_failed("example : (a : Prop) -> a -> a := fun (x : Prop) => 1\n");
        assert_eq!(
            kinds(&suggestions),
            vec![SuggestionKind::Restart {
                skeleton: "fun (a : Prop) => fun (x : a) => ???".to_string(),
            }],
            "the Pi telescope is peeled layer by layer; the anonymous Arrow \
             domain keeps its own type text"
        );
        assert!(
            !suggestions[0].verified,
            "the skeleton is structural: the kernel judges after the next edit"
        );
    }

    #[test]
    fn failed_decl_skeleton_restarts_as_an_open_exercise() {
        let suggestions =
            suggest_for_failed("example : (a : Prop) -> a -> a := fun (x : Prop) => 1\n");
        let SuggestionKind::Restart { skeleton } = &suggestions[0].kind else {
            panic!("expected a restart suggestion");
        };
        // 骨架落回值位后重查：kernel 接受它为合法起点，声明回到 Open，
        // 剩下最内层目标留给学生（kernel 逐层判）。
        let restarted = format!("example : (a : Prop) -> a -> a := {skeleton}\n");
        let report = check_document(&parse(&restarted).expect("parses"));
        assert_eq!(report.decls[0].status, DeclStatus::Open);
        assert_eq!(
            report.decls[0].goal.as_deref(),
            Some("a"),
            "the restart leaves the innermost goal open"
        );
    }

    #[test]
    fn failed_decl_renames_colliding_binder_names() {
        let suggestions = suggest_for_failed(
            "example : (x : Prop) -> x -> x := fun (x : Prop) => fun (h : x) => 1\n",
        );
        assert_eq!(
            kinds(&suggestions),
            vec![SuggestionKind::Restart {
                skeleton: "fun (x : Prop) => fun (x2 : x) => ???".to_string(),
            }],
            "the anonymous domain's default name `x` collides with the outer \
             binder and is renamed to `x2`"
        );
    }

    #[test]
    fn failed_long_telescope_caps_the_skeleton_at_three_layers() {
        let suggestions = suggest_for_failed(
            "example : (a : Prop) -> (b : Prop) -> (c : Prop) -> (d : Prop) -> a := \
fun (a : Prop) => fun (b : Prop) => fun (c : Prop) => fun (d : Prop) => 1\n",
        );
        assert_eq!(
            kinds(&suggestions),
            vec![SuggestionKind::Restart {
                skeleton: "fun (a : Prop) => fun (b : Prop) => fun (c : Prop) => ???".to_string(),
            }],
            "the telescope is capped at three layers; the rest stays a `???` goal"
        );
    }

    #[test]
    fn failed_decl_implicit_binder_keeps_its_style() {
        let suggestions =
            suggest_for_failed("example : {a : Prop} -> a -> a := fun (x : Prop) => 1\n");
        assert_eq!(
            kinds(&suggestions),
            vec![SuggestionKind::Restart {
                skeleton: "fun {a : Prop} => fun (x : a) => ???".to_string(),
            }],
            "an implicit telescope layer stays implicit in the skeleton"
        );
    }

    #[test]
    fn failed_decl_with_non_pi_type_gets_no_suggestion() {
        // kernel 拒绝但类型是 Prop（非 Pi）：没有可剥的望远镜，不出建议。
        assert!(suggest_for_failed("def bad : Prop := 1\n").is_empty());
    }
}

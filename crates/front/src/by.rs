//! `by` tactic 块引擎：把 `:= by intro a; exact h` 翻译成一个（可能带尾部
//! `sorry` 的）lambda AST，交给既有 `open_goal`/`build_theorem`/kernel 分流。
//!
//! 判定纪律（REQUIREMENTS §2.8）：每个 tactic 的裁决**永远走 kernel**——
//! 复用 [`crate::judge::judge_terms`]（合成完整声明交完整流水线裁决）与
//! [`crate::judge::judge_infer`]（推断被应用函数的类型）。`apply` 只做位置
//! spine 合一，不做完整高阶合一（教学子集）。
//!
//! 目标树：`apply` 会引入兄弟子目标（多目标），用节点 + 父指针建模；
//! 每个节点的 context = 根到该节点的所有 `intro` binder（沿父链收集）。

use crate::ast::{Binder, BinderKind, Expr, Tactic};
use crate::compile::CompileError;
use crate::compile::{CompileOptions, ErrorKind};
use crate::judge::{judge_infer, judge_terms, GoalBinderSpec, Judgement, OpenGoalSpec};
use crate::proof::{parse_expr_text, render_expr};
use crate::Span;

/// 编译期记录的每个 tactic 步执行后的 goal 状态（Phase 2 的 goal 面板用）。
#[derive(Debug, Clone, PartialEq)]
pub struct ByStep {
    /// 该 tactic 的源码 span。
    pub span: Span,
    /// 该步执行后的剩余目标；`None` = 所有目标已闭合。
    pub goal: Option<String>,
    /// 该步执行后的已引入假设（根到该步）。
    pub binders: Vec<Binder>,
}

/// 引擎结果：降级后的 lambda AST（可能带尾部 `Expr::Hole`）+ 每步状态。
pub struct ByOutcome {
    pub expr: Expr,
    pub steps: Vec<ByStep>,
}

/// 节点解决方案：洞 / 已闭合术语 / `apply`（f 应用于若干实参，其中子目标
/// 实参指向节点 id）。
#[derive(Clone)]
enum NodeKind {
    Hole,
    Closed(Expr),
    Apply { f: Expr, args: Vec<ApplyArg> },
}

#[derive(Clone)]
enum ApplyArg {
    TypeParam(Expr),
    SubGoal(usize),
}

struct GoalNode {
    ty: Expr,
    intros: Vec<Binder>,
    parent: Option<usize>,
    kind: NodeKind,
}

/// 值位是 `by` 块，或声明 binder 降级后「lambda 链 → by 块」的形态：
/// 返回 `(声明 binder, by 块)`；其它形态返回 `None`。
pub(crate) fn split_by_value(val: &Expr) -> Option<(Vec<Binder>, &Expr)> {
    let mut binders = Vec::new();
    let mut cur = val;
    while let Expr::Lambda {
        binders: bs, body, ..
    } = cur
    {
        binders.extend(bs.iter().cloned());
        cur = body;
    }
    if matches!(cur, Expr::By { .. }) {
        Some((binders, cur))
    } else {
        None
    }
}

/// 把 `ty`（声明类型）与 `by` 块降级成 lambda AST。
/// `initial_binders` 是声明级 binder（`theorem f (a : A) : B := by …` 里的
/// `a`）：它们是引擎的初始上下文，类型先剥掉对应层数，`by` 从 `B` 出发；
/// 没有声明 binder 时传空切片（旧行为）。
pub fn run_by(
    ty: &Expr,
    by: &Expr,
    initial_binders: &[Binder],
    prefix_src: &str,
    options: &CompileOptions,
) -> Result<ByOutcome, CompileError> {
    let Expr::By {
        tactics,
        span: by_span,
    } = by
    else {
        unreachable!("run_by called on non-By");
    };
    let root_ty = crate::proof::peel_pi_layers(ty, initial_binders.len()).ok_or_else(|| {
        let span = initial_binders.last().map(|b| b.span).unwrap_or(*by_span);
        CompileError::elab(
            ErrorKind::ElabTacticFailed,
            "internal: declared binders do not match the declaration type",
            span,
        )
    })?;
    let mut nodes: Vec<GoalNode> = Vec::new();
    let mut worklist: Vec<usize> = Vec::new();
    let mut steps: Vec<ByStep> = Vec::new();

    // 根目标 = 声明类型（先剥掉声明 binder）；声明 binder 进初始上下文。
    nodes.push(GoalNode {
        ty: root_ty,
        intros: initial_binders.to_vec(),
        parent: None,
        kind: NodeKind::Hole,
    });
    worklist.push(0);

    for tactic in tactics {
        // 当前要解的目标 = worklist 末尾。
        let cur = *worklist
            .last()
            .ok_or_else(|| tactic_error("by 块里没有待解目标", tactic.span()))?;
        match tactic {
            Tactic::Intro { name, span } => {
                let body = peel_pi(&nodes[cur].ty).ok_or_else(|| {
                    tactic_error(
                        "`intro` 需要一个函数目标（… -> … 或 forall …），当前目标不是函数",
                        *span,
                    )
                })?;
                let binder = Binder {
                    name: name.clone(),
                    ty: Some(Box::new(body.domain)),
                    style: BinderKind::Explicit,
                    span: *span,
                };
                nodes[cur].intros.push(binder);
                nodes[cur].ty = body.body;
            }
            Tactic::Exact { expr, span } => {
                let term = render_expr(expr);
                let j = judge(
                    prefix_src,
                    options,
                    &nodes,
                    &worklist,
                    cur,
                    &[term.as_str()],
                );
                match j {
                    Some(Judgement::Match) => {
                        nodes[cur].kind = NodeKind::Closed(expr.clone());
                        worklist.pop();
                    }
                    Some(Judgement::Mismatch { expected, actual }) => {
                        return Err(CompileError::elab(
                            ErrorKind::ElabTacticFailed,
                            format!("`exact` 类型不匹配：期望 `{expected}`，实际是 `{actual}`"),
                            *span,
                        ));
                    }
                    Some(Judgement::Error { message, .. }) => {
                        return Err(CompileError::elab(
                            ErrorKind::ElabTacticFailed,
                            format!("`exact` 判定失败：{message}"),
                            *span,
                        ));
                    }
                    None => unreachable!("judge returns one judgement per term"),
                }
            }
            Tactic::Assumption { span } => {
                let names: Vec<String> = context_binders(&nodes, cur)
                    .iter()
                    .rev()
                    .map(|b| b.name.clone())
                    .collect();
                let refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
                let js = judge(prefix_src, options, &nodes, &worklist, cur, &refs);
                let matched = js
                    .iter()
                    .position(|j| matches!(j, Judgement::Match))
                    .map(|i| names[i].clone());
                match matched {
                    Some(name) => {
                        let e = Expr::Ident { name, span: *span };
                        nodes[cur].kind = NodeKind::Closed(e);
                        worklist.pop();
                    }
                    None => {
                        return Err(CompileError::elab(
                            ErrorKind::ElabTacticFailed,
                            "`assumption` 没有找到类型与目标一致的假设",
                            *span,
                        ));
                    }
                }
            }
            Tactic::Rfl { span } => {
                let Some(candidate) = rfl_candidate(&nodes[cur].ty) else {
                    return Err(CompileError::elab(
                        ErrorKind::ElabTacticFailed,
                        "`rfl` 需要一个 `Eq α x y` 形状的目标",
                        *span,
                    ));
                };
                let j = judge(
                    prefix_src,
                    options,
                    &nodes,
                    &worklist,
                    cur,
                    &[candidate.as_str()],
                );
                match j.into_iter().next() {
                    Some(Judgement::Match) => {
                        let e = parse_expr_text(&candidate).map_err(|e| {
                            CompileError::elab(ErrorKind::ElabTacticFailed, e.to_string(), *span)
                        })?;
                        nodes[cur].kind = NodeKind::Closed(e);
                        worklist.pop();
                    }
                    Some(Judgement::Mismatch { expected, actual }) => {
                        return Err(CompileError::elab(
                            ErrorKind::ElabTacticFailed,
                            format!(
                                "`rfl` 判定失败：两边不相等（期望 `{expected}`，实际 `{actual}`）"
                            ),
                            *span,
                        ));
                    }
                    Some(Judgement::Error { message, .. }) => {
                        return Err(CompileError::elab(
                            ErrorKind::ElabTacticFailed,
                            format!("`rfl` 判定失败：{message}"),
                            *span,
                        ));
                    }
                    None => unreachable!(),
                }
            }
            Tactic::Apply { expr, span } => {
                apply_tactic(expr, *span, &mut nodes, &mut worklist, prefix_src, options)?;
            }
            // `sorry` = 占位：当前目标保持开放（no-op，节点仍是 Hole）。
            Tactic::Sorry { .. } => {}
        }
        // 记录当前（最新）被解目标的状态（Phase 2 goal 面板）。
        if let Some(&top) = worklist.last() {
            steps.push(ByStep {
                span: tactic.span(),
                goal: Some(render_expr(&nodes[top].ty)),
                binders: context_binders(&nodes, top),
            });
        } else {
            // 全部目标已闭合：记录闭合状态（goal = None）。
            steps.push(ByStep {
                span: tactic.span(),
                goal: None,
                binders: Vec::new(),
            });
        }
    }

    let expr = assemble(&nodes, 0, hole_span(tactics, *by_span));
    Ok(ByOutcome { expr, steps })
}

/// 未闭合目标的洞位：以最后一个 tactic 的 span 为准（`by … sorry` 里就是
/// `sorry` 的位置；`by intro a` 这种没写 sorry 的部分作答则落在块尾）。
fn hole_span(tactics: &[Tactic], by_span: Span) -> Span {
    tactics.last().map(Tactic::span).unwrap_or(by_span)
}

/// `apply f`：推断 f 的类型，位置 spine 合一 codomain 与目标，
/// 类型参数由 σ 填充，其余 binder 变成子目标。
fn apply_tactic(
    expr: &Expr,
    span: Span,
    nodes: &mut Vec<GoalNode>,
    worklist: &mut Vec<usize>,
    prefix_src: &str,
    options: &CompileOptions,
) -> Result<(), CompileError> {
    let cur = *worklist.last().ok_or_else(|| {
        CompileError::elab(ErrorKind::ElabTacticFailed, "by 块里没有待解目标", span)
    })?;
    let term = render_expr(expr);
    let spec = spec_of(nodes, cur);
    let f_ty = judge_infer(prefix_src, options, &spec.binders, &term).map_err(|j| match j {
        Judgement::Mismatch { .. } => CompileError::elab(
            ErrorKind::ElabTacticFailed,
            "无法推断被应用函数的类型",
            span,
        ),
        Judgement::Error { message, .. } => {
            CompileError::elab(ErrorKind::ElabTacticFailed, message, span)
        }
        Judgement::Match => unreachable!(),
    })?;
    let f_ty_expr = match parse_expr_text(&f_ty) {
        Ok(e) => e,
        Err(e) => {
            return Err(CompileError::elab(
                ErrorKind::ElabTacticFailed,
                format!("无法解析 `{term}` 的类型：{e}"),
                span,
            ));
        }
    };
    // 剥 Pi 链。
    let mut layers: Vec<(String, Expr)> = Vec::new();
    let mut t = f_ty_expr;
    let codomain = loop {
        let Some(pi) = peel_pi(&t) else {
            break t;
        };
        layers.push((pi.name, pi.domain));
        t = pi.body;
    };
    // 位置 spine 合一：codomain 与当前目标头相同、逐参数位对应 → σ。
    let goal = nodes[cur].ty.clone();
    let sigma = unify_spine(&codomain, &goal, &layers).ok_or_else(|| {
        CompileError::elab(
            ErrorKind::ElabTacticFailed,
            format!(
                "`apply` 的目标不匹配：`{}` 的结果是 `{}`，无法对齐当前目标 `{}`",
                render_expr(expr),
                render_expr(&codomain),
                render_expr(&goal)
            ),
            span,
        )
    })?;
    // 按层序构造实参：codomain 里出现的命名 binder = 类型参数（σ 填充）；
    // 其余 = 子目标（域类型 σ 代入）。
    let mut args: Vec<ApplyArg> = Vec::new();
    let mut sub_nodes: Vec<usize> = Vec::new();
    for (name, domain) in &layers {
        if !name.is_empty() && mentions(name, &codomain) {
            let filled = sigma.get(name).cloned().unwrap_or_else(|| Expr::Ident {
                name: name.clone(),
                span,
            });
            args.push(ApplyArg::TypeParam(filled));
        } else {
            let sub_ty = substitute(domain, &sigma);
            let id = nodes.len();
            nodes.push(GoalNode {
                ty: sub_ty,
                intros: Vec::new(),
                parent: Some(cur),
                kind: NodeKind::Hole,
            });
            args.push(ApplyArg::SubGoal(id));
            sub_nodes.push(id);
        }
    }
    // 子目标反序压 worklist（先解第一个）。
    for id in sub_nodes.iter().rev() {
        worklist.push(*id);
    }
    nodes[cur].kind = NodeKind::Apply {
        f: expr.clone(),
        args,
    };
    // 当前目标被 apply 消耗，不再是 worklist 顶（已 pop）。
    worklist.retain(|&x| x != cur);
    Ok(())
}

/// 组装整棵树 → 单一 lambda AST（叶子洞 = `Expr::Hole`，span 用未闭合目标
/// 的洞位——`by … sorry` 里就是 `sorry` 的位置，供 open_goal/hole 导航使用）。
fn assemble(nodes: &[GoalNode], id: usize, hole_span: Span) -> Expr {
    let node = &nodes[id];
    let body = match &node.kind {
        NodeKind::Hole => Expr::Hole { span: hole_span },
        NodeKind::Closed(e) => e.clone(),
        NodeKind::Apply { f, args } => {
            let mut app = f.clone();
            for arg in args {
                let arg_expr = match arg {
                    ApplyArg::TypeParam(e) => e.clone(),
                    ApplyArg::SubGoal(i) => assemble(nodes, *i, hole_span),
                };
                app = Expr::App {
                    fun: Box::new(app),
                    arg: Box::new(arg_expr),
                    span: Span::default(),
                };
            }
            app
        }
    };
    let mut expr = body;
    for binder in node.intros.iter().rev() {
        expr = Expr::Lambda {
            binders: vec![binder.clone()],
            body: Box::new(expr),
            span: Span::default(),
        };
    }
    expr
}

/// 目标节点 `id` 的判定规格（剩余目标 + 沿父链的全部 intros）。
fn spec_of(nodes: &[GoalNode], id: usize) -> OpenGoalSpec {
    OpenGoalSpec {
        universe: Vec::new(),
        ty: render_expr(&nodes[id].ty),
        binders: context_binders(nodes, id)
            .iter()
            .map(|b| GoalBinderSpec {
                name: b.name.clone(),
                ty: b.ty.as_deref().map(render_expr),
            })
            .collect(),
    }
}

/// 沿父链收集 intros（根在前）。
fn context_binders(nodes: &[GoalNode], id: usize) -> Vec<Binder> {
    let mut chain = Vec::new();
    let mut cur = Some(id);
    while let Some(i) = cur {
        chain.push(i);
        cur = nodes[i].parent;
    }
    chain.reverse();
    chain
        .into_iter()
        .flat_map(|i| nodes[i].intros.clone())
        .collect()
}

/// 对 `term` 在当前目标上下文做 kernel 判定（`judge_terms`）。
fn judge(
    prefix_src: &str,
    options: &CompileOptions,
    nodes: &[GoalNode],
    worklist: &[usize],
    cur: usize,
    terms: &[&str],
) -> Option<Judgement> {
    let spec = spec_of(nodes, cur);
    let _ = worklist;
    judge_terms(prefix_src, options, &spec, terms)
        .into_iter()
        .next()
}

/// `intro`：从目标剥一层 Pi/Arrow，返回 (name, domain, body)。
struct PiBody {
    name: String,
    domain: Expr,
    body: Expr,
}

fn peel_pi(expr: &Expr) -> Option<PiBody> {
    match expr {
        Expr::Forall { binders, body, .. } if !binders.is_empty() => {
            let b = &binders[0];
            let domain = b.ty.as_deref().cloned()?;
            // pp 会把 `(a : T) -> (b : T)` 折叠成 `forall (a b : T), ...`——
            // 逐 binder 剥，余下的重包成 Forall。
            let rest = if binders.len() > 1 {
                Expr::Forall {
                    binders: binders[1..].to_vec(),
                    body: body.clone(),
                    span: b.span,
                }
            } else {
                body.as_ref().clone()
            };
            Some(PiBody {
                name: b.name.clone(),
                domain,
                body: rest,
            })
        }
        Expr::Arrow {
            domain, codomain, ..
        } => Some(PiBody {
            name: String::new(),
            domain: domain.as_ref().clone(),
            body: codomain.as_ref().clone(),
        }),
        _ => None,
    }
}

/// 位置 spine 合一：`codomain` 与 `goal` 同头（Ident/UniverseApp 相同）时
/// 逐参数位对应，把 `layers` 里命名 binder 名 → goal 实参。
fn unify_spine(
    codomain: &Expr,
    goal: &Expr,
    layers: &[(String, Expr)],
) -> Option<std::collections::HashMap<String, Expr>> {
    let (chead, cargs) = spine_of(codomain);
    let (ghead, gargs) = spine_of(goal);
    if !same_head(chead, ghead) || cargs.len() != gargs.len() {
        return None;
    }
    // 把 codomain 实参位与 goal 实参位对应；codomain 里的命名 binder 是
    // 类型参数，映射到 goal 实参。
    let mut sigma = std::collections::HashMap::new();
    // 收集 codomain 实参里出现的命名 binder。
    for (name, _) in layers {
        for (carg, garg) in cargs.iter().copied().zip(gargs.iter().copied()) {
            if let Expr::Ident { name: n, .. } = carg {
                if n.as_str() == name.as_str() {
                    sigma.insert(name.clone(), garg.clone());
                }
            }
        }
    }
    Some(sigma)
}

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

fn same_head(a: &Expr, b: &Expr) -> bool {
    match (a, b) {
        (Expr::Ident { name: x, .. }, Expr::Ident { name: y, .. }) => x == y,
        (
            Expr::UniverseApp {
                name: x,
                levels: lx,
                ..
            },
            Expr::UniverseApp {
                name: y,
                levels: ly,
                ..
            },
        ) => x == y && lx == ly,
        _ => false,
    }
}

/// `name` 是否出现在 `expr` 的 Ident 节点里。
fn mentions(name: &str, expr: &Expr) -> bool {
    match expr {
        Expr::Ident { name: n, .. } => n == name,
        Expr::UniverseApp { .. } | Expr::Sort { .. } | Expr::Num { .. } | Expr::Hole { .. } => {
            false
        }
        Expr::Intro { .. } => false,
        Expr::App { fun, arg, .. } => mentions(name, fun) || mentions(name, arg),
        Expr::Lambda { binders, body, .. } | Expr::Forall { binders, body, .. } => {
            binders
                .iter()
                .any(|b| b.ty.as_deref().is_some_and(|t| mentions(name, t)))
                || mentions(name, body)
        }
        Expr::Arrow {
            domain, codomain, ..
        } => mentions(name, domain) || mentions(name, codomain),
        Expr::Plus { lhs, rhs, .. } => mentions(name, lhs) || mentions(name, rhs),
        Expr::By { .. } => false,
    }
}

/// 把 σ 代入 `expr` 里的命名 Ident（不改 span）。
fn substitute(expr: &Expr, sigma: &std::collections::HashMap<String, Expr>) -> Expr {
    match expr {
        Expr::Ident { name, span } => match sigma.get(name) {
            Some(repl) => repl.clone(),
            None => Expr::Ident {
                name: name.clone(),
                span: *span,
            },
        },
        Expr::Sort { sort, span } => Expr::Sort {
            sort: sort.clone(),
            span: *span,
        },
        Expr::UniverseApp { name, levels, span } => Expr::UniverseApp {
            name: name.clone(),
            levels: levels.clone(),
            span: *span,
        },
        Expr::Num { value, span } => Expr::Num {
            value: value.clone(),
            span: *span,
        },
        Expr::Hole { span } => Expr::Hole { span: *span },
        Expr::Intro { span } => Expr::Intro { span: *span },
        Expr::App { fun, arg, span } => Expr::App {
            fun: Box::new(substitute(fun, sigma)),
            arg: Box::new(substitute(arg, sigma)),
            span: *span,
        },
        Expr::Lambda {
            binders,
            body,
            span,
        } => Expr::Lambda {
            binders: binders.clone(),
            body: Box::new(substitute(body, sigma)),
            span: *span,
        },
        Expr::Forall {
            binders,
            body,
            span,
        } => Expr::Forall {
            binders: binders.clone(),
            body: Box::new(substitute(body, sigma)),
            span: *span,
        },
        Expr::Arrow {
            domain,
            codomain,
            span,
        } => Expr::Arrow {
            domain: Box::new(substitute(domain, sigma)),
            codomain: Box::new(substitute(codomain, sigma)),
            span: *span,
        },
        Expr::Plus { lhs, rhs, span } => Expr::Plus {
            lhs: Box::new(substitute(lhs, sigma)),
            rhs: Box::new(substitute(rhs, sigma)),
            span: *span,
        },
        Expr::By { tactics, span } => Expr::By {
            tactics: tactics.clone(),
            span: *span,
        },
    }
}

/// `rfl` 候选：目标 `Eq α x y` → `Eq.refl.{u} α x`（kernel 判定两边）。
fn rfl_candidate(goal: &Expr) -> Option<String> {
    let (head, args) = spine_of(goal);
    let level = match head {
        Expr::Ident { name, .. } if name == "Eq" => "0".to_string(),
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

fn tactic_error(msg: impl Into<String>, span: Span) -> CompileError {
    CompileError::elab(ErrorKind::ElabTacticFailed, msg, span)
}

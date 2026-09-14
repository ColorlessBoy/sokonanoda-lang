//! Goal walk：从「部分作答」恢复剩余目标与子洞期望类型。
//!
//! 本模块承载 open exercise 的语法级 walk（自 check.rs 抽出，2026-09-10）：
//! * lambda 链逐层消耗声明类型的 Pi；
//! * **构造子 spine**：值头是目标族构造子时，参数位由目标自动判定；
//! * **函数实参洞**（v1）：值是已知函数/常量的应用时，直接实参中的 `sorry`
//!   按函数望远镜实例化出期望类型（`Eq.subst.{1} Nat (sorry) …` → `Nat -> Prop`）。
//!
//! 全部产物只是**建议**：判定永远由完整内核在填洞后终审（REQUIREMENTS §2 第 8 条）。

use super::prelude::{CompileOptions, PreludeMode, PRELUDE_EQ_SRC};
use super::report::{GoalBinder, SubGoal};
use crate::proof::render_expr;
use crate::{Binder, Command, Expr, FolFile, Span};
use std::collections::HashMap;

/// 已知函数模板：函数望远镜（binder 名 + 类型），键 = 函数名（`Eq.subst`）。
/// `universe` 是该声明自身的宇宙参数名（源内 `{u}`），用于把期望类型里的
/// `Sort u` / `. {u}` 替换成调用点写的层级（`Eq.subst.{1}` → `Sort 1`）。
#[derive(Debug, Clone, Default)]
struct FuncTemplate {
    universe: Vec<String>,
    binder_names: Vec<String>,
    binder_tys: Vec<Option<Expr>>,
    /// 声明类型剥完望远镜后的结果（axiom/def/theorem 有；归纳构造子的
    /// func 视图没有）。超量应用走查需要它：实参落在结果之上时，把结果
    /// 按 def 体展开继续匹配。
    result_ty: Option<Expr>,
    /// def 的值（仅 `Command::Def`）：结果类型是 def 应用时展开一步
    /// （`Not a` ⇒ 体 `fun (a : Prop) => a -> False` 代入后 = `a -> False`）。
    def_body: Option<Expr>,
}

/// 构造子模板：来自归纳块构造子或「结果头是族应用」的源内 axiom。
#[derive(Debug, Clone)]
struct CtorTemplate {
    /// 构造子自身名字（`And.intro`），refine 骨架用。
    name: String,
    binder_names: Vec<String>,
    binder_tys: Vec<Option<Expr>>,
    result_arg_names: Vec<Option<String>>,
}

/// 目标 walk 的模板索引：
/// * `ctors` 键 = 族头名（`And`），来源为归纳块构造子与族结果 axiom；
/// * `funcs` 键 = 函数/常量名（`Eq.subst`），来源为源内 axiom/def/theorem、
///   归纳块构造子，以及 Full 模式下未被文件接管的 Eq prelude。
pub(crate) struct GoalTemplates {
    ctors: HashMap<String, CtorTemplate>,
    funcs: HashMap<String, FuncTemplate>,
}

impl GoalTemplates {
    /// 从文件命令与 prelude 选项构建索引。纯建议材料——内核仍是唯一裁判。
    pub(crate) fn new_for(file: &FolFile, options: &CompileOptions) -> Self {
        let mut templates = Self {
            ctors: HashMap::new(),
            funcs: HashMap::new(),
        };
        // Eq prelude 是受信任安装（源码文本预置），只在 Full 且文件没有
        // 自己占用 `Eq` 三名时参与模板（与 `install_eq_prelude` 同规则）。
        if options.prelude == PreludeMode::Full && !file_owns_eq(file) {
            if let Ok(parsed) = crate::parse(PRELUDE_EQ_SRC) {
                for command in &parsed.commands {
                    if let Command::Axiom {
                        name, universe, ty, ..
                    } = command
                    {
                        templates.insert_func(name, universe, ty);
                    }
                }
            }
        }
        for command in &file.commands {
            match command {
                Command::InductiveBlock {
                    name, constructors, ..
                } => {
                    for ctor in constructors {
                        let binder_names = ctor.binders.iter().map(|b| b.name.clone()).collect();
                        let binder_tys = ctor
                            .binders
                            .iter()
                            .map(|b| b.ty.as_deref().cloned())
                            .collect();
                        templates.ctors.entry(name.clone()).or_insert(CtorTemplate {
                            name: ctor.name.clone(),
                            binder_names,
                            binder_tys,
                            result_arg_names: Vec::new(),
                        });
                        templates.funcs.insert(
                            ctor.name.clone(),
                            FuncTemplate {
                                universe: Vec::new(),
                                binder_names: ctor.binders.iter().map(|b| b.name.clone()).collect(),
                                binder_tys: ctor
                                    .binders
                                    .iter()
                                    .map(|b| b.ty.as_deref().cloned())
                                    .collect(),
                                result_ty: None,
                                def_body: None,
                            },
                        );
                    }
                }
                Command::Def {
                    name,
                    universe,
                    ty,
                    val,
                    ..
                } => {
                    // def 记录体：结果类型是 def 应用时展开一步（`Not a`
                    // ⇒ `a -> False`），超量应用的 spine 走查靠它继续。
                    templates.insert_def_func(name, universe, ty, Some(val.clone()));
                }
                Command::Theorem {
                    name, universe, ty, ..
                } => {
                    templates.insert_def_func(name, universe, ty, None);
                }
                Command::Axiom {
                    name, universe, ty, ..
                } => {
                    let mut binders = Vec::new();
                    let result = peel_type(ty, &mut binders);
                    templates.insert_func_with_binders(
                        name,
                        universe,
                        binders.clone(),
                        Some(result.clone()),
                        None,
                    );
                    // 构造子索引：结果头是族应用的 axiom 才是 ctor 模板
                    //（与既有 I9 多洞语义一致）。
                    if let Some((head, result_args)) = spine_head_args(&result) {
                        if !result_args.is_empty() {
                            let binder_names = binders.iter().map(|(n, _)| n.clone()).collect();
                            let binder_tys = binders.iter().map(|(_, t)| t.clone()).collect();
                            let result_arg_names = result_args
                                .iter()
                                .map(|arg| match arg {
                                    Expr::Ident { name, .. } => Some(name.clone()),
                                    _ => None,
                                })
                                .collect();
                            templates.ctors.entry(head).or_insert(CtorTemplate {
                                name: name.clone(),
                                binder_names,
                                binder_tys,
                                result_arg_names,
                            });
                        }
                    }
                }
                _ => {}
            }
        }
        templates
    }

    fn insert_func(&mut self, name: &str, universe: &[String], ty: &Expr) {
        self.insert_def_func(name, universe, ty, None);
    }

    /// def/theorem/axiom 共用：剥望远镜存层，def 另存体；结果 = 望远镜
    /// 剥完的残余（超量应用走查的起点）。
    fn insert_def_func(
        &mut self,
        name: &str,
        universe: &[String],
        ty: &Expr,
        def_body: Option<Expr>,
    ) {
        let mut binders = Vec::new();
        let result = peel_type(ty, &mut binders);
        self.insert_func_with_binders(name, universe, binders, Some(result), def_body);
    }

    fn insert_func_with_binders(
        &mut self,
        name: &str,
        universe: &[String],
        binders: Vec<(String, Option<Expr>)>,
        result_ty: Option<Expr>,
        def_body: Option<Expr>,
    ) {
        self.funcs.insert(
            name.to_string(),
            FuncTemplate {
                universe: universe.to_vec(),
                binder_names: binders.iter().map(|(n, _)| n.clone()).collect(),
                binder_tys: binders.into_iter().map(|(_, t)| t).collect(),
                result_ty,
                def_body,
            },
        );
    }
}

/// 文件是否自己声明了 `Eq` 三件套中的任意一个（与 `install_eq_prelude`
/// 的 all-or-nothing 规则镜像；此时 prelude 整体跳过）。
fn file_owns_eq(file: &FolFile) -> bool {
    const EQ_NAMES: [&str; 3] = ["Eq", "Eq.refl", "Eq.subst"];
    file.commands.iter().any(|command| match command {
        Command::Def { name, .. }
        | Command::Theorem { name, .. }
        | Command::Axiom { name, .. }
        | Command::InductiveBlock { name, .. } => EQ_NAMES.contains(&name.as_str()),
        _ => false,
    })
}

/// The walk's answer for an open exercise: the remaining goal (rendered), the
/// lambda binders already written, every hole span, expected types for
/// sub-holes, and a refine skeleton when the goal's head is a known constructor.
pub(crate) struct OpenGoalInfo {
    pub goal: String,
    pub binders: Vec<GoalBinder>,
    pub holes: Vec<Span>,
    pub sub_goals: Vec<SubGoal>,
    pub refine_template: Option<String>,
}

/// Is the answer an open exercise: does it contain a `sorry`, and can the
/// remaining goal be recovered by walking the declared type alongside the
/// lambda binders already written (and constructor/function spine arguments)?
/// `None` means "no hole" or "hole in a place the goal cannot be recovered
/// from" (the latter falls through to normal elaboration, which reports
/// `elab-hole-misplaced` at the hole).
pub(crate) fn open_goal(ty: &Expr, val: &Expr, templates: &GoalTemplates) -> Option<OpenGoalInfo> {
    if !expr_has_hole(val) {
        return None;
    }
    let mut locals = HashMap::new();
    local_func_templates(val, &mut locals);
    goal_under_binders(ty, val, templates, &locals)
}

/// 值位 `funapply` 的**局部假设**模板覆盖层。
///
/// `apply h`（`h` 是当前 lambda 链已引入的假设）降低成 `h sorry` 之后，spine
/// 走查要能认出 `h` 的望远镜——但 [`GoalTemplates.funcs`] 只有全局名字，局部
/// 假设不在其中。这里沿答案的 lambda 链把「本声明已引入的假设」收成一张局部
/// 覆盖表，`func_spine_case` 在全局查不到时回退到这里。
///
/// 只收集**带类型**的假设（无类型的 binder 无法实例化期望类型，交回 elab 报
/// `elab-untyped-binder`）。
fn local_func_templates(val: &Expr, out: &mut HashMap<String, FuncTemplate>) {
    let mut cur = val;
    while let Expr::Lambda { binders, body, .. } = cur {
        for binder in binders {
            let Some(ty) = binder.ty.as_deref() else {
                continue;
            };
            let mut telescope = Vec::new();
            peel_type(ty, &mut telescope);
            out.insert(
                binder.name.clone(),
                FuncTemplate {
                    universe: Vec::new(),
                    binder_names: telescope.iter().map(|(n, _)| n.clone()).collect(),
                    binder_tys: telescope.into_iter().map(|(_, t)| t).collect(),
                    result_ty: None,
                    def_body: None,
                },
            );
        }
        cur = body;
    }
}

pub(crate) fn expr_has_hole(e: &Expr) -> bool {
    match e {
        Expr::Hole { .. } => true,
        Expr::App { fun, arg, .. }
        | Expr::Plus {
            lhs: fun, rhs: arg, ..
        } => expr_has_hole(fun) || expr_has_hole(arg),
        Expr::Lambda { binders, body, .. } | Expr::Forall { binders, body, .. } => {
            binders
                .iter()
                .any(|b| b.ty.as_deref().is_some_and(expr_has_hole))
                || expr_has_hole(body)
        }
        Expr::Arrow {
            domain, codomain, ..
        } => expr_has_hole(domain) || expr_has_hole(codomain),
        Expr::Let {
            binder, val, body, ..
        } => {
            binder.ty.as_deref().is_some_and(expr_has_hole)
                || expr_has_hole(val)
                || expr_has_hole(body)
        }
        _ => false,
    }
}

/// Every `sorry` span in `e`, source order (used to surface value-position
/// holes in a `let` as sub-goals whose expected type is the binder's `T`).
fn collect_hole_spans(e: &Expr, out: &mut Vec<Span>) {
    match e {
        Expr::Hole { span } => out.push(*span),
        Expr::App { fun, arg, .. }
        | Expr::Plus {
            lhs: fun, rhs: arg, ..
        } => {
            collect_hole_spans(fun, out);
            collect_hole_spans(arg, out);
        }
        Expr::Lambda { binders, body, .. } | Expr::Forall { binders, body, .. } => {
            for binder in binders {
                if let Some(ty) = binder.ty.as_deref() {
                    collect_hole_spans(ty, out);
                }
            }
            collect_hole_spans(body, out);
        }
        Expr::Arrow {
            domain, codomain, ..
        } => {
            collect_hole_spans(domain, out);
            collect_hole_spans(codomain, out);
        }
        Expr::Let {
            binder, val, body, ..
        } => {
            if let Some(ty) = binder.ty.as_deref() {
                collect_hole_spans(ty, out);
            }
            collect_hole_spans(val, out);
            collect_hole_spans(body, out);
        }
        _ => {}
    }
}

/// Flatten `C x1 … xn` (or `C.{u} x1 … xn`) into `(C, [x1, …, xn])`.
/// Anything whose head is not an identifier (lambda, pi, sort, hole, …) is
/// not a spine.
fn spine_head_args(e: &Expr) -> Option<(String, Vec<&Expr>)> {
    match e {
        Expr::Ident { name, .. } | Expr::UniverseApp { name, .. } => {
            Some((name.clone(), Vec::new()))
        }
        Expr::App { fun, arg, .. } => {
            let (head, mut args) = spine_head_args(fun)?;
            args.push(arg);
            Some((head, args))
        }
        _ => None,
    }
}

/// The base identifier of an application chain (`Eq.subst.{1} Nat …` →
/// `Eq.subst.{1}`), carrying any explicit universe levels.
fn spine_base(e: &Expr) -> Option<&Expr> {
    match e {
        Expr::App { fun, .. } => spine_base(fun),
        Expr::Ident { .. } | Expr::UniverseApp { .. } => Some(e),
        _ => None,
    }
}

/// Flatten a type into `(binder name, binder type)` pairs plus the result.
fn peel_type(ty: &Expr, out: &mut Vec<(String, Option<Expr>)>) -> Expr {
    match ty {
        Expr::Forall { binders, body, .. } => {
            for binder in binders {
                out.push((binder.name.clone(), binder.ty.as_deref().cloned()));
            }
            peel_type(body, out)
        }
        Expr::Arrow {
            domain, codomain, ..
        } => {
            // Anonymous binder (no name): a proof field that cannot be
            // auto-filled or name-mapped.
            out.push((String::new(), Some(domain.as_ref().clone())));
            peel_type(codomain, out)
        }
        other => other.clone(),
    }
}

/// Render a goal-type argument, parenthesizing compound expressions so it can
/// be spliced into a template argument list.
fn render_arg(expr: &Expr) -> String {
    match expr {
        Expr::Ident { .. } | Expr::Num { .. } => render_expr(expr),
        other => format!("({})", render_expr(other)),
    }
}

/// Field binder i's filled value: when its name is one of the constructor
/// result's arguments, the goal's argument at that position is the value.
fn template_arg(template: &CtorTemplate, i: usize, ty_args: &[&Expr]) -> Option<String> {
    let name = template.binder_names.get(i)?;
    let j = template
        .result_arg_names
        .iter()
        .position(|n| n.as_deref() == Some(name.as_str()))?;
    let arg = ty_args.get(j)?;
    Some(render_arg(arg))
}

/// Deep-substitute template binder names with ASTs (shadow-guarded: a
/// Forall/Lambda binder named like a key stops substitution beneath it —
/// innermost wins, like elab). `levels` maps the declaration's universe
/// parameter names to the call site's level texts (`u` → `1`).
fn substitute_names(
    expr: &Expr,
    map: &HashMap<String, Expr>,
    levels: &HashMap<String, String>,
) -> Expr {
    match expr {
        Expr::Ident { name, span } => match map.get(name) {
            // The replacement keeps the hit node's span so diagnostics point
            // at the substituted site.
            Some(replacement) => with_root_span(replacement.clone(), *span),
            None => expr.clone(),
        },
        Expr::App { fun, arg, .. } => Expr::App {
            fun: Box::new(substitute_names(fun, map, levels)),
            arg: Box::new(substitute_names(arg, map, levels)),
            span: expr.span(),
        },
        Expr::Arrow {
            domain, codomain, ..
        } => Expr::Arrow {
            domain: Box::new(substitute_names(domain, map, levels)),
            codomain: Box::new(substitute_names(codomain, map, levels)),
            span: expr.span(),
        },
        Expr::Plus { lhs, rhs, .. } => Expr::Plus {
            lhs: Box::new(substitute_names(lhs, map, levels)),
            rhs: Box::new(substitute_names(rhs, map, levels)),
            span: expr.span(),
        },
        Expr::Lambda {
            binders,
            body,
            span,
        } => {
            let (binders, sub) = substitute_binders(binders, map, levels);
            Expr::Lambda {
                binders,
                body: Box::new(substitute_names(body, &sub, levels)),
                span: *span,
            }
        }
        Expr::Forall {
            binders,
            body,
            span,
        } => {
            let (binders, sub) = substitute_binders(binders, map, levels);
            Expr::Forall {
                binders,
                body: Box::new(substitute_names(body, &sub, levels)),
                span: *span,
            }
        }
        Expr::Sort { sort, span } => match sort {
            crate::SortKind::Level(name) => match levels.get(name) {
                Some(text) => Expr::Sort {
                    sort: level_sort(text),
                    span: *span,
                },
                None => expr.clone(),
            },
            _ => expr.clone(),
        },
        Expr::UniverseApp {
            name,
            levels: lv,
            span,
        } => Expr::UniverseApp {
            name: name.clone(),
            levels: lv
                .iter()
                .map(|l| levels.get(l).cloned().unwrap_or_else(|| l.clone()))
                .collect(),
            span: *span,
        },
        Expr::Let {
            binder,
            val,
            body,
            span,
        } => {
            // binder 类型与值在 binder 自己的作用域之外；body 在其内，屏蔽同名。
            let mut binder = binder.clone();
            let ty = binder.ty.take();
            binder.ty = ty.map(|ty| Box::new(substitute_names(&ty, map, levels)));
            let mut sub = map.clone();
            sub.remove(&binder.name);
            Expr::Let {
                binder,
                val: Box::new(substitute_names(val, map, levels)),
                body: Box::new(substitute_names(body, &sub, levels)),
                span: *span,
            }
        }
        Expr::Num { .. } | Expr::Hole { .. } => expr.clone(),
        Expr::By { .. } => expr.clone(), // by 块在 elab 前已降级，不应出现在此
    }
}

/// A level text (`1`) becomes a literal sort; a name stays symbolic.
fn level_sort(text: &str) -> crate::SortKind {
    match text.parse::<u64>() {
        Ok(n) => crate::SortKind::Sort(n),
        Err(_) => crate::SortKind::Level(text.to_string()),
    }
}

/// Rewrite a Forall/Lambda's binder telescope and compute the map that
/// governs its body: a binder's name stops its own key's substitution
/// beneath (innermost wins), while its declared type sits outside its own
/// scope and still sees the earlier siblings of the same group.
fn substitute_binders(
    binders: &[Binder],
    map: &HashMap<String, Expr>,
    levels: &HashMap<String, String>,
) -> (Vec<Binder>, HashMap<String, Expr>) {
    let mut sub = map.clone();
    let binders = binders
        .iter()
        .map(|binder| {
            let mut binder = binder.clone();
            let ty = binder.ty.take();
            binder.ty = ty.map(|ty| Box::new(substitute_names(&ty, &sub, levels)));
            sub.remove(&binder.name);
            binder
        })
        .collect();
    (binders, sub)
}

/// Clone of `expr` with its root span replaced: every variant carries a
/// span field, so each variant is rebuilt with the given span.
fn with_root_span(expr: Expr, span: Span) -> Expr {
    match expr {
        Expr::Sort { sort, .. } => Expr::Sort { sort, span },
        Expr::Ident { name, .. } => Expr::Ident { name, span },
        Expr::UniverseApp { name, levels, .. } => Expr::UniverseApp { name, levels, span },
        Expr::Num { value, .. } => Expr::Num { value, span },
        Expr::Hole { .. } => Expr::Hole { span },
        Expr::App { fun, arg, .. } => Expr::App { fun, arg, span },
        Expr::Lambda { binders, body, .. } => Expr::Lambda {
            binders,
            body,
            span,
        },
        Expr::Forall { binders, body, .. } => Expr::Forall {
            binders,
            body,
            span,
        },
        Expr::Arrow {
            domain, codomain, ..
        } => Expr::Arrow {
            domain,
            codomain,
            span,
        },
        Expr::Plus { lhs, rhs, .. } => Expr::Plus { lhs, rhs, span },
        Expr::Let {
            binder, val, body, ..
        } => Expr::Let {
            binder,
            val,
            body,
            span,
        },
        Expr::By { tactics, .. } => Expr::By { tactics, span },
    }
}

/// Expected type text for the field at position `i`, instantiated through the
/// result-argument mapping (`binder name → rendered goal argument`). Bare
/// Ident fields keep the plain argument text; compound field types
/// (`Eq a b`, `And a b`, `p a`, …) are deep-substituted with the goal's own
/// argument ASTs (shadow-guarded) before rendering.
fn field_type_text(template: &CtorTemplate, i: usize, ty_args: &[&Expr]) -> Option<String> {
    let ty = template.binder_tys.get(i)?.as_ref()?;
    if let Expr::Ident { name, .. } = ty {
        if let Some(j) = template
            .result_arg_names
            .iter()
            .position(|n| n.as_deref() == Some(name.as_str()))
        {
            let arg = ty_args.get(j)?;
            return Some(render_arg(arg));
        }
        return Some(render_expr(ty));
    }
    // Every binder the goal determines goes into the map (via the
    // result-argument position relation), not just the first hit.
    let mut map: HashMap<String, Expr> = HashMap::new();
    for name in &template.binder_names {
        let Some(j) = template
            .result_arg_names
            .iter()
            .position(|n| n.as_deref() == Some(name.as_str()))
        else {
            continue;
        };
        let Some(arg) = ty_args.get(j) else {
            continue;
        };
        map.insert(name.clone(), (*arg).clone());
    }
    Some(render_expr(&substitute_names(ty, &map, &HashMap::new())))
}

/// The constructor-spine case of the walk: the answer is a (partial) ctor
/// application `ctor v1 … vn` against the goal `C t1 … tm` — every `sorry`
/// argument is a sub-hole. Parameter positions expect the goal's own
/// argument (the value is determined by the goal); proof-field positions
/// expect the instantiated field type.
fn ctor_spine_case(
    ty: &Expr,
    val: &Expr,
    binders: Vec<GoalBinder>,
    templates: &GoalTemplates,
) -> Option<OpenGoalInfo> {
    let (val_head, val_args) = spine_head_args(val)?;
    let (ty_head, ty_args) = spine_head_args(ty)?;
    let template = templates.ctors.get(&ty_head)?;
    // 值的头必须是该族的构造子（如目标头 `And` ↔ 构造子 `And.intro`）。
    if template.name != val_head || val_args.len() > template.binder_names.len() {
        return None;
    }
    let mut holes = Vec::new();
    let mut sub_goals = Vec::new();
    for (i, arg) in val_args.iter().enumerate() {
        if let Expr::Hole { span } = arg {
            holes.push(*span);
            let ty_text = template_arg(template, i, &ty_args)
                .or_else(|| field_type_text(template, i, &ty_args));
            sub_goals.push(SubGoal {
                span: *span,
                ty: ty_text,
            });
        }
    }
    if holes.is_empty() {
        return None; // fully applied, no holes: not an open exercise
    }
    Some(OpenGoalInfo {
        goal: render_expr(ty),
        binders,
        holes,
        sub_goals,
        refine_template: None,
    })
}

/// 函数实参洞（v1）：值是已知函数/常量的应用 `f v1 … vn`，直接实参里的
/// `sorry` 按函数望远镜实例化出期望类型。第 i 个洞的期望类型 =
/// `binder_tys[i]` 把前 i 个 binder 名替换成 `v1..vi`（含宇宙层级替换）。
/// 前置实参本身是洞时该洞类型无法确定 → `None`（面板显示 `?`）。
/// 判定仍是完整内核的事：这里只生成建议，形状是否真的对由填洞后的内核裁决。
fn func_spine_case(
    ty: &Expr,
    val: &Expr,
    binders: Vec<GoalBinder>,
    templates: &GoalTemplates,
    locals: &HashMap<String, FuncTemplate>,
) -> Option<OpenGoalInfo> {
    let (val_head, val_args) = spine_head_args(val)?;
    // 全局优先，局部假设（值位 `funapply` 引入的覆盖层）兜底。
    let template = templates
        .funcs
        .get(&val_head)
        .or_else(|| locals.get(&val_head))?;
    let overapplied = val_args.len() > template.binder_names.len();
    let base = spine_base(val)?;
    let levels = match base {
        Expr::UniverseApp { levels, .. } => {
            if levels.len() != template.universe.len() {
                return None; // arity 错误交给 elab 报告，不掩盖
            }
            template
                .universe
                .iter()
                .cloned()
                .zip(levels.iter().cloned())
                .collect()
        }
        // 无 `.{}` 时 elab 对每个宇宙参数取 0（见 elab.rs 的 Ident 分支）。
        _ => template
            .universe
            .iter()
            .map(|u| (u.clone(), "0".to_string()))
            .collect(),
    };
    let mut holes = Vec::new();
    let mut sub_goals = Vec::new();
    if overapplied {
        // 超量应用：实参落在声明望远镜的结果之上（`(And.right a (Not a) x)
        // sorry`——And.right 全量应用的结果是 `Not a`，`sorry` 是它的函数
        // 实参）。把结果按 def 体展开（`Not a` ⇒ `a -> False`）后继续按
        // 箭头逐层匹配剩余实参；展开不了或不是箭头就交回原路径。
        let result = template.result_ty.as_ref()?;
        let mut map: HashMap<String, Expr> = HashMap::new();
        for (name, arg) in template.binder_names.iter().zip(val_args.iter()) {
            if !name.is_empty() {
                map.insert(name.clone(), (*arg).clone());
            }
        }
        let mut res = substitute_names(result, &map, &levels);
        for arg in &val_args[template.binder_names.len()..] {
            for _ in 0..8 {
                if matches!(res, Expr::Arrow { .. }) {
                    break;
                }
                match unfold_def_once(&res, templates) {
                    Some(next) => res = next,
                    None => break,
                }
            }
            match res {
                Expr::Arrow {
                    domain, codomain, ..
                } => {
                    if let Expr::Hole { span } = arg {
                        holes.push(*span);
                        let text = render_expr(&domain);
                        sub_goals.push(SubGoal {
                            span: *span,
                            ty: if text.contains("sorry") {
                                None
                            } else {
                                Some(text)
                            },
                        });
                    }
                    res = *codomain;
                }
                _ => return None,
            }
        }
    } else {
        for (i, arg) in val_args.iter().enumerate() {
            if let Expr::Hole { span } = arg {
                holes.push(*span);
                sub_goals.push(SubGoal {
                    span: *span,
                    ty: instantiate_binder_type(template, i, &val_args, &levels),
                });
            }
        }
    }
    if holes.is_empty() {
        return None;
    }
    Some(OpenGoalInfo {
        goal: render_expr(ty),
        binders,
        holes,
        sub_goals,
        refine_template: None,
    })
}

/// 一步 delta-β 展开：`res` 的头是带 def 体的函数时，把体按实参代入。
/// 教学语言的简单 def（非递归）够用；无捕获重命名——体 binder 与文档
/// 局部名同名时恰好代入的就是该实参（`Not` 的 binder `a` 对文档局部
/// `a`），碰撞场景按名字直代可接受。
fn unfold_def_once(res: &Expr, templates: &GoalTemplates) -> Option<Expr> {
    let (head, args) = spine_head_args(res)?;
    let body = templates.funcs.get(&head)?.def_body.as_ref()?;
    let mut map: HashMap<String, Expr> = HashMap::new();
    let mut cur = body;
    let mut i = 0usize;
    while let Expr::Lambda {
        binders,
        body: inner,
        ..
    } = cur
    {
        for binder in binders {
            if let Some(arg) = args.get(i) {
                map.insert(binder.name.clone(), (*arg).clone());
            }
            i += 1;
        }
        cur = inner;
    }
    if i == 0 {
        return None;
    }
    Some(substitute_names(cur, &map, &HashMap::new()))
}

/// 第 `i` 个实参的期望类型文本：binder 类型经前置实参 AST 替换后渲染；
/// 替换结果含 `sorry`（前置洞未定）或 binder 无类型标注时为 `None`。
fn instantiate_binder_type(
    template: &FuncTemplate,
    i: usize,
    args: &[&Expr],
    levels: &HashMap<String, String>,
) -> Option<String> {
    let ty = template.binder_tys.get(i)?.as_ref()?;
    let mut map: HashMap<String, Expr> = HashMap::new();
    for (name, arg) in template.binder_names.iter().zip(args.iter()).take(i) {
        if name.is_empty() {
            continue;
        }
        map.insert(name.clone(), (*arg).clone());
    }
    let text = render_expr(&substitute_names(ty, &map, levels));
    if text.contains("sorry") {
        None
    } else {
        Some(text)
    }
}

/// The refine skeleton for a single-hole answer whose goal head is a known
/// constructor: parameters that the goal determines are auto-filled, proof
/// fields become `sorry`.
fn refine_template_for(ty: &Expr, templates: &GoalTemplates) -> Option<String> {
    // 目标本身可能是 Pi 链（`… -> And a b`）：剥到结果再取 spine。
    let mut binders = Vec::new();
    let result = peel_type(ty, &mut binders);
    let (head, ty_args) = spine_head_args(&result)?;
    let template = templates.ctors.get(&head)?;
    if template.binder_tys.len() != template.binder_names.len()
        || template.result_arg_names.is_empty()
    {
        return None;
    }
    let mut args = Vec::with_capacity(template.binder_names.len());
    let mut any_hole = false;
    for i in 0..template.binder_names.len() {
        match template_arg(template, i, &ty_args) {
            Some(value) => args.push(value),
            None => {
                args.push("sorry".to_string());
                any_hole = true;
            }
        }
    }
    if !any_hole {
        return None; // the goal is fully determined; nothing to refine
    }
    Some(format!("{} {}", template.name, args.join(" ")))
}

/// Walk the declared type and the (partial) answer in parallel: every lambda
/// in the answer consumes one Pi layer of the type; when the walk reaches a
/// hole (or a constructor/function spine with holes), the remaining type is
/// the exercise's current goal and the consumed binders are its context.
fn goal_under_binders(
    ty: &Expr,
    val: &Expr,
    templates: &GoalTemplates,
    locals: &HashMap<String, FuncTemplate>,
) -> Option<OpenGoalInfo> {
    match val {
        Expr::Hole { span } => {
            let holes = vec![*span];
            Some(OpenGoalInfo {
                goal: render_expr(ty),
                binders: Vec::new(),
                holes,
                sub_goals: Vec::new(),
                refine_template: refine_template_for(ty, templates),
            })
        }
        Expr::Lambda { binders, body, .. } => {
            let (binder, binders_rest) = binders.split_first()?;
            // Consume one Pi layer; remember the hypothesis it introduces.
            let (rest_ty, layer_ty_text) = match ty {
                Expr::Forall {
                    binders: tbinders,
                    body: tbody,
                    ..
                } => {
                    let (tbinder, trest) = tbinders.split_first()?;
                    let rest = if trest.is_empty() {
                        tbody.as_ref().clone()
                    } else {
                        Expr::Forall {
                            binders: trest.to_vec(),
                            body: tbody.clone(),
                            span: Span::default(),
                        }
                    };
                    let layer_text = tbinder.ty.as_deref().map(render_expr).unwrap_or_default();
                    (rest, layer_text)
                }
                Expr::Arrow {
                    domain, codomain, ..
                } => {
                    let text = render_expr(domain);
                    (codomain.as_ref().clone(), text)
                }
                _ => return None,
            };
            // The learner's own binder wins for the name and (if written) the
            // type; an untyped binder borrows the declared layer's type.
            let binder_text = binder
                .ty
                .as_deref()
                .map(render_expr)
                .unwrap_or(layer_ty_text);
            let introduced = GoalBinder {
                name: binder.name.clone(),
                ty: binder_text,
            };
            let mut info = if binders_rest.is_empty() {
                goal_under_binders(&rest_ty, body, templates, locals)?
            } else {
                let rest_val = Expr::Lambda {
                    binders: binders_rest.to_vec(),
                    body: body.clone(),
                    span: Span::default(),
                };
                goal_under_binders(&rest_ty, &rest_val, templates, locals)?
            };
            info.binders.insert(0, introduced);
            Some(info)
        }
        // 值位 `let`：值为洞 → 子目标期望 binder 注解 `T`；值闭合 → `x : T`
        // 进入局部假设，继续 walk body（§5.3）。
        Expr::Let {
            binder,
            val: let_val,
            body,
            ..
        } => {
            if expr_has_hole(let_val) {
                // 值位洞的期望类型 = binder 注解 T；整体剩余目标仍是声明类型。
                let ty_ast = binder.ty.as_deref()?;
                let precise = goal_under_binders(ty_ast, let_val, templates, locals);
                let (holes, precise_goals) = match precise {
                    Some(info) => (info.holes, info.sub_goals),
                    None => {
                        let mut holes = Vec::new();
                        collect_hole_spans(let_val, &mut holes);
                        (holes, Vec::new())
                    }
                };
                let text = render_expr(ty_ast);
                let sub_goals = holes
                    .iter()
                    .enumerate()
                    .map(|(i, span)| SubGoal {
                        span: *span,
                        ty: precise_goals
                            .get(i)
                            .and_then(|sub| sub.ty.clone())
                            .or_else(|| Some(text.clone())),
                    })
                    .collect();
                Some(OpenGoalInfo {
                    goal: render_expr(ty),
                    binders: Vec::new(),
                    holes,
                    sub_goals,
                    refine_template: None,
                })
            } else {
                // 闭合值：`x : T` 加入局部假设，继续 walk body。
                let ty_ast = binder.ty.as_deref()?;
                let introduced = GoalBinder {
                    name: binder.name.clone(),
                    ty: render_expr(ty_ast),
                };
                let mut info = goal_under_binders(ty, body, templates, locals)?;
                info.binders.insert(0, introduced);
                Some(info)
            }
        }
        // 构造子语义优先（参数位可由目标自动判定，信息更多）；函数兜底。
        _ => ctor_spine_case(ty, val, Vec::new(), templates)
            .or_else(|| func_spine_case(ty, val, Vec::new(), templates, locals)),
    }
}

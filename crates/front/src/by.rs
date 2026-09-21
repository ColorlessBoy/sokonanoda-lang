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

use crate::ast::{Binder, BinderKind, Expr, HaveValue, MatchArm, Pattern, Tactic};
use crate::compile::elab::{DefTable, InductiveTable, MatchCtor};
use crate::compile::CompileError;
use crate::compile::{CompileOptions, ErrorKind};
use crate::judge::{
    begin_batch, flush_batch, judge_infer, judge_terms, judge_terms_strict, GoalBinderSpec,
    Judgement, OpenGoalSpec,
};
use crate::proof::{parse_expr_text, render_expr};
use crate::spine::{
    head_and_args, mentions, peel_pi_delta, spine_of, substitute, unfold_head_once, unify_spine,
};
use crate::Span;

/// delta 展开的**宇宙层级提示**：`≠`（`Ne`）这类「头上不带 `.{}`」的常量
/// 展开时用它实例化定义体里的宇宙变量（`Ne` 的体是 `Eq.{u} α a b -> False`）。
///
/// 规则与 `elab.rs::universe_level_text_of_operands` 给记法求层级**同解**
/// （不同解 = 同一命题在「elaborate」与「delta 展开」两处落进不同的常量应用，
/// 而那种错两边渲染出来一模一样）：`{u}` 参数的类型是 `Sort u`，所以 `u` =
/// **那个类型参数的值的 sort**。值有两种来源，按**实参个数**区分：
///
/// | 形态 | 例 | 首实参是 | 层级 |
/// |---|---|---|---|
/// | 点名（实参 ≥ 形参） | `Ne (Set α) A B` | 那个类型参数自己 | `sort(首实参)` |
/// | 记法（实参 < 形参） | `A ≠ B` | 该项的项（前导类型参数被 elab 补掉） | `sort(type(首实参))` |
///
/// 实测两档都必须对：`Ne (Set α) …` 一步得 `1`（两步会得 `2`，静默错层级），
/// `p ≠ q`（`p q : Prop`）两步得 `1`（一步会得 `0`）。
///
/// 只在「头不带 `.{}`、定义**恰好一个**宇宙参数」时才被用（见
/// `spine::resolve_levels`）；读不出、或算出来不是具体数字 ⇒ `None`，
/// 展开保持既有行为（悬空变量**响亮**报错，绝不静默错层级）。
fn level_hint_of(
    expr: &Expr,
    defs: &DefTable,
    binders: &[GoalBinderSpec],
    prefix_src: &str,
    options: &CompileOptions,
) -> Option<String> {
    let (name, args) = head_and_args(expr)?;
    let (params, universes) = def_shape(defs, name)?;
    if universes != 1 {
        return None;
    }
    let first = args.first()?;
    let infer = |text: &str| judge_infer(prefix_src, options, binders, text).ok();
    let sort_text = if args.len() >= params {
        infer(&render_expr(first))?
    } else {
        // 记法形态：首实参是项 ⇒ 先取它的类型，再取那个类型的 sort。
        infer(&infer(&render_expr(first))?)?
    };
    let level = crate::compile::elab::level_text_of_sort(&sort_text)?;
    is_concrete_level(&level).then_some(level)
}

/// **受信任安装的 Eq prelude 常量**的 `(项参数个数, 宇宙参数个数)`。
///
/// 为什么需要这张表：`Eq`/`Eq.refl`/`Eq.subst` 是 `install_eq_prelude` 直接装进
/// 内核的 **axiom**——axiom 没有定义体，所以它们**不进源级 delta 表 `defs`**
/// （`DefTable` 只登记可展开的 def）。可是还原裸名的宇宙层级
/// （[`restore_universe_levels`]）必须知道「它有几个宇宙参数、几个项参数」，
/// 否则 pp 掉出来的裸 `Eq (f a) b` 在判定用的合成声明里按默认 `.{0}` elaborate
/// ⇒ 报「期望 `Sort(0)`，实际是 `β`」这种与现场毫无关系的内核消息（R2 实测：
/// `cases` 会把假设的书写类型换成 pp 形态，`Eq.{1} β (f a) b` → `Eq (f a) b`，
/// 于是 `cases` 臂里每条用到那条假设的 `exact` 都判红）。
///
/// 数字与 `PRELUDE_EQ_SRC` 的签名一一对应（`{u}` 是宇宙参数；项参数按
/// `params_of_ty` 的口径数 Forall/Arrow binder）：
///   `Eq {u} : {α : Sort u} -> α -> α -> Prop`                    → (3, 1)
///   `Eq.refl {u} : {α : Sort u} -> (a : α) -> Eq.{u} α a a`      → (2, 1)
///   `Eq.subst {u} : {α} -> {p} -> {a} -> {b} -> Eq… -> p a -> p b` → (6, 1)
///
/// 只在 `defs` 里查不到该名字时才用（用户自己声明同名常量 ⇒ 以用户声明为准）。
fn trusted_prelude_arity(name: &str) -> Option<(usize, usize)> {
    match name {
        "Eq" => Some((3, 1)),
        "Eq.refl" => Some((2, 1)),
        "Eq.subst" => Some((6, 1)),
        _ => None,
    }
}

/// `(项参数个数, 宇宙参数个数)`：优先源级 delta 表，其次受信任的 Eq prelude。
fn def_shape(defs: &DefTable, name: &str) -> Option<(usize, usize)> {
    if let Some(info) = defs.get(name) {
        return Some((info.params.len(), info.universes.len()));
    }
    trusted_prelude_arity(name)
}

/// **逐层 delta 剥 Pi**（最多 `limit` 层）：`intro` / `constructor` 这类
/// 「目标头是 def」的形状常常要展开**两层以上**才露出 Pi/归纳头。
///
/// 实测（R2 子代理报的）：`X ∈ 𝒫 B` 要 `Set.mem` → `Set.powerset` → `Set.subset`
/// 才到 `forall`；`x ∈ B ∩ C` 要 `Set.mem` → `Set.inter` 才到 `And`。只展开
/// 一层时学习者看到「`intro` 需要一个函数目标」/「`constructor` 需要归纳类型」，
/// 而目标明明就是集合成员关系——教学上完全是误导。
///
/// 每层都按**当前表达式**重算层级提示（`≠` 那类宇宙参数只有展开时才需要）。
fn peel_pi_delta_deep(
    expr: &Expr,
    defs: &DefTable,
    binders: &[GoalBinderSpec],
    prefix_src: &str,
    options: &CompileOptions,
    limit: usize,
) -> Option<crate::spine::PiBody> {
    let hint_of = |e: &Expr| level_hint_of(e, defs, binders, prefix_src, options);
    crate::spine::peel_pi_delta_n(expr, defs, &hint_of, limit)
}

/// 目标头**展开到归纳**（`cases`/`constructor`/`left`/`right`/`use` 共用）：
/// `x ∈ B ∩ C` 要展开两层才露出 `And`（`Set.mem` → `Set.inter`）。
fn goal_unfolded_to_inductive(
    ty: &Expr,
    inductives: &InductiveTable<'_>,
    defs: &DefTable,
    binders: &[GoalBinderSpec],
    prefix_src: &str,
    options: &CompileOptions,
) -> Expr {
    let hint = level_hint_of(ty, defs, binders, prefix_src, options);
    crate::spine::unfold_to_inductive(
        ty,
        &|name| inductives.contains_key(name),
        defs,
        4,
        hint.as_deref(),
    )
}

/// 把 pp 文本里**裸名**的带宇宙参数常量补成显式 `.{n}`（`Ne` → `Ne.{1}`）。
///
/// 为什么需要：内核 pp 省掉隐式宇宙参数（`Ne.{1} (Set α) A B` → 裸
/// `Ne (Set α) A B`），而裸名回读按默认 `.{0}` elaborate ⇒ 那是**另一个常量
/// 应用**（`Set α` 不是 `Sort 0` 的元素）⇒ 整条合成声明 elaborate 不了，报错却是
/// 「期望 Sort(0)，实际是 Sort(1)」这种与现场无关的话（实测：`cases` on
/// `∃ (U : Set α), ∀ A, A ⊆ U ∧ A ≠ U`——根因是**根目标**被 G-05 规范化成 pp
/// 文本，`≠` 的层级当场丢掉，后面每个 tactic 都继承它）。
///
/// 规则与记法路径、delta 展开**同一条**（`level_hint_of`）：一步还是两步由
/// 「实参个数 vs 形参个数」定。解不出具体数字就**原样留着**——回读时响亮报错，
/// 绝不猜一个层级。
/// `expr` 里有没有**带宇宙参数的常量以裸名出现**（`Ne` / `Eq.symm` …）。
///
/// 这是 [`restore_universe_levels`] 的**闸门**：那个函数每遇到一个这样的常量
/// 就要问一次内核（`judge_infer`），而判定是**每条 tactic 都跑**的路径——
/// 实测不设闸时判卷从秒级涨到分钟级（一次 `grade` 要把整份文档重编译很多遍）。
/// 绝大多数类型里没有这种常量（课程里只有 `≠` 与 `Eq.*` 家族），所以这道闸
/// 把常见路径的额外成本压到**一次结构遍历**（零内核查询）。
fn mentions_bare_universe_const(expr: &Expr, defs: &DefTable) -> bool {
    match expr {
        Expr::Ident { name, .. } => def_shape(defs, name).is_some_and(|(_, u)| u > 0),
        Expr::App { fun, arg, .. } => {
            mentions_bare_universe_const(fun, defs) || mentions_bare_universe_const(arg, defs)
        }
        Expr::Plus { lhs, rhs, .. } => {
            mentions_bare_universe_const(lhs, defs) || mentions_bare_universe_const(rhs, defs)
        }
        Expr::Lambda { binders, body, .. } | Expr::Forall { binders, body, .. } => {
            binders.iter().any(|b| {
                b.ty.as_deref()
                    .is_some_and(|t| mentions_bare_universe_const(t, defs))
            }) || mentions_bare_universe_const(body, defs)
        }
        Expr::Arrow {
            domain, codomain, ..
        } => {
            mentions_bare_universe_const(domain, defs)
                || mentions_bare_universe_const(codomain, defs)
        }
        Expr::SetLiteral { elements, .. } | Expr::AnonCtor { elements, .. } => elements
            .iter()
            .any(|e| mentions_bare_universe_const(e, defs)),
        Expr::Notation { lhs, rhs, .. } => {
            lhs.as_deref()
                .is_some_and(|e| mentions_bare_universe_const(e, defs))
                || rhs
                    .as_deref()
                    .is_some_and(|e| mentions_bare_universe_const(e, defs))
        }
        Expr::Let {
            binder, val, body, ..
        } => {
            binder
                .ty
                .as_deref()
                .is_some_and(|t| mentions_bare_universe_const(t, defs))
                || mentions_bare_universe_const(val, defs)
                || mentions_bare_universe_const(body, defs)
        }
        _ => false,
    }
}

/// 进 `fun`/`forall` 的体之前，把这一层的 binder **加进判定上下文**。
///
/// 为什么必需（R2 实测）：还原裸名宇宙层级要问内核「首实参的类型」
/// （[`level_hint_of`] 的「记法形态」），而首实参常常提到**这一层刚绑定的名字**
/// ——`∀ b, … Eq (g b) c` 里的 `b`。不把它加进 `binders`，`judge_infer` 报
/// `unknown identifier b` ⇒ 那一处的 `Eq` 保持裸名 ⇒ 组装出来的 `match` 在
/// 声明判定时按默认 `.{0}` elaborate，报「期望 `Sort(0)`，实际是 β」。
fn extend_binders(binders: &[GoalBinderSpec], bs: &[Binder]) -> Vec<GoalBinderSpec> {
    let mut out = binders.to_vec();
    out.extend(bs.iter().map(|b| GoalBinderSpec {
        name: b.name.clone(),
        ty: b.ty.as_deref().map(render_expr),
    }));
    out
}

fn restore_universe_levels(
    expr: &Expr,
    defs: &DefTable,
    binders: &[GoalBinderSpec],
    prefix_src: &str,
    options: &CompileOptions,
) -> Expr {
    // 闸门：没有裸的带宇宙参数常量就直接返回（**零内核查询**）。
    if !mentions_bare_universe_const(expr, defs) {
        return expr.clone();
    }
    match expr {
        Expr::App { .. } => {
            let (head, args) = crate::spine::spine_of(expr);
            if let Expr::Ident { name, span } = head {
                if def_shape(defs, name).is_some_and(|(_, u)| u == 1) {
                    if let Some(level) = level_hint_of(expr, defs, binders, prefix_src, options) {
                        let mut out = Expr::UniverseApp {
                            name: name.clone(),
                            levels: vec![level],
                            span: *span,
                        };
                        // **补回 pp 丢掉的前导类型实参**：`Expr::UniverseApp` 渲染时
                        // 带 `@`（全部实参显式），于是第一个实参会被当成**第一个形参**
                        // ——`Eq` 的形参是 `{α} a b`，pp 形态 `Eq (f a) b` 只有 2 个
                        // 实参，补上 `.{1}` 就成了 `@Eq.{1} (f a) b`，读作
                        // `α := (f a)` ⇒ 内核报「期望 `Sort(0)`，实际是 `β`」。
                        // 形参个数 > 实参个数时缺的一定是**前导**隐式实参，它们的值
                        // 就是首实参的类型（与 `level_hint_of` 的「记法形态」同解）。
                        if def_shape(defs, name).is_some_and(|(params, _)| args.len() < params) {
                            if let Some(first) = args.first() {
                                let ty_text =
                                    judge_infer(prefix_src, options, binders, &render_expr(first))
                                        .ok();
                                if let Some(ty) = ty_text.and_then(|t| parse_expr_text(&t).ok()) {
                                    out = Expr::App {
                                        fun: Box::new(out),
                                        arg: Box::new(ty),
                                        explicit_spine: false,
                                        span: *span,
                                    };
                                }
                            }
                        }
                        for arg in &args {
                            out = Expr::App {
                                fun: Box::new(out),
                                arg: Box::new(restore_universe_levels(
                                    arg, defs, binders, prefix_src, options,
                                )),
                                explicit_spine: false,
                                span: *span,
                            };
                        }
                        return out;
                    }
                }
            }
            let fun = restore_universe_levels(head, defs, binders, prefix_src, options);
            let mut out = fun;
            for arg in &args {
                out = Expr::App {
                    fun: Box::new(out),
                    arg: Box::new(restore_universe_levels(
                        arg, defs, binders, prefix_src, options,
                    )),
                    explicit_spine: false,
                    span: expr.span(),
                };
            }
            out
        }
        Expr::Lambda {
            binders: bs,
            body,
            span,
        } => Expr::Lambda {
            binders: bs
                .iter()
                .map(|b| Binder {
                    ty: b.ty.as_deref().map(|t| {
                        Box::new(restore_universe_levels(
                            t, defs, binders, prefix_src, options,
                        ))
                    }),
                    ..b.clone()
                })
                .collect(),
            body: Box::new(restore_universe_levels(
                body,
                defs,
                &extend_binders(binders, bs),
                prefix_src,
                options,
            )),
            span: *span,
        },
        Expr::Forall {
            binders: bs,
            body,
            span,
        } => Expr::Forall {
            binders: bs
                .iter()
                .map(|b| Binder {
                    ty: b.ty.as_deref().map(|t| {
                        Box::new(restore_universe_levels(
                            t, defs, binders, prefix_src, options,
                        ))
                    }),
                    ..b.clone()
                })
                .collect(),
            body: Box::new(restore_universe_levels(
                body,
                defs,
                &extend_binders(binders, bs),
                prefix_src,
                options,
            )),
            span: *span,
        },
        Expr::Arrow {
            domain,
            codomain,
            span,
        } => Expr::Arrow {
            domain: Box::new(restore_universe_levels(
                domain, defs, binders, prefix_src, options,
            )),
            codomain: Box::new(restore_universe_levels(
                codomain, defs, binders, prefix_src, options,
            )),
            span: *span,
        },
        Expr::Notation {
            symbol,
            target,
            assoc,
            lhs,
            rhs,
            alternatives,
            span,
        } => Expr::Notation {
            symbol: symbol.clone(),
            target: target.clone(),
            assoc: *assoc,
            lhs: lhs.as_deref().map(|e| {
                Box::new(restore_universe_levels(
                    e, defs, binders, prefix_src, options,
                ))
            }),
            rhs: rhs.as_deref().map(|e| {
                Box::new(restore_universe_levels(
                    e, defs, binders, prefix_src, options,
                ))
            }),
            alternatives: alternatives.clone(),
            span: *span,
        },
        other => other.clone(),
    }
}

/// 一个未闭合目标：内核渲染的类型文本 + 沿父链收集的已引入假设。
#[derive(Debug, Clone, PartialEq)]
pub struct ByGoal {
    pub ty: String,
    pub binders: Vec<Binder>,
}

/// 编译期记录的每个 tactic 步执行后的 goal 状态（Phase 2 的 goal 面板用）。
#[derive(Debug, Clone, PartialEq)]
pub struct ByStep {
    /// 该 tactic 的源码 span。
    pub span: Span,
    /// 该步执行后的**全部**未闭合目标，当前目标在首位；空 = 所有目标已闭合。
    pub goals: Vec<ByGoal>,
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
    Apply {
        f: Expr,
        args: Vec<ApplyArg>,
    },
    /// `cases`（设计 `docs/design/course-lean-style.md` L3.1）：降低成一个
    /// `match`——臂体是各自组装出来的项，递归子/iota 由**既有的 `match`
    /// 降低路径**处理（`compile/elab.rs`），引擎不手搓 recursor。
    Cases {
        scrutinee: Expr,
        arms: Vec<CasesArmNode>,
    },
    /// `have h : T := t`（L3.6）：降低成 **let 的应用形态**
    /// `(fun (h : T) => <body>) t`。
    ///
    /// 为什么用应用而不是 `Expr::Let`：引擎的上下文模型是「沿父链的
    /// `intros`」，`Have` 只要在父链上贡献一个 binder（`h : T`）就进了**所有**
    /// 判定（`judge_infer` 的 binder 规格）与组装；包成 lambda + 应用之后，
    /// `t : T` 由**内核**在应用处再判一次。`Expr::Let` 还要动 `judge.rs` 的
    /// 回读路径（设计 L3.6 原估 150–250 行），这条路 ~40 行就够。
    Have {
        /// 引入的假设名。
        name: String,
        /// 它的类型（**必填**：本语言不做隐式实参推断，省了就判不了 `t : T`）。
        ty: Expr,
        /// 值：项，或**已组装好的**嵌套 `by` 块结果（`assemble` 出来的项）。
        value: Expr,
        /// 引入 `h` 之后继续解的目标节点。
        body: usize,
    },
}

/// `cases` 的一个分支在目标树里的落点。
#[derive(Clone)]
struct CasesArmNode {
    pattern: Pattern,
    /// 子目标节点 id。
    node: usize,
    /// `node.intros` 的**前多少个**是模式绑定的分支假设——它们**不包 lambda**
    /// （由 `match` 的模式绑定），其余（臂体里自己 `intro` 的）才包。
    binders: usize,
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

/// 根目标的**规范名化**（G-05）：把源 AST 渲染成文本交给内核 pp 再读回来。
///
/// 只在文件用了 `namespace`/`open` 时才走（调用方决定）：源里的短名 `mem`
/// 在内核 pp 里是 `A.mem`，而 `apply` 的 `unify_spine` 按文本对齐——两边同源
/// 才不会假报不匹配。任何一步失败（解析不了/推断不了）都退回原 AST：行为与
/// 没有这一步时逐字相同，绝不因为"规范化失败"把好文件判红。
fn canonical_goal_type(
    ty: &Expr,
    initial_binders: &[Binder],
    prefix_src: &str,
    options: &CompileOptions,
    defs: &DefTable,
) -> Expr {
    let specs: Vec<GoalBinderSpec> = initial_binders
        .iter()
        .map(|b| GoalBinderSpec {
            name: b.name.clone(),
            ty: b.ty.as_deref().map(render_expr),
        })
        .collect();
    let Some(text) = crate::judge::judge_render_type(prefix_src, options, &specs, &render_expr(ty))
    else {
        return ty.clone();
    };
    let Ok(canonical) = parse_expr_text(&text) else {
        return ty.clone();
    };
    let specs_for_levels: Vec<GoalBinderSpec> = specs.clone();
    let canonical =
        restore_universe_levels(&canonical, defs, &specs_for_levels, prefix_src, options);
    // 与 [`canonical_goal_with_spec`] **同一条护栏**：pp 文本是可回读的才准用。
    // 这条不是理论上的洁癖——**课程文件的闭包前缀里有 `namespace Set`**，
    // 于是 G-05 的「用了 namespace ⇒ 根目标规范化」对整门课**全程为真**，
    // 根目标（以及由它派生的每个子目标）都带着 pp 文本；而 pp 会丢掉 `Eq` 的
    // 显式类型实参（`Eq.{1} (Set α) A ∅` → `Eq A ∅`），那段文本重解析成
    // `Eq.{0} A ∅`（`A` 落进类型位）⇒ 之后每条 `exact` 都假失败，报错里两边
    // **看起来一模一样**（实测：单元② 的两条证明卡在这里）。
    if !is_rereadable(&canonical, defs) {
        return ty.clone();
    }
    keep_if_lossless(ty, canonical)
}

/// 归一化**有损**时退回原 AST。
///
/// 内核 pp 会丢掉隐式实参与宇宙层级（`Eq.{1} (Set α) A B` → `Eq A B`），
/// 于是「把目标过一遍内核 pp」可能把它换成**少几个实参**的坏 AST——拿它当
/// 目标会让后面的 tactic 全错（实测：命名空间文件里 `by exact` 报
/// 「期望 `Sort(0)`，实际是 `Foo.[] $2`」）。
/// 判据：常量头的**实参个数变少**就是有损（丢的只会是前导隐式实参）。
fn keep_if_lossless(original: &Expr, canonical: Expr) -> Expr {
    let (_, oargs) = spine_of(original);
    let (_, cargs) = spine_of(&canonical);
    if cargs.len() < oargs.len() {
        return original.clone();
    }
    canonical
}

/// 同 [`canonical_goal_type`]，但直接吃**已经算好的 `OpenGoalSpec`**
/// （`apply` 的重试路径手上正好有一份，见 [`apply_tactic`]）。
/// 失败一律退回原 AST：绝不因为「规范化失败」把好文件判红。
fn canonical_goal_with_spec(
    ty: &Expr,
    spec: &OpenGoalSpec,
    prefix_src: &str,
    options: &CompileOptions,
    defs: &DefTable,
) -> Expr {
    let Some(text) =
        crate::judge::judge_render_type(prefix_src, options, &spec.binders, &render_expr(ty))
    else {
        return ty.clone();
    };
    match parse_expr_text(&text) {
        // **可回读**才准用：pp 是有损的（`Eq.{1} (Set α) A B` → `Eq A B`，丢掉
        // 类型实参），而这份文本马上会被重新解析、并作为**子目标的类型**传下去
        // ——`Eq A B` 重解析成 `Eq.{0} A B`（`A` 落进类型位），内核报
        // `expected Sort(0), actual Set α`，而报错信息里两边**看起来一样**，
        // 极难查（实测：`intro h` 引入 `h : Eq A (Set.empty α)`，后面每条
        // `exact` 都假失败）。
        //
        // `is_rereadable` 放行之后还要**补层级**：裸 `Ne`/`Eq` 这类带宇宙参数的
        // 常量 pp 会省掉 `.{u}`，不补就还是错的常量应用（见
        // [`restore_universe_levels`]）。
        //
        // **顺序与 [`canonical_goal_type`] 一致：先补、再判可回读。** 反过来的话
        // pp 形态 `Eq x x`（丢了类型实参、只剩 2 个实参）会先被 `is_rereadable`
        // 拒掉，而 `restore_universe_levels` 正是负责把那个类型实参补回来的
        // ——`rfl` 在 `x = x` / `A = A` 这类记法目标上因此永远拿不到规范形态。
        Ok(canonical) => {
            let canonical =
                restore_universe_levels(&canonical, defs, &spec.binders, prefix_src, options);
            if is_rereadable(&canonical, defs) {
                keep_if_lossless(ty, canonical)
            } else {
                ty.clone()
            }
        }
        _ => ty.clone(),
    }
}

/// 这份（由内核 pp 渲染再解析回来的）文本**能不能原样读回去**。
///
/// 今天只有一条判据，但它正好是唯一会咬人的那条：**`Eq` 应用必须带齐三个
/// 实参**（类型 + 两边）。pp 会按 `BinderStyle` 省掉隐式实参，而 `Eq` 的类型
/// 参数是**显式**的——一旦丢掉，`Eq a b` 会被读成「`a` 是类型」，语义全变。
fn is_rereadable(expr: &Expr, defs: &DefTable) -> bool {
    // ③ **裸名的宇宙层级**：pp 会省掉隐式宇宙参数（`Ne.{1} (Set α) A B` → 裸
    // `Ne (Set α) A B`），裸名回读按默认 `.{0}` elaborate ⇒ 另一个常量应用。
    // **这条不在这里拒**——pp 形态还有「参数写全」的优点（源 AST 的记法操作数
    // 落进被应用的位置，G-19 就是这么来的），拒了它反而更糟。层级由
    // [`restore_universe_levels`] 在**放行之后**补回来。
    let _ = defs;
    // ② **宇宙层级必须是具体数字**：pp 会打出**宇宙变量名**（`Ne.{u}` / `Eq.{u}`），
    // 而那段文本读回来时作用域里根本没有 `u` ⇒ `level_ptr` 报
    // `unknown universe level u`（实测：`{a} ≠ ∅` 的证明就死在这儿）。
    // 具体数字（`Eq.{1}`）没有这个问题。
    if let Expr::UniverseApp { levels, .. } = expr {
        if levels.iter().any(|level| !is_concrete_level(level)) {
            return false;
        }
    }
    let (head, args) = spine_of(expr);
    // 只对**裸名** `Eq` 查元数：pp 丢掉类型实参后 `Eq a b` 会被读成「`a` 是类型」。
    // `Eq.{1} α x`（带层级的 `UniverseApp`）渲染带 `@`，是**无歧义的部分应用**，
    // 整条脊的实参个数由外层脊负责（见下面的 `Expr::App` 分支）。
    if let Expr::Ident { name, .. } = head {
        if name == "Eq" && args.len() < 3 {
            return false;
        }
    }
    match expr {
        // **只在外层脊上查一次头**：`Eq.{1} α x x` 的二元 App 链里，内层节点
        // `Eq.{1} α x`（2 个实参）若也走一遍上面的 Eq 元数检查就会被误判
        // 「丢了类型实参」——而它只是**部分应用**，整条脊其实是齐的。
        Expr::App { .. } => {
            let (head, args) = spine_of(expr);
            is_rereadable(head, defs) && args.iter().all(|a| is_rereadable(a, defs))
        }
        Expr::Lambda { body, .. } | Expr::Forall { body, .. } => is_rereadable(body, defs),
        Expr::Arrow {
            domain, codomain, ..
        } => is_rereadable(domain, defs) && is_rereadable(codomain, defs),
        Expr::Notation { lhs, rhs, .. } => {
            lhs.as_deref().is_none_or(|e| is_rereadable(e, defs))
                && rhs.as_deref().is_none_or(|e| is_rereadable(e, defs))
        }
        Expr::UniverseApp { .. } => true, // 本节点已在函数开头判过
        _ => true,
    }
}

/// 层级文本是不是**具体数字**（`1`、`2`…）。`u` / `u+1` 这类含变量名的都算否
/// ——见 [`is_rereadable`]。
fn is_concrete_level(level: &str) -> bool {
    !level.is_empty() && level.chars().all(|c| c.is_ascii_digit())
}

/// 把 `ty`（声明类型）与 `by` 块降级成 lambda AST。
/// `initial_binders` 是声明级 binder（`theorem f (a : A) : B := by …` 里的
/// `a`）：它们是引擎的初始上下文，类型先剥掉对应层数，`by` 从 `B` 出发；
/// 没有声明 binder 时传空切片（旧行为）。
///
/// `canonical_goal`（G-05）：文件用了 `namespace`/`open` 时为 `true`，根目标
/// 先经内核 pp 规范名化（见 [`canonical_goal_type`]）；否则零开销。
/// **记法 / 集合字面量不在这里处理**：它们只在 `apply` 的失败重试路径上按需
/// 归一化（见 `apply_tactic`），根目标保持源 AST。
#[allow(clippy::too_many_arguments)]
pub(crate) fn run_by(
    ty: &Expr,
    by: &Expr,
    initial_binders: &[Binder],
    universe: &[String],
    prefix_src: &str,
    options: &CompileOptions,
    canonical_goal: bool,
    inductives: &InductiveTable<'_>,
    defs: &DefTable,
) -> Result<ByOutcome, CompileError> {
    // **乐观一趟**（0.62.0 性能）：`by` 块里的判定不逐步做，而是先记下来
    // （`judge_terms` 在批次里返回乐观的 `Match`），跑完由 `flush_batch` 把
    // **同一个前缀**的全部判定合成**一份文档**一次判完。
    //
    // 为什么这与旧行为等价（而不是"放宽判定"）：
    //   * 判定结果只在两处影响控制流——`Match` 才继续、否则报错；`assumption`
    //     是例外，它按结论**挑**哪条假设命中，所以它单独走严格通道；
    //   * 于是"这一趟里每一条判定真的都是 Match"时，乐观趟与逐条趟的**控制流
    //     逐字相同**，产物也就逐字相同；
    //   * 只要有一条不是 Match（或乐观趟自己报了别的错而判定并未全绿），就丢掉
    //     这一趟、改用逐条判定的**严格重跑**——诊断/位置/文案与改动前一致。
    // 代价：判定全绿的解答只走一遍前缀（原来每步一遍）；判定真的失败时多跑一趟，
    // 而失败通常发生在块的前几步，严格重跑也随之很短。
    let scope = begin_batch();
    let optimistic = run_by_inner(
        ty,
        by,
        initial_binders,
        universe,
        prefix_src,
        options,
        canonical_goal,
        inductives,
        defs,
    );
    let all_match = flush_batch(scope);
    match optimistic {
        // 判定全绿 ⇒ 这一趟就是严格趟（控制流相同），直接采信。
        Ok(outcome) if all_match => Ok(outcome),
        // 判定全绿但别处出错 ⇒ 这个错是真的，不必重跑。
        Err(error) if all_match => Err(error),
        // 有判定没通过 ⇒ 严格重跑，拿与改动前逐字相同的诊断。
        _ => run_by_inner(
            ty,
            by,
            initial_binders,
            universe,
            prefix_src,
            options,
            canonical_goal,
            inductives,
            defs,
        ),
    }
}

/// 跑一趟 `by` 块（判定走当前通道：乐观批次里 = 记录 + `Match`；否则 = 逐条判）。
#[allow(clippy::too_many_arguments)]
fn run_by_inner(
    ty: &Expr,
    by: &Expr,
    initial_binders: &[Binder],
    // 本声明的宇宙参数名（`theorem t {u} : …` 里的 `u`）。判定合成的声明必须
    // 带上它们，否则目标里的 `Sort u` / `Eq.{u}` 进内核就是「未声明宇宙变量」。
    // `example` 没有宇宙 binder，传空切片。
    universe: &[String],
    prefix_src: &str,
    options: &CompileOptions,
    canonical_goal: bool,
    inductives: &InductiveTable<'_>,
    defs: &DefTable,
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
    // G-05（设计 `docs/design/namespace-open.md` §4.6）：文件用了 namespace/open
    // 时，根目标先过一遍内核 pp —— `apply` 的 `unify_spine` 是**文本**对齐
    // （codomain 来自内核 pp、目标来自源 AST），源里的短名 `mem` 与内核的
    // `A.mem` 文本不同，会让 `apply` 假报不匹配。`canonical_goal == false`
    // （没碰 namespace 的文件）时这一步完全跳过：零额外开销、行为逐字不变。
    //
    // **记法 / 集合字面量不走这里**：它们只在 `apply` 的失败重试路径上按需
    // 归一化（见 [`apply_tactic`]）——根目标保持源 AST，`rfl`/`match` 这些
    // **要读目标结构**的 tactic 才不会被内核 pp 的「丢隐式实参」打坏
    // （实测：把根目标归一化会让 `rfl` 在 `Eq.{1} (Set α) (Aᶜ) …` 上报
    // 「需要一个 `Eq α x y` 形状的目标」）。
    let root_ty = if canonical_goal {
        canonical_goal_type(&root_ty, initial_binders, prefix_src, options, defs)
    } else {
        root_ty
    };
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

    run_tactics(
        tactics,
        &mut nodes,
        &mut worklist,
        &mut steps,
        universe,
        prefix_src,
        options,
        inductives,
        defs,
    )?;
    let expr = assemble(&nodes, 0, hole_span(tactics, *by_span));
    Ok(ByOutcome { expr, steps })
}

/// 跑一串 tactic（`by` 块的主循环，`cases` 的臂体递归复用它）。
#[allow(clippy::too_many_arguments)]
fn run_tactics(
    tactics: &[Tactic],
    nodes: &mut Vec<GoalNode>,
    worklist: &mut Vec<usize>,
    steps: &mut Vec<ByStep>,
    universe: &[String],
    prefix_src: &str,
    options: &CompileOptions,
    inductives: &InductiveTable<'_>,
    defs: &DefTable,
) -> Result<(), CompileError> {
    for tactic in tactics {
        // 当前要解的目标 = worklist 末尾。
        let cur = *worklist
            .last()
            .ok_or_else(|| tactic_error("by 块里没有待解目标", tactic.span()))?;
        match tactic {
            Tactic::Intro { names, span } => {
                // `intro a b c`：逐个名字剥一层（语义 = 连续写多个 `intro`）。
                for name in names {
                    // 先按**源 AST** 剥（零额外开销，绝大多数目标都在这儿解决）。
                    let body = match crate::spine::peel_pi(&nodes[cur].ty) {
                        Some(pi) => pi,
                        // 剥不动 ⇒ 头是 **def**（`A ⊆ B` 的 `Set.subset`、`¬ A` 的
                        // `Not`）。这时**先换用内核 pp 的规范形态**再剥：
                        //
                        // 源级 delta 展开（`unfold_head_once`）把操作数**原样**代进
                        // 定义体，于是记法操作数会落进「被应用」的位置——`∅ ⊆ A`
                        // 展开成 `forall (x : α), ∅ x -> A x`，而 `∅` 是**零元**
                        // 记法（`Set.empty` 需要前导 `α`），回读时补不出参数
                        // （台账 **G-19** / 设计 X14：课程 227 处 `Set.empty` 几乎
                        // 全在此形状）。内核 pp 出来的形态是**点名 + 参数写全**的
                        // `Set.subset α (Set.empty α) A`，展开后 `(Set.empty α) x`
                        // 回读无碍。
                        //
                        // `canonical_goal_with_spec` 失败一律退回原 AST，并且
                        // `keep_if_lossless` 挡住「pp 丢隐式实参」那一档 ⇒ 不会
                        // 因为归一化把好文件判红。归一化后仍剥不动才报错。
                        None => {
                            let spec = spec_of(nodes, cur, universe);
                            let canonical = canonical_goal_with_spec(
                                &nodes[cur].ty,
                                &spec,
                                prefix_src,
                                options,
                                defs,
                            );
                            // 层级提示按**各自要展开的那个表达式**算：源 AST 是
                            // 记法节点（`{a} ≠ ∅`）、规范形态是点名（`Ne …`），
                            // 两条路的操作数不同但必须解出同一个 `u`。
                            let canonical_hint =
                                level_hint_of(&canonical, defs, &spec.binders, prefix_src, options);
                            let source_hint = level_hint_of(
                                &nodes[cur].ty,
                                defs,
                                &spec.binders,
                                prefix_src,
                                options,
                            );
                            peel_pi_delta_deep(
                                &canonical,
                                defs,
                                &spec.binders,
                                prefix_src,
                                options,
                                4,
                            )
                            .or_else(|| {
                                peel_pi_delta_deep(
                                    &nodes[cur].ty,
                                    defs,
                                    &spec.binders,
                                    prefix_src,
                                    options,
                                    4,
                                )
                            })
                            .or_else(|| {
                                peel_pi_delta(&canonical, defs, canonical_hint.as_deref())
                            })
                            .or_else(|| {
                                peel_pi_delta(&nodes[cur].ty, defs, source_hint.as_deref())
                            })
                                .ok_or_else(|| {
                                    tactic_error(
                                        "`intro` 需要一个函数目标（… -> … 或 forall …），当前目标不是函数",
                                        *span,
                                    )
                                })?
                        }
                    };
                    // **改名**：余下的体引用的是**源/定义里的 binder 名**
                    // （`Set.subset` 的体是 `forall (x : α), A x -> B x`），而
                    // 学习者写的名字可能不同（`intro y`）。不改名的话目标里留着
                    // 悬空的 `x`，后面每条 tactic 都报 `unknown identifier x`
                    // ——报错点离根因很远（R2 实测，子代理报的 H5）。
                    // `peel_pi` 对 Arrow 给空名 ⇒ 不需要改。
                    let rest = if body.name.is_empty() || body.name == *name {
                        body.body
                    } else {
                        crate::spine::rename_free(&body.body, &body.name, name)
                    };
                    let binder = Binder {
                        name: name.clone(),
                        ty: Some(Box::new(body.domain)),
                        style: BinderKind::Explicit,
                        span: *span,
                    };
                    nodes[cur].intros.push(binder);
                    nodes[cur].ty = rest;
                }
            }
            Tactic::Exact { expr, span } => {
                exact_tactic(
                    expr, *span, nodes, worklist, universe, prefix_src, options, defs,
                )?;
            }
            // `have h : T := t`（L3.6）：**目标不变**，上下文里多一条 `h : T`。
            // 降低成 `(fun (h : T) => <rest>) t`（见 `NodeKind::Have`）。
            Tactic::Have {
                name,
                ty,
                value,
                span,
            } => {
                // ① 先算**值**。两种写法：项，或嵌套 `by` 块。
                //
                // ⚠️ **顺序要紧**：值必须在 `cur.kind` 变成 `Have` **之前**算完
                // ——否则 `context_binders` 会把 `h` 也算进它自己的上下文
                // （自己证明自己）。嵌套块的目标节点挂在 `cur` 下面，所以它
                // 天然看得见外层上下文，而看不见还没引入的 `h`。
                let value_expr = match value {
                    HaveValue::Term(expr) => expr.clone(),
                    HaveValue::By(tactics) => {
                        let id = nodes.len();
                        nodes.push(GoalNode {
                            ty: ty.clone(),
                            intros: Vec::new(),
                            parent: Some(cur),
                            kind: NodeKind::Hole,
                        });
                        let base = worklist.len();
                        worklist.push(id);
                        run_tactics(
                            tactics, nodes, worklist, steps, universe, prefix_src, options,
                            inductives, defs,
                        )?;
                        worklist.truncate(base);
                        assemble(nodes, id, *span)
                    }
                };
                // ② **立刻判一次** `value : T`。判定用一份「目标换成 T」的规格
                // （`spec_of` 给的是当前目标，不是 `have` 的类型）。
                // 为什么现在就判：错误要报在 `have` 这一行，而不是等整个证明
                // 组装完由内核在应用处报一个位置很远的错。
                let mut spec = spec_of(nodes, cur, universe);
                spec.ty = render_expr(ty);
                let term = render_expr(&value_expr);
                match judge_terms(prefix_src, options, &spec, &[term.as_str()])
                    .into_iter()
                    .next()
                {
                    Some(Judgement::Match) => {}
                    Some(Judgement::Mismatch { .. }) => {
                        return Err(CompileError::elab(
                            ErrorKind::ElabTacticFailed,
                            mismatch_message(
                                &format!("`have {name}` 的值类型不匹配"),
                                &render_expr(ty),
                                &term,
                                &spec,
                                prefix_src,
                                options,
                            ),
                            *span,
                        ));
                    }
                    Some(Judgement::Error { message, .. }) => {
                        return Err(CompileError::elab(
                            ErrorKind::ElabTacticFailed,
                            format!("`have {name}` 判定失败：{message}"),
                            *span,
                        ));
                    }
                    None => unreachable!("judge_terms returns one judgement per term"),
                }
                // ③ 把 `cur` 变成 `Have`，新目标节点接手继续解。
                let body = nodes.len();
                nodes.push(GoalNode {
                    ty: nodes[cur].ty.clone(),
                    intros: Vec::new(),
                    parent: Some(cur),
                    kind: NodeKind::Hole,
                });
                nodes[cur].kind = NodeKind::Have {
                    name: name.clone(),
                    ty: ty.clone(),
                    value: value_expr,
                    body,
                };
                worklist.pop();
                worklist.push(body);
            }
            // 以下五个（L3.2/L3.3/L3.4/L3.7）都建立在既有机械上：
            // `constructor`/`left`/`right`/`use` 是 `apply <构造子>` 的糖，
            // `exfalso` 是「换目标 + 组装时套 `False.elim`」。
            Tactic::Constructor { span } => {
                ctor_tactic(
                    0,
                    "constructor",
                    *span,
                    nodes,
                    worklist,
                    universe,
                    prefix_src,
                    options,
                    inductives,
                    defs,
                )?;
            }
            Tactic::Left { span } => {
                ctor_tactic(
                    0, "left", *span, nodes, worklist, universe, prefix_src, options, inductives,
                    defs,
                )?;
            }
            Tactic::Right { span } => {
                ctor_tactic(
                    1, "right", *span, nodes, worklist, universe, prefix_src, options, inductives,
                    defs,
                )?;
            }
            Tactic::Use { expr, span } => {
                // `use w` = `apply <唯一构造子>` + 立刻把**第一个子目标**
                // （证人位）用 `exact w` 交出去。
                ctor_tactic(
                    0, "use", *span, nodes, worklist, universe, prefix_src, options, inductives,
                    defs,
                )?;
                exact_tactic(
                    expr, *span, nodes, worklist, universe, prefix_src, options, defs,
                )?;
            }
            Tactic::Exfalso { span } => {
                // 目标换成 `False`，另开一个节点；当前节点改成
                // `False.elim <原目标> <False 的证明>`，组装时自然套上。
                let goal_ty = nodes[cur].ty.clone();
                let id = nodes.len();
                nodes.push(GoalNode {
                    ty: Expr::Ident {
                        name: "False".to_string(),
                        span: *span,
                    },
                    intros: Vec::new(),
                    parent: Some(cur),
                    kind: NodeKind::Hole,
                });
                nodes[cur].kind = NodeKind::Apply {
                    f: Expr::Ident {
                        name: "False.elim".to_string(),
                        span: *span,
                    },
                    args: vec![ApplyArg::TypeParam(goal_ty), ApplyArg::SubGoal(id)],
                };
                worklist.push(id);
            }
            Tactic::Assumption { span } => {
                let names: Vec<String> = context_binders(nodes, cur)
                    .iter()
                    .rev()
                    .map(|b| b.name.clone())
                    .collect();
                let refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
                // **严格通道**：`assumption` 要按结论**挑**哪条假设命中（不是
                // "通过/报错"二选一），乐观批次给不出这个信息。
                let js = judge_strict(prefix_src, options, nodes, worklist, cur, universe, &refs);
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
                // 目标头是**记法**（`a = b` 是 `Notation{target:"Eq"}`，不是 `Eq`
                // 应用节点）时，源 AST 认不出 `Eq α x y` ⇒ 退回内核 pp 的规范形态
                // （`canonical_goal_with_spec` 会把 `Eq` 丢掉的类型/宇宙实参
                // `restore_universe_levels` 补回来）。判据仍然是内核判定，
                // 归一化失败也一律退回原 AST（绝不因为 rfl 把好文件判红）。
                let candidate_and_closed = rfl_candidate(&nodes[cur].ty).or_else(|| {
                    let spec = spec_of(nodes, cur, universe);
                    let canonical =
                        canonical_goal_with_spec(&nodes[cur].ty, &spec, prefix_src, options, defs);
                    rfl_candidate(&canonical)
                });
                let Some((candidate, closed)) = candidate_and_closed else {
                    return Err(CompileError::elab(
                        ErrorKind::ElabTacticFailed,
                        "`rfl` 需要一个 `Eq α x y` 形状的目标",
                        *span,
                    ));
                };
                let j = judge(
                    prefix_src,
                    options,
                    nodes,
                    worklist,
                    cur,
                    universe,
                    &[candidate.as_str()],
                );
                match j.into_iter().next() {
                    Some(Judgement::Match) => {
                        nodes[cur].kind = NodeKind::Closed(closed);
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
                apply_tactic(
                    expr, *span, nodes, worklist, universe, prefix_src, options, defs,
                )?;
            }
            Tactic::Cases { expr, arms, span } => {
                cases_tactic(
                    expr, arms, *span, nodes, worklist, steps, universe, prefix_src, options,
                    inductives, defs,
                )?;
            }
            // `sorry` = 占位：当前目标保持开放（no-op，节点仍是 Hole）。
            Tactic::Sorry { .. } => {}
        }
        // 记录当前（最新）被解目标的状态（Phase 2 goal 面板）：所有未闭合
        // 目标都记下来（当前目标在首位），客户端才能一次看到完整目标列表。
        let goals = worklist
            .iter()
            .rev()
            .map(|&id| ByGoal {
                ty: render_expr(&nodes[id].ty),
                binders: context_binders(nodes, id),
            })
            .collect();
        steps.push(ByStep {
            span: tactic.span(),
            goals,
        });
    }
    Ok(())
}

/// 未闭合目标的洞位：以最后一个 tactic 的 span 为准（`by … sorry` 里就是
/// `sorry` 的位置；`by intro a` 这种没写 sorry 的部分作答则落在块尾）。
fn hole_span(tactics: &[Tactic], by_span: Span) -> Span {
    tactics.last().map(Tactic::span).unwrap_or(by_span)
}

/// 在 `pat` 里找到「恰好是 `Ident(name)`」的那个位置，返回 `actual` 中**同位**
/// 的子项。头/实参个数不一致就放弃（与 `unify_spine` 同强度，不做高阶匹配）。
fn align_ident(pat: &Expr, actual: &Expr, name: &str) -> Option<Expr> {
    if let Expr::Ident { name: n, .. } = pat {
        return (n.as_str() == name).then(|| actual.clone());
    }
    let (ph, pa) = crate::spine::spine_with_notation(pat);
    let (ah, aa) = crate::spine::spine_with_notation(actual);
    if !crate::spine::same_head(&ph, &ah) || pa.len() != aa.len() {
        return None;
    }
    pa.iter()
        .zip(aa.iter())
        .find_map(|(p, a)| align_ident(p, a, name))
}

/// 位置 spine 合一漏掉的**类型参数**，用「已填层的 domain ↔ 该层值的类型」反推。
///
/// 触发条件（见调用点的长注释）：层名会被判成类型参数、却不在 σ 里。做法是
/// 找一层 `j`：它已填（`σ[j]` 有值）、它的 domain 里出现待解的名字 `name`；
/// 问内核 `σ[j]` 的类型，再把 `layers[j].domain` 与那个类型**逐位对齐**，
/// `Ident(name)` 对上谁就取谁。多轮迭代（一轮可能解开依赖前一轮的参数）。
///
/// 只在真的缺项时才跑，且只对已填层付一次 `judge_infer`（判定结果有缓存）。
fn solve_type_params(
    layers: &[(String, Expr)],
    codomain: &Expr,
    goal: &Expr,
    sigma: &mut std::collections::HashMap<String, Expr>,
    spec: &OpenGoalSpec,
    prefix_src: &str,
    options: &CompileOptions,
) {
    // 先圈出「缺项 + 会被判成类型参数」的名字，避免无谓的内核查询。
    let missing: Vec<String> = layers
        .iter()
        .filter(|(name, domain)| {
            !name.is_empty()
                && !sigma.contains_key(name)
                && (mentions(name, codomain) || mentions(name, goal) || is_universe_domain(domain))
        })
        .map(|(name, _)| name.clone())
        .collect();
    if missing.is_empty() {
        return;
    }
    for _ in 0..layers.len() {
        let mut progress = false;
        for name in &missing {
            if sigma.contains_key(name) {
                continue;
            }
            let Some(j) = (0..layers.len()).find(|&j| {
                let (m, domain) = &layers[j];
                !m.is_empty() && sigma.contains_key(m) && mentions(name, domain)
            }) else {
                continue;
            };
            let probe = render_expr(&sigma[&layers[j].0]);
            let Ok(ty_text) = judge_infer(prefix_src, options, &spec.binders, &probe) else {
                continue;
            };
            let Ok(ty_expr) = parse_expr_text(&ty_text) else {
                continue;
            };
            if let Some(v) = align_ident(&layers[j].1, &ty_expr, name) {
                sigma.insert(name.clone(), v);
                progress = true;
            }
        }
        if !progress {
            break;
        }
    }
}

/// `apply f`：推断 f 的类型，位置 spine 合一 codomain 与目标，
/// 类型参数由 σ 填充，其余 binder 变成子目标。
// 参数已 8 个（0.62.0 起多一个 `universe`：判定合成声明要带声明的宇宙
// 参数，见 `docs/design/by-tactics.md` §12）。为压 clippy 把参数打包成
// 结构体只会给热路径加一层间接，得不偿失。
#[allow(clippy::too_many_arguments)]
fn apply_tactic(
    expr: &Expr,
    span: Span,
    nodes: &mut Vec<GoalNode>,
    worklist: &mut Vec<usize>,
    universe: &[String],
    prefix_src: &str,
    options: &CompileOptions,
    defs: &DefTable,
) -> Result<(), CompileError> {
    let cur = *worklist.last().ok_or_else(|| {
        CompileError::elab(ErrorKind::ElabTacticFailed, "by 块里没有待解目标", span)
    })?;
    let term = render_expr(expr);
    let spec = spec_of(nodes, cur, universe);
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
        // delta 版剥层：`h : A ⊆ B`（`Set.subset` 是 def）也要能当函数用
        // ——否则 `apply h` 报「`h` 的结果是 `Set.subset α A B`，无法对齐目标」。
        // 层级提示**逐轮按当前 `t` 算**（第一轮 `Ne (Set α) A B` ⇒ 1，展开后
        // 是 `Eq.{1} … -> False` 的 Arrow，不再展开）：拿别轮的提示填本轮的
        // 宇宙变量会静默错层级，比不填更糟。
        let hint = level_hint_of(&t, defs, &spec.binders, prefix_src, options);
        let Some(pi) = peel_pi_delta(&t, defs, hint.as_deref()) else {
            break t;
        };
        layers.push((pi.name, pi.domain));
        t = pi.body;
    };
    // 位置 spine 合一：codomain 与当前目标头相同、逐参数位对应 → σ。
    let goal = nodes[cur].ty.clone();
    let sigma = match unify_spine(&codomain, &goal, &layers) {
        Some(sigma) => sigma,
        None => {
            // **记法 / 集合字面量的失败重试**（G-04；设计
            // `docs/design/course-lean-style.md` X1）：目标里的 `A ∧ B` /
            // `x ∈ A` / `{a}` 只在前端存在，内核 pp 出的是 `And A B` /
            // `Set.mem α x A`——`unify_spine` 是**文本**对齐，头与实参个数
            // 都对不上，于是 `apply And.intro` 在 `A ∧ B` 上假报「目标不匹配」。
            // 把目标过一遍内核 pp（点名为准）再试一次。
            //
            // 只在**第一次失败**时才付这个代价，而且**不替换节点上的目标**
            // ——`rfl`/`match` 这些要读目标结构的 tactic 继续看源 AST
            // （实测：替换根目标会让 `rfl` 在 `Eq.{1} (Set α) (Aᶜ) …` 上报
            // 「需要一个 `Eq α x y` 形状的目标」，因为 pp 会丢掉隐式实参）。
            let canonical = canonical_goal_with_spec(&goal, &spec, prefix_src, options, defs);
            // 第二刀重试：**目标头再展开一层**。`h : A ⊆ B` 被 delta 展开后
            // codomain 是 `B x`（谓词应用形状），而目标 `a ∈ B` 是
            // `Set.mem α a B`——两边处在不同的展开层级，仍然对不上。
            // 把目标头也展开一层（`Set.mem α a B` → `B a`）两边就同形了。
            let unfolded_goal = {
                let hint = level_hint_of(&canonical, defs, &spec.binders, prefix_src, options);
                unfold_head_once(&canonical, defs, hint.as_deref())
                    .unwrap_or_else(|| canonical.clone())
            };
            unify_spine(&codomain, &canonical, &layers)
                .or_else(|| unify_spine(&codomain, &unfolded_goal, &layers))
                .ok_or_else(|| {
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
                })?
        }
    };
    // —— 类型参数**二次求解**（R2 实测的 `apply Set.ext` 静默错子目标）——
    //
    // 位置 spine 合一只认「codomain 实参位 = 层名」的形状。`Set.ext` 的 pp
    // 签名把 `Eq` 的类型实参丢了（`Eq A B`，见下）⇒ 类型参数 `α` **根本不在
    // 任何实参位上**，σ 里没有它。以前的做法是按名字回退到「同名上下文变量」，
    // 而 `Set.ext` 的参数恰好就叫 `α`：
    //   · 上下文没有 `α` ⇒ 合成声明里 `α` 变成**未知标识符**（`apply Set.ext`
    //     在元素类型叫 `β`/`γ` 的定理上报 `unknown identifier α`）；
    //   · 上下文**有**一个无关的 `α` ⇒ 静默填错，子目标变成 `y : α`，之后
    //     每条 `exact` 都报「期望 `C y`，实际是 `C y`」这种字面相同的怪消息
    //     （内核 def_eq 拿到的其实是 α/β 两个不同 binder 的 bvar）。
    //
    // 正确值可以从**已填层的 domain** 反推：`A : Set α` 且 `A` 已填成
    // `f '' (f ⁻¹' C)`，后者的类型是 `Set β` ⇒ `α := β`。只对「会被判成类型
    // 参数」的层做，且只在真的缺项时才付一次内核查询。
    let mut sigma = sigma;
    solve_type_params(
        &layers, &codomain, &goal, &mut sigma, &spec, prefix_src, options,
    );
    // 按层序构造实参：codomain 里出现的命名 binder = 类型参数（σ 填充）；
    // 其余 = 子目标（域类型 σ 代入）。
    let mut args: Vec<ApplyArg> = Vec::new();
    let mut sub_nodes: Vec<usize> = Vec::new();
    for (name, domain) in &layers {
        // 这一层是不是**类型参数**：看它的名字有没有出现在「结论」里。
        //
        // **两边都要看**：内核 pp 会丢掉隐式实参——`Eq.{1} (Set α) A B` 打成
        // `Eq A B`（设计 `docs/design/course-lean-style.md` L1.4 的 bug ②）。
        // 只看 codomain 的话 `α` 会被误判成子目标，`apply Set.ext` 于是多出
        // 一个 `Sort 1` 的垃圾子目标、真正的子目标还错位（实测）。
        // 目标侧保留着完整实参，补上这一眼即可；σ 里没有的名字（只在目标里
        // 出现的那些）按同名上下文变量填——它们本来就是这个作用域里的变量。
        // `is_universe_domain`：域是**宇宙**（`Type`/`Sort u`）的层**永远不是子目标**
        // ——`⊢ Sort 1` 是证不出来的。它必须算类型参数，否则会多出一个垃圾子目标、
        // 真子目标错位。
        //
        // 什么时候两个 `mentions` 都不成立却仍需这样判：**目标用了记法**。
        // 实测 `theorem t (α : Type) (A B : Set α) : A = B := by apply Set.ext`
        // ——目标 `A = B` 的源 AST 是 `Notation{=, [A, B]}`，**没有 `α`**；
        // 而 `judge_infer` 给的 `Set.ext` 签名文本是
        // `forall (α : Type 0) (A B : Set α), … -> Eq A B`，**pp 把 `Eq` 的
        // 类型参数丢了**（`Eq A B` 只有两个实参）⇒ 两边都不提 `α`，`α` 被判成
        // 子目标、类型是 `Type 0`（= `Sort 1`），于是 `intro x` 报
        // 「需要一个函数目标」。`Prop` **不在此列**：`(h : Prop) -> C` 是合法签名，
        // `⊢ Prop` 也确实是子目标。
        if !name.is_empty()
            && (mentions(name, &codomain) || mentions(name, &goal) || is_universe_domain(domain))
        {
            let filled = sigma.get(name).cloned().unwrap_or_else(|| Expr::Ident {
                name: name.clone(),
                span,
            });
            args.push(ApplyArg::TypeParam(filled));
        } else {
            // 代入之后**消 redex**：交集出去的是 lambda 证人（`fun a => f2 (f a)`），
            // 子目标类型里就会出现 `(fun …) …`；而**内核不做 beta 转换** ⇒ 不消
            // 掉的话后面每条 `exact` 都报"期望 (fun …) … 实际是 …"（R2 实测）。
            let sub_ty = crate::spine::beta_normalize(&substitute(domain, &sigma));
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

/// `cases h [with | ctor a b => <tactics> …]`。
///
/// 实现路线（设计 `docs/design/course-lean-style.md` L3.1）：**降低成 `match`**
/// ——每个分支造一个子目标节点（分支假设进 `intros`），臂体的 tactic 序列在
/// 那个节点上跑；最后父节点变成 `NodeKind::Cases`，`assemble` 产出
/// `Expr::Match`。递归子与 iota 规则交给**既有的 `match` 降低路径**，引擎不
/// 手搓 recursor；判定仍在内核。
///
/// 分支假设的**类型**来自归纳表的 `MatchField.src_ty`（声明里写的那个），
/// 用 scrutinee 的**实际参数**做代换：`h : Or (A x) (B x)` 的分支假设就是
/// `A x` 而不是声明里的参数名 `A`。参数从 scrutinee 的类型经**内核 pp**取
/// （`judge_infer`）——源里写的是 `A x ∨ B x`（记法节点），pp 之后才是
/// `Or (A x) (B x)` 这种带全部实参的点名形状。
#[allow(clippy::too_many_arguments)]
fn cases_tactic(
    expr: &Expr,
    arms: &[crate::ast::CasesArm],
    span: Span,
    nodes: &mut Vec<GoalNode>,
    worklist: &mut Vec<usize>,
    steps: &mut Vec<ByStep>,
    universe: &[String],
    prefix_src: &str,
    options: &CompileOptions,
    inductives: &InductiveTable<'_>,
    defs: &DefTable,
) -> Result<(), CompileError> {
    let cur = *worklist
        .last()
        .ok_or_else(|| tactic_error("by 块里没有待解目标", span))?;
    let Expr::Ident {
        name: scrutinee_name,
        ..
    } = expr
    else {
        return Err(tactic_error(
            "`cases` 的被消去项必须是一个**局部假设名**（例如 `cases h`）",
            span,
        ));
    };
    let goal_ty = nodes[cur].ty.clone();
    if mentions(scrutinee_name, &goal_ty) {
        return Err(tactic_error(
            format!(
                "`cases {scrutinee_name}` 暂不支持**目标依赖该假设**的情形（dependent elimination）：\
                 目标 `{}` 里提到了 `{scrutinee_name}`",
                render_expr(&goal_ty)
            ),
            span,
        ));
    }
    // scrutinee 的**规范类型**（内核 pp，点名为准，带全部实参）。
    let binders_before: Vec<GoalBinderSpec> = context_binders(nodes, cur)
        .iter()
        .map(|b| GoalBinderSpec {
            name: b.name.clone(),
            ty: b.ty.as_deref().map(render_expr),
        })
        .collect();
    let scrutinee_ty = judge_infer(prefix_src, options, &binders_before, scrutinee_name)
        .ok()
        .and_then(|text| parse_expr_text(&text).ok())
        .ok_or_else(|| tactic_error(format!("`cases` 读不到 `{scrutinee_name}` 的类型"), span))?;
    // 把该假设的**书写类型**换成规范形态：`cases` 降低成 `match`，而 `match`
    // 要从书写类型里取参数实参（`h : Or A B` ⇒ params `A B`）。源里写的是记法
    // `A ∨ B`（`Expr::Notation`），`src_spine` 看不见 ⇒ 报「被匹配项必须是一个
    // 书写类型为 `Or …` 的局部变量」。换掉之后两边一致（实测改前必炸）。
    // 只动这一个 binder 的类型，且只在 `cases` 路径上——其余目标的源 AST 不变。
    // **逐层 delta 展开到归纳**：`h : x ∈ A ∪ B` 的规范类型是
    // `Set.mem α x (Set.union α A B)`，而 `∈`/`∪` 都是 **def**——展开两层才
    // 露出 `Or`。不展开就报「`cases` 只支持归纳类型，但头 `Set.mem` 不在归纳表里」
    // （实测：集合论里最常见的 `cases h` on 并集成员就是这个形状）。
    // **先**取源类型：下面的 `canonicalize_binder_type` 会把这条假设的**书写类型**
    // 换成 pp 形态（`match` 需要），而 pp 形态丢掉隐式宇宙参数（`Ne.{1}` → 裸
    // `Ne`）——所以源类型必须在这一步之前抄下来（R2 实测：抄晚了就拿不到记法节点，
    // `cases` 臂里的假设类型带着 `Ne.{0}`）。
    let source_ty: Option<Expr> = context_binders(nodes, cur)
        .iter()
        .rev()
        .find(|b| b.name == *scrutinee_name)
        .and_then(|b| b.ty.as_deref().cloned());
    let scrutinee_hint = level_hint_of(&scrutinee_ty, defs, &binders_before, prefix_src, options);
    let scrutinee_ty = crate::spine::unfold_to_inductive(
        &scrutinee_ty,
        &|name| inductives.contains_key(name),
        defs,
        4,
        scrutinee_hint.as_deref(),
    );
    // **写回节点前先把宇宙层级补回来**（R2 实测）：pp 形态丢掉隐式宇宙参数
    // （`Eq.{1} β (f a) b` → 裸 `Eq (f a) b`），而这条类型不只判定时要用——
    // `match` 的组装与最终声明判定**都从节点上读它**。留着裸 `Eq`，臂里那条
    // 假设一被使用，内核就按默认 `.{0}` elaborate，报「期望 `Sort(0)`，
    // 实际是 β」；位置还在**整条声明**上，离根因极远。
    let scrutinee_ty =
        restore_universe_levels(&scrutinee_ty, defs, &binders_before, prefix_src, options);
    canonicalize_binder_type(nodes, Some(cur), scrutinee_name, &scrutinee_ty);
    // **头解析必须认记法**（R2.5 实测）：`unfold_to_inductive` 把 `Set.union` 的
    // 定义体代进来之后，形态取决于**定义体怎么写**——库里写 `Or (A x) (B x)`
    // 就是 `App` 链，写 `A x ∨ B x` 就是**记法节点**。`spine_of` 只走 `Expr::App`，
    // 于是「库改用记法」会把所有 `cases h`（`h : x ∈ A ∪ B`）整类打红，报
    // 「被消去项不是归纳类型的值」。`spine_with_notation` 两条都认（源实参那条
    // 路本来就在用它），所以这里改用它——记法节点的头就是它的 `target`。
    let (head_expr, written_args) = crate::spine::spine_with_notation(&scrutinee_ty);
    let ind_name = match &head_expr {
        Expr::Ident { name, .. } | Expr::UniverseApp { name, .. } => name.clone(),
        _ => {
            return Err(tactic_error(
                format!(
                    "`cases` 的被消去项不是归纳类型的值：`{}`",
                    render_expr(&scrutinee_ty)
                ),
                span,
            ))
        }
    };
    let Some(info) = inductives.get(&ind_name) else {
        return Err(tactic_error(
            format!(
                "`cases` 只支持**归纳类型**，但 `{scrutinee_name} : {}` 的头 `{ind_name}` 不在归纳表里",
                render_expr(&scrutinee_ty)
            ),
            span,
        ));
    };
    // 参数代换：声明里的参数名 → scrutinee 的实际实参。
    //
    // **实参优先取规范形态**（R2 修边刀实测；原先优先源类型，那一刀是错的）：
    // 源类型里的**记法节点**（`f '' A`）不带前导类型参数——`elab` 期才算得出来，
    // 源 AST 里没有——而 `spine::unfold_one` 只能把操作数**右对齐**到形参，
    // 于是定义体里提到前导参数的地方**留着定义自己的 binder 名**，在调用点按
    // 「同名上下文变量」解析：`Set.image` 的形参就叫 `α`/`β`，上下文里也有
    // `α`/`β`，于是 `f : α -> γ` 那条 `''` 把形参 `β` 代成了**上下文的 `β`**
    // （本该是 `γ`）⇒ 臂里的假设类型成了 `Eq.{1} β (f x) y`（β 是元素类型、
    // 这里该是 γ）⇒ 之后每条 `exact` 都报「期望 `…`，实际是 `…`」把同一条命题
    // 写成两种形态（实测：`unit12` 的 P1）。
    //
    // 规范形态（内核 pp：点名 + 全实参 + 无记法）**没有**这个问题；它唯一的短板
    // 是丢隐式宇宙参数，而这一点已由上面的 `restore_universe_levels`（写回节点
    // 之前就补齐）覆盖。源形态只在规范形态头对不上时兜底。
    let source_args: Option<Vec<Expr>> = source_ty
        .map(|src_ty| {
            let hint = level_hint_of(&src_ty, defs, &binders_before, prefix_src, options);
            let unfolded = crate::spine::unfold_to_inductive(
                &src_ty,
                &|name| inductives.contains_key(name),
                defs,
                4,
                hint.as_deref(),
            );
            let (head, args) = crate::spine::spine_with_notation(&unfolded);
            let head_name = match &head {
                Expr::Ident { name, .. } | Expr::UniverseApp { name, .. } => name.as_str(),
                _ => "",
            };
            // 头必须还是同一个归纳（否则源形态是别的层级，别乱代）。
            if head_name == ind_name {
                args
            } else {
                Vec::new()
            }
        })
        .filter(|args| !args.is_empty());
    let subst_args: Vec<Expr> = if written_args.is_empty() {
        source_args.unwrap_or_default()
    } else {
        written_args.iter().map(|a| (*a).clone()).collect()
    };
    let mut sigma: std::collections::HashMap<String, Expr> = std::collections::HashMap::new();
    for (i, param) in info.param_names.iter().enumerate() {
        if let Some(arg) = subst_args.get(i) {
            sigma.insert(param.clone(), arg.clone());
        }
    }
    // 每个构造子一个子目标；用户给了 `with` 就按用户的臂（按构造子名归一），
    // 否则按**声明顺序**自动造（分支假设用构造子字段名）。
    let mut arm_nodes: Vec<CasesArmNode> = Vec::new();
    let mut user_arms: Vec<Option<&crate::ast::CasesArm>> = Vec::new();
    for ctor in &info.ctors {
        let arm = arms.iter().find(|a| ctor_matches(&a.ctor, ctor));
        let names: Vec<String> = match arm {
            Some(a) => a.binders.clone(),
            None => ctor.fields.iter().map(|f| f.name.clone()).collect(),
        };
        if let Some(a) = arm {
            if a.binders.len() != ctor.fields.len() {
                return Err(tactic_error(
                    format!(
                        "`cases` 分支 `{}` 需要 {} 个假设名（构造子有 {} 个字段），实际给了 {} 个",
                        a.ctor,
                        ctor.fields.len(),
                        ctor.fields.len(),
                        a.binders.len()
                    ),
                    a.span,
                ));
            }
        }
        // **字段名 → 用户写的假设名**（R2 实测）：依赖字段的类型里会出现前面字段的
        // **声明名**——`Exists.intro (w : A) (h : p w)` 的 `h` 类型是 `p w`。学习者
        // 写 `| Exists.intro U hU =>` 时，`p w` 必须变成 `p U`，否则臂里留着悬空的
        // `w`，后面每条 tactic 都报 `unknown identifier w`（报错点离根因很远）。
        // 与 `intro` 的改名同一条纪律（`spine::rename_free` 的兄弟）。
        let mut field_sigma = sigma.clone();
        for (field, binder_name) in ctor.fields.iter().zip(names.iter()) {
            if !field.name.is_empty() && field.name != *binder_name {
                field_sigma.insert(
                    field.name.clone(),
                    Expr::Ident {
                        name: binder_name.clone(),
                        span,
                    },
                );
            }
        }
        let mut intros = Vec::new();
        for (field, binder_name) in ctor.fields.iter().zip(names.iter()) {
            // 代入之后**消 redex**（与 `apply` 的子目标同一条纪律，见上面
            // `apply_tactic` 的 `beta_normalize`）：依赖字段的类型里会嵌进
            // 参数代换后的 lambda——`Exists` 的 `h : p w` 代成
            // `(fun (b : β) => …) b`。**内核不做 beta 转换** ⇒ 留着这个 redex，
            // 臂里 `And.left … hb` / `hb` 的每次使用都对不上（实测报
            // 「期望 Sort(0)，实际是 $7」这种与现场毫无关系的消息）。
            let ty = field
                .src_ty
                .as_ref()
                .map(|t| crate::spine::beta_normalize(&substitute(t, &field_sigma)))
                .ok_or_else(|| {
                    tactic_error(
                        format!("`cases` 读不到构造子 `{}` 的字段类型", ctor.name),
                        span,
                    )
                })?;
            intros.push(Binder {
                name: binder_name.clone(),
                ty: Some(Box::new(ty)),
                style: BinderKind::Explicit,
                span,
            });
        }
        let pattern = Pattern::Ident {
            name: ctor.name.clone(),
            args: names
                .iter()
                .map(|n| Pattern::Ident {
                    name: n.clone(),
                    args: Vec::new(),
                    span,
                })
                .collect(),
            span,
        };
        let id = nodes.len();
        nodes.push(GoalNode {
            ty: goal_ty.clone(),
            intros,
            parent: Some(cur),
            kind: NodeKind::Hole,
        });
        arm_nodes.push(CasesArmNode {
            pattern,
            node: id,
            binders: names.len(),
        });
        user_arms.push(arm);
    }
    nodes[cur].kind = NodeKind::Cases {
        scrutinee: expr.clone(),
        arms: arm_nodes.clone(),
    };
    // 当前目标被 `cases` 消耗。
    worklist.retain(|&x| x != cur);
    if arms.is_empty() {
        // 不带 `with`：子目标按声明顺序留给后面的 tactic 逐个解。
        for arm in arm_nodes.iter().rev() {
            worklist.push(arm.node);
        }
        return Ok(());
    }
    // 带 `with`：每个臂的 tactic 序列在自己的子目标上跑完，然后从 worklist
    // 上摘掉（连同它自己推上去的子目标——用 truncate 保证外层状态干净）。
    for (arm, user) in arm_nodes.iter().zip(user_arms.iter()) {
        let Some(user) = user else { continue };
        let base = worklist.len();
        worklist.push(arm.node);
        run_tactics(
            &user.tactics,
            nodes,
            worklist,
            steps,
            universe,
            prefix_src,
            options,
            inductives,
            defs,
        )?;
        worklist.truncate(base);
    }
    Ok(())
}

/// 沿父链找名为 `name` 的假设，把它的书写类型换成 `canonical`
/// （内核 pp 的规范形态，点名为准）。找不到就什么都不做。
fn canonicalize_binder_type(
    nodes: &mut [GoalNode],
    mut id: Option<usize>,
    name: &str,
    canonical: &Expr,
) {
    while let Some(i) = id {
        if let Some(binder) = nodes[i].intros.iter_mut().find(|b| b.name == name) {
            binder.ty = Some(Box::new(canonical.clone()));
            return;
        }
        id = nodes[i].parent;
    }
}

/// 用户的臂名与归纳表里的构造子是不是同一个：裸名、点号名、以及
/// 「表里的全名以 `.<裸名>` 结尾」三种都认。
fn ctor_matches(written: &str, ctor: &MatchCtor<'_>) -> bool {
    written == ctor.name
        || written == ctor.canonical
        || ctor.name.ends_with(&format!(".{written}"))
        || ctor.canonical.ends_with(&format!(".{written}"))
}

/// 组装整棵树 → 单一 lambda AST（叶子洞 = `Expr::Hole`，span 用未闭合目标
/// 的洞位——`by … sorry` 里就是 `sorry` 的位置，供 open_goal/hole 导航使用）。
fn assemble(nodes: &[GoalNode], id: usize, hole_span: Span) -> Expr {
    let mut expr = node_body(nodes, id, hole_span);
    for binder in nodes[id].intros.iter().rev() {
        expr = Expr::Lambda {
            binders: vec![binder.clone()],
            body: Box::new(expr),
            span: Span::default(),
        };
    }
    expr
}

/// 组装节点的**体**（不含本节点的 intros 包装）。
fn node_body(nodes: &[GoalNode], id: usize, hole_span: Span) -> Expr {
    match &nodes[id].kind {
        NodeKind::Hole => Expr::Hole { span: hole_span },
        NodeKind::Closed(e) => e.clone(),
        // **`let h : T := t; <body>`**——`h` 在 `<body>` 里是自由变量，由这一层
        // 绑定（它同时出现在父链的 intros 里，供判定用）。
        //
        // 为什么用 `Expr::Let` 而不是 `(fun (h : T) => <body>) t`：`let` 的
        // **标注给值一个期望类型**。嵌套 `by` 块里只要用了 `cases`，组装出来的
        // 值就是一个 `Expr::Match`——作为应用的实参时没有期望类型，elab 直接报
        // `elab-match-no-expected-type`（实测）。`let` 的 `ty` 正好补上这一层。
        NodeKind::Have {
            name,
            ty,
            value,
            body,
        } => Expr::Let {
            binder: Binder {
                name: name.clone(),
                ty: Some(Box::new(ty.clone())),
                style: BinderKind::Explicit,
                span: Span::default(),
            },
            val: Box::new(value.clone()),
            body: Box::new(assemble(nodes, *body, hole_span)),
            span: Span::default(),
        },
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
                    explicit_spine: false,
                    span: Span::default(),
                };
            }
            app
        }
        NodeKind::Cases { scrutinee, arms } => Expr::Match {
            scrutinee: Box::new(scrutinee.clone()),
            arms: arms
                .iter()
                .map(|arm| MatchArm {
                    pattern: arm.pattern.clone(),
                    guard: None,
                    body: arm_body(nodes, arm, hole_span),
                    span: Span::default(),
                })
                .collect(),
            span: Span::default(),
        },
    }
}

/// 组装一个 `cases` 分支的体：**跳过**模式绑定的那几个 intro
/// （`| Or.inl ha => …` 里的 `ha` 由模式绑定，包成 lambda 就错了），
/// 只把臂体里自己 `intro` 出来的那些包上。
fn arm_body(nodes: &[GoalNode], arm: &CasesArmNode, hole_span: Span) -> Expr {
    let mut expr = node_body(nodes, arm.node, hole_span);
    for binder in nodes[arm.node].intros.iter().skip(arm.binders).rev() {
        expr = Expr::Lambda {
            binders: vec![binder.clone()],
            body: Box::new(expr),
            span: Span::default(),
        };
    }
    expr
}

/// 目标节点 `id` 的判定规格（剩余目标 + 沿父链的全部 intros）。
fn spec_of(nodes: &[GoalNode], id: usize, universe: &[String]) -> OpenGoalSpec {
    OpenGoalSpec {
        universe: universe.to_vec(),
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

/// 同 [`spec_of`]，但把假设类型的**裸名宇宙层级补回来**（判定用）。
///
/// 为什么判定这一侧要单独补：`cases` 会把假设的**书写类型**换成内核 pp 的
/// 规范形态（`match` 需要），而 pp 省掉隐式宇宙参数（`Ne.{1}` → 裸 `Ne`）——
/// 那条类型进不了判定（`Set α` 不是 `Sort 0` 的元素 ⇒ 整条合成声明 elaborate
/// 不了，报错却是「期望 Sort(0)，实际是 Sort(1)」）。补层级只动**判定用的
/// 文本**，不动节点上的类型（`match` 组装读的是节点）。
fn spec_of_for_judge(
    nodes: &[GoalNode],
    id: usize,
    defs: &DefTable,
    universe: &[String],
    prefix_src: &str,
    options: &CompileOptions,
) -> OpenGoalSpec {
    let base = spec_of(nodes, id, universe);
    let binders = context_binders(nodes, id)
        .iter()
        .map(|b| GoalBinderSpec {
            name: b.name.clone(),
            ty: b.ty.as_deref().map(|t| {
                render_expr(&restore_universe_levels(
                    t,
                    defs,
                    &base.binders,
                    prefix_src,
                    options,
                ))
            }),
        })
        .collect();
    OpenGoalSpec {
        universe: universe.to_vec(),
        ty: render_expr(&restore_universe_levels(
            &nodes[id].ty,
            defs,
            &base.binders,
            prefix_src,
            options,
        )),
        binders,
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
    let mut out: Vec<Binder> = Vec::new();
    for i in chain {
        out.extend(nodes[i].intros.iter().cloned());
        // `have` 引入的假设也进上下文（它的 lambda 在**组装**时才包上，
        // 但判定必须现在就看得到 `h : T`）。
        if let NodeKind::Have { name, ty, .. } = &nodes[i].kind {
            out.push(Binder {
                name: name.clone(),
                ty: Some(Box::new(ty.clone())),
                style: BinderKind::Explicit,
                span: Span::default(),
            });
        }
    }
    out
}

/// 这一层的域是不是**宇宙**（`Type` / `Type n` / `Sort u`）——`Prop` **不算**。
///
/// 用途见 `apply_tactic` 的类型参数判定：宇宙层不可能有子目标（`⊢ Sort 1`
/// 证不出来），必须当类型参数填。`Prop` 是合法的目标类型，所以排除在外。
fn is_universe_domain(domain: &Expr) -> bool {
    matches!(
        domain,
        Expr::Sort { sort, .. } if !matches!(sort, crate::SortKind::Prop)
    )
}

/// 类型不匹配的**教学文案**。
///
/// 内核给的 `expected`/`actual` 是**折叠回望远镜的完整声明类型**
/// （`Pi (A : Sort(0)), Pi (B : Sort(0)), …`）——准确，但学习者读不懂；实测
/// `exact h1 ha` 在目标 `B` 上就长这样（`have` 与 `exact` 同病）。这里改说人话：
/// **期望 `<目标>`，实际是 `<值的类型>`**——后者用 `judge_infer` 拿**值本身**的
/// 类型文本（只在**错误路径**上多问一次内核，好路径零开销）。
fn mismatch_message(
    // `what` 是完整的动词短语：`` `exact` 类型不匹配 `` / `` `have hb` 的值类型不匹配 ``。
    what: &str,
    goal_text: &str,
    term: &str,
    spec: &OpenGoalSpec,
    prefix_src: &str,
    options: &CompileOptions,
) -> String {
    let actual = match judge_infer(prefix_src, options, &spec.binders, term) {
        Ok(text) => text,
        Err(_) => "（值的类型也读不出来，检查它里面的名字）".to_string(),
    };
    format!("{what}：期望 `{goal_text}`，实际是 `{actual}`")
}

/// `exact e`：判定 `e` 的类型与**当前目标**一致 ⇒ 闭合它。
/// `use` 复用它交证人（所以单独抽出来，不复制判定逻辑）。
// 参数已 8 个（0.62.0 起多一个 `universe`：判定合成声明要带声明的宇宙
// 参数，见 `docs/design/by-tactics.md` §12）。为压 clippy 把参数打包成
// 结构体只会给热路径加一层间接，得不偿失。
#[allow(clippy::too_many_arguments)]
fn exact_tactic(
    expr: &Expr,
    span: Span,
    nodes: &mut [GoalNode],
    worklist: &mut Vec<usize>,
    universe: &[String],
    prefix_src: &str,
    options: &CompileOptions,
    defs: &DefTable,
) -> Result<(), CompileError> {
    let cur = *worklist
        .last()
        .ok_or_else(|| tactic_error("by 块里没有待解目标", span))?;
    let term = render_expr(expr);
    let j = judge_with_levels(
        prefix_src,
        options,
        nodes,
        worklist,
        cur,
        universe,
        &[term.as_str()],
        defs,
    );
    match j {
        Some(Judgement::Match) => {
            nodes[cur].kind = NodeKind::Closed(expr.clone());
            worklist.pop();
            Ok(())
        }
        Some(Judgement::Mismatch { .. }) => Err(CompileError::elab(
            ErrorKind::ElabTacticFailed,
            mismatch_message(
                "`exact` 类型不匹配",
                &render_expr(&nodes[cur].ty),
                &term,
                &spec_of(nodes, cur, universe),
                prefix_src,
                options,
            ),
            span,
        )),
        Some(Judgement::Error { message, .. }) => Err(CompileError::elab(
            ErrorKind::ElabTacticFailed,
            format!("`exact` 判定失败：{message}"),
            span,
        )),
        None => unreachable!("judge returns one judgement per term"),
    }
}

/// prelude 里以 **def** 形态存在的「单构造子类型」→ 它的构造子式引理。
///
/// **单一真相源在 `compile::elab::builtin_constructor_of`**（`⟨…⟩` 的 elab
/// 也要用它）：两处各写一份必然分叉。
use crate::compile::elab::builtin_constructor_of;

/// `constructor` / `left` / `right` / `use` 的公共主体。
///
/// **先把目标展开到归纳头**再选构造子（`x ∈ B ∩ C` 要 `Set.mem` → `Set.inter`
/// 两层才露出 `And`；只展开一层时学习者看到「`constructor` 需要目标是归纳类型，
/// 但当前目标的头 `Set.inter` 不在归纳表里」——而目标明明就是集合成员关系）。
///
/// 展开出来的形态**临时写回节点**再 `apply`：`apply` 的合一读的是
/// `nodes[cur].ty`，不写回就还是拿 `a ∈ B ∩ C` 去对齐 `And.intro` 的
/// `And a b`（实测报「目标不匹配」）。apply 之后**还原**原目标——子目标已经
/// 建好，还原只是让目标面板继续显示记法形态（教学更好读）。
#[allow(clippy::too_many_arguments)]
fn ctor_tactic(
    index: usize,
    what: &str,
    span: Span,
    nodes: &mut Vec<GoalNode>,
    worklist: &mut Vec<usize>,
    universe: &[String],
    prefix_src: &str,
    options: &CompileOptions,
    inductives: &InductiveTable<'_>,
    defs: &DefTable,
) -> Result<(), CompileError> {
    let cur = *worklist
        .last()
        .ok_or_else(|| tactic_error("by 块里没有待解目标", span))?;
    let spec = spec_of(nodes, cur, universe);
    let original = nodes[cur].ty.clone();
    let unfolded = goal_unfolded_to_inductive(
        &original,
        inductives,
        defs,
        &spec.binders,
        prefix_src,
        options,
    );
    nodes[cur].ty = unfolded.clone();
    let result = (|| {
        let ctor = nth_constructor(inductives, &unfolded, index, span, what)?;
        apply_tactic(
            &ctor, span, nodes, worklist, universe, prefix_src, options, defs,
        )
    })();
    if let Some(node) = nodes.get_mut(cur) {
        node.ty = original;
    }
    result
}

/// 目标**头**那个归纳的名字：`And A B` → `And`；记法目标 `A ∧ B` → 它的
/// **目标名** `And`（`Expr::Notation` 的 `target`）——`by` 引擎吃的是源 AST，
/// 记法节点还没展开，所以两种形态都要认。
fn goal_head_name(ty: &Expr) -> Option<&str> {
    if let Expr::Notation { target, .. } = ty {
        return Some(target);
    }
    let (head, _) = spine_of(ty);
    match head {
        Expr::Ident { name, .. } | Expr::UniverseApp { name, .. } => Some(name),
        _ => None,
    }
}

/// 目标头那个归纳的构造子（**声明顺序**，来自前端自己的归纳表）。
fn goal_constructors<'t, 'a>(
    inductives: &'t InductiveTable<'a>,
    ty: &Expr,
) -> Option<&'t [MatchCtor<'a>]> {
    let name = goal_head_name(ty)?;
    inductives.get(name).map(|info| info.ctors.as_slice())
}

/// 取目标头归纳的第 `index` 个构造子，包成一个可直接交给 `apply` 的项。
///
/// 找不到归纳 / 构造子不够都报**教学错误**（说清是哪个名字、有几个构造子），
/// 不做静默回退：`constructor` 这类糖一旦猜错，学习者看到的是内核的裸类型不符。
fn nth_constructor<'t, 'a>(
    inductives: &'t InductiveTable<'a>,
    ty: &Expr,
    index: usize,
    span: Span,
    what: &str,
) -> Result<Expr, CompileError> {
    let head = goal_head_name(ty).unwrap_or("?").to_string();
    let Some(ctors) = goal_constructors(inductives, ty) else {
        // 内建兜底：prelude 里有几个「类型构造子」是 **def** 而不是 inductive
        // ——`Iff A B` 展开成 `And (A -> B) (B -> A)`（`PRELUDE_L1_SRC`），
        // 归纳表里当然没有它，但 `Iff.intro` 存在且签名正确，Lean 里 `Iff`
        // 本来就是 structure ⇒ `constructor` 该能用（实测改前报「不在归纳表里」）。
        // **只列语言自己 prelude 定义的**：课程/用户自定义的 def 不猜——
        // 猜错会让学习者看到内核的裸类型不符，比一句人话报错糟得多。
        if index == 0 {
            if let Some(name) = builtin_constructor_of(&head) {
                return Ok(Expr::Ident {
                    name: name.to_string(),
                    span,
                });
            }
        }
        return Err(tactic_error(
            format!(
                "`{what}` 需要目标是**归纳类型**，但当前目标的头 `{head}` 不在归纳表里\
                 （它可能是公理、定义，或者是一个函数目标——先 `intro` 拆开试试）"
            ),
            span,
        ));
    };
    let Some(ctor) = ctors.get(index) else {
        return Err(tactic_error(
            format!(
                "`{what}` 需要目标至少有 {} 个构造子，但 `{head}` 只有 {} 个",
                index + 1,
                ctors.len()
            ),
            span,
        ));
    };
    Ok(Expr::Ident {
        name: ctor.canonical.clone(),
        span,
    })
}

/// 对 `term` 在当前目标上下文做 kernel 判定（`judge_terms`）。
// 参数已 8 个（0.62.0 起多一个 `universe`：判定合成声明要带声明的宇宙
// 参数，见 `docs/design/by-tactics.md` §12）。为压 clippy 把参数打包成
// 结构体只会给热路径加一层间接，得不偿失。
#[allow(clippy::too_many_arguments)]
fn judge_with_levels(
    prefix_src: &str,
    options: &CompileOptions,
    nodes: &[GoalNode],
    worklist: &[usize],
    cur: usize,
    universe: &[String],
    terms: &[&str],
    defs: &DefTable,
) -> Option<Judgement> {
    let spec = spec_of_for_judge(nodes, cur, defs, universe, prefix_src, options);
    let _ = worklist;
    judge_terms(prefix_src, options, &spec, terms)
        .into_iter()
        .next()
}

#[allow(clippy::too_many_arguments)]
fn judge(
    prefix_src: &str,
    options: &CompileOptions,
    nodes: &[GoalNode],
    worklist: &[usize],
    cur: usize,
    universe: &[String],
    terms: &[&str],
) -> Option<Judgement> {
    let spec = spec_of(nodes, cur, universe);
    let _ = worklist;
    judge_terms(prefix_src, options, &spec, terms)
        .into_iter()
        .next()
}

/// [`judge`] 的**严格**版本：绕过乐观批次，当场判（只有需要读结论本身的
/// tactic 才用它，见 `Tactic::Assumption`）。
#[allow(clippy::too_many_arguments)]
fn judge_strict(
    prefix_src: &str,
    options: &CompileOptions,
    nodes: &[GoalNode],
    worklist: &[usize],
    cur: usize,
    universe: &[String],
    terms: &[&str],
) -> Option<Judgement> {
    let spec = spec_of(nodes, cur, universe);
    let _ = worklist;
    judge_terms_strict(prefix_src, options, &spec, terms)
        .into_iter()
        .next()
}

/// `rfl` 候选：目标 `Eq α x y` → `Eq.refl.{u} α x`（kernel 判定两边）。
///
/// 返回 `(判定用文本, 闭合用 AST)`：**AST 直接构造**，不再把文本回读一遍。
/// 回读要认识记法（`Eq.refl.{1} (Set α) ((A ᶜ) ∪ B)` 里的符号），而 `by` 引擎
/// 手里没有记法表——构造 AST 从根上绕开这条文本往返（G-04 第二刀实测：记法
/// 操作数上的 `by rfl` 曾整条判红）。
fn rfl_candidate(goal: &Expr) -> Option<(String, Expr)> {
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
    let text = format!("Eq.refl.{{{level}}} {alpha} {a}");
    let span = goal.span();
    let refl = Expr::UniverseApp {
        name: "Eq.refl".to_string(),
        levels: vec![level],
        span,
    };
    let applied = Expr::App {
        fun: Box::new(refl),
        arg: Box::new(args[0].clone()),
        explicit_spine: false,
        span,
    };
    let expr = Expr::App {
        fun: Box::new(applied),
        arg: Box::new(args[1].clone()),
        explicit_spine: false,
        span,
    };
    Some((text, expr))
}

/// 原子位的文本：**与 `render_atom` 同源**（复合式补括号）。
///
/// 曾经这里自己维护一份括号清单，漏了 `Let`/`Match`/`Notation`——`rfl` 候选
/// `Eq.refl.{1} (Set α) (Aᶜ) ∪ B` 因此被读成 `(Eq.refl … Aᶜ) ∪ B`（G-04 第二刀
/// 实测：记法操作数上的 `by rfl` 全红）。括号规则只允许有一个实现。
fn atom_text(expr: &Expr) -> String {
    crate::proof::render_atom(expr)
}

fn tactic_error(msg: impl Into<String>, span: Span) -> CompileError {
    CompileError::elab(ErrorKind::ElabTacticFailed, msg, span)
}

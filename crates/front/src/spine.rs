//! Pi 望远镜 / 应用 spine 的共享机械。
//!
//! 这里的东西原先是 `by.rs` 的私有函数，值位 `apply`（`compile/apply.rs`）
//! 要用同一套——**两条 `apply` 路径必须共用一份 telescope 机械**，否则
//! 「tactic 里的 apply」与「值位 apply」会在合一强度、参数实例化上慢慢分叉
//! （`docs/design/term-apply.md` §3 明确要求）。
//!
//! 只做结构操作，不做判定：类型文本一律由 `judge::judge_infer` 问内核。

use crate::ast::{Binder, MatchArm};
use crate::Expr;
use crate::Span;

/// 剥一层 Pi/Arrow 的结果：binder 名（Arrow 层为空）、域、余下的体。
pub(crate) struct PiBody {
    pub(crate) name: String,
    pub(crate) domain: Expr,
    pub(crate) body: Expr,
}

/// 从 `expr` 剥一层 Pi/Arrow，返回 (name, domain, body)。
pub(crate) fn peel_pi(expr: &Expr) -> Option<PiBody> {
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

/// 剥一层 Pi/Arrow，**并把 `Not A` 看穿成 `A -> False`**（defeq 等价）。
///
/// `Not` 在 prelude 里是 `def Not (A : Prop) : Prop := A -> False`
/// （`compile/prelude.rs` 的 `PRELUDE_L1_SRC`），所以 `¬ A` 在**语法上**是
/// `App(Not, A)` 而不是 `Arrow`——[`peel_pi`] 看不穿定义，于是 `intro` 在
/// `¬ A` 目标上报「需要一个函数目标」（设计
/// `docs/design/course-lean-style.md` X13 实测）。否定目标在集合论课程里
/// 遍地都是（`x ∉ A`、`A ≠ B`），所以这一层必须看穿。
///
/// **只看穿 `Not` 这一层**，不做一般 whnf：一般 whnf 要内核暴露归约接口，
/// 那会碰到内核冻结（硬规则 1）。`Not` 是语言自己的词汇（prelude 定义），
/// 硬编码它是安全的——课程/入门课自己声明 `Not` 时语义也一致
/// （`Not a := a -> False`）。
pub(crate) fn peel_pi_delta(
    expr: &Expr,
    defs: &crate::compile::elab::DefTable,
    level_hint: Option<&str>,
) -> Option<PiBody> {
    peel_pi_delta_n(expr, defs, &|_| level_hint.map(str::to_string), 1)
}

/// 同 [`peel_pi_delta`]，但**逐层展开最多 `limit` 层**（每层用 `hint_of` 现算
/// 层级提示）。
///
/// 为什么需要：目标头是 def 时常常要展开**两层以上**才露出 Pi——`X ∈ 𝒫 B`
/// 是 `Set.mem` → `Set.powerset` → `Set.subset` 三层（实测：只展开一层时
/// 学习者看到「`intro` 需要一个函数目标」，而目标明明是集合成员关系）。
pub(crate) fn peel_pi_delta_n(
    expr: &Expr,
    defs: &crate::compile::elab::DefTable,
    hint_of: &dyn Fn(&Expr) -> Option<String>,
    limit: usize,
) -> Option<PiBody> {
    let mut cur = expr.clone();
    for _ in 0..limit.max(1) {
        if let Some(pi) = peel_pi(&cur) {
            return Some(pi);
        }
        // 用 `unfold_one` 而不是 `unfold_head_once`：后者在实参比形参**多**时
        // 直接放弃（内核 pp 把结果上的应用也摊平：`(Set.powerset α B) A` 是
        // 3 个实参、2 个形参），而前者会把多出来的贴回结果上（`beta_apply`）。
        // `X ∈ 𝒫 B` 要展开 `Set.mem → Set.powerset → Set.subset` 三层，正是这个
        // 形状（实测：只认 `unfold_head_once` 时第二层就断）。
        cur = unfold_one(&cur, defs, hint_of(&cur).as_deref())?;
    }
    peel_pi(&cur)
}

/// 调用点的**层级提示**：头上没写 `.{…}` 时用它填定义体的宇宙变量。
///
/// 为什么需要：`≠`（`Ne`）的源 AST 是**记法节点**、过一遍内核 pp 之后是**裸名**
/// `Ne`——两条路都**不带** `.{u}`（pp 会省掉隐式宇宙参数），而 `Ne` 的体是
/// `Eq.{u} α a b -> False`。没有提示就只能把 `u` 留成悬空变量（回读时报
/// `unknown universe level u`，报错点离根因很远）；有了提示，`intro h` 在
/// `{a} ≠ ∅` 上就能拿到正确的 `h : Eq.{1} (Set α) {a} ∅`。
///
/// 提示怎么来：调用方用**操作数的类型**问内核（`operand : T`、`T : Sort u`
/// ⇒ `u`）——与 `elab.rs::universe_level_text_of_operands` 给记法求层级是
/// **同一条规则**。两条路必须同解，否则同一个命题在「记法 elaborate」与
/// 「引擎 delta 展开」两处会落进不同的常量应用。
fn resolve_levels(
    head: &Expr,
    info: &crate::compile::elab::DefInfo,
    level_hint: Option<&str>,
) -> Vec<String> {
    if let Expr::UniverseApp { levels, .. } = head {
        return levels.clone();
    }
    // 裸名 / 记法节点：只在**恰好一个**宇宙参数、且调用方算出了提示时填。
    // 多个宇宙参数时提示不够用（今天没有这种形状的记法目标），保持既有行为
    // ——悬空变量会让回读**响亮地**报错，而不是静默错层级。
    match level_hint {
        Some(level) if info.universes.len() == 1 => vec![level.to_string()],
        _ => Vec::new(),
    }
}

/// **一层源级 delta 展开**：拿头名查 `DefTable`，按参数位置代换实参，返回定义体。
///
/// 为什么需要它：`A ⊆ B` 的 `Set.subset`、`¬ A` 的 `Not`、`A ↔ B` 的 `Iff`
/// 在 prelude/课程库里都是 **`def`**——语法上不是 Pi，`intro`/`apply` 会报
/// 「需要一个函数目标」。本语言**没有内核 whnf 的公开入口**（内核冻结），
/// 所以在前端做**源级**展开；展开出来的项仍然交内核判定（判定永远走 kernel）。
pub(crate) fn unfold_head_once(
    expr: &Expr,
    defs: &crate::compile::elab::DefTable,
    level_hint: Option<&str>,
) -> Option<Expr> {
    // 头与实参：`Ne.{1} α a b` 要同时拿到**名字**（查表）与**具体层级**
    // （实例化定义体里的宇宙变量——`Ne` 的体是 `Eq.{u} α a b -> False`）。
    let (head, args) = spine_of(expr);
    let name = match head {
        Expr::Ident { name, .. } | Expr::UniverseApp { name, .. } => name.as_str(),
        Expr::Notation { target, .. } => target.as_str(),
        _ => return None,
    };
    // 记法节点（`A ⊆ B`）：操作数就是实参。
    let args: Vec<Expr> = if let Expr::Notation { lhs, rhs, .. } = head {
        lhs.iter()
            .chain(rhs.iter())
            .map(|e| (**e).clone())
            .collect()
    } else {
        args.into_iter().cloned().collect()
    };
    let info = defs.get(name)?;
    // 参数对齐：**从右侧对齐**。记法只写出操作数（`A ⊆ B` 只有 2 个），
    // 而声明的参数可能更多（`Set.subset (α) (A) (B)` 有 3 个）——前导的
    // 类型参数被记法在 elab 期自动补上，源 AST 里没有。前导那几个**保持原名**
    // 不代换：课程库的参数名（`α`）与使用处的上下文变量同名，判卷时按上下文
    // 变量解析（这是源级展开的已知边界，写在这里以免以后当 bug 查）。
    if args.len() > info.params.len() {
        return None;
    }
    let offset = info.params.len() - args.len();
    let sigma: std::collections::HashMap<String, Expr> =
        info.params.iter().skip(offset).cloned().zip(args).collect();
    let body = instantiate_universes(
        &info.body,
        &info.universes,
        &resolve_levels(head, info, level_hint),
    );
    Some(substitute(&body, &sigma))
}

/// 记法节点也认：源里写 `A ⊆ B` / `¬ A` 时，引擎手上是
/// `Expr::Notation { target, lhs, rhs }`——头名是 `target`，实参是操作数
/// （前缀只有 `rhs`、后缀只有 `lhs`、二元是 `lhs, rhs`）。不认它，
/// 课程里最常见的 `intro x` on `A ⊆ B` 就永远展开不了（实测踩过）。
pub(crate) fn head_and_args(expr: &Expr) -> Option<(&str, Vec<Expr>)> {
    if let Expr::Notation {
        target, lhs, rhs, ..
    } = expr
    {
        let mut args: Vec<Expr> = Vec::new();
        if let Some(l) = lhs {
            args.push((**l).clone());
        }
        if let Some(r) = rhs {
            args.push((**r).clone());
        }
        return Some((target.as_str(), args));
    }
    let (head, args) = spine_of(expr);
    match head {
        Expr::Ident { name, .. } | Expr::UniverseApp { name, .. } => {
            Some((name.as_str(), args.into_iter().cloned().collect()))
        }
        _ => None,
    }
}

/// **逐层 delta 展开，直到头是归纳表的成员**（或展开不动了）。
///
/// 为什么需要它：集合论里 `cases` 的典型用法是 `cases h` on `h : x ∈ A ∪ B`
/// ——`∈`（`Set.mem`）与 `∪`（`Set.union`）都是 **def**，要展开**两层**才露出
/// `Or`，而 `cases` 只认归纳表里的头（实测：不展开就报「`cases` 只支持归纳类型，
/// 但头 `Set.mem` 不在归纳表里」）。
///
/// 与 [`unfold_head_once`] 的区别：那个只按**应用链**取实参，而这里 `(A ∪ B) x`
/// 的记法节点在**函数位**（实参不在 spine 里），所以走 [`head_and_args`] 把
/// 记法操作数也算上。
///
/// `limit` 次仍不是归纳 ⇒ 原样返回（调用方照旧报「不在归纳表里」）。
pub(crate) fn unfold_to_inductive(
    ty: &Expr,
    is_inductive: &dyn Fn(&str) -> bool,
    defs: &crate::compile::elab::DefTable,
    limit: usize,
    level_hint: Option<&str>,
) -> Expr {
    let mut cur = ty.clone();
    for _ in 0..limit {
        if let Some((name, _)) = head_and_args(&cur) {
            if is_inductive(name) {
                return cur;
            }
        }
        match unfold_one(&cur, defs, level_hint) {
            Some(next) if next != cur => cur = next,
            _ => return cur,
        }
    }
    cur
}

/// 展开**一层**（`None` = 展开不动）。两种形状都认：
///
/// 1. **记法在函数位**：`(A ∪ B) x` —— 记法节点是 `App` 的函数位，操作数
///    （`A`、`B`）不在 spine 里。这时先按操作数展开记法本身
///    （`Set.union α A B` → `Or (A x') (B x')`），**再把 spine 实参贴回去**
///    （`x`）。不能把 `x` 当参数喂给 `Set.union`——那会把 `A`/`B` 整体错位
///    （实测：右对齐后 `α := A`，展开出一个胡说八道的类型）。
/// 2. **普通应用链 / 记法节点本身**：交给 [`unfold_head_once`]。
pub(crate) fn unfold_one(
    expr: &Expr,
    defs: &crate::compile::elab::DefTable,
    level_hint: Option<&str>,
) -> Option<Expr> {
    let (head, spine_args) = spine_of(expr);
    // 头是**记法节点**时：操作数就是那个常量的实参（记法只写操作数，前导类型
    // 参数由 elab 期补，源 AST 里没有）；spine 上的实参是**结果**上的应用。
    let (name, const_args) = match head {
        Expr::Notation {
            target, lhs, rhs, ..
        } => (
            target.as_str(),
            lhs.iter()
                .chain(rhs.iter())
                .map(|e| (**e).clone())
                .collect::<Vec<Expr>>(),
        ),
        Expr::Ident { name, .. } | Expr::UniverseApp { name, .. } => (
            name.as_str(),
            spine_args
                .iter()
                .map(|e| (**e).clone())
                .collect::<Vec<Expr>>(),
        ),
        _ => return None,
    };
    let info = defs.get(name)?;
    // 记法路径：操作数**右对齐**到形参（前导类型参数缺着）。
    // 普通路径：实参可能比形参**多**——内核 pp 把结果上的应用也摊平了
    // （`Set.union α A B x` 四个实参、三个形参）⇒ 多出来的贴在结果上。
    let (const_args, extra) = if matches!(head, Expr::Notation { .. }) {
        (
            const_args,
            spine_args
                .iter()
                .map(|e| (**e).clone())
                .collect::<Vec<Expr>>(),
        )
    } else if const_args.len() > info.params.len() {
        let extra = const_args[info.params.len()..].to_vec();
        (const_args[..info.params.len()].to_vec(), extra)
    } else {
        (const_args, Vec::new())
    };
    if const_args.len() > info.params.len() {
        return None;
    }
    let offset = info.params.len() - const_args.len();
    let sigma: std::collections::HashMap<String, Expr> = info
        .params
        .iter()
        .skip(offset)
        .cloned()
        .zip(const_args)
        .collect();
    let levels = {
        let levels = actual_levels(head);
        if levels.is_empty() {
            resolve_levels(head, info, level_hint)
        } else {
            levels
        }
    };
    let unfolded = instantiate_universes(&info.body, &info.universes, &levels);
    let unfolded = substitute(&unfolded, &sigma);
    Some(beta_apply(unfolded, &extra))
}

/// 调用点头上的**具体层级**（`Ne.{1}` ⇒ `["1"]`；裸名 `Ne` ⇒ 空表）。
///
/// 空表由调用方用 [`resolve_levels`] 的层级提示补——源 AST 的记法节点与内核
/// pp 的裸名都不带 `.{u}`，不该在这里硬猜。
///
/// 裸名的 `"0"` 与 `elab_expr` 的 `Expr::Ident` 分支同一条默认（那边给每个宇宙
/// 参数填 `builder.zero()`）——所以这里填 `"0"` 是**忠实**的，不是猜。
fn actual_levels(head: &Expr) -> Vec<String> {
    match head {
        Expr::UniverseApp { levels, .. } => levels.clone(),
        _ => Vec::new(),
    }
}

/// 把定义体里的**宇宙变量**换成调用点的具体层级。
///
/// `Ne` 的定义体是 `Eq.{u} α a b -> False`——`u` 是 `Ne` 自己的宇宙参数。
/// delta 展开代换的是**项**参数，宇宙参数没人管，于是展开出来的 AST 里留着
/// **悬空的 `.{u}`**；这段 AST 一旦被回读（render → parse → elab）就报
/// `unknown universe level u`（实测：`{a} ≠ ∅` 的证明卡在这儿）。
///
/// `actual` 短于 `uparams` 时，多出来的按 `"0"` 补（与裸名等价）。
/// 层级里的 `+` 算术照抄（`u+1` 且 `u := 1` ⇒ `1+1`，`level_ptr` 认得）。
fn instantiate_universes(expr: &Expr, uparams: &[String], actual: &[String]) -> Expr {
    if uparams.is_empty() {
        return expr.clone();
    }
    // **只替换拿得到具体层级的那些**：裸名（`Ne`，pp 把隐式的 `{u}` 省了）拿不到
    // ⇒ 变量**原样留着**，让它在回读时**响亮地报错**（`unknown universe level u`），
    // 而不是默认成 0 —— 默认 0 会把 `Eq.{1}` 静默变成 `Eq.{0}`，那是个**更难查的
    // 错**（实测：binder 变成 `Eq.{0} (Set α) …`，内核报 `expected Sort(0),
    // actual Sort(1)`，而渲染出来两边一模一样）。
    let substitute_level = |level: &str| -> String {
        let mut out = level.to_string();
        for (i, name) in uparams.iter().enumerate() {
            if let Some(replacement) = actual.get(i) {
                out = out.replace(name.as_str(), replacement);
            }
        }
        out
    };
    fn walk(expr: &Expr, sub: &dyn Fn(&str) -> String) -> Expr {
        match expr {
            Expr::UniverseApp { name, levels, span } => Expr::UniverseApp {
                name: name.clone(),
                levels: levels.iter().map(|l| sub(l)).collect(),
                span: *span,
            },
            Expr::App { fun, arg, span, .. } => Expr::App {
                fun: Box::new(walk(fun, sub)),
                arg: Box::new(walk(arg, sub)),
                explicit_spine: false,
                span: *span,
            },
            Expr::Lambda {
                binders,
                body,
                span,
            } => Expr::Lambda {
                binders: binders.clone(),
                body: Box::new(walk(body, sub)),
                span: *span,
            },
            Expr::Forall {
                binders,
                body,
                span,
            } => Expr::Forall {
                binders: binders.clone(),
                body: Box::new(walk(body, sub)),
                span: *span,
            },
            Expr::Arrow {
                domain,
                codomain,
                span,
            } => Expr::Arrow {
                domain: Box::new(walk(domain, sub)),
                codomain: Box::new(walk(codomain, sub)),
                span: *span,
            },
            Expr::Notation {
                symbol,
                assoc,
                lhs,
                rhs,
                target,
                span,
                alternatives,
            } => Expr::Notation {
                symbol: symbol.clone(),
                assoc: *assoc,
                lhs: lhs.as_ref().map(|e| Box::new(walk(e, sub))),
                rhs: rhs.as_ref().map(|e| Box::new(walk(e, sub))),
                target: target.clone(),
                span: *span,
                alternatives: alternatives.clone(),
            },
            other => other.clone(),
        }
    }
    walk(expr, &substitute_level)
}

/// 把 `args` **beta-归约**进 `expr` 的 lambda 前缀：`(fun (x) => b) a` ⇒ `b[x := a]`。
/// 参数用不完 ⇒ 剩下的贴回结果上（`(fun (x) => b) a c` ⇒ `b[x := a] c`）。
///
/// **已知边界**（与引擎其余部分同级的近似）：`substitute` 不做 capture-avoiding
/// ——定义体若把形参名重新绑定，代换会串。课程库里没有这种写法（`Set.union`
/// 的 `fun (x : α) => Or (A x) (B x)` 用的是新名字 `x`，与形参 `α A B` 不撞）。
/// **beta 归约**（递归、只做 redex 消去）：`(fun (x : T) => body) a` ⇒ `body[x := a]`。
///
/// 为什么需要（R2 实测）：**内核不做 beta 转换**——`(fun (c : γ) => g (g2 c))
/// ((fun (a : α) => f2 (f a)) a)` 与原式 `g (g2 (f2 (f a)))` 在内核眼里**不是**
/// 同一个类型（报错正是"期望 `@Eq.{1} α ((fun …) ((fun …) a)) a`，实际是
/// `Eq (g (g2 (f2 (f a)))) a`"）。而 `apply` 会把交出去的**lambda 证人**代进
/// 子目标的类型里，于是子目标里全是 redex ⇒ 任何 `exact` 都失败。
///
/// 所以前端在**每次代入之后**把 redex 消掉（`substitute` 是制造 redex 的地方，
/// 见 `rename_bound_binders` 的兄弟注释）。
pub(crate) fn beta_normalize(expr: &Expr) -> Expr {
    match expr {
        Expr::App { .. } => {
            let (head, args) = spine_of(expr);
            let head = beta_normalize(head);
            if matches!(head, Expr::Lambda { .. }) {
                let args: Vec<Expr> = args.iter().map(|a| beta_normalize(a)).collect();
                return beta_normalize(&beta_apply(head, &args));
            }
            let mut out = head;
            for arg in &args {
                out = Expr::App {
                    fun: Box::new(out),
                    arg: Box::new(beta_normalize(arg)),
                    explicit_spine: false,
                    span: expr.span(),
                };
            }
            out
        }
        Expr::Arrow {
            domain,
            codomain,
            span,
        } => Expr::Arrow {
            domain: Box::new(beta_normalize(domain)),
            codomain: Box::new(beta_normalize(codomain)),
            span: *span,
        },
        Expr::Lambda {
            binders,
            body,
            span,
        } => Expr::Lambda {
            binders: binders
                .iter()
                .map(|b| Binder {
                    ty: b.ty.as_deref().map(|t| Box::new(beta_normalize(t))),
                    ..b.clone()
                })
                .collect(),
            body: Box::new(beta_normalize(body)),
            span: *span,
        },
        Expr::Forall {
            binders,
            body,
            span,
        } => Expr::Forall {
            binders: binders
                .iter()
                .map(|b| Binder {
                    ty: b.ty.as_deref().map(|t| Box::new(beta_normalize(t))),
                    ..b.clone()
                })
                .collect(),
            body: Box::new(beta_normalize(body)),
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
            lhs: lhs.as_deref().map(|e| Box::new(beta_normalize(e))),
            rhs: rhs.as_deref().map(|e| Box::new(beta_normalize(e))),
            alternatives: alternatives.clone(),
            span: *span,
        },
        Expr::Let {
            binder,
            val,
            body,
            span,
        } => Expr::Let {
            binder: binder.clone(),
            val: Box::new(beta_normalize(val)),
            body: Box::new(beta_normalize(body)),
            span: *span,
        },
        other => other.clone(),
    }
}

pub(crate) fn beta_apply(expr: Expr, args: &[Expr]) -> Expr {
    let mut cur = expr;
    let mut rest = args;
    while let Expr::Lambda { binders, body, .. } = &cur {
        if rest.is_empty() || binders.is_empty() {
            break;
        }
        let n = binders.len().min(rest.len());
        let mut sigma: std::collections::HashMap<String, Expr> = std::collections::HashMap::new();
        // **捕获避免**（R2 实测）：代入值是项、binder 名可能出现在它里面
        // ——`def powerset (α) (A) := fun (B : Set α) => subset α B A` 在
        // `(𝒫 B) A` 上把 `A := B`、再把 binder `B := A` 代进去，两次都按名字
        // 替换就把 `subset α A B` 变成 `subset α A A`（**静默错**：目标变成
        // `forall x, A x -> A x`，报错落在后面的 `exact` 上，说"期望 A x，
        // 实际是 B x"）。名字撞上就先给 binder 换一个不撞的名字。
        let mut body = body.as_ref().clone();
        for (binder, arg) in binders.iter().take(n).zip(rest.iter().take(n)) {
            let mut name = binder.name.clone();
            if !name.is_empty() && mentions(&name, arg) {
                let mut candidate = format!("{name}'");
                while mentions(&candidate, &body) || rest.iter().any(|a| mentions(&candidate, a)) {
                    candidate.push('\'');
                }
                body = rename_free(&body, &name, &candidate);
                name = candidate;
            }
            sigma.insert(name, arg.clone());
        }
        let substituted = substitute(&body, &sigma);
        let leftover = (n < binders.len()).then(|| binders[n..].to_vec());
        cur = match leftover {
            Some(binders) => Expr::Lambda {
                binders,
                body: Box::new(substituted),
                span: Span::default(),
            },
            None => substituted,
        };
        rest = &rest[n..];
    }
    let mut out = cur;
    for arg in rest {
        out = Expr::App {
            fun: Box::new(out),
            arg: Box::new(arg.clone()),
            explicit_spine: false,
            span: Span::default(),
        };
    }
    out
}

/// 把 `expr` 拆成 (头, 实参列表)。
pub(crate) fn spine_of(expr: &Expr) -> (&Expr, Vec<&Expr>) {
    let mut args = Vec::new();
    let mut cur = expr;
    while let Expr::App { fun, arg, .. } = cur {
        args.push(arg.as_ref());
        cur = fun;
    }
    args.reverse();
    (cur, args)
}

pub(crate) fn same_head(a: &Expr, b: &Expr) -> bool {
    // **只比名字，不比宇宙层级**：内核 pp 会把显式层级打掉
    // （`Eq.{1} (Set α) A B` 打成 `Eq A B`），而 `apply` 的两边一边来自
    // 内核 pp、一边来自源 AST——比层级会让最常用的 `apply Set.ext` 假失败
    // （设计 `docs/design/course-lean-style.md` L1.4 的 bug ②）。
    // 层级是否真的对得上由**内核**在最后一步判定（`judge_terms`），
    // 这里只是「能不能试着对齐」的粗筛。
    match (head_name(a), head_name(b)) {
        (Some(x), Some(y)) => x == y,
        _ => false,
    }
}

/// 「头 + 实参」的**拥有版**：记法节点算作 `target(操作数…)`。
///
/// `spine_of` 只能借用（返回 `&Expr`），而记法节点的头是它的 `target`
/// 字符串——没有现成的 `Expr` 可借，所以这里返回拥有值。只有
/// [`unify_spine`] 用它（每次 `apply` 一次，代价可忽略）。
pub(crate) fn spine_with_notation(expr: &Expr) -> (Expr, Vec<Expr>) {
    if let Expr::Notation {
        target,
        assoc,
        lhs,
        rhs,
        span,
        ..
    } = expr
    {
        let mut args: Vec<Expr> = Vec::new();
        // **binder 记法**（`∃ (x : α), p x`）的**应用形态**是
        // `Exists α (fun (x : α) => p x)`：记法只写那个 lambda，而常量的第一个
        // 参数（域 `α`）在应用里也要占位。不补这一位，`use w` 在记法目标上会
        // 右对齐错位（模板 2 个实参、目标 1 个）⇒ σ 解不出 `A` ⇒ 剩下的子目标
        // 里留着**悬空的 `A`**，报错落在后面那条 `exact` 上（实测）。
        if *assoc == crate::ast::NotationAssoc::Binder {
            if let Some(Expr::Lambda { binders, .. }) = rhs.as_deref() {
                if let Some(ty) = binders.first().and_then(|b| b.ty.as_deref()) {
                    args.push(ty.clone());
                }
            }
        }
        if let Some(l) = lhs {
            args.push((**l).clone());
        }
        if let Some(r) = rhs {
            args.push((**r).clone());
        }
        return (
            Expr::Ident {
                name: target.clone(),
                span: *span,
            },
            args,
        );
    }
    let (head, args) = spine_of(expr);
    (head.clone(), args.into_iter().cloned().collect())
}

/// 常量头的名字（`Eq` / `Eq.{1}` 都返回 `Eq`）。
fn head_name(e: &Expr) -> Option<&str> {
    match e {
        Expr::Ident { name, .. } | Expr::UniverseApp { name, .. } => Some(name.as_str()),
        _ => None,
    }
}

/// 把 `expr` 里**自由出现**的 `from` 改名为 `to`（**捕获避免**：内层同名
/// binder 挡住外层改名）。
///
/// 为什么需要（R2 实测，子代理报的 H5）：`intro` 剥一层 Pi 时用的是**用户
/// 写的名字**，而余下的体里引用的是**源/定义里的 binder 名**。`Set.subset` 的
/// 体是 `forall (x : α), A x -> B x`——学习者写 `intro y`，体里却还写着 `x`，
/// 于是 `exact h y hy` 报 `unknown identifier x`（**报错点离根因很远**：错的是
/// 目标里的名字，不是那条 `exact`）。改名后目标变成 `B y`，一切照旧。
///
/// 源级 `Forall` 目标同理（`∀ n, p n` 上 `intro m`）。
///
/// **捕获避免**：内层 binder（lambda/forall/let/模式）同名时不再往里改；
/// `to` 与内层 binder 同名这一档今天不处理（会捕获），但那种名字碰撞在课程里
/// 不存在，真撞上也是内核**响亮**报错，不会静默证错。
pub(crate) fn rename_free(expr: &Expr, from: &str, to: &str) -> Expr {
    if from == to {
        return expr.clone();
    }
    match expr {
        Expr::Ident { name, span } => {
            let name = if name == from {
                to.to_string()
            } else {
                name.clone()
            };
            Expr::Ident { name, span: *span }
        }
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
        Expr::App { fun, arg, span, .. } => Expr::App {
            fun: Box::new(rename_free(fun, from, to)),
            arg: Box::new(rename_free(arg, from, to)),
            explicit_spine: false,
            span: *span,
        },
        Expr::Plus { lhs, rhs, span } => Expr::Plus {
            lhs: Box::new(rename_free(lhs, from, to)),
            rhs: Box::new(rename_free(rhs, from, to)),
            span: *span,
        },
        Expr::Arrow {
            domain,
            codomain,
            span,
        } => Expr::Arrow {
            domain: Box::new(rename_free(domain, from, to)),
            codomain: Box::new(rename_free(codomain, from, to)),
            span: *span,
        },
        Expr::SetLiteral { elements, span } => Expr::SetLiteral {
            elements: elements.iter().map(|e| rename_free(e, from, to)).collect(),
            span: *span,
        },
        Expr::AnonCtor { elements, span } => Expr::AnonCtor {
            elements: elements.iter().map(|e| rename_free(e, from, to)).collect(),
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
            lhs: lhs.as_deref().map(|e| Box::new(rename_free(e, from, to))),
            rhs: rhs.as_deref().map(|e| Box::new(rename_free(e, from, to))),
            alternatives: alternatives.clone(),
            span: *span,
        },
        Expr::Lambda {
            binders,
            body,
            span,
        } => {
            let (binders, shadowed) = rename_binder_group(binders, from, to);
            Expr::Lambda {
                binders,
                body: Box::new(if shadowed {
                    body.as_ref().clone()
                } else {
                    rename_free(body, from, to)
                }),
                span: *span,
            }
        }
        Expr::Forall {
            binders,
            body,
            span,
        } => {
            let (binders, shadowed) = rename_binder_group(binders, from, to);
            Expr::Forall {
                binders,
                body: Box::new(if shadowed {
                    body.as_ref().clone()
                } else {
                    rename_free(body, from, to)
                }),
                span: *span,
            }
        }
        Expr::Let {
            binder,
            val,
            body,
            span,
        } => {
            let val = Box::new(rename_free(val, from, to));
            if binder.name == from {
                Expr::Let {
                    binder: binder.clone(),
                    val,
                    body: body.clone(),
                    span: *span,
                }
            } else {
                Expr::Let {
                    binder: binder.clone(),
                    val,
                    body: Box::new(rename_free(body, from, to)),
                    span: *span,
                }
            }
        }
        Expr::Match {
            scrutinee,
            arms,
            span,
        } => Expr::Match {
            scrutinee: Box::new(rename_free(scrutinee, from, to)),
            arms: arms
                .iter()
                .map(|arm| {
                    let mut binds = Vec::new();
                    collect_pattern_binds(&arm.pattern, &mut binds);
                    let shadowed = binds.iter().any(|b| b == from);
                    MatchArm {
                        pattern: arm.pattern.clone(),
                        guard: arm.guard.as_ref().map(|g| {
                            if shadowed {
                                g.clone()
                            } else {
                                rename_free(g, from, to)
                            }
                        }),
                        body: if shadowed {
                            arm.body.clone()
                        } else {
                            rename_free(&arm.body, from, to)
                        },
                        span: arm.span,
                    }
                })
                .collect(),
            span: *span,
        },
        // 记法命令 / 声明不是表达式：`Expr` 的穷尽性由编译器保证，这里只处理
        // 会出现在目标里的形状。
        other => other.clone(),
    }
}

/// 一组 binder 的改名：逐个改 binder 的类型（后一个 binder 的类型可能提到前
/// 一个），遇到与 `from` 同名的 binder 就返回 `shadowed = true`（它的体不再改）。
fn rename_binder_group(binders: &[Binder], from: &str, to: &str) -> (Vec<Binder>, bool) {
    let mut out = Vec::with_capacity(binders.len());
    let mut shadowed = false;
    for b in binders {
        let ty = b.ty.as_deref().map(|t| {
            if shadowed {
                t.clone()
            } else {
                rename_free(t, from, to)
            }
        });
        if b.name == from {
            shadowed = true;
        }
        out.push(Binder {
            name: b.name.clone(),
            ty: ty.map(Box::new),
            style: b.style.clone(),
            span: b.span,
        });
    }
    (out, shadowed)
}

/// `name` 是否出现在 `expr` 的 Ident 节点里。
pub(crate) fn mentions(name: &str, expr: &Expr) -> bool {
    match expr {
        Expr::Ident { name: n, .. } => n == name,
        Expr::UniverseApp { .. } | Expr::Sort { .. } | Expr::Num { .. } | Expr::Hole { .. } => {
            false
        }
        Expr::App { fun, arg, .. } => mentions(name, fun) || mentions(name, arg),
        Expr::SetLiteral { elements, .. } | Expr::AnonCtor { elements, .. } => {
            elements.iter().any(|e| mentions(name, e))
        }
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
        Expr::Let {
            binder, val, body, ..
        } => {
            binder.ty.as_deref().is_some_and(|t| mentions(name, t))
                || mentions(name, val)
                || mentions(name, body)
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            mentions(name, scrutinee)
                || arms.iter().any(|arm| {
                    arm.guard.as_ref().is_some_and(|g| mentions(name, g))
                        || mentions(name, &arm.body)
                })
        }
        // 记号节点（G-04 / WO-011）：符号与目标名不是标识符，只走操作数。
        Expr::Notation { lhs, rhs, .. } => {
            lhs.as_deref().is_some_and(|e| mentions(name, e))
                || rhs.as_deref().is_some_and(|e| mentions(name, e))
        }
        Expr::By { .. } => false,
    }
}

/// 位置 spine 合一：`codomain` 与 `goal` 同头（Ident/UniverseApp 相同）时
/// 逐参数位对应，给出「`layers` 里命名 binder 名 → goal 实参」的替换 σ。
///
/// 合一强度刻意只到这里（同头 + 实参个数相等），不做高阶匹配——两条 `apply`
/// 路径共用同一强度，教学子集的边界才说得清。
pub(crate) fn unify_spine(
    codomain: &Expr,
    goal: &Expr,
    layers: &[(String, Expr)],
) -> Option<std::collections::HashMap<String, Expr>> {
    // **两边都按「头 + 实参」拆，记法节点算它的目标名 + 操作数**。
    //
    // 这是 X1 的**根修**：源里写 `A ∧ B` / `A ⊆ B` 时引擎手上是
    // `Expr::Notation { target: "And", lhs: A, rhs: B }`——`spine_of` 只认
    // `App`，于是头对不上，`apply And.intro` 在 `A ∧ B` 上假报「目标不匹配」。
    // 以前靠「把目标过一遍内核 pp」绕，但那条路**有损**（pp 丢隐式实参与
    // 宇宙层级：`Eq.{1} (Set α) A B` → `Eq A B`），会把 σ 里塞进缺参数的坏
    // 类型（实测：`constructor` 之后的子目标变成 `Eq X Y -> …`，`exact` 报
    // 「期望 Sort(0)，实际是 (Set.[] $2)」）。认记法节点之后**不需要**归一化
    // 就能对齐，σ 直接来自源 AST ✓。
    let (chead, cargs) = spine_with_notation(codomain);
    let (ghead, gargs) = spine_with_notation(goal);
    if !same_head(&chead, &ghead) {
        return None;
    }
    // **实参从右侧对齐**：内核 pp 会丢掉前导的隐式实参（`Eq.{1} (Set α) A B`
    // 打成 `Eq A B`，少一个 `(Set α)`），所以两边的实参个数可能不同。
    // 从右对齐才是对的——被丢掉的永远是**前导**参数（`Eq` 的 `α`、`Or` 的
    // `A B` 之前没有别的了，但 `Set.mem` 的 `α` 就是典型）。
    // 多出来的前导实参只影响「这个实参位对应哪个类型参数」，不影响尾部对齐。
    let n = cargs.len().min(gargs.len());
    let cargs = &cargs[cargs.len() - n..];
    let gargs = &gargs[gargs.len() - n..];
    // 把 codomain 实参位与 goal 实参位对应；codomain 里的命名 binder 是
    // 类型参数，映射到 goal 实参。
    let mut sigma = std::collections::HashMap::new();
    for (name, _) in layers {
        for (carg, garg) in cargs.iter().zip(gargs.iter()) {
            if let Expr::Ident { name: n, .. } = carg {
                if n.as_str() == name.as_str() {
                    sigma.insert(name.clone(), garg.clone());
                }
            }
        }
    }
    Some(sigma)
}

/// 把 σ 代入 `expr` 里的命名 Ident（不改 span）。
/// 收集模式里绑定的变量名（`_`/字面量不绑定）。
pub(crate) fn collect_pattern_binds(pat: &crate::ast::Pattern, out: &mut Vec<String>) {
    match pat {
        crate::ast::Pattern::Wild { .. } | crate::ast::Pattern::Num { .. } => {}
        crate::ast::Pattern::Ident { name, args, .. } => {
            if args.is_empty() {
                out.push(name.clone());
            } else {
                for a in args {
                    collect_pattern_binds(a, out);
                }
            }
        }
    }
}

/// 代入前的**捕获回避**：某个 binder 的名字在任何一个代入值里自由出现时，
/// 把它换成一个不撞的名字（`B` → `B'`），并把 σ 的定义域跟着改。
///
/// 为什么需要（R2 实测，**静默错**）：`def powerset (α) (A) : Set (Set α) :=
/// fun (B : Set α) => subset α B A` 在 `(𝒫 B) A` 上展开时，σ 里
/// `A := B`（外层那个集合），而 lambda 的 binder **也叫 `B`** —— 按名字代换
/// 会把 `subset α B A` 变成 `subset α B B`，再 beta 一步就成了
/// `subset α A A`。于是 `A ∈ 𝒫 B` 的目标变成 `forall x, A x -> A x`，报错落在
/// 后面的 `exact` 上（"期望 `A x`，实际是 `B x`"），**根因离现场很远**。
///
/// 只改名字、不改语义：改名同时作用到**后续 binder 的类型**（依赖 binder）
/// 与体上。
fn rename_bound_binders(
    binders: &[Binder],
    body: &Expr,
    sigma: &std::collections::HashMap<String, Expr>,
) -> (Vec<Binder>, std::collections::HashMap<String, Expr>) {
    let mut out = Vec::with_capacity(binders.len());
    let mut sigma = sigma.clone();
    for b in binders {
        let mut name = b.name.clone();
        if !name.is_empty() && sigma.values().any(|v| mentions(&name, v)) {
            let mut candidate = format!("{name}'");
            let taken = |n: &str| {
                sigma.values().any(|v| mentions(n, v))
                    || binders.iter().any(|o| o.name == n)
                    || mentions(n, body)
            };
            while taken(&candidate) {
                candidate.push('\'');
            }
            sigma.insert(
                name.clone(),
                Expr::Ident {
                    name: candidate.clone(),
                    span: b.span,
                },
            );
            name = candidate;
        }
        // 前一个 binder 的改名要作用到这一个的类型上（依赖 binder）。
        let ty = b.ty.as_deref().map(|t| substitute(t, &sigma));
        out.push(Binder {
            name,
            ty: ty.map(Box::new),
            style: b.style.clone(),
            span: b.span,
        });
    }
    (out, sigma)
}

pub(crate) fn substitute(expr: &Expr, sigma: &std::collections::HashMap<String, Expr>) -> Expr {
    match expr {
        // 集合字面量（第三刀 §12.4）：逐元素代换。
        Expr::SetLiteral { elements, span } => Expr::SetLiteral {
            elements: elements.iter().map(|e| substitute(e, sigma)).collect(),
            span: *span,
        },
        Expr::AnonCtor { elements, span } => Expr::AnonCtor {
            elements: elements.iter().map(|e| substitute(e, sigma)).collect(),
            span: *span,
        },
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
        Expr::App { fun, arg, span, .. } => {
            let fun = substitute(fun, sigma);
            let arg = substitute(arg, sigma);
            // **代入即消 redex**：代入一个 lambda 之后立刻 beta 一步。内核不做
            // beta 转换（见 [`beta_normalize`]），留着 redex 的子目标后面每条
            // `exact` 都会假失败——而 `substitute` 正是制造 redex 的地方
            // （`apply` 把 lambda 证人代进子目标、delta 展开把操作数代进定义体）。
            if matches!(fun, Expr::Lambda { .. }) {
                return beta_normalize(&beta_apply(fun, &[arg]));
            }
            Expr::App {
                fun: Box::new(fun),
                arg: Box::new(arg),
                explicit_spine: false,
                span: *span,
            }
        }
        Expr::Lambda {
            binders,
            body,
            span,
        } => {
            let (binders, sigma) = rename_bound_binders(binders, body, sigma);
            Expr::Lambda {
                binders,
                body: Box::new(substitute(body, &sigma)),
                span: *span,
            }
        }
        Expr::Forall {
            binders,
            body,
            span,
        } => {
            let (binders, sigma) = rename_bound_binders(binders, body, sigma);
            Expr::Forall {
                binders,
                body: Box::new(substitute(body, &sigma)),
                span: *span,
            }
        }
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
        Expr::Let {
            binder,
            val,
            body,
            span,
        } => {
            let val = substitute(val, sigma);
            let (binders, sigma) = rename_bound_binders(std::slice::from_ref(binder), body, sigma);
            Expr::Let {
                binder: binders[0].clone(),
                val: Box::new(val),
                body: Box::new(substitute(body, &sigma)),
                span: *span,
            }
        }
        Expr::Match {
            scrutinee,
            arms,
            span,
        } => {
            // 模式的绑定变量对被匹配的 body/守卫是**阴影**：从代入里删掉它们，
            // 避免把模式变量错误替换掉（`docs/design/match-patterns.md` §5）。
            let scrutinee = Box::new(substitute(scrutinee, sigma));
            let arms = arms
                .iter()
                .map(|arm| {
                    let mut sigma = sigma.clone();
                    let mut binds = Vec::new();
                    collect_pattern_binds(&arm.pattern, &mut binds);
                    for name in &binds {
                        sigma.remove(name);
                    }
                    MatchArm {
                        pattern: arm.pattern.clone(),
                        guard: arm.guard.as_ref().map(|g| substitute(g, &sigma)),
                        body: substitute(&arm.body, &sigma),
                        span: arm.span,
                    }
                })
                .collect();
            Expr::Match {
                scrutinee,
                arms,
                span: *span,
            }
        }
        Expr::By { tactics, span } => Expr::By {
            tactics: tactics.clone(),
            span: *span,
        },
        // 记号节点（G-04 / WO-011）：只代换操作数。
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
            lhs: lhs.as_ref().map(|e| Box::new(substitute(e, sigma))),
            rhs: rhs.as_ref().map(|e| Box::new(substitute(e, sigma))),
            alternatives: alternatives.clone(),
            span: *span,
        },
    }
}

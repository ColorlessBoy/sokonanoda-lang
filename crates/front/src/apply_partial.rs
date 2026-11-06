//! `apply` 的**部分应用**（对齐 Lean 4 `MVarId.apply` 的多档实例化）。
//!
//! 现场（issue #10，`courses/set-theory/units/I.1/unit02-subsets-empty.sokonanoda`
//! 练习 3）：
//!
//! ```text
//! theorem subset_antisymm (α : Type) (A B : Set α)
//!     (h1 : A ⊆ B) (h2 : B ⊆ A) : A = B := by
//!   apply Set.ext
//!   intro x
//!   constructor
//!   apply h1        -- 目标 `x ∈ A → x ∈ B`：Lean 用 `h1 x` 直接关掉
//!   apply h2
//! ```
//!
//! **Lean 4 对照**（`~/Documents/lean/lean4/src/Lean/Meta/Tactic/Apply.lean` 的
//! `MVarId.apply`，本机 commit 与 `docs/design/metavar-pattern.md` §8 同源）：
//! `getExpectedNumArgsAux` 数出被应用项类型（`forallTelescopeReducing` 之后的）层数
//! `n`、目标类型的层数 `m`，然后 **从 `i = n - m` 开始**逐档试
//! `forallMetaTelescopeReducing eType i` + `isDefEq`。也就是说：
//!
//! * `i = n` 是今天的「**把整条望远镜剥完、结论对齐目标**」（`unify_spine` 那条路）；
//! * `i < n` 时 `eType` 还留着若干层 Pi——它要**整体**与目标同形 ⇒ 用 `f` 的前 `i`
//!   个参数（由合一赋值，剩下的变元成子目标），得到 `f a₁ … aᵢ`，它的类型正是目标。
//!
//! 本例：`h1 : A ⊆ B` 展开成 `∀ x, x ∈ A → x ∈ B`（`n = 2`），目标是 `x ∈ A → x ∈ B`
//! （`m = 1`）⇒ `i = 1`：`h1 ?x` 的类型 `?x ∈ A → ?x ∈ B` 与目标合一（`?x := x`）⇒
//! 目标用 `h1 x` 关掉、**没有**子目标。今天我们的 `apply` 只做 `i = n`，于是报
//! 「`h1` 的结果是 `x ∈ B`，无法对齐当前目标 `x ∈ A → x ∈ B`」。
//!
//! 本模块只做**匹配**（不碰节点、不碰判定）：给出 `(i, σ)`，由 `by.rs` 照既有
//! 角色规则（类型参数 / 子目标）构造 `Apply` 节点。匹配是**源级**的（与
//! `unify_spine` 同一强度），最终仍由内核终审。

use std::collections::{HashMap, HashSet};

use crate::ast::{Binder, Expr};
use crate::compile::elab::DefTable;
use crate::spine::{mentions, peel_pi, same_head, spine_with_notation, unfold_head_once};

/// **逃生门**：`SOKO_APPLY_PARTIAL=0`（或 `off`）⇒ 逐字节回到改动前（`apply` 仍只
/// 做「剥完整条望远镜、结论对整目标」一档）。判据的反向臂用它证明本模块真的是
/// 那条修法（front/CLI 的 issue #10 夹具都会在关掉后重新报「目标不匹配」）。
pub(crate) fn apply_partial_enabled() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| !std::env::var("SOKO_APPLY_PARTIAL").is_ok_and(|v| v == "0" || v == "off"))
}

/// 尝试「部分应用」：返回 `(消费的层数 i, σ)`。
///
/// 命中条件（与 Lean 的 `i` 档一致）：
/// * 目标是**语法上的** Pi 链（`→` / `∀`），层数 `m > 0`；
/// * `layers[i..]` 恰好 `m` 层，且逐层与目标的 Π 望远镜**同形**（`σ` 里是
///   `layers[..i]` 的赋值）；
/// * 留下来的 `layers[..i]` 由调用方按既有规则填参数 / 造子目标。
///
/// `i = 0` 不在此处（那是 `exact` 的语义，调用方已经试过，且内核已经说不成立）。
pub(crate) fn match_partial(
    layers: &[(String, Expr)],
    codomain: &Expr,
    goal: &Expr,
    defs: &DefTable,
) -> Option<(usize, HashMap<String, Expr>)> {
    if !apply_partial_enabled() {
        return None;
    }
    let arity = pi_arity(goal);
    if arity == 0 || arity > layers.len() {
        return None;
    }
    // Lean 的起点 = n - m；`i = 0` 交给 `exact`（见上），所以从 max(1, n - m) 起。
    let start = (layers.len() - arity).max(1);
    for used in start..layers.len() {
        let mut matcher = Matcher {
            defs,
            vars: layers[..used]
                .iter()
                .filter(|(name, _)| !name.is_empty())
                .map(|(name, _)| name.clone())
                .collect(),
            sigma: HashMap::new(),
            bound: HashMap::new(),
            goal_bound: HashSet::new(),
            budget: 96,
        };
        if matcher.chain(&layers[used..], codomain, goal)
            && args_are_buildable(&layers[..used], &matcher.sigma)
        {
            return Some((used, matcher.sigma));
        }
    }
    None
}

/// 前 `used` 层**组装得出来**吗：σ 没填的那些层会变成子目标，它们的域必须
/// ① 不含**别的未解层名**（否则子目标类型里带着作用域外的自由名）；
/// ② 不是宇宙域（`⊢ Type` 这种子目标今天没有可靠出路 ⇒ 宁可判「这条不匹配」，
/// 回落原来的 `apply` 诊断 —— 只影响**今天本来就报错**的输入 ✓）。
fn args_are_buildable(layers: &[(String, Expr)], sigma: &HashMap<String, Expr>) -> bool {
    let unassigned: Vec<&str> = layers
        .iter()
        .map(|(name, _)| name.as_str())
        .filter(|name| !name.is_empty() && !sigma.contains_key(*name))
        .collect();
    for (name, domain) in layers {
        if !name.is_empty() && sigma.contains_key(name) {
            continue;
        }
        if crate::apply_probe::is_universe_domain(domain) {
            return false;
        }
        if unassigned
            .iter()
            .any(|other| *other != name.as_str() && mentions(other, domain))
        {
            return false;
        }
    }
    true
}

/// 目标**语法上**的 Pi 层数（`→` 与 `∀` 都算一层；def 头不算——那是 `exact` 的
/// 地形，不是本模块的地形）。
fn pi_arity(expr: &Expr) -> usize {
    let mut cur = expr.clone();
    let mut n = 0;
    while let Some(pi) = peel_pi(&cur) {
        n += 1;
        cur = pi.body;
    }
    n
}

struct Matcher<'a> {
    defs: &'a DefTable,
    /// 可以被赋值的前导层名（`layers[..used]`，去掉了匿名层）。
    vars: HashSet<String>,
    sigma: HashMap<String, Expr>,
    /// 被应用项**剩余** Π 层里的 binder 名 → 目标对应 binder 名（α 改名）。
    bound: HashMap<String, String>,
    /// 目标里已经进入作用域的 `∀` binder 名。**不许**把前导层名赋成含这些名字的项：
    /// 应用项写在外层作用域（目标 binder 不在其中）⇒ 那样的赋值会组装出越界项。
    goal_bound: HashSet<String>,
    budget: u32,
}

impl Matcher<'_> {
    /// 剩余 Pi 链 `layers` + `codomain` 与目标（一整条 Pi 链）逐层对齐。
    fn chain(&mut self, layers: &[(String, Expr)], codomain: &Expr, goal: &Expr) -> bool {
        let Some(pi) = peel_pi(goal) else {
            return layers.is_empty() && self.ty(codomain, goal);
        };
        let Some((name, domain)) = layers.first() else {
            return false;
        };
        if !self.ty(domain, &pi.domain) {
            return false;
        }
        // 进入这一层的 binder 作用域：剩余层/结论里的同名标识符改读目标的 binder 名。
        let saved_bound = self.bound.clone();
        let saved_goal = self.goal_bound.clone();
        if !name.is_empty() && !pi.name.is_empty() {
            self.bound.insert(name.clone(), pi.name.clone());
        }
        if !pi.name.is_empty() {
            self.goal_bound.insert(pi.name.clone());
        }
        let ok = self.chain(&layers[1..], codomain, &pi.body);
        self.bound = saved_bound;
        self.goal_bound = saved_goal;
        ok
    }

    /// 两个类型/项同形。
    fn ty(&mut self, pat: &Expr, tgt: &Expr) -> bool {
        if self.budget == 0 {
            return false;
        }
        self.budget -= 1;
        let pat = self.subst(pat);
        if let Expr::Ident { name, .. } = &pat {
            if self.vars.contains(name) && !self.sigma.contains_key(name) {
                return self.assign(name, tgt);
            }
        }
        match (&pat, tgt) {
            (Expr::Sort { sort: a, .. }, Expr::Sort { sort: b, .. }) => a == b,
            (Expr::Ident { name: a, .. }, Expr::Ident { name: b, .. }) => a == b,
            (Expr::UniverseApp { name: a, .. }, Expr::UniverseApp { name: b, .. }) => a == b,
            // 内核 pp 会丢 `.{…}`（`Eq.{1} …` → `Eq …`）：名字相同就算同形。
            (Expr::UniverseApp { name: a, .. }, Expr::Ident { name: b, .. })
            | (Expr::Ident { name: a, .. }, Expr::UniverseApp { name: b, .. }) => a == b,
            (Expr::Num { value: a, .. }, Expr::Num { value: b, .. }) => a == b,
            (
                Expr::Arrow { .. } | Expr::Forall { .. },
                Expr::Arrow { .. } | Expr::Forall { .. },
            ) => self.pi_pair(&pat, tgt),
            _ => {
                // 应用/记法：头同名 + 实参位逐个对齐（与 `unify_spine` 同一强度）。
                let (ph, pa) = spine_with_notation(&pat);
                let (th, ta) = spine_with_notation(tgt);
                if pa.len() == ta.len() && same_head(&ph, &th) {
                    return pa.iter().zip(ta.iter()).all(|(p, t)| self.ty(p, t));
                }
                // 两边写法只差一层定义展开（`A x` ↔ `Set.mem x A`）时再试。
                // 两边写法只差**若干层**定义展开（`A x` ↔ `Set.mem x A`）时再试：
                // 逐层展开（**同一层反复展开不是新信息**），两侧各自 ≤2 层。
                let mut cur = pat.clone();
                for _ in 0..2 {
                    let Some(next) = unfold_head_once(&cur, self.defs, None) else {
                        break;
                    };
                    if self.ty(&next, tgt) {
                        return true;
                    }
                    cur = next;
                }
                let mut cur = tgt.clone();
                for _ in 0..2 {
                    let Some(next) = unfold_head_once(&cur, self.defs, None) else {
                        break;
                    };
                    if self.ty(&pat, &next) {
                        return true;
                    }
                    cur = next;
                }
                false
            }
        }
    }

    /// 两条 Pi 链逐层对齐（`ty` 的 Pi 分支；`chain` 只处理「被应用项的剩余层是
    /// `Vec`」那一侧）。
    fn pi_pair(&mut self, pat: &Expr, tgt: &Expr) -> bool {
        match (peel_pi(pat), peel_pi(tgt)) {
            (Some(p), Some(t)) => {
                if !self.ty(&p.domain, &t.domain) {
                    return false;
                }
                let saved_bound = self.bound.clone();
                let saved_goal = self.goal_bound.clone();
                if !p.name.is_empty() && !t.name.is_empty() {
                    self.bound.insert(p.name.clone(), t.name.clone());
                }
                if !t.name.is_empty() {
                    self.goal_bound.insert(t.name.clone());
                }
                let ok = self.pi_pair(&p.body, &t.body);
                self.bound = saved_bound;
                self.goal_bound = saved_goal;
                ok
            }
            (None, None) => self.ty(pat, tgt),
            _ => false,
        }
    }

    /// 把模式侧的**已赋值**层名 / 已改名 binder 换掉（一层代换，不递归进代换物，
    /// 避免 `x := x` 这种同名赋值打转）。
    fn subst(&self, expr: &Expr) -> Expr {
        self.subst_with(expr, &[])
    }

    fn subst_with(&self, expr: &Expr, shadow: &[String]) -> Expr {
        match expr {
            Expr::Ident { name, span } => {
                if !shadow.iter().any(|s| s == name) {
                    if let Some(v) = self.sigma.get(name) {
                        return v.clone();
                    }
                    if let Some(v) = self.bound.get(name) {
                        return Expr::Ident {
                            name: v.clone(),
                            span: *span,
                        };
                    }
                }
                expr.clone()
            }
            Expr::Sort { .. }
            | Expr::UniverseApp { .. }
            | Expr::Num { .. }
            | Expr::Hole { .. }
            | Expr::By { .. } => expr.clone(),
            Expr::App {
                fun,
                arg,
                explicit_spine,
                span,
            } => Expr::App {
                fun: Box::new(self.subst_with(fun, shadow)),
                arg: Box::new(self.subst_with(arg, shadow)),
                explicit_spine: *explicit_spine,
                span: *span,
            },
            Expr::Lambda {
                binders,
                body,
                span,
            }
            | Expr::Forall {
                binders,
                body,
                span,
            } => {
                let binders = self.subst_binders(binders, shadow);
                let mut inner: Vec<String> = shadow.to_vec();
                inner.extend(binders.iter().map(|b| b.name.clone()));
                let body = self.subst_with(body, &inner);
                if matches!(expr, Expr::Lambda { .. }) {
                    Expr::Lambda {
                        binders,
                        body: Box::new(body),
                        span: *span,
                    }
                } else {
                    Expr::Forall {
                        binders,
                        body: Box::new(body),
                        span: *span,
                    }
                }
            }
            Expr::Arrow {
                domain,
                codomain,
                span,
            } => Expr::Arrow {
                domain: Box::new(self.subst_with(domain, shadow)),
                codomain: Box::new(self.subst_with(codomain, shadow)),
                span: *span,
            },
            Expr::Plus { lhs, rhs, span } => Expr::Plus {
                lhs: Box::new(self.subst_with(lhs, shadow)),
                rhs: Box::new(self.subst_with(rhs, shadow)),
                span: *span,
            },
            Expr::Let {
                binder,
                val,
                body,
                span,
            } => {
                let binder = self.subst_binder(binder, shadow);
                let val = self.subst_with(val, shadow);
                let mut inner: Vec<String> = shadow.to_vec();
                inner.push(binder.name.clone());
                Expr::Let {
                    binder,
                    val: Box::new(val),
                    body: Box::new(self.subst_with(body, &inner)),
                    span: *span,
                }
            }
            Expr::Match {
                scrutinee,
                arms,
                span,
            } => Expr::Match {
                scrutinee: Box::new(self.subst_with(scrutinee, shadow)),
                arms: arms.clone(),
                span: *span,
            },
            Expr::Notation {
                symbol,
                symbol_span,
                target,
                assoc,
                lhs,
                rhs,
                alternatives,
                span,
            } => Expr::Notation {
                symbol: symbol.clone(),
                symbol_span: *symbol_span,
                target: target.clone(),
                assoc: *assoc,
                lhs: lhs.as_deref().map(|e| Box::new(self.subst_with(e, shadow))),
                rhs: rhs.as_deref().map(|e| Box::new(self.subst_with(e, shadow))),
                alternatives: alternatives.clone(),
                span: *span,
            },
            Expr::SetLiteral { elements, span } => Expr::SetLiteral {
                elements: elements
                    .iter()
                    .map(|e| self.subst_with(e, shadow))
                    .collect(),
                span: *span,
            },
            Expr::AnonCtor { elements, span } => Expr::AnonCtor {
                elements: elements
                    .iter()
                    .map(|e| self.subst_with(e, shadow))
                    .collect(),
                span: *span,
            },
            Expr::NamedArg { name, value, span } => Expr::NamedArg {
                name: name.clone(),
                value: Box::new(self.subst_with(value, shadow)),
                span: *span,
            },
        }
    }

    fn subst_binders(&self, binders: &[Binder], shadow: &[String]) -> Vec<Binder> {
        binders
            .iter()
            .map(|b| self.subst_binder(b, shadow))
            .collect()
    }

    fn subst_binder(&self, binder: &Binder, shadow: &[String]) -> Binder {
        Binder {
            name: binder.name.clone(),
            ty: binder
                .ty
                .as_deref()
                .map(|t| Box::new(self.subst_with(t, shadow))),
            style: binder.style.clone(),
            span: binder.span,
        }
    }

    /// 给前导层名 `name` 赋值。目标侧出现的 `goal_bound` 名字**不在应用项的作用域**
    /// （它们是目标自己的 `∀` binder）⇒ 这样的解会组装出越界项，判不成立。
    fn assign(&mut self, name: &str, tgt: &Expr) -> bool {
        if !self.goal_bound.is_empty() && self.goal_bound.iter().any(|binder| mentions(binder, tgt))
        {
            return false;
        }
        // 目标侧提到**别的**未解层名 ⇒ 解里会带一个应用项作用域里的自由层名，
        // 组装出来同样是越界项（同名碰撞不在此列：那是目标自己的局部变量）。
        if let Expr::Ident { name: other, .. } = tgt {
            if other != name && self.vars.contains(other) && !self.sigma.contains_key(other) {
                return false;
            }
        }
        self.sigma.insert(name.to_string(), tgt.clone());
        true
    }
}

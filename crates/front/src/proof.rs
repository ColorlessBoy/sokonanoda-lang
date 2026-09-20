//! A tiny tactic-draft state. Tactics do not add new logic: they build the
//! very lambda expression the kernel will check.
//!
//! I9 起，`exact` / `assumption` 的匹配判定走 [`crate::judge`]（合成完整声明
//! 交完整 kernel 裁决），文本比对已删除（REQUIREMENTS §2.8）。

use crate::compile::CompileOptions;
use crate::judge::{judge_terms, GoalBinderSpec, Judgement, OpenGoalSpec};
use crate::{Binder, BinderKind, Command, Expr, SortKind, Span, Tactic};

#[derive(Debug, Clone, PartialEq)]
pub enum ProofError {
    Parse(String),
    NotABinder,
    /// `assumption` 扫描全部假设后没有 kernel 判定的匹配。
    NoAssumption,
    /// kernel 拒绝：期望与实际类型（内核渲染）。
    Mismatch {
        expected: String,
        actual: String,
    },
    /// 术语无法 elaborate（来自判定管线的稳定错误码 + 消息）。
    Judge {
        code: String,
        message: String,
    },
    NoHole,
}

impl std::fmt::Display for ProofError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProofError::Parse(msg) => write!(f, "{msg}"),
            ProofError::NotABinder => write!(f, "the current goal is not a function type"),
            ProofError::NoAssumption => write!(f, "没有假设能直接结束当前目标（kernel 判定）"),
            ProofError::Mismatch { expected, actual } => {
                write!(f, "类型不匹配：期望 `{expected}`，实际是 `{actual}`")
            }
            ProofError::Judge { message, .. } => write!(f, "{message}"),
            ProofError::NoHole => write!(f, "there is no open hole; use `done`"),
        }
    }
}

pub fn parse_expr_text(text: &str) -> Result<Expr, ProofError> {
    parse_expr_text_with(text, &[])
}

/// 带**继承记法表**的表达式文本回读（G-04 第二刀）：判卷通道（`judge.rs`）
/// 的 `by` 块目标 / 项文本是 `render_expr` 打回来的**源码级文本**，可能含
/// 记法（`a ∈ A`、`Aᶜ`）。前缀源码里已经声明过的记法必须一起喂进来，否则
/// 重解析会把符号读成未声明符号（第一刀就有的边界，第二刀顺手修掉）。
pub fn parse_expr_text_with(
    text: &str,
    inherited: &[crate::ast::NotationDecl],
) -> Result<Expr, ProofError> {
    let file = crate::parser::parse_with_inherited(&format!("#check {text}"), inherited)
        .map_err(|e| ProofError::Parse(e.message))?;
    match file.commands.into_iter().next() {
        Some(Command::Check { expr, .. }) => Ok(expr),
        _ => Err(ProofError::Parse("could not parse expression".to_string())),
    }
}

#[derive(Clone)]
pub struct ProofState {
    goal: Expr,
    goal_source: String,
    binders: Vec<Binder>,
    solution: Option<Expr>,
    /// 每个 tactic 成功执行前的完整快照；`undo` 逐步回退。
    history: Vec<ProofState>,
}

impl ProofState {
    pub fn start(goal_text: &str) -> Result<Self, ProofError> {
        let goal = parse_expr_text(goal_text)?;
        Ok(Self {
            goal,
            goal_source: goal_text.to_string(),
            binders: Vec::new(),
            solution: None,
            history: Vec::new(),
        })
    }

    pub fn goal_text(&self) -> String {
        render_expr(&self.goal)
    }

    pub fn goal_source(&self) -> &str {
        &self.goal_source
    }

    pub fn lambda_text(&self) -> String {
        let solution = self.solution.clone().unwrap_or(Expr::Hole {
            span: Span::default(),
        });
        let mut term = solution;
        for binder in self.binders.iter().rev() {
            term = Expr::Lambda {
                binders: vec![binder.clone()],
                body: Box::new(term),
                span: Span::default(),
            };
        }
        render_expr(&term)
    }

    pub fn intro(&mut self, name: &str) -> Result<(), ProofError> {
        let (binder, body) = match &self.goal {
            Expr::Forall { binders, body, .. } if !binders.is_empty() => {
                let binder = binders[0].clone();
                (binder, body.as_ref().clone())
            }
            Expr::Arrow {
                domain, codomain, ..
            } => (
                Binder {
                    name: name.to_string(),
                    ty: Some(domain.clone()),
                    style: BinderKind::Explicit,
                    span: Span::default(),
                },
                codomain.as_ref().clone(),
            ),
            _ => return Err(ProofError::NotABinder),
        };
        self.push_snapshot();
        self.binders.push(binder);
        self.goal = body;
        Ok(())
    }

    pub fn exact(&mut self, term_text: &str) -> Result<(), ProofError> {
        let term = parse_expr_text(term_text)?;
        if self.solution.is_some() {
            return Err(ProofError::NoHole);
        }
        self.push_snapshot();
        self.solution = Some(term);
        Ok(())
    }

    /// 快照只记在成功路径上（所有失败分支在此之前已返回），
    /// 因此失败的 tactic 不入栈、不改变状态。
    fn push_snapshot(&mut self) {
        let mut snapshot = self.clone();
        snapshot.history = Vec::new();
        self.history.push(snapshot);
    }

    /// 回退到上一个 tactic 之前的快照；栈空时返回 false（无可撤销的步）。
    pub fn undo(&mut self) -> bool {
        let Some(mut snapshot) = self.history.pop() else {
            return false;
        };
        snapshot.history = std::mem::take(&mut self.history);
        *self = snapshot;
        true
    }

    /// 当前 proof 状态对应的判定规格（剩余目标 + 已写 binders）。
    fn spec(&self) -> OpenGoalSpec {
        OpenGoalSpec {
            universe: Vec::new(),
            ty: self.goal_text(),
            binders: self
                .binders
                .iter()
                .map(|b| GoalBinderSpec {
                    name: b.name.clone(),
                    ty: b.ty.as_ref().map(|ty| render_expr(ty)),
                })
                .collect(),
        }
    }

    /// kernel 判定的 `exact`：术语先经完整流水线裁决，通过才填入；
    /// 失败时返回内核的"期望 / 实际"反馈（I9）。
    pub fn exact_kernel(
        &mut self,
        term_text: &str,
        prefix_src: &str,
        options: &CompileOptions,
    ) -> Result<(), ProofError> {
        let judgement = judge_terms(prefix_src, options, &self.spec(), &[term_text])
            .into_iter()
            .next()
            .expect("judge_terms returns one judgement per term");
        match judgement {
            Judgement::Match => self.exact(term_text),
            Judgement::Mismatch { expected, actual } => {
                Err(ProofError::Mismatch { expected, actual })
            }
            Judgement::Error { code, message } => Err(ProofError::Judge { code, message }),
        }
    }

    /// kernel 判定的 `assumption`：从最内层 binder 起逐个让完整 kernel 裁决，
    /// 第一个通过者填入；全部失败返回 [`ProofError::NoAssumption`]。
    /// 旧的文本比对实现已删除（REQUIREMENTS §2.8）。
    pub fn assumption_kernel(
        &mut self,
        prefix_src: &str,
        options: &CompileOptions,
    ) -> Result<(), ProofError> {
        let names: Vec<String> = self.binders.iter().rev().map(|b| b.name.clone()).collect();
        let refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
        let judgements = judge_terms(prefix_src, options, &self.spec(), &refs);
        let matched = judgements
            .iter()
            .position(|j| matches!(j, Judgement::Match))
            .map(|i| names[i].clone());
        match matched {
            Some(name) => self.exact(&name),
            None => Err(ProofError::NoAssumption),
        }
    }

    pub fn done(&self) -> bool {
        self.solution.is_some()
    }
}

pub fn render_expr(expr: &Expr) -> String {
    match expr {
        Expr::Sort { sort, .. } => match sort {
            SortKind::Prop => "Prop".to_string(),
            SortKind::Type => "Type".to_string(),
            SortKind::Sort(n) => format!("Sort {n}"),
            // 层级算术文本（`u+1`）加括号：`Sort (u+1)` 与 `Sort u+1` 都能回读
            // （parser 的 `parse_level_text`），但带括号的形态在 render→parse
            // 往返里没有歧义（设计 `docs/design/type-level-syntax.md` §5）。
            SortKind::Level(name) if name.contains('+') => format!("Sort ({name})"),
            SortKind::Level(name) => format!("Sort {name}"),
        },
        Expr::Ident { name, .. } => name.clone(),
        Expr::UniverseApp { name, levels, .. } => {
            // **不再补 `@`**（IA-1）：`@` 以前只是"把实参写显式"的提示，现在是
            // **真语义**（关闭隐式实参插入）——渲染时凭空补一个 `@`，判卷通道
            // 打回文本再回读就会**改变含义**（`Eq.{1} β a b` 被读成 `α := β`）。
            // 用户自己写的 `@` 由 `Expr::App::explicit_spine` 负责渲染。
            format!("{name}.{{{}}}", levels.join(", "))
        }
        Expr::Num { value, .. } => value.clone(),
        Expr::Hole { .. } => "sorry".to_string(),
        Expr::App {
            fun,
            arg,
            explicit_spine,
            ..
        } => {
            if *explicit_spine {
                // Lean 的 `@`（IA-1）：整条脊**只打一个** `@`，紧贴在头前面
                // （`@f a b`）。逐节点递归渲染会打出 `@(@f a) b` —— 回读时
                // `@` 只作用于最内层，判卷器看到的就不是同一个项。
                let (head, args) = crate::spine::spine_of(expr);
                let mut out = format!("@{}", render_atom(head));
                for a in args {
                    out.push(' ');
                    out.push_str(&render_atom(a));
                }
                return out;
            }
            format!("{} {}", render_fun_position(fun), render_atom(arg))
        }
        Expr::Lambda { binders, body, .. } => {
            let prefix: Vec<_> = binders.iter().map(render_binder).collect();
            format!("fun {} => {}", prefix.join(" "), render_expr(body))
        }
        Expr::Forall { binders, body, .. } => {
            // **多 binder 组必须拆成单箭头链**（`(a : T) -> (b : T) -> …`）。
            //
            // `render_expr` 的产物是**回读通道的输入**：`judge.rs` 的判卷合成
            // （`fold_declared` / `wrap_binders`）与 `by` 引擎的目标文本都会把它
            // 重新交给 parser。而 `(a : T) (b : T) -> …`（旧写法：用空格拼前缀）
            // 在 parser 眼里第二个 binder 组后面缺 `->`——实测报
            // 「expected `->` after binder group, found LParen」。
            //
            // 内核 pp 会把相邻 binder 折叠成 `forall (a b : T), …`
            // （`docs/design/notation-subset.md` §11.9 已记这条），源里写
            // `∀ (a b : T), …` 也产出多 binder 的 `Forall`，所以这不是边角：
            // 实测两处必炸——假设类型是多 binder `∀` 时 `exact` 报
            // 「binder 缺少类型标注」，以及 `apply Or.inl` 报
            // 「无法解析 `Or.inl` 的类型」（设计
            // `docs/design/course-lean-style.md` L1.4 的 bug ①）。
            // 单 binder 时与旧写法**逐字节相同**（`(a : T) -> body`）。
            let mut text = render_expr(body);
            for binder in binders.iter().rev() {
                text = format!("{} -> {}", render_binder(binder), text);
            }
            text
        }
        Expr::Arrow {
            domain, codomain, ..
        } => {
            // domain 位置若是 Pi/箭頭/lambda 等复合式必须加括号，否则
            // `(k : Nat) -> P k -> Q` 会被右结合误读（judge_infer 的
            // render→parse 往返因此腐蚀 telescope）。
            format!(
                "{} -> {}",
                render_fun_position(domain),
                render_expr(codomain)
            )
        }
        Expr::Plus { lhs, rhs, .. } => {
            format!("{} + {}", render_expr(lhs), render_expr(rhs))
        }
        Expr::Let {
            binder, val, body, ..
        } => match binder.ty.as_deref() {
            Some(ty) => format!(
                "let {} : {} := {}; {}",
                binder.name,
                render_expr(ty),
                render_expr(val),
                render_expr(body)
            ),
            None => format!(
                "let {} := {}; {}",
                binder.name,
                render_expr(val),
                render_expr(body)
            ),
        },
        Expr::By { tactics, .. } => {
            let inner = tactics
                .iter()
                .map(render_tactic)
                .collect::<Vec<_>>()
                .join("; ");
            format!("by {inner}")
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            let rendered: Vec<String> = arms
                .iter()
                .map(|arm| {
                    let guard = arm
                        .guard
                        .as_ref()
                        .map(|g| format!(" if {}", render_expr(g)))
                        .unwrap_or_default();
                    format!(
                        "| {}{} => {}",
                        render_pattern(&arm.pattern),
                        guard,
                        render_expr(&arm.body)
                    )
                })
                .collect();
            format!(
                "match {} with {}",
                render_expr(scrutinee),
                rendered.join(" ")
            )
        }
        // 记号节点（G-04 / WO-011）：打回**记法**写法（`lhs ∈ rhs`、`𝒫 A`、
        // `Aᶜ`）。这是源级打印，与冻结内核的 pp 无关——goal/hover 里的类型文本
        // 仍由内核产出点名形式（设计 N7）。
        Expr::Notation {
            symbol,
            assoc,
            lhs,
            rhs,
            ..
        } => match (assoc, lhs, rhs) {
            (crate::ast::NotationAssoc::Prefix, _, Some(operand)) => {
                format!("{symbol} {}", render_atom(operand))
            }
            (crate::ast::NotationAssoc::Postfix, Some(operand), _) => {
                format!("{} {symbol}", render_atom(operand))
            }
            // binder 记法（第三刀 §12.1）：`∃ x, p`。操作数是一个 lambda，
            // 它的 binder 就是记法的 binder——渲染回 Lean 形状（不是点名）。
            (crate::ast::NotationAssoc::Binder, _, Some(operand)) => {
                render_binder_notation(symbol, operand)
            }
            (_, Some(lhs), Some(rhs)) => {
                format!("{} {symbol} {}", render_atom(lhs), render_atom(rhs))
            }
            _ => symbol.clone(),
        },
        // 集合字面量（第三刀 §12.4）：渲染回 `{a}` / `{a, b}`。
        Expr::SetLiteral { elements, .. } => format!(
            "{{{}}}",
            elements
                .iter()
                .map(render_expr)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        // `⟨a, b⟩`（L2.7）：回读通道必须逐字打得回来（判定合成声明会把它
        // 重新交给 parser）。
        Expr::AnonCtor { elements, .. } => format!(
            "⟨{}⟩",
            elements
                .iter()
                .map(render_expr)
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

/// 渲染 binder 记法：操作数是 `fun (x : A) => body`（两段式时 body 是
/// `And guard body`）⇒ 打回 `∃ (x : A), body` 的源级形状。
/// 打不出（操作数不是 lambda）⇒ 退回记法符号本身。
///
/// **必须带 binder 的类型标注**（课程 Lean 化实测发现，设计
/// `docs/design/course-lean-style.md` X11）：这段文本是**回读通道的输入**——
/// `judge.rs` 的判卷合成（`fold_declared` / `wrap_binders`）与 `by.rs` 的
/// 目标归一化都会把它重新交给 parser，而 binder 记法**要求**标注
/// （`docs/design/notation-subset.md` §14.1：一段式的类型只能来自标注）。
/// 打成 `∃ x, p x` 会让回读报 `elab-binder-notation-unsolved`——
/// 实测症状是「`∃` 出现在 `by` 块的目标/假设里必炸」。
///
/// 退化形状（没有标注 / 不是恰好一个 binder 组）打不出可回读的记法：
/// 退回「符号 + 名字 + 体」。它**不可回读**（binder 记法要求标注），
/// 但信息量最大，且与第三刀的行为逐字一致。
///
/// **已知边界（两段式 `∃ x ∈ s, p`）**：源 AST 里那个 binder **本来就没有
/// 标注**——它的类型是 elaborator 从 guard（`∈` 的 telescope）反解出来的
/// （`docs/design/notation-subset.md` §14.1），渲染期拿不到。所以两段式
/// `∃` 出现在 `by` 块的目标/假设里时，判卷回读仍然解不出类型。
/// **课程改写因此一律用一段式 `∃ x : α, p`**（原生 `∀ x ∈ s, p` 不受影响：
/// 它走 `Forall` 路径，binder 类型在源 AST 里就有）。
fn render_binder_notation(symbol: &str, operand: &Expr) -> String {
    let Expr::Lambda { binders, body, .. } = operand else {
        return symbol.to_string();
    };
    if let [binder] = binders.as_slice() {
        if binder.ty.is_some() {
            return format!("{symbol} {}, {}", render_binder(binder), render_expr(body));
        }
    }
    let names = binders
        .iter()
        .map(|binder| binder.name.clone())
        .collect::<Vec<_>>()
        .join(" ");
    format!("{symbol} {names}, {}", render_expr(body))
}

/// Render a `match` pattern back to teaching syntax (used by hover/error text).
///
/// **子模式只在"不是原子"时才加括号**（R2 实测的 `cases` 嵌套 `by` 回归）：
/// 模式应用脊是**平的**——`intro b hb` 是"构造子 `intro` + 两个子模式"，
/// 而 `intro (b hb)` 是"构造子 `intro` + **一个**子模式（它自己又是 `b` 应用
/// `hb`）"。以前无条件加括号，于是 `cases` 在 `have … := by` 里（那条路要把
/// 组装好的项**打回源码文本**再判卷）渲染出 `| intro (b hb) =>`，回读时字段数
/// 变成 1，报「构造子 `Exists.intro` 有 2 个字段，但这一支写了 1 个子模式」。
/// 只给**本身带子模式**的子模式加括号，嵌套构造子模式才散不开。
fn render_pattern(pat: &crate::ast::Pattern) -> String {
    match pat {
        crate::ast::Pattern::Wild { .. } => "_".to_string(),
        crate::ast::Pattern::Num { value, .. } => value.clone(),
        crate::ast::Pattern::Ident { name, args, .. } => {
            if args.is_empty() {
                name.clone()
            } else {
                let inner = args
                    .iter()
                    .map(|a| match a {
                        crate::ast::Pattern::Ident { args, .. } if !args.is_empty() => {
                            format!("({})", render_pattern(a))
                        }
                        other => render_pattern(other),
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                format!("{name} {inner}")
            }
        }
    }
}

fn render_tactic(tactic: &Tactic) -> String {
    use Tactic::*;
    match tactic {
        Intro { names, .. } => format!("intro {}", names.join(" ")),
        Exact { expr, .. } => format!("exact {}", render_expr(expr)),
        Apply { expr, .. } => format!("apply {}", render_expr(expr)),
        Assumption { .. } => "assumption".to_string(),
        Rfl { .. } => "rfl".to_string(),
        Constructor { .. } => "constructor".to_string(),
        Left { .. } => "left".to_string(),
        Right { .. } => "right".to_string(),
        Use { expr, .. } => format!("use {}", render_expr(expr)),
        Exfalso { .. } => "exfalso".to_string(),
        Cases { expr, arms, .. } => {
            let mut text = format!("cases {}", render_expr(expr));
            for arm in arms {
                text.push_str(&format!(
                    "\n  | {} {} => {}",
                    arm.ctor,
                    arm.binders.join(" "),
                    arm.tactics
                        .iter()
                        .map(render_tactic)
                        .collect::<Vec<_>>()
                        .join("; ")
                ));
            }
            text
        }
        Have {
            name, ty, value, ..
        } => {
            let value = match value {
                crate::ast::HaveValue::Term(expr) => render_expr(expr),
                crate::ast::HaveValue::By(tactics) => format!(
                    "by {}",
                    tactics
                        .iter()
                        .map(render_tactic)
                        .collect::<Vec<_>>()
                        .join("; ")
                ),
            };
            format!("have {name} : {} := {value}", render_expr(ty))
        }
        Sorry { .. } => "sorry".to_string(),
    }
}

/// 应用链左结合，函数位置的 App 不需要括号（`f x y` 而非 `(f x) y`）：
/// 只有 lambda/forall/arrow/plus 这些优先级低于应用的形状才补括号。
fn render_fun_position(expr: &Expr) -> String {
    let s = render_expr(expr);
    match expr {
        Expr::Lambda { .. }
        | Expr::Forall { .. }
        | Expr::Arrow { .. }
        | Expr::Plus { .. }
        | Expr::Let { .. }
        | Expr::Match { .. }
        | Expr::Notation { .. }
        | Expr::SetLiteral { .. }
        | Expr::AnonCtor { .. } => format!("({s})"),
        _ => s,
    }
}

/// 原子位的渲染：**复合式必须补括号**。
///
/// `pub(crate)`：`by.rs` 的 `rfl` 候选文本也用它（`Eq.refl.{u} α a` 里的 `a`
/// 是原子位）——两份括号规则必须同源，否则 `Eq.refl.{1} (Set α) (Aᶜ) ∪ B`
/// 会被读成 `(Eq.refl.{1} (Set α) Aᶜ) ∪ B`（第二刀实测踩过）。
pub(crate) fn render_atom(expr: &Expr) -> String {
    let s = render_expr(expr);
    match expr {
        Expr::App { .. }
        | Expr::Lambda { .. }
        | Expr::Forall { .. }
        | Expr::Arrow { .. }
        | Expr::Plus { .. }
        | Expr::Let { .. }
        | Expr::Match { .. }
        | Expr::Notation { .. }
        | Expr::SetLiteral { .. }
        | Expr::AnonCtor { .. } => format!("({s})"),
        _ => s,
    }
}

/// binder 名防撞（外层优先保留原名）：生成骨架（失败声明重启、值位
/// `intro` 展开）时，同名或匿名层的默认名 `x` 撞上已有名字就追加序号
/// （`x` → `x2` → …）。命名约定只有一处，两个生成器共用。
pub(crate) fn fresh_name(base: &str, used: &mut std::collections::HashSet<String>) -> String {
    let mut name = base.to_string();
    let mut n = 2;
    while !used.insert(name.clone()) {
        name = format!("{base}{n}");
        n += 1;
    }
    name
}

/// 从 `ty` 最外层剥 `n` 层 Pi/Forall binder，返回剩余类型；层数不够返回
/// `None`。声明 binder 的 `intro` 递归降低与 `by` 引擎的初始上下文共用。
pub fn peel_pi_layers(ty: &Expr, n: usize) -> Option<Expr> {
    if n == 0 {
        return Some(ty.clone());
    }
    match ty {
        Expr::Forall { binders, body, .. } if !binders.is_empty() => {
            if binders.len() >= n {
                if binders.len() == n {
                    Some(body.as_ref().clone())
                } else {
                    // 同一 Forall 节点里还有剩的 binder：重包余下的（与
                    // `by::peel_pi` 的逐层剥法同构）。
                    Some(Expr::Forall {
                        binders: binders[n..].to_vec(),
                        body: body.clone(),
                        span: Span::default(),
                    })
                }
            } else {
                peel_pi_layers(body, n - binders.len())
            }
        }
        Expr::Arrow { codomain, .. } => peel_pi_layers(codomain, n - 1),
        _ => None,
    }
}

fn render_binder(binder: &Binder) -> String {
    let ty = binder
        .ty
        .as_ref()
        .map(|ty| format!(" : {}", render_expr(ty)))
        .unwrap_or_default();
    match binder.style {
        BinderKind::Explicit => format!("({}{ty})", binder.name),
        BinderKind::Implicit => format!("{{{}{ty}}}", binder.name),
    }
}

pub fn render_lambda_for_example(
    state: &ProofState,
    goal_source: &str,
    extra_decls: &str,
) -> String {
    format!(
        "{extra_decls}\nexample : {goal_source} := {}",
        state.lambda_text()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile::compile_fol;
    use crate::parse;

    #[test]
    fn intro_builds_lambda_text() {
        let mut p = ProofState::start("{a : Prop} -> a -> a").unwrap();
        p.intro("a").unwrap();
        assert_eq!(p.goal_text(), "a -> a");
        assert_eq!(p.lambda_text(), "fun {a : Prop} => sorry");
        p.intro("h").unwrap();
        assert_eq!(p.lambda_text(), "fun {a : Prop} => fun (h : a) => sorry");
    }

    #[test]
    fn exact_fills_the_hole() {
        let mut p = ProofState::start("{a : Prop} -> a -> a").unwrap();
        p.intro("a").unwrap();
        p.intro("h").unwrap();
        p.exact("h").unwrap();
        assert_eq!(p.lambda_text(), "fun {a : Prop} => fun (h : a) => h");
        assert!(p.done());
    }

    #[test]
    fn generated_lambda_passes_the_kernel() {
        let mut p = ProofState::start("{a : Prop} -> a -> a").unwrap();
        p.intro("a").unwrap();
        p.intro("h").unwrap();
        p.exact("h").unwrap();
        let source = render_lambda_for_example(&p, "{a : Prop} -> a -> a", "");
        let file = parse(&source).expect("parse generated example");
        let out = compile_fol(&file);
        assert_eq!(out.errors, vec![]);
    }

    // ---- I9：kernel 判定的 exact / assumption ----

    #[test]
    fn assumption_kernel_matches_via_kernel() {
        let mut p = ProofState::start("(a : Prop) -> a -> a").unwrap();
        p.intro("a").unwrap();
        p.intro("h").unwrap();
        p.assumption_kernel("", &CompileOptions::default()).unwrap();
        assert_eq!(p.lambda_text(), "fun (a : Prop) => fun (h : a) => h");
        assert!(p.done());
    }

    #[test]
    fn assumption_kernel_reports_no_match() {
        // intro 后剩余目标是变量 a 本身；没有假设的类型是 a（Prop ≠ a）。
        let mut p = ProofState::start("(a : Prop) -> a").unwrap();
        p.intro("a").unwrap();
        let err = p
            .assumption_kernel("", &CompileOptions::default())
            .unwrap_err();
        assert_eq!(err, ProofError::NoAssumption);
        assert!(!p.done());
    }

    #[test]
    fn exact_kernel_reports_kernel_mismatch() {
        let mut p = ProofState::start("Prop -> Prop").unwrap();
        p.intro("x").unwrap();
        let err = p
            .exact_kernel("1", "", &CompileOptions::default())
            .unwrap_err();
        assert!(
            matches!(err, ProofError::Mismatch { .. }),
            "expected mismatch, got {err:?}"
        );
        assert!(!p.done());
    }

    #[test]
    fn exact_kernel_accepts_defeq_term() {
        // 目标 Prop -> Prop，术语 fun (x : Prop) => x 经 kernel 裁决通过。
        let mut p = ProofState::start("Prop -> Prop").unwrap();
        p.intro("x").unwrap();
        p.exact_kernel("x", "", &CompileOptions::default()).unwrap();
        assert!(p.done());
    }

    // ---- undo：每个成功 tactic 前的快照可逐步回退 ----

    #[test]
    fn undo_walks_back_through_intros_to_the_initial_state() {
        let mut p = ProofState::start("(a : Prop) -> a -> a").unwrap();
        let initial_goal = p.goal_text();
        let initial_lambda = p.lambda_text();
        let initial_binders = p.binders.clone();
        // 空栈：没有可撤销的步。
        assert!(!p.undo());
        p.intro("a").unwrap();
        p.intro("h").unwrap();
        assert_eq!(p.goal_text(), "a");
        assert!(p.undo());
        assert_eq!(p.goal_text(), "a -> a");
        assert_eq!(p.lambda_text(), "fun (a : Prop) => sorry");
        assert_eq!(p.binders.len(), 1);
        assert!(p.undo());
        assert_eq!(p.goal_text(), initial_goal);
        assert_eq!(p.lambda_text(), initial_lambda);
        assert_eq!(p.binders, initial_binders);
        // 第三次：栈已空。
        assert!(!p.undo());
    }

    #[test]
    fn undo_after_exact_restores_the_open_hole() {
        let mut p = ProofState::start("(a : Prop) -> a -> a").unwrap();
        p.intro("a").unwrap();
        p.intro("h").unwrap();
        p.exact("h").unwrap();
        assert!(p.done());
        assert!(p.undo());
        assert!(!p.done());
        assert_eq!(p.lambda_text(), "fun (a : Prop) => fun (h : a) => sorry");
        assert!(p.undo());
        assert_eq!(p.lambda_text(), "fun (a : Prop) => sorry");
        assert!(p.undo());
        assert_eq!(p.lambda_text(), "sorry");
        assert!(!p.undo());
    }

    #[test]
    fn undo_after_assumption_kernel_restores_the_open_hole() {
        let mut p = ProofState::start("(a : Prop) -> a -> a").unwrap();
        p.intro("a").unwrap();
        p.intro("h").unwrap();
        p.assumption_kernel("", &CompileOptions::default()).unwrap();
        assert!(p.done());
        assert!(p.undo());
        assert!(!p.done());
        assert_eq!(p.lambda_text(), "fun (a : Prop) => fun (h : a) => sorry");
    }

    #[test]
    fn failed_exact_kernel_leaves_no_undo_step() {
        let mut p = ProofState::start("Prop -> Prop").unwrap();
        p.intro("x").unwrap();
        assert!(p.exact_kernel("1", "", &CompileOptions::default()).is_err());
        // 失败的 exact_kernel 既不改状态也不入栈：第一次 undo 直接回到 intro 之前。
        assert!(p.undo());
        assert_eq!(p.lambda_text(), "sorry");
        assert!(!p.undo());
    }

    #[test]
    fn exact_kernel_surfaces_elab_errors() {
        let mut p = ProofState::start("Prop -> Prop").unwrap();
        p.intro("x").unwrap();
        let err = p
            .exact_kernel("nope", "", &CompileOptions::default())
            .unwrap_err();
        match &err {
            ProofError::Judge { code, .. } => assert_eq!(code, "elab-unknown-identifier"),
            other => panic!("expected judge error, got {other:?}"),
        }
        assert!(!p.done());
    }
}

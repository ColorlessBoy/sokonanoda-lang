//! A tiny tactic-draft state. Tactics do not add new logic: they build the
//! very lambda expression the kernel will check.
//!
//! I9 起，`exact` / `assumption` 的匹配判定走 [`crate::judge`]（合成完整声明
//! 交完整 kernel 裁决），文本比对已删除（REQUIREMENTS §2.8）。

use crate::compile::CompileOptions;
use crate::judge::{judge_terms, GoalBinderSpec, Judgement, OpenGoalSpec};
use crate::{parse, Binder, BinderKind, Command, Expr, SortKind, Span};

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
    let file = parse(&format!("#check {text}")).map_err(|e| ProofError::Parse(e.message))?;
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
}

impl ProofState {
    pub fn start(goal_text: &str) -> Result<Self, ProofError> {
        let goal = parse_expr_text(goal_text)?;
        Ok(Self {
            goal,
            goal_source: goal_text.to_string(),
            binders: Vec::new(),
            solution: None,
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
        self.binders.push(binder);
        self.goal = body;
        Ok(())
    }

    pub fn exact(&mut self, term_text: &str) -> Result<(), ProofError> {
        let term = parse_expr_text(term_text)?;
        if self.solution.is_some() {
            return Err(ProofError::NoHole);
        }
        self.solution = Some(term);
        Ok(())
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
            SortKind::Level(name) => format!("Sort {name}"),
        },
        Expr::Ident { name, .. } => name.clone(),
        Expr::UniverseApp { name, levels, .. } => {
            format!("@{name}.{{{}}}", levels.join(", "))
        }
        Expr::Num { value, .. } => value.clone(),
        Expr::Hole { .. } => "???".to_string(),
        Expr::App { fun, arg, .. } => {
            format!("{} {}", render_atom(fun), render_atom(arg))
        }
        Expr::Lambda { binders, body, .. } => {
            let prefix: Vec<_> = binders.iter().map(render_binder).collect();
            format!("fun {} => {}", prefix.join(" "), render_expr(body))
        }
        Expr::Forall { binders, body, .. } => {
            let prefix: Vec<_> = binders.iter().map(render_binder).collect();
            format!("{} -> {}", prefix.join(" "), render_expr(body))
        }
        Expr::Arrow {
            domain, codomain, ..
        } => {
            format!("{} -> {}", render_expr(domain), render_expr(codomain))
        }
        Expr::Plus { lhs, rhs, .. } => {
            format!("{} + {}", render_expr(lhs), render_expr(rhs))
        }
    }
}

fn render_atom(expr: &Expr) -> String {
    let s = render_expr(expr);
    match expr {
        Expr::App { .. } | Expr::Lambda { .. } | Expr::Forall { .. } | Expr::Arrow { .. } => {
            format!("({s})")
        }
        _ => s,
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

    #[test]
    fn intro_builds_lambda_text() {
        let mut p = ProofState::start("{a : Prop} -> a -> a").unwrap();
        p.intro("a").unwrap();
        assert_eq!(p.goal_text(), "a -> a");
        assert_eq!(p.lambda_text(), "fun {a : Prop} => ???");
        p.intro("h").unwrap();
        assert_eq!(p.lambda_text(), "fun {a : Prop} => fun (h : a) => ???");
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

//! A tiny tactic-draft state. Tactics do not add new logic: they build the
//! very lambda expression the kernel will check.

use crate::{parse, Binder, BinderKind, Command, Expr, SortKind, Span};

#[derive(Debug, Clone, PartialEq)]
pub enum ProofError {
    Parse(String),
    NotABinder,
    NoHole,
}

impl std::fmt::Display for ProofError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProofError::Parse(msg) => write!(f, "{msg}"),
            ProofError::NotABinder => write!(f, "the current goal is not a function type"),
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

    pub fn assumption(&mut self) -> Result<(), ProofError> {
        let goal = render_expr(&self.goal);
        for binder in self.binders.iter().rev() {
            if let Some(ty) = &binder.ty {
                if render_expr(ty) == goal {
                    self.solution = Some(Expr::Ident {
                        name: binder.name.clone(),
                        span: Span::default(),
                    });
                    return Ok(());
                }
            }
        }
        Err(ProofError::NotABinder)
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
}

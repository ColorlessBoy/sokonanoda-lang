//! Compile a parsed `.sokonanoda` file into kernel declarations and run the
//! complete sokonanoda kernel over them.

use crate::{Command, Expr, FolFile, SortKind, Span};
use sokonanoda::builder::EnvBuilder;
use sokonanoda::env::{Declar, DeclarInfo, EnvLimit, ReducibilityHint};
use sokonanoda::expr::BinderStyle;
use sokonanoda::util::{Config, ExprPtr};

#[derive(Debug, Clone, PartialEq)]
pub struct CompileError {
    pub message: String,
    pub span: Span,
}

impl CompileError {
    fn new(message: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            span,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CheckEvent {
    DeclarationChecked { name: String },
    ExampleChecked,
    TypeChecked { text: String },
    Reduced { text: String },
    ExerciseOpen,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct CompileOutput {
    pub events: Vec<CheckEvent>,
    pub errors: Vec<CompileError>,
}

impl CompileOutput {
    pub fn ok(&self) -> bool {
        self.errors.is_empty()
    }
}

enum PendingOp<'a> {
    Decl {
        name: String,
        declar: Declar<'a>,
        span: Span,
    },
    Example {
        declar: Declar<'a>,
        span: Span,
    },
    Check {
        expr: ExprPtr<'a>,
        decl_before: usize,
    },
    Reduce {
        expr: ExprPtr<'a>,
        decl_before: usize,
    },
    OpenExercise,
}

/// Compile and kernel-check a whole file in one arena session.
pub fn compile_fol(file: &FolFile) -> CompileOutput {
    let arena = stumpalo::Arena::new();
    let mut builder = EnvBuilder::new(arena.as_arena_ref(), Config::default());
    let mut ops: Vec<PendingOp<'_>> = Vec::new();
    let mut example_idx = 0usize;
    let mut out = CompileOutput::default();

    for command in &file.commands {
        match command {
            Command::Def {
                name,
                ty,
                val,
                span,
            } => {
                let res = build_def(&mut builder, name, ty, val);
                match res {
                    Ok(decl) => {
                        if let Err(e) = builder.add_declar(decl.clone()) {
                            out.errors.push(CompileError::new(e, *span));
                            continue;
                        }
                        ops.push(PendingOp::Decl {
                            name: name.clone(),
                            declar: decl,
                            span: *span,
                        });
                    }
                    Err(e) => out.errors.push(e),
                }
            }
            Command::Theorem {
                name,
                ty,
                val,
                span,
            } => {
                let res = build_theorem(&mut builder, name, ty, val);
                match res {
                    Ok(decl) => {
                        if let Err(e) = builder.add_declar(decl.clone()) {
                            out.errors.push(CompileError::new(e, *span));
                            continue;
                        }
                        ops.push(PendingOp::Decl {
                            name: name.clone(),
                            declar: decl,
                            span: *span,
                        });
                    }
                    Err(e) => out.errors.push(e),
                }
            }
            Command::Axiom { name, ty, span } => {
                let res = build_axiom(&mut builder, name, ty);
                match res {
                    Ok(decl) => {
                        if let Err(e) = builder.add_declar(decl.clone()) {
                            out.errors.push(CompileError::new(e, *span));
                            continue;
                        }
                        ops.push(PendingOp::Decl {
                            name: name.clone(),
                            declar: decl,
                            span: *span,
                        });
                    }
                    Err(e) => out.errors.push(e),
                }
            }
            Command::Example { ty, val, span } => {
                if matches!(val, Expr::Hole { .. }) {
                    ops.push(PendingOp::OpenExercise);
                    continue;
                }
                example_idx += 1;
                let name = format!("_example_{example_idx}");
                match build_def(&mut builder, &name, ty, val) {
                    Ok(decl) => {
                        if let Err(e) = builder.add_declar(decl.clone()) {
                            out.errors.push(CompileError::new(e, *span));
                            continue;
                        }
                        ops.push(PendingOp::Example {
                            declar: decl,
                            span: *span,
                        });
                    }
                    Err(e) => out.errors.push(e),
                }
            }
            Command::Check { expr, span: _ } => {
                let decl_before = builder.declaration_count();
                match elab_expr(&mut builder, expr, &mut Vec::new()) {
                    Ok(e) => ops.push(PendingOp::Check {
                        expr: e,
                        decl_before,
                    }),
                    Err(e) => out.errors.push(e),
                }
            }
            Command::Reduce { expr, span: _ } => {
                let decl_before = builder.declaration_count();
                match elab_expr(&mut builder, expr, &mut Vec::new()) {
                    Ok(e) => ops.push(PendingOp::Reduce {
                        expr: e,
                        decl_before,
                    }),
                    Err(e) => out.errors.push(e),
                }
            }
        }
    }

    if !out.errors.is_empty() {
        return out;
    }

    let env = builder.finish();
    for op in ops {
        match op {
            PendingOp::Decl { name, declar, span } => match env.try_check_declar(&declar) {
                Ok(()) => out.events.push(CheckEvent::DeclarationChecked { name }),
                Err(e) => out.errors.push(CompileError::new(format!("{e}"), span)),
            },
            PendingOp::Example { declar, span } => match env.try_check_declar(&declar) {
                Ok(()) => out.events.push(CheckEvent::ExampleChecked),
                Err(e) => out.errors.push(CompileError::new(format!("{e}"), span)),
            },
            PendingOp::Check { expr, decl_before } => {
                env.with_tc(EnvLimit::ByIndex(decl_before), |tc| {
                    let ty = tc.infer_closed_type(expr);
                    let text = tc.with_pp(|pp| pp.pp_expr(ty));
                    out.events.push(CheckEvent::TypeChecked { text });
                });
            }
            PendingOp::Reduce { expr, decl_before } => {
                env.with_tc(EnvLimit::ByIndex(decl_before), |tc| {
                    let reduced = tc.reduce_closed(expr);
                    let text = tc.with_pp(|pp| pp.pp_expr(reduced));
                    out.events.push(CheckEvent::Reduced { text });
                });
            }
            PendingOp::OpenExercise => out.events.push(CheckEvent::ExerciseOpen),
        }
    }
    out
}

fn build_def<'a>(
    builder: &mut EnvBuilder<'a>,
    name: &str,
    ty: &Expr,
    val: &Expr,
) -> Result<Declar<'a>, CompileError> {
    if matches!(val, Expr::Hole { .. }) {
        return Err(CompileError::new(
            "incomplete definitions are not checked yet",
            val.span(),
        ));
    }
    let mut scope = Vec::new();
    let ty = elab_expr(builder, ty, &mut scope)?;
    let val = elab_expr(builder, val, &mut scope)?;
    let name = builder.name_from_str(name);
    let uparams = builder.alloc_levels_slice(&[]);
    Ok(Declar::Definition {
        info: DeclarInfo { name, uparams, ty },
        val,
        hint: ReducibilityHint::Regular(0),
    })
}

fn build_theorem<'a>(
    builder: &mut EnvBuilder<'a>,
    name: &str,
    ty: &Expr,
    val: &Expr,
) -> Result<Declar<'a>, CompileError> {
    if matches!(val, Expr::Hole { .. }) {
        return Err(CompileError::new(
            "incomplete theorems are not checked yet",
            val.span(),
        ));
    }
    let mut scope = Vec::new();
    let ty = elab_expr(builder, ty, &mut scope)?;
    let val = elab_expr(builder, val, &mut scope)?;
    let name = builder.name_from_str(name);
    let uparams = builder.alloc_levels_slice(&[]);
    Ok(Declar::Theorem {
        info: DeclarInfo { name, uparams, ty },
        val,
    })
}

fn build_axiom<'a>(
    builder: &mut EnvBuilder<'a>,
    name: &str,
    ty: &Expr,
) -> Result<Declar<'a>, CompileError> {
    let mut scope = Vec::new();
    let ty = elab_expr(builder, ty, &mut scope)?;
    let name = builder.name_from_str(name);
    let uparams = builder.alloc_levels_slice(&[]);
    Ok(Declar::Axiom {
        info: DeclarInfo { name, uparams, ty },
    })
}

fn elab_expr<'a>(
    builder: &mut EnvBuilder<'a>,
    expr: &Expr,
    scope: &mut Vec<String>,
) -> Result<ExprPtr<'a>, CompileError> {
    match expr {
        Expr::Sort {
            sort: SortKind::Prop,
            span: _,
        } => {
            let z = builder.zero();
            Ok(builder.mk_sort(z))
        }
        Expr::Sort {
            sort: SortKind::Type,
            span: _,
        } => {
            let z = builder.zero();
            let ty = builder.succ(z);
            Ok(builder.mk_sort(ty))
        }
        Expr::Ident { name, span } => {
            if let Some(pos) = scope.iter().rposition(|candidate| candidate == name) {
                let idx = u16::try_from(scope.len() - 1 - pos).map_err(|_| {
                    CompileError::new("too many nested binders for kernel index", *span)
                })?;
                Ok(builder.mk_var(idx))
            } else {
                let name = builder.name_from_str(name);
                let levels = builder.alloc_levels_slice(&[]);
                Ok(builder.mk_const(name, levels))
            }
        }
        Expr::Num { span, .. } => Err(CompileError::new(
            "numeric literals need the Sokonanoda prelude (next milestone)",
            *span,
        )),
        Expr::Hole { span } => Err(CompileError::new(
            "`???` is only allowed as the value of an open exercise",
            *span,
        )),
        Expr::App { fun, arg, span: _ } => {
            let fun = elab_expr(builder, fun, scope)?;
            let arg = elab_expr(builder, arg, scope)?;
            Ok(builder.mk_app(fun, arg))
        }
        Expr::Lambda {
            binders,
            body,
            span: _,
        } => {
            let base = scope.len();
            let mut names = Vec::with_capacity(binders.len());
            let mut tys = Vec::with_capacity(binders.len());
            for binder in binders {
                let ty = match &binder.ty {
                    Some(ty) => elab_expr(builder, ty, scope)?,
                    None => {
                        return Err(CompileError::new(
                            "untyped binders need elaboration inference (not in v0)",
                            binder.span,
                        ));
                    }
                };
                let name = builder.name_from_str(&binder.name);
                tys.push(ty);
                names.push(name);
                scope.push(binder.name.clone());
            }
            let mut body_expr = elab_expr(builder, body, scope)?;
            scope.truncate(base);
            for (name, ty) in names.into_iter().zip(tys).rev() {
                body_expr = builder.mk_lambda(name, BinderStyle::Default, ty, body_expr);
            }
            Ok(body_expr)
        }
        Expr::Forall {
            binders,
            body,
            span: _,
        } => {
            let base = scope.len();
            let mut names = Vec::with_capacity(binders.len());
            let mut tys = Vec::with_capacity(binders.len());
            for binder in binders {
                let ty = match &binder.ty {
                    Some(ty) => elab_expr(builder, ty, scope)?,
                    None => {
                        return Err(CompileError::new(
                            "untyped binders need elaboration inference (not in v0)",
                            binder.span,
                        ));
                    }
                };
                let name = builder.name_from_str(&binder.name);
                tys.push(ty);
                names.push(name);
                scope.push(binder.name.clone());
            }
            let mut body_expr = elab_expr(builder, body, scope)?;
            scope.truncate(base);
            for (name, ty) in names.into_iter().zip(tys).rev() {
                body_expr = builder.mk_pi(name, BinderStyle::Default, ty, body_expr);
            }
            Ok(body_expr)
        }
        Expr::Arrow {
            domain,
            codomain,
            span: _,
        } => {
            let domain = elab_expr(builder, domain, scope)?;
            // `A → B` desugars to a Pi with an anonymous binder, so free
            // variables in the codomain live one binder deeper.
            scope.push(String::new());
            let codomain = elab_expr(builder, codomain, scope)?;
            scope.pop();
            let anon = builder.anonymous();
            Ok(builder.mk_pi(anon, BinderStyle::Default, domain, codomain))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse;

    #[test]
    fn checks_a_valid_file_end_to_end() {
        let file = parse("def id : Prop → Prop := fun (x : Prop) => x\n#check id\n").unwrap();
        let out = compile_fol(&file);
        assert_eq!(out.errors, vec![]);
        assert_eq!(
            out.events,
            vec![
                CheckEvent::DeclarationChecked { name: "id".into() },
                CheckEvent::TypeChecked {
                    text: "Prop → Prop".into()
                },
            ]
        );
    }

    #[test]
    fn accepts_open_exercise() {
        let file = parse("example : Prop → Prop := ???\n").unwrap();
        let out = compile_fol(&file);
        assert_eq!(out.errors, vec![]);
        assert_eq!(out.events, vec![CheckEvent::ExerciseOpen]);
    }

    #[test]
    fn checks_dependent_forall_with_lambda() {
        let file = parse(
            "theorem t : ∀ (P : Prop), P → P :=\n\
             fun (P : Prop) (hp : P) => hp\n",
        )
        .unwrap();
        let out = compile_fol(&file);
        assert_eq!(out.errors, vec![]);
        assert_eq!(
            out.events,
            vec![CheckEvent::DeclarationChecked { name: "t".into() }]
        );
    }

    #[test]
    fn checks_axioms_and_theorems_over_axioms() {
        let file = parse(
            "axiom p : Prop\n\
             theorem t : p → p := fun (h : p) => h\n",
        )
        .unwrap();
        let out = compile_fol(&file);
        assert_eq!(out.errors, vec![]);
        assert_eq!(
            out.events,
            vec![
                CheckEvent::DeclarationChecked { name: "p".into() },
                CheckEvent::DeclarationChecked { name: "t".into() },
            ]
        );
    }

    #[test]
    fn reports_kernel_rejection_with_span() {
        let file = parse("def bad : Prop → Type := fun (x : Prop) => x\n").unwrap();
        let out = compile_fol(&file);
        assert!(!out.errors.is_empty(), "expected a kernel rejection");
        assert!(out.errors[0].span.start.line >= 1);
    }
}

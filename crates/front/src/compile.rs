//! Compile a parsed `.sokonanoda` file into kernel declarations and run the
//! complete sokonanoda kernel over them.

use crate::{BinderKind, Command, Expr, FolFile, SortKind, Span};
use sokonanoda::builder::EnvBuilder;
use sokonanoda::env::{Declar, DeclarInfo, EnvLimit, ReducibilityHint};
use sokonanoda::expr::BinderStyle;
use sokonanoda::util::{Config, ExprPtr, LevelPtr, NamePtr};
use std::collections::HashMap;
use std::sync::Arc;

type UnivMap<'a> = HashMap<String, LevelPtr<'a>>;

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
    TypeChecked { text: String, span: Span },
    Reduced { text: String, span: Span },
    Printed { name: String, text: String },
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
        span: Span,
    },
    Reduce {
        expr: ExprPtr<'a>,
        decl_before: usize,
        span: Span,
    },
    Print {
        name: String,
        ptr: NamePtr<'a>,
        span: Span,
    },
    OpenExercise,
}

/// Compile and kernel-check a whole file in one arena session.
pub fn compile_fol(file: &FolFile) -> CompileOutput {
    let arena = stumpalo::Arena::new();
    let mut builder = EnvBuilder::new(arena.as_arena_ref(), Config::default());
    install_prelude(&mut builder);
    let mut known_universes: HashMap<String, Vec<String>> = HashMap::new();
    for builtin in ["Nat", "Nat.zero", "Nat.succ", "Nat.add"] {
        known_universes.insert(builtin.to_string(), Vec::new());
    }
    let no_universe: UnivMap = UnivMap::new();
    let mut ops: Vec<PendingOp<'_>> = Vec::new();
    let mut example_idx = 0usize;
    let mut out = CompileOutput::default();

    for command in &file.commands {
        match command {
            Command::Def {
                name,
                universe,
                ty,
                val,
                span,
            } => {
                let res = build_def(&mut builder, name, universe, ty, val, &known_universes);
                match res {
                    Ok(decl) => {
                        if let Err(e) = builder.add_declar(decl.clone()) {
                            out.errors.push(CompileError::new(e, *span));
                            continue;
                        }
                        known_universes.insert(name.clone(), universe.clone());
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
                universe,
                ty,
                val,
                span,
            } => {
                let res = build_theorem(&mut builder, name, universe, ty, val, &known_universes);
                match res {
                    Ok(decl) => {
                        if let Err(e) = builder.add_declar(decl.clone()) {
                            out.errors.push(CompileError::new(e, *span));
                            continue;
                        }
                        known_universes.insert(name.clone(), universe.clone());
                        ops.push(PendingOp::Decl {
                            name: name.clone(),
                            declar: decl,
                            span: *span,
                        });
                    }
                    Err(e) => out.errors.push(e),
                }
            }
            Command::Axiom {
                name,
                universe,
                ty,
                span,
            } => {
                let res = build_axiom(&mut builder, name, universe, ty, &known_universes);
                match res {
                    Ok(decl) => {
                        if let Err(e) = builder.add_declar(decl.clone()) {
                            out.errors.push(CompileError::new(e, *span));
                            continue;
                        }
                        known_universes.insert(name.clone(), universe.clone());
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
                match build_def(&mut builder, &name, &[], ty, val, &known_universes) {
                    Ok(decl) => {
                        if let Err(e) = builder.add_declar(decl.clone()) {
                            out.errors.push(CompileError::new(e, *span));
                            continue;
                        }
                        known_universes.insert(name.clone(), Vec::new());
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
                match elab_expr(
                    &mut builder,
                    expr,
                    &mut Vec::new(),
                    &no_universe,
                    &known_universes,
                ) {
                    Ok(e) => ops.push(PendingOp::Check {
                        expr: e,
                        decl_before,
                        span: expr.span(),
                    }),
                    Err(e) => out.errors.push(e),
                }
            }
            Command::Reduce { expr, span: _ } => {
                let decl_before = builder.declaration_count();
                match elab_expr(
                    &mut builder,
                    expr,
                    &mut Vec::new(),
                    &no_universe,
                    &known_universes,
                ) {
                    Ok(e) => ops.push(PendingOp::Reduce {
                        expr: e,
                        decl_before,
                        span: expr.span(),
                    }),
                    Err(e) => out.errors.push(e),
                }
            }
            Command::Print { name, span } => {
                let ptr = builder.name_from_str(name);
                ops.push(PendingOp::Print {
                    name: name.clone(),
                    ptr,
                    span: *span,
                });
            }
        }
    }

    if !out.errors.is_empty() {
        return out;
    }

    let mut env = builder.finish();
    // Print proof terms as terms instead of suppressing them to `_`; the
    // suppression path would try to infer types of open binder bodies.
    env.config.pp_options.proofs = true;
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
            PendingOp::Check {
                expr,
                decl_before,
                span,
            } => {
                env.with_tc(EnvLimit::ByIndex(decl_before), |tc| {
                    let ty = tc.infer_closed_type(expr);
                    let text = tc.with_pp(|pp| pp.pp_expr(ty));
                    out.events.push(CheckEvent::TypeChecked { text, span });
                });
            }
            PendingOp::Reduce {
                expr,
                decl_before,
                span,
            } => {
                env.with_tc(EnvLimit::ByIndex(decl_before), |tc| {
                    let reduced = tc.reduce_closed(expr);
                    let text = tc.with_pp(|pp| pp.pp_expr(reduced));
                    out.events.push(CheckEvent::Reduced { text, span });
                });
            }
            PendingOp::Print { name, ptr, span } => {
                let printed = env.with_pp(|pp| pp.pp_declar(ptr));
                match printed {
                    Some(text) => out.events.push(CheckEvent::Printed { name, text }),
                    None => out.errors.push(CompileError::new(
                        format!("unknown declaration `{name}`"),
                        span,
                    )),
                }
            }
            PendingOp::OpenExercise => out.events.push(CheckEvent::ExerciseOpen),
        }
    }
    out
}

/// Trusted built-in base declarations. These are never re-checked by the
/// kernel: they are the axioms/inductive spine that the teaching grammar is
/// built on. The kernel's native Nat reduction is enabled purely by the
/// matching declaration names.
fn install_prelude(builder: &mut EnvBuilder<'_>) {
    let anon = builder.anonymous();
    let empty = builder.alloc_levels_slice(&[]);
    let type_level = builder.succ(builder.zero());
    let type_sort = builder.mk_sort(type_level);

    let nat = builder.name_from_str("Nat");
    let nat_type = builder.mk_const(nat, empty);
    builder
        .add_inductive(
            DeclarInfo {
                name: nat,
                uparams: empty,
                ty: type_sort,
            },
            false,
            0,
            0,
            Arc::from([nat]),
            Arc::from([]),
        )
        .expect("builtin Nat already present");

    add_axiom(builder, "Nat.zero", nat_type);

    let succ_arrow = builder.mk_pi(anon, BinderStyle::Default, nat_type, nat_type);
    let succ_name = builder.name_from_str("Nat.succ");
    let succ_levels = builder.alloc_levels_slice(&[]);
    let succ_self = builder.mk_const(succ_name, succ_levels);
    add_definition(builder, "Nat.succ", succ_arrow, succ_self);

    let inner_arrow = builder.mk_pi(anon, BinderStyle::Default, nat_type, nat_type);
    let add_arrow = builder.mk_pi(anon, BinderStyle::Default, nat_type, inner_arrow);
    let add_name = builder.name_from_str("Nat.add");
    let add_levels = builder.alloc_levels_slice(&[]);
    let add_self = builder.mk_const(add_name, add_levels);
    add_definition(builder, "Nat.add", add_arrow, add_self);
}

fn add_axiom<'a>(builder: &mut EnvBuilder<'a>, name: &str, ty: ExprPtr<'a>) {
    let name = builder.name_from_str(name);
    let info = DeclarInfo {
        name,
        uparams: builder.alloc_levels_slice(&[]),
        ty,
    };
    builder
        .add_declar(Declar::Axiom { info })
        .expect("duplicate builtin axiom");
}

fn add_definition<'a>(builder: &mut EnvBuilder<'a>, name: &str, ty: ExprPtr<'a>, val: ExprPtr<'a>) {
    let name = builder.name_from_str(name);
    let info = DeclarInfo {
        name,
        uparams: builder.alloc_levels_slice(&[]),
        ty,
    };
    builder
        .add_declar(Declar::Definition {
            info,
            val,
            hint: ReducibilityHint::Regular(0),
        })
        .expect("duplicate builtin definition");
}

fn build_def<'a>(
    builder: &mut EnvBuilder<'a>,
    name: &str,
    universe: &[String],
    ty: &Expr,
    val: &Expr,
    known: &HashMap<String, Vec<String>>,
) -> Result<Declar<'a>, CompileError> {
    if matches!(val, Expr::Hole { .. }) {
        return Err(CompileError::new(
            "incomplete definitions are not checked yet",
            val.span(),
        ));
    }
    let mut scope = Vec::new();
    let univ = make_univ_map(builder, universe);
    let ty = elab_expr(builder, ty, &mut scope, &univ, known)?;
    let val = elab_expr(builder, val, &mut scope, &univ, known)?;
    let name = builder.name_from_str(name);
    let uparams = collect_uparams(builder, &univ, universe);
    Ok(Declar::Definition {
        info: DeclarInfo { name, uparams, ty },
        val,
        hint: ReducibilityHint::Regular(0),
    })
}

fn build_theorem<'a>(
    builder: &mut EnvBuilder<'a>,
    name: &str,
    universe: &[String],
    ty: &Expr,
    val: &Expr,
    known: &HashMap<String, Vec<String>>,
) -> Result<Declar<'a>, CompileError> {
    if matches!(val, Expr::Hole { .. }) {
        return Err(CompileError::new(
            "incomplete theorems are not checked yet",
            val.span(),
        ));
    }
    let mut scope = Vec::new();
    let univ = make_univ_map(builder, universe);
    let ty = elab_expr(builder, ty, &mut scope, &univ, known)?;
    let val = elab_expr(builder, val, &mut scope, &univ, known)?;
    let name = builder.name_from_str(name);
    let uparams = collect_uparams(builder, &univ, universe);
    Ok(Declar::Theorem {
        info: DeclarInfo { name, uparams, ty },
        val,
    })
}

fn build_axiom<'a>(
    builder: &mut EnvBuilder<'a>,
    name: &str,
    universe: &[String],
    ty: &Expr,
    known: &HashMap<String, Vec<String>>,
) -> Result<Declar<'a>, CompileError> {
    let mut scope = Vec::new();
    let univ = make_univ_map(builder, universe);
    let ty = elab_expr(builder, ty, &mut scope, &univ, known)?;
    let name = builder.name_from_str(name);
    let uparams = collect_uparams(builder, &univ, universe);
    Ok(Declar::Axiom {
        info: DeclarInfo { name, uparams, ty },
    })
}

fn make_univ_map<'a>(builder: &mut EnvBuilder<'a>, universe: &[String]) -> UnivMap<'a> {
    universe
        .iter()
        .map(|name| {
            let ptr = builder.name_from_str(name);
            let level = builder.level_param(ptr);
            (name.clone(), level)
        })
        .collect()
}

fn collect_uparams<'a>(
    builder: &mut EnvBuilder<'a>,
    univ: &UnivMap<'a>,
    universe: &[String],
) -> sokonanoda::util::LevelsPtr<'a> {
    let levels: Vec<LevelPtr<'a>> = universe.iter().map(|name| univ[name]).collect();
    builder.alloc_levels_slice(&levels)
}

fn level_ptr<'a>(
    builder: &mut EnvBuilder<'a>,
    level: &str,
    univ: &UnivMap<'a>,
    span: Span,
) -> Result<LevelPtr<'a>, CompileError> {
    if let Ok(n) = level.parse::<u64>() {
        let mut out = builder.zero();
        for _ in 0..n {
            out = builder.succ(out);
        }
        Ok(out)
    } else if let Some(level) = univ.get(level).copied() {
        Ok(level)
    } else {
        Err(CompileError::new(
            format!("unknown universe level `{level}`"),
            span,
        ))
    }
}

fn kernel_binder_style(kind: &BinderKind) -> BinderStyle {
    match kind {
        BinderKind::Explicit => BinderStyle::Default,
        BinderKind::Implicit => BinderStyle::Implicit,
    }
}

fn elab_expr<'a>(
    builder: &mut EnvBuilder<'a>,
    expr: &Expr,
    scope: &mut Vec<String>,
    univ: &UnivMap<'a>,
    known: &HashMap<String, Vec<String>>,
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
        Expr::Sort {
            sort: SortKind::Sort(n),
            span: _,
        } => {
            let mut level = builder.zero();
            for _ in 0..*n {
                level = builder.succ(level);
            }
            Ok(builder.mk_sort(level))
        }
        Expr::Sort {
            sort: SortKind::Level(name),
            span,
        } => {
            let level = univ.get(name).copied().ok_or_else(|| {
                CompileError::new(
                    format!("universe variable `{name}` is not declared in this declaration"),
                    *span,
                )
            })?;
            Ok(builder.mk_sort(level))
        }
        Expr::Ident { name, span } => {
            if let Some(pos) = scope.iter().rposition(|candidate| candidate == name) {
                let idx = u16::try_from(scope.len() - 1 - pos).map_err(|_| {
                    CompileError::new("too many nested binders for kernel index", *span)
                })?;
                Ok(builder.mk_var(idx))
            } else {
                let params = known.get(name).ok_or_else(|| {
                    CompileError::new(format!("unknown identifier `{name}`"), *span)
                })?;
                let levels: Vec<LevelPtr<'a>> = params.iter().map(|_| builder.zero()).collect();
                let levels = builder.alloc_levels_slice(&levels);
                let name = builder.name_from_str(name);
                Ok(builder.mk_const(name, levels))
            }
        }
        Expr::UniverseApp { name, levels, span } => {
            let params = known
                .get(name)
                .ok_or_else(|| CompileError::new(format!("unknown constant `{name}`"), *span))?;
            if params.len() != levels.len() {
                return Err(CompileError::new(
                    format!(
                        "constant `{name}` expects {} universe argument(s), got {}",
                        params.len(),
                        levels.len()
                    ),
                    *span,
                ));
            }
            let mut resolved = Vec::with_capacity(levels.len());
            for level in levels {
                resolved.push(level_ptr(builder, level, univ, *span)?);
            }
            let levels = builder.alloc_levels_slice(&resolved);
            let name = builder.name_from_str(name);
            Ok(builder.mk_const(name, levels))
        }
        Expr::Num { value, span } => {
            let n: num_bigint::BigUint = value.parse().map_err(|_| {
                CompileError::new(format!("invalid natural literal `{value}`"), *span)
            })?;
            let ptr = builder
                .alloc_bignum(n)
                .ok_or_else(|| CompileError::new("Nat literals are disabled", *span))?;
            builder
                .mk_nat_lit(ptr)
                .ok_or_else(|| CompileError::new("Nat literals are disabled", *span))
        }
        Expr::Hole { span } => Err(CompileError::new(
            "`???` is only allowed as the value of an open exercise",
            *span,
        )),
        Expr::App { fun, arg, span: _ } => {
            let fun = elab_expr(builder, fun, scope, univ, known)?;
            let arg = elab_expr(builder, arg, scope, univ, known)?;
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
            let mut styles = Vec::with_capacity(binders.len());
            for binder in binders {
                let ty = match &binder.ty {
                    Some(ty) => elab_expr(builder, ty, scope, univ, known)?,
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
                styles.push(kernel_binder_style(&binder.style));
                scope.push(binder.name.clone());
            }
            let mut body_expr = elab_expr(builder, body, scope, univ, known)?;
            scope.truncate(base);
            for ((name, ty), style) in names.into_iter().zip(tys).zip(styles).rev() {
                body_expr = builder.mk_lambda(name, style, ty, body_expr);
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
            let mut styles = Vec::with_capacity(binders.len());
            for binder in binders {
                let ty = match &binder.ty {
                    Some(ty) => elab_expr(builder, ty, scope, univ, known)?,
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
                styles.push(kernel_binder_style(&binder.style));
                scope.push(binder.name.clone());
            }
            let mut body_expr = elab_expr(builder, body, scope, univ, known)?;
            scope.truncate(base);
            for ((name, ty), style) in names.into_iter().zip(tys).zip(styles).rev() {
                body_expr = builder.mk_pi(name, style, ty, body_expr);
            }
            Ok(body_expr)
        }
        Expr::Arrow {
            domain,
            codomain,
            span: _,
        } => {
            let domain = elab_expr(builder, domain, scope, univ, known)?;
            // `A -> B` desugars to a Pi with an anonymous binder, so free
            // variables in the codomain live one binder deeper.
            scope.push(String::new());
            let codomain = elab_expr(builder, codomain, scope, univ, known)?;
            scope.pop();
            let anon = builder.anonymous();
            Ok(builder.mk_pi(anon, BinderStyle::Default, domain, codomain))
        }
        Expr::Plus { lhs, rhs, span: _ } => {
            let add = builder.name_from_str("Nat.add");
            let levels = builder.alloc_levels_slice(&[]);
            let add_const = builder.mk_const(add, levels);
            let lhs = elab_expr(builder, lhs, scope, univ, known)?;
            let rhs = elab_expr(builder, rhs, scope, univ, known)?;
            let applied = builder.mk_app(add_const, lhs);
            Ok(builder.mk_app(applied, rhs))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse;

    #[test]
    fn checks_a_valid_file_end_to_end() {
        let file = parse("def id : Prop -> Prop := fun (x : Prop) => x\n#check id\n").unwrap();
        let out = compile_fol(&file);
        assert_eq!(out.errors, vec![]);
        assert_eq!(out.events.len(), 2);
        assert!(matches!(
            &out.events[0],
            CheckEvent::DeclarationChecked { name } if name == "id"
        ));
        assert!(matches!(
            &out.events[1],
            CheckEvent::TypeChecked { text, .. } if text == "Prop -> Prop"
        ));
    }

    #[test]
    fn accepts_open_exercise() {
        let file = parse("example : Prop -> Prop := ???\n").unwrap();
        let out = compile_fol(&file);
        assert_eq!(out.errors, vec![]);
        assert_eq!(out.events, vec![CheckEvent::ExerciseOpen]);
    }

    #[test]
    fn checks_dependent_forall_with_lambda() {
        let file = parse(
            "theorem t : ∀ (P : Prop), P -> P :=\n\
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
             theorem t : p -> p := fun (h : p) => h\n",
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
        let file = parse("def bad : Prop -> Type := fun (x : Prop) => x\n").unwrap();
        let out = compile_fol(&file);
        assert!(!out.errors.is_empty(), "expected a kernel rejection");
        assert!(out.errors[0].span.start.line >= 1);
    }

    #[test]
    fn checks_nat_literals_and_reduces_addition() {
        let file = parse(
            "def two : Nat := 1 + 1\n\
             #check two\n\
             #reduce 1 + 1\n",
        )
        .unwrap();
        let out = compile_fol(&file);
        assert_eq!(out.errors, vec![]);
        assert_eq!(out.events.len(), 3);
        assert!(matches!(
            &out.events[1],
            CheckEvent::TypeChecked { text, .. } if text == "Nat"
        ));
        assert!(matches!(
            &out.events[2],
            CheckEvent::Reduced { text, .. } if text == "2"
        ));
    }

    #[test]
    fn checks_from_scratch_fol_proofs() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/fol-basics.sokonanoda"
        );
        let src = std::fs::read_to_string(path).expect("read fol-basics.sokonanoda");
        let file = parse(&src).expect("parse fol-basics.sokonanoda");
        let out = compile_fol(&file);
        assert_eq!(
            out.errors,
            vec![],
            "from-scratch FOL proofs should all check, got {:?}",
            out.errors
        );
        assert!(!out.events.is_empty());
    }

    #[test]
    fn checks_universe_levels_and_function_type_types() {
        let file = parse(
            "#check Sort 2\n\
             #check (fun (α : Sort 2) => α)\n",
        )
        .unwrap();
        let out = compile_fol(&file);
        assert_eq!(out.errors, vec![]);
        assert_eq!(out.events.len(), 2);
        assert!(matches!(
            &out.events[0],
            CheckEvent::TypeChecked { text, .. } if text == "Type 2"
        ));
        assert!(matches!(
            &out.events[1],
            CheckEvent::TypeChecked { text, .. } if text == "Type 1 -> Type 1"
        ));
    }

    #[test]
    fn prints_definitions_without_panicking_on_open_bodies() {
        let file = parse(
            "def id : Prop -> Prop := fun (x : Prop) => x\n\
             #print id\n",
        )
        .unwrap();
        let out = compile_fol(&file);
        assert_eq!(out.errors, vec![]);
        let printed = out.events.iter().find_map(|event| match event {
            CheckEvent::Printed { name, text } => Some((name.as_str(), text.as_str())),
            _ => None,
        });
        assert_eq!(
            printed,
            Some(("id", "def id : Prop -> Prop := fun (x : Prop) => x"))
        );
    }

    #[test]
    fn checks_universe_polymorphic_id_declaration() {
        let file = parse(
            "def id {u} : forall (α : Sort u), α -> α :=\n\
             fun (α : Sort u) => fun (a : α) => a\n",
        )
        .expect("parse universe-polymorphic declaration");
        let out = compile_fol(&file);
        assert_eq!(out.errors, vec![]);
    }

    #[test]
    fn checks_explicit_universe_application() {
        let file = parse(
            "def id {u} : forall (α : Sort u), α -> α :=\n\
             fun (α : Sort u) => fun (a : α) => a\n\
             def id2 {u} : forall (α : Sort u), α -> α := id.{u}\n",
        )
        .expect("parse explicit universe application");
        let out = compile_fol(&file);
        assert_eq!(out.errors, vec![]);
    }

    #[test]
    fn checks_plain_use_defaults_universe_to_zero() {
        let file = parse(
            "def id {u} : forall (α : Sort u), α -> α :=\n\
             fun (α : Sort u) => fun (a : α) => a\n\
             #check id\n",
        )
        .expect("parse plain use");
        let out = compile_fol(&file);
        assert_eq!(out.errors, vec![]);
        assert!(out
            .events
            .iter()
            .any(|event| matches!(event, CheckEvent::TypeChecked { .. })));
    }

    #[test]
    fn checks_explicit_literal_universe_application() {
        let file = parse(
            "def id {u} : forall (α : Sort u), α -> α :=\n\
             fun (α : Sort u) => fun (a : α) => a\n\
             def id0 : (α : Prop) -> α -> α :=\n\
             fun (α : Prop) => id.{0} α\n",
        )
        .expect("parse literal universe application");
        let out = compile_fol(&file);
        assert_eq!(out.errors, vec![]);
    }

    #[test]
    fn checks_axiom_with_two_universe_params() {
        let file = parse(
            "axiom cast {u, v} :\n\
             forall (α : Sort u), forall (β : Sort v), α -> β\n",
        )
        .expect("parse two universe params");
        let out = compile_fol(&file);
        assert_eq!(out.errors, vec![]);
    }

    #[test]
    fn rejects_undeclared_universe_variable() {
        let file = parse(
            "def bad : forall (α : Sort u), α -> α :=\n\
             fun (α : Sort u) => fun (a : α) => a\n",
        )
        .expect("parse undeclared universe");
        let out = compile_fol(&file);
        assert!(!out.errors.is_empty());
    }

    #[test]
    fn rejects_wrong_number_of_universe_arguments() {
        let file = parse(
            "def id {u} : forall (α : Sort u), α -> α :=\n\
             fun (α : Sort u) => fun (a : α) => a\n\
             def bad {u} : forall (α : Sort u), α -> α := id.{u, 0}\n",
        )
        .expect("parse wrong universe count");
        let out = compile_fol(&file);
        assert!(!out.errors.is_empty());
    }

    #[test]
    fn checks_at_marker_and_implicit_binders_in_py_fol_style() {
        let file = parse(
            "def id {u} : forall {α : Sort u}, forall (a : α), α :=\n\
             fun {α : Sort u} => fun (a : α) => a\n\
             def id0 : forall (α : Prop), forall (a : α), α :=\n\
             fun (α : Prop) => @id.{0} α\n",
        )
        .expect("parse @id.{0} and implicit binders");
        let out = compile_fol(&file);
        assert_eq!(out.errors, vec![]);
    }

    #[test]
    fn parses_implicit_binder_styles() {
        let file = parse("#check fun {x : Prop} => fun (y : Prop) => x\n").unwrap();
        assert_eq!(file.commands.len(), 1);
    }

    #[test]
    fn checks_ported_py_fol_core() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/py-fol-core.sokonanoda"
        );
        let src = std::fs::read_to_string(path).expect("read py-fol-core.sokonanoda");
        let file = parse(&src).expect("parse py-fol-core.sokonanoda");
        let out = compile_fol(&file);
        assert_eq!(out.errors, vec![]);
    }

    #[test]
    fn named_arrow_is_forall_with_explicit_binder() {
        let file = parse(
            "def id {u} : (α : Sort u) -> α -> α :=\n\
             fun {α : Sort u} => fun (a : α) => a\n",
        )
        .expect("parse named arrow");
        let out = compile_fol(&file);
        assert_eq!(out.errors, vec![]);
    }

    #[test]
    fn named_arrow_supports_implicit_binders() {
        let file = parse(
            "def id0 : {a : Prop} -> a -> a :=\n\
             fun {a : Prop} => fun (h : a) => h\n",
        )
        .expect("parse implicit named arrow");
        let out = compile_fol(&file);
        assert_eq!(out.errors, vec![]);
    }
}

//! AST → 内核表达式的 elaborate、声明构建（build_*）与 hover 记录。

use super::error::{CompileError, ErrorKind};
use crate::{BinderKind, CtorDecl, Expr, RecDecl, SortKind, Span};
use sokonanoda::builder::EnvBuilder;
use sokonanoda::env::{
    ConstructorData, Declar, DeclarInfo, RecRule, RecursorData, ReducibilityHint,
};
use sokonanoda::expr::BinderStyle;
use sokonanoda::util::{ExprPtr, LevelPtr, NamePtr};
use std::collections::HashMap;
use std::sync::Arc;

pub(crate) type UnivMap<'a> = HashMap<String, LevelPtr<'a>>;

pub(crate) struct ElabScope<'a> {
    names: Vec<String>,
    tys: Vec<ExprPtr<'a>>,
}

impl<'a> ElabScope<'a> {
    pub(crate) fn new() -> Self {
        Self {
            names: Vec::new(),
            tys: Vec::new(),
        }
    }
    fn len(&self) -> usize {
        self.names.len()
    }
    fn truncate(&mut self, len: usize) {
        self.names.truncate(len);
        self.tys.truncate(len);
    }
    fn push(&mut self, name: String, ty: ExprPtr<'a>) {
        self.names.push(name);
        self.tys.push(ty);
    }
}

/// One recorded sub-expression during elaboration, with the binder scope it
/// lives under (outermost first). Used to answer editor hovers.
pub(crate) struct HoverNode<'a> {
    pub(crate) span: Span,
    pub(crate) expr: ExprPtr<'a>,
    #[allow(dead_code)] // reserved for named display of open types
    pub(crate) scope_names: Vec<String>,
    pub(crate) scope_tys: Vec<ExprPtr<'a>>,
}

pub(crate) fn record_hover<'a>(
    hovers: &mut Vec<HoverNode<'a>>,
    scope: &ElabScope<'a>,
    span: Span,
    expr: ExprPtr<'a>,
) {
    hovers.push(HoverNode {
        span,
        expr,
        scope_names: scope.names.clone(),
        scope_tys: scope.tys.clone(),
    });
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn install_inductive_block<'a>(
    builder: &mut EnvBuilder<'a>,
    known: &mut HashMap<String, Vec<String>>,
    name: &str,
    ty: &Expr,
    constructors: &[CtorDecl],
    recursor: Option<&RecDecl>,
    iota_rules: &[crate::IotaRule],
    hovers: &mut Vec<HoverNode<'a>>,
) -> Result<(), CompileError> {
    let empty: UnivMap = UnivMap::new();
    let ty = elab_expr(builder, ty, &mut ElabScope::new(), &empty, known, hovers)?;
    let ind_name = builder.name_from_str(name);
    let ctor_names: Vec<NamePtr<'a>> = constructors
        .iter()
        .map(|c| builder.name_from_str(&c.name))
        .collect();
    let no_uparams = builder.alloc_levels_slice(&[]);
    builder
        .add_inductive(
            DeclarInfo {
                name: ind_name,
                uparams: no_uparams,
                ty,
            },
            true,
            0,
            0,
            Arc::from([ind_name]),
            Arc::from(ctor_names.clone()),
        )
        .map_err(|e| CompileError::elab(ErrorKind::ElabDuplicateDeclaration, e, Span::default()))?;
    known.insert(name.to_string(), Vec::new());

    for (idx, ctor) in constructors.iter().enumerate() {
        let ctor_ty = Expr::Forall {
            binders: ctor.binders.clone(),
            body: Box::new(ctor.result.clone()),
            span: ctor.span,
        };
        let ctor_ty = elab_expr(
            builder,
            &ctor_ty,
            &mut ElabScope::new(),
            &empty,
            known,
            hovers,
        )?;
        let ctor_name = ctor_names[idx];
        let no_uparams = builder.alloc_levels_slice(&[]);
        let num_fields = u16::try_from(ctor.binders.len()).map_err(|_| {
            CompileError::elab(
                ErrorKind::ElabTooManyCtorFields,
                "too many constructor fields",
                ctor.span,
            )
        })?;
        builder
            .add_declar(Declar::Constructor(ConstructorData {
                info: DeclarInfo {
                    name: ctor_name,
                    uparams: no_uparams,
                    ty: ctor_ty,
                },
                inductive_name: ind_name,
                ctor_idx: idx as u16,
                num_params: 0,
                num_fields,
            }))
            .map_err(|e| CompileError::elab(ErrorKind::ElabDuplicateDeclaration, e, ctor.span))?;
        known.insert(ctor.name.clone(), Vec::new());
    }

    if let Some(rec) = recursor {
        let univ = make_univ_map(builder, &rec.universe);
        let rec_ty = elab_expr(
            builder,
            &rec.ty,
            &mut ElabScope::new(),
            &univ,
            known,
            hovers,
        )?;
        let rec_name = builder.name_from_str(&rec.name);
        let known_rec_universes = rec.universe.clone();
        known.insert(rec.name.clone(), known_rec_universes.clone());

        let mut rules = Vec::with_capacity(iota_rules.len());
        for rule in iota_rules {
            let ctor_idx = constructors
                .iter()
                .position(|c| c.name == rule.ctor_name)
                .ok_or_else(|| {
                    CompileError::elab(
                        ErrorKind::ElabUnknownCtorForIota,
                        format!(
                            "iota rule refers to unknown constructor `{}`",
                            rule.ctor_name
                        ),
                        rule.span,
                    )
                })?;
            let ctor_name = ctor_names[ctor_idx];
            let val = elab_expr(
                builder,
                &rule.val,
                &mut ElabScope::new(),
                &univ,
                known,
                hovers,
            )?;
            rules.push(RecRule {
                ctor_name,
                ctor_telescope_size_wo_params: constructors[ctor_idx].binders.len() as u16,
                val,
            });
        }
        let info = DeclarInfo {
            name: rec_name,
            uparams: collect_uparams(builder, &univ, &known_rec_universes),
            ty: rec_ty,
        };
        builder
            .add_declar(Declar::Recursor(RecursorData {
                info,
                all_inductives: Arc::from([ind_name]),
                num_params: 0,
                num_indices: 0,
                num_motives: 1,
                num_minors: constructors.len() as u16,
                rec_rules: Arc::from(rules),
                is_k: false,
            }))
            .map_err(|e| CompileError::elab(ErrorKind::ElabDuplicateDeclaration, e, rec.span))?;
    }
    Ok(())
}

pub(crate) fn build_def<'a>(
    builder: &mut EnvBuilder<'a>,
    name: &str,
    universe: &[String],
    ty: &Expr,
    val: &Expr,
    known: &HashMap<String, Vec<String>>,
    hovers: &mut Vec<HoverNode<'a>>,
) -> Result<Declar<'a>, CompileError> {
    let mut scope = ElabScope::new();
    let univ = make_univ_map(builder, universe);
    let ty = elab_expr(builder, ty, &mut scope, &univ, known, hovers)?;
    let val = elab_expr(builder, val, &mut scope, &univ, known, hovers)?;
    let name = builder.name_from_str(name);
    let uparams = collect_uparams(builder, &univ, universe);
    Ok(Declar::Definition {
        info: DeclarInfo { name, uparams, ty },
        val,
        hint: ReducibilityHint::Regular(0),
    })
}

pub(crate) fn build_theorem<'a>(
    builder: &mut EnvBuilder<'a>,
    name: &str,
    universe: &[String],
    ty: &Expr,
    val: &Expr,
    known: &HashMap<String, Vec<String>>,
    hovers: &mut Vec<HoverNode<'a>>,
) -> Result<Declar<'a>, CompileError> {
    let mut scope = ElabScope::new();
    let univ = make_univ_map(builder, universe);
    let ty = elab_expr(builder, ty, &mut scope, &univ, known, hovers)?;
    let val = elab_expr(builder, val, &mut scope, &univ, known, hovers)?;
    let name = builder.name_from_str(name);
    let uparams = collect_uparams(builder, &univ, universe);
    Ok(Declar::Theorem {
        info: DeclarInfo { name, uparams, ty },
        val,
    })
}

pub(crate) fn build_example<'a>(
    builder: &mut EnvBuilder<'a>,
    name: &str,
    ty: &Expr,
    val: &Expr,
    known: &HashMap<String, Vec<String>>,
    hovers: &mut Vec<HoverNode<'a>>,
) -> Result<Declar<'a>, CompileError> {
    let mut scope = ElabScope::new();
    let univ = make_univ_map(builder, &[]);
    let ty = elab_expr(builder, ty, &mut scope, &univ, known, hovers)?;
    let val = elab_expr(builder, val, &mut scope, &univ, known, hovers)?;
    let name = builder.name_from_str(name);
    let uparams = builder.alloc_levels_slice(&[]);
    Ok(Declar::Definition {
        info: DeclarInfo { name, uparams, ty },
        val,
        hint: ReducibilityHint::Regular(0),
    })
}

pub(crate) fn build_axiom<'a>(
    builder: &mut EnvBuilder<'a>,
    name: &str,
    universe: &[String],
    ty: &Expr,
    known: &HashMap<String, Vec<String>>,
    hovers: &mut Vec<HoverNode<'a>>,
) -> Result<Declar<'a>, CompileError> {
    let mut scope = ElabScope::new();
    let univ = make_univ_map(builder, universe);
    let ty = elab_expr(builder, ty, &mut scope, &univ, known, hovers)?;
    let name = builder.name_from_str(name);
    let uparams = collect_uparams(builder, &univ, universe);
    Ok(Declar::Axiom {
        info: DeclarInfo { name, uparams, ty },
    })
}

pub(crate) fn make_univ_map<'a>(builder: &mut EnvBuilder<'a>, universe: &[String]) -> UnivMap<'a> {
    universe
        .iter()
        .map(|name| {
            let ptr = builder.name_from_str(name);
            let level = builder.level_param(ptr);
            (name.clone(), level)
        })
        .collect()
}

pub(crate) fn collect_uparams<'a>(
    builder: &mut EnvBuilder<'a>,
    univ: &UnivMap<'a>,
    universe: &[String],
) -> sokonanoda::util::LevelsPtr<'a> {
    let levels: Vec<LevelPtr<'a>> = universe.iter().map(|name| univ[name]).collect();
    builder.alloc_levels_slice(&levels)
}

pub(crate) fn level_ptr<'a>(
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
        Err(CompileError::elab(
            ErrorKind::ElabUnknownUniverseLevel,
            format!("unknown universe level `{level}`"),
            span,
        ))
    }
}

pub(crate) fn kernel_binder_style(kind: &BinderKind) -> BinderStyle {
    match kind {
        BinderKind::Explicit => BinderStyle::Default,
        BinderKind::Implicit => BinderStyle::Implicit,
    }
}

pub(crate) fn elab_expr<'a>(
    builder: &mut EnvBuilder<'a>,
    expr: &Expr,
    scope: &mut ElabScope<'a>,
    univ: &UnivMap<'a>,
    known: &HashMap<String, Vec<String>>,
    hovers: &mut Vec<HoverNode<'a>>,
) -> Result<ExprPtr<'a>, CompileError> {
    match expr {
        Expr::Sort {
            sort: SortKind::Prop,
            span,
        } => {
            let z = builder.zero();
            let out = builder.mk_sort(z);
            record_hover(hovers, scope, *span, out);
            Ok(out)
        }
        Expr::Sort {
            sort: SortKind::Type,
            span,
        } => {
            let z = builder.zero();
            let ty = builder.succ(z);
            let out = builder.mk_sort(ty);
            record_hover(hovers, scope, *span, out);
            Ok(out)
        }
        Expr::Sort {
            sort: SortKind::Sort(n),
            span,
        } => {
            let mut level = builder.zero();
            for _ in 0..*n {
                level = builder.succ(level);
            }
            let out = builder.mk_sort(level);
            record_hover(hovers, scope, *span, out);
            Ok(out)
        }
        Expr::Sort {
            sort: SortKind::Level(name),
            span,
        } => {
            let level = univ.get(name).copied().ok_or_else(|| {
                CompileError::elab(
                    ErrorKind::ElabUnknownUniverseLevel,
                    format!("universe variable `{name}` is not declared in this declaration"),
                    *span,
                )
            })?;
            let out = builder.mk_sort(level);
            record_hover(hovers, scope, *span, out);
            Ok(out)
        }
        Expr::Ident { name, span } => {
            let out = if let Some(pos) = scope.names.iter().rposition(|candidate| candidate == name)
            {
                let idx = u16::try_from(scope.names.len() - 1 - pos).map_err(|_| {
                    CompileError::elab(
                        ErrorKind::ElabTooManyBinders,
                        "too many nested binders for kernel index",
                        *span,
                    )
                })?;
                builder.mk_var(idx)
            } else {
                let params = known.get(name).ok_or_else(|| {
                    CompileError::elab(
                        ErrorKind::ElabUnknownIdentifier,
                        format!("unknown identifier `{name}`"),
                        *span,
                    )
                })?;
                let levels: Vec<LevelPtr<'a>> = params.iter().map(|_| builder.zero()).collect();
                let levels = builder.alloc_levels_slice(&levels);
                let name = builder.name_from_str(name);
                builder.mk_const(name, levels)
            };
            record_hover(hovers, scope, *span, out);
            Ok(out)
        }
        Expr::UniverseApp { name, levels, span } => {
            let params = known.get(name).ok_or_else(|| {
                CompileError::elab(
                    ErrorKind::ElabUnknownConstant,
                    format!("unknown constant `{name}`"),
                    *span,
                )
            })?;
            if params.len() != levels.len() {
                return Err(CompileError::elab(
                    ErrorKind::ElabUniverseArity,
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
            let out = builder.mk_const(name, levels);
            record_hover(hovers, scope, *span, out);
            Ok(out)
        }
        Expr::Num { value, span } => {
            let n: num_bigint::BigUint = value.parse().map_err(|_| {
                CompileError::elab(
                    ErrorKind::ElabInvalidNatLiteral,
                    format!("invalid natural literal `{value}`"),
                    *span,
                )
            })?;
            let ptr = builder.alloc_bignum(n).ok_or_else(|| {
                CompileError::elab(
                    ErrorKind::ElabNatLiteralDisabled,
                    "Nat literals are disabled",
                    *span,
                )
            })?;
            let out = builder.mk_nat_lit(ptr).ok_or_else(|| {
                CompileError::elab(
                    ErrorKind::ElabNatLiteralDisabled,
                    "Nat literals are disabled",
                    *span,
                )
            })?;
            record_hover(hovers, scope, *span, out);
            Ok(out)
        }
        Expr::Hole { span } => Err(CompileError::elab(
            ErrorKind::ElabHoleMisplaced,
            "`???` is only allowed as the value of an open exercise",
            *span,
        )),
        Expr::App { fun, arg, span } => {
            let fun = elab_expr(builder, fun, scope, univ, known, hovers)?;
            let arg = elab_expr(builder, arg, scope, univ, known, hovers)?;
            let out = builder.mk_app(fun, arg);
            record_hover(hovers, scope, *span, out);
            Ok(out)
        }
        Expr::Lambda {
            binders,
            body,
            span,
        } => {
            let base = scope.len();
            let mut names = Vec::with_capacity(binders.len());
            let mut tys = Vec::with_capacity(binders.len());
            let mut styles = Vec::with_capacity(binders.len());
            for binder in binders {
                let ty = match &binder.ty {
                    Some(ty) => elab_expr(builder, ty, scope, univ, known, hovers)?,
                    None => {
                        return Err(CompileError::elab(
                            ErrorKind::ElabUntypedBinder,
                            "untyped binders need elaboration inference (not in v0)",
                            binder.span,
                        ));
                    }
                };
                let name = builder.name_from_str(&binder.name);
                tys.push(ty);
                names.push(name);
                styles.push(kernel_binder_style(&binder.style));
                scope.push(binder.name.clone(), ty);
            }
            let mut body_expr = elab_expr(builder, body, scope, univ, known, hovers)?;
            scope.truncate(base);
            for ((name, ty), style) in names.into_iter().zip(tys).zip(styles).rev() {
                body_expr = builder.mk_lambda(name, style, ty, body_expr);
            }
            record_hover(hovers, scope, *span, body_expr);
            Ok(body_expr)
        }
        Expr::Forall {
            binders,
            body,
            span,
        } => {
            let base = scope.len();
            let mut names = Vec::with_capacity(binders.len());
            let mut tys = Vec::with_capacity(binders.len());
            let mut styles = Vec::with_capacity(binders.len());
            for binder in binders {
                let ty = match &binder.ty {
                    Some(ty) => elab_expr(builder, ty, scope, univ, known, hovers)?,
                    None => {
                        return Err(CompileError::elab(
                            ErrorKind::ElabUntypedBinder,
                            "untyped binders need elaboration inference (not in v0)",
                            binder.span,
                        ));
                    }
                };
                let name = builder.name_from_str(&binder.name);
                tys.push(ty);
                names.push(name);
                styles.push(kernel_binder_style(&binder.style));
                scope.push(binder.name.clone(), ty);
            }
            let mut body_expr = elab_expr(builder, body, scope, univ, known, hovers)?;
            scope.truncate(base);
            for ((name, ty), style) in names.into_iter().zip(tys).zip(styles).rev() {
                body_expr = builder.mk_pi(name, style, ty, body_expr);
            }
            record_hover(hovers, scope, *span, body_expr);
            Ok(body_expr)
        }
        Expr::Arrow {
            domain,
            codomain,
            span,
        } => {
            let domain = elab_expr(builder, domain, scope, univ, known, hovers)?;
            // `A -> B` desugars to a Pi with an anonymous binder, so free
            // variables in the codomain live one binder deeper.
            scope.push(String::new(), domain);
            let codomain = elab_expr(builder, codomain, scope, univ, known, hovers)?;
            scope.truncate(scope.len() - 1);
            let anon = builder.anonymous();
            let out = builder.mk_pi(anon, BinderStyle::Default, domain, codomain);
            record_hover(hovers, scope, *span, out);
            Ok(out)
        }
        Expr::Plus { lhs, rhs, span } => {
            let add = builder.name_from_str("Nat.add");
            let levels = builder.alloc_levels_slice(&[]);
            let add_const = builder.mk_const(add, levels);
            let lhs = elab_expr(builder, lhs, scope, univ, known, hovers)?;
            let rhs = elab_expr(builder, rhs, scope, univ, known, hovers)?;
            let applied = builder.mk_app(add_const, lhs);
            let out = builder.mk_app(applied, rhs);
            record_hover(hovers, scope, *span, out);
            Ok(out)
        }
    }
}

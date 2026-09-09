//! AST → 内核表达式的 elaborate、声明构建（build_*）与 hover 记录。

use super::error::{CompileError, ErrorKind};
use super::report::ResolvedTarget;
use crate::{Binder, BinderKind, CtorDecl, Expr, IotaRule, RecDecl, SortKind, Span};
use sokonanoda::builder::EnvBuilder;
use sokonanoda::env::{
    ConstructorData, Declar, DeclarInfo, RecRule, RecursorData, ReducibilityHint,
};
use sokonanoda::expr::BinderStyle;
use sokonanoda::util::{ExprPtr, LevelPtr, NamePtr};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

pub(crate) type UnivMap<'a> = HashMap<String, LevelPtr<'a>>;

pub(crate) struct ElabScope<'a> {
    names: Vec<String>,
    tys: Vec<ExprPtr<'a>>,
    /// Parallel to `names`: each binder's own source span, so a name use can
    /// record where its binder is defined.
    spans: Vec<Span>,
}

impl<'a> ElabScope<'a> {
    pub(crate) fn new() -> Self {
        Self {
            names: Vec::new(),
            tys: Vec::new(),
            spans: Vec::new(),
        }
    }
    fn len(&self) -> usize {
        self.names.len()
    }
    fn truncate(&mut self, len: usize) {
        self.names.truncate(len);
        self.tys.truncate(len);
        self.spans.truncate(len);
    }
    fn push(&mut self, name: String, ty: ExprPtr<'a>, span: Span) {
        self.names.push(name);
        self.tys.push(ty);
        self.spans.push(span);
    }
}

/// One recorded sub-expression during elaboration, with the binder scope it
/// lives under (outermost first). Used to answer editor hovers.
pub(crate) struct HoverNode<'a> {
    pub(crate) span: Span,
    pub(crate) expr: ExprPtr<'a>,
    pub(crate) scope_names: Vec<String>,
    pub(crate) scope_tys: Vec<ExprPtr<'a>>,
    /// When this node is an ident use point: where the name is defined.
    /// Top-level targets carry a placeholder span here and are backfilled
    /// from the file's name → def-span map in `run_pass`.
    pub(crate) resolution: Option<ResolvedTarget>,
    /// This node is a lambda/forall **binder declaration** (`name : ty`):
    /// the hover should render the declaration itself (not `expr : type`).
    pub(crate) binder: bool,
}

pub(crate) fn record_hover<'a>(
    hovers: &mut Vec<HoverNode<'a>>,
    scope: &ElabScope<'a>,
    span: Span,
    expr: ExprPtr<'a>,
    resolution: Option<ResolvedTarget>,
) {
    hovers.push(HoverNode {
        span,
        expr,
        scope_names: scope.names.clone(),
        scope_tys: scope.tys.clone(),
        resolution,
        binder: false,
    });
}

/// Record a binder-declaration hover row (`name : ty`), with the scope as it
/// was **before** this binder was pushed (the type is elaborated in that scope).
pub(crate) fn record_binder_hover<'a>(
    hovers: &mut Vec<HoverNode<'a>>,
    scope: &ElabScope<'a>,
    span: Span,
    ty: ExprPtr<'a>,
) {
    hovers.push(HoverNode {
        span,
        expr: ty,
        scope_names: scope.names.clone(),
        scope_tys: scope.tys.clone(),
        resolution: None,
        binder: true,
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
    built: &mut Vec<Declar<'a>>,
) -> Result<(), CompileError> {
    // 显式 rec 优先：源里有 rec 时零行为变化；无 rec 时自动派生等价的
    // RecDecl + iota 规则（py-nat 手写版同构），再走同一条 elab 路径。
    let owned_rec;
    let owned_rules;
    let (recursor, iota_rules): (&RecDecl, &[crate::IotaRule]) = match recursor {
        Some(rec) => (rec, iota_rules),
        None => {
            let (rec, rules) = derive_recursor(name, ty, constructors);
            owned_rec = rec;
            owned_rules = rules;
            (&owned_rec, &owned_rules)
        }
    };
    let empty: UnivMap = UnivMap::new();
    let ty = elab_expr(
        builder,
        ty,
        &mut ElabScope::new(),
        &empty,
        known,
        hovers,
        None,
    )?;
    let ind_name = builder.name_from_str(name);
    let ctor_names: Vec<NamePtr<'a>> = constructors
        .iter()
        .map(|c| builder.name_from_str(&c.name))
        .collect();
    // 内核按「构造子 binder 类型里是否提到归纳名」自算 is_recursive 并断言
    // 一致（inductive.rs::end_block）——这里从源码 AST 做同规则镜像，非递归
    // 块（Bool/Unit/Empty）才能通过声明检查。
    let is_recursive = constructors.iter().any(|ctor| {
        ctor.binders
            .iter()
            .any(|b| b.ty.as_deref().is_some_and(|ty| mentions_ident(ty, name)))
            || result_telescope_mentions(&ctor.result, name)
    });
    let no_uparams = builder.alloc_levels_slice(&[]);
    builder.begin_inductive_block();
    let ind_declar = builder
        .add_inductive(
            DeclarInfo {
                name: ind_name,
                uparams: no_uparams,
                ty,
            },
            is_recursive,
            0,
            0,
            Arc::from([ind_name]),
            Arc::from(ctor_names.clone()),
        )
        .map_err(|e| CompileError::elab(ErrorKind::ElabDuplicateDeclaration, e, Span::default()))?;
    built.push(ind_declar);
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
            None,
        )?;
        let ctor_name = ctor_names[idx];
        let no_uparams = builder.alloc_levels_slice(&[]);
        // 内核把构造子类型整体当 Pi 望远镜数字段（result 箭头链的 domain
        // 也是字段），num_fields 必须与之相等（check_declared_metadata）。
        let num_fields = u16::try_from(ctor_field_binders(ctor).len()).map_err(|_| {
            CompileError::elab(
                ErrorKind::ElabTooManyCtorFields,
                "too many constructor fields",
                ctor.span,
            )
        })?;
        let ctor_declar = Declar::Constructor(ConstructorData {
            info: DeclarInfo {
                name: ctor_name,
                uparams: no_uparams,
                ty: ctor_ty,
            },
            inductive_name: ind_name,
            ctor_idx: idx as u16,
            num_params: 0,
            num_fields,
        });
        builder
            .add_declar(ctor_declar.clone())
            .map_err(|e| CompileError::elab(ErrorKind::ElabDuplicateDeclaration, e, ctor.span))?;
        built.push(ctor_declar);
        known.insert(ctor.name.clone(), Vec::new());
    }

    {
        let rec = recursor;
        let univ = make_univ_map(builder, &rec.universe);
        let rec_ty = elab_expr(
            builder,
            &rec.ty,
            &mut ElabScope::new(),
            &univ,
            known,
            hovers,
            None,
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
                None,
            )?;
            rules.push(RecRule {
                ctor_name,
                // 与 num_fields 同规则：按整条 Pi 望远镜计（含 result 链）。
                ctor_telescope_size_wo_params: ctor_field_binders(&constructors[ctor_idx]).len()
                    as u16,
                val,
            });
        }
        let info = DeclarInfo {
            name: rec_name,
            uparams: collect_uparams(builder, &univ, &known_rec_universes),
            ty: rec_ty,
        };
        let rec_declar = Declar::Recursor(RecursorData {
            info,
            all_inductives: Arc::from([ind_name]),
            num_params: 0,
            num_indices: 0,
            num_motives: 1,
            num_minors: constructors.len() as u16,
            rec_rules: Arc::from(rules),
            is_k: false,
        });
        builder
            .add_declar(rec_declar.clone())
            .map_err(|e| CompileError::elab(ErrorKind::ElabDuplicateDeclaration, e, rec.span))?;
        built.push(rec_declar);
    }
    builder.end_inductive_block();
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
    let ty = elab_expr(builder, ty, &mut scope, &univ, known, hovers, None)?;
    let val = elab_expr(builder, val, &mut scope, &univ, known, hovers, Some(ty))?;
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
    let ty = elab_expr(builder, ty, &mut scope, &univ, known, hovers, None)?;
    let val = elab_expr(builder, val, &mut scope, &univ, known, hovers, Some(ty))?;
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
    let ty = elab_expr(builder, ty, &mut scope, &univ, known, hovers, None)?;
    let val = elab_expr(builder, val, &mut scope, &univ, known, hovers, Some(ty))?;
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
    let ty = elab_expr(builder, ty, &mut scope, &univ, known, hovers, None)?;
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

/// Peel one Pi layer off the expected type: returns the binder style, the
/// binder type and the remaining body. Used to infer untyped lambda binders
/// from the declared type of the surrounding declaration.
fn peel_expected<'a>(
    expected: Option<ExprPtr<'a>>,
) -> Option<(BinderStyle, ExprPtr<'a>, ExprPtr<'a>)> {
    match expected {
        Some(e) => match &*e {
            sokonanoda::expr::Expr::Pi {
                binder_style,
                binder_type,
                body,
                ..
            } => Some((*binder_style, *binder_type, *body)),
            _ => None,
        },
        None => None,
    }
}

/// Advance past one expected Pi layer without taking its binder (the binder
/// carries an explicit type annotation, so its kernel type comes from the
/// annotation instead).
fn drop_expected_layer(expected: Option<ExprPtr<'_>>) -> Option<ExprPtr<'_>> {
    match expected {
        Some(e) => match &*e {
            sokonanoda::expr::Expr::Pi { body, .. } => Some(*body),
            _ => None,
        },
        None => None,
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn elab_expr<'a>(
    builder: &mut EnvBuilder<'a>,
    expr: &Expr,
    scope: &mut ElabScope<'a>,
    univ: &UnivMap<'a>,
    known: &HashMap<String, Vec<String>>,
    hovers: &mut Vec<HoverNode<'a>>,
    expected: Option<ExprPtr<'a>>,
) -> Result<ExprPtr<'a>, CompileError> {
    match expr {
        Expr::Sort {
            sort: SortKind::Prop,
            span,
        } => {
            let z = builder.zero();
            let out = builder.mk_sort(z);
            record_hover(hovers, scope, *span, out, None);
            Ok(out)
        }
        Expr::Sort {
            sort: SortKind::Type,
            span,
        } => {
            let z = builder.zero();
            let ty = builder.succ(z);
            let out = builder.mk_sort(ty);
            record_hover(hovers, scope, *span, out, None);
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
            record_hover(hovers, scope, *span, out, None);
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
            record_hover(hovers, scope, *span, out, None);
            Ok(out)
        }
        Expr::Ident { name, span } => {
            let bound = scope.names.iter().rposition(|candidate| candidate == name);
            let (out, resolution) = match bound {
                Some(pos) => {
                    let idx = u16::try_from(scope.names.len() - 1 - pos).map_err(|_| {
                        CompileError::elab(
                            ErrorKind::ElabTooManyBinders,
                            "too many nested binders for kernel index",
                            *span,
                        )
                    })?;
                    (
                        builder.mk_var(idx),
                        Some(ResolvedTarget::Binder(scope.spans[pos])),
                    )
                }
                None => {
                    let params = known.get(name).ok_or_else(|| {
                        CompileError::elab(
                            ErrorKind::ElabUnknownIdentifier,
                            format!("unknown identifier `{name}`"),
                            *span,
                        )
                    })?;
                    // The defining command's span is backfilled in `run_pass`
                    // (placeholder survives until then; prelude names resolve
                    // to no source definition and drop the record there).
                    let target = ResolvedTarget::Declaration {
                        name: name.clone(),
                        span: Span::default(),
                    };
                    let levels: Vec<LevelPtr<'a>> = params.iter().map(|_| builder.zero()).collect();
                    let levels = builder.alloc_levels_slice(&levels);
                    let name = builder.name_from_str(name);
                    (builder.mk_const(name, levels), Some(target))
                }
            };
            record_hover(hovers, scope, *span, out, resolution);
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
            record_hover(hovers, scope, *span, out, None);
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
            record_hover(hovers, scope, *span, out, None);
            Ok(out)
        }
        Expr::Hole { span } => Err(CompileError::elab(
            ErrorKind::ElabHoleMisplaced,
            "`sorry` is only allowed as the value of an open exercise",
            *span,
        )),
        Expr::App { fun, arg, span } => {
            let fun = elab_expr(builder, fun, scope, univ, known, hovers, None)?;
            let arg = elab_expr(builder, arg, scope, univ, known, hovers, None)?;
            let out = builder.mk_app(fun, arg);
            record_hover(hovers, scope, *span, out, None);
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
            let mut rest = expected;
            for binder in binders {
                let (ty, style) = match &binder.ty {
                    Some(ty) => {
                        let t = elab_expr(builder, ty, scope, univ, known, hovers, None)?;
                        // The annotation wins, but the expected telescope
                        // still loses one layer so later untyped binders
                        // stay aligned with the declared type.
                        rest = drop_expected_layer(rest);
                        (t, kernel_binder_style(&binder.style))
                    }
                    None => match peel_expected(rest) {
                        Some((style, binder_ty, body)) => {
                            rest = Some(body);
                            (binder_ty, style)
                        }
                        None => {
                            return Err(CompileError::elab(
                                ErrorKind::ElabUntypedBinder,
                                "cannot infer the type of this binder: the declared type does not \
                                 provide a matching position (write it explicitly, e.g. fun (x : Nat) => x)",
                                binder.span,
                            ));
                        }
                    },
                };
                let name = builder.name_from_str(&binder.name);
                tys.push(ty);
                names.push(name);
                styles.push(style);
                record_binder_hover(hovers, scope, binder.span, ty);
                scope.push(binder.name.clone(), ty, binder.span);
            }
            let mut body_expr = elab_expr(builder, body, scope, univ, known, hovers, rest)?;
            scope.truncate(base);
            for ((name, ty), style) in names.into_iter().zip(tys).zip(styles).rev() {
                body_expr = builder.mk_lambda(name, style, ty, body_expr);
            }
            record_hover(hovers, scope, *span, body_expr, None);
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
                    Some(ty) => elab_expr(builder, ty, scope, univ, known, hovers, None)?,
                    None => {
                        return Err(CompileError::elab(
                            ErrorKind::ElabUntypedBinder,
                            "types must be written explicitly on Pi binders",
                            binder.span,
                        ));
                    }
                };
                let name = builder.name_from_str(&binder.name);
                tys.push(ty);
                names.push(name);
                styles.push(kernel_binder_style(&binder.style));
                record_binder_hover(hovers, scope, binder.span, ty);
                scope.push(binder.name.clone(), ty, binder.span);
            }
            let mut body_expr = elab_expr(builder, body, scope, univ, known, hovers, None)?;
            scope.truncate(base);
            for ((name, ty), style) in names.into_iter().zip(tys).zip(styles).rev() {
                body_expr = builder.mk_pi(name, style, ty, body_expr);
            }
            record_hover(hovers, scope, *span, body_expr, None);
            Ok(body_expr)
        }
        Expr::Arrow {
            domain,
            codomain,
            span,
        } => {
            let domain = elab_expr(builder, domain, scope, univ, known, hovers, None)?;
            // `A -> B` desugars to a Pi with an anonymous binder, so free
            // variables in the codomain live one binder deeper.
            scope.push(String::new(), domain, Span::default());
            let codomain = elab_expr(builder, codomain, scope, univ, known, hovers, None)?;
            scope.truncate(scope.len() - 1);
            let anon = builder.anonymous();
            let out = builder.mk_pi(anon, BinderStyle::Default, domain, codomain);
            record_hover(hovers, scope, *span, out, None);
            Ok(out)
        }
        Expr::Plus { lhs, rhs, span } => {
            // `+` is sugar for `Nat.add`; in bare mode (no Nat prelude) the
            // constant must not dangle — report a proper unknown identifier.
            if !known.contains_key("Nat.add") {
                return Err(CompileError::elab(
                    ErrorKind::ElabUnknownIdentifier,
                    "`+` needs Nat.add, which is not defined (install the prelude or define Nat yourself)",
                    *span,
                ));
            }
            let add = builder.name_from_str("Nat.add");
            let levels = builder.alloc_levels_slice(&[]);
            let add_const = builder.mk_const(add, levels);
            let lhs = elab_expr(builder, lhs, scope, univ, known, hovers, None)?;
            let rhs = elab_expr(builder, rhs, scope, univ, known, hovers, None)?;
            let applied = builder.mk_app(add_const, lhs);
            let out = builder.mk_app(applied, rhs);
            record_hover(hovers, scope, *span, out, None);
            Ok(out)
        }
    }
}

/// Whether any sub-expression of `e` uses the identifier `name` (mirror of
/// the kernel's own `is_recursive` scan over constructor binder types, which
/// checks binder types for a mention of an inductive name of the block).
fn mentions_ident(e: &Expr, name: &str) -> bool {
    match e {
        Expr::Ident { name: n, .. } => n == name,
        Expr::UniverseApp { name: n, .. } => n == name,
        Expr::Sort { .. } | Expr::Num { .. } | Expr::Hole { .. } => false,
        Expr::App { fun, arg, .. } => mentions_ident(fun, name) || mentions_ident(arg, name),
        Expr::Lambda { binders, body, .. } | Expr::Forall { binders, body, .. } => {
            binders
                .iter()
                .any(|b| b.ty.as_deref().is_some_and(|ty| mentions_ident(ty, name)))
                || mentions_ident(body, name)
        }
        Expr::Arrow {
            domain, codomain, ..
        } => mentions_ident(domain, name) || mentions_ident(codomain, name),
        Expr::Plus { lhs, rhs, .. } => mentions_ident(lhs, name) || mentions_ident(rhs, name),
    }
}

/// Walk the constructor's result as a Pi telescope (every arrow domain is a
/// binder type, the final codomain is not) and report whether any binder type
/// mentions `name` — the kernel scans the elaborated ctor type the same way.
fn result_telescope_mentions(result: &Expr, name: &str) -> bool {
    let mut current = result;
    loop {
        match current {
            Expr::Arrow {
                domain, codomain, ..
            } => {
                if mentions_ident(domain, name) {
                    return true;
                }
                current = codomain;
            }
            Expr::Forall { binders, body, .. } => {
                if binders
                    .iter()
                    .any(|b| b.ty.as_deref().is_some_and(|ty| mentions_ident(ty, name)))
                {
                    return true;
                }
                current = body;
            }
            _ => return false,
        }
    }
}

// ---------------------------------------------------------------------------
// 无显式 rec 的归纳块：recursor 自动派生
//
// 与 py-nat 的手写 rec 同构（内核按同形状重建规则并 def_eq 比对）：
//   rec <Ind>.rec {u} :
//     (motive : (x : Ind) -> Sort u) ->
//     (m<i> : forall (<字段望远镜> <ih…>), motive (<c_i> <字段>…)) …
//     (target : Ind) -> motive target
//   iota <c_i> := fun (motive) => fun (m_0) => … =>
//     fun (<字段望远镜>) => m_i <字段…> [<递归字段后的自调用>]
// ---------------------------------------------------------------------------

/// One constructor's derived view: its (hygiene-renamed) field telescope and,
/// per recursive field in declaration order, the field name plus the binder
/// telescope of the self-call (the Pi domains of the field type).
struct DerivedCtor {
    fields: Vec<Binder>,
    rec_args: Vec<(String, Vec<Binder>)>,
}

/// All fields of a constructor in declaration order: the explicit binders
/// followed by the domains of the result's arrow chain — the parser puts
/// `ctor base : (b : Bad) -> Bad`'s field in the result, and the kernel
/// counts the whole elaborated Pi telescope (`pi_telescope_size`).
fn ctor_field_binders(ctor: &CtorDecl) -> Vec<Binder> {
    let mut out: Vec<Binder> = ctor.binders.to_vec();
    out.extend(result_chain_binders(&ctor.result));
    out
}

/// The binder telescope of a (possibly arrow-chained) type: Forall binders
/// are collected verbatim, `A -> B` contributes one anonymous binder for `A`.
fn result_chain_binders(result: &Expr) -> Vec<Binder> {
    let mut out = Vec::new();
    let mut current = result;
    loop {
        match current {
            Expr::Arrow {
                domain, codomain, ..
            } => {
                out.push(Binder {
                    name: String::new(),
                    ty: Some(Box::new(domain.as_ref().clone())),
                    style: BinderKind::Explicit,
                    span: current.span(),
                });
                current = codomain;
            }
            Expr::Forall { binders, body, .. } => {
                out.extend(binders.iter().cloned());
                current = body;
            }
            _ => return out,
        }
    }
}

/// A name that no already-chosen binder uses (identifiers may shadow, so the
/// derived telescopes must avoid every name they will reference).
fn fresh_name(base: &str, taken: &mut HashSet<String>) -> String {
    let mut candidate = base.to_string();
    while taken.contains(&candidate) {
        candidate.push('_');
    }
    taken.insert(candidate.clone());
    candidate
}

/// `Prop`/`Sort 0` written as the block's declared sort. The kernel then only
/// allows large elimination when the block is empty or has a single ctor with
/// exclusively Prop-typed fields; a multi-ctor Prop block therefore gets a
/// small-elimination recursor (no universe parameter, motive into `Prop`).
fn is_prop_block_ty(ty: &Expr) -> bool {
    matches!(
        ty,
        Expr::Sort {
            sort: SortKind::Prop,
            ..
        } | Expr::Sort {
            sort: SortKind::Sort(0),
            ..
        }
    )
}

fn e_ident(name: &str, span: Span) -> Expr {
    Expr::Ident {
        name: name.to_string(),
        span,
    }
}

fn e_app(fun: Expr, arg: Expr, span: Span) -> Expr {
    Expr::App {
        fun: Box::new(fun),
        arg: Box::new(arg),
        span,
    }
}

fn e_forall(binders: Vec<Binder>, body: Expr, span: Span) -> Expr {
    Expr::Forall {
        binders,
        body: Box::new(body),
        span,
    }
}

fn e_lambda(binders: Vec<Binder>, body: Expr, span: Span) -> Expr {
    Expr::Lambda {
        binders,
        body: Box::new(body),
        span,
    }
}

fn e_universe_app(name: &str, levels: &[String], span: Span) -> Expr {
    Expr::UniverseApp {
        name: name.to_string(),
        levels: levels.to_vec(),
        span,
    }
}

/// Synthesize the recursor declaration and one iota rule per constructor for
/// a block written without `rec`. Every binder name is picked fresh against
/// the names the synthesized terms must reference (inductive, constructors,
/// source fields), so no derived binder can shadow a reference.
fn derive_recursor(name: &str, ty: &Expr, constructors: &[CtorDecl]) -> (RecDecl, Vec<IotaRule>) {
    let ty_span = ty.span();
    let small_elim = is_prop_block_ty(ty) && constructors.len() > 1;
    let universe: Vec<String> = if small_elim {
        Vec::new()
    } else {
        vec!["u".to_string()]
    };
    let motive_sort = |span: Span| {
        if small_elim {
            Expr::Sort {
                sort: SortKind::Prop,
                span,
            }
        } else {
            Expr::Sort {
                sort: SortKind::Level("u".to_string()),
                span,
            }
        }
    };

    let mut taken: HashSet<String> = HashSet::new();
    taken.insert(name.to_string());
    for ctor in constructors {
        taken.insert(ctor.name.clone());
    }
    for ctor in constructors {
        for field in ctor_field_binders(ctor) {
            if !field.name.is_empty() {
                taken.insert(field.name);
            }
        }
    }
    let motive = fresh_name("motive", &mut taken);
    let minors: Vec<String> = (0..constructors.len())
        .map(|i| fresh_name(&format!("m{i}"), &mut taken))
        .collect();
    let target = fresh_name("target", &mut taken);

    let motive_ty = e_forall(
        vec![Binder {
            name: "x".to_string(),
            ty: Some(Box::new(e_ident(name, ty_span))),
            style: BinderKind::Explicit,
            span: ty_span,
        }],
        motive_sort(ty_span),
        ty_span,
    );

    let derived: Vec<DerivedCtor> = constructors
        .iter()
        .map(|ctor| {
            let mut fields = Vec::new();
            for binder in ctor_field_binders(ctor) {
                let base = if binder.name.is_empty() {
                    "x"
                } else {
                    &binder.name
                };
                let field_name = fresh_name(base, &mut taken);
                fields.push(Binder {
                    name: field_name,
                    ty: binder.ty,
                    style: binder.style,
                    span: binder.span,
                });
            }
            let rec_args = fields
                .iter()
                .filter(|field| {
                    field
                        .ty
                        .as_deref()
                        .is_some_and(|ty| mentions_ident(ty, name))
                })
                .map(|field| {
                    let raw = result_chain_binders(field.ty.as_deref().expect("field has a type"));
                    let mut telescope = Vec::with_capacity(raw.len());
                    for binder in raw {
                        let base = if binder.name.is_empty() {
                            "x"
                        } else {
                            &binder.name
                        };
                        let binder_name = fresh_name(base, &mut taken);
                        telescope.push(Binder {
                            name: binder_name,
                            ty: binder.ty,
                            style: binder.style,
                            span: binder.span,
                        });
                    }
                    (field.name.clone(), telescope)
                })
                .collect();
            DerivedCtor { fields, rec_args }
        })
        .collect();

    // 每个构造子的 minor 前提：forall (字段… ih…), motive (<c_i> 字段…)。
    let minor_types: Vec<Expr> = constructors
        .iter()
        .zip(&derived)
        .map(|(ctor, d)| {
            let mut binders = d.fields.clone();
            for (field_name, telescope) in &d.rec_args {
                let field_app = telescope
                    .iter()
                    .fold(e_ident(field_name, ctor.span), |acc, binder| {
                        e_app(acc, e_ident(&binder.name, ctor.span), ctor.span)
                    });
                let ih_body = e_app(e_ident(&motive, ctor.span), field_app, ctor.span);
                let ih_ty = if telescope.is_empty() {
                    ih_body
                } else {
                    e_forall(telescope.clone(), ih_body, ctor.span)
                };
                let ih = fresh_name("ih", &mut taken);
                binders.push(Binder {
                    name: ih,
                    ty: Some(Box::new(ih_ty)),
                    style: BinderKind::Explicit,
                    span: ctor.span,
                });
            }
            let c_app = d
                .fields
                .iter()
                .fold(e_ident(&ctor.name, ctor.span), |acc, field| {
                    e_app(acc, e_ident(&field.name, ctor.span), ctor.span)
                });
            let body = e_app(e_ident(&motive, ctor.span), c_app, ctor.span);
            e_forall(binders, body, ctor.span)
        })
        .collect();

    let mut rec_binders = Vec::with_capacity(constructors.len() + 2);
    rec_binders.push(Binder {
        name: motive.clone(),
        ty: Some(Box::new(motive_ty.clone())),
        style: BinderKind::Explicit,
        span: ty_span,
    });
    for ((ctor, minor_name), minor_ty) in constructors.iter().zip(&minors).zip(&minor_types) {
        rec_binders.push(Binder {
            name: minor_name.clone(),
            ty: Some(Box::new(minor_ty.clone())),
            style: BinderKind::Explicit,
            span: ctor.span,
        });
    }
    rec_binders.push(Binder {
        name: target.clone(),
        ty: Some(Box::new(e_ident(name, ty_span))),
        style: BinderKind::Explicit,
        span: ty_span,
    });
    let rec_ty = e_forall(
        rec_binders,
        e_app(
            e_ident(&motive, ty_span),
            e_ident(&target, ty_span),
            ty_span,
        ),
        ty_span,
    );
    let rec = RecDecl {
        name: format!("{name}.rec"),
        universe: universe.clone(),
        ty: rec_ty,
        span: constructors.last().map(|ctor| ctor.span).unwrap_or(ty_span),
    };

    // 每构造子一条规则：telescope = (motive, 全部 minors, 本构造子字段)，
    // 返回 m_i <字段…>，递归字段后面追加自调用（py-nat succ 同形）。
    let rules = constructors
        .iter()
        .enumerate()
        .map(|(i, ctor)| {
            let d = &derived[i];
            let mut binders = Vec::with_capacity(constructors.len() + d.fields.len() + 1);
            binders.push(Binder {
                name: motive.clone(),
                ty: Some(Box::new(motive_ty.clone())),
                style: BinderKind::Explicit,
                span: ty_span,
            });
            for (minor_name, minor_ty) in minors.iter().zip(&minor_types) {
                binders.push(Binder {
                    name: minor_name.clone(),
                    ty: Some(Box::new(minor_ty.clone())),
                    style: BinderKind::Explicit,
                    span: ty_span,
                });
            }
            binders.extend(d.fields.iter().cloned());
            let mut body = e_ident(&minors[i], ctor.span);
            for field in &d.fields {
                body = e_app(body, e_ident(&field.name, ctor.span), ctor.span);
            }
            for (field_name, telescope) in &d.rec_args {
                let mut call = e_universe_app(&format!("{name}.rec"), &universe, ctor.span);
                call = e_app(call, e_ident(&motive, ctor.span), ctor.span);
                for minor_name in &minors {
                    call = e_app(call, e_ident(minor_name, ctor.span), ctor.span);
                }
                let field_app = telescope
                    .iter()
                    .fold(e_ident(field_name, ctor.span), |acc, binder| {
                        e_app(acc, e_ident(&binder.name, ctor.span), ctor.span)
                    });
                call = e_app(call, field_app, ctor.span);
                let self_call = if telescope.is_empty() {
                    call
                } else {
                    e_lambda(telescope.clone(), call, ctor.span)
                };
                body = e_app(body, self_call, ctor.span);
            }
            IotaRule {
                ctor_name: ctor.name.clone(),
                val: e_lambda(binders, body, ctor.span),
                span: ctor.span,
            }
        })
        .collect();
    (rec, rules)
}

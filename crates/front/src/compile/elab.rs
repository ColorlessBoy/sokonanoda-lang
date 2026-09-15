//! AST → 内核表达式的 elaborate、声明构建（build_*）与 hover 记录。

use super::error::{CompileError, ErrorKind};
use super::prelude::CompileOptions;
use super::report::ResolvedTarget;
use crate::judge::{judge_infer, GoalBinderSpec};
use crate::proof::render_expr;
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

/// One constructor field's elaborated type, for building a `match` minor.
#[derive(Debug, Clone)]
pub(crate) struct MatchField<'a> {
    /// The constructor's declared field name (diagnostics).
    pub name: String,
    pub ty: ExprPtr<'a>,
    pub style: BinderStyle,
    /// Source type (for binder hover / judge-inference scope).
    pub src_ty: Option<Expr>,
}

/// One constructor of a source-declared inductive, in declaration order.
#[derive(Debug, Clone)]
pub(crate) struct MatchCtor<'a> {
    pub name: String,
    pub fields: Vec<MatchField<'a>>,
}

/// Source-declared inductive metadata that `match` lowering reads (kernel frozen).
#[derive(Debug, Clone)]
pub(crate) struct InductiveInfo<'a> {
    pub ctors: Vec<MatchCtor<'a>>,
    pub recursor: String,
    pub rec_universe_arity: usize,
    pub recursive: bool,
    /// Non-indexed parameter count (`num_params=0` for `Nat`/prelude).
    pub num_params: usize,
    /// Parameter names in declaration order (for `match` param substitution).
    pub param_names: Vec<String>,
}

/// Forward-accumulated registry of the file's own `inductive` blocks, keyed by
/// inductive name. A `match` may only eliminate an inductive already declared
/// earlier in the file (design §4).
pub(crate) type InductiveTable<'a> = HashMap<String, InductiveInfo<'a>>;

/// Read-only context threaded through elaboration: source prefix + compile
/// options (for the [`judge_infer`] universe query that `match` needs) and the
/// inductive registry.
pub(crate) struct ElabCtx<'a, 'b> {
    pub prefix_src: &'b str,
    pub options: &'b CompileOptions,
    pub inductives: &'b InductiveTable<'a>,
}

pub(crate) struct ElabScope<'a> {
    names: Vec<String>,
    tys: Vec<ExprPtr<'a>>,
    /// Parallel to `names`: the binder's source type when it was written
    /// explicitly (used to synthesize `judge_infer` binder specs for `match`).
    src_tys: Vec<Option<Expr>>,
    /// Parallel to `names`: each binder's own source span, so a name use can
    /// record where its binder is defined.
    spans: Vec<Span>,
}

impl<'a> ElabScope<'a> {
    pub(crate) fn new() -> Self {
        Self {
            names: Vec::new(),
            tys: Vec::new(),
            src_tys: Vec::new(),
            spans: Vec::new(),
        }
    }
    fn len(&self) -> usize {
        self.names.len()
    }
    fn truncate(&mut self, len: usize) {
        self.names.truncate(len);
        self.tys.truncate(len);
        self.src_tys.truncate(len);
        self.spans.truncate(len);
    }
    fn push(&mut self, name: String, ty: ExprPtr<'a>, src_ty: Option<Expr>, span: Span) {
        self.names.push(name);
        self.tys.push(ty);
        self.src_tys.push(src_ty);
        self.spans.push(span);
    }
    /// Binder specs for [`judge_infer`]: named binders with a written source
    /// type, in scope order. Anonymous/untyped binders are dropped (nothing can
    /// reference them by name).
    fn judge_binders(&self) -> Vec<GoalBinderSpec> {
        self.names
            .iter()
            .zip(self.src_tys.iter())
            .filter(|(name, _)| !name.is_empty())
            .filter_map(|(name, src)| {
                src.as_ref().map(|ty| GoalBinderSpec {
                    name: name.clone(),
                    ty: Some(render_expr(ty)),
                })
            })
            .collect()
    }
    /// Like [`judge_binders`], but keeps only the binders `expr` (transitively)
    /// depends on, in scope order.
    ///
    /// `judge_infer` peels its answer one Pi layer at a time by re-rendering and
    /// re-parsing each intermediate type, and `render_expr` does not parenthesise
    /// a `forall` that sits in an arrow's domain. A binder whose written type is
    /// itself a function (`hs : (k : Nat) -> P k -> P (succ k)`) therefore
    /// corrupts that round trip and the query returns the wrong sub-term — even
    /// when the sort being asked about never mentions it. Passing only the needed
    /// binders keeps those unrelated function-typed binders out of the telescope.
    fn judge_binders_for(&self, expr: &Expr) -> Vec<GoalBinderSpec> {
        let n = self.len();
        let mut needed = vec![false; n];
        for (i, name) in self.names.iter().enumerate() {
            if !name.is_empty() && mentions_ident(expr, name) {
                needed[i] = true;
            }
        }
        // A binder's written type may only mention earlier binders, so a single
        // right-to-left pass closes the dependency set.
        for i in (0..n).rev() {
            if !needed[i] {
                continue;
            }
            if let Some(ty) = self.src_tys[i].as_ref() {
                for (j, name) in self.names.iter().enumerate().take(i) {
                    if !needed[j] && !name.is_empty() && mentions_ident(ty, name) {
                        needed[j] = true;
                    }
                }
            }
        }
        (0..n)
            .filter(|&i| needed[i])
            .filter_map(|i| {
                self.src_tys[i].as_ref().map(|ty| GoalBinderSpec {
                    name: self.names[i].clone(),
                    ty: Some(render_expr(ty)),
                })
            })
            .collect()
    }
    /// The written source type of the innermost binder named `name`.
    fn src_ty(&self, name: &str) -> Option<&Expr> {
        let pos = self.names.iter().rposition(|candidate| candidate == name)?;
        self.src_tys[pos].as_ref()
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
    table: &mut InductiveTable<'a>,
    prefix_src: &str,
    options: &CompileOptions,
    name: &str,
    params: &[Binder],
    ty: &Expr,
    constructors: &[CtorDecl],
    recursor: Option<&RecDecl>,
    iota_rules: &[crate::IotaRule],
    hovers: &mut Vec<HoverNode<'a>>,
    built: &mut Vec<Declar<'a>>,
) -> Result<(), CompileError> {
    let num_params = u16::try_from(params.len()).map_err(|_| {
        CompileError::elab(
            ErrorKind::ElabTooManyBinders,
            "too many inductive parameters",
            ty.span(),
        )
    })?;
    // 显式 rec 优先：源里有 rec 时零行为变化；无 rec 时自动派生等价的
    // RecDecl + iota 规则（py-nat 手写版同构），再走同一条 elab 路径。
    let owned_rec;
    let owned_rules;
    let (recursor, iota_rules): (&RecDecl, &[crate::IotaRule]) = match recursor {
        Some(rec) => (rec, iota_rules),
        None => {
            let (rec, rules) = derive_recursor(name, params, ty, constructors);
            owned_rec = rec;
            owned_rules = rules;
            (&owned_rec, &owned_rules)
        }
    };
    let empty: UnivMap = UnivMap::new();
    // 归纳声明自身内部出现 `match` 的情形按「本块尚未登记」处理（递归类型本就
    // 不在 v1 支持内）。这里借用既有登记表，插入在本函数末尾进行。
    let elab_ctx = ElabCtx {
        prefix_src,
        options,
        inductives: table,
    };
    // 归纳类型 = `forall params, ty`：params 是内核 Pi 望远镜最外层（顺序与
    // 声明的 binder 风格一致），ty 在它们的作用域内 elaborate。
    let ind_ty_src = if params.is_empty() {
        ty.clone()
    } else {
        Expr::Forall {
            binders: params.to_vec(),
            body: Box::new(ty.clone()),
            span: ty.span(),
        }
    };
    let ty = elab_expr(
        builder,
        &ind_ty_src,
        &mut ElabScope::new(),
        &empty,
        known,
        hovers,
        None,
        None,
        &elab_ctx,
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
            num_params,
            0,
            Arc::from([ind_name]),
            Arc::from(ctor_names.clone()),
        )
        .map_err(|e| CompileError::elab(ErrorKind::ElabDuplicateDeclaration, e, Span::default()))?;
    built.push(ind_declar);
    known.insert(name.to_string(), Vec::new());

    let mut match_ctors: Vec<MatchCtor<'a>> = Vec::with_capacity(constructors.len());
    for (idx, ctor) in constructors.iter().enumerate() {
        // ctor 类型 = `forall (params ++ fields), result`：参数先于字段，且必须
        // 与归纳声明的参数逐位同形（内核 check_ctor 会 def_eq 断言）。
        let mut ctor_binders: Vec<Binder> = params.to_vec();
        ctor_binders.extend(ctor.binders.iter().cloned());
        let ctor_ty = Expr::Forall {
            binders: ctor_binders,
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
            None,
            &elab_ctx,
        )?;
        // 字段元数据：类型用已 elaborate 的内核 Pi 望远镜（与 recursor 的
        // minor 形状逐位一致），源码 binder 提供 hover / judge 用的源类型。
        // 内核望远镜前 `num_params` 层是参数，字段从其后的位置开始。
        let src_fields = ctor_field_binders(ctor);
        let kernel_fields = kernel_field_binders(ctor_ty);
        let kernel_field_tys = kernel_fields.into_iter().skip(params.len());
        let fields = src_fields
            .iter()
            .zip(kernel_field_tys)
            .map(|(src, (style, kernel_ty))| MatchField {
                name: src.name.clone(),
                ty: kernel_ty,
                style,
                src_ty: src.ty.as_deref().cloned(),
            })
            .collect();
        match_ctors.push(MatchCtor {
            name: ctor.name.clone(),
            fields,
        });
        let ctor_name = ctor_names[idx];
        let no_uparams = builder.alloc_levels_slice(&[]);
        // 内核把构造子类型整体当 Pi 望远镜数字段（result 箭头链的 domain
        // 也是字段），num_fields = 望远镜 − 参数（check_declared_metadata）。
        // `ctor_field_binders` 只含字段，故无需再减参数。
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
            num_params,
            num_fields,
        });
        builder
            .add_declar(ctor_declar.clone())
            .map_err(|e| CompileError::elab(ErrorKind::ElabDuplicateDeclaration, e, ctor.span))?;
        built.push(ctor_declar);
        known.insert(ctor.name.clone(), Vec::new());
    }

    let rec_name_text = recursor.name.clone();
    let rec_universe_arity = recursor.universe.len();
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
            None,
            &elab_ctx,
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
                None,
                &elab_ctx,
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
            num_params,
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
    table.insert(
        name.to_string(),
        InductiveInfo {
            ctors: match_ctors,
            recursor: rec_name_text,
            rec_universe_arity,
            recursive: is_recursive,
            num_params: params.len(),
            param_names: params.iter().map(|b| b.name.clone()).collect(),
        },
    );
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn build_def<'a>(
    builder: &mut EnvBuilder<'a>,
    name: &str,
    universe: &[String],
    ty: &Expr,
    val: &Expr,
    known: &HashMap<String, Vec<String>>,
    hovers: &mut Vec<HoverNode<'a>>,
    ctx: &ElabCtx<'a, '_>,
) -> Result<Declar<'a>, CompileError> {
    let mut scope = ElabScope::new();
    let univ = make_univ_map(builder, universe);
    let ty_kernel = elab_expr(
        builder, ty, &mut scope, &univ, known, hovers, None, None, ctx,
    )?;
    let val_kernel = elab_expr(
        builder,
        val,
        &mut scope,
        &univ,
        known,
        hovers,
        Some(ty_kernel),
        Some(ty),
        ctx,
    )?;
    let name = builder.name_from_str(name);
    let uparams = collect_uparams(builder, &univ, universe);
    Ok(Declar::Definition {
        info: DeclarInfo {
            name,
            uparams,
            ty: ty_kernel,
        },
        val: val_kernel,
        hint: ReducibilityHint::Regular(0),
    })
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn build_theorem<'a>(
    builder: &mut EnvBuilder<'a>,
    name: &str,
    universe: &[String],
    ty: &Expr,
    val: &Expr,
    known: &HashMap<String, Vec<String>>,
    hovers: &mut Vec<HoverNode<'a>>,
    ctx: &ElabCtx<'a, '_>,
) -> Result<Declar<'a>, CompileError> {
    let mut scope = ElabScope::new();
    let univ = make_univ_map(builder, universe);
    let ty_kernel = elab_expr(
        builder, ty, &mut scope, &univ, known, hovers, None, None, ctx,
    )?;
    let val_kernel = elab_expr(
        builder,
        val,
        &mut scope,
        &univ,
        known,
        hovers,
        Some(ty_kernel),
        Some(ty),
        ctx,
    )?;
    let name = builder.name_from_str(name);
    let uparams = collect_uparams(builder, &univ, universe);
    Ok(Declar::Theorem {
        info: DeclarInfo {
            name,
            uparams,
            ty: ty_kernel,
        },
        val: val_kernel,
    })
}

pub(crate) fn build_example<'a>(
    builder: &mut EnvBuilder<'a>,
    name: &str,
    ty: &Expr,
    val: &Expr,
    known: &HashMap<String, Vec<String>>,
    hovers: &mut Vec<HoverNode<'a>>,
    ctx: &ElabCtx<'a, '_>,
) -> Result<Declar<'a>, CompileError> {
    let mut scope = ElabScope::new();
    let univ = make_univ_map(builder, &[]);
    let ty_kernel = elab_expr(
        builder, ty, &mut scope, &univ, known, hovers, None, None, ctx,
    )?;
    let val_kernel = elab_expr(
        builder,
        val,
        &mut scope,
        &univ,
        known,
        hovers,
        Some(ty_kernel),
        Some(ty),
        ctx,
    )?;
    let name = builder.name_from_str(name);
    let uparams = builder.alloc_levels_slice(&[]);
    Ok(Declar::Definition {
        info: DeclarInfo {
            name,
            uparams,
            ty: ty_kernel,
        },
        val: val_kernel,
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
    ctx: &ElabCtx<'a, '_>,
) -> Result<Declar<'a>, CompileError> {
    let mut scope = ElabScope::new();
    let univ = make_univ_map(builder, universe);
    let ty = elab_expr(
        builder, ty, &mut scope, &univ, known, hovers, None, None, ctx,
    )?;
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

/// Source-level mirror of [`peel_expected`]: the binder style, the written
/// domain and the remaining source type.
fn peel_expected_src(expected: Option<&Expr>) -> Option<(BinderStyle, Expr, Expr)> {
    match expected? {
        Expr::Arrow {
            domain, codomain, ..
        } => Some((
            BinderStyle::Default,
            domain.as_ref().clone(),
            codomain.as_ref().clone(),
        )),
        Expr::Forall { binders, body, .. } if !binders.is_empty() => {
            let binder = &binders[0];
            let domain = binder.ty.as_deref()?.clone();
            let rest = if binders.len() > 1 {
                Expr::Forall {
                    binders: binders[1..].to_vec(),
                    body: body.clone(),
                    span: binder.span,
                }
            } else {
                body.as_ref().clone()
            };
            Some((kernel_binder_style(&binder.style), domain, rest))
        }
        _ => None,
    }
}

/// Source-level mirror of [`drop_expected_layer`].
fn drop_expected_src_layer(expected: Option<&Expr>) -> Option<Expr> {
    match expected? {
        Expr::Arrow { codomain, .. } => Some(codomain.as_ref().clone()),
        Expr::Forall { binders, body, .. } if !binders.is_empty() => {
            if binders.len() > 1 {
                Some(Expr::Forall {
                    binders: binders[1..].to_vec(),
                    body: body.clone(),
                    span: binders[0].span,
                })
            } else {
                Some(body.as_ref().clone())
            }
        }
        _ => None,
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
    expected_src: Option<&Expr>,
    ctx: &ElabCtx<'a, '_>,
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
            let fun = elab_expr(builder, fun, scope, univ, known, hovers, None, None, ctx)?;
            let arg = elab_expr(builder, arg, scope, univ, known, hovers, None, None, ctx)?;
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
            let mut rest_src = expected_src.cloned();
            for binder in binders {
                let (ty, style, src_ty) = match &binder.ty {
                    Some(ty) => {
                        let t =
                            elab_expr(builder, ty, scope, univ, known, hovers, None, None, ctx)?;
                        // The annotation wins, but the expected telescope
                        // still loses one layer so later untyped binders
                        // stay aligned with the declared type.
                        rest = drop_expected_layer(rest);
                        rest_src = drop_expected_src_layer(rest_src.as_ref());
                        (
                            t,
                            kernel_binder_style(&binder.style),
                            Some(ty.as_ref().clone()),
                        )
                    }
                    None => match peel_expected(rest) {
                        Some((style, binder_ty, body)) => {
                            let src_layer = peel_expected_src(rest_src.as_ref());
                            rest = Some(body);
                            rest_src = src_layer.as_ref().map(|(_, _, body)| body.clone());
                            let src_ty = src_layer.map(|(_, domain, _)| domain);
                            (binder_ty, style, src_ty)
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
                scope.push(binder.name.clone(), ty, src_ty, binder.span);
            }
            let mut body_expr = elab_expr(
                builder,
                body,
                scope,
                univ,
                known,
                hovers,
                rest,
                rest_src.as_ref(),
                ctx,
            )?;
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
                    Some(ty) => {
                        elab_expr(builder, ty, scope, univ, known, hovers, None, None, ctx)?
                    }
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
                scope.push(
                    binder.name.clone(),
                    ty,
                    binder.ty.as_deref().cloned(),
                    binder.span,
                );
            }
            let mut body_expr =
                elab_expr(builder, body, scope, univ, known, hovers, None, None, ctx)?;
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
            let domain_src = domain.as_ref().clone();
            let domain = elab_expr(builder, domain, scope, univ, known, hovers, None, None, ctx)?;
            // `A -> B` desugars to a Pi with an anonymous binder, so free
            // variables in the codomain live one binder deeper.
            scope.push(String::new(), domain, Some(domain_src), Span::default());
            let codomain = elab_expr(
                builder, codomain, scope, univ, known, hovers, None, None, ctx,
            )?;
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
            let lhs = elab_expr(builder, lhs, scope, univ, known, hovers, None, None, ctx)?;
            let rhs = elab_expr(builder, rhs, scope, univ, known, hovers, None, None, ctx)?;
            let applied = builder.mk_app(add_const, lhs);
            let out = builder.mk_app(applied, rhs);
            record_hover(hovers, scope, *span, out, None);
            Ok(out)
        }
        Expr::Let {
            binder,
            val,
            body,
            span,
        } => {
            let base = scope.len();
            // 1) binder 类型在「未引入 x」的外层 scope 里 elaborate；无注解时
            //    问内核推断值 `v` 的类型（`judge_infer`，复用有界缓存）。
            let mut inferred_src: Option<Expr> = None;
            let ty = match binder.ty.as_deref() {
                Some(src_ty) => {
                    elab_expr(builder, src_ty, scope, univ, known, hovers, None, None, ctx)?
                }
                None => {
                    let binders = scope.judge_binders();
                    let text =
                        judge_infer(ctx.prefix_src, ctx.options, &binders, &render_expr(val))
                            .map_err(|_| {
                                CompileError::elab(
                            ErrorKind::ElabLetTypeQueryFailed,
                            "无法推断 `let` 绑定的类型；请补上类型标注，例如 `let x : Nat := 1; x`",
                            binder.span,
                        )
                            })?;
                    let parsed = crate::proof::parse_expr_text(&text).map_err(|_| {
                        CompileError::elab(
                            ErrorKind::ElabLetTypeQueryFailed,
                            "无法解析推断出的 `let` 绑定类型",
                            binder.span,
                        )
                    })?;
                    let kernel = elab_expr(
                        builder, &parsed, scope, univ, known, hovers, None, None, ctx,
                    )?;
                    inferred_src = Some(parsed);
                    kernel
                }
            };
            let ty_src = inferred_src.as_ref().or(binder.ty.as_deref());
            // 2) binder 声明行 hover（`x : T`），scope 仍是外层。
            record_binder_hover(hovers, scope, binder.span, ty);
            // 3) 值在期望类型 T 下 elaborate（未注解的 lambda binder 可借此推断）。
            let val = elab_expr(
                builder,
                val,
                scope,
                univ,
                known,
                hovers,
                Some(ty),
                ty_src,
                ctx,
            )?;
            // 4) 引入 x，body 在扩展 scope + 外层 expected 下 elaborate。
            scope.push(binder.name.clone(), ty, ty_src.cloned(), binder.span);
            let body = elab_expr(
                builder,
                body,
                scope,
                univ,
                known,
                hovers,
                expected,
                expected_src,
                ctx,
            )?;
            scope.truncate(base);
            // 5) 拼内核 Let 并落 hover（`nondep` 保守取 false，见设计 §3.3）。
            let name = builder.name_from_str(&binder.name);
            let out = builder.mk_let(name, ty, val, body, false);
            record_hover(hovers, scope, *span, out, None);
            Ok(out)
        }
        // `match`：降低为 `<Ind>.rec.{level} (fun (_ : Ind) => R) minor… scrutinee`
        // （design `docs/design/match.md` §5）。判定交给完整内核。
        Expr::Match {
            scrutinee,
            arms,
            span,
        } => {
            let (Some(_expected_kernel), Some(expected_src)) = (expected, expected_src) else {
                return Err(CompileError::elab(
                    ErrorKind::ElabMatchNoExpectedType,
                    "`match` 的结果类型必须已知：请把它放在有类型标注的位置（声明类型 / \
                     let / fun 的 binder 注解），或让外层 match 提供结果类型",
                    *span,
                ));
            };
            // 1) elaborate scrutinee (no expected), then find its inductive head.
            let scrutinee_kernel = elab_expr(
                builder, scrutinee, scope, univ, known, hovers, None, None, ctx,
            )?;
            let (ind_name, written_args) =
                infer_inductive_with_params(ctx, scope, scrutinee).ok_or_else(|| {
                    CompileError::elab(
                        ErrorKind::ElabMatchNotInductive,
                        "`match` 的被匹配项不是已知的归纳类型（本文件用 inductive 声明，或 prelude 的 Nat/Bool）",
                        scrutinee.span(),
                    )
                })?;
            let info = ctx.inductives.get(&ind_name).ok_or_else(|| {
                CompileError::elab(
                    ErrorKind::ElabMatchNotInductive,
                    format!(
                        "`match` 的被匹配项类型 `{ind_name}` 不是已知的归纳类型（本文件用 inductive 声明，或 prelude 的 Nat/Bool）"
                    ),
                    scrutinee.span(),
                )
            })?;
            // 参数化归纳的 params：scrutinee 必须是书写源类型为 `Ind p1 … pn`
            // 的局部变量；取前 num_params 个源实参。
            let param_args_src: Vec<Expr> = if info.num_params == 0 {
                Vec::new()
            } else {
                let args = written_args.as_ref().ok_or_else(|| {
                    CompileError::elab(
                        ErrorKind::ElabMatchParameterizedUnsupported,
                        format!(
                            "`match` 暂不支持这个参数化归纳形状 `{ind_name}`：被匹配项必须是一个书写类型为 `{ind_name} …` 的局部变量"
                        ),
                        scrutinee.span(),
                    )
                })?;
                if args.len() < info.num_params {
                    return Err(CompileError::elab(
                        ErrorKind::ElabMatchParameterizedUnsupported,
                        format!(
                            "`match` 暂不支持这个参数化归纳形状 `{ind_name}`：被匹配项的书写类型需要显式给出 {} 个参数",
                            info.num_params
                        ),
                        scrutinee.span(),
                    ));
                }
                args.iter().take(info.num_params).cloned().collect()
            };
            // 参数实参在进入构造子字段作用域之前 elaborate。
            let param_kernel: Vec<ExprPtr<'a>> = param_args_src
                .iter()
                .map(|arg| elab_expr(builder, arg, scope, univ, known, hovers, None, None, ctx))
                .collect::<Result<Vec<_>, _>>()?;
            // 递归归纳：字段源类型必须可知（用于定位递归字段并插 IH）。
            if info.recursive
                && info
                    .ctors
                    .iter()
                    .flat_map(|c| c.fields.iter())
                    .any(|f| f.src_ty.is_none())
            {
                return Err(CompileError::elab(
                    ErrorKind::ElabMatchRecursiveUnsupported,
                    format!(
                        "`match` 暂不支持这个递归归纳形状 `{ind_name}`：构造子字段缺少源类型，无法定位归纳假设"
                    ),
                    scrutinee.span(),
                ));
            }
            // 2) arms: bare ctor names, each covered exactly once, exact arity.
            let mut arm_by_ctor: HashMap<&str, &crate::ast::MatchArm> = HashMap::new();
            for arm in arms {
                if !info.ctors.iter().any(|c| c.name == arm.ctor) {
                    return Err(CompileError::elab(
                        ErrorKind::ElabMatchBadArm,
                        format!(
                            "`{ind_name}` 没有构造子 `{}`；可用的是：{}",
                            arm.ctor,
                            ctor_names_text(info)
                        ),
                        arm.span,
                    ));
                }
                if arm_by_ctor.insert(arm.ctor.as_str(), arm).is_some() {
                    return Err(CompileError::elab(
                        ErrorKind::ElabMatchBadArm,
                        format!("构造子 `{}` 被重复匹配了；每个构造子只能写一次", arm.ctor),
                        arm.span,
                    ));
                }
            }
            if let Some(missing) = info
                .ctors
                .iter()
                .find(|c| !arm_by_ctor.contains_key(c.name.as_str()))
            {
                return Err(CompileError::elab(
                    ErrorKind::ElabMatchNonExhaustive,
                    format!(
                        "`match` 漏掉了构造子 `{}`；请覆盖 `{ind_name}` 的每个构造子",
                        missing.name
                    ),
                    *span,
                ));
            }
            for ctor in &info.ctors {
                let arm = arm_by_ctor[ctor.name.as_str()];
                if arm.binders.len() != ctor.fields.len() {
                    return Err(CompileError::elab(
                        ErrorKind::ElabMatchBadArm,
                        format!(
                            "构造子 `{}` 有 {} 个字段，但这一支写了 {} 个模式变量；请写满字段：| {} {} => …",
                            ctor.name,
                            ctor.fields.len(),
                            arm.binders.len(),
                            ctor.name,
                            field_names_text(ctor)
                        ),
                        arm.span,
                    ));
                }
            }
            // 依赖 motive 触发（v1，design docs/design/match-dependent-motive.md §1）：
            // scrutinee 是裸局部变量 `x`，且 `x` 在结果类型 R 中出现。否则保持
            // 常量 motive（完全兼容既有行为）。
            let dependent_var: Option<String> = match &**scrutinee {
                Expr::Ident { name, .. }
                    if scope.names.iter().any(|n| n == name)
                        && mentions_ident(expected_src, name) =>
                {
                    Some(name.clone())
                }
                _ => None,
            };
            // 3) level：judge_infer(R) 的类型文本映射宇宙（design §5 step 3）。
            let level = infer_expected_level(ctx, scope, expected_src).ok_or_else(|| {
                CompileError::elab(
                    ErrorKind::ElabMatchNoExpectedType,
                    "无法确定 `match` 结果类型所在的宇宙层级（v1 只支持内核能推断出 Sort 的结果类型）",
                    *span,
                )
            })?;
            // 4) motive：依赖时为 `fun (t : Ind params) => R[x := t]`，否则
            //    `fun (_ : Ind params) => R`（v1 非依赖，见设计 §2）。
            let empty_levels = builder.alloc_levels_slice(&[]);
            let ind_ptr = builder.name_from_str(&ind_name);
            let ind_const = builder.mk_const(ind_ptr, empty_levels);
            let ind_applied = param_kernel
                .iter()
                .fold(ind_const, |acc, p| builder.mk_app(acc, *p));
            // motive 域的书写源类型 = `Ind params`（从 scrutinee 的书写参数）。
            let ind_ty_src = param_args_src.iter().fold(
                Expr::Ident {
                    name: ind_name.clone(),
                    span: *span,
                },
                |acc, p| Expr::App {
                    fun: Box::new(acc),
                    arg: Box::new(p.clone()),
                    span: *span,
                },
            );
            let (motive_name, body_src) = if let Some(x) = &dependent_var {
                // 新鲜 motive binder 名 `t`：避让作用域内全部名字（包括 x）。
                let motive_name = {
                    let mut candidate = String::from("t");
                    let mut k = 1;
                    while scope.names.iter().any(|n| n == &candidate) {
                        k += 1;
                        candidate = format!("t{k}");
                    }
                    candidate
                };
                let mut map = HashMap::new();
                map.insert(
                    x.clone(),
                    Expr::Ident {
                        name: motive_name.clone(),
                        span: *span,
                    },
                );
                let body_src = super::goals::substitute_names(expected_src, &map, &HashMap::new());
                (motive_name, body_src)
            } else {
                (String::new(), expected_src.clone())
            };
            // body 必须在 motive binder 的作用域里 elaborate：`mk_lambda` 不做
            // de Bruijn shift，body 的索引须相对扩展后的上下文。
            let outer = scope.len();
            scope.push(motive_name.clone(), ind_applied, Some(ind_ty_src), *span);
            let motive_body = elab_expr(
                builder, &body_src, scope, univ, known, hovers, None, None, ctx,
            )?;
            scope.truncate(outer);
            let motive_name_ptr = if motive_name.is_empty() {
                builder.anonymous()
            } else {
                builder.name_from_str(&motive_name)
            };
            let motive = builder.mk_lambda(
                motive_name_ptr,
                BinderStyle::Default,
                ind_applied,
                motive_body,
            );
            // 5) minors：按构造子声明序重排；**递归字段后插入归纳假设 IH**
            //    （类型 = motive 结果 R，v1 非依赖 motive；design §5 / Phase 2）。
            let base = scope.len();
            let mut minors = Vec::with_capacity(info.ctors.len());
            for ctor in &info.ctors {
                let arm = arm_by_ctor[ctor.name.as_str()];
                let mut minor_binders: Vec<(String, BinderStyle, ExprPtr<'a>, Option<Expr>)> =
                    Vec::new();
                for (field, binder) in ctor.fields.iter().zip(arm.binders.iter()) {
                    // 参数化归纳：把字段源类型里的参数名代换成 scrutinee 的
                    // 书写实参后再 elaborate，得到该构造子在其实例下的字段类型。
                    let parameterized = info.num_params > 0;
                    let (field_ty, field_src_ty): (ExprPtr<'a>, Option<Expr>) = if !parameterized {
                        (field.ty, field.src_ty.clone())
                    } else {
                        let src = field.src_ty.as_ref().ok_or_else(|| {
                            CompileError::elab(
                                ErrorKind::ElabMatchParameterizedUnsupported,
                                format!(
                                    "`match` 暂不支持这个参数化归纳形状 `{ind_name}`：构造子 `{}` 的字段缺少源类型",
                                    ctor.name
                                ),
                                binder.span,
                            )
                        })?;
                        let map: HashMap<String, Expr> = info
                            .param_names
                            .iter()
                            .cloned()
                            .zip(param_args_src.iter().cloned())
                            .collect();
                        let subst = super::goals::substitute_names(src, &map, &HashMap::new());
                        let k = elab_expr(
                            builder, &subst, scope, univ, known, hovers, None, None, ctx,
                        )?;
                        (k, Some(subst))
                    };
                    record_binder_hover(hovers, scope, binder.span, field_ty);
                    scope.push(
                        binder.name.clone(),
                        field_ty,
                        field_src_ty.clone(),
                        binder.span,
                    );
                    minor_binders.push((
                        binder.name.clone(),
                        field.style,
                        field_ty,
                        field_src_ty.clone(),
                    ));
                    let recursive_field = info.recursive
                        && field_src_ty
                            .as_ref()
                            .is_some_and(|t| mentions_ident(t, &ind_name));
                    if recursive_field {
                        // 归纳假设名避开既有绑定（`ih`、`ih2`、…），供 branch 引用。
                        let ih_name = {
                            let used = |name: &str| {
                                scope.names.iter().any(|n| n == name)
                                    || minor_binders.iter().any(|(n, ..)| n == name)
                            };
                            let mut candidate = String::from("ih");
                            let mut k = 1;
                            while used(&candidate) {
                                k += 1;
                                candidate = format!("ih{k}");
                            }
                            candidate
                        };
                        // 依赖 motive：IH 类型 = motive <field> = R[x := field]；
                        // 否则保持常量 R（Phase 2 非依赖）。须在当前 minor 作用域
                        // 里 elaborate（索引相对已推入的字段/IH binder）。
                        let ih_src = if let Some(x) = &dependent_var {
                            let mut map = HashMap::new();
                            map.insert(
                                x.clone(),
                                Expr::Ident {
                                    name: binder.name.clone(),
                                    span: binder.span,
                                },
                            );
                            super::goals::substitute_names(expected_src, &map, &HashMap::new())
                        } else {
                            expected_src.clone()
                        };
                        let ih_kernel = elab_expr(
                            builder, &ih_src, scope, univ, known, hovers, None, None, ctx,
                        )?;
                        record_binder_hover(hovers, scope, binder.span, ih_kernel);
                        scope.push(
                            ih_name.clone(),
                            ih_kernel,
                            Some(ih_src.clone()),
                            binder.span,
                        );
                        minor_binders.push((
                            ih_name,
                            BinderStyle::Default,
                            ih_kernel,
                            Some(ih_src),
                        ));
                    }
                }
                // 依赖 motive：分支期望类型 = R[x := C params v…]（把 scrutinee
                // 变量替换成该分支的构造子项）；否则保持常量 R。同样在当前 minor
                // 作用域里 elaborate（branch body 就在这个作用域下）。
                let branch_expected_src = if let Some(x) = &dependent_var {
                    let mut term = Expr::Ident {
                        name: ctor.name.clone(),
                        span: arm.span,
                    };
                    for p in &param_args_src {
                        term = Expr::App {
                            fun: Box::new(term),
                            arg: Box::new(p.clone()),
                            span: arm.span,
                        };
                    }
                    for b in &arm.binders {
                        term = Expr::App {
                            fun: Box::new(term),
                            arg: Box::new(Expr::Ident {
                                name: b.name.clone(),
                                span: b.span,
                            }),
                            span: arm.span,
                        };
                    }
                    let mut map = HashMap::new();
                    map.insert(x.clone(), term);
                    super::goals::substitute_names(expected_src, &map, &HashMap::new())
                } else {
                    expected_src.clone()
                };
                let branch_expected = elab_expr(
                    builder,
                    &branch_expected_src,
                    scope,
                    univ,
                    known,
                    hovers,
                    None,
                    None,
                    ctx,
                )?;
                let mut body = elab_expr(
                    builder,
                    &arm.body,
                    scope,
                    univ,
                    known,
                    hovers,
                    Some(branch_expected),
                    Some(&branch_expected_src),
                    ctx,
                )?;
                scope.truncate(base);
                for (name, style, ty, _src) in minor_binders.iter().rev() {
                    let nm = builder.name_from_str(name);
                    body = builder.mk_lambda(nm, *style, *ty, body);
                }
                minors.push(body);
            }
            // 6) `<Ind>.rec.{level} params motive minor_1 … minor_n scrutinee`.
            let rec_ptr = builder.name_from_str(&info.recursor);
            let rec_const = if info.rec_universe_arity == 0 {
                // Prop 小消去推导出的 recursor 没有宇宙参数（如多构造子 Prop 枚举）。
                let levels = builder.alloc_levels_slice(&[]);
                builder.mk_const(rec_ptr, levels)
            } else {
                let lvl = level_from_u64(builder, level);
                let levels = builder.alloc_levels_slice(&[lvl]);
                builder.mk_const(rec_ptr, levels)
            };
            let mut app = rec_const;
            for param in &param_kernel {
                app = builder.mk_app(app, *param);
            }
            app = builder.mk_app(app, motive);
            for minor in minors {
                app = builder.mk_app(app, minor);
            }
            app = builder.mk_app(app, scrutinee_kernel);
            record_hover(hovers, scope, *span, app, None);
            Ok(app)
        }
        // `by` 块应在 elab 前由引擎降级为 lambda AST；到不了这里。
        Expr::By { span, .. } => Err(CompileError::elab(
            ErrorKind::ElabHoleMisplaced,
            "internal: `by` block reached elaboration without being lowered",
            *span,
        )),
    }
}

/// Peel a constructor's elaborated kernel type into its field binder types
/// (dependencies resolved by de Bruijn), in declaration order.
fn kernel_field_binders<'a>(mut ty: ExprPtr<'a>) -> Vec<(BinderStyle, ExprPtr<'a>)> {
    let mut out = Vec::new();
    loop {
        match &*ty {
            sokonanoda::expr::Expr::Pi {
                binder_style,
                binder_type,
                body,
                ..
            } => {
                out.push((*binder_style, *binder_type));
                ty = *body;
            }
            _ => return out,
        }
    }
}

/// The head identifier of a type expression (`Color`, `Color A`, `@{…}`), used
/// to map a scrutinee/expected type onto the inductive registry.
fn head_ident(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Ident { name, .. } | Expr::UniverseApp { name, .. } => Some(name.clone()),
        Expr::App { fun, .. } => head_ident(fun),
        _ => None,
    }
}

/// Flatten a source type application `C p1 … pn` into its head and arguments
/// (owned clones, for `match` parameter instantiation). Non-spine heads yield
/// `None`.
fn src_spine(expr: &Expr) -> Option<(String, Vec<Expr>)> {
    match expr {
        Expr::Ident { name, .. } | Expr::UniverseApp { name, .. } => {
            Some((name.clone(), Vec::new()))
        }
        Expr::App { fun, arg, .. } => {
            let (head, mut args) = src_spine(fun)?;
            args.push(arg.as_ref().clone());
            Some((head, args))
        }
        _ => None,
    }
}

/// The inductive a `match` scrutinee eliminates, plus (when the scrutinee is a
/// local variable with a written source type) the source arguments of that
/// type application — `Option Nat` yields `["Nat"]`. The argument list is
/// `None` when the head came from the `judge_infer` fallback.
fn infer_inductive_with_params(
    ctx: &ElabCtx,
    scope: &ElabScope,
    scrutinee: &Expr,
) -> Option<(String, Option<Vec<Expr>>)> {
    if let Expr::Ident { name, .. } = scrutinee {
        if let Some(ty) = scope.src_ty(name) {
            if let Some((head, args)) = src_spine(ty) {
                return Some((head, Some(args)));
            }
        }
    }
    let binders = scope.judge_binders();
    let text = judge_infer(
        ctx.prefix_src,
        ctx.options,
        &binders,
        &render_expr(scrutinee),
    )
    .ok()?;
    let ty = crate::proof::parse_expr_text(&text).ok()?;
    let head = head_ident(&ty)?;
    Some((head, None))
}

/// Map the kernel-rendered sort of `R` to the recursor's universe level:
/// `Prop`→0, `Type`→1, `Sort n`→n (design §5).
fn sort_text_level(text: &str) -> Option<u64> {
    match text.trim() {
        "Prop" => Some(0),
        "Type" => Some(1),
        other => {
            if let Some(rest) = other.strip_prefix("Sort ") {
                rest.trim().parse::<u64>().ok()
            } else if let Some(rest) = other.strip_prefix("Type ") {
                rest.trim().parse::<u64>().ok().map(|n| n + 1)
            } else {
                None
            }
        }
    }
}

/// The recursor universe level for the expected result type `R`: the sort of
/// `R` as inferred by the kernel (`judge_infer` reuses the 128-entry cache).
fn infer_expected_level(ctx: &ElabCtx, scope: &ElabScope, expected_src: &Expr) -> Option<u64> {
    let term = render_expr(expected_src);
    let binders = scope.judge_binders_for(expected_src);
    let text = if binders.is_empty() {
        // `R` is closed w.r.t. the local context: add one dummy `Prop` binder so
        // `judge_infer`'s `fun … => R` wrapper still has a layer to peel.
        let dummy = vec![GoalBinderSpec {
            name: "_soko_expected_level".to_string(),
            ty: Some("Prop".to_string()),
        }];
        judge_infer(ctx.prefix_src, ctx.options, &dummy, &term).ok()?
    } else {
        judge_infer(ctx.prefix_src, ctx.options, &binders, &term).ok()?
    };
    sort_text_level(&text)
}

fn level_from_u64<'a>(builder: &mut EnvBuilder<'a>, n: u64) -> LevelPtr<'a> {
    let mut level = builder.zero();
    for _ in 0..n {
        level = builder.succ(level);
    }
    level
}

fn ctor_names_text(info: &InductiveInfo) -> String {
    info.ctors
        .iter()
        .map(|c| c.name.as_str())
        .collect::<Vec<_>>()
        .join("，")
}

fn field_names_text(ctor: &MatchCtor) -> String {
    ctor.fields
        .iter()
        .map(|f| f.name.as_str())
        .collect::<Vec<_>>()
        .join(" ")
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
        Expr::Let {
            binder, val, body, ..
        } => {
            binder
                .ty
                .as_deref()
                .is_some_and(|ty| mentions_ident(ty, name))
                || mentions_ident(val, name)
                || mentions_ident(body, name)
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            mentions_ident(scrutinee, name)
                || arms.iter().any(|arm| {
                    arm.binders
                        .iter()
                        .any(|b| b.ty.as_deref().is_some_and(|ty| mentions_ident(ty, name)))
                        || mentions_ident(&arm.body, name)
                })
        }
        Expr::By { .. } => false, // by 块在 elab 前已被引擎降级为普通表达式
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
fn derive_recursor(
    name: &str,
    params: &[Binder],
    ty: &Expr,
    constructors: &[CtorDecl],
) -> (RecDecl, Vec<IotaRule>) {
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
    // `Ind p1 … pn`（无参数时就是裸 `Ind`）。
    let ind_applied = |span: Span| {
        params.iter().fold(e_ident(name, span), |acc, p| {
            e_app(acc, e_ident(&p.name, span), span)
        })
    };

    let mut taken: HashSet<String> = HashSet::new();
    taken.insert(name.to_string());
    for ctor in constructors {
        taken.insert(ctor.name.clone());
    }
    // 参数名纳入卫生集合：派生的字段/motive/minor 名不得遮蔽参数引用。
    for param in params {
        if !param.name.is_empty() {
            taken.insert(param.name.clone());
        }
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

    let motive_x = fresh_name("x", &mut taken);
    let motive_ty = e_forall(
        vec![Binder {
            name: motive_x,
            ty: Some(Box::new(ind_applied(ty_span))),
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
            let c_app = params
                .iter()
                .fold(e_ident(&ctor.name, ctor.span), |acc, p| {
                    e_app(acc, e_ident(&p.name, ctor.span), ctor.span)
                });
            let c_app = d.fields.iter().fold(c_app, |acc, field| {
                e_app(acc, e_ident(&field.name, ctor.span), ctor.span)
            });
            let body = e_app(e_ident(&motive, ctor.span), c_app, ctor.span);
            e_forall(binders, body, ctor.span)
        })
        .collect();

    // 递归子望远镜：params（最外层，风格与声明一致）→ motive → minors → 目标。
    let mut rec_binders: Vec<Binder> = params.to_vec();
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
        ty: Some(Box::new(ind_applied(ty_span))),
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
            let mut binders =
                Vec::with_capacity(params.len() + constructors.len() + d.fields.len() + 1);
            // iota 值 lambda 序：params → motive → minors → 本构造子字段。
            binders.extend(params.iter().cloned());
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
                for param in params {
                    call = e_app(call, e_ident(&param.name, ctor.span), ctor.span);
                }
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

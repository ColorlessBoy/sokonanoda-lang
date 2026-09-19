//! AST → 内核表达式的 elaborate、声明构建（build_*）与 hover 记录。

use super::error::{CompileError, ErrorKind};
use super::prelude::CompileOptions;
use super::report::ResolvedTarget;
use crate::ast::{MatchArm, Pattern};
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
    /// The constructor's declared field name (used to rename it to the user's
    /// `match` binder when a later field type references it).
    pub name: String,
    pub ty: ExprPtr<'a>,
    pub style: BinderStyle,
    /// Source type (for binder hover / judge-inference scope).
    pub src_ty: Option<Expr>,
}

/// One constructor of a source-declared inductive, in declaration order.
#[derive(Debug, Clone)]
pub(crate) struct MatchCtor<'a> {
    /// The name as written in the source (`prod_mk`) — `match` arms match on
    /// this spelling (R3, source-level).
    pub name: String,
    /// The installed kernel name (`Prod.prod_mk`, R1) — recursor rules and
    /// hover/goto use it.
    pub canonical: String,
    pub fields: Vec<MatchField<'a>>,
}

/// What a name in the `known` table resolves to.
///
/// `Decl` is a real declaration (or a prelude name): its canonical spelling is
/// the key itself. `Alias` is the **subset extension** (R2): a bare constructor
/// name that is unique in the closure resolves to its canonical name — it must
/// never become a second kernel constant. `Ambiguous` is a bare name two
/// constructors claim (G-02's `mk`): it does not resolve at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum KnownName {
    Decl { universes: Vec<String> },
    Alias { canonical: String },
    Ambiguous { candidates: Vec<String> },
}

impl KnownName {
    /// The universes a real declaration was installed with (aliases carry the
    /// canonical declaration's own arity, so they elab the same way).
    pub(crate) fn universes(&self) -> &[String] {
        match self {
            KnownName::Decl { universes } => universes,
            KnownName::Alias { .. } | KnownName::Ambiguous { .. } => &[],
        }
    }
}

/// The name table threaded through elaboration: source spelling → resolution.
/// Closure-level and flat (module scoping is G-05), matching the flat
/// `check_name_collisions` contract.
pub(crate) type KnownTable = HashMap<String, KnownName>;

/// R1: the canonical (installed) constructor name. A ctor whose source name
/// already carries a dot is kept verbatim — that protects the prelude
/// (`Nat.zero`/`Bool.true`) and any explicit dotted spelling.
pub(crate) fn canonical_ctor_name(ind: &str, ctor: &str) -> String {
    if ctor.contains('.') {
        ctor.to_string()
    } else {
        format!("{ind}.{ctor}")
    }
}

/// R2: register one bare-name alias. The first constructor to claim a bare
/// name owns it; a second one turns it `Ambiguous` (never silently wins). Real
/// declarations are never overwritten by an alias.
pub(crate) fn insert_ctor_alias(known: &mut KnownTable, source_name: &str, canonical: &str) {
    if source_name.contains('.') {
        return; // already the canonical spelling: nothing to alias
    }
    match known.get(source_name) {
        Some(KnownName::Decl { .. }) => {} // a real declaration wins (R2)
        Some(KnownName::Alias { canonical: first }) if first != canonical => {
            let mut candidates = vec![first.clone()];
            candidates.push(canonical.to_string());
            known.insert(source_name.to_string(), KnownName::Ambiguous { candidates });
        }
        Some(KnownName::Ambiguous { .. }) | Some(KnownName::Alias { .. }) => {}
        None => {
            known.insert(
                source_name.to_string(),
                KnownName::Alias {
                    canonical: canonical.to_string(),
                },
            );
        }
    }
}

/// Resolve one `Expr::Ident` spelling to its canonical kernel name.
///
/// * a real declaration resolves to itself;
/// * a **unique** bare constructor alias resolves to its canonical name (R2);
/// * an ambiguous bare alias is an error naming both candidates;
/// * an unknown name is the ordinary `elab-unknown-identifier`.
pub(crate) fn resolve_known(
    known: &KnownTable,
    name: &str,
    span: Span,
) -> Result<String, CompileError> {
    match known.get(name) {
        Some(KnownName::Decl { .. }) => Ok(name.to_string()),
        Some(KnownName::Alias { canonical }) => Ok(canonical.clone()),
        Some(KnownName::Ambiguous { candidates }) => Err(CompileError::elab(
            ErrorKind::ElabAmbiguousCtorAlias,
            format!(
                "构造子名 `{name}` 有歧义：{} 都声明了它；请写全前缀名",
                candidates
                    .iter()
                    .map(|c| format!("`{c}`"))
                    .collect::<Vec<_>>()
                    .join(" 与 ")
            ),
            span,
        )),
        None => Err(CompileError::elab(
            ErrorKind::ElabUnknownIdentifier,
            format!("unknown identifier `{name}`"),
            span,
        )),
    }
}

/// Same as [`resolve_known`], with the constant-flavoured unknown code
/// (`#check Nat.add.{1}` / `Foo.{u}` style uses).
pub(crate) fn resolve_known_constant(
    known: &KnownTable,
    name: &str,
    span: Span,
) -> Result<String, CompileError> {
    match known.get(name) {
        Some(KnownName::Ambiguous { candidates }) => Err(CompileError::elab(
            ErrorKind::ElabAmbiguousCtorAlias,
            format!(
                "构造子名 `{name}` 有歧义：{} 都声明了它；请写全前缀名",
                candidates
                    .iter()
                    .map(|c| format!("`{c}`"))
                    .collect::<Vec<_>>()
                    .join(" 与 ")
            ),
            span,
        )),
        Some(_) => resolve_known(known, name, span),
        None => Err(CompileError::elab(
            ErrorKind::ElabUnknownConstant,
            format!("unknown constant `{name}`"),
            span,
        )),
    }
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
    /// Index count (`num_indices=0` for `Nat`/`Bool`/non-indexed inductives).
    pub num_indices: usize,
    /// Index binder source types in declaration order (for `match` motives).
    pub index_types: Vec<Expr>,
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
    known: &mut KnownTable,
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
    // 带索引归纳：`ty` 的 Pi 望远镜（本编译器把参数也一并记在这里，见
    // `derive_recursor` 的索引命名与 `recursor_telescope`；`num_indices` 的
    // 口径以既有行为为准，勿与内核的 `local_indices` 混为一谈）。
    let index_binders = result_chain_binders(ty);
    let num_indices = u16::try_from(index_binders.len()).map_err(|_| {
        CompileError::elab(
            ErrorKind::ElabTooManyBinders,
            "too many inductive indices",
            ty.span(),
        )
    })?;
    // K 目标标志由块形状唯一决定，**显式 rec 与派生 rec 必须给同一个值**：
    // 内核断言 `rd.is_k == st.k_target`（`kernel/src/inductive.rs:662`）。
    let is_k = is_k_target(ty, constructors);
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
    // `ind_ty_src` 供 `derive_recursor` 判「块是不是 Prop / 有没有索引」——
    // 那是**源级**问题，必须用参数未剥离的源类型。
    let ind_ty_src = if params.is_empty() {
        ty.clone()
    } else {
        Expr::Forall {
            binders: params.to_vec(),
            body: Box::new(ty.clone()),
            span: ty.span(),
        }
    };
    let kernel_ind_ty = elab_expr(
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
    // R1：安装名（规范名）= 内核里的构造子名。源名 `c.name` 仍用于源级匹配
    // （`iota` 规则、`match` 分支），别名（R2）只在解析层。
    let ctor_canonical: Vec<String> = constructors
        .iter()
        .map(|c| canonical_ctor_name(name, &c.name))
        .collect();
    let ctor_names: Vec<NamePtr<'a>> = ctor_canonical
        .iter()
        .map(|canonical| builder.name_from_str(canonical))
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
                ty: kernel_ind_ty,
            },
            is_recursive,
            num_params,
            num_indices,
            Arc::from([ind_name]),
            Arc::from(ctor_names.clone()),
        )
        .map_err(|e| CompileError::elab(ErrorKind::ElabDuplicateDeclaration, e, Span::default()))?;
    built.push(ind_declar);
    known.insert(
        name.to_string(),
        KnownName::Decl {
            universes: Vec::new(),
        },
    );

    let mut match_ctors: Vec<MatchCtor<'a>> = Vec::with_capacity(constructors.len());
    // 每个构造子已 elaborate 的**内核** Pi 望远镜（`params ++ fields`）——派生
    // recursor 的 large-elimination 判据要读它（见 `kernel_large_elim_test`）。
    let mut kernel_ctor_tys: Vec<ExprPtr<'a>> = Vec::with_capacity(constructors.len());
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
            canonical: ctor_canonical[idx].clone(),
            fields,
        });
        kernel_ctor_tys.push(ctor_ty);
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
        // R1：规范名进解析表；R2：裸名作为**别名键**（唯一才可解析）。
        known.insert(
            ctor_canonical[idx].clone(),
            KnownName::Decl {
                universes: Vec::new(),
            },
        );
        insert_ctor_alias(known, &ctor.name, &ctor_canonical[idx]);
    }

    // 显式 rec 优先：源里有 rec 时零行为变化；无 rec 时自动派生等价的
    // RecDecl + iota 规则（py-nat 手写版同构），再走同一条 elab 路径。
    //
    // 派生**推迟到这里**（ctor 类型 elaborate 之后，策略 A）：recursor 要不要
    // 额外宇宙参数，由内核 `large_elim_test` 的镜像判据决定，而它要读每个构造子
    // 已 elaborate 的**内核**字段类型与结果实参（G-03 / WO-006，设计
    // docs/design/prop-large-elim-mirror.md §3）。
    let owned_rec;
    let owned_rules;
    let (recursor, iota_rules): (&RecDecl, &[crate::IotaRule]) = match recursor {
        Some(rec) => (rec, iota_rules),
        None => {
            // `ty` 是**源级结果排序**（`Prop` / `A -> Prop`），参数不在其中：
            // `derive_recursor` 的索引望远镜与 `is_prop_block_ty` 都以此为口径。
            let block_is_prop = is_prop_block_ty(ty);
            let wants_u = large_elim_test_mirror(
                &elab_ctx,
                builder,
                params,
                name,
                constructors,
                &kernel_ctor_tys,
                block_is_prop,
            );
            let (rec, rules) =
                derive_recursor(name, params, ty, constructors, &ctor_canonical, wants_u);
            owned_rec = rec;
            owned_rules = rules;
            (&owned_rec, &owned_rules)
        }
    };

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
        known.insert(
            rec.name.clone(),
            KnownName::Decl {
                universes: known_rec_universes.clone(),
            },
        );

        let mut rules = Vec::with_capacity(iota_rules.len());
        for rule in iota_rules {
            // R3：显式 `iota` 规则按**源名**匹配（`iota zero :=` 里的 `zero`）；
            // 派生规则带的是规范名，两种拼写都命中。迁移轮把 `ctor zero` 改写成
            // `ctor Nat.zero` 时，同文件的 `iota` 也必须跟着写点名前缀。
            let ctor_idx = constructors
                .iter()
                .enumerate()
                .position(|(i, c)| c.name == rule.ctor_name || ctor_canonical[i] == rule.ctor_name)
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
            num_indices,
            num_motives: 1,
            num_minors: constructors.len() as u16,
            rec_rules: Arc::from(rules),
            is_k,
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
            num_indices: index_binders.len(),
            index_types: index_binders
                .iter()
                .map(|b| {
                    b.ty.as_deref()
                        .cloned()
                        .unwrap_or(Expr::Hole { span: b.span })
                })
                .collect(),
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
    known: &KnownTable,
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
    known: &KnownTable,
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
    known: &KnownTable,
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
    known: &KnownTable,
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

/// 记法展开（G-04 / WO-011，设计 N4）：把记号节点降级成 `App` 形状。
///
/// 目标 telescope 比操作数多出来的**前导参数**（`Set.mem (α : Type) …` 的
/// `α`）按固定顺序补全：
///
/// 1. 由**操作数**解出：第 2 个参数的类型就是裸变量 `α` ⇒ `α := typeof(a)`；
/// 2. 无操作数时由**期望类型**解出：`Set.empty : (α) → Set α` 对上期望
///    `Set α₀` ⇒ `α := α₀`。
///
/// 解不出 ⇒ `elab-notation-argument-unsolved`（hint 教点名写法）。
/// 目标名不存在 ⇒ `elab-notation-unknown-target`。
///
/// **补全只发生在这条路径上**：点名写法（`Set.mem a A`，省 `α`）继续被内核
/// 拒绝（设计 N4.3 的护城河）。
#[allow(clippy::too_many_arguments)]
fn elab_notation<'a>(
    builder: &mut EnvBuilder<'a>,
    symbol: &str,
    target: &str,
    operands: &[&Expr],
    span: Span,
    scope: &mut ElabScope<'a>,
    univ: &UnivMap<'a>,
    known: &KnownTable,
    hovers: &mut Vec<HoverNode<'a>>,
    expected_src: Option<&Expr>,
    ctx: &ElabCtx<'a, '_>,
) -> Result<ExprPtr<'a>, CompileError> {
    let canonical = resolve_known(known, target, span).map_err(|_| {
        CompileError::elab(
            ErrorKind::ElabNotationUnknownTarget,
            format!("记法 `{symbol}` 指向的目标 `{target}` 不存在：检查记法命令里的名字（要写点名，例如 Set.mem）"),
            span,
        )
    })?;
    // 目标自身的签名（`forall (α : Type 0), α -> Set α -> Prop`）由内核 pp
    // 渲染：与 `judge_infer` 读 `apply` 的函数类型同一条路（不做文本比对）。
    let binders = scope.judge_binders();
    let target_text = render_expr(&Expr::Ident {
        name: canonical.clone(),
        span,
    });
    let signature =
        judge_infer(ctx.prefix_src, ctx.options, &binders, &target_text).map_err(|j| {
            CompileError::elab(
                ErrorKind::ElabNotationUnknownTarget,
                format!(
                    "读不到记法 `{symbol}` 的目标 `{target}` 的类型：{}",
                    judgement_message(&j)
                ),
                span,
            )
        })?;
    let Some(prefix_args) = notation_prefix_args(&signature, operands, expected_src, ctx, scope)?
    else {
        return Err(CompileError::elab(
            ErrorKind::ElabNotationArgumentUnsolved,
            format!(
                "记法 `{symbol}` 展开成 `{target}` 时补不出前面的类型参数：请写出点名形式（例如 {target} α …）"
            ),
            span,
        ));
    };
    // 操作数的**期望类型**：`∅ ⊆ A` 里的 `∅` 自己也是零元记法，只有拿到
    // 「这里是 `Set α`」才知道补什么（设计 N4.2 ② 在嵌套位置上的同一规则）。
    // 期望类型文本由内核 pp 给出（`judge_infer` 读目标签名），再按前导参数
    // 的实例代换。
    let operand_expected = notation_operand_expected(&signature, &prefix_args, operands.len());
    // 源到源拼出完整应用，再交给**既有** elaborate 路径——类型错、`@`、
    // 宇宙参数等语义一字不改地复用。
    let const_name = builder.name_from_str(&canonical);
    let levels = builder.alloc_levels_slice(&[]);
    let mut app = builder.mk_const(const_name, levels);
    for arg in &prefix_args {
        let arg = elab_expr(builder, arg, scope, univ, known, hovers, None, None, ctx)?;
        app = builder.mk_app(app, arg);
    }
    for (i, operand) in operands.iter().enumerate() {
        let expected_src = operand_expected.get(i).and_then(|t| t.as_ref());
        let operand = elab_expr(
            builder,
            operand,
            scope,
            univ,
            known,
            hovers,
            None,
            expected_src,
            ctx,
        )?;
        app = builder.mk_app(app, operand);
    }
    Ok(app)
}

fn judgement_message(j: &crate::judge::Judgement) -> String {
    match j {
        crate::judge::Judgement::Error { message, .. } => message.clone(),
        crate::judge::Judgement::Mismatch { expected, actual } => {
            format!("期望 `{expected}`，实际是 `{actual}`")
        }
        crate::judge::Judgement::Match => "类型推断没有给出类型".to_string(),
    }
}

/// 补出目标 telescope 的**前导参数**（设计 N4.2 的裸变量匹配）。
///
/// 返回 `None` ⇒ 补不出（调用方报 `elab-notation-argument-unsolved`）。
/// 返回 `Some(vec![])` ⇒ 不需要补（目标参数个数正好等于操作数个数）。
///
/// 两条求解路径，都是同一个**头部匹配 + 从实参位提取裸变量**：
///
/// ① **由操作数解出**：找第一个 `j > i` 且域里提到参数名 `n` 的 binder，
///    把它的域与「第 j 个 binder 对应的那个操作数的类型」头部匹配
///    （`Set.mem` 的第 2 个参数域是裸变量 `α` ⇒ `α := typeof(a)`；
///    `Set.subset` 的第 2 个参数域是 `Set α`、`typeof(A) = Set α₀`
///    ⇒ `α := α₀`）。
/// ② **由期望类型解出**（零操作数时唯一的路）：把已解出的参数代进 telescope
///    剩余部分，与期望类型头部匹配（`Set.empty : (α) → Set α` 对上
///    `Set α₀` ⇒ `α := α₀`）。
fn notation_prefix_args(
    signature: &str,
    operands: &[&Expr],
    expected_src: Option<&Expr>,
    ctx: &ElabCtx<'_, '_>,
    scope: &ElabScope<'_>,
) -> Result<Option<Vec<Expr>>, CompileError> {
    let Ok(sig) = crate::proof::parse_expr_text(signature) else {
        return Ok(None);
    };
    let mut layers: Vec<(String, Expr)> = Vec::new();
    let mut result = sig;
    while let Some(pi) = crate::spine::peel_pi(&result) {
        layers.push((pi.name, pi.domain));
        result = pi.body;
    }
    // 操作数对齐到**最后** `operands.len()` 个参数；多出来的前导参数要补。
    let Some(missing) = layers.len().checked_sub(operands.len()) else {
        // 操作数比参数还多：交给既有应用路径报错（elab/kernel 的既有诊断）。
        return Ok(Some(Vec::new()));
    };
    if missing == 0 {
        return Ok(Some(Vec::new()));
    }
    // 只在参数是**显式**形态时补：隐式 binder（`{α : Type}`）在 v1 的展开里
    // 不插实参（语言不插入隐式实参，设计 §1 第 3 条）。
    let mut solved: Vec<Expr> = Vec::with_capacity(missing);
    for i in 0..missing {
        let name = layers[i].0.clone();
        if name.is_empty() {
            return Ok(None);
        }
        // ① 由操作数解出：第一个提到 `name` 的**后续** binder 的域。
        let mut arg: Option<Expr> = None;
        for (j, layer) in layers.iter().enumerate().skip(i + 1) {
            let Some(operand) = operands.get(j - missing) else {
                continue;
            };
            if !mentions_ident(&layer.1, &name) {
                continue;
            }
            let Some(actual) = infer_type_text(ctx, scope, operand)
                .and_then(|text| crate::proof::parse_expr_text(&text).ok())
            else {
                continue;
            };
            if let Some(found) = unify_extract(&layer.1, &actual, &name) {
                arg = Some(found);
                break;
            }
        }
        // ② 由期望类型解出：把已解出的参数代入 telescope 剩余部分。
        if arg.is_none() {
            if let Some(expected) = expected_src {
                let rest = substitute_prefix_params(&layers, &solved, i, &result);
                arg = unify_extract(&rest, expected, &name);
            }
        }
        let Some(arg) = arg else {
            return Ok(None);
        };
        solved.push(arg);
    }
    Ok(Some(solved))
}

/// 目标 telescope 里**操作数位**的期望类型（源级 AST），按已解出的前导参数
/// 代换：`Set.mem : (α) → (a : α) → (A : Set α) → Prop`、`α := Nat`
/// ⇒ `[Nat, Set Nat]`。解析不出时该位为 `None`（操作数照旧无期望类型地
/// elaborate，与今天的行为一致——绝不比既有路径差）。
fn notation_operand_expected(
    signature: &str,
    prefix_args: &[Expr],
    operand_count: usize,
) -> Vec<Option<Expr>> {
    let mut out: Vec<Option<Expr>> = vec![None; operand_count];
    let Ok(sig) = crate::proof::parse_expr_text(signature) else {
        return out;
    };
    let mut layers: Vec<(String, Expr)> = Vec::new();
    let mut result = sig;
    while let Some(pi) = crate::spine::peel_pi(&result) {
        layers.push((pi.name, pi.domain));
        result = pi.body;
    }
    let Some(missing) = layers.len().checked_sub(operand_count) else {
        return out;
    };
    let mut sigma: HashMap<String, Expr> = HashMap::new();
    for (k, arg) in prefix_args.iter().enumerate() {
        if let Some((name, _)) = layers.get(k) {
            sigma.insert(name.clone(), arg.clone());
        }
    }
    for (i, slot) in out.iter_mut().enumerate() {
        let Some((_, domain)) = layers.get(missing + i) else {
            continue;
        };
        *slot = Some(super::goals::substitute_names(
            domain,
            &sigma,
            &HashMap::new(),
        ));
    }
    out
}

/// 把 `solved` 里已经解出的前导参数代入 telescope 的第 `i+1` 个 binder 起
/// 的剩余部分（**不含**正在求解的第 `i` 层：那一层正是要被消掉的），得到「结果类型」模板：`(α : Type) → Set α` 代入 `α := α₀`
/// ⇒ `Set α₀`。
fn substitute_prefix_params(
    layers: &[(String, Expr)],
    solved: &[Expr],
    i: usize,
    result: &Expr,
) -> Expr {
    let mut sigma: HashMap<String, Expr> = HashMap::new();
    for (k, arg) in solved.iter().enumerate() {
        sigma.insert(layers[k].0.clone(), arg.clone());
    }
    let mut rest = result.clone();
    for (name, domain) in layers[(i + 1).min(layers.len())..].iter().rev() {
        rest = Expr::Forall {
            binders: vec![Binder {
                name: name.clone(),
                ty: Some(Box::new(super::goals::substitute_names(
                    domain,
                    &sigma,
                    &HashMap::new(),
                ))),
                style: BinderKind::Explicit,
                span: Span::default(),
            }],
            body: Box::new(rest),
            span: Span::default(),
        };
    }
    super::goals::substitute_names(&rest, &sigma, &HashMap::new())
}

/// 问内核要 `operand` 在**当前 binder 上下文**里的类型文本（与 `apply`
/// 读被应用函数类型同一条路：`judge_infer`，不做文本比对）。
fn infer_type_text(ctx: &ElabCtx<'_, '_>, scope: &ElabScope<'_>, operand: &Expr) -> Option<String> {
    let binders = scope.judge_binders();
    judge_infer(ctx.prefix_src, ctx.options, &binders, &render_expr(operand)).ok()
}

/// 头部匹配 + 提取裸变量：`template` 是 `name` 本身 ⇒ 取 `actual`；两者是
/// 同头、同实参个数的应用链且某个实参位恰好是裸变量 `name` ⇒ 取 `actual`
/// 对应位的实参。其余形状返回 `None`（v1 不做一般合一，设计 N4.2）。
fn unify_extract(template: &Expr, actual: &Expr, name: &str) -> Option<Expr> {
    if let Expr::Ident { name: n, .. } = template {
        if n == name {
            return Some(actual.clone());
        }
    }
    let (head, template_args) = crate::spine::spine_of(template);
    let (actual_head, actual_args) = crate::spine::spine_of(actual);
    let Expr::Ident {
        name: head_name, ..
    } = head
    else {
        return None;
    };
    let Expr::Ident {
        name: actual_head_name,
        ..
    } = actual_head
    else {
        return None;
    };
    if head_name != actual_head_name || template_args.len() != actual_args.len() {
        return None;
    }
    for (template_arg, actual_arg) in template_args.iter().zip(actual_args.iter()) {
        if let Expr::Ident { name: n, .. } = template_arg {
            if n == name {
                return Some((*actual_arg).clone());
            }
        }
    }
    None
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
    known: &KnownTable,
    hovers: &mut Vec<HoverNode<'a>>,
    expected: Option<ExprPtr<'a>>,
    expected_src: Option<&Expr>,
    ctx: &ElabCtx<'a, '_>,
) -> Result<ExprPtr<'a>, CompileError> {
    // Lambda-headed application with untyped binders: infer the binder types
    // from the arguments' types (non-dependent case, I6). Source-to-source
    // rewrite, then the normal path elaborates the annotated lambda.
    if let Some(rewritten) = annotate_application_lambda(expr, ctx, scope) {
        return elab_expr(
            builder,
            &rewritten,
            scope,
            univ,
            known,
            hovers,
            expected,
            expected_src,
            ctx,
        );
    }
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
                    // R1/R2：裸名别名解析到**规范名**——只往 `known` 里加一个
                    // 裸名键会造出第二个内核常量（`mk` ≠ `P1.mk`）。
                    let canonical = resolve_known(known, name, *span)?;
                    // The defining command's span is backfilled in `run_pass`
                    // (placeholder survives until then; prelude names resolve
                    // to no source definition and drop the record there).
                    let target = ResolvedTarget::Declaration {
                        name: canonical.clone(),
                        span: Span::default(),
                    };
                    let params = known[&canonical].universes();
                    let levels: Vec<LevelPtr<'a>> = params.iter().map(|_| builder.zero()).collect();
                    let levels = builder.alloc_levels_slice(&levels);
                    let name = builder.name_from_str(&canonical);
                    (builder.mk_const(name, levels), Some(target))
                }
            };
            record_hover(hovers, scope, *span, out, resolution);
            Ok(out)
        }
        Expr::UniverseApp { name, levels, span } => {
            let canonical = resolve_known_constant(known, name, *span)?;
            let params = known[&canonical].universes();
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
            let name = builder.name_from_str(&canonical);
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
        // 记号节点（G-04 / WO-011，设计 N4）：源到源降级成 `App` 形状。
        // 目标 telescope 比操作数多出来的**前导参数**由操作数类型 / 期望类型
        // 解出（裸变量匹配）；补全只发生在这条路径上——点名写法省参数**仍然
        // 被内核拒绝**（设计 N4.3 的护城河）。
        Expr::Notation {
            symbol,
            target,
            lhs,
            rhs,
            span,
            ..
        } => {
            let operands: Vec<&Expr> = [lhs.as_deref(), rhs.as_deref()]
                .into_iter()
                .flatten()
                .collect();
            let out = elab_notation(
                builder,
                symbol,
                target,
                &operands,
                *span,
                scope,
                univ,
                known,
                hovers,
                expected_src,
                ctx,
            )?;
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
            // 带索引归纳：scrutinee 书写类型里参数之后是索引实参（如
            // `v : Vec A n` 的 `n`）；motive/recursor 应用都要用它们。
            let index_args_src: Vec<Expr> = if info.num_indices == 0 {
                Vec::new()
            } else {
                let args = written_args.as_ref().ok_or_else(|| {
                    CompileError::elab(
                        ErrorKind::ElabMatchParameterizedUnsupported,
                        format!(
                            "`match` 暂不支持这个带索引归纳形状 `{ind_name}`：被匹配项必须是一个书写类型为 `{ind_name} <参数> <索引>` 的局部变量"
                        ),
                        scrutinee.span(),
                    )
                })?;
                if args.len() < info.num_params + info.num_indices {
                    return Err(CompileError::elab(
                        ErrorKind::ElabMatchParameterizedUnsupported,
                        format!(
                            "`match` 暂不支持这个带索引归纳形状 `{ind_name}`：被匹配项的书写类型需要显式给出 {} 个参数 + {} 个索引",
                            info.num_params, info.num_indices
                        ),
                        scrutinee.span(),
                    ));
                }
                args.iter()
                    .skip(info.num_params)
                    .take(info.num_indices)
                    .cloned()
                    .collect()
            };
            let index_kernel: Vec<ExprPtr<'a>> = index_args_src
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
            // 2) 模式编译器：canonical 化成「每构造子恰好一条 arm」，处理
            //    通配/绑定/嵌套构造子/Nat 字面量/守卫（docs/design/match-patterns.md §4）。
            let top_vars = vec![ColVar {
                expr: (**scrutinee).clone(),
                ind: Some(info),
                ind_name: Some(ind_name.clone()),
                subst: info
                    .param_names
                    .iter()
                    .cloned()
                    .zip(param_args_src.iter().cloned())
                    .collect(),
            }];
            let top_rows: Vec<PatternRow> = arms
                .iter()
                .map(|arm| PatternRow {
                    pats: vec![arm.pattern.clone()],
                    guard: arm.guard.clone(),
                    body: arm.body.clone(),
                })
                .collect();
            let compiled = compile_pattern_body(ctx, &top_vars, &top_rows, *span)?;
            let Expr::Match {
                arms: canon_arms, ..
            } = compiled
            else {
                // 所有模式都不可反驳（`| _ => …` / `| y => …`）：直接用替换后的 body。
                return elab_expr(
                    builder,
                    &compiled,
                    scope,
                    univ,
                    known,
                    hovers,
                    expected,
                    Some(expected_src),
                    ctx,
                );
            };
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
            // 带索引归纳：motive 先绑索引再绑 major（`Ind params i1…ik`），与内核
            // `mk_motive_dep` 一致。索引名新鲜、类型代入参数实参。
            let param_map: HashMap<String, Expr> = info
                .param_names
                .iter()
                .cloned()
                .zip(param_args_src.iter().cloned())
                .collect();
            let mut index_binder_srcs: Vec<(String, Expr)> = Vec::new();
            for (k, ty) in info.index_types.iter().enumerate() {
                let name = {
                    let mut candidate = format!("__soko_i{k}");
                    while scope.names.iter().any(|n| n == &candidate) {
                        candidate.push('_');
                    }
                    candidate
                };
                let ty = super::goals::substitute_names(ty, &param_map, &HashMap::new());
                let ty_kernel =
                    elab_expr(builder, &ty, scope, univ, known, hovers, None, None, ctx)?;
                scope.push(name.clone(), ty_kernel, Some(ty.clone()), *span);
                index_binder_srcs.push((name, ty));
            }
            // major 的书写源类型 = `Ind <参数实参> <索引名…>`。
            let ind_ty_src =
                index_binder_srcs
                    .iter()
                    .fold(ind_ty_src, |acc, (name, _)| Expr::App {
                        fun: Box::new(acc),
                        arg: Box::new(Expr::Ident {
                            name: name.clone(),
                            span: *span,
                        }),
                        span: *span,
                    });
            let ind_kernel = elab_expr(
                builder,
                &ind_ty_src,
                scope,
                univ,
                known,
                hovers,
                None,
                None,
                ctx,
            )?;
            scope.push(motive_name.clone(), ind_kernel, Some(ind_ty_src), *span);
            let motive_body = elab_expr(
                builder, &body_src, scope, univ, known, hovers, None, None, ctx,
            )?;
            scope.truncate(outer);
            let motive_name_ptr = if motive_name.is_empty() {
                builder.anonymous()
            } else {
                builder.name_from_str(&motive_name)
            };
            let mut motive = builder.mk_lambda(
                motive_name_ptr,
                BinderStyle::Default,
                ind_kernel,
                motive_body,
            );
            for (name, ty) in index_binder_srcs.iter().rev() {
                let ty_kernel =
                    elab_expr(builder, ty, scope, univ, known, hovers, None, None, ctx)?;
                let name_ptr = builder.name_from_str(name);
                motive = builder.mk_lambda(name_ptr, BinderStyle::Default, ty_kernel, motive);
            }
            // 5) minors：按构造子声明序重排；**递归字段后插入归纳假设 IH**
            //    （类型 = motive 结果 R，v1 非依赖 motive；design §5 / Phase 2）。
            let base = scope.len();
            let mut minors = Vec::with_capacity(info.ctors.len());
            for (ctor, canon) in info.ctors.iter().zip(canon_arms.iter()) {
                let arm = canon;
                // canonical arm 的参数就是该构造子的字段绑定（顺序与 fields 对齐）。
                let binders: Vec<Binder> = match &arm.pattern {
                    Pattern::Ident { args, .. } => args
                        .iter()
                        .map(|a| Binder {
                            name: match a {
                                Pattern::Ident { name, .. } => name.clone(),
                                _ => String::new(),
                            },
                            ty: None,
                            style: BinderKind::Explicit,
                            span: a.span(),
                        })
                        .collect(),
                    _ => Vec::new(),
                };
                let mut minor_binders: Vec<(String, BinderStyle, ExprPtr<'a>, Option<Expr>)> =
                    Vec::new();
                // 字段原名 → 用户模式里的绑定名：后面的字段类型可能引用前面的
                // 字段（`v : Vec A n` 引用 `n`），elaborate 前必须改名。
                let mut field_rename: HashMap<String, Expr> = HashMap::new();
                let param_map: HashMap<String, Expr> = info
                    .param_names
                    .iter()
                    .cloned()
                    .zip(param_args_src.iter().cloned())
                    .collect();
                for (field, binder) in ctor.fields.iter().zip(binders.iter()) {
                    // 参数化归纳：把字段源类型里的参数名代换成 scrutinee 的
                    // 书写实参后再 elaborate，得到该构造子在其实例下的字段类型。
                    let parameterized = info.num_params > 0;
                    let renamed_src = field.src_ty.as_ref().map(|src| {
                        let mut map = param_map.clone();
                        map.extend(field_rename.clone());
                        super::goals::substitute_names(src, &map, &HashMap::new())
                    });
                    let (field_ty, field_src_ty): (ExprPtr<'a>, Option<Expr>) = if !parameterized {
                        (field.ty, renamed_src)
                    } else {
                        let src = renamed_src.ok_or_else(|| {
                            CompileError::elab(
                                ErrorKind::ElabMatchParameterizedUnsupported,
                                format!(
                                    "`match` 暂不支持这个参数化归纳形状 `{ind_name}`：构造子 `{}` 的字段缺少源类型",
                                    ctor.name
                                ),
                                binder.span,
                            )
                        })?;
                        let k =
                            elab_expr(builder, &src, scope, univ, known, hovers, None, None, ctx)?;
                        (k, Some(src))
                    };
                    if !field.name.is_empty() && field.name != binder.name {
                        field_rename.insert(
                            field.name.clone(),
                            Expr::Ident {
                                name: binder.name.clone(),
                                span: binder.span,
                            },
                        );
                    }
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
                    for b in &binders {
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
            for index in &index_kernel {
                app = builder.mk_app(app, *index);
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

// ---- 应用位置的 lambda binder 类型推断（I6，非依赖）----

/// Rewrite a lambda-headed application whose leading binders lack annotations
/// by inferring those binder types from the argument types (kernel-backed
/// `judge_infer`, design `docs/design/elaborator-let-match.md`). Handles the
/// curried case `(fun x y => …) a b`. Returns `None` when the shape or the
/// query does not apply — the normal path then reports the untyped binder.
fn annotate_application_lambda(expr: &Expr, ctx: &ElabCtx, scope: &ElabScope) -> Option<Expr> {
    // Flatten the spine `f a1 … an` (leftmost non-App is the head).
    let mut args: Vec<&Expr> = Vec::new();
    let mut head = expr;
    while let Expr::App { fun, arg, .. } = head {
        args.push(arg);
        head = fun;
    }
    if args.is_empty() {
        return None;
    }
    args.reverse();
    let Expr::Lambda {
        binders,
        body,
        span,
    } = head
    else {
        return None;
    };
    let untyped: Vec<usize> = binders
        .iter()
        .enumerate()
        .filter(|(_, b)| b.ty.is_none())
        .map(|(i, _)| i)
        .collect();
    if untyped.is_empty() || args.len() < untyped.len() {
        return None;
    }
    let judge = scope.judge_binders();
    let mut new_binders = binders.clone();
    for (n, &i) in untyped.iter().enumerate() {
        let text = judge_infer(ctx.prefix_src, ctx.options, &judge, &render_expr(args[n])).ok()?;
        let ty = crate::proof::parse_expr_text(&text).ok()?;
        new_binders[i].ty = Some(Box::new(ty));
    }
    let mut rebuilt = Expr::Lambda {
        binders: new_binders,
        body: body.clone(),
        span: *span,
    };
    for arg in &args {
        rebuilt = Expr::App {
            fun: Box::new(rebuilt),
            arg: Box::new((*arg).clone()),
            span: expr.span(),
        };
    }
    Some(rebuilt)
}

// ---- 模式编译器（docs/design/match-patterns.md §4）----
//
// 把用户写的 `| <pattern> [if <guard>] => body`（可能含通配、绑定、嵌套构造子、
// Nat 字面量）**源到源** canonical 化成「每构造子恰好一条 arm、参数全是绑定」
// 的 `Expr::Match` 树；嵌套匹配生成在该 arm 的 body 里，交回本模块既有的
// lowering 逐层处理。好处：不手搓 de Bruijn，守卫复用 prelude `Bool.rec`。

/// 待匹配的一列（顶层是用户写的 scrutinee；嵌套是字段 binder 名）。
#[derive(Clone)]
struct ColVar<'c, 'a> {
    expr: Expr,
    /// 该列类型的归纳元数据（`None` = 参数/未知 → 只能绑定或通配）。
    ind: Option<&'c InductiveInfo<'a>>,
    ind_name: Option<String>,
    /// 该列类型的参数实例（参数名 → 书写实参），用于把字段类型 `A` 代换成
    /// `Option Nat` 里的 `Nat`（否则嵌套模式看不到内层归纳）。
    subst: HashMap<String, Expr>,
}

/// 编译器的一行：模式串（长度 = 列数）+ body + 守卫。
#[derive(Clone)]
struct PatternRow {
    pats: Vec<Pattern>,
    guard: Option<Expr>,
    body: Expr,
}

/// 模式在某一列的解析结果。
enum Resolved {
    Ctor { ci: usize, args: Vec<Pattern> },
    Bind(String),
    Wild,
}

fn bad_arm(message: &str, span: Span) -> CompileError {
    CompileError::elab(ErrorKind::ElabMatchBadArm, message.to_string(), span)
}

fn ident_expr(name: &str, span: Span) -> Expr {
    Expr::Ident {
        name: name.to_string(),
        span,
    }
}

/// 错误消息里列出的构造子拼写：源名与规范名都给（R1 之后源名已不是内核名，
/// 学员按提示写哪一个都能过 —— R3）。
fn ctor_names_text_from(info: &InductiveInfo) -> String {
    info.ctors
        .iter()
        .map(|c| {
            if c.name == c.canonical {
                format!("`{}`", c.name)
            } else {
                format!("`{}`（或 `{}`）", c.canonical, c.name)
            }
        })
        .collect::<Vec<_>>()
        .join("、")
}

/// 构造子下标：源名（`succ`，R3 的源级写法）、规范名（`Nat.succ`，R1）与
/// 该归纳内唯一的裸后缀都命中。
fn ctor_index(info: &InductiveInfo, name: &str) -> Option<usize> {
    if let Some(i) = info.ctors.iter().position(|c| c.name == name) {
        return Some(i);
    }
    if let Some(i) = info.ctors.iter().position(|c| c.canonical == name) {
        return Some(i);
    }
    if name.contains('.') {
        return None;
    }
    let hits: Vec<usize> = info
        .ctors
        .iter()
        .enumerate()
        .filter(|(_, c)| c.canonical.rsplit('.').next() == Some(name))
        .map(|(i, _)| i)
        .collect();
    if hits.len() == 1 {
        Some(hits[0])
    } else {
        None
    }
}

/// Nat 形状：零构造子（0 字段）+ succ 构造子（1 字段，类型回到自身）。
fn nat_shape(info: &InductiveInfo, name: &str) -> Option<(usize, usize)> {
    let zero = info.ctors.iter().position(|c| c.fields.is_empty())?;
    let succ = info.ctors.iter().position(|c| {
        c.fields.len() == 1
            && c.fields[0].src_ty.as_ref().and_then(head_ident).as_deref() == Some(name)
    })?;
    Some((zero, succ))
}

fn resolve_pattern(pat: &Pattern, col: &ColVar) -> Result<Resolved, CompileError> {
    match pat {
        Pattern::Wild { .. } => Ok(Resolved::Wild),
        Pattern::Num { value, span } => {
            let info = col.ind.ok_or_else(|| {
                bad_arm(
                    "数字字面量模式只能用在 Nat 上：这一列不是已知的归纳类型",
                    *span,
                )
            })?;
            let name = col.ind_name.as_deref().unwrap_or("");
            let (zero, succ) = nat_shape(info, name).ok_or_else(|| {
                bad_arm(
                    "数字字面量模式只能用在 Nat 上（0 元零构造子 + 一元 succ 构造子）",
                    *span,
                )
            })?;
            let n: u64 = value.parse().unwrap_or(0);
            if n == 0 {
                Ok(Resolved::Ctor {
                    ci: zero,
                    args: Vec::new(),
                })
            } else {
                Ok(Resolved::Ctor {
                    ci: succ,
                    args: vec![Pattern::Num {
                        value: (n - 1).to_string(),
                        span: *span,
                    }],
                })
            }
        }
        Pattern::Ident { name, args, span } => {
            if let Some(info) = col.ind {
                if let Some(ci) = ctor_index(info, name) {
                    let want = info.ctors[ci].fields.len();
                    if args.len() != want {
                        return Err(bad_arm(
                            &format!(
                                "构造子 `{}` 有 {} 个字段，但这一支写了 {} 个子模式；请写满字段",
                                info.ctors[ci].canonical,
                                want,
                                args.len()
                            ),
                            *span,
                        ));
                    }
                    return Ok(Resolved::Ctor {
                        ci,
                        args: args.clone(),
                    });
                }
            }
            if args.is_empty() {
                return Ok(Resolved::Bind(name.clone()));
            }
            let ty = col.ind_name.clone().unwrap_or_default();
            let available = col.ind.map(ctor_names_text_from).unwrap_or_default();
            Err(bad_arm(
                &format!("`{ty}` 没有构造子 `{name}`；可用的是：{available}"),
                *span,
            ))
        }
    }
}

/// 守卫链：`| p if g1 => b1 | …` 在「模式都已匹配」后按顺序判定，第一个为真
/// 的 body 胜；为假落到下一行；最后一行若仍有守卫 → 没有兜底。
fn guard_chain(rows: &[PatternRow], span: Span) -> Result<Expr, CompileError> {
    let Some(row) = rows.first() else {
        return Err(CompileError::elab(
            ErrorKind::ElabMatchNonExhaustive,
            "`match` 的守卫为假时没有兜底分支：请在后面补一条不加守卫的分支".to_string(),
            span,
        ));
    };
    match &row.guard {
        None => Ok(row.body.clone()),
        Some(g) => {
            let fallback = guard_chain(&rows[1..], span)?;
            let gspan = g.span();
            Ok(Expr::Match {
                scrutinee: Box::new(g.clone()),
                arms: vec![
                    MatchArm {
                        pattern: Pattern::Ident {
                            name: "Bool.true".to_string(),
                            args: Vec::new(),
                            span: gspan,
                        },
                        guard: None,
                        body: row.body.clone(),
                        span: gspan,
                    },
                    MatchArm {
                        pattern: Pattern::Ident {
                            name: "Bool.false".to_string(),
                            args: Vec::new(),
                            span: gspan,
                        },
                        guard: None,
                        body: fallback,
                        span: gspan,
                    },
                ],
                span,
            })
        }
    }
}

/// 子模式的列变量：字段的书写类型给出嵌套归纳。
fn field_col<'c, 'a>(
    name: &str,
    src_ty: Option<&Expr>,
    parent_subst: &HashMap<String, Expr>,
    ctx: &'c ElabCtx<'a, '_>,
) -> ColVar<'c, 'a> {
    // 参数化归纳：字段类型里的参数名先代入（`some (a : A)` 在 `Option Nat`
    // 下 → `Nat`），嵌套模式才能解析内层构造子。
    let substituted =
        src_ty.map(|t| super::goals::substitute_names(t, parent_subst, &HashMap::new()));
    let spine = substituted.as_ref().and_then(src_spine);
    let head = spine.as_ref().map(|(h, _)| h.clone());
    let args = spine.map(|(_, a)| a).unwrap_or_default();
    let ind = head.as_deref().and_then(|h| ctx.inductives.get(h));
    let subst = match ind {
        Some(info) => info
            .param_names
            .iter()
            .cloned()
            .zip(args)
            .collect::<HashMap<String, Expr>>(),
        None => HashMap::new(),
    };
    ColVar {
        expr: ident_expr(name, Span::default()),
        ind,
        ind_name: head,
        subst,
    }
}

/// 编译一层的 `match`（`vars` 都已在作用域里），返回一个 body 表达式
/// （可能是生成出来的嵌套 `Expr::Match`，也可能直接就是叶子 body）。
fn compile_pattern_body<'c, 'a>(
    ctx: &'c ElabCtx<'a, '_>,
    vars: &[ColVar<'c, 'a>],
    rows: &[PatternRow],
    span: Span,
) -> Result<Expr, CompileError> {
    if rows.is_empty() {
        return Err(CompileError::elab(
            ErrorKind::ElabMatchNonExhaustive,
            "`match` 的分支不完整：有些取值没有对应分支".to_string(),
            span,
        ));
    }
    let mut resolved: Vec<Vec<Resolved>> = Vec::with_capacity(rows.len());
    let mut all_irrefutable = true;
    for row in rows {
        let mut this = Vec::with_capacity(vars.len());
        for (j, pat) in row.pats.iter().enumerate() {
            let res = resolve_pattern(pat, &vars[j])?;
            if !matches!(res, Resolved::Bind(_) | Resolved::Wild) {
                all_irrefutable = false;
            }
            this.push(res);
        }
        resolved.push(this);
    }
    if all_irrefutable {
        // 这一层所有模式都不可反驳：顺序 + 守卫决定，无需再造 match。
        return guard_chain(rows, span);
    }
    // 选第一处含可反驳模式的列。
    let col = (0..vars.len())
        .find(|&j| {
            resolved
                .iter()
                .any(|r| matches!(r[j], Resolved::Ctor { .. }))
        })
        .expect("at least one refutable pattern exists");
    let info = vars[col]
        .ind
        .ok_or_else(|| bad_arm("无法确定被匹配类型的归纳信息", span))?;
    let mut arms: Vec<MatchArm> = Vec::with_capacity(info.ctors.len());
    for (ci, ctor) in info.ctors.iter().enumerate() {
        let k = ctor.fields.len();
        // 每个字段的嵌套归纳（用于判定某个子模式是不是构造子）。
        let field_inds: Vec<Option<&InductiveInfo<'a>>> = ctor
            .fields
            .iter()
            .map(|f| f.src_ty.as_ref().and_then(head_ident))
            .map(|head| head.and_then(|h| ctx.inductives.get(&h)))
            .collect();
        // 子模式是「绑定」吗（无子模式，且名字不是该字段类型的构造子）。
        let is_bind_arg = |j: usize, name: &str| -> bool {
            !name.contains('.') && field_inds[j].is_none_or(|ind| ctor_index(ind, name).is_none())
        };
        // 1) 字段名：某行在该字段是「绑定」时优先沿用它的名字（canonical 输入
        //    因此保持名字不变 → 编译幂等）；否则用新鲜名。
        let mut field_names: Vec<String> = Vec::with_capacity(k);
        for j in 0..k {
            let mut chosen: Option<String> = None;
            for (ri, _) in rows.iter().enumerate() {
                if let Resolved::Ctor { ci: rci, args } = &resolved[ri][col] {
                    if *rci == ci {
                        if let Pattern::Ident { name, args: a, .. } = &args[j] {
                            if a.is_empty() && is_bind_arg(j, name) && !field_names.contains(name) {
                                chosen = Some(name.clone());
                                break;
                            }
                        }
                    }
                }
            }
            field_names.push(chosen.unwrap_or_else(|| format!("__soko_m{col}_{ci}_{j}")));
        }
        // 2) 逐行特化（去掉 col，换成该构造子的 k 个子模式）。
        let mut sub_rows: Vec<PatternRow> = Vec::new();
        for (ri, row) in rows.iter().enumerate() {
            let (args, bound_column) = match &resolved[ri][col] {
                Resolved::Ctor { ci: rci, args } if *rci == ci => (args.clone(), None),
                Resolved::Ctor { .. } => continue,
                Resolved::Wild => (
                    vec![
                        Pattern::Wild {
                            span: row.pats[col].span(),
                        };
                        k
                    ],
                    None,
                ),
                Resolved::Bind(name) => (
                    vec![
                        Pattern::Wild {
                            span: row.pats[col].span(),
                        };
                        k
                    ],
                    Some(name.clone()),
                ),
            };
            let mut map: HashMap<String, Expr> = HashMap::new();
            for (j, arg) in args.iter().enumerate() {
                if let Pattern::Ident {
                    name, args: sub, ..
                } = arg
                {
                    if sub.is_empty() && is_bind_arg(j, name) && field_names[j] != *name {
                        map.insert(name.clone(), ident_expr(&field_names[j], arg.span()));
                    }
                }
            }
            if let Some(name) = bound_column {
                map.insert(name, vars[col].expr.clone());
            }
            let mut pats = row.pats.clone();
            pats.splice(col..=col, args.iter().cloned());
            let body = super::goals::substitute_names(&row.body, &map, &HashMap::new());
            let guard = row
                .guard
                .as_ref()
                .map(|g| super::goals::substitute_names(g, &map, &HashMap::new()));
            sub_rows.push(PatternRow { pats, guard, body });
        }
        // 3) 新列：去掉 col、在 col 处插入 k 个字段列。
        let mut next_vars: Vec<ColVar<'c, 'a>> = Vec::with_capacity(vars.len() - 1 + k);
        for (j, v) in vars.iter().enumerate() {
            if j != col {
                next_vars.push(v.clone());
            }
        }
        let field_vars: Vec<ColVar<'c, 'a>> = (0..k)
            .map(|j| {
                field_col(
                    &field_names[j],
                    ctor.fields[j].src_ty.as_ref(),
                    &vars[col].subst,
                    ctx,
                )
            })
            .collect();
        next_vars.splice(col..col, field_vars);
        let body = compile_pattern_body(ctx, &next_vars, &sub_rows, span)?;
        arms.push(MatchArm {
            pattern: Pattern::Ident {
                name: ctor.name.clone(),
                args: field_names
                    .iter()
                    .map(|n| Pattern::Ident {
                        name: n.clone(),
                        args: Vec::new(),
                        span,
                    })
                    .collect(),
                span,
            },
            guard: None,
            body,
            span,
        });
    }
    Ok(Expr::Match {
        scrutinee: Box::new(vars[col].expr.clone()),
        arms,
        span,
    })
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
                    // 模式本身不含表达式（v1）；但守卫与 body 是表达式。
                    arm.guard.as_ref().is_some_and(|g| mentions_ident(g, name))
                        || mentions_ident(&arm.body, name)
                })
        }
        Expr::By { .. } => false, // by 块在 elab 前已被引擎降级为普通表达式
        // 记号节点（G-04 / WO-011）：符号与目标名不是标识符，只走操作数。
        Expr::Notation { lhs, rhs, .. } => {
            lhs.as_deref().is_some_and(|e| mentions_ident(e, name))
                || rhs.as_deref().is_some_and(|e| mentions_ident(e, name))
        }
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
    /// `(field name, telescope, field index arguments)` for each recursive field.
    rec_args: Vec<(String, Vec<Binder>, Vec<Expr>)>,
    /// The ctor's result index arguments (`Vec A (Nat.succ n)` → `[Nat.succ n]`).
    ctor_indices: Vec<Expr>,
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
/// Head + arguments of the *codomain* of a possibly-arrow/forall type — a
/// recursive field's index arguments live under its telescope
/// (`(x : Nat) -> Vec A x` → `Vec A x`).
fn spine_of_codomain(ty: &Expr) -> Option<(String, Vec<Expr>)> {
    let mut cur = ty;
    loop {
        match cur {
            Expr::Arrow { codomain, .. } => cur = codomain,
            Expr::Forall { body, .. } => cur = body,
            _ => return src_spine(cur),
        }
    }
}

fn result_chain_binders(result: &Expr) -> Vec<Binder> {
    chain_binders_after(result, 0)
}

/// The k-th (0-based) Pi binders of a possibly arrow/forall-chained type: the
/// first `skip` are dropped. `A -> B` contributes one anonymous binder for `A`.
fn chain_binders_after(result: &Expr, skip: usize) -> Vec<Binder> {
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
            _ => break,
        }
    }
    out.split_off(skip.min(out.len()))
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
/// Whether this block's recursor is a **K target** (`RecursorData.is_k`) —
/// the front-side mirror of the kernel's `init_k_target`
/// (`crates/kernel/src/inductive.rs:1268-1276`). The kernel asserts
/// `rd.is_k == st.k_target` (`inductive.rs:662`), so a wrong flag makes
/// every derived recursor for such a block get rejected
/// (`recursor declares the wrong k-reduction flag`).
///
/// The kernel's predicate is exactly: the block lives in `Prop` (`is_zero`),
/// it is neither mutual nor nested (exactly one inductive in the block), and
/// `pi_telescope_size(only_ctor.ty) == local_params.len()`. The ctor's kernel
/// type is assembled below as `forall (params ++ fields), result`, so that
/// equality means **the single constructor has no fields of its own** — the
/// result's arrow chain counts as fields too. Mirror it literally:
/// `ctor_field_binders` is the front's side of that same telescope.
///
/// Do **not** approximate this predicate. Two shapes that "look singleton-ish"
/// are *not* K targets and get rejected if flagged: a ctor whose field count
/// merely equals the parameter count (`Both (A B : Prop)` / `mk (a : A)
/// (b : B)`), and — in the other direction — an *indexed* family with a single
/// field-less ctor (`Q : Nat -> Prop` / `q : Q 0`) which **is** a K target.
fn is_k_target(ty: &Expr, constructors: &[CtorDecl]) -> bool {
    let [only_ctor] = constructors else {
        return false;
    };
    is_prop_block_ty(ty) && ctor_field_binders(only_ctor).is_empty()
}

/// allows large elimination when the block is empty or has a single ctor with
/// exclusively Prop-typed fields; a multi-ctor Prop block therefore gets a
/// small-elimination recursor (no universe parameter, motive into `Prop`).
fn is_prop_block_ty(ty: &Expr) -> bool {
    // 带索引归纳的 `ty` 是索引望远镜（`Nat -> … -> Sort`）：先剥到最终 Sort。
    let mut cur = ty;
    loop {
        match cur {
            Expr::Arrow { codomain, .. } => cur = codomain,
            Expr::Forall { body, .. } => cur = body,
            Expr::Sort {
                sort: SortKind::Prop,
                ..
            }
            | Expr::Sort {
                sort: SortKind::Sort(0),
                ..
            } => return true,
            _ => return false,
        }
    }
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

/// The front's mirror of the kernel's `large_elim_test`
/// (`crates/kernel/src/inductive.rs:1201-1223`): does this block's recursor
/// carry an extra universe parameter?
///
/// The kernel *computes* that answer and then asserts the front's derived
/// recursor agrees (`assert_nonnested_recursors_def_eq` → `subst_expr_levels`
/// compares `rec_uparams`). A source-level approximation ("is the field type
/// spelled `Prop`?") therefore becomes a hard rejection wherever the two
/// disagree — the G-03 bug: `inductive Bar (A : Type) : Prop` +
/// `ctor mk (a : A) : Bar A` got a `Sort u` motive from the front while the
/// kernel wanted `Prop`.
///
/// Mirror the kernel literally:
///
/// * a block whose result sort is not `Prop` (`is_nonzero`) eliminates large;
/// * an **empty** Prop block (`[] => true`) eliminates large;
/// * a Prop block with **more than one** constructor does not (`_ => false`);
/// * a single-constructor Prop block asks [`large_elim_test_aux_mirror`].
///
/// Only the last case can disagree with the rule this replaced
/// (`is_prop_block_ty(ty) && constructors.len() > 1`), so the semantic work is
/// confined to it.
fn large_elim_test_mirror<'a>(
    ctx: &ElabCtx,
    builder: &mut EnvBuilder<'a>,
    params: &[Binder],
    name: &str,
    constructors: &[CtorDecl],
    kernel_ctor_tys: &[ExprPtr<'a>],
    block_is_prop: bool,
) -> bool {
    if !block_is_prop {
        // `is_nonzero`: the block lives in `Type <n>` and eliminates large.
        return true;
    }
    debug_assert_eq!(constructors.len(), kernel_ctor_tys.len());
    match (kernel_ctor_tys, constructors) {
        // An empty Prop block eliminates large (`[] => true`).
        ([], _) => true,
        // Exactly one constructor: the kernel's `large_elim_test_aux`.
        ([only], [ctor]) => large_elim_test_aux_mirror(ctx, builder, params, name, ctor, only),
        // More than one constructor: no large elimination.
        _ => false,
    }
}

/// The front's mirror of the kernel's `large_elim_test_aux`
/// (`crates/kernel/src/inductive.rs:1164-1199`) for one constructor.
///
/// The kernel walks the constructor's Pi telescope, skips the first
/// `num_params` binders, and records the de Bruijn *level* of every remaining
/// domain whose sort is not `Prop` (`is_prop_type`). It then asks whether each
/// of those fields, taken as a variable, occurs among the arguments of the
/// constructor's result type (`ind params ++ indices`). A non-`Prop` field that
/// is not one of the inductive's own arguments means the block only eliminates
/// into `Prop`.
///
/// Two kernel facts this mirror must not "improve" on:
///
/// * the subset test is **syntactic** (`unfold_apps` + pointer equality), so a
///   field must *be* the result's own argument: `PA A (ident A a)` does not
///   count as `a` even though `ident A a` reduces to it;
/// * "is this domain a `Prop`" is the kernel's `is_prop_type`, i.e. the sort of
///   the domain is `Sort 0`. Impredicativity is included, so `P -> Q` and
///   `forall (x : Nat), P` are `Prop`-typed (P10/P11) while `A` is not (P1).
fn large_elim_test_aux_mirror<'a>(
    ctx: &ElabCtx,
    builder: &mut EnvBuilder<'a>,
    params: &[Binder],
    name: &str,
    ctor: &CtorDecl,
    ctor_ty: &ExprPtr<'a>,
) -> bool {
    let (domains, result) = peel_pi_telescope(*ctor_ty);
    let num_params = params.len();
    let depth = u16::try_from(domains.len()).expect("constructor telescope exceeds u16");
    // 源级字段（显式 binder ++ 结果箭头链）与内核的 Pi 望远镜逐位同序。
    let src_fields = ctor_field_binders(ctor);
    debug_assert_eq!(domains.len(), num_params + src_fields.len());
    // 内核的 `is_prop_type` 需要「参数 + 前序字段」这个 binder 语境。
    let mut scope = ElabScope::new();
    for (binder, domain) in params.iter().zip(domains.iter()) {
        scope.push(
            binder.name.clone(),
            *domain,
            binder.ty.as_deref().cloned(),
            binder.span,
        );
    }
    let mut non_prop: Vec<ExprPtr<'a>> = Vec::new();
    for (offset, (src, domain)) in src_fields
        .iter()
        .zip(domains[num_params..].iter())
        .enumerate()
    {
        let level = u16::try_from(num_params + offset).expect("telescope level exceeds u16");
        if !field_type_is_prop(ctx, &scope, name, src) {
            non_prop.push(builder.mk_var(depth - 1 - level));
        }
        scope.push(
            src.name.clone(),
            *domain,
            src.ty.as_deref().cloned(),
            src.span,
        );
    }
    let (_, args) = unfold_apps(result);
    non_prop.iter().all(|field| args.contains(field))
}

/// The kernel's `is_prop_type` for one constructor field: does the field's own
/// type live in `Sort 0`?
///
/// * A reference to the block's own inductive is a proposition: the caller only
///   reaches this for a `Prop` block, and the inductive is not yet in the
///   `judge_infer` prefix (it is being defined right now), so this case cannot
///   go to the kernel. A recursive field such as `h : Bar A` is a proof, not
///   data, and must not be mistaken for one.
/// * Everything else is semantic — a `Prop` parameter, `P -> Q`, a `forall`
///   ending in a proposition (impredicativity: `imax(_, 0) == 0`), a **named**
///   `Prop` definition — and is asked of the kernel. A syntactic "does the
///   source say `Prop`" test gets P10/P11/P13/P14 wrong and would trade this
///   assertion for another (`left:0/right:1`).
fn field_type_is_prop(ctx: &ElabCtx, scope: &ElabScope, ind_name: &str, field: &Binder) -> bool {
    let Some(src_ty) = field.ty.as_deref() else {
        // 无类型标注的字段：elaborate 阶段已报 `elab-untyped-binder`，这里按
        // 非 Prop 保守处理，不吞掉内核本该给出的诊断。
        return false;
    };
    if head_ident(src_ty).as_deref() == Some(ind_name) {
        return true;
    }
    field_sort_via_kernel(ctx, scope, src_ty)
}

/// Ask the kernel for the sort of `src_ty` and report whether it is `Prop`.
///
/// `judge_infer` synthesizes `#check fun <binders> => <src_ty>` and compiles it
/// with the real kernel, so this is a kernel verdict rather than a text test —
/// the same oracle `match` uses for its motive level
/// ([`infer_expected_level`]). It correctly answers `Prop` for a `Prop`
/// parameter, for `P -> Q`, and for a **named** `Prop` definition
/// (`def Named : Prop := …`), which a syntactic "does it say `Prop`" check
/// would get wrong and thereby trade this assertion for another.
///
/// A failed query yields `false` (non-`Prop`), keeping the kernel's own
/// diagnostic for the block instead of masking it.
fn field_sort_via_kernel(ctx: &ElabCtx, scope: &ElabScope, src_ty: &Expr) -> bool {
    let term = render_expr(src_ty);
    let mut binders = scope.judge_binders_for(src_ty);
    if binders.is_empty() {
        // 与 `infer_expected_level` 同法：`judge_infer` 要剥掉一层 binder 才能
        // 把答案读成「term 的类型」。
        binders.push(GoalBinderSpec {
            name: "_soko_field_sort".to_string(),
            ty: Some("Prop".to_string()),
        });
    }
    let Ok(text) = judge_infer(ctx.prefix_src, ctx.options, &binders, &term) else {
        return false;
    };
    sort_text_level(&text) == Some(0)
}

/// Peel a Pi telescope into `(domains, body)`, in declaration order.
fn peel_pi_telescope<'a>(mut ty: ExprPtr<'a>) -> (Vec<ExprPtr<'a>>, ExprPtr<'a>) {
    let mut domains = Vec::new();
    loop {
        match &*ty {
            sokonanoda::expr::Expr::Pi {
                binder_type, body, ..
            } => {
                domains.push(*binder_type);
                ty = *body;
            }
            _ => return (domains, ty),
        }
    }
}

/// `f a₀ … aₙ` → `(f, [a₀, …, aₙ])` over elaborated expressions. The kernel's
/// subset test uses its own `TcCtx::unfold_apps`; expressions are hash-consed in
/// the arena, so pointer equality is structural equality here just as there.
fn unfold_apps<'a>(mut e: ExprPtr<'a>) -> (ExprPtr<'a>, Vec<ExprPtr<'a>>) {
    let mut args = Vec::new();
    while let sokonanoda::expr::Expr::App { fun, arg, .. } = *e {
        args.push(arg);
        e = fun;
    }
    args.reverse();
    (e, args)
}

/// Synthesize the recursor declaration and one iota rule per constructor for
/// a block written without `rec`. Every binder name is picked fresh against
/// the names the synthesized terms must reference (inductive, constructors,
/// source fields), so no derived binder can shadow a reference.
///
/// `large_elim` is the **kernel's own** large-elimination verdict for this block
/// ([`large_elim_test_mirror`], a literal mirror of
/// `kernel/src/inductive.rs::large_elim_test`). The kernel asserts that the
/// derived recursor carries exactly the universe parameters it computed
/// (`assert_nonnested_recursors_def_eq` → `subst_expr_levels`), so this flag —
/// not a source-level approximation — decides the recursor's shape.
fn derive_recursor(
    name: &str,
    params: &[Binder],
    ty: &Expr,
    constructors: &[CtorDecl],
    ctor_canonical: &[String],
    large_elim: bool,
) -> (RecDecl, Vec<IotaRule>) {
    let ty_span = ty.span();
    let small_elim = !large_elim;
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
    // 索引望远镜（`ty` 在 params 之外）：给每个索引一个新鲜名字，供 motive/
    // recursor 引用（内核的 motive = `forall indices, Ind params indices -> Sort`）。
    let index_binders: Vec<Binder> = result_chain_binders(ty)
        .into_iter()
        .map(|b| {
            let base = if b.name.is_empty() { "i" } else { &b.name };
            Binder {
                name: fresh_name(base, &mut taken),
                ty: b.ty,
                style: b.style,
                span: b.span,
            }
        })
        .collect();
    let index_names: Vec<String> = index_binders.iter().map(|b| b.name.clone()).collect();

    // `Ind p1 … pn i1 … ik`（无参数/索引时就是裸 `Ind`）。
    let ind_applied = |span: Span| {
        let mut e = params.iter().fold(e_ident(name, span), |acc, p| {
            e_app(acc, e_ident(&p.name, span), span)
        });
        for index in &index_names {
            e = e_app(e, e_ident(index, span), span);
        }
        e
    };

    let motive = fresh_name("motive", &mut taken);
    let minors: Vec<String> = (0..constructors.len())
        .map(|i| fresh_name(&format!("m{i}"), &mut taken))
        .collect();
    let target = fresh_name("target", &mut taken);

    let motive_x = fresh_name("x", &mut taken);
    // motive : forall (indices…), (x : Ind params indices) -> Sort
    let motive_ty = e_forall(
        index_binders.clone(),
        e_forall(
            vec![Binder {
                name: motive_x,
                ty: Some(Box::new(ind_applied(ty_span))),
                style: BinderKind::Explicit,
                span: ty_span,
            }],
            motive_sort(ty_span),
            ty_span,
        ),
        ty_span,
    );

    // 派生字段名（避让参数/motive 等），并把 ctor 字段名 → 派生名的替换同时作用
    // 到字段类型与构造子结果的索引实参上（索引可能引用字段，如 `Vec A n`）。
    let derived: Vec<DerivedCtor> = constructors
        .iter()
        .map(|ctor| {
            let mut rename: HashMap<String, Expr> = HashMap::new();
            let mut fields = Vec::new();
            for binder in ctor_field_binders(ctor) {
                let base = if binder.name.is_empty() {
                    "x"
                } else {
                    &binder.name
                };
                let field_name = fresh_name(base, &mut taken);
                if !binder.name.is_empty() && binder.name != field_name {
                    rename.insert(binder.name.clone(), e_ident(&field_name, binder.span));
                }
                let ty = binder.ty.map(|t| {
                    Box::new(super::goals::substitute_names(&t, &rename, &HashMap::new()))
                });
                fields.push(Binder {
                    name: field_name,
                    ty,
                    style: binder.style,
                    span: binder.span,
                });
            }
            // A ctor's result may be written in the arrow chain
            // (`ctor ps (n : Nat) : P n -> P (Nat.succ n)`); the indices live in
            // the *codomain*, so read them through `spine_of_codomain`. Reading
            // `ctor.result` with `src_spine` returned `None` for `Arrow`/`Forall`
            // and silently dropped the minor's index arguments, which the kernel
            // then rejected (docs/design/agent-query-channel.md H6-C / TODO A).
            let ctor_indices: Vec<Expr> = spine_of_codomain(&ctor.result)
                .filter(|(head, _)| head.as_str() == name)
                .map(|(_, args)| {
                    args.into_iter()
                        .skip(params.len())
                        .map(|a| super::goals::substitute_names(&a, &rename, &HashMap::new()))
                        .collect()
                })
                .unwrap_or_default();
            let rec_args = fields
                .iter()
                .filter(|field| {
                    field
                        .ty
                        .as_deref()
                        .is_some_and(|ty| mentions_ident(ty, name))
                })
                .map(|field| {
                    let field_ty = field.ty.as_deref().expect("field has a type");
                    let raw = result_chain_binders(field_ty);
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
                    let index_args: Vec<Expr> = spine_of_codomain(field_ty)
                        .map(|(_, args)| args.into_iter().skip(params.len()).collect())
                        .unwrap_or_default();
                    (field.name.clone(), telescope, index_args)
                })
                .collect();
            DerivedCtor {
                fields,
                rec_args,
                ctor_indices,
            }
        })
        .collect();

    // 每个构造子的 minor 前提：forall (字段… ih…), motive <ctor 索引实参> (C 字段…)。
    let minor_types: Vec<Expr> = constructors
        .iter()
        .enumerate()
        .zip(&derived)
        .map(|((i, ctor), d)| {
            let mut binders = d.fields.clone();
            for (field_name, telescope, field_indices) in &d.rec_args {
                let field_app = telescope
                    .iter()
                    .fold(e_ident(field_name, ctor.span), |acc, binder| {
                        e_app(acc, e_ident(&binder.name, ctor.span), ctor.span)
                    });
                // IH : motive <field 索引实参> <field 应用>
                let mut ih_body = e_ident(&motive, ctor.span);
                for index in field_indices {
                    ih_body = e_app(ih_body, index.clone(), ctor.span);
                }
                ih_body = e_app(ih_body, field_app, ctor.span);
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
            // R1：minor 里生成的 `C params fields` 项必须用**规范名**——
            // 内核按名字重建比对 recursor 的 minor 与 iota 规则。
            let canonical = &ctor_canonical[i];
            let mut c_app = params.iter().fold(e_ident(canonical, ctor.span), |acc, p| {
                e_app(acc, e_ident(&p.name, ctor.span), ctor.span)
            });
            c_app = d.fields.iter().fold(c_app, |acc, field| {
                e_app(acc, e_ident(&field.name, ctor.span), ctor.span)
            });
            let mut body = e_ident(&motive, ctor.span);
            for index in &d.ctor_indices {
                body = e_app(body, index.clone(), ctor.span);
            }
            let body = e_app(body, c_app, ctor.span);
            e_forall(binders, body, ctor.span)
        })
        .collect();

    // 递归子望远镜：params → motive → minors → 索引 → 目标
    // （内核 `major_idx = params + motives + minors + indices`）。
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
    rec_binders.extend(index_binders.clone());
    rec_binders.push(Binder {
        name: target.clone(),
        ty: Some(Box::new(ind_applied(ty_span))),
        style: BinderKind::Explicit,
        span: ty_span,
    });
    let mut rec_body = e_ident(&motive, ty_span);
    for index in &index_names {
        rec_body = e_app(rec_body, e_ident(index, ty_span), ty_span);
    }
    rec_body = e_app(rec_body, e_ident(&target, ty_span), ty_span);
    let rec_ty = e_forall(rec_binders, rec_body, ty_span);
    let rec = RecDecl {
        name: format!("{name}.rec"),
        universe: universe.clone(),
        ty: rec_ty,
        span: constructors.last().map(|ctor| ctor.span).unwrap_or(ty_span),
    };

    // 每构造子一条规则：telescope = (params, motive, 全部 minors, 本构造子字段)，
    // 返回 m_i <字段…>，递归字段后面追加自调用（携带该字段的索引实参）。
    let rules = constructors
        .iter()
        .enumerate()
        .map(|(i, ctor)| {
            let d = &derived[i];
            let mut binders =
                Vec::with_capacity(params.len() + constructors.len() + d.fields.len() + 1);
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
            for (field_name, telescope, field_indices) in &d.rec_args {
                let mut call = e_universe_app(&format!("{name}.rec"), &universe, ctor.span);
                for param in params {
                    call = e_app(call, e_ident(&param.name, ctor.span), ctor.span);
                }
                call = e_app(call, e_ident(&motive, ctor.span), ctor.span);
                for minor_name in &minors {
                    call = e_app(call, e_ident(minor_name, ctor.span), ctor.span);
                }
                for index in field_indices {
                    call = e_app(call, index.clone(), ctor.span);
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
                // 内核断言 `rule.ctor_name == ctor.name`（inductive.rs:1590），
                // 而 ctor.name 已是安装名（规范名）——这里必须同步。
                ctor_name: ctor_canonical[i].clone(),
                val: e_lambda(binders, body, ctor.span),
                span: ctor.span,
            }
        })
        .collect();

    (rec, rules)
}

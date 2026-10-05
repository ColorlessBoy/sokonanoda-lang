//! **R1c-2b（2026-10-05）**：声明的**出口** —— 把仍**没解出**的层元变量换成 `param` ✓。
//!
//! ## Lean 4 对照（本机源码 HEAD `d0493e4c1e` ✓）
//!
//! * `Elab/Declaration.lean:118-127`（`elabAxiom`）：
//!   `type ← Term.levelMVarToParam type` ⇒ `usedParams := collectLevelParams {} type |>.params`
//!   ⇒ `sortDeclLevelParams scopeLevelNames allUserLevelNames usedParams` ⇒
//!   `levelParams := levelParams` ⇒ 然后才 `addDecl decl` ✓。
//! * `TermElabM.lean:981-987` `levelMVarToParam` ⇒ `MCtx.levelMVarToParam`
//!   （`MetavarContext.lean:1475`）⇒ `visitLevel`（`:1427-1441`）：
//!   **已赋值 ⇒ 代进去（`some v => visitLevel v` ✓）· 未赋值 ⇒ fresh 名
//!   （`mkParamName`：`u` + `appendIndexAfter nextParamIdx` ⇒ `u_1`/`u_2`… ✓，
//!   避开 `alreadyUsedPred` ✓）并 `assignLevelMVar mvarId p`** ✓。
//!
//! ## 两处偏离（白纸黑字 ✗，见 `docs/notes/HANDOFF-kernel.md` 的「Lean 4 对照」）
//!
//! 1. Lean 在**核项**上原地重写 ✓；我们的内核 `TcCtx` 在 `ExportFile::with_ctx` 的
//!    **作用域 arena** 里分配（`util.rs:895-900` ⇒ `ExprPtr::local` ✓）⇒ 指针**出不了**
//!    那个作用域 ✗ ⇒ 这里用 `EnvBuilder`（**持久 DAG** ✓）的构造器重建 ✓，
//!    与内核那份 `TcCtx::level_mvar_to_param_expr`（R1c-1 ✓）**逐臂同形** ✓。
//! 2. **收集**仍走内核的 `TcCtx::collect_level_mvars_expr`（R1c-2a ✓，只读 ✓）——
//!    与 Lean 的 `Expr.collectLevelMVars` 同一条 ✓。
//!
//! ## ⚠ 为什么挂在 **walk**（`add_declar` 之前）而不是内核阶段
//!
//! 本仓库的声明是 **walk 当场 `add_declar`**（切片 1b ✓ —— 内核阶段只**读**环境 ✓）
//! ⇒ 内核阶段再转就**晚了** ✗：环境里那份还带 mvar ⇒ 后面的声明引用它时，
//! `infer` 的 `all_uparams_defined`（`infer.rs:103/114`）判拒 ✗。
//! Lean 的形状也正是「**先转、后 `addDecl`**」✓ ⇒ 这里对齐它 ✓。

use sokonanoda::builder::EnvBuilder;
use sokonanoda::env::{Declar, DeclarInfo, EnvLimit};
use sokonanoda::expr::{Expr, LetData};
use sokonanoda::level::Level;
use sokonanoda::util::{ExprPtr, LevelPtr, NamePtr};

/// 把一个声明**收口**：`ty`/`val` 里的层元变量 ⇒ `param`，并把新名并进 `uparams` ✓。
///
/// 返回值第二项 = **新加的宇宙参数名**（保序 ✓）—— 调用方把它并进 `DeclState.universe`
/// （对齐 Lean 的 `setLevelNames (r.newParamNames …)` ✓，`TermElabM.lean:984-986` ✓）。
///
/// **快路** ✓：一个层元变量都没有 ⇒ **原样返回** ✓（今天所有输入都走这条 ⇒ 逐字节不变 ✓）。
pub(crate) fn discharge_declar<'a>(
    builder: &mut EnvBuilder<'a>,
    declar: Declar<'a>,
) -> (Declar<'a>, Vec<String>) {
    // **R2b-1/R2b-2 的待解表** ✓：**取出并清空**（对齐 Lean 的「出口读 `MCtx`」✓）。
    // ⚠ 表**不是**权威来源 ✗ —— elaborate 期间被丢弃的中间项
    // （`infer_type_text_inplace` 的 scratch 作用域 ✓）也会往里 push ✓
    // ⇒ 权威来源是下面那次**收集**（对齐 Lean：`levelMVarToParam` 只走声明那个项 ✓）。
    let pending = crate::compile::meta::level_mvar_table::take();
    let mut ids: Vec<u64> = Vec::new();
    collect_declar_mvars(builder, &declar, &mut ids);
    if ids.is_empty() {
        return (declar, Vec::new());
    }
    let uparams = declar.info().uparams;
    let (mapping, names) = level_mapping(builder, uparam_names(uparams), &ids, &pending);
    let uparams = extend_uparams(builder, uparams, &mapping);
    let ty = rewrite_expr(builder, declar.info().ty, &mapping);
    let name = declar.info().name;
    let out = match declar {
        Declar::Axiom { .. } => Declar::Axiom {
            info: DeclarInfo { name, uparams, ty },
        },
        Declar::Quot { .. } => Declar::Quot {
            info: DeclarInfo { name, uparams, ty },
        },
        Declar::Theorem { val, .. } => Declar::Theorem {
            info: DeclarInfo { name, uparams, ty },
            val: rewrite_expr(builder, val, &mapping),
        },
        Declar::Definition { val, hint, .. } => Declar::Definition {
            info: DeclarInfo { name, uparams, ty },
            val: rewrite_expr(builder, val, &mapping),
            hint,
        },
        Declar::Opaque { val, .. } => Declar::Opaque {
            info: DeclarInfo { name, uparams, ty },
            val: rewrite_expr(builder, val, &mapping),
        },
        // `Inductive` 的字段是 `pub(crate)`（内核私有 ✓）⇒ 前端**造不出来** ✗
        // ⇒ 走 [`discharge_info`]（调用方在 `add_inductive` **之前**调 ✓）。
        other @ Declar::Inductive(_) => other,
        Declar::Constructor(data) => {
            let mut data = data;
            data.info = DeclarInfo { name, uparams, ty };
            Declar::Constructor(data)
        }
        Declar::Recursor(data) => {
            let mut data = data;
            data.info = DeclarInfo { name, uparams, ty };
            Declar::Recursor(data)
        }
    };
    (out, names)
}

/// 只收口一个**项**（`OpenExercise` 的 `declared_ty` 那条路 ✓）。
///
/// ⚠ **必须与 [`discharge_declar`] 同名同序** ✗：`declared_ty` 与签名探针是**两次**
/// elaborate 同一份签名（mvar id 不同 ✓）⇒ 两次都从 `u_1` 起、按同一顺序编号
/// ⇒ 同一份签名拿到**同一批名字** ✓（`existing` 两次都传源级 `universe` ✓）。
pub(crate) fn discharge_expr<'a>(
    builder: &mut EnvBuilder<'a>,
    e: ExprPtr<'a>,
    existing: &[String],
) -> (ExprPtr<'a>, Vec<String>) {
    let mut ids: Vec<u64> = Vec::new();
    collect_mvars(builder, e, None, &mut ids);
    if ids.is_empty() {
        return (e, Vec::new());
    }
    let used = existing
        .iter()
        .map(|name| builder.name_from_str(name))
        .collect();
    // ⚠ 这里**不读待解表** ✗（`#check`/`#reduce`/`declared_ty` 都不是"声明" ✓）——
    // 项里**还留着**的 mvar 按构造就是**没解出**的 ✓（解出的当场已换掉 ✓）。
    let (mapping, names) = level_mapping(builder, used, &ids, &[]);
    (rewrite_expr(builder, e, &mapping), names)
}

/// `Inductive` 专用（它的 `Declar` 字段是内核私有 ✓）：**在 `add_inductive` 之前**收口 `info` ✓。
pub(crate) fn discharge_info<'a>(
    builder: &mut EnvBuilder<'a>,
    info: DeclarInfo<'a>,
) -> (DeclarInfo<'a>, Vec<String>) {
    let mut ids: Vec<u64> = Vec::new();
    collect_mvars(builder, info.ty, None, &mut ids);
    if ids.is_empty() {
        return (info, Vec::new());
    }
    let used = uparam_names(info.uparams);
    // 归纳块**在 `add_inductive` 之前**收口 ✓ ⇒ 待解表里还有本块的 id ✓
    // （与 `discharge_declar` 同款 ✓）。
    let pending = crate::compile::meta::level_mvar_table::take();
    let (mapping, names) = level_mapping(builder, used, &ids, &pending);
    let uparams = extend_uparams(builder, info.uparams, &mapping);
    let ty = rewrite_expr(builder, info.ty, &mapping);
    (
        DeclarInfo {
            name: info.name,
            uparams,
            ty,
        },
        names,
    )
}

/// 声明里出现的层元变量 id（去重、保序 ✓）—— 走内核的 `collect_level_mvars_expr` ✓。
fn collect_declar_mvars<'a>(builder: &mut EnvBuilder<'a>, declar: &Declar<'a>, out: &mut Vec<u64>) {
    let ty = declar.info().ty;
    let val = match declar {
        Declar::Theorem { val, .. }
        | Declar::Definition { val, .. }
        | Declar::Opaque { val, .. } => Some(*val),
        _ => None,
    };
    collect_mvars(builder, ty, val, out);
}

fn collect_mvars<'a>(
    builder: &mut EnvBuilder<'a>,
    ty: ExprPtr<'a>,
    val: Option<ExprPtr<'a>>,
    out: &mut Vec<u64>,
) {
    let limit = EnvLimit::ByIndex(builder.declaration_count());
    builder.with_env(|ef| {
        ef.with_tc(limit, |tc| {
            let ctx = &mut *tc.ctx;
            ctx.collect_level_mvars_expr(ty, out);
            if let Some(val) = val {
                ctx.collect_level_mvars_expr(val, out);
            }
        })
    });
}

/// 给每个 id 定一个**层** ✓ —— 两档，**逐条对齐 Lean `visitLevel`**
/// （`MetavarContext.lean:1427-1441` ✓）：
///
/// * **表说已解出**（`Some(text)` ✓）⇒ 代那个层 ✓（`some v => visitLevel v` ✓）。
///   ⚠ 理论上**不该走到** ✗ —— 解出时已在应用处当场 `relabel_app_head` 换掉了 ✓；
///   留着这一档是 Lean 的**完整语义** ✓（也是"表必须被读"的那一处 ✓）。
///   文本解析不回（`max` 之类 ✓）⇒ **不猜** ✗，落回下面那档 ✓。
/// * **其余**（表里没有 / 没解出 ✓）⇒ fresh `u_N` 名 ⇒ `param` ✓
///   （`none ⇒ mkParamName` ✓，避开 `alreadyUsedPred` ✓）。
///
/// 返回 `(id ⇒ 层, 新加的 param 名)` ✓ —— 只有**新加的 param** 才并进 `uparams` ✓
/// （对齐 Lean：赋过值的 mvar 不产生参数 ✓）。
fn level_mapping<'a>(
    builder: &mut EnvBuilder<'a>,
    used: Vec<NamePtr<'a>>,
    ids: &[u64],
    pending: &[(u64, Option<String>)],
) -> (Vec<(u64, LevelPtr<'a>)>, Vec<String>) {
    // 声明自己的宇宙参数名 ⇒ `UnivMap`（把「已解出」的层文本解析回内核层 ✓）。
    let universe: Vec<String> = used
        .iter()
        .map(|name| builder.name_to_string(*name))
        .collect();
    let univ = crate::compile::elab::make_univ_map(builder, &universe);
    let mut used = used;
    let mut next = 1u64;
    let mut mapping: Vec<(u64, LevelPtr<'a>)> = Vec::with_capacity(ids.len());
    let mut names: Vec<String> = Vec::with_capacity(ids.len());
    for id in ids {
        let solved = pending
            .iter()
            .rev()
            .find(|(i, _)| i == id)
            .and_then(|(_, text)| text.as_deref());
        if let Some(text) = solved {
            if let Ok(level) =
                crate::compile::elab::level_ptr(builder, text, &univ, crate::Span::default())
            {
                mapping.push((*id, level));
                continue;
            }
        }
        let (text, ptr) = loop {
            let text = format!("u_{next}");
            next += 1;
            let ptr = builder.name_from_str(&text);
            if !used.contains(&ptr) {
                break (text, ptr);
            }
        };
        used.push(ptr);
        mapping.push((*id, builder.level_param(ptr)));
        names.push(text);
    }
    (mapping, names)
}

/// 已有的宇宙参数名（按**指针**判重 ✓ —— 名字是 intern 的 ✓）。
fn uparam_names<'a>(uparams: sokonanoda::util::LevelsPtr<'a>) -> Vec<NamePtr<'a>> {
    let mut used: Vec<NamePtr<'a>> = Vec::new();
    for level in uparams.iter() {
        if let Level::Param(name, _) = **level {
            used.push(name);
        }
    }
    used
}

fn extend_uparams<'a>(
    builder: &mut EnvBuilder<'a>,
    uparams: sokonanoda::util::LevelsPtr<'a>,
    mapping: &[(u64, LevelPtr<'a>)],
) -> sokonanoda::util::LevelsPtr<'a> {
    let mut levels: Vec<LevelPtr<'a>> = uparams.iter().copied().collect();
    for (_, param) in mapping {
        if !levels.contains(param) {
            levels.push(*param);
        }
    }
    builder.alloc_levels_slice(&levels)
}

/// 把项里的层元变量换成 `param` ✓ —— **逐臂对齐**内核的 `TcCtx::level_mvar_to_param_expr`
/// （R1c-1 ✓）与 Lean 的 `LevelMVarToParam.main`（`MetavarContext.lean:1442-1465` ✓）。
fn rewrite_expr<'a>(
    builder: &mut EnvBuilder<'a>,
    e: ExprPtr<'a>,
    mapping: &[(u64, LevelPtr<'a>)],
) -> ExprPtr<'a> {
    match &*e {
        Expr::Var { .. } | Expr::NatLit { .. } | Expr::StringLit { .. } | Expr::Meta { .. } => e,
        Expr::Sort { level, .. } => {
            let level = rewrite_level(builder, *level, mapping);
            builder.mk_sort(level)
        }
        Expr::Const { name, levels, .. } => {
            let levels: Vec<LevelPtr<'a>> = levels
                .iter()
                .map(|l| rewrite_level(builder, *l, mapping))
                .collect();
            let levels = builder.alloc_levels_slice(&levels);
            builder.mk_const(*name, levels)
        }
        Expr::App { fun, arg, .. } => {
            let fun = rewrite_expr(builder, *fun, mapping);
            let arg = rewrite_expr(builder, *arg, mapping);
            builder.mk_app(fun, arg)
        }
        Expr::Pi {
            binder_name,
            binder_style,
            binder_type,
            body,
            ..
        } => {
            let binder_type = rewrite_expr(builder, *binder_type, mapping);
            let body = rewrite_expr(builder, *body, mapping);
            builder.mk_pi(*binder_name, *binder_style, binder_type, body)
        }
        Expr::Lambda {
            binder_name,
            binder_style,
            binder_type,
            body,
            ..
        } => {
            let binder_type = rewrite_expr(builder, *binder_type, mapping);
            let body = rewrite_expr(builder, *body, mapping);
            builder.mk_lambda(*binder_name, *binder_style, binder_type, body)
        }
        Expr::Let { data, .. } => {
            // ⚠ 这里匹配的是 `&*e`（引用 ✓）⇒ 默认绑定模式让 `data` 是 `&&LetData` ✗
            // ⇒ 必须 `**data`（字段全 `Copy` ✓）才拿得到值 ✓。
            let LetData {
                binder_name,
                binder_type,
                val,
                body,
                nondep,
            } = **data;
            let binder_type = rewrite_expr(builder, binder_type, mapping);
            let val = rewrite_expr(builder, val, mapping);
            let body = rewrite_expr(builder, body, mapping);
            builder.mk_let(binder_name, binder_type, val, body, nondep)
        }
        Expr::Proj {
            ty_name,
            idx,
            structure,
            ..
        } => {
            let structure = rewrite_expr(builder, *structure, mapping);
            builder.mk_proj(*ty_name, *idx, structure)
        }
    }
}

/// 层里的层元变量换成 `param` ✓（对齐 `visitLevel` 的 `none ⇒ mkParamName` 那一臂 ✓）。
fn rewrite_level<'a>(
    builder: &mut EnvBuilder<'a>,
    level: LevelPtr<'a>,
    mapping: &[(u64, LevelPtr<'a>)],
) -> LevelPtr<'a> {
    match &*level {
        // `zero` / `param` 原样 ✓（Lean 的 `visitLevel` 同款 ✓）。
        Level::Zero | Level::Param(..) => level,
        Level::Succ(inner, _) => {
            let inner = rewrite_level(builder, *inner, mapping);
            builder.succ(inner)
        }
        Level::Max(l, r, _) => {
            let l = rewrite_level(builder, *l, mapping);
            let r = rewrite_level(builder, *r, mapping);
            builder.level_max(l, r)
        }
        Level::IMax(l, r, _) => {
            let l = rewrite_level(builder, *l, mapping);
            let r = rewrite_level(builder, *r, mapping);
            builder.level_imax(l, r)
        }
        Level::MVar(id, _) => mapping
            .iter()
            .find(|(i, _)| i == id)
            .map_or(level, |(_, param)| *param),
    }
}

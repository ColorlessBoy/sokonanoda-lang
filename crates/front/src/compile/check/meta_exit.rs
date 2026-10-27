//! **S4 / B2-a（2026-10-23）**：binder 类型元变量的**声明出口** ✓。
//!
//! ## 这是什么
//!
//! `elab_expr` 的 `Expr::Lambda` 臂在**三档都剥不出** binder 类型时（`peel_expected`
//! / `peel_expected_src` / `b2_whnf_peel`），开关 `SOKO_DECL_METAS=1` 下造一对
//! **同 id** 的元变量（[`crate::compile::elab::b2_meta_pair`]）并把那个 binder
//! **记进本声明的表**（[`crate::compile::meta::DeclMetas`]）。本模块是那条路的**出口**：
//!
//! 1. **快路**：表空 ⇒ 原样返回（默认档所有输入都走这条 ⇒ 零成本、逐字节不变 ✓）；
//! 2. **收集**：声明 `ty`/`val` 里**真的出现**的 id（表只是记账，scratch 泄漏不算 ✓）；
//! 3. **求解**：`with_env` + `with_tc` 里 `infer_closed_type(val)` 再与声明类型
//!    `assert_def_eq` ⇒ 内核合一给 binder 元变量赋值（panic 被 `quiet_catch` 吃掉）；
//!    赋值读 `tc.ctx.mvars.value(id)`、用 `tc.with_pp` **渲成文本**（跨 arena 只能传
//!    `String` ✓）；含 `$`（松散变量）的赋值**当作没解出**（开项赋值要 scope seeding，
//!    B2-a 不做 ✗）；
//! 4. **未解 ⇒ 今天那条错**：`ElabUntypedBinder` + **逐字复用**的
//!    [`B2_UNTYPED_BINDER_MSG`]，span = 表里**第一条**落在本声明里的 binder（= 今天
//!    抛错的那一个 ⇒ 负例的码/句/span **逐字节不变** ✓）；**绝不**把元变量交给
//!    `add_declar`（K1 硬不变式①会把拒绝面扩大成难懂的 `kernel-rejected` ✗）；
//! 5. **全解 ⇒ 源级按 span 重写 binder 注解 + 重 elaborate**（不是 kernel zonk 走）：
//!    产出**天然无 meta** ✓、hover 行记的是真类型 ✓、不需要 de Bruijn 论证 ✓；
//!    重 elaborate 前 `hovers.truncate` 掉第一遍的行；最后 `declar_contains_meta`
//!    **硬闸**（release 档也拦）⇒ 还剩 meta 就退回今天那条错 ✓。
//!
//! ## 三条白纸黑字的偏离（见 `plans/apply-unification-plan.md` §21 与施工图 §7）
//!
//! * **sort 只取 `Type`（`Sort 1`）**：`Expr::Meta{ty}` 的 `ty` 必须是**合法类型**，
//!   而 binder 类型的 `ty` 必须是个 **sort**；失败点连 sort 都不知道（施工图 C1）
//!   ⇒ `Prop` 值 binder 仍然解不出 ⇒ 报**与今天相同**的错（不是回归 ✓）。真修法
//!   （sort 元变量 / 允许未知 sort）留 K0-b。
//! * **出口 = 源级重写**（施工图 §7 第 2 条已拍板）：kernel zonk 会把 `⟨meta#7⟩`
//!   漏到 hover（`pretty_printer.rs` 的 Meta 臂）。
//! * **span 取「表里第一条落在声明里的 binder」而不是「第一条未解」**（施工图 §4.4
//!   第 4 步写的是后者 ✗）：`def f : Nat := (fun x y => x) 1` 今天在 **`x`** 抛错
//!   （`x` 的元变量其实会被实参 `1` 解出、`y` 才未解）⇒ 用「第一条未解」会把 span
//!   挪到 `y`，负例的 span 就变了 ✗。今天抛错的位置**恒等于**第一个造元变量的 binder
//!   （三档剥不出 ⇒ 立刻 `return Err`）⇒ 取「第一条在声明里的表项」才是**逐字节**
//!   复现 ✓。

use sokonanoda::builder::EnvBuilder;
use sokonanoda::env::{Declar, EnvLimit};
use sokonanoda::expr::{BinderStyle, Expr as KernelExpr};
use sokonanoda::util::ExprPtr;

use crate::ast::Expr;
use crate::compile::elab::{b2_meta_pair, kernel_binder_style, ElabCtx};
use crate::compile::error::{CompileError, ErrorKind};
use crate::compile::meta::{
    decl_metas_enabled, fresh_decl_meta_id, DeclMeta, DeclMetaCtx, MetaKind,
};
use crate::span::Span;

/// **今天那条诊断的句子** —— **逐字复用** ✓（`docs/protocol.md` 与三条既有测试都钉它；
/// 出口报错必须与 `elab_expr` 的失败分支**同一个常量**，否则负例会改字 ✗）。
pub(crate) const B2_UNTYPED_BINDER_MSG: &str = concat!(
    "cannot infer the type of this binder: the declared type does not provide a matching position ",
    "(write it explicitly, e.g. fun (x : Nat) => x)",
);

/// 今天那条 `elab-untyped-binder`（码 + 句 + span 三件都在这里 ⇒ 一处定义 ✓）。
pub(crate) fn untyped_binder(span: Span) -> CompileError {
    CompileError::elab(ErrorKind::ElabUntypedBinder, B2_UNTYPED_BINDER_MSG, span)
}

/// 声明的**值位**（`Definition`/`Theorem`/`Opaque`）。
///
/// ⚠ **不能用 `Declar::value()`** ✗：内核那个访问器只认 `Definition`/`Opaque`，
/// **`Theorem` 返回 `None`** ⇒ 定理的证明体既不会被收集、也不会被硬闸看见（实测：
/// `theorem t : (let P := (True -> True); P) := fun h => h` 的元变量因此漏进内核 ⇒
/// `kernel-rejected` + `?m0`）。这里逐臂照 `level_exit::collect_declar_mvars` ✓。
fn declar_val<'a>(d: &Declar<'a>) -> Option<ExprPtr<'a>> {
    match d {
        Declar::Theorem { val, .. }
        | Declar::Definition { val, .. }
        | Declar::Opaque { val, .. } => Some(*val),
        _ => None,
    }
}

/// `ElabCtx` 的构造器（`prelude.rs` 的三处字面量共用）。
///
/// ⚠ **`walk.rs` 不用它** ✗：那边 `self` 的**分字段借用**（`&self.inductives` /
/// `&self.ns` / `&self.display`）不能被 `&self` 方法替代（会让 `&mut self.builder` 冲突），
/// 且 `decl_ctx` 的 7 个实参按 rustfmt 必须一行一个（9 行 > 原来的 7 行字面量）⇒
/// 那里保留字面量 + 新字段（施工图 §4.3 的 11 处）。
pub(crate) fn decl_ctx<'a, 'b>(
    prefix_src: &'b str,
    options: &'b crate::compile::prelude::CompileOptions,
    inductives: &'b crate::compile::elab::InductiveTable<'a>,
    ns: &'b crate::compile::scope::NamespaceScope,
    notations: Option<&'b crate::display::DisplayNotations>,
    defs: &'b crate::compile::elab::DefTable,
    metas: Option<&'b DeclMetaCtx>,
) -> ElabCtx<'a, 'b> {
    ElabCtx {
        prefix_src,
        options,
        inductives,
        ns,
        notations,
        defs,
        metas,
    }
}

// ───────────────────── B2-P4a：声明出口的三趟 + 收口（子模块）─────────────────────
//
// 为什么另起一个文件 ✗→✓：S4（本文件）与 B2-P4a 是**两条出口**（前者解 binder 类型元变量、
// 后者把声明级 store 收口并重放）；判据 ⑨ 的软线是 **500 行**，本文件在 P4a 之前已经
// 390 行 ⇒ 再加 ~210 行就超线 ⇒ 拆 `p4`（模块结构，不是搬家）。
pub(crate) use p4::{add_decl_silent, build_axiom, build_value, gate_path_active, ValKind};
mod p4;
// **B2-P4b**：应用结点的「结果类型 ≟ 期望类型」方程（`implicit.rs` 的贪心出口调它）。
pub(crate) mod p4b;

/// 一条声明的元变量上下文（S4 表 + B2-P1 的 store；**栈上**持有 ⇒ 词法作用域；见 `meta.rs` 的 R4 说明）。
pub(crate) fn new_meta_table() -> DeclMetaCtx {
    DeclMetaCtx::new()
}

/// **Lambda 臂的兜底**（`elab_expr` 的三档都剥不出时调）：
/// 开关开着 ⇒ 造一对元变量、记账、返回**核项侧**类型 + binder 风格（源侧留给 B2-b）；
/// 开关关着 / 不在声明通道（`ctx.metas == None`）⇒ `None` ⇒ 调用方报**今天那条错** ✓。
///
/// ⚠ **返回的源级 `src_ty` 必须是 `None`**（调用方写死）：给 `Some(Ident("\0soko_mN"))`
/// 会让 `judge_binders` 把它渲进合成文档（`\0` 词法非法 ✗）、让 `peel_expected_src`
/// 把它当期望类型 elaborate（unknown identifier ✗）。
pub(crate) fn b2_binder<'a>(
    builder: &mut EnvBuilder<'a>,
    ctx: &ElabCtx<'a, '_>,
    binder: &crate::ast::Binder,
) -> Option<(ExprPtr<'a>, BinderStyle)> {
    let table = ctx.metas?;
    if !decl_metas_enabled() {
        return None;
    }
    let id = fresh_decl_meta_id();
    // **C1**：binder 类型的自身类型只能猜一个 sort ⇒ B2-a 取 `Type`（`Sort 1`）。
    let z = builder.zero();
    let one = builder.succ(z);
    let sort = builder.mk_sort(one);
    let (_src, kernel) = b2_meta_pair(builder, id, MetaKind::Natural, sort, binder.span);
    table.borrow_mut().entries.push(DeclMeta {
        id,
        span: binder.span,
    });
    Some((kernel, kernel_binder_style(&binder.style)))
}

/// 声明出口：解出表里的 binder 元变量；解不出 ⇒ **今天那条错**；全解 ⇒ 源级重写 + 重 elaborate。
///
/// `rebuild` = "把（重写后的）源值位重新 elaborate 成一条声明"（由 `build_*` 给，
/// 它握着 `scope`/`univ`/`known`/`hovers`/`ctx` 与第一遍的 `ty_kernel`）。
pub(crate) fn finish<'a>(
    builder: &mut EnvBuilder<'a>,
    decl: Declar<'a>,
    val_src: &Expr,
    table: Option<&DeclMetaCtx>,
    mut rebuild: impl FnMut(&mut EnvBuilder<'a>, &Expr) -> Result<Declar<'a>, CompileError>,
) -> Result<Declar<'a>, CompileError> {
    let Some(table) = table else {
        return Ok(decl);
    };
    let metas = std::mem::take(&mut *table.borrow_mut());
    // ① **快路**：默认档所有输入（表恒空）⇒ 一次 `is_empty` ✓。
    if metas.entries.is_empty() {
        return Ok(decl);
    }
    // ② **权威来源 = 声明里真的出现的 id**（表可能有 scratch 泄漏 ✓）。
    let mut ids: Vec<u32> = Vec::new();
    collect_meta_ids(decl.info().ty, &mut ids);
    if let Some(v) = declar_val(&decl) {
        collect_meta_ids(v, &mut ids);
    }
    if ids.is_empty() {
        return Ok(decl); // 表里全是 scratch ⇒ 声明本身没有元变量 ✓
    }
    // 今天抛错的位置 = **第一条落在声明里的**表项（见模块头的第 3 条偏离）。
    let in_decl: Vec<&DeclMeta> = metas
        .entries
        .iter()
        .filter(|e| ids.contains(&e.id))
        .collect();
    let first = in_decl.first().map_or(metas.entries[0].span, |e| e.span);
    // ③ **求解通道**（唯一能产解的地方）：
    let limit = EnvLimit::ByIndex(builder.declaration_count());
    let solved: Vec<(u32, String)> = builder.with_env(|ef| {
        ef.with_tc(limit, |tc| {
            if let Some(v) = declar_val(&decl) {
                // `infer_closed_type` = `InferOnly` 推类型；与声明类型 `def_eq` 时
                // 内核的 meta 臂（`unify_meta`）才会赋值 ⇒ panic 只是"解不出"。
                let inferred = tc.infer_closed_type(v);
                let _ = super::quiet_catch(|| tc.assert_def_eq(inferred, decl.info().ty));
            }
            ids.iter()
                .filter_map(|id| {
                    let v = tc.ctx.mvars.value(*id)?;
                    let text = tc.with_pp(|pp| pp.pp_expr(v));
                    // 开项赋值（松散变量印成 `$N`）需要 scope seeding ⇒ B2-a 不猜 ✗。
                    if text.contains('$') {
                        return None;
                    }
                    Some((*id, text))
                })
                .collect()
        })
    });
    // ④ 未解 ⇒ **逐字**今天那条错（span = 第一条落在声明里的 binder ✓）。
    let solved_in_decl: Vec<(Span, String)> = in_decl
        .iter()
        .filter_map(|e| {
            solved
                .iter()
                .find(|(id, _)| *id == e.id)
                .map(|(_, text)| (e.span, text.clone()))
        })
        .collect();
    if solved_in_decl.len() < in_decl.len() {
        return Err(untyped_binder(first));
    }
    // ⑤ **源级按 span 重写 binder 注解 + 重 elaborate**（产出天然无 meta ✓）。
    let Some(rewritten) = rewrite_binders_by_span(val_src, &solved_in_decl) else {
        return Err(untyped_binder(first));
    };
    let Ok(decl2) = rebuild(builder, &rewritten) else {
        // 第二遍 elaborate 失败 = 我们注入的注解没用 ⇒ 退回今天那条错（绝不改判定面 ✗）。
        return Err(untyped_binder(first));
    };
    // ⑥ **硬闸**（release 档也拦）：还剩元变量 ⇒ 绝不交给 `add_declar` ✗。
    // 内核那条 `declar_contains_meta` 覆盖 `ty`/`rec_rules`/`Definition`/`Opaque` 的 val，
    // **但 `Theorem` 的 val 是空的**（`Declar::value()` 不含它 ✗）⇒ 这里显式补上。
    if builder.declar_contains_meta(&decl2) || declar_val(&decl2).is_some_and(|v| has_meta(v)) {
        return Err(untyped_binder(first));
    }
    Ok(decl2)
}

/// 这个项里有没有元变量（**只读**；`EnvBuilder::contains_meta` 是 `pub(crate)` ⇒
/// 前端拿不到，这里用收集器实现同一件事 ✓）。
fn has_meta(e: ExprPtr<'_>) -> bool {
    let mut ids = Vec::new();
    collect_meta_ids(e, &mut ids);
    !ids.is_empty()
}

/// 声明里出现的元变量 id（去重、保序 ✓）—— 逐臂对齐内核 `EnvBuilder::contains_meta`。
fn collect_meta_ids(e: ExprPtr<'_>, out: &mut Vec<u32>) {
    match &*e {
        KernelExpr::Meta { id, .. } => {
            if !out.contains(id) {
                out.push(*id);
            }
        }
        KernelExpr::App { fun, arg, .. } => {
            collect_meta_ids(*fun, out);
            collect_meta_ids(*arg, out);
        }
        KernelExpr::Pi {
            binder_type, body, ..
        }
        | KernelExpr::Lambda {
            binder_type, body, ..
        } => {
            collect_meta_ids(*binder_type, out);
            collect_meta_ids(*body, out);
        }
        KernelExpr::Let { data, .. } => {
            let data = **data;
            collect_meta_ids(data.binder_type, out);
            collect_meta_ids(data.val, out);
            collect_meta_ids(data.body, out);
        }
        KernelExpr::Proj { structure, .. } => collect_meta_ids(*structure, out),
        KernelExpr::Var { .. }
        | KernelExpr::Sort { .. }
        | KernelExpr::Const { .. }
        | KernelExpr::StringLit { .. }
        | KernelExpr::NatLit { .. } => {}
    }
}

/// **纯源函数**：把解出的类型文本填进**按 span 命中**的 binder 注解（其余逐字保留 ✓）。
///
/// 为什么按 span：`b2_meta_pair` 就是按 `binder.span` 造的，且 `def` 的值位在非 `by`
/// 情况下就是源 AST（`walk.rs` 的 `lower_value` 只在 `by` 时换源）⇒ span 仍有效 ✓。
/// 任一文本**解析不回**（`parse_expr_text` 只带内建记法表 ⇒ 用户记法会失败）⇒
/// `None` ⇒ 出口按"解不出"处理（**绝不猜** ✗）。
fn rewrite_binders_by_span(e: &Expr, solved: &[(Span, String)]) -> Option<Expr> {
    let node = match e {
        Expr::Lambda {
            binders,
            body,
            span,
        } => {
            let mut binders = binders.clone();
            for b in &mut binders {
                if b.ty.is_some() {
                    continue;
                }
                if let Some((_, text)) = solved.iter().find(|(sp, _)| *sp == b.span) {
                    b.ty = Some(Box::new(crate::proof::parse_expr_text(text).ok()?));
                }
            }
            Expr::Lambda {
                binders,
                body: body.clone(),
                span: *span,
            }
        }
        other => other.clone(),
    };
    // 子项遍历**只有一处**知道哪些字段是子表达式（`spine::map_children` ✓）——
    // 新 `Expr` 变体不会在这里静默漏掉 ✓。
    let mut ok = true;
    let out = crate::spine::map_children(&node, &mut |child| match rewrite_binders_by_span(
        child, solved,
    ) {
        Some(n) => n,
        None => {
            ok = false;
            child.clone()
        }
    });
    ok.then_some(out)
}

#[cfg(test)]
mod tests;

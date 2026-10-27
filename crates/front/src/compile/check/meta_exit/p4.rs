//! **B2-P4a（2026-10-26）**：声明出口 —— 把声明级 `MetaStore` **收口**，并**重建**声明，
//! 让 `Expr::Meta`（载体 `\0soko_m{id}`）**绝不**走到 `walk` 的 `add_declar`。
//!
//! ## 为什么这里（而不是 `meta.rs`）
//!
//! `meta.rs` 是**引擎数据/合一**（判据 ⑨ 的存量基线 1849 行，**只许减不许增**）；本片要加的
//! 是**出口策略**（三趟 + 收口 + 重放）⇒ 天然属于 `check/meta_exit`（S4 的同一条出口线）。
//! `meta_exit.rs` 本身在 P4a 之前已 390 行 ⇒ 再加就超软线 500 ⇒ 拆成这个子模块 ✓。
//!
//! ## 三条硬约束（判据钉住，改之前先读）
//!
//! 1. **闸门关（`SOKO_DECL_MVARS` 默认关）零成本**：`build_value` 一次 `Option` 判空就
//!    返回今天的出口 ⇒ **零** park / **零**重放 / **零**额外 elaborate ⇒ `--json` 逐字节
//!    不变 ✓（7 个基线文件 + `/tmp/soko-pre-p4a` 对拍）；
//! 2. **元变量绝不进项**：收口 + 重放之后还有载体 ⇒ **重放闸门关**（`metas: None`）并返回
//!    **那一趟**的结果 —— 诊断**构造上**逐字节等于闸门关 ✓；
//! 3. **闸门关的重放必须真的是闸门关**：`metas: None` 只是摘掉 store；凡"按进程环境变量
//!    开闸"的旁路都必须改判 **`ctx` 里有没有 store**（已改：`elab.rs` 的 B3 守卫 ——
//!    否则重放会带着守卫跑，实测把 `And.left` 判成 `elab-implicit-argument-unsolved`，
//!    而闸门关明明是绿的 ✗）。
//!
//! 设计 ⇒ `docs/design/metavar-engine.md` §2.10 / §4 B2 · `docs/design/metavar-pattern.md`
//! §3.4/§7（B2-P4：声明末 defaulting + zonk；P4a = front rebuild，**无内核改动**）。

use sokonanoda::builder::EnvBuilder;
use sokonanoda::env::{Declar, DeclarInfo, ReducibilityHint};

use super::{declar_val, finish, has_meta};
use crate::ast::Expr;
use crate::compile::elab::ElabCtx;
use crate::compile::error::{CompileError, ErrorKind};
use crate::compile::event::CompileOutput;
use crate::compile::meta::decl_mvars_enabled;
use crate::span::Span;

/// 三个 `build_*` 的值位变体（`build_value` 的 `kind`）。
#[derive(Clone, Copy)]
pub(crate) enum ValKind {
    Def,
    Theorem,
    Example,
}

/// 值位的**唯一**实现（**B2-P4a 起是三趟**）：elaborate（第一趟，共用声明级 store）
/// → **声明出口收口**（[`close_meta_store`]，恰好一次）→ 用已收口的 store 重 elaborate
/// （`ty` + `val`）→ S4 出口 [`finish`] → **硬后置条件**（绝不把含元变量的声明交出去）。
/// `build_def`/`build_theorem`/`build_example` 只差 `kind`（`uparams` 的来源）✓。
///
/// ## 三趟的理由（改之前先读；设计 `metavar-engine.md` §2.10 · `metavar-pattern.md` §3.4/§7）
///
/// 1. **第一趟 = 今天**：`solve_prefix` 现在可以返回 `MetaSolve::Partial`（P3b）⇒ 未解的
///    前导隐式实参以**载体** `\0soko_m{id}` 进项（核侧 `Expr::Meta`）。载体**绝不许**活过
///    声明 —— `walk` 在 `add_declar` **之前**就得看到无 meta 的项（K1 硬不变式；
///    `walk.rs` 的 `declar_contains_meta` 分支会让整条声明**静默缺席** ⇒ `decl_idx` 空
///    ⇒ 内核阶段假 `unknown const`）。
/// 2. **收口**（[`close_meta_store`]）：`park()` → `unify_all()` → `default_unresolved()`
///    → `unsolved()` → `commit()`。**一条声明恰好一次**（`unify_all` 幂等 ⇒ 再调是空转）。
/// 3. **第二趟**：同一份源、同一个 `ctx` ⇒ 约束**重放**；store 已收口 ⇒ 这次求解看得见
///    第一趟定不下来的解 ⇒ **载体消失**（**不**做代换 / lift：白纸黑字的边界 ✓）。
///    ⚠ S4 的 binder 表非空时这一趟**由 `finish` 的重 elaborate 担当**（它本来就要按解出
///    的 binder 注解重 elaborate 一次）⇒ 不重复劳动。
/// 4. **硬后置条件**：`declar_contains_meta`（`ty`/`rec_rules`/`Definition`/`Opaque` 的值）
///    **或** `Theorem` 的值位还剩载体 ⇒ **第三趟 = 闸门关的重放**（`metas: None`）并返回
///    **它**的结果 —— 诊断因此**逐字节**等于闸门关（构造上，不是巧合）✓。
///
/// ## 三条不变式（判据钉住）
///
/// * **闸门关**（`SOKO_DECL_MVARS` 默认关）⇒ 一次 `Option` 判空就返回 ⇒ **零** park /
///   **零**重放 ⇒ `--json` 逐字节不变 ✓；
/// * 收口**恰好一次**、**幂等**；第二趟之后**不再**收口（剩下只有"重放闸门关"这一个出口）；
/// * S4 的 [`finish`] **在收口之后/之外**跑（它的重 elaborate 用已收口的 store）✓。
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_value<'a>(
    builder: &mut EnvBuilder<'a>,
    kind: ValKind,
    name: &str,
    universe: &[String],
    ty: &Expr,
    val: &Expr,
    scope: &mut crate::compile::elab::ElabScope<'a>,
    univ: &crate::compile::elab::UnivMap<'a>,
    known: &crate::compile::elab::KnownTable,
    hovers: &mut Vec<crate::compile::elab::HoverNode<'a>>,
    ctx: &ElabCtx<'a, '_>,
) -> Result<Declar<'a>, CompileError> {
    let hover_mark = hovers.len();
    // 一趟 = 同一个函数：**`ty` 与 `val` 一起**重建（载体会同时出现在两者里 ✓）。
    // `ty` 是**捕获**、`ctx` 是**参数** ⇒ 闸门关的重放只需换一个 `ElabCtx`（`metas: None`）✓；
    // `val_src` 是参数（S4 的 binder 注解重写会换掉值位的源 ✓）。
    let mut build = |b: &mut EnvBuilder<'a>,
                     val_src: &Expr,
                     ctx: &ElabCtx<'a, '_>|
     -> Result<Declar<'a>, CompileError> {
        // 重 elaborate 前丢掉上一趟的 hover 行（第一趟时是空操作 ✓）。
        hovers.truncate(hover_mark);
        let ty_kernel =
            crate::compile::elab::elab_expr(b, ty, scope, univ, known, hovers, None, None, ctx)?;
        let val_kernel = crate::compile::elab::elab_expr(
            b,
            val_src,
            scope,
            univ,
            known,
            hovers,
            Some(ty_kernel),
            Some(ty),
            ctx,
        )?;
        let name = b.name_from_str(name);
        match kind {
            ValKind::Theorem => {
                let uparams = crate::compile::elab::collect_uparams(b, univ, universe);
                Ok(Declar::Theorem {
                    info: DeclarInfo {
                        name,
                        uparams,
                        ty: ty_kernel,
                    },
                    val: val_kernel,
                })
            }
            ValKind::Def | ValKind::Example => {
                let uparams = match kind {
                    ValKind::Def => crate::compile::elab::collect_uparams(b, univ, universe),
                    _ => b.alloc_levels_slice(&[]),
                };
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
        }
    };
    // ── 第一趟（= 今天）：共用 store，可能出 `Partial` 载体 ──
    let mut decl = match build(builder, val, ctx) {
        Ok(d) => d,
        Err(e) => return replay_after_failure(builder, val, ctx, &mut build, e),
    };
    // ── 声明出口：**收口恰好一次**（闸门关 / 非声明通道 ⇒ 一次判空就走到今天的出口）──
    let Some(_closed) = close_meta_store(ctx) else {
        return finish(builder, decl, val, ctx.metas, |b, v| build(b, v, ctx));
    };
    // ── 第二趟：只在**这一趟真的还有载体**时才做（无载体 ⇒ 零额外 elaborate）──
    let s4_rebuilds = ctx
        .metas
        .is_some_and(|m| !m.table.borrow().entries.is_empty());
    if !s4_rebuilds && declaration_has_meta(builder, &decl) {
        // **第二趟**：同一个 `ctx`（store 已收口）⇒ 约束重放。只在**真的还有载体**时才做
        // （无载体 ⇒ 零额外 elaborate ⇒ 闸门关路径不受影响）。
        //
        // 接受条件两条（缺一条都退回**闸门关重放**）：
        // 1. **无载体**（本片的硬不变式）；
        // 2. **内核接受**（`quiet_catch` 包住：内核用 panic 报拒绝）。第 2 条是**安全网**：
        //    收口给的解不一定对（`default_unresolved` 只能借同形兄弟），错的项进了环境
        //    只会把"一条 `elab-*`"换成"后面全 `unknown const`" ⇒ **绝不比闸门关更坏**。
        match build(builder, val, ctx) {
            Ok(d) if !declaration_has_meta(builder, &d) && kernel_accepts(builder, &d) => decl = d,
            Ok(_) => return replay_gate_off(builder, val, ctx, &mut build),
            Err(e) => return replay_after_failure(builder, val, ctx, &mut build, e),
        }
    }
    // ── S4（B2-a）：**在收口之后/之外** —— 它的重 elaborate 用已收口的 store ──
    let decl = match finish(builder, decl, val, ctx.metas, |b, v| build(b, v, ctx)) {
        Ok(d) => d,
        Err(e) => return replay_after_failure(builder, val, ctx, &mut build, e),
    };
    // ── **硬后置条件**（release 档也拦）：载体一个都不许出门 ──
    if declaration_has_meta(builder, &decl) {
        // 收口已经做过（`closed`），第二趟也试过 —— 只剩"重放闸门关"这一个出口。
        return replay_gate_off(builder, val, ctx, &mut build);
    }
    Ok(decl)
}

/// **type-only 声明的出口**（B2-P4a）：`axiom` / **开放练习**（`theorem … := sorry`）/ 受信前缀
/// ——这类声明的**全部内容**就是 `ty`，所以载体会出现在**类型**里。
///
/// 三趟与 [`build_value`] 同构（第一趟 → **收口恰好一次** → 第二趟 → 硬后置条件 + 闸门关重放）；
/// 差别只有两条：① 没有 `val`；② 没有 S4 的 binder 表（binder 元变量只出现在值位）。
///
/// ⚠ **为什么必须覆盖这条**（实测，`courses/set-theory` 的 gate-on 全课）✗→✓：开放练习的
/// 类型由 `build_axiom`（`elab_type_for_hovers`）elaborate，**不经过** `build_value` ⇒ 载体
/// 直接进 `add_declar` ⇒ 内核硬拒被映射成 `elab-duplicate-declaration`（unit12/unit19 两条
/// 红）。收口 + 重放之后这条路径与闸门关同判 ✓。
pub(crate) fn build_axiom<'a>(
    builder: &mut EnvBuilder<'a>,
    name: &str,
    universe: &[String],
    ty: &Expr,
    known: &crate::compile::elab::KnownTable,
    hovers: &mut Vec<crate::compile::elab::HoverNode<'a>>,
    ctx: &ElabCtx<'a, '_>,
) -> Result<Declar<'a>, CompileError> {
    let hover_mark = hovers.len();
    let mut build = |b: &mut EnvBuilder<'a>,
                     _val: &Expr,
                     ctx: &ElabCtx<'a, '_>|
     -> Result<Declar<'a>, CompileError> {
        hovers.truncate(hover_mark);
        let mut scope = crate::compile::elab::ElabScope::new();
        let univ = crate::compile::elab::make_univ_map(b, universe);
        let ty_kernel = crate::compile::elab::elab_expr(
            b, ty, &mut scope, &univ, known, hovers, None, None, ctx,
        )?;
        let name = b.name_from_str(name);
        let uparams = crate::compile::elab::collect_uparams(b, &univ, universe);
        Ok(Declar::Axiom {
            info: DeclarInfo {
                name,
                uparams,
                ty: ty_kernel,
            },
        })
    };
    // `val` 位没有 ⇒ 传一份空源（闭包不读它）。
    let no_val = Expr::Hole {
        span: Span::default(),
    };
    let decl = match build(builder, &no_val, ctx) {
        Ok(d) => d,
        Err(e) => return replay_after_failure(builder, &no_val, ctx, &mut build, e),
    };
    // **收口恰好一次**（闸门关 / 非声明通道 ⇒ 一次判空就返回 ✓）。
    let Some(_closed) = close_meta_store(ctx) else {
        return Ok(decl);
    };
    if !declaration_has_meta(builder, &decl) {
        return Ok(decl);
    }
    // **第二趟**：同一个 store（已收口）⇒ 约束重放；接受条件 = 无载体 **且** 内核接受。
    match build(builder, &no_val, ctx) {
        Ok(d) if !declaration_has_meta(builder, &d) && kernel_accepts(builder, &d) => Ok(d),
        _ => replay_gate_off(builder, &no_val, ctx, &mut build),
    }
}

/// **声明出口的收口**（B2-P4a）：把声明级 store **收口恰好一次**。
///
/// 顺序（白纸黑字，改之前先读）：`park()` → `unify_all()` → `default_unresolved()` →
/// `unsolved()` → `commit()`。
///
/// * `park()` = 拿到**独占** store（`StoreGuard`；收口期间别的求解拿不到它）；
/// * `unify_all()` = 待定约束跑到不动点（`Undef`/`No` **都不在这里判失败**：声明已经
///   由第一趟的 `discharge_keep_unsolved` 走过一次，这里只负责"把能解的解掉"）；
/// * `default_unresolved()` = E19 选择规则的显式化（**跳过 `SyntheticOpaque`** ✓）；
/// * `unsolved()` = 还剩几个（**读数**；第二趟是否真的没载体由硬后置条件判）；
/// * `commit()` = 收口是**终止动作**（不是探针）⇒ **不许**在 `Drop` 里回滚
///   （否则第二趟看不见任何赋值 ⇒ 等于没做）✓。
///
/// 返回 `None` = **闸门关 / 非声明通道**（`ctx.metas` 没有 store）⇒ 调用方**原样**走今天
/// 的出口（零 park、零重放 ⇒ `--json` 逐字节不变 ✓）。
pub(crate) fn close_meta_store<'a>(ctx: &ElabCtx<'a, '_>) -> Option<MetaClose> {
    // 一次短路：闸门关 ⇒ 连 `DeclMetaCtx` 都不用看（`decl_mvars_enabled` 自己 `OnceLock`）。
    if !decl_mvars_enabled() {
        return None;
    }
    let store = ctx.metas?.store.as_ref()?;
    let unfold = |e: &Expr| {
        crate::spine::unfold_to_inductive(
            e,
            &|n| ctx.inductives.get(n).is_some(),
            ctx.defs,
            8,
            None,
            crate::spine::UnfoldAlign::Short,
        )
    };
    let mut guard = store.park();
    let unsolved = {
        let mut meta = guard.meta_ctx(&unfold);
        let _tri = meta.unify_all();
        let _ = meta.default_unresolved();
        meta.unsolved().len()
    };
    guard.commit();
    Some(MetaClose { unsolved })
}

/// [`close_meta_store`] 的读数（判据读它：`unsolved` = 收口后仍没有赋值的位数）。
pub(crate) struct MetaClose {
    /// 收口（`unify_all` + `default_unresolved`）之后仍然没有赋值的位数。
    #[allow(dead_code)] // 生产路径只判"收口有没有发生"；读数由 `meta_exit/tests.rs` 与
    // `b2p3_*` 的集成测试用（`SOKO_DECL_MVARS=1` 下的结构读数）。
    pub(crate) unsolved: usize,
}

/// **第一趟失败 ⇒ 换成闸门关的诊断**（B2-P4a 第 2 条）。
///
/// 为什么不直接返回第一趟那个错 ✗：第一趟可能**因为载体**失败在别处（载体一进项，
/// 后面的诊断就成了 `unknown identifier` / `kernel-rejected` 那一族）—— 那**不是**
/// 闸门关会说的话。闸门关的诊断只能由**闸门关的路径**给出（`metas: None` ⇒ 构造上
/// 逐字节相同 ✓）。
///
/// 闸门关 / 非声明通道 ⇒ **原样**返回第一趟的错（零重放 ✓）。
fn replay_after_failure<'a>(
    builder: &mut EnvBuilder<'a>,
    val: &Expr,
    ctx: &ElabCtx<'a, '_>,
    build: &mut impl FnMut(
        &mut EnvBuilder<'a>,
        &Expr,
        &ElabCtx<'a, '_>,
    ) -> Result<Declar<'a>, CompileError>,
    first: CompileError,
) -> Result<Declar<'a>, CompileError> {
    if !decl_mvars_enabled() || ctx.metas.is_none() {
        return Err(first);
    }
    replay_gate_off(builder, val, ctx, build)
}

/// **闸门关的重放**：同一份源、同一个 `ctx`，只把声明级 store 摘掉（`metas: None`）。
///
/// 与 [`crate::compile::elab::ElabCtx::without_metas`] 一起构成"诊断逐字节"的**构造性**
/// 保证：闸门关的编译走的**就是**这条路径，重放走的是**同一条**。
fn replay_gate_off<'a>(
    builder: &mut EnvBuilder<'a>,
    val: &Expr,
    ctx: &ElabCtx<'a, '_>,
    build: &mut impl FnMut(
        &mut EnvBuilder<'a>,
        &Expr,
        &ElabCtx<'a, '_>,
    ) -> Result<Declar<'a>, CompileError>,
) -> Result<Declar<'a>, CompileError> {
    // **把进程闸门也按闸门关处理**（`gate_path_active()` 会读这个标志）—— 只摘 `metas`
    // 是不够的：凡按环境变量开闸的旁路都得跟着关（B3 守卫、pp 占位符守卫）。
    let _suppress = GateSuppress::new();
    let off = ctx.without_metas();
    build(builder, val, &off)
}

thread_local! {
    /// **重放期间**置位：这一段里 `gate_path_active()` 要说"闸门关"（`replay_gate_off` 的 RAII）。
    static GATE_SUPPRESSED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// [`replay_gate_off`] 的 RAII：**进入 = 按闸门关处理，退出（含 panic 展开）= 还原**。
struct GateSuppress {
    prev: bool,
}

impl GateSuppress {
    fn new() -> Self {
        let prev = GATE_SUPPRESSED.with(|c| c.replace(true));
        Self { prev }
    }
}

impl Drop for GateSuppress {
    fn drop(&mut self) {
        GATE_SUPPRESSED.with(|c| c.set(self.prev));
    }
}

/// **B2-P4a 的闸门判据（在 ctx / 重放上，不在进程环境变量上）**：
/// `true` = 这一趟要走"声明级元变量"的语义（B3 的作用域守卫开、judge 缓存**不**共槽）。
///
/// ⚠ 三处必须**同一判据**（缺一处就会让重放/闸门关互不相等，实测踩过两次）：
/// 1. `elab.rs::operand_type_expr` 的 **B3 守卫**；
/// 2. `elab.rs::infer_type_text` 的 **judge 缓存**（开闸的文本会带 `⟨meta#N⟩`，
///    与闸门关共槽 ⇒ 重放会解析出**匿名构造子**垃圾 ✗）；
/// 3. [`build_without_store`]（P3a 档重建）—— 它**故意**保持 `true`（守卫要开）。
pub(crate) fn gate_path_active() -> bool {
    decl_mvars_enabled() && !GATE_SUPPRESSED.with(|c| c.get())
}

/// **声明里有元变量吗**（B2-P4a 的硬后置条件 / walk 的静默缺席守卫共用**这一条**判据）。
///
/// 内核那套 `declar_contains_meta` 覆盖 `ty`/`rec_rules`/`Definition`/`Opaque` 的值，
/// **但 `Theorem` 的值不在 `Declar::value()` 里** ⇒ 这里显式补上（与 [`finish`] 的硬闸
/// 同一判据 ✓）。
pub(crate) fn declaration_has_meta(builder: &EnvBuilder<'_>, d: &Declar<'_>) -> bool {
    builder.declar_contains_meta(d) || declar_val(d).is_some_and(has_meta)
}

/// **重建出来的声明内核接受吗**（B2-P4a 的安全网；**只在第二趟**上问）。
///
/// 为什么要它 ✗→✓（实测）：第二趟的项可能**没有载体但类型不对** —— `default_unresolved`
/// 只能借"同形兄弟"（E19 是**选择**不是推理），而收口后的 store 里可能两条约束都没有
/// （P0 形状实测：`m0 := Nat` / `m3 := Set Nat`，`m1`/`m2` 悬着）。那种项进了环境 = 把一条
/// `elab-*` 诊断换成后面全 `unknown const` ✗ ⇒ **接受前先问内核**：不过就退回闸门关 ⇒
/// 诊断**绝不比闸门关更坏** ✓。
///
/// 口径与 `kernel_phase` 的主声明检查同一把：`EnvLimit::ByIndex(当前声明数)`（= 到此刻为
/// 止已进环境的前缀）+ `quiet_catch`（内核用 panic 报拒绝 —— 架构 §8 gotcha 0）。
fn kernel_accepts(builder: &mut EnvBuilder<'_>, d: &Declar<'_>) -> bool {
    let limit = sokonanoda::env::EnvLimit::ByIndex(builder.declaration_count());
    let d = d.clone();
    builder.with_env(|ef| super::super::quiet_catch(|| ef.try_check_declar_at(&d, limit)).is_ok())
}

/// **`add_declar` 的唯一静默入口**（B2-P4a）：替换 `walk` 里 6 处
/// `let _ = builder.add_declar(decl)` ✓。
///
/// **今天的行为逐字保留**：加不进去（重名 / 归纳块未开 / 内核硬拒 …）就**静默不加**
/// —— 判定面不变、闸门关的 `--json` 逐字节不变 ✓。
///
/// **唯一新增**的是一条**硬不变式**（只在 `SOKO_DECL_MVARS` 开时生效）：声明出口（收口 +
/// 重放）之后，**用户声明**里还剩元变量 = P4a 的**内部错误** ⇒ 一条 `kernel-internal`
/// 诊断，**绝不**让它静默缺席（静默缺席 ⇒ `decl_idx` 为空 ⇒ 内核阶段报**假的**
/// `unknown const`，现场见 B2-P0）。
///
/// **合法的载体例外**：judge 的合成声明 `_soko_judge_{k}`（S3 的 `?mN` 通道）——
/// 它本来就**不许进环境**（K1 硬拒不许绕过）⇒ 照旧静默跳过 ✓。
pub(crate) fn add_decl_silent<'a>(
    out: &mut CompileOutput,
    builder: &mut EnvBuilder<'a>,
    decl: Declar<'a>,
    name: Option<&str>,
    cmd: usize,
) {
    if decl_mvars_enabled() && !is_judge_synthetic(name) && declaration_has_meta(builder, &decl) {
        // 内部错误的 span 用空（它没有用户位置可指；`cmd` 已经把诊断绑到那条命令上）。
        out.push_error(
            cmd,
            CompileError::kernel(
                ErrorKind::KernelInternal,
                P4A_META_LEAK_MSG,
                Span::default(),
                None,
            ),
        );
        return;
    }
    let _ = builder.add_declar(decl);
}

/// **P4a 的硬不变式**的报错句（内部错误；只在 `SOKO_DECL_MVARS` 开时可能出现）：
/// 声明出口**之后**还有元变量 ⇒ 声明**绝不**交给 `add_declar`（K1 硬拒 + `decl_idx` 空
/// 会让整份文件出现假的 `unknown const`）。看到这句 = 收口/重放漏了一条路。
pub(crate) const P4A_META_LEAK_MSG: &str = concat!(
    "internal: this declaration still contains metavariable placeholders after the ",
    "declaration exit (B2-P4a invariant; the declaration was not added to the ",
    "environment) — please report this",
);

/// **S3 的合法载体**：judge 的合成声明 `_soko_judge_{k}`（`?mN` 通道）。
///
/// 这类声明**本来就**不许进环境（K1 硬拒含占位符的声明，不许绕过）⇒ `walk` 照旧**静默**
/// 跳过它们 ✓。P4a 的硬不变式（"用户声明绝不许含载体"）用的就是这条排除：收口之后还剩
/// 载体的**用户**声明是**内部错误**，而合成声明是**设计如此** ✓。
fn is_judge_synthetic(name: Option<&str>) -> bool {
    name.is_some_and(|n| n.starts_with("_soko_judge_"))
}

/// **B2-P4a**：同一个 [`ElabCtx`]、**闸门关**的那一份（`metas: None` ⇒ 既没有 S4 的 binder
/// 表、也没有声明级 store）。
///
/// 唯一用途 = 声明出口**重放**一次"闸门关的路径"（诊断因此**构造上**逐字节相同 ✓）：第一趟
/// 可能因为**载体**失败在别处（`unknown identifier` / `kernel-rejected` 那一族），那**不是**
/// 闸门关会说的话。字段全是 `&'b`（Copy）⇒ 只换 `metas` 一个字段。
impl<'a, 'b> ElabCtx<'a, 'b> {
    pub(crate) fn without_metas(&self) -> ElabCtx<'a, 'b> {
        ElabCtx {
            prefix_src: self.prefix_src,
            options: self.options,
            inductives: self.inductives,
            ns: self.ns,
            notations: self.notations,
            defs: self.defs,
            metas: None,
        }
    }
}

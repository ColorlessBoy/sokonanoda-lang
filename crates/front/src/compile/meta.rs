//! **元参数引擎**（IA-4 **M1**）：元变量存储 + **三值合一** + occurs / type-occurs / 作用域 +
//! **有界待定约束** + 出口 `zonk`。
//!
//! 设计 ⇒ `docs/design/metavar-engine.md` §2（§1.2 是 Lean 4 / Coq 的 17 行机制对照）；
//! M0 的基线与 13 形状能力清单 ⇒ `docs/design/metavar-m0.md`（含两条**硬约束**：S13 ⇒ 必须复用
//! delta 展开兜底；13 形状的 26 个读数 = `sibling` 档的回归臂）。
//!
//! **三条设计红线**（照抄设计 §2，改之前先读）：
//! 1. **元变量不进项**：引擎只在**一次求解内**活着，出口 `zonk` 之后不许留 `\0soko_m*`（本模块的
//!    `discharge` 出口自检 + 调用方 `elab_expr` 遇未知名自然报错 = 双保险）；
//! 2. **三值**（`Yes`/`No`/`Undef`）：`Undef` = **弃权**（不是失败）⇒ 进待定队列，等后续赋值再试；
//! 3. **有界**：深度 / 轮数 / 步数三上限 + `unify_all` 的**停条件 = 待定计数不再严格下降**
//!    （与 Lean `processPostponed` 同款）；**超限绝不改变结论** ✓ —— 撞预算 ⇒ **加大预算重试** ✓
//!    （G-88 真修，2026-10-04 ✓），升满仍撞 ⇒ **弃权 `Tri::Undef`** ✓（**不是**「按无解处理」✗）。
//!
//! 元变量的**表示**：保留前缀的 fresh 名 `\0soko_m{id}`（`\0` 不可能出现在源标识符里 ⇒ 与
//! `implicit::telescope` 的 `\0soko_p{i}` 同一纪律）⇒ `spine::substitute`/`mentions`/`same_shape`
//! **一个字都不用改**就能在模板里代换元变量。

use std::cell::RefCell;
use std::collections::HashMap;

use crate::ast::Expr;
use crate::span::Span;

/// 元变量名的保留前缀（`\0` 前缀不可能与用户标识符相撞）。
const META_PREFIX: &str = "\u{0}soko_m";

/// 元变量的名字（`\0soko_m{id}`）。
pub(crate) fn meta_name(id: u32) -> String {
    format!("{META_PREFIX}{id}")
}

/// 这个名字是不是元变量？（`zonk` 之后不许再出现 ⇒ 出口自检用它。）
/// **S3**：内部回读通道的元变量引用 `?m<digits>` ⇒ id（用户源码不会出现这种名字）。
pub(crate) fn meta_ref_id(name: &str) -> Option<u32> {
    name.strip_prefix("?m")?.parse::<u32>().ok()
}

pub(crate) fn is_meta_name(name: &str) -> bool {
    name.starts_with(META_PREFIX)
}

/// **B2-P2（2026-10-25）**：`\0soko_m{id}` ⇒ **绝对 id**（不是元变量名 / 尾数不是纯数字 ⇒ `None`）。
///
/// ⚠ 与 [`meta_ref_id`]（`?mN`）**是两条通道**：那条的类型来自 judge 的登记表
/// （`meta_registry`），这条来自**声明级 store**（P1）。两条都先于 unknown-identifier。
pub(crate) fn decl_meta_id_of(name: &str) -> Option<u32> {
    name.strip_prefix(META_PREFIX)?.parse::<u32>().ok()
}

// ───────────────────────── U1：宇宙层的元变量（§2.9）─────────────────────────
//
// **U1 第 1 片（2026-10-05）**：先落**编码**与 **occurs** —— 与元变量同款
// （`\0` 前缀的保留名 ⇒ 与用户标识符**永不撞车** ✓）。
//
// **Lean 4 对照** ✓（本机源码 HEAD `d0493e4c1e` ✓）：
// * Lean 的层是 **`Level.mvar LMVarId`**（`Lean/Level.lean`），由
//   `Meta/LevelDefEq.lean` 的 `isLevelDefEqAux`/`solve` **合一**（含 `solveSelfMax`）；
// * **刚性宇宙约束推迟**到项元变量被赋值（`postponeIsLevelDefEq`）；
// * 赋值处有 **occurs**（`u` 出现在自己的解里 ⇒ 拒）。
//
// ⚠ **偏离 1 条（白纸黑字）**：Lean 用**独立的 `LMVarId` 类型**，我们用**名字编码**
// （`\0soko_u{id}`）—— 与 `MetaId` 同款理由：源级表示**一个字都不用改**
// （`spine::substitute`/`mentions`/`same_shape` 全按名字走），面更小。
// ⚠ **本片零行为变化**：没有任何生产调用方；快路径「`SortKind` 字面相等 ⇒ Ok」原样保留。

/// 层元变量名的保留前缀（与 `META_PREFIX` **不同** ⇒ 两类元变量不会互相误判）。
const LEVEL_PREFIX: &str = "\u{0}soko_u";

/// 层元变量的名字（`\0soko_u{id}`）。
#[allow(dead_code)] // U1 第 2 片（约束存储 + 合一）会用
pub(crate) fn level_name(id: u32) -> String {
    format!("{LEVEL_PREFIX}{id}")
}

/// 这个名字是不是层元变量？
#[allow(dead_code)] // U1 第 2 片会用
pub(crate) fn is_level_name(name: &str) -> bool {
    name.starts_with(LEVEL_PREFIX)
}

/// **R1b（2026-10-05）**：分配一个 **fresh 层元变量 id** ✓ ——
/// **对齐 Lean `mkFreshLevelMVar`**（`Meta/LevelDefEq.lean` 用它给每个宇宙位一个 mvar ✓）。
///
/// ⚠ **进程内单调递增** ✓（Lean 的 `LMVarId` 也是全局计数器 ✓）；
/// ⚠ **本片不接线** ✗ ⇒ **零行为变化** ✓。
#[allow(dead_code)] // R1c（出口 levelMVarToParam）接线
pub(crate) fn fresh_level_mvar_id() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

/// **R2b-1（2026-10-05）**：**本声明的待解层元变量表** ✓ ——
/// **对齐 Lean 的 `MCtx`**（层 mvar 的赋值状态挂在 context 上 ✓，
/// 出口 `levelMVarToParam` 读它 ✓）。
///
/// **R2b-2（2026-10-05）**：表项从裸 `id` 扩成 **`(id, 解出的层文本)`** ✓ ——
/// `None` = 还没解出 ✓（对齐 `MCtx` 的 `getLevelMVarAssignment? ⇒ none` ✓）、
/// `Some(text)` = 应用处合一解出 ✓（`some v => visitLevel v` ✓）。
///
/// ⚠ 用 `thread_local` 承载「当前声明」的待解表 ✓（编译是单线程 ✓）。
/// ⚠ **出口的权威来源是「声明 `ty`/`val` 里真的出现的 mvar」** ✗ —— 本表只做记账 ✓：
/// elaborate 期间会为**丢弃的**中间项（`infer_type_text_inplace` 的 scratch 作用域 ✓）
/// 也生成 mvar ⇒ 表里会有**不属于本声明**的 id ✓ ⇒ 出口按 `collect_level_mvars_expr`
/// 的结果走 ✓（对齐 Lean：`levelMVarToParam` 只走**声明那个项** ✓）。
pub(crate) mod level_mvar_table {
    use std::cell::RefCell;

    thread_local! {
        /// 当前声明里生成的层 mvar（保序 ✓）：`(id, 解出的层文本)` ✓。
        static PENDING: RefCell<Vec<(u64, Option<String>)>> = const { RefCell::new(Vec::new()) };
    }

    /// 记一个层 mvar id（生成处调 ✓ —— 对齐 `mkFreshLevelMVars` ✓）。
    pub(crate) fn push(id: u64) {
        PENDING.with(|p| p.borrow_mut().push((id, None)));
    }

    /// 记一个**解出的**层（应用处合一成功时调 ✓ —— 对齐 `assignLevelMVar` ✓）。
    pub(crate) fn solve(id: u64, text: &str) {
        PENDING.with(|p| {
            let mut pending = p.borrow_mut();
            if let Some(entry) = pending.iter_mut().rev().find(|(i, _)| *i == id) {
                entry.1 = Some(text.to_string());
            }
        });
    }

    /// **取出并清空**（声明收口处调 ✓ —— 对齐 Lean 的「出口」✓）。
    pub(crate) fn take() -> Vec<(u64, Option<String>)> {
        PENDING.with(|p| std::mem::take(&mut *p.borrow_mut()))
    }
}

// ─────────────────── S4/B2-a：**声明级**元变量表（binder 类型）───────────────────
//
// **为什么不用 `thread_local`**（与上面的层元变量表不同 ✗）：judge 的 `judge_infer`
// 在**同一线程**里递归跑 walk（合成一份文档）⇒ thread_local 会把外层声明的元变量
// 漏进合成声明 ✗（施工图 R4）。⇒ 由 `walk` 的处理器在**栈上**持有、经
// `ElabCtx.metas` **显式**传进来（词法作用域 ⇒ 结构上不可能泄漏 ✓）。

/// 一条声明里造出来的 binder 类型元变量（**只是记账** ✓；权威来源 = 声明里真的出现的 id）。
pub(crate) struct DeclMeta {
    pub id: u32,
    /// 那个**未注解 binder** 的源 span（出口报错时逐字复用它 ✓）。
    pub span: Span,
}

/// 一次声明的元变量表（由 `walk` 的处理器**栈上**持有，经 `ElabCtx.metas` 传进来）。
#[derive(Default)]
pub(crate) struct DeclMetas {
    pub entries: Vec<DeclMeta>,
}

/// **B2-P1 · R2**：两种 id 命名空间的**分界** —— 引擎（`MetaCtx::fresh`）的 id 从 0 往上长
/// （`MetaStore.base`），S4 的 `fresh_decl_meta_id` 从**这里**往上长 ⇒ **永不撞名** ✓。
///
/// ⚠ 为什么必须分：两者都编成 `\0soko_m{id}`，而 S4 的 id 还会进**核项** `Expr::Meta{id}`
/// （`b2_meta_pair` 靠 id 把源/核两侧接起来）⇒ 撞名就是**串味**（B2 的 P2 载体一接线就会真咬 ✗）。
/// ⚠ 上限：S4 用掉 `2^32 - 2^30` 个 id 才回绕（进程内不可能）；引擎一侧的 id 见 [`SCRATCH_BLOCK`]。
pub(crate) const DECL_META_ID_BASE: u32 = 1 << 30;

/// **B2-P1**：**重入**（嵌套）求解的 scratch store 每层占的 id 块大小（见 [`DeclMVarStore`] 不变量 3）。
pub(crate) const SCRATCH_BLOCK: u32 = 1 << 12;

/// 进程内单调的元变量 id（**S4 命名空间**：`[DECL_META_ID_BASE, 2^32)`）——
/// **不要**用 `MetaCtx::fresh` 的 `store.mvars.len()` ✗（那是**引擎**命名空间，从 0 起；
/// R2：两者混用会在同一条声明里撞 `\0soko_m{id}`）。
pub(crate) fn fresh_decl_meta_id() -> u32 {
    use std::sync::atomic::{AtomicU32, Ordering};
    static NEXT: AtomicU32 = AtomicU32::new(0);
    DECL_META_ID_BASE + NEXT.fetch_add(1, Ordering::Relaxed)
}

/// **S4 逃生门**：`SOKO_DECL_METAS=1` 才开（**默认关** ⇒ 今天的行为逐字节不变 ✓）。
pub(crate) fn decl_metas_enabled() -> bool {
    use std::sync::OnceLock;
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var("SOKO_DECL_METAS").is_ok_and(|v| v != "0"))
}

/// **B2 闸门**：**默认开**（`SOKO_DECL_MVARS=0` 逃生 ⇒ 每个求解点仍拿自己的
/// `MetaStore::default()`，`--json`/golden **逐字节**等于旧行为 ✓）。只读一次 ✓。
pub(crate) fn decl_mvars_enabled() -> bool {
    use std::sync::OnceLock;
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var("SOKO_DECL_MVARS").map_or(true, |v| v != "0"))
}

/// 一条声明的元变量上下文（**S4 的 binder 表 + B2-P1 的 store**，同一份句柄）；
/// 由 `walk` 的四个声明处理器**栈上**持有，经 `ElabCtx.metas` 传进来。
///
/// ⚠ 两者**同一个** `Option`：非声明通道（`#check`/judge/预置库…）传 `None` ⇒
/// 既没有 S4 表、也没有声明级 store（词法作用域 ⇒ 元变量**不可能**漏进非声明通道 ✓）。
pub(crate) struct DeclMetaCtx {
    /// S4/B2-a 的 binder 类型元变量表（**只是记账** ✓；权威来源 = 声明里真的出现的 id）。
    pub table: RefCell<DeclMetas>,
    /// **B2-P1**：声明级 `MetaStore` 的句柄（闸门关 ⇒ `None` ⇒ 一次求解一个 store ✓）。
    pub store: Option<DeclMVarStore>,
    /// **B2-P2**：**正在展开声明类型**的元变量 id（`\0soko_m{id}` ⇒ `Expr::Meta` 那一刻）。
    /// 自引用/环的守卫：同一个 id 再进来 ⇒ 拒（走今天的 unknown-identifier）✓。
    expanding: RefCell<Vec<u32>>,
}

impl DeclMetaCtx {
    fn build(store: Option<DeclMVarStore>) -> Self {
        Self {
            table: RefCell::new(DeclMetas::default()),
            store,
            expanding: RefCell::new(Vec::new()),
        }
    }

    /// 闸门在这里**读一次**（`decl_mvars_enabled` 自己 `OnceLock`）—— 声明处理器只调本函数。
    pub(crate) fn new() -> Self {
        Self::build(decl_mvars_enabled().then(DeclMVarStore::new))
    }

    /// 单测用：**显式**给档位（不走环境 ⇒ 无全局竞态 ✓ —— 见 `scripts/check-test-env-isolation.py`
    /// 的纪律；闸门 ON/OFF 两半都这样测）。
    #[cfg(test)]
    pub(crate) fn with_flag(decl_mvars: bool) -> Self {
        Self::build(decl_mvars.then(DeclMVarStore::new))
    }

    /// **B2-P2**：按**绝对 id** 只读查声明级 store 里的 `(声明类型, 种类)` ✓。
    ///
    /// ⚠ 闸门关（`store: None`）/ store 正被外层求解 park 走（嵌套求解）/ id 不在本
    /// store ⇒ `None` ⇒ 调用方回落到**今天**的「未知标识符」路 ✓（**绝不**发明类型 ✗）。
    pub(crate) fn lookup_meta(&self, id: u32) -> Option<(Expr, MetaKind)> {
        self.store.as_ref()?.lookup(id)
    }

    /// **B2-P2**：进入「正在展开 `id` 的声明类型」——`None` = **自引用/环** ⇒ 拒。
    ///
    /// 守卫是 RAII（[`MetaExpandGuard`]）⇒ **出错 / panic 展开都弹栈** ✓（不毒化后续展开）。
    pub(crate) fn enter_meta_expansion(&self, id: u32) -> Option<MetaExpandGuard<'_>> {
        let mut expanding = self.expanding.borrow_mut();
        if expanding.contains(&id) {
            return None;
        }
        expanding.push(id);
        Some(MetaExpandGuard { decl: self, id })
    }
}

/// 让既有代码（`table.borrow_mut().entries` / `finish(…, ctx.metas, …)`）在换成
/// [`DeclMetaCtx`] 后**一个字不用改** ✓（表仍是那个 `RefCell<DeclMetas>`）。
impl std::ops::Deref for DeclMetaCtx {
    type Target = RefCell<DeclMetas>;
    fn deref(&self) -> &Self::Target {
        &self.table
    }
}

/// 层元变量的 id（不是层元变量名 ⇒ `None`）。
#[allow(dead_code)] // U1 第 2 片会用
pub(crate) fn level_id_of(name: &str) -> Option<u32> {
    name.strip_prefix(LEVEL_PREFIX)?.parse().ok()
}

/// **occurs 检查**（对齐 Lean 赋值处的那一条）：`u` 出现在自己的解里 ⇒ **拒**。
///
/// ⚠ **作用域是「层文本」不是「整项」** —— 层的解永远是**层表达式**（`u` / `u+1` /
/// `max u v`），而层只出现在 `Sort` 位 ⇒ 检查**那段文本**就够，**不必**写整棵 AST 的
/// 遍历（源 AST 没有通用子项遍历，写了就是大改）。
///
/// ⚠ **为什么必须有**：没有它，`u := u + 1` 这类赋值会让 `zonk` 的链式迭代
/// **不终止或给错值** ⇒ 这是**正确性**要求，不是优化。
///
/// 判据（词边界）：`u` 命中 · `u1` **不**命中 · `uu` **不**命中 · `max u v` 命中。
#[allow(dead_code)] // U1 第 2 片（赋值）会用
pub(crate) fn level_occurs_in_text(target: u32, text: &str) -> bool {
    let needle = level_name(target);
    let is_ident = |c: char| c.is_alphanumeric() || c == '_' || c == '\'';
    let bytes = text.as_bytes();
    let nb = needle.as_bytes();
    let mut i = 0usize;
    while i + nb.len() <= bytes.len() {
        if &bytes[i..i + nb.len()] == nb {
            let before_ok = i == 0 || !is_ident(text[..i].chars().next_back().unwrap_or(' '));
            let after_ok = i + nb.len() == bytes.len()
                || !is_ident(text[i + nb.len()..].chars().next().unwrap_or(' '));
            if before_ok && after_ok {
                return true;
            }
        }
        i += 1;
    }
    false
}

/// **U1 第 3 片（2026-10-05）**：层元变量的**赋值**（对齐 Lean `LevelDefEq.solve` 的
/// `!u.occurs v ⇒ assign u := v` 那一条 ✓）。
///
/// **Lean 4 对照** ✓（本机源码 HEAD `d0493e4c1e` ✓，`Meta/LevelDefEq.lean:90-125` ✓）：
/// ```lean
/// | Level.mvar mvarId, _ =>
///     if readOnly ⇒ undef
///     else if !u.occurs v ⇒ assign u := v; true       ← **本片落的这一条** ✓
///     else if v.isMax && !strictOccursMax u v ⇒ solveSelfMax …   ← **推迟** ✗
///     else ⇒ undef
/// ```
///
/// ⚠ **推迟的两档（白纸黑字）** ✗：`solveSelfMax`（`?m =?= max ?m v` ⇒ fresh `?n` + `?m := max ?n v` ✓）
/// 与 `tryApproxSelfMax` 都要**解析**层文本 ✗ —— 我们的 `Level` 是**文本**不是结构 ✓
/// （Lean 是 `Level.mvar`/`max`/`succ` 结构 ✓）⇒ 留到 U1 后续片或 U2 ✓。
///
/// 返回值：`Ok(())` 赋值成功 ✓ · `Err(())` **弃权**（occurs 命中 ⇒ 不许赋值 ✗，对齐 `LBool.undef` ✓）。
#[allow(dead_code)] // U1 第 5 片（接线到 Sort/Level 路径）会用
pub(crate) fn level_assign(
    values: &mut Vec<Option<String>>,
    id: u32,
    text: &str,
) -> Result<(), ()> {
    if level_occurs_in_text(id, text) {
        // **occurs 命中 ⇒ 拒** ✗（`u := u + 1` 会让 zonk 不终止或给错值 ✓）。
        return Err(());
    }
    let i = id as usize;
    if values.len() <= i {
        values.resize(i + 1, None);
    }
    values[i] = Some(text.to_string());
    Ok(())
}

/// **U1 第 4 片（2026-10-05）**：宇宙约束的**惰性批量检查**（§2.9：「**对齐 Coq**：
/// 约束攒着，声明收尾才查 ⇒ 不逐次合一都查图」✓）。
///
/// **Lean 4 / Coq 对照** ✓（本机源码 HEAD `d0493e4c1e` ✓）：
/// * **Lean** 侧：`Meta/LevelDefEq.lean:77` `postponeIsLevelDefEq` —— **刚性宇宙约束推迟**
///   到项元变量被赋值 ✓；最终由 `LevelDefEq` 的检查收口 ✓；
/// * **Coq** 侧：`UState` + `UnivProblem` 约束集 + **惰性批量检查**（§4 的 #8 行 ✓，
///   `check_univ_implication` 是**那一列**的说法 ✗ —— **不是** Lean 的 ✓）；
/// * **我们**：§2.9 明说「**对齐 Coq**」✓ ⇒ 本片落**批量检查**的形状 ✓。
///
/// ⚠ **本片只落「能定的判掉」** ✓（零行为变化 ✓）：约束两侧**都已有赋值**时当场判
/// （`Eq` 相等 / `Le` 数值 ≤ ✓）；**任一侧没赋值 ⇒ 弃权**（留给后续片 ✓，对齐 `LBool.undef`
/// 的「不知道」而不是「失败」✓ —— 报成失败会**假红** ✗）。
///
/// 返回：`Ok(())` 全部可判且成立 ✓ · `Err(UnivUnsolved::Clash)` **已判否** ✗ ·
/// `Err(UnivUnsolved::Pending)` **还有没赋值的** ✓（声明出口才需要它变成错 ✗ —— 本片不接 ✓）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[allow(dead_code)] // U1 第 5 片（接线）会用
pub(crate) enum UnivUnsolved {
    /// 有约束被**判否**（真的冲突 ✗）。
    Clash,
    /// 还有约束**定不了**（两侧没赋值完 ✓）。
    Pending,
}

#[allow(dead_code)] // U1 第 5 片（接线）会用
pub(crate) fn univ_check(
    constraints: &[UnivConstraint],
    values: &[Option<String>],
) -> Result<(), UnivUnsolved> {
    let val = |id: u32| -> Option<u64> {
        values
            .get(id as usize)
            .and_then(|v| v.as_ref())
            .and_then(|t| t.trim().parse::<u64>().ok())
    };
    let mut pending = false;
    for c in constraints {
        match *c {
            UnivConstraint::Eq { lhs, rhs } => match (val(lhs), val(rhs)) {
                (Some(a), Some(b)) if a == b => {}
                (Some(_), Some(_)) => return Err(UnivUnsolved::Clash),
                _ => pending = true,
            },
            UnivConstraint::Le { lhs, rhs } => match (val(lhs), val(rhs)) {
                (Some(a), Some(b)) if a <= b => {}
                (Some(_), Some(_)) => return Err(UnivUnsolved::Clash),
                _ => pending = true,
            },
            // ⚠ `Lub` 要**算 max** ✗（不是判关系 ✓）⇒ 本片**弃权** ✓（留给后续片 ✓）。
            UnivConstraint::Lub { .. } => pending = true,
        }
    }
    if pending {
        Err(UnivUnsolved::Pending)
    } else {
        Ok(())
    }
}

/// 层文本的 **zonk**：把已赋值的层元变量代进文本（有界迭代，链式赋值 ✓）。
///
/// ⚠ **有界**（`LEVEL_ZONK_MAX` ✓）：与项元变量的 `zonk` 同款纪律 ✓ —— 环由 `level_assign`
/// 的 occurs 闸**从源头**挡住 ✓，这里的上界是**兜底** ✓（真出环也不会挂死 ✓）。
#[allow(dead_code)] // U1 第 5 片会用
pub(crate) fn level_zonk_text(values: &[Option<String>], text: &str) -> String {
    const LEVEL_ZONK_MAX: usize = 32;
    let mut cur = text.to_string();
    for _ in 0..LEVEL_ZONK_MAX {
        let mut changed = false;
        for (i, v) in values.iter().enumerate() {
            let Some(val) = v else { continue };
            if level_occurs_in_text(i as u32, &cur) {
                cur = cur.replace(&level_name(i as u32), val);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    cur
}

/// 元变量 id（**只在一次求解内有效**，不跨调用、不进项）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct MetaId(pub u32);

/// 元变量的**种类**（对齐 Lean 的 `MetavarKind`）：
/// `Natural` = 可被合一赋值；`SyntheticOpaque` = **永不被类型约束填**（洞的语义，M1 只留接缝）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[allow(dead_code)] // `SyntheticOpaque` 的用武之地在 M2/B2（洞的语义）
pub(crate) enum MetaKind {
    Natural,
    SyntheticOpaque,
}

/// **B2-P2**：前端的 [`MetaKind`] ⇒ **内核**的 `MetaKind`（K1 的 `Expr::Meta` 要这个）。
///
/// ⚠ 两种表示**只有 id 与 kind 相同**才对得上（源侧 `\0soko_m{id}` / 核侧 `Expr::Meta`）✓。
pub(crate) fn kernel_meta_kind(kind: MetaKind) -> sokonanoda::expr::MetaKind {
    match kind {
        MetaKind::Natural => sokonanoda::expr::MetaKind::Natural,
        MetaKind::SyntheticOpaque => sokonanoda::expr::MetaKind::SyntheticOpaque,
    }
}

/// 三值结果（对齐 Lean 的 `LBool`）：`Yes` 成立 · `No` 不成立 · `Undef` **弃权**（不是失败）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Tri {
    Yes,
    No,
    Undef,
}

/// 引擎的硬错误（`unify` 会把它们折成 `Tri::No`；单测直接看它）。
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum MetaErr {
    /// occurs check：元变量出现在自己的解里
    Occurs,
    /// **type-occurs**：元变量出现在「解里提到的另一个元变量」的**类型**里（Lean 的 `typeOccursCheck`）
    TypeOccurs,
    /// **作用域**：解里提到了该元变量作用域外的名字（与 occurs 分开报 —— 两者是不同 bug）
    Scope,
    /// **sort/kind 不对**（M3）：值的 sort 与元变量声明类型的 sort **确定**不符
    /// （只拒"确定错"的；不知道就放行 ⇒ 不可能产生假拒绝 ✓）
    Kind,
    /// **刚性冲突**（两侧头不同、且都没有可赋值的元变量）
    Clash,
}

/// 一次求解的**结局**（M3 的"三通道"；**不是**新的错误码 —— 用户可见的码仍是既有两条，
/// 通道只决定 **hint 说哪一句**，见 `docs/design/metavar-engine.md` §2.6）：
/// `Unsolved` = 补不出（既有语义）· `Kind` = 值的 sort 确定不对 · `Clash` = 两条约束刚性冲突。
#[derive(Debug)]
pub(crate) enum MetaSolve {
    Solved(Vec<Expr>),
    /// **B2-P3b**：共用 store 的部分出口 —— 值里允许留未解元变量（`still` = 载体 id）。
    #[allow(dead_code)] // P3b：`still` 由 P4（声明末 zonk）读；本片只按"非空 ⇒ Partial"用。
    Partial(Vec<Expr>, Vec<MetaId>),
    Unsolved,
    Kind,
    Clash,
}

impl MetaSolve {
    /// 给只关心"成没成"的调用方（`solve_prefix` 的三个调用方里有两个不报错）。
    pub(crate) fn into_option(self) -> Option<Vec<Expr>> {
        match self {
            MetaSolve::Solved(v) => Some(v),
            _ => None,
        }
    }
}

struct MVar {
    /// **HO3a**：**创建时**的局部上下文（`fresh` 时的 `MetaCtx.lctx`）—— pattern 赋值判作用域用。
    lctx: pattern::Lctx,
    /// 声明类型（= 该前导层的**域**，已把更早的元变量代进去）。
    /// **B2-P2**：载体按绝对 id 读它（[`DeclMVarStore::lookup`]）。
    ty: Expr,
    /// 赋值（`None` = 未解）
    value: Option<Expr>,
    /// 种类（M1 只有 `Natural` 会真的被赋值；`SyntheticOpaque` 的接缝留给 M2/B2）。
    /// **B2-P2**：载体按绝对 id 读它（核侧 `Expr::Meta` 的 `kind` 与解同源 ✓）；**B2-P4a**：defaulting 也读它。
    kind: MetaKind,
    /// 作用域外的名字（= 该位**更晚**的望远镜参数名；赋值时不许提到它们）
    out_of_scope: Vec<String>,
}

/// `unify_all` 的轮数上限（**另一族** ✗ —— 它是「重扫待定队列到不动点」的**轮数**闸 ✓，
/// 与 Lean 的 `maxRecDepth`（递归深度）/ `synthInstance.maxHeartbeats`（步数）都不是一回事 ✗
/// ⇒ 本轮**只登记、不动数值** ✓；台账里把它写成 `maxSynthDepth 8 → 32`，但**本机 Lean
/// 没有 `maxSynthDepth` 这个选项** ✗（`grep -rn maxSynthDepth src/` = 0 ✓）⇒ 留待下一笔 ✓）。
const MAX_ROUNDS: u32 = 8;

// ── **求解预算三旋钮**（G-88 第二笔 · 2026-10-07 ✓）──────────────────────────
//
// ⚠ 取值**不再写在这个文件里** ✗ —— 登记在**闸取值登记表** `sokonanoda::gates::limits` ✓
// （与 `max_tracked` **同一处** ✓、同一套 `SOKO_LIMIT_*` 命名 ✓、同一份
// `env_u32` + `OnceLock` 解析纪律 ✓ —— 非法值/缺省 ⇒ 默认值 ✓）：
//
// | 旋钮（本文件） | 环境变量 | 默认值 | Lean 出处 |
// |---|---|---|---|
// | `fuel` | `SOKO_LIMIT_MAX_HEARTBEATS` | 20000 | `synthInstance.maxHeartbeats`（`Meta/SynthInstance.lean:20`） |
// | `max_depth` | `SOKO_LIMIT_MAX_REC_DEPTH` | 512 | `maxRecDepth`（`Util/RecDepth.lean:15` ⇒ `Init/Prelude.lean:4760`） |
// | `escalations` 上界 | `SOKO_LIMIT_MAX_ESCALATIONS` | 6 | **无对应物** ✗（Lean 耗尽即报错） |
//
// ⚠ **语义一个字没动** ✓：撞预算 ⇒ **加大预算重试** ✓（= 只变慢 ✓），升满仍撞 ⇒
// **弃权 `Tri::Undef`** ✓（= 「判不了 ⇒ 走慢路」✓），**绝不** `Tri::No` ✗
// （= 「判不了 ⇒ 当成否」✗）。本轮只改**数值与来源** ✓。

/// `zonk` 的链式代换轮数上限（赋值链 `?a := ?b`、`?b := Nat` 这种）。
const MAX_ZONK_ROUNDS: u32 = 8;

/// 求解上下文（**一次 `solve_prefix` 一个**；严格档一次都不构造 ⇒ 默认路径零分配 ✓）。
/// **IA-4 B2（2026-10-05 ✓）**：元变量的**数据**（**无生命周期** ✓）——
/// 它要**跨一次求解调用**活着 ✓（用户 00:05：「允许活过一次求解调用」✓），
/// 所以从 `MetaCtx` 里抽出来 ✗（ctx 带 `unfold` 的生命周期 ✗，数据不带 ✓）。
/// ⚠ 字段全是 **owned** ✓（`MVar` 存 `Expr` ✓、`postponed` 存 `(Expr, Expr)` ✓）⇒ **零生命周期** ✓。
#[derive(Default)]
pub(crate) struct MetaStore {
    /// **B2-P1**：本 store 的 id 块**起点**（`MetaId = base + mvars 下标`）。
    ///
    /// `0` = 今天的行为（每次求解新建的 store / 声明级的**唯一** store ✓）；
    /// **重入**求解的 scratch store 从 `DECL_META_ID_BASE - depth*SCRATCH_BLOCK` 起
    /// ⇒ 与外层 store、S4 的 id **互不撞名** ✓（R2；见 [`DeclMVarStore`] 不变量 3）。
    base: u32,
    mvars: Vec<MVar>,
    /// 待定约束（`Undef` 的叶子自己压进来；`unify_all` 重扫到不动点）
    postponed: Vec<(Expr, Expr)>,
    /// **层元变量的赋值表**（U1 第 3 片 ✓）—— `level_values[id] = Some(层文本)` ✓。
    ///
    /// ⚠ **表示差异（白纸黑字）** ✗：Lean 的 `Level` 是**结构化**的（`Level.mvar`/`max`/`succ` ✓），
    /// 我们的是**文本**（源 AST 的 `SortKind::Level(String)` ✓）⇒ 本片只落**核心档** ✓：
    /// `?u := <文本>` ✓（`occurs` 闸用第 1 片的 `level_occurs_in_text` ✓）；
    /// ⚠ **`max` 两档推迟** ✗（`solveSelfMax` / `tryApproxSelfMax` 要**解析**层文本 ✓，
    /// 而层文本的解析器今天在前端的 `level_ptr` 里、是**内核层指针**那一侧 ✗）。
    #[allow(dead_code)] // U1 第 5 片（接线）才读；本片只有单测用
    pub(crate) level_values: Vec<Option<String>>,
    /// **宇宙约束**（U1 第 2 片 ✓；§4 的 #6 行：「`postponed` 含宇宙约束，**U1 起分表**」✓）。
    ///
    /// ⚠ **为什么分表** ✗：项约束与层约束的**求解时机不同** ✓ —— 层约束要
    /// **推迟到项元变量被赋值之后**（Lean 的 `postponeIsLevelDefEq` ✓，§2.9 ✓），
    /// 而项约束是 `unify_all` 当场重扫 ✓ ⇒ 混在一起会让层约束被**过早**求解 ✗。
    ///
    /// ⚠ **本片只落存储** ✓（零行为变化 ✓ —— 没有生产调用方 ✓，合一与惰性检查是 ③/④ ✓）。
    #[allow(dead_code)] // U1 第 3/4 片（合一 + 惰性检查）才读；本片只有单测用
    pub(crate) univ: Vec<UnivConstraint>,
}

/// **一次求解的回滚点**（`mvars`/`postponed` 的长度；B2-P1 的「失败不落盘」用它）。
#[derive(Clone, Copy)]
pub(crate) struct StoreMark {
    mvars: usize,
    postponed: usize,
}

impl MetaStore {
    /// 指定 id 块起点的 store（见 [`MetaStore::base`]；`0` = 默认/今天的行为）。
    pub(crate) fn with_base(base: u32) -> Self {
        Self {
            base,
            ..Self::default()
        }
    }

    pub(crate) fn mark(&self) -> StoreMark {
        StoreMark {
            mvars: self.mvars.len(),
            postponed: self.postponed.len(),
        }
    }

    /// 把 store 退回到 `mark`（**只删本次求解造的**；更早的求解已经在 `commit` 里定下来了 ✓）。
    ///
    /// ⚠ 只覆盖 `mvars`/`postponed`：求解路径不写 `level_values`/`univ`（U1 接线时**必须**
    /// 在这里补上，否则失败求解会漏掉层约束 ✗）。
    pub(crate) fn rollback_to(&mut self, mark: StoreMark) {
        self.mvars.truncate(mark.mvars);
        self.postponed.truncate(mark.postponed);
    }

    /// 已经攒了多少个元变量（P1 判据：跨求解**只增不减** ⇒ 共用了一个 store ✓）。
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.mvars.len()
    }
}

/// **B2-P1（2026-10-25）**：声明级 `MetaStore` 的句柄 —— **独占式换出（swap-out）**。
///
/// ## R1：为什么不能直接 `RefCell<MetaStore>` 借一辈子
/// 求解会**重入** `elab_expr`（`alt_type`/`ty_of` 闭包，`implicit::solve_prefix_meta_alt`
/// 的 ③/⑤ 与 `MetaCtx::refine_by_types`）；若外层求解**持有** `borrow_mut()` 跨过那次重入，
/// 内层求解再借就 **panic** ✗。⇒ 句柄里只有一个**槽**：外层求解把 store **取走**（槽变 `None`，
/// 此后**不持有任何 `RefCell` 借用**），结束（含 panic 展开）时再**放回**。
///
/// ## 不变量（改这个类型前先读）
/// 1. 槽里 `Some` ⇒ 没有人在用（只能被 [`DeclMVarStore::park`] 取走）；
/// 2. 槽里 `None` ⇒ 外层求解正**独占**着 ⇒ 内层（重入）求解拿**自己的 scratch store**
///    （= 今天「一次求解一个 store」的行为 ✓），**重叠借用不可能发生** ✓；
///    ⇒ **嵌套求解看不到外层的约束/赋值**（P1 的边界：跨求解的**载体**是 P2、声明末 zonk 是 P4 ✗）；
/// 3. scratch store 的 id 从**错开的块**下来（`DECL_META_ID_BASE - depth*SCRATCH_BLOCK`）
///    ⇒ 与外层 store（从 0 往上）和 S4（从 `DECL_META_ID_BASE` 往上）**永不撞名** ✓；
///    块内最多 `SCRATCH_BLOCK` 个元变量（一次求解只造 `k` 个前导位 ⇒ 远小于 4096 ✓）；
///    深度上限 `2^30/2^12 = 262144` 层（栈深不可能到）；
/// 4. **只有成功（`Solved`）的求解才留在 store 里**：`StoreGuard` 默认在 `Drop` 里回滚到
///    进场时的 `mark`，求解点只在 `discharge` 判 `Solved` 时 `commit()` ⇒ 失败的**探针**
///    求解（`apply`/期望类型回退里很常见）不会毒化后面的求解 ✓。
///    ⚠ 这**不是** P3（部分出口）：P3 是「**允许**未解元变量活到声明末尾」，这里恰恰相反。
pub(crate) struct DeclMVarStore {
    slot: RefCell<StoreSlot>,
    /// **B2-P3a**：`(ty, kind)` 的**停车可见**类型日志（绝对 id 索引；`lookup` 只读它、不读槽 ✓；回滚时随 `mvars` 截断）。
    log: RefCell<Vec<(Expr, MetaKind)>>,
    /// **HO3a/HO3b**：当前求解点的上下文快照（`pattern::note_ctx` 写、`park` 读 ⇒ `MetaCtx.lctx/check`）。
    scope: RefCell<pattern::Scope>,
}

struct StoreSlot {
    /// `Some` = 空闲；`None` = 正被外层求解**独占**。
    store: Option<MetaStore>,
    /// 正在跑的**重入**求解层数（每层一个 id 块）。
    depth: u32,
}

impl DeclMVarStore {
    pub(crate) fn new() -> Self {
        Self {
            slot: RefCell::new(StoreSlot {
                store: Some(MetaStore::default()),
                depth: 0,
            }),
            log: RefCell::new(Vec::new()),
            scope: RefCell::new(Default::default()),
        }
    }

    /// **取走** store（或用 scratch）—— 返回的 guard 是唯一能碰 store 的东西。
    pub(crate) fn park(&self) -> StoreGuard<'_> {
        let owned = self.slot.borrow_mut().store.take();
        match owned {
            Some(store) => StoreGuard {
                slot: Some(&self.slot),
                log: Some(&self.log),
                scope: Some(&self.scope),
                owns_slot: true,
                mark: store.mark(),
                commit: false,
                store: Some(store),
            },
            None => {
                let depth = {
                    let mut slot = self.slot.borrow_mut();
                    slot.depth += 1;
                    slot.depth
                };
                debug_assert!(
                    u64::from(depth) * u64::from(SCRATCH_BLOCK) < u64::from(DECL_META_ID_BASE),
                    "scratch 层数超出 id 空间（见 DeclMVarStore 不变量 3）"
                );
                let base = DECL_META_ID_BASE.saturating_sub(depth.saturating_mul(SCRATCH_BLOCK));
                let store = MetaStore::with_base(base);
                StoreGuard {
                    slot: Some(&self.slot),
                    log: None,
                    scope: None,
                    owns_slot: false,
                    mark: store.mark(),
                    commit: false,
                    store: Some(store),
                }
            }
        }
    }

    /// **P1 判据**（单测）：store 里攒了多少个元变量。
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.slot.borrow().store.as_ref().map_or(0, MetaStore::len)
    }

    /// **P1 判据**（单测）：已分配的**绝对 id**（保序）⇒ 第二次求解必须**接着**第一次的往后长。
    #[cfg(test)]
    pub(crate) fn ids(&self) -> Vec<u32> {
        let slot = self.slot.borrow();
        let Some(store) = slot.store.as_ref() else {
            return Vec::new();
        };
        (0..store.mvars.len())
            .map(|i| store.base + i as u32)
            .collect()
    }

    /// **B2-P2/P3a**：只读按**绝对 id** 查 `(ty, kind)` —— 读**类型日志**、**不读槽** ✓
    /// （停车期间也可查；scratch/s4 或已回滚的 id ⇒ `None`；闸门关 ⇒ 调用方回落 ✓）。
    pub(crate) fn lookup(&self, id: u32) -> Option<(Expr, MetaKind)> {
        self.log.borrow().get(id as usize).cloned()
    }
}

/// [`DeclMVarStore`] 换出期间的独占 guard（见它的不变量 1/2/4）。
pub(crate) struct StoreGuard<'a> {
    slot: Option<&'a RefCell<StoreSlot>>,
    /// **B2-P3a**：类型日志（`None` = scratch / detached ⇒ 不记账 ✓）。
    log: Option<&'a RefCell<Vec<(Expr, MetaKind)>>>,
    /// **HO3a/HO3b**：上下文快照（`None` = scratch / detached ⇒ `MetaCtx.lctx` 空 ⇒ pattern 弃权）。
    scope: Option<&'a RefCell<pattern::Scope>>,
    /// `true` = 手里是**声明级** store（`Drop` 放回槽）；`false` = 重入的 scratch（`Drop` 减层数）。
    owns_slot: bool,
    mark: StoreMark,
    /// `true` = 本次求解成功 ⇒ 不回滚。
    commit: bool,
    store: Option<MetaStore>,
}

impl<'a> StoreGuard<'a> {
    /// **闸门关 / 非声明通道**：本轮私有的 store（= 今天 `MetaStore::default()` 逐字节相同 ✓）。
    pub(crate) fn detached() -> Self {
        let store = MetaStore::default();
        StoreGuard {
            slot: None,
            log: None,
            scope: None,
            owns_slot: false,
            mark: store.mark(),
            commit: false,
            store: Some(store),
        }
    }

    /// **B2-P3b**：`true` = 手里是**声明级** store（⇒ 部分出口允许未解）；`false` = detached/scratch。
    pub(crate) fn is_shared(&self) -> bool {
        self.owns_slot
    }

    pub(crate) fn store_mut(&mut self) -> &mut MetaStore {
        self.store
            .as_mut()
            .expect("StoreGuard 在存续期间恒持有 store")
    }

    /// **B2-P3a**：本 guard 的 [`MetaCtx`]（自动接上类型日志 ⇒ 生产调用点只改这一行 ✓）。
    pub(crate) fn meta_ctx<'b>(&'b mut self, unfold: &'b dyn Fn(&Expr) -> Expr) -> MetaCtx<'b> {
        let (log, scope) = (self.log, self.scope);
        let mut ctx = MetaCtx::new(unfold, self.store_mut());
        ctx.set_type_log(log);
        // **HO3a/HO3b**：上下文快照（`lctx` + checker）⇒ 见 `pattern::attach_scope`。
        pattern::attach_scope(&mut ctx, scope);
        ctx
    }

    /// **求解成功**：把本次造的元变量/赋值留在 store 里（失败路径**不调**）。
    pub(crate) fn commit(&mut self) {
        self.commit = true;
    }
}

impl Drop for StoreGuard<'_> {
    fn drop(&mut self) {
        // ① **失败不落盘**（R3 的边界：不把失败的探针留给后面的求解）。
        if !self.commit {
            if let Some(store) = self.store.as_mut() {
                store.rollback_to(self.mark);
            }
            // **B2-P3a**：类型日志与 `mvars` **同生共死**（失败求解不许留下可查的 id ✓）。
            if let Some(log) = self.log.filter(|_| self.owns_slot) {
                log.borrow_mut().truncate(self.mark.mvars);
            }
        }
        // ② 放回 / 减层数（**panic 展开也走这里** ⇒ store 不会丢）。
        let Some(slot) = self.slot else { return };
        let mut slot = slot.borrow_mut();
        if self.owns_slot {
            slot.store = self.store.take();
        } else {
            slot.depth = slot.depth.saturating_sub(1);
        }
    }
}

/// **B2-P1**：把「这条声明有没有声明级 store」变成一次求解的 [`StoreGuard`]。
/// `None` ⇒ [`StoreGuard::detached`]（逐字节等于今天 ✓）。
pub(crate) fn park_decl_store(decl: Option<&DeclMVarStore>) -> StoreGuard<'_> {
    match decl {
        Some(handle) => handle.park(),
        None => StoreGuard::detached(),
    }
}

// ─────────────────── B2-P2（2026-10-25）：内核载体 `Expr::Meta` ───────────────────
//
// **载体**：`\0soko_m{id}`（源级）⇒ [`sokonanoda::expr::Expr::Meta`]（核侧，K1 ✓），
// 类型/种类都取**声明级 store**（P1）里那一条 ✓。**elab 侧只有一处**接线
// （`elab.rs` 的 `Expr::Ident` 臂）⇒ 这里只放**三闸合一**与守卫。

/// **B2-P2**：carrier 的**三道闸**合成**一次**判断 ——
/// `Some((id, guard, ty, kind))` ⇒ 可以造 `Expr::Meta`；`None` ⇒ 调用方走**今天**的
/// unknown-identifier 路（**绝不**发明类型 ✗）。
///
/// 三道闸（缺一条都 `None`）：
/// 1. `ctx` 可用（非声明通道传 `None`）；
/// 2. store **现在**可读（闸门开 **且** 没被外层求解 park 走 —— 嵌套求解看不见外层 ✓）；
/// 3. `id` 在 store 里、且**不在**正在展开的集合里（自引用 / 环 ⇒ 拒 ✓）。
///
/// ⚠ 返回的 [`MetaExpandGuard`] **必须活到 `ty` elaborate 完**（RAII 弹栈 ⇒ 环路守卫
/// 只在这一次展开期间生效 ✓）。
pub(crate) fn decl_meta_carrier<'a>(
    ctx: Option<&'a DeclMetaCtx>,
    name: &str,
) -> Option<(u32, MetaExpandGuard<'a>, Expr, MetaKind)> {
    let decl = ctx?;
    let id = decl_meta_id_of(name)?;
    let (ty, kind) = decl.lookup_meta(id)?;
    let guard = decl.enter_meta_expansion(id)?;
    Some((id, guard, ty, kind))
}

/// [`DeclMetaCtx::enter_meta_expansion`] 的 RAII 守卫（`Drop` 弹栈 ✓；出错 / panic 展开都走 ✓）。
pub(crate) struct MetaExpandGuard<'a> {
    decl: &'a DeclMetaCtx,
    id: u32,
}

impl Drop for MetaExpandGuard<'_> {
    fn drop(&mut self) {
        let mut expanding = self.decl.expanding.borrow_mut();
        if let Some(pos) = expanding.iter().rposition(|x| *x == self.id) {
            expanding.remove(pos);
        }
    }
}

/// **宇宙约束**（U1 第 2 片 ✓；§2.9：「`ULe`/`UEq`/`ULub` **三种够用**」✓）。
///
/// **Lean 4 对照** ✓（本机源码 HEAD `d0493e4c1e` ✓）：Lean 的约束是
/// `LevelDefEq` 里的 `ULevel` 问题（`ULe`/`UEq`/`ULub` 一族 ✓），
/// **刚性约束推迟**到项元变量被赋值（`postponeIsLevelDefEq` ✓）。
///
/// ⚠ **id 都是层元变量的 id** ✓（`level_name(id)` 的那个 id ✓）⇒ 与 `Level` 表示解耦 ✓。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[allow(dead_code)] // U1 第 3/4 片（合一 + 惰性检查）会用
pub(crate) enum UnivConstraint {
    /// `lhs ≤ rhs`。
    Le { lhs: u32, rhs: u32 },
    /// `lhs = rhs`。
    Eq { lhs: u32, rhs: u32 },
    /// `out = max(a, b)`。
    Lub { out: u32, a: u32, b: u32 },
}

/// **B2-P3a**：一个 pass 建前导 `k` 个元变量（`ty_i` 先代换更早的前缀名 → 载体）⇒ `ty` 里没有裸 `\0soko_p*`/`\0soko_mp*` ✓（调用方已保证前 `k` 位都有名字 ✓）。
pub(crate) fn fresh_telescope_metas(
    meta: &mut MetaCtx<'_>,
    tel: &[(&str, &Expr)],
    k: usize,
) -> (Vec<MetaId>, HashMap<String, Expr>) {
    let (mut ids, mut sigma) = (Vec::with_capacity(k), HashMap::new());
    for i in 0..k {
        let ty = crate::spine::substitute(tel[i].1, &sigma);
        let mut later: Vec<String> = tel[i + 1..].iter().map(|x| x.0.to_string()).collect();
        later.retain(|n| !n.is_empty());
        ids.push(meta.fresh(ty, MetaKind::Natural, later));
        sigma.insert(tel[i].0.to_string(), meta.meta_expr(ids[i]));
    }
    (ids, sigma)
}

pub(crate) struct MetaCtx<'a> {
    /// **数据借自 `MetaStore`** ✓（B2：让它可以跨求解活着 ✓ —— 今天调用方一次求解一个 store ✓，
    /// B2 会把它挂到声明级 ✓）。
    store: &'a mut MetaStore,
    fuel: u32,
    depth: u32,
    /// **本次求解的递归深度上限** ✓（撞了就**加倍**，见 `unify_impl` ✓）。
    /// 它是**可变**的 ✗→✓（先前直接用常量 ⇒ 撞上就只能判否 ✗）；
    /// 初值取自登记表 `sokonanoda::gates::limits::max_rec_depth()` ✓。
    max_depth: u32,
    /// 已经「加大预算重试」过几次 ✓（上界 `limits::max_escalations()` ✓）。
    escalations: u32,
    /// 本次求解**第一个**硬错误（M3 的通道归因；`unify` 把它折成 `Tri::No`，但通道要留住）
    first_err: Option<MetaErr>,
    /// **模板/实参两侧的 delta 展开兜底**（M0 的 S13 硬约束：不接它就会把
    /// 「两条约束 defeq 一致」的形状误判成刚性冲突 ✗）
    unfold: &'a dyn Fn(&Expr) -> Expr,
    /// **B2-P3a**：停车可见的 `(ty, kind)` 日志（`None` = 不记账；`meta_ctx`/`set_type_log` 接上）。
    log: Option<&'a RefCell<Vec<(Expr, MetaKind)>>>,
    /// **HO3a**：本次求解的局部上下文（空 = detached/scratch ⇒ pattern 规则弃权）。
    lctx: pattern::Lctx,
    /// **HO3a/HO3b**：抽象 λ 的**完整类型检查**句柄（`None` ⇒ pattern 规则弃权）。
    check: Option<pattern::CheckFn>,
}

impl<'a> MetaCtx<'a> {
    pub(crate) fn new(unfold: &'a dyn Fn(&Expr) -> Expr, store: &'a mut MetaStore) -> Self {
        MetaCtx {
            store,
            fuel: sokonanoda::gates::limits::max_heartbeats(),
            depth: 0,
            max_depth: sokonanoda::gates::limits::max_rec_depth(),
            escalations: 0,
            first_err: None,
            unfold,
            log: None,
            lctx: Default::default(),
            check: None,
        }
    }

    /// **B2-P3a**：接上类型日志（`new` 的测试调用方**零改动** ✓ —— 需要停车的可查性才接）。
    pub(crate) fn set_type_log(&mut self, log: Option<&'a RefCell<Vec<(Expr, MetaKind)>>>) {
        self.log = log;
    }

    /// `MetaId` → `store.mvars` 的**下标**（`MetaId` 是绝对 id：`base + 下标`）。
    fn idx(&self, m: MetaId) -> usize {
        (m.0 - self.store.base) as usize
    }

    /// 建一个元变量。`out_of_scope` = 该位**更晚**的望远镜参数名（作用域检查用）。
    pub(crate) fn fresh(&mut self, ty: Expr, kind: MetaKind, out_of_scope: Vec<String>) -> MetaId {
        // **B2-P1**：`MetaId` = `base + 下标`（`base==0` ⇒ 与今天逐字节相同 ✓）。
        let id = MetaId(self.store.base + self.store.mvars.len() as u32);
        // **B2-P3a**：先记日志（**绝对 id 索引**；日志只属于主 store ⇒ base 0 ✓）。
        if let Some(log) = self.log {
            log.borrow_mut().push((ty.clone(), kind));
        }
        self.store.mvars.push(MVar {
            lctx: self.lctx.clone(),
            ty,
            value: None,
            kind,
            out_of_scope,
        });
        id
    }

    /// 元变量在表达式里的形态（`\0soko_m{id}` 这个 `Ident`）。
    pub(crate) fn meta_expr(&self, m: MetaId) -> Expr {
        Expr::Ident {
            name: meta_name(m.0),
            span: Span::default(),
        }
    }

    #[allow(dead_code)] // M2 的调用方（`implicit::solve_prefix` 的接线）会用
    pub(crate) fn value(&self, m: MetaId) -> Option<Expr> {
        self.store.mvars[self.idx(m)].value.clone()
    }

    /// **出口 zonk**：把已赋值的元变量代进 `e`（链式赋值有界迭代）。
    pub(crate) fn zonk(&self, e: &Expr) -> Expr {
        let mut sigma: HashMap<String, Expr> = HashMap::new();
        for (i, m) in self.store.mvars.iter().enumerate() {
            if let Some(v) = &m.value {
                sigma.insert(meta_name(self.store.base + i as u32), v.clone());
            }
        }
        if sigma.is_empty() {
            return e.clone();
        }
        let mut cur = e.clone();
        for _ in 0..MAX_ZONK_ROUNDS {
            let next = crate::spine::substitute(&cur, &sigma);
            if crate::spine::same_shape(&next, &cur) {
                break;
            }
            cur = next;
        }
        cur
    }

    /// 赋值（**occurs + type-occurs + 作用域**三道闸门；M3 起再加 sort/kind 检查）。
    pub(crate) fn assign(&mut self, m: MetaId, v: Expr) -> Result<(), MetaErr> {
        let v = self.zonk(&v);
        if crate::spine::mentions(&meta_name(m.0), &v) {
            return Err(MetaErr::Occurs);
        }
        // type-occurs：解里提到的**未解元变量**，其声明类型里不许出现 `m`（防经类型的环）
        for other in meta_ids_in(&v) {
            if other == m {
                continue;
            }
            if crate::spine::mentions(&meta_name(m.0), &self.store.mvars[self.idx(other)].ty) {
                return Err(MetaErr::TypeOccurs);
            }
        }
        for name in &self.store.mvars[self.idx(m)].out_of_scope {
            if !name.is_empty() && crate::spine::mentions(name, &v) {
                return Err(MetaErr::Scope);
            }
        }
        // **sort/kind 检查**（M3，设计 §2.5）：`?m : T` 要求 `v : T` **恰好**成立
        // （本内核**非累积**）。三值语法近似：**只拒"确定错"的**，`None` = 不知道 ⇒ 放行 ✓。
        if let (Some(k), Some(n)) = (
            sort_of_value(&v),
            sort_of_type(&self.store.mvars[self.idx(m)].ty),
        ) {
            if k != n {
                return Err(MetaErr::Kind);
            }
        }
        let idx = self.idx(m);
        self.store.mvars[idx].value = Some(v);
        Ok(())
    }

    /// 合一一步：`Yes` 成立 / `No` 不成立 / `Undef` **弃权**（已压进待定队列）。
    pub(crate) fn unify(&mut self, l: &Expr, r: &Expr) -> Tri {
        self.depth = 0;
        // **C 子项（2026-10-12）**：`Eq.{u} …`（模板，含**层元变量**）与 `Eq …`
        // （裸 `Ident`）是**同一个头**，但结构分解对不上 ⇒ 判成刚性冲突 ✗
        // （实测 `[clash] 左=Eq.{u} ｜ 右=Eq`）。
        //
        // 出处：全显式 pp 打的是 `@Eq α a x` —— **没有** `.{…}` 层级标记
        // （层级在 pp 里是隐式的）⇒ 回读成裸 `Ident`；而签名模板的层是
        // `Eq.{u}`（`u` 是层元变量）。两者语义相同：`@Eq` 在 Lean 里就是
        // 「宇宙也交给元变量」（`mkFreshLevelMVars` ✓）。
        //
        // **只在这一对形状上归一**（一侧 `UniverseApp`、另一侧**同名** `Ident`）
        // ⇒ 其它任何形状一步都不进 ✓（既有判定面零变化 ✓）；层本身仍由出口的
        // `levelMVarToParam` / 后续约束处理 ✓（这里**不**丢信息：裸 `Ident` 那侧
        // 本来就没有层级可言 ✓）。
        self.unify_impl(l, r, true)
    }

    /// **E1 路线 ⑤ · 类型约束**（2026-10-18 用户硬令 ②）：`?m := v` 隐含的那条方程
    /// `T(?m) ≟ typeof(v)` —— 逐位补上。
    ///
    /// **Lean 4 对照**（本机 `~/Documents/lean/lean4` HEAD `d0493e4c1e` ✓）：
    /// `Meta/ExprDefEq.lean` 的 `processAssignment` 在做完 `?m := fun a… => t` 的抽象后
    /// **一定**走 `checkTypesAndAssign`（:1186/:1220）—— 赋值**不是**只写进去，还要
    /// 把值的类型与元变量的声明类型合一 ✓。这正是 `apply Eq.subst _ h` 里
    /// `?p := A` 顺带把 `?α := α` 定下来的那一步 ✓（`Elab/App.lean:1176 elabAppArgs`
    /// 给每个隐式位建元变量、`propagateExpectedType` 再用期望类型收口 ✓）。
    ///
    /// ⚠ **只在判 `Yes` 时生效**：`No`/`Undef` ⇒ `first_err` 与**元变量赋值全部回滚**
    /// ✗ —— 我们的 oracle 是**源级近似**（`operand_type_expr`，不是内核）⇒
    /// **绝不用它造新的拒绝面** ✓（对齐 Lean 的 `checkpointDefEq`：失败即回滚 ✓）。
    ///
    /// ⚠ **只在「还有未解位」时由调用方进入** ✓（成功路径一次都不跑 ✓）。
    pub(crate) fn refine_by_types(
        &mut self,
        ids: &[MetaId],
        layers: &[crate::compile::implicit::Layer],
        sigma: &HashMap<String, Expr>,
        ty_of: &dyn Fn(&Expr) -> Option<Expr>,
    ) {
        for (i, id) in ids.iter().enumerate() {
            let Some(v) = self.value(*id) else { continue };
            let v = self.zonk(&v);
            let Some(vty) = ty_of(&v) else { continue };
            let ty = crate::spine::substitute(&layers[i].domain, sigma);
            let saved_err = self.first_err.clone();
            let snapshot: Vec<Option<Expr>> =
                self.store.mvars.iter().map(|m| m.value.clone()).collect();
            if self.unify_impl(&ty, &vty, true) != Tri::Yes {
                self.first_err = saved_err;
                for (m, v) in self.store.mvars.iter_mut().zip(snapshot) {
                    m.value = v;
                }
            }
        }
    }

    fn unify_impl(&mut self, l: &Expr, r: &Expr, allow_unfold: bool) -> Tri {
        // **C 子项（2026-10-12）**：`Eq.{u} …`（模板，含**层元变量**）与 `Eq …`
        // （裸 `Ident`）是**同一个头**，但 `App` 分解会把这一对顶到刚性冲突 ✗
        // （实测 `[clash] 左=Eq.{u} ｜ 右=Eq` —— 归一必须挂在**递归入口**上 ✓，
        // 只挂 `unify` 顶层会被 `fun` 字段的递归绕过 ✗）。
        //
        // 出处：全显式 pp 打的是 `@Eq α a x` —— **没有** `.{…}` 层级标记
        // （层级在 pp 里是隐式的）⇒ 回读成裸 `Ident`；而签名模板的层是
        // `Eq.{u}`（`u` 是层元变量）。两者语义相同：`@Eq` 在 Lean 里就是
        // 「宇宙也交给元变量」（`mkFreshLevelMVars` ✓）。
        //
        // **只在这一对形状上归一**（一侧 `UniverseApp`、另一侧**同名** `Ident`）
        // ⇒ 其它任何形状一步都不进 ✓（既有判定面零变化 ✓）；层本身仍由出口的
        // `levelMVarToParam` / 后续约束处理 ✓（这里**不**丢信息：裸 `Ident` 那侧
        // 本来就没有层级可言 ✓）。
        if let (Expr::UniverseApp { name: a, .. }, Expr::Ident { name: b, .. }) = (l, r) {
            if a == b {
                return self.unify_impl(&bare_head(l), r, allow_unfold);
            }
        }
        if let (Expr::Ident { name: a, .. }, Expr::UniverseApp { name: b, .. }) = (l, r) {
            if a == b {
                return self.unify_impl(l, &bare_head(r), allow_unfold);
            }
        }
        if self.fuel == 0 || self.depth >= self.max_depth {
            // ── **G-88 真修（2026-10-04 ✓）：撞预算 = 加大预算继续算** ─────────────
            // 先前这里是 `return Tri::No;`（原话「预算耗尽 ⇒ 按**无解**处理」✗）——
            // 那是**唯一一处「判不了 ⇒ 当成否」**✗：学习者写长一点的证明，系统说
            // 「解不出来」，而**那不是真的无解** ✗（值守 13:12「这种闸我都不能接受」✓）。
            // 总规矩：**预算耗尽只允许变慢，绝不允许变错** ✓。这里逐字实现它：
            // * 还有升级额度 ⇒ **fuel / max_depth 各翻倍，继续算** ✓（= 变慢 ✓，
            //   答案**不变** ✓ —— 预算够的那次会给出同一个结论 ✓）；
            // * 升级到底还撞 ⇒ **弃权** `Tri::Undef`（= 「判不了 ⇒ 走慢路」✓），
            //   **绝不再返回 `Tri::No`** ✗（= 「判不了 ⇒ 当成否」✗）。
            // 上界是**常数倍**（`SOKO_LIMIT_MAX_ESCALATIONS` ✓）⇒ 不挂死 ✓。
            // **计数出口**（G-91 ✓）：升级与最终弃权**各一个** ✓（整本课程实测
            // **两个都是 0** ✓ ⇒ 这条改动在语料上**零行为变化** ✓，判据见
            // `crates/front/tests/gate_census.rs` ✓）。
            if self.escalations < sokonanoda::gates::limits::max_escalations() {
                self.escalations += 1;
                // ⚠ 升级基准取**默认值**（不是"当前值"✗）：这样拧到 1 时，
                // 升满升级额度之后预算**一定 ≥ 默认** ✓
                // ⇒ 默认预算能判的，拧到 1 也一定判得出来 ✓（只是慢 ✓）。
                self.fuel = self
                    .fuel
                    .max(sokonanoda::gates::limits::max_heartbeats())
                    .saturating_mul(2);
                self.max_depth = self
                    .max_depth
                    .max(sokonanoda::gates::limits::max_rec_depth())
                    .saturating_mul(2);
                sokonanoda::gates::META_BUDGET_ESCALATED.bump();
            } else {
                sokonanoda::gates::META_BUDGET_EXHAUSTED.bump();
                return Tri::Undef;
            }
        }
        self.fuel -= 1;
        self.depth += 1;
        let out = self.unify_body(l, r, allow_unfold);
        self.depth -= 1;
        out
    }

    /// **一次合一的入口**（2026-10-15）：见 [`Self::unify_body_inner`]。
    ///
    /// ⚠ **返回 `Yes` ⇒ 回滚本次调用内部的"探索期"硬错误** ✗→✓。
    ///
    /// `unify_body_inner` 的路线是**探索式**的（结构分解 → delta 兜底展开 → …）：
    /// 一条**走不通**的路线会在 `first_err` 上留下 `Clash`，而它是**粘性**的 ✗ ⇒
    /// 后面哪怕另一条路线把这一对**判成了 `Yes`**，出口 `discharge` 的第二道闸
    /// （`if self.first_err.is_some() { return self.channel(); }`）照样把它归因成
    /// `Clash` ✗ —— **一对已经合一的项被判成"两条线索互相矛盾"**。
    ///
    /// 实测（决定性 · 2026-10-15 `Eq.subst` 的谓词位）：④ `?p ?b ≟ Set.mem x A`
    /// 先走结构分解 ⇒ `A ≟ Set.mem x` 判 No（**留下 `Clash`** ✗），紧接着 delta
    /// 兜底把 `Set.mem x A` 展成 `A x` ⇒ `A x ≟ A x` **判 `Yes`** ✓ —— 而出口照样
    /// 报 `Clash` ✗。这就是 `cmp2`（`intro` 之后的目标来自 `Set.subset` 的**定义体**、
    /// 是点名应用链 `Set.mem x A`）失败、而 `cmp1`（签名里的期望类型是**记法节点**
    /// `x ∈ A`，走不到结构分解 ⇒ 没有那次假冲突）成功的**唯一**差别。
    ///
    /// 判据：**`Yes` = 这一对已经成立** ⇒ 本次调用内部任何一条走不通的路线留下的
    /// 错误都**不是**这个求解的错误 ✓；进入本调用**之前**记下的错误**原样保留** ✓
    /// （别的约束真错了仍然照报 ✓）。与设计 §1.2 第 7 行 Lean 的 `checkpointDefEq`
    /// 「失败回滚状态」同源，方向相反：**成功即回滚**。
    fn unify_body(&mut self, l: &Expr, r: &Expr, allow_unfold: bool) -> Tri {
        let saved_err = self.first_err.clone();
        let out = self.unify_body_inner(l, r, allow_unfold);
        if out == Tri::Yes {
            self.first_err = saved_err;
        }
        out
    }

    fn unify_body_inner(&mut self, l: &Expr, r: &Expr, allow_unfold: bool) -> Tri {
        let l = self.zonk(l);
        let r = self.zonk(r);
        // **G-63 ④ 的修（2026-10-03 ✓）**：源码里的**记法节点**（`a = b` ⇒
        // `Expr::Notation { target: "Eq", lhs, rhs }` ✓）与 elaborate 后的
        // `Eq.{u} α a b` **结构不同** ✗ ⇒ 判成刚性冲突 ✗（决定性 trace 实测：本层**见到**了记法对 ✓
        // 但旧规则判 `false` ✗ —— 因为只比**一层** ✗：模板的尾部实参是 `Quot.mk α (qr α) a` ✓
        // 而源码操作数是 `Quot.mk (qr α) a` ✗，差在**内层的隐式 `α`** ✓）。
        // ⇒ 规则必须是**递归 + 按尾部对齐** ✓（允许 elaborate 侧多出前导隐式 ✓）。
        if notation_matches_app(&l, &r) || notation_matches_app(&r, &l) {
            return Tri::Yes;
        }
        // 快路径：结构相同（忽略 span）⇒ 立刻成立（顺序纪律：便宜检查在前）
        if crate::spine::same_shape(&l, &r) {
            return Tri::Yes;
        }
        if let Some(m) = meta_of(&l) {
            return self.tri_assign(m, r);
        }
        if let Some(m) = meta_of(&r) {
            return self.tri_assign(m, l);
        }
        // **HO3a/HO3b**：pattern 赋值（`?m a₁…aₙ ≟ t` ⇒ `fun a₁…aₙ => t`；近似档见 `approx`）。**只在成功时**
        // 返回 `Some(Yes)` ⇒ 弃权 ⇒ 下面今天的结构分解/delta/postponed **逐字节照旧**。
        if let Some(t) = self.pattern_try(&l, &r) {
            return t;
        }
        // **C 子项（2026-10-12）：实参位要「按尾部对齐」** ——
        // 一侧可能多出**前导隐式实参**（elaborate 加上的 ✓），逐位对齐会把
        // `Eq.{u} α ?m2 ?m3` 与 `Eq x a` 摆成 `α ≟ x` ⇒ 假冲突 ✗
        // （实测：`Eq.subst h h` 的路线②，`[clash] 左=Eq.{u} α ｜ 右=Eq`）。
        //
        // 出处（`crate::spine::unify_spine` 的同一条注释 ✓）：内核 pp 会丢
        // **前导**隐式实参（`Eq.{1} (Set α) A B` 打成 `Eq A B` ✓）⇒ 被丢掉的
        // 永远是**前面**那些 ⇒ 只有**尾部对齐**才是有意义的对应 ✓。
        //
        // 做法：把**长**的那一侧的**前导**实参丢掉，重建成同头、同实参个数的
        // 应用链，再走原样的逐位分解 ✓。⚠ 只在**同头 + 实参个数不同**时进
        //（同头判据复用 `spine::same_head`：只比名字、不比层级 ✓）⇒ 其余形状
        // **逐字节不变** ✓。
        if let Some((ln, rn)) = tail_aligned_pair(&l, &r) {
            return self.unify_impl(&ln, &rn, allow_unfold);
        }
        // 结构分解（**两侧**；今天只做单侧匹配）。
        //
        // ⚠ **结构分解判否不许"提前收摊"**（2026-10-15 · `Eq.subst` 的谓词位）：
        // 两侧**形状相同**、但一侧还**没 delta 归约**时（`A x` ≟ `Set.mem x A` ——
        // `Set.mem a A` 的定义体就是 `A a` ✓），旧写法 `if h != Tri::Yes { return h; }`
        // 直接把 `No` 返回 ⇒ 下面那条 **S13 的 delta 展开兜底一步都进不去** ✗
        // ⇒ 同一个判断「期望类型是**记法节点**」（走不到这条分支 ⇒ 兜底接管 ⇒ Yes ✓）
        // 与「期望类型是**点名应用链**」（走到这条分支 ⇒ 当场 No ✗）**判得不一样** ✗。
        //
        // 实测（`Eq.subst (Eq.symm hx) h` 的 ④：`?p ?b ≟ 期望`）：`intro` 之后的目标
        // 来自 `Set.subset` 的定义体（点名形态 `Set.mem x A`）⇒ 结构比较 `A ≟ Set.mem x`
        // 判 No ⇒ 报 `elab-implicit-argument-unsolved`（用户 2026-10-15 的
        // `subset_of_mem_singleton`）；而签名里写的 `x ∈ A` 是**记法节点** ⇒ 不匹配
        // App/App 臂 ⇒ 落到兜底 ⇒ `unfold` 成 `A x` ⇒ Yes ✓（`cmp1` 过、`cmp2` 不过的
        // **唯一**差别就是这一条）。
        //
        // ⇒ 改成：结构判 **`Yes` 立刻返回** ✓；判 `No`/`Undef` 就**记下来**，让兜底先试
        // （兜底**只认 `Yes`** ✓ ⇒ 它只会把「因为没归约而判错」的 No 翻成 Yes，
        // 不会把 No 洗成别的结论 ✗）；兜底没拿到 `Yes` ⇒ **原样返回**结构那次的结论 ✓
        // （`Undef` 那条「子对已压进待定队列」的语义逐字不变 ✓）。
        let mut structural: Option<Tri> = None;
        match (&l, &r) {
            (
                Expr::App {
                    fun: f1, arg: a1, ..
                },
                Expr::App {
                    fun: f2, arg: a2, ..
                },
            ) => {
                let h = self.unify_impl(f1, f2, allow_unfold);
                // **C 子项（2026-10-12）：实参位要「按尾部对齐」** ——
                structural = Some(if h == Tri::Yes {
                    self.unify_impl(a1, a2, allow_unfold)
                } else {
                    h
                });
            }
            (
                Expr::Arrow {
                    domain: d1,
                    codomain: c1,
                    ..
                },
                Expr::Arrow {
                    domain: d2,
                    codomain: c2,
                    ..
                },
            ) => {
                let d = self.unify_impl(d1, d2, allow_unfold);
                structural = Some(if d == Tri::Yes {
                    self.unify_impl(c1, c2, allow_unfold)
                } else {
                    d
                });
            }
            // 洞是通配（既有语义：洞不约束）
            (Expr::Hole { .. }, _) | (_, Expr::Hole { .. }) => return Tri::Yes,
            _ => {}
        }
        if structural == Some(Tri::Yes) {
            return Tri::Yes;
        }
        // delta 展开兜底（**S13 硬约束**）：任一侧展开后形状变了 ⇒ 再试一次（只展开一层）
        if allow_unfold {
            let ul = (self.unfold)(&l);
            if !crate::spine::same_shape(&ul, &l) {
                let t = self.unify_impl(&ul, &r, false);
                if t == Tri::Yes {
                    return Tri::Yes;
                }
            }
            let ur = (self.unfold)(&r);
            if !crate::spine::same_shape(&ur, &r) {
                let t = self.unify_impl(&l, &ur, false);
                if t == Tri::Yes {
                    return Tri::Yes;
                }
            }
        }
        // **B0（2026-10-23）**：记法节点 ⇄ 目标应用（`x ∈ A` ≟ `Set.mem x A`）。
        // 只在兜底里摊开、且**只认 Yes** ⇒ 今天能过的输入不改结论。
        if allow_unfold {
            let dl = desugar_notation(&l);
            let dr = desugar_notation(&r);
            let changed = !(crate::spine::same_shape(&dl, &l) && crate::spine::same_shape(&dr, &r));
            if changed && self.unify_impl(&dl, &dr, false) == Tri::Yes {
                return Tri::Yes;
            }
        }
        // 兜底没拿到 `Yes` ⇒ **结构分解那次的结论说了算**（见上面那条注释）：
        // `Undef` 原样返回（子对已经压进待定队列 ✓）、`No` 原样返回（照旧归因 Clash ✓）。
        if let Some(t) = structural {
            return t;
        }
        // 刚性冲突 vs 弃权：还有**未解**元变量 ⇒ 弃权（压进待定队列，等后续赋值）
        if has_unassigned_meta(&l) || has_unassigned_meta(&r) {
            self.store.postponed.push((l, r));
            return Tri::Undef;
        }
        // 刚性冲突：**归因**（M3 的三通道之一），调用方据此换一句 message（码不变）
        if self.first_err.is_none() {
            self.first_err = Some(MetaErr::Clash);
        }
        // **G-63 归因**：这里若出现裸模板名（`\0soko_p*`）⇒ 上游 `solve_prefix_meta` 的
        // 模板代换/层域有漏（不是本处判错）。`SOKO_CLASH_TRACE=1` 打两侧落地项（纯诊断）。
        if std::env::var("SOKO_CLASH_TRACE").is_ok() {
            // **渲染会折叠记法**（`Eq.{u} …` 显示成 `= …` ✓）⇒ 光看渲染分不出"真不同"还是"只差写法" ✗
            // ⇒ 同时打**结构 Debug**（G-63 ④ 的钉死就靠它 ✓）。
            eprintln!(
                "[clash] 左={} ｜ 右={}\n[clash-raw] 左={l:?}\n[clash-raw] 右={r:?}",
                crate::proof::render_expr(&l),
                crate::proof::render_expr(&r)
            );
        }
        Tri::No
    }

    /// 跑到不动点：`Yes` 队列空 / `No` 有硬冲突 / `Undef` 还剩待定。
    /// **停条件 = 待定计数不再严格下降**（与 Lean `processPostponed` 同款）。
    pub(crate) fn unify_all(&mut self) -> Tri {
        for _ in 0..MAX_ROUNDS {
            if self.store.postponed.is_empty() {
                return Tri::Yes;
            }
            let before = self.store.postponed.len();
            let batch = std::mem::take(&mut self.store.postponed);
            for (l, r) in batch {
                // 仍然卡住的叶子会**自己重新压回** `postponed`
                if self.unify(&l, &r) == Tri::No {
                    return Tri::No;
                }
            }
            if self.store.postponed.is_empty() {
                return Tri::Yes;
            }
            if self.store.postponed.len() >= before {
                // **弃权**（正当 ✓：判不了 ⇒ 走慢路 ✓）—— 但仍要有出口 ✗（G-91 ✓）。
                sokonanoda::gates::UNIFY_NO_PROGRESS.bump();
                return Tri::Undef; // 没有净进展 ⇒ 停（不是无界工作队列 ✓）
            }
        }
        // 轮数用光 ⇒ 同样是**弃权** ✓（不是"判否"✗）—— 出口同上 ✓。
        sokonanoda::gates::UNIFY_ROUNDS_EXHAUSTED.bump();
        Tri::Undef
    }

    /// **E19 选择规则的显式化**（设计 §2.7）：未解的位借**声明类型同形**的**已解**兄弟的值
    /// （按 index 升序取第一个）。**这不是推理、是选择** —— 与 `fill_pending_by_shape` 同一判据。
    pub(crate) fn default_unresolved(&mut self) -> Result<(), MetaErr> {
        for i in 0..self.store.mvars.len() {
            // **B2-P4a**：`SyntheticOpaque` = **保留种类**（洞的语义；内核也拒赋值，`mvar.rs:183`）⇒ **不替它选值**。
            let m = &self.store.mvars[i];
            if m.value.is_some() || m.kind == MetaKind::SyntheticOpaque {
                continue;
            }
            let ti = self.zonk(&self.store.mvars[i].ty.clone());
            let mut picked: Option<Expr> = None;
            for j in 0..self.store.mvars.len() {
                if i == j {
                    continue;
                }
                let Some(v) = self.store.mvars[j].value.clone() else {
                    continue;
                };
                let tj = self.zonk(&self.store.mvars[j].ty.clone());
                if crate::compile::pp_shape::borrowable(&ti, &tj) {
                    picked = Some(v);
                    break;
                }
            }
            if let Some(v) = picked {
                self.assign(MetaId(self.store.base + i as u32), v)?;
            }
        }
        Ok(())
    }

    pub(crate) fn unsolved(&self) -> Vec<MetaId> {
        self.store
            .mvars
            .iter()
            .enumerate()
            .filter(|(_, m)| m.value.is_none())
            .map(|(i, _)| MetaId(self.store.base + i as u32))
            .collect()
    }

    fn tri_assign(&mut self, m: MetaId, v: Expr) -> Tri {
        match self.assign(m, v) {
            Ok(()) => Tri::Yes,
            Err(e) => {
                if self.first_err.is_none() {
                    self.first_err = Some(e);
                }
                Tri::No
            }
        }
    }

    /// 本次求解的**通道**（M3）：`Kind` / `Clash` / `Unsolved`（都映射到**既有码**）。
    pub(crate) fn channel(&self) -> MetaSolve {
        match &self.first_err {
            Some(MetaErr::Kind) => MetaSolve::Kind,
            Some(_) => MetaSolve::Clash,
            None => MetaSolve::Unsolved,
        }
    }

    /// **出口**：不动点 + defaulting + 全部解出 + zonk。任一不成立 ⇒ 对应的**通道**
    /// （调用方照旧报既有码，只是 hint 说哪一句不同 ✓）。
    pub(crate) fn discharge(&mut self, ids: &[MetaId]) -> MetaSolve {
        if self.unify_all() == Tri::No {
            return self.channel();
        }
        // 中途有过**硬错误**（kind 不对 / 刚性冲突）⇒ 这次求解已经知道是坏的，直接归因
        // （否则会被"还有未解元变量"盖成 `Unsolved`，通道就丢了 ✗）
        if self.first_err.is_some() {
            return self.channel();
        }
        if let Err(e) = self.default_unresolved() {
            return match e {
                MetaErr::Kind => MetaSolve::Kind,
                _ => MetaSolve::Clash,
            };
        }
        if !self.unsolved().is_empty() {
            return MetaSolve::Unsolved;
        }
        let mut out = Vec::with_capacity(ids.len());
        for id in ids {
            let v = self.zonk(&self.meta_expr(*id));
            // 出口自检：**不许留元变量**（设计红线 1）
            if has_unassigned_meta(&v) {
                return MetaSolve::Unsolved;
            }
            out.push(v);
        }
        MetaSolve::Solved(out)
    }

    /// **IA-4 B2 第 2 步（2026-10-05 ✓）**：出口的**放宽版** ✓ —— **不要求全解出** ✗。
    ///
    /// 与 [`Self::discharge`] 的区别**两处**（都是有意的）✗：① 那道 `unsolved()` 闸**不开** ✓
    /// ② **不 defaulting** ✗ —— B2 的约束可以**跨求解**到达（P4 的声明末 zonk 才是收口点），
    /// 在这里按"同形兄弟"提前猜一个值会把 `Set.image` 的 `β` 猜成 `α` 的值 ⇒ 后面真约束到了
    /// 反倒已经写死（p0 的实测形状）✓。已解的照旧 `zonk` ✓，**未解的连 id 一起返回** ✓。
    ///
    /// ⚠ **红线 1 仍然守** ✓：本方法**只允许在 elaborate 期用** ✗ —— 返回的 `Vec<Expr>` 里
    /// **可能含元变量名** ✓（`\0soko_m{id}` ✓），它们**绝不许进内核** ✗（K1 的
    /// `add_declar` 入口硬拒是兜底 ✓，`builder.rs:399` ✓）。
    ///
    /// ⚠ **硬错误照旧失败** ✓（`kind` 不对 / 刚性冲突 ⇒ 通道归因 ✓，不会因为"允许未解"被盖掉 ✗）。
    pub(crate) fn discharge_keep_unsolved(
        &mut self,
        ids: &[MetaId],
    ) -> Result<(Vec<Expr>, Vec<MetaId>), MetaSolve> {
        if self.unify_all() == Tri::No {
            return Err(self.channel());
        }
        if self.first_err.is_some() {
            return Err(self.channel());
        }
        let mut out = Vec::with_capacity(ids.len());
        let mut still = Vec::new();
        for id in ids {
            let v = self.zonk(&self.meta_expr(*id));
            if has_unassigned_meta(&v) {
                // **未解** ✓ ⇒ **原样返回元变量名** ✓（不进内核 ✗）+ 记下 id ✓。
                still.push(*id);
            }
            out.push(v);
        }
        Ok((out, still))
    }
}

/// `v : Sort n` 的那个 `n`（**三值**：`None` = 不知道 ⇒ 调用方放行）。
///
/// 只认**语法上是 sort** 的形状（设计 §2.5 的表）：`Prop`=1 · `Type`(`Type 0`)=2 ·
/// `Sort(n)`=n+1 · `Sort(Level(_))`/`App`/`Lambda`/`Ident`/其余 = **不知道**。
/// ⚠ 局部变量那一档（查书写类型）留给后续片：**放行**是保守方向（不会假拒绝）✓。
pub(crate) fn sort_of_value(v: &Expr) -> Option<u64> {
    match v {
        Expr::Sort {
            sort: crate::ast::SortKind::Prop,
            ..
        } => Some(1),
        Expr::Sort {
            sort: crate::ast::SortKind::Type,
            ..
        } => Some(2),
        Expr::Sort {
            sort: crate::ast::SortKind::Sort(n),
            ..
        } => Some(n + 1),
        _ => None,
    }
}

/// 声明类型 `t` 作为 sort 的层级：`Prop`=0 · `Type`=1 · `Sort(n)`=n · 其余 = 不知道。
fn sort_of_type(t: &Expr) -> Option<u64> {
    match t {
        Expr::Sort {
            sort: crate::ast::SortKind::Prop,
            ..
        } => Some(0),
        Expr::Sort {
            sort: crate::ast::SortKind::Type,
            ..
        } => Some(1),
        Expr::Sort {
            sort: crate::ast::SortKind::Sort(n),
            ..
        } => Some(*n),
        _ => None,
    }
}

/// 这个表达式**就是**一个元变量吗？
/// **只读**：这个表达式**就是**一个元变量吗（`implicit.rs` 的 C 子项要用它判
/// "模板的头是不是刚性的" ✓ —— 与内部 `meta_of` 同一份判据 ✓）。
pub(crate) fn meta_of_expr(e: &Expr) -> Option<MetaId> {
    meta_of(e)
}

fn meta_of(e: &Expr) -> Option<MetaId> {
    if let Expr::Ident { name, .. } = e {
        if is_meta_name(name) {
            return name[META_PREFIX.len()..].parse::<u32>().ok().map(MetaId);
        }
    }
    None
}

/// 表达式里出现的**未解**元变量（`zonk` 之后剩下的元变量名都是未解的）。
/// 遍历形状与 [`crate::spine::mentions`] 同一份（**漏一个变体就会漏一次检查** ✗）。
fn meta_ids_in(e: &Expr) -> Vec<MetaId> {
    let mut out = Vec::new();
    collect_meta_ids(e, &mut out);
    out
}

fn collect_meta_ids(e: &Expr, out: &mut Vec<MetaId>) {
    if let Some(m) = meta_of(e) {
        out.push(m);
        return;
    }
    match e {
        Expr::Ident { .. }
        | Expr::UniverseApp { .. }
        | Expr::Sort { .. }
        | Expr::Num { .. }
        | Expr::Hole { .. }
        | Expr::By { .. } => {}
        Expr::App { fun, arg, .. } => {
            collect_meta_ids(fun, out);
            collect_meta_ids(arg, out);
        }
        Expr::SetLiteral { elements, .. } | Expr::AnonCtor { elements, .. } => {
            for el in elements {
                collect_meta_ids(el, out);
            }
        }
        Expr::NamedArg { value, .. } => collect_meta_ids(value, out),
        Expr::Lambda { binders, body, .. } | Expr::Forall { binders, body, .. } => {
            for b in binders {
                if let Some(t) = b.ty.as_deref() {
                    collect_meta_ids(t, out);
                }
            }
            collect_meta_ids(body, out);
        }
        Expr::Arrow {
            domain, codomain, ..
        } => {
            collect_meta_ids(domain, out);
            collect_meta_ids(codomain, out);
        }
        Expr::Plus { lhs, rhs, .. } => {
            collect_meta_ids(lhs, out);
            collect_meta_ids(rhs, out);
        }
        Expr::Let {
            binder, val, body, ..
        } => {
            if let Some(t) = binder.ty.as_deref() {
                collect_meta_ids(t, out);
            }
            collect_meta_ids(val, out);
            collect_meta_ids(body, out);
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            collect_meta_ids(scrutinee, out);
            for arm in arms {
                if let Some(g) = arm.guard.as_ref() {
                    collect_meta_ids(g, out);
                }
                collect_meta_ids(&arm.body, out);
            }
        }
        Expr::Notation { lhs, rhs, .. } => {
            if let Some(l) = lhs.as_deref() {
                collect_meta_ids(l, out);
            }
            if let Some(r) = rhs.as_deref() {
                collect_meta_ids(r, out);
            }
        }
    }
}

/// **B0（2026-10-23）**：把记法节点摊成它的目标应用（`x ∈ A` ⇒ `Set.mem x A`、
/// `∅` ⇒ `Set.empty`），让合一能像普通应用脊一样分解它（只在兜底里用）。
pub(crate) fn desugar_notation(e: &Expr) -> Expr {
    let Expr::Notation {
        target,
        assoc,
        lhs,
        rhs,
        span,
        ..
    } = e
    else {
        return e.clone();
    };
    let head = Expr::Ident {
        name: target.clone(),
        span: *span,
    };
    let app = |f: &Expr, a: &Expr| Expr::App {
        fun: Box::new(f.clone()),
        arg: Box::new(a.clone()),
        explicit_spine: false,
        span: *span,
    };
    match assoc {
        crate::ast::NotationAssoc::Nullary => head,
        crate::ast::NotationAssoc::Prefix => rhs.as_deref().map(|r| app(&head, r)).unwrap_or(head),
        crate::ast::NotationAssoc::Postfix => lhs.as_deref().map(|l| app(&head, l)).unwrap_or(head),
        _ => match (lhs.as_deref(), rhs.as_deref()) {
            (Some(l), Some(r)) => app(&app(&head, l), r),
            _ => e.clone(),
        },
    }
}

/// **P2-b（2026-10-23）**：把整棵项里的记法节点都摊成目标脊（递归版；只在
/// `eqsubst` 的**兜底**里用，判定路径不用它）。
pub(crate) fn desugar_deep(e: &Expr) -> Expr {
    match e {
        Expr::App {
            fun,
            arg,
            explicit_spine,
            span,
        } => Expr::App {
            fun: Box::new(desugar_deep(fun)),
            arg: Box::new(desugar_deep(arg)),
            explicit_spine: *explicit_spine,
            span: *span,
        },
        Expr::Notation { .. } => {
            let d = desugar_notation(e);
            if matches!(d, Expr::Notation { .. }) {
                e.clone()
            } else {
                desugar_deep(&d)
            }
        }
        Expr::Lambda {
            binders,
            body,
            span,
        } => Expr::Lambda {
            binders: binders.clone(),
            body: Box::new(desugar_deep(body)),
            span: *span,
        },
        Expr::Forall {
            binders,
            body,
            span,
        } => Expr::Forall {
            binders: binders.clone(),
            body: Box::new(desugar_deep(body)),
            span: *span,
        },
        Expr::Arrow {
            domain,
            codomain,
            span,
        } => Expr::Arrow {
            domain: Box::new(desugar_deep(domain)),
            codomain: Box::new(desugar_deep(codomain)),
            span: *span,
        },
        other => other.clone(),
    }
}

fn has_unassigned_meta(e: &Expr) -> bool {
    !meta_ids_in(e).is_empty()
}

/// **G-63 ④**：记法节点（源码形态 ✓）与 `target(lhs, rhs)` 应用（elaborate 后 ✓）判同一件事 ✓。
fn notation_matches_app(nota: &Expr, app: &Expr) -> bool {
    let Expr::Notation {
        target, lhs, rhs, ..
    } = nota
    else {
        return false;
    };
    let (Some(l), Some(r)) = (lhs.as_deref(), rhs.as_deref()) else {
        return false;
    };
    let span = nota.span();
    let source = Expr::App {
        fun: Box::new(Expr::App {
            fun: Box::new(Expr::Ident {
                name: target.clone(),
                span,
            }),
            arg: Box::new(l.clone()),
            explicit_spine: false,
            span,
        }),
        arg: Box::new(r.clone()),
        explicit_spine: false,
        span,
    };
    tail_aligned_eq(&source, app)
}

/// 把 `UniverseApp {name, levels}` 降成裸 `Ident {name}`（**只用于上面那一对**：
/// 另一侧是裸 `Ident` 时层级无从比较 ✓，而它的层仍由出口按 Lean 的
/// `levelMVarToParam` 处理 ✓）。`Ident` 原样返回 ✓。
fn bare_head(e: &Expr) -> Expr {
    match e {
        Expr::UniverseApp { name, span, .. } => Expr::Ident {
            name: name.clone(),
            span: *span,
        },
        other => other.clone(),
    }
}

/// **按尾部对齐的一对**（C 子项）：同头、实参个数不同 ⇒ 把**长**的那侧的前导
/// 实参丢掉，返回重建后的 `(左, 右)`；否则 `None`（**其余形状一步都不碰** ✓）。
fn tail_aligned_pair(l: &Expr, r: &Expr) -> Option<(Expr, Expr)> {
    // **快路径（零分配）**：先只**数**实参个数 —— `unify_body` 是热路径（每次
    // 递归比较都过这里），`spine_of` 会为两侧各分配一个 `Vec` ✗。绝大多数调用
    // 在个数相同 / 头不同时就返回 ⇒ 一次分配都不做 ✓（实测：这一条把
    // `unit08` 的 release 耗时从 962ms 压回 ~5xx ms ✓）。
    let (h1, n1) = spine_head_and_len(l);
    let (h2, n2) = spine_head_and_len(r);
    if n1 == n2 || !crate::spine::same_head(h1, h2) {
        return None;
    }
    // 只有**真需要重建**时才分配 ✓（罕见形状）。
    let (_, s1) = crate::spine::spine_of(l);
    let (_, s2) = crate::spine::spine_of(r);
    let (n, drop1, drop2) = if s1.len() > s2.len() {
        (s2.len(), s1.len() - s2.len(), 0)
    } else {
        (s1.len(), 0, s2.len() - s1.len())
    };
    let rebuild = |head: &Expr, args: &[&Expr], drop: usize| -> Expr {
        let span = head.span();
        let mut out = head.clone();
        for a in args.iter().skip(drop).take(n) {
            out = Expr::App {
                fun: Box::new(out),
                arg: Box::new((*a).clone()),
                explicit_spine: false,
                span,
            };
        }
        out
    };
    Some((rebuild(h1, &s1, drop1), rebuild(h2, &s2, drop2)))
}

/// **头 + 实参个数**（**零分配** —— `unify_body` 的热路径快筛 ✓）。
/// 与 `crate::spine::spine_of` 同一份走法（`App` 链），只是不建 `Vec` ✓。
fn spine_head_and_len(e: &Expr) -> (&Expr, usize) {
    let mut n = 0usize;
    let mut cur = e;
    while let Expr::App { fun, .. } = cur {
        n += 1;
        cur = fun;
    }
    (cur, n)
}

/// 头名（`Ident` / `UniverseApp` 都按名字 ✓）。
fn head_name_of(e: &Expr) -> Option<&str> {
    match e {
        Expr::Ident { name, .. } | Expr::UniverseApp { name, .. } => Some(name.as_str()),
        _ => None,
    }
}

/// **按尾部对齐的递归相等**（G-63 ④ ✓）：头名相同（`Ident` / `UniverseApp` 都按名字 ✓），
/// 实参**从尾部**逐个递归比较 ✓ —— 允许一侧**多出前导隐式实参**（elaborate 加上的 ✓）。
fn tail_aligned_eq(a: &Expr, b: &Expr) -> bool {
    let (ha, aa) = crate::spine::spine_of(a);
    let (hb, ab) = crate::spine::spine_of(b);
    match (head_name_of(ha), head_name_of(hb)) {
        (Some(x), Some(y)) if x == y => {}
        _ => return false,
    }
    let n = aa.len().min(ab.len());
    (0..n).all(|k| tail_aligned_eq(aa[aa.len() - 1 - k], ab[ab.len() - 1 - k]))
}

#[cfg(test)]
mod r1b_level_mvar_tests {
    use super::*;

    /// **R1b** ✓：id **单调递增**（对齐 Lean 的 `LMVarId` 全局计数器 ✓）。
    #[test]
    fn fresh_level_mvar_ids_are_monotonic() {
        let a = fresh_level_mvar_id();
        let b = fresh_level_mvar_id();
        assert!(b > a, "fresh id 必须单调递增：{a} ⇒ {b}");
    }
}

pub(crate) mod pattern;

#[cfg(test)]
mod tests;

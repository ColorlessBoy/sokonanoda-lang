//! **切片 1b**：一次 `build <dir>` 内**共享一套 DAG** —— 共享库层只编一次，入口各自复用。
//!
//! 为什么是这个形状（设计 `docs/design/module-artifacts.md` §9）：front **不能命名** kernel 的
//! `pub(crate)` 别名 `DeclarMap` ⇒ 检查点**不能进具名字段** ⇒ 把"库层 + 各入口"的循环整个放进
//! 本函数，检查点只做**局部变量**（类型推断即可）。
//!
//! 语义：库层（各 `lib/*`）编一次 ⇒ `hide_declars()` 留检查点 ⇒ 每个入口 `restore_declars`
//! 回到"只有库层"⇒ **只走该入口自己的命令** ⇒ 编完 `hide_declars()` 丢掉入口声明
//! ⇒ **单元之间从不共处一个环境**（09-25 那次假"重复声明"的结构性根因因此消失）。
//!
//! **§33（2026-10-08 · G-29 第 5 棒）**：同一份"只有库层"的环境还要**跨调用**活着
//! —— 改一行入口文件时，库层（依赖模块）的源文本一个字节没变，却每次都要重新
//! elaborate（实测 unit08 改一行 ≈ **695ms / 24%**，见设计 §33 的成本分解）。
//! [`with_project_session_reusing`] 就是那条路：把"库层趟"的产物（`EnvBuilder` +
//! `PassTables` + 库层输出/报告/区间）留在**跨调用的持有者**里，摘要逐字相同就直接
//! 接着编入口（省掉库层趟）✓。
//!
//! ⚠ **检查点不能进 `QueryDoc`**（2026-10-08 实测 ✗）：内核环境借 `&'a ArenaRef<'a>`，
//! 而 `ArenaRef` 是 `!Send`（内部是 `Cell<*mut u8>` 与裸指针）⇒ 带检查点的 `QueryDoc`
//! 立刻撞 LSP 的 `tokio::spawn`（`Doc` 必须 `Send + Sync`，`crates/lsp/src/lib.rs:443`
//! 的 `static EMPTY: OnceLock<Doc>` 与 `:549` 的 spawn 都会判红 ✗）。⇒ 持有者改成
//! **线程局部**（[`LIB_CHECKPOINT`]）：它不需要 `Send` ✓，代价是检查点**只对同一条
//! 线程**可见 ⇒ 用它的那条路（LSP）必须把编译钉在一条线程上（见 `crates/lsp/src/lib.rs`
//! 的编译专用 runtime）。

use std::cell::RefCell;
use std::sync::atomic::{AtomicUsize, Ordering};

use sokonanoda::builder::EnvBuilder;
use sokonanoda::util::Config;
use stumpalo::ArenaRef;

use crate::compile::{run_pass_with, split_report, CompileOptions, PassTables, SourceUnit};
use crate::compile::{CompileOutput, DocumentReport};

/// **S2 步 2**：某个入口那一趟的**信任前缀**（I8 的 `TrustPlan` + 已缓存失败）。
///
/// 为什么需要：闭包编译今天对入口走的是**整份重查**（`check/mod.rs:608` 原文
/// "闭包编译不使用 TrustPlan（v1）"）⇒ 改一个 `theorem` 要把**前面所有**声明
/// 重查一遍（实测 unit08 改一行 **2189ms**，且与改动位置无关 —— 见
/// `docs/design/declaration-incremental.md` §1.2）。
///
/// `plan.before` 与 `failures` 的键都在**该入口自己的命令流**坐标系里
/// （入口趟 `run_pass_with` 只看到 `entry_units`，库层命令不在它的下标空间里）✓。
///
/// ⚠ **本结构只声明"这段前缀的文本没变过、上一轮查过"** —— 文本没变 ⇒ 前缀
/// 语义不变，是既有 I8 不变式（单文件路径已用了很久）。**被信任的那段不会
/// 出现在返回的报告里**（状态由调用方的会话缓存补）⇒ 调用方必须自己拼回去，
/// 否则报告会缺声明（设计 §4.2）。
pub(crate) struct EntryTrust {
    pub plan: crate::compile::TrustPlan,
    pub failures: crate::compile::KernelFailed,
}

/// **库层级检查点**（设计 §33）：库层那趟跑完之后的**活环境** + 该趟的产物。
///
/// 「只有库层」= `builder` 里**只有库层声明**（没有任何入口声明）：接着编入口时
/// 克隆它即可（[`EnvBuilder: Clone`] 是浅拷贝 ⇒ 同一份 arena、同一批指针 ✓）。
///
/// ⚠ **可复用的充要条件**（三条，缺一不可 —— 设计 §33 的不变量）：
/// ① **库层摘要逐字相同**（[`lib_key`]：模块集合**与顺序** · 名字/路径/源文本 ·
///    import 边 · prelude 模式 · 模块根 —— 后两者在 `src`/`path` 与 `options` 里）；
/// ② 入口那一趟的输入**逐字重算**（`entry_units`/`entry_prefixes`/`entry_display`/
///    `entry_defs`/`options`/`trust` 每次都由本次调用现算，**不从检查点里拿** ✗）；
/// ③ 复用来的环境与被复用者**逐字段同一**（浅拷贝 ⇒ 指针同一 ⇒ 内核那些按指针
///    比较的地方不变，见 `crates/kernel/src/builder.rs` 的两向判据 ✓）。
pub(crate) struct LibCheckpoint<'a> {
    /// 库层摘要（[`lib_key`]）—— 复用判据的键。
    key: String,
    /// 库层趟跑完之后的 builder（**含库层声明**，不是 hidden 状态）。
    builder: EnvBuilder<'a>,
    /// 库层趟跑完之后的登记表（`PassTables` 是跨趟累加的 ⇒ 必须与 `builder` 同代）。
    tables: PassTables<'a>,
    /// 库层那趟的合并输出（`kernel_checks` 已搬好）。
    out: CompileOutput,
    /// 库层逐模块报告（**并集顺序**）。
    reports: Vec<DocumentReport>,
    /// 并集顺序下每个库模块的命令区间。
    ranges: Vec<std::ops::Range<usize>>,
    /// 库层那趟的命令数（合并输出的偏移量）。
    n_commands: usize,
    /// 库层那趟**去掉 `import` 行**之后的命令数（入口趟 `judge_prefix_offset`）。
    prefix_commands: usize,
    /// 这份检查点被复用了多少次（单份 arena 的增长上界，见 [`MAX_REUSES_PER_CHECKPOINT`]）。
    reuses: usize,
}

/// **上界 ①**：进程内**泄漏的库层 arena** 数的上限。
///
/// `Box::leak`（零 `unsafe`，设计 §33 授权的形状 (a)）拿不到 `'static` 的另一种写法
/// —— 泄漏的 arena **永远不回收**（回收要 `unsafe` ✗）⇒ 必须给上界。到顶之后
/// **不再建新检查点**（回退到今天那条路：栈上 arena + 逐入口 `hide/restore`，
/// **逐字节相同**）⇒ 泄漏量 ≤ `MAX_LEAKED_LIB_ARENAS × 一份库层 arena` ✓。
const MAX_LEAKED_LIB_ARENAS: usize = 8;

/// **上界 ②**：一份检查点最多被复用多少次。
///
/// arena 是 **bump allocator**：入口那趟的每一次分配都留在同一份 arena 里
/// （内核的指针同一性要求入口与库层共用一份 arena ⇒ 不能给入口单独开一份 ✗）
/// ⇒ 复用次数不设上限的话，单份 arena 会随按键无界增长 ✗。到顶就**轮换**
/// （丢掉检查点、下次重建）⇒ 单份 arena 的增长有界 ✓（代价 = 每 N 次按键多付
/// 一次库层趟 ≈ 695ms，摊到每次 ≈ 11ms）。
const MAX_REUSES_PER_CHECKPOINT: usize = 64;

/// 进程内**已经泄漏**的库层 arena 数（只增不减 —— 泄漏的定义）。
static LEAKED_LIB_ARENAS: AtomicUsize = AtomicUsize::new(0);

thread_local! {
    /// **跨调用的持有者**：本线程的库层检查点（最多一份 —— 换掉旧的 ⇒ 旧的
    /// **语义上不可达** ✓）。
    ///
    /// 为什么是线程局部而不是 `QueryDoc` 的字段：内核环境是 `!Send`（见文件头），
    /// 而 `QueryDoc` 必须 `Send + Sync`（LSP 把它放进 `tokio::spawn` 的 future）✗。
    /// 为什么不是 `static Mutex<...>`：那要求 `Send` ✗（同一个原因）。
    static LIB_CHECKPOINT: RefCell<Option<LibCheckpoint<'static>>> = const { RefCell::new(None) };
}

/// **上界判据的读数**（`#[doc(hidden)]`，判据用）：已经泄漏的库层 arena 数。
#[doc(hidden)]
pub fn lib_checkpoint_arenas_leaked() -> usize {
    LEAKED_LIB_ARENAS.load(Ordering::Relaxed)
}

/// **上界判据的读数**（`#[doc(hidden)]`）：本线程现在有没有活着的检查点。
#[doc(hidden)]
pub fn lib_checkpoint_is_live() -> bool {
    LIB_CHECKPOINT.with(|slot| slot.borrow().is_some())
}

/// **上界判据的读数**（`#[doc(hidden)]`）：本线程那份检查点被复用了多少次。
#[doc(hidden)]
pub fn lib_checkpoint_reuses() -> usize {
    LIB_CHECKPOINT.with(|slot| slot.borrow().as_ref().map_or(0, |cp| cp.reuses))
}

/// **判据用**（`#[doc(hidden)]`）：清掉本线程的检查点与泄漏计数（测试隔离）。
#[doc(hidden)]
pub fn lib_checkpoint_reset() {
    LIB_CHECKPOINT.with(|slot| *slot.borrow_mut() = None);
    LEAKED_LIB_ARENAS.store(0, Ordering::Relaxed);
}

/// 库层摘要（**可复用判据的键**，设计 §33 的不变量 ①）。
///
/// 进键的每一样都必须**逐字节**决定库层那趟的输入：
/// * **模块集合与顺序**：按 `lib_units` 的顺序拼（顺序不同 ⇒ 键不同 ✓）；
/// * **名字 · 路径 · 源文本**：路径进键的理由同 `ProjectPlan::digest`（T-A06：
///   内容相同不代表位置相同 ⇒ 两个目录下逐字相同的项目不许共用检查点 ✗）；
///   `src` 里含 `import` 行 ⇒ **import 边**也在键里 ✓；
/// * **prelude 模式 / 版本 / build stamp / `SOKO_*` 开关 / 元变量档**：由
///   [`crate::compile::cache::key`] 统一折入（**单一真相** ✓，别在这里另造一套）；
/// * **模块根**：每个模块的**绝对路径**已在键里 ⇒ 根变了路径就变 ✓。
fn lib_key(lib_units: &[SourceUnit<'_>], options: &CompileOptions) -> String {
    let mut text = String::from("soko.lib-checkpoint/1\0");
    for unit in lib_units {
        text.push_str(unit.name);
        text.push('\0');
        if let Some(path) = unit.path {
            text.push_str(&path.to_string_lossy());
        }
        text.push('\0');
        text.push_str(&unit.file.src);
        text.push('\0');
    }
    crate::compile::cache::key(&text, options)
}

/// 跑一次"库层一次 + 各入口各自"的编译会话；每个入口的结果经 `on_entry` 交回。
///
/// `lib_units` = 共享库层的单元（拓扑序）· `entries[i]` = 第 i 个入口**自己的**单元
/// （**不含**依赖 —— 依赖已在环境里）。
pub fn with_project_session<R>(
    lib_units: &[SourceUnit<'_>],
    entries: &[Vec<SourceUnit<'_>>],
    options: &CompileOptions,
    on_entry: impl FnMut(
        usize,
        CompileOutput,
        Vec<DocumentReport>,
        &[DocumentReport],
        // **并集顺序**下每个库模块的命令区间（修法 A：按各入口自己的闭包顺序拼接 + 重编号）。
        &[std::ops::Range<usize>],
        // 该入口在**合并输出**里的命令区间（`lib_n..lib_n + 入口那趟命令数`）。
        std::ops::Range<usize>,
    ) -> R,
) -> Vec<R> {
    with_project_session_trusted(lib_units, entries, options, &[], on_entry)
}

/// 把库层单元**克隆进泄漏内存** ⇒ `&'static [SourceUnit<'static>]`。
///
/// 为什么需要：`run_pass_with` 要求 `units: &'a [SourceUnit<'a>]` 与 arena **同一个
/// 生命周期参数**，而检查点要**跨调用**活着（`'static`）⇒ 库层那趟必须跑在
/// `'static` 的单元上。克隆**逐字节相同**（`FolFile` 没有内部生命周期）⇒ 不影响
/// 任何判定 ✓；代价 = 每个检查点多泄漏一份库层源文本（与 arena 同一个上界 ✓）。
fn leak_lib_units(lib_units: &[SourceUnit<'_>]) -> &'static [SourceUnit<'static>] {
    let units: Vec<SourceUnit<'static>> = lib_units
        .iter()
        .map(|unit| SourceUnit {
            name: Box::leak(unit.name.to_string().into_boxed_str()),
            path: unit
                .path
                .map(|path| &*Box::leak(path.to_path_buf().into_boxed_path())),
            file: Box::leak(Box::new(unit.file.clone())),
        })
        .collect();
    Box::leak(units.into_boxed_slice())
}

/// 同 [`with_project_session`]，但**每个入口可以带一份信任前缀**（S2 步 2）。
///
/// `entry_trust[i]` = 第 i 个入口的 [`EntryTrust`]；`None`/缺省 ⇒ 那一趟与今天
/// **逐字节相同**（整份重查）⇒ 既有调用方（CLI `build`）行为零变化 ✓。
pub(crate) fn with_project_session_trusted<R>(
    lib_units: &[SourceUnit<'_>],
    entries: &[Vec<SourceUnit<'_>>],
    options: &CompileOptions,
    entry_trust: &[Option<EntryTrust>],
    on_entry: impl FnMut(
        usize,
        CompileOutput,
        Vec<DocumentReport>,
        &[DocumentReport],
        &[std::ops::Range<usize>],
        std::ops::Range<usize>,
    ) -> R,
) -> Vec<R> {
    // **这条路的 arena 建在栈上**（一次调用一份，调用结束即释放）⇒ 与今天逐字节
    // 相同 ✓（CLI `build` 走的就是它；检查点那条路见
    // [`with_project_session_reusing`]）。
    let arena = stumpalo::Arena::new();
    let builder = EnvBuilder::new(arena.as_arena_ref(), Config::default());
    let lib = run_library_pass(
        builder,
        PassTables::new(),
        lib_units,
        options,
        String::new(),
    );
    run_entries(&lib, lib_units, entries, options, entry_trust, on_entry)
}

/// **设计 §33 的落地**：库层检查点**跨调用**复用（同一份库层摘要 ⇒ 省掉库层趟）。
///
/// 与 [`with_project_session_trusted`] 的唯一区别：库层那趟的产物留在
/// **线程局部**（[`LIB_CHECKPOINT`]）里；下一次调用若库层摘要**逐字相同**就直接
/// 克隆它接着编入口 ✓。
///
/// **回退是默认**（设计 §33）：摘要不等 · 检查点为空 · 入口被阻断 · 库层为空 ·
/// 上界用尽 ⇒ **整条重编**并清掉检查点（与今天逐字节相同）✓。**不猜** ✓。
pub(crate) fn with_project_session_reusing<R>(
    lib_units: &[SourceUnit<'_>],
    entries: &[Vec<SourceUnit<'_>>],
    options: &CompileOptions,
    entry_trust: &[Option<EntryTrust>],
    on_entry: impl FnMut(
        usize,
        CompileOutput,
        Vec<DocumentReport>,
        &[DocumentReport],
        &[std::ops::Range<usize>],
        std::ops::Range<usize>,
    ) -> R,
) -> Vec<R> {
    let key = lib_key(lib_units, options);
    LIB_CHECKPOINT.with(|slot| {
        let mut slot = slot.borrow_mut();
        // ① **复用判据**：摘要逐字相同 + 复用次数未到上界（上界 ②）。
        let hit = !lib_units.is_empty()
            && slot
                .as_ref()
                .is_some_and(|cp| cp.key == key && cp.reuses < MAX_REUSES_PER_CHECKPOINT);
        if hit {
            let cp = slot.as_mut().expect("上面刚判过 Some");
            cp.reuses += 1;
            // `&LibCheckpoint<'static>` 按协变缩到本次 units 的寿命 ✓（浅拷贝 ⇒
            // 指针同一 ✓）。
            let lib: &LibCheckpoint<'_> = cp;
            return run_entries(lib, lib_units, entries, options, entry_trust, on_entry);
        }
        // ② **重建**：库层趟跑在一份**泄漏的** arena 上（`Box::leak` = 零 `unsafe`
        //    的 `'static` 来源 ✓）。库层为空（单文件）或泄漏上界用尽（上界 ①）
        //    ⇒ 不建检查点，走栈上 arena（= 今天那条路，逐字节相同）✓。
        let can_leak = !lib_units.is_empty()
            && LEAKED_LIB_ARENAS.load(Ordering::Relaxed) < MAX_LEAKED_LIB_ARENAS;
        if can_leak {
            let arena: &'static ArenaRef<'static> =
                Box::leak(Box::new(stumpalo::Arena::new())).as_arena_ref();
            LEAKED_LIB_ARENAS.fetch_add(1, Ordering::Relaxed);
            let builder = EnvBuilder::new(arena, Config::default());
            // 库层趟跑在**泄漏的单元副本**上（`run_pass_with` 要求 units 与 arena
            // 同寿命 ⇒ 要存成 `'static` 检查点就得让 units 也是 `'static` ✓）。
            let units = leak_lib_units(lib_units);
            let lib = run_library_pass(builder, PassTables::new(), units, options, key);
            let out = run_entries(&lib, lib_units, entries, options, entry_trust, on_entry);
            // 换掉旧的检查点 ⇒ 旧的**语义上不可达** ✓（它的 arena 已经泄漏，
            // 但**活着的**检查点恒 ≤ 1 份 ✓）。
            *slot = Some(lib);
            out
        } else {
            *slot = None;
            let arena = stumpalo::Arena::new();
            let builder = EnvBuilder::new(arena.as_arena_ref(), Config::default());
            let lib = run_library_pass(builder, PassTables::new(), lib_units, options, key);
            run_entries(&lib, lib_units, entries, options, entry_trust, on_entry)
        }
    })
}

/// **A5（2026-10-08）**：只跑**库层趟**、把检查点喂热 —— **不跑入口趟、不产报告、不发诊断**。
///
/// ## 为什么需要它（开工 profiling 实测 · `PLAN-cli-editor-perf.md` §8.2）
///
/// **产物命中**那条路（`crates/lsp/src/lib.rs` 的 `set_cached_entry`）**一趟 pass 都不跑**
/// ⇒ 本线程的 [`LIB_CHECKPOINT`] 是**冷的** ⇒ **开档后的第一次编辑**要走
/// [`with_project_session_reusing`] 的 miss 分支，把**整条库闭包重编一遍**：
/// 实测 unit08 同一刀 = `modules=5 by=87 prefix=16` · **1233ms**，而检查点热的
/// 同一刀只要 `modules=1 by=63` · **321ms** ✗。用户看到的就是"打开很快、敲第一个
/// 字符卡一秒"。
///
/// ## 为什么它**不会更坏**（调用方敢在后台起它的理由）
///
/// 库层趟是"首次编辑那次编译**本来就必须做**的那部分功"的**子集**：
/// * 编辑**先**到 ⇒ 由编辑自己的编译建检查点，总功不变（调用方起预热前会再看一眼
///   有没有待编的编辑，有就跳过 ⇒ 这一趟压根不会跑）；
/// * 编辑**后**到 ⇒ 省下这一趟 ✓。
///
/// ⇒ 与"今天"相比**功只减不增**，最坏情况 = 今天。
///
/// ## 判据
///
/// 返回 `true` = 真的建了一份**新**检查点（判据读数：预热有没有生效）；
/// `false` = 库层为空 / 已经热的（键相同）/ 泄漏上界用尽 ⇒ **什么都不做**。
pub(crate) fn warm_library(lib_units: &[SourceUnit<'_>], options: &CompileOptions) -> bool {
    if lib_units.is_empty() {
        return false;
    }
    let key = lib_key(lib_units, options);
    LIB_CHECKPOINT.with(|slot| {
        let mut slot = slot.borrow_mut();
        // 已经热的（键逐字相同）⇒ 别重复做（否则每开一次档白烧一趟库层 ✗）。
        if slot.as_ref().is_some_and(|cp| cp.key == key) {
            return false;
        }
        // 上界 ①（与 miss 分支同一条）：到顶就不再建新检查点，回退到今天那条路。
        if LEAKED_LIB_ARENAS.load(Ordering::Relaxed) >= MAX_LEAKED_LIB_ARENAS {
            return false;
        }
        let arena: &'static ArenaRef<'static> =
            Box::leak(Box::new(stumpalo::Arena::new())).as_arena_ref();
        LEAKED_LIB_ARENAS.fetch_add(1, Ordering::Relaxed);
        let builder = EnvBuilder::new(arena, Config::default());
        let units = leak_lib_units(lib_units);
        let lib = run_library_pass(builder, PassTables::new(), units, options, key);
        // 换掉旧的 ⇒ 旧的**语义上不可达** ✓（与 miss 分支同一条不变量）。
        *slot = Some(lib);
        true
    })
}

/// **库层趟**（切片 1b 的第 ① 步）：编共享库层，产出检查点（含活环境）。
///
/// 与既有代码逐字同构：`install_preludes = true`（**只装一次** —— 入口趟必须
/// `false`，否则实测 `duplicate declaration Nat` ✗）、`judge_prefix_offset = 0`
/// （库层趟的 `idx` 与 judge 的前缀**同坐标系** ✓）。
fn run_library_pass<'a>(
    builder: EnvBuilder<'a>,
    tables: PassTables<'a>,
    lib_units: &'a [SourceUnit<'a>],
    options: &CompileOptions,
    key: String,
) -> LibCheckpoint<'a> {
    // 影子不建：`None` ⇒ 不需要额外的局部 arena，见 `run_pass_with` 的注释。
    let (lib_pass, builder, tables) = run_pass_with(
        builder, None, true, tables, lib_units, options, true, None, None, None, None, None, None,
        // 建议材料：库层趟按 `lib_units` 自己算 ✓。
        None,
        // 库层趟：judge 的前缀（`closure_prefixes_for(lib_units)`）与本趟 `idx`
        // **同坐标系** ✓ ⇒ 不平移。
        0,
    );
    // **G-29 第 3 棒**：入口趟的 `idx` 是**入口空间**的，而 judge 的合成前缀是
    // **整条闭包** ⇒ 压栈的担保必须平移"**库层那一段的命令数**" ✓，否则
    // `synthesized_trust` 的闸门 `before >= prefix_commands` 恒不成立 ⇒ 入口趟
    // 每次 `judge_infer` 未命中都整份重编 ✗（实测 7 次 · 见设计 §33）。
    //
    // ⚠ 数的是**去掉 `import` 行之后**的命令数（`importless_source` 会剥掉它们，
    // 见 `closure_prefixes_for` ✓）—— 多算只会被 `before.min(prefix_commands)`
    // 夹回（更保守 ✓），**少算才会漏担保** ✗ ⇒ 必须按同一口径数 ✓。
    let prefix_commands: usize = lib_units
        .iter()
        .map(|unit| {
            unit.file
                .commands
                .iter()
                .filter(|command| !matches!(command, crate::ast::Command::Import { .. }))
                .count()
        })
        .sum();
    // 读在 `lib_pass.report` 被搬走**之前**（`split_report` 会吃掉它）。
    let checks = lib_pass.kernel_checks();
    let ranges = crate::compile::unit_ranges(lib_units);
    let reports = split_report(
        lib_pass.report,
        &lib_pass.out.error_cmds,
        &lib_pass.out.warning_cmds,
        lib_units,
    );
    let mut out = lib_pass.out;
    out.stats.kernel_checks = checks;
    LibCheckpoint {
        key,
        builder,
        tables,
        out,
        reports,
        ranges,
        n_commands: lib_pass.n_commands,
        prefix_commands,
        reuses: 0,
    }
}

/// **各入口各自一趟**（切片 1b 的第 ③ 步）：每个入口从"只有库层"的环境起跑。
///
/// 与既有代码的差别只有一处：入口趟拿的是检查点 builder 的**克隆**
/// （`hide_declars`/`restore_declars` 的等价物 —— `DeclarMap` 在 front 里**不能命名**
/// ⇒ 检查点只能整份拿着；浅克隆 ⇒ 指针同一 ✓）。**入口之间仍从不共处一个环境** ✓
/// （G-68：登记表也是逐入口克隆的，前一个入口的声明不会泄进后一个 ✓）。
fn run_entries<'a, R>(
    lib: &LibCheckpoint<'a>,
    lib_units: &'a [SourceUnit<'a>],
    entries: &'a [Vec<SourceUnit<'a>>],
    options: &CompileOptions,
    entry_trust: &[Option<EntryTrust>],
    mut on_entry: impl FnMut(
        usize,
        CompileOutput,
        Vec<DocumentReport>,
        &[DocumentReport],
        &[std::ops::Range<usize>],
        std::ops::Range<usize>,
    ) -> R,
) -> Vec<R> {
    let lib_n = lib.n_commands;
    let mut out = Vec::with_capacity(entries.len());
    for (index, entry_units) in entries.iter().enumerate() {
        // ③ 回到只有库层的状态 ⇒ 入口之间不共享环境（克隆 = 浅拷贝 ⇒ 指针同一 ✓）。
        let builder = lib.builder.clone();
        let tables = lib.tables.clone();
        // **切片 1 路乙**：入口趟必须拿到"**该入口闭包**"的闭包前缀与记法表 ——
        // 否则入口里的 `judge_infer` **看不到库层声明**（它只吃源码字符串，
        // `judge.rs:949`）⇒ 实测这是三次接线失败的同一个根因
        //（`docs/design/incremental-environment.md` §29.2）。
        // 该入口闭包 = `lib_units`（本 session 的库层）+ 该入口自己的单元。
        let entry_closure: Vec<SourceUnit<'_>> = lib_units
            .iter()
            .chain(entry_units.iter())
            .map(|u| SourceUnit {
                name: u.name,
                path: u.path,
                file: u.file,
            })
            .collect();
        // ⚠ **取"最后一格"是必须的**（2026-09-29 实测踩到）：入口趟传给
        // `run_pass_with` 的 `units` 是 **`entry_units`（长度 1）** ⇒ walk 里
        // `unit_idx` **恒为 0** ⇒ 它要的是"**入口那一格**"的前缀
        // = `entry_prefixes` 的**最后一格**（前几格属于库单元，第一格还是**空串**）。
        // 直接把整个 `entry_prefixes` 传进去 ⇒ `get(0)` = 空串 ⇒ 走 `_ =>` 分支
        // ⇒ **入口没有库层前缀** ✗ ⇒ 实测报「前缀源码无法解析」+ 一串记法解析失败
        // （`≠`/`{a,b}`/`=`/`∈` 全都"读不到目标类型"）。
        let entry_prefixes_all = crate::compile::closure_prefixes_for(&entry_closure);
        let entry_prefixes: Vec<String> = entry_prefixes_all
            .last()
            .map(|last| vec![last.clone()])
            .unwrap_or_default();
        let entry_display = crate::compile::display_notations(&entry_closure);
        // **跨模块 hover 回填**（切片 1b 的入口趟）：`resolution` 要指向**库层**声明
        // 的真实 span，而入口趟的 `units` 只有入口 ⇒ 不传这张表的话，入口里
        // `Point`（来自 `import Lib`）的 hover `resolution` 会退化成 `None`
        // ⇒ F12/高亮在跨模块名字上失效 ✗（实测：与会话外整份编译的报告因此不同）。
        let entry_defs = crate::compile::top_level_def_spans_over(&entry_closure);
        // **S2 步 2**：该入口这一趟的信任前缀（缺省 = 整份重查，与今天逐字节相同）。
        let trusted = entry_trust.get(index).and_then(|slot| slot.as_ref());
        let (pass, _next, _next_tables) = run_pass_with(
            builder,
            None,
            false,
            tables,
            entry_units,
            options,
            true,
            trusted.map(|t| &t.failures),
            trusted.map(|t| &t.plan),
            None,
            Some(&entry_prefixes),
            Some(&entry_display),
            Some(&entry_defs),
            // **G-29 第 5 棒**：建议材料按**该入口闭包**算 —— 入口趟的 `units` 只有
            // 入口（长度 1）⇒ 不传这一格的话 refine/intro 建议会**丢掉被导入模块里
            // 的构造子** ✗（实测：`code_actions_work_in_a_project_entry` 的
            // `refine And.intro` 消失 ✗）。
            Some(&entry_closure),
            // **G-29 第 3 棒**：把本趟 `idx` 平移到闭包坐标系（见 `prefix_commands`）。
            lib.prefix_commands,
        );
        let entry_range = lib_n..lib_n + pass.n_commands;
        // 读在 `pass.report` 被搬走**之前**（`split_report` 会吃掉它）。
        let entry_checks = pass.kernel_checks();
        let entry_reports = split_report(
            pass.report,
            &pass.out.error_cmds,
            &pass.out.warning_cmds,
            entry_units,
        );
        // **闭包级扁平输出**（库层在前、入口在后，命令号整体偏移 `lib_n`）⇒ 接线方
        // 能按 `unit_ranges(闭包 units)` 正确切分事件（与今天逐字节等价的前提）。
        let mut merged = lib.out.clone();
        merged.events.extend(pass.out.events.iter().cloned());
        merged
            .event_cmds
            .extend(pass.out.event_cmds.iter().map(|c| c + lib_n));
        merged.errors.extend(pass.out.errors.iter().cloned());
        merged
            .error_cmds
            .extend(pass.out.error_cmds.iter().map(|c| c + lib_n));
        merged.warnings.extend(pass.out.warnings.iter().cloned());
        merged
            .warning_cmds
            .extend(pass.out.warning_cmds.iter().map(|c| c + lib_n));
        // **S2 步 2 顺带修的一个漏**：`run` 会把 `pass.checks` 搬进
        // `out.stats.kernel_checks`，而这条路以前**只加了没赋值的那个 0** ⇒
        // 合并输出里的 `kernel_checks` 恒为 0（判据读不到"少查了多少"）。
        merged.stats.kernel_checks += entry_checks;
        out.push(on_entry(
            index,
            merged,
            entry_reports,
            &lib.reports,
            &lib.ranges,
            entry_range,
        ));
        // ④ 丢掉这个入口的声明：入口趟的 builder/表是**克隆**，作用域结束即丢 ✓
        //    （下一次循环再从检查点克隆一份）。
    }
    out
}

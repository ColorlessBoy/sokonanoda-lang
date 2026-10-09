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
//! **线程局部**（[`LIB_CHECKPOINTS`]）：它不需要 `Send` ✓，代价是检查点**只对同一条
//! 线程**可见 ⇒ 用它的那条路（LSP）必须把编译钉在一条线程上（见 `crates/lsp/src/lib.rs`
//! 的编译专用 runtime）。

use std::cell::RefCell;
use std::sync::atomic::{AtomicUsize, Ordering};

use sokonanoda::builder::EnvBuilder;
use sokonanoda::util::Config;
use stumpalo::ArenaRef;

use crate::compile::{
    install_all_preludes, prelude_shape, CompileOutput, DocumentReport, ResumeState,
};
use crate::compile::{run_pass_with, split_report, CompileOptions, PassTables, SourceUnit};
use crate::display::DisplayNotations;

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
    ///
    /// **T1-A（2026-10-09）**：粒度从"**整条库层**"细到"**一个模块前缀**"——
    /// 这条键 = `lib_key(lib_units[0..=j])`（第 j 个模块编完时的前缀）✓。
    key: String,
    /// **T1-A**：这条检查点所属那趟的**库层 prelude 形状**（[`crate::compile::prelude_shape`]）。
    ///
    /// 为什么必须比对（**前缀复用独有的口子**）：`install_all_preludes` 是按
    /// **整条闭包**判让位的（`explicit_nat`/`explicit_bool`/`shadowed`）⇒ 两个
    /// 库层**前缀相同、整条不同**的话，prelude 环境可能不同 ⇒ 复用 = 静默改判 ✗。
    ///
    /// `None` = 这份检查点**不参与前缀复用**（整条一趟建的那条路：
    /// `run_library_pass` / A5 预热）。整条命中**不需要**它：键已经把整条源文本
    /// 折进去了 ⇒ 形状是键的纯函数 ✓（今天本来就不比 ✓）。
    shape: Option<crate::compile::PreludeShape>,
    /// **T1-A**：续编状态（`closure_id`/`exports`/`example_idx`）。
    /// 整条一趟建的那条路填 [`ResumeState::default`]（它只服务**整条命中**，
    /// 而整条命中**不从断点续编** ⇒ 这三个累加器用不上 ✓）。
    resume: ResumeState,
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
    /// **T2-B0（2026-10-09）**：**库层那一段**的显示记法表 —— 它是闭包文本的纯函数 ✓，
    /// 随检查点存**一次**，每刀只建"入口那一段"再 [`merged_with`](crate::display::DisplayNotations::merged_with) ✓。
    lib_display: crate::display::DisplayNotations,
    /// **T2-B0**：**库层那一段**的定义 span 表（同 [`Self::lib_display`] 的理由 ✓）。
    /// 合并顺序 = **库层先**（与"库层 ++ 入口"的一次性建表同序 ⇒ `or_insert` 语义一致 ✓）。
    lib_defs: std::collections::HashMap<String, crate::Span>,
    /// **A4a（2026-10-08）**：库层全部单元拼接之后的**闭包前缀**（去 `import` 行 + 补行尾换行）。
    ///
    /// 入口趟要的前缀恰好就是它（`entry_closure` 的最后一格 = 库层那一段 ✓）——
    /// 它是 `lib_key` 的**纯函数** ⇒ 随检查点存一次、每刀克隆即可 ✓
    /// （以前每一刀重跑一遍 O(闭包) 的累加 ✗，判据读数 `closure_prefix_builds_total`）。
    lib_prefix: String,
}

/// **T1-A（2026-10-09）**：逐模块库层趟的**游标** —— "已经编好的前缀"的全部状态。
///
/// 它与 [`LibCheckpoint`] 的差别只有一个：**寿命**。检查点要进线程局部（`'static`），
/// 游标只活在**这一趟续编**里（`'a` = 本次调用的单元寿命）⇒ **续编不必泄漏单元**
/// （今天只有"冷建检查点"那条路才 `leak_lib_units` ✓ —— 泄漏量因此仍受
/// [`MAX_LEAKED_LIB_ARENAS`] 管 ✓）。
struct LibCursor<'a> {
    builder: EnvBuilder<'a>,
    tables: PassTables<'a>,
    out: CompileOutput,
    reports: Vec<DocumentReport>,
    n_commands: usize,
    prefix_commands: usize,
    lib_prefix: String,
    /// 下一个模块的**续编状态**（第一个模块用调用方给的那份）。
    resume: Option<ResumeState>,
    /// 下一个要编的模块下标。
    next: usize,
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

/// **T1-A（2026-10-09）上界 ③**：LRU 里**模块级检查点**的份数上限。
///
/// 与上界 ①（泄漏的 **arena**）分工不同，别混：
/// * ① 管的是 **arena**（每份 ≈ 一整条库层的**项图**，最贵的那部分）；
/// * 本上界管的是**检查点条目**（每份 = 一份 `EnvBuilder` **浅拷贝** + 三张前端表
///   —— 只复制**表项指针**，**不复制 arena 里的项** ✗）。
///
/// 为什么要比 arena 数大：一份 arena 上现在可以挂**多个模块边界**（一条库层有
/// `n` 个模块 ⇒ 最多 `n` 份），而"换一个入口、共享前几个模块"要的正是**浅**的那几份 ✓。
/// 32 的取法：真课程单条库闭包 ≤ 8 个模块（unit08 实测 ✓）⇒ 32 够装**四条**不同的
/// 库层前缀族；到顶按 LRU 淘汰 ⇒ **活着的**检查点恒 ≤ 32 份 ✓。
const MAX_MODULE_CHECKPOINTS: usize = 32;

/// 进程内**已经泄漏**的库层 arena 数（只增不减 —— 泄漏的定义）。
static LEAKED_LIB_ARENAS: AtomicUsize = AtomicUsize::new(0);

/// **诊断读数（2026-10-09）**：**上一趟"库层从哪来"** —— 0 = 没走会话 ·
/// 1 = 线程局部检查点（①）· 2 = **磁盘产物**（①.5）· 3 = 前缀续编（②）· 4 = 整条重建（③）。
///
/// **为什么要有它**：`crates/lsp` 的 `LSP_TRACE compile …` 行**只报结构计数**
/// （`modules`/`by`/`prefix` ✓）⇒ 从读数**反推不出**是哪条路服务的 ✗
/// （第 50 轮我就是拿"stderr 里没有我打的 trace"下了**错**结论 ✗ ⇒ §41 的撤回 ✓）。
/// 有了这条，探针**只读既有 trace 行**就能回答"走没走到产物那条" ✓。
static LAST_LIB_SOURCE: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

/// 见 [`LAST_LIB_SOURCE`]：`"lru"` / `"artifact"` / `"prefix"` / `"rebuilt"` / `"none"`。
#[doc(hidden)]
pub fn last_lib_source() -> &'static str {
    match LAST_LIB_SOURCE.load(Ordering::Relaxed) {
        1 => "lru",
        2 => "artifact",
        3 => "prefix",
        4 => "rebuilt",
        _ => "none",
    }
}

/// 见 [`last_lib_source`]（`#[doc(hidden)]`，只给诊断用）。
#[doc(hidden)]
pub fn set_last_lib_source(v: u8) {
    LAST_LIB_SOURCE.store(v, Ordering::Relaxed);
}

thread_local! {
    /// **跨调用的持有者**：本线程的库层检查点，**最多 [`MAX_MODULE_CHECKPOINTS`] 份**，
    /// 按 **LRU** 淘汰（**队首 = 最近用过** ✓）。
    ///
    /// ## 为什么不是一份（2026-10-08 实测驱动 · A3 读数的 ④ 号事实）
    ///
    /// 以前只有一份 ⇒ **换一个闭包就把上一条入口的检查点挤掉** ✗：实测
    /// `MainA → MainB → MainA` 的第三刀**又是整条闭包重编**（3 个模块，而不是 1 个 ✗）
    /// —— 学生在**几个单元之间来回切**时，每次回头都白付一趟库层（unit08 量级 ≈ 700ms ✗）。
    ///
    /// **T1-A（2026-10-09）**：槽位从"整条库层"细到"**模块边界**"（键 = 前缀的
    /// `lib_key`）—— 两条入口**只共享前几个模块**时，就能从那个边界**续编**
    /// （a3 的 `MainA → MainB`：3 → **2** ✓）。容量因此从
    /// [`MAX_LEAKED_LIB_ARENAS`] 换成 [`MAX_MODULE_CHECKPOINTS`]（**条目**数），
    /// 而**泄漏的 arena** 仍由 [`MAX_LEAKED_LIB_ARENAS`] 封顶 ✓（两个上界各管一头）。
    ///
    /// 为什么是线程局部而不是 `QueryDoc` 的字段：内核环境是 `!Send`（见文件头），
    /// 而 `QueryDoc` 必须 `Send + Sync`（LSP 把它放进 `tokio::spawn` 的 future）✗。
    /// 为什么不是 `static Mutex<...>`：那要求 `Send` ✗（同一个原因）。
    static LIB_CHECKPOINTS: RefCell<Vec<LibCheckpoint<'static>>> = const { RefCell::new(Vec::new()) };
}

/// **把一个检查点放进 LRU 队首**（同键的旧份先丢掉 —— 它已经过期 ✓），并淘汰到容量内 ✓。
///
/// 淘汰只丢**可达性**（arena 早已泄漏、无法回收 ✓）⇒ 不变量：**活着的**检查点
/// ≤ [`MAX_MODULE_CHECKPOINTS`] 份 ✓。
fn push_checkpoint(slots: &mut Vec<LibCheckpoint<'static>>, cp: LibCheckpoint<'static>) {
    slots.retain(|old| old.key != cp.key);
    slots.insert(0, cp);
    slots.truncate(MAX_MODULE_CHECKPOINTS);
}

/// **上界判据的读数**（`#[doc(hidden)]`，判据用）：已经泄漏的库层 arena 数。
#[doc(hidden)]
pub fn lib_checkpoint_arenas_leaked() -> usize {
    LEAKED_LIB_ARENAS.load(Ordering::Relaxed)
}

/// **上界判据的读数**（`#[doc(hidden)]`）：本线程现在有没有活着的检查点。
#[doc(hidden)]
pub fn lib_checkpoint_is_live() -> bool {
    LIB_CHECKPOINTS.with(|slot| !slot.borrow().is_empty())
}

/// **上界判据的读数**（`#[doc(hidden)]`）：本线程那份检查点被复用了多少次。
#[doc(hidden)]
pub fn lib_checkpoint_reuses() -> usize {
    LIB_CHECKPOINTS.with(|slot| slot.borrow().iter().map(|cp| cp.reuses).max().unwrap_or(0))
}

/// **判据用**（`#[doc(hidden)]`）：清掉本线程的检查点与泄漏计数（测试隔离）。
#[doc(hidden)]
pub fn lib_checkpoint_reset() {
    LIB_CHECKPOINTS.with(|slot| slot.borrow_mut().clear());
    LEAKED_LIB_ARENAS.store(0, Ordering::Relaxed);
}

/// **A5c**（2026-10-09）：算"这个库层闭包的产物键" —— 给**上层**判断"要不要投机预热"用 ✓。
///
/// `None` = 库层为空（单文件 ⇒ 没有产物这一层 ✓）。
#[doc(hidden)]
pub fn lib_artifact_key(lib_units: &[SourceUnit<'_>], options: &CompileOptions) -> Option<String> {
    (!lib_units.is_empty()).then(|| lib_key(lib_units, options))
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
    // ⚠ **O(#units) 一次哈希**（热按键每次都要走这里 ✗→✓ 别改成前缀链）：
    // `lib_prefix_keys` 是 O(Σ 前缀字节) 的，只能用在**整条 miss 之后**那条路上
    // （见它的注释 ✓）。两条路共用下面那个"折叠进文本"的助手 ⇒ 文本规则单一 ✓。
    let mut text = String::from(LIB_KEY_TAG);
    for unit in lib_units {
        push_lib_key_text(&mut text, unit);
    }
    crate::compile::cache::key(&text, options)
}

/// 库层键文本的**头**（与 `cache::key` 一起构成"这条链是什么"的身份）。
const LIB_KEY_TAG: &str = "soko.lib-checkpoint/1\0";

/// 把一个单元**折叠进**库层键文本（[`lib_key`] 与 [`lib_prefix_keys`] 的**唯一**实现 ✓）。
fn push_lib_key_text(text: &mut String, unit: &SourceUnit<'_>) {
    text.push_str(unit.name);
    text.push('\0');
    if let Some(path) = unit.path {
        text.push_str(&path.to_string_lossy());
    }
    text.push('\0');
    text.push_str(&unit.file.src);
    text.push('\0');
}

/// **T1-A（2026-10-09）**：**逐模块前缀键** —— `prefix_keys[j] = lib_key(lib_units[0..=j])`。
///
/// 与 [`lib_key`] **同一条链**（同一段文本 + 同一个 `cache::key` ✓）—— `lib_key`
/// 就是本函数的最后一格 ✓（单一真相，别在这里另造哈希 ✗）。
///
/// ⚠ **只在整条键 miss 之后才调用**（调用方负责 ✓）：它是 O(Σ 前缀字节) 的
/// （n 个模块各哈希一次越来越长的文本）⇒ 放进**热按键**那条路就是白白烧 CPU ✗。
/// 整条命中（今天那条路，每次按键 ✓）只花 O(#units) 一次哈希 ✓。
fn lib_prefix_keys(lib_units: &[SourceUnit<'_>], options: &CompileOptions) -> Vec<String> {
    let mut text = String::from(LIB_KEY_TAG);
    let mut keys = Vec::with_capacity(lib_units.len());
    for unit in lib_units {
        push_lib_key_text(&mut text, unit);
        keys.push(crate::compile::cache::key(&text, options));
    }
    keys
}

/// 一组单元里**去掉 `import` 行**之后的命令数（`run_library_pass` 与逐模块趟**共用**
/// 同一条口径 —— 判据 `judge_prefix_offset` 靠它 ✓）。
fn commands_excluding_imports(units: &[SourceUnit<'_>]) -> usize {
    units
        .iter()
        .map(|unit| {
            unit.file
                .commands
                .iter()
                .filter(|command| !matches!(command, crate::ast::Command::Import { .. }))
                .count()
        })
        .sum()
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

/// **T1-B 批 2 的逃生门**：`SOKONANODA_NO_MODULE_ARTIFACTS=1`（或既有的
/// `SOKONANODA_NO_PROJECT_ARTIFACTS=1`）⇒ 模块产物**读写都关**。
///
/// 为什么要有它：产物是"**跨进程加速件**"，一旦怀疑它参与了某个怪现象，用户/agent
/// 必须能**一条环境变量**把它摘掉再复现 —— 与 `compiled/` 那条既有纪律同源 ✓
/// （`SOKONANODA_NO_PROJECT_ARTIFACTS`，见 `AGENTS.md`）。
/// ⚠ 摘掉之后行为 = **今天**（整条库层重编）⇒ 只是慢，不是错 ✓。
pub fn module_artifacts_enabled() -> bool {
    std::env::var_os("SOKONANODA_NO_MODULE_ARTIFACTS").is_none()
        && std::env::var_os("SOKONANODA_NO_PROJECT_ARTIFACTS").is_none()
}

/// **T1-B 批 2**：把一份磁盘产物装成 [`LibCheckpoint`]（`'static` —— 要进线程局部 LRU ✓）。
///
/// ⚠ **只装 arena 不装单元**：产物里没有源文本（它存的是**结果** ✓）⇒ 这条路上
/// **不需要** `leak_lib_units`（省一份泄漏 ✓）。
///
/// 装出来的检查点与"整条趟"那份**逐字段可比**：`key`（产物键 = 库层摘要 ✓）·
/// `shape = None`（整条命中不比形状 —— 键已经把整条源文本折进去了 ✓，与
/// [`LibCheckpoint`] 的既有约定一致 ✓）· `resume = default`（同上 ✓）·
/// `ranges` 由**本次的 units** 现算（它是 units 的纯函数 ✓，不存进产物 ✓）。
fn load_lib_checkpoint(
    text: &str,
    key: &str,
    lib_units: &[SourceUnit<'_>],
) -> Option<LibCheckpoint<'static>> {
    let arena: &'static ArenaRef<'static> =
        Box::leak(Box::new(stumpalo::Arena::new())).as_arena_ref();
    LEAKED_LIB_ARENAS.fetch_add(1, Ordering::Relaxed);
    load_lib_checkpoint_in(arena, text, key, lib_units)
}

/// 装载的**内核**：把产物装进**调用方给的 arena**（寿命由调用方负责 ✓）。
///
/// * `'static` arena（LSP 那条路）⇒ 结果进线程局部 LRU ✓；
/// * **栈上** arena（CLI 那条路）⇒ 结果只活本次调用 ⇒ **零泄漏** ✓。
fn load_lib_checkpoint_in<'a>(
    arena: &'a ArenaRef<'a>,
    text: &str,
    key: &str,
    lib_units: &[SourceUnit<'_>],
) -> Option<LibCheckpoint<'a>> {
    let (env, tables, facts) = crate::project::artifacts::decode_payload(arena, text).ok()?;
    let builder = EnvBuilder::from_export_file(arena, env);
    Some(LibCheckpoint {
        key: key.to_string(),
        shape: None,
        resume: ResumeState::default(),
        builder,
        tables,
        out: facts.out,
        reports: facts.reports,
        ranges: facts.ranges.iter().map(|(a, b)| *a..*b).collect(),
        n_commands: facts.n_commands,
        prefix_commands: facts.prefix_commands,
        reuses: 0,
        // **产物里不存这两张表**（它们是**文本的纯函数** ✓，存了只是把产物撑大 ✗）⇒
        // 装载时按 `lib_units` 现算**一次** ✓（不是每刀 ✓）。
        lib_display: crate::compile::display_notations(lib_units),
        lib_defs: crate::compile::top_level_def_spans_over(lib_units),
        lib_prefix: facts.lib_prefix,
    })
}

/// **T1-B 批 2**：库层趟跑完 ⇒ 写一份产物。**best-effort**（任何一步失败都静默跳过 ✓）：
/// 产物是**加速件**，写不出来只该"下次还慢"，**绝不该**让本次编译失败或改判 ✗。
fn write_lib_artifact(
    root: &std::path::Path,
    key: &str,
    lib: &LibCheckpoint<'_>,
    lib_units: &[SourceUnit<'_>],
    options: &CompileOptions,
) {
    let facts = crate::project::artifacts::LibPassFacts {
        out: lib.out.clone(),
        reports: lib.reports.clone(),
        ranges: crate::compile::unit_ranges(lib_units)
            .into_iter()
            .map(|r| (r.start, r.end))
            .collect(),
        n_commands: lib.n_commands,
        prefix_commands: lib.prefix_commands,
        lib_prefix: lib.lib_prefix.clone(),
    };
    let Ok(text) =
        crate::project::artifacts::encode_payload(&lib.builder.snapshot(), &lib.tables, &facts)
    else {
        return;
    };
    let _ = crate::project::artifacts::write(root, key, &text, options);
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

/// **T1-B 批 2 的公开入口**：与 [`with_project_session_reusing`] 同一条路，
/// 但**多一层磁盘产物**（`<模块根>/.sokonanoda/artifacts/`）。
///
/// 分层的顺序（**这个顺序本身就是判据**）：
/// * **① 线程局部检查点**（同进程、同一刀）—— 最快，先看它 ✓；
/// * **①.5 磁盘产物**（**跨进程**）—— 线程局部 miss 之后才看 ✓；
/// * **② 模块前缀续编** ⇒ **③ 整条重建**（并把产物写出去 ✓）。
///
/// ⇒ 这正是 T4-A 那条契约的实现：**跨按键增量归检查点，跨进程增量归产物** ✓。
pub fn with_project_session_artifacts<R>(
    lib_units: &[SourceUnit<'_>],
    entries: &[Vec<SourceUnit<'_>>],
    options: &CompileOptions,
    root: &std::path::Path,
    on_entry: impl FnMut(
        usize,
        CompileOutput,
        Vec<DocumentReport>,
        &[DocumentReport],
        &[std::ops::Range<usize>],
        std::ops::Range<usize>,
    ) -> R,
) -> Vec<R> {
    with_project_session_reusing_at(lib_units, entries, options, &[], Some(root), on_entry)
}

fn with_project_session_reusing_at<R>(
    lib_units: &[SourceUnit<'_>],
    entries: &[Vec<SourceUnit<'_>>],
    options: &CompileOptions,
    entry_trust: &[Option<EntryTrust>],
    artifacts_root: Option<&std::path::Path>,
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
    // **逃生门**在这里一次收口：关掉之后 `artifacts_root` 变 `None` ⇒ 读写两条路都不走 ✓。
    let artifacts_root = artifacts_root.filter(|_| module_artifacts_enabled());
    LIB_CHECKPOINTS.with(|slots| {
        let mut slots = slots.borrow_mut();
        // ① **复用判据**：摘要逐字相同 + 复用次数未到上界（上界 ②）—— 多槽里找 ✓。
        let hit = if lib_units.is_empty() {
            None
        } else {
            slots
                .iter()
                .position(|cp| cp.key == key && cp.reuses < MAX_REUSES_PER_CHECKPOINT)
        };
        if let Some(at) = hit {
            set_last_lib_source(1); // ① 线程局部检查点
                                    // LRU：用过就提到队首 ✓（**这一提就是本改动的全部收益来源**：换过闭包之后
                                    // 回头再开原入口，原来那一份还在 ⇒ 库层趟不用重付 ✓）。
            let mut cp = slots.remove(at);
            cp.reuses += 1;
            // `&LibCheckpoint<'static>` 按协变缩到本次 units 的寿命 ✓（浅拷贝 ⇒
            // 指针同一 ✓）。
            let out = {
                let lib: &LibCheckpoint<'_> = &cp;
                run_entries(lib, lib_units, entries, options, entry_trust, on_entry)
            };
            slots.insert(0, cp);
            return out;
        }
        // ①.5 **磁盘产物命中**（T1-B 批 2 · 跨进程那层）：线程局部 miss 之后才看它 ✓。
        //
        // ⚠ 三条都与"回退是默认"同一条纪律：产物读不出 / 装载失败 / 泄漏上界用尽
        // ⇒ **什么都不做**，继续往下走（前缀续编 ⇒ 整条重建），**绝不半用** ✓。
        if let Some(root) = artifacts_root {
            if !lib_units.is_empty()
                && LEAKED_LIB_ARENAS.load(Ordering::Relaxed) < MAX_LEAKED_LIB_ARENAS
            {
                if let Some(lib) = crate::project::artifacts::read(root, &key, options)
                    .and_then(|text| load_lib_checkpoint(&text, &key, lib_units))
                {
                    set_last_lib_source(2); // ①.5 磁盘产物
                    let out = run_entries(&lib, lib_units, entries, options, entry_trust, on_entry);
                    // 顺手喂热线程局部（**下一刀就命中 ①** ⇒ 产物只为"冷进程"付一次 ✓）。
                    push_checkpoint(&mut slots, lib);
                    return out;
                }
            }
        }
        // ② **模块级前缀搜索**（T1-A · 只在整条 miss 之后才跑）：
        //    两条入口**只共享前几个模块**时，从那个边界**续编**剩下的模块 ✓
        //    （a3 的 `MainA → MainB`：3 → **2** ✓）。
        //
        //    ⚠ **形状必须比对**（前缀复用独有的口子）：prelude 是按**整条闭包**
        //    判让位的（`prelude_shape`）⇒ 前缀相同、整条不同也可能落在**不同的
        //    prelude 环境**上 ⇒ 不比 = 静默改判 ✗。整条命中**不需要**比（键已经把
        //    整条源文本折进去了 ⇒ 形状是键的纯函数 ✓）。
        let prefix_keys = lib_prefix_keys(lib_units, options);
        let shape = (!lib_units.is_empty()).then(|| prelude_shape(lib_units));
        let resume_at = (0..lib_units.len().saturating_sub(1)).rev().find(|&j| {
            slots.iter().any(|cp| {
                cp.key == prefix_keys[j]
                    && cp.shape.as_ref() == shape.as_ref()
                    && cp.reuses < MAX_REUSES_PER_CHECKPOINT
            })
        });
        if let Some(j) = resume_at {
            let at = slots
                .iter()
                .position(|cp| cp.key == prefix_keys[j] && cp.shape.as_ref() == shape.as_ref())
                .expect("上面刚找到");
            let mut cp = slots.remove(at);
            cp.reuses += 1;
            // **续编**：拿这份检查点当游标（**不泄漏任何新东西** ✓ —— 游标只活
            // 在本次调用里；检查点的 arena 还是它原来那份 ✓）。
            let cursor = LibCursor {
                builder: cp.builder.clone(),
                tables: cp.tables.clone(),
                out: cp.out.clone(),
                reports: cp.reports.clone(),
                n_commands: cp.n_commands,
                prefix_commands: cp.prefix_commands,
                lib_prefix: cp.lib_prefix.clone(),
                resume: Some(cp.resume.clone()),
                next: j + 1,
            };
            // 检查点回队首（它**没有**过期：前缀的文本一个字节没变 ✓）。
            slots.insert(0, cp);
            let (prefixes, total) = crate::compile::closure_prefixes_and_total(lib_units);
            let display = crate::compile::display_notations(lib_units);
            let defs = crate::compile::top_level_def_spans_over(lib_units);
            let (cursor, _made) = run_library_from(
                cursor,
                lib_units,
                options,
                &prefixes,
                &total,
                &prefix_keys,
                shape.as_ref().expect("非空前缀搜索必有形状 ✓"),
                &display,
                &defs,
                // **续编那几趟不建检查点**：它们的单元是**本次调用的**（非 `'static`）
                // ⇒ 存不进线程局部 ✗（建了也白建）。代价如实记：**续编过的库层**
                // 不留"整条"检查点 ⇒ 同一条入口**再开**仍走"前缀续编"（模块数照旧
                // 便宜），只是比"整条命中"多一趟浅前缀的克隆 ✓。
                false,
            );
            let lib = LibCheckpoint {
                key: prefix_keys.last().cloned().unwrap_or_else(|| key.clone()),
                shape: None,
                resume: ResumeState::default(),
                builder: cursor.builder,
                tables: cursor.tables,
                out: cursor.out,
                reports: cursor.reports,
                ranges: crate::compile::unit_ranges(lib_units),
                n_commands: cursor.n_commands,
                prefix_commands: cursor.prefix_commands,
                reuses: 0,
                lib_display: crate::compile::display_notations(lib_units),
                lib_defs: crate::compile::top_level_def_spans_over(lib_units),
                lib_prefix: cursor.lib_prefix,
            };
            set_last_lib_source(3); // ② 前缀续编
            return run_entries(&lib, lib_units, entries, options, entry_trust, on_entry);
        }
        // ③ **重建**：库层趟跑在一份**泄漏的** arena 上（`Box::leak` = 零 `unsafe`
        //    的 `'static` 来源 ✓）。库层为空（单文件）或泄漏上界用尽（上界 ①）
        //    ⇒ 不建检查点，走栈上 arena（= 今天那条路，逐字节相同）✓。
        let can_leak = !lib_units.is_empty()
            && LEAKED_LIB_ARENAS.load(Ordering::Relaxed) < MAX_LEAKED_LIB_ARENAS;
        if can_leak {
            set_last_lib_source(4); // ③ 整条重建
            let arena: &'static ArenaRef<'static> =
                Box::leak(Box::new(stumpalo::Arena::new())).as_arena_ref();
            LEAKED_LIB_ARENAS.fetch_add(1, Ordering::Relaxed);
            // 库层趟跑在**泄漏的单元副本**上（`run_pass_with` 要求 units 与 arena
            // 同寿命 ⇒ 要存成 `'static` 检查点就得让 units 也是 `'static` ✓）。
            let units = leak_lib_units(lib_units);
            let mut builder = EnvBuilder::new(arena, Config::default());
            let mut tables = PassTables::new();
            // **prelude 按整条库层装一次**：与今天**同一个函数**、**同一份上下文**
            // （`units` = 整条库层 ✓）——逐模块趟传 `install_preludes = false`
            // （否则第二个模块起会重复装 ⇒ 实测 `duplicate declaration Nat` ✗）。
            install_all_preludes(
                &mut builder,
                &mut tables.known,
                &mut tables.inductives,
                &mut tables.defs,
                units,
                options,
            );
            let (prefixes, total) = crate::compile::closure_prefixes_and_total(units);
            let prefix_keys = lib_prefix_keys(units, options);
            // **形状算一次**（不是每个模块各算一遍 —— 那是 O(n × 闭包) 白跑 ✗）。
            let shape = prelude_shape(units);
            let display = crate::compile::display_notations(units);
            let defs = crate::compile::top_level_def_spans_over(units);
            let cursor = LibCursor {
                builder,
                tables,
                out: CompileOutput::default(),
                reports: Vec::new(),
                n_commands: 0,
                prefix_commands: 0,
                lib_prefix: String::new(),
                resume: None,
                next: 0,
            };
            let (_cursor, mut made) = run_library_from(
                cursor,
                units,
                options,
                &prefixes,
                &total,
                &prefix_keys,
                &shape,
                &display,
                &defs,
                true,
            );
            // `made` 按模块序（浅 → 深）⇒ **最后一份 = 整条库层** ✓（与今天那份同键 ✓）。
            let lib = made
                .pop()
                .expect("库层至少一个模块：上面 `lib_units` 非空 ✓");
            // **T1-B 批 2**：把整条库层**写出去**（best-effort —— 写不出来绝不影响本次编译 ✓）。
            if let Some(root) = artifacts_root {
                write_lib_artifact(root, &key, &lib, lib_units, options);
            }
            let out = run_entries(&lib, lib_units, entries, options, entry_trust, on_entry);
            // 入 LRU：先前缀（浅 → 深），再**整条**（队首 = 最近用过 ✓）。
            for checkpoint in made {
                push_checkpoint(&mut slots, checkpoint);
            }
            push_checkpoint(&mut slots, lib);
            out
        } else {
            slots.clear();
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
/// ⇒ 本线程的 [`LIB_CHECKPOINTS`] 是**冷的** ⇒ **开档后的第一次编辑**要走
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
    LIB_CHECKPOINTS.with(|slots| {
        let mut slots = slots.borrow_mut();
        // 已经热的（**任何一槽**键逐字相同）⇒ 别重复做（否则每开一次档白烧一趟库层 ✗）。
        if slots.iter().any(|cp| cp.key == key) {
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
        // 入 LRU 队首（与 miss 分支同一条不变量 ✓）。
        push_checkpoint(&mut slots, lib);
        true
    })
}

/// **T1-B 批 2**：LSP 那条路（`reuse_library = true`）的入口。
///
/// ⚠ **它就是"原来那个 `with_project_session_reusing` + 产物"**：原来的无产物口子
/// 已被本函数取代 —— 逃生门（`module_artifacts_enabled()`）关掉之后，
/// `artifacts_root` 变 `None` ⇒ **逐字节回到原来那条路** ✓（所以不必留两份实现 ✓）。
pub(crate) fn with_project_session_reusing_artifacts<R>(
    lib_units: &[SourceUnit<'_>],
    entries: &[Vec<SourceUnit<'_>>],
    options: &CompileOptions,
    entry_trust: &[Option<EntryTrust>],
    root: &std::path::Path,
    on_entry: impl FnMut(
        usize,
        CompileOutput,
        Vec<DocumentReport>,
        &[DocumentReport],
        &[std::ops::Range<usize>],
        std::ops::Range<usize>,
    ) -> R,
) -> Vec<R> {
    with_project_session_reusing_at(
        lib_units,
        entries,
        options,
        entry_trust,
        Some(root),
        on_entry,
    )
}

/// **T1-B 批 2**：`with_project_session_trusted` 的**带产物**口子（**CLI 那条路** ✓）。
///
/// ## 为什么它与 reusing 那条**不同**（本函数存在的唯一理由）
///
/// CLI 是**短命进程**：线程局部检查点"只有代价没有收益"（T4-A 的契约 ✓）。
/// ⇒ 这条口子**不碰 `LIB_CHECKPOINTS`**，装载出来的检查点**只活这一次调用** ⇒
/// **arena 建在栈上**（不需要 `'static`）⇒ **零泄漏** ✓
/// （`lib_checkpoint_arenas_leaked()` 保持 **0** ⇒ T4-A 的守卫原样绿 ✓）。
///
/// 于是产物与检查点各归其位：**跨进程增量归产物（CLI ✓）· 跨按键增量归检查点（LSP ✓）**
/// —— 正好是 T4-A 那条契约的字面意思 ✓。
///
/// **逃生门**：`module_artifacts_enabled()` 为假 ⇒ 本函数**逐字节等价于**
/// [`with_project_session_trusted`]（连 `key` 都传 `String::new()` ✓）。
pub(crate) fn with_project_session_artifacts_trusted<R>(
    lib_units: &[SourceUnit<'_>],
    entries: &[Vec<SourceUnit<'_>>],
    options: &CompileOptions,
    root: &std::path::Path,
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
    let arena = stumpalo::Arena::new();
    let on_artifacts = module_artifacts_enabled() && !lib_units.is_empty();
    let key = on_artifacts.then(|| lib_key(lib_units, options));
    // ① **产物命中**：栈上 arena ⇒ 装载出来的检查点只活这一次调用 ✓（零泄漏 ✓）。
    if let (true, Some(key)) = (on_artifacts, key.as_ref()) {
        if let Some(lib) = crate::project::artifacts::read(root, key, options)
            .and_then(|text| load_lib_checkpoint_in(arena.as_arena_ref(), &text, key, lib_units))
        {
            return run_entries(&lib, lib_units, entries, options, entry_trust, on_entry);
        }
    }
    // ② 未命中：照今天那条路跑库层趟（栈上 arena、`key` 传空 —— 与
    //    `with_project_session_trusted` **逐字相同** ✓）⇒ 跑完把产物写出去。
    let builder = EnvBuilder::new(arena.as_arena_ref(), Config::default());
    let lib = run_library_pass(
        builder,
        PassTables::new(),
        lib_units,
        options,
        String::new(),
    );
    if let Some(key) = key.as_ref() {
        write_lib_artifact(root, key, &lib, lib_units, options);
    }
    run_entries(&lib, lib_units, entries, options, entry_trust, on_entry)
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
    let (lib_pass, builder, tables, lib_state, _tail) = run_pass_with(
        builder, None, true, tables, lib_units, options, true, None, None, None, None, None, None,
        // 建议材料：库层趟按 `lib_units` 自己算 ✓。
        None,
        // 库层趟：judge 的前缀（`closure_prefixes_for(lib_units)`）与本趟 `idx`
        // **同坐标系** ✓ ⇒ 不平移。
        0,
        // **T4-B（第 74 轮）**：`snapshot_state` 从 `false` 改成 **`true`** ✓ ——
        // 库层趟的**累加状态**里带着 `exports`（`export` 是唯一跨 `import` 的可见性通道 ✓），
        // 而入口趟要靠它才看得到库层的 `export` ✓（判据 `t4b_plan_parity` 的 export 那条 ✓）。
        // 以前 `false` ⇒ 状态是 `None` ⇒ 入口趟拿到空状态 ⇒ 导入方报 `unknown identifier` ✗。
        true, None, None, false, false,
    );
    // **G-29 第 3 棒**：入口趟的 `idx` 是**入口空间**的，而 judge 的合成前缀是
    // **整条闭包** ⇒ 压栈的担保必须平移"**库层那一段的命令数**" ✓，否则
    // `synthesized_trust` 的闸门 `before >= prefix_commands` 恒不成立 ⇒ 入口趟
    // 每次 `judge_infer` 未命中都整份重编 ✗（实测 7 次 · 见设计 §33）。
    //
    // ⚠ 数的是**去掉 `import` 行之后**的命令数（`importless_source` 会剥掉它们，
    // 见 `closure_prefixes_for` ✓）—— 多算只会被 `before.min(prefix_commands)`
    // 夹回（更保守 ✓），**少算才会漏担保** ✗ ⇒ 必须按同一口径数 ✓。
    let prefix_commands: usize = commands_excluding_imports(lib_units);
    // 读在 `lib_pass.report` 被搬走**之前**（`split_report` 会吃掉它）。
    let checks = lib_pass.kernel_checks();
    let ranges = crate::compile::unit_ranges(lib_units);
    // **A4a**：库层那一段的闭包前缀（与入口趟**同一套累加规则** ✓）。
    let lib_prefix = crate::compile::closure_accumulated_over(lib_units);
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
        // **T1-A**：整条一趟那条路**不参与前缀复用** ⇒ `shape = None`
        // （整条命中不需要它：形状是键的纯函数 ✓；`Some` 会让 `prelude_shape`
        // 的 O(闭包) 扫描白跑一趟 —— CLI `build`/`check` 对性能敏感 ✗）。
        shape: None,
        // ⭐ **T4-B（第 74 轮）**：`ResumeState` 里带着 **`exports`** —— 而 `export` 是**唯一**
        // 跨 `import` 的可见性通道（设计 §N7）⇒ 以前恒给 `default()` ⇒ **入口趟看不到库层的
        // `export`** ✗（判据 `t4b_plan_parity::the_export_dimension_is_a_known_divergence_today`
        // 把它钉住了 ✓）。⇒ 收**真状态** ✓。
        resume: lib_state.unwrap_or_default(),
        builder,
        tables,
        out,
        reports,
        ranges,
        n_commands: lib_pass.n_commands,
        prefix_commands,
        reuses: 0,
        lib_display: crate::compile::display_notations(lib_units),
        lib_defs: crate::compile::top_level_def_spans_over(lib_units),
        lib_prefix,
    }
}

/// **T1-A（2026-10-09）**：**逐模块**跑库层趟 —— 每编完一个模块产出一份检查点，
/// 返回 `(最终游标, 按模块序的检查点)`。
///
/// ## 为什么必须逐模块（而不是"一趟 + 中途快照"）
///
/// `walk` **无条件** `add_declar`（到当前命令为止的声明**尚未过内核检查**），
/// 而 `finish_pass` 是**整趟一次**做的 ⇒ 想在"编完第 j 个模块"处得到一份
/// **合法的**环境，就只能让那一趟**在那里结束** ✓（`snapshot_state` 拿到的
/// `ResumeState` 正好是"下一趟从这儿接着跑"所需的三样累加器 ✓）。
///
/// ## 逐字节等价（每一格都是**现成覆盖入口**，不新造语义）
///
/// | 本趟要什么 | 怎么给 | 对得上今天吗 |
/// |---|---|---|
/// | 建议材料（累计） | `template_closure = &units[..=j]` | ✓（单单元趟 + 覆盖 = 整条一趟里的第 j 格） |
/// | judge 前缀 | `closure_prefixes_override = &[prefixes[j]]` | ✓（`prefixes[j]` = 同一套累加规则 ✓） |
/// | 记法表 / 定义 span 表 | `display_override` / `defs_override` = **整条** | ✓（今天也是整条闭包算的 ✓） |
/// | 命令坐标系平移 | `judge_prefix_offset` = 前缀的命令数（去 `import`） | ✓（今天库里第 j 个模块的 `idx` 就在那个坐标 ✓） |
/// | 跨单元累加器 | `resume`（`closure_id`/`exports`/`example_idx`） | ✓（`ResumeState` 的字段注释 ✓） |
///
/// `store` = `true` 时把每份检查点也建出来（**冷建**那条路：单元是泄漏的
/// `'static` ⇒ 检查点能进线程局部 ✓）；`false` 时只跑不建（**续编**那条路：
/// 单元是本次调用的 ⇒ 建了也存不进 ✗）。
#[allow(clippy::too_many_arguments)]
fn run_library_from<'a>(
    mut cursor: LibCursor<'a>,
    lib_units: &'a [SourceUnit<'a>],
    options: &CompileOptions,
    prefixes: &[String],
    total: &str,
    prefix_keys: &[String],
    shape: &crate::compile::PreludeShape,
    display: &DisplayNotations,
    defs: &std::collections::HashMap<String, crate::Span>,
    store: bool,
) -> (LibCursor<'a>, Vec<LibCheckpoint<'a>>) {
    let n = lib_units.len();
    let mut made: Vec<LibCheckpoint<'a>> = Vec::new();
    while cursor.next < n {
        let j = cursor.next;
        let units_j: &'a [SourceUnit<'a>] = &lib_units[j..=j];
        let prefix_j: [String; 1] = [prefixes[j].clone()];
        let (pass, builder, tables, state, _tail) = run_pass_with(
            cursor.builder,
            None,
            // prelude **只在调用方那一步装一次**（见调用点的注释）✓。
            false,
            cursor.tables,
            units_j,
            options,
            true,
            None,
            None,
            None,
            Some(&prefix_j),
            Some(display),
            Some(defs),
            Some(&lib_units[..=j]),
            cursor.prefix_commands,
            // 本趟结束 = **模块边界** ⇒ 要那份续编状态 ✓。
            true,
            cursor.resume.take(),
            None,
            false,
            false,
        );
        let n_cmds = pass.n_commands;
        let checks = pass.kernel_checks();
        let mut out = pass.out;
        // 报告要**本趟坐标系**的命令号（`split_report` 会按它归因 ✓）⇒ 先切、后偏移 ✓。
        let reports = split_report(pass.report, &out.error_cmds, &out.warning_cmds, units_j);
        let offset = cursor.n_commands;
        for cmd in out.event_cmds.iter_mut() {
            *cmd += offset;
        }
        for cmd in out.error_cmds.iter_mut() {
            *cmd += offset;
        }
        for cmd in out.warning_cmds.iter_mut() {
            *cmd += offset;
        }
        out.stats.kernel_checks = checks;
        // 合并进游标（与 `run_entries` 合入口输出的手法同一条 ✓）。
        cursor.out.events.extend(out.events);
        cursor.out.event_cmds.extend(out.event_cmds);
        cursor.out.errors.extend(out.errors);
        cursor.out.error_cmds.extend(out.error_cmds);
        cursor.out.warnings.extend(out.warnings);
        cursor.out.warning_cmds.extend(out.warning_cmds);
        cursor.out.stats.kernel_checks += checks;
        cursor.reports.extend(reports);
        cursor.n_commands += n_cmds;
        cursor.prefix_commands += commands_excluding_imports(units_j);
        cursor.lib_prefix = if j + 1 < n {
            prefixes[j + 1].clone()
        } else {
            total.to_string()
        };
        cursor.builder = builder;
        cursor.tables = tables;
        cursor.next = j + 1;
        cursor.resume = state;
        if store {
            made.push(LibCheckpoint {
                key: prefix_keys[j].clone(),
                // **T1-A**：前缀复用要的形状判据（**整条库层**的 `prelude_shape` ✓）
                // —— 调用方**算好传进来**（在循环里算 = O(n × 闭包) 白跑 ✗）。
                shape: Some(shape.clone()),
                resume: cursor.resume.clone().unwrap_or_default(),
                builder: cursor.builder.clone(),
                tables: cursor.tables.clone(),
                out: cursor.out.clone(),
                reports: cursor.reports.clone(),
                ranges: crate::compile::unit_ranges(&lib_units[..=j]),
                n_commands: cursor.n_commands,
                prefix_commands: cursor.prefix_commands,
                reuses: 0,
                // ⚠ 这是**前缀**（`lib_units[..=j]`）那一段的表 ⇒ 按前缀算 ✓
                // （每模块一次、只在建检查点时 ✓ —— 不是每刀 ✓）。
                lib_display: crate::compile::display_notations(&lib_units[..=j]),
                lib_defs: crate::compile::top_level_def_spans_over(&lib_units[..=j]),
                lib_prefix: cursor.lib_prefix.clone(),
            });
        }
    }
    (cursor, made)
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
        let mut builder = lib.builder.clone();
        // **T2-A（2026-10-09）**：库层**封层** —— 之后入口的插入走**本地层**
        // （`DeclarMap` 的 `local`），共享的库层只读 ✓。不封层的话，共享的 `base`
        // 会在第一次插入时被 `Arc::make_mut` **整份复制**（O(#decls)）✗
        // ⇒ 判据①当场退回 O(#decls)（反向验证见 `docs/design/persistent-declarations.md` §4）。
        // 语义零变化：封层只决定"新声明插进哪一层"，**取用顺序仍是插入序** ✓。
        builder.seal_library_layer();
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
        // **A4a（2026-10-08）**：入口趟要的前缀 = **库层全部**那一段 ——
        // 它是检查点里**存好的**（`lib.lib_prefix`，`lib_key` 的纯函数 ✓）⇒
        // 每一刀只克隆一次，**不再重跑 O(闭包) 的累加** ✓。
        // ⚠ **逐字节等价**：旧写法 `closure_prefixes_for(&entry_closure).last()`
        // 拿的就是"库层全部单元拼接之后"那一份（入口是 `entry_closure` 的最后一格
        // ⇒ 最后一格 = units[0..n-1] = 全部库层 ✓）；`lib_prefix` 用**同一套累加规则**
        // （`closure_accumulated_over` 与 `closure_prefixes_for` 共用一个实现 ✓）。
        // 库层为空（单文件）⇒ 保持旧路（那时 `closure_prefixes_for` 返回**空 Vec** ✓）。
        let entry_prefixes: Vec<String> = if lib_units.is_empty() {
            crate::compile::closure_prefixes_for(&entry_closure)
                .last()
                .map(|last| vec![last.clone()])
                .unwrap_or_default()
        } else {
            vec![lib.lib_prefix.clone()]
        };
        // **T2-B0**：库层那一段**随检查点存好了** ⇒ 每刀只建"**入口那一段**"再合并 ✓
        // （合并与"一次性建表"逐位相同 —— 判据 `t2b0_display_merge_parity` ✓）。
        let entry_display = if std::env::var_os("SOKO_T2B0_NO_SPLIT").is_some() {
            // **反向验证的逃生门**（默认关 ⇒ 生产零影响 ✓）：退回"按整条闭包建表"那条老路
            // ⇒ 每刀处理的**单元数**从 1 回到 O(闭包) ⇒ `t2b0_table_rebuilds` 当场判红 ✓。
            crate::compile::display_notations(&entry_closure)
        } else {
            lib.lib_display
                .merged_with(&crate::compile::display_notations(entry_units))
        };
        // **跨模块 hover 回填**（切片 1b 的入口趟）：`resolution` 要指向**库层**声明
        // 的真实 span，而入口趟的 `units` 只有入口 ⇒ 不传这张表的话，入口里
        // `Point`（来自 `import Lib`）的 hover `resolution` 会退化成 `None`
        // ⇒ F12/高亮在跨模块名字上失效 ✗（实测：与会话外整份编译的报告因此不同）。
        // **T2-B0**：同 `entry_display` —— 库层先、入口后（与一次性建表同序 ✓）。
        let entry_defs = {
            let mut defs = lib.lib_defs.clone();
            for (name, span) in crate::compile::top_level_def_spans_over(entry_units) {
                defs.entry(name).or_insert(span);
            }
            defs
        };
        // **S2 步 2**：该入口这一趟的信任前缀（缺省 = 整份重查，与今天逐字节相同）。
        let trusted = entry_trust.get(index).and_then(|slot| slot.as_ref());
        let (pass, _next, _next_tables, _state, _walk_tail) = run_pass_with(
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
            // **T1-A**：入口趟**不做逐模块检查点** ⇒ 这一格仍 `false` ✓。
            false,
            // ⭐ **但累加状态要接上**（T4-B · 第 74 轮）：`exports` 在上头，以前给 `None`
            // ⇒ 库层的 `export` 到不了入口 ✗（见检查点里那条注释 ✓）。
            Some(lib.resume.clone()),
            // **T2-B**：`resume_walk`/`snapshot_walk` 仍关（**接线待下一刀**：
            // 报告侧累加器与 `EntryCache` 前缀拼接**各算一份** ⇒ 报告里每条声明
            // 出现两次 ✗ —— 守卫 `t2b_resumed_report_has_no_duplicate_declarations`
            // 逮到了它 ⇒ **不许带着它落地** ✗）；`count_entry_commands = true` ✓
            // ⇒ 判据读数（"改最后一条 ⇒ 1"）先量着、机制已就绪。
            None,
            false,
            true,
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

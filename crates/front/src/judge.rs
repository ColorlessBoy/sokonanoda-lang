//! kernel 判定的术语匹配（I9 goal 视图深化）。
//!
//! 设计（docs/design/i8-i9.md §2）：不发明第二套判定逻辑——把"候选术语 +
//! 已写 binders"合成一条**完整的声明**（`def _soko_judge_k : <声明类型> :=
//! fun <binders> => <术语>`），交给标准流水线（含 prelude 决策与
//! check-then-add 语义），由完整 kernel 当裁判：
//!
//! - 通过 → [`Judgement::Match`]；
//! - 内核拒绝且带 `def_eq mismatch expected/actual` → [`Judgement::Mismatch`]
//!   （"期望 X / 实际 Y"直接来自内核，呼应 Lean `exact?` 的教训：无效建议
//!   根本不该出现）；
//! - elaborate 失败 → [`Judgement::Error`]（稳定错误码 + 教学提示）。
//!
//! 注意：prelude 决策扫描的是"前缀 + 合成声明"，看不到文档后缀。若用户在
//! 目标声明之后才定义自己的 `Eq`/`Nat`（遮蔽 prelude），判定环境与文档环境
//! 可能有差别——教学文档（练习先于解答）不会出现这种形态。
//!
//! [`judge_value_replace`] 服务失败声明（kernel 拒绝、没有洞）：候选整体
//! 替换 `:=` 之后的值位，同名机制合成判定声明。失败声明的针对性建议
//! （`suggest` 的 Eq 形状 rfl 替换）用它做 kernel 终审。
//!
//! 判定永远走 kernel，不做文本比对（REQUIREMENTS §2.8）。

use crate::compile::{
    check_document_with, compile_fol_with, run_incremental, CheckEvent, CompileError,
    CompileOptions, CompileOutput, DeclStatus, DocumentReport, ErrorKind, KernelFailed, TrustPlan,
};
use crate::proof::{parse_expr_text, render_expr};
use crate::span::Pos;
use crate::{tokenize, Binder, BinderKind, Command, Expr, FolFile, Span, Token, TokenKind};

/// 一个开放练习的判定规格：**剩余目标**（与 `DeclState.goal` /
/// `ProofState::goal_text` 同语义）、声明的宇宙参数、已写 binders
/// （名字 + 类型文本；类型必须可解析，`DeclState.binders` 恒有类型——
/// 未写时借用声明层）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenGoalSpec {
    pub universe: Vec<String>,
    pub ty: String,
    pub binders: Vec<GoalBinderSpec>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoalBinderSpec {
    pub name: String,
    pub ty: Option<String>,
}

impl OpenGoalSpec {
    /// 从 `#prove` 会话状态构造（REPL 用）。
    pub fn from_goal(universe: Vec<String>, ty: String, binders: Vec<GoalBinderSpec>) -> Self {
        Self {
            universe,
            ty,
            binders,
        }
    }
}

/// 一次 kernel 判定的结论。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Judgement {
    /// 术语的类型与剩余目标 definitional equal（kernel 判定通过）。
    Match,
    /// kernel 拒绝：期望类型与实际类型（两者都来自内核渲染）。
    ///
    /// `kind` = 内核对**这次拒绝**的分类（`refine_kernel_kind` 的产物，G-21）。
    /// 为什么要它：`expected`/`actual` 只是两段文本，而**根因**在分类里 ——
    /// 「期望 `Sort(n)`，实际是裸绑元 `$k`」= 一个项落在了类型位上，声明位那条
    /// 诊断（`error.rs::classify_term_in_type_position`）早就把它归到
    /// [`ErrorKind::KernelExpectedSort`]、由 hint 点名「点名调用漏了前导类型参数」；
    /// `by` 路径此前把分类丢掉、只把 actual 换成 `judge_infer` 的类型文本
    /// ⇒ 学习者看到「期望 `A a`，实际是 `mymem a A`」这种**同形**对照 ✗。
    ///
    /// ⚠ **判定控制流不看它** ✓（`Match`/`Mismatch`/`Error` 三分法的用法一个字
    /// 不动）：它只服务**报错文案**，谁用谁不用由消费方（`by.rs`）决定。
    Mismatch {
        expected: String,
        actual: String,
        kind: ErrorKind,
    },
    /// 术语无法 elaborate（错误码 + 消息，教学提示同诊断管线）。
    Error { code: String, message: String },
}

/// 对开放声明 `open` 逐个判定 `terms` 是否能填进洞里。
/// 返回值与 `terms` 等长、按序对应；每次调用独立跑一遍前缀流水线。
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{Mutex, OnceLock};

/// 判定结果缓存（I13 性能收口）：judge_* 的每次调用都要**重编译整个文档
/// 前缀**（拼合成 `#check` 声明后走完整 compile）——by 块 tactic `apply`/
/// `exact` 让这个 O(前缀) 成本落在每一次按键上。缓存按请求指纹命中，
/// 容量封顶（防内存膨胀）；前缀文本参与指纹，文档任何更早的编辑都会
/// 失效缓存——**保守但正确**。
// 判定缓存容量。**128 是 R2 实测的灾难值**，不是保守值：判定的前缀重编译会
// **递归**触发更早声明的 `by` 块判定（前缀里就有那些 `by`），而 FIFO 128 条
// 一被挤爆，缓存就再也接不住这次递归 ⇒ 成本随声明数**指数**增长
// （实测：单元④ 解答 6 条声明 8.9s、第 7 条 → >60s；整份 >600s 不返回）。
// 课程 Lean 化之前每份文件只有个位数判定，128 够用；tactic 风格之后一份文件
// 轻松上百次判定 ⇒ 把容量提到与"一次判卷的全部判定数"同量级。
const JUDGE_CACHE_CAP: usize = 4096;

/// 判定缓存的值：`Infer` = judge_infer 的类型文本（Ok/Err 都缓存），
/// `Terms` = judge_terms / judge_hole_fill 的结论序列。
#[derive(Clone)]
enum JudgeCacheValue {
    Infer(Result<String, Judgement>),
    Terms(Vec<Judgement>),
}

/// 缓存存储：指纹 → 结论；`Vec` 记录插入序（FIFO 淘汰）。
type JudgeCacheStore = (HashMap<u64, JudgeCacheValue>, Vec<u64>);

fn judge_cache() -> &'static Mutex<JudgeCacheStore> {
    static CACHE: OnceLock<Mutex<JudgeCacheStore>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new((HashMap::new(), Vec::new())))
}

fn judge_cache_key(parts: &[&str]) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    for part in parts {
        part.hash(&mut h);
        0u8.hash(&mut h); // 分隔符，避免拼接歧义
    }
    h.finish()
}

fn judge_cache_get(key: u64) -> Option<JudgeCacheValue> {
    judge_cache()
        .lock()
        .expect("judge cache")
        .0
        .get(&key)
        .cloned()
}

/// **类型查询**（[`judge_type_of`]）的独立缓存。
///
/// 为什么不与 `judge_infer` 共用：两者是**不同的查询**（一个问"项的类型"、
/// 一个问"项在 binder 语境下的类型"），共用一张 FIFO 会让记法展开的类型查询
/// 把 tactic 判定的条目挤出去（实测：`judge_cache_returns_identical_results_and_
/// stores_entries` 在全量跑里被挤到容量上限而判红）。
fn type_cache() -> &'static Mutex<JudgeCacheStore> {
    static CACHE: OnceLock<Mutex<JudgeCacheStore>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new((HashMap::new(), Vec::new())))
}

fn type_cache_get(key: u64) -> Option<JudgeCacheValue> {
    type_cache()
        .lock()
        .expect("type cache")
        .0
        .get(&key)
        .cloned()
}

fn type_cache_put(key: u64, value: JudgeCacheValue) {
    let mut cache = type_cache().lock().expect("type cache");
    if cache.0.insert(key, value).is_none() {
        cache.1.push(key);
        while cache.1.len() > JUDGE_CACHE_CAP {
            let oldest = cache.1.remove(0);
            cache.0.remove(&oldest);
            // **闸类出口**（G-91 乙类 ✓）：`JUDGE_CACHE_CAP` 挤掉一条 ⇒ 它下次
            // 必然 miss ⇒ 重跑整份前缀 ✓ —— **只变慢** ✓（重算结论逐字节相同 ✓），
            // 但**必须看得见** ✗（`hits/misses` 分不出"新键"与"被挤掉" ✓）。
            sokonanoda::gates::JUDGE_CACHE_EVICTED.bump();
        }
    }
}

fn judge_cache_put(key: u64, value: JudgeCacheValue) {
    let mut cache = judge_cache().lock().expect("judge cache");
    if cache.0.insert(key, value.clone()).is_none() {
        cache.1.push(key);
        while cache.1.len() > JUDGE_CACHE_CAP {
            let oldest = cache.1.remove(0);
            cache.0.remove(&oldest);
            // 同上（两张表共用 `JUDGE_CACHE_CAP` 与同一套 FIFO ⇒ 共用一个出口 ✓）。
            sokonanoda::gates::JUDGE_CACHE_EVICTED.bump();
        }
    }
}

#[cfg(test)]
pub(crate) fn judge_cache_len() -> usize {
    judge_cache().lock().expect("judge cache").0.len()
}

/// 某个键**在不在**缓存里（第三刀：容量是 FIFO 的 `JUDGE_CACHE_CAP`，用
/// "长度变大了"当"键进去了"的判据会被别的测试挤爆——直接问键，与容量无关）。
#[cfg(test)]
pub(crate) fn judge_cache_contains(key: u64) -> bool {
    judge_cache()
        .lock()
        .expect("judge cache")
        .0
        .contains_key(&key)
}

fn options_key(options: &CompileOptions) -> String {
    format!("{:?}", options.prelude)
}

pub fn judge_terms(
    prefix_src: &str,
    options: &CompileOptions,
    open: &OpenGoalSpec,
    terms: &[&str],
) -> Vec<Judgement> {
    // 乐观批次活跃（`by` 块正在跑）：只记录，返回 `Match`。批次由
    // [`flush_batch`] 一次判完；有一条不是 `Match`，`run_by` 就严格重跑。
    // 记录到的都是**同一个 `by` 块**里的判定，前缀相同 ⇒ 一次文档走查问完。
    if batching_on() {
        if let Some(recorded) = record_in_batch(prefix_src, options, open, terms) {
            return recorded;
        }
    }
    judge_terms_with("", prefix_src, options, open, terms)
}

/// 批次活跃时把这一问记下来，并返回乐观结论（全 `Match`）；没有批次返回 `None`。
fn record_in_batch(
    prefix_src: &str,
    options: &CompileOptions,
    open: &OpenGoalSpec,
    terms: &[&str],
) -> Option<Vec<Judgement>> {
    BATCH.with(|b| {
        let mut slot = b.borrow_mut();
        let items = slot.as_mut()?;
        for term in terms {
            items.push(BatchItem {
                extra_prefix: String::new(),
                prefix_src: prefix_src.to_string(),
                options: *options,
                spec: open.clone(),
                term: (*term).to_string(),
            });
        }
        Some(vec![Judgement::Match; terms.len()])
    })
}

/// 同 [`judge_terms`]，但把 `extra_prefix`（闭包上下文：被导入模块的声明文本）
/// 拼在文档前缀之前——项目模式下 quick-fix 才看得见导入的名字。
pub fn judge_terms_with(
    extra_prefix: &str,
    prefix_src: &str,
    options: &CompileOptions,
    open: &OpenGoalSpec,
    terms: &[&str],
) -> Vec<Judgement> {
    // **测量开关**（`SOKO_NO_JUDGE=1`）：跳过真正的判定、一律答"过"。
    //
    // ⚠ **只许用来量成本，绝不许进判定路径** ✗ —— 它会**改判定**（本来该报错的
    // 现在不报），所以它永远不能默认开、也不能用来"让构建变快"。
    // 它的用途只有一个：回答"**judge 环节占 build 多少**"（用户 09:22 的问题 2）
    // ⇒ 真课程实测：`218.8s → 见 docs/perf/course-profile-2026-09-29.md`。
    // 它**不写缓存**（否则污染后续真实判定）✓。
    if no_judge() {
        return terms.iter().map(|_| Judgement::Match).collect::<Vec<_>>();
    }
    let key = judge_cache_key(&[
        extra_prefix,
        prefix_src,
        &options_key(options),
        &format!("{open:?}"),
        &format!("{terms:?}"),
    ]);
    if let Some(JudgeCacheValue::Terms(j)) = judge_cache_get(key) {
        stats::HITS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        return j;
    }
    stats::MISSES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let pairs: Vec<JudgePair> = terms
        .iter()
        .map(|term| JudgePair {
            spec: open.clone(),
            term: (*term).to_string(),
        })
        .collect();
    let j = judge_pairs_uncached(key, extra_prefix, prefix_src, options, &pairs);
    judge_cache_put(key, JudgeCacheValue::Terms(j.clone()));
    j
}

/// 一「对」判定请求：**一个目标规格 + 一条候选术语文本**。
///
/// 为什么要有它（0.62.0 性能）：`by` 块的**每一步** tactic 都会问一次判定
/// （`have` / `exact` / `rfl` 各算一次），而每次判定都要**重跑整份文档前缀**
/// （O(前缀)）。一份 tactic 风格解答轻松上百次判定 ⇒ 整份文件退化成
/// O(前缀 × 步数)。把「一个 `by` 块里的全部判定」合成**一份文档**一次问完，
/// 前缀就只走一遍——见 [`begin_batch`] / [`flush_batch`]。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JudgePair {
    pub spec: OpenGoalSpec,
    pub term: String,
}

/// 一批判定一次问完（[`judge_terms_with`] 是它的单规格特例）。
///
/// **判定语义与逐条调用逐字相同**：合成声明的名字仍是 `_soko_judge_{k}`（`k` 是
/// 这一批里的序号），每条各自带自己的目标类型与 binder 折叠，交给**同一份**
/// 文档流水线（含 prelude 决策、check-then-add、完整 kernel）。唯一的差别是
/// "一个前缀走一遍"而不是"每条走一遍"。
pub fn judge_pairs_with(
    extra_prefix: &str,
    prefix_src: &str,
    options: &CompileOptions,
    pairs: &[JudgePair],
) -> Vec<Judgement> {
    let key = judge_cache_key(&[
        extra_prefix,
        prefix_src,
        &options_key(options),
        &format!("{pairs:?}"),
    ]);
    if let Some(JudgeCacheValue::Terms(j)) = judge_cache_get(key) {
        stats::HITS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        return j;
    }
    stats::MISSES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let j = judge_pairs_uncached(key, extra_prefix, prefix_src, options, pairs);
    judge_cache_put(key, JudgeCacheValue::Terms(j.clone()));
    j
}

/// 判定阶段的分阶段计数（T-K03）。
///
/// `SOKO_JUDGE_STATS=1` 时在进程退出前打到 stderr。为什么要有它：这一阶段的
/// 开销是**整个判卷的大头**（`unit12-solution` 实测 **64%**，见 `docs/PERF.md`
/// 的分阶段表），而它以前只有一次性探针量过（`docs/design/by-tactics.md` §13）。
/// 做成常驻开关之后，任何一次内核/前端改动都能**同口径**重量。
pub(crate) mod stats {
    use std::sync::atomic::{AtomicU64, Ordering};

    pub(crate) static CALLS: AtomicU64 = AtomicU64::new(0);
    pub(crate) static NANOS: AtomicU64 = AtomicU64::new(0);
    pub(crate) static PAIRS: AtomicU64 = AtomicU64::new(0);
    pub(crate) static PREFIX_BYTES: AtomicU64 = AtomicU64::new(0);
    static PRINTED: std::sync::Once = std::sync::Once::new();

    /// 每次判定调用的明细（`SOKO_JUDGE_STATS=2`）：调用序号 / 前缀字节 / 对数 /
    /// 耗时。**这一行直接回答"每加一个 tactic 是不是就重编前面全部"**——
    /// 看前缀字节是不是逐次增长、调用次数是不是等于 tactic 步数。
    pub(crate) static VERBOSE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    pub(crate) static SEQ: AtomicU64 = AtomicU64::new(0);
    /// 缓存命中 / 未命中（T-K20′ 的诊断：706 趟 pass 里有多少是"本该命中"）。
    pub(crate) static HITS: AtomicU64 = AtomicU64::new(0);
    pub(crate) static MISSES: AtomicU64 = AtomicU64::new(0);
    /// `judge_infer`（记法消解推类型）的调用数/耗时/命中——它是**第二个 G-31**：
    /// 缓存键含整段前缀 ⇒ 每条声明的每个记法展开都换一个键，未命中就全前缀重编译
    /// （G-34）。`HITS`/`MISSES` 这两个**是它的**，与 `judge_pairs` 那组分开：
    /// 实测 `unit12-solution` 命中 50,909 次只花 744ms，而 247 次未命中吃掉 6.2s
    /// ⇒ 优化必须打**未命中**（即"别问内核"），不是打哈希。
    pub(crate) static INFER_CALLS: AtomicU64 = AtomicU64::new(0);
    pub(crate) static INFER_NANOS: AtomicU64 = AtomicU64::new(0);
    pub(crate) static INFER_FAILS: AtomicU64 = AtomicU64::new(0);
    pub(crate) static INFER_HITS: AtomicU64 = AtomicU64::new(0);
    pub(crate) static INFER_MISSES: AtomicU64 = AtomicU64::new(0);
    /// 缓存**键构造**本身的耗时（含哈希整段前缀）——用来分辨"未命中重编译"
    /// 与"命中也要哈希"哪个是大头。
    pub(crate) static KEY_NANOS: AtomicU64 = AtomicU64::new(0);
    pub(crate) static HIT_NANOS: AtomicU64 = AtomicU64::new(0);

    // ── **P1-a 定向量具**（2026-09-29）：回答"judge 那 147s 里有多少能被
    //    「裸常量就地查表」消掉"。**只加计数、不改判定**（零语义风险 ✓）。
    //
    //    `judge_infer` 收的是**源码文本**（`term: &str`）⇒ 就地查表要先把文本
    //    认成"一个常量名"。
    /// `judge_infer` 总调用数（= 唯一可优化的分母）。
    pub(crate) static CLASSIFY_CALLS: AtomicU64 = AtomicU64::new(0);
    /// 其中 **term 可解析且是"裸常量"**（`Ident`/`UniverseApp`，**至少一段限定**，
    /// 即 `A.b` 这种）⇒ **结构上有可能**就地查表。
    pub(crate) static CLASSIFY_BARE: AtomicU64 = AtomicU64::new(0);
    /// 其中 **未命中**的（命中本来就不贵 ⇒ 只该打未命中）且是裸常量。
    pub(crate) static CLASSIFY_BARE_MISS: AtomicU64 = AtomicU64::new(0);
    /// 裸常量里，**名字在 prefix 文本中能原样找到**的（弱信号：说明它来自前缀，
    /// 大概率能在环境里查到）。**只作交叉参考**，不是判据。
    pub(crate) static CLASSIFY_RESOLVABLE: AtomicU64 = AtomicU64::new(0);
    /// 裸常量 + 未命中 + 可解析 的**累计耗时**（纳秒，只算 judge_infer 那一段）。
    pub(crate) static CLASSIFY_BARE_MISS_NANOS: AtomicU64 = AtomicU64::new(0);
    /// **所有**未命中的数与耗时（用来对照"是不是只有裸常量贵"）。
    pub(crate) static CLASSIFY_ALL_MISS: AtomicU64 = AtomicU64::new(0);
    pub(crate) static CLASSIFY_ALL_MISS_NANOS: AtomicU64 = AtomicU64::new(0);
    /// **重跑前缀的趟数**（判据的第二个读数；字节数用上面既有的 `PREFIX_BYTES`）。
    /// 噪声免疫（确定性）、不会被并发重复计时污染 ⇒ 可以作判据 ✓。
    pub(crate) static PREFIX_RUNS: AtomicU64 = AtomicU64::new(0);
    /// **身份退回原文的趟数**（判据 ③，2026-10-04 值守派单 ✓）。
    ///
    /// 为什么要有它 ✗：`canonical_prefix_cached` 有一条**保守退路** —— 前缀
    /// **解析不过**（或**超尺寸闸**）⇒ 直接用**原文**当身份 ✓（安全 ✓ 但**放弃特性** ✗：
    /// 改证明体会让下游全失效 ✓）。先前**没有任何东西**会因为"闸被触发"而判红 ✗
    /// ⇒ 「声明与守卫之间有缝」的又一例 ✗（值守原话 ✓）。
    ///
    /// **判据** ✓：整本课程跑完，这个数**必须 == 0** ✓；`> 0` 即判红 ✓，
    /// 并把**第一份**退回的前缀头 80 字节记在 `FALLBACK_HEAD` 里（够定位到模块 ✓）。
    pub(crate) static PREFIX_FALLBACKS: AtomicU64 = AtomicU64::new(0);
    /// 第一份退回原文的前缀**头 80 字节**（定位用 ✓；只在退回时写一次 ✓）。
    pub(crate) static FALLBACK_HEAD: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

    /// **增量身份自检的两读数**（判据 ④，2026-10-04 ✓）：`SOKO_PREFIX_ID_CHECK=1` 时
    /// walker 逐命令把「AST 累加出的身份」与「`canonical_prefix_id(整份前缀)`」比一遍 ✓。
    ///
    /// * `MISMATCHES` —— 两边**逐位不等**的条数 ✗。**必须 0** ✓（不等 = 键与真身份脱钩 =
    ///   错编的入口 ✗）。
    /// * `UNCOMPARABLE` —— 前缀**解析不过**（`canonical_prefix_id` 走了**退回原文** ✓）
    ///   因而**那次不比**的条数 ✓。
    ///
    /// ⚠ **为什么必须把 `UNCOMPARABLE` 单独报出来** ✗：上一棒只看"有没有 `MISMATCH`"，
    /// 而当时**所有**条都在 `UNCOMPARABLE` 那一类里 ⇒ 读到的是"**零分歧**" ✗，
    /// 实际是**整段降级**（实测：课程 `unit08` 1540 条里 1505 条解析不过 ✗）。
    /// 「咬不住的守卫等于没有」——一个会**全被跳过**的判据等于没判据 ✗。
    pub(crate) static IDENTITY_MISMATCHES: AtomicU64 = AtomicU64::new(0);
    pub(crate) static IDENTITY_UNCOMPARABLE: AtomicU64 = AtomicU64::new(0);
    /// **身份重解析的趟数**（O(n²) 的**结构读数** ✓，噪声免疫 ✓）：`canonical_prefix_cached`
    /// **没命中**（⇒ 真的 `parse` 了一整份前缀 ✗）的次数 ✓。
    /// 判据：预置生效时它应当**远小于命令数**（理想 0 ✓）；失效时会涨到 ≈ 命令数 ✗。
    pub(crate) static IDENTITY_PARSES: AtomicU64 = AtomicU64::new(0);
    /// 探针**跑过**多少条（防"守卫空转" ✗：`> 0` 才说明判据真的在比 ✓）。
    pub(crate) static IDENTITY_PROBED: AtomicU64 = AtomicU64::new(0);
    /// **记忆表淘汰了多少条** ✗（G-91 要求的"闸类计数出口" ✓）。
    ///
    /// 这张表是 `前缀原文哈希 → 环境身份哈希` 的**有界记忆化** ✓（每条约 16 字节 ✓）。
    /// 淘汰**只允许变慢** ✓（淘汰 ⇒ 下次重解析 ⇒ 结果**一模一样** ✓）——但**它必须可见** ✗：
    /// 淘汰到"刚种进去就被挤掉"的程度，预置就白做了 ✓（实测：`CAP=4096` 时
    /// 整本课程 `identity_parses=3062` ✗ —— 表比工作集小 ⇒ **抖动** ✗）。
    /// 判据：整本课程 `evictions` **必须 == 0** ✓（工作集装得下 ✓）。
    pub(crate) static IDENTITY_EVICTIONS: AtomicU64 = AtomicU64::new(0);

    /// 记一次「身份退回原文」✓（判据 ③ 的写入端）。
    pub(crate) fn note_prefix_fallback(src: &str, why: &str) {
        PREFIX_FALLBACKS.fetch_add(1, Ordering::Relaxed);
        if let Ok(mut slot) = FALLBACK_HEAD.lock() {
            if slot.is_none() {
                let head: String = src.chars().take(80).collect();
                *slot = Some(format!("[{why}] len={} head={head:?}", src.len()));
            }
        }
    }
    /// **P1-a 就地判定**（`SOKO_JUDGE_INPLACE`）的四个数：
    /// `USED` = 就地答上了（**没跑前缀**）· `FALLBACK` = 就地答不出、退回源码重跑 ·
    /// `SHADOW_SAME` / `SHADOW_DIFF` = 影子档下两条路的文本**逐字节是否相同**。
    /// `SHADOW_DIFF > 0` ⇒ 就地路径**不许开**（`on`），先查分叉。
    /// **未命中调用点探针**（`SOKO_JUDGE_CALLERS=<路径>`，G-31 选点用）。
    ///
    /// 为什么**不走 `atexit` 打印器**：那条路有早退门（`calls == 0 && INFER_CALLS == 0
    /// && …`），实测在"就地判定生效 / 只走 infer"的路上会把整份报告吞掉
    /// （台账与 2026-10-01 各撞过一次 ✗）⇒ 探针直接**追加到文件**，最钝但一定出数 ✓。
    /// 只在未命中时写（每次按键几十行，可忽略）。
    pub(crate) fn note_miss_caller(loc: &'static std::panic::Location<'static>) {
        let Ok(path) = std::env::var("SOKO_JUDGE_CALLERS") else {
            return;
        };
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
        {
            let short = loc.file().rsplit('/').next().unwrap_or(loc.file());
            let _ = writeln!(f, "{short}:{}", loc.line());
        }
    }

    pub(crate) static INPLACE_USED: AtomicU64 = AtomicU64::new(0);
    pub(crate) static INPLACE_FALLBACK: AtomicU64 = AtomicU64::new(0);
    pub(crate) static INPLACE_SHADOW_SAME: AtomicU64 = AtomicU64::new(0);
    pub(crate) static INPLACE_SHADOW_DIFF: AtomicU64 = AtomicU64::new(0);
    /// **E4 的判据读数（2026-10-08）**：影子档里因「prelude 安装期」**不比**而
    /// 提前返回的次数（`elab.rs` 那条排除分支 ✓）。
    ///
    /// **为什么要有它**：那条排除分支是 `course-stdlib.md` §7 要撤掉的东西 ——
    /// 撤掉之后这个数**必须是 0** ✓（"排除不再命中"= 判据本身 ✓）。今天它是
    /// **> 0** 的（prelude 安装期的判定全走排除 ✓）⇒ 撤之前先把这个数记下来 ✓
    /// （同 A4b 的"先建读数"纪律 ✓）。
    pub(crate) static INPLACE_SHADOW_PRELUDE_EXCLUDED: AtomicU64 = AtomicU64::new(0);
    /// **P1-b 第二刀（`by` 路径）**的两个数：`INPLACE_BY_USED` = `judge_render_type`
    /// 那一趟就地答上了（**没跑前缀**）· `INPLACE_BY_FALLBACK` = 就地答不出/开关关着。
    /// ⚠ 单独一组：`INPLACE_USED` 只记 `elab` 那条路，混在一起就**看不出
    /// `by` 这一档到底有没有生效**（"判据不许空转"）。
    pub(crate) static INPLACE_BY_USED: AtomicU64 = AtomicU64::new(0);
    pub(crate) static INPLACE_BY_FALLBACK: AtomicU64 = AtomicU64::new(0);
    /// **影子档**（`SOKO_JUDGE_INPLACE_BY=shadow`）的两个数：两条路的文本
    /// **逐字节是否相同**。`SHADOW_DIFF > 0` ⇒ 这一档**不许开**，先查分叉。
    pub(crate) static INPLACE_BY_SHADOW_SAME: AtomicU64 = AtomicU64::new(0);
    pub(crate) static INPLACE_BY_SHADOW_DIFF: AtomicU64 = AtomicU64::new(0);
    /// 影子档**第一个分叉**的原样记录（诊断；只记第一条，免得刷爆 ✗）。
    pub(crate) static INPLACE_BY_FIRST_DIFF: std::sync::Mutex<Option<String>> =
        std::sync::Mutex::new(None);

    /// **T3-B1 ①（2026-10-08）**：`cases` 被消去项的**就地**读数 —— **单独一组**，
    /// 否则会混进 `judge_render_type` 那一档 ⇒ **看不出这条接线到底有没有生效** ✗
    /// （"判据不许空转"）。`(used, fallback)` = On 档答上 / 答不出；
    /// 影子档另记 `(same, diff)`。
    pub(crate) static INPLACE_CASES_USED: AtomicU64 = AtomicU64::new(0);
    pub(crate) static INPLACE_CASES_FALLBACK: AtomicU64 = AtomicU64::new(0);
    pub(crate) static INPLACE_CASES_SHADOW_SAME: AtomicU64 = AtomicU64::new(0);
    pub(crate) static INPLACE_CASES_SHADOW_DIFF: AtomicU64 = AtomicU64::new(0);
    /// `cases` 影子档**第一个分叉**的原样记录（只记第一条 ✓）。
    pub(crate) static INPLACE_CASES_FIRST_DIFF: std::sync::Mutex<Option<String>> =
        std::sync::Mutex::new(None);

    /// **T3-B1 ③（2026-10-09）**：`level_hint_of` 的就地读数（**单独一组** ✓，
    /// 同 `cases` 的理由：混进别的档就分不清哪条接线生效 ⇒ 判据空转 ✗）。
    /// `(used, fallback)`：On 档答上 / 答不出。记法形态也计入（T3-B1 ③ 起）。
    pub(crate) static INPLACE_LEVEL_HINT_USED: AtomicU64 = AtomicU64::new(0);
    pub(crate) static INPLACE_LEVEL_HINT_FALLBACK: AtomicU64 = AtomicU64::new(0);

    /// `level_hint` 就地读数 `(used, fallback)`。
    pub fn inplace_level_hint() -> (u64, u64) {
        (
            INPLACE_LEVEL_HINT_USED.load(Ordering::Relaxed),
            INPLACE_LEVEL_HINT_FALLBACK.load(Ordering::Relaxed),
        )
    }

    /// **T3-B2 的判据读数**（`PLAN-align-lean4.md` §4.3：合成趟的精确结构计数，2026-10-09）：
    /// `(趟数, Σ命令数, 回退趟数)`。
    ///
    /// **为什么需要它**：`prefix=`（`PREFIX_RUNS`）**只数 `judge_infer` 的合成前缀重跑** ✗，
    /// 不数 `judge_type_of` / `judge_pairs`（by 路径）的合成趟 ⇒ 「合成趟还剩多少」
    /// 此前**没有出口** ✗。`MODULE_COMPILES` 也不行（它只在 `check::run` 里按
    /// `units.len()` 累加，而合成趟走 `run_incremental` ✗）。
    ///
    /// * `趟数` = 真的跑了 `run_synthesized_incremental` 的次数（有担保、走了增量）；
    /// * `Σ命令数` = 那些趟**重新 elaborate 的合成文档命令数**（= 前缀 + 合成声明，
    ///   是"消合成趟"要消掉的**工作量**读数 ✓）；
    /// * `回退趟数` = 没有担保 ⇒ `check_document_with` / `compile_fol_with` **整份重查**的次数
    ///   （比"合成趟"更贵，回落是默认不是异常 ⇒ 必须与合成趟分开数 ✓）。
    pub(crate) static SYNTHESIZED_PASSES: AtomicU64 = AtomicU64::new(0);
    pub(crate) static SYNTHESIZED_COMMANDS: AtomicU64 = AtomicU64::new(0);
    pub(crate) static SYNTHESIZED_FALLBACKS: AtomicU64 = AtomicU64::new(0);

    pub fn synthesized() -> (u64, u64, u64) {
        (
            SYNTHESIZED_PASSES.load(Ordering::Relaxed),
            SYNTHESIZED_COMMANDS.load(Ordering::Relaxed),
            SYNTHESIZED_FALLBACKS.load(Ordering::Relaxed),
        )
    }

    /// **T3-B1 · §4.2 第 5 条（2026-10-09）**：记法目标签名（`judge_type_of_constant`）
    /// 的就地读数。`(used, fallback)` = On 档就地答上 / 答不出；
    /// 影子档另记 `(same, diff)`。**单独一组**（不与别的档混 ⇒ 判据不空转 ✓）。
    pub(crate) static INPLACE_TOC_USED: AtomicU64 = AtomicU64::new(0);
    pub(crate) static INPLACE_TOC_FALLBACK: AtomicU64 = AtomicU64::new(0);
    pub(crate) static INPLACE_TOC_SHADOW_SAME: AtomicU64 = AtomicU64::new(0);
    pub(crate) static INPLACE_TOC_SHADOW_DIFF: AtomicU64 = AtomicU64::new(0);
    pub(crate) static INPLACE_TOC_FIRST_DIFF: std::sync::Mutex<Option<String>> =
        std::sync::Mutex::new(None);

    pub fn inplace_type_of_constant() -> (u64, u64) {
        (
            INPLACE_TOC_USED.load(Ordering::Relaxed),
            INPLACE_TOC_FALLBACK.load(Ordering::Relaxed),
        )
    }

    pub fn inplace_type_of_constant_shadow() -> (u64, u64) {
        (
            INPLACE_TOC_SHADOW_SAME.load(Ordering::Relaxed),
            INPLACE_TOC_SHADOW_DIFF.load(Ordering::Relaxed),
        )
    }

    /// `cases` 就地读数 `(used, fallback)`。
    pub fn inplace_cases() -> (u64, u64) {
        (
            INPLACE_CASES_USED.load(Ordering::Relaxed),
            INPLACE_CASES_FALLBACK.load(Ordering::Relaxed),
        )
    }

    /// `cases` 影子档读数 `(same, diff)`。
    pub fn inplace_cases_shadow() -> (u64, u64) {
        (
            INPLACE_CASES_SHADOW_SAME.load(Ordering::Relaxed),
            INPLACE_CASES_SHADOW_DIFF.load(Ordering::Relaxed),
        )
    }

    /// 影子档报告 `(same, diff)`。
    pub fn inplace_by_shadow() -> (u64, u64) {
        (
            INPLACE_BY_SHADOW_SAME.load(Ordering::Relaxed),
            INPLACE_BY_SHADOW_DIFF.load(Ordering::Relaxed),
        )
    }

    /// `by` 就地路径**答不出的原因**直方图（诊断用）。
    /// ⚠ **这一档没有它就是盲飞**：本轮全靠它才从"`used=0`"定位到
    /// `eval: loose bvar` 与 `peel`（附十）。
    pub(crate) static INPLACE_BY_REASONS: std::sync::Mutex<String> =
        std::sync::Mutex::new(String::new());

    /// 影子档**第一条分叉**的原样记录（只记第一条，免得刷爆 ✗）。
    pub(crate) fn note_first_diff(
        ty_text: &str,
        pp: &str,
        fast: &Option<String>,
        slow: &Option<String>,
    ) {
        if let Ok(mut first) = INPLACE_BY_FIRST_DIFF.lock() {
            if first.is_none() {
                let pp = if pp.is_empty() { "<none>" } else { pp };
                *first = Some(format!(
                    "ty={ty_text:?} | inplace_pp={pp:?} | fast={fast:?} | slow={slow:?}"
                ));
            }
        }
    }

    /// 记一笔 `by` 就地路径的放弃原因。
    /// **就地失败原因 → 文件**（`SOKO_JUDGE_INPLACE_LOG=<路径>`）。
    ///
    /// 为什么**不能**靠 `atexit` 打印器（2026-10-01 实测，白找了三轮）：
    /// ① LSP **不响应 `exit`**，量具靠 `child.kill()` 收尾 ⇒ `atexit` **根本不跑** ✗；
    /// ② 打印器还有早退门（`calls == 0 && INFER_CALLS == 0 && …`），
    ///    磁盘缓存命中的整趟跑完是**没有活可报**的 ⇒ 也一个字不打。
    /// ⇒ 与 `note_miss_caller` 同款：**发生时就追加到文件**，最钝但一定出数 ✓。
    pub(crate) fn note_inplace_fail(why: &str) {
        let Ok(path) = std::env::var("SOKO_JUDGE_INPLACE_LOG") else {
            return;
        };
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
        {
            let _ = writeln!(f, "{why}");
        }
    }

    pub(crate) fn note_by_reason(why: &str) {
        if let Ok(mut reasons) = INPLACE_BY_REASONS.lock() {
            reasons.push_str(why);
            reasons.push(' ');
        }
    }

    /// **`by` 路径**的两个数 `(used, fallback)`（P1-b 第二刀）。
    pub fn inplace_by() -> (u64, u64) {
        (
            INPLACE_BY_USED.load(Ordering::Relaxed),
            INPLACE_BY_FALLBACK.load(Ordering::Relaxed),
        )
    }

    /// 就地路径**答不出的原因**直方图（诊断用；影子档每分叉一次记一笔）。
    pub(crate) static INPLACE_FAIL_REASONS: std::sync::Mutex<String> =
        std::sync::Mutex::new(String::new());

    /// 未命中按 **[是否裸常量][term 长度桶]** 的 (次数, 耗时)。
    /// 桶：0 = `<16` 字节 · 1 = `<48` · 2 = `<160` · 3 = `≥160`。
    pub(crate) static CLASSIFY_BUCKET_N: [[AtomicU64; 4]; 2] =
        [const { [const { AtomicU64::new(0) }; 4] }; 2];
    pub(crate) static CLASSIFY_BUCKET_NS: [[AtomicU64; 4]; 2] =
        [const { [const { AtomicU64::new(0) }; 4] }; 2];

    /// 分桶报告（`SOKO_JUDGE_CLASSIFY=1`）。
    pub fn classify_buckets() -> [(u64, u64); 8] {
        let mut out = [(0u64, 0u64); 8];
        for bare in 0..2 {
            for b in 0..4 {
                out[bare * 4 + b] = (
                    CLASSIFY_BUCKET_N[bare][b].load(Ordering::Relaxed),
                    CLASSIFY_BUCKET_NS[bare][b].load(Ordering::Relaxed),
                );
            }
        }
        out
    }

    /// P1-a 量具报告（`SOKO_JUDGE_CLASSIFY=1` 时由 `check` 的 stage_stats 一起打）。
    /// 重跑前缀的结构读数（判据用）。
    pub fn prefix_runs() -> (u64, u64) {
        (
            PREFIX_RUNS.load(Ordering::Relaxed),
            PREFIX_BYTES.load(Ordering::Relaxed),
        )
    }

    /// **身份退回原文的趟数** ✓（判据 ③，2026-10-04 值守派单 ✓）。
    ///
    /// 判据：整本课程跑完**必须 == 0** ✓；`> 0` 判红，并把**第一份**退回的前缀头
    /// 一并返回（够定位到模块 ✓）。
    pub fn prefix_fallbacks() -> (u64, Option<String>) {
        (
            PREFIX_FALLBACKS.load(Ordering::Relaxed),
            FALLBACK_HEAD.lock().ok().and_then(|slot| slot.clone()),
        )
    }

    /// 就地判定的四个数（`JUDGE_INPLACE` 行）。
    pub fn inplace() -> (u64, u64, u64, u64) {
        (
            INPLACE_USED.load(Ordering::Relaxed),
            INPLACE_FALLBACK.load(Ordering::Relaxed),
            INPLACE_SHADOW_SAME.load(Ordering::Relaxed),
            INPLACE_SHADOW_DIFF.load(Ordering::Relaxed),
        )
    }

    /// 见 [`INPLACE_SHADOW_PRELUDE_EXCLUDED`]：**E4 撤掉排除分支之后必须是 0** ✓。
    pub fn inplace_shadow_prelude_excluded() -> u64 {
        INPLACE_SHADOW_PRELUDE_EXCLUDED.load(Ordering::Relaxed)
    }

    pub fn classify() -> (u64, u64, u64, u64, u64, u64, u64) {
        (
            CLASSIFY_CALLS.load(Ordering::Relaxed),
            CLASSIFY_BARE.load(Ordering::Relaxed),
            CLASSIFY_BARE_MISS.load(Ordering::Relaxed),
            CLASSIFY_RESOLVABLE.load(Ordering::Relaxed),
            CLASSIFY_BARE_MISS_NANOS.load(Ordering::Relaxed),
            CLASSIFY_ALL_MISS.load(Ordering::Relaxed),
            CLASSIFY_ALL_MISS_NANOS.load(Ordering::Relaxed),
        )
    }

    /// 判断 `term` 是不是"**裸常量**"（`A.b` / `A.b.{u}`，不含空格、不含记法）。
    ///
    /// **为什么用"至少一段限定"**：裸的 `x` 更可能是**局部变量**（T-K22 那条快路
    /// 已经先拦了），而 `Set.mem` 这种限定名**必然来自环境** ⇒ 它才是可查表的。
    /// ⚠ 这是**启发式**（不做完整 parse，省得在热路径上付 parse 成本）；
    /// 它只用来**估上界**，不作判据 ✓。
    pub(crate) fn looks_like_bare_const(term: &str) -> bool {
        // 去掉宇宙层 `.{u, v}` 尾巴。
        let head = match term.find(".{") {
            Some(i) if term.ends_with('}') => &term[..i],
            _ => term,
        };
        if head.is_empty() || head.contains(' ') || head.contains('(') || head.contains(')') {
            return false;
        }
        // 至少要有一段 `.`（限定名）；且每段都是标识符字符。
        let mut segs = head.split('.');
        let first = segs.next().unwrap_or("");
        if first.is_empty() || !first.chars().all(is_ident_char) {
            return false;
        }
        let rest: Vec<&str> = segs.collect();
        !rest.is_empty()
            && rest
                .iter()
                .all(|s| !s.is_empty() && s.chars().all(is_ident_char))
    }

    fn is_ident_char(c: char) -> bool {
        c.is_alphanumeric() || c == '_' || c == '\'' || c == '!' || c == '?'
    }

    /// 判定累计耗时（纳秒）——给 `check::stage_stats` 的分段账单用。
    pub fn hits() -> u64 {
        HITS.load(Ordering::Relaxed)
    }

    pub fn misses() -> u64 {
        MISSES.load(Ordering::Relaxed)
    }

    pub fn nanos() -> u64 {
        NANOS.load(Ordering::Relaxed)
    }

    pub(crate) fn verbose() -> bool {
        *VERBOSE.get_or_init(|| std::env::var("SOKO_JUDGE_STATS").is_ok_and(|v| v == "2"))
    }

    pub(crate) fn install_printer() {
        // `SOKO_JUDGE_INPLACE` 也要能打（就地路径的四个数是它自己的判据）。
        if std::env::var_os("SOKO_JUDGE_STATS").is_none()
            && std::env::var_os("SOKO_JUDGE_INPLACE").is_none()
            && std::env::var_os("SOKO_JUDGE_ENV_PROBE").is_none()
        {
            return;
        }
        PRINTED.call_once(|| {
            // 进程退出前打一次。`atexit` 之外没有更早的钩子，而判卷是 CLI 的
            // 最后一步 ⇒ 这个时机正好。
            extern "C" fn report() {
                let calls = CALLS.load(Ordering::Relaxed);
                // `SOKO_JUDGE_INPLACE` 下有可能**一次 `by` 判定都没发生**
                // （命中直接由调用方查表返回 ⇒ `judge_infer` 也不进）⇒ 只看 `CALLS`
                // 会把整份报告吞掉 ✗（实测：`on` 档单文件跑完一行都不打）。
                let (iu, ifb, iss, isd) = inplace();
                if calls == 0
                    && INFER_CALLS.load(Ordering::Relaxed) == 0
                    && iu == 0
                    && ifb == 0
                    && iss == 0
                    && isd == 0
                {
                    return;
                }
                let ms = NANOS.load(Ordering::Relaxed) / 1_000_000;
                eprintln!(
                    "JUDGE_STATS calls={calls} total_ms={ms} avg_ms={} pairs={} prefix_bytes={}",
                    ms / calls.max(1),
                    PAIRS.load(Ordering::Relaxed),
                    PREFIX_BYTES.load(Ordering::Relaxed),
                );
                let ic = INFER_CALLS.load(Ordering::Relaxed);
                let ims = INFER_NANOS.load(Ordering::Relaxed) / 1_000_000;
                eprintln!(
                    "JUDGE_INFER calls={ic} total_ms={ims} avg_us={} fails={}",
                    INFER_NANOS.load(Ordering::Relaxed) / ic.max(1) / 1_000,
                    INFER_FAILS.load(Ordering::Relaxed),
                );
                eprintln!(
                    "JUDGE_INFER_SPLIT hits={} misses={} key_ms={} hit_ms={}",
                    INFER_HITS.load(Ordering::Relaxed),
                    INFER_MISSES.load(Ordering::Relaxed),
                    KEY_NANOS.load(Ordering::Relaxed) / 1_000_000,
                    HIT_NANOS.load(Ordering::Relaxed) / 1_000_000,
                );
                if crate::judge::env_probe::on() {
                    eprintln!("{}", crate::judge::env_probe::report_line());
                }
                let (used, fallback, same, diff) = inplace();
                if std::env::var_os("SOKO_JUDGE_INPLACE").is_some() {
                    eprintln!(
                        "JUDGE_INPLACE used={used} fallback={fallback} shadow_same={same} \
                         shadow_diff={diff}"
                    );
                    let (bu, bf) = inplace_by();
                    let (bs, bd) = inplace_by_shadow();
                    eprintln!("JUDGE_INPLACE_BY used={bu} fallback={bf} shadow_same={bs} shadow_diff={bd}");
                    if let Ok(first) = INPLACE_BY_FIRST_DIFF.lock() {
                        if let Some(text) = first.as_ref() {
                            eprintln!("JUDGE_INPLACE_BY_FIRST_DIFF {text}");
                        }
                    }
                    if let Ok(reasons) = INPLACE_BY_REASONS.lock() {
                        if !reasons.is_empty() {
                            // ⚠ **截断**：未命中可能几万条，全打会把终端刷爆 ✗
                            let head: String = reasons.chars().take(240).collect();
                            eprintln!("JUDGE_INPLACE_BY_WHY {head}");
                        }
                    }
                    if let Ok(mut reasons) = INPLACE_FAIL_REASONS.lock() {
                        if !reasons.is_empty() {
                            let mut counts: Vec<(String, usize)> = Vec::new();
                            for word in reasons.split_whitespace() {
                                match counts.iter_mut().find(|(k, _)| k == word) {
                                    Some((_, n)) => *n += 1,
                                    None => counts.push((word.to_string(), 1)),
                                }
                            }
                            counts.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
                            let shown: Vec<String> =
                                counts.iter().map(|(k, n)| format!("{k}={n}")).collect();
                            eprintln!("JUDGE_INPLACE_WHY {}", shown.join(" "));
                            reasons.clear();
                        }
                    }
                }
            }
            unsafe extern "C" {
                fn atexit(cb: extern "C" fn()) -> i32;
            }
            unsafe {
                atexit(report);
            }
        });
    }
}

/// **本趟 pass「成功进环境」的声明名表**（G-31/G-92 的第二刀，2026-10-07 ✓）。
///
/// 为什么需要它：合成判定文档要**重跑前缀**才能建出环境，而前缀里那些
/// `theorem … := by …` 的**证明体**对下游**零可观测**（`conv.rs::unfold_hint`
/// ⇒ 定理一律 `Opaque`、**永不展开** ✓；`walk.rs` 的 `defs` delta 表**只收 `fn def`** ✓）
/// ⇒ 只要知道「调用方那一趟**确实加过**这个名字」，内层就可以**只 elaborate 类型**、
/// 按**不透明常量**加进去 ⇒ 前缀的 `by` **不再重跑**（`by_calls` 的 Σ(1..N) 消失 ✓）。
///
/// ⚠ 用**名字**而不是**命令号** ✗→✓（2026-10-07 **实测** ✓）：两套坐标系**不对齐**
/// —— G-92 夹具（单文件）`JUDGE_ENV_PROBE` 读数 `exact=20/overshoot=0` ✓，但真实课程
/// （带 `import`）是 `before = prefix_commands + 1`（实测 `28/27 29/28 …`）
/// ⇒ 位置对齐**不成立** ✗（`importless_source` 剥掉的那条 `import` 命令）。
/// 名字是**声明身份** ⇒ 与坐标系无关 ✓。
pub(crate) type EnteredNames = std::rc::Rc<std::cell::RefCell<std::collections::HashSet<String>>>;

/// `TRUSTED_PREFIX` 的栈项。
struct TrustEntry {
    /// 调用方已核的命令数。
    before: usize,
    /// 那些命令里失败的那部分。
    failures: HashMap<usize, CompileError>,
    /// 调用方那一趟**成功进环境**的声明名（`None` = 不提供 ⇒ 不透明快路**关** ✓）。
    entered: Option<EnteredNames>,
}

thread_local! {
    /// **外层 pass 能担保的"已核前缀"栈**（T-K11 / K1-a）。
    ///
    /// 每项 = `(已核命令数, 那些命令的失败表, 成功进环境的声明名)`。由 `run_incremental`
    /// 在跑 pass 期间压栈（进出成对 ✓ 见 `with_trusted_prefix`）；judge 的 **cache miss**
    /// 路径读栈顶。
    ///
    /// 语义（**只在满足条件时才复用**）：只有 `before` **覆盖住本次合成文档的全部
    /// 前缀命令**，才说明这些声明的内核检查在本轮 compile 里**已经被担保过**
    /// （增量会话里它们来自上一次会话的缓存）。否则老老实实整份重查。
    static TRUSTED_PREFIX: std::cell::RefCell<Vec<TrustEntry>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

/// 在"外层 pass 可担保 `[0, before)` 已核"的上下文里跑 `f`（T-K11）。
///
/// **栈式**：judge 的合成文档在 elaborate 期间又会触发 judge（递归）⇒ 每层看到
/// 自己那一层，不会串味；`before` 的比较在 [`check_synthesized`] 里做。
///
/// `entered` = 本趟 pass 的「成功进环境」名表（`walk.rs` 逐命令压栈时带上自己的那张 ✓）；
/// **只有** `run_synthesized_incremental` 那条路会把它交给内层（其余一律 `None`
/// ⇒ 快路**关** ⇒ 逐字节回到今天 ✓）。
pub(crate) fn with_trusted_prefix<R>(
    before: usize,
    failures: &HashMap<usize, CompileError>,
    entered: Option<EnteredNames>,
    f: impl FnOnce() -> R,
) -> R {
    TRUSTED_PREFIX.with(|cell| {
        cell.borrow_mut().push(TrustEntry {
            before,
            failures: failures.clone(),
            entered,
        })
    });
    struct Pop;
    impl Drop for Pop {
        fn drop(&mut self) {
            TRUSTED_PREFIX.with(|cell| {
                cell.borrow_mut().pop();
            });
        }
    }
    let _pop = Pop;
    f()
}

/// 开关（仿 `SOKO_NO_JUDGE_BATCH`）：前缀复用 —— **对拍用**：
/// 开与关必须给出**逐字节相同**的 `--json`（已实测 ✓，两态 md5 相同 ✓）。
///
/// **默认值变过两次，两次都按"收益成立才默认打开"那条规则** ✓：
///
/// * **2026-09-25 → 默认关** ✗⇒✓：那时**只有 LSP 增量会话**会压栈，而收益在
///   那条路上**被证否** —— 在重度走到该路径的真实套件上两态对拍
///   （`cargo test -p sokonanoda-lsp --lib`，**12.8 万次判卷**、
///   **命中率 99.6%**（`hits=128215/misses=485`））：
///   ```text
///   复用开：JUDGE_STATS total_ms=9706   JUDGE_INFER total_ms=28247
///   复用关：JUDGE_STATS total_ms=9614   JUDGE_INFER total_ms=28320
///   ```
///   ⇒ 差 **< 0.3%** 且**方向相反** ⇒ **Δ 是噪声** ⇒ 按规则关掉 ✓。
///   ⚠ 那个结论**对那条路仍然成立** ✓（`docs/design/incremental-environment.md` §30.1：
///   T-K11 只服务 LSP 增量会话，`build` 那条路从不触发它）。
/// * **2026-09-30 → 默认开** ✓：§3.C 把担保接到了**主编译 pass**
///   （`compile/check/walk.rs` 压栈），收益变成**全课程 `build`
///   126.4s → 47.8s（2.65×）**、`judge_ms` **−83%** ✓ ⇒ 同一条规则 ⇒ 该开 ✓。
///
/// **两个逃生门**：`SOKO_JUDGE_ENV_REUSE=0`（关这一层）·
/// `SOKO_JUDGE_ENV_VOUCH=0`（只关主编译 pass 那一档的担保）✓。
/// **要复现 09-25 那份数字**：`SOKO_JUDGE_STATS=1 cargo test -q -p sokonanoda-lsp --lib`
/// （两态各一次，`SOKO_JUDGE_ENV_REUSE=1` / `=0` ✓）。
fn judge_env_reuse_enabled() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| {
        // 显式 `SOKO_JUDGE_ENV_REUSE=0` ⇒ 关（**逃生门**，压过下面的默认）✓。
        if let Ok(v) = std::env::var("SOKO_JUDGE_ENV_REUSE") {
            return v != "0";
        }
        // ⚠ **未设时跟 `SOKO_JUDGE_ENV_VOUCH` 走**（2026-09-30 改）：
        // 旧默认是"关"，依据是**当时**在 LSP 会话路径上量到收益是噪声
        //（<0.3%，见上面那段注释）—— 那个结论对**那条路**仍然成立 ✓。
        // 但 §3.C 把担保接到了**主编译 pass**（`walk.rs` 压栈）之后，
        // 收益变成**全课程 `build` 126.3s → 47.6s（2.65×）**、`judge_ms` −83% ✓
        // ⇒ 按"**收益成立才默认打开**"那条规则，现在**该开** ✓。
        // 想回到旧行为：`SOKO_JUDGE_ENV_REUSE=0`（一行，无其他耦合）✓。
        env_probe::vouch_mode() != env_probe::VouchMode::Off
    })
}

/// **合成文档里「前缀定理装成不透明常量」那一刀的档位**（G-31/G-92 第二刀，2026-10-07）。
///
/// * `On`（**默认**）⇒ 快路生效：内层 walk 对「调用方那趟确实加过」的 `theorem`
///   **只 elaborate 类型**、按 `Declar::Axiom` 加进环境（**证明体不重跑** ✓）；
/// * `Off` ⇒ **逐字节回到今天**（**反向验证**用 ✓，也是逃生门 ✓）；
/// * `Shadow` ⇒ 合成编译**两条都跑**、逐条比 `judgement_of`（**判据级** ✓，不比报告形状
///   —— §31.3 的教训 ✓），**返回 `Off` 那一份** ⇒ 行为零变化、只取证 ✓。
///
/// ⚠ 档位**只在这里读一次**（`OnceLock`）⇒ 影子档靠**显式传参**跑两遍，
/// **不改全局状态** ⇒ 多线程下也不会串味 ✓。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum PrefixOpaqueMode {
    Off,
    Shadow,
    On,
}

pub(crate) fn prefix_opaque_mode() -> PrefixOpaqueMode {
    static MODE: OnceLock<PrefixOpaqueMode> = OnceLock::new();
    *MODE.get_or_init(
        || match std::env::var("SOKO_JUDGE_PREFIX_OPAQUE").ok().as_deref() {
            Some("off") | Some("0") => PrefixOpaqueMode::Off,
            Some("shadow") => PrefixOpaqueMode::Shadow,
            // 未设 / `on` / 其它 ⇒ 快路（默认开）。
            _ => PrefixOpaqueMode::On,
        },
    )
}

/// judge 合成的文档送内核（T-K11 / K1-a）：**前缀已被外层担保**时走
/// `run_incremental`（前缀不再重查），否则回退到原来的 `check_document_with`
/// （整份重查）——回退是**默认**，不是异常路径。
///
/// `prefix_commands` = 这份合成文档里**属于前缀**的命令数（合成声明接在其后）。
fn check_synthesized(
    file: &FolFile,
    options: &CompileOptions,
    prefix_commands: usize,
    // 影子档要按 `_soko_judge_{k}`（k < pairs_len）逐条比对**判据** ✓。
    pairs_len: usize,
) -> DocumentReport {
    // **只读取证**（零行为变化）：量"若主编译 pass 压了栈，能不能担保住"。
    if env_probe::on() {
        let top = TRUSTED_PREFIX.with(|cell| cell.borrow().last().map(|e| e.before));
        env_probe::record(top, prefix_commands);
    }
    // **不透明快路的影子档**（`SOKO_JUDGE_PREFIX_OPAQUE=shadow`）：两条都跑、
    // 逐条比 `judgement_of`（**判据级** ✓），**返回 `off` 那一份** ⇒ 行为零变化 ✓。
    if prefix_opaque_mode() == PrefixOpaqueMode::Shadow {
        let Some((_out, base, _before)) =
            run_synthesized_incremental(file, options, prefix_commands, false)
        else {
            return check_document_with(file, options);
        };
        let Some((_out2, fast, _before2)) =
            run_synthesized_incremental(file, options, prefix_commands, true)
        else {
            env_probe::note_opaque_shadow(false, "快路那一趟没有可用担保（不该发生）");
            return base;
        };
        let mut first_bad: Option<String> = None;
        for k in 0..pairs_len {
            let a = judgement_of(&base, k);
            let b = judgement_of(&fast, k);
            if format!("{a:?}") != format!("{b:?}") {
                first_bad = Some(format!(
                    "OPAQUE_SHADOW k={k} prefix_commands={prefix_commands}\n  不透明=关 {a:?}\n  不透明=开 {b:?}"
                ));
                break;
            }
        }
        match &first_bad {
            None => env_probe::note_opaque_shadow(true, ""),
            Some(d) => env_probe::note_opaque_shadow(false, d),
        }
        return base;
    }
    let Some((_out, trusted_report, before)) = run_synthesized_incremental(
        file,
        options,
        prefix_commands,
        prefix_opaque_mode() == PrefixOpaqueMode::On,
    ) else {
        note_synthesized_fallback();
        return check_document_with(file, options);
    };
    // **影子档**：再跑一次"整份重查"，比对**判据**（行为仍返回整份那一份）。
    if env_probe::vouch_mode() == env_probe::VouchMode::Shadow {
        let full = check_document_with(file, options);
        // ⚠⚠ **比的是"判据"，不是"报告"** —— 第一版拿 `Debug` 比整份
        // `DocumentReport` ⇒ **265/265 全判 diff** ✗，而那是**假分叉**：
        // `run_incremental` 的 `decls` **只含它这一段新查的**（`_soko_judge_*`），
        // 而 `check_document_with` 的 `decls` 含**整个前缀**的声明
        //（实测：`trusted` 28,342 字符 vs `full` 224,204 字符，首个不同就在
        // `_soko_judge_0` vs `Exists`）⇒ 差的是**报告的范围**，不是**判定** ✗。
        // ⇒ 判据必须绑**消费方看得见的结果**：调用方只读 `judgement_of(report, k)`
        //（`judge_pairs_uncached` 结尾那句）⇒ 就比它 ✓
        //（同 `AGENTS.md` 验证设计纪律第 1 条：**断言用户可见的结果**）。
        let mut first_bad: Option<String> = None;
        for k in 0..pairs_len {
            let a = judgement_of(&trusted_report, k);
            let b = judgement_of(&full, k);
            if format!("{a:?}") != format!("{b:?}") {
                first_bad = Some(format!(
                    "k={k} prefix_commands={prefix_commands} before={before}\n                       担保路={a:?}\n  整份重查={b:?}"
                ));
                break;
            }
        }
        match &first_bad {
            None => env_probe::note_shadow(true, ""),
            Some(d) => env_probe::note_shadow(false, d),
        }
        return full;
    }
    trusted_report
}

/// **从一次编译输出里取「最后那条命令」的 `TypeChecked` 文本** ✓
/// （`#check` 两条路的**唯一实现** ✗ —— 别各写一份 ✓）。
///
/// 先按**命令号**过滤（合成的前缀里可能本来就有 `#check` ✓，它们的事件排在前面 ✓），
/// 对不上再退回「最后一条 `TypeChecked`」✓（`event_cmds` 理论上总与 `events` 平行；
/// 若解析把查询并进了别的命令，命令号就对不上 ✓）。
fn pick_type_checked(out: &CompileOutput, last_cmd: Option<usize>) -> Option<String> {
    out.events
        .iter()
        .zip(out.event_cmds.iter())
        .filter(|(_, cmd)| Some(**cmd) == last_cmd)
        .find_map(|(e, _)| match e {
            CheckEvent::TypeChecked { text, .. } => Some(text.clone()),
            _ => None,
        })
        .or_else(|| {
            out.events.iter().rev().find_map(|e| match e {
                CheckEvent::TypeChecked { text, .. } => Some(text.clone()),
                _ => None,
            })
        })
}

/// **合成文档的「受信任前缀」担保** ✓（**唯一实现** ✗ —— 三处消费点共用它 ✓）。
///
/// 返回 `(before, failures, entered)`，其中 `before` **已经夹到 `prefix_commands`** ✓；
/// 没有可用担保 ⇒ `None`（**调用方必须逐字回退** ✓）。
/// `entered` = 调用方那趟「成功进环境」的声明名表（`None` ⇒ 不透明快路**关** ✓）。
fn synthesized_trust(
    prefix_commands: usize,
) -> Option<(usize, KernelFailed, Option<EnteredNames>)> {
    if !judge_env_reuse_enabled() {
        return None;
    }
    let trusted = TRUSTED_PREFIX.with(|cell| {
        cell.borrow()
            .last()
            .filter(|e| e.before >= prefix_commands)
            .map(|e| (e.before, e.failures.clone(), e.entered.clone()))
    });
    let (before, failures, entered) = trusted?;
    REUSED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    if reuse_stats() {
        eprintln!(
            "JUDGE_ENV_REUSE: 命中（before={before} prefix_commands={prefix_commands} 累计={}）",
            REUSED.load(std::sync::atomic::Ordering::Relaxed)
        );
    }
    // ⚠⚠ **必须夹到 `prefix_commands`**（2026-09-30 实测踩到）：
    //
    // `before` 是**调用方坐标系**里的"已核命令数"，而 `run_incremental` 要的是
    // **这份合成文档**里的命令数 —— 两个坐标系**可以不等**。
    // 实测（主编译 pass 压 `idx` 的那一版）：`idx` 恒比 `prefix_commands` **大 2**
    //（探针：`exact=86 · overshoot=179`）⇒ 不夹会**多担保 2 条命令**，
    // 而那 2 条正是追加的合成声明 `_soko_judge_k` ⇒ **判定声明根本没被检查** ✗
    // ⇒ 症状：`--json` 里 `compiled` 变 `failed`（**38 行不同**）✗✗。
    //
    // **夹是安全的、且严格更保守**：`before >= prefix_commands` 只说明"调用方已核
    // 的**至少覆盖**了前缀"（前缀文本是调用方文本的**前段** ⇒ 它的命令必然落在
    // `[0, before)` 里 ✓），所以担保上界就是 `prefix_commands` 本身 ✓。
    // 夹完只会"少担保 ⇒ 多检查" ⇒ 不引入新的不健全 ✓。
    //
    // ⚠ **2026-10-07 追加实测** ✓（G-31 那一刀的前提核对）：夹**不等于**两套坐标系
    // 同构 ✗ —— 真实课程（带 `import`）实测 `before = prefix_commands + 1`
    //（`JUDGE_ENV_PROBE`：`28/27 29/28 …`，差的那条是 `importless_source` 剥掉的
    // `import` 命令）⇒ **按命令号对齐不成立** ✗。所以「哪些命令成功进环境」这份
    // 信息**必须按声明名传**（见 `EnteredNames` ✓），不许按位置 ✗。
    Some((before.min(prefix_commands), failures, entered))
}

/// **T3-B2 的读数**：没有可用担保 ⇒ **整份重查**的回退（比"合成趟"更贵 ⇒ 分开数 ✓）。
pub(crate) fn note_synthesized_fallback() {
    stats::SYNTHESIZED_FALLBACKS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}

/// **受信任前缀下的合成编译** ✓（**唯一实现** ✗ —— `by` 路径与 `#check` 路径共用 ✓）。
///
/// `Some((CompileOutput, DocumentReport))` = 走了**增量**路 ✓（前缀**不再重查** ✓，
/// 只有追加的合成命令真的被检查 ✓）；`None` = 没有可用担保 ⇒ **调用方逐字回退** ✓
/// （`by` 路径回 `check_document_with` · `#check` 路径回 `compile_fol_with` ✓）。
///
/// **为什么需要它（G-92 真修 ✓）**：`#check` 那两条路（`judge_infer_uncached` /
/// `judge_type_of_uncached`）先前直接 `compile_fol_with` ⇒ **整份重编** ✗
/// ⇒ 前缀里那些 `by` 声明**又被 elaborate 一遍** ✗ ⇒ 一个文件里 N 条各自需要
/// 新判定的 `by` ⇒ 总成本 **N²** ✗（实测 `by_calls` 451→1996，**4.4×** ✗）。
///
/// `opaque` = 要不要把「调用方那趟确实加过」的前缀 `theorem` 装成**不透明常量**
/// （G-31/G-92 第二刀 ✓，见 `EnteredNames`）：`false` ⇒ **逐字节回到今天** ✓。
fn run_synthesized_incremental(
    file: &FolFile,
    options: &CompileOptions,
    prefix_commands: usize,
    opaque: bool,
) -> Option<(CompileOutput, DocumentReport, usize)> {
    let (before, failures, entered) = synthesized_trust(prefix_commands)?;
    if env_probe::on() {
        env_probe::record_clamped(before, prefix_commands);
    }
    let plan = TrustPlan {
        before,
        trusted_extra: Vec::new(),
        prev_signatures: Vec::new(),
        text_unchanged: Vec::new(),
        allow_cutoff: false,
        // **G-31/G-92 第二刀** ✓：把「调用方那趟成功进环境的声明名」交给内层
        // walk ⇒ 那些 `theorem` 只 elaborate 类型、按不透明常量加进去（证明体不重跑 ✓）。
        // ⚠ **只有这里**会设它 ⇒ 快路的爆炸半径 = judge 的合成文档 ✓（其余 pass 一律
        // `None` ⇒ 逐字节回到今天 ✓）。`opaque == false`（反向验证 / 影子档的对照趟）⇒
        // 同样 `None` ✓。
        trusted_entered: if opaque { entered } else { None },
    };
    // **S2 步 1**：`run_incremental` 的单元由调用方给（此前它写死单文件）。
    // 这条路是"judge 在**单文件**文本上重查前缀"，所以仍然是一个单元。
    let units = [crate::compile::SourceUnit::single("", file)];
    // **T3-B2 的判据读数**（`PLAN-align-lean4.md` §4.3）：真的跑了一趟合成编译
    // ⇒ 记趟数 + **重新 elaborate 的命令数**（= 要消掉的工作量 ✓）。
    stats::SYNTHESIZED_PASSES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    stats::SYNTHESIZED_COMMANDS.fetch_add(
        file.commands.len() as u64,
        std::sync::atomic::Ordering::Relaxed,
    );
    let (out, report, _checks, _sigs, _cutoff) = run_incremental(&units, options, &plan, &failures);
    Some((out, report, before))
}

/// **G-29 的结构读数**（判据用）：类型推断那条路的
/// `(调用数, 命中, 未命中, 前缀重跑趟数, 前缀字节数)`。
///
/// **为什么要公开它**：一次按键的真实成本落在 `judge_infer` 的**未命中**上
/// （每次未命中都把整份前缀从零编一遍）——而这条读数此前只在**进程退出**时
/// 打（`atexit`），LSP 又**不响应 `exit`**（实测：发 `exit` 后进程不退出）
/// ⇒ 在"按键"那条路上根本量不到 ✗。公开成函数之后，会话式路径（`QueryDoc`）
/// 的集成测试可以**前后取差**，判据因此是**结构计数**而不是墙钟 ✓
/// （消费者：`crates/front/tests/keystroke_structure.rs`）。
/// **增量身份自检的读数** ✓（判据 ④，2026-10-04）：`(probed, mismatches, uncomparable)`。
///
/// `probed` **必须 > 0** ✓ —— 否则判据是**空转**的（上一棒的教训 ✓：只读
/// "有没有 mismatch"，而当时**所有**条都落在 uncomparable 里 ⇒ 读成"零分歧" ✗）。
#[doc(hidden)]
pub fn identity_probe() -> (u64, u64, u64) {
    (
        stats::IDENTITY_PROBED.load(std::sync::atomic::Ordering::Relaxed),
        stats::IDENTITY_MISMATCHES.load(std::sync::atomic::Ordering::Relaxed),
        stats::IDENTITY_UNCOMPARABLE.load(std::sync::atomic::Ordering::Relaxed),
    )
}

/// **身份重解析的趟数** ✓（O(n²) 的**结构读数**，判据 ②）：`canonical_prefix_cached`
/// 没命中 ⇒ 真的 `parse` 了一整份前缀 ✗。预置生效时它应当**远小于命令数** ✓。
#[doc(hidden)]
pub fn identity_parses() -> u64 {
    stats::IDENTITY_PARSES.load(std::sync::atomic::Ordering::Relaxed)
}

/// **记忆表淘汰条数** ✓（判据 ② 的第二读数 + G-91 的闸类计数出口 ✓）。
#[doc(hidden)]
pub fn identity_evictions() -> u64 {
    stats::IDENTITY_EVICTIONS.load(std::sync::atomic::Ordering::Relaxed)
}

/// **整本课程跑完，`fallbacks` 必须 == 0** ✓（判据 ③）。
#[doc(hidden)]
pub fn prefix_fallbacks_report() -> (u64, Option<String>) {
    stats::prefix_fallbacks()
}

#[doc(hidden)]
pub fn infer_totals() -> (u64, u64, u64, u64, u64) {
    (
        stats::INFER_CALLS.load(std::sync::atomic::Ordering::Relaxed),
        stats::INFER_HITS.load(std::sync::atomic::Ordering::Relaxed),
        stats::INFER_MISSES.load(std::sync::atomic::Ordering::Relaxed),
        stats::PREFIX_RUNS.load(std::sync::atomic::Ordering::Relaxed),
        stats::PREFIX_BYTES.load(std::sync::atomic::Ordering::Relaxed),
    )
}

/// `SOKO_JUDGE_REUSE_STATS=1` ⇒ 每次命中打一行（判断"到底有没有触发"用；
/// 只看耗时区分不出"触发了但收益小"与"根本没触发"）。
fn reuse_stats() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var("SOKO_JUDGE_REUSE_STATS").is_ok())
}

/// 命中"前缀复用"的次数（判据：`SOKO_JUDGE_ENV_REUSE=0/1` 下都该有正确的行为，
/// 而开启时这个数应当 > 0 —— 否则说明条件从没满足、等于没生效）。
pub static REUSED: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// **只读取证（§31.2 的"先证明再动手"）**：`check_synthesized` 每次被调用时，
/// 记一笔"**如果**主编译 pass 压了栈，能不能担保住"。
///
/// ⚠ **纯计数，零行为变化**：三个计数器只在 `SOKO_JUDGE_ENV_PROBE=1` 时累加，
/// 且**不参与任何判定** ✓。它的用途是回答 §31.2 的第 ① 条前提：
/// **"judge 的文本前缀"与"主编译 pass 已核的命令"是否同序同源** ——
/// 若 `would_hit` 占比高 ⇒ 那条路成立 ✓；若普遍不等 ⇒ 作废 ✗。
pub mod env_probe {
    use std::sync::atomic::{AtomicU64, Ordering};

    /// `check_synthesized` 被调用的总次数。
    pub static CALLS: AtomicU64 = AtomicU64::new(0);
    /// 其中**栈顶 `before >= prefix_commands`**（即"能担保住"）的次数。
    pub static WOULD_HIT: AtomicU64 = AtomicU64::new(0);
    /// 栈**空着**的次数（= 主编译 pass 没压栈 ⇒ 这条路当前完全没生效）。
    pub static STACK_EMPTY: AtomicU64 = AtomicU64::new(0);
    /// 栈非空但 `before < prefix_commands` 的次数（= 担保**不够长**）。
    pub static TOO_SHORT: AtomicU64 = AtomicU64::new(0);

    pub fn on() -> bool {
        static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        *ON.get_or_init(|| std::env::var("SOKO_JUDGE_ENV_PROBE").is_ok())
    }

    /// **影子档**：`SOKO_JUDGE_ENV_VOUCH=shadow` ⇒ **两条路都跑**、比对报告，
    /// **返回"整份重查"那一份**（行为零变化，只取证）。
    ///
    /// ⚠ **这一档是必需品，不是可选项** —— P1-b 的实测教训：`on` 档的
    /// `--json` 逐字节相同**不足以**证明两条路一致（那一档第一版多剥一层，
    /// `on` 照样全绿，是**影子档**把 `diff=37508` 抓出来的）✓。
    pub static SHADOW_SAME: AtomicU64 = AtomicU64::new(0);
    pub static SHADOW_DIFF: AtomicU64 = AtomicU64::new(0);
    pub static SHADOW_FIRST_DIFF: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

    /// **「前缀定理装成不透明常量」那一刀自己的影子档**（`SOKO_JUDGE_PREFIX_OPAQUE=shadow`
    /// ✓，2026-10-07）：合成编译**两条都跑**（关 / 开）、逐条比 `judgement_of`
    /// （**判据级** ✓，不比报告形状 —— §31.3 的教训 ✓）。
    ///
    /// ⚠ **与上面那组 `SHADOW_*` 分开** ✗：那组量的是"受信任前缀 vs 整份重查"，
    /// 这组量的是"不透明快路 开 vs 关" —— 混在一起就分不清是哪条路分叉 ✗。
    pub static OPAQUE_SAME: AtomicU64 = AtomicU64::new(0);
    pub static OPAQUE_DIFF: AtomicU64 = AtomicU64::new(0);
    pub static OPAQUE_FIRST_DIFF: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub enum VouchMode {
        Off,
        Shadow,
        On,
    }

    /// **默认 `On`**（2026-09-30 起）—— 证据链（缺一不可）：
    /// ① 全课程影子档（**判据级**比对，两条路都跑）**`shadow_same=265 · diff=0`** ✓；
    /// ② `off` vs 默认的 `--json`（剔 `build.tick`/`build.progress`）**逐行不同 0 行** ✓；
    /// ③ 反向判据实测（`crates/front/tests/judge_env_vouch.rs`：**去掉夹紧 ⇒ 判红**）✓；
    /// ④ **带开关跑完整 `gate` PASS**（含 LSP 那 12.8 万次判卷与课程门禁）✓；
    /// ⑤ 读数：全课程墙钟 **126.3s → 47.6s（2.65×）**、`judge_ms` **−83%** ✓。
    ///
    /// **逃生门**：`SOKO_JUDGE_ENV_VOUCH=0`（只关这一档）·
    /// `SOKO_JUDGE_ENV_REUSE=0`（只关"复用前缀"那个更底层的开关）✓。
    pub fn vouch_mode() -> VouchMode {
        static M: std::sync::OnceLock<VouchMode> = std::sync::OnceLock::new();
        *M.get_or_init(
            || match std::env::var("SOKO_JUDGE_ENV_VOUCH").ok().as_deref() {
                Some("shadow") => VouchMode::Shadow,
                Some("0") | Some("off") => VouchMode::Off,
                _ => VouchMode::On,
            },
        )
    }

    /// 影子档的诊断输出开关（`SOKO_JUDGE_ENV_SHADOW_VERBOSE=1`）。
    pub fn shadow_verbose() -> bool {
        static V: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        *V.get_or_init(|| std::env::var("SOKO_JUDGE_ENV_SHADOW_VERBOSE").is_ok())
    }

    /// **影子档记一笔**（`same` = 两条路的报告**逐字节相同**）。
    pub fn note_shadow(same: bool, detail: &str) {
        if same {
            SHADOW_SAME.fetch_add(1, Ordering::Relaxed);
        } else {
            SHADOW_DIFF.fetch_add(1, Ordering::Relaxed);
            if shadow_verbose() {
                eprintln!("JUDGE_ENV_SHADOW_DIFF: {detail}");
            }
            if let Ok(mut f) = SHADOW_FIRST_DIFF.lock() {
                if f.is_none() {
                    *f = Some(detail.to_string());
                }
            }
        }
    }

    /// **不透明快路那一刀的影子档记一笔**（判据级 ✓）。
    pub fn note_opaque_shadow(same: bool, detail: &str) {
        if same {
            OPAQUE_SAME.fetch_add(1, Ordering::Relaxed);
        } else {
            OPAQUE_DIFF.fetch_add(1, Ordering::Relaxed);
            // 分叉**必须可见**（咬不住的守卫等于没有 ✓）：默认就打第一份，
            // 不受 `SOKO_JUDGE_ENV_SHADOW_VERBOSE` 管（那条是给上面那组的 ✓）。
            eprintln!("JUDGE_PREFIX_OPAQUE_SHADOW_DIFF: {detail}");
            if let Ok(mut f) = OPAQUE_FIRST_DIFF.lock() {
                if f.is_none() {
                    *f = Some(detail.to_string());
                }
            }
        }
    }

    /// **生效开关**（改行为）：`SOKO_JUDGE_ENV_VOUCH=1` ⇒ 主编译 pass 压栈担保。
    ///
    /// ⚠ **默认关**（一档一个 commit 的纪律；收益量到之后再议默认）。
    /// ⚠ 与 [`on`]（只读取证）**分开**：否则"量到的"与"生效的"分不清 ✗。
    pub fn vouch_enabled() -> bool {
        static V: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        *V.get_or_init(|| std::env::var("SOKO_JUDGE_ENV_VOUCH").is_ok_and(|v| v != "0"))
    }

    /// **精确相等**的次数（`before == prefix_commands`）—— 只有这一档才是**真的**
    /// "担保的正是前缀"，`before > prefix_commands` 会把**合成命令**也一起担保掉 ✗。
    pub static EXACT: AtomicU64 = AtomicU64::new(0);
    /// `before > prefix_commands` 的次数（**危险档**：多担保了）。
    pub static OVERSHOOT: AtomicU64 = AtomicU64::new(0);
    /// 观测到的 `(before, prefix_commands)` 差值直方图（前几条，诊断用）。
    pub static SAMPLES: std::sync::Mutex<Vec<(usize, usize)>> = std::sync::Mutex::new(Vec::new());
    /// 夹紧之后实际用的 `before` 与 `prefix_commands` 不等的次数（**必须恒为 0**）。
    pub static CLAMPED: AtomicU64 = AtomicU64::new(0);

    /// 记录"夹紧后仍不等"（非 0 ⇒ 夹的逻辑错了）。
    pub fn record_clamped(before: usize, prefix_commands: usize) {
        if !on() {
            return;
        }
        if before != prefix_commands {
            CLAMPED.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn record(stack_top: Option<usize>, prefix_commands: usize) {
        if !on() {
            return;
        }
        CALLS.fetch_add(1, Ordering::Relaxed);
        match stack_top {
            None => {
                STACK_EMPTY.fetch_add(1, Ordering::Relaxed);
            }
            Some(before) => {
                if before == prefix_commands {
                    EXACT.fetch_add(1, Ordering::Relaxed);
                } else if before > prefix_commands {
                    OVERSHOOT.fetch_add(1, Ordering::Relaxed);
                } else {
                    TOO_SHORT.fetch_add(1, Ordering::Relaxed);
                }
                if let Ok(mut s) = SAMPLES.lock() {
                    if s.len() < 12 {
                        s.push((before, prefix_commands));
                    }
                }
            }
        }
    }

    /// 一行报告（`SOKO_JUDGE_ENV_PROBE=1` ⇒ 进程退出前由 `install_printer` 打）。
    pub fn report_line() -> String {
        let c = CALLS.load(Ordering::Relaxed);
        let h = WOULD_HIT.load(Ordering::Relaxed);
        let e = STACK_EMPTY.load(Ordering::Relaxed);
        let s = TOO_SHORT.load(Ordering::Relaxed);
        let pct = if c == 0 {
            0.0
        } else {
            h as f64 / c as f64 * 100.0
        };
        let ex = EXACT.load(Ordering::Relaxed);
        let ov = OVERSHOOT.load(Ordering::Relaxed);
        let samples = SAMPLES
            .lock()
            .map(|s| {
                s.iter()
                    .map(|(b, p)| format!("{b}/{p}"))
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .unwrap_or_default();
        let ss = SHADOW_SAME.load(Ordering::Relaxed);
        let sd = SHADOW_DIFF.load(Ordering::Relaxed);
        let os = OPAQUE_SAME.load(Ordering::Relaxed);
        let od = OPAQUE_DIFF.load(Ordering::Relaxed);
        let cl = CLAMPED.load(Ordering::Relaxed);
        format!(
            "JUDGE_ENV_PROBE calls={c} exact={ex} overshoot={ov} too_short={s}              stack_empty={e} would_hit={h} ({pct:.1}%) clamped_bad={cl} shadow_same={ss} shadow_diff={sd} opaque_same={os} opaque_diff={od} | (before/prefix): {samples}"
        )
    }
}

fn judge_pairs_uncached(
    key: u64,
    extra_prefix: &str,
    prefix_src: &str,
    options: &CompileOptions,
    pairs: &[JudgePair],
) -> Vec<Judgement> {
    stats::install_printer();
    stats::CALLS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    stats::PAIRS.fetch_add(pairs.len() as u64, std::sync::atomic::Ordering::Relaxed);
    stats::PREFIX_BYTES.fetch_add(
        prefix_src.len() as u64,
        std::sync::atomic::Ordering::Relaxed,
    );
    let _timer = {
        struct T(std::time::Instant, u64, usize, usize, u64);
        impl Drop for T {
            fn drop(&mut self) {
                let ms = self.0.elapsed().as_millis();
                stats::NANOS.fetch_add(
                    self.0.elapsed().as_nanos() as u64,
                    std::sync::atomic::Ordering::Relaxed,
                );
                if stats::verbose() {
                    eprintln!(
                        "JUDGE_CALL #{:>3} key={:016x} prefix_bytes={:>8} pairs={:>3} ms={ms}",
                        self.1, self.4, self.2, self.3
                    );
                }
            }
        }
        T(
            std::time::Instant::now(),
            stats::SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1,
            prefix_src.len(),
            pairs.len(),
            key,
        )
    };
    let mut judgements = vec![
        Judgement::Error {
            code: "judge-not-run".to_string(),
            message: "判定未执行".to_string(),
        };
        pairs.len()
    ];
    if pairs.is_empty() {
        return judgements;
    }
    #[cfg(test)]
    PASSES.with(|c| c.set(c.get() + 1));
    // 剩余目标解析失败 → 全部判为解析错误。
    //
    // **前缀先解析**（顺序对调，G-04 第二刀）：前缀的 `FolFile` 里带着本文件
    // 声明过的记法，`render_expr` 打回来的目标文本（`a ∈ A`、`Aᶜ`）必须用同一张
    // 表回读，否则符号会被读成「未声明符号」——这是第一刀就有的边界（`by` 块的
    // 目标文本走 `render_expr` + `parse_expr_text` 往返），第二刀顺手修掉。前缀
    // 本来就为后面的合成声明解析，所以**零额外解析开销**；前缀里没有记法命令时
    // 逐字节等于旧行为。
    let full_prefix = synthesized_prefix(extra_prefix, prefix_src);
    let Ok(prefix_file) = parse_prefix(&full_prefix) else {
        return vec![
            Judgement::Error {
                code: "parse".to_string(),
                message: "前缀源码无法解析".to_string(),
            };
            pairs.len()
        ];
    };
    // 记法表走**唯一实现**（T-C04，`crate::notation::notation_table`）：显示路径
    // （线 C 的 print-back）要用同一张表反向折叠，两边分叉就是"两套真相"。
    // 这里传的是**已经解析好的**前缀命令 ⇒ 零额外解析开销，行为逐字节不变。
    let notations = crate::notation::notation_table(&prefix_file.commands);

    let mut commands = prefix_file.commands;
    // The synthesized declarations sit *after* the real prefix in the source:
    // give them a span past `prefix_src` and hand the prefix as the file text so
    // `command.span().start`-based prefix lookup (which `match`'s universe query
    // uses) sees the real declarations again (`Color`, …). Without this, a
    // `by exact match c with …` judgement would fail `elab-match-no-expected-type`.
    let prefix_len = full_prefix.len();
    let after_prefix = Span::new(
        Pos {
            offset: prefix_len,
            line: 0,
            column: 0,
        },
        Pos {
            offset: prefix_len,
            line: 0,
            column: 0,
        },
    );
    // 解析不成功的那些**不合成命令**，但**保留序号**（`_soko_judge_{k}` 里的 k
    // 仍是这一批里的位置）——`judgement_of` 按名字回查，序号不能顺延。
    // 合成声明接在前缀之后 ⇒ **此刻**的 commands.len() 就是"前缀命令数"（K1-a 用它判断能否复用）。
    let prefix_commands = commands.len();
    let mut pre_judged: Vec<bool> = vec![false; pairs.len()];
    for (k, pair) in pairs.iter().enumerate() {
        let Ok(goal) = crate::proof::parse_expr_text_with(&pair.spec.ty, &notations) else {
            judgements[k] = Judgement::Error {
                code: "parse".to_string(),
                message: format!("无法解析目标类型 `{}`", pair.spec.ty),
            };
            pre_judged[k] = true;
            continue;
        };
        // 把已写 binders 折叠回声明类型：`(b1 : T1) -> (b2 : T2) -> 剩余目标`。
        // binder 名字与显隐风格不影响内核检查（只影响打印），统一折成命名箭头。
        let ty = match fold_declared(goal, &pair.spec.binders, &notations) {
            Ok(ty) => ty,
            Err(missing) => {
                judgements[k] = Judgement::Error {
                    code: "elab-untyped-binder".to_string(),
                    message: format!("binder `{missing}` 缺少类型标注，无法合成判定声明"),
                };
                pre_judged[k] = true;
                continue;
            }
        };
        let Ok(term_expr) = crate::proof::parse_expr_text_with(&pair.term, &notations) else {
            judgements[k] = Judgement::Error {
                code: "parse".to_string(),
                message: format!("无法解析术语 `{}`", pair.term),
            };
            pre_judged[k] = true;
            continue;
        };
        let val = wrap_binders(&pair.spec.binders, term_expr, &notations);
        commands.push(Command::Def {
            name: format!("_soko_judge_{k}"),
            universe: pair.spec.universe.clone(),
            ty,
            val,
            span: after_prefix,
        });
    }
    let report = check_synthesized(
        &FolFile {
            commands,
            src: full_prefix,
        },
        options,
        prefix_commands,
        pairs.len(),
    );
    for (k, judgement) in judgements.iter_mut().enumerate() {
        if pre_judged[k] {
            continue;
        }
        *judgement = judgement_of(&report, k);
    }
    judgements
}

/// 当场判（**不做乐观批处理**）。
///
/// 少数 tactic 需要**当场**拿到结论才能继续（`assumption` 要按结论**挑**哪条
/// 假设命中，不是"通过/报错"二选一），它们用这个入口；其余（`have`/`exact`/
/// `rfl`）走 [`begin_batch`] 的乐观通道。
pub fn judge_terms_strict(
    prefix_src: &str,
    options: &CompileOptions,
    open: &OpenGoalSpec,
    terms: &[&str],
) -> Vec<Judgement> {
    judge_terms_with("", prefix_src, options, open, terms)
}

/// 乐观判定批次的记录项。
struct BatchItem {
    extra_prefix: String,
    prefix_src: String,
    options: CompileOptions,
    spec: OpenGoalSpec,
    term: String,
}

/// 乐观批处理总开关（测试/排错用；默认开）。
///
/// 关掉之后 `judge_terms` 恢复"每次调用当场判一遍"，用于**对拍**：
/// 开与关必须给出逐字相同的结论与诊断（`by::batch_matches_strict` 系列测试）。
static BATCHING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);

/// 打开/关闭乐观批处理；返回原值。
pub fn set_batching(on: bool) -> bool {
    BATCHING.swap(on, std::sync::atomic::Ordering::Relaxed)
}

/// `SOKO_JUDGE_INPLACE` 的三个档位（**P1-a 的就地判定**，2026-09-29）。
///
/// * `On`（**默认**，2026-09-29 开）⇒ **未命中**时就地答（**不编译前缀**），
///   答不出（elaborate 失败 / 内核拒绝）⇒ **退回源码重跑** ✓；
///   命中仍走今天那条哈希快路（[`judge_infer_lookup`]）✓；
/// * `off` ⇒ **完全回到今天的行为**（只走源码重跑）—— 回退开关，逐字节不变 ✓；
/// * `shadow` ⇒ **两条都跑**、比对文本，不一致就计数并打印（返回源码重跑那份
///   ⇒ 判定结果仍逐字节不变 ✓）—— 这是"就地路径可不可信"的判据档。
///
/// **为什么敢默认开**（判据，不是感觉）：`shadow` 档**全课程**实测
/// `shadow_same=555552` · **`shadow_diff=0`** ✓；且 `off` vs `on` 两份
/// `build --json`（剔除按设计随墙钟变的 `build.tick` 心跳）**逐字节相同** ✓
/// —— 读数见 `docs/design/p1a-measurements.md` 附七。
///
/// 只读一次环境（热路径上）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum InplaceMode {
    Off,
    Shadow,
    On,
}

pub(crate) fn inplace_mode() -> InplaceMode {
    static MODE: OnceLock<InplaceMode> = OnceLock::new();
    *MODE.get_or_init(
        || match std::env::var("SOKO_JUDGE_INPLACE").ok().as_deref() {
            Some("off") => InplaceMode::Off,
            Some("shadow") => InplaceMode::Shadow,
            // 未设 / `on` / 其它 ⇒ 就地（默认开）。
            _ => InplaceMode::On,
        },
    )
}

/// **`On` 档的就地失败要不要记原因**（`SOKO_INPLACE_WHY=1`，诊断用，默认零成本 ✓）。
///
/// 为什么单开一个开关：`On` 档以前**只计数不记因** ✗（原因只在 shadow 档且两条
/// 分叉时才记 ✗）⇒ "就地路为什么答不出"只能靠猜。而实测 `used=303 / fallback=695`
/// （**69.6% 答不出** ✗）⇒ 先量清是哪一类，再决定改哪儿 ✓。
pub(crate) fn inplace_why_enabled() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("SOKO_INPLACE_WHY").is_some())
}

/// **P1-b 的 wide 那两个接线点**是否生效（`guarded_binder_type` /
/// `solve_prefix_args`；`elab.rs` 的 4 个签名多一个 `InplaceEnv`）。
///
/// **2026-09-30 起默认开**（P1-b 收口）—— 与 `by` 那一档**同一个口径**：
/// 主开关 `SOKO_JUDGE_INPLACE=on`（默认）⇒ 本档也开；`=off` ⇒ 全关 ✓。
/// 单独回退留 `SOKO_JUDGE_INPLACE_WIDE=0`（**保留逃生门**，不改主开关就关掉它）。
pub(crate) fn inplace_wide() -> bool {
    static WIDE: OnceLock<bool> = OnceLock::new();
    *WIDE.get_or_init(|| {
        // 显式 `=0` ⇒ 只关这一档（逃生门）；否则跟主开关走。
        if std::env::var("SOKO_JUDGE_INPLACE_WIDE").is_ok_and(|v| v == "0") {
            return false;
        }
        inplace_mode() != InplaceMode::Off
    })
}

/// **P1-b 第二刀：`by` 路径**（`judge_render_type` 那 536 趟）是否生效。
///
/// **2026-09-30 起默认开**（P1-b 收口）：与 wide 同一条口径 ——
/// 主开关 `on`（默认）⇒ 开；`off` ⇒ 关；单独回退 `SOKO_JUDGE_INPLACE_BY=0` ✓。
///
/// ⚠ 默认开的**证据链**（缺一不可）：① 全课程影子档 **`shadow_same=44234` ·
/// `shadow_diff=0`**；② `off` vs `on` 的 `--json`（剔 `build.tick`/`build.progress`）
/// **逐行不同 0 行**；③ 反向判据实测（`crates/front/tests/judge_inplace_by.rs`）；
/// ④ 每个接线点都断言"**路径真被走到**"（`used=320`）✓。
pub(crate) fn inplace_by() -> bool {
    static BY: OnceLock<bool> = OnceLock::new();
    *BY.get_or_init(|| {
        if std::env::var("SOKO_JUDGE_INPLACE_BY").is_ok_and(|v| v == "0") {
            return false;
        }
        inplace_mode() != InplaceMode::Off
    })
}

/// **`by` 路径的档位**（与 P1-a 的 `InplaceMode` 同形；附九："影子档是这一档的
/// **必需品**，不是可选项" —— 因为 `by` 的判定决定后续 tactic 步进，分叉会以
/// "步进不同"出现，比 `elab` 路径难定位得多）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ByMode {
    Off,
    Shadow,
    On,
}

/// **`by` 路径的档位**：跟主开关走，另留两个显式值 ✓。
///
/// * `SOKO_JUDGE_INPLACE_BY=shadow` ⇒ **两条路都跑**，比对文本、记 `same/diff`，
///   **返回慢路那一份**（行为零变化，只取证 —— 这一档**永远**可以单独开影子）✓；
/// * `SOKO_JUDGE_INPLACE_BY=0|off` ⇒ **只关这一档**（逃生门）；
/// * 其它 / 未设 ⇒ 跟 `SOKO_JUDGE_INPLACE`：`off` ⇒ `Off`，否则 ⇒ `On` ✓。
pub(crate) fn inplace_by_mode() -> ByMode {
    static MODE: OnceLock<ByMode> = OnceLock::new();
    *MODE.get_or_init(|| {
        match std::env::var("SOKO_JUDGE_INPLACE_BY").ok().as_deref() {
            // 影子档**优先**：它行为零变化，任何时候都该能开 ✓
            Some("shadow") => return ByMode::Shadow,
            Some("0") | Some("off") => return ByMode::Off,
            _ => {}
        }
        match inplace_mode() {
            InplaceMode::Off => ByMode::Off,
            // 主开关的 `shadow` 对 `by` 这一档也意味着"两条路都跑" ✓
            InplaceMode::Shadow => ByMode::Shadow,
            InplaceMode::On => ByMode::On,
        }
    })
}

/// **`by` 路径的就地环境**：开关关着 ⇒ `None`（`walk` 那边一行都不用改行为）。
///
/// ⚠ 收成一个函数：`walk.rs` 有三个 `lower_value` 调用点，写三遍
/// `if inplace_by() { Some(…) } else { None }` 就是三份重复的**开关判定** ✗。
pub(crate) fn inplace_env_for_by<'e, 'a>(
    slot: &mut Option<crate::compile::elab::InplaceEnv<'e, 'a>>,
    builder: &'e mut sokonanoda::builder::EnvBuilder<'a>,
    known: &'e crate::compile::elab::KnownTable,
) {
    *slot = if inplace_by() {
        Some(crate::compile::elab::InplaceEnv { builder, known })
    } else {
        None
    };
}

/// `judge_render_type` 的**查询文本**（慢路与缓存键共用同一份，必须同源）。
pub(crate) fn render_type_query(ty: &str) -> String {
    // 绑定名必须**不可能与目标里的自由变量同名**（见 `judge_render_type` 的注释：
    // `fun (x : x = x) => x` 会把目标里的 `x` 捕获掉）。
    format!("fun (__soko_render : {ty}) => __soko_render")
}

/// **收尾**：内核 pp 文本 → 剥 `extra_binders` 层 → 剥 `__soko_render` 那层 →
/// 回读 → `render_roundtrip`。**三条出口（命中 / 就地 / 慢路）共用一个实现** ✓。
///
/// ⚠ `peel_binders` 失败时是 `break`（**静默**原样返回）—— 所以就地路**不许**
/// 依赖"文本剥层"来对齐形态（附十）：就地路现在**在项层面剥完**才交进来
/// ⇒ 这里传 `extra_binders = 0`。
/// **`judge_render_type` 的就地兄弟**（P1-b 第二刀，2026-09-30）。
///
/// 与慢路的**唯一**差别：查询项**不渲染成文本、不合成前缀重跑**，而是拿调用方
/// 手里的**源 AST** 直接 elaborate（`elab_expr`），再 `infer_type_text_at_peeled`
/// 在**项层面**剥掉整条望远镜后 pp ✓（为什么必须项层面剥：见 `elab.rs` 那条的注释）。
///
/// ⚠ 收 `ty: &Expr` 而**不是** `&str`：一旦经过 `render_expr`→`parse_expr_text`，
/// 前缀里声明的**源级记法**（`∈` / `ᶜ` / `''`）就解析不回来了
/// （P1-a 实测：单文件 **77822 次 Parse 失败**）。
///
/// `None` ⇒ 调用方**逐字**退回慢路 ✓。
pub(crate) fn judge_render_type_inplace<'a>(
    env: &mut crate::compile::elab::InplaceEnv<'_, 'a>,
    ctx: &crate::compile::elab::ElabCtx<'a, '_>,
    binders: &[crate::ast::Binder],
    ty: &Expr,
) -> Option<String> {
    crate::compile::elab::inplace_render_type(env, ctx, binders, ty)
}

/// **A2a（2026-10-08）**：带 **explicit pp 档**的就地渲染 —— 与慢路
/// [`judge_render_type_explicit`] 读**同一个线程局部**（`EXPLICIT_PP`）✓。
///
/// 为什么需要它：慢路在 `needs_explicit`（目标里有**前导隐式 ≥ 2** 的 def 头）时
/// 走**全显式 pp**（`judge_render_type_explicit`），而就地路的内核 pp 读的是
/// [`explicit_pp_active`]（`elab.rs` 的 `pp_options.explicit = judge::explicit_pp_active()`）
/// ⇒ **就地路必须把同一档置起来**，否则两条路的文本形态会分叉 ✗。
/// G-71 闸（0.81.0）当初把就地路整个关掉就是因为这个；而 **2026-10-04 就地路已经
/// 接上了 `explicit_pp_active()`** ⇒ 闸的理由已过期 ✓ —— 本函数就是把那一档
/// **补到就地路上**，让两条路同源。
///
/// `ExplicitPpGuard` 是**私有**的 ⇒ 这里开一个**受控入口**：只暴露"带档跑一次就地
/// 渲染"，不把 guard 本身交出去 ⇒ 调用方**不可能忘记还原** ✓（RAII 在函数内收口）。
pub(crate) fn judge_render_type_inplace_with_explicit<'a>(
    explicit: bool,
    env: &mut crate::compile::elab::InplaceEnv<'_, 'a>,
    ctx: &crate::compile::elab::ElabCtx<'a, '_>,
    binders: &[crate::ast::Binder],
    ty: &Expr,
) -> Option<String> {
    let _guard = ExplicitPpGuard::new(explicit);
    judge_render_type_inplace(env, ctx, binders, ty)
}

pub(crate) fn judge_render_type_finish(text: &str, extra_binders: usize) -> Option<String> {
    let rest = peel_binders(text.to_string(), extra_binders);
    let parsed = parse_expr_text(&rest).ok()?;
    let rest = peel_one_binder(&parsed)?;
    Some(render_roundtrip(&rest))
}

/// **只查缓存**（不跑前缀）：命中 ⇒ `Some(judge_infer` 那一份已剥 `n` 层的文本`)`。
///
/// P1-a 的同一条纪律：就地路径**只做未命中** —— 慢路对命中只花一次哈希
/// （~11 µs），换成一次 elaborate+推断+pp（~1 ms）是**负优化** ✓。
pub(crate) fn judge_render_type_lookup(
    prefix_src: &str,
    options: &CompileOptions,
    binders: &[GoalBinderSpec],
    term: &str,
) -> Option<String> {
    judge_infer_lookup("", prefix_src, options, binders, term)?.ok()
}

/// 与 [`judge_render_type_lookup`] 配对：把**就地**答出的文本写回**同一张缓存**
/// （键不变 ⇒ 下一次同样的问只花一次哈希 ✓）。
pub(crate) fn judge_render_type_store(
    prefix_src: &str,
    options: &CompileOptions,
    binders: &[GoalBinderSpec],
    term: &str,
    text: &str,
) {
    judge_infer_store(
        "",
        prefix_src,
        options,
        binders,
        term,
        &Ok(text.to_string()),
    );
}

/// **就地判定只在"未命中"时接管**所需的两个口子（P1-a，2026-09-29）。
///
/// **为什么必须让调用方先查缓存**（实测教训，不是设计偏好）：慢路对**缓存命中**
/// 只花一次哈希（实测 110 万次命中一共 **12.1 s ≈ 11 µs/次**），而就地路径每次都要
/// elaborate + 推断 + pp（**~1 ms/次**）⇒ 若在**每一次**调用上生效，就把 15 万次
/// 廉价命中换成 150 s 的活儿 ✗✗（实测：`unit12-solution` 单文件在 shadow 档
/// **400 s 跑不完**）。⇒ **就地只做那 3759 趟未命中**，命中仍走今天那条快路 ✓。
///
/// 两个函数与 `judge_infer_cached` **共用同一个键**（`judge_cache_key` 的同一串），
/// 所以"同样的键 ⇒ 同样的答案"这条性质不变 ✓。
pub(crate) fn judge_infer_lookup(
    extra_prefix: &str,
    prefix_src: &str,
    options: &CompileOptions,
    binders: &[GoalBinderSpec],
    term: &str,
) -> Option<Result<String, Judgement>> {
    let key = judge_infer_key(extra_prefix, prefix_src, options, binders, term);
    match judge_cache_get(key) {
        Some(JudgeCacheValue::Infer(r)) => Some(r),
        _ => None,
    }
}

/// 与 [`judge_infer_lookup`] 配对：把就地的答案写回**同一张缓存**（键不变）✓。
pub(crate) fn judge_infer_store(
    extra_prefix: &str,
    prefix_src: &str,
    options: &CompileOptions,
    binders: &[GoalBinderSpec],
    term: &str,
    r: &Result<String, Judgement>,
) {
    let key = judge_infer_key(extra_prefix, prefix_src, options, binders, term);
    judge_cache_put(key, JudgeCacheValue::Infer(r.clone()));
}

/// **G-85**：前缀**源码 → 环境身份**的**有界记忆化** ✓（键要用身份，见
/// `compile::canonical_prefix_id` ✓）。
///
/// **为什么必须记忆化**：键是**每次判定**都要算的 ✓（实测十几万次调用 ✗）⇒
/// 不缓存就得解析十几万次前缀 ✗✗。前缀的**种数**很少（每个单元一份 ✓）⇒
/// 一张小表就够 ✓；**有界**（满了整表清空 ✓）⇒ 不会随会话无限长 ✗。
/// ⚠ **返回的是哈希，不是身份文本本身** ✗→✓（2026-10-04 实测修正 ✓）：
/// 先前返回 `String` ⇒ **每次判定都克隆一整份前缀身份**（大单元 ~100 KB × 上千次 ✗✗）
/// ⇒ 冷开**变慢**（实测 `unit12-synthesis` didOpen **8.5s → 11.2s** ✗）。
/// 现在表里存 **u64 哈希** ✓ ⇒ 命中只读一个 8 字节 ✓。
#[track_caller]
/// **A7（2026-10-08）**：`src` 的**文本哈希**的小 LRU 记忆化（4 条）。
///
/// **为什么需要它**（端到端 profiling 实测）：`sample` 里 **judge 缓存键的 SipHash 占一次
/// 按键编译样本的 22.5%** ✗ —— `judge_infer_cache_key` **每次调用**都对**整份前缀文本**
/// 跑一遍 `judge_cache_key`（`canonical_prefix_cached` 的 `text_key`），而同一份前缀在一次
/// 编译里会被问几百次 ⇒ 同一段字节被反复哈希 ✗。
///
/// **为什么不换更快的哈希**：键值一变，碰撞预算就要重新论证，而"**错键 = 静默用旧答案**"
/// 是 B4 的红线 ✗。这里**哈希函数一字不动** ✓ —— 只是**不再重复算它**：
/// 命中判据是**逐字节相等**（`String == &str`）⇒ 命中与否**只由内容决定** ⇒
/// 指针复用（ABA）**不可能**给出错的哈希 ✓（哈希本来就是内容的函数）。
///
/// 守卫：`judge::tests::the_text_hash_memo_never_changes_a_key`（同一份文本的键恒等 +
/// 不同文本的键不同 ✓）。
fn text_hash_memo() -> &'static std::sync::Mutex<Vec<(String, u64)>> {
    static MEMO: std::sync::OnceLock<std::sync::Mutex<Vec<(String, u64)>>> =
        std::sync::OnceLock::new();
    MEMO.get_or_init(|| std::sync::Mutex::new(Vec::new()))
}

/// 文本 → 哈希（带小 LRU；见 [`text_hash_memo`]）。
fn canonical_text_key(src: &str) -> u64 {
    /// 4 条够用：一次编译里被反复问的前缀就那几份（`extra_prefix` + 闭包前缀）。
    const MEMO: usize = 4;
    let mut memo = text_hash_memo().lock().expect("text hash memo");
    if let Some(pos) = memo.iter().position(|(text, _)| text == src) {
        let hit = memo.remove(pos);
        let h = hit.1;
        memo.push(hit); // LRU：命中挪到末尾
        return h;
    }
    let h = judge_cache_key(&[src]);
    if memo.len() >= MEMO {
        memo.remove(0);
    }
    memo.push((src.to_string(), h));
    h
}

fn canonical_prefix_cached(src: &str) -> u64 {
    if src.is_empty() {
        return 0;
    }
    // ⚠ **尺寸闸已删** ✗→✓（2026-10-04 值守派单 · 用户 13:08 拍板「不许降级修」✓）：
    // 先前这里有一条 `PARSE_LIMIT = 64 KB` ⇒ 超过就 `return judge_cache_key(&[src])`（退回**原文** ✗）
    // —— 那让**大单元（unit12 等）根本没吃到「只改证明」这个特性** ✗（改证明体照样全失效 ✓）。
    // 正解是**增量身份** ✓：调用方（`compile/check/walk.rs`）边读边累加身份 ✓，
    // 用 [`seed_canonical_prefix`] **预置**进这张表 ✓ ⇒ 这里照样**命中** ✓、一次都不用解析 ✓。
    // 判据 ③（`judge::tests::a_large_prefix_must_not_fall_back_to_raw_text` ✓）钉住这条缝 ✓。
    // ⚠ **按文本哈希存** ✗→✓（2026-10-04 **实验定位** ✓）：表原本以**前缀原文**为键 ✗
    // ⇒ walker 逐命令种 ✓ 会把表灌满 ⇒ **别人的条目被挤掉** ✗ ⇒ 它们重解析 ✗，
    // 而其中**解析不过的**（如 goals 的 `extra_prefix` 片段 ✗）会**退回原文** ✗ ⇒ **键变了** ✗
    // ⇒ 多出 5 趟 `prefix` ✗（实验：`seed=off ⇒ prefix=0` ✓ / `seed=on ⇒ 5` ✗，
    // 且 `inplace=off` 时 `seed=on ⇒ 40` ✗ ⇒ 就是这条机制 ✓）。
    // 改成 **`u64` 文本哈希做键** ✓ ⇒ 4096 条只占 ~64 KB ✓ ⇒ 灌不满 ✓、不挤别人 ✓。
    // ⚠ 与判定缓存同一套 `u64` 键 ✓（同样的碰撞量级 ✓，一致 ✓）。
    // ⚠ **`CAP` 4096 → 65536** ✗→✓（2026-10-04 **实测定位** ✓）：这张表是
    // `前缀原文哈希 → 环境身份哈希` 的**有界记忆化** ✓（每条约 16 字节 ⇒ 65536 条约 1 MB ✓）。
    // **4096 比整本课程的工作集还小** ✗ ⇒ 表一满，**每次新种就挤掉一条活条目** ✗
    // ⇒ 那条前缀下次被问到时**重解析** ✗ ⇒ 实测整本课程 `identity_parses=3062` ✗、
    // `passes` 还多出 33 趟 ✗ —— **预置白做** ✓。淘汰本身**不改答案** ✓（重解析给出
    // 一模一样的身份 ✓），但它把"删掉 O(n²)"这件事**又还回去了** ✗。
    // ⇒ 判据：整本课程 `evictions == 0` ✓（`identity_evictions()` / `STAGE_STATS` ✓）。
    const CAP: usize = 65536;
    let text_key = canonical_text_key(src);
    if let Some(hit) = canonical_prefix_table()
        .lock()
        .expect("canonical prefix table")
        .get(&text_key)
    {
        return *hit;
    }
    // **结构读数**（判据 ②）：走到这里 = **真的 parse 了一整份前缀** ✗（O(n²) 的源头 ✓）。
    // 预置（`seed_canonical_prefix` ✓）生效时这里应当几乎不涨 ✓。
    stats::IDENTITY_PARSES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let id = crate::compile::canonical_prefix_id(src);
    // 与判定缓存同一套哈希（`judge_cache_key` ✓）⇒ 身份文本只在这里过一遍 ✓。
    let hash = judge_cache_key(&[&id]);
    let mut table = canonical_prefix_table()
        .lock()
        .expect("canonical prefix table");
    if table.len() >= CAP {
        if let Some(victim) = table.keys().next().copied() {
            table.remove(&victim);
            stats::IDENTITY_EVICTIONS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
    }
    table.insert(text_key, hash);
    hash
}

fn canonical_prefix_table() -> &'static Mutex<std::collections::HashMap<u64, u64>> {
    static TABLE: OnceLock<Mutex<std::collections::HashMap<u64, u64>>> = OnceLock::new();
    TABLE.get_or_init(|| Mutex::new(std::collections::HashMap::new()))
}

/// **增量身份** ✓（值守 2026-10-04 派单 ✓）：调用方**边读边累加**出身份后，用它**预置**进表 ✓。
///
/// **为什么必须有它** ✗：`canonical_prefix_cached` 的键是**前缀原文** ✓，而 walker 的前缀
/// 是**逐命令增长**的 ✗ ⇒ 每个命令一份**新**文本 ⇒ 表**永不命中** ✗ ⇒ 每个命令解析整份
/// 前缀 = **O(n²)** ✗（unit12 实测 **+2.3s** ✗，那正是我先前往里塞尺寸闸的原因 ✗）。
/// 调用方手上有**解析好的命令** ✓ ⇒ 它累加身份是 O(总长) ✓ ⇒ 预置之后这里只做一次查表 ✓。
///
/// ⚠ **等价性是这个函数的契约** ✓：`identity` 必须与 `compile::canonical_prefix_id(text)`
/// **逐位相等** ✗→✓（判据 ④：合成工程 + 真实大模块逐命令断言 ✓）。不等 = **错编** ✓。
pub fn seed_canonical_prefix(text: &str, identity: &str) {
    if text.is_empty() {
        return;
    }
    // 与读取侧同一个记忆化（种进来的那一份往往就是接下来被问几百次的那一份 ✓）。
    let text_key = canonical_text_key(text);
    let hash = judge_cache_key(&[identity]);
    let mut table = canonical_prefix_table()
        .lock()
        .expect("canonical prefix table");
    // 容量与 `canonical_prefix_cached` **同一个数** ✓（两处必须一致 ✗ —— 种进来的
    // 条目被读的那一侧挤掉，是这张表最隐蔽的失效方式 ✓）。**4096 太小** ✗：整本课程
    // 的工作集比它大 ⇒ **抖动** ✗（实测 `identity_parses=3062` ✗）⇒ 见那边的长注释 ✓。
    // 淘汰**只允许变慢** ✓、**必须可见** ✓（`identity_evictions()` ✓）。
    if table.len() >= 65536 {
        if let Some(victim) = table.keys().next().copied() {
            table.remove(&victim);
            stats::IDENTITY_EVICTIONS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
    }
    table.insert(text_key, hash);
}

fn judge_infer_key(
    extra_prefix: &str,
    prefix_src: &str,
    options: &CompileOptions,
    binders: &[GoalBinderSpec],
    term: &str,
) -> u64 {
    judge_infer_cache_key(extra_prefix, prefix_src, options, binders, term)
}

/// **G-85**：`judge_infer` 家族的**唯一**一把键 ✓（**慢路** + **就地路的查/写**都用它 ✓）。
///
/// **为什么要"唯一"** ✗（2026-10-04 实测定位 ✓）：先前 `judge_infer_cached`（慢路）
/// **自己内联**构造了一遍 ✓，`judge_infer_key`（就地路的查/写）又构造了一遍 ✓ ⇒
/// 两把键**不是同一把** ✗✗：
/// * 慢路的键里是**前缀原文** ✗ ⇒ 改一条**靠前**定理的**证明体**（类型一字不动）⇒
///   后面每条判定**全部 miss** ✗ ⇒ 各自重跑整份前缀 ✓ —— 这正是「只改证明，
///   后面不需要重编」要修的 ✓（实测 `prefix=8` ✗；规范化只加在 `judge_infer_key` 上时
///   **判据纹丝不动** ✗，正因为它没覆盖慢路 ✓）；
/// * 查/写的键里**没有 pp 标记** ✗ ⇒ 全显式 pp 与非全显式**互相命中** ✗（形态分叉 ✓）。
///
/// **键里放什么** ✓：前缀的**环境身份**（`compile::canonical_prefix_id` ✓ = 名字 + 类型；
/// `theorem`/`example` 的**证明体不进** ✓）+ 编译选项 + binders + 被问的**问题本身** +
/// pp 标记 ✓。**不放** ✗：前缀**原文** · 任何**字节偏移 / 源码位置** ✓。
#[track_caller]
fn judge_infer_cache_key(
    extra_prefix: &str,
    prefix_src: &str,
    options: &CompileOptions,
    binders: &[GoalBinderSpec],
    term: &str,
) -> u64 {
    // 两份前缀用**哈希**进键 ✓（`u64` ⇒ 每次判定只读 8 字节 ✓，不克隆身份文本 ✗）。
    let extra_hash = format!("{:016x}", canonical_prefix_cached(extra_prefix));
    let prefix_hash = format!("{:016x}", canonical_prefix_cached(prefix_src));
    judge_cache_key(&[
        &extra_hash,
        &prefix_hash,
        &options_key(options),
        &format!("{binders:?}"),
        term,
        // **G-71**：全显式 pp 与非全显式是**两份不同的文本** ⇒ 键必须分开，
        // 否则两条路互相命中、形态分叉 ✗。
        if explicit_pp_active() {
            "pp=explicit"
        } else {
            "pp=plain"
        },
    ])
}

/// **P1-b 第二刀（`by` 路径）的读数**：`(used, fallback)`。
///
/// 判据用法（`crates/front/tests/judge_inplace_by.rs`）：`used > 0` 证明
/// **就地路径真的走到了**（否则判据空转 ✗），`fallback` 只作参考。
pub fn inplace_by_report() -> (u64, u64) {
    stats::inplace_by()
}

/// **`by` 影子档的读数**：`(same, diff)`。`diff == 0` 是这一档能开的前提 ✓。
/// **E4 的判据读数**（2026-10-08）：影子档里因「prelude 安装期」**不比**而提前返回的
/// 次数 —— 撤掉那条排除分支（`course-stdlib.md` §7 的目标）之后**必须是 0** ✓。
pub fn inplace_shadow_prelude_excluded() -> u64 {
    stats::inplace_shadow_prelude_excluded()
}

pub fn inplace_by_shadow() -> (u64, u64) {
    stats::inplace_by_shadow()
}

/// **T3-B1 ① 的读数**（`cases` 被消去项的就地路）：`(used, fallback)`。
/// 判据用法：`used > 0` 证明这条接线**真的被走到**（否则判据空转 ✗）。
pub fn inplace_cases_report() -> (u64, u64) {
    stats::inplace_cases()
}

/// **T3-B1 ① 的影子档读数**：`(same, diff)`；`diff == 0` 是本档能开的前提 ✓。
pub fn inplace_cases_shadow() -> (u64, u64) {
    stats::inplace_cases_shadow()
}

/// **T3-B1 ③ 的读数**（`level_hint_of` 的就地路，含记法形态）：`(used, fallback)`。
/// 判据用法：`used > 0` 证明这条接线**真的被走到**（否则判据空转 ✗）。
pub fn inplace_level_hint_report() -> (u64, u64) {
    stats::inplace_level_hint()
}

/// **T3-B2 的读数**（合成趟）：`(趟数, Σ命令数, 回退趟数)`。
///
/// 判据用法（`PLAN-align-lean4.md` §4.3）：**先建这个读数**，再决定 T3-B2 开不开工
/// —— `prefix=` 只数 `judge_infer` ✗，不数 `judge_type_of`/`judge_pairs` 的合成趟 ✓。
pub fn synthesized_report() -> (u64, u64, u64) {
    stats::synthesized()
}

/// **T3-B1 · §4.2 第 5 条的读数**（记法目标签名的就地路）：`(used, fallback)`。
/// `used > 0` 证明这条接线**真的被走到**（否则判据空转 ✗）。
pub fn type_of_constant_report() -> (u64, u64) {
    stats::inplace_type_of_constant()
}

/// **同上 · 影子档读数**：`(same, diff)`；`diff == 0` 是本档能开的前提 ✓。
pub fn type_of_constant_shadow() -> (u64, u64) {
    stats::inplace_type_of_constant_shadow()
}

/// **诊断**：影子档第一条分叉的原样记录（只记第一条）。
pub fn type_of_constant_first_diff() -> Option<String> {
    stats::INPLACE_TOC_FIRST_DIFF
        .lock()
        .ok()
        .and_then(|g| g.clone())
}

/// **P1-a 就地判定的读数**（集成测试 / 诊断用；进程级，见 [`stats::inplace`]）：
/// `(used, fallback, shadow_same, shadow_diff)`。
///
/// 判据用法（`crates/front/tests/judge_inplace.rs`）：
/// `shadow_same > 0` 证明**就地路径真的走到了**（否则判据空转 ✗ ——
/// "咬不住的守卫等于没有"），`shadow_diff == 0` 证明两条路**文本逐字节相同** ✓。
pub fn inplace_report() -> (u64, u64, u64, u64) {
    stats::inplace()
}

/// `SOKO_NO_JUDGE=1`：**测量专用**开关（跳过判定、一律答"过"）。
/// 见 [`judge_terms_with`] 里的注释 —— **绝不许进判定路径**。
fn no_judge() -> bool {
    static OFF: OnceLock<bool> = OnceLock::new();
    *OFF.get_or_init(|| std::env::var("SOKO_NO_JUDGE").is_ok())
}

/// `SOKO_NO_JUDGE_BATCH=1` 强制关掉乐观批处理（**对拍用**：开与关必须给出
/// 逐字相同的结论与诊断）。只读一次环境（进程级开关）。
fn batching_on() -> bool {
    static ENV_OFF: OnceLock<bool> = OnceLock::new();
    let off = *ENV_OFF.get_or_init(|| std::env::var("SOKO_NO_JUDGE_BATCH").is_ok());
    !off && BATCHING.load(std::sync::atomic::Ordering::Relaxed)
}

/// 判定**文档走查次数**（`cfg(test)` 计数用）。
///
/// 每一次 `judge_pairs_uncached` 都要把整份前缀重跑一遍——这是"判定不逐步做"
/// 之后唯一剩下的 O(前缀) 成本，所以它是性能回归最灵敏的指标：一个 `by` 块里
/// 的 N 次判定，开批处理应当是 **1** 次走查，关掉是 N 次。
#[cfg(test)]
pub(crate) fn pass_count() -> usize {
    PASSES.with(|c| c.get())
}

#[cfg(test)]
pub(crate) fn reset_pass_count() {
    PASSES.with(|c| c.set(0));
}

#[cfg(test)]
thread_local! {
    static PASSES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

thread_local! {
    /// 当前活跃的乐观批次（`None` = 没有批次，判定立即执行）。
    ///
    /// 用 thread-local 而不是把 `&mut Batch` 一路穿过 `run_tactics` /
    /// `exact_tactic` / `cases_tactic`：那些函数已经在 `clippy::too_many_arguments`
    /// 的边上（8 个参数），再加一个只会让每次判定多一层间接。
    static BATCH: std::cell::RefCell<Option<Vec<BatchItem>>> =
        const { std::cell::RefCell::new(None) };
}

/// 开一个乐观批次：期间 [`judge_terms`] **只记录不判**，一律返回 `Match`。
///
/// 语义靠 [`flush_batch`] 兜底：记录下来的每一对都真的判一遍，只要有**一条**
/// 不是 `Match`，调用方（`by::run_by`）就丢掉这一趟的结果、改用逐条判定的
/// **严格重跑**——所以失败路径的诊断与改动前逐字相同。
#[must_use = "批次必须 flush（或者显式 drop）；见 flush_batch"]
pub(crate) struct BatchScope {
    /// 外层批次（嵌套时的栈式恢复：flush 期间跑的嵌套 pass 会开自己的批次）。
    outer: Option<Vec<BatchItem>>,
}

pub(crate) fn begin_batch() -> BatchScope {
    let outer = BATCH.with(|b| b.borrow_mut().take());
    BATCH.with(|b| *b.borrow_mut() = Some(Vec::new()));
    BatchScope { outer }
}

impl BatchScope {
    /// 取走这一批记录（不判定）。调用方随后用 [`judge_pairs_with`] 判。
    fn take(&self) -> Vec<BatchItem> {
        BATCH.with(|b| b.borrow_mut().take()).unwrap_or_default()
    }
}

impl Drop for BatchScope {
    fn drop(&mut self) {
        // 恢复外层批次（嵌套 pass 结束后，外层继续记录）。
        let outer = self.outer.take();
        BATCH.with(|b| *b.borrow_mut() = outer);
    }
}

/// **B3（2026-10-08）**：当前批次的**条数** —— 顶层 tactic 用它记边界
/// （`by.rs` 的 `marks`：每条 tactic **开跑前**记一格 `(下标, 批长)` ✓）。
pub(crate) fn batch_len() -> usize {
    BATCH.with(|b| b.borrow().as_ref().map_or(0, |items| items.len()))
}

/// 把一批记录判掉，返回**首个非 `Match` 的下标**（`None` = 全 `Match`）。
///
/// **B3 起返回值从 `bool` 变成 `Option<usize>`**：调用方（`by::run_by`）需要知道
/// "第一个翻车的是哪一条" —— 如果它属于**失败的那条 tactic**，前几条在两条路下
/// 控制流相同（判定全是 `Match` ✓）⇒ 可以**从失败点续跑**，不必整段重放 ✓；
/// 若它属于**更早**的 tactic，严格路会在那里就失败 ⇒ 续跑会给出不同诊断 ✗
/// ⇒ 调用方退回"整段严格重跑"（今天那条路）✓。
pub(crate) fn flush_batch(scope: BatchScope) -> Option<usize> {
    let items = scope.take();
    drop(scope);
    if items.is_empty() {
        return None;
    }
    // 一批里的 `(前缀, 选项)` 恒相同（同一个 `by` 块），但按 key 分组更稳：
    // 分组键变了就分开判，绝不把不同前缀的判定混进同一份文档。
    let mut first_bad: Option<usize> = None;
    let mut start = 0usize;
    while start < items.len() {
        let key = (
            items[start].extra_prefix.clone(),
            items[start].prefix_src.clone(),
            options_key(&items[start].options),
        );
        let mut end = start + 1;
        while end < items.len()
            && items[end].extra_prefix == key.0
            && items[end].prefix_src == key.1
            && options_key(&items[end].options) == key.2
        {
            end += 1;
        }
        let pairs: Vec<JudgePair> = items[start..end]
            .iter()
            .map(|item| JudgePair {
                spec: item.spec.clone(),
                term: item.term.clone(),
            })
            .collect();
        let judgements = judge_pairs_with(
            &items[start].extra_prefix,
            &items[start].prefix_src,
            &items[start].options,
            &pairs,
        );
        if first_bad.is_none() {
            if let Some(offset) = judgements
                .iter()
                .position(|j| !matches!(j, Judgement::Match))
            {
                first_bad = Some(start + offset);
            }
        }
        start = end;
    }
    first_bad
}

/// **一个项的类型文本**（不合成 lambda、不剥 binder）：合成 `#check <term>`
/// 走完整流水线，取 `TypeChecked` 的 `inferred_type`。
///
/// 用途：记法展开要读**目标常量的签名**（`Set.powerset : (α : Type) → Set α →
/// Set (Set α)`）。[`judge_infer`] 那条路要先合成 `fun (binders) => term` 再逐层
/// 剥 binder，目标**本身是函数**时内核 pp 的多 binder 折叠会让剥离结果错位
/// （第二刀实测：`Set.image` 的签名被剥成
/// `(β : Sort 1) -> … -> (α : Sort 1) (β : Sort 1) -> …`，`''` 的前导参数
/// 永远解不出）。这里直接问，不剥——签名是常量自己的，与调用点的 binder 无关。
pub fn judge_type_of(
    prefix_src: &str,
    options: &CompileOptions,
    term: &str,
) -> Result<String, Judgement> {
    let key = judge_cache_key(&[prefix_src, &options_key(options), "type-of", term]);
    if let Some(JudgeCacheValue::Infer(r)) = type_cache_get(key) {
        return r;
    }
    let r = judge_type_of_uncached(prefix_src, options, term);
    type_cache_put(key, JudgeCacheValue::Infer(r.clone()));
    r
}

/// 记法目标的**签名缓存**（与 [`judge_type_of`] 分开，见
/// `docs/design/course-lean-style.md` §9「判卷成本」）。
///
/// 为什么需要：`elab_notation` 每展开一个符号都要问一次内核「目标常量的类型」，
/// 而 [`judge_type_of`] 的缓存键**含整段前缀**——同一份文件里前缀随每条声明
/// 增长 ⇒ 每条声明的每个记法都命中不了缓存，退化成**全前缀重编译**，总量 O(n²)
/// （实测：课程 Lean 化之后，8 模块闭包从 1.3s 涨到 11.5s，单元解答从秒级涨到
/// 分钟级）。
///
/// 常量的签名与「谁在用它」无关（名字唯一且单调增长），所以这里的键只有
/// （选项, 规范名）。**只缓存成功**：失败照旧走原路（那可能只是"还没声明"）。
pub fn judge_type_of_constant(
    prefix_src: &str,
    options: &CompileOptions,
    name: &str,
) -> Result<String, Judgement> {
    static CACHE: OnceLock<Mutex<HashMap<String, Result<String, Judgement>>>> = OnceLock::new();
    const CAP: usize = 4096;
    let key = format!("{}|{name}", options_key(options));
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(hit) = cache.lock().ok().and_then(|c| c.get(&key).cloned()) {
        return hit;
    }
    let result = judge_type_of(prefix_src, options, name);
    if result.is_ok() {
        if let Ok(mut c) = cache.lock() {
            if c.len() < CAP {
                c.insert(key, result.clone());
            } else {
                // **闸类出口**（G-91 乙类 ✓）：`CAP = 4096` 满 ⇒ 这条**静默**不写
                // ⇒ 下次同一个名字还要再走一遍全前缀重编译 ✓ —— **只变慢** ✓
                //（结论一字不变 ✓），但表满了之后**一个字节都不再长** ⇒ 没有出口
                // 就分不出"缓存正常"与"缓存已饱和" ✗（`hits/misses` 也分不出 ✓）。
                sokonanoda::gates::CONST_SIG_CACHE_FULL.bump();
            }
        }
    }
    result
}

/// **T3-B1 · §4.2 第 5 条（2026-10-09）**：记法目标签名的**就地**版。
///
/// `elab_notation` 每展开一个记法符号都要问一次「目标常量的类型」
/// （[`judge_type_of_constant`] ⇒ `judge_type_of` ⇒ **合成 `#check` + 整份前缀
/// 从零重跑** ✗）。常量就在**活环境**里（它刚被 elaborate 过）⇒ 不必重跑 ✓。
///
/// 就地路 = 把常量名当 `Expr::Ident` 在活环境上 elaborate，再走
/// [`crate::compile::elab::infer_type_text_inplace`] 的内核 pp —— 它设的
/// `proofs = true` 与 `explicit_pp_active()` 与 `#check` 出口**同款** ✓
/// ⇒ 文本与慢路**同源**。答不出 / 开关关着 ⇒ `None`，调用方**原样**回落慢路 ✓。
///
/// ⚠ **文本必须与慢路逐字节相同**：签名文本会被回读成记法目标 —— 文本分叉
/// 就是 **elaborate 分叉** ✗ ⇒ 影子档逐条比对（见 [`type_of_constant_prefer_inplace`]）。
fn judge_type_of_constant_inplace(
    known: &crate::compile::elab::KnownTable,
    name: &str,
) -> Option<String> {
    if crate::judge::inplace_mode() == InplaceMode::Off {
        return None;
    }
    // **与慢路同源**：`#check <裸常量>` 的渲染走 `walk::check` 的 `sig` 快路
    // （`known.get(name).signature()` ✓，U2）⇒ 这里**读同一张表**、取**同一个字段**
    // ⇒ 文本**构造性相同** ✓（不必再 elaborate、不碰内核 pp ✗）。
    known
        .get(name)
        .and_then(|info| info.signature())
        .map(str::to_string)
}

/// **记法目标签名的档位入口**（T3-B1 · §4.2 第 5 条）：On 就地优先、Shadow 两条都跑、
/// Off 原样走 [`judge_type_of_constant`]。
///
/// ⚠ Shadow 档**返回慢路那一份** ⇒ 行为零变化、只取证 ✓（同 A2a 的档位纪律）。
pub(crate) fn type_of_constant_prefer_inplace(
    known: &crate::compile::elab::KnownTable,
    prefix_src: &str,
    options: &CompileOptions,
    name: &str,
) -> Result<String, Judgement> {
    match crate::judge::inplace_mode() {
        InplaceMode::Off => judge_type_of_constant(prefix_src, options, name),
        InplaceMode::On => match judge_type_of_constant_inplace(known, name) {
            Some(text) => {
                stats::INPLACE_TOC_USED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                Ok(text)
            }
            None => {
                stats::INPLACE_TOC_FALLBACK.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                judge_type_of_constant(prefix_src, options, name)
            }
        },
        InplaceMode::Shadow => {
            let fast = judge_type_of_constant_inplace(known, name);
            let slow = judge_type_of_constant(prefix_src, options, name);
            let same = matches!((&fast, &slow), (Some(t), Ok(s)) if t == s);
            if same {
                stats::INPLACE_TOC_SHADOW_SAME.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            } else {
                stats::INPLACE_TOC_SHADOW_DIFF.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                if let Ok(mut first) = stats::INPLACE_TOC_FIRST_DIFF.lock() {
                    if first.is_none() {
                        *first = Some(format!(
                            "type-of-constant {name:?} | inplace={fast:?} | slow={slow:?}"
                        ));
                    }
                }
            }
            slow
        }
    }
}

fn judge_type_of_uncached(
    prefix_src: &str,
    options: &CompileOptions,
    term: &str,
) -> Result<String, Judgement> {
    let mut src = String::from(prefix_src);
    let query_start = src.len();
    src.push_str("#check ");
    src.push_str(term);
    src.push('\n');
    // 片段模式（G-05 §4.1）：前缀可能停在未闭合的 `namespace` 里。
    let Ok(file) = crate::parse_fragment(&src) else {
        return Err(Judgement::Error {
            code: "parse".to_string(),
            message: "无法解析类型查询".to_string(),
        });
    };
    // **G-92 真修**：同 `judge_infer_uncached` ✓ —— 受信任前缀那条路 ✓
    // （前缀已核过 ⇒ 不重编 ✗；没有受信任前缀 ⇒ 逐字回退到 `compile_fol_with` ✓）。
    let prefix_commands = file.commands.len().saturating_sub(1);
    let out = match run_synthesized_incremental(
        &file,
        options,
        prefix_commands,
        prefix_opaque_mode() == PrefixOpaqueMode::On,
    ) {
        Some((out, _report, _before)) => out,
        None => {
            note_synthesized_fallback();
            compile_fol_with(&file, options)
        }
    };
    if let Some(err) = query_error(query_start, &out.errors) {
        return Err(err);
    }
    let last_cmd = file.commands.len().checked_sub(1);
    pick_type_checked(&out, last_cmd).ok_or_else(|| {
        prefix_error(&out.errors).unwrap_or(Judgement::Error {
            code: "judge-infer-none".to_string(),
            message: "内核未返回类型".to_string(),
        })
    })
}

/// 推断 `term` 在 `binders` 语境下的**类型文本**（kernel 判定驱动，供
/// `apply` 读取被应用函数的类型）。合成 `<prefix>\n#check fun <binders> =>
/// <term>\n` 走完整流水线，取 `TypeChecked` 事件文本，再剥掉 n 层
/// binder 箭头得 `term` 的类型。
#[track_caller]
pub fn judge_infer(
    prefix_src: &str,
    options: &CompileOptions,
    binders: &[GoalBinderSpec],
    term: &str,
) -> Result<String, Judgement> {
    judge_infer_with("", prefix_src, options, binders, term)
}

thread_local! {
    /// **G-71（0.81.0）**：本次判定查询要不要**全显式 pp**。
    ///
    /// 背景：tactic 上下文的 binder 类型是**文本**（`#check fun (x : T) => …` 的
    /// 结果），而内核 pp **只丢第一个隐式实参**（`Set.image {α β} (f) (A)` ⇒
    /// `Set.image β f A`）—— 这份文本再被回读时，嵌套的记法/应用**对不上号**
    /// （实测：`(β ⁻¹' f) C` 回读成 `Set.preimage β f C` ⇒ `exact` 判红）。
    /// `explicit = true` 时 pp 打出 `@Set.image α β f A`（**每个实参都写出来**）
    /// ⇒ 文本**完整可回读** ✓。
    ///
    /// ⚠ 旗标**只在这两个 explicit 入口里打开**（`cases` 的被消去项、
    /// `canonical_goal_type` 的规范目标），出口立刻还原 ⇒ 其它任何 `#check`
    /// 的输出**逐字节不变** ✓。
    static EXPLICIT_PP: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// 全显式 pp 是否打开（`compile/check/kernel_phase.rs` 的 `#check` 出口读它）。
pub(crate) fn explicit_pp_active() -> bool {
    EXPLICIT_PP.with(|c| c.get())
}

/// RAII：进时置位、出时**还原**（panic 也还原 ✓）。
struct ExplicitPpGuard(bool);

impl ExplicitPpGuard {
    fn new(on: bool) -> Self {
        Self(EXPLICIT_PP.with(|c| c.replace(on)))
    }
}

impl Drop for ExplicitPpGuard {
    fn drop(&mut self) {
        EXPLICIT_PP.with(|c| c.set(self.0));
    }
}

/// **全显式**版 [`judge_infer`]（G-71）：返回的文本里**每个实参都写出来**
/// （`@Set.preimage α β f C`）⇒ **完整可回读** ✓ —— tactic 上下文要把这份文本
/// 再回读成项，非全显式的 pp 文本会错位 ✗。
pub fn judge_infer_explicit(
    prefix_src: &str,
    options: &CompileOptions,
    binders: &[GoalBinderSpec],
    term: &str,
) -> Result<String, Judgement> {
    let _guard = ExplicitPpGuard::new(true);
    judge_infer_with("", prefix_src, options, binders, term)
}

/// **T3-B1 ①（2026-10-08）**：[`judge_infer_explicit`] 的**就地兄弟** —— 与慢路读
/// **同一个** `EXPLICIT_PP` 线程局部（[`explicit_pp_active`]，`elab.rs` 的
/// `infer_type_text_inplace` 第 ④ 步也读它）⇒ 两条路的 pp 形态同源 ✓。
///
/// 存在的理由与 [`judge_render_type_inplace_with_explicit`] 完全同款：`cases` 的
/// **被消去项**是一个局部假设名，它的类型**就在活环境里** —— 不必合成 `#check`
/// 文档、从零重跑整份前缀 ✗（那是 `judge_infer_explicit` 今天做的事）。
///
/// `ExplicitPpGuard` 是私有的 ⇒ 这里开**受控入口**（RAII 在函数内收口，
/// 调用方不可能忘记还原 ✓）。答不出 ⇒ `None`，调用方**原样**回落慢路 ✓。
pub(crate) fn judge_infer_inplace_with_explicit<'a>(
    explicit: bool,
    env: &mut crate::compile::elab::InplaceEnv<'_, 'a>,
    ctx: &crate::compile::elab::ElabCtx<'a, '_>,
    binder_srcs: &[(String, Expr)],
    operand: &Expr,
) -> Option<String> {
    let _guard = ExplicitPpGuard::new(explicit);
    crate::compile::elab::infer_type_text_inplace(
        env,
        ctx,
        binder_srcs,
        operand,
        binder_srcs.len(),
        None,
    )
    .ok()
}

/// **全显式**版 [`judge_render_type`]（G-71）：同 [`judge_infer_explicit`]，
/// 但收的是"把 `ty` 渲染成规范文本"那条路（`canonical_goal_type` 用）。
/// ⚠ **不走就地快路**：就地路的内核 pp 用的是默认选项 ⇒ 文本形态会与这条分叉 ✗。
pub fn judge_render_type_explicit(
    prefix_src: &str,
    options: &CompileOptions,
    binders: &[GoalBinderSpec],
    ty: &str,
) -> Option<String> {
    let _guard = ExplicitPpGuard::new(true);
    judge_render_type(prefix_src, options, binders, ty)
}

/// 同 [`judge_infer`]，但把 `extra_prefix`（闭包上下文）拼在文档前缀之前。
#[track_caller]
/// **带环境提供方的判定入口** ✓（本片第 2 步 ✓，设计 `incremental-environment.md` §2/§5 ✓）。
///
/// **契约（三条，缺一不算成立 ✓）**：
/// 1. `provider = None` ⇒ **逐字节回退**到今天的行为 ✓（合成前缀 + 重跑 ✓）
///    —— 这既是**回退机制**，也是**判据之一** ✓；
/// 2. `provider = Some(p)` 且 `p` 答得上 ⇒ **直接返回它给的类型文本** ✓（**不再合成前缀** ✓）；
/// 3. `p` 答不上（`None`）或**输入解析不出来** ⇒ 同样回退 ✓（**不许猜** ✗）。
///
/// ⚠ **为什么是"加法式"新函数** ✗→✓：`judge_infer_with` 的调用点很多 ✓ ⇒ 直接改签名会让
/// 整条链都要动 ✗；新函数让**接线**与**实现 provider** 可以**各自独立落地** ✓
/// （本步 = 只把入口建好 + 用假 provider 钉住形状 ✓，真 provider 在 `elab.rs` 那条线收口后接 ✓）。
///
/// ⚠ **文本 ⇒ AST 是本函数自己做的** ✓（设计 §0.2 #1 的「出路 ①」✓）：复用现成的
/// `synthesized_check_term`（它已经把 `binders` 合成 `fun (b1 : T1) => <term>` ✓）
/// ⇒ 解析一次、拆出 `(名字, 源类型)` 对 + 操作数 ✓。
pub fn judge_infer_with_env(
    provider: Option<&dyn EnvProvider>,
    extra_prefix: &str,
    prefix_src: &str,
    options: &CompileOptions,
    binders: &[GoalBinderSpec],
    term: &str,
) -> Result<String, Judgement> {
    if let Some(provider) = provider {
        if let Some((binder_srcs, operand)) = provider_inputs(binders, term) {
            if let Some(text) = provider.infer_type_text(&binder_srcs, &operand) {
                return Ok(text);
            }
        }
    }
    judge_infer_with(extra_prefix, prefix_src, options, binders, term)
}

/// 把 `binders` + `term` 变成 provider 要的**源 AST 形** ✓（`(名字, 源类型)` 对 + 操作数 ✓）。
///
/// 复用 `synthesized_check_term` 的合成形状（`fun (b1 : T1) … => <term>` ✓）⇒ 解析后拆开 ✓；
/// **任一步失败就 `None`** ✓（调用方据此回退 ✓，**不许猜** ✗）。
fn provider_inputs(binders: &[GoalBinderSpec], term: &str) -> Option<(Vec<(String, Expr)>, Expr)> {
    let query = synthesized_check_term(binders, term).ok()?;
    // ⚠ 合成的是**裸项**（`fun (b1 : T1) … => <term>` ✓）⇒ 要包成 `#check` 才**解析得成文件** ✓
    // （`crate::parse` 收的是文件 ✓；这一步失败就 `None` ⇒ 回退 ✓）。
    let file = crate::parse(&format!("#check {query}\n")).ok()?;
    let expr = file.commands.iter().find_map(|command| match command {
        Command::Check { expr, .. } => Some(expr),
        _ => None,
    })?;
    let mut binder_srcs = Vec::with_capacity(binders.len());
    let operand = match expr {
        // ⚠ **合成的是 λ 不是 ∀** ✗→✓（实测 ✓）：`synthesized_check_term` 给的是
        // `fun (b1 : T1) => <term>` ✓ ⇒ 解析出来是 **`Expr::Lambda`** ✓；写 `Forall`
        // 会让这个分支**永不命中** ⇒ `binder_srcs` 恒空 ✗ —— 判据
        // `env_provider_is_consulted_and_none_falls_back_byte_for_byte` 第一次跑就抓到了 ✓
        // （这正是"先建判据"的价值 ✓）。
        Expr::Lambda {
            binders: bs, body, ..
        } => {
            // **无类型 binder 直接跳过** ✓ —— 与 as-built 的 `judge_binder_srcs()`
            // （`elab.rs:577` ✓ 的 `.filter_map(... src.as_ref().map(...))` ✓）**同一口径** ✓。
            for binder in bs {
                if let Some(ty) = &binder.ty {
                    binder_srcs.push((binder.name.clone(), (**ty).clone()));
                }
            }
            // ⚠ 必须写 **`Expr::clone(body)`** ✗→✓：`(*body).clone()` 会被**自动解引用**解析成
            // `Box::clone` ⇒ 类型仍是 `Box<Expr>` ✗（实测卡了一轮 ✓）；显式写 `Expr::clone`
            // 让 `&Box<Expr>` 走 **deref coercion** 变成 `&Expr` ✓。
            Expr::clone(body)
        }
        other => other.clone(),
    };
    Some((binder_srcs, operand))
}

pub fn judge_infer_with(
    extra_prefix: &str,
    prefix_src: &str,
    options: &CompileOptions,
    binders: &[GoalBinderSpec],
    term: &str,
) -> Result<String, Judgement> {
    let t0 = std::time::Instant::now();
    // ⚠ **这里也必须装打印机**（2026-09-28 修 ✓）：`install_printer` 原来只在
    // `judge_pairs_uncached`（**`by` 路径**）里调 ⇒ 项风格之后 `by` 调用数归零
    // ⇒ `report()` 里那句 `calls == 0 → return` **直接早退** ⇒ **`JUDGE_INFER`
    // 一行都不打** ✗ —— 而 `JUDGE_INFER` 恰恰是项风格下的**唯一大头**
    //（`docs/PERF.md`：`calls=126105 total_ms=12304`）⇒ 量具等于失效 ✗。
    // 它本身是 `call_once`（幂等 ✓、非热路径 ✓），两处都调是安全的 ✓。
    stats::install_printer();
    // **测量开关**（与 `judge_terms_with` 的 `no_judge()` 同一个）：`judge_infer`
    // 占 `judge_ms` 的**大头**（真课程实测：只跳 `judge_terms` ⇒ 218.8s → 221.7s，
    // **只有 3%** ✗；跳掉 `judge_infer` 才看得见另一半）。⚠ 同样**只许量成本**。
    if no_judge() {
        return Err(Judgement::Error {
            code: "skipped".to_string(),
            message: "SOKO_NO_JUDGE=1（测量专用）".to_string(),
        });
    }
    let r = judge_infer_cached(extra_prefix, prefix_src, options, binders, term);
    stats::INFER_CALLS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    stats::INFER_NANOS.fetch_add(
        t0.elapsed().as_nanos() as u64,
        std::sync::atomic::Ordering::Relaxed,
    );
    if r.is_err() {
        stats::INFER_FAILS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
    r
}

#[track_caller]
fn judge_infer_cached(
    extra_prefix: &str,
    prefix_src: &str,
    options: &CompileOptions,
    binders: &[GoalBinderSpec],
    term: &str,
) -> Result<String, Judgement> {
    let t0 = std::time::Instant::now();
    // **G-85**：用**唯一**那把键 ✓ —— 先前这里是**内联的第二份** ✗（原文 + pp 标记 ✓），
    // 与 `judge_infer_key`（就地路的查/写）**不是同一把** ✗ ⇒ 改证明体会让这里全 miss ✗。
    let key = judge_infer_cache_key(extra_prefix, prefix_src, options, binders, term);
    stats::KEY_NANOS.fetch_add(
        t0.elapsed().as_nanos() as u64,
        std::sync::atomic::Ordering::Relaxed,
    );
    // **P1-a 量具**（只计数、不改判定 ✓）：这一趟是不是"裸常量"查询？
    // 先算分类，命中与未命中都要记 —— 因为**可优化的只有未命中**那部分。
    let classify = std::env::var_os("SOKO_JUDGE_CLASSIFY").is_some();
    let bare = classify && stats::looks_like_bare_const(term);
    if classify {
        stats::CLASSIFY_CALLS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if bare {
            stats::CLASSIFY_BARE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            // 弱信号：名字在**前缀文本**里能原样找到 ⇒ 它来自前缀，大概率可查表。
            let short = match term.find(".{") {
                Some(i) => &term[..i],
                None => term,
            };
            if prefix_src.contains(short) {
                stats::CLASSIFY_RESOLVABLE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
        }
    }
    if let Some(JudgeCacheValue::Infer(r)) = judge_cache_get(key) {
        stats::INFER_HITS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        stats::HIT_NANOS.fetch_add(
            t0.elapsed().as_nanos() as u64,
            std::sync::atomic::Ordering::Relaxed,
        );
        return r;
    }
    // **P1-a**：所有未命中都记（`bare` 只是其中一类）—— 要能回答
    // "**是不是只有裸常量那类才贵**"。
    let miss_t0 = std::time::Instant::now();
    // **P1-a 分布量具**：未命中按 `term` 前缀长度分桶累计（回答"贵的那些长什么样"）。
    let term_len = term.len();
    let bucket = if term_len < 16 {
        0
    } else if term_len < 48 {
        1
    } else if term_len < 160 {
        2
    } else {
        3
    };
    if classify {
        stats::CLASSIFY_ALL_MISS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if bare {
            stats::CLASSIFY_BARE_MISS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
    }
    let miss = stats::INFER_MISSES.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
    // 常驻诊断（`SOKO_INFER_TRACE=<n>[,<n>…]|all`）：打出每次未命中的查询与
    // 前缀长度；**在号上打调用栈**（`SOKO_INFER_TRACE=100` 就给第 100 次的栈）。
    // 未命中一次 = 全前缀重编译一趟 pass ⇒ 这几行直接指认"谁在重编译"。
    if let Some(spec) = infer_trace_spec() {
        let bt = if spec == "all" || spec.split(',').any(|t| t.trim() == miss.to_string()) {
            format!("\n{}", std::backtrace::Backtrace::force_capture())
        } else {
            String::new()
        };
        eprintln!(
            "INFER_MISS #{} prefix={} binders={} term={} at={}:{}{}",
            miss,
            prefix_src.len(),
            binders.len(),
            term.chars().take(60).collect::<String>(),
            std::panic::Location::caller().file(),
            std::panic::Location::caller().line(),
            bt
        );
    }
    let r = judge_infer_uncached(extra_prefix, prefix_src, options, binders, term);
    // **P1-a**：未命中的**完整**耗时（重跑前缀那段）——这才是可就地消掉的部分。
    if classify {
        let dt = miss_t0.elapsed().as_nanos() as u64;
        stats::CLASSIFY_BUCKET_N[bare as usize][bucket]
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        stats::CLASSIFY_BUCKET_NS[bare as usize][bucket]
            .fetch_add(dt, std::sync::atomic::Ordering::Relaxed);
        stats::CLASSIFY_ALL_MISS_NANOS.fetch_add(dt, std::sync::atomic::Ordering::Relaxed);
        if bare {
            stats::CLASSIFY_BARE_MISS_NANOS.fetch_add(
                miss_t0.elapsed().as_nanos() as u64,
                std::sync::atomic::Ordering::Relaxed,
            );
        }
    }
    judge_cache_put(key, JudgeCacheValue::Infer(r.clone()));
    r
}

/// **当前 pass 的只读环境视图**（设计 `docs/design/incremental-environment.md` §2）。
///
/// ⚠⚠ **本 trait 至今零实现、零接线** ✗（2026-10-05 对账 ✓）：全仓只有本处定义与
/// 3 处注释引用，**没有任何 `impl`**。**as-built 的接口不是它** —— P1-a/P1-b 实际走
/// `compile/elab.rs` 的 `InplaceEnv`（活 `&mut EnvBuilder` + `KnownTable`）+
/// `infer_type_text_inplace`（**按源 AST 而不是文本**，且**在调用点**做，不进
/// `judge_infer`）⇒ 见设计 **§0.2 #2 与 §32**（含"为什么不是这个形状"的三条实测）。
/// **下一步二选一**：按 as-built 重写本 trait 并接线，或删掉它（台账 G-92 的
/// `expected_lean` 引用了它 ⇒ 删之前先改台账）。
///
/// **为什么需要它**：`judge_infer` 今天只拿到 `prefix_src: &str` ⇒ 只能把**整段前缀**
/// 合成一份文件、交 `check_document_with` **从零重跑一趟 pass** ✗。实测（真课程）：
/// judge 占墙钟 **≈88%**（219.3s → 跳掉后 **26.8s**）、合成 pass **253513** 次
/// = 自身声明事件（2647）的 **95.8×**，且成本**随声明在文件里的序号线性增长**
/// （41 模块里 20 个 r>0.5、均值 +0.44）⇒ **O(N²)**。
///
/// 机理：缓存键含**整段前缀的哈希** ⇒ 前缀随序号变长 ⇒ 后段全 miss ⇒ 前缀从零重跑。
///
/// **实现方**：`Walk`（`compile/check/walk.rs`）—— 它在 walk 期间**无条件**
/// `add_declar`（9 处）⇒ 环境里**已经有到当前命令为止的声明** ✓。
///
/// **`None` 的语义**：没有环境（单文件/测试路径）⇒ **逐字节回退到今天的行为**
/// （合成前缀 + 重跑）。这是**回退机制**，也是判据之一 ✓。
///
/// ⚠ **签名已按 as-built 对齐**（2026-10-05 ✓，设计 `docs/design/incremental-environment.md`
/// §0.2 #2 + §32 ✓）：as-built 的机器是 **`InplaceEnv`**（活 `&mut EnvBuilder` + `KnownTable`）
/// + **`infer_type_text_inplace`**（`compile/elab.rs` ✓，**收 `(名字, 源类型)` 对 + `operand: &Expr`** ✓）
///   —— 旧签名收 `term: &str` 是**文本形** ✗，与它**接不上** ✓（这正是它至今零接线的原因之一 ✓）。
///
/// **借用形态（⚠ 2026-10-05 更正 ✓ —— 我先前写在这里的「`&self` + 实现方 `RefCell`」是错的 ✗）**：
/// `infer_type_text_inplace`（`compile/elab.rs:3016` ✓）**真的要改 builder** ✗ ——
/// `elab_expr(env.builder, …)` ✓ · `env.builder.mk_lambda(…)` ✓ · `env.builder.with_env(|ef| …)` ✓
/// ⇒ **`&mut` 是真需求** ✗，不是签名保守 ✓。而判定调用发生在**调用方已持有 `&mut builder`** 的深处 ✓
/// ⇒ 那一刻**没有任何地方能塞进 `RefCell`** ✗ ⇒ `&self` + 内部可变性**结构上不可能** ✗。
///
/// ✅ **正解 = 重借链** ✓：`InplaceEnv::reborrow`（`elab.rs:2851` ✓，注释就是为循环重借写的 ✓）
/// ⇒ 把 `&mut InplaceEnv` **顺着调用链透传**到判定点 ✓（= 设计 §6 的「`ElabCtx` 加 `env_view` +
/// 各处透传」✓，**链宽但每处只加一个参数** ✓）。
/// ⚠ ⇒ **本 trait 的形状本身还要重设计** ✗：要么改成**闭包式**接口 ✓
/// （设计 §2 的 `with_project_session` 同款 ✓：在借出窗口内回调 ✓），要么**不用 trait** ✓、
/// 把判定点直接放进 walk 的借出窗口 ✓。**先定这个，再写接线代码** ✓。
pub trait EnvProvider {
    /// 在**当前环境**上求 `operand` 在 `binder_srcs` 语境下的类型文本（与今天 `#check` 同形）。
    ///
    /// `None` ⇒ 这条环境答不了（调用方**必须**回退到合成前缀那条路，**不许猜** ✗）。
    fn infer_type_text(&self, binder_srcs: &[(String, Expr)], operand: &Expr) -> Option<String>;
}

/// `SOKO_INFER_TRACE` 的取值（读一次就缓存——它在热路径上）。
fn infer_trace_spec() -> Option<&'static str> {
    static SPEC: OnceLock<Option<String>> = OnceLock::new();
    SPEC.get_or_init(|| std::env::var("SOKO_INFER_TRACE").ok())
        .as_deref()
}

/// 合成查询项 `fun (b1 : T1) (b2 : T2) => <term>` 的**唯一实现**。
///
/// **为什么单独抽出来**（P1-a，2026-09-29）：判定有两条路 ——
/// ① **源码重跑**（今天）：把它拼成 `#check <它>` 交内核从零编前缀；
/// ② **就地**（P1-a 新增）：把它 `parse_expr_text` 回一个项，在**活环境**上
/// elaborate（`elab.rs::infer_type_text_inplace`）。
/// 两条路必须给出**逐字节相同**的文本 ⇒ 项形态由这一处决定，
/// **不可能分叉** ✓（附二 D1/D2 的风险就靠这条挡住）。
///
/// `Err` = 某个 binder 缺类型标注（今天的 `elab-untyped-binder`，一个字都不改）。
pub(crate) fn synthesized_check_term(
    binders: &[GoalBinderSpec],
    term: &str,
) -> Result<String, String> {
    let mut text = String::from("fun ");
    for b in binders {
        match &b.ty {
            Some(ty) => text.push_str(&format!("({} : {}) ", b.name, ty)),
            None => {
                return Err(format!("binder `{}` 缺少类型标注，无法推断", b.name));
            }
        }
    }
    text.push_str("=> ");
    text.push_str(term);
    Ok(text)
}

/// 剥掉 `fun (b1:T1) => … => <codomain>` 的 n 层 binder 箭头（**两条路共用的唯一实现**）。
///
/// pp 可能把相邻 binder 折叠成 `forall (a b : Prop), …`（一个 Forall 多
/// binder），所以逐 **单个** binder 剥；余下重渲染成可回读的单箭头链。
///
/// ⚠ **就地路径必须复用它**（不许自己再写一套剥法）：只要两边喂进来的类型文本
/// 相同，剥法相同 ⇒ 交出去的文本**逐字节相同** ✓。
pub(crate) fn peel_binders(ty: String, n: usize) -> String {
    let mut t = ty;
    for _ in 0..n {
        let Ok(e) = parse_expr_text(&t) else {
            break;
        };
        match peel_one_binder(&e) {
            Some(rest) => t = render_roundtrip(&rest),
            None => break,
        }
    }
    t
}

#[track_caller]
/// **构建身份** ✓（`AGENTS.md` 2026-10-04 值守拍板「探针读数必须带构建身份」✓）：
/// 探针行首自带它 ⇒ 两次读数**并排就自明**是不是同一份构建 ✓（不可比就别比 ✗）。
///
/// 取**当前可执行文件的 mtime** ✓（同一处取、模板统一 ✓）；取不到（库单测等）就退版本号 + `0` ✓。
pub(crate) fn probe_build_id() -> &'static str {
    static ID: OnceLock<String> = OnceLock::new();
    ID.get_or_init(|| {
        let mtime = std::env::current_exe()
            .ok()
            .and_then(|path| std::fs::metadata(path).ok())
            .and_then(|meta| meta.modified().ok())
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|delta| delta.as_secs())
            .unwrap_or(0);
        format!("{}@{}", env!("CARGO_PKG_VERSION"), mtime)
    })
}

fn judge_infer_uncached(
    extra_prefix: &str,
    prefix_src: &str,
    options: &CompileOptions,
    binders: &[GoalBinderSpec],
    term: &str,
) -> Result<String, Judgement> {
    let query = match synthesized_check_term(binders, term) {
        Ok(q) => q,
        Err(message) => {
            return Err(Judgement::Error {
                code: "elab-untyped-binder".to_string(),
                message,
            })
        }
    };
    let mut text = String::from("#check ");
    text.push_str(&query);
    text.push('\n');
    // **P1-a 结构量具**（判据用，噪声免疫）：重跑前缀的**字节数**累计 ——
    // 它是 O(N²) 放大最直接的读数（前缀随声明序号线性变长 ⇒ 总字节随 N² 涨）。
    // ⚠ **不要用"miss 耗时"下结论**：未命中可能并发/嵌套重叠 ⇒ 累加会**超过墙钟** ✗
    // （实测 235.7s > 216.9s）。字节数**没有这个问题** ✓。
    stats::PREFIX_BYTES.fetch_add(
        (extra_prefix.len() + prefix_src.len()) as u64,
        std::sync::atomic::Ordering::Relaxed,
    );
    stats::PREFIX_RUNS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    // **5 趟探针**（`SOKO_PREFIX_MISS_PROBE=1` ✓，2026-10-05）：`seed=on ⇒ prefix=5` ✗ 的
    // 机制两次假设都被实验否掉 ✓ ⇒ **直接打这 5 趟的输入** ✓，不再猜机制 ✗。
    // ⚠ **带构建身份** ✓（`AGENTS.md` 2026-10-04 值守拍板：跨轮比较读数前先确认同一份构建 ✓；
    // 行首自带 ⇒ 并排就自明不可比 ✓）。
    if std::env::var_os("SOKO_PREFIX_MISS_PROBE").is_some() {
        eprintln!(
            "PREFIX_MISS[{}] extra={} prefix={} key={:016x} term={:?} binders={}",
            probe_build_id(),
            extra_prefix.len(),
            prefix_src.len(),
            judge_infer_cache_key(extra_prefix, prefix_src, options, binders, term),
            term,
            binders.len()
        );
    }
    // **按调用点计未命中**（`track_caller` 链透传 ⇒ 原始调用点 ✓）。判据必须是
    // **未命中**而不是**调用**：调用大头是缓存命中（不重跑前缀）✗。
    stats::note_miss_caller(std::panic::Location::caller());
    let mut src = synthesized_prefix(extra_prefix, prefix_src);
    let query_start = src.len();
    src.push_str(&text);
    // 片段模式（G-05 §4.1）：前缀可能停在未闭合的 `namespace` 里，合成的
    // `#check` 必须落在**仍然打开的**那个命名空间内。
    let Ok(file) = crate::parse_fragment(&src) else {
        return Err(Judgement::Error {
            code: "parse".to_string(),
            message: "无法解析推断请求".to_string(),
        });
    };
    // **G-92 真修（2026-10-04 ✓）：合成的前缀走「受信任前缀」那条路** ✗→✓。
    //
    // 病根（实测 ✓）：这里先前是 `compile_fol_with(&file, options)` —— **整份重编** ✗
    // ⇒ 前缀里那些 `by` 声明**又被 elaborate 一遍** ✗（`by_calls` 数的是 `by` 引擎调用 ✓）
    // ⇒ 一个文件里 N 条各自需要新判定的 `by` ⇒ 总成本 **N²** ✗
    // （实测：`by_calls` 10→20 条声明 = 451→1996，**4.4×** ✗；而趟数 `runs` 21→41 = 2.0× ✓ 线性 ✓）。
    //
    // 正解 = **`check_synthesized`** ✓ —— 它和 `by` 路径（`judge_pairs_uncached` ✓）
    // 用的是**同一套机制** ✓：walk 用 `with_trusted_prefix(idx, …)` 压栈 ✓，
    // 这里读栈、把「前缀那 `prefix_commands` 条命令**外层已经核过**」告诉
    // `run_incremental` ⇒ 前缀**不再重查** ✓、只有追加的 `#check` 真的被检查 ✓。
    // ⚠ **没有受信任前缀时它逐字回退到 `check_document_with`** ✓（同一条路 ✓）
    // ⇒ 走查之外调用 `judge_infer`（单文件/测试路径 ✓）**行为一字不变** ✓。
    // ⚠ 夹取在 `check_synthesized` 里做 ✓（`before.min(prefix_commands)` ✓ ——
    // 不夹会**多担保**追加的合成命令 ⇒ 判定声明根本没被检查 ✗，2026-09-30 实测踩过 ✓）。
    let prefix_commands = file.commands.len().saturating_sub(1);
    let (out, used_incremental) = match run_synthesized_incremental(
        &file,
        options,
        prefix_commands,
        prefix_opaque_mode() == PrefixOpaqueMode::On,
    ) {
        Some((out, _report, _before)) => (out, true),
        None => {
            note_synthesized_fallback();
            (compile_fol_with(&file, options), false)
        }
    };
    if let Some(err) = query_error(query_start, &out.errors) {
        return Err(err);
    }
    // 取**最后一条命令**的 `TypeChecked`：合成的前缀里可能本来就有 `#check`
    // （课程/playground 里很常见），它们的事件排在前面；而我们要的是刚追加的
    // 那条查询。按命令号过滤，不做文本比对。
    let last_cmd = file.commands.len().checked_sub(1);
    let ty = pick_type_checked(&out, last_cmd).ok_or_else(|| {
        prefix_error(&out.errors).unwrap_or(Judgement::Error {
            code: "judge-infer-none".to_string(),
            message: "内核未返回类型".to_string(),
        })
    })?;
    // **影子档**（`SOKO_JUDGE_ENV_VOUCH=shadow` ✓）：走增量路时，再整份重跑一次，
    // 比对**消费方看得见的结果**（返回的类型文本 ✓）⇒ 分叉立刻可见 ✓。
    // ⚠ 判据必须绑"调用方读到的东西" ✗ —— 比报告范围会得到**假分叉**（2026-09-30 踩过 ✓）。
    if used_incremental && env_probe::vouch_mode() == env_probe::VouchMode::Shadow {
        let full = compile_fol_with(&file, options);
        match pick_type_checked(&full, last_cmd) {
            Some(b) if b == ty => env_probe::note_shadow(true, ""),
            other => env_probe::note_shadow(
                false,
                &format!(
                    "judge_infer prefix_commands={prefix_commands}\n  担保路={ty:?}\n  整份重查={other:?}"
                ),
            ),
        }
    }
    // 剥掉 `fun (b1:T1) => ... => <codomain>` 的 n 层 binder 箭头。
    // pp 可能把相邻 binder 折叠成 `forall (a b : Prop), ...`（一个 Forall 多
    // binder），所以逐 **单个** binder 剥；余下重渲染成可回读的单箭头链。
    Ok(peel_binders(ty, binders.len()))
}

/// 剥掉 `expr` 的第一个 binder（多 binder Forall 去掉首个、单 binder 去 body、
/// Arrow 去 codomain），返回余下结构。
fn peel_one_binder(expr: &Expr) -> Option<Expr> {
    match expr {
        Expr::Forall { binders, body, .. } if !binders.is_empty() => {
            if binders.len() > 1 {
                Some(Expr::Forall {
                    binders: binders[1..].to_vec(),
                    body: body.clone(),
                    span: Span::default(),
                })
            } else {
                Some(body.as_ref().clone())
            }
        }
        Expr::Arrow { codomain, .. } => Some(codomain.as_ref().clone()),
        _ => None,
    }
}

/// 渲染成**可回读**的文本：多 binder Forall 逐名字拆成 `(a : T) -> … ->` 单
/// 箭头链（`proof::render_expr` 会拼成 `(a) (b) ->`，无法再 parse）。
fn render_roundtrip(expr: &Expr) -> String {
    match expr {
        Expr::Forall { binders, body, .. } if !binders.is_empty() => {
            let mut rest = render_roundtrip(body);
            for binder in binders.iter().rev() {
                let ty = binder.ty.as_deref().map(render_expr).unwrap_or_default();
                let (l, r) = match binder.style {
                    BinderKind::Explicit => ("(", ")"),
                    BinderKind::Implicit => ("{", "}"),
                };
                rest = format!("{l}{} : {ty}{r} -> {rest}", binder.name);
            }
            rest
        }
        _ => render_expr(expr),
    }
}

/// 报告里的错误**按归属分拣**：只认落在追加查询那一段里的那条。
///
/// 为什么需要（设计 `docs/design/course-lean-style.md` L2.10）：判定通道是
/// 「文档前缀 + 一条合成查询」拼起来**重编译**。前缀里**任何一条先前失败的
/// 声明**都会让这次重编译报错，而旧代码取 `errors[0]`——那通常是**前缀里的**
/// 错，于是每条 tactic、每个记法都会收到一条与它无关的诊断（实测：一个坏声明
/// 让后面整片文件报 `elab-notation-unknown-target`，改写期极难定位）。
///
/// 判据是**字节偏移**（`span.start.offset` 与查询起点比较），不做文本比对：
/// 解析器给的 span 就是这份拼接文本上的位置。`query_start` = 拼接前
/// `src.len()`（查询文本紧跟在它后面）。
///
/// **前缀里的错不在这里报**——它们在声明通道有自己的 span（G-10/G-15），
/// 而且不该让一条与它们无关的查询失败。只有在查询**自己也没拿到结果**时才用
/// [`prefix_error`] 把它们抬出来解释原因。
fn query_error(query_start: usize, errors: &[CompileError]) -> Option<Judgement> {
    errors
        .iter()
        .find(|e| e.span.start.offset >= query_start)
        .map(|e| Judgement::Error {
            code: e.code().to_string(),
            // **G-49**：内核原文的人话化在**构造点**（`CompileError::kernel` ✓）——
            // 这里拿到的 `e.message` 已经渲染过 ✓（裸 `$2` 漏不到这里 ✗）。
            message: e.message.clone(),
        })
}

/// 查询没拿到结果时的**解释**：前缀里有声明没通过就如实说（比「内核未返回类型」
/// 有信息量），否则 `None`（调用方给既有的兜底码）。
fn prefix_error(errors: &[CompileError]) -> Option<Judgement> {
    let first = errors.first()?;
    Some(Judgement::Error {
        code: "prefix-decl-failed".to_string(),
        message: format!(
            "前面的声明没通过（第 {} 行）：{}——先修它，这条查询才有意义",
            first.span.start.line, first.message
        ),
    })
}

/// 内核 pp 渲染的**类型文本**（G-05，设计 `docs/design/namespace-open.md` §4.6）。
///
/// 合成 `#check fun (x : <ty>) => x` 走完整流水线，取回的文本是
/// `(x : T) -> T`，剥掉那一层 binder 就是 `T` 的**规范文本**（内核自己的
/// pretty printer 产出：命名空间里的短名会渲染成全名 `A.mem`）。
///
/// 为什么需要：`by` 引擎的 `apply` 用**文本**把「被应用函数的 codomain」与
/// 「当前目标」对齐（`unify_spine`），而 codomain 来自内核 pp、目标来自源 AST。
/// 源里写短名时两边文本不同（`mem` vs `A.mem`）——把目标也过一遍内核，
/// 两边就同源了（判定仍在内核，不做文本比对）。
///
/// 失败一律 `None`（调用方退回源 AST：行为与加这条之前逐字相同）。
pub fn judge_render_type(
    prefix_src: &str,
    options: &CompileOptions,
    binders: &[GoalBinderSpec],
    ty: &str,
) -> Option<String> {
    // 绑定名必须**不可能与目标里的自由变量同名**：`fun (x : x = x) => x` 会把
    // 目标里的 `x` 捕获掉（`judge_infer` 随后 elaborate 不了 / 返回错的类型），
    // 于是 `canonical_goal_with_spec` 静默退回源 AST，`rfl` 这类要读目标结构的
    // tactic 在 `= ` 记法目标上就永远拿不到规范形态（实测：`x = x` / `A = A`）。
    let term = render_type_query(ty);
    // `judge_infer` 交出来的文本**已经剥过 `binders.len()` 层**（见
    // `judge_infer_uncached` 的结尾）⇒ 收尾不用再剥，传 0 ✓。
    let text = judge_infer(prefix_src, options, binders, &term).ok()?;
    judge_render_type_finish(&text, 0)
}

/// 把 doc 中 decl_span 命令里的 hole_span 替换为候选 term，改名合成声明
/// 后走完整流水线判定（与 [`judge_terms`] 同语义：合成声明是唯一裁判）。
///
/// 与 [`judge_terms`] 的差别：判定发生在**文档里的真实命令**中。声明名换成
/// `_soko_judge_k`（`example` 声明无名字：把首 token `example` 换成
/// `def _soko_judge_k`；宇宙参数 `{u}` 等保留原样），指定洞的 `sorry` 换成
/// 候选，其余洞保持原样。因此命令里只要还有剩余洞，合成声明就仍是 open
/// 练习，结论如实是 [`Judgement::Error`]——多洞状态的逐洞判定由调用方
/// （`suggest`）改用 [`judge_terms`] 按子洞期望类型完成。
pub fn judge_hole_fill(
    doc_src: &str,
    options: &CompileOptions,
    decl_span: Span,
    hole_span: Span,
    candidates: &[&str],
) -> Vec<Judgement> {
    judge_hole_fill_with("", doc_src, options, decl_span, hole_span, candidates)
}

/// 同 [`judge_hole_fill`]，但把 `extra_prefix`（闭包上下文）拼在文档前缀之前。
pub fn judge_hole_fill_with(
    extra_prefix: &str,
    doc_src: &str,
    options: &CompileOptions,
    decl_span: Span,
    hole_span: Span,
    candidates: &[&str],
) -> Vec<Judgement> {
    let key = judge_cache_key(&[
        extra_prefix,
        doc_src,
        &options_key(options),
        &format!("{decl_span:?}"),
        &format!("{hole_span:?}"),
        &format!("{candidates:?}"),
    ]);
    if let Some(JudgeCacheValue::Terms(j)) = judge_cache_get(key) {
        stats::HITS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        return j;
    }
    stats::MISSES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let j = judge_hole_fill_uncached(
        extra_prefix,
        doc_src,
        options,
        decl_span,
        hole_span,
        candidates,
    );
    judge_cache_put(key, JudgeCacheValue::Terms(j.clone()));
    j
}

fn judge_hole_fill_uncached(
    extra_prefix: &str,
    doc_src: &str,
    options: &CompileOptions,
    decl_span: Span,
    hole_span: Span,
    candidates: &[&str],
) -> Vec<Judgement> {
    let mut judgements = vec![
        Judgement::Error {
            code: "judge-not-run".to_string(),
            message: "判定未执行".to_string(),
        };
        candidates.len()
    ];
    if candidates.is_empty() {
        return judgements;
    }
    let all_parse_error = |message: String| -> Vec<Judgement> {
        vec![
            Judgement::Error {
                code: "parse".to_string(),
                message,
            };
            candidates.len()
        ]
    };
    let decl_start = decl_span.start.offset.min(doc_src.len());
    let decl_end = decl_span.end.offset.clamp(decl_start, doc_src.len());
    let slice = &doc_src[decl_start..decl_end];
    let Ok(tokens) = tokenize(slice) else {
        return all_parse_error("声明命令切片无法分词".to_string());
    };
    // 声明名 token 的切片内区间：`example` 换成 `def _soko_judge_k`；
    // def/theorem/axiom 的名字 token 换成 `_soko_judge_k`。名字定位复用
    // `references::decl_name_span`（token 精确，绝不扫描文本）。
    let (name_start, name_end, example_keyword) =
        match decl_name_segment(doc_src, decl_span, decl_start, &tokens) {
            Ok(segment) => segment,
            Err(message) => return all_parse_error(message),
        };
    // 洞的切片内区间：必须确实落在命令里，且切片就是 `sorry`。
    let hole_start = hole_span.start.offset;
    let hole_end = hole_span.end.offset;
    let in_decl = hole_start >= decl_start && hole_end <= decl_end && hole_start < hole_end;
    if !in_decl || &doc_src[hole_start..hole_end] != "sorry" {
        return all_parse_error("洞位置不在该声明的 `sorry` 上".to_string());
    }
    let hole_start = hole_start - decl_start;
    let hole_end = hole_end - decl_start;
    if name_start < hole_end && hole_start < name_end {
        return all_parse_error("声明名与洞重叠，无法合成判定声明".to_string());
    }
    // 逐候选：名字段与洞段两处替换，按偏移拼接出合成命令文本。
    let mut commands: Vec<Command> = Vec::with_capacity(candidates.len());
    let mut failed_parse: Vec<Option<String>> = vec![None; candidates.len()];
    for (k, candidate) in candidates.iter().enumerate() {
        let name = format!("_soko_judge_{k}");
        let name_text = if example_keyword {
            format!("def {name}")
        } else {
            name.clone()
        };
        // 名字段在声明头、洞段在其后；仍按偏移排序，防御性处理乱序输入。
        let segments: [(usize, usize, &str); 2] = if name_start <= hole_start {
            [
                (name_start, name_end, name_text.as_str()),
                (hole_start, hole_end, (*candidate)),
            ]
        } else {
            [
                (hole_start, hole_end, (*candidate)),
                (name_start, name_end, name_text.as_str()),
            ]
        };
        let mut synth = String::with_capacity(slice.len() + name_text.len() + candidate.len());
        let mut cursor = 0usize;
        for (start, end, text) in segments {
            synth.push_str(&slice[cursor..start]);
            synth.push_str(text);
            cursor = end;
        }
        synth.push_str(&slice[cursor..]);
        match crate::parse(&synth) {
            Ok(file) => match file.commands.as_slice() {
                [parsed @ (Command::Def {
                    name: parsed_name, ..
                }
                | Command::Theorem {
                    name: parsed_name, ..
                })] if parsed_name == &name => {
                    commands.push(parsed.clone());
                }
                _ => {
                    failed_parse[k] =
                        Some(format!("合成文本不是声明 `{name}`（候选 `{candidate}`）"));
                }
            },
            Err(err) => {
                failed_parse[k] = Some(format!(
                    "无法解析合成命令（候选 `{candidate}`）：{}",
                    err.message
                ));
            }
        }
    }
    // 前缀命令表 + 合成声明，走与 judge_terms 一致的完整流水线。
    // `extra_prefix`（闭包上下文）拼在文档前缀之前：项目模式下合成声明才能看见
    // 被导入的声明（否则 `And.intro` 之类会报未定义标识符）。
    let full_prefix = synthesized_prefix(extra_prefix, &doc_src[..decl_start]);
    let Ok(mut file) = parse_prefix(&full_prefix) else {
        return all_parse_error("前缀源码无法解析".to_string());
    };
    let prefix_commands = file.commands.len();
    file.commands.extend(commands);
    let report = check_synthesized(&file, options, prefix_commands, judgements.len());
    for (k, judgement) in judgements.iter_mut().enumerate() {
        if let Some(message) = failed_parse[k].take() {
            *judgement = Judgement::Error {
                code: "parse".to_string(),
                message,
            };
            continue;
        }
        *judgement = judgement_of(&report, k);
    }
    judgements
}

/// 合成判定文件的完整前缀：**闭包上下文**（被导入模块的声明文本）+ 文档前缀。
///
/// 项目模式下 `judge_*` 合成的文件只有文档本身时，内核看不见被导入的名字
/// （`front::suggest` 因此给不出 quick-fix，`match`/`by` 也会报未定义）。
/// `extra_prefix` 为空（单文件）时与今天逐字节相同。
fn synthesized_prefix(extra_prefix: &str, doc_prefix: &str) -> String {
    let mut out = String::with_capacity(extra_prefix.len() + doc_prefix.len());
    out.push_str(extra_prefix);
    out.push_str(doc_prefix);
    out
}

/// 前缀解析：走**片段模式**（G-05，设计 `docs/design/namespace-open.md` §4.1）。
///
/// 前缀是文件的一个片段，`by` 块所在的声明在 `namespace Foo` 里时它必然带着
/// 一个未闭合的 `namespace`——严格入口会报 `parse-namespace-unclosed`，而这里
/// 必须容忍，并且**保持命名空间打开**（拼在后面的合成 `#check`/合成声明要落在
/// 里面，引用才按 N4 解析）。
fn parse_prefix(prefix_src: &str) -> Result<FolFile, ()> {
    crate::parse_fragment(prefix_src).map_err(|_| ())
}

/// 声明名 token 在命令切片内的区间（切片相对偏移）：`example` 返回关键字
/// token 自身（合成时换成 `def _soko_judge_k`）；def/theorem/axiom 返回
/// 名字 token（经 `references::decl_name_span` 定位，绝不扫描文本）。
/// [`judge_hole_fill`] 与 [`judge_value_replace`] 共用。第四个返回值表示
/// 声明是匿名 `example`。
fn decl_name_segment(
    doc_src: &str,
    decl_span: Span,
    decl_start: usize,
    tokens: &[Token],
) -> Result<(usize, usize, bool), String> {
    match tokens.first().map(|t| &t.kind) {
        Some(TokenKind::Ident(kw)) if kw == "example" => {
            let token = &tokens[0];
            Ok((token.span.start.offset, token.span.end.offset, true))
        }
        Some(TokenKind::Ident(kw)) if matches!(kw.as_str(), "def" | "theorem" | "axiom") => {
            let Some(TokenKind::Ident(name)) = tokens.get(1).map(|t| &t.kind) else {
                return Err("声明名缺失，无法合成判定声明".to_string());
            };
            let Some(name_span) = crate::references::decl_name_span(doc_src, decl_span, name)
            else {
                return Err(format!("找不到声明名 `{name}` 的 token"));
            };
            let start = name_span.start.offset.saturating_sub(decl_start);
            let end = name_span.end.offset.saturating_sub(decl_start);
            Ok((start, end, false))
        }
        _ => Err("只有 def/theorem/example 声明可以合成判定".to_string()),
    }
}

/// 把 doc 中 decl_span 命令里 `:=` 之后的**整个值位**替换为候选，改名合成
/// 声明后走完整流水线判定（与 [`judge_hole_fill`] 同语义：合成声明是唯一
/// 裁判）。服务失败声明（kernel 拒绝、没有洞）的针对性建议：候选被 kernel
/// 接受才值得呈现。值位起点由 tokenize 定位（`:=` 后第一个 token），
/// 终点即声明 span 末尾（解析器把 span 收在值的最后一个 token 上）。
///
/// 与 [`judge_hole_fill`] 的差别：没有洞可填——候选**整体替换值位**，
/// 宇宙参数与值位之前的命令头原样保留；`example` 声明仍按首 token 换名。
/// 解析失败一律报 [`Judgement::Error`]，绝不 panic。
pub fn judge_value_replace(
    doc_src: &str,
    options: &CompileOptions,
    decl_span: Span,
    candidates: &[&str],
) -> Vec<Judgement> {
    judge_value_replace_with("", doc_src, options, decl_span, candidates)
}

/// 同 [`judge_value_replace`]，但把 `extra_prefix`（闭包上下文）拼在文档前缀之前。
pub fn judge_value_replace_with(
    extra_prefix: &str,
    doc_src: &str,
    options: &CompileOptions,
    decl_span: Span,
    candidates: &[&str],
) -> Vec<Judgement> {
    let mut judgements = vec![
        Judgement::Error {
            code: "judge-not-run".to_string(),
            message: "判定未执行".to_string(),
        };
        candidates.len()
    ];
    if candidates.is_empty() {
        return judgements;
    }
    let all_parse_error = |message: String| -> Vec<Judgement> {
        vec![
            Judgement::Error {
                code: "parse".to_string(),
                message,
            };
            candidates.len()
        ]
    };
    let decl_start = decl_span.start.offset.min(doc_src.len());
    let decl_end = decl_span.end.offset.clamp(decl_start, doc_src.len());
    let slice = &doc_src[decl_start..decl_end];
    let Ok(tokens) = tokenize(slice) else {
        return all_parse_error("声明命令切片无法分词".to_string());
    };
    let (name_start, name_end, example_keyword) =
        match decl_name_segment(doc_src, decl_span, decl_start, &tokens) {
            Ok(segment) => segment,
            Err(message) => return all_parse_error(message),
        };
    // 值位：`:=` 后第一个 token 起，到声明 span 末尾。
    let Some(colon_eq) = tokens.iter().position(|t| t.kind == TokenKind::ColonEq) else {
        return all_parse_error("声明没有 `:=` 值位，无法替换判定".to_string());
    };
    let Some(value_tok) = tokens.get(colon_eq + 1) else {
        return all_parse_error("声明没有值位，无法替换判定".to_string());
    };
    if value_tok.kind == TokenKind::Eof {
        return all_parse_error("声明没有值位，无法替换判定".to_string());
    }
    let value_start = value_tok.span.start.offset;
    if name_end > value_start {
        return all_parse_error("声明名与值位重叠，无法合成判定声明".to_string());
    }
    // 逐候选：名字段与值段两处替换，按偏移拼接出合成命令文本（值段延伸到
    // 切片末尾，其后没有剩余文本）。
    let mut commands: Vec<Command> = Vec::with_capacity(candidates.len());
    let mut failed_parse: Vec<Option<String>> = vec![None; candidates.len()];
    for (k, candidate) in candidates.iter().enumerate() {
        let name = format!("_soko_judge_{k}");
        let name_text = if example_keyword {
            format!("def {name}")
        } else {
            name.clone()
        };
        let mut synth = String::with_capacity(slice.len() + name_text.len() + candidate.len());
        synth.push_str(&slice[..name_start]);
        synth.push_str(&name_text);
        synth.push_str(&slice[name_end..value_start]);
        synth.push_str(candidate);
        match crate::parse(&synth) {
            Ok(file) => match file.commands.as_slice() {
                [parsed @ (Command::Def {
                    name: parsed_name, ..
                }
                | Command::Theorem {
                    name: parsed_name, ..
                })] if parsed_name == &name => {
                    commands.push(parsed.clone());
                }
                _ => {
                    failed_parse[k] =
                        Some(format!("合成文本不是声明 `{name}`（候选 `{candidate}`）"));
                }
            },
            Err(err) => {
                failed_parse[k] = Some(format!(
                    "无法解析合成命令（候选 `{candidate}`）：{}",
                    err.message
                ));
            }
        }
    }
    // 前缀命令表 + 合成声明，走与 judge_terms 一致的完整流水线
    // （`extra_prefix` 见 [`judge_hole_fill_with`]）。
    let full_prefix = synthesized_prefix(extra_prefix, &doc_src[..decl_start]);
    let Ok(mut file) = parse_prefix(&full_prefix) else {
        return all_parse_error("前缀源码无法解析".to_string());
    };
    let prefix_commands = file.commands.len();
    file.commands.extend(commands);
    let report = check_synthesized(&file, options, prefix_commands, judgements.len());
    for (k, judgement) in judgements.iter_mut().enumerate() {
        if let Some(message) = failed_parse[k].take() {
            *judgement = Judgement::Error {
                code: "parse".to_string(),
                message,
            };
            continue;
        }
        *judgement = judgement_of(&report, k);
    }
    judgements
}

/// 把剩余目标与已写 binders 折叠成完整声明类型：一个 Forall 望远镜
/// `forall (b1 : T1) (b2 : T2), 剩余目标`——与命名箭头的语法语义一致
/// （后一个 binder 的类型可以引用前一个，必须在同一 telescope 内 elaborate）。
/// 返回 `Err(binder_name)` 表示该 binder 缺少类型标注。
fn fold_declared(
    goal: Expr,
    binders: &[GoalBinderSpec],
    notations: &[crate::ast::NotationDecl],
) -> Result<Expr, String> {
    let mut parsed = Vec::with_capacity(binders.len());
    for binder in binders {
        let Some(text) = &binder.ty else {
            return Err(binder.name.clone());
        };
        // binder 的类型也是 `render_expr` 打回来的源码文本：`intro h` 在
        // `a ∈ A -> …` 上引入的 `h` 类型就是 `a ∈ A`（G-04 第二刀）。
        let Ok(domain) = crate::proof::parse_expr_text_with(text, notations) else {
            return Err(binder.name.clone());
        };
        parsed.push(Binder {
            name: binder.name.clone(),
            ty: Some(Box::new(domain)),
            style: BinderKind::Explicit,
            span: Span::default(),
        });
    }
    if parsed.is_empty() {
        return Ok(goal);
    }
    Ok(Expr::Forall {
        binders: parsed,
        body: Box::new(goal),
        span: Span::default(),
    })
}

/// 把术语包上已写 binders：`fun (b1 : T1) => fun (b2 : T2) => term`。
/// 未写类型的 binder 留空，交给声明类型驱动的 binder 推断（I6）。
///
/// **binder 类型文本必须带记法表回读**（课程 Lean 化实测发现，设计
/// `docs/design/course-lean-style.md` X2）：`GoalBinderSpec.ty` 是
/// `render_expr` 打回来的**源码级**文本（`intro h` 在目标 `¬ A -> …` 上
/// 引入的 `h` 类型就是 `¬ A`）。用不带记法表的 `parse_expr_text` 回读时，
/// 数学码点类之外的符号（`¬`U+00AC / `↔`U+2194 / `→`U+2192）会被读成
/// **标识符**，于是 `¬ A` 变成 `App(Ident("¬"), A)`，判卷报
/// `unknown identifier ¬`。`fold_declared`（本文件 `:981`）从一开始就传了
/// `notations`，这里漏了——两处现在同口径。
fn wrap_binders(
    binders: &[GoalBinderSpec],
    term: Expr,
    notations: &[crate::ast::NotationDecl],
) -> Expr {
    let mut term = term;
    for binder in binders.iter().rev() {
        let ty = match &binder.ty {
            Some(text) => crate::proof::parse_expr_text_with(text, notations)
                .ok()
                .map(Box::new),
            None => None,
        };
        term = Expr::Lambda {
            binders: vec![Binder {
                name: binder.name.clone(),
                ty,
                style: BinderKind::Explicit,
                span: Span::default(),
            }],
            body: Box::new(term),
            span: Span::default(),
        };
    }
    term
}

/// 从合成声明的检查结果提取判定结论。
fn judgement_of(report: &DocumentReport, k: usize) -> Judgement {
    let name = format!("_soko_judge_{k}");
    let Some(state) = report
        .decls
        .iter()
        .find(|d| d.name.as_deref() == Some(name.as_str()))
    else {
        return Judgement::Error {
            code: "judge-missing".to_string(),
            message: format!("合成声明 `{name}` 没有产生状态（内部错误）"),
        };
    };
    match state.status {
        DeclStatus::Checked => Judgement::Match,
        DeclStatus::Open => Judgement::Error {
            code: "elab-hole-misplaced".to_string(),
            message: "洞不在可填写的位置".to_string(),
        },
        DeclStatus::Failed => {
            let Some(err) = &state.error else {
                return Judgement::Error {
                    code: "judge-unknown".to_string(),
                    message: "判定失败但没有错误信息（内部错误）".to_string(),
                };
            };
            match (&err.expected, &err.actual) {
                (Some(expected), Some(actual)) => Judgement::Mismatch {
                    expected: expected.clone(),
                    actual: actual.clone(),
                    // **分类照原样带出去**（G-21）：`err.kind` 是
                    // `refine_kernel_kind` 在**人话化之前**的原始载荷上算的 ⇒
                    // 「项落在类型位」（`KernelExpectedSort`）这种根因形状
                    // 不会因为 `$2` → 「第 2 个绑元」的文本替换而丢失 ✓。
                    kind: err.kind,
                },
                _ => Judgement::Error {
                    code: err.code().to_string(),
                    message: err.message.clone(),
                },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile::{check_document, DeclState, DeclStatus, PreludeMode};
    use crate::parse;

    /// **判据：`EnvProvider` 入口的三条契约** ✓（本片第 2 步 ✓，设计 §2/§5 ✓）。
    ///
    /// ① `None` ⇒ **逐字节回退**今天的行为 ✓（回退机制兼判据 ✓）；
    /// ② `Some(p)` 且 `p` 答得上 ⇒ **用它的答案** ✓、**不再合成前缀** ✓；
    /// ③ `p` 收到的是**解析后的 AST**（`(名字, 源类型)` 对 + 操作数 ✓），**不是文本** ✗
    ///    —— 这是 as-built（`infer_type_text_inplace`）要的形状 ✓。
    ///
    /// ⚠ 夹具用 **`Cell` + `&self`** ✓ —— 它证明的只是**这个 trait 形状能编译、能被调用** ✓；
    /// ⚠ **不**证明真 provider 能这么写 ✗ —— 真 provider 要 `&mut InplaceEnv` ✗，
    /// 而判定点那一刻 builder 已被调用方借走 ✓ ⇒ **trait 形状还要重设计** ✓
    /// （见 trait 上方的更正注释 ✓：正解是**重借链** / 闭包式接口 ✓）。
    #[test]
    fn env_provider_is_consulted_and_none_falls_back_byte_for_byte() {
        struct Fake(std::cell::Cell<usize>);
        impl EnvProvider for Fake {
            fn infer_type_text(
                &self,
                binder_srcs: &[(String, Expr)],
                _operand: &Expr,
            ) -> Option<String> {
                self.0.set(self.0.get() + 1);
                assert_eq!(
                    binder_srcs.len(),
                    1,
                    "binder 的「名字 + 源类型」对必须传进来 ✓"
                );
                assert_eq!(binder_srcs[0].0, "h", "binder 名字要对 ✓");
                Some("PROVIDER-ANSWER".to_string())
            }
        }
        let prefix = "axiom P : Prop\n";
        let binders = vec![GoalBinderSpec {
            name: "h".into(),
            ty: Some("P".into()),
        }];
        let options = CompileOptions::default();

        let fake = Fake(std::cell::Cell::new(0));
        let got = judge_infer_with_env(Some(&fake), "", prefix, &options, &binders, "h");
        assert_eq!(
            got.ok().as_deref(),
            Some("PROVIDER-ANSWER"),
            "provider 答得上就必须用它的答案 ✓（否则接线等于没接 ✗）"
        );
        assert_eq!(fake.0.get(), 1, "provider 必须被问到**恰好一次** ✓");

        let with_none = judge_infer_with_env(None, "", prefix, &options, &binders, "h");
        let direct = judge_infer_with("", prefix, &options, &binders, "h");
        assert_eq!(
            format!("{with_none:?}"),
            format!("{direct:?}"),
            "`None` ⇒ 必须**逐字节回退**今天的行为 ✓（这是回退判据 ✓）"
        );
    }

    /// **判据：大前缀不许退回原文** ✗（判据 ③，值守 2026-10-04 派单 ✓）。
    ///
    /// 背景 ✗：我先前为了修一处冷开回归，在 `canonical_prefix_cached` 里加了一道
    /// **尺寸闸**（`PARSE_LIMIT = 64 KB` ⇒ 超过就 `return judge_cache_key(&[src])`）✗
    /// —— 那等于**大单元（unit12 等）根本没吃到「只改证明」这个特性** ✗：
    /// 键里是**原文** ⇒ 改证明体照样全失效 ⇒ 下游照样重跑 ✓。
    /// 用户 13:08 拍板：「**不许降级修**」✗ ⇒ 这条判据就是那条缝的守卫 ✓。
    ///
    /// **A7（2026-10-08）**：文本哈希的**小 LRU 不许改变键** ✓。
    ///
    /// 记忆化是**纯优化**：命中判据是逐字节相等 ⇒ 键只由**内容**决定 ✓。
    /// 这条判据把它钉死：① 同一份文本任何时候都得到同一个哈希；② 不同文本不同；
    /// ③ 与**不用记忆化**的算法（`judge_cache_key(&[src])`）**逐字相同**（"键没变"的定义 ✓）；
    /// ④ LRU 被灌满（> 4 条、发生淘汰）之后**仍然**逐字相同 ✓。
    /// **反向验证**：把命中判据改成"只比长度" ⇒ ③ 必红 ✓。
    #[test]
    fn the_text_hash_memo_never_changes_a_key() {
        let a = "axiom P : Prop\n";
        let b = "axiom Q : Prop\n";
        let ha = canonical_text_key(a);
        let hb = canonical_text_key(b);
        assert_eq!(ha, canonical_text_key(a), "同一份文本的键必须恒等");
        assert_eq!(hb, canonical_text_key(b));
        assert_ne!(ha, hb, "不同文本必须得到不同的键");
        assert_eq!(ha, judge_cache_key(&[a]), "键必须与不用记忆化时逐字相同");
        assert_eq!(hb, judge_cache_key(&[b]));
        // 灌满 LRU（发生淘汰）之后，键**仍然**逐字相同 ✓。
        for i in 0..8 {
            let _ = canonical_text_key(&format!("axiom P{i} : Prop\n"));
        }
        assert_eq!(canonical_text_key(a), judge_cache_key(&[a]));
        assert_eq!(canonical_text_key(b), judge_cache_key(&[b]));
    }

    /// **判据本身** ✓：走 `canonical_prefix_cached`（闸就在它里面 ✓）⇒
    /// ① 不许有任何一次「退回原文」✓；② 身份**不许等于原文的键** ✓（等于就是退回 ✓）。
    /// ⚠ 计数器是**进程级**的 ✗（同 crate 的其它测试并行跑会串味 ✓）⇒ 取**差量** ✓。
    #[test]
    fn a_large_prefix_must_not_fall_back_to_raw_text() {
        let mut src = String::new();
        let mut i = 0usize;
        while src.len() < 96 * 1024 {
            src.push_str(&format!("def f{i} (α : Type) (a : α) : α := a\n"));
            i += 1;
        }
        assert!(
            src.len() > 64 * 1024,
            "夹具前提：这份前缀必须**超过尺寸闸** ✗"
        );
        let before = stats::prefix_fallbacks().0;
        let hash = canonical_prefix_cached(&src);
        let after = stats::prefix_fallbacks().0;
        // ⚠ **计数器是进程级的** ✗（本 crate 的其它测试并行跑会串味 —— 实测全量跑时
        // 本判据假红过 ✓）⇒ 这里**只断言不受污染的那一半** ✓：身份**不等于原文的键** ✓。
        // 课程级的「fallbacks == 0」归 **CLI 侧**（独立进程 ✓，见 `STAGE_STATS … fallbacks=N` ✓）。
        let _ = (before, after);
        assert_ne!(
            hash,
            judge_cache_key(&[&src]),
            "身份**不许等于原文的键** ✗ —— 等于就是「退回原文」✗（判据 ③，值守 2026-10-04）"
        );
    }

    ///
    /// 五条判据（**两个方向都要有** ✓）：
    /// ① 证明体变（陈述一字不动）⇒ 身份**不变** ✓（`theorem` 的值是证明，证明不参与 `def_eq` ✓）；
    /// ② 陈述变（类型改掉）⇒ 身份**必变** ✓；
    /// ③ `def` 的**定义体**变 ⇒ 身份**必变** ✓（`ReducibilityHint::Regular/Abbrev` 会被下游展开
    ///    ⇒ 不把定义体放进身份就是**错编** ✗）；
    /// ④ 更早的单元（库层）变 ⇒ 身份**必变** ✓（前缀是**拼接**出来的 ✓）；
    /// ⑤ 记法声明变 ⇒ 身份**必变** ✓（它是**环境级特性** ✓）。
    ///
    /// ⚠ 夹具必须**解析得过** ✓ —— 解析失败时 `canonical_prefix_id` 退回**原文** ✓（保守 ✓），
    /// 那时①会**误判为变** ✗ ⇒ 这条判据顺带守住「夹具没坏」 ✓。
    #[test]
    fn canonical_prefix_id_tracks_the_interface_not_the_proof_body() {
        let id = crate::compile::canonical_prefix_id;
        let base = "def f (x : Nat) : Nat := x\n\n\
                    theorem t (n : Nat) : f n = n := Eq.refl.{1} Nat (f n)\n";
        assert_ne!(
            id(base),
            base,
            "夹具前提：`base` 必须**解析得过**（解析失败 ⇒ 身份退回**原文** ⇒ 判据①假红 ✗）"
        );
        // ① 证明体变 ⇒ 身份不变 ✓
        let body2 = base.replace("Eq.refl.{1} Nat (f n)", "Eq.refl.{1} Nat (f n) ");
        assert_ne!(body2, base, "夹具前提：①这一刀必须真的改到文本");
        assert_eq!(id(base), id(&body2), "改**证明体**不许改身份 ✗");
        // ② 陈述变 ⇒ 身份必变 ✓
        let ty2 = base.replace(": f n = n :=", ": f n = f n :=");
        assert_ne!(ty2, base, "夹具前提：②这一刀必须真的改到文本");
        assert_ne!(id(base), id(&ty2), "改**陈述**必须改身份 ✗");
        // ③ def 定义体变 ⇒ 身份必变 ✓
        let def2 = base.replace(
            "def f (x : Nat) : Nat := x",
            "def f (x : Nat) : Nat := succ x",
        );
        assert_ne!(def2, base, "夹具前提：③这一刀必须真的改到文本");
        assert_ne!(
            id(base),
            id(&def2),
            "改 **def 定义体**必须改身份 ✗（否则是错编 ✗）"
        );
        // ④ 库层变 ⇒ 身份必变 ✓
        let with_lib = format!("axiom P : Prop\n{base}");
        assert_ne!(id(base), id(&with_lib), "改**库层**必须改身份 ✗");
        // ⑥ `example` **匿名** ⇒ **完全不进**身份 ✓（改它连类型一起改也不惊动任何东西 ✓）
        let with_example = base.replace("def f", "example : True := True.intro\n\ndef f");
        assert_ne!(with_example, base, "夹具前提：⑥这一刀必须真的改到文本");
        assert_eq!(
            id(&with_example),
            id(base),
            "`example` 是**匿名**的 ⇒ 不许进身份 ✗（下游引用不到它 ✓）"
        );
        // ⑤ 记法声明变 ⇒ 身份必变 ✓
        let with_notation = base.replace("def f", "notation \"z\" => f\n\ndef f");
        assert_ne!(with_notation, base, "夹具前提：⑤这一刀必须真的改到文本");
        assert_ne!(id(base), id(&with_notation), "改**记法声明**必须改身份 ✗");
    }

    fn spec(ty: &str, binders: &[(&str, Option<&str>)]) -> OpenGoalSpec {
        OpenGoalSpec {
            universe: Vec::new(),
            ty: ty.to_string(),
            binders: binders
                .iter()
                .map(|(name, ty)| GoalBinderSpec {
                    name: name.to_string(),
                    ty: ty.map(|t| t.to_string()),
                })
                .collect(),
        }
    }

    #[test]
    fn judge_cache_returns_identical_results_and_stores_entries() {
        // 缓存是性能设施（by 块 tactic 的 judge_infer 每键全前缀重编译是
        // funapply 移除的根因）：命中必须返回与直算一致的结果，且条目入库。
        let prefix = "axiom P : Prop\naxiom Q : Prop\n";
        let binders = vec![crate::judge::GoalBinderSpec {
            name: "h".into(),
            ty: Some("P".into()),
        }];
        let options = CompileOptions::default();
        let first = judge_infer(prefix, &options, &binders, "h");
        let before = judge_cache_len();
        let second = judge_infer(prefix, &options, &binders, "h");
        assert_eq!(first, second, "cache must not change results");
        assert!(judge_cache_len() >= before, "the request must be cached");

        // 不同 prelude 模式是指纹的一部分：不串台。
        // 判据是**键在不在**（不是"缓存变长了"）：容量是 FIFO 的
        // `JUDGE_CACHE_CAP`，别的测试把缓存填满时长度不再增长，旧判据会假红
        // （第三刀实测：新增的记法测试把缓存填到上限）。
        let bare = CompileOptions {
            prelude: crate::compile::PreludeMode::Bare,
        };
        let _ = judge_infer(prefix, &bare, &binders, "h");
        // **键只许有一处真相** ✓（2026-10-04 收敛 ✓）：直接调**唯一**那把键函数 ✓ ——
        // 先前这里**手工拼**了一遍 ✗（"与 `judge_infer_with` 逐字一致" ✗）⇒ 键一改
        // 这条判据就假红 ✗，而且它**复制**的正是要消灭的那份重复 ✓。
        let bare_key = judge_infer_cache_key("", prefix, &bare, &binders, "h");
        assert!(
            judge_cache_contains(bare_key),
            "different options = different key"
        );
        assert!(judge_cache_len() >= before);

        // **G-71**：**全显式 pp** 与非全显式是**两份不同的文本** ⇒ 必须是**两把键**
        // （否则两条路互相命中、形态分叉 ✗）。⚠ 两把键**都由唯一那把键函数算** ✓ ——
        // 它们的差别来自 `explicit_pp_active()` ✓（`judge_infer_explicit` 内部会把它置上 ✓）。
        let _ = crate::judge::judge_infer_explicit(prefix, &options, &binders, "h");
        let plain_key = judge_infer_cache_key("", prefix, &options, &binders, "h");
        let explicit_key = {
            let _guard = ExplicitPpGuard::new(true);
            judge_infer_cache_key("", prefix, &options, &binders, "h")
        };
        assert_ne!(plain_key, explicit_key, "全显式 pp 必须是另一把键");
        assert!(
            judge_cache_contains(explicit_key),
            "全显式 pp 的答案要落进它自己那把键"
        );
    }

    /// **批处理 ≡ 逐条判**（0.62.0 性能改动的判据）。
    ///
    /// 一次 `judge_pairs_with` 与逐条 `judge_terms_strict` 必须给出**逐字相同**的
    /// 结论——包括命中、类型不匹配（`expected`/`actual` 文本来自内核）、以及
    /// 解析失败/缺类型标注这两条**预判**路径（它们不合成命令、靠保留的序号回查）。
    #[test]
    fn batched_judgements_match_strict_ones() {
        let prefix = "axiom P : Prop\naxiom Q : Prop\n";
        let options = CompileOptions::default();
        let pairs = vec![
            // 命中：`(h : P) -> P` 里的 `h`。
            JudgePair {
                spec: spec("P", &[("h", Some("P"))]),
                term: "h".to_string(),
            },
            // 不匹配：`(h : P) -> Q` 里的 `h`（内核报 expected/actual）。
            JudgePair {
                spec: spec("Q", &[("h", Some("P"))]),
                term: "h".to_string(),
            },
            // 术语解析失败（预判路径）。
            JudgePair {
                spec: spec("P", &[("h", Some("P"))]),
                term: "fun (".to_string(),
            },
            // binder 缺类型标注（预判路径）。
            JudgePair {
                spec: spec("P", &[("h", None)]),
                term: "h".to_string(),
            },
            // 又来一条能命中的，验证"预判不合成命令"没有把后面的序号错位。
            JudgePair {
                spec: spec("P", &[("h", Some("P"))]),
                term: "h".to_string(),
            },
        ];
        let batched = judge_pairs_with("", prefix, &options, &pairs);
        let strict: Vec<Judgement> = pairs
            .iter()
            .map(|pair| {
                judge_terms_strict(prefix, &options, &pair.spec, &[pair.term.as_str()])
                    .into_iter()
                    .next()
                    .expect("one judgement per term")
            })
            .collect();
        assert_eq!(batched.len(), strict.len());
        for (k, (b, s)) in batched.iter().zip(strict.iter()).enumerate() {
            assert_eq!(b, s, "第 {k} 条判定：批处理与逐条判不一致");
        }
        assert_eq!(batched[0], Judgement::Match);
        assert_eq!(batched[4], Judgement::Match, "序号不能因预判而错位");
        assert!(matches!(batched[1], Judgement::Mismatch { .. }));
        assert!(matches!(batched[2], Judgement::Error { .. }));
        assert!(matches!(batched[3], Judgement::Error { .. }));
    }

    /// **一个 `by` 块只付一次文档走查**（性能改动的机制判据）。
    ///
    /// 关掉批处理时，N 次判定 = N 次整前缀走查；打开时 = **1** 次。这条断言
    /// 直接钉住"逐步重判整份文档"这个根因不会回来。
    #[test]
    fn one_by_block_pays_a_single_document_pass() {
        let src = "theorem t (A B : Prop) (h1 : A) (h2 : B) : A \u{2227} B := by\n  have a : A := h1\n  have b : B := h2\n  exact \u{27e8}a, b\u{27e9}\n";
        let file = parse(src).expect("parse");

        let previous = set_batching(true);
        reset_pass_count();
        let batched = check_document(&file);
        let batched_passes = pass_count();
        set_batching(previous);

        let previous = set_batching(false);
        reset_pass_count();
        let strict = check_document(&file);
        let strict_passes = pass_count();
        set_batching(previous);

        // 结论一致（两条路都判过），但代价差一个数量级。
        assert_eq!(
            batched
                .decls
                .iter()
                .map(|d| (d.name.clone(), d.status))
                .collect::<Vec<_>>(),
            strict
                .decls
                .iter()
                .map(|d| (d.name.clone(), d.status))
                .collect::<Vec<_>>(),
            "批处理不能改变判卷结论"
        );
        assert_eq!(
            batched_passes, 1,
            "一个 by 块（3 次判定）应当只走一遍文档，实际 {batched_passes} 遍"
        );
        assert!(
            strict_passes >= 3,
            "关掉批处理应当逐条判（≥3 遍），实际 {strict_passes} 遍"
        );

        // ── **B3（2026-10-08）**：失败块的严格趟**从失败步续跑**（P3 成本面）──────
        //
        // 判据（两条，都**不依赖判定缓存的冷热** ✓）：
        // 1. **直接读数** `by_failure_resume_from()`：`0` = 没续跑（退回整段严格重放 ✗）、
        //    `k+1` = 从第 k 条续跑 ✓ —— 它直接回答"机制有没有生效"，**撤掉续跑必红** ✓；
        // 2. **诊断红线**：`batched` 与 `strict`（关批处理）的诊断**逐字相同** ✓。
        //
        // ⚠ **为什么不用"文档走查数"当判据**（原计划那条）：`pass_count()` 数的是
        // `judge_pairs_uncached`，而**命中判定缓存**的判定一次都不计 ✗ —— 同一夹具
        // 编第二遍就读到 **0**（实测踩到 ✗），于是"撤掉续跑"那条反向验证**咬不住**
        // （"4 → 2"那个前后对比是**冷缓存**下的读数，不能当守卫 ✓）。⇒ 守卫改读
        // **机制本身**（同 `INPLACE_BY_SHADOW_*` 的纪律：别拿语义相近的量当判据 ✗）。
        // 冷缓存下的历史读数（如实留档、不作断言）：失败位置 3 ⇒ B3 前 **4** 遍、B3 后 **2** 遍。
        //
        // 夹具要"前面的 tactic 一定成功、且不动目标"⇒ 两条 `have`（只往上下文加东西 ✓）
        // + 最后一条 `exact h1` 对目标 `A ∧ B` 必然不匹配 ⇒ 失败位置 = 第 `bad` 条 ✓
        // （用 `apply And.intro` 那类**会动目标**的 tactic 会把失败位置挪到第 2 条 ✗）。
        // 夹具名字带 `tag`：判定缓存按前缀文本做键 ⇒ 每次测量用不同的 `tag` ✓。
        let block = |bad: usize, tag: &str| {
            let steps = ["have a : A := h1", "have b : B := h2", "exact h1"];
            let mut body = String::new();
            for (i, step) in steps.iter().enumerate() {
                if i + 1 == bad {
                    body.push_str("  exact h1\n");
                } else {
                    body.push_str(&format!("  {step}\n"));
                }
            }
            format!("theorem t_{tag} (A B : Prop) (h1 : A) (h2 : B) : A \u{2227} B := by\n{body}")
        };
        let run_for = |bad: usize, batching: bool, tag: &str| -> (usize, Vec<String>) {
            let file = parse(&block(bad, tag)).expect("parse");
            let previous = set_batching(batching);
            reset_pass_count();
            let report = check_document(&file);
            let passes = pass_count();
            set_batching(previous);
            let errors: Vec<String> = report
                .errors
                .iter()
                .map(|e| format!("{}:{}", e.code(), e.message))
                .collect();
            (passes, errors)
        };
        let resume_for = |bad: usize, tag: &str| -> usize {
            crate::by::by_failure_resume_reset();
            let _ = run_for(bad, true, tag);
            crate::by::by_failure_resume_from()
        };
        assert_eq!(
            resume_for(3, "r3"),
            3,
            "**B3 的正身**：第 3 条错 ⇒ 必须**从第 3 条续跑**（读数 = k+1 = 3）；\
             0 = 没续跑（退回整段严格重放 ✗）"
        );
        assert_eq!(
            resume_for(1, "r1"),
            1,
            "第 1 条错 ⇒ 从第 1 条续跑（读数 = 1）—— 与「整段重放」在这条夹具上等价，\
             但读数仍然证明**走的是续跑那条路** ✓"
        );
        let (_, errors_first) = run_for(1, true, "p1");
        let (_, errors_last) = run_for(3, true, "p3");
        let (_, strict_first) = run_for(1, false, "s1");
        let (_, strict_last) = run_for(3, false, "s3");
        assert_eq!(
            errors_first, strict_first,
            "第 1 条错：**续跑不许改变诊断**（判定红线）"
        );
        assert_eq!(
            errors_last, strict_last,
            "第 3 条错：**续跑不许改变诊断**（判定红线）"
        );
        assert!(
            !errors_last.is_empty(),
            "夹具前提：这一块必须真的失败（否则量的是成功路径）"
        );
    }

    #[test]
    fn matching_hypothesis_is_a_kernel_match() {
        let prefix = "axiom a : Prop\n";
        // 剩余目标 a，已写 binder h : a ⇒ 折叠出的声明类型是 (h : a) -> a。
        let open = spec("a", &[("h", Some("a"))]);
        let judgements = judge_terms(
            prefix,
            &CompileOptions::default(),
            &open,
            &["h", "fun (x : Prop) => x"],
        );
        assert_eq!(judgements[0], Judgement::Match);
        match &judgements[1] {
            Judgement::Mismatch {
                expected, actual, ..
            } => {
                // debug printer 渲染：Prop → Sort(0)，宇宙参数带 .[] 后缀。
                assert!(expected.contains("a"), "expected: {expected}");
                assert!(actual.contains("Sort(0)"), "actual: {actual}");
            }
            other => panic!("expected mismatch, got {other:?}"),
        }
    }

    #[test]
    fn dependent_binder_types_are_judged_correctly() {
        // h : a（依赖前面的 binder a）与剩余目标 a 相同。
        let open = spec("a", &[("a", Some("Prop")), ("h", Some("a"))]);
        let judgements = judge_terms("", &CompileOptions::default(), &open, &["h"]);
        assert_eq!(judgements[0], Judgement::Match);
    }

    #[test]
    fn binder_without_type_annotation_is_reported() {
        // 声明层总会为未写类型的 binder 借来类型，所以 None 只可能是
        // 防御性输入；判定明确报错而不是静默猜测。
        let open = spec("Prop", &[("x", None)]);
        let judgements = judge_terms("", &CompileOptions::default(), &open, &["x"]);
        match &judgements[0] {
            Judgement::Error { code, message } => {
                assert_eq!(code, "elab-untyped-binder");
                assert!(message.contains('x'), "message: {message}");
            }
            other => panic!("expected untyped-binder error, got {other:?}"),
        }
    }

    #[test]
    fn defeq_but_differently_written_type_matches() {
        // `Not a` 与 `a -> False` 文本不同但 definitional equal —— 文本比对
        // 会漏掉它，kernel 判定能识别（REQUIREMENTS §2.8 的意义所在）。
        let prefix = "axiom False : Prop\ndef Not : Prop -> Prop := fun (a : Prop) => a -> False\n";
        let open = spec("Not a", &[("a", Some("Prop")), ("h", Some("a -> False"))]);
        let judgements = judge_terms(prefix, &CompileOptions::default(), &open, &["h"]);
        assert_eq!(judgements[0], Judgement::Match);
    }

    #[test]
    fn unknown_term_reports_elab_error() {
        let open = spec("Prop", &[]);
        let judgements = judge_terms("", &CompileOptions::default(), &open, &["nope"]);
        match &judgements[0] {
            Judgement::Error { code, message } => {
                assert_eq!(code, "elab-unknown-identifier");
                assert!(message.contains("nope"));
            }
            other => panic!("expected elab error, got {other:?}"),
        }
    }

    #[test]
    fn failed_prefix_declaration_keeps_its_name_free() {
        // check-then-add：前缀里被 kernel 拒绝的名字不占用。引用它的候选在
        // pass1 能通过（用的是"幽灵"声明类型），pass2 会重新 elaborate 并
        // 得到真正的 unknown-identifier —— 这正是教学想要的判定语义。
        let prefix = "def broken : Prop -> Type := fun (x : Prop) => x\n";
        let open = spec("Prop -> Type", &[]);
        let judgements = judge_terms(prefix, &CompileOptions::default(), &open, &["broken"]);
        match &judgements[0] {
            Judgement::Error { code, .. } => {
                assert_eq!(code, "elab-unknown-identifier");
            }
            other => panic!("expected unknown identifier, got {other:?}"),
        }
    }

    #[test]
    fn bare_mode_judges_without_prelude() {
        let options = CompileOptions {
            prelude: PreludeMode::Bare,
        };
        let open = spec("Prop -> Prop", &[]);
        let judgements = judge_terms("", &options, &open, &["fun (x : Prop) => x"]);
        assert_eq!(judgements[0], Judgement::Match);
        // Bare 模式下 Nat 不存在。
        let open_nat = spec("Nat", &[]);
        let judgements = judge_terms("", &options, &open_nat, &["2"]);
        match &judgements[0] {
            Judgement::Error { code, .. } => {
                assert_eq!(code, "elab-unknown-identifier");
            }
            other => panic!("expected unknown Nat in bare mode, got {other:?}"),
        }
    }

    #[test]
    fn universe_carrying_open_goal_judges() {
        // 带宇宙参数的声明：剩余目标 α，合成声明必须携带同样的 {u}。
        let prefix =
            "def id {u} : {α : Sort u} -> (a : α) -> α := fun {α : Sort u} => fun (a : α) => a\n";
        let open = OpenGoalSpec {
            universe: vec!["u".to_string()],
            ty: "α".to_string(),
            binders: vec![
                GoalBinderSpec {
                    name: "α".to_string(),
                    ty: Some("Sort u".to_string()),
                },
                GoalBinderSpec {
                    name: "a".to_string(),
                    ty: Some("α".to_string()),
                },
            ],
        };
        let judgements = judge_terms(
            prefix,
            &CompileOptions::default(),
            &open,
            &["a", "id.{u} α a"],
        );
        assert_eq!(judgements[0], Judgement::Match);
        assert_eq!(judgements[1], Judgement::Match);
    }

    // ---- judge_hole_fill：文档真实命令里的逐洞判定 ----

    /// 取文档里第一个 open 练习的 DeclState（span 与洞位都来自完整流水线）。
    fn open_decl(doc: &str) -> DeclState {
        let report = check_document(&parse(doc).expect("parses"));
        report
            .decls
            .iter()
            .find(|d| d.status == DeclStatus::Open)
            .expect("open exercise")
            .clone()
    }

    const SUB_HOLE_DOC: &str = "axiom And : Prop -> Prop -> Prop\n\
         axiom And.intro : (a : Prop) -> (b : Prop) -> a -> b -> And a b\n\
         theorem t : (a : Prop) -> (b : Prop) -> (ha : a) -> (hb : b) -> And a b := \
         fun (a : Prop) => fun (b : Prop) => fun (ha : a) => fun (hb : b) => And.intro a b ha sorry\n";

    #[test]
    fn hole_fill_accepts_hypothesis_in_a_sub_hole() {
        // 剩一个子洞的 spine 状态：填对假设 ⇒ 整个证明被完整 kernel 接受；
        // 填错 ⇒ 内核给出"期望 / 实际"（类型不合的候选 → Mismatch）。
        let d = open_decl(SUB_HOLE_DOC);
        let judgements = judge_hole_fill(
            SUB_HOLE_DOC,
            &CompileOptions::default(),
            d.span,
            d.holes[0],
            &["hb", "ha"],
        );
        assert_eq!(judgements[0], Judgement::Match);
        assert!(
            matches!(judgements[1], Judgement::Mismatch { .. }),
            "type-mismatched candidate must be a kernel mismatch, got {:?}",
            judgements[1]
        );
    }

    #[test]
    fn hole_fill_renames_examples_by_replacing_the_first_token() {
        // `example` 无名字：首 token `example` 换成 `def _soko_judge_k`。
        let doc = "axiom False : Prop\n\
                   example : (h : False) -> False := fun (h : False) => sorry\n";
        let d = open_decl(doc);
        let judgements = judge_hole_fill(
            doc,
            &CompileOptions::default(),
            d.span,
            d.holes[0],
            &["h", "False"],
        );
        assert_eq!(judgements[0], Judgement::Match);
        match &judgements[1] {
            Judgement::Mismatch {
                expected, actual, ..
            } => {
                // 内核渲染：`False.[]` 保留原名，Prop 显示为 Sort(0)。
                assert!(expected.contains("False"), "expected: {expected}");
                assert!(actual.contains("Sort(0)"), "actual: {actual}");
            }
            other => panic!("expected mismatch, got {other:?}"),
        }
    }

    #[test]
    fn hole_fill_keeps_universe_params_from_the_source() {
        // def 的宇宙参数 `{u}` 原样保留在合成命令里，Sort u 目标可判定。
        let doc =
            "def idT {u} : {α : Sort u} -> (a : α) -> α := fun {α : Sort u} => fun (a : α) => a\n\
                   theorem t {u} : {α : Sort u} -> (a : α) -> Eq.{u} α a a := \
                   fun {α : Sort u} => fun (a : α) => sorry\n";
        let d = open_decl(doc);
        let judgements = judge_hole_fill(
            doc,
            &CompileOptions::default(),
            d.span,
            d.holes[0],
            &["Eq.refl.{u} α a"],
        );
        assert_eq!(judgements[0], Judgement::Match);
    }

    #[test]
    fn hole_fill_with_remaining_holes_reports_open_honestly() {
        // 其余洞保持 `sorry` ⇒ 合成声明仍是 open 练习：kernel 没能整体裁决，
        // 结论如实为 Error（绝不把"没判过"说成 Match）。
        let doc = "axiom And : Prop -> Prop -> Prop\n\
                   axiom And.intro : (a : Prop) -> (b : Prop) -> a -> b -> And a b\n\
                   theorem t : (a : Prop) -> (b : Prop) -> (ha : a) -> (hb : b) -> And a b := \
                   fun (a : Prop) => fun (b : Prop) => fun (ha : a) => fun (hb : b) => And.intro sorry sorry\n";
        let d = open_decl(doc);
        let judgements = judge_hole_fill(
            doc,
            &CompileOptions::default(),
            d.span,
            d.holes[0],
            &["ha", "hb"],
        );
        assert!(
            judgements.iter().all(
                |j| matches!(j, Judgement::Error { code, .. } if code == "elab-hole-misplaced")
            ),
            "remaining holes keep the fill open: {judgements:?}"
        );
    }

    #[test]
    fn hole_fill_reports_parse_failures_as_errors_not_panics() {
        let d = open_decl(SUB_HOLE_DOC);
        let judgements = judge_hole_fill(
            SUB_HOLE_DOC,
            &CompileOptions::default(),
            d.span,
            d.holes[0],
            &["no(", "hb"],
        );
        match &judgements[0] {
            Judgement::Error { code, .. } => assert_eq!(code, "parse"),
            other => panic!("expected parse error, got {other:?}"),
        }
        assert_eq!(judgements[1], Judgement::Match);
    }

    // ---- judge_value_replace：失败声明的值位整体替换判定 ----

    /// 取文档里第一个 failed 声明的 DeclState（span 来自完整流水线）。
    fn failed_decl(doc: &str) -> DeclState {
        let report = check_document(&parse(doc).expect("parses"));
        report
            .decls
            .iter()
            .find(|d| d.status == DeclStatus::Failed)
            .expect("failed declaration")
            .clone()
    }

    #[test]
    fn value_replace_accepts_a_kernel_verified_rfl_candidate() {
        // 匿名 example：首 token 换名后整值替换。kernel 接受的 rfl 候选
        // Match，两边不同的候选被内核以 Mismatch 拒绝。
        let doc = "example : Eq.{1} Nat 2 2 := 3\n";
        let d = failed_decl(doc);
        let judgements = judge_value_replace(
            doc,
            &CompileOptions::default(),
            d.span,
            &["Eq.refl.{1} Nat 2", "Eq.refl.{1} Nat 3"],
        );
        assert_eq!(judgements[0], Judgement::Match);
        assert!(
            matches!(judgements[1], Judgement::Mismatch { .. }),
            "3 ≢ 2 must be a kernel mismatch, got {:?}",
            judgements[1]
        );
    }

    #[test]
    fn value_replace_keeps_universe_params_and_declared_binders() {
        // def 的宇宙参数 `{u}` 与值位之前的命令头原样保留：候选 lambda
        // 引用 Sort u 必须仍可 elaborate，内核才判得出 Match。
        let doc =
            "def idT {u} : {α : Sort u} -> (a : α) -> α := fun {α : Sort u} => fun (a : α) => 1\n";
        let d = failed_decl(doc);
        let judgements = judge_value_replace(
            doc,
            &CompileOptions::default(),
            d.span,
            &[
                "fun {α : Sort u} => fun (a : α) => a",
                "fun {α : Sort u} => fun (a : α) => 2",
            ],
        );
        assert_eq!(judgements[0], Judgement::Match);
        assert!(
            matches!(judgements[1], Judgement::Mismatch { .. }),
            "Nat ≢ α must be a kernel mismatch, got {:?}",
            judgements[1]
        );
    }

    #[test]
    fn value_replace_reports_unparseable_candidates_as_errors() {
        let doc = "example : Eq.{1} Nat 2 2 := 3\n";
        let d = failed_decl(doc);
        let judgements =
            judge_value_replace(doc, &CompileOptions::default(), d.span, &["no(", "nope"]);
        match &judgements[0] {
            Judgement::Error { code, message } => {
                assert_eq!(code, "parse");
                assert!(message.contains("no("), "message: {message}");
            }
            other => panic!("expected parse error, got {other:?}"),
        }
        match &judgements[1] {
            Judgement::Error { code, .. } => assert_eq!(code, "elab-unknown-identifier"),
            other => panic!("expected elab error, got {other:?}"),
        }
    }

    #[test]
    fn value_replace_reports_declarations_without_a_value_as_errors() {
        // axiom 没有 `:=` 值位：明确报错，绝不 panic。
        let doc = "axiom bad : undefined_name\n";
        let d = failed_decl(doc);
        let judgements = judge_value_replace(doc, &CompileOptions::default(), d.span, &["Prop"]);
        match &judgements[0] {
            Judgement::Error { code, .. } => assert_eq!(code, "parse"),
            other => panic!("expected parse error, got {other:?}"),
        }
    }
}

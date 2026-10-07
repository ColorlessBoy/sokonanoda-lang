//! **闸类计数出口**（G-91，2026-10-04 值守派单 ✓）。
//!
//! ## 为什么要有这个模块
//!
//! 值守 13:12 的总规矩（用户原话「这种闸我都不能接受」✓）：
//!
//! > **预算 / 尺寸 / 规模耗尽，只允许「变慢」，绝不允许「变错 / 变差」。**
//! > 凡「**超过某个数字就换一条路**」的分支，**必须带一个计数出口**；否则不许进仓。
//!
//! 界线写清楚 ✓：
//!
//! * **`判不了 ⇒ 走慢路`** —— 可以 ✓（结果不变，只是慢）；
//! * **`判不了 ⇒ 当成否`** —— **不可以** ✗（那就是"判不出来 ⇒ 判你错"）。
//!
//! ## 这个模块做什么
//!
//! 把内核里**每一个**「超限就换路」的分支记一笔 ✓（**只加计数、零行为变化** ✓）。
//!
//! **两笔** ✓：① 内核侧**六项**（2026-10-04 铺七项 ✓，**2026-10-07 G-89 收口删掉第一项** ✓
//! —— `PROBE_CAP` 那道闸 Lean 4 没有 ✓ ⇒ **闸与出口一起删** ✓，见下面 [`report`] 的说明 ✓）；
//! ② **乙类 4 处**（G-91 第二笔，2026-10-07 ✓，`judge_cache_evicted` … `skeleton_layers_clamped` ✓）
//! —— 它们**全都只变慢 / 掉显示精度** ✓，**没有一处**碰判定 ✓。
//! 它回答的正是那句"**先加计数、跑一遍课程报真实触发次数**"：
//!
//! * 触发 **0** 次 ⇒ 现在没炸，但闸还在 ⇒ **留闸 + 计数 + 断言 0** ✓；
//! * 触发 **> 0** 次 ⇒ **活的内核级降级** ✗ ⇒ 必须真修 ✓。
//!
//! ## 为什么必须有它（教训 ✓）
//!
//! 先前 `PARSE_LIMIT` 那道闸被抓到**纯属侥幸** —— 因为它慢了 30% 才被性能测试注意到 ✗。
//! **只影响正确性、不影响速度的闸永远抓不到** ✗ —— 没有计数出口 = 你、我、CI 全都不会知道 ✗。

use std::sync::atomic::{AtomicU64, Ordering};

/// **闸的「被触发」读数 + 闸本身的取值**（2026-10-04 ✓）。
///
/// ## 为什么闸的取值要能注入（而不是写死 `const`）
///
/// 判据要能回答一句**实验性**的话 ✓：
///
/// > 把这道闸**逼到极限**（`=1`），**结论会不会变**？
///
/// 「预算耗尽**只允许变慢** ✓，绝不允许**变错** ✗」这句话如果是真的 ✓，
/// 那么把闸拧到最小 ⇒ 结论**逐字节相同** ✓（只是慢 ✗）。
/// 写死 `const` 就**做不了这个实验** ✗ ⇒ 只能靠读代码下结论 ✗ —— 而这次
/// 值守抓到的三处「甲类闸」恰恰说明：**读代码得出的"应该没问题"不算证据** ✗。
///
/// ⚠ 这是**取证/排查口** ✓，**不是**用户配置面 ✗ —— 用户面走
/// `CompileOptions` + `sokonanoda.toml [limits]` + CLI（下一笔 ✓，见交接单 §3）。
/// 默认值**一个不动** ✓ ⇒ 零行为变化 ✓（整本课程 `--json` 逐字节相同 ✓）。
/// 命名对齐 Lean 的旋钮名 ✓（`maxHeartbeats` / `maxRecDepth` / `maxSize` ✓）。
pub mod limits {
    use std::sync::OnceLock;

    /// 读一个正整数的环境覆盖（**只读一次** ✓；非法值/缺省 ⇒ `None` ✓）。
    fn env_u32(name: &str) -> Option<u32> {
        std::env::var(name).ok()?.trim().parse::<u32>().ok()
    }

    /// **签名能记多少个参数**（G-90 · `MAX_TRACKED`）。
    ///
    /// 位掩码是 `u64` ⇒ **硬上界 64** ✓（传再大也只到 64 ✓，不许静默截断成错 ✗）。
    /// Lean 的对应物 = `synthInstance.maxSize = 128` ✓（那是另一套表示 ⇒ 终点见交接单 ✓）。
    pub fn max_tracked() -> u32 {
        static V: OnceLock<u32> = OnceLock::new();
        *V.get_or_init(|| env_u32("SOKO_LIMIT_MAX_TRACKED").unwrap_or(64).clamp(1, 64))
    }

    // ── **求解预算三旋钮**（G-88 第二笔 · 2026-10-07 ✓）──────────────────────
    //
    // **登记在同一处** ✓（不是另造一套 ✗）：与上面两个闸取值共用 `env_u32` + `OnceLock`
    // 那套纪律 ✓，名字也沿用 `SOKO_LIMIT_*` 家族 ✓。
    // 取值规则 = [`resolve`]（**纯函数** ✓ ⇒ 单测直接打它，不碰环境、无 `OnceLock` 隔离问题 ✓）：
    // **缺省 / 非法（非数字 / 负数 / 超 `u32`）⇒ 默认值** ✓，再夹到 `[min, max]` ✓。
    //
    // ⚠ 语义**一个字没动** ✓：撞预算 ⇒ **加大预算重试** ✓，升满仍撞 ⇒ **弃权** ✓，
    // **绝不**判否 ✗（见 `crates/front/src/compile/meta.rs` 的 `unify_impl` ✓）。

    /// 求解**步数**预算（fuel）的默认值 = Lean `synthInstance.maxHeartbeats` ✓
    /// （`src/Lean/Meta/SynthInstance.lean:20-23` = **20000** ✓）。
    ///
    /// ⚠ **别拿命令级那个** ✗：Lean 另有一个 `maxHeartbeats`（`src/Lean/CoreM.lean:26-29`
    /// = **200000** ✓）—— 那是**每条命令**的额度 ✓，粒度与我们「一次求解」不同 ✗。
    pub const DEFAULT_MAX_HEARTBEATS: u32 = 20000;

    /// 求解**递归深度**上限的默认值 = Lean `maxRecDepth` ✓
    /// （`src/Lean/Util/RecDepth.lean:15-18` 的 `defValue := defaultMaxRecDepth` ⇒
    /// `src/Init/Prelude.lean:4760` = **512** ✓；Lean 的合一 `isExprDefEqAuxImpl`
    /// 正是被 `withIncRecDepth` 包住的那一处 ✓ —— `src/Lean/Meta/ExprDefEq.lean:2108`）。
    ///
    /// ⚠ 本机 Lean（`~/Documents/lean/lean4`，master `d0493e4c1e` 2026-01-14；tag
    /// `v4.27.0-rc1` `2fcce7258e` 2025-12-14）**两个都是 512** ✓ ——
    /// 台账原先记的 3200 **查不到** ✗（`grep -rn 3200 src/` 只有 JsonRpc 错误码 ✓）。
    pub const DEFAULT_MAX_REC_DEPTH: u32 = 512;

    /// **撞预算后「加大预算重试」几次**（G-88 真修的**上界** ✓）。
    ///
    /// Lean **没有对应物** ✗（Lean 耗尽即报错 ⇒ 见 `Util/RecDepth.lean` 的
    /// `throwMaxRecDepthAt` ✓）⇒ 保持 6 ✓ 不假装对齐 ✓。
    pub const DEFAULT_MAX_ESCALATIONS: u32 = 6;

    /// 升级次数的**配置上界**：每次翻倍 ⇒ 16 次 = 65536× ✓；
    /// 再大只是把 `saturating_mul` 之后的时间无限拉长（挂死风险 ✗）⇒ 夹到 16 ✓
    /// （与 `max_tracked` 的 `clamp(1, 64)` 同一纪律：**上界要有理由** ✓）。
    const MAX_ESCALATIONS_CEILING: u32 = 16;

    /// 取值规则（**纯函数** ✓）：`None`（缺省 / 非法）⇒ `default` ✓，再夹到 `[min, max]` ✓。
    fn resolve(parsed: Option<u32>, default: u32, min: u32, max: u32) -> u32 {
        parsed.unwrap_or(default).clamp(min, max)
    }

    /// **求解步数预算**（G-88 · `meta.rs` 的 `fuel`）—— 环境变量 `SOKO_LIMIT_MAX_HEARTBEATS`。
    ///
    /// ⚠ Lean 的 `maxHeartbeats = 0` 表示**无限制** ✗ —— 我们**不支持**（会挂死 ✗）⇒
    /// 下界夹到 1 ✓；"还要更宽"用有界的 `SOKO_LIMIT_MAX_ESCALATIONS` ✓。
    pub fn max_heartbeats() -> u32 {
        static V: OnceLock<u32> = OnceLock::new();
        *V.get_or_init(|| {
            resolve(
                env_u32("SOKO_LIMIT_MAX_HEARTBEATS"),
                DEFAULT_MAX_HEARTBEATS,
                1,
                u32::MAX,
            )
        })
    }

    /// **求解递归深度上限**（G-88 · `meta.rs` 的 `max_depth`）—— `SOKO_LIMIT_MAX_REC_DEPTH`。
    pub fn max_rec_depth() -> u32 {
        static V: OnceLock<u32> = OnceLock::new();
        *V.get_or_init(|| {
            resolve(
                env_u32("SOKO_LIMIT_MAX_REC_DEPTH"),
                DEFAULT_MAX_REC_DEPTH,
                1,
                u32::MAX,
            )
        })
    }

    /// **撞预算后「加大预算重试」几次**（G-88）—— `SOKO_LIMIT_MAX_ESCALATIONS`。
    ///
    /// `0` = **从不升级**（一撞预算就弃权 ✓ —— 合法配置 ✓，也是判据要的那一档 ✓）。
    pub fn max_escalations() -> u32 {
        static V: OnceLock<u32> = OnceLock::new();
        *V.get_or_init(|| {
            resolve(
                env_u32("SOKO_LIMIT_MAX_ESCALATIONS"),
                DEFAULT_MAX_ESCALATIONS,
                0,
                MAX_ESCALATIONS_CEILING,
            )
        })
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        /// **G-88 判据②**：缺省值 = **Lean 的数值** ✓（出处见各常量 ✓）。
        ///
        /// ⚠ 断言的是**常量**（不是"读环境得到的值" ✗）—— 后者会被外部环境变量污染 ✗
        /// （`SOKO_LIMIT_*` 是**进程级**的，见 `scripts/check-test-env-isolation.py` ✓）。
        #[test]
        fn budget_defaults_are_the_lean_numbers() {
            assert_eq!(
                DEFAULT_MAX_HEARTBEATS, 20000,
                "Lean `synthInstance.maxHeartbeats`（Meta/SynthInstance.lean:20）= 20000"
            );
            assert_eq!(
                DEFAULT_MAX_REC_DEPTH, 512,
                "Lean `maxRecDepth`（Util/RecDepth.lean:15 ⇒ Init/Prelude.lean:4760）= 512"
            );
            assert_eq!(
                DEFAULT_MAX_ESCALATIONS, 6,
                "Lean 没有对应物（耗尽即报错）⇒ 保持 6（不假装对齐）"
            );
        }

        /// **G-88 判据①的解析那一半**：缺省 / 非法 ⇒ 默认 ✓；**拧到极小真的生效** ✓；上下界 ✓。
        #[test]
        fn resolve_falls_back_on_bad_values_and_clamps() {
            let (d, lo, hi) = (DEFAULT_MAX_HEARTBEATS, 1, u32::MAX);
            assert_eq!(resolve(None, d, lo, hi), 20000, "缺省 ⇒ Lean 的数值");
            assert_eq!(resolve(Some(1), d, lo, hi), 1, "拧到 1 ⇒ 必须真的生效");
            assert_eq!(resolve(Some(0), d, lo, hi), 1, "0（Lean 的「无限制」）⇒ 夹到 1");
            assert_eq!(resolve(Some(999_999), d, lo, hi), 999_999, "放大 ⇒ 原样生效");
            let (d, lo, hi) = (DEFAULT_MAX_ESCALATIONS, 0, MAX_ESCALATIONS_CEILING);
            assert_eq!(resolve(None, d, lo, hi), 6, "缺省 ⇒ 6");
            assert_eq!(resolve(Some(0), d, lo, hi), 0, "0 = 从不升级（合法档）");
            assert_eq!(resolve(Some(999), d, lo, hi), 16, "上界 16（防挂死）");
        }

        /// **非法值 ⇒ 回默认** ✓ 的那一半：`env_u32` 只认十进制正整数 ✓。
        #[test]
        fn env_u32_reads_numbers_only() {
            assert_eq!(
                env_u32("SOKO_LIMIT_THIS_NAME_IS_NEVER_SET"),
                None,
                "缺省 ⇒ None ⇒ 走默认值"
            );
            assert_eq!(env_u32("PATH"), None, "非数字（PATH）⇒ None ⇒ 走默认值");
        }
    }
}

/// 一个**只增不减**的闸类计数（`Relaxed` 足够 —— 只做统计 ✓）。
pub struct Counter(AtomicU64);

impl Counter {
    const fn new() -> Self {
        Self(AtomicU64::new(0))
    }

    /// 记一笔（写端 ✓）。
    #[inline]
    pub fn bump(&self) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }

    /// 读当前值 ✓。
    #[inline]
    pub fn get(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }

    /// 归零（判据取**差量**用 ✓ —— 计数是**进程级**的，见交接单 §5 ✓）。
    pub fn reset(&self) {
        self.0.store(0, Ordering::Relaxed);
    }
}

// ⚠ **这里原来有一个出口：`PROBE_CAP` 耗尽**（G-89 · `conv.rs`）——
// **2026-10-07 随闸一起删除** ✓（**不是留一个永不 bump 的空转出口** ✗）。
//
// **为什么删**：Lean 4 **没有**这道闸 ✓（对照见 `docs/notes/HANDOFF-kernel.md` 的
// 「G-89」条 ✓）⇒ 按值守口径**去掉、不许换数字留着** ✗。闸一去，探查**跑到底** ✓
// ⇒ 「耗尽」这件事**不存在了** ⇒ 出口**没有写入点** ⇒ 留着就是空转 ✗（读数永远是 0，
// 而 0 的含义从「没触发」变成「不存在」✗ ⇒ 分不清 = 假守卫 ✗）。
//
// **保留了什么**：`conv.rs` 的**探查本身**一字未动 ✓（`spine_probe` / `probe_pairs` /
// `probe_pass` ✓）—— 它仍是**纯优化** ✓：只可能给出「相等」✓，`false` 永远是
// 「没抄近路 ⇒ 调用方走全量」✓，**没有**「判不了 ⇒ 当成否」✗。
// **代价**（去掉上限后探查可能做得更多）：见 `docs/perf/ledger.jsonl` 与
// `STATUS.md` 第 135 棒 ✓。

/// **`MAX_TRACKED` 丢精度**（G-90 · `relevance.rs`）：参数望远镜超过 64 位 ⇒
/// 签名只能记到 64 个 ⇒ `terminal = None` / `result_known` 不再置位 ⇒
/// 相等性的**相关性捷径静默失效** ✗（**只该变慢** ✓）。
///
/// Lean 的对应物是 `synthInstance.maxSize = 128` ✓ ⇒ 终点是**对齐到 128** ✓。
pub static SIG_OVERFLOW: Counter = Counter::new();

/// **`MAX_TRACKED` 在 `conv.rs` 的应用点被截断**（G-90 的第二个落点）：
/// `k >= MAX_TRACKED` ⇒ 这一层的相关性判定**放弃**（同样只该变慢 ✓）。
pub static SIG_ARITY_CLAMPED: Counter = Counter::new();

/// **`unify_all` 的 `MAX_ROUNDS` 用光**（G-88 的邻居 ✓）：跑到轮数上限仍未到不动点
/// ⇒ 返回 `Tri::Undef` ✓。
///
/// ⚠ 这一条**是正当的** ✓（`Undef` = **弃权** ⇒ 走慢路 ✓，与 Lean 的
/// `processPostponed` 同款 ✓）—— 但**仍然要有出口** ✗：没有出口就分不清
/// 「弃权了」和「没弃权」✓。
pub static UNIFY_ROUNDS_EXHAUSTED: Counter = Counter::new();

/// **`unify_all` 因"没有净进展"停下**（邻居 ✓，同样是 `Tri::Undef` = 弃权 ✓）。
pub static UNIFY_NO_PROGRESS: Counter = Counter::new();

/// **`unify_impl` 的 `fuel` / `max_depth` 用光、且升级额度也用光 ⇒ 弃权** ✓（**G-88 本体** ·
/// `crates/front/src/compile/meta.rs`）。
///
/// ⚠ 它曾经是**唯一一处**「判不了 ⇒ **当成否**」的内核级分支 ✗ —— 学习者的长证明会被判成
/// 「解不出来」，而**那不是真的无解** ✗。**2026-10-04 已真修** ✓：撞预算 ⇒ 加大预算重试 ✓，
/// 升满仍撞 ⇒ **弃权 `Tri::Undef`** ✓（**绝不** `Tri::No` ✗）⇒ 本计数现在只数"弃权" ✓。
/// **2026-10-07 收口**（G-88 第二笔 ✓）：两个旋钮**可配置** ✓（`limits::{max_heartbeats,
/// max_rec_depth,max_escalations}` ✓）且**默认值 = Lean 的数值** ✓（20000 / 512 ✓，出处见常量 ✓）。
pub static META_BUDGET_EXHAUSTED: Counter = Counter::new();

/// **撞预算后「加大预算重试」的次数** ✓（G-88 真修的**主**读数 ✓）。
///
/// 它 **> 0** 只说明"变慢了" ✓（答案不变 ✓）—— 与 `META_BUDGET_EXHAUSTED`
/// （**升级到底仍弃权** ✗）分开数 ✓，两者含义完全不同 ✗：
/// 前者 = 走慢路 ✓；后者 = 判不了 ⇒ 弃权（**不是**判否 ✗）。
pub static META_BUDGET_ESCALATED: Counter = Counter::new();

// ===========================================================================
// **乙类 4 处**（G-91 第二笔，2026-10-07 ✓）—— 与上面七项同一个总规矩 ✓：
// 「超过某个数字就换一条路」必须**看得见** ✗；下面四处**全都只允许变慢 / 掉
// 显示精度** ✓，**一处都不许**改判定 ✓（`判不了 ⇒ 走慢路` ✓ / `当成否` ✗）。
// ===========================================================================

/// **判定结果缓存被 FIFO 挤掉**（`JUDGE_CACHE_CAP = 4096` · `judge.rs`
/// `judge_cache_put` / `type_cache_put`）⇒ 被挤掉的那条**下次必然 miss** ⇒
/// 重跑一遍整份前缀 ✓。
///
/// **触发意味着什么**：**只变慢** ✓（重算给出**一模一样**的结论 ✓ —— 缓存是
/// 纯记忆化，不是判定的一部分 ✓）。但它**必须看得见** ✗：容量一旦小于工作集，
/// 缓存就从"命中"退化成"抖动"，而 `hits/misses` 分不出"新键"和"被挤掉" ✓。
/// **计数点**：`while cache.1.len() > JUDGE_CACHE_CAP { … }` 每淘汰一条 ✓
/// （两张表共用这一个常数、同一套 FIFO ✓ ⇒ 共用一个出口 ✓）。
pub static JUDGE_CACHE_EVICTED: Counter = Counter::new();

/// **常量签名缓存满了 ⇒ 静默停止写入**（`judge_type_of_constant` ·
/// `judge.rs`：`if c.len() < CAP { c.insert(…) }`，`CAP = 4096`）⇒ 新条目**不再
/// 进表** ⇒ 下次还走全前缀重编译 ✓。
///
/// **触发意味着什么**：**只变慢** ✓（少缓存一条 = 多算一次，结论不变 ✓）。
/// ⚠ 与 [`JUDGE_CACHE_EVICTED`] 不同型 ✗：那边是"旧的被挤掉"，这边是"新的进不来"
/// —— 表满了之后**一个字节都不再增长** ✓，所以它是最容易被误读成"缓存正常"的
/// 那种饱和 ✗（`hits/misses` 同样分不出来 ✓）。
/// **计数点**：`if c.len() < CAP` 的 **else**（即"本该写但写不进去"那一次 ✓）。
pub static CONST_SIG_CACHE_FULL: Counter = Counter::new();

/// **目标分解失败 ⇒ 退回 generic 兜底**（`goals.rs::open_goal` 整条返回 `None`
/// 而值里有洞 ⇒ `walk.rs` 的 `None if expr_has_hole(val)` 分支 ✓）。
///
/// **触发意味着什么**：**判定不变** ✓（值位仍是一个可填的练习，不是报错 ✓）、
/// **显示降质** ✗：题面状态退回"整句声明类型 / `src_root` 兜底"，子洞期望类型
/// 也不再精确 ✓（G-82 修过的那条路 ✓）。
/// ⚠ **只数"整条走查失败"** ✗ —— `goals.rs` 内部各策略（ctor / func / 依赖
/// motive…）的 `return None` 是**正常的不匹配** ✓（那条形状不归它管，另一条会接 ✓），
/// 数进去只会把正常路径也计成降级 ⇒ 读数失去意义 ✗。
/// **计数点**：`walk.rs` 里真正**构造 generic 兜底**的那两处（`Theorem` / `Example` ✓）。
pub static GOAL_DECOMPOSE_FALLBACK: Counter = Counter::new();

/// **重启骨架被 `SKELETON_MAX_LAYERS = 3` 截断**（`suggest.rs::restart_skeleton`）。
///
/// **触发意味着什么**：**只掉提示精度** ✓（骨架少剥几层 Pi，学习者补上剩下的
/// `fun … =>` 即可 ✓），**判定一字不动** ✓（建议本身 `verified: false` ✓）。
/// ⚠ 只有"**还有没剥完的望远镜**"才算触发 ✗ —— 望远镜**恰好 3 层**时循环也会
/// 在同一个 `if` 上退出，但那是**正常终止** ✓（数进去 = 假读数 ✗）。
/// **计数点**：`restart_skeleton` 收口处的 `truncated` 标志 ✓（两处 break 合并判一次 ✓）。
pub static SKELETON_LAYERS_CLAMPED: Counter = Counter::new();

/// **一次性读数**（`(名字, 次数)` ✓）——给 `STAGE_STATS` / 判据用 ✓。
///
/// 顺序**固定** ✓（判据要能按位置读，不许靠 map 顺序 ✗）；
/// **前六项的位置一个都不许动** ✗ —— 乙类四项**追加在末尾** ✓（6 → 10 ✓）。
///
/// ⚠ **G-89 收口**（2026-10-07 ✓）：第一项（`PROBE_CAP` 的耗尽计数）**已删** ✓
/// —— 闸没了、没有写入点 ⇒ 留着是**空转出口** ✗（见上面那段说明 ✓）。
/// 七项 → **六项**，乙类仍在末尾 ⇒ 现在共 **10** 项 ✓。
pub fn report() -> [(&'static str, u64); 10] {
    [
        ("sig_overflow", SIG_OVERFLOW.get()),
        ("sig_arity_clamped", SIG_ARITY_CLAMPED.get()),
        ("unify_rounds_exhausted", UNIFY_ROUNDS_EXHAUSTED.get()),
        ("unify_no_progress", UNIFY_NO_PROGRESS.get()),
        ("meta_budget_exhausted", META_BUDGET_EXHAUSTED.get()),
        ("meta_budget_escalated", META_BUDGET_ESCALATED.get()),
        // —— 乙类 4 处（G-91 第二笔 ✓，**追加** ✓）——
        ("judge_cache_evicted", JUDGE_CACHE_EVICTED.get()),
        ("const_sig_cache_full", CONST_SIG_CACHE_FULL.get()),
        ("goal_decompose_fallback", GOAL_DECOMPOSE_FALLBACK.get()),
        ("skeleton_layers_clamped", SKELETON_LAYERS_CLAMPED.get()),
    ]
}

/// **所有闸都归零** ✓（判据取差量用 ✓）。
pub fn reset() {
    for c in [
        &SIG_OVERFLOW,
        &SIG_ARITY_CLAMPED,
        &UNIFY_ROUNDS_EXHAUSTED,
        &UNIFY_NO_PROGRESS,
        &META_BUDGET_EXHAUSTED,
        &META_BUDGET_ESCALATED,
        &JUDGE_CACHE_EVICTED,
        &CONST_SIG_CACHE_FULL,
        &GOAL_DECOMPOSE_FALLBACK,
        &SKELETON_LAYERS_CLAMPED,
    ] {
        c.reset();
    }
}

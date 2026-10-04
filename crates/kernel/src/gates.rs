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

/// **`PROBE_CAP` 耗尽**（G-89 · `conv.rs`）：相等性**探查**的步数预算用光 ⇒
/// `probe_exhausted = true`。
///
/// ⚠ **Lean 4 没有此物** ✓ ⇒ 终点是**去掉**（**不许换个数字继续留着** ✗）。
/// 但"去掉"之前必须先看清它现在触发多少次 ✓ —— 这个计数就是那一步 ✓。
/// 另：耗尽**只许**表示「**这次探查不可信 ⇒ 弃权走全量**」✓，**绝不许**表示「不相等」✗。
pub static PROBE_EXHAUSTED: Counter = Counter::new();

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

/// **`unify_impl` 的 `fuel` / `MAX_DEPTH` 用光 ⇒ `Tri::No`** ✗（**G-88 本体** ·
/// `crates/front/src/compile/meta.rs`）。
///
/// ⚠ 这是**唯一一处**「判不了 ⇒ **当成否**」的内核级分支 ✗ ——
/// 学习者的长证明会被判成「解不出来」，而**那不是真的无解** ✗。
/// 终点（值守 13:21/13:24 两步走 ✓）：① 可配置化（默认值不动 ⇒ 零行为变化 ✓）；
/// ② 放宽到 Lean 的数值（`maxHeartbeats 4096 → 20000` · `maxRecDepth 64 → 3200` ✓）。
pub static META_BUDGET_EXHAUSTED: Counter = Counter::new();

/// **一次性读数**（`(名字, 次数)` ✓）——给 `STAGE_STATS` / 判据用 ✓。
///
/// 顺序**固定** ✓（判据要能按位置读，不许靠 map 顺序 ✗）。
pub fn report() -> [(&'static str, u64); 6] {
    [
        ("probe_exhausted", PROBE_EXHAUSTED.get()),
        ("sig_overflow", SIG_OVERFLOW.get()),
        ("sig_arity_clamped", SIG_ARITY_CLAMPED.get()),
        ("unify_rounds_exhausted", UNIFY_ROUNDS_EXHAUSTED.get()),
        ("unify_no_progress", UNIFY_NO_PROGRESS.get()),
        ("meta_budget_exhausted", META_BUDGET_EXHAUSTED.get()),
    ]
}

/// **所有闸都归零** ✓（判据取差量用 ✓）。
pub fn reset() {
    for c in [
        &PROBE_EXHAUSTED,
        &SIG_OVERFLOW,
        &SIG_ARITY_CLAMPED,
        &UNIFY_ROUNDS_EXHAUSTED,
        &UNIFY_NO_PROGRESS,
        &META_BUDGET_EXHAUSTED,
    ] {
        c.reset();
    }
}

//! **G-89 收口守卫**（2026-10-07 ✓）：相等性探查**没有步数预算** ✓，
//! 也**没有**「耗尽」出口 ✓ —— 谁把闸装回来，这里就判红 ✓。
//!
//! ## 为什么这里只能钉**结构**（行为夹具在 front 侧 ✓）
//!
//! 探查是**纯优化** ✓（只可能给出「相等」✓，`false` 永远是「没抄近路 ⇒ 调用方
//! 走全量」✓）⇒ **判定输出对它免疫** ✓：闸在 / 闸不在，整门课 `--json` 逐字节相同 ✓
//! （实测见 `STATUS.md` 第 135 棒 ✓）。所以「闸没了」这件事**行为上不可观测** ✗
//! —— 结构判据在这里，行为判据（该判相等的仍判相等 ✓ / 刚性不等仍判不等 ✓ /
//! 以前会耗尽旧预算的夹具现在仍判相等 ✓）在
//! `crates/front/src/compile/tests.rs` 的 `probe_*` 三条里 ✓。
//!
//! ⚠ 断言里的**针**一律用 `concat!` 拼 ✓ —— 否则断言自己会把针写进被检查的源码
//! ⇒ `contains` 恒真/恒假、**自咬** ✗。

/// **闸与出口都不许回来** ✓ + **探查本身必须还在** ✓（本条只许去掉预算 ✗）。
#[test]
fn probe_has_no_step_budget_and_no_exhaustion_outlet() {
    let conv = include_str!("../conv.rs");
    let gates = include_str!("../gates.rs");

    // ① 闸的痕迹：旋钮 / 步数预算字段 / 耗尽标志 / 只在探查内读的截断态否定缓存 ✓。
    //    （Lean 4 的对应处只有 `withIncRecDepth` 那个**全局**递归深度 ✓ ——
    //     `Meta/ExprDefEq.lean:2108`；**没有**「探查步数上限」这种东西 ✓。）
    for needle in [
        concat!("probe", "_cap"),
        concat!("probe", "_budget"),
        concat!("probe", "_exhausted"),
        concat!("conv_cache_neg", "_probe"),
        concat!("PROBE", "_EXHAUSTED"),
    ] {
        assert!(
            !conv.contains(needle) && !gates.contains(needle),
            "**G-89**：探查预算的痕迹 `{needle}` 又回到了 kernel 源码里 ✗ —— \
             Lean 4 没有这道闸（`Meta/ExprDefEq.lean` 只有 `withIncRecDepth` 那个\
             **全局**递归深度 ✓）⇒ 去掉，**不许换个数字留着** ✗"
        );
    }
    // ② `report()` 里也不许再有那个槽位（否则 `STAGE_STATS` 多一个**永远读 0** 的读数 ✗：
    //    0 的含义会从「没触发」变成「这条闸不存在」⇒ 分不清 = 假守卫 ✗）。
    assert!(
        !gates.contains(concat!("\"probe", "_exhausted\"")),
        "**G-89**：`gates::report()` 里又出现了探查耗尽槽位 ✗ —— 闸没了就没有写入点 \
         ⇒ 它会**永远读 0**（空转出口 ✗）"
    );
    // ③ 反向：探查**必须还在** ✓（删掉探查 = 改判定 ✗，不是本条的目的 ✗）。
    for needle in ["fn spine_probe", "fn probe_pairs", "fn probe_pass"] {
        assert!(
            conv.contains(needle),
            "**G-89**：探查 `{needle}` 不见了 ✗ —— 本条只去掉**预算** ✓，\
             探查本身是纯优化、一个字都不许动 ✗"
        );
    }
}

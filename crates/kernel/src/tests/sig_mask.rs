//! **G-90 收口守卫**（2026-10-07 ✓）：签名相关性掩码的载体是 **`u128`**（128 位 ✓）、
//! 闸值默认 **128** ✓ —— 谁把它拧回 64 / `u64`，这里就判红 ✓。
//!
//! ## 这一条钉的是什么（缺口原文 ✓）
//!
//! `MAX_TRACKED = 64` ⇒ **常量参数 ≥ 64 个就丢精度** ✗（`terminal = None` ⇒
//! `result_known` 不再置位 ⇒ 相等性的相关性捷径静默失效 ✗）。三条出口**全是保守方向** ✓
//! （丢精度 ⇒ **多比**，绝不少比 ✗）⇒ 只变慢 ✓，但**必须可见** ✗。
//! 终点 = 对齐 Lean 4 的 `synthInstance.maxSize = 128` ✓（**数字**对齐 ✓，语义不同 ✓：
//! Lean 拿它限 typeclass 求解的实例项规模 ✓，见 `gates.rs::limits::max_tracked` 的对照 ✓）。
//!
//! ## 为什么这一半在 kernel（另一半在 `crates/front/tests/g90_max_tracked.rs` ✓）
//!
//! * **这里** = 掩码**语义**（哪一位能表示、掩码外必须保守 ✓）+ **结构**（载体 / 默认值 /
//!   计数出口还在 ✓）—— 不需要 env、不需要语料，毫秒级 ✓；
//! * **那边**（独立进程 ✓）= **真实签名**（64..129 个参数的公理 ✓）+ **计数差量**
//!   （`sig_overflow` / `sig_arity_clamped` ✓）+ **判定方向**（假的相等仍被拒 ✗）。
//!
//! ⚠ 断言里的**针**一律用 `concat!` 拼 ✓（否则断言自己会把针写进被检查的源码
//! ⇒ `contains` 自咬 ✗，同 `tests/probe.rs` ✓）。

use crate::relevance::{max_tracked, Sig};

fn sig(prop_arg: u128, arg_known: u128, absent_arg: u128, prop_result: u128, result_known: u128) -> Sig {
    Sig { arity: 0, prop_arg, arg_known, absent_arg, prop_result, result_known }
}

/// **掩码语义**：第 0 / 63 / 64 / 127 位都要能**独立**表示 ✓，
/// 掩码**外**（第 128 位）必须**保守**（`false` = 「不可忽略 ⇒ 照样比」✓）。
///
/// ⚠ 取证口 `SOKO_LIMIT_MAX_TRACKED` 被拧过时**跳过**默认值相关的下标断言 ✓
/// —— 那是**排查口** ✓，判据不许被它判红 ✗（拧到 1 的实验见 `scripts/limits-only-slow.sh` ✓）。
#[test]
fn sig_masks_carry_128_argument_bits() {
    let dialed = std::env::var_os("SOKO_LIMIT_MAX_TRACKED").is_some();
    if !dialed {
        assert_eq!(max_tracked(), 128, "G-90：`MAX_TRACKED` 的默认值必须是 **128** ✗（对齐 Lean ✓）");

        // ① 每一位**独立** ✓：第 64 位不许绕回第 0 位 ✗（`u64` 时代 `1u64 << 64` 就是绕回 ✗）。
        for i in [0u32, 63, 64, 127] {
            let s = sig(1u128 << i, 1u128 << i, 0, 0, 0);
            assert!(s.arg_is_ignorable(i), "G-90：第 {i} 位必须能表示（命题参数 ⇒ 可忽略 ✓）");
            assert!(s.masks_any_arg());
            for other in [0u32, 63, 64, 127].into_iter().filter(|o| *o != i) {
                assert!(!s.arg_is_ignorable(other), "G-90：第 {i} 位不许撞到第 {other} 位 ✗（载体必须 128 位 ✓）");
            }
        }

        // ② 结果格：第 127 格能判 ✓；**第 128 格在掩码外** ⇒ 保守 `false` ✓（照样比 ✓）。
        let s = sig(0, 0, 0, 0, 1u128 << 127);
        assert!(s.result_is_not_proof(127), "G-90：第 127 格结果必须能判 ✓");
        assert!(!s.result_is_not_proof(128), "G-90：掩码外必须保守（false = 不抄近路 ⇒ 照样比 ✓）");

        // ③ `absent_arg`（第三条出口 ✓）同样要能表示第 127 位 ✓。
        let s = sig(0, 0, 1u128 << 127, 0, 0);
        assert!(s.arg_is_ignorable(127), "G-90：`absent_arg` 的第 127 位必须能表示 ✓");
    } else {
        assert!(max_tracked() <= 128, "G-90：取证口也不许把闸拧到载体之外 ✗（上界 = 128 ✓）");
    }

    // ④ 方向：全零掩码 ⇒ 一条捷径都不许有 ✓（`masks_any_arg` = false ⇒ 调用方走全量 ✓）。
    let none = Sig::ALL_RELEVANT;
    assert!(!none.masks_any_arg());
    for i in [0u32, 63, 64, 127, 128, 129] {
        assert!(!none.arg_is_ignorable(i), "G-90：掩码为 0 ⇒ 不许可忽略 ✗（那是「判不了 ⇒ 当成是」✗）");
        assert!(!none.result_is_not_proof(i), "G-90：掩码为 0 ⇒ 不许判「不是命题」✗");
    }
}

/// **载体 + 默认值 + 计数出口**（结构 ✓）：`u64` / 64 的痕迹**不许回来** ✗，
/// 两个计数出口必须还在 ✓ 且**仍有写入点** ✓（≥128 才该触发 ✓）。
#[test]
fn max_tracked_is_128_and_the_carrier_holds_128_bits() {
    let gates = include_str!("../gates.rs");
    let rel = include_str!("../relevance.rs");
    let conv = include_str!("../conv.rs");

    // ① 默认值与上界（缺口原文：`unwrap_or(64).clamp(1, 64)` ✗）。
    assert!(
        gates.contains(concat!("unwrap_or(128)", ".clamp(1, 128)")),
        "G-90：`MAX_TRACKED` 的默认值/上界必须是 **128** ✗（缺口原文是 `unwrap_or(64).clamp(1, 64)` ✗）"
    );
    assert!(
        !gates.contains(concat!("unwrap_or(64)", ".clamp(1, 64)")),
        "G-90：64 / `clamp(1, 64)` 又回来了 ✗ —— 参数 ≥64 个就丢精度的缺口会跟着回来 ✗"
    );

    // ② 载体：五个掩码都是 `u128` ✓（`u64` 装不下 64..128 个参数 ✗）。
    for field in ["prop_arg", "arg_known", "absent_arg", "prop_result", "result_known"] {
        assert!(
            rel.contains(&format!("{field}: u128")),
            "G-90：`Sig::{field}` 的载体必须是 `u128` ✗（`u64` 装不下 128 位 ✗）"
        );
        assert!(!rel.contains(&format!("{field}: u64")), "G-90：`Sig::{field}` 又变回 `u64` 了 ✗");
    }
    // 载体宽度兜底：五个掩码 ⇒ ≥ 80 字节（`u64` 时代是 40 ✗）。
    assert!(
        std::mem::size_of::<Sig>() >= 80,
        "G-90：`Sig` 太小（{} 字节）⇒ 掩码载体不像 128 位 ✗",
        std::mem::size_of::<Sig>()
    );

    // ③ 两个计数出口必须还在 ✓（只声明不写 = 空转 ✗）——「闸还在」这件事必须**可见** ✗。
    assert!(rel.contains(concat!("crate::gates::SIG_OVERFLOW", ".bump();")), "G-90：`SIG_OVERFLOW` 的写入点不见了 ✗");
    assert!(
        conv.contains(concat!("crate::gates::SIG_ARITY_CLAMPED", ".bump();")),
        "G-90：`SIG_ARITY_CLAMPED` 的写入点不见了 ✗"
    );
    assert!(gates.contains("pub static SIG_OVERFLOW"), "G-90：`SIG_OVERFLOW` 出口不见了 ✗");
    assert!(gates.contains("pub static SIG_ARITY_CLAMPED"), "G-90：`SIG_ARITY_CLAMPED` 出口不见了 ✗");

    // ④ 第三条出口（`absent_args`）的**精确查询**不许被简化回位掩码快路 ✗：
    //    值体的 loose bvar 超过 64 时，`Expr::fv_mask`（u64）**不够用** ⇒ 必须逐位查 ✓。
    assert!(
        rel.contains(concat!("!self.ctx.", "has_loose_bvar(body, j as u16)")),
        "G-90：`absent_args` 的精确查询不见了 ✗ —— `fv_mask` 是 u64，`arity > 64` 时\
         它给不出第 64..127 位 ⇒ 会**少比**（变错 ✗），不是变慢 ✓"
    );
}

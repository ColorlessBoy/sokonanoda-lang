//! **G-90 收口判据**（2026-10-07 ✓）：`MAX_TRACKED` **64 → 128** ✓、载体 **`u64` → `u128`** ✓
//! —— 「常量参数 ≥ 64 个 ⇒ 签名丢精度」✗ 这条缺口的行为面判据 ✓。
//!
//! ## 为什么在**独立文件**里（一个文件 = 一个进程 ✓）
//!
//! 计数是**进程级**的（`gates.rs` 的 `AtomicU64` ✓）⇒ 同一个测试二进制里多个
//! `#[test]` **并行**跑会互相看见对方的 bump ✗（`crates/front/src/compile/tests.rs`
//! 里那条只能断言**单调方向** ✓）。本条要断言的是「**不涨**」（= 不再丢精度 ✓）
//! ⇒ 必须**独占进程** ✓：本文件只放**一个** `#[test]`，且这些计数在别处**没有**
//! 写入者（写入者只在 `sig_compute` / `statically_not_proof` 里 ✓，触发条件是
//! **≥128 个参数** ⇒ 只有本文件的夹具够得着 ✓）。
//!
//! ## 判据（四向，缺一不算 ✓）
//!
//! ① **控制组**：小 arity（2）**不许**触发 ✓（判据不许过敏 ✗）；
//! ② **64..127**：加宽后**不再丢精度** ⇒ `sig_overflow` / `sig_arity_clamped`
//!    **一个都不许涨** ✓（修前：64 就涨 ✗ —— 这就是缺口本身 ✓）；
//! ③ **128 / 129**：参数掩码记满 128 位 ✓，但「全应用后的结果格」（第 129 格）与
//!    `k ≥ 128` 的实参个数仍在掩码外 ⇒ **计数照旧咬得住** ✓（保守 ✓，**≥128 才触发** ✓）；
//! ④ **方向**：假的相等**必须仍被拒** ✗ —— 尤其第三条出口（`absent_args` ✓）：
//!    值体**用到了第 1 个参数**（它在值体里的 de Bruijn 下标 = 127 ⇒ 超出
//!    `Expr::fv_mask` 的 64 位 ✓）⇒ 谁把它算成"没用"就会**少比**（变错 ✗）。

use sokonanoda::gates::{SIG_ARITY_CLAMPED, SIG_OVERFLOW};
use sokonanoda_front::compile::{compile_all_units, CompileOptions, SourceUnit};
use sokonanoda_front::parse;

/// `n` 元公理，**结果位是 Prop**（⇒ 两个应用会被拿去比 ✓）+ 两个命题实参
/// `p` / `q`（只有**证明无关**能让它们相等 ✓ ⇒ 一定会走到相关性捷径 ✓）。
fn prop_sig_accepts(n: usize) -> String {
    let doms = std::iter::once("A".to_string())
        .chain(std::iter::repeat_n("Nat".to_string(), n - 1))
        .collect::<Vec<_>>()
        .join(" -> ");
    let ap = std::iter::once("p".to_string())
        .chain(std::iter::repeat_n("0".to_string(), n - 1))
        .collect::<Vec<_>>()
        .join(" ");
    let aq = std::iter::once("q".to_string())
        .chain(std::iter::repeat_n("0".to_string(), n - 1))
        .collect::<Vec<_>>()
        .join(" ");
    format!(
        "axiom A : Prop\n\
         axiom p : A\n\
         axiom q : A\n\
         axiom G{n} : {doms} -> Prop\n\
         axiom h{n} : G{n} {aq}\n\
         theorem t{n} : G{n} {ap} := h{n}\n"
    )
}

/// `n` 元 `def`，值体**只用到第 1 个参数**（de Bruijn 下标 = `n - 1`）⇒
/// 只有**精确**的 used 判定才敢说"别的参数没用" ✓（`n ≥ 65` 时 `fv_mask` 够不着 ✓）。
fn absent_probe_rejects(n: usize) -> String {
    let binders = (1..=n)
        .map(|i| format!("(a{i} : Nat) -> "))
        .collect::<Vec<_>>()
        .join("");
    let funs = (1..=n)
        .map(|i| format!("fun (a{i} : Nat) => "))
        .collect::<Vec<_>>()
        .join("");
    let lhs = std::iter::once("1".to_string())
        .chain(std::iter::repeat_n("0".to_string(), n - 1))
        .collect::<Vec<_>>()
        .join(" ");
    let rhs = std::iter::once("2".to_string())
        .chain(std::iter::repeat_n("0".to_string(), n - 1))
        .collect::<Vec<_>>()
        .join(" ");
    format!(
        "def g{n} : {binders}Nat := {funs}a1\n\
         theorem bad{n} : g{n} {lhs} = g{n} {rhs} := by rfl\n"
    )
}

/// `n` 元公理上的**假**相等（最后一个数据实参不同）⇒ 必须被拒 ✗。
fn wrong_last_arg_rejects(n: usize) -> String {
    let doms = std::iter::once("A".to_string())
        .chain(std::iter::repeat_n("Nat".to_string(), n - 1))
        .collect::<Vec<_>>()
        .join(" -> ");
    let head = std::iter::once("p".to_string())
        .chain(std::iter::repeat_n("0".to_string(), n - 2))
        .collect::<Vec<_>>()
        .join(" ");
    format!(
        "axiom A : Prop\n\
         axiom p : A\n\
         axiom G{n} : {doms} -> Prop\n\
         theorem bad{n} : G{n} {head} 0 = G{n} {head} 1 := by rfl\n"
    )
}

/// 编译一份源码，返回 `(错误条数, (sig_overflow, sig_arity_clamped))` ✓。
fn compile_counters(src: &str) -> (usize, (u64, u64)) {
    let file = parse(src).unwrap_or_else(|e| panic!("parse failed: {e:?}\n{src}"));
    let units = [SourceUnit::single("G90", &file)];
    let before = (SIG_OVERFLOW.get(), SIG_ARITY_CLAMPED.get());
    let (out, _reports) = compile_all_units(&units, &CompileOptions::default());
    let after = (SIG_OVERFLOW.get(), SIG_ARITY_CLAMPED.get());
    (out.errors.len(), (after.0 - before.0, after.1 - before.1))
}

#[test]
fn max_tracked_is_exact_up_to_128_and_counted_beyond() {
    // ① 控制组：小 arity ⇒ 两个计数都不许涨 ✓（判据过敏 = 假红 ✗）。
    let (errs, (sig, clamp)) = compile_counters(&prop_sig_accepts(2));
    assert_eq!(errs, 0, "控制组必须接受 ✓");
    assert_eq!(
        (sig, clamp),
        (0, 0),
        "G-90：小 arity 不该碰闸 ✗（判据空转/过敏 ✗）"
    );

    // ② 64..127：**加宽后不再丢精度** ⇒ 计数一个都不许涨 ✓
    //    （修前实测：64 就已经 `sig_overflow=2` ✗ —— 见本条的 commit message ✓）。
    for n in [64usize, 65, 127] {
        let (errs, (sig, clamp)) = compile_counters(&prop_sig_accepts(n));
        assert_eq!(errs, 0, "n={n}：证明无关仍必须让它通过 ✓");
        assert_eq!(
            (sig, clamp),
            (0, 0),
            "G-90：n={n} 个参数**不许**再丢精度 ✗（`sig_overflow`/`sig_arity_clamped` 涨了\
             ⇒ 掩码没记全 ⇒ 缺口还在 ✗）"
        );
    }

    // ③ 128 / 129：参数掩码记满 128 位 ✓，掩码外的那些格/实参仍在 ⇒ **计数咬得住** ✓
    //    （保守方向 ✓：多比一次，绝不"判不了 ⇒ 判否" ✗）。
    for n in [128usize, 129] {
        let (errs, (sig, clamp)) = compile_counters(&prop_sig_accepts(n));
        assert_eq!(errs, 0, "n={n}：丢精度只该**变慢** ✓ —— 判定不许变 ✗");
        assert!(
            sig >= 1 && clamp >= 1,
            "G-90：n={n} 已到/超过闸值 ⇒ `sig_overflow`/`sig_arity_clamped` **必须**涨 ✓\
             （读到 sig={sig} clamp={clamp} ⇒ 出口咬不住 = 假守卫 ✗）"
        );
    }

    // ④ 方向：假的相等**必须仍被拒** ✗ —— 三条出口全是保守方向 ✓，加宽不许变成"更能接受" ✗。
    for n in [127usize, 129] {
        let (errs, _) = compile_counters(&wrong_last_arg_rejects(n));
        assert!(
            errs > 0,
            "G-90：n={n} 上最后一个实参不同 ⇒ 必须拒绝 ✗（少比 = 变错 ✗）"
        );
    }
    // 第三条出口（`absent_args` ✓）的**精确**方向：`n = 128` 的值体用到 de Bruijn 下标 127
    // （`Expr::fv_mask` 是 u64 ⇒ 够不着 ✓）⇒ 谁把它当"没用"就会错误接受 ✗。
    for n in [64usize, 128] {
        let (errs, _) = compile_counters(&absent_probe_rejects(n));
        assert!(
            errs > 0,
            "G-90：`g{n} a1 … = g{n} a1+1 …` 必须拒绝 ✗ —— 值体**用到了** a1\
             （其 de Bruijn 下标 = {} ⇒ 超出 `fv_mask` 的 64 位 ✓）⇒ 判成 absent = 少比 = 变错 ✗",
            n - 1
        );
    }
}

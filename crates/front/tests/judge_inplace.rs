//! **P1-a 第一步的判据**：就地判定（`SOKO_JUDGE_INPLACE`）与"源码重跑整段前缀"
//! 必须给出**逐字节相同**的答案，而且**真的被走到**。
//!
//! **为什么放集成测试**：`inplace_report()` 是**进程级**计数，开关又只读一次环境
//! ⇒ 放进 lib 测试会被并行的别的测试干扰（与 `session_reuse.rs` 同一个理由 ✓）；
//! 集成测试各自独立进程 ⇒ 天然隔离 ✓。
//!
//! 三条判据（缺一不算成立）：
//! 1. **两条路答案逐字节相同** ⇒ `shadow_diff == 0`（`SOKO_JUDGE_INPLACE=shadow`
//!    下两条都跑、比对文本，返回的仍是源码重跑那份 ⇒ 判定本身逐字节不变 ✓）；
//! 2. **就地路径真的走到了**（`shadow_same > 0`）—— 否则判据**空转** ✗
//!    （"咬不住的守卫等于没有"：一个永远走不到的实现也能让 1 变绿）；
//! 3. **反向判据：前缀真变必须跟着变** —— 同一段项，前缀里少一条声明 ⇒
//!    结论**必须不同**；堵死"把某一趟的结论缓存住、跨前缀复用"的实现 ✗。
use sokonanoda_front::compile::{compile_fol_with, CompileOptions};
use sokonanoda_front::judge::inplace_report;
use sokonanoda_front::parse;

/// 编译一份源码，返回**可逐字节比对**的文本（解析失败也算一种结论）。
fn compile(src: &str) -> String {
    let Ok(file) = parse(src) else {
        return "parse-error".to_string();
    };
    let out = compile_fol_with(&file, &CompileOptions::default());
    format!(
        "errors={:?}\nevents={:?}\nwarnings={:?}",
        out.errors, out.events, out.warnings
    )
}

/// 走 `elab_notation`（`=` 的宇宙层级查询）+ `try_implicit_application`
/// （隐式前导实参的贴合判据）的一段真源码：两条被接线的判定点都要踩到。
const MAIN: &str = "\
theorem eq_symm_nat (n m : Nat) (h : n = m) : m = n := Eq.symm h
theorem and_comm_nat (P Q : Prop) (h : P ∧ Q) : Q ∧ P := And.intro (And.right h) (And.left h)
";

/// 反向判据用的两段：**完全相同的声明体**，只有**前缀里那条 `infix` 指向谁**不同
/// （一个指向存在的 `rr`、一个指向不存在的 `nosuchrel`）⇒ 结论必须跟着前缀变。
const PREFIX_GOOD: &str = "\
axiom Rel : Type
axiom rr : Rel -> Rel -> Prop
infix:50 \" ~~ \" => rr
theorem uses_it (x : Rel) : x ~~ x := by sorry
";
const PREFIX_BAD: &str = "\
axiom Rel : Type
axiom rr : Rel -> Rel -> Prop
infix:50 \" ~~ \" => nosuchrel
theorem uses_it (x : Rel) : x ~~ x := by sorry
";

#[test]
fn inplace_shadow_never_diverges_and_is_actually_used() {
    // 开关只读一次环境 ⇒ 必须在**本进程第一次判定之前**设好（本文件只有一个测试
    // ⇒ 不存在与别的测试抢环境变量的竞态 ✓）。
    std::env::set_var("SOKO_JUDGE_INPLACE", "shadow");

    // ① 判据 2：就地路径**真的走到了**（走不到 ⇒ 下面那条断言毫无意义）。
    let _ = compile(MAIN);
    let (used, _fallback, same, diff) = inplace_report();
    assert!(
        same > 0,
        "就地路径一次都没走到（same=0, used={used}）⇒ 判据空转：\
         这份源码没有踩到被接线的判定点（`elab_notation` / `args_fit_layers_in_order`）"
    );

    // ② 判据 1：两条路**文本逐字节相同**（`shadow_diff` 只统计"两条都跑"的那些）。
    assert_eq!(
        diff, 0,
        "就地路径与源码重跑分叉了 {diff} 次 —— 分叉详情见 stderr 的 \
         `JUDGE_INPLACE_MISMATCH` 行；分叉存在 ⇒ 就地路径**不许**开（`on`）"
    );

    // ③ 反向判据：**前缀真变 ⇒ 结论必须跟着变**。
    let good = compile(PREFIX_GOOD);
    let bad = compile(PREFIX_BAD);
    assert_ne!(
        good, bad,
        "前缀里的 `infix` 目标换成一个不存在的名字之后结论没变 ⇒ 判定没有跟着前缀走 ✗"
    );

    // 反向判据跑完再看一眼：**第二次编译也必须一致**（跨前缀不许复用旧结论）。
    let (_used, _fallback, _same, diff_after) = inplace_report();
    assert_eq!(diff_after, 0, "换前缀之后出现 {diff_after} 次分叉");
}

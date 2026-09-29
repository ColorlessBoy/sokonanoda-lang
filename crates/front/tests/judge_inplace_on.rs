//! **`SOKO_JUDGE_INPLACE=on` 档的判据**：前缀 / 依赖**真变** ⇒ **必须重算**
//! （值守 2026-09-29 的红线之一；**要实测，不许只写"已考虑"**）。
//!
//! **为什么放集成测试**：`inplace_report()` 是进程级计数、开关只读一次环境
//! ⇒ 与 lib 测试并行会互相干扰（同 `session_reuse.rs` 的理由）；集成测试各自
//! 独立进程 ⇒ 天然隔离 ✓。本文件只有一个测试 ⇒ 不存在抢环境变量的竞态 ✓。
//!
//! 判据（缺一不算成立）：
//! 1. **依赖内容变 ⇒ 结论必须变**（不许把上一趟的结论捞回来）；
//! 2. **而且那次结论是"就地"重新算出来的** —— `INPLACE_USED` **必须增长**
//!    （否则"结论变了"可能只是缓存整体失效后的巧合，证不到就地路径跟着前缀走）；
//! 3. 就地路径**真的被走到**过（`used > 0`），否则判据空转 ✗。
use sokonanoda_front::compile::{compile_all_units, CompileOptions, SourceUnit};
use sokonanoda_front::judge::inplace_report;
use sokonanoda_front::parse;

/// 编译 `[Dep, Entry]`（与真课程的闭包同形：依赖在前、入口在后）。
fn compile(dep_src: &str, entry_src: &str) -> String {
    let dep = parse(dep_src).expect("dep parses");
    let entry = parse(entry_src).expect("entry parses");
    let units = [
        SourceUnit::single("Dep", &dep),
        SourceUnit::single("Entry", &entry),
    ];
    let (out, _reports) = compile_all_units(&units, &CompileOptions::default());
    format!("errors={:?}\nevents={:?}", out.errors, out.events)
}

/// 入口踩的是 `args_fit_layers_in_order`（`And.intro` 有前导隐式 binder ⇒
/// 实参类型要走 `infer_type_text`）—— 那是被接线的两个判定点之一 ✓。
const ENTRY: &str = "\
import Dep
theorem uses_dep (Q : Prop) (hq : Q) : P ∧ Q := And.intro hp hq
";

/// 依赖只差**一个声明的类型**（`hp : P` → `hp : P -> P`）⇒ 前缀/闭包文本变；
/// 两份**结论必须不同**（后者 `And.intro hp hq` 类型不符 ⇒ 报错）——
/// 而**判定问的恰恰是 `hp` 的类型**（前者答 `P`、后者答 `P -> P`）
/// ⇒ "重算"是**可观测**的，不是靠计数自证 ✓。
const DEP_VALUE: &str = "\
axiom P : Prop
axiom hp : P
";
const DEP_FUNCTION: &str = "\
axiom P : Prop
axiom hp : P -> P
";

#[test]
fn changing_a_dependency_forces_a_recompute() {
    std::env::set_var("SOKO_JUDGE_INPLACE", "on");

    let before = inplace_report();
    let a = compile(DEP_VALUE, ENTRY);
    let mid = inplace_report();
    let b = compile(DEP_FUNCTION, ENTRY);
    let after = inplace_report();

    // ③ 就地路径真的被走到（否则下面两条都是空转）✓。
    assert!(
        mid.0 > before.0,
        "就地路径没被走到（used {} → {}）⇒ 判据空转：这份源码没踩到被接线的判定点",
        before.0,
        mid.0
    );

    // ① 依赖内容变 ⇒ 结论必须变。
    assert_ne!(a, b, "换了依赖之后结论没变 ⇒ 判定没有跟着前缀走 ✗");

    // ② 第二次**也是就地重算**的（计数增长 ⇒ 不是"捞旧结论"）。
    assert!(
        after.0 > mid.0,
        "换依赖后 `INPLACE_USED` 没增长（{} → {}）⇒ 那次结论不是就地重算出来的 ✗",
        mid.0,
        after.0
    );
}

//! **P1-b 第二刀（`by` 路径）的反向判据** —— 结论必须**跟着依赖走**。
//!
//! ⚠ 与影子档**分成两个文件**：`SOKO_JUDGE_INPLACE*` 走 `OnceLock` 只读一次环境，
//! 同一文件里的多个 `#[test]` **共享进程** ⇒ 抢同一个 `OnceLock` ⇒ 档位随机
//! （实测合并时约 **2/3 判红** ✗）。一个档位一个文件 = 独立进程 = 天然隔离 ✓。
//!
//! 判据：换依赖里一个声明的类型 ⇒ 结论**必须变**（证明是**重算**，不是捞旧结论）。

use sokonanoda_front::compile::{compile_all_units, CompileOptions, SourceUnit};
use sokonanoda_front::judge::inplace_by_report;
use sokonanoda_front::parse;

/// 依赖：`Set` + `mem`（两版只差 `mem` 的**参数类型**）。
const DEP_HEAD: &str = "\
axiom U : Type
axiom Set : Type -> Type
";
const MEM_OK: &str = "axiom mem : {a : Type} -> a -> Set a -> Prop\n";
/// 换成"第二个参数是 `a`、第三个是 `Set a`"⇒ 同一个目标的意义变了。
const MEM_SWAPPED: &str = "axiom mem : {a : Type} -> Set a -> a -> Prop\n";

/// 入口：**必须带 `namespace`**（否则 `canonical_goal == false`
/// ⇒ `canonical_goal_type` 根本不被调用 ⇒ 判据空转 ✗）+ 一个 `by` 块。
const ENTRY: &str = "\
import Dep
infix:50 \" ∈ \" => mem
namespace N
theorem t (A : Set U) (a : U) : a ∈ A := by
  sorry
end N
";

fn dep(mem: &str) -> String {
    format!("{DEP_HEAD}{mem}")
}

fn compile(dep: &str, entry: &str) -> String {
    let dep = parse(dep).expect("dep parses");
    let entry = parse(entry).expect("entry parses");
    let units = [
        SourceUnit::single("Dep", &dep),
        SourceUnit::single("Entry", &entry),
    ];
    let (out, _reports) = compile_all_units(&units, &CompileOptions::default());
    format!("errors={:?}\nevents={:?}", out.errors, out.events)
}

/// **反向判据**：换依赖里一个声明的类型 ⇒ 结论**必须变**（证明是重算）。
#[test]
fn by_inplace_recomputes_when_the_dependency_changes() {
    std::env::set_var("SOKO_JUDGE_INPLACE", "on");
    std::env::set_var("SOKO_JUDGE_INPLACE_BY", "1");

    let before = inplace_by_report().0;
    let ok = compile(&dep(MEM_OK), ENTRY);
    let mid = inplace_by_report().0;
    assert!(
        mid > before,
        "`by` 就地路径一次都没走到（used {} → {}）⇒ 判据空转",
        before,
        mid
    );

    let swapped = compile(&dep(MEM_SWAPPED), ENTRY);
    assert_ne!(
        ok, swapped,
        "换了依赖（`mem` 的参数类型）之后结论没变 ⇒ 判定没有跟着依赖走 ✗"
    );
}

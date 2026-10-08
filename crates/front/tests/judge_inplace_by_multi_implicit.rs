//! **A2a（2026-10-08）的判据**：**前导隐式 ≥ 2** 的目标（G-71 闸当初挡的就是这一类）
//! 也必须两条路**逐字节相同** —— 这是"抬闸"的**唯一**依据 ✓。
//!
//! ## 为什么单独一个文件
//!
//! `SOKO_JUDGE_INPLACE*` 走 `OnceLock` **只读一次环境**，而计数器是**进程级**的
//! ⇒ 一个文件一个场景（与 `judge_inplace_by.rs` / `judge_inplace_by_reverse.rs`
//! 同一条理由 ✓）：本文件的 `same` 从 0 起算 ⇒ `same > 0` 才**真的**说明
//! "多隐式那一档被走到了" ✓（挤在别的文件里会被别人的计数掩盖 ✗）。
//!
//! ## 判据（缺一不算）
//!
//! 1. **不空转**：夹具必须踩到 `mentions_multi_implicit`（否则 `needs_explicit`
//!    为假 ⇒ 量的是普通目标 ⇒ 这条判据什么都没证明 ✗）；
//! 2. **影子档**：`same > 0 && diff == 0`。
//!
//! ## 它今天（抬闸前）**必须判红**
//!
//! `by.rs` 的 G-71 闸把 `needs_explicit` 的目标压成 `ByMode::Off` ⇒ **影子分支
//! 一次都走不到** ⇒ `same == 0` ⇒ 本用例红 ✓ —— 那正是"这一档零覆盖"的实测证据
//! （先看红、再抬闸 ✓）。

use sokonanoda_front::compile::{compile_all_units, CompileOptions, SourceUnit};
use sokonanoda_front::judge::{inplace_by_report, inplace_by_shadow};
use sokonanoda_front::parse;

/// 依赖：`img` 有**两个**前导隐式（`{α β}`）⇒ `implicit_prefix == 2` ✓
/// （`mentions_multi_implicit` 的判据就是它）。
const DEP: &str = "\
axiom U : Type
axiom Set : Type -> Type
def img {α : Type} {β : Type} (f : α -> β) (A : Set α) : Set α := A
";

/// 入口：目标里出现 `img`（多隐式 def 头）+ `namespace`（`canonical_goal` 的前提）
/// + `by` 块（`canonical_goal_type` 的调用条件）✓。
const ENTRY: &str = "\
import Dep
namespace N
theorem t (A : Set U) (f : U -> U) : img f A = img f A := by
  sorry
end N
";

#[test]
fn by_inplace_shadow_agrees_on_multi_implicit_goals() {
    std::env::set_var("SOKO_JUDGE_INPLACE", "on");
    std::env::set_var("SOKO_JUDGE_INPLACE_BY", "shadow");

    let dep = parse(DEP).expect("dep parses");
    let entry = parse(ENTRY).expect("entry parses");
    let units = [
        SourceUnit::single("Dep", &dep),
        SourceUnit::single("Entry", &entry),
    ];
    let (out, _reports) = compile_all_units(&units, &CompileOptions::default());
    assert!(
        out.errors.is_empty(),
        "夹具本身必须能编（否则量的是错误路径）：{:?}",
        out.errors
    );

    let (used, fallback) = inplace_by_report();
    let (same, diff) = inplace_by_shadow();
    assert!(
        used > 0,
        "`by` 就地路径一次都没走到（used={used} fallback={fallback}）⇒ **判据空转**"
    );
    assert!(
        same > 0,
        "**多隐式那一档一次都没比对**（same={same} diff={diff} used={used}）——\n\
         ⇒ 要么夹具没踩到 `mentions_multi_implicit`（`img` 的两个前导隐式没进 DefTable ✗），\n\
         要么 G-71 闸把这一档压成了 `ByMode::Off`（抬闸前**就是**这个原因 ✓）"
    );
    assert_eq!(
        diff, 0,
        "两条路的文本**必须逐字节相同**（same={same} diff={diff}）—— 多隐式目标上\
         分叉会让 `cases`/`apply` 的臂目标回读错位 ✗"
    );
}

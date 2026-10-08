//! **T3-B1 ①（2026-10-08）的判据**：`cases` **被消去项**的就地路。
//!
//! ## 这条接线是什么
//!
//! `cases h` 要先问内核「`h` 的类型是什么」（`by.rs::cases_tactic`）。在此之前它
//! 走 `judge_infer_explicit` ⇒ **合成一份 `#check` 文档、把整份前缀从零重跑** ✗
//! （每一次未命中 = 一整趟 pass）。被消去项是一个**局部假设名** ⇒ 它的类型本来
//! 就在**活环境**里，`infer_type_text_inplace` 正是这条语义 ⇒ 不必重跑 ✓。
//!
//! ## 判据（两向，缺一不算）
//!
//! 1. **不空转**：`cases` 的就地路**必须被走到**（`same > 0`）—— 否则这条判据
//!    测的是别的东西 ✗（同 `judge_inplace_by.rs` 的纪律）；
//! 2. **影子档逐字节相同**：`diff == 0`。⚠ `on` 档 `--json` 相同**不足以**证明
//!    两条路一致（附十的教训）—— 必须比**调用方读到的结论文本** ✓。
//!
//! ## 为什么是独立文件
//!
//! `SOKO_JUDGE_INPLACE*` 走 `OnceLock` **只读一次环境**，而同一集成测试文件里的
//! 多个 `#[test]` **共享一个进程** ⇒ 两个档位互相抢（同 `judge_inplace_by.rs`）。
//! 这里只有一个档位（`shadow`）⇒ 单文件即可 ✓。

use sokonanoda_front::compile::{compile_all_units, CompileOptions, SourceUnit};
use sokonanoda_front::judge::{inplace_cases_report, inplace_cases_shadow};
use sokonanoda_front::parse;

/// 夹具：`cases h` 的被消去项类型是 `x ∈ A ∪ Set.univ` —— `∈`/`∪` 都是**多隐式**
/// 的 def（`Set.mem {α}`、`Set.union {α}`）⇒ 正是需要**全显式 pp** 才能回读的形态
/// （与 `compile/tests.rs::cases_unfolds_a_short_form_application_by_its_explicit_parameters`
/// 同一份夹具，这里只多要一条影子读数）。
const SRC: &str = "\
def Set (α : Type) : Type := α -> Prop
namespace Set
def mem {α : Type} (a : α) (A : Set α) : Prop := A a
def univ {α : Type} : Set α := fun (x : α) => True
def union {α : Type} (A B : Set α) : Set α := fun (x : α) => A x ∨ B x
end Set
infix:50 \" ∈ \" => Set.mem
infixr:70 \" ∪ \" => Set.union
theorem t (α : Type) (A : Set α) (x : α) (h : x ∈ A ∪ Set.univ) : True := by
  cases h with
    | inl ha => exact True.intro
    | inr hu => exact True.intro
";

#[test]
fn cases_inplace_shadow_agrees_on_every_query() {
    std::env::set_var("SOKO_JUDGE_INPLACE", "on");
    std::env::set_var("SOKO_JUDGE_INPLACE_BY", "shadow");

    let file = parse(SRC).expect("夹具必须解析");
    let units = [SourceUnit::single("Entry", &file)];
    let (out, _reports) = compile_all_units(&units, &CompileOptions::default());
    assert!(
        out.errors.is_empty(),
        "夹具必须编过（否则量的是错误路径 ✗）：{:?}",
        out.errors
    );

    let (_used, _fallback) = inplace_cases_report();
    let (same, diff) = inplace_cases_shadow();
    assert!(
        same > 0,
        "`cases` 的就地路一次都没比对上（same={same}）⇒ **判据空转**：\
         夹具没踩到 `cases_tactic` 的那条提问，或就地路整条没接上"
    );
    assert_eq!(
        diff, 0,
        "两条路的文本**必须逐字节相同**（same={same} diff={diff}）—— \
         分叉的入口是 `explicit` pp 档没与慢路同源（见 `judge_infer_inplace_with_explicit`）"
    );
}

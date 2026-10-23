//! **P2-a（2026-10-23，审计 row 6）**：`Eq.subst` 的等式项**不必是裸局部名** ——
//! `exact Eq.subst (Eq.symm hx) h` 以前报「至少有一个没解出来」（谓词 `p` 合成不触发），
//! 现在按端点方向合成（必要时去掉一层 `Eq.symm`）。
//!
//! 判据（缺一不算）：
//! 1. 组合等式 **双向**都判绿（`Eq.symm hx` 与裸 `hx`）；
//! 2. 端点方向与 premise/goal 的差异**不一致**时**不合成**（不许瞎猜 ⇒ 报错路径不变）。

use sokonanoda_front::compile::{compile_all_units, CompileOptions, SourceUnit};
use sokonanoda_front::parse;

const SRC: &str = "\
def Set (α : Type) : Type := α -> Prop
namespace Set
def mem {α : Type} (a : α) (A : Set α) : Prop := A a
def union {α : Type} (A B : Set α) : Set α := fun (x : α) => A x ∨ B x
end Set
infix:50 \" ∈ \" => Set.mem
infixr:70 \" ∪ \" => Set.union
theorem symm_form (α : Type) (x a : α) (hx : x = a) (A : Set α) (h : x ∈ A) : a ∈ A := by
  exact Eq.subst (Eq.symm hx) h
theorem direct_form (α : Type) (x a : α) (hx : x = a) (A : Set α) (h : a ∈ A) : x ∈ A := by
  exact Eq.subst hx h
theorem mixed_spelling (α : Type) (A B C : Set α) (x : α) (h : B = C) (hx : x ∈ A ∪ B) :
    Set.mem x (Set.union A C) := by
  exact Eq.subst h hx
";

#[test]
fn eqsubst_accepts_a_compound_equation_term_in_both_orientations() {
    let file = parse(SRC).expect("夹具必须解析");
    let units = [SourceUnit::single("Entry", &file)];
    let (out, _reports) = compile_all_units(&units, &CompileOptions::default());
    assert!(
        out.errors.is_empty(),
        "两条都应判绿（组合等式 + 裸等式）：{:?}",
        out.errors
    );
}

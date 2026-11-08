//! **G-118 / issue #25**：投影/递归子必须归约「major 是 def、def 的分量含中性
//! `Nat.add`」的情形。
//!
//! 复现（`docs/gaps/repro/G118-prod-fst-nat-add-def.sokonanoda`）：
//! `fst Nat Nat (h a b)` 里 `h` 是 def ⇒ `Pair.rec` 的 major 是 `Unfold(h)`；
//! whnf 后是 `pair_mk (Nat.add a b) (Nat.add b a)` ✓。但**判定**（conversion）
//! 里另一侧 `Nat.add a b` 是卡住的 `Unfold`（参数是变量 ⇒ nat 扩展不动）⇒
//! 旧 `conv` 的 `(_, Unfold)` 臂先命中、展不动就直接 `false` ⇒ **iota 一次都没跑** ✗。
//!
//! 判据（缺一不算）：
//! 1. **正例判绿**：`fst (h a b)` 与 `Nat.add a b` 判定相等，`gap` 被 checked；
//! 2. **反向判据（不许借机过度归约）**：`Nat.add a b` 与 `Nat.add b a` 仍**必须**判红
//!    —— 修法只是给 iota 一次机会，不是把中性 `Nat.add` 当成可交换/可展开。
//!
//! 反向验证（实测）：未改 `crates/kernel/src/conv.rs` 前，同一段 `SRC` 的 `gap`
//! 报 `kernel-rejected`（期望 `fst … = Nat.add a b`，实际 `Nat.add a b = Nat.add a b`）；
//! 改后 `gap` checked ✓ —— 本文件的第 1 条断言在旧代码上**必红**。

use sokonanoda_front::compile::{compile_all_units, CompileOptions, SourceUnit};
use sokonanoda_front::parse;

const SRC: &str = "\
inductive Pair (A B : Type) : Type
ctor pair_mk (a : A) (b : B) : Pair A B
end

def fst (A B : Type) (p : Pair A B) : A :=
  Pair.rec A B (fun (_ : Pair A B) => A) (fun (a : A) (b : B) => a) p

def g (a b : Nat) : Pair Nat Nat := pair_mk a b
theorem control (a b : Nat) : fst Nat Nat (g a b) = a := Eq.refl a

def h (a b : Nat) : Pair Nat Nat :=
  pair_mk (Nat.add a b) (Nat.add b a)
theorem gap (a b : Nat) : fst Nat Nat (h a b) = Nat.add a b :=
  Eq.refl (Nat.add a b)
";

const SWAPPED_NAT_ADD: &str = "\
theorem bad (a b : Nat) : Nat.add a b = Nat.add b a := Eq.refl (Nat.add a b)
";

fn compile(src: &str) -> sokonanoda_front::compile::CompileOutput {
    let file = parse(src).expect("夹具必须解析");
    let units = [SourceUnit::single("Entry", &file)];
    let (out, _reports) = compile_all_units(&units, &CompileOptions::default());
    out
}

#[test]
fn projection_over_a_def_with_a_neutral_nat_add_component_reduces() {
    let out = compile(SRC);
    assert!(
        out.errors.is_empty(),
        "`fst (h a b)` 应判定到 `Nat.add a b`（G-118）：{:?}",
        out.errors
    );
    let checked: Vec<&str> = out
        .events
        .iter()
        .filter_map(|e| match e {
            sokonanoda_front::compile::CheckEvent::DeclarationChecked { name } => {
                Some(name.as_str())
            }
            _ => None,
        })
        .collect();
    for want in ["fst", "g", "control", "h", "gap"] {
        assert!(
            checked.contains(&want),
            "`{want}` 必须被 checked：{checked:?}"
        );
    }
}

#[test]
fn the_iota_opportunity_does_not_equate_swapped_nat_add_arguments() {
    let out = compile(SWAPPED_NAT_ADD);
    assert!(
        !out.errors.is_empty(),
        "`Nat.add a b = Nat.add b a` 反身性不成立，必须判红（修法不许过度归约）"
    );
}

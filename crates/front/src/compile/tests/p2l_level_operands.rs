//! **P2-L-fix（REQUIREMENTS §9.39，2026-10-31）**：`=`/`≠` 的宇宙层级要**逐个操作数试**。
//!
//! `universe_level_text_of_operands` 给 `Eq.{u}`/`Ne.{u}` 定 `u` 时，第一个操作数单看推不出
//! 类型（`{∅}`、`∅`：元素类型要靠期望类型）就**整体放弃** ⇒ 默认 `u = 0` ⇒ 内核拒
//! `Sort(0)` vs `Sort(1)`。修后：推不出就**跳过、试下一个**（= Lean 4 `Elab/Extra.lean`
//! `analyze.go` 的 `isUnknown → hasUnknown` 继续扫）。
//!
//! 语料里**没有**这种形状的文件 ⇒ 不靠本模块，回退没人发现。夹具自带一份**最小 `Set`**
//! （与课程 `lib/Set` 同形的几条 `def` + `∅` 记法；`{a}` 是内建集合字面量 → `Set.singleton`）。
//!
//! 反向验证开关 [`TEST_LEVEL_ABORT_ON_UNKNOWN`]（线程局部，默认关）：开 ⇒ 修前行为。
//! 「现在要收」那组必须在开关打开时**判红**（否则本模块咬不住回退）。

use super::super::tests::on_compile_stack;
use super::super::{compile_fol_with, CheckEvent, CompileOptions, Dialect, PreludeMode};
use crate::parse;

thread_local! {
    /// `true` ⇒ 回到修前「第一个推不出的操作数就整体 `None`」（只给反向验证用）。
    pub(crate) static TEST_LEVEL_ABORT_ON_UNKNOWN: std::cell::Cell<bool> =
        const { std::cell::Cell::new(false) };
}

const LIB: &str = "\
def Set (α : Type) : Type := α → Prop
def Set.mem {α : Type} (a : α) (A : Set α) : Prop := A a
def Set.empty {α : Type} : Set α := fun (x : α) => False
def Set.univ {α : Type} : Set α := fun (x : α) => True
def Set.singleton {α : Type} (a : α) : Set α := fun (x : α) => x = a
infix:50 \" ∈ \" => Set.mem
notation \"∅\" => Set.empty
";

/// 第一个操作数单看推不出、**后面**的操作数定层级（修前被 `?` 截断的那一族）。
const NOW_ACCEPTED: &[(&str, &str)] = &[
    ("m5", "theorem m5 : {∅} = @Set.univ (Set Nat) := sorry"),
    ("lb", "theorem lb (B : Set (Set Nat)) : {∅} = B := sorry"),
    ("lc", "theorem lc : ∅ = @Set.univ Nat := sorry"),
    ("lk", "theorem lk (B : Set Nat) : ∅ = B := sorry"),
    ("le", "theorem le : {∅} ≠ @Set.univ (Set Nat) := sorry"),
];

/// 修前就收的对照（第一个操作数单看就推得出）。
const CONTROLS: &[(&str, &str)] = &[
    ("lh", "theorem lh : @Set.univ (Set Nat) = {∅} := sorry"),
    (
        "ld",
        "theorem ld (a : Set Nat) (B : Set (Set Nat)) : {a} = B := sorry",
    ),
];

/// 真错：判定不许因"跳过推不出的操作数"而放松。
const ILL_TYPED: &str = "theorem bad (n : Nat) (A : Set Nat) : n = A := sorry";

const DIALECTS: [Dialect; 2] = [Dialect::Loose, Dialect::Strict];

/// 编一份 `LIB + decl`，返回 (错误码列表, 是否发了该名的 `exercise.open`)。
fn run(tag: &str, decl: &str, dialect: Dialect, abort_on_unknown: bool) -> (Vec<String>, bool) {
    let src = format!("{LIB}\n{decl}\n");
    let tag = tag.to_string();
    on_compile_stack(move || {
        TEST_LEVEL_ABORT_ON_UNKNOWN.with(|c| c.set(abort_on_unknown));
        let file = parse(&src).unwrap_or_else(|e| panic!("parse: {e:?}\n{src}"));
        let options = CompileOptions {
            prelude: PreludeMode::Full,
            dialect,
        };
        let out = compile_fol_with(&file, &options);
        let errors = out
            .errors
            .iter()
            .map(|e| format!("{}: {}", e.kind.code(), e.message))
            .collect();
        let open = out
            .events
            .iter()
            .any(|ev| matches!(ev, CheckEvent::ExerciseOpen { name: Some(n) } if *n == tag));
        (errors, open)
    })
}

fn assert_accepted(tag: &str, decl: &str, dialect: Dialect) {
    let (errors, open) = run(tag, decl, dialect, false);
    let d = dialect.keyword();
    assert!(
        errors.is_empty(),
        "[{d}] `{decl}` 必须收（无诊断）：{errors:#?}"
    );
    assert!(open, "[{d}] `{decl}` 必须是开放练习（签名过、值位 sorry）");
}

#[test]
fn an_unknown_first_operand_no_longer_pins_the_equation_level_to_zero() {
    for dialect in DIALECTS {
        for (tag, decl) in NOW_ACCEPTED {
            assert_accepted(tag, decl, dialect);
        }
    }
}

/// 反向验证（内建）：开关回到修前行为 ⇒ 「现在要收」那组**每一条**都必须判红，
/// 且红的正是那条 `Sort(0)`/`Sort(1)` 的内核拒绝。
#[test]
fn the_old_abort_on_unknown_behaviour_rejects_them_again() {
    for dialect in DIALECTS {
        for (tag, decl) in NOW_ACCEPTED {
            let (errors, open) = run(tag, decl, dialect, true);
            let d = dialect.keyword();
            assert!(
                errors.iter().any(|e| e.starts_with("kernel-rejected")),
                "[{d}] 开关开（修前行为）⇒ `{decl}` 必须被内核拒，否则本模块咬不住回退：{errors:#?}"
            );
            assert!(!open, "[{d}] 修前行为下 `{decl}` 不许是开放练习");
        }
    }
}

#[test]
fn equations_whose_first_operand_already_inferred_stay_accepted() {
    for dialect in DIALECTS {
        for abort in [false, true] {
            for (tag, decl) in CONTROLS {
                let (errors, open) = run(tag, decl, dialect, abort);
                let d = dialect.keyword();
                assert!(
                    errors.is_empty() && open,
                    "[{d} abort={abort}] 对照 `{decl}` 必须收：{errors:#?}"
                );
            }
        }
    }
}

#[test]
fn a_genuinely_ill_typed_equation_is_still_rejected() {
    for dialect in DIALECTS {
        for abort in [false, true] {
            let (errors, open) = run("bad", ILL_TYPED, dialect, abort);
            let d = dialect.keyword();
            assert!(
                errors.iter().any(|e| e.starts_with("kernel-rejected")),
                "[{d} abort={abort}] `{ILL_TYPED}`（Nat vs Set Nat）必须仍被内核拒：{errors:#?}"
            );
            assert!(!open, "[{d} abort={abort}] 真错的签名不许成为开放练习");
        }
    }
}

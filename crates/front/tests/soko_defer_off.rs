//! **S5 第 4 步（2026-10-23）**：`SOKO_DEFER` **默认关**的判据 —— 延迟开关一个字都
//! 不许改变今天的行为（`plans/apply-unification-plan.md` §18 第 2–4 步）。
//!
//! ⚠ **一个文件一个 `#[test]`**（`scripts/check-test-env-isolation.py`：同文件里
//! `set_var` + 两个 `#[test]` = 红）。本文件**不设**环境变量 ⇒ 走默认档；
//! 反向验证（`SOKO_DEFER=1` 下本文件必须**红**）由施工单单独跑，不写进文件 ✓。
//!
//! 夹具 `/tmp/s5probe/d.sokonanoda`：第 3 条 `apply Eq.subst (Eq.symm hx) h` 的隐式
//! 实参解不出来（`Clash` 通道）⇒ 默认档必须**照今天**判死：一条诊断 @9:3、声明
//! `Failed`、读数空（不记延迟义务），**警告面也是空的**（一条 `deferred-tactic` 都不发）。

use sokonanoda_front::by_defer::{deferred_report, deferred_reset};
use sokonanoda_front::compile::{compile_all_with, CompileOptions, DeclStatus};
use sokonanoda_front::parse;

const SRC: &str = "\
def Set (α : Type) : Type := α → Prop
def mem {α : Type} (a : α) (A : Set α) : Prop := A a
def subset {α : Type} (A B : Set α) : Prop := ∀ (x : α), mem x A → mem x B
def singleton {α : Type} (a : α) : Set α := fun (x : α) => x = a

theorem t (α : Type) (A : Set α) (a b : α) (h : mem b A) : subset (singleton a) A := by
  intro x hx
  unfold singleton mem at hx
  apply Eq.subst (Eq.symm hx) h
";

#[test]
fn without_the_flag_a_deferrable_apply_still_kills_the_block() {
    deferred_reset();
    let file = parse(SRC).expect("fixture parses");
    let (out, report) = compile_all_with(&file, &CompileOptions::default());

    // ① 默认档 = 今天：一条诊断，位置就在那条 `apply` 上。
    assert_eq!(
        out.errors.len(),
        1,
        "默认档必须照今天判死：{:?}",
        out.errors
    );
    assert_eq!(out.errors[0].code(), "elab-implicit-argument-unsolved");
    assert_eq!(
        (
            out.errors[0].span.start.line,
            out.errors[0].span.start.column
        ),
        (9, 3)
    );
    // ② 声明 Failed（不是开放练习）。
    let d = report
        .decls
        .iter()
        .find(|d| d.name.as_deref() == Some("t"))
        .expect("declaration t");
    assert_eq!(d.status, DeclStatus::Failed);
    // ③ 读数必须空：开关关着 ⇒ 一条延迟义务都不许记。
    assert!(
        deferred_report().is_empty(),
        "默认档不许记延迟义务：{:?}",
        deferred_report()
    );
    // ④ **S5 第 5/6 步**：警告面也是默认关 —— 一条 `deferred-tactic` 都不许发。
    assert!(
        !out.warnings.iter().any(|w| w.code() == "deferred-tactic"),
        "默认档不许发 deferred-tactic：{:?}",
        out.warnings.iter().map(|w| w.code()).collect::<Vec<_>>()
    );
}

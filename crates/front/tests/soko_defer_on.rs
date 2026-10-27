//! **S5 第 4 步（2026-10-23）**：`SOKO_DEFER=1` 的判据 —— 一条隐式实参解不出来的
//! `apply` **不判死**：记成延迟义务、目标**留着**、块**继续**跑（`plans/apply-unification-plan.md`
//! §18 第 2–4 步）。
//!
//! ⚠ **一个文件一个 `#[test]`**（`scripts/check-test-env-isolation.py`：同文件里
//! `set_var` + 两个 `#[test]` = 红）。`SOKO_DEFER` 走 `OnceLock` **只读一次环境**，
//! 同进程里两个档位会抢同一个 `OnceLock` ⇒ 一个档位一个文件（各自独立进程 ✓）。
//!
//! 夹具 = `/tmp/s5probe/d.sokonanoda` + 末尾一条 `sorry`（**第 4 条 tactic**）：
//! 第 3 条 `apply Eq.subst (Eq.symm hx) h` 的隐式实参解不出来（`Clash` 通道）⇒
//! 默认档在这里判死（`soko_defer_off.rs` 钉着：诊断 @9:3、`Failed`），推迟档则
//! 记一条 @9:3、目标仍开着（`Open`）、**第 4 条 tactic 真的跑了**（`by_steps == 4`）。
//! **第 5/6 步**（同批）：同一处断言 `out.warnings` 里有一条 `deferred-tactic`（span @9:3）——
//! 延迟义务**穿出引擎并上警告面**，不是只在读数里。

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
  sorry
";

#[test]
fn a_deferrable_apply_is_recorded_and_the_block_continues() {
    std::env::set_var("SOKO_DEFER", "1");
    // HO4 起 pattern/approx 默认开会把这条的失败归类成 `elab-tactic-failed`（不再是
    // defer 的 `Unsolved` 通道）⇒ 本测只钉 defer，显式关两档。单文件单 `#[test]` ✓。
    std::env::set_var("SOKO_META_PATTERN", "0");
    std::env::set_var("SOKO_META_APPROX", "0");
    deferred_reset();
    let file = parse(SRC).expect("fixture parses");
    let (out, report) = compile_all_with(&file, &CompileOptions::default());

    // ① 不再判死。
    assert!(
        out.errors.is_empty(),
        "推迟档不许把这条变成硬错：{:?}",
        out.errors
    );
    // ② 目标**留着** ⇒ 声明是开放练习（默认档这里是 `Failed` + 一条诊断）。
    let d = report
        .decls
        .iter()
        .find(|d| d.name.as_deref() == Some("t"))
        .expect("declaration t");
    assert_eq!(d.status, DeclStatus::Open);
    // ③ 块真的继续跑了：4 条 tactic 都有 step（默认档在第 3 条就停）。
    assert_eq!(
        d.by_steps.len(),
        4,
        "推迟那条之后还得继续跑：{:?}",
        d.by_steps
            .iter()
            .map(|s| (s.span.start.line, s.span.start.column))
            .collect::<Vec<_>>()
    );
    assert_eq!(d.by_steps[2].goals.len(), 1, "推迟之后目标仍在");
    assert_eq!(
        (
            d.by_steps[3].span.start.line,
            d.by_steps[3].span.start.column
        ),
        (10, 3)
    );
    // ④ 记下了 (node, span, code, message)：位置就是引入它的那条 tactic。
    let pend = deferred_report();
    assert_eq!(pend.len(), 1, "{pend:?}");
    assert_eq!((pend[0].span.start.line, pend[0].span.start.column), (9, 3));
    assert_eq!(pend[0].code, "elab-implicit-argument-unsolved");
    assert!(pend[0].message.contains("隐式"), "{}", pend[0].message);
    // ⑤ **S5 第 5/6 步**：穿出引擎、上警告面 —— 一条 `deferred-tactic`，位置就是
    //    引入它的那条 tactic（不判红、不改退出码）。
    let warning = out
        .warnings
        .iter()
        .find(|w| w.code() == "deferred-tactic")
        .unwrap_or_else(|| {
            panic!(
                "推迟档要发一条 deferred-tactic warning：{:?}",
                out.warnings.iter().map(|w| w.code()).collect::<Vec<_>>()
            )
        });
    assert_eq!((warning.span.start.line, warning.span.start.column), (9, 3));
    assert!(
        warning.message.contains("第 9 行第 3 列"),
        "{}",
        warning.message
    );
}

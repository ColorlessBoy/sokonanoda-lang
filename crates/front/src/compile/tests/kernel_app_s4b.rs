//! **P2-12 S4b 判据**：记法补的前导位在内核 `Unsolved` 时**在调内核之前**物化、重试一次
//! （实施单 `reports/2026-11-01-p212-s4b-spec.md` §3 的"至少 2 条：正 + 反"）。
//!
//! 夹具 = 根因报告 `reports/2026-11-01-p2-s4b-setle-rootcause.md` §Q1 的 6 行最小文件（`min3`：
//! `fallback_old_ok_notation_unsolved = 2`，零 import、零课程依赖）与它的对照 `min9`（`∅ ≼ ∅`：
//! 两个前导位都没人定、同形兄弟一个都借不到 ⇒ 老路也拒绝，重试**必须不接**——"宁可回落，不交病项"）。
//!
//! 三条判据：**正**（内核 `Solved`、`fallback_old_ok_notation_unsolved == 0`、判卷与闸门关逐字相同）·
//! **反**（把重试开关关掉 ⇒ 回落回来 ⇒ 正的那条判红）· **不接**（借不到值 ⇒ 一次都不重试、诊断逐字不变）。
//! 读数用线程局部（`TEST_SOLVED` / `TEST_FALLBACK` / `TEST_SETTLED` / `TEST_RETRY`）：全局计数会被并行测试串扰。

use super::kernel_app::{TEST_FALLBACK, TEST_OVERRIDE, TEST_SOLVED};
use super::kernel_app_census::TEST_SETTLED;
use super::kernel_app_notation::{TEST_NO_RETRY, TEST_RETRY};
use super::tests::on_compile_stack;
use super::{compile_fol_with, CompileOptions, Dialect, PreludeMode};
use crate::parse;

/// 根因报告 §Q1 的 **min3**：`A ≼ ∅`（`β` 只能由 `∅` 的元素类型定，而它又只能由 `β` 定 ⇒ 闭环）。
const MIN3: &str = "\
def Set (α : Type) : Type := α → Prop
def Set.Le (α β : Type) (A : Set α) (B : Set β) : Prop := True
infix:50 \" ≼ \" => Set.Le
def Set.empty (α : Type) : Set α := fun (x : α) => False
notation \"∅\" => Set.empty
theorem t3 (α : Type) (A : Set α) (h : A ≼ ∅) : True := True.intro
";

/// 根因报告 §Q1 的 **min9**：`∅ ≼ ∅` —— 两个前导位都待定、没有已解出的同形兄弟
/// ⇒ `fill_pending_by_shape` 借不到 ⇒ 老路也拒绝（rc=1）。重试必须**不接**。
const MIN9: &str = "\
def Set (α : Type) : Type := α → Prop
def Set.Le (α β : Type) (A : Set α) (B : Set β) : Prop := True
infix:50 \" ≼ \" => Set.Le
def Set.empty (α : Type) : Set α := fun (x : α) => False
notation \"∅\" => Set.empty
theorem t9 (h : ∅ ≼ ∅) : True := True.intro
";

/// 一次编译的读数。
struct Out {
    errors: Vec<String>,
    /// 内核路径解出的次数（本线程）。
    solved: u64,
    /// 内核路径回落的次数（本线程）。
    fallback: u64,
    /// 老路结局 `(ok|err, why)`——`fallback_old_ok_notation_<why>` 的**同源**读数。
    settled: Vec<(&'static str, &'static str)>,
    /// `(重试次数, 重试后 Solved 次数)`。
    retries: (u64, u64),
}

impl Out {
    fn texts(&self) -> Vec<String> {
        self.errors.clone()
    }
}

/// 老路结局里"老路接住（`ok`）+ 这条原因"的条数——就是 census 的 `fallback_old_ok_<route>_<why>`。
fn old_ok(v: &[(&'static str, &'static str)], why: &str) -> usize {
    v.iter()
        .filter(|(tag, w)| *tag == "ok" && *w == why)
        .count()
}

/// 在新编译栈线程里、闸门强制开、strict 方言下编 `src`；`retry = false` ⇒ 关掉 S4b 的重试。
///
/// **P2-12 S6 翻默认**：strict 档默认不回落 ⇒ "关掉重试"这一臂要量的**正是回落**（老路接住这一族），
/// 所以显式把 `TEST_NO_FALLBACK` 设成 `false`。判据与断言一字未改。
fn run(src: &str, retry: bool) -> Out {
    let src = src.to_string();
    on_compile_stack(move || {
        TEST_OVERRIDE.with(|c| c.set(Some(true)));
        super::kernel_app_fallback::TEST_NO_FALLBACK.with(|c| c.set(Some(false)));
        TEST_NO_RETRY.with(|c| c.set(!retry));
        TEST_SOLVED.with(|c| c.set(0));
        TEST_FALLBACK.with(|c| c.set(0));
        TEST_RETRY.with(|c| c.set((0, 0)));
        TEST_SETTLED.with(|v| v.borrow_mut().clear());
        let file = parse(&src).unwrap_or_else(|e| panic!("parse: {e:?}\n{src}"));
        let options = CompileOptions {
            prelude: PreludeMode::Full,
            dialect: Dialect::Strict,
        };
        let out = compile_fol_with(&file, &options);
        let run = Out {
            errors: out.errors.iter().map(|e| format!("{e:?}")).collect(),
            solved: TEST_SOLVED.with(|c| c.get()),
            fallback: TEST_FALLBACK.with(|c| c.get()),
            settled: TEST_SETTLED.with(|v| v.borrow().clone()),
            retries: TEST_RETRY.with(|c| c.get()),
        };
        TEST_OVERRIDE.with(|c| c.set(None));
        TEST_NO_RETRY.with(|c| c.set(false));
        run
    })
}

/// **正**：`A ≼ ∅` 由内核解出（不是老路接住）——`fallback_old_ok_notation_unsolved == 0`，判卷不变。
#[test]
fn retry_solves_the_notation_prefix_family_and_removes_the_old_ok_fallback() {
    let on = run(MIN3, true);
    assert_eq!(on.texts(), Vec::<String>::new(), "判卷必须仍然通过");
    assert!(
        on.solved > 0,
        "重试后必须**由内核**解出（solved={} fallback={} retries={:?}）",
        on.solved,
        on.fallback,
        on.retries
    );
    assert_eq!(
        old_ok(&on.settled, "unsolved"),
        0,
        "`fallback_old_ok_notation_unsolved` 必须归零（老路一次都不许再被调来救这一族）：{:?}",
        on.settled
    );
    assert!(on.retries.0 > 0, "重试必须真的发生过：{:?}", on.retries);
    assert_eq!(
        on.retries,
        (on.retries.0, on.retries.0),
        "凡是重试的都该解出（否则这次重试就是白跑+多一轮回落）：{:?}",
        on.retries
    );
}

/// **反**：把重试开关关掉 ⇒ 回落回来（`fallback_old_ok_notation_unsolved ≥ 2`、`solved == 0`）
/// ⇒ 上面那条断言在开关关时**判红**。判卷仍通过（老路本来就能接住这一族）。
#[test]
fn turning_the_retry_off_restores_the_fallback_and_the_judgment_bites() {
    let off = run(MIN3, false);
    assert_eq!(off.texts(), Vec::<String>::new(), "老路接住 ⇒ 判卷不变");
    assert_eq!(off.solved, 0, "关掉重试 ⇒ 内核一次都解不出");
    assert!(
        old_ok(&off.settled, "unsolved") >= 2,
        "关掉重试 ⇒ `fallback_old_ok_notation_unsolved` 回到基线（min3 = 2）：{:?}",
        off.settled
    );
    assert_eq!(off.retries, (0, 0), "开关关 ⇒ 一次都不试");
    // 判卷逐字相同（刚才那条"正"的判据不会拿诊断的变化冒充机制的变化）。
    assert_eq!(off.texts(), run(MIN3, true).texts());
}

/// **不接**：借不到前导值（`∅ ≼ ∅`）⇒ 一次都不重试，诊断与关掉重试时**逐字相同**（不交病项）。
#[test]
fn no_prefix_value_means_no_retry_and_no_change_in_the_verdict() {
    let on = run(MIN9, true);
    let off = run(MIN9, false);
    assert_eq!(on.retries, (0, 0), "借不到值 ⇒ 重试一次都不许发生");
    assert_eq!(on.texts(), off.texts(), "诊断必须逐字不变");
    assert!(!off.errors.is_empty(), "夹具有效性：`∅ ≼ ∅` 今天就被拒绝");
}

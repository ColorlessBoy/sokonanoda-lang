//! **G1-R**（`reports/2026-11-01-capability-gaps-ridge-and-ne.md` GAP-R）：`Ne` 的隐式前导 `α`
//! 能不能由**内核按域 elaborate**解出，而不是靠 pp 回读操作数类型（老路 `notation_prefix_args`）。
//!
//! ## 结论（2026-11-01 本刀实测，两个档位同一份二进制）
//!
//! **不能** —— 目标形状的项**本质欠定**：`(fun (_ : Set Nat) => ∅) '' (…)` 里那个 `∅` 的元素类型
//! 没有任何来源 ⇒ 内核永远拿不到 `Solved`。证据链（全部零 import，见下面三个夹具）：
//!
//! | 夹具 | 欠定的是什么 | strict |
//! |---|---|---|
//! | [`FX12`]（报告 §4.2 的 `fx12` 逐字） | `∅` 的元素类型（`?γ`） | **rc=1**，1 条 `elab-implicit-argument-unsolved` |
//! | [`FX12_PINNED`]（**只**把 λ 的体写成 `(@Set.empty Nat)`） | 没有 | **rc=0 / 0 诊断**，`solved ≥ 1`、`fallback_old_ok_notation_unsolved = 0` |
//! | [`FX12_PINNED_BY_EXPECTED`]（λ 的体仍写 `∅`，但整条被 `≠ @Set.univ (Set Nat)` 钉住 β） | 没有 | **rc=0** |
//!
//! 修前/修后**同一份**读数（本文件一个字节的判定逻辑都没改内核路径）：`FX12` 在 strict 下仍是 1 条诊断，
//! 所以第二条断言的是**当前行为**（防漂移），而第三条（目标）暂时 `#[ignore]`。
//!
//! ## 卡点（探针读数，`SOKO_G1R_TRACE`，已还原）
//!
//! ```text
//! [G1R] site args=[12:6,12:36] prefix=2 explicit=?S -> Unsolved [Implicit { index: 1 }]     ← Set.image：β 没定
//! [G1R]   arg0 postponed after Err: 记法 `∅` 展开成 `Set.empty` 时补不出前面的类型参数       ← 12:27
//! [G1R] site args=[12:6,13:11] prefix=1 explicit=?? -> Unsolved [Implicit{0}, Level{u}]      ← Ne：两个操作数都被推迟
//! [G1R]   elab_ready domains=[] depth=0                                                      ← 「按域 elaborate」没有域可给
//! ```
//!
//! 即：`Ne` 的两个操作数**都**被推迟（各自的 `Set.image` 先死了）⇒ `NeedArgs{domains}` 恒空
//! （全 `blocked`）⇒ `elab_ready` 一次都拿不到域；而里面那层 `Set.image` 早就 **`α` 已定、只剩 `β`**：
//! `Unsolved [Implicit { index: 1 }]` 里下标 0（`α`）不在名单上 = 由第二个操作数 `s1 ∩ s2 : Set (Set Nat)`
//! 赋值成功，剩下的是 `β`，它只能由 λ 的**体**给出（`β = Set ?γ`）⇒ `?γ` 无来源。
//! **`α` 不是瓶颈，`?γ` 才是** —— 这也解释了为什么 `Ne` 那条诊断（点名 `α`）指错了地方。
//!
//! 可用的两条出路（都要改**非本刀**的东西）：
//! 1. **源级改法**（实测：真文件 `unit12-solution` 的类型区 7 处 λ 体写成 `(@Set.empty Nat)` ⇒ strict
//!    **rc=0 / 0 诊断**，loose 同样 rc=0）——正是 strict 自己那条诊断教的「写出点名形式」；
//! 2. **让内核路径能吃带元变量的域 / 实参**（Lean `elabAndAddNewArg` 的语义）⇒ 要动 `crates/kernel/**`。

use super::kernel_app::{TEST_FALLBACK, TEST_OVERRIDE, TEST_SOLVED};
use super::kernel_app_census::TEST_SETTLED_ROUTED;
use super::kernel_app_fallback::{
    TEST_DEAD, TEST_KEPT, TEST_NO_FALLBACK, TEST_NO_PREFIX_META_KEPT,
};
use super::kernel_app_notation::{TEST_NAIVE_MISSING, TEST_NO_NOTATION, TEST_NO_RETRY, TEST_RETRY};
use super::tests::on_compile_stack;
use super::{compile_fol_with, CompileOptions, Dialect, PreludeMode};
use crate::parse;

/// 报告 §4.2 的夹具 A（GAP-R），**逐字**（13 行）。修前/修后读数相同：strict rc=1 + 1 条
/// `elab-implicit-argument-unsolved`（`Ne` 的 `α`，位置 12:6）。
const FX12: &str = "\
def Set (α : Type) : Type := α → Prop
def Set.empty {α : Type} : Set α := fun (_ : α) => False
def Set.singleton {α : Type} (a : α) : Set α := fun (x : α) => x = a
def Set.univ {α : Type} : Set α := fun (_ : α) => True
def Set.inter {α : Type} (A B : Set α) : Set α := fun (x : α) => A x ∧ B x
def Set.image {α β : Type} (f : α → β) (A : Set α) : Set β := fun (y : β) => True
notation \"∅\" => Set.empty
infixl:70 \" ∩ \" => Set.inter
infixr:80 \" '' \" => Set.image

theorem t :
    (fun (_ : Set Nat) => ∅) '' ((@Set.singleton (Set Nat) (∅)) ∩ (@Set.singleton (Set Nat) (@Set.univ Nat)))
      ≠ ((fun (_ : Set Nat) => ∅) '' (@Set.singleton (Set Nat) (∅))) ∩ ((fun (_ : Set Nat) => ∅) '' (@Set.singleton (Set Nat) (@Set.univ Nat))) := by
  sorry
";

/// [`FX12`] **只**把两处 λ 的体写成 `(@Set.empty Nat)`（其余一字不改）⇒ `?γ` 被钉成 `Nat` ⇒ strict 过。
const FX12_PINNED: &str = "\
def Set (α : Type) : Type := α → Prop
def Set.empty {α : Type} : Set α := fun (_ : α) => False
def Set.singleton {α : Type} (a : α) : Set α := fun (x : α) => x = a
def Set.univ {α : Type} : Set α := fun (_ : α) => True
def Set.inter {α : Type} (A B : Set α) : Set α := fun (x : α) => A x ∧ B x
def Set.image {α β : Type} (f : α → β) (A : Set α) : Set β := fun (y : β) => True
notation \"∅\" => Set.empty
infixl:70 \" ∩ \" => Set.inter
infixr:80 \" '' \" => Set.image

theorem t :
    (fun (_ : Set Nat) => (@Set.empty Nat)) '' ((@Set.singleton (Set Nat) (∅)) ∩ (@Set.singleton (Set Nat) (@Set.univ Nat)))
      ≠ ((fun (_ : Set Nat) => (@Set.empty Nat)) '' (@Set.singleton (Set Nat) (∅))) ∩ ((fun (_ : Set Nat) => (@Set.empty Nat)) '' (@Set.singleton (Set Nat) (@Set.univ Nat))) := by
  sorry
";

/// 同形，但**结果类型被 `≠ @Set.univ (Set Nat)` 钉住**（真文件第一条合取项的形状）⇒ 域从另一侧到手。
const FX12_PINNED_BY_EXPECTED: &str = "\
def Set (α : Type) : Type := α → Prop
def Set.empty {α : Type} : Set α := fun (_ : α) => False
def Set.singleton {α : Type} (a : α) : Set α := fun (x : α) => x = a
def Set.univ {α : Type} : Set α := fun (_ : α) => True
def Set.inter {α : Type} (A B : Set α) : Set α := fun (x : α) => A x ∧ B x
def Set.image {α β : Type} (f : α → β) (A : Set α) : Set β := fun (y : β) => True
notation \"∅\" => Set.empty
infixl:70 \" ∩ \" => Set.inter
infixr:80 \" '' \" => Set.image

theorem t :
    (fun (_ : Set Nat) => ∅) '' ((@Set.singleton (Set Nat) (∅)) ∩ (@Set.singleton (Set Nat) (@Set.univ Nat)))
      ≠ (@Set.univ (Set Nat)) := by
  sorry
";

struct Out {
    errors: Vec<String>,
    /// 判卷错误里的 `ErrorKind`（`ElabImplicitArgumentUnsolved` 这条缺口的身份）。
    kinds: Vec<String>,
    solved: u64,
    fallback: u64,
    /// `(ok|err, <route><why>)` —— 与 census 的 `fallback_old_<tag>_<route><why>` **同源**。
    settled: Vec<(&'static str, String)>,
    retries: (u64, u64),
    /// `nofallback_kept_*` 的原因序列（开关开着仍回落的那一条）。
    kept: Vec<&'static str>,
    /// 开关开着时**报错**的次数（= `nofallback_dead`）。
    dead: u64,
}

#[derive(Clone, Copy, Default)]
struct Knobs {
    /// 关掉**内核路径**（`Effective = false` ⇒ 老路独自 elaborate；反向验证用）。
    kernel_off: bool,
}

fn old_ok(v: &[(&'static str, String)], why: &str) -> usize {
    v.iter()
        .filter(|(tag, w)| *tag == "ok" && w.as_str() == why)
        .count()
}

/// 在新编译栈线程里、闸门强制开、strict 方言下编 `src`（`#[cfg(test)]` 钩子 = 生产 strict 的默认档）。
fn run(src: &str, knobs: Knobs) -> Out {
    let src = src.to_string();
    on_compile_stack(move || {
        TEST_OVERRIDE.with(|c| c.set(Some(!knobs.kernel_off)));
        TEST_NO_NOTATION.with(|c| c.set(false));
        TEST_NAIVE_MISSING.with(|c| c.set(false));
        TEST_NO_RETRY.with(|c| c.set(false));
        // 生产 strict 的默认 = **不回落**（S6 翻默认）⇒ 这里也用 `None`（= 读档位）。
        TEST_NO_FALLBACK.with(|c| c.set(None));
        TEST_NO_PREFIX_META_KEPT.with(|c| c.set(false));
        TEST_SOLVED.with(|c| c.set(0));
        TEST_FALLBACK.with(|c| c.set(0));
        TEST_RETRY.with(|c| c.set((0, 0)));
        TEST_DEAD.with(|c| c.set(0));
        TEST_KEPT.with(|v| v.borrow_mut().clear());
        TEST_SETTLED_ROUTED.with(|v| v.borrow_mut().clear());
        let file = parse(&src).unwrap_or_else(|e| panic!("parse: {e:?}\n{src}"));
        let options = CompileOptions {
            prelude: PreludeMode::Full,
            dialect: Dialect::Strict,
        };
        let out = compile_fol_with(&file, &options);
        let done = Out {
            errors: out.errors.iter().map(|e| format!("{e:?}")).collect(),
            kinds: out.errors.iter().map(|e| format!("{:?}", e.kind)).collect(),
            solved: TEST_SOLVED.with(|c| c.get()),
            fallback: TEST_FALLBACK.with(|c| c.get()),
            settled: TEST_SETTLED_ROUTED.with(|v| v.borrow().clone()),
            retries: TEST_RETRY.with(|c| c.get()),
            kept: TEST_KEPT.with(|v| v.borrow().clone()),
            dead: TEST_DEAD.with(|c| c.get()),
        };
        TEST_OVERRIDE.with(|c| c.set(None));
        TEST_NO_NOTATION.with(|c| c.set(false));
        done
    })
}

/// **达标的那一半（活的守卫）**：λ 的体一旦有类型（`(@Set.empty Nat)`），同一个 `≠` 形状就由**内核**
/// 按域 elaborate 解出 —— `0 诊断`、`fallback_old_ok_notation_unsolved == 0`、`solved ≥ 1`。
///
/// 这条咬的是"按域 elaborate 的通道真的在跑"：把记法路由关掉（`TEST_NO_NOTATION`）它就红。
#[test]
fn the_ne_shape_is_kernel_solved_once_the_lambda_body_has_a_type() {
    let on = run(FX12_PINNED, Knobs::default());
    assert_eq!(on.errors, Vec::<String>::new(), "λ 体写全 ⇒ 必须 0 诊断");
    assert_eq!(
        old_ok(&on.settled, "notation_unsolved"),
        0,
        "`fallback_old_ok_notation_unsolved` 必须归零：{:?}",
        on.settled
    );
    assert!(
        on.solved > 0,
        "必须由**内核**解出（solved={} fallback={} settled={:?}）",
        on.solved,
        on.fallback,
        on.settled
    );
    // 反向验证：关掉**内核路径**（只留老路）⇒ 判卷仍过（老路也接得住）但内核一次都没解出
    // ⇒ 证明上面那三条断言咬的是"内核解出"，不是"判卷过了"。
    let off = run(
        FX12_PINNED,
        Knobs {
            kernel_off: true,
        },
    );
    assert_eq!(off.errors, Vec::<String>::new(), "老路也接得住（对照组）");
    assert_eq!(
        off.solved, 0,
        "关掉内核路径 ⇒ 内核解出次数必须是 0（{}）",
        off.solved
    );
}

/// **同一条通道**：λ 的体仍写 `∅`，但结果类型被整条 `≠ @Set.univ (Set Nat)` 钉住 ⇒ 域从另一侧到手
/// ⇒ 内核照样解出（真文件第一条合取项的形状）。⇒ 卡点**不是** `≠`/记法路由，而是 `?γ` 有没有来源。
#[test]
fn the_same_shape_is_solved_when_the_result_type_is_pinned_from_the_other_side() {
    let out = run(FX12_PINNED_BY_EXPECTED, Knobs::default());
    assert_eq!(out.errors, Vec::<String>::new(), "必须 0 诊断");
    assert_eq!(
        old_ok(&out.settled, "notation_unsolved"),
        0,
        "`fallback_old_ok_notation_unsolved` 必须归零：{:?}",
        out.settled
    );
}

/// **已知限制（防漂移）**：`fx12` 逐字（`∅` 的体 ⇒ `?γ` 无来源）在 strict 下**仍然**是 1 条
/// `ElabImplicitArgumentUnsolved`。这条**故意**断言"今天的现状"：
/// 一旦有人真把 `?γ` 的来源做出来（内核吃带元变量的域 / 或别的路子），它就红 ⇒ 逼着重看目标那条。
#[test]
fn the_undetermined_lambda_body_is_what_strict_cannot_infer() {
    let out = run(FX12, Knobs::default());
    assert_eq!(
        out.kinds,
        vec!["ElabImplicitArgumentUnsolved".to_string()],
        "现状 = 恰好一条隐式实参诊断：{:?}",
        out.errors
    );
    // 5 个站点报死（3 个 `Set.image` + 1 个 `Set.inter` + 1 个 `Ne`；与 CLI 的 `nofallback_dead=5` 同），
    // 但只有**最外层**那条变成用户可见的诊断（内层的 Err 在 `run_once` 里被吞成"推迟"）。
    assert_eq!(out.dead, 5, "站点数（与 CLI 逐项相同）：{}", out.dead);
    // 对照：同一份夹具只把 λ 的体写全（`FX12_PINNED`）⇒ 0 诊断 ⇒ 差别只在 `?γ` 有没有来源。
    let pinned = run(FX12_PINNED, Knobs::default());
    assert_eq!(pinned.errors, Vec::<String>::new(), "对照臂必须绿");
}

/// **目标（暂时 `#[ignore]`）**：`fx12` 逐字在 strict 下 **0 诊断**、且 `fallback_old_ok_*` 全归零
/// （= 内核解出，老路一次没接住）。
///
/// 为什么现在判红（2026-11-01 实测，见模块文档的三条读数）：`?γ` = λ 体里 `∅` 的元素类型没有任何来源，
/// 内核 `Solved` 的定义是"项里没有未赋值元变量" ⇒ 这条形状**不可能**由内核解出；`elab_ready` 在
/// 这两个站点上一次都拿不到域（`domains=[]`，全 `blocked`）。⇒ 只剩两条路：源级写全（见
/// `FX12_PINNED` + 真文件读数）或让内核吃带元变量的域/实参（`crates/kernel/**`，本刀不许碰）。
#[test]
#[ignore = "G1-R 目标：需要内核吃带元变量的域/实参（或源级写全），见模块文档"]
fn the_ne_implicit_alpha_is_solved_by_the_kernel_under_strict() {
    let out = run(FX12, Knobs::default());
    assert_eq!(
        out.errors,
        Vec::<String>::new(),
        "`fx12` 在 strict 下必须 0 诊断"
    );
    assert_eq!(out.dead, 0, "一次诊断都不许出");
    assert_eq!(
        out.kept,
        Vec::<&'static str>::new(),
        "写明的例外一次都不许用：{:?} / retries={:?}",
        out.kept,
        out.retries
    );
    assert_eq!(
        old_ok(&out.settled, "notation_unsolved"),
        0,
        "`fallback_old_ok_notation_unsolved` 必须归零：{:?}",
        out.settled
    );
}

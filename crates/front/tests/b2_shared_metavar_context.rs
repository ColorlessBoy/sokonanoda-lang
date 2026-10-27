//! **B2-P0：两次 sub-elaboration 必须共用一个 mvar 上下文**（目标夹具；P4b 后默认绿）。
//!
//! 形状（7 行）：外层头的**前导隐式只能由内层项定出来** ——
//!
//! ```text
//! def t (A : Set Nat) (B : Set (Set Nat)) : Prop :=
//!   Set.Equiv (Set.image Set.singleton A) B
//! ```
//!
//! 逐位看（`Set.Equiv {α β} (A : Set α) (B : Set β) : Prop`，课程库 `lib/Set` 的原签名）：
//!
//! * **期望类型 `Prop` 给不出 `α`、`β`** —— 与 `Set.powerset (Set.image …)` 那种「期望类型
//!   恰好能反解前导隐式」的形状不同，这里外层前缀**没有任何外部线索**；
//! * `β := Set (Set Nat)` 来自第二个实参 `B` 的类型；
//! * `α := Set Nat` **只能来自内层项** `Set.image Set.singleton A` 的类型 —— 而它自己又
//!   要一个前导隐式：`A : Set Nat` 只给 `Set.image` 的 `α`，`Set.image` 的 `β` **只能来自
//!   最内层项** `Set.singleton` 的签名（`Set.singleton {γ} : γ → Set γ` ⇒ `β := Set α`）。
//!
//! ⇒ 三层前缀求解必须**共用一个元变量表**：外层 `Set.Equiv` 的 `α` 活到内层
//! `Set.image` 的求解里（当它的期望类型 `Set ?α`），内层 `Set.singleton` 的 `{γ}` 再与
//! `Set.image` 的 `α` 合一。今天这条链断在三处（B2 的三件事，缺一条本夹具都不会绿）：
//! 1. **store 跨调用**：`MetaStore` 在每次 `solve_prefix` 里新建
//!    （`crates/front/src/compile/implicit.rs` / `elab.rs` 的 `MetaStore::default()`）
//!    ⇒ 三次求解的元变量互不相识；
//! 2. **元变量进项**：`Set.image` 那位要带着**含元变量的期望类型**（`Set ?α`）被 elaborate；
//!    今天源级 `\0soko_m{id}` **没有 `Ident` 臂**（只有 `?mN` 那条判卷回读通道），
//!    于是那个位置只能塞**签名原文里的裸 `α`**；
//! 3. **出口 zonk**：`discharge` 要求全解出（`discharge_keep_unsolved` 至今无生产调用方），
//!    也没有声明末尾 zonk ⇒ 只能整体拒。
//!
//! **今天（2026-10-25，当前源码）**：`--json` **exit 1**、**恰好一条**诊断
//! `elab-unknown-identifier` / ``unknown identifier `α` ``（span 落在**第 1 行** —— 裸名是从
//! 签名原文漏出来的，连 span 都不对）；严格档 `SOKO_METAVAR=off` 下换成
//! `elab-implicit-argument-unsolved`（**同一个**解不出前缀，两档都红）。
//!
//! **B2-P3b（2026-10-26）之后**：`SOKO_DECL_MVARS=1` 下这条的码曾**换成 `kernel-rejected`**
//! （载体经 `MetaSolve::Partial` 进项 ⇒ walk 静默跳过 ⇒ 内核阶段假 `unknown const`）。
//!
//! **B2-P4a（2026-10-26）之后**：声明出口把 store **收口**（`unify_all` + `default_unresolved`
//! + `commit`）并**重建**声明；载体绝不出门 ⇒ 那条 `kernel-rejected` **消失**，诊断回落到
//! **闸门关**（`elab-unknown-identifier` / 裸 `α`）。**仍不是全绿**：本形状缺一条约束
//! （`Set.image` 的结果类型 `Set ?β` ≟ 它自己的层域 `?α → ?β`；`Set.singleton` 的类型在
//! **闸门关**下拿不到）⇒ 收口时 `?α`/`?β` 都悬着、`default_unresolved`（E19 是**选择**）
//! 给不出 `Set Nat` ⇒ 第二趟仍留载体 ⇒ 出口**重放闸门关**。
//!
//! ⇒ 全绿需要 B2 剩下的那半（期望类型 ≟ 头结果类型的 `isDefEq`，Lean `Elab/App.lean:405`）
//! + 翻闸门 = **P4b**；**P4a 的判据是"绝不比闸门关更坏"**，那条钉在
//! `b2p3_partial_carriers.rs`（闸门开 == 闸门关）。
//! 对照组（把那个隐式写全：`@Set.singleton Nat`）**今天就是绿的** ⇒ 夹具良类型，
//! 缺的只是「隐式实参由期望类型推出 + 元变量活过多次求解」。
//!
//! ⚠ **不是 B1 的形状**：B1（期望类型沿嵌套实参传播）的三个逃生门
//! `SOKO_ARG_EXPECTED=0` / `SOKO_CONST_ARG_EXPECTED=0` / `SOKO_LEGACY_NO_EXPECTED=1`
//! 以及 `SOKO_EXPECTED_SPINE=0`、`SOKO_METAVAR=engine|sibling`、S4 的 `SOKO_DECL_METAS=1`
//! 下本夹具**逐档同判（都红）** ⇒ 没有任何既有开关能把它翻绿。
//!
//! **跑法**：`cargo test -p sokonanoda-front --test b2_shared_metavar_context`（闸门默认开；`SOKO_DECL_MVARS=0` 是旧行为对照）。

#![allow(clippy::doc_lazy_continuation)]

use sokonanoda_front::compile::{compile_all_with, CompileOptions, DeclStatus, DocumentReport};
use sokonanoda_front::parse;

/// 隐式实参**省掉**的靶子（今天红）：`Set.Equiv` 的前导 `α` 只由内层项给。
const ELIDED: &str = "\
def Set (α : Type) : Type := α -> Prop
def Set.singleton {α : Type} (a : α) : Set α := fun (x : α) => x = a
def Set.image {α β : Type} (f : α → β) (A : Set α) : Set β := fun (_ : β) => True
def Set.Equiv {α β : Type} (A : Set α) (B : Set β) : Prop := True

def t (A : Set Nat) (B : Set (Set Nat)) : Prop := Set.Equiv (Set.image Set.singleton A) B
";

/// 对照组：同一个项，只把被推的那一位写全 —— **今天必须绿**（证明夹具良类型）。
const EXPLICIT: &str = "\
def Set (α : Type) : Type := α -> Prop
def Set.singleton {α : Type} (a : α) : Set α := fun (x : α) => x = a
def Set.image {α β : Type} (f : α → β) (A : Set α) : Set β := fun (_ : β) => True
def Set.Equiv {α β : Type} (A : Set α) (B : Set β) : Prop := True

def t (A : Set Nat) (B : Set (Set Nat)) : Prop := Set.Equiv (Set.image (@Set.singleton Nat) A) B
";

fn compile(src: &str) -> (Vec<(String, String)>, DocumentReport) {
    let file = parse(src).expect("夹具必须能解析");
    let (out, report) = compile_all_with(&file, &CompileOptions::default());
    let errors = out
        .errors
        .iter()
        .map(|e| (e.code().to_string(), e.message.clone()))
        .collect();
    (errors, report)
}

fn is_checked(report: &DocumentReport, name: &str) -> bool {
    report
        .decls
        .iter()
        .any(|d| d.name.as_deref() == Some(name) && d.status == DeclStatus::Checked)
}

/// B2 之后：靶子与对照组**同判**（无诊断 + `t` checked）。
///
/// P4b 之后两步都必须通过（gate-on 默认）；`SOKO_DECL_MVARS=0` 下靶子仍红（旧行为对照）。
#[test]
fn two_sub_elaborations_share_one_metavar_context() {
    // ① 对照组：`@Set.singleton Nat` 写全 ⇒ 今天就是绿的。
    //    它同时证明 `Set.Equiv (Set.image … A) B` 的类型算术是对的（`α := Set Nat`）。
    let (control_errors, control_report) = compile(EXPLICIT);
    assert_eq!(
        control_errors,
        vec![],
        "对照组（隐式写全）今天必须无诊断 —— 夹具或类型算术变了：{control_errors:?}"
    );
    assert!(
        is_checked(&control_report, "t"),
        "对照组的 `t` 必须 `decl.checked`（否则夹具本身缺前提）"
    );

    // ② 靶子：只把 `Set.singleton` 的 `Nat` 省掉 ⇒ **B2 之后必须与对照组同判**。
    //    今天这里会失败：`elab-unknown-identifier` / ``unknown identifier `α` ``。
    let (errors, report) = compile(ELIDED);
    assert_eq!(
        errors,
        vec![],
        "B2（声明级 MetaStore + 元变量进项 + 出口 zonk）之后这里必须无诊断：\
         `Set.Equiv` 的前导 `α` 只能来自内层项 `Set.image Set.singleton A`，而后者自己的 \
         `β` 又只能来自 `Set.singleton` 的签名 —— 三层前缀求解必须共用一个元变量表；\
         今天报的是 {errors:?}"
    );
    assert!(
        is_checked(&report, "t"),
        "B2 之后 `t` 必须 `decl.checked`（与写全的对照组同判）"
    );
}

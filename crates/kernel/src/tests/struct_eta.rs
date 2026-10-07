//! **G-61 收口守卫**（2026-10-07 ✓）：单构造子 inductive 的 **η** 是**既有能力** ✓ ——
//! 谁把它删掉、或者**放宽闸**（让多构造子 / 带索引 / 递归的 inductive 也拿到 η），
//! 这里判红 ✗。
//!
//! ## 为什么这一半只能钉**结构**（行为夹具在 front 侧 ✓）
//!
//! η 的**行为**判据要跑真判卷（`crates/front/src/compile/tests.rs` 的
//! `struct_eta_*` / `multi_constructor_inductives_do_not_get_eta` /
//! `setoid_and_quotient_wrapper_is_definitional` 四条 ✓ —— 它们跑 `.sokonanoda` 源文本 ✓）。
//! 内核 crate 单测没有「源文本 ⇒ 环境」的通道 ✗（parser 在 front ✓）⇒ 这里钉**机制与闸**
//! （源码级 ✓，秒级 ✓），照 G-89 `tests/probe.rs` 的先例 ✓。
//!
//! ⚠ 断言里的**针**一律用 `concat!` 拼 ✓ —— 否则断言自己会把针写进被检查的源码
//! ⇒ `contains` 恒真/恒假、**自咬** ✗。
//!
//! ## 对齐的 Lean 4 出处（动手前读的 ✓，本机 `~/Documents/lean/lean4` master `d0493e4c1e`）
//!
//! * **结构 η**：`src/kernel/type_checker.cpp:784` `try_eta_struct_core`（`s` 是构造子应用、
//!   参数个数 = `nparams + nfields`、`is_structure_like`、逐字段比 `mk_proj`）；
//!   调用点 `:1112` `try_eta_struct(t_n, s_n)`（两向 ✓）。我们的对应物 = `conv.rs` 的
//!   `try_struct_eta` + `try_eta_struct_v`（`conv.rs:596` / `:684` ✓）。
//! * **主前提 η 展开**（iota 那一半 ✓）：`src/kernel/inductive.h:66` `to_cnstr_when_structure`
//!   —— 主前提**不是**构造子应用、而其类型是 structure-like 时，先展成 `C.mk e.1 … e.n`
//!   再套 iota 规则 ✓（`src/kernel/inductive.cpp:98` `expand_eta_struct` ✓）。我们的对应物 =
//!   `eval.rs::try_struct_eta_reduce`（`fire_recursor` 里 ✓）。
//! * **单位元**：`type_checker.cpp:1035` `is_def_eq_unit_like`（单构造子 + 零字段 ⇒ 全等 ✓）
//!   ↔ 我们的 `conv.rs::is_unit_inductive` ✓。
//! * **闸**：`src/kernel/inductive.cpp:27` `is_structure_like` = **1 构造子 + 0 索引 + 非递归** ✓
//!   ↔ 我们的 `env.rs::get_structure`（同三条 ✓）。
//!
//! ⇒ **多构造子 / 带索引 / 递归的 inductive 在 Lean 里拿不到 η** ✓ —— 我们也一样 ✓；
//! 闸一旦放宽就是**判定变宽**（false → true 之外的**错误**方向）⇒ 必须判红 ✗。

/// ① **四条 η 机制都在** ✓（删掉任何一条 ⇒ η 回退 ✗）。
#[test]
fn struct_eta_mechanisms_are_present() {
    let conv = include_str!("../conv.rs");
    let eval = include_str!("../eval.rs");
    for needle in [
        concat!("fn try", "_struct_eta("),
        concat!("fn try_eta", "_struct_v("),
        concat!("fn is_", "unit_inductive("),
    ] {
        assert!(conv.contains(needle), "**G-61**：`conv.rs` 里 `{needle}` 不见了 ✗（η 回退）");
    }
    assert!(
        eval.contains(concat!("fn try_struct", "_eta_reduce(")),
        "**G-61**：`eval.rs` 里主前提的 η 展开不见了 ✗（`Box.id b = b` 那一条会回退）"
    );
}

/// ② **闸 = Lean 的 `is_structure_like`** ✓：**1 构造子 · 0 索引 · 非递归** ——
/// 三条缺一不可（放宽 = 多构造子 / 带索引 / 递归也拿 η ⇒ 判定变宽 ✗）。
#[test]
fn struct_eta_gate_matches_lean_structure_like() {
    let env = include_str!("../env.rs");
    for needle in [
        concat!("all_ctor_names.len()", " == 1"),
        concat!("*num_indices", " == 0"),
        concat!("rec_ok || ", "!is_recursive"),
    ] {
        assert!(
            env.contains(needle),
            "**G-61**：`env.rs` 的结构闸条件 `{needle}` 不见了 ✗ —— \
             Lean 的 `is_structure_like`（`src/kernel/inductive.cpp:27`）是\
             **1 构造子 + 0 索引 + 非递归** ✓，放宽就是判定变宽 ✗"
        );
    }
    // 两个 η 点都必须**走这道闸** ✓（少一处 ⇒ 那一处会对多构造子也生效 ✗）。
    let conv = include_str!("../conv.rs");
    let eval = include_str!("../eval.rs");
    assert!(
        conv.contains(concat!("can_be_struct", "_memo(ind_name)")),
        "**G-61**：`conv.rs` 的结构 η 不再过闸 ✗（多构造子会拿到 η ✗）"
    );
    assert!(
        eval.contains(concat!("can_be_struct", "_memo(rec_induct)")),
        "**G-61**：`eval.rs` 的主前提 η 展开不再过闸 ✗"
    );
}

/// ③ **iota 先于 η** ✓（`fire_recursor` 的调用顺序）：主前提**真是构造子**时由
/// `major_to_ctor` 直接命中规则 ✓ —— η 只是「主前提不是构造子、但类型是结构」时的**补路** ✓。
/// 顺序一反过来，iota 的既有行为就可能被 η 影响 ⇒ **判定中性**的论证就断了 ✗。
#[test]
fn iota_fires_before_eta_in_fire_recursor() {
    let eval = include_str!("../eval.rs");
    let ctor = eval
        .find(concat!(".major_to_ctor(depth, ", "major)"))
        .expect("**G-61**：`fire_recursor` 里 `major_to_ctor` 不见了 ✗");
    let eta = eval
        .find(concat!(".or_else(|| self.try_struct_eta", "_reduce(depth, major, rec))"))
        .expect("**G-61**：`fire_recursor` 里 η 补路不见了 ✗");
    assert!(
        ctor < eta,
        "**G-61**：`fire_recursor` 里 η 补路跑到了构造子快路**前面** ✗ —— \
         真构造子的 iota 不许经过 η（`Box.get (Box.mk a) = a` 是逐字照旧的红线 ✓）"
    );
}

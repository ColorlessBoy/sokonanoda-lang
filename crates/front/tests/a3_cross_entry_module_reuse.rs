//! **A3-刀 2 的判据读数**（2026-10-08）：**跨入口**打开时，共享的库层模块被编了几次。
//!
//! ## 这道门已经过了（§8.11），本文件是它"若做"那半边的**读数前置**
//!
//! 规划 §`A3` 的决策门已由 §8.11 量完 ⇒ **值得做** ✓（"跨入口的库层重复仍是主要
//! 成本"）。判据（若做）里点名要"`module_keys` **命中计数**（新增）" ✓ ——
//! 本文件先把**今天的行为**钉成可判据的读数 ✓，A3 落地时按文件头写的方式翻转 ✓。
//!
//! ## 病灶（读码 + 规划核实）
//!
//! 磁盘上是 `<根>/.sokonanoda/compiled/<**整闭包** digest>.json`（一入口一条 ✓），
//! 而库层检查点（G-29）的键是**整条闭包摘要** ⇒ **换一个入口（闭包不同）就整条库层
//! 重编** ✗ —— 哪怕其中**大部分模块的源文本一个字都没变** ✓。
//! 内核没有 `ExportFile → EnvBuilder` 的入口 ⇒ 刀 2 是**新机制**，不是接线 ✓。
//!
//! ## 夹具（三个模块 + 两条入口，共享一个 `Common`）
//!
//! ```text
//! Common ── LibA ── MainA
//!        └─ LibB ── MainB
//! ```
//!
//! ⇒ 两条入口的闭包都是 3 个模块（`Common` 共享 ✓、`LibX`/`MainX` 各自独有 ✓）。
//!
//! ## 读数与出口
//!
//! * **今天**：先开 `MainA` ⇒ 编 **3**；再开 `MainB`（同进程、同模块根）⇒ **又编 3**
//!   （`Common` 白编一遍 ✗）。
//! * **A3 落地后**：第二条入口应当只编 **2**（`LibB` + `MainB`；`Common` 从**模块级
//!   产物**复用 ✓）⇒ **那时把下面的断言从 3 改成 2** ✓ —— 那是本文件唯一的正确出口，
//!   **不许**用放宽它的办法变绿 ✗（同 `g29_closure_recompile.rs` 的纪律 ✓）。
//!
//! ## 为什么是独立测试文件
//!
//! `closure_module_compiles_total()` 是**进程级**计数器 ⇒ 独立进程 = 天然隔离 ✓。

use std::path::PathBuf;

use sokonanoda_front::compile::closure_module_compiles_total;
use sokonanoda_front::project::session::lib_checkpoint_reset;
use sokonanoda_front::query::QueryDoc;

/// 合成夹具：`Common` 被两条入口链共享（`Common ← LibX ← MainX`）。
fn gen_project(tag: &str) -> (PathBuf, PathBuf) {
    let dir = std::env::temp_dir().join(format!("soko-a3-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");

    std::fs::write(dir.join("Common.sokonanoda"), "axiom P : Prop\n").expect("Common");
    std::fs::write(
        dir.join("LibA.sokonanoda"),
        "import Common\n\naxiom a : P\n",
    )
    .expect("LibA");
    std::fs::write(
        dir.join("LibB.sokonanoda"),
        "import Common\n\naxiom b : P\n",
    )
    .expect("LibB");
    std::fs::write(
        dir.join("MainA.sokonanoda"),
        "import LibA\n\ntheorem ta : P := a\n",
    )
    .expect("MainA");
    let main_b = dir.join("MainB.sokonanoda");
    std::fs::write(&main_b, "import LibB\n\ntheorem tb : P := b\n").expect("MainB");
    (dir, main_b)
}

/// 打开一条入口，返回**这一刀编了几个模块**（进程级计数器的增量 ✓）。
fn modules_for(doc: &mut QueryDoc, entry: &PathBuf, version: u64) -> u64 {
    let text = std::fs::read_to_string(entry).expect("read entry");
    let before = closure_module_compiles_total();
    doc.path = Some(entry.clone());
    doc.set_text(&text, version, None);
    closure_module_compiles_total() - before
}

#[test]
fn a_second_entry_with_a_shared_library_module_recompiles_the_whole_closure() {
    lib_checkpoint_reset();
    let (dir, main_b) = gen_project("cross-entry");
    let main_a = dir.join("MainA.sokonanoda");

    let mut doc = QueryDoc::new();
    // 冷开第一条入口（= LSP 的 `didOpen`）：整条闭包 3 个模块（`Common`/`LibA`/`MainA`）。
    let cold_a = modules_for(&mut doc, &main_a, 1);
    assert_eq!(
        cold_a, 3,
        "夹具自检：`MainA` 的闭包 = `Common` + `LibA` + `MainA` = **3** ✓（实测 {cold_a}）\
         —— 不等于 3 ⇒ 夹具或计数口径变了，读数不可用 ✗"
    );
    assert!(
        doc.report.is_some(),
        "冷开必须产出报告（否则量的是错误路径 ✗）"
    );

    // **夹具自检**（这条读数真的分得清「复用/没复用」吗？）：**紧接着**再开一次同一条入口
    // ⇒ 闭包完全相同 ⇒ G-29 的库层检查点命中 ⇒ **只编入口那 1 个模块** ✓。
    // 没有这一臂，"换入口 = 3"可能只是"计数器恒为闭包大小" ✗。
    let again = modules_for(&mut doc, &main_a, 2);
    assert_eq!(
        again, 1,
        "**自检**：同一入口紧接着再开一次 ⇒ 库层检查点命中 ⇒ **只该编 1 个模块**（实测 {again}）\
         —— 若这里也是 3 ⇒ 计数器分不清「复用/没复用」⇒ 下面那条读数**没有判别力** ✗"
    );

    // **被量的那一刀**：同进程、同模块根，换一条入口（闭包不同但**共享 `Common`**）。
    //
    // ⚠ **顺带量到的第二条事实**（本轮实测，值得记下来 ✓）：换入口**不只是**重编整条
    // 闭包 —— 它还把**上一条入口的库层检查点挤掉**了（本用例末尾那臂：换过 `MainB`
    // 之后再回头开 `MainA`，**又是 3** ✗，不是 1）。⇒ A3-刀 2 要解决的**不止**"共享模块
    // 重编"，还有"**检查点只有一份、换闭包即失效**"✓（§8.11 的 `5 → 8 → 8` 同源 ✓）。
    let second = modules_for(&mut doc, &main_b, 2);
    assert_eq!(
        second, 3,
        "**A3-刀 2 的起点读数**：第二条入口**又编了整条闭包 3 个模块**（实测 {second}）\
         —— 其中 `Common` 的源文本**一个字都没变** ✗。\n\
         A3 落地后这里应当是 **2**（只编 `LibB` + `MainB`；`Common` 从模块级产物复用 ✓）\
         ⇒ **那时把本断言改成 2** ✓（唯一正确出口，不许放宽 ✗）。"
    );

    // **第二条事实**（见上面的 ⚠）：换过入口之后**回头**再开 `MainA` ⇒ 又是 **3**
    // （库层检查点被 `MainB` 那一刀挤掉了 ✗）⇒ A3 落地后这里应当是 **2**（只编 `MainA`；
    // `Common`/`LibA` 从模块级产物复用 ✓）—— **同一条出口**：改成 2 ✓。
    let back = modules_for(&mut doc, &main_a, 3);
    assert_eq!(
        back, 3,
        "**A3-刀 2 的起点读数（第二条）**：换过闭包之后回头开原入口 ⇒ **又是整条闭包 3** \
         （实测 {back}）✗ —— 检查点只有一份、换闭包即失效。A3 落地后这里应当是 **2** ✓。"
    );
}

//! **多槽 LRU 检查点的编辑器层判据**（2026-10-08）：**在两条入口之间来回切**。
//!
//! ## 为什么要单独一条（front 层已经有判据了）
//!
//! 库层检查点是**线程局部**的（内核环境 `!Send` ⇒ 见 `project/session.rs` 文件头）。
//! ⇒ front 层的读数（`a3_cross_entry_module_reuse`）**证明不了**编辑器里也省下来 ✗：
//! 若 LSP 把编译放到**另一条线程**（或每条请求一条 ✗），检查点永远不会命中，
//! 用户侧的"来回切"照样白付整条库层趟 ✗。
//! ⇒ 这一条**在真 LSP 进程里**量同一个场景（AGENTS.md 的"验收必须断言用户可见的结果" ✓）。
//!
//! ## 场景与读数
//!
//! ```text
//! Shared ── A ── EntryA.sokonanoda     （闭包 3）
//!        └─ B ── EntryB.sokonanoda     （闭包 3，与上一条**不同**）
//! ```
//!
//! 1. 开 `EntryA` ⇒ 编整条闭包（**3**）；
//! 2. 改 `EntryA` ⇒ 检查点命中 ⇒ 只编入口（**1**）；
//! 3. 开 `EntryB` ⇒ 它的闭包是新的 ⇒ 编 **3**（这一步会不会把 `EntryA` 那份**挤掉**？——
//!    这正是本用例量的东西 ✓）；
//! 4. **再改 `EntryA`** ⇒ 多槽 LRU 命中原来那份 ⇒ **1** ✓；单槽会读到 **3** ✗。
//!
//! **反向验证**：把 `LIB_CHECKPOINTS` 退回单份（`Option`）⇒ 第 4 步读到 3 ⇒ 本用例判红 ✓。
//!
//! ## 两个夹具坑（都实测踩过，别再踩 ✗）
//!
//! * **模块名要能解析**：库文件放**根**目录时 import 写 `import A` ✓；若把文件放
//!   `lib/A.sokonanoda` 却写 `import A` ✗ ⇒ 闭包解析不到 ⇒ 编译早早失败、`modules=0`
//!   （实测假读数 ✗）。
//! * **关掉 A5 的开档预热**（`SOKO_NO_LIB_WARMUP=1` ✓）：预热会**另外**打一条
//!   `LSP_TRACE warm-library …` 行，而 `last_trace()` 读的是**最后一条** ⇒ 与编译那条
//!   **抢**（实测读到预热行 ⇒ `modules=0` 假读数 ✗）。预热是另一件事，它自己的判据在
//!   `lsp_artifact_warmup.rs` ✓。

mod common;

use common::Client;
use std::path::PathBuf;

const SHARED: &str = "axiom P : Prop\n";
const LIB_A: &str = "import Shared\n\naxiom pa : P\n";
const LIB_B: &str = "import Shared\n\naxiom pb : P\n";
const ENTRY_A: &str = "import A\n\ntheorem ta : P := pa\n";
const ENTRY_B: &str = "import B\n\ntheorem tb : P := pb\n";
/// 改一行（只加一条注释 ⇒ 只该重编入口 ✓）。
const ENTRY_A_EDITED: &str = "import A\n\ntheorem ta : P := pa\n-- 注释\n";
/// **第四次那一刀必须是一份新文本** ⚠：把文档改回**已经编过的**内容（`ENTRY_A`）会走
/// **产物/缓存命中**那条路（实测 `modules=0`、0ms ✗）—— 那量的是缓存，不是检查点 ✗。
const ENTRY_A_EDITED_2: &str = "import A\n\ntheorem ta : P := pa\n-- 注释2\n";

fn fixture(tag: &str) -> (PathBuf, PathBuf, PathBuf) {
    let root = std::env::temp_dir().join(format!(
        "sokonanoda-lsp-multislot-{tag}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("mkdir root");
    std::fs::write(root.join("Shared.sokonanoda"), SHARED).expect("Shared");
    std::fs::write(root.join("A.sokonanoda"), LIB_A).expect("A");
    std::fs::write(root.join("B.sokonanoda"), LIB_B).expect("B");
    let entry_a = root.join("EntryA.sokonanoda");
    std::fs::write(&entry_a, ENTRY_A).expect("EntryA");
    let entry_b = root.join("EntryB.sokonanoda");
    std::fs::write(&entry_b, ENTRY_B).expect("EntryB");
    (root, entry_a, entry_b)
}

fn cache_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sokonanoda-lsp-multislot-cache-{tag}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

/// 等下一次编译的 trace 行，返回它的 `modules=` 字段（附 trace 原文 ✓）。
fn modules_after(client: &mut Client, action: impl FnOnce(&mut Client)) -> (u64, String) {
    // 基线取**落定值** ✓（`trace_len()` 直接读会漏掉"行已写、读线程还没收"的那一趟 ✗
    // —— 2026-10-08 CI 实测同形的判红，见 `Client::settled_compile_count` ✓）。
    let before = client.settled_compile_count();
    action(client);
    let _ = client.wait_for_trace_after(before);
    let line = client.last_trace();
    (Client::trace_field(&line, "modules"), line)
}

#[test]
fn switching_between_two_entries_keeps_both_library_checkpoints() {
    let (root, entry_a, entry_b) = fixture("switch");
    let cache = cache_dir("switch");
    let mut client = Client::start_traced_with_env(&cache, &[("SOKO_NO_LIB_WARMUP", "1")]);
    let uri_a = Client::file_uri(&entry_a);
    let uri_b = Client::file_uri(&entry_b);

    // ① 开 A（冷）：整条闭包 3 个模块（`Shared` + `A` + 入口）。
    let (cold_a, line_a) = modules_after(&mut client, |c| {
        let _ = c.open(&root, &uri_a, ENTRY_A);
    });
    assert_eq!(
        cold_a, 3,
        "夹具自检：`A` 的闭包 = `Shared` + `A` + 入口 = **3**（实测 {cold_a}）—— \
         不等于 3 ⇒ 夹具或计数口径变了 ✗\n  {line_a}"
    );

    // ② 改 A：检查点命中 ⇒ 只编入口 1 个。
    let (warm_a, line_warm) = modules_after(&mut client, |c| {
        let _ = c.did_change(&uri_a, 2, ENTRY_A_EDITED);
    });
    assert_eq!(
        warm_a, 1,
        "**敏感性自检**：改同一份入口 ⇒ 检查点命中 ⇒ 只该编 **1** 个模块（实测 {warm_a}）\
         —— 若这里也是 3 ⇒ 检查点根本没命中（多槽的收益就量不出来 ✗）\n  {line_warm}"
    );

    // ③ 开 B（闭包不同：`Shared` + `B` + 入口）⇒ 整条 3 个；**这一步会挤掉 A 那份吗？**
    let (cold_b, line_b) = modules_after(&mut client, |c| {
        let _ = c.open(&root, &uri_b, ENTRY_B);
    });
    assert_eq!(
        cold_b, 3,
        "夹具自检：`B` 的闭包 = `Shared` + `B` + 入口 = **3**（实测 {cold_b}）\n  {line_b}"
    );

    // ④ **被量的那一刀**：回头再改 A ⇒ 多槽 LRU 必须命中**原来那份** ⇒ 只编入口 1 个 ✓。
    let (back, line_back) = modules_after(&mut client, |c| {
        let _ = c.did_change(&uri_a, 3, ENTRY_A_EDITED_2);
    });
    assert_eq!(
        back, 1,
        "**多槽 LRU 的编辑器层判据**：在两条入口之间来回切之后，回头那一刀必须**命中**\
         原来那份库层检查点 ⇒ 只编 **1** 个模块（实测 {back}）—— 若读到 **3** ⇒ 检查点退回\
         **单份**（换闭包即失效 ✗）：学生在几个单元之间来回切时，每次回头都白付一趟库层\
         （unit08 量级 ≈ 700ms ✗）。这条同时证明**检查点真的在 LSP 那条编译线程上** ✓\
         （它是线程局部的：换线程 ⇒ 这里必然读到 3 ✗）\n  {line_back}"
    );
}

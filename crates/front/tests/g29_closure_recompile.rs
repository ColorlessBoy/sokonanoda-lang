//! **G-29 的防漂移判据**（**断言当前行为** ✓ · 2026-10-07）—— 「改一行 ⇒ 整条闭包重编」。
//!
//! **今天**：改一行 ⇒ `closure_modules` = 库层 + 入口 = **2**，与**冷开相等**。
//! 这就是台账 G-29 那句话（「依赖的源文本没变，但它们的**内核环境**在编译结束后就没了
//! ⇒ 要接着编入口就必须重新 elaborate 它们」）的**精确结构读数** ✓。
//!
//! **修好之后**（设计 `docs/design/incremental-environment.md` §33：把库层检查点
//! **跨调用**留着）它应当变成 **1**（只重编入口）⇒ **那时把下面的断言改成 1** ——
//! 那是本条唯一的正确出口，**不许**用放宽它的办法变绿 ✗。
//!
//! ## 为什么必须常驻
//!
//! 没有它，"改一行到底重编了几个**闭包模块**"这件事**没有任何判据**：
//! 复现件 `docs/gaps/repro/G29-edit-recompiles-whole-closure.js` 读的 `modules=`
//! 数的是 `MODULE_COMPILES`，而它只在 `check::run` 里按 `units.len()` 累加 ⇒
//! **会话那条路**（`project/session.rs` → `run_pass_with`）一次都不计 ✗，judge 的
//! **合成编译**（`compile_fol_with`，`units=1 names=[""]`）**照计** ✓ ⇒
//! 它在"改一行"那一刀上读到的是**合成编译次数**（实测 unit08 = 7 次，backtrace
//! `compile_fol_with ← judge_infer_uncached ← … ← build_def`），**不是**闭包模块数
//! （真值 5，一次都没被计）✗。⇒ 那句「改一行 `modules=7` = 整条闭包重编」是**误归因**；
//! 本文件用 `closure_module_compiles_total()`（只数 `path: Some(..)` 的真模块、
//! 老路与会话路**同口径**）把它量对。
//!
//! ## 为什么是**独立测试文件**
//!
//! `closure_module_compiles_total()` 是**进程级**计数器 ⇒ 同一进程里并行的用例会互相
//! 污染取差（实测：与 `keystroke_structure.rs` 同进程跑时本判据假红 ✗）。独立文件 =
//! 独立进程 ⇒ 天然隔离（同 `session_shared.rs` / `session_reuse.rs` 的纪律）。
//!
//! ## 为什么用**结构计数**而不是墙钟
//!
//! `AGENTS.md` 判据纪律②：该复现件的墙钟判据翻过两次面 —— 本机**同一份构建**实测
//! 冷开 **1386ms ↔ 4079ms**（2.9×），而 `modules`/`by`/`prefix` 三个结构计数
//! **一字不变**。墙钟只兜数量级。

use std::path::PathBuf;

use sokonanoda_front::compile::closure_module_compiles_total;
use sokonanoda_front::project::session::{
    lib_checkpoint_arenas_leaked, lib_checkpoint_is_live, lib_checkpoint_reset,
    lib_checkpoint_reuses,
};
use sokonanoda_front::query::QueryDoc;

/// 合成闭包夹具：`Lib.sokonanoda`（一条自定义记法 + 两条公理）← `Main.sokonanoda`。
///
/// 与 `keystroke_structure.rs::gen_project` 同纪律（**合成**、不依赖课程目录）；
/// 这里只要**两个模块**，够读出"库层有没有被重编"。
fn gen_project(tag: &str) -> (PathBuf, String) {
    let dir = std::env::temp_dir().join(format!("soko-g29-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");

    let lib = "axiom Point : Type\n\
               axiom EqP : Point -> Point -> Prop\n\
               infix:50 \" \u{2248} \" => EqP\n\
               axiom refl : (a : Point) -> a \u{2248} a\n";
    std::fs::write(dir.join("Lib.sokonanoda"), lib).expect("write Lib");

    let main = "import Lib\n\n\
                theorem t00 (a b : Point) (h : a \u{2248} b) : a \u{2248} b := by exact h\n\n\
                theorem t01 (a b : Point) (h : a \u{2248} b) : a \u{2248} b := by exact t00 a b h\n";
    let entry = dir.join("Main.sokonanoda");
    std::fs::write(&entry, main).expect("write Main");
    (entry, main.to_string())
}

/// 一次**真实且长度不变**的按键（改 `t00` 的局部 binder 名 `h` → `k`）。
///
/// ⚠ **必须长度不变**：改了字节数 ⇒ 后面每条命令的起点平移 ⇒ 缓存里的 span 不能再当
/// 新的用 ⇒ 它们**必须**进脏集，量到的就不是"这一条改动"✗（同 `keystroke_structure.rs`）。
fn edit_decl(text: &str) -> String {
    let needle = "theorem t00 ";
    let at = text.find(needle).expect("夹具里必须有 `t00`");
    let end = text[at..].find('\n').map_or(text.len(), |n| at + n);
    let line = &text[at..end];
    let edited = line
        .replacen("(h :", "(k :", 1)
        .replacen("exact h", "exact k", 1);
    assert_eq!(
        edited.len(),
        line.len(),
        "这次按键必须**长度不变**（否则下游命令起点平移 ⇒ 量到的不是这一次改动）"
    );
    format!("{}{}{}", &text[..at], edited, &text[end..])
}

/// **判据**：改一行只重编**入口**那一个模块（**1**）—— 库层检查点跨调用复用（设计 §33）。
#[test]
fn g29_edit_recompiles_the_whole_closure() {
    // 判据自己起跑：线程局部（检查点）与进程级（泄漏计数）都要干净。
    lib_checkpoint_reset();
    let (entry, text) = gen_project("drift");
    let mut doc = QueryDoc::new();
    doc.path = Some(entry.clone());

    // 冷开（= LSP 的 `didOpen`）：整条闭包。
    let base = closure_module_compiles_total();
    doc.set_text(&text, 1, None);
    let cold = closure_module_compiles_total() - base;

    let base = closure_module_compiles_total();
    doc.set_text(&edit_decl(&text), 2, None);
    let edit = closure_module_compiles_total() - base;

    // 夹具自检：`Lib` + `Main` = 2 个模块（冷开必须编整条闭包）。
    assert_eq!(
        cold, 2,
        "冷开必须编**整条闭包**（`Lib` + `Main` = 2 个模块），实测 {cold} ⇒ 夹具或计数口径变了"
    );
    // 冷开必须**留下**库层检查点 —— 否则改一行没有可复用的东西（回退路 ⇒ 又编 2 个）。
    assert!(
        lib_checkpoint_is_live(),
        "冷开之后必须留下库层检查点（设计 §33 的跨调用复用就靠它）"
    );
    // 上界 ①：冷开只许泄漏**一份**库层 arena。
    assert_eq!(
        lib_checkpoint_arenas_leaked(),
        1,
        "库层检查点只许泄漏一份 arena（`MAX_LEAKED_LIB_ARENAS` 之内）"
    );
    // 正确性不变量：改 `t00` 之后，没改过的 `t01` 不许掉出 `checked`。
    let report = doc.report.as_ref().expect("改完必须有报告");
    let t01 = report
        .decls
        .iter()
        .find(|d| d.name.as_deref() == Some("t01"))
        .expect("`t01` 必须在报告里");
    assert_eq!(
        format!("{:?}", t01.status),
        "Checked",
        "改 `t00` ⇒ 没改过的 `t01` 不许掉出 checked"
    );

    assert_eq!(
        edit, 1,
        "**G-29 仍在**：改一行重编了 {edit} 个闭包模块（冷开 {cold}）⇒ 依赖的源文本一个\
         字节没变也照编 ✗。修好（库层检查点**跨调用**复用，设计 §33）之后这里是 **1**\
         （只重编入口）—— 这条断言已经改成 1，**不许**再放宽 ✗。"
    );
    // 复用**不许**再泄漏 arena（上界 ①：反复改 N 次后仍只有 1 份）。
    assert_eq!(
        lib_checkpoint_arenas_leaked(),
        1,
        "复用路不许泄漏新 arena（上界 ①）"
    );

    // **上界判据**（下面那个函数）：`closure_module_compiles_total()` 与泄漏计数都是
    // **进程级**的 ⇒ 本文件只能有**一个** `#[test]`（同进程并行会互相污染取差 ✗，
    // 见文件头的纪律）⇒ 上界判据作为**同一个测试**的第二段跑。
    g29_library_checkpoint_is_bounded();
}

/// **上界判据**（设计 §33 的"必须有上界"）——检查点**不会无界增长**：
/// ① 反复改**入口** N 次 ⇒ 泄漏的 arena 恒 **1** 份、活着的检查点恒 **1** 份；
/// ② 改**库层** ⇒ 摘要变 ⇒ 旧检查点被换掉（**语义上不可达**）+ 新的一份 arena；
/// ③ 泄漏 arena 数**封顶** `MAX_LEAKED_LIB_ARENAS`，到顶之后走回退路
///    （栈上 arena = 今天那条路）且**结果照旧正确** ✓。
fn g29_library_checkpoint_is_bounded() {
    lib_checkpoint_reset();
    let (entry, text) = gen_project("bound");
    let mut doc = QueryDoc::new();
    doc.path = Some(entry.clone());
    doc.set_text(&text, 1, None);
    assert_eq!(lib_checkpoint_arenas_leaked(), 1, "冷开建一份检查点");

    // ① 反复改入口（长度不变的按键 ⇒ 只动那一条声明）：检查点**复用**，不再泄漏。
    let mut current = text.clone();
    for version in 2..=20u64 {
        current = edit_decl(&current);
        doc.set_text(&current, version, None);
    }
    assert_eq!(
        lib_checkpoint_arenas_leaked(),
        1,
        "改入口 N 次 ⇒ 泄漏仍只有 1 份（复用路不建新检查点）"
    );
    assert!(lib_checkpoint_is_live(), "检查点仍活着");
    assert_eq!(
        lib_checkpoint_reuses(),
        19,
        "20 次 `set_text` = 冷开建 + 19 次复用"
    );
    let report = doc.report.as_ref().expect("必须有报告");
    assert!(
        report
            .decls
            .iter()
            .any(|d| d.name.as_deref() == Some("t01") && format!("{:?}", d.status) == "Checked"),
        "反复改入口之后 `t01` 仍必须 `checked`（复用不许改变判定）"
    );

    // ② 改**库层**：摘要变 ⇒ 检查点被换掉（旧的语义上不可达）+ 泄漏一份新 arena。
    let lib_path = entry.with_file_name("Lib.sokonanoda");
    let lib_src = std::fs::read_to_string(&lib_path).expect("读 Lib");
    for round in 0..12u64 {
        std::fs::write(&lib_path, format!("{lib_src}axiom extra{round} : Point\n"))
            .expect("写 Lib");
        doc.set_text(&current, 100 + round, None);
    }
    // ③ 泄漏 arena 数封顶（上界 ①）；到顶之后走回退路 —— 报告仍必须正确。
    assert_eq!(
        lib_checkpoint_arenas_leaked(),
        8,
        "泄漏 arena 数必须封顶在 `MAX_LEAKED_LIB_ARENAS`（12 轮库层改动 ⇒ 8 份封顶）"
    );
    assert!(
        !lib_checkpoint_is_live(),
        "上界用尽之后不许再留检查点（回退路：栈上 arena，与今天逐字节相同）"
    );
    let report = doc.report.as_ref().expect("必须有报告");
    assert!(
        report
            .decls
            .iter()
            .any(|d| d.name.as_deref() == Some("t01") && format!("{:?}", d.status) == "Checked"),
        "上界用尽后的回退路仍必须给出正确判定（`t01` checked）"
    );
}

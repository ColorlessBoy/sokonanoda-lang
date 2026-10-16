//! **T2-B 的入口尾快照：连着改最后一条命令时，续编必须**每一刀都命中** ✓。
//!
//! ## 为什么单开一个文件（不是"顺手放进 `keystroke_structure.rs`"）
//!
//! 同 `g29_closure_recompile.rs` 的理由（那条注释里记着实测）：**同一进程里的用例
//! 会互相污染** ✗。这里的具体污染源是 **`LEAKED_LIB_ARENAS`** —— 它是
//! `crates/front/src/project/session.rs` 里的**进程级** `AtomicUsize`（上界
//! `MAX_LEAKED_LIB_ARENAS = 8`）：库层检查点要存成 `'static` 就得 `Box::leak`
//! 一份 arena，泄漏数**只增不减**；用尽之后会话落进"**栈上 arena + `tails = None`**"
//! 那条路 ⇒ **本进程内后续每一次编译都不再续编**（实测：`keystroke_structure.rs`
//! 的 9 个用例并行跑时，排到最后的用例读到 `第一刀 = N` ✗）。
//!
//! 那是**另一条**性能悬崖（见 `docs/CI-FAILURES.md` 2026-10-16 那条），**不该**让
//! 本文件的读数变成**顺序相关** ✗ ⇒ 本文件的用例**独立成进程**（集成测试一个文件
//! 一个二进制 ✓）⇒ 预算恒是新的 ✓，断言**一个字不放宽** ✓。
//!
//! ## 判据（`PLAN-align-lean4` §3.3 T2-B）
//!
//! | 场景 | [`elaborated_commands_total`] 增量 |
//! | --- | --- |
//! | 开档后**第一刀**改最后一条 | **1** |
//! | **紧接着第二刀**改最后一条 | **1**（✗→✓ 2026-10-16：修前 = N） |
//!
//! **反向验证**：`SOKO_NO_ENTRY_SNAPSHOT=1`（`project/session.rs` 的逃生门）⇒
//! 两刀都回到 **N** ⇒ 两个用例都判红 ✓。

use std::path::{Path, PathBuf};

use sokonanoda_front::compile::elaborated_commands_total;
use sokonanoda_front::query::QueryDoc;

/// 入口文件里 `t…` 的条数（`keystroke_structure.rs` 的 `DECLS` 同值）。
const DECLS: usize = 8;

/// 入口文件的**命令数**（`import Lib` + `DECLS` 条 `t…` + 3 条 `d…`）—— T2-B 的 N ✓。
const ENTRY_COMMANDS: u64 = DECLS as u64 + 1 + 3;

/// 合成项目（`Lib ← Main`），与 `keystroke_structure.rs::gen_project` **同纪律**
/// （合成、不依赖课程目录 ✓）；`tag` 只用来隔离临时目录。
fn gen_project(tag: &str) -> (PathBuf, String) {
    let dir = std::env::temp_dir().join(format!("soko-t2b-tail-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    let lib = "axiom Point : Type\n\
               axiom EqP : Point -> Point -> Prop\n\
               infix:50 \" \u{2248} \" => EqP\n\
               axiom refl : (a : Point) -> a \u{2248} a\n\
               axiom trans : (a b c : Point) -> a \u{2248} b -> b \u{2248} c -> a \u{2248} c\n";
    std::fs::write(dir.join("Lib.sokonanoda"), lib).expect("write Lib");
    let mut main = String::from("import Lib\n\n");
    for i in 0..DECLS {
        main.push_str(&format!(
            "theorem t{i:02} (a b : Point) (h : a \u{2248} b) : a \u{2248} b := by exact h\n\n"
        ));
    }
    main.push_str(
        "theorem d00 (a b : Point) (h : a \u{2248} b) : a \u{2248} b := by exact t00 a b h\n\n",
    );
    main.push_str(
        "theorem d01 (a b : Point) (h : a \u{2248} b) : a \u{2248} b := by exact d00 a b h\n\n",
    );
    main.push_str(
        "theorem d02 (a b : Point) (h : a \u{2248} b) : a \u{2248} b := by exact t00 a b h\n\n",
    );
    let entry = dir.join("Main.sokonanoda");
    std::fs::write(&entry, &main).expect("write Main");
    (entry, main)
}

fn open_doc(entry: &Path, text: &str) -> QueryDoc {
    let mut doc = QueryDoc::new();
    doc.path = Some(entry.to_path_buf());
    doc.set_text(text, 1, None);
    doc
}

/// 一次**改最后一条命令**的按键（长度不变 ✓）：`d02` 的 binder `(h :` → `(k :`
/// （`exact t00 a b h` → `exact t00 a b k`）。
fn edit_last_decl(text: &str) -> String {
    rewrite_d02(text, "(h :", "(k :", "exact t00 a b h", "exact t00 a b k")
}

/// **第二次**改最后一条命令（长度不变 ✓）：`(k :` → `(j :`
/// （`exact t00 a b k` → `exact t00 a b j`）。
///
/// ⚠ **必须与 [`edit_last_decl`] 是两份不同的文本**：同文本会命中 T-A21 的
/// 「文本没变就短路」⇒ 什么都不编 ⇒ 读数 0，量到的不是 T2-B ✗（实测踩到）。
fn edit_last_decl_again(text: &str) -> String {
    rewrite_d02(text, "(k :", "(j :", "exact t00 a b k", "exact t00 a b j")
}

/// 只重写 `d02` 那一行（两处替换 + 长度不变断言），其余字节不动 ✓。
fn rewrite_d02(text: &str, from_b: &str, to_b: &str, from_e: &str, to_e: &str) -> String {
    let at = text.find("theorem d02 ").expect("找不到 `theorem d02`");
    let end = text[at..].find('\n').map_or(text.len(), |n| at + n);
    let line = &text[at..end];
    let edited = line.replacen(from_b, to_b, 1).replacen(from_e, to_e, 1);
    assert_ne!(edited, line, "这一刀必须真的改变文本（否则量到的是短路 ✗）");
    assert_eq!(
        edited.len(),
        line.len(),
        "这次按键必须**长度不变**（否则下游起点平移，量到的不是「最后一条」）"
    );
    format!("{}{}{}", &text[..at], edited, &text[end..])
}

/// 一次 `set_text` **真的 elaborate 了几条命令**（进程级累计取差；同线程 ✓）。
fn elaborated_by<F: FnOnce()>(f: F) -> u64 {
    let before = elaborated_commands_total();
    f();
    elaborated_commands_total() - before
}

/// **T2-B 的判据**（2026-10-09 落地 · 2026-10-13 订正注释 · 2026-10-16 移进本文件）：
/// 改**最后一条命令**时，入口趟**只 elaborate 那一条** ✓ —— 命令级环境快照
/// （`WalkCheckpoint` + `checkpoint()`/`restore_from()`）命中检查点 ⇒ 前缀整段跳过 ✓。
///
/// **反向验证**：改依赖文件 ⇒ 回到 N；逃生门 `SOKO_NO_ENTRY_SNAPSHOT=1`
/// （`project/session.rs`）⇒ 回到"整条重走"⇒ 本判据**判红** ✓。
#[test]
fn t2b_last_command_edit_reelaborates_only_that_command() {
    let (entry, text) = gen_project("one");
    let mut doc = open_doc(&entry, &text);
    let edited = edit_last_decl(&text);
    let n = elaborated_by(|| doc.set_text(&edited, 2, None));
    println!("PERF t2b 改最后一条：elaborated_commands={n}（入口命令数 N={ENTRY_COMMANDS}）");
    assert!(
        n > 0,
        "改最后一条必须真的 elaborate 了东西（读到 0 ⇒ 量具坏了 ✗）"
    );
    assert_eq!(
        n, 1,
        "改**最后一条**命令 ⇒ 入口趟只许 elaborate **1** 条；实测 {n} ⇒ 检查点没命中 \
         （或判据被人放宽了 ✗）"
    );
}

/// **T2-B 的第二刀**（2026-10-16 · 队列 `LSP-T2B-TRAILING-BY-LE-3-RED-ON-CI` 的根因）：
/// **连着两次**改最后一条命令，**两次都必须只 elaborate 1 条** ✓。
///
/// ## 为什么必须有它（上一版判据只覆盖第一刀）
///
/// [`t2b_last_command_edit_reelaborates_only_that_command`] 只量**开档之后的第一刀**
/// ⇒ 掩盖了一条真实的不变式断裂 ✗：入口趟的**尾快照只取在 `flat.len() - 2` 这个
/// 边界上**（`check/walk.rs` 的 `idx + 2 == flat.len()`），而**续编那一趟从
/// `cp.idx + 1` 起走** ⇒ 它**走不到**那个边界 ⇒ **取不到新快照** ✗。而调用方
/// （`project/session.rs`）对快照的纪律是「**搬走**（`store.remove`）⇒ 跑完存回
/// 新的」—— 新的没来 ⇒ 那份**被搬走的就没了** ⇒ **下一刀必然整条重走** ✗。
///
/// 实测（本机 macOS/arm64 · 真 LSP 子进程 · `units/I.3/unit08-images-preimages`，
/// 只往文件末尾追加注释）：`by` = **9 · 1 · 9 · 1 · 9 · 1 · 9** —— 正是这个**交替** ✓。
/// 判据 `crates/lsp/tests/perf_keystroke_wallclock.rs` 读的是**最后一刀** ⇒ 它读到的
/// 是 1 还是 9 **取决于奇偶**（= 之前真的跑了几趟入口 pass），而那是**环境相关**的
/// （缓存/产物命中与否）⇒ **CI 上偶发判红** ✗（CI run `38828745723` 实得 9）。
///
/// ## 判据
///
/// 两刀**各自**的增量都 = **1** ✓。**先建先红**：修根因之前，第一刀 1、**第二刀 = N** ✓。
#[test]
fn t2b_two_consecutive_last_command_edits_each_reelaborate_only_that_command() {
    let (entry, text) = gen_project("twice");
    let mut doc = open_doc(&entry, &text);
    let first = edit_last_decl(&text);
    let n1 = elaborated_by(|| doc.set_text(&first, 2, None));
    let second = edit_last_decl_again(&first);
    let n2 = elaborated_by(|| doc.set_text(&second, 3, None));
    println!(
        "PERF t2b-twice 连着两次改最后一条：第一刀={n1} 第二刀={n2}\
         （入口命令数 N={ENTRY_COMMANDS}）"
    );
    assert!(
        n1 > 0 && n2 > 0,
        "两次改最后一条都必须真的 elaborate 了东西（读到 {n1} / {n2} ⇒ 量具坏了或短路 ✗）"
    );
    assert_eq!(
        n1, 1,
        "**第一刀**改最后一条 ⇒ 只许 elaborate 1 条；实测 {n1}"
    );
    assert_eq!(
        n2, 1,
        "**第二刀**改最后一条 ⇒ 也必须只 elaborate 1 条；实测 {n2}（= 入口命令数 ⇒ \
         尾快照被「搬走」之后**没有补回来**，下一刀整条重走 ✗）"
    );
}

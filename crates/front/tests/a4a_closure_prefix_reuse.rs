//! **A4a 的判据**（2026-10-08）：闭包前缀的**累加**不许每一刀重跑一遍。
//!
//! ## 病灶（读码 + 规划 §2 A4a 的收窄版）
//!
//! 入口趟要的前缀 = **库层全部单元拼接之后**那一段（`entry_closure` 的最后一格 ✓）。
//! 它是 `lib_key` 的**纯函数**，却在**每一刀**都由 `closure_prefixes_for(&entry_closure)`
//! 重跑一遍 O(闭包) 的累加 ✗ —— 而库层检查点（G-29）**早就**把同一个库层留着 ✓。
//!
//! ⚠ **为什么不是"三张表都按闭包文本做键"**（原判据的收窄）：那三张表是**入口闭包**
//! （库层 + 入口）的函数，而**入口文本每一刀都在变** ⇒ 按闭包文本做键**必然 miss** ✗。
//! 真正可缓存的是**库层那一段**（`lib_key` 的函数 ✓）—— 本判据量的就是它。
//!
//! ## 判据（结构计数 · 独立进程 ⇒ 进程级计数器不串味 ✓）
//!
//! * 冷开（= LSP 的 `didOpen`）：`closure_prefix_builds_total()` 至少 +1（库层趟那一次）；
//! * **第 2 刀**（库层检查点复用）：**+0** ✓ —— 这就是"A4a 落地"的正身。
//!
//! **反向验证**：把入口趟改回 `closure_prefixes_for(&entry_closure).last()`
//! ⇒ 第 2 刀必须 **> 0**（本用例判红）✓。
//!
//! 逐字节等价由 `--json` 对拍另证（`crates/cli/tests/` 与发版节点的全课程逐字节 ✓）。

use std::path::PathBuf;

use sokonanoda_front::compile::closure_prefix_builds_total;
use sokonanoda_front::project::session::lib_checkpoint_reset;
use sokonanoda_front::query::QueryDoc;

/// 合成闭包夹具（与 `g29_closure_recompile.rs` 同纪律：**合成**、不依赖课程目录）。
fn gen_project(tag: &str) -> (PathBuf, String) {
    let dir = std::env::temp_dir().join(format!("soko-a4a-{tag}-{}", std::process::id()));
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

/// 一次**真实且长度不变**的按键（改 `t00` 的局部 binder 名 `h` → `k`）—— 同 g29 的夹具纪律
/// （长度一变，后面命令的起点平移 ⇒ 量到的就不是这一次改动 ✗）。
fn edit_decl(text: &str) -> String {
    let needle = "theorem t00 ";
    let at = text.find(needle).expect("夹具里必须有 `t00`");
    let end = text[at..].find('\n').map_or(text.len(), |n| at + n);
    let line = &text[at..end];
    let edited = line
        .replacen("(h :", "(k :", 1)
        .replacen("exact h", "exact k", 1);
    assert_eq!(edited.len(), line.len(), "这次按键必须**长度不变**");
    format!("{}{}{}", &text[..at], edited, &text[end..])
}

#[test]
fn closure_prefix_is_accumulated_once_per_checkpoint_not_per_keystroke() {
    lib_checkpoint_reset();
    let (entry, text) = gen_project("prefix");
    let mut doc = QueryDoc::new();
    doc.path = Some(entry.clone());

    // 冷开：库层趟 + 入口趟各累加一次（**至少 1** —— 具体次数是实现细节，
    // 判据要的是"第 2 刀起为 0"这个**方向** ✓）。
    let base = closure_prefix_builds_total();
    doc.set_text(&text, 1, None);
    let cold = closure_prefix_builds_total() - base;
    assert!(
        cold >= 1,
        "冷开必须至少累加一次闭包前缀（实测 {cold}）⇒ 夹具或计数口径变了"
    );
    assert!(
        doc.report.is_some(),
        "冷开必须产出报告（否则量的是错误路径）"
    );

    // **被量的那一刀**：库层检查点复用 ⇒ 前缀从检查点克隆，**不许再累加** ✓。
    let base = closure_prefix_builds_total();
    doc.set_text(&edit_decl(&text), 2, None);
    let edit = closure_prefix_builds_total() - base;
    assert_eq!(
        edit, 0,
        "**A4a 的正身**：库层检查点命中时，闭包前缀**不许**再重跑一遍累加（实测 {edit} 次）✗\n\
         ⇒ 入口趟该用 `LibCheckpoint::lib_prefix`（库层那一段是 `lib_key` 的纯函数 ✓），\n\
         而不是 `closure_prefixes_for(&entry_closure).last()`。"
    );

    // 正确性不变量（顺带）：改 `t00` 之后没改过的 `t01` 不许掉出 checked。
    let report = doc.report.as_ref().expect("改完必须有报告");
    let t01 = report
        .decls
        .iter()
        .find(|d| d.name.as_deref() == Some("t01"))
        .expect("`t01` 必须在报告里");
    assert_eq!(
        format!("{:?}", t01.status),
        "Checked",
        "改 `t00` ⇒ 没改过的 `t01` 不许掉出 checked（前缀复用不许把环境搬错 ✗）"
    );
}

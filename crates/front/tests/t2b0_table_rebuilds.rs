//! **T2-B0 的前置读数**（2026-10-09）：入口趟那**两张派生表**（记法表 · 定义 span 表）
//! 今天**每刀重算** ✗ —— 本文件先把"几次"量出来（`PLAN-align-lean4.md` 的 T2-B0 ✓）。
//!
//! ## 为什么先量
//!
//! 这两张表都是**闭包文本的纯函数**（`display_notations` / `top_level_def_spans_over` ✓），
//! 按 A4a 的教训（`fc636b3e`）正确切法是"**库层那一段随检查点存 + 入口那一段每刀只算入口**" ✓
//! —— 但**在动手之前**先要能读到"今天建了几次" ✓：在此之前它们**连一次都量不到** ✗。
//!
//! ⚠ **本文件的断言写的是"今天的行为"**（每刀 ≥1）—— 它是**防漂移**用的 ✓；
//! T2-B0 落地之后按判据改成 **0**（照 T3-B2 的先例：**不许放宽**、要**改判** ✓）。
//!
//! ⚠ **冷开前必须先清产物**（§32 的纪律 ✓）：模块根里若已有磁盘产物，`QueryDoc` 的"冷开"
//! 会**从产物装载库层** ⇒ 冷开那一段的计数会变 ✗。

use sokonanoda_front::compile::{def_spans_builds_total, notation_table_builds_total};
use sokonanoda_front::query::QueryDoc;

fn module_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("courses")
        .join("set-theory")
}

/// 在 `Set.mem_image α β f A<k 空格>y` 里再加一个空格 ⇒ **每一刀都是没编过的新文本** ✓。
fn add_one_space(current: &str) -> String {
    let marker = "Set.mem_image α β f A";
    let at = current.find(marker).expect("夹具前提：锚点必须在");
    let rest = &current[at + marker.len()..];
    let spaces = rest.chars().take_while(|c| *c == ' ').count();
    let mut next = String::with_capacity(current.len() + 1);
    next.push_str(&current[..at + marker.len()]);
    next.push_str(&" ".repeat(spaces + 1));
    next.push_str(&rest[spaces..]);
    next
}

#[test]
fn the_two_derived_tables_are_rebuilt_on_every_keystroke_today() {
    let root = module_root();
    let entry = root.join("units/I.3/unit08-images-preimages.sokonanoda");
    if !entry.is_file() {
        eprintln!("跳过：找不到课程单元（课程仓可分开检出）");
        return;
    }
    // **冷开前先清产物**（§32 的纪律 ✓）：否则"冷开"会从产物装载库层 ⇒ 计数会变 ✗。
    let _ = sokonanoda_front::project::cache::clean_at(&root);

    let text = std::fs::read_to_string(&entry).expect("read unit08");
    assert!(
        text.contains("Set.mem_image α β f A y"),
        "夹具前提：编辑锚点必须在"
    );

    let mut doc = QueryDoc::new();
    doc.path = Some(entry.clone());

    let n0 = notation_table_builds_total();
    let d0 = def_spans_builds_total();
    doc.set_text(&text, 1, None);
    let cold = (
        notation_table_builds_total() - n0,
        def_spans_builds_total() - d0,
    );
    assert!(
        cold.0 > 0 && cold.1 > 0,
        "冷开必须真的建过这两张表（否则计数器没接上 / 路径变了 ✗）—— 实测 {cold:?}"
    );

    // 三刀**新文本**（插入空格 ⇒ 后缀偏移整体平移 ⇒ 不是"两份文本来回"那个假象 ✓）。
    let mut current = text.clone();
    let mut per: Vec<(u64, u64)> = Vec::new();
    for v in 2..5u64 {
        current = add_one_space(&current);
        let a = notation_table_builds_total();
        let b = def_spans_builds_total();
        doc.set_text(&current, v, None);
        per.push((
            notation_table_builds_total() - a,
            def_spans_builds_total() - b,
        ));
    }
    println!("PERF t2b0 tables: cold={cold:?} per_keystroke={per:?}");

    // ⚠ **这条断言写的是"今天的行为"**（防漂移 ✓）：每刀都重算。
    // T2-B0 落地之后按判据改成 **0**（不许放宽 ✗）。
    assert!(
        per.iter().all(|(n, d)| *n >= 1 && *d >= 1),
        "**今天的行为**：每刀重算这两张表（T2-B0 的目标是 0）—— 实测 {per:?}"
    );
}

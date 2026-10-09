//! **`LineIndex` 与 `line_col_of` 的逐位 parity 判据（第 99 轮 · 平行线）** —— 补上第 98 轮
//! 落地"作用域行首索引"时**缺的那条直接判据** ✗（当时只有"LSP 全套 + 响应体积逐字节不变"这种
//! **间接**证据，不够 ✓）。
//!
//! 判据：对**每一个** offset（含 0、含 EOF、含越界 ✓）与一组**刁钻文本**（多行 / 中文 /
//! emoji（代理对）/ 组合字符 / CRLF / 空行 ✓），索引路与旧扫描路必须给出**完全相同**的
//! `(行, 列)` ✓（1-based、列按 **UTF-16** ✓）。**先建先红**：把索引实现换成"行号恒 1"必红 ✓。

use sokonanoda_front::query::{line_col_of, LineIndex};

fn cases() -> Vec<(&'static str, String)> {
    vec![
        ("空文本", String::new()),
        ("单行无换行", "theorem t : P := h".to_string()),
        ("结尾换行", "a\n".to_string()),
        ("多行", "a\nbb\nccc\n".to_string()),
        ("空行夹心", "a\n\n\nb\n".to_string()),
        ("中文（3 字节）", "定理\n引理 x\n".to_string()),
        ("emoji（代理对）", "a😀b\n😀😀\n".to_string()),
        ("组合字符", "e\u{301}x\n".to_string()),
        ("CRLF", "a\r\nb\r\n".to_string()),
        ("仅换行", "\n\n".to_string()),
        ("长行 + 尾注释", format!("{}\n-- 尾巴\n", "x".repeat(300))),
    ]
}

#[test]
fn the_line_index_matches_the_reference_for_every_offset() {
    let mut checked = 0usize;
    for (name, text) in cases() {
        let idx = LineIndex::new(&text);
        // **含越界**（`len+1`）：旧路会走完整个文本、索引路要给同一个答案 ✓。
        for offset in 0..=(text.len() + 1) {
            let want = line_col_of(&text, offset);
            let got = idx.line_col_of(&text, offset);
            assert_eq!(
                got,
                want,
                "`{name}` · offset {offset}/{}：索引路 {got:?} ≠ 旧路 {want:?}",
                text.len()
            );
            checked += 1;
        }
    }
    assert!(checked > 100, "判据要真的跑起来（检查了 {checked} 个点）");
    println!("PERF line-index-parity: 逐位相同 ✓ · 共比对 {checked} 个 offset（含 0/EOF/越界/代理对/CRLF ✓）");
}

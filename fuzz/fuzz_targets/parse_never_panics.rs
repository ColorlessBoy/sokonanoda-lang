//! Fuzz target: parser/lexer 永不 panic（REQUIREMENTS 硬要求）。
//!
//! - `parse` 返回 Result：任意输入只能是编译错误，不得 panic；
//! - `semantic_tokens`：词法分类对任意 UTF-8 输入不 panic；
//! - `prelude_mode_from_source`：纯源码扫描不 panic；
//! - parse 成功时走一次完整 `check_document`（含内核）——若某种输入能让
//!   内核 panic，那是真 bug，fuzz 会抓住。

#![no_main]
use libfuzzer_sys::fuzz_target;
fuzz_target!(|data: &[u8]| {
    let Ok(src) = std::str::from_utf8(data) else { return };
    let _ = sokonanoda_front::parse(src);                       // parse 永不 panic（Result）
    let _ = sokonanoda_front::semantic::semantic_tokens(src);   // 词法分类不 panic
    let _ = sokonanoda_front::compile::prelude_mode_from_source(src);
    // 更狠的一段：parse 成功时走一次完整 check_document（含内核），
    // 若发现内核对某种输入 panic——那是真 bug，fuzz 会抓住。
    if let Ok(file) = sokonanoda_front::parse(src) {
        let _ = sokonanoda_front::compile::check_document(&file);
    }
});

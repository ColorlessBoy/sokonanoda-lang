//! **内核 pp 的 `Type 0` 按 Lean 4 印成 `Type`**（显示层，判定一行不动）。
//!
//! 从 `crates/lsp/src/render.rs` 搬来（2026-10-15，**唯一实现** ✗→✓）：`#check <def>`
//! 的**声明卡片**（用户 2026-10-15 15:22 的需求，见 `REQUIREMENTS.md` §9.22）也用这条
//! 改写 ⇒ 两条出口（编辑器卡片 / 命令行 `#check`）必须过**同一条**规则，而它得落在
//! **两层都能调**的地方：`front`。LSP 侧改为**转发**（`lsp::render` 的 re-export）。
//! 本文件**独立成模块**（不塞进 `display.rs`）是判据 ⑨ 的要求：`display.rs` 已在
//! 源码行数基线上，**只许减不许增** ✗（`AGENTS.md` 硬规则 5）。
//!
//! ## Lean 4 对照（本机源码 `~/Documents/lean/lean4/src/Lean/**`）
//!
//! * `PrettyPrinter/Delaborator/Builtins.lean:82-91` `delabSort`：
//!   `Level.zero => Prop` · **`Level.succ .zero => Type`**（**层级 0 是丢掉的**）·
//!   更高层 `Type n` · 非数字层 `Sort u`。
//! * `…/Builtins.lean:1009`（`delabForall`）与 `:913`（`delabLam`）的 binder 域走
//!   `withBindingDomain delab` ⇒ **binder 位与结论位同一条规则，没有特判**。
//! * `…/Delaborator/Options.lean:56` `pp.universes` 默认 `false`，且 `delabSort`
//!   **根本不读它**（它只管常量应用的层实参，`Builtins.lean:113`）⇒ 与本规则无关。
//!
//! ## 我们这边的缝（F6，2026-10-13）
//!
//! 内核 pp 的 `ty` 是**原样搬运** —— `crates/kernel/src/pretty_printer.rs` 的
//! `pp_sort` 对 `Succ` **无条件**写层 ⇒ `Type 0`；而目标卡片那一半
//! （`binders[].ty`）走的是**源级**渲染（写的是 `Type`）⇒ 同一份类型两处两种写法。
//! 内核 pp 的修法在 `crates/kernel`，本层做的是**显示层**的等价改写。
//!
//! ## 为什么 runs 必须一起搬
//!
//! `ty_runs` 是 `ty` 的着色分段（T-A5：**文本与 runs 必须成对产出**）。删掉两个字节
//! 而不同步搬偏移 ⇒ 从第一个 `Type 0` 起的高亮全部错位（"数据对了 ≠ 用户看见了"）。
//! `lean4_universe_runs`（LSP 侧，要 `crate::protocol::RunInfo`）按**分段自己的文本**切，
//! 偏移由拼接位置重算 ⇒ 输出的"文本 ↔ runs"自洽是**构造保证**的。
//!
//! **退役条件**：内核 `pp_sort` 按 Lean 4 丢掉 `Type 0` 的层之后，本组函数整体删掉。

/// `Type 0` 里那两个字节（` 0`）在 `text` 里的字节区间。
///
/// 判据（与 Lean 4 的 `delabSort` 同口径）：`Type` 是**独立 token**
/// （前一个字节不是标识符字符）、后面恰好是**空格 + `0`**、且 `0` 后面不是数字。
/// `Type 1` / `Type u` / `Sort u` / `Prop` **一个都不动** ✓。
pub fn level_zero_cuts(text: &str) -> Vec<(usize, usize)> {
    let bytes = text.as_bytes();
    let mut cuts: Vec<(usize, usize)> = Vec::new();
    let mut from = 0usize;
    while let Some(found) = text[from..].find("Type") {
        let at = from + found;
        from = at + "Type".len();
        let prev_is_ident = at > 0 && {
            let b = bytes[at - 1];
            b.is_ascii_alphanumeric() || b == b'_' || b == b'\'' || b == b'.' || b >= 0x80
        };
        if prev_is_ident {
            continue;
        }
        let rest = &text[from..];
        let spaces = rest.len() - rest.trim_start_matches(' ').len();
        if spaces == 0 {
            continue;
        }
        let after = &rest[spaces..];
        if !after.starts_with('0') || after[1..].starts_with(|c: char| c.is_ascii_digit()) {
            continue;
        }
        // 删掉 ` 0` 两个字节（`spaces >= 1` ⇒ `zero_at - 1` 就是那个空格）。
        let zero_at = from + spaces;
        cuts.push((zero_at - 1, zero_at + 1));
    }
    cuts
}

/// 文本版：`Type 0` → `Type`（给 Markdown 形态的卡片与 `#check <def>` 的结论用；
/// 没有 runs 要搬）。
pub fn lean4_universe_text(text: &str) -> String {
    let cuts = level_zero_cuts(text);
    if cuts.is_empty() {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    let mut cursor = 0usize;
    for (start, end) in &cuts {
        out.push_str(&text[cursor..*start]);
        cursor = *end;
    }
    out.push_str(&text[cursor..]);
    out
}

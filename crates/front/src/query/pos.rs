//! 位置换算：字节 offset ↔ **1-based** 行/列（列按 UTF-16 code unit）。
//!
//! 从 `mod.rs` 拆出（模块化硬规则）。真相层内部一律用 offset；只有适配器的
//! 边界（CLI 的 `--line/--col`、MCP 的 `line/character`）才换算，且**只有这里**
//! 一份实现——列口径错了会让所有消费者一起错。

/// 位置换算：字节 offset → **1-based** 行/列（列按 UTF-16 code unit，与 LSP 的
/// `character` 口径一致）。适配器负责把它转成自己要的基数。
pub fn line_col_of(text: &str, offset: usize) -> (usize, usize) {
    // **作用域索引命中** ⇒ 走索引 ✓（结果逐位相同 ✓，见 `with_line_index` 与 parity 判据 ✓）。
    let hit = SCOPED_INDEX.with(|c| {
        c.borrow()
            .as_ref()
            .filter(|(p, n, _)| *p == text.as_ptr() as usize && *n == text.len())
            .map(|(_, _, idx)| idx.line_col_of(text, offset))
    });
    if let Some(pair) = hit {
        return pair;
    }
    let mut line = 1usize;
    let mut col = 1usize;
    for (i, ch) in text.char_indices() {
        if i >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += ch.len_utf16();
        }
    }
    (line, col)
}

/// 位置换算：**1-based** 行/列（列按 UTF-16 code unit）→ 字节 offset。
/// 越界时返回 `None`（调用方把它变成 [`QueryError::PositionOutOfRange`]）。
pub fn offset_of_line_col(text: &str, line: usize, col: usize) -> Option<usize> {
    if line == 0 || col == 0 {
        return None;
    }
    let mut cur_line = 1usize;
    let mut cur_col = 1usize;
    for (i, ch) in text.char_indices() {
        if cur_line == line && cur_col == col {
            return Some(i);
        }
        if ch == '\n' {
            if cur_line == line {
                // 请求的行在这一行的换行处结束：夹到行尾。
                return Some(i);
            }
            cur_line += 1;
            cur_col = 1;
        } else {
            cur_col += ch.len_utf16();
        }
    }
    if cur_line == line {
        return Some(text.len());
    }
    None
}

/// **行首索引**（第 98 轮 · 平行线）：一次 `O(文本)` 建表 ⇒ 之后每次换算只 `O(该行长度)` ✓。
///
/// **为什么需要它**：`line_col_of` 是"**从文本头扫到 offset**"✗（源码就这么写的 ✓），
/// 而 `crates/lsp/src/query_map.rs::decl_info` 对**每条声明**（外加每个洞 ✓）各调它 **2 次** ⇒
/// 真 unit08 上 27+ 条 × O(24KB) ⇒ 实测把 `soko/goals` 的往返推到 ~6ms ✗
/// （读数见 `crates/lsp/tests/goals_lsp_layers.rs` ✓：固定开销 0.1ms、全在"每声明"那段 ✓）。
///
/// **契约**：`line_col_of` 与它**逐位相同**（1-based 行/列 ✓，列按 **UTF-16** code unit ✓）——
/// 判据 `crates/front/tests/line_index_parity.rs`（多语言/多行/边界偏移逐一对比 ✓）。
pub struct LineIndex {
    /// 每**行首**的字节偏移（含第 0 行 = 0 ✓）。
    starts: Vec<usize>,
}

impl LineIndex {
    /// 建表：一次线性扫描 ✓（只记 `\n` 之后的偏移 ✓）。
    pub fn new(text: &str) -> Self {
        let mut starts = vec![0usize];
        for (i, ch) in text.char_indices() {
            if ch == '\n' {
                starts.push(i + 1);
            }
        }
        Self { starts }
    }

    /// 与 [`line_col_of`] **逐位相同**的 1-based 行/列（列按 UTF-16 ✓）。
    pub fn line_col_of(&self, text: &str, offset: usize) -> (usize, usize) {
        // 找"最后一个行首 ≤ offset"的那一行（`partition_point` = 二分 ✓）。
        let line = self.starts.partition_point(|s| *s <= offset);
        let line = line.max(1); // 1-based ✓
        let start = self.starts[line - 1];
        // 行内：从行首数到 offset 的 UTF-16 长度 ✓（短 ✓）。
        let mut col = 1usize;
        for (i, ch) in text[start..].char_indices() {
            if start + i >= offset {
                break;
            }
            col += ch.len_utf16();
        }
        (line, col)
    }
}

thread_local! {
    /// **作用域内**的行首索引（见 [`with_line_index`] ✓）。键 = `(ptr, len)` —— **只在作用域内有效** ✓
    /// （作用域退出即清 ✓）⇒ 不会拿"别的文本"的索引算位置 ✓（那会静默算错 ✗）。
    static SCOPED_INDEX: std::cell::RefCell<Option<(usize, usize, LineIndex)>> =
        const { std::cell::RefCell::new(None) };
}

/// **在作用域内用行首索引加速位置换算**（不改变任何签名 ✓）。
///
/// 动机：`line_col_of` 是"从头扫到 offset" ✗，而 `query_map` 对**每条声明/每个洞**各调 2 次
/// ⇒ 真 unit08 上 27+ 条 × O(24KB) ⇒ 实测把 `soko/goals` 推到 ~6ms ✗
/// （`crates/lsp/tests/goals_lsp_layers.rs` ✓：固定开销 0.1ms、全在"每声明"那段 ✓）。
///
/// **语义**：作用域内、且**同一份 `text`**（按 `(ptr, len)` 认 ✓）⇒ 走索引 ✓（结果与旧路**逐位相同** ✓，
/// 判据 `crates/front/tests/line_index_parity.rs` ✓）；其余情况一律**退回原扫描** ✓ ⇒ 行为不变 ✓。
/// **为什么不是参数**：`range_of_offsets` 有 8 处调用、`decl_info`/`hole_info` 还要跨 crate 传 —— 签名
/// 改动面太大 ✗（这一区的纪律："只许一处新增或手工单点" ✓）。
pub fn with_line_index<T>(text: &str, f: impl FnOnce() -> T) -> T {
    struct Clear;
    impl Drop for Clear {
        fn drop(&mut self) {
            SCOPED_INDEX.with(|c| *c.borrow_mut() = None);
        }
    }
    SCOPED_INDEX.with(|c| {
        *c.borrow_mut() = Some((text.as_ptr() as usize, text.len(), LineIndex::new(text)));
    });
    let _clear = Clear;
    f()
}

//! 位置换算：字节 offset ↔ **1-based** 行/列（列按 UTF-16 code unit）。
//!
//! 从 `mod.rs` 拆出（模块化硬规则）。真相层内部一律用 offset；只有适配器的
//! 边界（CLI 的 `--line/--col`、MCP 的 `line/character`）才换算，且**只有这里**
//! 一份实现——列口径错了会让所有消费者一起错。

/// 位置换算：字节 offset → **1-based** 行/列（列按 UTF-16 code unit，与 LSP 的
/// `character` 口径一致）。适配器负责把它转成自己要的基数。
pub fn line_col_of(text: &str, offset: usize) -> (usize, usize) {
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

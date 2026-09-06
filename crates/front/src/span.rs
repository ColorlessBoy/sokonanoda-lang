//! 源码位置与区间：`Pos` 与 `Span`。

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Pos {
    pub offset: usize,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Span {
    pub start: Pos,
    pub end: Pos,
}

impl Span {
    pub fn new(start: Pos, end: Pos) -> Self {
        Self { start, end }
    }
}

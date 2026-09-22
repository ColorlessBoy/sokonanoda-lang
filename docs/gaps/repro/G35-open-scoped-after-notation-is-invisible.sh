#!/usr/bin/env bash
# G-35 复现：回读用的记法表是**扫一遍**的 —— `open scoped Foo` 写在
# `scoped infix` **之后**（正常写法）时，表里收不到那条记法。
#
# 退出码（docs/gaps/README.md）：0 = 缺口仍在 · 1 = 行为已变 · 2 = 环境异常。
#
# 为什么用 cargo 而不是 `scripts/soko`：`notation_table` 是 `pub(crate)`，
# CLI 走不到它；而这个缺口的**用户可见路径**今天不存在（课程不用 `scoped` 记法），
# 所以只能钉内部行为——`crates/front/src/notation.rs` 的那条特征化测试就是判据。
# 它绿 = 行为还是"扫一遍"（缺口在）；它红 = 有人改成两遍扫描了（回来关账）。
set -u
cd "$(dirname "$0")/../../.." || exit 2
command -v cargo >/dev/null 2>&1 || { echo "需要 cargo" >&2; exit 2; }
# macOS 上链接需要 CommandLineTools（Xcode license 未接受时 cc 会失败）。
if [ -d /Library/Developer/CommandLineTools ]; then
  DEVELOPER_DIR=/Library/Developer/CommandLineTools
  export DEVELOPER_DIR
fi
if cargo test -q -p sokonanoda-front --lib \
  notation::tests::open_scoped_after_the_notation_does_not_bring_it_back >/dev/null 2>&1; then
  echo "   → 缺口仍在：open scoped 写在记法之后 ⇒ 回读表里没有它（扫一遍）"
  exit 0
fi
echo "   → 行为已变（特征化测试红了）⇒ 回来更新台账并关账"
exit 1

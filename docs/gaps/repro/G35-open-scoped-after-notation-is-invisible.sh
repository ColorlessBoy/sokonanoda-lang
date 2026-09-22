#!/usr/bin/env bash
# G-35 复现：回读用的记法表曾经是**扫一遍**的 —— `open scoped Foo` 写在
# `scoped infix` **之后**（正常写法）时收不到那条记法。
#
# 退出码（docs/gaps/README.md）：0 = 缺口仍在 · 1 = 已修 · 2 = 环境异常。
#
# 判据是 `crates/front/src/notation.rs` 的特征化测试
# `open_scoped_after_the_notation_is_collected`：它断言**正确**行为
# （两个方向都收得到）。所以：
#   测试红 = 缺口仍在（扫一遍）⇒ exit 0
#   测试绿 = 已修（两遍扫描）  ⇒ exit 1
#
# 为什么用 cargo 而不是 `scripts/soko`：`notation_table` 是 `pub(crate)`，
# CLI 走不到它；而这个缺口的**用户可见路径**很窄（课程不用 `scoped` 记法），
# 所以只能钉内部行为。
set -u
cd "$(dirname "$0")/../../.." || exit 2
command -v cargo >/dev/null 2>&1 || { echo "需要 cargo" >&2; exit 2; }
# macOS 上链接需要 CommandLineTools（Xcode license 未接受时 cc 会失败）。
if [ -d /Library/Developer/CommandLineTools ]; then
  DEVELOPER_DIR=/Library/Developer/CommandLineTools
  export DEVELOPER_DIR
fi
if cargo test -q -p sokonanoda-front --lib \
  notation::tests::open_scoped_after_the_notation_is_collected >/dev/null 2>&1; then
  echo "   → 已修：open scoped 写在记法之前或之后都收得到（两遍扫描）"
  exit 1
fi
echo "   → 缺口仍在：回读表扫一遍 ⇒ open scoped 写在记法之后时收不到它"
exit 0

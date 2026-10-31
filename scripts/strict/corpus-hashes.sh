#!/bin/sh
# 用法：corpus-hashes.sh <binary> <out-file>
# 仓库里全部 tracked .sokonanoda × {loose, strict}（357 × 2 = 714 行），排序后写进 <out-file>。
# 两个二进制比较：diff <base-out> <new-out>（空 = 逐字节相同）。
#
# 先把二进制拷到私有路径再跑：agent 的 target 目录可能在跑的途中被重编
# （2026-10-30 P2-3：跑前 de78cb7f、跑后 104f14cb ⇒ 714 组混用两个二进制，结果作废）。
bin="$1"; out="$2"
here=$(cd "$(dirname "$0")" && pwd)
cd "$here/../.." || exit 2
sha=$(shasum -a 256 "$bin" | cut -c1-16) || exit 2
priv="/tmp/corpus-bin-$sha"
cp "$bin" "$priv" && chmod +x "$priv" || exit 2
echo "pinned $bin -> $priv (sha $sha)"
(git ls-files '*.sokonanoda' | sed 's/^/loose /'; git ls-files '*.sokonanoda' | sed 's/^/strict /') \
  | xargs -P 6 -n 2 "$here/corpus-one.sh" "$priv" | sort > "$out.tmp" && mv "$out.tmp" "$out"
after=$(shasum -a 256 "$priv" | cut -c1-16)
[ "$after" = "$sha" ] || { echo "pinned binary changed during run ($sha -> $after)"; exit 3; }
echo "$(wc -l < "$out" | tr -d ' ') lines ⇒ $out (bin $sha)"

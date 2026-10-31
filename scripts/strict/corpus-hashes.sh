#!/bin/sh
# 用法：corpus-hashes.sh <binary> <out-file>
# 仓库里全部 tracked .sokonanoda × {loose, strict}（357 × 2 = 714 行），排序后写进 <out-file>。
# 两个二进制比较：diff <base-out> <new-out>（空 = 逐字节相同）。**基线与新二进制要在同一检出目录跑**
# （G77 的诊断带模块根绝对路径，换 worktree 哈希就变）。
#
# 先把二进制拷到私有路径再跑：agent 的 target 目录可能在跑的途中被重编
# （2026-10-30 P2-3：跑前 de78cb7f、跑后 104f14cb ⇒ 714 组混用两个二进制，结果作废）。
# 已钉过同一 sha 就复用；新拷走「临时名 + mv」换新 inode ⇒ 绝不原地覆盖正在跑的那份
# （2026-10-31 P2-11a-B：两次对拍并发 cp 到同一路径 ⇒ 12 个进程卡死在 UE 状态 45 分钟）。
bin="$1"; out="$2"
[ -f "$bin" ] || { echo "no binary: $bin" >&2; exit 2; }
here=$(cd "$(dirname "$0")" && pwd)
cd "$here/../.." || exit 2
sha=$(shasum -a 256 "$bin" | cut -c1-16) || exit 2
priv="/tmp/corpus-bin-$sha"
if [ "$(shasum -a 256 "$priv" 2>/dev/null | cut -c1-16)" != "$sha" ]; then
  tmp="$priv.tmp.$$"
  cp "$bin" "$tmp" && chmod +x "$tmp" && mv -f "$tmp" "$priv" || exit 2
fi
echo "pinned $bin -> $priv (sha $sha)"
n=$(git ls-files '*.sokonanoda' | wc -l | tr -d ' ')
(git ls-files '*.sokonanoda' | sed 's/^/loose /'; git ls-files '*.sokonanoda' | sed 's/^/strict /') \
  | xargs -P 6 -n 2 "$here/corpus-one.sh" "$priv" | sort > "$out.tmp" && mv "$out.tmp" "$out" || exit 2
after=$(shasum -a 256 "$priv" | cut -c1-16)
[ "$after" = "$sha" ] || { echo "pinned binary changed during run ($sha -> $after)" >&2; exit 3; }
# 不许静默：行数 = 2 × 文件数；空输出哈希只许 lib/Logic（它两档本来就不输出）⇒ 别处出现 = 超时/崩溃。
lines=$(wc -l < "$out" | tr -d ' ')
[ "$lines" -eq $((2 * n)) ] || { echo "expected $((2 * n)) lines, got $lines" >&2; exit 4; }
bad=$(grep ' e3b0c44298fc1c14$' "$out" | grep -v ' courses/set-theory/lib/Logic.sokonanoda ')
[ -z "$bad" ] || { echo "empty output (timeout/crash?):" >&2; echo "$bad" >&2; exit 4; }
echo "$lines lines ⇒ $out (bin $sha)"

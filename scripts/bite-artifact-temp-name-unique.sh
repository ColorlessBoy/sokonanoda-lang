#!/usr/bin/env bash
# **咬住证明**：产物载荷的**临时名唯一性**（`FLAKY-CONCURRENT-ARTIFACT-WRITERS-TEST`，
# high，2026-10-16 · CI `78cfcaac` 上 `concurrent_writers_never_produce_a_torn_payload`
# 判红 1/1002，而本机同一条命令 20/20 绿 ⇒ 真因是**竞态**，不是"CI 运气差" ✗）。
#
# 用法：`scripts/bite-artifact-temp-name-unique.sh <pinned-rev>` —— **必须**是 `HEAD` 的
# 祖先（C5 可达性：CI 只检出**一个 ref** 的历史 ⇒ 别的分支上的 rev `git show` rc=128 ✗）。
#
# 咬什么：`crates/front/src/project/artifacts.rs` 的 `write()` 里那个临时名。钉住的那一版
# 只带 `std::process::id()` ⇒ **同进程每个线程算出的临时名完全相同** ⇒ 两位写者交错时
# "甲写临时名 → 乙覆盖同一个临时名 → 甲 `rename`" ⇒ 落地的载荷 = **甲的头 + 乙的尾** ✓。
#
# ⚠ **为什么只换 `write()` 的函数体、不换整棵**（`bite2` 的坑 ✗）：换整棵会把**新判据自己**
# 一起换掉 ⇒ `--exact <测试名>` 变成 0 条测试 ⇒ `cargo test` **rc=0** ⇒ 守卫**空转**却被印成
# "咬不住" ✗。这里换完 `write()` 后**先断言新判据还在**（空转闸 ✓），再断言它**真的跑了**
# （`test result: FAILED. … 1 failed` ⇒ 不是"0 条测试"那种假绿 ✓）。
#
# ⚠ **换版前先 `cp` 存一份、还原用 `cp`**（`fx1` 的坑 ✗）：本脚本会**就地**改工作区文件
# ⇒ 绝不能用 `git checkout --` / `git show HEAD:… >` 还原（那会**抹掉未提交改动** ✗）。
set -u

REV=${1:?用法: scripts/bite-artifact-temp-name-unique.sh <pinned-rev>（HEAD 的祖先）}
F=crates/front/src/project/artifacts.rs
TEST=project::artifacts::tests::concurrent_writers_at_a_barrier_never_tear_the_payload

[ -f "$F" ] || { echo "✗ 不在仓库根跑（缺 $F）"; exit 9; }
T=$(mktemp -d) || exit 9
# 无论怎么退出都还原（⚠ 用 `cp` 存的那一份，不是 `git checkout --` ✗）。
# ⚠ 备份还没存下时**绝不能**动 `$F`（否则 `cp` 空源会把它清成 0 字节 ✗）⇒ 先判存在 ✓。
trap 'if [ -f "$T/fixed.rs" ]; then cp "$T/fixed.rs" "$F" 2>/dev/null; fi; rm -rf "$T"' EXIT

git show "$REV:$F" > "$T/old.rs" || { echo "✗ 取不到 $REV:$F"; exit 9; }
cp "$F" "$T/fixed.rs" || exit 9

# 只换 `write()` 的函数体（到 `/// 把产物目录压回` 那条 `prune` 的文档注释为止 ✓）。
python3 -c '
import sys
src, dst, oldf = sys.argv[1], sys.argv[2], sys.argv[3]
def body(p):
    t = open(p, encoding="utf-8").read()
    a = t.index("pub fn write(")
    b = t.index("\n/// 把产物目录压回", a)
    return t[a:b]
s = open(src, encoding="utf-8").read()
o = body(oldf)
a = s.index("pub fn write(")
b = s.index("\n/// 把产物目录压回", a)
open(dst, "w", encoding="utf-8").write(s[:a] + o + s[b:])
' "$T/fixed.rs" "$F" "$T/old.rs" || exit 9

# 空转闸 ①：换的是**旧**临时名（换版真生效 ✓）。口径绑**形状**（不是某一种拼法 ✗）：
# 那份源码里带 `process::id()`、**不**带 `thread::current().id()` ⇒ 同进程写者共享临时名 ✓。
# ⚠ 别只 grep `let tmp = …` 那一行：`rustfmt` 会把旧的那一行**折成多行** ⇒ 断行位置是
#   **格式**的函数、不是形状的函数 ✗（实测：折行后 `process::id()` 落在下一行）。
if ! grep -q 'std::process::id()' "$F" || grep -q 'thread::current().id()' "$F"; then
    echo "✗ 换版没生效：$F 里还不是旧形状（要有 process::id()、不要有 thread::current().id()）"
    echo "   ⇒ 这条证明量不到坏输入"
    exit 9
fi
# 空转闸 ②：**新判据还在**（否则 `--exact` 会 0 条测试 ⇒ 假绿 ✗）。
grep -q "fn concurrent_writers_at_a_barrier_never_tear_the_payload" "$F" || {
    echo "✗ 换版把新判据也换掉了 ⇒ 下面那条会 0 条测试 ⇒ 假绿 ✗"; exit 9; }

out=$(cargo test -p sokonanoda-front --lib -- --exact "$TEST" 2>&1)
rc=$?
printf '%s\n' "$out"
# 空转闸 ③：那条测试**真的跑了**（不是"0 passed; 0 failed"的空转 ✗）。
printf '%s\n' "$out" | grep -q "1 failed" || {
    echo "✗ 咬住证明空转：那条测试没真跑（期望 1 failed）⇒ 读数不可归属"; exit 9; }
exit $rc

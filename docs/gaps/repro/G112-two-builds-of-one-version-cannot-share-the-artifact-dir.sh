#!/usr/bin/env bash
# G-112 —— **同一个版本号的两个构建**（release / debug）共用一个模块根的
# `.sokonanoda/compiled/`，而条目的键里带 `build_stamp` ⇒ 两边**永远互不命中**。
#
# 用户现场（2026-10-11，原话）：Infoview 报「产物：1 条 · 697.5 KiB · 由编译器 0.88.1
# 写入」——他刚装好 0.88.1、**还没 rebuild**，VS Code 自动编了一次 `unit01-…`，整个
# 项目的 `meta.json` 就跟着改了。现场读数：`compiled/` 248 条里 **247 条是
# `0.88.1+209c01a9b479e460+…`、1 条是 `0.88.1+751385453009d556+…`**（同一版本、两个戳），
# 而那条独苗 714200 字节 = **697.46 KiB** —— 就是面板上那一行。
#
# 根因（两半，本条钉第二半）：
#   ① 目录级 `meta.json.compiler/build_stamp` 被**任何一次写入**刷成"当前编译器"
#      （`project::cache::update_index`）—— 显示层已不读它（G-106 修过）；
#   ② `build_stamp` 是**编译期常量**（`compile::cache::build_stamp`：profile × OS ×
#      arch × `SOKO_BUILD_LABEL`）⇒ `release` 无 label = `209c01a9b479e460`、
#      `debug` 无 label = `751385453009d556`。键（`key_parts`）折了它 ⇒ **同版本的两个
#      构建写的条目，对方一条都命中不了**；面板据此报 `stale: true` 并建议 Rebuild。
#
# 判据（真 CLI，两个构建各编同一个项目，看**条目文件名**）：
#   ① 两个二进制必须是**同一版本**（否则测的是"升级"，不是本缺口 ⇒ 环境异常 exit 2）；
#   ② 各自编一遍 ⇒ 各自写出**恰好一条**条目（夹具自检：读侧没坏、路径通）；
#   ③ 两条条目的 `<stamp>` 段**不同** ⇒ 缺口仍在（exit 0）；
#      相同 ⇒ 已修（exit 1，那时两个构建能共用同一份条目）；
#   ④ 收尾：目录里躺着**两条**不同戳的条目，而 `meta.json` 只报得出**一个** compiler
#      —— 这正是用户现场的形状（"整个项目的 meta.json 已经改了"）。
#
# 退出码（docs/gaps/README.md）：0 = 缺口仍在 · 1 = 已修 · 2 = 环境/形状异常
set -u
# 自定位（绝不写死绝对路径；⚠ 2026-10-11 CI 就是这么判红的）。
cd "$(dirname "$0")/../../.." || exit 2
command -v python3 >/dev/null 2>&1 || { echo "需要 python3" >&2; exit 2; }
[ -x scripts/soko ] || { echo "找不到 scripts/soko" >&2; exit 2; }
# ⚠ 本件测的**就是**模块根产物：`gap.py check` 会给所有复现件设
# `SOKONANODA_NO_PROJECT_ARTIFACTS=1`（把复现环境固定在旧语义上）—— 那会把被测
# 功能整个关掉 ✗（同款纪律见 G106 与 `crates/cli/tests/artifacts.rs` 的 `run*`）。
unset SOKONANODA_NO_PROJECT_ARTIFACTS

# ② 的夹具：两个构建。**必须点名二进制、不能走 `scripts/soko`** —— 启动器的
# `repoBuild()` 取 `target/{release,debug}` 里 **mtime 最新**的那个 ⇒ 它可能两次
# 都解析到同一个二进制，夹具就测不到东西 ✗（本件第一次跑就是这么假绿的）。
# ⚠ 顺带记下这个真实风险：**同一次会话里 `scripts/soko` 解析到哪个构建，取决于
# 谁最后被编**，而扩展的 `builtBinaryCandidates()` 是 `debug → release` 的**固定序**
# ⇒ 两条路可以各自选中不同构建（G-112 的现场形状）。
# 两个二进制缺一 ⇒ **先试着按需补建**（debug 那份；release 由调用方负责 ——
# `scripts/soko gate` 与 CI 的 ledger job 都已经建了它）。
# ⚠ **为什么必须"按需补建"而不是直接 exit 2**（2026-10-11 CI 实测判红 ✗）：
# CI 的 `ledger` job 原来**只**建 `--release`（那一步叫 "Build CLI for the gap repros"）
# ⇒ 本件在 CI 上拿到的是 `环境不齐：target/debug/sokonanoda 不存在` ⇒ `exit 2`
# ⇒ `gap.py` 判「环境异常」⇒ **判红** ✗。本件测的**就是**"同一版本的两个构建"，
# 所以它天生需要两份不同 profile 的产物 —— 由它自己保证前提，别指望调用方。
# 超时/失败仍 `exit 2`（环境异常 ⇒ 响亮，不当成"已修" ✓）。
SOKO_A=target/release/sokonanoda
SOKO_B=target/debug/sokonanoda
[ -x "$SOKO_A" ] || {
  echo "环境不齐：$SOKO_A 不存在（cargo build --release -p sokonanoda-cli --locked）" >&2
  exit 2
}
if [ ! -x "$SOKO_B" ]; then
  command -v cargo >/dev/null 2>&1 || {
    echo "环境不齐：$SOKO_B 不存在，且没有 cargo 可以补建" >&2
    exit 2
  }
  echo "· $SOKO_B 不存在 ⇒ 按需补建（debug CLI；cargo 缓存通常已热）"
  timeout 240 cargo build -p sokonanoda-cli --locked >&2 || {
    echo "环境不齐：补建 $SOKO_B 失败或超时" >&2
    exit 2
  }
fi
for bin in "$SOKO_A" "$SOKO_B"; do
  [ -x "$bin" ] || {
    echo "环境不齐：$bin 仍不存在" >&2
    exit 2
  }
done

ver_a=$("$SOKO_A" --version 2>/dev/null | tr -d '\r' | awk '{print $NF}')
ver_b=$("$SOKO_B" --version 2>/dev/null | tr -d '\r' | awk '{print $NF}')
[ -n "$ver_a" ] && [ -n "$ver_b" ] || { echo "取不到版本号" >&2; exit 2; }
if [ "$ver_a" != "$ver_b" ]; then
  echo "形状异常：两个二进制版本不同（$ver_a vs ${ver_b}）⇒ 测的不是本缺口" >&2
  exit 2
fi

root=$(mktemp -d) || exit 2
cache_a=$(mktemp -d) || exit 2
cache_b=$(mktemp -d) || exit 2
trap 'rm -rf "$root" "$cache_a" "$cache_b"' EXIT

# 清单 + 一条 import：与用户现场同构（零配置的单文件 `build` 不写模块根产物）。
printf 'name = "demo"\n' >"$root/sokonanoda.toml"
cat >"$root/Lib.sokonanoda" <<'EOF'
axiom P : Prop
axiom proofP : P
EOF
cat >"$root/Main.sokonanoda" <<'EOF'
import Lib

theorem u : P := proofP
EOF

entries() { ls "$root/.sokonanoda/compiled" 2>/dev/null | grep '\.json$' | sort; }

# ③ 构建 A（release）
SOKONANODA_CACHE_DIR="$cache_a" "$SOKO_A" build "$root/Main.sokonanoda" >/dev/null 2>&1 || {
  echo "① 构建 A 失败（形状异常）" >&2
  exit 2
}
a=$(entries)
n_a=$(printf '%s\n' "$a" | grep -c . )
[ "$n_a" = 1 ] || { echo "① 构建 A 写出了 $n_a 条（期望 1）⇒ 夹具坏了" >&2; exit 2; }
stamp_a=$(printf '%s' "$a" | cut -d+ -f2)

# ③ 构建 B（debug，同一个模块根 —— 用户现场就是"编辑器 + CLI 一处取用"）
SOKONANODA_CACHE_DIR="$cache_b" "$SOKO_B" build "$root/Main.sokonanoda" >/dev/null 2>&1 || {
  echo "① 构建 B 失败（形状异常）" >&2
  exit 2
}
all=$(entries)
n_all=$(printf '%s\n' "$all" | grep -c . )
b=$(printf '%s\n' "$all" | grep -v -F "$a" || true)
n_b=$(printf '%s\n' "$b" | grep -c . )
# ⚠ **先判"已修"再判夹具形状**（反向验证实测踩到）：两个构建一旦同戳（修好的样子），
# B 会**命中** A 的条目 ⇒ 没有新条目（n_b=0）—— 那时该报"已修"（exit 1），
# 而不是拿"夹具坏了"（exit 2）把它盖掉 ✗。
if [ "$n_b" = 0 ]; then
  echo "✓ 已修：构建 B 命中了 A 写的条目（没有新条目）⇒ 两个构建共用同一份产物"
  echo "    A 写的：$a"
  exit 1
fi
[ "$n_b" = 1 ] || { echo "① 构建 B 写出了 $n_b 条新条目（期望 1）⇒ 夹具坏了" >&2; exit 2; }
stamp_b=$(printf '%s' "$b" | cut -d+ -f2)

if [ "$stamp_a" = "$stamp_b" ]; then
  echo "✓ 已修：两个构建的 build_stamp 相同（${stamp_a}）⇒ 能共用同一份条目（共 $n_all 条）"
  exit 1
fi

echo "✗ 缺口仍在：同一版本 $ver_a 的两个构建，build_stamp 不同 ⇒ 键不同 ⇒ 互不命中"
echo "    release 写的：$a   （stamp ${stamp_a}）"
echo "    debug   写的：$b   （stamp ${stamp_b}）"

# ④ **替换**（`update_index` 的既有语义：一个入口只留**一条**条目 ⇒ 后编的
#    把先编的那条删掉）⇒ 两份构建**轮流**擦掉对方的条目。两条断言：
#    条目数仍是 1（不是各留一条）；再跑一次 B ⇒ `hit: 0`（一份都没被复用）。
n_after=$(printf '%s\n' "$all" | grep -c . )
[ "$n_after" = 1 ] || {
  echo "形状变了：B 之后目录里有 $n_after 条（期望 1 —— 一个入口一条）⇒ 夹具/语义变了" >&2
  exit 2
}
SOKONANODA_CACHE_DIR="$cache_b" "$SOKO_B" build --json "$root/Main.sokonanoda" >"$root/again.jsonl" 2>/dev/null || {
  echo "① 构建 B 第二次失败（形状异常）" >&2
  exit 2
}
hit=$(python3 -c "
import json
n = 0
for line in open('$root/again.jsonl'):
    line = line.strip()
    if not line:
        continue
    try:
        e = json.loads(line)
    except Exception:
        continue
    if e.get('type') == 'build.summary':
        n = e.get('hit', 0)
print(n)
")
echo "    B 再编一次 ⇒ hit=${hit}（键里带 stamp ⇒ 自己刚写的能命中、对方写的不能）"
echo "    目录里现在只剩 $n_after 条（B 把 A 的那条删了）："
printf '%s\n' "$all" | sed 's/^/      /'
echo "    而 meta.json 只报得出一个写者（$(python3 -c "
import json
try:
    m = json.load(open('$root/.sokonanoda/meta.json'))
    print(m.get('compiler'), m.get('build_stamp'))
except Exception:
    print('?')
")）—— 用户看到的『整个项目的 meta.json 已经改了』就是这个字段。"
exit 0

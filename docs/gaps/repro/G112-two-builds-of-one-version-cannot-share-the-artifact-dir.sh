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
#
# ⭐ **同源前提**（2026-10-15，第十七次复核实测的**假红**）：
# `<stamp>` 的差异**只有**在两份二进制**同源**时才归因给本缺口 —— 否则会把"环境里
# 混着两个不同时代的构建"读成"缺口仍在"（**exit 0**）✗，而 exit 0 与 exit 2 的语义
# 完全不同（0 = 缺口仍在 · 1 = 已修 · 2 = 环境异常）。
# **实测链条**：本机 `target/debug`（10-14 21:31）早于 `6ae7849b`（10-15 00:28 把
# **引擎源码摘要**折进 `build_stamp`）⇒ 两份戳**必然**不同 ⇒ 复现件 exit 0 ⇒
# `python3 scripts/gap.py check` rc=1（**陈旧假红，不是回归** ✗ —— 归因已复核：
# 隔离 `CARGO_TARGET_DIR` 新建 debug 跑同一份复现件 ⇒「✓ 已修」(exit 1) ✓）。
# ⇒ ③ 用 `stamp_of`/`SOKO_CACHE_KEY_NO_ENGINE_ID` 探针**先核同源**，不同源一律
# **exit 2**（**绝不许报 exit 0** ✗）。
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
# 两份二进制必须是**两次不同的构建**：release 与 debug 永远不逐字节相同（debug 带调试信息
# 与 `debug_assertions` 的代码），相同 ⇒ `target/debug` 是**拷来的**（或软链到 release）
# ⇒ 后面"两个构建共用一份条目"是**自证空转** ✗（本件的绿会变成假绿 ✗）。
cmp -s "$SOKO_A" "$SOKO_B" && {
  echo "形状异常：$SOKO_A 与 $SOKO_B 逐字节相同 ⇒ 不是两次构建 ⇒ 夹具坏了（守卫会空转）" >&2
  exit 2
}

# 夹具内容：清单 + 一条 import（与用户现场同构：零配置的单文件 `build` 不写模块根产物）。
# **探针与主夹具共用这一份**（两边量到的戳必须一致，见 ③ 末尾的交叉断言 ✓）。
mk_fixture() { # $1 = 目录
  printf 'name = "demo"\n' >"$1/sokonanoda.toml"
  cat >"$1/Lib.sokonanoda" <<'EOF'
axiom P : Prop
axiom proofP : P
EOF
  cat >"$1/Main.sokonanoda" <<'EOF'
import Lib

theorem u : P := proofP
EOF
}

# ③ **同源前提**：量一份二进制的 `build_stamp` —— 从它**写出的条目文件名**
#    `<compiler>+<stamp16>+<key>` 里读第二段（`project::cache::entry_stem` 的形状）。
#
# `$2` 给额外环境（本件只用 `SOKO_CACHE_KEY_NO_ENGINE_ID=1`）：
# 那是 `engine_digest::engine_identity()` 的逃生门 ⇒ 戳里**不再折引擎源码摘要**。
# 于是同一份二进制量出**一对**戳：
#   `E` = 默认（折引擎身份）· `N` = `NO_ENGINE_ID=1`（不折）
#   `E != N` ⇒ 这份二进制**折了**引擎身份（`6ae7849b` 之后）✓
#   `E == N` ⇒ 它比那次改动**旧**（逃生门它不认识 ⇒ 引擎身份还没进戳）✗
# ⚠ **为什么用引擎身份而不是 mtime**（复核者给的二选一，这里选"引擎摘要"）：
# 戳的**源码分量就是引擎摘要**，所以"同不同源"是**内容**问题，不是"文件什么时候写下去的"
# 问题 —— mtime 在 `cp`/`tar`/缓存恢复下会说谎（**G-27 的教训**：mtime 不是身份 ✗），
# 而本件的夹具恰恰就是"把别处那份旧二进制搬进来"（搬完 mtime 是**新**的 ⇒ mtime 判据
# 看不见它 ✗）。⚠ 这里**不**碰 `crates/**`（引擎摘要读不出来 ⇒ 只能从**行为**上问它）。
stamp_of() { # $1 = 二进制 · $2 = 额外环境（可为空）⇒ stdout = 16 位戳；失败 = 空
  local bin=$1 extra=${2:-} root cache entry stamp
  root=$(mktemp -d) || return 1
  cache=$(mktemp -d) || { rm -rf "$root"; return 1; }
  mk_fixture "$root"
  if [ -n "$extra" ]; then
    env "$extra" SOKONANODA_CACHE_DIR="$cache" "$bin" build "$root/Main.sokonanoda" >/dev/null 2>&1
  else
    SOKONANODA_CACHE_DIR="$cache" "$bin" build "$root/Main.sokonanoda" >/dev/null 2>&1
  fi
  entry=$(ls "$root/.sokonanoda/compiled" 2>/dev/null | grep '\.json$' | head -1)
  stamp=$(printf '%s' "$entry" | cut -d+ -f2)
  rm -rf "$root" "$cache"
  # 形状必须是**恰好 16 位十六进制**：旧命名 `<key>.json` 没有 `+` ⇒ `cut` 会把整行
  # 还回来 ⇒ 拿它冒充戳就是自欺 ✗（宁可 exit 2 说"量不到"）。
  case "$stamp" in
    [0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f])
      printf '%s' "$stamp" ;;
    *) return 1 ;;
  esac
}

stamp_or_die() { # $1 = 二进制 · $2 = 额外环境 · $3 = 人话里的名字
  # ⚠ 这里**不能** `exit 2`：调用方用 `$( )` 取值 ⇒ 那是在**子 shell** 里 exit ✗
  # （本会话实测踩到：子 shell 一退，`pe_a` 成了空串 ⇒ 前提判据**静默放行** ✗）。
  # ⇒ 函数只**打印人话并返回 1**，由调用方（顶层）`|| exit 2` ✓。
  local s
  s=$(stamp_of "$1" "${2:-}")
  [ -n "$s" ] && { printf '%s' "$s"; return 0; }
  echo "环境异常：量不到 $3 的 build_stamp（build 失败，或条目形状不认识）" >&2
  return 1
}

# ⚠ 变量名与下面主夹具的计数（`n_a`/`n_b` = 条目数）**不许撞** ⇒ 探针一律 `pe_`/`pn_`。
pe_a=$(stamp_or_die "$SOKO_A" "" "release 二进制 $SOKO_A") || exit 2
pe_b=$(stamp_or_die "$SOKO_B" "" "debug 二进制 $SOKO_B") || exit 2
pn_a=$(stamp_or_die "$SOKO_A" SOKO_CACHE_KEY_NO_ENGINE_ID=1 "release 二进制 $SOKO_A") || exit 2
pn_b=$(stamp_or_die "$SOKO_B" SOKO_CACHE_KEY_NO_ENGINE_ID=1 "debug 二进制 $SOKO_B") || exit 2
eng_a=0; [ "$pe_a" != "$pn_a" ] && eng_a=1
eng_b=0; [ "$pe_b" != "$pn_b" ] && eng_b=1

# ⚠ **恰好一份**折引擎身份 ⇒ 环境里混着两个时代的构建 ⇒ 戳**不可比** ✗
# （本件第十七次复核的**假红真身**：release 折了引擎摘要、旧 debug 没折）。
# 两份都折、或两份都没折 ⇒ 继续（都折 = 正常；都没折 = 它们同为旧构建 ⇒ 差异仍可归因，
# 而且将来若撤掉那条逃生门，本件**不会**因此变红 ✓ —— 前提判据要能优雅降级）。
if [ "$eng_a" != "$eng_b" ]; then
  echo "环境异常：两份二进制的**引擎身份**不一致（release $([ "$eng_a" = 1 ] && echo 折了 || echo 没折)·debug $([ "$eng_b" = 1 ] && echo 折了 || echo 没折)）⇒ stamp **不可比** ✗" >&2
  echo "          一份折了引擎源码摘要、另一份没折 ⇒ 其中一份比 6ae7849b（引擎摘要折进 build_stamp）旧" >&2
  echo "          ⇒ 先重建它：cargo build -p sokonanoda-cli --locked（release 用 --release）" >&2
  echo "          ⚠ 本件**不**替你重建已经存在的那份二进制 —— 那会把「环境陈旧」洗成「已修」✗" >&2
  exit 2
fi

root=$(mktemp -d) || exit 2
cache_a=$(mktemp -d) || exit 2
cache_b=$(mktemp -d) || exit 2
trap 'rm -rf "$root" "$cache_a" "$cache_b"' EXIT

mk_fixture "$root"

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

# ③ 的**交叉断言**：探针（另一份临时模块根 + 另一份临时缓存）量到的戳，必须与主夹具
# 条目名里的戳**逐字相同** —— 同一份二进制 + 同一份夹具内容 ⇒ 同一个戳。
# 不同 ⇒ 二进制或环境在本次运行**中途变了** ⇒ 前后读数不可比 ✗（AGENTS.md
# 「探针读数必须带构建身份」；跨调用比读数要钉住环境）。
[ "$stamp_a" = "$pe_a" ] && [ "$stamp_b" = "$pe_b" ] || {
  echo "环境异常：探针量到的戳与主夹具条目里的不一致（$pe_a/$stamp_a · $pe_b/$stamp_b）" >&2
  echo "          ⇒ 二进制或环境在本次运行中途变了 ⇒ 读数不可比 ✗" >&2
  exit 2
}

if [ "$stamp_a" = "$stamp_b" ]; then
  echo "✓ 已修：两个构建的 build_stamp 相同（${stamp_a}）⇒ 能共用同一份条目（共 $n_all 条）"
  exit 1
fi

# 同源前提（③）已成立 ⇒ 现在 stamp 的差异**可以归因**了：
#   · 摘掉引擎分量后两边**仍然不同** ⇒ 差异来自**非引擎**分量（profile / prelude / label）
#     ⇒ 那正是本缺口的机制本身 ⇒ 缺口仍在（exit 0 ✓，**不许**把它洗成环境异常 ✗）；
#   · 摘掉引擎分量后两边**相同**（且两边都折引擎身份）⇒ 差异**只**可能来自引擎源码摘要
#     ⇒ 两份二进制不是同一份源码编出来的 ⇒ 环境异常（exit 2 ✗，**不许**报成缺口仍在 ✗）。
if [ "$eng_a" = 1 ] && [ "$pn_a" = "$pn_b" ]; then
  echo "环境异常：两份二进制的**引擎源码摘要**不同 —— 摘掉引擎分量后戳相同" >&2
  echo "          （${pn_a}），⇒ 差异只可能来自引擎摘要 ⇒ 它们不是同一份源码编出来的" >&2
  echo "          release=${pe_a} · debug=${pe_b} ⇒ 先重建：cargo build -p sokonanoda-cli --locked" >&2
  exit 2
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

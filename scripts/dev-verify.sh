#!/usr/bin/env bash
# **开发验证内环**（值守 2026-10-04 12:00 派单 ✓）：把「改一处 ⇒ 拿到可信结论」
# 从**分钟**压到**秒** ✓。
#
# **为什么需要它**（实测 ✓）：0:00–11:52 的内核线动作里 **48% 在等编译**、
# **25% 在等整本课程重新判定** ⇒ **72% 在等机器** ✗。而仓库里**已经有**正确的做法
# （`crates/cli/tests/project_recompiles_shared_deps.rs` 文件头自己写着「合成夹具能在
# **一秒内**跑完（真课程要 4 分钟）」✗）—— 只是没被当默认路径用 ✓。
#
# **本脚本干什么** ✓：铺一个**合成工程**（4 个库 + 6 个入口 ✓），跑
# 「**改一行 ⇒ 重编闭包**」这条判定路径 ✓，打印**结构计数**（`passes` / `files` ✓，
# 机器无关 ✓），**冷跑 ≤ 2 秒** ✓。
#
# **纪律不换** ✓（值守边界 ✓）：本脚本**只换场景**（合成夹具 ✓），**不换严格度** ✗ ——
# 复现件在册 + 前后翻转 + 反向验证三条一条不省 ✓，只是**日常**在最小夹具上做 ✓；
# `cargo test --workspace` / 整本课程 `tools/check.py` / 语料对拍 **只在收尾验收各跑一次** ✓。
#
# 用法：
#   scripts/dev-verify.sh              # 默认：release 仓库构建（没有就 debug）
#   SOKO_BIN=/path/to/sokonanoda scripts/dev-verify.sh
#   scripts/dev-verify.sh --keep       # 保留临时目录（排查用）
#   scripts/dev-verify.sh --granularity # 追加跑 **G-29/G-68 的结构判据**（合成夹具 · **1.3 秒** ✓）
#                                       #   `passes(1)/passes(6)` 的**边际式** ✗ —— 它才咬得住
#                                       #   「改一行 ⇒ 重编整条闭包」（本脚本自己的 `passes` 只到
#                                       #   入口闭包粒度 ✓，够快但不够尖 ✗）
#
# 退出码：0 = 判据全过 ✓ · 1 = 判据判红 ✗ · 2 = 环境不对（二进制不在等）。
set -u

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
KEEP=0
[ "${1:-}" = "--keep" ] && KEEP=1

# ── ① 解析被测二进制（**打印出来** ✓ —— "0 有两种来源" ✗，先自证真的跑了 ✓）──
BIN="${SOKO_BIN:-}"
if [ -z "$BIN" ]; then
  for cand in "$ROOT/target/release/sokonanoda" "$ROOT/target/debug/sokonanoda"; do
    [ -x "$cand" ] && BIN="$cand" && break
  done
fi
if [ -z "$BIN" ] || [ ! -x "$BIN" ]; then
  echo "环境不对：找不到 sokonanoda 二进制（先 cargo build --release -p sokonanoda-cli）" >&2
  echo "  找过：\$SOKO_BIN · target/release/sokonanoda · target/debug/sokonanoda" >&2
  exit 2
fi
echo "== dev-verify（合成夹具 · 改一行 ⇒ 重编闭包）=="
echo "   被测二进制: $BIN"
echo "   版本: $("$BIN" --version 2>/dev/null | head -1)"

# ── ② 铺夹具（**与 CLI 判据同形** ✓：4 库链 + 6 入口；库放**模块根下** ✓ ——
#      `import A` 找的是根下的 `A.sokonanoda` ✓，放 `lib/` 会 import-not-found ✗）──
DIR="$(mktemp -d "${TMPDIR:-/tmp}/soko-dev-verify-XXXXXX")"
trap '[ "$KEEP" = 1 ] || rm -rf "$DIR"' EXIT
printf '[project]\nname = "devverify"\n' > "$DIR/sokonanoda.toml"
printf 'def A.id (α : Type) (a : α) : α := a\n' > "$DIR/A.sokonanoda"
printf 'import A\n\ndef B.wrap (α : Type) (a : α) : α := A.id α a\n' > "$DIR/B.sokonanoda"
printf 'import B\n\ndef C.twice (α : Type) (a : α) : α := B.wrap α (B.wrap α a)\n' > "$DIR/C.sokonanoda"
printf 'import C\n\ndef D.thrice (α : Type) (a : α) : α := C.twice α (B.wrap α a)\n' > "$DIR/D.sokonanoda"
for i in 0 1 2 3 4 5; do
  printf 'import A\nimport B\nimport C\nimport D\n\ndef e%s_v (α : Type) (a : α) : α := B.wrap α (A.id α a)\n' "$i" \
    > "$DIR/e$i.sokonanoda"
done

# 一次 `build --json` ⇒ 打印 (passes, files, 墙钟秒)。
run_build() {
  local tag="$1" t0 t1 out err
  t0=$(python3 -c 'import time;print(time.time())')
  err="$(cd "$DIR" && SOKO_STAGE_STATS=1 SOKONANODA_CACHE_DIR="$DIR/.cache" \
    timeout 120 "$BIN" build --json "$DIR" 2>&1 >"$DIR/$tag.json")"
  t1=$(python3 -c 'import time;print(time.time())')
  local passes files
  # ⚠ **锚定** ✓：`.*passes=` 是**贪婪**的 ✗ ⇒ 会匹配到行尾的 `doc_passes=0` ✗
  # ⇒ `passes` 读成 0（实测踩过 ✓ —— 那正是"0 有两种来源" ✗）。
  passes="$(printf '%s\n' "$err" | grep '^STAGE_STATS ' | tail -1 | sed -n 's/^STAGE_STATS passes=\([0-9]*\).*/\1/p')"
  files="$(python3 - "$DIR/$tag.json" <<'PY'
import json, sys
n = 0
for line in open(sys.argv[1]):
    try:
        if json.loads(line).get("type") == "build.file":
            n += 1
    except Exception:
        pass
print(n)
PY
)"
  local failed
  failed="$(grep -c '"status":"failed"' "$DIR/$tag.json" 2>/dev/null || true)"
  printf '%s %s %s %s\n' "$passes" "$files" "$(python3 -c "print(f'{$t1-$t0:.2f}')")" "${failed:-0}"
}

cold_passes="" cold_files="" cold_s="" cold_failed=""
read -r cold_passes cold_files cold_s cold_failed <<<"$(run_build cold)"
if [ -z "$cold_passes" ] || [ "$cold_files" = "0" ]; then
  echo "✗ 冷跑没拿到读数（passes='$cold_passes' files='$cold_files'）—— 这**不是**"很快" ✗，是没跑起来 ✓" >&2
  exit 2
fi
echo "   冷跑: passes=$cold_passes files=$cold_files failed=$cold_failed wall=${cold_s}s"

# ── ③ **改一行**（改**一个入口**的一条定义体 ✓ —— 陈述不动 ✓）⇒ 再跑 ──
sed -i.bak 's/def e0_v (α : Type) (a : α) : α := B.wrap α (A.id α a)/def e0_v (α : Type) (a : α) : α := A.id α (B.wrap α a)/' "$DIR/e0.sokonanoda"
if ! grep -q 'A.id α (B.wrap α a)' "$DIR/e0.sokonanoda"; then
  echo "✗ 夹具前提：那一刀没改到东西 ✗（判据会假绿 ✓）" >&2
  exit 2
fi
hot_passes="" hot_files="" hot_s="" hot_failed=""
read -r hot_passes hot_files hot_s hot_failed <<<"$(run_build hot)"
echo "   改一行后: passes=$hot_passes files=$hot_files failed=$hot_failed wall=${hot_s}s"

# ── ④ 判据（**结构计数** ✓，机器无关 ✓；不用绝对毫秒判 ✗）──
rc=0
if [ "$cold_files" != "10" ]; then
  echo "✗ 夹具前提：模块数应为 10（4 库 + 6 入口），实测 $cold_files ✗" >&2
  rc=1
fi
if [ "$cold_failed" != "0" ] || [ "$hot_failed" != "0" ]; then
  echo "✗ 夹具前提：不许有 failed 模块（冷 $cold_failed / 热 ${hot_failed}）✗" >&2
  rc=1
fi
# **这一刀要判的**：改一个入口 ⇒ **不许**把整条闭包按入口数各编一遍 ✗。
# 结构判据 = 热跑的 `passes` 必须**明显小于**冷跑（库层与其余入口该复用 ✓）。
if [ "$hot_passes" -ge "$cold_passes" ]; then
  echo "✗ 改一行后 passes **没有下降**（冷 $cold_passes ⇒ 热 ${hot_passes}）✗ —— 复用没生效 ✓" >&2
  rc=1
fi
if [ "$rc" = "0" ]; then
  echo "✓ dev-verify 全过：冷 ${cold_s}s（passes=${cold_passes}）⇒ 改一行 ${hot_s}s（passes=${hot_passes}）"
  echo "  读法：passes = Σ 闭包模块编译次数（机器无关 ✓）· files = 去重后的模块数 ✓"
fi
[ "$KEEP" = 1 ] && echo "   （--keep：夹具留在 $DIR ✓）"

# ── ⑤ 可选：**G-29/G-68 的结构判据**（合成夹具 ✓ 1.3 秒 ✓）──
# 判据 = `passes(N)` 的**边际式**（`(passes(N)-passes(1))/(N-1) ≤ 1.5` ✓ 理想 1.0 ✓）——
# 「共享依赖不许按入口各编一遍」✗。它**已红** ✗（实测 `passes(1)=13 passes(6)=24 marginal=2.20` ✗），
# 根因写在它自己的失败信息里 ✓：「G-68：缓存键是 per-entry-closure」✓。
# ⚠ 单点验证：只跑**那一个**测试二进制 ✓（不重链整个 CLI 集成测试 ✗ —— 值守 12:00 第 3 条 ✓）。
if [ "${1:-}" = "--granularity" ] || [ "${SOKO_DEV_VERIFY_GRANULARITY:-}" = "1" ]; then
  echo "== 结构判据：改一行 ⇒ 重编整条闭包？（合成夹具 · 1.3 秒 ✓）=="
  if (cd "$ROOT" && timeout 600 cargo test -p sokonanoda-cli --test project_recompiles_shared_deps \
      -- --nocapture 2>&1 | grep -E 'PERF g68|test result'); then :; fi
  echo "   读法：marginal ≤ 1.5 才算过 ✓（理想 1.0 ✓）；实测 2.0–3.0 ✗ ⇒ 缺口仍在 ✓"
fi
exit "$rc"

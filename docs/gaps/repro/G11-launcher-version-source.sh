#!/usr/bin/env bash
# G-11 + G-16 自断言复现：**课程仓的版本源链**与**「无期望版本绝不 exec」守卫**
#
# 退出码约定（全部 repro 脚本一致，见 docs/gaps/README.md）：
#   0 = 缺口仍在，且与台账 `today` 一致 · 1 = 行为变了（回来更新台账）· 2 = 环境不满足
#
# 场景（夹具 = 最小课程仓，形状与 scripts/new-course-repo.sh 的产物一致）：
#   只有 vendored 的 `scripts/soko` + `sokonanoda-version.txt` + `sokonanoda.toml`，
#   **没有 Cargo.toml**；缓存里预置一个 marker 陈旧的假二进制（0.58.0，钉是 0.42.0）。
#
# 修好前的行为（G-11 + G-16，台账 `today`）：
#   ① `version --json` 的 cli/lsp 为 {}（解析不出任何二进制）——课程仓没有版本源；
#   ② 有旧缓存时 `repoVersion()` 返回 undefined，`resolve()` 把 probe 的
#      `cache(STALE…)` 改写成 `cache(unknown repo version)`，守卫只认 `cache(STALE`
#      前缀 ⇒ 陈旧二进制被**静默执行**（现场是 0.55.0 的旧 CLI 报裸 ENOENT）。
#
# 修好后的契约（脚本翻成 exit 1）：
#   ① 版本钉被读出：`version` = 0.42.0、`version_source` = version.txt（不再有 `v?`）；
#   ② marker 对不上就**拒绝执行**：转发命令 exit 3 + 人话（期望版本 / 来源文件 /
#      marker / 下一步），且**绝不出现被 exec 的陈旧二进制的裸 `os error`**；
#   ③ 无版本源的裸目录同样拒绝，且文案指名找过的文件；
#   ④ `SOKONANODA_BIN` 兜底仍然可用（既有解析链不许被删）。

set -u
cd "$(dirname "$0")/../../.." || exit 2
ROOT="$PWD"
command -v node >/dev/null 2>&1 || { echo "需要 node" >&2; exit 2; }
# 本脚本测的是**启动器自己的解析链**：环境里若有显式二进制覆盖，夹具会被短路
# （陈旧缓存不再被拒绝 ⇒ 假报「缺口仍在」）。需要覆盖的 ③ 自己 inline 赋值。
unset SOKONANODA_BIN SOKONANODA_LSP_BIN
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

# ── 夹具：最小课程仓（vendored 启动器 + 版本钉 + 清单，没有 Cargo.toml）────────
mkdir -p "$TMP/repo/scripts" "$TMP/repo/cache"
cp "$ROOT/scripts/soko" "$TMP/repo/scripts/soko"
printf '0.42.0\n' > "$TMP/repo/sokonanoda-version.txt"
printf 'name = "demo-course"\nrequires = "0.42.0"\n' > "$TMP/repo/sokonanoda.toml"
printf 'def x : Prop -> Prop := fun (p : Prop) => p\n' > "$TMP/repo/u.sokonanoda"

# 预置一个陈旧缓存：marker 是 0.58.0，而钉是 0.42.0。
for base in sokonanoda sokonanoda-lsp; do
  printf '#!/bin/sh\necho STALE-BINARY-EXECUTED\n' > "$TMP/repo/cache/$base"
  chmod +x "$TMP/repo/cache/$base"
  printf '0.58.0 nowhere-target\n' > "$TMP/repo/cache/$base.version"
done

# ── ① 版本源链：从 sokonanoda-version.txt 解析出钉住的版本 ────────────────────
echo "== ① 课程仓的版本钉：sokonanoda-version.txt =="
V="$( SOKONANODA_CACHE_DIR="$TMP/repo/cache" SOKONANODA_OFFLINE=1 \
      node "$TMP/repo/scripts/soko" version --json 2>&1 )"
printf '%s\n' "$V" | sed -n '1,8p'
printf '%s' "$V" | python3 -c '
import json,sys
info = json.load(sys.stdin)
ok = info.get("version") == "0.42.0" and info.get("version_source") == "version.txt"
print("   → version=0.42.0 且来源 version.txt（预期）：", ok)
sys.exit(0 if ok else 1)
' || { echo "结论：课程仓仍解析不出自己的版本钉 ⇒ G-11 仍在（夹具或台账待查）。" >&2; exit 0; }
echo

# ── ② 陈旧缓存必须被拒绝执行，且给人话 ──────────────────────────────────────
echo "== ② marker 对不上：转发命令必须拒绝执行（exit 3 + 人话）=="
# 注意：`VAR="$( … )"` 的退出码要被下一行立刻读走（先过一遍 `: $?`）。
OUT="$( SOKONANODA_CACHE_DIR="$TMP/repo/cache" SOKONANODA_OFFLINE=1 \
        node "$TMP/repo/scripts/soko" grade "$TMP/repo/u.sokonanoda" 2>&1 )"
CODE=$?
: "$CODE"
printf '%s\n' "$OUT" | sed -n '1,8p'
echo
echo "   exit=${CODE}（预期 3）"
if [ "$CODE" -ne 3 ]; then
  echo "结论：陈旧缓存没有被拒绝 ⇒ G-16 仍在。"; exit 0
fi
if printf '%s' "$OUT" | grep -q 'STALE-BINARY-EXECUTED'; then
  echo "结论：陈旧缓存二进制**真的被执行了** ⇒ G-16 仍在。" >&2; exit 0
fi
if ! printf '%s' "$OUT" | grep -q '0\.42\.0'; then
  echo "结论：拒绝文案没有说出期望版本 0.42.0 ⇒ 还不算人话。" >&2; exit 0
fi
if ! printf '%s' "$OUT" | grep -q 'sokonanoda-version.txt'; then
  echo "结论：拒绝文案没有指名该改哪个文件（sokonanoda-version.txt）。" >&2; exit 0
fi
# 现场反例：静默执行旧二进制时 stderr 只有 `error: No such file or directory (os error 2)`。
if printf '%s' "$OUT" | grep -q 'No such file or directory'; then
  echo "结论：仍是被 exec 的旧二进制在报裸 ENOENT（没有守卫）。" >&2; exit 0
fi
echo "   → 陈旧缓存被拒绝、文案指名版本与文件：True"
echo

echo "== ②b 无任何版本源的裸目录：同样拒绝并列出找过的文件 =="
mkdir -p "$TMP/bare/scripts"
cp "$ROOT/scripts/soko" "$TMP/bare/scripts/soko"
BARE="$( SOKONANODA_CACHE_DIR="$TMP/bare/cache" SOKONANODA_OFFLINE=1 \
         SOKONANODA_BIN= SOKONANODA_LSP_BIN= \
         node "$TMP/bare/scripts/soko" grade "$TMP/repo/u.sokonanoda" 2>&1 )"
BARE_CODE=$?
echo "   exit=${BARE_CODE}（预期 3）"
printf '%s\n' "$BARE" | sed -n '1,6p'
if [ "$BARE_CODE" -ne 3 ] || ! printf '%s' "$BARE" | grep -q 'Cargo.toml'; then
  echo "结论：无版本源时没有给出「找过哪些文件」的拒绝文案。" >&2; exit 0
fi
echo

# ── ③ 兜底：显式 SOKONANODA_BIN（可用，但绕过版本钉）─────────────────────────
echo "== ③ 兜底：显式 SOKONANODA_BIN 仍然可用 =="
if [ -x "$ROOT/target/debug/sokonanoda" ]; then
  OVER="$( SOKONANODA_BIN="$ROOT/target/debug/sokonanoda" SOKONANODA_CACHE_DIR="$TMP/repo/cache" \
           node "$TMP/repo/scripts/soko" version --json 2>&1 )"
  printf '%s' "$OVER" | python3 -c '
import json,sys
info = json.load(sys.stdin)
ok = info.get("cli", {}).get("source") == "override"
print("   → cli.source=override（预期）：", ok)
sys.exit(0 if ok else 1)
' || { echo "结论：SOKONANODA_BIN 兜底坏了（既有解析链回归）。" >&2; exit 0; }
else
  echo "   跳过：$ROOT/target/debug/sokonanoda 不存在"
fi
echo

echo "结论：G-11/G-16 已修（版本源链读课程仓的钉；无期望版本绝不 exec，改给人话）。"
exit 1

#!/usr/bin/env bash
# 咬住证明 —— `scripts/check-lean4-alignment.py`（洞 `LEAN4-ALIGNMENT-HOOK-CRASHES-ON-ENCODING`）
#
# ## 为什么需要它（而不是台账里一条 rc 比较）
#
# 这条洞的形状是「**守卫崩溃而不是判定**」：旧版在非 UTF-8 提交信息上抛
# `UnicodeDecodeError` ⇒ rc=1。而**修好之后**，同一条坏输入上守卫**也**返回 rc=1
# （那是**判定**：拒绝提交 ✓）。⇒ **只比 rc 的咬住证明分不出这两者** ✗ ——
# 把守卫改回崩溃版，`bite` 照样 rc=1 ⇒ 台账**照样绿** ⇒ **守卫空转** ✗。
# 所以这一条必须断言**形状**：rc=1 **且没有 `Traceback`** **且**有 `❌` 判定行。
#
# ## 判据（`bite` 模式，全部成立才 exit 1）
#
#   ① **钉死的旧 rev**（`BAD_REV`）的守卫，在夹具仓里喂**用户 case 的逐字节形状**
#      （`byte 0xbc in position 79`）⇒ 必须**崩溃**（`Traceback` 在 stderr 里）
#      —— 这一半证明"那条坏输入**真实存在**、旧版真的没在判" ✓；
#   ② **工作区**的守卫，同一条坏输入 ⇒ 必须**判定**：rc=1 · **没有** `Traceback` ·
#      有 `❌` 判定行 —— 这一半才是"**它咬住了**" ✓；
#   ③ **正对照**：同一条非 UTF-8 信息，ASCII 段里有 `Lean4: n/a` ⇒ 必须 **rc=0**
#      —— 防"把什么都判红"冒充咬住 ✗（也钉住"不放宽"的另一面：标记照旧读得出来 ✓）；
#   ④ **同族那条**（`git commit -F` 的 stale 假绿）：`pre-commit` 形状（**不带** `--message-file`）
#      ⇒ 旧版会拿**上一笔**的 `COMMIT_EDITMSG` 判、印出 `✓ 已在**提交信息**找到 Lean 4 对照` ✗；
#      工作区版必须印 `不判定` 且**不出现**那句假绿 ✓。
#   ⑤ **空转闸**：`BAD_REV` 的那份文件与工作区**逐字节相同** ⇒ **exit 3**
#      （绝不冒充咬住 ✗ —— pr1 实测踩过的坑：坏版本原写 `HEAD`，提交后 `HEAD`
#      就是修好的那一版 ⇒ 换版逐字节无效 ⇒ bite rc=0 ⇒ 台账当场报"咬不住" ✗）。
#
# ## 退出码
#
#   `pass` ⇒ 0 = 当前守卫的判据通道自检绿 / 非 0 = 自检红（原样透传）
#   `bite` ⇒ **1 = 咬住了**（①②③④ 全成立）· **0 = 没咬住**（⇒ 台账 C2 判红 ✓）· 3 = 空转/环境错
#
# ⚠ **零写入**：一切都在 `mktemp -d` 里（夹具仓、旧版副本、信息文件）⇒ 不需要"跑完还原" ✓。
# ⚠ **不碰** `crates/**`：夹具里那个 `crates/kernel/dummy.rs` 长在临时目录里 ✓。
set -uo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
GUARD_REL="scripts/check-lean4-alignment.py"
GUARD="$ROOT/$GUARD_REL"
PY=${PYTHON:-python3}

# ⚠ **钉死一个固定旧 rev**（不许写 `HEAD` ✗，见文件头 ④）。挑 `9e4a9c3a` 而不是开工时的
# `3f184f21`：两版**逐字节相同**（实测 sha256 = `6f8a825a…` ✓），而它是 main 深处
# 2026-10-13 的那一笔 ⇒ **land / rebase 之后仍然是祖先**（`3f184f21` 只是当时的 tip，
# 被 rebase 掉就可能不可达 ⇒ 换版会失败 ✗）。
BAD_REV="9e4a9c3a17567c608c6efacd804bd60413e4178a"
BAD_REV_SHORT="9e4a9c3a"

die() { printf 'lean4-alignment-bite: %s\n' "$*" >&2; exit 3; }

[ -f "$GUARD" ] || die "找不到 $GUARD_REL"
command -v "$PY" >/dev/null 2>&1 || die "没有 python3（**判不了 ≠ 咬住** ✗）"
git -C "$ROOT" cat-file -e "$BAD_REV^{commit}" 2>/dev/null \
  || die "钉死的旧 rev \`$BAD_REV_SHORT\` 取不到（**不许拿 HEAD 顶替** ✗）"

TMP=$(mktemp -d "${TMPDIR:-/tmp}/soko-lean4-bite.XXXXXX") || die "mktemp 失败"
trap 'rm -rf "$TMP"' EXIT

# 旧版副本（**唯一**的换版动作；只在临时目录里落盘 ⇒ 工作区零写入 ✓）
git -C "$ROOT" show "$BAD_REV:$GUARD_REL" > "$TMP/old-guard.py" || die "取不出旧版文件"

# ── 空转闸（④）──────────────────────────────────────────────────────────────
old_sha=$(shasum -a 256 "$TMP/old-guard.py" | cut -d' ' -f1)
cur_sha=$(shasum -a 256 "$GUARD" | cut -d' ' -f1)
if [ "$old_sha" = "$cur_sha" ]; then
  printf 'lean4-alignment-bite: ⚠ **空转** —— 钉死旧版（%s）与工作区**逐字节相同**（%s）\n' \
    "$BAD_REV_SHORT" "${cur_sha:0:16}" >&2
  printf '  ⇒ 换版**没有换到任何东西** ⇒ 绝不冒充咬住 ✗（pr1 的坑：坏版本写 `HEAD`）\n' >&2
  exit 3
fi

# ── 夹具仓（真 git 仓 + 真暂存区 ⇒ `staged_files()` 走真路径 ✓）───────────────
# ⚠ **先落一笔带标记的提交**：这样 `.git/COMMIT_EDITMSG` 里就留着**上一笔**的信息
#   ⇒ ④ 那个 stale 假绿的形状才**真实存在**（否则文件不存在 ⇒ 测的是另一条分支 ✗）。
mkfix() {  # $1 = 目录 · $2 = 守卫源文件
  mkdir -p "$1/scripts" "$1/crates/kernel" || return 1
  cp "$2" "$1/$GUARD_REL" || return 1
  printf 'fn main() {}\n' > "$1/crates/kernel/dummy.rs" || return 1
  git -C "$1" init -q . || return 1
  git -C "$1" add -A || return 1
  git -C "$1" -c user.email=bite@local -c user.name=bite \
      commit -q -m 'fix: 按 Lean 4 对齐（App.lean:1176）' || return 1
  # 再动一个**判定路径**文件并暂存 ⇒ `touches_judgment_path()` 命中 ✓
  printf 'fn main() { let _ = 1; }\n' > "$1/crates/kernel/dummy.rs" || return 1
  git -C "$1" add -A || return 1
}

# **用户 case 的逐字节形状**（登记里写的 `byte 0xbc in position 79` ✓）
"$PY" - "$TMP" <<'PY' || die "造信息文件失败"
import sys, pathlib
tmp = pathlib.Path(sys.argv[1])
prefix = b"fix: align one spot in the ledger tool"
nu = prefix + b" " * (79 - len(prefix)) + bytes([0xbc]) + b" trailing ascii\n"
assert nu.index(0xbc) == 79, nu.index(0xbc)
(tmp / "msg-non-utf8.txt").write_bytes(nu)
(tmp / "msg-non-utf8-marker.txt").write_bytes(nu + b"\nLean4: n/a\n")
PY

run_guard() {  # $1 = 夹具目录 · $2 = 信息文件（"" = 不给 ⇒ pre-commit 形状）
  local repo="$1" msg="$2" args=(--check)
  [ -n "$msg" ] && args+=(--message-file "$msg")
  ( cd "$repo" && "$PY" "$repo/$GUARD_REL" "${args[@]}" 2>&1 )
}

mode=${1:-pass}
case "$mode" in
  pass)
    # 正对照：守卫自己的判据通道（已含真 CLI 的 4 组 ⇒ 它就是"守卫绿着"的读数 ✓）
    exec "$PY" "$GUARD" --selftest
    ;;
  bite)
    fail=0

    # ① 旧版 ⇒ 必须**崩溃**
    mkfix "$TMP/old" "$TMP/old-guard.py" || die "夹具仓（旧版）建不起来"
    old_out=$(run_guard "$TMP/old" "$TMP/msg-non-utf8.txt"); old_rc=$?
    if printf '%s' "$old_out" | grep -q "Traceback" && [ "$old_rc" -ne 0 ]; then
      printf '  ✓ ① 钉死旧版（%s）在用户 case 上**崩溃** ⇒ rc=%s + Traceback（= 崩溃而不是判定 ✗）\n' \
        "$BAD_REV_SHORT" "$old_rc"
    else
      printf '  ✗ ① 钉死旧版（%s）**没有崩溃**（rc=%s，Traceback=%s）⇒ 那条坏输入的形状变了 ⇒ 本条证明失效 ✗\n' \
        "$BAD_REV_SHORT" "$old_rc" \
        "$(printf '%s' "$old_out" | grep -c Traceback)" >&2
      fail=1
    fi

    # ② 工作区版 ⇒ 必须**判定**（rc=1 · 无 Traceback · 有 ❌）
    mkfix "$TMP/new" "$GUARD" || die "夹具仓（工作区版）建不起来"
    new_out=$(run_guard "$TMP/new" "$TMP/msg-non-utf8.txt"); new_rc=$?
    if [ "$new_rc" -eq 1 ] && ! printf '%s' "$new_out" | grep -q "Traceback" \
       && printf '%s' "$new_out" | grep -q "❌"; then
      printf '  ✓ ② 工作区版在**同一条**坏输入上**判定**：rc=1 · 无 Traceback · 有 ❌ ⇒ **咬住了** ✓\n'
      printf '       人话：%s\n' "$(printf '%s' "$new_out" | grep -m1 '不是合法 UTF-8' | cut -c1-150)"
    else
      printf '  ✗ ② 工作区版**没咬住**：rc=%s · Traceback=%s · ❌=%s（期望 rc=1 · 无栈 · 有判定 ✗）\n' \
        "$new_rc" "$(printf '%s' "$new_out" | grep -c Traceback)" \
        "$(printf '%s' "$new_out" | grep -c '❌')" >&2
      fail=1
    fi

    # ③ 正对照：非 UTF-8 但 ASCII 段有标记 ⇒ 必须 rc=0（防"什么都判红"冒充咬住 ✗）
    ctl_out=$(run_guard "$TMP/new" "$TMP/msg-non-utf8-marker.txt"); ctl_rc=$?
    if [ "$ctl_rc" -eq 0 ] && ! printf '%s' "$ctl_out" | grep -q "Traceback" \
       && printf '%s' "$ctl_out" | grep -q "✓"; then
      printf '  ✓ ③ 正对照：同一条非 UTF-8 信息 + ASCII 段 `Lean4: n/a` ⇒ rc=0（标记照旧读得出来 ✓）\n'
    else
      printf '  ✗ ③ 正对照**红了**：rc=%s（期望 0）⇒ 守卫把合法的东西也判红 ⇒ 不是咬住 ✗\n' \
        "$ctl_rc" >&2
      fail=1
    fi

    # ④ 同族那条（stale 假绿）：`pre-commit` 形状 = 不带 `--message-file`
    #    旧版 ⇒ 拿上一笔的 `COMMIT_EDITMSG`（带标记）判 ⇒ 印假绿 ✓；工作区版 ⇒ 印「不判定」✓
    old_pre_out=$(run_guard "$TMP/old" ""); old_pre_rc=$?
    new_pre_out=$(run_guard "$TMP/new" ""); new_pre_rc=$?
    if printf '%s' "$old_pre_out" | grep -q '已在\*\*提交信息\*\*找到'; then
      printf '  ✓ ④a 旧版在 `pre-commit` 形状上印**假绿**（`✓ 已在**提交信息**找到 Lean 4 对照`）⇒ 那条同族形状真实存在 ✓\n'
    else
      printf '  ✗ ④a 旧版**没有**印出 stale 假绿（rc=%s）⇒ 夹具没造出那条形状 ⇒ 本条证明失效 ✗\n' \
        "$old_pre_rc" >&2
      fail=1
    fi
    if [ "$new_pre_rc" -eq 0 ] && printf '%s' "$new_pre_out" | grep -q "不判定" \
       && ! printf '%s' "$new_pre_out" | grep -q '已在\*\*提交信息\*\*找到'; then
      printf '  ✓ ④b 工作区版在 `pre-commit` 形状上明说**不判定**、不再印假绿 ✓\n'
    else
      printf '  ✗ ④b 工作区版仍会印假绿（rc=%s · 不判定=%s · 假绿=%s）✗\n' \
        "$new_pre_rc" "$(printf '%s' "$new_pre_out" | grep -c '不判定')" \
        "$(printf '%s' "$new_pre_out" | grep -c '已在\*\*提交信息\*\*找到')" >&2
      fail=1
    fi

    if [ "$fail" -eq 0 ]; then
      printf 'lean4-alignment-bite: **咬住了** ✓（①②③④ 全成立）⇒ bite exit 1（台账 `expect_rc: 1`）\n'
      exit 1
    fi
    printf 'lean4-alignment-bite: **没咬住** ✗ ⇒ exit 0（台账 C2 会因此判红 ✓）\n' >&2
    exit 0
    ;;
  *)
    printf '用法：%s [pass|bite]\n' "$0" >&2
    exit 2
    ;;
esac

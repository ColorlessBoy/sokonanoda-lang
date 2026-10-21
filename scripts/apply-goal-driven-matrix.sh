#!/usr/bin/env bash
# **apply-goal-driven 矩阵**（2026-10-21）：断言 5 条判据，任何一条不符就 exit 1。
# 用法：bash scripts/apply-goal-driven-matrix.sh [<二进制>]
set -u
BIN="${1:-${SOKO:-sokonanoda}}"
DIR="$(cd "$(dirname "$0")/.." && pwd)"
FILE="$DIR/docs/gaps/repro/G-apply-goal-driven.sokonanoda"
out="$("$BIN" query goals --file "$FILE" 2>/dev/null)" || { echo "✗ query goals 失败"; exit 1; }
printf '%s' "$out" | python3 -c '
import json,sys
d=json.load(sys.stdin)["data"]
by={x.get("name"):x for x in d}
def g(name):
    x=by[name]; return [x.get("goal")]+list(x.get("goals") or [])
fail=0
def need(name,want):
    global fail
    got=g(name)
    ok = got==want
    print(("✓" if ok else "✗"), name, "goals=", got)
    if not ok: print("   期望", want); fail=1
def closed(name):
    global fail
    st=by[name].get("status")
    ok = st=="checked"
    print(("✓" if ok else "✗"), name, "status=", st)
    if not ok: fail=1
closed("apply_trans_defaulting"); closed("apply_symm_close"); closed("apply_refl_level")
need("apply_symm_level", ["a = x","x = a"])
need("apply_bad_universe", ["a = x","x = a"])          # 不许出现 Prop
need("apply_unused_type_param", ["P","Sort 1","P"])    # 显式参数就是目标
print("MATRIX", "PASS" if fail==0 else "FAIL")
sys.exit(fail)
'; exit $?

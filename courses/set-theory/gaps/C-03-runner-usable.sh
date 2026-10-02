#!/usr/bin/env bash
# ============================================================================
# C-03 复现（课程侧 · 环境）：**判卷器可用**（版本钉与构建/缓存对齐）
#
# 原始事故（第 679 轮）：版本钉被升到 0.81.0，而仓库构建 0.80.0、缓存 0.73.0
#   => 启动器**拒绝运行**（refusing to run an unverified sokonanoda binary ... STALE）
#   => 课程门禁与一切判卷**跑不了**。
# 结案（第 681 轮）：重建 `cargo build --release -p sokonanoda-cli` => `sokonanoda 0.81.0`
#   => 门禁复核 234/1983/876/0，与重建前逐项一致。
#
# 登记状态：**closed**（OPEN-ITEMS.md C-03）
# 两态（缺一不可）：
#   (a) 自然环境下 `scripts/soko version --json` 必须 **exit 0**，且自述版本 == 版本钉；
#   (a) 正常路径**真的判卷**（`query check` 一个课程文件）必须 **exit 0**，且自述版本 == 版本钉；
#   (b) **注入原始坏状态**（`SOKONANODA_VERSION=0.99.9`：钉到一个**没有任何二进制匹配**的
#       版本 —— 这正是事故的本质：钉 0.81.0 而构建 0.80.0 / 缓存 0.73.0，谁都对不上）
#       必须 **exit != 0**（启动器拒绝运行）。
#       ⚠ 两条实测换来的：① 别拿 0.73.0 当注入值（**缓存里正好有它** ⇒ 合法使用、照样 exit 0）；
#       ② 注入必须配**真判卷调用** —— `version --json` **不需要二进制**，不经过"拒绝运行"那条路
#       （第 682 轮两次假注入都踩在这上面）。
# 期望：exit 0 = 两态都与登记一致 ；exit != 0 = 与登记不一致。
#
# 反向验证（实测，第 682 轮）：把 (b) 的期望反过来（要求注入后仍 exit 0）=> 脚本判红；
#   恢复 => 判绿。也就是说 (b) 不是装饰：它真的会咬。
#
# 用法：bash courses/set-theory/gaps/C-03-runner-usable.sh
# ============================================================================
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
fail=0

PIN="$(grep -m1 '^version' "$ROOT/Cargo.toml" | sed 's/.*"\(.*\)".*/\1/')"
if [ -z "$PIN" ]; then echo "BAD C-03: cannot read the version pin from Cargo.toml"; exit 1; fi

# (a) 正常路径必须可用：真判卷一次
node "$ROOT/scripts/soko" query check --file "$ROOT/courses/set-theory/lib/Order.sokonanoda" > /tmp/c03-grade.json 2>&1
rc=$?
if [ "$rc" -ne 0 ]; then
  echo "BAD C-03(a): the runner refused to run (exit=$rc) -- the original incident is back"
  sed -n '1,3p' /tmp/c03-grade.json
  fail=1
else
  node "$ROOT/scripts/soko" version --json > /tmp/c03-version.json 2>&1
  got="$(python3 -c 'import json; d=json.load(open("/tmp/c03-version.json")); print(d.get("version") or (d.get("data") or {}).get("version") or "")' 2>/dev/null)"
  if [ "$got" = "$PIN" ]; then
    echo "OK  C-03(a): runner usable (grading exit=0), self-reported version $got == pin $PIN"
  else
    echo "BAD C-03(a): runner version '$got' != pin '$PIN'"
    fail=1
  fi
fi

# (b) 注入原始坏状态：钉到没有任何二进制匹配的版本 + 真判卷 => 必须被拒
SOKONANODA_VERSION=0.99.9 node "$ROOT/scripts/soko" query check --file "$ROOT/courses/set-theory/lib/Order.sokonanoda" > /tmp/c03-inject.json 2>&1
rc2=$?
if [ "$rc2" -ne 0 ]; then
  echo "OK  C-03(b): a pin with no matching binary is still refused (exit=$rc2) -- the guard really bites"
else
  echo "BAD C-03(b): a pin with no matching binary was ACCEPTED -- the version guard is gone"
  fail=1
fi

exit $fail

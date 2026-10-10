#!/usr/bin/env bash
# G106 —— 「产物：… 由编译器 X 写入」必须是**条目自己记的写者**，不是"最后一次写入"的戳。
#
# 用户现场（2026-10-10）：「我用 0.87.2 编完，装上 0.87.3 还没 rebuild，infoview 就显示
# 由编译器 0.87.3 写入 —— 这个版本号是假的，失去了这个版本号的意义」。
# 根因：读侧取 `meta.json.compiler`，而它被**任何一次写入**刷成当前编译器
# ⇒ 一条新条目就把另外 247 条的历史改写掉。
#
# 复现形状（与现场同构，但不依赖两份真实构建）：
#   ① 用**当前**编译器编一遍（产物目录建好、条目写全）；
#   ② 把**条目文件名**改成旧写者（`0.0.1+<stamp>+<key>.json`）——"这堆产物是 0.0.1 编的"；
#   ③ 让 `meta.json` 的目录级戳说**当前**版本（= 现场那次"新条目刷戳"之后的形状）。
# 判据（用户可见面，走 `query project`，与 Infoview 同一份数据）：
#   `artifacts.compiler` 必须是 **0.0.1**（照实）、且 `artifacts.stale` 为真
#   （不是这份编译器写的 ⇒ 键不同 ⇒ 不可能命中 ⇒ 面板会提示 Rebuild）。
#
# 退出码（docs/gaps/README.md）：0 = 缺口仍在 · 1 = 已修 · 2 = 环境/形状异常
set -u
cd "$(dirname "$0")/../../.." || exit 2
command -v python3 >/dev/null 2>&1 || { echo "需要 python3" >&2; exit 2; }
SOKO=scripts/soko
[ -x "$SOKO" ] || { echo "找不到 $SOKO" >&2; exit 2; }
# ⚠ 本件测的**就是**模块根产物 ⇒ 必须自己控制这个开关：`gap.py check` 会给所有
# 复现件设 `SOKONANODA_NO_PROJECT_ARTIFACTS=1`（把复现环境固定在旧语义上，
# 免得"产物换地方"让老缺口的结果漂移）—— 那会把被测功能整个关掉 ✗。
# 同一条纪律见 `crates/cli/tests/artifacts.rs` 的 `run*`（都显式 env_remove）✓。
unset SOKONANODA_NO_PROJECT_ARTIFACTS

root=$(mktemp -d) || exit 2
cache=$(mktemp -d) || exit 2
trap 'rm -rf "$root" "$cache"' EXIT

# 清单：与用户现场一致（课程/项目都有 `sokonanoda.toml`；零配置下 CLI 的
# `build <file>` 走"整条闭包一趟"那条路、**不写模块根产物** —— 那是另一件事 ✓）。
printf 'name = "demo"\n' >"$root/sokonanoda.toml"

cat >"$root/Lib.sokonanoda" <<'EOF'
def lib_id : Nat -> Nat := fun (n : Nat) => n
EOF
cat >"$root/Main.sokonanoda" <<'EOF'
import Lib

#check lib_id
EOF

# ① 真编一遍（写产物 + meta.json）
SOKONANODA_CACHE_DIR="$cache" "$SOKO" build "$root/Main.sokonanoda" >/dev/null 2>&1 || {
  echo "① build 失败（形状异常）" >&2
  exit 2
}

meta="$root/.sokonanoda/meta.json"
compiled="$root/.sokonanoda/compiled"
[ -f "$meta" ] || { echo "① 没写出 meta.json（形状异常）" >&2; exit 2; }

python3 - "$meta" "$compiled" <<'PY' || exit 2
import json, os, sys
meta_path, compiled = sys.argv[1], sys.argv[2]
meta = json.load(open(meta_path, encoding="utf-8"))
current = meta.get("compiler")
if not current:
    print("① meta.json 没有 compiler（形状异常）", file=sys.stderr)
    sys.exit(1)
# ② 条目文件名改称"旧写者 0.0.1 编的"：`<compiler>+<stamp>+<key>.json`
renamed = 0
for name in os.listdir(compiled):
    if not name.endswith(".json") or "+" not in name:
        continue
    parts = name[:-5].split("+")
    if len(parts) != 3:
        continue
    old = "0.0.1+" + parts[1] + "+" + parts[2] + ".json"
    os.rename(os.path.join(compiled, name), os.path.join(compiled, old))
    renamed += 1
if renamed == 0:
    print("② 没有条目可改（形状异常：文件名不带写者标记？）", file=sys.stderr)
    sys.exit(1)
# ③ 目录级戳说"当前版本"（= 现场那次写入之后的谎）
meta["compiler"] = current
json.dump(meta, open(meta_path, "w", encoding="utf-8"), ensure_ascii=False)
PY
[ $? -eq 0 ] || exit 2

out=$(SOKONANODA_CACHE_DIR="$cache" "$SOKO" query project --file "$root/Main.sokonanoda" 2>/dev/null) || {
  echo "query project 失败（形状异常）" >&2
  exit 2
}
printf '%s' "$out" >"$root/answer.json"
python3 - "$root/answer.json" <<'PY'
import json, sys
with open(sys.argv[1], encoding="utf-8") as handle:
    answer = json.load(handle)
art = (answer.get("data", {}).get("project") or {}).get("artifacts")
if not isinstance(art, dict):
    print("没有 artifacts 快照（形状异常）", file=sys.stderr)
    raise SystemExit(2)
compiler, stale = art.get("compiler"), art.get("stale")
print("artifacts.compiler = %r · stale = %r · writers = %r"
      % (compiler, stale, art.get("writers")))
if compiler != "0.0.1":
    print("缺口仍在：显示的不是条目自己记的写者（读的是 meta.json 那个会被刷新的戳）")
    raise SystemExit(0)
if stale is not True:
    print("缺口仍在：写者与当前编译器不一致，却没有 stale（面板不会提示 Rebuild）")
    raise SystemExit(0)
print("已修：写者照实（0.0.1）+ stale 为真")
raise SystemExit(1)
PY

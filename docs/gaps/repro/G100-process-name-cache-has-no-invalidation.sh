#!/usr/bin/env bash
# G-100 —— `judge::judge_type_of_constant` 的**进程级名字键缓存**（`type_of_constant_cache`，
# `static Mutex<HashMap<…>>`）**没有任何失效路径**：键 = `options_key(options) + "|" + name`，
# 而 `options_key` 只有 prelude 模式 ⇒ 同一个名字在**另一份环境**里（另一个文件、另一趟
# 编译、用户改了签名）会答出**旧签名** ✗。
#
# 现场（真 CLI，一个进程编两个文件）：`a.sokonanoda` 声明
# `soko_shared_op : (α : Type) -> α -> α -> Prop`，`b.sokonanoda` 声明**同名**但
# `soko_shared_op : Prop -> Prop -> Prop`。两份文本**各自都合法**（对照臂证明 ✓）；
# 但一个进程里先编 a 再编 b 时，b 的记法展开读到的是 **a 的签名** ⇒ 多补一个前导参数
# ⇒ `b` 被判红（`kernel-rejected: 类型不匹配：期望 Sort(0)，实际是 Sort(1)`）✗。
#
# ⚠ **为什么把 `SOKO_JUDGE_INPLACE=off` 写进夹具**：默认档（`On`）下记法目标的签名
# 先在**活环境**（`KnownTable`）里就地取，只有拿不到源级签名（今天只有内建 `Nat.add`）
# 才回落这张缓存 ⇒ 默认档下这条缺口**够不着**。`SOKO_JUDGE_INPLACE=off` 是**既有的、
# 有文档的**档位开关（b1/u1 一族），它让**每一次**签名查询都走这张缓存 ⇒ 缺口的
# 那条路被完整走到 ✓（本件测的是**缓存本身**的失效语义，不是某一档的行为）。
#
# 判据（三条臂）：
#   ① 对照：`b` **单独**编（新进程 + 新缓存目录）⇒ 必须干净（exit 0 / 诊断 0）。
#      不干净 ⇒ **夹具坏了**（exit 2），别把它读成"已修" ✗。
#   ② 缺口臂：一个进程里编 `Lib` / `a` / `b`（`SOKONANODA_BUILD_JOBS=1` ⇒ 串行、
#      顺序确定）⇒ `b` 的 `build.file` 状态。`failed` ⇒ 缺口仍在（exit 0）；
#      `compiled` ⇒ 环境变了会重算（exit 1，已修）。
#   ③ 缓存仍然活着（**不许靠"把缓存删了"过关**）：同一个缓存目录再编一遍 ⇒
#      `build.summary.hit ≥ 1`；命中为 0 ⇒ 判据无效（exit 2）。
#
# 退出码（docs/gaps/README.md）：0 = 缺口仍在 · 1 = 已修 · 2 = 环境/形状异常
set -u
cd "$(dirname "$0")/../../.." || exit 2
command -v python3 >/dev/null 2>&1 || { echo "需要 python3" >&2; exit 2; }
[ -x scripts/soko ] || { echo "找不到 scripts/soko" >&2; exit 2; }
# 本件测的是**进程内**那张判定缓存（不是产物层）⇒ 把产物目录固定关掉，
# 免得模块根的 `.sokonanoda/compiled/` 也参与进来（gap.py 本来就会设它 ✓）。
export SOKONANODA_NO_PROJECT_ARTIFACTS=1

root=$(mktemp -d) || exit 2
cache=$(mktemp -d) || exit 2
control_cache=$(mktemp -d) || exit 2
trap 'rm -rf "$root" "$cache" "$control_cache"' EXIT

cat >"$root/Lib.sokonanoda" <<'EOF'
axiom baseP : Prop
EOF
# a：目标的签名带**前导显式参数** `(α : Type)`（`∈` 的形状）。
cat >"$root/a.sokonanoda" <<'EOF'
import Lib

axiom soko_shared_op : (α : Type) -> α -> α -> Prop

infix:50 " ⊗ " => soko_shared_op

def ta (p q : Prop) : Prop := p ⊗ q
EOF
# b：**同名**，但签名没有前导参数。两条文本各自都合法（对照臂 ✓）。
cat >"$root/b.sokonanoda" <<'EOF'
import Lib

axiom soko_shared_op : Prop -> Prop -> Prop

infix:50 " ⊗ " => soko_shared_op

def tb (p q : Prop) : Prop := p ⊗ q
EOF

# ① 对照臂：b 单独编（新进程 + 新缓存目录）⇒ 必须干净。
SOKONANODA_CACHE_DIR="$control_cache" SOKO_JUDGE_INPLACE=off \
  timeout 300 scripts/soko grade "$root/b.sokonanoda" >"$root/control.jsonl" 2>&1
control_exit=$?
control_diag=$(grep -c '"type":"diagnostic"' "$root/control.jsonl" || true)
if [ "$control_exit" != 0 ] || [ "$control_diag" != 0 ]; then
  echo "环境/夹具异常：对照臂（b 单独编）exit=$control_exit · 诊断=$control_diag" >&2
  echo "⇒ b 的文本本身就不合法，本件测不到缺口（先修夹具）" >&2
  tail -3 "$root/control.jsonl" >&2
  exit 2
fi

# ② 缺口臂：一个进程里编 Lib / a / b（串行 ⇒ 顺序确定：a 先、b 后）。
SOKONANODA_CACHE_DIR="$cache" SOKONANODA_BUILD_JOBS=1 SOKO_JUDGE_INPLACE=off \
  timeout 300 scripts/soko build --json "$root" >"$root/build.jsonl" 2>"$root/build.err"
build_exit=$?
if [ "$build_exit" != 0 ] && [ "$build_exit" != 1 ]; then
  echo "环境异常：缺口臂的 build exit=${build_exit}（不是 0/1）" >&2
  tail -5 "$root/build.err" >&2
  exit 2
fi

status_b=$(python3 - "$root/build.jsonl" "$root/b.sokonanoda" <<'PY'
import json, sys
path, entry = sys.argv[1], sys.argv[2]
for line in open(path, encoding="utf-8", errors="replace"):
    line = line.strip()
    if not line.startswith("{"):
        continue
    try:
        e = json.loads(line)
    except Exception:
        continue
    if e.get("type") == "build.file" and e.get("file") == entry:
        print(e.get("status", "?"))
        break
else:
    print("missing")
PY
)
failed_n=$(python3 - "$root/build.jsonl" <<'PY'
import json, sys
n = None
for line in open(sys.argv[1], encoding="utf-8", errors="replace"):
    line = line.strip()
    if not line.startswith("{"):
        continue
    try:
        e = json.loads(line)
    except Exception:
        continue
    if e.get("type") == "build.summary":
        n = e.get("failed")
print("" if n is None else n)
PY
)
if [ "$status_b" = "missing" ] || [ -z "$failed_n" ]; then
  echo "形状异常：build 事件流里找不到 b 的 build.file / build.summary" >&2
  tail -5 "$root/build.jsonl" >&2
  exit 2
fi

# ③ 缓存仍然活着（不许靠"删缓存"过关）：同一缓存目录再编一遍 ⇒ 必须有命中。
SOKONANODA_CACHE_DIR="$cache" SOKONANODA_BUILD_JOBS=1 SOKO_JUDGE_INPLACE=off \
  timeout 300 scripts/soko build --json "$root" >"$root/again.jsonl" 2>/dev/null
hit_n=$(python3 - "$root/again.jsonl" <<'PY'
import json, sys
n = None
for line in open(sys.argv[1], encoding="utf-8", errors="replace"):
    line = line.strip()
    if not line.startswith("{"):
        continue
    try:
        e = json.loads(line)
    except Exception:
        continue
    if e.get("type") == "build.summary":
        n = e.get("hit")
print("" if n is None else n)
PY
)
if [ -z "$hit_n" ] || [ "$hit_n" = "0" ]; then
  echo "形状异常：第二次 build 一次都没命中（hit=${hit_n:-?}）⇒ 本件分不出\"缓存失效\"与\"缓存没了\"" >&2
  exit 2
fi

if [ "$status_b" = "failed" ]; then
  echo "✗ 缺口仍在：一个进程里先编 a 再编 b ⇒ b 的记法展开用的是 **a 的签名**（进程级名字键缓存没有失效路径）"
  echo "    对照（b 单独编）：exit=0 · 诊断=0 ✓  ⇒ b 的文本是合法的"
  echo "    缺口臂（同进程 a→b）：b 的状态=${status_b}（build.summary.failed=${failed_n}）"
  grep '"status":"failed"' "$root/build.jsonl" | sed 's/^/      /'
  echo "    第二次 build：hit=${hit_n}（缓存本身是活的 ⇒ 上面那条判红不是「缓存没写进去」）"
  exit 0
fi

echo "✓ 已修：同进程 a→b 时 b 的状态=${status_b}（failed=${failed_n}）⇒ 环境变了会重算，不再答旧签名"
echo "    对照（b 单独编）：exit=0 · 诊断=0 ✓"
echo "    第二次 build：hit=${hit_n}（缓存仍在服务同一环境内的重复查询 ✓）"
exit 1

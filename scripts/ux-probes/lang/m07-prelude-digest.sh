#!/usr/bin/env bash
# m07-prelude-digest.sh —— 缓存/模块产物的键**不含内建 prelude 摘要**（主会话 M-07）
# 现象: 改一行 `prelude/**` 后**只重建、不清缓存** ⇒ 旧产物被复用 ⇒ 用户/agent 看到旧 prelude。
#       主会话实测：`target/debug/sokonanoda` 07:30 重建（含 prelude 改动）后，
#       `import lib.Set` + `#print And.left` 仍打印 `def And.left`；删掉 `<模块根>/.sokonanoda/`
#       后同一条命令立刻变 `theorem` ⇒ 是**旧产物被复用**，不是安装路的问题。
# 根因: `crates/front/src/compile/cache.rs::build_stamp()` = FNV-1a(profile, OS, ARCH
#       [, 编译期 `SOKO_BUILD_LABEL`]) —— **不含二进制内容、也不含内建 prelude 的源**；
#       而 prelude 是 `include_str!` 进二进制的 ⇒「同版本/同档位、只改了 prelude」时键完全不变。
# 判据: exit 0 = 键里没有 prelude 摘要（实测 build_stamp == 独立复算的 FNV(profile/OS/ARCH)）
#       exit 1 = 键里出现额外分量（prelude 摘要已折进 ⇒ 已修；也可能是编译期 SOKO_BUILD_LABEL）
# 备注: 本脚本**不修改 prelude**（那超出探针边界）；它验的是"键的构成"这条不变量。
set -uo pipefail
REPO=$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)
# ⚠ **M-07：干净缓存纪律**（2026-10-12 实测咬过一次）—— 内建 prelude / 判定逻辑改了以后
# **只重建不清缓存**时，旧条目会继续答旧行为（同一二进制无 import 时 `#print And.left` = theorem、
# `import lib.Set` 时却 = def）⇒ 每条探针都必须在**新 mktemp 缓存 + 关两处产物**下取读数，
# 否则「缺口仍在」可能是**旧答案**（r04 就这么假红过一次：`apply` 结论校验已修好、探针仍报 exit 0）。
FRESH_CACHE=$(mktemp -d)
export SOKONANODA_CACHE_DIR="$FRESH_CACHE"
export SOKONANODA_NO_PROJECT_ARTIFACTS=1
export SOKONANODA_NO_MODULE_ARTIFACTS=1
trap 'rm -rf "$FRESH_CACHE"' EXIT
# 状态/夹具落 `.cache/`（gitignore）⇒ 跑探针不会把仓库弄脏；干净缓存纪律见
# docs/ux-findings-2026-10-12.md「复核方法」。
W="$REPO/.cache/ux/lang/repro/.work"
mkdir -p "$W"
[ -f "$W/sokonanoda.toml" ] || printf 'name = "ux-lang-work"\n' > "$W/sokonanoda.toml"
[ -e "$W/lib" ] || ln -s "$REPO/courses/set-theory/lib" "$W/lib"

printf 'import lib.Set\n\n#print And.left\n' > "$W/m07.sokonanoda"
timeout 300 "$REPO/scripts/soko" query check --file "$W/m07.sokonanoda" >/dev/null 2>&1
META="$W/.sokonanoda/meta.json"
[ -f "$META" ] || { echo "环境缺失：没有 $META（先跑一次 query check）"; exit 2; }

python3 - "$META" "$REPO" <<'PY'
import json, pathlib, platform, subprocess, sys

meta = json.loads(pathlib.Path(sys.argv[1]).read_text())
repo = pathlib.Path(sys.argv[2])
got = int(meta["build_stamp"], 16)

# 独立复算 build_stamp（**不看源码**，按 cache.rs 文档化的公式重算）
try:
    exe = json.loads(subprocess.run([str(repo / "scripts/soko"), "version", "--json"],
                                    capture_output=True, text=True).stdout)["cli"]["path"]
except Exception:
    exe = ""
profile = "r" if "/release/" in exe else "d"
os_name = {"darwin": "macos", "linux": "linux", "win32": "windows"}.get(sys.platform, sys.platform)
arch = {"arm64": "aarch64", "aarch64": "aarch64", "x86_64": "x86_64", "AMD64": "x86_64"}.get(platform.machine(), platform.machine())
h = 0xCBF29CE484222325
for b in f"{profile}{os_name}/{arch}".encode():
    h ^= b
    h = (h * 0x100000001B3) % (1 << 64)

print(f"二进制          : {exe or '（解析不出）'}")
print(f"产物 build_stamp: {got:#018x}")
print(f"独立复算 FNV    : {h:#018x}   （{profile}{os_name}/{arch}）")
if got == h:
    print("STILL REPRODUCES: 键 = 纯 FNV(profile/OS/ARCH)，**没有** prelude 摘要")
    print("⇒ 同版本/同档位、只改 prelude ⇒ 键不变 ⇒ 旧产物答旧行为（判据：改一行 prelude、只重建、")
    print("  不清缓存 ⇒ 行为必须变；反向：改回去 ⇒ 再变）")
    sys.exit(0)
print("CHANGED: 键里出现了额外分量（prelude 摘要已折进 = 已修；也可能是编译期 SOKO_BUILD_LABEL）")
sys.exit(1)
PY

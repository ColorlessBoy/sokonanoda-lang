#!/usr/bin/env bash
# m07-prelude-digest.sh —— 缓存/模块产物的键**不含内建 prelude 摘要**（主会话 M-07）
# 现象: 改一行 `prelude/**` 后**只重建、不清缓存** ⇒ 旧产物被复用 ⇒ 用户/agent 看到旧 prelude。
#       主会话实测：`target/debug/sokonanoda` 07:30 重建（含 prelude 改动）后，
#       `import lib.Set` + `#print And.left` 仍打印 `def And.left`；删掉 `<模块根>/.sokonanoda/`
#       后同一条命令立刻变 `theorem` ⇒ 是**旧产物被复用**，不是安装路的问题。
# 根因: `crates/front/src/compile/cache.rs::build_stamp()` 原 = FNV-1a(profile, OS, ARCH
#       [, 编译期 `SOKO_BUILD_LABEL`]) —— **不含二进制内容、也不含内建 prelude 的源**；
#       而 prelude 是 `include_str!` 进二进制的 ⇒「同版本/同档位、只改了 prelude」时键完全不变。
# 判据: 按**同一份 build_stamp 公式**独立复算两侧，与产物 `meta.json` 里的 build_stamp 比：
#       * == 旧公式 FNV(profile/OS/ARCH)            ⇒ exit 0 = 缺口仍在（prelude 摘要没进键）；
#       * == 新公式 FNV(profile/OS/ARCH/prelude摘要) ⇒ exit 1 = 已修（内建 prelude 进键 ✓）；
#       * 两个都不是                                  ⇒ exit 2 = **判不出来**（见下）。
# 为什么要两公式（M-07 的探针口径修正，2026-10-12）:
#       旧版只比"== 旧公式"，命中的**反面**一律算"已修" ⇒ 把「prelude 摘要已折进」与
#       「编译期设了 `SOKO_BUILD_LABEL`」混为一谈 ✗（两者都让 stamp ≠ 旧公式）。
#       现在按**新公式**（含 prelude 摘要）与产物逐字节比对：
#         · 摘要已折进 ⇒ 新公式**精确命中** ✓（label 不可能碰巧补上这段字节）
#         · 只有 label ⇒ 两个公式**都不命中** ⇒ 报「判不出来」，并点名 label（不冒充已修 ✓）
#       prelude 摘要的算法与 `builtin_prelude_digest()` 逐字节一致（三段 `include_str!` 源
#       + 两张名字表，表项 `\0` 分隔）；这里是**独立复算**，公式变了必须同轮改这一份 ✓。
#       ⚠ 若 `prelude/**` 改了却**没重建** ⇒ 复算出的新公式与二进制里的旧值必然不等 ⇒
#       也会落进 exit 2（"先重建再跑"）—— 这正是想要的：**不许把陈旧构建读成"已修"** ✓。
# 三侧读数（2026-10-12 实测，同一台机）:
#       · 修前二进制（`cargo build` 于 HEAD 的 cache.rs）⇒
#         stamp == 旧公式 0x751385453009d556 ⇒ **exit 0**（STILL REPRODUCES ✓）
#       · 修后二进制 ⇒ stamp == 新公式 0x7c9e787ad7cb869c ⇒ **exit 1**（CHANGED ✓）
#       · `SOKO_BUILD_LABEL=fix-a5-probe` 构建（修后源码）⇒ stamp 0x016a289dc3c48e43
#         ≠ 两公式 ⇒ **exit 2**（UNDECIDABLE；旧版探针在这里会**假报"已修"** ✗）
# 备注: 本脚本**不修改 prelude**、不重建（那超出探针边界）；它验的是"键的构成"这条不变量。
#       行为那一半（改一行 prelude ⇒ 只重建 ⇒ 行为必须变）由 `.cache/ux/fix-a5/VERDICT.md`
#       的两条读数钉住。
set -uo pipefail
REPO=$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)

# ⚠ **读数必须可归属**（2026-10-12 独立复核发现的两条同族病，一次治齐）：
#
# ① **假绿**：`scripts/soko` 没有可信构建时会拒跑（exit 3）或回落缓存，而本类探针的
#    判定形态是「**grep 不到复现标记 ⇒ exit 1（已修）**」⇒ 拒跑的输出里当然没有那个
#    标记 ⇒ 掉进假绿。复核实测（未建任何二进制时）`r07` 打印 `FIXED: …` 并 exit 1，
#    而下方正是 `refusing to run an unverified sokonanoda binary` ✗。
# ② **判错二进制**：无自建构建时 `scripts/soko` 会解析到**用户缓存**——实测
#    `cli.source = cache(STALE: expected 0.90.0 darwin-arm64, found 0.83.0)`，
#    即**拿 0.83.0 的二进制去判 0.90.0 的仓库**，读数与仓库状态**毫无关系** ✗。
#
# ⇒ 预检两条：**(a) 启动器答得上** · **(b) 它给的是 `repo-build`（不是缓存）**。
#    任一不满足 ⇒ **exit 2 = 环境不满足**（`judge_rc` 把 2 判成 `error`、`gap.py` 判成
#    「环境异常」⇒ 它是**失败**，绝不会被读成"已修" ✓）。
#    ⚠ 这就是 `AGENTS.md`「探针读数必须带构建身份」的**机器化**：读不出来就不许判 ✓。
_SOKO_VER="$("$REPO/scripts/soko" version --json 2>/dev/null)"
if [ -z "$_SOKO_VER" ]; then
  echo "环境不满足：scripts/soko 答不上（无可信构建）⇒ 本件**无法判定**（exit 2）" >&2
  exit 2
fi
if ! printf '%s' "$_SOKO_VER" | grep -q '"source": *"repo-build"'; then
  echo "环境不满足：解析到的**不是仓库构建**（是缓存/扩展）⇒ 读数无法归属本仓库 ⇒ 本件**无法判定**（exit 2）" >&2
  printf '%s' "$_SOKO_VER" | head -c 200 >&2; echo >&2
  exit 2
fi

# ⚠ **M-07：干净缓存纪律**（2026-10-12 实测咬过一次）—— 内建 prelude / 判定逻辑改了以后
# **只重建不清缓存**时，旧条目会继续答旧行为（同一二进制无 import 时 `#print And.left` = theorem、
# `import lib.Set` 时却 = def）⇒ 每条探针都必须在**新 mktemp 缓存 + 关模块产物**下取读数，
# 否则「缺口仍在」可能是**旧答案**（r04 就这么假红过一次：`apply` 结论校验已修好、探针仍报 exit 0）。
FRESH_CACHE=$(mktemp -d)
export SOKONANODA_CACHE_DIR="$FRESH_CACHE"
# ⚠ **不能关 `SOKONANODA_NO_PROJECT_ARTIFACTS`**（M-07 实测踩到）：本探针的读数就是
# `<模块根>/.sokonanoda/meta.json` 的 `build_stamp`，而那份 `meta.json` 是**项目条目**
# 落盘时由 `project::cache::update_index` 写的 ⇒ 关掉那条路，`build` 退出 0 却一个字节
# 都不写 ⇒ 探针把它误报成"环境缺失" ✗。**模块产物**（`.sokonanoda/artifacts/`，库层内核
# 环境）与这份读数无关 ⇒ 只关它（省一轮库层落盘；它的键走同一条 `compile::cache::key`，
# 见 VERDICT.md 的验证 ✓）。
export SOKONANODA_NO_MODULE_ARTIFACTS=1
trap 'rm -rf "$FRESH_CACHE"' EXIT
# 状态/夹具落 `.cache/`（gitignore）⇒ 跑探针不会把仓库弄脏；干净缓存纪律见
# docs/ux-findings-2026-10-12.md「复核方法」。
W="$REPO/.cache/ux/lang/repro/.work"
mkdir -p "$W"
[ -f "$W/sokonanoda.toml" ] || printf 'name = "ux-lang-work"\n' > "$W/sokonanoda.toml"
[ -e "$W/lib" ] || ln -s "$REPO/courses/set-theory/lib" "$W/lib"

printf 'import lib.Set\n\n#print And.left\n' > "$W/m07.sokonanoda"
# ⚠ 用 `build`（**总是走完整编译并写产物**）而不是 `query check`：后者命中缓存时
# **直接回放、不写任何文件** ⇒ 冷缓存下 `meta.json` 不会被建出来（M-07 实测：
# 删掉 `.sokonanoda/` 后 `query check` 返回 0 却没写 meta.json ⇒ 探针报"环境缺失"，
# 把"缓存写没写"误报成"环境缺失" ✗）。产物落在**模块根**（`<模块根>/.sokonanoda/`）。
# ⚠ **不许关 `SOKONANODA_NO_PROJECT_ARTIFACTS`**（实测踩到）：`meta.json` 是**项目**
# 条目落盘时由 `project::cache::update_index` 写的；关掉那条路 ⇒ `build` 退出 0 却
# 一个字节都不写 ⇒ 同上一样的假"环境缺失" ✗。**模块产物**（`.sokonanoda/artifacts/`）
# 与这份读数无关 ⇒ 只关它（省一轮库层落盘）。
timeout 300 env SOKONANODA_NO_MODULE_ARTIFACTS=1 \
  "$REPO/scripts/soko" build "$W/m07.sokonanoda" >/dev/null 2>&1
BUILD_EXIT=$?
META="$W/.sokonanoda/meta.json"
# ⚠ **bash 3.2 的坑**（macOS 自带 3.2.57）：`$VAR` 紧跟一个多字节字符时，
# 3.2.57 会把那个多字节字符的头一字节**并进变量名** ⇒ `set -u` 报
# `META?: unbound variable`（实测：`echo "$X（x）"` 输出 `v=<乱码>x）` ✗）。
# ⇒ 这行**故意**把全角括号与变量分开写（`"…没有 " "$META" " …"`）。
if [ ! -f "$META" ]; then
  echo "环境缺失：产物目录里没有 " "$META" " —— 先跑一次 build 预热产物"
  echo "  诊断：build 退出码 $BUILD_EXIT · 目录内容 = $(ls "$W/.sokonanoda" 2>/dev/null | tr '\n' ' ')"
  exit 2
fi

python3 - "$META" "$REPO" <<'PY'
import json, pathlib, platform, re, subprocess, sys

meta = json.loads(pathlib.Path(sys.argv[1]).read_text())
repo = pathlib.Path(sys.argv[2])
got = int(meta["build_stamp"], 16)

try:
    exe = json.loads(subprocess.run([str(repo / "scripts/soko"), "version", "--json"],
                                    capture_output=True, text=True).stdout)["cli"]["path"]
except Exception:
    exe = ""
profile = "r" if "/release/" in exe else "d"
os_name = {"darwin": "macos", "linux": "linux", "win32": "windows"}.get(sys.platform, sys.platform)
arch = {"arm64": "aarch64", "aarch64": "aarch64", "x86_64": "x86_64", "AMD64": "x86_64"}.get(platform.machine(), platform.machine())
label = "编译期 SOKO_BUILD_LABEL = 已设" if "SOKO_BUILD_LABEL" in __import__("os").environ else "编译期 SOKO_BUILD_LABEL = 未设（本进程环境看不到它是正常的：option_env! 是**构建时**取值）"

FNV_OFFSET = 0xCBF29CE484222325
FNV_PRIME = 0x100000001B3
MASK = (1 << 64) - 1

def mix(h, data: bytes):
    for b in data:
        h = ((h ^ b) * FNV_PRIME) & MASK
    return h

def names_of(table: str):
    """从 prelude.rs 源码里抠出 `pub const <table>: &[&str] = &[ … ];` 的字符串字面量。

    与 Rust 侧同一个来源（那张表就是编进二进制的表 ✓），所以不会与实现漂移。
    """
    text = (repo / "crates/front/src/compile/prelude.rs").read_text()
    m = re.search(rf"pub const {table}: &\[&str\] = &\[(.*?)\];", text, re.S)
    if not m:
        return None
    body = re.sub(r"//[^\n]*", "", m.group(1))
    return re.findall(r'"([^"]*)"', body)

def prelude_digest():
    """`crates/front/src/compile/cache.rs::builtin_prelude_digest()` 的独立复算。"""
    h = FNV_OFFSET
    for f in ("Eq.sokonanoda", "L1.sokonanoda", "Quot.sokonanoda"):
        h = mix(h, (repo / "prelude" / f).read_bytes())
        h = mix(h, b"\0")
    for table in ("PRELUDE_NAMES", "PRELUDE_NEVER_YIELDS"):
        names = names_of(table)
        if names is None:
            return None
        for name in names:
            h = mix(h, name.encode())
            h = mix(h, b"\0")
        h = mix(h, b"\0")
    return h

# 旧公式（M-07 修前）：只有 profile/OS/ARCH。
old = mix(FNV_OFFSET, f"{profile}{os_name}/{arch}".encode())
# 新公式（M-07 修后）：再折内建 prelude 摘要。
digest = prelude_digest()
new = None if digest is None else mix(mix(old, b"/"), digest.to_bytes(8, "little"))

print(f"二进制            : {exe or '（解析不出）'}")
print(f"产物 build_stamp  : {got:#018x}")
print(f"独立复算（旧公式）: {old:#018x}   （{profile}{os_name}/{arch}）")
print(f"独立复算（新公式）: {'（抠不出名字表）' if new is None else f'{new:#018x}'}"
      f"   （旧公式 + 内建 prelude 摘要 {0 if digest is None else digest:#018x}）")
print(f"环境              : {label}")

if got == old:
    print("STILL REPRODUCES: 键 = 纯 FNV(profile/OS/ARCH)，**没有** prelude 摘要")
    print("⇒ 同版本/同档位、只改 prelude ⇒ 键不变 ⇒ 旧产物答旧行为（判据：改一行 prelude、只重建、")
    print("  不清缓存 ⇒ 行为必须变；反向：改回去 ⇒ 再变）")
    sys.exit(0)
if new is not None and got == new:
    print("CHANGED（已修，且**可分辨**）：stamp == 旧公式 + **内建 prelude 摘要** —— 不是 SOKO_BUILD_LABEL")
    print("⇒ 只改一行 `prelude/**` ⇒ stamp 变 ⇒ 缓存键/模块产物键全变 ⇒ 必 miss ✓")
    sys.exit(1)
print("UNDECIDABLE: stamp 既不是旧公式、也不是「旧公式 + 内建 prelude 摘要」")
print("  ⇒ 判不出「摘要已折进」还是「编译期 SOKO_BUILD_LABEL」（两者都会偏离旧公式 ✗）；")
print("     也可能是二进制与工作区源码不同步（改了 prelude/cache.rs 没重建）。先重建再跑本探针。")
sys.exit(2)
PY

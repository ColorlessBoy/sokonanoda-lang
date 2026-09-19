#!/usr/bin/env bash
# G-12 自断言复现：**相对路径入口 + 祖先清单 ⇒ 模块根退化成空路径 ⇒ 所有 import 报找不到**
#
# 退出码约定（全部 repro 脚本一致，见 docs/gaps/README.md）：
#   0 = **G-12 的症状仍在**（相对入口坏 / 裸文件名上溯断在空分量 / 模块根为空）
#   1 = 症状消失（已修复）⇒ 回来更新台账（写 fixed_in）并升级课程
#   2 = 环境/前置缺失（跑不起来，不算结果）
#
# "缺口即测试"：修好后本脚本**必须** exit 1（`scripts/gap.py close` 的判据）；
# 哪天 G-12 回退，它会重新 exit 0，`python3 scripts/gap.py check` 立刻变红。
#
# 现场：课程仓的典型布局——清单在仓库根，库在 lib/，单元在 units/。
# 三个读数，缺一不可（2026-09-18 实测，本脚本同轮补上形状②）：
#   ① **相对路径**入口（AGENTS.md 的文档写法）：`grade units/u.sokonanoda`；
#   ①′ **绝对路径**入口（同一文件同一内容）：必须与①**逐行一致**；
#   ② **裸文件名** + cwd 在清单的**子目录**里：`cd units && grade u.sokonanoda`
#      ——上溯要跨过空目录分量（`Path::new("units").parent() == Some("")`）；
#   ③ `query project` 的模块根：**绝对、非空**，且等于清单所在目录。
#
# 每个相位用**独立的** `SOKONANODA_CACHE_DIR`：本脚本量的是模块根发现（G-12），
# 不是缓存行为。（项目缓存**热命中**时 `query project` 另有一个既有的
# `project:null / reason:"no-path"` 缺陷，在 HEAD 上就能复现，不在 G-12 范围内。）
#
# 根因（读代码定位，非推测）：
#   * `find_manifest`（crates/front/src/project/manifest.rs）从**相对**入口目录向上走，
#     走到 `""` 时 `Path::new("").join("sokonanoda.toml")` = `"sokonanoda.toml"` 命中；
#   * 于是清单路径没有目录分量，`module_root`（同文件）拿 `path.parent()` = `Some("")`
#     ⇒ 模块根 = **空 PathBuf**；
#   * `resolve_module` → `exact_lookup` 的 `fs::read_dir("")` 直接 ENOENT ⇒ 每个 import
#     都 `import-not-found`（`query project` 会把 root 显示成 `''`）。
#   修法：`plan_project` 里把入口与 `root_override` **一起**词法绝对化（cwd 只参与
#   这一步）；`find_manifest` 不把空分量当上溯的一站；`module_root` 把空 parent 当 `.`。

set -u
cd "$(dirname "$0")/../../.." || exit 2
SOKO="$PWD/scripts/soko"
command -v node >/dev/null 2>&1 || { echo "需要 node（scripts/soko 是零依赖 Node 启动器）" >&2; exit 2; }
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

mkdir -p "$TMP/units" "$TMP/lib"
printf 'name = "g12"\n' > "$TMP/sokonanoda.toml"
printf 'axiom P : Prop\n' > "$TMP/lib/Lib.sokonanoda"
cat > "$TMP/units/u.sokonanoda" <<'EOF'
import lib.Lib
theorem t (h : P) : P := h
EOF

# 判卷一次：`grade(<cwd>, <入口参数>, <缓存标签>)`，合并 stderr（human/JSON 两种视图都在)。
grade() {
  ( cd "$1" && SOKONANODA_CACHE_DIR="$TMP/cache-$3" node "$SOKO" grade "$2" 2>&1 )
}

SICK=0   # 1 = 观察到 G-12 症状

echo "== ① 相对路径入口（文档写法，cwd = 清单根）=="
REL="$(grade "$TMP" units/u.sokonanoda rel)"
printf '%s\n' "$REL" | tail -1
if printf '%s' "$REL" | grep -q '"code":"import-not-found"'; then
  SICK=1
  echo "   → import-not-found：**G-12 症状**"
else
  echo "   → import 解析成功（绝对/相对同判）"
fi
echo

echo "== ①′ 绝对路径入口（同一文件同一内容）=="
ABS="$(grade "$TMP" "$TMP/units/u.sokonanoda" abs)"
printf '%s\n' "$ABS" | tail -1
if printf '%s' "$ABS" | grep -q '"type":"decl.checked"'; then
  echo "   → 判卷通过"
else
  SICK=1
  echo "   → **没有 decl.checked**（绝对路径也坏 = 症状）"
fi
if [ "$REL" = "$ABS" ]; then
  echo "   → 两种写法的事件流逐行一致"
else
  SICK=1
  echo "   → **两种写法不一致**（同一文件只因入口写法不同就换结果 = 症状）"
fi
echo

echo "== ② 裸文件名 + cwd 在清单子目录（上溯要跨过空分量）=="
BARE="$(grade "$TMP/units" u.sokonanoda bare)"
printf '%s\n' "$BARE" | tail -1
if printf '%s' "$BARE" | grep -q '"code":"import-not-found"'; then
  SICK=1
  echo "   → import-not-found：**G-12 症状（形状②）**"
else
  echo "   → 祖先清单被找到（零配置退路没有抢跑）"
fi
if [ "$BARE" = "$REL" ]; then
  echo "   → 与①逐行一致"
else
  SICK=1
  echo "   → **与①不一致**（cwd 在参与语义 = 症状）"
fi
echo

echo "== ③ 项目视图里的模块根（必须是 ${TMP} 的真实路径）=="
EXPECT="$(python3 -c 'import os,sys; print(os.path.realpath(sys.argv[1]))' "$TMP")"
ROOT="$( cd "$TMP" && SOKONANODA_CACHE_DIR="$TMP/cache-query" node "$SOKO" query project --file units/u.sokonanoda --compact 2>/dev/null \
  | python3 -c 'import json,sys; data=json.load(sys.stdin)["data"]; p=data["project"]; print("" if p is None else p["root"])' )"
echo "   root='$ROOT'"
if [ -n "$ROOT" ] && [ "$ROOT" = "$EXPECT" ]; then
  echo "   → 模块根 = 清单目录（绝对、非空）"
else
  SICK=1
  echo "   → **模块根为空 / 不等于清单目录**（应有 '$EXPECT' = 症状）"
fi
echo

if [ "$SICK" = 1 ]; then
  echo "结论：G-12 仍在（相对入口坏 / 裸文件名上溯断掉 / 模块根为空）——与台账一致。"
  exit 0
fi
echo "结论：G-12 已修（相对=绝对、裸文件名上溯正常、模块根 = 清单目录）⇒ 台账该写 fixed_in 并升级课程。" >&2
exit 1

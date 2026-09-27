#!/usr/bin/env bash
# G-46 自断言复现：**记法 `×ˢ` 的目标 `Set.prod` 只活在单元⑤ 画布里 ⇒ 库闭包用不了它**
#
# 退出码约定（全部 repro 脚本一致，见 docs/gaps/README.md）：
#   0 = 缺口仍在（下面"修后应有的形状"被打破 ⇒ 缺口回来了）
#   1 = 行为已变（修后形状成立 ⇒ 关账）
#   2 = 环境不满足
#
# 现场（E02 之前，实测）：`infixr:80 " ×ˢ " => Set.prod` 声明在
# `courses/set-theory/lib/Set.sokonanoda`，而 `Set.prod` 的**定义**只在
# `courses/set-theory/units/unit05-pairs-products.sokonanoda` 的画布里
# ⇒ 任何"只 `import` 库"的闭包写 `s ×ˢ t` 都报：
#     `elab-notation-unknown-target`：记法 `×ˢ` 指向的目标 `Set.prod` 不存在
# ⇒ exit 1。这也是 E07「跨模块记法目标」那一半的根因（A10 的症状之一）。
#
# 修后（E02，0.74.0）：`Set.prod` 收进 `courses/set-theory/lib/Prod.sokonanoda`
# ⇒ 目标在库内。本脚本量的是**库闭包自足**：把 `lib/` 拷进临时模块根、写一个
# **只 import 库**、用 `×ˢ` 的画布 —— 判卷必须干净（exit 0 + 有 `decl.checked`
# + 无 diagnostic）。任一条不满足 ⇒ 缺口回来了 ⇒ exit 0。
#
# ⚠ 夹具里**不许**出现任何 `units/`：单元⑤ 里那份定义会把缺口遮住（这正是
# 本缺口"在课程门禁里看不出来"的原因 ✗）。

set -u
cd "$(dirname "$0")/../../.." || exit 2
SOKO="$PWD/scripts/soko"
LIB="$PWD/courses/set-theory/lib"
command -v node >/dev/null 2>&1 || { echo "需要 node" >&2; exit 2; }
[ -d "$LIB" ] || { echo "找不到课程库目录：$LIB" >&2; exit 2; }
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
mkdir -p "$TMP/lib" || exit 2
cp "$LIB"/*.sokonanoda "$TMP/lib/" || { echo "拷课程库失败" >&2; exit 2; }

cat > "$TMP/probe.sokonanoda" <<'EOF'
import lib.Set
import lib.Prod

def sq (A B : Type) (s : Set A) (t : Set B) : Set (Prod A B) :=
  s ×ˢ t
EOF

OUT="$(SOKONANODA_CACHE_DIR="$TMP/cache" node "$SOKO" grade --root "$TMP" "$TMP/probe.sokonanoda" 2>&1)"
CODE=$?

if [ "$CODE" -ne 0 ]; then
  echo "缺口仍在：只 import 库的闭包写 \`×ˢ\` 判卷失败（exit=${CODE}）"
  printf '%s\n' "$OUT" | grep -o '"message":"[^"]*"' | head -3
  exit 0
fi
if ! printf '%s\n' "$OUT" | grep -q '"type":"decl.checked"'; then
  echo "缺口仍在：判卷 exit 0 但**一条 decl.checked 都没有**（不是干净的判卷）"
  exit 0
fi
if printf '%s\n' "$OUT" | grep -q '"type":"diagnostic"'; then
  echo "缺口仍在：判卷 exit 0 但有 diagnostic"
  printf '%s\n' "$OUT" | grep -o '"message":"[^"]*"' | head -3
  exit 0
fi

echo "行为已变：只 import 库就能写 \`s ×ˢ t\`（目标 Set.prod 在 lib/Prod 里）⇒ G-46 关账"
exit 1

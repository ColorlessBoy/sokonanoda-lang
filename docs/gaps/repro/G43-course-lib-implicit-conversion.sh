#!/usr/bin/env bash
# G-43：**跨模块**（`import`）之后，记法路径就解不出前导隐式参数 ✗。
#
# 这是 **B2（课程库隐式化）** 的最后一块：`lib/Image` 的 `Set.mem_image` /
# `Set.mem_preimage` 两条定理卡在这里，而 `scripts/notation-lint.py:89` **要求**
# 写 `f '' A`（点名 `Set.image ` 会被判红 ✗）⇒ 没有"改成点名"的退路 ✓。
#
# **最小复现（两个文件，别的地方都不需要）**：
#   A.sokonanoda：`def Set.image {α β} (f : α → β) (A : Set α) : Set β` + `infix '' `
#   B.sokonanoda：`import A` + `theorem … : y ∈ f '' A ↔ …`
# ⇒ **红** ✗：`类型不匹配：期望 Sort(1)，实际是 Pi ( : $4), $4`
# **把同样内容放进一个文件** ⇒ **绿** ✓ ⇒ 触发条件是 **import** ✓（不是记法本身 ✗）。
#
# 临时探针（`SOKO_DEBUG_IP`，已撤 ✓）显示：钩子**根本没被调用**到 `Set.image` 上
# （只看到 `head=Set`）⇒ 跨模块时记法的展开路径**没走**那条隐式插入钩子 ✓。
#
# 期望：缺口**仍在**时 exit 0；修好后 exit 1。
set -uo pipefail
cd "$(dirname "$0")/../../.." || exit 2
BIN=./target/release/sokonanoda
[ -x "$BIN" ] || { echo "G-43：先跑 cargo build --release -p sokonanoda-cli" >&2; exit 2; }

DIR=$(mktemp -d)
trap 'rm -rf "$DIR"' EXIT
cat > "$DIR/A.sokonanoda" <<'EOF'
def Set (α : Type) : Type := α -> Prop
namespace Set
def image {α β : Type} (f : α -> β) (A : Set α) : Set β := fun (y : β) => True
end Set
infix:60 " '' " => Set.image
EOF
cat > "$DIR/B.sokonanoda" <<'EOF'
import A

theorem t {α β : Type} (f : α → β) (A : Set α) (y : β) :
    y ∈ f '' A ↔ (y ∈ f '' A) :=
    Iff.intro (fun (h : y ∈ f '' A) => h) (fun (h : y ∈ f '' A) => h)
EOF
# 让 `∈` 也可用：放进 A（`Set.mem` + infix）✓
python3 - "$DIR/A.sokonanoda" <<'PY'
import sys
p=sys.argv[1]; s=open(p).read()
s=s.replace("end Set\n", "def mem {α : Type} (a : α) (A : Set α) : Prop := A a\nend Set\ninfix:50 \" ∈ \" => Set.mem\n")
open(p,'w').write(s)
PY

out=$("$BIN" grade --root "$DIR" "$DIR/B.sokonanoda" 2>&1)
if printf '%s' "$out" | grep -q '"code"'; then
  echo "G-43 仍在：跨模块（import）之后记法解不出前导隐式参数（B2 的最后一块）" >&2
  printf '%s' "$out" | grep -oE '"message":"[^"]{0,84}' | head -2 >&2
  exit 0
fi
echo "G-43 已修：跨模块记法也能解出隐式参数 ✓" >&2
exit 1

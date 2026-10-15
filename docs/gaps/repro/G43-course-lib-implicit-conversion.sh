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
# **优先 `scripts/soko`**（受管环境里 `CARGO_TARGET_DIR` 指向别处 ⇒ 下面那些
# `target/{release,debug}` 路径永远不存在 ⇒ 本件一律 exit 2 = "环境异常"，
# 而 `gap.py check`（`soko gate`/CI 的一步）会因此判红 ✗ —— 2026-10-12 独立复核发现）。
# ⚠ 先探测它**真的能答**：无可信构建时 `scripts/soko` **exit 3 拒跑**，不探测就会
# 掉进"grep 不到复现标记 ⇒ 已修"的**假绿** ✗。
# ⚠⚠ **2026-10-15 换探针**（洞 `GATE-FALSE-GREEN-11-MORE` 的 G-43 专项）：旧探针
# `scripts/soko version` **咬不住** ✗ —— `version` 是**启动器自己**答的（**不转发给 CLI** ✗），
# 所以"没有任何可执行的 CLI"时它照样 **rc=0**（实测两态：`SOKONANODA_BIN=<零输出空转量具>`
# ⇒ rc=0 / 零输出；`SOKONANODA_BIN=/nonexistent/sokonanoda` ⇒ 也 rc=0）⇒ 探针通过 ⇒ 选
# `scripts/soko` ⇒ 后面 `grade` 零输出 ⇒ 旧判据报「已修」= **假绿** ✗。
# 新探针 `--version` **会被转发给 CLI**（`scripts/soko <其余子命令>` 原样转发 ✓）⇒ 只有真的
# exec 到二进制、且它**答了话**（输出非空 ✓）才算过；空转（零输出）/ 拒跑 / 无二进制 ⇒
# 探针不过 ⇒ 落到下面的 `exit 2` ✓。**为什么它咬得住**：判据从"启动器自述"换成了
# "被 exec 的二进制吐出的字节"——前者与被量对象无关，后者可归属 ✓。
if [ -x scripts/soko ] && [ -n "$(scripts/soko --version 2>/dev/null)" ]; then BIN=scripts/soko
else BIN=./target/release/sokonanoda; fi
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

# ⭐ **零观测闸门**（2026-10-15，洞 `GATE-FALSE-GREEN-11-MORE`）：判据是"输出里有没有
# `"code"`"，而「已修」那一档要的正是"一条诊断都没有" ⇒ **量具空转（零事件 / 零输出）
# 时读数与「已修」逐字相同** ✗ = **假绿**。⇒ 先要求这次判卷真的观测到**本夹具的声明
# 事件**（可归属 ✓ —— B 里只有 `t`）或**一条诊断**；两样都没有 ⇒ **exit 2 + 人话**。
if ! printf '%s' "$out" | grep -qE '"name":"t","type"|"type":"diagnostic"'; then
  echo "G-43：观测不到判卷事件（既没有 t 的声明事件，也没有诊断）⇒ 环境/形状异常（exit 2），不是「已修」✗" >&2
  echo "      · 这条判据量的就是诊断码；一条事件都没有时 codes 恒为空 ⇒ 「已修」没有信息量 ✗" >&2
  echo "      · 常见成因：scripts/soko 拒跑（exit 3 / 无版本匹配构建）/ 量具空转 / 夹具或模块根不对" >&2
  printf '%s\n' "$out" | grep -v '^[[:space:]]*$' | head -3 | sed 's/^/        → /' >&2
  exit 2
fi

if printf '%s' "$out" | grep -q '"code"'; then
  echo "G-43 仍在：跨模块（import）之后记法解不出前导隐式参数（B2 的最后一块）" >&2
  printf '%s' "$out" | grep -oE '"message":"[^"]{0,84}' | head -2 >&2
  exit 0
fi
echo "G-43 已修：跨模块记法也能解出隐式参数 ✓" >&2
exit 1

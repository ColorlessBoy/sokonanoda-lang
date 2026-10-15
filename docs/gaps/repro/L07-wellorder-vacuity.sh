#!/usr/bin/env bash
# L-07 复现/守卫件：`lib/Order` 的**非严格良序曾经空真** ——
# 修前：`IsWellOrder` 配的是「极小」版 `HasMin`（`∃ m, P m ∧ ∀ y, P y → ¬ r y m`），
#   而 `IsWellOrder` 用的是**非严格**线性序（含自反）⇒ 取 `y := m` 即矛盾 ⇒
#   那个组合**只在空论域上可满足** ✗ ⇒ 单元⑬⑭ 的良序定理**全部空真** ✗。
# 修后：非严格侧一律用「**最小**」版 `HasLeast`（`∀ y, P y → r m y`）✓，
#   并另立 `IsStrictWellOrder`（严格序 + 良基）配「极小」版 ✓。
#
# 判据（**跑一次真判卷**，答的是审计清单的**第 2 问「有没有居民」**）：
#   在**非空**论域（`Nat`）上造一个 `HasLeast` 的实例 —— 全关系 + 全谓词，
#   见证取 `0` ✓。**若有人把 `HasLeast` 改回「极小」版**，这条证明立刻判红 ⇒ 守卫咬住 ✓。
# ⚠ 探针里用**点名写法** —— 记法是**文件内作用域**的。
#
# 退出码约定（见 docs/gaps/README.md）：0 = 缺口仍在 · 1 = 已修 · 2 = 环境不满足
set -u
ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$ROOT" || exit 2
PROBE="courses/set-theory/units/zzprobe-l07.sokonanoda"

cat > "$PROBE" <<'SOKO'
import lib.Order

-- 非空论域（Nat）上的 HasLeast 居民：全关系 + 全谓词，见证 0 ✓
theorem l07_resident :
    HasLeast Nat (fun (a b : Nat) => True) (fun (x : Nat) => True) :=
  Exists.intro Nat (fun (x : Nat) => True ∧ (forall (y : Nat), True -> True)) 0
    (And.intro True (forall (y : Nat), True -> True) True.intro
      (fun (y : Nat) (hy : True) => True.intro))
SOKO

out="$(node scripts/soko query check --file "$ROOT/$PROBE" 2>&1)"
rc=$?
# ⚠ 探针文件先删（提到闸门**之前** ✓）—— 否则闸门 `exit 2` 时会把探针残留在 `courses/`里 ✗
rm -f "$PROBE"
# ⭐ **零观测闸门**（2026-10-15，洞 `GATE-FALSE-GREEN-11-MORE` 的横向扫）：本件的判据
# **只有 `query check` 的退出码**，而**量具空转（零输出 / 拒跑）时它恒为 0** ⇒ 逐字落进
# 「已修」= **假绿** ✗。⇒ 先要求这次查询真的**答出了本夹具**（可归属 ✓：
# `decl_checked ≥ 1` 或 `failed` 非空 —— 两者都只可能来自真的跑了夹具 ✓）；观测不到 ⇒
# **exit 2 + 人话**，不拿空观测当结论 ✗。
if ! printf '%s' "$out" | grep -qE '"decl_checked": [1-9]|"failed": \[$'; then
  echo "L-07：观测不到 query check 的答案（既没有判过的声明，也没有失败项）⇒ 环境/形状异常（exit 2），不是判据结论 ✗" >&2
  echo "      · 常见成因：scripts/soko 拒跑（exit 3 / 无版本匹配构建）/ 量具空转 / 夹具或模块根不对" >&2
  printf '%s\n' "$out" | grep -v '^[[:space:]]*$' | head -3 | sed 's/^/        → /' >&2
  exit 2
fi

if [ "$rc" -eq 0 ]; then
  echo "L-07 已修：非空间域上有 HasLeast 居民（『最小』版语义）"
  exit 1
fi
echo "L-07 回退：非空居民造不出来（HasLeast 又配回『极小』版？）"
exit 0

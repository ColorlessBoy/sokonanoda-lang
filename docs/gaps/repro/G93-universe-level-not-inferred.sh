#!/usr/bin/env bash
# **G-93**：**显式宇宙多态的常量，宇宙层不参与推断 —— 一律默认 `0`** ✗
# （最刺眼的受害者 = `Eq.refl α a` ✗ —— 它是课程里最常见的证明项 ✓）
#
# 退出码（`docs/gaps/README.md` 的约定 ✓）：
#   0 = 缺口仍在（`myax α a` 判红 ✗）
#   1 = 行为变了（判绿 ⇒ 修好了 ⇒ 回来更新台账 ✓）
#   2 = 环境不对（**含对照相位**：显式 `.{1}` 必须判绿 ✓ —— 它红了说明夹具坏了 ✗）
#
# ## 它是什么（2026-10-04 第 18 棒实测定位 ✓）
#
# 前置库自己就写着这条形状（`crates/front/src/compile/prelude.rs:177` ✓）：
#
#     axiom Eq.refl {u} : {α : Sort u} -> (a : α) -> Eq.{u} α a a
#
# ⇒ `Eq.refl α a`（`α : Type`）要求 **`u := 1`** ✓。实测：**没有推断，直接当 `0`** ✗
# ⇒ 内核报「**期望 `Sort(0)`，实际是 `Sort(1)`**」✗。
#
# ## 两个**独立**触发（2026-10-04 第 19 棒实测 ✓ —— 这一对是决定性的 ✓）
#
# | 声明（值位都是 `sorry` ⇒ **值完全不参与** ✗） | 结果 |
# |---|---|
# | `: Eq α a a := sorry`（**指向式** ✗） | **红** ✗ ⇒ **病在类型侧** ✓ |
# | `: a = a := sorry`（**记法** ✓） | **绿** ✓ ⇒ **记法那条路会推断层** ✓ |
#
# ⇒ 触发面有**两个、且互相独立** ✓：
#   ① **类型侧**：指向式 `Eq α a a` ✗（记法 `a = a` 不触发 ✓）；
#   ② **值侧**：`Eq.refl α a` 当项 ✗（类型写成记法也照样红 ✗ —— 相位 C ✓）。
#
# ## 实测矩阵（release · **改前的基线二进制与当轮二进制都复现** ✓ ⇒ **既有** ✓ · 确定性 ✓）
#
# | 声明 | `grade` |
# |---|---|
# | `{α : Prop}`（= `Sort 0`）⇒ 默认值**碰巧对** ✓ | **绿** ✓ |
# | `{α : Type}`（= `Sort 1`） | **红** ✗「期望 `Sort(0)`，实际是 `Sort(1)`」 |
# | `{α : Type 1}`（= `Sort 2`） | **红** ✗「期望 `Sort(0)`，实际是 `Sort(2)`」 |
# | 两个宇宙位 `{u v}` | **红** ✗ |
# | **`myax.{1} α a`（显式写层）** | **绿** ✓ ⇒ **就是层没推断** ✓ |
# | `{α : Type}` **单态**写法 | **绿** ✓ |
#
# ⇒ **病灶 = 显式 `{u}` 位的常量，实参类型里的层不回流到 `u`** ✗（一律取 `0` ✓）。
# ⚠ 前置库自己**到处显式写层**（`Eq.refl.{u} α a` ✓ · `Eq.subst.{u}` ✓ —— `prelude.rs:255/257` ✓）
# ⇒ **这条限制作者是知道的、绕着走的** ✗，而**课程作者不知道** ✗。
#
# ## 为什么用户会撞上（**症状面** ✓）
#
# * `grade` 对 `theorem t (α : Type) (a : α) : a = a := Eq.refl α a` **判红** ✗，
#   而 `:= rfl` ✓ / `:= by rfl` ✓ **判绿** ⇒ **同一个命题、换个写法一个绿一个红** ✗✗。
# * ⚠ 另有一条**路径分叉**（一并记 ✓）：`build` 对同一条**判绿** ✓ 而 `grade` **判红** ✗；
#   且 `build` 在**带 `import`** 时也判红 ✗（不带则绿 ✓）⇒ `build` / `grade` / 单文件 / 闭包
#   **四个格子对不齐** ✗ —— 正是 G-72 那一族「两条路径的形态不同一」✗。
#
# ## 出路（写给下一位 ✓）
#
# ① **真修** = 让显式 `{u}` 位的层从实参类型回流 ✓。
#    ⚠ **已经有先例可抄** ✓（第 19 棒查证 ✓）：
#      · `infer_recursor_universes`（`elab.rs:6264` ✓）—— **就是**「拿期望类型的宇宙
#        把裸常量默认成 0 的层补回来」✓，G-58/G-59 用它修好了递归子 ✓ ⇒ 同形 ✓；
#      · `universe_level_text_of_operands`（`elab.rs:1652` ✓）—— **记法那条路**
#        已经在推断层了 ✓（相位 E 绿 ✓ 就是它 ✓）⇒ 机制现成 ✓。
#    落点：`try_implicit_application`（`elab.rs:3473` ✓ —— **头和整条实参脊都在手上** ✓）
#    或 App 臂加一次后置补层 ✓。**保守闸门**：只在「层是**默认**出来的」时动手 ✓
#    ⇒ 显式 `.{n}` 与今天**逐字节不变** ✓（照 `infer_recursor_universes` 的写法 ✓）；
#    ⚠ 与排队里的 **IA-4 · U1/U2 宇宙层**同源 ✗ —— 设计 `docs/design/metavar-engine.md` §2.9 ✓，
#    它把「**G-63 复现件**」列为判据 ✓ ⇒ **G-93 应当一并列为 U1/U2 的判据** ✓；
# ② **逃生门**（**不是结案** ✗）= 写 `Eq.refl.{1} α a` ✓（实测绿 ✓）。
set -u
ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$ROOT" || exit 2

BIN="${SOKO_BIN:-}"
if [ -z "$BIN" ]; then
# **优先 `scripts/soko`**（受管环境里 `CARGO_TARGET_DIR` 指向别处 ⇒ 下面那些
# `target/{release,debug}` 路径永远不存在 ⇒ 本件一律 exit 2 = "环境异常"，
# 而 `gap.py check`（`soko gate`/CI 的一步）会因此判红 ✗ —— 2026-10-12 独立复核发现）。
# ⚠ 先探测它**真的能答**：无可信构建时 `scripts/soko` **exit 3 拒跑**，不探测就会
# 掉进"grep 不到复现标记 ⇒ 已修"的**假绿** ✗。
  if [ -x "$ROOT/scripts/soko" ] && (cd "$ROOT" && scripts/soko version >/dev/null 2>&1); then
    BIN="$ROOT/scripts/soko"
  fi
  for cand in "$ROOT/target/release/sokonanoda" "$ROOT/target/debug/sokonanoda"; do
    [ -n "$BIN" ] || { [ -x "$cand" ] && BIN="$cand" && break; }
  done
fi
[ -n "$BIN" ] && [ -x "$BIN" ] || { echo "环境不对：找不到 sokonanoda 二进制" >&2; exit 2; }

DIR="$(mktemp -d "${TMPDIR:-/tmp}/soko-g93-XXXXXX")"
trap 'rm -rf "$DIR"' EXIT
printf '[project]\nname = "g93"\n' > "$DIR/sokonanoda.toml"

judge() { # $1 = 文件 ⇒ 打印「绿」/「红:<诊断>」/「空观测」
  rm -rf "$DIR/cache"
  SOKONANODA_NO_PROJECT_ARTIFACTS=1 SOKONANODA_CACHE_DIR="$DIR/cache" \
    timeout 120 "$BIN" grade --json "$1" 2>/dev/null | python3 -c '
import json, sys
bad = []
events = 0
for line in sys.stdin:
    try:
        e = json.loads(line)
    except Exception:
        continue
    events += 1
    if e.get("type") == "diagnostic":
        bad.append(str(e.get("code")) + ": " + str(e.get("message")))
# ⭐ **零观测闸门**（2026-10-15，洞 `GATE-FALSE-GREEN-11-MORE`）：判据是"有没有诊断"，
# 而**量具空转（零事件 / 零输出）时 bad 必然为空** ⇒ 逐字打印「绿」⇒ 五档全绿 ⇒
# 「行为变了 ⇒ 已修」= **假绿** ✗。⇒ 零事件一律打印 `空观测`（**不是**「绿」✗），
# 由下面的控制相位把它判成 exit 2 ✓。同族形状：G69 的「无事件行 ⇒ exit 2」。
if events == 0:
    print("空观测")
else:
    print("绿" if not bad else "红:" + " / ".join(bad[:2]))
'
}

# **相位 A（缺口）**：显式 `{u}` + 实参 `Type` ⇒ 现在判红 ✗
printf 'axiom myax {u} : {α : Sort u} -> (a : α) -> α\ndef d (α : Type) (a : α) : α := myax α a\n' \
  > "$DIR/Implicit.sokonanoda"
# **相位 B（对照）**：同一件事、显式写层 ⇒ 必须判绿 ✓
printf 'axiom myax {u} : {α : Sort u} -> (a : α) -> α\ndef d (α : Type) (a : α) : α := myax.{1} α a\n' \
  > "$DIR/Explicit.sokonanoda"
# **相位 C（用户症状面）**：`Eq.refl` 当项用 ⇒ 必须判绿 ✓（现在红 ✗）
printf 'theorem t (α : Type) (a : α) : a = a := Eq.refl α a\n' \
  > "$DIR/UserVisible.sokonanoda"
# **相位 D（本轮新增 · 决定性 ✓）**：**指向式**类型 + 值位 `sorry`（值完全不参与 ✗）
# ⇒ 判红 ✗ ⇒ 证明**病在类型侧** ✓（不是"值 elaborate 错了"✗）。
printf 'theorem t (α : Type) (a : α) : Eq α a a := sorry\n' \
  > "$DIR/PointedType.sokonanoda"
# **相位 E（相位 D 的对照 ✓）**：同一命题写**记法** ⇒ 必须判绿 ✓
# ⇒ **记法那条路已经会推断层** ✓（`universe_level_text_of_operands` ✓）——
# 差的只是**普通常量应用**那条路 ✗。
printf 'theorem t (α : Type) (a : α) : a = a := sorry\n' \
  > "$DIR/NotationType.sokonanoda"

a="$(judge "$DIR/Implicit.sokonanoda")"
b="$(judge "$DIR/Explicit.sokonanoda")"
c="$(judge "$DIR/UserVisible.sokonanoda")"
d="$(judge "$DIR/PointedType.sokonanoda")"
e="$(judge "$DIR/NotationType.sokonanoda")"

echo "G-93 读数："
echo "  相位 A（显式 {u} · 实参 Type）    : $a"
echo "  相位 B（同一件事 · 显式 .{1} 对照）: $b"
echo "  相位 C（用户症状面 · Eq.refl 当项）: $c"
echo "  相位 D（指向式类型 · 值位 sorry）  : $d"
echo "  相位 E（记法类型 · 值位 sorry 对照）: $e"

# ⭐ **零观测闸门**（2026-10-15，洞 `GATE-FALSE-GREEN-11-MORE`）：五档读数全部来自
# `judge()` 的"有没有诊断"，量具空转时它恒为空 ⇒ 旧代码把五档都印成「绿」⇒ 逐字落进
# 「行为变了 ⇒ 已修」= **假绿** ✗。⇒ 任何一档是 `空观测` ⇒ **exit 2 + 人话**，
# 绝不拿"空观测"当判据结论 ✗（下面的 B/E 对照仍然照旧 ✓）。
for _p in "A:$a" "B:$b" "C:$c" "D:$d" "E:$e"; do
  if [ "${_p#*:}" = "空观测" ]; then
    echo "G-93：相位 ${_p%%:*} 观测不到任何判卷事件 ⇒ 环境/形状异常（exit 2），不是判据结论 ✗" >&2
    echo "      · 本件五档判据量的都是「有没有诊断」；零事件时它恒为空 ⇒ 「绿」没有信息量 ✗" >&2
    echo "      · 常见成因：scripts/soko 拒跑（exit 3 / 无版本匹配构建）/ 量具空转 / 夹具或模块根不对" >&2
    exit 2
  fi
done
if [ "$b" != "绿" ] || [ "$e" != "绿" ]; then
  echo "环境/夹具异常：对照相位 B / E 必须判绿 ✓（它们红了 ⇒ 夹具或二进制不对 ✗）" >&2
  exit 2
fi
if [ "$a" = "绿" ] && [ "$c" = "绿" ]; then
  echo "行为变了：显式 {u} 的层推断好了 ✓ ⇒ 回来更新台账 ✓"
  exit 1
fi
echo "缺口仍在：G-93「显式 {u} 的宇宙层不参与推断 ⇒ 一律默认 0」✗"
echo "  病灶：实参类型里的层不回流到 u ✗（前置库自己到处显式写层绕着走 ✗ —— prelude.rs:255/257 ✓）"
echo "  症状面：Eq.refl 当项用判红 ✗，而 rfl / by rfl 判绿 ✓（同一命题、换写法两态 ✗）"
echo "  判据：**相位 B 是对照** ✓（它证明「就是层」而不是别的 ✗）"
exit 0

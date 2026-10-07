#!/usr/bin/env bash
# L-06 复现/守卫件：**「没有累积性」与「`Exists.elim` 的 Q 只能是 Prop」都与
#   官方 Lean 4 一致 ⇒ 不是缺口**（本件钉住这两条事实与课程绕法）。
#
# ── 审计结论（2026-10-07，收口轮；源码级对照见下）────────────────────────────
# 原台账把两件事捆在一起，并断言 `expected_lean` = 「Lean 4 有累积性
# （Prop ⊆ Type）且有 Classical.choice / Exists.choose 取数据」。逐条核 Lean 4
# 源码（本机 `~/Documents/lean/lean4`，HEAD d0493e4c1e）后**两半都不成立**：
#
# ① 累积性：**Lean 4 没有宇宙累积性**，`def T : Type := True` 在 Lean 里同样过不了。
#    · `src/kernel/type_checker.cpp` 的 `quick_is_def_eq`（Sort 分支）：
#      `Sort u =?= Sort v` ⇔ `is_def_eq(sort_level(t), sort_level(s))`，而 level 的
#      `is_def_eq` = `is_equivalent`（**相等**，不是 `≤`）；
#    · `src/Lean/Meta/ExprDefEq.lean:1661`：`.sort u, .sort v => isLevelDefEqAux u v`；
#      `src/Lean/Meta/LevelDefEq.lean:113`：`Level.zero, Level.succ .. => LBool.false`
#      —— 且 `.false` 是终局（`ExprDefEq.lean:2109` 的 `whenUndefDo`）⇒ 精化器同样
#      只认「层级相等」；
#    · `src/Lean/Meta/ExprDefEq.lean:104` 的注释把 Lean 称作
#      "a system without universe cumulativity"；
#    · `src/Init/Prelude.lean:842` 有 `structure PLift (α : Sort u) : Type u`
#      （"Wraps a proof or value to increase its type's universe level by 1"），
#      并在 `src/Init/Data/AC.lean:24-30` 真的用来把 Prop 事实放进 Type 结构里 ——
#      有累积性就不需要它；`src/Init/Coe.lean` 里也没有 `Coe Prop (Type u)` 实例
#      （⇒ 没有强转这条后门）。
#    ⇒ 本语言内核（`crates/kernel/src/conv.rs:154` 的
#      `(Sort lx, Sort ly) => eq_antisymm(lx, ly)`）与 Lean **同形**：不是偏离。
#
# ② `Exists.elim` 的 Q：Lean 的真签名就是 `{b : Prop}`（**结果限制在 Prop**）——
#    `src/Init/Core.lean:1060`：
#      `theorem Exists.elim {α : Sort u} {p : α → Prop} {b : Prop}
#         (h₁ : Exists (fun x => p x)) (h₂ : ∀ (a : α), p a → b) : b`
#    取数据在 Lean 里同样要选择公理：`src/Init/Classical.lean:204,207` 的
#    `Exists.choose`（= `Classical.choose`）+ 成对的 `Exists.choose_spec`
#    （`Nonempty.elim` 也是 `{p : Prop}`，`src/Init/Prelude.lean:794`）。
#    ⇒ 本课程 lib 的 `Exists.elim {…} {Q : Prop}` 与单元⑩ 的
#      `Exists.choose`/`Exists.choose_spec` **成对公理**，就是 Lean 的同款路线；
#      「满射可裂证不出来」在 Lean 里同样成立（要用 `Classical.choice`）。
#
# ⇒ 本文件是**修后形状的回归守卫**（不是"缺口仍在"）：三臂都成立 ⇒ exit 1。
#   行为变了（内核开始接受 Prop 当 Type / `Exists.rec` 能取数据 / 课程绕法消失 /
#   hint 又回头声称 Lean 有累积性）⇒ exit 0，逼台账重新开账。
#
# ── 三臂 ────────────────────────────────────────────────────────────────────
#   ① 结构：课程绕法在册 —— `lib/Exists` 的 `Exists.elim` 结果是 `{Q : Prop}`；
#      单元⑩ 有成对的 `axiom Exists.choose` / `Exists.choose_spec`；`lib/Equiv` 的
#      `Set.Equiv … : Prop`。**防空转**：逐条命中数 == 期望数（不是"grep 没报错"）。
#   ② 行为：真判卷三份夹具 —— (A) 内核形状：裸排序 `def T : Type := True` ⇒
#      **专用码 `kernel-prop-not-cumulative`**（期望 `Sort(1)`、实际 `Sort(0)`，
#      hint 含「没有累积性」且**不再**声称「Lean 4 有累积性」）；`Exists.witness`
#      （用 `Exists.rec` 取数据）⇒ `kernel-rejected`；Pi 形状保持通用码；
#      正向对照 `def T2 : Type := Nat` 必须 checked。(B) 课程 lib 上 `Exists.elim`
#      **可用**（Prop 目标判绿）。(C) 课程 lib 上取数据**仍被拒**。
#   ③ 反向（**咬得住**）：把夹具 A 里的 `def T : Type := True` 改成 `:= Nat` ⇒
#      专用码必须**消失**（判据能红）；把课程 lib 副本里的 `Exists.elim` 改名 ⇒
#      夹具 B 必须判红并点名它（守卫不许空转）。
#
# ⚠ 夹具跑在**临时模块根**里（`lib/` 是当次从课程库**复制**的）⇒ 不往课程树写
#   任何东西 ✓。⚠ 判卷用 `grade`（模块根 = 入口目录）；**不用** `grade --root` ——
#   `grade` 子命令把 `--root` 吃掉了（`crates/cli/src/env/mod.rs::grade` 硬编码
#   `root: None`，与 `AGENTS.md` 的写法不符 ⇒ 那是**另一笔**，本件不依赖它）。
# ⚠ 反向验证用：`SOKO_L06_ROOT=<另一份仓库根> bash <本件的副本>` 可以把三臂指向
#   另一棵树（本件自身只用它做"改坏 ⇒ 判红"的取证）。
#
# 退出码约定（`docs/gaps/README.md`）：0 = 行为变了（台账要重开）· 1 = 已满足
#   （两半都与 Lean 一致、课程绕法在册）· 2 = 环境/判据失效
set -uo pipefail

ROOT="${SOKO_L06_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)}"
cd "$ROOT" || exit 2

SOKO="$ROOT/scripts/soko"
LIB_EXISTS="courses/set-theory/lib/Exists.sokonanoda"
LIB_EQUIV="courses/set-theory/lib/Equiv.sokonanoda"
UNIT10="courses/set-theory/units/I.3/unit10-cantor.sokonanoda"

command -v node >/dev/null 2>&1 || { echo "L-06: 需要 node（scripts/soko 是 Node 启动器）⇒ 环境不满足" >&2; exit 2; }
command -v python3 >/dev/null 2>&1 || { echo "L-06: 需要 python3（解析 --json 事件流）⇒ 环境不满足" >&2; exit 2; }
[ -f "$SOKO" ] || { echo "L-06: 找不到 $SOKO ⇒ 环境不满足" >&2; exit 2; }
for f in "$LIB_EXISTS" "$LIB_EQUIV" "$UNIT10"; do
  [ -f "$f" ] || { echo "L-06: 找不到 $f ⇒ 环境不满足" >&2; exit 2; }
done

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

# ── 构建身份（跨轮比较读数前先确认同一份构建；见 AGENTS.md「探针读数必须带构建身份」）──
timeout 120 node "$SOKO" version --json > "$TMP/version.json" 2>/dev/null
IDENT="$(python3 - "$TMP/version.json" <<'PY'
import hashlib, json, sys
try:
    d = json.load(open(sys.argv[1]))
except Exception:
    print("（拿不到 version --json）"); raise SystemExit
cli = d.get("cli") or {}
path = cli.get("path") or ""
digest = ""
try:
    digest = hashlib.sha256(open(path, "rb").read()).hexdigest()[:12]
except Exception:
    digest = "?"
print(f"期望版本 {d.get('version')} · 二进制来源 {cli.get('source')} · {path} · sha256:{digest}（缓存 marker {cli.get('marker')}，本件不用缓存）")
PY
)"
echo "构建身份：$IDENT"

# ── ① 结构：课程绕法在册（逐条命中数 == 期望数）──────────────────────────────
echo "① 结构（课程绕法，L-06 的 workaround 面）："
struct_bad=0

elim_hits=$(grep -cE '^def Exists\.elim' "$LIB_EXISTS")
elim_prop=$(grep -E '^def Exists\.elim' "$LIB_EXISTS" | grep -c '{Q : Prop}')
choose_hits=$(grep -cE '^axiom Exists\.choose :' "$UNIT10")
choose_spec_hits=$(grep -cE '^axiom Exists\.choose_spec :' "$UNIT10")
equiv_hits=$(grep -cE '^def Set\.Equiv .*: Prop :=' "$LIB_EQUIV")
echo "   lib/Exists：def Exists.elim ${elim_hits}（期望 1，其中结果限 Prop 的 ${elim_prop} 条，期望 1）"
echo "   单元⑩：axiom Exists.choose ${choose_hits}（期望 1）· Exists.choose_spec ${choose_spec_hits}（期望 1）"
echo "   lib/Equiv：Set.Equiv … : Prop ${equiv_hits}（期望 1）"
[ "$elim_hits" = 1 ] && [ "$elim_prop" = 1 ] || { echo "   ✗ lib/Exists 的 Exists.elim 不在册（或结果不再是 Q : Prop）" >&2; struct_bad=1; }
[ "$choose_hits" = 1 ] && [ "$choose_spec_hits" = 1 ] || { echo "   ✗ 单元⑩ 的成对选择公理不在册" >&2; struct_bad=1; }
[ "$equiv_hits" = 1 ] || { echo "   ✗ lib/Equiv 的命题版 Set.Equiv 不在册" >&2; struct_bad=1; }

if [ "$struct_bad" != 0 ]; then
  echo "结论：L-06 的行为变了（课程绕法被改动）⇒ exit 0（台账要重开）" >&2
  exit 0
fi

# ── 判卷一次、读数一次（`--json` 事件流：一行一个 JSON）─────────────────────
grade() { ( cd "$ROOT" && timeout 300 node "$SOKO" grade --json "$1" 2>&1 ); }

# 夹具 A：内核形状（自带 Exists，不依赖课程库）
mkdir -p "$TMP/kernel"
cat > "$TMP/kernel/probe.sokonanoda" <<'SOKO'
inductive Exists (A : Type) (p : A -> Prop) : Prop
ctor intro (w : A) (h : p w) : Exists A p
end
def Exists.elim (A : Type) (p : A -> Prop) (Q : Prop) (h : Exists A p) (f : (w : A) -> p w -> Q) : Q :=
  Exists.rec A p (fun (_ : Exists A p) => Q) f h
def Exists.witness (A : Type) (p : A -> Prop) (h : Exists A p) : A :=
  Exists.rec A p (fun (_ : Exists A p) => A) (fun (w : A) (hw : p w) => w) h
def T : Type := True
def bad : Prop -> Type := fun (x : Prop) => x
def T2 : Type := Nat
SOKO

# 夹具 B/C：课程 lib（复制到临时模块根；课程树一个字节都不动）
mkdir -p "$TMP/courselib/lib"
cp "$ROOT"/courses/set-theory/lib/*.sokonanoda "$TMP/courselib/lib/"
cat > "$TMP/courselib/probe_ok.sokonanoda" <<'SOKO'
import lib.Exists
theorem l06_elim_usable (A : Type) (p : A -> Prop) (h : Exists A p) : ∃ (w : A), p w :=
  Exists.elim h (fun (w : A) => fun (hw : p w) => Exists.intro w hw)
SOKO
cat > "$TMP/courselib/probe_bad.sokonanoda" <<'SOKO'
import lib.Exists
def Exists.witness (A : Type) (p : A -> Prop) (h : Exists A p) : A :=
  Exists.rec A p (fun (_ : Exists A p) => A) (fun (w : A) (hw : p w) => w) h
SOKO

# ── ② 行为：真判卷 ──────────────────────────────────────────────────────────
echo "② 行为（真判卷）："

OUT_A="$(grade "$TMP/kernel/probe.sokonanoda")"; rc_a=$?
chk_a=$(printf '%s' "$OUT_A" | grep -c '"type":"decl.checked"')
diag_a=$(printf '%s' "$OUT_A" | grep -c '"type":"diagnostic"')
code_nc=$(printf '%s' "$OUT_A" | grep -c '"code":"kernel-prop-not-cumulative"')
code_gen=$(printf '%s' "$OUT_A" | grep -c '"code":"kernel-rejected"')
hint_nc=$(printf '%s' "$OUT_A" | grep '"code":"kernel-prop-not-cumulative"' | grep -o '"hint":"[^"]*"' | head -1)
msg_nc=$(printf '%s' "$OUT_A" | grep '"code":"kernel-prop-not-cumulative"' | grep -o '"message":"[^"]*"' | head -1)
echo "   A 内核形状：exit=${rc_a}（期望 1）· decl.checked=${chk_a}（期望 3）· diagnostic=${diag_a}（期望 3）"
echo "      专用码 ${code_nc}（期望 1）· 通用码 ${code_gen}（期望 2，= 取数据 + Pi 形状）"
echo "      ${msg_nc}"

OUT_B="$(grade "$TMP/courselib/probe_ok.sokonanoda")"; rc_b=$?
chk_b=$(printf '%s' "$OUT_B" | grep -c '"type":"decl.checked"')
diag_b=$(printf '%s' "$OUT_B" | grep -c '"type":"diagnostic"')
echo "   B 课程 lib 上 Exists.elim 可用：exit=${rc_b}（期望 0）· decl.checked=${chk_b}（期望 1）· diagnostic=${diag_b}（期望 0）"

OUT_C="$(grade "$TMP/courselib/probe_bad.sokonanoda")"; rc_c=$?
chk_c=$(printf '%s' "$OUT_C" | grep -c '"type":"decl.checked"')
diag_c=$(printf '%s' "$OUT_C" | grep -c '"type":"diagnostic"')
code_c=$(printf '%s' "$OUT_C" | grep -c '"code":"kernel-rejected"')
echo "   C 课程 lib 上取数据被拒：exit=${rc_c}（期望 1）· decl.checked=${chk_c}（期望 0）· diagnostic=${diag_c}（期望 1）· 通用码 ${code_c}（期望 1）"

if [ "$rc_a" = 124 ] || [ "$rc_b" = 124 ] || [ "$rc_c" = 124 ]; then
  echo "结论：判卷超时（环境慢，非形状问题）⇒ exit 2" >&2
  exit 2
fi

fail=0
if [ "$rc_a" != 1 ] || [ "$chk_a" != 3 ] || [ "$diag_a" != 3 ] || [ "$code_nc" != 1 ] || [ "$code_gen" != 2 ]; then
  echo "   ✗ 夹具 A 与「与 Lean 一致」的形状不符（内核判定变了？）" >&2
  fail=1
fi
# hint 必须仍说「没有累积性」，且**不许**再声称「Lean 4 有累积性」（本轮更正过的那句）。
if ! printf '%s' "$hint_nc" | grep -q '没有累积性'; then
  echo "   ✗ 专用码的 hint 丢了「没有累积性」⇒ 判据失效" >&2
  fail=1
fi
if printf '%s' "$hint_nc" | grep -q 'Lean 4 有累积性'; then
  echo "   ✗ 专用码的 hint 又回头声称「Lean 4 有累积性」（与 Lean 源码不符）" >&2
  fail=1
fi
if ! printf '%s' "$msg_nc" | grep -q '期望 `Sort(1)`，实际是 `Sort(0)`'; then
  echo "   ✗ 专用码的内核消息不再是 `Sort(1)` vs `Sort(0)`（内核判定变了？）" >&2
  fail=1
fi
if [ "$rc_b" != 0 ] || [ "$chk_b" != 1 ] || [ "$diag_b" != 0 ]; then
  echo "   ✗ 课程 lib 上 Exists.elim 用不起来（工作绕法破了）" >&2
  printf '%s\n' "$OUT_B" | grep -o '"message":"[^"]*"' | head -3 >&2
  fail=1
fi
if [ "$rc_c" != 1 ] || [ "$chk_c" != 0 ] || [ "$diag_c" != 1 ] || [ "$code_c" != 1 ]; then
  echo "   ✗ 课程 lib 上「取数据」不再被拒（大消去被放宽了？）" >&2
  printf '%s\n' "$OUT_C" | grep -o '"message":"[^"]*"' | head -3 >&2
  fail=1
fi

if [ "$fail" != 0 ]; then
  echo "结论：L-06 的行为变了 ⇒ exit 0（台账要重开）" >&2
  exit 0
fi

# ── ③ 反向：改坏 ⇒ 判红（守卫不许空转）──────────────────────────────────────
echo "③ 反向（副本里改坏，课程树/内核都不动）："

# ③-A：夹具 A 的裸排序形状换成合法形状 ⇒ 专用码必须消失（判据能红）。
mkdir -p "$TMP/neg-shape"
sed 's/^def T : Type := True$/def T : Type := Nat/' "$TMP/kernel/probe.sokonanoda" > "$TMP/neg-shape/probe.sokonanoda"
if cmp -s "$TMP/kernel/probe.sokonanoda" "$TMP/neg-shape/probe.sokonanoda"; then
  echo "   ✗ 反向臂 A：sed 没改到夹具（判据失效）⇒ exit 2" >&2
  exit 2
fi
OUT_NA="$(grade "$TMP/neg-shape/probe.sokonanoda")"; rc_na=$?
nc_na=$(printf '%s' "$OUT_NA" | grep -c '"code":"kernel-prop-not-cumulative"')
gen_na=$(printf '%s' "$OUT_NA" | grep -c '"code":"kernel-rejected"')
chk_na=$(printf '%s' "$OUT_NA" | grep -c '"type":"decl.checked"')
if [ "$rc_na" != 1 ] || [ "$nc_na" != 0 ] || [ "$gen_na" != 2 ] || [ "$chk_na" != 4 ]; then
  echo "   ✗ 反向臂 A：形状换掉后专用码仍出现 / 文件整体崩了（rc=${rc_na} 专用码=${nc_na} 通用码=${gen_na} checked=${chk_na}）⇒ 判据失效" >&2
  exit 2
fi
echo "   反向臂 A：形状换掉 ⇒ 专用码 1 → 0、正向对照 checked 3 → 4 ✓（专用码不是「见红就贴」）"

# ③-B：课程 lib 副本里把 Exists.elim 改名 ⇒ 夹具 B 必须判红并点名它。
mkdir -p "$TMP/neg-lib/lib"
cp "$ROOT"/courses/set-theory/lib/*.sokonanoda "$TMP/neg-lib/lib/"
sed 's/^def Exists\.elim /def Exists.elim_renamed /' "$TMP/neg-lib/lib/Exists.sokonanoda" > "$TMP/neg-lib/lib/Exists.tmp"
mv "$TMP/neg-lib/lib/Exists.tmp" "$TMP/neg-lib/lib/Exists.sokonanoda"
cp "$TMP/courselib/probe_ok.sokonanoda" "$TMP/neg-lib/probe_ok.sokonanoda"
OUT_NB="$(grade "$TMP/neg-lib/probe_ok.sokonanoda")"; rc_nb=$?
if [ "$rc_nb" = 0 ]; then
  echo "   ✗ 反向臂 B：库里的 Exists.elim 改名后夹具 B 仍判绿 ⇒ 守卫空转（判据失效）" >&2
  exit 2
fi
if ! printf '%s' "$OUT_NB" | grep -q 'Exists.elim'; then
  echo "   ✗ 反向臂 B：判红了，但诊断没有点名 Exists.elim ⇒ 判据失效" >&2
  exit 2
fi
echo "   反向臂 B：库里改名 ⇒ 夹具 B exit=${rc_nb} 且诊断点名 Exists.elim ✓"

echo "结论：L-06 已满足（两半都与官方 Lean 4 一致；课程绕法在册、真判卷可用、反向臂咬得住）⇒ exit 1" >&2
exit 1

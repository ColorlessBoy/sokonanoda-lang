#!/usr/bin/env bash
# r17-common-lean-syntax.sh —— 常见 Lean 语法（intros / simp / rw）不支持，且提示不说是语法不支持
# 现象: 从任何 Lean 教程来的初学者都会先写 `intros` / `simp` / `rw [...]`。实测：
#   · `intros x hx` → parse 失败（Rust Debug dump，见 r07），没有任何一句说「本子集只有 intro」；
#   · `simp`         → 同上；
#   · `rw [Set.mem_singleton_iff]` → `expected a valid .sokonanoda token, found [`。
#   白名单只写在画布**注释**里（unit01 第 4-5 行），工具侧不给。
# 期望: 不支持的 tactic/语法要在报错里点名并给出替代（如「用 intro 代替 intros」）。
# 判据: exit 0 = 问题仍在（三条里至少一条的报错不含「不支持/请改用」字样）/ exit 1 = 已给替代提示
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
W="$REPO/.cache/ux/lang/repro/.work"
printf 'import lib.Logic\nimport lib.Set\n\ntheorem t (p q : Prop) (hp : p) : p := by\n  intros\n  exact hp\n' > "$W/r17-intros.sokonanoda"
printf 'import lib.Logic\nimport lib.Set\n\ntheorem t (p q : Prop) (hp : p) : p := by\n  simp\n' > "$W/r17-simp.sokonanoda"
printf 'import lib.Logic\nimport lib.Set\n\ntheorem t (α : Type) (x a : α) (hx : x \xe2\x88\x88 {a}) : x = a := by\n  rw [Set.mem_singleton_iff]\n  exact hx\n' > "$W/r17-rw.sokonanoda"
bad=0
for f in r17-intros r17-simp r17-rw; do
  o=$(timeout 300 "$REPO/scripts/soko" query check --file "$W/$f.sokonanoda" 2>&1)
  echo "$f: $(grep -o '"code": "[^"]*"' <<<"$o" | head -1)"
  grep -qE '不支持|请改用|请用' <<<"$o" || bad=1
done
if [ "$bad" -eq 1 ]; then
  echo "STILL REPRODUCES: 常见 Lean 语法被拒时不提示「不支持/请改用」"; exit 0
fi
echo "FIXED: 已给出替代写法提示"; exit 1

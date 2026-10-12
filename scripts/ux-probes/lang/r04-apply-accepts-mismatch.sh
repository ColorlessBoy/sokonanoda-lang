#!/usr/bin/env bash
# r04-apply-accepts-mismatch.sh —— `apply` 接受结论对不上的 Eq.subst，静默造出不可能的子目标
# 现象: 目标 `A x`，写 `apply Eq.subst α (fun y => A y) x a`（x a 写反）。
#       `Eq.subst α P x a : x = a → P x → P a`，结论 `P a` 与目标 `A x` **对不上**，
#       `apply` 本该失败（对照：`apply hpr` 在结论不匹配时正确报
#       「`apply` 的目标不匹配：`hpr` 的结果是 `r`，无法对齐当前目标 `q`」）。
#       实测它**成功了**，并留下子目标 `x = a`（对任意 x a 不可证）+ 原目标。
#       ⇒ 错误行不报错，错误在 2 行后以「`exact` 类型不匹配」冒出来（画布 p03 实况）。
# 期望: apply 必须校验结论；对不上就在**写错的那一行**报错。
# 判据: exit 0 = 问题仍在（apply 行无诊断且留下 x = a 子目标）/ exit 1 = 已修
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
F="$REPO/.cache/ux/lang/repro/.work/r04.sokonanoda"
cat > "$F" <<'SOKO'
import lib.Logic
import lib.Set

theorem apply_should_fail (α : Type) (A : Set α) (a x : α) (ha : A a) : A x := by
  apply Eq.subst α (fun (y : α) => A y) x a
  sorry
SOKO
out=$(timeout 300 "$REPO/scripts/soko" query check --file "$F" 2>&1); rc=$?
st=$(timeout 300 "$REPO/scripts/soko" query state --file "$F" --line 7 --col 3 2>&1)
echo "grade rc=$rc"; echo "state entering line 7: $(grep -o '"goal":"[^"]*"' <<<"$st" | tr '\n' ' ')"
if [ "$rc" -eq 0 ] && grep -q 'x = a' <<<"$st"; then
  echo "STILL REPRODUCES: apply 未报错，且留下不可证的子目标 x = a"; exit 0
fi
echo "FIXED: apply 已拒绝结论不匹配的项"; echo "$out" | head -c 400; exit 1

#!/usr/bin/env bash
# G-72 自断言复现：**同一个模块，单文件判绿、被 import 时判红**（"库自己绿、每个使用者红"）
#
# 退出码约定（全部 repro 脚本一致，见 docs/gaps/README.md）：
#   0 = **症状仍在**（模块单独判卷 exit 0，入口 import 它后 exit ≠ 0）
#   1 = 症状消失（已修复）⇒ 回来更新台账（写 fixed_in）并升级课程
#   2 = 环境/前置缺失（跑不起来，不算结果）
#
# 现场（2026-10-01，卷 I 单元⑬ 写 `lib/Order` 时撞到，觅了几十分钟）：
#   模块里写 `def strictOf (α : Type) (r : α → α → Prop) : α → α → Prop :=
#   fun (a b : α) => r a b ∧ a ≠ b` —— **返回类型是字面箭头 + λ 体 + 体里带 `∧`**。
#   ① 直接判这个模块：**全绿**（本脚本的相位 A）；
#   ② 任何 `import` 它的文件：报 `unknown identifier \`b\`` —— λ 体里**第二个绑定变量丢了**
#      （相位 B）。实测还见过更坏的一档：`fun (a b : α) => r a b`（体里不引用 `b`）
#      判**绿**，但定义体已被悄悄改坏 ⇒ 属于「**假绿**」那一类。
#   绕法（课程已采用，写在 `lib/Order` 文件头）：返回类型换成**具名别名** `Rel α α`
#   （`lib/Rel` 的 `Rel A B := A → B → Prop`），实测两态都对。
#
# ── **已修（G-72，0.81.0）** ──
#   **根因**（不在 elaborate 的两条路上，在**源级 delta 表**里）：
#   `crates/front/src/compile/elab.rs::params_of_ty` 把**返回类型里的 `->`** 也当
#   参数数进来 —— `def mkRel (α : Type) (r : α → α → Prop) : α → α → Prop :=
#   fun (a b : α) => …` 的类型是 `(α) → (r) → α → α → Prop` ⇒ `params` = **4** 条，
#   而值位外面的 lambda 只有 **2** 层 ⇒ `strip_lambdas_n(val, 4)` **多剥两层**
#   （把 `fun (a b : α) =>` 也剥掉）⇒ 登记进 `defs` 的「定义体」里 `a`/`b` 悬空。
#   **为什么恰好是"单文件绿、import 红"**：模块自己判卷**不展开**这层 delta；
#   入口引用它时（`And.left h` 要解隐式实参）才走 `unfold_one_with` 展开 ⇒
#   回读悬空的 `b` ⇒ `unknown identifier b` ✗。
#   **修法**：`params_of_ty` **只数前导 `Forall`**（返回类型的 `->` 不是参数），
#   并把「完整望远镜」的口径单独留给要它的地方（`telescope_arity_of_ty` /
#   `DefInfo.telescope_arity`，`by.rs::def_shape` 用）；`strip_lambdas_n` 的层数
#   与 `params` **同源** ✓。
#
# 判据形状：**两态必须一致**。修好后相位 A 与相位 B 都 exit 0 ⇒ 本脚本 exit 1。

set -u
cd "$(dirname "$0")/../../.." || exit 2
SOKO="$PWD/scripts/soko"
command -v node >/dev/null 2>&1 || { echo "需要 node（scripts/soko 是零依赖 Node 启动器）" >&2; exit 2; }
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

mkdir -p "$TMP/lib" "$TMP/units"
printf 'name = "g72"\n' > "$TMP/sokonanoda.toml"
cat > "$TMP/lib/Lib.sokonanoda" <<'EOF'
axiom P : Prop

def BinRel (α : Type) : Type := α → α → Prop

def mkRel (α : Type) (r : α → α → Prop) : α → α → Prop :=
  fun (a b : α) => r a b ∧ P
EOF
cat > "$TMP/units/u.sokonanoda" <<'EOF'
import lib.Lib

theorem use (α : Type) (r : α → α → Prop) (a : α) (h : mkRel α r a a) : r a a :=
  And.left h
EOF

run() { # $1 = 绝对路径 · $2 = 输出落盘（原来直接丢弃 ⇒ 没法判"有没有观测" ✗）
  node "$SOKO" query check --file "$1" > "$2" 2>&1
  echo $?
}

A=$(run "$TMP/lib/Lib.sokonanoda" "$TMP/a.json")
B=$(run "$TMP/units/u.sokonanoda" "$TMP/b.json")
echo "相位 A（模块单文件判卷） exit=$A   （期望 0）"
echo "相位 B（入口 import 后判卷） exit=$B   （今天 ≠ 0）"

# ⭐ **零观测闸门**（2026-10-15，洞 `GATE-FALSE-GREEN-11-MORE`）：本件的读数**只有
# `query check` 的 exit 码**，而量具空转（零输出 / 拒跑）时它**恒为 0** ⇒ 两态都读成
# "都绿" ⇒ 逐字落进「G-72 已修」= **假绿** ✗。⇒ 先要求两次查询真的**答出了本夹具的
# 声明**（可归属 ✓：`query check` 的 JSON 里 `decl_checked ≥ 1` 或 `failed` 非空 ——
# 两者都只可能来自真的跑了夹具 ✓）；观测不到 ⇒ **exit 2 + 人话**。
need_answer() { # $1 = 档名 · $2 = JSON 文件
  if [ ! -s "$2" ] || ! grep -qE '"decl_checked": [1-9]|"failed": \[$' "$2"; then
    echo "G-72 $1：观测不到 query check 的答案（既没有判过的声明，也没有失败项）⇒ 环境/形状异常（exit 2），不是判据结论 ✗" >&2
    echo "      · 本件的读数就是 query check 的退出码；零输出时它恒为 0 ⇒ 「已修」没有信息量 ✗" >&2
    echo "      · 常见成因：scripts/soko 拒跑（exit 3 / 无版本匹配构建）/ 量具空转 / 夹具或模块根不对" >&2
    printf '%s\n' "$(head -3 "$2" 2>/dev/null)" | sed 's/^/        → /' >&2
    exit 2
  fi
}
need_answer "相位 A（$TMP/lib/Lib.sokonanoda）" "$TMP/a.json"
need_answer "相位 B（$TMP/units/u.sokonanoda）" "$TMP/b.json"

if [ "$A" != "0" ]; then
  echo "✗ 相位 A 都不绿 ⇒ 复现件的形状变了（不是 G-72）" >&2
  exit 1
fi
if [ "$B" = "0" ]; then
  echo "✓ 两态一致（都绿）⇒ G-72 已修，回来关账" >&2
  exit 1
fi
echo "✓ 症状仍在：模块单文件绿、被 import 红（G-72）"
exit 0

#!/usr/bin/env bash
# **G-97 自断言复现**：prelude 的**生效源**（编辑器 F12 打开的那一份）自己编译不过。
#
# 缺口（2026-10-10 实测）：`prelude/L1.sokonanoda` 的 `Classical.byContradiction` 里
# `Or.elim` **漏了动机位 `c`**（`Classical.em p` 被塞进 `f`）⇒ 这份源 43 条声明里
# 1 条判红（`kernel-rejected`：期望 `Sort(0)`，实际 `Or p (Not p)`）。它从落地那天起
# 就没编译过，只是**没有任何判据把它当普通文档判一遍** ✗ —— prelude 是「受信任安装」
# （装进环境后不进 `PendingOp`、内核从不重查），`install_l1_command` 的 `.expect(...)`
# **只保证 elaborate 成功、不保证内核接受** ⇒ 类型错的**体**被静默装进环境，而**类型**
# 是对的 ⇒ 所有使用点照常判绿。
#
# 修后契约（2026-10-10）：`scripts/soko grade --json prelude/Prelude.sokonanoda` ⇒ **exit 0**、
# **0 诊断**（43 条 `decl.checked`）；常驻守卫 = `crates/front/tests/prelude_mirror.rs::
# the_prelude_source_grades_clean_as_an_ordinary_document`（把生效源当普通文档判）。
#
# 退出码约定（全部 repro 脚本一致，见 docs/gaps/README.md）：
#   0 = 缺口仍在 · 1 = 已修 · 2 = 环境/形状异常（需要人看）
set -u
cd "$(dirname "$0")/../../.." || exit 2

for f in prelude/Prelude.sokonanoda prelude/L1.sokonanoda; do
  out=$(timeout 300 scripts/soko grade --json "$f" 2>&1)
  code=$?
  if [ "$code" = 2 ]; then
    echo "G-97: 跑不起来（scripts/soko grade $f ⇒ exit 2）" >&2
    exit 2
  fi
  diag=$(printf '%s\n' "$out" | grep -c '"type":"diagnostic"')
  if [ "$code" != 0 ] || [ "$diag" != 0 ]; then
    echo "G-97 缺口仍在 ✗：$f ⇒ exit=$code / 诊断 $diag 条"
    printf '%s\n' "$out" | grep '"type":"diagnostic"' | head -3
    exit 0
  fi
  echo "G-97 ✓ $f ⇒ exit 0 / 0 诊断"
done
exit 1

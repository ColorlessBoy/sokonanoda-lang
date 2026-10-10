#!/usr/bin/env bash
# **G-105 复现件**：记法折叠把「部分应用」当「完全应用」折坏。
#
# 现场（2026-10-10 用户实测）：`exact Eq.refl α a` 的 hover 里类型行显示
#   `Eq.refl : {α : Sort u} → (a : α) → (α = a) a`      ✗（应为 `… → a = a`）
# 根因：折叠表的 arity 用的是**显式 binder 数**，而内核 pp 对**开项**会打**全量实参**
# （`Eq.{u} α a a`）⇒ `Eq α a` 这个**部分应用**被当成饱和应用折成 `α = a`，尾巴 ` a`
# 留在外面 ⇒ 父节点重渲染成 `(α = a) a`。同族：`Set.mem α a A` → `(α ∈ a) A`。
#
# ⚠ **判据必须打在"真现场"那条路上**：编译路的 `ty_text` 对**用户声明**打的是
# **闭项**形状（`Eq a a`，pp 省掉前导隐式）—— **旧口径本来就能把它折对** ⇒ 拿
# `query goals` 的 `ty` 当判据会**空转**（修前也绿 ✗，"咬不住的守卫等于没有"）。
# 真现场在 **hover / `#check` 的渲染路**（开项 `Eq.{u} α a a`）⇒ 判据取
# **真相层 `messages_at` 的文本**（= Infoview「命令输出」显示的那一份），
# 由共用的 `editor-hover-probe.js` 的 `fold` 模式断言（见那里的头注释）。
#
# 退出码（全部 repro 脚本一致，见 docs/gaps/README.md）：
#   0 = 缺口仍在 · 1 = 已修 · 2 = 环境/形状异常（需要人看）
set -u
cd "$(dirname "$0")/../../.." || exit 2
command -v node >/dev/null 2>&1 || { echo "需要 node" >&2; exit 2; }
exec node docs/gaps/repro/editor-hover-probe.js fold

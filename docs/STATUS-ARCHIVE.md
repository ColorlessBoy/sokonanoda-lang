# STATUS 归档（逐轮过程记录）

> **本文件是"旧轮的去处"** ✓（`AGENTS.md` §收尾义务 / `scripts/status-lint.py` 的口径 ✓）：
> `STATUS.md` 只留**最近 ≤3 轮**，更早的轮次进这里 ✓；这里再更早的进
> `docs/archive/`（gzip ✓，**归档 ≠ 销毁** ✓）。
>
> **已归档**（`docs/archive/` ✓，**归档 ≠ 销毁** ✓）：**第 1–104 轮**（2026-09-06 → 09-19）
> 与更早的散段 ⇒ `docs/archive/status-archive-older-rounds.md.gz`（`gunzip -c … | less` ✓）。
> **本文件保留**：轮号最大的 **8 段**（第 122–321 轮）—— 最近的过程记录。

## 第 491 轮（2026-09-28）：**ST16–ST19 收口**（v1 保留的四项）

- **用户拍板**：「ST16/ST18/ST19 收完 ⇒ 本地 `soko gate` exit 0 ⇒ bump `v0.77.1`」
  「**严禁为了让 scoped"有用"去改记法判定或硬造一个用法**」「登记即交付物，这条不算失败」✓。
- **ST16 三元素 `{a,b,c}`** ✓（**教学侧做、语言侧不做**）：今天报专用诊断 `set-literal-shape`
  （「集合字面量 v1 只支持 1–2 个元素…三个及以上请用点名形式」）；本档补**教学落点** ——
  `lib/Set` 新增 `abbrev triple`（三元素的点名替身）⇒ 学习者有明确替代，不用猜 ✓。
  语言侧实现要递归嵌套 + hover/跳转接线（`notation-subset.md` §14.4 的 N11）⇒ 归 G-60 同族、本版不做 ✓。
- **ST17 `abbrev` ✅ / `scoped` ✗（边界已实测，不再挖）**：`abbrev` 落了真实用法
  （`abbrev triple`）✓。`scoped` 试过 `scoped notation "⋂ₚ" => Set.sep` 放 `namespace Set`：
  **同模块可用、跨模块必然失败**（使用者 `open scoped Set` 后报 `elab-notation-argument-unsolved`）
  ⚠ **对照组钉死根因**：同一形状的**非 scoped** 顶层写法（`notation "∈₉" => Set.mem` 写在
  **另一个文件**里）**报同一个错** ⇒ **不是 `scoped` 机制的问题**，是**记法展开时补不出前导类型
  参数**这条**既有边界**（前导参数只在「目标与记法声明同模块」时解得出）⇒ 那条记法**已从库里移除**
  （留着会误导：库内可用、学习者一用就报错），结论与理由写进 `lib/Set` 文件尾 ✓。
- **ST18 速查表清账** ✓：两处「待登记台账」的旧声称**全部 0.77.0 实测复核** ——
  ① 「点名漏 `α` 的报错形态在 0.62.0 变了」⇒ 复测**逐字相同**（`expected A a` / `actual Set.mem a A`）；
  ② 「`∅ = A` / `∅ ≠ A` 仍判红」⇒ **仍红**，拿到 0.77.0 原文
  「`def_eq failed: def_eq mismatch expected: Sort(0) | actual: Sort(1)`」；
  **新增两条今天能过的形态**（`(Set.empty α) = (Set.empty α)`、`(Set.empty α) = A → A = (Set.empty α)`）
  ⇒ 文档与实现对齐 ✓；速查表判卷 **18 checked / 0 诊断** ✓。
- **ST19 补记法**（能做的那半做了）✓：清账后确认**表 1/2/2b/2c 的记法全部真能用** ✓；
  **补不了的三条如实登记**：`{x ∈ A | P x}`（**G-60**）· 三元素 `{a,b,c}`（G-60 同族）·
  **跨模块的 `scoped` 记法**（ST17 的边界）—— 三条都**不是记法写错**，是**语言边界** ✓。
- **门禁** ✓：课程 **43 目标 · 376 checked · 99 open · 0 判负** · `docs-lint` ✓ · `status-lint` ✓ ·
  **内核零改动**（`git diff crates/kernel/` 为空）✓。
- ⚠ **ST6 / ST7 / ST9 / ST11 = blocked-by-kernel**（**不含糊**）：四条都卡在
  **G-56**（`Acc` 立不起来：`check_uniform_inductive_occurrences_at` 的「参数位/指标位」两段语义
  没做出来）+ **G-58/G-59**（大消去）；**下标写返回位**那条另卡 **G-64**（递归子的宇宙代入）✓。

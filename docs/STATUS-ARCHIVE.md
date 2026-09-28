# STATUS 归档（逐轮过程记录）

> **本文件是"旧轮的去处"** ✓（`AGENTS.md` §收尾义务 / `scripts/status-lint.py` 的口径 ✓）：
> `STATUS.md` 只留**最近 ≤3 轮**，更早的轮次进这里 ✓；这里再更早的进
> `docs/archive/`（gzip ✓，**归档 ≠ 销毁** ✓）。
>
> **已归档**（`docs/archive/` ✓，**归档 ≠ 销毁** ✓）：**第 1–104 轮**（2026-09-06 → 09-19）
> 与更早的散段 ⇒ `docs/archive/status-archive-older-rounds.md.gz`（`gunzip -c … | less` ✓）。
> **本文件保留**：轮号最大的 **8 段**（第 122–321 轮）—— 最近的过程记录。

## 第 490 轮（2026-09-28）：G-56/G-64 内核可行性探针（**两道门都定死，都不修**）+ 交接书

- **用户授权**：Acc 那条内核活「肯定要做」⇒ 派了两轮有界探针（**时间盒 2–3 轮，已用满**）。
  **结论：带索引归纳类型有两道门，本版两条都不修**（用户：不可行 ⇒ 登记 +
  补 ST15 清单，**登记本身就是交付物，不许硬凑**）✓。
- **写法 A（下标写块头）⇒ G-56**：判定本体 `inductive.rs:153`（唯一调用点 `:44`，前端零镜像）。
  **插桩把机制钉死了**：卫 `args_rev.len() <= num_params` 对 `Acc` **为假 ⇒ 完全跳过**；
  **判红其实来自另一个块**（`num_params=3`、单构造子、实参 `Var(4)·Var(3)·Var(1)` 期望 `4,3,2`）。
  ⚠ **放宽不是挪一个比较符**：改成 `>=`（只校验前 `num_params` 个实参）**打破了原本能过的块**
  （它靠"跳过"过关）⇒ 实测判红 ⇒ 正解要**区分参数位与指标位两段语义**。
- **写法 B（下标写返回位，Lean 官方写法）⇒ G-64**：**过了 uniform 与 `SPEC0`**
  （实测 `local_params.len=2 indices.len=1` ✓）—— 只剩最后一步，比写法 A **窄得多**。
  卡在 `expr.rs:381` `subst_expr_levels` 的 `assert_eq!(ks.len(), vs.len())`（`left: 0 / right: 1`）。
  **定位手法**：内核 `try_check_declar_at`（`util.rs:674`）**自己装了静默 hook** 吞栈 ⇒
  改那个 hook 打 `Backtrace::force_capture()` 才拿到（改 `main.rs` 会被 `quiet()` 覆盖 ✗）。
- **第 2 轮（最后一次）**：定死**不是调用方传错，是断言对「无可代入」那一支写得过强**
  （`ks` 空 ⇒ 原样返回；`vs` 多出的层级**没有消费者**）。**已验证修法**（实测有效、**未合入**）：
  `ks.is_empty()` 单独提前返回、保留 `ks` 非空时的长度断言（**反向判据**：真不匹配仍判红 ✓）。
  改完 `left: 0 / right: 1` **消失** ✓，但 variant ② **换成 G-59 的判红**（motive 被钉在 `Sort 0`）
  ⇒ **G-64 是第一道门、G-59 是下一道** ⇒ 时间盒用满，**停手**。
- **红线** ✓：**内核零改动**（`git diff crates/kernel/` 为空）· `grep '@@@' crates/kernel/src/` **0 处** ✓。
- **交付** ✓：**新登记 G-64**（`expr.rs:383` 的断言 + 触发形状 + `left/right` 含义 + 已验证修法）·
  G-56 机制补全 · ST15 清单同步（现 **3 blocker + 5 painful**）·
  **交接书 `docs/HANDOFF-0.77.md`**（状态 / 两道门 / 下一步 / 硬规则 / 本轮 5 个坑）。
- **v0.77.0 已发布** ✓（tag `v0.77.0`；release `36382099817` **11/11 success**；
  发版依据轮 `36378945287` ⇒ `ci-green.py` **exit 0**）。
  ⚠ 发版轮那条红是 **`keystroke_recompile_closure` 的绝对毫秒判据跨机不可转移**（同代码 44ms↔2431ms）
  ⇒ 已换成**同机比值 + 宽天花板**并记入 `docs/CI-FAILURES.md` ✓。

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

# 当前快照（2026-09-28）

- **进度**：**64/66**（E2 50/50 ✓ + **批次 N** 记法×隐式参数×产品交互 14/16 ✓）· 🚀 **v0.74.0：11/11 全部 ✅ 已发布** · 🚀 **v0.75.0：4/4 ✅ 已发布** · 🚀 **v0.76.0：3/3 ✅ 已发布** · 🚀 **v0.77.0：ST1 ✅（决策记录 + 两端对账守卫）· ST2 ✅（商类型 `Quot` 装进源语言，路线 A）· ST10 未开**
- **已发布**：**`sokonanoda v0.76.0`** ✓（`gh release list` 显示 **Latest** ✓ · **2026-09-27T22:44:45Z** ✓ · tag `v0.76.0` · bump `b907a20` · CI run `36354848755` 真绿 · release workflow `36356038826` **11/11 success** ✓）
- **A 组（A0–A5）全部落地** ✓：显示层混合形态（`ty_text` ASCII `->` **227 → 0**）· `{a}` 折回 ·
  `{a}` 可跳转（根因：开放练习的签名**一条 hover 行都没有**）· prelude 可跳转（F12 落到前奏源文件
  的真 span）· `flawed_equalities_refuted` 去点名（标记 5 → 3）· 洞的期望类型**两半分开钉**
  （显示副本必折 / 真相字段一个字节都不许折）。A0 守卫基线 **68 → 59**（结论：**59 是地板**）。
- **真宿主 e2e**：A1/A2/A3/A4 四条 ✓（全量 **32/32**）· 反向验证撤修复 ⇒ **1 passed / 3 failed** ✓。
- **B3 第一刀** ✓（`04d863a`）：**裸常量**的隐式插入（G-40 收口）—— `∅` = 裸 `Set.empty` 以前停在
  那个 Pi 上、`rfl` 判不出来 ✗ ⇒ 走 `solve_prefix` 路线 ② 从期望类型补 ✓；判据 + 反向验证 ✓。
  新登记 **G-41**（记法路径 + 隐式 binder ⇒ 真库 `328/0` 掉到 `226 checked · 14 判负` ✗）与
  **G-42 已修 ✓**、**G-43（B2 的第一个真缺口）已登记** ⇒ **B2 仍被 G-43 挡住** ✗。
- **最终证据表（第 31 轮全面复跑 ✓）**：front **747/0** · LSP **164/0** · 课程门禁
  **36 目标 · 328 checked · 99 open · 0 判负** · webview **19/19** · stub 宿主 **37/37** ·
  复现件 **G-40/41/42 ⇒ exit 1（已修 ✓）**、**G-43 ⇒ exit 0（仍在 ✓）** · 六个守卫全 **exit 0** ✓。
- **CI**：**九轮连续 success** ✓（`36270722504` 起）；此前三轮 `cancelled` 的真因已修 ✓（`1be86f9`）。
- **P4（概览尺 + 整行装饰）** ✓：编译期间给当前文档加整行装饰 ⇒ 右侧**概览尺**（Right 道）与
  行背景同时亮 ✓；**零资源**、**成对**（`end` 必须清空 ✗ 否则高亮永远留着）；反向验证 ✓（37/37 ✓）。
- **Infoview 字号** ✓（`b210974`）：`0.78em` ⇒ 1em、去掉**双重压暗**的 opacity、行高 1.5、
  新设置 `sokonanoda.infoview.fontScale`；CSS 契约判据 + 反向验证 ✓。
- **编译进度 P1/P2/P3主机侧/P6** ✓（`deeaf8b`+`a26bd33`+`b182972`）：LSP **成对**报 `$/progress`
  （令牌按 uri ✓）· 状态栏"编译中"态（P2）· Infoview **3 行**进度区（P3，判据钉"正好 3 行 /
  就地更新 / end 清干净"✓）· 节流 `sokonanoda.progress.throttleMs`（只节流 `report` ✓）。
- **B3 第二刀（路线③）** ✓：**只有隐式 binder 的常量**被应用时（`Set.univ x`）富余实参落到
  **结果类型**上、参数从富余实参的类型解出 ⇒ **G-41 收口**（判据先判红 + 反向验证 ✓；内核零
  改动 ✓）。⚠ 第一版放宽到"任意富余实参"**当场打红 prelude** ⇒ 收紧到 `explicit_layers == 0`
  （那一档**没有**旧写法歧义 ✓）；放宽要先能判定"旧写法是否良型"（Lean 用元变量 ✗）。
- ⚠ **本轮的工作方式教训（耗时账 / CI 被顶掉 / clippy 被本地门禁抓到）** ⇒ 见下面「未决项」✓

## 第 487 轮（2026-09-28）：G-56 顶层探针 —— **大消去不可用**（ST15 第一批条目：G-56 / G-58 / G-59）

- **用户指派**：「先用最低成本把 G-56 的『顶层能不能通』探清楚，再决定投入多少」+ **时间盒 2–3 轮**、
  「把不足逼出来才算是做完，把它修好不是本版目标」。**零改动**（`git diff crates/kernel/` 空 ✓）。
- **① 大消去探针 ⇒ 被拒** ✗：`def andToType (A B : Prop) (h : A ∧ B) : Type :=
  And.rec A B (fun (_ : A ∧ B) => Type) A B h` ⇒ `kernel-rejected` /「期望
  `Pi (x : ((And.[] $2) $1)), Sort(0)`，实际是 `Pi (_ : ((And.[] $2) $1)), Sort(2)`」。
  **对照组**（消去到 `Prop`）**checked** ✓ ⇒ 不是写错 recursor。机制：`mk_elim_level` 问
  `large_elim_test`，false ⇒ `elim_level = 0` ⇒ `mk_motive_dep` 把 motive 钉在 `Sort(0)`。
  ⇒ **即使 `Acc` 声明成功，ST7 的 rank 也返回不了序数**（`Acc` 在 Prop、`Ordinal` 在 Type）✗。
- **② 拦路点确认唯一**（只读）✓：`crates/kernel/src/inductive.rs:44`（`check_ctor` 循环）调
  `check_uniform_inductive_occurrences`；`:150` 是 `_at` 入口包装、其余全自递归；**前端零镜像**。
  它要求递归出现**恰好**套用 `num_params` 个实参（`args_rev.len() <= num_params` 才进断言）⇒
  `Acc r b` 有 2 个实参 > `num_params=1` ⇒ 直接 assert 失败。⚠ 同文件 `which_valid_ind_app_v`
  （`:1049`）用 `is_bvar_at` 判参数位置、**允许索引位任意** ⇒ 最小放宽应**对齐它**。
  改动**局部**：只跑在声明级（`check_ctor` 里、`mk_elim_level` 之前），不参与
  `check_generated_recursors` / `check_positivity1` / 归约。
- **③ 顺带发现（独立）** ✗：住在 `Type` 的归纳块**默认 motive 也是 `Prop`**；显式 `.{1}` 只修好
  「`#check` 的显示」，**消去仍被拒**（`Sort(1)` vs `Sort(2)`）⇒ **本版没有绕法**。
  内核探针实测（临时 `eprintln`，**已还原**）：该块 `large_elim_test` 返回 **true** ⇒
  疑似**前端派生递归子的宇宙参数默认值**与内核期望不一致（根因未定位，时间盒到了）。
- **落台账（ST15 第一批条目）** ✓：**G-56**（`Acc` 声明被拒，收敛为一条 + 拦路点定位）·
  **G-58**（大消去不可用）· **G-59**（`Type` 值块默认 motive + 无绕法）；两个 `.sokonanoda`
  复现件 + `scripts/gap.py check` 全绿 ✓；判据
  `crates/front/src/compile/tests.rs::g58_g59_large_elimination_is_unavailable`（三条断言，含
  「显式 `.{1}` 仍被拒」的防漂移）。
- **结论与下一步** ✓：G-56 那四个环节（ST6/ST7/ST9/ST11）**本版不做**、留给 ST15；
  按用户决策规则**直接开 ST3/ST4/ST5**（按 ST1 结论只差 binder 记法、不需要任何新机制）。

## 第 488 轮（2026-09-28）：ST3 / ST4 / ST5 / ST8 / ST10 —— **五章收口** + 四条新缺口（G-60…G-63）

- **用户拍板顺序**：「① 先推进不依赖 Acc 的：ST8 → ST10 → ST12 → ST13 → ST14 → ST15；
  ② 撞墙的一律不硬做，登记进台账 + 进 ST15 清单（**登记本身就是交付物**）；
  ③ 卡住的四项（ST6/ST7/ST9/ST11）放到最后，别自己开工」。本轮按此推进 ✓。
- **ST3 分离** ✓：`lib/Set` 新增 `Set.sep` + `mem_sep_iff`/`sep_subset`/`sep_self`。
  ⚠ **记法那一半做不到** ⇒ 新登记 **G-60**（`{x | P x}` ⇒ `set-literal-shape`；
  `{x : α | x ∈ A}` ⇒ `unexpected-token`「found Pipe」；根因：六种记法形状里**没有
  「操作数在括号里」**，且 `{` 被判成 binder 组）⇒ 课程点名写 `Set.sep` ✓。
- **ST4 集族并交** ✓：**新模块** `lib/SUnion`（`Set.sUnion`/`Set.sInter` + 四条展开引理 +
  **记法 `⋃₀`/`⋂₀`**）。**有意偏离 Mathlib**：按**依赖**拆模块（无限并要 `∃`，
  而 `lib/Set` 不 import `Exists`）⇒ 理由写进文件头 ✓。
- **ST5 不交并 / 函数空间** ✓：**新模块** `lib/Sum`（+ 记法 `⊕`）；`lib/Fun` 增 `Set.pi`
  + `Set.mem_pi`（**不另立** `Set.funSpace` —— 非依赖版就是 `Set.pi s (fun _ => t)` ✓）。
- **ST8 序数（谓词式）** ✓：**新模块** `lib/Ordinal`（7 定义 + 3 条 L2 引理），**零新类型**。
  ⚠ 与 Isabelle 的**两处有意不同**写进文件头：① Isabelle 的 `Ord` **不含良基性**（靠全局
  公理 `foundation`），我们**显式写进 `IsOrdinal`**（本语言没有那条公理、也没有 `Acc`）；
  ② `Transset` 用 `E` 的传递性而非子集序 ✓。
- **ST10 基数（类型的商）** ✓：**新模块** `lib/Cardinal`，**7 条全 checked**（含 `Cardinal.sound`
  = `Quot.sound` 直接实例、`Cardinal.lift_mk` = `Eq.refl` 级 ⇒ **ST2 的归约真的发生** ✓）
  —— **ST2 的 `Quot` 第一次实战检验通过** ✓。
  ⚠ **三条新缺口**（ST15 条目）：**G-61** 没有 η ⇒ `Quotient`/`Setoid` 包装做不出来；
  **G-62** def/展开不同一 ⇒ `Type.Equiv` 三条等价律写不出来（核心不受影响）；
  **G-63** `Quot.lift` 宇宙实参对不上内核签名 —— ⚠ **实测推翻「Quot 消去只进 Prop」的初判**
  ✗✓（`Quot.lift.{1, 2}` **能**进 `Type` ✓）⇒ 是**实参难对准**，不是缺能力。
- **门禁** ✓：课程 **40 目标 · 360 checked · 99 open · 0 判负**；`notation-lint` OK；
  `scripts/gap.py check` 全部与台账一致（G-56…G-63 八条 open 全带复现件）；
  `scripts/soko gate` **exit 0** ✓。

## 第 489 轮（2026-09-28）：**v0.77.0 分批表收口** —— ST12/ST13/ST14/ST15 四章 + **内核不足清单**

- **用户拍板顺序**「① ST8 → ST10 → **ST12 → ST13 → ST14 → ST15**；② 撞墙的不硬做，
  登记本身就是交付物；③ 卡住的四项放最后、别自己开工」⇒ 本轮把 ① 全部走完 ✓。
- **ST12 选择公理** ✓：新模块 `lib/Choice` —— `axiom choice`（外挂，与 `Set.ext` 同档）+
  `Nonempty` + 2 条展开，**4 条全 checked**。形式照 Isabelle `AC_imp_2`（集合版）。
  ⚠ 从 `choice` **取函数**要在 `Prop` 里取数据 ⇒ **撞 G-58** ⇒ L3/后续。
- **ST13 ZF 公理系统本体** ✓：新模块 `lib/ZF` 7 条全 checked（`axiom regularity` +
  `IsEmpty`/`IsPair`/`IsUnionOf` + 3 条展开）。**核心产出是「ZF 公理表」**（文件头）：
  分离/配对/并集/幂集/替换**都是定理**（谓词就是集合），**只有外延性、正则性、选择必须外挂** ✓
  —— 这是 ST1 结论 ① 的具体体现。
- **ST14 `propext`/`funext`** ✓：新模块 `lib/Extensionality` 4 条全 checked。
  ⚠ **核心判断**：`Set.ext`/`Rel.ext` 在 Mathlib 里是这两条的**推论**，我们两者都没有 ⇒
  那两条一直是**公理**；本档立起来后**原则上可改写**，但**本版不做**（重构 ≠ 补缺口）✗。
  **univalence 不做、也不登记**（更强的公理，非本课必需件）✓。
- **ST15 内核不足清单** ✓（**本版的核心交付物**）：`docs/design/v077-kernel-deficiencies.md`（130 行）——
  **3 blocker**（G-56 `Acc` · G-58 大消去 · G-59 `Type` 值块也消去不到 `Type`）+
  **4 painful**（G-60 花括号 · G-61 没有 η · G-62 def/展开不同一 · G-63 `Quot.lift` 宇宙实参），
  每条带**复现件 + 影响面 + 绕法 + 拦路点源码定位**；卡住的四章（ST6/ST7/ST9/ST11）逐条对照
  「被谁挡住」；**给后来者的动手顺序建议**（G-58 先修 → G-59 同批 → G-56 最后，且要先问用户）✓。
- **v0.77.0 分批表 ① 全部收口** ✓：完成 **9 章**（ST1/ST2/ST3/ST4/ST5/ST8/ST10/ST12/ST13/ST14）
  —— 实为 10 章，其中 ST1/ST2 在第 485/486 轮 ✓。
- **门禁** ✓：课程 **43 目标 · 375 checked · 99 open · 0 判负**；`notation-lint` OK；
  `gap.py check` 全部一致（G-56…G-63 八条 open 全带复现件）；`soko gate` **exit 0** ✓。
- **改内核判定：零** ✓（`git diff crates/kernel/` 为空）—— 用户明确「不许为了让它 checked
  通过去改内核判定」✓。

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

## 当前快照（2026-09-28 · 第 491 轮）

- **v0.77.0 已发布** ✓（tag `v0.77.0`；release `36382099817` **11/11 success**）。
- **本版验收产出物** = `docs/design/v077-kernel-deficiencies.md`（内核不足清单：
  **3 blocker** G-56/G-58/G-59 + **5 painful** G-60…G-64，每条带自断言复现件）。
- **v0.77.0 已完成 10 章**：ST1 · ST2 · ST3 · ST4 · ST5 · ST8 · ST10 · ST12 · ST13 · ST14；
  **ST15** 清单 + **ST16–ST19** 收口 ✓。
- **卡住的四章 = blocked-by-kernel**：**ST6** · **ST7** · **ST9** · **ST11**
  —— 卡在 **G-56**（`Acc`：参数位/指标位两段语义未做）+ **G-58/G-59**（大消去）；
  下标写返回位那条另卡 **G-64**（递归子的宇宙代入）✓。
- **内核零改动** ✓（`git diff crates/kernel/` 为空）；交接书 `docs/HANDOFF-0.77.md` ✓。

## 未决项

- ✅ **清理推送 CI 全绿** ✓（`42be634` ✓ · **绿 28 · 红 0 · skipped 1** ✓ —— 只有 `fast-fail` ✓，
  它只在有失败时才跑 ✓）。⚠ **我先前的预测"预期 `ledger` 红"是错的** ✗✓ ⇒ **已更正** ✓：**本地**
  跑 `gap.py check --strict`（Mac · 50 条串行 · 有负载）⇒ 红 ✗；**CI** 跑（**三片分片** · ubuntu ·
  负载低）⇒ **ledger (1)(2)(3) 全绿** ✓✓ ⇒ ⇒ **台账门禁是环境/负载敏感的** ✗✓（与 **G-25** 的
  「单独 10.1× ✓ vs 门禁里假红」**同源** ✓）；**G-44** 已登记 ✓（`open` + 缺口仍在 = 契约一致 ✓）。
- ⚠ **G-25 是环境敏感项** ✓（**不是回归** ✗）：单独跑 `node …G25….js` ⇒ **exit=1
  「已修」** ✓（冷 162ms / 热 **16ms** · **10.1×**）；门禁里几十条连跑 ⇒ "缺口仍在" ✗
  （冷 5364ms / 热 3641ms）⇒ **时间型判据在负载下失真** ✗✓。

- ⚠ **工作方式四条纪律**（用户 2026-09-26 ⇒ **已进 `AGENTS.md`** ✓）：① 后台落盘 + `timeout` +
  轮询 ② 同一个编译只跑一次 ③ 慢要量出来 ④ 带耗时账 + 推完确认跑绿 ✓（另加「同一处连红 3 次 ⇒ 换招或降级」✓）。
- **耗时账**（纪律④）：front **115s** 墙钟 / 执行 **1.58s**（**98.6% 是编译+链接** ✗）·
  LSP **127s** / **37.3s** · CLI 全量 **>25 分钟** ✗ · `ci-local --fast` **28–197s** ✓ ·
  `--timings` 显示 **Fresh 109 / Dirty 1** ⇒ 慢在**链接** ✗（"少跑测试"省不出来 ✓）。
- ⚠ **CI 假红已修**（`1be86f9`）：`e2e-ledger` 在 e2e 全 skipped 时**假红** ✗ ⇒ `if` 补
  `needs.*.result == 'success'` ✓；记进 `docs/CI-FAILURES.md` ✓（详见该文件）。
- ⚠ **clippy 是本地门禁抓到的**（不是 CI ✓）：`args.len() > 0` ⇒ `clippy::len_zero`
  （本仓 `-D warnings`）⇒ 修完重跑 `ci-local --fast` **30 ✅ / 0 ❌**（197s ✓）才推 ✓。
- **B2 降级为「已定界、可交接」（第 32 轮 ✓）**：拦路的是**记法求解器**（无元变量 ⇒
  前导类型参数在"无信息"时猜出错层级 ✗，`Sort(1)` vs `Sort(2)`）⇒ **设计级** ✗，独立批次 ✓；
  全部证据与结论在台账 **G-43** ✓。
- **B 组**：B3 三刀已落（G-40/G-41/G-42 收口 ✓）· **G-42 已修 ✓**（`9ec3ba3`+`d5b7ac78`）：
  ① 用**解析后的规范名**查签名表 ⇒ `namespace` 里的裸名也触发隐式插入 ✓；② 补"实参**逐位贴合**
  层域"的入口 ⇒ 不抢走 `some Nat` 的旧写法 ✓（判据 + **反向验证**都咬得住 ✓）。
  ⚠ 上一轮试 5 次全撤、这一轮第 6 次才成的差别是消掉两个"假阴"：`Expr` 的 `PartialEq`
  **含 `span`** ✗、`Type` 与 `Sort 1` **同义而异形** ✗ ⇒ 正解 = 比**类型头**（`type_head` 按
  层级归一 ✓）且**不碰显示路径** ✓（第一版用 `render_expr` 被**记法路径守卫**抓到 ✗）。

## 硬事实（接手先读这 5 条 ✓）

- **硬规则**：内核可改，唯一红线是**判定正确性不变** ✓（`REQUIREMENTS.md` §2 ✓ · `docs/architecture.md` §6/§8 ✓）
- **批次制**：一个批次**只 push 一次** ✓（`AGENTS.md` §CI 节奏 ✓）
- **等待流水线不用轮询** ✓：`gh run watch <id> --exit-status` ✓（`docs/CI-FAILURES.md` "轮询不是工作" ✓）
- **性能门禁只跟同宿主的记录比** ✓（`AGENTS.md` §性能回归门禁 ✓）
- **文档预算** ✓：`python3 scripts/docs-lint.py`（活文档 ≤3.0 MB · 入口 ≤800 行 ·
  新设计 ≤150 行 / 既有冻结 · 归档必须被索引点名 ✓ —— `docs/design/docs-diet.md` ✓）
- **本文件受 lint 约束** ✓：`python3 scripts/status-lint.py` ✓（≤200 行 · 禁词 0 · 每段 ≤30 ✓）

---


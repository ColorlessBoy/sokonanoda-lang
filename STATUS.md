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

## 第 485 轮（2026-09-28）：🚀 v0.77.0 · **ST1 ✅**（类型论/外挂分界 —— 决策记录 + 两端对账守卫）

- **变了什么** ✓：**新增决策记录** `docs/design/v077-st1-boundary.md`（89 行）—— 逐项判定
  「哪些用类型论自身表达、哪些确实必须外挂」：① `Set α` 谓词式**够用**（分离 / 无限并交 / 幂集）
  ② **序数写成谓词**（Isabelle `Ord x ≡ Transset x ∧ …`；**全体序数 `ON` 不是集合** ⇒ 本来就不该是类型）
  ③ **基数必须有商**（Mathlib `Cardinal := Quotient Cardinal.isEquivalent`）④ **秩/超限递归两条路都要
  「良基递归可用」**（Mathlib `Acc.recOn` · Isabelle `foundation` **公理** + `wfrec`）。
  **基准逐条带 URL**（Mathlib `Ordinal/Basic` · `Cardinal/Defs` · `Ordinal/Rank` · Lean core `Init/WF`
  · AFP `ZFC_in_HOL` §1.4/§2.7/§2.6 · TPiL §12.4）—— 用户要求「不许凭记忆编」✓。
- **判红（实测原文）** ✓：**两条新的** —— ① 源语言**没有 `Quot`**：`elab-unknown-identifier` /
  「unknown identifier `Quot`」（内核其实**内建** `Declar::Quot` + `Quot.lift`/`ind` 的 iota 归约 ⇒
  缺的是**前端产出**，不是内核）② **`Acc` 立不起来**：`kernel-rejected` /「rejected: inductive
  occurrence is not applied uniformly to the block parameters and universe levels」——**对照组**
  `Even : Nat → Prop` 同形状能过 ⇒ 被拒的是「**下标会变**」，不是「载体是函数」。
- **判据与测试** ✓：**新增** `crates/cli/tests/st1_boundary.rs`（2 个判据）+ **4 个自足复现件**
  `docs/gaps/repro/ST1-*.sokonanoda`（无 `import` ⇒ 单文件判卷）+ **新缺口 G-56**
  （`docs/gaps/repro/G56-acc-well-founded-recursion.sh`，含对照组；`scripts/gap.py check` 全绿 ✓）。
  守卫是**两端对账**：记录里写的诊断必须在探针输出里**逐字**出现，探针输出的每条诊断也必须在记录里
  找到（记录漏记 / 结论过期都判红）✓；另有「`ST1-*.sokonanoda` 与对账表一一对应」的反向守卫 ✓。
- **反向验证两次** ✓：① 把记录里 `Quot` 那行改成 `positive / 7` ⇒ 判红「`decl.checked` 数与记录不符
  （记录 7 / 内核 0）」；② 把 `Acc` 探针换成一条必过的声明 ⇒ 判红「记录 0 / 内核 1」；两次都**撤掉即回绿** ✓。
- **没做** ✗：**ST2（商类型）未开** —— 用户明确「等我对 ST1 的决策记录确认后再开」✓；
  内核判定零改动 ✓、课程内容零改动 ✓。
- **耗时账** ✓：`cargo test -p sokonanoda-cli --test st1_boundary` **0.6 s**（2 判据，跑 4 个复现件）；
  `python3 scripts/gap.py check` 全量 **~1 min**（含 G-56）；`python3 scripts/docs-lint.py` ✓。

## 第 486 轮（2026-09-28）：🚀 v0.77.0 · **ST2 ✅**（商类型 `Quot` 装进源语言 —— 路线 A）

- **变了什么** ✓：`install_quot`（`crates/front/src/compile/prelude.rs`）把 `QUOT_TYPES_SRC`
  五条类型交给**前端自己的 elaborator** 建成 **`Declar::Quot`**（`Quot`/`Quot.mk`/`Quot.lift`/
  `Quot.ind`）+ `Quot.sound`（**唯一**公理，TPiL §12.4）。**内核零改动** ✓（用户核实：
  `quot.rs`、`RigidHead::QuotConst`、`STANDARD_AXIOMS`、按名查找四条全在，缺的只是前端产出）。
  用户拍板**路线 A**（理由见 `docs/design/v077-st1-boundary.md` §3）。
- **判红（修前原文）** ✓：`def mkQuot … := Quot α r` ⇒ `elab-unknown-identifier` /
  「unknown identifier `Quot`」。
- **⚠ 最贵的一课（10+ 轮）** ✗✓：内核 `quot.rs::check_quot` 的 `mk_var(n)` 索引与「按
  de Bruijn 深度推」**不一致** —— 手搓 `EnvBuilder` 表达式结构「看起来对」（`#check` 能渲染
  对的形状），但 `def q … := Quot.{1} α r` 判红「期望 `… $0 …`，实际 `… $2 …`」✗。
  **正解 = 类型写成源文本交给前端 elaborator，只改声明种类** ✓（文本是真的、与
  `prelude_source()` 同源、F12 可用）。
- **判据（放在归约上）** ✓：`docs/gaps/repro/ST2-quot-reduces.sokonanoda`（4 checked，含
  `Eq.refl` 证 `Quot.lift … (Quot.mk …) = f a`）+ `ST2-quot-family-yields`（让位口径）+
  `crates/front/src/compile/tests.rs` 的 `st2_*` **五条** + `crates/cli/tests/st2_quot.rs`
  **两条**；新登记 **G-57**（fixed_in 0.77.0）。**为什么必须在归约上**：装成普通 `Axiom`
  时名字在、类型对、**归约死** ⇒ 只有「算得出来」同时证明「装上了 + 类型对 + 种类对」✓。
- **反向验证** ✓：撤掉 re-kind ⇒ `st2_quot_lift_computes_on_quot_mk` **判红**（`Quot.ind`
  那条仍绿 —— 它靠 `False.elim` 也能过，**这正是"判据要选对那条"的实测**）；
  撤掉 `install_quot` ⇒ `st2_quot_names_are_installed` 判红。
- **ST1 记录随之更新** ✓：ST2 一落地，ST1 守卫**当场咬住**（`ST1-quot-unavailable` 记录 0 /
  内核 1 ⇒ 判红）⇒ 商那一半搬进 ST2 探针，ST1 只留「没有累积性」（L-06，改名
  `ST1-no-cumulativity.sokonanoda`）✓。
- **没做** ✗：**ST10（基数）未开**（用户明确「做完停下，不要顺手开」）· **G-56 本轮不修** ✓。

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

## 第 484 轮（2026-09-27 深夜）：🚀 v0.75.0「跳转与高亮」**E05–E08 全 ✅**

- **E05 + E06** ✓（`3c6ac01`）：**记法声明的目标名 F12 落点**修好（Bug A，一行级）—— `range` 原来用了
  **光标处**那个 span（真定义 span 被 `_` 丢掉）⇒ **原地跳** ✗。真课程库实测：G-37 复现件
  **exit 0 → exit 1**，`Set.powerset` **L125→L81** · `Set.compl` **L126→L79** ✓；单测钉「落点行号 == `def`
  那一行、不许自跳」；反向验证（换回光标 span）报错**逐字一致** ✓。**G-37 → fixed** ✓。
- **E07** ✓（`eee6cc2`）**定案**：① 落点与**编辑器 F12 一致**（同一条通道）⇒ 闭包外**诚实 `null`**、
  **不扩到全仓**（要先编译闭包外模块，代价不成比例）② 高亮**实测已一致** ✓（真库 **11 条**目标名全
  `DefUse`；计划里「3 条 `variable`」是过时描述 ✗✓）⇒ 产出 = **两条防漂移判据**；3 条登记 **G-54**。
- **E08** ✓（`eee6cc2`）**定案：本轮不做**（要做先回答「目标名的『同一个定义』含哪些位置」，三种备选语义
  不同 ⇒ 设计决定，不顺手拍）⇒ **G-55** + 「断言当前行为（`null`）」的防漂移判据 ✓。
- ⚠ **判据自己假绿过一次** ✗✓：E07 第一版只断言「11 条**同色**」⇒ 反向验证（全变 `UnknownIdent`）时
  "同色"**依然成立**、判据照样绿 ✗ ⇒ 补「必须是**已知引用**那一种颜色」后才咬得住 ✓。

## 第 483 轮（2026-09-27）：Infoview 面收口 —— **E27 / E28 / E30 / E31** ✓（v0.74.0 收尾，11/11）

- **E31** ✓（`bc9a62c`）：新命令 **`Clean Cache (清除编译缓存)`** —— **只清不编**（Rebuild 是
  clean→build 串成一步，用户没有"清完就停"的入口 ✗）；三个数逐字来自 CLI 的 `build.clean` 事件；
  判据 stub 40/40 + e2e **实测 N→0 且等 1.5s 仍为 0**（专防"clean 偷偷编了一次"）；反向验证两条各自判红 ✓。
- **E28** ✓（`086c220`）：**主机侧**三态判据（loading / ready-empty / error 分得开）—— 渲染层早有
  判据、**接缝没人验** ✗；反向验证：把失败说成"没有声明" ⇒ 判红 ✓（E28 只补判据、不改行为 ✓）。
- **E30** ✓（`5323889`）：Infoview **「项目」区块**（转发 `soko/project`、**零额外取数**；
  `requires_warning` 是**可见块不是 tooltip** = G-24 的唯一可见信号）；顺手补上
  `audit-wire-fields.py` 的 **front 侧结构解析**（= 审计 #17 的 7 处盲区）并**收紧扫描器**
  （去注释 + 引号内不算读取；⚠ 顺序错会让 `compiled/*.tmp` 的散文卡住 `in_block` ⇒ **整份文件后半段被静默跳过**，
  实测把 `value_runs` 也扫没了 ✗）。
- **E27** ✓（`1444780`）：Infoview **声明名可点 ⇒ 跳到定义**（`executeDefinitionProvider` = 编辑器 F12
  同一条命令；落点与 F12 **逐字段相同**、跨文件 ✓）；反向验证退回 `reveal` 判红 ✓。
  ⚠ **未接的一半**：**记法符号**（`{a}`/`∈`）—— runs 只有 `{text,kind}`（实测 577 条）⇒ 要动 wire，
  登记 **G-53**（open，带自足复现件）✓；⚠ 另实测**声明名位置不返回定义**（本 LSP 解析使用处）⇒ 点名字会
  看到"这里没有可跳转的定义"（诚实，但有用的落点正是被 G-53 挡住的符号）。
- **预算放宽（用户 23:12 拍板：可放宽，但同一 commit 记一笔）**：`STATUS.md` 上限 **200 → 240 行**
  （`scripts/status-lint.py` 的 `MAX_TOTAL`）—— 原因：一轮里落了 6 个环节，200 行顶格后**只能删旧轮**，
  而归档目标 `docs/STATUS-ARCHIVE.md` 也被冻结 ⇒ 实际是"逼着删历史" ✗；`MAX_GROWTH`（≤60）**不动** ✓。
  **后续统一 refactor 时清理**（旧轮搬进 `docs/archive/` 再调回 200）。

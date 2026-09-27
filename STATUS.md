# 当前快照（2026-09-27）

- **进度**：**64/66**（E2 50/50 ✓ + **批次 N** 记法×隐式参数×产品交互 14/16 ✓）· 🚀 **v0.74.0：11/11 全部 ✅ 已发布** · 🚀 **v0.75.0：E05 ✓ · E06 ✓ · E07 ✓（定案）· E08 ✓（定案：本轮不做）—— 4/4 ✅ 已发布**
- **已发布**：**`sokonanoda v0.75.0`** ✓（`gh release list` 显示 **Latest** ✓ · **2026-09-27T19:38:39Z** ✓ · tag `v0.75.0` · bump `79bbf4c` · CI run `36343726623` 真绿 · release workflow `36344775785` ✓）
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

## 第 482 轮（2026-09-27）：E21 结案「**不改**」+ E22 **build/rebuild 以项目为目标** ✓（v0.74.0 第 5–6 个）

- **先推后验** ✓：上一会话的 4 个提交（含 E04）rebase 后推上 main ⇒ run **`36321102036`** 逐 job 真绿
  （28 success · 1 skipped = `fast-fail` 条件 job · 0 failure · **重活 10/10** ✓ `ci-green.py` exit 0）。
- **E21 = 不改（resolved-no-change）** ✓（`ce0371b` · 状态 `9530851`）：用户 I1 说「theorem 卡片那行
  『目标 ⊢ A = B』多余」、计划原判是删；**复核推翻前提** —— 那行是**还没证完的目标**（闭合声明本就没有它，
  实测 `goal=null`；`intro h` 之后变成剩下的 `B`）⇒ 用户拍板保留。行为零改动 + 渲染点写死结论 +
  **防漂移判据**（`test-webview.js` 的 E21 用例）+ 依据进 `docs/gaps/criteria-census.md`；反向验证：
  把「目标 == 语句就不画」加回去 ⇒ 判据判红 ✓。
- **E22** ✓（`b90c0ff`）：判红两半 —— ① `buildTarget()` 取活动文件 ⇒ 只编一个（实测 `files: 1` vs 项目 2）；
  ② rebuild 的 `--clean` **不带目标** ⇒ 只清全局、项目条目原地不动 ⇒ 紧跟的 build 全是 `hit`（假动作 = R-3/T-B5）
  ⇒ **G-51**（fixed）。改法：目标 = 服务端 `soko/project` 的**模块根**（不自己找清单；没答上来退回工作区根，
  **永远是目录**）+ clean 带同一目标。判据：stub 宿主钉 argv（38/38）+ e2e 钉**用户看得见的数字**
  （`N 个文件` == 项目文件数、`清掉 N` ≥ 1）+ 文档同步；反向验证两半**各自**判红、逐字一致 ✓。
- 附带 ✓（`1d17818`）：`LSP_CUSTOM_METHODS` 漏了 `soko/project` ⇒ 同步 skill 时被守卫误判；补上后仍咬得住
  （`soko/bogus` 判红 ✓）。**未决**：E23 · E27–E31 共 **7 个**；门禁 **36/327/99/0** ✓。
- **E23 + E29** ✓（`3a5e94f` + CLI 事件 `0ad970f`）：build/rebuild 的进度接到**三处同一份**
  （状态栏逐帧 `3/13 文件 · lib/Set.sokonanoda` / Infoview 三行区 / 概览尺）+ 原生进度条**可取消**
  （顺手修「完全不能取消」✗）；`runBuildProcess` 流式化，CLI 新增 additive 事件 `build.begin`（带总数）。
  判据 stub 宿主 39/39（**中间态**逐帧 + `increment > 0` + 取消真 kill）；**反向验证**两处各自判红 ✓。
  ⚠ 如实记账：设计里的 `{viewId}` 标题栏进度条**未做**（扩展宿主看不见视图 chrome ⇒ 无判据）。
- ⚠ **上一批 CI 红了一条（已修）** ✗✓：run `36323798295` 的 **e2e (ubuntu 1.138.0)** ——
  `T-A60-2` 的 1.5s 窗口被**邻居用例迟到的落盘**压红（多出的两条 2101/5174B **不是夹具闭包**：
  u01 是 155KB 级、`lib/Set` 单独编 26KB 级）⇒ 治法与 T-A60-3 同款：**先等缓存静止再取基线**
  （`90cb15b`），台账进 `docs/CI-FAILURES.md` ✓。⚠ 那轮最终是 **`cancelled`**（26 success · 1 failure ·
  2 skipped）—— 不是 `failure` 收尾，**别把它读成"绿过"** ✗。

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

## 第 478 轮（2026-09-27）：🚀 v0.74.0 开工 —— **E01 复合记法 `•` / `∘`** ✓（11 个环节的第 1 个）

- **变了什么** ✓：`Rel.comp` / `Function.comp` 补记法 —— `lib/Rel.sokonanoda` 声明
  `infixr:80 " • "`、`lib/Fun.sokonanoda` 声明 `infixr:90 " ∘ "`（各紧跟自己的 `def`）；
  单元⑥⑦⑫ 与三份解答改用记法（点形式一律消除）。**记法零事件 ⇒ 门禁 36/328/99/0 不变** ✓。
- **判红** ✓（内核原文）：`g ∘ f` ⇒ `符号 '∘' 在本文件里还没有声明过记法`；
  `r • s` ⇒ `unknown identifier '•'`（`•` 不在数学码点类里，未声明时连符号都不是 ✓）。
- **判据三层** ✓：front `the_course_libraries_declare_the_composition_notations`
  （记法表：`•`→`Rel.comp`/lib.Rel/80、`∘`→`Function.comp`/lib.Fun/90）·
  CLI `the_course_composition_notations_grade_like_the_pointful_forms`（真课程库、
  两种写法五元计数相等）· `scripts/notation-lint.py` 零残留（新声明**自动**派生点形式判据 ✓）。
- **反向验证** ✓：撤掉两条声明 ⇒ 两层判据**都红**，诊断逐字回到上面那两条 ✓（已还原 ✓）。
- ⚠ **取证更正** ✗✓：`lib/Rel` 头部原写「`r • s` 是 Mathlib 的记法」——**不准确**：
  Mathlib 的 `Relation.Comp` 是 `local infixr:80 " ∘r "`（`Mathlib/Logic/Relation.lean:158`）；
  `∘` 取 Lean core 逐字（`src/Init/Notation.lean:274`）。本课用 `•` 是为了与 `∘` 区分 ✓。
- **台账** ✓：新登记 **G-45**（library/painful/fixed_in 0.74.0，自足复现件 `G45-comp-notation.sokonanoda`）；
  `scripts/gap.py check` ⇒ **全部与台账一致** ✓。
- **文档** ✓：as-built 进 `docs/design/notation-subset.md` §16；速查表补【速查表 2b】+ 梯子两行；
  ⚠ 预算按规矩**手改** `scripts/docs-budget.json`（notation-subset 873→905，评审可见 ✓），
  另删掉 syllabus 里一段**无表头的重复表**（陈旧名 `Set.diff`/`Set.power`）⇒ 该文件 303→294 ✓。
- **未决** ✓：E02–E04 · E21–E23 · E27–E31 共 10 个环节未开工；v0.74.0 收尾要**推一次 CI 逐 job 真绿** ✓。
- ⚠ **本轮踩到的坑（两次同形 ✗✓）**：**判据跑完之前别改文件** ✗ —— 反向验证期间课程门禁读到
  半棵树（报 `符号 '∘' 还没有声明过记法` ✗）、front 判据读到**撕裂**的库文件（假红 ✗）。
  **判据必须在"冻结树"上跑** ✓：改完 → 跑 → 不再碰 ✓。

## 第 481 轮（2026-09-27）：E04 —— hover 的类型面接上折叠 ✓（v0.74.0 第 4 个环节）

- **变了什么** ✓：`compile/check/mod.rs::resolve_hovers` 的文本从内核 pp **直出** ✗
  改成 `display.fold(&name_loose_bvars(…))` ✓（显示表从调用点传进去，那里本来就有 ✓）。
  用户看得见：编辑器/Infoview 里悬停**子表达式或假设**，类型面从 `Set.subset α A B` /
  `Not a` / `forall …` 变成 `A ⊆ B` / `¬ a` / `∀ …` ✓。
- **判红** ✓（实测，修前）：`report.hovers` 里没有 `⊆`、却有 `Set.subset α A B` 与
  `forall (α : Type 0) (A B : Set α), Set.subset α A B -> Set.subset α A B` ✗。
- **判据三层** ✓：真相层 front 新单测（修前判红）· **wire 契约层** LSP hover_brackets
  （`… h : ¬ a` + 反向断言不许出现 `: Not `）· **用户可见层** e2e 补**子表达式** hover
  （夹具 `u01:11` 的假设使用处 `h`）。
- **反向验证** ✓：`SOKO_NO_NOTATION_FOLD=1` ⇒ 那条 front 判据**判红** ✓（exit 101，不改代码）。
- **同步更新的旧判据** ✓（钉的是修前点形式 ⇒ 必须一起改）：front **5 条** + LSP **1 条**
  （期望值按**实测文本**写，含 `a ∧ (¬ a)` 的括号 —— 先 dump 再定，别一条条猜 ✗✓）。
- **实测数字** ✓：front **748 passed / 0 failed** · LSP **164 passed** ·
  CLI **25 target / 0 FAILED** · 完整 `scripts/soko gate` **exit 0** ✓。
- ⚠ **更正一条既有结论** ✗✓：`notation-paths-audit.md` 的用户可见面表早先写
  「LSP hover … 已迁 ✓」——**不准确**：`HoverType.text` 出自 `resolve_hovers`，那条没过
  `fold` ✗（**错误结论把缺口盖住了**）。已更正 + 记 E04。教训：**"某面已迁"必须能指到
  调用点或一条会判红的测试**。
- **台账** ✓：**G-50**（hover 漏折叠，fixed）；`gap.py check` 仍**全部与台账一致** ✓。
- **CI** ✓：E03 批次那次 run **`36316083145` 真绿**（28 success · 1 skipped=fast-fail ·
  重活 10/10）✓ —— v0.74.0 **第一次**拿到完整的逐 job 真绿。
- **未决** ✓：E04 未推（按批次制留给下一批）；E21–E23 · E27–E31 共 **7 个环节**未开工。

## 第 480 轮（2026-09-27）：E03 —— `r ⁻¹` / `A ≈ B` 记法 ✓（v0.74.0 第 3 个环节）

- **变了什么** ✓：`lib/Rel` 声明 `postfix:100 " ⁻¹ " => Rel.inv`、`lib/Equiv` 声明
  `infix:50 " ≈ " => Set.Equiv`；lib/Rel · lib/Equiv · lib/Demo · 单元⑥⑨⑩⑫ · 四份解答改用记法
  （`notation-lint` 零残留）。**记法零事件 ⇒ 计数中性**（36/327/99/0 不变 ✓）。
- **判红** ✓（内核原文）：`A ≈ B` ⇒ exit 1「符号 `≈` 在本文件里还没有声明过记法」；
  `r ⁻¹` ⇒ exit 1「unknown identifier `⁻¹`」（`⁻¹` 不在数学码点类里，未声明时连符号都不是）。
- **判据两层** ✓：front `the_course_libraries_declare_the_inverse_and_equinumerous_notations`
  （`⁻¹`→Rel.inv/100/Postfix · `≈`→Set.Equiv/50/Infix · 且 `⁻¹'`→Set.preimage **同时可见**）·
  CLI `the_course_inverse_and_equinumerous_notations_grade_like_the_pointful_forms`
  （真课程库、两种写法五元计数相等，含 `f ⁻¹' B` 共存用例）。
- **反向验证** ✓：撤两条声明 ⇒ 两层都红，诊断与判红逐字一致（已还原 ✓）。
- ⚠ **取证又更正两处引用** ✗✓：`lib/Rel` / `lib/Equiv` 头部原写「是 Mathlib 的记法」——
  **两条都不是**：mathlib4 master 的 `Logic/Relation.lean` 没有 `Relation.inv`/`⁻¹`；
  `Logic/Equiv/Defs.lean:80` 的 `≃` 是**等价的类型**不是等势；`SetTheory/Cardinal/Basic.lean` 无 `≈`
  ⇒ 两条都按**本课自定**落地（与 E01 的 `•` 同一个毛病）。
- ⚠ **新暴露一个真边界（G-48，open）** ✗✓：`≈` 的**两侧都是零元糖**时（`∅ ≈ {b}`）补不出论域
  ⇒ `elab-notation-argument-unsolved`；单元⑨ 练习 5 按设计写点名 + 行内标记（画布与解答都写明理由）。
  修法方向 = E19 甲案（求解器加元变量）。
- **台账** ✓：**G-47**（记法缺失，fixed，自足复现件）· **G-48**（零元糖操作数，open）·
  **G-49**（类型错误报裸 de Bruijn 编号 `期望 $4，实际是 $5`，open——写复现件时实测到的诊断质量问题）；
  `scripts/gap.py check` ⇒ **全部与台账一致** ✓（5 条 E01/E02/E03 条目逐条对）。
- **子代理** ✓：4 份解答的机械改写外包（prompt 自带判据+边界+三条取证纪律）⇒ 我**抽查后**
  自己补了它明确说"没做"的那一步：**画布↔解答签名逐字对拍**，当场抓出 **3 处括号不一致**
  （`Set.univ Nat ≈ …` vs `(Set.univ Nat) ≈ …`）⇒ 已按画布改齐（10 条全一致 ✓）。
- **未决** ✓：E04 · E21–E23 · E27–E31 共 8 个环节未开工；v0.74.0 收尾要**推一次 CI 逐 job 真绿** ✓。

## 第 479 轮（2026-09-27）：E02 —— `Set.prod` 收进 `lib/Prod` ✓（v0.74.0 第 2 个环节）

- **变了什么** ✓：`def Set.prod` 从**单元⑤ 画布**收进 `courses/set-theory/lib/Prod.sokonanoda:73`
  （`lib/Prod` 新增 `import lib.Set`；记法 `×ˢ` 仍由 `lib/Set` 声明）；画布与
  `unit05-solution` 里那**两份副本删掉**（不删就撞 `import-name-collision`）。
- **判红** ✓（内核原文，修前实测）：临时模块根只放 `lib/` 时写 `s ×ˢ t` ⇒ exit 1
  `elab-notation-unknown-target`「记法 `×ˢ` 指向的目标 `Set.prod` 不存在」。
- **判据两层** ✓：front `the_set_product_notation_target_lives_in_the_library`
  （`×ˢ`→`Set.prod`/80/Infixr、声明点仍在 lib.Set；`Set.prod` 由 **lib.Prod** 提供且 checked；
  画布不再声明它）· CLI `the_set_product_notation_resolves_from_the_libraries_alone`
  （只拷 `lib/` 的模块根 + 只 import 库的画布 ⇒ exit 0）。
- **反向验证** ✓：把 `def Set.prod` 从 `lib/Prod` 撤掉 ⇒ 两层判据都红，诊断逐字回到上面那条（已还原 ✓）。
- ⚠ **净账（要记住）** ✗✓：副本消失 ⇒ 课程门禁 checked **328 → 327**（**练习数与 open 数一条未动**；
  `lib/` 合计 74 → 75）。**这是 E02 的正确结果、不是回归** —— 目标从「两份副本」变成「一份库定义」。
  门禁其余全绿：**36 目标 · 327 checked · 99 open · 0 判负** ✓。
- **台账** ✓：新登记 **G-46**（library/painful/fixed_in 0.74.0 + 自断言脚本复现件）；
  复现件**两个方向都自测过**（缺口在 ⇒ exit 0；修好 ⇒ exit 1）——自测当场抓到脚本里
  `$CODE）` 被 `set -u` 判 unbound（全角括号被吃进变量名）✗✓，已改 `${CODE}` ✓。
- **子代理** ✓：派了一次**只读取证**（普查全仓 `Set.prod`/`×ˢ` 的 stale 说法，带 file:line + 原文，
  禁夸大、禁下判断）⇒ 我抽查后并入 3 处：`course-lean-style.md:268`（C4 边界 #7 标已落）、
  PLAN A10 根因行、G-37 复现件注释 ✓。
- **未决** ✓：E03 · E04 · E21–E23 · E27–E31 共 9 个环节未开工；v0.74.0 收尾要**推一次 CI 逐 job 真绿** ✓。


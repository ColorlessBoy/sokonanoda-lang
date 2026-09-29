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

## 第 500 轮（2026-09-29）：**剖面链路 + perf-gate 判红 + 入口级并行**（HEAD `afc82709`）
- **用户 09:12/09:20/09:22/10:06 四条**全部落地；**内核零改动** ✓ · 工作区干净 ✓。
- **① 剖面链路**（`11cd7272`）：`scripts/profile-course.py`（一条命令 · 四段 · JSON 进
  `docs/perf/`，schema `soko.course-profile/1`，**`profile` 字段在输出里**）· 埋点
  `SOKO_DECL_PROFILE`（逐声明事件，阈值仿 `trace.profiler.threshold`）+ `SOKO_NO_JUDGE`
  （**测量专用**，注释写明绝不许进判定路径）。**第一份数**：**219.3s**（方差 2.7%）·
  `passes` 4126 · 42/0 ✓；**judge ≈ 192s ≈ 88%**（两个 judge 入口都跳 ⇒ **26.8s** / 387 趟）。
  ⚠ **更正旧说法**：`judge_ms` 148.4s 是**嵌套累加** ⇒ "68%"低估；**大头是 `judge_infer`**
  （只跳 `by` 判定只有 **3%**）。
- **假设 A/B/C**：**A 成立**（41 模块 20 个 r>0.5、0 个 <−0.5、均值 **+0.44**）· **B 成立**
  （Top10 占 18.3%，都在后段 ⇒ 与 A 同病）· **C 不成立**（0.64s/入口）。机理 = `judge_infer`
  缓存键含**整段前缀哈希**（`judge.rs:932`）⇒ 后段全 miss ⇒ 前缀从零重跑；judge 合成
  **253513** 次 pass = 自身声明事件的 **95.8×**。
- **② perf-gate 撤"只报不拦"**（`0780f7f4`）：**三层同时失效** —— `continue-on-error` +
  每 case `|| true` + **台账只有 Darwin 记录**（CI 是 ubuntu）⇒ 按宿主过滤后**每条都
  "（无基线）"** ⇒ **撤开关也抓不到东西**（守卫空转）。修法：基线换**同 runner 家族的
  Actions cache**（`SOKO_PERF_LEDGER`，仅 `main` 更新）· 撤两处 · 阈值 50%。**反向验证：
  伪造 10× 快基线 ⇒ `exit 1`** ✓。新纪律进 `AGENTS.md`（435/435 行，**一行没涨**）。
- **③ 入口级并行**（`afc82709`）：`thread::scope` + 原子队列 · worker 自攒结果 · **输出仍在
  主线程按 `files` 顺序重放** ⇒ 确定性部分**逐字节相同** ✓（⚠ `build.tick` 215/107 —— 它带
  `elapsed_ms`，本来就与墙钟相关）。**A/B**：1 job **215.3s** → 4 jobs
  **108.7s**（**1.98×**）→ 8 jobs 108.8s → 10 jobs 125.9s。**核秒**：214.3s → **404.9s** →
  **709.7s** ⇒ **并行只省墙钟不省总功**（judge 两张全局 `Mutex` 竞争）⇒ **4 jobs 是拐点**
  ⇒ **瓶颈是单线程算法（judge 的 O(N²)），不是核数** ✓。
- **下一刀 = 架构重改**（用户 10:49 拍板，**彻底版**）：环境**可增量扩展**，judge 查常量类型变
  **查表**。⚠ **反直觉实测**：前缀复用（`SOKO_JUDGE_ENV_REUSE`）**已实现过**，收益 **< 0.3%**
  （LSP 12.8 万次判卷、命中率 99.6%）⇒ 按"收益不成立就默认关"关掉了 —— **命中率 99.6%
  ⇒ 复用只作用在已经很便宜的路径上**，**88% 全在 miss 上**。四阶段：**0 定接口 → 1 judge
  走环境 → 2 elaboration 主路径 → 3 并行下复用**；每阶段独立 commit + 独立真绿 + 独立回退。

## 第 501 轮（2026-09-29）：**分片否决 · 切片 1 三次失败全勘明 · 主线转 judge 前缀增量**（HEAD `620a0a4b`）
- **详细交接 → [`docs/HANDOVER-slice1.md`](HANDOVER-slice1.md)（162 行）+
  [`docs/design/p1a-measurements.md`](design/p1a-measurements.md)（320 行附录）**；本文只留结论。
- **① 分片实测否决 + 精确 revert**（`89b91fa6`）：单片冷跑 **317.71s** ≈ 全量 **313.78s**
  （期望 ~78s）⇒ 切的是"目标数"、切不掉共享 `lib/*` 闭包重复编译。**保留**课程产物缓存
  （冷 313.8s → 热 **0.50s**，**628×**）与 `ci-green.py --selftest` 夹具修复（**3/6 → 6/6**）。
- **② 切片 1（一次 session 覆盖全部入口）三次接线全失败，根因**：
  ① 报告拼接越界崩（`project/mod.rs:480`）② 把 `lib/*` 当入口 ③ **session 入口趟拿不到闭包前缀**
  （`judge_infer` 只吃源码字符串 `judge.rs:949`）⇒ 全部修掉（`9543405a` 前缀取**最后一格**是关键），
  **但 17:36 实测 `passes 4141` / `judge_ms 146.9s` ≈ 基线 ⇒ 不提速** ⇒ **接线已撤、切片 1 挂起**。
  零件留 main（`assemble_from_session`/`merge_session_reports`/`precheck_plan`/`closure_prefixes_for` + 内核两笔）。
- **③ 🎯 主线转「judge 前缀增量」**（用户 17:52「内核层级编译优化势在必行」）。**权威读数**
  （release · 冷缓存 · 1 job · 墙钟 **215.0s**）：`JUDGE_INFER total_ms **247.4s**` ·
  `misses **3759**` ⇒ **未命中 ≈ 222.6s（90%）** · `JUDGE_PREFIX runs=3759 bytes=**1.74 亿**`
  （**结构判据**，噪声免疫）· **3759 趟只对应 488 个前缀（7.7× 重复）** · `judge/墙钟 ≈ 68%`。
- **④ 新增量具**（`b9ee531d`，**只加计数、不改判定**）：`JUDGE_PREFIX runs/bytes` +
  `SOKO_JUDGE_CLASSIFY` 分桶。**⑤ 两个假口径主动作废**：`all_miss_ms 235.7s`（**> 墙钟** ⇒
  并发重复计时）· `SOKO_NO_JUDGE=1` 26.17s（实测 **`compiled:1 failed:41`**）。
  **⑥ 靶子重定**：「裸常量就地查表」判死 ⇒ 真靶子 = **那 3759 趟前缀重跑本身**。
- **下一刀**：让前缀不再重跑（per-前缀 builder 池 / judge 接收调用方 builder）；
  ⚠ 真障碍：`compile_fol_with`（`check/mod.rs:286`）每次**从零造 `EnvBuilder`**，而它不是 `Clone`。

## 第 502 轮（2026-09-29）：**P1-a 第一步落地 —— 就地判定接在一个判定点上**（🚀 v0.78.1 · 默认已开）
- **按调用点归因 3759 趟**（全量 `INFER_MISS` 回溯，`/tmp/base-trace.txt`）：**`infer_type_text`
  一个判定点 = 2697 趟（72%）/ 120.5 MB（73%）**；其余 `lower_value` 536 · `universe_level_text_of_operands`
  468（第二问）· `judge_render_type` · `application_arg_expected` 34 · **`infer_expected_level` 只 18 趟（0.5%）**
  ⇒ 附二 §E 原定的"判定点"**被数据判死**，真判定点是 `infer_type_text` ✓。
- **落地**（`crates/front/src/judge.rs` + `compile/elab.rs`，开关 `SOKO_JUDGE_INPLACE=off|shadow|on`，**默认 off**）：
  把 `EnvProvider` 的意图**反向**接 —— 判定**就地**用调用方手里的 `&mut EnvBuilder` + `KnownTable`
  （`InplaceEnv`）elaborate **源 AST**（不 render/回读、不碰前缀），再走
  `ExportFile::infer_type_text_at`（内核里那个**零调用点**的零件首次接线）；
  **只做未命中**（命中仍走今天 ~11µs 的哈希快路 —— 每题都就地是**负优化**，实测过）。
- **判据**：`shadow` 档单文件 `unit12-solution` **`shadow_same=139561 shadow_diff=0`** ✓；
  反向判据**有实测**（`crates/front/tests/judge_inplace_on.rs`：换依赖里一个声明的类型 ⇒
  结论变 **且** `INPLACE_USED` 增长）；两个测试都断言**判据不空转**（就地路径真被走到）✓。
- **三个实测坑**（都写进注释）：① pp 默认 `proofs=false` ⇒ 对每个子项 `is_proof`（空上下文）
  ⇒ `loose bvar in infer` panic（必须与 `kernel_phase` 的 `#check` 同档设 `proofs=true`）；
  ② `parse_expr_text` 不认识前缀里声明的**源级记法** ⇒ 单文件 77822 次 Parse 失败；
  ③ "接上前缀再解析" ⇒ 每问 46 KB、而对**每次调用**生效 ⇒ 400s 跑不完 ⇒ 改"不解析"。

- **全课程实测（release · 冷缓存 · 1 job · `build --json courses/set-theory`）**：
  `--json` 剔 `build.tick` **逐字节相同**（2691 行 / 0 行不同；`build.decl` 2647 ·
  `build.file` 42 · `build.begin` 1 · `build.summary` 1 两边一致）✓ ·
  `passes` **4126 → 2248（−45.5%）** · `doc_passes` 266 → 266 ✓ ·
  `JUDGE_PREFIX runs` **3759 → 1881（−50.0%）** · bytes **1.74 亿 → 9386 万（−46.1%）** ·
  `judge_ms` **146.6s → 120.4s** · `used=1878 fallback=39`（2.0%）·
  **墙钟 214.19s → 158.90s（1.35×）** ⇒ **默认已开**（`SOKO_JUDGE_INPLACE=off` 是回退开关）·
  账对得上：`used(1878) + runs_on(1881) = runs_off(3759)` ✓。
- **下一刀（不是终点）**：仍剩 1881 趟前缀重跑（`by` 路径 536 · `universe_level_text_of_operands`
  第二问 468 · 未接线小点）；**真正的大头是"前缀环境可保存/恢复"**（`with_env_scope` 那条腿），
  与"就地判定"是**两条腿**、不互相替代。

## 第 503 轮（2026-09-29）：**P1-b 第一档 —— 1881 趟逐条归因 + 两个未接线点**（暂存开关默认关）
- **先量后动**：给 `judge_infer*` 加 `#[track_caller]`，`INFER_MISS` 行多打 `at=file:line`
  ⇒ **1881 趟逐条归因到行**（附八表）：`infer_type_text` 的 `slow()` 闭包 **819 趟（44%）**
  = 两个**未接线**判定点 · `universe_level_text_of_operands` 第二问 468（**文本输入**，先放着）·
  `by` 路径 536（`judge_render_type` 320 + `by.rs` 216）· 其余小点 58。
- **本档只做 819 那两条**（`guarded_binder_type` / `solve_prefix_args` —— 各只有一个调用方，
  照抄 P1-a 的 `InplaceEnv` 形状，多传一级签名）+ `SOKO_JUDGE_INPLACE_WIDE`（**默认关**）。
- **读数（release · 冷缓存 · 1 job · 全课程）**：`--json` 剔 `build.tick` **逐字节相同**
  （2691 行 / 0 行不同；`compiled:42 failed:0`）✓ · `JUDGE_PREFIX runs` **1881 → 1086（−42%）** ·
  bytes **9386 万 → 5457 万** · `passes` **2248 → 1452（−35%）** · `judge_ms` 120.4s → 111.8s ·
  `used=2641 fallback=50` · shadow（wide）**diff=0** ✓ · **墙钟 158.90s → 134.77s**
  （**vs off 216.14s = 1.60×**）✓。⚠ `doc_passes` 266→265（计数口径差 1，输出逐字节相同）。
- **判据不空转**：证据是结构性的 —— `runs` 掉 795 ≈ 新增接线点的作答数（接线死掉不会动）✓。

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


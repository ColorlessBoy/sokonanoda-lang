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

## 第 508 轮（2026-09-30）：**§3.C「前缀环境」落地 —— 全课程 `build` 2.65×**
- **先勘后动**（开工单 §3.C 明写要求）：让勘的**两条路都够不着**那 791 趟
  （候选① per-前缀 builder 池 = §17 的 B″ 同死因：arena 生命周期；候选② judge 收调用方
  builder = 借用可行但**前缀位置不同**）；**我据此推断的第三处（提升 arena 作用域）
  也被逐趟明细推翻** ✗ ⇒ 真刀口是「**judge 每次都从源码重编整个前缀**」。
- **真数字**（`SOKO_JUDGE_STATS=2`）：`by` 路径 **265 趟 = 115.9s = `judge_ms` 的 99%**，
  而 `judge_infer` 的 71126 次只 38.5s（P1-b 已吃干净）；那 265 趟 **key 全不重复 ⇒ 加缓存没用** ✗。
- **刀口**：把既有的 `TRUSTED_PREFIX` **接到主编译 pass**（`walk.rs` 每检查完一条命令压栈担保）。
  ⚠ **关键一行 `before.min(prefix_commands)`** —— 两个坐标系不同（AST 序号**含 `import`**，
  judge 前缀文本走 `importless_source`）⇒ 不夹会**多担保合成声明** ⇒ 判定声明没被检查
  （`--json` **38 行不同**，与 P1-b 第一次失败同签名）✗。
- **读数**：墙钟 **126.4s → 47.8s（2.65×）** · `judge_ms` 113,659 → **19,223（−83%）** ·
  `pass_total_ms` 276,407 → **92,058** · `by_calls` 不变 ✓。
- **证据链**：影子档（**判据级**，两条路都跑）`shadow_same=265 · diff=0` · `--json`
  **0 行不同** · 反向判据（去掉夹紧 ⇒ 判红）· 带开关跑完整 `gate` PASS · `grade` 错误路径同诊断 ✓。
  ⚠ 影子档第一版比**整份报告** ⇒ 265/265 **假分叉**（差的是报告**范围**不是**判定**）✗ ⇒ 改比**判据**。
- **默认开**（收益成立才开）；逃生门 `SOKO_JUDGE_ENV_VOUCH=0` / `SOKO_JUDGE_ENV_REUSE=0`。
- 🚀 **`v0.78.3` 已发布并闭环** ✓：CI `36655613663` **28 success / 0 failure**（1 skipped）·
  tag `v0.78.3` → release workflow `36656518982` success · `gh release list` **Latest** ·
  26 个资产（8 CLI + 8 LSP + 9 VSIX + 1 源码）✓。
  ⚠ **CI 侧也看得到提速**：`gates-course` **12m26s → 6m38s**（同一 job、同一 runner 家族）✓。

## 第 509 轮（2026-09-30）：**K1 线收尾 —— K1-a 两态终于有判据 + K1-b 的 `decl_idx` 钉子**
- **`by-prefix-reuse.md` 落后于工作区**：K1-a（T-K11）与 K1-b 的内核那一处（`with_env`，T-K12a）
  都早已落地、§3.C 后复用**默认就是开的** ⇒ 本轮正体是**补齐 §4 的验收**（内核 0 行改动）。
- **勘出一处假声明**：§4 写「两态对拍 `assert_same_both_ways` 已具备」——**错的** ✗：那条比的是
  `SOKO_NO_JUDGE_BATCH`，与 `SOKO_JUDGE_ENV_REUSE` **无关** ⇒ 这条验收**此前没有判据**。补
  `crates/cli/tests/judge_env_reuse.rs`（两态逐字节 + **不空转**）；反向验证（去掉 §3.C 的夹紧）⇒ **判红** ✓。
- ⚠ **新坑**：**模块根产物不受 `SOKONANODA_NO_CACHE` 管** —— 一热就跳过整份编译 ⇒ 两态「相同」
  是**空转的相同**（164 命中/23s vs **895 命中/283s**）⇒ 判据必须带 `…NO_PROJECT_ARTIFACTS=1`。
- **读数**：全语料两态 **128 文件 · 逐字节差异 0 · 命中 895 · 283s** · `unit12-solution` **7641ms → 6107ms（1.25×）**
  ⇒ 进 `docs/perf/ledger.jsonl`，并**更正** 2026-09-24 那条「K1-a 零收益」（只对「担保没接到主编译 pass」的那天成立）。
- **K1-b**：补 §4 要求却一直缺的 `decl_idx` 钉子（`memory_api.rs` 的
  `cross_builder_name_lookup_is_silently_positional_without_with_env`；反向验证 ✓）。
- **验收五步**（baseline = v0.77.0）：① workspace 全测 ✓ · ② 全语料逐字节 **零差异（9 组）** ✓ ·
  ③ 课程门禁 **43 目标 · 376 checked · 99 open · 0 判负** ✓ · ④ 性能台账 ✓ · ⑤ **明确跳过**（`LEAN_KERNEL_ARENA` 未设）。
- **文档**：`by-prefix-reuse.md` §6 as-built · `docs-budget.json` 153 → **183**。

## 第 507 轮（2026-09-30）：**CI 判红两轮收口 —— 两个真 bug 都是本批引入的**
- **① 心跳口径**（`gates-fast` 判红）：第一版"默认一律不发"漏了工作单的**后半句**
  「机器消费者仍能拿到心跳」✗ ⇒ 改成**「非管道不发」**（`IsTerminal`：终端静默、
  管道照发 1s，契约不变）✓。教训：**改"发给谁"时不许顺手改"发多快"**（周期是契约）。
- **② `Heartbeat::stop()` 等满一个周期**（`test (sokonanoda-cli, tests)` 判红）：
  `join()` + 线程在 `sleep(period)` 里 ⇒ **每次 build 白等 up to 1s**
  （线索：冷热**都** ~1025ms，**对称**得不正常）✗ ⇒ `Condvar::wait_timeout` ✓。
  定位法：`git worktree` 逐 commit 二分 ⇒ 一次锁定 `291cae9f`。
  修后 `perf_project` **cold 72.8ms / warm 4.7ms**（比值 15×）。
- **③ `judge_inplace_by` 间歇判红（约 2/3）**：**我自己的测试写错了** —— 同文件两条
  `#[test]` 共享进程、抢同一个 `OnceLock`（而头注释写着"天然隔离" ✗）⇒ **一个档位一个文件**；
  **6 连跑全绿** ✓。按纪律**横向排查**（全仓无同类）+ 落守卫
  `scripts/check-test-env-isolation.py`（进 `gate` 与 CI，`--selftest` 4 夹具，
  反向验证：复原事故形态 ⇒ 判红 ✓）。
- **顺带**：`gate --fast` 写死 `--lib` ⇒ 改了 `crates/cli/`（bin crate，无 lib target）
  **必红** ✗ ⇒ 按 crate 分开选（`--lib` / `--bins`）。
- ⚠ **元层教训**：①② 两版**各引入一个不同 bug**，而都过了 `gate --fast` ——
  因为 `--fast` **跳过集成测试** ⇒ **改了 CLI 运行时行为要跑那个 crate 的 `--test` 全集**。

## 第 506 轮（2026-09-30）：**P1-b 收口 + P1-c 双数字 + Q1/Q2 · 🚀 v0.78.2 已推**
- **P1-b 第二刀（`by` 路径）**：第 1 步（`canonical_goal_type`）曾因判定分叉回退
  （**附十**勘明根因）；本轮按附十的切口重做 —— **项层面剥 Pi**
  （新增 `ExportFile::infer_type_text_at_peeled`）+ **带 scope 的 pp**
  （`with_pp_scoped`，否则松散变量印成 `$3 $2 $1`）⇒ **影子档全课程 `diff=0`** ✓；
  第 2 步铺开另三处（`canonical_goal_with_spec`）—— 附九说"要先解决规格 → 源 AST"，
  **实测不需要**（`GoalNode` 本就存着源 AST binder，`context_binders` 直接可用）✓。
- **P1-b 收口**：`WIDE` + `BY` **并进主开关**（默认全开，各留单独回退与 `shadow` 取证）
  ⇒ `JUDGE_PREFIX runs` **3759 → 791（−79%）**；红线：`off` vs 默认的 `--json`
  （剔心跳/进度）**逐行不同 0 行** ✓ · 影子档 **`shadow_same=44234` · `shadow_diff=0`** ✓。
- **P1-c 双数字**（各 3 轮中位数）：**214.08s → 126.80s = 1.688×**（极差 0.20%/0.28%）✓
  —— ⚠ 计数降 79% 而墙钟只降 41%，**两者都对**（省的是前缀重跑，固定开销与编译本身不变）。
- **Q1**：`sokonanoda.build.timeoutMs`（默认 300000，**0 = 不限制**）+ 超时消息写明去哪改；
  **值**判据（旧守卫只锁"有常量 + 会 kill"这个形状，**从没断言值够不够用** ✗）。
- **Q2**：`sokonanoda.installCli`（**自带即装**、离线、不校 SHA256）+ **值**判据
  （**跑 `--version`** 与插件版本比 —— Q2 之前"版本对齐"是一句**没有守卫的声明** ✗；
  反向验证：注入自述 0.74.0 的陈旧二进制 ⇒ 判红 ✓；**bump 时它当场抓到 `bin/` 还是 0.78.1** ✓）。
- **🚀 `v0.78.2`**：`bump.py` + `CHANGELOG` 手写；按推送三步（fetch → `gh run list` 无在飞
  → `pull --rebase`）**一次推**；`pre-push` 三层守卫（本地快层 / 时序证据 / 无并发 run）全过 ✓。

## 第 504 轮（2026-09-29）：**文档收敛 —— 计划/交接类 11 份删除，接手入口只剩一份**
- 用户口径：*「一堆 HANDOVER 都可以删了，没必要」* + *「没建立起来很好的 doc 清理机制」* +
  *「为什么需要两个文件」* ⇒ ① **删**（**不归档**：`PLAN-0.74-0.79` · `HANDOVER.md` ·
  `HANDOVER-slice1.md` · `HANDOFF-0.74…0.78` · `HANDOFF-session-2026-09-27` · `E2-HANDOVER` ·
  `PLAN-appendix-goal-and-antifragile`，共 **11 份**）② **`docs/NEXT.md` 与 `docs/ONBOARDING.md`
  合并为一份**（保留被门禁/引用锚定的 `ONBOARDING.md`；它现在 = 必读表 + **开工单**
  〔现在在哪 / 下一步 / 判据 / 纪律 / 续做 prompt〕）③ 新增**清理机制** `scripts/docs-gc.py`
  （**报告式**：零引用 / 不在权威链 / ≥30 天没动 ⇒ 报候选，**不自删**；首次跑报 7 个候选）。
- **被删文件里必须活下来的东西已并入 ONBOARDING**：推送节奏四条（中间环节不推 CI · 攒到发版点
  一次推 · 不 `--watch` · 不连推 —— **本会话违反了前两条**，如实记）· P/Q/E 组状态 · 零件清单结论。
- **判据**：`docs-lint ✓`（活文档 411 → **399** · 6.26 → **6.05 MB**）· `status-lint ✓` ·
  `plan.py check ✓`（66 环节）· `gap.py check ✓` · **`cargo test -p sokonanoda-cli --test skill`
  4 passed**（它咬住过一处：skills 里引用的 `docs/HANDOVER.md` 已删 ⇒ **守卫工作正常** ✓）。
- ⚠ 遗留：`docs/design/**` 与 `ROADMAP.md`/`REQUIREMENTS.md` 里仍有指向已删文件的**文字引用**
  （无判据咬，已记入 `docs-gc.py` 的后续处置）。

## 未决项

- ✅ **用户实测 2026-09-29「编译 UI 与功能没联动」三条已全部收口**（第 505 轮，各带
  **反向验证过**的判据）：③ 版本戳陈旧（`3e24ad24`）· ① `build.tick` 刷屏（`aecf2804`）·
  ② Rebuild `0%` 突跳（`eee2d9ed`，真宿主复现 → 修 → 真宿主 36/36）。**无遗留。**
- ⬜ **P1-b 剩余：`by` 路径 536 趟**（`p1a-measurements.md` **附九**勘明管道、**附十**勘明
  第 1 步为什么分叉 —— **两篇都先读，别重勘**）。2026-09-30 已试过一次第 1 步并回退：
  **根因** = `peel_binders` **失败时是 `break` 不是 `Err`**，而它第一步的 `parse_expr_text`
  **不认源级记法** ⇒ 就地路（拿**原始** pp 文本、要自己剥 `n` 层）**一层都没剥**。
  **切口已定**：就地路**别走文本剥层**，改在**项层面**剥 `n+1` 层 Pi 再 pp ⇒ 与慢路天然同构。
  然后：shadow + 全课程对拍 → 再铺 `canonical_goal_with_spec` 三处（只拿得到**渲染文本**
  的 binder ⇒ 要先解决"规格 → 源 AST"）→ 最后并 `SOKO_JUDGE_INPLACE_WIDE`。
  ⚠ 附十还有**三条实测死路** + 两条纪律 + 必须重加的 `JUDGE_INPLACE_BY used=/WHY` 计数。
- ✅ **P1-b / P1-c / Q1 / Q2 全部收口**（第 506 轮），**`v0.78.2` 已推**。
- ⬜ **前缀环境可保存/恢复**（`with_env_scope` 那条腿，开工单 §3.C）—— **下一刀**：
  目标 `runs` 从 791 再**大幅**降（剩下的是**编译本身**，不是前缀重跑）。
  开工前先勘"per-前缀 builder 池 vs judge 接收调用方 builder"两条路的借用/生命周期。

## 硬事实（接手先读这 5 条 ✓）

- **硬规则**：内核可改，唯一红线是**判定正确性不变** ✓（`REQUIREMENTS.md` §2 ✓ · `docs/architecture.md` §6/§8 ✓）
- **批次制**：一个批次**只 push 一次** ✓（`AGENTS.md` §CI 节奏 ✓）
- **等待流水线不用轮询** ✓：`gh run watch <id> --exit-status` ✓（`docs/CI-FAILURES.md` "轮询不是工作" ✓）
- **性能门禁只跟同宿主的记录比** ✓（`AGENTS.md` §性能回归门禁 ✓）
- **文档预算** ✓：`python3 scripts/docs-lint.py`（活文档 ≤3.0 MB · 入口 ≤800 行 ·
  新设计 ≤150 行 / 既有冻结 · 归档必须被索引点名 ✓ —— `docs/design/docs-diet.md` ✓）
- **本文件受 lint 约束** ✓：`python3 scripts/status-lint.py` ✓（≤200 行 · 禁词 0 · 每段 ≤30 ✓）

---


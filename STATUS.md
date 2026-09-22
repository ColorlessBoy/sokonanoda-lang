# 当前状态与进度日志（agents 先读这里）

> 快照：2026-09-21（第一百二十三轮：**G-34 记法消解重编前缀** —— 与 G-31 同一个病
> 的第二个入口；T-K22 落地，`unit12-solution` 11.4s → 7.5s；版本 **0.64.1**）
> 仓库：`sokonanoda-lang`；权威计划 = `ROADMAP.md`；**用户要求总账 = `REQUIREMENTS.md`（先读）**；
> **文档地图 = `docs/README.md`**（入口/权威在仓库根，开发者参考在 `docs/` 顶层，
> 设计在 `docs/design/`，调研笔记在 `docs/notes/`）；
> 架构/内核 = `docs/architecture.md`；协议 = `docs/protocol.md`；测试地图 = `docs/TESTING.md`；
> 经验台账 = `docs/LESSONS.md`；CI 失败台账 = `docs/CI-FAILURES.md`；发布 = `docs/RELEASE.md`；
> agent 入口 = `AGENTS.md` + `skills/`；**harness 适配 = `docs/design/deepseek-harness.md`**；
> **agent 查询通道 = `docs/design/agent-query-channel.md`**（ROADMAP I15）。

## 一句话

`.sokonanoda` = **纯声明式教学文件（无 `#` 命令）+ 完整 sokonanoda 内核 + LSP 反馈通道**。
练习 = 带 `sorry` 洞的 `def name : T` / `theorem name : T` / `example : T` 声明。
CLI/REPL 的 `#check` 等只是调试/自测工具，不是文件格式。

## 本轮进度（2026-09-21，第一百二十三轮：**记法消解也在重编前缀** —— G-34，`judge_infer` 那一刀）

> 用户：「我还是很疑惑，lean 的 by 风格有这么耗时吗？是不是我们的 by 的实现方案
> 有问题呢？」「那要插入方案进计划里，这个违背我们的红线，也违背我们的热编译的
> 设计初衷。」「前端有问题，前端也一起配合改掉，这属于重大事故的 bug。」
>
> 上一轮把 `solutions/` 改成项风格之后，`by` 判定掉到 11 次 / 0.9s——
> **但 `unit12-solution` 仍然要 11.4–12.0 秒。大头换了人。**

1. **先定位"这几百趟 pass 到底是谁在调"**：新加两个**常驻**诊断开关
   ——`SOKO_PASS_TRACE=<n>`（第 n 趟 `run_pass` 的调用栈）与
   `SOKO_INFER_TRACE=<n>|all`（每次 `judge_infer` 未命中的查询 + 栈）。
   第一次就量出：**380 趟 pass 全部来自 `elab_notation` → `solve_prefix_args`
   → `infer_type_text` → `judge_infer`**。
2. **缺口 G-34 成立**：`judge_infer` 的缓存键**含整段前缀**，前缀随每条声明
   增长 ⇒ 同一批查询每次换一个键；**未命中一次 = 合成 `<前缀>#check fun
   (binders) => term` 把整段前缀从零重跑一趟 pass**。实测
   **126,105 次调用 / 363 次未命中**（`unit12-solution`，release，冷缓存），
   `judge_infer` 累计 12.3s。这与 G-31 是**同一个病、不同入口**。
3. **命中侧不是问题，别打错靶**：`JUDGE_INFER_SPLIT hits=50909 misses=363
   key_ms=741 hit_ms=744` ⇒ 5 万次命中只花 744ms，**优化必须打未命中**
   （即"别问内核"），不是打哈希。这条读数纪律写进了 `docs/PERF.md`。
4. **T-K22 那一刀（已落地）**：`elab.rs` 新增 `operand_type_expr`——
   **局部变量先取 `ElabScope::source_type_of`（书写类型，零内核调用）**，
   拿不到才退回 `infer_type_text`。判据与 `implicit.rs` 里 `arg_tys` 的
   **既有判据完全相同**（书写类型不但零调用，还比内核 pp 更准——pp 会丢嵌套
   常量的隐式实参）。三个入口同时换：`solve_prefix_args`（主）、
   `guarded_binder_type`、`arg_tys`。

   | 指标 | 改前 | 改后 |
   |---|---|---|
   | `judge_infer` 调用 | 126,105 | **51,156** |
   | 未命中（= 整段前缀重编一趟） | 363 | **247** |
   | `judge_infer` 累计 | 12.3s | **6.9s** |
   | **墙钟** | **11.4–12.0s** | **7.5–8.4s** |

5. **这是缓解，不是根治**（计划里明写）：剩下的 247 次未命中来自冗余 `sorry`
   探针（`fun (__soko_render : T) => __soko_render`）、`And.intro` 这类**裸常量
   头**、inductive 安装与闭包里的记法——**没有局部类型可拿** ⇒ 只能靠 T-K20′
   的「就地拿当前 pass 的环境」，而那套设施要**同时**覆盖 `judge_pairs` 与
   `judge_infer`（已写进 `by-judge-reuse.md` §7、计划 §5.6.1 T-K20′、REQUIREMENTS §9）。
6. **判据**：`scripts/kernel-diff.sh` 全语料逐字节对拍 **零差异** · `cargo test
   --workspace --locked` 全绿 · 课程门禁计数不变 · 缺口复现
   `docs/gaps/repro/G34-notation-type-query-recompiles-prefix.sh` 已进
   `gap.py check`（gate + CI）。
7. **顺带修掉一条会随机翻红的复现**：G-29 的判据 `edit*2 < cold` 余量只有 ~13%
   （4413ms vs 2489ms），机器一抖就翻面 ⇒ `gap.py check` 随机红（本轮实测翻过
   一次）。冷开里混着**进程启动**，本来就不该进分母；改成与**热开**比
   （`edit < 10×warmOpen + 200ms`，实测 2582ms vs 预算 280ms，余量 9×）。
8. **不变的**：内核**一个字节未改**（这一刀全在前端）；不调用官方 Lean 工具链；
   用户/agent 路径仍零 cargo。版本 bump 到 **0.64.1**（patch：纯提速，无新能力）。

## 本轮进度（2026-09-21，第一百二十二轮：**课程记法规则**重建 + 基础类型隐式实参对齐 Lean）

> 用户两条指令：「重新设置一个 courses 的规则，至少 notation 都要换掉，lib 和正文都
> 换掉，不要有些还是老版本的。你先实现一个检查脚本，然后一个文件一个文件过。tactic
> 还比较费时……可以先保持一部分的 term」；「基础类型的隐变量也可以尝试和 lean 对齐。
> ……`Eq.{1}` 直接就是一个等于号」。

1. **规则成了脚本**：新增 `scripts/notation-lint.py`（覆盖卷 I 的 lib + units +
   solutions、入门课 `course/`、`playground.sokonanoda`；**代码与注释都算**；
   `notation-cheatsheet*` 整文件豁免；行内 `-- soko:notation-ok: <理由>` 豁免；
   `--json`/`--list`/`--root`）。接进 **`scripts/soko gate`** 与 **`ci.yml`**
   的 `test` job。施工手册 `docs/notes/course-lean-style/notation-rewrite-brief.md`，
   as-built `docs/design/course-lean-style.md` §11。**当前 `84 个文件 · 0 残留`**。
2. **语言侧（`crates/front`，内核零改动）**：prelude 的 `And/Or/Iff/Not/False/absurd`
   与**构造子**改成隐式前导参数（`And.intro h1 h2` / `And.left h` / `Or.inl h` /
   `Exists.intro w hw`……）；`KnownName` 存**源级签名**（应用路径不再 `judge_infer`
   ⇒ 消除 prelude 自举无限递归）；`PreludeInstallGuard`；「实参 > 显式层数」判为旧式
   写全、一次装完（向后兼容）；`arg_tys` 优先取局部变量的**书写类型**（修 `Eq` 合取）；
   `telescope` 把签名参数**换成 fresh 名**（修名字捕获，`congrArg` 在
   `theorem (α β) (g : β → α)` 下的判红）；路线 ② 增**期望类型反解**并对
   期望/实参类型做 **delta 展开**（`a ∈ A ∪ B` → `Or …`，修 `Or.inl h`/`And.left h`
   在 def-headed 目标上的大头）。回归：front **670 passed**、notation **47 passed**、
   LSP **146 passed**。
3. **课程一个文件一个文件过完**：卷 I `lib/`（9）+ `units/` 与 `solutions/`（24）+
   卷 I 记法对照页豁免；入门课 `course/`（50 文件）；`playground.sokonanoda`。
   **判据**：卷 I 门禁 **36 目标 · 328 checked · 99 open · 0 判负**（计数中性）；
   入门课 `checked 56 · open 66 · failed 0`；playground `decl.checked 23 ·
   example.checked 2 · exercise.open 4`（+1 条既有 `Prop` warning）。
4. **明说的边界**（不假装已对齐，全部有 `-- soko:notation-ok` 标记）：宇宙多态的
   等式族**证明项**（`Eq.refl/symm/trans/subst`、`congrArg`）仍要显式宇宙与参数
   （应用路径的宇宙层级推断未做）；`congrArg` 参数顺序按 Lean 改成
   `{α β} {a b} (f) (h)`（**契约变更**，旧顺序判红）；`Set.univ α`；`by rfl` 读源 AST
   不认 `=` 记法目标；若干 def-headed / 复合操作数 / 嵌套 `Exists.elim` 形状。
   **标记数：卷 I lib 9 · units 401 · 入门课 50 · playground 5**（def 展开修复后
   正在回收）。
5. **不变的**：内核一个字节未改；不调用官方 Lean 工具链；用户/agent 路径仍零 cargo。
6. **收尾两条**（同日）：① `rfl` 认 `=` 记法目标（`by.rs`/`judge.rs` 三处：绑定名捕获、
   `canonical_goal_with_spec` 的补层级顺序、`is_rereadable` 在部分应用上误判 `Eq`
   元数）——入门课 6 处标记随之收回；② **Infoview 声明卡片跟随活动文档**（用户报的
   bug）：`trackEditor` 在切文档时主动取一次 `soko/goals`，不再只靠"树可见才
   `getChildren`／等诊断"（项目模块不发诊断、树收在侧栏时卡片会停在上一份文档）。
   实测：stub 宿主 **29/29**、真 VS Code e2e **16/16**（`docs/e2e/`）。版本随之
   bump 到 **0.63.0**（语言批 + 扩展修复同一 tag）。

## 本轮进度（2026-09-21，第一百二十一轮：`by` 块判定的**根因**修复 —— 每步重判整份文档 → 一次判完）

> 用户：「修改吧，而且性能能再恢复吗？」——上一轮只做了"抬超时 + 换 release"两件
> 临时手段，并把根因写进了站点「未来的计划」。本轮把根因修掉。

1. **先把账量清楚**（临时探针，只用不改语义；数据留在 `docs/design/by-tactics.md` §13）。
   最坏样本 = 卷 I 单元⑫ 的解答（526 行、9 道题、全 tactic）：release 构建判一次
   **91.2 s**，而同一份内容改回 term 风格的基线只要 **3.6 s**。探针说得很干净：
   `hits=20764 misses=89 judge_time=83.5s avg=938.6ms`，其中**解析只占 0.2 s**，
   **99.6% 花在 `check_document_with` 重跑整份前缀**（每次平均 69 条命令 = 闭包前缀 +
   本文件已判过的声明）。也就是 89 次未命中的判定吃掉 92% 的运行时间。
2. **先说清哪条路是死的**：最自然的修法是复用前缀的**已判定环境**，但内核 API 不允许
   ——`EnvBuilder` 字段私有、`new(arena, config)` 是唯一入口、`finish(self)` 消费自身，
   `ExportFile` 只读，而 **`crates/kernel/` 是冻结快照（硬规则 1）**；名字下标
   （`NamePtr::decl_idx`）绑定在构建它的 builder 上，换个 builder 造的 `Declar` 查旧环境
   会查错槽位。⇒ "oleans 式复用"要等内核开口子，**不是前端能自己做的**（免得下一刀再试）。
3. **修的是更窄但足够的那条**：同一个 `by` 块里**不再逐步判**，而是"先记下来、跑完一次
   判完"。`judge.rs` 新增 `JudgePair` + `judge_pairs_with`（N 条判定合成**一份文档**，
   每条各自带目标类型与 binder 折叠，合成声明仍叫 `_soko_judge_{k}`）；`begin_batch` /
   `flush_batch` 用 thread-local（**不**往 `run_tactics` 那 8 个参数的函数再加参数），
   批次活跃时 `judge_terms` 只记录并返回乐观的 `Match`；`by.rs` 的 `run_by` = 乐观一趟
   + flush。
4. **等价性论证（这是它能成立的全部理由）**：判定结果只在两处影响控制流——`Match` 才继续、
   否则在**那一步**报错；所以"这一趟里每一条判定真的都是 `Match`"时，乐观趟与逐条趟的
   **控制流逐字相同**。只要有一条不是 `Match`，就丢掉乐观结果、**严格重跑**（逐条判、
   逐条报错），诊断/位置/文案与改动前一致；乐观趟自己报了别的错而判定并未全绿时不必重跑
   ——那个错是真的。**例外**：`assumption` 走 `judge_terms_strict`（当场判），它要按结论
   **挑**哪条假设命中，乐观值给不出这个信息。
5. **实测（同一台机器、同一个 release 二进制，开关 = `SOKO_NO_JUDGE_BATCH=1`）**：

   | 输入 | 关（= 改动前） | 开（本次） | 倍数 |
   |---|---|---|---|
   | 单元⑫ 解答（526 行全 tactic） | 91.2 s | **28.7 s** | **3.2×** |
   | 整卷课程门禁（36 目标） | 4m33s | **2m56s** | **1.55×** |

   两态都是 **36 目标 · 328 checked · 99 open · 0 判负**（逐项相同）。CI 侧的连锁收益：
   单目标从 93.5 s（release）降到 28.7 s，`GRADE_TIMEOUT=600` 的余量从 6.4× 变成 21×，
   那个"runner 慢一点就撞上限"的发版卡点不复存在。
6. **判据三层**（都在 `docs/design/by-tactics.md` §13）：①
   `judge.rs::tests::batched_judgements_match_strict_ones`（命中/类型不匹配/术语解析失败/
   binder 缺类型四条路径逐条相等，且"预判不合成命令"没有让后面的序号错位）；②
   `judge.rs::tests::one_by_block_pays_a_single_document_pass`（一个 `by` 块 3 次判定
   **只走 1 遍文档**，关掉开关 ≥3 遍，两态结论相同）；③
   `crates/cli/tests/judge_batch.rs` **端到端对拍**：正常解答、**判定失败**的解答、
   带开放练习的画布、解答钥匙，四条输入在两态下 `--json` 事件流**逐字节相同**。
7. **还没还清的（别当成已还清）**：term 风格基线 3.6 s 说明离"零判定成本"还有距离——
   剩下的是"每个带 tactic 的声明仍要重走一遍前缀"（单元⑫ = 9 条声明 + 库里带 tactic 的
   声明 ≈ 20 遍）。再往前一步必须走内核侧的环境复用（见第 2 条），**没有进缺口台账**：
   台账判的是"行为/语言缺口 + 可复现的判据"，性能债的复现会随机器档位漂，塞进去只会变成
   噪声；它的判据就是本轮 §5 的 A/B 与上表。
8. **顺带**：`site/index.html`「未来的计划」里那条性能项改写成"已修 3.2×、还剩什么"，
   不能继续写着"根因没修"。

## 本轮进度（2026-09-21，第一百一十九轮（站点线）：站点事实改锚发布 tag —— 并修掉一个把正确复现判成红的陷阱）

> 用户：「那边功能侧 agent 已经提交了，你这边看看哪里要适配改一下，然后也提交一下。」
> 语言线的 `08b6782`（全课程 Lean 4 化）与站点只有一处冲突，但那一处会变成**用户可见的
> 假话**：它把 `Cargo.toml` 推到 `0.62.0`，而站点的每个页脚与每条下载指令都从版本号拼出来。

1. **适配（真问题，不是形式）**。原 `gen-site-data.py` 是 `version <- Cargo.toml`，
   而 `Cargo.toml` 在**整个发行窗口里一直领先于最后一个 tag**。按原样重跑一次，实测写出：
   `version: 0.62.0`（没有 tag、没有 Release、没有产物 = 读者下载不到），并把课程计数
   从 **329 改成 328 checked**（用工作树的课程 + 仓库 debug 构建量的）。而站点自己明说
   这个数是「已发布的版本，不是工作树」（`compare.html`）。
   **修法**：生成器改为锚**发布 tag** —— `release_version()`（最新 `vX.Y.Z` tag）+
   `release_tree()`（解出该 tag 的课程 / `scripts/soko` / `Cargo.toml`）+
   `release_binary()`（VS Code 扩展里那份已发布 CLI），判卷一律走
   `SOKONANODA_BIN=<已发布>`；拿不到 tag 时**沿用上次写下的版本**，**绝不退回 `Cargo.toml`**。
   `pages.yml` 的 checkout 补 `fetch-depth: 0`（默认浅克隆不带 tag，生成器就解析不出发布版本）。
   **实测**：`version: 0.61.0（发布 tag v0.61.0）；Cargo.toml 已是 0.62.0，尚无 tag —— 站点仍写已发布版本`
   + `set_theory: 门禁实测（v0.61.0 的课程 × 0.61.0 的二进制）36 目标 · 329 checked · 99 open · 0 判负`
   —— 与站点原值**逐项相同**，且 `counts_source` 从 `previous-run` 变回 `gate`（真的在实测了）。
2. **两条计数判据的基准从 `HEAD` 收紧到发布 tag**（K12 课程 / K16 playground），并修掉
   收紧后立刻暴露的一个陷阱：`courses/set-theory/tools/check.py` 要**沿目录向上找到
   `scripts/soko`** 才认「本检出」，再从那个目录的 `Cargo.toml` 读版本钉、与二进制
   `--version` 比对，不一致就 **exit 2 拒绝判卷**。只解课程到 `.cache/` 时它会一路上溯、
   撞到本仓库根，拿 HEAD 的钉（0.62.0）去卡已发布的 0.61.0 二进制 ⇒ 一次**正确的复现被判红**。
   **门禁是对的**（它那双眼睛分不出"已发布二进制"和"走错门的工作树"），错的是临时目录
   不是一个自洽的检出。三样一起解（课程 + `scripts/soko` + `Cargo.toml`）即自洽，判据才真的
   在说"能不能复现"。实测修完 **`v0.61.0 + 0.61.0 实测 36/329/99/0`**。
3. **顺手抓到并修掉页面上一句已经变成谎话的话（同一类病的第三处）**。首页「它现在还
   不能做什么」里写着 `` `counts_source: previous-run` `` —— 那是**当时**的取值；生成器在
   有已发布二进制的机器上会实测成 `gate`，于是这句话随环境变化而真假交替，**而没有任何
   检查会响**（K10 原先只抽三类"逐字复制"的字段，散文里引用的数据盖不住）。处置两件：
   - 页面改成**描述机制、不引用取值**（「实测发生在有已发布判卷二进制的那台机器上，
     没有的机器沿用上一轮的数；数据文件记着是哪一种」）；
   - K10 补第二类抽取：首页编辑器插画引用的 playground 计数（`decl_checked: 30`）按
     **playground 事件名翻译**后回查 `evidence.json`（`decl.checked`）——这两个数每次动
     playground 都会变，之前完全没人看着。反向测试做过（改成 31 → 判红）。
     现为 **93 条录制值回查**。
     **通用版故意不做**：路径、`infix: 50 " ∈ " => …` 这类记法语法、URL 都会撞上同一个
     正则，通用版只能靠手工豁免表活着，那种检查会烂掉；真正的解法是把这类值挪进数据
     文件由 JS 回填（版本号就是这么做的），属于页面改版的事。已记 `STATE.md` §7。
4. **文档同步（项数与契约都对不上实测）**：`AGENTS.md` / `ROADMAP.md` 的「14 项 / 13 项」
   → 真实 **18 项 / 16 项**（CI 跑 `--quick`）；`check-site.py` 的「9 条断言」→ **10 条**
   （docstring 里 sitemap 那条**从来没列进去**）；`pages.yml` 的「16 项」与两处生成器清单；
   `site-rebuild/spec/D3-lab-data.md` §1.1 的 `version` 契约（**信封版本 ≠ 站点版本**，
   原先写的是"同一条正则、两处不会读出不同的版本"——现在正是两个不同的事实）；
   `site-rebuild/STATE.md` §0（现状已不是"缺 21 个页面"）/§5 #18（新增本轮实测）/
   §7（13→18 项、K12 的新基准、K17 的动机）/§7.1+§7.2（新基准与陷阱的完整记录）。
5. **补一条判据 K17（版本必须是发布 tag）——修好的东西得有人守着**。前两条只管计数，
   而版本号本身没人管（28 个页脚 + 每条下载指令都从它拼出来）。它写错时 K12/K16 会
   **安静跳过**（按 `site.json` 的版本去找已发布二进制与 tag，找不到就报"跳过：不算通过"，
   措辞与平时一模一样）⇒ 站点能带着一个下载不到的版本上线而报告全绿。K17 **直接调用
   生成器自己的 `release_version()`**（`importlib` 按路径载入 `gen-site-data.py`，
   判据与产它的人不会各自漂移）。**"谎话"与"落后"分开判**：站点版本**没有任何 tag**
   ⇒ 判红（读者下载会 404）；有 tag、只是比最新的旧 ⇒ 只记一笔——发布刚落地、站点
   数据还没重跑时 `pages.yml` 正在部署，判红会让整条部署挂掉，把一个"该重跑生成器"
   的待办伪装成故障。三种情形实测过：正确 → `v0.61.0`；临时造 `v9.9.9` → 记一笔
   （造完即删）；写成无 tag 的 `0.99.0` → 判红并指名。
   **推送前又抓到一处"这次必红"的陷阱**：`gen-site-search.py --check` 原先拿整份
   载荷**逐字节**比对，而载荷里带 `source_commit` —— 生成索引 → 提交 → HEAD 变了 ⇒
   重建的载荷必然不同。这是"给自己拍一张带自己哈希的照片"：文件里记的是 `702e444`，
   CI 在 `4afe42a` 上重算得到 `4afe42a`，**部署必挂**。修法：比较用的规范形式抹掉
   `source_commit`（写盘仍带），判据从此只在**页面文字真的变了**时才红；两个方向都
   实测过（不动页面 → 绿；改一个 `<title>` → 红）。教训：**把"产物自身所在的那次提交"
   写进产物，再拿它自检，判据就永远不可能通过**。
6. **合入 main（本轮末）**：`i16-imports-and-projects` 与 `origin/main`（多一个 e2e
   台账提交 `d05dd47`）合并，推送 main 并删除该分支。**注意这里有个必然的连锁**：
   `origin/main` 上的 `Cargo.toml` 已是 0.62.0，所以**任何**一次 push main 都会让
   `ci.yml` 的 `auto-tag` 打出 `v0.62.0` 并派发 release（它只在 lint/test/e2e/
   e2e-macos 全绿后才跑，红则不发布——失败模式是安全的）。发布落地后站点需要按
   `site-rebuild/STATE.md` §12.2 重跑生成器（K17 会以"落后"的形式提示，不判红）。
   **合并后实测**：`pages` 作业 **25s 全绿并部署**（新站 28 页上线，线上
   `data/version.json` = `{"version":"0.61.0"}` —— 正是「已发布版本」）；而 `ci`
   作业红在**课程门禁超时**上（见下）。分支 `i16-imports-and-projects` 已删除。
   **顺手解开卡住 0.62.0 的那颗钉子**：`08b6782` 的 ci（`35512977249`）红的不是代码，
   而是 `Course gate` 的**两层**超时 —— 课程长大了，阈值没跟着实测走：
   外层步骤 5 分钟（改成 20）、内层 `check.py` 的单目标判卷预算 180s（改成 600，
   实测 unit12 解答在 runner 上约 4m06s > 180s ⇒ `exit=124`，G1/G3/G4 连锁判负）。
   用 `--selftest` 当「同工作量的标尺」换算：本机整卷 **4m46s**、`--selftest` 8.5s，
   同一次 CI 的 `--selftest` **16.7s** ⇒ runner ≈ 本机 ×2.0 ⇒ 整卷约 **9.5 分钟**。
   改成 **20 分钟**（对 runner 实测的 9m12s 有 2.2× 余量），并按纪律记进
   `docs/CI-FAILURES.md`（两条 + 一条修正：**本机自己会在快/慢两档间差 2×**，
   「慢档本机」恰好与 runner 同速——所以跨机比值只在两边同时量了同源步骤时才可信）。**判据门禁红 ⇒ auto-tag 不发版**：所以修它不是「求绿」，
   而是 0.62.0 能不能上线的必要一步。
7. **本机无法跑 Rust 门禁（环境限制，非代码问题）**：`scripts/soko gate` 在本机
   失败在**链接**阶段——`cc` 报 `You have not agreed to the Xcode license agreements`
   （`cc t.c -o t.out` 同样失败，exit 69）。`sudo xcodebuild -license` 需要交互式
   sudo，不在 agent 能力内。因此 `cargo test` / clippy / gate **本机不可用**，
   推送前只能验：站点 18/18、版本契约（`Cargo.toml` 与 `package.json` 都是 0.62.0）、
   工作流 YAML、git 层合并无冲突。**Rust 侧的结论以 CI 为准**——本次推送新增的提交
   只碰 `site/` `scripts/` `docs/` 与 workflow，未碰 `crates/`。
8. **站点进度页读的轮次也跟着走**：`site/data/site.json` 的 `round` 108→119
   （= `STATUS.md` 最新一轮），`set_theory` 新增 `released_tag: v0.61.0` 作为出处。
   6 份 `site/data/*.json` 的 lab 数据**故意不重跑**：它们描述的是已发布快照
   （`source_commit` 084da05），现在重跑会把未发布状态混进站点（见第 7 条）。
9. **验证**：`python3 scripts/site-verify.py` → **18/18 全绿**（28 页、K10 93 条录制值
   全部回查、K12 `v0.61.0 + 0.61.0 实测 36/329/99/0`、K16 `decl.checked 30 /
   example.checked 2 / exercise.open 4 / warning 2`、K17 `v0.61.0`、
   K3/K4/K13 渲染审计、K15 18 项交互实跑）；`check-site.py` 10 条断言亦绿。
10. **仍欠 / 下一轮**：`gen-site-lab.py` 的 6 份数据文件信封**仍直接读 `Cargo.toml`** ——
   发布前重跑会把未发布的 `source_commit`（08b6782）与用工作树量出的数混进站点，
   K10（`kernel.html` 逐字引用 `source_commit`/`generated_at`）与 K16 会判红。
   已写进 `STATE.md` §5 #18 与 `spec/D3-lab-data.md` §1.1 的警告框；**0.62.0 发布后**
   按 `STATE.md` §12.2 的五步一次做完（记法三条 + 6 份 lab 数据 + K12/K16 + 全绿重跑）。


# 当前状态与进度日志（agents 先读这里）

> 快照：2026-09-19（第一百〇六轮：**0.59.0 收尾**——语言线五刀（签名受检 /
> 构造子命名空间 / 派生 recursor 判据 / L1 prelude / 用户自定义记法）收成一个版本，
> 课程门禁接进 `scripts/soko gate` 与 CI，卷 I 上站点；版本 **0.59.0**，
> 发布由 push main → auto-tag 全自动；缺口台账门禁（`gap.py selftest` + `check`）
> 同轮接进 gate 与 CI，24 条缺口 **18 条 `fixed_in = 0.59.0`、`check` 全绿**；
> WO-010（诊断坐标）与 P4（课程跟随 prelude）同日落地）
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
3. **文档同步（项数与契约都对不上实测）**：`AGENTS.md` / `ROADMAP.md` 的「14 项 / 13 项」
   → 真实 **17 项 / 15 项**（CI 跑 `--quick`）；`check-site.py` 的「9 条断言」→ **10 条**
   （docstring 里 sitemap 那条**从来没列进去**）；`pages.yml` 的「16 项」与两处生成器清单；
   `site-rebuild/spec/D3-lab-data.md` §1.1 的 `version` 契约（**信封版本 ≠ 站点版本**，
   原先写的是"同一条正则、两处不会读出不同的版本"——现在正是两个不同的事实）；
   `site-rebuild/STATE.md` §0（现状已不是"缺 21 个页面"）/§5 #18（新增本轮实测）/
   §7（13→18 项、K12 的新基准、K17 的动机）/§7.1+§7.2（新基准与陷阱的完整记录）。
4. **补一条判据 K17（版本必须是发布 tag）——修好的东西得有人守着**。前两条只管计数，
   而版本号本身没人管（28 个页脚 + 每条下载指令都从它拼出来）。它写错时 K12/K16 会
   **安静跳过**（按 `site.json` 的版本去找已发布二进制与 tag，找不到就报"跳过：不算通过"，
   措辞与平时一模一样）⇒ 站点能带着一个下载不到的版本上线而报告全绿。K17 **直接调用
   生成器自己的 `release_version()`**（`importlib` 按路径载入 `gen-site-data.py`，
   判据与产它的人不会各自漂移）。**反向测试做过**：把 `version` 改成 `Cargo.toml` 的
   那个（`0.62.0`）→ 立即判红并指名原因；改回 → 绿。
5. **站点进度页读的轮次也跟着走**：`site/data/site.json` 的 `round` 108→119
   （= `STATUS.md` 最新一轮），`set_theory` 新增 `released_tag: v0.61.0` 作为出处。
   6 份 `site/data/*.json` 的 lab 数据**故意不重跑**：它们描述的是已发布快照
   （`source_commit` 084da05），现在重跑会把未发布状态混进站点（见第 7 条）。
6. **验证**：`python3 scripts/site-verify.py` → **18/18 全绿**（28 页、K10 91 条录制值
   全部回查、K12 `v0.61.0 + 0.61.0 实测 36/329/99/0`、K16 `decl.checked 30 /
   example.checked 2 / exercise.open 4 / warning 2`、K17 `v0.61.0`、
   K3/K4/K13 渲染审计、K15 18 项交互实跑）；`check-site.py` 10 条断言亦绿。
7. **仍欠 / 下一轮**：`gen-site-lab.py` 的 6 份数据文件信封**仍直接读 `Cargo.toml`** ——
   发布前重跑会把未发布的 `source_commit`（08b6782）与用工作树量出的数混进站点，
   K10（`kernel.html` 逐字引用 `source_commit`/`generated_at`）与 K16 会判红。
   已写进 `STATE.md` §5 #18 与 `spec/D3-lab-data.md` §1.1 的警告框；**0.62.0 发布后**
   按 `STATE.md` §12.2 的五步一次做完（记法三条 + 6 份 lab 数据 + K12/K16 + 全绿重跑）。

## 本轮进度（2026-09-21，第一百一十八轮：用户四条指令落地——报错质量 / 写进教学 / 速查表 / 站点交接文档）

> 用户在同一轮给了四条指令：①「你改好吧，问题我没看懂」（= 上一轮报的 `Set.mem a A`
> 报错质量问题）；②「写进教学」（= 把删骨架后新解锁的 tactic 教进课程）；③「修改」
> （= 记法对照页按 C1.5 重定位）；④「不要动 site，但你实现的功能特性都写到开发文档里，
> 不要让 site agent 搞不懂」。

1. **① 报错质量：修了一半 + 如实登记另一半（G-21）**。
   - **已修（声明位）**：`error.rs` 新增 `classify_term_in_type_position`——识别内核
     「`expected: Sort(n) | actual: $k`」（**项落在类型位**）这个形状，把它从泛化的
     `kernel-rejected` 归到 `kernel-expected-sort`，并把那条 hint 改写成**指根因**：
     「点名调用漏了前导类型参数（`Set.mem a A` 应为 `Set.mem α a A`），或直接用记法
     `a ∈ A` 让它自动补 `α`」。测试：既有的内核消息分类表加两条（新形状 + 一条对照，
     确认 L-06 的 Prop-not-cumulative 没被吞）。
   - **同轮重钉了「护城河」测试**（设计早预告过这一步）：`notation.rs::the_pointful_spelling_keeps_working_and_the_moat_holds`
     以前钉 `code == "kernel-rejected"`，现在钉 `kernel-expected-sort` **并新增一条断言**——
     hint 必须同时说出「前导类型参数」与记法出路。**护城河本身没变**（省略 `α` 仍判红、
     仍在 kernel 阶段、仍是同一条声明），变精确的只是诊断码与提示。
   - **仍欠（`by` 路径）**：`… : Set.mem a A -> A a := by intro h; exact h` 还是报
     「期望 `A a`，实际是 `Set.mem a A`」这种**同形**对照。根因查明：**`by` 块跑的
     时候声明签名还没被内核检查过**（`open_signature` 只用 axiom 探针、且只在值位是洞
     时才走）。修法是「签名检查前移到 `by` 之前」，但那要确认探针不进声明表 + 不为每条
     `by` 声明付额外 elaborate，**不在本轮预算内**，故如实登记。
   - 台账 **G-21**（`kind: language`、`severity: painful`、`status: open`）+ 自断言
     repro `docs/gaps/repro/G21-omitted-type-argument.{sokonanoda,sh}`：
     脚本同时断言两半，②一旦修好就转 exit 1、`gap.py check` 会提醒关账。
     `python3 scripts/gap.py check` → **全部与台账一致** ✓。
2. **② 写进教学：单元④ 从六条 tactic 扩到八条**（subagent 执行 + 我复核）。
   - 新增 `demo_by_constructor`（`∧` 目标上 `constructor` 拆两子目标）与
     `demo_by_cases`（`∨` 假设上 `cases h with | inl … | inr …`），开头说明改成
     **八条**并写明 `left`/`right`/`use` 与 `constructor` 同族；删掉已不成立的
     「其余 tactic 随后面的单元解锁」。练习**没加**（现成的 `by_ex4`/`by_ex6` 已能吃下
     这两个 tactic；历史上专门删过重复练习），编号无跳号、解答逐名覆盖。
   - 复核：4 个文件 rc=0、诊断 0、**CN/EN 剥注释后逐字节相同**；计数
     画布 `(9,6,0)`、解答 `(15,0,0)`；四处钉子重钉（`course.rs` GOLDEN、
     `course_status.rs` 逐单元 + summary **56/66**、`cli.rs` checked 56）→
     `course`/`course_status`/`course_shared`/`cli` **114 条全绿**。
   - **subagent 挖到一条真边界（值得记）**：`use` 要求目标是**真归纳**，而单元⑧ 的
     `∃` 是那里自己声明的 **axiom** ⇒ `use 0` 在单元⑧ 判红（报错原文进了交付）。
     它没有照我的字面要求写"use 在这里可用"，而是写成实情——**这是对的做法**。
     （把单元⑧ 的 `Exists` 改成 `inductive` 就能解锁 `use`，且更贴近 Lean；
     但那是与 C2.5 同族的新一刀，**留给下一轮拍板**。）
3. **③ 记法对照页重定位成速查表**（subagent 执行）：页头补一张**全符号速查表**
   （逻辑连接符内建 + 集合论符号的记法→点名→声明形状→优先级梯子）+ 三条使用规则
   （记法是源级糖 / 点名形式永久可用 / 记法自动补前导类型参数），9 对演示与 3 道练习
   **保留**（它们是"两种写法同判"的证据），「本页的定位」改成"参考页不是单元"。
   **计数不变**：画布 18 checked / 3 open、解答 21 checked / 0 open（只动注释与版式）。
4. **④ 站点交接文档（不碰 `site/`，也不进 site-rebuild 的地盘）**：新建
   `docs/design/lean-style-0.62.md`——给站点/文档 agent 的**事实清单**：12 项用户可见
   特性（记法 / tactic / 隐式实参 / 记法输入 / 判定侧修复 / 新诊断码
   `elab-implicit-argument-unsolved`）、课程内容的事实变化（两门课 + playground 的
   当前计数、C2.5 的后果）、**站点不该误解的三件事**（G-21 半修、记法在实参位的
   `elab-notation-argument-unsolved` 边界、记法对照页的双写法是**故意**的），
   并显式标注「工作树 = 未发布 0.62.0」+ 指向 `site-rebuild/STATE.md` #13 的测量陷阱。
   已挂进 `docs/README.md` 文档地图与 `docs/HANDOVER.md` 的关键文档索引。
5. **仍欠 / 下一轮拍板项**：单元⑧ 的 `Exists` 要不要改 `inductive`（解锁 `use`，
   与 C2.5 同族）；G-21 的 `by` 路径那一半；R2.5 的 IA-2/IA-3；`judge_infer` 的宇宙参数。

## 本轮进度（2026-09-21，第一百一十七轮：收尾三查——占位、hint 词汇、护城河话术）

> C2.5 落地后，按计划的「收尾同步」逐项**复查**（每项都先量规模再决定做不做）。

1. **练习占位再查（D3 / C1.2 的最后缺口）**：全课程扫「值位不是 `by` 形式的 `sorry`」，
   发现**卷 I 单元⑧ 还有 9 处**是 `:=` 换行 `sorry`（R2 那一刀漏掉的），
   已改成 `:= by` + 续行 `sorry`（**不动行数/span**）；改完该文件仍 rc=0、
   `15 checked / 9 open` 逐项不变。**现在两门课的占位一律是 `by` 形式**。
   剩下的 13 处「项模式 `example`/`theorem`」经复核**都是画布的项模式演示**
   （设计 C2.2 有意保留：`example : True ∨ False := Or.inl …`、`example : Sort 1 := Nat`…），
   不是漏改。
2. **C1.3（hint 词汇）实测后判定"已满足、无需改"**：全卷 I 扫「教项模式写法的 hint」
   （`fun (` / `:= fun` 等）只有 **3 条**，逐条看下来**都不是证明写法**——它们是在
   指名一个 **lambda 项**（对角线集的定义 `D := C' \ (fun y => f y y)`、`Eq.subst` 的
   motive、目标的字面形状 `x ∈ (fun y => ¬ f y y)`），属于设计要求的「关键件」。
   同期统计：**92 条 hint 本来就用 tactic 词汇**。⇒ 设计表里「294 条 hint 要换词汇」
   这一项的**真实规模远小于当时的估计**，本轮判定关闭（证据在上）。
3. **C1.5 顺带发现一处话术过期 + 一条报错质量退化**：记法对照页的「护城河」一节写
   `Set.mem a A ← 仍被内核拒绝（kernel-rejected）：缺 α`。**实测（0.62.0）**：
   - `Set.mem α a A -> A a` ✅、`a ∈ A -> A a` ✅、**`Set.mem a A -> A a` ❌** ——
     结论仍成立（省略 `α` 不可用），但**症状变了**：不再报「缺 `α`」，而是在**更晚**处
     炸成一条**同形**的不匹配：`` `exact` 类型不匹配：期望 `A a`，实际是 `Set.mem a A` ``。
   - 这是 **IA-1（隐式实参路线 C）之后的报错质量退化**：根因信息从"指出缺哪个参数"
     变成"两个几乎同名的类型不符"。**结论不变、教学话术不用改，但错误信息应该修**
     —— 记为「仍欠」，**尚未进 `gap.py` 台账**（登记要配 repro 脚本并过
     `gap.py check`，留到下一轮与修复一起做）。
   - 页面已改成**实测口径**（写明 0.62.0 的报错形态 + 待登记）。
4. **验证**：改后 `notation-cheatsheet` 仍 rc=0（18 checked / 3 open 不变）；
   卷 I 门禁 328/99/0；`scripts/soko gate` 见日志（本轮末尾启动）。
5. **仍欠**：`Set.mem a A` 的报错质量（上面第 3 条，需登记台账 + 修）；
   C1.5 的「重定位成速查表」是**内容改动**（本页现在同时承担"对照"与"参考"两个角色，
   要不要拆成两页、或把 9 对演示压成一张速查表，是教学决定）；
   「删骨架后要不要把 `constructor`/`cases` 写进 ①④⑧ 的教学」（上一轮留的教学决定）；
   R2.5 的 IA-2/IA-3；`judge_infer` 的宇宙参数。

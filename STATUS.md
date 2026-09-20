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

## 本轮进度（2026-09-21，第一百二十轮：官网 28 页 → **单页**；0.62.0 的第二个发版卡点）

> 用户两条指令：「site 刚刚被重构了，但是属于灾难，你把所有 site 简化吧：单个网页，
> 只说明：是什么，怎么安装，有什么核心特点，未来的计划。这几件事情。」
> 「然后把正规的 0.62.0 给我发布上去，耽误我正经测试了。」

1. **站点简化（已完成）**。28 页 + 8 份生成数据 + 11 个生成器/检查器（约 6800 行）
   → **1 个页面**（是什么 / 怎么安装 / 核心特点 / 未来的计划）+ 1 份样式表 + 1 个脚本
   + 1 个生成器 + 1 个验收脚本。删除：27 个页面、`site/en/`、`_partials/`、6 份 lab 数据、
   `gen-site-nav.py` / `gen-site-search.py` / `gen-site-lab.py` / `gen-site-fonts.py` /
   `gen-diagnostics-page.py` / `site-verify.py` / `site-audit.py` / `site-functest.py`
   / `site-shot.py`。**设计语言一个字没改**（方格纸 / 推理横线 / 绿=内核通过过、
   朱=诊断与限制 / 17px 中文锚点 / 40rem 行长 / 自托管字体子集）——砍掉的是规模，
   不是主张；正当化与边界写进 **`docs/design/site-single-page.md`（新权威）**，
   `docs/design/site.md` 与 `docs/design/site-rebuild/` 降级为历史存档（`STATE.md` 顶部
   加了横幅，§5/#13 的"已发布版本事实"陷阱仍然有效）。
2. **防漂移只留三条**（都删不掉）：版本号唯一来源（`gen-site-data.py` 从**最新 tag**
   生成 `site/data/site.json`，页面回填，零手写）、`sitemap` 与页面集合双向相等、
   `scripts/check-site.py` 一条命令 10 项验收（结构 / 链接 / 版本 / 元数据 / 体积 /
   已发布版本一致；`--browser` 追加真 Chrome：资源零 404 + 版本号已回填）。
   实测 **10/10 绿**（版本 0.61.0——0.62.0 尚未发出，这一项会随发版自动跟上）。
   顺带一条实测：macOS 上 `--headless=new --dump-dom` **会挂死**（120s 不返回），
   旧版 `--headless` + `--timeout` 才稳定。
3. **`pages.yml` 重写**并加 `release: types: [published]`：tag 一落地站点自动重新部署
   （站点写的是已发布版本的事实，而 tag 是 push 之外的动作——不挂这条，线上会一直
   停在上一个版本号）。`docs/RELEASE.md` §6.5 同步成两条命令。
4. **0.62.0 的第二个卡点（已解）**。前一提交把**外层**步骤超时 5 → 20 分钟修好之后，
   门禁**跑完了但自己判红**：`GRADE_TIMEOUT = 180s` 被 debug 构建的
   `units/solutions/unit12-solution.sokonanoda`（526 行、9 道题、全 tactic）顶穿
   （本机实测 debug **190s**）。本轮实测并落地：**release 构建 93.5s（2.0×）**，
   门禁改用 release 判卷（`ci.yml` 新增一步 `cargo build --release -p sokonanoda-cli`，
   `SOKONANODA_BIN` 指向 `target/release/`）——**这也更忠实**（发布形态就是 release）。
   口径修正：`docs/CI-FAILURES.md` 里"release 快一个量级"是估的，实测是 **2×**。
5. **根因未修，不许当成已修**：`by` 块每走一步 tactic，前端都会**重新判定整份文档**
   ⇒ 代价随文件规模超线性（profile 佐证：`run_tactics → judge_terms → check_document_with
   → run_pass` 占了整轮的一半）。真正的修法是给**未改动的前缀**做缓存；本轮只做了
   "抬上限 + 换 release"这两件临时手段，已写进站点「未来的计划」与本文 §4。
   未进缺口台账：进台账要配一条**CI 预算内**的确定性复现，不为了凑格式塞一条跑不动的。

6. **0.62.0 已发布（2026-09-21）**：`ci` 全绿（22m49s）→ auto-tag 打 `v0.62.0`
   → release 流水线 **5m35s 全绿**（8 平台 build + package-vsix + marketplace-publish
   + github-release），Release **26 资产**（8 CLI tarball + 8 LSP tarball + 9 VSIX
   + `SHA256SUMS`）。**发布产物实测**：下载 `sokonanoda-cli-aarch64-apple-darwin.tar.gz`
   → `sokonanoda 0.62.0`，判 `playground.sokonanoda` 的事件类型计数与仓库 release
   构建**逐项相同**（`decl.checked 23 / example.checked 2 / exercise.open 4 / warning 1`），
   真课程文件（卷 I 单元①）判卷正常。
7. **站点已跟上 0.62.0**：`site/data/site.json` → `v0.62.0`，`check-site.py` 9/9 +
   `--browser` 10/10。
   **一条实测修正**：`pages.yml` 的 `release: [published]` 对**自动发版不生效**——
   release 由 `release.yml` 用 `GITHUB_TOKEN` 创建，而 GITHUB_TOKEN 触发的事件不会再
   触发其它 workflow（与 auto-tag 那条注释同一个防递归坑；v0.62.0 实测一次都没触发）。
   所以"发布后刷新站点"的真正机制仍是 `docs/RELEASE.md` §6.5 那两步；那条 trigger
   留着只对"人在 UI 上发布 release"有效，注释已就地更正。

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


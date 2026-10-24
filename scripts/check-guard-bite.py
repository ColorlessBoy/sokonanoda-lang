#!/usr/bin/env python3
"""**咬住证明**（`docs/guard-bite.jsonl`）：守卫必须**咬得住一条已存在的坏输入**。

## 为什么要有这个文件（2026-10-15 用户【④】："停止守卫的守卫 ⇒ 出路是端到端判据"）

用户原话：

    现状循环：建守卫 → 假绿 → 建守卫看守卫 → …（**没有出口**）✗
    纠正：**任何新守卫必须自带"咬住证明"** —— 必须有一条**已存在的坏输入**
         （历史 bug / 用户的 case）被它咬住，且这条证明要**跑给人看**（一条命令 + 实测读数）✓
         仓库**已有**这条规则（「咬不住的守卫等于没有」）—— **只是没强制** ✗→✓
    判据：新守卫的交付物里没有"咬住证明" ⇒ **不算收口** ✗

⚠ 那条规则**原文在 `AGENTS.md:289`**（验证设计纪律 §3「反向验证是硬要求」）——
本文件**没有新造一条规则**，只是把**已有的**那条从"纪律"变成"判据" ✓
（用户说的就是这个：**"只是没强制"** ✗→✓）。

## 判据（三条，都能机械判）

* **C1 覆盖**：`--since <基点>..HEAD` 里**新增/修改的每一个守卫**都要在台账里有一行 ✓
  ⇒ 缺 ⇒ 判红（"新守卫的交付物里没有咬住证明 ⇒ 不算收口" ✗）。
  ⚠ 覆盖基线（`coverage_since`，写在台账抬头的注释里）**之前**就存在的守卫是
  **grandfathered**（不在强制面内）—— 这是**如实划的口径**，不是豁免 ✗：
  它们的清单由 `--list` 印出来（**评审可见** ✓），要收进来就补一行 ✓。
* **C2 咬得住**：每一行的 `pass` 命令必须 `rc == expect_rc`，且 **`bite` 命令必须
  `rc == expect_rc` 且 `expect_rc != 0`** ✗→✓ ——
  即"这条守卫在**那条已存在的坏输入**上**真的报红**" ✓。
  ⚠ `bite` 期望 0 ⇒ **判红**（"它咬不住" ✗）；`bite` 真的返回 0 ⇒ **判红** ✓。
* **C3 翻账（用户【①】）**：`--reopen-empty` —— 咬住证明**失效**的那一行，
  如果它点名了 `queue_id`，就**把那条已 done 的条目 reopen**（回 `todo`）✓，
  **不是**只记一条新账 ✗ ⇒ **"绿"不再免费** ✓。
* **C4 接线（2026-10-16，CI 实测红 ✗→✓）**：每条证明**声明它在哪个 CI job 里跑**
  （台账字段 `ci_job`，缺省 = `guard-bite`）⇒ 那个 job 必须在 `.github/workflows/ci.yml`
  里**真的存在**、且**真的有一跑它**（那个 job 的 **`run:` 正文**里出现 `--job <名字>`）✗→✓。
  ⚠ **"真的有一跑"= `run:` 正文**（复用 `run_bodies()` ✓）：**注释与 step 名都不算** ✗ ——
  与 `wired_scripts()`（接线臂）**同一条口径** ✓（`run_bodies()` 的 docstring 里就写着
  「step 名同理（`name:` 不是命令 ✓）」）。⚠ **第 6 轮 `ng7` 收窄**：此前这里拿的是
  **整个 job 块的文本**（`job_blocks()` 的原文）⇒ 只在 **step 名**里点名
  `check-guard-bite.py` 就能让 C4 通过（= 接线只靠提及 ✗，与 `ng3` §3.5 ① 同族）。
  收窄的读数（真 `ci.yml`）：台账声明的 **2** 个 job（`editor` / `guard-bite`）
  **全部**有 `run:` 正文命中 ⇒ 收窄**一条都不放走**（0 格 · 0 条红变绿 ✓）。
  ⚠ 为什么必须分派（**实测**）：`guard-bite` 是 `timeout-minutes: 10`、**不装
  `editor/vscode/node_modules`、也没有真 VS Code** —— 而第三层那几条证明要的正是那些
  （`scripts/third-layer-bite.sh` 环境缺失 ⇒ **exit 3**，而 `pass` 期望 0 ⇒ 判红 ✓）。
  ⇒ "一条命令扫全部"（`--all` 不分工）在 CI 上**必然红**，红的是"环境不在"、
  **不是**"守卫咬不住" ✗ —— 那是**假红**（读数不可归属 ✗）。
  ⚠ **这不是放宽** ✗：C4 保证**没有一条证明能靠"搬去一个不存在的 job"蒙混** ——
  声明了 job 而**没人跑它** ⇒ 判红 ✓（**空转闸** ✓）。要一次全跑（开发机）用 `--job all` ✓。
  ⚠ **分派的代价如实标注** ✗：`ci_job: <非默认>` 的那几条只在那个 job 的触发条件成立时
  跑（`editor` job 的条件是 `rust || editor` 改动）⇒ **纯 docs 推送不跑第三层那几条** ——
  它们**物理上**跑不进 `guard-bite` 的 10 分钟预算 ✓（不是口径松了 ✓）。
* **C13 登记接线（2026-10-18，用户 2026-10-17 23:1x 的硬令 ✗→✓）**：**第二本登记表**
  （`scripts/ux-queue.py` 的 `REGISTERED_SCRIPTS` = **规范判据 / 对照臂脚本**的地面真相 ✓）
  点名的**每一条**都必须**真的有一跑** ✗→✓ —— 否则判红（"**登记了但没接线 ⇒ 判红**" ✓）。
  ⚠ ⭐ **"真的跑"与 C4 逐字同一条口径**（`run_bodies()` ✓ —— 注释与 step 名都不算 ✗；
  `ci-local.sh` 走非注释行 ✓）。⚠ 两条出路写在红里：①**真跑**它 ✓；②把它从登记表里删掉 ✓。
  ⚠ **为什么必须有它（用户原话的机制 ✓）**：用户 2026-10-17 23:1x「**它事情做不对，
  并且还会把事情做坏，比如 vim 冲突**」⇒ 查透 = **做坏的机制 = "修了判据、但判据没接线"** ✗✗：
  10-12 修了 vim（`abbreviation-rewriter.js` 211 行 + `package.json` 的 `input.leader`）
  **并写了 1033 行判据**（`scripts/vscode-input-vim-e2e.mjs`，9 用例 × 2 臂 ✓ 很规范 ✓）
  —— **真接线 0/2** ✗（`grep -rn vscode-input-vim-e2e .github/workflows/ scripts/ci-local.sh
  scripts/githooks/` ⇒ **0 处** ✗；只命中登记清单与文档 ✗）⇒ **判据从来没跑过** ✗
  ⇒ **"0.91.0 自称改好了" = 它自己说的** ✗（**修了不守 ⇒ 会漂** ✗）。
  ⚠ **不是"加新守卫"** ✗：**本文件一个都没新增**（B 类 21/21 满 ⇒ 不许增殖 ✗）——
  只是把**已有的那本登记表**接上**已有的那个接线闸形状**（C4 ✓）。
  ⚠ **它为什么不能由 C4 覆盖**（先量后改 ✓）：C4 的面是**第一本**登记表
  （`docs/guard-bite.jsonl` 的 `ci_job` 字段 ✓），而这两条判据是 `.mjs` ⇒ **既不在**
  `guard_scripts()`（`GUARD_GLOBS` / `_SCRIPT_PATH_RE` 都是 `.py`-only ✓）**也不在**
  咬住证明台账里 ⇒ 没有任何一条已有判据看得见它们 ✗（实测读数见
  `.cache/ux/fix-v1/VERDICT.md` ✓）。
  ⚠ **表不腐烂**（同层② R2 的形状 ✓）：登记表点名了一个**不在树里**的脚本 ⇒ 也判红 ✓。
  ⚠ **如实划的边界** ✗：这一条管"**有没有一处真的跑它**"，**不管跑在哪一面**
  （CI ⇄ 本地 gate）—— 哪一面**按定义不可能**跑它，由那条红的出路②或
  `.cache/ux/fix-v1/VERDICT.md` 如实写清 ✓（**不是**在这里放豁免 ✗）。
* **C5 可达性（2026-10-16，CI 实测红 ✗→✓）**：证明里**钉的旧 rev**（"已存在的坏输入"
  的锚点 ✓）必须**在本仓里真的存在**、**且是 `HEAD` 的祖先** ⇒ 否则判红 ✗（**两档**）。
  ⚠ 为什么（**实测**，run `38828745723` · step 9）：`guard-bite` **有** `fetch-depth: 0`
  （`ci.yml:256`）⇒ 缺的**不是历史深度** ✗ —— 缺的是**那个对象**：CI 只检出**一个 ref**
  的历史，而开发机的 `.git` 里躺着 ~100 棵链接工作树/分支的对象 ⇒ 一个**只在分支上
  存在**的 rev（`10a6a621` = `agent/ed1` 的 tip、`land` rebase **之前**的孪生）在本机
  `git show` 得到、在 CI 里 `fatal: Not a valid object name` ⇒ 那两条证明的**空转闸**
  grep 不中 ⇒ `exit 0` ⇒ C2 印「这条守卫咬不住」—— 而真相是**前提不在** ✗。
  这就是"**本机绿、CI 红**"的形状 ✓（本机用 `git clone --single-branch --branch main`
  的**全深度**夹具**逐字**复现了 CI 那两行 ✓ —— 注意它 `--is-shallow-repository` = false
  ⇒ 与 ed1 那次**不是**同一个病 ✗）。
  ⚠ ⭐ **两档都必须判红**（**实测踩到过** ✗）：只判"能解析、但不是祖先"那一档 ⇒ 在
  **CI 形状的树**里（那个 rev **根本不在**）判据会**跳过**它 ⇒ 那条证明**空转** ⇒
  **CI 仍会红** ✗✗ ⇒ 现在「本仓里不存在」与「存在但不是 `HEAD` 的祖先」**都**判红 ✓
  ⇒ **同一份坏台账**在开发机与 CI 形状的树里**都**判红 ✓（咬住证明因此在两种树上都成立 ✓）。
  ⚠ ⭐ **只在"台账所属的那个仓"里判**（**实测踩到过** ✗）：**夹具仓**会把**真台账**拷进去
  （`check-attention-budget.py:_fixture_repo` 的既定形状 ——「照真仓的形状（拷真文件），
  不是打桩 ✗」✓）⇒ 那些**真 rev 当然不在**夹具仓里 ⇒ 不设这条，**合法夹具会假红** ✗
  （实测：那条 ④ 臂 rc=1）。⇒ 仓身份取台账**自己的** `# coverage_since:`（按定义它是
  "台账所属那个仓"的一个提交 ✓）：**它在本仓解析不了 ⇒ 不是台账的仓 ⇒ 本跑不判**
  （**印出来** ✓ —— 既不假红、也不假装判过 ✓）。⚠ **不是放宽** ✗：真仓里缺基线那一档
  **照样**判红，由 **C1** 兜住（`git diff` rc=128 ⇒ **exit 3** ✓），且 C5 用的是**台账的**
  `coverage_since`（**不看** `--since` ⇒ 没人能用 `--since` 绕开它 ✓）。
  ⚠ **同族第三次，两半合起来才是完整前提** ✓：ed1 的
  `scripts/check-history-depth-wiring.py`（R1）管"**那个 job 要有那份历史**"，
  本判据管"**那份历史里真有它**" ✗→✓。
  ⚠ **不是放宽** ✗：C5 只**加**判红，一条 `bite` 的期望 rc 都没动 ✓。
  ⚠ **锚 = `HEAD`**（= CI **检出的那个提交** ⇒ 与 CI 同口径 ✓；推 main 时 `HEAD` =
  推上去的那个 sha ✓）。
  ⚠ **如实划的边界** ✗：在**链接工作树**里 `HEAD` = 分支 tip ⇒ 这一条判的是"相对
  **本分支**可达" ⇒ 一条**只在本分支上**的 rev（`land` rebase 之后会被重写）在这里会
  放行、而 CI 会红 —— 那一档由 **CI 自己**兜住（C2 + 下面那条**人话提示** ✓，即把
  「咬不住」翻译成「钉的 rev 不在本仓里」✓）。
  ⚠ **口径：不解析 shell**（**刻意收窄**，照 `check-history-depth-wiring.py` 的
  「只认活行」「`fetch-depth` 只认那一步」两条先例 ✓）：命令里**任何** hex token 都算
  "钉了" ⇒ `grep` 的**模式**里写了 rev 字面量**也会**被算上 ✗
  （实测：本判据落地时它自己的第一条咬住证明就撞上了 ✓）。**为什么不去猜 shell 语义**：
  猜错 = 假红**或**假绿，而"换个不需要字面量的 grep 写法"是**有出路**的代价 ✓
  （判红时那条消息会直接把出路印出来 ✓）。⚠ 64 位的 sha256 **不**进这个面
  （`{7,40}` 在 40 位处没有词边界 ⇒ 整个 token 不匹配 ✓）。
  ⚠ **为什么没有"0 个 rev ⇒ 空转红"**：那会是一条**没有出路的红**（台账里有没有钉死的
  rev 是**数据**条件、不是判据条件 ⇒ 照 `check-attention-budget.py` 的
  `judge_round()`/`converged()` 先例：「再开轮也降不下去 ⇒ 判红会是一条没有出路的红」✗）
  ⇒ 改成**印出判到的 rev 数**（评审可见 ✓）+ 自检的**正反两向**臂（见 `selftest()`）✓。
* **C11 命令前提（2026-10-16，CI 实测红 ✗→✓）**：证明的**命令位**上那个路径
  （`scripts/foo.sh` / `/bin/foo` / `python3 scripts/foo.py`）必须**真的在树里**
  （直接 exec 的还要**可执行位** ✓）⇒ 否则判红 ✗，消息是「**前提不在**」而**不是**
  「这条守卫咬不住」✗。
  ⚠ 为什么（**实测**，洞 `BITE-RC-127-COMMAND-NOT-FOUND`）：`11bc8e3f` 那一跑的
  `editor` job 里，台账行 `crates/front/src/project/artifacts.rs` 的 `bite` 是
  `scripts/bite-artifact-temp-name-unique.sh e2cc1012`，而那个脚本**当时还没提交**
  （台账行 20:24 落、脚本 20:39 才落 ✗）⇒ 树里没有它 ⇒ `/bin/sh` 给 **127**
  （`command not found`）⇒ C2 印出「**这条守卫咬不住它声称咬住的那条坏输入**」——
  而真相是**命令根本没跑起来**（**前提不在** ✗）。
  ⇒ 本判据管**命令自己的文件**（与 **C5** 管*钉的 rev*、**C10** 管*钉的状态夹具*
  合起来才是"证明的前提"的**三档** ✓）；C2 那两处 `rc=126/127` 也改成印
  「命令**根本没跑起来**」✓（**归属闸**：读数可归属 ✓）。
  ⚠ **不是放宽** ✗：只**加**判红 —— 一条 `bite` 的期望 rc 都没动，也**不许**把
  `expect_rc` 改成 127 / 标成 skip / 删掉那一行（那是把「咬不住」说成「咬得住」✗）。
  ⚠ **口径：不解析 shell**（**刻意收窄**，同 C5/C10 ✓）：只认「**顶层**简单命令的
  **命令位**是一个路径」与「解释器的**第一个参数**是一个路径」两种形状（含前导赋值里的
  `$(…)` ✓）；其余**一律不判**并**印出来**（`ⓘ C11 不判 N 条` ✓）—— 参数里的路径
  **故意**可能不存在（台账两条 `SOKO_RATCHET_STATE=/nonexistent` ✓）⇒ 判它就是假红 ✗。

## 一处定义（`AGENTS.md` 硬规则 0(b)）

**守卫的枚举**（`guard_scripts()`）与**证明怎么跑**（`run_proof()`）**只有这一份** ✓；
`scripts/check-attention-budget.py --open-round` **调本文件**、不另抄 ✗。

### ⭐ 枚举 = **两条臂的并集**（洞 `GUARD-ENUMERATION-IS-NAME-BASED`，2026-10-16 ✗→✓）

修前只有**一条名字臂**（`GUARD_GLOBS` 的 glob）⇒ **改个名就绕过 C1/C6** ✗
（rec1 实测的活例子：U1 新增的 `scripts/release-metric.py` **不在** glob 里 ⇒ 两条判据
都看不见它，而它**本来就带着**咬住证明与锚 ✓）。现在两条臂**取并集**：

* **① 名字臂**（存量，**一个字没动** ✓）：`GUARD_GLOBS` 那五个 glob。
* **② 接线臂**（新 ✓，**位置口径**）：凡被 **`scripts/ci-local.sh`** 或
  **`.github/workflows/ci.yml`** 的 **step 真调用**过的 `scripts/*.py`
  —— `run:` 正文才算命令（**注释不算** ✗，`ci.yml` 的注释里到处点名脚本）、
  **只收仓里真存在的文件**（幽灵名字不算 ✓）。
  ⇒ **改个名绕不过去**：接线还在，它就在口径里 ✓（"能绕过去"与"该绕过去"是两件事 ✓）。

⚠ **不是放宽** ✗：并集 ⇒ **只加不减** —— 名字臂里那 6 个**没被接线**的守卫
（`check-site.py` / `target-hygiene.py` / `check-json-identity.py` / `check-one-run.py` /
`check-queue-rewrite-trigger.py` / `check-lean4-alignment.py`）**照样在口径里** ✓
（接线臂**不替代**名字臂：一个守卫暂时没接进 gate 不等于它不是守卫 ✗）。
⚠ **2026-10-17 更新**（`SUB-narrow-guards-to-one-layer` 第 2 轮）：名单里的
`check-queue-rewrite-trigger.py` 已与 `check-queue-views.py` **合并**成
`scripts/check-queue-integrity.py`（**B 21 → 20**，上限未动 ✗）⇒ 今天名字臂里
没被接线的是 **5** 个（上面那 6 个减掉它 ✓）；合并后的那条在**接线臂**里（ci-local.sh ∪ ci.yml ✓）。

* **C6 覆盖 / C7 有底 / C8 预算（2026-10-16，`SUB-break-the-guard-recursion` ①）**：
  登记表 `docs/guard-anchors.jsonl` 给**每一层**钉一个底 ——
  * **C6 覆盖**：`guard_scripts()` 口径下的**每一个**都要有**分类** ——
    `kind: guard`（判据）**或** `kind: tool`（接线臂收进来的工具 / 渲染器，见 C9 ✓）
    （新守卫没分类 ⇒ 判红 ✗；表里点名了不在口径里的 ⇒ 也判红，防表自己腐烂 ✗）。
  * **C9 登记（`kind: tool`，随接线臂落地 ✓）**：`tool` + `why`（**必填**：
    它为什么**不是**独立判据）+ `covered_by`（**可选**：独立覆盖它的 `kind: guard`；
    **悬空 ⇒ 判红** ✗）。
    ⚠ **为什么需要它**（如实划的口径 ✗）：接线臂收进来的 8 个里有 6 个是**工具**
    （`plan.py` / `ux-queue.py` / `compile-cache.py` / `ci-push-range.py` /
    `e2e-merge.py` / `e2e-summary.py`）
    —— 把它们记成 `kind: guard` 会撞 **C8 的绝对上限**（B **21/21**，用户拍板
    "判据不许增殖"+"不许抬上限"）⇒ 两条判据**冲突**时本文件选择**如实登记 + 印出缺口**
    （没有独立覆盖的工具进 `ⓘ`、`--counts` 有 `coverless_tools` 计数），
    **既不抬上限、也不放宽 C8** ✓。
    ⚠ **边界（如实写，不是密封 ✗）**：一个**新判据**若把自己登记成 `tool`，它就绕开
    C1/C6 —— 但那**至少要写一行登记 + 一个 `why`**，且没有独立覆盖会被**印出来**；
    修前它连一行都不用写（**改名即隐身** ✗）⇒ 仍是**收紧**，但不是密封 ✗。
  * **C7 有底**：每条守卫行必须指回一条 **A 类锚（`kind: anchor` = 用户现场）** ⇒
    `anchors` 里的每个 id 都必须在表里存在、且锚行必须写出 `site`/`action`/`reading` ✓。
    ⚠ **底是结构性的**：`anchors` **只许**填 anchor id ⇒ **B 不能指向 B**（递归到不了守卫层 ✓）。
  * **C8 预算**：**B 类总数 ≤ `caps.b_class_guards`**，且 `caps` **只许减**（每轮 −2，到 `floor`
    停住）—— 绝对上限，**不是比例**（比例可以两边一起涨 ✗）。⭐ **A 类不进任何上限**：
    "A 类增长 ⇒ 判红"是**结构性不可能**（A 不参与 C8）✓ —— 锚定型**无限预算** ✓。
  ⚠ **为什么塞进本文件而不是新开一个 `check-anchors.py`** ✗：那正是用户说的病
  （"判据的判据"再开一个文件 ⇒ 判据文件数 +1 ⇒ 递归）。`SUB-narrow-guards-to-one-layer` ⑥
  的原话就是：「`check-guard-bite.py` 是「守卫必须咬得住」的底 ⇒ **不合并**，但要按 ① 的
  要求：**每条 B 类都要能指回一条 A 类**」⇒ **本文件就是它指定的落点** ✓（判据文件数 0 增 ✓）。

* **C12 双面（本地自动面 ⇄ CI 面；第 4 轮 `ng4` 落地 · 第 5 轮 `ng6` 收紧）**：
  `guard_scripts()` 口径里的每一条，**两面都必须有它**（或按闭集 witness 声明**那一面上
  对象按定义不存在**）—— 缺任一面 ⇒ **判红** ✗。CI 侧缺 = 在**不跑 hook 的推送**
  （`--no-verify` / 机器人 / 没装 `core.hooksPath` 的检出）上它**整个不存在** ✗；
  本地侧缺 = 那条红**只能走到 CI 才被发现**（实测一轮 CI 33–35 分钟 ✗）。
  ⚠ **为什么需要它**：修前"两面"只长在 `.cache/ux/fix-ng*/census.py`（那是**证据**、
  不是闸 ✓）—— `wired_scripts()` 是 `ci-local.sh` **∪** `ci.yml` 的**并集** ⇒
  **只删一面** ⇒ 枚举**不变** ⇒ **C6 照样绿** ✗。
  **豁免**只给「**那一面上那个对象按定义不存在**」的闸：`docs/guard-anchors.jsonl` 那一行加
  `faces` 字段 + **闭集 witness**（一档一份实现就在本文件里 ⇒ **数据只能选、不能定义** ✓）；
  witness 的事实**每次现算** ⇒ **事实消失 / 豁免腐烂（那一面其实有）都判红** ✗。
  ⚠ **第 4 轮刻意不对称、第 5 轮才收紧**（如实留痕 ✓）：当时本地侧**只印 `ⓘ`**，因为
  唯一那条真洞（`scripts/check-site.py`）接上去**当场会红**（rc=1 · `site/data/site.json`
  与最新 tag 不一致 = **没有出路的红** ✗ E18）⇒ 先修红（第 5 轮的前置 = `ng5` 的发布闭环
  ⇒ `10/10 rc=0`）、再接线（`scripts/ci-local.sh` · **零前提**那档 ✓）、**再**把本地侧
  收紧成判红 ✓（逐条读数见 `.cache/ux/fix-ng{4,5,6}/VERDICT.md`）。

## 退出码

  0 = 覆盖齐 + 每条都咬得住 + 分类齐且都有底 / 1 = 判红 / 2 = 用法错 /
  3 = 环境错（台账或分类登记表缺失等，**不假装判过** ✗）

## 用法

  python3 scripts/check-guard-bite.py                 # C1（基线..HEAD 的守卫）+ C2（逐条真跑）+ C4/C13 接线 + C6/C7/C8 + C12
  python3 scripts/check-guard-bite.py --since <sha>   # 覆盖面改成 <sha>..HEAD
  python3 scripts/check-guard-bite.py --all           # C2 跑**本 job 的**每一条（不只范围内的）
  python3 scripts/check-guard-bite.py --all --job all # C2 跑**台账里每一条**（开发机；CI 分派跑 ✗）
  python3 scripts/check-guard-bite.py --list          # 只印：台账覆盖了谁 · 谁是 grandfathered · A/B 分类
  python3 scripts/check-guard-bite.py --counts        # 只印 A/B/锚/上限/两面的**机器可读**计数（JSON）
  python3 scripts/check-guard-bite.py --anchors <f>   # 换一份分类登记表（复现 / 咬住证明用 ✓）
  python3 scripts/check-guard-bite.py --reopen-empty  # ⭐ ① 翻账：咬不住的 ⇒ reopen 它关掉的那条
  python3 scripts/check-guard-bite.py --selftest      # 正反两向（含"bite 返回 0 ⇒ 必须判红"·"只删任一面 ⇒ 判红"）
"""
from __future__ import annotations

import argparse
import ast
import contextlib
import fnmatch
import io
import json
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import git_env  # noqa: E402  （同目录库：**干净 git 环境**的唯一出处 ✓）
import threshold_ratchet  # noqa: E402  （同目录库：ratchet 状态不变量的**唯一出处** ✓
#   —— C10 的"夹具前提"直接复用它，**不新造谓词** ✗（照 C6/C7/C8 复用锚登记表的先例 ✓））

ROOT = Path(__file__).resolve().parent.parent
LEDGER = ROOT / "docs" / "guard-bite.jsonl"
ANCHORS = ROOT / "docs" / "guard-anchors.jsonl"
NONBITING_FIXTURE = ROOT / "scripts" / "fixtures" / "guard-bite-nonbiting.jsonl"
CI_YML = ROOT / ".github" / "workflows" / "ci.yml"

# ── **证明在哪个 CI job 里跑**（C4；唯一一处 ✓）───────────────────────────────
# 缺省 = 跑 `--all` 的那个"每推必跑"的 job（`ci.yml` 的 `guard-bite`，无 `if` 过滤器 ✓）。
DEFAULT_JOB = "guard-bite"
# `--job all` = **不分工**（开发机一次全跑；CI 分派跑，理由见 docstring 的 C4 ✓）。
JOB_ALL = "all"
# job 名的合法形状（`ci.yml` 里 `  <name>:` 那一层）
_JOB_KEY_RE = re.compile(r"^  ([A-Za-z0-9_-]+):(\s|$)")

# ── **守卫的枚举**（唯一一处 ✓）──────────────────────────────────────────────
# 口径 = **两条臂的并集**（并集 ⇒ **只加不减** ✓；洞 `GUARD-ENUMERATION-IS-NAME-BASED`）：
#   ① **名字臂**（存量，2026-10-16 之前唯一的一条）= `scripts/` 下**判红用的**脚本
#      （`check-*` / `*-lint.py` / 两本台账 / 审计）；
#   ② **接线臂**（新 ✓，**位置口径**）= 被 `WIRING_FILES` 的 **step 真调用**过的
#      `scripts/*.py` ⇒ **改个名绕不过去** ✓（接线还在，它就在口径里）。
# ⚠ 加了一类新命名 ⇒ 只改这里一处（`check-attention-budget.py` 通过 CLI 复用 ✓）。
GUARD_GLOBS = ("scripts/check-*.py", "scripts/*-lint.py",
               "scripts/gap.py", "scripts/target-hygiene.py",
               "scripts/audit-*.py")
# 接线臂的两份接线（**唯一一处** ✓）：本地 gate（"第一次测试永远是这条脚本"）+ CI。
# ⚠ `ci.yml` 走 **`run:` 正文**（注释/step 名里点名脚本**不算**调用 ✗ —— 实测：注释里
#   点名 `perf-compare.py` / `threshold_ratchet.py`，扫全文会把没人调用的文件算进来 ✗）；
#   `ci-local.sh` 是 shell（非注释行**就是**命令）⇒ 走 `_non_comment_lines()` ✓。
WIRING_FILES = ("scripts/ci-local.sh", ".github/workflows/ci.yml")
_SCRIPT_PATH_RE = re.compile(r"scripts/[A-Za-z0-9_./\-]+\.py")
# ⚠ `_RUN_KEY_RE`（`run:` 那一行的形状）**不在本文件**：它随 `run_bodies()` 一起住在
#   `scripts/threshold_ratchet.py`（**唯一一处** ✓ —— 第 1 轮 `gs1` 把"读哪一层"的三件
#   （`live_lines()` / `run_bodies()` / `command_layer()`）收进那里，两条接缝判据与
#   本文件的接线臂**共用同一层定义** ✓）。
# 证明的超时（秒）：证明里可能起子进程 ⇒ 挂住会把门禁一起挂住（`gap.py` 的教训 ✓）
PROOF_TIMEOUT_S = 300

# ── **证明里钉的旧 rev**（C5；唯一一处 ✓）────────────────────────────────────
# 只认**独立的** hex token（`\b` 两端 ✓）⇒ 64 位的 sha256 片段**不会**被当 rev
# （`{7,40}` 在 40 位处没有词边界 ⇒ 整个 token 不匹配 ✓ —— 实测 `--list`/`--check`
# 的输出里那些 sha256 摘要**不进** C5 的判据面 ✓）。
REV_TOKEN_RE = re.compile(r"\b[0-9a-f]{7,40}\b")

# ── **证明里钉的状态夹具**（C10；唯一一处 ✓）──────────────────────────────────
# `SOKO_RATCHET_STATE=<path>` 是 `threshold_ratchet.state_path()` 的**复现 / 咬住证明**入口
# （它自己的 docstring 写着"**它不是旁路**"✓）⇒ 那个文件**存在**的话，它必须是**合法**状态
# —— 判据复用 `threshold_ratchet.state_problems()`（**一处定义**：与 `docs-lint.py` /
# `check-threshold-ratchet.py` 消费它时**同一条** ✓，**不新造谓词** ✗）。
STATE_FIXTURE_RE = re.compile(r"\bSOKO_RATCHET_STATE=(\S+)")


# ── **读哪一层**（`live_lines()` / `run_bodies()`）的**唯一一处**在
#    `scripts/threshold_ratchet.py`（第 1 轮 `gs1` 收窄 ⇒ 两条**接缝判据**与本文件的
#    **接线臂 / C4 / C12** 共用同一层定义 ✓ —— 本文件**不再自己抄一份** ✗）。
#    保留旧私有名 ⇒ 下面**一字未动** ✓（口径逐字节相同：`ci.yml` / `ci-local.sh` 里
#    以 `//` 开头的行 = **0**（实测）⇒ 多剥那一半对这两个输入无影响 ✓）。
_non_comment_lines = threshold_ratchet.live_lines
run_bodies = threshold_ratchet.run_bodies


def wired_scripts(root: Path = ROOT) -> list[str]:
    """**接线臂**：被 gate / CI 的 step **真调用**过的 `scripts/*.py`（**唯一一处** ✓）。

    ⚠ **只收仓里真存在的文件**（`(root / tok).is_file()`）⇒ 接线里点了一个**幽灵名字**
    （历史残留 / 别的工作树）**不会**被算成守卫 ✓（"别把真的不存在的文件也算进去" ✓）。
    """
    found: set[str] = set()
    for rel in WIRING_FILES:
        p = root / rel
        if not p.is_file():
            continue
        text = p.read_text(encoding="utf-8", errors="replace")
        body = run_bodies(text) if rel.endswith(".yml") else text
        for tok in _SCRIPT_PATH_RE.findall(_non_comment_lines(body)):
            if (root / tok).is_file():
                found.add(tok)
    return sorted(found)


def guard_scripts(root: Path = ROOT) -> list[str]:
    """**全部守卫**（仓库相对路径，排序）。**唯一一处** ✓ —— **两条臂的并集** ✓。"""
    found: set[str] = set()
    for pattern in GUARD_GLOBS:
        for path in root.glob(pattern):
            if path.is_file():
                found.add(path.relative_to(root).as_posix())
    found.update(wired_scripts(root))
    return sorted(found)


def is_guard(path: str, wired: set[str]) -> bool:
    """`path` 算不算守卫 —— **与 `guard_scripts()` 逐字同口径**（名字臂 ∪ 接线臂 ✓）。

    `wired` = 预先算好的接线臂（`changed_guards` 逐条问它 ⇒ 别每条都重扫接线 ✗）。
    """
    return any(fnmatch.fnmatch(path, g) for g in GUARD_GLOBS) or path in wired


def entry_job(e: dict) -> str:
    """这条证明声明在哪个 CI job 里跑（缺省 = `DEFAULT_JOB` ✓）。"""
    return str(e.get("ci_job") or DEFAULT_JOB)


def job_blocks(text: str) -> dict[str, str]:
    """`ci.yml` 里每个 job 的**文本块**（`  <name>:` 到下一个同缩进的键）。

    只做**文本**扫描（与 `scripts/check-ci-gate-coverage.py` 同一条路 ✓）——
    C4 要的是"这个 job 里到底有没有那一跑"，不是"YAML 能不能解析" ✗。
    """
    blocks: dict[str, list[str]] = {}
    cur: str | None = None
    for line in text.splitlines():
        m = _JOB_KEY_RE.match(line)
        if m:
            cur = m.group(1)
            blocks.setdefault(cur, [])
        if cur is not None:
            blocks[cur].append(line)
    return {k: "\n".join(v) for k, v in blocks.items()}


def _resolves(root: Path, rev: str) -> bool:
    """`rev` 在本仓里能不能解析成**提交**对象（C5 的唯一判据输入 ✓）。

    ⚠ `env=`（洞 `FIXTURE-GIT-ENV-LEAKS-INTO-REAL-REPO`）：`GIT_DIR` 优先于 `cwd` ⇒
    不清就会拿**调用者的仓**当"本仓" ⇒ C5 会替**别人的**提交背书 ✗（实测：夹具仓里
    本该"判不了"的那一档，因为 `GIT_DIR` 指向真仓而**误判成"台账所属的仓"** ⇒ 从
    ⓘ 分支翻到 `error:` 分支 ⇒ 夹具 `--selftest` 判红 ✗）。
    """
    return subprocess.run(["git", "cat-file", "-e", f"{rev}^{{commit}}"],
                          cwd=root, capture_output=True,
                          env=git_env.clean_git_env()).returncode == 0


def _is_ancestor(root: Path, rev: str, head: str) -> bool:
    return subprocess.run(["git", "merge-base", "--is-ancestor", rev, head],
                          cwd=root, capture_output=True,
                          env=git_env.clean_git_env()).returncode == 0


def rev_reachability_problems(root: Path, entries: list[dict],
                             coverage_since: str) -> tuple[list[str], list[str], list[str], str]:
    """**C5 可达性**：证明里钉的旧 rev 必须**在本仓里存在**且是 `HEAD` 的祖先 ⇒
    `(红行, 判到的 rev, 全部候选, 跳过原因)` ✓。**两档都判红** ✓（"不存在" / "不是祖先"）。

    ⚠ **为什么"不存在"也必须判红**（**实测踩到过** ✗）：只判"能解析但不是祖先"那一档
    ⇒ 在 **CI 形状的树**里（那个 rev **根本不在**）判据会**跳过**它 ⇒ 那条证明**空转**
    ⇒ 咬不住 ✗。⇒ 两档都判红之后，**同一份坏台账**在开发机与 CI 形状的树里**都**判红 ✓
    （咬住证明因此在两种树上都成立 ✓）。

    ⚠ ⭐ **只在"台账所属的那个仓"里判**（**实测踩到过** ✗）：**夹具仓**会把**真台账**
    拷进去（`check-attention-budget.py:_fixture_repo` 的既定形状 ——「照真仓的形状（拷真
    文件），不是打桩 ✗」✓）⇒ 那些**真 rev 当然不在**夹具仓里 ⇒ 不设这条，**合法夹具会假红**
    ✗（实测：`check-attention-budget.py --selftest` 的 ④ 臂 rc=1）。⇒ 判据的**仓身份**取
    台账**自己的** `# coverage_since:`（它按定义是"台账所属那个仓"的一个提交 ✓）：
    **它在本仓解析不了 ⇒ 这不是台账的仓 ⇒ 不判**（**印出来** ✓，既不假红、也不假装判过 ✓）。
    ⚠ **不是放宽** ✗：真仓里缺基线那一档**照样**判红 —— 由 **C1** 兜住（`git diff` rc=128
    ⇒ **exit 3**，浅克隆那条路 ✓），而 `--since` 显式给定时 C5 用的仍是台账的
    `coverage_since`（**不看** `--since` ⇒ 没人能用 `--since` 把这条绕开 ✓）。

    ⚠ **口径：不解析 shell**（**刻意收窄**，照 `check-history-depth-wiring.py` 的
    「只认活行」「`fetch-depth` 只认那一步」两条先例 ✓）：命令里**任何** hex token 都算
    "钉了" ⇒ `grep` 的**模式**里写了 rev 字面量**也会**被算上 ✗
    （实测：本判据落地时它自己的第一条咬住证明就撞上了 ✓）。**为什么不去猜 shell 语义**：
    猜错 = 假红**或**假绿，而"换个不需要字面量的 grep 写法"是**有出路**的代价 ✓
    （判红时那条消息会直接把出路印出来 ✓）。⚠ 64 位的 sha256 **不**进这个面
    （`{7,40}` 在 40 位处没有词边界 ⇒ 整个 token 不匹配 ✓）。

    ⚠ 判不到 `HEAD` ⇒ **判红**（"不假装判过" ✗ —— 与 C1 拿不到 `git diff` 同口径 ✓）。
    """
    head_out, head_err = _git(root, ["rev-parse", "HEAD"])
    head = head_out.strip()
    if head_err or not head:
        return ([f"**C5 可达性** ✗：判不到 `HEAD`（{head_err or '空输出'}）⇒ "
                 f"钉的 rev 有没有在那份历史里**判不了** ⇒ **不假装判过** ✗"], [], [], "")
    if coverage_since and not _resolves(root, coverage_since):
        return ([], [], [], (f"台账的覆盖基线 `{coverage_since}` **不在本仓** ⇒ 这**不是台账"
                             f"所属的那个仓**（夹具仓 / 拷来的台账 ⇒ 真 rev 当然不在 ✓）⇒ "
                             f"可达性**判不了** ⇒ 如实印出、**既不假红也不假装判过** ✓"
                             f"（真仓里缺基线那一档由 **C1** 兜住 ⇒ `exit 3` ✓）"))
    red: list[str] = []
    judged: list[str] = []
    candidates: list[str] = []
    for e in entries:
        for half in ("pass", "bite"):
            cmd = str(e[half].get("cmd", ""))
            for tok in sorted(set(REV_TOKEN_RE.findall(cmd))):
                candidates.append(tok)
                if not _resolves(root, tok):
                    # ⚠ **这一档必须也判红**（**实测踩到过** ✗）：只判"能解析但不是祖先"
                    #   ⇒ 在 **CI 形状的树**里（那个 rev **根本不在**）判据会**跳过**它
                    #   ⇒ 咬住证明**空转**（grep 不中 ⇒ `exit 0`）⇒ 咬不住 ✗。
                    red.append(
                        f"**C5 可达性** ✗：`{e['guard']}` 的 `{half}` 钉了 rev `{tok}`，"
                        f"它**在本仓里根本不存在**（`git cat-file -e {tok}^{{commit}}` rc≠0）"
                        f"⇒ 那条证明**量不到**它 ⇒ 空转闸让它 `exit 0` ⇒ 印成「这条守卫咬不住」，"
                        f"而真相是**前提不在** ✗（CI 只检出**一个 ref** 的历史 ⇒ 一条**只在别的"
                        f"分支上**存在的 rev 就是这样 ✗）⇒ 换成 **`HEAD` 的祖先**里那个同形状的 "
                        f"rev（`git merge-base --is-ancestor <rev> HEAD` 核 ✓；"
                        f"**别**把证明挪到别的 job ✗ —— 没有哪个 job 有别的分支的对象 ✓）。"
                        f"⚠ 本判据**不解析 shell**（口径见 docstring ✗）：命令里**任何** hex "
                        f"字面量都算「钉了」⇒ 若它其实只是个 `grep` 模式，换成**不需要字面量**的"
                        f"写法 ✓（grep **守卫名** / 中文结论 —— 本仓既有 5 条空转闸的形状 ✓）")
                    continue
                judged.append(tok)
                if _is_ancestor(root, tok, head):
                    continue
                red.append(
                    f"**C5 可达性** ✗：`{e['guard']}` 的 `{half}` 钉了 rev `{tok}`，"
                    f"它**不是 `HEAD`（{head[:12]}）的祖先** ⇒ CI 只检出**一个 ref** 的"
                    f"历史 ⇒ `git show {tok}:…` **rc=128** ⇒ 那条证明在 CI 里**量不到**"
                    f"（空转闸让它 `exit 0` ⇒ 印成「这条守卫咬不住」，而真相是**前提不在** ✗）"
                    f"⇒ 换成 **`HEAD` 的祖先**里那个同形状的 rev"
                    f"（`git merge-base --is-ancestor <rev> HEAD` 核 ✓；"
                    f"**别**把证明挪到别的 job ✗ —— 没有哪个 job 有别的分支的对象 ✓）。"
                    f"⚠ 本判据**不解析 shell**（口径见 docstring ✗）：`grep` 模式里的 rev "
                    f"字面量**也算**「钉了」⇒ 若它其实只是个 `grep` 模式，换成**不需要字面量**的"
                    f"写法 ✓（grep **守卫名** / 中文结论 —— 本仓既有 5 条空转闸的形状 ✓）")
    return red, sorted(set(judged)), sorted(set(candidates)), ""


def unresolvable_revs(root: Path, cmd: str) -> list[str]:
    """这条证明里**解析不了**的 rev token（C2 判红时附的人话 ✓）。**唯一一处** ✓。"""
    return [tok for tok in sorted(set(REV_TOKEN_RE.findall(cmd)))
            if not _resolves(root, tok)]


def state_fixture_problems(root: Path, entries: list[dict]) -> tuple[list[str], list[str]]:
    """**C10 夹具前提**：证明里钉的 `SOKO_RATCHET_STATE` 状态夹具，**存在**的话必须是
    **合法**状态 ⇒ 否则判红 ✗（**不是**「这条守卫咬不住」✗）。

    ⚠ 为什么要有这一条（**实测**，2026-10-16，洞 `GUARD-BITE-SELFTEST-ONE-GROUP-RED-ON-CI`）：
    `4a4264bd` 修了 `threshold_ratchet.state_problems()` 的 off-by-one（history 复查从
    "比**下一轮** caps、跳过末轮"改成"比**那一轮自己的** caps"）之后，夹具
    `scripts/fixtures/threshold-ratchet-tight.json`（`1a3b334c` 建的、之后一字未改）第 8 轮
    的读数（6.41 MB）越过了**它自己的**上限（6.00 MB）⇒ `caps_or_error()` 判「状态不健康」
    ⇒ `docs-lint.py` 退 **2**（台账期望 **1**）⇒ **C2 印「这条守卫咬不住」**，而真相是
    **夹具不再是合法状态** ✗ —— 与 C5 **逐字同一条**纪律（「印成咬不住，而真相是
    **前提不在**」✓）。⚠ 判据是**修好的那一条照亮了本来就坏的夹具**（**不是** `4a4264bd`
    引入的 ✗ —— 归因要准 ✓）：`open_round`（`threshold_ratchet.py:318-347`）的闸门是
    "读数必须落进**收紧后**的上限" ⇒ 合法 history **不可能**出现"读数 > 那一轮自己的 caps"
    ⇒ 那份夹具**本来就**不是它自称的"**合法**的第 8 轮状态" ✗。

    ⚠ **文件不存在 ⇒ 不判**（那是**故意**的坏输入 ✓）：台账里有两条
    `SOKO_RATCHET_STATE=/nonexistent`（期望 rc=3 = "不假装判过" ✓）—— 判它们红就是**假红** ✗
    （与 C5 的"两档都判红"**不冲突**：那里"不存在"**本身**就是坏输入，这里"不存在"
    **正是** bite 要考的那条坏输入 ✓）。

    ⚠ **口径：不解析 shell**（同 C5 ✓）：`SOKO_RATCHET_STATE=` 后面**第一个**空白分隔的
    token 就是它钉的路径 ⇒ 路径里带空格会被读成半个（**有出路**：换个不带空格的名字 ✓）。
    """
    red: list[str] = []
    judged: list[str] = []
    for e in entries:
        for half in ("pass", "bite"):
            cmd = str(e[half].get("cmd", ""))
            for tok in sorted(set(STATE_FIXTURE_RE.findall(cmd))):
                p = Path(tok)
                if not p.is_absolute():
                    p = root / p
                if not p.exists():
                    continue
                judged.append(tok)
                try:
                    st = threshold_ratchet.state_load(p)
                except threshold_ratchet.RatchetError as err:
                    red.append(
                        f"**C10 夹具前提** ✗：`{e['guard']}` 的 `{half}` 钉了状态夹具 `{tok}`，"
                        f"而它**读不了**（{err}）⇒ 那条证明**量不到**它 ⇒ 会印成「这条守卫"
                        f"咬不住」，而真相是**前提坏了** ✗ ⇒ 把夹具修成一个**合法**状态 ✓")
                    continue
                problems = threshold_ratchet.state_problems(st)
                if problems:
                    red.append(
                        f"**C10 夹具前提** ✗：`{e['guard']}` 的 `{half}` 钉了状态夹具 `{tok}`，"
                        f"而它**不是合法状态**（{problems[0]}）⇒ 那条证明**量不到**它 ⇒ 会印成"
                        f"「这条守卫咬不住」，而真相是**前提坏了** ✗ ⇒ 把夹具修成**合法**状态 ✓"
                        f"（⚠ `open_round` 只产出「读数落进**收紧后**的上限」的轮次 ⇒ 手写 "
                        f"history 很容易越过它**自己**的 caps ✗）")
    return red, sorted(set(judged))


# ── **证明命令的前提**（C11；唯一一处 ✓）────────────────────────────────────
# 一条 `pass`/`bite` 命令的**命令位**上如果写着**仓内路径**（`scripts/foo.sh`）或
# **绝对路径**（`/bin/foo`），那个文件就**必须真的在树里**（直接 exec 的还要**可执行位** ✓）
# —— 否则 shell 只会给 **127**（`command not found`）/ **126**（不可执行）⇒ 那条证明
# **根本没跑起来** ⇒ 读数**不可归属**（C2 会把它印成「这条守卫咬不住」，而真相是
# **前提不在** ✗）。
#
# ⚠ **为什么必须收窄**（照 C5/C10 的「不解析 shell」先例 ✓）：本判据**只**认
#   「**顶层**简单命令的**命令位**是一个路径」/「解释器的**第一个参数**是一个路径」
#   这两种形状 —— 变量赋值（`VAR=…`）、`$(…)`/反引号/引号里的 token、`-` 开头的参数、
#   以及**任何判不了**的形状，一律**不判**（`--check` 把判到的条数印出来 ✓）。
#   猜 shell 语义 = 假红**或**假绿 ⇒ 宁可少判 ✓（C5 的 docstring 已经把这条写死了 ✓）。
#
# ⚠ **为什么不是「宽一点更安全」** ✗：命令的**参数**里到处是路径，而有些路径
#   **故意不存在**（台账里两条 `SOKO_RATCHET_STATE=/nonexistent` 就是**坏输入本身** ✓
#   —— C10 的 docstring 专门写过这一档 ✗）⇒ 把"参数里的路径"也判红就是**假红** ✗。
_COMMAND_SEP = ";|&\n"
# 判不了的字符（出现在**命令位** token 里 ⇒ 那是展开/引号/子 shell ⇒ **不猜** ✓）
_UNJUDGEABLE = set("$`\"'(){}*?<>|&;=")
# 解释器：它的**第一个参数**若是一个路径 ⇒ 那是被读的脚本文件 ⇒ 必须存在 ✓
_INTERPRETERS = frozenset({"python3", "python", "node", "bash", "sh", "dash", "ruby", "perl"})
_ASSIGN_RE = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*=")
# shell 的「**根本没跑起来**」两个退出码（C11 的**归属**面 ✓）：
#   127 = `command not found` · 126 = 找到了但不可执行 ✓
NOT_EXECUTED_RC = (126, 127)


def _not_executed_reason(rc: int) -> str:
    """`rc` 的人话（C11 归属面**唯一一处** ✓）：把 127/126 翻成"命令根本没跑起来" ✓。"""
    if rc == 127:
        return "rc=127 = shell 的 `command not found`（那条命令**不在** PATH / 树里）"
    if rc == 126:
        return "rc=126 = shell 的 `Permission denied`（找到了但**没有可执行位**）"
    return f"rc={rc}"


def top_level_segments(cmd: str) -> list[str]:
    """按**顶层**分隔符（`;` `|` `&` 换行）切一段命令。**唯一一处** ✓

    ⚠ **刻意收窄**（同 C5/C10 的「不解析 shell」✓）：只做**引号与括号的配对**
    （`'…'` / `"…"` / `` `…` `` / `$(…)` / `(…)` 里的分隔符**不算** ✓），
    不做变量展开、不认别名/函数/heredoc ⇒ **判不了的一律不判**（`<<` 整条不判 ✓）。
    """
    if "<<" in cmd:                     # heredoc：判不了 ⇒ 整条不判（宁可少判 ✓）
        return []
    out: list[str] = []
    buf: list[str] = []
    sq = dq = bt = False
    depth = 0
    i = 0
    while i < len(cmd):
        ch = cmd[i]
        if sq:
            sq = ch != "'"
        elif dq:
            if ch == "\\" and i + 1 < len(cmd):
                buf.append(ch)
                i += 1
                ch = cmd[i]
            dq = ch != '"'
        elif bt:
            bt = ch != "`"
        elif ch == "'":
            sq = True
        elif ch == '"':
            dq = True
        elif ch == "`":
            bt = True
        elif ch == "$" and i + 1 < len(cmd) and cmd[i + 1] == "(":
            depth += 1
            buf.append("$(")
            i += 2
            continue
        elif ch == "(":
            depth += 1
        elif ch == ")":
            depth = max(0, depth - 1)
        elif depth == 0 and ch in _COMMAND_SEP:
            out.append("".join(buf))
            buf = []
            i += 1
            continue
        buf.append(ch)
        i += 1
    out.append("".join(buf))
    return [s for s in out if s.strip()]


def judgeable_command_paths(seg: str) -> list[tuple[str, bool]]:
    """一段的**可执行位路径** ⇒ `[(路径, 是否要可执行位)]`。判不了 ⇒ `[]`（**不猜** ✓）。

    ⚠ **前导赋值里的命令替换**（`out=$(python3 scripts/x.py …)`，本仓最常见的形状 ✓）：
    `$(` 一出现，命令位就在**它后面** ⇒ 取**赋值 token 里 `$(` 之后的那半**当命令位
    （`python3`），**不是**把 `scripts/x.py` 当命令位 ✗ —— 否则每条
    `out=$(python3 …)` 都会被误判成"直接 exec 一个 `.py`"（实测：14 条假红 ✗→✓）。

    ⚠ `(` 开头的子 shell（`(cd … && …)`）只剥掉**前导**括号再取命令位 —— 里面的
    分隔符已经被 `top_level_segments()` 的 `depth` 挡住 ⇒ 这一段的命令位是**第一个**
    命令 ✓（同一段里**后面**的命令这一条**不判** —— 少判，不猜 ✓）。
    """
    toks = seg.strip().lstrip("(").strip().split()
    i = 0
    word: str | None = None
    while i < len(toks) and _ASSIGN_RE.match(toks[i]):
        head = toks[i]
        if "$(" in head:                 # 赋值里起了**命令替换** ⇒ 命令位在 `$(` 之后 ✓
            word = head.rsplit("$(", 1)[1]
            i += 1
            break
        i += 1
    if word is None:
        if i >= len(toks):
            return []
        word = toks[i]
        i += 1
    if word.startswith("-") or (_UNJUDGEABLE & set(word)):
        return []
    if "/" in word:                      # 命令位就是路径 ⇒ **直接 exec** ⇒ 还要可执行位 ✓
        return [(word, True)]
    if word in _INTERPRETERS and i < len(toks):
        arg = toks[i]
        if "/" in arg and not arg.startswith("-") and not (_UNJUDGEABLE & set(arg)):
            return [(arg, False)]        # 解释器**读**脚本 ⇒ 只要存在 ✓
    return []


def command_premise_problems(root: Path, entries: list[dict],
                             own_repo: bool = True) -> tuple[list[str], list[str], list[str]]:
    """**C11 命令前提**：证明命令**可执行位**上那个路径必须真的在树里（可执行位对 ✓）
    ⇒ `(红行, 判到的路径, 判不了的条目)` ✓。

    ⚠ **为什么要有这一条**（**CI 实测** ✗→✓，洞 `BITE-RC-127-COMMAND-NOT-FOUND`，
    2026-10-16 · job `editor`）：台账里 `crates/front/src/project/artifacts.rs` 那一行的
    `bite` 是 `scripts/bite-artifact-temp-name-unique.sh e2cc1012`，而那个脚本**当时还没提交**
    （台账行先落、脚本 3 分钟后才落 ✗）⇒ `11bc8e3f` 的树里**没有它** ⇒ `/bin/sh` 给
    **127**（`command not found`）⇒ C2 印出「**这条守卫咬不住它声称咬住的那条坏输入**」——
    而真相是**命令根本没跑起来**（**前提不在** ✗）。⚠ 这条洞**从台账行落地起就一直在** ✗：
    没有 C11，任何人都可以登记一条**指向还不存在的文件**的证明，而 CI 只会说"咬不住" ✗。

    ⚠ **与 C5/C10 逐字同一族**（三档都是"证明的**前提**在不在"）：C5 管**钉的 rev**、
    C10 管**钉的状态夹具**、C11 管**命令自己的文件** ✓ —— 三档的判红消息都必须是
    「**前提不在**」而**不是**「这条守卫咬不住」✗（**读数可归属** ✓）。

    ⚠ ⭐ **`own_repo=False` ⇒ 本跑不判**（与 **C5/C6 逐字同一条口径** ✓，`check()` 传进来）：
    **夹具仓**会把**真台账**拷进去（`check-attention-budget.py:_fixture_repo` 的既定形状 ——
    「照真仓的形状（拷真文件），不是打桩 ✗」✓）⇒ 那些命令位路径**当然不在**夹具仓里 ⇒
    不设这条，C11 会把**合法夹具**判红 ✗（**实测**：加 C11 后
    `check-attention-budget.py --selftest` 当场 4 组红 ✗→✓）。⚠ **不是放宽** ✗：真仓里
    照样判红（CI 形状的 `11bc8e3f` 树上实测 **1 条红**、指名那一行 ✓），且夹具仓那条路
    由 **C1** 兜住（`git diff` rc=128 ⇒ `exit 3` ✓）。

    ⚠ **不过滤 job**（同 C5/C10）：命令文件不在是**台账**的缺陷，不是"哪批证明跑在哪"的事
    ⇒ 每个 job 都判它（否则一条声明在别的 job 的证明能把缺文件藏起来 ✗）。

    ⚠ **不判"判不了"的条目**：`judgeable_command_paths()` 只认两种形状 ⇒ 其余**如实印出**
    （`--check` 的 `ⓘ` 行 + 计数 ✓），**不**当空转判红 —— 那会是一条**没有出路的红**
    （命令怎么写是**数据**条件 ⇒ 照 C5 的「0 个 rev 不判红」先例 ✓）。
    """
    if not own_repo:
        return [], [], []
    referrers: dict[str, list[str]] = {}
    need_x: dict[str, bool] = {}
    judged: list[str] = []
    skipped: list[str] = []
    for e in entries:
        for half in ("pass", "bite"):
            cmd = str(e[half].get("cmd", ""))
            hits = [h for seg in top_level_segments(cmd) for h in judgeable_command_paths(seg)]
            if not hits:
                skipped.append(f"`{e['guard']}` `{half}`")
                continue
            for tok, x in hits:
                judged.append(tok)
                need_x[tok] = need_x.get(tok, False) or x
                referrers.setdefault(tok, []).append(f"`{e['guard']}` `{half}`")
    red: list[str] = []
    for tok in sorted(referrers):
        p = Path(tok)
        if not p.is_absolute():
            p = root / tok
        who = " · ".join(sorted(set(referrers[tok])))
        if not p.exists():
            red.append(
                f"**C11 命令前提** ✗：{who} 的**命令位**上写着 `{tok}`，而它**不在树里**"
                f"（`{p}`）⇒ shell 只会给 **127**（`command not found`）⇒ 那条证明"
                f"**根本没跑起来** ⇒ 读数**不可归属**（C2 会把它印成「这条守卫咬不住」，"
                f"而真相是**前提不在** ✗）⇒ 把那个文件**跟台账行一起提交** ✓"
                f"（⚠ **别**把 `expect_rc` 改成 127 ✗ —— 那是把「咬不住」说成「咬得住」✗；"
                f"**也别**把它标成 skip ✗）")
        elif need_x[tok] and p.is_file() and not os.access(p, os.X_OK):
            red.append(
                f"**C11 命令前提** ✗：{who} 的**命令位**上直接 exec `{tok}`，而它**没有"
                f"可执行位**（`os.access(…, X_OK)` = False）⇒ shell 只会给 **126**"
                f"（`Permission denied`）⇒ 那条证明**根本没跑起来** ⇒ 读数**不可归属** ✗"
                f"⇒ `chmod +x {tok}` 并**一起提交**（`git ls-files -s` 要看到 `100755` ✓）")
    return red, sorted(set(judged)), skipped


def wiring_problems(root: Path, entries: list[dict]) -> list[str]:
    """**C4 接线**：每条证明声明的 `ci_job` 都要在 `ci.yml` 里存在且**真的跑它** ✓。

    ⚠ 这是"分派"的**空转闸**（docstring C4）：没有它，把一条证明标成
    `ci_job: 某个不存在的 job` 就等于**把它藏起来** ✗ —— 那才是放宽 ✗。

    ⚠ ⭐ **"真的跑"= `run:` 正文**（`run_bodies()` ✓ —— **注释与 step 名都不算** ✗）：
    第 6 轮 `ng7` 收窄（此前是**整个 job 块的文本** ⇒ 只在 step 名里点名就能通过 ✗）。
    """
    jobs = sorted({entry_job(e) for e in entries})
    if not jobs:
        return []
    yml = root / ".github" / "workflows" / "ci.yml"
    if not yml.exists():
        return [f"**C4 接线** ✗：`{yml}` 不在 ⇒ 判不了 `ci_job` 的接线"
                f"（**不假装判过** ✗）"]
    blocks = job_blocks(yml.read_text(encoding="utf-8", errors="replace"))
    red: list[str] = []
    for j in jobs:
        blk = blocks.get(j)
        if blk is None:
            red.append(f"**C4 接线** ✗：台账里有条目声明 `ci_job: {j}`，"
                       f"而 `{yml.name}` 里**没有这个 job** ⇒ 那条证明**没有任何 CI 会跑它**"
                       f"（藏起来 = 空转 ✗）")
            continue
        # ⭐ **只认 `run:` 正文**（第 6 轮 `ng7`）：注释与 step 名**不算"跑"** ✗ ——
        #   否则"删 `run:` 正文、只留 step 名"就能让 C4 通过（接线只靠提及 ✗）。
        body = run_bodies(blk)
        if "check-guard-bite.py" not in body:
            red.append(f"**C4 接线** ✗：`ci_job: {j}` 的那个 job 里**没有任何一步**跑 "
                       f"`check-guard-bite.py` ⇒ 那批证明在 CI 里**空转** ✗"
                       f"（⚠ **`run:` 正文里没有** —— 注释 / step 名里点名**不算** ✗）")
            continue
        if j != DEFAULT_JOB and f"--job {j}" not in body:
            red.append(f"**C4 接线** ✗：`ci_job: {j}` 的那个 job 里**没有** `--job {j}` "
                       f"⇒ 它跑的是**默认 job 的那批**，这批仍然没人跑 ✗"
                       f"（⚠ 同样只认 **`run:` 正文** ✓）")
    return red


# ── **第二本登记表的接线闸**（C13；唯一一处 ✓）────────────────────────────────
# 第一本登记表是 C4 的 `docs/guard-bite.jsonl`（每条证明声明 `ci_job` ✓）；
# 第二本是 `scripts/ux-queue.py` 的 `REGISTERED_SCRIPTS`（**规范判据 / 对照臂脚本**的
# **地面真相**清单 ✓）。**两本的口径逐字相同**：**登记 = 声明"它是一条该跑的判据"**
# ⇒ 那就必须**真的有一跑** ✗→✓（"真的跑" = `run:` 正文 / `ci-local.sh` 非注释行 ✓）。
REGISTRY_FILE = "scripts/ux-queue.py"
REGISTRY_NAME = "REGISTERED_SCRIPTS"
# ⚠ 与 `_SCRIPT_PATH_RE`（`wired_scripts()` 用）**故意不同** —— **不是"另抄一份"** ✗：
#   `.py`-only 是**接线臂刻意的口径**（它的输出喂 `guard_scripts()` ⇒ C6 分类面 ⇒
#   多收一个 `.mjs` 就多一条要分类的守卫 ✗）；而 C13 这本登记表里放的就是 `.mjs` ✓
#   ⇒ 这里必须认**任意脚本扩展名**。两层共用的仍是**同一处**「读哪一层」
#   （`run_bodies()` / `live_lines()` ✓）。
_ANY_SCRIPT_PATH_RE = re.compile(r"scripts/[A-Za-z0-9_./\-]+\.[A-Za-z0-9]+")


def registered_scripts(root: Path = ROOT) -> tuple[list[str], str | None]:
    """读登记表 `scripts/ux-queue.py` 的 `REGISTERED_SCRIPTS`（**唯一一处** ✓）。

    ⚠ **用 `ast` 读那个字面量、不 import `ux-queue.py`** ✗：import 会把队列工具的执行
    带进来（它有自己的 `ROOT`/IO 前提与副作用）—— 本判据要的只是**那本登记表**这个
    **数据** ✓（与 `check-queue-integrity.py` 的复核层**不 import** `ux-queue.py`
    **同一条口径** ✓）。读不出来 ⇒ 返回原因（调用方 `exit 3`，**不假装判过** ✗）。
    """
    p = root / REGISTRY_FILE
    if not p.is_file():
        return [], f"登记表 `{REGISTRY_FILE}` 不在 `{root}` ⇒ 判不了接线（**不假装判过** ✗）"
    try:
        tree = ast.parse(p.read_text(encoding="utf-8", errors="replace"))
    except SyntaxError as exc:
        return [], f"`{REGISTRY_FILE}` 解析不了（{exc}）⇒ 登记表读不出来"
    for node in tree.body:
        if not isinstance(node, ast.Assign):
            continue
        if not any(isinstance(t, ast.Name) and t.id == REGISTRY_NAME for t in node.targets):
            continue
        try:
            val = ast.literal_eval(node.value)
        except ValueError as exc:
            return [], (f"`{REGISTRY_FILE}` 的 `{REGISTRY_NAME}` 不是字面量集合（{exc}）"
                        f"⇒ 登记表读不出来")
        return sorted(str(x) for x in val), None
    return [], f"`{REGISTRY_FILE}` 里找不到 `{REGISTRY_NAME}` ⇒ 登记表读不出来"


def wired_script_paths(root: Path = ROOT) -> set[str]:
    """**接线面**：被 gate / CI 的 step **真的调用**过的**脚本路径**（任意扩展名 ✓）。

    与 `wired_scripts()` **同一条「读哪一层」**（`run_bodies()` / `live_lines()` ✓）——
    只有 token 形状不同（那个刻意 `.py`-only，理由见上面那段注释 ✓）。
    ⚠ **只收仓里真存在的文件**（幽灵名字不算 ✓，同 `wired_scripts()` ✓）。
    """
    found: set[str] = set()
    for rel in WIRING_FILES:
        p = root / rel
        if not p.is_file():
            continue
        text = p.read_text(encoding="utf-8", errors="replace")
        body = run_bodies(text) if rel.endswith(".yml") else text
        for tok in _ANY_SCRIPT_PATH_RE.findall(_non_comment_lines(body)):
            if (root / tok).is_file():
                found.add(tok)
    return found


def registered_wiring_problems(root: Path = ROOT) -> tuple[list[str], list[str], str | None]:
    """**C13 登记接线**：登记表点名的每一条都必须**真的被跑**（⇒ 红, 判到的登记项, 读表原因）。

    ⚠ 这是"登记"的**空转闸**（与 C4 **同形**，第二本登记表）：没有它，把一条判据写进
    `REGISTERED_SCRIPTS` 就等于**宣布它有人守** ✗ —— 而它可能**任何地方都不跑**
    （2026-10-17 实测：`scripts/vscode-input-vim-e2e.mjs`（1033 行 · 9 用例 × 2 臂）
    与 `scripts/vscode-input-e2e.mjs`（11 用例）**真接线 0/2** ✗ —— 修好了、写了判据、
    **登记了**，就是**没让它跑** ✗ ⇒ 判据从来没在 CI / gate 里跑过 ⇒ "修了不守 ⇒ 会漂" ✗）。

    ⚠ ⭐ **"真的跑"= `run:` 正文**（`run_bodies()` ✓ —— **注释与 step 名都不算** ✗；
    `ci-local.sh` 走非注释行 ✓）—— 与 C4 / `wired_scripts()` **逐字同一条口径** ✓。
    ⚠ 两条出路（写在红里 ✓）：①**真跑**它；②把它从登记表里删掉（登记 = 声明 ✗）。
    ⚠ **表不腐烂**（同层② R2 的形状 ✓）：点名了一个**不在树里**的脚本 ⇒ 也判红
    （那说明登记表在指一个**不存在**的判据 ✗）。
    """
    reg, err = registered_scripts(root)
    if err:
        return [], [], err
    wired = wired_script_paths(root)
    red: list[str] = []
    for rel in reg:
        if not (root / rel).is_file():
            red.append(f"**C13 登记表腐烂** ✗：登记表（`{REGISTRY_FILE}` 的 `{REGISTRY_NAME}`）"
                       f"点名的 `{rel}` **不在树里** ⇒ 那条判据**不存在**（登记表在指一个"
                       f"不存在的脚本 ✗）。出路二选一：①把文件补回来 ✓；"
                       f"②把它从登记表里删掉 ✓")
            continue
        if rel not in wired:
            red.append(f"**C13 登记接线** ✗：登记表（`{REGISTRY_FILE}` 的 `{REGISTRY_NAME}`）"
                       f"点名的 `{rel}` **没有任何一处真的跑它** ⇒ 它**登记了但没接线**"
                       f"（空转 ✗）。⚠ **\"真的跑\" = `run:` 正文**（`run_bodies()` ✓ —— "
                       f"注释与 step 名都不算 ✗；`{WIRING_FILES[0]}` 走非注释行 ✓）"
                       f"⇒ 出路二选一：①在 `{'` / `'.join(WIRING_FILES)}` 的正文里**真跑**它 ✓；"
                       f"②把它从登记表里删掉（登记 = 声明\"它是一条该跑的判据\" ✗）")
    return red, reg, None


def _git(root: Path, argv: list[str]) -> tuple[str, str | None]:
    # ⚠ `env=`（洞 `FIXTURE-GIT-ENV-LEAKS-INTO-REAL-REPO`）：`GIT_DIR` **优先于 cwd** ⇒
    #   从链接工作树的 hook 里跑时，本函数的 `git diff/log` 会量到**调用者的仓**，
    #   而 C1 的覆盖面就是从这里来的 ⇒ **判据会审错对象** ✗（仓库发现只许由 cwd 决定 ✓）。
    out = subprocess.run(["git", *argv], cwd=root, capture_output=True, text=True,
                         env=git_env.clean_git_env())
    if out.returncode != 0:
        return "", f"`git {' '.join(argv)}` rc={out.returncode}：{out.stderr.strip()[:200]}"
    return out.stdout, None


def changed_guards(root: Path, since: str | None) -> tuple[list[str], str | None]:
    """`since..HEAD` 里**新增/修改**的守卫（`--diff-filter=AMR` ⇒ 只算加/改/改名 ✓）。

    ⚠ **改名的形状**：`git diff` 报的是**新路径** ⇒ 它必须在**接线臂**里（改名后接线
    也改了 ⇒ 还在 ✓）或名字臂里 ⇒ **改名绕不过去** ✓（洞 `GUARD-ENUMERATION-IS-NAME-BASED`）。
    """
    if not since:
        return [], None
    out, err = _git(root, ["diff", "--name-only", "--diff-filter=AMR",
                           f"{since}..HEAD"])
    if err:
        return [], err
    wired = set(wired_scripts(root))
    return sorted({p for p in out.split() if is_guard(p, wired)}), None


def shallow_hint(root: Path) -> str:
    """本仓若是**浅克隆** ⇒ 给出**可操作的**人话（否则空串 ✓）。

    ⚠ 为什么要有（2026-10-16 **CI 实测** ✗→✓，`EDITOR-JOB-BITE-STEP-EXIT-3`）：
    `editor` job 的 checkout 没有 `fetch-depth: 0` ⇒ 覆盖面基线（台账抬头的
    `# coverage_since:`）那个对象不在 ⇒ `changed_guards` 拿到 `git` 的 rc=128 ⇒
    本文件**只印一行原始 git 报错**，读日志的人**看不出缺什么、更不知道怎么办** ✗。
    同族的 `check-attention-budget.py` / `check-queue-integrity.py` / `check-threshold-ratchet.py`
    **早就**给了人话（"浅克隆 ⇒ CI 里给这个 job `fetch-depth: 0`"）⇒ 这里补齐，口径一致 ✓。

    ⚠ **只改话术，不改判据** ✗：退出码仍是 3（"环境不在，**不假装判过**"），
    C1/C2 一个字没动 ⇒ **绝不静默判过** ✓。
    """
    try:
        out = subprocess.run(["git", "rev-parse", "--is-shallow-repository"],
                             cwd=root, capture_output=True, text=True, timeout=60,
                             env=git_env.clean_git_env())
    except (OSError, subprocess.SubprocessError):
        return ""
    if out.returncode != 0 or out.stdout.strip() != "true":
        return ""
    return ("本仓是**浅克隆**（`git rev-parse --is-shallow-repository` = true ⇒ "
            "**历史深度不够**，覆盖面基线那个对象不在 ⇒ 不是『这条守卫咬不住』 ✗）"
            "⇒ 出路：本机 `git fetch --unshallow`；**CI 里给跑这一步的那个 job 加 "
            "`fetch-depth: 0`** ✓（实测 `editor` job 缺的正是它：run 38817419732 · "
            "step 11；同 run 里 `guard-bite` 有它 ⇒ success ✓）。"
            "⚠ 别改判据来消这条红 ✗ —— 缺的是**前提**。")


def load_ledger(path: Path) -> tuple[list[dict], str, str | None]:
    """读台账 ⇒ (行, coverage_since, 错误)。`#` 开头的行是注释（照 `ux-queue-extras` 的先例 ✓）。"""
    if not path.exists():
        return [], "", (f"咬住证明台账不存在：`{path}` ⇒ **不假装判过** ✗。"
                        f"格式与写法见本文件 docstring；"
                        f"抬头的 `# coverage_since: <sha>` 是覆盖基线 ✓")
    entries: list[dict] = []
    coverage_since = ""
    for lineno, raw in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        line = raw.strip()
        if not line:
            continue
        if line.startswith("#"):
            if "coverage_since:" in line:
                coverage_since = line.split("coverage_since:", 1)[1].strip()
            continue
        try:
            entry = json.loads(line)
        except json.JSONDecodeError as err:
            return [], coverage_since, f"{path}:{lineno} 不是合法 JSON：{err}"
        missing = [k for k in ("guard", "bug", "pass", "bite") if k not in entry]
        if missing:
            return [], coverage_since, f"{path}:{lineno} 缺字段 {missing}"
        for half in ("pass", "bite"):
            if not isinstance(entry[half], dict) or "cmd" not in entry[half] \
                    or "expect_rc" not in entry[half]:
                return [], coverage_since, (f"{path}:{lineno} 的 `{half}` 要 "
                                            f"{{'cmd': …, 'expect_rc': …}}")
        entries.append(entry)
    return entries, coverage_since, None


# ── **锚登记表**（C6 覆盖 / C7 有底 / C8 预算 / C9 工具登记；唯一一处 ✓）────────
# `tool`（2026-10-16，随**接线臂**落地 ✓）：接线臂收进来但**不是独立判据**的文件
# （工具 / 渲染器 / 自检模式）⇒ 记 `tool` + `why`（必填）+ `covered_by`（可选，悬空判红 ✗）。
# ⚠ **为什么不记成 `guard`**：那会撞 C8 的**绝对上限**（B 21/21）⇒ 见 docstring 的 C9 ✓。
ANCHOR_KINDS = ("anchor", "guard", "caps", "tool")


def load_anchors(path: Path) -> tuple[list[dict], str | None]:
    """读分类登记表 ⇒ (行, 错误)。**缺失/坏 ⇒ 错误**（**绝不静默判过** ✗，与台账同口径 ✓）。

    ⚠ 为什么缺文件是**错误**而不是"没分类 ⇒ 全绿"✗：修前那份仓库状态就是
    「32 个守卫 · 0 条分类」—— 那正是 C6 要判红的形状 ✓（**不许**因为表不在就放行 ✗）。
    """
    if not path.exists():
        return [], (f"分类登记表不存在：`{path}` ⇒ **不假装判过** ✗（C6 覆盖 / C7 有底 / C8 预算 "
                    f"都要它）。格式与写法见本文件 docstring 的 C6/C7/C8 ✓")
    rows: list[dict] = []
    for lineno, raw in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        try:
            row = json.loads(line)
        except json.JSONDecodeError as err:
            return [], f"{path}:{lineno} 不是合法 JSON：{err}"
        kind = row.get("kind")
        if kind not in ANCHOR_KINDS:
            return [], (f"{path}:{lineno} 的 `kind` 只能是 {' / '.join(ANCHOR_KINDS)}"
                        f"（给的是 {kind!r}）")
        if kind == "tool":
            # ⚠ **必填两项**（C9 ✓）：没有 `why` 的 tool 行 = 一句"我不是判据"的免死金牌 ✗
            if not str(row.get("tool", "")).strip():
                return [], (f"{path}:{lineno} 的 `kind: tool` 行必须写 `tool`"
                            f"（仓库相对路径 ✓）")
            if not str(row.get("why", "")).strip():
                return [], (f"{path}:{lineno} 的 `kind: tool` 行必须写 `why`"
                            f"（**它为什么不是独立判据** —— 说不出来就不许登记成 tool ✗）")
        rows.append(row)
    return rows, None


def anchor_problems(root: Path, rows: list[dict]) -> tuple[list[str], list[str], dict]:
    """**纯函数**：C6 覆盖 + C7 有底 + C8 预算 ⇒ (红, 信息, 计数)。

    ⚠ **A 类不进任何上限**（用户【①】：锚定型**无限预算** ✓）⇒ "A 类增长 ⇒ 判红"是
    **结构性不可能**：`counts` 里的 A 只被**印出来**，从不与任何 cap 比较 ✓。
    """
    red: list[str] = []
    info: list[str] = []
    scope = guard_scripts(root)
    guards = {r["guard"]: r for r in rows if r.get("kind") == "guard"}
    tools = {r["tool"]: r for r in rows if r.get("kind") == "tool"}
    anchors = {r["id"]: r for r in rows if r.get("kind") == "anchor"}
    caps_row = next((r for r in rows if r.get("kind") == "caps"), None)

    # ── C6 覆盖：口径里的每一个都要有**分类**（`guard` 或 `tool` ✓）；
    #    表里也不许点名口径外的（防表自己腐烂 ✗）────────────────────────────
    for g in scope:
        if g not in guards and g not in tools:
            red.append(f"**C6 覆盖** ✗：`{g}` **没有分类**（登记表里既没有它的 "
                       f"`kind: guard` 行、也没有 `kind: tool` 行）⇒ 它保护的**用户现场**"
                       f"说不出来 ⇒ 这一层**没有底** ✗（在 `{ANCHORS.name}` 补一行："
                       f"`guard` / `class` A|B / `anchors` ✓；**若它不是独立判据** ⇒ "
                       f"`tool` / `why` / `covered_by` ✓）")
    for g in sorted(set(guards) | set(tools)):
        if g not in scope:
            red.append(f"**C6 表腐烂** ✗：登记表点名了 `{g}`，但它**不在** `guard_scripts()` "
                       f"口径里（文件没了 / 改了名 / 不在 gate·CI 的接线里）⇒ 表与仓库两头漂移 ✗")

    # ── C9 工具登记（`kind: tool`）：`why` 在 `load_anchors` 里必填 ✓；
    #    `covered_by` 点名一条**独立覆盖它的守卫** ⇒ 悬空 ⇒ 判红 ✗ ────────────
    coverless: list[str] = []
    for t, row in sorted(tools.items()):
        cb = str(row.get("covered_by") or "").strip()
        if not cb:
            coverless.append(t)
            continue
        if cb not in guards:
            red.append(f"**C9 悬空覆盖** ✗：`{t}` 的 `covered_by` 点名 `{cb}`，但登记表里"
                       f"**没有它的 `kind: guard` 行** ⇒ 它声称的独立覆盖**不存在** "
                       f"（悬空 ⇒ 这条 `tool` 行**没底** ✗）")

    # ── C7 有底：每条守卫行必须指回一条**存在且写全**的 A 类锚（用户现场）──
    for g, row in sorted(guards.items()):
        cls = row.get("class")
        if cls not in ("A", "B"):
            red.append(f"**C7** ✗：`{g}` 的 `class` 只能是 `A` / `B`（给的是 {cls!r}）")
            continue
        anc = row.get("anchors")
        if not isinstance(anc, list) or not anc:
            red.append(f"**C7 没底** ✗：`{g}` 的 `anchors` 必须**非空**（一条守卫说不出它保护的"
                       f"用户现场 ⇒ 它就是**收窄/合并的候选** ✗）")
            continue
        for aid in anc:
            if aid not in anchors:
                red.append(f"**C7 悬空锚** ✗：`{g}` 指向 `{aid}`，但登记表里**没有这条锚** ⇒ "
                           f"它的底**悬空**（B 只许指向 `kind: anchor` 的**用户现场** ✓）")
    for aid, row in sorted(anchors.items()):
        for key in ("site", "action", "reading"):
            if not str(row.get(key, "")).strip():
                red.append(f"**C7 锚没写全** ✗：锚 `{aid}` 缺 `{key}` ⇒ 它**不是**一个用户现场"
                           f"（`site` 用户在哪 / `action` 用户做什么 / `reading` 怎么量）✗")
        if row.get("strength") not in ("direct", "process"):
            red.append(f"**C7** ✗：锚 `{aid}` 的 `strength` 只能是 `direct` / `process`")

    # ── C8 预算：B 类**绝对上限** + 只许减；A 类**不进上限** ──
    a_list = sorted(g for g, r in guards.items() if r.get("class") == "A")
    b_list = sorted(g for g, r in guards.items() if r.get("class") == "B")
    if caps_row is None:
        red.append("**C8** ✗：登记表里没有 `kind: caps` 行 ⇒ **B 类没有上限** ⇒ 判据可以无限加 ✗")
        cap = floor = None
    else:
        cap = (caps_row.get("caps") or {}).get("b_class_guards")
        floor = (caps_row.get("floor") or {}).get("b_class_guards")
        hist = caps_row.get("history") or []
        if not isinstance(cap, int) or not isinstance(floor, int):
            red.append("**C8** ✗：`caps` / `floor` 里的 `b_class_guards` 必须是整数")
        else:
            if cap < floor:
                red.append(f"**C8** ✗：`b_class_guards` = {cap} **低于收敛目标** {floor} ⇒ "
                           f"状态被手改过 ✗")
            if int(caps_row.get("round", -1)) != len(hist) - 1:
                red.append(f"**C8** ✗：`round` = {caps_row.get('round')} 与 `history` 长度 "
                           f"{len(hist)} 不自洽（round 应 = len(history) − 1）")
            prev = None
            for entry in hist:
                cur = (entry.get("caps") or {}).get("b_class_guards")
                if not isinstance(cur, int):
                    red.append(f"**C8** ✗：history 第 {entry.get('round')} 轮缺 `b_class_guards`")
                    continue
                if prev is not None:
                    if cur > prev:
                        red.append(f"**C8 放松** ✗：`b_class_guards` 第 {entry.get('round')} 轮 "
                                   f"{cur} > 上一轮 {prev} ⇒ 上限被抬过 ✗（方向只能是收紧）")
                    elif cur == prev and cur > floor:
                        red.append(f"**C8 空转** ✗：`b_class_guards` 第 {entry.get('round')} 轮与"
                                   f"上一轮都停在 {cur}（还没到收敛目标 {floor}）⇒ ratchet 空转了 ✗")
                prev = cur
            if hist and (hist[-1].get("caps") or {}).get("b_class_guards") != cap:
                red.append("**C8** ✗：`caps` 与 `history` 末轮不一致（同一份真相不许两处各写一份）")
            if len(b_list) > cap:
                red.append(f"**C8 超预算** ✗：B 类 **{len(b_list)} 条 > 上限 {cap}** ⇒ "
                           f"守卫型判据在增殖 ✗。出路（**只许减** ✓）：把同一关注点的 B 合并成"
                           f"一本登记表 + 一个驱动器，或删掉指不回用户现场的那条 ⇒ "
                           f"**新加一条 B ⇒ 必须删/合并一条**（零和 ✓）")

    # ── ⓘ 只报告（**不判红** ✓）：弱锚 / 没人量的锚 —— 这两条是**收窄的候选清单** ✓ ──
    weak = sorted(g for g in b_list if any(anchors.get(x, {}).get("strength") == "process"
                                           for x in (guards[g].get("anchors") or [])))
    measured: set[str] = set()
    for r in guards.values():
        measured.update(r.get("anchors") or [])
    unmeasured = sorted(set(anchors) - measured)
    if weak:
        info.append(f"ⓘ **弱锚 B**（指回的是 `process` 锚：量流程、后果落在用户身上）"
                    f"{len(weak)}/{len(b_list)} 条：{' · '.join(weak)}")
    if unmeasured:
        info.append(f"ⓘ **没人量的锚**（是缺口，不是红）：{' · '.join(unmeasured)}")
    if coverless:
        info.append(f"ⓘ **没有独立覆盖的工具**（`kind: tool` 里没写 `covered_by` 的 —— "
                    f"是缺口，不是红：它们**不是判据**，但也没有第二条独立判据看着它）："
                    f"{' · '.join(coverless)}")
    counts = {"a": len(a_list), "b": len(b_list), "total": len(scope),
              "guards": len(guards),
              "tools": len(tools), "coverless_tools": coverless,
              "anchors": len(anchors), "cap": cap, "floor": floor,
              "weak_b": len(weak), "unmeasured_anchors": unmeasured,
              "b_guards": b_list, "a_guards": a_list}
    return red, info, counts


# ── **C12 双面**（本地自动面 ⇄ CI 面；**唯一一处** ✓）────────────────────────────
#
# 洞 `ZERO-OBS-GATE-CENSUS-BY-BEHAVIOUR` 第 4 轮（2026-10-17，`ng4`）。修前"两面"这个
# 口径只长在 `.cache/ux/fix-ng*/census.py`（**证据**、不是闸 ✓）：`wired_scripts()` 是
# `ci-local.sh` **∪** `ci.yml` 的**并集** ⇒ **只删 CI 面、本地还在** ⇒ 枚举不变 ⇒
# C6 照样绿 ✗。C12 把那一格钉住。
#
# 口径（与 `.cache/ux/fix-ng{1..4}/census.py` 的 (d) **逐条同读** —— 实测 41 条上
# 读数**完全相同** ✓：两面都有 29 · CI 面缺 6 · 本地面缺 6 · 两面都缺 0）：
#   ⚠ 那两个 6 的**现状**（第 5 轮 `ng6`）：C12 判红 `plan.py` 后补上 CI 面、又把
#   `check-site.py` 接进本地面 ⇒ **两面都有 31 · CI 面缺 5 · 本地面缺 5 · 两面都缺 0**
#   （两个 5 全部有闭集 witness ✓ · `total` 仍 41 ✓ —— 枚举一个字没动 ✗）。
#   * **本地面** = `scripts/ci-local.sh` 的非注释行 ∪ `scripts/githooks/*` 的非注释行
#     （两条自动路：`pre-push` → `ci-local.sh --fast` ✓ · `core.hooksPath` 的钩子 ✓）；
#   * **CI 面** = **任一** `.github/workflows/*.yml` 的 **`run:` 正文**（复用
#     `run_bodies()` ✓ —— 注释与 step 名里点名**不算**调用，同接线臂 ✓）。
#   ⚠ **如实划的两条边界** ✗（不是密封）：①本判据**不建模** `if [ "$FAST" = 0 ]` 门控
#   （本地面按"非注释行"算）—— 实测 41 条里**没有**任何守卫只出现在门控步里（门控步
#   4 个 · 只在门控步里的守卫 **0** 条 ✓），且差异只可能在**更严**那一侧（只在门控步 +
#   CI 面也缺 ⇒ 照样判红 ✓）；②行内注释不剥（与 `_non_comment_lines()` **同一个口径** ✓，
#   实测与普查逐条同读 ✓）。
#
# **判红（两面都判）**：第 4 轮（`ng4`）**刻意不对称** —— CI 侧缺 ⇒ 判红（不跑 hook 的
#   推送上它**整个不存在** ✗）；本地侧缺 ⇒ **只印 `ⓘ`**（保护还在、只是红**晚** 33–35 分钟
#   被发现 ✗），因为当时唯一那一条真洞（`scripts/check-site.py`）接上去**当场会红**
#   （实测 rc=1 · `site/data/site.json` 与最新 tag 不一致 = **没有出路的红** · E18）。
#   第 5 轮（`ng6`）那条前提**消失了**（`ng5` 已把红修掉 ⇒ `10/10 rc=0` ✓）⇒ 收紧成**对称**：
#   本地侧缺**也判红**（豁免仍只走闭集 witness ✓）。逐条读数见 `.cache/ux/fix-ng4/VERDICT.md`
#   （留洞那一刻）与 `.cache/ux/fix-ng6/VERDICT.md`（收紧这一刻）。
#
# **豁免**（登记表 `docs/guard-anchors.jsonl` 的 `guard`/`tool` 行加 `faces` 字段）：
#     "faces": {"ci": {"witness": "<闭集里的一档>", "object": "<只有 local-artifact 用>"}}
# ⚠⭐ **数据只能「选」witness，不能「定义」它**（一档一份实现、就在下面 ✓ ⇒ 加一档 =
#   改本判据 = 一次评审 ✓）；每档都要过**判据自己现算**的机械事实 ⇒ **事实消失 ⇒ 豁免
#   当场失效（判红）** ✓；**豁免腐烂**（声明那一面不存在、其实有）也判红 ✗ ⇒ 这就是它
#   **不会变成"什么都豁免"**的那条底 ✓（逐档理由见下表）。
FACE_WITNESSES = {
    "tool": "该行本来就是 `kind: tool`（**不是独立判据** ⇒ 没有「该在哪判」这回事 ✓）",
    "masked-readout": "它在本地面上的调用点**全部**在 `soft()` 里（rc 不传播 ⇒ 它是"
                      "**读数不是闸** ⇒ 两面都没有判红义务 ✓）",
    "local-artifact": "它判的是**本机产物**（声明的 `object` **不在** `git ls-files` 里、"
                      "且被 `git check-ignore` 命中 ⇒ CI 的干净检出里**没有这个对象** ✓）",
    "pre-push-object": "它的自动面是 **git 钩子**（推送前那一刻）：对象是**那一刻才有的**"
                       "（钩子实参 `\"$1\"` / `--from-hook` 对照表 / 远端未完成的 run ✓）"
                       "⇒ CI 上没有那个时刻 ✓",
    "binary-precondition": "它需要**一份不在仓库里的可执行产物**：喂它一个解析不到的"
                           "`SOKONANODA_BIN` ⇒ 它**自报判不了（rc=3）**而不是判绿 ⇒ "
                           "接进 pre-push 会在**没编过的树**里结构性假红 ✗",
}
FACE_NAMES = ("local", "ci")
FACE_LOCAL_FILES = ("scripts/ci-local.sh",)
FACE_HOOK_DIR = "scripts/githooks"
FACE_WORKFLOW_GLOB = ".github/workflows/*.yml"
FACE_PROBE_TIMEOUT_S = 120


def _face_scripts(text: str, root: Path) -> set[str]:
    """文本里**真存在**的 `scripts/*.py`（幽灵名字不算 ✓ —— 同 `wired_scripts()` 的口径 ✓）。"""
    return {tok for tok in _SCRIPT_PATH_RE.findall(text) if (root / tok).is_file()}


def _face_shell_lines(text: str) -> str:
    """shell 文件的**非注释行**：整行注释 + **行内注释**都去掉 ✓。

    ⚠ **行内注释必须去**（**实测** ✗，2026-10-17）：`ci-local.sh` 的 `soft()` **定义行**
    尾巴上写着「理由见 `scripts/release-metric.py` 头部」⇒ 不去的话 `release-metric.py`
    会被算成**一处 `run` 步**（它其实**只有**一处 `soft` 调用）⇒ `masked-readout` 的
    witness 当场**假红** ✓（C12 自己逮到的第一件事）。口径与 `.cache/ux/fix-ng*/census.py`
    的 `strip_comments()` **逐字同一条**（只认**前面有空白**的 `#` ✓）。
    ⚠ **不动 `_non_comment_lines()`** ✗ —— 它是 `wired_scripts()`（接线臂 / C6 的口径）用的，
    改它会动**枚举**（`guard_scripts()`）⇒ 那是另一件事 ✓。
    """
    out: list[str] = []
    for ln in text.splitlines():
        if ln.strip().startswith("#"):
            continue
        for i, ch in enumerate(ln):
            if ch == "#" and i > 0 and ln[i - 1] in " \t":
                ln = ln[:i]
                break
        out.append(ln)
    return "\n".join(out)


def _face_code(root: Path, rel: str) -> str:
    """一个文件的**非注释源码**（点名要真的在**代码**里 —— 注释里点名不算 ✗）。

    shell（`ci-local.sh` / `scripts/githooks/*`）走**行内注释也剥**那一档 ✓；
    Python 守卫沿用 `_non_comment_lines()`（与接线臂**同一个口径** ✓，实测与普查逐条同读 ✓）。
    """
    p = root / rel
    if not p.is_file():
        return ""
    text = p.read_text(encoding="utf-8", errors="replace")
    if rel.endswith(".sh") or rel.startswith(f"{FACE_HOOK_DIR}/"):
        return _face_shell_lines(text)
    return _non_comment_lines(text)


def local_face_scripts(root: Path = ROOT) -> set[str]:
    """**本地自动面**：`ci-local.sh` 非注释行 ∪ `scripts/githooks/*` 非注释行 ✓（唯一一处 ✓）。"""
    found: set[str] = set()
    for rel in FACE_LOCAL_FILES:
        found |= _face_scripts(_face_code(root, rel), root)
    hooks = root / FACE_HOOK_DIR
    if hooks.is_dir():
        for h in sorted(hooks.iterdir()):
            if h.is_file():
                found |= _face_scripts(_face_code(root, f"{FACE_HOOK_DIR}/{h.name}"), root)
    return found


def ci_face_scripts(root: Path = ROOT) -> set[str]:
    """**CI 面**：**任一** workflow 的 `run:` 正文里真调用的 `scripts/*.py` ✓（唯一一处 ✓）。"""
    found: set[str] = set()
    for wf in sorted(root.glob(FACE_WORKFLOW_GLOB)):
        if wf.is_file():
            found |= _face_scripts(run_bodies(wf.read_text(encoding="utf-8", errors="replace")), root)
    return found


def _face_git(root: Path, argv: list[str]) -> tuple[int, str]:
    """C12 要**退出码**（`check-ignore` 的 0/1/128 三档语义不同 ✓）⇒ 与 `_git()` 分开这一小档。

    ⚠ 同样 `env=git_env.clean_git_env()`（`GIT_DIR` **优先于 cwd** ⇒ 不清会量到**调用者**的仓 ✗）。
    """
    out = subprocess.run(["git", *argv], cwd=root, capture_output=True, text=True,
                         errors="replace", env=git_env.clean_git_env())
    return out.returncode, out.stdout


def _face_witness_problems(root: Path, g: str, face: str, spec: dict, row: dict) -> list[str]:
    """一条 `faces` 声明要过的**机械事实**（**一档一份实现** ✓）。空 = 成立 ✓。"""
    red: list[str] = []
    kind = spec.get("witness")
    if kind not in FACE_WITNESSES:
        return [f"**C12 witness 不认** ✗：`{g}` 的 `{face}` 面声明了 {kind!r} —— 只许闭集里的 "
                f"{sorted(FACE_WITNESSES)}（**数据只能选、不能定义** ✗）"]
    extra = sorted(set(spec) - {"witness", "object"})
    if extra:
        red.append(f"**C12 witness 多余字段** ✗：`{g}` 的 `{face}` 声明里多了 {extra}"
                   f"（只许 `witness` / `object`）")
    code = _face_code(root, g)
    if kind == "tool":
        if row.get("kind") != "tool":
            red.append(f"**C12 witness 不成立** ✗：`{g}` 声明 `tool`，但登记表里它不是 "
                       f"`kind: tool` 行 ⇒ 「我不是独立判据」这句话**没有登记表撑腰** ✗")
    elif kind == "masked-readout":
        hits = [ln for ln in _face_code(root, FACE_LOCAL_FILES[0]).splitlines()
                if g in ln or Path(g).name in ln]
        if not hits:
            red.append(f"**C12 witness 不成立** ✗：`{g}` 声明 `masked-readout`，但它在 "
                       f"`{FACE_LOCAL_FILES[0]}` 的**非注释行**里一处也没有 ⇒ 它**不在本地面** ⇒ "
                       f"这个 witness 是空话 ✗")
        elif not all(ln.strip().startswith("soft ") for ln in hits):
            red.append(f"**C12 witness 不成立** ✗：`{g}` 声明 `masked-readout`，但它在本地面"
                       f"**不是全部**被 `soft()` 包着（找到 {len(hits)} 处，其中 "
                       f"{sum(1 for ln in hits if not ln.strip().startswith('soft '))} 处是 `run` 步 "
                       f"⇒ **它会判红** ⇒ 判红语义存在 ⇒ 该接进 CI ✗）")
    elif kind == "local-artifact":
        obj = str(spec.get("object") or "").strip()
        if not obj or obj.startswith("/") or ".." in obj.split("/"):
            red.append(f"**C12 witness 不成立** ✗：`{g}` 的 `local-artifact` 必须写一个"
                       f"**仓库相对**的 `object`（给的是 {obj!r}）")
        else:
            trc, tout = _face_git(root, ["ls-files", "--cached", "--", obj])
            irc, _iout = _face_git(root, ["check-ignore", "-q", "--", obj])
            if trc != 0 or irc not in (0, 1):
                red.append(f"**C12 witness 判不了** ✗：`{g}` 的 `object` = `{obj}` 在 `{root}` 上"
                           f"问不了 git（`ls-files` rc={trc} · `check-ignore` rc={irc}）⇒ "
                           f"**不假装判过** ✗")
            elif tout.strip():
                red.append(f"**C12 witness 不成立** ✗：`{g}` 声明 `{obj}` 是**本机产物**，但它"
                           f"**在仓库里**（`git ls-files` 命中）⇒ CI 的检出**有**这个对象 ⇒ "
                           f"CI 面在语义上**存在** ✗")
            elif irc != 0:
                red.append(f"**C12 witness 不成立** ✗：`{g}` 声明 `{obj}` 是**本机产物**，但它"
                           f"**没有被 `git check-ignore` 命中**（不是被忽略的本机产物）✗")
            if obj and obj not in code:
                red.append(f"**C12 witness 不成立** ✗：`{g}` 声明它的对象是 `{obj}`，但 `{obj}` "
                           f"在**它自己的非注释源码**里一次都没出现 ⇒ 声明与闸对不上"
                           f"（**声明不能凭空指一个别处的路径** ✗）")
    elif kind == "pre-push-object":
        hook_names: list[str] = []
        hooks_dir = root / FACE_HOOK_DIR
        if hooks_dir.is_dir():
            for h in sorted(hooks_dir.iterdir()):
                if not h.is_file():
                    continue
                htext = _face_code(root, f"{FACE_HOOK_DIR}/{h.name}")
                if g in htext or Path(g).name in htext:
                    hook_names.append(h.name)
        if not hook_names:
            red.append(f"**C12 witness 不成立** ✗：`{g}` 声明 `pre-push-object`，但它不在 "
                       f"`{FACE_HOOK_DIR}/` 的**非注释行**里 ⇒ 它没有「推送前那一刻」这条面 ✗")
        else:
            hook_text = "\n".join(_face_code(root, f"{FACE_HOOK_DIR}/{n}") for n in hook_names)
            if not ('"$1"' in hook_text or "--from-hook" in hook_text or "gh" in code):
                red.append(f"**C12 witness 不成立** ✗：`{g}` 声明 `pre-push-object`，但它所在的"
                           f"钩子**没传「那一刻才有的实参」**（`\"$1\"` / `--from-hook`）、"
                           f"它的源码里也没有 `gh`（远端 run 状态）⇒ 对象**不是那一刻才有的** ✗")
    elif kind == "binary-precondition":
        if "SOKONANODA_BIN" not in code:
            red.append(f"**C12 witness 不成立** ✗：`{g}` 声明 `binary-precondition`，但它的"
                       f"非注释源码里**没有** `SOKONANODA_BIN` ⇒ 它**不吃这个前提** ✗")
        elif (root / g).resolve() == Path(__file__).resolve():
            red.append("**C12 witness 判不了** ✗：探针的对象是**本文件自己** ⇒ 会递归 ⇒ "
                       "**不假装判过** ✗（换一档 witness，或把这档的探针改成不跑自己 ✗）")
        else:
            missing = str(Path(tempfile.gettempdir()) / "soko-c12-no-such-binary")
            try:
                proc = subprocess.run([sys.executable, str(root / g)], cwd=root,
                                      capture_output=True, text=True, encoding="utf-8",
                                      errors="replace", timeout=FACE_PROBE_TIMEOUT_S,
                                      env={**os.environ, "SOKONANODA_BIN": missing})
                if proc.returncode != 3:
                    red.append(f"**C12 witness 不成立** ✗：`{g}` 声明 `binary-precondition`，但喂它 "
                               f"`SOKONANODA_BIN={missing}` 时它 **rc={proc.returncode}**（不是 3）⇒ "
                               f"它并不「判不了」，而是**能判** ⇒ 该接进本地面 ✗")
            except subprocess.TimeoutExpired:
                red.append(f"**C12 witness 判不了** ✗：`{g}` 的探针超时（>{FACE_PROBE_TIMEOUT_S}s）"
                           f"⇒ **不假装判过** ✗")
    return red


def face_problems(root: Path = ROOT, rows: list[dict] | None = None) -> tuple[list[str], list[str], dict]:
    """**C12 双面** ⇒ (红, 信息, 计数)。**唯一一处** ✓（口径与豁免的闭集见上面那段注释 ✓）。

    ⚠ **两面都判红**（**第 5 轮 `ng6` 收紧** —— 第 4 轮时本地侧刻意只印 `ⓘ` ✗）：
    * **CI 侧缺 ⇒ 判红**：本地面有、CI 面没有 ⇒ 在**不跑 hook 的推送**上它**整个不存在**
      ⇒ 那条闸的**全部**保护都蒸发了 ✗（这是"不存在"，不是"晚知道"）。
    * **本地侧缺 ⇒ 也判红**（第 5 轮改）：CI 面有、本地面没有 ⇒ 保护**在**、只是红**晚**
      33–35 分钟被发现 ✗。⚠ **第 4 轮为什么只印**：当时唯一那条真洞（`scripts/check-site.py`）
      接上去**当场会红**（实测 `rc=1`：`site/data/site.json` 与最新 tag 不一致；`pages.yml`
      是**路径门控**的 ⇒ 这条红**在真仓里已经存在**、只是没人看见）⇒ 按 `E18`「守卫一旦叫
      狼来了，正确的反应就变成加豁免 ⇒ 它就死了」**如实留洞 + 印出来**，**不许**用豁免盖住 ✗。
      ⚠ **第 5 轮凭什么能收紧**：前提**消失了** —— `ng5` 走完发布闭环（`site.json` 跟到最新
      tag ⇒ `10/10 rc=0` ✓），再把 `check-site.py` 接进 `scripts/ci-local.sh`（**零前提**那档 ✓）
      ⇒ 本地侧**一条真洞都不剩**（`local_open` 计数照样印出来 ✓）⇒ 判红**当场不误伤任何人** ✓。
    ⚠ **豁免路径两面共用**（`_face_witness_problems()` 一档一份实现 ✓，闭集没动 ✗）。
    ⚠ **`info` 形参保留、C12 现在恒空**（改返回形状要动 8 条自检臂的调用 ✗ ⇒ 不值）。
    ⚠ 与 C6 的分工：**没分类的守卫由 C6 判**（这里跳过、不重复判 ✗）。
    """
    red: list[str] = []
    info: list[str] = []
    if rows is None:
        rows, err = load_anchors(ANCHORS)
        if err:
            return [f"**C12 判不了** ✗：{err}"], [], {}
    scope = guard_scripts(root)
    local = local_face_scripts(root)
    ci = ci_face_scripts(root)
    by_path: dict[str, dict] = {}
    for r in rows:
        if r.get("kind") == "guard" and r.get("guard"):
            by_path[str(r["guard"])] = r
        elif r.get("kind") == "tool" and r.get("tool"):
            by_path[str(r["tool"])] = r
    counts: dict = {"total": len(scope), "both": 0, "ci_missing": 0, "local_missing": 0,
                    "neither": 0, "ci_exempt": 0, "local_exempt": 0,
                    "ci_open": [], "local_open": [], "neither_open": [],
                    "witnesses": {}}
    for g in scope:
        row = by_path.get(g)
        if row is None:
            continue                      # 没分类 ⇒ **C6 覆盖**在判它（不重复判 ✗）
        decl = row.get("faces") or {}
        if not isinstance(decl, dict):
            red.append(f"**C12** ✗：`{g}` 的 `faces` 必须是**对象**（面 → witness ✓）")
            continue
        have = {"local": g in local, "ci": g in ci}
        for face in sorted(decl):
            if face not in FACE_NAMES:
                red.append(f"**C12 未知面** ✗：`{g}` 的 `faces` 里出现 `{face}`"
                           f"（只能是 {' / '.join(FACE_NAMES)}）")
                continue
            if have[face]:
                red.append(f"**C12 豁免腐烂** ✗：`{g}` 声明 `{face}` 面不存在，但它**有**那一面 "
                           f"（接线在 `{' / '.join(FACE_LOCAL_FILES)}` 或 `{FACE_HOOK_DIR}/*` 或 "
                           f"`{FACE_WORKFLOW_GLOB}` 的 `run:` 正文里 ✓）⇒ 豁免是**多余的** ⇒ "
                           f"**删掉这条 `faces` 声明** ✗")
                continue
            spec = decl[face]
            if not isinstance(spec, dict):
                red.append(f"**C12** ✗：`{g}` 的 `faces.{face}` 必须是对象"
                           f"（`{{\"witness\": …}}` ✓）")
                continue
            wred = _face_witness_problems(root, g, face, spec, row)
            red.extend(wred)
            if not wred:
                counts[f"{face}_exempt"] += 1
                w = str(spec.get("witness"))
                counts["witnesses"][w] = counts["witnesses"].get(w, 0) + 1
        if have["local"] and have["ci"]:
            counts["both"] += 1
        elif have["local"] and not have["ci"]:
            counts["ci_missing"] += 1
            if "ci" not in decl:
                counts["ci_open"].append(g)
                red.append(
                    f"**C12 CI 面缺** ✗：`{g}` **本地面有它**（`{' / '.join(FACE_LOCAL_FILES)}` 的"
                    f"非注释行 / `{FACE_HOOK_DIR}/*` ✓）但 **CI 面没有**（**任一** workflow 的 "
                    f"`run:` 正文里都没有它）⇒ 在**不跑 hook 的推送**（`--no-verify` / 机器人 / "
                    f"没装 `core.hooksPath` 的检出）上它**整个不存在** ✗。出路二选一："
                    f"①接进某个 workflow 的 `run:` 正文（**注释 / step 名里点名不算** ✗）；"
                    f"②若它的判据对象**在 CI 上按定义不存在** ⇒ 在 `{ANCHORS.name}` 那一行加 "
                    f"`\"faces\": {{\"ci\": {{\"witness\": \"…\"}}}}`（闭集："
                    f"{' / '.join(sorted(FACE_WITNESSES))} —— **数据只能选、不能定义** ✓）")
        elif not have["local"] and not have["ci"]:
            counts["neither"] += 1
            if "local" not in decl or "ci" not in decl:
                counts["neither_open"].append(g)
                red.append(f"**C12 两面都缺** ✗：`{g}` 在 `{' / '.join(FACE_LOCAL_FILES)}` / "
                           f"`{FACE_HOOK_DIR}/*` 与**任一** workflow 的 `run:` 正文里**都没有** ⇒ "
                           f"它**谁都不跑**（名字臂收进来的守卫也可能一格都没接 ⇒ 同族病 ✗）⇒ "
                           f"接进任一面，或按闭集 witness 声明**两面** ✓")
        else:
            counts["local_missing"] += 1
            if "local" not in decl:
                counts["local_open"].append(g)
                red.append(
                    f"**C12 本地面缺** ✗：`{g}` **CI 面有它**（`{FACE_WORKFLOW_GLOB}` 的 `run:` "
                    f"正文 ✓）但 **本地面没有**（`{' / '.join(FACE_LOCAL_FILES)}` 的**非注释行** / "
                    f"`{FACE_HOOK_DIR}/*` 里都没有它）⇒ 这条红**只能走到 CI 才被发现**"
                    f"（实测一轮 CI 33–35 分钟 ✗）。出路二选一：①接进 "
                    f"`{FACE_LOCAL_FILES[0]}` 的**非注释行**（**注释 / step 名里点名不算** ✗；"
                    f"**零前提 + 亚秒级**的才该无条件跑 ✓）或 `{FACE_HOOK_DIR}/*`；②若它的判据"
                    f"对象**在本地自动面上按定义不存在** ⇒ 在 `{ANCHORS.name}` 那一行加 "
                    f"`\"faces\": {{\"local\": {{\"witness\": \"…\"}}}}`（闭集："
                    f"{' / '.join(sorted(FACE_WITNESSES))} —— **数据只能选、不能定义** ✓）")
    return red, info, counts


def run_proof(cmd: str, root: Path, timeout: int = PROOF_TIMEOUT_S) -> tuple[int, str]:
    """跑一条证明 ⇒ (rc, 尾行摘要)。**唯一一处** ✓（`shell=True`：证明是**一条命令** ✓）。"""
    try:
        proc = subprocess.run(cmd, shell=True, cwd=root, capture_output=True, text=True,
                              encoding="utf-8", errors="replace", timeout=timeout)
    except subprocess.TimeoutExpired:
        return 124, f"证明超时（>{timeout}s）⇒ 按**判红**处理（不许静默跳过 ✗）"
    tail = [r.strip() for r in (proc.stdout + proc.stderr).splitlines() if r.strip()]
    return proc.returncode, " / ".join(r[:160] for r in tail[-2:])


def check(root: Path, ledger_path: Path, since: str | None, run_all: bool,
          reopen_empty: bool = False, quiet: bool = False,
          job: str = DEFAULT_JOB, anchors_path: Path | None = None) -> int:
    """C1 覆盖 + C2 咬得住（+ C3 翻账 + C4 接线 + C6/C7/C8 锚 + C13 登记接线）。

    `job` = 这一次跑**哪个 CI job 的那批证明**（`JOB_ALL` ⇒ 不分工，开发机用 ✓）。
    """
    entries, coverage_since, err = load_ledger(ledger_path)
    if err:
        print(f"error: {err}", file=sys.stderr)
        return 3
    by_guard = {e["guard"]: e for e in entries}
    base = since or coverage_since
    if not base:
        print("error: 台账抬头没有 `# coverage_since: <sha>`，也没给 `--since` ⇒ "
              "覆盖面算不了（**不假装判过** ✗）", file=sys.stderr)
        return 3

    scope, cerr = changed_guards(root, base)
    if cerr:
        # ⚠ **响亮 + 印出缺什么**（2026-10-16 CI 实测）：光印 `git` 的 rc=128
        # 读不出"缺的是历史深度" ⇒ 浅克隆时补一行**可操作**的人话 ✓。
        # 退出码**仍是 3**（环境不在 ⇒ 不假装判过 ✗）—— 只是话术，不是放宽 ✓。
        print(f"error: {cerr}", file=sys.stderr)
        hint = shallow_hint(root)
        if hint:
            print(f"error: {hint}", file=sys.stderr)
        return 3

    # **本 job 的那批**（C4 的分派面 ✓）：`JOB_ALL` = 不分工。
    pool = entries if job == JOB_ALL else [e for e in entries if entry_job(e) == job]

    red: list[str] = []
    if not quiet:
        print(f"咬住证明（台账 `{ledger_path.name}` · 覆盖基线 `{base[:12]}` · "
              f"job `{job}`）：{len(entries)} 条证明 · 本 job {len(pool)} 条 · "
              f"基线以来新增/修改的守卫 {len(scope)} 个")

    # ── C2 空转闸：`--job <名字>` 必须在台账里**真有人声明** ────────────────
    #   ⚠ 没有这一条，`--job 打错的名字` ⇒ **0 条证明、全绿** ⇒ "跑了"其实是空转 ✗
    #   （与 C4 对称：C4 管"声明的 job 有人跑"，这一条管"跑的名字有人声明" ✓）。
    if job != JOB_ALL and not pool:
        red.append(f"**C2 空转** ✗：`--job {job}` 在台账里**没有任何条目**声明它 ⇒ "
                   f"这一跑**什么都没跑**（空转 ✗；台账里声明的 job："
                   f"{' · '.join(sorted({entry_job(e) for e in entries}))}）")

    # ── C1 覆盖 ─────────────────────────────────────────────────────────
    missing = [g for g in scope if g not in by_guard]
    if missing:
        red.append("**C1 覆盖** ✗：基线以来新增/修改的守卫**没有咬住证明** ⇒ 本轮不算收口："
                   + " · ".join(f"`{g}`" for g in missing)
                   + f"（在 `{ledger_path.name}` 补一行：`guard` / `bug`（**已存在的坏输入**）"
                     f"/ `pass` / `bite`（`expect_rc` **非 0** ✓））")
    elif not quiet:
        print(f"  ✓ C1 覆盖：基线以来的 {len(scope)} 个守卫都有证明"
              + (f"（{', '.join(scope)}）" if scope else "（本窗没有新增/修改的守卫）"))

    # ── C4 接线（分派的空转闸 ✓）──────────────────────────────────────────
    c4 = wiring_problems(root, entries)
    red.extend(c4)
    if not quiet and not c4:
        jobs = sorted({entry_job(e) for e in entries})
        print(f"  ✓ C4 接线：台账点名的 {len(jobs)} 个 job 都在 `{CI_YML.name}` 里且**真的跑它**"
              f"（{' · '.join(jobs)}）")

    # ── C13 登记接线（**登记了但没接线 ⇒ 判红** ✓ —— C4 的同族，第二本登记表 ✓）──
    #   ⚠ **不过滤 job**（与 C5/C6/C12 同一条口径 ✓）：接线是**全仓**的事，
    #   不是"哪批证明跑在哪"的事 ⇒ 每个 job 都判它 ✓。
    #   ⚠ **前提不在就不判**（夹具仓的形状，同 C6 的"表不在本仓"那条 ✓）：
    #   `check-budgets.py` 的 `_fixture_repo()` 拷的是**真台账 + 真 `ci.yml`**、
    #   **不拷** `ux-queue.py` ⇒ 硬判会把它判红 ✗（合法夹具假红 ✗ —— 实测踩到过
    #   同形两次，见 C10/C11 的注释 ✓）。
    if not (root / REGISTRY_FILE).is_file():
        if not quiet:
            print(f"  ⓘ C13 登记接线：**本跑不判** —— 登记表 `{REGISTRY_FILE}` 不在本仓"
                  f"（夹具仓 / 拷来的树 ⇒ 表本来就不在那里 ✓）⇒ 如实印出、"
                  f"**既不假红也不假装判过** ✓")
    else:
        c13, c13_reg, c13_err = registered_wiring_problems(root)
        if c13_err:
            print(f"error: {c13_err}", file=sys.stderr)
            return 3
        red.extend(c13)
        if not quiet and not c13:
            if c13_reg:
                print(f"  ✓ C13 登记接线：登记表点名的 {len(c13_reg)} 条都在接线里"
                      f"**真的跑** ✓（{' · '.join(c13_reg)}）")
            else:
                print(f"  ⓘ C13 登记接线：登记表 `{REGISTRY_NAME}` 是**空的** ⇒ 这一条本跑"
                      f"**无输入** —— 如实在此印出，**不**当空转判红 ✓"
                      f"（那会是一条没有出路的红，同 C5/C10 的处置 ✓）")

    # ── C5 可达性（钉的旧 rev 必须**在本仓里**且是 `HEAD` 的祖先 ✓）───────────
    c5, pinned, cands, c5_skip = rev_reachability_problems(root, entries, coverage_since)
    red.extend(c5)
    if not quiet and not c5:
        if c5_skip:
            print(f"  ⓘ C5 可达性：**本跑不判** —— {c5_skip}")
        else:
            print(f"  ✓ C5 可达性：证明里钉的 {len(pinned)} 个 rev 都在本仓里且是 `HEAD` 的祖先 ✓"
                  + (f"（{' · '.join(pinned)}）" if pinned
                     else "（本台账**没有任何** hex token ⇒ 这一条本跑无输入 —— 如实在此印出，"
                          "**不**当空转判红：那会是一条没有出路的红，不是本跑判的 ✓）"))

    # ── C10 夹具前提（钉的状态夹具必须是**合法**状态 ✓）──────────────────────
    #   ⚠ 与 C5 **同一族**：都判"证明的**前提**在不在" ⇒ 不设它，前提坏了会被印成
    #   「这条守卫咬不住」（读数**不可归属** ✗ —— 2026-10-16 实测踩到 ✓）。
    #   ⚠ **不过滤 job**（同 C5）：夹具坏了是**台账**的缺陷，不是"哪批证明跑在哪"的事 ⇒
    #   每个 job 都判它（否则一条声明在别的 job 的证明能把坏夹具藏起来 ✗）。
    c10, fx_judged = state_fixture_problems(root, entries)
    red.extend(c10)
    if not quiet and not c10:
        print(f"  ✓ C10 夹具前提：证明里钉的 {len(fx_judged)} 个状态夹具都是**合法**状态 ✓"
              + (f"（{' · '.join(fx_judged)}）" if fx_judged
                 else "（本台账**没有**钉状态夹具 ⇒ 这一条本跑无输入 —— 如实在此印出，"
                      "**不**当空转判红：那会是一条没有出路的红，不是本跑判的 ✓）"))

    # ── C11 命令前提（证明命令**可执行位**上那个路径必须在树里 ✓）──────────────
    #   ⚠ 与 C5/C10 **同一族**（三档都是"证明的**前提**在不在"）：C5 管**钉的 rev**、
    #   C10 管**钉的状态夹具**、C11 管**命令自己的文件** ⇒ 三档都必须印
    #   「**前提不在**」而**不是**「这条守卫咬不住」✗（**读数可归属** ✓）。
    #   ⚠ **不过滤 job**（同 C5/C10）：命令文件不在是**台账**的缺陷 ✓。
    #   ⚠ **为什么在 C2 之前判**：命令文件都不在 ⇒ 那条证明**跑不出可归属的读数** ⇒
    #   先把它印成"前提不在"，别让 C2 的"咬不住"抢走归因 ✓（判据只加，不放宽 ✗）。
    #   ⚠ **"这是不是台账所属的那个仓"** —— 用 **C5/C6 的同一条口径**（`coverage_since`
    #   能不能在本仓里解析成提交 ✓），**不新造谓词** ✗：夹具仓会把**真台账**拷进去
    #   ⇒ 命令位路径当然不在那里 ⇒ 不设这条，**合法夹具会假红** ✗（实测：
    #   `check-attention-budget.py --selftest` 的 `_fixture_repo()` 当场 4 组红 ✗→✓）。
    own_repo = bool(coverage_since) and _resolves(root, coverage_since)
    c11, c11_judged, c11_skipped = command_premise_problems(root, entries, own_repo=own_repo)
    red.extend(c11)
    if not quiet and not c11:
        if not own_repo:
            print(f"  ⓘ C11 命令前提：**本跑不判** —— 台账的覆盖基线 `{coverage_since[:12]}` "
                  f"**不在本仓** ⇒ 这**不是台账所属的那个仓**（夹具仓 / 拷来的台账 ⇒ 命令位"
                  f"那些路径本来就不在那里 ✓）⇒ 如实印出、**既不假红也不假装判过** ✓"
                  f"（真仓里缺文件那一档**照样**判红 ✓）")
        else:
            print(f"  ✓ C11 命令前提：证明命令**可执行位**上的 {len(c11_judged)} 个路径都在树里 ✓"
                  + (f"（{' · '.join(c11_judged)}）" if c11_judged
                     else "（本台账**没有**命令位路径 ⇒ 这一条本跑无输入 —— 如实在此印出，"
                          "**不**当空转判红 ✓）"))
            if c11_skipped:
                print(f"    ⓘ C11 **不判** {len(c11_skipped)} 条（命令位不是「一个路径」/"
                      f"「解释器 + 一个路径」这两种形状 ⇒ **不猜 shell 语义** ✓ —— "
                      f"这是**如实划的口径**，不是判过 ✗）")

    # ── C6/C7/C8 锚（**每一层都要有一个底** ✓ —— 用户 2026-10-16 09:5x ①）──────────
    #   ⚠ **不过滤 job**：锚是**分类面**的事，不是"哪批证明跑在哪"的事 ⇒ 每个 job 都判它 ✓。
    apath = anchors_path or (root / "docs" / "guard-anchors.jsonl")
    #   ⚠ **"这是不是台账所属的那个仓"** —— 用 **C5 的同一条口径**（`coverage_since` 能不能
    #   在本仓里解析成提交 ✓），**不新造谓词** ✗：`check-attention-budget.py --selftest` 会造一个
    #   **真 git 夹具仓**、把**真台账**拷进去（`_fixture_repo()`，照真仓的形状不打桩 ✓）——
    #   那个仓里当然没有分类登记表 ⇒ 若在这里硬判 rc=3，**夹具仓会把守卫自己判红** ✗
    #   （实测：2026-10-16 加 C6 后 `check-attention-budget.py --selftest` 立刻 4 组红 ✗→✓）。
    #   ⇒ 夹具仓**如实印出不判**（与 C5 的 `ⓘ 本跑不判` 逐字同一条规则 ✓）；**真仓里**
    #   缺表 ⇒ rc=3 ✓（下面那条分支）。⚠ **显式 `--anchors` 一律判**（调用方点名要它 ⇒
    #   没有"跳过"这条路 ⇒ 咬住证明那条 bite 就是在旧树上点名它 ✓）。
    #   ⚠ `own_repo` 在 **C11 之前**就算好了（同一条口径，一处计算 ✓）。
    if not apath.exists() and anchors_path is None and not own_repo:
        if not quiet:
            print(f"  ⓘ C6/C7/C8 **本跑不判** —— 分类登记表 `{apath.name}` 不在本仓，而台账的"
                  f"覆盖基线 `{coverage_since[:12]}` 也**不在本仓** ⇒ 这**不是台账所属的那个仓**"
                  f"（夹具仓 / 拷来的台账 ⇒ 表本来就不在那里 ✓）⇒ 可达性判不了 ⇒ 如实印出、"
                  f"**既不假红也不假装判过** ✓（真仓里缺表那一档由下面那条分支兜住 ⇒ `exit 3` ✓）")
    else:
        arows, aerr = load_anchors(apath)
        if aerr:
            print(f"error: {aerr}", file=sys.stderr)
            return 3
        a_red, a_info, a_counts = anchor_problems(root, arows)
        red.extend(a_red)
        if not quiet:
            if not a_red:
                print(f"  ✓ C6 覆盖：口径里 {a_counts['total']} 个**每一个**都有分类"
                      f"（`guard` A {a_counts['a']} · B {a_counts['b']} · "
                      f"`tool` {a_counts['tools']}）")
                print(f"  ✓ C7 有底：每条都指回一条**用户现场**锚（{a_counts['anchors']} 条锚）"
                      f" —— B 只许指向锚 ⇒ **B 不能指向 B**，递归到底 ✓")
                print(f"  ✓ C8 预算：B **{a_counts['b']} ≤ {a_counts['cap']}**（每轮 −2，目标 "
                      f"{a_counts['floor']}）· ⭐ A **{a_counts['a']} 条不进任何上限**（锚定型无限预算 ✓）")
            for m in a_info:
                print(f"  {m}")

        # ── **C12 双面**（本地自动面 ⇄ CI 面；第 4 轮 `ng4` ✓）───────────────────
        #   ⚠ **不过滤 job**（与 C6/C7/C8 同一条口径 ✓）：两面是**接线**的事，不是
        #   "哪批证明跑在哪"的事 ⇒ 每个 job 都判它 ✓。
        f_red, f_info, f_counts = face_problems(root, arows)
        red.extend(f_red)
        if not quiet:
            if not f_red:
                print(f"  ✓ C12 双面：两面都有 **{f_counts['both']}** 条 · **CI 面缺** "
                      f"{f_counts['ci_missing']}（witness {f_counts['ci_exempt']} · "
                      f"**真洞 {len(f_counts['ci_open'])}**）· **本地面缺** "
                      f"{f_counts['local_missing']}（witness {f_counts['local_exempt']} · "
                      f"**真洞 {len(f_counts['local_open'])}**）· 两面都缺 {f_counts['neither']}")
                print(f"    ⓘ C12 豁免分档（**数据只能选 witness、不能定义它** ✓ —— 一档一份实现"
                      f"就在本文件里）：" + " · ".join(
                          f"{k} ×{v}" for k, v in sorted(f_counts["witnesses"].items())))
            for m in f_info:
                print(f"  {m}")

    # ── C2 咬得住 ───────────────────────────────────────────────────────
    targets = pool if run_all else [e for e in pool if e["guard"] in scope]
    if run_all:
        # `--all` 另加一条：台账里点名的守卫**必须真的存在**（防表自己腐烂 ✗）
        for e in entries:
            if not (root / e["guard"]).exists():
                red.append(f"**C2 台账腐烂** ✗：`{e['guard']}` 在台账里、但文件不存在 ✗")
    if not quiet:
        print(f"  C2 逐条真跑（{len(targets)} 条；`bite` 必须**非 0** ⇒ 它真的咬住了那条坏输入 ✓）")
    empty: list[dict] = []
    for e in targets:
        g = e["guard"]
        if not (root / g).exists():
            red.append(f"**C2** ✗：`{g}` 不在仓库里（台账点名了一个不存在的守卫）✗")
            continue
        prc, ptail = run_proof(e["pass"]["cmd"], root)
        brc, btail = run_proof(e["bite"]["cmd"], root)
        p_ok = prc == e["pass"]["expect_rc"]
        b_ok = brc == e["bite"]["expect_rc"] and e["bite"]["expect_rc"] != 0
        if not quiet:
            print(f"    {'✓' if p_ok else '✗'} {g} `pass` rc={prc}"
                  f"（期望 {e['pass']['expect_rc']}）")
            print(f"    {'✓' if b_ok else '✗'} {g} `bite` rc={brc}"
                  f"（期望 {e['bite']['expect_rc']}，**必须非 0**）")
            # ⚠ ⭐ **读数可归属**（2026-10-16 实测的误读 ✗→✓，洞
            #   `BITE-EXPECTATION-GOES-STALE-WHEN-READING-IMPROVES` 的**近因**）：
            #   证明**自身的输出**必须**单独一行**印（旧版拼在 `✓` 行尾巴上 ✗）——
            #   它里面**故意**可能带着 `C2 ✗`：本文件自己的 `bite` 就是拿
            #   **故意咬不住**的夹具（`scripts/fixtures/guard-bite-nonbiting.jsonl`）跑的 ✓，
            #   而那份夹具点名的守卫（2026-10-17 起 = `scripts/check-budgets.py` ✓）⇒ 旧输出里出现
            #   `✓ … ⇒ · **C2** ✗：`scripts/check-budgets.py` 的 `bite` rc=0（期望 1）`
            #   ⇒ 人和 agent 都把它读成**本跑判红**（实测：一条 high 队列条目 + 一次
            #   「`--check --all` rc=1」的结论就是这么来的 ✗ —— 那一跑的 rc 其实是 **0**）。
            #   ⇒ 判据行**只**放本跑的判据 ✓；证明自述另起一行、带 `↳` 与「不是本跑的判据」✓。
            #   ⚠ **只改排印，不改判据** ✗：rc / C1–C5 一个字没动 ✓（自检里有一条臂钉住它 ✓）。
            if b_ok and btail:
                print(f"      ↳ 证明自身的输出（**不是本跑的判据** ✓）：{btail[:110]}")
        if e["bite"]["expect_rc"] == 0:
            red.append(f"**C2** ✗：`{g}` 的 `bite.expect_rc = 0` ⇒ **它咬不住** "
                       f"（咬住证明必须是「坏输入 ⇒ **非 0**」✗→✓）")
        elif not p_ok and prc in NOT_EXECUTED_RC and e["pass"]["expect_rc"] not in NOT_EXECUTED_RC:
            # ⚠ ⭐ **C11 归属面**（与 C5/C10 同一条纪律：**读数必须可归属** ✗→✓）：
            #   `rc=127` 是 shell 的 `command not found`、`126` 是不可执行 ⇒ 那条命令
            #   **根本没跑起来** ⇒ 把它印成「守卫自己坏了」是**误归因** ✗。
            red.append(f"**C11 命令前提** ✗：`{g}` 的 `pass` rc={prc}（期望 "
                       f"{e['pass']['expect_rc']}）—— **{_not_executed_reason(prc)}** ⇒ 那条命令"
                       f"**根本没跑起来** ⇒ 这**不是**「守卫自己坏了」，是**前提不在** ✗"
                       f"（⚠ **别**把 `expect_rc` 改成 {prc} ✗ —— 那是把「没跑」说成「跑过了」✗）："
                       f"{ptail[:160]}")
        elif not p_ok:
            red.append(f"**C2** ✗：`{g}` 的 `pass` rc={prc}（期望 {e['pass']['expect_rc']}）"
                       f" ⇒ 守卫自己坏了：{ptail[:160]}")
        elif not b_ok and brc in NOT_EXECUTED_RC and e["bite"]["expect_rc"] not in NOT_EXECUTED_RC:
            # ⚠ ⭐ **C11 归属面**（**CI 实测** ✗→✓，洞 `BITE-RC-127-COMMAND-NOT-FOUND`）：
            #   修前这里印的是「**这条守卫咬不住它声称咬住的那条坏输入**」—— 而真相是
            #   **命令根本没跑起来**（`11bc8e3f` 的树里那个脚本**还没提交** ✗）⇒
            #   读日志的人会去查"守卫为什么咬不住"，而**该查的是前提** ✗（误归因 ✗）。
            red.append(f"**C11 命令前提** ✗：`{g}` 的 `bite` rc={brc}（期望 "
                       f"{e['bite']['expect_rc']}）—— **{_not_executed_reason(brc)}** ⇒ 那条命令"
                       f"**根本没跑起来** ⇒ 这**不是**「这条守卫咬不住它声称咬住的那条坏输入」"
                       f"（`{e.get('bug', '')[:80]}`），是**前提不在** ✗ ⇒ 先修前提"
                       f"（把命令位那个文件**跟台账行一起提交** ✓ / 换成两边都有的命令 ✓），"
                       f"再谈咬不咬得住 ✓（⚠ **别**把 `expect_rc` 改成 {brc} ✗，"
                       f"**也别**把它标成 skip / 删掉那一行 ✗）：{btail[:160]}")
            empty.append(e)
        elif not b_ok:
            # ⚠ **缺前提时给可操作的人话**（2026-10-16 CI 实测 ✗→✓，与 `shallow_hint` 同一条纪律
            #   ✓）：证明钉的旧 rev 在 CI 的树里**不存在**时，`bite` 的空转闸会让它 `exit 0`
            #   ⇒ 这里印的是「这条守卫咬不住」—— 而**真相是前提不在** ✗ ⇒ 指名那个 rev，
            #   把「咬不住」翻译成「钉的 rev 不在本仓里」✓。⚠ **只改话术，不改判据** ✗：
            #   退出码与 C1/C2/C5 一个字没动 ✓（C5 在**本机**就会先判红 ✓）。
            missing = unresolvable_revs(root, str(e["bite"].get("cmd", "")))
            hint = ("  ⚠ 这条命令钉的 rev " + " · ".join(f"`{m}`" for m in missing)
                    + " 在**本仓里不存在** ⇒ CI 只检出**一个 ref** 的历史"
                      "（`fetch-depth: 0` 给的是**那份**历史的全部，**不是**别的分支的对象 ✗）"
                      "⇒ 钉的必须是 `HEAD` 的祖先 ✓（`git merge-base --is-ancestor <rev> HEAD`）")
            red.append(f"**C2** ✗：`{g}` 的 `bite` rc={brc}（期望 {e['bite']['expect_rc']}）"
                       f" ⇒ **这条守卫咬不住它声称咬住的那条坏输入**"
                       f"（`{e.get('bug', '')[:80]}`）：{btail[:160]}"
                       + (hint if missing else ""))
            empty.append(e)

    # ── C3 翻账（用户【①】）─────────────────────────────────────────────
    if reopen_empty and empty:
        print(f"\n⭐ **翻账**（用户 2026-10-15【①】：假绿要 **reopen 它关掉的那条**，"
              f"不只是记新账 ✓）：{len(empty)} 条守卫空了", file=sys.stderr)
        for e in empty:
            qid = e.get("queue_id")
            if not qid:
                print(f"  · `{e['guard']}` 的台账行**没有 `queue_id`** ⇒ 不知道该 reopen 谁 ✗ "
                      f"（**如实说**：这条只能靠外部验收方手工翻账）", file=sys.stderr)
                continue
            cmd = [sys.executable, str(root / "scripts" / "ux-queue.py"),
                   "--advance", qid, "--to", "todo", "--reason",
                   f"守卫 {e['guard']} 的咬住证明已失效（假绿）⇒ 翻账（check-guard-bite --reopen-empty）"]
            out = subprocess.run(cmd, cwd=root, capture_output=True, text=True,
                                 encoding="utf-8", errors="replace")
            if out.returncode == 0:
                print(f"  ✓ **reopen `{qid}`**（{e['guard']}）⇒ 回 `todo` ✓", file=sys.stderr)
            else:
                # ⚠ **实测的可行性缺口**（2026-10-16，`file:line` = `ux-queue.py:2768-2787`）：
                #   来源处置**已关账**（extras 的 `disposition=fixed`）时，`--advance --to todo`
                #   **故意**拒（否则造出「来源关账 + 调度器 todo」= 反向两层漂移 ✗）。
                #   ⇒ 用户 2026-10-15【①】假设的机制（"队列已有 `--advance … --to todo --reason`"）
                #   **只在来源没关账时成立** ✗ —— 而 extras 里 201/208 条都是 `fixed` ✗。
                #   ⇒ 翻账要**两步**，且第一步是 `--triage`（**接口 agent 契约里没有** ✗）。
                #   本文件**不绕过**它（写权限是队列自己的口径 ✓）—— **如实印出可复制的两步** ✓。
                reason = (f"守卫 {e['guard']} 的咬住证明已失效（假绿）⇒ 翻账："
                          f"{str(e.get('bug', ''))[:80]}")
                print(f"  ✗ `{qid}` reopen 没成（rc={out.returncode}）—— "
                      f"**来源已关账时 `--advance` 回不去**（`ux-queue.py:2768` 的口径 ✓），"
                      f"可复制的两步（**第一步是 `--triage`** ⇒ 接口 agent 契约里没有 ✗，"
                      f"只有主进程能做）：\n"
                      f"      ① python3 scripts/ux-queue.py --triage {qid} --to deferred "
                      f"--reason '{reason}'\n"
                      f"      ② python3 scripts/ux-queue.py --advance {qid} --to todo "
                      f"--reason '{reason}'\n"
                      f"    （**假绿本身另记一条**——用户【①】的后半句：\n"
                      f"      `python3 scripts/ux-queue.py --schedule {qid}-FALSE-GREEN "
                      f"--facet 主线 --severity high --repro '（无复现件：判据见 where）' "
                      f"--why '<守卫 {e['guard']} 的咬住证明失效 ⇒ 那条 done 是假绿>' "
                      f"--where '{e['guard']} 的 bite 命令'`）\n"
                      f"    {out.stderr.strip()[:160]}", file=sys.stderr)

    if red:
        print(f"\n✗ 咬住证明 **{len(red)} 条红**：", file=sys.stderr)
        for m in red:
            print(f"  · {m}", file=sys.stderr)
        return 1
    if not quiet:
        print("✓ 咬住证明：覆盖齐 + 每条都**真的咬住了**一条已存在的坏输入 ✓")
    return 0


def list_coverage(root: Path, ledger_path: Path, anchors_path: Path | None = None) -> int:
    """`--list`：谁有证明 · 谁是 grandfathered（**评审可见** ✓，不是豁免 ✗）· A/B 分类。"""
    entries, coverage_since, err = load_ledger(ledger_path)
    if err:
        print(f"error: {err}", file=sys.stderr)
        return 3
    covered = {e["guard"] for e in entries}
    allg = guard_scripts(root)
    print(f"台账 `{ledger_path}`：{len(entries)} 条证明 · 覆盖基线 `{coverage_since[:12]}`")
    for e in entries:
        print(f"  ✓ {e['guard']}  [job {entry_job(e)}]\n"
              f"      坏输入：{e.get('bug', '')[:110]}\n"
              f"      bite：`{e['bite']['cmd'][:110]}` ⇒ 期望 rc={e['bite']['expect_rc']}")
    for j in sorted({entry_job(e) for e in entries}):
        n = sum(1 for e in entries if entry_job(e) == j)
        print(f"  · job `{j}`：{n} 条" + ("（默认，`--all` 就跑这批 ✓）"
                                          if j == DEFAULT_JOB else "（`--job %s` ✓）" % j))
    rest = [g for g in allg if g not in covered]
    # ⭐ **枚举面**（洞 `GUARD-ENUMERATION-IS-NAME-BASED`）：两条臂各收了谁 ⇒ 评审可见 ✓
    wired = wired_scripts(root)
    named = [g for g in allg if any(fnmatch.fnmatch(g, p) for p in GUARD_GLOBS)]
    print(f"\n口径 = **两条臂的并集**（{len(allg)} 个）：名字臂 {len(named)} · "
          f"**接线臂 {len(wired)}**（`ci-local.sh` / `ci.yml` 的 step 真调用 ✓ —— "
          f"**改名绕不过去** ✓）")
    for g in wired:
        print(f"    · {g}" + ("" if g in named else "  ← **只靠接线臂进来的** ✓"))
    print(f"\n⚠ **grandfathered**（基线之前就存在、**不在强制面内** ✗ —— "
          f"如实划的口径，不是豁免 ✓）：{len(rest)} 个")
    for g in rest:
        print(f"    · {g}")

    # ── A/B 分类（C6/C7/C8 的**可读面** ✓）：评审一眼看得出"谁在量用户现场"──
    arows, aerr = load_anchors(anchors_path or (root / "docs" / "guard-anchors.jsonl"))
    if aerr:
        print(f"\n⚠ 分类登记表读不了 ⇒ C6/C7/C8 判不了：{aerr}")
        return 0
    _a_red, a_info, c = anchor_problems(root, arows)
    by = {r["guard"]: r for r in arows if r.get("kind") == "guard"}
    print(f"\nA/B 分类（`docs/guard-anchors.jsonl`）：**A {c['a']}** 条（量用户现场）· "
          f"**B {c['b']}** 条（量别的判据）· 上限 **B ≤ {c['cap']}**（每轮 −2，目标 {c['floor']}）")
    for cls, names in (("A", c["a_guards"]), ("B", c["b_guards"])):
        print(f"  【{cls}】{'量用户现场 ✓' if cls == 'A' else '量别的判据（必须指回 A）'}")
        for g in names:
            r = by.get(g) or {}
            print(f"    {g} → {' · '.join(r.get('anchors') or []) or '（**没底** ✗）'}")
    trows = {r["tool"]: r for r in arows if r.get("kind") == "tool"}
    if trows:
        print(f"  【tool】接线臂里的**工具 / 渲染器**（不是独立判据 ⇒ 记 `tool` ✓，"
              f"C9：`why` 必填 · `covered_by` 悬空判红 ✗）")
        for t in sorted(trows):
            cb = str(trows[t].get("covered_by") or "").strip()
            print(f"    {t} → 独立覆盖：{cb or '（**没有** ⇒ ⓘ 缺口，不是红 ✓）'}")
    for m in a_info:
        print(f"  {m}")
    return 0


# ── 判据通道自检（反向验证：故意造出该现象 ⇒ 必须判红）──────────────────────
def selftest(job: str = DEFAULT_JOB) -> int:
    bad = 0
    # ⚠ ⭐ **两向的组数是算出来的、不是手写的常量** ✗→✓（2026-10-16 实测：那行
    #   `正对照 11 + 反向 17` 是**手维护**的 ⇒ 加了臂之后它**当场就旧了** ✗ ——
    #   而它是**给人读的读数**、会被别的条目**引用**（CI 证据 / 队列条目都引过 ✓）⇒
    #   旧数字会把读的人带偏 ✗。**在打印机上兜**：按臂名归类、末尾照实印 ✓。
    tally: dict[str, int] = {}

    def gate(name: str, ok: bool, note: str = "") -> None:
        nonlocal bad
        kind = "正对照" if "正对照" in name else ("反向" if "反向" in name else "其它")
        tally[kind] = tally.get(kind, 0) + 1
        # ⚠ ⭐ **读数可归属**（与 `check()` 里那条**同一条规则**，这里兜住整个 `--selftest`
        #   流 ✓）：`✓` 判据行的正文里**不许**出现判红标记 —— 证明**自身的输出**可能
        #   **故意**是红的（本文件自己的 `bite` 跑的就是**故意咬不住**的夹具 ✓）⇒ 它必须
        #   另起一行印（`↳ …`）✓。**在打印机上**兜 ⇒ 连臂名/注脚一起管住 ✓（同类一次改齐 ✓）。
        mixed = ok and ("✗" in name or "✗" in note)
        if not ok or mixed:
            bad += 1
        print(f"  {'✓' if ok else '✗'} {name}{('：' + note) if note else ''}")
        if mixed:
            print("      ↳ **这条 `✓` 判据行的正文里带了判红标记** ⇒ 读数**不可归属** "
                  "（证明自述必须单独一行 ✓）")

    print(f"咬住证明自检（**真跑**，每条都带正反两向 ✓ · job `{job}`）：")

    # ── 正对照：**真台账**里**本 job 的**每条逐条跑 ⇒ 必须 rc=0，且 bite 必须真的非 0 ──
    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        rc = check(ROOT, LEDGER, None, run_all=True, quiet=False, job=job)
    out = buf.getvalue()
    print(out, end="")                       # 判据输出照旧**看得见** ✓（只是被抄了一份 ✓）
    gate(f"正对照：**真台账**（`--all` 逐条真跑 · job `{job}`）⇒ rc=0", rc == 0, f"rc={rc}")
    # ⚠ ⭐ **读数可归属**（2026-10-16 实测的误读 ✗→✓）：`✓` 判据行的正文里**不许**出现
    #   判红标记 —— 证明**自身的输出**（里面**故意**可能有 `C2 ✗`：本文件自己的 `bite`
    #   跑的就是**故意咬不住**的夹具 ✓）必须**单独一行**印（`↳ …`）✓。
    #   ⚠ 这条臂**不是**装饰：它钉住"`✓` 行与证明自述**不混排**"这一条，
    #   而那个混排**真实发生过**（一条 high 队列条目把它读成「`--check --all` rc=1」✗，
    #   而那一跑的 rc 是 0 ✓）⇒ 咬住证明：`docs/guard-bite.jsonl` 里本文件那一行
    #   （`bite` 拿**修前**的 `b21b71c1` 跑同一条 `--check --all` ⇒ 必须复现出混排 ⇒ rc=1 ✓）。
    traps = [ln for ln in out.splitlines() if re.match(r"^\s*✓ ", ln) and "✗" in ln]
    gate("正对照：**读数可归属** —— 判据行里不夹带**另一条判据的判红标记**（证明自述单独一行 ✓）",
         not traps, f"混排行 {len(traps)}" + (f" · 例：{traps[0].strip()[:70]}" if traps else ""))
    entries, _cs, err = load_ledger(LEDGER)
    gate("正对照：真台账读得进来且非空", not err and bool(entries),
         f"{len(entries)} 条" + (f" · {err}" if err else ""))
    gate("正对照：每条 `bite.expect_rc` 都是**非 0**（咬住 = 坏输入 ⇒ 红 ✓）",
         all(e["bite"]["expect_rc"] != 0 for e in entries),
         " · ".join(f"{Path(e['guard']).name}→{e['bite']['expect_rc']}" for e in entries))
    gate("正对照：每条都有 `bug`（**已存在的坏输入**是必填 ✓）",
         all(str(e.get("bug", "")).strip() for e in entries))

    # ── 锚登记表（C6/C7/C8）—— **两头都要验** ✓（B 超上限 ⇒ 判红 · A 增长 ⇒ **不**判红）──
    real_rows, real_err = load_anchors(ANCHORS)
    gate("正对照：真分类登记表读得进来（`docs/guard-anchors.jsonl`）",
         not real_err and bool(real_rows), f"{len(real_rows)} 行" + (f" · {real_err}" if real_err else ""))
    with tempfile.TemporaryDirectory(prefix="soko-anchor-") as td_a:
        ta = Path(td_a)
        a_red, _a_info, ac = anchor_problems(ROOT, real_rows)
        gate("正对照：真登记表 ⇒ C6 覆盖 / C7 有底 / C8 预算**无红**",
             not a_red, f"A {ac['a']} · B {ac['b']} · cap {ac['cap']} · 锚 {ac['anchors']}"
                        + (f" · 红：{a_red[:1]}" if a_red else ""))

        # 反向⑪：**B 超上限** ⇒ 必须判红（预算真的在拦，不是装饰 ✗）
        over = [dict(r) for r in real_rows]
        for r in over:
            if r.get("kind") == "caps":
                r["caps"] = {"b_class_guards": ac["b"] - 1}
                r["floor"] = {"b_class_guards": min(ac["b"] - 1, r["floor"]["b_class_guards"])}
                r["round"] = 0
                r["history"] = [{"round": 0, "caps": dict(r["caps"]),
                                 "reading": dict(r["caps"]), "why": "夹具：故意把上限压到 B−1"}]
        red_over, _i, _c = anchor_problems(ROOT, over)
        gate("反向⑪：**B 超上限**（cap = B−1）⇒ 判红（C8 超预算 + 给出路「新加一条 B 必须删/合并一条」）",
             any("C8 超预算" in m for m in red_over), f"红 {len(red_over)} 条")

        # 反向⑦：**B 指一条不存在的锚** ⇒ 必须判红（底悬空 ⇒ 递归没到底 ✗）
        dang = [dict(r) for r in real_rows]
        for r in dang:
            if r.get("kind") == "guard" and r.get("class") == "B":
                r["anchors"] = ["A-NO-SUCH-ANCHOR"]
                break
        red_dang, _i, _c = anchor_problems(ROOT, dang)
        gate("反向⑫：**B 指向一条不存在的锚** ⇒ 判红（C7 悬空锚 ⇒ 它的底悬空）",
             any("C7 悬空锚" in m for m in red_dang), f"红 {len(red_dang)} 条")

        # 反向⑧：**口径里有守卫没分类** ⇒ 必须判红（C6 覆盖 —— 修前那份仓库状态就是这个形状 ✓）
        red_miss, _i, _c = anchor_problems(ROOT, [r for r in real_rows if r.get("kind") != "guard"])
        gate("反向⑬：**有守卫没分类**（C6）⇒ 判红（新守卫没分类不许进仓库）",
             any("C6 覆盖" in m for m in red_miss), f"红 {len(red_miss)} 条")

        # ⭐⭐ **接线臂**（洞 `GUARD-ENUMERATION-IS-NAME-BASED`，2026-10-16 ✗→✓）：
        #   判据 = **位置口径**（gate / CI 的 step 真调用）⇒ **改个名仍被 C1/C6 看到** ✓。
        #   ⚠ 这一组臂**必须**打在"名字臂**抓不到**"的那个文件上（否则它证明不了新那一半 ✗）。
        def _wire_root() -> Path:
            """夹具：一个**改了名**的守卫（`scripts/renamed-guard.py`，**不在** glob 里）被
            `scripts/ci-local.sh` 的 step **真调用**；外加两个**幽灵名字**（接线里有、文件
            不在 / 只在注释里）⇒ **不许**算进口径 ✗。"""
            r = ta / "wire"
            (r / "scripts").mkdir(parents=True, exist_ok=True)
            (r / "scripts" / "renamed-guard.py").write_text("# 夹具：改了名的守卫\n", encoding="utf-8")
            (r / "scripts" / "check-fixture-a0.py").write_text("# 夹具：名字臂守卫\n", encoding="utf-8")
            (r / "scripts" / "ci-local.sh").write_text(
                'run "夹具" "gates" python3 scripts/renamed-guard.py\n'
                '# 注释里点名**不算调用** ✗：python3 scripts/ghost-in-comment.py\n'
                'run "夹具2" "gates" python3 scripts/ghost-missing.py\n', encoding="utf-8")
            return r

        wroot = _wire_root()
        wscope = guard_scripts(wroot)
        gate("⭐⭐ 正对照：**接线臂**（位置口径）—— 一个**改了名**的守卫"
             "（`scripts/renamed-guard.py`：**名字臂抓不到** ✓）被 gate 真调用 ⇒ **在口径里** ✓",
             "scripts/renamed-guard.py" in wscope
             and not is_guard("scripts/renamed-guard.py", set()),
             f"口径 {len(wscope)} 个：{' · '.join(wscope)}")
        gate("⭐⭐ 正对照：接线里的**幽灵名字不进**口径（文件不在 / 只在注释里）",
             "scripts/ghost-missing.py" not in wscope and "scripts/ghost-in-comment.py" not in wscope,
             f"口径 {len(wscope)} 个")

        def _wire_rows(tool_row: dict | None) -> list[dict]:
            rows = [{"kind": "caps", "round": 0, "caps": {"b_class_guards": 0},
                     "floor": {"b_class_guards": 0},
                     "history": [{"round": 0, "caps": {"b_class_guards": 0},
                                  "reading": {"b_class_guards": 0}, "why": "夹具：B 预算 0"}]},
                    {"kind": "anchor", "id": "A-FIXTURE", "strength": "direct",
                     "site": "夹具：真 VS Code", "action": "打开文件", "reading": "夹具读数"},
                    {"kind": "guard", "guard": "scripts/check-fixture-a0.py", "class": "A",
                     "anchors": ["A-FIXTURE"], "why": "夹具：名字臂守卫"}]
            return rows + ([tool_row] if tool_row else [])

        # ⚠ **可归属**：名字臂那个文件**有**分类行 ⇒ 唯一的红只可能来自**改了名的那一个** ✓
        #   ⚠ 判据行（`gate()` 的 name/note）里**不许**出现判红标记 ✗（本文件自己的纪律，
        #   实测踩到过：第一版把判红原文塞进 note ⇒ `--selftest` 当场计 4 组不对 ✓）
        #   ⇒ 判红原文另起一行印（`↳`，与证明自述同一条规则 ✓）。
        red_wire, _i, _c = anchor_problems(wroot, _wire_rows(None))
        gate("⭐⭐ 反向⑯：**改了名的守卫没有分类** ⇒ 判红（C6 覆盖 —— 修前它对名字臂**隐身**）",
             any("C6 覆盖" in m and "renamed-guard" in m for m in red_wire),
             f"红 {len(red_wire)} 条")
        if red_wire:
            print(f"      ↳ 本臂判红原文（**不是本跑的判据** ✓）：{red_wire[0][:90]}")
        red_ok, _i, _c = anchor_problems(wroot, _wire_rows(
            {"kind": "tool", "tool": "scripts/renamed-guard.py",
             "why": "夹具：它是工具、不是独立判据", "covered_by": "scripts/check-fixture-a0.py"}))
        gate("⭐⭐ 正对照：同一个夹具登记成 `tool`（+ 独立覆盖）⇒ **不判红** ✓（C9 的正当用法）",
             not red_ok, f"红 {len(red_ok)} 条")
        red_dang2, _i, _c = anchor_problems(wroot, _wire_rows(
            {"kind": "tool", "tool": "scripts/renamed-guard.py",
             "why": "夹具：故意指一条不存在的独立覆盖", "covered_by": "scripts/no-such-guard.py"}))
        gate("⭐⭐ 反向⑰：`tool` 的 `covered_by` **悬空** ⇒ 判红（C9 悬空覆盖 ⇒ 这条 tool 行没底）",
             any("C9 悬空覆盖" in m for m in red_dang2), f"红 {len(red_dang2)} 条")
        red_cl, info_cl, c_cl = anchor_problems(wroot, _wire_rows(
            {"kind": "tool", "tool": "scripts/renamed-guard.py",
             "why": "夹具：没有独立覆盖（缺口）"}))
        gate("⭐⭐ 正对照：`tool` **没有独立覆盖** ⇒ 不判红、但**进 `ⓘ` 且 `--counts` 有计数** ✓"
             "（缺口**可见**，不是静默放行）",
             (not red_cl) and any("没有独立覆盖的工具" in m for m in info_cl)
             and c_cl["coverless_tools"] == ["scripts/renamed-guard.py"],
             f"红 {len(red_cl)} · info {len(info_cl)} · coverless {c_cl['coverless_tools']}")

        # ⭐⭐ **C12 双面**（第 4 轮 `ng4`）：**只删 CI 面、本地还在** ⇒ 必须判红 ✓
        #   （修前这条形状**枚举不变** ⇒ C6 照样绿 ✗ —— 本轮把它变成判红 ✓）。
        def _face_root(tag: str, ci_line: str | None, local_kind: str = "run") -> Path:
            """夹具：一个守卫**本地面有**（`ci-local.sh` 的 `run`/`soft` 步 ✓）+ CI 面按参数 ✓。

            ⚠ `tag` **必须每个臂都不一样**（**实测踩到过** ✗：同名目录会让上一条臂留下的
            `githooks/` 影响下一条 ⇒ 反向㉓ 拿到的是"CI 面缺"而不是"两面都缺" ✓）。
            ⚠ 夹具仓里**没有** `docs/guard-anchors.jsonl` ⇒ 真跑时 C12 会被 C6 那条
            "不是台账所属的仓 ⇒ 本跑不判"兜住 ✓；这里**直接调** `face_problems()` ✓。
            """
            r = ta / f"faces-{tag}"
            (r / "scripts").mkdir(parents=True, exist_ok=True)
            (r / "scripts" / "check-fixture-face.py").write_text(
                "# 夹具守卫（不吃 SOKONANODA_BIN ✓）\n", encoding="utf-8")
            (r / "scripts" / "ci-local.sh").write_text(
                f'{local_kind} "夹具" python3 scripts/check-fixture-face.py\n', encoding="utf-8")
            wf = r / ".github" / "workflows"
            wf.mkdir(parents=True, exist_ok=True)
            step = f"      - run: {ci_line}\n" if ci_line else "      - run: echo hi\n"
            (wf / "ci.yml").write_text(f"jobs:\n  lint:\n    steps:\n{step}", encoding="utf-8")
            return r

        def _face_row(extra: dict | None = None) -> list[dict]:
            row = {"kind": "guard", "guard": "scripts/check-fixture-face.py", "class": "A",
                   "anchors": ["A-FIXTURE"], "why": "夹具：双面守卫"}
            if extra:
                row.update(extra)
            return [row]

        f_both = _face_root("both", "python3 scripts/check-fixture-face.py")
        f_red, _fi, fc = face_problems(f_both, _face_row())
        gate("⭐⭐ 正对照：夹具守卫**两面都有** ⇒ C12 无红", not f_red,
             f"两面都有 {fc.get('both')} · CI 面缺 {fc.get('ci_missing')}")
        # ⭐⭐ **本轮的反向验证**：只删 CI 面（本地的 `run` 步还在）⇒ 必须判红并点名它 ✓
        f_noci = _face_root("noci", None)
        f_red2, _fi2, _fc2 = face_problems(f_noci, _face_row())
        gate("⭐⭐ 反向⑳：**只删 CI 面、本地还在** ⇒ 判红（C12 —— 修前这条形状枚举不变、C6 绿）",
             any("C12 CI 面缺" in m and "check-fixture-face" in m for m in f_red2),
             f"红 {len(f_red2)} 条")
        if f_red2:
            print(f"      ↳ 本臂判红原文（**不是本跑的判据** ✓）：{f_red2[0][:90]}")
        # ⭐⭐ **第 5 轮的反向验证**：只删**本地面**（CI 面还在）⇒ 必须判红 ✓
        #   （第 4 轮这条形状**只印 `ⓘ`** —— 那时的前提是"接上去当场会红"；那条前提已随
        #   `ng5` 的发布闭环消失 ⇒ 第 5 轮收紧成判红 ✓）
        f_nolocal = _face_root("nolocal", "python3 scripts/check-fixture-face.py")
        (f_nolocal / "scripts" / "ci-local.sh").write_text(
            "# 夹具：本地面没有调用\n", encoding="utf-8")
        f_red9, _fi9, _fc9 = face_problems(f_nolocal, _face_row())
        gate("⭐⭐ 反向㉔：**只删本地面、CI 还在** ⇒ 判红（第 5 轮收紧 —— 修前这条只印 ⓘ）",
             any("C12 本地面缺" in m and "check-fixture-face" in m for m in f_red9),
             f"红 {len(f_red9)} 条")
        if f_red9:
            print(f"      ↳ 本臂判红原文（**不是本跑的判据** ✓）：{f_red9[0][:90]}")
        # 正对照：**豁免成立**（`masked-readout` + 本地确实是 `soft` 步）⇒ 不判红 ✓
        f_soft = _face_root("soft", None, local_kind="soft")
        f_red3, _fi3, fc3 = face_problems(f_soft, _face_row(
            {"faces": {"ci": {"witness": "masked-readout"}}}))
        gate("⭐⭐ 正对照：**豁免成立**（`masked-readout` + 本地确实是 `soft` 步）⇒ 不判红",
             not f_red3, f"CI 面缺 {fc3.get('ci_missing')} · 豁免 {fc3.get('ci_exempt')}")
        # 正对照：`pre-push-object`（钩子面 + 那一刻的实参）真的成立 ⇒ 不判红 ✓
        f_hook = _face_root("hook", None)
        (f_hook / "scripts" / "githooks").mkdir(parents=True, exist_ok=True)
        (f_hook / "scripts" / "githooks" / "commit-msg").write_text(
            'python3 scripts/check-fixture-face.py --check --message-file "$1"\n', encoding="utf-8")
        f_red4, _fi4, fc4 = face_problems(f_hook, _face_row(
            {"faces": {"ci": {"witness": "pre-push-object"}}}))
        gate('⭐⭐ 正对照：`pre-push-object`（钩子面 + `"$1"` 那一刻的实参）⇒ 不判红',
             not f_red4, f"CI 面缺 {fc4.get('ci_missing')} · 豁免 {fc4.get('ci_exempt')}")
        # 正对照：`tool`（`kind: tool` 那一行）⇒ 豁免成立 ✓
        f_red5, _fi5, fc5 = face_problems(f_noci, [
            {"kind": "tool", "tool": "scripts/check-fixture-face.py",
             "why": "夹具：工具、不是独立判据", "faces": {"ci": {"witness": "tool"}}}])
        gate("⭐⭐ 正对照：`tool`（登记表那一行本来就是 `kind: tool`）⇒ 豁免成立、不判红",
             not f_red5, f"CI 面缺 {fc5.get('ci_missing')} · 豁免 {fc5.get('ci_exempt')}")
        # 反向：**豁免腐烂**（声明那一面不存在、其实有）⇒ 判红 ✓
        f_red6, _fi6, _fc6 = face_problems(f_both, _face_row(
            {"faces": {"ci": {"witness": "masked-readout"}}}))
        gate("⭐⭐ 反向㉑：**豁免腐烂**（声明 CI 面不存在、其实两面都有）⇒ 判红",
             any("C12 豁免腐烂" in m for m in f_red6), f"红 {len(f_red6)} 条")
        # 反向：**witness 不成立**（声明 `binary-precondition`，夹具却**不吃** `SOKONANODA_BIN`）⇒ 判红 ✓
        f_red7, _fi7, _fc7 = face_problems(f_noci, _face_row(
            {"faces": {"ci": {"witness": "binary-precondition"}}}))
        gate("⭐⭐ 反向㉒：witness **不成立**（夹具不吃 `SOKONANODA_BIN`）⇒ 判红"
             "（**数据只能选、不能定义**）",
             any("C12 witness 不成立" in m for m in f_red7), f"红 {len(f_red7)} 条")
        # 反向：**两面都缺**（连本地调用也删掉）⇒ 判红（谁都不跑 ⇒ 同族病）✓
        f_nowhere = _face_root("nowhere", None)
        (f_nowhere / "scripts" / "ci-local.sh").write_text("# 夹具：没有调用\n", encoding="utf-8")
        f_red8, _fi8, _fc8 = face_problems(f_nowhere, _face_row())
        gate("⭐⭐ 反向㉓：**两面都缺**（本地调用也删掉）⇒ 判红（谁都不跑 ⇒ 同族病）",
             any("C12 两面都缺" in m for m in f_red8), f"红 {len(f_red8)} 条")

        # ⭐⭐ 反向⑨ = **两头的那一头**：**A 类增长 ⇒ 不判红** ✓
        #   A 类**不进任何上限**（用户【①】：锚定型无限预算）⇒ 这不是"我们选择不判"，
        #   是**结构性不可能**：`anchor_problems` 里 A 只被印出来、从不与任何 cap 比较 ✓。
        def _grow_root(n: int) -> Path:
            """造一个**只含 n 个 A 类守卫**的合成口径（真文件 ⇒ 走 `guard_scripts()` 的 glob ✓）。"""
            r = ta / f"root{n}"
            (r / "scripts").mkdir(parents=True, exist_ok=True)
            for i in range(n):
                (r / "scripts" / f"check-fixture-a{i}.py").write_text("# 夹具守卫\n", encoding="utf-8")
            return r

        def _grow_rows(n: int) -> list[dict]:
            rows = [{"kind": "caps", "round": 0, "caps": {"b_class_guards": 0},
                     "floor": {"b_class_guards": 0},
                     "history": [{"round": 0, "caps": {"b_class_guards": 0},
                                  "reading": {"b_class_guards": 0}, "why": "夹具：B 预算 0"}]},
                    {"kind": "anchor", "id": "A-FIXTURE", "strength": "direct",
                     "site": "夹具：真 VS Code", "action": "打开文件", "reading": "夹具读数"}]
            for i in range(n):
                rows.append({"kind": "guard", "guard": f"scripts/check-fixture-a{i}.py", "class": "A",
                             "anchors": ["A-FIXTURE"], "why": "夹具：A 类守卫"})
            return rows

        red0, _i, c0 = anchor_problems(_grow_root(0), _grow_rows(0))
        red2, _i, c2 = anchor_problems(_grow_root(2), _grow_rows(2))
        gate("⭐⭐ 反向⑭（**两头的那一头**）：**A 类增长 0 → 2 ⇒ 不判红**（A 不进任何上限 ✓）",
             (not red0) and (not red2) and c0["a"] == 0 and c2["a"] == 2,
             f"cap 一个字没动 ⇒ A {c0['a']} → {c2['a']} · B {c0['b']} → {c2['b']} · "
             f"红 {len(red0)} → {len(red2)}")

        # 反向⑩：**caps 被抬过**（第 1 轮 > 第 0 轮）⇒ 判红（方向只能是收紧 ✗）
        up = [dict(r) for r in real_rows]
        for r in up:
            if r.get("kind") == "caps":
                base = r["caps"]["b_class_guards"]
                r["round"] = 1
                r["caps"] = {"b_class_guards": base + 2}
                r["history"] = [{"round": 0, "caps": {"b_class_guards": base}, "reading": {}},
                                {"round": 1, "caps": {"b_class_guards": base + 2}, "reading": {}}]
        red_up, _i, _c = anchor_problems(ROOT, up)
        gate("反向⑮：**caps 被抬过**（第 1 轮 > 第 0 轮）⇒ 判红（上限方向只能是收紧）",
             any("C8 放松" in m for m in red_up), f"红 {len(red_up)} 条")

    # ── 反向①：**bite 返回 0** ⇒ 必须判红（"它咬不住" ✗）──────────────────
    with tempfile.TemporaryDirectory(prefix="soko-bite-") as td:
        tmp = Path(td)
        # 覆盖基线 = HEAD ⇒ 范围内没有守卫 ⇒ 只考 C2
        head, _herr = _git(ROOT, ["rev-parse", "HEAD"])
        head = head.strip()
        nonbiting = tmp / "nonbiting.jsonl"
        nonbiting.write_text(
            f"# coverage_since: {head}\n"
            + json.dumps({"guard": "scripts/check-budgets.py",
                          "bug": "故意：bite 命令永远返回 0（= 咬不住）",
                          "pass": {"cmd": "true", "expect_rc": 0},
                          "bite": {"cmd": "true", "expect_rc": 0}},
                         ensure_ascii=False) + "\n", encoding="utf-8")
        rc_nb = check(ROOT, nonbiting, None, run_all=True, quiet=True)
        gate("反向①：`bite.expect_rc = 0`（**咬不住**）⇒ 判红 rc=1", rc_nb == 1, f"rc={rc_nb}")

        # ── 反向②：**bite 真的返回 0**（期望非 0）⇒ 必须判红 ──────────────
        lying = tmp / "lying.jsonl"
        lying.write_text(
            f"# coverage_since: {head}\n"
            + json.dumps({"guard": "scripts/check-budgets.py",
                          "bug": "故意：期望它报红，它却返回 0",
                          "pass": {"cmd": "true", "expect_rc": 0},
                          "bite": {"cmd": "true", "expect_rc": 1}},
                         ensure_ascii=False) + "\n", encoding="utf-8")
        rc_lie = check(ROOT, lying, None, run_all=True, quiet=True)
        gate("反向②：`bite` 期望 1 却返回 0（**守卫空了**）⇒ 判红 rc=1",
             rc_lie == 1, f"rc={rc_lie}")

        # ── 反向③：**pass 坏了** ⇒ 必须判红 ───────────────────────────────
        broken = tmp / "broken.jsonl"
        broken.write_text(
            f"# coverage_since: {head}\n"
            + json.dumps({"guard": "scripts/check-budgets.py",
                          "bug": "故意：守卫自己坏了",
                          "pass": {"cmd": "false", "expect_rc": 0},
                          "bite": {"cmd": "false", "expect_rc": 1}},
                         ensure_ascii=False) + "\n", encoding="utf-8")
        rc_b = check(ROOT, broken, None, run_all=True, quiet=True)
        gate("反向③：`pass` 返回 1（期望 0）⇒ 判红 rc=1", rc_b == 1, f"rc={rc_b}")

        # ── 反向④：**C1 覆盖** —— 范围内有守卫但台账里没有 ⇒ 必须判红 ──────
        # 夹具：`--since` 指向一个真守卫被改过的提交 ⇒ 范围非空；台账空 ⇒ 必红。
        out, _e = _git(ROOT, ["log", "--format=%H", "--diff-filter=AMR",
                              "-1", "--", "scripts/check-budgets.py"])
        touch = out.strip().splitlines()[-1] if out.strip() else ""
        if touch:
            empty_ledger = tmp / "empty.jsonl"
            empty_ledger.write_text(f"# coverage_since: {head}\n", encoding="utf-8")
            rc_cov = check(ROOT, empty_ledger, touch + "~1", run_all=True, quiet=True)
            gate("反向④：**C1 覆盖** —— 范围内改了守卫、台账里没有 ⇒ 判红 rc=1",
                 rc_cov == 1, f"rc={rc_cov}（since={touch[:12]}~1）")
        else:
            gate("反向④：**C1 覆盖** —— 范围内改了守卫、台账里没有 ⇒ 判红 rc=1",
                 False, "**夹具前提没成立**（git log 没找到改过该文件的提交）✗")

        # ── 反向⑤：**台账腐烂**（点名不存在的守卫）⇒ 必须判红 ────────────
        rotten = tmp / "rotten.jsonl"
        rotten.write_text(
            f"# coverage_since: {head}\n"
            + json.dumps({"guard": "scripts/check-does-not-exist.py",
                          "bug": "故意：点一个不存在的守卫",
                          "pass": {"cmd": "true", "expect_rc": 0},
                          "bite": {"cmd": "false", "expect_rc": 1}},
                         ensure_ascii=False) + "\n", encoding="utf-8")
        rc_r = check(ROOT, rotten, None, run_all=True, quiet=True)
        gate("反向⑤：台账点名**不存在的守卫** ⇒ 判红 rc=1", rc_r == 1, f"rc={rc_r}")

        # ── 反向⑥：台账**缺失** ⇒ rc=3（**不假装判过** ✗）────────────────
        rc_m = check(ROOT, tmp / "nope.jsonl", None, run_all=True, quiet=True)
        gate("反向⑥：台账**不存在** ⇒ rc=3（**不假装判过** ✓）", rc_m == 3, f"rc={rc_m}")

        # ── 反向⑦：**C4 接线** —— 声明了一个 `ci.yml` 里没有的 job ⇒ 判红 ──
        #   （**分派的空转闸**：没有它，把证明标成不存在的 job 就等于藏起来 ✗）
        #   ⚠ 这条夹具**其余都合法**（pass/bite 都对）⇒ 判红**可归属**到 C4 ✓。
        ghost = tmp / "ghost-job.jsonl"
        ghost.write_text(
            f"# coverage_since: {head}\n"
            + json.dumps({"guard": "scripts/check-budgets.py",
                          "ci_job": "no-such-job-in-ci-yml",
                          "bug": "故意：把证明声明到一个不存在的 CI job（= 藏起来）",
                          "pass": {"cmd": "true", "expect_rc": 0},
                          "bite": {"cmd": "false", "expect_rc": 1}},
                         ensure_ascii=False) + "\n", encoding="utf-8")
        rc_g = check(ROOT, ghost, None, run_all=True, quiet=True)
        gate("反向⑦：**C4 接线** —— `ci_job` 指向不存在的 job（藏起来 = 空转）⇒ 判红 rc=1",
             rc_g == 1, f"rc={rc_g}")

        # ── 反向⑧：job 存在、但**里面没有那一跑** ⇒ 判红（同上，另一档）──────
        unwired = tmp / "unwired-job.jsonl"
        unwired.write_text(
            f"# coverage_since: {head}\n"
            + json.dumps({"guard": "scripts/check-budgets.py",
                          "ci_job": "lint-fmt",
                          "bug": "故意：job 是真的，但里面没有 `--job lint-fmt` 那一跑",
                          "pass": {"cmd": "true", "expect_rc": 0},
                          "bite": {"cmd": "false", "expect_rc": 1}},
                         ensure_ascii=False) + "\n", encoding="utf-8")
        rc_u = check(ROOT, unwired, None, run_all=True, quiet=True)
        gate("反向⑧：job 存在但**没有那一跑**（`--job <名字>` 不在）⇒ 判红 rc=1",
             rc_u == 1, f"rc={rc_u}")

        # ⭐⭐ **第 6 轮 `ng7`**：C4 的"真的跑"收窄成 **`run:` 正文** —— 两向各一条臂 ✓
        #   （收窄前它拿的是**整个 job 块的文本** ⇒ 只在 **step 名**里点名就能通过 ✗）。
        def _c4_root(tag: str, step: str) -> Path:
            """夹具仓：只有一份 `ci.yml`（C4 只读它 ✓ —— 不碰别的判据的输入 ✓）。

            ⚠ `tag` **每个臂都不一样**（同 `_face_root` 的实测教训 ✗）。
            """
            r = ta / f"c4-{tag}"
            wf = r / ".github" / "workflows"
            wf.mkdir(parents=True, exist_ok=True)
            (wf / "ci.yml").write_text(
                "jobs:\n  guard-bite:\n    runs-on: ubuntu-24.04\n    steps:\n" + step,
                encoding="utf-8")
            return r

        def _c4_entry(job: str = DEFAULT_JOB) -> list[dict]:
            return [{"guard": "scripts/check-budgets.py", "ci_job": job,
                     "bug": "夹具：C4 的「真的跑」口径（第 6 轮收窄）",
                     "pass": {"cmd": "true", "expect_rc": 0},
                     "bite": {"cmd": "false", "expect_rc": 1}}]

        # 正对照：`run:` 正文里**真的**跑它 ⇒ 不许红 ✓（收窄不误伤真跑 ✓）。
        c4_real = _c4_root("real-run", "      - run: python3 scripts/check-guard-bite.py --check --all\n")
        c4_real_red = wiring_problems(c4_real, _c4_entry())
        gate("⭐⭐ 正对照：C4 —— `run:` 正文里**真的**跑 `check-guard-bite.py` ⇒ 不许红",
             not c4_real_red, f"红 {len(c4_real_red)} 条")

        # 反向：**删 `run:` 正文、只留 step 名** ⇒ 必须判红（提及 ≠ 跑 ✗）。
        c4_mention = _c4_root(
            "mention-only",
            "      - name: 跑 scripts/check-guard-bite.py --check --all（`run:` 正文被删 ✗）\n"
            "        run: echo hi\n")
        c4_mention_red = wiring_problems(c4_mention, _c4_entry())
        c4_named = any("C4 接线" in m and "guard-bite" in m for m in c4_mention_red)
        gate("⭐⭐ 反向㉕：C4 —— **删 `run:` 正文、只留 step 名** ⇒ 判红（第 6 轮收窄；"
             "收窄前这条**通过** ⇒ 接线只靠提及）", c4_named, f"红 {len(c4_mention_red)} 条")
        if c4_mention_red:
            print(f"      ↳ 本臂判红原文（**不是本跑的判据** ✓）：{c4_mention_red[0][:90]}")

        # ── C13 登记接线：**正反两向**（2026-10-18 · 用户 2026-10-17 的硬令 ✓）────────
        #   ⚠ 夹具**照真仓的形状**（拷真形状、不打桩 ✗）：一份 `scripts/ux-queue.py`
        #   （**只有** `REGISTERED_SCRIPTS` 那一行 —— C13 只读它 ✓）+ 一份接线
        #   （C13 只读 `WIRING_FILES` ✓）+ 登记表点名的**真存在的**脚本文件 ✓。
        #   ⚠ `tag` **每个臂都不一样**（同 `_c4_root` / `_face_root` 的实测教训 ✗）。
        def _c13_root(tag: str, *, registry: str | None, wiring: str | None,
                      scripts: tuple[str, ...] = ("scripts/fixture-a.mjs",
                                                  "scripts/fixture-b.mjs")) -> Path:
            r = ta / f"c13-{tag}"
            (r / "scripts").mkdir(parents=True, exist_ok=True)
            if registry is not None:
                (r / REGISTRY_FILE).write_text(registry, encoding="utf-8")
            for rel in scripts:
                (r / rel).write_text("// 夹具（C13 只判它在不在树里 ✓）\n", encoding="utf-8")
            if wiring is not None:
                wf = r / ".github" / "workflows"
                wf.mkdir(parents=True, exist_ok=True)
                (wf / "ci.yml").write_text(
                    "jobs:\n  guard-bite:\n    runs-on: ubuntu-24.04\n    steps:\n" + wiring,
                    encoding="utf-8")
            return r

        _reg_a = f'{REGISTRY_NAME} = {{\n    "scripts/fixture-a.mjs",\n}}\n'

        # 正对照：登记表点名的那条在 `run:` 正文里**真的跑** ⇒ 不许红 ✓。
        c13_ok = _c13_root("real-run", registry=_reg_a,
                           wiring="      - run: node scripts/fixture-a.mjs\n")
        c13_ok_red, c13_ok_reg, c13_ok_err = registered_wiring_problems(c13_ok)
        gate("⭐⭐ 正对照：C13 —— 登记表点名的脚本在 `run:` 正文里**真的跑** ⇒ 不许红",
             not c13_ok_red and c13_ok_err is None
             and c13_ok_reg == ["scripts/fixture-a.mjs"],
             f"红 {len(c13_ok_red)} 条" + (f" · {c13_ok_err}" if c13_ok_err else ""))

        # ⭐⭐ 反向㉖：**登记了但没接线**（登记表点名、任何地方都不跑）⇒ 必须判红 ✓ ——
        #   这就是用户 2026-10-17 报的形状（`vscode-input-vim-e2e.mjs` **真接线 0/2** ✗）。
        c13_bare = _c13_root("registered-not-wired", registry=_reg_a, wiring=None)
        c13_bare_red, _c13r, _c13e = registered_wiring_problems(c13_bare)
        gate("⭐⭐ 反向㉖：C13 —— **登记了但没接线**（`REGISTERED_SCRIPTS` 点名、"
             "任何 `run:` 正文里都没有它）⇒ 判红（修前这条形状**没有任何判据看得见**）",
             any("C13 登记接线" in m and "fixture-a.mjs" in m for m in c13_bare_red),
             f"红 {len(c13_bare_red)} 条")
        # ⚠ **本臂的名字里不许带判红标记**（`✗`）—— 2026-10-18 CI 实测 ✗：`gate()` 的
        #   「读数可归属」那条规则管的是**整个 `--selftest` 流**（`mixed` ⇒ `bad += 1`），
        #   而本臂的名字原来写成「…没有任何判据看得见 **✗**）」⇒ 一个**字面 `✗`**
        #   就把整轮 `--selftest` 判成「1 组不对」✗（`queue-lint` 当场红 ⇒ 挡发版）。
        #   ⇒ 判红标记**只许**出现在 `↳` 自述行里（下面那行 ✓），**不许**进 `✓` 判据行 ✓。
        if c13_bare_red:
            print(f"      ↳ 本臂判红原文（**不是本跑的判据** ✓）：{c13_bare_red[0][:90]}")

        # 反向㉗：**删 `run:` 正文、只留 step 名** ⇒ 必须判红（口径与 C4 逐字相同 ✓）。
        c13_mention = _c13_root(
            "mention-only", registry=_reg_a,
            wiring="      - name: node scripts/fixture-a.mjs（`run:` 正文被删 ✗）\n"
                    "        run: echo hi\n")
        c13_mention_red, _c13r, _c13e = registered_wiring_problems(c13_mention)
        gate("⭐⭐ 反向㉗：C13 —— **删 `run:` 正文、只留 step 名** ⇒ 判红"
             "（口径与 C4 逐字相同 ✓）",
             any("C13 登记接线" in m and "fixture-a.mjs" in m for m in c13_mention_red),
             f"红 {len(c13_mention_red)} 条")

        # 反向㉘：**登记表点名一个不在树里的脚本** ⇒ 判红（表不腐烂，同层② R2 的形状 ✓）。
        c13_rot = _c13_root("registry-rot", registry=_reg_a, wiring=None, scripts=())
        c13_rot_red, _c13r, _c13e = registered_wiring_problems(c13_rot)
        gate("⭐⭐ 反向㉘：C13 —— 登记表点名**不在树里**的脚本 ⇒ 判红（表不腐烂 ✓）",
             any("C13 登记表腐烂" in m and "fixture-a.mjs" in m for m in c13_rot_red),
             f"红 {len(c13_rot_red)} 条")

        # 正对照：`scripts/ci-local.sh` 的**非注释行**也算接线 ⇒ 不许红 ✓
        #   （⚠ 同一份文件里那行**注释**不算 —— 正是"注释不算跑"那一半 ✓）。
        c13_local = _c13_root("local-gate", registry=_reg_a, wiring=None)
        (c13_local / WIRING_FILES[0]).write_text(
            "#!/bin/sh\n# node scripts/fixture-a.mjs（**注释不算** ✗）\n"
            "node scripts/fixture-a.mjs\n", encoding="utf-8")
        c13_local_red, _c13r, _c13e = registered_wiring_problems(c13_local)
        gate("⭐⭐ 正对照：C13 —— `scripts/ci-local.sh` 的**非注释行**也算接线 ⇒ 不许红"
             "（同文件里那行注释不算 ✓）", not c13_local_red, f"红 {len(c13_local_red)} 条")

        # ⭐⭐ 正对照（**本判据自己的咬住面** ✓）：**真仓**的登记表点名的每一条
        #   都必须在接线里**真的跑** ⇒ 不许红（把接线删掉 ⇒ 这条当场红 ✓）。
        real_reg, real_reg_err = registered_scripts(ROOT)
        real_c13_red, _c13r, _c13e = registered_wiring_problems(ROOT)
        gate("⭐⭐ 正对照：C13 —— **真仓**的登记表点名的每一条都在接线里真的跑 ✓",
             not real_c13_red and real_reg_err is None,
             f"登记 {len(real_reg)} 条（{' · '.join(real_reg)}）· 红 {len(real_c13_red)} 条"
             + (f" · {real_reg_err}" if real_reg_err else ""))

        # ── 反向⑨：`--job <没人声明的名字>` ⇒ 判红（**空转闸**：0 条不许算绿 ✗）──
        rc_e = check(ROOT, LEDGER, None, run_all=True, quiet=True, job="no-such-job")
        gate("反向⑨：`--job <台账里没人声明的名字>`（0 条 = 空转）⇒ 判红 rc=1",
             rc_e == 1, f"rc={rc_e}")

        # ── C5 可达性：**正反两向**（2026-10-16 CI 实测红 ✗→✓）────────────────
        #   ⚠ 夹具的 rev 必须**确定性**地"能解析、但不是 `HEAD` 的祖先" ⇒ 用
        #   `git commit-tree HEAD^{tree}` 造一个**无父**的提交（**不建 ref** ⇒ 不可达 ✓，
        #   松散对象会被 gc 收掉 ✓）。⚠ **身份必须显式给**（实测踩到过 ✗）：CI 的
        #   checkout **不保证**有 `user.name`/`user.email` ⇒ 裸 `git commit-tree` 会以
        #   「Committer identity unknown」失败 ⇒ 自检臂在 CI 上**自己红** ✗ ⇒
        #   `-c user.name=… -c user.email=…` + `GIT_*_DATE`（与宿主配置**无关** ✓）。
        orphan = subprocess.run(
            ["git", "-c", "user.name=soko-bite", "-c", "user.email=bite@invalid.invalid",
             "commit-tree", "HEAD^{tree}", "-m", "soko-bite-selftest orphan（不可达）"],
            cwd=ROOT, capture_output=True, text=True,
            # ⚠ 先摘 `GIT_*` 再补日期（洞 `FIXTURE-GIT-ENV-LEAKS-INTO-REAL-REPO`）：
            #   原来直接 `{**os.environ, …}` ⇒ 调用方的 `GIT_DIR` 会让 `HEAD^{tree}`
            #   与那个松散对象都落到**调用者的仓**上 ✗（本句是**写**操作，不只是读数 ✗）。
            env={**git_env.clean_git_env(),
                 "GIT_AUTHOR_DATE": "2000-01-01T00:00:00+0000",
                 "GIT_COMMITTER_DATE": "2000-01-01T00:00:00+0000"})
        orph = orphan.stdout.strip()
        if orphan.returncode == 0 and orph:
            def _c5_fixture(name: str, rev: str) -> Path:
                """同一个夹具，只换钉的 rev（⇒ 判红**可归属**到 C5 ✓）。"""
                p = tmp / name
                p.write_text(
                    f"# coverage_since: {head}\n"
                    + json.dumps({"guard": "scripts/check-budgets.py",
                                  "bug": f"故意：钉一个 rev（{rev[:12]}）",
                                  "pass": {"cmd": "true", "expect_rc": 0},
                                  # `bite` 恒 rc=1 ⇒ C2 放行 ⇒ 判红只可能来自 C5 ✓
                                  "bite": {"cmd": f"git cat-file -e {rev}^{{commit}}; false",
                                           "expect_rc": 1}},
                                 ensure_ascii=False) + "\n", encoding="utf-8")
                return p

            rc_c5ok = check(ROOT, _c5_fixture("c5-reachable.jsonl", head),
                            None, run_all=True, quiet=True)
            gate("正对照：**C5** —— 钉的 rev 是 `HEAD` 的祖先 ⇒ **不**判红 rc=0",
                 rc_c5ok == 0, f"rc={rc_c5ok}（rev=HEAD={head[:12]}）")
            rc_c5 = check(ROOT, _c5_fixture("c5-orphan.jsonl", orph),
                          None, run_all=True, quiet=True)
            gate("反向⑩：**C5 可达性** —— 钉的 rev **能解析**、但**不是** `HEAD` 的祖先"
                 "（`land` rebase 之前的孪生就是这个形状）⇒ 判红 rc=1",
                 rc_c5 == 1, f"rc={rc_c5}（rev={orph[:12]}）")
        else:
            gate("反向⑩：**C5 可达性** ⇒ 判红 rc=1", False,
                 f"**夹具前提没成立**（`git commit-tree` rc={orphan.returncode}："
                 f"{orphan.stderr.strip()[:120]}）✗")

    # ── C10 夹具前提：**正反两向**（2026-10-16 · 洞 `GUARD-BITE-SELFTEST-ONE-GROUP-RED-ON-CI`）──
    #   正对照 = **真台账**里钉的状态夹具（修好之后）都是**合法**状态 ⇒ 不判红 ✓；
    #   反向 = 把**真夹具**的末轮读数抬到越过它**自己**的 caps ⇒ 必须判红 ✓
    #   （⚠ 反向量**从真夹具派生** ⇒ 与"夹具长什么样"不脱钩 ✓；**钉死旧 rev** 的那条
    #    在台账里 ✓ —— 这里不钉 rev，免得 rebase 之后 `--selftest` 自己坏 ✗）。
    fx_red, fx_judged = state_fixture_problems(ROOT, entries)
    gate("正对照：**真台账**里钉的状态夹具都是**合法**状态 ⇒ 不判红",
         not fx_red, f"判到 {len(fx_judged)} 个：{' · '.join(fx_judged) or '（无）'}")
    real_fx = ROOT / "scripts" / "fixtures" / "threshold-ratchet-tight.json"
    if real_fx.exists():
        with tempfile.TemporaryDirectory(prefix="soko-statefx-") as td_fx:
            bad_fx = Path(td_fx) / "tight-broken.json"
            st_bad = json.loads(real_fx.read_text(encoding="utf-8"))
            last = st_bad["history"][-1]
            # 越过**它自己的** caps（= 修前那份真夹具的形状 ✓，只是让它**确定性**地越界）
            last["reading"]["docs_live_bytes"] = int(last["caps"]["docs_live_bytes"]) + 1
            bad_fx.write_text(json.dumps(st_bad, ensure_ascii=False), encoding="utf-8")
            synth = [{"guard": "scripts/docs-lint.py",
                      "bug": "夹具：末轮读数越过它**自己**的上限",
                      "pass": {"cmd": "true", "expect_rc": 0},
                      "bite": {"cmd": f"SOKO_RATCHET_STATE={bad_fx} true", "expect_rc": 1}}]
            fx_bad, _j = state_fixture_problems(ROOT, synth)
            gate("⭐⭐ 反向⑱：证明钉的状态夹具**不是合法状态**（末轮读数越过**它自己的** caps）"
                 "⇒ 判红（C10 夹具前提 —— 修前那份真夹具就是这个形状）",
                 any("C10 夹具前提" in m for m in fx_bad), f"红 {len(fx_bad)} 条")
        # ⚠ **不判不存在的那一档**（它是**故意**的坏输入 ✓）：`/nonexistent` 那两条期望 rc=3
        #   ⇒ 判它们红就是**假红** ✗ —— 这条臂把它钉住（防止有人"顺手"把它也判红 ✗）。
        synth_none = [{"guard": "scripts/check-site.py",
                       "bug": "夹具：状态文件**故意**不存在",
                       "pass": {"cmd": "true", "expect_rc": 0},
                       "bite": {"cmd": "SOKO_RATCHET_STATE=/nonexistent true", "expect_rc": 3}}]
        fx_none, _j2 = state_fixture_problems(ROOT, synth_none)
        gate("⭐⭐ 正对照：状态夹具**故意不存在**（`SOKO_RATCHET_STATE=/nonexistent`，"
             "台账里两条期望 rc=3）⇒ **不**判红（判它就是假红）",
             not fx_none, f"红 {len(fx_none)} 条")
    else:
        gate("⭐⭐ 反向⑱：状态夹具越界 ⇒ 判红", False,
             f"**夹具前提没成立**（`{real_fx}` 不在）✗")

    # ── C11 命令前提：**正反两向**（2026-10-16 · 洞 `BITE-RC-127-COMMAND-NOT-FOUND`）──
    #   正对照 = **真台账**的命令位路径**都在树里** ⇒ 不判红 ✓；
    #   反向 = 命令位上钉一个**不在树里**的路径 ⇒ 必须判红 ✓。
    #   ⚠ 这一组是**纯函数**臂（`command_premise_problems` 只读 `root`）⇒ 造一个**空根**，
    #   判红就**可归属**到 C11 ✓ —— **不钉 rev**（免得 rebase 之后 `--selftest` 自己坏 ✗）；
    #   钉死旧 rev 的那一条在台账里 ✓（`11bc8e3f` = CI 真红的那一跑的树）。
    c11_red, c11_judged, c11_skip = command_premise_problems(ROOT, entries)
    gate("⭐⭐ 正对照：**真台账**的命令位路径**都在树里** ⇒ C11 不判红",
         not c11_red, f"判到 {len(c11_judged)} 个 · 不判 {len(c11_skip)} 条")
    with tempfile.TemporaryDirectory(prefix="soko-cmdpremise-") as td_c11:
        empty_root = Path(td_c11)        # **空根** ⇒ 每个命令位路径都不在 ⇒ 必红 ✓
        synth_missing = [{"guard": "scripts/check-guard-bite.py",
                          "bug": "夹具：命令位钉了一个**不在树里**的脚本"
                                 "（= 台账行先落、脚本后落）",
                          "pass": {"cmd": "true", "expect_rc": 0},
                          "bite": {"cmd": "scripts/bite-artifact-temp-name-unique.sh e2cc1012",
                                   "expect_rc": 101}}]
        c11_bad, _j, _s = command_premise_problems(empty_root, synth_missing)
        gate("⭐⭐ 反向⑲：证明的**命令位**钉了一个**不在树里**的路径 ⇒ 判红"
             "（C11 命令前提 —— 修前它被印成「这条守卫咬不住」）",
             any("C11 命令前提" in m for m in c11_bad), f"红 {len(c11_bad)} 条")
        if c11_bad:
            print(f"      ↳ 本臂判红原文（**不是本跑的判据** ✓）：{c11_bad[0][:90]}")
        # ⚠ **不判"参数里的路径"那一档**（**如实划的口径**，不是判过 ✗）：参数里到处是
        #   路径，而有些**故意不存在**（台账里两条 `SOKO_RATCHET_STATE=/nonexistent`
        #   期望 rc=3 ✓）⇒ 判它们红就是**假红** ✗ —— 这条臂钉住"只看命令位"这一条 ✓。
        synth_arg = [{"guard": "scripts/check-site.py",
                      "bug": "夹具：路径在**参数**里（而且故意不存在）",
                      "pass": {"cmd": "true", "expect_rc": 0},
                      "bite": {"cmd": "SOKO_RATCHET_STATE=/nonexistent python3 "
                                       "scripts/check-site.py --only assets",
                               "expect_rc": 3}}]
        c11_arg, c11_arg_j, _s2 = command_premise_problems(ROOT, synth_arg)
        gate("⭐⭐ 正对照：路径在**参数**里（`SOKO_RATCHET_STATE=/nonexistent` 那两条的形状）"
             "⇒ **不**判红（C11 只看**命令位** ⇒ 判它就是假红）",
             (not c11_arg) and "/nonexistent" not in c11_arg_j,
             f"红 {len(c11_arg)} · 判到 {c11_arg_j}")
        # ⚠ ⭐ **夹具仓 ⇒ 本跑不判**（与 C5/C6 **逐字同一条口径** ✓）：这一条臂是**实测**
        #   逼出来的 —— C11 落地后 `check-attention-budget.py --selftest` 的
        #   `_fixture_repo()`（把**真台账**拷进一个 `git init` 的夹具仓 ✓）当场 **4 组红** ✗
        #   （夹具仓里那些命令位路径**当然不在**）⇒ C11 必须认「这不是台账所属的那个仓」✓。
        #   ⚠ **不是放宽** ✗：真仓里照样判红（CI 形状的 `11bc8e3f` 树上实测 1 条红 ✓）。
        c11_fx, _jf, _sf = command_premise_problems(empty_root, synth_missing, own_repo=False)
        gate("⭐⭐ 正对照：**夹具仓**（`coverage_since` 不在本仓 ⇒ `own_repo=False`）"
             "⇒ C11 **本跑不判**（与 C5/C6 同一条口径；判它就是**合法夹具假红**）",
             not c11_fx, f"红 {len(c11_fx)} 条")

    # ── 正对照：**真坏输入**被咬住的形状（`--until c82097fe` = 历史最差点）──
    entries2, _cs2, _e2 = load_ledger(LEDGER)
    hist = next((e for e in entries2
                 if e["guard"].endswith("check-budgets.py")), None)
    if hist:
        rc_h, tail_h = run_proof(hist["bite"]["cmd"], ROOT)
        gate("⭐ 正对照：**历史最差点**（`--until c82097fe` ⇒ 19.32 : 1）**真被咬住**",
             rc_h == hist["bite"]["expect_rc"] and rc_h != 0, f"rc={rc_h}")
        # ⚠ **读数可归属**（同 `check()` 那条）：证明自身的输出**单独一行** ✓ ——
        #   它这里**必然**是红的（那条 bite 就是要把 19.32 : 1 判红 ✓）。
        if rc_h != 0:
            print(f"      ↳ 证明自身的输出（**不是本跑的判据** ✓）：{tail_h[:100]}")
    else:
        gate("⭐ 正对照：历史最差点被咬住", False, "台账里没有 check-budgets 那一行 ✗")

    print(f"\n自检：{'✓ 全过' if bad == 0 else f'✗ {bad} 组不对'}"
          f"（正对照 {tally.get('正对照', 0)} + 反向 {tally.get('反向', 0)} + 真跑台账；"
          f"含**锚的两头**：B 超上限 ⇒ 判红 · A 增长 ⇒ 不判红 ✓"
          f" · **枚举面**：改了名的守卫仍在口径里 ✓ · 幽灵名字不进 ✓ · `tool` 悬空覆盖 ⇒ 判红 ✓"
          f" · **夹具前提**：钉的状态夹具不是合法状态 ⇒ 判红 ✓"
          f" · **命令前提**：命令位钉了一个不在树里的路径 ⇒ 判红 ✓（参数里的路径**不**判 ✓）"
          f" · **两面**：只删 CI 面 ⇒ 判红 ✓ · **只删本地面 ⇒ 也判红 ✓**（第 5 轮收紧）"
          f" · 豁免腐烂 / witness 不成立 ⇒ 判红 ✓）")
    return 0 if bad == 0 else 1


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description="咬住证明（守卫必须咬得住一条已存在的坏输入）")
    ap.add_argument("--root", default=str(ROOT))
    ap.add_argument("--ledger", default=str(LEDGER))
    ap.add_argument("--since", default=None, help="覆盖面 = <sha>..HEAD（默认台账的 coverage_since）")
    ap.add_argument("--all", action="store_true",
                    help="C2 跑**本 job 的**每一条（不只范围内的）")
    ap.add_argument("--job", default=DEFAULT_JOB,
                    help=f"跑哪个 CI job 的那批证明（台账的 `ci_job`；缺省 `{DEFAULT_JOB}`；"
                         f"`{JOB_ALL}` = 不分工，开发机一次全跑 ✓）。"
                         f"⚠ 每个 `ci_job` 都必须在 `ci.yml` 里**真的有一跑** ⇒ 否则 C4 判红 ✗")
    ap.add_argument("--check", action="store_true",
                    help="（默认动作；显式写出来给调用方用 ✓ —— `check-budgets.py "
                         "--layer attention --open-round` 就是这么调的 ✓）")
    ap.add_argument("--list", action="store_true", help="只印覆盖情况（含 grandfathered 清单 + A/B 分类）")
    ap.add_argument("--counts", action="store_true",
                    help="只印 A/B/锚/上限/两面的**机器可读**计数（JSON；给 ratchet 读数用 ✓）")
    ap.add_argument("--anchors", default=None,
                    help="分类登记表路径（缺省 `docs/guard-anchors.jsonl`；复现 / 咬住证明用 ✓）")
    ap.add_argument("--reopen-empty", action="store_true",
                    help="⭐ ① 翻账：咬不住的 ⇒ reopen 它关掉的那条（回 todo）")
    ap.add_argument("--selftest", action="store_true")
    args = ap.parse_args(argv)

    for stream in (sys.stdout, sys.stderr):
        try:
            stream.reconfigure(encoding="utf-8", errors="backslashreplace")
        except (AttributeError, ValueError, OSError):
            pass

    if args.selftest:
        return selftest(args.job)

    root = Path(args.root).resolve()
    ledger = Path(args.ledger)
    if not ledger.is_absolute():
        ledger = (root / ledger).resolve()
    anchors = None
    if args.anchors:
        anchors = Path(args.anchors)
        if not anchors.is_absolute():
            anchors = (root / anchors).resolve()
    if args.counts:
        arows, aerr = load_anchors(anchors or (root / "docs" / "guard-anchors.jsonl"))
        if aerr:
            print(json.dumps({"ok": False, "error": aerr}, ensure_ascii=False))
            return 3
        a_red, _a_info, a_counts = anchor_problems(root, arows)
        f_red, _f_info, f_counts = face_problems(root, arows)
        print(json.dumps({"ok": not (a_red or f_red), "red": a_red + f_red, **a_counts,
                          "faces": f_counts}, ensure_ascii=False))
        return 1 if (a_red or f_red) else 0
    if args.list:
        return list_coverage(root, ledger, anchors)
    return check(root, ledger, args.since, args.all, reopen_empty=args.reopen_empty,
                 job=args.job, anchors_path=anchors)


if __name__ == "__main__":
    sys.exit(main())

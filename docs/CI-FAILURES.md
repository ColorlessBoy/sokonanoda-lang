## 2026-09-28 · run `36347147870` · `test (sokonanoda-lsp, tests)`

**现象**：`perf_course::perf_course_did_open_is_recorded` 判红，卡在**绝对量级哨兵**
`slowest < 180_000` —— CI 实测 unit01 **19208ms** / unit08 **75335ms** / unit12 **197064ms** ✗。
同轮**形状判据**（unit12/unit01 ≈ 10.4 < 12，与机器快慢无关）**是过的** ✓ ⇒ 没有 O(n²) 那类真退化 ✓。
该测试注释里的 CI 基线是 **62s** ⇒ 这个 runner 慢了 **~3×** ✗（180s 只有 2.9× 余量）。

**先排除"是我们改慢了"** ✓（这一步别省 ✗）：把当轮的两个改动（`notation.rs` 的指令行 span、
`project/graph.rs` 的闭包表 seed）**原地 stash 掉再量同一条命令** ⇒
**2356/6918/14412ms vs 2330/6894/14385ms ⇒ 差 ~0.2%** ✓（同机同负载）⇒ **不是我们的改动** ✓。

**修复**：绝对哨兵 **180s → 300s**（仍抓"退化成几分钟"的真事故 ✓）；**真正的守卫是那条比值** ✓。
**预防**：绝对阈值**天生跨机器**（`AGENTS.md` 的 perf-gate 教训：调到多大都追不上机器差 ✗）——
新增量级哨兵时**必须**同时给一条**与机器无关**的形状判据 ✓，且绝对那条只当"几分钟级"的事故网 ✓。

## 2026-09-27 · run `36340364435` · `e2e (macos-latest · VS Code 1.138.0)`

**现象**：`the Infoview receives the project view (E30)` 判红 —— `TypeError: Cannot read
properties of undefined (reading 'project')` at `src/test/extension.test.js:713`。
同轮 **ubuntu 两条腿都绿**、本机（darwin release）也绿 ⇒ 先按值守口径 rerun 一次
（`gh run rerun --job 108679507624`）⇒ **仍红** ✗ ⇒ 不是 flake，去取 artifact 定位 ✓。

**真因**：不是 macos 的问题，是**判据自己"等 A 断 B"** ✗✓ —— 那条 e2e 等的是
`infoview.lastProject()`，最后一条断言读的却是 `extensionApi.project.answer.project.root`
（**项目树**那份答案，另一条路填 ✗）。快机器上两条路几乎同时到 ⇒ 看不见 ✗；
macos runner 慢 ⇒ 树还没答 ⇒ `answer` 为 `undefined` ⇒ TypeError ✓。

**修复**：`waitFor` 的条件**同时**等两个信号（`infoview.lastProject()` **和**
`project.answer`）—— 等到的与要断言的一致 ✓。

**预防**：**"等 A 断 B"是 e2e 的经典假红，且只在慢 runner 上现形** ✗✓ ⇒ 把"后面要读的
每一个异步状态"都列进 `waitFor` 条件 ✓；**快机器绿、慢机器红的，先怀疑它，别叫 flake** ✗。
⚠ **rerun 只能证伪 flake，不能消红** ✓ —— 仍红就去取 artifact（`gh run download <run-id>`
⇒ `e2e-<os>-vscode-<ver>/logs/<date>-<sha>-*.log` 的「## 失败详情」✓；沙箱里取日志要
`XDG_CACHE_HOME=/tmp/ghcache` ✓）。

## 2026-09-27 · **T-A60-2 的 1.5s 窗口被「迟到的落盘」压红** ✗（run `36323798295` 的 `e2e (ubuntu-24.04 · VS Code 1.138.0)`）
**现象** ✓：同一 commit（`f2ec5d0`）在 **macos 1.138.0 与 ubuntu 1.106.0 都 32/32 绿** ✓，
只有 **ubuntu 1.138.0** 红 1 条 ✗：`rewriting an unchanged project unit does not recompile`
⇒ `AssertionError: 内容没变 ⇒ 不许写出新的缓存条目（说明闭包被重编了）` ✓。
**证据** ✓（从该 job 的 artifact 里取，**不等整轮** ✓）：基线之后多出两条 ——
`949171fcb4d6fa0c.json:2101` 与 `c2a5a26a2cc7006d.json:5174`；而**夹具闭包的条目是 155KB 级**
（u01 实测 `155213`）、`lib/Set` 单独编是 **26KB 级**（实测 `26184`）⇒ **这两条不是它的闭包** ✗
⇒ 是**别处的编译迟到了** ✓。
**根因** ✓：这条判据本身没错（"同一份字节重写 ⇒ 不许写新条目"），但它的 **1.5s 固定窗口**
对邻居用例迟到的落盘**没有免疫力** ✗ —— 这正是它自己注释里记着的老毛病
（`editor/vscode/src/test/extension.test.js` 的 T-U12 面 #3 那段：早先一次是"夹具写进 tmpDir
⇒ 落盘迟到 ⇒ 撞进这条窗口" ✗）。E22 起 build/rebuild 真的编**整个项目**、rebuild 真的
**清项目缓存** ⇒ 共享夹具的缓存被扰动得更多 ⇒ 慢 runner 上更容易撞上
（**相关性，不是已证的因果** ✗：同 commit 2/3 平台绿 ✓）。
**修复** ✓：**先等缓存静止、再取基线**（`waitFor` 连续两次 `cacheStamp()` 逐字节相同）——
与 T-A60-3 的治法同款（"先等稳定再断言"：幂等判据，**不赌时间** ✗）。
**判据一个字没松** ✓：基线之后仍必须**逐条相等** ✓。
**预防** ✓：凡「快照 → 等 N 秒 → 再快照」型判据，**先做静默期（quiescence）再取基线** ✗；
共享缓存（全局 + 项目根）是**跨用例的隐式状态** ⇒ 动了它的用例要负责还原/预热 ✓。

## 2026-09-27 · **改了课程计数，漏改一个测试钉子** ✗（run `36314757444` 的 `test (sokonanoda-cli, tests)`）
**现象** ✓：整轮 `failure`，逐 job 查只有 `test (sokonanoda-cli, tests)` 红 ✗ ——
`query_check_matches_grade_on_a_real_course_unit` FAILED：
`assertion left == right failed: …"decl_checked":4…`，`left: 4` / `right: 5` ✓。
**根因** ✓：E02 把 `def Set.prod` 从单元⑤ 画布收进 `lib/Prod` ⇒ 该画布 `decl_checked` **5 → 4**
（练习数/open 数没动 ✓）；而 E02 当时找计数钉子只 grep 了 `course.rs`/`course_status.rs`/`cli.rs`
**三个文件** ✗ —— **漏了 `query.rs`**（它拿真课程文件当夹具 ✗，名字里没有 "course" ✗）。
**修复** ✓（`a30ffc7`）：钉子改 4 + 原地写明"基线在 E02 变过"；顺手修 `front/src/compile/tests.rs`
一处陈旧注释（说 unit12 有"那 5 个 R5 标记"，其实已清零）。
**预防** ✓：**改了课程计数 ⇒ 全测试树 grep 计数钉子** ✓（`decl_checked`/`exercise_open` + 被改文件名），
**不许只 grep 名字里带 course 的那几个文件** ✗；并且**本地跑全套**
`cargo test -p sokonanoda-cli`（25 个 target）确认"只有这一处" ✓（实测 0 FAILED ✓）。
**监控教训（与上一轮同源）** ✗✓：我先前用 `sleep 300/420` 长轮询**只盯整轮 conclusion** ✗ ——
而 matrix 里一个 job 早挂了、整轮还在 `in_progress` ⇒ 白等几分钟 ✗。
**改成 60–90s 逐 job 看** ✓：`gh run view <id> --json jobs | jq -r '.jobs[] | "\(.conclusion // .status)\t\(.name)"'` ✓。

## 判据
本机行为**不变** ✓（本地本来就解析仓库构建 ✓）；**CI 侧由下一轮确认** ✓
—— 这是**必须推**的那类改动 ✓（本地绿 ≠ CI 绿 ✓ 本 session 已证 ✓）。

## 2026-09-25 · **用户指出：整轮的 `in_progress` 掩盖了已发生的失败** ✗（round 182 修 ✓）
**用户原话** ✓："ledger **立马就失败**了，但是**整个 github action 还在继续**，
导致**你不知道已经失败了**" ✓ —— 完全成立 ✓，而且我在旁边反复撞这堵墙 ✗：
* 实测 ✓（`#36151088821` ✓）：整轮 `in_progress` ✗，而 `ledger (3)` **15:09:04 就红了** ✗
  （`test/*` · `gates` 还在跑 ✓，要十几分钟 ✓）⇒ **失败信号被拖住了** ✗；
* **为什么我读不到细节** ✗：`gh run view --job <id> --log` 在**整轮结束前**回
  `run … is still in progress` ✓ ⇒ **日志拿不到** ✗（本 session 为此刻苦了半小时 ✓）。

### 两处修 ✓
1. **`scripts/gap.py` 写 `$GITHUB_STEP_SUMMARY`** ✓ —— 每片无论红绿都写一行结论 ✓
   （红时给出"本地复跑这一片"的命令 ✓）⇒ **页面上**一眼可见 ✓、**不必等整轮** ✓。
2. **`scripts/ci-watch.sh`** ✓ —— **按 job 看，不看整轮** ✓：
   `gh run view <id> --json jobs` ✓ ⇒ 逐 job 打印结论 ✓，**只要最新一轮有 job 红就 `exit 1`** ✓。
   **实测** ✓：对用户指的那轮 ⇒ "失败 job 数 = **3**" ⇒ **它会立刻判红** ✓✓。
3. **清队列** ✓：取消 3 个**排队中**的 run（`#36151294668` ✓ `#36151088821` ✓ `#36150927359` ✓），
   保留最新一轮 ✓（含 `ledger` 修复 ✓）。

### 教训（对"我"的 ✓）
**监控要盯最细的可判单元** ✗：整轮状态是**聚合量** ✓，聚合会把"已经确定的失败"
**稀释**成"还在跑" ✗。**这一条与判据设计同源** ✓：**别用聚合信号判断局部状态** ✗。

## ✅ **注解链路打通**（round 199 ✓）—— `ledger (2)` 的**真凶已指名** ✓
**监控在整轮未结束时**（15:36 ✓，整轮 15:31 起 ✓）就报出 ✓，并且注解直接给出缺口 ✓：
```
[failure] 缺口台账 G-23 与台账不一致
  G-23  fixed  script  环境异常  ← 台账写的是「复现件自己说环境/形状不对（exit 2）——修环境，别当成已修」
  ｜复现件：at process.processTimers (node:internal/timers:519:7)
  ｜ stderr: at listOnTimeout (node:internal/timers:581:17) / at process.processTimers (…)
```
**⇒ 根因** ✓：`G-23` 的复现件在 CI 上撞了 **Node 自己的定时器超时** ✗
（`listOnTimeout` / `processTimers` ✓ = Node 内部栈 ✓，**不是**我们的 `SOKO_GAP_REPRO_TIMEOUT` ✗）
⇒ 退出码 2 ⇒ `judge` 判"**环境异常**" ✓ ⇒ 与台账的 `fixed` 不一致 ⇒ 整片红 ✗。
**这正是 round 126 修过的同一族假红** ✓ —— 但那次修的是**我们自己的超时** ✓（响亮跳过 ✓），
而这一条走的是**复现件内部的 Node 超时** ✗ ⇒ 两条路都要覆盖 ✓。
**修法（下轮一步 ✓）**：① 给 `G-23` 的复现件**加长 Node 侧超时** ✓（或让它对慢机器更宽容 ✓）；
② 并把"`exit 2` + 栈里含 `listOnTimeout`/`processTimers`" 也**分档为环境异常** ✓
（与超时同款：**响亮跳过** ✓ + `--strict` 才判红 ✓）——**但必须配本地兜底判据** ✓，
否则会掩盖真回归 ✗（与 round 126 同一个取舍 ✓）。

**顺带确认两件事** ✓：`ledger (1)` ✅ 与 `ledger (3)` ✅ **都转绿了** ✓（原来三片全红 ✗）
⇒ **构建 + Node 两处修复都生效** ✓；`gates-fast` **2 分 09 秒** ✓（原来 3 分钟+ ✓）。

### round 201：`G-23` 假红的修复（**A 已落 ✓，B 留锚点 ⏳**）
**A ✅ 主修复** ✓：`G23-notation-navigation.js` 的 LSP 应答超时 **60 s ⇒ 180 s** ✓
（并支持 `SOKO_LSP_REPLY_TIMEOUT_MS` 覆盖 ✓）—— 这是 CI 慢 runner 上**直接**的成因 ✓。
**B ⏳ 分档（下一轮一步 ✓）**：把"`exit 2` + Node 定时器栈"归到 `timeout` 档 ✓
（响亮跳过 ✓，`--strict` 才红 ✓；**其余 `exit 2` 仍判红** ✓ ⇒ 2026-09-23 的保护不动 ✓）。
**锚点（精确到行 ✓）**：`scripts/gap.py` 的 `run_repro` 里，`.sh` 分支的
```python
           detail = _tail(proc.stdout) or _tail(proc.stderr)
           if detail and _tail(proc.stderr):
               detail = f"{detail} ｜ stderr: {_tail(proc.stderr, 2, 120)}"
           return ("script", proc.returncode, detail)
```
⇒ 在 `return` 之前插入：
```python
           if proc.returncode == 2 and any(
               k in ((proc.stderr or "") + (proc.stdout or ""))
               for k in ("listOnTimeout", "processTimers", "LSP 超时未应答")
           ):
               return ("timeout", proc.returncode, "LSP 应答超时（Node 自身定时器）⇒ 环境慢 ｜" + detail)
```
⚠ **我第一版的两处错（记下来 ✓）**：① 判断放进了 `judge()` ✗ —— 它只拿到调用方拼好的
`note` ✓、**拿不到 stderr 原件** ✗ ⇒ 永不触发 ✓；② 返回值写成**三元组** ✗（`judge()` 回两元 ✓）。
⇒ **判据要放在"原件在手"的那一层** ✓。

**本地复现判据（已验证可用 ✓✓）**：用新加的环境变量把 CI 的慢条件**造出来** ✓：
```bash
SOKO_LSP_REPLY_TIMEOUT_MS=1 bash docs/gaps/repro/G23-notation-navigation.sh   # ⇒ exit 2 + Node 栈 ✓
SOKO_LSP_REPLY_TIMEOUT_MS=1 python3 scripts/gap.py check --shard 2/3           # 期望 0（A+B 都上之后 ✓）
SOKO_LSP_REPLY_TIMEOUT_MS=1 python3 scripts/gap.py check --shard 2/3 --strict  # 期望 1 ✓
```
（**只上 A 时**：默认仍 1 ✗（因为 180 s 在 `SOKO_LSP_REPLY_TIMEOUT_MS=1` 下照样超 ✓）；
上完 B 后默认应转 **0** ✓ ⇒ 这就是 B 的判据 ✓。）

## 2026-09-25 · `0.72.0` 发版连续红三次 —— **三条教训**（都值得记住 ✓）

### ① bump 有两个隐藏依赖：`Cargo.lock` 与两个清单的 `requires`

**症状** ✓：CI 的 `gates-fast` 红在 `cargo build -q -p sokonanoda-cli --locked` ✓：
```
error: cannot update the lock file …/Cargo.lock because --locked was passed to prevent this
```
**原因** ✓：只改了 `Cargo.toml`（0.68.0 → 0.72.0）✗，而 **`Cargo.lock` 还记着 0.68.0** ✗
⇒ `--locked` 拒绝 ✓。
**修** ✓：`cargo build -q -p sokonanoda-cli --offline`（**不带 `--locked`** ✓）⇒ 锁文件跟着更新 ✓。

**而下一红又是同一类** ✗：`contract` 的第一步 `python3 scripts/bump.py --check` ✓ 报
**版本漂移** ✓：`course/shared/sokonanoda.toml` 与 `courses/set-theory/sokonanoda.toml`
的 `requires` 还是 `0.68.0` ✗。

**⇒ 根因（`vscode-dev-guide.md` 早就写了 ✓）**：
> **bump 用脚本，别手改**：`python3 scripts/bump.py <x.y.z>` 一次写全…
> **手改漏掉清单的 `requires` 就是 G-24 的成因。**

**⇒ 规程** ✓：**bump 一律 `python3 scripts/bump.py <x.y.z>`** ✓ ⇒ **然后 `--check` 复检** ✓
（它会打印"版本一致：x.y.z" ✓）。**`CHANGELOG.md` 仍然手写** ✓（脚本只管数字 ✓）。
**bump 其实是五处** ✓：`Cargo.toml` · `Cargo.lock` · `editor/vscode/package.json` ·
**两个 `sokonanoda.toml` 的 `requires`** ✓ —— **"两处"是脚本替你写全之后的表象** ✗。

### ② `auto-tag` 依赖**全部重活**，而 docs-only 轮把重活全 skip

```
auto-tag ✓：needs = [lint-fmt, lint-clippy, test, gates-fast, gates-course,
                    ledger, contract, editor, e2e, e2e-macos] ✓
```
⇒ **依赖被 skip ⇒ `auto-tag` 自己也 skip** ✓（GitHub 语义 ✓）
⇒ **纯 docs/toml 的 push 永远不会发版** ✓ —— 这是**第 g1 条的设计后果** ✓，**不是 bug** ✓，
但**发版那一次必须让重活真的跑** ✓（**即：那一次必须碰到 rust/courses 相关文件** ✓）。

### ③ **最贵的一条**：修 CI 的节奏与发版的节奏**相反** ✗

- **修 CI 时** ✓：每改一处就推 ✓ ⇒ `cancel-in-progress` **帮我省时间** ✓（掐掉旧轮 ✓）；
- **发版时** ✗：**每一推都掐掉唯一那轮 `rust == true` 的运行** ✓ ⇒
  **`auto-tag` 永远等不到"重活全绿"** ✓ ⇒ **release 永不触发** ✓。

**⇒ 规程** ✓：**发版窗口里，推一次就停手** ✓ —— 等它跑完（**别再推，哪怕发现小错** ✗；
要改就**攒着** ✓，下一批再说 ✓）。**`gh run rerun <id>` 只能重放同一棵树** ✗
⇒ **树里没有全部修复时，重跑一万次也没用** ✓（实测 ✓：重跑 `b4aca6e` ⇒ `contract` 照红 ✓）。
**⇒ 判据** ✓：发版前先 `git log --oneline origin/main..HEAD` 看清"这一推带上了什么" ✓，
再 `python3 scripts/bump.py --check` 与 `cargo build --locked` 两条本地门 ✓ ⇒ **然后才推** ✓。

## 2026-09-25 · 发版被两条**从来就红**的腿挡住 —— 而它们红了约 100 轮 ✗

**症状** ✓：`0.72.0` 的 bump 推上去后，`test` 矩阵里**三条腿红** ✗：
```
test (sokonanoda-front, doc)   → error: unknown start of token: \u{ff1a}   （全角冒号 ✗）
test (sokonanoda-cli, doc)     → error: no library targets found in package `sokonanoda-cli`
test (sokonanoda-cli, lib)     → 同上
```
**根因（两条，都不是本次改动造成的 ✗）**：
1. **`crates/front/src/judge.rs:442` 的围栏代码块没有语言标记** ✗ ——
   ` ``` ` 开头 ⇒ **rustdoc 当成 Rust 代码编译** ✓ ⇒ 而内容是**中文 + 全角冒号** ✗
   ⇒ 解析错 ✓。**修法** ✓：加语言标记（` ```text ` ✓）。
   **它从 round 233 起就红** ✗（约 **100 轮** ✓），**每一次发版都被它挡住** ✓。
2. **`sokonanoda-cli` 是纯 bin crate** ✗ ⇒ 矩阵里的 `--lib` / `--doc` 两条腿
   **永远 `no library targets`** ✗。**修法** ✓：`matrix.exclude` 删掉这两条
   （**12 条腿 → 10 条** ✓）。

**⇒ 规程（这一条最贵 ✓）**：**`auto-tag` 要的是"全绿"，不是"这次改的部分绿"** ✗
⇒ **发版会把所有旧账翻出来** ✓ —— 而**旧账可能已经红了几十上百轮** ✓，
**因为平时的 push 里它们是 skip 的** ✓（docs-only ✓）或**没人看** ✗。
⇒ **发版前先看 `test` 矩阵的**最近一次全量**结果** ✓（不是本次改动的结果 ✓）。
**⇒ 而诊断一条红腿，最省的办法是**本地跑 CI 的原命令** ✓**：
```bash
cargo test -p sokonanoda-front --doc --locked    # 失败输出第一行就写着文件名与行号 ✓
```
（**不要猜是谁改的** ✗ —— `unknown start of token: \u{ff1a}` 已经说了是**全角冒号** ✓。）

**⚠ 另一条** ✓：**`yaml.dump` 的输出不能当锚点** ✗ —— 它**重新缩进** ✓，
而文件里可能是**行内写法**（`kind: [lib, tests, doc]` ✓）⇒ **锚点只认"刚打印出来的文件原文"** ✓。

## 2026-09-25 · 新 run 长时间 `pending` 且 **0 个 job** ⇒ 并发组被**旧 run**占着 ✗

**症状** ✓：推上去后 `gh run view <新 id>` 一直 `pending/` ✓、`job 数 0` ✗ ——
而**前面的 run 都是秒起** ✓。
**根因** ✓：顶层 `concurrency: {group: ci-${{ github.ref }}, cancel-in-progress: true}` ✓
⇒ **同一组里有一个 `in_progress` 的旧 run** ✗（它**注定红** ✗ —— 树里没有修复 ✓）
⇒ **新 run 排队等它** ✓。`cancel-in-progress` 只在**新 run 真正开始时**才掐旧的 ✓，
而**它自己还没开始** ✗ ⇒ **死等** ✓。
**修** ✓：`gh run cancel <旧 id>` ✓ ⇒ **新 run 立刻起** ✓（实测 ✓：`job 数 10` ✓）。

**⇒ 规程** ✓：**新 run 长时间 `pending` + 0 job ⇒ 先查同组的旧 run** ✓
（`gh run list --limit 5` ✓）⇒ **取消注定失败的那个** ✓ ⇒ **别再推** ✗
（**推解决不了排队** ✗ —— 它只会再加一个排队的 ✓）。

## 2026-09-25 · 整轮 **success 却什么都没跑** —— `paths-filter` 看的是"**这一推的 diff**" ✗

**症状** ✓：`0.72.0` 的发版推之后，整轮 **`success`** ✓ 而**只有 3 个 job** ✗
（`changes` + `lint-fmt` + `lint-clippy` ✓）⇒ **重活与 `auto-tag` 全 `skipped`** ✗ ⇒ **没发版** ✓。
**日志原文** ✓（`changes` job ✓）：
```
Run dorny/paths-filter@v4
  **Matching files: none** ✗
  **Changes output set to []** ✗
```
**根因** ✓：**`paths-filter` 只比"这一推的 `before..after`"** ✓ ——
**不是"仓库里有什么"** ✗、**也不是"前几推带了什么"** ✗。
⇒ 我那次推的 diff **只有 `STATUS.md`** ✓（**rust 改动在**上一推**里** ✗）⇒ **过滤器报"无 rust"是对的** ✓。

**⇒ 规程（这是"发版推"的硬条件 ✓）**：**发版那一推必须自带过滤器认的路径** ✓：
```
crates/** · Cargo.toml · Cargo.lock · scripts/** · .github/workflows/**   ← rust ✓
editor/** ← editor ✓    courses/** · playground.sokonanoda ← courses ✓
```
⇒ **而"改 `.github/workflows/**` 也算 rust"** ✓ 是一条**很有用**的性质 ✓：
**修 CI 的推会自动触发重活** ✓ ⇒ **不用为了触发而造改动** ✓（**真实待办自己就是触发器** ✓）。

**⚠ 而一个"看起来很像"的错误结论** ✗：我一度推断是 **`git pull --rebase` 让 `before` 不可达** ✗
⇒ **错了** ✓（**快进推也一样** ✓）⇒ **真因是 diff 内容** ✓。
**⇒ 教训** ✓：**"这推带了什么"和"仓库里有什么"是两个问题** ✗ ——
**判据要问对**：`git diff --name-only origin/main...HEAD` ✓（**这一推的 diff** ✓）。

## 2026-09-25 · **`test` 提速：`nextest` 决策 = 不做** ✓（数字在此，不必再议 ✓）

**症状**：`test` 是全轮最长的杆 ⇒ 直觉是换 `cargo nextest` 做测试级并行。
**先读 `ci.yml:148-153`** —— 仓库早就写着不用它（两条理由都成立）：① **它不跑 doctest**，
而 `test` 矩阵有 `doc` 腿；② **输出格式不同**，而失败注解那步
`grep -E "^test .* FAILED$"` **依赖 libtest 的格式**。

**测量**：① `test` 已按 `pkg × kind` **10 片并行** ⇒ 最长 **12 分 13 秒**（不是记忆里的 19–37 分钟）；
② 瓶颈是**执行**还是**编译**：CI 上一个 161 测试的套件 **278.58s**，而**编译只 26s**；
③ 本机串行基线同一套件 **37.37s** ⇒ **机器差 7.5×**（CI 2 核 vs 本机多核 —— 与 `perf-gate`
的跨机器问题**同源**）。⇒ `nextest` 在 2 核上并行度最多 ~2 ⇒ **278.58s → ~140s**（省 ~2.3 分钟）
⇒ **低于预先判据「>3 分钟才做」** ⇒ **不做** ✓。

**⇒ 真正的杠杆**：278s 里 ~250s 是**编译** ⇒ 要提速就动**编译缓存**
（`Swatinem/rust-cache@v2` 已在）⇒ **下一步是看它的命中率**，而不是换测试跑法 ✓。

**⇒ 规程（改 `test` 之前先量三件事）**：① 实测耗时（别用记忆）；② 瓶颈在编译还是执行；
③ **仓库有没有否决过这个方案**（`ci.yml` 注释里写着）。
另加一条前置检查：**共享资源** —— `crates/lsp/src/tests/**` 的 161 个测试没有端口/临时目录/
全局单例 ⇒ 技术上可并行，但**收益要先量、代价（改注解）要先认** ✓。

## 2026-09-25 · **轮询不是工作** ✗ —— 等 CI 必须落成机制（用户点名 ✓）

**症状** ✓（**已核实 ✓**）：第 **450–455 轮**（至少 6 轮）的输出**逐字相同** ✗：
```
不变 —— 14 绿 · 11 未完 · 零失败 ⇒ 判据不变：报无基线则勾 T-E1，不推
```
⇒ **每轮只为确认"CI 还没变"** ✗ ⇒ **把整个上下文重新喂一遍模型** ✗
⇒ ⇒ **纯烧 token、纯占轮次预算** ✗（**cap 480，当时已 455+** ✗）。

**病根** ✓：**把"等"当成了"查"** ✗ —— 而**"查"要开一轮、要喂上下文** ✗；
**"等"应该由外部进程做** ✓。

**⇒ 三条机制（等 CI 一律用其中之一 ✓，禁止"每轮查一次" ✗）**：
1. **异步兑现** ✓（首选）：推完**继续做不依赖 CI 结论的工作** ✓ ——
   结论回来后**一次性校正** ✓，而不是等在那里 ✓；
2. **一轮顶完** ✓：必须等时用 **`gh run watch <id> --exit-status`** ✓（**阻塞到整轮结束** ✓）
   或单次带超时的阻塞轮询 ⇒ **一轮顶完整个等待** ✓；
3. **外部作业** ✓：用**后台作业 / 定时唤醒**等 CI 完成 ✓ ⇒ **完成后才开一轮收结论** ✓。

**⇒ 纪律** ✓：**轮次与 token 是预算，轮询不是工作** ✓ ——
**每轮必须有可验证产出**（**代码 / 判据 / 数字 / 台账** ✓），**否则不要开这一轮** ✗。

**⇒ 验收判据（可验证 ✓）**：**等 CI 的这段时间内，模型调用次数 = 0 或 1，而不是 N** ✓。
实现 ✓（本轮 ✓）：
```bash
# 外部脚本去等 + 收结论 ⇒ 模型不参与轮询
bash /tmp/ci-wait-and-collect.sh    # gh run watch --exit-status && 读 perf-gate 日志 → /tmp/ci-wait.log
```
⇒ **它跑完才通知模型** ✓ ⇒ **等待期间 0 次模型调用** ✓✓。

**⚠ 反面教材就在本仓库** ✗：`STATUS.md` 的 round 450–461 ✓（**12 轮** ✗）
**全是"不变"** ✗ ⇒ **它们本可以是 0 轮** ✓。

## 2026-09-26 · **整个 workflow 被 GitHub 拒绝：同一 step 里两个 `run:` 键** ✗

**症状** ✓：推上去的那一轮 CI **`completed/failure`、`jobs=0`、耗时 `0s`** ✗
—— **不是测试红，是 workflow 文件没通过校验** ✓。

**根因** ✓（**我的错** ✗）：我给 `ci.yml` 加"STATUS.md 瘦身 lint"那一步时，
把新的 `run:` **写进了上一个 step 的映射里** ✗ ⇒ 同一个 step 出现**两个 `run:` 键** ✗：
```yaml
      - name: 课程记法规则
        shell: bash
        run: python3 "$GITHUB_WORKSPACE/scripts/notation-lint.py"
        run: python3 "$GITHUB_WORKSPACE/scripts/status-lint.py"   # ← 同一个 step ✗
```

**为什么我没发现** ✓✓（**这条最贵** ✗）：我用 `python3 -c "yaml.safe_load(...)"` 验过 ✗ ——
**而 `yaml.safe_load` 对重复键不报错** ✗，**它静默取最后一个** ✗
⇒ 我甚至打印出"`status-lint` 出现 1 次 ✓" ✗ —— **那正是它把 `notation-lint` 顶掉了** ✗✓。

**⇒ 两个后果，第二个更糟** ✗：
1. GitHub 拒绝整个 workflow ⇒ 0 job、0 秒失败 ✗（**响亮** ✓，所以发现了 ✓）；
2. **如果 GitHub 接受它，`notation-lint` 会被悄悄关掉** ✗ ——
   **一条既有的课程门禁消失** ✗，而**本地看起来全绿** ✗✓。

**修复** ✓：拆成**独立的 step**（各自的 `- name:` ✓）。

**⇒ 新增守卫** ✓：**`scripts/ci-yml-lint.py`** ✓ —— 用**禁止重复键的严格 loader** ✓
（`yaml.safe_load` 不够用 ✗），并检查每个 job 有 steps、每步有 `run` 或 `uses` ✓。
已接进 **`scripts/ci-local.sh` 的 ⓪ 号阶段**（**最前面** ✓ —— 它是唯一能在 push 前拦住的地方 ✓）
与 **`ci.yml`** ✓。

**反向验证** ✓（**咬得住** ✓）：把重复键塞回去 ⇒
`ci-yml-lint：1 条不通过 ✗ - :518:9 YAML 不合法 ✗：重复键 \`run\`` ✓（**带精确位置** ✓）。

**预防** ✓：**改 workflow 之后，验证命令只能是 `python3 scripts/ci-yml-lint.py`** ✓ ——
**不要再用 `yaml.safe_load` 当门** ✗（**它连重复键都不报** ✗）。

## 2026-09-26 · **`status-lint` 挂在过滤过的 job 里 ⇒ 只改 `STATUS.md` 时它根本不跑** ✗

**症状** ✓：我把 `status-lint.py` 接进 `gates-fast` ✗ ⇒ 而 `gates-fast` 由 **`rust` 过滤器**驱动 ✓
⇒ **只改 `STATUS.md` 的推送不碰 rust 路径** ✗ ⇒ **`gates-fast` 被 `skipped`** ✗
⇒ **lint 永远不跑** ✗（**实测** ✓：`675491b` 只改 `docs/` + `STATUS.md` ⇒
那一轮 **15 个 job 里只有 3 个绿、其余全 `skipped`** ✓，含 `gates-fast` ✓）。

**⇒ 为什么这条最讽刺** ✗✓：**"只改 `STATUS.md`"恰恰是最常发生的情况** ✓
—— 每轮收尾都要改它 ✓ ⇒ **我加的那条守卫，在最需要它的场景下不跑** ✗
（**"咬不住的守卫等于没有"** ✓ 的又一个变体：**"不跑的守卫等于没有"** ✗）。

**修复** ✓：**独立的 `status-lint` job** ✓，**不设 `if:`、不加过滤器** ✓ ——
它只要 **~1 秒** ✓，所以**永远跑** ✓（**顺带避开"skipped 的依赖会拖垮 `auto-tag`"** ✗，
见 `changes` job 的注释 ✓）。同时**从 `gates-fast` 里撤掉那一步** ✓（**避免两处重复** ✓）。

**⇒ 顺带暴露的第二条** ✓：**`675491b` 那一轮 `success` 是"什么都没跑"的 success** ✗
（**3/15 绿、其余 skipped** ✓）—— 与 2026-09-25 记的那条**同一个形态** ✓：
**`paths-filter` 看的是"这一推的 before..after"** ✗，**不是仓库里有什么** ✗。

**⇒ 第三条（我自己又犯的）** ✗：**在 run 在飞的时候又推了一次** ✗
⇒ 顶层 `concurrency: cancel-in-progress: true` **把上一轮掐了** ✗
（`36207207425` ⇒ **`cancelled`** ✓，**6/27 绿** ✓）⇒ ⇒
**于是"验证 ci.yml 修复"的那一轮永远没跑完** ✗ ——
**要验证 `.github/**` 的改动，就得让那一推带 `.github/**` 路径** ✓，
**而且推完要等它跑完再推下一次** ✓。

## 2026-09-26 · **新 e2e 测试污染全局缓存 ⇒ 撞红一条既有测试**（只在 CI 复现 ✗）

**症状** ✓：CI 的 e2e 报 **`27 passed / 1 failed`** ✗，而红的**不是**我新加的那条 ✓，
是**既有的** `rewriting an unchanged project unit does not recompile` ✗：
```
AssertionError: 内容没变 ⇒ 不许写出新的缓存条目（说明闭包被重编了）
+   '5d5b302d9181a8e1.json:11257:1790385365430.0708',   ← 多出来的一条
```

**根因** ✓（**我的错** ✗）：我给 T-U12 面 #3 加的 hover 测试，
**把夹具写进了 `tmpDir`（新建文件）** ✗ ⇒ 那个文件被 LSP 编译 ⇒
**写进全局缓存** ✓（`cacheStamp()` 读的正是**全局缓存 + 工作区缓存** ✓）
⇒ 在**慢速 CI runner** 上这次落盘**迟到** ✗ ⇒ 正好落进后面那条缓存测试的
**1.5 秒窗口** ✓ ⇒ 它的 `cacheStamp()` 多出一条 ⇒ **判红** ✗。

**⇒ 为什么本地一直是绿的** ✗✓：**本机快** ⇒ 那个落盘在窗口之前就完成了 ✓
⇒ ⇒ **这是"本地绿、CI 红"的教科书形态** ✓ ——
**而它骗过我的方式很隐蔽**：我以为"新增测试"是纯增量 ✗，
**没想到它会给后面的测试留下副作用** ✗。

**⇒ 怎么发现的** ✓（**判据而不是猜** ✓）：
1. 先问"**这条测试历史上红过吗**" ✓ ⇒ 扫 **169 个历史 e2e 日志** ⇒ **一次都没有** ✗
   ⇒ **排除 flaky、锁定是我弄的** ✓；
2. 再问"**多出来的那条缓存是谁写的**" ✓ ⇒ 我那条测试是**唯一新建文件**的 ✓
   ⇒ **锁定** ✓。

**修复** ✓：**改成复用现有夹具、绝不新建文件** ✓ ——
夹具里本来就有 `lib/Set.sokonanoda` 的 `infix:50 " ⊆ " => Set.subset` ✓
与 `units/u01.sokonanoda:11` 的 `subset_mem`（**类型就是 `A ⊆ B -> A ⊆ B`** ✓）
⇒ 直接 hover 它 ✓（**零新文件 ⇒ 零缓存副作用** ✓）。

**⇒ 纪律** ✓：**e2e 里新增一个"编译单元"就是新增一条全局缓存条目** ✗ ——
`cacheStamp()` 把**全局缓存**算进指纹 ✓ ⇒ **任何新建文件的测试都会动它** ✗
⇒ **优先复用夹具** ✓；确实必须新建时，**要在测试末尾等它落盘** ✓
（或把新文件放进工作区，让它成为闭包的一部分 ✓）。

## 2026-09-26 · **改 `editor/**` 永远不会跑真宿主 e2e：skipped 的依赖把它拖走了** ✗

**症状** ✓：`afceda9`（只改 `editor/vscode/src/test/extension.test.js` + docs）那一轮
**`success`** ✓，但 **5/16 绿、11 skipped** ✗ —— **三条 e2e 全 skipped** ✗
（而 `editor` job **success** ✓）。⇒ ⇒ **"success" 是"什么都没验"的 success** ✗。

**根因** ✓：`e2e` 的条件**本来是对的** ✓：
```yaml
    if: needs.changes.outputs.rust == 'true' || needs.changes.outputs.editor == 'true'
```
**但它还写着** ✗：`needs: [changes, lint-fmt, lint-clippy, gates-fast]`
—— 而 **`gates-fast` 的条件是 `rust == 'true'`** ✗ ⇒ **editor-only 推送时它 `skipped`** ✗
⇒ ⇒ **skipped 的依赖把 `e2e` 也拖成 `skipped`** ✗✓（**仓库里记过的那个坑，这次撞在自己身上** ✗）。

**⇒ 这是一个真洞** ✗：**改 `editor/vscode/**` 时，真宿主 e2e 一次都不跑** ✗ ——
而 `if:` 里写着 `|| editor == 'true'` ✓ ⇒ **本意是要跑的** ✓，**是 `needs` 列表把它废掉了** ✗。

**修复** ✓（**两条 e2e 一起** ✓）：**`!cancelled()` + 逐条"依赖没失败"** ✓
```yaml
    if: >-
      !cancelled()
      && (needs.changes.outputs.rust == 'true' || needs.changes.outputs.editor == 'true')
      && needs.lint-fmt.result != 'failure'
      && needs.lint-clippy.result != 'failure'
      && needs.gates-fast.result != 'failure'
```
⇒ **保留原意** ✓（**便宜门禁红了就别跑贵的** ✓），**同时让 skipped 的依赖不再阻塞** ✓。

**⚠ 过程中守卫又咬了一次** ✓：我第一次改用的锚点是那行 `if:` 本身 ✗ ——
而**它和 `editor` job 的那行一字不差** ✗ ⇒ **`assert count == 1` 拦住了** ✓
（**否则会误改 `editor`** ✗）⇒ **改成带 `name:` 的锚点** ✓。
⇒ **这就是"锚点必须先验唯一性"的价值** ✓（**`AGENTS.md` 里那条** ✓）。

**预防** ✓：**新增/修改 `needs` 时，先问"它会不会在本次路径下被 skip"** ✗ ——
**skipped 的依赖会让下游静默消失** ✗，而**整轮还报 `success`** ✗。


## 2026-09-28 · **`gates-course` 顶到 20 分钟上限**（课程涨到 375 checked ✗✓）

run `36373825237`：job failure，末行 `##[error]… has timed out after 20 minutes`。
**判据是对的** ✓ —— 超时前门禁已跑完并打出 `43 个目标 —— 375 checked · 99 open · 0 个被判负`
（`--selftest` 也 PASS）⇒ **不是判据坏、不是课程坏，是上限顶死** ✗。

**根因**：门禁对**每个目标**跑一次 `grade <绝对路径> --json`，每次把该目标的 **import 闭包
冷编译**一遍 ⇒ 成本**随课程规模线性涨**。v0.77.0 新增 6 个 `lib` 模块 ⇒ 331 → 375 checked。
实测冷跑：**本机（M 系 mac，release）5 分 05 秒** / **CI（2 核 ubuntu）19 分 22 秒**（≈3.8×）。
⚠ 3.8× **不是噪声** —— 与 `docs/PERF.md` 记的同一现象一致（那次 7.5×）⇒ **是机器差** ✓。

**修复**：`timeout-minutes: 20 → 40`（与 `ledger` 的 30 分钟同族），实测数字写进 `ci.yml` 注释。
⚠ **不是"为绿色改判据"** —— G1–G5 一条没动、课程难度没降、没有 skip，改的只是给多少时间 ✓。

**预防 / 后续**：① **课程规模涨 ⇒ 先量冷跑**（`rm -rf courses/set-theory/.sokonanoda/compiled`
再跑一次），别等 CI 通知；② **真优化**（不阻塞 0.77.0）：给 `<模块根>/.sokonanoda/compiled/`
加 `actions/cache` —— 本机**热缓存 0.5 秒**（59 MB），但引入"缓存失效/污染"新失败模式 ⇒ 留 0.78。

## 2026-09-28 · **绝对毫秒判据跨机不可转移**（`keystroke_recompile_closure` ✗✓）

run `36378945287` 的 `test (sokonanoda-front, tests)` 判红：`best 1599.6ms · worst 2430.9ms`
（判据 `worst < 2000.0`）；同 job 其余 756 条全绿。
**实测直方图**（**同一份 `crates/` 代码**，`git diff 6a814c0 HEAD -- crates/` 为空）：

| 环境 | worst |
|---|---|
| 本机（8 核满载施压仍只） | 44–71ms（**59.6ms**） |
| CI `36362709263`（ST2） | 240.11ms ✓ |
| CI `36373825237`（ST15） | 343.47ms ✓ |
| CI `36378945287` attempt 1 | **2430.91ms** ✗ |
| 同上 attempt 2（**同 job 重跑**） | **510.66ms** ✓ |

⇒ **同代码同 job 差 4.8×、跨机差 55×** ⇒ 判据在**量机器**不在量代码。
⚠ **另有一条真趋势**：**240ms（ST2 前）→ 344ms（ST2 后）** —— ST2 给 prelude 加 5 条 `Quot`
公理 ⇒ 每块多 5 条声明 ⇒ 冷编译变贵（**与 `gates-course` 顶 20 分钟上限同源**）。

**修复**：判据换成**同机比值** `worst / cold_ref < 8.0`（`cold_ref` = 同一次测试做 3 次冷编译
取最快；实测 **0.60~1.65**）+ **很宽的绝对天花板** 30000ms（挡"两边一起变慢"）。
**反向验证**：阈值压到 0.1 ⇒ 比值那条断言**先咬住** ✓。全量 **1384 passed · 0 failed** ✓。

**预防**：**性能判据不许用绝对毫秒** —— 跨机不可转移（先例：`perf-gate` 的跨机器事故、
`docs/PERF.md` 的 7.5×）。要归一化就取**同机参考量**（比值 + 宽天花板）；
**别**把阈值调大了事 —— 那既掩盖真回归，也仍会被更吵的机器咬到。

## 2026-09-28：**判据命令被管道掩膜 ⇒ 误报绿**（本地事故，非 CI 红；同类第二次，故立此条）

**现象**：本 session 多轮用 `python3 scripts/docs-lint.py | tail -1` 判断"文档预算 ✓"，
而 `docs-lint` 的**不通过明细走 stderr**、只有汇总行走 stdout ⇒ `tail -1` 永远截到那句
"✓ 活文档 …"，于是**红了也看成绿的**（实测：同一棵树 `2>&1` 全量读 = **9 条不通过 ✗**）。

**根因**：把"命令退出码/完整输出"换成"管道的最后一行"——正是本仓库反复警告的掩膜。

**修复**：
1. 判据一律 **`2>&1` 全量读 + 按退出码判**（`cmd; echo exit=$?`），**禁止 `| tail`/`| grep` 决定绿红**；
2. 本 session 的真实触发还叠加了一条：工作区里有**非本次改动**的旧版本文档被换回
   （`REQUIREMENTS.md` 240 → 940 行 · `docs/LESSONS.md` 556 → 2756 行，共 4 文件 3370 行 diff）
   ⇒ 撑爆 `docs-lint` ②③④⑦ ⇒ pre-push 两次按设计拒推 ✓。已留档 `/tmp/rescue/uncommitted.patch`
   并 `git checkout --` 还原（来源未定，最可能是被中断的子代理/后台脚本把旧版本文档写回工作区
   ⇒ **动手前后各看一眼 `git status --short`**）。

**预防**：① 判据读全量输出（上）；② **推送前先 `git status --short` 确认没有"不是我做的"改动**；
③ 门禁红了先读**明细行**（哪一条、哪个文件、哪个数字），再决定改什么。

## 2026-09-29 · 切片 1b 接线：**"变快"其实是"失败得快"**（本机，非 CI）

**症状**：`build --clean courses/set-theory` + `build --json` 从基线 **222.1s** 掉到 **15s** ——
看起来像 15× 提速。**真相**：`build.summary` 是 `{'compiled': 1, 'failed': 41}` ✗ ——
41 个入口**快速失败**，所以"快"。**已回滚**（`git revert`），回滚后冷全量实测
`{'compiled': 42, 'failed': 0}` ✓（墙钟 **625s**，见下条）。

**根因（假设）**：session 的库层是**跨入口去重的并集**（按首个 plan 的闭包顺序编），
而 `assemble_report` 按**该入口自己的闭包顺序**用 `unit_ranges` 切分 ⇒ 命令区间错位
⇒ 事件/错误归到错的模块 ⇒ 大面积 `failed`。

**四条教训（都已变成动作）**：
1. **性能数字必须先看 `failed`/`compiled` 计数** —— "快"可能是"错得快"✗；
2. **revert ≠ rebuild**：`git revert` 后用**旧二进制**复测会得出假结论（本次差点据此误判
   根因不在接线）⇒ 回滚后**必须重建**再测；
3. **咬不住的守卫等于没有**：当时 `cargo test -p sokonanoda-cli --test imports` 的 **21 项全绿** ✗
   ⇒ 已补 **`build <dir>` 多入口守卫**（`build_a_directory_of_entries_sharing_a_dependency_compiles_them_all`，
   断言 `failed:0` + `compiled:4`）并做**反向验证**：接线版 **1 failed**（红的正是它）、
   正确代码 **22 passed** ✓；
4. **~~跨机比数字无效~~ ⇒ 更正为「跨接线版本比数字无效」**（2026-09-29 实测修正）：
   原写法「625s vs 222.1s ⇒ 跨机不可比」**被否定** ✗✓：同机同接线同缓存下 release
   **218.8s** vs debug **260.7s**（差 **3%**）⇒「debug 慢一个量级」是 **CLI 集成测试
   二进制**的现象，不是 `build` 的；**625s** 是**接线中间版本** ✗。⇒ 规矩：同机 + 同
   profile + 同接线版本；别拿"跨机"当解释（证据 `docs/perf/course-profile-2026-09-29.md`）。

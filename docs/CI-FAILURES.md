# CI 失败台账（**同一类失败不犯第二次** ✓）

> **规则**（`AGENTS.md` §CI 失败记录 ✓）：每次 CI 红了就**追加一条** ✓
> （原因 / 修复 / 预防 ✓）。**本文件只留最近 15 条** ✓（用户 2026-09-26 文档瘦身要求）；
> 更早的 53 条 ⇒ `docs/archive/ci-failures-2026-09-10-to-2026-09-25.md.gz`（gzip ✓，**归档 ≠ 销毁** ✓）。
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

## 2026-09-25 · **想给 `test` 提速？先读 `ci.yml:148-153`** —— `nextest` 被否决过 ✗

**症状** ✓：`test` job 是全轮最长的杆 ✓ ⇒ **直觉是"换 `cargo nextest` 做测试级并行"** ✓
⇒ **而仓库里早就写着"不用它"，并给了两条理由** ✓✓：
```
ci.yml:148-153 ✓：
  # 实测相当均衡（7m39s / 8m27s / 10m41s / 9m12s）=> 按 crate 再拆没有空间；
  # 而 --lib / --tests / --doc 是互不重叠的三块 => 拆开后最长杆从 ~10m41s 降到 ~4 分钟
  # ⚠ 不用 cargo nextest：它**不跑 doctest**，且输出格式会打断既有的失败注解。
```
**两条理由都成立** ✓：
1. **它不跑 doctest** ✓ —— 而 `test` 矩阵有 **`doc` 腿** ✓；
2. **输出格式不同** ✓ —— `Surface failing tests as annotations` 那一步
   **`grep -E "^test .* FAILED$"`** ✓（`ci.yml:202-207` ✓）**依赖 libtest 的格式** ✗。

**⇒ 规程** ✓：**改 `test` 之前先量三件事** ✓：
① **实测耗时** ✓（**别用记忆里的"19–37 分钟"** ✗ —— 实测 **10 条腿并行、最长 12分13秒** ✓）；
② **瓶颈在编译还是执行** ✓（**实测：一个 161 测试的套件 278.58s，而编译只 26s** ✓）；
③ **仓库有没有否决过这个方案** ✓（**`ci.yml` 的注释里写着** ✓）。

**⇒ 而"能不能并行"还有一个前置检查** ✓：**共享资源** ✓ ——
`crates/lsp/src/tests/**` 的 161 个测试**没有端口 / 临时目录 / 全局单例** ✓
⇒ **技术上可并行** ✓ ⇒ **但收益要先量、代价（改注解）要先认** ✓。

## 2026-09-25 · **`nextest` 决策：不做** ✓（**数字在此，不必再议** ✓）

**问题** ✓：`test` job 是全轮最长的杆 ⇒ 想换 `cargo nextest` 做测试级并行 ✓。

**测量（三步 ✓）**：
1. **`test` 已是 10 片并行** ✓（matrix `pkg × kind` ✓）⇒ **最长 12分13秒** ✓（**不是 19–37 分钟** ✗）；
2. **瓶颈是执行还是编译** ✓：CI 上一个 **161 测试的套件 278.58s** ✓，**而编译只 26s** ✓；
3. **本机串行基线** ✓：同一套件 **`finished in 37.37s`** ✓（`real 5m19s` 里 **~4m40s 是编译** ✓）。

**⇒ 机器差 7.5×** ✗✓：**CI 2 核 runner vs 本机多核** ✓（**与 `perf-gate` 的跨机器问题同源** ✓）。
⇒ **`nextest` 在 2 核上并行度最多 ~2** ✗ ⇒ **278.58s → ~140s** ✓ ⇒ **省 ~2.3 分钟** ✗
⇒ **低于预先判据 ">3 分钟才做"** ✓ ⇒ ⇒ **决定：不做** ✓。

**⇒ 而代价本来也在** ✓（**`ci.yml:152` 写着** ✓）：**它不跑 doctest** ✓ · **输出格式打断失败注解** ✗
（**注解那一步 `grep -E "^test .* FAILED$"` 依赖 libtest 格式** ✓）。

**⇒ 真正的杠杆** ✓：**278s 里 ~250s 是编译** ✗ ⇒ **要提速就动编译缓存** ✓
（**`Swatinem/rust-cache@v2` 已在** ✓ ⇒ **下一步是看它的命中率** ✓，**而不是换测试跑法** ✗）。

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

## 2026-09-26 · **skip 沿依赖链传播：`e2e-ledger` 在 editor-only 推送里被静默跳过** ✗

**症状** ✓：`6e19f23`（**只改 `editor/**`**）那一轮 **`success`** ✓，但
**三条 e2e 全 success** ✓ 而 **`e2e ledger (commit back on main)` 是 `skipped`、`steps=0`** ✗。

**先钉事实** ✓（**不推理** ✗）：
- `headSha=6e19f23` ✓（确认就是那一轮 ✓）；
- **三份 artifact 都在** ✓（含 `e2e-macos-latest-vscode-1.138.0` ✓）
  ⇒ **`e2e` 与 `e2e-macos` 都真跑了** ✓；
- `e2e` 矩阵**只有 2 条 ubuntu 腿** ✓（文件原文 ✓）；
- `e2e-ledger` 的 `if` **解析值**就是 `github.event_name == 'push' && github.ref == 'refs/heads/main'` ✓
  （**用 YAML 解析器看的** ✓，不是肉眼读的 ✗）；`needs: [e2e, e2e-macos]` ✓ 两条都 `success` ✓。

⇒ **按"只有 `needs` 里有 skipped 才拖垮下游"的直觉，它必须跑** ✗ —— **可它没跑** ✗。

**机制** ✓（**GitHub 文档** ✓）：
> a failure or skip applies to **all jobs in the dependency chain** from the point of failure or skip onwards
—— **是"链条"，不是"直接依赖"** ✗✓。`gates-fast` 被 skip ✗ ⇒ `e2e` 靠**它自己的**
`!cancelled()` 跑起来了 ✓，**但那个 skip 仍污染链条** ✗ ⇒ `e2e-ledger` 的 `if` 里
**没有状态函数** ⇒ **被跳过** ✗。

**实验** ✓（**唯一变量** ✓）：
| 推 | 路径 | `e2e-ledger` 的 `if` | 结果 |
|---|---|---|---|
| `6e19f23` | 只 `editor/**` | `push && main` | **skipped** ✗ |
| `34a2814` | 只 `editor/**`（+ 非 rust 路径 ✓） | `!cancelled() && push && main` | **success** ✓✓ |

⇒ **同样的 editor-only 场景，唯一的变化就是那个状态函数** ✓ ⇒ **机制确认** ✓。

**修复** ✓：`e2e-ledger` 的 `if` 加 `!cancelled()` ✓。

**⚠ 实验设计上我自己先踩的坑** ✗：第一版想一次推完（`ci.yml` + `editor/**`）✗ ——
而**改 `ci.yml` 会让 `rust=true`** ✗，`rust=true` 时 `e2e-ledger` **本来就会跑** ✓
⇒ **实验等于白做** ✗ ⇒ **拆成两推** ✓（先布条件、再复现 ✓）。

**⇒ 纪律** ✓：**凡 `needs` 里可能有 skipped 的 job，`if` 都要带状态函数** ✓
（`always()` / `!cancelled()` / `success()` / `failure()` ✓）——
**否则它会静默消失，而整轮还报 `success`** ✗。


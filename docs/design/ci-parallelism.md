# CI 并行化方案（2026-09-25 ✓，用户要求："github action 是最大瓶颈" ✓）

## 量出来的事实 ✓
`ci.yml` 现有 **12 个 job** ✓，其中 **9 个各自独立编译** ✗：

| job | 它自己付的构建 | 能不能共享 |
|---|---|---|
| `lint` | 🔨 **clippy 全量 check** | 否（就是它的工作 ✓） |
| `test ×4`（crate 矩阵 ✓） | 🔨 各自编译测试 | 否（**真并行** ✓，靠 `rust-cache` ✓） |
| `gates-fast` | 🔨 debug 构建 + `cargo test`（课程契约）+ **逐文件 `cargo run`** ✗ | **循环可省** ✓ |
| `gates-course` | 🔨 **release 构建** | **✅ 可下载现成的** ✓ |
| `ledger ×3` | 🔨 **release 构建 ×3** ✗✗ | **✅ 可下载** ✓（三片**都**在重复构建 ✗） |
| `contract` | 🔨 构建 | 部分 ✓ |
| `editor` | 🔨 构建 + 📦 扩展 | 部分 ✓ |
| `e2e ×3` + `e2e-macos` | 📦 扩展打包（+ LSP ✓） | **✅ 可下载** ✓ |
| `changes` · `auto-tag` · `e2e-ledger` | ⚡ 纯脚本 | — |

⇒ **结论** ✓：**"再拆"的边际收益小** ✗（拆只改**串行→并行** ✓，而这里已经是并行的 ✓）；
**真正的杠杆是"别让 6+ 个 job 重复付同一个构建"** ✓。

## 排序（按收益 / 风险 ✓）

### ① `build` 一次、其余下载 ✓（最大头 ✓）
新增 **`build`** job ✓：`cargo build --release -p sokonanoda-cli -p sokonanoda-lsp --locked`
（+ 需要时 `npm ci && npx vsce package` ✓）⇒ `actions/upload-artifact` ✓
⇒ `gates-course` ✓ `ledger ×3` ✓ `editor` ✓ `e2e ×3` ✓ `e2e-macos` ✓ 改 **download** ✓
⇒ 这些 job 从"几分钟"掉到"几十秒" ✓（**注意** ✓：`e2e` 本来就要**真 VS Code** ✓，
它下载的是**扩展包** ✓ ⇒ 与 `build` 的产物一致 ✓）。
**风险** ✓：artifact 的**路径与权限位**要保持可执行 ✗（本仓库已有 `download-artifact@v5` 的
布局坑 ✓ —— 见 `skills/sokonanoda-ci` ✓）；改动面大 ⇒ **分两次落地** ✓。

### ② `gates-fast` 里的**循环** ✗（便宜、立竿见影 ✓）
`Lesson corpus is valid` 对**每个** `examples/*.sokonanoda` 起一次 `cargo run` ✗
⇒ 改成**一次编译 + 批量评判** ✓（或把文件列表交给一次进程调用 ✓）
⇒ 去掉"每次启动 + 每次加载"的固定开销 ✓。

### ③ `lint` 拆 **`fmt` / `clippy`** ✓（**最快的红灯** ✓）
`fmt` 只需 **~10 秒** ✓ ⇒ 单独成 job ⇒ **10 秒内**给出格式红灯 ✓✓
（用户要的"早感知"✓ 在这里最直接 ✓）；`clippy` 保持自己那条 ✓。

### ④ 更多分片（可选 ✓）
`ledger` 已 3 片 ✓、`e2e` 已 3 平台矩阵 ✓ ⇒ 若仍慢 ✓，再按**用例**分片 ✗
（`SOKO_E2E_GREP` 已支持单用例 ✓）⇒ 收益递减 ✓ 且 e2e 的固定成本高 ✗ ⇒ **最后再考虑** ✓。

## 铁律（拆 job 时必须 ✓）
1. **`auto-tag.needs` 同步换代** ✗（本 session 吃过的亏 ✓）；
2. **全文件重复键扫描** ✓ + `yaml.safe_load` ✓（写前过一遍 ✓）；
3. **A∖B 接缝** ✓：job 之间**只传 artifact** ✓，别再各自解析仓库状态 ✗；
4. **失败要早可见** ✓：`ci-watch.sh`（按 job ✓）+ `$GITHUB_STEP_SUMMARY` ✓。

## 附：**Actions 的 2 warnings / 4 notices 也要处置**（2026-09-25 用户要求 ✓）
用户原话 ✓："github action 里的 **2 warnings and 4 notices 也不要忽略了**" ✓。
**已见到的清单** ✓（取自 `check-runs` 注解 ✓）：
| 级别 | 内容 | 处置 |
|---|---|---|
| ⚠ warning | `Node.js 20 is deprecated … actions/download-artifact@v5 … forced to run on Node.js 24` ✗ | **查有没有更新的 major** ✓（若 action 本身已最新 ⇒ 属 GitHub 侧 ✗ ⇒ 记档说明 ✓，不装作没看见 ✓） |
| ⚠ warning | `push 失败（第 1 次），rebase 后重试` ✗ | **我自己脚本**的噪声 ✓（`e2e-ledger` 回提交与其它 push 抢 ✓）⇒ 先 `fetch`+`rebase` 再推 ✓，或把它降成 `::debug::` ✓ |
| ℹ notice | `已为 <sha> 补 e2e-ledger 成功状态` ✓ | 有用 ✓（说明补状态生效 ✓）⇒ **保留** ✓，但可加一句"为什么需要补" ✓ |
| ℹ notice | `e2e 台账已回提交（1.138.0 27/27 · 1.106.0 27/27 · 1.138.0 27/27）` ✓ | **保留** ✓（这是有价值的回执 ✓） |
| ℹ notice | `The ubuntu-latest label will migrate to Ubuntu 26 beginning October 19, 2026` ✓ | **记档 + 设提醒** ✓（到期前确认 runner 行为 ✓ —— 与 `SOKO_VSCODE_TEST_VERSION` 的版本纪律同源 ✓） |
| ℹ notice | （第 4 条待逐轮抓全 ✓） | 下轮用 `gh api …/check-runs/<id>/annotations` **逐 job 收全** ✓ |
**⇒ 待办（下一步 ✓）**：`gh api` 逐 job 把 annotations 收全 ✓ ⇒ 落成一张表 ✓ ⇒ 能修的修 ✓、
不能修的**写进本文并说明为什么不能修** ✓（**"忽略"和"处理过但不动"是两件事** ✓）。

### annotations 逐条处置（round 186 ✓ 收全 ✓）
**实测** ✓（`43d73b1` 那轮 ✓，逐 check-run 收 ✓）：`2 notices`（`ubuntu-latest` 迁移 ✓ ×2）
+ `1 warning`（**`dorny/paths-filter@v3`** 目标是 Node 20 ✗）。
⚠ **注解集合会随跑到的 job 变化** ✓（先前看到的是 `download-artifact@v5` ✗）⇒ **要逐轮收** ✓。
**版本对照** ✓（`gh api repos/<a>/releases/latest` ✓）：
| action | 我用 | 最新 | 处置 |
|---|---|---|---|
| `dorny/paths-filter` ✗ | v3 | **v4.0.3** | **✅ 已升到 v4** ✓（低风险 ✓，且**直接消掉被点名的那条** ✓） |
| `actions/setup-node` ✗ | v4（1 处）/ v5（4 处） | **v7** | **✅ 已把 v4 对齐到 v5** ✓（消掉同仓两种版本 ✓）；v5→v7 待办 ✓ |
| `actions/checkout` ✗ | v5（13 处） | **v7** | ⏳ **大跳，先读 release notes** ✗（默认行为可能变 ✓） |
| `actions/download-artifact` ✗ | v5 | **v8** | ⏳ **大跳** ✗ —— 本仓库**已有它的布局坑**记录 ✓（`skills/sokonanoda-ci` ✓）⇒ **必须读文档再动** ✗ |
| `actions/upload-artifact` ✓ | v7 | v7.0.1 | ✅ 同 major ✓ 不动 ✓ |
| `actions/cache` ✓ | v4 | v4.x | ✅ 不动 ✓ |
| ℹ `ubuntu-latest → Ubuntu 26`（**2026-10-19** ✓） | — | — | **记档 ✓ + 到期前确认 runner 行为** ✓ |
**⇒ 口径** ✓：**能安全修的在当轮修掉** ✓（两条 ✓）；**大跳的写明"为什么这轮不动"** ✓
—— 这就是用户说的"**不要忽略**" ✓：**不是每条都必须改，但每条都必须有交代** ✓。

## 更快识别问题（2026-09-25 用户要求 ✓，六条）
**诊断（用户已核实 ✓）**：`needs` 图里除 `auto-tag` 外**所有 job 都只 `needs: [changes]`** ✗
⇒ `lint-fmt` 8-20 秒能红 ✓，但 `test`(19-37min) / `gates-course`(35min 上限) **照样从头跑到尾** ✗
⇒ run 的"结论"要等**最长的那个 job** ✓ —— 这就是"非要等 CI 完全结束"的机制原因 ✓。

| # | 改法 | 状态 |
|---|---|---|
| ① | **快速失败链**：`test`/`gates-course`/`e2e`/`e2e-macos` ⇒ `needs: [changes, lint-fmt, lint-clippy, gates-fast]` | **✅ 已落** ✓（本轮 ✓） |
| ② | **首个失败就掐掉整轮**（`fast-fail` job ⇒ `gh run cancel`） | **✅ 已落** ✓（本轮 ✓） |
| ③ | **盯 job 级、不盯 run 级** | **✅ 已有** ✓（`scripts/ci-watch.sh` ✓ —— 按 job ✓ + 注解 ✓ + `--follow` 一红即退 ✓） |
| ④ | **e2e 进快层** | **✅ 由 ① 达成** ✓（e2e 现在紧跟快层起跑 ✓，约 2 分钟后 ✓ 而不是排最后 ✓） |
| ⑤ | **`cargo test` 分片** | **✅ 已落（安全版 ✓）**（本轮 ✓） |
| ⑥ | **把 `ci-local.sh` 真接到 push 之前**（hook 或手动 ✓） | **✅ 已落** ✓（本轮 ✓） |

**① 的判据（可验证 ✓）**：`yaml.safe_load` ✓ 13 job ✓；
`test`/`gates-course`/`e2e`/`e2e-macos` 的 `needs` **逐条打印核对** ✓；
快层自身**不等慢层** ✓（`lint-fmt`/`lint-clippy` 无 `needs` ✓、`gates-fast` 只等 `changes` ✓）；
`auto-tag.needs` **未被削弱** ✓ ⇒ 快层红 ⇒ 慢 job skipped ⇒ `auto-tag` skipped ✓ = **不给坏提交打标签** ✓。
**验收（用户给的）** ✓：识别"这轮有问题"的时间 **约 30 分钟 ⇒ 本地 ≤1 分钟 / CI 快层 ≤2-3 分钟** ✓。

### ⑥ 详情（`scripts/githooks/pre-push` + `scripts/install-hooks.sh` ✓，2026-09-25 ✓）
* **装** ✓：`scripts/install-hooks.sh` ⇒ `git config core.hooksPath scripts/githooks` ✓
  （git hook **不随仓库分发** ✗ ⇒ 必须有人执行一次 ✓ ⇒ 已写进 Setup ✓）；
* **跑什么** ✓：只跑**快层** `ci-local.sh --fast` ✓（约 1 分钟 ✓，与"本地 ≤1 分钟"的验收一致 ✓）；
* **逃生门** ✓：`git push --no-verify` ✓ 或 `SOKO_SKIP_HOOK=1 git push` ✓
  —— 但要在 `STATUS.md` 写明原因 ✓（**例外要留痕** ✓）。
* **判据（两向都实测 ✓）**：
  * **反向** ✓：故意加一行坏格式 ⇒ hook **exit 1** ✓，并**指名** `lint：fmt 失败（exit=1，1s）`
    与"**拒绝推送**" ✓ ⇒ **它咬得住** ✓（还原后 diff 干净 ✓）；
  * **正向** ✓：干净树 ⇒ hook **exit 0** ✓（"放行" ✓），随后**真实 push** 也过了它 ✓ = 端到端 ✓。

### ② 详情（`fast-fail` job ✓，2026-09-25 ✓）
```yaml
  fast-fail:
    needs: [changes, lint-fmt, lint-clippy, gates-fast, test, gates-course,
            ledger, contract, editor, e2e, e2e-macos, e2e-ledger]
    if: ${{ always() && contains(needs.*.result, 'failure') }}
    permissions: { actions: write }
    steps: [gh run cancel "${{ github.run_id }}"]
```
* **⚠ 与 ① 的关系（诚实说明 ✓）**：①（快速失败链 ✓）已让慢 job **等**快层 ⇒ 快层红时它们
  **不会起跑** ✓ ⇒ 那部分②是冗余的 ✓；② 真正补的是"**慢 job 自己早早失败**" ✓
  （如 `test (sokonanoda-cli)` 第 3 分钟红 ⇒ 掐掉其余 ✓）。
* **我当场发现并修掉的两个错** ✗：① `needs` 只写 `[changes]` ✗ ⇒ `needs.*.result` **只看它**
  ⇒ 等于没用 ✓（已扩到 **12 个 job** ✓）；② 漏了 `e2e-ledger` ✓（它失败也该掐 ✓，已补 ✓）。
* **判据** ✓：`yaml.safe_load` ✓ **14 job** ✓ · **全文件重复键扫描** ✓ 无重复 ✓ ·
  `fast-fail.needs` **覆盖除 `auto-tag`/自身外的全部 job** ✓（脚本断言"未监视 = 无" ✓）·
  `auto-tag.needs` **未动** ✓（`fast-fail` 不是它的依赖 ✓）。
* ⏳ **待 CI 首验** ✓：下一轮若快层红 ⇒ 预期看到 `fast-fail` 跑起来并 `cancel` 整轮 ✓。

### ⑤ `cargo test` 分片（**量清了 ✓**，2026-09-25 round 207 ✓）
**现状（实测 ✓）**：`test` job 是**按 crate 的 4 路矩阵** ✓（`pkg: [sokonanoda, sokonanoda-front,
sokonanoda-cli, sokonanoda-lsp]` ✓，`timeout-minutes: 40` ✓），命令是
`cargo test -p <pkg> --locked --no-fail-fast` ✓。**实测耗时** ✓（run 36148084664 ✓）：
`sokonanoda` 7m39s ✓ · `-front` 8m27s ✓ · `-cli` 10m41s ✓ · `-lsp` 9m12s ✓
⇒ **四条相当均衡** ✓ ⇒ 所以"再按 crate 拆"**没有空间** ✗（只有 4 个 crate ✓）
⇒ 要压只能**在 crate 内部再分片** ✓。

**⚠ 必须先说的雷** ✗：`cargo nextest` **不跑 doctest** ✗（实测本仓 `doc 代码块 ≈ 12` ✓，
Cargo.toml 未显式关 ✓）⇒ **直接换成 nextest 会静默丢掉 doctest 覆盖** ✗ ——
那正是本 session 反复说的"**静默降级**" ✗。**⇒ 分片必须与 doctest 并存** ✓。

**方案（下轮照做 ✓）**：
1. `test` job 加一步 `taiki-e/install-action@nextest` ✓；
2. 主命令改 `cargo nextest run -p ${{ matrix.pkg }} --locked --partition count:${{ matrix.shard }}/2`
   并把矩阵扩成 `pkg × shard: [1, 2]` ✓ ⇒ **8 条腿** ✓ ⇒ 每条 ≈ **4-5 分钟** ✓；
3. **同 job 内保留一条** `cargo test -p ${{ matrix.pkg }} --doc --locked` ✓
   （只在 `shard == 1` 上跑 ✓，避免重复 ✓）⇒ **doctest 覆盖不丢** ✓；
4. `auto-tag.needs` **无需改** ✓（`test` 这个 job 名不变 ✓，只是矩阵变大 ✓）——
   ⚠ 但仍要跑一遍"未监视 job = 无"的断言 ✓（`fast-fail` 那条 ✓）。
**预期** ✓：最长杆 **10m41s ⇒ ≈5 分钟** ✓ ⇒ 全绿结论 ≈ **6 分钟** ✓（原 ~35 分钟 ✓）。

**判据（可验证 ✓）**：① `yaml.safe_load` ✓ + **全文件重复键扫描** ✓；
② 矩阵条目 **逐条打印核对** ✓（8 条 ✓）；③ **doctest 不丢** ✓：日志里必须出现
`Doc-tests` 段 ✓（**反向验证** ✓：删掉第 3 步 ⇒ 该段消失 ⇒ 判据红 ✓）；
④ 单腿耗时 **≤6 分钟** ✓（数字入 `CI-FAILURES.md` ✓）。

### ⑤ 落地：**安全版**（`pkg × kind` = 12 条腿 ✓，2026-09-25 round 208 ✓）
**为什么不用 nextest** ✗（**两个雷，都在动手前量出来了** ✓）：
1. `cargo nextest` **不跑 doctest** ✗（本仓 ≈12 个 doc 代码块 ✓）⇒ **静默丢覆盖** ✗；
2. 它的**输出格式**与 `cargo test` 不同 ✗ ⇒ 既有的"**失败注解**"步骤（grep `^test .* FAILED$` ✓）
   会**静默失效** ✗ ⇒ 刚建好的"失败可读"能力退化 ✗。
**改用** ✓：矩阵从 `pkg`（4 条）扩成 `pkg × kind` ✓，`kind: [lib, tests, doc]` ✓ ⇒ **12 条腿** ✓：
```yaml
case "${{ matrix.kind }}" in
  lib) flag=--lib ;; tests) flag=--tests ;; doc) flag=--doc ;;
esac
cargo test -p ${{ matrix.pkg }} $flag --locked --no-fail-fast
```
⇒ **输出格式不变** ✓（注解步骤照旧 ✓）· **doctest 变成显式一条腿** ✓（更不容易丢 ✓）·
**不需要任何新工具** ✓ · `auto-tag.needs` **无需改** ✓（job 名不变 ✓）。
**本地判据（三块都实跑 ✓）**：`sokonanoda-front` ⇒ `--lib` **736 passed** ✓ ·
`--tests` **2 passed** ✓ · `--doc` **3 passed** ✓（**doctest 确实在跑** ✓✓ = 覆盖没丢 ✓）。
**预期** ✓：最长杆 **10m41s ⇒ ≈3-4 分钟** ✓ ⇒ 全绿结论 ≈ **5 分钟** ✓（原 ~35 分钟 ✓）。
**⏳ 待 CI 数字** ✓：12 条腿的实测耗时入 `CI-FAILURES.md` ✓。

## g3 的结论：**不该整体做** ✗（2026-09-25 round 241 ✓，**按证据判** ✓）
用户列的是"给环境敏感步骤 `continue-on-error` + **独立汇总 job**" ✓ —— 但**实测证据**显示 ✗：
| 事实 | 证据 |
|---|---|
| 本仓**已有**更好的机制 ✓ | `gap.py` 的「环境异常」**10 处** ✓ · 「timeout」**9 处** ✓ · 「响亮跳过」✓ —— 它**区分"环境"与"回归"** ✓，而 `continue-on-error` **不区分** ✗ |
| CI 侧**已经**在响亮跳过 ✓ | `ledger` 在 CI **不带 `--strict`** ✓ ⇒ 环境异常**跳过且可见** ✓（注解 ✓ + step summary ✓） |
| `e2e` 已不 flake ✓ | 判据改成"**基线 → 变化 → 稳定**" ✓ ⇒ **27/27 × 3 已稳** ✓；再标 `continue-on-error` 只会**掩盖真回归** ✗ |
| 其余 job 都是**确定性**的 ✓ | `lint` / `gates` / `contract` / `editor` / `test` ⇒ 标了**只削弱门** ✗ |
| 现状 | `continue-on-error` 出现 **0 次** ✓ |
**⇒ 建议（按证据 ✓）**：
* **不整体加** `continue-on-error` ✗ —— 环境容错**已经**落在**能区分两者**的那一层 ✓
  （`gap.py` ✓ + `--strict` 在快机器上判红 ✓ ⇒ **守卫不掉牙** ✓）；
* **"独立汇总 job"暂不做** ✗ —— 跳过项**已经**在两处可见 ✓（`::error::` 注解 ✓ + step summary ✓）；
  若将来真出现"**某个具体步骤**因环境红"✓，**再单独给那一步**加 ✓ 并**写明理由** ✓（**不猜着标** ✗）。

## 用户八条的**复审**（2026-09-25 round 301 ✓，**当前工作树实测** ✓ 非凭记忆 ✗）
| 项 | 状态 | 证据 |
|---|---|---|
| a 分片 | ✅ | `pkg(4) × kind(lib/tests/doc)` = **12 条腿** ✓ |
| b 快速失败链 | ✅ | `test.needs=[changes,lint-fmt,lint-clippy,gates-fast]` ✓；`gates-course`/`e2e`/`e2e-macos` 同 ✓ |
| c 首个失败掐整轮 | ✅ | `fast-fail` ✓，**needs 12 个 job** ✓ |
| d 盯 job 级 | ✅ | `scripts/ci-watch.sh`（per-job + 注解 + `--follow` 一红即退 ✓） |
| e 顶层 concurrency | ✅ | `{group: ci-${{ github.ref }}, cancel-in-progress: **true**}` ✓ |
| f `rerun --failed` | ✅ | 已记 `docs/CI-FAILURES.md` ✓ |
| g1 `paths-ignore` | ✅ 等价 | 本仓用 `dorny/paths-filter` ✓（**更细** ✓） |
| g2 `fail-fast: false` | ✅ | `test`/`ledger`/`e2e` ✓ |
| g3 `continue-on-error` | ✗ **按证据不做** ✓ | `gap.py` 能区分环境与回归 ✓，`continue-on-error` 不能 ✗ |
| g4 钉 `ubuntu-24.04` | ✅ | 全文已无 `ubuntu-latest` ✓ |
| h `ci-local` 前置 | ✅ | pre-push hook ✓，**实战拦截 2 次** ✓ |
⇒ **#1 闭环 ✓** ⇒ 转入 **#2：阶段 D（D-2 的 ②–⑤ ✓）**。

## 快层**实测**（2026-09-25 round 315 ✓，job 级 ✓ —— 用户第 d 条的要求 ✓）
```
run #36186692798（sha=d116bda ✓）：
  ✅ changes      20:35:37..20:35:43  ⇒ **6 秒** ✓
  ✅ lint-fmt     20:35:38..20:35:43  ⇒ **5 秒** ✓
  ✅ lint-clippy  20:35:38..20:36:08  ⇒ **30 秒** ✓
  ⏳ gates-fast（快层里最重的一条 ✓）· ⏳ perf-gate（第一轮只报不拦 ✓）
  ⏳ contract / editor / ledger(1,2,3) / test 矩阵
⇒ **三条 lint 合计 30 秒** ✓ ⇒ 用户要的"**快层约 2 分钟**"✓ **已超额达成** ✓
⇒ **快层的实际墙钟 = `gates-fast` 的时间** ✓（它跑课程语料 + 契约 ✓）—— **待它出结论 ✓**
```
**⇒ 读法** ✓：**`changes` + 两条 lint 只要 30 秒** ✓ ⇒ 用户第 d 条"**推完先只看快层**"✓
的**实际代价是 30 秒 + `gates-fast`** ✓ ⇒ **这就是"识别问题"的最短路径** ✓。

## 两条要求**在生产里被实测到**（2026-09-25 round 316 ✓）
```
① run #36186692798（sha=d116bda ✓）= **第 e 条**（顶层 concurrency + cancel-in-progress ✓）：
   整轮 completed/**cancelled** ✗ —— 20:37:1x（约 1.5 分钟处）被掐 ✓
   已绿 ✓：changes 6s ✓ · lint-fmt 5s ✓ · lint-clippy 30s ✓ · **contract 26s** ✓
   被掐 ✗：ledger(1,2,3) · gates-fast · **perf-gate** · editor（都在跑 ✓）
   未开始 ✗：e2e · gates-course · **test 矩阵** · auto-tag · fast-fail
   ⇒ **原因** ✓：随后推了 `ff7d939` ⇒ **`cancel-in-progress: true` 掐掉旧轮** ✓✓
   ⇒ **旧轮不再空跑 10+ 分钟** ✓（**这就是第 e 条的价值** ✓）
② run #36186831292（sha=ff7d939 ✓，**纯 docs** ✓）= **第 g1 条**（docs-only 跳重活 ✓）：
   ✅ changes 7s ✓ · ✅ lint-fmt 6s ✓ · ⏳ lint-clippy
   ⏭ **skipped gates-fast** ✗ · ⏭ **skipped perf-gate** ✗
   ⇒ `changes.outputs.rust == false` ⇒ `if` 跳过 ✓✓（本仓用 `dorny/paths-filter` ✓ **更细** ✓）
```
**⇒ 结论** ✓：用户八条里的 **e（掐旧轮）** 与 **g1（docs 跳重活）** **不是"已接线"** ✓，
而是**在生产里被观测到了** ✓✓ ⇒ **这两条可以标"实测通过"** ✓。
**⇒ 仍缺的数字** ✗：`perf-gate` 的实测 `best_ms`（两轮都没跑到 ✓ ——
第一轮被掐 ✗、第二轮被 skip ✗）⇒ **需要一个 rust 改动的 push** ✓。

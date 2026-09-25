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

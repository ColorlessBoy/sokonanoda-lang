# VS Code 扩展开发规范

> 适用：`editor/vscode/` 下所有改动。接手前先读本文 + `editor/vscode/README.md`。
> 违反版本纪律或跳过测试层 = commit 打回。

## 1. 文件职责

| 文件 | 职责 | 禁止 |
|---|---|---|
| `extension.js` | 扩展入口：LSP 客户端接线、命令注册、练习树/课程树/项目树/状态栏/inlay/跳洞 | 业务逻辑、kernel 调用 |
| `src/abbreviations.js` | 记法缩写表（`\and`→`∧`）：`crates/front/src/notation_input.rs` 的**逐字镜像**，纯 JSON 数组字面量（Rust 契约测试真解析它） | 自己加/改条目（先改 Rust 表）、非 JSON 的表格式 |
| `src/abbreviation-rewriter.js` | 缩写改写器状态机：Tab 命令、`sokonanoda.input.eager`、context key（Tab 的 `when` 子句）、一次 edit = 一个 undo 单元 | 命令注册（在 `extension.js`）、判定/编译 |
| `project-tree.js` | 项目树渲染（只吃 `soko/project` 的答案：根 = 模块根 + 清单来源 + 计数，子 = 拓扑序模块 + 状态图标；单文件一条占位行）。**请求在 extension.js**，这里只有渲染与"答案指名别的文档 ⇒ 丢弃" | 发请求、判定项目状态 |
| `server.js` | 服务器获取：平台→target 映射、bundled `bin/<target>/` 解析、exec 位修复、版本锁定下载（**无 `vscode` 依赖，可纯 Node 单测**） | UI/命令逻辑 |
| `scripts/stage-lsp.js` | 打包前把构建产物 stage 到 `bin/<target>/`（chmod 755），支持 `--package` 出 host VSIX | 运行时逻辑 |
| `test-server.js` / `test-download.js` | 纯 Node 单测（解析顺序/版本锁定 URL/重定向/解压） | — |
| `package.json` | 清单：contributes、dependencies、engines、**description/keywords（市场门面）** | 运行时逻辑 |
| `README.md` | **Marketplace 页面正文**——安装方式、功能清单、agent 集成卖点 | 与实际行为不符的描述 |
| `CHANGELOG.md` | 市场可见的版本历史（Keep a Changelog） | 与 commit 内容不符的条目 |
| `syntaxes/*.tmLanguage.json` | TextMate 语法（即时高亮，LSP 语义高亮的降级层） | — |
| `language-configuration.json` | 括号配对、注释、缩进 | — |

## 2. 版本纪律

**bump 用脚本，别手改**：`python3 scripts/bump.py <x.y.z>` 一次写全
（`Cargo.toml` + `editor/vscode/package.json` + `Cargo.lock` + 仓库里所有清单的
`requires`）；`python3 scripts/bump.py --check` 是 gate 与 CI 的那条门禁。
手改漏掉清单的 `requires` 就是 G-24 的成因。`CHANGELOG.md` 仍然手写。
（semver，硬规则）

### 判断标准

| 变更类型 | 版本升 | 判断依据 | 例 |
|---|---|---|---|
| **新功能** | **minor**（0.2 → 0.3） | 学习者能做**之前做不到的事** | 新增练习树、新增跳洞、新增 inlay hints、新增 rename |
| **改进 / bug 修复** | **patch**（0.3.0 → 0.3.1） | 已有功能变得**更好用或更正确**，但没多出新能力 | hover 临近回退、括号悬停、诊断分级、修坐标偏移 |
| **破坏性变更** | **major**（1.0.0） | 设置改名、命令移除、行为不兼容 | 移除某个设置项 |

### 判断口诀

> 问自己：学习者**能不能做一件之前做不了的事**？
> 能 → minor。不能（只是做得更好/更对）→ patch。

### 实际案例

| 改动 | 正确版本 | 为什么 |
|---|---|---|
| 新增练习面板 | minor | 学习者多了"看练习列表"的能力 |
| 新增跳洞（alt+n） | minor | 学习者多了"跳洞"的能力 |
| hover 括号回退 | **patch** | hover 一直存在，只是现在括号上也有信息了 |
| 修复 VSIX 打包 | **patch** | 没多出新能力，修的是安装后坏的问题 |
| 新增 rename | minor | 学习者多了"重命名"的能力 |

### 多个改动合并提交

一个 commit 里既有 minor 又有 patch 改动 → 取**最高的**（minor）。
一个 commit 纯 patch 改动 → patch。不确定时 → patch（宁可低不可虚高）。

### 什么时候 bump（**2026-09-21 修订：bump = 发布边界，不是 commit 边界**）

> 旧规定是"每次 commit 涉及 `editor/vscode/` 必须 bump"。那条**已作废**——
> 它会让每个小环节都触发一次完整 release，而大计划（`docs/design/vscode-editor-feedback-plan.md`
> 的 100+ 环节）是按"一次少做点"推进的。现在的规则是**按阈值 bump**。

**先记住 CI 的实际行为**（`.github/workflows/ci.yml:39-71` 的 `auto-tag`）：
push main 且 lint/test/e2e/e2e-macos 全绿后，它读 `Cargo.toml` 的版本，
**只有当 `v${version}` 这个 tag 还不存在时**才打 tag 并 dispatch release。

⇒ **不动版本号，推多少次 main 都只跑 CI、不发版。**

**bump 触发（命中任一就 bump，不必等整批做完）：**

1. 一条**用户可感知的能力**落地（不是内部重构）——例如"声明栏不再为空"、
   "目标显示记法"、"打开不再等 5 秒"；
2. 距上次 bump 已过 **≥ 8 个环节**；
3. 用户要拿去测。

**类型**：新能力 = `minor`；只是修好/更快 = `patch`（口诀见上）。

**bump 动作（四步，缺一步就出问题）：**

1. **两处版本必须相等**：`Cargo.toml` 的 workspace version +
   `editor/vscode/package.json` 的 version。CI 的 `auto-tag` 与 release 的
   version gate 都会比对，不等直接失败；契约测试
   `cargo_and_extension_versions_match` 本地就会红。
2. **版本号只能是纯 `x.y.z`**。`scripts/soko` 的解析正则是
   `^v?(\d+)\.(\d+)(?:\.(\d+))?$`（`scripts/soko:110`），带后缀（`0.63.0-local`）
   会让启动器解析不出期望版本、按 G-16 **拒绝运行**。
3. 同步更新 `editor/vscode/CHANGELOG.md`（Keep a Changelog 格式）。
4. bump 后重新 `vsce package` + `code --install-extension` 并在本地 VS Code 验证
   （或走 CI 出的 VSIX）。

**bump 的两个已知代价（接受即可）：**
① **编译缓存全失效**（缓存键含 `CARGO_PKG_VERSION`）⇒ bump 后第一次打开会重编一遍；
② 每次 release 跑完整流水线（8 LSP tarball + 8 CLI + 9 VSIX + SHA256SUMS +
provenance + Marketplace 发布），历史上 gallery 会间歇超时（`docs/CI-FAILURES.md`）。

**清单 `requires` 要跟着 bump**（**这是 G-24**）：`courses/*/sokonanoda.toml` 与
`course/*/sokonanoda.toml` 里的 `requires` 若不跟着走，`requires_warning` 会让
`ProjectReport::is_clean()` 为假 ⇒ **项目编译缓存被静默关掉**（实测：整个卷 I
每个文件每次打开都从零重编）。门禁见 `scripts/check-manifests.py`（T-A08）。

## 3. 测试三层

| 层 | 工具 | 覆盖 | 文件 |
|---|---|---|---|
| 纯 Node 单测 | `npm run test:unit` | server.js 解析顺序/版本锁定 URL/exec 位修复、重定向、解压 | `test-server.js` / `test-download.js` |
| 静态契约 | `cargo test -p sokonanoda-cli --test extension` | package.json 字段完整性、命令注册一致性、依赖打包安全、bundled 解析/版本一致/市场元数据 | `crates/cli/tests/extension.rs` |
| 打包冒烟 | CI `Package host VSIX` step | `bin/<target>/` 入包、exec 位、`TargetPlatform` | ci.yml |
| 宿主接线（stub host） | `node editor/vscode/test-extension-host.js`（`npm run test:unit` 的第 4 个文件） | **行为**：诊断事件过滤/去抖/合并、并发 `soko/goals` 合并、切文件丢弃过期答案、Infoview `decls` 去重、课程树缓存、**项目树三态**（闭包渲染 / 单文件占位 / 丢弃他人答案）、**记法缩写改写器**（`\and`+Tab、前缀陷阱、孤立 `\`、多光标、一次 undo 单元、eager 开关、Tab 的 context key）。用 stub 的 `vscode` / `vscode-languageclient` / `child_process` + 假定时器跑真 `extension.js`，零依赖、毫秒级 | `editor/vscode/test-extension-host.js` |
| 集成测试（**单用例，每环节跑**） | `scripts/vscode-e2e.sh --grep "<用例名>" --profile debug --no-build` | **真宿主 + 真 LSP 的单条用例**（~5 秒，实测）；`--grep`/`--profile`/`--no-build` 三个开关见 `docs/E2E.md` §1b。环节循环的 L4 层，见本文 §4.1 | `editor/vscode/src/test/extension.test.js` |
| 集成测试（**例行化，全量**） | `SOKO_VSCODE_TEST_VERSION=1.138.0 scripts/vscode-e2e.sh`（内部 `npm test` → @vscode/test-electron） | 真宿主端到端：激活、语言 id、诊断、inlay/hover、重启、Infoview、doctor、**项目树**（真 `soko/project` 答案渲染的行）；结果记进 `docs/e2e/ledger.jsonl`（`soko.e2e/1`）。手册 = `docs/E2E.md` | `editor/vscode/src/test/extension.test.js` |
| 手动验证 | F5 开发宿主 | 全功能（面板、树、inlay、跳转、补全、安装态离线） | — |

**commit 前**：至少跑静态契约 + 集成测试；**发 tag 前**：三层全跑。
**扩展改动后**：`scripts/soko gate`（Rust + 契约层）+ `node test-extension-host.js`（stub 层）+
`scripts/vscode-e2e.sh`（真宿主层，~1 分钟；结果进 `docs/e2e/`）。
CI 的 `e2e` job 跑的是**同一条命令**（3 条腿：ubuntu × VS Code 1.138.0 与 1.106.0
（声明的最低版本）、macOS × 1.138.0 只在 main 上跑；`auto-tag` 等它）——本地跑绿
基本等于 CI 绿；红了的排查顺序见 `docs/E2E.md` §4。

## 4. 开发循环

### 4.1 环节循环（**一次少做点、快速反馈**）

> 大计划（`docs/design/vscode-editor-feedback-plan.md`，100+ 环节）按"一个环节
> 一个可验收的小改动"推进。反馈分 5 层，**越靠前越快**：

| 层 | 命令 | 耗时 | 用在哪 |
|---|---|---|---|
| **L1** LSP 直探（不开 VS Code） | `bash docs/gaps/repro/G2x-*.sh` | 秒级 | 绝大多数环节的判据就是它 |
| **L2** 扩展 stub 宿主 | `node editor/vscode/test-extension-host.js` | 秒级 | 只改 `extension.js` 的环节 |
| **L3** Rust 单测过滤 | `cargo test -p sokonanoda-front <filter>` | 十秒级 | 每个 Rust 环节 |
| **L4** 真 VS Code **单用例** e2e | `scripts/vscode-e2e.sh --grep "<用例名>" --profile debug --no-build` | **~5 秒**（实测） | 每个修复环节（DoD 第 4 步） |
| **L5** 检查点全量 | `scripts/soko gate` + `scripts/vscode-e2e.sh`（不带 `--grep`） | 分钟级 | 只在批次收尾 |

**全貌**（六条用户反馈的现状，一条命令）：`scripts/verify-editor-issues.sh`。

### 4.2 肉眼看效果（**不用重装 VSIX**）

- **Rust 侧改动（绝大多数）**——`serverOverride` 回路：
  ```bash
  scripts/dev-loop.sh lsp        # cargo build -p sokonanoda-lsp -p sokonanoda-cli（debug）
  # VS Code 命令面板 → sokonanoda: restart server
  ```
  一次性设置：`"sokonanoda.serverOverride": true` +
  `"sokonanoda.serverPath": "<本仓绝对路径>/target/debug/sokonanoda-lsp"`。
  不设 `serverOverride` 的话扩展优先用它自带的 bundled 服务器，这份 debug 构建**被忽略**。
  **限制**：这只换服务器二进制，**换不了扩展代码** ⇒ 客户端改动要用 4.3。
- **扩展侧改动**——F5 开发宿主：`cd editor/vscode && npm run clean:lsp`，
  然后按 F5；改 `extension.js` 按开发宿主的重载按钮。
- **要装成真扩展**（只在检查点）：`npm run package:host` +
  `code --install-extension sokonanoda.vsix --force` + Reload Window。

### 4.3 两条老路（打包 / 安装态验收）

```bash
# 改 extension.js / server.js 后：
cd editor/vscode
npm run test:unit                 # 纯 Node 单测（秒级，先跑这个）
cd ../.. && cargo test -p sokonanoda-cli --test extension   # 静态契约
cd editor/vscode

# 本地验收安装态（bundled 路径）：stage 本机 release 二进制 + 打平台 VSIX
cargo build --release -p sokonanoda-lsp
npm run package:host
code --install-extension sokonanoda.vsix --force
# 手动 Reload Window（Cmd+Shift+P → Reload Window）

# 回到纯开发发现（可选）：清掉 stage 的 bin/，F5 会走 target/
npm run clean:lsp
```

或按 F5 用开发宿主调试（`.vscode/launch.json` 已配置）。注意：F5 时
`bin/` 不存在（gitignored），解析会落到 `target/debug`；若刚跑过
`package:host`，会优先使用 stage 的 release 二进制——要回到 debug 发现先
`npm run clean:lsp`。

### 4.4 环境前置：macOS 的 Xcode 许可

`xcrun --show-sdk-path` 失败时，**任何 Rust 链接都报**
`error: linking with cc failed: exit status: 69`（不是代码问题）。两种解法：

```bash
sudo xcodebuild -license accept                              # 正解
export DEVELOPER_DIR=/Library/Developer/CommandLineTools     # 绕过（只影响本 shell）
```

`scripts/dev-loop.sh` 会检测并打印这条提示（**不替你改环境**）。

## 5. 常见坑（全部踩过）

1. **`node_modules` 不能进 `.vscodeignore`**——vsce 靠它把生产依赖装进 VSIX；
2. **`vscode-languageclient` 必须在 `dependencies`**——放 `devDependencies` 的 VSIX 装上即坏；
3. **打包冒烟别带 `--no-dependencies`**——该 flag 跳过生产依赖收集，会打出
   12 文件/21KB 的"空壳 VSIX"。正确基线（2026-09-13 实测 0.19.0）：
   **平台包 ≈328 文件/4.3MB**（含 `bin/<target>/` 的 LSP+CLI ≈9.5MB 解压）、
   **universal ≈326 文件/0.5MB**（不含 bin，走版本锁定下载）；冒烟后核对
   文件数**和 bin 平台归属**再认定通过；
4. **didOpen 是通知**——不发 id，不期待响应；探针/测试里发 id 会被当作未知请求；
5. **LSP 帧格式**——头块以 `\r\n\r\n` 结尾；探针/测试必须完整消费头块再读 body；
6. **设置项**（`contributes.configuration`）：`serverPath` / `serverOverride` /
   `trace.server` / `input.eager` / **`warmCacheOnOpen`**（T-A52，默认关——激活时
   后台把工作区根 `build` 一遍预热缓存）。**只在 `activate()` 里读一次**的设置
   （`warmCacheOnOpen` 就是）测试没法事后开：stub 宿主用
   `activateExtension({ warmCacheOnOpen: true })`，真宿主靠夹具工作区的
   `.vscode/settings.json`（`src/test/fixtures/workspace/.vscode/settings.json`）。
7. **服务器更新后须重载窗口**——LSP 进程在窗口激活时 spawn，改 Rust 代码后不重载 = 旧服务器；二进制原地更新（重建 / 缓存刷新 / 改 `serverPath`）可用命令 `sokonanoda: restart server` 重新解析并重启（命令回执会显示重启前后的服务器版本与 pid）。**0.27.1 起**：restart 走与激活同一条解析链（含 `v<扩展版本>` 锁定下载兜底），解析不到可用服务器时直接报错而**不再静默重启旧命令/旧缓存**；若检测到磁盘上已安装更新的扩展而当前宿主仍是旧版，会提示 `Developer: Reload Window`（扩展本体升级仍需重载——restart 只能换服务器二进制，换不了扩展代码）。**0.31.0 起**：解析默认 **bundled-first**（`sokonanoda.serverOverride` 默认 `false`，`serverPath`/env/工作区构建被忽略并弹一次提示），杜绝「陈旧本地构建静默压过内置服务器」；`sokonanoda: doctor` 只读输出解析来源（`source=`）、运行/扩展版本、被忽略的覆盖、缓存与旧版本堆积等自检项；
7. **`code` CLI 与已开实例冲突**——集成测试在 macOS 上报"another instance running"时关掉 VS Code 再跑；
8. **代理**——vsce/Node 不读系统代理；需要时设 `HTTPS_PROXY=http://127.0.0.1:7890`。
9. **exec 位只能在 Linux/macOS 打包**——Windows 上 `vsce package` 会丢 unix
   mode（zip external attributes），装到 mac/linux 后二进制不可执行。CI 在
   ubuntu 打包；`scripts/stage-lsp.js` staging 时 `chmod 755`；冒烟用
   python `zipfile` 断言 `mode & 0o111`。
10. **`bin/` 不进仓库、也不出包外**——`editor/vscode/bin/` 是 staging 目录
    （gitignored），`.vscodeignore` 不许排除它（契约测试守护）；每次打包前
    先 `clean:lsp`，保证一个 VSIX 只带一个平台。
11. **下载回退禁止 `latest`**——只允许
    `releases/download/v${extensionVersion}/…`（契约测试断言 server.js 不含
    `/latest/`）；否则旧插件会拉到新服务器，协议错配且不可复现。
12. **Linux 目标必须用 cargo-zigbuild + `.2.28`**——ubuntu-latest 原生构建
    会带 glibc 2.39 符号（VS Code 自己的 Linux 底线是 2.28）；Zig 0.16.0 /
    cargo-zigbuild 0.23.4 版本钉死。Alpine（musl）必须是静态链接
    （`ldd` 报 "not a dynamic executable"）；`win32-arm64` 在 windows-latest
    原生构建即可（VS ARM64 工具链预装）。
13. **bump 版本后跑集成测试必须先 stage/指定二进制**——服务器解析顺序是
    `setting` → `SOKONANODA_LSP_BIN` → bundled `bin/<target>/` → 工作区
    `target/` → 按当前版本命中的缓存（`server.js` `resolveServerCommand`）。
    版本号一涨，缓存里那份旧版本就不再「命中」，而新版本的 release 还不存在
    → 解析返回 `undefined` → 回退下载 404 → **整个 LSP 起不来，全部集成
    用例集体超时**（症状像代码回归，实则环境）。跑 `npm test` 前先
    `npm run stage:lsp`（或 `SOKONANODA_LSP_BIN=$(pwd)/../../
    target/debug/sokonanoda-lsp`）。注意集成测试自己的 skip 守卫是
    `findServerBinary()`：它只认「测试文件上四级的 `target/debug|release`」
    与 `PATH`，与扩展的解析顺序**不是同一套**——两处都要能满足。
14. **仓库路径过长时 VS Code 集成测试起不来**——`IPC handle ... is longer
    than 103 chars`（macOS socket 上限）。**0.58.0 起 `.vscode-test.mjs` 自己把
    `--user-data-dir`/`--extensions-dir` 指到 `<tmpdir>/soko-vscode-test`，本仓库的
    长路径可以直接跑**（socket 路径因此只有 ~90 字符）；只有 `tmpdir` 本身也过长时
    才需要下面这套拷贝法：`rsync -a --exclude node_modules --exclude .vscode-test --exclude bin
    editor/vscode/ /tmp/v/`，再 `ln -s` 回 `node_modules` 与 `.vscode-test`
    （省去重复下载），并把服务器二进制所在目录塞进 `PATH`。此时
    `REPO_ROOT` 会退化，`bin/` staging 与 `PATH` 两个条件都得显式满足（见 13）。

15. **多文件项目改变了"一个文档一次编译"的假设（0.57.0，I16）**——LSP 现在
    同时跟踪多个文档（`Docs{map,order,root,active}`），而**入口文档的诊断是整个
    import 闭包的结果**：诊断里带 `file`/`module` 的属于**别的文件**，`goto_definition`
    可能返回另一个文档的 `Location`（扩展的跳转不要假设同文件）。另外两条实测语义：
    ① 改依赖会**立刻**让含它的打开文档重编译重发（未落盘编辑经内存覆盖可见），
    诊断只在真的变化时才发；② `initialize` 的 `rootUri` 决定模块根，多根工作区
    目前只取第一个 folder；③ 编辑器**外**改文件（git checkout / 别的工具）现在会
    触发刷新（服务端实现了 `workspace/didChangeWatchedFiles`：只重编译闭包里含该
    路径的已打开文档，缓冲区优先）。写多文档测试必须用
    `testutil::notify_with_drain`（先等通知再读 socket 会死锁，见
    `docs/TESTING.md` §5.7）；夹具起点：`handshake_with_root` /
    `did_open_at_drained` / `did_change_at_drained` 与 `crates/lsp/src/tests/project.rs`。

16. **诊断事件是"全窗口"的，必须自己过滤（0.57.0 实测）**——`onDidChangeDiagnostics`
    会把**别的扩展**（TS/ESLint/rust-analyzer）的诊断也报给你；它也不做内容 diff
    （`DiagnosticCollection.set` 每次都触发）。早先的监听器收到任何事件都
    `refresh() + ensureDeclarations() + stateAt`，于是：一个无关 `.ts` 文件报错会跑
    一整轮 `soko/goals`；项目模式一次编辑发多份文档 ⇒ 一次事件里跑 **2 次**
    `soko/goals` + 2 次 Infoview 整表重建。现在：`event.uris` 里必须有
    `.sokonanoda` + 150ms 去抖 + 并发请求合并 + 载荷指纹去重。
    守护：`editor/vscode/test-extension-host.js` 的前两例（对着旧代码会红）。
17. **异步请求回来时文档可能已经换了（0.57.0 实测）**——`loadDeclarations()` 曾在
    `await` **之后**读 `this.uri` 建树节点：请求是 A 发的、回来时用户切到 B，
    结果树上那行的标签是 A 的声明、点击命令却是 `revealRange(B, A 的洞)`。
    规矩：**请求发起时把 URI 钉住**（`const requestedUri = this.uri`），`await` 之后
    先比对再落地（`requestCursorState` 早就是这么做的）；跨文件跳转的目标 URI
    必须来自钉住的那份文档。守护：`test-extension-host.js` 的"切文件丢弃过期答案"。
18. **一次课程树 resolve = 一个 CLI 进程 = 11 个单元编译**（release 热缓存实测
    ~320ms）——树在结果回来前是空的，而 VS Code 会在展开/可见性变化时重新 resolve
    根节点，所以必须缓存（现在 30s TTL；`sokonanoda.courseRefresh` 与激活强制重跑）。
    另外 `server.js` 的下载回退里**不能**用 `execSync`（会冻结整个扩展宿主），
    已改 `await execFile`。
19. **`activate()` 里的提前 `return` 会掐掉语言服务器（0.58.0 踩过）**——异步续段
    （`(async () => { … client.start() … })()`）挂在 `activate` 的**尾部**；为了给集成
    测试暴露 provider 而在中间 `return`，测试宿主里就**永远不起服务器**：整套用例集体
    超时，且 `SOKO_E2E_LOG` 里连 `server command` 都没有。规矩：test-mode 的返回值在
    函数**最后**返回（`const testApi = …; … (async () => …)(); return testApi;`）。
    延伸纪律：真宿主测试必须能回答"服务器起没起"——doctor 的 `server-version` 行 +
    `SOKO_E2E_LOG` 就是那两个答案。
20. **真宿主 e2e 的第一现场是 `SOKO_E2E_LOG`**——扩展宿主的 `console`/output channel
    在 `vscode-test` 的输出里取不到。`scripts/vscode-e2e.sh` 会设这个环境变量，扩展据此
    把"解析到的服务器命令、客户端状态、活跃文档、`soko/project` 结果"写进文件；判读口径
    见 `docs/E2E.md` §4。生产路径零开销（环境变量不存在时只做一次 `undefined` 判断）。
21. **被 ignore 的 `bin/` 会悄悄变旧**——扩展 bundled-first，集成测试测的就是
    `bin/<target>/`；它不在 git 里，很容易停在几天前的构建上（第一次例行跑：14 条里
    10 条超时，对着 0.20.0 的服务器断言 0.58.0 的行为）。`scripts/vscode-e2e.sh` 把
    "构建 release + stage" 做成固定步骤，台账里另记 `lsp_sha256_16` 与 doctor 的
    `server-version` 行，用来证明"测的是当前构建"。
22. **`.vscodeignore` 的 `src/**` 会把运行时模块一起排除**（0.62.0，NI-2）——
    `src/` 从前只住集成测试（`src/test/**`），整目录排除一直没人发现；一旦有运行时
    模块住进去（`src/abbreviations.js`、`src/abbreviation-rewriter.js`，由
    `extension.js` require），打出来的 VSIX 装上即坏
    （`Cannot find module './src/abbreviation-rewriter'`），而 stub / 契约 / e2e
    **三层全发现不了**——它们跑的是仓库里的文件，不是 VSIX 里的那份。规矩：排除写成
    `src/test/**`；往 `src/` 加运行时文件后，跑一次 `npm run package:host` 并核对
    VSIX 里真的有它（文件数基线见 §5.3）。

## 5b. Infoview/视图的硬规矩（0.49.0 教训）

1. **provider 先注册**：`activate` 最前面同步 `registerWebviewViewProvider` / `createTreeView`，
   任何 `await`（服务解析/下载/启动）放到之后；否则面板有一段"无 provider"的空白期。
2. **别给 webview 视图加 `when`**：扩展容器 `hideIfEmpty: true`，条件不满足会隐藏整个
   容器（面板"弹不出来"）。要常驻就 `visibility: visible` + 不写 `when`。
3. **激活入口**：`activationEvents` 显式加 `onView:<viewId>`，保证没开 `.sokonanoda`
   文件时点面板也能激活。
4. **绝不静默**：面板先渲染骨架 + `status`（编译中/已就绪/等待文件）；契约测试见
   `crates/cli/tests/extension.rs`。

## 6. 发布

```bash
# 本地发布（需要 PAT，见 docs/RELEASE.md）
cd editor/vscode
npx --yes @vscode/vsce publish

# CI 自动发布：bump 版本 + push main 即可——ci.yml 的 auto-tag 自动打 tag
# 并 dispatch release.yml（docs/RELEASE.md §3）。手动推 tag 仅应急：
git tag v0.X.Y && git push origin v0.X.Y
```

发布形态：per-target VSIX（内嵌各平台 LSP）+ universal 回退包；完整流程、
版本门禁与 dry-run 见 `docs/RELEASE.md`。发布前检查清单：
- [ ] `package.json` version 已 bump
- [ ] `CHANGELOG.md` 已更新
- [ ] `README.md` 与当前行为一致（见 §7 文档同步）
- [ ] `cargo test --workspace --locked` 全绿
- [ ] `npm test` 集成测试全绿
- [ ] `icon.png` 存在且 ≥128×128 PNG

## 7. 文档同步（市场页面即门面，硬规则）

> 历史教训（2026-09-09）：server 早已实现 GitHub Release 自动下载
> （rust-analyzer 模式），README 的 Quick start 还在教 `cargo build`；
> agent skills 是项目最大卖点，市场介绍里只字未提。过时的门面 =
> 用户以为插件不可用。

### 必须同步的三个门面文件

| 文件 | 出现在哪 | 内容 |
|---|---|---|
| `README.md` | Marketplace 页面正文 | 安装/获取方式、功能清单、agent 集成 |
| `package.json` → `description` | 搜索列表的一行简介 | 一句话卖点（零安装门槛 + 教学 + agent）；**≤ 300 字符**，超了 Marketplace 硬截断（无省略号，切在词中间——见下） |
| `CHANGELOG.md` | 页面"Changelog"标签 | 每个版本用户可感知的变化 |
| `skills/sokonanoda-{teacher,dev,ci}/` | agent 加载的操作手册（符号链接到 `~/.agents/skills`） | 新能力/新命令/新坑：写成**可直接执行**的命令，少 token |
| `AGENTS.md` + 本文 | 仓库/扩展开发入口 | 流程、门禁、同步义务 |

### 同步触发器（命中任一 = 同一 commit 里改门面）

1. **安装/获取方式变化**：server 下载策略、发现顺序、缓存路径、新增设置项；
2. **功能集变化**：新命令/键位/树/视图（对照 `package.json` contributes）。
   0.60.0 的例子（`sokonanoda.build`/`rebuild`，把 CLI 的编译缓存预热接进编辑器）：
   `contributes.commands` + `keybindings` + `menus.view/title`、`extension.js` 注册与
   实现（子进程超时 kill、JSON Lines 事件、跑完刷新三个视图）、
   `crates/cli/tests/extension.rs::build_and_rebuild_commands_warm_the_compile_cache`、
   真宿主冒烟一例、README/CHANGELOG；
3. **反馈行为变化**：诊断分级、hover 内容、inlay（用户能在编辑器里"感觉到"的）；
4. **agent 集成变化**：skills 增删、opencode 接线、CLI 事件面；
5. **任何用户可见改动**：同步 `editor/vscode/`（README/CHANGELOG/package.json）**与**
   `skills/` 三个技能 —— 二者**同一轮一起改**，别留到"以后再补"。

### description 硬上限 300 字符（2026-09-15 教训）

Marketplace 详情页把 `description` 当"短简介"，**超过 300 字符直接截断**
（无省略号、切在词中间）。0.39.1 的实际表现：

```
… it works offline. Ships agent skills (Claude Code / ope
```

即 348 字符的简介被切在 `opencode` 中间——第一屏门面直接"翻车"。规程：

- `description` = 一句话，**≤ 300 字符**（留余量，别贴着上限写），必须 ASCII、
  以句号结尾；
- 长卖点放 `README.md`（Marketplace 正文完整渲染，不截断）；
- 护栏：`crates/cli/tests/extension.rs::marketplace_description_fits_the_gallery_limit`
  断言长度/结尾/ASCII——改坏了 `cargo test` 直接红。

### 事实校对规程

- README 里的每个行为声明，**必须能在 `extension.js`/LSP 服务器里指出
  对应实现**（如"自动下载"→ `downloadLspBinary` 的 URL 与缓存路径）；
- 数字必须可复现（VSIX 文件数基线、VS Code 最低版本 = `engines.vscode`）；
- 疑似过时 → 以代码为准改文档，不许"以后再改"；
- `crates/cli/tests/extension.rs` 静态契约守护元数据存在性——文案正确性
  靠本节规程人肉把关（契约测试读不出"说谎"）。

### 发布后验证

```bash
HTTPS_PROXY=http://127.0.0.1:7890 npx --yes @vscode/vsce show <publisher>.<name>
# 核对：Version 与 tag 一致、description 已更新、Marketplace 网页 README 渲染正常
```

23. **同一条"能不能用"的判据写两遍，就会有一处漏掉（G-22，2026-09-21）** ——
    "单独 parse 失败但 `import` 闭包编译成功 ⇒ 这份文档可用"这条判据，曾经在
    `QueryDoc::check()` 与 LSP 的 `Doc::set_text` 里各写了一遍，而
    `QueryDoc::goals()` **漏了** ⇒ 项目入口（用库记法的课程单元）的
    `soko/goals` 恒为空：**声明栏空、`alt+n` 没反应**，而目标栏/悬停/文档符号
    全都正常——这种"一半好一半坏"的不对称最难查。现在判据收敛成
    **一处** `QueryDoc::usable()`。规矩：**凡是"这份文档能不能用"的判断，
    只能有一个函数**；发现第二处就合并。
    同族的第二条（E6）：**"取过没有"不能看 `declItems` 的真值**——
    `decls = []` 时 `declItems = []`，而 `![]` 是 `false` ⇒ 之后每次
    `ensureDeclarations()` 全空转。用 URI（`_declsUri`）记，别用真值。
    第三条：**取数失败（`response === undefined`）不算"取过了"**——
    否则"激活时活动编辑器已是 `.sokonanoda`"（VS Code 重启的常态）会永久空转。

## 坑：显示层的"每层都做一遍"在声明上千的文件上是 O(n·深度)（2026-09-21）

线 C 的记法折叠第一版让**每一层** `App` 祖先都 `render_expr` 一遍整棵子树
（"折过的子树被应用时把替换范围上提到应用脊根"那条规则）——结果 `did_open`
三档全中 **+18~22%**。改成只让**最外层**记一次（`fold_collecting_inner` 的
`in_spine` 开关）后回到噪声内。**显示层挂在每条声明的出口上时，先问一句
"这个操作是每声明一次还是每节点一次"。** 账见 `docs/PERF.md`。

## 坑：`perf_course` 的 LSP 用例量的是 **debug 构建**

`crates/lsp/src/tests/perf_course.rs` 走 `testutil::test_service()`——**进程内**
服务 ⇒ 它量的是 debug（`cli_profile=release` 指的是另跑的 CLI 档）。debug 下
每声明的额外开销会被放大、代码布局变化也会整体影响内联决策 ⇒ 拿它当"用户能
感觉到的退化"会误判。要看用户侧的数字得量 release（`query`/`grade` 冷跑）。
台账里 `did_open` 比 26 小时前高 ~18% 这件事的排查记录在 `docs/PERF.md`
（已排除折叠与防抖，待查）。

## 迭代速度：`gate --fast`（2026-09-23，用户报"gate 太慢、严重阻碍迭代"）

**先量再改**。完整 gate 的账（本机，warm 构建；CI 侧的 suite 耗时见下）：

| 阶段 | 改前 | 改后 | 怎么省的 |
|---|---|---|---|
| `cargo fmt --check` | 1s | 1s | — |
| `cargo clippy --workspace --all-targets` | 0s（缓存） | 0s | — |
| `cargo test --workspace` | ~250s | 只跑**改动过的 crate** 的 `--lib` | 最贵的是**课程规模**的 suite：LSP 库 129s、`judge_batch` 87s（CI 实测） |
| 课程门禁 `check.py` | **164s** | **0s** | **持久编译缓存**（`target/gate-cache`）：它以前每次都是冷的（`grade()` 只继承环境，没人给缓存目录） |
| 缺口台账门禁 `gap.py check` | 94s | 跳过（提交前跑） | 它是**提交前**的契约，不是迭代信号 |
| **合计** | **~15 分钟** | **~30 秒**（`gate --fast`） | |

**用法**：

```bash
scripts/soko gate --fast   # 迭代：改了就敲这条（~30s）
scripts/soko gate          # 提交/推送前：完整那一条（~6.6 分钟）
```

**两条纪律**：
1. **`--fast` 不是"更弱的判据"，是"更小的范围"**：它照样跑 fmt / clippy / 锚点 /
   **课程门禁**（那条最要紧的不变量，现在免费）；只是**只测改动过的 crate**、
   不跑集成测试、不查台账契约。**推送前必须跑完整 gate**（CI 也会跑）。
2. **持久缓存是安全的，但要知道它在**：缓存是**内容键**的，且 `soko` 对版本不匹配
   **拒绝运行**（G-16）⇒ 复用不会掩盖过期。想回到冷缓存：
   `SOKONANODA_CACHE_DIR=$(mktemp -d) scripts/soko gate`。

**为什么课程门禁能快 164s → 0s**：`check.py` 的 `grade()` 用 `subprocess.run` 且
**只继承环境**——没人给它 `SOKONANODA_CACHE_DIR` ⇒ 34 个目标每次都从头编。
给它一个**仓库内的持久目录**（`scripts/soko` 里注入）之后，第二次起全是命中。
CI 侧同理（但 CI 每次是全新 runner，所以那边省不掉——这也是"本地快、CI 慢"
这条差异的来源之一）。

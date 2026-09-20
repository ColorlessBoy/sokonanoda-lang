# C3 — 工具链与 AI-agent 集成事实档案

> 用途：官网重建的内容底稿（工具链 / 安装 / agent 集成 / 质量证据）。
> 取证环境：仓库 `sokonanoda-lang`，`Cargo.toml` 版本 **0.61.0**、`editor/vscode/package.json`
> 版本 **0.61.0**；取证方式为**只读**（`read` / `grep` / `node -e` / 真二进制跑判卷），
> 未修改任何仓库文件。凡不能证实的一律写 **未证实**。
> 每条事实带 `file:line`；命令都回到源码确认存在，不凭记忆。

---

## 1. 三类用户的三条路径

### 1.1 学习者（VS Code，零工具链）

OS 假设：**macOS / Linux / Windows 均可**，只要 VS Code ≥ 1.106
（`editor/vscode/package.json:12-14` 的 `engines.vscode = "^1.106.0"`；
`editor/vscode/README.md:199-200`）。

```text
1. 在 Marketplace 安装扩展 sokonanoda-lang.sokonanoda
   （URL：https://marketplace.visualstudio.com/items?itemName=sokonanoda-lang.sokonanoda）
2. 打开任意 .sokonanoda 文件（或仓库自带的 playground.sokonanoda）
3. 完成——语言服务器与 sokonanoda CLI 已在扩展内，无需下载
```

- 扩展 id / publisher：`editor/vscode/package.json:2,6`（`sokonanoda` / `sokonanoda-lang`）；
  Marketplace 链接见 `README.md:11-12`。
- 平台包内嵌 LSP **与** CLI：`editor/vscode/README.md:3-8`、`editor/vscode/README.md:123-125`。
- 支持的平台（平台包）：macOS arm64/x64、Linux x64/arm64、Alpine x64/arm64、
  Windows x64/arm64 —— `editor/vscode/README.md:5-6`。
- 完全离线可用（有平台包的平台）：`editor/vscode/README.md:144-145`。
- **没有任何命令行步骤**，因此本路径无需验证命令。

### 1.2 AI-agent 用户 / 命令行用户（零 cargo）

OS 假设：**macOS / Linux / Windows**（启动器零依赖 Node、无 shell：
`scripts/soko:19-20`）。前提是机器上有 Node（DSH 环境自带）。

```bash
# ① 拿仓库（agent 的入口文件是 AGENTS.md）
git clone https://github.com/ColorlessBoy/sokonanoda-lang
cd sokonanoda-lang

# ② 环境：版本锁定的 CLI + LSP → 缓存（幂等；零 cargo）
scripts/soko setup

# ③ 就绪诊断；0 = 就绪，3 = 未就绪
scripts/soko doctor --json

# ④ 判卷（全量 JSON 事件流）
scripts/soko grade playground.sokonanoda --json

# ⑤ 提问式查询（单 JSON 对象）：某处还差什么
scripts/soko query state --file playground.sokonanoda --line 327 --col 4

# ⑥ 仓库版本 / 解析来源 / 缓存标记
scripts/soko version --json

# ⑦ 缓存过期时刷新；0 = 写成了，3 = 没写成（stderr 给 download 原因）
scripts/soko update
```

逐条核验：

| 命令 | 是否存在 | 证据 |
|---|---|---|
| `scripts/soko setup` | ✅ | `scripts/soko:721-791`（`setup`/`update` 共用 case，`force = command === 'update'` 在 `:733`） |
| `scripts/soko doctor --json` | ✅ | `scripts/soko:832-872`；退出码 `ready ? 0 : 3` 在 `:871` |
| `scripts/soko grade … --json` | ✅ | `scripts/soko:967-990` 原样转发给 CLI；CLI 侧 `crates/cli/src/env/mod.rs:181-212`（`grade` 强制 `json: true`） |
| `scripts/soko query state --file … --line … --col …` | ✅ | `crates/cli/src/query.rs:73-96`（`--file`/`--line`/`--col`/`--offset`）；op 列表 `crates/cli/src/help.rs:19-30` |
| `scripts/soko version --json` | ✅ | `scripts/soko:793-830`（`--json` 字段见 `:799-814`） |
| `scripts/soko update` | ✅ | `scripts/soko:721-791`，`if (force)` 分支 `:765-788` |

**不克隆仓库的等价路径**（README 的“手动下载”片段，macOS/Linux 示例）：

```bash
V=$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)   # 必须来自本 checkout，不能硬编码
TARGET=aarch64-apple-darwin   # linux: x86_64-unknown-linux-gnu / aarch64-unknown-linux-gnu; win: x86_64-pc-windows-msvc
BIN="$HOME/.local/share/sokonanoda/bin"; mkdir -p "$BIN"
for pkg in sokonanoda-cli sokonanoda-lsp; do
  curl -fsSL "https://github.com/ColorlessBoy/sokonanoda-lang/releases/download/v${V}/${pkg}-${TARGET}.tar.gz" \
    | tar xz -C "$BIN"
done
"$BIN/sokonanoda" --json your-file.sokonanoda
```

来源 `README.md:91-100`。资产名 `sokonanoda-cli-<triple>.tar.gz` /
`sokonanoda-lsp-<triple>.tar.gz` 与 `release.yml` 一致（见 §5）。

**POSIX 安装脚本**（macOS/Linux；`--version` 或 `SOKONANODA_VERSION` **必填**）：

```bash
sh scripts/install.sh --version v0.61.0
# 或
SOKONANODA_VERSION=v0.61.0 sh scripts/install.sh
```

- 脚本存在：`scripts/install.sh`；用法在 `scripts/install.sh:13-24`，必填校验在
  `scripts/install.sh:80-83`，OS/arch → triple 映射在 `scripts/install.sh:89-120`。
- 装到 `${SOKONANODA_HOME:-$HOME/.local/share/sokonanoda}/bin`：`scripts/install.sh:28`。
- README 里的调用形式是 `SOKONANODA_VERSION="v${TAG}" sh scripts/install.sh`（`README.md:106-109`）。

**包管理器备选**：README 声称 `cargo binstall sokonanoda-cli` 与
`mise github:ColorlessBoy/sokonanoda-lang` 可用（`README.md:116-119`）。
**未证实**：全仓没有任何 `[package.metadata.binstall]` 配置（`grep -rn binstall --include=*.toml` 无命中），
两者依赖第三方工具自身的 GitHub Release 启发式，本仓库未提供机器可验证的证据。

### 1.3 贡献者（需要 Rust）

OS 假设：macOS / Linux（Windows 亦可，但 `cargo fmt` 门禁覆盖教学 crates）。

```bash
git clone https://github.com/ColorlessBoy/sokonanoda-lang
cd sokonanoda-lang

# ① 源码构建（CLI + LSP）
cargo build --release --locked -p sokonanoda-cli -p sokonanoda-lsp
export PATH="$PWD/target/release:$PATH"

# ② 一条命令跑完 CI 门禁
#    = cargo fmt + clippy + test + playground 锚点 + 课程门禁 + 缺口台账门禁
#    后两步要 python3；探不到就 exit 3，绝不静默跳过
scripts/soko gate
```

- 构建命令：`skills/sokonanoda-dev/SKILL.md:124-126`。
- `gate` 的组成：`scripts/soko:874-944`（python3 探测 `:646-663`、缺 python3 → exit 3
  在 `:884-890`；cargo gate `:895`；课程门禁 `:902-923`；`scripts/gap.py check` `:933-943`）。
- `gate` 内部的 cargo 三步：`crates/cli/src/env/mod.rs:256-270`
  （`fmt -p sokonanoda-front -p sokonanoda-cli -p sokonanoda-lsp -- --check`、
  `clippy --workspace --all-targets`、`test --workspace --locked`），
  随后用**运行中二进制内嵌的编译器**编译 `playground.sokonanoda`（`:285-301`）。
- **版本一致性守卫**：二进制版本 ≠ 仓库版本时 `gate` 直接 exit 3 并说明
  “anchor 结果不可信”（`crates/cli/src/env/mod.rs:244-255`）。

手动的等价命令（`AGENTS.md:144-148`、`skills/sokonanoda-dev/SKILL.md:78-88`）：

```bash
cargo fmt -p sokonanoda-front -p sokonanoda-cli -p sokonanoda-lsp -- --check
# 禁止 cargo fmt --all：会重排冻结内核（AGENTS.md:145）
cargo clippy --workspace --all-targets
cargo test --workspace --locked
cd editor/vscode && npm run test:unit
python3 courses/set-theory/tools/check.py --selftest
python3 scripts/gap.py selftest && python3 scripts/gap.py check
```

扩展改动的三层（`AGENTS.md:188-190`）：

```bash
scripts/soko gate
node editor/vscode/test-extension-host.js
SOKO_VSCODE_TEST_VERSION=1.138.0 scripts/vscode-e2e.sh
```

### 1.4 三条路径的边界（官网必须讲清）

| | 需要 Rust | 需要 Node | 需要网络 | 主要入口 |
|---|---|---|---|---|
| 学习者（VS Code） | 否 | 否 | 仅安装扩展时需要 | Marketplace |
| AI-agent / CLI | 否 | 是（`scripts/soko`） | 首次 `setup` 需要 | `scripts/soko` |
| 贡献者 | 是 | 是（扩展测试） | 是 | `cargo` + `scripts/soko gate` |

硬规则：**用户/agent 路径零工具链依赖**（`AGENTS.md:125-127`，REQUIREMENTS §2 第 9 条）。

---

## 2. VS Code 扩展能力全表

### 2.1 计数（程序化提取）

计数命令（原文照抄，可在仓库根直接跑）：

```bash
node -e '
const p=require("./editor/vscode/package.json"); const c=p.contributes;
console.log("commands",c.commands.length);
console.log("keybindings",c.keybindings.length);
console.log("views",Object.values(c.views).flat().length);
console.log("settings",Object.keys(c.configuration.properties).length);
console.log("activationEvents",p.activationEvents.length);
'
```

结果：**commands 14 · keybindings 5 · views 4 · settings 3 · activationEvents 2**。
扩展版本 `0.61.0`，`engines.vscode ^1.106.0`，`extensionKind: ["workspace"]`
（`editor/vscode/package.json:5,12-14,19-21`）。

### 2.2 贡献的命令全表（14 条）

| # | command id | 标题 | 键位 | package.json |
|---|---|---|---|---|
| 1 | `sokonanoda.showStatus` | sokonanoda: show exercise status | `alt+s` | `:60-62`；键位 `:255-257` |
| 2 | `sokonanoda.status` | sokonanoda: show declaration status | — | `:65-67`；命令面板隐藏 `:170-171` |
| 3 | `sokonanoda.nextHole` | sokonanoda: go to next hole | `alt+n` | `:69-71`；键位 `:260-262` |
| 4 | `sokonanoda.previousHole` | sokonanoda: go to previous hole | `alt+shift+n` | `:74-76`；键位 `:265-267` |
| 5 | `sokonanoda.goals.refresh` | sokonanoda: refresh exercise panel | — | `:79-81`；面板标题栏 `:192-195` |
| 6 | `sokonanoda.courseRefresh` | sokonanoda: 刷新课程地图 | — | `:84-86`；标题栏 `:207-210` |
| 7 | `sokonanoda.revealRange` | sokonanoda: reveal range | — | `:89-91`；命令面板隐藏 `:174-175` |
| 8 | `sokonanoda.revealHint` | 揭示下一条提示 | — | `:93-95` |
| 9 | `sokonanoda.restartServer` | restart server | — | `:98-100` |
| 10 | `sokonanoda.openInfoview` | sokonanoda: 打开目标面板 (Infoview) | — | `:104-106`；标题栏 `:202-205` |
| 11 | `sokonanoda.doctor` | doctor: 诊断服务器与版本 | — | `:110-112` |
| 12 | `sokonanoda.build` | build（编译当前文件/工作区，预热缓存） | `alt+b` | `:116-118`；键位 `:270-272`；项目视图标题栏 `:217-219` |
| 13 | `sokonanoda.rebuild` | rebuild（清空编译缓存后重编译） | `alt+shift+b` | `:121-123`；键位 `:275-277` |
| 14 | `sokonanoda.project.refresh` | sokonanoda: refresh project view | — | `:126-128`；标题栏 `:212-215` |

注册处（全部 14 条一一对应）：`editor/vscode/extension.js:1646-1678`
（`registerCommands()` 在 `:1607`，`activate()` 在 `:1796` 调用）。

键位全部带 `when: editorTextFocus && editorLangId == sokonanoda`
（`editor/vscode/package.json:257,262,267,272,277`）——**只在 `.sokonanoda` 文件里生效**。

命令面板隐藏（`when: false`）的 5 条：`status` / `revealRange` / `goals.refresh` /
`courseRefresh` / `project.refresh`（`editor/vscode/package.json:168-188`）。

### 2.3 视图与容器（4 个视图 / 2 个容器）

| 视图 id | 名称 | 位置 | 类型 | when | package.json |
|---|---|---|---|---|---|
| `sokonanoda.goals` | 练习 | Explorer | tree | `resourceLangId == sokonanoda` | `:143-146` |
| `sokonanoda.courseMap` | 课程 | Explorer | tree | `resourceLangId == sokonanoda` | `:148-151` |
| `sokonanoda.project` | 项目 | Explorer | tree | `resourceLangId == sokonanoda` | `:153-156` |
| `sokonanoda.infoview` | 目标面板 (Infoview) | 自有容器（右侧辅助侧栏） | webview，`visibility: visible` | 无 | `:160-165` |

容器：`viewsContainers.secondarySidebar` 的 `sokonanoda`（标题 `sokonanoda`，
图标 `$(mortar-board)`）—— `editor/vscode/package.json:132-138`。
Infoview 落在右侧辅助侧栏、需要 VS Code ≥ 1.106（`editor/vscode/README.md:51-60,199-200`）。

视图创建：练习树 `extension.js:1694-1697`、项目树 `:1705-1709`、
课程树 `:1790-1794`、Infoview webview `:1719-1724`。

### 2.4 设置（3 个）

| 设置 | 类型 | 默认 | 作用 | package.json |
|---|---|---|---|---|
| `sokonanoda.serverPath` | string | `""` | 自定义 `sokonanoda-lsp` 路径；**除非打开 `serverOverride`，否则被忽略**；受限工作区忽略 | `:226-231` |
| `sokonanoda.serverOverride` | boolean | `false` | 打开后才恢复顺序 `serverPath → SOKONANODA_LSP_BIN → bundled → workspace target → download cache` | `:232-237` |
| `sokonanoda.trace.server` | string（enum off/messages/verbose） | `"off"` | 把 LSP 流量写进 `sokonanoda` output channel | `:238-248` |

另有一项 `configurationDefaults`：`"[sokonanoda]": { "editor.unicodeHighlight.ambiguousCharacters": false }`
（`editor/vscode/package.json:250-255`），即希腊字母 `α`/`β` 不显示易混字符框
（`editor/vscode/README.md:114-115`）。

受限工作区（Restricted Mode）：`capabilities.untrustedWorkspaces` 为 `limited`，
`restrictedConfigurations` 正是上面两个 server 设置，描述为“受限模式下总是用内置服务器”
（`editor/vscode/package.json:27-36`）。

### 2.5 激活时发生什么

`activationEvents`：`onLanguage:sokonanoda`、`onView:sokonanoda.infoview`
（`editor/vscode/package.json:22-25`）。

`activate()` 的顺序是**刻意设计的**（注释在 `extension.js:1683-1692`：所有视图/命令必须在
第一个 `await` 之前注册，否则 webview 容器 `hideIfEmpty` 会隐藏、面板空白）：

1. 建练习树 + 状态栏（`extension.js:1693-1700`）；
2. 建项目树（`:1704-1709`）；
3. 注册 Infoview webview provider + 主题变更监听（`:1715-1724`）；
4. 挂编辑器事件：切换活动编辑器、**选区变化去抖 200ms** 后请求 `soko/stateAt`、
   诊断变化去抖 150ms 后刷新练习树/项目树/声明表（`:1726-1785`）；
5. 建课程树并立即 `refresh()`（`:1789-1797`）；
6. 注册 14 条命令（`:1796`）；
7. **异步续段（fire-and-forget）**：`resolveServerForStart()` → `new LanguageClient(...)`
   → `client.start()` → 刷新面板 + `loadProject()` → **自动跑一次只读 `doctor`**
   （`:1814-1857`）。

LSP 客户端选择器：`documentSelector: [{ language: "sokonanoda", scheme: "file" }]`，
并监听 `**/*.sokonanoda` 文件事件（`extension.js:1832-1835`）。
测试模式（`ExtensionMode.Test`）下 `activate()` 返回三个 tree provider 供集成测试断言
（`extension.js:1799-1812`）。

### 2.6 bundled-LSP 故事（官网的核心卖点）

**用户视角**：装完扩展即可用——LSP 与 CLI 都在 VSIX 里，无下载、无 Rust
（`editor/vscode/README.md:3-8`）。

**服务端解析顺序**（扩展内，`editor/vscode/README.md:127-142`）：

1. **bundled**：`<extensionPath>/bin/<target>/sokonanoda-lsp[.exe]`
   —— 存在即用；先做 `X_OK` 检测，缺可执行位时 best-effort `chmod 0o755`；
2. 版本锁定的下载缓存 —— **只给没有内置包的平台**（如 Linux armhf），
   且始终钉到扩展自己的 release tag，**从不 `latest`**。

`sokonanoda.serverPath` / `SOKONANODA_LSP_BIN` 与 workspace 的
`target/{debug,release}` 构建**默认被忽略**，除非打开 `sokonanoda.serverOverride`
（`editor/vscode/README.md:136-142`）。设计动机：防止旧本地构建静默覆盖内置服务器
（“服务器还是 0.26.0”这类事故）。

**打包机制**（`docs/design/bundled-lsp.md`）：

- 方案选型：per-target VSIX + universal fallback（`bundled-lsp.md:114-119`）；
- 打包布局 `bin/<target>/{sokonanoda-lsp,sokonanoda}[.exe]`（`bundled-lsp.md:125-128`）；
- **exec 位必须在 Linux/macOS 上打包**（Windows 上 `vsce package` 会丢 unix mode，
  `bundled-lsp.md:24-26`、`skills/sokonanoda-ci/SKILL.md:61`）；
- 覆盖 8 个平台，`linux-armhf` 未覆盖 → universal 版本锁定下载兜底
  （`bundled-lsp.md:240-245`）；
- 体积：release LSP 实测 3.4 MB，每平台 VSIX ≈ 4 MB（`bundled-lsp.md:27-29`）。

**doctor 命令**：`sokonanoda: doctor` 显示当前用的是哪个服务器、`source`
（bundled / override / cache）、运行版本 vs 扩展版本（`editor/vscode/README.md:140-142`）。
激活后自动跑一次（`extension.js:1855-1857`）。

### 2.7 其它用户可见能力（README 归纳）

- 悬停任意表达式给类型；部分应用打印真实 binder 名（`editor/vscode/README.md:29-32`）；
- **签名也受检**：`theorem t : 3 := sorry` 报 `kernel-expected-sort`，
  `theorem` 的类型必须是 `Prop`（`editor/vscode/README.md:23-28`）；
- 警告：`reserved-declaration-name`、`redundant-sorry`（`editor/vscode/README.md:33-39`）；
- 目标视图 + `alt+n`/`alt+shift+n` 跳洞（`editor/vscode/README.md:45-50`）；
- Infoview：多目标列、`by k/n` 进度、声明列表、实时服务器版本（`:51-60`）；
- 提示阶梯：画布 `-- soko:hint` 指令，逐条揭示（`:61-62`）；
- 用户自定义记法 `infix:N` / `infixl:N` / `infixr:N` / `notation`（`:63-69`）；
- 多文件项目 `import` + 可选 `sokonanoda.toml`，跨文件跳定义/引用/改名（`:70-80`）；
- 项目树（`:81-90`）、课程地图 11 单元（`:91-97`）；
- 补全、跳定义、改名、找引用、inlay hints、code actions、语义高亮、折叠、
  smart select（`:99-115`）；
- 就地重启语言服务器 `sokonanoda: restart server`（`:111-113`）；
- 编译缓存两条命令 `build`（`alt+b`）/`rebuild`（`alt+shift+b`）（`:172-187`）。

---

## 3. CLI 全表

### 3.1 `sokonanoda` 顶层调度

分发表在 `crates/cli/src/main.rs:96-150`；全局 flag 解析在 `:34-95`。
裸位置参数 = 判卷一个文件；`-` = stdin（`crates/cli/src/main.rs:148-149`、
`crates/cli/src/help.rs:7-8`）。

**全局 flag**：

| flag | 作用 | 源码 |
|---|---|---|
| `--json` | JSON Lines 事件流（机器/agent 视图） | `main.rs:36` |
| `--bare` | 不装任何 prelude（等价于文件里写 `-- sokonanoda:prelude none`） | `main.rs:37`；`help.rs:32-35` |
| `--force` | 只对 `setup` 有意义（强制重下） | `main.rs:38` |
| `--clean` | 只对 `build` 有意义（清空编译缓存） | `main.rs:39` |
| `--doc <file>` | 只对 `watch` 有意义（单文档） | `main.rs:40-49` |
| `--workspace <root>` | 只对 `watch` 有意义（递归监控整棵树） | `main.rs:64-73` |
| `--root <dir>` | `import` 的模块根（默认：向上最近的 `sokonanoda.toml`，否则入口目录） | `main.rs:50-59`；`help.rs:43-46` |
| `--no-project` | 忽略 `sokonanoda.toml`，模块根 = 入口文件所在目录 | `main.rs:60`；`help.rs:47-49` |
| `--all` | 只对 `course` 有意义（递归找所有 `course.json`） | `main.rs:61-63` |
| `-h` / `--help` | 打印帮助并 exit 0 | `main.rs:74-77` |
| `-V` / `--version` | 打印 `sokonanoda <CARGO_PKG_VERSION>` 并 exit 0 | `main.rs:78-81` |

形状检查：`--doc`/`--workspace` 只能配 `watch`（`main.rs:86-91`）；
`--all` 只能配 `course`（`main.rs:92-95`）；`query` 不接受 `--json`（`main.rs:107-110`）；
`repl` 与 `lsp` 同样拒绝 `--json`（`main.rs:112-116`、`:126-129`）。

### 3.2 `sokonanoda` 子命令全表

| 子命令 | 一句话用途 | 真实调用示例 | 源码 |
|---|---|---|---|
| `<file>` / `-` | 判卷一个文件或 stdin（人类文本） | `sokonanoda examples/lesson-01.sokonanoda` | `main.rs:148-149` |
| `--json <file>` | 同上，但输出 JSON Lines 事件流 | `sokonanoda --json playground.sokonanoda` | `main.rs:148-149`；`crates/cli/src/json_report.rs:27-95` |
| `version [--json]` | 仓库版本 + target + 缓存标记是否匹配 | `sokonanoda version --json` | `crates/cli/src/env/mod.rs:32-79` |
| `doctor [--json]` | 只读就绪诊断；**0 = 就绪，3 = 未就绪** | `sokonanoda doctor --json` | `env/mod.rs:82-144` |
| `setup [--force]` | 幂等下载版本锁定的 CLI + LSP 到缓存；失败 exit 3 | `sokonanoda setup` | `env/mod.rs:147-172` |
| `update` | 强制把缓存刷到本二进制版本（= `setup --force`） | `sokonanoda update` | `env/mod.rs:175-178` |
| `grade <file...>` | 对一个或多个文件跑 `--json` 批量判卷 | `sokonanoda grade playground.sokonanoda` | `env/mod.rs:181-212` |
| `gate` | 贡献者门禁：cargo fmt/clippy/test + playground 锚点 | `sokonanoda gate` | `env/mod.rs:244-307` |
| `query <op>` | 内核真相的**单 JSON 对象**视图（7 个 op） | `sokonanoda query check --file playground.sokonanoda` | `crates/cli/src/query.rs`；op 列表 `help.rs:19-30` |
| `build [--clean] [<file>\|<dir>…]` | 预热（或清空）共享编译缓存 | `sokonanoda build --clean` | `main.rs:111`；`crates/cli/src/build.rs` |
| `repl` | 交互式 REPL（`#check`/`#reduce`/`#print`/`#prove`） | `sokonanoda repl` | `main.rs:112-116`；`crates/cli/src/repl.rs` |
| `lsp` | 在 stdio 上跑语言服务器（编辑器 spawn 它） | `sokonanoda lsp` | `main.rs:119-129` |
| `watch [<file>\|--doc <f>\|--workspace <root>]` | 常驻监控，逐版发 JSON Lines 事件 | `sokonanoda watch playground.sokonanoda` | `main.rs:130-146`；`crates/cli/src/watch.rs` |
| `course <path>… [--all] [--json]` | 聚合课程清单成进度地图 | `sokonanoda course course/course.json --json` | `main.rs:147`；`crates/cli/src/course/` |

**`query` 的 7 个 op**（`crates/cli/src/help.rs:20-28`、`docs/protocol.md:771-779`）：

| op | 必需参数 | 一句话 |
|---|---|---|
| `check` | — | 计数 + 失败 + 告警（`--json` 事件流的单对象摘要） |
| `state` | `--line L --col C` 或 `--offset N` | 光标处的目标状态（Lean `goalsAt?` 语义） |
| `goals` | —（可选 `--probe`） | 每个声明的类型/状态/目标/洞 |
| `holes` | —（可选 `--offset N --direction next\|prev`） | 全部洞（稳定 id `declName:index`）+ 可选步进 |
| `hints` | `--line L --col C` 或 `--offset N` | 该声明的 `-- soko:hint` 阶梯 |
| `reduce` | `--expr E` | 表达式在内核里的范式 |
| `project` | —（可选 `--root <dir>`） | 项目闭包：根/清单/模块状态 |

`query` 的输入：`--file <path>` / `--text <src>` / stdin（`-`）；
`--compact` 输出一行（`crates/cli/src/query.rs:73-101`；`docs/protocol.md:781-782`）。

**没有“下载器子命令”**：下载能力内嵌在 `setup`/`update` 里
（`crates/cli/src/env/download.rs:22-67`），旧的 `scripts/soko.sh` 已删除
（`AGENTS.md:69-70`）。设计说明 `docs/design/binary-cli.md`、`docs/design/onboarding.md:3-6`。

### 3.3 `scripts/soko`（harness 中立启动器）

零依赖 Node、无 shell、跨平台（`scripts/soko:19-24`）。
它**自己拦截** 7 个子命令，其余原样转发（`scripts/soko:692-990`）：

| `scripts/soko` 子命令 | 行为 | 退出码 | 源码 |
|---|---|---|---|
| （无参）/ `--help` / `-h` | 打印 usage | 0 | `:696-719` |
| `setup` | 解析版本钉 → 确保 CLI + LSP 在缓存 | 0 / 3 | `:721-791` |
| `update` | 强制重下；**缓存没写成也 exit 3** | 0 / 3 | `:721-791`（`force` 分支 `:765-788`） |
| `version [--json]` | 只读：版本钉 + 解析来源 + 缓存标记（**不下载**） | 永远 0 | `:793-830` |
| `doctor [--json]` | 只读就绪诊断 | 0 / 3 | `:832-872` |
| `gate` | CLI 的 cargo 门禁 + 课程门禁 + 缺口台账门禁 | 0 / 传播 / 3 | `:874-944` |
| `mcp` | 跑 `dsh/mcp/server.js`（MCP stdio server） | 0 / 3 | `:946-965` |
| `lsp` | 解析 LSP 二进制并 exec（**丢掉 `lsp` 这个词**，stdout 保持纯帧流） | 0 / 3 | `:967-990` |
| 其余 | 原样转发给 `sokonanoda` CLI（`grade`/`query`/`course`/`watch`/`repl`/`build`/裸文件…） | 传播 / 3 | `:967-990` |

用法示例：

```bash
scripts/soko setup
scripts/soko doctor --json
scripts/soko grade playground.sokonanoda --json
scripts/soko query check --file playground.sokonanoda
scripts/soko gate
scripts/soko mcp
```

**二进制解析顺序**（实现链，`scripts/soko:543-588`；执行门 `canExec()` 在 `:603-608`）：

1. **显式 override** `$SOKONANODA_BIN` / `$SOKONANODA_LSP_BIN`（`:554`、`:689-690`）；
2. **版本匹配的仓库构建** `target/{release,debug}`，按 mtime 取新（`:344-357`），
   且必须 `--version` 自报版本符合钉（`:530-540`）；
3. **缓存**（标记必须等于钉，`:557-559`）；标记不符 → `cache(STALE: …)`（`:566`），
   无钉 → `cache(unverified: …)` / `cache(unknown repo version: …)`（`:560-565`）——两者都被拒；
4. **按版本钉锁定下载**，仅当钉是完整 `x.y.z`（`:578-581`），URL 永远带 `v${version}`
   （`:464`）。

> ⚠️ **文档与实现不一致（必须诚实标注）**：`AGENTS.md:58-61`、`skills/README.md:33-35`、
> `dsh/README.md:169-170` 都写解析链含“**VS Code 扩展自带**”这一环，
> 但 `scripts/soko` 里 `extensionServer()`（`:385-424`）**从未被调用**——
> `grep -n extensionServer scripts/soko` 只命中声明行；`canExec()` 里的
> `result.source === 'extension'`（`:605`）是死分支。
> **实际运行时链只有 override → repo-build → cache → download 四步**。
> 对照：opencode 插件**确实**实现了这一环（`.opencode/plugins/sokonanoda.ts:291-302`，
> `resolveServer()` 里 `extensionServer(target)` 在 `:300-301` 被调用）。

**版本钉源链**（`scripts/soko:210-296`）：

1. `$SOKONANODA_VERSION`（`:223`）；
2. `<repo>/sokonanoda-version.txt`（第一行有效行；跳过 `#` 与空行，`:138-149`）；
3. `<repo>/sokonanoda.toml` 的 `requires`（正则 `^\s*requires\s*=\s*["']([^"']+)["']`，`:154-162`）
   —— 完整 `x.y.z` 才算钉，`0.58` 只是约束（`:131-134`）；
4. `<repo>/Cargo.toml` 的 `version = "…"`（`:164-168`）。

规则：**所有出现的源必须一致（比较 major.minor）**，否则指名文件报错（`:280-292`）；
只有约束没有锚 → 报“不是 release tag”（`:258-267`）；一个源都没有 → 列出找过的文件（`:268-276`）。
**解析不出期望版本就绝不 exec**：转发命令/`lsp`/`gate` 走 `refuseUntrusted()` → exit 3
（`:626-641`、`:975-980`）。

**缓存布局**：`$SOKONANODA_CACHE_DIR` 或 `~/.local/share/sokonanoda/bin`
（Windows 用 `%USERPROFILE%`）（`:66-70`）；标记文件 `<base>.version`，内容
**`"<version> <target>\n"`**（`:304-306`、`:337-339`），校验时 target 必须相等、
版本 major/minor 相等、完整钉还要求 patch 精确相等（`:326-335`）。

**下载**：先 `curl`（`-fsSL --retry 2 --connect-timeout 20 --max-time 600`，`:435-439`），
失败回落 Node `fetch`（`:444-448`）——因为 Node `fetch` 忽略 `HTTPS_PROXY` 而 curl 认
（注释 `:430-432`）；解包用 `tar xzf`（`:473`），只取同名二进制，非 win32 补 `chmod 0o755`
（`:485-491`），最后写标记（`:492`）。

**相关环境变量**：`SOKONANODA_BIN`、`SOKONANODA_LSP_BIN`、`SOKONANODA_OFFLINE=1`、
`SOKONANODA_CACHE_DIR`、`SOKONANODA_RELEASE_BASE`、`SOKONANODA_VERSION`、
`SOKONANODA_NO_CURL`（`:689-690,104-106,66-70,57-59,223,434`）；
`HTTPS_PROXY` 脚本自身不读，靠 curl（提示文案 `:756`、`:784`）。
**`releases/latest` 在 `scripts/soko` 里不存在**（唯一含 `latest` 的是注释 `:22`）。

### 3.4 `grade --json` 的确切事件形状

实现：`crates/cli/src/json_report.rs:27-95`；契约文档 `docs/protocol.md:48-96`。
**每行一个 JSON 对象**，全部带 `type` 与 `human`：

| `type` | 载荷字段 | 触发 |
|---|---|---|
| `decl.checked` | `name` | 声明通过完整内核（`json_report.rs:31-35`） |
| `example.checked` | — | 填好的 `example` 通过内核（`:36-39`） |
| `expr.typed` | `text`, `inferred_type`, `span` | `#check`（`:40-49`） |
| `expr.reduced` | `text`, `value`, `span` | `#reduce`（`:50-56`） |
| `decl.printed` | `name`, `text` | `#print`（`:57-62`） |
| `exercise.open` | `name`（可缺） | 值位是 `sorry` 且**签名合法**的开放练习（`:63-72`） |
| `warning` | `code`, `message`, `hint`, `span` | 非致命 lint，**永不影响退出码**（`:75-84`） |
| `diagnostic` | `stage`, `code`, `message`, `hint`, `span` | 任何错误（`:85-94`） |

`span` 形状（`json_report.rs:10-23`）：

```json
{"start":{"offset":52,"line":2,"column":8},"end":{"offset":54,"line":2,"column":10}}
```

实例（本机真跑，见 §6.5）：

```json
{"type":"decl.checked","human":"checked declaration id","name":"id"}
{"type":"exercise.open","human":"exercise open (fill the sorry)","name":"ex"}
{"type":"diagnostic","stage":"kernel","code":"kernel-rejected","message":"rejected: def_eq failed","hint":"…","span":{...}}
```

**`grade` 的退出码**：0 = 全部通过；1 = 有拒绝（`crates/cli/src/env/mod.rs:207-211`）。

### 3.5 `query check` 的确切 JSON 形状

信封（每个答案都有，成功或失败）：`crates/cli/src/query.rs:213-234`、
`docs/protocol.md:784-790`：

```json
{"schema":"soko.query/1","op":"check","version":1,"ok":true,"data":{…}}
{"schema":"soko.query/1","op":"check","version":1,"ok":false,
 "error":{"code":"not-parsable","message":"…"}}
```

`check` 的 `data`（`docs/protocol.md:773`）：

```json
{"version":1,
 "counts":{"decl_checked":30,"example_checked":2,"exercise_open":4,
           "expr_typed":0,"expr_reduced":0,"decl_printed":0},
 "failed":[{"name":null,"code":"unexpected-token","message":"…",
            "start":0,"end":0,"start_line":1,"start_col":1,"end_line":1,"end_col":1}],
 "warnings":[{"code":"redundant-sorry","message":"…","hint":"…",
              "start":20818,"end":20823,"start_line":333,"start_col":3,
              "end_line":333,"end_col":8}]}
```

要点（`docs/protocol.md:773-802`）：

- **两套坐标都给**：`start`/`end` 是**字节** offset（入口文件），
  `start_line`/`start_col`/`end_line`/`end_col` 是 1-based 行列 —— 消费者不用自己数；
- `failed[]` **同时**承载内核拒绝与 parse 诊断；解析失败时 `counts` 全 0、
  `failed[]` 恰好一条 parse 诊断、`ok` 仍是 `true`、**退出码 1**（与 `grade` 同口径）；
- 依赖模块的错**不**进 `failed[]`，只以 `import-dependency-failed` 落在入口的 `import` 行；
- 退出码：`0` = 答上了（开放 `sorry` 是合法状态）、`1` = 有拒绝（内核拒绝**或**解析失败）、
  `2` = 用法错误（`crates/cli/src/query.rs:8-10`）；
- **`ok:false` 不是空结果**：`error.code` ∈ `not-parsable` / `outside-declarations` /
  `position-out-of-range`（`docs/protocol.md:792-797`）。

---

## 4. AI agent 集成

### 4.1 角色技能（3 个）+ 运维命令（2 个）

正文唯一源在 `skills/<name>/SKILL.md`（`skills/README.md:5-7`）。

| 技能 | 给谁 | 何时加载 | 确切命令 | 正文 |
|---|---|---|---|---|
| `sokonanoda-teacher` | 当老师的 agent | 用户想学 Lean 式证明 / 要判卷 / 要画布状态 | `scripts/soko grade playground.sokonanoda --json`；`scripts/soko query state --file playground.sokonanoda --line L --col C` | `skills/sokonanoda-teacher/SKILL.md`（356 行）；frontmatter `:1-4` |
| `sokonanoda-dev` | 接手开发的 agent | 加语法 / 动编译器 / 动 CI 文档 | `scripts/soko gate`；`cargo test -p sokonanoda-front compile::tests::` | `skills/sokonanoda-dev/SKILL.md`（174 行）；frontmatter `:1-4` |
| `sokonanoda-ci` | 推代码/发布/查 CI 的 agent | push、tag、release、Actions 排错 | `scripts/soko gate; echo "EXIT=$?"`；`gh run view <id> --json jobs` | `skills/sokonanoda-ci/SKILL.md`（195 行）；frontmatter `:1-4` |
| `sokonanoda-update` | **人工**运维 | 环境过期（`doctor` 报 `ready:false` / 启动器 exit 3） | `scripts/soko update` | `skills/sokonanoda-update/SKILL.md`（79 行） |
| `sokonanoda-doctor` | **人工**运维 | 会话开始 / 启动器拒绝运行 | `scripts/soko doctor --json` | `skills/sokonanoda-doctor/SKILL.md`（41 行） |

前三个是**角色技能**（模型按任务加载）；后两个是**运维命令**：DSH 入口带
`disable-model-invocation: true`，不进模型目录（`skills/README.md:17-19`、
`.agents/skills/sokonanoda-update/SKILL.md:5-6,40-43`）。

**teacher 的五条不可违反规则**（`skills/sokonanoda-teacher/SKILL.md:21-31`）：
判定永远走 kernel / `sorry` 是合法开放状态 / 出题必配 2–3 条 `-- soko:hint` 阶梯 /
解答钥匙只在明确要求或卡壳 ≥3 轮时揭示 / 按学生实时适配。

**teacher 的环境与判卷命令**（`skills/sokonanoda-teacher/SKILL.md:45-74`）：

```bash
scripts/soko setup
scripts/soko update
scripts/soko version --json
scripts/soko doctor --json
scripts/soko grade playground.sokonanoda --json
scripts/soko query check  --file playground.sokonanoda
scripts/soko query state  --file playground.sokonanoda --line 327 --col 4
scripts/soko query holes  --file playground.sokonanoda
scripts/soko query hints  --file playground.sokonanoda --line 323 --col 3
scripts/soko query goals  --file playground.sokonanoda
```

**dev 的接手顺序**（`skills/sokonanoda-dev/SKILL.md:10-20`）：`AGENTS.md` →
`REQUIREMENTS.md` → `docs/HANDOVER.md` → `STATUS.md` → `ROADMAP.md` §10 →
`docs/README.md` → `docs/architecture.md` → `docs/design/`。

**skill 的守护测试**：`crates/cli/tests/skill.rs`（正文 frontmatter + 引用路径 + 事件词汇）、
`crates/cli/tests/dsh.rs`（DSH 入口双向无孤儿 + 启动器解析链 + patch 形状）
（`skills/README.md:6-7,94-100`）。

### 4.2 DSH 侧技能入口（`.agents/skills/`）

5 个薄入口文件，正文仍指回 `skills/`：

```text
.agents/skills/sokonanoda-{teacher,dev,ci,update,doctor}/SKILL.md
```

DSH 只扫 `<repo>/.dsh/skills` 与 `<repo>/.agents/skills`（不递归）
（`dsh/README.md:118-122`）。**技能名本身就是斜杠命令**：
`/sokonanoda-teacher`、`/sokonanoda-dev`、`/sokonanoda-ci`、
`/sokonanoda-update`、`/sokonanoda-doctor`（`dsh/README.md:25-28`）。
DSH 命令名文法不允许 `/`，所以 opencode 的 `/sokonanoda/update` 在 DSH 侧写作
`/sokonanoda-update`（`skills/README.md:51-52`）。

### 4.3 MCP 工具（7 个）

声明处：`dsh/mcp/server.js:182-300` 的 `TOOLS` 对象（键：`check`、`state`、`goals`、
`holes`、`hints`、`project`、`reduce`）；映射表在 `dsh/README.md:68-76`。

| MCP 工具名 | 转发到 | 关键参数 | 源码 |
|---|---|---|---|
| `mcp__sokonanoda__check` | `query check` | `file?`, `text?` | `server.js:183-198` |
| `mcp__sokonanoda__state` | `query state` | `file`/`text`, `line`, `character`（1-based UTF-16） | `:199-213` |
| `mcp__sokonanoda__goals` | `query goals` | `file`/`text`, `probe?` | `:214-229` |
| `mcp__sokonanoda__holes` | `query holes` | `file`/`text`, `offset?`, `direction?`（`next`/`prev`） | `:230-252` |
| `mcp__sokonanoda__hints` | `query hints` | `file`/`text`, `line`, `character` | `:253-267` |
| `mcp__sokonanoda__project` | `query project` | `file`/`text`, `root?` | `:268-282` |
| `mcp__sokonanoda__reduce` | `query reduce` | `file`/`text`, `expr`（必填） | `:283-299` |

工具名规则：DSH 暴露为 `mcp__<serverName>__<rawName>`，超 64 字符截断 + 12 位哈希，
所以原始名故意短（`server.js:176-177`；`docs/design/agent-query-channel.md:298`）。

server 的定位：**零依赖的 JSON 转发器**，`spawn` `scripts/soko query …` 并原样转发，
不含任何 Lean 逻辑（`dsh/README.md:94-98`、`docs/design/agent-query-channel.md:289-290`）。
`crates/cli/tests/dsh.rs` 钉死 MCP 工具 ↔ `query` op 一一对应
（`skills/sokonanoda-dev/SKILL.md:107-109`）。

**文档漂移（诚实标注）**：`dsh/cordis.patch.yml:22-24` 的注释仍写“**six** `mcp__sokonanoda__*` tools”
且只列 `{check,state,goals,holes,hints,reduce}`（漏 `project`）；
设计文档 `docs/design/agent-query-channel.md:311-318` 写的是 6 个带 `soko_` 前缀的原始名
（`soko_check` …），与 as-built 的 7 个裸名不一致。

### 4.4 `soko/*` 自定义 LSP 请求（6 个）

契约：`docs/protocol.md:297-505`。

| 请求 | 参数 | 返回要点 | 协议文档 |
|---|---|---|---|
| `soko/goals` | `{textDocument:{uri}, position}` | 每个声明的 `name`/`kind`/`status`/`range`/`ty`/`ty_runs`/`goal`/`goals`/`binders`/`hole`/`holes[{id,redundant}]`/`sub_goals`；回显 `uri` + `version` | `protocol.md:303-376` |
| `soko/hints` | `{textDocument, position}` | `{hints:[string]}`，来自 `-- soko:hint` 阶梯；服务端无状态 | `protocol.md:377-390` |
| `soko/stateAt` | `{textDocument, position}` | `{version, decl, goal, goal_runs, binders, goals[], span, step, total}` | `protocol.md:392-447` |
| `soko/project` | `{textDocument:{uri}}` | `{uri, version, project:{entry,root,manifest,requires_warning,modules[],diagnostics[],counts}, reason}` | `protocol.md:450-497` |
| `soko/version` | `{}` | `{version:"<CARGO_PKG_VERSION>", pid}` | `protocol.md:499-505` |
| `soko/nextHole` | `{textDocument, position, forward}` | `null` 或下一个洞的 `range` | `protocol.md:528-549` |

**消费者只有 VS Code 扩展与 opencode**；DSH 侧**无消费者**（`AGENTS.md:170-175`）。
`soko/nextHole` 有已知限制：多子目标共享一个源位置，导航是“组级”不是“目标级”，
稳定引用是 `holes[i].id`（`protocol.md:535-552`）。

### 4.5 opencode 集成

配置：`opencode.json`（35 行）。

- `skills.paths: ["./skills"]` —— `opencode.json:3-7`；
- Lean 工具链 deny：`permission.bash` 的 `lean*`/`lake*`/`elan*`/`leanc*` → `deny`
  （`opencode.json:8-16`）；
- watcher ignore + 关闭 Rust 自动格式化（`opencode.json:17-34`）。

启动插件：`.opencode/plugins/sokonanoda.ts`（352 行）。

- 解析顺序 `resolveServer()`：`SOKONANODA_LSP_BIN` → 仓库构建 → **VS Code 扩展自带**
  → 缓存（标记必须匹配）→ 版本锁定下载（`:291-316`）；
- 用 `config` 钩子把 `lsp.sokonanoda.command` 改写为原生二进制绝对路径，
  但**尊重用户自己写的配置**（`:328-343`）；
- `shell.env` 把缓存目录注入 PATH（`:344-350`）；
- 顺带把匹配版本的 CLI 也下到缓存（`:321-325`）。

命令文件 7 个（`.opencode/command/sokonanoda/{setup,update,version,doctor,check,gate,round}.md`
→ `/sokonanoda/setup` 等；`skills/README.md:67`、`docs/design/onboarding.md:68-72`）。
主 agent `teacher`（`.opencode/agent/teacher.md`，23 行，Tab 切换；`skills/README.md:68`）。
非 opencode 的 shim：`.opencode/lsp/sokonanoda-lsp.sh` → `scripts/soko lsp`
（`AGENTS.md:167-168`）。

### 4.6 DeepSeek Harness 集成

**一分钟接入**（`dsh/README.md:8-16`）：

```bash
scripts/soko setup
scripts/soko doctor --json     # 期望 "ready": true
cd <这个仓库>
SOKO_REPO=$PWD dsh web --patch ./dsh/cordis.patch.yml
```

- 工作区必须是本仓库根（`lsp` 工具只在会话 workspace 内解析 `file_path`）；
  从别处启动用 `SOKO_REPO=<绝对路径>`（`dsh/README.md:18-20`）。
- patch 里的 `!!js` 表达式**必须单行**，且**不能用 `baseUrl`**（那是 `$DSH_HOME/profiles/`）
  —— `dsh/README.md:21-23`、`dsh/cordis.patch.yml:40-49`。

`dsh/cordis.patch.yml`（85 行）声明三行：`dsh-lsp` + `dsh-lsp-stdio`
（server command = `<repo>/scripts/soko`，args `['lsp']`，`shutdownTimeoutMs: 10000`）
+ `dsh-tool-lsp` + `mcp-sokonanoda`（`@deepseek-ai/dsh-mcp-client`，`transport: stdio`，
`serverName: sokonanoda`，`args: ['mcp']`，`toolCallTimeoutMs: 60000`，
`failOnStartupError: true`）—— `dsh/cordis.patch.yml:51-85`。

常驻安装：把 `insert:` 列表整体复制进 `$DSH_HOME/profiles/web/cordis.patch.yml`
或 `$DSH_HOME/cordis.patch.yml`（`dsh/README.md:100-116`）。三个坑：
`- id:` 覆写是整块替换会丢 `!!js`；patch 不能为空或只有注释（禁用写 `[]`）；
自查用 `dsh --profile web --dump-config`（`dsh/README.md:110-116`）。

**Lean 工具链 deny**：仓库自带 `dsh/hooks/hooks.json` +
`dsh/hooks/refuse-lean-toolchain.js`（命中即退出码 2，理由回给模型），
但 DSH 的 hooks 桥只读一个进程级 `configPath`、不做项目发现，所以要在 profile 插一行
（`dsh/README.md:130-148`）。一行验证：

```bash
echo '{"tool_name":"bash","tool_input":{"command":"lake build"}}' | node dsh/hooks/refuse-lean-toolchain.js; echo "EXIT=$?"
```

匹配的是**命令位**的 `lean|lake|leanc|lean4export|elan`，所以 `grep -rn lean docs/` 不会误拦
（`dsh/README.md:156-157`）。**未启用时这条硬规则退回为文档纪律**
（`dsh/README.md:158`；`AGENTS.md:110-113`）。

### 4.7 诚实清单：DSH 侧缺什么

`docs/design/deepseek-harness.md` 的 G1–G10 差距清单（`:82-173`）：

| 编号 | 缺什么 | 后果 | 行 |
|---|---|---|---|
| G1 (P0) | 技能不能被 DSH 发现（`skills/` 不在 DSH 扫描根） | `skill` 工具目录为空，“当老师”主循环断掉 | `:86-90` |
| G2 (P1) | 七个 `/sokonanoda/*` 斜杠命令不存在 | `/sokonanoda/check`、`/round` 无人应答 | `:92-96` |
| G3 (P1) | 「老师」主 agent 不存在（DSH 无项目级 agent 目录） | 无 teacher 角色可切 | `:98-102` |
| G4 (P0') | `.sokonanoda` 没有 LSP 接线；且 DSH 的 LSP **只有 4 个只读操作**、**诊断被显式丢弃** | 内核诊断不进 agent 上下文也不进 Web UI；`soko/goals`/`stateAt`/`nextHole`/`hints`/inlay/code action/semantic tokens 全部无消费者 | `:104-115` |
| G5 (P0) | 二进制不在 PATH，DSH 无法注入 PATH | `sokonanoda` 是 `command not found`，连 `setup` 都跑不起来 | `:117-130` |
| G6 (P2) | Lean 工具链 deny 无对应物（DSH 无命令黑名单） | 硬规则从“配置强制”退化为“文档自觉” | `:132-143` |
| G7 (P2) | 文档与契约测试只认 opencode（33 个文件提到） | DSH 里的 agent 照 opencode 路径操作然后失败 | `:145-150` |
| G8 (P2) | `AGENTS.md` 的 agent 适配原则没有 DSH 条目 | DSH 侧债务累积 | `:152-156` |
| G9 (P2) | 项目级自动 provisioning 无等价路径 | 首次使用多一步人工 | `:158-165` |
| G10 (P3) | 用户级技能环境干扰（`~/.agents/skills/lean4` 断链软链） | 技能观察“不完整”，不影响本仓库技能 | `:167-173` |

**H5 backlog（远期，不做承诺，状态 ⬜ 未做）**（`:343-361`、`:425`）：

- **B1** DSH 客户端插件复刻 Infoview（需宿主插件 + host handler）；
- **B2** 诊断通道 —— **已并入** `query` + MCP（`:353-356`）；
- **B3** `SessionStart` 启动钩子自动 `setup`；
- **B4** 打包一个 `dsh` 插件 npm 包（启动器 + `tools/pre-execute` 拦截 + 自定义命令）。

**已落地但仍是能力边界的事实**：DSH 的 `lsp` 工具只能
`goToDefinition`/`findReferences`/`goToImplementation`/`hover`
（`deepseek-harness.md:58` D8）；服务端 `publishDiagnostics` 被 MVP host 明确忽略
（`:59` D9，证据 `packages/lsp/lsp-stdio/src/connection.ts:248`）；
LSP 不在任何 shipped bundle，必须 `--patch` 显式挂（`:56` D6）；
`lsp` 工具要求文件在会话 workspace root 内（`:75` D25）。
**结论：DSH 里判卷一律走 CLI `--json` / `query`，别等 LSP 诊断**
（`dsh/README.md:42-43`）。

**未核实**：DSH 是否计划投递 `publishDiagnostics`、客户端插件能否注入自定义 LSP 方法
（`deepseek-harness.md:77-78`，明确“不作为承诺”）。

### 4.8 agent 安装 prompt（现状 + 将重写）

唯一来源文件 `site/assets/agent-prompt.js`（97 行），全站共用
（`index.html`/`get-started.html`/`agents.html`/`en/index.html`），页面不得内联自己的副本
（`site/assets/agent-prompt.js:1-8`）。中文全文在 `:11-24`，英文在 `:25-38`。

当前中文 prompt 的**四步结构**（逐字要点，`site/assets/agent-prompt.js:12-24`）：

1. `git clone https://github.com/ColorlessBoy/sokonanoda-lang` + `cd sokonanoda-lang`；
2. 读 `AGENTS.md`，按 Setup 准备环境（仓库根、harness 中立、零 cargo）：
   先 `scripts/soko setup`，再 `scripts/soko doctor --json`；版本按仓库 `Cargo.toml` 锁定
   —— 禁用 `releases/latest`、不用 cargo、不碰官方 Lean 工具链；
3. 按 `skills/sokonanoda-teacher/SKILL.md` 当老师：在 `playground.sokonanoda` 上出带
   `sorry` 的练习，练习前挂 2-3 行 `-- soko:hint ...` 阶梯；DSH 里 `.agents/skills/`
   会被自动发现、直接输入 `/sokonanoda-teacher`；
4. 判卷只认内核：`scripts/soko grade playground.sokonanoda --json` 逐行读 JSON 事件
   （禁止文本比对）；问“某处还差什么”用
   `scripts/soko query state --file playground.sokonanoda --line <行> --col <列>`。

> **该 prompt 将被重写**（本轮官网重建的直接动因之一）：设计文档已把它列为待改项
> —— `docs/design/deepseek-harness.md:335-336`（安装 prompt 增加 DSH 说明）、
> `:424`（as-built 记“site 安装 prompt 改 `scripts/soko` + DSH 说明”）、
> `docs/design/agent-query-channel.md:593`（门面同步一句）。
> 重写时注意 prompt 里已经有的两处**过时措辞**：`AGENTS.md:74-80` 说明 DSH 命令写作
> `/sokonanoda-update`（不是 `/sokonanoda/update`）；技能实际落在 `.agents/skills/`
> 而非设计文档 `deepseek-harness.md:336` 写的 `.dsh/skills`。

按钮/占位符用法：`<button type="button" data-copy-agent-prompt>` +
`<pre data-agent-prompt>`（`site/assets/agent-prompt.js:6-8`）；剪贴板优先
`navigator.clipboard`，失败回退 `execCommand("copy")`（`:53-81`），
复制后 1600ms 还原文案（`:87-95`）。

---

## 5. 发布与产物

### 5.1 平台矩阵（8 个 Rust triple，精确）

`.github/workflows/release.yml:26-34` 的 `matrix.include`，**共 8 条**
（计数命令：`grep -c '^          - { os:' .github/workflows/release.yml` → 8）：

| # | 行 | runner | Rust triple | 工具 | glibc |
|---|---|---|---|---|---|
| 1 | `release.yml:27` | macos-latest | `aarch64-apple-darwin` | cargo | — |
| 2 | `release.yml:28` | macos-latest | `x86_64-apple-darwin` | cargo | — |
| 3 | `release.yml:29` | windows-latest | `x86_64-pc-windows-msvc` | cargo | — |
| 4 | `release.yml:30` | windows-latest | `aarch64-pc-windows-msvc` | cargo | — |
| 5 | `release.yml:31` | ubuntu-latest | `x86_64-unknown-linux-gnu` | zig | `.2.28` |
| 6 | `release.yml:32` | ubuntu-latest | `aarch64-unknown-linux-gnu` | zig | `.2.28` |
| 7 | `release.yml:33` | ubuntu-latest | `x86_64-unknown-linux-musl` | zig | — |
| 8 | `release.yml:34` | ubuntu-latest | `aarch64-unknown-linux-musl` | zig | — |

工具链钉：Zig `0.16.0`（`release.yml:45`）、`cargo-zigbuild@0.23.4`（`:50`）、node 22
（`:120`、`:315`）、Rust `dtolnay/rust-toolchain@stable`（`:38-40`，仓库**刻意不钉**工具链，
见 `docs/design/onboarding.md:131-134`）。

质量断言：Linux glibc 地板 `readelf` 断 `GLIBC_2.28`（`:66-75`）；
musl 断 `statically linked` + `ldd` 无动态依赖（`:76-89`）。

rust triple ↔ vsce target 映射 8 条（`release.yml:154-162`）：
`x86_64-unknown-linux-gnu:linux-x64`、`aarch64-unknown-linux-gnu:linux-arm64`、
`x86_64-unknown-linux-musl:alpine-x64`、`aarch64-unknown-linux-musl:alpine-arm64`、
`aarch64-apple-darwin:darwin-arm64`、`x86_64-apple-darwin:darwin-x64`、
`x86_64-pc-windows-msvc:win32-x64`、`aarch64-pc-windows-msvc:win32-arm64`。

### 5.2 资产清单（精确 26 个）

| 资产名模式 | 数量 | 源码 |
|---|---|---|
| `sokonanoda-lsp-<triple>.tar.gz` | **8** | `release.yml:266` |
| `sokonanoda-cli-<triple>.tar.gz` | **8** | `release.yml:277` |
| `sokonanoda-<vsce-target>.vsix`（8 平台）+ `sokonanoda-universal.vsix` | **9** | 平台包 `release.yml:172-173`；universal `:180-181` |
| `SHA256SUMS` | **1** | `release.yml:288-290` |
| **合计** | **26** | 8+8+9+1 |

- 平台 VSIX 内**同时**内嵌 LSP 与 CLI：`stage-lsp.js --rust-target <r> --binary …lsp-<r>/sokonanoda-lsp --cli-binary …cli-<r>/sokonanoda`（`release.yml:168-171`）。
- universal 包**不含** `bin/`（`release.yml:176,180-181`）。
- VSIX 冒烟：断言 `TargetPlatform`、`extension/bin/<target>/` 两个二进制、
  `file_size > 1_000_000`、非 win32 断 `mode & 0o111`（`release.yml:191-222`）。
- tarball 打包前 `chmod +x`，打包后 `tar tzvf | grep -q '^-rwx'` 断 exec 位
  （`release.yml:265-268`、`:276-279`）——因为 artifact 往返会丢 unix mode
  （`skills/sokonanoda-ci/SKILL.md:190`）。
- 上传全部 `--clobber`（`release.yml:270,281,284,290`）。
- SLSA provenance：`actions/attest-build-provenance@v2`（`:297`），覆盖
  lsp/cli/vsix/SHA256SUMS 四组 glob（`:299-303`），校验
  `gh attestation verify <file> -R ColorlessBoy/sokonanoda-lang`（`docs/RELEASE.md:140`）。
- **`docs/RELEASE.md:90` 的“共 25 个”是过期数字**；同文件 `:137` 与 `STATUS.md:59`
  的 **26** 才是对的。

### 5.3 发版流程（已全自动）

jobs 依赖图：`build`（`:22`）→ `package-vsix`（`:113`，`needs: build`）→
`github-release`（`:232`，`needs: [build, package-vsix]`）与
`marketplace-publish`（`:308`，`needs: [package-vsix]`）（`release.yml:114,233,309`）。

触发：`push: tags: ["v*"]` + `workflow_dispatch`（无 inputs）（`release.yml:6-9`）。
**dry-run**：dispatch 时创建 Release / attest / Marketplace 三步带
`if: startsWith(github.ref, 'refs/tags/')` 会被跳过，只跑 build + package-vsix
（`release.yml:248,296,324`；`docs/RELEASE.md:103-108`）。

正常发版（人只做两步）：

```bash
# ① bump 两处版本（必须一致）
#    Cargo.toml 的 [workspace.package].version
#    editor/vscode/package.json 的 version
cargo check                 # 让 Cargo.lock 跟上
git commit -am "…" && git push origin main
# ② CI 的 auto-tag 自动打 tag 并 dispatch release.yml —— 不需要手动 tag
```

- 步骤文档：`docs/RELEASE.md:77-88`；auto-tag 实现 `ci.yml:39-71`
  （版本一致性检查 `:55-60`、tag 幂等 `:61-65`、`git tag` + `push` + `gh workflow run release.yml --ref "$tag"` `:68-70`）。
- 为什么两步（推 tag + 显式 dispatch）：GITHUB_TOKEN 推的 tag 不触发其它 workflow
  （防递归），`workflow_dispatch` 是例外（`ci.yml:32-38`）。
- 手动 tag **仅应急**：`git tag vX.Y.Z && git push origin vX.Y.Z`（`docs/RELEASE.md:83-88`）。
- 删 tag 重推：`git push origin :refs/tags/vX.Y.Z`（`docs/RELEASE.md:98-99`）。
- Marketplace 发布顺序：**先 universal、后 8 个平台包**，每包重试 4 次 ×30s
  （`release.yml:327-348`；原因见 `skills/sokonanoda-ci/SKILL.md:62` 的竞态）。

### 5.4 版本如何被钉住

四条铁律（`release.yml:14-16`）：
**tag == `Cargo.toml` == `editor/vscode/package.json` == VSIX 内嵌的 LSP 二进制版本**
（`docs/RELEASE.md:10`）。

- release 侧的 version gate 在 `package-vsix` job（`release.yml:124-136`）：
  从 `Cargo.toml` 的 `[workspace.package]` 抠版本（`:127-128`），
  用 `node -p` 读 `package.json`（`:129`），断言相等（`:130-131`），
  再断言 `GITHUB_REF_NAME == v${ext_version}`（`:132-135`，`workflow_dispatch` 下跳过 tag 检查）。
- CI 侧的反向守卫在 auto-tag（`ci.yml:55-60`）——tag 之前就挡住漂移。
- 契约测试 `crates/cli/tests/extension.rs::cargo_and_extension_versions_match`
  （`docs/design/bundled-lsp.md:211-213`）。
- **下载侧**同样钉版本：`scripts/soko` 只在完整 `x.y.z` 时下载，URL 恒为
  `…/releases/download/v${version}/<pkg>-<triple>.tar.gz`（`scripts/soko:464,578`）；
  CLI 内嵌下载器同样（`crates/cli/src/env/download.rs:10-18`）；
  `scripts/install.sh` 要求显式 tag（`scripts/install.sh:18,80-83`）。

### 5.5 为什么禁用 `releases/latest`

规则出处：`AGENTS.md:87`、`README.md:113-114`、`skills/sokonanoda-teacher/SKILL.md:100-101`、
`skills/sokonanoda-update/SKILL.md:74`、`REQUIREMENTS.md:242`、
`crates/cli/src/env/download.rs:9`（注释 “Version-pinned asset URL. Never `latest`.”）。

**理由（原文）**：`README.md:113-114` —— “**Never use `releases/latest`** — always pin
`v${version}`, otherwise **a newer server would be paired with an older client**.”
`skills/sokonanoda-update/SKILL.md:74` 同义：那会让新客户端配上旧服务器。

历史根因（`docs/design/bundled-lsp.md:69-83`）：旧扩展端下载用的是
`releases/**latest**/download/sokonanoda-lsp-${target}.tar.gz`，
`.version` 标记只回答“要不要重新下载”、不锁“下载哪个版本”，于是
①旧插件 + 新 Release 会拉到比插件新的 LSP，`soko/*` 协议静默漂移；
②扩展比 Release 新时会 404 或拉旧；③同版本扩展在不同时间装到的二进制可能不同；
④tag / `Cargo.toml` / `package.json` 无一致性门禁；⑤LSP 无版本握手。

**结构性防护（代码级）**：
- `scripts/soko`：只有 `fullVersion()`（必须有 patch）能成为下载锚（`:131-134`）；
  下载仅在 `version?.patch !== undefined` 时触发（`:578`）；
  纯约束 `0.58` 被显式拒绝（`:262-263`）；缓存标记按版本比对（`:326-335`）。
- `scripts/soko` 文件里**没有** `releases/latest` 字符串（唯一含 `latest` 的是注释 `:22`）。
- `release.yml` 里**没有** `releases/latest`（14 处 `latest` 全是 runner 标签）；
  注释 `release.yml:16`：“下载 URL 锁定 v${version}，永不 latest”。
- 契约测试断言扩展的 `server.js` 永不从 `releases/latest` 下载
  （`crates/cli/tests/extension.rs:407-408,815`）。

---

## 6. 质量证据

> 本节所有数字都在本机 0.61.0 检出上**重新数过**，并给出确切命令。
> 官网引用时请连命令一起标注，避免“数字会漂”。

### 6.1 Rust 测试：1163 个测试函数（+ 6 个 doc-test）

| 数字 | 含义 | 确切命令 | 结果 |
|---|---|---|---|
| **1163** | `cargo test --workspace --locked` 实际运行的测试函数 | `cargo test --workspace --locked -- --list \| grep ': test$' \| grep -vc ' - '` | 1163 |
| 1169 | 上面再加 6 条 doc-test 行 | `cargo test --workspace --locked -- --list \| grep -c ': test$'` | 1169 |
| 6 | doc-test（3 个 target） | `cargo test --workspace --locked -- --list \| grep -c ' - '`；`… --list 2>&1 \| grep -c '^   Doc-tests'` | 6 / 3 |
| 31 | 测试二进制数（`Running` 行） | `cargo test --workspace --locked -- --list 2>&1 \| grep -c '^     Running'` | 31 |
| 25 | `crates/*/tests/*.rs` 集成测试文件 | `ls crates/*/tests/*.rs \| wc -l` | 25 |
| 1030 | `#[test]` 出现次数 | `grep -rn "#\[test\]" crates --include=*.rs \| wc -l` | 1030 |
| 134 | `#[tokio::test]`（全在 lsp） | `grep -rn "#\[tokio::test\]" crates --include=*.rs \| wc -l` | 134 |
| 0 | `#[ignore]` | `grep -rn "#\[ignore" crates --include=*.rs \| wc -l` | 0 |

分 crate（`grep -rn "#\[test\]" crates/<crate> --include=*.rs | wc -l`）：
kernel **53**、front **662**、cli **308**、lsp **7**（+134 tokio）。
1030 + 134 − 1（`crates/kernel/src/tests/natlit.rs:704` 是一条注释掉的 `//#[test]`）= **1163**。

**交叉印证**：`STATUS.md:55` 记录 v0.61.0 的
`cargo test --workspace --locked` 结果为 **1163 passed / 0 failed**。

集成测试文件清单（25 个）：cli 21 个（`cli.rs` 100 条、`extension.rs` 34、`notation.rs` 19、
`query.rs` 19、`imports.rs` 17、`project_features.rs` 14、`protocol.rs` 12、`namespace.rs` 10、
`watch.rs` 10、`course_manifest.rs` 8、`course_project.rs` 8、`dsh.rs` 8、`launcher.rs` 8、
`opencode.rs` 8、`course.rs` 6、`course_shared.rs` 4、`course_status.rs` 4、
`single_file_vs_project.rs` 4、`skill.rs` 4、`perf_project.rs` 2、`examples.rs` 1），
front 2 个（`perf.rs` 3、`perf_project.rs` 6），kernel 2 个（`memory_api.rs` 8、`arena.rs` 1）。

**过时数字（不要用）**：`docs/TESTING.md:666-673` 记 888（0.58.0）、`:632` 记 862、
`:647` 记 871；`skills/sokonanoda-dev/SKILL.md:83` 写“4 个 lib test target + 13 个集成测试文件”
——今天的实测是 25 个集成测试文件、31 个测试二进制。

### 6.2 Node / 扩展测试

| 套件 | 条数 | 命令 |
|---|---|---|
| `editor/vscode/test-server.js` | 18 | `grep -oE "\btest\(" editor/vscode/test-server.js \| wc -l`（减 1 个本地 `test()` helper） |
| `editor/vscode/test-download.js` | 7 | 同上 |
| `editor/vscode/test-webview.js` | 10 | 同上 |
| `editor/vscode/test-extension-host.js` | 14 | 同上 |
| `editor/vscode/src/test/extension.test.js`（真 VS Code） | **15** | `grep -cE "^\s*(test\|it)\(" editor/vscode/src/test/extension.test.js` |

统一入口 `npm run test:unit`（`editor/vscode/package.json` 的 `scripts.test:unit`：
`node test-server.js && node test-download.js && node test-webview.js && node test-extension-host.js`）。
真宿主 15 条与 `docs/E2E.md:42` 的“0.60.0 起 15 条”一致。

### 6.3 CI 门禁全表

`ci.yml` 触发器：`push`（无分支过滤）+ `pull_request`（`ci.yml:3-5`）。

| job | 行 | 门禁 |
|---|---|---|
| `lint` | `ci.yml:8-30` | `cargo fmt -p sokonanoda-front -p sokonanoda-cli -p sokonanoda-lsp -- --check`（`:25`）；`cargo clippy --workspace --all-targets`（`:30`） |
| `test` | `ci.yml:73-275` | workspace 测试（`:93` `cargo test --workspace --locked --no-fail-fast`）；协议一致性 `cargo test -p sokonanoda-cli --test protocol --locked`（`:115`）；课程语料 `cargo test -p sokonanoda-cli --test course --locked`（`:163`）；**课程门禁** `python3 courses/set-theory/tools/check.py --selftest` + 全量（`:190-194`）；**缺口台账门禁** `python3 scripts/gap.py selftest` + `check`（`:216-217`）；`--json` 事件良构（`:221-223`）；`npm run test:unit`（`:251`）；host VSIX 冒烟（`:258-269`） |
| `e2e` | `ci.yml:288-358` | 2 条腿：ubuntu × VS Code **1.138.0** 与 **1.106.0**（`:294-300`），`xvfb-run -a scripts/vscode-e2e.sh`（`:336`） |
| `e2e-macos` | `ci.yml:363-416` | macos-latest × 1.138.0，**只 push 到 main**（`:365,396-397`） |
| `e2e-ledger` | `ci.yml:421-503` | 合并 e2e artifact 并 `python3 scripts/e2e-merge.py --check`（`:457`），回提交台账 + 补 commit status（`:486-489`） |
| `auto-tag` | `ci.yml:39-71` | `needs: [lint, test, e2e, e2e-macos]`（`:40`）——**任一红都挡住发布** |

要点：
- **没有任何测试条数断言**（`grep -c count ci.yml` = 0）；课程判据显式“与规模无关、
  不锁计数”（`ci.yml:173`）。
- **perf 台账没有 CI 门禁**（`grep -rn "perf-ledger\|perf/ledger" .github/workflows/` 为空）；
  perf 阈值哨兵在 workspace 测试内强制，`Performance report` 步骤只收集 `^PERF` 行成 artifact
  （`ci.yml:120-153`，带 `|| true`）。
- **playground 锚点不在 CI 里**（`grep playground .github/workflows/ci.yml` 无命中）；
  它在 `sokonanoda gate` 内（`crates/cli/src/env/mod.rs:285-301`），
  命令只出现在 `docs/RELEASE.md:116`。**因此 CI 的 `test` job 与本地 `scripts/soko gate`
  并不等价**（CI 少 anchor 那一步）。

### 6.4 真 VS Code E2E 台账

- 文件：`docs/e2e/ledger.jsonl`，**37 条**（`wc -l docs/e2e/ledger.jsonl`）；
  校验命令 `python3 scripts/e2e-merge.py --check` → `e2e ledger: ok（37 条，日志齐全，按日期升序）`。
- 字段：`schema`、`kind`、`version`、`commit`、`dirty`、`date`、`host{system,machine,release}`、
  `vscode`、`tests{passed,failed,pending}`、`exit`、`server`、`lsp_sha256_16`、`log`。
- 最新一条（`:37`，逐字）：

```json
{"schema": "soko.e2e/1", "kind": "vscode-integration", "version": "0.61.0", "commit": "f3902b3ef77290ab0f37dbd54cd18a1599b4395d", "dirty": false, "date": "2026-09-19T10:56:42Z", "host": {"system": "Darwin", "machine": "arm64", "release": "25.6.0"}, "vscode": "1.138.0", "tests": {"passed": 15, "failed": 0, "pending": 0}, "exit": 0, "server": "0.61.0 (pid 36685) == 扩展 v0.61.0 (source=bundled)", "lsp_sha256_16": "ab8f7f61862b58cd", "log": "docs/e2e/logs/2026-09-19-f3902b3-vc1.138.0.log"}
```

- 分布（`python3` 扫全表）：**37 条全部 `exit=0`、`failed=0`**；`passed` 14（28 条）/ 15（9 条）；
  VS Code 1.138.0 ×25、1.106.0 ×12；版本 0.58.0 ×16、0.59.0 ×12、0.60.0 ×6、0.61.0 ×3；
  宿主 Linux ×20、Darwin ×17。
- 它证明什么（`docs/E2E.md:35-40,56-58`）：扩展在**真 VS Code** 里激活 → 拉起**真 LSP** →
  诊断 / inlay / hover / CodeLens / 重启 / Infoview / doctor / 项目树端到端成立；
  `server` 行回答“被测服务器是不是当前构建”；`lsp_sha256_16` 给被测二进制留指纹。
- 运行命令：`SOKO_VSCODE_TEST_VERSION=1.138.0 scripts/vscode-e2e.sh`
  （`AGENTS.md:138`、`docs/E2E.md:10`）；脚本做四件事（构建 release → stage → `npm test` →
  追加台账 + `latest.json` + 日志），退出码 0/1/2/3（`docs/E2E.md:18-31`）。
- 台账义务：**版本 bump / 扩展改动 / 换 VS Code 版本后各跑一次**（`docs/E2E.md:59`）。

### 6.5 性能台账

- 文件：`docs/perf/ledger.jsonl`，**14 条**（`wc -l`）；最新一条是 v0.59.0 / 2026-09-19
  （**比 e2e 台账旧**，引用时注意）。
- 套件 = `scripts/perf-ledger.sh:46-62` 的四条 cargo 调用（全部 `--locked --nocapture
  --test-threads=1`，grep `^PERF`）：`front --test perf`、`front --test perf_project`、
  `lsp --lib -- perf_`、`cli --test perf_project --release`；产出
  `docs/perf/ledger.jsonl` + `docs/perf/latest.json`。
- 12 个 case：`closure_stages`、`closure_compile_scaling`、`keystroke_recompile_closure`、
  `teaching_scale_keystroke`、`overlay_overhead`、`judge_prefix_with_imports`、
  `request_latency`、`dependency_edit_refresh`、`did_open_and_keystroke`、
  `cold_warm_check`、`dependency_edit_cache_miss`、`query_and_build`。
- 最新一条的关键数字（`:14`，逐字）：
  - 4 模块 × 20 声明：`plan_ms 0.26` · `digest_ms 0.004` · **`compile_ms 34.23`** · `total_ms 34.5`；
  - 缩放 4/8/16 模块：`18.42 / 31.93 / 62.76 ms`，`ratio_4x 3.41`（线性）；
  - 一次按键重编译整闭包：`best_ms 35.26` / `worst_ms 35.93`；
  - 教学规模一次按键 2/3/5 模块：`13.38 / 17.62 / 25.92 ms`；
  - 内存覆盖 vs 读盘：`34.47` vs `34.30 ms`（无额外成本）；
  - 判据前缀（2 模块）：`72.65 ms`；
  - LSP 项目 didOpen / 一次按键：`14 / 14 ms`，`publishes_per_keystroke 1`；
  - LSP hover / definition / goals：各 `0 ms`（<1ms 量级）；
  - 改依赖 ⇒ 下游刷新：`3 ms`，2 份诊断；
  - CLI 项目冷 / 热：`30.2 / 3.43 ms`；依赖改动后（必 miss）：`23.99 ms`；
  - `build 32.24 ms` / `query` 冷 `23.17 ms` / `query` 热 `2.9 ms`。
- 阈值哨兵（在 `cargo test` 内强制，不在台账里，`docs/PERF.md:22-27,102-111`）：
  400 块 ≤ 12× 50 块；增量编辑 < 50ms 且 `kernel_checks ≤ 1`；LSP didChange < 50ms；
  completion/hover < 10ms；`soko/goals` < 10ms；项目 4×20 总成本 < 2000ms；
  一次按键重编译闭包 < 2000ms；LSP 项目一次按键 < **800ms**（原 300ms 已被
  `docs/PERF.md:182-189` 取代，代码 `crates/lsp/src/tests/perf.rs:271`）；
  项目 hover/definition/goals 各 < 50ms。
- 台账义务：动编译/项目/LSP 路径后跑并**提交**（`AGENTS.md:137`、`docs/PERF.md:117`）；
  跨版本比较用 ±25% 口径、优先 `best_ms`（`docs/PERF.md:190`）。

### 6.6 其它台账（都是“契约门禁”，不是计数门禁）

| 台账 | 条数 | 门禁 | 命令 |
|---|---|---|---|
| 缺口台账 `docs/gaps/ledger.jsonl` | **24**（22 `fixed` + 2 `workaround`，0 `open`） | ✅ CI `ci.yml:207-217` + `scripts/soko gate` | `python3 scripts/gap.py selftest && python3 scripts/gap.py check` |
| 课程台账 `docs/courses/ledger.jsonl` | 1 条 | 课程门禁 `--ledger`（默认关） | `python3 courses/set-theory/tools/check.py --ledger` |
| CI 失败台账 `docs/CI-FAILURES.md` | —（人写） | 纪律：每次 CI 红都追加一条 | `skills/sokonanoda-ci/SKILL.md:173-176` |

课程最新一条（`docs/courses/ledger.jsonl`，逐字要点）：`version 0.60.0` ·
`targets 36` · `checked 329` · `open 99` · `rejected 0` · `elapsed_ms 20024` ·
`solutions_open 0`。与 `STATUS.md:56` 的“课程门禁 **36 目标 · 329 checked · 99 open · 0 判负**”一致。

### 6.7 playground 锚点的当前实测数字

锚点规则：`playground.sokonanoda` 必须用**运行中二进制内嵌的编译器**判卷通过
（只要求无 error，warnings 允许；`crates/cli/src/env/mod.rs:285-306`），
且二进制版本必须等于仓库版本，否则 `gate` exit 3（`:244-255`）。

本机实测（`./target/debug/sokonanoda`，自报 `0.61.0`）：

```bash
./target/debug/sokonanoda query check --file playground.sokonanoda --compact   # exit 0
./target/debug/sokonanoda grade playground.sokonanoda --json                   # exit 0
```

| 视图 | 结果 |
|---|---|
| `query check` 的 `counts` | `decl_checked 30` · `example_checked 2` · `exercise_open 4` · `expr_typed 0` · `expr_reduced 0` · `decl_printed 0` |
| `query check` 的 `failed` | `[]`（0 条） |
| `query check` 的 `warnings` | 2 条：`reserved-declaration-name`（第 84 行）、`redundant-sorry`（第 333 行） |
| `grade --json` 事件计数 | `decl.checked 30` · `example.checked 2` · `exercise.open 4` · `warning 2`（共 38 行） |
| 退出码 | 两者都是 **0** |

**两个视图计数逐项一致** —— 这正是 `crates/cli/tests/query.rs` 钉死的契约
（`skills/sokonanoda-dev/SKILL.md:107-109`）。

**过时数字（不要用）**：`editor/vscode/README.md:122` 说 playground 有“12 exercises”，
`docs/TESTING.md:555` 说“checked=14 / open=5 / warning=1”，
`docs/TESTING.md:472` 说“20/9/0”——三者都与当前实测不符。

**本机缓存状态（真实故障现场，可用于官网 troubleshooting 示例）**：
`~/.local/share/sokonanoda/bin/*.version` 是 `0.55.0 darwin-arm64`，仓库是 `0.61.0`，
因此 `sokonanoda doctor --json` 报 `"ready": false`（exit 3），
而 `scripts/soko doctor --json` 因为解析到版本匹配的 `target/debug` 构建而报
`"ready": true`（exit 0）——两个 `doctor` 的**判定面不同**，见 §7。

---

## 7. 安装痛点清单（官网“疑难解答”页的底稿）

### 7.1 痛点总表

| # | 症状 | 根因 | 项目做了什么 | 证据 |
|---|---|---|---|---|
| 1 | 启动器/`grade` 直接 exit 3，拒绝运行 | **缓存过期**（标记 ≠ 版本钉） | 启动器**拒绝 exec** 并打印 `expected version … (from …)`、缓存 marker、下一步命令；`cache(STALE: expected X target, found M)` | `scripts/soko:566,603-608,626-641` |
| 2 | 同上，但连版本钉都解析不出来 | 版本源缺失或不一致 | 列出找过的文件 + `pinAdvice()`；**绝不猜版本**（G-16） | `scripts/soko:186-196,268-276,280-292` |
| 3 | `sokonanoda.toml` 只写了 `requires = "0.58"`，下载失败 | `x.y` 是**约束**不是钉，没有对应的 release tag | 明确报“there is no release tag for it”，并说明该文件保留 `requires` 作为项目契约 | `scripts/soko:258-267` |
| 4 | 完全离线 | 下载需要网络 | `SOKONANODA_OFFLINE=1` 时给出人话并 exit 3；已缓存的匹配版本仍可用；VSIX 平台包**完全离线** | `scripts/soko:104-106,459`；`crates/cli/src/env/mod.rs:153-161`；`editor/vscode/README.md:144-145` |
| 5 | 公司代理 / 需要证书 | Node `fetch` **不读** `HTTPS_PROXY` | 启动器**优先用 `curl`**（curl 认代理与企业 CA），失败才回落 `fetch`；失败时把 `curl exit …` / `fetch failed …` 写进 stderr | `scripts/soko:430-449,756,784` |
| 6 | `npm test` / VS Code 下载卡住（E2E） | `@vscode/test-electron` **不读 `HTTPS_PROXY`**，只认 npm 变量 | 文档给出 `npm_config_https_proxy=http://127.0.0.1:7890 scripts/vscode-e2e.sh --version 1.106.0` | `docs/E2E.md:114-121`；`scripts/vscode-e2e.sh:28-29` |
| 7 | `~/.local/share` 不可写（沙箱/只读） | 缓存写不进去 | `chmod` 与写标记失败被容忍；`update` **明确 exit 3 + `cache NOT refreshed`**，并给 `SOKONANODA_CACHE_DIR` 出路——**不会假装成功** | `scripts/soko:485-491,765-788` |
| 8 | `update` 打印了路径但缓存没变 | 历史上“有可用回退就报成功” | 现在的判据是 `refreshed` 标志：`0` = 缓存写成了，`3` = 没写成；唯一可信证据是缓存 `marker` + 缓存二进制自述版本 | `scripts/soko:766-788`；`skills/sokonanoda-update/SKILL.md:23-29` |
| 9 | 机器上没有 cargo | 误以为使用需要 Rust | **用户/agent 路径零工具链依赖**是硬规则；Release tarball 里就是可执行二进制；VSIX 内嵌两者 | `AGENTS.md:125-127`；`README.md:88-89`；`editor/vscode/README.md:3-8` |
| 10 | 贡献者 `gate` 报 exit 3 但 cargo 全绿 | `gate` 用的是**运行中二进制**的内嵌编译器，与仓库版本不符 | 报“anchor 结果不可信”并给两条出路（`scripts/soko update` 或 `cargo run …`） | `crates/cli/src/env/mod.rs:244-255`；`AGENTS.md:140-142` |
| 11 | 贡献者 `gate` 报 exit 3，提示找不到 python3 | 课程门禁 + 缺口台账门禁需要 python3 | **绝不静默跳过**：exit 3，并给 `brew install python3` / `apt install python3` / `winget install Python.Python.3` 指引；Windows 认 `py -3` | `scripts/soko:646-663,884-890` |
| 12 | Windows 上路径 / 可执行位 / python 名字 | 跨平台差异 | 二进制加 `.exe`；缓存目录回退 `USERPROFILE`；非 win32 才 `chmod 0o755`；python 探测顺序 `py -3` → `python3` → `python`；全程 `spawnSync` + argv 数组、无 shell | `scripts/soko:68,72-74,85,98-99,485,648-649` |
| 13 | macOS Gatekeeper 拦 LSP 二进制 | quarantine 属性 | 文档给 `xattr -d com.apple.quarantine <extension>/bin/<target>/sokonanoda-lsp`；并说明 VSIX 解压**通常不带** quarantine | `docs/RELEASE.md:156-157`；`docs/design/bundled-lsp.md:246-247` |
| 14 | Marketplace 装不上 / 公司禁 Marketplace | 分发渠道限制 | Release 页有 **9 个 VSIX**（含 universal）+ 8 个 CLI tarball，可 headless 手动安装；扩展端有版本锁定的下载回退 | `AGENTS.md:176-180`；`docs/design/bundled-lsp.md:104,205-206` |
| 15 | Marketplace 索引延迟 / 503 | 服务端瞬时故障 | CI 每包重试 4 次 ×30s；文档给出 `extensionquery` 探活与“别查一次就判失败”的规程 | `release.yml:340-348`；`skills/sokonanoda-ci/SKILL.md:159,163` |
| 16 | 平台没有内置包（如 linux-armhf） | 8 平台矩阵未覆盖 | universal VSIX 的**版本锁定**下载兜底；unsupported 平台给明确指引 | `docs/design/bundled-lsp.md:240-245`；`docs/RELEASE.md:148-151` |
| 17 | “服务器还是 0.26.0” | 旧本地构建/旧缓存静默覆盖 | bundled-first：`serverPath`/`SOKONANODA_LSP_BIN`/workspace `target/` **默认被忽略**，要显式开 `serverOverride`；`doctor` 显示 `source` | `editor/vscode/README.md:127-142` |
| 18 | LSP 与扩展协议错配 | 版本错配 | 下载 URL 一律 `v${version}`；禁 `releases/latest`；release 有 tag↔版本门禁；CI 有版本一致性契约测试 | `release.yml:124-136`；`ci.yml:55-60`；`README.md:113-114` |
| 19 | DSH 里 `sokonanoda: command not found` | DSH 不能给工具调用注入 PATH（G5） | 文档统一用 `scripts/soko`（显式路径、可 commit）；`scripts/soko` 是 harness 中立入口 | `docs/design/deepseek-harness.md:117-130`；`AGENTS.md:58-68` |
| 20 | DSH 里等不到诊断 | DSH 的 LSP host 显式忽略 `publishDiagnostics`（G4/D9） | 判卷一律走 CLI `--json` / `query`；MCP 七工具把查询包成模型工具 | `dsh/README.md:38-43`；`deepseek-harness.md:59,104-115` |
| 21 | DSH 里 `lean`/`lake` 没被拦 | DSH 无命令黑名单（G6） | 自带 `dsh/hooks/hooks.json` + `refuse-lean-toolchain.js`，但需在 profile 插一行；**未启用时退回文档纪律** | `dsh/README.md:130-158` |
| 22 | 面板“像没反应” / 第一次按键慢 | 编译缓存冷 / 在编辑器外改了依赖 | `sokonanoda: build`（`alt+b`）预热、`rebuild`（`alt+shift+b`）先 `--clean` 再编；结果进 output channel | `editor/vscode/README.md:172-187`；`skills/sokonanoda-teacher/SKILL.md:331-336` |
| 23 | `setup` 成功但 `sokonanoda` 仍不在 PATH | 启动器只解析、不改 shell PATH | opencode 插件用 `shell.env` 注入 PATH；DSH/其它 harness 用 `scripts/soko` 形式或自行加 PATH | `.opencode/plugins/sokonanoda.ts:344-350`；`deepseek-harness.md:124-127` |
| 24 | 两个 `doctor` 结论不一致 | `sokonanoda doctor` 只看**缓存**；`scripts/soko doctor` 看**解析链**（含仓库构建） | 二者 JSON 字段不同：CLI 侧 `{ready,version,target,rust_target,cache,offline,cli{path,present,version_match},lsp{…}}`；启动器侧多 `version_source`/`launcher`，二进制块用 `ready`+`marker` | `crates/cli/src/env/mod.rs:89-107`；`scripts/soko:841-858` |

### 7.2 本机实测的两个真实故障样本

```bash
$ ./target/debug/sokonanoda doctor --json
{"cache":"/Users/…/.local/share/sokonanoda/bin",
 "cli":{"path":"…/sokonanoda","present":true,"version_match":false},
 "lsp":{"path":"…/sokonanoda-lsp","present":true,"version_match":false},
 "offline":false,"ready":false,"rust_target":"aarch64-apple-darwin",
 "target":"darwin-arm64","version":"0.61.0"}
$ echo $?
3
```

```bash
$ scripts/soko doctor --json
{"ready": true, "version": "0.61.0", "version_source": "Cargo.toml",
 "target": "darwin-arm64", "rust_target": "aarch64-apple-darwin",
 "cache": "…/bin", "offline": false, "launcher": "…/scripts/soko",
 "cli": {"path":"…/target/debug/sokonanoda","present":true,"ready":true,
         "marker":"0.55.0 darwin-arm64"},
 "lsp": {"path":"…/target/debug/sokonanoda-lsp","present":true,"ready":true,
         "marker":"0.55.0 darwin-arm64"}}
$ echo $?
0
```

同一个仓库、同一时刻，**CLI 的 doctor 说未就绪（缓存 0.55.0），启动器的 doctor 说就绪
（解析到了版本匹配的 `target/debug` 构建）**。官网写 troubleshooting 时必须讲清
“先看你用的是哪个 doctor”。

### 7.3 明确“不做”的事（避免官网过度承诺）

- **不为使用仓库安装 Rust/cargo**（`AGENTS.md:87`）；
- **不用 `releases/latest`**（`AGENTS.md:87`、`README.md:113-114`）；
- 不调用官方 Lean 工具链（`lean`/`lake`/`lean4export`/`leanc`/`elan`）
  —— opencode 由 `opencode.json:8-16` 强制，DSH 靠 hook 或文档纪律
  （`AGENTS.md:110-113`）；
- **不做 WASM/web 版内核**、**不做扩展内自动更新二进制**（`docs/design/bundled-lsp.md:106-110`）；
- DSH 侧**不做** MCP prompt 模板、不做服务端推送（DSH 不支持）
  （`docs/design/agent-query-channel.md:305`）。

---

## 8. 数字总表（官网可直接引用）

> 规则：每个数字给出**含义 / 确切命令 / 来源文件**。标 **未证实** 的不要上官网。
> 所有可复现数字均在 0.61.0 检出上重跑过。

### 8.1 代码与测试规模

| 数字 | 含义 | 确切命令 | 来源 |
|---|---|---|---|
| **1163** | `cargo test --workspace --locked` 的测试函数总数（passed 1163 / failed 0） | `cargo test --workspace --locked -- --list \| grep ': test$' \| grep -vc ' - '` | 实测；`STATUS.md:55` |
| 1030 | `#[test]` 出现次数 | `grep -rn "#\[test\]" crates --include=*.rs \| wc -l` | 实测 |
| 134 | `#[tokio::test]`（全在 lsp） | `grep -rn "#\[tokio::test\]" crates --include=*.rs \| wc -l` | 实测 |
| 662 / 308 / 53 / 7 | front / cli / kernel / lsp 的 `#[test]` | `grep -rn "#\[test\]" crates/<c> --include=*.rs \| wc -l` | 实测 |
| 25 | 集成测试文件数 | `ls crates/*/tests/*.rs \| wc -l` | 实测 |
| 31 | 测试二进制数 | `cargo test --workspace --locked -- --list 2>&1 \| grep -c '^     Running'` | 实测 |
| 6 | doc-test 条数 | `cargo test --workspace --locked -- --list \| grep -c ' - '` | 实测 |
| 0 | `#[ignore]` | `grep -rn "#\[ignore" crates --include=*.rs \| wc -l` | 实测 |
| 15 | 真 VS Code 集成测试条数 | `grep -cE "^\s*(test\|it)\(" editor/vscode/src/test/extension.test.js` | 实测；`docs/E2E.md:42` |
| 18 / 7 / 10 / 14 | `test-server` / `test-download` / `test-webview` / `test-extension-host` 条数 | `grep -oE "\btest\(" editor/vscode/<file> \| wc -l`（各减 1 个 helper） | 实测 |

### 8.2 扩展与 CLI 表面

| 数字 | 含义 | 确切命令 | 来源 |
|---|---|---|---|
| **14** | 贡献的命令数 | `node -e 'console.log(require("./editor/vscode/package.json").contributes.commands.length)'` | 实测；`editor/vscode/package.json:58-129` |
| **5** | 键位数 | 同上，`.keybindings.length` | 实测；`:254-278` |
| **4** | 视图数 | 同上，`Object.values(c.views).flat().length` | 实测；`:142-166` |
| **3** | 设置数 | 同上，`Object.keys(c.configuration.properties).length` | 实测；`:223-249` |
| **2** | 激活事件数 | 同上，`p.activationEvents.length` | 实测；`:22-25` |
| 7 | `query` 的 op 数 | — | `crates/cli/src/help.rs:20-28` |
| 7 | MCP 工具数 | — | `dsh/mcp/server.js:182-300` |
| 6 | `soko/*` 自定义 LSP 请求数 | — | `docs/protocol.md:297-505` |
| 5 | Agent Skill 数（3 角色 + 2 运维） | `ls skills/*/SKILL.md \| wc -l` | 实测；`skills/README.md:9-19` |
| 8 | `--json` 事件类型数 | — | `crates/cli/src/json_report.rs:27-95` |

### 8.3 发布产物

| 数字 | 含义 | 确切命令 | 来源 |
|---|---|---|---|
| **8** | 构建矩阵平台数（Rust triple） | `grep -c '^          - { os:' .github/workflows/release.yml` | 实测；`release.yml:26-34` |
| **8** | `sokonanoda-lsp-<triple>.tar.gz` 数 | — | `release.yml:266` |
| **8** | `sokonanoda-cli-<triple>.tar.gz` 数 | — | `release.yml:277` |
| **9** | VSIX 数（8 平台 + 1 universal） | — | `release.yml:172-181` |
| **26** | GitHub Release 资产总数（8+8+9+1） | — | `release.yml:266,277,283-290`；`docs/RELEASE.md:137`；`STATUS.md:59` |
| 4 | release job 数 | — | `release.yml:22,113,232,308` |
| 8 | 平台 VSIX 的 vsce target 数 | — | `release.yml:154-162` |

### 8.4 质量台账

| 数字 | 含义 | 确切命令 | 来源 |
|---|---|---|---|
| **37** | E2E 台账条数（全部 `exit=0`、`failed=0`） | `wc -l docs/e2e/ledger.jsonl`；`python3 scripts/e2e-merge.py --check` | 实测；`docs/e2e/ledger.jsonl` |
| 15 / 14 | E2E 最新一条 / 早期条目通过数 | `tail -1 docs/e2e/ledger.jsonl` | `docs/e2e/ledger.jsonl:37` |
| 25 / 12 | E2E 覆盖的 VS Code 1.138.0 / 1.106.0 次数 | `python3 -c "import json,collections;print(collections.Counter(json.loads(l)['vscode'] for l in open('docs/e2e/ledger.jsonl')))"` | 实测 |
| **14** | 性能台账条数 | `wc -l docs/perf/ledger.jsonl` | 实测 |
| 12 | 性能 case 数 | `python3 -c "import json;print(len(json.loads(open('docs/perf/ledger.jsonl').readlines()[-1])['records']))"` | 实测 |
| **24** | 缺口台账条数（22 fixed + 2 workaround + 0 open） | `wc -l docs/gaps/ledger.jsonl` | 实测；`docs/gaps/ledger.jsonl` |
| 36 / 329 / 99 / 0 | 课程门禁：目标 / checked / open / 判负 | — | `docs/courses/ledger.jsonl`；`STATUS.md:56` |

### 8.5 性能（最新台账一条，v0.59.0）

| 数字 | 含义 | 来源 |
|---|---|---|
| 34.23 ms | 4 模块 × 20 声明的闭包编译 | `docs/perf/ledger.jsonl:14`（`closure_stages.compile_ms`） |
| 18.42 / 31.93 / 62.76 ms | 4 / 8 / 16 模块 × 10 声明，`ratio_4x 3.41` | 同上（`closure_compile_scaling`） |
| 35.26 ms | 一次按键重编译整个闭包（`best_ms`） | 同上（`keystroke_recompile_closure`） |
| 13.38 / 17.62 / 25.92 ms | 教学规模（2/3/5 模块 × 12 声明）一次按键 | 同上（`teaching_scale_keystroke`） |
| 14 / 14 ms | LSP 项目 didOpen / 一次按键，`publishes_per_keystroke 1` | 同上（`did_open_and_keystroke`） |
| 30.2 / 3.43 ms | CLI 项目冷 / 热判卷 | 同上（`cold_warm_check`） |
| 32.24 / 23.17 / 2.9 ms | `build` / `query` 冷 / `query` 热 | 同上（`query_and_build`） |

### 8.6 playground 锚点（本机实测，0.61.0）

| 数字 | 含义 | 确切命令 |
|---|---|---|
| 30 | `decl.checked` | `./target/debug/sokonanoda query check --file playground.sokonanoda --compact` |
| 2 | `example.checked` | 同上 |
| 4 | 开放练习（`exercise_open`） | 同上 |
| 0 | `failed` | 同上 |
| 2 | warnings（`reserved-declaration-name` @84、`redundant-sorry` @333） | 同上 |
| 0 | 退出码（`query check` 与 `grade --json` 都是 0） | 同上 + `./target/debug/sokonanoda grade playground.sokonanoda --json` |

### 8.7 明确标记「未证实」的项（不要上官网）

1. `cargo binstall sokonanoda-cli` / `mise github:ColorlessBoy/sokonanoda-lang`
   可用性 —— 仓库无 `[package.metadata.binstall]`，仅有 `README.md:116-119` 的文字声明。
2. `docs/design/deepseek-harness.md:336` 写的技能路径 `.dsh/skills`
   —— 实际落地在 `.agents/skills/`（`:203`、`:420`）。
3. `docs/TESTING.md:156` 说的“2 个被 ignore 的 kernel fixture 测试”
   —— 今天 `#[ignore]` 计数为 **0**。
4. 任何“当前版本 playground 的期望计数”的**文档**记载
   —— 文档里的 `14/5/1`、`20/9/0`、扩展 README 的“12 exercises”都已过时；
   只有 §8.6 的实测数字可用。
5. `docs/TESTING.md:666-673` 的 888 / `:632` 的 862 / `:647` 的 871 测试数
   —— 全部是历史版本数字（0.58.0 及更早）。
6. `docs/RELEASE.md:90` 的“资产共 25 个” —— 正确值是 **26**。
7. `dsh/cordis.patch.yml:22-24` 注释说的 “six MCP tools” 与设计文档
   `docs/design/agent-query-channel.md:311-318` 的 6 个 `soko_*` 工具名
   —— as-built 是 **7 个裸名工具**。
8. `docs/design/deepseek-harness.md:3` 的“实现未开始” —— 与同文件 §9
   （H0–H4 全部落地，`:414-428`）矛盾，以后者为准。
9. `docs/design/deepseek-harness.md:77-78` 把两个未核实项归为 “H4 backlog”
   —— 正文 H4 没有这两项，实际对应 H5 的 B1/B2。
10. DSH 是否计划投递 `publishDiagnostics` / 客户端插件能否注入自定义 LSP 方法
    —— 设计文档明确“不作为承诺”。

---

## 附：取证命令汇总（可一键复跑）

```bash
# 计数（扩展）
node -e 'const c=require("./editor/vscode/package.json").contributes;
console.log(c.commands.length,c.keybindings.length,
Object.values(c.views).flat().length,
Object.keys(c.configuration.properties).length)'

# 计数（Rust 测试）
cargo test --workspace --locked -- --list | grep ': test$' | grep -vc ' - '

# 计数（台账）
wc -l docs/e2e/ledger.jsonl docs/perf/ledger.jsonl docs/gaps/ledger.jsonl

# 计数（发布矩阵与资产）
grep -c '^          - { os:' .github/workflows/release.yml
grep -n 'tar\.gz\|\.vsix\|SHA256SUMS' .github/workflows/release.yml

# 锚点实测
./target/debug/sokonanoda query check --file playground.sokonanoda --compact
./target/debug/sokonanoda grade playground.sokonanoda --json

# 环境诊断（注意两者判定面不同）
./target/debug/sokonanoda doctor --json
scripts/soko doctor --json
```

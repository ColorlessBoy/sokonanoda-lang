# 设计：扩展服务器策略——强制内置 LSP + 自动体检（2026-09-14）

> 触发：用户报告扩展升到 0.29.0，`restart server` 仍回弹 `0.26.0 → 0.26.0`。
> 盘链路后确认：VS Code 用户设置里的 `sokonanoda.serverPath` 指向仓库里一个**陈旧
> 的 `target/debug/sokonanoda-lsp`（0.26.0）**；显式路径在解析链里优先级最高，
> 于是内置的 0.29.0 服务器永远用不上。用户明确要求：**扩展强制用它自己附带的
> LSP**，并且要**自动检测所有这类版本问题**。

## 1. 根因

`editor/vscode/server.js::resolveServerCommand` 的语义是「显式覆盖优先」：
`setting → env → 内置 bin → 工作区 target → 当前缓存 → (下载兜底)`。对**开发**是
合理的（改 Rust 后立即生效），但对**用户**是隐患：任何写死的 `serverPath`
（尤其是仓库构建产物）会静默压过内置服务器，导致版本失真且无从发现。

此外没有任何「自检」入口：版本不一致、宿主未重载、旧版本堆叠都只能靠人肉判断。

## 2. 决策：默认强制内置，覆盖改为显式 opt-in

- 新设置 **`sokonanoda.serverOverride`（boolean，默认 `false`）**。
- **`false`（默认，面向用户）**：解析链 = **内置 bin → 当前缓存（仅 universal
  包）→ 版本锁定下载兜底**。`serverPath` / `SOKONANODA_LSP_BIN` / 工作区
  `target/{debug,release}` **一律忽略**（若设置了 `serverPath`/env，弹一次
  非阻塞提示说明「已忽略，正在用内置服务器；要覆盖请打开 serverOverride」）。
- **`true`（面向贡献者）**：恢复旧链
  `setting → env → 内置 → 工作区 → 缓存 → 下载`。
- 内置 bin 缺失（universal 包/不支持平台）时，两条链都继续走**缓存(仅当
  marker 匹配) → `v<扩展版本>` 锁定下载**。

这样：用户永远拿到与自己扩展同版本的内置服务器；贡献者显式开开关即可继续
「改 Rust 立刻生效」。

## 3. 自动体检 `sokonanoda: doctor`

新增命令 `sokonanoda.doctor`，并**在激活后与 `restart server` 后自动运行一次**
（结果写入输出通道；有 `error` 级问题时弹一条非阻塞通知，带「运行 doctor」按钮）。

检查项（每项给出：状态、发现的版本、成因、可执行建议）：

| # | 检查 | 判定 | 建议 |
|---|---|---|---|
| 1 | **解析结果与来源** | `bundled` / `override(setting\|env\|workspace)` / `cache` / `download` | 显示实际命令路径 |
| 2 | **运行服务器版本** | `soko/version.version` == `extensionVersion` | 不等 → 指出是否被 override；给「重载窗口/清 serverPath」 |
| 3 | **扩展宿主版本** | 运行宿主 == 磁盘上最新已安装版本 | 不等 → `Developer: Reload Window` |
| 4 | **被忽略的覆盖** | 设置了 `serverPath`/`SOKONANODA_LSP_BIN` 而 `serverOverride=false` | 说明已忽略；给「改用内置（清设置）/开启 override」 |
| 5 | **下载缓存** | `~/.local/share/sokonanoda/bin` 的 cli/lsp marker == 扩展版本 | 不等 → 提示 `sokonanoda update`（仅影响 CLI/universal 兜底，不影响内置） |
| 6 | **旧版本堆积**（信息项） | `bin`/extensions 目录里 `.obsolete`/多版本数量 | 提示「完整重启 VS Code 会自动清理」 |

Doctor 只读：**绝不改用户设置、绝不删文件**；最多给「复制命令 / 打开设置」的
动作（`vscode.commands.executeCommand` 白名单命令），不做隐式写操作。

## 4. 与既有能力的关系

- `restartServer` 继续复用 `resolveServerForStart`；新增返回「来源」用于消息
  （`server restarted — 0.26.0 (pid) → 0.30.0 (pid)，source=bundled`）。
- 0.27.1 加的 `newestInstalledExtensionVersion` 警告并入 doctor 的检查 #3；
  `restart server` 仍即时提示。
- webview Infoview 的 `server` 消息（`{running, version, pid}`）可叠加
  `source`，供面板显示「内置/覆盖」。
- opencode 插件（`.opencode/plugins/sokonanoda.ts`）解析链**保持不变**（它面向
  贡献者仓库，repo build 优先是刻意设计）；本设计只改 VS Code 扩展。

## 5. 测试三层

- **单元**（`editor/vscode/test-server.js`）：`resolveServerCommand` 的
  `override=false` → 即使有 setting/env/workspace 也返回内置；`override=true` →
  旧序；无内置时走「当前缓存/undefined」；`serverPath` 存在但被忽略不改结果。
- **静态契约**（`crates/cli/tests/extension.rs`）：`package.json` 声明
  `serverOverride` 设置与 `sokonanoda.doctor` 命令且 `extension.js` 注册；
  `extension.js` 默认 `override=false`（负断言：不存在「setting 先于 bundled」
  的旧链）；doctor 覆盖 6 项检查标识。
- **集成**（`editor/vscode/src/test/extension.test.js`）：`doctor` 命令可执行、
  输出包含 `source=` 与版本行；无服务器时 skip 不 fail。
- **手动**：真实装 VSIX 后 doctor 报 bundled=扩展版本；写一个 serverPath 指向
  旧二进制 → doctor 报「已忽略 + 建议」。

## 6. 版本与门面

新增设置 + 命令 = **minor**（`docs/vscode-dev-guide.md` §2）：0.30.0 →
0.31.0；同步 `CHANGELOG.md`（Added/Fixed）、`README.md`、`package.json`
description；Rust/扩展版本一致（`cargo_and_extension_versions_match`）。

## 7. 明确不做

- 不改 opencode 插件解析链；不删用户文件/设置（doctor 只读）；
- 不做远程/市场版本查询（离线可用，只比对本地版本）；
- 不引入第二条文档/服务器真相来源。

---

## 8. as-built（2026-09-14，0.31.0）

- **解析链**：`resolveServerCommand` 增 `override`（默认 `false`）并返回
  `{command, source}`（`bundled|setting|env|workspace|cache`）。
  `override=false`：**bundled → 当前缓存 → undefined（调用方下载）**，
  setting/env/workspace 忽略；`override=true`：恢复旧序。
- **设置**：新增 `sokonanoda.serverOverride`（boolean，默认 false；
  `restricted` 配置项）；`serverPath` 描述改为「仅 override 时生效」。
- **提示**：`override=false` 且设置了 `serverPath`/`SOKONANODA_LSP_BIN` 时
  弹一次非阻塞通知（含「打开设置」「运行 doctor」动作）。
- **doctor**：`sokonanoda.doctor` 只读输出 6 项检查（解析来源/运行版本/
  宿主版本/被忽略覆盖/缓存/旧版本堆积），激活与 restart 后自动跑一次；
  有问题时非阻塞通知带「运行 doctor」。
- **restart 回执**：带 `source=`（如 `… → 0.31.0 (pid N), source=bundled`）。
- **测试**：`test-server.js` 增 override 单测（默认 bundled、陈旧 serverPath
  不覆盖、忽略 workspace、override=true 旧序、无内置走当前缓存）；
  `crates/cli/tests/extension.rs` 静态契约（设置/命令默认 bundled-first）；
  `extension.test.js` doctor 冒烟（返回含 `source=` 与版本行）。
- **文档**：`editor/vscode/README.md` 解析顺序改写、`docs/vscode-dev-guide.md`
  §5.6 补 bundled-first + doctor。
- **版本** 0.30.0 → **0.31.0**（新设置 + 命令 = minor）。

> 立即止血（本机）：用户设置里的 `sokonanoda.serverPath` 已移除（备份
> `settings.json.bak-sokonanoda`）；8 个 `.obsolete` 旧版本已删，只剩 0.29.0。

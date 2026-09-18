# 例行化：真实 VS Code 集成测试（`scripts/vscode-e2e.sh`）

> 0.58.0（第九十七轮）起，扩展的**真宿主**验证从"想起来才手工跑一次"变成一条
> 命令 + 一份提交进仓库的台账。CI 也跑同一套用例（Linux + `xvfb-run`），本地
> 这条的价值是：**换机器/换 VS Code 版本/改扩展后随时复跑，并把结果留档**。

## 1. 一条命令

```bash
SOKO_VSCODE_TEST_VERSION=1.138.0 scripts/vscode-e2e.sh   # 钉版本，免下载
scripts/vscode-e2e.sh                                     # 默认 stable（与 CI 一致）
```

脚本做四件事（缺一步都会得出"看起来像回归、其实是环境"的结论）：

1. `cargo build --release -p sokonanoda-lsp -p sokonanoda-cli` —— 被测的就是发布形态；
2. `node scripts/stage-lsp.js` —— 把刚构建的二进制 stage 进 `editor/vscode/bin/<target>/`。
   **必须做**：扩展默认 bundled-first，而 `bin/` 是 gitignored 目录，很容易停在几天前的
   旧二进制上（第一次例行跑就对着 0.20.0 的服务器断言 0.58.0 的功能，10 个用例集体超时）；
3. `npm test`（`@vscode/test-cli` → `@vscode/test-electron`）——真 VS Code + 真 LSP + 真扩展宿主；
   同时设 `SOKO_E2E_LOG`，让扩展把接线事实写一份文件日志（见 §4）；
4. 记账：`docs/e2e/ledger.jsonl`（追加，`schema: soko.e2e/1`）+ `docs/e2e/latest.json`
   + 裁剪后的日志 `docs/e2e/logs/<date>-<sha>.log`（doctor 块 + 用例清单 + 扩展接线日志
   + 失败详情）。

退出码：`0` 全绿 · `1` 有用例失败 · `2` 用法错误 · `3` 前置缺失（cargo / node /
`editor/vscode/node_modules`）。**先 `(cd editor/vscode && npm install)`** 一次性准备依赖。

## 2. 这一层守什么（与其它层不重叠）

| 层 | 谁来跑 | 守什么 |
| --- | --- | --- |
| 纯 Node 单测 | `npm run test:unit` | server.js 解析顺序/下载 URL、webview DOM、**stub 宿主**的扩展接线（含项目树渲染） |
| 静态契约 | `cargo test -p sokonanoda-cli --test extension` | package.json 字段/命令注册/打包与版本一致性 |
| **真宿主（本文）** | `scripts/vscode-e2e.sh` | 扩展在**真 VS Code** 里激活 → 起**真 LSP** → 诊断/inlay/hover/codeLens/重启/Infoview/doctor/项目树**端到端**成立；`.sokonanoda` 语言 id、项目树的行来自真 `soko/project` 答案 |
| 手工 F5 | 开发者 | 肉眼观感、主题、Marketplace 安装态 |

用例清单在 `editor/vscode/src/test/extension.test.js`（0.58.0 起 14 条）；新增用户可见
行为时**同一轮**加一条真宿主断言，并在 `docs/TESTING.md` 的集成测试小节登记。

## 3. 台账字段（`docs/e2e/ledger.jsonl`）

```json
{"schema":"soko.e2e/1","kind":"vscode-integration","version":"0.58.0",
 "commit":"…","dirty":false,"date":"2026-09-18T07:36:47Z",
 "host":{"system":"Darwin","machine":"arm64","release":"25.6.0"},
 "vscode":"1.138.0","tests":{"passed":14,"failed":0,"pending":0},"exit":0,
 "server":"0.58.0 (pid 83493) == 扩展 v0.58.0 (source=bundled)",
 "lsp_sha256_16":"a176502a3189c17b","log":"docs/e2e/logs/2026-09-18-0f91e9e.log"}
```

* `dirty` = 记录时工作区是否有未提交改动（有 ⇒ 结果对应的代码比 commit 新）；
* `server` 那行来自 doctor 的 `server-version` 检查，直接回答"测的是不是当前构建"；
* `lsp_sha256_16` 是被测 `bin/<target>/sokonanoda-lsp` 的指纹前 16 位；
* 版本 bump / 扩展改动 / 换 VS Code 版本后各跑一次，历史上就能看出"哪次开始红的"。

## 4. 卡住时的第一现场：`SOKO_E2E_LOG`

扩展宿主的 `console` 在 `vscode-test` 的输出里**看不到**，output channel 的文件也取不到，
所以扩展在 `SOKO_E2E_LOG` 指向的文件里追加关键事实：

```
server command: …/bin/darwin-arm64/sokonanoda-lsp (source=bundled)
client state: starting → running
active document changed: file:///…/Main.sokonanoda
soko/project ok: project=Main (2 modules) reason=null
```

判断口径：**有 `client state: running` + `soko/project ok` ⇒ 服务器与协议都通**，问题在
断言/渲染；只有 `server command` 没有 `running` ⇒ 服务器起不来（先看 doctor 的
`server-version` 行与 `bin/` 是否 stage 过）；完全没有日志 ⇒ `activate()` 没走到异步续段
（见 `docs/vscode-dev-guide.md` 坑 19）。

## 5. 已知环境坑

* **macOS 的 unix socket 路径上限 103 字符**：`.vscode-test.mjs` 自己把
  `--user-data-dir` / `--extensions-dir` 指到 `<tmpdir>/soko-vscode-test`，所以本仓库
  的长路径可以直接跑（0.58.0 修）；若 `tmpdir` 本身很长，退回"拷到 `/tmp/v`"的老办法
  （`docs/vscode-dev-guide.md` 坑 14）。
* **VS Code 版本**：默认 `stable`，stable 升级会重新下载 ~300MB；例行复跑建议
  `SOKO_VSCODE_TEST_VERSION=1.138.0`（或任何已缓存版本）钉住。
* **Linux 无显示器**：CI 用 `xvfb-run -a npm test`；本地无头环境同理。
* **`code` CLI 冲突**：macOS 上若报 "another instance running"，先关掉正在跑的 VS Code。

## 6. 与 CI 的关系

CI 的 `VS Code extension integration tests` 步骤（ubuntu + `xvfb-run`）跑的是**同一套
用例**，但：

* CI 不记录 `docs/e2e/ledger.jsonl`（那份台账由本地脚本写，提交进仓库）；
* CI 每次用 `stable`（跟随升级），本地可以钉版本复现历史结果；
* CI 只有 Linux；**macOS/Windows 的真宿主行为只能靠本文这条本地例行**（macOS 的
  文件系统大小写、`/var`→`/private/var` 符号链接等差异正是真宿主才会暴露的）。

# 例行化：真实 VS Code 集成测试（`scripts/vscode-e2e.sh`）

> 0.58.0（第九十七轮）起，扩展的**真宿主**验证从"想起来才手工跑一次"变成一条
> 命令 + 一份提交进仓库的台账。CI 也跑同一套用例（Linux + `xvfb-run`），本地
> 这条的价值是：**换机器/换 VS Code 版本/改扩展后随时复跑，并把结果留档**。

## 1. 一条命令

```bash
scripts/vscode-e2e.sh                     # 默认钉住"已知良好"版本（当前 1.138.0）
scripts/vscode-e2e.sh --version stable    # 跟随最新稳定版（升级当天会重下 ~300MB）
scripts/vscode-e2e.sh --version 1.106.0   # 试声明的最低版本（engines.vscode）
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

## 5. VS Code 版本策略（调研结论 + 我们的选择）

**上游默认是什么**：`@vscode/test-cli` 的 `version` 选项**默认 `stable` 频道**
（官方文档：*"version - The version of VS Code to use for running tests
(defaults to stable)"*）；[官方文档](https://code.visualstudio.com/api/working-with-extensions/testing-extension)
的高级示例写的是 `version: 'insiders'`，[官方 sample](https://github.com/microsoft/vscode-extension-samples)
（`helloworld-test-cli-sample`）**不写 `version`**，即接受默认。也就是说：
**"把某个具体版本设成默认"不是生态惯例**——惯例是跟频道（stable/insiders），
只有需要复现时才钉版本（钉法：配置里的 `version: '1.85.0'`，或
`@vscode/test-electron` 的 `downloadAndUnzipVSCode('1.85.0')`）。

**我们的选择**（0.58.0）：*本地例行*与 *CI* 都**钉一个具体版本**，理由是这一层的
产物是**台账**——`docs/e2e/ledger.jsonl` 要回答"哪次开始红的"，如果宿主跟着 stable
漂，历史条目就不可比（stable 一升级，同一份代码的宿主、DOM、API 全换了）。
具体做法：

| 场景 | 版本 | 怎么给 |
| --- | --- | --- |
| 本地例行（默认） | **1.138.0**（已知良好） | `scripts/vscode-e2e.sh` 里的 `default_version` |
| 本地试新 | `stable` / `insiders` / 任意具体版本 | `--version <v>` 或 `SOKO_VSCODE_TEST_VERSION=<v>` |
| CI（ubuntu） | 矩阵显式给：`1.138.0`（当前稳定）+ **`1.106.0`（声明的最低版本）** | `.github/workflows/ci.yml` 的 `e2e.matrix` |
| CI（macOS） | `1.138.0`，**只在 push 到 main 时跑** | 同上（job 级 `if`） |
| 声明的最低版本（`engines.vscode ^1.106.0`） | ✅ **2026-09-18 本地已验证 14/14**（台账 `b0bcba3` 那条，VS Code 1.106.0 + bundled 0.58.0；项目树三条同样绿），随后进 CI 矩阵 | `--version 1.106.0` |

> 升级流程：先 `--version <新版本>` 本地跑绿 → 改 `scripts/vscode-e2e.sh` 的
> `default_version` 与 `ci.yml` 的 `e2e.matrix`（两处）+ 记一条台账。

## 6. 已知环境坑

* **macOS 的 unix socket 路径上限 103 字符**：`.vscode-test.mjs` 自己把
  `--user-data-dir` / `--extensions-dir` 指到 `<tmpdir>/soko-vscode-test`，所以本仓库
  的长路径可以直接跑（0.58.0 修）；若 `tmpdir` 本身很长，退回"拷到 `/tmp/v`"的老办法
  （`docs/vscode-dev-guide.md` 坑 14）。
* **缓存会变大**：每个版本一份 ~900MB 的 VS Code；换钉版本后旧的可以删
  （`editor/vscode/.vscode-test/vscode-<platform>-<version>/`）。
* **网络受限时用 npm 的代理变量**：`@vscode/test-electron` 只读
  **`npm_config_proxy` / `npm_config_https_proxy`**（源码 `util.js` 里建
  `HttpProxyAgent`/`HttpsProxyAgent` 就是这两个），**不读 `HTTPS_PROXY`**。
  实测（2026-09-18）：直连 `vscode.download.prss.microsoft.com` 反复
  `Recv failure: Connection reset by peer`，加
  `npm_config_https_proxy=http://127.0.0.1:7890` 后 1.106.0 一次重试就下完、14/14 全绿：
  ```bash
  npm_config_https_proxy=http://127.0.0.1:7890 scripts/vscode-e2e.sh --version 1.106.0
  ```
* **下载老版本可能被 15s 无数据超时打断**：`@vscode/test-electron` 的下载超时是
  `timeout: 15_000`（**无数据** 15 秒即 abort，与总时长无关），本机拉 1.106.0 时反复
  `aborted`，而 1.138.0 正常。绕过办法是**预置缓存**（等价于它自己做完的事）：
  ```bash
  v=1.106.0; d=editor/vscode/.vscode-test/vscode-darwin-arm64-$v
  curl -L --retry 5 -C - -o /tmp/vscode-$v.zip \
    "https://update.code.visualstudio.com/$v/darwin-arm64/stable"
  mkdir -p "$d" && unzip -q /tmp/vscode-$v.zip -d "$d" && touch "$d/is-complete"
  SOKO_VSCODE_TEST_VERSION=$v scripts/vscode-e2e.sh
  ```
  > 这条预置法是 2026-09-18 直连反复被 reset 时的备用手段；同一天用上面那个
  > `npm_config_https_proxy` 就正常下完了（**优先用代理**，预置法留给代理也不可用的
  > 环境）。
* **Linux 无显示器**：CI 用 `xvfb-run -a npm test`；本地无头环境同理。
* **`code` CLI 冲突**：macOS 上若报 "another instance running"，先关掉正在跑的 VS Code。

## 7. 与 CI 的关系（0.58.0 起：CI 也跑这条命令）

CI 有独立的 **`e2e` job**（`.github/workflows/ci.yml`）：

* **矩阵（3 条腿）**：`ubuntu-latest` × VS Code `1.138.0`（当前稳定）与 `1.106.0`
  （声明的最低版本）**每个 PR/分支 push 都跑**（`xvfb-run -a`）；
  `macos-latest` × `1.138.0` **只在 push 到 main 时跑**（macOS 差异值得守，但每个 PR
  多 ~10 分钟不划算）；
* **同一条命令**：两步都是 `scripts/vscode-e2e.sh`（构建 release → stage → 真宿主 →
  记账）——本地与 CI 不会漂；
* **留档**：`docs/e2e/` 作为 artifact 上传（`e2e-<os>-vscode-<version>`），并把
  `scripts/e2e-summary.py` 的渲染写进 **job summary**（结果/版本/服务器/LSP 指纹/log）；
  CI **不回提交**仓库（本地那份 `docs/e2e/ledger.jsonl` 由人提交）；
* **门禁**：`auto-tag` 的 `needs` 含 `e2e` ⇒ **e2e 红了就不发版**。

本地的不可替代之处：macOS 的真宿主差异（`/var`→`/private/var` 符号链接、大小写不敏感
文件系统）、换 VS Code 版本复现历史、以及"钉住版本 + 提交台账"这件事本身（CI 的
runner 每次都是干净的，历史趋势只在仓库里）。

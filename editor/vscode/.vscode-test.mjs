// @vscode/test-cli 的运行配置（`npm test` → `vscode-test`）。
// 前置条件：当前构建的服务器二进制已在 `bin/<target>/`（`npm run stage:lsp`）
// 或 `target/debug|release/`，测试自身绝不构建服务器。
// 服务器解析顺序（extension.js / server.js）：bundled `bin/<target>/` 优先，
// 其次工作区文件夹 + 仓库根（editor/vscode 的上两级）下的
// target/debug|release/sokonanoda-lsp。CI 会先 stage 到 bin/，让集成测试
// 走用户安装平台包后的同一条路径。
//
// 这里的所有旋钮都为**例行化本地跑**服务（`scripts/vscode-e2e.sh`）：
//   * `--user-data-dir` / `--extensions-dir` 必须显式给**短路径**：VS Code 的
//     单实例 socket 是 `<user-data-dir>/<ver>-main.sock`，macOS 的 unix socket
//     路径上限 103 字符——默认位置（仓库内 `.vscode-test/user-data`）在本仓库
//     的长路径下直接 `EINVAL` 起不来（实测）。
//   * 版本默认 `stable`（与 CI 一致），可用 `SOKO_VSCODE_TEST_VERSION` 钉住
//     （例行复跑建议钉住，省掉 stable 升级时的 ~300MB 下载）。
import { defineConfig } from "@vscode/test-cli";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const dirname = path.dirname(fileURLToPath(import.meta.url));
const stateDir = path.join(os.tmpdir(), "soko-vscode-test");

export default defineConfig({
  // 扩展本身是纯 JS，测试同样保持纯 JS（无 TS 构建步骤）。
  files: "src/test/**/*.test.js",
  version: process.env.SOKO_VSCODE_TEST_VERSION || "stable",
  // 打开仓库根作为工作区会连带触发课程地图的 CLI 子进程；专用夹具工作区
  // 让测试只依赖 LSP 服务器本身，结果确定。
  workspaceFolder: path.join(dirname, "src/test/fixtures/workspace"),
  launchArgs: [
    "--disable-extensions", // 隔离：不带用户已装扩展
    "--disable-workspace-trust", // 跳过信任对话框（会阻塞激活）
    `--user-data-dir=${path.join(stateDir, "user-data")}`,
    `--extensions-dir=${path.join(stateDir, "extensions")}`,
  ],
  mocha: {
    // 首个用例要等扩展激活 + 语言服务器起进程，给足余量。
    timeout: 60000,
    // 单用例快跑（L4 层，`scripts/vscode-e2e.sh --grep <名字>`）：
    // `@vscode/test-cli` 把 config 的 `mocha` 原样交给 Mocha
    // （node_modules/@vscode/test-cli/out/runner.cjs:12-17），所以 `grep` 直接生效。
    // 不设 `SOKO_E2E_GREP` 时是 `undefined` ⇒ 行为与从前逐字节相同（跑全量）。
    grep: process.env.SOKO_E2E_GREP || undefined,
  },
});

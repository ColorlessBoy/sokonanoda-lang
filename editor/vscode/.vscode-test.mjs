// @vscode/test-cli 的运行配置（`npm test` → `vscode-test`）。
// 前置条件：`cargo build -p sokonanoda-lsp` 已产出服务器二进制（测试自身
// 绝不构建服务器；CI 与本地都先构建再跑测试）。
// 服务器解析顺序（extension.js / server.js）：bundled `bin/<target>/` 优先，
// 其次工作区文件夹 + 仓库根（editor/vscode 的上两级）下的
// target/debug|release/sokonanoda-lsp。CI 会先 stage 到 bin/，让集成测试
// 走用户安装平台包后的同一条路径。
import { defineConfig } from "@vscode/test-cli";
import path from "node:path";
import { fileURLToPath } from "node:url";

const dirname = path.dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  // 扩展本身是纯 JS，测试同样保持纯 JS（无 TS 构建步骤）。
  files: "src/test/**/*.test.js",
  version: "stable",
  // 打开仓库根作为工作区会连带触发课程地图的 CLI 子进程；专用夹具工作区
  // 让测试只依赖 LSP 服务器本身，结果确定。
  workspaceFolder: path.join(dirname, "src/test/fixtures/workspace"),
  launchArgs: [
    "--disable-extensions", // 隔离：不带用户已装扩展
    "--disable-workspace-trust", // 跳过信任对话框（会阻塞激活）
  ],
  mocha: {
    // 首个用例要等扩展激活 + 语言服务器起进程，给足余量。
    timeout: 60000,
  },
});

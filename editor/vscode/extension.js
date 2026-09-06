// Minimal VS Code client for the sokonanoda language server.
// Requirements: `cargo build -p sokonanoda-lsp` first (or have it on PATH).
const path = require("path");
const { spawn } = require("child_process");
const vscode = require("vscode");
const { LanguageClient, ServerOptions, TransportKind } = require("vscode-languageclient/node");

let client;

function serverCommand() {
  // 1. explicit env var  2. bundled target/debug binary  3. PATH
  if (process.env.SOKONANODA_LSP_BIN) return process.env.SOKONANODA_LSP_BIN;
  const repo = path.join(__dirname, "..", "..", "..");
  const bundled = path.join(repo, "target", "debug", "sokonanoda-lsp");
  return bundled;
}

async function activate(context) {
  const command = serverCommand();
  if (!require("fs").existsSync(command)) {
    const pick = await vscode.window.showWarningMessage(
      "sokonanoda-lsp 还没编译。先在仓库根目录运行 cargo build -p sokonanoda-lsp，或设置 SOKONANODA_LSP_BIN。",
      "打开仓库"
    );
    if (pick) {
      vscode.commands.executeCommand("vscode.openFolder", vscode.Uri.file(path.join(__dirname, "..", "..", "..")));
    }
    return;
  }
  const serverOptions = { run: { command }, debug: { command } };
  client = new LanguageClient("sokonanoda", "sokonanoda", serverOptions, {
    documentSelector: [{ language: "sokonanoda", scheme: "file" }],
    synchronize: { fileEvents: vscode.workspace.createFileSystemWatcher("**/*.sokonanoda") },
  });
  context.subscriptions.push(client.start());
}

async function deactivate() {
  if (client) await client.stop();
}
module.exports = { activate, deactivate };

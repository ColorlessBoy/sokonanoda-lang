// VS Code 集成测试：扩展在真实 VS Code（@vscode/test-electron）里跑，
// 由 `vscode-test`（@vscode/test-cli）经 .vscode-test.mjs 启动。
// 前置条件：`cargo build -p sokonanoda-lsp` 必须先执行——测试自身绝不构建
// 服务器，只假设二进制已在 target/debug|release（CI 与本地都先构建）。
// 判定全部走真实 kernel：诊断是服务器发布的，hover 是服务器算的；这里不
// 复刻任何前端逻辑（客户端不做文本判定的教训同样适用于测试）。
const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const vscode = require("vscode");

const EXTENSION_ID = "sokonanoda-lang.sokonanoda";
const WAIT_MS = 30000;
const POLL_MS = 100;

// 内联自 examples/lesson-01.sokonanoda：干净教学文件，期望 0 诊断
//（开放练习 `sorry` 是成功态，不是错误）。
const LESSON_01 = [
  "-- Lesson 1: functions and types",
  "",
  "def id : Prop -> Prop := fun (x : Prop) => x",
  "",
  "#check id",
  "",
  "example : Prop -> Prop := sorry",
  "",
].join("\n");

// kernel 拒绝样例：lambda 的实际类型是 Prop -> Prop，与声明的 Prop -> Type
// 不匹配（与 crates/front compile/tests.rs 的 Failed 用例同族）。
const KERNEL_BAD = "def bad : Prop -> Type := fun (x : Prop) => x\n";

// 开放练习：`sorry` 是洞，hover 应给出非空信息（目标/引导）。
const EXERCISE = "-- 练习：把 Prop -> Prop 证掉。\nexample : Prop -> Prop := sorry\n";

suite("sokonanoda extension (VS Code integration)", () => {
  // 测试文档写到系统临时目录：file:// URI 能被 documentSelector 命中，
  // 且不污染夹具工作区。
  let tmpDir;

  suiteSetup(() => {
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "sokonanoda-vscode-test-"));
  });

  suiteTeardown(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  teardown(async () => {
    await vscode.commands.executeCommand("workbench.action.closeAllEditors");
  });

  async function writeDoc(name, content) {
    const file = path.join(tmpDir, name);
    fs.writeFileSync(file, content);
    return vscode.Uri.file(file);
  }

  function sleep(ms) {
    return new Promise((resolve) => setTimeout(resolve, ms));
  }

  // 轮询直到 cond 为真；超时报出上下文（诊断/hover 的到达是异步的）。
  async function waitFor(desc, cond, timeout = WAIT_MS) {
    const start = Date.now();
    for (;;) {
      if (await cond()) return;
      if (Date.now() - start > timeout) {
        throw new Error(`timed out after ${timeout}ms waiting for ${desc}`);
      }
      await sleep(POLL_MS);
    }
  }

  async function hoverTextAt(uri, line, character) {
    const hovers = await vscode.commands.executeCommand(
      "vscode.executeHoverProvider",
      uri,
      new vscode.Position(line, character),
    );
    const hover = hovers && hovers.length > 0 ? hovers[0] : undefined;
    const contents = hover?.contents;
    if (contents === undefined) return "";
    if (typeof contents === "string") return contents;
    if (Array.isArray(contents)) {
      return contents.map((part) => (typeof part === "string" ? part : part.value ?? "")).join("\n");
    }
    return contents.value ?? "";
  }

  test("extension activates", async () => {
    const ext = vscode.extensions.getExtension(EXTENSION_ID);
    assert.ok(ext, `extension ${EXTENSION_ID} must be present in the test instance`);
    await ext.activate();
    assert.ok(ext.isActive, "extension must report active after activate()");
  });

  test("clean lesson publishes empty diagnostics", async () => {
    const uri = await writeDoc("lesson-01.sokonanoda", LESSON_01);
    await vscode.workspace.openTextDocument(uri);
    await vscode.window.showTextDocument(uri, { preview: false, preserveFocus: true });
    // ready 信号：hover 非空 = 服务器已编译完这份文档（refresh 先发诊断再
    // 返回，hover 在其后），此时「没有诊断」才有意义。
    await waitFor("the first hover on the clean lesson", async () =>
      (await hoverTextAt(uri, 2, 10)) !== "",
    );
    const diagnostics = vscode.languages.getDiagnostics(uri);
    assert.deepStrictEqual(
      diagnostics.map((d) => d.code),
      [],
      `clean lesson must have no diagnostics, got: ${JSON.stringify(diagnostics.map((d) => [d.code, d.message]))}`,
    );
  });

  test("kernel-rejected declaration carries the kernel-rejected code", async () => {
    const uri = await writeDoc("kernel-bad.sokonanoda", KERNEL_BAD);
    await vscode.workspace.openTextDocument(uri);
    await vscode.window.showTextDocument(uri, { preview: false, preserveFocus: true });
    await waitFor("a kernel-rejected diagnostic", async () =>
      vscode.languages.getDiagnostics(uri).some((d) => d.code === "kernel-rejected"),
    );
    const diagnostic = vscode.languages
      .getDiagnostics(uri)
      .find((d) => d.code === "kernel-rejected");
    assert.ok(diagnostic, "the kernel-rejected diagnostic must stay published");
    assert.strictEqual(diagnostic.source, "sokonanoda", "diagnostics must be sourced");
    assert.strictEqual(
      diagnostic.severity,
      vscode.DiagnosticSeverity.Error,
      "kernel rejections are errors",
    );
  });

  test("open exercise shows a hover on the hole", async () => {
    const uri = await writeDoc("exercise.sokonanoda", EXERCISE);
    await vscode.workspace.openTextDocument(uri);
    await vscode.window.showTextDocument(uri, { preview: false, preserveFocus: true });
    // 第 1 行 `example : Prop -> Prop := sorry`，光标落在 sorry 上。
    let text = "";
    await waitFor("a hover on the sorry hole", async () => {
      text = await hoverTextAt(uri, 1, 28);
      return text !== "";
    });
    assert.ok(text.trim().length > 0, "hover markup must be non-empty");
    // 开放练习是成功态：练习不产生诊断。
    assert.deepStrictEqual(
      vscode.languages.getDiagnostics(uri).map((d) => d.code),
      [],
      "an open exercise is a success state, not an error",
    );
  });
});

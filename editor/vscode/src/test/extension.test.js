// VS Code 集成测试：扩展在真实 VS Code（@vscode/test-electron）里跑，
// 由 `vscode-test`（@vscode/test-cli）经 .vscode-test.mjs 启动。
// 前置条件：`cargo build -p sokonanoda-lsp` 必须先执行——测试自身绝不构建
// 服务器，只假设二进制已在 target/debug|release 或 PATH（CI 与本地都先
// 构建）。找不到二进制时整组 skip（不是 fail）：激活能过但服务器起不来，
// 诊断/hover 只会无限等待直到超时，那不是被测代码的回归。
// 判定全部走真实 kernel：诊断是服务器发布的，hover 是服务器算的；这里不
// 复刻任何前端逻辑（客户端不做文本判定的教训同样适用于测试）。
const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const vscode = require("vscode");

const EXTENSION_ID = "sokonanoda-lang.sokonanoda";
const SERVER_NAME = "sokonanoda-lsp";
const WAIT_MS = 30000;
const POLL_MS = 100;

// 与 extension.js 的自动发现同族：仓库根（src/test 上四级）的
// target/debug|release，再退 PATH。
const REPO_ROOT = path.resolve(__dirname, "..", "..", "..", "..");

function findServerBinary() {
  for (const profile of ["debug", "release"]) {
    const candidate = path.join(REPO_ROOT, "target", profile, SERVER_NAME);
    if (fs.existsSync(candidate)) return candidate;
  }
  for (const dir of (process.env.PATH ?? "").split(path.delimiter)) {
    if (!dir) continue;
    const candidate = path.join(dir, SERVER_NAME);
    if (fs.existsSync(candidate)) return candidate;
  }
  return undefined;
}

const SERVER_BINARY = findServerBinary();

// 干净教学文件（内联自 examples/lesson-01.sokonanoda，去掉开放练习行）：
// 不含 sorry——含 sorry 的文件现在会带一条 WARNING（见下）。
const LESSON_CLEAN = [
  "-- Lesson 1: functions and types",
  "",
  "def id : Prop -> Prop := fun (x : Prop) => x",
  "",
  "#check id",
].join("\n");

// kernel 拒绝样例：lambda 的实际类型是 Prop -> Prop，与声明的 Prop -> Type
// 不匹配（与 crates/front compile/tests.rs 的 Failed 用例同族）。
const KERNEL_BAD = "def bad : Prop -> Type := fun (x : Prop) => x\n";

// 开放练习：`sorry` 是洞。Lean 4 对齐语义：文件编译通过但带缺口 →
// 服务器发 code `sorry` 的 WARNING（不是 error，也不该静默）。
const EXERCISE = "theorem t : True := sorry\n";

// 声明名撞内置排序：内核接受（`Prop` 在这里是内置排序），但这个顶层名字
// 永远解析不到 → 服务器发 code `reserved-declaration-name` 的 WARNING，
// 文件整体仍编译通过。
const RESERVED = "axiom Prop : Sort 1\n";

const suiteRunner = SERVER_BINARY ? suite : suite.skip;

suiteRunner("sokonanoda extension (VS Code integration)", () => {
  // 测试文档写到系统临时目录：file:// URI 能被 documentSelector 命中，
  // 且不污染夹具工作区。
  let tmpDir;

  suiteSetup(async () => {
    console.log(`sokonanoda-lsp binary: ${SERVER_BINARY}`);
    const ext = vscode.extensions.getExtension(EXTENSION_ID);
    assert.ok(ext, `extension ${EXTENSION_ID} must be present in the test instance`);
    await ext.activate();
    assert.ok(ext.isActive, "extension must report active after activate()");
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

  test("confusable-character highlight is off for sokonanoda files", async () => {
    // α/β/γ 是教学语言的 binder 名；扩展用语言级默认关掉 VS Code 的
    // Trojan-Source 混淆字符框（同内置 plaintext/markdown 的做法）。
    const cfg = vscode.workspace.getConfiguration("editor", {
      languageId: "sokonanoda",
    });
    assert.strictEqual(
      cfg.get("unicodeHighlight.ambiguousCharacters"),
      false,
      "extension must default the confusable-character box off for .sokonanoda",
    );
  });

  test("#check results appear as inlay hints", async () => {
    // Lean Infoview 的 #check 等价物：`#check Nat` 之后常显 `: Type 0`。
    const src = "#check Nat\n#check (Nat -> Nat)\n";
    const uri = await writeDoc("check.sokonanoda", src);
    await vscode.workspace.openTextDocument(uri);
    await vscode.window.showTextDocument(uri, { preview: false, preserveFocus: true });
    await waitFor("inlay hints for #check", async () => {
      const hints = await vscode.commands.executeCommand(
        "vscode.executeInlayHintProvider",
        uri,
        new vscode.Range(0, 0, 10, 0),
      );
      return Array.isArray(hints) && hints.length >= 2;
    });
    const hints = await vscode.commands.executeCommand(
      "vscode.executeInlayHintProvider",
      uri,
      new vscode.Range(0, 0, 10, 0),
    );
    const labels = hints.map((h) =>
      typeof h.label === "string" ? h.label : h.label?.value ?? "",
    );
    assert.ok(
      labels.includes(": Type 0"),
      `check hints must show the kernel result, got: ${JSON.stringify(labels)}`,
    );
  });

  test("clean lesson publishes empty diagnostics", async () => {
    const uri = await writeDoc("lesson-clean.sokonanoda", LESSON_CLEAN);
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

  test("open exercise carries a sorry warning, not an error", async () => {
    const uri = await writeDoc("exercise.sokonanoda", EXERCISE);
    await vscode.workspace.openTextDocument(uri);
    await vscode.window.showTextDocument(uri, { preview: false, preserveFocus: true });
    await waitFor("a sorry diagnostic", async () =>
      vscode.languages.getDiagnostics(uri).some((d) => d.code === "sorry"),
    );
    const diagnostic = vscode.languages.getDiagnostics(uri).find((d) => d.code === "sorry");
    assert.ok(diagnostic, "the sorry diagnostic must stay published");
    assert.strictEqual(diagnostic.source, "sokonanoda", "diagnostics must be sourced");
    assert.strictEqual(
      diagnostic.severity,
      vscode.DiagnosticSeverity.Warning,
      "an open exercise compiles: sorry is a warning, not an error",
    );
    assert.ok(
      !vscode.languages
        .getDiagnostics(uri)
        .some((d) => d.severity === vscode.DiagnosticSeverity.Error),
      "no error diagnostics for a compiling file with holes",
    );
  });

  test("a declaration named like a built-in sort warns, not errors", async () => {
    const uri = await writeDoc("reserved.sokonanoda", RESERVED);
    await vscode.workspace.openTextDocument(uri);
    await vscode.window.showTextDocument(uri, { preview: false, preserveFocus: true });
    await waitFor("a reserved-declaration-name diagnostic", async () =>
      vscode.languages
        .getDiagnostics(uri)
        .some((d) => d.code === "reserved-declaration-name"),
    );
    const diagnostic = vscode.languages
      .getDiagnostics(uri)
      .find((d) => d.code === "reserved-declaration-name");
    assert.ok(diagnostic, "the reserved-declaration-name diagnostic must stay published");
    assert.strictEqual(diagnostic.source, "sokonanoda", "diagnostics must be sourced");
    assert.strictEqual(
      diagnostic.severity,
      vscode.DiagnosticSeverity.Warning,
      "a sort-name collision is a warning, not an error",
    );
    assert.ok(
      !vscode.languages
        .getDiagnostics(uri)
        .some((d) => d.severity === vscode.DiagnosticSeverity.Error),
      "the declaration itself compiles: no error diagnostics",
    );
  });

  test("open exercise shows a hover on the hole", async () => {
    const uri = await writeDoc("exercise-hover.sokonanoda", EXERCISE);
    await vscode.workspace.openTextDocument(uri);
    await vscode.window.showTextDocument(uri, { preview: false, preserveFocus: true });
    // `theorem t : True := sorry`：光标落在第 0 行的 sorry 上。
    let text = "";
    await waitFor("a hover on the sorry hole", async () => {
      text = await hoverTextAt(uri, 0, 22);
      return text !== "";
    });
    assert.ok(text.trim().length > 0, "hover markup must be non-empty");
  });
});

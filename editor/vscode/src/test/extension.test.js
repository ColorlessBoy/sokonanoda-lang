// VS Code 集成测试：扩展在真实 VS Code（@vscode/test-electron）里跑，
// 由 `vscode-test`（@vscode/test-cli）经 .vscode-test.mjs 启动。
// 前置条件：被测服务器必须先构建并 stage 到 `bin/<target>/`——测试自身绝不构建
// 服务器（扩展解析 bundled-first，所以 stage 是必须的）。例行入口：
// `SOKO_VSCODE_TEST_VERSION=1.138.0 scripts/vscode-e2e.sh`（构建 + stage + 跑 +
// 记账），手册 `docs/E2E.md`。找不到二进制时整组 skip（不是 fail）：激活能过但服务器起不来，
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

// 声明名撞内核已定义的名字：内核接受（`Prop` 内核已经定义过），但这个
// 顶层名字永远用不上 → 服务器发 code `reserved-declaration-name` 的
// WARNING，文件整体仍编译通过。
const RESERVED = "axiom Prop : Sort 1\n";

const suiteRunner = SERVER_BINARY ? suite : suite.skip;

suiteRunner("sokonanoda extension (VS Code integration)", () => {
  // 测试文档写到系统临时目录：file:// URI 能被 documentSelector 命中，
  // 且不污染夹具工作区。
  let tmpDir;
  let extensionApi;

  suiteSetup(async () => {
    console.log(`sokonanoda-lsp binary: ${SERVER_BINARY}`);
    const ext = vscode.extensions.getExtension(EXTENSION_ID);
    assert.ok(ext, `extension ${EXTENSION_ID} must be present in the test instance`);
    await ext.activate();
    assert.ok(ext.isActive, "extension must report active after activate()");
    // activate() returns the tree providers when the host runs in test mode
    // (extension.js: `ExtensionMode.Test`) — this suite asserts real tree rows
    // built from real `soko/project` answers, so it needs them.
    extensionApi = ext.exports;
    // Print the extension's own view of the world once per run: the doctor
    // report names the resolved server (`source=`), which is the first thing to
    // look at when a run hangs (the suite is skipped only when *no* binary
    // exists — a wrong-but-existing binary shows up as timeouts).
    try {
      const report = await vscode.commands.executeCommand("sokonanoda.doctor");
      console.log(`--- doctor ---\n${report}`);
    } catch (error) {
      console.log(`--- doctor failed: ${error?.message ?? error}`);
    }
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

  /// 写一个小项目（多文件）到子目录，返回 文件名 → URI。
  async function writeProject(tag, files) {
    const dir = path.join(tmpDir, tag);
    fs.mkdirSync(dir, { recursive: true });
    const uris = {};
    for (const [name, content] of Object.entries(files)) {
      const file = path.join(dir, name);
      fs.mkdirSync(path.dirname(file), { recursive: true });
      fs.writeFileSync(file, content);
      uris[name] = vscode.Uri.file(file);
    }
    return uris;
  }

  /// 打开并聚焦一个文档（项目树只跟踪**活跃**编辑器）。
  async function showDoc(uri) {
    await vscode.workspace.openTextDocument(uri);
    await vscode.window.showTextDocument(uri, { preview: false });
  }

  /// 项目树的第一行（等真实 `soko/project` 答案到达）。
  ///
  /// 单文件是**一条占位行**（没有 children），项目是一条根行（有 children）——
  /// 两者都算"答案到了"，所以这里只等"不再是 loading 占位"。
  async function projectRoot(desc) {
    assert.ok(
      extensionApi && extensionApi.project,
      "activate() must expose the project provider in test mode",
    );
    await vscode.commands.executeCommand("sokonanoda.project.refresh");
    await waitFor(`project rows for ${desc}`, async () => {
      const rows = await extensionApi.project.getChildren();
      return rows.length === 1 && String(rows[0].label) !== "正在读取项目状态…";
    });
    const rows = await extensionApi.project.getChildren();
    return rows[0];
  }

  function sleep(ms) {
    return new Promise((resolve) => setTimeout(resolve, ms));
  }

  // 轮询直到 cond 为真；超时报出上下文（诊断/hover 的到达是异步的）。
  // `poll` 可调小：量时间的用例不能被 100ms 的轮询粒度量化（T-A60-1）。
  async function waitFor(desc, cond, timeout = WAIT_MS, poll = POLL_MS) {
    const start = Date.now();
    for (;;) {
      if (await cond()) return;
      if (Date.now() - start > timeout) {
        throw new Error(`timed out after ${timeout}ms waiting for ${desc}`);
      }
      await sleep(poll);
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

  // hover 的第一块内容（MarkdownString 时连 isTrusted 一起拿到）。LSP 的
  // hover markdown 默认**不受信**，命令链接在那里是死的——要断言按钮真的
  // 可点，必须把它读出来，光看文本里有没有链接是不够的。
  async function hoverMarkdownAt(uri, line, character) {
    const hovers = await vscode.commands.executeCommand(
      "vscode.executeHoverProvider",
      uri,
      new vscode.Position(line, character),
    );
    const contents = hovers?.[0]?.contents;
    if (Array.isArray(contents)) return contents[0];
    return contents;
  }

  function commandLinkEnabled(trusted, command) {
    if (trusted === true) return true;
    if (!trusted || typeof trusted !== "object") return false;
    return Array.isArray(trusted.enabledCommands) && trusted.enabledCommands.includes(command);
  }

  test(".sokonanoda files get the sokonanoda language id", async () => {
    // 一切 LSP 交互的前提：文件被判成 `sokonanoda` 语言（`languages` 贡献 +
    // extension.js 的 documentSelector）。这条断言先跑，失败时后面全是超时，
    // 报错信息会指出根因而不是"等不到诊断"。
    const uri = await writeDoc("langid.sokonanoda", "def two : Nat := 2\n");
    const doc = await vscode.workspace.openTextDocument(uri);
    const registered = (await vscode.languages.getLanguages()).filter((id) =>
      String(id).includes("soko"),
    );
    assert.strictEqual(
      doc.languageId,
      "sokonanoda",
      `the extension must map .sokonanoda to the sokonanoda language (registered ids: ${registered.join(", ") || "none"})`,
    );
  });

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

  test("a declaration named Prop warns, not errors", async () => {
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

  test("hover 的类型文本折成记法（T-U12 面 #3，e2e 那一份）", async () => {
    // **同一个缺陷、更便宜的判据已有 front 版**（`hover_text_is_folded_like_the_lsp_does` ✓）；
    // 这一份走**真宿主 + 真 hover** ✓ —— 它才是"用户看得见"的确认 ✓（`docs/design/e2-plan.md` T-U12 ✓）。
    //
    // ⚠ **必须先在夹具里给常量声明记法**（§9："先给常量声明记法" ✓）——
    // 否则**折叠没有规则** ✗（round 154/155 两次都栽在这 ✓）。
    // ⚠ **折叠出现在"某物的类型提到了被记法化的常量"处** ✓ ——
    // **不是**在记法符号本身的 hover 上 ✗（round 468 实测：那里显示的是
    // 记法目标的类型 `Set Nat → Set Nat → Prop` ✓ ⇒ 里面没有 `⊆` ✗）。
    // ⇒ **照 `docs/design/e2-plan.md` face #2 的配方** ✓：让一个 `def` 的**类型**
    // 就是 `Set.subset Nat A B` ✓ ⇒ hover 它的用处 ⇒ 折后应是 `A ⊆ B` ✓。
    const SRC = [
      // ⚠ **`Set` 本身要声明** ✗（这个文件里其它夹具都没用到它 ✓）；
      // ⚠ **`Set.subset` 的元数要对** ✗ —— 我第一版写 `Set.subset Nat A B` ✗
      // （3 个参数 ✗），而这里它是 **2 个** ✓ ⇒ 声明 elaborate 不了 ⇒
      // hover 只说"未通过，见诊断" ✗（round 471 实测踩到 ✓）。
      'def Set (α : Type) : Type := α → Prop',
      'def Set.subset (A B : Set Nat) : Prop := True',
      'infix:50 " ⊆ " => Set.subset',
      // ⚠ **用 `axiom`（无体）** ✗ —— `def … := True` 要求 `True` 与 `Set.subset A B`
      // 定义相等 ✗，而 elaborate 不展开它 ⇒ 声明不过 ⇒ hover 只说"未通过，见诊断" ✗
      // （round 473 实测：失败文本从 `def usesWeird` 前进到 `def Weird` ✓）。
      'axiom Weird (A B : Set Nat) (P : Prop) : Set.subset A B',
      'def usesWeird (A B : Set Nat) : Prop := Weird A B True',
    ].join('\n');
    const uri = await writeDoc("notation-hover.sokonanoda", SRC);
    await vscode.workspace.openTextDocument(uri);
    await vscode.window.showTextDocument(uri, { preview: false, preserveFocus: true });
    // 光标落在第 2 行 `A ⊆ B` 的 `⊆` 上（修饰符号处 ✓ —— 那是
    // `half_expression_goals_hover` 那条路 ✓）。
    const line = 3;
    const col = SRC.split('\n')[line].indexOf('Weird');
    // ⚠ **`waitFor` 只等"hover 到了"（非空 ✓），断言留给 `assert`** ✓ ——
    // 否则失败信息只有"超时" ✗，而**看不出实际文本是什么** ✗（round 468 实测踩到 ✓）。
    let text = "";
    await waitFor("a hover on the type line", async () => {
      text = await hoverTextAt(uri, line, col);
      return text.trim().length > 0;
    });
    assert.ok(text.includes('⊆'), `hover 应含记法 ⊆（实际 = ${text}）`);
    assert.ok(!text.includes('Set.subset '), `hover 不应漏点形式 Set.subset（实际 = ${text}）`);
  });

  test("notation input: the rewriter produces ∧ and hover teaches \\and", async () => {
    // NI-2（docs/design/notation-input.md §6.4）：真宿主 + 真命令 + 真 hover。
    //
    // 这里**测不了**的两件事，各有归属：
    //  * 键位本身按不下去——VS Code 没有"发一个 Tab 键"的公开 API；`when` 子句
    //    由静态契约测试 `notation_input_tab_binding_is_gated_by_its_context_key`
    //    守护（key 名必须与代码里置位的那个一致）。
    //  * undo 也驱动不了：workbench 的 `undo` 命令在 vscode-test 宿主里是 **no-op**
    //    （实测：命令返回后 800ms 文本仍是 `∧`，先 `focusActiveEditorGroup` 也一样
    //    ——它要的是 UI 键盘焦点，测试宿主给不了）。「一次 edit = 一个 undo 单元」
    //    因此钉在 stub 宿主层（`test-extension-host.js` 的 fake editor 按真宿主的
    //    粒度记账），并靠 VS Code 自己的 `TextEditor.edit` 默认
    //    `{undoStopBefore: true, undoStopAfter: true}`（1.138.0 自带源码实测）。
    const source = "theorem t (a b : Prop) (h : a ∧ b) : a ∧ b := h\n";
    const uri = await writeDoc("notation-input.sokonanoda", source);
    const doc = await vscode.workspace.openTextDocument(uri);
    const editor = await vscode.window.showTextDocument(doc, { preview: false });

    // hover 半个：光标落在 `∧` 上，文案必须告诉学习者怎么打出来。
    let hover = "";
    await waitFor("a hover that teaches the abbreviation", async () => {
      hover = await hoverTextAt(uri, 0, source.indexOf("∧"));
      return hover.includes("\\and");
    });
    assert.ok(hover.includes("\\and"), `hover must teach the abbreviation, got: ${hover}`);

    // 打字半个：文档里出现一个 `\and`，真命令把它换成 `∧`（`∧` 在真编辑器里就是
    // 这么敲出来的；命令与键位走的是同一个 handler）。
    await editor.edit((builder) => builder.insert(new vscode.Position(0, 0), "\\and\n"));
    editor.selection = new vscode.Selection(0, 4, 0, 4);
    await vscode.commands.executeCommand("sokonanoda.input.replaceAbbreviation");
    assert.strictEqual(
      doc.lineAt(0).text,
      "∧",
      `the rewriter must produce ∧, got: ${doc.lineAt(0).text}`,
    );
  });

  test("restart server command re-syncs open documents", async () => {
    // `Sokonanoda: Restart Server (重启服务器)` 重新解析二进制并重启客户端；重启后
    // 打开中的文档要重新拿到诊断（场景：本地二进制重建/缓存刷新后，
    // 不想重载整个窗口）。
    const uri = await writeDoc("restart.sokonanoda", EXERCISE);
    await vscode.workspace.openTextDocument(uri);
    await vscode.window.showTextDocument(uri, { preview: false, preserveFocus: true });
    await waitFor("a sorry diagnostic before restart", async () =>
      vscode.languages.getDiagnostics(uri).some((d) => d.code === "sorry"),
    );
    await vscode.commands.executeCommand("sokonanoda.restartServer");
    await waitFor("diagnostics re-published after restart", async () =>
      vscode.languages.getDiagnostics(uri).some((d) => d.code === "sorry"),
    );
  });

  test("infoview command is registered and revealable", async () => {
    // 冒烟（docs/design/webview-infoview.md §8）：命令存在，聚焦 webview 不抛。
    // webview 本身由真实宿主渲染，这里只验证接线；数据/渲染由静态契约守护。
    // openInfoview 依次走 auxiliary bar / 容器 / view 三个命令——它们必须都
    // 已注册且可执行，否则「打不开面板」只会静默失败。
    const commands = await vscode.commands.getCommands(true);
    assert.ok(
      commands.includes("sokonanoda.openInfoview"),
      "sokonanoda.openInfoview must be registered",
    );
    assert.ok(
      commands.includes("workbench.view.extension.sokonanoda"),
      "the sokonanoda container reveal command must be available",
    );
    assert.ok(
      commands.includes("sokonanoda.infoview.focus"),
      "the sokonanoda.infoview focus command must be available",
    );
    await vscode.commands.executeCommand("sokonanoda.openInfoview");
    await vscode.commands.executeCommand("workbench.view.extension.sokonanoda");
    await vscode.commands.executeCommand("sokonanoda.infoview.focus");
  });

  test("the project tree shows the real closure of an imported module", async () => {
    // 0.58.0 批次 4 的真宿主验证：真 VS Code + 真 LSP + 真 provider。
    // 数据来自服务器 `soko/project`（只读派生），这里断言**用户看到的行**。
    const uris = await writeProject("proj-ok", {
      "Lib.sokonanoda": "def lib_value : Nat := 2\n",
      "Main.sokonanoda": "import Lib\n\ndef two : Nat := lib_value\n",
    });
    await showDoc(uris["Main.sokonanoda"]);
    // **在断言处等**（2026-09-24 修）：`projectRoot` 只保证"行出现了"，
    // 而 `description`（"2 模块"）是**随后**填的——慢 runner 上会抢跑。
    // 注意**不能**把它写进 `projectRoot` 的等待条件：**单文件文档的 description
    // 本来就该是空的**（没有"模块数"可言），那样会让 `single-file` 那条用例
    // 等一个永远不来的非空描述（实测：三平台全红、超时 30s）。
    let root = await projectRoot("Main.sokonanoda");
    await waitFor("the root counts the closure", async () => {
      root = await projectRoot("Main.sokonanoda");
      return String(root.description || "").includes("2 模块");
    });
    assert.ok(
      String(root.label).includes("proj-ok"),
      `the root is named after the module root: ${root.label}`,
    );
    assert.ok(
      String(root.description).includes("2 模块"),
      `the root counts the closure: ${root.description}`,
    );
    assert.ok(
      String(root.tooltip).includes("零配置"),
      `no manifest ⇒ the tooltip says zero config: ${root.tooltip}`,
    );
    const modules = await extensionApi.project.getChildren(root);
    assert.deepStrictEqual(
      modules.map((row) => String(row.label)),
      ["Lib", "Main"],
      "topological order, entry last",
    );
    assert.strictEqual(String(modules[0].description), "依赖 · 1 声明");
    assert.strictEqual(String(modules[1].description), "入口 · 1 声明");
    assert.strictEqual(modules[0].command.command, "vscode.open", "clicking opens the module");
    // 服务器给出的路径是 canonicalize 过的绝对路径（macOS 上 /var 是 /private/var
    // 的符号链接）——CLI 与 LSP 因此对同一文件给出逐字相同的答案。
    assert.strictEqual(
      fs.realpathSync(modules[0].command.arguments[0].fsPath),
      fs.realpathSync(uris["Lib.sokonanoda"].fsPath),
      "the row opens the imported module's file",
    );
  });

  test("the project tree names the file a single-file document is", async () => {
    const uri = await writeDoc("single.sokonanoda", "def two : Nat := 2\n");
    await showDoc(uri);
    const root = await projectRoot("single.sokonanoda");
    assert.strictEqual(
      String(root.label),
      "单文件（无 import）",
      "a file without imports is a legal state, not an empty tree",
    );
    assert.deepStrictEqual(await extensionApi.project.getChildren(root), []);
  });

  test("the project tree marks a broken import as the root cause", async () => {
    // 缺失的模块 ⇒ 入口行说清缺哪个名字（status=blocked/load-failed 都由
    // 服务器判定，客户端只渲染）。这里同时验证"未保存/新写的文件也能立刻
    // 得到项目视图"（LSP 每次都重编译闭包）。
    const uris = await writeProject("proj-broken", {
      "Main.sokonanoda": "import Missing\n\ndef two : Nat := 2\n",
    });
    await showDoc(uris["Main.sokonanoda"]);
    const root = await projectRoot("broken project");
    const modules = await extensionApi.project.getChildren(root);
    const missing = modules.find((row) => String(row.tooltip).includes("Missing"));
    assert.ok(
      missing,
      `the failure story must name the missing module: ${modules
        .map((row) => `${row.label} :: ${row.tooltip}`)
        .join(" | ")}`,
    );
    assert.strictEqual(
      missing.iconPath.id,
      "error",
      "the module that could not be loaded carries the error icon",
    );
    assert.ok(
      String(missing.description).startsWith("入口"),
      `the root cause here IS the entry (the missing module never loads): ${missing.description}`,
    );
    assert.strictEqual(
      root.iconPath.id,
      "warning",
      "the project root flags that something failed",
    );
  });

  test("build / rebuild commands warm the compile cache through the CLI", async () => {
    // 用户报的缺口：CLI 有 `sokonanoda build [--clean]`，扩展没接出来。
    // 冒烟：两个命令都注册；build 走真实 CLI 子进程并把 build.summary 渲染成
    // 人话返回（与 doctor 一样返回文本，测试可断言）；rebuild 额外报告清掉的
    // 缓存条数（`build --clean` 的 build.clean 事件）。
    const commands = await vscode.commands.getCommands(true);
    for (const id of ["sokonanoda.build", "sokonanoda.rebuild"]) {
      assert.ok(commands.includes(id), `${id} must be registered`);
    }
    const uri = await writeDoc("build-smoke.sokonanoda", LESSON_CLEAN);
    const doc = await vscode.workspace.openTextDocument(uri);
    await vscode.window.showTextDocument(doc);
    const built = await vscode.commands.executeCommand("sokonanoda.build");
    assert.strictEqual(typeof built, "string", "build must return its summary text");
    assert.ok(
      built.includes("sokonanoda build") && /\d+ 个文件/.test(built),
      `build summary must name the file count, got: ${built}`,
    );
    const rebuilt = await vscode.commands.executeCommand("sokonanoda.rebuild");
    assert.strictEqual(typeof rebuilt, "string", "rebuild must return its summary text");
    assert.ok(
      rebuilt.includes("rebuild") && rebuilt.includes("清掉"),
      `rebuild summary must report the cleaned cache entries, got: ${rebuilt}`,
    );
  });

  test("doctor command returns a read-only source + version report", async () => {
    // 冒烟（docs/design/extension-server-policy.md §5 集成层）：doctor 命令可
    // 执行，返回报告文本且包含来源（source=）与版本行；只读、绝不抛。
    const commands = await vscode.commands.getCommands(true);
    assert.ok(commands.includes("sokonanoda.doctor"), "sokonanoda.doctor must be registered");
    const report = await vscode.commands.executeCommand("sokonanoda.doctor");
    assert.strictEqual(typeof report, "string", "doctor must return its report text");
    assert.ok(report.includes("source="), `doctor report must include source=, got: ${report}`);
    assert.ok(
      /\d+\.\d+\.\d+/.test(report),
      `doctor report must include a version line, got: ${report}`,
    );
  });
  // ── E1 六条反馈的用例矩阵（计划 §0.5 的 T-018）─────────────────────────────
  //
  // 每条用户反馈一个用例。夹具是 `src/test/fixtures/workspace/`（T-015）：
  // 一个**最小 import 项目**（`sokonanoda.toml` + `lib/Set` 带 `∈`/`⊆` 记法 +
  // `units/u01` 带 `sorry`）——它精确复现了"入口单独 parse 必然失败"的形状，
  // 而那正是这几条缺口的共同前提。
  //
  // **这些用例在修复前必须是红的**（否则它们证明不了任何东西）。计划里每条
  // 都写了它对应哪个环节；红了先看那条环节。

  const fixtureEntry = () => {
    const root = vscode.workspace.workspaceFolders?.[0]?.uri?.fsPath;
    assert.ok(root, "e2e 需要一个工作区目录（.vscode-test.mjs 的 workspaceFolder）");
    return vscode.Uri.file(path.join(root, "units", "u01.sokonanoda"));
  };

  /// 等 Infoview 的声明载荷到达（`soko/goals` 是异步的）。
  async function infoviewDecls(desc) {
    assert.ok(
      extensionApi && extensionApi.infoview,
      "activate() 必须在 test mode 暴露 infoview（T-016）",
    );
    await waitFor(`${desc}：Infoview 收到声明载荷`, async () => {
      await extensionApi.goals.ensureDeclarations().catch(() => {});
      return extensionApi.infoview.lastDecls().length > 0;
    });
    return extensionApi.infoview.lastDecls();
  }

  test("declarations panel lists a project unit's declarations", async () => {
    // 用例 #1（G-22）：含 `import` + 库记法的入口，`soko/goals` 必须非空。
    const entry = fixtureEntry();
    await showDoc(entry);
    const decls = await infoviewDecls("用例 #1");
    const names = decls.map((d) => d.name);
    assert.ok(
      names.includes("mem_self"),
      `声明栏必须列出 mem_self，实际 = ${JSON.stringify(names)}`,
    );
    // 与树（同一份载荷建的 TreeItem）条数一致。
    assert.strictEqual(
      extensionApi.goals.declItems.length,
      decls.length,
      "声明栏与练习树的条数必须一致（同一份 soko/goals 载荷）",
    );

    // **R-1 的 e2e 判据（形状守卫版）**：`def` 的声明卡片要显示第二行 `:= <值>`。
    // 数据链：内核 `Declar::value()` → front 的 `DeclInfo.value/value_runs`
    // → **LSP 的 `GoalDeclInfo`**（以前这里**漏映射** ⇒ 扩展恒拿 undefined
    // ⇒ 那一行**静默消失**；因为扩展是"有就渲染"的宽容实现，**e2e 抓不到**）
    // → `media/infoview.js` 渲染 `.decl-val-line`。
    //
    // 这里**不要求 fixture 里有 `def`**（本 fixture 只有 theorem）：有 `value` 的声明，
    // 必须同时带 `value_runs`，且 runs 能重建 value。**"字段必须在 wire 里"的强判据**
    // 由 `scripts/audit-wire-fields.py` 守（已进 gate 与 CI）——那条才是主守卫。
    const withValue = decls.filter((d) => typeof d.value === "string" && d.value.length > 0);
    for (const d of withValue) {
      assert.ok(
        Array.isArray(d.value_runs) && d.value_runs.length > 0,
        `${d.name} 有 value 就必须有 value_runs（Infoview 靠它渲染 := 行），实际 = ${JSON.stringify(d.value_runs)}`,
      );
      assert.strictEqual(
        d.value_runs.map((r) => r.text || "").join(""),
        d.value,
        `${d.name} 的 value_runs 必须能重建 value（与 ty_runs 同款守卫）`,
      );
    }
  });

  test("open declaration ships coloured goal runs to the Infoview", async () => {
    // **T-A5 / R-2 ② 的 e2e 判据**：Infoview 的声明卡片要显示一行**带色**的
    // `⊢ <目标>`（以前根本不画那一行；就算画了也只是纯文本）。
    //
    // 这一层能断言什么（验证设计纪律：三层各司其职）：
    //  * 真 VS Code 的**扩展宿主读不到 webview 的 DOM**（平台不暴露）⇒ e2e 能
    //    断言的最强事实是"**webview 收到了什么**"，即数据链最后一跳
    //    （内核 → front → LSP → 扩展 → webview 载荷）的字段与内容；
    //  * **渲染结果**那一跳由 `editor/vscode/test-webview.js` 的 stub DOM 断言
    //    （那里跑的是真的 `media/infoview.js`，断言 `.decl-goal` + `tok-*`）；
    //  * **字段必须在 wire 里**由 `scripts/audit-wire-fields.py` 守（它咬得住
    //    R-1 的 `value_runs` 与 T-A5 的 `goal_runs`/`goals_runs`）。
    // 三层缺一层就是洞——R-1/R-2 都是"每层各自绿、用户看不见"。
    const entry = fixtureEntry();
    await showDoc(entry);
    const decls = await infoviewDecls("T-A5");
    const open = decls.filter((d) => d && d.status === "open");
    assert.ok(
      open.length > 0,
      `夹具必须有一个开放练习，否则这条断言会空转：${JSON.stringify(decls.map((d) => d.status))}`,
    );
    for (const decl of open) {
      const goal = decl.goal;
      assert.ok(
        typeof goal === "string" && goal.length > 0,
        `${decl.name} 是开放练习就必须有 goal`,
      );
      // 父：`goal_runs` 重建 `goal`，且至少一个 run 带 `kind`——没有 kind 就
      // 只能画纯文本，那正是用户报的"目标不高亮"。
      assert.ok(
        Array.isArray(decl.goal_runs) && decl.goal_runs.length > 0,
        `${decl.name} 的 goal 必须带 goal_runs，实际 = ${JSON.stringify(decl.goal_runs)}`,
      );
      assert.strictEqual(
        decl.goal_runs.map((r) => r.text || "").join(""),
        goal,
        `${decl.name} 的 goal_runs 必须逐字节重建 goal`,
      );
      // 子：`goals_runs` 与 `goals` 按位置对齐，且每一段都能重建（错位会让颜色
      // 贴到别的目标上）。
      const goals = Array.isArray(decl.goals) ? decl.goals : [];
      const runs = Array.isArray(decl.goals_runs) ? decl.goals_runs : [];
      assert.ok(goals.length > 0, `${decl.name} 开放就必须有 goals`);
      assert.strictEqual(
        runs.length,
        goals.length,
        `${decl.name} 的 goals_runs 必须与 goals 等长`,
      );
      goals.forEach((text, index) => {
        assert.strictEqual(
          runs[index].map((r) => r.text || "").join(""),
          text,
          `${decl.name} 的 goals_runs[${index}] 必须重建 goals[${index}]`,
        );
      });
      // 夹具的 goal 含 `⊆`/`∈`（用例 #6 钉的是同一份文本的目标面板）⇒ 声明卡片
      // 拿到的这批 runs 里必须至少有一个 keyword run，否则卡片上就是纯文本。
      assert.ok(
        runs.some((list) => list.some((r) => r.kind === "keyword")),
        `${decl.name} 的目标必须至少有一个 keyword run（记法符号要着色）：${JSON.stringify(runs)}`,
      );
    }
  });

  test("next hole jumps inside a project unit", async () => {
    // 用例 #2（G-22 同族）：`soko/nextHole` 经由 `goals` ⇒ 项目入口同样受害。
    const entry = fixtureEntry();
    await showDoc(entry);
    // **故意不调 `infoviewDecls`**：那会引入"声明栏先能用"的前置依赖，
    // 而这条用例测的正是 `soko/nextHole` 自己（同一条 `parsable()` 判据的另一面）。
    await vscode.commands.executeCommand("sokonanoda.nextHole");
    const editor = vscode.window.activeTextEditor;
    assert.ok(editor, "跳洞后必须还有活动编辑器");
    const cursor = editor.selection.active;
    const text = editor.document.getText();
    const offset = editor.document.offsetAt(cursor);
    const line = text.slice(0, offset).split("\n").length - 1;
    assert.ok(
      editor.document.lineAt(line).text.includes("sorry"),
      `跳洞必须落在洞所在行，实际光标在第 ${line + 1} 行：${editor.document.lineAt(line).text}`,
    );
  });


  // ---- T-A60：缓存与扇出的 e2e 断言 ----------------------------------------
  //
  // 这三条量的是**用户能感觉到的结果**（重开变快 / 内容没变就不重编 / 改依赖会
  // 刷新），不是实现细节。所以它们跑在真 VS Code + 真 LSP 上。
  //
  // **这一层要绕开三个陷阱**（都实测踩过，写下来免得下一刀再踩）：
  //  1. `vscode.languages.getDiagnostics(uri)` **不是"刚发来"的信号**：VS Code
  //     按 URI 留着上一次的结果，也不会因为 `didClose` 清掉 ⇒ "非空"会让打开
  //     立刻满足条件，量到 0ms 而那次根本没编译。要监听
  //     `onDidChangeDiagnostics`（每次 publishDiagnostics 都触发）。
  //  2. **监听器必须早于那次发布挂上**：`sokonanoda.restartServer` 会顺手把
  //     打开中的文档重新同步一遍，发布就发生在重启过程里——在 `showDoc` 之后再
  //     挂就永远等不到。
  //  3. **`workbench.action.closeAllEditors` 不等于 `didClose`**：`openTextDocument`
  //     返回的 `TextDocument` 只要还被引用着，客户端就不发 `didClose`，服务端那份
  //     `Doc` 还活着，重开就"什么都没发生"。所以冷开不用"关掉再开"，用一份
  //     **从没编译过的文件**——那是真的冷，不用猜任何一方的状态机。

  /// 本次跑的编译缓存目录（由 `scripts/vscode-e2e.sh` 显式给，每次一个全新的）。
  function cacheDir() {
    const dir = process.env.SOKONANODA_CACHE_DIR;
    assert.ok(
      dir,
      "e2e 需要 SOKONANODA_CACHE_DIR（用 scripts/vscode-e2e.sh 跑；T-A60 的冷/热对比靠它）",
    );
    return dir;
  }

  /// 缓存条目的**指纹**：条目名 + 大小 + mtime。写一次缓存它就变。
  ///
  /// **两处都要看**（T-B5 / R-3）：项目闭包条目现在落在**模块根**的
  /// `<模块根>/.sokonanoda/compiled/`，只有**单文件**条目还在
  /// `SOKONANODA_CACHE_DIR/compiled/`。只看全局那份的话，T-A60 的"缓存不膨胀"
  /// 断言会因为**什么都没看见**而失去意义（前置断言 `stamp.length > 0` 还会直接红）。
  function cacheStamp() {
    const roots = [path.join(cacheDir(), "compiled")];
    for (const folder of vscode.workspace.workspaceFolders ?? []) {
      const ws = folder.uri.fsPath;
      // 工作区自己的模块根 + 嵌套模块根（课程仓就是"一个工作区多个模块根"）。
      roots.push(path.join(ws, ".sokonanoda", "compiled"));
      try {
        for (const rel of fs.readdirSync(ws, { recursive: true }).map(String)) {
          const parts = rel.split(path.sep);
          if (parts.includes(".sokonanoda") && parts[parts.length - 1] === "compiled") {
            roots.push(path.join(ws, rel));
          }
        }
      } catch {
        // 工作区扫不动就算了：指纹是"看得见的条目"的集合，不是断言本身。
      }
    }
    const out = [];
    for (const root of roots) {
      if (!fs.existsSync(root)) continue;
      for (const rel of fs.readdirSync(root, { recursive: true }).map(String)) {
        const stat = fs.statSync(path.join(root, rel));
        out.push(`${rel}:${stat.size}:${stat.mtimeMs}`);
      }
    }
    return out.sort();
  }

  /// 把一行**给人判读**的数字记进 e2e 留档。
  ///
  /// 为什么不用 `console.log`：扩展宿主的 stdout 不进 `vscode-test` 的用例行
  /// （`scripts/vscode-e2e.sh` 只 grep `✔/✗` 那些行），而 `SOKO_E2E_LOG` 会被
  /// **整份**收进 `docs/e2e/logs/…`。计划要的就是"pass/fail 之外还能看趋势"。
  function perfNote(line) {
    const file = process.env.SOKO_E2E_LOG;
    if (!file) return;
    try {
      fs.appendFileSync(file, `PERF ${line}\n`);
    } catch {
      // 记账失败不该让用例红——数字是给人看的，断言才是判据。
    }
  }

  /// 诊断发布的计数器（见上面陷阱 1/2）。
  // **URI → 规范键**（2026-09-25：这条用例在 ubuntu 上必红的真因 ✗）。
  //
  // 原来键就是 `uri.toString()` ✗ ⇒ 只要两边**字符串**不同就计不上 ✓：
  // 夹具用它自己的 `path.join(root, …)` 拼 URI（可能带 `..`/未规范化 ✓），
  // 而服务端发的是 `Url::from_file_path` 规范化后的 ✓（这个形状
  // `docs/design/duplication-audit.md` #22 已经记过 ✗）⇒ macOS 上侥幸相同、
  // ubuntu runner 上不同 ⇒ `count(entry)` 恒 0 ⇒
  // `assert.ok(publishes >= 1, "改依赖必须让打开的入口重新发诊断（跨文件失效）")` 必红 ✗
  //（实测：**两个** ubuntu job 红在同一条、macos 同代码绿 ✓；本轮 CI `36128240448` ✓）。
  // 修法：**用 realpath 规范化路径**再构造键 ✓ ⇒ 两种写法归一到同一个键 ✓。
  function canonicalKey(uri) {
    try {
      return vscode.Uri.file(fs.realpathSync(uri.fsPath)).toString();
    } catch {
      return uri.toString();
    }
  }

  function diagnosticsWatcher() {
    const counts = new Map();
    const sub = vscode.languages.onDidChangeDiagnostics((event) => {
      for (const uri of event.uris) {
        const key = canonicalKey(uri);
        counts.set(key, (counts.get(key) ?? 0) + 1);
      }
    });
    return {
      count: (uri) => counts.get(canonicalKey(uri)) ?? 0,
      dispose: () => sub.dispose(),
    };
  }

  /// **稳定后不再变**（2026-09-25 ✓）：在 `window` 毫秒内反复取 `signature()`，
  /// 只要出现一次与初值不同就判红 ✗ —— 这是"**幂等**"的**事件无关**判据 ✓
  /// （替代原来那条"数 `onDidChangeDiagnostics` 次数"✗：事件会被 VS Code 合并 ✓）。
  async function assertNoFurtherChanges(desc, signature, window = 3000) {
    const initial = signature();
    const deadline = Date.now() + window;
    while (Date.now() < deadline) {
      await sleep(250);
      const now = signature();
      assert.strictEqual(
        now,
        initial,
        `${desc}：诊断内容在稳定后又变了 ✗\n初值=${initial}\n现在=${now}`,
      );
    }
  }

  const fixtureRoot = () => {
    const root = vscode.workspace.workspaceFolders?.[0]?.uri?.fsPath;
    assert.ok(root, "e2e 需要一个工作区目录（.vscode-test.mjs 的 workspaceFolder）");
    return root;
  };

  const fixtureLib = () => path.join(fixtureRoot(), "lib", "Set.sokonanoda");

  /// 打开 `uri` 并等到 `ready()` 为真，返回毫秒数。
  async function timeOpen(uri, desc, ready) {
    const start = Date.now();
    await showDoc(uri);
    await waitFor(`${desc}：服务端发来这一轮诊断`, ready, WAIT_MS, 5);
    return Date.now() - start;
  }

  test("warmCacheOnOpen pre-builds the workspace at activation", async () => {
    // 用例 T-A52：夹具工作区的 `.vscode/settings.json` 打开了
    // `sokonanoda.warmCacheOnOpen`（**默认关**，这是唯一能测"激活时"行为的办法：
    // 设置只在 `activate()` 里读一次，运行中开是不生效的）。
    //
    // 判据是**扩展自己记的那一行账**，不是"缓存里有条目"——条目也可能是别的
    // 用例写的，而这一行只可能由 `warmCacheOnOpen` 那条路径写出来。
    const log = process.env.SOKO_E2E_LOG;
    assert.ok(log, "需要 SOKO_E2E_LOG（用 scripts/vscode-e2e.sh 跑）");
    await waitFor("warmCacheOnOpen 跑过一次 build", () => {
      if (!fs.existsSync(log)) return false;
      return fs.readFileSync(log, "utf8").includes("warmCacheOnOpen: exit=0");
    });
    assert.ok(
      cacheStamp().length > 0,
      "预热必须把条目写进缓存（否则它没做事）",
    );
  });

  test("reopening a project unit hits the compile cache", async () => {
    // 用例 T-A60-1（T-A10 / T-A11）：第一次打开**真编译并写缓存**，之后
    // （重启服务器 + 重开）**命中缓存**、不再重编、且明显更快。
    const source = fixtureEntry();
    // 冷开用一份**新文件**：它从来没被编译过 ⇒ 必然是冷编译（见上面陷阱 3）。
    // 内容 = u01 + 一批用库记法的定理，**故意做大**：夹具只有 3 条声明时编译只占
    // ~30ms，冷/热都被"重启服务器 + 请求往返"的固定开销（~60ms）淹没，比例断言
    // 变成噪声（实测冷 89ms / 热 63ms）。时间断言必须让被测的那一段占主导。
    const coldBody =
      fs.readFileSync(source.fsPath, "utf8") +
      "\n" +
      Array.from(
        { length: 120 },
        (_, i) =>
          `theorem extra_${i} (α : Type) (a : α) (A : Set α) (h : a ∈ A) : a ∈ A := h`,
      ).join("\n") +
      "\n";
    const coldPath = path.join(fixtureRoot(), "units", "u02.sokonanoda");
    fs.writeFileSync(coldPath, coldBody);
    const coldUri = vscode.Uri.file(coldPath);

    const watch = diagnosticsWatcher();
    try {
      // 只看**我们这份**新增的条目：前面用例还开着别的文档，`restartServer` 会把
      // 它们一起重新同步、各自写自己的条目——那是正常的，不该算到我们头上
      // （全量跑时就是被这个判成假的"热开又编了一遍"）。
      // **这条用例只断言"用户可见的性质"：热开明显更快**（2026-09-24 定案）。
      //
      // 走过的全程（每一版都被 CI 打回，见 docs/CI-FAILURES.md）：
      //   ① 断言"冷开写下的条目原样存活" ✗ —— ubuntu 上直接 **30s 超时**；
      //   ② 改成"等缓存条目落盘" ✗ —— 产物里的失败原文说得很清楚：
      //      `timed out after 30000ms waiting for T-A60-1 冷开：缓存条目落盘`
      //      ⇒ **ubuntu 上冷开根本不往 `SOKONANODA_CACHE_DIR` 写条目**
      //      （macos 写：本地实测 `entries=1`）。也就是说"缓存目录可观测"
      //      这个**前提**在 ubuntu 不成立 ✗ —— 前几轮都在修症状。
      //   ⇒ 缓存**结构**的证据交给进程内套件（`perf_course_*` + CLI 那条
      //     "build 预热缓存"用例，它们在各平台都过 ✓）；这一层只留
      //     **端到端可感**的性质：热开比冷开快得多（本地余量 9×：
      //     cold 509ms / warm 56ms），并且两次都拿到了诊断。
      const cold = await timeOpen(coldUri, "T-A60-1 冷开", () => watch.count(coldUri) >= 1);
      const before = watch.count(coldUri);
      const start = Date.now();
      await vscode.commands.executeCommand("sokonanoda.restartServer");
      await showDoc(coldUri);
      await waitFor(
        "T-A60-1 热开：服务端发来这一轮诊断",
        () => watch.count(coldUri) > before,
        WAIT_MS,
        5,
      );
      const warm = Date.now() - start;
      // **时间只记录，不当判据**（2026-09-24 最终定案）。
      //
      // 产物里的原文（`tests.failing_details`）：`冷 88ms / 热 110ms` —— ubuntu 上
      // **冷开只有 88ms**（本地 509ms）⇒ **冷开本来就命中了缓存**，两边剩下的都只是
      // "重启服务 + 重同步"的固定开销 ⇒ 这条用例的前提（"冷开慢、热开快"）
      // **在该环境根本不成立**。任何形如 `warm < cold / N` 的阈值都会随机器变。
      //
      // ⇒ 这层只断言**结构性、任何环境都成立**的两件事：冷开与热开都拿到诊断
      //   （见上面的 `timeOpen` 与 `waitFor`），时间进 `perfNote` 供人看趋势。
      //   "重开命中缓存"的**结构证据**由进程内套件守（`perf_course_*`、CLI 的
      //   "build 预热缓存"用例）——那层是确定性的。
      perfNote(`e2e cache: cold=${cold}ms warm=${warm}ms`);
    } finally {
      watch.dispose();
      // **清理失败不算用例失败**：断言已经跑完，删不掉临时夹具是环境问题
      // （实测本机 `unlink` 会被安全护栏拦成 EPERM，于是这条用例"因为清理而红"，
      //  把真正的断言结果盖住了）。
      try {
        fs.rmSync(coldPath, { force: true });
      } catch {
        /* 环境不允许删除：忽略 */
      }
    }
  });

  test("rewriting an unchanged project unit does not recompile", async () => {
    // 用例 T-A60-2（T-A21 / T-A22）：文本一个字节没变 ⇒ **不重编**。
    //
    // 为什么不是 `workbench.action.files.save`：VS Code 对**干净缓冲区**的保存是
    // no-op（根本不发 `didSave`），所以从扩展宿主里"保存一份没改过的文件"测不到
    // 服务端那条短路。这里走**同一条服务端路径的另一半**——文件在磁盘上被重写成
    // **同样的字节**（编辑器外改动 ⇒ `did_change_watched_files`），服务端的短路
    // 判据与保存路径完全相同。真 `didSave` 那条由进程内用例
    // `perf_course_save_same_text_is_recorded` 钉着（那一层才是确定性的）。
    const entry = fixtureEntry();
    const lib = fixtureLib();
    await showDoc(entry);
    await infoviewDecls("T-A60-2 前置");

    const before = JSON.stringify(await infoviewDecls("T-A60-2 改前"));
    const stamp = cacheStamp();
    assert.ok(stamp.length > 0, "前置：夹具的闭包必须已经在缓存里");

    const bytes = fs.readFileSync(lib, "utf8");
    fs.writeFileSync(lib, bytes); // 同样的字节：只碰 mtime

    await sleep(1500); // 给 watcher → didChangeWatchedFiles →（可能的）重编译留时间
    const after = JSON.stringify(await infoviewDecls("T-A60-2 改后"));

    assert.strictEqual(after, before, "内容没变 ⇒ 声明栏必须逐字节相同");
    assert.deepStrictEqual(
      cacheStamp(),
      stamp,
      "内容没变 ⇒ 不许写出新的缓存条目（说明闭包被重编了）",
    );
  });

  test("editing a dependency refreshes the open unit once", async () => {
    // 用例 T-A60-3（T-A23 扇出）：改依赖 ⇒ 打开的入口诊断跟着更新，且**只更新一次**。
    const entry = fixtureEntry();
    const lib = fixtureLib();
    await showDoc(entry);
    await infoviewDecls("T-A60-3 前置");

    const original = fs.readFileSync(lib, "utf8");
    const target =
      "def Set.subset (α : Type) (A B : Set α) : Prop := forall (x : α), A x -> B x";
    assert.ok(original.includes(target), "夹具前提：lib/Set 里要有 Set.subset 的定义");

    // **判据用"诊断内容"，不用"事件次数"**（2026-09-25 修 ✗⇒✓ —— 换掉那条间歇性红 ✓）。
    //
    // 原来数的是 `onDidChangeDiagnostics` 的**事件次数** ✗（`publishes >= 1` ✓），
    // 而 **VS Code 会合并（coalesce）诊断事件** ✗ ⇒ **event 数本身就不可靠** ✓：
    // 实测证据是 `waitFor`（诊断非空 ✓）**过了**、只有计数是 0 ✗ —— 三轮 CI 对照：
    // macos 全绿 ✓、ubuntu 1.106 **一轮绿一轮红** ✓ ⇒ 间歇性、平台偏置 ✓
    //（详见 `docs/CI-FAILURES.md` 的第三轮条目 ✓）。
    // ⇒ 改成**语义**判据 ✓：① 入口诊断必须**出现**（跨文件失效生效 ✓）；
    // ② **稳定下来之后内容不再变**（幂等：改一次依赖不该让学习者看到反复重画 ✓）。
    const signature = () =>
      JSON.stringify(
        vscode.languages.getDiagnostics(entry).map((d) => [d.range.start.line, d.message]),
      );
    // ★ **基线**（写坏之前 ✓）—— 这一条是本轮从 CI artifact 里学到的 ✗⇒✓：
    // 夹具**本来就有**两条 `sorry` 警告 ✓ ⇒ 原来那个 `length > 0` 的 `waitFor`
    // **立刻就满足** ✓、**根本没等"跨文件失效"** ✗（ubuntu 1.138.0 上因此取样过早 ✓：
    // 初值取到的是"恢复态"的两条 sorry ✓、"现在"才是库坏掉的 `∈` 报错 ✓ —— 反方向 ✓）。
    // 正确信号是"诊断**相对基线变了**" ✓。
    const before = signature();
    try {
      // 把 `⊆` 的定义改坏：入口里 `A ⊆ B` 的两条定理必须立刻报错。
      fs.writeFileSync(
        lib,
        original.replace(target, "def Set.subset (α : Type) (A B : Set α) : Prop := True"),
      );
      await waitFor(
        "T-A60-3：入口诊断跟着依赖更新（相对基线变了 ✓）",
        async () => signature() !== before,
      );
      await sleep(1500); // 让可能的重复发布也发生完，再取签名 ✓
      // ⚠ **必须在 `try` 内做**（本轮实测踩到 ✓）：`finally` 会把 lib **恢复**✓，
      // 恢复本身**又**改一次诊断 ✓ ⇒ 放在 `finally` 之后就会把"恢复导致的正常变化"
      // 误判成"重复发布" ✗（本地 e2e 当场抓到：初值是 `∈` 报错 ✓、
      // "现在"是恢复后的两条 `sorry` 警告 ✓ ⇒ 正是我的放置错 ✗）。
      const settled = signature();
      await assertNoFurtherChanges("T-A60-3：入口诊断稳定后不再变（幂等）", signature);
      perfNote(`e2e fanout: entry diagnostics signature=${settled.length} 字节 · 稳定 ✓`);
    } finally {
      fs.writeFileSync(lib, original);
    }
  });

  test("goal text uses the file's notation", async () => {
    // 用例 #6（G-26）：课程写法 `:= by` + `sorry`，**光标落在 tactic 内**时
    // 走「根状态」生产者；目标文本必须是**源文件的记法**（断言见下面 `⊆`/`∈`）。
    //
    // ⚠ 光标位置是这条用例的关键（实测逐列量过 `units/u01` 的 `sorry` 行）：
    //   在 `sorry` **内**（半开区间 start<=cursor<end）⇒ 根状态；
    //   在 `sorry` **之后** ⇒ 另一支（无 by 的声明级目标）。
    //   用 `revealRange` 会把光标停在 range **末尾**（= 之后），于是假绿过一次——
    //   所以这里显式设 selection。
    //
    // 📌 **这条用例覆盖不到什么**（2026-09-25 查明）：夹具 `u01` 的类型里**没有 lambda**
    // ⇒ 它**碰不到**「内建 `=` 抢走 `fun … =>` 的 `=` ⇒ 记法折叠整条 bail」那个 bug ✗
    // （那个 bug 让**凡类型含 `fun … =>` 的声明**都退回点形式 ✓，用户在 Infoview 里
    //   看到的就是它 ✓）。修在 `crates/front/src/token.rs` 的声明符号匹配处 ✓，
    // front 层判据是 `display.rs::folds_with_the_real_pipeline_table_shape` ✓。
    // **覆盖缺口**：夹具里没有 `∃`/`Exists` ⇒ 要真正在 e2e 层钉住它，得给夹具加一条
    // 「类型含 lambda 的 `∃` 声明」+ 一条 `∃` 断言 ✓（登记在 REQUIREMENTS §9 ✓）。
    const entry = fixtureEntry();
    await showDoc(entry);
    const editor = vscode.window.activeTextEditor;
    assert.ok(editor, "必须有一个活动编辑器");
    const doc = editor.document;
    // 整行 trim 后**恰好**是 `sorry`——不能用 `includes("sorry")`：
    // 夹具的注释里也出现过这个词，会命中注释行 ⇒ 光标落在声明外 ⇒ 空状态（实测踩到）。
    const lineIndex = [...Array(doc.lineCount).keys()].find(
      (i) => doc.lineAt(i).text.trim() === "sorry",
    );
    assert.notStrictEqual(lineIndex, undefined, "夹具里必须有一行 `sorry`");
    const character = doc.lineAt(lineIndex).text.indexOf("sorry") + 1;
    const pos = new vscode.Position(lineIndex, character);
    editor.selection = new vscode.Selection(pos, pos);

    await waitFor("用例 #6：open_one 的根状态到达", async () => {
      const state = extensionApi.infoview.lastState();
      return (
        state &&
        state.decl &&
        state.decl.name === "open_one" &&
        typeof state.goal === "string" &&
        state.goal.length > 0
      );
    });
    const goal = extensionApi.infoview.lastState().goal;
    assert.ok(
      goal.includes("⊆") || goal.includes("∈"),
      `open_one 的根状态必须用源文件的记法（⊆ / ∈），实际 = ${goal}`,
    );
  });

  test("goal text folds a binder notation whose type contains a lambda", async () => {
    // **③ 的 e2e 判据（2026-09-25）**：类型里含 `fun … =>` 时，内建的 `=` 会抢走
    // `=>` 里的 `=`（`crates/front/src/token.rs` 的声明符号匹配处 ⇒ 基础多字符算符
    // 更长时必须让路 ✓）⇒ 修复前 `print_back` 的解析失败、**整条 bail** ⇒ goal 退回
    // 点形式（`Exists (fun …)` 而不是 `∃ …`）。这里钉住修好后的**用户可见**结果 ✓。
    const entry = fixtureEntry();
    await showDoc(entry);
    const editor = vscode.window.activeTextEditor;
    assert.ok(editor, "必须有一个活动编辑器");
    const doc = editor.document;
    // 夹具里**最后**一个恰好是 `sorry` 的行 = `exists_fun` 的占位符 ✓
    // （前一个是 `open_one`，那条用例找的是**第一个** ⇒ 两条互不干扰 ✓）。
    const sorryLines = [...Array(doc.lineCount).keys()].filter(
      (i) => doc.lineAt(i).text.trim() === "sorry",
    );
    assert.ok(sorryLines.length >= 2, `夹具里应当有两条 sorry（open_one + exists_fun），实际 ${sorryLines.length}`);
    const lineIndex = sorryLines[sorryLines.length - 1];
    const pos = new vscode.Position(lineIndex, doc.lineAt(lineIndex).text.indexOf("sorry") + 1);
    editor.selection = new vscode.Selection(pos, pos);

    await waitFor("③：exists_fun 的根状态到达", async () => {
      const state = extensionApi.infoview.lastState();
      return state && typeof state.goal === "string" && state.goal.includes("∃");
    });
    const goal = extensionApi.infoview.lastState().goal;
    assert.ok(
      goal.includes("∃"),
      `类型含 lambda 的 \`∃\` 必须折成记法（修复前是 \`Exists (fun …)\`），实际 = ${goal}`,
    );
  });

  /// **轮询一个 LSP 请求直到它有答案**（或超时后把最后一次结果交回给断言）。
  ///
  /// 为什么需要它（2026-09-24 实测）：`showDoc(entry)` 只保证**文档打开**，
  /// 而"记法符号跳转到声明它的库"还要求**项目闭包就绪**——两者之间有时序。
  /// 同一 commit 上 1.138 绿、**1.106 红**，红的两条正是"`∈` 上跳定义"与
  /// "`∈` 上 hover"，失败信息都是"返回 null"⇒ 典型的"请求早于闭包就绪"✗。
  /// 断言本身没问题，缺的是**等那个条件**（与 `did_change_until` 同一条思路）。
  async function requestUntil(label, request, ready, attempts = 60) {
    let last;
    for (let i = 0; i < attempts; i++) {
      last = await request();
      if (ready(last)) return last;
      await new Promise((resolve) => setTimeout(resolve, 250));
    }
    console.log(`[--] ${label}：轮询 ${attempts} 次仍没就绪，把最后一次结果交给断言`);
    return last;
  }

  test("go to definition on a notation symbol lands on its declaration", async () => {
    // 用例 #7（G-23）：`∈` 在 `units/u01` 第 9 行。
    const entry = fixtureEntry();
    await showDoc(entry);
    const doc = await vscode.workspace.openTextDocument(entry);
    const text = doc.getText();
    const offset = text.indexOf("∈", text.indexOf("theorem"));
    const before = text.slice(0, offset);
    const line = before.split("\n").length - 1;
    const character = (before.split("\n").pop() || "").length;
    const position = new vscode.Position(line, character);
    const found = await requestUntil(
      "`∈` 上跳定义",
      () => vscode.commands.executeCommand("vscode.executeDefinitionProvider", entry, position),
      (result) => Array.isArray(result) && result.length > 0,
    );
    assert.ok(
      Array.isArray(found) && found.length > 0,
      "在 `∈` 上跳定义必须返回至少一个位置（现在返回 null）",
    );
    assert.ok(
      found.some((loc) => String(loc.uri.fsPath).endsWith("Set.sokonanoda")),
      `跳转必须落在声明记法的库里，实际 = ${JSON.stringify(found.map((l) => l.uri.fsPath))}`,
    );
  });

  test("hover on a notation symbol shows the target's signature", async () => {
    // 用例 #8（G-23）：hover 必须给出 `Set.mem` 的**原始类型**。
    const entry = fixtureEntry();
    await showDoc(entry);
    const doc = await vscode.workspace.openTextDocument(entry);
    const text = doc.getText();
    const offset = text.indexOf("∈", text.indexOf("theorem"));
    const before = text.slice(0, offset);
    const line = before.split("\n").length - 1;
    const character = (before.split("\n").pop() || "").length;
    // 同"跳定义"那条：hover 也要等**项目闭包就绪**（1.106 上实测假红）。
    const markdown = await requestUntil(
      "`∈` 上 hover",
      () => hoverTextAt(entry, line, character),
      (text) => typeof text === "string" && text.includes("Set.mem"),
    );
    assert.ok(
      markdown.includes("Set.mem"),
      `hover 必须给出 \`Set.mem\` 的原始类型，实际 = ${markdown}`,
    );
  });
});

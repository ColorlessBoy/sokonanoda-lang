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

  /// 项目根下 `.sokonanoda` 文件的个数 —— **与 CLI 的 `build <dir>` 同一口径**
  /// （`crates/cli/src/build.rs::collect_files`：递归、只认 `.sokonanoda` 扩展名）。
  /// E22 的 e2e 判据拿它当期望值，免得把夹具的文件数写死。
  function countSokonanodaFiles(root) {
    let count = 0;
    const walk = (dir) => {
      for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
        const full = path.join(dir, entry.name);
        if (entry.isDirectory()) walk(full);
        else if (entry.name.endsWith(".sokonanoda")) count += 1;
      }
    };
    walk(root);
    return count;
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
    // **同一个缺陷、更便宜的判据已有 front 版**（`hover_text_is_folded_like_the_lsp_does`）；
    // 这一份走**真宿主 + 真 hover** —— 它才是"用户看得见"的确认（docs/design/e2-plan.md T-U12）。
    //
    // ⚠ **复用现有夹具，绝不新建文件**（2026-09-26 实测的教训）：
    // 第一版把夹具写进 tmpDir —— 那个文件会被编译 ⇒ **写进全局缓存**
    // ⇒ 在**慢速 CI** 上这次落盘**迟到**，正好落进后面那条
    // "rewriting an unchanged project unit does not recompile" 的 1.5s 窗口
    // ⇒ 它的 `cacheStamp()` 多出一条 ⇒ **它红了**（本机快，所以本机一直绿）。
    // 而夹具里本来就有：`lib/Set.sokonanoda` 声明 `infix:50 " ⊆ " => Set.subset`，
    // 且 `units/u01.sokonanoda:11` 的 `subset_mem` 类型正是 `A ⊆ B -> A ⊆ B`。
    const entry = fixtureEntry();
    await showDoc(entry);
    await infoviewDecls("T-U12 面 #3 前置");
    // ⚠ **行列号从夹具文本推导** ✓ —— 原来写死 `(10, 10)` ✗：
    // 夹具一改行数，那个坐标就**静默指到别处** ✗（测试可能仍"绿"却测了别的东西 ✗）。
    const lines = fs.readFileSync(entry.fsPath, "utf8").split("\n");
    const line = lines.findIndex((l) => l.includes("theorem subset_mem"));
    assert.ok(line >= 0, "夹具前提：u01 里要有 `theorem subset_mem`");
    const col = lines[line].indexOf("subset_mem");
    assert.ok(col >= 0, "夹具前提：该行要有 `subset_mem` 这个名字");
    const text = await hoverTextAt(entry, line, col);
    assert.ok(text.includes("⊆"), `hover 应含记法 ⊆（实际 = ${text}）`);
    assert.ok(
      !text.includes("Set.subset "),
      `hover 不应漏点形式 Set.subset（实际 = ${text}）`,
    );

    // **E04 面**：上面那条悬停的是**声明名**（走 `ty_text`，早就折了）。
    // 这一条悬停的是**子表达式 `h`（假设的使用处）** —— 它的类型文本走
    // `resolve_hovers`，**E04 之前是内核 pp 直出** ✗ ⇒ 屏幕上会显示
    // `h : Set.subset α A B` ✗。E04 起同一个折叠入口 ⇒ `h : A ⊆ B` ✓。
    const bodyCol = lines[line].lastIndexOf("h");
    assert.ok(bodyCol > col, "夹具前提：该行末尾要有假设 `h` 的使用处");
    const bodyText = await hoverTextAt(entry, line, bodyCol);
    assert.ok(
      bodyText.trim().length > 0,
      `假设使用处的 hover 不该是空的（实际 = ${JSON.stringify(bodyText)}）`,
    );
    assert.ok(
      bodyText.includes("⊆"),
      `假设使用处的类型面应折成记法 ⊆（E04；实际 = ${bodyText}）`,
    );
    assert.ok(
      !bodyText.includes("Set.subset "),
      `假设使用处的类型面不应漏点形式 Set.subset（E04；实际 = ${bodyText}）`,
    );
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

  test("notation input: greek letters and the anon-ctor brackets, in a real host", async () => {
    // 用户反馈（2026-10-01）：「`α` 没有快捷输入（如 `\a` 或 `\alpha`）」。
    // 希腊字母与 `⟨`/`⟩` 走的是**另一条** hover 路（标识符 / 语法括号，不是记法
    // 符号 ⇒ `notation_symbol_hover` 够不着），所以真宿主里各验一次——stub 宿主
    // 验的是状态机，这里验的是「用户屏幕上真的打出来了、hover 真的说了」。
    const source = "theorem g (α : Prop) (h : α) : α := h\n";
    const uri = await writeDoc("notation-input-greek.sokonanoda", source);
    const doc = await vscode.workspace.openTextDocument(uri);
    const editor = await vscode.window.showTextDocument(doc, { preview: false });

    // hover：光标落在**绑定变量** `α` 上（不是符号），文案必须说怎么打。
    let hover = "";
    await waitFor("a hover that teaches \\alpha", async () => {
      hover = await hoverTextAt(uri, 0, source.indexOf("α"));
      return hover.includes("\\alpha");
    });
    assert.ok(hover.includes("\\alpha"), `hover must teach \\alpha, got: ${hover}`);
    assert.ok(!hover.includes("记法符号"), `α 是标识符，不是记法符号: ${hover}`);

    // 打字：`\a`（用户点名的短键）+ Tab → `α`；`\alpha` 同样。
    await editor.edit((builder) => builder.insert(new vscode.Position(0, 0), "\\a\n"));
    editor.selection = new vscode.Selection(0, 2, 0, 2);
    await vscode.commands.executeCommand("sokonanoda.input.replaceAbbreviation");
    assert.strictEqual(
      doc.lineAt(0).text,
      "α",
      `\\a + Tab must produce α, got: ${doc.lineAt(0).text}`,
    );

    await editor.edit((builder) => builder.insert(new vscode.Position(0, 0), "\\alpha\n"));
    editor.selection = new vscode.Selection(0, 6, 0, 6);
    await vscode.commands.executeCommand("sokonanoda.input.replaceAbbreviation");
    assert.strictEqual(
      doc.lineAt(0).text,
      "α",
      `\\alpha + Tab must produce α, got: ${doc.lineAt(0).text}`,
    );

    // `\<`（全表唯一的非字母缩写）：真编辑器里也要能出 ⟨。
    await editor.edit((builder) => builder.insert(new vscode.Position(0, 0), "\\<\n"));
    editor.selection = new vscode.Selection(0, 2, 0, 2);
    await vscode.commands.executeCommand("sokonanoda.input.replaceAbbreviation");
    assert.strictEqual(
      doc.lineAt(0).text,
      "⟨",
      `\\< + Tab must produce ⟨, got: ${doc.lineAt(0).text}`,
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
    //
    // **E22 起目标是「项目」**（用户 I2①：「点哪个文件，编译哪个文件」）：
    // 活动文件是夹具入口 `units/u01.sokonanoda`（有 `import lib.Set`）⇒ 模块根
    // = 夹具工作区根 ⇒ 编的必须是**整个项目**，而不是活动文件那一个。
    // 判据钉在**用户看得见的数字**上：通知里的 `N 个文件`（= CLI 的
    // `build.summary.files`）—— 修前这里是 1（只有活动文件）。
    const commands = await vscode.commands.getCommands(true);
    for (const id of ["sokonanoda.build", "sokonanoda.rebuild"]) {
      assert.ok(commands.includes(id), `${id} must be registered`);
    }
    const entry = fixtureEntry();
    await showDoc(entry);
    // build 的目标取自服务端 `soko/project` 的模块根 ⇒ 先等那份答案到
    // （与声明栏/项目树同一份，客户端不自己找清单）。
    await waitFor("E22：项目视图就绪（build 的目标取它的 root）", async () => {
      return typeof extensionApi?.project?.answer?.project?.root === "string";
    });
    const root = extensionApi.project.answer.project.root;
    const expected = countSokonanodaFiles(root);
    assert.ok(
      expected > 1,
      `夹具项目必须不止一个文件，否则这条断言会空转（数到 ${expected} 个）`,
    );
    const built = await vscode.commands.executeCommand("sokonanoda.build");
    assert.strictEqual(typeof built, "string", "build must return its summary text");
    const files = Number((/build：(\d+) 个文件/.exec(built) ?? [])[1]);
    assert.strictEqual(
      files,
      expected,
      `Build 必须编**整个项目**（${expected} 个文件），不是活动文件那一个 —— 实际：${built}`,
    );
    const rebuilt = await vscode.commands.executeCommand("sokonanoda.rebuild");
    assert.strictEqual(typeof rebuilt, "string", "rebuild must return its summary text");
    const removed = Number((/清掉 (\d+) 条缓存/.exec(rebuilt) ?? [])[1]);
    assert.ok(
      rebuilt.includes("rebuild") && removed >= 1,
      "Rebuild 必须真的清掉**项目**的缓存条目（`--clean` 不带目标时实测是 0，" +
        `紧跟的 build 全是命中）—— 实际：${rebuilt}`,
    );
  });

  test("Clean Cache empties the project store and does not compile (E31)", async () => {
    // **E31（用户：「vscode 命令缺一个 clean，清除缓存」）** —— 这一条钉**用户看得见
    // 的结果**，而且是**实测数字**（不是"看起来清了"✗）：
    //   ① 先 `build` 把项目缓存**填上**（前置：条目数 N > 0）；
    //   ② 跑 `Clean Cache` ⇒ `<模块根>/.sokonanoda/compiled/` 条目数 **N → 0**；
    //   ③ **不触发重编**：clean 之后缓存目录必须**仍然是空的**（等一会儿再数一次）
    //      —— 这条专门防「clean 偷偷跟着编了一次」（= Rebuild 的行为）✗；
    //   ④ 报告里三个数都来自 CLI 的 `build.clean` 事件。
    const entry = fixtureEntry();
    await showDoc(entry);
    await waitFor("E31：项目视图就绪", async () => {
      return typeof extensionApi?.project?.answer?.project?.root === "string";
    });
    const root = extensionApi.project.answer.project.root;
    const compiledDir = path.join(root, ".sokonanoda", "compiled");
    const entries = () =>
      fs.existsSync(compiledDir) ? fs.readdirSync(compiledDir).filter((f) => f.endsWith(".json")) : [];

    await vscode.commands.executeCommand("sokonanoda.build");
    assert.ok(entries().length > 0, `前置：build 之后项目缓存里要有条目（实际 ${entries().length}）`);

    const text = await vscode.commands.executeCommand("sokonanoda.clean");
    assert.strictEqual(typeof text, "string", "clean 必须返回它的报告文本");
    assert.match(
      text,
      /^sokonanoda clean：清掉 \d+ 条缓存（全局 \d+ · 项目 \d+）（\d+ms）$/,
      `报告必须是三个数、且来自 CLI 事件：${text}`,
    );
    assert.strictEqual(
      entries().length,
      0,
      `Clean 之后项目条目必须 N → 0（实际还剩 ${entries().length}）`,
    );
    // ③ 不重编：给它时间，缓存目录必须**仍然**是空的。
    await sleep(1500);
    assert.strictEqual(
      entries().length,
      0,
      `Clean **不许**跟着重编（那是 Rebuild 的行为）：${JSON.stringify(entries())}`,
    );

    // **还原共享状态** ✓：这条用例把项目缓存清空了，而**后续用例默认夹具的闭包
    // 已经在缓存里**（如 T-A60-2 的前置 `stamp.length > 0`）—— 实测漏了这一步
    // ⇒ T-A60-2 当场判红（`前置：夹具的闭包必须已经在缓存里` ✗）。
    // 纪律：**动了共享缓存的用例要自己负责预热回去** ✓
    //（`docs/CI-FAILURES.md` 的同一条：共享缓存是跨用例的隐式状态）。
    await vscode.commands.executeCommand("sokonanoda.build");
    assert.ok(
      entries().length > 0,
      `收尾：必须把项目缓存预热回去，别把空缓存留给后面的用例（实际 ${entries().length}）`,
    );
  });

  test("Rebuild shows a non-zero percent while it is still running (②)", async () => {
    // **② 用户 2026-09-29 实测**：*「Rebuild 时 Infoview 长时间 `0%`、最后几秒突跳
    // 『编译已完成』」* —— 原话「属于**虚假的进度展现**」。
    //
    // ## 根因（本轮实测，在 CLI 侧量出来的）
    //
    // `runBuild` 的进度是**文件级**的，`percent = done/total`；而 `build.file`
    // 要等**全部**文件编译完才按 `files` 顺序重放（`crates/cli/src/build.rs`：
    // 并行编译阶段只往 `per_file` 里塞结果，**一条事件都不发**）⇒ 于是：
    //   * `build.begin` 一帧 `0%`；
    //   * 然后**整段编译期间一条 `build.file` 都没有**（实测冷编课程：首条
    //     `build.decl` 在 **160.8s** 之后）；
    //   * 重放阶段几十毫秒内把 42 条 `build.file` 全发完 ⇒ `0% → 100%` 突跳。
    //
    // ⇒ 这是**真缺陷**，不是观感问题：屏幕上确实从头到尾只有一个 `0%`。
    //
    // ## 判据（绑"屏幕上看什么"）
    //
    // 在**真 VS Code 宿主**里跑 `rebuild`，**轮询面板当前那一帧**，
    // 要求**至少有一帧** `0 < percent < 100`。修前这一条必然拿不到（只有 0 与 100）。
    //
    // ⚠ **不许靠"事件在流里出现过"代替** ✗ —— 用户看到的是**某一帧**，
    // 不是事件流的全集（这正是 §0 那条"判据必须绑用户可见结果"）。
    const entry = fixtureEntry();
    await showDoc(entry);
    await waitFor("②：项目视图就绪（rebuild 的目标取它的 root）", async () => {
      return typeof extensionApi?.project?.answer?.project?.root === "string";
    });

    // 前置：先 build 一次把缓存**填上**，这样 rebuild 是一次**真的冷编**
    // （`--clean` 之后全是 miss）—— 否则 rebuild 秒完，判据会**空转** ✗。
    await vscode.commands.executeCommand("sokonanoda.build");

    // 采样器：`lastProgress()` 就是 webview 收到的那一帧（`setProgress` 先存再发）。
    const frames = [];
    let sampling = true;
    const sampler = (async () => {
      while (sampling) {
        const frame = extensionApi?.infoview?.lastProgress?.();
        if (frame) frames.push({ ...frame, at: Date.now() });
        // **采样要够密**（2026-10-03 ✓）：这条判据问的是"**用户看到进度在走**" ✓，
        // 而用户的眼睛是**连续**的 ✓ ⇒ 采样 25ms 会在快机器上**漏掉中间帧** ✗
        //（CI 实测：macOS runner 上 rebuild 太快 ⇒ 只采到一帧 `[67]` ✗ ⇒ 断言 ② 判红 ✗）。
        // ⇒ 改成 **5ms** ✓ —— 这**不是**放宽判据 ✗（判据仍是"≥2 个不同的中间百分比" ✓），
        // 而是让**观测**忠实于用户所见 ✓（等价于把"人眼帧率"提上来 ✓）。
        await sleep(5);
      }
    })();

    const started = Date.now();
    let rebuilt;
    try {
      rebuilt = await vscode.commands.executeCommand("sokonanoda.rebuild");
    } finally {
      sampling = false;
      await sampler;
    }
    const wall = Date.now() - started;
    assert.strictEqual(typeof rebuilt, "string", "rebuild 必须返回摘要文本");

    // **自检（防空转）**：夹具不够慢 ⇒ "中途"根本不存在 ⇒ 判据无意义 ✗。
    assert.ok(
      wall >= 400,
      `rebuild 只跑了 ${wall}ms —— 太快的夹具会让"中途必须看到非 0%"**空转**，` +
        `必须让它真的编一会儿`,
    );

    const mid = frames.filter(
      (f) => typeof f.percent === "number" && f.percent > 0 && f.percent < 100,
    );
    assert.ok(
      mid.length > 0,
      `rebuild 中途**必须**至少有一帧非 0 百分比（用户实测的"长时间 0% 突跳"）—— ` +
        `采样 ${frames.length} 帧、耗时 ${wall}ms，percent 取值集合：` +
        `${JSON.stringify([...new Set(frames.map((f) => f.percent))])}`,
    );
    // 顺带钉住"它是**持续在动**、不是某一帧的毛刺"：至少两帧不同的中间值。
    assert.ok(
      new Set(mid.map((f) => f.percent)).size >= 2,
      `中间帧必须**持续在动**（用户要的是"进度在走"，不是一帧毛刺）：` +
        `${JSON.stringify(mid.map((f) => f.percent))}`,
    );
  });

  test("the Infoview receives the project view (E30)", async () => {
    // **E30**（用户：「监测到 toml 等项目信息也应该在 infoview 里展现出来」）。
    //
    // 真宿主这一层能断言的最强事实是"**webview 收到了什么**"（webview 的 DOM 在
    // iframe 里，扩展宿主够不到 ✗ —— 与 T-A5 同款）。渲染结果那一跳由
    // `test-webview.js` 的 stub DOM 承担（那里跑的是真的 `infoview.js`，
    // 断言 `requires_warning` **是可见文本、不是 tooltip**）。
    const entry = fixtureEntry();
    await showDoc(entry);
    // ⚠ **等的东西必须覆盖后面断言的东西** ✗✓ —— CI run `36340364435`（macos）实测红：
    //   原来只等 `infoview.lastProject()`，却在最后一条断言里读
    //   `extensionApi.project.answer.project.root`（**项目树**那份答案，走另一条路 ✗）
    //   ⇒ 慢 runner 上树还没答，`answer` 是 `undefined` ⇒
    //   `TypeError: Cannot read properties of undefined (reading 'project')` ✗。
    //   ⇒ 两个信号都等到再往下走 ✓。
    await waitFor("E30：项目载荷到达 Infoview 与项目树", async () => {
      const posted = extensionApi?.infoview?.lastProject?.();
      const answer = extensionApi?.project?.answer;
      return !!(posted && posted.project && answer && answer.project);
    });
    const posted = extensionApi.infoview.lastProject();
    assert.ok(
      typeof posted.project.root === "string" && posted.project.root.length > 0,
      `项目载荷必须带模块根：${JSON.stringify(posted)}`,
    );
    assert.ok(
      Array.isArray(posted.project.modules) && posted.project.modules.length > 0,
      `项目载荷必须带模块表（Infoview 要画模块行）：${JSON.stringify(posted.project.modules)}`,
    );
    assert.strictEqual(
      posted.project.root,
      extensionApi.project.answer.project.root,
      "Infoview 拿到的必须是**同一份**答案（不是第二次取数 ✗）",
    );
  });

  test("Infoview 'definition' lands exactly where the editor F12 lands (E27)", async () => {
    // **E27 的落点判据（PLAN §E27 判据 ②）**：真宿主 + 真 LSP。
    //
    // ⚠ **老实说**：webview 的 DOM 在 iframe 里，**e2e 点不到** ✗ ⇒ 这里从
    // **真的消息处理器**（`infoview._onMessage` —— 扩展侧唯一入口）喂一条
    // `definition` 消息，走完 `executeDefinitionProvider` → 落点的整条链；
    // 「点击真的发出 `definition`（而不是 `reveal`）」那一端由 webview 判据钉
    //（`test-webview.js::decls: clicking a name asks for the DEFINITION…`，
    // 含"不许发 reveal"的反向守卫）。**两条合起来才是完整的链** ✓；
    // 单看这一条**不算**"点得动"的真宿主 e2e ✗（如实记账，不记成 e2e 通过）。
    const entry = fixtureEntry();
    await showDoc(entry);
    const doc = await vscode.workspace.openTextDocument(entry);
    const text = doc.getText();
    // 位置取**记法符号 `∈`**（夹具 `units/u01` 第 9 行）：它是**跨文件**定义
    //（lib/Set 的 `Set.mem`）—— 与既有 F12 用例同款，也是 PLAN §E27 说的
    // "落点必须**跨文件正确**" ✓。
    // ⚠ 实测：本 LSP 的 definition 解析的是**使用处**，声明名本身**不返回定义**
    //（拿声明名当位置 ⇒ 空结果 ⇒ 面板如实说"这里没有可跳转的定义"）。⇒ 判据取
    // 使用处；**webview 目前接的是声明名**（见下面的如实记账）。
    const offset = text.indexOf("∈", text.indexOf("theorem"));
    assert.ok(offset >= 0, "夹具前提：u01 里要有记法符号 `∈`");
    const before = text.slice(0, offset);
    const line = before.split("\n").length - 1;
    const character = (before.split("\n").pop() || "").length;
    const position = new vscode.Position(line, character);

    // 基准：编辑器里按 F12（VS Code 自己的那条命令）。
    const editorLanding = await requestUntil(
      "E27 基准：编辑器 F12 的落点",
      () => vscode.commands.executeCommand("vscode.executeDefinitionProvider", entry, position),
      (result) => Array.isArray(result) && result.length > 0,
    );
    assert.ok(
      Array.isArray(editorLanding) && editorLanding.length > 0,
      "基准：编辑器 F12 必须能跳到定义（否则这条判据没有比较对象）",
    );
    assert.ok(
      editorLanding.some((loc) => String(loc.uri.fsPath).endsWith("Set.sokonanoda")),
      `基准必须落在库里（跨文件）：${JSON.stringify(editorLanding.map((l) => l.uri.fsPath))}`,
    );

    // Infoview 那条链：喂给**真的**处理器（同一条消息、同一个位置）。
    await extensionApi.infoview._onMessage({
      protocol: 1,
      type: "definition",
      uri: entry.toString(),
      position: { line, character },
    });
    const editor = vscode.window.activeTextEditor;
    assert.ok(editor, "跳转之后必须有活动编辑器");
    assert.strictEqual(
      editor.document.uri.toString(),
      editorLanding[0].uri.toString(),
      "Infoview 的落点文件必须与编辑器 F12 **逐字段相同**（uri）",
    );
    assert.strictEqual(
      editor.selection.start.line,
      editorLanding[0].range.start.line,
      "落点行必须与编辑器 F12 相同（line）",
    );
    // **如实记账**：webview 目前把**声明名**接到这条消息上（用户报的"声明名点不动"✓）；
    // 记法符号（`{a}`/`∈`）**还没接** ✗ —— 它们需要 wire 的 runs 带**位置**，
    // 而 runs 现在只有 `{text, kind}`（加位置是 front + LSP 的协议改动，另立条目）。
    // ⇒ 这条判据证的是**主机那一半链**（消息 → F12 语义 → 跨文件落点）✓，
    // **不是**"在 Infoview 里点 `∈` 能跳" ✗。
  });

  test("import 行的模块名是链接，指向被导入的文件（documentLink）", async () => {
    // **2026-10-08 用户**：「import 这一行的代码增加跳转功能，打开对应的文件」。
    //
    // 判据绑**用户动作的后果** ✓（AGENTS.md 验证设计纪律第 0 条 (a)）：服务端的
    // payload 由 LSP 单测钉（`crates/lsp/src/tests/project.rs` ✓），这里在**真 VS Code**
    // 里问 `vscode.executeLinkProvider` —— **编辑器渲染 documentLink 用的就是它** ⇒
    // 数据对了 ≠ 用户看见了，这一条才是"看得见"的那层 ✓。
    const uris = await writeProject("import-links", {
      "Logic.sokonanoda": "axiom And : Prop -> Prop -> Prop\n",
      "Entry.sokonanoda":
        "import Logic\n\ntheorem t (a b : Prop) : And a b -> And a b := fun (h : And a b) => h\n",
    });
    const entry = uris["Entry.sokonanoda"];
    await showDoc(entry);
    const doc = await vscode.workspace.openTextDocument(entry);
    const text = doc.getText();
    const offset = text.indexOf("Logic");
    assert.ok(offset >= 0, "夹具前提：入口第一行是 `import Logic`");
    const before = text.slice(0, offset);
    const line = before.split("\n").length - 1;
    const character = (before.split("\n").pop() || "").length;

    const links = await requestUntil(
      "documentLink：import 行的模块名",
      () => vscode.commands.executeCommand("vscode.executeLinkProvider", entry),
      (result) => Array.isArray(result) && result.length > 0,
    );
    const targets = links.map((link) => String(link.target?.fsPath ?? link.target ?? ""));
    assert.ok(
      targets.some((target) => target.endsWith("Logic.sokonanoda")),
      `链接必须指向**被导入的文件**：${JSON.stringify(targets)}`,
    );
    const link = links.find((l) => String(l.target?.fsPath ?? "").endsWith("Logic.sokonanoda"));
    assert.strictEqual(link.range.start.line, line, "range 必须落在 `import` 那一行");
    assert.strictEqual(link.range.start.character, character, "range 起点 = 模块名起点");
    assert.strictEqual(
      link.range.end.character,
      character + "Logic".length,
      "range 终点 = 模块名终点（盖整行会让点注释也跳 ✗）",
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
    // ⚠ **文件名不能撞 git 跟踪的夹具** ✗（2026-09-26 实测 ✓）：
    // 这里原本写 `u02.sokonanoda` ✗ —— 而**它是 git 跟踪的文件** ✓
    // ⇒ 本用例先**覆盖**它、再在 `finally` **删掉**它 ✗
    // ⇒ **每跑一次 e2e，工作区就多一个"已删除"的夹具** ✗
    // （CI 上被 `git status` 之外的地方掩盖，本地每次都要手工 `git checkout --` 还原 ✗）。
    // 换成一个**不会被跟踪**的名字 ✓ —— 目录不变（模块根不变 ✓）。
    const coldPath = path.join(fixtureRoot(), "units", "u02-cold-open.sokonanoda");
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
    // ⚠ **先等缓存静止，再取基线**（2026-09-27 实测，ubuntu-24.04 · VS Code 1.138.0 判红）：
    // 这一条判的是"同一份字节重写 ⇒ 不许写新条目"，而它的 1.5s 窗口对**邻居用例
    // 迟到的落盘**毫无免疫力 ✗ —— 那一轮基线之后多出 `949171fc…`（2101B）与
    // `c2a5a26a…`（5174B）两条，**两条都不是夹具闭包**（u01 的条目是 155KB 级、
    // `lib/Set` 单独编是 26KB 级 ⇒ 是别处的编译迟到了）。E22 起 build/rebuild
    // 真的编**整个项目**、rebuild 真的清项目缓存 ⇒ 共享夹具的缓存被扰动得更多
    // ⇒ 慢 runner 上更容易撞上。治法与 T-A60-3 同款：**先等稳定、再取基线**
    //（幂等判据，不赌时间 ✗）。判据本身一个字没松：基线之后仍必须逐条相等 ✓。
    let previous = cacheStamp();
    await waitFor("T-A60-2：缓存先静止（基线不许被迟到的落盘污染）", async () => {
      await sleep(400);
      const current = cacheStamp();
      const stable = JSON.stringify(current) === JSON.stringify(previous);
      previous = current;
      return stable;
    });
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
    // **从声明推导坐标**（2026-09-26 修）：原来取"**最后一个**恰好是 `sorry` 的行"，
    // 而那是**夹具形状**的假设 ✗ —— 夹具尾部一加题（今天加了 `singleton_use`），
    // 它就**静默指到别处**、还报"等 `∃` 超时"这种看不出真因的假红（本轮实测踩到）。
    // 改成：先找 `theorem exists_fun` 那一行，再取它**之后**第一条 `sorry`。
    const declLine = [...Array(doc.lineCount).keys()].find((i) =>
      doc.lineAt(i).text.includes("theorem exists_fun"),
    );
    assert.notStrictEqual(declLine, undefined, "夹具里必须有 `theorem exists_fun`");
    const lineIndex = [...Array(doc.lineCount).keys()].find(
      (i) => i > declLine && doc.lineAt(i).text.trim() === "sorry",
    );
    assert.notStrictEqual(lineIndex, undefined, "`exists_fun` 之后必须有一行 `sorry`");
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

  test("Infoview 的 `⊢` 用记法箭头 `→`，且 runs 与 text 同源", async () => {
    // **A1 的真宿主判据**（2026-09-26 用户报告第 1 条）：根状态的文本必须是
    // **唯一接口折过**的那一份 —— 修复前是混合形态（`∀` 折了、`->` 没折）。
    // 判据三件：① 出现 `→`；② **不得**出现 ASCII `->`；③
    // `goal_runs` 的文本拼接**逐字节等于** `goal`（不变量：同一次转化 ✓）。
    const entry = fixtureEntry();
    await showDoc(entry);
    const editor = vscode.window.activeTextEditor;
    assert.ok(editor, "必须有一个活动编辑器");
    const doc = editor.document;
    // **从声明推导坐标**（同上）：找 `theorem open_one`，再取它之后第一条 `sorry`。
    const declLine = [...Array(doc.lineCount).keys()].find((i) =>
      doc.lineAt(i).text.includes("theorem open_one"),
    );
    assert.notStrictEqual(declLine, undefined, "夹具里必须有 `theorem open_one`");
    const lineIndex = [...Array(doc.lineCount).keys()].find(
      (i) => i > declLine && doc.lineAt(i).text.trim() === "sorry",
    );
    assert.notStrictEqual(lineIndex, undefined, "`open_one` 之后必须有一行 `sorry`");
    const pos = new vscode.Position(lineIndex, doc.lineAt(lineIndex).text.indexOf("sorry") + 1);
    editor.selection = new vscode.Selection(pos, pos);

    await waitFor("A1：open_one 的根状态到达", async () => {
      const state = extensionApi.infoview.lastState();
      return (
        state && state.decl && state.decl.name === "open_one" && typeof state.goal === "string"
      );
    });
    const state = extensionApi.infoview.lastState();
    const goal = state.goal;
    assert.ok(
      goal.includes("→"),
      `\`⊢\` 后的文本必须用记法箭头 \`→\`（A1 之前是混合形态），实际 = ${goal}`,
    );
    assert.ok(
      !goal.includes("->"),
      `\`⊢\` 后的文本不许留 ASCII \`->\`，实际 = ${goal}`,
    );
    const runs = state.goal_runs || [];
    assert.ok(runs.length > 0, "`goal_runs` 必须非空（它决定高亮）");
    assert.strictEqual(
      runs.map((r) => r.text).join(""),
      goal,
      "`goal_runs` 的拼接必须逐字节等于 `goal`（同一次转化 ✓）",
    );
  });

  test("Infoview 里集合字面量显示成 `{a}`（不是 `Set.singleton α a`）", async () => {
    // **A2 的真宿主判据**（用户报告第 2 条）：`{a}` 是内建语法、展开成
    // `Set.singleton α a` ⇒ 显示面必须**折回** `{a}`。
    const entry = fixtureEntry();
    await showDoc(entry);
    const editor = vscode.window.activeTextEditor;
    assert.ok(editor, "必须有一个活动编辑器");
    const doc = editor.document;
    const lineIndex = [...Array(doc.lineCount).keys()].find(
      (i) => doc.lineAt(i).text.includes("singleton_use"),
    );
    assert.notStrictEqual(lineIndex, undefined, "夹具里必须有 `singleton_use`");
    const sorryLine = lineIndex + 1;
    assert.strictEqual(
      doc.lineAt(sorryLine).text.trim(),
      "sorry",
      "夹具前提：`singleton_use` 的下一行是 `sorry`",
    );
    const pos = new vscode.Position(sorryLine, doc.lineAt(sorryLine).text.indexOf("sorry") + 1);
    editor.selection = new vscode.Selection(pos, pos);

    await waitFor("A2：singleton_use 的根状态到达", async () => {
      const state = extensionApi.infoview.lastState();
      return (
        state && state.decl && state.decl.name === "singleton_use" && typeof state.goal === "string"
      );
    });
    const goal = extensionApi.infoview.lastState().goal;
    assert.ok(
      goal.includes("{a}"),
      `\`{a}\` 必须折回花括号写法，实际 = ${goal}`,
    );
    assert.ok(
      !goal.includes("Set.singleton"),
      `\`{a}\` 不许显示成点名展开 \`Set.singleton …\`，实际 = ${goal}`,
    );
  });

  test("go to definition on a `{a}` set literal lands on `Set.singleton`", async () => {
    // **A3 的真宿主判据**（用户报告第 3 条）：光标落在**左花括号**上按 F12
    // 必须跳到 `lib/Set.sokonanoda` 的 `Set.singleton`。
    //
    // ⚠ **已知限制**（判据取符号位，不取操作数）：`{a}` 内部那个 `a` 是**局部
    // 变量**，它自己有一条 `ResolvedTarget::Binder` 的 hover 行，而
    // "最小的使用点胜出" ⇒ 在 `a` 上按 F12 跳的是**变量**（与 Lean 一致：
    // `{a}` 展开成 `Set.singleton a`，那个 `a` 就是变量）。
    const entry = fixtureEntry();
    await showDoc(entry);
    const doc = await vscode.workspace.openTextDocument(entry);
    const text = doc.getText();
    const declAt = text.indexOf("singleton_use");
    assert.notStrictEqual(declAt, -1, "夹具里必须有 `singleton_use`");
    const offset = text.indexOf("{a}", declAt);
    assert.notStrictEqual(offset, -1, "`singleton_use` 的语句里必须有 `{a}`");
    const before = text.slice(0, offset);
    const position = new vscode.Position(
      before.split("\n").length - 1,
      (before.split("\n").pop() || "").length,
    );
    const found = await requestUntil(
      "`{a}` 上跳定义",
      () => vscode.commands.executeCommand("vscode.executeDefinitionProvider", entry, position),
      (result) => Array.isArray(result) && result.length > 0,
    );
    assert.ok(
      Array.isArray(found) && found.length > 0,
      "在 `{a}` 上跳定义必须返回至少一个位置（A3 之前返回 null）",
    );
    assert.ok(
      found.some((loc) => String(loc.uri.fsPath).endsWith("Set.sokonanoda")),
      "跳转必须落在 `Set.singleton` 的库里，实际 = " +
        JSON.stringify(found.map((l) => l.uri.fsPath)),
    );
  });

  test("go to definition on a prelude name lands in the prelude source", async () => {
    // **A4 的真宿主判据**（2026-09-26 用户报告第 4 条）：prelude 要像 Lean 4 ——
    // 有内嵌代码且**可跳转**。`Or` / `And` / `Iff` / `False` 是**内置前奏**，
    // 它们的 hover 行按设计 `resolution: None` ⇒ 以前 F12 **静默无反应**。
    //
    // 判据不是"返回了个位置就算"✗ —— 要**把跳过去的那个文件打开、读出那一行**，
    // 断言屏幕上看到的就是 `inductive Or …` ✓。
    const entry = fixtureEntry();
    await showDoc(entry);
    const doc = await vscode.workspace.openTextDocument(entry);
    const text = doc.getText();
    const declAt = text.indexOf("theorem prelude_or");
    assert.notStrictEqual(declAt, -1, "夹具里必须有 `theorem prelude_or`");
    const offset = text.indexOf("Or", declAt);
    assert.notStrictEqual(offset, -1, "`prelude_or` 的语句里必须有 `Or`");
    const before = text.slice(0, offset);
    const position = new vscode.Position(
      before.split("\n").length - 1,
      (before.split("\n").pop() || "").length,
    );
    const found = await requestUntil(
      "`Or` 上跳定义",
      () => vscode.commands.executeCommand("vscode.executeDefinitionProvider", entry, position),
      (result) => Array.isArray(result) && result.length > 0,
    );
    assert.ok(
      Array.isArray(found) && found.length > 0,
      "在 `Or` 上跳定义必须返回至少一个位置（A4 之前返回 null）",
    );
    const target = found[0];
    assert.ok(
      String(target.uri.fsPath).endsWith("Prelude.sokonanoda"),
      "跳转必须落在前奏源文件上，实际 = " + String(target.uri.fsPath),
    );
    // **把那个文件打开、读那一行** —— 用户看到的就是它。
    const preludeDoc = await vscode.workspace.openTextDocument(target.uri);
    const line = preludeDoc.lineAt(target.range.start.line).text;
    assert.ok(
      line.startsWith("inductive Or "),
      `跳过去的第 ${target.range.start.line} 行必须是 \`Or\` 的声明行，实际 = ${line}`,
    );
    assert.strictEqual(
      target.range.start.character,
      0,
      "声明行必须从行首开始（span 来自真 parser）",
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

  // ── 编辑响应延迟（性能，2026-10-01 用户反馈）───────────────────────────
  //
  // 用户原话：「在 VS Code 里编辑 .sokonanoda 太卡，输入后过很久才有反应
  // （诊断/目标面板更新），与 Lean4 不可比」。这条用例把**用户动作 → 屏幕上
  // 看得见的结果**量成毫秒，并且**两个结果都要**：
  //   ① 诊断更新（编辑器里的波浪线）；
  //   ② 目标面板拿到新的声明/目标（Infoview 的 `decls`/`state`）。
  // 只量①会漏掉面板那条链（用户看的是面板）；只量②会漏掉"诊断到了但没推"。
  //
  // ⚠ **绝对毫秒只做数量级兜底**（`AGENTS.md`「判据不许用绝对毫秒」）：
  // 断言的是**结构**——最后一个键必须真的引起一次诊断发布与一次面板更新；
  // 毫秒只写进 `perfNote` 供人判读趋势。**不许**把 500ms 之类写进 `assert`：
  // 共享 runner 上墙钟不可转移（实测本机 37s vs CI 278s = 7.5×）。
  test("edit responsiveness: last keystroke reaches diagnostics and the goal panel", async () => {
    // 夹具形状照着 `playground.sokonanoda`（29 条声明、400 行、无 import）：
    // 单文件、warm 缓存下服务端每次编译 ~0ms ⇒ 量到的就是**客户端**那一段。
    const lines = [];
    for (let i = 0; i < 24; i++) {
      lines.push(`theorem prior_${String(i).padStart(2, "0")} (P : Prop) (h : P) : P := h`);
    }
    const targetLine = lines.length; // 0-based 行号 = 前面的行数
    lines.push("theorem typing_target (P Q : Prop) (h : P) : Q → P := by");
    lines.push("  sorry");
    lines.push("");
    const uri = await writeDoc("typing-latency.sokonanoda", lines.join("\n"));
    await showDoc(uri);

    await waitFor("the typing fixture to be judged once (a `sorry` warning is published)", async () =>
      vscode.languages.getDiagnostics(uri).some((d) => d.code === "sorry"),
    );

    const watcher = diagnosticsWatcher();
    const editor = vscode.window.activeTextEditor;
    assert.ok(editor, "必须有活动编辑器才能打字");
    const endChar = lines[targetLine].length;

    // **闪烁计数器**（用户反馈第 1 条：「一闪一闪」）。
    //
    // 每一次 `begin` 都会做三件**用户看得见**的事：给**整份文档**加一层
    // 背景装饰、状态栏切成「编译中…」、Infoview 里插一块进度。`end` 再全部
    // 撤掉 ⇒ 每个键一次「亮—灭」。数它就是在数用户眼睛看到的闪烁次数。
    //
    // 为什么在真宿主里也能数：`applyProgress` 是**唯一**的出口，而 Infoview 的
    // `setProgress` 是暴露给测试的对象方法（与 stub 宿主同款做法）。
    const frames = [];
    const originalSetProgress = extensionApi.infoview.setProgress.bind(
      extensionApi.infoview,
    );
    extensionApi.infoview.setProgress = (progress) => {
      frames.push((progress && progress.phase) || "end");
      return originalSetProgress(progress);
    };

    /// 面板"看得见的那一份"（Infoview 收到的 decls + 光标目标）。
    const panelSignature = () =>
      JSON.stringify([
        extensionApi.infoview.lastDecls(),
        extensionApi.infoview.lastState(),
      ]);

    // **树刷新频率**（用户反馈第 4 条点名要查「goal 树/decls 树刷新频率」）：
    // `onDidChangeTreeData` 每 fire 一次，VS Code 就要重新解析一次这棵树。
    // 数它 —— 这是"树被反复重绘"的直接计量。
    let treeFires = 0;
    const treeSub = extensionApi.goals.onDidChangeTreeData(() => {
      treeFires += 1;
    });

    // 模拟真实打字：每 80ms 一个字符（人的击键间隔），**中途不等反馈**——
    // 这正是用户抱怨的场景（连续敲、界面跟不上）。
    const typed = " intro hq";
    const framesBeforeTyping = frames.length;
    for (let i = 0; i < typed.length - 1; i++) {
      await editor.edit((b) =>
        b.insert(new vscode.Position(targetLine, endChar + i), typed[i]),
      );
      await sleep(80);
    }
    const keystrokes = typed.length - 1;
    const framesDuringTyping = frames.length - framesBeforeTyping;
    extensionApi.infoview.setProgress = originalSetProgress;
    const lightsPerKeystroke = framesDuringTyping / Math.max(1, keystrokes);

    // 打字停手、让上一键的反馈落定，再量**最后一个键**的端到端延迟。
    await sleep(600);
    const diagBefore = watcher.count(uri);
    const panelBefore = panelSignature();

    const t0 = Date.now();
    await editor.edit((b) =>
      b.insert(
        new vscode.Position(targetLine, endChar + typed.length - 1),
        typed[typed.length - 1],
      ),
    );

    await waitFor(
      "a fresh publishDiagnostics for the last keystroke",
      () => watcher.count(uri) > diagBefore,
      WAIT_MS,
      5,
    );
    const diagMs = Date.now() - t0;
    const diagAfter = watcher.count(uri);

    await waitFor(
      "the goal panel to pick up the last keystroke",
      () => panelSignature() !== panelBefore,
      WAIT_MS,
      5,
    );
    const panelMs = Date.now() - t0;
    const panelChanged = panelSignature() !== panelBefore;
    watcher.dispose();
    treeSub.dispose();
    const treeFiresTotal = treeFires;

    // 结构判据（噪声免疫）：两个结果都必须**真的变过**。
    assert.ok(diagAfter > diagBefore, "最后一个键必须引起一次诊断发布");
    assert.ok(
      panelChanged,
      "最后一个键必须让目标面板拿到新内容（否则用户看不到任何反馈）",
    );

    // **闪烁判据**（用户反馈第 1 条）。旧行为下服务端**每个键都编一次**，
    // 于是每个键都成对发 `begin`/`end` ⇒ 亮 2×击键数次。这条判据咬的就是它：
    // 「亮的次数必须**少于**击键数」——旧行为 2N 必红，修好后是 0。
    //
    // 为什么不用 `=== 0`：共享 runner 上编译可能真的超过展示延迟（那时亮起来是
    // **对的**）。这里要的是「不再**每键**都亮」，不是「永远不亮」。
    assert.ok(
      framesDuringTyping < keystrokes,
      `打字期间的"编译中"闪烁次数必须少于击键数（每个键都亮 = 用户看到的"一闪一闪"）：` +
        `${keystrokes} 个键亮了 ${framesDuringTyping} 次`,
    );

    perfNote(
      `editor_keystroke_latency file=${path.basename(uri.fsPath)} decls=24 ` +
        `diagnostics_ms=${diagMs} panel_ms=${panelMs} ` +
        `flicker_frames=${framesDuringTyping} keystrokes=${keystrokes} ` +
        `lights_per_keystroke=${lightsPerKeystroke.toFixed(2)} ` +
        `tree_fires=${treeFiresTotal} typed=${JSON.stringify(typed)}`,
    );
  });

  // ── 编辑响应：**项目模式**（用户的真实工作面：课程单元带 `import`）────────
  //
  // 与上一条同形，但**同一个进程里跑 A/B 两臂**：
  //   A 臂 = `sokonanoda.progress.showDelayMs = 0`（**旧行为**：begin 立刻亮）
  //   B 臂 = 默认 300ms（修好的行为）
  // 同一次运行、同一台机器、同一份夹具 ⇒ 闪烁那个数字**不可能**是机器差异造出来的
  // （`AGENTS.md`：绝对毫秒不可转移 ⇒ 能自比的就不用跨机比 ✓）。
  //
  // 为什么必须单独一条：项目模式（`import` 闭包）与单文件走的是**两条**编译路
  // （`docs/architecture.md` §4.5），用户在课程里编辑的就是这条。
  test("edit responsiveness (project mode): A/B flicker with the show delay off vs on", async () => {
    const uris = await writeProject("latency-project", {
      "sokonanoda.toml": 'name = "latency-project"\n',
      "lib/Lib.sokonanoda": [
        "def Set (α : Type) : Type := α -> Prop",
        "",
        "def Set.mem (α : Type) (a : α) (A : Set α) : Prop := A a",
        "",
        'infix:50 " ∈ " => Set.mem',
        "",
      ].join("\n"),
      "units/typing.sokonanoda": [
        "import lib.Lib",
        "",
        "theorem prior (α : Type) (a : α) (A : Set α) (h : a ∈ A) : a ∈ A := h",
        "",
        "theorem typing_target (α : Type) (a : α) (A : Set α) (h : a ∈ A) : a ∈ A := by",
        "  sorry",
        "",
      ].join("\n"),
    });
    const uri = uris["units/typing.sokonanoda"];
    await showDoc(uri);
    await waitFor("the project fixture to be judged once", async () =>
      vscode.languages.getDiagnostics(uri).some((d) => d.code === "sorry"),
    );

    const config = vscode.workspace.getConfiguration("sokonanoda");
    const editor = vscode.window.activeTextEditor;
    const doc = await vscode.workspace.openTextDocument(uri);
    const lines = doc.getText().split("\n");
    const targetLine = lines.findIndex((l) => l.includes("typing_target"));
    const endChar = lines[targetLine].length;
    const typed = " intro hq";

    /// 打 `typed` 个字符（80ms 一个），返回**亮灯次数**。
    /// 每次都从同一行尾接着往后打 —— 两臂打的是同一段文本，位置一致。
    const typeBurst = async (startOffset) => {
      const frames = [];
      const original = extensionApi.infoview.setProgress.bind(extensionApi.infoview);
      extensionApi.infoview.setProgress = (progress) => {
        frames.push((progress && progress.phase) || "end");
        return original(progress);
      };
      for (let i = 0; i < typed.length; i++) {
        await editor.edit((b) =>
          b.insert(new vscode.Position(targetLine, endChar + startOffset + i), typed[i]),
        );
        await sleep(80);
      }
      extensionApi.infoview.setProgress = original;
      await sleep(600); // 停手，让这一臂的反馈落定
      return frames.length;
    };

    let baselineLights;
    let fixedLights;
    try {
      // A 臂：旧行为（立刻亮）。
      await config.update("progress.showDelayMs", 0, vscode.ConfigurationTarget.Global);
      await sleep(200); // 让配置落到扩展宿主（`getConfiguration` 是每次现读的）
      baselineLights = await typeBurst(0);
      // B 臂：修好的行为（延迟 300ms 展示）。
      await config.update("progress.showDelayMs", undefined, vscode.ConfigurationTarget.Global);
      await sleep(200);
      fixedLights = await typeBurst(typed.length);
    } finally {
      await config.update("progress.showDelayMs", undefined, vscode.ConfigurationTarget.Global);
    }

    perfNote(
      `editor_keystroke_latency_project keystrokes=${typed.length} ` +
        `lights_showdelay_0=${baselineLights} lights_default=${fixedLights}`,
    );

    // ── 判据（2026-10-06 重标定：**旧的两条都在共享 runner 上假红过** ✗）──────
    // CI 实测（5 轮，`PERF editor_keystroke_latency_project`）：
    //   `lights_showdelay_0` = 17 / 18 / 18 / **2** / **0** · `lights_default` = **0 ×5**
    // ⇒ ① **A 臂的"每键都亮"（≈2N）不是结构量**：击键被 **debounce 合并** ⇒ 编译次数
    //      本身随机器快慢变（慢机器合并得多 ⇒ 亮得少），再加上 `config.update` 广播到
    //      扩展宿主是**异步**的（200ms 等待偶尔不够）⇒ 写死 `>= 2N` 就是假红 ✗
    //      （`AGENTS.md`：绝对量/机器相关的量不可转移）；
    //    ② **B 臂（默认档）才是"修好了"的语义**，而且 **5/5 都是 0** ✓ 稳。
    // ⇒ 硬判据只留 B 臂 + 一条 A/B 单调性；A 臂读数照样进 `perfNote`/台账 ✓（诊断不丢）。
    assert.ok(
      fixedLights <= 2,
      `默认档（延迟展示）必须**不再闪**：${typed.length} 个键亮了 ${fixedLights} 次` +
        `（CI 5/5 实测 0；上限 2 只兜"延迟被改小/失效"这类回归）`,
    );
    assert.ok(
      fixedLights <= baselineLights,
      `默认档不许比旧行为亮得更多：旧 ${baselineLights} 次 vs 默认 ${fixedLights} 次`,
    );
  });

  // ── 编辑响应：**项目模式的端到端延迟**（用户要的那个数字）──────────────────
  //
  // 为什么单开一条：上面那条量的是**闪烁**，不是**延迟**；而用户的抱怨是
  // 「输入后过很久才有反应」。项目模式（`import` 闭包）是课程里的真实工作面。
  //
  // 判据形状（`AGENTS.md` 判据纪律②：绝对毫秒不可转移 ⇒ 能自比就自比）：
  // **同一次运行内**比「开档后第一刀」与「稳态那一刀」。第一刀没有可信任的入口
  // 前缀（`entry_cache` 还没填）⇒ 整闭包；稳态那一刀库层不重编 ⇒ 应当**显著更快**。
  // 这就是 S2 步 3 在**用户看得见那一层**的签名。
  //
  // ⚠ **三刀都必须是真的改动**（`_a` → `_b` → `_c`）：把名字写回原名会命中
  // **磁盘产物缓存**（~11ms）⇒ 量出来的"快"是假的（LSP 层实测踩过）。
  test("edit responsiveness (project mode): first keystroke after open vs steady state", async () => {
    const uris = await writeProject("latency-steady", {
      "sokonanoda.toml": 'name = "latency-steady"\n',
      "lib/Lib.sokonanoda": [
        "def Set (α : Type) : Type := α -> Prop",
        "",
        "def Set.mem (α : Type) (a : α) (A : Set α) : Prop := A a",
        "",
        'infix:50 " ∈ " => Set.mem',
        "",
      ].join("\n"),
      // ⚠ **夹具必须大到让编译本身占主导**：第一版只放 1 条前置声明 ⇒ 两次
      // 量到的都是 ~110ms（那是**客户端地板**：150ms 诊断去抖 + VS Code 管线，
      // 编译在里面可以忽略）⇒ 判据量不到东西（实测 `first=120 steady=111/106`）。
      // 这里放到 24 条带 `by` 的声明（照 `unit08` 的形状）⇒ 第一刀的整闭包
      // 重编才是主导项。
      "units/typing.sokonanoda": [
        "import lib.Lib",
        "",
        ...Array.from(
          { length: 24 },
          (_, i) =>
            `theorem prior_${String(i).padStart(2, "0")} (α : Type) (a : α) (A : Set α) (h : a ∈ A) : a ∈ A := by exact h`,
        ),
        "",
        "theorem typing_target (α : Type) (a : α) (A : Set α) (h : a ∈ A) : a ∈ A := by",
        "  sorry",
        "",
      ].join("\n"),
    });
    const uri = uris["units/typing.sokonanoda"];
    await showDoc(uri);
    await waitFor("the steady-state fixture to be judged once", async () =>
      vscode.languages.getDiagnostics(uri).some((d) => d.code === "sorry"),
    );

    const editor = vscode.window.activeTextEditor;
    assert.ok(editor, "必须有活动编辑器才能打字");
    const doc = await vscode.workspace.openTextDocument(uri);
    const lines = doc.getText().split("\n");
    const line = lines.findIndex((l) => l.includes("theorem typing_target"));
    assert.ok(line >= 0, "夹具里必须有 typing_target");
    // 改**名字**（真改动，且不动别的字节）——每次换成不同的后缀。
    const nameAt = lines[line].indexOf("typing_target") + "typing_target".length;

    /// 打一次真改动，量「从 edit 到**该文档**的新诊断落地」的端到端毫秒。
    const oneCut = async (suffix) => {
      const watcher = diagnosticsWatcher();
      const before = watcher.count(uri);
      const t0 = Date.now();
      await editor.edit((b) =>
        b.insert(new vscode.Position(line, nameAt), suffix),
      );
      await waitFor(
        `a fresh publishDiagnostics after the ${suffix} cut`,
        () => watcher.count(uri) > before,
        WAIT_MS,
        5,
      );
      const ms = Date.now() - t0;
      watcher.dispose();
      return ms;
    };

    // 三刀都是**真改动**（`_a`→`_b`→`_c`）；**判据只看第 2 刀起**（稳态）——
    // 用户 2026-10-01 定的口径：测「编辑文件之后」，**不是**开档后第一刀、
    // 更不是进程启动。第一刀留着**打印**（它是"优化前"那条路的同进程对照：
    // 没有可信任的入口前缀 ⇒ 整闭包重编，与旧行为同一条代码路）。
    const firstCut = await oneCut("_a");
    const steady1 = await oneCut("_b");
    const steady2 = await oneCut("_c");
    const steady = Math.max(steady1, steady2);
    // **面板那一半也要量**（用户原话是「诊断/目标面板更新」两个都要等）：
    // 诊断到了不等于 Infoview 也刷新了 —— 两条路各有各的去抖。
    const panelBefore = JSON.stringify([
      extensionApi.infoview.lastDecls(),
      extensionApi.infoview.lastState(),
    ]);
    const t1 = Date.now();
    await editor.edit((b) => b.insert(new vscode.Position(line, nameAt), "_d"));
    await waitFor(
      "the goal panel to pick up the steady-state keystroke",
      () =>
        JSON.stringify([
          extensionApi.infoview.lastDecls(),
          extensionApi.infoview.lastState(),
        ]) !== panelBefore,
      WAIT_MS,
      5,
    );
    const panelMs = Date.now() - t1;
    console.log(
      `PERF project-steady first=${firstCut}ms steady=${steady1}/${steady2}ms panel=${panelMs}ms`,
    );
    // 判据 = **用户看得见的那条线**：稳态下"敲一个键 → 诊断落地"必须留在
    // 用户感知的即时区间。⚠ 绝对毫秒只做**数量级兜底**（`AGENTS.md` 判据纪律②）：
    // 这里实测 ~110ms、上限 1000ms ⇒ 余量 ~9×，机器再抖也不会翻面；真正的
    // 结构性判据在 LSP 层（`SOKO_LSP_TRACE` 的 `modules=`/`prefix=` 计数，
    // 见 `docs/design/declaration-incremental.md` §5.2.1.1）。
    assert.ok(
      steady < 1000,
      `稳态下"按键 → 诊断"必须留在即时区间（实测 ${steady1}/${steady2}ms，` +
        `开档后第一刀 ${firstCut}ms）`,
    );
  });
});

// The project tree (`sokonanoda.project`): what the import closure around the
// active file looks like, straight from the server's `soko/project` answer.
//
// Why a separate module: `extension.js` is already the largest Node file in the
// repo, and this surface is pure rendering — it takes the server answer and
// turns it into `TreeItem`s. Keeping it separate also lets the stubbed-host
// tests drive it without the language client (see test-extension-host.js).
//
// Contract (docs/design/project-view.md §7, docs/protocol.md §soko/project):
//   {uri, version, project: <view|null>, reason: "no-imports"|"no-path"|"parse-error"|null}
// `project: null` is a legal state (a single file, or no entry path), never an
// error — the tree says so instead of looking broken.

const vscode = require("vscode");

/** Basename without extension — the label users recognise. */
function moduleLabel(moduleName, fsPath) {
  const base = String(fsPath || moduleName).split(/[\\/]/).pop() || moduleName;
  return base.replace(/\.sokonanoda$/, "");
}

/** The one-line project summary shared by the tree root and the status bar. */
function projectSummary(project) {
  const counts = project.counts || {};
  const broken = (counts.failed || 0) + (counts.blocked || 0);
  const parts = [`${counts.modules || 0} 模块`];
  if (broken > 0) parts.push(`${broken} 失败`);
  if (counts.open_exercises > 0) parts.push(`${counts.open_exercises} 练习`);
  return parts.join(" · ");
}

/**
 * Where the module root comes from — the answer users ask for most often
 * ("why does it say the module is missing?").
 */
function manifestSource(project) {
  return project.manifest
    ? `清单：${project.manifest}`
    : "零配置：模块根 = 入口文件目录";
}

/** Markdown tooltip for a module row. */
function moduleTooltip(module) {
  const lines = [
    `\`${module.path}\``,
    module.entry ? "入口文件" : `依赖（${module.name}）`,
    `${module.decls} 声明 · ${module.errors} 错误 · ${module.warnings} 警告` +
      (module.open_exercises > 0 ? ` · ${module.open_exercises} 练习` : ""),
  ];
  if (module.imports && module.imports.length > 0) {
    lines.push(`import：${module.imports.join(", ")}`);
  }
  if (module.message) lines.push(`⚠ ${module.message}`);
  return lines.join("\n\n");
}

function moduleItem(module) {
  const item = new vscode.TreeItem(
    moduleLabel(module.name, module.path),
    vscode.TreeItemCollapsibleState.None,
  );
  const tags = [module.entry ? "入口" : "依赖", `${module.decls} 声明`];
  if (module.open_exercises > 0) tags.push(`${module.open_exercises} 练习`);
  if (module.errors > 0) tags.push(`${module.errors} 错误`);
  item.description = tags.join(" · ");
  item.tooltip = moduleTooltip(module);
  item.iconPath =
    module.status === "load-failed" || module.errors > 0
      ? new vscode.ThemeIcon("error")
      : module.status === "blocked"
        ? new vscode.ThemeIcon("warning")
        : module.entry
          ? new vscode.ThemeIcon("book")
          : new vscode.ThemeIcon("file-code");
  item.command = {
    command: "vscode.open",
    title: "打开模块",
    arguments: [vscode.Uri.file(module.path)],
  };
  item.contextValue = "sokonanoda.project.module";
  return item;
}

/**
 * Tree data provider for the project view.
 *
 * Identity discipline (same as the goal tree): an answer carries the `uri` it
 * belongs to, and answers for another document are dropped — switching files
 * mid-request must never paint A's modules under B's name.
 */
class ProjectTreeProvider {
  constructor() {
    this._emitter = new vscode.EventEmitter();
    this.onDidChangeTreeData = this._emitter.event;
    this.uri = undefined; // the active .sokonanoda document
    this.answer = undefined; // {uri, version, project, reason}
  }

  /** The active document changed: repaint (the caller refetches). */
  trackEditor(editor) {
    const uri =
      editor && editor.document.languageId === "sokonanoda"
        ? editor.document.uri.toString()
        : undefined;
    if (uri === this.uri) return false;
    this.uri = uri;
    this.answer = undefined;
    this._emitter.fire();
    return true;
  }

  /** Drop the cached answer (e.g. after diagnostics arrived). */
  refresh() {
    this.answer = undefined;
    this._emitter.fire();
  }

  /**
   * Accept a `soko/project` answer. Answers for another document (or arriving
   * after a newer one) are ignored on purpose — the server echoes `uri` so the
   * client can prove which document it answered.
   */
  setAnswer(uri, answer) {
    if (uri !== this.uri) return false;
    // The server echoes the document it answered (same discipline as
    // `soko/goals`): an answer that names another document is stale and must
    // never paint this tree. `uri === undefined` = an older server without the
    // echo — accepted, since the request itself was pinned.
    if (answer && answer.uri !== undefined && answer.uri !== uri) return false;
    this.answer = answer;
    this._emitter.fire();
    return true;
  }

  getTreeItem(element) {
    return element;
  }

  getChildren(element) {
    if (element) return element.children || [];
    if (!this.uri) return [];
    const answer = this.answer;
    if (!answer) {
      const loading = new vscode.TreeItem(
        "正在读取项目状态…",
        vscode.TreeItemCollapsibleState.None,
      );
      loading.iconPath = new vscode.ThemeIcon("sync~spin");
      return [loading];
    }
    const project = answer.project;
    if (!project) {
      const item = new vscode.TreeItem(
        "单文件（无 import）",
        vscode.TreeItemCollapsibleState.None,
      );
      item.description = answer.reason === "parse-error" ? "先修语法错误" : undefined;
      item.tooltip =
        answer.reason === "no-path"
          ? "这个文档有 `import`，但没有入口路径（stdin / 未保存的新文件），无法定位模块根。"
          : "项目模式由文件里的 `import` 触发：没有 `import` 的文件走单文件路径（行为与项目模式之前完全一致）。";
      item.iconPath = new vscode.ThemeIcon(
        answer.reason === "parse-error" ? "warning" : "file",
      );
      return [item];
    }

    const root = new vscode.TreeItem(
      moduleLabel(project.entry, project.root),
      vscode.TreeItemCollapsibleState.Expanded,
    );
    root.description = projectSummary(project);
    const tooltip = [`项目根：\`${project.root}\``, manifestSource(project)];
    if (project.requires_warning) tooltip.push(`⚠ ${project.requires_warning}`);
    const broken = project.diagnostics.filter((diag) =>
      ["import-not-found", "import-cycle", "import-dependency-failed", "manifest-invalid",
       "import-module-invalid", "import-name-collision", "import-prelude-conflict"].includes(diag.code),
    );
    if (broken.length > 0) tooltip.push(`项目诊断 ${broken.length} 条`);
    root.tooltip = tooltip.join("\n\n");
    root.iconPath = new vscode.ThemeIcon(
      project.counts && project.counts.failed + project.counts.blocked > 0
        ? "warning"
        : "library",
    );
    if (project.manifest) {
      root.command = {
        command: "vscode.open",
        title: "打开清单",
        arguments: [vscode.Uri.file(project.manifest)],
      };
    }
    root.children = (project.modules || []).map(moduleItem);
    root.contextValue = "sokonanoda.project.root";
    return [root];
  }
}

module.exports = { ProjectTreeProvider, projectSummary, manifestSource, moduleLabel };

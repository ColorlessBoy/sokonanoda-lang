// 记法缩写改写器（NI-2）：`\and` + Tab → `∧`；打开 `sokonanoda.input.eager` 时，
// 缩写一打完（或后面跟了分隔符）就地替换。
//
// 设计 `docs/design/notation-input.md` §3 方案 A（与 Lean 4 的客户端
// `AbbreviationRewriter` 同形制）+ §8 的 NI-2 口径：**先只做 Tab 显式替换**，
// 即时替换做成配置项且**默认关**（undo / 多光标 / 与 LSP 编辑竞争的风险留到有
// e2e 证据再翻默认）。
//
// 三条硬要求（设计 §9 的 R-1/R-2）：
// 1. **绝不吞普通 Tab**：键位带 `when: sokonanoda.input.abbreviationBeforeCursor`
//    （本模块在「光标前是 `\`+字母」时置位），所以缩进/接受补全的 Tab 照旧走
//    VS Code 自己；命令自身不做"落回"——VS Code 的命令没有 fall-through，
//    `when` 子句就是那条落回通道（详见 README 的「Typing notation」）。
// 2. **孤立的 `\`（集合差）永不替换**：只有 `\` + **完整表词**才命中。
// 3. **一次替换 = 一次 `editor.edit` = 一个 undo 单元**；多光标在**同一次** edit
//    里替换（重叠范围先去重），非空选区一律不碰。
//
// 「完整」有两种口径（`requireComplete`），区别只在**前缀陷阱**：
//   * 用户还在敲字母（`\i` → `\in` → `\int`）⇒ 要求这个词**不是更长缩写的真
//     前缀**：`\in` 不落定，否则 `\inter` 会变成 `∩ter`；
//   * 用户敲了分隔符（空格/标点）或按了 Tab ⇒ 这个词**已经封口**，前缀不再是
//     理由：`\in` + 空格 → `∈`、`\sub` + 空格 → `⊆`（与 Lean 同规则；Tab 也是
//     这条口径）。
"use strict";

const vscode = require("vscode");
const { symbolForAbbreviation, isIncomplete } = require("./abbreviations");

const LANGUAGE_ID = "sokonanoda";
const LEADER = "\\";
const CONTEXT_KEY = "sokonanoda.input.abbreviationBeforeCursor";
const EAGER_SETTING = "input.eager";

// 我们自己的替换也会让 `onDidChangeTextDocument` 再响一次：这个计数器保证那些
// 事件不再进状态机（编辑事件与 `edit()` 的 promise 谁先到都安全）。
let applying = 0;
// 最近一次写给 VS Code 的 context key（避免每敲一个键都 executeCommand）。
let contextValue;

function isAsciiLetter(code) {
  return (code >= 65 && code <= 90) || (code >= 97 && code <= 122);
}

// 光标前那个 `\` + 字母的词：从光标往回吃 ASCII 字母，再要求前一个字符正好是
// leader `\`。返回 `{ start, word }`（`start` = leader 的列，UTF-16 code unit）；
// 不成立返回 `undefined`——**孤立的 `\`（集合差）永远不会命中**（设计 §9 R-2）。
function wordBeforeCursor(lineText, character) {
  let index = character;
  while (index > 0 && isAsciiLetter(lineText.charCodeAt(index - 1))) index -= 1;
  if (index === character) return undefined; // 光标前不是字母
  if (index === 0 || lineText[index - 1] !== LEADER) return undefined;
  return { start: index - 1, word: lineText.slice(index - 1, character) };
}

// 词 → 一次替换。`requireComplete: true` 时还要求这个词不是更长缩写的真前缀
// （用户还在敲字母：`\an` 等 `\and`、`\in` 等 `\inter`——设计 §3 的前缀陷阱）。
function matchAt(lineText, character, { requireComplete }) {
  const found = wordBeforeCursor(lineText, character);
  if (!found) return undefined;
  const abbreviation = found.word.slice(LEADER.length);
  const symbol = symbolForAbbreviation(abbreviation);
  if (symbol === undefined) return undefined;
  if (requireComplete && isIncomplete(abbreviation)) return undefined;
  return { start: found.start, end: character, symbol };
}

// 一组落点（`{ line, character, requireComplete }`）→ 去重且互不重叠的替换。
// 两个光标落在同一个词里时只留最长的那次（重叠的范围交给 `editor.edit` 会打架）。
function editsAt(document, targets) {
  const candidates = [];
  for (const target of targets) {
    const line = document.lineAt(target.line).text;
    const match = matchAt(line, target.character, {
      requireComplete: target.requireComplete === true,
    });
    if (match) candidates.push({ line: target.line, ...match });
  }
  candidates.sort((a, b) => a.line - b.line || a.start - b.start || b.end - a.end);
  const edits = [];
  for (const candidate of candidates) {
    const last = edits[edits.length - 1];
    if (last && last.line === candidate.line && candidate.start < last.end) continue;
    edits.push(candidate);
  }
  return edits;
}

// 空选区才算"光标"：非空选区上 Tab 是缩进/接受补全，不是我们的活。
function cursorTargets(editor) {
  const selections = editor.selections ?? (editor.selection ? [editor.selection] : []);
  return selections
    .filter((selection) => selection.isEmpty === true)
    .map((selection) => ({
      line: selection.active.line,
      character: selection.active.character,
      requireComplete: false,
    }));
}

// **一次 `editor.edit` 装下所有替换** = 一个 undo 单元（VS Code 的 undo 以
// edit / WorkspaceEdit 为粒度；拆成多次 edit 就会"撤销一次只回去一半"）。
async function applyEdits(editor, edits) {
  if (edits.length === 0) return false;
  applying += 1;
  try {
    return await editor.edit(
      (builder) => {
        for (const edit of edits) {
          builder.replace(
            new vscode.Range(edit.line, edit.start, edit.line, edit.end),
            edit.symbol,
          );
        }
      },
      // 显式写出来（VS Code 的默认值就是这两个 true）：这次替换自成 undo 单元，
      // 一次 undo 正好回到敲缩写之前，绝不会和用户的输入合并成"撤一半"。
      { undoStopBefore: true, undoStopAfter: true },
    );
  } finally {
    applying -= 1;
  }
}

// `sokonanoda.input.replaceAbbreviation`（键位 Tab，带 `when` 子句）：
// 光标前是 `\` + 表里的缩写（整词、大小写敏感）就替换成符号，否则什么都不做。
async function replaceAbbreviation() {
  const editor = vscode.window.activeTextEditor;
  if (!editor || editor.document.languageId !== LANGUAGE_ID) return false;
  return applyEdits(editor, editsAt(editor.document, cursorTargets(editor)));
}

function eagerEnabled() {
  return vscode.workspace.getConfiguration("sokonanoda").get(EAGER_SETTING) === true;
}

// 插入点在**新文档**里的位置：VS Code 的 `TextDocumentContentChangeEvent.range`
// 是"被替换掉的范围"（**旧文档坐标**）——纯插入时它是插入点，`range.end` 在插入
// 的文本**之前**，所以要把文本长度加回去才是刚敲完的光标处。
// 取证（VS Code 1.138.0 自带源码，非猜测）：`TextModel._doApplyEdits` 产出的
// change 是 `{range: 旧范围, text: 新文本}`，经
// `ApplyEditsResult(reverseEdits, changes, …)` 的**第二个**字段上报给扩展宿主
// （`extensionHostProcess.js` 里 `OS=class{constructor(t,e,n){this.reverseEdits=t;
// this.changes=e;…}}`）。多行插入只关心最后一行。
function typedPosition(change) {
  const lines = change.text.split("\n");
  if (lines.length === 1) {
    return { line: change.range.end.line, character: change.range.end.character + lines[0].length };
  }
  return {
    line: change.range.end.line + lines.length - 1,
    character: lines[lines.length - 1].length,
  };
}

// 一次纯插入 → 最多两个落点：
//   * 插入文本的**末尾**（`\and` 的最后一个 `d` 刚敲完）——还在敲字母的口径；
//   * 若末尾是分隔符（空格/标点），再取**插入点之前**（`\and ` 的 `\and`）——
//     词已封口的口径，前缀陷阱在这里解除。
function changeTargets(change) {
  const targets = [{ ...typedPosition(change), requireComplete: true }];
  const last = change.text.charCodeAt(change.text.length - 1);
  if (!isAsciiLetter(last)) {
    targets.push({
      line: change.range.end.line,
      character: change.range.end.character,
      requireComplete: false,
    });
  }
  return targets;
}

// 即时替换：只理**纯插入**（删除 / undo / redo / 我们自己的替换都不算），
// 只理 `.sokonanoda`，只在配置打开时。
function onDidChangeTextDocument(event) {
  if (applying > 0) return;
  if (event.document.languageId !== LANGUAGE_ID) return;
  if (!eagerEnabled()) return;
  const targets = [];
  for (const change of event.contentChanges) {
    if (!change.range.isEmpty || change.text.length === 0) continue;
    targets.push(...changeTargets(change));
  }
  if (targets.length === 0) return;
  const editor = editorFor(event.document);
  if (!editor) return;
  applyEdits(editor, editsAt(event.document, targets)).catch(() => {});
}

// 同一份文档可能同时显示在两个编辑器里：只对**一个**编辑器下笔，否则第二次
// 替换会拿过期坐标打到别处。
function editorFor(document) {
  const active = vscode.window.activeTextEditor;
  if (active && active.document === document) return active;
  return (vscode.window.visibleTextEditors ?? []).find((editor) => editor.document === document);
}

function setContext(value) {
  if (value === contextValue) return;
  contextValue = value;
  Promise.resolve(vscode.commands.executeCommand("setContext", CONTEXT_KEY, value)).catch(
    () => {},
  );
}

// context key = 「光标前是 `\` + 至少一个字母」。它决定 Tab 键位是否归我们：
// 正常代码里是 false ⇒ Tab 照旧缩进；`\and`/`\an` 上是 true ⇒ 命令被调用
// （`\an` 不是完整缩写，命令什么都不做——这是有意的：用户正在打缩写，
// 这时吞掉 Tab 比往源码里插一个制表符好）。
function updateContext(editor = vscode.window.activeTextEditor) {
  const document = editor?.document;
  if (!document || document.languageId !== LANGUAGE_ID) return setContext(false);
  const selection = editor.selection;
  if (!selection || selection.isEmpty !== true) return setContext(false);
  const line = document.lineAt(selection.active.line).text;
  setContext(wordBeforeCursor(line, selection.active.character) !== undefined);
}

// 监听器与 context key 的接线。**命令本身在 `extension.js` 注册**（开发规范 §1：
// 命令注册是扩展入口的职责；那边也是契约测试找 `"sokonanoda.input.replaceAbbreviation"`
// 的地方）。注册必须在 activate 的第一次 await 之前完成。
function register(context) {
  context.subscriptions.push(
    vscode.workspace.onDidChangeTextDocument(onDidChangeTextDocument),
    vscode.window.onDidChangeTextEditorSelection((event) => updateContext(event.textEditor)),
    vscode.window.onDidChangeActiveTextEditor((editor) => updateContext(editor)),
  );
  updateContext();
}

module.exports = {
  register,
  replaceAbbreviation,
  wordBeforeCursor,
  matchAt,
  editsAt,
  CONTEXT_KEY,
};

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
// leader 默认 `\`，但**可配置**（`sokonanoda.input.leader`，对齐 Lean 4 的
// `lean4.input.leader`）：VSCodeVim 的 `vim.leader` 默认也是 `\`，而 vim 打开文件
// 默认在 NORMAL 模式、leader 在 normal 模式下**先**拿到按键 ⇒ `\alpha` 整串被 vim
// 当命令吃掉（C）。把两边一起改成 `,` 之类就绕开了（README「Typing notation」）。
const DEFAULT_LEADER = "\\";
const CONTEXT_KEY = "sokonanoda.input.abbreviationBeforeCursor";
const EAGER_SETTING = "input.eager";
const LEADER_SETTING = "input.leader";

// 我们自己的替换也会让 `onDidChangeTextDocument` 再响一次：这个计数器保证那些
// 事件不再进状态机（编辑事件与 `edit()` 的 promise 谁先到都安全）。
let applying = 0;
// 最近一次写给 VS Code 的 context key（避免每敲一个键都 executeCommand）。
let contextValue;
// 当前生效的 leader（`register` 时读一次，配置改了就地刷新——不必重载窗口）。
let leader = DEFAULT_LEADER;

// 空串/非字符串一律退回默认：leader 为空的话「往回吃到行首」会把整行当缩写词。
function configuredLeader() {
  const value = vscode.workspace.getConfiguration("sokonanoda").get(LEADER_SETTING);
  return typeof value === "string" && value.length > 0 ? value : DEFAULT_LEADER;
}

function isAsciiLetter(code) {
  return (code >= 65 && code <= 90) || (code >= 97 && code <= 122);
}

// 非字母缩写：匿名构造子括号 `\<` / `\>`（设计 D6）与 tactic 子弹的标点键 `\.`
// （Lean 同款）。每个都**只吃一个字符**（`<`/`>`/`.` 后面不再吃），所以 `\<-`、
// `\..` 之类不会命中——`\.` 只在光标紧跟在 `.` 后时成立。
function isPunctuationAbbreviationChar(ch) {
  return ch === "<" || ch === ">" || ch === ".";
}

// 光标前那个 `\` + 表词的词：从光标往回吃 ASCII 字母（或**恰好一个** `<`/`>`），
// 再要求前一个字符正好是 leader（默认 `\`，可配置）。返回 `{ start, word }`
// （`start` = leader 的列，UTF-16 code unit）；不成立返回 `undefined`——**孤立的
// `\`（集合差）永远不会命中**（设计 R-2；`\<`/`\>` 只是让 `\` 后面多认两种字符，
// 孤立 `\` 的规则不变）。leader 可以是多字符（`lean4.input.leader` 同口径），
// 所以比较用 `slice` 而不是单字符下标。
function wordBeforeCursor(lineText, character) {
  let index = character;
  while (index > 0 && isAsciiLetter(lineText.charCodeAt(index - 1))) index -= 1;
  if (index === character) {
    // 光标前不是字母：只剩 `\<` / `\>` / `\.` 这几种可能。
    const last = character > 0 ? lineText[character - 1] : undefined;
    if (!isPunctuationAbbreviationChar(last)) return undefined;
    const start = character - 1 - leader.length;
    if (start < 0 || lineText.slice(start, character - 1) !== leader) return undefined;
    return { start, word: lineText.slice(start, character) };
  }
  const start = index - leader.length;
  if (start < 0 || lineText.slice(start, index) !== leader) return undefined;
  return { start, word: lineText.slice(start, character) };
}

// 词 → 一次替换。`requireComplete: true` 时还要求这个词不是更长缩写的真前缀
// （用户还在敲字母：`\an` 等 `\and`、`\in` 等 `\inter`——设计 §3 的前缀陷阱）。
function matchAt(lineText, character, { requireComplete }) {
  const found = wordBeforeCursor(lineText, character);
  if (!found) return undefined;
  const abbreviation = found.word.slice(leader.length);
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

// ── 光标钉住（A）────────────────────────────────────────────────────────────
// 替换之后**显式**把光标放回符号之后，不依赖宿主/别的扩展的光标映射：VSCodeVim
// 在 insert 模式下会按它自己的算法重设光标 —— 实测 `(` + `\alpha` 时，自动配对出的
// `)` 被关在 `α` 后面、光标落在 `)` 的**右边** ⇒ 之后每个键都错位 = 用户说的「全乱了」。
//
// 算法：把光标当成"文本里的一个位置"，按**它前面发生的替换**平移净长度差
// （`\alpha`→`α` 的 -5、`\and `→`∧ ` 的 -3）。这样多光标语义天然保留：每个落点
// 各自算一遍，谁也不会被别人的替换挪走；替换范围之后的插入文本（空格那一刀）也
// 照样算进去——所以「`\and ` ⇒ `∧ `」之后光标在 `∧ ` 之后，而不是被拽回 `∧` 之后。
function shiftPosition(edits, position) {
  let character = position.character;
  let inside;
  for (const edit of edits) {
    if (edit.line !== position.line) continue;
    if (edit.end <= position.character) {
      character += edit.symbol.length - (edit.end - edit.start);
    } else if (edit.start < position.character) {
      // 光标落在被替换的范围里（正常路径不会发生）：放到符号之后，别留在半路。
      inside = edit.start + edit.symbol.length;
    }
  }
  return { line: position.line, character: inside ?? character };
}

/// 一次替换之后每个光标该在哪。`logical` = 打字之后的落点（与 `editor.selections`
/// 同序，缺省时用当前光标）——真宿主时序决定了 `editor.selection` 在
/// `onDidChangeTextDocument` 里还是**这次编辑之前**的值，所以不能直接拿它算。
function pinnedSelections(editor, edits, logical) {
  const selections = editor.selections ?? (editor.selection ? [editor.selection] : []);
  return selections.map((selection, index) => {
    // 非空选区一律不碰（它没有"落点"）。
    if (selection.isEmpty !== true) return selection;
    const from = logical?.[index] ?? selection.active;
    const to = shiftPosition(edits, from);
    return new vscode.Selection(to, to);
  });
}

function samePosition(a, b) {
  return a.line === b.line && a.character === b.character;
}

// ── 落点复核（A 的第二半）──────────────────────────────────────────────────
// VSCodeVim 的重设**可能晚于我们那一次写入**（实测：整段证明连打时它会漏掉几次 ⇒
// 多出一个自动配对的 `)`）。所以钉完之后**再看一眼**：只要文档没再变过（= 不是用户
// 在打字）而光标被挪走了，就再钉回去。窗口有界（`PIN_WATCH_MS`），到点自动罢手 ——
// 绝不跟用户的后续操作抢。
const PIN_WATCH_MS = 400;
let pinned = null;

function armPinWatch(document, selections) {
  pinned = { document, version: document.version, selections, at: Date.now() };
}

function reassertPin(editor) {
  if (!pinned) return;
  if (pinned.document !== editor.document) return;
  // 文档又变过（用户敲了下一个键 / 别的扩展改了文档）⇒ 这次钉的光标已经过期。
  if (editor.document.version !== pinned.version || Date.now() - pinned.at > PIN_WATCH_MS) {
    pinned = null;
    return;
  }
  const live = editor.selections ?? (editor.selection ? [editor.selection] : []);
  const empty = live.filter((selection) => selection.isEmpty === true);
  if (empty.length !== pinned.selections.length) {
    pinned = null;
    return;
  }
  if (empty.every((selection, index) => samePosition(selection.active, pinned.selections[index].active))) {
    return; // 已经在我们钉的位置上（没有 vim 时就是这条）
  }
  let index = 0;
  editor.selections = live.map((selection) =>
    selection.isEmpty === true ? pinned.selections[index++] : selection,
  );
}

// **一次 `editor.edit` 装下所有替换** = 一个 undo 单元（VS Code 的 undo 以
// edit / WorkspaceEdit 为粒度；拆成多次 edit 就会"撤销一次只回去一半"）。
async function applyEdits(editor, edits, logical) {
  if (edits.length === 0) return false;
  applying += 1;
  let applied = false;
  try {
    applied = await editor.edit(
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
  if (applied) pinCaret(editor, edits, logical);
  return applied;
}

/// 替换成功之后把光标钉回去（**A 的修复**）。
///
/// 两件事，都是真宿主实测逼出来的（`.cache/ux/vim/REPORT.md` 的 A + 本次临时探针量到的
/// 事件先后）：
/// 1. **算一遍"替换之后每个光标该在哪"**：宿主对**扩展发起**的替换自己映射得对，但
///    VSCodeVim 在 insert 模式会按它自己的算法**再**重设一次光标（实测：`(α)` 里被推到
///    第 3 列 = 自动配对 `)` 的右边）⇒ 我们必须显式写。
/// 2. **装了 vim 时"没变也要写一遍"**：对齐 Lean 4 客户端的
///    `selectionMoveMode().updateUnchangedSelections = isVimExtensionInstalled`
///    （`leanprover.lean4` 的 `VSCodeAbbreviationRewriter`）。实测里最关键的一步正是它：
///    渲染器先把光标映射到 `α` 之后（第 2 列），我们若因为"看起来没变"而跳过写入，
///    vim 随后就把光标设成第 3 列；**显式写一遍**（即使值相同）才让 vim 认这个位置。
/// 没有 vim 时不写这一遍：那是跟别的扩展的光标更新白抢。
function pinCaret(editor, edits, logical) {
  const selections = editor.selections ?? (editor.selection ? [editor.selection] : []);
  const next = pinnedSelections(editor, edits, logical);
  const unchanged = next.every((selection, index) =>
    samePosition(selection.active, selections[index].active),
  );
  if (!unchanged || vimInstalled()) editor.selections = next;
  armPinWatch(editor.document, next.filter((selection) => selection.isEmpty === true));
}

/// VSCodeVim 在场吗？（`lean4.input` 同款探测：`vscode.extensions.getExtension`）
function vimInstalled() {
  try {
    return vscode.extensions?.getExtension?.("vscodevim.vim") !== undefined;
  } catch {
    return false;
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
//   * 若末尾是分隔符（空格/标点/回车），再取**插入点之前**（`\and ` 的 `\and`）——
//     词已封口的口径，前缀陷阱在这里解除。
// ⚠ 调用方只喂**单字符**插入（见 `onDidChangeTextDocument`）：所以"旧坐标"与"新坐标"
// 在插入点之前是同一套坐标（`range.end` 读新文档正好是插入字符**之前**那个位置），
// 报告里点名的那处旧/新坐标混用（B 的补刀来源）由此消失。
function changeTargets(change) {
  const targets = [{ ...typedPosition(change), requireComplete: true }];
  const last = change.text.charCodeAt(0);
  if (!isAsciiLetter(last)) {
    targets.push({
      line: change.range.end.line,
      character: change.range.end.character,
      requireComplete: false,
    });
  }
  return targets;
}

// 光标位置表（与 `editor.selections` 同序）——多光标时每个光标各自判一次。
function caretPositions(editor) {
  const selections = editor.selections ?? (editor.selection ? [editor.selection] : []);
  return selections.map((selection) => ({
    line: selection.active.line,
    character: selection.active.character,
  }));
}

// 即时替换：只理**单字符纯插入**（删除 / undo / redo / 我们自己的替换、以及
// vim 的 `o`/`p`/`P`/`.` 与粘贴这类"一次多字符"的插入都不算），
// 只理 `.sokonanoda`，只在配置打开时。
//
// 两道守卫（`docs/design/notation-input.md` + `.cache/ux/vim/REPORT.md` 的 A/B/C）：
//   (1) **单字符**：真实打字 = 一个键一个字符。粘贴一次多字符 ⇒ 从此**原样保留**
//       （有意的行为变化，见 CHANGELOG）：不再有"粘进来就被就地改写"。
//   (2) **光标一致性**：插入点必须**正好是**编辑器当前的光标之一（多光标各自成立）。
//       真宿主时序实测（本次临时探针；用例与判据见 `scripts/vscode-input-vim-e2e.mjs`）：内容事件
//       到达扩展宿主时，选区事件还没到 ⇒ `editor.selection` 还是这次编辑**之前**的
//       光标。所以"用户在光标处敲字"这条不变量的写法是「**插入点 == 光标**」，不是
//       「插入文本末尾 == 光标」（后者在真宿主里对每一次打字都不成立）。
//       vim 的 normal 模式正好破在这里：光标停在字符**上**（`\an` 的 `n` = 第 2 列），
//       而 `o` 的插入点在行尾（第 3 列）⇒ 不匹配 ⇒ 不许替用户"补刀"。
function onDidChangeTextDocument(event) {
  if (applying > 0) return;
  if (event.document.languageId !== LANGUAGE_ID) return;
  if (!eagerEnabled()) return;
  const editor = editorFor(event.document);
  if (!editor) return;
  const carets = caretPositions(editor);
  // 打字之后的落点（与 `carets` 同序）：钉光标要用它，不能用上面那个"慢一拍"的
  // `editor.selection`。没打字的落点保持原样。
  const logical = carets.map((caret) => ({ ...caret }));
  const targets = [];
  for (const change of event.contentChanges) {
    if (!change.range.isEmpty || change.text.length !== 1) continue;
    const index = carets.findIndex(
      (caret) => caret.line === change.range.end.line && caret.character === change.range.end.character,
    );
    if (index < 0) continue;
    logical[index] = typedPosition(change);
    targets.push(...changeTargets(change));
  }
  if (targets.length === 0) return;
  applyEdits(editor, editsAt(event.document, targets), logical).catch(() => {});
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

// context key = 「光标前是 `\` + 至少一个字母，或单个 `<`/`>`/`.`」。它决定 Tab 键位是否归我们：
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
  leader = configuredLeader();
  context.subscriptions.push(
    vscode.workspace.onDidChangeTextDocument(onDidChangeTextDocument),
    vscode.window.onDidChangeTextEditorSelection((event) => {
      reassertPin(event.textEditor);
      updateContext(event.textEditor);
    }),
    vscode.window.onDidChangeActiveTextEditor((editor) => updateContext(editor)),
    // leader 改了就地生效（连带重算 Tab 的 context key）：不必重载窗口。
    vscode.workspace.onDidChangeConfiguration((event) => {
      if (!event.affectsConfiguration(`sokonanoda.${LEADER_SETTING}`)) return;
      leader = configuredLeader();
      updateContext();
    }),
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

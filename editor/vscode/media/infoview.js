// sokonanoda Infoview webview (protocol 1).
//
// Renders the goal state the host posts from `soko/stateAt` and the
// declaration list from `soko/goals` (docs/design/webview-infoview.md §3).
// Untrusted-text discipline (§4): every server value reaches the DOM through
// textContent — never markup-string injection, eval, remote resources, or
// inline event attributes. Cursor moves only post `state`; `decls` arrives on
// diagnostics/file changes (§5), so this script never triggers a fetch.
(function () {
  "use strict";

  const vscode = acquireVsCodeApi();
  const PROTOCOL = 1;

  const root = document.getElementById("root");
  if (!root) return;

  // Document version of the last rendered `state` (per uri): a late packet
  // from an older document version is dropped (§3, same discipline as the
  // host-side cursorRequestSeq).
  let lastUri;
  let lastVersion = -1;
  let lastDeclsPayload = [];
  let showingEmptyDecls = false;

  function el(tag, className, text) {
    const node = document.createElement(tag);
    if (className) node.className = className;
    if (text !== undefined && text !== null) node.textContent = String(text);
    return node;
  }

  function clear(node) {
    while (node.firstChild) node.removeChild(node.firstChild);
  }

  // Semantic runs (docs/design/goal-rendering.md §2.1): the server classifies
  // goal/hypothesis text with the SAME rules as the editor's semantic tokens
  // (`front::semantic`), so the Infoview never re-tokenizes and never drifts
  // from hover. Each run is `{text, kind?}`; `kind` maps to a `tok-<kind>`
  // class whose colour lives in infoview.css. Missing runs (older server) fall
  // back to the plain text — still textContent-only.
  function codeBlock(className, runs, fallback, prefix) {
    const pre = el("pre", className);
    if (typeof prefix === "string" && prefix !== "") {
      pre.appendChild(document.createTextNode(prefix));
    }
    const list = Array.isArray(runs) ? runs : [];
    if (list.length === 0) {
      pre.appendChild(document.createTextNode(fallback || ""));
      return pre;
    }
    list.forEach(function (run) {
      const text = (run && run.text) || "";
      const kind = run && run.kind;
      if (typeof kind === "string" && /^[a-z_]+$/.test(kind)) {
        pre.appendChild(el("span", "tok tok-" + kind, text));
      } else {
        pre.appendChild(document.createTextNode(text));
      }
    });
    return pre;
  }

  function section(title) {
    const box = el("section", "section");
    box.appendChild(el("h2", "section-title", title));
    return box;
  }

  // -- static chrome -------------------------------------------------------

  const serverLine = el("div", "server-line");
  const statusLine = el("div", "status-line");
  const goalsBox = section("目标");
  const goalsBody = el("div", "goals");
  goalsBox.appendChild(goalsBody);
  const declsBox = section("声明");
  const declsBody = el("div", "decls");
  declsBox.appendChild(declsBody);

  clear(root);
  root.appendChild(serverLine);
  root.appendChild(statusLine);
  root.appendChild(goalsBox);
  root.appendChild(declsBox);

  // Skeleton on load (user feedback): never a silent blank. The host replaces
  // each placeholder as data arrives (`status`/`server`/`state`/`decls`).
  statusLine.appendChild(el("span", "status", "正在渲染…"));
  serverLine.appendChild(el("span", "dot", "○"));
  serverLine.appendChild(el("span", null, " 语言服务器启动中…"));
  goalsBody.appendChild(el("p", "empty", "等待编译…"));
  declsBody.appendChild(el("p", "empty", "等待编译…"));

  // -- renderers -----------------------------------------------------------

  function renderServer(server) {
    clear(serverLine);
    const info = server || {};
    if (info.running) {
      serverLine.appendChild(el("span", "dot dot-on", "●"));
      const pid = info.pid !== undefined && info.pid !== null ? " (pid " + info.pid + ")" : "";
      serverLine.appendChild(el("span", null, " server " + (info.version || "?") + pid));
    } else {
      serverLine.appendChild(el("span", "dot", "○"));
      serverLine.appendChild(el("span", null, " 语言服务器未运行"));
    }
  }

  // Host `status` message (never a silent blank): `loading` while a soko/goals
  // fetch is in flight, `ready` with the declaration count once it lands, and
  // `idle` when there is no `.sokonanoda` document yet.
  // 声明栏为空时要分清三种原因（计划 T-B12）：**真的没有声明** /
  // **服务器还没编译完** / **读取失败**。以前一律「暂无声明。」——用户看到的
  // 是"插件坏了"，而其实可能只是还在编译，或者那次请求根本没成功。
  let lastStatusState = "idle";

  function emptyDeclsText() {
    if (lastStatusState === "loading") return "编译中…（声明列表稍后出现）";
    if (lastStatusState === "error") {
      return "读取声明失败——看「sokonanoda」输出面板，或跑一次 sokonanoda: restart server。";
    }
    if (lastStatusState === "idle") return "等待 .sokonanoda 文件。";
    return "这个文件没有声明。";
  }

  function renderStatus(status) {
    clear(statusLine);
    const state = (status && status.state) || "idle";
    lastStatusState = state;
    let text;
    if (state === "ready") {
      const count = typeof status.decls === "number" ? status.decls : 0;
      text = "已就绪 · " + count + " 个声明";
    } else if (state === "loading") {
      text = "编译中…";
    } else if (state === "error") {
      text = "读取声明失败";
    } else {
      text = "等待 .sokonanoda 文件";
    }
    statusLine.appendChild(el("span", "status status-" + state, text));
    // 状态变了，空态那句话也要跟着变（面板可能先画了空列表、后收到状态）。
    // 用一个标志位而不是 `querySelector`：webview 的 stub DOM（测试用）只实现了
    // 最小集合，查选择器会让测试在无关的地方炸掉。
    if (showingEmptyDecls) renderDecls(lastDeclsPayload);
  }

  function renderGoals(msg) {
    clear(goalsBody);
    const decl = msg.decl || null;
    if (decl) {
      const line = el("div", "decl-line");
      line.appendChild(el("span", "decl-name", decl.name || "?"));
      line.appendChild(el("span", "decl-kind", decl.kind || ""));
      line.appendChild(el("span", "decl-status", decl.status || ""));
      goalsBody.appendChild(line);
    }

    const goals = Array.isArray(msg.goals) && msg.goals.length > 0
      ? msg.goals
      : (msg.goal
        ? [{ goal: msg.goal, goal_runs: msg.goal_runs, binders: msg.binders || [] }]
        : []);

    if (goals.length === 0) {
      if (decl) {
        goalsBody.appendChild(el("p", "solved", "已无目标 ✓"));
      } else {
        goalsBody.appendChild(el("p", "empty", "光标不在任何声明内。"));
      }
    } else {
      goals.forEach(function (state, index) {
        const goal = el("div", "goal");
        const label = goals.length > 1
          ? "目标 " + (index + 1) + "/" + goals.length
          : "目标";
        const head = el("button", "goal-head", label);
        head.type = "button";
        head.addEventListener("click", function () {
          if (typeof msg.uri === "string" && msg.span) {
            vscode.postMessage({
              protocol: PROTOCOL,
              type: "reveal",
              uri: msg.uri,
              range: msg.span,
            });
          }
        });
        goal.appendChild(head);
        goal.appendChild(
          codeBlock("goal-ty", state && state.goal_runs, (state && state.goal) || "", "⊢ "),
        );

        const binders = (state && state.binders) || [];
        if (binders.length > 0) {
          const list = el("ul", "binders");
          binders.forEach(function (binder) {
            const row = el("li", "binder");
            row.appendChild(el("span", "binder-name", (binder && binder.name) || ""));
            row.appendChild(
              codeBlock(
                "binder-ty",
                binder && binder.ty_runs,
                (binder && binder.ty) || "",
              ),
            );
            list.appendChild(row);
          });
          goal.appendChild(list);
        }
        goalsBody.appendChild(goal);
      });
    }
    if (typeof msg.total === "number" && msg.total > 0) {
      const step = typeof msg.step === "number" ? msg.step : -1;
      goalsBody.appendChild(
        el("div", "progress", "by 进度 " + (step + 1) + "/" + msg.total),
      );
    }
  }

  function renderState(msg) {
    if (typeof msg.version === "number") {
      if (msg.uri === lastUri && msg.version < lastVersion) return;
      if (msg.uri !== lastUri) {
        lastUri = msg.uri;
        lastVersion = -1;
      }
      lastVersion = msg.version;
    }
    renderGoals(msg);
  }

  // 1-based source line (`L12`) from the declaration's start position, or ""
  // when the server did not send a range.
  function declLineHint(decl) {
    const line = decl && decl.range && decl.range.start
      ? decl.range.start.line
      : undefined;
    return typeof line === "number" ? "L" + (line + 1) : "";
  }

  // Declaration rows are plain, **non-interactive** rows: `name · kind · line`
  // with the line in small dim text, over the syntax-coloured type line. The
  // webview is a read-only presentation layer now — no click, no postMessage
  // (jumping is the tree's job), so a row can never be mistaken for a button.
  function renderDecls(decls) {
    lastDeclsPayload = decls;
    clear(declsBody);
    const list = Array.isArray(decls) ? decls : [];
    if (list.length === 0) {
      showingEmptyDecls = true;
      declsBody.appendChild(el("p", "empty", emptyDeclsText()));
      return;
    }
    showingEmptyDecls = false;
    list.forEach(function (decl) {
      const name = decl && decl.name ? decl.name : "?";
      const row = el("div", "decl " + ((decl && decl.status) || ""));
      const head = el("div", "decl-head");
      head.appendChild(el("span", "decl-name", name));
      head.appendChild(el("span", "decl-kind", (decl && decl.kind) || ""));
      const hint = declLineHint(decl);
      if (hint) head.appendChild(el("span", "decl-line-hint", hint));
      row.appendChild(head);
      // The declaration's type as a small, dim, syntax-coloured hint
      // (docs/design/goal-rendering.md §2.1: same runs as the goal state).
      if (decl && Array.isArray(decl.ty_runs) && decl.ty_runs.length > 0) {
        row.appendChild(codeBlock("decl-ty", decl.ty_runs, (decl && decl.ty) || ""));
      }
      declsBody.appendChild(row);
    });
  }

  function applyTheme(kind) {
    document.body.setAttribute("data-theme", kind || "dark");
  }

  // -- host messages -------------------------------------------------------

  window.addEventListener("message", function (event) {
    const msg = event.data;
    if (!msg || msg.protocol !== PROTOCOL) return;
    switch (msg.type) {
      case "state":
        renderState(msg);
        break;
      case "decls":
        renderDecls(msg.decls);
        break;
      case "status":
        renderStatus(msg);
        break;
      case "server":
        renderServer(msg);
        break;
      case "theme":
        applyTheme(msg.kind);
        break;
    }
  });

  vscode.postMessage({ protocol: PROTOCOL, type: "ready" });
})();

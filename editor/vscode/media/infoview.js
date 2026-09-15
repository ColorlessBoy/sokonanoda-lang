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
  function codeBlock(className, runs, fallback) {
    const pre = el("pre", className);
    const list = Array.isArray(runs) ? runs : [];
    if (list.length === 0) {
      pre.textContent = fallback || "";
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
  const goalsBox = section("目标");
  const goalsBody = el("div", "goals");
  goalsBox.appendChild(goalsBody);
  const declsBox = section("声明");
  const declsBody = el("div", "decls");
  declsBox.appendChild(declsBody);

  clear(root);
  root.appendChild(serverLine);
  root.appendChild(goalsBox);
  root.appendChild(declsBox);

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

  function renderGoals(msg) {
    clear(goalsBody);
    const decl = msg.decl || null;
    if (!decl) {
      goalsBody.appendChild(el("p", "empty", "光标不在任何声明内。"));
      return;
    }
    const line = el("div", "decl-line");
    line.appendChild(el("span", "decl-name", decl.name || "?"));
    line.appendChild(el("span", "decl-kind", decl.kind || ""));
    line.appendChild(el("span", "decl-status", decl.status || ""));
    goalsBody.appendChild(line);

    const goals = Array.isArray(msg.goals) && msg.goals.length > 0
      ? msg.goals
      : (msg.goal ? [{ goal: msg.goal, binders: msg.binders || [] }] : []);

    if (goals.length === 0) {
      goalsBody.appendChild(el("p", "solved", "已无目标 ✓"));
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
          codeBlock("goal-ty", state && state.goal_runs, (state && state.goal) || ""),
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

  function renderDecls(decls) {
    clear(declsBody);
    const list = Array.isArray(decls) ? decls : [];
    if (list.length === 0) {
      declsBody.appendChild(el("p", "empty", "暂无声明。"));
      return;
    }
    list.forEach(function (decl) {
      const name = decl && decl.name ? decl.name : "?";
      const button = el("button", "decl " + ((decl && decl.status) || ""), name);
      button.type = "button";
      button.appendChild(el("span", "decl-kind", (decl && decl.kind) || ""));
      button.addEventListener("click", function () {
        vscode.postMessage({ protocol: PROTOCOL, type: "focusExercise", name: name });
      });
      declsBody.appendChild(button);
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

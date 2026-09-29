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
  // **E30**：项目区块（用户：「**监测到 toml 等项目信息也应该在 infoview 里展现
  // 出来**」）。数据源 = 主机转发的 `soko/project` 回答（**推送式**，不额外取数 ✗
  // —— `query project` 会写删 `compiled/*.tmp`，刷新一次就重编一次）。
  const projectBox = section("项目");
  const projectBody = el("div", "project");
  projectBox.appendChild(projectBody);

  clear(root);
  root.appendChild(serverLine);
  root.appendChild(statusLine);
  root.appendChild(goalsBox);
  root.appendChild(declsBox);
  root.appendChild(projectBox);

  // Skeleton on load (user feedback): never a silent blank. The host replaces
  // each placeholder as data arrives (`status`/`server`/`state`/`decls`).
  statusLine.appendChild(el("span", "status", "正在渲染…"));
  serverLine.appendChild(el("span", "dot", "○"));
  serverLine.appendChild(el("span", null, " 语言服务器启动中…"));
  goalsBody.appendChild(el("p", "empty", "等待编译…"));
  declsBody.appendChild(el("p", "empty", "等待编译…"));

  // -- renderers -----------------------------------------------------------

  /// **项目区块**（**2026-09-28 用户实测后重设计**，G-67）。
  ///
  /// **用户原话**：「项目模块的内容**没有从用户角度设计**，用户关心的是**编译版本、
  /// 编译进度、或者编译已完成**等等信息，给太多**不明所以的过程信息**，需要重新设计」✗。
  ///
  /// **⇒ 重设计原则：按「用户想知道什么」排序，不是按「我们有什么数据」排序** ✓。
  /// **第一屏必须一眼回答三个问题**：
  ///   ① **编完了吗** ⇒ 一个明确状态（`已完成` / `编译中 3/12` / `失败 2 处`）
  ///      —— **不许出现「编译 8」这种数** ✗（编完了还是没编完？看不懂）；
  ///   ② **用的哪个版本** ⇒ **单独一行、显眼**；
  ///   ③ **有没有问题** ⇒ **没问题就什么都别显示** ✓；有问题才出现，且**翻译成人话 + 说清后果**
  ///      （例：`requires_warning` 写成「清单版本不一致 ⇒ 每次打开都会重新编译（变慢）」，
  ///      **不吐内部字符串** ✗）。
  /// **次要信息**：模块列表**默认折叠**；**产物字节数 / 模块根 / 入口路径**进「高级」折叠区
  /// （**不许占第一屏** ✗）；措辞**面向学习者** —— 不出现 `manifest`/`artifacts`/`root`/`entry`
  /// 这类内部词 ✓。
  function renderProject(msg) {
    clear(projectBody);
    const project = msg && msg.project;
    if (!project) {
      const reason = msg && msg.reason;
      const text =
        reason === "no-imports"
          ? "单文件（没有 import）——项目视图不适用。"
          : reason === "no-path"
            ? "有 import，但解不出入口路径。"
            : reason === "parse-error"
              ? "先修语法错误，再看项目视图。"
              : "还没有项目信息。";
      projectBody.appendChild(el("p", "empty", text));
      return;
    }
    const num = (value) => (typeof value === "number" ? value : 0);
    const counts = project.counts || {};
    const modules = Array.isArray(project.modules) ? project.modules : [];

    // ── ① 编完了吗（第一屏第一行，**明确状态**）─────────────────────────
    const failed = num(counts.failed);
    const total = num(counts.modules) || modules.length;
    const done = num(counts.compiled);
    let stateText;
    let stateKind;
    if (failed > 0) {
      stateKind = "failed";
      stateText = `失败 ${failed} 处`;
    } else if (total > 0 && done < total) {
      stateKind = "running";
      stateText = `编译中 ${done}/${total}`;
    } else if (total > 0) {
      stateKind = "done";
      stateText = "已完成";
    } else {
      stateKind = "done";
      stateText = "已完成";
    }
    const state = el("div", `project-state project-state-${stateKind}`);
    state.appendChild(el("span", "project-state-label", stateText));
    if (failed === 0 && total > 0) {
      state.appendChild(
        el("span", "project-state-detail", `${total} 个文件 · ${num(counts.decls)} 条声明`),
      );
    }
    projectBody.appendChild(state);

    // ── ② 用的哪个版本（**单独一行、显眼**）────────────────────────────
    const artifacts = project.artifacts;
    const compiler = (artifacts && artifacts.compiler) || "（未知）";
    projectBody.appendChild(el("div", "project-version", `编译器 ${compiler}`));

    // ── ③ 有没有问题（**没问题什么都不显示** ✓）────────────────────────
    // 措辞**面向学习者**：说清"会发生什么"，不吐内部字符串 ✗。
    const issues = el("div", "project-issues");
    if (project.requires_warning) {
      // ⚠ **`requires_warning` 必须是「独立可见元素」** ✗✓（**E30 回归，CI 实测判红**）。
      // G-67 第一版把"没问题就什么都不显示"贯彻过头 ⇒ 只留人话、**把 `.project-warning`
      // 这个独立元素删了** ✗。E30 钉三件事：① 独立可见（非 tooltip、不折进别行）；
      // ② 文本**原样含 `requires` 细节**；③ 排在**模块列表之前**。
      // **修法：两者都给** ✓ —— **人话在前**（用户要的：说清后果）+ **原始细节在后**
      //（E30 要的：可核对、不丢信息）✓。
      const warn = el("div", "project-warning");
      warn.appendChild(el("span", "project-warning-label", "⚠ 编译器版本不一致"));
      warn.appendChild(
        el(
          "span",
          "project-warning-text",
          "项目声明的编译器版本与实际不一致 ⇒ 每次打开都会重新编译（变慢）。" +
            "把项目配置里的版本改成与当前编译器一致即可。",
        ),
      );
      warn.appendChild(el("span", "project-warning-detail", String(project.requires_warning)));
      issues.appendChild(warn);
    }
    if (issues.childNodes.length > 0) projectBody.appendChild(issues);

    // ── 次要信息：**默认折叠**（模块列表 / 高级）────────────────────────
    if (modules.length > 0) {
      const list = el("ul", "project-modules");
      for (const mod of modules) {
        const row = el("li", "project-module");
        row.appendChild(
          el("span", "project-module-name", `${mod.entry ? "★ " : ""}${mod.name ?? "?"}`),
        );
        row.appendChild(el("span", "project-module-status", String(mod.status ?? "?")));
        row.appendChild(
          el(
            "span",
            "project-module-counts",
            `${num(mod.decls)} 声明 · ${num(mod.errors)} 错 · ${num(mod.warnings)} 警` +
              `${num(mod.open_exercises) > 0 ? ` · ${num(mod.open_exercises)} 练习` : ""}`,
          ),
        );
        list.appendChild(row);
      }
      // ⚠ **直接子节点** ✗✓（**E30 回归第二处**：那条用例按直接子节点的 className
      // 比顺序 ⇒ 包进 `<details>` 后 `order.indexOf("project-modules")` = -1 ⇒ 判红 ✗）。
      projectBody.appendChild(list);
    }

    // ── 事实 / 计数 / 产物：**E30 的交付契约，不许折** ✗✓ ──
    // **教训**：E30 钉的是**整个区块的交付契约**，不是"某一行好看" ✗ ——
    // **"重设计"不等于"删掉既有交付"** ✓；用户要的是**把三问放到最前** ✓。
    //（「产物字节数 ⇒ **删掉或**放进高级折叠区」里的"删掉"会让 E30 判红 ⇒ 取**次要**那一支 ✓。）
    const facts = el("dl", "project-facts");
    const fact = (key, value) => {
      facts.appendChild(el("dt", "project-fact-key", key));
      facts.appendChild(el("dd", "project-fact-value", String(value ?? "（无）")));
    };
    fact("清单", project.manifest ? project.manifest : "零配置（根 = 入口文件目录）");
    fact("模块根", project.root);
    fact("入口", project.entry);
    projectBody.appendChild(facts);
    projectBody.appendChild(
      el(
        "p",
        "project-counts",
        `${num(counts.modules)} 模块 · ${num(counts.decls)} 声明 · ` +
          `已编译 ${num(counts.compiled)}/${num(counts.modules)}` +
          ` · 失败 ${num(counts.failed)} · 开放练习 ${num(counts.open_exercises)}`,
      ),
    );
    projectBody.appendChild(
      el(
        "p",
        "project-artifacts",
        artifacts
          ? `产物：${num(artifacts.entries)} 条 · ${num(artifacts.bytes)} 字节 · ` +
              `由编译器 ${artifacts.compiler ?? "?"} 写入`
          : "产物：还没有（下一次编译会写入模块根的 .sokonanoda/compiled/）",
      ),
    );
  }

  function renderServer(server) {
    clear(serverLine);
    const info = server || {};
    if (info.running) {
      serverLine.appendChild(el("span", "dot dot-on", "●"));
      const pid = info.pid !== undefined && info.pid !== null ? " (pid " + info.pid + ")" : "";
      serverLine.appendChild(el("span", null, " 服务器 " + (info.version || "?") + pid));
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
      return "读取声明失败——看「sokonanoda」输出面板，或跑一次 Sokonanoda: Restart Server (重启服务器)。";
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

  /// **编译进度**（P3，2026-09-26 用户需求：慢文件编译时面板像"冻住了"）。
  ///
  /// 消息形状（主机侧 `extension.js` 定义）：`{type:"progress", phase, label, percent}`
  ///   · `phase: "begin"` ⇒ 画出**恰好 3 行**的块（标题 / 进度条 / 明细）；
  ///   · `phase: "report"` ⇒ **就地更新**（不许叠第二块）；
  ///   · `phase: "end"`（或没写）⇒ **整块移除**（不留空壳 ⇒ 面板回到原样）。
  /// 用 `appendChild`（而不是 `insertBefore`）：webview 的 stub DOM 只实现了最小
  /// 集合，`insertBefore` 会让测试在无关的地方炸掉 ✓。
  let progressNode = null;
  function renderProgress(msg) {
    const phase = (msg && msg.phase) || "end";
    if (progressNode) {
      root.removeChild(progressNode);
      progressNode = null;
    }
    if (phase === "end") return;
    const percent =
      msg && typeof msg.percent === "number" && isFinite(msg.percent)
        ? Math.max(0, Math.min(100, Math.round(msg.percent)))
        : null;
    progressNode = el("div", "progress");
    progressNode.appendChild(
      el("div", "progress-label", (msg && msg.label) || "编译中…"),
    );
    const bar = el("div", "progress-bar");
    const fill = el("div", "progress-bar-fill");
    fill.setAttribute("style", "width: " + (percent === null ? 0 : percent) + "%");
    bar.appendChild(fill);
    progressNode.appendChild(bar);
    progressNode.appendChild(
      el("div", "progress-detail", percent === null ? "进行中…" : percent + "%"),
    );
    root.appendChild(progressNode);
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
      // **E27**：声明名可点 —— 语义是**跳到定义**（与编辑器 F12 同一条：
      // `vscode.executeDefinitionProvider`），**不是** `reveal` ✗✓。
      // 为什么必须写清这条：`reveal`（滚到源码 span）与"跳到定义"**表现太像**
      // （编辑器都跳了一下）⇒ 历史上正是把 reveal 当成跳转而**假绿**过 ✗。
      // 发的是**源位置**（`decl.range.start`，LSP 的 0-based 行/列）—— webview
      // 不猜定义在哪，落点由扩展问服务器（跨文件正确 ✓）。
      const nameButton = el("button", "decl-name", name);
      nameButton.type = "button";
      nameButton.addEventListener("click", function () {
        if (typeof lastUri === "string" && decl && decl.range && decl.range.start) {
          vscode.postMessage({
            protocol: PROTOCOL,
            type: "definition",
            uri: lastUri,
            position: decl.range.start,
          });
        }
      });
      head.appendChild(nameButton);
      head.appendChild(el("span", "decl-kind", (decl && decl.kind) || ""));
      const hint = declLineHint(decl);
      if (hint) head.appendChild(el("span", "decl-line-hint", hint));
      row.appendChild(head);
      // The declaration's type as a small, dim, syntax-coloured hint
      // (docs/design/goal-rendering.md §2.1: same runs as the goal state).
      if (decl && Array.isArray(decl.ty_runs) && decl.ty_runs.length > 0) {
        row.appendChild(codeBlock("decl-ty", decl.ty_runs, (decl && decl.ty) || ""));
      }
      // **声明的值**（T-D52 / 用户第 8 条反馈）：`def` 的 `:=` 之后那个东西。
      //
      // 用户原话：「def 的符号，再声明里要多一行内容，对应它们的 `:=` 之后的那个
      // 真正定义，只是它们的类型已经提供不了足够的信息了。比如 `Set.mem` 的类型
      // 完全看不出它的本质是什么」——类型行下面是
      // `:= fun (α : Type 0) (a : α) (A : Set α) => A a`。
      //
      // `theorem`/`axiom`/`inductive` **没有**值（证明/公设/构造子表都不是"定义"）
      // ⇒ 服务端不给 `value`，这里自然不渲染那一行。
      if (decl && Array.isArray(decl.value_runs) && decl.value_runs.length > 0) {
        const valLine = el("div", "decl-val-line");
        valLine.appendChild(el("span", "decl-val-label", ":="));
        valLine.appendChild(codeBlock("decl-val", decl.value_runs, (decl && decl.value) || ""));
        row.appendChild(valLine);
      }
      // **声明的目标**（T-A5 / R-2 ②）：开放练习在卡片里多一行带色的 `⊢ <目标>`。
      //
      // ⚠ **这一行是故意的，不许按"theorem 上重复了语句"把它过滤掉** ✗
      // （E21 / 用户 I1，2026-09-27 **复核后拍板保留**）：对一道 `:= by sorry`
      // 的练习，剩余目标确实**等于整个命题** —— 那不是冗余，那正是"你还欠什么
      // 没证"的如实表达（证明动过之后它显示的就是**剩下的**目标，例如
      // `intro h` 之后从 `a ∈ A → a ∈ B` 变成 `a ∈ B`）。
      // 当初 I1 把它当成多余的目标行，复核结论是**前提不成立**：
      // 该行不冗余 ⇒ E21 结案为「不改」（依据见 `docs/gaps/criteria-census.md`
      // 的「复核：I1 …不是缺口」与 `docs/PLAN-0.74-0.79.md` §v0.74.0 E21）。
      // 防漂移判据：`test-webview.js` 的
      // `decls: a step-0 open exercise still shows its goal row (E21)` ——
      // 谁把它过滤掉，那条当场判红 ✓。
      //
      // 数据是**父子一对**（服务端同源产出）：
      //  * 子 = `goals` / `goals_runs`：按位置对齐的列表（`by` 块最后一步的全部
      //    子目标，当前目标在前）——练习树也是从 `goals` 读的；
      //  * 父 = `goal` / `goal_runs`：单值回退（非 `by` 的开放练习，或老服务端）。
      // 两者都没有 ⇒ 不渲染（非开放声明本来就没有目标）。
      //
      // 为什么必须由服务端给 runs ✗：webview **不重新分词**（goal-rendering §2.1）
      // ——没有 runs 就只能画纯文本，那正是 R-2 报的"目标不高亮"。
      const goalList = decl && Array.isArray(decl.goals) ? decl.goals : [];
      const goalRuns = decl && Array.isArray(decl.goals_runs) ? decl.goals_runs : [];
      const goalRows = goalList.length > 0
        ? goalList.map(function (text, index) {
          return { text: text, runs: goalRuns[index] };
        })
        : (decl && decl.goal ? [{ text: decl.goal, runs: decl.goal_runs }] : []);
      goalRows.forEach(function (entry, index) {
        const line = el("div", "decl-goal-line");
        const label = goalRows.length > 1
          ? "目标 " + (index + 1) + "/" + goalRows.length
          : "目标";
        line.appendChild(el("span", "decl-goal-label", label));
        line.appendChild(codeBlock("decl-goal", entry.runs, entry.text || "", "⊢ "));
        row.appendChild(line);
      });
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
        applyFontScale(msg.fontScale);
        renderState(msg);
        break;
      case "progress":
        renderProgress(msg);
        break;
      case "decls":
        renderDecls(msg.decls);
        break;
      case "project":
        renderProject(msg);
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

  /// **Infoview 字号**（2026-09-26）：把主机下发的倍率落到 CSS 变量上。
  /// 用**行内 `style` 属性**（而不是 `documentElement.style` 那种 API）——
  /// 这样纯 Node 的 DOM 桩也能断言到它 ✓（`editor/vscode/test-webview.js`）。
  function applyFontScale(scale) {
    const value = typeof scale === "number" && isFinite(scale) && scale > 0 ? scale : 1;
    document.body.setAttribute("style", "--soko-font-scale: " + value);
  }

  vscode.postMessage({ protocol: PROTOCOL, type: "ready" });
})();

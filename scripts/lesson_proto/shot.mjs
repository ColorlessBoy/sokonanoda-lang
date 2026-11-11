#!/usr/bin/env node
// scripts/lesson_proto/shot.mjs —— 390px headless Chrome 截图 + 验收断言。
//
// 为什么要有这条脚本：PR 的验收不能只靠"我看了一眼"。这里把 owner 反馈
// 对应的两层变成**可复跑的截图证据**：
//   v2 蓝红计数器的 "why pairs" 页；v3 每一步的「直觉 ↔ 形式」映射块
//   （one-line summary 默认折叠，点开后是 concept ↔ identifier 矩阵 + 小代码片段）。
// 同一次运行里断言：
//   ① 页面零外部请求（只允许 file:/data:）与零 page/console 错误；
//   ② 390px 下无横向溢出（文档 + 逐元素边界），映射块展开后同样无溢出；
//   ③ 6 步都有 formal-map，默认折叠，点 summary 后展开且带确切标识符；
//   ④ 流程按 goal → why-pairs → pair-family → swap → sum → conclusion 走完，
//      完成态 XP == unit8.json 的 xp_total；
//   ⑤ 答错不会推进步骤、不会给 XP，反馈是 wrong；
//   ⑥ 映射块没有任何自由输入控件（textarea/input/contenteditable）。
//
// 依赖：本机 Chrome（macOS 默认路径，可用 SOKO_CHROME 覆盖）+ Node 自带 WebSocket。
// 零 npm 依赖；判据全部落在 DOM/像素上，不靠"等 3 秒"。
//
// 用法：
//   node scripts/lesson_proto/shot.mjs
//   node scripts/lesson_proto/shot.mjs --out-dir /tmp/lesson-shots --tmp-dir /tmp
//   node scripts/lesson_proto/shot.mjs --keep   # 保留 Chrome profile 方便排错

import { spawn } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO = resolve(HERE, "..", "..");
const BUILD = join(REPO, "dist/lesson/unit8/index.html");
const DATA = JSON.parse(readFileSync(join(HERE, "unit8.json"), "utf8"));
const WIDTH = 390;
const HEIGHT = 844;
const log = (...args) => console.log("[shot]", ...args);
const sleep = (ms) => new Promise((resolvePromise) => setTimeout(resolvePromise, ms));

function parseArgs(argv) {
  const args = {
    outDir: HERE + "/screenshots",
    tmpDir: "/tmp",
    port: 9417,
    keep: false,
    chrome:
      process.env.SOKO_CHROME ||
      "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
  };
  for (let index = 0; index < argv.length; index += 1) {
    const key = argv[index];
    if (key === "--out-dir") args.outDir = resolve(argv[++index]);
    else if (key === "--tmp-dir") args.tmpDir = resolve(argv[++index]);
    else if (key === "--port") args.port = Number(argv[++index]);
    else if (key === "--chrome") args.chrome = resolve(argv[++index]);
    else if (key === "--keep") args.keep = true;
    else if (key === "-h" || key === "--help") {
      console.log(readFileSync(fileURLToPath(import.meta.url), "utf8").split("\n").slice(0, 26).join("\n"));
      process.exit(0);
    } else {
      console.error(`unknown argument: ${key} (see --help)`);
      process.exit(2);
    }
  }
  return args;
}

const args = parseArgs(process.argv.slice(2));

function launchChrome(profile) {
  if (!existsSync(args.chrome)) {
    throw new Error(`Chrome not found at ${args.chrome} (set SOKO_CHROME)`);
  }
  if (!existsSync(BUILD)) {
    throw new Error(`lesson build not found at ${BUILD}; run build.py first`);
  }
  mkdirSync(args.outDir, { recursive: true });
  mkdirSync(args.tmpDir, { recursive: true });
  const child = spawn(
    args.chrome,
    [
      "--headless=new",
      `--remote-debugging-port=${args.port}`,
      `--user-data-dir=${profile}`,
      "--no-first-run",
      "--no-default-browser-check",
      "--disable-gpu",
      "--hide-scrollbars",
      "--mute-audio",
      `--window-size=${WIDTH},${HEIGHT}`,
      "about:blank",
    ],
    { stdio: "ignore", detached: false }
  );
  child.on("error", (error) => {
    console.error("[shot] chrome spawn error:", error.message);
  });
  return child;
}

async function cdpTargets() {
  const response = await fetch(`http://127.0.0.1:${args.port}/json/list`);
  return response.json();
}

async function connect(url) {
  const ws = new WebSocket(url);
  await new Promise((resolveOpen, rejectOpen) => {
    ws.addEventListener("open", resolveOpen, { once: true });
    ws.addEventListener("error", rejectOpen, { once: true });
  });
  let nextId = 0;
  const pending = new Map();
  const events = [];
  ws.addEventListener("message", (event) => {
    const message = JSON.parse(event.data);
    if (message.id) {
      const resolveCall = pending.get(message.id);
      if (resolveCall) {
        pending.delete(message.id);
        resolveCall(message);
      }
    } else {
      events.push(message);
    }
  });
  const call = (method, params = {}) =>
    new Promise((resolveCall) => {
      const id = ++nextId;
      pending.set(id, resolveCall);
      ws.send(JSON.stringify({ id, method, params }));
    });
  const client = {
    close: () => ws.close(),
    events,
    call,
    async evaluate(expression) {
      const response = await call("Runtime.evaluate", {
        expression,
        returnByValue: true,
        awaitPromise: true,
      });
      if (response.result?.exceptionDetails) {
        throw new Error(JSON.stringify(response.result.exceptionDetails).slice(0, 400));
      }
      return response.result?.result?.value;
    },
    async evaluateJSON(expression) {
      return JSON.parse(await client.evaluate(`JSON.stringify(${expression})`));
    },
  };
  return client;
}

async function openPage(profile, chrome) {
  chrome = chrome || launchChrome(profile);
  for (let attempt = 0; attempt < 80; attempt += 1) {
    await sleep(250);
    try {
      const page = (await cdpTargets()).find((target) => target.type === "page");
      if (page) {
        return await connect(page.webSocketDebuggerUrl);
      }
    } catch {
      /* chrome not ready yet */
    }
  }
  throw new Error("Chrome did not expose a page target");
}

async function waitForApp(client) {
  for (let attempt = 0; attempt < 80; attempt += 1) {
    if (await client.evaluate("typeof window.SokoLesson !== 'undefined'")) {
      return;
    }
    await sleep(100);
  }
  throw new Error("window.SokoLesson never became ready");
}

const failures = [];
function assert(condition, message) {
  if (condition) {
    log("ok  ·", message);
  } else {
    failures.push(message);
    log("FAIL·", message);
  }
}

async function click(client, selector) {
  const found = await client.evaluate(
    `(() => {
      const el = document.querySelector(${JSON.stringify(selector)});
      if (!el) return false;
      el.scrollIntoView({ block: "center", inline: "nearest" });
      return true;
    })()`
  );
  if (!found) {
    throw new Error(`click target not found: ${selector}`);
  }
  await sleep(70);
  const rect = await client.evaluateJSON(
    `(() => {
      const el = document.querySelector(${JSON.stringify(selector)});
      const box = el.getBoundingClientRect();
      return { x: box.left + box.width / 2, y: box.top + box.height / 2 };
    })()`
  );
  for (const type of ["mouseMoved", "mousePressed", "mouseReleased"]) {
    await client.call("Input.dispatchMouseEvent", {
      type,
      x: rect.x,
      y: rect.y,
      button: "left",
      clickCount: 1,
    });
  }
  await sleep(160);
}

async function stepState(client) {
  return client.evaluateJSON("window.SokoLesson.getState()");
}

async function formalMapState(client, stepId) {
  const selector = `[data-formal-map="${stepId}"]`;
  return client.evaluateJSON(
    `(() => {
      const map = document.querySelector(${JSON.stringify(selector)});
      const text = (node) => node ? node.textContent.trim() : "";
      return {
        exists: !!map,
        open: !!map && map.open,
        badge: text(map && map.querySelector(".fm-badge")),
        summary: text(map && map.querySelector(".fm-summary-text")),
        summaryClipped: (() => {
          const node = map ? map.querySelector(".fm-summary-text") : null;
          return node ? Math.max(0, node.scrollWidth - node.clientWidth) : 0;
        })(),
        title: map && map.querySelector("summary")
          ? map.querySelector("summary").getAttribute("title") || ""
          : "",
        note: text(map && map.querySelector(".fm-note")),
        rows: map ? map.querySelectorAll(".fm-row:not(.fm-head)").length : 0,
        head: map
          ? [...map.querySelectorAll(".fm-head [role=columnheader]")]
              .map((node) => node.textContent.trim())
              .filter(Boolean)
          : [],
        concepts: map
          ? [...map.querySelectorAll(".fm-concept")].map((node) => node.textContent.trim())
          : [],
        formals: map
          ? [...map.querySelectorAll(".fm-formal")].map((node) => node.textContent.trim())
          : [],
        ids: map
          ? [...map.querySelectorAll(".fm-id")].map((node) => node.textContent.trim())
          : [],
        snippets: map ? map.querySelectorAll(".fm-snippet").length : 0,
        codes: map
          ? [...map.querySelectorAll(".fm-code")].map((node) => node.textContent)
          : [],
        code: map
          ? [...map.querySelectorAll(".fm-code")]
              .map((node) => node.textContent)
              .join("\\n")
          : "",
        inputFree: map
          ? map.querySelectorAll("textarea, input, [contenteditable]").length === 0
          : false,
      };
    })()`
  );
}

async function expandFormalMap(client, stepId) {
  const before = await formalMapState(client, stepId);
  assert(before.exists, `${stepId}: 直觉 ↔ 形式 mapping block exists`);
  assert(before.badge === "直觉 ↔ 形式", `${stepId}: mapping badge reads 直觉 ↔ 形式`);
  assert(before.summary.length > 0, `${stepId}: a one-line mapping summary is visible`);
  assert(
    before.summaryClipped <= 1,
    `${stepId}: the one-line summary is fully visible, not ellipsized (clipped ${before.summaryClipped}px)`
  );
  assert(before.title === before.summary, `${stepId}: full summary is exposed on the collapsed line`);
  assert(!before.open, `${stepId}: mapping starts collapsed on mobile`);
  await click(client, `[data-formal-map="${stepId}"] summary`);
  const after = await formalMapState(client, stepId);
  assert(after.open, `${stepId}: clicking the summary expands the mapping`);
  return after;
}

async function assertMapping(client, stepId, expected) {
  const map = await expandFormalMap(client, stepId);
  assert(map.rows === expected.rows, `${stepId}: ${expected.rows} concept ↔ formal rows (got ${map.rows})`);
  assert(
    JSON.stringify(map.head) === JSON.stringify(expected.head),
    `${stepId}: mapping columns are ${expected.head.join(" / ")} (got ${map.head.join(" / ")})`
  );
  for (const identifier of expected.ids) {
    assert(
      map.ids.includes(identifier),
      `${stepId}: a row maps to the exact identifier ${identifier}`
    );
  }
  const prose = [map.summary, map.note, ...map.concepts, ...map.formals].join(" ");
  for (const token of expected.tokens || []) {
    assert(prose.includes(token), `${stepId}: mapping explains ${JSON.stringify(token)}`);
  }
  assert(map.snippets >= 1, `${stepId}: mapping includes a formatted Lean snippet`);
  assert(map.code.includes(expected.code), `${stepId}: snippet shows ${JSON.stringify(expected.code)}`);
  assert(map.inputFree, `${stepId}: mapping has no free-form input control`);
  return map;
}

async function shot(client, name) {
  // Reset to the phone viewport before measuring: a previous full-page capture
  // must not inflate 100dvh and force the next screenshot to stay tall.
  await client.call("Emulation.setDeviceMetricsOverride", {
    width: WIDTH,
    height: HEIGHT,
    deviceScaleFactor: 2,
    mobile: true,
    screenWidth: WIDTH,
    screenHeight: HEIGHT,
  });
  await sleep(80);
  const height = await client.evaluate(
    "Math.ceil(Math.max(document.documentElement.scrollHeight, document.body.scrollHeight, 844))"
  );
  const viewportHeight = Math.min(Math.max(height, HEIGHT), 4200);
  await client.call("Emulation.setDeviceMetricsOverride", {
    width: WIDTH,
    height: viewportHeight,
    deviceScaleFactor: 2,
    mobile: true,
    screenWidth: WIDTH,
    screenHeight: viewportHeight,
  });
  await sleep(220);
  const layout = await client.evaluateJSON(
    `(() => {
      const vw = window.innerWidth;
      const max = Math.max(document.documentElement.scrollWidth, document.body.scrollWidth);
      const offenders = [];
      for (const el of document.querySelectorAll("body *")) {
        if (el.matches(".sr-only, .skip-link, .confetti, .confetti *")) continue;
        const box = el.getBoundingClientRect();
        if (box.width <= 0 || box.height <= 0) continue;
        if (box.right > vw + 1 || box.left < -1) {
          const name = typeof el.className === "string" && el.className
            ? el.className.trim().split(/\\s+/)[0]
            : el.tagName.toLowerCase();
          offenders.push(name + " " + Math.round(box.left) + ".." + Math.round(box.right));
          if (offenders.length >= 6) break;
        }
      }
      return { vw, max, overflow: max - vw, offenders };
    })()`
  );
  assert(layout.overflow <= 1, `${name}: no document horizontal overflow (${layout.overflow}px)`);
  assert(
    layout.offenders.length === 0,
    `${name}: no element crosses the ${WIDTH}px viewport (${layout.offenders.join(", ") || "none"})`
  );
  const response = await client.call("Page.captureScreenshot", { format: "png" });
  const buffer = Buffer.from(response.result.data, "base64");
  writeFileSync(join(args.outDir, `${name}.png`), buffer);
  writeFileSync(join(args.tmpDir, `lesson-v3-${name}.png`), buffer);
  log(`png · ${join(args.outDir, `${name}.png`)} (390×${viewportHeight} css, 2x)`);
  return layout;
}

async function main() {
  const profile = mkdtempSync(join(tmpdir(), "soko-lesson-shot-"));
  let chrome = null;
  let client = null;
  try {
    chrome = launchChrome(profile);
    client = await openPage(profile, chrome);

    await client.call("Page.enable");
    await client.call("Runtime.enable");
    await client.call("Network.enable");
    await client.call("Emulation.setEmulatedMedia", {
      features: [{ name: "prefers-reduced-motion", value: "reduce" }],
    });
    await client.call("Emulation.setDeviceMetricsOverride", {
      width: WIDTH,
      height: HEIGHT,
      deviceScaleFactor: 2,
      mobile: true,
      screenWidth: WIDTH,
      screenHeight: HEIGHT,
    });

    const fileUrl = `file://${BUILD}`;
    await client.call("Page.navigate", { url: fileUrl });
    await waitForApp(client);
    await sleep(250);

    // ── 1. Goal card ───────────────────────────────────────────────────
    let state = await stepState(client);
    assert(state.stepId === "goal" && !state.completed, "step 1 is the goal card");
    const exposedSteps = await client.evaluate("window.SokoLesson.steps");
    assert(
      DATA.steps.length === 6 && exposedSteps === 6,
      `lesson embeds ${DATA.steps.length} steps and exposes ${exposedSteps} (expected 6)`
    );
    const goalTitle = await client.evaluate('document.getElementById("stepTitle").textContent');
    assert(goalTitle.includes("整数里每个数都有相反数"), "goal card states the math goal");
    await shot(client, "first");
    const goalMap = await assertMapping(client, "goal", {
      rows: 3,
      head: ["直觉", "形式实现", "标识符"],
      ids: ["int_add_neg", "Quot.sound", "negRep"],
      tokens: ["代表元", "x + (-x) = 0"],
      code: "theorem int_add_neg",
    });
    assert(
      goalMap.summary.includes("int_add_neg") && goalMap.note.includes("Quot.sound"),
      "goal: the collapsed line names int_add_neg and the expanded note explains Quot.sound"
    );

    // ── 2. Why pairs? (the new representation layer) ───────────────────
    await click(client, "#primaryAction");
    state = await stepState(client);
    assert(state.stepId === "why-pairs", "step 2 is the new why-pairs insight");
    const intro = await client.evaluateJSON(
      `(() => {
        const cards = [...document.querySelectorAll(".chip-card")];
        return {
          cards: cards.length,
          chips: document.querySelectorAll(".chip-card .chip").length,
          pairs: cards.map((card) => card.querySelector(".chip-pair").textContent.trim()),
          meanings: cards.map((card) => card.querySelector(".chip-meaning").textContent.trim()),
          callout: (document.querySelector(".insight-callout") || {}).textContent || "",
          body: document.querySelector(".step-card .body-copy").textContent,
        };
      })()`
    );
    const chipStyle = await client.evaluateJSON(
      `(() => {
        const chip = document.querySelector(".chip-card .chip.tone-blue");
        const style = getComputedStyle(chip);
        return {
          width: chip.getBoundingClientRect().width,
          background: style.backgroundImage,
          radius: style.borderRadius,
        };
      })()`
    );
    assert(
      chipStyle.width >= 12 && chipStyle.background.includes("gradient"),
      `why-pairs paints visible blue chips (${Math.round(chipStyle.width)}px, gradient)`
    );
    assert(intro.cards === 2, "why-pairs shows two counter cards");
    assert(intro.chips === 6, `why-pairs draws 6 blue/red chips (got ${intro.chips})`);
    assert(
      intro.pairs.join(" ") === "(3,0) (0,3)",
      `why-pairs labels the pairs (3,0) and (0,3) (got ${intro.pairs.join(" ")})`
    );
    assert(
      intro.meanings.join(" ").includes("0 个蓝 · 3 个红"),
      "why-pairs spells out the zero-blue case for -3"
    );
    assert(
      intro.body.includes("自然数") && intro.callout.includes("配对写法"),
      "why-pairs explains why natural numbers force the pair representation"
    );
    const pairsMap = await assertMapping(client, "why-pairs", {
      rows: 5,
      head: ["直觉", "形式实现", "标识符"],
      ids: ["@prod_mk Nat Nat a b", "Prod.fst", "Prod.snd", "intRel", "Quot intRel"],
      tokens: ["蓝", "红", "Prod Nat Nat", "Quot intRel"],
      code: "def Int : Type := @Quot.{1} (Prod Nat Nat) intRel",
    });
    assert(
      pairsMap.concepts.join(" ").includes("蓝筹 = 正向") &&
        pairsMap.concepts.join(" ").includes("红筹 = 负向"),
      "why-pairs maps blue=fst and red=snd explicitly"
    );
    await shot(client, "representation-map");

    // ── 3. Same integer, many pairs (representation insight) ───────────
    await click(client, "#primaryAction");
    state = await stepState(client);
    assert(state.stepId === "pair-family", "step 3 asks which pair also represents 3");
    const family = await client.evaluateJSON(
      `(() => ({
        rows: document.querySelectorAll(".family-row").length,
        cancelled: document.querySelectorAll(".family-row .chip.is-cancelled").length,
        prompt: document.querySelector(".step-card .body-copy").textContent,
        caption: (document.querySelector(".family-board .board-caption") || {}).textContent || "",
      }))()`
    );
    const cancelStyle = await client.evaluateJSON(
      `(() => {
        const chip = document.querySelector(".family-row .chip.is-cancelled");
        return { border: getComputedStyle(chip).borderStyle };
      })()`
    );
    assert(cancelStyle.border === "dashed", "cancelled chips use the struck/dashed style");
    assert(family.rows === 3, `pair-family shows (3,0), (4,1), (5,2) (got ${family.rows} rows)`);
    assert(
      family.cancelled === 6,
      `pair-family marks the 3 cancelled blue+red pairs (got ${family.cancelled} cancelled chips)`
    );
    assert(
      family.prompt.includes("抵消"),
      "pair-family prompt teaches one-blue-one-red cancellation"
    );
    await assertMapping(client, "pair-family", {
      rows: 3,
      head: ["直觉", "形式实现", "标识符"],
      ids: ["intRel", "Quot intRel"],
      tokens: ["等价类", "交叉和"],
      code: "intRel (@prod_mk Nat Nat 3 0)",
    });

    // ── 4. Negation = swap ─────────────────────────────────────────────
    await click(client, '[data-choice-id="pair-six-three"]');
    state = await stepState(client);
    assert(state.attempts === 1 && state.xp === 35, `answering (6,3) is correct first try and awards step XP (xp=${state.xp})`);
    const reRenderedMap = await formalMapState(client, "pair-family");
    assert(
      reRenderedMap.open,
      "pair-family: the mapping stays open when the answer feedback re-renders the step"
    );
    await click(client, "#primaryAction");
    state = await stepState(client);
    assert(state.stepId === "pair-swap", "step 4 is negation = swap");
    const swap = await client.evaluateJSON(
      `(() => ({
        chips: document.querySelectorAll(".pairboard .pair-stage").length,
        before: document.querySelectorAll(".pairboard .chip").length,
      }))()`
    );
    assert(swap.chips === 2 && swap.before >= 3, "swap visual shows counters on both sides");
    await click(client, '[data-choice-id="pair-zero-three"]');
    state = await stepState(client);
    assert(state.attempts === 1 && state.xp === 50, `answering (0,3) is correct first try and awards step XP (xp=${state.xp})`);
    const swapped = await client.evaluate(
      'document.querySelectorAll(".pair-stage.is-answer .chip.tone-red").length'
    );
    assert(swapped === 3, `negation swaps (3,0) into 3 red counters (got ${swapped})`);
    await assertMapping(client, "pair-swap", {
      rows: 3,
      head: ["直觉", "形式实现", "标识符"],
      ids: ["negRep", "negRep_wd"],
      tokens: ["良定义", "交换"],
      code: "def negRep (p : Prod Nat Nat)",
    });

    // ── 5. Addition = add counters ─────────────────────────────────────
    await click(client, "#primaryAction");
    state = await stepState(client);
    assert(state.stepId === "pair-sum", "step 5 is addition = add counters");
    const sum = await client.evaluateJSON(
      `(() => ({
        chips: document.querySelectorAll(".sum-board .chip").length,
        labels: [...document.querySelectorAll(".sum-chip-label")].map((node) => node.textContent.trim()),
      }))()`
    );
    assert(sum.chips === 6, `sum board draws 3 blue + 3 red chips (got ${sum.chips})`);
    assert(
      sum.labels.join(" ").includes("3 + 0 = 3") && sum.labels.join(" ").includes("0 + 3 = 3"),
      "sum board adds same colours component-wise"
    );
    await click(client, '[data-choice-id="zero"]');
    state = await stepState(client);
    assert(state.attempts === 1 && state.xp === 65, `answering 0 is correct first try and awards step XP (xp=${state.xp})`);
    const solvedSum = await client.evaluateJSON(
      `(() => ({
        solved: document.querySelector(".sum-board").classList.contains("is-solved"),
        cancelled: document.querySelectorAll(".sum-board .chip.is-cancelled").length,
        result: (document.querySelector(".cancel-result") || {}).textContent.trim(),
      }))()`
    );
    assert(
      solvedSum.solved && solvedSum.cancelled === 6 && solvedSum.result === "0",
      `sum board cancels 3 blue + 3 red to 0 (cancelled=${solvedSum.cancelled}, result=${solvedSum.result})`
    );
    const sumMap = await assertMapping(client, "pair-sum", {
      rows: 4,
      head: ["直觉", "形式实现", "标识符"],
      ids: ["addRep", "add_neg_rep", "int_add_neg", "Quot.sound"],
      tokens: ["代表元", "相等", "同色"],
      code: "Quot.sound (add_neg_rep a b)",
    });
    assert(
      sumMap.snippets === 2,
      `pair-sum shows both addRep and the Quot.sound bridge (got ${sumMap.snippets} snippets)`
    );
    await shot(client, "formal-map");

    // ── 6. Conclusion + badge ──────────────────────────────────────────
    await click(client, "#primaryAction");
    state = await stepState(client);
    assert(state.stepId === "conclusion", "step 6 is the conclusion");
    const conclusionMap = await assertMapping(client, "conclusion", {
      rows: 7,
      head: ["概念", "形式对象", "引理"],
      ids: [
        "Quot intRel",
        "Quot.sound",
        "int_add_zero",
        "negRep_wd",
        "addRep",
        "add_neg_rep",
        "int_add_neg",
      ],
      tokens: ["概念", "形式对象", "引理", "代表元层"],
      code: "add_neg_rep (a b)",
    });
    assert(
      conclusionMap.summary.includes("概念") && conclusionMap.summary.includes("引理"),
      "conclusion: the summary announces the concept / formal object / lemma table"
    );
    const stepProof = await client.evaluateJSON(
      `(() => {
        const proof = document.querySelector(".step-card .proof");
        return {
          exists: !!proof,
          open: !!proof && proof.open,
        };
      })()`
    );
    assert(
      stepProof.exists && !stepProof.open,
      "conclusion: the formal proof evidence layer exists and stays collapsed"
    );
    await click(client, '[data-choice-id="inverse"]');
    state = await stepState(client);
    assert(state.attempts === 1 && state.xp === 80, `conclusion is correct first try and awards the final choice XP (xp=${state.xp})`);
    await click(client, "#primaryAction");
    state = await stepState(client);
    assert(state.completed, "lesson reaches completion");
    assert(
      state.xp === DATA.xp_total && state.xp === 95,
      `completion XP equals xp_total (${state.xp}/${DATA.xp_total})`
    );
    assert(state.maxStreak === 4 && state.firstTryCount === 4, "4 graded steps, all first-try");
    const completionXp = state.xp;
    const complete = await client.evaluateJSON(
      `(() => ({
        badge: (document.querySelector(".badge-pill") || {}).textContent || "",
        title: (document.getElementById("stepTitle") || {}).textContent || "",
        proofOpen: !!document.querySelector(".proof") && document.querySelector(".proof").open,
        stepLabel: (document.getElementById("stepLabel") || {}).textContent || "",
        progress: document.getElementById("progressTrack").getAttribute("aria-valuenow"),
      }))()`
    );
    assert(complete.badge.includes("整数加法有逆元"), "completion unlocks the unit ⑧ badge");
    assert(
      !complete.proofOpen,
      "formal proof stays collapsed on the completion path"
    );
    assert(complete.progress === "100", "progress reaches 100% on completion");
    await shot(client, "complete");

    // ── 7. Wrong answer must not advance or award ──────────────────────
    await client.call("Page.navigate", { url: fileUrl });
    await waitForApp(client);
    await sleep(200);
    await click(client, "#primaryAction");
    await click(client, "#primaryAction");
    state = await stepState(client);
    assert(state.stepId === "pair-family" && state.xp === 15, "wrong-answer run reaches step 3 at 15 XP");
    await click(client, '[data-choice-id="pair-four-two"]');
    state = await stepState(client);
    const wrong = await client.evaluateJSON(
      `(() => ({
        feedback: (document.querySelector(".feedback") || {}).className || "",
        body: (document.querySelector(".feedback-body") || {}).textContent || "",
      }))()`
    );
    assert(state.stepId === "pair-family" && state.attempts === 1, "wrong answer does not advance the step");
    assert(state.xp === 15, "wrong answer does not award XP");
    assert(wrong.feedback.includes("is-wrong"), "wrong answer shows the wrong-feedback panel");
    assert(wrong.body.includes("4 − 2 = 2"), "wrong answer explains (4,2) instead of a generic retry");

    // ── 8. Network + JS health ────────────────────────────────────────
    const requests = [];
    const pageErrors = [];
    const consoleErrors = [];
    for (const event of client.events) {
      if (event.method === "Network.requestWillBeSent") {
        requests.push(event.params.request.url);
      } else if (event.method === "Runtime.exceptionThrown") {
        pageErrors.push(event.params.exceptionDetails?.text || "uncaught exception");
      } else if (
        event.method === "Runtime.consoleAPICalled" &&
        event.params.type === "error"
      ) {
        consoleErrors.push(
          (event.params.args || [])
            .map((arg) => arg.value ?? arg.description ?? "")
            .join(" ") || "console.error"
        );
      }
    }
    const external = requests.filter((url) => !/^(file|data|blob):/.test(url));
    assert(requests.length > 0, "the lesson document was requested");
    assert(external.length === 0, `zero external requests (got ${external.join(", ") || "none"})`);
    assert(pageErrors.length === 0, `no uncaught page errors (got ${pageErrors.join(" | ") || "none"})`);
    assert(consoleErrors.length === 0, `no console errors (got ${consoleErrors.join(" | ") || "none"})`);

    if (failures.length) {
      console.error(`[shot] ${failures.length} check(s) failed`);
      return 1;
    }
    const summary = {
      ok: true,
      steps: DATA.steps.length,
      xp: completionXp,
      mapping_blocks: DATA.steps.length,
      screenshots: ["first", "representation-map", "formal-map", "complete"].map(
        (name) => join(args.outDir, `${name}.png`)
      ),
      requests: requests.length,
      external_requests: 0,
      horizontal_overflow: false,
    };
    console.log(JSON.stringify(summary, null, 2));
    return 0;
  } finally {
    try {
      client?.close();
    } catch {
      /* ignore */
    }
    try {
      if (!args.keep) chrome?.kill("SIGTERM");
    } catch {
      /* ignore */
    }
    if (!args.keep) {
      await sleep(150);
      try {
        rmSync(profile, { recursive: true, force: true });
      } catch {
        /* ignore */
      }
    }
    client = null;
    chrome = null;
  }
}

try {
  process.exitCode = await main();
} catch (error) {
  console.error("[shot] fatal:", error.stack || error.message);
  process.exitCode = 1;
}

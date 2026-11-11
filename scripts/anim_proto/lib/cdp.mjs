// scripts/anim_proto/lib/cdp.mjs —— 最小 Chrome DevTools Protocol 客户端。
//
// 不依赖 npm：Node 24 自带 fetch / WebSocket / net。只做本探针需要的几件事：
//   启动 headless Chrome → 找到 page target → 连接 → Runtime/Network/Page。
// 截帧与验收共用这一份，避免 shot 脚本各写一套。

import { spawn, spawnSync } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import net from "node:net";

export const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

const MAC_CHROME = "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";

function which(name) {
  const found = spawnSync("which", [name], { encoding: "utf8" });
  return found.status === 0 && found.stdout.trim() ? found.stdout.trim() : "";
}

export function findChrome(explicit = "") {
  if (explicit) {
    if (!existsSync(explicit)) throw new Error(`Chrome not found at ${explicit}`);
    return explicit;
  }
  if (process.env.SOKO_CHROME) {
    if (!existsSync(process.env.SOKO_CHROME)) {
      throw new Error(`SOKO_CHROME points at a missing file: ${process.env.SOKO_CHROME}`);
    }
    return process.env.SOKO_CHROME;
  }
  if (process.platform === "darwin" && existsSync(MAC_CHROME)) return MAC_CHROME;
  for (const candidate of [
    "google-chrome-stable",
    "google-chrome",
    "chromium",
    "chromium-browser",
    "microsoft-edge",
    "microsoft-edge-stable",
  ]) {
    const found = which(candidate);
    if (found) return found;
  }
  throw new Error(
    "Chrome/Chromium not found. Set SOKO_CHROME=/path/to/chrome; this probe does not install dependencies."
  );
}

export async function freePort() {
  const server = net.createServer();
  await new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", resolve);
  });
  const address = server.address();
  const port = address.port;
  await new Promise((resolve) => server.close(resolve));
  return port;
}

export function launchChrome({ chrome, port, width = 1440, height = 1100, profile }) {
  const userDataDir = profile || mkdtempSync(join(tmpdir(), "soko-anim-profile-"));
  const args = [
    "--headless=new",
    `--remote-debugging-port=${port}`,
    `--user-data-dir=${userDataDir}`,
    "--no-first-run",
    "--no-default-browser-check",
    "--disable-gpu",
    "--hide-scrollbars",
    "--mute-audio",
    "--force-color-profile=srgb",
    "--disable-background-timer-throttling",
    "--disable-renderer-backgrounding",
    "--disable-backgrounding-occluded-windows",
    `--window-size=${width},${height}`,
    "about:blank",
  ];
  const child = spawn(chrome, args, { stdio: ["ignore", "ignore", "pipe"] });
  let stderr = "";
  child.stderr?.on("data", (buffer) => {
    stderr = (stderr + buffer.toString()).slice(-4000);
  });
  child.on("error", (error) => {
    stderr += `\nspawn error: ${error.message}`;
  });
  child.getStderr = () => stderr;
  child.userDataDir = userDataDir;
  return child;
}

async function targets(port) {
  const response = await fetch(`http://127.0.0.1:${port}/json/list`);
  if (!response.ok) throw new Error(`CDP /json/list returned ${response.status}`);
  return response.json();
}

export async function connectPage({ port, timeoutMs = 25000 }) {
  const deadline = Date.now() + timeoutMs;
  let lastError = null;
  while (Date.now() < deadline) {
    try {
      const pages = (await targets(port)).filter((target) => target.type === "page");
      if (pages.length) return await connect(pages[0].webSocketDebuggerUrl);
    } catch (error) {
      lastError = error;
    }
    await sleep(150);
  }
  throw new Error(`Chrome did not expose a page target on :${port} (${lastError?.message || "timeout"})`);
}

export async function connect(wsUrl) {
  const ws = new WebSocket(wsUrl);
  await new Promise((resolve, reject) => {
    ws.addEventListener("open", resolve, { once: true });
    ws.addEventListener("error", () => reject(new Error("CDP websocket failed to open")), { once: true });
  });
  let nextId = 0;
  const pending = new Map();
  const events = [];
  ws.addEventListener("message", (event) => {
    const message = JSON.parse(event.data);
    if (message.id) {
      const resolver = pending.get(message.id);
      if (resolver) {
        pending.delete(message.id);
        resolver(message);
      }
    } else {
      events.push(message);
    }
  });
  const call = (method, params = {}) =>
    new Promise((resolve) => {
      const id = ++nextId;
      pending.set(id, resolve);
      ws.send(JSON.stringify({ id, method, params }));
    });
  const client = {
    events,
    call,
    close: () => {
      try { ws.close(); } catch { /* ignore */ }
    },
    async evaluate(expression) {
      const response = await call("Runtime.evaluate", {
        expression,
        returnByValue: true,
        awaitPromise: true,
      });
      if (response.result?.exceptionDetails) {
        throw new Error(
          "page evaluate failed: " +
            (response.result.exceptionDetails.exception?.description ||
              response.result.exceptionDetails.text ||
              "unknown exception")
        );
      }
      return response.result?.result?.value;
    },
    async evaluateJSON(expression) {
      const value = await client.evaluate(`JSON.stringify(${expression})`);
      return value === undefined ? undefined : JSON.parse(value);
    },
  };
  return client;
}

export async function openProbe({
  htmlPath,
  chrome,
  width = 1440,
  height = 1100,
  dpr = 2,
  port = null,
  keep = false,
}) {
  const browser = findChrome(chrome);
  const chosenPort = port || (await freePort());
  const child = launchChrome({ chrome: browser, port: chosenPort, width, height });
  let client = null;
  const close = async () => {
    try { client?.close(); } catch { /* ignore */ }
    try { child.kill("SIGTERM"); } catch { /* ignore */ }
    await sleep(220);
    if (!child.killed && child.exitCode === null) {
      try { child.kill("SIGKILL"); } catch { /* ignore */ }
    }
    if (!keep) {
      try { rmSync(child.userDataDir, { recursive: true, force: true }); } catch { /* ignore */ }
    }
  };
  try {
    client = await connectPage({ port: chosenPort });
    await client.call("Page.enable");
    await client.call("Runtime.enable");
    await client.call("Network.enable");
    await setViewport(client, { width, height, dpr, mobile: width < 700 });
    const url = "file://" + htmlPath;
    const nav = await client.call("Page.navigate", { url });
    if (nav.errorText) throw new Error(`navigation failed: ${nav.errorText}`);
    const deadline = Date.now() + 20000;
    while (Date.now() < deadline) {
      const ready = await client.evaluate("typeof window.SokoAnim !== 'undefined'");
      if (ready) return { client, child, port: chosenPort, close, url };
      await sleep(100);
    }
    throw new Error("window.SokoAnim never became ready");
  } catch (error) {
    await close();
    throw error;
  }
}

export async function setViewport(client, { width, height, dpr = 1, mobile = false }) {
  await client.call("Emulation.setDeviceMetricsOverride", {
    width,
    height,
    deviceScaleFactor: dpr,
    mobile,
    screenWidth: width,
    screenHeight: height,
  });
}

export async function clickSelector(client, selector) {
  // 页面有 scroll-behavior: smooth；先 instant 滚到可视区，等一帧再量坐标，
  // 否则首次点击会落在滚动前的旧位置（实测 Step 第一次会点空）。
  const found = await client.evaluate(
    `(() => {
      const el = document.querySelector(${JSON.stringify(selector)});
      if (!el) return false;
      el.scrollIntoView({ block: "center", inline: "nearest", behavior: "instant" });
      return true;
    })()`
  );
  if (!found) throw new Error(`click target not found: ${selector}`);
  await sleep(60);
  const rect = await client.evaluateJSON(
    `(() => {
      const el = document.querySelector(${JSON.stringify(selector)});
      const r = el.getBoundingClientRect();
      return { x: r.left + r.width / 2, y: r.top + r.height / 2, w: r.width, h: r.height };
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
  await sleep(120);
  return rect;
}

export async function pressKey(client, { key, code, windowsVirtualKeyCode, text }) {
  const base = {
    key,
    code: code || key,
    windowsVirtualKeyCode: windowsVirtualKeyCode || 0,
    nativeVirtualKeyCode: windowsVirtualKeyCode || 0,
  };
  await client.call("Input.dispatchKeyEvent", { type: "keyDown", ...base, ...(text ? { text } : {}) });
  await client.call("Input.dispatchKeyEvent", { type: "keyUp", ...base });
  await sleep(120);
}

export async function capture(client, { format = "png", clip = null, captureBeyondViewport = false } = {}) {
  const params = { format };
  if (clip) params.clip = { ...clip, scale: clip.scale || 1 };
  params.captureBeyondViewport = captureBeyondViewport;
  const response = await client.call("Page.captureScreenshot", params);
  if (response.error) throw new Error(`captureScreenshot failed: ${JSON.stringify(response.error)}`);
  return Buffer.from(response.result.data, "base64");
}

export async function setReducedMotion(client, reduce) {
  await client.call("Emulation.setEmulatedMedia", {
    media: "screen",
    features: [{ name: "prefers-reduced-motion", value: reduce ? "reduce" : "no-preference" }],
  });
  await sleep(80);
}

export function ensureDir(path) {
  mkdirSync(path, { recursive: true });
}

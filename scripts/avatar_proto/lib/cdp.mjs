// scripts/avatar_proto/lib/cdp.mjs — minimal Chrome DevTools Protocol client.
//
// No npm dependencies: Node 24 already ships fetch / WebSocket / net.
// We only need launch headless Chrome, attach to the page, drive real clicks,
// measure layout, watch network/console, and capture screenshots.
// The same module powers check.mjs (assertions) and shots.mjs (evidence PNGs).

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
    "Chrome/Chromium not found. Set SOKO_CHROME=/path/to/chrome; this probe installs nothing."
  );
}

export async function freePort() {
  const server = net.createServer();
  await new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", resolve);
  });
  const { port } = server.address();
  await new Promise((resolve) => server.close(resolve));
  return port;
}

function launchChrome({ chrome, port, width, height }) {
  const userDataDir = mkdtempSync(join(tmpdir(), "soko-avatar-profile-"));
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
    stderr = (stderr + buffer.toString()).slice(-6000);
  });
  child.on("error", (error) => {
    stderr += `\nspawn error: ${error.message}`;
  });
  child.getStderr = () => stderr;
  child.userDataDir = userDataDir;
  return child;
}

async function listTargets(port) {
  const response = await fetch(`http://127.0.0.1:${port}/json/list`);
  if (!response.ok) throw new Error(`CDP /json/list returned ${response.status}`);
  return response.json();
}

async function connectPage({ port, timeoutMs = 25000 }) {
  const deadline = Date.now() + timeoutMs;
  let lastError = null;
  while (Date.now() < deadline) {
    try {
      const pages = (await listTargets(port)).filter((target) => target.type === "page");
      if (pages.length) return await connect(pages[0].webSocketDebuggerUrl);
    } catch (error) {
      lastError = error;
    }
    await sleep(120);
  }
  throw new Error(`Chrome exposed no page target on :${port} (${lastError?.message || "timeout"})`);
}

async function connect(wsUrl) {
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
      try {
        ws.close();
      } catch {
        /* ignore */
      }
    },
    async evaluate(expression) {
      const response = await call("Runtime.evaluate", {
        expression,
        returnByValue: true,
        awaitPromise: true,
      });
      if (response.result?.exceptionDetails) {
        const details = response.result.exceptionDetails;
        throw new Error(
          "page evaluate failed: " +
            (details.exception?.description || details.text || "unknown exception")
        );
      }
      return response.result?.result?.value;
    },
  };
  return client;
}

/**
 * Open index.html in headless Chrome and wait until `readyExpression` is truthy.
 * @param {object} opts
 * @param {string} opts.htmlPath absolute path
 * @param {string} [opts.query] e.g. "?view=sheet"
 * @param {string} [opts.readyExpression] defaults to window.SokoAvatar
 */
export async function openProbe({
  htmlPath,
  chrome,
  width = 1440,
  height = 1000,
  dpr = 1,
  port = null,
  query = "",
  readyExpression = "typeof window.SokoAvatar !== 'undefined'",
}) {
  const browser = findChrome(chrome);
  const chosenPort = port || (await freePort());
  const child = launchChrome({ chrome: browser, port: chosenPort, width, height });
  let client = null;
  const close = async () => {
    try {
      client?.close();
    } catch {
      /* ignore */
    }
    try {
      child.kill("SIGTERM");
    } catch {
      /* ignore */
    }
    await sleep(180);
    if (!child.killed && child.exitCode === null) {
      try {
        child.kill("SIGKILL");
      } catch {
        /* ignore */
      }
    }
    try {
      rmSync(child.userDataDir, { recursive: true, force: true });
    } catch {
      /* ignore */
    }
  };
  try {
    client = await connectPage({ port: chosenPort });
    await client.call("Page.enable");
    await client.call("Runtime.enable");
    await client.call("Network.enable");
    await setViewport(client, { width, height, dpr, mobile: width < 700 });
    const url = "file://" + htmlPath + query;
    const nav = await client.call("Page.navigate", { url });
    if (nav.errorText) throw new Error(`navigation failed: ${nav.errorText}`);
    const deadline = Date.now() + 20000;
    while (Date.now() < deadline) {
      try {
        const ready = await client.evaluate(`Boolean(${readyExpression})`);
        if (ready) return { client, child, port: chosenPort, close, url, htmlPath, chrome: browser };
      } catch {
        // document.body can be null while the new document is still parsing;
        // evaluate() throws in that window, so keep polling.
      }
      await sleep(80);
    }
    throw new Error("page never became ready: " + readyExpression);
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

export async function setReducedMotion(client, reduce) {
  await client.call("Emulation.setEmulatedMedia", {
    media: "screen",
    features: [{ name: "prefers-reduced-motion", value: reduce ? "reduce" : "no-preference" }],
  });
  await sleep(60);
}

export async function reload(client) {
  await client.call("Page.reload", { ignoreCache: true });
}

export async function waitFor(client, expression, timeoutMs = 5000) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    try {
      if (await client.evaluate(`Boolean(${expression})`)) return true;
    } catch {
      // document.body can be null while a reload is still parsing.
    }
    await sleep(40);
  }
  throw new Error(`waitFor timed out: ${expression}`);
}

export async function clickSelector(client, selector) {
  const found = await client.evaluate(
    `(() => {
      const el = document.querySelector(${JSON.stringify(selector)});
      if (!el) return false;
      el.scrollIntoView({ block: "center", inline: "nearest", behavior: "instant" });
      return true;
    })()`
  );
  if (!found) throw new Error(`click target not found: ${selector}`);
  await sleep(50);
  const rect = await client.evaluate(
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
  await sleep(90);
  return rect;
}

export async function pressKey(client, { key, code, windowsVirtualKeyCode }) {
  const base = {
    key,
    code: code || key,
    windowsVirtualKeyCode: windowsVirtualKeyCode || 0,
    nativeVirtualKeyCode: windowsVirtualKeyCode || 0,
  };
  await client.call("Input.dispatchKeyEvent", { type: "keyDown", ...base });
  await client.call("Input.dispatchKeyEvent", { type: "keyUp", ...base });
  await sleep(80);
}

export async function capture(client, { format = "png", clip = null, captureBeyondViewport = false } = {}) {
  const params = { format };
  if (clip) params.clip = { ...clip, scale: clip.scale || 1 };
  params.captureBeyondViewport = captureBeyondViewport;
  const response = await client.call("Page.captureScreenshot", params);
  if (response.error) throw new Error(`captureScreenshot failed: ${JSON.stringify(response.error)}`);
  return Buffer.from(response.result.data, "base64");
}

export async function fullPageCapture(client, { format = "png", maxHeight = 12000 } = {}) {
  const metrics = await client.call("Page.getLayoutMetrics");
  const result = metrics.result || metrics;
  const content = result.cssContentSize || result.contentSize;
  const width = Math.ceil(content.width);
  const height = Math.min(Math.ceil(content.height), maxHeight);
  return capture(client, {
    format,
    clip: { x: 0, y: 0, width, height, scale: 1 },
    captureBeyondViewport: true,
  });
}

export function ensureDir(path) {
  mkdirSync(path, { recursive: true });
}

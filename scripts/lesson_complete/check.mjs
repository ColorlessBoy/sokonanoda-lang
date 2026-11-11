#!/usr/bin/env node
// scripts/lesson_complete/check.mjs — one re-runnable acceptance command.
//
//   node scripts/lesson_complete/check.mjs
//   node scripts/lesson_complete/check.mjs --json
//   node scripts/lesson_complete/check.mjs --no-browser
//   SOKO_CHROME=/path/to/chrome node scripts/lesson_complete/check.mjs
//
// Exit codes: 0 = all checks pass, 1 = a check failed, 3 = Chrome unavailable.

import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { findChrome, openProbe } from "./lib/cdp.mjs";
import { staticChecks } from "./lib/verify-static.mjs";
import { browserChecks } from "./lib/verify-browser.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const log = (...args) => console.log("[check]", ...args);

function parseArgs(argv) {
  const args = {
    html: join(HERE, "index.html"),
    chrome: "",
    port: null,
    keep: false,
    json: false,
    browser: true,
  };
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === "--html") args.html = resolve(argv[++i]);
    else if (arg === "--chrome") args.chrome = resolve(argv[++i]);
    else if (arg === "--port") args.port = Number(argv[++i]);
    else if (arg === "--keep") args.keep = true;
    else if (arg === "--json") args.json = true;
    else if (arg === "--no-browser") args.browser = false;
    else if (arg === "-h" || arg === "--help") {
      console.log(
        "Usage: node scripts/lesson_complete/check.mjs [--html FILE] [--chrome PATH] [--port N] [--keep] [--json] [--no-browser]"
      );
      process.exit(0);
    } else {
      console.error(`[check] unknown argument: ${arg}`);
      process.exit(2);
    }
  }
  return args;
}

const args = parseArgs(process.argv.slice(2));
const failures = [];

const staticResult = staticChecks();
if (!args.json) {
  for (const failure of staticResult.failures) log("FAIL ·", failure);
  log(
    `static: ${staticResult.ok ? "ok" : "failed"} · ${staticResult.checks} checks · ${staticResult.info.bytes ?? 0} bytes · sha ${(staticResult.info.sha256 ?? "?").slice(0, 12)}`
  );
}
if (!staticResult.ok) failures.push(...staticResult.failures.map((message) => `static: ${message}`));

let browserResult = null;
let browserError = null;

if (args.browser && staticResult.ok) {
  let probe = null;
  try {
    findChrome(args.chrome);
    probe = await openProbe({
      htmlPath: args.html,
      chrome: args.chrome,
      width: 1440,
      height: 1000,
      dpr: 1,
      port: args.port,
      keep: args.keep,
    });
    browserResult = await browserChecks({
      client: probe.client,
      log: args.json ? () => {} : (message) => log(message),
    });
    if (!args.json) {
      for (const failure of browserResult.failures) log("FAIL ·", failure);
      log(`browser: ${browserResult.ok ? "ok" : "failed"} · ${browserResult.checks} checks`);
    }
    if (!browserResult.ok) {
      failures.push(...browserResult.failures.map((message) => `browser: ${message}`));
    }
  } catch (error) {
    browserError = error;
    failures.push(`browser: ${error.message}`);
    if (!args.json) log("FAIL ·", error.stack || error.message);
  } finally {
    if (probe) await probe.close();
  }
} else if (args.browser && !staticResult.ok) {
  if (!args.json) log("browser: skipped because static checks failed");
}

const summary = {
  ok: failures.length === 0,
  static: {
    ok: staticResult.ok,
    checks: staticResult.checks,
    failures: staticResult.failures,
    bytes: staticResult.info.bytes ?? null,
    sha256: staticResult.info.sha256 ?? null,
    identical: staticResult.info.identical ?? false,
    mathChecks: staticResult.info.mathChecks ?? 0,
  },
  browser: browserResult
    ? {
        ok: browserResult.ok,
        checks: browserResult.checks,
        failures: browserResult.failures,
        screenshots: browserResult.details.screenshots,
        layout: browserResult.details.layout,
        mathRows: browserResult.details.mathRows,
        requests: browserResult.details.requests,
        consoleErrors: browserResult.details.consoleErrors,
      }
    : null,
  failures,
};

if (args.json) {
  console.log(JSON.stringify(summary, null, 2));
} else {
  console.log(
    JSON.stringify(
      {
        ok: summary.ok,
        static: summary.static.ok,
        browser: summary.browser ? summary.browser.ok : false,
        checks: staticResult.checks + (browserResult ? browserResult.checks : 0),
        failures: failures.length,
        screenshots: browserResult ? browserResult.details.screenshots : [],
        layout: browserResult
          ? Object.fromEntries(
              Object.entries(browserResult.details.layout).map(([name, value]) => [
                name,
                { width: value.width, scrollWidth: value.scrollWidth, scrollHeight: value.scrollHeight, bad: value.bad },
              ])
            )
          : null,
      },
      null,
      2
    )
  );
  if (browserError && /Chrome\/Chromium not found/.test(browserError.message)) {
    console.error("[check] Chrome is required for the browser layer; set SOKO_CHROME.");
  }
}

if (failures.length > 0 && browserError && /Chrome\/Chromium not found/.test(browserError.message) && staticResult.ok) {
  process.exitCode = 3;
} else {
  process.exitCode = failures.length === 0 ? 0 : 1;
}

#!/usr/bin/env node
// scripts/avatar_proto/check.mjs — runnable acceptance for the 小析 probe.
//
//   node scripts/avatar_proto/check.mjs
//   node scripts/avatar_proto/check.mjs --json
//   SOKO_CHROME=/path/to/chrome node scripts/avatar_proto/check.mjs
//
// rc 0 = all checks pass; 1 = a check failed; 2 = usage; 3 = Chrome missing.

import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { openProbe, findChrome } from "./lib/cdp.mjs";
import { staticChecks, browserChecks, sheetChecks } from "./lib/verify.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const args = { html: join(HERE, "index.html"), chrome: "", json: false };
for (let i = 2; i < process.argv.length; i += 1) {
  const arg = process.argv[i];
  if (arg === "--html") args.html = resolve(process.argv[++i]);
  else if (arg === "--chrome") args.chrome = resolve(process.argv[++i]);
  else if (arg === "--json") args.json = true;
  else if (arg === "-h" || arg === "--help") {
    console.log("Usage: node scripts/avatar_proto/check.mjs [--html FILE] [--chrome PATH] [--json]");
    process.exit(0);
  } else {
    console.error(`[check] unknown argument: ${arg}`);
    process.exit(2);
  }
}

const log = (...values) => {
  if (!args.json) console.log("[check]", ...values);
};
const failures = [];
let browser = null;
let sheet = null;
let chromePath = "";
let chromeMissing = false;

const previewPath = join(HERE, "preview", "index.html");

// File layer first: it is fast and catches accidental external assets.
const staticResult = staticChecks({ htmlPath: args.html, previewPath });
if (!args.json) {
  log(`static: ${staticResult.ok ? "ok" : "failed"} · ${staticResult.info.bytes ?? 0} bytes · sha ${staticResult.info.sha256 ?? "?"}`);
  for (const failure of staticResult.failures) log("FAIL ·", failure);
}
if (!staticResult.ok) failures.push(...staticResult.failures.map((failure) => `static: ${failure}`));

if (staticResult.ok) {
  try {
    chromePath = findChrome(args.chrome);
    const probe = await openProbe({
      htmlPath: args.html,
      chrome: args.chrome,
      width: 1440,
      height: 1100,
      dpr: 1,
      readyExpression: "document.body && document.body.dataset.ready === '1'",
    });
    try {
      browser = await browserChecks({ client: probe.client, log });
      if (!args.json) {
        log(`browser: ${browser.ok ? "ok" : "failed"} · ${browser.checks} checks`);
        for (const failure of browser.failures) log("FAIL ·", failure);
        log(`detail: ${JSON.stringify(browser.details)}`);
      }
      if (!browser.ok) failures.push(...browser.failures.map((failure) => `browser: ${failure}`));
    } finally {
      await probe.close();
    }

    const sheetProbe = await openProbe({
      htmlPath: args.html,
      chrome: args.chrome,
      width: 1440,
      height: 900,
      dpr: 1,
      query: "?view=sheet",
      readyExpression: "document.body && document.body.dataset.ready === '1'",
    });
    try {
      sheet = await sheetChecks({ client: sheetProbe.client, log });
      if (!args.json) {
        log(`sheet: ${sheet.ok ? "ok" : "failed"} · ${sheet.checks} checks`);
        for (const failure of sheet.failures) log("FAIL ·", failure);
      }
      if (!sheet.ok) failures.push(...sheet.failures.map((failure) => `sheet: ${failure}`));
    } finally {
      await sheetProbe.close();
    }
  } catch (error) {
    if (/Chrome\/Chromium not found|SOKO_CHROME points at a missing file|Chrome not found/.test(error.message)) {
      chromeMissing = true;
      failures.push(`browser: ${error.message}`);
    } else {
      failures.push(`browser: ${error.stack || error.message}`);
    }
    if (!args.json) log("FAIL ·", error.message);
  }
}

const summary = {
  ok: failures.length === 0,
  static: { ok: staticResult.ok, ...staticResult.info },
  browser: browser ? { ok: browser.ok, checks: browser.checks, details: browser.details } : null,
  sheet: sheet ? { ok: sheet.ok, checks: sheet.checks } : null,
  chrome: chromePath || null,
  failures,
};
if (args.json) {
  console.log(JSON.stringify(summary, null, 2));
} else {
  console.log("[check]", JSON.stringify({
    ok: summary.ok,
    static: summary.static.ok,
    browser: summary.browser ? summary.browser.ok : false,
    browser_checks: summary.browser?.checks ?? 0,
    sheet: summary.sheet ? summary.sheet.ok : false,
    sheet_checks: summary.sheet?.checks ?? 0,
    failures: summary.failures.length,
  }));
}
process.exitCode = failures.length === 0 ? 0 : chromeMissing ? 3 : 1;

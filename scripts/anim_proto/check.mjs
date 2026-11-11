#!/usr/bin/env node
// scripts/anim_proto/check.mjs —— 可复跑的验收入口（无 npm 依赖）。
//
//   node scripts/anim_proto/check.mjs
//   node scripts/anim_proto/check.mjs --json
//   SOKO_CHROME=/path/to/chrome node scripts/anim_proto/check.mjs
//
// 断言分文件层与浏览器层；退出码 0 = 全部通过，1 = 有失败，3 = 依赖缺失。

import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { openProbe, findChrome } from "./lib/cdp.mjs";
import { staticChecks, browserChecks } from "./lib/verify.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const log = (...args) => console.log("[check]", ...args);

function parseArgs(argv) {
  const args = { html: join(HERE, "index.html"), chrome: "", port: null, keep: false, json: false };
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === "--html") args.html = resolve(argv[++i]);
    else if (arg === "--chrome") args.chrome = resolve(argv[++i]);
    else if (arg === "--port") args.port = Number(argv[++i]);
    else if (arg === "--keep") args.keep = true;
    else if (arg === "--json") args.json = true;
    else if (arg === "-h" || arg === "--help") {
      console.log(`Usage: node scripts/anim_proto/check.mjs [--html FILE] [--chrome PATH] [--port N] [--keep] [--json]`);
      process.exit(0);
    } else {
      console.error(`[check] unknown argument: ${arg}`);
      process.exit(2);
    }
  }
  return args;
}

const args = parseArgs(process.argv.slice(2));
const previewPath = join(HERE, "preview", "index.html");
const failures = [];

const staticResult = staticChecks({ htmlPath: args.html, previewPath });
if (!args.json) {
  for (const failure of staticResult.failures) log("FAIL ·", failure);
  log(`static: ${staticResult.ok ? "ok" : "failed"} · ${staticResult.info.bytes ?? 0} bytes · sha ${staticResult.info.sha256 ?? "?"}`);
}
if (!staticResult.ok) {
  failures.push(...staticResult.failures.map((message) => `static: ${message}`));
}

let browserResult = null;
let probe = null;
if (staticResult.ok) {
  try {
    findChrome(args.chrome);
    probe = await openProbe({
      htmlPath: args.html,
      chrome: args.chrome,
      width: 1440,
      height: 1100,
      dpr: 1,
      port: args.port,
      keep: args.keep,
    });
    browserResult = await browserChecks({ client: probe.client });
    if (!args.json) {
      for (const failure of browserResult.failures) log("FAIL ·", failure);
      log(`browser: ${browserResult.ok ? "ok" : "failed"} · ${browserResult.checks} checks`);
    }
    if (!browserResult.ok) failures.push(...browserResult.failures.map((message) => `browser: ${message}`));
  } catch (error) {
    const message = error.stack || error.message;
    if (!args.json) log("FAIL ·", message);
    failures.push(`browser: ${error.message}`);
  } finally {
    if (probe) await probe.close();
  }
}

const summary = {
  ok: failures.length === 0,
  static: { ok: staticResult.ok, ...staticResult.info },
  browser: browserResult
    ? {
        ok: browserResult.ok,
        checks: browserResult.checks,
        details: browserResult.details,
      }
    : null,
  failures,
};

if (args.json) {
  console.log(JSON.stringify(summary, null, 2));
} else {
  console.log(JSON.stringify({
    ok: summary.ok,
    static: summary.static.ok,
    browser: summary.browser ? summary.browser.ok : false,
    checks: summary.browser?.checks ?? 0,
    failures: summary.failures.length,
    external_requests: summary.browser?.details?.externalRequests ?? null,
    layout: summary.browser?.details?.layout
      ? Object.fromEntries(Object.entries(summary.browser.details.layout).map(([name, value]) => [name, { overflow: value.overflow, labels: value.labels }]))
      : null,
  }, null, 2));
}
process.exitCode = failures.length === 0 ? 0 : 1;

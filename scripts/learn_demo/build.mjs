#!/usr/bin/env node
// scripts/learn_demo/build.mjs — validate content and regenerate index.html.
//
//   node scripts/learn_demo/build.mjs            # write index.html + preview
//   node scripts/learn_demo/build.mjs --check    # fail if committed output drifts
//   node scripts/learn_demo/build.mjs --json
//
// No npm dependencies, no network, no external assets.

import { buildDemo } from "./lib/page.mjs";

const args = process.argv.slice(2);
let write = true;
let json = false;
for (const arg of args) {
  if (arg === "--check") write = false;
  else if (arg === "--json") json = true;
  else if (arg === "-h" || arg === "--help") {
    console.log("Usage: node scripts/learn_demo/build.mjs [--check] [--json]");
    process.exit(0);
  } else {
    console.error(`[build] unknown argument: ${arg}`);
    process.exit(2);
  }
}

const result = buildDemo({ write });
const summary = {
  ok: result.ok && (!write ? !result.drift : true),
  errors: result.errors,
  bytes: result.bytes ?? 0,
  sha256: result.sha256 ?? null,
  identical: Boolean(result.identical),
  drift: Boolean(result.drift),
  wrote: write,
};
if (json) {
  console.log(JSON.stringify(summary, null, 2));
} else {
  if (!result.ok) {
    for (const error of result.errors) console.error("[build] FAIL ·", error);
  } else {
    console.log(
      `[build] ${write ? "wrote" : "checked"} index.html + preview/index.html · ${result.bytes} bytes · sha ${result.sha256.slice(0, 12)}`
    );
    if (!write && result.drift) console.error("[build] FAIL · committed output drifted from sources; run node scripts/learn_demo/build.mjs");
    if (!result.identical) console.error("[build] FAIL · preview copy is not byte-identical");
  }
}
process.exitCode = summary.ok ? 0 : 1;

#!/usr/bin/env node
// scripts/lesson_complete/build.mjs — render the self-contained lesson page.
//
//   node scripts/lesson_complete/build.mjs
//   node scripts/lesson_complete/build.mjs --check   # rc=1 on drift
//   node scripts/lesson_complete/build.mjs --json

import { buildLesson } from "./lib/page.mjs";

const args = process.argv.slice(2);
const check = args.includes("--check");
const json = args.includes("--json");
if (args.some((arg) => !["--check", "--json", "-h", "--help"].includes(arg))) {
  console.error("Usage: node scripts/lesson_complete/build.mjs [--check] [--json]");
  process.exit(2);
}
if (args.includes("-h") || args.includes("--help")) {
  console.log("Usage: node scripts/lesson_complete/build.mjs [--check] [--json]");
  process.exit(0);
}

let result;
try {
  result = buildLesson({ check, write: true });
} catch (error) {
  if (json) console.log(JSON.stringify({ ok: false, error: error.message }, null, 2));
  else console.error("[build] FAIL ·", error.message);
  process.exit(1);
}

if (json) {
  console.log(JSON.stringify(result, null, 2));
} else if (check) {
  console.log(
    `[build] ${result.ok ? "ok" : "DRIFT"} · ${result.bytes} bytes · sha ${result.sha256.slice(0, 12)}` +
      (result.ok ? " · index.html and preview/index.html are current" : " · run node scripts/lesson_complete/build.mjs")
  );
} else {
  console.log(
    `[build] wrote index.html + preview/index.html · ${result.bytes} bytes · sha ${result.sha256.slice(0, 12)}`
  );
}

process.exit(result.ok ? 0 : 1);

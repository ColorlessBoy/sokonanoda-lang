#!/usr/bin/env node
// scripts/avatar_proto/build.mjs — generate the raw.githack preview copy.
//
// The page is one self-contained HTML file. preview/ is a byte-identical copy
// (no injection, no rewriting) so the acceptance check can diff it and reviewers
// can open it through raw.githack.
//
//   node scripts/avatar_proto/build.mjs
//   node scripts/avatar_proto/build.mjs --check   # verify only, write nothing

import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { createHash } from "node:crypto";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const SOURCE = join(HERE, "index.html");
const PREVIEW = join(HERE, "preview", "index.html");
const checkOnly = process.argv.includes("--check");

const source = readFileSync(SOURCE);
const html = source.toString("utf8");
const problems = [];
if (/https?:\/\//i.test(html)) problems.push("absolute http(s) URL present");
if (!/<meta name="viewport"/.test(html)) problems.push("missing viewport meta");
if (!/<html lang="zh-CN">/.test(html)) problems.push("missing zh-CN lang");
if (!/<title>[^<]*小析[^<]*<\/title>/.test(html)) problems.push("title does not name 小析");
if (problems.length) {
  console.error("[build] refusing to build:");
  for (const problem of problems) console.error("  -", problem);
  process.exit(1);
}

const sha = createHash("sha256").update(source).digest("hex");
if (!checkOnly) {
  mkdirSync(dirname(PREVIEW), { recursive: true });
  writeFileSync(PREVIEW, source);
}

let preview = null;
try {
  preview = readFileSync(PREVIEW);
} catch {
  if (!checkOnly) throw new Error(`preview was not written: ${PREVIEW}`);
}
const identical = Boolean(preview && preview.equals(source));
const result = {
  ok: identical,
  source: SOURCE,
  preview: PREVIEW,
  bytes: source.length,
  sha256: sha,
  identical,
  written: !checkOnly,
};
console.log(JSON.stringify(result, null, 2));
if (!identical) process.exitCode = 1;

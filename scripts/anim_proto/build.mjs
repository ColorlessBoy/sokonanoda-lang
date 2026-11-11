#!/usr/bin/env node
// scripts/anim_proto/build.mjs —— 生成 preview/ 下的逐字节副本。
//
// 页面是单文件、无外部依赖；preview 只是同一份文件，供 raw.githack 评审。
// 这里故意不做任何注入/改写，便于 check.mjs 断言两份文件逐字节一致。

import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { createHash } from "node:crypto";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const SOURCE = join(HERE, "index.html");
const PREVIEW = join(HERE, "preview", "index.html");

const source = readFileSync(SOURCE);
const html = source.toString("utf8");
const stripped = html.replaceAll("http://www.w3.org/2000/svg", "");
if (/https?:\/\//i.test(stripped)) {
  console.error("[build] refusing to build: index.html contains an absolute http(s) URL");
  process.exit(1);
}
if (!/<meta name="viewport"/.test(html)) {
  console.error("[build] refusing to build: missing viewport meta");
  process.exit(1);
}
mkdirSync(dirname(PREVIEW), { recursive: true });
writeFileSync(PREVIEW, source);
const sha = createHash("sha256").update(source).digest("hex");
console.log(JSON.stringify({
  ok: true,
  source: SOURCE,
  preview: PREVIEW,
  bytes: source.length,
  sha256: sha,
  identical: readFileSync(PREVIEW).equals(source),
}, null, 2));

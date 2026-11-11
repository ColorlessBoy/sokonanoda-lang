// scripts/learn_demo/lib/verify-static.mjs — file-level and math invariants.
//
// The static layer answers: is the committed page exactly what the sources say,
// is it self-contained, does the content contract hold, and does the ε–N
// formula satisfy strictness for every slider value?
//
// It also runs the negative contract cases the brief asked for: a missing
// character or a missing question type must be rejected.

import { existsSync, readFileSync } from "node:fs";
import { createHash } from "node:crypto";
import { join } from "node:path";
import { ROOT, readSources, renderPage } from "./page.mjs";
import { validateContent, REQUIRED_TYPES } from "./contract.mjs";
import {
  TERMS,
  EPS_MIN,
  EPS_MAX,
  computeN,
  insideCount,
  termsFor,
  boundaryHolds,
  quantizeEpsilon,
} from "./math.mjs";

export function staticChecks({ root = ROOT } = {}) {
  const failures = [];
  let checks = 0;
  const check = (condition, label, extra) => {
    checks += 1;
    if (!condition) {
      failures.push(label + (extra === undefined ? "" : ` :: ${typeof extra === "string" ? extra : JSON.stringify(extra)}`));
    }
  };
  const info = {};

  const index = join(root, "index.html");
  const preview = join(root, "preview", "index.html");
  check(existsSync(index), `index.html exists at ${index}`);
  check(existsSync(preview), `preview/index.html exists at ${preview}`);
  if (!existsSync(index) || !existsSync(preview)) {
    return { ok: false, checks, failures, info };
  }

  const sources = readSources(root);
  const data = sources.content;
  const validation = validateContent(data);
  check(validation.ok, "content.json satisfies the demo contract", validation.errors.join(" | "));

  let expected = "";
  try {
    expected = renderPage(sources);
    check(true, "sources render without external URLs");
  } catch (error) {
    check(false, "sources render without external URLs", error.message);
  }
  const expectedBuf = Buffer.from(expected, "utf8");
  const indexBuf = readFileSync(index);
  const previewBuf = readFileSync(preview);
  info.bytes = indexBuf.length;
  info.sha256 = createHash("sha256").update(indexBuf).digest("hex");
  check(indexBuf.equals(expectedBuf), "committed index.html matches a fresh build");
  check(previewBuf.equals(indexBuf), "preview/index.html is byte-identical to index.html");
  info.identical = previewBuf.equals(indexBuf);

  const html = indexBuf.toString("utf8");
  const markup = html
    .replace(/<script\b[\s\S]*?<\/script>/gi, " ")
    .replace(/<style\b[\s\S]*?<\/style>/gi, " ");

  // ── document shape ─────────────────────────────────────────
  check(/^<!doctype html>/i.test(html.trimStart()), "starts with <!doctype html>");
  check(/<html lang="zh-CN">/.test(html), "lang is zh-CN");
  check(/<meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover">/.test(html), "responsive viewport meta");
  check(/<title>[^<]*ε–N[^<]*<\/title>/.test(html), "title names the ε–N concept");
  check(/<meta name="description"/.test(html), "meta description present");
  check(/<meta name="robots" content="noindex">/.test(html), "demo is noindex");
  check(/window\.SokoLearnDemo/.test(html), "exposes window.SokoLearnDemo");
  check(/prefers-reduced-motion/.test(html), "declares prefers-reduced-motion handling");
  check(/aria-live="polite"/.test(html), "has a polite live region");
  check((html.match(/role="status"/g) || []).length >= 2, "bubble + feedback are status regions");
  check(/id="scene-caption"/.test(html), "visible scene caption exists");
  check(/cast-card/.test(html), "cast card styles/markup exist");
  check(/dataset\.character/.test(html), "cast cards are wired through data-character");
  check(/type="range"/.test(html), "ε is controlled by a native range input");

  // ── self-contained: no network surface at all ──────────────
  check(!/https?:\/\//i.test(html), "no absolute http(s) URL anywhere");
  check(!/url\(\s*["']?(?:https?:|\/\/)/i.test(html), "no external CSS url()");
  check(!/@import\b/i.test(html), "no CSS @import");
  check(!/@font-face\b/i.test(html), "no external @font-face");
  check(!/<link\b/i.test(markup), "no <link> elements");
  check(!/<img\b/i.test(markup), "no <img> elements");
  check(!/<iframe\b|<video\b|<audio\b|<object\b|<embed\b|<source\b/i.test(markup), "no media/iframe elements");
  check(!/<script\b[^>]*\bsrc\s*=/i.test(html), "no external script src");
  check(!/\bfetch\s*\(/i.test(html), "does not call fetch()");
  check(!/\bXMLHttpRequest\b/i.test(html), "does not use XMLHttpRequest");
  check(!/\bWebSocket\b|\bEventSource\b/i.test(html), "does not open a socket");
  check(!/\bnew\s+Worker\b/i.test(html), "does not start a Worker");
  check(!/\bimport\s*\(/i.test(html), "does not use dynamic import()");

  // ── no free-form input / no code editor ────────────────────
  check(!/<textarea\b/i.test(markup), "no textarea");
  check(!/\bcontenteditable\b/i.test(markup), "no contenteditable");
  check(!/<form\b/i.test(markup), "no form submit path");
  check(!/<select\b/i.test(markup), "no select input");
  const inputs = [...markup.matchAll(/<input\b[^>]*>/gi)].map((match) => match[0]);
  check(inputs.length === 1, `one static <input> (got ${inputs.length})`);
  check(inputs.every((tag) => /type="range"/i.test(tag)), "the only static input is a range");
  check(!/type="(?:text|search|email|url|number|password)"/i.test(markup), "no free-form text input");

  // ── embedded content is the canonical content.json ────────
  const embeddedMatch = html.match(/<script type="application\/json" id="demo-data">([\s\S]*?)<\/script>/);
  check(Boolean(embeddedMatch), "content is embedded as application/json");
  let embedded = null;
  if (embeddedMatch) {
    try {
      embedded = JSON.parse(embeddedMatch[1]);
      check(true, "embedded content JSON parses");
    } catch (error) {
      check(false, "embedded content JSON parses", error.message);
    }
  }
  check(embedded !== null && JSON.stringify(embedded) === JSON.stringify(data), "embedded content matches content.json");
  check(data.characters.length >= 2, "at least two characters");
  check(new Set(data.characters.map((character) => character.id)).size === data.characters.length, "character ids are unique");
  check(new Set(data.characters.map((character) => character.name)).size === data.characters.length, "character names are distinct");
  check(new Set(data.characters.map((character) => character.avatar)).size === data.characters.length, "avatar visuals are distinct");
  check(data.questions.length === 5, "exactly five game questions");
  const types = data.questions.map((question) => question.type).sort();
  check(JSON.stringify(types) === JSON.stringify([...REQUIRED_TYPES].sort()), "all five required interaction types are present", types);
  check(data.badge.title === "你理解了 ε–N", "completion badge uses the exact required title");
  check(!data.characters.some((character) => Object.values(character.lines).some((line) => /[!！]/.test(line))), "avatar lines contain no hype/exclamation");

  // ── README is part of the deliverable ─────────────────────
  const readmePath = join(root, "README.md");
  check(existsSync(readmePath), "README.md exists");
  if (existsSync(readmePath)) {
    const readme = readFileSync(readmePath, "utf8");
    check(readme.includes("node scripts/learn_demo/check.mjs"), "README documents the check command");
    check(readme.includes("preview/index.html"), "README documents the preview copy");
  }

  // ── math invariants over every slider value ────────────────
  const sliderFailures = [];
  let monotonic = true;
  let previousN = Infinity;
  for (let k = 1; k <= 250; k += 1) {
    const epsilon = k / 1000;
    const expectedN = Math.floor(1000 / k) + 1;
    const N = computeN(epsilon);
    if (N !== expectedN) sliderFailures.push(`${epsilon}: N=${N} expected ${expectedN}`);
    if (!(1 / N < epsilon)) sliderFailures.push(`${epsilon}: 1/${N} is not < ε`);
    if (!(1 / (N - 1) >= epsilon - 1e-12)) sliderFailures.push(`${epsilon}: 1/${N - 1} is not >= ε`);
    if (!boundaryHolds(epsilon).holds) sliderFailures.push(`${epsilon}: boundaryHolds false`);
    if (N > previousN) monotonic = false;
    previousN = N;
    const terms = termsFor(epsilon);
    if (terms.length !== TERMS) sliderFailures.push(`${epsilon}: ${terms.length} terms`);
    const expectedInside = Math.max(0, TERMS - N + 1);
    if (insideCount(N) !== expectedInside) sliderFailures.push(`${epsilon}: insideCount mismatch`);
    for (const term of terms) {
      if (term.inTail !== term.n >= N) sliderFailures.push(`${epsilon}: tail mismatch n=${term.n}`);
      if (term.inside !== 1 / term.n < epsilon) sliderFailures.push(`${epsilon}: membership mismatch n=${term.n}`);
    }
  }
  check(sliderFailures.length === 0, "all 250 slider values satisfy strict N = ⌊1/ε⌋ + 1", sliderFailures.slice(0, 4).join(" | "));
  check(monotonic, "N is non-increasing as ε increases");

  const boundaries = [
    [0.25, 5],
    [0.22, 5],
    [0.1, 11],
    [0.052, 20],
    [0.001, 1001],
  ];
  for (const [epsilon, expected] of boundaries) {
    check(computeN(epsilon) === expected, `computeN(${epsilon}) = ${expected}`, computeN(epsilon));
  }
  check(computeN(0.25) === 5 && !(1 / 4 < 0.25), "ε = 0.25 boundary term a₄ is excluded");
  check(computeN(0.1) === 11 && !(1 / 10 < 0.1), "ε = 0.10 boundary term a₁₀ is excluded");
  check(computeN(0.22) === 5 && 1 / 5 < 0.22 && 1 / 4 >= 0.22, "ε = 0.22 lands at N = 5");
  check(computeN(0.001) === 1001 && insideCount(1001) === 0, "ε = 0.001 puts N outside the 20 drawn terms");

  let throws = 0;
  for (const bad of [0, -0.1, Number.NaN, Number.POSITIVE_INFINITY]) {
    try {
      computeN(bad);
    } catch {
      throws += 1;
    }
  }
  check(throws === 4, "computeN rejects 0, negative, NaN, and Infinity");
  check(quantizeEpsilon(0.22) === 0.22, "quantizeEpsilon keeps 0.22 exact");
  check(quantizeEpsilon(0.2499) === 0.25, "quantizeEpsilon rounds to thousandths");
  check(quantizeEpsilon(-1) === EPS_MIN && quantizeEpsilon(9) === EPS_MAX, "quantizeEpsilon clamps to [0.001, 0.25]");

  // ── negative contract cases ────────────────────────────────
  const clone = () => JSON.parse(JSON.stringify(data));
  const negatives = [
    ["missing character", (copy) => copy.characters.pop(), /characters/],
    ["one character only", (copy) => { copy.characters = copy.characters.slice(0, 1); }, /characters/],
    ["missing question", (copy) => copy.questions.splice(2, 1), /questions/],
    ["no questions", (copy) => { copy.questions = []; }, /questions/],
    ["wrong Q1 answer", (copy) => { copy.questions[0].answer = "n4"; }, /questions\[q1\]\.answer/],
    ["wrong Q3 dot", (copy) => { copy.questions[2].answer = 4; }, /questions\[q3\]\.answer/],
    ["wrong Q4 order", (copy) => { copy.questions[3].answer = [...copy.questions[3].answer].reverse(); }, /questions\[q4\]\.answer/],
    ["wrong Q5 N", (copy) => { copy.questions[4].answer = "n10"; }, /questions\[q5\]\.answer/],
    ["missing success line", (copy) => { delete copy.questions[0].successLines.ali; }, /successLines/],
    ["missing avatar line", (copy) => { delete copy.characters[0].lines.q3; }, /\.lines\.q3/],
  ];
  for (const [name, mutate, pattern] of negatives) {
    const copy = clone();
    mutate(copy);
    const result = validateContent(copy);
    check(
      !result.ok && result.errors.some((error) => pattern.test(error)),
      `negative contract: ${name} is rejected`,
      result.errors.slice(0, 3).join(" | ")
    );
  }
  check(validateContent(data).ok, "the real content still passes after negative cases");

  return { ok: failures.length === 0, checks, failures, info, data };
}

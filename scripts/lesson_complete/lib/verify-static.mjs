// scripts/lesson_complete/lib/verify-static.mjs — file-level, structure and
// math invariants.  The browser layer verifies what a learner actually sees.
//
// This file intentionally checks the committed artifact against a fresh build,
// the seven-screen structure, the exact copy from the brief, the absence of
// external/network surface, and 250+ ε values with an independent reference
// implementation (not a second call into the page's formatter).

import { existsSync, readFileSync } from "node:fs";
import { createHash } from "node:crypto";
import { createRequire } from "node:module";
import { join } from "node:path";
import { ROOT, readSources, renderPage, validateContent } from "./page.mjs";

const require = createRequire(import.meta.url);
const M = require("../math.js");

const FORBIDDEN_TERMS = [
  [/\bcast\b/i, "cast"],
  [/\bbible\b/i, "bible"],
  [/\bxp\b/i, "XP"],
  [/\bstreak\b/i, "streak"],
  [/\bdebug\b/i, "debug"],
  [/\bdashboard\b/i, "dashboard"],
  [/角色圣经/, "角色圣经"],
  [/控制台/, "控制台"],
  [/切换角色/, "切换角色"],
  [/积分/, "积分"],
  [/连对/, "连对"],
  [/完成度/, "完成度"],
];

function decodeEntities(text) {
  return text
    .replace(/&lt;/g, "<")
    .replace(/&gt;/g, ">")
    .replace(/&quot;/g, '"')
    .replace(/&#39;/g, "'")
    .replace(/&amp;/g, "&")
    .replace(/&nbsp;/g, " ");
}

function visibleText(markup) {
  return decodeEntities(
    markup
      .replace(/<[^>]*>/g, " ")
      .replace(/\s+/g, " ")
      .trim()
  );
}

function screenSections(markup) {
  const sections = [];
  const pattern = /<section\b[^>]*\bdata-screen="([^"]+)"[^>]*>([\s\S]*?)<\/section>/g;
  let match;
  while ((match = pattern.exec(markup))) {
    sections.push({ name: match[1], html: match[0], body: match[2] });
  }
  return sections;
}

function attr(tag, name) {
  const match = tag.match(new RegExp(`\\b${name}="([^"]*)"`));
  return match ? match[1] : null;
}

function count(html, pattern) {
  return (html.match(pattern) || []).length;
}

function referenceN(epsilon) {
  let n = Math.ceil(1 / epsilon);
  if (!(1 / n < epsilon)) n += 1;
  return n;
}

function referenceStrict(epsilon) {
  const N = referenceN(epsilon);
  const first = 1 / N;
  const previous = N > 1 ? 1 / (N - 1) : Infinity;
  return {
    N,
    firstOk: first < epsilon,
    previousOk: N === 1 || previous >= epsilon,
    holds: first < epsilon && (N === 1 || previous >= epsilon),
  };
}

export function staticChecks({ root = ROOT } = {}) {
  const failures = [];
  let checks = 0;
  const info = {};
  const check = (condition, label, extra) => {
    checks += 1;
    if (!condition) {
      failures.push(label + (extra === undefined ? "" : ` :: ${typeof extra === "string" ? extra : JSON.stringify(extra)}`));
    }
  };
  const fail = (label, error) => {
    checks += 1;
    failures.push(label + " :: " + error.message);
  };

  // ── files and build determinism ─────────────────────────────
  const indexPath = join(root, "index.html");
  const previewPath = join(root, "preview", "index.html");
  check(existsSync(indexPath), `index.html exists at ${indexPath}`);
  check(existsSync(previewPath), `preview/index.html exists at ${previewPath}`);
  check(existsSync(join(root, "README.md")), "README.md exists");
  check(existsSync(join(root, "check.mjs")), "check.mjs exists");
  check(existsSync(join(root, "build.mjs")), "build.mjs exists");
  for (const name of ["cover", "sequence", "band", "q1", "q2", "q3", "summary"]) {
    const shot = join(root, "screenshots", `${name}.png`);
    check(existsSync(shot), `screenshot ${name}.png is committed`);
    check(existsSync(shot) && readFileSync(shot).length > 5000, `screenshot ${name}.png is non-empty`);
  }
  if (!existsSync(indexPath) || !existsSync(previewPath)) {
    return { ok: false, checks, failures, info };
  }

  let sources = null;
  let fresh = "";
  try {
    sources = readSources(root);
    fresh = renderPage(sources);
    check(true, "sources render without external URLs");
  } catch (error) {
    fail("sources render without external URLs", error);
    return { ok: false, checks, failures, info };
  }

  const indexBuf = readFileSync(indexPath);
  const previewBuf = readFileSync(previewPath);
  const html = indexBuf.toString("utf8");
  const bytes = indexBuf.length;
  const sha = createHash("sha256").update(indexBuf).digest("hex");
  info.bytes = bytes;
  info.sha256 = sha;
  info.screens = 7;
  check(indexBuf.equals(Buffer.from(fresh, "utf8")), "committed index.html equals a fresh build");
  check(previewBuf.equals(indexBuf), "preview/index.html is byte-identical to index.html");
  info.identical = previewBuf.equals(indexBuf);

  // ── document shell ─────────────────────────────────────────
  check(/^<!doctype html>/i.test(html.trimStart()), "starts with <!doctype html>");
  check(/<html lang="zh-CN">/.test(html), "lang is zh-CN");
  check(/<meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover">/.test(html), "responsive viewport meta");
  check(/<title>[^<]*ε–N[^<]*<\/title>/.test(html), "title names the ε–N concept");
  check(/<meta name="description"/.test(html), "meta description present");
  check(/<meta name="robots" content="noindex">/.test(html), "lesson is noindex");
  check(/prefers-reduced-motion/.test(html), "declares prefers-reduced-motion handling");
  check((html.match(/aria-live="polite"/g) || []).length >= 7, "has polite live regions (feedback + hints + announcer)");
  check((html.match(/aria-atomic="true"/g) || []).length >= 4, "feedback and announcer are atomic live regions");
  check(/id="announcer"/.test(html), "page-level live announcer exists");

  const markup = html
    .replace(/<script\b[\s\S]*?<\/script>/gi, " ")
    .replace(/<style\b[\s\S]*?<\/style>/gi, " ");
  const text = visibleText(markup);
  const compactText = text.replace(/\s+/g, "");
  info.textLength = text.length;

  // ── exactly seven screens, one primary action each ─────────
  const sections = screenSections(markup);
  const names = sections.map((section) => section.name);
  check(
    JSON.stringify(names) === JSON.stringify(["cover", "sequence", "band", "q1", "q2", "q3", "summary"]),
    "exactly seven screens in linear order",
    names
  );
  check(sections.length === 7, "seven [data-screen] sections", sections.length);
  check((markup.match(/<section\b[^>]*data-screen=/g) || []).length === 7, "no extra data-screen sections");
  check(count(markup, /<h1\b/g) === 1, "exactly one h1 (the cover)");
  check(count(markup, /<h2\b/g) === 6, "six lesson-step h2 headings");
  check(count(markup, /tabindex="-1"/g) === 7, "all seven screen headings can take programmatic focus");
  const visibleSections = sections.filter((section) => !/\bhidden\b/.test(section.html.split(">")[0]));
  check(visibleSections.length === 1 && visibleSections[0].name === "cover", "cover is the only statically visible screen", visibleSections.map((s) => s.name));

  const primaryByScreen = {};
  for (const section of sections) {
    primaryByScreen[section.name] = count(section.html, /class="[^"]*\bbtn-primary\b[^"]*"/g);
  }
  check(
    Object.values(primaryByScreen).every((value) => value === 1),
    "every screen has exactly one primary action",
    primaryByScreen
  );

  const goto = [...markup.matchAll(/<button\b[^>]*\bdata-goto="([^"]+)"[^>]*>([\s\S]*?)<\/button>/g)].map((match) => ({
    target: match[1],
    tag: match[0].split(">")[0] + ">",
    label: visibleText(match[2]),
  }));
  check(
    JSON.stringify(goto.map((item) => item.target)) === JSON.stringify(["sequence", "band", "q1", "q2", "q3", "summary"]),
    "the forward path is cover → sequence → band → q1 → q2 → q3 → summary",
    goto.map((item) => item.target)
  );
  check(goto[0]?.label === "开始", "cover action is 开始", goto[0]);
  check(goto.slice(1).every((item) => item.label === "下一步"), "all forward actions are 下一步");
  for (const guard of ["q1", "q2", "q3"]) {
    const button = goto.find((item) => item.target === (guard === "q1" ? "q2" : guard === "q2" ? "q3" : "summary"));
    check(Boolean(button && /\bdata-guard="[^"]+"/.test(button.tag) && /\bdisabled\b/.test(button.tag)), `${guard} next starts disabled behind its guard`, button?.tag);
  }
  check(!/<a\b/i.test(markup), "no links at all (no next-lesson 404)");
  check(!/\bdata-(?:debug|dashboard|cast)\b/i.test(html), "no debug/dashboard/cast hooks");
  check(/下一课（占位）/.test(text), "summary names the disabled next lesson");
  check(/下一课还在准备中，这里先不放链接。/.test(text), "summary explains the placeholder is not a link");
  const placeholder = markup.match(/<button\b[^>]*disabled[^>]*>下一课（占位）<\/button>/);
  check(Boolean(placeholder), "next lesson is a disabled placeholder button");

  // ── progress ───────────────────────────────────────────────
  check(count(markup, /class="progress-seg"/g) === 6, "six progress segments");
  check(/id="progress"[^>]*hidden/.test(markup), "progress is hidden on the cover");
  check(/\.progress\[hidden\][\s\S]{0,80}?display:\s*none/.test(html), "the hidden progress element is display:none");
  check(/id="progress-label">第 1 \/ 6 步</.test(markup), "progress label starts at 第 1 / 6 步");
  check(/第 1 \/ 6 步/.test(html), "step counter is visible copy");
  check(!/\d+%/.test(text), "no completion percentage clutter");

  // ── ε slider and band readouts ─────────────────────────────
  const inputs = [...markup.matchAll(/<input\b[^>]*>/gi)].map((match) => match[0]);
  check(inputs.length === 1, `one static input (got ${inputs.length})`);
  check(inputs.every((tag) => /type="range"/i.test(tag)), "the only input is a range");
  check(/id="eps-range"[^>]*min="0.001"[^>]*max="0.25"[^>]*step="0.001"/.test(markup), "ε slider is 0.001–0.25 with 0.001 steps");
  check(/id="eps-range"[^>]*value="0.22"/.test(markup), "ε slider starts at 0.22");
  const presets = [...markup.matchAll(/class="preset"[^>]*data-eps="([^"]+)"/g)].map((match) => Number(match[1]));
  check(JSON.stringify(presets) === JSON.stringify([0.25, 0.22, 0.1, 0.05]), "four ε presets are 0.25 / 0.22 / 0.10 / 0.05", presets);
  check(/id="n-formula">N = ⌊1\/0\.220⌋ \+ 1 = 5</.test(markup), "band formula is a live N readout");
  check(/id="check-first"/.test(markup) && /id="check-prev"/.test(markup), "both strict boundary checks exist");
  check(/id="band-stage"[^>]*data-n="5"/.test(markup), "band scene starts at the correct N = 5");
  check(!/<textarea\b/i.test(markup), "no textarea");
  check(!/<form\b/i.test(markup), "no form submit path");
  check(!/<select\b/i.test(markup), "no select input");
  check(!/\bcontenteditable\b/i.test(markup), "no contenteditable");
  check(!/type="(?:text|search|email|url|number|password)"/i.test(markup), "no free-form text input");

  // ── questions, options, hints, feedback ────────────────────
  const expected = {
    q1: { choices: ["4", "5", "10"], prompt: "ε = 0.22 时，N 是多少？" },
    q2: { choices: ["true", "false"], prompt: "N = 4 对 ε = 0.22 够用吗？" },
    q3: { choices: ["10", "11", "21"], prompt: "ε = 0.10 时，N 是多少？" },
  };
  for (const [id, spec] of Object.entries(expected)) {
    const section = sections.find((item) => item.name === id);
    check(Boolean(section), `${id} screen exists`);
    if (!section) continue;
    const prompt = section.body.match(new RegExp(`id="${id}-prompt">([^<]+)<`));
    check(Boolean(prompt && decodeEntities(prompt[1]).trim() === spec.prompt), `${id} prompt is exact`, prompt?.[1]);
    const groupMatch = section.body.match(new RegExp(`<div class="options[^"]*"[^>]*data-options="${id}"[^>]*>([\\s\\S]*?)</div>`));
    check(Boolean(groupMatch), `${id} option group exists`);
    if (groupMatch) {
      const choices = [...groupMatch[1].matchAll(/data-choice="([^"]+)"/g)].map((match) => match[1]);
      check(JSON.stringify(choices) === JSON.stringify(spec.choices), `${id} choices are ${spec.choices.join(" / ")}`, choices);
      check(new RegExp(`aria-labelledby="${id}-prompt"`).test(groupMatch[0]), `${id} options are labelled by the question`);
    }
    check(count(section.body, new RegExp(`data-hint="${id}"`, "g")) === 1, `${id} has one optional hint button`);
    check(count(section.body, new RegExp(`id="${id}-hint"`, "g")) === 1, `${id} has one hint region`);
    check(new RegExp(`id="${id}-hint"[^>]*hidden`).test(section.body), `${id} hint is hidden until requested`);
    check(new RegExp(`id="${id}-hint"[^>]*aria-live="polite"`).test(section.body), `${id} hint is a live region`);
    check(count(section.body, new RegExp(`id="${id}-feedback"`, "g")) === 1, `${id} has one feedback region`);
    check(new RegExp(`id="${id}-feedback"[^>]*role="status"[^>]*aria-live="polite"`).test(section.body), `${id} feedback is an aria-live status`);
    check(new RegExp(`id="${id}-feedback"[^>]*aria-atomic="true"`).test(section.body), `${id} feedback is atomic`);
  }

  // ── exact copy from the brief ──────────────────────────────
  check(compactText.includes("ε–N：一条带子，一个起点"), "exact copy present: cover title");
  const exactCopy = [
    "3 分钟看懂：为什么 1/n 会无限接近 0，以及 N 到底怎么选。",
    "这些点越来越接近 0——但“越来越近”不是证明。怎么把“足够近”说死？",
    "你把允许误差 ε 定下来，N 就是“从此以后都在带内”的起点。",
    "你理解了 ε–N",
    "∀ε>0, ∃N, ∀n≥N, |aₙ−L|<ε",
    "1/n < ε ⇔ n > 1/ε",
    "N = ⌊1/ε⌋ + 1",
    "当 1/ε 正好是整数时",
    "a_N = ε",
    "ε = 0.10 → 1/ε = 10 → N = 11",
    "再学一遍",
  ];
  for (const copy of exactCopy) check(text.includes(copy), `exact copy present: ${copy.slice(0, 32)}`);

  // ── content contract + embedded JSON ───────────────────────
  const validation = validateContent(sources.content);
  check(Array.isArray(validation) && validation.length === 0, "shipping content has zero contract errors", validation);
  const embeddedMatch = html.match(/<script type="application\/json" id="lesson-data">([\s\S]*?)<\/script>/);
  check(Boolean(embeddedMatch), "content is embedded as application/json");
  if (embeddedMatch) {
    try {
      const embedded = JSON.parse(embeddedMatch[1]);
      check(JSON.stringify(embedded) === JSON.stringify(sources.content), "embedded content equals content.json");
    } catch (error) {
      fail("embedded content parses", error);
    }
  }

  const negatives = [];
  const negative = (mutator) => {
    const copy = structuredClone(sources.content);
    mutator(copy);
    return validateContent(copy).length > 0;
  };
  negatives.push([negative((content) => { delete content.questions[0].hint; }), "missing hint must be rejected"]);
  negatives.push([negative((content) => { content.questions[0].answer = 4; }), "wrong Q1 answer must be rejected"]);
  negatives.push([negative((content) => { content.questions[1].answer = true; }), "Q2=true must be rejected"]);
  negatives.push([negative((content) => { content.questions[2].answer = 10; }), "wrong Q3 answer must be rejected"]);
  negatives.push([negative((content) => { content.questions[2].choices = [10, 11]; }), "missing Q3 choice must be rejected"]);
  negatives.push([negative((content) => { content.questions = content.questions.slice(0, 2); }), "two questions must be rejected"]);
  for (const [ok, label] of negatives) check(ok, label);

  // ── math invariants: 250 milli values + extra values ───────
  let mathChecks = 0;
  let mathFailures = 0;
  const mathCheck = (condition, label) => {
    mathChecks += 1;
    if (!condition) {
      mathFailures += 1;
      failures.push("math :: " + label);
    }
  };
  for (let k = 1; k <= 250; k += 1) {
    const epsilon = k / 1000;
    const N = Math.floor(1000 / k) + 1;
    mathCheck(M.computeN(epsilon) === N, `computeN(${epsilon}) must be ${N}`);
    // Exact rational strictness: 1/N < k/1000 ⇔ 1000 < k*N.
    mathCheck(1000 < k * N, `1/N < ε for ε=${epsilon}`);
    mathCheck(N === 1 || 1000 >= k * (N - 1), `1/(N−1) ≥ ε for ε=${epsilon}`);
    const strict = M.strictChecks(epsilon);
    mathCheck(strict.N === N && strict.firstOk && strict.previousOk && strict.holds, `strictChecks holds for ε=${epsilon}`);
    mathCheck(M.insideVisibleCount(epsilon) === [4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]
      .filter((n) => 1 / n < epsilon).length, `visible inside count for ε=${epsilon}`);
  }
  const extras = [0.052, 0.1234, 0.0015, 0.249, 0.079, 0.0333, 0.2, 0.25, 0.1, 0.001, 0.0049, 0.111];
  for (const epsilon of extras) {
    const ref = referenceStrict(epsilon);
    const got = M.strictChecks(epsilon);
    mathCheck(got.N === ref.N, `computeN(${epsilon}) matches reference ${ref.N}`);
    mathCheck(got.firstOk === ref.firstOk && got.previousOk === ref.previousOk && got.holds, `strict boundary for ε=${epsilon}`);
    if (Number.isInteger(Math.round(epsilon * 1000)) && Math.abs(epsilon * 1000 - Math.round(epsilon * 1000)) < 1e-9) {
      mathCheck(M.computeN(epsilon) === Math.floor(1000 / Math.round(epsilon * 1000)) + 1, `milli path for ε=${epsilon}`);
    }
  }
  mathCheck(M.fmtEpsilon(0.22) === "0.220", "ε formatting uses three decimals");
  mathCheck(M.fmtTight(1 / 1001) === "0.000999", "1/1001 is readable at the boundary");
  mathCheck(M.fmtTight(1 / 5) === "0.2000", "1/5 is readable at the boundary");
  mathCheck(M.exactReciprocal(0.1) === 10 && M.exactReciprocal(0.22) === null, "integer recipient detection");
  mathCheck(M.computeN(0.22) === 5, "Q1: ε = 0.22 → N = 5");
  mathCheck(!(1 / 4 < 0.22), "Q2: N = 4 is not enough for ε = 0.22");
  mathCheck(M.computeN(0.1) === 11, "Q3: ε = 0.10 → N = 11");
  for (const bad of [0, -0.1, NaN, Infinity]) {
    let threw = false;
    try {
      M.computeN(bad);
    } catch {
      threw = true;
    }
    mathCheck(threw, `computeN rejects ${String(bad)}`);
  }
  const quantized = [0, 0.0001, 0.2509, 1, NaN].map((value) => M.quantizeEpsilon(value));
  mathCheck(JSON.stringify(quantized) === JSON.stringify([0.001, 0.001, 0.25, 0.25, 0.001]), "quantizeEpsilon clamps and quantizes");
  info.mathChecks = mathChecks;
  info.mathFailures = mathFailures;
  checks += mathChecks;

  // ── self-contained: zero network surface ──────────────────
  const withoutNamespace = html.replaceAll("http://www.w3.org/2000/svg", "");
  check(!/https?:\/\//i.test(withoutNamespace), "no absolute http(s) URL anywhere");
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
  check(!/\beval\s*\(/i.test(html), "does not use eval()");
  check(!/\bconsole\.(?:log|debug|warn)\b/i.test(html), "does not log to the console");

  // ── math source is shared, not duplicated ─────────────────
  const app = readFileSync(join(root, "app.js"), "utf8");
  check(/window\.LessonMath/.test(app) || /\bLessonMath\b/.test(app), "app uses the shared LessonMath module");
  check(/\bM\.strictChecks\b/.test(app) && /\bM\.insideVisibleCount\b/.test(app), "app asks the shared module for N, strictness and band membership");
  check(!/\bfunction\s+computeN\b/.test(app), "app does not duplicate computeN");
  check(/module\.exports\s*=\s*api/.test(readFileSync(join(root, "math.js"), "utf8")), "math.js exports to Node for checks");

  // ── forbidden dashboard/evidence strings ──────────────────
  for (const [pattern, label] of FORBIDDEN_TERMS) {
    check(!pattern.test(text), `visible copy does not expose ${label}`);
  }
  check(!/🏆|⭐|🔥|💎|🎮|🧠/.test(text), "no game/dashboard emoji");

  // ── one companion, one hint lineage ───────────────────────
  check(count(markup, /data-coach/g) === 5, "five coach slots (one per non-summary screen)");
  check(count(markup, /class="companion-avatar"/g) === 1, "one companion SVG template, not a cast");
  check(count(markup, /class="bubble-name">小析</g) === 5, "every bubble is spoken by 小析");
  check(!/阿理/.test(text), "阿理 is not a permanent visible character");

  info.checks = checks;
  return { ok: failures.length === 0, checks, failures, info };
}

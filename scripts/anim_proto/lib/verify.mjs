// scripts/anim_proto/lib/verify.mjs —— 静态检查 + 浏览器内验收。
//
// 判据分两层：
//   ① 文件层：没有外部引用、没有自由输入、正式定义/文案/公式都在；
//   ② 浏览器层：N 公式在边界值上成立、滑杆/按钮/键盘真的改变可见状态、
//      390/1440/320 无横向溢出、SVG 标签不互相压住、零外部请求/零页面错误。
//
// 这不是“看一眼”的验收：所有断言都落在 DOM/属性/像素布局上。

import { existsSync, readFileSync } from "node:fs";
import { createHash } from "node:crypto";
import {
  sleep,
  setViewport,
  clickSelector,
  pressKey,
  setReducedMotion,
} from "./cdp.mjs";

const FORMAL = "∀ ε > 0, ∃ N, ∀ n ≥ N, |1/n − 0| < ε";
const PLAIN = "不管误差带多小，总能找到起点 N，之后所有项都在带内。";
const WHY = "1/n < ε ⇔ n > 1/ε";

function decode(html) {
  return html
    .replace(/&lt;/g, "<")
    .replace(/&gt;/g, ">")
    .replace(/&amp;/g, "&")
    .replace(/&quot;/g, '"')
    .replace(/&#39;/g, "'")
    .replace(/&nbsp;/g, " ");
}

function visibleText(html) {
  return decode(
    html
      .replace(/<script\b[\s\S]*?<\/script>/gi, " ")
      .replace(/<style\b[\s\S]*?<\/style>/gi, " ")
      .replace(/<[^>]+>/g, " ")
  )
    .replace(/\s+/g, " ")
    .replace(/\s+([，。、；：！？,.;:!?])/g, "$1")
    .trim();
}

function sha(buffer) {
  return createHash("sha256").update(buffer).digest("hex").slice(0, 16);
}

// ── 文件层 ───────────────────────────────────────────────────
export function staticChecks({ htmlPath, previewPath }) {
  const failures = [];
  const info = {};
  const note = (condition, message) => {
    if (!condition) failures.push(message);
  };

  if (!existsSync(htmlPath)) {
    failures.push(`index.html missing at ${htmlPath}`);
    return { ok: false, failures, info };
  }
  const source = readFileSync(htmlPath);
  const html = source.toString("utf8");
  info.bytes = source.length;
  info.sha256 = sha(source);

  note(/^<!doctype html>/i.test(html.trimStart()), "index.html starts with <!doctype html>");
  note(/<html lang="zh-CN">/.test(html), 'lang is zh-CN');
  note(/<meta name="viewport" content="width=device-width, initial-scale=1">/.test(html), "has a responsive viewport meta");
  note(/prefers-reduced-motion/.test(html), "declares prefers-reduced-motion handling");
  note(/window\.SokoAnim/.test(html), "exposes window.SokoAnim");
  note(/SokoAnim\.seek|function seek/.test(html), "exposes deterministic seek");

  // 外部引用：唯一的白名单是 SVG 命名空间；它是元素命名空间，不是网络请求。
  const stripped = html.replaceAll("http://www.w3.org/2000/svg", "");
  const externalPatterns = [
    [/https?:\/\//i, "contains an absolute http(s) URL"],
    [/<(?:script|img|link|iframe|source|video|audio|object|embed)\b[^>]*\b(?:src|href)\s*=\s*["'](?:https?:)?\/\//i, "loads an external resource"],
    [/url\(\s*["']?(?:https?:|\/\/)/i, "CSS references an external URL"],
    [/@import\b/i, "uses CSS @import"],
    [/\bfetch\s*\(/i, "calls fetch()"],
    [/\bXMLHttpRequest\b/i, "uses XMLHttpRequest"],
    [/\bWebSocket\b/i, "uses WebSocket"],
    [/\bEventSource\b/i, "uses EventSource"],
    [/\bnew\s+Worker\b/i, "starts a Worker"],
    [/\bimport\s*\(/i, "uses dynamic import()"],
  ];
  for (const [pattern, message] of externalPatterns) note(!pattern.test(stripped), `no external refs: ${message}`);

  // 不允许自由输入；只有一个 range 控件。
  note(!/<textarea\b/i.test(html), "no textarea (no free-form input)");
  note(!/\bcontenteditable\b/i.test(html), "no contenteditable (no free-form input)");
  const inputs = [...html.matchAll(/<input\b[^>]*>/gi)].map((match) => match[0]);
  note(inputs.length === 1, `exactly one <input> (got ${inputs.length})`);
  note(inputs.length === 1 && /type="range"/i.test(inputs[0]), "the only input is type=range");
  note(inputs.length === 1 && /aria-label=/.test(inputs[0]), "the range has an aria-label");

  const text = visibleText(html);
  const squash = (value) => value.replace(/\s+/g, "");
  note(text.includes(FORMAL), `formal definition text is present: ${FORMAL}`);
  note(squash(text).includes(squash(PLAIN)), `plain-language caption is present: ${PLAIN}`);
  note(text.includes(WHY), `why line is present: ${WHY}`);
  note(text.includes("不管误差带多小"), "plain caption explains arbitrary small epsilon");
  note(text.includes("N = ⌊1/ε⌋ + 1"), "shows the strict N formula");
  note(text.includes("N = ⌈1/ε⌉"), "mentions the textbook ceiling shorthand");
  note(text.includes("起点 N"), "names the starting point N");
  note(text.includes("1/N") && text.includes("1/(N−1)"), "shows the two-sided minimality evidence");
  note(/aria-valuetext/.test(html), "range exposes aria-valuetext");
  note(/aria-live="polite"/.test(html), "has a polite live region");
  note(/tabindex|button/.test(html), "controls are native buttons");
  note(!/<form\b/i.test(html), "no form submission");

  if (!existsSync(previewPath)) {
    failures.push(`preview/index.html missing at ${previewPath}; run node scripts/anim_proto/build.mjs`);
  } else {
    const preview = readFileSync(previewPath);
    info.previewBytes = preview.length;
    info.previewSha256 = sha(preview);
    note(preview.equals(source), "preview/index.html is a byte-identical generated copy");
  }

  return { ok: failures.length === 0, failures, info };
}

// ── 浏览器层 ─────────────────────────────────────────────────
function fmt(value, digits = 4) {
  return Number(value).toFixed(digits);
}

function fmtTight(value) {
  if (!Number.isFinite(value)) return "∞";
  if (value !== 0 && Math.abs(value) < 0.01) return Number(value).toFixed(6);
  return Number(value).toFixed(4);
}

function same(a, b, tolerance = 1e-9) {
  return Math.abs(a - b) <= tolerance;
}

function overlaps(a, b) {
  return !(a.right <= b.left + 0.5 || b.right <= a.left + 0.5 || a.bottom <= b.top + 0.5 || b.bottom <= a.top + 0.5);
}

export async function browserChecks({ client }) {
  const failures = [];
  let checks = 0;
  const details = {};
  const CLIPPED_SELECTORS = [
    ".caption-text", ".formal", ".plain", ".why-note", ".why-caveat",
    ".readout strong", ".btn", ".preset", ".band-key .long", ".strict-flag",
    ".viz-label", ".viz-label .note", ".keys", ".foot span",
    ".side-note-list", ".side-note-foot", ".side-note-head",
  ];
  const check = (condition, message) => {
    checks += 1;
    if (!condition) failures.push(message);
    return condition;
  };

  const evalJSON = (expression) => client.evaluateJSON(expression);
  const stateNow = () => evalJSON("window.SokoAnim.getState()");
  const setEps = async (eps) => {
    await client.evaluate(`document.activeElement && document.activeElement.blur(); window.SokoAnim.setEpsilon(${JSON.stringify(eps)});`);
    await sleep(40);
  };
  const reset = async () => {
    await client.evaluate("document.activeElement && document.activeElement.blur(); window.SokoAnim.reset();");
    await sleep(40);
  };

  await sleep(120);

  // 0. API / 静态 DOM
  const initial = await evalJSON(`(() => {
    const q = (s) => document.querySelector(s);
    const n = (s) => document.querySelectorAll(s).length;
    return {
      version: window.SokoAnim.version,
      terms: window.SokoAnim.terms,
      duration: window.SokoAnim.duration,
      presets: window.SokoAnim.presets,
      state: window.SokoAnim.getState(),
      counts: {
        context: n('#contextSvg [data-term]'),
        detail: n('#detailSvg [data-term]'),
        index: n('#indexSvg [data-index-term]'),
        range: n('#epsRange'),
        textInputs: n('textarea, input:not([type="range"]), [contenteditable="true"]'),
        buttons: n('button'),
      },
      formal: q('.formal') ? q('.formal').textContent.replace(/\\s+/g, ' ').trim() : '',
      plain: q('.plain') ? q('.plain').textContent.replace(/\\s+/g, ' ').trim() : '',
      why: q('.why') ? q('.why').textContent.replace(/\\s+/g, ' ').trim() : '',
      caveat: q('.why-caveat') ? q('.why-caveat').textContent.replace(/\\s+/g, ' ').trim() : '',
      caption: q('#captionText') ? q('#captionText').textContent.replace(/\\s+/g, ' ').trim() : '',
      rangeType: q('#epsRange') ? q('#epsRange').type : '',
      rangeMin: q('#epsRange') ? q('#epsRange').min : '',
      rangeMax: q('#epsRange') ? q('#epsRange').max : '',
    };
  })()`);

  check(initial.version === "1.0.0", "SokoAnim API version is 1.0.0");
  check(initial.terms === 20, "SokoAnim.terms is 20");
  check(initial.duration >= 5, "animation duration is at least 5s");
  check(initial.presets.length === 4, "four epsilon presets are exposed");
  check(initial.counts.context === 20, `context number line shows 20 terms (got ${initial.counts.context})`);
  check(initial.counts.detail === 17, `detail zoom shows n=4..20 (17 terms, got ${initial.counts.detail})`);
  check(initial.counts.index === 20, `index ribbon shows 20 terms (got ${initial.counts.index})`);
  check(initial.counts.range === 1, "exactly one range slider");
  check(initial.counts.textInputs === 0, "no free-form text input / contenteditable");
  check(initial.counts.buttons >= 4, "at least four control buttons");
  check(initial.rangeType === "range", "slider is a native range input");
  check(initial.rangeMin === "0" && initial.rangeMax === "1000", "slider uses a 0..1000 log position");
  check(initial.formal.includes(FORMAL), `formal definition reads ${FORMAL}`);
  check(initial.plain.includes(PLAIN), `plain-language caption reads ${PLAIN}`);
  check(initial.why.includes(WHY), "why card shows 1/n < ε ⇔ n > 1/ε");
  check(initial.why.includes("N = ⌊1/ε⌋ + 1"), "why card shows N = floor(1/ε)+1");
  check(initial.caveat.includes("N = ⌈1/ε⌉"), "why card mentions N = ceil(1/ε)");
  check(initial.caveat.includes("严格小于"), "why card explains the strict-inequality correction");
  check(initial.caption.length > 20, "caption is populated on load");

  // 1. N 公式：严格小于 ⇒ 第一个 > 1/ε 的整数。
  const cases = [0.001, 0.002, 0.005, 0.01, 0.02, 0.05, 0.052, 0.07, 0.1, 0.125, 0.2, 0.21, 0.2499, 0.25, 0.2501, 0.3, 0.0999, 0.1001, 1 / 3];
  const formula = await evalJSON(
    `(${JSON.stringify(cases)}).map((eps) => {
      const N = window.SokoAnim.computeN(eps);
      const inv = 1 / eps;
      const nearest = Math.round(inv);
      return {
        eps, N, inv, invN: 1 / N,
        prevInv: N > 1 ? 1 / (N - 1) : null,
        isInteger: Math.abs(inv - nearest) <= 1e-9,
        ceil: Math.ceil(inv),
        floorPlusOne: Math.floor(inv + 1e-9) + 1,
        exactFlag: window.SokoAnim.isIntegerReciprocal(eps),
        inside: window.SokoAnim.insideCountFor(N)
      };
    })`
  );
  let formulaPasses = 0;
  for (const item of formula) {
    const label = `ε=${item.eps}`;
    const okFormula = check(item.N === item.floorPlusOne, `${label}: N = floor(1/ε)+1 (got ${item.N}, expected ${item.floorPlusOne})`);
    const okStrict = check(item.invN < item.eps + 1e-12, `${label}: 1/N < ε (1/${item.N}=${item.invN} vs ${item.eps})`);
    const okMinimal = check(item.prevInv === null || item.prevInv >= item.eps - 1e-12, `${label}: 1/(N−1) ≥ ε (minimal N)`);
    const okInteger = check(item.exactFlag === item.isInteger, `${label}: integer-reciprocal flag matches`);
    const okShorthand = item.isInteger
      ? check(item.N === item.ceil + 1, `${label}: exact reciprocal needs ceil+1 (got ${item.N}, ceil=${item.ceil})`)
      : check(item.N === item.ceil, `${label}: non-integer reciprocal matches ceil(1/ε) (got ${item.N}, ceil=${item.ceil})`);
    const okInside = check(item.inside === Math.max(0, 21 - item.N), `${label}: in-band count formula`);
    if (okFormula && okStrict && okMinimal && okInteger && okShorthand && okInside) formulaPasses += 1;
  }
  details.formulaCases = formula.length;
  details.formulaPasses = formulaPasses;
  check(formulaPasses === formula.length, "all N formula cases pass");

  // 2. 滑杆 / 读数 / 带 / 输入状态
  const states = [
    { eps: 0.25, N: 5, inside: 16, exact: true, preset: "0.25" },
    { eps: 0.1, N: 11, inside: 10, exact: true, preset: "0.10" },
    { eps: 0.052, N: 20, inside: 1, exact: false, preset: "0.052" },
    { eps: 0.01, N: 101, inside: 0, exact: true, preset: "0.01" },
    { eps: 0.001, N: 1001, inside: 0, exact: true, preset: null },
    { eps: 0.21, N: 5, inside: 16, exact: false, preset: null },
  ];
  const stateReports = [];
  for (const expected of states) {
    await setEps(expected.eps);
    const report = await evalJSON(`(() => {
      const q = (s) => document.querySelector(s);
      const n = (s) => document.querySelectorAll(s).length;
      const num = (s, attr) => parseFloat(q(s).getAttribute(attr));
      const state = window.SokoAnim.getState();
      const range = q('#epsRange');
      const sym = q('#symBand');
      const track = q('.sym-track');
      return {
        state,
        epsOut: q('#epsOut').textContent.trim(),
        nOut: q('#nOut').textContent.trim(),
        insideOut: q('#insideOut').textContent.trim(),
        caption: q('#captionText').textContent.replace(/\\s+/g, ' ').trim(),
        captionMath: q('#captionMath').textContent.replace(/\\s+/g, ' ').trim(),
        phase: q('#phaseLabel').textContent.replace(/\\s+/g, ' ').trim(),
        whyHead: q('#whyHead').textContent.trim(),
        whyValue: q('#whyValue').textContent.trim(),
        whyNote: q('#whyNote').textContent.replace(/\\s+/g, ' ').trim(),
        invOut: q('#invOut').textContent.trim(),
        invNOut: q('#invNOut').textContent.trim(),
        verdict: q('#verdictOut').textContent.trim(),
        strictFlag: q('#strictFlag').textContent.trim(),
        strictHot: q('#strictFlag').classList.contains('is-hot'),
        contextInside: n('#contextSvg .dot-in'),
        detailInside: n('#detailSvg .dot-in'),
        detailOutside: n('#detailSvg .dot-out'),
        indexInside: n('#indexSvg .index-cell-in'),
        indexOutside: n('#indexSvg .index-cell-out'),
        bandWidth: num('#detailSvg .band-fill', 'width'),
        edgeX: num('#detailSvg .band-outline', 'x2'),
        nX: num('#detailSvg .n-line', 'x1'),
        nPill: q('#detailSvg .n-pill-text').textContent.trim(),
        ribbon: q('#ribbonNote').textContent.trim(),
        activePresets: [...document.querySelectorAll('.preset.is-active')].map((b) => b.textContent.trim()),
        sliderValue: Number(range.value),
        ariaValueText: range.getAttribute('aria-valuetext'),
        symWidth: parseFloat(getComputedStyle(sym).width),
        trackWidth: track.getBoundingClientRect().width,
        visibleDetailLabels: [...document.querySelectorAll('#detailSvg .dot-label')].filter((el) => getComputedStyle(el).opacity > 0.05).length,
        rangeFocused: document.activeElement === range
      };
    })()`);
    stateReports.push({ expected, report });

    const { state } = report;
    const label = `ε=${expected.eps}`;
    check(same(state.epsilon, expected.eps), `${label}: state epsilon is ${expected.eps} (got ${state.epsilon})`);
    check(state.N === expected.N, `${label}: N is ${expected.N} (got ${state.N})`);
    check(state.inside === expected.inside, `${label}: in-band count is ${expected.inside} (got ${state.inside})`);
    check(state.exactReciprocal === expected.exact, `${label}: exact reciprocal flag`);
    check(report.epsOut === fmt(expected.eps), `${label}: ε readout is ${fmt(expected.eps)}`);
    check(report.nOut === String(expected.N), `${label}: N readout is ${expected.N}`);
    check(report.insideOut === String(expected.inside), `${label}: in-band readout is ${expected.inside}`);
    check(report.contextInside === expected.inside, `${label}: context line colors ${expected.inside} terms in-band (got ${report.contextInside})`);
    check(report.detailInside === expected.inside, `${label}: detail line colors ${expected.inside} terms in-band (got ${report.detailInside})`);
    check(report.indexInside === expected.inside, `${label}: index ribbon colors ${expected.inside} cells in-band (got ${report.indexInside})`);
    check(report.indexInside + report.indexOutside === 20, `${label}: index ribbon accounts for 20 terms`);
    check(report.detailInside + report.detailOutside === 17, `${label}: detail line accounts for 17 terms`);
    check(report.bandWidth > 0, `${label}: band has a positive width`);
    check(report.nX < report.edgeX, `${label}: N threshold 1/N is left of the band edge ε`);
    check(report.nPill.includes(`N=${expected.N}`), `${label}: N pill reads N=${expected.N}`);
    check(report.whyHead === `N = ${expected.N}`, `${label}: why header is N = ${expected.N}`);
    check(report.whyValue === String(expected.N), `${label}: why value is ${expected.N}`);
    check(report.invOut === fmt(1 / expected.eps), `${label}: 1/ε readout`);
    check(report.invNOut === fmtTight(1 / expected.N), `${label}: 1/N readout`);
    check(report.verdict.includes("≥"), `${label}: minimality verdict uses ≥`);
    check(report.verdict.includes(fmt(expected.eps)), `${label}: minimality verdict compares to ε`);
    check(report.strictHot === expected.exact, `${label}: strict-boundary flag is hot exactly at integer 1/ε`);
    check(expected.exact ? report.strictFlag.includes("不在带内") : report.strictFlag.includes("严格小于"), `${label}: strict-boundary flag text`);
    check(expected.exact ? report.whyNote.includes("是整数") : report.whyNote.includes("不是整数"), `${label}: why note explains integer vs non-integer reciprocal`);
    check(report.caption.includes(`N = ${expected.N}`), `${label}: caption names N`);
    check(report.caption.includes(`ε = ${fmt(expected.eps)}`), `${label}: caption names ε`);
    check(report.captionMath.includes("< ε"), `${label}: minimality line shows 1/N < ε`);
    if (expected.N > 500) {
      check(report.captionMath.includes(fmtTight(1 / expected.N)), `${label}: 1/N keeps enough decimals to be strictly below ε`);
    }
    if (expected.N > 20) {
      check(report.caption.includes("画面外"), `${label}: caption explains off-screen tail for N > 20`);
    }
    if (expected.exact) {
      check(report.caption.includes("是整数"), `${label}: caption explains the integer-reciprocal boundary`);
    }
    check(report.phase.includes("手动"), `${label}: manual state is labelled 手动`);
    check(report.ariaValueText.includes(`N = ${expected.N}`), `${label}: aria-valuetext announces N`);
    check(report.ariaValueText.includes(fmt(expected.eps)), `${label}: aria-valuetext announces ε`);
    const expectedSlider = Math.round((Math.log(expected.eps / 0.001) / Math.log(0.25 / 0.001)) * 1000);
    check(Math.abs(report.sliderValue - expectedSlider) <= 1, `${label}: slider position tracks ε (got ${report.sliderValue}, expected ${expectedSlider})`);
    const expectedSym = expected.eps / 0.25;
    check(Math.abs(report.symWidth / report.trackWidth - expectedSym) < 0.03, `${label}: symmetric gauge width tracks ε`);
    if (expected.preset) {
      check(report.activePresets.includes(expected.preset), `${label}: preset ${expected.preset} is active`);
    } else {
      check(report.activePresets.length === 0, `${label}: no preset is active for a non-preset ε`);
    }
    details[`state_${expected.eps}`] = { N: state.N, inside: state.inside, exact: state.exactReciprocal };
  }
  details.states = stateReports.length;

  // 3. 用户动作：单步循环
  await reset();
  const stepNs = [];
  for (let i = 0; i < 4; i += 1) {
    await clickSelector(client, "#stepBtn");
    stepNs.push((await stateNow()).N);
  }
  check(JSON.stringify(stepNs) === JSON.stringify([11, 20, 101, 5]), `Step cycles 5 → 11 → 20 → 101 → 5 (got ${stepNs.join(" → ")})`);
  details.stepCycle = stepNs;

  // 4. 用户动作：播放 / 暂停
  await reset();
  await clickSelector(client, "#playBtn");
  await sleep(180);
  const playing = await stateNow();
  check(playing.playing === true, "clicking Play starts the animation");
  check(playing.mode === "script", "playing state uses the script mode");
  await sleep(900);
  const advanced = await stateNow();
  check(advanced.epsilon < 0.25 - 1e-9, `Play advances ε after the initial hold (got ${advanced.epsilon})`);
  const playPressed = await client.evaluate("document.getElementById('playBtn').getAttribute('aria-pressed')");
  check(playPressed === "true", "Play button exposes aria-pressed=true while playing");
  await clickSelector(client, "#playBtn");
  const paused = await stateNow();
  check(paused.playing === false, "clicking Pause stops the animation");
  await reset();

  // 5. 用户动作：全局键盘快捷键
  await client.evaluate("document.activeElement && document.activeElement.blur(); window.SokoAnim.reset();");
  await pressKey(client, { key: "s", code: "KeyS", windowsVirtualKeyCode: 83 });
  const afterS = await stateNow();
  check(afterS.N === 11, `key S steps to ε=0.10 (N=11), got N=${afterS.N}`);
  await pressKey(client, { key: "0", code: "Digit0", windowsVirtualKeyCode: 48 });
  const after0 = await stateNow();
  check(same(after0.epsilon, 0.25), `key 0 resets ε to 0.25 (got ${after0.epsilon})`);

  // 6. 用户动作：滑杆键盘（Home/End + 方向键）
  await clickSelector(client, "#epsRange");
  await pressKey(client, { key: "Home", code: "Home", windowsVirtualKeyCode: 36 });
  await sleep(80);
  const afterHome = await stateNow();
  check(same(afterHome.epsilon, 0.001), `Home moves the slider to ε=0.001 (got ${afterHome.epsilon})`);
  check(afterHome.N === 1001, `Home state has N=1001 (got ${afterHome.N})`);
  const focusAfterHome = await client.evaluate("document.activeElement && document.activeElement.id");
  check(focusAfterHome === "epsRange", "slider keeps keyboard focus after Home");
  await pressKey(client, { key: "ArrowRight", code: "ArrowRight", windowsVirtualKeyCode: 39 });
  await sleep(80);
  const afterRight = await stateNow();
  check(afterRight.epsilon > 0.001, `ArrowRight increases ε at the minimum (got ${afterRight.epsilon})`);
  check(afterRight.N < 1001, `ArrowRight reduces N at the minimum (got ${afterRight.N})`);
  await pressKey(client, { key: "End", code: "End", windowsVirtualKeyCode: 35 });
  await sleep(80);
  const afterEnd = await stateNow();
  check(same(afterEnd.epsilon, 0.25), `End moves the slider to ε=0.25 (got ${afterEnd.epsilon})`);
  await reset();

  // 7. 减少动态偏好：直接落到最终状态
  await setReducedMotion(client, true);
  await reset();
  await clickSelector(client, "#playBtn");
  await sleep(160);
  const reduced = await evalJSON(`(() => ({
    state: window.SokoAnim.getState(),
    phase: document.getElementById('phaseLabel').textContent.replace(/\\s+/g, ' ').trim(),
    playLabel: document.getElementById('playLabel').textContent.trim()
  }))()`);
  check(reduced.state.reducedMotion === true, "reduced-motion media query is observed");
  check(same(reduced.state.epsilon, 0.052), `reduced motion jumps straight to ε=0.052 (got ${reduced.state.epsilon})`);
  check(reduced.state.playing === false, "reduced motion does not run a tween");
  check(reduced.phase.includes("静态"), "reduced-motion state is labelled 静态");
  check(reduced.playLabel === "播放", "Play button returns to 播放 after the instant state");
  await setReducedMotion(client, false);
  await reset();

  // 8. seek() 确定性 + 单调性
  await client.evaluate("window.SokoAnim.beginCapture()");
  const seeks = await evalJSON(`(() => {
    const out = [];
    for (let i = 0; i <= 30; i += 1) {
      const t = (window.SokoAnim.duration * i) / 30;
      const s = window.SokoAnim.seek(t);
      out.push({ t, epsilon: s.epsilon, N: s.N, inside: s.inside, u: s.u });
    }
    return out;
  })()`);
  let monotonic = true;
  for (let i = 1; i < seeks.length; i += 1) {
    if (seeks[i].epsilon > seeks[i - 1].epsilon + 1e-9) monotonic = false;
    if (seeks[i].N < seeks[i - 1].N) monotonic = false;
  }
  check(monotonic, "seek(t) keeps ε non-increasing and N non-decreasing");
  check(same(seeks[0].epsilon, 0.25), "seek(0) is ε=0.25");
  const midSeek = seeks[Math.round(seeks.length / 2)];
  check(midSeek.epsilon > 0.052 && midSeek.epsilon < 0.25, "seek(mid) is between the endpoints");
  const endSeek = await client.evaluate("window.SokoAnim.seek(window.SokoAnim.duration)");
  check(same(endSeek.epsilon, 0.052), "seek(duration) is ε=0.052");
  check(endSeek.N === 20, "seek(duration) has N=20");
  await client.evaluate("window.SokoAnim.endCapture()");
  const captureClass = await client.evaluate("document.documentElement.classList.contains('capture')");
  check(captureClass === false, "endCapture removes the capture class");
  await reset();

  // 9. 布局：390 / 1440 / 320
  const viewports = [
    { name: "mobile 390", width: 390, height: 844, mobile: true, stacked: true },
    { name: "desktop 1440", width: 1440, height: 1100, mobile: false, stacked: false },
    { name: "narrow 320", width: 320, height: 700, mobile: true, stacked: true },
  ];
  details.layout = {};
  for (const viewport of viewports) {
    await setViewport(client, { width: viewport.width, height: viewport.height, dpr: 1, mobile: viewport.mobile });
    await client.evaluate("window.scrollTo(0, 0)");
    await sleep(180);
    const layout = await evalJSON(`(() => {
      const rect = (el) => {
        const b = el.getBoundingClientRect();
        return { left: +b.left.toFixed(1), top: +b.top.toFixed(1), right: +b.right.toFixed(1), bottom: +b.bottom.toFixed(1), width: +b.width.toFixed(1), height: +b.height.toFixed(1) };
      };
      const q = (s) => document.querySelector(s);
      const offenders = [];
      for (const el of document.querySelectorAll("body *")) {
        if (el.closest("svg")) continue;
        if (el.matches(".sr-only, .skip")) continue;
        const b = el.getBoundingClientRect();
        if (b.width <= 0 || b.height <= 0) continue;
        if (b.right > innerWidth + 1 || b.left < -1) {
          offenders.push({ node: (typeof el.className === "string" && el.className ? el.className.split(/\\s+/)[0] : el.tagName.toLowerCase()), left: +b.left.toFixed(1), right: +b.right.toFixed(1) });
          if (offenders.length >= 8) break;
        }
      }
      const clipped = [];
      for (const sel of ${JSON.stringify(CLIPPED_SELECTORS)}) {
        for (const el of document.querySelectorAll(sel)) {
          if (el.clientWidth > 0 && el.scrollWidth > el.clientWidth + 1) {
            clipped.push({ sel, text: (el.textContent || "").trim().slice(0, 36), scrollWidth: el.scrollWidth, clientWidth: el.clientWidth });
          }
        }
      }
      const labelRects = (root) => [...document.querySelectorAll(root + " .dot-label")].filter((el) => getComputedStyle(el).opacity > 0.05).map((el) => ({ text: el.textContent, ...rect(el) }));
      const overlapPairs = (a, b) => {
        const out = [];
        for (let i = 0; i < a.length; i += 1) for (let j = i + 1; j < a.length; j += 1) {
          const x = a[i], y = a[j];
          if (!(x.right <= y.left + 0.5 || y.right <= x.left + 0.5 || x.bottom <= y.top + 0.5 || y.bottom <= x.top + 0.5)) out.push([x.text, y.text]);
        }
        return out;
      };
      const labels = labelRects("#detailSvg");
      const pill = rect(q("#detailSvg .n-pill-text"));
      const edge = rect(q("#detailSvg .band-label"));
      const idxFlag = rect(q("#indexSvg .ribbon-n-text"));
      const idxLabels = [...document.querySelectorAll("#indexSvg .index-label")].map((el) => ({ text: el.textContent, ...rect(el) }));
      return {
        viewport: [innerWidth, innerHeight],
        document: [document.documentElement.scrollWidth, document.documentElement.scrollHeight],
        overflow: document.documentElement.scrollWidth - innerWidth,
        offenders,
        clipped,
        stage: rect(q("#stageCard")),
        definition: rect(q(".definition-card")),
        context: rect(q("#contextSvg")),
        detail: rect(q("#detailSvg")),
        index: rect(q("#indexSvg")),
        caption: rect(q(".caption")),
        controls: rect(q(".controls")),
        labels: labels.length,
        labelPairs: overlapPairs(labels),
        pillEdgeOverlap: !(pill.right <= edge.left + 0.5 || edge.right <= pill.left + 0.5 || pill.bottom <= edge.top + 0.5 || edge.bottom <= pill.top + 0.5),
        indexPairs: overlapPairs(idxLabels),
        indexFlagOverlaps: idxLabels.filter((label) => !(label.right <= idxFlag.left + 0.5 || idxFlag.right <= label.left + 0.5 || label.bottom <= idxFlag.top + 0.5 || idxFlag.bottom <= label.top + 0.5)).map((label) => label.text),
        phase: q("#phaseLabel").textContent.replace(/\\s+/g, " ").trim(),
      };
    })()`);
    details.layout[viewport.name] = layout;
    check(layout.overflow <= 1, `${viewport.name}: no document horizontal overflow (${layout.overflow}px)`);
    check(layout.offenders.length === 0, `${viewport.name}: no element crosses the viewport (${layout.offenders.map((o) => o.node).join(", ") || "none"})`);
    check(layout.clipped.length === 0, `${viewport.name}: no key text is horizontally clipped (${layout.clipped.map((c) => c.sel).join(", ") || "none"})`);
    check(layout.stage.right <= viewport.width + 1 && layout.stage.left >= -1, `${viewport.name}: stage card stays in the viewport`);
    check(layout.definition.right <= viewport.width + 1 && layout.definition.left >= -1, `${viewport.name}: definition card stays in the viewport`);
    check(layout.context.right <= viewport.width + 1 && layout.detail.right <= viewport.width + 1 && layout.index.right <= viewport.width + 1, `${viewport.name}: SVG canvases stay in the viewport`);
    check(layout.labelPairs.length === 0, `${viewport.name}: detail labels do not overlap (${JSON.stringify(layout.labelPairs)})`);
    check(layout.pillEdgeOverlap === false, `${viewport.name}: N pill and ε edge label do not overlap`);
    check(layout.indexPairs.length === 0, `${viewport.name}: index labels do not overlap`);
    check(layout.indexFlagOverlaps.length === 0, `${viewport.name}: index N flag does not cover labels`);
    check(layout.labels === (viewport.mobile ? 10 : 17), `${viewport.name}: ${viewport.mobile ? 10 : 17} detail labels are visible (got ${layout.labels})`);
    if (viewport.stacked) {
      check(layout.definition.top >= layout.stage.bottom - 1, `${viewport.name}: cards are stacked (definition below stage)`);
    } else {
      check(layout.definition.left >= layout.stage.right - 1 && Math.abs(layout.definition.top - layout.stage.top) < 2, `${viewport.name}: cards are side by side and aligned`);
    }
  }

  // 9b. 极值状态：390px + ε=0.001（N=1001，画面内 0 项在带内）
  await setViewport(client, { width: 390, height: 844, dpr: 1, mobile: true });
  await setEps(0.001);
  await client.evaluate("window.scrollTo(0, 0)");
  await sleep(160);
  const extreme = await evalJSON(`(() => {
    const rect = (el) => { const b = el.getBoundingClientRect(); return { left: +b.left.toFixed(1), top: +b.top.toFixed(1), right: +b.right.toFixed(1), bottom: +b.bottom.toFixed(1), width: +b.width.toFixed(1), height: +b.height.toFixed(1) }; };
    const q = (s) => document.querySelector(s);
    const offenders = [];
    for (const el of document.querySelectorAll("body *")) {
      if (el.closest("svg") || el.matches(".sr-only, .skip")) continue;
      const b = el.getBoundingClientRect();
      if (b.width > 0 && b.height > 0 && (b.right > innerWidth + 1 || b.left < -1)) {
        offenders.push({ node: (typeof el.className === "string" && el.className ? el.className.split(/\s+/)[0] : el.tagName.toLowerCase()), left: +b.left.toFixed(1), right: +b.right.toFixed(1) });
      }
    }
    const clipped = [];
    for (const sel of ${JSON.stringify(CLIPPED_SELECTORS)}) {
      for (const el of document.querySelectorAll(sel)) {
        if (el.clientWidth > 0 && el.scrollWidth > el.clientWidth + 1) clipped.push({ sel, text: (el.textContent || "").trim().slice(0, 36) });
      }
    }
    return {
      viewport: [innerWidth, innerHeight],
      overflow: document.documentElement.scrollWidth - innerWidth,
      offenders,
      clipped,
      stage: rect(q("#stageCard")),
      caption: rect(q(".caption")),
      nOut: q("#nOut").textContent.trim(),
      epsOut: q("#epsOut").textContent.trim(),
      insideOut: q("#insideOut").textContent.trim(),
      nPill: q("#detailSvg .n-pill-text").textContent.trim(),
      ribbonFlag: q("#indexSvg .ribbon-n-text").textContent.trim(),
      captionText: q("#captionText").textContent.replace(/\s+/g, " ").trim(),
      aria: q("#epsRange").getAttribute("aria-valuetext"),
      detailDotsIn: document.querySelectorAll("#detailSvg .dot-in").length,
      indexCellsIn: document.querySelectorAll("#indexSvg .index-cell-in").length,
    };
  })()`);
  check(extreme.overflow <= 1, `extreme 390/ε=0.001: no horizontal overflow (${extreme.overflow}px)`);
  check(extreme.offenders.length === 0, `extreme 390/ε=0.001: no element crosses the viewport (${extreme.offenders.map((o) => o.node).join(", ") || "none"})`);
  check(extreme.clipped.length === 0, `extreme 390/ε=0.001: no key text is clipped (${extreme.clipped.map((c) => c.sel).join(", ") || "none"})`);
  check(extreme.stage.right <= 391, "extreme 390/ε=0.001: stage card stays in the viewport");
  check(extreme.nOut === "1001", `extreme 390/ε=0.001: N readout is 1001 (got ${extreme.nOut})`);
  check(extreme.epsOut === "0.0010", `extreme 390/ε=0.001: ε readout is 0.0010 (got ${extreme.epsOut})`);
  check(extreme.insideOut === "0", `extreme 390/ε=0.001: in-band readout is 0 (got ${extreme.insideOut})`);
  check(extreme.detailDotsIn === 0 && extreme.indexCellsIn === 0, "extreme 390/ε=0.001: zero visible terms are in-band");
  check(extreme.nPill.includes("N=1001"), `extreme: N pill reads N=1001 (got ${extreme.nPill})`);
  check(extreme.ribbonFlag.includes("N=1001"), `extreme: index flag reads N=1001 (got ${extreme.ribbonFlag})`);
  check(extreme.captionText.includes("画面外还有无穷多项"), "extreme: caption explains the off-screen infinite tail");
  check(extreme.aria.includes("N = 1001"), "extreme: aria-valuetext announces N=1001");
  details.extreme = { overflow: extreme.overflow, offenders: extreme.offenders.length, clipped: extreme.clipped.length, N: extreme.nOut };
  await reset();

  // 10. 网络 / 页面错误
  const requests = [];
  const pageErrors = [];
  const consoleErrors = [];
  for (const event of client.events) {
    if (event.method === "Network.requestWillBeSent") requests.push(event.params.request.url);
    else if (event.method === "Runtime.exceptionThrown") pageErrors.push(event.params.exceptionDetails?.text || "uncaught exception");
    else if (event.method === "Runtime.consoleAPICalled" && event.params.type === "error") {
      consoleErrors.push((event.params.args || []).map((arg) => arg.value ?? arg.description ?? "").join(" "));
    }
  }
  const external = requests.filter((url) => !/^(file|data|blob):/.test(url));
  const localIndex = requests.filter((url) => /index\.html(?:[?#]|$)/.test(url));
  check(requests.length > 0, "the probe document was requested");
  check(localIndex.length > 0, "the probe HTML itself was loaded from file:");
  check(external.length === 0, `zero external requests (got ${external.join(", ") || "none"})`);
  check(pageErrors.length === 0, `zero uncaught page errors (got ${pageErrors.join(" | ") || "none"})`);
  check(consoleErrors.length === 0, `zero console errors (got ${consoleErrors.join(" | ") || "none"})`);

  details.requests = requests.length;
  details.externalRequests = external.length;
  details.pageErrors = pageErrors.length;
  details.consoleErrors = consoleErrors.length;

  return {
    ok: failures.length === 0,
    checks,
    failures,
    details,
  };
}

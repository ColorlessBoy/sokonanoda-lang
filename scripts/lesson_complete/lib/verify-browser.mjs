// scripts/lesson_complete/lib/verify-browser.mjs — real Chrome checks.
//
// Every assertion is tied to a learner-visible action: click 开始, drag ε,
// answer, press Enter/Arrow keys.  The page is opened from file://, so any
// http(s) request would be an external request.

import { existsSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import {
  sleep,
  setViewport,
  setReducedMotion,
  reload,
  waitFor,
  clickSelector,
  focusSelector,
  pressKey,
  pressTab,
  pressEnter,
  capture,
} from "./cdp.mjs";
import { ROOT } from "./page.mjs";

const SHOT_PREFIX = "/tmp/lesson-complete-";
const SCREEN_NAMES = ["cover", "sequence", "band", "q1", "q2", "q3", "summary"];
const FORBIDDEN = /\b(?:cast|bible|xp|streak|debug|dashboard)\b|角色圣经|控制台|切换角色|积分|连对|完成度/i;

async function visibleState(client) {
  return client.evaluate(`(() => {
    const visible = [...document.querySelectorAll('[data-screen]')].filter((s) => !s.hidden);
    const screen = visible[0] || null;
    const next = screen ? screen.querySelector('[data-goto], [data-guard]') : null;
    const options = screen ? [...screen.querySelectorAll('.option')] : [];
    const feedback = screen ? screen.querySelector('.feedback') : null;
    const hint = screen ? screen.querySelector('.hint') : null;
    const avatar = screen ? screen.querySelector('.companion-avatar') : null;
    return {
      screen: screen ? screen.dataset.screen : null,
      visibleCount: visible.length,
      progress: document.querySelector('#progress-label').textContent.trim(),
      progressHidden: document.querySelector('#progress').hidden,
      progressDisplay: getComputedStyle(document.querySelector('#progress')).display,
      primaryCount: screen ? screen.querySelectorAll('.btn-primary').length : 0,
      nextDisabled: next ? Boolean(next.disabled) : null,
      nextText: next ? next.textContent.trim() : null,
      feedbackText: feedback ? feedback.textContent.trim() : null,
      feedbackClass: feedback ? feedback.className : null,
      hintHidden: hint ? hint.hidden : null,
      hintText: hint ? hint.textContent.trim() : null,
      hintExpanded: screen ? (screen.querySelector('.hint-btn')?.getAttribute('aria-expanded') ?? null) : null,
      hintDisabled: screen ? Boolean(screen.querySelector('.hint-btn')?.disabled) : null,
      expression: avatar ? avatar.getAttribute('data-expression') : null,
      companionCount: screen ? screen.querySelectorAll('.companion-avatar').length : 0,
      optionsLocked: screen ? Boolean(screen.querySelector('.options.is-locked')) : false,
      options: options.map((o) => ({
        value: o.dataset.choice,
        disabled: o.disabled,
        className: o.className,
        pressed: o.getAttribute('aria-pressed'),
        invalid: o.getAttribute('aria-invalid')
      })),
      active: document.activeElement ? {
        tag: document.activeElement.tagName,
        text: (document.activeElement.textContent || '').trim().slice(0, 24),
        id: document.activeElement.id || '',
        type: document.activeElement.type || ''
      } : null,
      panelCount: document.querySelectorAll('.cast-card, .bible, .stats, #xp-value, #streak-value, [data-character], [data-debug], [data-dashboard]').length,
      links: document.querySelectorAll('a[href]').length,
      forbidden: ${FORBIDDEN}.test(document.body.innerText),
      bodyText: document.body.innerText
    };
  })()`);
}

async function assertScreen(client, expect, name, step) {
  const state = await visibleState(client);
  expect(state.screen === name, `visible screen is ${name}`, state.screen);
  expect(state.visibleCount === 1, `${name}: exactly one screen is visible`, state.visibleCount);
  expect(state.primaryCount === 1, `${name}: exactly one primary action`, state.primaryCount);
  expect(state.panelCount === 0, `${name}: no cast/bible/XP/debug panels`, state.panelCount);
  expect(state.forbidden === false, `${name}: no forbidden dashboard copy on screen`);
  expect(state.links === 0, `${name}: no links (so no next-lesson 404)`, state.links);
  if (name === "cover") {
    expect(state.progressHidden === true, "cover hides the step counter");
    expect(state.progressDisplay === "none", "cover step counter has display:none, not just the hidden property", state.progressDisplay);
    expect(state.nextText === "开始", "cover primary action is 开始", state.nextText);
  } else {
    expect(state.progressHidden === false, `${name}: step counter is visible`);
    expect(state.progressDisplay !== "none", `${name}: step counter really renders`, state.progressDisplay);
    expect(state.progress === `第 ${step} / 6 步`, `${name}: progress is 第 ${step} / 6 步`, state.progress);
  }
  return state;
}

async function assertLayout(client, expect, label, details) {
  const report = await client.evaluate(`(() => {
    const screen = document.querySelector('.screen:not([hidden])');
    window.scrollTo(0, 0);
    const bad = [];
    for (const el of document.querySelectorAll('body *')) {
      if (!(el instanceof HTMLElement)) continue;
      const style = getComputedStyle(el);
      if (style.display === 'none' || style.visibility === 'hidden') continue;
      const rect = el.getBoundingClientRect();
      if (rect.width <= 0 || rect.height <= 0) continue;
      if (rect.right > innerWidth + 1 || rect.left < -1) {
        bad.push({ tag: el.tagName, cls: (typeof el.className === 'string' ? el.className : ''), left: +rect.left.toFixed(1), right: +rect.right.toFixed(1) });
        if (bad.length >= 8) break;
      }
    }
    return {
      screen: screen ? screen.dataset.screen : null,
      width: innerWidth,
      height: innerHeight,
      scrollWidth: document.documentElement.scrollWidth,
      bodyScrollWidth: document.body.scrollWidth,
      scrollHeight: document.documentElement.scrollHeight,
      bad
    };
  })()`);
  details.layout[label] = {
    width: report.width,
    height: report.height,
    scrollWidth: report.scrollWidth,
    scrollHeight: report.scrollHeight,
    bad: report.bad.length,
  };
  expect(report.scrollWidth <= report.width + 1, `${label}: no horizontal overflow`, report.scrollWidth);
  expect(report.bodyScrollWidth <= report.width + 1, `${label}: body has no horizontal overflow`, report.bodyScrollWidth);
  expect(report.bad.length === 0, `${label}: no element crosses the viewport`, report.bad);
  expect(report.scrollHeight <= report.height + 4, `${label}: one screen fits one viewport`, {
    scrollHeight: report.scrollHeight,
    height: report.height,
  });
  return report;
}

async function screenshot(client, expect, name, details) {
  try {
    await client.evaluate("window.scrollTo(0, 0)");
    await sleep(150);
    const buffer = await capture(client, { format: "png" });
    writeFileSync(`${SHOT_PREFIX}${name}.png`, buffer);
    const committed = join(ROOT, "screenshots", `${name}.png`);
    if (process.env.SOKO_UPDATE_SHOTS === "1" || !existsSync(committed)) {
      writeFileSync(committed, buffer);
    }
    details.screenshots.push(`${SHOT_PREFIX}${name}.png`);
    expect(buffer.length > 5000, `screenshot ${name} is non-empty`, buffer.length);
  } catch (error) {
    expect(false, `screenshot ${name} captured`, error.message);
  }
}

async function click(client, selector) {
  await clickSelector(client, selector);
  await sleep(420);
}

async function questionNext(client, selector) {
  await clickSelector(client, selector);
  await sleep(420);
}

export async function browserChecks({ client, log = () => {} } = {}) {
  const failures = [];
  let checks = 0;
  const details = { screenshots: [], requests: [], layout: {}, mathRows: 0 };
  const expect = (condition, label, extra) => {
    checks += 1;
    if (!condition) {
      failures.push(label + (extra === undefined ? "" : ` :: ${typeof extra === "string" ? extra : JSON.stringify(extra)}`));
    }
  };
  const section = (name) => log(`· ${name}`);
  const state = () => visibleState(client);
  const text = (selector) => client.evaluate(
    `(() => { const el = document.querySelector(${JSON.stringify(selector)}); return el ? el.textContent.trim() : null; })()`
  );

  await sleep(500);

  // ── cover ──────────────────────────────────────────────────
  section("cover, seven screens, no dashboard surface");
  const allScreens = await client.evaluate("document.querySelectorAll('[data-screen]').length");
  expect(allScreens === 7, "exactly seven screens exist", allScreens);
  await assertScreen(client, expect, "cover", 0);
  const coverCopy = await client.evaluate(`(() => {
    const title = document.querySelector('#cover-title');
    const line = document.querySelector('.cover-line');
    const start = document.querySelector('[data-goto="sequence"]');
    const tr = title.getBoundingClientRect();
    const lr = line.getBoundingClientRect();
    return {
      title: title.textContent.replace(/\s+/g, ''),
      titleHeight: tr.height,
      line: line.textContent.trim(),
      lineHeight: lr.height,
      start: start.textContent.trim(),
      startWidth: start.getBoundingClientRect().width
    };
  })()`);
  expect(coverCopy.title === "ε–N：一条带子，一个起点", "cover title is exact and visible", coverCopy.title);
  expect(coverCopy.titleHeight > 60, "cover title renders on screen", coverCopy.titleHeight);
  expect(coverCopy.line === "3 分钟看懂：为什么 1/n 会无限接近 0，以及 N 到底怎么选。", "cover promise is exact", coverCopy.line);
  expect(coverCopy.lineHeight > 20, "cover promise renders on screen", coverCopy.lineHeight);
  expect(coverCopy.start === "开始" && coverCopy.startWidth >= 120, "cover start button is obvious", coverCopy);
  const coverLayout = await assertLayout(client, expect, "1440-cover", details);
  expect(coverLayout.screen === "cover", "cover layout measured on the cover");
  await screenshot(client, expect, "cover", details);

  // ── sequence ───────────────────────────────────────────────
  section("sequence: dots settle, one bubble, one next");
  await click(client, '[data-goto="sequence"]');
  await sleep(1000);
  const seq = await assertScreen(client, expect, "sequence", 1);
  expect(seq.companionCount === 1, "sequence has one companion", seq.companionCount);
  expect(seq.expression === "curious", "sequence companion is curious", seq.expression);
  const seqDots = await client.evaluate(`(() => {
    const dots = [...document.querySelectorAll('.seq-dot')];
    const styles = getComputedStyle(dots[0] || document);
    return {
      count: dots.length,
      opacity: dots.map((dot) => getComputedStyle(dot).opacity),
      animation: styles.animationName,
      bubble: document.querySelector('.screen:not([hidden]) .bubble-text').textContent.trim()
    };
  })()`);
  expect(seqDots.count === 20, "sequence draws 20 terms", seqDots.count);
  expect(seqDots.opacity.every((value) => value === "1"), "all sequence dots are visible after settling", seqDots.opacity.slice(0, 3));
  expect(seqDots.animation !== "none", "sequence dots use a motion animation");
  expect(seqDots.bubble === "这些点越来越接近 0——但“越来越近”不是证明。怎么把“足够近”说死？", "sequence bubble is exact");
  await assertLayout(client, expect, "1440-sequence", details);
  await screenshot(client, expect, "sequence", details);

  // ── band + 250-value math sweep ────────────────────────────
  section("band: ε slider, strict N, terms exit, N moves right");
  await click(client, '[data-goto="band"]');
  const band = await assertScreen(client, expect, "band", 2);
  expect(band.companionCount === 1, "band has one companion", band.companionCount);
  expect(band.expression === "thinking", "band companion is thinking", band.expression);
  expect((await text("#band-caption")).includes("n = 5"), "band starts with N = 5 and the tail");
  expect((await text(".screen:not([hidden]) .bubble-text")) === "你把允许误差 ε 定下来，N 就是“从此以后都在带内”的起点。", "band bubble is exact");
  await screenshot(client, expect, "band", details);

  const sweep = await client.evaluate(`(() => {
    const range = document.querySelector('#eps-range');
    const stage = document.querySelector('#band-stage');
    const rows = [];
    for (let k = 1; k <= 250; k += 1) {
      range.value = String(k / 1000);
      range.dispatchEvent(new Event('input', { bubbles: true }));
      rows.push({
        eps: k / 1000,
        n: Number(stage.dataset.n),
        okFirst: stage.dataset.okFirst,
        okPrev: stage.dataset.okPrev,
        inside: Number(stage.dataset.inside),
        formula: document.querySelector('#n-formula').textContent,
        first: document.querySelector('#check-first').textContent,
        prev: document.querySelector('#check-prev').textContent,
        caption: document.querySelector('#band-caption').textContent,
        valuetext: document.querySelector('#eps-range').getAttribute('aria-valuetext')
      });
    }
    return rows;
  })()`);
  details.mathRows = sweep.length;
  expect(sweep.length === 250, "UI sweep covers all 250 slider values", sweep.length);
  let sweepFailures = 0;
  for (const row of sweep) {
    const k = Math.round(row.eps * 1000);
    const N = Math.floor(1000 / k) + 1;
    const inside = [4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20].filter((n) => 1 / n < row.eps).length;
    const bad = [];
    if (row.n !== N) bad.push(`N ${row.n} != ${N}`);
    if (row.okFirst !== "true" || row.okPrev !== "true") bad.push("strict flags false");
    if (!row.formula.includes(`= ${N}`)) bad.push(`formula ${row.formula}`);
    if (!row.first.includes(`1/${N}`)) bad.push(`first ${row.first}`);
    if (N > 1 && !row.prev.includes(`1/${N - 1}`)) bad.push(`prev ${row.prev}`);
    if (row.inside !== inside) bad.push(`inside ${row.inside} != ${inside}`);
    if (!row.valuetext.includes(`N = ${N}`)) bad.push(`valuetext ${row.valuetext}`);
    if (bad.length) {
      sweepFailures += 1;
      if (sweepFailures <= 3) failures.push(`math sweep ε=${row.eps} :: ${bad.join(" | ")}`);
    }
  }
  checks += 1;
  if (sweepFailures > 0) {
    failures.push(`math sweep: ${sweepFailures} of ${sweep.length} ε values disagree with the reference`);
  }

  // Put the slider back on the lesson's starting value with a real click.
  await click(client, '.preset[data-eps="0.22"]');
  const band22 = await client.evaluate(`(() => ({
    n: Number(document.querySelector('#band-stage').dataset.n),
    epsX: Number(document.querySelector('#band-eps-line').getAttribute('x1')),
    nX: Number(document.querySelector('#band-n-line').getAttribute('x1')),
    dot5: document.querySelector('#band-dots [data-term="5"]').getAttribute('class'),
    dot4: document.querySelector('#band-dots [data-term="4"]').getAttribute('class'),
    pressed: [...document.querySelectorAll('.preset')].map((b) => b.getAttribute('aria-pressed')),
    valuetext: document.querySelector('#eps-range').getAttribute('aria-valuetext'),
    formula: document.querySelector('#n-formula').textContent
  }))()`);
  expect(band22.n === 5, "ε = 0.22 → N = 5", band22.n);
  expect(band22.dot5.includes("is-in") && band22.dot4.includes("is-out"), "a₅ is in and a₄ is out at ε = 0.22", band22);
  expect(band22.formula === "N = ⌊1/0.220⌋ + 1 = 5", "formula is live and exact", band22.formula);
  expect(band22.pressed.filter((value) => value === "true").length === 1, "exactly one preset is pressed", band22.pressed);
  expect(band22.valuetext.includes("N = 5"), "slider value text reports N = 5", band22.valuetext);

  await click(client, '.preset[data-eps="0.10"]');
  const band10 = await client.evaluate(`(() => ({
    n: Number(document.querySelector('#band-stage').dataset.n),
    epsX: Number(document.querySelector('#band-eps-line').getAttribute('x1')),
    nX: Number(document.querySelector('#band-n-line').getAttribute('x1')),
    dot5: document.querySelector('#band-dots [data-term="5"]').getAttribute('class'),
    dot11: document.querySelector('#band-dots [data-term="11"]').getAttribute('class'),
    caption: document.querySelector('#band-caption').textContent
  }))()`);
  expect(band10.n === 11, "ε = 0.10 → N = 11", band10.n);
  expect(band10.dot5.includes("is-out"), "a₅ exits the band when ε shrinks to 0.10", band10.dot5);
  expect(band10.dot11.includes("is-in"), "a₁₁ is the first inside term at ε = 0.10", band10.dot11);
  expect(band10.nX > band22.nX, "N moves right on the index axis when ε shrinks", { from: band22.nX, to: band10.nX });
  expect(band10.epsX < band22.epsX, "the band narrows when ε shrinks", { from: band22.epsX, to: band10.epsX });
  expect(band10.caption.includes("1/ε = 10 是整数"), "integer reciprocal boundary is explained", band10.caption);

  await click(client, '.preset[data-eps="0.05"]');
  const band05 = await state();
  expect(band05.screen === "band", "band is still visible at ε = 0.05");
  expect((await text("#band-stage")).includes("N = 21 →") || (await text("#band-n-text")).includes("21"), "N = 21 is shown outside the drawn index axis", await text("#band-n-text"));
  expect((await text("#band-caption")).includes("序号轴外"), "caption says N is outside the drawn axis");
  await client.evaluate(`(() => { const r = document.querySelector('#eps-range'); r.value = '0.001'; r.dispatchEvent(new Event('input', {bubbles:true})); })()`);
  const band001 = await client.evaluate(`(() => ({
    n: Number(document.querySelector('#band-stage').dataset.n),
    pill: document.querySelector('#band-n-text').textContent,
    caption: document.querySelector('#band-caption').textContent,
    visibleDots: [...document.querySelectorAll('#band-dots .band-dot.is-in')].length
  }))()`);
  expect(band001.n === 1001, "ε = 0.001 → N = 1001", band001.n);
  expect(band001.pill.includes("1001"), "N pill reports 1001", band001.pill);
  expect(band001.visibleDots === 0, "no drawn term is inside at ε = 0.001", band001.visibleDots);
  expect(band001.caption.includes("序号轴外"), "caption is honest about the drawn 20 terms", band001.caption);
  await click(client, '.preset[data-eps="0.25"]');
  const band25 = await client.evaluate(`(() => ({
    n: Number(document.querySelector('#band-stage').dataset.n),
    dot4: document.querySelector('#band-dots [data-term="4"]').getAttribute('class'),
    dot5: document.querySelector('#band-dots [data-term="5"]').getAttribute('class')
  }))()`);
  expect(band25.n === 5 && band25.dot4.includes("is-out") && band25.dot5.includes("is-in"), "ε = 0.25 → N = 5; the boundary term a₄ is out", band25);
  await click(client, '.preset[data-eps="0.22"]');
  await assertLayout(client, expect, "1440-band", details);

  // ── Q1 ─────────────────────────────────────────────────────
  section("Q1: wrong answer does not advance, hint is optional, retry works");
  await click(client, '[data-goto="q1"]');
  const q1 = await assertScreen(client, expect, "q1", 3);
  expect(q1.nextDisabled === true, "Q1 next is disabled before answering");
  expect(q1.feedbackText === "", "Q1 feedback starts empty");
  expect(q1.expression === "curious", "Q1 companion starts curious", q1.expression);
  expect((await text("#q1-prompt")) === "ε = 0.22 时，N 是多少？", "Q1 prompt is exact");
  await screenshot(client, expect, "q1", details);
  await assertLayout(client, expect, "1440-q1", details);

  await click(client, '[data-screen="q1"] .option[data-choice="4"]');
  const q1Wrong = await assertScreen(client, expect, "q1", 3);
  expect(q1Wrong.nextDisabled === true, "wrong Q1 answer does not enable 下一步");
  expect(q1Wrong.feedbackClass?.includes("is-wrong"), "wrong Q1 feedback is marked wrong", q1Wrong.feedbackClass);
  expect(q1Wrong.feedbackText.includes("0.25") && q1Wrong.feedbackText.includes("带外"), "wrong Q1 feedback explains a₄", q1Wrong.feedbackText);
  expect(q1Wrong.expression === "oops", "wrong Q1 sets the oops expression", q1Wrong.expression);
  expect(q1Wrong.options.some((option) => option.invalid === "true"), "wrong Q1 option is aria-invalid", q1Wrong.options);
  await sleep(250);
  expect((await state()).screen === "q1", "Q1 does not auto-advance after a wrong answer");

  await click(client, '[data-screen="q1"] [data-hint="q1"]');
  const hinted = await state();
  expect(hinted.hintHidden === false, "需要提示 reveals one hint line");
  expect(hinted.hintExpanded === "true", "hint button expands", hinted.hintExpanded);
  expect(hinted.hintText.startsWith("阿理："), "hint is attributed to 阿理", hinted.hintText);
  expect(hinted.hintText.includes("4.545"), "hint names the 1/0.22 computation", hinted.hintText);
  expect(hinted.expression === "thinking", "hint sets the thinking expression", hinted.expression);
  const visibleHints = await client.evaluate("document.querySelectorAll('.screen:not([hidden]) .hint:not([hidden])').length");
  expect(visibleHints === 1, "exactly one hint line is visible", visibleHints);

  await click(client, '[data-screen="q1"] .option[data-choice="10"]');
  const q1Wrong2 = await state();
  expect(q1Wrong2.nextDisabled === true, "second wrong Q1 answer still does not advance");
  expect(q1Wrong2.feedbackText.includes("最小"), "N = 10 feedback explains minimality", q1Wrong2.feedbackText);

  await click(client, '[data-screen="q1"] .option[data-choice="5"]');
  const q1Right = await assertScreen(client, expect, "q1", 3);
  expect(q1Right.nextDisabled === false, "correct Q1 enables 下一步");
  expect(q1Right.feedbackClass?.includes("is-correct"), "correct Q1 feedback is marked correct", q1Right.feedbackClass);
  expect(q1Right.feedbackText.includes("= 5"), "correct Q1 feedback gives N = 5", q1Right.feedbackText);
  expect(q1Right.expression === "happy", "correct Q1 sets the happy expression", q1Right.expression);
  expect(q1Right.options.every((option) => option.disabled), "Q1 options lock after the correct answer", q1Right.options);
  expect(q1Right.hintDisabled === true, "Q1 hint disables after the correct answer", q1Right.hintDisabled);
  expect(q1Right.active?.text === "下一步", "Q1 focuses 下一步 without advancing", q1Right.active);
  await sleep(300);
  expect((await state()).screen === "q1", "Q1 does not auto-advance after a correct answer");
  await questionNext(client, '[data-screen="q1"] [data-goto="q2"]');
  await assertScreen(client, expect, "q2", 4);

  // ── Q2 ─────────────────────────────────────────────────────
  section("Q2: true/false with the strict boundary");
  const q2 = await state();
  expect(q2.nextDisabled === true, "Q2 next starts disabled");
  expect(q2.expression === "curious", "Q2 companion starts curious", q2.expression);
  expect((await text("#q2-prompt")) === "N = 4 对 ε = 0.22 够用吗？", "Q2 prompt is exact");
  await screenshot(client, expect, "q2", details);
  await assertLayout(client, expect, "1440-q2", details);
  await click(client, '[data-screen="q2"] .option[data-choice="true"]');
  const q2Wrong = await state();
  expect(q2Wrong.nextDisabled === true, "Q2=true is rejected", q2Wrong.nextDisabled);
  expect(q2Wrong.feedbackText.includes("0.25"), "Q2 wrong feedback compares 1/4 with 0.22", q2Wrong.feedbackText);
  expect(q2Wrong.expression === "oops", "Q2 wrong sets oops", q2Wrong.expression);
  await click(client, '[data-screen="q2"] .option[data-choice="false"]');
  const q2Right = await state();
  expect(q2Right.nextDisabled === false, "Q2=false enables 下一步");
  expect(q2Right.feedbackText.includes("N = 5"), "Q2 correct feedback names N = 5", q2Right.feedbackText);
  expect(q2Right.expression === "happy", "Q2 correct sets happy", q2Right.expression);
  await sleep(300);
  expect((await state()).screen === "q2", "Q2 does not auto-advance");
  await questionNext(client, '[data-screen="q2"] [data-goto="q3"]');
  await assertScreen(client, expect, "q3", 5);

  // ── Q3 ─────────────────────────────────────────────────────
  section("Q3: integer reciprocal boundary");
  const q3 = await state();
  expect(q3.nextDisabled === true, "Q3 next starts disabled");
  expect((await text("#q3-prompt")) === "ε = 0.10 时，N 是多少？", "Q3 prompt is exact");
  await screenshot(client, expect, "q3", details);
  await assertLayout(client, expect, "1440-q3", details);
  await click(client, '[data-screen="q3"] .option[data-choice="10"]');
  const q3Wrong = await state();
  expect(q3Wrong.nextDisabled === true, "Q3=N=10 is rejected", q3Wrong.nextDisabled);
  expect(q3Wrong.feedbackText.includes("等于 ε"), "Q3 wrong feedback explains a₁₀ = ε", q3Wrong.feedbackText);
  await click(client, '[data-screen="q3"] .option[data-choice="11"]');
  const q3Right = await state();
  expect(q3Right.nextDisabled === false, "Q3=N=11 enables 下一步");
  expect(q3Right.feedbackText.includes("= 11"), "Q3 correct feedback gives N = 11", q3Right.feedbackText);
  expect(q3Right.expression === "happy", "Q3 correct sets happy", q3Right.expression);
  await sleep(300);
  expect((await state()).screen === "q3", "Q3 does not auto-advance");
  await questionNext(client, '[data-screen="q3"] [data-goto="summary"]');

  // ── summary ────────────────────────────────────────────────
  section("summary: formal definition, boundary explanation, badge, placeholder");
  const summary = await assertScreen(client, expect, "summary", 6);
  expect(summary.primaryCount === 1, "summary has one primary action (replay)");
  const summaryData = await client.evaluate(`(() => ({
    badge: document.querySelector('#summary-badge').innerText.replace(/\\s+/g, ' ').trim(),
    badgeVisible: document.querySelector('#summary-badge').checkVisibility(),
    definition: document.querySelector('#summary-definition').textContent.trim(),
    definitionVisible: document.querySelector('#summary-definition').checkVisibility(),
    definitionRect: (() => {
      const r = document.querySelector('#summary-definition').getBoundingClientRect();
      return { w: +r.width.toFixed(1), h: +r.height.toFixed(1) };
    })(),
    card: document.querySelector('.summary-card').innerText.replace(/\\s+/g, ' ').trim(),
    replay: document.querySelector('[data-action="replay"]').textContent.trim(),
    placeholder: document.querySelector('.summary-actions .btn-secondary').textContent.trim(),
    placeholderDisabled: document.querySelector('.summary-actions .btn-secondary').disabled,
    links: document.querySelectorAll('a[href]').length,
    companions: document.querySelectorAll('.screen:not([hidden]) .companion-avatar').length
  }))()`);
  expect(summaryData.badge === "✓ 你理解了 ε–N", "badge is exactly 你理解了 ε–N", summaryData.badge);
  expect(summaryData.badgeVisible === true, "badge is visible");
  expect(summaryData.definition === "∀ε>0, ∃N, ∀n≥N, |aₙ−L|<ε", "formal definition is exact", summaryData.definition);
  expect(summaryData.definitionVisible === true, "formal definition is visible");
  expect(summaryData.definitionRect.h > 20 && summaryData.definitionRect.w > 100, "formal definition has a readable box", summaryData.definitionRect);
  expect(summaryData.card.includes("N = ⌊1/ε⌋ + 1"), "summary explains N = ⌊1/ε⌋ + 1", summaryData.card);
  expect(summaryData.card.includes("严格小于"), "summary explains the strict inequality", summaryData.card);
  expect(summaryData.card.includes("N = 11"), "summary keeps the ε = 0.10 example", summaryData.card);
  expect(summaryData.replay === "再学一遍", "summary offers 再学一遍");
  expect(summaryData.placeholder === "下一课（占位）", "summary names 下一课（占位）");
  expect(summaryData.placeholderDisabled === true, "next lesson is disabled, never a 404");
  expect(summaryData.links === 0, "summary has no link to a missing lesson");
  expect(summaryData.companions === 0, "summary has no extra companion panel");
  await screenshot(client, expect, "summary", details);
  await assertLayout(client, expect, "1440-summary", details);

  // ── replay reset ───────────────────────────────────────────
  section("replay resets the whole lesson");
  await click(client, '[data-action="replay"]');
  const replay = await assertScreen(client, expect, "cover", 0);
  expect(replay.progressHidden === true, "replay returns to the cover and hides progress");
  const reset = await client.evaluate(`(() => ({
    eps: document.querySelector('#eps-range').value,
    q1Next: document.querySelector('[data-guard="q1"]').disabled,
    q1Feedback: document.querySelector('#q1-feedback').textContent,
    q1HintHidden: document.querySelector('#q1-hint').hidden,
    q1Locked: document.querySelector('[data-screen="q1"] .options').className.includes('is-locked')
  }))()`);
  expect(reset.eps === "0.22", "replay resets ε to 0.22", reset.eps);
  expect(reset.q1Next === true, "replay disables Q1 next again", reset.q1Next);
  expect(reset.q1Feedback === "", "replay clears Q1 feedback", reset.q1Feedback);
  expect(reset.q1HintHidden === true, "replay hides the Q1 hint", reset.q1HintHidden);
  expect(reset.q1Locked === false, "replay unlocks Q1 options", reset.q1Locked);
  expect(replay.active?.tag === "H1" && replay.active?.text.includes("ε–N"), "replay focuses the cover title", replay.active);

  // ── keyboard path ──────────────────────────────────────────
  section("keyboard: Tab / Enter / Arrow keys, no auto-advance");
  await pressTab(client);
  let active = await client.evaluate(`({tag: document.activeElement.tagName, text: (document.activeElement.textContent || '').trim()})`);
  expect(active.tag === "BUTTON" && active.text === "开始", "Tab reaches 开始", active);
  await pressEnter(client);
  await sleep(350);
  await assertScreen(client, expect, "sequence", 1);
  await pressTab(client);
  active = await client.evaluate(`({tag: document.activeElement.tagName, text: (document.activeElement.textContent || '').trim()})`);
  expect(active.tag === "BUTTON" && active.text === "下一步", "Tab reaches 下一步", active);
  await pressEnter(client);
  await sleep(350);
  await assertScreen(client, expect, "band", 2);
  await pressTab(client);
  active = await client.evaluate(`({tag: document.activeElement.tagName, type: document.activeElement.type})`);
  expect(active.tag === "INPUT" && active.type === "range", "Tab reaches the ε slider", active);
  await pressKey(client, { key: "ArrowRight", code: "ArrowRight", windowsVirtualKeyCode: 39 });
  const slid = await client.evaluate(`({value: document.querySelector('#eps-range').value, out: document.querySelector('#eps-value').textContent})`);
  expect(slid.value === "0.221" && slid.out === "ε = 0.221", "ArrowRight changes ε by one step", slid);
  for (let i = 0; i < 5; i += 1) await pressTab(client);
  active = await client.evaluate(`({tag: document.activeElement.tagName, text: (document.activeElement.textContent || '').trim()})`);
  expect(active.tag === "BUTTON" && active.text === "下一步", "Tab reaches band 下一步", active);
  await pressEnter(client);
  await sleep(350);
  await assertScreen(client, expect, "q1", 3);
  await pressTab(client);
  active = await client.evaluate(`({tag: document.activeElement.tagName, text: (document.activeElement.textContent || '').trim()})`);
  expect(active.text === "N = 4", "Tab reaches the first Q1 option", active);
  await pressKey(client, { key: "ArrowDown", code: "ArrowDown", windowsVirtualKeyCode: 40 });
  active = await client.evaluate(`(document.activeElement.textContent || '').trim()`);
  expect(active === "N = 5", "ArrowDown moves to N = 5", active);
  await pressEnter(client);
  await sleep(300);
  let q1Keyboard = await state();
  expect(q1Keyboard.nextDisabled === false, "keyboard answer enables 下一步", q1Keyboard.nextDisabled);
  expect(q1Keyboard.screen === "q1", "keyboard answer does not auto-advance", q1Keyboard.screen);
  await pressEnter(client);
  await sleep(350);
  await assertScreen(client, expect, "q2", 4);
  await pressTab(client);
  await pressKey(client, { key: "ArrowRight", code: "ArrowRight", windowsVirtualKeyCode: 39 });
  active = await client.evaluate(`(document.activeElement.textContent || '').trim()`);
  expect(active === "错误", "ArrowRight moves to the false option", active);
  await pressEnter(client);
  await sleep(300);
  let q2Keyboard = await state();
  expect(q2Keyboard.nextDisabled === false && q2Keyboard.screen === "q2", "Q2 keyboard answer stays on the screen", q2Keyboard);
  await pressEnter(client);
  await sleep(350);
  await assertScreen(client, expect, "q3", 5);
  await pressTab(client);
  await pressKey(client, { key: "ArrowRight", code: "ArrowRight", windowsVirtualKeyCode: 39 });
  active = await client.evaluate(`(document.activeElement.textContent || '').trim()`);
  expect(active === "N = 11", "ArrowRight moves to N = 11", active);
  await pressEnter(client);
  await sleep(300);
  let q3Keyboard = await state();
  expect(q3Keyboard.nextDisabled === false && q3Keyboard.screen === "q3", "Q3 keyboard answer stays on the screen", q3Keyboard);
  await pressEnter(client);
  await sleep(350);
  await assertScreen(client, expect, "summary", 6);

  // ── reduced motion ─────────────────────────────────────────
  section("reduced motion: no running animation, dots readable");
  await setReducedMotion(client, true);
  await reload(client);
  await waitFor(client, "document.readyState === 'complete' && document.querySelectorAll('[data-screen]').length === 7");
  await sleep(350);
  const reduced = await client.evaluate(`(() => ({
    media: matchMedia('(prefers-reduced-motion: reduce)').matches,
    attr: document.querySelector('#app').dataset.reducedMotion,
    running: document.getAnimations().filter((animation) => animation.playState === 'running').length,
    dotAnimation: getComputedStyle(document.querySelector('.seq-dot')).animationName,
    dotOpacity: getComputedStyle(document.querySelector('.seq-dot')).opacity
  }))()`);
  expect(reduced.media === true, "reduced-motion media query is emulated");
  expect(reduced.attr === "true", "page records the reduced-motion preference", reduced.attr);
  expect(reduced.running === 0, "no animation runs under reduced motion", reduced.running);
  expect(reduced.dotAnimation === "none", "sequence dots have no animation", reduced.dotAnimation);
  expect(reduced.dotOpacity === "1", "sequence dots are immediately visible", reduced.dotOpacity);
  await click(client, '[data-goto="sequence"]');
  await sleep(200);
  const reducedSeq = await client.evaluate(`(() => ({
    screen: document.querySelector('.screen:not([hidden])').dataset.screen,
    running: document.getAnimations().filter((animation) => animation.playState === 'running').length,
    dots: [...document.querySelectorAll('.seq-dot')].every((dot) => getComputedStyle(dot).opacity === '1')
  }))()`);
  expect(reducedSeq.screen === "sequence", "reduced motion still moves to the sequence");
  expect(reducedSeq.running === 0, "screen change does not animate under reduced motion", reducedSeq.running);
  expect(reducedSeq.dots === true, "all dots are visible without animation");

  // ── 390px layout ───────────────────────────────────────────
  section("390px: one screen, zero horizontal overflow, no long scroll");
  await setReducedMotion(client, false);
  await setViewport(client, { width: 390, height: 844, dpr: 2, mobile: true });
  await reload(client);
  await waitFor(client, "document.readyState === 'complete' && document.querySelectorAll('[data-screen]').length === 7");
  await sleep(500);
  await assertLayout(client, expect, "390-cover", details);
  await click(client, '[data-goto="sequence"]');
  await assertLayout(client, expect, "390-sequence", details);
  await click(client, '[data-goto="band"]');
  await assertLayout(client, expect, "390-band", details);
  await click(client, '[data-goto="q1"]');
  await assertLayout(client, expect, "390-q1", details);
  await click(client, '[data-screen="q1"] .option[data-choice="5"]');
  await click(client, '[data-screen="q1"] [data-goto="q2"]');
  await assertLayout(client, expect, "390-q2", details);
  await click(client, '[data-screen="q2"] .option[data-choice="false"]');
  await click(client, '[data-screen="q2"] [data-goto="q3"]');
  await assertLayout(client, expect, "390-q3", details);
  await click(client, '[data-screen="q3"] .option[data-choice="11"]');
  await click(client, '[data-screen="q3"] [data-goto="summary"]');
  await assertLayout(client, expect, "390-summary", details);
  const mobile = await visibleState(client);
  expect(mobile.screen === "summary", "390px path reaches the summary");
  expect(mobile.panelCount === 0, "390px summary has no dashboard panels");

  // ── network + console ──────────────────────────────────────
  section("zero external requests, zero console/page errors");
  const requests = client.events
    .filter((event) => event.method === "Network.requestWillBeSent")
    .map((event) => event.params.request.url);
  const external = requests.filter((url) => /^https?:/i.test(url));
  details.requests = external;
  expect(external.length === 0, "no http(s) requests were made", external);
  const errors = client.events.filter((event) => {
    if (event.method === "Runtime.exceptionThrown") return true;
    if (event.method === "Log.entryAdded" && event.params.entry.level === "error") return true;
    if (event.method === "Runtime.consoleAPICalled" && event.params.type === "error") return true;
    return false;
  }).map((event) => ({
    method: event.method,
    text: event.params?.entry?.text || event.params?.exceptionDetails?.text || event.params?.type || "error",
  }));
  details.consoleErrors = errors;
  expect(errors.length === 0, "no console or page errors", errors.slice(0, 3));

  return { ok: failures.length === 0, checks, failures, details };
}

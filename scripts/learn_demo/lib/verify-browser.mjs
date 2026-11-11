// scripts/learn_demo/lib/verify-browser.mjs — real Chrome checks.
//
// Every assertion is tied to a user-visible action: click a character, drag the
// ε slider, answer a question, press Enter on a focused choice.  Data-only
// checks (like "the JSON says 5") are not enough; the page must show it.

import { existsSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import {
  sleep,
  setViewport,
  clickSelector,
  focusSelector,
  pressKey,
  setReducedMotion,
  reload,
  waitFor,
  fullPageCapture,
  ensureDir,
} from "./cdp.mjs";
import { computeN, termsFor, TERMS } from "./math.mjs";
import { ROOT } from "./page.mjs";

const TMP_PREFIX = "/tmp/learn-demo-";

async function state(client) {
  return client.evaluate("window.SokoLearnDemo.getState()");
}

async function text(client, selector) {
  return client.evaluate(`(() => {
    const el = document.querySelector(${JSON.stringify(selector)});
    return el ? (el.innerText || el.textContent || "").trim() : null;
  })()`);
}

async function count(client, selector) {
  return client.evaluate(`document.querySelectorAll(${JSON.stringify(selector)}).length`);
}

async function layoutReport(client) {
  return client.evaluate(`(() => {
    const bad = [...document.querySelectorAll("body *")].filter((el) => {
      const rect = el.getBoundingClientRect();
      const style = getComputedStyle(el);
      if (rect.width <= 0 || rect.height <= 0) return false;
      if (style.position === "fixed") return false;
      return rect.right > innerWidth + 1 || rect.left < -1;
    }).slice(0, 12).map((el) => ({
      tag: el.tagName,
      cls: typeof el.className === "string" ? el.className : (el.className && el.className.baseVal) || "",
      left: +el.getBoundingClientRect().left.toFixed(1),
      right: +el.getBoundingClientRect().right.toFixed(1),
    }));
    const box = (selector) => {
      const el = document.querySelector(selector);
      if (!el) return null;
      const rect = el.getBoundingClientRect();
      return { x: +rect.x.toFixed(1), y: +rect.y.toFixed(1), w: +rect.width.toFixed(1), h: +rect.height.toFixed(1) };
    };
    return {
      width: innerWidth,
      height: innerHeight,
      scrollWidth: document.documentElement.scrollWidth,
      bodyScrollWidth: document.body.scrollWidth,
      bad,
      cast: [...document.querySelectorAll("[data-character]")].map((el) => {
        const rect = el.getBoundingClientRect();
        return { id: el.dataset.character, w: +rect.width.toFixed(1), h: +rect.height.toFixed(1), pressed: el.getAttribute("aria-pressed") };
      }),
      svg: box("#scene-svg"),
      plot: box(".plot-wrap"),
      bubble: box(".bubble"),
      bubbleText: box(".bubble-text"),
      game: box(".game-card"),
      question: box(".question-root"),
      dotPlot: box(".dot-plot"),
      formal: box(".formal-card"),
      completion: box(".completion"),
    };
  })()`);
}

export async function browserChecks({ client, log = () => {} } = {}) {
  const failures = [];
  let checks = 0;
  const details = { screenshots: [], requests: [], layout: {} };
  const expect = (condition, label, extra) => {
    checks += 1;
    if (!condition) {
      failures.push(
        label + (extra === undefined ? "" : ` :: ${typeof extra === "string" ? extra : JSON.stringify(extra)}`)
      );
    }
  };
  const section = (name) => log(`· ${name}`);

  const assertLayout = async (label) => {
    const report = await layoutReport(client);
    details.layout[label] = { width: report.width, scrollWidth: report.scrollWidth, bad: report.bad.length };
    expect(report.scrollWidth <= report.width + 0.5, `${label}: document has no horizontal overflow`, report.scrollWidth);
    expect(report.bodyScrollWidth <= report.width + 0.5, `${label}: body has no horizontal overflow`, report.bodyScrollWidth);
    expect(report.bad.length === 0, `${label}: no element crosses the viewport`, report.bad);
    expect(report.cast.length === 2 && report.cast.every((card) => card.w > 80 && card.h > 100), `${label}: both cast cards stay visible`, report.cast);
    return report;
  };

  const screenshot = async (name, width, height) => {
    try {
      await setViewport(client, { width, height, mobile: width < 700 });
      await sleep(180);
      await client.evaluate("window.scrollTo(0, 0)");
      await sleep(80);
      const buffer = await fullPageCapture(client);
      writeFileSync(`${TMP_PREFIX}${name}.png`, buffer);
      const committed = join(ROOT, "screenshots", `${name}.png`);
      // Chrome's PNG bytes vary by a few pixels between runs.  Do not dirty the
      // committed evidence on every check; refresh it explicitly instead.
      if (process.env.SOKO_UPDATE_SHOTS === "1" || !existsSync(committed)) {
        ensureDir(join(ROOT, "screenshots"));
        writeFileSync(committed, buffer);
      }
      details.screenshots.push(`${TMP_PREFIX}${name}.png`);
      expect(buffer.length > 5000, `screenshot ${name} is non-empty`, buffer.length);
    } catch (error) {
      expect(false, `screenshot ${name} captured`, error.message);
    }
  };

  const click = async (selector) => {
    await clickSelector(client, selector);
    await sleep(100);
  };

  await sleep(300);

  // ── initial state + cast ────────────────────────────────────
  section("initial state, cast, and accessible structure");
  let s = await state(client);
  expect(s.characters.length === 2, "two characters are rendered", s.characters.length);
  expect(new Set(s.characters.map((c) => c.avatar)).size === 2, "the two avatars are visually distinct");
  expect(new Set(s.characters.map((c) => c.id)).size === 2, "character ids are distinct");
  expect(s.characters.some((c) => c.name === "小析") && s.characters.some((c) => c.name === "阿理"), "cast names are 小析 and 阿理");
  const avatars = await client.evaluate(
    `[...document.querySelectorAll(".companion-avatar")].map((svg) => svg.dataset.avatar)`
  );
  expect(avatars.length === 2 && avatars[0] !== avatars[1], "two distinct 2D SVG avatars are in the DOM", avatars);
  const cast = await client.evaluate(
    `[...document.querySelectorAll("[data-character]")].map((el) => ({
      id: el.dataset.character,
      pressed: el.getAttribute("aria-pressed"),
      label: el.getAttribute("aria-label"),
      w: el.getBoundingClientRect().width,
      h: el.getBoundingClientRect().height
    }))`
  );
  expect(cast.length === 2, "two cast buttons", cast.length);
  expect(cast.every((card) => card.w > 80 && card.h > 100), "both cast cards are visible", cast);
  expect(cast.filter((card) => card.pressed === "true").length === 1, "exactly one active speaker", cast);
  expect(cast.find((card) => card.pressed === "true").id === "xiaoxi", "小析 is the initial speaker");
  expect(
    cast.every((card) => card.label && card.label.includes(card.id === "xiaoxi" ? "小析" : "阿理")),
    "cast buttons have accessible names",
    cast
  );
  expect(s.activeSpeaker === "xiaoxi", "state names 小析 as speaker");
  expect(s.bubble.text.includes("带子"), "小析's first line points at the band", s.bubble);
  expect(s.questionId === "q1" && s.questionType === "multiple-choice", "the first game question is Q1 multiple-choice");
  expect(s.epsilon === 0.22 && s.N === 5 && s.inside === 16, "initial ε = 0.22 → N = 5 with 16 shown terms", s);
  expect(s.formulaHolds === true, "initial strict boundary invariant holds");
  expect(s.xp === 0 && s.streak === 0 && s.progress === 0, "game starts at 0 XP, 0 streak, 0 progress");
  expect(s.badgeVisible === false && s.completed === false, "completion badge is hidden at the start");
  expect(s.hintButtonDisabled === false, "hint button is available at the start");

  const a11y = await client.evaluate(
    `(() => {
      const bubble = document.querySelector("#bubble");
      const feedback = document.querySelector("#feedback");
      const announcer = document.querySelector("#announcer");
      const progress = document.querySelector("#progress-track");
      const caption = document.querySelector("#scene-caption");
      const range = document.querySelector("#eps-range");
      return {
        bubbleRole: bubble.getAttribute("role"),
        bubbleLive: bubble.getAttribute("aria-live"),
        feedbackLive: feedback.getAttribute("aria-live"),
        announcerLive: announcer.getAttribute("aria-live"),
        progressRole: progress.getAttribute("role"),
        progressNow: progress.getAttribute("aria-valuenow"),
        caption: caption.textContent.trim(),
        rangeLabel: range.getAttribute("aria-valuetext"),
        rangeMin: range.getAttribute("min"),
        rangeMax: range.getAttribute("max"),
        rangeStep: range.getAttribute("step"),
        inputs: [...document.querySelectorAll("input")].map((el) => el.type),
        textareas: document.querySelectorAll("textarea").length,
        contenteditable: document.querySelectorAll('[contenteditable="true"]').length,
        forms: document.querySelectorAll("form").length,
        castCards: document.querySelectorAll(".cast-card").length
      };
    })()`
  );
  expect(a11y.bubbleRole === "status" && a11y.bubbleLive === "polite", "speech bubble is a polite status region", a11y);
  expect(a11y.feedbackLive === "polite" && a11y.announcerLive === "polite", "feedback and announcer are aria-live", a11y);
  expect(a11y.progressRole === "progressbar" && a11y.progressNow === "0", "progress bar exposes aria-valuenow", a11y);
  expect(a11y.caption.length > 0, "visible caption is populated");
  expect(a11y.rangeLabel.includes("N = 5"), "slider aria-valuetext reports N = 5", a11y.rangeLabel);
  expect(a11y.rangeMin === "0.001" && a11y.rangeMax === "0.25" && a11y.rangeStep === "0.001", "ε slider exposes the real 0.001–0.25 range", a11y);
  expect(a11y.inputs.length === 1 && a11y.inputs.every((type) => type === "range"), "initial input surface is one range", a11y.inputs);
  expect(a11y.textareas === 0 && a11y.contenteditable === 0 && a11y.forms === 0, "no free-form text or code editor path", a11y);
  expect(a11y.castCards === 2, "both cast cards exist", a11y.castCards);

  // ── scene geometry + formula ───────────────────────────────
  section("ε–N number line, band, N marker, formula");
  const geom = await client.evaluate(
    `(() => {
      const band = document.querySelector("#sc-band");
      const axis = document.querySelector("#sc-axis");
      const nline = document.querySelector("#sc-n-line");
      const x0 = Number(axis.getAttribute("x1"));
      const x1 = Number(axis.getAttribute("x2"));
      return {
        band: Number(band.getAttribute("width")),
        width: x1 - x0,
        nOffset: Number(nline.getAttribute("x1")) - x0,
        dots: [...document.querySelectorAll(".sc-dot")].map((dot) => ({
          n: Number(dot.dataset.term),
          in: dot.classList.contains("is-in")
        })),
        inCount: document.querySelectorAll(".sc-dot.is-in").length,
        outCount: document.querySelectorAll(".sc-dot.is-out").length
      };
    })()`
  );
  expect(geom.dots.length === TERMS, "scene draws 20 sequence dots", geom.dots.length);
  expect(Math.abs(geom.band / geom.width - 0.22) < 0.012, "band width is 22% of the axis", geom.band / geom.width);
  expect(Math.abs(geom.nOffset / geom.width - 0.2) < 0.012, "N marker sits at 1/5 = 20% of the axis", geom.nOffset / geom.width);
  expect(geom.inCount === 16 && geom.outCount === 4, "16 dots in band, 4 outside", geom);
  expect(
    geom.dots.every((dot) => dot.in === dot.n >= 5),
    "dot membership matches n ≥ N",
    geom.dots
  );
  expect(s.formulaText.includes("⌊") && s.formulaText.includes("= 5"), "formula shows the floor computation", s.formulaText);
  expect(s.boundaryText.includes("0.2000") && s.boundaryText.includes("0.2200"), "boundary line shows 1/5 < 0.22", s.boundaryText);
  expect(s.caption.includes("第 5 项") && s.caption.includes("16"), "caption names N and the inside count", s.caption);

  // ── cast switching ─────────────────────────────────────────
  section("active speaker switching");
  await click('[data-character="ali"]');
  s = await state(client);
  expect(s.activeSpeaker === "ali", "clicking 阿理 makes him the speaker", s.activeSpeaker);
  expect(s.bubble.speaker.includes("阿理"), "bubble speaker label changes to 阿理", s.bubble);
  let pressed = await client.evaluate(
    `[...document.querySelectorAll("[data-character]")].map((el) => el.getAttribute("aria-pressed"))`
  );
  expect(pressed.filter((value) => value === "true").length === 1 && pressed[1] === "true", "exactly 阿理 is pressed", pressed);
  await click('[data-character="xiaoxi"]');
  s = await state(client);
  expect(s.activeSpeaker === "xiaoxi" && s.bubble.speaker.includes("小析"), "clicking 小析 switches back", s.bubble);
  const visibleBoth = await client.evaluate(
    `[...document.querySelectorAll("[data-character]")].every((el) => {
      const r = el.getBoundingClientRect();
      return r.width > 80 && r.height > 100 && getComputedStyle(el).display !== "none";
    })`
  );
  expect(visibleBoth === true, "both characters remain visible after switching");

  // ── hint tiers via 阿理 ────────────────────────────────────
  section("three hint tiers, no XP for hints");
  const hintTexts = [];
  for (let tier = 1; tier <= 3; tier += 1) {
    await click("#hint-btn");
    s = await state(client);
    hintTexts.push(s.bubble.text);
    expect(s.hintTier === tier, `hint tier ${tier} is tracked`, s.hintTier);
    expect(s.activeSpeaker === "ali" && s.bubble.speaker.includes("阿理"), `hint ${tier} is spoken by 阿理`, s.bubble);
    expect(s.xp === 0, `hint ${tier} awards no XP`, s.xp);
  }
  expect(new Set(hintTexts).size === 3, "all three hint texts are different", hintTexts);
  s = await state(client);
  expect(s.hintButtonDisabled === true, "hint button disables after tier 3");

  // ── ε presets and strict boundaries ───────────────────────
  section("preset ε values and strict boundary");
  await click('.preset[data-preset="0.10"]');
  s = await state(client);
  expect(s.epsilon === 0.1 && s.N === 11 && s.inside === 10, "preset 0.10 → N = 11", s);
  expect(s.formulaHolds === true && s.formulaText.includes("= 11"), "0.10 formula and strict invariant hold", s.formulaText);
  expect(s.caption.includes("第 11 项"), "caption for 0.10 names the 11th term", s.caption);
  expect((await count(client, ".sc-dot.is-in")) === 10, "0.10 paints exactly 10 terms inside");

  await click('.preset[data-preset="0.25"]');
  s = await state(client);
  expect(s.N === 5 && s.formulaHolds === true, "preset 0.25 → N = 5", s);
  expect(s.caption.includes("1/ε = 4 是整数") && s.caption.includes("N = 5"), "0.25 explains the integer boundary", s.caption);
  expect(s.boundaryText.includes("0.2500") === true, "boundary line shows ε = 0.2500 in the strict check", s.boundaryText);
  const strict = await text(client, "#strict-out");
  expect(strict && strict.includes("0.2000") && strict.includes("0.2500"), "strict readout shows 1/5 < 0.25 and 1/4 ≥ 0.25", strict);

  await click('.preset[data-preset="0.01"]');
  s = await state(client);
  expect(s.N === 101 && s.inside === 0 && s.formulaHolds === true, "preset 0.01 → N = 101 outside the drawn terms", s);
  expect(s.caption.includes("画面外"), "0.01 caption says N is outside the picture", s.caption);
  expect((await count(client, ".sc-dot.is-in")) === 0, "0.01 paints zero terms inside");

  await click('.preset[data-preset="0.22"]');
  s = await state(client);
  expect(s.N === 5 && s.epsilon === 0.22, "preset 0.22 restores the default", s);

  // ── slider keyboard + large N ─────────────────────────────
  section("slider keyboard control");
  await focusSelector(client, "#eps-range");
  await pressKey(client, { key: "Home", code: "Home", windowsVirtualKeyCode: 36 });
  await sleep(120);
  s = await state(client);
  expect(s.epsilon === 0.001 && s.N === 1001, "Home sets ε = 0.001 → N = 1001", s);
  expect(s.caption.includes("无穷多项") || s.caption.includes("画面外"), "ε = 0.001 explains terms beyond the picture", s.caption);
  expect((await count(client, ".sc-dot.is-in")) === 0, "all 20 drawn terms are outside at ε = 0.001");
  const valueText = await client.evaluate('document.querySelector("#eps-range").getAttribute("aria-valuetext")');
  expect(valueText.includes("N = 1001"), "slider aria-valuetext reports N = 1001", valueText);
  await pressKey(client, { key: "End", code: "End", windowsVirtualKeyCode: 35 });
  await sleep(120);
  s = await state(client);
  expect(s.epsilon === 0.25 && s.N === 5, "End sets ε = 0.25 → N = 5", s);
  await click('.preset[data-preset="0.22"]');

  // ── play / step / pause ───────────────────────────────────
  section("play, pause, step");
  await click("#play-btn");
  await sleep(900);
  s = await state(client);
  expect(s.playing === true, "play is running", s.playing);
  expect(s.epsilon < 0.25 && s.epsilon >= 0.05, "play animates ε downward", s.epsilon);
  const playPressed = await client.evaluate('document.querySelector("#play-btn").getAttribute("aria-pressed")');
  expect(playPressed === "true", "play button exposes aria-pressed=true");
  await click("#play-btn");
  s = await state(client);
  expect(s.playing === false, "clicking play again pauses", s.playing);
  await click("#reset-btn");
  s = await state(client);
  expect(s.epsilon === 0.22 && s.N === 5, "reset returns to ε = 0.22");
  await click("#step-btn");
  s = await state(client);
  expect(s.epsilon === 0.18 && s.N === 6, "step tightens ε to 0.18 → N = 6", s);
  expect(s.formulaText.includes("= 6"), "step updates the formula", s.formulaText);
  await click("#reset-btn");

  // ── formal bridge ─────────────────────────────────────────
  section("collapsible formal definition bridge");
  const formalClosed = await client.evaluate('document.querySelector("#formal-card").open');
  expect(formalClosed === false, "formal card starts collapsed");
  await click("#formal-summary");
  const formal = await client.evaluate(
    `(() => ({
      open: document.querySelector("#formal-card").open,
      def: document.querySelector("#formal-def").textContent,
      why: document.querySelector("#formal-why").textContent,
      rows: document.querySelectorAll("#bridge-table .bridge-row").length,
      proof: document.querySelectorAll("#formal-proof li").length,
      caveat: document.querySelector("#formal-caveat").textContent
    }))()`
  );
  expect(formal.open === true, "clicking the summary opens the formal card");
  expect(formal.def.includes("∀") && formal.def.includes("∃") && formal.def.includes("< ε"), "formal definition has the quantifiers and strict inequality", formal.def);
  expect(formal.why.includes("N = ⌊1/ε⌋ + 1"), "why card shows N = ⌊1/ε⌋ + 1", formal.why);
  expect(formal.rows === 5 && formal.proof >= 3, "bridge table and proof sketch are present", formal);
  expect(formal.caveat.includes("内核尚未接入"), "caveat names the missing kernel grading", formal.caveat);
  await click("#formal-summary");

  // ── math invariants through the public scene API ──────────
  section("browser math invariants across ε");
  const values = [0.25, 0.22, 0.2, 0.18, 0.15, 0.12, 0.1, 0.08, 0.06, 0.05, 0.04, 0.03, 0.02, 0.01, 0.005, 0.001];
  let invariantFailure = "";
  for (const epsilon of values) {
    await client.evaluate(`window.SokoLearnDemo.scene.setEpsilon(${epsilon})`);
    await sleep(25);
    const current = await state(client);
    const expected = computeN(epsilon);
    const independent = Math.floor(1 / epsilon + 1e-12) + 1;
    if (current.N !== expected || current.N !== independent) {
      invariantFailure = `ε=${epsilon}: app N=${current.N}, lib N=${expected}, floor N=${independent}`;
      break;
    }
    if (!current.formulaHolds) {
      invariantFailure = `ε=${epsilon}: formulaHolds=false`;
      break;
    }
    const terms = current.terms;
    if (terms.length !== TERMS) {
      invariantFailure = `ε=${epsilon}: ${terms.length} terms`;
      break;
    }
    for (const term of terms) {
      if (term.inTail !== term.n >= current.N || term.inside !== 1 / term.n < epsilon) {
        invariantFailure = `ε=${epsilon}: term ${term.n} membership mismatch`;
        break;
      }
    }
    if (invariantFailure) break;
  }
  expect(invariantFailure === "", "app N and band membership agree with the independent formula for 16 ε values", invariantFailure);
  await click('.preset[data-preset="0.22"]');

  // ── layout at 1440 and 390, initial scene ──────────────────
  section("layout at 1440 and 390");
  await setViewport(client, { width: 1440, height: 1000, mobile: false });
  await sleep(220);
  let report = await assertLayout("initial-1440");
  expect(report.svg && report.svg.h / report.svg.w > 0.3, "desktop scene is not a thin strip", report.svg);
  expect(report.bubbleText && report.bubbleText.h >= 30, "desktop bubble text is visible", report.bubbleText);
  await screenshot("overview-1440", 1440, 1000);

  await setViewport(client, { width: 390, height: 844, mobile: true });
  await sleep(260);
  report = await assertLayout("initial-390");
  expect(report.svg && report.svg.h >= 190, "mobile scene keeps a usable height", report.svg);
  expect(report.cast.every((card) => card.w > 100), "both mobile cast cards are wide enough", report.cast);
  const clipped = await client.evaluate(
    `(() => {
      const el = document.querySelector(".bubble");
      return { scroll: el.scrollHeight, client: el.clientHeight };
    })()`
  );
  expect(clipped.scroll <= clipped.client + 1, "mobile bubble text is not clipped", clipped);
  await screenshot("cast-scene-390", 390, 844);

  // ── Q1: wrong answer no award, then correct ───────────────
  section("Q1 multiple choice");
  await setViewport(client, { width: 1440, height: 1000, mobile: false });
  await sleep(180);
  await click('[data-choice="n4"]');
  s = await state(client);
  expect(s.questionStatus === "pending", "Q1 wrong answer keeps the question open", s.questionStatus);
  expect(s.xp === 0 && s.streak === 0 && s.progress === 0, "Q1 wrong answer awards no XP and no progress", { xp: s.xp, streak: s.streak, progress: s.progress });
  let feedback = await text(client, "#feedback");
  expect(feedback && feedback.includes("0.25"), "Q1 wrong feedback shows a₄ = 0.25", feedback);
  expect((await client.evaluate('document.querySelector("#feedback").hidden')) === false, "Q1 wrong feedback is visible");

  await click('[data-choice="n5"]');
  s = await state(client);
  expect(s.questionStatus === "correct", "Q1 correct answer locks the question");
  expect(s.xp === 5 && s.streak === 1 && s.progress === 20, "Q1 retry + hints award the 5 XP floor", { xp: s.xp, streak: s.streak, progress: s.progress });
  expect((await count(client, "#next-question")) === 1, "Q1 exposes the next button");
  feedback = await text(client, "#feedback");
  expect(feedback && feedback.includes("⌊4.545…⌋"), "Q1 explanation shows the floor computation", feedback);
  await setViewport(client, { width: 390, height: 844, mobile: true });
  await sleep(240);
  await assertLayout("q1-correct-390");

  // ── Q2: true/false via keyboard ───────────────────────────
  section("Q2 true/false and keyboard activation");
  await click("#next-question");
  s = await state(client);
  expect(s.questionId === "q2" && s.questionType === "true-false", "Q2 is the true/false question", s);
  expect((await text(client, ".statement-box")) !== null, "Q2 shows the statement");
  await focusSelector(client, '[data-choice="false"]');
  await pressKey(client, { key: "Enter", code: "Enter", windowsVirtualKeyCode: 13, text: "\r" });
  await sleep(120);
  s = await state(client);
  expect(s.questionStatus === "correct", "Enter on the focused false button answers Q2");
  expect(s.xp === 25 && s.streak === 2 && s.progress === 40, "Q2 first-try award is +20 XP", { xp: s.xp, streak: s.streak, progress: s.progress });
  feedback = await text(client, "#feedback");
  expect(feedback && feedback.includes("0.25") && feedback.includes("0.22"), "Q2 explanation compares 0.25 and 0.22", feedback);
  await assertLayout("q2-correct-390");

  // ── Q3: click the first dot inside ────────────────────────
  section("Q3 click first dot inside the band");
  await click("#next-question");
  s = await state(client);
  expect(s.questionId === "q3" && s.questionType === "click-dot", "Q3 is the dot-click question", s);
  expect((await count(client, ".seq-dot")) === 20, "Q3 renders 20 sequence dots");
  expect((await count(client, ".dot-band")) === 1, "Q3 renders the ε band");
  const dotHit = await client.evaluate(
    `(() => {
      const dot = document.querySelector('[data-dot="5"]');
      const rect = dot.getBoundingClientRect();
      const at = document.elementFromPoint(rect.left + rect.width / 2, rect.top + rect.height / 2);
      return { hit: at === dot || dot.contains(at), label: dot.getAttribute("aria-label"), w: rect.width, h: rect.height };
    })()`
  );
  expect(dotHit.hit === true && dotHit.label.includes("第 5 项"), "the n = 5 dot is the top hit target and names a₅", dotHit);
  expect(dotHit.w >= 24 && dotHit.h >= 24, "dot targets meet the 24px target size", dotHit);
  await assertLayout("q3-390");
  await screenshot("game-q3-390", 390, 844);
  await click('[data-dot="4"]');
  s = await state(client);
  expect(s.questionStatus === "pending" && s.xp === 25 && s.streak === 0, "Q3 wrong dot awards nothing", { status: s.questionStatus, xp: s.xp, streak: s.streak });
  feedback = await text(client, "#feedback");
  expect(feedback && feedback.includes("0.2500") && feedback.includes("带外"), "Q3 wrong dot compares a₄ = 0.25 with ε", feedback);
  await click('[data-dot="5"]');
  s = await state(client);
  expect(s.questionStatus === "correct" && s.xp === 40 && s.streak === 1 && s.progress === 60, "Q3 correct dot awards XP and progress", { xp: s.xp, streak: s.streak, progress: s.progress });

  // ── Q4: order quantifier blocks ───────────────────────────
  section("Q4 quantifier ordering");
  await click("#next-question");
  s = await state(client);
  expect(s.questionId === "q4" && s.questionType === "order", "Q4 is the ordering question", s);
  expect((await count(client, ".order-slot")) === 4 && (await count(client, ".block-pool .order-block")) === 4, "Q4 has four slots and four blocks");
  for (const id of ["forall-eps", "forall-n", "exists-N", "dist"]) await click(`[data-block="${id}"]`);
  s = await state(client);
  expect(JSON.stringify(s.order) === JSON.stringify(["forall-eps", "forall-n", "exists-N", "dist"]), "Q4 click-to-place records the wrong order", s.order);
  await click("#check-order");
  s = await state(client);
  expect(s.questionStatus === "pending" && s.xp === 40 && s.streak === 0, "Q4 wrong order awards nothing", { status: s.questionStatus, xp: s.xp, streak: s.streak });
  feedback = await text(client, "#feedback");
  expect(feedback && feedback.includes("第 2 格") && feedback.includes("∃N"), "Q4 names the first wrong slot and the expected block", feedback);
  await assertLayout("q4-wrong-390");
  await screenshot("game-q4-390", 390, 844);
  await click("#clear-order");
  for (const id of ["forall-eps", "exists-N", "forall-n", "dist"]) await click(`[data-block="${id}"]`);
  await click("#check-order");
  s = await state(client);
  expect(s.questionStatus === "correct" && s.xp === 55 && s.streak === 1 && s.progress === 80, "Q4 correct order awards XP and progress", { xp: s.xp, streak: s.streak, progress: s.progress });
  expect((await count(client, "#check-order")) === 0, "Q4 hides the check button after success");
  expect((await count(client, "#clear-order")) === 0, "Q4 hides the clear button after success");

  // ── Q5: slider + N choice ─────────────────────────────────
  section("Q5 slider at ε = 0.10");
  await click("#next-question");
  s = await state(client);
  expect(s.questionId === "q5" && s.questionType === "slider-choice", "Q5 is the slider-choice question", s);
  expect((await count(client, "#q5-range")) === 1, "Q5 renders its own ε slider");
  await click('[data-choice="n11"]');
  s = await state(client);
  expect(s.questionStatus === "pending" && s.xp === 55 && s.streak === 1, "choosing N before setting ε is a setup nudge, not a wrong answer", { status: s.questionStatus, xp: s.xp, streak: s.streak });
  feedback = await text(client, "#feedback");
  expect(feedback && feedback.includes("先把 ε 调到 0.10"), "Q5 nudge asks for ε = 0.10 first", feedback);
  await client.evaluate(
    `(() => {
      const range = document.querySelector("#q5-range");
      range.value = "0.1";
      range.dispatchEvent(new Event("input", { bubbles: true }));
      return range.value;
    })()`
  );
  await sleep(100);
  s = await state(client);
  expect(s.slider === 0.1, "Q5 slider stores ε = 0.10", s.slider);
  expect((await text(client, "#q5-out")).includes("0.10"), "Q5 output shows ε = 0.10");
  await click('[data-choice="n10"]');
  s = await state(client);
  expect(s.questionStatus === "pending" && s.xp === 55 && s.streak === 0, "Q5 N = 10 is wrong and awards nothing", { status: s.questionStatus, xp: s.xp, streak: s.streak });
  feedback = await text(client, "#feedback");
  expect(feedback && feedback.includes("a₁₀ = 0.10") && feedback.includes("严格小于"), "Q5 explains the integer boundary", feedback);
  await click('[data-choice="n11"]');
  s = await state(client);
  expect(s.questionStatus === "correct" && s.xp === 70 && s.streak === 1 && s.progress === 100, "Q5 N = 11 awards XP and completes progress", { xp: s.xp, streak: s.streak, progress: s.progress });
  await assertLayout("q5-correct-390");

  // ── completion badge ──────────────────────────────────────
  section("completion badge");
  await click("#finish-demo");
  await sleep(350);
  s = await state(client);
  expect(s.completed === true && s.badgeVisible === true, "finishing shows the completion card", s);
  expect(s.correctCount === 5 && s.progress === 100, "all five questions are correct", { correct: s.correctCount, progress: s.progress });
  expect(s.xp === 70, "final XP reflects hints and retries", s.xp);
  const badge = await client.evaluate(
    `(() => ({
      title: document.querySelector("#completion-title")?.textContent,
      text: document.querySelector("#completion")?.innerText || "",
      recap: document.querySelectorAll(".recap li").length,
      questionHidden: document.querySelector("#question-root").hidden,
      progressNow: document.querySelector("#progress-track").getAttribute("aria-valuenow")
    }))()`
  );
  expect(badge.title === "你理解了 ε–N", "the badge title is exactly 你理解了 ε–N", badge.title);
  expect(badge.text.includes("5 / 5"), "badge detail says 5 / 5", badge.text);
  expect(badge.recap === 5, "completion recaps the five answers", badge.recap);
  expect(badge.questionHidden === true, "question card is hidden at completion");
  expect(badge.progressNow === "100", "progress bar reaches 100", badge.progressNow);
  await assertLayout("complete-390");
  await screenshot("complete-390", 390, 844);
  await setViewport(client, { width: 1440, height: 1000, mobile: false });
  await sleep(240);
  await assertLayout("complete-1440");
  await screenshot("complete-1440", 1440, 1000);

  // ── restart ───────────────────────────────────────────────
  section("restart");
  await click("#restart-demo");
  await sleep(200);
  s = await state(client);
  expect(s.completed === false && s.badgeVisible === false && s.xp === 0 && s.streak === 0 && s.correctCount === 0, "restart clears the game", s);
  expect(s.questionId === "q1" && s.questionStatus === "pending" && s.epsilon === 0.22, "restart returns to Q1 and ε = 0.22", s);

  // ── reduced motion ────────────────────────────────────────
  section("reduced motion");
  await setReducedMotion(client, true);
  await reload(client);
  await waitFor(client, "window.SokoLearnDemo && window.SokoLearnDemo.getState");
  await sleep(200);
  s = await state(client);
  expect(s.reducedMotion === true, "reduced-motion preference is read on load", s.reducedMotion);
  const animation = await client.evaluate(
    `(() => {
      const figure = document.querySelector(".av-figure");
      const style = getComputedStyle(figure);
      return { name: style.animationName, duration: style.animationDuration };
    })()`
  );
  expect(animation.name === "none" || animation.duration === "0.001ms", "avatar animation is disabled under reduced motion", animation);
  await click("#play-btn");
  await sleep(250);
  s = await state(client);
  expect(s.playing === false && s.epsilon === 0.05, "play jumps to the final ε instead of animating", { playing: s.playing, epsilon: s.epsilon });
  await setReducedMotion(client, false);
  await reload(client);
  await waitFor(client, "window.SokoLearnDemo && window.SokoLearnDemo.getState");
  await sleep(180);
  s = await state(client);
  expect(s.reducedMotion === false, "no-preference is restored after the reduced-motion probe");

  // ── network + console ─────────────────────────────────────
  section("network, console, and page errors");
  const requests = client.events
    .filter((event) => event.method === "Network.requestWillBeSent")
    .map((event) => event.params.request.url);
  details.requests = requests;
  const external = requests.filter((url) => !/^file:/i.test(url) && !/^data:/i.test(url));
  expect(external.length === 0, "no external http(s) requests", external);
  expect(requests.some((url) => /^file:/i.test(url)), "the page was loaded from file://", requests.slice(0, 3));
  const failedLoads = client.events.filter((event) => event.method === "Network.loadingFailed");
  expect(failedLoads.length === 0, "no failed network loads", failedLoads.slice(0, 2));
  const pageErrors = client.events
    .filter((event) => event.method === "Runtime.exceptionThrown")
    .map((event) => event.params.exceptionDetails?.exception?.description || event.params.exceptionDetails?.text);
  const consoleErrors = client.events
    .filter((event) => event.method === "Runtime.consoleAPICalled" && event.params.type === "error")
    .map((event) => event.params.args?.map((arg) => arg.value || arg.description).join(" "));
  const logErrors = client.events
    .filter((event) => event.method === "Log.entryAdded" && event.params.entry?.level === "error")
    .map((event) => event.params.entry.text);
  expect(pageErrors.length === 0, "no uncaught page exceptions", pageErrors);
  expect(consoleErrors.length === 0, "no console errors", consoleErrors);
  expect(logErrors.length === 0, "no browser log errors", logErrors);

  return {
    ok: failures.length === 0,
    checks,
    failures,
    details,
  };
}

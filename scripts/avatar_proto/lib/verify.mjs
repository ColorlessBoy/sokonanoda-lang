// scripts/avatar_proto/lib/verify.mjs — static + browser acceptance checks.
//
// Everything here is deliberately observable: file bytes, computed styles,
// bounding boxes, aria state, and real CDP mouse clicks. No "looks fine".

import { existsSync, readFileSync } from "node:fs";
import { createHash } from "node:crypto";
import {
  sleep,
  setViewport,
  clickSelector,
  setReducedMotion,
  reload,
  waitFor,
} from "./cdp.mjs";

export const EXPRESSIONS = ["neutral", "curious", "thinking", "happy", "oops"];
export const EXPR_LABEL = {
  neutral: "平静",
  curious: "好奇",
  thinking: "思考",
  happy: "开心",
  oops: "哎呀",
};
const SAMPLE_LINES = [
  "这条带子就是允许误差 ε。",
  "从第 N 项开始，后面的点都进来了吗？",
  "ε 再小一点，N 会怎么变？",
];

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
    .trim();
}

function sha(buffer) {
  return createHash("sha256").update(buffer).digest("hex").slice(0, 16);
}

// ── file layer ────────────────────────────────────────────────
export function staticChecks({ htmlPath, previewPath }) {
  const failures = [];
  const info = {};
  const check = (condition, message) => {
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

  check(/^<!doctype html>/i.test(html.trimStart()), "starts with <!doctype html>");
  check(/<html lang="zh-CN">/.test(html), "lang is zh-CN");
  check(
    /<meta name="viewport" content="width=device-width, initial-scale=1">/.test(html),
    "has a responsive viewport meta"
  );
  check(/<title>[^<]*小析[^<]*<\/title>/.test(html), "title names 小析");
  check(/<meta name="description"/.test(html), "has a meta description");
  check(/window\.SokoAvatar/.test(html), "exposes window.SokoAvatar for checks/embedding");
  check(/prefers-reduced-motion/.test(html), "declares prefers-reduced-motion handling");
  check(/data-motion=/.test(html) || /dataset\.motion/.test(html), "exposes a motion state");
  check(/aria-live="polite"/.test(html), "has a polite live region");
  check(/role="status"/.test(html), "the speech bubble is a status region");
  check(/role="img"/.test(html), "the character is exposed as an image");
  check(/aria-label="小析/.test(html), "the character has an accessible name");

  // No external resources. Inline SVG in HTML needs no xmlns URL, so the page
  // should contain literally zero absolute URLs.
  check(!/https?:\/\//i.test(html), "no absolute http(s) URL anywhere");
  check(!/url\(\s*["']?(?:https?:|\/\/)/i.test(html), "no external CSS url()");
  check(!/@import\b/i.test(html), "no CSS @import");
  check(!/@font-face\b/i.test(html), "no external @font-face");
  check(!/<link\b/i.test(html), "no <link> elements");
  check(!/<img\b/i.test(html), "no <img> elements");
  check(!/<iframe\b|<video\b|<audio\b|<object\b|<embed\b|<source\b/i.test(html), "no media/iframe elements");
  check(!/<script\b[^>]*\bsrc\s*=/i.test(html), "no external script src");
  check(!/\bfetch\s*\(/i.test(html), "does not call fetch()");
  check(!/\bXMLHttpRequest\b/i.test(html), "does not use XMLHttpRequest");
  check(!/\bWebSocket\b|\bEventSource\b/i.test(html), "does not open a socket");
  check(!/\bnew\s+Worker\b/i.test(html), "does not start a Worker");
  check(!/\bimport\s*\(/i.test(html), "does not use dynamic import()");

  // Relative-only links; the page must not need a network.
  const hrefs = [...html.matchAll(/\bhref\s*=\s*"([^"]*)"/gi)].map((m) => m[1]);
  check(
    hrefs.every((href) => href.startsWith("#") || href.startsWith("?") || !/^[a-z]+:/i.test(href)),
    `all hrefs are relative (#/?/file): ${hrefs.join(", ")}`
  );

  // No free-form input: one native checkbox, no textarea/contenteditable/form.
  check(!/<textarea\b/i.test(html), "no textarea");
  check(!/\bcontenteditable\b/i.test(html), "no contenteditable");
  check(!/<form\b/i.test(html), "no form submit path");
  const inputs = [...html.matchAll(/<input\b[^>]*>/gi)].map((m) => m[0]);
  check(inputs.length === 1, `exactly one <input> (got ${inputs.length})`);
  check(inputs.length === 1 && /type="checkbox"/i.test(inputs[0]), "the only input is a checkbox");
  check(inputs.length === 1 && /id="motion-toggle"/.test(inputs[0]), "the checkbox is the motion switch");

  // Controls and required states.
  check(EXPRESSIONS.every((name) => html.includes(`data-expr-btn="${name}"`)), "all 5 expression buttons exist");
  check(
    EXPRESSIONS.every((name) => new RegExp(`data-expression="${name}"`).test(html) || html.includes(`"${name}":`)),
    "all 5 expression names exist in the state machine"
  );
  check((html.match(/data-expr-btn="/g) || []).length === 5, "exactly 5 expression buttons");
  check((html.match(/data-persona-btn="/g) || []).length === 2, "exactly 2 persona buttons");
  check((html.match(/data-line-btn="/g) || []).length === 3, "exactly 3 sample-line buttons");
  check((html.match(/data-sheet-card="/g) || []).length === 5, "character sheet has 5 cards");
  check(/id="avatar-toggle"/.test(html) && /aria-pressed/.test(html), "hide/show control is a pressed-state button");
  check(/id="wave-btn"/.test(html), "has an explicit wave control");
  check(/id="motion-toggle"/.test(html), "has a reduce-motion control");
  check(/data-focus="band"/.test(html), "scene has a band focus state");
  check(/ghost-band/.test(html) && /ghost-n-line/.test(html), "scene has a half-epsilon what-if layer");
  check(/data-n="10"/.test(html), "sequence dots include n=10");

  // Character bible sections + voice samples.
  const text = visibleText(html);
  const has = (needle) => text.includes(needle);
  for (const heading of ["身份", "声音", "状态", "外观", "出场", "接入"]) {
    check(has(heading), `bible section present: ${heading}`);
  }
  for (const moment of ["开场", "提示", "答对", "没接住", "收尾"]) {
    check(has(moment), `voice moment present: ${moment}`);
  }
  check(has("小析") && has("同伴型引导者"), "bible names the role as peer-guide");
  check(has("不假装人类"), "bible forbids fake-human claims");
  check(has("短句") && has("一个数学动作"), "bible states the one-action-per-line voice rule");
  check(has("笔记本") && has("铅笔"), "bible names the notebook + pencil props");
  check(has("角落") && has("隐藏"), "bible states corner placement + hideability");
  check(has("5–10s") && has("开场"), "bible covers the 5–10s intro");
  check(has("Do") && has("Don’t"), "bible has do/don't rules");
  check(has("眼睛") && has("成年 / 中性"), "bible has appearance rules + adult neutral register");
  check(has("Web Component") && has("Rive") && has("Lottie") && has("Remotion"), "bible covers future embedding paths");

  // The three requested sample lines exist verbatim (line 1 also renders by default).
  for (const line of SAMPLE_LINES) {
    check(html.includes(line), `sample speech line is present: ${line}`);
  }
  check(html.includes("从第 N 项开始，后面的点都进来了。"), "mentor variant of line 2 exists");
  check(html.includes("先估一眼"), "mentor variant of line 3 exists");

  // The epsilon-N math is exact and strict (ε=0.22 ⇒ N=5; ε=0.11 ⇒ N=10).
  check(has("ε = 0.22") && has("0.22"), "scene states epsilon = 0.22");
  check(has("N = 5") && has("N = 5"), "scene states N = 5");
  check(html.includes("a₅ = 0.2") && html.includes("a₄ = 0.25"), "scene shows the boundary terms a5/a4");
  check(html.includes("ε 减半到 0.11") && html.includes("N 从 5 变成 10"), "what-if states epsilon/2 ⇒ N=10");
  check(has("1/N < ε"), "bible hint states the strict inequality 1/N < ε");
  check(has("∀ ε > 0, ∃ N, ∀ n ≥ N"), "scene shows the formal quantifier string");
  check(has("序列本身没有最后一项"), "scene is honest that only 10 terms are drawn");
  check(has("不是课程单元") && has("scripts/avatar_proto"), "page labels itself as a probe, not a course unit");

  // Palette (the probe's three accents + surfaces).
  for (const hex of ["#17324d", "#1f8a7a", "#e9a23b", "#f4e3cb", "#fffdf8"]) {
    check(html.toLowerCase().includes(hex), `palette token present: ${hex}`);
  }

  if (!existsSync(previewPath)) {
    failures.push(`preview/index.html missing at ${previewPath}; run node scripts/avatar_proto/build.mjs`);
  } else {
    const preview = readFileSync(previewPath);
    info.previewBytes = preview.length;
    info.previewSha256 = sha(preview);
    check(preview.equals(source), "preview/index.html is a byte-identical generated copy");
  }

  return { ok: failures.length === 0, failures, info };
}

// ── browser layer ─────────────────────────────────────────────
function box(rect) {
  return rect && {
    x: rect.x,
    y: rect.y,
    w: rect.width,
    h: rect.height,
    right: rect.right,
    bottom: rect.bottom,
    top: rect.top,
    left: rect.left,
  };
}

function intersects(a, b) {
  return a && b && a.left < b.right - 1 && b.left < a.right - 1 && a.top < b.bottom - 1 && b.top < a.bottom - 1;
}

function overlapsHoriz(a, b) {
  return a.left < b.right - 0.5 && b.left < a.right - 0.5;
}

async function snapshot(client) {
  return client.evaluate(`(() => {
    const $ = (sel) => document.querySelector(sel);
    const $$ = (sel) => [...document.querySelectorAll(sel)];
    const r = (sel) => { const el = $(sel); if (!el) return null; const b = el.getBoundingClientRect(); return { x:+b.x.toFixed(1), y:+b.y.toFixed(1), width:+b.width.toFixed(1), height:+b.height.toFixed(1), top:+b.top.toFixed(1), bottom:+b.bottom.toFixed(1), left:+b.left.toFixed(1), right:+b.right.toFixed(1) }; };
    const cs = (sel, prop) => { const el = $(sel); return el ? getComputedStyle(el)[prop] : null; };
    const mouth = {};
    for (const e of ${JSON.stringify(EXPRESSIONS)}) mouth[e] = parseFloat(cs('.mouth-' + e, 'opacity'));
    const controls = $$('button, input, a.quiet-link, .switch').filter((el) => {
      const b = el.getBoundingClientRect();
      return b.width > 2 && b.height > 2;
    }).map((el) => {
      const b = el.getBoundingClientRect();
      return {
        tag: el.tagName,
        name: (el.getAttribute('aria-label') || el.textContent || el.id || '').trim().replace(/\\s+/g, ' ').slice(0, 40),
        w: +b.width.toFixed(1), h: +b.height.toFixed(1),
      };
    });
    const buttons = $$('button').filter((el) => {
      const b = el.getBoundingClientRect();
      return b.width > 2 && b.height > 2;
    }).map((el) => ({
      cls: el.className,
      pressed: el.getAttribute('aria-pressed'),
      disabled: el.disabled,
      name: (el.textContent || '').trim().replace(/\\s+/g, ' ').slice(0, 30),
    }));
    const labels = $$('.dot-label').filter((el) => getComputedStyle(el).display !== 'none').map((el) => {
      const b = el.getBoundingClientRect();
      return { text: el.textContent, left:+b.left.toFixed(1), right:+b.right.toFixed(1), top:+b.top.toFixed(1), bottom:+b.bottom.toFixed(1) };
    });
    const all = $$('*');
    const overflow = all.filter((el) => {
      const b = el.getBoundingClientRect();
      if (!b.width || !b.height) return false;
      if (getComputedStyle(el).position === 'fixed') return false;
      return b.right > innerWidth + 0.6 || b.left < -0.6;
    }).slice(0, 8).map((el) => ({ tag: el.tagName, cls: String(el.className && el.className.baseVal !== undefined ? el.className.baseVal : el.className).slice(0, 50) }));
    const bubble = $('#bubble');
    const bubbleText = $('#bubble-text');
    return {
      vw: innerWidth, vh: innerHeight,
      scrollWidth: document.documentElement.scrollWidth,
      bodyScrollWidth: document.body.scrollWidth,
      state: window.SokoAvatar.state(),
      data: { persona: document.body.dataset.persona, line: document.body.dataset.line, motion: document.body.dataset.motion, avatar: document.body.dataset.avatar },
      bubble: {
        text: bubbleText.textContent,
        speaker: $('#bubble-speaker').textContent,
        role: bubble.getAttribute('role'),
        live: bubble.getAttribute('aria-live'),
        rect: r('#bubble'),
        clipped: bubbleText.scrollWidth > bubbleText.clientWidth + 1 || bubbleText.scrollHeight > bubbleText.clientHeight + 1,
      },
      chip: $('#persona-chip').textContent,
      stateUse: $('#state-use').textContent,
      note: $('#scene-note-text').textContent,
      motionNote: $('#motion-note').textContent,
      focus: $('#plot').dataset.focus,
      avatar: {
        expr: $('#xiao-xi').dataset.expression,
        blink: $('#xiao-xi').dataset.blink,
        aria: $('#xiao-xi').getAttribute('aria-label'),
        rect: r('#xiao-xi'),
        waving: $('#xiao-xi').classList.contains('is-waving'),
        waveArm: parseFloat(cs('.wave-arm', 'opacity')),
        armL: parseFloat(cs('.arm-l', 'opacity')),
      },
      mouth,
      eyes: { open: parseFloat(cs('.eyes-open', 'opacity')), closed: parseFloat(cs('.eyes-closed', 'opacity')), happy: parseFloat(cs('.eyes-happy', 'opacity')) },
      blush: parseFloat(cs('.blush', 'opacity')),
      ghost: parseFloat(cs('.ghost-band', 'opacity')),
      bandAnimation: cs('.band', 'animationName'),
      figureAnimation: cs('.figure', 'animationName'),
      anims: document.getAnimations().length,
      companion: { hidden: $('#companion').hidden, display: cs('#companion', 'display'), rect: r('#companion') },
      plot: r('#plot'), plotMain: r('.plot-main'), stage: r('.stage'), noteRect: r('.scene-note'), linebar: r('.linebar'),
      bubbleRect: r('#bubble'), avatarWrap: r('.avatar-wrap'),
      band: r('.band'), nLine: r('.n-line'), bandTag: r('.band-tag'), nTag: r('.n-tag'),
      dots: { total: $$('.dot').length, in: $$('.dot.in').length, out: $$('.dot.out').length, labels },
      buttons,
      toggle: {
        hideLabel: $('#avatar-toggle').textContent,
        hidePressed: $('#avatar-toggle').getAttribute('aria-pressed'),
        waveDisabled: $('#wave-btn').disabled,
        motionChecked: $('#motion-toggle').checked,
        motionLabel: r('.switch'),
      },
      voiceHighlight: {
        peer: cs('.voice-table .col-peer', 'backgroundColor'),
        mentor: cs('.voice-table .col-mentor', 'backgroundColor'),
      },
      controls,
      overflow,
    };
  })()`);
}

const click = async (client, selector) => {
  await clickSelector(client, selector);
  await sleep(170);
};

export async function browserChecks({ client, log = () => {} }) {
  const failures = [];
  let checks = 0;
  const details = {};
  const expect = (condition, label, extra) => {
    checks += 1;
    if (!condition) {
      failures.push(label + (extra === undefined ? "" : ` :: ${typeof extra === "string" ? extra : JSON.stringify(extra)}`));
    }
  };
  const section = (name) => log(`· ${name}`);

  // Let the one-shot greeting wave finish; it is deliberately not a loop.
  await sleep(2800);

  // ── initial state + math scene ──────────────────────────────
  section("initial state + epsilon-N geometry");
  let s = await snapshot(client);
  details.initial = { state: s.state, scrollWidth: s.scrollWidth, anims: s.anims };
  expect(s.state.persona === "peer", "initial persona is peer", s.state);
  expect(s.state.expression === "neutral", "initial expression is neutral", s.state);
  expect(s.state.line === 0, "initial line is the band line", s.state);
  expect(s.bubble.text === SAMPLE_LINES[0], "initial bubble shows the epsilon line", s.bubble.text);
  expect(s.focus === "band", "initial scene focus is the band", s.focus);
  expect(s.dots.total === 10 && s.dots.in === 6 && s.dots.out === 4, "scene draws 10 dots: 6 in, 4 out", s.dots);
  expect(s.band && s.plotMain, "band and plot exist");
  const bandPct = s.band.width / s.plotMain.width;
  const nPct = (s.nLine.left - s.plotMain.left) / s.plotMain.width;
  expect(Math.abs(bandPct - 0.22) < 0.012, "epsilon band is 22% of the axis", bandPct);
  expect(Math.abs(nPct - 0.2) < 0.012, "N marker sits at 1/5 = 20% of the axis", nPct);
  expect(!intersects(s.plot, s.companion.rect), "companion never overlaps the math plot", { plot: s.plot, companion: s.companion.rect });
  expect(s.bubbleRect.left >= s.stage.left && s.bubbleRect.right <= s.stage.right, "bubble is inside the stage", s.bubbleRect);
  expect(!s.bubble.clipped, "bubble text is not clipped");
  expect(s.scrollWidth <= s.vw + 0.5, "1440px page has no horizontal overflow", s.scrollWidth);
  expect(s.overflow.length === 0, "1440px has no out-of-viewport element", s.overflow);
  expect(s.bandTag && s.nTag && !intersects(s.bandTag, s.nTag), "band and N callouts do not collide", { b: s.bandTag, n: s.nTag });
  expect(s.mouth.neutral > 0.9 && s.mouth.happy < 0.1, "neutral mouth is the default", s.mouth);
  expect(s.controls.length >= 10, "the page exposes the full control surface", s.controls.length);

  // ── line switcher + personas ───────────────────────────────
  section("line switcher + persona microcopy");
  await click(client, '[data-line-btn="1"]');
  s = await snapshot(client);
  expect(s.state.line === 1, "line 2 sets state.line=1", s.state);
  expect(s.bubble.text === SAMPLE_LINES[1], "peer line 2 is the question", s.bubble.text);
  expect(s.focus === "tail", "line 2 focuses the N/tail", s.focus);
  expect(s.avatar.expr === "curious", "line 2 suggests the curious expression", s.avatar.expr);
  expect(s.ghost < 0.1, "the half-epsilon layer is hidden on line 2", s.ghost);
  expect(s.buttons.filter((b) => b.cls.includes("line-btn") && b.pressed === "true").length === 1, "exactly one line button is pressed", s.buttons);

  const peerExpression = s.avatar.expr;
  await click(client, '[data-persona-btn="mentor"]');
  s = await snapshot(client);
  expect(s.state.persona === "mentor", "mentor button flips the persona", s.state);
  expect(s.bubble.text === "从第 N 项开始，后面的点都进来了。", "mentor line 2 becomes a statement", s.bubble.text);
  expect(s.bubble.speaker.includes("导师"), "bubble speaker says mentor", s.bubble.speaker);
  expect(s.chip === "导师模式", "stage chip says mentor", s.chip);
  expect(s.avatar.expr === peerExpression, "persona toggle does not change the shared visual", { before: peerExpression, after: s.avatar.expr });
  expect(s.buttons.filter((b) => b.cls.includes("persona-btn") && b.pressed === "true").length === 1, "exactly one persona is pressed", s.buttons);
  expect(s.voiceHighlight.mentor !== s.voiceHighlight.peer, "bible voice table highlights the active persona", s.voiceHighlight);
  expect(!s.bubble.clipped, "mentor bubble is not clipped");

  await click(client, '[data-persona-btn="peer"]');
  s = await snapshot(client);
  expect(s.state.persona === "peer" && s.bubble.text === SAMPLE_LINES[1], "peer button restores the question", s.bubble.text);

  // ── what-if line ───────────────────────────────────────────
  section("epsilon halves: what-if scene");
  await click(client, '[data-line-btn="2"]');
  s = await snapshot(client);
  expect(s.bubble.text === SAMPLE_LINES[2], "line 3 is the what-if question", s.bubble.text);
  expect(s.focus === "whatif", "line 3 focuses the what-if layer", s.focus);
  expect(s.ghost > 0.9, "half-epsilon band becomes visible", s.ghost);
  expect(s.note.includes("0.11") && s.note.includes("10"), "note explains epsilon/2 = 0.11 and N=10", s.note);
  expect(s.avatar.expr === "curious", "peer line 3 uses the curious expression", s.avatar.expr);

  await click(client, '[data-line-btn="0"]');
  s = await snapshot(client);
  expect(s.focus === "band" && s.ghost < 0.1, "line 1 returns to the band focus", { focus: s.focus, ghost: s.ghost });
  expect(s.avatar.expr === "neutral", "line 1 returns to neutral", s.avatar.expr);

  // ── all five expressions ───────────────────────────────────
  section("five expression states");
  for (const name of EXPRESSIONS) {
    await click(client, `[data-expr-btn="${name}"]`);
    s = await snapshot(client);
    expect(s.state.expression === name && s.avatar.expr === name, `expression button sets ${name}`, s.avatar.expr);
    expect(s.mouth[name] > 0.9, `${name} mouth is visible`, s.mouth);
    expect(EXPRESSIONS.filter((other) => other !== name).every((other) => s.mouth[other] < 0.1), `${name} hides the other mouths`, s.mouth);
    expect(s.buttons.filter((b) => b.cls.includes("expr-btn") && b.pressed === "true").length === 1, `exactly one expression is pressed for ${name}`, s.buttons);
    expect(s.stateUse.includes(EXPR_LABEL[name]), `state-use names ${EXPR_LABEL[name]}`, s.stateUse);
    expect(s.avatar.aria.includes(EXPR_LABEL[name]), `avatar aria-label names ${EXPR_LABEL[name]}`, s.avatar.aria);
    if (name === "happy") {
      expect(s.eyes.happy > 0.9 && s.eyes.open < 0.1, "happy uses arc eyes", s.eyes);
      expect(s.blush > 0.3, "happy shows blush", s.blush);
    } else {
      expect(s.eyes.open > 0.9 && s.eyes.happy < 0.1, `${name} uses open eyes`, s.eyes);
    }
    if (name === "oops") expect(s.blush > 0.1, "oops shows a little blush", s.blush);
  }
  await click(client, '[data-expr-btn="neutral"]');

  // ── blink (same path as the idle timer) ────────────────────
  section("idle blink");
  const didBlink = await client.evaluate("window.SokoAvatar.blink()");
  expect(didBlink === true, "blink is available when motion is on");
  await sleep(70);
  s = await snapshot(client);
  expect(s.avatar.blink === "1", "blink closes the eyes", s.avatar.blink);
  expect(s.eyes.closed > 0.9 && s.eyes.open < 0.1, "closed eye group is visible during blink", s.eyes);
  await sleep(180);
  s = await snapshot(client);
  expect(s.avatar.blink === "0" && s.eyes.open > 0.9, "blink reopens the eyes", { blink: s.avatar.blink, eyes: s.eyes });

  // ── wave (real button) ─────────────────────────────────────
  section("wave control");
  await click(client, "#wave-btn");
  s = await snapshot(client);
  expect(s.avatar.waving === true, "wave button starts a wave");
  expect(s.avatar.waveArm > 0.9 && s.avatar.armL < 0.1, "wave swaps in the raised arm", { waveArm: s.avatar.waveArm, armL: s.avatar.armL });
  await sleep(1900);
  s = await snapshot(client);
  expect(s.avatar.waving === false, "wave ends by itself");
  expect(s.avatar.waveArm < 0.1 && s.avatar.armL > 0.9, "base arm returns after wave", { waveArm: s.avatar.waveArm, armL: s.avatar.armL });

  // ── hide / show ────────────────────────────────────────────
  section("hide / show avatar");
  const plotBefore = (await snapshot(client)).plot;
  await click(client, "#avatar-toggle");
  s = await snapshot(client);
  expect(s.companion.hidden === true && s.companion.display === "none", "hide removes the companion", s.companion);
  expect(s.data.avatar === "hidden", "body records hidden avatar", s.data);
  expect(s.toggle.hidePressed === "false" && s.toggle.hideLabel === "显示小析", "hide button flips label/pressed", s.toggle);
  expect(s.toggle.waveDisabled === true, "wave is disabled while hidden", s.toggle);
  expect(JSON.stringify(s.plot) === JSON.stringify(plotBefore), "hiding the avatar does not move the math", { before: plotBefore, after: s.plot });
  expect(!s.bubbleRect || s.bubbleRect.width === 0, "hidden bubble has no box", s.bubbleRect);
  expect(s.scrollWidth <= s.vw + 0.5, "hidden state has no overflow", s.scrollWidth);
  await click(client, "#avatar-toggle");
  s = await snapshot(client);
  expect(s.companion.hidden === false && s.data.avatar === "shown", "show restores the companion", s.companion);
  expect(s.toggle.hidePressed === "true" && s.toggle.hideLabel === "隐藏小析", "show button flips back", s.toggle);
  expect(s.toggle.waveDisabled === false, "wave is available again", s.toggle);

  // ── reduce motion ──────────────────────────────────────────
  section("reduce motion");
  await click(client, ".switch");
  s = await snapshot(client);
  expect(s.data.motion === "off" && s.toggle.motionChecked === true, "switch turns reduce-motion on", { motion: s.data.motion, checked: s.toggle.motionChecked });
  expect(s.anims === 0, "reduce motion leaves zero running animations", s.anims);
  expect(s.figureAnimation === "none", "breathing animation is disabled", s.figureAnimation);
  await click(client, "#wave-btn");
  s = await snapshot(client);
  expect(s.avatar.waving === false, "reduce motion suppresses the wave");
  expect(s.motionNote.includes("减少动态"), "the page explains that the wave was skipped", s.motionNote);
  const reducedBlink = await client.evaluate("window.SokoAvatar.blink()");
  expect(reducedBlink === false, "reduce motion suppresses blinking", reducedBlink);
  await click(client, ".switch");
  s = await snapshot(client);
  expect(s.data.motion === "on" && s.toggle.motionChecked === false, "switch restores motion", { motion: s.data.motion, checked: s.toggle.motionChecked });
  expect(s.anims > 0, "motion restores running animations", s.anims);

  // ── accessibility basics (desktop) ─────────────────────────
  section("accessibility");
  s = await snapshot(client);
  expect(s.bubble.role === "status" && s.bubble.live === "polite", "bubble remains a polite status", { role: s.bubble.role, live: s.bubble.live });
  expect(s.avatar.aria.startsWith("小析"), "avatar has an accessible name", s.avatar.aria);
  const visibleButtons = s.controls.filter((c) => c.tag === "BUTTON");
  expect(visibleButtons.every((c) => c.name.length > 0), "every visible button has an accessible name", visibleButtons);
  expect(visibleButtons.every((c) => c.h >= 36), "every visible button is at least 36px tall", visibleButtons);
  expect(s.toggle.motionLabel && s.toggle.motionLabel.height >= 36, "the switch label has a 36px+ hit area", s.toggle.motionLabel);
  expect(s.buttons.every((b) => b.pressed === null || b.pressed === "true" || b.pressed === "false"), "all aria-pressed values are valid", s.buttons);
  expect(s.buttons.filter((b) => b.pressed !== null).length >= 10, "pressed-state controls are wired", s.buttons.length);
  expect(s.dots.labels.length === 5, "desktop shows the five key dot labels", s.dots.labels);
  for (let i = 0; i < s.dots.labels.length; i += 1) {
    for (let j = i + 1; j < s.dots.labels.length; j += 1) {
      expect(
        !overlapsHoriz(s.dots.labels[i], s.dots.labels[j]),
        `dot labels do not overlap: ${s.dots.labels[i].text}/${s.dots.labels[j].text}`,
        [s.dots.labels[i], s.dots.labels[j]]
      );
    }
  }

  // ── 390px layout + controls ────────────────────────────────
  section("390px layout");
  await setViewport(client, { width: 390, height: 844, dpr: 1, mobile: true });
  await sleep(300);
  s = await snapshot(client);
  details.mobile = { scrollWidth: s.scrollWidth, companion: s.companion.rect, plot: s.plot };
  expect(s.scrollWidth <= 390.5, "390px page has no horizontal overflow", s.scrollWidth);
  expect(s.bodyScrollWidth <= 390.5, "390px body has no horizontal overflow", s.bodyScrollWidth);
  expect(s.overflow.length === 0, "390px has no out-of-viewport element", s.overflow);
  expect(s.plot.right <= 390 && s.plot.left >= 0, "mobile plot stays inside the viewport", s.plot);
  expect(!intersects(s.plot, s.companion.rect), "mobile companion never overlaps the math plot", { plot: s.plot, companion: s.companion.rect });
  expect(s.bubbleRect.right <= 390 && s.bubbleRect.left >= 0, "mobile bubble is inside the viewport", s.bubbleRect);
  expect(!s.bubble.clipped, "mobile bubble text is not clipped");
  expect(s.dots.labels.length === 4, "mobile keeps 4 non-colliding dot labels", s.dots.labels);
  for (let i = 0; i < s.dots.labels.length; i += 1) {
    for (let j = i + 1; j < s.dots.labels.length; j += 1) {
      expect(!overlapsHoriz(s.dots.labels[i], s.dots.labels[j]), `mobile dot labels do not overlap: ${s.dots.labels[i].text}/${s.dots.labels[j].text}`, [s.dots.labels[i], s.dots.labels[j]]);
    }
  }
  expect(s.controls.filter((c) => c.tag === "BUTTON").every((c) => c.h >= 40), "mobile buttons are at least 40px tall", s.controls);
  await click(client, '[data-line-btn="2"]');
  s = await snapshot(client);
  expect(s.ghost > 0.9, "mobile line 3 shows the what-if band", s.ghost);
  expect(s.scrollWidth <= 390.5, "mobile what-if has no overflow", s.scrollWidth);
  await click(client, '[data-persona-btn="mentor"]');
  s = await snapshot(client);
  expect(s.state.persona === "mentor" && s.bubble.speaker.includes("导师"), "mobile persona toggle works", s.bubble);
  expect(s.bubble.text.includes("先估一眼"), "mobile mentor copy is visible", s.bubble.text);
  expect(s.scrollWidth <= 390.5, "mobile mentor state has no overflow", s.scrollWidth);
  await click(client, "#avatar-toggle");
  s = await snapshot(client);
  expect(s.companion.hidden === true, "mobile hide works", s.companion);
  expect(s.scrollWidth <= 390.5, "mobile hidden state has no overflow", s.scrollWidth);
  await click(client, "#avatar-toggle");
  s = await snapshot(client);
  expect(s.companion.hidden === false, "mobile show works", s.companion);

  // ── system reduced motion is honored at load ───────────────
  section("system prefers-reduced-motion");
  await setReducedMotion(client, true);
  await reload(client);
  await sleep(200);
  await waitFor(client, "document.body && document.body.dataset.ready === '1'", 8000);
  await sleep(250);
  s = await snapshot(client);
  expect(s.data.motion === "off", "system reduce-motion initializes the page in reduced mode", s.data);
  expect(s.toggle.motionChecked === true, "system reduce-motion checks the switch", s.toggle.motionChecked);
  expect(s.anims === 0, "system reduce-motion leaves zero animations", s.anims);
  expect(s.avatar.blink === "0", "system reduce-motion disables blinking", s.avatar.blink);
  await click(client, '[data-line-btn="1"]');
  s = await snapshot(client);
  expect(s.bubble.text === SAMPLE_LINES[1], "controls still work under system reduced motion", s.bubble.text);
  await setReducedMotion(client, false);
  await reload(client);
  await sleep(200);
  await waitFor(client, "document.body && document.body.dataset.ready === '1'", 8000);
  await sleep(200);
  s = await snapshot(client);
  expect(s.data.motion === "on" && s.toggle.motionChecked === false, "clearing the system preference restores motion", s.data);

  // ── network / page health ──────────────────────────────────
  section("network + page health");
  const external = client.events
    .filter((event) => event.method === "Network.requestWillBeSent")
    .map((event) => event.params.request.url)
    .filter((url) => /^https?:/i.test(url));
  const pageErrors = client.events.filter((event) => event.method === "Runtime.exceptionThrown");
  const consoleErrors = client.events.filter(
    (event) => event.method === "Runtime.consoleAPICalled" && event.params.type === "error"
  );
  details.network = { external: external.length, pageErrors: pageErrors.length, consoleErrors: consoleErrors.length };
  expect(external.length === 0, "zero external network requests", external.slice(0, 3));
  expect(pageErrors.length === 0, "zero uncaught page errors", pageErrors.map((e) => e.params.exceptionDetails?.text));
  expect(consoleErrors.length === 0, "zero console errors", consoleErrors.length);

  return { ok: failures.length === 0, checks, failures, details };
}

// ── character sheet view ─────────────────────────────────────
export async function sheetChecks({ client, log = () => {} }) {
  const failures = [];
  let checks = 0;
  const expect = (condition, label, extra) => {
    checks += 1;
    if (!condition) failures.push(label + (extra === undefined ? "" : ` :: ${JSON.stringify(extra)}`));
  };
  await sleep(300);
  const info = await client.evaluate(`(() => {
    const $ = (sel) => document.querySelector(sel);
    const cards = [...document.querySelectorAll('[data-sheet-card]')].map((card) => {
      const svg = card.querySelector('svg');
      const b = card.getBoundingClientRect();
      return { name: card.dataset.sheetCard, svgs: card.querySelectorAll('svg').length, expr: svg?.dataset.expression || null, aria: svg?.getAttribute('aria-label') || null, w: +b.width.toFixed(1), h: +b.height.toFixed(1) };
    });
    return {
      title: document.title,
      pageHidden: $('#page').hidden,
      sheetHidden: $('#sheet').hidden,
      cards,
      motion: document.body.dataset.motion,
      anims: document.getAnimations().length,
      scrollWidth: document.documentElement.scrollWidth,
      vw: innerWidth,
      back: !!$('.sheet-top a'),
    };
  })()`);
  log(`· sheet view · ${info.cards.length} cards`);
  expect(info.pageHidden === true && info.sheetHidden === false, "?view=sheet shows the sheet and hides the page", info);
  expect(info.cards.length === 5, "sheet has five expression cards", info.cards.length);
  expect(new Set(info.cards.map((c) => c.expr)).size === 5, "sheet clones use five distinct expressions", info.cards.map((c) => c.expr));
  expect(info.cards.every((c) => c.svgs === 1), "each sheet card contains exactly one character", info.cards);
  expect(info.cards.every((c) => c.aria && c.aria.startsWith("小析")), "sheet characters have accessible names", info.cards);
  expect(info.cards.every((c) => c.w > 200 && c.h > 200), "sheet cards have real layout", info.cards);
  expect(info.motion === "off" && info.anims === 0, "sheet is a static reference (motion off)", { motion: info.motion, anims: info.anims });
  expect(info.scrollWidth <= info.vw + 0.5, "sheet has no horizontal overflow", info.scrollWidth);
  expect(info.back === true, "sheet has a back link");
  return { ok: failures.length === 0, checks, failures, info };
}

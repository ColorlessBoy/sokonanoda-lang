/* scripts/learn_demo/app.js — one page, no dependencies.
 *
 * Reads the embedded content JSON, draws two SVG companions, computes the
 * ε–N scene from a single formula, and runs five answer-checked game types.
 * The math lives in computeN()/isInside() only; the UI never hardcodes N.
 */
(function () {
  "use strict";

  var DATA = JSON.parse(document.getElementById("demo-data").textContent);
  var TERMS = 20;
  var EPS_SCALE = 1000;
  var EPS_MIN = 0.001;
  var EPS_MAX = 0.25;

  var $ = function (selector, root) { return (root || document).querySelector(selector); };
  var $$ = function (selector, root) { return Array.prototype.slice.call((root || document).querySelectorAll(selector)); };
  var clamp = function (value, lo, hi) { return Math.min(hi, Math.max(lo, value)); };
  var esc = function (value) {
    return String(value).replace(/[&<>"']/g, function (ch) {
      return { "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[ch];
    });
  };

  // ── math: the only source of truth for N and band membership ──
  function computeN(epsilon) {
    if (!(epsilon > 0) || !isFinite(epsilon)) throw new RangeError("epsilon must be positive and finite");
    var milli = Math.round(epsilon * EPS_SCALE);
    if (Math.abs(epsilon * EPS_SCALE - milli) <= 1e-7) {
      return Math.floor(EPS_SCALE / Math.max(1, milli)) + 1;
    }
    var inv = 1 / epsilon;
    var nearest = Math.round(inv);
    if (Math.abs(inv - nearest) <= 1e-9) return nearest + 1;
    return Math.floor(inv) + 1;
  }
  function quantizeEpsilon(value) {
    if (!isFinite(value)) return EPS_MIN;
    return Math.round(clamp(value, EPS_MIN, EPS_MAX) * EPS_SCALE) / EPS_SCALE;
  }
  function isInside(n, epsilon) { return n >= 1 && n <= TERMS && 1 / n < epsilon; }
  function insideCount(N) { return Math.max(0, TERMS - N + 1); }
  function fmtEps(epsilon) { return Number(epsilon).toFixed(3); }
  function fmtTight(value) {
    if (!isFinite(value)) return "∞";
    if (value !== 0 && Math.abs(value) < 0.01) return Number(value).toFixed(6);
    return Number(value).toFixed(4);
  }
  function fmtTerm(n) {
    var value = 1 / n;
    return Number.isInteger(value) ? String(value) : value.toFixed(4);
  }
  function termsFor(epsilon) {
    var N = computeN(epsilon);
    var list = [];
    for (var n = 1; n <= TERMS; n += 1) {
      list.push({ n: n, value: 1 / n, inside: isInside(n, epsilon), inTail: n >= N });
    }
    return list;
  }

  // ── avatars: same 2D family, two distinct silhouettes/roles ──
  var AVATARS = {
    xiaoxi: [
      '<svg class="companion-avatar avatar-xiaoxi" data-avatar="xiaoxi" data-expression="curious" viewBox="0 0 260 310" aria-hidden="true" focusable="false">',
      '<ellipse class="av-ground" cx="130" cy="298" rx="60" ry="9"/>',
      '<g class="av-figure">',
      '<path class="av-torso" d="M70 198 Q70 174 102 170 L158 170 Q190 174 190 198 L190 272 Q190 288 172 288 L88 288 Q70 288 70 272 Z"/>',
      '<path class="av-arm av-arm-l" d="M82 206 Q70 236 88 258"/>',
      '<path class="av-arm av-arm-r" d="M178 206 Q190 236 172 258"/>',
      '<rect class="av-neck" x="116" y="152" width="28" height="26" rx="11"/>',
      '<path class="av-scarf-tail" d="M112 184 Q100 204 106 224 Q111 235 120 229 Q113 209 120 190 Z"/>',
      '<path class="av-scarf" d="M103 174 Q130 189 157 174 L154 192 Q130 205 106 192 Z"/>',
      '<circle class="av-scarf-pin" cx="111" cy="181" r="3.4"/>',
      '<g class="av-pencil" transform="rotate(20 205 70)">',
      '<rect x="201" y="34" width="8" height="52" rx="4" fill="#e9a23b"/>',
      '<path d="M201 34 L205 23 L209 34 Z" fill="#f4e3cb"/>',
      '<path d="M203.4 27 L205 22 L206.6 27 Z" fill="#16202c"/>',
      '<rect x="201" y="82" width="8" height="7" rx="2" fill="#c96f52"/>',
      '<rect x="201" y="78" width="8" height="5" fill="#17324d"/>',
      '</g>',
      '<g class="av-head">',
      '<rect class="av-face" x="64" y="30" width="132" height="128" rx="44"/>',
      '<path class="av-hair" d="M58 112 C58 52 82 24 130 24 C178 24 202 52 202 112 L202 120 C196 104 190 92 180 82 C166 96 146 102 126 101 C104 100 86 93 74 80 C66 90 62 104 64 122 Z"/>',
      '<path class="av-hair-side" d="M60 94 C56 116 59 136 68 152 C66 140 66 126 70 110 Z"/>',
      '<path class="av-hair-side" d="M200 94 C204 116 201 136 192 152 C194 140 194 126 190 110 Z"/>',
      '<path class="av-hair-hi" d="M84 52 C104 36 142 32 166 44"/>',
      '<g class="av-brows"><path class="av-brow av-brow-l" d="M91 111 Q103 106 115 111"/><path class="av-brow av-brow-r" d="M145 111 Q157 106 169 111"/></g>',
      '<g class="av-eyes av-eyes-open"><ellipse class="av-eye" cx="104" cy="125" rx="5.2" ry="7.2"/><ellipse class="av-eye" cx="156" cy="125" rx="5.2" ry="7.2"/><circle class="av-catch" cx="102.2" cy="122.2" r="1.5"/><circle class="av-catch" cx="154.2" cy="122.2" r="1.5"/></g>',
      '<g class="av-eyes av-eyes-closed"><path d="M97 126 Q104 131 111 126"/><path d="M149 126 Q156 131 163 126"/></g>',
      '<g class="av-eyes av-eyes-happy"><path d="M97 128 Q104 120 111 128"/><path d="M149 128 Q156 120 163 128"/></g>',
      '<ellipse class="av-blush" cx="88" cy="140" rx="9" ry="4.2"/><ellipse class="av-blush" cx="172" cy="140" rx="9" ry="4.2"/>',
      '<g class="av-mouths"><path class="av-mouth av-mouth-neutral" d="M121 145 Q130 151 139 145"/><ellipse class="av-mouth av-mouth-curious" cx="130" cy="148" rx="4" ry="5.4"/><path class="av-mouth av-mouth-thinking" d="M122 148 Q130 145 138 148"/><path class="av-mouth av-mouth-happy" d="M117 144 Q130 160 143 144"/><path class="av-mouth av-mouth-oops" d="M122 148 Q130 144 138 149"/></g>',
      '<g class="av-thought"><circle cx="52" cy="82" r="1.6"/><circle cx="44" cy="72" r="2.2"/><circle cx="36" cy="62" r="2.8"/></g>',
      '</g>',
      '<g class="av-notebook" transform="rotate(-4 130 250)"><rect class="av-nb-cover" x="88" y="224" width="84" height="58" rx="7"/><rect class="av-nb-spine" x="88" y="224" width="9" height="58" rx="4"/><path class="av-nb-rule" d="M106 240 H160"/><path class="av-nb-rule" d="M106 249 H154"/><path class="av-nb-rule" d="M106 258 H162"/><path class="av-nb-eps" d="M173 245 C166 243 163 248 168 251 C174 254 174 262 165 260"/></g>',
      '<circle class="av-hand av-hand-l" cx="94" cy="257" r="8.5"/><circle class="av-hand av-hand-r" cx="167" cy="256" r="8.5"/>',
      '</g>',
      '</svg>'
    ].join(""),
    ali: [
      '<svg class="companion-avatar avatar-ali" data-avatar="ali" data-expression="neutral" viewBox="0 0 260 310" aria-hidden="true" focusable="false">',
      '<ellipse class="av-ground" cx="130" cy="298" rx="64" ry="9"/>',
      '<g class="av-figure">',
      '<path class="av-torso" d="M62 198 Q62 170 100 166 L160 166 Q198 170 198 198 L198 276 Q198 290 180 290 L80 290 Q62 290 62 276 Z"/>',
      '<path class="av-shirt" d="M104 168 L130 214 L156 168 Q142 160 130 160 Q118 160 104 168 Z"/>',
      '<path class="av-collar" d="M101 168 L130 216 L116 220 L94 178 Z"/>',
      '<path class="av-collar" d="M159 168 L130 216 L144 220 L166 178 Z"/>',
      '<path class="av-tie" d="M126 184 L134 184 L138 224 L130 236 L122 224 Z"/>',
      '<path class="av-arm av-arm-l" d="M78 204 Q64 238 84 260"/>',
      '<path class="av-arm av-arm-r" d="M182 204 Q200 186 206 166"/>',
      '<rect class="av-neck" x="116" y="150" width="28" height="26" rx="11"/>',
      '<g class="av-pointer" transform="rotate(10 208 148)"><line x1="207" y1="164" x2="226" y2="86"/><circle cx="226" cy="82" r="4.4"/></g>',
      '<g class="av-head">',
      '<rect class="av-face" x="64" y="30" width="132" height="128" rx="44"/>',
      '<path class="av-hair" d="M58 112 C58 50 82 24 130 24 C178 24 202 50 202 112 L202 118 C192 100 182 88 168 80 C148 92 118 96 92 88 C78 92 68 104 64 122 Z"/>',
      '<path class="av-hair-gray" d="M62 92 C58 108 60 122 65 136 L73 128 C68 116 68 104 72 94 Z"/>',
      '<path class="av-hair-gray" d="M198 92 C202 108 200 122 195 136 L187 128 C192 116 192 104 188 94 Z"/>',
      '<path class="av-hair-hi" d="M92 48 C112 36 148 36 170 48"/>',
      '<g class="av-brows"><path class="av-brow av-brow-l" d="M91 108 Q103 104 115 109"/><path class="av-brow av-brow-r" d="M145 109 Q157 104 169 108"/></g>',
      '<g class="av-glasses"><rect x="88" y="114" width="33" height="23" rx="8"/><rect x="139" y="114" width="33" height="23" rx="8"/><path d="M121 124 Q130 120 139 124"/><path d="M88 123 L77 120"/><path d="M172 123 L183 120"/></g>',
      '<g class="av-eyes av-eyes-open"><ellipse class="av-eye" cx="104" cy="126" rx="4.6" ry="6.4"/><ellipse class="av-eye" cx="156" cy="126" rx="4.6" ry="6.4"/><circle class="av-catch" cx="102.4" cy="123.6" r="1.3"/><circle class="av-catch" cx="154.4" cy="123.6" r="1.3"/></g>',
      '<g class="av-eyes av-eyes-closed"><path d="M98 127 Q104 131 110 127"/><path d="M150 127 Q156 131 162 127"/></g>',
      '<g class="av-eyes av-eyes-happy"><path d="M98 128 Q104 121 110 128"/><path d="M150 128 Q156 121 162 128"/></g>',
      '<ellipse class="av-blush" cx="88" cy="142" rx="8" ry="3.8"/><ellipse class="av-blush" cx="172" cy="142" rx="8" ry="3.8"/>',
      '<g class="av-mouths"><path class="av-mouth av-mouth-neutral" d="M121 146 Q130 150 139 146"/><ellipse class="av-mouth av-mouth-curious" cx="130" cy="148" rx="3.6" ry="4.8"/><path class="av-mouth av-mouth-thinking" d="M122 148 Q130 145 138 148"/><path class="av-mouth av-mouth-happy" d="M118 144 Q130 157 142 144"/><path class="av-mouth av-mouth-oops" d="M122 148 Q130 144 138 149"/></g>',
      '<g class="av-thought"><circle cx="52" cy="82" r="1.6"/><circle cx="44" cy="72" r="2.2"/><circle cx="36" cy="62" r="2.8"/></g>',
      '</g>',
      '<circle class="av-hand av-hand-l" cx="85" cy="260" r="8"/><circle class="av-hand av-hand-r" cx="207" cy="162" r="8"/>',
      '<path class="av-pocket" d="M94 236 H124"/><path class="av-pocket" d="M136 236 H166"/>',
      '</g>',
      '</svg>'
    ].join("")
  };

  // ── state ──────────────────────────────────────────────────
  var state = {
    epsilon: DATA.scene.defaultEpsilon,
    activeSpeaker: DATA.characters[0].id,
    focusArea: "intro", // intro | scene | question | complete
    activeQuestion: 0,
    questions: DATA.questions.map(function (question) {
      return {
        status: "pending",
        wrong: 0,
        hints: 0,
        chosen: null,
        order: [],
        slider: typeof question.sliderStart === "number" ? question.sliderStart : null
      };
    }),
    xp: 0,
    streak: 0,
    maxStreak: 0,
    correctCount: 0,
    completed: false,
    playing: false,
    raf: 0,
    playStartedAt: 0,
    reducedMotion: window.matchMedia("(prefers-reduced-motion: reduce)").matches,
    lastAnnouncedN: null,
    lastAnnounceAt: 0
  };

  // ── elements ───────────────────────────────────────────────
  var dom = {
    xp: $("#xp-value"),
    streak: $("#streak-value"),
    progressLabel: $("#progress-label"),
    progressTrack: $("#progress-track"),
    progressFill: $("#progress-fill"),
    heroDek: $("#hero-dek"),
    heroChips: $("#hero-chips"),
    castList: $("#cast-list"),
    bubbleSpeaker: $("#bubble-speaker"),
    bubbleText: $("#bubble-text"),
    hintBtn: $("#hint-btn"),
    hintNote: $("#hint-note"),
    sceneSvg: $("#scene-svg"),
    sceneTitle: $("#scene-svg-title"),
    sceneLayer: $("#scene-layer"),
    sceneCaption: $("#scene-caption"),
    formulaLine: $("#formula-line"),
    boundaryLine: $("#boundary-line"),
    epsRange: $("#eps-range"),
    epsOut: $("#eps-out"),
    nOut: $("#n-out"),
    insideOut: $("#inside-out"),
    invOut: $("#inv-out"),
    strictOut: $("#strict-out"),
    playBtn: $("#play-btn"),
    stepBtn: $("#step-btn"),
    resetBtn: $("#reset-btn"),
    questionRoot: $("#question-root"),
    qCounter: $("#q-counter"),
    qDots: $("#q-dots"),
    feedback: $("#feedback"),
    completion: $("#completion"),
    gameTitle: $("#game-title"),
    announcer: $("#announcer"),
    formalCard: $("#formal-card"),
    formalDef: $("#formal-def"),
    formalNote: $("#formal-note"),
    formalPlain: $("#formal-plain"),
    formalWhy: $("#formal-why"),
    formalProof: $("#formal-proof"),
    bridgeTable: $("#bridge-table"),
    formalCaveat: $("#formal-caveat"),
    footNote: $("#foot-note")
  };
  dom.feedback.tabIndex = -1;

  function currentQuestion() { return DATA.questions[state.activeQuestion]; }
  function currentQS() { return state.questions[state.activeQuestion]; }
  function currentCharacter() {
    return DATA.characters.filter(function (c) { return c.id === state.activeSpeaker; })[0] || DATA.characters[0];
  }
  function characterById(id) {
    return DATA.characters.filter(function (c) { return c.id === id; })[0] || DATA.characters[0];
  }
  function currentContext() {
    if (state.completed || state.focusArea === "complete") return "complete";
    if (state.focusArea === "scene") return "scene";
    if (state.focusArea === "intro") return "intro";
    return currentQuestion().id;
  }
  function currentLine() {
    var character = currentCharacter();
    var context = currentContext();
    var qs = currentQS();
    if (context === "question") context = currentQuestion().id;
    if (state.focusArea === "question" && qs && qs.status === "correct") {
      return currentQuestion().successLines[character.id] || character.lines[currentQuestion().id];
    }
    if (state.focusArea === "question" && character.id === "ali" && qs && qs.hints > 0) {
      return currentQuestion().hints[qs.hints - 1];
    }
    return character.lines[context] || character.lines.intro;
  }

  // ── static content ─────────────────────────────────────────
  function applyContent() {
    document.title = DATA.title + " · 虚拟同伴演示";
    dom.heroDek.textContent = DATA.hero.dek;
    dom.heroChips.innerHTML = DATA.hero.chips.map(function (chip) {
      return '<li class="chip">' + esc(chip) + "</li>";
    }).join("");
    dom.formalDef.textContent = DATA.formal.definition;
    dom.formalNote.textContent = DATA.formal.definitionNote;
    dom.formalPlain.textContent = DATA.formal.plain;
    dom.formalWhy.textContent = DATA.formal.whyN;
    dom.formalProof.innerHTML = DATA.formal.proof.map(function (step) {
      return "<li>" + esc(step) + "</li>";
    }).join("");
    dom.formalCaveat.textContent = DATA.formal.note;
    dom.bridgeTable.innerHTML =
      '<div class="bridge-row bridge-head" role="row">' +
      '<span role="columnheader">直觉</span><span role="columnheader">形式</span><span role="columnheader">桥</span></div>' +
      DATA.formal.rows.map(function (row) {
        return '<div class="bridge-row" role="row">' +
          '<span role="cell">' + esc(row.concept) + "</span>" +
          '<code role="cell">' + esc(row.formal) + "</code>" +
          '<span role="cell">' + esc(row.bridge) + "</span></div>";
      }).join("");
    dom.footNote.textContent = DATA.footer;
    dom.epsRange.min = String(DATA.scene.epsilonMin);
    dom.epsRange.max = String(DATA.scene.epsilonMax);
    dom.epsRange.step = String(DATA.scene.epsilonMin);
  }

  // ── cast ───────────────────────────────────────────────────
  var castButtons = [];
  function buildCast() {
    dom.castList.innerHTML = "";
    castButtons = DATA.characters.map(function (character) {
      var button = document.createElement("button");
      button.type = "button";
      button.className = "cast-card";
      button.dataset.character = character.id;
      button.innerHTML =
        '<span class="cast-stage">' + (AVATARS[character.avatar] || AVATARS.xiaoxi) + "</span>" +
        '<span class="cast-name">' + esc(character.name) + "</span>" +
        '<span class="cast-role">' + esc(character.role) + "</span>" +
        '<span class="cast-tagline">' + esc(character.tagline) + "</span>";
      button.addEventListener("click", function () {
        state.activeSpeaker = character.id;
        if (state.focusArea === "intro") state.focusArea = "scene";
        syncCast();
        renderBubble();
        announce(character.name + " 发言：" + currentLine());
      });
      dom.castList.appendChild(button);
      return button;
    });
  }

  function expressionFor(character) {
    if (state.completed) return character.id === state.activeSpeaker ? "happy" : "neutral";
    var qs = currentQS();
    if (state.focusArea === "question" && qs && qs.status === "correct") {
      return character.id === state.activeSpeaker ? "happy" : "neutral";
    }
    if (state.focusArea === "question" && character.id === "ali" && qs && qs.hints > 0) return "thinking";
    if (state.focusArea === "scene" && state.playing) return character.id === "ali" ? "thinking" : "curious";
    if (state.focusArea === "question") return character.id === "xiaoxi" ? "curious" : "neutral";
    return character.id === "xiaoxi" ? "curious" : "neutral";
  }

  function syncCast() {
    castButtons.forEach(function (button) {
      var character = characterById(button.dataset.character);
      var active = character.id === state.activeSpeaker;
      button.setAttribute("aria-pressed", String(active));
      button.classList.toggle("is-active", active);
      button.setAttribute(
        "aria-label",
        character.name + "，" + character.role + (active ? "，正在发言" : "，点此发言")
      );
      var svg = $(".companion-avatar", button);
      if (svg) svg.setAttribute("data-expression", expressionFor(character));
    });
  }

  function renderBubble() {
    var character = currentCharacter();
    var text = currentLine();
    dom.bubbleSpeaker.textContent = character.name + " · " + character.role;
    if (dom.bubbleText.textContent !== text) dom.bubbleText.textContent = text;
    var qs = currentQS();
    var disabled = state.completed || !qs || qs.status === "correct" || qs.hints >= 3;
    dom.hintBtn.disabled = disabled;
    dom.hintBtn.textContent = state.completed
      ? "提示已收起"
      : qs && qs.status === "correct"
        ? "本题已答对"
        : qs && qs.hints >= 3
          ? "提示已用完（3/3）"
          : "让阿理给提示（" + (qs ? qs.hints + 1 : 1) + "/3）";
    dom.hintNote.textContent = state.completed
      ? "5 题都答对了；提示按钮已收起。"
      : qs && qs.status === "correct"
        ? "本题已答对；提示不再扣分。"
        : qs && qs.hints >= 3
          ? "三层提示都用完了；答对本题仍会计分（最低 5 XP）。"
          : "提示会减 XP，但不会替你作答。";
  }

  function announce(message) {
    if (!dom.announcer) return;
    dom.announcer.textContent = message;
  }

  function setFocusArea(area) {
    state.focusArea = area;
    syncCast();
    renderBubble();
  }

  function useHint() {
    var qs = currentQS();
    if (state.completed || !qs || qs.status === "correct" || qs.hints >= 3) return;
    qs.hints += 1;
    state.activeSpeaker = "ali";
    state.focusArea = "question";
    syncCast();
    renderBubble();
    announce("阿理，提示 " + qs.hints + " / 3：" + currentQuestion().hints[qs.hints - 1]);
  }

  // ── ε–N scene ──────────────────────────────────────────────
  var SCENE = { x0: 64, x1: 936, width: 872, axisY: 236, bandTop: 54, bandBottom: 220, indexY: 316, indexH: 30 };
  var sceneRefs = null;

  function buildScene() {
    var ticks = [0, 0.25, 0.5, 0.75, 1];
    var tickMarkup = ticks.map(function (tick) {
      var x = SCENE.x0 + tick * SCENE.width;
      var anchor = tick === 0 ? "start" : tick === 1 ? "end" : "middle";
      return '<line class="sc-tick" x1="' + x + '" x2="' + x + '" y1="' + SCENE.bandBottom + '" y2="252"/>' +
        '<text class="sc-tick-label" x="' + x + '" y="274" text-anchor="' + anchor + '">' + tick + "</text>";
    }).join("");
    var dots = "";
    for (var n = 1; n <= TERMS; n += 1) {
      var x = SCENE.x0 + (1 / n) * SCENE.width;
      dots += '<circle class="sc-dot" data-term="' + n + '" cx="' + x.toFixed(2) + '" cy="' + SCENE.axisY + '" r="6"/>';
    }
    var cells = "";
    var cellW = SCENE.width / TERMS;
    for (var i = 0; i < TERMS; i += 1) {
      var term = i + 1;
      var cellX = SCENE.x0 + i * cellW + 1.5;
      cells += '<g class="sc-cell" data-cell="' + term + '">' +
        '<rect x="' + cellX.toFixed(2) + '" y="' + SCENE.indexY + '" width="' + (cellW - 3).toFixed(2) + '" height="' + SCENE.indexH + '" rx="4"/>' +
        '<text x="' + (cellX + (cellW - 3) / 2).toFixed(2) + '" y="' + (SCENE.indexY + 20) + '" text-anchor="middle">' + term + "</text></g>";
    }
    dom.sceneLayer.innerHTML =
      '<rect class="sc-band" id="sc-band" x="' + SCENE.x0 + '" y="' + SCENE.bandTop + '" width="0" height="' + (SCENE.bandBottom - SCENE.bandTop) + '" rx="4"/>' +
      '<text class="sc-band-label" id="sc-band-label" x="' + (SCENE.x0 + 10) + '" y="' + (SCENE.bandTop - 10) + '">误差带 0 ≤ aₙ &lt; ε</text>' +
      '<line class="sc-axis" id="sc-axis" x1="' + SCENE.x0 + '" x2="' + SCENE.x1 + '" y1="' + SCENE.axisY + '" y2="' + SCENE.axisY + '"/>' +
      tickMarkup +
      '<line class="sc-eps-line" id="sc-eps-line" x1="0" x2="0" y1="42" y2="256"/>' +
      '<text class="sc-eps-label" id="sc-eps-label" x="0" y="34" text-anchor="start">ε</text>' +
      '<g id="sc-n-pill"><rect id="sc-n-rect" x="0" y="286" width="60" height="24" rx="12"/><text id="sc-n-text" x="0" y="302" text-anchor="middle">N</text></g>' +
      '<line class="sc-n-line" id="sc-n-line" x1="0" x2="0" y1="42" y2="256"/>' +
      '<circle class="sc-n-halo" id="sc-n-halo" cx="0" cy="' + SCENE.axisY + '" r="12" opacity="0"/>' +
      '<g id="sc-dots">' + dots + '</g>' +
      '<text class="sc-axis-name" x="' + SCENE.x0 + '" y="30">值轴 0–1 · aₙ = 1/n</text>' +
      '<text class="sc-axis-name" x="' + SCENE.x0 + '" y="298">序号轴 n = 1…20</text>' +
      '<g id="sc-cells">' + cells + '</g>' +
      '<line class="sc-index-line" id="sc-index-line" x1="0" x2="0" y1="310" y2="352" opacity="0"/>' +
      '<text class="sc-index-note" id="sc-index-note" x="' + SCENE.x0 + '" y="372" text-anchor="start"></text>';
    sceneRefs = {
      band: $("#sc-band"),
      bandLabel: $("#sc-band-label"),
      epsLine: $("#sc-eps-line"),
      epsLabel: $("#sc-eps-label"),
      nLine: $("#sc-n-line"),
      nRect: $("#sc-n-rect"),
      nText: $("#sc-n-text"),
      nHalo: $("#sc-n-halo"),
      dots: $$("#sc-dots .sc-dot"),
      cells: $$("#sc-cells .sc-cell"),
      indexLine: $("#sc-index-line"),
      indexNote: $("#sc-index-note")
    };
  }

  function renderScene() {
    var eps = state.epsilon;
    var N = computeN(eps);
    var inside = insideCount(N);
    var bandWidth = Math.max(1.5, eps * SCENE.width);
    var epsX = SCENE.x0 + eps * SCENE.width;
    var nX = SCENE.x0 + Math.min(1 / N, 1) * SCENE.width;
    var cellW = SCENE.width / TERMS;

    sceneRefs.band.setAttribute("width", bandWidth.toFixed(2));
    sceneRefs.epsLine.setAttribute("x1", epsX.toFixed(2));
    sceneRefs.epsLine.setAttribute("x2", epsX.toFixed(2));
    var epsAnchor = epsX > SCENE.x1 - 110 ? "end" : "start";
    sceneRefs.epsLabel.setAttribute("x", (epsX + (epsAnchor === "end" ? -6 : 6)).toFixed(2));
    sceneRefs.epsLabel.setAttribute("text-anchor", epsAnchor);
    sceneRefs.epsLabel.textContent = "ε = " + fmtEps(eps);

    sceneRefs.nLine.setAttribute("x1", nX.toFixed(2));
    sceneRefs.nLine.setAttribute("x2", nX.toFixed(2));
    var pillText = N <= TERMS ? "N = " + N : "N = " + N + " →";
    var pillWidth = Math.max(58, pillText.length * 8.2 + 20);
    var pillX = clamp(nX - pillWidth / 2, SCENE.x0, SCENE.x1 - pillWidth);
    sceneRefs.nRect.setAttribute("x", pillX.toFixed(2));
    sceneRefs.nRect.setAttribute("width", pillWidth.toFixed(2));
    sceneRefs.nText.setAttribute("x", (pillX + pillWidth / 2).toFixed(2));
    sceneRefs.nText.textContent = pillText;
    sceneRefs.nHalo.setAttribute("cx", nX.toFixed(2));
    sceneRefs.nHalo.setAttribute("opacity", N <= TERMS ? "1" : "0");

    var compactScene = window.innerWidth <= 600;
    sceneRefs.dots.forEach(function (dot) {
      var n = Number(dot.dataset.term);
      var inTail = n >= N;
      dot.setAttribute("class", "sc-dot " + (inTail ? "is-in" : "is-out"));
      var base = compactScene ? 9 : 6;
      dot.setAttribute("r", n === N && N <= TERMS ? String(base + 2) : inTail ? String(base + 0.5) : String(base));
    });

    sceneRefs.cells.forEach(function (cell) {
      var n = Number(cell.dataset.cell);
      var inTail = n >= N;
      cell.setAttribute("class", "sc-cell " + (inTail ? "is-in" : "is-out"));
    });

    if (N <= TERMS) {
      var boundary = SCENE.x0 + (N - 1) * cellW;
      sceneRefs.indexLine.setAttribute("x1", boundary.toFixed(2));
      sceneRefs.indexLine.setAttribute("x2", boundary.toFixed(2));
      sceneRefs.indexLine.setAttribute("opacity", "1");
      sceneRefs.indexNote.setAttribute("x", clamp(boundary + 4, SCENE.x0, SCENE.x1 - 160).toFixed(2));
      sceneRefs.indexNote.setAttribute("text-anchor", "start");
      sceneRefs.indexNote.textContent = "从 n = " + N + " 起：带内 " + inside + " 项";
    } else {
      sceneRefs.indexLine.setAttribute("opacity", "0");
      sceneRefs.indexNote.setAttribute("x", String(SCENE.x0));
      sceneRefs.indexNote.setAttribute("text-anchor", "start");
      sceneRefs.indexNote.textContent = "N = " + N + " → 画面外（前 20 项都在带外）";
    }

    var caption;
    if (N <= TERMS) {
      caption = "ε = " + fmtEps(eps) + " → N = " + N + "：第 " + N + " 项起，画面里的 " + inside +
        " 项都在带内；前 " + (N - 1) + " 项在带外。";
    } else {
      caption = "ε = " + fmtEps(eps) + " → N = " + N + "：N 已超出画面 20 项；定义说的是第 N 项之后，画面外还有无穷多项。";
    }
    var inv = 1 / eps;
    var exactReciprocal = Math.abs(inv - Math.round(inv)) <= 1e-9;
    if (exactReciprocal) {
      caption += " 1/ε = " + Math.round(inv) + " 是整数：第 " + Math.round(inv) +
        " 项正好等于 ε，不算“严格小于”，所以 N = " + N + "。";
    }
    dom.sceneCaption.textContent = caption;

    var invText = exactReciprocal ? String(Math.round(inv)) : (1 / eps).toFixed(3) + "…";
    dom.formulaLine.textContent = "N = ⌊1/" + fmtEps(eps) + "⌋ + 1 = ⌊" + invText + "⌋ + 1 = " + N;
    dom.boundaryLine.textContent = "a" + subscript(N) + " = " + fmtTight(1 / N) + " < ε = " + fmtTight(eps) + " ✓";
    dom.nOut.textContent = String(N);
    dom.insideOut.textContent = String(inside);
    dom.invOut.textContent = fmtTight(1 / N);
    var strict = "1/" + N + " = " + fmtTight(1 / N) + " < " + fmtTight(eps) + " ✓";
    if (N > 1) strict += " · 1/" + (N - 1) + " = " + fmtTight(1 / (N - 1)) + " ≥ " + fmtTight(eps) + " ✓";
    if (N > TERMS) strict += " · N 在画面外";
    dom.strictOut.textContent = strict;

    dom.epsRange.value = String(eps);
    dom.epsOut.textContent = fmtEps(eps);
    dom.epsRange.setAttribute(
      "aria-valuetext",
      "ε = " + fmtEps(eps) + "，起点 N = " + N + "，画面内带内 " + inside + " 项"
    );
    $$(".preset").forEach(function (button) {
      var value = Number(button.dataset.preset);
      button.classList.toggle("is-active", Math.abs(value - eps) <= 1e-9);
      button.setAttribute("aria-pressed", String(Math.abs(value - eps) <= 1e-9));
    });

    var playLabel = state.playing ? "❚❚ 暂停" : state.epsilon <= DATA.scene.playTo + 1e-9 ? "▶ 再放一次" : "▶ 播放 ε 收紧";
    dom.playBtn.textContent = playLabel;
    dom.playBtn.setAttribute("aria-pressed", String(state.playing));
    dom.playBtn.classList.toggle("is-playing", state.playing);
    dom.stepBtn.disabled = eps <= DATA.scene.epsilonMin + 1e-9;

    dom.sceneTitle.textContent = "ε = " + fmtEps(eps) + " 的数轴：N = " + N + "，从第 " + N +
      " 项起带内；前 20 项里有 " + inside + " 项在带内。";

    var now = Date.now();
    var key = N + ":" + fmtEps(eps);
    if (key !== state.lastAnnouncedN && (!state.playing || now - state.lastAnnounceAt > 900)) {
      state.lastAnnouncedN = key;
      state.lastAnnounceAt = now;
      announce(caption);
    }
  }

  function subscript(n) {
    var map = { "0": "₀", "1": "₁", "2": "₂", "3": "₃", "4": "₄", "5": "₅", "6": "₆", "7": "₇", "8": "₈", "9": "₉" };
    return String(n).split("").map(function (ch) { return map[ch] || ch; }).join("");
  }

  function applyEpsilon(epsilon, options) {
    var opts = options || {};
    state.epsilon = quantizeEpsilon(epsilon);
    renderScene();
    if (!opts.silent) setFocusArea("scene");
    syncCast();
  }

  function pausePlayback() {
    if (state.raf) cancelAnimationFrame(state.raf);
    state.raf = 0;
    state.playing = false;
    renderScene();
    syncCast();
  }

  function play() {
    if (state.playing) { pausePlayback(); return; }
    if (state.reducedMotion) {
      state.epsilon = quantizeEpsilon(DATA.scene.playTo);
      state.playing = false;
      renderScene();
      setFocusArea("scene");
      announce("已按“减少动态”偏好直接显示最终状态：ε = " + fmtEps(state.epsilon) + "。");
      return;
    }
    state.playing = true;
    state.epsilon = quantizeEpsilon(DATA.scene.playFrom);
    state.playStartedAt = performance.now();
    setFocusArea("scene");
    renderScene();
    syncCast();
    announce("开始播放：ε 从 " + fmtEps(DATA.scene.playFrom) + " 收紧，观察 N 向右移动。");
    state.raf = requestAnimationFrame(tickPlayback);
  }

  function tickPlayback(now) {
    if (!state.playing) return;
    var duration = Math.max(500, DATA.scene.playDurationMs);
    var t = clamp((now - state.playStartedAt) / duration, 0, 1);
    var eased = t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2;
    state.epsilon = quantizeEpsilon(DATA.scene.playFrom + (DATA.scene.playTo - DATA.scene.playFrom) * eased);
    renderScene();
    if (t < 1) {
      state.raf = requestAnimationFrame(tickPlayback);
    } else {
      state.epsilon = quantizeEpsilon(DATA.scene.playTo);
      state.playing = false;
      state.raf = 0;
      renderScene();
      syncCast();
      announce("播放结束：ε = " + fmtEps(state.epsilon) + "，N = " + computeN(state.epsilon) + "；带内项全部在 N 之后。");
    }
  }

  function stepEpsilon() {
    if (state.playing) pausePlayback();
    var stops = DATA.scene.steps.slice().sort(function (a, b) { return b - a; });
    var next = stops.filter(function (value) { return value < state.epsilon - 1e-9; })[0];
    if (typeof next !== "number") next = DATA.scene.epsilonMin;
    applyEpsilon(next);
    announce("ε 收紧到 " + fmtEps(state.epsilon) + "，N = " + computeN(state.epsilon) + "。");
  }

  function resetScene() {
    if (state.playing) pausePlayback();
    state.epsilon = quantizeEpsilon(DATA.scene.defaultEpsilon);
    renderScene();
    setFocusArea("scene");
    announce("已重置到 ε = " + fmtEps(state.epsilon) + "，N = " + computeN(state.epsilon) + "。");
  }

  // ── game ───────────────────────────────────────────────────
  function awardFor(qs) {
    var scoring = DATA.scoring;
    var award = scoring.perQuestion - qs.hints * scoring.hintPenalty - (qs.wrong > 0 ? scoring.retryPenalty : 0);
    return Math.max(scoring.minAward, award);
  }

  function updateStats() {
    dom.xp.textContent = String(state.xp);
    dom.streak.textContent = String(state.streak);
    var progress = Math.round((state.correctCount / DATA.questions.length) * 100);
    dom.progressLabel.textContent = "已完成 " + state.correctCount + " / " + DATA.questions.length;
    dom.progressTrack.setAttribute("aria-valuenow", String(progress));
    dom.progressFill.style.width = progress + "%";
    dom.qCounter.textContent = "第 " + (state.activeQuestion + 1) + " / " + DATA.questions.length + " 题";
    dom.qDots.innerHTML = DATA.questions.map(function (question, index) {
      var qs = state.questions[index];
      var cls = qs.status === "correct" ? "is-done" : index === state.activeQuestion ? "is-current" : "";
      return '<span class="q-dot ' + cls + '"></span>';
    }).join("");
  }

  function showFeedback(kind, title, body, extra) {
    dom.feedback.hidden = false;
    dom.feedback.className = "feedback is-" + kind;
    dom.feedback.innerHTML =
      '<p class="feedback-title">' + esc(title) + "</p>" +
      '<p class="feedback-body">' + esc(body) + "</p>" +
      (extra ? '<p class="feedback-extra">' + esc(extra) + "</p>" : "");
  }

  function clearFeedback() {
    dom.feedback.hidden = true;
    dom.feedback.className = "feedback";
    dom.feedback.innerHTML = "";
  }

  function markWrong(message) {
    var qs = currentQS();
    qs.wrong += 1;
    state.streak = 0;
    updateStats();
    showFeedback("wrong", "再试一次", message, "这次不计分；重试答对仍会得 " + DATA.scoring.minAward + "+ XP。");
    announce("还不对：" + message);
    renderQuestion({ focus: "feedback" });
  }

  function markCorrect() {
    var question = currentQuestion();
    var qs = currentQS();
    if (qs.status === "correct") return;
    qs.status = "correct";
    state.correctCount += 1;
    state.streak += 1;
    state.maxStreak = Math.max(state.maxStreak, state.streak);
    var award = awardFor(qs);
    state.xp += award;
    setFocusArea("question");
    state.activeSpeaker = state.activeSpeaker || DATA.characters[0].id;
    updateStats();
    showFeedback("correct", "答对了 · +" + award + " XP", question.explanation, "连对 " + state.streak + " 题。");
    syncCast();
    renderBubble();
    announce("答对了，加 " + award + " XP。" + question.explanation);
    renderQuestion({ focus: "next" });
  }

  function renderQuestion(options) {
    var opts = options || {};
    if (state.completed) {
      dom.questionRoot.hidden = true;
      return;
    }
    dom.questionRoot.hidden = false;
    var question = currentQuestion();
    var qs = currentQS();
    var body = "";
    var type = question.type;

    if (type === "multiple-choice" || type === "true-false") {
      if (type === "true-false") {
        body += '<div class="statement-box"><span class="statement-tag">判断</span><p>' + esc(question.statement) + "</p></div>";
      }
      body += '<fieldset class="choice-fieldset"><legend class="sr-only">' + esc(question.prompt) + "</legend>";
      body += '<div class="choice-list">';
      question.choices.forEach(function (choice) {
        var chosen = qs.chosen === choice.id;
        var wrong = chosen && qs.status !== "correct" && choice.id !== question.answer;
        var correct = qs.status === "correct" && choice.id === question.answer;
        var stateAttr = correct ? "correct" : wrong ? "wrong" : "";
        body += '<button type="button" class="choice" data-choice="' + esc(choice.id) + '"' +
          (stateAttr ? ' data-state="' + stateAttr + '"' : "") +
          (qs.status === "correct" ? " disabled" : "") +
          '><span class="choice-label">' + esc(choice.label) + '</span><span class="choice-mark" aria-hidden="true">' +
          (correct ? "✓" : wrong ? "×" : "") + "</span></button>";
      });
      body += "</div></fieldset>";
    } else if (type === "click-dot") {
      body += renderDotQuestion(question, qs);
    } else if (type === "order") {
      body += renderOrderQuestion(question, qs);
    } else if (type === "slider-choice") {
      body += renderSliderQuestion(question, qs);
    }

    var canNext = qs.status === "correct";
    var isLast = state.activeQuestion === DATA.questions.length - 1;
    body += '<div class="q-actions">';
    if (type === "order" && qs.status !== "correct") {
      body += '<button type="button" class="btn btn-soft" id="clear-order"' + (qs.order.length ? "" : " disabled") + ">清空顺序</button>";
      body += '<button type="button" class="btn btn-check" id="check-order"' + (qs.order.length === 4 ? "" : " disabled") + ">检查顺序</button>";
    }
    if (canNext) {
      body += '<button type="button" class="btn btn-next" id="' + (isLast ? "finish-demo" : "next-question") + '">' +
        (isLast ? "查看完成徽章" : "下一题") + "</button>";
    }
    body += "</div>";

    dom.questionRoot.innerHTML =
      '<div class="q-head"><span class="q-index">第 ' + (state.activeQuestion + 1) + " / " + DATA.questions.length + " 题</span>" +
      '<span class="q-type">' + esc(question.typeLabel) + "</span>" +
      (qs.status === "correct" ? '<span class="q-done">✓ 已答对</span>' : "") + "</div>" +
      '<h3 class="q-title" id="question-title" tabindex="-1">' + esc(question.title) + "</h3>" +
      '<p class="q-prompt" id="question-prompt">' + esc(question.prompt) + "</p>" +
      body;

    updateStats();
    renderBubble();
    if (opts.focus === "next") {
      var next = $("#next-question") || $("#finish-demo");
      if (next) next.focus();
    } else if (opts.focus === "feedback") {
      dom.feedback.focus();
    } else if (opts.focus === "title") {
      var title = $("#question-title");
      if (title) title.focus();
    }
  }

  function renderDotQuestion(question, qs) {
    var eps = question.epsilon;
    var N = computeN(eps);
    var bandY = 8 + (1 - eps) * 82;
    var dots = "";
    for (var n = 1; n <= question.dotCount; n += 1) {
      var value = 1 / n;
      var left = 5 + ((n - 1) / (question.dotCount - 1)) * 90;
      var top = 8 + (1 - value) * 82;
      var wrong = qs.chosen === n && qs.status !== "correct";
      var correct = qs.status === "correct" && n === N;
      dots += '<button type="button" class="seq-dot" data-dot="' + n + '"' +
        ' style="left:' + left.toFixed(2) + "%;top:" + top.toFixed(2) + '%"' +
        ' aria-label="第 ' + n + " 项，a" + subscript(n) + " ≈ " + value.toFixed(3) + '"' +
        (correct ? ' data-state="correct"' : wrong ? ' data-state="wrong"' : "") +
        (qs.status === "correct" ? " disabled" : "") + '><span class="seq-dot-core" aria-hidden="true"></span></button>';
    }
    return '<div class="dot-plot" id="dot-plot" aria-describedby="q3-scale">' +
      '<div class="dot-band" style="--eps-y:' + bandY.toFixed(2) + '%"><span class="dot-band-label">aₙ &lt; ε = ' + fmtEps(eps) + "</span></div>" +
      '<span class="dot-axis-label dot-axis-top" aria-hidden="true">aₙ</span>' +
      '<span class="dot-axis-label dot-axis-bottom" aria-hidden="true">0</span>' +
      dots +
      '<div class="dot-scale" id="q3-scale"><span>n = 1</span><span>n = ' + Math.round(question.dotCount / 2) + "</span><span>n = " + question.dotCount + "</span></div></div>" +
      '<p class="q-micro">带内 = 点的值严格小于 ε；纵轴是 aₙ，横轴是项号 n。</p>';
  }

  function renderOrderQuestion(question, qs) {
    var blocks = {};
    question.blocks.forEach(function (block) { blocks[block.id] = block; });
    var slots = "";
    for (var index = 0; index < 4; index += 1) {
      var id = qs.order[index];
      slots += '<li class="order-slot" data-slot="' + index + '">';
      if (id) {
        slots += '<button type="button" class="order-block is-placed" data-placed="' + esc(id) + '"' +
          (qs.status === "correct" ? " disabled" : "") + '>' +
          '<span class="order-num" aria-hidden="true">' + (index + 1) + "</span>" +
          '<span class="order-text">' + esc(blocks[id].text) + "</span>" +
          '<span class="order-remove" aria-hidden="true">取回</span></button>';
      } else {
        slots += '<span class="order-empty">第 ' + (index + 1) + " 格</span>";
      }
      slots += "</li>";
    }
    var pool = question.pool.map(function (id) {
      var placed = qs.order.indexOf(id) !== -1;
      return '<button type="button" class="order-block" data-block="' + esc(id) + '"' +
        (placed ? " disabled" : "") + ">" + esc(blocks[id].text) + "</button>";
    }).join("");
    return '<ol class="order-slots" aria-label="已放置的量词顺序">' + slots + "</ol>" +
      '<div class="block-pool" role="group" aria-label="可放置的量词块">' + pool + "</div>" +
      '<p class="q-micro">先点下方块放入下一格；点已放置的块可以取回再调整。</p>';
  }

  function renderSliderQuestion(question, qs) {
    var slider = typeof qs.slider === "number" ? qs.slider : question.sliderStart;
    var choices = question.choices.map(function (choice) {
      var chosen = qs.chosen === choice.id;
      var wrong = chosen && qs.status !== "correct" && choice.id !== question.answer;
      var correct = qs.status === "correct" && choice.id === question.answer;
      var stateAttr = correct ? "correct" : wrong ? "wrong" : "";
      return '<button type="button" class="choice" data-choice="' + esc(choice.id) + '"' +
        (stateAttr ? ' data-state="' + stateAttr + '"' : "") +
        (qs.status === "correct" ? " disabled" : "") +
        '><span class="choice-label">' + esc(choice.label) + '</span><span class="choice-mark" aria-hidden="true">' +
        (correct ? "✓" : wrong ? "×" : "") + "</span></button>";
    }).join("");
    return '<div class="q5-slider">' +
      '<label for="q5-range">题内 ε</label>' +
      '<input id="q5-range" type="range" min="' + question.sliderMin + '" max="' + question.sliderMax +
      '" step="' + question.sliderStep + '" value="' + slider + '"' +
      ' aria-describedby="q5-out" aria-valuetext="ε = ' + Number(slider).toFixed(2) + '"' +
      (qs.status === "correct" ? " disabled" : "") + ">" +
      '<output id="q5-out" for="q5-range">ε = ' + Number(slider).toFixed(2) + "</output></div>" +
      '<div class="choice-list q5-choices">' + choices + "</div>" +
      '<p class="q-micro">先把滑块拖到目标值，再选 N；两件事都对才算通过。</p>';
  }

  function answerChoice(choiceId) {
    var question = currentQuestion();
    var qs = currentQS();
    if (qs.status === "correct") return;
    setFocusArea("question");
    qs.chosen = choiceId;
    var choice = question.choices.filter(function (c) { return c.id === choiceId; })[0];
    if (!choice) return;

    if (question.type === "slider-choice") {
      var slider = typeof qs.slider === "number" ? qs.slider : question.sliderStart;
      if (Math.abs(slider - question.targetEpsilon) > 1e-9) {
        qs.chosen = null;
        setFocusArea("question");
        showFeedback(
          "wrong",
          "先把 ε 调到 " + Number(question.targetEpsilon).toFixed(2),
          "当前题内滑块是 ε = " + Number(slider).toFixed(2) + "。先拖到 " +
            Number(question.targetEpsilon).toFixed(2) + "，再选 N；这一步不计错。",
          "题内滑块精确到 0.01。"
        );
        announce("先把题内 ε 调到 " + Number(question.targetEpsilon).toFixed(2) + "，再选 N。");
        renderQuestion({ focus: "title" });
        return;
      }
      if (choice.value !== computeN(question.targetEpsilon)) {
        markWrong(choice.why || "这个 N 不满足严格小于。");
        return;
      }
      markCorrect();
      return;
    }

    if (choiceId === question.answer) {
      markCorrect();
      return;
    }
    markWrong(choice.why || "这个选项不满足 N = ⌊1/ε⌋ + 1。");
  }

  function answerDot(n) {
    var question = currentQuestion();
    var qs = currentQS();
    if (qs.status === "correct") return;
    setFocusArea("question");
    qs.chosen = n;
    var N = computeN(question.epsilon);
    if (n === N) {
      markCorrect();
      return;
    }
    var value = 1 / n;
    if (value < question.epsilon) {
      markWrong("a" + subscript(n) + " = " + fmtTerm(n) + " 确实在带内，但不是第一个；再往左找第一个满足 aₙ < ε 的 n。");
    } else {
      markWrong("a" + subscript(n) + " = " + fmtTerm(n) + " ≥ ε = " + fmtEps(question.epsilon) + "，这个点还在带外。");
    }
  }

  function placeBlock(id) {
    var qs = currentQS();
    if (qs.status === "correct" || qs.order.length >= 4 || qs.order.indexOf(id) !== -1) return;
    setFocusArea("question");
    qs.order.push(id);
    renderQuestion();
    var nextBlock = $(".block-pool .order-block:not(:disabled)");
    if (nextBlock) nextBlock.focus();
    else {
      var title = $("#question-title");
      if (title) title.focus();
    }
  }

  function removeBlock(id) {
    var qs = currentQS();
    if (qs.status === "correct") return;
    setFocusArea("question");
    qs.order = qs.order.filter(function (value) { return value !== id; });
    renderQuestion();
    var poolButton = $('[data-block="' + id + '"]');
    if (poolButton) poolButton.focus();
  }

  function checkOrder() {
    var question = currentQuestion();
    var qs = currentQS();
    if (qs.order.length !== 4 || qs.status === "correct") return;
    setFocusArea("question");
    var mismatch = -1;
    for (var i = 0; i < 4; i += 1) {
      if (qs.order[i] !== question.answer[i]) { mismatch = i; break; }
    }
    if (mismatch === -1) {
      markCorrect();
      return;
    }
    var expectedId = question.answer[mismatch];
    var block = question.blocks.filter(function (b) { return b.id === expectedId; })[0];
    markWrong("第 " + (mismatch + 1) + " 格应是「" + block.text + "」：" + block.why);
  }

  function onSliderInput(event) {
    var question = currentQuestion();
    var qs = currentQS();
    if (question.type !== "slider-choice") return;
    qs.slider = Number(event.target.value);
    setFocusArea("question");
    var out = $("#q5-out");
    if (out) out.textContent = "ε = " + qs.slider.toFixed(2);
    event.target.setAttribute("aria-valuetext", "ε = " + qs.slider.toFixed(2));
    renderBubble();
  }

  function goNext() {
    if (state.activeQuestion >= DATA.questions.length - 1) {
      completeDemo();
      return;
    }
    state.activeQuestion += 1;
    state.activeSpeaker = DATA.characters[0].id;
    state.focusArea = "question";
    clearFeedback();
    renderQuestion({ focus: "title" });
    syncCast();
    renderBubble();
    var title = $("#question-title");
    if (title) title.focus();
    dom.gameTitle.scrollIntoView({ behavior: state.reducedMotion ? "auto" : "smooth", block: "start" });
  }

  function completeDemo() {
    if (state.correctCount !== DATA.questions.length) return;
    state.completed = true;
    state.focusArea = "complete";
    state.activeSpeaker = DATA.characters[0].id;
    clearFeedback();
    pausePlayback();
    var recap = [
      "ε = 0.22 → N = " + computeN(0.22) + "（a₅ = 0.2 < 0.22）",
      "N = 4 不成立：a₄ = 0.25 ≥ 0.22",
      "第一个带内点：n = " + computeN(0.22),
      "∀ε > 0 → ∃N → ∀n ≥ N → |aₙ − L| < ε",
      "ε = 0.10 → N = " + computeN(0.1) + "（1/ε 是整数，N 要 +1）"
    ];
    dom.completion.hidden = false;
    dom.questionRoot.hidden = true;
    dom.completion.innerHTML =
      '<div class="badge-card">' +
      '<p class="badge-kicker">COMPLETE</p>' +
      '<span class="badge-icon" aria-hidden="true">' + esc(DATA.badge.icon) + "</span>" +
      '<h3 class="badge-title" id="completion-title" tabindex="-1">' + esc(DATA.badge.title) + "</h3>" +
      '<p class="badge-detail">' + esc(DATA.badge.detail) + " · XP " + state.xp + " · 最长连对 " + state.maxStreak + "</p>" +
      '<ul class="recap">' + recap.map(function (line) { return "<li>" + esc(line) + "</li>"; }).join("") + "</ul>" +
      '<button type="button" class="btn btn-next" id="restart-demo">再做一次</button>' +
      "</div>";
    dom.completion.scrollIntoView({ behavior: state.reducedMotion ? "auto" : "smooth", block: "start" });
    syncCast();
    renderBubble();
    updateStats();
    renderScene();
    announce("完成。你理解了 ε–N。5 题全对，XP " + state.xp + "。");
    var title = $("#completion-title");
    if (title) title.focus();
  }

  function restartDemo() {
    state.questions = DATA.questions.map(function (question) {
      return {
        status: "pending",
        wrong: 0,
        hints: 0,
        chosen: null,
        order: [],
        slider: typeof question.sliderStart === "number" ? question.sliderStart : null
      };
    });
    state.activeQuestion = 0;
    state.activeSpeaker = DATA.characters[0].id;
    state.focusArea = "intro";
    state.xp = 0;
    state.streak = 0;
    state.maxStreak = 0;
    state.correctCount = 0;
    state.completed = false;
    state.epsilon = DATA.scene.defaultEpsilon;
    state.playing = false;
    if (state.raf) cancelAnimationFrame(state.raf);
    state.raf = 0;
    dom.completion.hidden = true;
    dom.completion.innerHTML = "";
    dom.questionRoot.hidden = false;
    clearFeedback();
    renderQuestion({ focus: "title" });
    renderScene();
    syncCast();
    renderBubble();
    updateStats();
    window.scrollTo({ top: 0, behavior: state.reducedMotion ? "auto" : "smooth" });
    announce("演示已重置：ε 回到 " + fmtEps(state.epsilon) + "，5 道题重新开始。");
  }

  // ── events ─────────────────────────────────────────────────
  function bindEvents() {
    dom.hintBtn.addEventListener("click", useHint);

    dom.epsRange.addEventListener("input", function () {
      if (state.playing) pausePlayback();
      applyEpsilon(Number(dom.epsRange.value));
    });
    $$(".preset").forEach(function (button) {
      button.addEventListener("click", function () {
        if (state.playing) pausePlayback();
        applyEpsilon(Number(button.dataset.preset));
      });
    });
    dom.playBtn.addEventListener("click", play);
    dom.stepBtn.addEventListener("click", stepEpsilon);
    dom.resetBtn.addEventListener("click", resetScene);

    dom.questionRoot.addEventListener("click", function (event) {
      var target = event.target.closest("button");
      if (!target || target.disabled) return;
      if (target.dataset.choice) { answerChoice(target.dataset.choice); return; }
      if (target.dataset.dot) { answerDot(Number(target.dataset.dot)); return; }
      if (target.dataset.block) { placeBlock(target.dataset.block); return; }
      if (target.dataset.placed) { removeBlock(target.dataset.placed); return; }
      if (target.id === "check-order") { checkOrder(); return; }
      if (target.id === "clear-order") {
        var qs = currentQS();
        qs.order = [];
        clearFeedback();
        renderQuestion({ focus: "title" });
        return;
      }
      if (target.id === "next-question") { goNext(); return; }
      if (target.id === "finish-demo") { completeDemo(); return; }
    });

    dom.questionRoot.addEventListener("input", function (event) {
      if (event.target.id === "q5-range") onSliderInput(event);
    });

    dom.completion.addEventListener("click", function (event) {
      var target = event.target.closest("button");
      if (target && target.id === "restart-demo") restartDemo();
    });

    var motion = window.matchMedia("(prefers-reduced-motion: reduce)");
    var onMotion = function (event) {
      state.reducedMotion = event.matches;
      if (event.matches && state.playing) {
        state.playing = false;
        if (state.raf) cancelAnimationFrame(state.raf);
        state.raf = 0;
        state.epsilon = quantizeEpsilon(DATA.scene.playTo);
        renderScene();
        setFocusArea("scene");
      }
    };
    if (motion.addEventListener) motion.addEventListener("change", onMotion);
    else if (motion.addListener) motion.addListener(onMotion);

    var resizeTimer = 0;
    window.addEventListener("resize", function () {
      if (resizeTimer) clearTimeout(resizeTimer);
      resizeTimer = setTimeout(function () {
        resizeTimer = 0;
        renderScene();
      }, 120);
    });

    document.addEventListener("visibilitychange", function () {
      if (document.hidden && state.playing) pausePlayback();
    });
  }

  // ── init ───────────────────────────────────────────────────
  function init() {
    applyContent();
    buildCast();
    buildScene();
    bindEvents();
    renderQuestion();
    renderScene();
    syncCast();
    renderBubble();
    updateStats();
  }

  // ── public surface for checks / embedding (read-only + scene controls) ──
  window.SokoLearnDemo = {
    version: 1,
    data: DATA,
    getState: function () {
      var question = currentQuestion();
      var qs = currentQS();
      var N = computeN(state.epsilon);
      var inside = insideCount(N);
      return {
        version: 1,
        epsilon: state.epsilon,
        N: N,
        inside: inside,
        terms: termsFor(state.epsilon),
        bandPct: state.epsilon,
        nLinePct: Math.min(1, 1 / N),
        formulaHolds: 1 / N < state.epsilon + 1e-12 && (N === 1 || 1 / (N - 1) >= state.epsilon - 1e-12),
        formulaText: dom.formulaLine.textContent,
        boundaryText: dom.boundaryLine.textContent,
        caption: dom.sceneCaption.textContent,
        activeSpeaker: state.activeSpeaker,
        context: currentContext(),
        focusArea: state.focusArea,
        bubble: { speaker: dom.bubbleSpeaker.textContent, text: dom.bubbleText.textContent },
        hintTier: qs ? qs.hints : 0,
        hintButtonDisabled: dom.hintBtn.disabled,
        questionIndex: state.activeQuestion,
        questionId: question ? question.id : null,
        questionType: question ? question.type : null,
        questionStatus: qs ? qs.status : null,
        wrongAttempts: qs ? qs.wrong : 0,
        order: qs ? qs.order.slice() : [],
        slider: qs ? qs.slider : null,
        xp: state.xp,
        streak: state.streak,
        maxStreak: state.maxStreak,
        correctCount: state.correctCount,
        progress: Math.round((state.correctCount / DATA.questions.length) * 100),
        completed: state.completed,
        badgeVisible: !dom.completion.hidden,
        reducedMotion: state.reducedMotion,
        playing: state.playing,
        characters: DATA.characters.map(function (character) {
          return { id: character.id, name: character.name, role: character.role, avatar: character.avatar };
        }),
        questions: state.questions.map(function (item, index) {
          return { id: DATA.questions[index].id, type: DATA.questions[index].type, status: item.status, wrong: item.wrong, hints: item.hints };
        })
      };
    },
    math: {
      computeN: computeN,
      isInside: isInside,
      insideCount: insideCount,
      termsFor: termsFor
    },
    scene: {
      setEpsilon: function (value) { applyEpsilon(Number(value)); },
      play: play,
      pause: pausePlayback,
      step: stepEpsilon,
      reset: resetScene
    }
  };

  if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", init);
  else init();
})();

/* scripts/lesson_complete/app.js — one linear lesson, no dependencies.
 *
 * The page is a seven-screen SPA: cover → sequence → band → Q1 → Q2 → Q3 →
 * summary. Only one screen is exposed at a time; the only forward action is
 * the labelled "下一步" button. Mathematics comes from math.js; this file
 * renders the scene and decides whether an answer is right.
 */
(function () {
  "use strict";

  var M = window.LessonMath;
  var DATA = JSON.parse(document.getElementById("lesson-data").textContent);
  var SCREEN_ORDER = ["cover", "sequence", "band", "q1", "q2", "q3", "summary"];
  var STEP_COUNT = SCREEN_ORDER.length - 1; // cover is not a numbered step
  var $ = function (selector, root) { return (root || document).querySelector(selector); };
  var $$ = function (selector, root) {
    return Array.prototype.slice.call((root || document).querySelectorAll(selector));
  };

  function esc(value) {
    return String(value).replace(/[&<>"']/g, function (ch) {
      return { "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[ch];
    });
  }

  function subscript(n) {
    var map = { "0": "₀", "1": "₁", "2": "₂", "3": "₃", "4": "₄", "5": "₅", "6": "₆", "7": "₇", "8": "₈", "9": "₉" };
    return String(n).split("").map(function (ch) { return map[ch] || ch; }).join("");
  }

  var QUESTIONS = {};
  DATA.questions.forEach(function (question) {
    QUESTIONS[question.id] = question;
  });

  var state = {
    screen: "cover",
    epsilon: DATA.scene.defaultEpsilon,
    questions: {},
  };
  DATA.questions.forEach(function (question) {
    state.questions[question.id] = { solved: false, wrong: false, selected: null, hintOpen: false };
  });

  var dom = {
    app: $("#app"),
    progress: $("#progress"),
    progressLabel: $("#progress-label"),
    progressSegs: $$(".progress-seg"),
    announcer: $("#announcer"),
    epsRange: $("#eps-range"),
    epsValue: $("#eps-value"),
    nFormula: $("#n-formula"),
    checkFirst: $("#check-first"),
    checkPrev: $("#check-prev"),
    bandStage: $("#band-stage"),
    bandCaption: $("#band-caption"),
  };

  var motionQuery = window.matchMedia("(prefers-reduced-motion: reduce)");
  function syncMotionFlag() {
    dom.app.dataset.reducedMotion = motionQuery.matches ? "true" : "false";
  }
  syncMotionFlag();
  if (typeof motionQuery.addEventListener === "function") {
    motionQuery.addEventListener("change", syncMotionFlag);
  }

  var announceTimer = 0;
  function announce(text) {
    window.clearTimeout(announceTimer);
    dom.announcer.textContent = "";
    announceTimer = window.setTimeout(function () {
      dom.announcer.textContent = text;
    }, 40);
  }

  // ── companion ──────────────────────────────────────────────
  function mountCompanions() {
    var template = document.getElementById("companion-template");
    if (!template) return;
    $$("[data-coach]").forEach(function (slot) {
      slot.appendChild(template.content.cloneNode(true));
    });
  }

  function visibleCompanion() {
    return $(".screen:not([hidden]) .companion-avatar");
  }

  function setExpression(expression) {
    var avatar = visibleCompanion();
    if (avatar) avatar.setAttribute("data-expression", expression);
  }

  function questionExpression(id) {
    var qs = state.questions[id];
    if (!qs) return "curious";
    if (qs.solved) return "happy";
    if (qs.wrong) return "oops";
    if (qs.hintOpen) return "thinking";
    return "curious";
  }

  // ── sequence scene ─────────────────────────────────────────
  var SEQ = { x0: 70, x1: 930, axisY: 210 };

  function buildSequence() {
    var width = SEQ.x1 - SEQ.x0;
    var ticks = [0, 0.25, 0.5, 0.75, 1];
    var tickMarkup = ticks.map(function (tick) {
      var x = SEQ.x0 + tick * width;
      var anchor = tick === 0 ? "start" : tick === 1 ? "end" : "middle";
      var label = tick === 0.5 ? "1/2" : String(tick);
      return '<line class="sc-tick" x1="' + x.toFixed(2) + '" x2="' + x.toFixed(2) +
        '" y1="' + (SEQ.axisY + 10) + '" y2="' + (SEQ.axisY + 20) + '"/>' +
        '<text class="sc-tick-label" x="' + x.toFixed(2) + '" y="' + (SEQ.axisY + 46) +
        '" text-anchor="' + anchor + '">' + label + "</text>";
    }).join("");

    var dots = "";
    for (var n = 1; n <= M.TERMS; n += 1) {
      var x = SEQ.x0 + (1 / n) * width;
      dots += '<circle class="seq-dot" data-term="' + n + '" cx="' + x.toFixed(2) +
        '" cy="' + SEQ.axisY + '" r="8.6" style="--i:' + (n - 1) + '">' +
        "<title>a" + n + " = " + esc(M.fmtTerm(n)) + "</title></circle>";
    }

    $("#sequence-layer").innerHTML =
      '<text class="sc-axis-name" x="' + SEQ.x0 + '" y="42">点的位置 = aₙ 的值</text>' +
      '<text class="sc-axis-name" x="' + SEQ.x1 + '" y="42" text-anchor="end">n = 1 … 20</text>' +
      '<line class="sc-axis" x1="' + SEQ.x0 + '" x2="' + SEQ.x1 +
      '" y1="' + SEQ.axisY + '" y2="' + SEQ.axisY + '"/>' +
      '<line class="sc-tick" x1="' + SEQ.x0 + '" x2="' + SEQ.x0 + '" y1="86" y2="236"/>' +
      tickMarkup +
      '<g id="sequence-dots">' + dots + "</g>";
  }

  // ── band + N scene ─────────────────────────────────────────
  var BAND = {
    x0: 70,
    x1: 930,
    valueTop: 66,
    valueBottom: 220,
    axisY: 164,
    indexY: 296,
    valueMax: 0.26,
    indexFirst: 1,
    indexLast: 20,
  };

  function xForValue(value) {
    return BAND.x0 + (value / BAND.valueMax) * (BAND.x1 - BAND.x0);
  }

  function xForIndex(n) {
    var clamped = M.clamp(n, BAND.indexFirst, BAND.indexLast);
    return BAND.x0 + ((clamped - BAND.indexFirst) / (BAND.indexLast - BAND.indexFirst)) * (BAND.x1 - BAND.x0);
  }

  function buildBand() {
    var tickValues = [0, 0.05, 0.1, 0.15, 0.2, 0.25];
    var ticks = tickValues.map(function (value) {
      var x = xForValue(value);
      var anchor = value === 0 ? "start" : value === 0.25 ? "end" : "middle";
      return '<line class="sc-tick" x1="' + x.toFixed(2) + '" x2="' + x.toFixed(2) +
        '" y1="' + (BAND.axisY + 10) + '" y2="' + (BAND.axisY + 20) + '"/>' +
        '<text class="sc-tick-label" x="' + x.toFixed(2) + '" y="' + (BAND.axisY + 45) +
        '" text-anchor="' + anchor + '">' + value.toFixed(2) + "</text>";
    }).join("");

    var dots = "";
    for (var n = M.VISIBLE_FIRST; n <= M.VISIBLE_LAST; n += 1) {
      var x = xForValue(1 / n);
      dots += '<circle class="band-dot sc-dot" data-term="' + n + '" cx="' + x.toFixed(2) +
        '" cy="' + BAND.axisY + '" r="10"><title>a' + n + " = " + esc(M.fmtTerm(n)) + "</title></circle>";
    }

    var indexTicks = "";
    for (var i = BAND.indexFirst; i <= BAND.indexLast; i += 1) {
      var ix = xForIndex(i);
      indexTicks += '<line class="sc-index-tick" x1="' + ix.toFixed(2) + '" x2="' + ix.toFixed(2) +
        '" y1="' + (BAND.indexY + 4) + '" y2="' + (BAND.indexY + 12) + '"/>';
    }
    var indexLabels = [1, 5, 10, 15, 20].map(function (n) {
      var x = xForIndex(n);
      var anchor = n === 1 ? "start" : n === 20 ? "end" : "middle";
      return '<text class="sc-index-label" x="' + x.toFixed(2) + '" y="' + (BAND.indexY + 38) +
        '" text-anchor="' + anchor + '">' + n + "</text>";
    }).join("");

    $("#band-layer").innerHTML =
      '<text class="sc-axis-name" x="' + BAND.x0 + '" y="34">值轴 · 0 附近放大</text>' +
      '<text class="sc-axis-name scene-top-right" x="' + BAND.x1 + '" y="34" text-anchor="end">n = 4 … 20</text>' +
      '<rect class="sc-band" id="band-rect" x="' + BAND.x0 + '" y="' + BAND.valueTop +
      '" width="0" height="' + (BAND.valueBottom - BAND.valueTop) + '" rx="8"/>' +
      '<text class="sc-band-label" x="' + (BAND.x0 + 10) + '" y="' + (BAND.valueTop + 26) +
      '">带内 0 &lt; 1/n &lt; ε</text>' +
      '<line class="sc-axis" x1="' + BAND.x0 + '" x2="' + BAND.x1 +
      '" y1="' + BAND.axisY + '" y2="' + BAND.axisY + '"/>' +
      ticks +
      '<line class="sc-eps-line" id="band-eps-line" x1="0" x2="0" y1="54" y2="232"/>' +
      '<text class="sc-eps-label" id="band-eps-label" x="0" y="60">ε</text>' +
      '<g id="band-dots">' + dots + "</g>" +
      '<text class="sc-focus-label" id="band-focus-label" x="0" y="140" text-anchor="middle" opacity="0"></text>' +
      '<line class="sc-n-line" id="band-n-line" x1="0" x2="0" y1="268" y2="' + BAND.indexY + '"/>' +
      '<g class="sc-n-pill" id="band-n-pill">' +
        '<rect id="band-n-rect" x="0" y="238" width="86" height="27" rx="13.5"/>' +
        '<text id="band-n-text" x="0" y="257" text-anchor="middle">N = 5</text>' +
      "</g>" +
      '<line class="sc-index-axis" x1="' + BAND.x0 + '" x2="' + BAND.x1 +
      '" y1="' + BAND.indexY + '" y2="' + BAND.indexY + '"/>' +
      '<rect class="sc-index-tail" id="band-index-tail" x="0" y="' + (BAND.indexY - 5) +
      '" width="0" height="10" rx="5"/>' +
      indexTicks +
      indexLabels;
  }

  function renderBand() {
    var epsilon = state.epsilon;
    var checks = M.strictChecks(epsilon);
    var N = checks.N;
    var inside = M.insideVisibleCount(epsilon);
    var exact = M.exactReciprocal(epsilon);
    var bandWidth = Math.max(3, xForValue(epsilon) - BAND.x0);
    var epsX = xForValue(epsilon);

    $("#band-rect").setAttribute("width", bandWidth.toFixed(2));
    $("#band-eps-line").setAttribute("x1", epsX.toFixed(2));
    $("#band-eps-line").setAttribute("x2", epsX.toFixed(2));
    var epsAnchor = epsX > BAND.x1 - 130 ? "end" : "start";
    $("#band-eps-label").setAttribute("x", (epsX + (epsAnchor === "end" ? -6 : 6)).toFixed(2));
    $("#band-eps-label").setAttribute("text-anchor", epsAnchor);
    $("#band-eps-label").textContent = "ε = " + M.fmtEpsilon(epsilon);

    $$("#band-dots .band-dot").forEach(function (dot) {
      var n = Number(dot.dataset.term);
      var isIn = M.isInside(n, epsilon);
      var isFocus = n === N;
      dot.setAttribute("class", "band-dot sc-dot " + (isIn ? "is-in" : "is-out") + (isFocus ? " is-focus" : ""));
      dot.setAttribute("r", isFocus ? "13" : "10");
    });

    var focusLabel = $("#band-focus-label");
    if (N <= M.VISIBLE_LAST) {
      focusLabel.setAttribute("x", xForValue(1 / N).toFixed(2));
      focusLabel.setAttribute("opacity", "1");
      focusLabel.textContent = "a" + subscript(N);
    } else {
      focusLabel.setAttribute("opacity", "0");
      focusLabel.textContent = "";
    }

    var markerX = xForIndex(N);
    var pillText = N <= M.VISIBLE_LAST ? "N = " + N : "N = " + N + " →";
    var pillWidth = Math.max(84, pillText.length * 14 + 26);
    var pillX = M.clamp(markerX - pillWidth / 2, BAND.x0, BAND.x1 - pillWidth);
    $("#band-n-rect").setAttribute("x", pillX.toFixed(2));
    $("#band-n-rect").setAttribute("width", pillWidth.toFixed(2));
    $("#band-n-text").setAttribute("x", (pillX + pillWidth / 2).toFixed(2));
    $("#band-n-text").textContent = pillText;
    $("#band-n-line").setAttribute("x1", markerX.toFixed(2));
    $("#band-n-line").setAttribute("x2", markerX.toFixed(2));

    var tail = $("#band-index-tail");
    if (N <= M.VISIBLE_LAST) {
      tail.setAttribute("x", markerX.toFixed(2));
      tail.setAttribute("width", Math.max(0, BAND.x1 - markerX).toFixed(2));
      tail.setAttribute("opacity", "1");
    } else {
      tail.setAttribute("opacity", "0");
    }

    dom.epsValue.textContent = "ε = " + M.fmtEpsilon(epsilon);
    dom.epsRange.value = String(epsilon);
    dom.epsRange.setAttribute(
      "aria-valuetext",
      "ε = " + M.fmtEpsilon(epsilon) + "，N = " + N + "，画面里 " + inside + " 项在带内"
    );
    dom.nFormula.textContent = "N = ⌊1/" + M.fmtEpsilon(epsilon) + "⌋ + 1 = " + N;
    dom.checkFirst.textContent = "1/N < ε：1/" + N + " = " + M.fmtTight(checks.first) + " < " + M.fmtTight(epsilon) + " ✓";
    dom.checkPrev.textContent = N > 1
      ? "1/(N−1) ≥ ε：1/" + (N - 1) + " = " + M.fmtTight(checks.previous) + " ≥ " + M.fmtTight(epsilon) + " ✓"
      : "N = 1";

    var caption;
    if (N > M.VISIBLE_LAST) {
      caption = "ε = " + M.fmtEpsilon(epsilon) + "：画面里没有项在带内；N = " + N +
        " 已在序号轴外，但定义仍要求 n ≥ N 的所有项。";
    } else {
      caption = "ε = " + M.fmtEpsilon(epsilon) + "：画面里 n = " + N + " … 20 在带内；N = " + N + "。";
    }
    if (exact !== null && exact <= M.VISIBLE_LAST + 1) {
      caption += " 1/ε = " + exact + " 是整数，a" + subscript(exact) + " = ε 不算严格小于。";
    }
    dom.bandCaption.textContent = caption;

    dom.bandStage.dataset.eps = M.fmtEpsilon(epsilon);
    dom.bandStage.dataset.n = String(N);
    dom.bandStage.dataset.okFirst = String(checks.firstOk);
    dom.bandStage.dataset.okPrev = String(checks.previousOk);
    dom.bandStage.dataset.inside = String(inside);

    $$(".preset").forEach(function (button) {
      var active = Math.abs(Number(button.dataset.eps) - epsilon) <= 1e-9;
      button.setAttribute("aria-pressed", String(active));
    });
  }

  function announceBand() {
    var checks = M.strictChecks(state.epsilon);
    announce("ε = " + M.fmtEpsilon(state.epsilon) + "，N = " + checks.N + "。");
  }

  // ── screens ────────────────────────────────────────────────
  function updateProgress(step) {
    dom.progress.hidden = step === 0;
    dom.progressLabel.textContent = "第 " + step + " / " + STEP_COUNT + " 步";
    dom.progressSegs.forEach(function (segment, index) {
      segment.classList.toggle("is-on", index < step);
      segment.classList.toggle("is-current", index === step - 1);
    });
  }

  function showScreen(name) {
    if (SCREEN_ORDER.indexOf(name) < 0 || name === state.screen) return;
    state.screen = name;
    var target = $('.screen[data-screen="' + name + '"]');
    $$(".screen").forEach(function (screen) {
      screen.hidden = screen !== target;
    });
    updateProgress(Number(target.dataset.step));
    window.scrollTo(0, 0);
    var expression = name === "sequence" ? "curious" : name === "band" ? "thinking" : "curious";
    if (name === "summary") expression = "happy";
    setExpression(expression);
    if (name === "band") renderBand();
    var heading = target.querySelector("h1, h2");
    if (heading) heading.focus({ preventScroll: true });
  }

  function resetQuestion(id) {
    var qs = state.questions[id];
    qs.solved = false;
    qs.wrong = false;
    qs.selected = null;
    qs.hintOpen = false;
    var card = $('.question-card[data-question="' + id + '"]');
    if (!card) return;
    var options = $$(".option", card);
    options.forEach(function (option) {
      option.disabled = false;
      option.classList.remove("is-wrong", "is-correct");
      option.removeAttribute("aria-pressed");
      option.removeAttribute("aria-invalid");
    });
    var group = $(".options", card);
    if (group) group.classList.remove("is-locked");
    var feedback = $(".feedback", card);
    if (feedback) {
      feedback.textContent = "";
      feedback.className = "feedback";
    }
    var hint = $(".hint", card);
    if (hint) {
      hint.hidden = true;
      hint.textContent = "";
    }
    var hintButton = $(".hint-btn", card);
    if (hintButton) {
      hintButton.disabled = false;
      hintButton.setAttribute("aria-expanded", "false");
      hintButton.textContent = "需要提示";
    }
    var note = $("#" + id + "-note");
    if (note) note.textContent = "答对后才能继续";
    var next = $('[data-guard="' + id + '"]');
    if (next) next.disabled = true;
  }

  function resetLesson() {
    state.epsilon = DATA.scene.defaultEpsilon;
    DATA.questions.forEach(function (question) { resetQuestion(question.id); });
    renderBand();
    state.screen = "";
    showScreen("cover");
    updateProgress(0);
    window.scrollTo(0, 0);
  }

  function solveQuestion(id, option) {
    var question = QUESTIONS[id];
    var qs = state.questions[id];
    if (!question || qs.solved) return;
    qs.solved = true;
    qs.wrong = false;
    qs.selected = option.dataset.choice;
    var card = $('.question-card[data-question="' + id + '"]');
    var group = $(".options", card);
    if (group) group.classList.add("is-locked");
    $$(".option", card).forEach(function (choice) {
      choice.disabled = true;
      choice.classList.remove("is-wrong");
      choice.removeAttribute("aria-invalid");
    });
    option.classList.add("is-correct");
    option.setAttribute("aria-pressed", "true");
    var feedback = $(".feedback", card);
    if (feedback) {
      feedback.className = "feedback is-correct";
      feedback.textContent = "✓ 对了。" + question.explanation;
    }
    var hintButton = $(".hint-btn", card);
    if (hintButton) hintButton.disabled = true;
    var note = $("#" + id + "-note");
    if (note) note.textContent = "答对了，继续吧";
    var next = $('[data-guard="' + id + '"]');
    if (next) {
      next.disabled = false;
      next.focus();
    }
    setExpression("happy");
  }

  function rejectQuestion(id, option) {
    var question = QUESTIONS[id];
    var qs = state.questions[id];
    if (!question || qs.solved) return;
    qs.wrong = true;
    qs.selected = option.dataset.choice;
    $$('.question-card[data-question="' + id + '"] .option').forEach(function (choice) {
      choice.classList.toggle("is-wrong", choice === option);
      if (choice === option) choice.setAttribute("aria-invalid", "true");
      else choice.removeAttribute("aria-invalid");
      choice.setAttribute("aria-pressed", choice === option ? "true" : "false");
    });
    var message = (question.wrong && question.wrong[option.dataset.choice]) ||
      "再看一眼带内的条件：1/n < ε。";
    var feedback = $('.question-card[data-question="' + id + '"] .feedback');
    if (feedback) {
      feedback.className = "feedback is-wrong";
      feedback.textContent = "还不对。" + message;
    }
    var note = $("#" + id + "-note");
    if (note) note.textContent = "再试一次";
    setExpression("oops");
  }

  function toggleHint(id) {
    var question = QUESTIONS[id];
    var qs = state.questions[id];
    var card = $('.question-card[data-question="' + id + '"]');
    var hint = $(".hint", card);
    var button = $(".hint-btn", card);
    if (!question || !hint || !button || qs.solved) return;
    qs.hintOpen = !qs.hintOpen;
    hint.hidden = !qs.hintOpen;
    button.setAttribute("aria-expanded", String(qs.hintOpen));
    button.textContent = qs.hintOpen ? "收起提示" : "需要提示";
    if (qs.hintOpen) {
      hint.textContent = "";
      var name = document.createElement("span");
      name.className = "hint-name";
      name.textContent = "阿理：";
      hint.appendChild(name);
      hint.appendChild(document.createTextNode(question.hint));
      setExpression("thinking");
    } else {
      hint.textContent = "";
      setExpression(questionExpression(id));
    }
  }

  function parseChoice(question, raw) {
    return question.kind === "boolean" ? raw === "true" : Number(raw);
  }

  // ── events ─────────────────────────────────────────────────
  document.addEventListener("click", function (event) {
    var goto = event.target.closest("[data-goto]");
    if (goto) {
      if (goto.disabled) return;
      if (goto.dataset.guard && !state.questions[goto.dataset.guard].solved) return;
      showScreen(goto.dataset.goto);
      return;
    }

    var replay = event.target.closest('[data-action="replay"]');
    if (replay) {
      resetLesson();
      return;
    }

    var option = event.target.closest(".option");
    if (option && !option.disabled) {
      var group = option.closest("[data-options]");
      if (!group) return;
      var id = group.dataset.options;
      var question = QUESTIONS[id];
      var qs = state.questions[id];
      if (!question || !qs || qs.solved) return;
      var value = parseChoice(question, option.dataset.choice);
      if (value === question.answer) solveQuestion(id, option);
      else rejectQuestion(id, option);
      return;
    }

    var hintButton = event.target.closest("[data-hint]");
    if (hintButton) {
      toggleHint(hintButton.dataset.hint);
      return;
    }

    var preset = event.target.closest(".preset");
    if (preset) {
      state.epsilon = M.quantizeEpsilon(Number(preset.dataset.eps));
      renderBand();
      announceBand();
    }
  });

  dom.epsRange.addEventListener("input", function () {
    state.epsilon = M.quantizeEpsilon(Number(dom.epsRange.value));
    renderBand();
  });

  dom.epsRange.addEventListener("change", function () {
    state.epsilon = M.quantizeEpsilon(Number(dom.epsRange.value));
    renderBand();
    announceBand();
  });

  $$(".options").forEach(function (group) {
    group.addEventListener("keydown", function (event) {
      var keys = ["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown"];
      if (keys.indexOf(event.key) < 0) return;
      var buttons = $$(".option:not([disabled])", group);
      var index = buttons.indexOf(document.activeElement);
      if (index < 0) return;
      event.preventDefault();
      var step = event.key === "ArrowRight" || event.key === "ArrowDown" ? 1 : -1;
      buttons[(index + step + buttons.length) % buttons.length].focus();
    });
  });

  // ── boot ───────────────────────────────────────────────────
  mountCompanions();
  buildSequence();
  buildBand();
  renderBand();
  updateProgress(0);
  setExpression("curious");
})();

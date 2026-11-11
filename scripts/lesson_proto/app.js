(function () {
  "use strict";

  var data = JSON.parse(document.getElementById("lesson-data").textContent);
  var TOTAL = data.steps.length;
  var GRADED = data.steps.filter(function (step) {
    return step.type === "choice";
  }).length;
  var MAX_XP = data.xp_total;
  var reduceMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  var state = {
    index: 0,
    xp: 0,
    streak: 0,
    maxStreak: 0,
    hintsUsed: 0,
    firstTryCount: 0,
    hintTier: 0,
    wrongAttempts: 0,
    attempts: 0,
    solved: false,
    completed: false,
    feedback: null,
    wrongChoice: null,
    markerValue: null,
    startedAt: null,
    lastXp: 0,
    lastStreak: 0,
    awarded: {},
    help: null,
    helpTimer: null,
    helpEnds: 0,
    announceTimer: null,
    toastTimer: null
  };

  function $(selector, root) {
    return (root || document).querySelector(selector);
  }

  function $$(selector, root) {
    return Array.prototype.slice.call((root || document).querySelectorAll(selector));
  }

  function esc(value) {
    return String(value).replace(/[&<>"']/g, function (character) {
      return {
        "&": "&amp;",
        "<": "&lt;",
        ">": "&gt;",
        '"': "&quot;",
        "'": "&#39;"
      }[character];
    });
  }

  function stepAt(index) {
    return data.steps[index];
  }

  function symbolAt(id) {
    return data.symbols[id];
  }

  function announce(message) {
    var node = document.getElementById("announcer");
    if (!node) {
      return;
    }
    window.clearTimeout(state.announceTimer);
    node.textContent = "";
    state.announceTimer = window.setTimeout(function () {
      node.textContent = message;
    }, 30);
  }

  function showToast(message) {
    var node = document.getElementById("toast");
    if (!node) {
      return;
    }
    window.clearTimeout(state.toastTimer);
    node.textContent = message;
    node.classList.add("is-visible");
    state.toastTimer = window.setTimeout(function () {
      node.classList.remove("is-visible");
    }, 2400);
  }

  function popStat(node) {
    if (!node) {
      return;
    }
    node.classList.remove("is-bump");
    void node.offsetWidth;
    node.classList.add("is-bump");
    window.clearTimeout(node._bumpTimer);
    node._bumpTimer = window.setTimeout(function () {
      node.classList.remove("is-bump");
    }, 280);
  }

  function pairHTML(values, label) {
    var slots = values
      .map(function (value) {
        var unknown = value === "?";
        return (
          '<span class="slot' +
          (unknown ? " is-unknown" : "") +
          '">' +
          esc(String(value)) +
          "</span>"
        );
      })
      .join('<span class="comma" aria-hidden="true">,</span>');
    return (
      '<span class="pair" aria-label="' +
      esc(label || "二元组 " + values.join("，")) +
      '"><span class="paren" aria-hidden="true">(</span>' +
      slots +
      '<span class="paren" aria-hidden="true">)</span></span>'
    );
  }

  function chipDots(count, tone, options) {
    options = options || {};
    var chips = [];
    for (var index = 0; index < count; index += 1) {
      var cancelled =
        tone === "blue"
          ? typeof options.cancelFrom === "number" && index >= options.cancelFrom
          : typeof options.cancelRed === "number" && index < options.cancelRed;
      chips.push(
        '<span class="chip tone-' +
          tone +
          (cancelled ? " is-cancelled" : "") +
          '" style="--i:' +
          index +
          '"></span>'
      );
    }
    if (!chips.length) {
      return '<span class="chip-empty" aria-hidden="true">0</span>';
    }
    return (
      '<span class="chip-dots" aria-hidden="true">' + chips.join("") + "</span>"
    );
  }

  function chipLine(label, count, tone, options) {
    return (
      '<div class="chip-line"><span class="chip-key tone-' +
      tone +
      '">' +
      esc(label) +
      " <b>" +
      count +
      "</b></span>" +
      chipDots(count, tone, options) +
      "</div>"
    );
  }

  function pairChips(a, b, options) {
    options = options || {};
    return (
      '<div class="chip-block">' +
      chipLine("蓝", a, "blue", options) +
      chipLine("红", b, "red", options) +
      "</div>"
    );
  }

  function chipsLegend(visual, withCancel) {
    var legend = visual.legend || {};
    return (
      '<div class="chips-legend"><span class="chip tone-blue"></span>' +
      esc(legend.blue || "蓝 = 正向") +
      '<span class="chip tone-red"></span>' +
      esc(legend.red || "红 = 负向") +
      (withCancel
        ? '<span class="legend-cancel"><span class="chip tone-blue is-cancelled"></span><span class="chip tone-red is-cancelled"></span>' +
          esc(legend.cancel || "一对抵消") +
          "</span>"
        : "") +
      "</div>"
    );
  }

  function mathHTML(value) {
    var parts = String(value).split(" = ");
    if (parts.length !== 2) {
      return esc(value);
    }
    return (
      '<span class="chip-eq-lhs">' +
      esc(parts[0]) +
      '</span><span class="chip-eq-op" aria-hidden="true">=</span><strong class="chip-result">' +
      esc(parts[1]) +
      "</strong>"
    );
  }

  function numberlinePosition(value) {
    return ((Number(value) + 5) / 10) * 100;
  }

  function staticMarker(value, tone, revealed) {
    var label =
      value > 0 ? "+" + value + "°" : value < 0 ? value + "°" : "0°";
    return (
      '<span class="nl-marker tone-' +
      tone +
      " is-static" +
      (revealed ? " is-revealed" : "") +
      '" style="--x:' +
      numberlinePosition(value) +
      '%" aria-hidden="true"><span class="nl-bubble">' +
      esc(label) +
      '</span><span class="nl-dot"></span></span>'
    );
  }

  function numberlineVisual(visual, mode) {
    var ticks = [];
    for (var value = -5; value <= 5; value += 1) {
      var major = value === 0 || Math.abs(value) === 3 || Math.abs(value) === 5;
      var label = major
        ? "<span>" + (value === 0 ? "0" : value > 0 ? "+" + value : value) + "</span>"
        : "";
      ticks.push(
        '<span class="nl-tick' +
          (major ? " is-major" : "") +
          '" style="--x:' +
          numberlinePosition(value) +
          '%" aria-hidden="true">' +
          label +
          "</span>"
      );
    }

    var markers = "";
    var ghost = "";
    var caption = visual.caption || "";
    var ariaLabel = "数轴：−3 与 +3 关于 0 对称";

    if (mode === "interactive") {
      markers = visual.markers
        .map(function (marker) {
          var active = state.markerValue === marker.value;
          return (
            '<button class="nl-marker tone-' +
            esc(marker.tone) +
            (active ? " is-active" : "") +
            '" style="--x:' +
            numberlinePosition(marker.value) +
            '%" type="button" data-marker="' +
            esc(marker.value) +
            '" data-note="' +
            esc(marker.note) +
            '" aria-pressed="' +
            (active ? "true" : "false") +
            '" aria-label="选择数轴上的 ' +
            esc(marker.label) +
            '"><span class="nl-bubble">' +
            esc(marker.label) +
            '</span><span class="nl-dot"></span></button>'
          );
        })
        .join("");
      caption = state.markerValue === null ? visual.default_note : caption;
      ariaLabel = "可点的温度数轴：−3 与 +3 关于 0 对称";
    } else if (mode === "quiz") {
      markers = staticMarker(Number(visual.value), "warm", false);
      if (state.solved) {
        markers += staticMarker(-Number(visual.value), "cold", true);
        caption = "找到了：−3 与 +3 关于 0 对称。";
        ariaLabel = "数轴：−3 与 +3 关于 0 对称";
      } else {
        ghost =
          '<span class="nl-ghost" style="--x:' +
          numberlinePosition(-Number(visual.value)) +
          '%" aria-hidden="true">?</span>';
        caption = visual.caption || "";
        ariaLabel = "数轴：3 在 0 右边 3 格";
      }
    }

    var captionId = mode === "interactive" ? ' id="markerCaption" aria-live="polite"' : "";
    return (
      '<div class="numberline-wrap"><div class="numberline" role="' +
      (mode === "interactive" ? "group" : "img") +
      '" aria-label="' +
      esc(ariaLabel) +
      '"><div class="nl-track"><span class="nl-rail" aria-hidden="true"></span>' +
      ticks.join("") +
      '<span class="nl-zero" aria-hidden="true"></span>' +
      ghost +
      markers +
      '</div></div><p class="nl-caption"' +
      captionId +
      ">" +
      esc(caption) +
      "</p></div>"
    );
  }

  function pairsIntroVisual(visual) {
    var cards = (visual.cards || [])
      .map(function (card, index) {
        var pair = card.pair || [0, 0];
        return (
          '<article class="chip-card' +
          (index === 1 ? " is-negative" : "") +
          '"><div class="chip-card-head"><span class="chip-pair">(' +
          pair[0] +
          "," +
          pair[1] +
          ')</span><span class="chip-tag">' +
          esc(card.tag) +
          '</span></div><p class="chip-meaning">' +
          esc(card.meaning) +
          "</p>" +
          pairChips(pair[0], pair[1], {}) +
          '<div class="chip-eq">' +
          mathHTML(card.math) +
          "</div></article>"
        );
      })
      .join("");

    return (
      '<div class="chips-board">' +
      chipsLegend(visual, false) +
      '<div class="chip-card-grid">' +
      cards +
      "</div>" +
      '<p class="board-caption">' +
      esc(visual.caption) +
      "</p></div>"
    );
  }

  function pairFamilyVisual(visual) {
    var rows = (visual.rows || [])
      .map(function (row) {
        var pair = row.pair || [0, 0];
        // The base 3 blue chips are kept; every added blue/red pair cancels.
        var options = { cancelFrom: 3, cancelRed: row.added };
        return (
          '<div class="family-row"><div class="family-row-head"><span class="family-pair">(' +
          pair[0] +
          "," +
          pair[1] +
          ')</span><span class="family-note">' +
          esc(row.note) +
          '</span><span class="family-math">' +
          esc(row.math) +
          "</span></div>" +
          pairChips(pair[0], pair[1], options) +
          "</div>"
        );
      })
      .join("");

    return (
      '<div class="family-board' +
      (state.solved ? " is-solved" : "") +
      '">' +
      chipsLegend(visual, true) +
      '<div class="family-rows">' +
      rows +
      "</div>" +
      '<p class="board-caption">' +
      esc(state.solved ? visual.result_caption : visual.caption) +
      "</p></div>"
    );
  }

  function renderVisual(step) {
    var visual = step.visual || {};

    if (visual.type === "numberline") {
      return numberlineVisual(visual, "interactive");
    }

    if (visual.type === "numberline_quiz") {
      return numberlineVisual(visual, "quiz");
    }

    if (visual.type === "pairs_intro") {
      return pairsIntroVisual(visual);
    }

    if (visual.type === "pair_family") {
      return pairFamilyVisual(visual);
    }

    if (visual.type === "pair_swap") {
      var solved = state.solved;
      return (
        '<div class="pairboard' +
        (solved ? " is-solved" : "") +
        '"><div class="pair-stage"><span class="pair-caption">取负前</span>' +
        pairHTML(visual.before) +
        pairChips(visual.before[0], visual.before[1], {}) +
        '</div><div class="swap-arrow" aria-hidden="true"><span>⇄</span><small>交换计数器</small></div>' +
        '<div class="pair-stage is-answer"><span class="pair-caption">取负后</span>' +
        (solved
          ? pairHTML(visual.after, "取负后的二元组") +
            pairChips(visual.after[0], visual.after[1], {})
          : '<span class="pair-pending" aria-hidden="true">?</span>') +
        "</div></div><p class=\"board-caption\">" +
        esc(solved ? visual.after_caption : visual.caption) +
        "</p>"
      );
    }

    if (visual.type === "pair_sum") {
      var sumSolved = state.solved;
      var sum = visual.sum || [0, 0];
      var cancelOptions = sumSolved ? { cancelFrom: 0, cancelRed: sum[1] } : {};
      var cancelCount = Math.min(sum[0], sum[1]);
      return (
        '<div class="sum-board' +
        (sumSolved ? " is-solved" : "") +
        '"><div class="sum-line">' +
        pairHTML(visual.left) +
        '<span class="op" aria-hidden="true">+</span>' +
        pairHTML(visual.right) +
        '<span class="op" aria-hidden="true">=</span>' +
        pairHTML(sum) +
        '</div><div class="sum-chips"><div class="sum-chip-col"><span class="sum-chip-label">蓝：' +
        visual.left[0] +
        " + " +
        visual.right[0] +
        " = " +
        sum[0] +
        "</span>" +
        chipDots(sum[0], "blue", cancelOptions) +
        '</div><div class="sum-chip-col"><span class="sum-chip-label">红：' +
        visual.left[1] +
        " + " +
        visual.right[1] +
        " = " +
        sum[1] +
        "</span>" +
        chipDots(sum[1], "red", cancelOptions) +
        '</div></div><div class="cancel-strip"><span class="cancel-badge">' +
        cancelCount +
        ' 对蓝红互相抵消</span><span class="cancel-arrow" aria-hidden="true">⇒</span><strong class="cancel-result">' +
        (sumSolved ? "0" : "?") +
        "</strong></div><p class=\"board-caption\">" +
        esc(sumSolved ? visual.result_caption : visual.caption) +
        "</p></div>"
      );
    }

    if (visual.type === "conclusion") {
      var chips = visual.chips || {};
      return (
        '<div class="conclusion-board"><div class="target" aria-hidden="true"><span>0</span></div>' +
        '<div class="conclusion-eq">' +
        esc(visual.equation) +
        '</div><div class="conclusion-flow" aria-hidden="true">' +
        chipDots(chips.blue, "blue", {}) +
        '<span class="flow-op">+</span>' +
        chipDots(chips.red, "red", {}) +
        '<span class="flow-op">=</span><span class="flow-zero">' +
        esc(chips.result) +
        '</span></div><div class="conclusion-chip">' +
        esc(visual.chip) +
        "</div></div>"
      );
    }

    return "";
  }

  function renderFormulaHelp(step) {
    var first = symbolAt(step.help[0]);
    var chips = step.help
      .map(function (id) {
        var symbol = symbolAt(id);
        return (
          '<button class="help-chip" type="button" data-symbol="' +
          esc(id) +
          '" aria-label="打开 20 秒符号卡：' +
          esc(symbol.term) +
          '"><span class="help-q" aria-hidden="true">?</span>' +
          esc(symbol.term) +
          "</button>"
        );
      })
      .join("");

    return (
      '<div class="formula-row"><div class="formula" aria-label="' +
      esc(step.formula) +
      '">' +
      esc(step.formula) +
      '</div><button class="help-btn" type="button" data-symbol="' +
      esc(step.help[0]) +
      '" aria-label="打开 20 秒符号卡：' +
      esc(first.term) +
      '" title="20 秒符号卡">?</button></div>' +
      '<div class="symbol-help"><span class="symbol-help-label">符号小帮手</span>' +
      chips +
      "</div>"
    );
  }

  function renderChoices(step) {
    var choices = step.choices
      .map(function (choice, index) {
        var isCorrect = state.solved && choice.id === step.answer;
        var isWrong = !state.solved && state.wrongChoice === choice.id;
        var mark = isCorrect ? "✓" : isWrong ? "!" : "";
        return (
          '<button class="choice' +
          (choice.sub ? " is-sentence" : "") +
          (isCorrect ? " is-correct" : "") +
          (isWrong ? " is-wrong" : "") +
          '" type="button" data-choice-id="' +
          esc(choice.id) +
          '" data-choice-index="' +
          (index + 1) +
          '" aria-pressed="' +
          (isCorrect ? "true" : "false") +
          '" aria-label="' +
          esc(choice.label + "，" + choice.aria) +
          '"' +
          (state.solved ? " disabled" : "") +
          "><span class=\"choice-key\" aria-hidden=\"true\">" +
          (index + 1) +
          '</span><span class="choice-copy"><span class="choice-label">' +
          esc(choice.label) +
          "</span>" +
          (choice.sub ? '<span class="choice-sub">' + esc(choice.sub) + "</span>" : "") +
          '</span><span class="choice-mark" aria-hidden="true">' +
          mark +
          "</span></button>"
        );
      })
      .join("");
    return '<div class="choices" role="group" aria-label="选项">' + choices + "</div>";
  }

  function renderFeedback(step) {
    if (!state.feedback) {
      return "";
    }
    var feedback = state.feedback;
    var icon = feedback.kind === "correct" ? "✓" : feedback.kind === "wrong" ? "!" : "?";
    var meta = "";
    if (feedback.kind === "correct") {
      meta +=
        '<span class="xp-award">+' +
        feedback.xpEarned +
        " XP" +
        (feedback.bonus > 0 ? " · 首次答对 +" + feedback.bonus : "") +
        "</span>";
    } else if (feedback.kind === "hint") {
      meta +=
        '<span class="hint-meta">提示 ' +
        state.hintTier +
        " / " +
        step.hints.length +
        " · 不扣 XP</span>";
    } else {
      meta += '<span class="hint-meta">再试一次 · 不扣 XP</span>';
    }
    return (
      '<div class="feedback is-' +
      feedback.kind +
      '" id="feedback" tabindex="-1"><div class="feedback-head"><span class="feedback-icon" aria-hidden="true">' +
      icon +
      '</span><div class="feedback-copy"><p class="feedback-title">' +
      esc(feedback.title) +
      '</p><p class="feedback-body">' +
      esc(feedback.body) +
      '</p><div class="feedback-meta">' +
      meta +
      "</div></div></div></div>"
    );
  }

  function renderActions(step) {
    if (step.type === "goal") {
      return (
        '<div class="action-row"><button class="btn btn-primary btn-wide" type="button" data-action="start" id="primaryAction">' +
        esc(step.primary) +
        ' <span aria-hidden="true">→</span></button><span class="micro">点选即可 · 无需输入</span></div>'
      );
    }

    if (step.type === "insight") {
      return (
        '<div class="action-row"><button class="btn btn-primary btn-wide" type="button" data-action="continue" id="primaryAction">' +
        esc(step.primary) +
        ' <span aria-hidden="true">→</span></button><span class="micro">看懂了就继续 · 无判分题</span></div>'
      );
    }

    if (state.solved) {
      return (
        '<div class="action-row"><button class="btn btn-primary btn-wide" type="button" data-action="continue" id="primaryAction">' +
        esc(step.continue_label) +
        ' <span aria-hidden="true">→</span></button></div>'
      );
    }

    var allHints = state.hintTier >= step.hints.length;
    var hintLabel = allHints
      ? "提示已用完"
      : state.hintTier === 0
        ? "提示"
        : "下一条提示 " + (state.hintTier + 1) + "/" + step.hints.length;
    var note =
      state.hintTier === 0
        ? "点选即判定 · 提示不扣 XP"
        : "再点选一次 · 提示不扣 XP";

    return (
      '<div class="action-row"><button class="btn btn-hint" type="button" data-action="hint"' +
      (allHints ? " disabled" : "") +
      '><span aria-hidden="true">💡</span> ' +
      hintLabel +
      '</button><span class="action-note">' +
      note +
      "</span></div>"
    );
  }

  function renderStep() {
    var step = stepAt(state.index);
    var parts = [];
    parts.push(
      '<article class="step-card" data-step-id="' +
        esc(step.id) +
        '" data-step-type="' +
        esc(step.type) +
        '" aria-labelledby="stepTitle">'
    );
    parts.push(
      '<div class="step-top"><span class="kicker">' +
        esc(step.kicker) +
        '</span><span class="step-mini">' +
        (state.index + 1) +
        " / " +
        TOTAL +
        "</span></div>"
    );
    parts.push(
      '<h1 class="step-title" id="stepTitle" tabindex="-1">' + esc(step.title) + "</h1>"
    );

    if (step.type === "goal") {
      parts.push('<div class="body-copy">' + esc(step.body) + "</div>");
      parts.push(
        '<div class="goal-meta"><span class="meta-pill">' +
          TOTAL +
          " 个互动</span><span class=\"meta-pill\">无需写代码</span><span class=\"meta-pill\">约 " +
          data.estimated_minutes +
          " 分钟</span></div>"
      );
    } else if (step.type === "insight") {
      parts.push('<div class="body-copy">' + esc(step.body) + "</div>");
      parts.push('<div class="body-copy is-second">' + esc(step.body2) + "</div>");
    } else {
      parts.push('<div class="body-copy">' + esc(step.prompt) + "</div>");
    }

    parts.push(renderFormulaHelp(step));
    parts.push(renderVisual(step));
    if (step.type === "insight") {
      parts.push(
        '<p class="insight-callout"><span class="insight-icon" aria-hidden="true">💡</span><span>' +
          esc(step.callout) +
          "</span></p>"
      );
    }
    if (step.type === "choice") {
      parts.push(renderChoices(step));
      parts.push(renderFeedback(step));
    }
    parts.push(renderActions(step));
    parts.push("</article>");
    document.getElementById("main").innerHTML = parts.join("");
  }

  function renderJourney() {
    var journey = document.getElementById("journey");
    if (!journey) {
      return;
    }
    var items = data.steps
      .map(function (step, index) {
        var isCurrent = !state.completed && index === state.index;
        var isDone =
          state.completed || index < state.index || (index === state.index && state.solved);
        var marker = isDone ? "✓" : String(index + 1);
        var classes =
          "journey-item" +
          (isDone ? " is-done" : "") +
          (isCurrent ? " is-current" : "");
        return (
          '<li class="' +
          classes +
          '"' +
          (isCurrent ? ' aria-current="step"' : "") +
          '><span class="j-marker" aria-hidden="true">' +
          marker +
          '</span><span class="j-copy"><strong>' +
          esc(step.short) +
          "</strong><small>" +
          esc(step.title) +
          "</small></span></li>"
        );
      })
      .join("");

    var foot = state.completed
      ? '<div class="journey-unlock">✓ 徽章已解锁 · ' + esc(data.badge.title) + "</div>"
      : '<div class="journey-foot">完成 ' +
        TOTAL +
        " 步，解锁「" +
        esc(data.badge.title) +
        "」</div>";

    journey.innerHTML =
      '<div class="journey-card"><p class="journey-kicker">本课路线</p><ol class="journey-list">' +
      items +
      "</ol>" +
      foot +
      "</div>";
  }

  function formatDuration() {
    if (!state.startedAt) {
      return "—";
    }
    var seconds = Math.max(1, Math.round((Date.now() - state.startedAt) / 1000));
    if (seconds < 60) {
      return seconds + " 秒";
    }
    var minutes = Math.floor(seconds / 60);
    var rest = seconds % 60;
    return rest ? minutes + " 分 " + rest + " 秒" : minutes + " 分钟";
  }

  function renderCompletion() {
    var completion = data.completion;
    var proof = data.formal_proof;
    var firstTryRate =
      GRADED > 0 ? Math.round((state.firstTryCount / GRADED) * 100) : 0;
    var theorems = proof.theorems
      .map(function (theorem) {
        return (
          '<li class="theorem"><div class="theorem-head"><code>' +
          esc(theorem.name) +
          '</code><span class="theorem-kind">' +
          esc(theorem.kind) +
          '</span></div><p>' +
          esc(theorem.plain) +
          "</p></li>"
        );
      })
      .join("");

    var stats = [
      ["本轮 XP", state.xp, "/ " + MAX_XP + " XP"],
      ["首次答对", state.firstTryCount + " / " + GRADED, firstTryRate + "%"],
      ["最高连对", state.maxStreak, "共 " + GRADED + " 道"],
      ["用时", formatDuration(), state.hintsUsed + " 次提示"]
    ]
      .map(function (item) {
        return (
          '<div class="stat-box"><span class="stat-box-label">' +
          esc(item[0]) +
          '</span><span class="stat-box-value">' +
          esc(item[1]) +
          '</span><span class="stat-box-note">' +
          esc(item[2]) +
          "</span></div>"
        );
      })
      .join("");

    document.getElementById("main").innerHTML =
      '<article class="completion-card" data-step-id="complete" aria-labelledby="stepTitle">' +
      '<div class="completion-head"><div class="medal" aria-hidden="true"><span class="medal-check">✓</span></div>' +
      '<p class="kicker">' +
      esc(completion.eyebrow) +
      '</p><h1 class="completion-title" id="stepTitle" tabindex="-1">' +
      esc(completion.title) +
      '</h1><p class="completion-body">' +
      esc(completion.body) +
      '</p><span class="badge-pill"><span class="badge-check" aria-hidden="true">✓</span>徽章解锁 · ' +
      esc(data.badge.title) +
      "</span></div>" +
      '<div class="stats-grid">' +
      stats +
      "</div>" +
      '<div class="completion-actions"><button class="btn btn-primary" type="button" data-action="replay" id="primaryAction">' +
      esc(completion.replay) +
      ' <span aria-hidden="true">↻</span></button></div>' +
      '<details class="proof"><summary><span class="proof-plus" aria-hidden="true">+</span><span><strong>看形式证明</strong><small>可选 · 后台证据，不是主界面</small></span></summary>' +
      '<div class="proof-body"><p class="proof-summary">' +
      esc(proof.summary) +
      '</p><p class="proof-lead">' +
      esc(proof.theorem_lead) +
      '</p><ol class="theorem-list">' +
      theorems +
      '</ol><p class="proof-evidence">' +
      esc(proof.evidence) +
      '</p><p class="proof-note">' +
      esc(proof.note) +
      "</p></div></details>" +
      '<p class="completion-next">' +
      esc(completion.next) +
      "</p></article>";
  }

  function updateHeader() {
    var progress = state.completed
      ? 1
      : Math.min(1, (state.index + (state.solved ? 1 : 0.22)) / TOTAL);
    var percent = Math.max(5, Math.min(100, Math.round(progress * 100)));
    var fill = document.getElementById("progressFill");
    var track = document.getElementById("progressTrack");
    if (fill) {
      fill.style.width = percent + "%";
    }
    if (track) {
      track.setAttribute("aria-valuenow", String(percent));
    }
    var stepLabel = document.getElementById("stepLabel");
    if (stepLabel) {
      stepLabel.textContent = state.completed
        ? TOTAL + " / " + TOTAL + " · 完成"
        : "第 " + (state.index + 1) + " / " + TOTAL + " 步";
    }
    var xpValue = document.getElementById("xpValue");
    if (xpValue) {
      xpValue.textContent = String(state.xp);
    }
    var streakValue = document.getElementById("streakValue");
    if (streakValue) {
      streakValue.textContent = String(state.streak);
    }
    if (state.lastXp !== state.xp) {
      popStat(document.getElementById("xpStat"));
      state.lastXp = state.xp;
    }
    if (state.lastStreak !== state.streak) {
      popStat(document.getElementById("streakStat"));
      state.lastStreak = state.streak;
    }
  }

  function render() {
    if (state.completed) {
      renderCompletion();
    } else {
      renderStep();
    }
    renderJourney();
    updateHeader();
  }

  function focusStep() {
    window.requestAnimationFrame(function () {
      var title = document.getElementById("stepTitle");
      if (title) {
        title.focus({ preventScroll: true });
      }
    });
  }

  function scrollToTop() {
    window.scrollTo({ top: 0, behavior: reduceMotion ? "auto" : "smooth" });
  }

  function revealFeedback() {
    window.requestAnimationFrame(function () {
      var target =
        document.getElementById("primaryAction") ||
        document.getElementById("feedback");
      if (target) {
        target.scrollIntoView({
          block: "nearest",
          behavior: reduceMotion ? "auto" : "smooth"
        });
      }
    });
  }

  function resetStepState() {
    state.hintTier = 0;
    state.wrongAttempts = 0;
    state.attempts = 0;
    state.solved = false;
    state.feedback = null;
    state.wrongChoice = null;
    state.markerValue = null;
  }

  function awardStepXp(step, bonus) {
    var key = String(state.index);
    if (state.awarded[key]) {
      return 0;
    }
    var earned = step.xp + (bonus || 0);
    state.awarded[key] = true;
    state.lastXp = state.xp;
    state.xp += earned;
    return earned;
  }

  function goNext() {
    if (state.index >= TOTAL - 1) {
      finishLesson();
      return;
    }
    state.index += 1;
    resetStepState();
    render();
    focusStep();
    scrollToTop();
  }

  function startLesson() {
    if (state.completed) {
      return;
    }
    if (state.startedAt === null) {
      state.startedAt = Date.now();
    }
    awardStepXp(stepAt(0), 0);
    goNext();
  }

  function continueLesson() {
    if (state.completed) {
      return;
    }
    var step = stepAt(state.index);
    if (step.type === "choice" && !state.solved) {
      return;
    }
    if (step.type === "insight") {
      awardStepXp(step, 0);
    }
    if (state.index >= TOTAL - 1) {
      finishLesson();
    } else {
      goNext();
    }
  }

  function answerChoice(choiceId) {
    if (state.solved || state.completed) {
      return;
    }
    var step = stepAt(state.index);
    if (step.type !== "choice") {
      return;
    }
    var choice = step.choices.filter(function (item) {
      return item.id === choiceId;
    })[0];
    if (!choice) {
      return;
    }
    state.attempts += 1;

    if (choiceId === step.answer) {
      var firstTry = state.attempts === 1;
      var bonus = firstTry ? step.first_try_bonus : 0;
      state.solved = true;
      state.wrongChoice = null;
      var earned = awardStepXp(step, bonus);
      state.streak += 1;
      state.maxStreak = Math.max(state.maxStreak, state.streak);
      state.firstTryCount += firstTry ? 1 : 0;
      state.feedback = {
        kind: "correct",
        title: step.feedback.correct.title,
        body: step.feedback.correct.body,
        xpEarned: earned,
        bonus: bonus,
        firstTry: firstTry
      };
      state.wrongAttempts = 0;
      render();
      announce("回答正确。" + step.feedback.correct.body);
      window.requestAnimationFrame(function () {
        var primary = document.getElementById("primaryAction");
        if (primary) {
          primary.focus({ preventScroll: true });
        }
        revealFeedback();
      });
      return;
    }

    state.wrongAttempts += 1;
    state.streak = 0;
    state.wrongChoice = choiceId;
    state.feedback = {
      kind: "wrong",
      title: "再想一想",
      body: step.feedback.wrong[choiceId]
    };
    render();
    announce("回答不对。" + step.feedback.wrong[choiceId]);
    window.requestAnimationFrame(function () {
      var button = document.querySelector('[data-choice-id="' + choiceId + '"]');
      if (button) {
        button.focus({ preventScroll: true });
      }
      revealFeedback();
    });
  }

  function showHint() {
    if (state.completed || state.solved) {
      return;
    }
    var step = stepAt(state.index);
    if (step.type !== "choice" || state.hintTier >= step.hints.length) {
      return;
    }
    var hint = step.hints[state.hintTier];
    state.hintTier += 1;
    state.hintsUsed += 1;
    state.feedback = {
      kind: "hint",
      title: "提示 " + state.hintTier + "/" + step.hints.length + " · " + hint.title,
      body: hint.body
    };
    render();
    announce("提示 " + state.hintTier + "。" + hint.body);
    window.requestAnimationFrame(function () {
      var feedback = document.getElementById("feedback");
      if (feedback) {
        feedback.focus({ preventScroll: true });
      }
      revealFeedback();
    });
  }

  function selectMarker(button) {
    var note = button.getAttribute("data-note") || "";
    var value = Number(button.getAttribute("data-marker"));
    state.markerValue = value;
    $$(".nl-marker[data-marker]").forEach(function (marker) {
      marker.classList.remove("is-active");
      marker.setAttribute("aria-pressed", "false");
    });
    button.classList.add("is-active");
    button.setAttribute("aria-pressed", "true");
    var caption = document.getElementById("markerCaption");
    if (caption) {
      caption.textContent = note;
    }
    announce(note);
  }

  function closeHelp(restoreFocus) {
    if (!state.help && !state.helpTimer) {
      return;
    }
    var trigger = state.help ? state.help.trigger : null;
    window.clearInterval(state.helpTimer);
    state.helpTimer = null;
    state.help = null;
    state.helpEnds = 0;
    var root = document.getElementById("modalRoot");
    if (root) {
      root.innerHTML = "";
    }
    document.body.classList.remove("is-locked");
    if (restoreFocus !== false && trigger && trigger.isConnected) {
      trigger.focus({ preventScroll: true });
    }
  }

  function tickHelp() {
    if (!state.help) {
      return;
    }
    var total = state.help.symbol.seconds * 1000;
    var left = Math.max(0, state.helpEnds - performance.now());
    var ratio = total > 0 ? left / total : 0;
    var bar = document.getElementById("helpTimerBar");
    if (bar) {
      bar.style.transform = "scaleX(" + ratio + ")";
    }
    var count = document.getElementById("helpCount");
    if (count) {
      count.textContent = Math.ceil(left / 1000) + "s";
    }
    if (left <= 0) {
      closeHelp();
      announce("20 秒到，符号卡已收起。");
      showToast("20 秒到，符号卡已收起");
    }
  }

  function openHelp(symbolId, trigger) {
    var symbol = symbolAt(symbolId);
    if (!symbol) {
      return;
    }
    closeHelp(false);
    state.help = { symbolId: symbolId, symbol: symbol, trigger: trigger };
    state.helpEnds = performance.now() + symbol.seconds * 1000;
    var root = document.getElementById("modalRoot");
    root.innerHTML =
      '<div class="modal-backdrop" id="helpBackdrop"><article class="help-sheet" id="helpSheet" role="dialog" aria-modal="true" aria-labelledby="helpTitle" aria-describedby="helpPlain">' +
      '<div class="help-timer" aria-hidden="true"><span class="help-timer-bar" id="helpTimerBar"></span></div>' +
      '<div class="help-top"><span class="help-kicker">20 秒符号卡</span><span class="help-count" id="helpCount" aria-hidden="true">' +
      symbol.seconds +
      's</span></div><h2 id="helpTitle">' +
      esc(symbol.term) +
      '</h2><p class="help-reading">' +
      esc(symbol.reading) +
      '</p><p class="help-plain" id="helpPlain">' +
      esc(symbol.plain) +
      '</p><div class="help-example">' +
      esc(symbol.example) +
      '</div><p class="help-foot">' +
      esc(symbol.help) +
      '</p><button class="btn btn-primary" type="button" data-action="close-help" id="helpClose">知道了</button></article></div>';
    document.body.classList.add("is-locked");
    window.requestAnimationFrame(function () {
      var close = document.getElementById("helpClose");
      if (close) {
        close.focus();
      }
    });
    window.clearInterval(state.helpTimer);
    state.helpTimer = window.setInterval(tickHelp, 100);
    tickHelp();
  }

  function launchConfetti(count) {
    if (reduceMotion) {
      return;
    }
    var layer = document.getElementById("confetti");
    if (!layer) {
      return;
    }
    layer.innerHTML = "";
    var colors = ["#58cc02", "#1cb0f6", "#ff9600", "#8b5cf6", "#ff4b4b", "#ffd166"];
    var fragment = document.createDocumentFragment();
    for (var i = 0; i < count; i += 1) {
      var piece = document.createElement("span");
      piece.className = "confetti-piece";
      piece.style.left = Math.random() * 100 + "%";
      piece.style.background = colors[i % colors.length];
      piece.style.width = 6 + Math.random() * 6 + "px";
      piece.style.height = 10 + Math.random() * 10 + "px";
      piece.style.setProperty("--drift", Math.round(Math.random() * 260 - 130) + "px");
      piece.style.setProperty("--spin", Math.round(Math.random() * 900 - 450) + "deg");
      piece.style.setProperty("--delay", (Math.random() * 0.45).toFixed(2) + "s");
      piece.style.setProperty("--dur", (2.1 + Math.random() * 1.6).toFixed(2) + "s");
      fragment.appendChild(piece);
    }
    layer.appendChild(fragment);
    window.setTimeout(function () {
      if (layer) {
        layer.innerHTML = "";
      }
    }, 4400);
  }

  function finishLesson() {
    if (state.completed) {
      return;
    }
    if (state.startedAt === null) {
      state.startedAt = Date.now();
    }
    state.xp += data.completion.xp;
    state.completed = true;
    state.solved = true;
    state.feedback = null;
    render();
    focusStep();
    scrollToTop();
    launchConfetti(40);
    announce(
      "本课完成。徽章已解锁：" +
        data.badge.title +
        "。本轮 " +
        state.xp +
        " XP。"
    );
    var elapsedSeconds = Math.max(
      1,
      Math.round((Date.now() - state.startedAt) / 1000)
    );
    window.dispatchEvent(
      new CustomEvent("soko:lesson-complete", {
        detail: {
          lessonId: data.id,
          badge: data.badge.title,
          xp: state.xp,
          maxXp: MAX_XP,
          firstTryCount: state.firstTryCount,
          gradedSteps: GRADED,
          maxStreak: state.maxStreak,
          hintsUsed: state.hintsUsed,
          elapsedSeconds: elapsedSeconds
        }
      })
    );
  }

  function replayLesson() {
    closeHelp(false);
    state.index = 0;
    state.xp = 0;
    state.streak = 0;
    state.maxStreak = 0;
    state.hintsUsed = 0;
    state.firstTryCount = 0;
    state.startedAt = null;
    state.lastXp = 0;
    state.lastStreak = 0;
    state.completed = false;
    state.awarded = {};
    resetStepState();
    var confetti = document.getElementById("confetti");
    if (confetti) {
      confetti.innerHTML = "";
    }
    render();
    focusStep();
    scrollToTop();
    announce("重新开始。");
  }

  function onClick(event) {
    if (event.target && event.target.id === "helpBackdrop") {
      closeHelp();
      return;
    }

    var actionNode = event.target.closest("[data-action]");
    if (actionNode) {
      var action = actionNode.getAttribute("data-action");
      if (action === "start") {
        startLesson();
      } else if (action === "continue") {
        continueLesson();
      } else if (action === "hint") {
        showHint();
      } else if (action === "replay") {
        replayLesson();
      } else if (action === "close-help") {
        closeHelp();
      }
      return;
    }

    var helpNode = event.target.closest("[data-symbol]");
    if (helpNode) {
      event.preventDefault();
      openHelp(helpNode.getAttribute("data-symbol"), helpNode);
      return;
    }

    var choiceNode = event.target.closest("[data-choice-id]");
    if (choiceNode) {
      answerChoice(choiceNode.getAttribute("data-choice-id"));
      return;
    }

    var markerNode = event.target.closest("[data-marker]");
    if (markerNode) {
      selectMarker(markerNode);
    }
  }

  function onKeyDown(event) {
    if (state.help) {
      if (event.key === "Escape") {
        event.preventDefault();
        closeHelp();
      } else if (event.key === "Tab") {
        event.preventDefault();
        var close = document.getElementById("helpClose");
        if (close) {
          close.focus();
        }
      }
      return;
    }

    if (
      event.metaKey ||
      event.ctrlKey ||
      event.altKey ||
      event.defaultPrevented ||
      event.repeat ||
      state.completed
    ) {
      return;
    }

    var step = stepAt(state.index);
    if (!step || step.type !== "choice" || state.solved) {
      return;
    }
    if (/^[1-9]$/.test(event.key)) {
      var index = Number(event.key) - 1;
      var choice = step.choices[index];
      if (choice) {
        event.preventDefault();
        answerChoice(choice.id);
      }
    }
  }

  function renderProgressTicks() {
    var track = document.querySelector(".progress-ticks");
    if (!track) {
      return;
    }
    var ticks = [];
    for (var index = 1; index < TOTAL; index += 1) {
      ticks.push(
        '<i style="left:' + ((index / TOTAL) * 100).toFixed(3) + '%"></i>'
      );
    }
    track.innerHTML = ticks.join("");
  }

  function init() {
    var brandCourse = document.getElementById("brandCourse");
    if (brandCourse) {
      brandCourse.textContent = data.course_title + " · Unit ⑧";
    }
    var progressUnit = document.getElementById("progressUnit");
    if (progressUnit) {
      progressUnit.textContent =
        "Unit ⑧ · 约 " + data.estimated_minutes + " 分钟";
    }
    renderProgressTicks();
    render();
    window.SokoLesson = Object.freeze({
      version: data.version,
      lessonId: data.id,
      steps: TOTAL,
      getState: function () {
        return {
          lessonId: data.id,
          stepIndex: state.index,
          stepId: state.completed ? null : stepAt(state.index).id,
          completed: state.completed,
          xp: state.xp,
          maxXp: MAX_XP,
          streak: state.streak,
          maxStreak: state.maxStreak,
          firstTryCount: state.firstTryCount,
          hintsUsed: state.hintsUsed,
          attempts: state.attempts,
          wrongAttempts: state.wrongAttempts,
          hintTier: state.hintTier,
          badge: state.completed ? data.badge.title : null
        };
      },
      reset: replayLesson
    });
    window.dispatchEvent(
      new CustomEvent("soko:lesson-ready", {
        detail: { lessonId: data.id, steps: TOTAL }
      })
    );
  }

  document.addEventListener("click", onClick);
  document.addEventListener("keydown", onKeyDown);
  init();
})();

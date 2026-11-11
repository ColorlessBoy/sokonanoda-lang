/* scripts/lesson_complete/math.js — the lesson's only math source.
 *
 * For aₙ = 1/n → 0:
 *   |aₙ| < ε  ⇔  1/n < ε  ⇔  n > 1/ε.
 *
 * N is the FIRST integer strictly greater than 1/ε:
 *   N = ⌊1/ε⌋ + 1
 * even when 1/ε is an integer — the boundary term a_N = ε is not inside
 * the band, because the definition uses strict "<".
 *
 * The file is a tiny UMD module: the browser gets window.LessonMath, Node
 * gets module.exports. The page, build and checks all import this one file.
 */
(function (root, factory) {
  "use strict";
  var api = factory();
  if (typeof module === "object" && module.exports) module.exports = api;
  if (root) root.LessonMath = api;
})(typeof globalThis !== "undefined" ? globalThis : this, function () {
  "use strict";

  var EPS_MIN = 0.001;
  var EPS_MAX = 0.25;
  var EPS_SCALE = 1000;
  var TERMS = 20;
  var VISIBLE_FIRST = 4; // the zoomed band scene draws n = 4 … 20
  var VISIBLE_LAST = 20;

  function clamp(value, lo, hi) {
    return Math.min(hi, Math.max(lo, value));
  }

  function isFinitePositive(value) {
    return typeof value === "number" && isFinite(value) && value > 0;
  }

  function quantizeEpsilon(value) {
    if (typeof value !== "number" || !isFinite(value)) return EPS_MIN;
    return Math.round(clamp(value, EPS_MIN, EPS_MAX) * EPS_SCALE) / EPS_SCALE;
  }

  function computeN(epsilon) {
    if (!isFinitePositive(epsilon)) {
      throw new RangeError("epsilon must be a positive finite number");
    }
    var milli = Math.round(epsilon * EPS_SCALE);
    if (Math.abs(epsilon * EPS_SCALE - milli) <= 1e-7) {
      var k = Math.max(1, Math.round(milli));
      return Math.floor(EPS_SCALE / k) + 1;
    }
    var inv = 1 / epsilon;
    var nearest = Math.round(inv);
    if (Math.abs(inv - nearest) <= 1e-9) return nearest + 1;
    return Math.floor(inv) + 1;
  }

  /** Band membership for a term value: 0 < value < ε. */
  function isInsideValue(value, epsilon) {
    return value > 0 && value < epsilon;
  }

  /** Band membership for a term index: n ≥ 1 and 1/n < ε. */
  function isInside(n, epsilon) {
    return n >= 1 && isInsideValue(1 / n, epsilon);
  }

  /** How many of the drawn terms (n = 4 … 20) are inside the band. */
  function insideVisibleCount(epsilon) {
    var count = 0;
    for (var n = VISIBLE_FIRST; n <= VISIBLE_LAST; n += 1) {
      if (isInside(n, epsilon)) count += 1;
    }
    return count;
  }

  /** Strict boundary checks: 1/N < ε and 1/(N−1) ≥ ε. */
  function strictChecks(epsilon) {
    var N = computeN(epsilon);
    var first = 1 / N;
    var previous = N > 1 ? 1 / (N - 1) : Infinity;
    return {
      N: N,
      first: first,
      previous: previous,
      firstOk: first < epsilon,
      previousOk: N === 1 || previous >= epsilon,
      holds: first < epsilon && (N === 1 || previous >= epsilon),
    };
  }

  /** Returns the integer k when 1/ε = k exactly; otherwise null. */
  function exactReciprocal(epsilon) {
    var inv = 1 / epsilon;
    var nearest = Math.round(inv);
    return Math.abs(inv - nearest) <= 1e-9 ? nearest : null;
  }

  function fmtEpsilon(epsilon) {
    return Number(epsilon).toFixed(3);
  }

  /** Keeps the strict-boundary numbers readable near 0. */
  function fmtTight(value) {
    if (!isFinite(value)) return "∞";
    if (value !== 0 && Math.abs(value) < 0.01) return Number(value).toFixed(6);
    return Number(value).toFixed(4);
  }

  function fmtTerm(n) {
    var value = 1 / n;
    return Number.isInteger(value) ? String(value) : value.toFixed(4);
  }

  return {
    EPS_MIN: EPS_MIN,
    EPS_MAX: EPS_MAX,
    EPS_SCALE: EPS_SCALE,
    TERMS: TERMS,
    VISIBLE_FIRST: VISIBLE_FIRST,
    VISIBLE_LAST: VISIBLE_LAST,
    clamp: clamp,
    quantizeEpsilon: quantizeEpsilon,
    computeN: computeN,
    isInside: isInside,
    isInsideValue: isInsideValue,
    insideVisibleCount: insideVisibleCount,
    strictChecks: strictChecks,
    exactReciprocal: exactReciprocal,
    fmtEpsilon: fmtEpsilon,
    fmtTight: fmtTight,
    fmtTerm: fmtTerm,
  };
});

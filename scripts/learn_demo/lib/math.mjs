// scripts/learn_demo/lib/math.mjs — the single source of truth for the demo's math.
//
// ε–N for aₙ = 1/n → 0:
//   |aₙ| < ε  ⇔  1/n < ε  ⇔  n > 1/ε.
// N is the FIRST integer strictly greater than 1/ε, hence N = ⌊1/ε⌋ + 1 even
// when 1/ε is an integer (the boundary term a_N = ε is NOT inside the band).
//
// The slider stores ε in integer thousandths, so the core path uses exact
// integer arithmetic: N = ⌊1000/k⌋ + 1 for ε = k/1000.  The general branch is
// kept for programmatic/non-milli values and still honours strictness.

export const TERMS = 20;
export const EPS_MIN = 0.001;
export const EPS_MAX = 0.25;
export const EPS_SCALE = 1000;

export function clamp(value, lo, hi) {
  return Math.min(hi, Math.max(lo, value));
}

export function quantizeEpsilon(value) {
  if (!Number.isFinite(value)) return EPS_MIN;
  return Math.round(clamp(value, EPS_MIN, EPS_MAX) * EPS_SCALE) / EPS_SCALE;
}

export function computeN(epsilon) {
  if (!(epsilon > 0) || !Number.isFinite(epsilon)) {
    throw new RangeError("epsilon must be a positive finite number");
  }
  const milli = Math.round(epsilon * EPS_SCALE);
  if (Math.abs(epsilon * EPS_SCALE - milli) <= 1e-7) {
    const k = Math.max(1, Math.round(milli));
    // k/1000 is exact enough; the floor is over exact integers.
    return Math.floor(EPS_SCALE / k) + 1;
  }
  const inv = 1 / epsilon;
  const nearest = Math.round(inv);
  if (Math.abs(inv - nearest) <= 1e-9) return nearest + 1;
  return Math.floor(inv) + 1;
}

export function isInside(n, epsilon) {
  return n >= 1 && n <= TERMS && 1 / n < epsilon;
}

export function firstInside(epsilon) {
  const N = computeN(epsilon);
  return N <= TERMS ? N : null;
}

export function insideCount(N) {
  return Math.max(0, TERMS - N + 1);
}

export function termsFor(epsilon) {
  const N = computeN(epsilon);
  return Array.from({ length: TERMS }, (_, index) => {
    const n = index + 1;
    return { n, value: 1 / n, inside: isInside(n, epsilon), inTail: n >= N };
  });
}

/** Strict boundary invariant: 1/N < ε and (N = 1 or 1/(N−1) ≥ ε). */
export function boundaryHolds(epsilon, tolerance = 1e-12) {
  const N = computeN(epsilon);
  const first = 1 / N;
  const previous = N > 1 ? 1 / (N - 1) : Infinity;
  return {
    N,
    firstInside: first,
    previousOutside: previous,
    holds: first < epsilon + tolerance && (N === 1 || previous >= epsilon - tolerance),
  };
}

export function fmtEpsilon(epsilon, digits = 3) {
  return Number(epsilon).toFixed(digits);
}

/** Keep 1/N readable when N is large: 0.000999… needs more than 4 decimals. */
export function fmtTight(value) {
  if (!Number.isFinite(value)) return "∞";
  if (value !== 0 && Math.abs(value) < 0.01) return Number(value).toFixed(6);
  return Number(value).toFixed(4);
}

export function fmtTerm(n) {
  const value = 1 / n;
  if (Number.isInteger(value)) return String(value);
  return value.toFixed(4);
}

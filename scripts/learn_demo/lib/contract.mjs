// scripts/learn_demo/lib/contract.mjs — the demo's content contract.
//
// Build and check both call validateContent().  The contract is deliberately
// stricter than the UI: it recomputes every answer key from the ε–N formula
// instead of trusting the JSON, and it refuses the two failure modes the brief
// names (missing character / missing question type).

import {
  TERMS,
  EPS_MIN,
  EPS_MAX,
  computeN,
  boundaryHolds,
} from "./math.mjs";

export const REQUIRED_TYPES = [
  "multiple-choice",
  "true-false",
  "click-dot",
  "order",
  "slider-choice",
];

export const CANONICAL_BLOCKS = [
  { id: "forall-eps", text: "∀ε > 0" },
  { id: "exists-N", text: "∃N" },
  { id: "forall-n", text: "∀n ≥ N" },
  { id: "dist", text: "|aₙ − L| < ε" },
];

const REQUIRED_BASE_CONTEXTS = ["intro", "scene", "complete"];
const MAX_LINE = 90;

function isObject(value) {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function nonEmpty(value) {
  return typeof value === "string" && value.trim().length > 0;
}

function close(a, b) {
  return typeof a === "number" && Number.isFinite(a) && Math.abs(a - b) < 1e-9;
}

function allFiniteNumbers(list) {
  return Array.isArray(list) && list.length > 0 && list.every((value) => Number.isFinite(value));
}

export function requiredContexts(data) {
  const questions = Array.isArray(data?.questions) ? data.questions : [];
  return [...REQUIRED_BASE_CONTEXTS.slice(0, 2), ...questions.map((q) => q?.id), "complete"];
}

/**
 * @returns {{ok: boolean, errors: string[]}}
 */
export function validateContent(data) {
  const errors = [];
  const fail = (path, message) => errors.push(`${path}: ${message}`);

  if (!isObject(data)) return { ok: false, errors: ["content: not a JSON object"] };
  if (data.schema !== "soko.learn_demo/1") fail("schema", 'must be "soko.learn_demo/1"');
  if (!nonEmpty(data.id)) fail("id", "required");
  if (!nonEmpty(data.title)) fail("title", "required");
  if (!nonEmpty(data.concept)) fail("concept", "required");
  if (!isObject(data.hero) || !nonEmpty(data.hero.title) || !nonEmpty(data.hero.dek)) {
    fail("hero", "title and dek required");
  }
  if (!isObject(data.badge) || data.badge.title !== "你理解了 ε–N") {
    fail("badge.title", 'must be exactly "你理解了 ε–N"');
  }

  // ── scene ──────────────────────────────────────────────────
  const scene = data.scene;
  if (!isObject(scene)) {
    fail("scene", "required");
  } else {
    if (!close(scene.defaultEpsilon, 0.22)) fail("scene.defaultEpsilon", "demo default is 0.22");
    if (!close(scene.epsilonMin, EPS_MIN)) fail("scene.epsilonMin", `must be ${EPS_MIN}`);
    if (!close(scene.epsilonMax, EPS_MAX)) fail("scene.epsilonMax", `must be ${EPS_MAX}`);
    if (!allFiniteNumbers(scene.presets)) {
      fail("scene.presets", "must be a non-empty numeric array");
    } else {
      for (const [index, value] of scene.presets.entries()) {
        if (value < EPS_MIN - 1e-12 || value > EPS_MAX + 1e-12) {
          fail(`scene.presets[${index}]`, `outside [${EPS_MIN}, ${EPS_MAX}]`);
        }
      }
      for (const required of [0.25, 0.22, 0.1]) {
        if (!scene.presets.some((value) => close(value, required))) {
          fail("scene.presets", `must include the preset ${required}`);
        }
      }
    }
    if (!allFiniteNumbers(scene.steps)) fail("scene.steps", "must be a non-empty numeric array");
    if (!(scene.playFrom > scene.playTo && scene.playTo > 0)) {
      fail("scene.play", "playFrom must be greater than playTo, and playTo > 0");
    }
    if (!(Number.isFinite(scene.playDurationMs) && scene.playDurationMs >= 500)) {
      fail("scene.playDurationMs", "must be >= 500");
    }
    if (scene.terms !== TERMS) fail("scene.terms", `demo draws exactly ${TERMS} terms`);
  }

  // ── characters (at least two, same family, distinct) ───────
  const characters = Array.isArray(data.characters) ? data.characters : [];
  if (characters.length < 2) fail("characters", "at least 2 characters are required");
  const characterIds = new Set();
  const characterNames = new Set();
  const avatarKeys = new Set();
  for (const [index, character] of characters.entries()) {
    const base = `characters[${index}]`;
    if (!isObject(character)) {
      fail(base, "must be an object");
      continue;
    }
    if (!nonEmpty(character.id)) fail(`${base}.id`, "required");
    else if (characterIds.has(character.id)) fail(`${base}.id`, `duplicate id ${character.id}`);
    characterIds.add(character.id);
    if (!nonEmpty(character.name)) fail(`${base}.name`, "required");
    else if (characterNames.has(character.name)) fail(`${base}.name`, `duplicate name ${character.name}`);
    characterNames.add(character.name);
    if (!nonEmpty(character.role)) fail(`${base}.role`, "required");
    if (!nonEmpty(character.tagline)) fail(`${base}.tagline`, "required");
    if (!nonEmpty(character.avatar)) fail(`${base}.avatar`, "required");
    else if (avatarKeys.has(character.avatar)) fail(`${base}.avatar`, "characters need distinct visual ids");
    avatarKeys.add(character.avatar);
    if (!isObject(character.lines)) {
      fail(`${base}.lines`, "required");
    } else {
      for (const context of [...REQUIRED_BASE_CONTEXTS.slice(0, 2), ...(Array.isArray(data.questions) ? data.questions.map((q) => q?.id) : []), "complete"]) {
        const line = character.lines[context];
        if (!nonEmpty(line)) {
          fail(`${base}.lines.${context}`, "required (every avatar line serves a math action)");
        } else if (line.length > MAX_LINE) {
          fail(`${base}.lines.${context}`, `one line only (<= ${MAX_LINE} chars)`);
        } else if (/[!！]/.test(line)) {
          fail(`${base}.lines.${context}`, "no hype/exclamation in an avatar line");
        }
      }
    }
  }

  // ── questions ─────────────────────────────────────────────
  const questions = Array.isArray(data.questions) ? data.questions : [];
  if (questions.length !== REQUIRED_TYPES.length) {
    fail("questions", `exactly ${REQUIRED_TYPES.length} questions required (got ${questions.length})`);
  }
  const qids = new Set();
  for (const [index, question] of questions.entries()) {
    const base = `questions[${index}]`;
    if (!isObject(question)) {
      fail(base, "must be an object");
      continue;
    }
    if (!nonEmpty(question.id)) fail(`${base}.id`, "required");
    else if (qids.has(question.id)) fail(`${base}.id`, `duplicate id ${question.id}`);
    qids.add(question.id);
    if (!REQUIRED_TYPES.includes(question.type)) {
      fail(`${base}.type`, `unknown type ${question.type}`);
    } else if (question.type !== REQUIRED_TYPES[index]) {
      fail(`${base}.type`, `expected the ${REQUIRED_TYPES[index]} slot, got ${question.type}`);
    }
    for (const field of ["title", "prompt", "explanation"]) {
      if (!nonEmpty(question[field])) fail(`${base}.${field}`, "required");
    }
    if (!Array.isArray(question.hints) || question.hints.length !== 3) {
      fail(`${base}.hints`, "exactly 3 hint tiers required");
    } else {
      for (const [hintIndex, hint] of question.hints.entries()) {
        if (!nonEmpty(hint)) fail(`${base}.hints[${hintIndex}]`, "required");
      }
    }
    if (!isObject(question.successLines)) {
      fail(`${base}.successLines`, "required");
    } else {
      for (const character of characters) {
        if (!nonEmpty(question.successLines[character.id])) {
          fail(`${base}.successLines.${character.id}`, "required");
        }
      }
    }
  }

  const byId = new Map(questions.map((q) => [q?.id, q]));
  validateQ1(byId.get("q1"), fail);
  validateQ2(byId.get("q2"), fail);
  validateQ3(byId.get("q3"), fail);
  validateQ4(byId.get("q4"), fail);
  validateQ5(byId.get("q5"), fail);

  // ── scoring + formal bridge ────────────────────────────────
  const scoring = data.scoring;
  if (!isObject(scoring)) {
    fail("scoring", "required");
  } else {
    if (scoring.perQuestion !== 20) fail("scoring.perQuestion", "demo is 20 XP per question");
    if (scoring.hintPenalty !== 5) fail("scoring.hintPenalty", "demo subtracts 5 per hint tier");
    if (scoring.retryPenalty !== 5) fail("scoring.retryPenalty", "demo subtracts 5 after a wrong attempt");
    if (scoring.minAward !== 5) fail("scoring.minAward", "demo floor is 5");
  }

  const formal = data.formal;
  if (!isObject(formal)) {
    fail("formal", "required");
  } else {
    for (const field of ["definition", "definitionNote", "plain", "whyN", "note"]) {
      if (!nonEmpty(formal[field])) fail(`formal.${field}`, "required");
    }
    if (!Array.isArray(formal.rows) || formal.rows.length < 4) {
      fail("formal.rows", "at least 4 concept ↔ formal rows");
    }
    if (!Array.isArray(formal.proof) || formal.proof.length < 3) {
      fail("formal.proof", "at least 3 proof steps");
    }
    const definition = formal.definition || "";
    for (const token of ["∀", "∃", "∀ n ≥ N", "< ε"]) {
      if (!definition.includes(token)) fail("formal.definition", `must contain ${token}`);
    }
  }

  return { ok: errors.length === 0, errors };
}

function validateQ1(question, fail) {
  const base = "questions[q1]";
  if (!question) return fail(base, "missing");
  if (question.type !== "multiple-choice") fail(`${base}.type`, "must be multiple-choice");
  if (!close(question.epsilon, 0.22)) fail(`${base}.epsilon`, "must be 0.22");
  const choices = Array.isArray(question.choices) ? question.choices : [];
  const values = choices.map((choice) => choice?.value);
  if (choices.length !== 3) fail(`${base}.choices`, "exactly 3 choices");
  for (const expected of [4, 5, 10]) {
    if (!values.some((value) => value === expected)) fail(`${base}.choices`, `must include ${expected}`);
  }
  const answer = choices.find((choice) => choice?.id === question.answer);
  if (!answer) fail(`${base}.answer`, "must name an existing choice");
  const expected = computeN(question.epsilon);
  if (answer && answer.value !== expected) {
    fail(`${base}.answer`, `should be N = ${expected} for ε = ${question.epsilon}`);
  }
  if (answer && answer.value !== 5) fail(`${base}.answer`, "expected 5");
}

function validateQ2(question, fail) {
  const base = "questions[q2]";
  if (!question) return fail(base, "missing");
  if (question.type !== "true-false") fail(`${base}.type`, "must be true-false");
  if (!close(question.epsilon, 0.22)) fail(`${base}.epsilon`, "must be 0.22");
  if (!nonEmpty(question.statement)) fail(`${base}.statement`, "required");
  const expected = computeN(question.epsilon);
  if (expected !== 5) fail(`${base}.epsilon`, "demo expects N = 5 for ε = 0.22");
  // The statement "N = 4 works" is false exactly because 1/4 >= 0.22.
  if (!(1 / 4 >= question.epsilon)) fail(`${base}.epsilon`, "1/4 must be >= ε for the false statement");
  const answer = (question.choices || []).find((choice) => choice?.id === question.answer);
  if (!answer) fail(`${base}.answer`, "must name an existing choice");
  if (answer && answer.value !== false) fail(`${base}.answer`, "must be false");
  if (!(question.explanation || "").includes("1/4")) fail(`${base}.explanation`, "must show 1/4");
  if (!(question.explanation || "").includes("0.25")) fail(`${base}.explanation`, "must show 0.25");
  if (!(question.explanation || "").includes("0.22")) fail(`${base}.explanation`, "must show 0.22");
}

function validateQ3(question, fail) {
  const base = "questions[q3]";
  if (!question) return fail(base, "missing");
  if (question.type !== "click-dot") fail(`${base}.type`, "must be click-dot");
  if (!close(question.epsilon, 0.22)) fail(`${base}.epsilon`, "must be 0.22");
  if (question.dotCount !== TERMS) fail(`${base}.dotCount`, `must be ${TERMS}`);
  const expected = computeN(question.epsilon);
  if (question.answer !== expected) fail(`${base}.answer`, `first inside dot is n = ${expected}`);
  if (!(1 / question.answer < question.epsilon)) fail(`${base}.answer`, "answer must be strictly inside");
  if (!(question.answer === 1 || 1 / (question.answer - 1) >= question.epsilon)) {
    fail(`${base}.answer`, "the previous term must be outside");
  }
  if (!boundaryHolds(question.epsilon).holds) fail(`${base}.epsilon`, "boundary invariant fails");
}

function validateQ4(question, fail) {
  const base = "questions[q4]";
  if (!question) return fail(base, "missing");
  if (question.type !== "order") fail(`${base}.type`, "must be order");
  const blocks = Array.isArray(question.blocks) ? question.blocks : [];
  if (blocks.length !== CANONICAL_BLOCKS.length) fail(`${base}.blocks`, "exactly 4 blocks");
  const byId = new Map(blocks.map((block) => [block?.id, block]));
  const expectedIds = CANONICAL_BLOCKS.map((block) => block.id);
  if (JSON.stringify(question.answer) !== JSON.stringify(expectedIds)) {
    fail(`${base}.answer`, `must be ${expectedIds.join(" → ")}`);
  }
  for (const [index, canonical] of CANONICAL_BLOCKS.entries()) {
    const block = byId.get(canonical.id);
    if (!block) {
      fail(`${base}.blocks`, `missing ${canonical.id}`);
      continue;
    }
    if (block.text !== canonical.text) fail(`${base}.blocks[${index}].text`, `must be "${canonical.text}"`);
    if (!nonEmpty(block.why)) fail(`${base}.blocks[${index}].why`, "required");
  }
  if (!Array.isArray(question.pool) || question.pool.length !== blocks.length) {
    fail(`${base}.pool`, "must list every block exactly once");
  } else {
    const pool = [...question.pool].sort();
    const ids = blocks.map((block) => block?.id).sort();
    if (JSON.stringify(pool) !== JSON.stringify(ids)) {
      fail(`${base}.pool`, "pool must be a permutation of the block ids");
    }
  }
}

function validateQ5(question, fail) {
  const base = "questions[q5]";
  if (!question) return fail(base, "missing");
  if (question.type !== "slider-choice") fail(`${base}.type`, "must be slider-choice");
  if (!close(question.targetEpsilon, 0.1)) fail(`${base}.targetEpsilon`, "must be 0.10");
  const choices = Array.isArray(question.choices) ? question.choices : [];
  const values = choices.map((choice) => choice?.value);
  for (const expected of [10, 11, 21]) {
    if (!values.some((value) => value === expected)) fail(`${base}.choices`, `must include ${expected}`);
  }
  const answer = choices.find((choice) => choice?.id === question.answer);
  const expected = computeN(question.targetEpsilon);
  if (expected !== 11) fail(`${base}.targetEpsilon`, "demo expects N = 11 for ε = 0.10");
  if (answer && answer.value !== expected) fail(`${base}.answer`, `should be N = ${expected}`);
  if (!(1 / 10 >= question.targetEpsilon)) fail(`${base}.targetEpsilon`, "1/10 must be >= ε");
  if (!(1 / 11 < question.targetEpsilon)) fail(`${base}.targetEpsilon`, "1/11 must be < ε");
  const { sliderMin, sliderMax, sliderStep } = question;
  if (!(sliderMin <= question.targetEpsilon && question.targetEpsilon <= sliderMax)) {
    fail(`${base}.slider`, "target ε must be inside the slider range");
  }
  if (!(sliderStep > 0)) fail(`${base}.sliderStep`, "must be > 0");
  if (Math.abs(question.targetEpsilon - sliderMin) % sliderStep > 1e-9) {
    fail(`${base}.sliderStep`, "target ε must be reachable from sliderMin by whole steps");
  }
}

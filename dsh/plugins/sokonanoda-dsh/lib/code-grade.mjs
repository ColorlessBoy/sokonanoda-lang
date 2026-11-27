// code-grade —— 代码练习的「真内核判卷」桥（C9.4）。
//
// 输入：引擎发出的代码题（含 id/course/template、数据集题由 compose.py 组装完整文件）
// 和用户在聊天里提交的围栏代码；输出：只把目标声明的**证明体**贴回可信签名之后，
// 交给 `scripts/soko grade --root <course> <file> --json`，解析
// `decl.checked` / `exercise.open` / `diagnostic`。
//
// 三条硬约束：
//   1. **绝不伪造判定**：只有目标声明 `decl.checked`、目标没有 `exercise.open`、
//      且没有任何 `diagnostic` 才算通过；exit 0 本身不算通过（模板的 sorry 也 exit 0）。
//   2. **签名不可被用户替换**：用户声明名必须与目标一致，签名始终取课程模板的；
//      用户只交证明体 ⇒ 改签名/换命题不能骗过判卷。
//   3. **失败 fail-open**：compose/soko 缺失、超时、输出不可解析都抛 `code` 错误，
//      由调用方回落到 `next()`；用户代码本身写错是内核 diagnostic，不是基础设施失败。
//
// 临时文件只落在系统临时目录，判完即删；错误与日志里不出现用户代码。

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';

const execFileAsync = promisify(execFile);

export const CODE_MAX_BYTES = 64 * 1024;
export const DEFAULT_GRADE_TIMEOUT_MS = 30_000;
export const DEFAULT_COMPOSE_TIMEOUT_MS = 10_000;

const DECL_LINE_RE = /^(?:theorem|lemma|def|example|abbrev|structure|inductive|instance)\s+/gm;
const DECL_NAME_RE = /^(?:theorem|lemma|def|example|abbrev|structure|inductive|instance)\s+([A-Za-z_][A-Za-z0-9_.']*)/m;
const TACTIC_RE = /^(?:intro|intros|exact|apply|constructor|rw|simp|omega|linarith|nlinarith|ring|norm_num|cases|induction|ext|funext|rfl|trivial|assumption|contradiction|exfalso|sorry|decide|positivity|gcongr|field_simp|ring_nf|push_neg|congr|subst|injection|fin_cases|norm_cast|use|refine|obtain|rcases|specialize|symm|trans|change|convert|generalize|clear|rename_i|repeat|first|try|all_goals|any_goals|focus|rotate_left|swap|have|show|let)\b/;

/** 一个带 `code` 标记的错误：调用方据此区分「基础设施失败 ⇒ fail-open」。 */
function codedError(code, message) {
  const error = new Error(message);
  error.code = code;
  return error;
}

/**
 * 取用户消息里**最后一个**完整围栏代码块（``` 或 ~~~）。
 * @param {string} raw 用户消息原文。
 * @returns {string|null} 代码正文，或 null（没有围栏）。
 */
export function extractFencedCode(raw) {
  if (typeof raw !== 'string') return null;
  const re = /(```|~~~)[ \t]*([A-Za-z0-9_+.-]*)[ \t]*\r?\n([\s\S]*?)\1/g;
  let match = null;
  let last = null;
  while ((match = re.exec(raw)) !== null) last = match;
  return last === null ? null : last[3];
}

/** 最后一个顶层声明在文本里的起始下标；没有返回 -1。 */
export function lastDeclarationStart(text) {
  const value = String(text || '');
  let start = -1;
  DECL_LINE_RE.lastIndex = 0;
  let match;
  while ((match = DECL_LINE_RE.exec(value)) !== null) start = match.index;
  return start;
}

/** 从模板/记录里取目标声明名；匿名 example 返回空串。 */
export function expectedDeclarationName(exercise) {
  const value = exercise || {};
  if (value.anonymous === true) return '';
  if (typeof value.name === 'string' && value.name.trim() !== '') return value.name.trim();
  const match = DECL_NAME_RE.exec(String(value.template || ''));
  return match === null ? '' : match[1];
}

/** 用户代码的两种形态：整条声明，或只交证明体。 */
export function parseUserCode(raw, expectedName = '') {
  const code = String(raw || '').replace(/\r\n/g, '\n').trim();
  if (code === '') return { kind: 'empty', body: '' };
  const decls = [];
  DECL_LINE_RE.lastIndex = 0;
  let match;
  while ((match = DECL_LINE_RE.exec(code)) !== null) {
    const rest = code.slice(match.index);
    const nameMatch = DECL_NAME_RE.exec(rest);
    decls.push({ index: match.index, name: nameMatch === null ? '' : nameMatch[1] });
  }
  const expected = String(expectedName || '');
  let picked = null;
  if (expected !== '') picked = decls.find((item) => item.name === expected) || null;
  if (picked === null && decls.length > 0) {
    const first = decls[0];
    const before = code.slice(0, first.index).replace(/--[^\n]*/g, '').trim();
    if (before === '') picked = first;
  }
  if (picked === null) return { kind: 'body', body: code, name: '' };
  const declaration = code.slice(picked.index);
  const valueAt = declaration.indexOf(':=');
  if (valueAt < 0) return { kind: 'decl', name: picked.name, body: '' };
  return { kind: 'decl', name: picked.name, body: declaration.slice(valueAt + 2) };
}

/** 术语体原样保留；tactic 体补 `by` 并缩进，避免把 `intro` 当项用。 */
function normalizeProofBody(body) {
  const value = String(body || '').trim();
  if (value === '') return '';
  if (/^by\b/.test(value)) return value;
  if (TACTIC_RE.test(value)) {
    const lines = value.split('\n');
    return 'by\n' + lines.map((line) => (line === '' || /^\s/.test(line) ? line : '  ' + line)).join('\n');
  }
  return value;
}

/**
 * 把用户代码贴到**课程模板的可信签名**之后：前缀 + 模板签名 + `:=` + 用户证明体。
 * 用户写的签名一律丢弃 ⇒ 改签名/换命题不能骗过判卷；声明名不一致直接拒绝。
 * @returns {{ok:true,text:string}|{ok:false,code:string,expectedName?:string,foundName?:string}}
 */
export function assembleSubmission(baseText, userCode, options = {}) {
  const base = String(baseText || '');
  const expectedName = typeof options.expectedName === 'string' ? options.expectedName : '';
  const anonymous = options.anonymous === true;
  const declStart = lastDeclarationStart(base);
  if (declStart < 0) return { ok: false, code: 'no-target' };
  const templateDecl = base.slice(declStart);
  const valueAt = templateDecl.indexOf(':=');
  if (valueAt < 0) return { ok: false, code: 'no-target-value' };
  const signature = templateDecl.slice(0, valueAt).replace(/[ \t]+$/, '');
  const prefix = base.slice(0, declStart);
  const parsed = parseUserCode(userCode, expectedName);
  if (parsed.kind === 'empty') return { ok: false, code: 'empty' };
  if (parsed.kind === 'decl' && !anonymous && expectedName !== '') {
    if (parsed.name !== expectedName) {
      return { ok: false, code: 'name-mismatch', expectedName, foundName: parsed.name };
    }
  }
  const body = normalizeProofBody(parsed.body);
  if (body === '') return { ok: false, code: 'empty' };
  return { ok: true, text: prefix + signature + ' := ' + body + '\n' };
}

/** 解析 `soko grade --json` 的 JSONL 事件流；坏行跳过。 */
export function parseGradeEvents(stdout) {
  const events = [];
  for (const line of String(stdout || '').split('\n')) {
    const text = line.trim();
    if (!text.startsWith('{')) continue;
    try {
      const event = JSON.parse(text);
      if (event !== null && typeof event === 'object' && typeof event.type === 'string') events.push(event);
    } catch {
      /* 忽略非事件行 */
    }
  }
  return events;
}

/** 只认目标声明的 checked + 无诊断；exit 0 / 别处的 open 都不算通过。 */
export function evaluateGrade(events, exercise, expectedName = '') {
  const checked = [];
  const open = [];
  const diagnostics = [];
  let exampleChecked = 0;
  for (const event of events) {
    if (event.type === 'decl.checked') checked.push(String(event.name || ''));
    else if (event.type === 'example.checked') exampleChecked += 1;
    else if (event.type === 'exercise.open') open.push(String(event.name || ''));
    else if (event.type === 'diagnostic') {
      diagnostics.push({
        code: String(event.code || event.stage || 'error'),
        message: String(event.message || ''),
        line: event.span !== null && typeof event.span === 'object' && event.span.start !== null
          && typeof event.span.start === 'object' && Number.isFinite(Number(event.span.start.line))
          ? Number(event.span.start.line)
          : null,
      });
    }
  }
  const anonymous = (exercise || {}).anonymous === true;
  const target = anonymous ? '' : (expectedName || expectedDeclarationName(exercise));
  const passed = diagnostics.length === 0 && (anonymous
    ? exampleChecked > 0
    : target !== '' && checked.includes(target) && !open.includes(target));
  return { passed, target, checked, open, diagnostics, exampleChecked };
}

async function runProcess(file, args, options = {}, allowFailure = false) {
  try {
    const result = await execFileAsync(file, args, {
      cwd: options.cwd,
      timeout: options.timeoutMs,
      maxBuffer: options.maxBuffer || 8 * 1024 * 1024,
      encoding: 'utf8',
      env: { ...process.env, PYTHONDONTWRITEBYTECODE: '1', PYTHONIOENCODING: 'utf-8' },
    });
    return { stdout: String(result.stdout || ''), stderr: String(result.stderr || ''), exitCode: 0 };
  } catch (error) {
    if (allowFailure && error !== null && typeof error === 'object' && typeof error.code === 'number') {
      return {
        stdout: String(error.stdout || ''),
        stderr: String(error.stderr || ''),
        exitCode: error.code,
      };
    }
    if (error !== null && typeof error === 'object'
      && (error.killed === true || (error.signal !== undefined && error.signal !== null))) {
      throw codedError('timeout', '判卷进程超时');
    }
    if (error !== null && typeof error === 'object' && error.code === 'ENOENT') {
      throw codedError('missing-tool', '找不到判卷工具');
    }
    throw codedError('run-failed', '判卷进程启动失败');
  }
}

function truncate(value, limit = 400) {
  const text = String(value || '').replace(/\s+/g, ' ').trim();
  return text.length > limit ? text.slice(0, limit) + '…' : text;
}

/**
 * 判一道代码题。返回 `{ok:true, passed, ...}`；基础设施失败抛带 `code` 的错误。
 */
export async function gradeCodeSubmission(options = {}) {
  const repoRoot = path.resolve(options.repoRoot || process.cwd());
  const exercise = options.exercise || {};
  const userCode = String(options.userCode || '');
  const bytes = Buffer.byteLength(userCode, 'utf8');
  if (bytes > (options.maxBytes || CODE_MAX_BYTES)) {
    return { ok: false, code: 'too-large', bytes, max: options.maxBytes || CODE_MAX_BYTES };
  }
  const exerciseId = String(exercise.id || '');
  if (exerciseId === '') return { ok: false, code: 'no-exercise' };
  const course = String(exercise.course || 'set-theory');
  const expectedName = expectedDeclarationName(exercise);
  const composeTimeoutMs = Number(options.composeTimeoutMs) > 0
    ? Number(options.composeTimeoutMs) : DEFAULT_COMPOSE_TIMEOUT_MS;
  const timeoutMs = Number(options.timeoutMs) > 0 ? Number(options.timeoutMs) : DEFAULT_GRADE_TIMEOUT_MS;
  const startedAt = Date.now();
  const workDir = fs.mkdtempSync(path.join(os.tmpdir(), 'soko-dsh-code-'));
  try {
    let baseText = '';
    if (exercise.dataset === true) {
      const composePath = path.join(repoRoot, 'courses', 'exercise-dataset', 'compose.py');
      if (!fs.existsSync(composePath)) throw codedError('missing-compose', '找不到 compose.py');
      const out = path.join(workDir, 'exercise.sokonanoda');
      await runProcess('python3', [composePath, exerciseId, '--out', out, '--json'], {
        cwd: repoRoot,
        timeoutMs: composeTimeoutMs,
      });
      baseText = fs.readFileSync(out, 'utf8');
    } else {
      baseText = String(exercise.template || '');
    }
    if (baseText.trim() === '') return { ok: false, code: 'no-template' };
    const assembled = assembleSubmission(baseText, userCode, {
      expectedName,
      anonymous: exercise.anonymous === true,
    });
    if (assembled.ok !== true) return assembled;
    const targetFile = path.join(workDir, 'submission.sokonanoda');
    fs.writeFileSync(targetFile, assembled.text, 'utf8');
    const soko = path.join(repoRoot, 'scripts', 'soko');
    if (!fs.existsSync(soko)) throw codedError('missing-soko', '找不到 scripts/soko');
    const courseRoot = path.join(repoRoot, 'courses', course);
    const run = await runProcess(process.execPath, [soko, 'grade', '--root', courseRoot, targetFile, '--json'], {
      cwd: repoRoot,
      timeoutMs,
    }, true);
    const events = parseGradeEvents(run.stdout);
    if (events.length === 0) {
      throw codedError('no-events', '判卷没有输出事件：' + truncate(run.stderr));
    }
    const result = evaluateGrade(events, exercise, expectedName);
    return {
      ok: true,
      ...result,
      exitCode: run.exitCode,
      bytes,
      course,
      ms: Date.now() - startedAt,
    };
  } finally {
    try { fs.rmSync(workDir, { recursive: true, force: true }); } catch { /* 临时目录清理失败不影响判卷 */ }
  }
}

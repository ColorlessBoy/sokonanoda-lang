#!/usr/bin/env node
// DeepSeek Harness MCP server for sokonanoda — a thin, zero-dependency stdio
// bridge that forwards every tool call to `scripts/soko query <op>`.
//
// Why it exists: DSH's LSP host drops server diagnostics and never calls custom
// requests, so an agent in DSH cannot see the kernel's judgement unless it runs
// the CLI itself. This server turns the kernel-truth queries into ordinary MCP
// tools (`mcp__sokonanoda__state`, …) so the model gets them as first-class
// tool calls instead of hand-built shell pipelines.
//
// Design: docs/design/agent-query-channel.md §6. Rules kept on purpose:
//   * NO Lean logic here — this file only declares schemas and forwards JSON.
//     All judgement lives in `front::query` behind `sokonanoda query`.
//   * NO environment assumptions — the contract is (cwd = repo root) or
//     SOKO_REPO; nothing else (DSH scrubs credential-shaped names and DSH_*).
//   * Stateless and restartable — the MCP client may probe and reap several
//     processes (see `server/discover` below), so never keep process state.
//
// Wire behaviour verified against the client DSH pins
// (`@modelcontextprotocol/client@2.0.0`):
//   * framing = newline-delimited JSON-RPC 2.0 on stdout (LF), no
//     Content-Length headers; stdout carries MCP messages only, diagnostics go
//     to stderr;
//   * the client first probes `server/discover` (revision 2026-07-28) from a
//     throwaway sibling process. Answering with a JSON-RPC error makes it fall
//     back to the legacy `initialize` handshake immediately; staying silent
//     costs the SDK's full 60 s probe timeout (measured 60,058 ms vs 76 ms);
//   * `initialize` must echo a pre-2026-07-28 protocolVersion the client
//     offered, and the result must advertise `capabilities.tools` — DSH
//     registers zero tools otherwise;
//   * tool results are model-visible only through `content[].text`, so the
//     child's JSON object is forwarded verbatim as text.
//
// Usage: `dsh web --patch ./dsh/cordis.patch.yml` (see dsh/README.md), or run
// it by hand for a smoke test: `echo '<initialize>' | node dsh/mcp/server.js`.

'use strict' // no-op under ESM; this file has no static imports so it loads both ways

// `process.getBuiltinModule` (Node >= 22.3) loads builtins in BOTH module
// systems, so this file works whether the host treats `.js` as CJS or ESM —
// same trick as `scripts/soko`.
const { spawn } = process.getBuiltinModule('node:child_process')
const path = process.getBuiltinModule('node:path')
const fs = process.getBuiltinModule('node:fs')

/** Legacy revisions the pinned client accepts; the client's choice is echoed. */
const LEGACY_VERSIONS = ['2025-11-25', '2025-06-18', '2025-03-26', '2024-11-05', '2024-10-07']
const PROTOCOL_VERSION = '2025-11-25' // version-sensitive: SDK 2.0.0 LATEST_PROTOCOL_VERSION

/**
 * `serverInfo.version` is the **repository** version, read from `Cargo.toml`
 * (the single version source, `docs/RELEASE.md` §2) — never a hand-written
 * constant: the host shows this in its MCP status, and a stale literal is
 * exactly the version-drift failure this repo keeps re-learning. `process.argv[1]`
 * is used instead of `__dirname` so the file keeps working under both CJS and
 * ESM. Unreadable manifest → `0.0.0` (the bridge still works; only the label
 * is unknown).
 */
function repoVersion() {
  try {
    const script = process.argv[1] ?? ''
    const manifest = path.join(path.dirname(script), '..', '..', 'Cargo.toml')
    const match = /^version\s*=\s*"([^"]+)"/m.exec(fs.readFileSync(manifest, 'utf8'))
    return match ? match[1] : '0.0.0'
  } catch {
    return '0.0.0'
  }
}
const SERVER_INFO = { name: 'sokonanoda', version: repoVersion() }
const CAPABILITIES = { tools: { listChanged: false } }
const METHOD_NOT_FOUND = -32601
const INVALID_PARAMS = -32602
const INTERNAL_ERROR = -32603

const log = (...parts) => process.stderr.write(`[sokonanoda-mcp] ${parts.join(' ')}\n`)
const write = message => process.stdout.write(`${JSON.stringify(message)}\n`)
const reply = (id, result) => write({ jsonrpc: '2.0', id, result })
const replyError = (id, code, message) => write({ jsonrpc: '2.0', id, error: { code, message } })

// ── the repository launcher ──────────────────────────────────────────────────

/** `SOKO_REPO` wins (the DSH patch sets `cwd` to the repo root instead). */
function launcherPath() {
  const root = process.env.SOKO_REPO ?? process.cwd()
  return path.join(root, 'scripts', 'soko')
}

/**
 * Run `scripts/soko query <op> …` and return `{ code, json, stderr }`.
 * One child per call: bounded, cancellable, and it keeps this server stateless.
 */
function runQuery(op, flags, source) {
  return new Promise((resolve, reject) => {
    const launcher = launcherPath()
    if (!fs.existsSync(launcher)) {
      reject(new Error(`no launcher at ${launcher}; start the harness from the repository root or set SOKO_REPO`))
      return
    }
    const child = spawn(launcher, ['query', op, ...flags, '--compact'], {
      cwd: path.dirname(path.dirname(launcher)),
      stdio: ['pipe', 'pipe', 'pipe'],
    })
    let stdout = ''
    let stderr = ''
    child.stdout.setEncoding('utf8')
    child.stderr.setEncoding('utf8')
    child.stdout.on('data', chunk => {
      stdout += chunk
    })
    child.stderr.on('data', chunk => {
      stderr += chunk
    })
    child.on('error', reject)
    child.on('close', code => resolve({ code, stdout, stderr }))
    child.stdin.end(source)
  })
}

/** The child's single JSON object, or an error to surface as a tool failure. */
async function queryText(op, flags, source) {
  const { code, stdout, stderr } = await runQuery(op, flags, source)
  const text = stdout.trim()
  if (text === '') {
    throw new Error(
      `sokonanoda query ${op} produced no output (exit ${code})${stderr.trim() ? `: ${stderr.trim()}` : ''}`,
    )
  }
  try {
    JSON.parse(text) // validate: a non-JSON answer must not reach the model as text
  } catch {
    throw new Error(`sokonanoda query ${op} returned non-JSON output: ${text.slice(0, 200)}`)
  }
  // Exit 0 = answered (an open `sorry` is a legal state), 1 = the file has
  // kernel-rejected declarations, 2 = usage. All three carry a usable payload,
  // so forward it; the envelope's `ok`/`data.failed` tells the model the rest.
  if (code !== 0 && code !== 1) {
    throw new Error(`sokonanoda query ${op} failed (exit ${code})${stderr.trim() ? `: ${stderr.trim()}` : ''}`)
  }
  return text
}

// ── tools ────────────────────────────────────────────────────────────────────

/** Shared input: exactly one source channel, like the CLI. */
const sourceProperties = {
  file: { type: 'string', description: 'Path to a .sokonanoda file (relative to the repository root).' },
  text: { type: 'string', description: 'Source text, for an edit that is not on disk yet.' },
}
const positionProperties = {
  line: { type: 'integer', description: 'Cursor line, 1-based.' },
  character: { type: 'integer', description: 'Cursor column, 1-based, counted in UTF-16 code units (same as the LSP `lsp` tool).' },
}

/** Append the source flags for a call. Defaults to stdin when neither is given. */
function sourceFlags(args, stdinSource) {
  if (typeof args.file === 'string' && typeof args.text === 'string') {
    throw new Error('give exactly one of `file` or `text`')
  }
  if (typeof args.file === 'string') return { flags: ['--file', args.file], stdin: '' }
  if (typeof args.text === 'string') return { flags: ['--text', args.text], stdin: '' }
  if (typeof stdinSource === 'string') return { flags: [], stdin: stdinSource }
  throw new Error('give one of `file` or `text`')
}

function positionFlags(args) {
  if (!Number.isInteger(args.line) || !Number.isInteger(args.character)) {
    throw new Error('give both `line` and `character` (1-based)')
  }
  return ['--line', String(args.line), '--col', String(args.character)]
}

/**
 * name -> { description, inputSchema, run(args) }.
 *
 * Names are short on purpose: DSH exposes them as
 * `mcp__sokonanoda__<name>` and truncates past 64 characters.
 * Descriptions say WHEN to use the tool (that text is what the model routes on)
 * and repeat the two rules that matter: judging goes through the kernel, and an
 * open `sorry` is a legal state rather than an error.
 */
const TOOLS = {
  check: {
    description:
      'Grade a whole .sokonanoda file with the real kernel and get counts, kernel rejections and warnings as one JSON object. ' +
      'Use this first, or after editing, to see whether declarations pass. An open `sorry` exercise is a legal state (it appears in `counts.exercise_open`), not an error.',
    inputSchema: {
      type: 'object',
      properties: { ...sourceProperties },
      additionalProperties: false,
    },
    run: args => {
      const { flags, stdin } = sourceFlags(args)
      return queryText('check', flags, stdin)
    },
  },
  state: {
    description:
      'The goal state at a cursor position: remaining goals and the hypotheses in scope, rendered by the kernel. ' +
      'Use it to answer "what do I still have to prove here?" instead of guessing from the source. Cursor on a tactic shows the state ENTERING that tactic.',
    inputSchema: {
      type: 'object',
      properties: { ...sourceProperties, ...positionProperties },
      required: ['line', 'character'],
      additionalProperties: false,
    },
    run: args => {
      const { flags, stdin } = sourceFlags(args)
      return queryText('state', [...flags, ...positionFlags(args)], stdin)
    },
  },
  goals: {
    description:
      'Every declaration in the file with its kernel-rendered type, status, open goals and addressable holes. ' +
      'Use it for a whole-file overview or to find the open exercises. Set `probe` to also fill each sub-hole expected type via a kernel probe. ' +
      'A hole with `redundant: true` is a LEFTOVER `sorry` (the answer already proves the goal): say "delete that line", never "not yet solved".',
    inputSchema: {
      type: 'object',
      properties: { ...sourceProperties, probe: { type: 'boolean', description: 'Run the kernel probe to fill sub-goal expected types (slower).' } },
      additionalProperties: false,
    },
    run: args => {
      const { flags, stdin } = sourceFlags(args)
      return queryText('goals', args.probe === true ? [...flags, '--probe'] : flags, stdin)
    },
  },
  holes: {
    description:
      'List every `sorry` hole with a stable id (`declName:index`), its expected type and a `redundant` flag, optionally stepping to the next/previous hole. ' +
      'Use the id — two sub-goals of one `apply` share a source position, so positional stepping crosses them as a group. ' +
      '`redundant: true` means the answer is already complete and that line must be deleted (docs/design/redundant-sorry.md).',
    inputSchema: {
      type: 'object',
      properties: {
        ...sourceProperties,
        offset: { type: 'integer', description: 'Byte offset to step from (pair with `direction`).' },
        direction: { type: 'string', enum: ['next', 'prev'], description: 'Step direction from `offset`.' },
      },
      additionalProperties: false,
    },
    run: args => {
      const { flags, stdin } = sourceFlags(args)
      const extra = []
      if (Number.isInteger(args.offset)) extra.push('--offset', String(args.offset))
      if (args.direction === 'next' || args.direction === 'prev') extra.push('--direction', args.direction)
      return queryText('holes', [...flags, ...extra], stdin)
    },
  },
  hints: {
    description:
      'The hint ladder the canvas author wrote for the declaration at a cursor (`-- soko:hint` comments), in order. ' +
      'Use it when a learner is stuck; reveal one step at a time. The answers are never part of the ladder.',
    inputSchema: {
      type: 'object',
      properties: { ...sourceProperties, ...positionProperties },
      required: ['line', 'character'],
      additionalProperties: false,
    },
    run: args => {
      const { flags, stdin } = sourceFlags(args)
      return queryText('hints', [...flags, ...positionFlags(args)], stdin)
    },
  },
  reduce: {
    description:
      'Evaluate an expression with the kernel and return its normal form. Use it to check what a definition computes to, e.g. `1 + 1`.',
    inputSchema: {
      type: 'object',
      properties: { ...sourceProperties, expr: { type: 'string', description: 'The expression to evaluate.' } },
      required: ['expr'],
      additionalProperties: false,
    },
    run: args => {
      if (typeof args.expr !== 'string' || args.expr.trim() === '') {
        throw new Error('`expr` must be a non-empty string')
      }
      const { flags, stdin } = sourceFlags(args, '')
      return queryText('reduce', [...flags, '--expr', args.expr], stdin)
    },
  },
}

const listTools = () =>
  Object.entries(TOOLS).map(([name, tool]) => ({
    name,
    description: tool.description,
    inputSchema: tool.inputSchema,
  }))

// ── dispatch ─────────────────────────────────────────────────────────────────

async function callTool(id, params) {
  const name = typeof params?.name === 'string' ? params.name : ''
  const tool = Object.hasOwn(TOOLS, name) ? TOOLS[name] : undefined
  if (tool === undefined) {
    // Protocol error: an unknown tool name is the client's mistake, and DSH
    // surfaces the JSON-RPC error as a model-visible failure too.
    replyError(id, METHOD_NOT_FOUND, `Unknown tool: ${name}`)
    return
  }
  const args = params?.arguments ?? {}
  if (typeof args !== 'object' || args === null || Array.isArray(args)) {
    replyError(id, INVALID_PARAMS, 'arguments must be an object')
    return
  }
  try {
    const text = await tool.run(args)
    reply(id, { content: [{ type: 'text', text }] })
  } catch (error) {
    // Tool failure (bad input, missing binary, kernel-rejected payload the model
    // should see): a successful response with isError, so the model can correct
    // itself from the text.
    reply(id, {
      content: [{ type: 'text', text: error instanceof Error ? error.message : String(error) }],
      isError: true,
    })
  }
}

async function dispatch({ id, method, params }) {
  const isRequest = id !== undefined && id !== null
  switch (method) {
    case 'server/discover':
      // The 2026-07-28 probe: refuse fast so the client classifies us legacy.
      // Never answer -32022 (that would make it think we are modern).
      replyError(id, METHOD_NOT_FOUND, 'Method not found: server/discover (legacy server)')
      return
    case 'initialize': {
      const requested = params?.protocolVersion
      reply(id, {
        protocolVersion: LEGACY_VERSIONS.includes(requested) ? requested : PROTOCOL_VERSION,
        capabilities: CAPABILITIES,
        serverInfo: SERVER_INFO,
        // Server instructions become their own system-prompt section; keep them
        // short and put the invariants the model must not get wrong here.
        instructions:
          'sokonanoda grades Lean-style proofs with its own complete kernel. Every tool forwards to that kernel: ' +
          'never judge a proof by looking at the text. An open `sorry` is a legal, in-progress state (reported as ' +
          '`exercise_open`), not an error. Positions are 1-based line/character with UTF-16 columns, matching the ' +
          '`lsp` tool. Prefer `state` over guessing what remains to be proved, and `check` after an edit.',
      })
      return
    }
    case 'notifications/initialized':
      return // client -> server only
    case 'notifications/cancelled':
      log('cancelled', String(params?.requestId))
      return
    case 'ping':
      if (isRequest) reply(id, {})
      return
    case 'tools/list':
      // Never emit nextCursor: the client walks pages up to 64 times.
      if (isRequest) reply(id, { tools: listTools() })
      return
    case 'tools/call':
      if (isRequest) await callTool(id, params)
      return
    default:
      if (isRequest) replyError(id, METHOD_NOT_FOUND, `Method not found: ${String(method)}`)
  }
}

// ── stdio framing ────────────────────────────────────────────────────────────

let buffer = ''
process.stdin.setEncoding('utf8')
process.stdin.on('data', chunk => {
  buffer += chunk
  let index
  while ((index = buffer.indexOf('\n')) !== -1) {
    const line = buffer.slice(0, index).replace(/\r$/, '')
    buffer = buffer.slice(index + 1)
    if (line.trim() === '') continue
    let message
    try {
      message = JSON.parse(line)
    } catch {
      log('dropped non-JSON line')
      continue
    }
    Promise.resolve()
      .then(() => dispatch(message))
      .catch(error => {
        log('internal error', String(error))
        if (message && message.id !== undefined) replyError(message.id, INTERNAL_ERROR, String(error))
      })
  }
})

// Shutdown: the client (and its disposable probe) closes stdin. Set exitCode
// rather than process.exit() so queued stdout writes flush first.
process.stdin.on('end', () => {
  process.exitCode = 0
})
process.on('SIGTERM', () => {
  process.exitCode = 0
})

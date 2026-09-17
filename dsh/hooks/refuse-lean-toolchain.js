#!/usr/bin/env node
// DeepSeek Harness `PreToolUse` hook: refuse the official Lean toolchain.
//
// `REQUIREMENTS.md` §2 rule 2 forbids calling lean / lake / lean4export / leanc /
// elan from this repository. opencode enforces that with a permission rule in
// `opencode.json`; DSH has no command-pattern policy, so the equivalent is a
// Claude Code style `PreToolUse` hook used through `@deepseek-ai/dsh-hooks-claude-code`.
//
// DSH reads one process-level `configPath` and does NOT discover a repository's
// hook configuration, so enabling this takes one line in the user's profile —
// see `dsh/README.md` § "禁用官方 Lean 工具链".
//
// Protocol (packages/hooks/hook-protocol): a JSON object on stdin with
// `tool_name` and `tool_input`; exit 2 blocks the call and stderr becomes the
// model-visible reason, or print the documented JSON decision. This script uses
// exit 2 — the simplest reliable signal.

// A command *position*: start of string, or right after a shell separator /
// substitution opener, with optional leading whitespace. Matching the position
// rather than the bare word keeps `grep -rn lean docs/` or `ls lean.lean`
// allowed while still blocking the real invocations.
const AT_COMMAND_START = String.raw`(?:^|[\n;|&(){}\`]|\$\()\s*`

const DENY = [
  {
    // lean / lake / leanc / lean4export / elan, optional .exe, as the command.
    pattern: new RegExp(`${AT_COMMAND_START}(?:lean|lake|leanc|lean4export|elan)(?:\\.exe)?(?:\\s|$)`),
    name: 'lean / lake / leanc / lean4export / elan',
  },
]

function main() {
  let payload = ''
  process.stdin.setEncoding('utf8')
  process.stdin.on('data', chunk => {
    payload += chunk
  })
  process.stdin.on('end', () => {
    let event
    try {
      event = JSON.parse(payload || '{}')
    } catch {
      // Unparseable input is not our business: let the tool run.
      process.exit(0)
    }
    if (event.tool_name !== 'bash' && event.tool_name !== 'pwsh') process.exit(0)
    const command = String(event.tool_input?.command ?? '')
    if (!command) process.exit(0)
    for (const { pattern, name } of DENY) {
      if (pattern.test(command)) {
        process.stderr.write(
          `sokonanoda: refusing \`${name}\` — this repository must not depend on the official Lean ` +
            'toolchain (REQUIREMENTS.md §2 rule 2). The teaching compiler is self-contained: use ' +
            '`scripts/soko grade <file> --json`, `scripts/soko gate`, or `cargo` for contributor builds.\n',
        )
        process.exit(2)
      }
    }
    process.exit(0)
  })
}

main()

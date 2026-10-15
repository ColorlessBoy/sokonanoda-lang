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
//
// ── 命令位：拦「调用」，放「引用」──────────────────────────────────────────────
// （2026-10-15 修 `SUB-toolchain-guard-code-fence-false-positive`，用户亲报）
// 硬规则 2 禁的是**调用**；**引用**（证据 / commit message / 文档里贴工具链名字）
// 必须放行 —— 否则守卫会挡住本仓自己要求的动作：「卡住 ⇒ 先读 Lean 4 源码，第一次
// 动手前先交 3–6 行『Lean 4 对照』⇒ **落 commit message**」（2026-10-05 拍板）。
// 用户实测代价：`ux-queue.py --submit` 两次被拒，最后靠 sed 把词替换进去才过 ✗。
//
// 旧口径 `(?:^|[\n;|&(){}\`]|\$\()\s*` 把**反引号**也算作命令位 ⇒ markdown 代码
// 围栏（三反引号 + 语言标签）与行内引用 `` `lake` `` 全部命中 ✗。新口径分两步：
//
//   ① 先剥**文档块载荷**：围栏块（``` / ~~~）与 markdown 表格行。它们不是 shell
//      语法，里面的行首工具链名是「引用」。剥掉不损失真调用 —— shell 里一行以 `|`
//      开头是语法错误（上一行 `\` 续行的那种保留 ✓），围栏块更不可能是命令。
//   ② 命令位 = shell 的**语句起点**：串首 / 换行 / `;` `|` `&`（含 `&&` `||`）/
//      命令替换 `$(` / 进程替换 `<(` `>(` / 保留字后的子壳 `do (…)` / 分组 `(` `{`
//      —— **分组符自己也得在命令位上**：shell 里 `foo (x)` 是语法错误，而散文里的
//      `(lean 4)` 正是这个形状 ⇒ 既放行引用、又不放过 `(lake build)` ✓。
//      ⚠ **反引号不算命令位**：它在 shell 里是过时命令替换、在 markdown 里是引用
//      语法，本仓的文档载荷大量用后者 ⇒ 混为一谈就是「写文档被拦」。现代写法
//      `$( … )` 照旧拦 ✓（自检 `cmd-subst`）；代价是 `` `lean --version` `` 这种
//      过时命令替换不再单独命中（里面的 `;` `&&` 照旧命中）。
//
// 口径由 `dsh/hooks/refuse-lean-toolchain.test.js` 钉住（改口径必须同时改它）：
//   node dsh/hooks/refuse-lean-toolchain.test.js

// ① 文档块载荷 —— markdown 的**块结构**（shell 里没有别的含义）。
// 围栏块要求开合成对（不成对 ⇒ 原样保留：宁可漏放，不可误剥真命令）。
const FENCED_BLOCK = /^[ \t]*(`{3,}|~{3,})[^\n]*\n[\s\S]*?^[ \t]*\1[ \t]*(?=\n|$)/gm
// 表格行 = 行首（可缩进）的 `|`；上一行以 `\` 续行时那是真管道 ⇒ 保留。
const TABLE_ROW = /(^|(?<!\\)\n)[ \t]*\|[^\n]*/g

// ② 命令位 = shell 的语句起点（逐条见文件头）。
const AT_COMMAND_START =
  String.raw`(?:^|[\n;|&)]|\$\(|[<>]\()\s*` +
  String.raw`(?:(?:do|then|else|elif|if|while|until|time|!)\s+)?` +
  String.raw`(?:[({]\s*)*`

const DENY = [
  {
    // lean / lake / leanc / lean4export / elan, optional .exe, as the command.
    pattern: new RegExp(`${AT_COMMAND_START}(?:lean|lake|leanc|lean4export|elan)(?:\\.exe)?(?:\\s|$)`),
    name: 'lean / lake / leanc / lean4export / elan',
  },
]

// 剥掉文档块载荷后的命令串 —— 判定与自检**共用这一条**（别在别处重写）。
function stripDocBlocks(command) {
  return command.replace(FENCED_BLOCK, '').replace(TABLE_ROW, '$1')
}

// 命中就返回那一条的 name，否则 null。
function denyName(command) {
  const code = stripDocBlocks(command)
  for (const { pattern, name } of DENY) {
    if (pattern.test(code)) return name
  }
  return null
}

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
    const name = denyName(command)
    if (name) {
      process.stderr.write(
        `sokonanoda: refusing \`${name}\` — this repository must not depend on the official Lean ` +
          'toolchain (REQUIREMENTS.md §2 rule 2). The teaching compiler is self-contained: use ' +
          '`scripts/soko grade <file> --json`, `scripts/soko gate`, or `cargo` for contributor builds.\n',
      )
      process.exit(2)
    }
    process.exit(0)
  })
}

module.exports = { AT_COMMAND_START, DENY, stripDocBlocks, denyName }

if (require.main === module) main()

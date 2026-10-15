#!/usr/bin/env node
'use strict'

// Self-test for `refuse-lean-toolchain.js` — the DSH `PreToolUse` deny hook.
//
//   node dsh/hooks/refuse-lean-toolchain.test.js [<hook.js>]
//
// exit 0 = 每条都符合预期 · exit 1 = 至少一条不符（打印逐条表）。
//
// 为什么有这份自检：守卫的全部工作就是一条**边界** —— 「调用」必须拦、「引用」必须放。
// 正则读一遍是审不出边界的（2026-10-15 实测：命令位字符集里那个反引号让**每个**
// markdown 围栏 + 语言标签、**每个**行内 `` `名字` `` 都命中 ⇒ 守卫挡住了本仓自己
// 要求写的文档 ✗，用户两次 `--submit` 被拒、最后靠 sed 把词替换进去才过）。
// 所以边界在这里用**用例**钉死，而且跑的是**真程序 + 真协议**（stdin 喂 JSON、exit 2 = 拦），
// 不是把正则抄一遍 —— 抄一遍的守卫咬不住自己 ✗。
//
// 五组：判定表（本洞验收面）· 真命令（不许放过去）· 引用（必须放）·
// 残余误报（**断言当前行为**，每条写明为什么留着）· 有意让路 / 已知缺口。

const fs = require('node:fs')
const path = require('node:path')
const crypto = require('node:crypto')
const { spawnSync } = require('node:child_process')

const HOOK = process.argv[2] || path.join(__dirname, 'refuse-lean-toolchain.js')

const fence = (...lines) => lines.join('\n')

const CASES = [
  // ── 一、判定表（用户 2026-10-15 报的那四行；本洞的验收面）──────────────────
  { id: 'fence-lang-tag', group: 'table', expect: 'allow',
    cmd: fence('```lean', 'theorem foo : True := trivial', '```'),
    note: '代码围栏（三反引号 + 语言标签）⇒ 引用，不是调用' },
  { id: 'inline-ref', group: 'table', expect: 'allow',
    cmd: 'the rule forbids the `lean --version` form',
    note: '行内反引号引用工具链名字' },
  { id: 'real-cmd', group: 'table', expect: 'block',
    cmd: 'lean --version',
    note: '真正的命令（名字 + --version）⇒ 必须拦' },
  { id: 'doc-grep', group: 'table', expect: 'allow',
    cmd: '`grep -rn lean docs/`',
    note: '文档里的 grep ⇒ 放行（旧口径也放行）' },

  // ── 二、真命令正对照（不许为了放行围栏而放过去）──────────────────────────
  { id: 'lake-build', group: 'positive', expect: 'block', cmd: 'lake build' },
  { id: 'leanc', group: 'positive', expect: 'block', cmd: 'leanc foo.c -o foo' },
  { id: 'lean4export', group: 'positive', expect: 'block', cmd: 'lean4export out.olean' },
  { id: 'elan', group: 'positive', expect: 'block', cmd: 'elan toolchain list' },
  { id: 'exe-suffix', group: 'positive', expect: 'block', cmd: 'lean.exe --version' },
  { id: 'and-chain', group: 'positive', expect: 'block', cmd: 'cd /tmp && lake build' },
  { id: 'or-chain', group: 'positive', expect: 'block', cmd: 'false || elan install' },
  { id: 'cmd-subst', group: 'positive', expect: 'block', cmd: 'echo $(lean --version)',
    note: '现代命令替换：反引号让路，`$()` 照旧拦' },
  { id: 'semicolon', group: 'positive', expect: 'block', cmd: 'ls; lean --version' },
  { id: 'pipe', group: 'positive', expect: 'block', cmd: 'cat x | lake build' },
  { id: 'newline', group: 'positive', expect: 'block', cmd: 'cd /tmp\nlake build' },
  { id: 'indented-cmd', group: 'positive', expect: 'block', cmd: 'if [ -f x ]; then\n  lake build\nfi',
    note: '缩进在 shell 里不改变命令位 ⇒ 不许当文档剥掉' },
  { id: 'subshell-start', group: 'positive', expect: 'block', cmd: '(lake build)' },
  { id: 'subshell-after-and', group: 'positive', expect: 'block', cmd: 'true && (lake build)' },
  { id: 'nested-subshell', group: 'positive', expect: 'block', cmd: '((lake build))' },
  { id: 'process-subst', group: 'positive', expect: 'block', cmd: 'diff <(lean --version) other' },
  { id: 'brace-group', group: 'positive', expect: 'block', cmd: '{ lake build; }' },
  { id: 'func-body', group: 'positive', expect: 'block', cmd: 'f() { lake build; }' },
  { id: 'case-arm', group: 'positive', expect: 'block', cmd: 'case x in a) lake build;; esac' },
  { id: 'reserved-do', group: 'positive', expect: 'block', cmd: 'do (lake build)' },
  { id: 'reserved-then', group: 'positive', expect: 'block', cmd: 'then (lake build)' },
  { id: 'reserved-while', group: 'positive', expect: 'block', cmd: 'while (lean --version); do :; done' },
  { id: 'chained-real', group: 'positive', expect: 'block', cmd: 'cd /tmp && lake build && lean --version' },

  // ── 三、引用 / 文档（必须放行）────────────────────────────────────────────
  { id: 'fence-bash-body', group: 'reference', expect: 'allow',
    cmd: fence('```bash', 'lake build', '```'),
    note: '围栏**内容**里的行首命令也是引用（文档载荷）' },
  { id: 'fence-untagged', group: 'reference', expect: 'allow', cmd: fence('```', 'lake build', '```') },
  { id: 'fence-tilde', group: 'reference', expect: 'allow',
    cmd: fence('~~~lean4', 'example : True := trivial', '~~~') },
  { id: 'fence-lang-is-name', group: 'reference', expect: 'allow', cmd: fence('```lake', 'build', '```'),
    note: '语言标签本身是工具链名' },
  { id: 'fence-real-world', group: 'reference', expect: 'allow',
    cmd: ['python3 scripts/ux-queue.py --submit gate --where dsh/hooks/refuse-lean-toolchain.js:22 \\',
      '  --why "guard blocks its own docs" --evidence "',
      '## 判定表',
      fence('```bash', 'lean --version', 'lake build', '```'),
      '⇒ 围栏里的是引用。"'].join('\n'),
    note: '本洞的真实形状：`--submit` 的 evidence 里带围栏' },
  { id: 'table-rows', group: 'reference', expect: 'allow',
    cmd: ['| 工具 | 判定 |', '|---|---|', '| lean | 禁止 |', '| lean4export | 禁止 |'].join('\n'),
    note: 'markdown 表格行：`|` 是表格分隔符，不是管道' },
  { id: 'table-row-single', group: 'reference', expect: 'allow', cmd: '| lake build |' },
  { id: 'quote-paren', group: 'reference', expect: 'allow', cmd: 'echo "see (lean 4 docs)"',
    note: '引号里的散文括号 ⇒ 引用（shell 里 `foo (x)` 是语法错误）' },
  { id: 'quote-brace', group: 'reference', expect: 'allow', cmd: 'echo "{lean 4}"' },
  { id: 'rule-text', group: 'reference', expect: 'allow',
    cmd: 'echo "不调用官方 Lean 工具链（lean/lake/lean4export/leanc/elan）"',
    note: 'REQUIREMENTS.md §2 规则原文本身' },
  { id: 'rule-text-ascii', group: 'reference', expect: 'allow',
    cmd: "echo 'rule 2: no (lean/lake/lean4export/leanc/elan) calls'" },
  { id: 'commit-msg-inline', group: 'reference', expect: 'allow',
    cmd: 'git commit -m "docs: `lake build` is forbidden (REQUIREMENTS §2)"' },
  { id: 'grep-docs', group: 'reference', expect: 'allow', cmd: 'grep -rn lake docs/' },
  { id: 'ls-toolchain-file', group: 'reference', expect: 'allow', cmd: 'ls lean-toolchain' },
  { id: 'path-not-command', group: 'reference', expect: 'allow', cmd: 'cat ~/lean/README.md' },
  { id: 'word-prefix', group: 'reference', expect: 'allow', cmd: 'cleanup.sh --now' },

  // ── 四、残余误报（**断言当前行为**防漂移；每条写明为什么留着）──────────────
  { id: 'residual-close-paren', group: 'residual', expect: 'block',
    cmd: 'the toolchain (see REQUIREMENTS §2) lean is forbidden',
    note: '残余误报：`)` 仍是命令位。修它要丢掉 `f() { … }` / `case a) …` 两个真形状 ⇒ 留着' },
  { id: 'residual-indented-block', group: 'residual', expect: 'block',
    cmd: 'doc:\n    lean --version',
    note: '残余误报：缩进代码块。缩进在 shell 里不改变命令位（见 indented-cmd）⇒ 留着' },
  { id: 'residual-reserved-prose', group: 'residual', expect: 'block',
    cmd: 'then (lean 4) is the tag',
    note: '残余误报：保留字后跟括号的散文。为不放过 `do (lake build)`（reserved-do）而留' },
  { id: 'residual-midline-table', group: 'residual', expect: 'block',
    cmd: 'echo "| NDJSON | lean4export 的导出格式（上游内核的检查输入） |"',
    note: '残余误报：**行内**表格行（docs/architecture.md:642 的真实行）。行首的表格行会剥掉' +
      '（table-rows），行内这个与真管道 `a | b` 无法区分 ⇒ 留着' },
  { id: 'residual-midline-fence', group: 'residual', expect: 'block',
    cmd: ['python3 scripts/ux-queue.py --submit gate --evidence "```bash', 'lake build', '```"'].join('\n'),
    note: '残余误报：围栏开头**不在行首**（紧跟引号）⇒ 按 markdown 规则它**不是**围栏 ⇒ 不剥，' +
      '里面那个行首名字照旧拦（保守方向）。正解 = 把围栏写到行首（fence-real-world ✓）；' +
      '本条是写复核命令时自己踩到的形状 ⇒ 留档' },

  // ── 五、有意让路 / 已知缺口（旧口径拦、新口径放；逐条给理由）────────────────
  { id: 'tradeoff-backtick-subst', group: 'gap', expect: 'allow',
    cmd: 'echo `lean --version`',
    note: '有意取舍：过时反引号命令替换不再单独命中（`$( … )` 照旧拦）—— 反引号=markdown 引用' },
  { id: 'gap-invalid-shell-paren', group: 'gap', expect: 'allow',
    cmd: 'foo (lake build)',
    note: '已知缺口：shell 里 `foo (x)` 本身是语法错误 ⇒ 不算真调用（旧口径误拦）' },
  { id: 'gap-absolute-path', group: 'gap', expect: 'allow',
    cmd: '/usr/local/bin/lean --version',
    note: '已知缺口（旧口径也有）：绝对路径调用认不出 —— 本守卫是绊线，不是沙箱' },
  { id: 'gap-env-prefix', group: 'gap', expect: 'allow',
    cmd: 'env lean --version',
    note: '已知缺口（旧口径也有）：`env`/`xargs`/`bash -c` 这类间接调用认不出' },
]

const GROUPS = [
  ['table', '一、判定表（本洞验收面）'],
  ['positive', '二、真命令正对照（必须拦）'],
  ['reference', '三、引用 / 文档（必须放行）'],
  ['residual', '四、残余误报（断言当前行为，留档）'],
  ['gap', '五、有意让路 / 已知缺口（旧口径拦、新口径放）'],
]

// 跑**真程序 + 真协议**：stdin 喂一个 PreToolUse 事件，exit 2 = 拦。
function verdict(hook, command) {
  const run = spawnSync(process.execPath, [hook], {
    input: JSON.stringify({ tool_name: 'bash', tool_input: { command } }),
    encoding: 'utf8',
  })
  if (run.error) throw run.error
  if (run.status === 0) return 'allow'
  if (run.status === 2) return 'block'
  return `exit=${run.status}`
}

const source = fs.readFileSync(HOOK)
console.log(`hook:   ${HOOK}`)
console.log(`身份:   sha256=${crypto.createHash('sha256').update(source).digest('hex').slice(0, 16)}` +
  ` · ${source.length} bytes`)

let failed = 0
for (const [group, title] of GROUPS) {
  const cases = CASES.filter(c => c.group === group)
  if (!cases.length) continue
  console.log(`\n${title}`)
  for (const c of cases) {
    const got = verdict(HOOK, c.cmd)
    const ok = got === c.expect
    if (!ok) failed += 1
    console.log(`  ${ok ? 'ok  ' : 'FAIL'} ${c.id.padEnd(24)} expect=${c.expect.padEnd(5)} got=${got}`)
    if (c.note) console.log(`       ↳ ${c.note}`)
    if (!ok) console.log(`       cmd = ${JSON.stringify(c.cmd)}`)
  }
}

console.log(`\n合计：${CASES.length} 条 · 不符 ${failed} 条`)
process.exit(failed ? 1 : 0)

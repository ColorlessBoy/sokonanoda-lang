/**
 * Run both guards of `sokonanoda-dsh` and exit non-zero when either fails.
 *
 *   node test/run.mjs
 */
import { spawnSync } from 'node:child_process'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const TEST_DIR = dirname(fileURLToPath(import.meta.url))
const entries = [
  ['host', join(TEST_DIR, 'host.test.mjs')],
  ['client smoke', join(TEST_DIR, 'client.smoke.cjs')],
]

let failed = false
for (const [label, file] of entries) {
  console.log('== ' + label + ' (' + file + ')')
  const run = spawnSync(process.execPath, [file], { stdio: 'inherit' })
  if (run.status !== 0) {
    failed = true
    console.error('-- ' + label + ' exited with ' + (run.status === null ? 'signal ' + run.signal : 'code ' + run.status))
  }
}
if (failed) process.exitCode = 1

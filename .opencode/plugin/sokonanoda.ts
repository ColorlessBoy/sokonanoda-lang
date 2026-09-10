// Auto-provision the version-pinned CLI/LSP at opencode startup (best effort)
// by running `scripts/soko.sh setup`, and expose the cache directory to shell
// tools via `shell.env`.
//
// Design: docs/design-onboarding.md. The provisioning step is idempotent and
// fast when the cache is current; failures never block the session (opencode
// logs plugin errors and continues). Set SOKONANODA_OFFLINE=1 to stay offline.
import { spawnSync } from "node:child_process"
import { existsSync } from "node:fs"
import path from "node:path"
import type { Plugin } from "@opencode-ai/plugin"

/// opencode may be opened in a repo subdirectory; walk up to the repo root
/// that owns `scripts/soko.sh`.
function findRepoRoot(start: string): string | undefined {
  let dir = start
  for (;;) {
    if (existsSync(path.join(dir, "scripts", "soko.sh"))) return dir
    const parent = path.dirname(dir)
    if (parent === dir) return undefined
    dir = parent
  }
}

export const Sokonanoda: Plugin = async ({ directory }) => {
  const repo = findRepoRoot(directory)
  const script = repo ? path.join(repo, "scripts", "soko.sh") : undefined
  if (script && !process.env.SOKONANODA_OFFLINE) {
    spawnSync("bash", [script, "setup"], { stdio: "ignore", timeout: 120000 })
  }
  return {
    "shell.env": async (_input, output) => {
      const cache = path.join(
        process.env.HOME ?? "",
        ".local",
        "share",
        "sokonanoda",
        "bin",
      )
      output.env ??= {}
      const current = output.env.PATH ?? process.env.PATH ?? ""
      if (!current.split(path.delimiter).includes(cache)) {
        output.env.PATH = `${cache}${path.delimiter}${current}`
      }
    },
  }
}
